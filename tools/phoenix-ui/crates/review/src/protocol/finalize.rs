use super::*;
use crate::{
    capture::CellKey,
    policy::{Disposition, GapStatus, Outcome, PriorDisposition, RefactorHistory, Severity},
};
use crate::{policy, scoring::RoundScores};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Decision {
    pub source: FileEvidence,
    pub quote: String,
    pub date: String,
    pub tested_version: TestedVersion,
    pub issue_ids: Vec<String>,
    pub impact: String,
    #[serde(default)]
    pub kind: DecisionKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecisionKind {
    /// User direction on named gaps; ends their pending-decision state.
    #[default]
    Direction,
    /// The user explicitly accepts the whole current UI (用户豁免通过). It is
    /// appended after the verdict and never resolves gaps or changes scores.
    WholeUiWaiver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Waiver {
    pub round_id: String,
    pub decision: Decision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Page,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finding {
    pub key: String,
    pub existing_issue: Option<String>,
    pub severity: Severity,
    pub description: String,
    pub expected: String,
    pub basis: FileEvidence,
    pub fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeCheck {
    pub schema_version: u32,
    pub kind: CheckKind,
    pub tool: String,
    pub taken_at: String,
    pub tested_version: TestedVersion,
    pub cell: CellKey,
    pub image: FileEvidence,
    pub status: CheckStatus,
    pub notes: String,
    pub findings: Vec<Finding>,
    /// Host recheck of earlier host/machine-measured gaps on this page.
    #[serde(default)]
    pub reassessed: Vec<Reassessment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reassessment {
    pub issue_id: String,
    pub status: GapStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Adjudication {
    pub schema_version: u32,
    pub cycle_id: String,
    pub round_id: String,
    pub machine_checks: Vec<FileEvidence>,
    pub main_checks: Vec<FileEvidence>,
    pub decisions: Vec<Decision>,
    pub applied: Vec<FileEvidence>,
    pub notes: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    ConfirmationRequired {
        outcome: Outcome,
    },
    Finalized {
        round_id: String,
        revision: u64,
        result: FileEvidence,
        report: FileEvidence,
        outcome: Outcome,
    },
    /// An append-only whole-UI waiver on the latest sealed round; `revision` is
    /// the cycle index revision and the sealed outcome is unchanged.
    Waived {
        round_id: String,
        revision: u64,
        outcome: Outcome,
        waivers: Vec<Waiver>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GapDetail {
    pub reviewer_id: String,
    pub judge: FileEvidence,
    pub gap: judgement::GapInput,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapOrigin {
    /// Raised by an independent reviewer; reassessed by later reviewers.
    #[default]
    Reviewer,
    /// Raised by a page measurement; rechecked by the host on the page.
    Runtime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gap {
    pub id: String,
    pub unit_id: String,
    pub severity: Severity,
    pub status: GapStatus,
    pub details: Vec<GapDetail>,
    pub runtime_findings: Vec<(FileEvidence, Finding)>,
    pub history: Vec<crate::policy::Assessment>,
    pub refactor: crate::policy::RefactorHistory,
    pub pending_decision: bool,
    pub recaptured: bool,
    #[serde(default)]
    pub origin: GapOrigin,
    /// Round of the latest real (not 本轮未复核) assessment; attempts count from it.
    #[serde(default)]
    pub assessed_round: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnitConclusion {
    pub unit_id: String,
    pub selected_reviewer: Option<String>,
    pub selected: Option<judgement::UnitInput>,
    pub score: Option<Decimal>,
    pub judge_refs: Vec<FileEvidence>,
    pub gaps: Vec<Gap>,
    pub unreviewed_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResultRecord {
    pub schema_version: u32,
    pub cycle_id: String,
    pub round_id: String,
    pub baseline: String,
    pub target_fingerprint: String,
    pub full: bool,
    pub evidence_complete: bool,
    pub tested_version: TestedVersion,
    pub units: Vec<UnitConclusion>,
    /// Open gaps of units outside this round's scope, carried as 本轮未复核.
    #[serde(default)]
    pub carried: Vec<Gap>,
    pub adjudication: Adjudication,
    pub outcome: Outcome,
    /// The sealed report bytes; a later whole-UI waiver adds only its generated block.
    pub report: FileEvidence,
    pub judge_projection: FileEvidence,
}

fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::P0 => 0,
        Severity::P1 => 1,
        Severity::P2 => 2,
        Severity::P3 => 3,
    }
}

pub(super) fn current_capture<'a>(
    round: &'a Round,
    cell: &CellKey,
) -> Option<&'a RegisteredCapture> {
    round
        .captures
        .iter()
        .filter(|capture| capture.input.cell == *cell)
        .max_by_key(|capture| capture.input.attempt)
}

fn current_judge(
    lock: &CycleLock,
    round: &Round,
    record: &judgement::JudgeRecord,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<bool, StoreError> {
    let reference = round
        .packets
        .iter()
        .find(|packet| packet.packet_hash == record.input.packet_hash)
        .ok_or_else(|| error(4, "Judge packet is missing"))?;
    let manifest = packet::load(lock, round, reference, sources)?;
    Ok(packet::current(round, &manifest).is_ok())
}

pub(super) fn load_result(
    lock: &CycleLock,
    cycle: &Cycle,
    round: &Round,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<ResultRecord, StoreError> {
    // Validating a round recurses into earlier rounds through its judge packets;
    // each exact cycle/round/result state is validated once per held lock.
    let key = format!("result:{}:{}", hash(&encode(cycle)?), hash(&encode(round)?));
    lock.memoized(key, sources, |sources| {
        validate_result(lock, cycle, round, sources)
    })
}

fn validate_result(
    lock: &CycleLock,
    cycle: &Cycle,
    round: &Round,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<ResultRecord, StoreError> {
    let reference = &round
        .finalization
        .as_ref()
        .ok_or_else(|| error(4, "Round is not finalized"))?
        .result;
    let result: ResultRecord = serde_json::from_slice(&require_file(lock, reference, sources)?)
        .map_err(|e| error(4, e))?;
    let units: BTreeSet<_> = result
        .units
        .iter()
        .map(|unit| unit.unit_id.as_str())
        .collect();
    if result.schema_version != 1
        || result.cycle_id != round.cycle_id
        || result.round_id != round.round_id
        || result.baseline != round.baseline
        || result.target_fingerprint != round.target_fingerprint
        || result.full != round.full
        || encode(&result.tested_version)? != encode(&round.tested_version)?
        || units != round.matrix.selected_units().into_iter().collect()
        || units.len() != result.units.len()
        || result.outcome.verdict.is_none()
        || result.outcome.scoring.confirmation_required
        || result.report.path != report_path(&round.prefix, &round.round_id)
        || Path::new(&result.judge_projection.path)
            != round_path(&round.prefix, ordinal(&round.round_id)?)
                .parent()
                .unwrap()
                .join("judge.md")
    {
        return Err(error(
            4,
            "Finalized typed result has invalid round, unit or projection bindings",
        ));
    }
    sealed_report(lock, cycle, round, &result.report, sources)?;
    require_file(lock, &result.judge_projection, sources)?;
    require_file(lock, &round.target_snapshot.expectations, sources)?;
    for attachment in &round.target_snapshot.attachments {
        if let Some(file) = &attachment.frozen {
            require_file(lock, file, sources)?;
        }
    }
    for capture in &round.captures {
        if let Some(file) = &capture.frozen_image {
            require_file(lock, file, sources)?;
        }
    }
    let mut records = Vec::new();
    for file in &round.judges {
        let record = judgement::load(lock, round, cycle, file, sources)?;
        let current = current_judge(lock, round, &record, sources)?;
        records.push((file, record, current));
    }
    for unit in &result.units {
        let registered: Vec<_> = records
            .iter()
            .filter_map(|(file, record, current)| {
                record
                    .input
                    .units
                    .iter()
                    .find(|input| input.unit_id == unit.unit_id)
                    .map(|input| (*file, record.input.reviewer_id.as_str(), input, *current))
            })
            .collect();
        let chosen = registered
            .iter()
            .any(|(_, _, _, current)| *current)
            .then(|| {
                registered
                    .iter()
                    .min_by_key(|(_, _, input, _)| crate::sum_dimensions(input.scores).unwrap())
            })
            .flatten();
        if unit.judge_refs.len() != registered.len()
            || registered
                .iter()
                .any(|(file, _, _, _)| !unit.judge_refs.contains(file))
            || !match (chosen, &unit.selected, &unit.selected_reviewer, unit.score) {
                (Some((_, reviewer, original, _)), Some(selected), Some(actual), Some(score)) => {
                    actual == reviewer
                        && encode(original)? == encode(selected)?
                        && crate::sum_dimensions(selected.scores).map_err(|e| error(4, e))? == score
                }
                (None, None, None, None) => unit
                    .unreviewed_reason
                    .as_ref()
                    .is_some_and(|reason| !reason.trim().is_empty()),
                _ => false,
            }
        {
            return Err(error(
                4,
                "Finalized score or selected whole-review group changed",
            ));
        }
        let mut gaps = BTreeSet::new();
        for gap in &unit.gaps {
            if gap.unit_id != unit.unit_id
                || !gaps.insert(&gap.id)
                || !cycle
                    .issues
                    .iter()
                    .any(|issue| issue.id == gap.id && issue.unit_id == gap.unit_id)
                || gap.history.is_empty()
                || gap.history.last().unwrap().status != gap.status
            {
                return Err(error(
                    4,
                    "Finalized gap identity or assessment history changed",
                ));
            }
        }
    }
    for gap in &result.carried {
        if units.contains(gap.unit_id.as_str())
            || gap.status != GapStatus::NotReviewed
            || gap
                .history
                .last()
                .is_none_or(|last| last.status != gap.status)
            || !cycle
                .issues
                .iter()
                .any(|issue| issue.id == gap.id && issue.unit_id == gap.unit_id)
        {
            return Err(error(4, "Carried gap scope or assessment history changed"));
        }
    }
    let expected_score = (round.full
        && result.evidence_complete
        && result.units.iter().all(|unit| unit.score.is_some()))
    .then(|| {
        result
            .units
            .iter()
            .filter_map(|unit| unit.score)
            .min()
            .unwrap()
    });
    if result.outcome.scoring.score != expected_score {
        return Err(error(
            4,
            "Finalized full score does not match complete minimum unit score",
        ));
    }
    Ok(result)
}

fn new_gap(id: String, unit: String, severity: Severity, origin: GapOrigin, round: &Round) -> Gap {
    Gap {
        id,
        unit_id: unit,
        severity,
        status: GapStatus::Present,
        details: Vec::new(),
        runtime_findings: Vec::new(),
        history: vec![policy::Assessment {
            status: GapStatus::Present,
            prior_disposition: PriorDisposition::default(),
            disposition_unknown: false,
        }],
        refactor: policy::RefactorHistory::NotRecorded,
        pending_decision: false,
        recaptured: false,
        origin,
        assessed_round: Some(round.round_id.clone()),
    }
}

pub(super) fn closed(status: GapStatus) -> bool {
    matches!(status, GapStatus::Resolved | GapStatus::NotApplicable)
}

/// Several reviewers' conclusions are a union: any open conclusion keeps the gap
/// open, and among open ones the more serious state is kept.
fn merged(statuses: &[GapStatus]) -> GapStatus {
    [
        GapStatus::Regressed,
        GapStatus::Present,
        GapStatus::Partial,
        GapStatus::Unverified,
        GapStatus::Resolved,
        GapStatus::NotApplicable,
    ]
    .into_iter()
    .find(|status| statuses.contains(status))
    .unwrap_or(GapStatus::NotReviewed)
}

fn set_status(gap: &mut Gap, status: GapStatus, round: &Round) {
    gap.status = status;
    gap.history.last_mut().unwrap().status = status;
    if status != GapStatus::NotReviewed {
        gap.assessed_round = Some(round.round_id.clone());
    }
}

/// Facts derived from sealed rounds and repair records, never from judge or
/// adjudication claims.
struct History<'a> {
    open: Vec<Gap>,
    records: &'a [applied::Record],
    pending: BTreeSet<String>,
    directions: BTreeSet<String>,
    legacy: Option<&'a legacy::LegacyMap>,
}

/// The dispositions since the gap was last judged, combining imported legacy
/// facts (one legacy disposition precedes every managed round); an unknown legacy
/// count stays unknown unless a real modification is known.
fn prior(history: &History<'_>, old: &Gap) -> Result<(PriorDisposition, bool), StoreError> {
    let managed = disposition(history.records, &old.id, old.assessed_round.as_deref())?;
    let since = history
        .legacy
        .filter(|map| {
            old.assessed_round
                .as_deref()
                .is_none_or(|round| map.is_legacy_round(round))
        })
        .and_then(|map| map.gap(&old.id))
        .map(|gap| &gap.since_last_assessment);
    Ok(match since {
        None => (managed, false),
        Some(since) if since.state == legacy::Knowledge::Unknown && !managed.attempted => {
            (managed, true)
        }
        Some(since) => (
            PriorDisposition {
                attempted: managed.attempted || since.disposition == Some(Disposition::Modified),
                latest: managed.latest.or(since.disposition),
            },
            false,
        ),
    })
}

/// A later record of the same round replaces that round's earlier disposition.
/// Across rounds since the reviewer last judged the gap, a round whose final
/// disposition is 已修改待评审 is an attempt; the most recent round's final
/// disposition is kept separately for pending decisions.
fn disposition(
    records: &[applied::Record],
    id: &str,
    since: Option<&str>,
) -> Result<PriorDisposition, StoreError> {
    let since = since.map(ordinal).transpose()?;
    let mut rounds = BTreeMap::new();
    for record in records {
        let round = ordinal(&record.round_id)?;
        if since.is_some_and(|since| round < since) {
            continue;
        }
        for item in record.input.items.iter().filter(|item| item.issue_id == id) {
            rounds.insert(round, item.disposition);
        }
    }
    Ok(PriorDisposition {
        attempted: rounds.values().any(|d| *d == Disposition::Modified),
        latest: rounds.values().next_back().copied(),
    })
}

fn refactor(
    records: &[applied::Record],
    legacy: Option<&legacy::LegacyMap>,
    baseline: &str,
    scope: &applied::RefactorScope,
) -> RefactorHistory {
    if records
        .iter()
        .filter(|record| record.baseline == baseline)
        .any(|record| {
            record
                .input
                .restructure_scopes
                .iter()
                .any(|r| r.scope == *scope)
        })
    {
        RefactorHistory::Recorded
    } else {
        legacy.map_or(RefactorHistory::NotRecorded, |map| {
            map.refactor(baseline, scope)
        })
    }
}

/// Carries an open gap into this round; reviewers or host checks may then reassess it.
fn carry(old: &Gap, history: &History<'_>) -> Result<Gap, StoreError> {
    let mut gap = old.clone();
    gap.details.clear();
    gap.runtime_findings.clear();
    gap.recaptured = false;
    gap.pending_decision = (old.pending_decision || history.pending.contains(&old.id))
        && !history.directions.contains(&old.id);
    let (prior_disposition, disposition_unknown) = prior(history, old)?;
    gap.history.push(policy::Assessment {
        status: GapStatus::NotReviewed,
        prior_disposition,
        disposition_unknown,
    });
    gap.status = GapStatus::NotReviewed;
    Ok(gap)
}

fn conclusions(
    lock: &CycleLock,
    cycle: &Cycle,
    round: &Round,
    history: &History<'_>,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<(Vec<UnitConclusion>, Vec<Gap>), StoreError> {
    let mut records = Vec::new();
    for file in &round.judges {
        let record = judgement::load(lock, round, cycle, file, sources)?;
        let current = current_judge(lock, round, &record, sources)?;
        records.push((file.clone(), record, current));
    }
    let selected = round.matrix.selected_units();
    let recaptured: BTreeSet<&str> = round
        .captures
        .iter()
        .filter(|capture| capture.input.attempt > 1)
        .map(|capture| capture.input.cell.unit_id.as_str())
        .collect();
    let mut units = Vec::new();
    for id in &selected {
        let id = *id;
        let reviews: Vec<_> = records
            .iter()
            .filter_map(|(file, record, current)| {
                record
                    .input
                    .units
                    .iter()
                    .find(|unit| unit.unit_id == id)
                    .map(|unit| (file, record, unit, *current))
            })
            .collect();
        let selected_review = reviews
            .iter()
            .any(|(_, _, _, current)| *current)
            .then(|| {
                reviews
                    .iter()
                    .min_by_key(|(_, _, unit, _)| crate::sum_dimensions(unit.scores).unwrap())
            })
            .flatten();
        let mut gaps: BTreeMap<String, Gap> = BTreeMap::new();
        for old in history.open.iter().filter(|gap| gap.unit_id == id) {
            gaps.insert(old.id.clone(), carry(old, history)?);
        }
        let mut described = BTreeSet::new();
        for (file, record, unit, _) in &reviews {
            for detail in &unit.gaps {
                let assignment = record
                    .assignments
                    .iter()
                    .find(|assigned| assigned.key == detail.key && assigned.unit_id == id)
                    .ok_or_else(|| error(4, "Registered gap assignment is missing"))?;
                let gap = gaps.entry(assignment.id.clone()).or_insert_with(|| {
                    new_gap(
                        assignment.id.clone(),
                        id.into(),
                        detail.severity,
                        GapOrigin::Reviewer,
                        round,
                    )
                });
                if described.insert(assignment.id.clone())
                    || rank(detail.severity) < rank(gap.severity)
                {
                    gap.severity = detail.severity;
                }
                gap.details.push(GapDetail {
                    reviewer_id: record.input.reviewer_id.clone(),
                    judge: (*file).clone(),
                    gap: detail.clone(),
                });
            }
        }
        for gap in gaps.values_mut() {
            if history.open.iter().any(|old| old.id == gap.id) {
                let statuses = |current_only: bool| -> Vec<GapStatus> {
                    reviews
                        .iter()
                        .filter(|(_, _, _, current)| !current_only || *current)
                        .flat_map(|(_, record, _, _)| {
                            record
                                .input
                                .previous
                                .iter()
                                .filter(|assessment| assessment.id == gap.id)
                                .map(|assessment| assessment.status)
                        })
                        .collect()
                };
                let mut found = statuses(true);
                if found.is_empty() {
                    found = statuses(false);
                }
                set_status(gap, merged(&found), round);
            }
            gap.recaptured = recaptured.contains(id);
        }
        units.push(UnitConclusion {
            unit_id: id.into(),
            selected_reviewer: selected_review
                .map(|(_, record, _, _)| record.input.reviewer_id.clone()),
            selected: selected_review.map(|(_, _, unit, _)| (*unit).clone()),
            score: selected_review
                .map(|(_, _, unit, _)| crate::sum_dimensions(unit.scores).unwrap()),
            judge_refs: reviews
                .iter()
                .map(|(file, _, _, _)| (*file).clone())
                .collect(),
            gaps: gaps.into_values().collect(),
            unreviewed_reason: selected_review.is_none().then(|| {
                "No independent conclusion is registered for the current unit evidence".into()
            }),
        });
    }
    let carried = history
        .open
        .iter()
        .filter(|gap| !selected.contains(&gap.unit_id.as_str()))
        .map(|gap| carry(gap, history))
        .collect::<Result<_, _>>()?;
    Ok((units, carried))
}

fn validate_adjudication(
    lock: &CycleLock,
    cycle: &mut Cycle,
    round: &Round,
    input: &Adjudication,
    units: &mut [UnitConclusion],
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<Vec<(FileEvidence, RuntimeCheck)>, StoreError> {
    if input.schema_version != 1
        || input.cycle_id != round.cycle_id
        || input.round_id != round.round_id
        || input.notes.trim().is_empty()
    {
        return Err(error(
            2,
            "Adjudication identity and actual observation notes are required",
        ));
    }
    if !input.applied.is_empty() {
        return Err(error(
            2,
            "Applied history requires typed registered disposition records",
        ));
    }
    let mut paths = BTreeSet::new();
    let mut checks = Vec::new();
    let mut rechecked = BTreeSet::new();
    for file in input.machine_checks.iter().chain(&input.main_checks) {
        if !paths.insert(&file.path) {
            return Err(error(2, "Adjudication checks must be unique"));
        }
        let check: RuntimeCheck =
            serde_json::from_slice(&require_file(lock, file, sources)?).map_err(|e| error(2, e))?;
        let image =
            current_capture(round, &check.cell).and_then(|capture| capture.frozen_image.as_ref());
        if check.schema_version != 1
            || check.tool.trim().is_empty()
            || check.notes.trim().is_empty()
            || check.taken_at.trim().is_empty()
            || encode(&check.tested_version)? != encode(&round.tested_version)?
            || image != Some(&check.image)
            || (check.status != CheckStatus::Failed && !check.findings.is_empty())
            || (check.status == CheckStatus::Failed && check.findings.is_empty())
        {
            return Err(error(2,"Runtime checks must identify current frozen page evidence, version and complete findings"));
        }
        require_file(lock, &check.image, sources)?;
        let mut keys = BTreeSet::new();
        for finding in &check.findings {
            if finding.key.trim().is_empty()
                || !keys.insert(&finding.key)
                || [&finding.description, &finding.expected, &finding.fix]
                    .iter()
                    .any(|value| value.trim().is_empty())
                || !(finding.basis == round.target_snapshot.expectations
                    || round
                        .target_snapshot
                        .attachments
                        .iter()
                        .any(|attachment| attachment.frozen.as_ref() == Some(&finding.basis)))
            {
                return Err(error(
                    2,
                    "Runtime findings need unique keys and a frozen target basis",
                ));
            }
            require_file(lock, &finding.basis, sources)?;
            if check.kind == CheckKind::Source {
                continue;
            }
            let unit = units
                .iter_mut()
                .find(|unit| unit.unit_id == check.cell.unit_id)
                .ok_or_else(|| error(2, "Runtime finding is outside the frozen scope"))?;
            let id = if let Some(id) = &finding.existing_issue {
                if !unit.gaps.iter().any(|gap| gap.id == *id) {
                    return Err(error(
                        2,
                        "Runtime issue mapping must identify the same unit's open gap",
                    ));
                }
                id.clone()
            } else {
                let id = format!("UI-{:02}", cycle.next_issue_id);
                cycle.next_issue_id = cycle
                    .next_issue_id
                    .checked_add(1)
                    .ok_or_else(|| error(2, "Issue number limit reached"))?;
                cycle.issues.push(judgement::Issue {
                    id: id.clone(),
                    unit_id: unit.unit_id.clone(),
                    first_round: round.round_id.clone(),
                });
                id
            };
            if !unit.gaps.iter().any(|gap| gap.id == id) {
                unit.gaps.push(new_gap(
                    id.clone(),
                    unit.unit_id.clone(),
                    finding.severity,
                    GapOrigin::Runtime,
                    round,
                ));
            }
            let gap = unit.gaps.iter_mut().find(|gap| gap.id == id).unwrap();
            if rank(finding.severity) < rank(gap.severity) {
                gap.severity = finding.severity;
            }
            let observed = [gap.status, GapStatus::Present];
            set_status(gap, merged(&observed), round);
            gap.runtime_findings.push((file.clone(), finding.clone()));
        }
        checks.push((file.clone(), check));
    }
    for (_, check) in &checks {
        for reassessment in &check.reassessed {
            let gap = units
                .iter_mut()
                .filter(|unit| unit.unit_id == check.cell.unit_id)
                .flat_map(|unit| unit.gaps.iter_mut())
                .find(|gap| gap.id == reassessment.issue_id);
            let Some(gap) = gap
                .filter(|gap| gap.origin == GapOrigin::Runtime && gap.runtime_findings.is_empty())
            else {
                return Err(error(
                    2,
                    "Host rechecks may only reassess this unit's earlier measured gaps",
                ));
            };
            if check.kind != CheckKind::Page
                || check.status == CheckStatus::Unavailable
                || reassessment.status == GapStatus::NotReviewed
                || !rechecked.insert(reassessment.issue_id.clone())
            {
                return Err(error(
                    2,
                    "A measured gap needs one actual page recheck with a definite status",
                ));
            }
            set_status(gap, reassessment.status, round);
        }
    }
    for decision in &input.decisions {
        let bytes = require_file(lock, &decision.source, sources)?;
        let text = std::str::from_utf8(&bytes).map_err(|e| error(2, e))?;
        if decision.quote.trim().is_empty()
            || !text.contains(&decision.quote)
            || decision.date.trim().is_empty()
            || decision.impact.trim().is_empty()
            || encode(&decision.tested_version)? != encode(&round.tested_version)?
            || decision
                .issue_ids
                .iter()
                .any(|id| !cycle.issues.iter().any(|issue| issue.id == *id))
        {
            return Err(error(2,"User decisions require an unchanged actual quote, date, version, affected gaps and impact"));
        }
    }
    Ok(checks)
}

pub fn run(
    project: &Path,
    path: &Path,
    expected_revision: u64,
    input: Adjudication,
) -> Result<Response, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let prefix = prefix_from_round(path)?;
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    let lock = CycleLock::acquire_project(&root, &index)?;
    let recovered = lock.recover_publication()?.is_some();
    let index_before = lock
        .read(&index)?
        .ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let mut cycle =
        load_cycle(&lock, &prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let (before, mut round) = load_round(&lock, &cycle, path)?;
    if let (true, Some(finalization)) = (
        recovered && Some(round.revision) == expected_revision.checked_add(1),
        &round.finalization,
    ) {
        // Re-running the exact adjudication whose interrupted publication this
        // call just recovered returns the sealed result; later replays conflict.
        let result = load_result(&lock, &cycle, &round, &mut Vec::new())?;
        if encode(&result.adjudication)? == encode(&input)? {
            return Ok(Response::Finalized {
                round_id: round.round_id.clone(),
                revision: round.revision,
                result: finalization.result.clone(),
                report: result.report,
                outcome: result.outcome,
            });
        }
    }
    if round.finalization.is_some() && waiver_only(&input) {
        return Err(error(
            2,
            "A whole-UI waiver on a sealed round compares the cycle revision; pass --expect-cycle-revision",
        ));
    }
    if round.finalization.is_some() || round.revision != expected_revision {
        return Err(error(3, "Round is finalized or its revision changed"));
    }
    let mut sources = verified_sources(&root, &lock, &cycle, &round, path)?;
    let mut prior = Vec::new();
    for link in cycle
        .rounds
        .iter()
        .filter(|link| link.round_id != round.round_id)
    {
        if ordinal(&link.round_id)? >= ordinal(&round.round_id)? {
            return Err(error(3, "Only the current draft can be finalized"));
        }
        let (_, old) = load_round(&lock, &cycle, Path::new(&link.path))?;
        prior.push(load_result(&lock, &cycle, &old, &mut sources)?);
    }
    let records = applied::load(&lock, &cycle, &mut sources)?.records;
    let legacy = legacy::load(&lock, &cycle, &mut sources)?;
    let last = prior.last();
    let history = History {
        open: match (last, &legacy) {
            (Some(old), _) => old
                .units
                .iter()
                .flat_map(|unit| &unit.gaps)
                .chain(&old.carried)
                .filter(|gap| !closed(gap.status))
                .cloned()
                .collect(),
            (None, Some(map)) => map.open_gaps().map(legacy::LegacyGap::to_gap).collect(),
            (None, None) => Vec::new(),
        },
        records: &records,
        pending: match (last, &legacy) {
            (Some(old), _) => old.outcome.pending_decisions.iter().cloned().collect(),
            (None, Some(map)) => map
                .open_gaps()
                .filter(|gap| gap.pending_decision)
                .map(|gap| gap.id.clone())
                .collect(),
            (None, None) => BTreeSet::new(),
        },
        legacy: legacy.as_ref(),
        directions: input
            .decisions
            .iter()
            .chain(
                records
                    .iter()
                    .filter(|record| last.is_some_and(|old| old.round_id == record.round_id))
                    .flat_map(|record| &record.input.decisions),
            )
            .filter(|decision| decision.kind == DecisionKind::Direction)
            .flat_map(|decision| decision.issue_ids.iter().cloned())
            .collect(),
    };
    let (mut units, mut carried) = conclusions(&lock, &cycle, &round, &history, &mut sources)?;
    let issue_count = cycle.next_issue_id;
    let checks =
        validate_adjudication(&lock, &mut cycle, &round, &input, &mut units, &mut sources)?;
    whole_ui(
        &input.decisions,
        units.iter().flat_map(|unit| &unit.gaps).chain(&carried),
    )?;
    for gap in units
        .iter_mut()
        .flat_map(|unit| &mut unit.gaps)
        .chain(&mut carried)
    {
        gap.refactor = refactor(
            &records,
            legacy.as_ref(),
            &round.baseline,
            &applied::RefactorScope::Unit {
                unit_id: gap.unit_id.clone(),
            },
        );
    }
    let images_complete = round.matrix.selected_cells().all(|cell| {
        current_capture(&round, &cell.key).is_some_and(|capture| {
            matches!(capture.input.observation, Observation::Captured { .. })
        })
    });
    let mut judges = Vec::new();
    for file in &round.judges {
        let record = judgement::load(&lock, &round, &cycle, file, &mut sources)?;
        let current = current_judge(&lock, &round, &record, &mut sources)?;
        judges.push((file.clone(), record, current));
    }
    let mut unseen = Vec::new();
    for (_, record, _) in judges.iter().filter(|(_, _, current)| *current) {
        for cell in record
            .input
            .units
            .iter()
            .flat_map(|unit| &unit.unseen_cells)
            .filter(|unseen| {
                !checks.iter().any(|(_, check)| {
                    check.kind == CheckKind::Page
                        && check.status == CheckStatus::Passed
                        && check.cell == unseen.cell
                })
            })
        {
            if !unseen.contains(&cell.cell) {
                unseen.push(cell.cell.clone());
            }
        }
    }
    let unavailable: Vec<_> = checks
        .iter()
        .filter(|(_, check)| {
            check.kind == CheckKind::Page && check.status == CheckStatus::Unavailable
        })
        .map(|(_, check)| check.cell.clone())
        .collect();
    let evidence_complete = round.target_snapshot.missing.is_empty()
        && round.tested_version.known()
        && images_complete
        && unseen.is_empty()
        && unavailable.is_empty();
    let mut confirmed = true;
    for unit in &units {
        let mut reviewers = BTreeSet::new();
        for file in &unit.judge_refs {
            let record = judgement::load(&lock, &round, &cycle, file, &mut sources)?;
            if current_judge(&lock, &round, &record, &mut sources)? {
                reviewers.insert(record.input.reviewer_id);
            }
        }
        confirmed &= reviewers.len() >= 2;
    }
    let scores = RoundScores {
        full: round.full,
        evidence_complete,
        unit_scores: units.iter().map(|unit| unit.score).collect(),
        threshold: round.threshold,
        gating_gaps: false,
        confirmed,
        previous_full_scores: match &legacy {
            Some(map) => map.scores(&round.baseline)?,
            None => Vec::new(),
        }
        .into_iter()
        .chain(
            prior
                .iter()
                .filter(|old| old.baseline == round.baseline)
                .filter_map(|old| old.outcome.scoring.score),
        )
        .collect(),
    };
    let gaps = units
        .iter()
        .flat_map(|unit| unit.gaps.iter().map(|gap| (gap, true)))
        .chain(carried.iter().map(|gap| (gap, false)))
        .map(|(gap, in_scope)| policy::Gap {
            id: gap.id.clone(),
            unit_id: gap.unit_id.clone(),
            severity: gap.severity,
            assessments: gap.history.clone(),
            in_scope,
            pending_decision: gap.pending_decision,
            refactor: gap.refactor,
            recaptured: gap.recaptured,
        })
        .collect();
    let outcome = policy::finalize(&policy::PolicyInput {
        scores,
        gaps,
        global_refactor: refactor(
            &records,
            legacy.as_ref(),
            &round.baseline,
            &applied::RefactorScope::Global,
        ),
    })
    .map_err(|e| error(3, e))?;
    if outcome.scoring.confirmation_required {
        return Ok(Response::ConfirmationRequired { outcome });
    }
    let cells = |cells: &[CellKey]| {
        cells
            .iter()
            .map(|cell| {
                format!(
                    "{} / {} / {} / {} / {}",
                    cell.unit_id, cell.state_id, cell.viewport_id, cell.theme_id, cell.step_id
                )
            })
            .collect::<Vec<_>>()
            .join("、")
    };
    let mut evidence_gaps = Vec::new();
    if !round.target_snapshot.missing.is_empty() {
        evidence_gaps.push(format!(
            "目标包缺少 {}，补齐期望或附件",
            round.target_snapshot.missing.join("、")
        ));
    }
    if !round.tested_version.known() {
        evidence_gaps.push("被测版本未知，begin 时提供 HEAD 加工作树指纹或可核对的构建 ID".into());
    }
    let missing = missing_cells(&round);
    if !missing.is_empty() {
        evidence_gaps.push(format!(
            "{} 个格子没有采到，按目标包「采集」节补采：{}",
            missing.len(),
            cells(&missing)
        ));
    }
    if !unseen.is_empty() {
        evidence_gaps.push(format!(
            "评审看不出的格子，补采后另派全新评审：{}",
            cells(&unseen)
        ));
    }
    if !unavailable.is_empty() {
        evidence_gaps.push(format!("页面实测不可用：{}", cells(&unavailable)));
    }
    let unscored: Vec<_> = units
        .iter()
        .filter(|unit| unit.score.is_none())
        .map(|unit| unit.unit_id.as_str())
        .collect();
    if !unscored.is_empty() {
        evidence_gaps.push(format!(
            "单元 {} 没有有效的独立评审结论，派发全新评审",
            unscored.join("、")
        ));
    }
    let unverified: Vec<_> = units
        .iter()
        .flat_map(|unit| &unit.gaps)
        .filter(|gap| {
            gap.recaptured
                && gap.status == GapStatus::Unverified
                && matches!(gap.severity, Severity::P0 | Severity::P1)
        })
        .map(|gap| gap.id.as_str())
        .collect();
    if !unverified.is_empty() {
        evidence_gaps.push(format!(
            "P0/P1 差距补采后仍待核实：{}",
            unverified.join("、")
        ));
    }
    let body = report::report(&report::ReportInput {
        cycle: &cycle,
        round: &round,
        units: &units,
        carried: &carried,
        outcome: &outcome,
        adjudication: &input,
        prior: &prior,
        records: &records,
        legacy: legacy.as_ref(),
        judges: &judges,
        checks: &checks,
        previous_open: &history.open,
        evidence_gaps: &evidence_gaps,
    })?;
    let report_path = relative(&report_path(&prefix, &round.round_id))?;
    let report = FileEvidence {
        path: report_path.to_string_lossy().into_owned(),
        sha256: hash(&body),
    };
    let judge_path = path.parent().unwrap().join("judge.md");
    let judge_bytes = report::judge_list(&cycle, &round, &units, &prior, &judges)?;
    let judge_projection = FileEvidence {
        path: judge_path.to_string_lossy().into_owned(),
        sha256: hash(&judge_bytes),
    };
    let result = ResultRecord {
        schema_version: 1,
        cycle_id: round.cycle_id.clone(),
        round_id: round.round_id.clone(),
        baseline: round.baseline.clone(),
        target_fingerprint: round.target_fingerprint.clone(),
        full: round.full,
        evidence_complete,
        tested_version: round.tested_version.clone(),
        units,
        carried,
        adjudication: input,
        outcome,
        report: report.clone(),
        judge_projection: judge_projection.clone(),
    };
    let result_path = path.parent().unwrap().join("result.json");
    let result_bytes = encode(&result)?;
    let result_file = FileEvidence {
        path: result_path.to_string_lossy().into_owned(),
        sha256: hash(&result_bytes),
    };
    let mut reports = vec![
        ReportFile {
            path: report_path,
            expected_hash: None,
            bytes: body,
        },
        ReportFile {
            path: judge_path,
            expected_hash: None,
            bytes: judge_bytes,
        },
        ReportFile {
            path: result_path,
            expected_hash: None,
            bytes: result_bytes,
        },
    ];
    if cycle.next_issue_id != issue_count {
        cycle.revision = cycle
            .revision
            .checked_add(1)
            .ok_or_else(|| error(2, "Cycle revision limit reached"))?;
        reports.push(ReportFile {
            path: index,
            expected_hash: Some(index_before.hash),
            bytes: encode(&cycle)?,
        });
    }
    round.machine_checks = result
        .adjudication
        .machine_checks
        .iter()
        .chain(&result.adjudication.main_checks)
        .cloned()
        .collect();
    round.finalization = Some(Finalization {
        status: "finalized".into(),
        result: result_file.clone(),
    });
    round.revision = round
        .revision
        .checked_add(1)
        .ok_or_else(|| error(2, "Round revision limit reached"))?;
    lock.prepare_publication(path, &before, &encode(&round)?, &reports, &sources)?;
    lock.recover_publication()?;
    Ok(Response::Finalized {
        round_id: round.round_id,
        revision: round.revision,
        result: result_file,
        report,
        outcome: result.outcome,
    })
}

fn waiver_only(input: &Adjudication) -> bool {
    !input.decisions.is_empty()
        && input
            .decisions
            .iter()
            .all(|decision| decision.kind == DecisionKind::WholeUiWaiver)
        && input.machine_checks.is_empty()
        && input.main_checks.is_empty()
        && input.applied.is_empty()
}

/// A whole-UI waiver must name every open gap of the round. Accepting only some
/// gaps changes the comparison basis: that is a confirmed target change and a new
/// baseline, not a waiver.
fn whole_ui<'a>(
    decisions: &[Decision],
    gaps: impl Iterator<Item = &'a Gap>,
) -> Result<(), StoreError> {
    let open: BTreeSet<&str> = gaps
        .filter(|gap| !closed(gap.status))
        .map(|gap| gap.id.as_str())
        .collect();
    for decision in decisions
        .iter()
        .filter(|decision| decision.kind == DecisionKind::WholeUiWaiver)
    {
        let named: BTreeSet<&str> = decision.issue_ids.iter().map(String::as_str).collect();
        if named != open || named.len() != decision.issue_ids.len() {
            return Err(error(
                2,
                "A whole-UI waiver must list every open gap; partial acceptance is a confirmed target change and a new baseline",
            ));
        }
    }
    Ok(())
}

pub(super) fn waiver_body(waivers: &[Waiver]) -> Result<Vec<u8>, StoreError> {
    let mut text = String::from("## 用户豁免通过\n\n");
    for waiver in waivers {
        let decision = &waiver.decision;
        text.push_str(&format!(
            "- {} · {} · 原话：{} · 被测版本：{} · 豁免差距：{} · 剩余影响：{} · 来源：{}；原分数、差距与判定保留，豁免不算已解决。\n",
            report::round_label(&waiver.round_id),
            table_text(&decision.date),
            table_text(&decision.quote),
            report::version_text(&decision.tested_version),
            if decision.issue_ids.is_empty() {
                "无".to_owned()
            } else {
                decision.issue_ids.join("、")
            },
            table_text(&decision.impact),
            decision.source.path
        ));
    }
    Ok(text.into_bytes())
}

/// Appends a whole-UI waiver to the latest sealed round. Its compare-and-swap
/// token is the cycle revision, never the sealed round's revision.
pub fn append_waiver(
    project: &Path,
    path: &Path,
    expected_cycle_revision: u64,
    input: Adjudication,
) -> Result<Response, StoreError> {
    let root = fs::canonicalize(project).map_err(|e| error(4, e))?;
    let prefix = prefix_from_round(path)?;
    let index = relative(&format!("{prefix}.ui-review.json"))?;
    let lock = CycleLock::acquire_project(&root, &index)?;
    lock.recover_publication()?;
    let cycle = load_cycle(&lock, &prefix)?.ok_or_else(|| error(4, "Managed cycle is missing"))?;
    let (_, round) = load_round(&lock, &cycle, path)?;
    if round.finalization.is_none() || !waiver_only(&input) {
        return Err(error(
            2,
            "--expect-cycle-revision only appends a whole-UI waiver to the latest sealed round",
        ));
    }
    waive(
        &lock,
        &index,
        cycle,
        &round,
        path,
        expected_cycle_revision,
        input,
    )
}

fn waiver_block_id(cycle_id: &str, round_id: &str) -> String {
    format!("waiver-{}-{round_id}", &cycle_id[..16])
}

/// The generated block of whole-UI waivers appended to one sealed round, if any.
pub(super) fn waiver_block(cycle: &Cycle, round_id: &str) -> Result<Option<Vec<u8>>, StoreError> {
    let waivers: Vec<Waiver> = cycle
        .waivers
        .iter()
        .filter(|waiver| waiver.round_id == round_id)
        .cloned()
        .collect();
    if waivers.is_empty() {
        return Ok(None);
    }
    let id = waiver_block_id(&cycle.cycle_id, round_id);
    Ok(Some(
        project_block(b"", &id, None, &waiver_body(&waivers)?)?.bytes,
    ))
}

/// Separates a round report into its sealed bytes and its generated waiver block.
pub(super) fn split_report(
    bytes: &[u8],
    cycle_id: &str,
    round_id: &str,
) -> Result<(Vec<u8>, Option<Vec<u8>>), StoreError> {
    let id = waiver_block_id(cycle_id, round_id);
    let begin = format!("<!-- phoenix-ui:{id}:begin -->\n");
    let end = format!("<!-- phoenix-ui:{id}:end -->\n");
    let find = |marker: &str| -> Vec<usize> {
        bytes
            .windows(marker.len())
            .enumerate()
            .filter(|(_, window)| *window == marker.as_bytes())
            .map(|(n, _)| n)
            .collect()
    };
    match (find(&begin).as_slice(), find(&end).as_slice()) {
        ([], []) => Ok((bytes.to_vec(), None)),
        ([start], [stop])
            if start < stop
                && (*start == 0 || bytes[start - 1] == b'\n')
                && bytes.get(stop + end.len()) == Some(&b'\n') =>
        {
            let after = stop + end.len();
            let mut sealed = bytes[..*start].to_vec();
            sealed.extend_from_slice(&bytes[after + 1..]);
            Ok((sealed, Some(bytes[*start..after].to_vec())))
        }
        _ => Err(error(
            3,
            "The report's generated waiver block boundaries were edited",
        )),
    }
}

/// Places a waiver block after the verdict: before the section that follows the
/// verdict and after any waiver recorded when the round was sealed.
pub(super) fn with_waiver_block(sealed: &[u8], block: Option<&[u8]>) -> Vec<u8> {
    let Some(block) = block else {
        return sealed.to_vec();
    };
    let verdict = "\n## 验收判定\n".as_bytes();
    let mut at = sealed.len();
    let mut offset = sealed
        .windows(verdict.len())
        .position(|w| w == verdict)
        .map_or(0, |found| found + verdict.len() - 1);
    while let Some(found) = sealed[offset..].windows(4).position(|w| w == b"\n## ") {
        let start = offset + found + 1;
        if !sealed[start..].starts_with("## 用户豁免通过\n".as_bytes()) {
            at = start;
            break;
        }
        offset = start;
    }
    let mut bytes = sealed[..at].to_vec();
    bytes.extend_from_slice(block);
    bytes.push(b'\n');
    bytes.extend_from_slice(&sealed[at..]);
    bytes
}

/// Whether a round report's generated waiver block matches the cycle's waivers.
pub(super) fn waiver_intact(
    cycle: &Cycle,
    round_id: &str,
    bytes: &[u8],
) -> Result<bool, StoreError> {
    Ok(match split_report(bytes, &cycle.cycle_id, round_id) {
        Ok((_, block)) => block == waiver_block(cycle, round_id)?,
        Err(_) => false,
    })
}

/// A round report minus its generated waiver block must be the sealed report.
fn sealed_report(
    lock: &CycleLock,
    cycle: &Cycle,
    round: &Round,
    report: &FileEvidence,
    sources: &mut Vec<(PathBuf, String)>,
) -> Result<(), StoreError> {
    let path = relative(&report.path)?;
    let bytes = fs::read(lock.path(&path)?).map_err(|e| error(3, e))?;
    let (sealed, _) = split_report(&bytes, &cycle.cycle_id, &round.round_id)?;
    if hash(&sealed) != report.sha256 {
        return Err(error(
            3,
            format!("Registered evidence changed: {}", report.path),
        ));
    }
    sources.push((path, hash(&bytes)));
    Ok(())
}

/// Appends the waiver as a generated block after the verdict inside that round's
/// report; the sealed report bytes, result and round facts stay unchanged.
fn waive(
    lock: &CycleLock,
    index: &Path,
    mut cycle: Cycle,
    round: &Round,
    path: &Path,
    expected_revision: u64,
    input: Adjudication,
) -> Result<Response, StoreError> {
    let index_before = lock
        .read(index)?
        .ok_or_else(|| error(4, "Managed cycle is missing"))?;
    if cycle.revision != expected_revision {
        return Err(error(
            3,
            "Cycle revision changed; reload status before appending a waiver",
        ));
    }
    if cycle
        .rounds
        .last()
        .is_none_or(|link| link.round_id != round.round_id)
    {
        return Err(error(
            3,
            "Only the latest sealed round can receive a waiver",
        ));
    }
    if input.schema_version != 1
        || input.cycle_id != round.cycle_id
        || input.round_id != round.round_id
        || input.notes.trim().is_empty()
    {
        return Err(error(2, "Waiver identity and notes are required"));
    }
    let mut sources = Vec::new();
    let result = load_result(lock, &cycle, round, &mut sources)?;
    let report = relative(&result.report.path)?;
    let current = fs::read(lock.path(&report)?).map_err(|e| error(3, e))?;
    if !waiver_intact(&cycle, &round.round_id, &current)? {
        return Err(error(
            3,
            "The report's generated waiver block was edited; save the edit and re-base the projection first",
        ));
    }
    let gaps = result
        .units
        .iter()
        .flat_map(|unit| &unit.gaps)
        .chain(&result.carried);
    whole_ui(&input.decisions, gaps)?;
    let mut changed = false;
    for decision in input.decisions {
        let bytes = require_file(lock, &decision.source, &mut sources)?;
        let text = std::str::from_utf8(&bytes).map_err(|e| error(2, e))?;
        if decision.quote.trim().is_empty()
            || !text.contains(&decision.quote)
            || decision.date.trim().is_empty()
            || decision.impact.trim().is_empty()
            || decision.tested_version != round.tested_version
        {
            return Err(error(
                2,
                "A waiver needs the user's unchanged actual quote, date, tested version and impact",
            ));
        }
        let waiver = Waiver {
            round_id: round.round_id.clone(),
            decision,
        };
        let bytes = encode(&waiver)?;
        if !cycle
            .waivers
            .iter()
            .any(|old| encode(old).is_ok_and(|old| old == bytes))
        {
            cycle.waivers.push(waiver);
            changed = true;
        }
    }
    let waivers: Vec<_> = cycle
        .waivers
        .iter()
        .filter(|waiver| waiver.round_id == round.round_id)
        .cloned()
        .collect();
    if changed {
        let (sealed, _) = split_report(&current, &cycle.cycle_id, &round.round_id)?;
        let bytes = with_waiver_block(&sealed, waiver_block(&cycle, &round.round_id)?.as_deref());
        cycle.revision = cycle
            .revision
            .checked_add(1)
            .ok_or_else(|| error(2, "Cycle revision limit reached"))?;
        let round_snapshot = lock
            .read(path)?
            .ok_or_else(|| error(4, "Round state is missing"))?;
        sources.push((path.to_path_buf(), round_snapshot.hash));
        sources.retain(|(source, _)| *source != report);
        lock.prepare_draft(
            index,
            Some(&index_before),
            &encode(&cycle)?,
            &[ReportFile {
                path: report,
                expected_hash: Some(hash(&current)),
                bytes,
            }],
            &sources,
        )?;
        lock.recover_publication()?;
    }
    Ok(Response::Waived {
        round_id: round.round_id.clone(),
        revision: cycle.revision,
        outcome: result.outcome,
        waivers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewer_status_precedence_is_the_shared_rule_table_order() {
        let rules: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../skills/review-ui/references/rules.v1.json"
        ))
        .unwrap();
        let order: Vec<GapStatus> =
            serde_json::from_value(rules["reviewerStatusPrecedence"].clone()).unwrap();
        assert_eq!(order.len(), 6);
        for (n, higher) in order.iter().enumerate() {
            for lower in &order[n + 1..] {
                assert_eq!(merged(&[*lower, *higher]), *higher);
                assert_eq!(merged(&[*higher, *lower]), *higher);
            }
        }
    }
}
