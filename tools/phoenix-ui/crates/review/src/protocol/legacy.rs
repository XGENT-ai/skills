//! `review import`: explicit takeover of legacy Markdown history, and re-basing a
//! hand-edited generated block after the host saved the manual text. Unknown
//! legacy counts stay unknown; every input keeps its hash and a legacy marker.
use super::*;
use crate::policy::{
    Assessment, Disposition, GapStatus, PriorDisposition, RefactorHistory, Severity,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub const MARKER: &str = "legacy-markdown";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Knowledge {
    Known,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyScore {
    pub state: Knowledge,
    #[serde(default)]
    pub value: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyDisposition {
    pub state: Knowledge,
    #[serde(default)]
    pub disposition: Option<Disposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyFingerprint {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyRound {
    pub round_id: String,
    pub report: FileEvidence,
    pub target_fingerprint: Option<LegacyFingerprint>,
    pub full_score: LegacyScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyAssessment {
    pub round_id: String,
    pub status: GapStatus,
    pub prior_disposition: LegacyDisposition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyGap {
    pub id: String,
    pub unit_id: String,
    pub severity: Severity,
    pub origin: finalize::GapOrigin,
    pub assessments: Vec<LegacyAssessment>,
    pub since_last_assessment: LegacyDisposition,
    pub pending_decision: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyUnitRefactor {
    pub unit_id: String,
    pub refactor: RefactorHistory,
}

/// Refactor counts of the last legacy baseline; every target unit is explicit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyRefactors {
    pub global: RefactorHistory,
    pub units: Vec<LegacyUnitRefactor>,
}

/// The unit's nearest covering legacy round: a unit-only judge excerpt and its images.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyMaterial {
    pub unit_id: String,
    pub round_id: String,
    pub judge: FileEvidence,
    pub images: Vec<FileEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyMap {
    pub kind: String,
    pub schema_version: u32,
    pub prefix: String,
    pub target: String,
    pub plan: Option<String>,
    pub record_path: Option<String>,
    pub threshold: Decimal,
    pub rounds: Vec<LegacyRound>,
    pub gaps: Vec<LegacyGap>,
    pub refactors: LegacyRefactors,
    pub units: Vec<LegacyMaterial>,
    pub sources: Vec<FileEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rebase {
    pub kind: String,
    pub schema_version: u32,
    pub prefix: String,
    pub cycle_revision: u64,
    pub file: String,
    pub current_sha256: String,
    pub saved_copy: FileEvidence,
}

/// Recorded in the cycle index: the original input and its frozen copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyRef {
    pub marker: String,
    pub input: FileEvidence,
    pub map: FileEvidence,
    pub last_round: String,
}

#[derive(Deserialize)]
struct Kind {
    kind: String,
}

pub fn map_path(prefix: &str) -> String {
    format!("{prefix}.ui-review.legacy/map.json")
}

impl LegacyMap {
    pub(super) fn round(&self, id: &str) -> Option<&LegacyRound> {
        self.rounds.iter().find(|round| round.round_id == id)
    }

    fn fingerprint(&self, id: &str) -> Option<&str> {
        self.round(id)
            .and_then(|round| round.target_fingerprint.as_ref())
            .map(|fingerprint| fingerprint.value.as_str())
    }

    /// Baselines run over consecutive rounds with one known v1 fingerprint; an
    /// unknown fingerprint cannot be compared and stands alone.
    pub(super) fn baseline(&self, id: &str) -> Option<String> {
        let index = self.rounds.iter().position(|round| round.round_id == id)?;
        let Some(value) = self.fingerprint(id) else {
            return Some(format!("{id}:unknown"));
        };
        let mut first = index;
        while first > 0 && self.fingerprint(&self.rounds[first - 1].round_id) == Some(value) {
            first -= 1;
        }
        Some(format!("{}:{value}", self.rounds[first].round_id))
    }

    pub(super) fn last(&self) -> &LegacyRound {
        self.rounds.last().unwrap()
    }

    pub(super) fn refactor(
        &self,
        baseline: &str,
        scope: &applied::RefactorScope,
    ) -> RefactorHistory {
        if self.baseline(&self.last().round_id).as_deref() != Some(baseline) {
            return RefactorHistory::NotRecorded;
        }
        match scope {
            applied::RefactorScope::Global => self.refactors.global,
            applied::RefactorScope::Unit { unit_id } => self
                .refactors
                .units
                .iter()
                .find(|unit| unit.unit_id == *unit_id)
                .map_or(RefactorHistory::Unknown, |unit| unit.refactor),
        }
    }

    /// Full scores earlier in `baseline`; `Err` when one is unknown.
    pub(super) fn scores(&self, baseline: &str) -> Result<Vec<Decimal>, StoreError> {
        let mut scores = Vec::new();
        for round in &self.rounds {
            if self.baseline(&round.round_id).as_deref() != Some(baseline) {
                continue;
            }
            match round.full_score.state {
                Knowledge::Known => scores.push(round.full_score.value.unwrap()),
                Knowledge::Absent => (),
                Knowledge::Unknown => {
                    return Err(error(
                        3,
                        "Legacy full-score history in this baseline is unknown; continue manually or confirm a new baseline",
                    ))
                }
            }
        }
        Ok(scores)
    }

    pub(super) fn open_gaps(&self) -> impl Iterator<Item = &LegacyGap> {
        self.gaps
            .iter()
            .filter(|gap| !finalize::closed(gap.assessments.last().unwrap().status))
    }

    pub(super) fn gap(&self, id: &str) -> Option<&LegacyGap> {
        self.gaps.iter().find(|gap| gap.id == id)
    }

    pub(super) fn is_legacy_round(&self, id: &str) -> bool {
        self.round(id).is_some()
    }

    pub(super) fn unknowns(&self) -> Vec<String> {
        let mut unknowns = Vec::new();
        for round in &self.rounds {
            if round.target_fingerprint.is_none() {
                unknowns.push(format!("{} target fingerprint", round.round_id));
            }
            if round.full_score.state == Knowledge::Unknown {
                unknowns.push(format!("{} full score", round.round_id));
            }
        }
        for gap in &self.gaps {
            if gap.since_last_assessment.state == Knowledge::Unknown
                || gap
                    .assessments
                    .iter()
                    .any(|a| a.prior_disposition.state == Knowledge::Unknown)
            {
                unknowns.push(format!("{} attempt history", gap.id));
            }
        }
        if self.refactors.global == RefactorHistory::Unknown {
            unknowns.push("global refactor count".into());
        }
        for unit in &self.refactors.units {
            if unit.refactor == RefactorHistory::Unknown {
                unknowns.push(format!("{} refactor count", unit.unit_id));
            }
        }
        unknowns
    }
}

impl LegacyGap {
    /// The canonical gap as carried into the first managed round.
    pub(super) fn to_gap(&self) -> finalize::Gap {
        let history: Vec<Assessment> = self
            .assessments
            .iter()
            .map(|assessment| Assessment {
                status: assessment.status,
                // A legacy assessment records one disposition before it.
                prior_disposition: PriorDisposition {
                    attempted: assessment.prior_disposition.disposition
                        == Some(Disposition::Modified),
                    latest: assessment.prior_disposition.disposition,
                },
                disposition_unknown: assessment.prior_disposition.state == Knowledge::Unknown,
            })
            .collect();
        let latest = self.assessments.last().unwrap();
        finalize::Gap {
            id: self.id.clone(),
            unit_id: self.unit_id.clone(),
            severity: self.severity,
            status: latest.status,
            details: Vec::new(),
            runtime_findings: Vec::new(),
            history,
            refactor: RefactorHistory::NotRecorded,
            pending_decision: self.pending_decision,
            recaptured: false,
            origin: self.origin,
            assessed_round: self
                .assessments
                .iter()
                .rev()
                .find(|assessment| assessment.status != GapStatus::NotReviewed)
                .map(|assessment| assessment.round_id.clone()),
        }
    }
}

/// Loads the frozen legacy map of an imported cycle; drift is a conflict.
pub(super) fn load(
    lock: &CycleLock,
    cycle: &Cycle,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<Option<LegacyMap>, StoreError> {
    let Some(reference) = &cycle.legacy else {
        return Ok(None);
    };
    let bytes = require_file(lock, &reference.map, sources)?;
    let map: LegacyMap = serde_json::from_slice(&bytes).map_err(|e| error(4, e))?;
    if map.kind != "legacyMarkdown"
        || map.schema_version != 1
        || map.prefix != cycle.prefix
        || map.rounds.last().map(|round| &round.round_id) != Some(&reference.last_round)
    {
        return Err(error(4, "Frozen legacy map does not match its cycle"));
    }
    Ok(Some(map))
}

fn read_evidence(root: &Path, evidence: &FileEvidence) -> Result<Vec<u8>, StoreError> {
    let path = checked(root, &evidence.path)?;
    let bytes = fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => error(
            2,
            format!("Legacy mapping names a missing file: {}", evidence.path),
        ),
        _ => error(4, e),
    })?;
    if hash(&bytes) != evidence.sha256 {
        return Err(error(
            3,
            format!("Legacy file changed since it was mapped: {}", evidence.path),
        ));
    }
    Ok(bytes)
}

fn knowledge_ok(state: Knowledge, present: bool) -> bool {
    (state == Knowledge::Known) == present
}

pub fn run(project: &Path, prefix: &str, input: &str, bytes: &[u8]) -> Result<Value, StoreError> {
    relative(prefix)?;
    relative(input)?;
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let kind: Kind = serde_json::from_slice(bytes)
        .map_err(|e| error(2, format!("Import input needs a kind: {e}")))?;
    let input = FileEvidence {
        path: input.into(),
        sha256: hash(bytes),
    };
    match kind.kind.as_str() {
        "legacyMarkdown" => {
            let map: LegacyMap = serde_json::from_slice(bytes)
                .map_err(|e| error(2, format!("Legacy map does not match the schema: {e}")))?;
            import_legacy(&root, prefix, input, bytes, map)
        }
        "projectionRebase" => {
            let rebase: Rebase = serde_json::from_slice(bytes).map_err(|e| {
                error(
                    2,
                    format!("Projection rebase does not match the schema: {e}"),
                )
            })?;
            rebase_projection(&root, prefix, rebase)
        }
        _ => Err(error(
            2,
            "Import kind must be legacyMarkdown or projectionRebase",
        )),
    }
}

fn import_legacy(
    root: &Path,
    prefix: &str,
    input: FileEvidence,
    bytes: &[u8],
    map: LegacyMap,
) -> Result<Value, StoreError> {
    if map.schema_version != 1 || map.prefix != prefix {
        return Err(error(
            2,
            "Legacy map schema or prefix does not match --prefix",
        ));
    }
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    let lock = CycleLock::acquire_project(root, &index)?;
    lock.recover_publication()?;
    if lock.read(&index)?.is_some() {
        return Err(error(
            3,
            "This cycle is already managed; history is not re-imported",
        ));
    }
    if !manual_history(root, prefix)? {
        return Err(error(
            2,
            "No legacy Markdown history exists for this prefix; begin a managed cycle instead",
        ));
    }
    let mut sources = vec![(relative(&input.path)?, input.sha256.clone())];
    let mut verify = |evidence: &FileEvidence| -> Result<Vec<u8>, StoreError> {
        let bytes = read_evidence(root, evidence)?;
        sources.push((relative(&evidence.path)?, evidence.sha256.clone()));
        Ok(bytes)
    };
    let target = checked(root, &map.target)?;
    let text = fs::read_to_string(&target).map_err(|e| {
        error(
            2,
            format!("Target package {} is unreadable: {e}", map.target),
        )
    })?;
    let matrix = target_matrix::parse(&text, None)?;
    let units: BTreeSet<&str> = matrix
        .units
        .iter()
        .map(|unit| unit.identity.id.as_str())
        .collect();
    if let Some(plan) = &map.plan {
        fs::read(checked(root, plan)?).map_err(|e| error(2, e))?;
    }
    if let Some(record) = &map.record_path {
        if !record.ends_with(".md") {
            return Err(error(
                2,
                "The legacy repair record location must be Markdown",
            ));
        }
        relative(record)?;
    }
    if map.rounds.is_empty() {
        return Err(error(2, "Legacy map needs its rounds"));
    }
    for (n, round) in map.rounds.iter().enumerate() {
        if round.round_id != format!("r{}", n + 1)
            || !knowledge_ok(round.full_score.state, round.full_score.value.is_some())
            || round
                .target_fingerprint
                .as_ref()
                .is_some_and(|fingerprint| {
                    fingerprint.algorithm != "phoenix-ui-target-v1"
                        || fingerprint.value.len() != 64
                        || !fingerprint
                            .value
                            .bytes()
                            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
                })
        {
            return Err(error(
                2,
                "Legacy rounds must be r1..rN with v1 fingerprints (or unknown) and consistent full-score knowledge",
            ));
        }
        verify(&round.report)?;
    }
    // An unknown full score in the baseline a managed round would continue leaves
    // its historical best ambiguous; the cycle stays manual.
    if let Some(baseline) = map.baseline(&map.last().round_id) {
        map.scores(&baseline)?;
    }
    let mut ids = BTreeSet::new();
    let mut next_issue_id = 1;
    for gap in &map.gaps {
        let number = judgement::issue_number(&gap.id);
        let mut previous = 0;
        let ordered = gap.assessments.iter().all(|assessment| {
            let n = ordinal(&assessment.round_id).unwrap_or(0);
            let ok = n > previous
                && map.round(&assessment.round_id).is_some()
                && knowledge_ok(
                    assessment.prior_disposition.state,
                    assessment.prior_disposition.disposition.is_some(),
                );
            previous = n;
            ok
        });
        if number.is_none()
            || !ids.insert(gap.id.as_str())
            || !units.contains(gap.unit_id.as_str())
            || gap.assessments.is_empty()
            || !ordered
            || !knowledge_ok(
                gap.since_last_assessment.state,
                gap.since_last_assessment.disposition.is_some(),
            )
        {
            return Err(error(
                2,
                "Legacy gaps need unique UI-NN IDs, target units and ordered, explicit assessments",
            ));
        }
        next_issue_id = next_issue_id.max(number.unwrap() + 1);
    }
    let listed: Vec<&str> = map
        .refactors
        .units
        .iter()
        .map(|unit| unit.unit_id.as_str())
        .collect();
    if listed.len() != units.len() || listed.iter().copied().collect::<BTreeSet<_>>() != units {
        return Err(error(
            2,
            "Legacy refactor counts must state every target unit explicitly (recorded, not_recorded or unknown)",
        ));
    }
    let mut material = BTreeSet::new();
    for unit in &map.units {
        if !units.contains(unit.unit_id.as_str())
            || !material.insert(unit.unit_id.as_str())
            || map.round(&unit.round_id).is_none()
        {
            return Err(error(
                2,
                "Legacy unit material must name each target unit once",
            ));
        }
        verify(&unit.judge)?;
        for image in &unit.images {
            if !matches!(
                Path::new(&image.path)
                    .extension()
                    .and_then(|ext| ext.to_str()),
                Some("png" | "jpg" | "jpeg" | "gif" | "webp")
            ) {
                return Err(error(2, "Legacy unit images must be image files"));
            }
            verify(image)?;
        }
    }
    if map.open_gaps().any(|gap| {
        gap.origin == finalize::GapOrigin::Reviewer && !material.contains(gap.unit_id.as_str())
    }) {
        return Err(error(
            2,
            "Open legacy gaps need their unit's previous judge material before reviewers can reassess them",
        ));
    }
    for source in &map.sources {
        verify(source)?;
    }
    let frozen = FileEvidence {
        path: map_path(prefix),
        sha256: hash(bytes),
    };
    if lock.path(&relative(&frozen.path)?)?.exists() {
        return Err(error(
            3,
            "A frozen legacy map already exists for this prefix",
        ));
    }
    let cycle = Cycle {
        schema_version: 1,
        mode: "managed".into(),
        rules_version: 1,
        cycle_id: cycle_id(prefix),
        prefix: prefix.into(),
        target: map.target.clone(),
        plan: map.plan.clone(),
        record_path: map.record_path.clone(),
        next_issue_id,
        issues: map
            .gaps
            .iter()
            .map(|gap| judgement::Issue {
                id: gap.id.clone(),
                unit_id: gap.unit_id.clone(),
                first_round: gap.assessments[0].round_id.clone(),
            })
            .collect(),
        revision: 1,
        rounds: Vec::new(),
        waivers: Vec::new(),
        legacy: Some(LegacyRef {
            marker: MARKER.into(),
            input,
            map: frozen.clone(),
            last_round: map.last().round_id.clone(),
        }),
    };
    lock.prepare_draft(
        &index,
        None,
        &encode(&cycle)?,
        &[ReportFile {
            path: relative(&frozen.path)?,
            expected_hash: None,
            bytes: bytes.to_vec(),
        }],
        &sources,
    )?;
    lock.recover_publication()?;
    Ok(
        json!({"kind": "legacyMarkdown", "cycleId": cycle.cycle_id, "cycleRevision": cycle.revision,
        "legacyRounds": map.rounds.len(), "nextIssueId": next_issue_id,
        "frozenMap": frozen, "unknowns": map.unknowns()}),
    )
}

fn rebase_projection(root: &Path, prefix: &str, input: Rebase) -> Result<Value, StoreError> {
    if input.schema_version != 1 || input.prefix != prefix {
        return Err(error(
            2,
            "Projection rebase schema or prefix does not match --prefix",
        ));
    }
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    let lock = CycleLock::acquire_project(root, &index)?;
    lock.recover_publication()?;
    let index_before = lock
        .read(&index)?
        .ok_or_else(|| error(2, "No managed cycle exists for this prefix"))?;
    let mut cycle = load_cycle(&lock, prefix)?
        .ok_or_else(|| error(2, "No managed cycle exists for this prefix"))?;
    if cycle.revision != input.cycle_revision {
        return Err(error(
            3,
            "Cycle revision changed; reload status before re-basing",
        ));
    }
    let file = relative(&input.file)?;
    let current = fs::read(lock.path(&file)?).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => error(2, "The projection file does not exist"),
        _ => error(4, e),
    })?;
    if hash(&current) != input.current_sha256 {
        return Err(error(
            3,
            "The projection changed after its manual text was saved",
        ));
    }
    let saved = relative(&input.saved_copy.path)?;
    let saved_bytes = match fs::read(lock.path(&saved)?) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(error(
                2,
                "Save the manual text before re-basing its generated block",
            ))
        }
        Err(e) => return Err(error(4, e)),
    };
    if saved == file || saved_bytes != current || input.saved_copy.sha256 != input.current_sha256 {
        return Err(error(
            2,
            "The saved copy must be a separate file with the exact current bytes",
        ));
    }
    let mut sources = vec![(saved, input.saved_copy.sha256.clone())];
    let short = &cycle.cycle_id[..16];
    let path = input.file.as_str();
    let rounds: Vec<Round> = cycle
        .rounds
        .iter()
        .map(|link| load_round(&lock, &cycle, Path::new(&link.path)).map(|(_, round)| round))
        .collect::<Result<_, _>>()?;
    let applied = applied::load(&lock, &cycle, &mut sources)?;
    let sealed_report = rounds
        .iter()
        .find(|round| round.finalization.is_some() && path == report_path(prefix, &round.round_id));
    let (bytes, restored) = if let Some(round) = sealed_report {
        // Loading the round verified the sealed text; only an appended waiver
        // block in it is generated, and that block is restored from the cycle.
        let Some(block) = finalize::waiver_block(&cycle, &round.round_id)? else {
            return Err(error(
                3,
                "Sealed review history cannot be re-based or imported",
            ));
        };
        let (sealed, _) = finalize::split_report(&current, &cycle.cycle_id, &round.round_id)?;
        let bytes = finalize::with_waiver_block(&sealed, Some(&block));
        let restored = usize::from(bytes != current);
        (bytes, restored)
    } else {
        let mut blocks: Vec<(String, Vec<u8>, String)> = Vec::new();
        if let Some(ledger) = applied
            .ledger
            .as_ref()
            .filter(|ledger| ledger.record_path == path)
        {
            blocks.push((
                format!("apply-{short}"),
                applied::body(
                    &applied.records,
                    &cycle,
                    &ledger.record_path,
                    &applied.verdicts,
                )?,
                ledger.projection_hash.clone(),
            ));
        } else if let Some(round) = rounds.last().filter(|round| {
            round.finalization.is_none()
                && Path::new(path)
                    == round_path(prefix, ordinal(&round.round_id).unwrap_or(0))
                        .parent()
                        .unwrap()
                        .join("capture.md")
        }) {
            if let Some(expected) = &round.capture_projection_hash {
                blocks.push((
                    format!("{}-{}-capture", cycle.cycle_id, round.round_id),
                    capture_body(round)?,
                    expected.clone(),
                ));
            }
        } else if path == format!("{prefix}.ui-review.md")
            || path.starts_with(&format!("{prefix}.ui-review.r"))
            || path.starts_with(&format!("{prefix}.ui-captures/"))
        {
            return Err(error(
                3,
                "Sealed review history cannot be re-based or imported",
            ));
        } else {
            return Err(error(
                2,
                "The file is not a generated projection of this cycle",
            ));
        }
        let mut bytes = current.clone();
        let mut restored = 0;
        for (block, body, expected) in &blocks {
            if project_block(&bytes, block, Some(expected), body).is_ok() {
                continue;
            }
            let projection = crate::projection::rebase_block(&bytes, block, body)?;
            if projection.block_hash != *expected {
                return Err(error(
                    4,
                    "Regenerated block does not match its recorded state",
                ));
            }
            bytes = projection.bytes;
            restored += 1;
        }
        (bytes, restored)
    };
    if restored == 0 {
        return Ok(
            json!({"kind": "projectionRebase", "file": path, "restoredBlocks": 0,
            "cycleRevision": cycle.revision, "changed": false}),
        );
    }
    cycle.revision = cycle
        .revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Cycle revision limit reached"))?;
    lock.prepare_draft(
        &index,
        Some(&index_before),
        &encode(&cycle)?,
        &[ReportFile {
            path: file,
            expected_hash: Some(input.current_sha256.clone()),
            bytes,
        }],
        &sources,
    )?;
    lock.recover_publication()?;
    Ok(
        json!({"kind": "projectionRebase", "file": path, "restoredBlocks": restored,
        "cycleRevision": cycle.revision, "changed": true,
        "savedCopy": input.saved_copy}),
    )
}
