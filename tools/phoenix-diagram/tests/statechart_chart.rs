use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType};
use phoenix_diagram::statechart::chart::{
    Arrow, Child, Clause, Fallback, Mark, Note, Region, When,
};
use phoenix_diagram::statechart::{analyze, Analysis};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/statechart/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn parsed(document: &Value) -> Result<Value, phoenix_diagram::diagnostic::Diagnostics> {
    parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Statechart,
        &CompileOptions::default(),
    )
    .map(|input| input.document)
}

fn run(document: &Value) -> Analysis {
    analyze(&parsed(document).unwrap()).unwrap()
}

/// The first diagnostic of a document that fails static checks or analysis.
fn failure(document: &Value) -> (String, String, Value) {
    let errors = match parsed(document) {
        Ok(document) => analyze(&document).err().expect("analysis fails"),
        Err(errors) => errors,
    };
    let error = &errors[0];
    (
        error.code.clone(),
        error.subject.path.clone(),
        error.evidence.clone(),
    )
}

fn path(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn arrow<'a>(region: &'a Region, from: &[&str], to: &[&str]) -> &'a Arrow {
    region
        .arrows
        .iter()
        .find(|arrow| arrow.from == path(from) && arrow.to == path(to))
        .unwrap_or_else(|| panic!("no arrow {from:?} -> {to:?} in {:?}", region.arrows))
}

fn state<'a>(region: &'a Region, at: &[&str]) -> &'a phoenix_diagram::statechart::chart::State {
    region
        .states
        .iter()
        .find(|state| state.path == path(at))
        .unwrap()
}

#[test]
fn a_flat_chart_merges_states_and_labels_and_draws_the_bug_loop() {
    let analysis = run(&fixture("checkout"));
    let chart = &analysis.chart;
    assert_eq!(chart.regions.len(), 1);
    let region = &chart.regions[0];
    assert_eq!(region.states.len(), 4);
    assert_eq!(region.arrows.len(), 4);
    // Two model steps from the two starts share one arrow.
    assert_eq!(arrow(region, &["购物车"], &["支付中"]).labels, ["去结账"]);
    assert_eq!(
        arrow(region, &["支付中"], &["支付失败"]).labels,
        ["支付失败", "超时"]
    );
    let loop_arrow = arrow(region, &["支付中"], &["支付中"]);
    assert_eq!(loop_arrow.labels, ["网关扣款", "网关又扣了一次"]);
    assert!(loop_arrow.bug_path);
    // The gateway outage stays in one chart state; the recovery is hidden.
    assert_eq!((chart.inside, chart.hidden), (3, 3));
    let paying = state(region, &["支付中"]);
    assert_eq!(paying.mark, Some(Mark::MayBreak));
    assert_eq!(paying.may_break, ["chargedOnce"]);
    assert!(paying.bug_path);
    assert_eq!(state(region, &["购物车"]).color.as_deref(), Some("cyan"));
    assert_eq!(paying.color.as_deref(), Some("emerald"));
    assert_eq!(state(region, &["购物车"]).mark, None);
    let story = &chart.stories[0];
    assert_eq!(story.rule, "chargedOnce");
    let steps: Vec<_> = story
        .steps
        .iter()
        .map(|(label, _)| label.as_str())
        .collect();
    assert_eq!(steps, ["去结账", "网关扣款", "网关又扣了一次"]);
}

#[test]
fn hiding_a_step_on_a_bug_path_is_an_error() {
    let mut document = fixture("checkout");
    document["view"]["steps"][2]["label"] = Value::Null;
    let (code, at, evidence) = failure(&document);
    assert_eq!(code, "statechart/hidden-bug-step");
    assert_eq!(at, "/view/steps/2/label");
    assert_eq!(evidence["rule"], "chargedOnce");
    assert_eq!(evidence["action"], "charge");
}

#[test]
fn parent_states_group_their_children_and_lift_a_shared_exit() {
    let analysis = run(&fixture("login-parents"));
    let region = &analysis.chart.regions[0];
    assert_eq!(region.parents.len(), 1);
    assert_eq!(region.parents[0].path, ["已登录"]);
    assert_eq!(region.parents[0].children.len(), 3);
    assert_eq!(region.top, [Child::State(0), Child::Parent(0)]);
    let exits: Vec<_> = region
        .arrows
        .iter()
        .filter(|arrow| arrow.labels == ["退出"])
        .collect();
    assert_eq!(exits.len(), 1);
    assert_eq!(exits[0].from, ["已登录"]);
    assert_eq!(exits[0].to, ["未登录"]);
    // Only two of the three children go home, so that arrow stays per child.
    let home: Vec<_> = region
        .arrows
        .iter()
        .filter(|arrow| arrow.labels == ["回首页"])
        .collect();
    assert_eq!(home.len(), 2);
    assert_eq!(region.arrows.len(), 8);
}

#[test]
fn regions_put_conditions_on_arrows_and_note_shared_steps() {
    let analysis = run(&fixture("login-regions"));
    let chart = &analysis.chart;
    assert_eq!(chart.fallback, None);
    assert_eq!(chart.region_names, ["会话", "错误次数"]);
    let session = &chart.regions[0];
    let sign_in = session
        .arrows
        .iter()
        .find(|arrow| arrow.labels == ["登录"])
        .unwrap();
    assert_eq!(
        sign_in.when,
        Some(When::All(vec![Clause {
            region: "错误次数".into(),
            negated: true,
            values: path(&["已锁定"]),
        }]))
    );
    assert_eq!(
        arrow(session, &["未登录"], &["已登录"]).labels,
        ["记住的设备"]
    );
    let fails = &chart.regions[1];
    assert_eq!(
        arrow(fails, &["2 次"], &["已锁定"]).when,
        Some(When::All(vec![Clause {
            region: "会话".into(),
            negated: false,
            values: path(&["未登录"]),
        }]))
    );
    assert_eq!(
        chart.notes,
        [
            Note::Together {
                step: "登录".into(),
                sets: vec![path(&["会话", "错误次数"])],
            },
            Note::OneAtATime {
                step: "管理员处理".into(),
                regions: path(&["会话", "错误次数"]),
            },
        ]
    );
    // The broken combination is signed in and locked; each of its states is
    // feasible on its own, so both are marked.
    assert_eq!(state(session, &["已登录"]).mark, Some(Mark::MayBreak));
    assert_eq!(state(fails, &["已锁定"]).mark, Some(Mark::MayBreak));
    assert_eq!(state(fails, &["0 次"]).mark, None);
    assert_eq!(chart.stories[0].steps.len(), 4);
}

#[test]
fn a_misleading_regions_chart_falls_back_to_parents_and_keeps_region_stories() {
    let analysis = run(&fixture("two-phase"));
    let chart = &analysis.chart;
    let combo = |items: [&str; 3]| -> Vec<(String, Vec<String>)> {
        ["协调者", "参与者A", "参与者B"]
            .into_iter()
            .zip(items)
            .map(|(region, leaf)| (region.into(), path(&[leaf])))
            .collect()
    };
    assert_eq!(
        chart.fallback,
        Some(Fallback::MoveMissing {
            from: combo(["进行中", "工作中", "工作中"]),
            label: "放弃".into(),
            to: combo(["进行中", "工作中", "已放弃"]),
        })
    );
    assert!(chart.notes.is_empty());
    assert_eq!(chart.regions.len(), 1);
    let region = &chart.regions[0];
    assert_eq!(region.name, None);
    assert_eq!(
        region.states[0].path,
        path(&["协调者 进行中", "参与者A 工作中", "参与者B 工作中"])
    );
    let broken = state(
        region,
        &["协调者 已提交", "参与者A 已提交", "参与者B 已放弃"],
    );
    assert_eq!(broken.mark, Some(Mark::Breaks));
    assert_eq!(broken.breaks, ["agree"]);
    let story = &chart.stories[0];
    assert_eq!(story.start.len(), 3);
    assert_eq!(story.start[1], (Some("参与者A".into()), path(&["工作中"])));
    assert_eq!(story.steps.len(), 5);
}

#[test]
fn mixed_groups_leave_a_state_uncoloured() {
    let analysis = run(&fixture("mixed-groups"));
    let region = &analysis.chart.regions[0];
    let working = state(region, &["处理中"]);
    assert_eq!(working.color, None);
    assert_eq!(working.mark, Some(Mark::MayBreak));
    assert_eq!(working.may_break, ["publishedUntouched"]);
    assert_eq!(state(region, &["草稿"]).color.as_deref(), Some("violet"));
}

#[test]
fn view_mistakes_are_reported_where_they_are_written() {
    let mut parent_leaf = fixture("login-parents");
    parent_leaf["view"]["state"][1]["label"] = json!("已登录");
    let (code, at, evidence) = failure(&parent_leaf);
    assert_eq!(
        (code.as_str(), at.as_str()),
        ("statechart/leaf-is-parent", "/view/state")
    );
    assert_eq!(evidence["path"], json!(["已登录"]));

    let mut unlabeled = fixture("login-parents");
    unlabeled["view"]["state"].as_array_mut().unwrap().pop();
    let (code, at, evidence) = failure(&unlabeled);
    assert_eq!(
        (code.as_str(), at.as_str()),
        ("statechart/unlabeled-state", "/view/state")
    );
    assert_eq!(evidence["state"], "profile");
    assert_eq!(evidence["path"], json!(["login", "home", "profile"]));

    let mut uncoloured = fixture("mixed-groups");
    uncoloured["view"]["colors"]
        .as_object_mut()
        .unwrap()
        .remove("审核");
    let (code, at, _) = failure(&uncoloured);
    assert_eq!(
        (code.as_str(), at.as_str()),
        ("statechart/group-color", "/view/groups/1/group")
    );
}
