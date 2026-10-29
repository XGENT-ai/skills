use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType};
use serde_json::json;

#[test]
fn legacy_presentation_has_final_dispositions_without_a_fictitious_later_phase() {
    let mut doc: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../upstream/legacy-2.17/examples/cache-miss-request.sequence.json"
    ))
    .unwrap();
    doc["participants"][0]["brand"] = json!("postgresql");
    doc["meta"]["animation"] = json!("trace");
    let input = parse_input(
        &serde_json::to_vec(&doc).unwrap(),
        DiagramType::Sequence,
        &CompileOptions::default(),
    )
    .unwrap();
    let compatibility = serde_json::to_value(input.compatibility).unwrap();
    assert_eq!(compatibility["deferredFields"], json!([]));
    assert_eq!(compatibility["status"], "partial");
    assert!(compatibility["dispositions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["path"] == "/participants/0/brand" && item["disposition"] == "supported"));
}

#[test]
fn fixed_catalog_query_alias_and_domain_never_need_a_cache() {
    use phoenix_diagram::brands;
    assert_eq!(brands::list("").matches.len(), 107);
    assert!(brands::list("not-a-brand-xyz").matches.is_empty());
    assert_eq!(brands::find("PostgreSQL").unwrap().id, "postgresql");
    assert_eq!(
        brands::find("https://docs.postgresql.org/path?token=secret")
            .unwrap()
            .id,
        "postgresql"
    );
    assert!(brands::find("https://postgresql.org.attacker.example").is_none());
    assert!(brands::find("PostgreSQL-like").is_none());
}

fn architecture(brand: serde_json::Value) -> serde_json::Value {
    json!({"schema_version":1,"diagram_type":"architecture","meta":{"title":"Brands","output":"brands.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},"components":[{"id":"db","type":"database","label":"Store","sublabel":"Owns records","pos":[100,100],"brand":brand}],"connections":[]})
}

#[test]
fn builtin_resource_is_bound_embedded_and_retains_role_and_input_bytes() {
    use phoenix_diagram::{
        brands, layout,
        render::{self, EmbeddedAssets},
    };
    let bytes = serde_json::to_vec(&architecture(json!("postgresql"))).unwrap();
    let options = CompileOptions::default();
    let mut input = parse_input(&bytes, DiagramType::Architecture, &options).unwrap();
    let initial = input.config_sha256.clone();
    let resources =
        brands::prepare(&input, std::path::Path::new("not-created-brand-cache")).unwrap();
    assert_eq!(resources.len(), 1);
    input.bind_brands(resources.clone());
    assert_ne!(input.config_sha256, initial);
    assert_eq!(input.source_bytes, bytes);
    let scene = layout::compile(&input, &options).unwrap();
    assert_eq!(scene.nodes[0].authored.facts["type"], "database");
    assert!(scene.nodes[0].brand.is_some());
    let html =
        String::from_utf8(render::render(&scene, &EmbeddedAssets::builtin()).unwrap()).unwrap();
    assert!(html.contains("data-brand-mark=\"postgresql\""));
    assert!(html.contains("Owns records"));
    assert_eq!(brands::bound(&input, &resources).unwrap(), resources);
}

#[test]
fn unknown_unpinned_and_missing_pinned_resources_fail_without_network() {
    use phoenix_diagram::brands;
    for (brand, code) in [
        (json!("not-a-brand-xyz"), "brand/unknown"),
        (json!("https://example.com/icon.png"), "brand/unpinned-url"),
        (
            json!({"url":"https://example.com/icon.png","sha256":"0".repeat(64)}),
            "brand/cache-missing",
        ),
    ] {
        let input = parse_input(
            &serde_json::to_vec(&architecture(brand)).unwrap(),
            DiagramType::Architecture,
            &CompileOptions::default(),
        )
        .unwrap();
        let errors =
            brands::prepare(&input, std::path::Path::new("not-created-brand-cache")).unwrap_err();
        assert_eq!(errors[0].code, code);
        assert!(!std::path::Path::new("not-created-brand-cache").exists());
    }
}

#[test]
fn pinned_geometry_is_not_moved_or_shrunk_to_hide_brand_overflow() {
    use phoenix_diagram::{brands, layout};
    let mut doc = architecture(json!("postgresql"));
    doc["components"][0]["size"] = json!([60, 60]);
    let options = CompileOptions::default();
    let mut input = parse_input(
        &serde_json::to_vec(&doc).unwrap(),
        DiagramType::Architecture,
        &options,
    )
    .unwrap();
    input.bind_brands(
        brands::prepare(&input, std::path::Path::new("not-created-brand-cache")).unwrap(),
    );
    let errors = layout::compile(&input, &options).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "brand/title-capacity"),
        "{errors:?}"
    );
    assert_eq!(input.document["components"][0]["pos"], json!([100, 100]));
}

#[test]
fn five_title_rails_keep_brand_label_and_semantic_role() {
    use phoenix_diagram::{
        brands, layout,
        render::{self, EmbeddedAssets},
    };
    let cases = [
        (DiagramType::Architecture, architecture(json!("postgresql"))),
        (
            DiagramType::Workflow,
            json!({"schema_version":1,"diagram_type":"workflow","meta":{"title":"Work","output":"work.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},"lanes":[{"id":"worker","label":"Worker"}],"nodes":[{"id":"a","lane":"worker","col":0,"type":"backend","label":"Store","brand":"postgresql"}],"edges":[]}),
        ),
        (
            DiagramType::Sequence,
            json!({"schema_version":1,"diagram_type":"sequence","meta":{"title":"Call","output":"call.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},"participants":[{"id":"a","type":"backend","label":"Store","brand":"postgresql"},{"id":"b","type":"backend","label":"Client"}],"messages":[{"from":"b","to":"a","label":"Read","y":180}]}),
        ),
        (
            DiagramType::Dataflow,
            json!({"schema_version":1,"diagram_type":"dataflow","meta":{"title":"Flow","output":"flow.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},"stages":[{"label":"Input"},{"label":"Store"}],"nodes":[{"id":"a","type":"backend","label":"Store","stage":0,"row":0,"brand":"postgresql"},{"id":"b","type":"backend","label":"Client","stage":1,"row":0}],"flows":[{"from":"a","to":"b","label":"Data"}]}),
        ),
        (
            DiagramType::Lifecycle,
            json!({"schema_version":3,"diagram_type":"lifecycle","meta":{"title":"State","output":"state.html","input_contract":"phoenix-v1","legend":{"mode":"hidden"}},"mainPath":["a","b"],"states":[{"id":"a","type":"start","label":"Store","brand":"postgresql"},{"id":"b","type":"success","label":"Done"}],"transitions":[{"from":"a","to":"b","label":"Commit"}]}),
        ),
    ];
    for (kind, doc) in cases {
        let options = CompileOptions::default();
        let mut input = parse_input(&serde_json::to_vec(&doc).unwrap(), kind, &options)
            .unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        input.bind_brands(
            brands::prepare(&input, std::path::Path::new("not-created-brand-cache")).unwrap(),
        );
        let scene = layout::compile(&input, &options).unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        let node = scene.nodes.iter().find(|n| n.brand.is_some()).unwrap();
        assert_eq!(node.label, "Store");
        assert_eq!(
            node.authored.facts["type"],
            if kind == DiagramType::Architecture {
                json!("database")
            } else if kind == DiagramType::Lifecycle {
                json!("start")
            } else {
                json!("backend")
            }
        );
        if kind != DiagramType::Lifecycle {
            assert!(
                node.sigil.is_some(),
                "{kind:?} lost its existing semantic sigil"
            );
        }
        let html =
            String::from_utf8(render::render(&scene, &EmbeddedAssets::builtin()).unwrap()).unwrap();
        assert!(html.contains("data-brand-mark=\"postgresql\""));
    }
}

#[test]
fn explicit_cli_catalog_and_capture_return_distinct_structured_receipts() {
    use std::process::Command;
    let bin = env!("CARGO_BIN_EXE_phoenix-diagram");
    let output = Command::new(bin)
        .args(["brands", "postgresql", "--json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["command"], "brands");
    assert_eq!(receipt["matches"][0]["id"], "postgresql");
    let output = Command::new(bin)
        .args([
            "brands",
            "capture",
            "http://127.0.0.1/icon.png?secret=x",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let receipt: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["command"], "brands capture");
    assert_eq!(receipt["diagnostics"][0]["code"], "brand/blocked");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("secret"));
    assert!(receipt["resource"].is_null());
}

#[test]
fn pinned_local_resource_delivers_offline_with_safe_facts_and_strict_identity() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use phoenix_diagram::{
        brands,
        check::FileIdentity,
        delivery::{self, CompileRequest, EngineIdentity, Status},
        input::sha256,
        render::EmbeddedAssets,
    };
    use std::fs;
    let temp = std::env::temp_dir().join(format!("phoenix-brand-delivery-{}", std::process::id()));
    fs::create_dir(&temp).unwrap();
    let cache = temp.join("cache");
    fs::create_dir(&cache).unwrap();
    let bytes = include_bytes!("fixtures/brands/official-test.png");
    let digest = sha256(bytes);
    let url = "https://example.invalid/icon.png?token=secret#session";
    let url_sha = sha256(url.as_bytes());
    let cache_file = cache.join(format!("{url_sha}-{digest}.json"));
    fs::write(&cache_file,serde_json::to_vec(&json!({"schemaVersion":1,"urlSha256":url_sha,"sourceUrl":"https://example.invalid/icon.png","resolvedUrl":"https://example.invalid/asset.png","mediaType":"image/png","bytes":bytes.len(),"sha256":digest,"dataBase64":STANDARD.encode(bytes)})).unwrap()).unwrap();
    let candidate = temp.join("input.json");
    let original = serde_json::to_vec(&architecture(json!({"url":url,"sha256":digest}))).unwrap();
    fs::write(&candidate, &original).unwrap();
    let options = CompileOptions::default();
    let assets = EmbeddedAssets::builtin();
    let engine = EngineIdentity {
        source_input_sha256: sha256(b"test engine"),
        input_contract_schema: 1,
        receipt_schema: 1,
        viewer_revision: "phoenix-v2".into(),
        binary: FileIdentity::of(b"test engine"),
    };
    let request = CompileRequest {
        kind: DiagramType::Architecture,
        candidate: &candidate,
        options: &options,
        engine: &engine,
        assets: &assets,
        repo_root: None,
        brand_cache: Some(&cache),
    };
    let output = temp.join("diagram.html");
    let receipt = delivery::deliver(&request, &output);
    assert_eq!(receipt.status, Status::Pass, "{:?}", receipt.diagnostics);
    let html = fs::read_to_string(&output).unwrap();
    assert!(html.contains("data:image/png;base64,"));
    assert!(!html.contains("token=secret"));
    assert!(!html.contains("#session"));
    assert!(!html.contains(cache.to_str().unwrap()));
    assert_eq!(fs::read(&candidate).unwrap(), original);
    let checked = delivery::check(&output, true, &engine);
    assert_eq!(checked.status, Status::Pass, "{:?}", checked.diagnostics);
    let input = parse_input(&original, DiagramType::Architecture, &options).unwrap();
    let bindings = brands::prepare(&input, &cache).unwrap();
    let mut tampered = bindings.clone();
    tampered.values_mut().next().unwrap().resource.sha256 = "0".repeat(64);
    assert!(brands::bound(&input, &tampered).is_err());
    fs::write(&cache_file, b"corrupted cache").unwrap();
    let failed = delivery::deliver(&request, &output);
    assert_eq!(failed.status, Status::Failed);
    assert_eq!(fs::read_to_string(&output).unwrap(), html);
    fs::remove_file(&cache_file).unwrap();
    let failed = delivery::validate(&request);
    assert_eq!(failed.exit_code(), 5);
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn legacy_five_types_resolve_brands_without_rewriting_authored_input() {
    use phoenix_diagram::{brands, layout};
    let cases = [
        (DiagramType::Architecture, "web-app.architecture.json"),
        (DiagramType::Workflow, "agent-tool-call.workflow.json"),
        (DiagramType::Sequence, "cache-miss-request.sequence.json"),
        (DiagramType::Dataflow, "product-analytics.dataflow.json"),
        (DiagramType::Lifecycle, "deployment-release.lifecycle.json"),
    ];
    for (kind, name) in cases {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("upstream/legacy-2.17/examples")
            .join(name);
        let mut doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let key = brands::collection(kind).unwrap();
        doc[key][0]["brand"] = json!("postgresql");
        doc[key][0]["label"] = json!("Store");
        let bytes = serde_json::to_vec(&doc).unwrap();
        let options = CompileOptions::default();
        let mut input = parse_input(&bytes, kind, &options).unwrap();
        input.bind_brands(
            brands::prepare(&input, std::path::Path::new("not-created-brand-cache")).unwrap(),
        );
        let scene = layout::compile(&input, &options).unwrap_or_else(|e| panic!("{kind:?}: {e:?}"));
        assert!(scene.nodes.iter().any(|n| n.brand.is_some()), "{kind:?}");
        assert_eq!(input.source_bytes, bytes);
        assert_eq!(input.document, doc);
    }
}
