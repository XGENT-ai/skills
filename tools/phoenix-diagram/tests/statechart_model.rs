use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType};
use phoenix_diagram::statechart::{analyze, Analysis, Verdict};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/statechart/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn parsed(document: &Value) -> Result<Value, Vec<String>> {
    parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Statechart,
        &CompileOptions::default(),
    )
    .map(|input| input.document)
    .map_err(|errors| errors.into_iter().map(|error| error.code).collect())
}

fn run(document: &Value) -> Analysis {
    analyze(&parsed(document).unwrap()).unwrap()
}

fn rejects(document: &Value, code: &str, path: &str) {
    let errors = parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Statechart,
        &CompileOptions::default(),
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == code && error.subject.path == path),
        "expected {code} at {path}, got {errors:?}"
    );
}

/// Everything an author can observe about one exploration, as one string.
fn fingerprint(analysis: &Analysis) -> String {
    let exploration = &analysis.exploration;
    let model = &analysis.model;
    let states: Vec<_> = exploration
        .states
        .iter()
        .map(|state| model.state_name(state))
        .collect();
    let steps: Vec<_> = exploration
        .steps
        .iter()
        .map(|step| {
            format!(
                "{}-{}->{}",
                step.from, model.actions[step.action].id, step.to
            )
        })
        .collect();
    let outcomes: Vec<_> = analysis
        .outcomes
        .iter()
        .map(|outcome| json!([outcome.rule, outcome.verdict, analysis.path_names(outcome)]))
        .collect();
    json!({"states":states,"steps":steps,"outcomes":outcomes,"complete":exploration.complete})
        .to_string()
}

#[test]
fn die_hard_reaches_four_litres_in_six_steps() {
    let analysis = run(&fixture("diehard"));
    assert!(analysis.complete());
    let outcome = analysis.outcome("neverFour").unwrap();
    assert_eq!(outcome.verdict, Verdict::Breaks);
    let path = analysis.path_names(outcome);
    assert_eq!(path.len(), 7, "{path:?}");
    assert_eq!(path[0], "big=0 small=0");
    assert!(path[6].starts_with("big=4 "), "{path:?}");
    assert_eq!(
        analysis.outcome("deadlock").unwrap().verdict,
        Verdict::Holds
    );
    assert_eq!(analysis.exploration.states.len(), 16);
}

#[test]
fn a_stop_is_a_deadlock_until_ends_declares_it_terminal() {
    let mut document = fixture("deadlock");
    let analysis = run(&document);
    let deadlock = analysis.outcome("deadlock").unwrap();
    assert_eq!(deadlock.verdict, Verdict::Breaks);
    assert_eq!(analysis.path_names(deadlock), ["idle", "running", "done"]);

    document["model"]["deadlock"] = json!({"ends":{"eq":[{"var":"phase"},"done"]}});
    let analysis = run(&document);
    assert_eq!(
        analysis.outcome("deadlock").unwrap().verdict,
        Verdict::Holds
    );

    document["model"]["deadlock"] = json!({"check":false});
    assert!(run(&document).outcome("deadlock").is_none());
}

#[test]
fn the_state_limit_leaves_unseen_violations_not_checked() {
    let analysis = run(&fixture("limit"));
    assert!(!analysis.complete());
    assert_eq!(analysis.exploration.states.len(), 100_000);
    assert_eq!(
        analysis.outcome("xBelowThree").unwrap().verdict,
        Verdict::Breaks
    );
    assert_eq!(
        analysis
            .path_names(analysis.outcome("xBelowThree").unwrap())
            .len(),
        4
    );
    assert_eq!(
        analysis.outcome("boundedSum").unwrap().verdict,
        Verdict::NotChecked
    );
    assert_eq!(
        analysis.outcome("deadlock").unwrap().verdict,
        Verdict::NotChecked
    );
    assert_eq!(analysis.warnings.len(), 1);
    assert_eq!(analysis.warnings[0].code, "statechart/state-limit");
    assert_eq!(analysis.warnings[0].evidence["limit"], 100_000);
}

#[test]
fn results_do_not_depend_on_action_declaration_order() {
    let document = fixture("diehard");
    let mut reversed = document.clone();
    reversed["model"]["actions"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(fingerprint(&run(&document)), fingerprint(&run(&reversed)));
}

#[test]
fn initial_alternatives_and_pick_parameters_multiply_the_states() {
    let document = json!({
        "schema_version": 1, "diagram_type": "statechart",
        "meta": {"title": "Pick", "output": "pick.html"},
        "model": {
            "variables": [
                {"id": "light", "values": ["red", "green"]},
                {"id": "online", "values": [true, false]},
                {"id": "level", "range": [0, 3]}
            ],
            "init": {"light": ["red", "green"], "online": [true, false], "level": 0},
            "actions": [{"id": "setLevel", "pick": {"to": [1, 2, 3]}, "set": {"level": {"param": "to"}}}]
        }
    });
    let analysis = run(&document);
    assert_eq!(analysis.exploration.initial, 4);
    assert_eq!(analysis.exploration.states.len(), 16);
    assert_eq!(analysis.exploration.steps.len(), 16 * 3);
    assert_eq!(
        analysis.model.state_name(&analysis.exploration.states[0]),
        "light=\"green\" online=false level=0"
    );
}

#[test]
fn a_set_outside_its_domain_fails_with_the_path_that_reached_it() {
    let mut document = fixture("deadlock");
    document["model"]["variables"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"tries","range":[0,1]}));
    document["model"]["init"]["tries"] = json!(0);
    document["model"]["actions"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "retry", "when": {"eq": [{"var": "phase"}, "running"]},
            "set": {"tries": {"add": [{"var": "tries"}, 1]}}
        }));
    let errors = analyze(&parsed(&document).unwrap()).err().unwrap();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "statechart/value-out-of-domain");
    assert_eq!(error.subject.path, "/model/actions/2/set/tries");
    assert_eq!(error.evidence["action"], "retry");
    assert_eq!(error.evidence["variable"], "tries");
    assert_eq!(error.evidence["value"], "2");
    assert_eq!(
        error.evidence["path"],
        json!([
            "phase=\"idle\" tries=0",
            "phase=\"running\" tries=0",
            "phase=\"running\" tries=1"
        ])
    );
}

#[test]
fn static_checks_name_each_mistake_with_its_pointer() {
    let base = fixture("deadlock");
    let with = |pointer: &str, value: Value| {
        let mut document = base.clone();
        *document.pointer_mut(pointer).unwrap() = value;
        document
    };
    rejects(
        &with(
            "/model/actions/0/when",
            json!({"eq":[{"var":"stage"},"idle"]}),
        ),
        "statechart/unknown-variable",
        "/model/actions/0/when/eq/0/var",
    );
    rejects(
        &with(
            "/model/actions/0/when",
            json!({"eq":[{"param":"to"},"idle"]}),
        ),
        "statechart/unknown-param",
        "/model/actions/0/when/eq/0/param",
    );
    rejects(
        &with(
            "/model/actions/0/when",
            json!({"eq":[{"next":"phase"},"idle"]}),
        ),
        "statechart/next-outside-step",
        "/model/actions/0/when/eq/0/next",
    );
    rejects(
        &with("/model/actions/0/when", json!({"add":[1,2]})),
        "statechart/expression-type",
        "/model/actions/0/when",
    );
    rejects(
        &with(
            "/model/actions/0/when",
            json!({"eq":[{"var":"phase"},"idel"]}),
        ),
        "statechart/value-not-in-domain",
        "/model/actions/0/when/eq/1",
    );
    rejects(
        &with("/model/actions/0/set/phase", json!("paused")),
        "statechart/value-not-in-domain",
        "/model/actions/0/set/phase",
    );
    rejects(
        &with("/model/init", json!({})),
        "statechart/init-incomplete",
        "/model/init",
    );
    rejects(
        &with("/model/actions/1/id", json!("start")),
        "input/duplicate-id",
        "/model/actions/1/id",
    );
    let mut reserved = base.clone();
    reserved["model"]["rules"] = json!([{"id":"deadlock","text":"不会卡住","holds":true}]);
    rejects(
        &reserved,
        "statechart/reserved-rule-id",
        "/model/rules/0/id",
    );
    let mut placeholder = base.clone();
    placeholder["view"] = json!({"state":[{"label":"阶段 {stage}"}]});
    rejects(
        &placeholder,
        "statechart/unknown-placeholder",
        "/view/state/0/label",
    );
}

#[test]
fn integers_a_model_cannot_hold_are_reported_where_written() {
    let code = "statechart/integer-out-of-range";
    let big = json!(10_000_000_000_000_000_000u64);
    let base = fixture("deadlock");
    let with_variable = |variable: Value| {
        let mut document = base.clone();
        document["model"]["variables"]
            .as_array_mut()
            .unwrap()
            .push(variable);
        document["model"]["init"]["n"] = json!(0);
        document
    };
    let with = |pointer: &str, value: Value| {
        let mut document = base.clone();
        *document.pointer_mut(pointer).unwrap() = value;
        document
    };

    let mut ruled = with_variable(json!({"id":"n","values":[0, big]}));
    ruled["model"]["rules"] =
        json!([{"id":"stays-small","text":"n 不会变大","holds":{"ne":[{"var":"n"}, big]}}]);
    rejects(&ruled, code, "/model/variables/1/values/1");
    rejects(&ruled, code, "/model/rules/0/holds/ne/1");
    rejects(
        &with_variable(json!({"id":"n","range":[0, big]})),
        code,
        "/model/variables/1/range/1",
    );
    let mut picked = base.clone();
    picked["model"]["actions"][0]["pick"] = json!({"to":[1, big]});
    rejects(&picked, code, "/model/actions/0/pick/to/1");
    rejects(
        &with(
            "/model/actions/0/when",
            json!({"in":[{"var":"phase"}, [big]]}),
        ),
        code,
        "/model/actions/0/when/in/1/0",
    );
    rejects(
        &with("/model/actions/0/set/phase", json!(1e20)),
        code,
        "/model/actions/0/set/phase",
    );
    rejects(&with("/model/init/phase", big), code, "/model/init/phase");
}

#[test]
fn the_minimal_example_passes_schema_and_static_checks() {
    let path = format!(
        "{}/../../skills/phoenix-diagram/examples/minimal.statechart.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let document: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let analysis = run(&document);
    assert!(analysis
        .outcomes
        .iter()
        .any(|outcome| outcome.verdict == Verdict::Breaks && outcome.rule != "deadlock"));
    assert_eq!(
        analysis.outcome("deadlock").unwrap().verdict,
        Verdict::Holds
    );
}
