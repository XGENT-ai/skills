use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, Quality};
use serde_json::json;

#[path = "support/input_corpus.rs"]
mod corpus;

fn sequence() -> serde_json::Value {
    serde_json::from_slice(include_bytes!(
        "fixtures/input/phoenix-v1/cache-miss-request.sequence.json"
    ))
    .unwrap()
}

fn rejects(bytes: &[u8], options: CompileOptions, code: &str, path: &str) {
    let diagnostics = parse_input(bytes, DiagramType::Sequence, &options).unwrap_err();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.subject.path == path),
        "Expected {code} at {path}, got {diagnostics:?}"
    );
}

#[test]
fn unsupported_types_are_rejected_before_reading_input() {
    for kind in ["unknown", "Sequence", "ERD", "chart", "dashboard"] {
        let diagnostic = kind.parse::<DiagramType>().unwrap_err().remove(0);
        assert_eq!(diagnostic.code, "input/unsupported-type");
    }
}

#[test]
fn invalid_utf8_and_unpaired_surrogates_are_rejected() {
    rejects(&[0xff], CompileOptions::default(), "input/json", "");
    rejects(
        br#"{"title":"\uD800"}"#,
        CompileOptions::default(),
        "input/json",
        "",
    );
    rejects(
        br#"{"title":"\uDFFF"}"#,
        CompileOptions::default(),
        "input/json",
        "",
    );
}

#[test]
fn explicit_quality_cannot_override_persisted_quality() {
    rejects(
        &serde_json::to_vec(&json!({"meta":{"quality_profile":"showcase"}})).unwrap(),
        CompileOptions {
            quality: Some(Quality::Standard),
            ..Default::default()
        },
        "input/quality-conflict",
        "/meta/quality_profile",
    );
}

#[test]
fn unknown_persisted_options_fail_before_schema_or_layout() {
    rejects(
        br#"{"meta":{"quality_profile":"fast"}}"#,
        CompileOptions::default(),
        "input/unknown-quality",
        "/meta/quality_profile",
    );
}

#[test]
fn source_bytes_and_text_are_preserved_and_options_are_bound() {
    let mut document = sequence();
    document["meta"]["title"] = json!("中文 🦀 </script><script>alert(1)</script> $& \" '&");
    document["meta"]
        .as_object_mut()
        .unwrap()
        .remove("quality_profile");
    let bytes = serde_json::to_vec_pretty(&document).unwrap();
    let first = parse_input(&bytes, DiagramType::Sequence, &CompileOptions::default()).unwrap();
    assert_eq!(first.document, document);
    assert_eq!(first.source_bytes, bytes);
    assert_eq!(first.quality, Quality::Showcase);
    let explicit = parse_input(
        &bytes,
        DiagramType::Sequence,
        &CompileOptions {
            quality: Some(Quality::Showcase),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(first.config_sha256, explicit.config_sha256);
    let standard = parse_input(
        &bytes,
        DiagramType::Sequence,
        &CompileOptions {
            quality: Some(Quality::Standard),
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(standard.config_sha256, first.config_sha256);
    let viewer = parse_input(
        &bytes,
        DiagramType::Sequence,
        &CompileOptions {
            viewer_revision: "another-viewer".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(viewer.config_sha256, first.config_sha256);
    assert_eq!(first.source_sha256.len(), 64);
    assert_eq!(first.config_sha256.len(), 64);
}

fn minimal(kind: DiagramType) -> serde_json::Value {
    let path = format!(
        "{}/../../skills/phoenix-diagram/examples/minimal.{}.json",
        env!("CARGO_MANIFEST_DIR"),
        kind.as_str()
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn every_type_rejects_unknown_meta_fields_through_its_schema() {
    for kind in DiagramType::ALL {
        let document = minimal(kind);
        parse_input(
            &serde_json::to_vec(&document).unwrap(),
            kind,
            &CompileOptions::default(),
        )
        .unwrap_or_else(|errors| panic!("{kind:?}: {errors:?}"));
        for (field, value) in [
            ("input_contract", json!("phoenix-v1")),
            ("views", json!([{"id":"view","label":"View","focus":["a"]}])),
        ] {
            let mut extended = document.clone();
            extended["meta"][field] = value;
            let diagnostics = parse_input(
                &serde_json::to_vec(&extended).unwrap(),
                kind,
                &CompileOptions::default(),
            )
            .unwrap_err();
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.code == "input/schema"
                        && diagnostic.subject.path == "/meta"),
                "{kind:?} meta.{field}: {diagnostics:?}"
            );
        }
    }
}

#[test]
fn phoenix_output_path_keeps_every_portability_check_except_impossible_surrogates() {
    let mut document = sequence();
    for path in ["out/🦀.html", "中文/图.HTML"] {
        document["meta"]["output"] = json!(path);
        parse_input(
            &serde_json::to_vec(&document).unwrap(),
            DiagramType::Sequence,
            &CompileOptions::default(),
        )
        .unwrap();
    }
    for path in [
        "/absolute.html",
        "../escape.html",
        "a/../escape.html",
        "a\\b.html",
        "https://x/y.html",
        "CON.html",
        "a. /b.html",
        "no-extension",
        "a\u{0}.html",
        "a/<b>.html",
    ] {
        document["meta"]["output"] = json!(path);
        let diagnostics = parse_input(
            &serde_json::to_vec(&document).unwrap(),
            DiagramType::Sequence,
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "input/schema"
                    && diagnostic.subject.path == "/meta/output"),
            "{path:?}: {diagnostics:?}"
        );
    }
}

#[test]
fn valid_surrogate_pair_is_decoded_without_changing_raw_input() {
    let bytes = serde_json::to_string(&sequence())
        .unwrap()
        .replace("Cache", r"\uD83E\uDD80")
        .into_bytes();
    let input = parse_input(&bytes, DiagramType::Sequence, &CompileOptions::default()).unwrap();
    assert_eq!(input.source_bytes, bytes);
    assert!(input.document["meta"]["title"]
        .as_str()
        .unwrap()
        .contains('🦀'));
}

#[test]
fn duplicate_ids_endpoints_and_numeric_ranges_fail_before_layout() {
    let original = sequence();
    let mut duplicate = original.clone();
    let same = duplicate["participants"][0].clone();
    duplicate["participants"].as_array_mut().unwrap().push(same);
    rejects(
        &serde_json::to_vec(&duplicate).unwrap(),
        CompileOptions::default(),
        "input/duplicate-id",
        &format!(
            "/participants/{}/id",
            duplicate["participants"].as_array().unwrap().len() - 1
        ),
    );
    let mut cross = original.clone();
    cross["messages"][0]["id"] = cross["participants"][0]["id"].clone();
    rejects(
        &serde_json::to_vec(&cross).unwrap(),
        CompileOptions::default(),
        "input/duplicate-id",
        "/messages/0/id",
    );
    let mut unknown = original.clone();
    unknown["messages"][0]["to"] = json!("missing");
    rejects(
        &serde_json::to_vec(&unknown).unwrap(),
        CompileOptions::default(),
        "input/unknown-reference",
        "/messages/0/to",
    );
    let mut range = original;
    range["activations"] = json!([{"participant":"user","from":300,"to":200}]);
    rejects(
        &serde_json::to_vec(&range).unwrap(),
        CompileOptions::default(),
        "input/range",
        "/activations/0/to",
    );
}

#[test]
fn stage_references_check_schema_integer_values_including_decimal_representation() {
    let mut document: serde_json::Value = serde_json::from_slice(include_bytes!(
        "fixtures/input/phoenix-v1/product-analytics.dataflow.json"
    ))
    .unwrap();
    document["schema_version"] = json!(1.0);
    document["nodes"][0]["stage"] = json!(0.0);
    parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Dataflow,
        &CompileOptions::default(),
    )
    .unwrap();
    document["nodes"][0]["stage"] = json!(document["stages"].as_array().unwrap().len() as f64);
    let diagnostics = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Dataflow,
        &CompileOptions::default(),
    )
    .expect_err("An out-of-bounds floating representation of an integer stage must fail");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "input/unknown-reference"
                && diagnostic.subject.path == "/nodes/0/stage"),
        "{diagnostics:?}"
    );
}
