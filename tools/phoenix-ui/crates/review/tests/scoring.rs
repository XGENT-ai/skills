use phoenix_ui_review::scoring::{select_review, summarize, RoundScores, UnitReview};
use phoenix_ui_review::Decimal;

fn d(value: &str) -> Decimal {
    value.parse().unwrap()
}
fn review(id: &str, scores: [&str; 4]) -> UnitReview {
    UnitReview {
        reviewer_id: id.into(),
        scores: scores.map(d),
        reasons: std::array::from_fn(|i| format!("{id} dimension {i}")),
    }
}
fn round(scores: &[&str]) -> RoundScores {
    RoundScores {
        full: true,
        evidence_complete: true,
        unit_scores: scores.iter().map(|s| Some(d(s))).collect(),
        threshold: d("8"),
        gating_gaps: false,
        confirmed: false,
        previous_full_scores: vec![],
    }
}

#[test]
fn lowest_unit_is_the_round_score() {
    let result = summarize(&round(&["9", "7"]));
    assert_eq!(result.score, Some(d("7")));
    assert_eq!(result.historical_best, Some(d("7")));
    assert!(!result.confirmation_required);
}

#[test]
fn confirmation_keeps_a_complete_review_and_its_reasons() {
    let reviews = [
        review("first", ["2.4", "2.3", "2.6", "0.7"]),
        review("confirmation", ["2.6", "2.1", "2.4", "0.7"]),
    ];
    let selected = select_review(&reviews).unwrap().unwrap();
    assert_eq!(selected.score, d("7.8"));
    assert_eq!(selected.review.reviewer_id, "confirmation");
    assert_eq!(selected.review.scores, reviews[1].scores);
    assert_eq!(selected.review.reasons, reviews[1].reasons);
    let tied = [
        review("a", ["2", "3", "2", "0.5"]),
        review("b", ["3", "2", "2", "0.5"]),
    ];
    assert_eq!(
        select_review(&tied).unwrap().unwrap().review.reviewer_id,
        "a"
    );
}

#[test]
fn incomplete_judges_do_not_become_zero_scores() {
    assert!(select_review(&[]).unwrap().is_none());
    let mut invalid = review("a", ["3", "3", "3", "1"]);
    invalid.reasons[2] = " ".into();
    assert!(select_review(&[invalid]).is_err());
    assert!(select_review(&[review("a", ["3.1", "3", "2", "1"])]).is_err());
    let mut missing = round(&["9"]);
    missing.unit_scores.push(None);
    assert_eq!(summarize(&missing).score, None);
    assert_eq!(summarize(&round(&[])).score, None);
}

#[test]
fn confirmation_uses_the_exact_open_half_point_window() {
    assert!(summarize(&round(&["8.2", "8.3"])).confirmation_required);
    assert!(!summarize(&round(&["8.5"])).confirmation_required);
    assert!(summarize(&round(&["8.499999999999999999999999999999999999"])).confirmation_required);
    let mut max = round(&["10"]);
    max.threshold = d("10");
    assert!(summarize(&max).confirmation_required);
    max.confirmed = true;
    assert!(!summarize(&max).confirmation_required);
}

#[test]
fn a_gating_gap_prevents_a_premature_confirmation() {
    let mut input = round(&["8.2"]);
    input.gating_gaps = true;
    assert!(!summarize(&input).confirmation_required);
}

#[test]
fn scoped_rounds_do_not_update_the_global_score_or_window() {
    let mut input = round(&["9"]);
    input.full = false;
    input.previous_full_scores = vec![d("6"), d("6.5")];
    let result = summarize(&input);
    assert_eq!(result.score, None);
    assert_eq!(result.historical_best, Some(d("6.5")));
    assert!(!result.global_plateau);
    assert!(!result.confirmation_required);
}

#[test]
fn incomplete_capture_does_not_update_the_global_window() {
    let mut input = round(&["6.8"]);
    input.evidence_complete = false;
    input.previous_full_scores = vec![d("6"), d("6.5")];
    let result = summarize(&input);
    assert_eq!(result.score, None);
    assert_eq!(result.historical_best, Some(d("6.5")));
    assert!(!result.global_plateau);
}

#[test]
fn plateau_uses_historical_best_two_valid_rounds_earlier() {
    for (previous, current, expected) in [
        (["6", "6.5"], "6.8", true),
        (["6", "6.5"], "7", false),
        (["3.1", "3.6"], "4.1", false),
        (["8.2", "7.5"], "7.6", true),
        (["6", "6.5"], "8.2", false),
    ] {
        let mut input = round(&[current]);
        input.previous_full_scores = previous.map(d).to_vec();
        assert_eq!(
            summarize(&input).global_plateau,
            expected,
            "{previous:?} -> {current}"
        );
    }
    let mut input = round(&["6.8"]);
    input.previous_full_scores = vec![d("6.5")];
    assert!(!summarize(&input).global_plateau);
}

#[test]
fn shared_rule_table_scoring_counterexamples() {
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        input: RoundScores,
        expected: phoenix_ui_review::scoring::Summary,
    }
    let rules: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../skills/review-ui/references/rules.v1.json"
    ))
    .unwrap();
    let cases: Vec<Case> = serde_json::from_value(rules["scoringCases"].clone()).unwrap();
    assert_eq!(cases.len(), 15);
    for case in cases {
        assert_eq!(summarize(&case.input), case.expected, "{}", case.name);
    }
}
