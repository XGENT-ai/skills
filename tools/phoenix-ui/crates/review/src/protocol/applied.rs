use super::*;
use crate::policy::Disposition;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelfCheck {
    Page {
        capture: Box<CaptureInput>,
        notes: String,
    },
    NotRun {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub issue_id: String,
    pub verification: String,
    pub disposition: Disposition,
    pub changes: Vec<FileEvidence>,
    pub basis: Vec<FileEvidence>,
    pub self_check: SelfCheck,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum RefactorScope {
    Global,
    Unit {
        #[serde(rename = "unitId")]
        unit_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Refactor {
    pub scope: RefactorScope,
    pub hypothesis: String,
    pub change: String,
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandCheck {
    pub command: String,
    pub status: finalize::CheckStatus,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub schema_version: u32,
    pub cycle_id: String,
    pub round_id: String,
    pub record_path: Option<String>,
    pub at: String,
    pub tested_version: TestedVersion,
    pub items: Vec<Item>,
    pub restructure_scopes: Vec<Refactor>,
    pub decisions: Vec<finalize::Decision>,
    pub checks: Vec<CommandCheck>,
    pub unfinished: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageEvidence {
    pub issue_id: String,
    pub input: CaptureInput,
    pub frozen_image: FileEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    pub schema_version: u32,
    pub revision: u64,
    pub cycle_id: String,
    pub round_id: String,
    pub baseline: String,
    pub source_result: FileEvidence,
    pub input: Input,
    pub page_evidence: Vec<PageEvidence>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ledger {
    pub schema_version: u32,
    pub cycle_id: String,
    pub revision: u64,
    pub record_path: String,
    pub projection_hash: String,
    pub records: Vec<FileEvidence>,
}

pub(super) struct Loaded {
    pub before: Option<Snapshot>,
    pub ledger: Option<Ledger>,
    pub records: Vec<Record>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub round_id: String,
    pub revision: u64,
    pub record: FileEvidence,
    pub record_path: String,
}

pub fn record(
    project: &Path,
    path: &Path,
    expected_revision: u64,
    mut input: Input,
) -> Result<Receipt, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let prefix = prefix_from_round(path)?;
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    let applied = relative(&format!("{prefix}.ui-review.applied.json"))?;
    let lock = CycleLock::acquire_project(&root, &index)?;
    lock.recover_publication()?;
    let index_before = lock
        .read(&index)?
        .ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let mut cycle =
        load_cycle(&lock, &prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let (round_before, round) = load_round(&lock, &cycle, path)?;
    if cycle
        .rounds
        .last()
        .is_none_or(|link| link.round_id != round.round_id)
        || round.finalization.is_none()
    {
        return Err(error(
            3,
            "Only the latest finalized round can receive repair records",
        ));
    }
    let mut sources = Vec::new();
    let result = finalize::load_result(&lock, &cycle, &round, &mut sources)?;
    let Loaded {
        before,
        ledger: previous,
        records,
    } = load(&lock, &cycle, &mut sources)?;
    if previous.as_ref().map_or(0, |ledger| ledger.revision) != expected_revision {
        return Err(error(
            3,
            "Applied revision changed; resume the existing repair record",
        ));
    }
    if input.schema_version != 1
        || input.cycle_id != cycle.cycle_id
        || input.round_id != round.round_id
        || !capture_evidence::timestamp(&input.at)
        || !input.tested_version.known()
        || input.items.is_empty()
        || input.unfinished.iter().any(|s| s.trim().is_empty())
        || input
            .checks
            .iter()
            .any(|c| c.command.trim().is_empty() || c.summary.trim().is_empty())
    {
        return Err(error(
            2,
            "Applied records need this cycle/round, a known version, actual time and dispositions",
        ));
    }
    let record_path = cycle
        .record_path
        .clone()
        .or_else(|| input.record_path.clone())
        .ok_or_else(|| {
            error(
                2,
                "Select the plan's UI repair record location before the first disposition",
            )
        })?;
    if input
        .record_path
        .as_ref()
        .is_some_and(|p| p != &record_path)
    {
        return Err(error(
            3,
            "The cycle's repair record location cannot be changed",
        ));
    }
    let record_file = relative(&record_path)?;
    if record_file.extension().and_then(|s| s.to_str()) != Some("md")
        || record_path == cycle.target
        || record_path == report_path(&prefix, "r1")
        || cycle
            .rounds
            .iter()
            .any(|link| record_path == report_path(&prefix, &link.round_id))
        || round
            .target_snapshot
            .attachments
            .iter()
            .any(|a| a.source.as_ref().is_some_and(|f| f.path == record_path))
    {
        return Err(error(
            2,
            "Repair projections cannot replace a target or review report",
        ));
    }
    let revision = expected_revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Applied revision limit reached"))?;
    let directory = relative(&format!(
        "{prefix}.ui-review.applied/{}/rev{revision}",
        round.round_id
    ))?;
    let mut output = Vec::new();
    let mut page_evidence = Vec::new();
    let mut unique = BTreeSet::new();
    let round_gaps: BTreeMap<_, _> = result
        .units
        .iter()
        .flat_map(|u| &u.gaps)
        .chain(&result.carried)
        .map(|g| (g.id.as_str(), g))
        .collect();
    for item in &mut input.items {
        let gap = round_gaps
            .get(item.issue_id.as_str())
            .filter(|gap| !finalize::closed(gap.status))
            .ok_or_else(|| {
                error(
                    2,
                    "Dispositions only target open gaps in this round's report gap list",
                )
            })?;
        if !unique.insert(item.issue_id.clone())
            || item.verification.trim().is_empty()
            || item.notes.trim().is_empty()
        {
            return Err(error(
                2,
                "Each disposition needs a unique gap, verification and concrete notes",
            ));
        }
        if item.disposition == Disposition::Modified && item.changes.is_empty() {
            return Err(error(
                2,
                "Modified means actual changes with current file hashes",
            ));
        }
        if item.disposition != Disposition::Modified && !item.changes.is_empty() {
            return Err(error(
                2,
                "Unmodified dispositions cannot claim changed files",
            ));
        }
        let mut changed_paths = BTreeSet::new();
        for change in &item.changes {
            if !changed_paths.insert(&change.path)
                || change.path == cycle.target
                || change.path.starts_with(&format!("{prefix}.ui-review"))
                || change.path.starts_with(&format!("{prefix}.ui-captures/"))
            {
                return Err(error(
                    2,
                    "Changes must name unique product files, not review evidence",
                ));
            }
            require_file(&lock, change, &mut sources)?;
        }
        for basis in &item.basis {
            if basis != &round.target_snapshot.expectations
                && !round
                    .target_snapshot
                    .attachments
                    .iter()
                    .any(|a| a.frozen.as_ref() == Some(basis))
            {
                return Err(error(
                    2,
                    "Disposition counterevidence must cite the frozen target or accepted rules",
                ));
            }
            require_file(&lock, basis, &mut sources)?;
        }
        match &mut item.self_check {
            SelfCheck::NotRun { reason } => {
                if reason.trim().is_empty()
                    || (item.disposition == Disposition::Declined && item.basis.is_empty())
                {
                    return Err(error(
                        2,
                        "Not-run needs a reason; declined needs explicit counterevidence",
                    ));
                }
            }
            SelfCheck::Page { capture, notes } => {
                if notes.trim().is_empty()
                    || capture.cell.unit_id != gap.unit_id
                    || capture.round_id != round.round_id
                {
                    return Err(error(
                        2,
                        "Self-check must observe this gap's unit and round",
                    ));
                }
                let Observation::Captured { image, address, .. } = &mut capture.observation else {
                    return Err(error(2, "A missing image must be recorded as not-run"));
                };
                let image_source = relative(&image.path)?;
                let parent = image_source
                    .parent()
                    .ok_or_else(|| error(2, "Self-check image directory is missing"))?;
                if cycle
                    .rounds
                    .iter()
                    .any(|link| Path::new(&link.path).parent() == Some(parent))
                {
                    return Err(error(
                        2,
                        "Sealed review screenshots cannot be reused as current repair evidence",
                    ));
                }
                let mut url = url::Url::parse(address).map_err(|e| error(2, e))?;
                let _ = url.set_username("");
                let _ = url.set_password(None);
                url.set_query(None);
                url.set_fragment(None);
                *address = url.to_string();
                let image = image.clone();
                capture_evidence::verify(
                    capture.as_ref().clone(),
                    &CaptureContext {
                        project: &root,
                        round_directory: parent,
                        round_id: &round.round_id,
                        matrix: &round.matrix,
                        version: &input.tested_version,
                    },
                )?;
                let bytes = require_file(&lock, &image, &mut sources)?;
                let frozen = FileEvidence {
                    path: directory
                        .join("evidence")
                        .join(format!("{}.png", item.issue_id))
                        .to_string_lossy()
                        .into_owned(),
                    sha256: hash(&bytes),
                };
                output.push(ReportFile {
                    path: relative(&frozen.path)?,
                    expected_hash: None,
                    bytes,
                });
                page_evidence.push(PageEvidence {
                    issue_id: item.issue_id.clone(),
                    input: capture.as_ref().clone(),
                    frozen_image: frozen,
                });
            }
        }
    }
    let legacy = legacy::load(&lock, &cycle, &mut sources)?;
    let mut refactor_scopes = BTreeSet::new();
    for refactor in &input.restructure_scopes {
        let key = String::from_utf8(encode(&refactor.scope)?).unwrap();
        let unit = match &refactor.scope {
            RefactorScope::Global => None,
            RefactorScope::Unit { unit_id } => Some(unit_id),
        };
        if !refactor_scopes.insert(key)
            || [&refactor.hypothesis, &refactor.change, &refactor.expected]
                .iter()
                .any(|s| s.trim().is_empty())
            || !input.items.iter().any(|item| {
                item.disposition == Disposition::Modified
                    && unit.is_none_or(|unit| round_gaps[&item.issue_id.as_str()].unit_id == *unit)
            })
        {
            return Err(error(2, "A refactor needs one explicit scope, an actual change, hypothesis and expected improvement"));
        }
        if records
            .iter()
            .filter(|old| old.baseline == round.baseline)
            .any(|old| {
                old.input
                    .restructure_scopes
                    .iter()
                    .any(|r| r.scope == refactor.scope)
            })
            || legacy.as_ref().is_some_and(|map| {
                map.refactor(&round.baseline, &refactor.scope)
                    == crate::policy::RefactorHistory::Recorded
            })
        {
            return Err(error(
                3,
                "A refactor is already recorded for this baseline and scope",
            ));
        }
        if legacy.as_ref().is_some_and(|map| {
            map.refactor(&round.baseline, &refactor.scope)
                == crate::policy::RefactorHistory::Unknown
        }) {
            return Err(error(
                3,
                "The imported refactor count for this baseline and scope is unknown; no new refactor opportunity is assumed",
            ));
        }
    }
    for decision in &input.decisions {
        if decision.kind != finalize::DecisionKind::Direction {
            return Err(error(
                2,
                "Whole-UI waivers are appended by review finalize, not repair records",
            ));
        }
        let bytes = require_file(&lock, &decision.source, &mut sources)?;
        let text = std::str::from_utf8(&bytes).map_err(|e| error(2, e))?;
        if decision.quote.trim().is_empty()
            || !text.contains(&decision.quote)
            || decision.date.trim().is_empty()
            || decision.impact.trim().is_empty()
            || decision.tested_version != input.tested_version
            || decision.issue_ids.is_empty()
            || decision
                .issue_ids
                .iter()
                .any(|id| !round_gaps.contains_key(id.as_str()))
        {
            return Err(error(2,"User decisions need unchanged source, original quote/date/version and known gap IDs"));
        }
    }
    let entry = Record {
        schema_version: 1,
        revision,
        cycle_id: cycle.cycle_id.clone(),
        round_id: round.round_id.clone(),
        baseline: round.baseline.clone(),
        source_result: round.finalization.as_ref().unwrap().result.clone(),
        input,
        page_evidence,
    };
    let bytes = encode(&entry)?;
    let file = FileEvidence {
        path: directory.join("record.json").to_string_lossy().into_owned(),
        sha256: hash(&bytes),
    };
    let mut all_records = records.to_vec();
    all_records.push(entry);
    let body = body(&all_records, &cycle, &record_path)?;
    let _record_lock = CycleLock::acquire_project(&root, &record_file)?;
    let current = match fs::read(lock.path(&record_file)?) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error(4, e)),
    };
    let projection = project_block(
        current.as_deref().unwrap_or_default(),
        &block_id(&cycle),
        previous
            .as_ref()
            .map(|ledger| ledger.projection_hash.as_str()),
        &body,
    )?;
    let mut record_refs = previous
        .as_ref()
        .map_or_else(Vec::new, |ledger| ledger.records.clone());
    record_refs.push(file.clone());
    let ledger = Ledger {
        schema_version: 1,
        cycle_id: cycle.cycle_id.clone(),
        revision,
        record_path: record_path.clone(),
        projection_hash: projection.block_hash,
        records: record_refs,
    };
    output.push(ReportFile {
        path: relative(&file.path)?,
        expected_hash: None,
        bytes,
    });
    output.push(ReportFile {
        path: record_file,
        expected_hash: current.as_deref().map(hash),
        bytes: projection.bytes,
    });
    if cycle.record_path.is_none() {
        cycle.record_path = Some(record_path.clone());
        cycle.revision = cycle
            .revision
            .checked_add(1)
            .ok_or_else(|| error(2, "Cycle revision limit reached"))?;
        output.push(ReportFile {
            path: index,
            expected_hash: Some(index_before.hash),
            bytes: encode(&cycle)?,
        });
    } else {
        sources.push((index, index_before.hash));
    }
    sources.push((path.to_path_buf(), round_before.hash.clone()));
    lock.prepare_draft(
        &applied,
        before.as_ref(),
        &encode(&ledger)?,
        &output,
        &sources,
    )?;
    lock.recover_publication()?;
    Ok(Receipt {
        round_id: round.round_id,
        revision,
        record: file,
        record_path,
    })
}

fn block_id(cycle: &Cycle) -> String {
    format!("apply-{}", &cycle.cycle_id[..16])
}

pub(super) fn load(
    lock: &CycleLock,
    cycle: &Cycle,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<Loaded, StoreError> {
    let path = relative(&format!("{}.ui-review.applied.json", cycle.prefix))?;
    let Some(snapshot) = lock.read(&path)? else {
        return Ok(Loaded {
            before: None,
            ledger: None,
            records: Vec::new(),
        });
    };
    let ledger: Ledger = serde_json::from_slice(&snapshot.bytes).map_err(|e| error(4, e))?;
    if ledger.schema_version != 1
        || ledger.cycle_id != cycle.cycle_id
        || ledger.revision != snapshot.revision
        || ledger.revision == 0
        || ledger.records.len() as u64 != ledger.revision
        || cycle.record_path.as_deref() != Some(ledger.record_path.as_str())
    {
        return Err(error(
            4,
            "Applied schema, revision or cycle record location changed",
        ));
    }
    let mut records = Vec::new();
    let mut previous_round = 0;
    for (n, file) in ledger.records.iter().enumerate() {
        let bytes = require_file(lock, file, sources)?;
        let record: Record = serde_json::from_slice(&bytes).map_err(|e| error(4, e))?;
        let expected = format!(
            "{}.ui-review.applied/{}/rev{}/record.json",
            cycle.prefix,
            record.round_id,
            n + 1
        );
        let round_number = ordinal(&record.round_id)?;
        if record.schema_version != 1
            || record.revision != (n + 1) as u64
            || record.cycle_id != cycle.cycle_id
            || record.input.schema_version != 1
            || record.input.cycle_id != record.cycle_id
            || record.input.round_id != record.round_id
            || file.path != expected
            || round_number < previous_round
            || !record.input.tested_version.known()
            || !capture_evidence::timestamp(&record.input.at)
        {
            return Err(error(
                4,
                "Applied record identity, version or ordering changed",
            ));
        }
        previous_round = round_number;
        let link = cycle
            .rounds
            .iter()
            .find(|link| link.round_id == record.round_id)
            .ok_or_else(|| error(4, "Applied round is unregistered"))?;
        let (_, round) = load_round(lock, cycle, Path::new(&link.path))?;
        if round
            .finalization
            .as_ref()
            .is_none_or(|f| f.result != record.source_result)
            || round.baseline != record.baseline
        {
            return Err(error(4, "Applied record is not bound to the sealed result"));
        }
        let result = finalize::load_result(lock, cycle, &round, sources)?;
        let ids: BTreeSet<_> = result
            .units
            .iter()
            .flat_map(|u| &u.gaps)
            .chain(&result.carried)
            .map(|g| g.id.as_str())
            .collect();
        let mut seen = BTreeSet::new();
        for item in &record.input.items {
            if !ids.contains(item.issue_id.as_str()) || !seen.insert(&item.issue_id) {
                return Err(error(4, "Applied gap references changed"));
            }
            let page = record
                .page_evidence
                .iter()
                .find(|p| p.issue_id == item.issue_id);
            if matches!(&item.self_check,SelfCheck::Page{capture,..} if page.is_none_or(|p|p.input!=**capture))
                || matches!(&item.self_check,SelfCheck::NotRun{..} if page.is_some())
            {
                return Err(error(4, "Applied page evidence binding changed"));
            }
        }
        for page in &record.page_evidence {
            let expected = format!(
                "{}.ui-review.applied/{}/rev{}/evidence/{}.png",
                cycle.prefix,
                record.round_id,
                n + 1,
                page.issue_id
            );
            if page.frozen_image.path != expected || !seen.contains(&page.issue_id) {
                return Err(error(4, "Applied frozen image path changed"));
            }
            require_file(lock, &page.frozen_image, sources)?;
        }
        records.push(record);
    }
    let projected = project_block(
        &[],
        &block_id(cycle),
        None,
        &body(&records, cycle, &ledger.record_path)?,
    )?;
    if projected.block_hash != ledger.projection_hash {
        return Err(error(
            4,
            "Applied projection hash does not match its records",
        ));
    }
    Ok(Loaded {
        before: Some(snapshot),
        ledger: Some(ledger),
        records,
    })
}

fn escaped(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn link(label: &str, target: &str, record_path: &str) -> Result<String, StoreError> {
    let file = relative(target)?;
    let record = relative(record_path)?;
    let parent: Vec<_> = record
        .parent()
        .unwrap_or(Path::new(""))
        .components()
        .collect();
    let parts: Vec<_> = file.components().collect();
    let common = parent
        .iter()
        .zip(&parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut path = PathBuf::new();
    for _ in common..parent.len() {
        path.push("..");
    }
    for component in &parts[common..] {
        path.push(component.as_os_str());
    }
    let target = path.to_string_lossy().replace('\\', "/");
    let mut encoded = String::new();
    for byte in target.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(format!("[{}]({encoded})", escaped(label)))
}

pub(super) fn body(
    records: &[Record],
    cycle: &Cycle,
    record_path: &str,
) -> Result<Vec<u8>, StoreError> {
    let mut groups = BTreeMap::<u32, Vec<&Record>>::new();
    for record in records {
        groups
            .entry(ordinal(&record.round_id)?)
            .or_default()
            .push(record);
    }
    let mut text = "## UI 验收修复\n\n".to_owned();
    for entries in groups.values() {
        let latest = entries.last().unwrap();
        text.push_str(&format!(
            "### {} → 修复 · {}\n\n- 来源：{}；目标包 {}（目标基线 {}）\n- 修复版本：{}\n",
            latest.round_id,
            escaped(&latest.input.at),
            link(
                &format!("{} 报告", latest.round_id),
                &report_path(&cycle.prefix, &latest.round_id),
                record_path,
            )?,
            link("目标包", &cycle.target, record_path)?,
            latest.baseline,
            escaped(&String::from_utf8(encode(&latest.input.tested_version)?).unwrap())
        ));
        let refactors: Vec<_> = entries
            .iter()
            .flat_map(|r| &r.input.restructure_scopes)
            .collect();
        if refactors.is_empty() {
            text.push_str("- 重构轮：无\n");
        }
        for refactor in refactors {
            text.push_str(&format!(
                "- 重构轮：{}；假设 {} → {}；预期 {}\n",
                escaped(&String::from_utf8(encode(&refactor.scope)?).unwrap()),
                escaped(&refactor.hypothesis),
                escaped(&refactor.change),
                escaped(&refactor.expected)
            ));
        }
        text.push_str(
            "\n| 差距 | 核实 | 处置 | 改动落点 | 自检 |\n| --- | --- | --- | --- | --- |\n",
        );
        let mut items = BTreeMap::new();
        for record in entries {
            for item in &record.input.items {
                items.insert(&item.issue_id, (item, *record));
            }
        }
        for (id, (item, record)) in &items {
            let status = String::from_utf8(encode(&item.disposition)?).unwrap();
            let check = match &item.self_check {
                SelfCheck::NotRun { reason } => format!("未运行：{}", reason),
                SelfCheck::Page { notes, .. } => format!(
                    "{}；{}",
                    link(
                        &format!("{id} 自检"),
                        &record
                            .page_evidence
                            .iter()
                            .find(|p| p.issue_id == **id)
                            .unwrap()
                            .frozen_image
                            .path,
                        record_path,
                    )?,
                    notes
                ),
            };
            text.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                escaped(id),
                escaped(&item.verification),
                status.trim_matches('"'),
                escaped(
                    &item
                        .changes
                        .iter()
                        .map(|f| f.path.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                escaped(&check)
            ));
        }
        for (id, (item, _)) in &items {
            text.push_str(&format!("\n- {}：{}\n", escaped(id), escaped(&item.notes)));
        }
        if entries
            .iter()
            .all(|record| record.input.decisions.is_empty())
        {
            text.push_str("\n- 用户方向：无\n");
        }
        for record in entries {
            for decision in &record.input.decisions {
                text.push_str(&format!(
                    "\n- 用户方向：{}（{}；{}；{}）\n",
                    escaped(&decision.quote),
                    link("决定来源", &decision.source.path, record_path)?,
                    escaped(&decision.date),
                    escaped(&decision.impact)
                ));
            }
        }
        for check in &latest.input.checks {
            text.push_str(&format!(
                "\n- 检查：{} · {} · {}\n",
                escaped(&check.command),
                match check.status {
                    finalize::CheckStatus::Passed => "通过",
                    finalize::CheckStatus::Failed => "失败",
                    finalize::CheckStatus::Unavailable => "未运行",
                },
                escaped(&check.summary)
            ));
        }
        if latest.input.unfinished.is_empty() {
            text.push_str("\n- 未完项：无\n");
        }
        for unfinished in &latest.input.unfinished {
            text.push_str(&format!("\n- 未完项：{}\n", escaped(unfinished)));
        }
        text.push('\n');
    }
    Ok(text.into_bytes())
}
