use phoenix_diagram::i18n::{resolve, Locale};
use phoenix_diagram::input::{parse_input, CompileOptions, DiagramType};
use phoenix_diagram::statechart::chart::{Arrow, Clause, When};
use phoenix_diagram::statechart::report::{arrow_label, cards, receipt, when_text};
use phoenix_diagram::statechart::{analyze, Analysis};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/tests/fixtures/statechart/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn run(document: &Value) -> (Analysis, Locale) {
    let parsed = parse_input(
        &serde_json::to_vec(document).unwrap(),
        DiagramType::Statechart,
        &CompileOptions::default(),
    )
    .unwrap()
    .document;
    (analyze(&parsed).unwrap(), resolve(&parsed["meta"]))
}

fn arrow<'a>(analysis: &'a Analysis, region: usize, from: &str, to: &str) -> &'a Arrow {
    analysis.chart.regions[region]
        .arrows
        .iter()
        .find(|arrow| arrow.from == [from] && arrow.to == [to])
        .unwrap()
}

fn items(card: &Value) -> Vec<&str> {
    card["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect()
}

#[test]
fn region_arrows_name_their_condition_and_the_bug_card_names_each_region() {
    let (analysis, locale) = run(&fixture("login-regions"));
    let session = &analysis.chart.regions[0];
    let sign_in = session
        .arrows
        .iter()
        .find(|arrow| arrow.labels == ["登录"])
        .unwrap();
    assert_eq!(arrow_label(sign_in, 2, &locale), "登录 [除非 已锁定]");
    assert!(
        arrow_label(arrow(&analysis, 1, "2 次", "已锁定"), 2, &locale).ends_with(" [当 未登录 时]")
    );

    let cards = cards(&analysis, &locale);
    let summary = items(&cards[0]);
    assert_eq!(
        &summary[summary.len() - 2..],
        [
            "「登录」会同时改变 会话、错误次数。",
            "「管理员处理」每次只改变 会话、错误次数 中的一个。",
        ]
    );
    assert_eq!(cards[1]["dot"], "rose");
    assert_eq!(
        cards[1]["title"],
        "违反：锁定后不能登录（signedInWhileLocked）"
    );
    let path = items(&cards[1]);
    assert_eq!(path[0], "最短路径 4 步");
    assert_eq!(path[1], "0. 会话 未登录，错误次数 0 次");
    assert_eq!(path.len(), 6);
}

#[test]
fn a_flat_chart_merges_labels_and_reports_the_bug_path_in_its_receipt() {
    let (analysis, locale) = run(&fixture("checkout"));
    assert_eq!(
        arrow_label(arrow(&analysis, 0, "支付中", "支付失败"), 1, &locale),
        "支付失败 或 超时"
    );
    let model = receipt(&analysis, &locale);
    let rule = &model["rules"][0];
    assert_eq!(
        (&rule["id"], &rule["outcome"]),
        (&json!("chargedOnce"), &json!("breaks"))
    );
    let path = rule["path"].as_array().unwrap();
    assert_eq!(path.len(), 4);
    assert_eq!(path[0].as_object().unwrap().len(), 1);
    assert_eq!(path[1]["step"], "去结账");
    assert_eq!(path[1]["state"], "支付中");
    assert_eq!(model["chart"]["regionsFallback"], Value::Null);
    assert_eq!(
        (&model["chart"]["inside"], &model["chart"]["hidden"]),
        (&json!(3), &json!(3))
    );
    assert_eq!(model["stateLimit"], 100_000);

    let summary = cards(&analysis, &locale);
    assert!(items(&summary[0]).contains(&"3 步停留在同一图状态内，图中不画。"));
    assert!(items(&summary[0]).contains(&"view 隐藏了 3 步。"));
}

#[test]
fn english_documents_get_english_report_words() {
    let mut document = fixture("checkout");
    document["meta"]["locale"] = json!("en");
    let (analysis, locale) = run(&document);
    assert_eq!(
        arrow_label(arrow(&analysis, 0, "支付中", "支付失败"), 1, &locale),
        "支付失败 or 超时"
    );
    let cards = cards(&analysis, &locale);
    assert_eq!(cards[0]["title"], "Summary");
    assert_eq!(
        cards[1]["title"],
        "Breaks: 一笔订单最多扣一次款 (chargedOnce)"
    );
}

#[test]
fn a_misleading_regions_chart_says_why_it_draws_parents() {
    let (analysis, locale) = run(&fixture("two-phase"));
    let reason = "模型经「放弃」从 协调者 进行中，参与者A 工作中，参与者B 工作中 到 协调者 进行中，参与者A 工作中，参与者B 已放弃，区域图画不出来。";
    let summary = cards(&analysis, &locale);
    assert!(items(&summary[0]).contains(&format!("{reason}因此图中用父状态代替区域。").as_str()));
    let model = receipt(&analysis, &locale);
    assert_eq!(model["chart"]["regionsFallback"], reason);
    assert_eq!(
        model["chart"]["regions"],
        json!(["协调者", "参与者A", "参与者B"])
    );
}

#[test]
fn a_stopped_check_says_so_and_lists_the_rules_it_did_not_check() {
    let (analysis, locale) = run(&fixture("limit"));
    let cards = cards(&analysis, &locale);
    assert!(items(&cards[0]).contains(&"到达 100000 个状态的上限后停止，未见违反的规则未检查。"));
    let last = cards.last().unwrap();
    assert_eq!(
        (&last["dot"], &last["title"]),
        (&json!("amber"), &json!("未检查的规则"))
    );
    assert!(items(last).contains(&"三个计数之和不超过 297"));
    assert_eq!(receipt(&analysis, &locale)["complete"], false);
}

#[test]
fn three_regions_name_every_region_in_a_condition() {
    let locale = resolve(&json!({"locale": "zh-CN"}));
    let clause = |region: &str, negated, values: &[&str]| Clause {
        region: region.into(),
        negated,
        values: values.iter().map(|value| (*value).to_owned()).collect(),
    };
    let all = When::All(vec![
        clause("参与者A", false, &["工作中"]),
        clause("参与者B", true, &["已提交", "已放弃"]),
    ]);
    assert_eq!(
        when_text(&all, 3, &locale),
        "当 参与者A 是 工作中 且 参与者B 不是 已提交、已放弃 中任何一个 时"
    );
    let any = When::Any(vec![
        clause("参与者A", false, &["已放弃", "工作中"]),
        clause("协调者", false, &["已中止"]),
    ]);
    assert_eq!(
        when_text(&any, 3, &locale),
        "当 参与者A 是 已放弃、工作中 之一 或 协调者 是 已中止 时"
    );
}
