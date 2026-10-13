use crate::scoring::{summarize, RoundScores, Summary};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapStatus {
    #[serde(rename = "已解决")]
    Resolved,
    #[serde(rename = "部分解决")]
    Partial,
    #[serde(rename = "仍存在")]
    Present,
    #[serde(rename = "回归")]
    Regressed,
    #[serde(rename = "待核实")]
    Unverified,
    #[serde(rename = "不再适用")]
    NotApplicable,
    #[serde(rename = "本轮未复核")]
    NotReviewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    #[serde(rename = "已修改待评审")]
    Modified,
    #[serde(rename = "不采纳")]
    Declined,
    #[serde(rename = "待决定")]
    AwaitingDecision,
    #[serde(rename = "阻塞")]
    Blocked,
    #[serde(rename = "未处理")]
    Untreated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefactorHistory {
    NotRecorded,
    Recorded,
    Unknown,
}

/// Repair records since the gap's last independent judgement. Within one round a
/// later record replaces that round's earlier disposition; `attempted` holds when
/// any round's final disposition is 已修改待评审, and `latest` is the most recent
/// round's final disposition. Both are evaluated independently.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PriorDisposition {
    pub attempted: bool,
    pub latest: Option<Disposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assessment {
    pub status: GapStatus,
    pub prior_disposition: PriorDisposition,
    /// Imported legacy history could not establish whether a real modification
    /// preceded this assessment; it is never read as "no attempt".
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disposition_unknown: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gap {
    pub id: String,
    pub unit_id: String,
    pub severity: Severity,
    pub assessments: Vec<Assessment>,
    pub in_scope: bool,
    pub pending_decision: bool,
    pub refactor: RefactorHistory,
    pub recaptured: bool,
}

// Scope, dispositions and refactor history must be derived from verified records.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyInput {
    pub scores: RoundScores,
    pub gaps: Vec<Gap>,
    pub global_refactor: RefactorHistory,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    #[serde(rename = "需补证")]
    NeedEvidence,
    #[serde(rename = "通过验收")]
    Passed,
    #[serde(rename = "需用户裁定")]
    NeedDecision,
    #[serde(rename = "需要修改")]
    NeedModification,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Scope {
    Global,
    Gap {
        #[serde(rename = "gapId")]
        gap_id: String,
        #[serde(rename = "unitId")]
        unit_id: String,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningState {
    #[serde(rename = "停滞预警")]
    Warning,
    #[serde(rename = "停滞")]
    Stalled,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Warning {
    pub scope: Scope,
    pub state: WarningState,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Outcome {
    pub verdict: Option<Verdict>,
    pub scoring: Summary,
    pub warnings: Vec<Warning>,
    pub pending_decisions: Vec<String>,
}

impl Gap {
    fn closed(&self) -> bool {
        self.assessments
            .last()
            .is_some_and(|a| matches!(a.status, GapStatus::Resolved | GapStatus::NotApplicable))
    }

    fn gating(&self) -> bool {
        matches!(self.severity, Severity::P0 | Severity::P1)
    }

    fn pending(&self) -> bool {
        let latest = self.assessments.last().unwrap();
        self.pending_decision
            || matches!(
                latest.prior_disposition.latest,
                Some(Disposition::AwaitingDecision | Disposition::Blocked)
            )
            || (latest.prior_disposition.latest == Some(Disposition::Declined)
                && matches!(
                    latest.status,
                    GapStatus::Present | GapStatus::Regressed | GapStatus::Partial
                ))
    }

    fn stubborn(&self) -> Result<bool, &'static str> {
        const UNKNOWN: &str = "gap attempt history is unknown";
        if self.severity == Severity::P3 {
            return Ok(false);
        }
        let latest = self.assessments.last().unwrap();
        let open = matches!(
            latest.status,
            GapStatus::Present | GapStatus::Regressed | GapStatus::Partial
        );
        if latest.disposition_unknown && open {
            return Err(UNKNOWN);
        }
        if !latest.prior_disposition.attempted {
            return Ok(false);
        }
        if matches!(latest.status, GapStatus::Present | GapStatus::Regressed) {
            return Ok(true);
        }
        let prior = self
            .assessments
            .iter()
            .rev()
            .skip(1)
            .find(|prior| prior.status != GapStatus::NotReviewed);
        if latest.status == GapStatus::Partial
            && prior.is_some_and(|prior| {
                prior.status == GapStatus::Partial && prior.disposition_unknown
            })
        {
            return Err(UNKNOWN);
        }
        Ok(latest.status == GapStatus::Partial
            && prior.is_some_and(|prior| {
                prior.status == GapStatus::Partial && prior.prior_disposition.attempted
            }))
    }
}

fn warning_state(
    history: RefactorHistory,
    unknown: &'static str,
) -> Result<WarningState, &'static str> {
    match history {
        RefactorHistory::NotRecorded => Ok(WarningState::Warning),
        RefactorHistory::Recorded => Ok(WarningState::Stalled),
        RefactorHistory::Unknown => Err(unknown),
    }
}

pub fn finalize(input: &PolicyInput) -> Result<Outcome, &'static str> {
    for gap in input.gaps.iter().filter(|g| g.in_scope) {
        if gap.assessments.is_empty() {
            return Err("gap review history is unknown");
        }
    }
    let open: Vec<_> = input
        .gaps
        .iter()
        .filter(|g| g.in_scope && !g.closed())
        .collect();
    let mut scores = input.scores.clone();
    scores.gating_gaps = open.iter().any(|g| g.gating());
    let mut outcome = Outcome {
        verdict: None,
        scoring: summarize(&scores),
        warnings: vec![],
        pending_decisions: vec![],
    };
    let mut pending: BTreeSet<_> = open
        .iter()
        .filter(|g| g.pending())
        .map(|g| g.id.clone())
        .collect();
    let minimum = scores.unit_scores.iter().flatten().copied().min();
    let covered = scores.evidence_complete
        && !scores.unit_scores.is_empty()
        && scores.unit_scores.iter().all(Option::is_some);
    let units_above_threshold = minimum.is_some_and(|score| score >= scores.threshold);
    if !covered
        || open.iter().any(|g| {
            g.gating()
                && g.recaptured
                && g.assessments.last().unwrap().status == GapStatus::Unverified
        })
        || (!scores.full && units_above_threshold && !scores.gating_gaps)
    {
        outcome.verdict = Some(Verdict::NeedEvidence);
        outcome.scoring.confirmation_required = false;
        outcome.pending_decisions = pending.into_iter().collect();
        return Ok(outcome);
    }
    for gap in &open {
        if !gap.stubborn()? {
            continue;
        }
        let state = warning_state(gap.refactor, "gap refactor history is unknown")?;
        if state == WarningState::Stalled {
            pending.insert(gap.id.clone());
        }
        outcome.warnings.push(Warning {
            scope: Scope::Gap {
                gap_id: gap.id.clone(),
                unit_id: gap.unit_id.clone(),
            },
            state,
        });
    }
    let mut global_stalled = false;
    if outcome.scoring.global_plateau {
        let state = warning_state(input.global_refactor, "global refactor history is unknown")?;
        global_stalled = state == WarningState::Stalled;
        outcome.warnings.push(Warning {
            scope: Scope::Global,
            state,
        });
    }
    outcome.verdict = if scores.full && units_above_threshold && !scores.gating_gaps {
        if outcome.scoring.confirmation_required {
            None
        } else {
            Some(Verdict::Passed)
        }
    } else if global_stalled
        || open.iter().all(|g| pending.contains(&g.id))
        || (units_above_threshold
            && scores.gating_gaps
            && open
                .iter()
                .filter(|g| g.gating())
                .all(|g| pending.contains(&g.id)))
    {
        Some(Verdict::NeedDecision)
    } else {
        Some(Verdict::NeedModification)
    };
    outcome.pending_decisions = pending.into_iter().collect();
    Ok(outcome)
}
