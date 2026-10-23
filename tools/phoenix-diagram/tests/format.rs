use phoenix_diagram::format::{number, time};
use phoenix_diagram::layout::waterfall;
use serde_json::Value;

fn oracle() -> Value {
    serde_json::from_str(include_str!("fixtures/format/oracle.json")).unwrap()
}

fn value(case: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(case["bits"].as_str().unwrap(), 16).unwrap())
}

#[test]
fn numbers_follow_v8_and_icu_en_us() {
    let oracle = oracle();
    let cases = oracle["numbers"].as_array().unwrap();
    assert!(cases.len() > 400);
    for case in cases {
        let v = value(case);
        let label = format!("{v:e} ({})", case["bits"]);
        assert_eq!(number::js_to_string(v), case["string"], "String {label}");
        for (precision, expected) in case["precision"].as_object().unwrap() {
            assert_eq!(
                number::js_to_precision(v, precision.parse().unwrap()),
                *expected,
                "toPrecision({precision}) {label}"
            );
        }
        assert_eq!(
            number::en_us_max_fraction(v, 2),
            case["fraction2"],
            "maximumFractionDigits {label}"
        );
        assert_eq!(
            number::en_us_max_significant(v, 3),
            case["significant3"],
            "maximumSignificantDigits {label}"
        );
        assert_eq!(waterfall::number(v), case["waterfall"], "waterfall {label}");
    }
}

#[test]
fn plan_number_boundaries_are_in_the_oracle() {
    let oracle = oracle();
    let values: Vec<f64> = oracle["numbers"]
        .as_array()
        .unwrap()
        .iter()
        .map(value)
        .collect();
    for required in [
        1.005,
        2.675,
        0.125,
        0.005,
        -0.5,
        1234567.891,
        0.001235,
        0.0012345,
        1.2345e-7,
        1.235e-7,
    ] {
        assert!(values.contains(&required), "{required}");
    }
    assert_eq!(number::en_us_max_fraction(1.005, 2), "1.01");
    assert_eq!(number::en_us_max_fraction(2.675, 2), "2.68");
    assert_eq!(number::js_round(-0.5).to_bits(), (-0.0f64).to_bits());
    assert_eq!(number::js_round(2.5), 3.0);
    assert_eq!(number::js_round(0.49999999999999994), 0.0);
}

#[test]
fn timestamps_follow_date_parse_and_the_calendar_check() {
    let oracle = oracle();
    let cases = oracle["timestamps"].as_array().unwrap();
    assert!(cases.iter().filter(|case| case["schema"] == true).count() > 30);
    for case in cases.iter().filter(|case| case["schema"] == true) {
        let at = case["at"].as_str().unwrap();
        assert_eq!(
            time::parse_timestamp(at),
            case["ms"].as_i64(),
            "Date.parse {at}"
        );
        assert_eq!(
            time::valid_calendar_date(at),
            case["calendar"] == true,
            "calendar {at}"
        );
    }
}

fn clock(wall: &time::Wall) -> String {
    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        wall.year, wall.month, wall.day, wall.hour, wall.minute, wall.second
    )
}

#[test]
fn zones_follow_icu_acceptance_wall_clock_and_offset_labels() {
    let oracle = oracle();
    let mut instants = 0;
    for case in oracle["zones"].as_array().unwrap() {
        let name = case["timezone"].as_str().unwrap();
        let zone = time::zone(name);
        assert_eq!(
            zone.is_some(),
            case["accepted"] == true,
            "accepted {name:?}"
        );
        let Some(zone) = zone else { continue };
        for sample in case["samples"].as_array().unwrap() {
            let ms = sample[0].as_i64().unwrap();
            assert_eq!(
                clock(&time::wall(ms, &zone)),
                sample[1].as_str().unwrap(),
                "wall {name} {ms}"
            );
            assert_eq!(
                time::offset_label(ms, &zone),
                sample[2].as_str().unwrap(),
                "offset {name} {ms}"
            );
            instants += 1;
        }
    }
    assert!(instants > 10_000);
}

#[test]
fn fixed_offsets_normalize_like_icu() {
    for (written, label) in [
        ("+05:30", "UTC+05:30"),
        ("+0530", "UTC+05:30"),
        ("+01", "UTC+01:00"),
        ("-2359", "UTC-23:59"),
        ("\u{2212}05:00", "UTC-05:00"),
        ("-00:00", "UTC+00:00"),
    ] {
        let zone = time::zone(written).unwrap_or_else(|| panic!("{written}"));
        assert!(matches!(zone, time::Zone::Fixed(_)), "{written}");
        assert_eq!(time::offset_label(0, &zone), label, "{written}");
    }
    for rejected in [
        "+24:00",
        "+05:60",
        "+5:30",
        "+05:30:00",
        "Z",
        "UTC+05:30",
        " +05:30",
        "+05:30 ",
    ] {
        assert!(time::zone(rejected).is_none(), "{rejected:?}");
    }
}

#[test]
fn every_system_v_zone_has_historical_and_gregorian_cycle_samples() {
    let oracle = oracle();
    for name in [
        "SystemV/AST4ADT",
        "SystemV/CST6CDT",
        "SystemV/EST5EDT",
        "SystemV/MST7MDT",
        "SystemV/PST8PDT",
        "SystemV/YST9YDT",
    ] {
        assert!(matches!(time::zone(name), Some(time::Zone::SystemV(_))));
        let case = oracle["zones"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["timezone"] == name)
            .unwrap();
        assert_eq!(case["accepted"], true);
        assert!(case["samples"].as_array().unwrap().len() > 3500, "{name}");
        assert_eq!(time::zone(&name.to_lowercase()), time::zone(name));
        assert_eq!(time::zone(&name.to_uppercase()), time::zone(name));
    }
    for name in ["SystemV/AST4EDT", "SystemV/EST5EDT ", "SystemV/ZZZ0DST"] {
        assert!(time::zone(name).is_none(), "{name}");
    }
}
