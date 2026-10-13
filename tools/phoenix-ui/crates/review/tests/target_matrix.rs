use phoenix_ui_review::target_matrix::parse;

fn target(rows: &str) -> String {
    format!("# Target\n\n## 单元\n\n| 单元 | 必需状态 | 视口 | 主题 | 来源 |\n| --- | --- | --- | --- | --- |\n{rows}\n\n## 规范要点\n\nKeep the accepted project design.\n")
}

#[test]
fn real_m1_target_freezes_the_complete_twenty_cell_matrix() {
    let text = include_str!("../../../../../test/phoenix-ui/fixtures/web/fixture.ui-target.md");
    let matrix = parse(text, None).unwrap();
    assert!(matrix.full);
    assert_eq!(matrix.cells.len(), 20);
    assert_eq!(matrix.units[0].identity.id, "U1");
    assert_eq!(matrix.units[0].identity.label, "任务列表");
    assert_eq!(matrix.units[0].viewports[1].id, "mobile");
    assert!(matrix
        .cells
        .iter()
        .any(|cell| cell.filename == "U1-empty-390-dark.png"));
    matrix.validate().unwrap();
}

#[test]
fn explicit_stable_ids_keep_display_labels_separate() {
    let matrix = parse(&target("| U1 订单 | default 默认、empty 空状态 | mobile 390×844@2 | light 浅色、dark 深色 | DESIGN.md |"), None).unwrap();
    assert_eq!(matrix.units[0].states[1].identity.id, "empty");
    assert_eq!(matrix.units[0].states[1].identity.label, "空状态");
    assert_eq!(matrix.units[0].viewports[0].dpr, 2.0);
    assert_eq!(matrix.cells.len(), 4);
}

#[test]
fn combined_contract_column_preserves_the_cartesian_product() {
    let text = "## 单元\n\n| 单元 | 状态 | 视口 × 主题 | 目标 | 比较口径 |\n| --- | --- | --- | --- | --- |\n| U1 Orders | default、empty | 1440×900、390×844 × light、dark | DESIGN.md | Norm |\n\n## 规范要点\nKeep the project norms.\n";
    let matrix = parse(text, None).unwrap();
    assert_eq!(matrix.cells.len(), 8);
    assert_eq!(matrix.units[0].viewports[1].width, 390);
}

#[test]
fn omitted_dpr_is_explicitly_frozen_as_one_and_same_width_sizes_do_not_overwrite() {
    let matrix = parse(
        &target("| U1 | default | compact 390×700、tall 390×900@2 | light | DESIGN.md |"),
        None,
    )
    .unwrap();
    assert_eq!(matrix.units[0].viewports[0].dpr, 1.0);
    assert!(matrix.cells[0].filename.contains("390-h700-dpr1"));
    assert!(matrix.cells[1].filename.contains("390-h900-dpr2"));
}

#[test]
fn scope_never_removes_regression_cells_or_claims_full() {
    let text = target("| U1 | default | 390×844 | light | DESIGN.md |\n| U2 | error | 390×844 | light | DESIGN.md |");
    let matrix = parse(&text, Some(vec!["U2".into()])).unwrap();
    assert!(!matrix.full);
    assert_eq!(matrix.cells.len(), 2);
    assert_eq!(matrix.selected_units(), ["U2"]);
}

#[test]
fn fenced_examples_and_quoted_tables_cannot_define_product_units() {
    let real = target("| U1 | default | 390×844 | light | DESIGN.md |");
    let examples = format!("```markdown\n{real}\n```\n\n> ## 单元\n>\n> | 单元 | 状态 | 视口 | 主题 |\n> | --- | --- | --- | --- |\n> | U9 | wrong | 1×1 | dark |\n\n{real}");
    let matrix = parse(&examples, None).unwrap();
    assert_eq!(matrix.units.len(), 1);
    assert_eq!(matrix.units[0].identity.id, "U1");
}

#[test]
fn dimensions_without_a_height_or_nonpositive_dpr_are_not_guessed() {
    for viewport in [
        "390",
        "390×0",
        "390×844@0",
        "390×844@NaN",
        "390×844@2 trailing",
    ] {
        assert_eq!(
            parse(
                &target(&format!(
                    "| U1 | default | {viewport} | light | DESIGN.md |"
                )),
                None
            )
            .unwrap_err()
            .code,
            2,
            "{viewport}"
        );
    }
}

#[test]
fn duplicate_axes_and_ambiguous_unit_sections_fail_before_freezing() {
    for rows in [
        "| U1 | default、default | 390×844 | light | DESIGN.md |",
        "| U1 | default | 390×844 | light、light | DESIGN.md |",
        "| U1 | default | 390×844 | light | DESIGN.md |\n| u1 | error | 390×844 | dark | DESIGN.md |",
    ] {
        assert_eq!(parse(&target(rows), None).unwrap_err().code, 2);
    }
    let duplicate = format!(
        "{}\n## 单元\n",
        target("| U1 | default | 390×844 | light | DESIGN.md |")
    );
    assert_eq!(parse(&duplicate, None).unwrap_err().code, 2);
}

#[test]
fn nonascii_labels_need_explicit_stable_ids_in_managed_targets() {
    let text = target("| U1 | 默认、空 | 390×844 | light | DESIGN.md |");
    assert_eq!(parse(&text, None).unwrap_err().code, 2);
}

#[test]
fn machine_matrix_supports_state_specific_interaction_steps_and_full_page() {
    let text = r#"## 单元

```phoenix-ui-matrix
{"schemaVersion":1,"units":[{"identity":{"id":"U1","label":"Checkout"},"states":[{"identity":{"id":"flow","label":"Submit"},"steps":[{"id":"01","label":"Open"},{"id":"03","label":"Submit"}]}],"viewports":[{"id":"desktop","width":1440,"height":900,"dpr":2,"fullPage":true}],"themes":[{"id":"light","label":"Light"}]}]}
```

## 规范要点
Keep the project norms.
"#;
    let matrix = parse(text, None).unwrap();
    assert_eq!(matrix.cells.len(), 2);
    assert_eq!(matrix.cells[1].key.step_id, "03");
    assert_eq!(matrix.cells[1].filename, "U1-flow-1440-light-03.png");
    assert!(matrix.cells[1].viewport.full_page);
    assert_eq!(
        parse(
            &text.replace("\"schemaVersion\":1", "\"schemaVersion\":2"),
            None
        )
        .unwrap_err()
        .code,
        2
    );
    assert_eq!(
        parse(
            &text.replace("\"schemaVersion\":1", "\"full\":true,\"schemaVersion\":1"),
            None
        )
        .unwrap_err()
        .code,
        2
    );
}

#[test]
fn competing_table_and_machine_definitions_are_an_explicit_conflict() {
    let machine = "```phoenix-ui-matrix\n{\"schemaVersion\":1,\"units\":[]}\n```";
    let text = target("| U1 | default | 390×844 | light | DESIGN.md |")
        .replace("## 规范要点", &format!("{machine}\n\n## 规范要点"));
    assert_eq!(parse(&text, None).unwrap_err().code, 2);
}

#[test]
fn a_missing_or_unrecognizable_unit_matrix_is_an_input_error() {
    for text in [
        "# Target\n## 规范要点\nKeep norms.",
        "## 单元\nNo table.\n## 规范要点\nKeep norms.",
        "## 单元\n| Unit | State |\n| --- | --- |\n| U1 | default |\n",
    ] {
        assert_eq!(parse(text, None).unwrap_err().code, 2);
    }
}
