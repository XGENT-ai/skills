use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, InputContract, Quality};
use serde_json::json;

#[path = "support/input_corpus.rs"]
mod corpus;

fn sequence() -> serde_json::Value {
    let mut document: serde_json::Value = serde_json::from_slice(include_bytes!(
        "fixtures/input/phoenix-v1/cache-miss-request.sequence.json"
    ))
    .unwrap();
    document["meta"]["input_contract"] = json!("phoenix-v1");
    document
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
        &serde_json::to_vec(
            &json!({"meta":{"input_contract":"phoenix-v1","quality_profile":"showcase"}}),
        )
        .unwrap(),
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
        br#"{"meta":{"input_contract":"phoenix-v2"}}"#,
        CompileOptions::default(),
        "input/unknown-contract",
        "/meta/input_contract",
    );
    rejects(
        br#"{"meta":{"input_contract":"phoenix-v1","quality_profile":"fast"}}"#,
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
    assert_eq!(first.input_contract, InputContract::PhoenixV1);
    assert_eq!(first.quality, Quality::Showcase);
    let explicit = parse_input(
        &bytes,
        DiagramType::Sequence,
        &CompileOptions {
            input_contract: Some(InputContract::PhoenixV1),
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
fn unmarked_original_five_and_the_retired_legacy_contract_are_rejected() {
    for kind in DiagramType::ALL {
        let marked = minimal(kind);
        let bytes = serde_json::to_vec(&marked).unwrap();
        let parsed = parse_input(&bytes, kind, &CompileOptions::default()).unwrap();
        assert_eq!(parsed.input_contract, InputContract::PhoenixV1);

        let mut unmarked = marked.clone();
        unmarked["meta"]
            .as_object_mut()
            .unwrap()
            .remove("input_contract");
        let bytes = serde_json::to_vec(&unmarked).unwrap();
        let result = parse_input(&bytes, kind, &CompileOptions::default());
        if kind.is_original_five() {
            let diagnostics = result.unwrap_err();
            assert_eq!(diagnostics[0].code, "input/contract-required", "{kind:?}");
            assert_eq!(diagnostics[0].subject.path, "/meta/input_contract");
            assert!(diagnostics[0]
                .message
                .contains("--input-contract phoenix-v1"));
            let explicit = parse_input(
                &bytes,
                kind,
                &CompileOptions {
                    input_contract: Some(InputContract::PhoenixV1),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(explicit.input_contract, InputContract::PhoenixV1);
        } else {
            assert_eq!(result.unwrap().input_contract, InputContract::PhoenixV1);
        }

        let mut legacy = marked;
        legacy["meta"]["input_contract"] = json!("legacy-2.17");
        let diagnostics = parse_input(
            &serde_json::to_vec(&legacy).unwrap(),
            kind,
            &CompileOptions::default(),
        )
        .unwrap_err();
        assert_eq!(diagnostics[0].code, "input/unknown-contract", "{kind:?}");
    }
}

#[test]
fn phoenix_output_path_keeps_every_portability_check_except_impossible_surrogates() {
    let mut document = sequence();
    document["meta"]["input_contract"] = json!("phoenix-v1");
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
fn final_dispositions_are_reported_individually_without_removing_facts() {
    let mut document = sequence();
    document["meta"]["input_contract"] = json!("phoenix-v1");
    document["participants"][0]["brand"] = json!("example-brand");
    document["meta"]["views"] =
        json!([{"id":"view","label":"View","focus":[document["participants"][0]["id"]]}]);
    document["meta"]["repository"] =
        json!({"url":"https://example.invalid/repo","revision":"a".repeat(40)});
    let input = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Sequence,
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(input.compatibility.status, "partial");
    assert_eq!(input.compatibility.core_compatibility, "pass");
    assert!(input
        .compatibility
        .dispositions
        .iter()
        .any(|field| field.path == "/participants/0/brand" && field.disposition == "supported"));
    assert!(!input
        .compatibility
        .deferred_fields
        .iter()
        .any(|field| field.path == "/meta/repository"));
    assert_eq!(input.document, document);
}

#[test]
fn stage_references_check_schema_integer_values_including_decimal_representation() {
    let mut document: serde_json::Value = serde_json::from_slice(include_bytes!(
        "fixtures/input/phoenix-v1/product-analytics.dataflow.json"
    ))
    .unwrap();
    document["meta"]["input_contract"] = json!("phoenix-v1");
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
