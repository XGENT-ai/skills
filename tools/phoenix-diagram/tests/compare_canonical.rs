use phoenix_diagram::compare::canonical::{
    canonical_architecture, canonical_json, compare_architecture,
};
use serde_json::{json, Value};

fn snapshot() -> Value {
    json!({
        "schema_version": 1, "diagram_type": "architecture",
        "meta": {"title": "Service", "output": "before.html"},
        "components": [
            {"id":"api", "type":"backend", "label":"API", "pos":[100,100], "size":[160,80], "sources":[{"path":"src/b.rs","line":2},{"path":"src/a.rs","line":1}]},
            {"id":"db", "type":"database", "label":"Database", "pos":[400,100], "size":[160,80]}
        ],
        "connections": [{"id":"query", "from":"api", "to":"db", "label":"Query", "variant":"sync"}],
        "boundaries": [{"kind":"region", "label":"System", "wraps":["api","db"], "pad":20}],
        "cards":[]
    })
}
fn compare(base: &Value, head: &Value) -> Value {
    compare_architecture(base, head, &json!({})).unwrap()
}
fn failure(base: &Value, head: &Value, code: &str) -> Value {
    let diagnostics = compare_architecture(base, head, &json!({})).unwrap_err();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].stage, "compare");
    assert_eq!(diagnostics[0].code, code);
    diagnostics[0].evidence.clone()
}

#[test]
fn canonical_is_stable_across_authored_set_order_output_and_object_key_order() {
    let base = snapshot();
    let mut reordered = base.clone();
    reordered["meta"]["output"] = json!("another.html");
    reordered["components"].as_array_mut().unwrap().reverse();
    reordered["components"][1]["sources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reordered["boundaries"][0]["wraps"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(canonical_json(&base), canonical_json(&reordered));
    assert!(canonical_architecture(&base)["meta"]
        .get("output")
        .is_none());
    assert_eq!(
        compare(&base, &reordered)["changes"],
        json!({"components":[],"connections":[],"boundaries":[]})
    );
    assert_eq!(
        compare(&base, &reordered)["summary"]["presentationChanged"],
        false
    );
}

#[test]
fn canonical_bytes_use_javascript_numbers_and_utf16_key_order() {
    let mut diagram = snapshot();
    diagram["components"][0]["pos"] = serde_json::from_str("[100.0,-0.0]").unwrap();
    diagram["meta"]["\u{e000}"] = json!(1e21);
    diagram["meta"]["😀"] = json!(1e-7);
    let text = String::from_utf8(canonical_json(&diagram)).unwrap();
    assert!(text.contains("\"pos\":[100,0]"), "{text}");
    assert!(text.contains("\"😀\":1e-7,\"\u{e000}\":1e+21"), "{text}");
}

#[test]
fn classifications_are_authored_field_groups_and_semantics_take_precedence() {
    let base = snapshot();
    let cases = [
        (
            "/components/0/sublabel",
            json!("New responsibility"),
            "components",
            "changed",
            json!(["semantic"]),
            json!(["/sublabel"]),
        ),
        (
            "/components/0/brand",
            json!("redis"),
            "components",
            "changed",
            json!(["semantic"]),
            json!(["/brand"]),
        ),
        (
            "/components/0/icon",
            json!("clock"),
            "components",
            "changed",
            json!(["semantic"]),
            json!(["/icon"]),
        ),
        (
            "/components/0/pos",
            json!([120, 100]),
            "components",
            "moved",
            json!(["geometry"]),
            json!(["/pos"]),
        ),
        (
            "/components/0/sources",
            json!([{"path":"src/new.rs","line":1}]),
            "components",
            "evidence-changed",
            json!(["evidence"]),
            json!(["/sources"]),
        ),
        (
            "/connections/0/via",
            json!([[200, 100], [200, 200]]),
            "connections",
            "rerouted",
            json!(["geometry"]),
            json!(["/via"]),
        ),
        (
            "/connections/0/to",
            json!("api"),
            "connections",
            "changed",
            json!(["topology"]),
            json!(["/to"]),
        ),
        (
            "/boundaries/0/pad",
            json!(24),
            "boundaries",
            "geometry-changed",
            json!(["geometry"]),
            json!(["/pad"]),
        ),
        (
            "/boundaries/0/wraps",
            json!(["api"]),
            "boundaries",
            "changed",
            json!(["scope"]),
            json!(["/wraps"]),
        ),
    ];
    for (pointer, value, collection, status, classifications, changed_fields) in cases {
        let mut head = base.clone();
        let (parent, field) = pointer.rsplit_once('/').unwrap();
        head.pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(field.into(), value);
        let receipt = compare(&base, &head);
        let changes = receipt["changes"][collection].as_array().unwrap();
        assert_eq!(changes.len(), 1, "{pointer}");
        assert_eq!(changes[0]["status"], status, "{pointer}");
        assert_eq!(changes[0]["classifications"], classifications, "{pointer}");
        assert_eq!(changes[0]["changedFields"], changed_fields, "{pointer}");
    }
    let mut head = base.clone();
    head["components"][0]["pos"] = json!([120, 100]);
    head["components"][0]["label"] = json!("New label");
    head["components"][0]["sources"] = json!([]);
    let receipt = compare(&base, &head);
    let change = &receipt["changes"]["components"][0];
    assert_eq!(change["status"], "changed");
    assert_eq!(
        change["classifications"],
        json!(["evidence", "geometry", "semantic"])
    );
    assert_eq!(
        change["changedFields"],
        json!(["/label", "/pos", "/sources"])
    );
    assert_eq!(receipt["summary"]["components"]["changed"], 1);
    assert_eq!(receipt["summary"]["components"]["moved"], 0);
}

#[test]
fn stable_ids_define_add_remove_and_boundary_derived_identity() {
    let base = snapshot();
    let mut head = base.clone();
    head["components"][1]["id"] = json!("cache");
    head["connections"][0]["id"] = json!("cache-query");
    head["boundaries"][0]["label"] = json!("Renamed system");
    let receipt = compare(&base, &head);
    assert_eq!(
        receipt["summary"]["components"],
        json!({"added":1,"changed":0,"evidenceChanged":0,"removed":1,"moved":0})
    );
    assert_eq!(
        receipt["changes"]["components"],
        json!([
            {"id":"cache","headLabel":"Database","status":"added","classifications":["semantic"],"changedFields":[]},
            {"id":"db","baseLabel":"Database","status":"removed","classifications":["semantic"],"changedFields":[]}
        ])
    );
    assert_eq!(
        receipt["changes"]["connections"][0]["classifications"],
        json!(["topology"])
    );
    assert_eq!(
        receipt["changes"]["boundaries"][0]["classifications"],
        json!(["scope"])
    );
    assert_eq!(
        receipt["changes"]["boundaries"][0]["key"],
        "region:Renamed system"
    );
}

#[test]
fn missing_duplicate_ambiguous_and_unrelated_identity_fail_with_frozen_diagnostics() {
    let base = snapshot();
    let mut head = base.clone();
    head["connections"][0].as_object_mut().unwrap().remove("id");
    assert_eq!(
        failure(&base, &head, "delta/relationship-id-required")["paths"],
        json!(["/connections/0/id"])
    );
    head = base.clone();
    head["components"][0].as_object_mut().unwrap().remove("id");
    assert_eq!(
        failure(&base, &head, "delta/stable-id-required")["side"],
        "head"
    );
    head = base.clone();
    head["components"][1]["id"] = json!("api");
    assert_eq!(
        failure(&base, &head, "delta/duplicate-stable-id")["ids"],
        json!(["api"])
    );
    head = base.clone();
    let duplicate = head["boundaries"][0].clone();
    head["boundaries"].as_array_mut().unwrap().push(duplicate);
    assert_eq!(
        failure(&base, &head, "delta/boundary-key-ambiguous")["boundaries"],
        json!(["region:System"])
    );
    head = base.clone();
    head["components"] = json!([{"id":"unrelated","label":"Other"}]);
    failure(&base, &head, "delta/no-shared-component-id");
    head = base.clone();
    head["schema_version"] = json!(2);
    assert_eq!(
        failure(&base, &head, "delta/schema-version-mismatch")["actual"],
        2
    );
    head = base.clone();
    head["diagram_type"] = json!("workflow");
    failure(&base, &head, "delta/type-mismatch");
}

#[test]
fn verified_pinned_same_repository_is_required_for_revision_proof() {
    let mut base = snapshot();
    let mut head = base.clone();
    base["meta"]["repository"] =
        json!({"url":"git@github.com:Example/Service.git","revision":"A".repeat(40)});
    head["meta"]["repository"] =
        json!({"url":"https://github.com/example/service","revision":"b".repeat(40)});
    let evidence = json!({"baseVerified":true,"headVerified":true,"baseRawSha256":"base-raw","headSemanticSha256":"head-semantic","baseBytes":123});
    let receipt = compare_architecture(&base, &head, &evidence).unwrap();
    assert_eq!(receipt["proofLevel"], "revision-pinned");
    assert_eq!(
        receipt["base"],
        json!({"title":"Service","rawSha256":"base-raw","bytes":123,"revision":"a".repeat(40)})
    );
    assert_eq!(receipt["head"]["semanticSha256"], "head-semantic");
    assert_eq!(
        receipt["provenance"]["changedFields"],
        json!(["/revision", "/url"])
    );
    assert_eq!(
        receipt["provenance"]["base"],
        json!({"revision":"a".repeat(40)})
    );
    assert_eq!(receipt["summary"]["provenanceChanged"], true);
    assert_eq!(compare(&base, &head)["proofLevel"], "authored");
    head["meta"]["repository"]["revision"] = json!("main");
    assert_eq!(
        compare_architecture(&base, &head, &evidence).unwrap()["proofLevel"],
        "authored"
    );
    head["meta"]["repository"]["url"] = json!("https://github.com/example/different");
    failure(&base, &head, "delta/repository-mismatch");
}

#[test]
fn presentation_and_provenance_do_not_invent_entity_changes_and_null_is_authored() {
    let base = snapshot();
    let mut head = base.clone();
    head["meta"]["legend"] = json!({"entries":{"database":{"visible":false}}});
    let receipt = compare(&base, &head);
    assert_eq!(receipt["summary"]["presentationChanged"], true);
    assert_eq!(
        receipt["changes"],
        json!({"components":[],"connections":[],"boundaries":[]})
    );
    head = base.clone();
    head["components"][0]["sublabel"] = Value::Null;
    assert_eq!(
        compare(&base, &head)["changes"]["components"][0]["changedFields"],
        json!(["/sublabel"])
    );
}
