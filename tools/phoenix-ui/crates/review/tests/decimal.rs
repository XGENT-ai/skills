use phoenix_ui_review::{sum_dimensions, Decimal};

fn d(value: &str) -> Decimal {
    value.parse().unwrap()
}

#[test]
fn dimensions_sum_exactly_at_the_threshold() {
    let score = sum_dimensions([d("2.4"), d("2.3"), d("2.6"), d("0.7")]).unwrap();
    assert_eq!(score, d("8"));
    assert_eq!(serde_json::to_string(&score).unwrap(), "\"8\"");
}

#[test]
fn scientific_notation_and_numeric_strings_are_canonical() {
    for input in ["8.000e0", "8e+0", "80e-1", "\"8.000\""] {
        let value: Decimal = serde_json::from_str(input).unwrap();
        assert_eq!(value, d("8"));
        assert_eq!(serde_json::to_string(&value).unwrap(), "\"8\"");
    }
    assert_eq!(d("1e-3").to_string(), "0.001");
}

#[test]
fn raw_json_numbers_keep_precision_beyond_binary_float() {
    let token = "0.123456789012345678901234567890123456";
    let value: Decimal = serde_json::from_str(token).unwrap();
    assert_eq!(value.to_string(), token);
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        format!("\"{token}\"")
    );
    assert!(d("0.02") < d("0.1"));
    assert!(d("8.499999999999999999999999999999999999") < d("8.5"));
}

#[test]
fn confirmation_margin_has_an_exact_open_boundary() {
    assert!(d("8.499999999999999999999999999999999999").growth_less_than(d("8"), d("0.5")));
    assert!(!d("8.5").growth_less_than(d("8"), d("0.5")));
    assert!(!d("10").growth_less_than(d("10"), d("0")));
}

#[test]
fn nested_judge_fields_preserve_the_original_numeric_tokens() {
    #[derive(serde::Deserialize)]
    struct Judge {
        scores: [Decimal; 4],
    }
    let judge: Judge =
        serde_json::from_str(r#"{"scores":[2.4,2.3,2.6,0.700000000000000000000000000000000001]}"#)
            .unwrap();
    assert_eq!(
        sum_dimensions(judge.scores).unwrap().to_string(),
        "8.000000000000000000000000000000000001"
    );
}

#[test]
fn one_point_growth_is_not_a_plateau() {
    for (earlier, current) in [("3.1", "4.1"), ("6", "7"), ("0.3", "1.3")] {
        assert!(!d(current).growth_less_than(d(earlier), d("1")));
    }
    assert!(d("6.8").growth_less_than(d("6.5"), d("1")));
}

#[test]
fn invalid_or_non_finite_scores_are_rejected() {
    for token in [
        "", "NaN", "Infinity", "-0.1", "10.0001", "1e2", "+1", "01", "1.", ".1", "1e", "[]",
        "null", "true",
    ] {
        assert!(token.parse::<Decimal>().is_err(), "{token}");
    }
    for token in ["null", "true", "[]", "{}", "\"NaN\""] {
        assert!(serde_json::from_str::<Decimal>(token).is_err(), "{token}");
    }
}

#[test]
fn dimension_ranges_preserve_the_three_three_three_one_scale() {
    assert_eq!(
        sum_dimensions([d("3"), d("3"), d("3"), d("1")]).unwrap(),
        d("10")
    );
    for scores in [
        [d("3.001"), d("0"), d("0"), d("0")],
        [d("0"), d("0"), d("0"), d("1.001")],
    ] {
        assert!(sum_dimensions(scores).is_err());
    }
}

#[test]
fn resource_limits_return_errors_without_rounding_or_truncation() {
    assert_eq!(d("1e-36").to_string(), format!("0.{}1", "0".repeat(35)));
    assert!("1e-37".parse::<Decimal>().is_err());
    assert!(format!("0.{}1", "0".repeat(4096))
        .parse::<Decimal>()
        .is_err());
    assert!("1e9999999999999999999999999".parse::<Decimal>().is_err());
    assert_eq!(d(&format!("3.{}", "0".repeat(100))).to_string(), "3");
}

#[test]
fn equivalent_decimal_and_zero_forms_compare_equal() {
    for token in ["0", "0.0", "-0", "0e36", "-0.000"] {
        assert_eq!(d(token), d("0"));
        assert_eq!(d(token).to_string(), "0");
    }
    assert_eq!(d("1.0"), d("10e-1"));
}
