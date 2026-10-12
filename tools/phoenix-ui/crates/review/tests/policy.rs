use phoenix_ui_review::policy::{finalize, Outcome, PolicyInput};

#[test]
fn shared_rule_table_adjudication_counterexamples() {
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        input: PolicyInput,
        expected: Option<Outcome>,
        error: Option<String>,
    }
    let rules: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../skills/review-ui/references/rules.v1.json"
    ))
    .unwrap();
    let cases: Vec<Case> = serde_json::from_value(rules["adjudicationCases"].clone()).unwrap();
    assert_eq!(cases.len(), 29);
    for case in cases {
        let actual = finalize(&case.input);
        if let Some(error) = case.error {
            assert_eq!(actual.unwrap_err(), error, "{}", case.name);
        } else {
            assert_eq!(actual.unwrap(), case.expected.unwrap(), "{}", case.name);
        }
    }
}
