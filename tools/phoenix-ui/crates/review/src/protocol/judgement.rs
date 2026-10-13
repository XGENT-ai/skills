use super::*;
use crate::{
    capture::CellKey,
    policy::{GapStatus, Severity},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub fresh_context: bool,
    pub read_only: bool,
    pub inherited_model: bool,
    pub opened_images: Vec<FileEvidence>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Conversion {
    ReviewerStructured,
    HostVerified {
        #[serde(rename = "rawSha256")]
        raw_sha256: String,
        #[serde(rename = "checkedUnits")]
        checked_units: Vec<String>,
        #[serde(rename = "checkedGaps")]
        checked_gaps: Vec<String>,
        notes: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnseenCell {
    pub cell: CellKey,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GapInput {
    pub key: String,
    pub cell: CellKey,
    pub image: FileEvidence,
    pub dimension: String,
    pub location: String,
    pub current: String,
    pub expected: String,
    pub basis: FileEvidence,
    pub basis_location: String,
    pub fix: String,
    pub severity: Severity,
    pub recoverable: Decimal,
    /// The reviewer's own short title for the gap, copied from the raw text when it
    /// has one; reports otherwise derive the title from `current`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnitInput {
    pub unit_id: String,
    pub scores: [Decimal; 4],
    pub reasons: [String; 4],
    pub gaps: Vec<GapInput>,
    pub suggestions: Vec<String>,
    pub unseen_cells: Vec<UnseenCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OldAssessment {
    pub id: String,
    pub status: GapStatus,
    pub basis: AssessmentBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurrentEvidence {
    pub cell: CellKey,
    pub image: FileEvidence,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssessmentBasis {
    Current(CurrentEvidence),
    LegacyText(String),
}

impl From<&str> for AssessmentBasis {
    fn from(text: &str) -> Self {
        Self::LegacyText(text.into())
    }
}

impl From<String> for AssessmentBasis {
    fn from(text: String) -> Self {
        Self::LegacyText(text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JudgeInput {
    pub schema_version: u32,
    pub cycle_id: String,
    pub round_id: String,
    pub packet_hash: String,
    pub reviewer_id: String,
    pub raw_sha256: String,
    pub provenance: Provenance,
    pub conversion: Conversion,
    pub units: Vec<UnitInput>,
    pub unreviewed_units: Vec<packet::UnreviewedUnit>,
    pub previous: Vec<OldAssessment>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GapMapping {
    pub key: String,
    pub id: String,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assignment {
    pub key: String,
    pub id: String,
    pub unit_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Issue {
    pub id: String,
    pub unit_id: String,
    pub first_round: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JudgeRecord {
    pub schema_version: u32,
    pub input: JudgeInput,
    pub raw: FileEvidence,
    pub assignments: Vec<Assignment>,
    pub mappings: Vec<GapMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JudgeReceipt {
    pub file: FileEvidence,
    pub assignments: Vec<Assignment>,
}

pub(super) fn issue_number(id: &str) -> Option<u64> {
    id.strip_prefix("UI-")
        .and_then(|n| n.parse::<u64>().ok())
        .filter(|n| *n > 0 && format!("UI-{n:02}") == id)
}

fn portable(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}

fn image_file(path: &str) -> bool {
    matches!(
        Path::new(path).extension().and_then(|ext| ext.to_str()),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp")
    )
}

fn strings_match(actual: &[String], expected: &BTreeSet<String>) -> bool {
    actual.len() == expected.len() && actual.iter().cloned().collect::<BTreeSet<_>>() == *expected
}

fn temporary(key: &str) -> bool {
    key.strip_prefix("新-")
        .and_then(|n| n.parse::<u64>().ok())
        .is_some_and(|n| n > 0 && key == format!("新-{n}"))
}

fn validate_mappings(
    input: &JudgeInput,
    mappings: &[GapMapping],
    manifest: &packet::Manifest,
    cycle: &Cycle,
    round: &Round,
) -> Result<BTreeMap<String, String>, StoreError> {
    let gaps: BTreeMap<_, _> = input
        .units
        .iter()
        .flat_map(|unit| {
            unit.gaps
                .iter()
                .map(move |gap| (gap.key.as_str(), unit.unit_id.as_str()))
        })
        .collect();
    let mut mapped = BTreeMap::new();
    for mapping in mappings {
        if !temporary(&mapping.key)
            || mapping.basis.trim().is_empty()
            || mapped
                .insert(mapping.key.clone(), mapping.id.clone())
                .is_some()
            || !gaps.get(mapping.key.as_str()).is_some_and(|unit| {
                cycle.issues.iter().any(|issue| {
                    issue.id == mapping.id
                        && issue.unit_id == *unit
                        && (issue.first_round == round.round_id
                            || manifest.previous_units.iter().any(|old| {
                                old.unit_id == *unit
                                    && old.gaps.iter().any(|gap| gap.id == mapping.id)
                            }))
                })
            })
        {
            return Err(error(2, "Gap mappings must explicitly link unique new keys to the same unit's assigned issue"));
        }
    }
    Ok(mapped)
}

fn validate_input(
    input: &JudgeInput,
    manifest: &packet::Manifest,
    round: &Round,
) -> Result<(), StoreError> {
    if input.schema_version != 1
        || input.cycle_id != round.cycle_id
        || input.round_id != round.round_id
        || !portable(&input.reviewer_id)
        || input.summary.trim().is_empty()
        || !input.provenance.fresh_context
        || !input.provenance.read_only
        || !input.provenance.inherited_model
        || input.provenance.notes.trim().is_empty()
    {
        return Err(error(
            2,
            "Judge identity, summary and declared independent provenance are required",
        ));
    }
    let images: Vec<_> = manifest
        .files
        .iter()
        .filter(|file| image_file(&file.path))
        .collect();
    if input.provenance.opened_images.len() != images.len()
        || input
            .provenance
            .opened_images
            .iter()
            .any(|file| !images.contains(&file))
        || input
            .provenance
            .opened_images
            .iter()
            .map(|file| &file.path)
            .collect::<BTreeSet<_>>()
            .len()
            != images.len()
    {
        return Err(error(
            2,
            "Every image in the assigned packet must be declared opened exactly once",
        ));
    }
    let mut units = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for unit in &input.units {
        if !manifest.units.contains(&unit.unit_id)
            || !units.insert(unit.unit_id.clone())
            || unit.reasons.iter().any(|reason| reason.trim().is_empty())
            || unit
                .suggestions
                .iter()
                .any(|reason| reason.trim().is_empty())
        {
            return Err(error(
                2,
                "Scored units must be unique, assigned and have all dimension reasons",
            ));
        }
        let score = crate::sum_dimensions(unit.scores).map_err(|e| error(2, e))?;
        let mut recovery = Decimal::zero();
        for gap in &unit.gaps {
            if !keys.insert(gap.key.clone())
                || (!temporary(&gap.key)
                    && !manifest.previous_units.iter().any(|old| {
                        old.unit_id == unit.unit_id
                            && old.gaps.iter().any(|old_gap| old_gap.id == gap.key)
                    }))
                || gap.cell.unit_id != unit.unit_id
                || !["布局", "视觉", "交互与状态", "细节"].contains(&gap.dimension.as_str())
                || [
                    &gap.location,
                    &gap.current,
                    &gap.expected,
                    &gap.basis_location,
                    &gap.fix,
                ]
                .iter()
                .any(|value| value.trim().is_empty())
                || gap
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.trim().is_empty())
            {
                return Err(error(
                    2,
                    "Each gap needs a unique key, dimension, location, basis and executable fix",
                ));
            }
            let capture = manifest
                .captures
                .iter()
                .find(|capture| capture.current && capture.input.cell == gap.cell);
            if !capture.is_some_and(|capture| matches!(&capture.input.observation,Observation::Captured{image,..} if image==&gap.image)) {
                return Err(error(2,"Gap image and cell must reference the packet's current capture"));
            }
            if !(gap.basis.path == "expectations.txt" || gap.basis.path.starts_with("target/"))
                || !manifest.files.contains(&gap.basis)
            {
                return Err(error(
                    2,
                    "Deductions require a frozen target image or expectation basis",
                ));
            }
            recovery = recovery.add(gap.recoverable).map_err(|e| error(2, e))?;
        }
        let half: Decimal = "0.5".parse().unwrap();
        let scale = score.scale.max(recovery.scale).max(half.scale);
        let loss = 10 * 10u128.pow(scale) - score.scaled(scale);
        if loss.abs_diff(recovery.scaled(scale)) > half.scaled(scale) {
            return Err(error(
                2,
                "Gap recovery must explain the unit's loss within 0.5",
            ));
        }
        let mut unseen = BTreeSet::new();
        for cell in &unit.unseen_cells {
            if cell.cell.unit_id != unit.unit_id
                || cell.reason.trim().is_empty()
                || !round
                    .matrix
                    .cells
                    .iter()
                    .any(|frozen| frozen.key == cell.cell)
                || !unseen.insert(serde_json::to_string(&cell.cell).map_err(|e| error(2, e))?)
            {
                return Err(error(
                    2,
                    "Unseen cells must name unique assigned cells and reasons",
                ));
            }
        }
    }
    let scored = units.clone();
    if !input.unreviewed_units.is_empty() || units != manifest.units.iter().cloned().collect() {
        return Err(error(
            2,
            "Every assigned unit needs a complete score before a judge can be registered",
        ));
    }
    let assigned: BTreeMap<_, _> = manifest
        .previous_units
        .iter()
        .flat_map(|unit| {
            unit.gaps
                .iter()
                .map(|gap| (gap.id.as_str(), unit.unit_id.as_str()))
        })
        .collect();
    let mut assessed = BTreeSet::new();
    for assessment in &input.previous {
        let AssessmentBasis::Current(basis) = &assessment.basis else {
            return Err(error(
                2,
                "Previous gap conclusions need typed current image evidence",
            ));
        };
        if assigned.get(assessment.id.as_str()) != Some(&basis.cell.unit_id.as_str())
            || !assessed.insert(assessment.id.as_str())
            || assessment.status == GapStatus::NotReviewed
            || basis.notes.trim().is_empty()
            || !manifest.captures.iter().any(|capture| {
                capture.current && capture.input.cell == basis.cell
                    && matches!(&capture.input.observation, Observation::Captured { image, .. } if image == &basis.image)
            })
        {
            return Err(error(2, "Previous conclusions must uniquely assess assigned IDs with the same unit's current image and basis"));
        }
        keys.insert(assessment.id.clone());
    }
    if assessed != assigned.keys().copied().collect() {
        return Err(error(
            2,
            "Every assigned previous gap needs a complete reassessment",
        ));
    }
    if let Conversion::HostVerified {
        raw_sha256,
        checked_units,
        checked_gaps,
        notes,
    } = &input.conversion
    {
        if raw_sha256 != &input.raw_sha256
            || notes.trim().is_empty()
            || !strings_match(checked_units, &scored)
            || !strings_match(checked_gaps, &keys)
        {
            return Err(error(2,"Host conversion must attest every unit and gap against the unchanged raw conclusion"));
        }
    }
    Ok(())
}

pub(super) fn load(
    lock: &CycleLock,
    round: &Round,
    cycle: &Cycle,
    file: &FileEvidence,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<JudgeRecord, StoreError> {
    let record: JudgeRecord =
        serde_json::from_slice(&require_file(lock, file, sources)?).map_err(|e| error(4, e))?;
    let digest = hash(&encode(&(&record.input, &record.mappings))?);
    let directory = round_path(&round.prefix, ordinal(&round.round_id)?)
        .parent()
        .unwrap()
        .join("judges")
        .join(&record.input.reviewer_id)
        .join(digest);
    if record.schema_version != 1
        || file.path != directory.join("record.json").to_string_lossy()
        || record.raw.path != directory.join("raw.md").to_string_lossy()
        || record.raw.sha256 != record.input.raw_sha256
    {
        return Err(error(
            4,
            "Judge registration, reviewer or raw binding changed",
        ));
    }
    let raw = require_file(lock, &record.raw, sources)?;
    if std::str::from_utf8(&raw)
        .map_err(|e| error(4, e))?
        .trim()
        .is_empty()
    {
        return Err(error(4, "Raw judge conclusion is missing"));
    }
    let reference = round
        .packets
        .iter()
        .find(|packet| packet.packet_hash == record.input.packet_hash)
        .ok_or_else(|| error(4, "Judge packet is not registered"))?;
    let manifest = packet::load(lock, round, reference, sources)?;
    validate_input(&record.input, &manifest, round).map_err(|e| error(4, e.message))?;
    let mapped = validate_mappings(&record.input, &record.mappings, &manifest, cycle, round)
        .map_err(|e| error(4, e.message))?;
    let expected: Vec<_> = record
        .input
        .units
        .iter()
        .flat_map(|unit| unit.gaps.iter().map(move |gap| (&gap.key, &unit.unit_id)))
        .collect();
    if record.assignments.len() != expected.len()
        || record
            .assignments
            .iter()
            .zip(expected)
            .any(|(assignment, (key, unit))| {
                &assignment.key != key
                    || &assignment.unit_id != unit
                    || !cycle.issues.iter().any(|issue| {
                        issue.id == assignment.id && issue.unit_id == assignment.unit_id
                    })
                    || if !temporary(key) {
                        &assignment.id != key
                    } else if let Some(id) = mapped.get(key.as_str()) {
                        &assignment.id != id
                    } else {
                        !cycle.issues.iter().any(|issue| {
                            issue.id == assignment.id && issue.first_round == round.round_id
                        })
                    }
            })
    {
        return Err(error(4, "Judge issue assignment changed"));
    }
    Ok(record)
}

pub fn add(
    project: &Path,
    path: &Path,
    expected_revision: u64,
    input: JudgeInput,
    raw: &[u8],
    mappings: Vec<GapMapping>,
) -> Result<JudgeReceipt, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let prefix = prefix_from_round(path)?;
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    let lock = CycleLock::acquire_project(&root, &index)?;
    lock.recover_publication()?;
    let index_before = lock
        .read(&index)?
        .ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let mut cycle =
        load_cycle(&lock, &prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let (before, mut round) = load_round(&lock, &cycle, path)?;
    if round.revision != expected_revision {
        return Err(error(
            3,
            "Round revision changed; reload before registering a judge",
        ));
    }
    if round.finalization.is_some() {
        return Err(error(3, "Finalized judge inputs are immutable"));
    }
    let mut sources = verified_sources(&root, &lock, &cycle, &round, path)?;
    let reference = round
        .packets
        .iter()
        .find(|packet| packet.packet_hash == input.packet_hash)
        .ok_or_else(|| error(2, "Judge packet is not registered"))?;
    let manifest = packet::load(&lock, &round, reference, &mut sources)?;
    packet::current(&round, &manifest)?;
    validate_input(&input, &manifest, &round)?;
    if hash(raw) != input.raw_sha256
        || std::str::from_utf8(raw)
            .map_err(|e| error(2, e))?
            .trim()
            .is_empty()
    {
        return Err(error(
            2,
            "The complete raw conclusion must match its registered SHA-256",
        ));
    }
    let digest = hash(&encode(&(&input, &mappings))?);
    let directory = path
        .parent()
        .unwrap()
        .join("judges")
        .join(&input.reviewer_id)
        .join(digest);
    let record_path = directory.join("record.json");
    for file in &round.judges {
        let old = load(&lock, &round, &cycle, file, &mut sources)?;
        if Path::new(&file.path) == record_path {
            return Ok(JudgeReceipt {
                file: file.clone(),
                assignments: old.assignments,
            });
        }
        if old.input.reviewer_id == input.reviewer_id
            && input.units.iter().any(|unit| {
                old.input
                    .units
                    .iter()
                    .any(|scored| scored.unit_id == unit.unit_id)
            })
        {
            return Err(error(
                3,
                "The same reviewer may only continue previously missing units",
            ));
        }
    }
    let mapped = validate_mappings(&input, &mappings, &manifest, &cycle, &round)?;
    let mut assignments = Vec::new();
    let previous: BTreeMap<_, _> = manifest
        .previous_units
        .iter()
        .flat_map(|unit| {
            unit.gaps
                .iter()
                .map(|gap| (gap.id.as_str(), unit.unit_id.as_str()))
        })
        .collect();
    for unit in &input.units {
        for gap in &unit.gaps {
            let temporary = temporary(&gap.key);
            let existing = previous.get(gap.key.as_str()) == Some(&unit.unit_id.as_str());
            if !temporary && !existing {
                return Err(error(
                    2,
                    "Judge gaps must use temporary 新-N or their assigned previous ID",
                ));
            }
            if existing && mapped.contains_key(gap.key.as_str()) {
                return Err(error(2, "A retained previous ID cannot be remapped"));
            }
            let id = if existing {
                gap.key.clone()
            } else if let Some(id) = mapped.get(gap.key.as_str()) {
                id.clone()
            } else {
                let id = format!("UI-{:02}", cycle.next_issue_id);
                cycle.next_issue_id = cycle
                    .next_issue_id
                    .checked_add(1)
                    .ok_or_else(|| error(2, "Issue counter limit reached"))?;
                cycle.issues.push(Issue {
                    id: id.clone(),
                    unit_id: unit.unit_id.clone(),
                    first_round: round.round_id.clone(),
                });
                id
            };
            assignments.push(Assignment {
                key: gap.key.clone(),
                id,
                unit_id: unit.unit_id.clone(),
            });
        }
    }
    let raw_file = FileEvidence {
        path: directory.join("raw.md").to_string_lossy().into_owned(),
        sha256: input.raw_sha256.clone(),
    };
    let record = JudgeRecord {
        schema_version: 1,
        input,
        raw: raw_file.clone(),
        assignments: assignments.clone(),
        mappings,
    };
    let bytes = encode(&record)?;
    let file = FileEvidence {
        path: record_path.to_string_lossy().into_owned(),
        sha256: hash(&bytes),
    };
    round.judges.push(file.clone());
    round.revision = round
        .revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Round revision limit reached"))?;
    cycle.revision = cycle
        .revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Cycle revision limit reached"))?;
    let output = [
        ReportFile {
            path: relative(&raw_file.path)?,
            expected_hash: None,
            bytes: raw.to_vec(),
        },
        ReportFile {
            path: record_path,
            expected_hash: None,
            bytes,
        },
        ReportFile {
            path: index,
            expected_hash: Some(index_before.hash),
            bytes: encode(&cycle)?,
        },
    ];
    lock.prepare_draft(path, Some(&before), &encode(&round)?, &output, &sources)?;
    lock.recover_publication()?;
    Ok(JudgeReceipt { file, assignments })
}
