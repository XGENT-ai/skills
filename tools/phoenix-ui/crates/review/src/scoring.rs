use crate::{sum_dimensions, Decimal};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnitReview {
    pub reviewer_id: String,
    pub scores: [Decimal; 4],
    pub reasons: [String; 4],
}

pub struct SelectedReview<'a> {
    pub review: &'a UnitReview,
    pub score: Decimal,
}

pub fn select_review(reviews: &[UnitReview]) -> Result<Option<SelectedReview<'_>>, &'static str> {
    let mut selected: Option<SelectedReview<'_>> = None;
    for review in reviews {
        if review.reviewer_id.trim().is_empty()
            || review.reasons.iter().any(|r| r.trim().is_empty())
        {
            return Err("reviewer identity and all dimension reasons are required");
        }
        let score = sum_dimensions(review.scores)?;
        if selected.as_ref().is_none_or(|prior| score < prior.score) {
            selected = Some(SelectedReview { review, score });
        }
    }
    Ok(selected)
}

// These facts come from the frozen round/capture manifest, never judge input.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoundScores {
    pub full: bool,
    pub evidence_complete: bool,
    pub unit_scores: Vec<Option<Decimal>>,
    pub threshold: Decimal,
    pub gating_gaps: bool,
    pub confirmed: bool,
    // Complete full rounds only, from the current target baseline, in round order.
    pub previous_full_scores: Vec<Decimal>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub score: Option<Decimal>,
    pub historical_best: Option<Decimal>,
    pub confirmation_required: bool,
    pub global_plateau: bool,
}

pub fn summarize(round: &RoundScores) -> Summary {
    let complete = round.full
        && round.evidence_complete
        && !round.unit_scores.is_empty()
        && round.unit_scores.iter().all(Option::is_some);
    let score = complete.then(|| round.unit_scores.iter().flatten().copied().min().unwrap());
    let historical_best = round
        .previous_full_scores
        .iter()
        .copied()
        .chain(score)
        .max();
    let confirmation_required = score.is_some_and(|score| {
        score >= round.threshold
            && !round.gating_gaps
            && !round.confirmed
            && score.growth_less_than(round.threshold, "0.5".parse().unwrap())
    });
    let global_plateau = score.is_some_and(|score| {
        if score >= round.threshold || round.previous_full_scores.len() < 2 {
            return false;
        }
        let earlier_best = round.previous_full_scores[..round.previous_full_scores.len() - 1]
            .iter()
            .copied()
            .max()
            .unwrap();
        historical_best
            .unwrap()
            .growth_less_than(earlier_best, "1".parse().unwrap())
    });
    Summary {
        score,
        historical_best,
        confirmation_required,
        global_plateau,
    }
}
