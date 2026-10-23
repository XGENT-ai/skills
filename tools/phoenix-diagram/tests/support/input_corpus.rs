use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType, InputContract};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Deserialize)]
struct Oracle {
    format: u32,
    examples: Vec<Example>,
    specimens: Vec<Specimen>,
    cases: Vec<SchemaCase>,
}

#[derive(Deserialize)]
struct Example {
    contract: String,
    kind: String,
    file: String,
    source: String,
    sha256: String,
}

#[derive(Deserialize)]
struct Specimen {
    id: String,
    contract: String,
    kind: String,
    document: Value,
    fields: Vec<String>,
}

#[derive(Deserialize)]
struct SchemaCase {
    specimen: String,
    name: String,
    operations: Vec<Operation>,
    valid: bool,
    errors: Value,
}

#[derive(Deserialize)]
struct Operation {
    op: String,
    path: String,
    value: Option<Value>,
}

fn oracle() -> Oracle {
    serde_json::from_str(include_str!("../fixtures/input/schema-oracle.json")).unwrap()
}

fn options(contract: &str) -> CompileOptions {
    CompileOptions {
        input_contract: match contract {
            "legacy-2.17" => None,
            "phoenix-v1" => Some(InputContract::PhoenixV1),
            other => panic!("Unknown fixture contract: {other}"),
        },
        ..Default::default()
    }
}

fn patch(document: &mut Value, operation: &Operation) {
    if operation.path.is_empty() {
        *document = operation.value.clone().unwrap_or(Value::Null);
        return;
    }
    let (parent_path, encoded_key) = operation.path.rsplit_once('/').unwrap();
    let key = encoded_key.replace("~1", "/").replace("~0", "~");
    let parent = document.pointer_mut(parent_path).unwrap();
    if let Some(array) = parent.as_array_mut() {
        let index = key.parse::<usize>().unwrap();
        if operation.op == "remove" {
            array.remove(index);
        } else {
            array[index] = operation.value.clone().unwrap_or(Value::Null);
        }
    } else if operation.op == "remove" {
        parent.as_object_mut().unwrap().remove(&key);
    } else {
        parent[key] = operation.value.clone().unwrap_or(Value::Null);
    }
}

#[test]
fn frozen_examples_preserve_original_documents_and_bytes_in_both_contracts() {
    let oracle = oracle();
    assert_eq!(oracle.format, 1);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/input");
    let mut counts = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    for example in oracle.examples {
        *counts.entry(example.contract.clone()).or_insert(0) += 1;
        kinds.insert((example.contract.clone(), example.kind.clone()));
        let bytes = std::fs::read(root.join(&example.file)).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            example.sha256,
            "{} copied from {}",
            example.file,
            example.source
        );
        let kind = example.kind.parse::<DiagramType>().unwrap();
        let parsed = parse_input(&bytes, kind, &options(&example.contract))
            .unwrap_or_else(|errors| panic!("{}: {errors:?}", example.file));
        assert_eq!(parsed.source_bytes, bytes, "{}", example.file);
        assert_eq!(parsed.source_sha256, example.sha256, "{}", example.file);
        assert_eq!(
            parsed.document,
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            "{}",
            example.file
        );
        assert_eq!(parsed.diagram_type, kind);
        assert_eq!(
            parsed.input_contract,
            if example.contract == "legacy-2.17" {
                InputContract::Legacy217
            } else {
                InputContract::PhoenixV1
            }
        );
    }
    assert_eq!(counts["legacy-2.17"], 14);
    assert_eq!(counts["phoenix-v1"], 25);
    assert_eq!(kinds.len(), 15);
}

#[test]
fn field_omission_enum_and_boundary_behavior_matches_frozen_node_schema_oracles() {
    let oracle = oracle();
    let specimens: BTreeMap<_, _> = oracle
        .specimens
        .iter()
        .map(|specimen| (specimen.id.as_str(), specimen))
        .collect();
    let mut valid = 0;
    let mut invalid = 0;
    for case in &oracle.cases {
        let specimen = specimens[case.specimen.as_str()];
        let mut document = specimen.document.clone();
        for operation in &case.operations {
            patch(&mut document, operation);
        }
        let diagnostics = parse_input(
            &serde_json::to_vec(&document).unwrap(),
            specimen.kind.parse().unwrap(),
            &options(&specimen.contract),
        )
        .err()
        .unwrap_or_default();
        if case.valid {
            valid += 1;
            // Synthetic schema specimens deliberately do not invent valid semantic references.
            // Schema parity is separate from the accepted-document and semantic tests below.
            assert!(
                diagnostics.iter().all(|diagnostic| matches!(
                    diagnostic.code.as_str(),
                    "input/duplicate-id"
                        | "input/unknown-reference"
                        | "input/main-path"
                        | "input/range"
                        | "input/semantic-contract"
                ) || [
                    "erd/",
                    "class/",
                    "tree/",
                    "timeline/",
                    "waterfall/"
                ]
                .iter()
                .any(|prefix| diagnostic.code.starts_with(prefix))),
                "{} {}: Node accepted; Rust: {diagnostics:?}",
                specimen.id,
                case.name
            );
        } else {
            invalid += 1;
            assert!(
                diagnostics.iter().any(|diagnostic| matches!(
                    diagnostic.code.as_str(),
                    "input/schema" | "input/type-mismatch" | "input/unknown-quality"
                )),
                "{} {}: Node rejected with {}; Rust: {diagnostics:?}",
                specimen.id,
                case.name,
                case.errors
            );
        }
    }
    assert!(
        valid > 1_000 && invalid > 1_000,
        "Incomplete oracle corpus: {valid} valid, {invalid} invalid"
    );
    for specimen in oracle.specimens {
        for field in specimen.fields {
            assert!(
                oracle
                    .cases
                    .iter()
                    .any(|case| case.specimen == specimen.id
                        && case.name == format!("{field}:omitted")),
                "Missing omission case: {} {field}",
                specimen.id
            );
        }
    }
}

// The types each contract defines; erd..waterfall exist only in phoenix-v1.
fn kinds(contract: &str) -> impl Iterator<Item = DiagramType> + '_ {
    DiagramType::ALL
        .into_iter()
        .filter(move |kind| contract == "phoenix-v1" || kind.has_legacy_contract())
}

// Types whose explicit entity and relation ids share one identity domain.
fn relational(contract: &str) -> impl Iterator<Item = DiagramType> + '_ {
    kinds(contract).filter(|kind| {
        kind.has_legacy_contract() || matches!(kind, DiagramType::Erd | DiagramType::Class)
    })
}

fn minimal(kind: DiagramType, contract: &str) -> Value {
    let mut document = json!({
        "schema_version": 1,
        "diagram_type": kind.as_str(),
        "meta": { "title": "原文 🌳 </script><svg onload=alert(1)>", "output": "corpus.html" },
    });
    let fields = match kind {
        DiagramType::Architecture => json!({
            "components": [{"id":"a","type":"frontend","label":"A"},{"id":"b","type":"backend","label":"B"}],
            "connections": [{"from":"a","to":"b","label":"request"}],
        }),
        DiagramType::Workflow => {
            document["schema_version"] = json!(2);
            json!({
                "lanes": [{"id":"work","label":"Work"}],
                "nodes": [{"id":"a","lane":"work","col":0,"type":"frontend","label":"A"},{"id":"b","lane":"work","col":1,"type":"backend","label":"B"}],
                "mainPath": ["a", "b"],
                "edges": [{"from":"a","to":"b","label":"request"}],
            })
        }
        DiagramType::Sequence => json!({
            "participants": [{"id":"a","type":"frontend","label":"A"},{"id":"b","type":"backend","label":"B"}],
            "messages": [{"from":"a","to":"b","y":160,"label":"request","note":"async → reply"}],
        }),
        DiagramType::Dataflow => json!({
            "stages": [{"label":"Input"},{"label":"Output"}],
            "nodes": [{"id":"a","type":"frontend","label":"A","stage":0,"row":0},{"id":"b","type":"backend","label":"B","stage":1,"row":0}],
            "flows": [{"from":"a","to":"b","label":"request","classification":"public"}],
        }),
        DiagramType::Lifecycle if contract == "legacy-2.17" => json!({
            "lanes": [{"id":"main","label":"Work"}],
            "states": [{"id":"a","type":"start","label":"A","lane":"main","col":0},{"id":"b","type":"success","label":"B","lane":"main","col":1}],
            "transitions": [{"from":"a","to":"b","label":"request"}],
        }),
        DiagramType::Lifecycle => {
            document["schema_version"] = json!(3);
            json!({
                "mainPath": ["a", "b"],
                "states": [{"id":"a","type":"start","label":"A"},{"id":"b","type":"success","label":"B"}],
                "transitions": [{"from":"a","to":"b","label":"request"}],
            })
        }
        DiagramType::Erd => json!({
            "entities": [
                {"id":"a","label":"A","row":0,"col":0,"attributes":[{"name":"id","type":"uuid","key":"pk"}]},
                {"id":"b","label":"B","row":0,"col":1,"attributes":[{"name":"a_id","type":"uuid","key":["fk"],"references":"a.id"}]},
            ],
            "relationships": [{"from":"a","to":"b","fromCardinality":"one","toCardinality":"many","label":"has"}],
        }),
        DiagramType::Class => json!({
            "types": [{"id":"a","label":"A","kind":"interface","row":0,"col":0},{"id":"b","label":"B","kind":"class","row":1,"col":0.5,"attributes":[{"name":"id"}]}],
            "relationships": [{"from":"b","to":"a","kind":"realization"}],
        }),
        DiagramType::Tree => json!({
            "nodes": [{"id":"a","label":"A"},{"id":"b","label":"B","parent":"a"}],
        }),
        DiagramType::Timeline => {
            document["meta"]["evidence"] = json!("observed");
            json!({
                "lanes": [{"id":"ops","label":"Ops"}],
                "events": [{"id":"a","at":"2026-01-01T10:00Z","title":"A","lane":"ops"},{"id":"b","at":"2026-01-01T11:00:30.5+08:00","title":"B","lane":"ops"}],
            })
        }
        DiagramType::Waterfall => {
            document["meta"]["evidence"] = json!("measured");
            json!({
                "spans": [{"id":"a","name":"A","start":0,"end":10},{"id":"b","name":"B","start":2,"duration":3,"parent":"a"},{"id":"c","name":"C","start":4,"status":"incomplete","parent":"a"}],
            })
        }
    };
    document
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    document
}

fn collections(kind: DiagramType) -> (&'static str, &'static str) {
    match kind {
        DiagramType::Architecture => ("components", "connections"),
        DiagramType::Workflow => ("nodes", "edges"),
        DiagramType::Sequence => ("participants", "messages"),
        DiagramType::Dataflow => ("nodes", "flows"),
        DiagramType::Lifecycle => ("states", "transitions"),
        DiagramType::Erd => ("entities", "relationships"),
        DiagramType::Class => ("types", "relationships"),
        DiagramType::Tree => ("nodes", "relationships"),
        DiagramType::Timeline => ("events", "relationships"),
        DiagramType::Waterfall => ("spans", "relationships"),
    }
}

fn expect_error(document: &Value, kind: DiagramType, contract: &str, code: &str, label: &str) {
    let diagnostics = parse_input(
        &serde_json::to_vec(document).unwrap(),
        kind,
        &options(contract),
    )
    .unwrap_err();
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.code == code),
        "{contract}/{} {label}: expected {code}, got {diagnostics:?}",
        kind.as_str()
    );
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.subject.diagram_type.as_deref() == Some(kind.as_str())));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| !diagnostic.subject.path.is_empty()),
        "{label}: diagnostics need a source pointer"
    );
}

#[test]
fn minimal_documents_and_legacy_column_fit_omission_are_accepted_unchanged() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        for kind in kinds(contract) {
            let document = minimal(kind, contract);
            let bytes = serde_json::to_vec(&document).unwrap();
            let parsed = parse_input(&bytes, kind, &options(contract))
                .unwrap_or_else(|errors| panic!("{contract}/{}: {errors:?}", kind.as_str()));
            assert_eq!(parsed.document, document);
            assert_eq!(parsed.source_bytes, bytes);
            assert_eq!(document["meta"]["column_fit"], Value::Null);
        }
        let mut workflow_v1 = minimal(DiagramType::Workflow, contract);
        workflow_v1["schema_version"] = json!(1);
        parse_input(
            &serde_json::to_vec(&workflow_v1).unwrap(),
            DiagramType::Workflow,
            &options(contract),
        )
        .unwrap();
    }
}

#[test]
fn container_namespaces_may_share_ids_with_entities_and_each_other() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        let mut document = minimal(DiagramType::Workflow, contract);
        document["lanes"][0]["id"] = json!("a");
        document["nodes"][0]["lane"] = json!("a");
        document["nodes"][1]["lane"] = json!("a");
        document["phases"] = json!([{"id":"a","label":"Phase","fromCol":0,"toCol":1}]);
        document["groups"] = json!([{"id":"a","label":"Group","lane":"a","fromCol":0,"toCol":1}]);
        document["meta"]["views"] = json!([{"id":"a","label":"View","focus":["a"]}]);
        let parsed = parse_input(
            &serde_json::to_vec(&document).unwrap(),
            DiagramType::Workflow,
            &options(contract),
        )
        .unwrap();
        assert_eq!(parsed.document, document);
    }
}

#[test]
fn duplicate_entity_relation_and_cross_collection_identities_are_rejected() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        for kind in relational(contract) {
            let (entities, relations) = collections(kind);
            let mut document = minimal(kind, contract);
            document[entities][1]["id"] = json!("a");
            expect_error(
                &document,
                kind,
                contract,
                "input/duplicate-id",
                "entity duplicate",
            );

            let mut document = minimal(kind, contract);
            document[relations][0]["id"] = json!("relation");
            let duplicate = document[relations][0].clone();
            document[relations].as_array_mut().unwrap().push(duplicate);
            expect_error(
                &document,
                kind,
                contract,
                "input/duplicate-id",
                "relation duplicate",
            );

            let mut document = minimal(kind, contract);
            document[relations][0]["id"] = json!("a");
            expect_error(
                &document,
                kind,
                contract,
                "input/duplicate-id",
                "entity/relation identity conflict",
            );

            let mut document = minimal(kind, contract);
            document["meta"]["views"] = json!([
                {"id":"view","label":"View A","focus":["a"]},
                {"id":"view","label":"View B","focus":["b"]},
            ]);
            expect_error(
                &document,
                kind,
                contract,
                "input/duplicate-id",
                "view duplicate",
            );
        }
    }
}

#[test]
fn relation_endpoints_and_guided_view_focus_require_existing_entities() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        for kind in relational(contract) {
            let (_, relations) = collections(kind);
            for endpoint in ["from", "to"] {
                let mut document = minimal(kind, contract);
                document[relations][0][endpoint] = json!("missing");
                expect_error(
                    &document,
                    kind,
                    contract,
                    "input/unknown-reference",
                    endpoint,
                );
            }
            let mut document = minimal(kind, contract);
            document["meta"]["views"] = json!([{"id":"view","label":"View","focus":["missing"]}]);
            expect_error(
                &document,
                kind,
                contract,
                "input/unknown-reference",
                "view focus",
            );
        }
    }
}

#[test]
fn boundary_lane_group_and_stage_references_are_validated_in_their_own_collections() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        let mut document = minimal(DiagramType::Architecture, contract);
        document["boundaries"] = json!([{"kind":"region","label":"Region","wraps":["missing"]}]);
        expect_error(
            &document,
            DiagramType::Architecture,
            contract,
            "input/unknown-reference",
            "boundary wraps",
        );

        let mut document = minimal(DiagramType::Workflow, contract);
        document["nodes"][0]["lane"] = json!("missing");
        expect_error(
            &document,
            DiagramType::Workflow,
            contract,
            "input/unknown-reference",
            "node lane",
        );
        let mut document = minimal(DiagramType::Workflow, contract);
        document["groups"] =
            json!([{"id":"group","label":"Group","lane":"missing","fromCol":0,"toCol":1}]);
        expect_error(
            &document,
            DiagramType::Workflow,
            contract,
            "input/unknown-reference",
            "group lane",
        );

        for collection in ["lanes", "phases", "groups"] {
            let mut document = minimal(DiagramType::Workflow, contract);
            document[collection] = match collection {
                "lanes" => json!([{"id":"work","label":"Work"}]),
                "phases" => json!([{"id":"phase","label":"Phase","fromCol":0,"toCol":1}]),
                _ => json!([{"id":"group","label":"Group","lane":"work","fromCol":0,"toCol":1}]),
            };
            let duplicate = document[collection][0].clone();
            document[collection].as_array_mut().unwrap().push(duplicate);
            expect_error(
                &document,
                DiagramType::Workflow,
                contract,
                "input/duplicate-id",
                collection,
            );
        }

        let mut document = minimal(DiagramType::Dataflow, contract);
        document["nodes"][0]["stage"] = json!(2);
        expect_error(
            &document,
            DiagramType::Dataflow,
            contract,
            "input/unknown-reference",
            "out of bounds stage index",
        );
    }
    let mut document = minimal(DiagramType::Lifecycle, "legacy-2.17");
    document["states"][0]["lane"] = json!("missing");
    expect_error(
        &document,
        DiagramType::Lifecycle,
        "legacy-2.17",
        "input/unknown-reference",
        "legacy state lane",
    );
    let mut document = minimal(DiagramType::Lifecycle, "legacy-2.17");
    document["lanes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"main","label":"Duplicate lane"}));
    expect_error(
        &document,
        DiagramType::Lifecycle,
        "legacy-2.17",
        "input/duplicate-id",
        "legacy lane duplicate",
    );
}

#[test]
fn main_paths_require_existing_unique_entities_and_real_edges_in_authored_order() {
    for (kind, contract) in [
        (DiagramType::Workflow, "legacy-2.17"),
        (DiagramType::Workflow, "phoenix-v1"),
        (DiagramType::Lifecycle, "phoenix-v1"),
    ] {
        let mut document = minimal(kind, contract);
        document["mainPath"] = json!(["a", "missing"]);
        expect_error(
            &document,
            kind,
            contract,
            "input/unknown-reference",
            "main path unknown entity",
        );
        for path in [json!(["a", "a"]), json!(["b", "a"])] {
            let mut document = minimal(kind, contract);
            document["mainPath"] = path;
            expect_error(
                &document,
                kind,
                contract,
                "input/main-path",
                "duplicate or reversed main path",
            );
        }
        let mut document = minimal(kind, contract);
        document[collections(kind).1] = json!([]);
        expect_error(
            &document,
            kind,
            contract,
            "input/main-path",
            "main path missing edge",
        );
    }
}

#[test]
fn workflow_semantic_assertions_reject_unknown_entities_before_layout() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        for assertions in [
            json!({"allowedRoots":["missing"]}),
            json!({"allowedTerminals":["missing"]}),
            json!({"requiredEdges":[{"from":"missing","to":"b"}]}),
            json!({"requiredEdges":[{"from":"a","to":"missing"}]}),
            json!({"requiredPaths":[{"from":"missing","to":"b"}]}),
            json!({"requiredPaths":[{"from":"a","to":"missing"}]}),
        ] {
            let mut document = minimal(DiagramType::Workflow, contract);
            document["semanticChecks"] = assertions;
            expect_error(
                &document,
                DiagramType::Workflow,
                contract,
                "input/unknown-reference",
                "semantic assertion",
            );
        }
    }
}

#[test]
fn workflow_semantic_contracts_distinguish_direct_edges_reachable_paths_roots_and_terminals() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        let mut document = minimal(DiagramType::Workflow, contract);
        document["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"c","lane":"work","col":2,"type":"database","label":"C"}));
        document["edges"]
            .as_array_mut()
            .unwrap()
            .push(json!({"from":"b","to":"c"}));
        document["semanticChecks"] = json!({
            "allowedRoots": ["a"],
            "allowedTerminals": ["c"],
            "requiredEdges": [{"from":"a","to":"b"}],
            "requiredPaths": [{"from":"a","to":"c"}],
        });
        parse_input(
            &serde_json::to_vec(&document).unwrap(),
            DiagramType::Workflow,
            &options(contract),
        )
        .unwrap();
        for assertions in [
            json!({"allowedRoots":[]}),
            json!({"allowedRoots":["b"]}),
            json!({"allowedTerminals":[]}),
            json!({"allowedTerminals":["a"]}),
            json!({"requiredEdges":[{"from":"a","to":"c"}]}),
            json!({"requiredPaths":[{"from":"c","to":"a"}]}),
        ] {
            let mut invalid = document.clone();
            invalid["semanticChecks"] = assertions;
            expect_error(
                &invalid,
                DiagramType::Workflow,
                contract,
                "input/semantic-contract",
                "workflow semantic contract",
            );
        }
    }
}

#[test]
fn sequence_activation_participants_and_nonempty_ranges_are_validated() {
    for contract in ["legacy-2.17", "phoenix-v1"] {
        let mut document = minimal(DiagramType::Sequence, contract);
        document["activations"] = json!([{"participant":"missing","from":160,"to":200}]);
        expect_error(
            &document,
            DiagramType::Sequence,
            contract,
            "input/unknown-reference",
            "activation participant",
        );
        for end in [160, 159] {
            for collection in ["activations", "segments"] {
                let mut document = minimal(DiagramType::Sequence, contract);
                document[collection] = if collection == "activations" {
                    json!([{"participant":"a","from":160,"to":end}])
                } else {
                    json!([{"label":"Phase","from":160,"to":end}])
                };
                expect_error(
                    &document,
                    DiagramType::Sequence,
                    contract,
                    "input/range",
                    collection,
                );
            }
        }
        for collection in ["phases", "groups"] {
            let mut document = minimal(DiagramType::Workflow, contract);
            document[collection] = if collection == "phases" {
                json!([{"id":"phase","label":"Phase","fromCol":1,"toCol":0}])
            } else {
                json!([{"id":"group","label":"Group","lane":"work","fromCol":1,"toCol":0}])
            };
            expect_error(
                &document,
                DiagramType::Workflow,
                contract,
                "input/range",
                collection,
            );
        }
    }
}

fn expect_at(document: &Value, kind: DiagramType, code: &str, path: &str) {
    let diagnostics = parse_input(
        &serde_json::to_vec(document).unwrap(),
        kind,
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.subject.path == path),
        "{}: expected {code} at {path}, got {diagnostics:?}",
        kind.as_str()
    );
}

fn accepts(document: &Value, kind: DiagramType) {
    parse_input(
        &serde_json::to_vec(document).unwrap(),
        kind,
        &CompileOptions::default(),
    )
    .unwrap_or_else(|errors| panic!("{}: {errors:?}", kind.as_str()));
}

#[test]
fn new_types_default_to_phoenix_v1_and_have_no_legacy_contract() {
    for kind in kinds("phoenix-v1").filter(|kind| !kind.has_legacy_contract()) {
        let document = minimal(kind, "phoenix-v1");
        let parsed = parse_input(
            &serde_json::to_vec(&document).unwrap(),
            kind,
            &CompileOptions::default(),
        )
        .unwrap();
        assert_eq!(parsed.input_contract, InputContract::PhoenixV1);
        let mut marked = document.clone();
        marked["meta"]["input_contract"] = json!("legacy-2.17");
        expect_at(
            &marked,
            kind,
            "input/contract-unsupported",
            "/meta/input_contract",
        );
        let diagnostics = parse_input(
            &serde_json::to_vec(&document).unwrap(),
            kind,
            &CompileOptions {
                input_contract: Some(InputContract::Legacy217),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(diagnostics[0].code, "input/contract-unsupported");
    }
    // The original five keep their unmarked legacy reading.
    let parsed = parse_input(
        &serde_json::to_vec(&minimal(DiagramType::Sequence, "legacy-2.17")).unwrap(),
        DiagramType::Sequence,
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(parsed.input_contract, InputContract::Legacy217);
}

#[test]
fn erd_attributes_references_relationships_and_grid_cells_are_checked() {
    let kind = DiagramType::Erd;
    let mut document = minimal(kind, "phoenix-v1");
    document["entities"][0]["attributes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"id"}));
    expect_at(
        &document,
        kind,
        "erd/duplicate-attribute",
        "/entities/0/attributes/1/name",
    );
    for references in ["missing.id", "a.missing"] {
        let mut document = minimal(kind, "phoenix-v1");
        document["entities"][1]["attributes"][0]["references"] = json!(references);
        let path = "/entities/1/attributes/0/references";
        expect_at(&document, kind, "input/unknown-reference", path);
    }
    let mut document = minimal(kind, "phoenix-v1");
    document["relationships"][0]["to"] = json!("a");
    expect_at(&document, kind, "erd/self-relationship", "/relationships/0");
    let mut document = minimal(kind, "phoenix-v1");
    document["entities"][1]["col"] = json!(0);
    expect_at(&document, kind, "erd/grid-cell-conflict", "/entities/1");
    let mut document = minimal(kind, "phoenix-v1");
    document["entities"][1]
        .as_object_mut()
        .unwrap()
        .remove("row");
    expect_at(&document, kind, "erd/grid-placement", "/entities/1");
    let mut document = minimal(kind, "phoenix-v1");
    for entity in document["entities"].as_array_mut().unwrap() {
        entity.as_object_mut().unwrap().remove("row");
        entity.as_object_mut().unwrap().remove("col");
    }
    expect_at(&document, kind, "erd/grid-placement", "/entities/0");
    document["entities"][0]["pos"] = json!([40, 40]);
    document["entities"][1]["pos"] = json!([400, 40]);
    accepts(&document, kind);
    let mut document = minimal(kind, "phoenix-v1");
    document["entities"][1]["attributes"][0]
        .as_object_mut()
        .unwrap()
        .remove("references");
    expect_at(&document, kind, "input/schema", "/entities/1/attributes/0");
}

#[test]
fn class_kinds_generalization_cycles_and_half_column_cells_are_checked() {
    let kind = DiagramType::Class;
    let mut document = minimal(kind, "phoenix-v1");
    document["types"][1]["attributes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"id"}));
    expect_at(
        &document,
        kind,
        "class/duplicate-attribute",
        "/types/1/attributes/1/name",
    );
    let mut document = minimal(kind, "phoenix-v1");
    document["types"][0]["kind"] = json!("class");
    expect_at(
        &document,
        kind,
        "class/realization-target",
        "/relationships/0",
    );
    let mut document = minimal(kind, "phoenix-v1");
    document["relationships"][0]["kind"] = json!("inheritance");
    expect_at(
        &document,
        kind,
        "class/inheritance-kind",
        "/relationships/0",
    );
    let mut document = minimal(kind, "phoenix-v1");
    document["types"][0]["kind"] = json!("class");
    document["relationships"] = json!([
        {"from":"b","to":"a","kind":"inheritance"},
        {"from":"a","to":"b","kind":"inheritance"},
    ]);
    expect_at(&document, kind, "class/inheritance-cycle", "/relationships");
    let mut document = minimal(kind, "phoenix-v1");
    document["types"][1]["row"] = json!(0);
    expect_at(&document, kind, "class/grid-cell-conflict", "/types/1");
    let mut document = minimal(kind, "phoenix-v1");
    document["types"][1].as_object_mut().unwrap().remove("row");
    expect_at(&document, kind, "class/grid-placement", "/types/1");
    let mut document = minimal(kind, "phoenix-v1");
    document["relationships"][0]["to"] = json!("b");
    expect_at(
        &document,
        kind,
        "class/self-relationship",
        "/relationships/0",
    );
}

#[test]
fn tree_hierarchy_has_one_root_declared_parents_and_no_cycle() {
    let kind = DiagramType::Tree;
    let mut document = minimal(kind, "phoenix-v1");
    document["nodes"][1]["id"] = json!("a");
    expect_at(&document, kind, "tree/duplicate-id", "/nodes/1/id");
    let mut document = minimal(kind, "phoenix-v1");
    document["nodes"][1]["parent"] = json!("missing");
    expect_at(&document, kind, "tree/missing-parent", "/nodes/1/parent");
    let mut document = minimal(kind, "phoenix-v1");
    document["nodes"][1]
        .as_object_mut()
        .unwrap()
        .remove("parent");
    expect_at(&document, kind, "tree/root-count", "/nodes");
    let mut document = minimal(kind, "phoenix-v1");
    document["nodes"][0]["parent"] = json!("b");
    expect_at(&document, kind, "tree/root-count", "/nodes");
    expect_at(&document, kind, "tree/cycle", "/nodes/0/parent");
    let mut document = minimal(kind, "phoenix-v1");
    document["meta"]["views"] = json!([{"id":"view","label":"View","focus":["missing"]}]);
    expect_at(
        &document,
        kind,
        "input/unknown-reference",
        "/meta/views/0/focus/0",
    );
}

#[test]
fn timeline_zones_timestamps_lanes_and_day_precision_follow_the_frozen_clock() {
    let kind = DiagramType::Timeline;
    for zone in [
        "UTC",
        "asia/calcutta",
        "Asia/Shanghai",
        "IST",
        "+05:30",
        "+0530",
        "+01",
        "-2359",
        "\u{2212}05:00",
        "-00:00",
        "SystemV/AST4ADT",
        "SystemV/CST6CDT",
        "SystemV/EST5EDT",
        "SystemV/MST7MDT",
        "SystemV/PST8PDT",
        "SystemV/YST9YDT",
    ] {
        let mut document = minimal(kind, "phoenix-v1");
        document["meta"]["timezone"] = json!(zone);
        accepts(&document, kind);
    }
    for zone in [
        "Mars/Base",
        "+24:00",
        "+05:60",
        "+5:30",
        "+05:30:00",
        "Z",
        "UTC+05:30",
        " UTC",
        "SystemV/AST4EDT",
    ] {
        let mut document = minimal(kind, "phoenix-v1");
        document["meta"]["timezone"] = json!(zone);
        expect_at(
            &document,
            kind,
            "timeline/invalid-timezone",
            "/meta/timezone",
        );
    }
    for at in [
        "2026-02-30T10:00Z",
        "2026-01-01T24:01Z",
        "2026-01-01T23:60Z",
        "2026-01-01T10:00+24:00",
        "2026-13-01T10:00Z",
    ] {
        let mut document = minimal(kind, "phoenix-v1");
        document["events"][0]["at"] = json!(at);
        expect_at(
            &document,
            kind,
            "timeline/invalid-timestamp",
            "/events/0/at",
        );
    }
    for at in [
        "2026-01-01T24:00Z",
        "2024-02-29T00:00Z",
        "0000-01-01T00:00Z",
        "2026-01-01T10:00+23:59",
    ] {
        let mut document = minimal(kind, "phoenix-v1");
        document["events"][0]["at"] = json!(at);
        accepts(&document, kind);
    }
    let mut document = minimal(kind, "phoenix-v1");
    document["lanes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"ops","label":"Again"}));
    expect_at(&document, kind, "timeline/duplicate-lane-id", "/lanes/1/id");
    let mut document = minimal(kind, "phoenix-v1");
    document["events"][1]["id"] = json!("a");
    expect_at(&document, kind, "timeline/duplicate-id", "/events/1/id");
    let mut document = minimal(kind, "phoenix-v1");
    document["events"][1]
        .as_object_mut()
        .unwrap()
        .remove("lane");
    expect_at(&document, kind, "timeline/unknown-lane", "/events/1/lane");
    for (at, valid) in [
        ("2026-01-01T12:00Z", true),
        ("2026-01-01T12:00:00.000+08:00", true),
        ("2026-01-01T12:00:01Z", false),
        ("2026-01-01T10:00Z", false),
    ] {
        let mut document = minimal(kind, "phoenix-v1");
        document["events"][0]["at"] = json!(at);
        document["events"][0]["precision"] = json!("day");
        if valid {
            accepts(&document, kind);
        } else {
            expect_at(
                &document,
                kind,
                "timeline/day-precision-time",
                "/events/0/precision",
            );
        }
    }
    let mut document = minimal(kind, "phoenix-v1");
    document["events"][0]["precision"] = json!("hour");
    expect_at(&document, kind, "input/schema", "/events/0/precision");
}

#[test]
fn waterfall_spans_need_recorded_consistent_timing_and_acyclic_parents() {
    let kind = DiagramType::Waterfall;
    let mutations: [(&str, Value, &str, &str); 6] = [
        (
            "/spans/1/id",
            json!("a"),
            "waterfall/duplicate-id",
            "/spans/1/id",
        ),
        ("/spans/0/end", json!(-1), "input/schema", "/spans/0/end"),
        (
            "/spans/1/parent",
            json!("missing"),
            "waterfall/missing-parent",
            "/spans/1/parent",
        ),
        (
            "/spans/2/status",
            json!("ok"),
            "waterfall/invalid-timing",
            "/spans/2",
        ),
        (
            "/spans/0/parent",
            json!("b"),
            "waterfall/cycle",
            "/spans/0/parent",
        ),
        (
            "/spans/1/end",
            json!(9),
            "waterfall/invalid-timing",
            "/spans/1",
        ),
    ];
    for (pointer, value, code, path) in mutations {
        let mut document = minimal(kind, "phoenix-v1");
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        document.pointer_mut(parent).unwrap()[key] = value;
        expect_at(&document, kind, code, path);
    }
    let mut document = minimal(kind, "phoenix-v1");
    document["spans"][0]["start"] = json!(20);
    expect_at(&document, kind, "waterfall/invalid-timing", "/spans/0");
    let mut document = minimal(kind, "phoenix-v1");
    document["spans"][1]["end"] = json!(5);
    accepts(&document, kind);
}

#[test]
fn deferred_fields_move_to_p3_and_repository_evidence_is_no_longer_deferred() {
    let mut document = minimal(DiagramType::Architecture, "phoenix-v1");
    document["meta"]["input_contract"] = json!("phoenix-v1");
    document["components"][0]["brand"] = json!("example");
    document["components"][0]["sources"] = json!([{"path":"src/main.rs","line":1}]);
    document["meta"]["repository"] =
        json!({"url":"https://github.com/example/repo","revision":"a".repeat(40)});
    document["meta"]["views"] = json!([{"id":"view","label":"View","focus":["a"]}]);
    let input = parse_input(
        &serde_json::to_vec(&document).unwrap(),
        DiagramType::Architecture,
        &CompileOptions::default(),
    )
    .unwrap();
    let paths: Vec<_> = input
        .compatibility
        .deferred_fields
        .iter()
        .map(|field| (field.path.as_str(), field.phase.as_str()))
        .collect();
    assert_eq!(
        paths,
        [("/meta/views", "P3"), ("/components/0/brand", "P3")]
    );
    let mut legacy = minimal(DiagramType::Sequence, "legacy-2.17");
    legacy["meta"]["animation"] = json!("trace");
    let input = parse_input(
        &serde_json::to_vec(&legacy).unwrap(),
        DiagramType::Sequence,
        &CompileOptions::default(),
    )
    .unwrap();
    assert_eq!(
        input.compatibility.deferred_fields[0].path,
        "/meta/animation"
    );
    assert_eq!(input.compatibility.deferred_fields[0].phase, "P3");
}
