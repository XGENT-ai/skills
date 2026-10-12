use crate::{
    capture::{append_capture, CaptureAttempt, CaptureStatus},
    capture_evidence::{
        self, CaptureContext, CaptureInput, FileEvidence, Observation, TestedVersion,
    },
    matrix::FrozenMatrix,
    projection::project_block,
    publication::ReportFile,
    storage::{error, hash, CycleLock, Snapshot, StoreError},
    target, target_matrix, Decimal,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub mod applied;
pub mod finalize;
pub mod history;
pub mod judgement;
pub mod legacy;
pub mod packet;
pub mod status;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoundLink {
    pub round_id: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cycle {
    pub schema_version: u32,
    pub mode: String,
    pub rules_version: u32,
    pub cycle_id: String,
    pub prefix: String,
    pub target: String,
    pub plan: Option<String>,
    pub record_path: Option<String>,
    pub next_issue_id: u64,
    #[serde(default)]
    pub issues: Vec<judgement::Issue>,
    pub revision: u64,
    pub rounds: Vec<RoundLink>,
    /// Append-only whole-UI waivers (用户豁免通过) added after a round was sealed;
    /// each round's generated waiver block inside its report is derived from them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waivers: Vec<finalize::Waiver>,
    /// Present only for cycles taken over from Markdown history by `review import`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy: Option<legacy::LegacyRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenTarget {
    pub algorithm: String,
    pub fingerprint: String,
    pub source: FileEvidence,
    pub expectations: FileEvidence,
    pub attachments: Vec<FrozenAttachment>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenAttachment {
    pub logical_path: String,
    pub source: Option<FileEvidence>,
    pub frozen: Option<FileEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisteredCapture {
    pub input: CaptureInput,
    pub content_hash: String,
    pub image_pixels: Option<[u32; 2]>,
    pub frozen_image: Option<FileEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finalization {
    pub status: String,
    pub result: FileEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Round {
    pub schema_version: u32,
    pub revision: u64,
    pub cycle_id: String,
    pub prefix: String,
    pub round_id: String,
    pub baseline: String,
    pub target_fingerprint: String,
    pub target_snapshot: FrozenTarget,
    pub scope: Option<Vec<String>>,
    pub full: bool,
    pub matrix: FrozenMatrix,
    pub threshold: Decimal,
    pub tested_version: TestedVersion,
    pub captures: Vec<RegisteredCapture>,
    pub judges: Vec<FileEvidence>,
    pub machine_checks: Vec<FileEvidence>,
    #[serde(default)]
    pub packets: Vec<packet::PacketRef>,
    pub finalization: Option<Finalization>,
    pub capture_projection_hash: Option<String>,
}

pub struct BeginOptions {
    pub prefix: String,
    pub target: String,
    pub plan: Option<String>,
    pub scope: Option<Vec<String>>,
    pub threshold: Option<Decimal>,
    pub version: TestedVersion,
}

fn relative(value: &str) -> Result<PathBuf, StoreError> {
    if value.contains(['\\', ':'])
        || value
            .split('/')
            .any(|p| p.is_empty() || matches!(p, "." | ".."))
    {
        return Err(error(
            2,
            "Review paths must be canonical project-relative paths",
        ));
    }
    Ok(value.into())
}

fn checked(root: &Path, value: &str) -> Result<PathBuf, StoreError> {
    let relative = relative(value)?;
    let mut path = root.to_path_buf();
    for part in relative.components() {
        path.push(part.as_os_str());
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(error(2, "Review paths cannot follow symbolic links"))
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(error(4, e)),
        }
    }
    Ok(path)
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, StoreError> {
    serde_json::to_vec_pretty(value).map_err(|e| error(2, e))
}

fn cycle_id(prefix: &str) -> String {
    hash(format!("phoenix-ui-cycle-v1:{prefix}").as_bytes())
}

fn ordinal(id: &str) -> Result<u32, StoreError> {
    let n = id
        .strip_prefix('r')
        .and_then(|n| n.parse::<u32>().ok())
        .filter(|n| *n > 0 && format!("r{n}") == id)
        .ok_or_else(|| error(4, "Round identity is invalid"))?;
    Ok(n)
}

pub fn load_cycle(lock: &CycleLock, prefix: &str) -> Result<Option<Cycle>, StoreError> {
    relative(prefix)?;
    let Some(snapshot) = lock.read(Path::new(&format!("{prefix}.ui-review.json")))? else {
        return Ok(None);
    };
    let cycle: Cycle = serde_json::from_slice(&snapshot.bytes).map_err(|e| error(4, e))?;
    if cycle.schema_version != 1
        || cycle.mode != "managed"
        || cycle.rules_version != 1
        || cycle.prefix != prefix
        || cycle.cycle_id != cycle_id(prefix)
        || cycle.next_issue_id == 0
        || cycle.revision != snapshot.revision
        || (cycle.rounds.is_empty() && cycle.legacy.is_none())
        || cycle.legacy.as_ref().is_some_and(|legacy| {
            legacy.marker != legacy::MARKER
                || legacy.map.path != legacy::map_path(prefix)
                || ordinal(&legacy.last_round).is_err()
        })
    {
        return Err(error(4, "Cycle identity, mode or schema is invalid"));
    }
    let legacy_rounds = match &cycle.legacy {
        Some(legacy) => ordinal(&legacy.last_round)?,
        None => 0,
    };
    relative(&cycle.target)?;
    if let Some(plan) = &cycle.plan {
        relative(plan)?;
    }
    if let Some(record) = &cycle.record_path {
        relative(record)?;
    }
    let mut previous = legacy_rounds;
    for link in &cycle.rounds {
        let n = ordinal(&link.round_id)?;
        if n <= previous || link.path != round_path(prefix, n).to_string_lossy() {
            return Err(error(
                4,
                "Cycle round links must be unique and in round order",
            ));
        }
        previous = n;
    }
    let mut ids = std::collections::BTreeSet::new();
    if cycle.issues.iter().any(|issue| {
        !judgement::issue_number(&issue.id).is_some_and(|n| n < cycle.next_issue_id)
            || !ids.insert(&issue.id)
            || issue.unit_id.trim().is_empty()
            || !(cycle
                .rounds
                .iter()
                .any(|link| link.round_id == issue.first_round)
                || ordinal(&issue.first_round).is_ok_and(|n| n <= legacy_rounds))
    }) {
        return Err(error(
            4,
            "Cycle issue identities or allocation history changed",
        ));
    }
    if cycle.waivers.iter().any(|waiver| {
        waiver.decision.kind != finalize::DecisionKind::WholeUiWaiver
            || !cycle
                .rounds
                .iter()
                .any(|link| link.round_id == waiver.round_id)
    }) {
        return Err(error(4, "Cycle waiver records changed"));
    }
    Ok(Some(cycle))
}

pub fn load_round(
    lock: &CycleLock,
    cycle: &Cycle,
    path: &Path,
) -> Result<(Snapshot, Round), StoreError> {
    let snapshot = lock
        .read(path)?
        .ok_or_else(|| error(4, "Round state is missing"))?;
    let round: Round = serde_json::from_slice(&snapshot.bytes).map_err(|e| error(4, e))?;
    let n = ordinal(&round.round_id)?;
    if round.schema_version != 1
        || round.revision != snapshot.revision
        || round.cycle_id != cycle.cycle_id
        || round.prefix != cycle.prefix
        || path != round_path(&cycle.prefix, n)
        || !cycle
            .rounds
            .iter()
            .any(|link| link.round_id == round.round_id && Path::new(&link.path) == path)
        || round.full != round.matrix.full
        || round.scope != round.matrix.scope
        || round.target_fingerprint != round.target_snapshot.fingerprint
        || round.target_snapshot.algorithm != "phoenix-ui-target-v1"
        || round.target_snapshot.source.path != cycle.target
    {
        return Err(error(4, "Round identity, target or matrix binding changed"));
    }
    round.matrix.validate().map_err(|e| error(4, e.message))?;
    round
        .tested_version
        .validate()
        .map_err(|e| error(4, e.message))?;
    if let Some(finalization) = &round.finalization {
        if finalization.status != "finalized"
            || Path::new(&finalization.result.path) != path.parent().unwrap().join("result.json")
        {
            return Err(error(4, "Finalization status or result binding is invalid"));
        }
        finalize::load_result(lock, cycle, &round, &mut Vec::new())?;
    }
    Ok((snapshot, round))
}

fn manual_history(root: &Path, prefix: &str) -> Result<bool, StoreError> {
    for suffix in [
        ".ui-review.md",
        ".ui-review.applied.md",
        ".ui-review.applied.json",
    ] {
        if checked(root, &format!("{prefix}{suffix}"))?.exists() {
            return Ok(true);
        }
    }
    let base = checked(root, prefix)?;
    let start = format!(
        "{}.ui-review.r",
        base.file_name().unwrap().to_string_lossy()
    );
    let entries = match fs::read_dir(base.parent().unwrap()) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(error(4, e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| error(4, e))?;
        let name = entry.file_name();
        if name
            .to_str()
            .and_then(|s| s.strip_prefix(&start))
            .and_then(|s| s.strip_suffix(".md"))
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()))
        {
            return Ok(true);
        }
    }
    let captures = checked(root, &format!("{prefix}.ui-captures"))?;
    Ok(captures.exists()
        && fs::read_dir(captures)
            .map_err(|e| error(4, e))?
            .next()
            .is_some())
}

fn original_sources(
    root: &Path,
    frozen: &target::TargetSnapshot,
    target_path: &str,
) -> Result<Vec<(PathBuf, String)>, StoreError> {
    let mut sources = vec![(relative(target_path)?, frozen.source_hash.clone())];
    for attachment in &frozen.attachments {
        if let Some(digest) = &attachment.sha256 {
            let path = attachment
                .source
                .strip_prefix(root)
                .map_err(|_| error(2, "External target sources require explicit registration"))?;
            sources.push((relative(&path.to_string_lossy())?, digest.clone()));
        }
    }
    Ok(sources)
}

fn freeze_files(
    root: &Path,
    path: &Path,
    target_path: &str,
    frozen: &target::TargetSnapshot,
) -> Result<(FrozenTarget, Vec<ReportFile>), StoreError> {
    let directory = path.parent().unwrap().join("target");
    let expectations_path = directory.join("expectations.md");
    let mut text = String::new();
    for section in &frozen.sections {
        text.push_str(&section.text);
        if !text.ends_with('\n') {
            text.push('\n');
        }
        for reference in &section.references {
            text.push_str(&format!(
                "\n[{}]: <{}> {:?}\n",
                reference.id, reference.destination, reference.title
            ));
        }
    }
    let expectations = FileEvidence {
        path: expectations_path.to_string_lossy().into_owned(),
        sha256: hash(text.as_bytes()),
    };
    let mut files = vec![ReportFile {
        path: expectations_path,
        expected_hash: None,
        bytes: text.into_bytes(),
    }];
    let mut attachments = Vec::new();
    for (index, attachment) in frozen.attachments.iter().enumerate() {
        let (source, copy) =
            if let (Some(digest), Some(bytes)) = (&attachment.sha256, &attachment.bytes) {
                let source = attachment.source.strip_prefix(root).map_err(|_| {
                    error(2, "External target sources require explicit registration")
                })?;
                let extension = Path::new(&attachment.logical_path)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .filter(|ext| ext.len() <= 10 && ext.bytes().all(|c| c.is_ascii_alphanumeric()))
                    .unwrap_or("bin");
                let copy_path =
                    directory.join(format!("files/{index:04}-{}.{}", &digest[..12], extension));
                let copy = FileEvidence {
                    path: copy_path.to_string_lossy().into_owned(),
                    sha256: digest.clone(),
                };
                files.push(ReportFile {
                    path: copy_path,
                    expected_hash: None,
                    bytes: bytes.clone(),
                });
                (
                    Some(FileEvidence {
                        path: source.to_string_lossy().into_owned(),
                        sha256: digest.clone(),
                    }),
                    Some(copy),
                )
            } else {
                (None, None)
            };
        attachments.push(FrozenAttachment {
            logical_path: attachment.logical_path.clone(),
            source,
            frozen: copy,
        });
    }
    Ok((
        FrozenTarget {
            algorithm: frozen.algorithm.into(),
            fingerprint: frozen.fingerprint.clone(),
            source: FileEvidence {
                path: target_path.into(),
                sha256: frozen.source_hash.clone(),
            },
            expectations,
            attachments,
            missing: frozen.missing.clone(),
        },
        files,
    ))
}

pub fn begin(project: &Path, options: BeginOptions) -> Result<Round, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    relative(&options.prefix)?;
    let target_path = checked(&root, &options.target)?;
    options.version.validate()?;
    if let Some(plan) = &options.plan {
        fs::read(checked(&root, plan)?).map_err(|e| error(4, e))?;
    }
    let index = relative(&format!("{}.ui-review.json", options.prefix))?;
    let lock = CycleLock::acquire_project(&root, &index)?;
    lock.recover_publication()?;
    let before = lock.read(&index)?;
    let mut cycle = match load_cycle(&lock, &options.prefix)? {
        Some(cycle) => {
            if cycle.target != options.target
                || (options.plan.is_some() && options.plan != cycle.plan)
            {
                return Err(error(
                    3,
                    "The cycle target or plan binding cannot be changed",
                ));
            }
            cycle
        }
        None => {
            if manual_history(&root, &options.prefix)? {
                return Err(error(
                    3,
                    "Manual review history requires an explicit verified import",
                ));
            }
            Cycle {
                schema_version: 1,
                mode: "managed".into(),
                rules_version: 1,
                cycle_id: cycle_id(&options.prefix),
                prefix: options.prefix.clone(),
                target: options.target.clone(),
                record_path: options
                    .plan
                    .is_none()
                    .then(|| format!("{}.ui-review.applied.md", options.prefix)),
                plan: options.plan.clone(),
                next_issue_id: 1,
                issues: Vec::new(),
                revision: 0,
                rounds: Vec::new(),
                waivers: Vec::new(),
                legacy: None,
            }
        }
    };
    let previous = cycle
        .rounds
        .last()
        .map(|link| load_round(&lock, &cycle, Path::new(&link.path)))
        .transpose()?;
    if let Some((_, previous)) = &previous {
        if previous.finalization.is_none() {
            return Err(error(
                3,
                format!(
                    "Continue {} at revision {}; the current draft cannot be skipped",
                    previous.round_id, previous.revision
                ),
            ));
        }
    }
    let legacy = legacy::load(&lock, &cycle, &mut Vec::new())?;
    let last = match (&previous, &legacy) {
        (Some((_, r)), _) => ordinal(&r.round_id)?,
        (None, Some(map)) => ordinal(&map.last().round_id)?,
        (None, None) => 0,
    };
    let n = last
        .checked_add(1)
        .ok_or_else(|| error(2, "Round number limit reached"))?;
    let path = round_path(&options.prefix, n);
    let frozen = target::freeze(&target_path, &root, &[])?;
    let text = fs::read_to_string(&target_path).map_err(|e| error(2, e))?;
    let matrix = target_matrix::parse(&text, options.scope.clone())?;
    let sources = original_sources(&root, &frozen, &options.target)?;
    let (target_snapshot, mut files) = freeze_files(&root, &path, &options.target, &frozen)?;
    let threshold = options
        .threshold
        .or_else(|| previous.as_ref().map(|(_, r)| r.threshold))
        .or_else(|| legacy.as_ref().map(|map| map.threshold))
        .unwrap_or("8".parse().unwrap());
    // A baseline continues only while consecutive rounds keep the same target
    // fingerprint; any change starts a new baseline named by its first round.
    let baseline = match (&previous, &legacy) {
        (Some((_, old)), _) if old.target_fingerprint == frozen.fingerprint => old.baseline.clone(),
        (None, Some(map))
            if map
                .last()
                .target_fingerprint
                .as_ref()
                .is_some_and(|fingerprint| fingerprint.value == frozen.fingerprint) =>
        {
            map.baseline(&map.last().round_id).unwrap()
        }
        _ => format!("r{n}:{}", frozen.fingerprint),
    };
    let round = Round {
        schema_version: 1,
        revision: 1,
        cycle_id: cycle.cycle_id.clone(),
        prefix: cycle.prefix.clone(),
        round_id: format!("r{n}"),
        baseline,
        target_fingerprint: frozen.fingerprint,
        target_snapshot,
        scope: matrix.scope.clone(),
        full: matrix.full,
        matrix,
        threshold,
        tested_version: options.version,
        captures: Vec::new(),
        judges: Vec::new(),
        machine_checks: Vec::new(),
        packets: Vec::new(),
        finalization: None,
        capture_projection_hash: None,
    };
    files.push(ReportFile {
        path: path.clone(),
        expected_hash: None,
        bytes: encode(&round)?,
    });
    cycle.revision = cycle
        .revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Cycle revision limit reached"))?;
    cycle.rounds.push(RoundLink {
        round_id: round.round_id.clone(),
        path: path.to_string_lossy().into_owned(),
    });
    lock.prepare_draft(&index, before.as_ref(), &encode(&cycle)?, &files, &sources)?;
    lock.recover_publication()?;
    Ok(round)
}

fn prefix_from_round(path: &Path) -> Result<String, StoreError> {
    if path.file_name().and_then(|s| s.to_str()) != Some("round.json") {
        return Err(error(2, "Expected a round.json path"));
    }
    let round_dir = path
        .parent()
        .ok_or_else(|| error(2, "Round directory is missing"))?;
    ordinal(round_dir.file_name().and_then(|s| s.to_str()).unwrap_or(""))
        .map_err(|e| error(2, e.message))?;
    let captures = round_dir
        .parent()
        .ok_or_else(|| error(2, "Capture directory is missing"))?;
    let name = captures
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_suffix(".ui-captures"))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error(2, "Round must belong to a cycle capture directory"))?;
    let prefix = captures
        .parent()
        .unwrap_or(Path::new(""))
        .join(name)
        .to_string_lossy()
        .into_owned();
    relative(&prefix)?;
    Ok(prefix)
}

fn require_file(
    lock: &CycleLock,
    evidence: &FileEvidence,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<Vec<u8>, StoreError> {
    let path = relative(&evidence.path)?;
    let bytes = fs::read(lock.path(&path)?).map_err(|e| error(3, e))?;
    if hash(&bytes) != evidence.sha256 {
        return Err(error(
            3,
            format!("Registered evidence changed: {}", evidence.path),
        ));
    }
    sources.push((path, evidence.sha256.clone()));
    Ok(bytes)
}

fn verified_sources(
    root: &Path,
    lock: &CycleLock,
    cycle: &Cycle,
    round: &Round,
    path: &Path,
) -> Result<Vec<(PathBuf, String)>, StoreError> {
    let current = target::freeze(&checked(root, &cycle.target)?, root, &[])?;
    if current.fingerprint != round.target_fingerprint {
        return Err(error(3, "Target expectations or attachments changed; open a new baseline after the current round"));
    }
    let (expected, _) = freeze_files(root, path, &cycle.target, &current)?;
    if expected.expectations != round.target_snapshot.expectations
        || expected.attachments != round.target_snapshot.attachments
        || expected.missing != round.target_snapshot.missing
    {
        return Err(error(
            4,
            "Frozen target metadata does not match its expectations or attachments",
        ));
    }
    let mut sources = original_sources(root, &current, &cycle.target)?;
    let directory = path.parent().unwrap();
    if Path::new(&round.target_snapshot.expectations.path)
        != directory.join("target/expectations.md")
    {
        return Err(error(4, "Frozen expectation path changed"));
    }
    require_file(lock, &round.target_snapshot.expectations, &mut sources)?;
    for attachment in &round.target_snapshot.attachments {
        if let Some(file) = &attachment.frozen {
            if !relative(&file.path)?.starts_with(directory.join("target/files")) {
                return Err(error(4, "Frozen attachment is outside its round"));
            }
            require_file(lock, file, &mut sources)?;
        }
    }
    let context = CaptureContext {
        project: root,
        round_directory: directory,
        round_id: &round.round_id,
        matrix: &round.matrix,
        version: &round.tested_version,
    };
    let mut history = Vec::new();
    for entry in &round.captures {
        let verified = capture_evidence::verify(entry.input.clone(), &context)
            .map_err(|e| error(3, e.message))?;
        if verified.content_hash != entry.content_hash
            || verified.image_pixels != entry.image_pixels
        {
            return Err(error(4, "Capture content or pixel binding changed"));
        }
        if let Observation::Captured { image, .. } = &entry.input.observation {
            require_file(lock, image, &mut sources)?;
            let frozen = entry
                .frozen_image
                .as_ref()
                .ok_or_else(|| error(4, "Captured image snapshot is missing"))?;
            let filename = round
                .matrix
                .screenshot_name(&entry.input.cell, entry.input.attempt)
                .map_err(|e| error(4, e.message))?;
            if relative(&frozen.path)? != directory.join("evidence").join(filename)
                || image.sha256 != frozen.sha256
            {
                return Err(error(4, "Capture snapshot binding changed"));
            }
            require_file(lock, frozen, &mut sources)?;
        } else if entry.frozen_image.is_some() {
            return Err(error(4, "Missing captures cannot claim an image"));
        }
        if !append_capture(
            &round
                .matrix
                .cells
                .iter()
                .map(|cell| cell.key.clone())
                .collect::<Vec<_>>(),
            &mut history,
            capture_attempt(entry),
            false,
        )
        .map_err(|e| error(4, e))?
        {
            return Err(error(4, "Round contains duplicate capture registrations"));
        }
    }
    Ok(sources)
}

fn capture_attempt(entry: &RegisteredCapture) -> CaptureAttempt {
    CaptureAttempt {
        cell: entry.input.cell.clone(),
        attempt: entry.input.attempt,
        content_hash: entry.content_hash.clone(),
        status: match &entry.input.observation {
            Observation::Missing { reason } => CaptureStatus::Missing {
                reason: reason.clone(),
            },
            Observation::Captured { .. } => CaptureStatus::Captured,
        },
    }
}

fn table_text(text: &str) -> String {
    text.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn capture_body(round: &Round) -> Result<Vec<u8>, StoreError> {
    let mut body = format!("# {} · 采集清单\n\n- 目标指纹：{} · {}\n- 被测版本：{}\n- 每格只以最新 attempt 计算覆盖；历史图片保留为取证记录。\n\n| 单元 | 状态 | 视口 | 主题 | 步骤 | attempt | 有效记录 | 图像/缺项 | 出处与核对 |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- |\n", round.round_id, round.target_snapshot.algorithm, round.target_fingerprint, table_text(&String::from_utf8(encode(&round.tested_version)?).unwrap()));
    for entry in &round.captures {
        let cell = &entry.input.cell;
        let current = !round
            .captures
            .iter()
            .any(|other| other.input.cell == *cell && other.input.attempt > entry.input.attempt);
        let details = match &entry.input.observation {
            Observation::Missing { reason } => format!("missing | {}", table_text(reason)),
            Observation::Captured {
                tool,
                address,
                taken_at,
                human_check,
                interactions,
                ..
            } => format!(
                "{} | {} · {} · {} · {} · {}",
                entry.frozen_image.as_ref().unwrap().path,
                table_text(tool),
                table_text(address),
                table_text(taken_at),
                table_text(&human_check.notes),
                table_text(&serde_json::to_string(interactions).map_err(|e| error(2, e))?)
            ),
        };
        body.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            cell.unit_id,
            cell.state_id,
            cell.viewport_id,
            cell.theme_id,
            cell.step_id,
            entry.input.attempt,
            if current { "当前" } else { "历史" },
            details
        ));
    }
    for cell in &round.matrix.cells {
        if !round
            .captures
            .iter()
            .any(|entry| entry.input.cell == cell.key)
        {
            body.push_str(&format!(
                "\n- 未采：{} / {} / {} / {} / {}\n",
                cell.key.unit_id,
                cell.key.state_id,
                cell.key.viewport_id,
                cell.key.theme_id,
                cell.key.step_id
            ));
        }
    }
    Ok(body.into_bytes())
}

pub fn add_capture(
    project: &Path,
    path: &Path,
    expected_revision: u64,
    mut input: CaptureInput,
) -> Result<Round, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let prefix = prefix_from_round(path)?;
    relative(&path.to_string_lossy())?;
    let lock = CycleLock::acquire_project(&root, Path::new(&format!("{prefix}.ui-review.json")))?;
    lock.recover_publication()?;
    let cycle = load_cycle(&lock, &prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let (before, mut round) = load_round(&lock, &cycle, path)?;
    if before.revision != expected_revision || round.finalization.is_some() {
        return Err(error(
            3,
            "Round revision changed or the round is already finalized",
        ));
    }
    let mut sources = verified_sources(&root, &lock, &cycle, &round, path)?;
    if let Observation::Captured { address, .. } = &mut input.observation {
        let mut url = url::Url::parse(address).map_err(|e| error(2, e))?;
        if !url.username().is_empty() || url.password().is_some() {
            return Err(error(2, "Capture addresses must not contain credentials"));
        }
        url.set_query(None);
        *address = url.into();
    }
    let verified = capture_evidence::verify(
        input,
        &CaptureContext {
            project: &root,
            round_directory: path.parent().unwrap(),
            round_id: &round.round_id,
            matrix: &round.matrix,
            version: &round.tested_version,
        },
    )?;
    let mut entry = RegisteredCapture {
        input: verified.input,
        content_hash: verified.content_hash,
        image_pixels: verified.image_pixels,
        frozen_image: None,
    };
    let mut history = round
        .captures
        .iter()
        .map(capture_attempt)
        .collect::<Vec<_>>();
    let changed = append_capture(
        &round
            .matrix
            .cells
            .iter()
            .map(|cell| cell.key.clone())
            .collect::<Vec<_>>(),
        &mut history,
        capture_attempt(&entry),
        false,
    )
    .map_err(|e| error(2, e))?;
    let projection_path = path.parent().unwrap().join("capture.md");
    let existing = match fs::read(lock.path(&projection_path)?) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error(4, e)),
    };
    let block_id = format!("{}-{}-capture", cycle.cycle_id, round.round_id);
    let mut files = Vec::new();
    if changed {
        if let Observation::Captured { image, .. } = &entry.input.observation {
            let bytes = require_file(&lock, image, &mut sources)?;
            let copy_path = path
                .parent()
                .unwrap()
                .join("evidence")
                .join(Path::new(&image.path).file_name().unwrap());
            entry.frozen_image = Some(FileEvidence {
                path: copy_path.to_string_lossy().into_owned(),
                sha256: image.sha256.clone(),
            });
            files.push(ReportFile {
                path: copy_path,
                expected_hash: None,
                bytes,
            });
        }
        round.captures.push(entry);
    }
    let projection = project_block(
        existing.as_deref().unwrap_or_default(),
        &block_id,
        round.capture_projection_hash.as_deref(),
        &capture_body(&round)?,
    )?;
    if !changed {
        return Ok(round);
    }
    round.capture_projection_hash = Some(projection.block_hash);
    round.revision = round
        .revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Round revision limit reached"))?;
    files.push(ReportFile {
        path: projection_path,
        expected_hash: existing.as_deref().map(hash),
        bytes: projection.bytes,
    });
    lock.prepare_draft(path, Some(&before), &encode(&round)?, &files, &sources)?;
    lock.recover_publication()?;
    Ok(round)
}

/// Cells whose latest attempt is not a registered image (never captured or missing).
pub fn missing_cells(round: &Round) -> Vec<crate::capture::CellKey> {
    round
        .matrix
        .cells
        .iter()
        .filter(|cell| {
            !round
                .captures
                .iter()
                .filter(|capture| capture.input.cell == cell.key)
                .max_by_key(|capture| capture.input.attempt)
                .is_some_and(|capture| {
                    matches!(capture.input.observation, Observation::Captured { .. })
                })
        })
        .map(|cell| cell.key.clone())
        .collect()
}

pub fn round_path(prefix: &str, number: u32) -> PathBuf {
    PathBuf::from(format!("{prefix}.ui-captures/r{number}/round.json"))
}

/// The round report keeps the Markdown loop's names: the first round is
/// `<prefix>.ui-review.md`, later rounds `<prefix>.ui-review.r<n>.md`.
pub fn report_path(prefix: &str, round_id: &str) -> String {
    if round_id == "r1" {
        format!("{prefix}.ui-review.md")
    } else {
        format!("{prefix}.ui-review.{round_id}.md")
    }
}
