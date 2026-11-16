use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

// Single-input change marking on workflow: effective marks, counts, band and
// artifact agreement (plan §5.2/§5.4; the V-5 workflow portion of M1).

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "phoenix-diagram-change-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, value: &Value) -> PathBuf {
        let file = self.0.join(name);
        fs::write(&file, serde_json::to_vec(value).unwrap()).unwrap();
        file
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn engine() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_phoenix-diagram"))
}

fn run(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(engine()).args(args).output().unwrap()
}

fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "{e}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn flag(value: &str) -> &std::ffi::OsStr {
    value.as_ref()
}

fn codes(receipt: &Value) -> Vec<(String, String)> {
    receipt["diagnostics"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        item["code"].as_str().unwrap_or_default().to_owned(),
                        item["subject"]["path"].as_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn marked_workflow() -> Value {
    json!({
        "schema_version": 2,
        "diagram_type": "workflow",
        "meta": {"title": "Change smoke", "output": "change.html", "legend": {"mode": "hidden"}},
        "lanes": [{"id": "lane-a", "label": "Prep"}, {"id": "lane-b", "label": "Run"}],
        "nodes": [
            {"id": "a", "lane": "lane-a", "col": 0, "type": "frontend", "label": "Start"},
            {"id": "b", "lane": "lane-a", "col": 1, "type": "backend", "label": "Prepare", "change": "changed"},
            {"id": "c", "lane": "lane-b", "col": 2, "type": "backend", "label": "Annotate", "change": "added"},
            {"id": "d", "lane": "lane-b", "col": 3, "type": "backend", "label": "Deliver"}
        ],
        "edges": [
            {"from": "a", "to": "b", "label": "draft", "change": "removed"},
            {"from": "a", "to": "c", "label": "route", "change": "added"},
            {"from": "c", "to": "d", "label": "then", "change": "added"},
            {"from": "b", "to": "d", "label": "merge"}
        ]
    })
}

fn unmarked_workflow() -> Value {
    let mut value = marked_workflow();
    for node in value["nodes"].as_array_mut().unwrap() {
        node.as_object_mut().unwrap().remove("change");
    }
    for edge in value["edges"].as_array_mut().unwrap() {
        edge.as_object_mut().unwrap().remove("change");
    }
    value
}

#[test]
fn unmarked_workflow_keeps_the_scene_free_of_change_facts() {
    let project = Project::new();
    let candidate = project.write("candidate.json", &unmarked_workflow());
    let output = run(&[
        flag("validate"),
        flag("workflow"),
        candidate.as_os_str(),
        flag("--layout-json"),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let scene = &parsed(&output)["layout"];
    assert!(scene.get("changes").is_none(), "unmarked scene must not carry change facts");
    let html = project.0.join("plain.html");
    let output = run(&[
        flag("deliver"),
        flag("workflow"),
        candidate.as_os_str(),
        html.as_os_str(),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let body = fs::read_to_string(&html).unwrap();
    // The figure SVG must be free of change surface; the reader shell's
    // stylesheet and fragments legitimately mention the attribute names.
    let svg_start = body
        .find("<svg xmlns=\"http://www.w3.org/2000/svg\"")
        .expect("figure svg");
    let svg_end = body[svg_start..].find("</svg>").unwrap() + svg_start;
    let figure = &body[svg_start..svg_end];
    assert!(!figure.contains("data-change-ui"));
    assert!(!figure.contains("data-change-band"));
    assert!(!figure.contains("data-change-mark"));
}

#[test]
fn workflow_marks_produce_counts_badges_band_and_a_clean_check() {
    let project = Project::new();
    let candidate = project.write("candidate.json", &marked_workflow());
    let output = run(&[
        flag("validate"),
        flag("workflow"),
        candidate.as_os_str(),
        flag("--layout-json"),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let scene = parsed(&output);
    let changes = &scene["layout"]["changes"];
    assert_eq!(changes["counts"], json!({"added": 3, "removed": 1, "changed": 1}));
    assert_eq!(changes["nodes"], json!({"b": "changed", "c": "added"}));
    // Aligned with scene.relations order, which is route order: the four
    // authored marks appear exactly once each.
    let marks: Vec<Value> = changes["relations"].as_array().unwrap().clone();
    let mut sorted: Vec<String> = marks
        .iter()
        .map(|mark| mark.as_str().unwrap_or("None").to_owned())
        .collect();
    sorted.sort();
    assert_eq!(sorted, vec!["None", "added", "added", "removed"]);
    assert!(changes["band"]["bounds"]["height"].as_f64().unwrap() > 0.0);
    let view_box = &scene["layout"]["viewBox"];
    assert!(
        view_box["height"].as_f64().unwrap()
            > changes["band"]["bounds"]["height"].as_f64().unwrap()
    );

    let html = project.0.join("marked.html");
    let output = run(&[
        flag("deliver"),
        flag("workflow"),
        candidate.as_os_str(),
        html.as_os_str(),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let body = fs::read_to_string(&html).unwrap();
    let svg_start = body
        .find("<svg xmlns=\"http://www.w3.org/2000/svg\"")
        .expect("figure svg");
    let svg_end = body[svg_start..].find("</svg>").unwrap() + svg_start;
    let figure = &body[svg_start..svg_end];
    assert!(figure.contains("data-change-ui"));
    assert!(figure.contains("data-change-view=\"changes\""));
    assert!(figure.contains("data-change-band"));
    // One badge per mark: 2 added edges + 1 added node, 1 removed edge,
    // 1 changed node.
    assert_eq!(figure.matches("<g data-change-mark=\"added\" role=\"img\"").count(), 3);
    assert_eq!(figure.matches("<g data-change-mark=\"removed\" role=\"img\"").count(), 1);
    assert_eq!(figure.matches("<g data-change-mark=\"changed\" role=\"img\"").count(), 1);
    assert!(figure.contains("data-change-count=\"added\">3</tspan>"));
    assert!(figure.contains("data-change-count=\"removed\">1</tspan>"));
    assert!(figure.contains("data-change-count=\"changed\">1</tspan>"));
    // A changed node exists, so the before/after groups carry the note.
    assert_eq!(figure.matches("data-change-note").count() >= 2, true);

    let output = run(&[flag("check"), html.as_os_str(), flag("--json")]);
    assert_eq!(output.status.code(), Some(0));
    let receipt = parsed(&output);
    assert_eq!(receipt["status"], "pass");
    assert!(
        codes(&receipt).iter().all(|(code, _)| code != "artifact/change-mark"),
        "artifact check must agree with the change facts"
    );
}

#[test]
fn a_changed_endpoint_keeps_its_relations_in_both_views() {
    let project = Project::new();
    let mut value = marked_workflow();
    // Node b is changed; its unmarked incoming relation must not dangle.
    value["edges"][0].as_object_mut().unwrap().remove("change");
    let candidate = project.write("candidate.json", &value);
    let output = run(&[
        flag("validate"),
        flag("workflow"),
        candidate.as_os_str(),
        flag("--layout-json"),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{:?}", parsed(&output));
    let changes = parsed(&output)["layout"]["changes"].clone();
    assert_eq!(changes["relations"][0], Value::Null);
}

#[test]
fn removed_and_added_endpoints_demand_matching_relation_marks() {
    let project = Project::new();
    // Removed endpoint with an unmarked relation dangles in the after view.
    let mut value = unmarked_workflow();
    value["nodes"][0]["change"] = json!("removed");
    let candidate = project.write("removed.json", &value);
    let output = run(&[flag("validate"), flag("workflow"), candidate.as_os_str(), flag("--json")]);
    assert_eq!(output.status.code(), Some(1));
    let receipt = parsed(&output);
    assert!(codes(&receipt).iter().any(|(code, path)| {
        code == "change/dangling-relation" && path == "/edges/0"
    }));
    let diagnostic = receipt["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["code"] == "change/dangling-relation")
        .unwrap()
        .clone();
    assert!(
        diagnostic["supportedFixes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fix| fix.as_str().unwrap().contains("/edges/0/change")),
        "the fix must name the authored relation"
    );

    // Added endpoint with an unmarked relation dangles in the before view.
    let mut value = unmarked_workflow();
    value["nodes"][2]["change"] = json!("added");
    let candidate = project.write("added.json", &value);
    let output = run(&[flag("validate"), flag("workflow"), candidate.as_os_str(), flag("--json")]);
    assert_eq!(output.status.code(), Some(1));
    let receipt = parsed(&output);
    assert!(codes(&receipt).iter().any(|(code, path)| {
        code == "change/dangling-relation" && path == "/edges/2"
    }));
}

#[test]
fn relation_and_value_marks_outside_the_contract_are_schema_errors() {
    let project = Project::new();
    // A relation cannot be "changed": rewriting it is removal plus addition.
    let mut value = unmarked_workflow();
    value["edges"][0]["change"] = json!("changed");
    let candidate = project.write("relation-changed.json", &value);
    let output = run(&[flag("validate"), flag("workflow"), candidate.as_os_str(), flag("--json")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(codes(&parsed(&output)).iter().any(|(code, path)| {
        code == "input/schema" && path == "/edges/0/change"
    }));

    let mut value = unmarked_workflow();
    value["nodes"][0]["change"] = json!("moved");
    let candidate = project.write("unknown-value.json", &value);
    let output = run(&[flag("validate"), flag("workflow"), candidate.as_os_str(), flag("--json")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(codes(&parsed(&output)).iter().any(|(code, path)| {
        code == "input/schema" && path == "/nodes/0/change"
    }));
}

#[test]
fn tree_reparent_appends_a_before_connection_and_counts_both_views() {
    let project = Project::new();
    let value = json!({
        "schema_version": 1,
        "diagram_type": "tree",
        "meta": {"title": "Reparent", "output": "tree.html"},
        "nodes": [
            {"id": "r", "label": "Root"},
            {"id": "a", "parent": "r", "label": "Branch A"},
            {"id": "g", "parent": "a", "label": "Leaf G", "parent_before": "r"},
            {"id": "b", "parent": "a", "label": "Leaf B"}
        ]
    });
    let candidate = project.write("tree.json", &value);
    let output = run(&[
        flag("validate"),
        flag("tree"),
        candidate.as_os_str(),
        flag("--layout-json"),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{:?}", parsed(&output));
    let scene = parsed(&output);
    let changes = &scene["layout"]["changes"];
    assert_eq!(changes["counts"], json!({"added": 1, "removed": 1, "changed": 0}));
    let relations = scene["layout"]["relations"].as_array().unwrap();
    assert!(relations.iter().any(|relation| {
        relation["authored"]["sourcePath"].as_str() == Some("/nodes/2/parent_before")
            && relation["authored"]["kind"] == "tree-edge-before"
    }));
    let before_index = relations
        .iter()
        .position(|relation| relation["authored"]["kind"] == "tree-edge-before")
        .unwrap();
    assert_eq!(changes["relations"][before_index], json!("removed"));
}

#[test]
fn erd_entity_marks_reach_every_field_row_and_the_artifact() {
    let project = Project::new();
    let value = json!({
        "schema_version": 1,
        "diagram_type": "erd",
        "meta": {"title": "Erd change", "output": "erd.html"},
        "entities": [
            {"id": "users", "label": "users", "row": 0, "col": 0, "change": "changed",
             "attributes": [
                {"name": "id", "type": "int", "key": "pk"},
                {"name": "email", "type": "text"}
             ]},
            {"id": "orders", "label": "orders", "row": 0, "col": 3,
             "attributes": [{"name": "id", "type": "int", "key": "pk"}]}
        ],
        "relationships": [
            {"from": "users", "to": "orders", "fromCardinality": "one", "toCardinality": "many"}
        ]
    });
    let candidate = project.write("erd.json", &value);
    let output = run(&[
        flag("validate"),
        flag("erd"),
        candidate.as_os_str(),
        flag("--layout-json"),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{:?}", parsed(&output));
    let scene = parsed(&output);
    let changes = &scene["layout"]["changes"];
    assert_eq!(changes["nodes"], json!({"users": "changed"}));
    let fields = changes["fields"].as_object().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.values().all(|mark| mark == "changed"));
    assert_eq!(changes["counts"]["changed"], 3);

    let html = project.0.join("erd.html");
    let output = run(&[
        flag("deliver"),
        flag("erd"),
        candidate.as_os_str(),
        html.as_os_str(),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let body = fs::read_to_string(&html).unwrap();
    assert_eq!(body.matches("er-change-row er-change-row-changed").count(), 2);
    let output = run(&[flag("check"), html.as_os_str(), flag("--json")]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(parsed(&output)["status"], "pass");
}

#[test]
fn lifecycle_transition_marks_count_and_pass_the_artifact_check() {
    let project = Project::new();
    let value = json!({
        "schema_version": 3,
        "diagram_type": "lifecycle",
        "meta": {"title": "Lifecycle change", "output": "life.html"},
        "mainPath": ["open", "busy", "done"],
        "states": [
            {"id": "open", "type": "start", "label": "Open"},
            {"id": "busy", "type": "active", "label": "Busy"},
            {"id": "done", "type": "success", "label": "Done"},
            {"id": "blocked", "type": "active", "label": "Blocked", "change": "added"}
        ],
        "transitions": [
            {"from": "open", "to": "busy", "label": "start"},
            {"from": "busy", "to": "done", "label": "finish", "change": "removed"},
            {"from": "open", "to": "blocked", "label": "stall", "change": "added"},
            {"from": "blocked", "to": "open", "label": "resume", "change": "added"}
        ]
    });
    let candidate = project.write("life.json", &value);
    let output = run(&[
        flag("validate"),
        flag("lifecycle"),
        candidate.as_os_str(),
        flag("--layout-json"),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{:?}", parsed(&output));
    let changes = parsed(&output)["layout"]["changes"].clone();
    assert_eq!(changes["counts"], json!({"added": 3, "removed": 1, "changed": 0}));
    assert!(changes["junctions"].is_array());

    let html = project.0.join("life.html");
    let output = run(&[
        flag("deliver"),
        flag("lifecycle"),
        candidate.as_os_str(),
        html.as_os_str(),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let output = run(&[flag("check"), html.as_os_str(), flag("--json")]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(parsed(&output)["status"], "pass");
}

#[test]
fn band_notes_appear_only_when_a_changed_mark_exists() {
    let project = Project::new();
    let mut value = marked_workflow();
    value["nodes"][1].as_object_mut().unwrap().remove("change");
    value["edges"][0]["change"] = json!("removed");
    let candidate = project.write("no-changed.json", &value);
    let html = project.0.join("no-changed.html");
    let output = run(&[
        flag("deliver"),
        flag("workflow"),
        candidate.as_os_str(),
        html.as_os_str(),
        flag("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{:?}", parsed(&output));
    let body = fs::read_to_string(&html).unwrap();
    assert_eq!(body.matches("data-change-note").count(), 0);
}
