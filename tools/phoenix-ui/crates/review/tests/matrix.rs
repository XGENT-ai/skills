use phoenix_ui_review::{
    matrix::{freeze, FrozenMatrix, Identity, State, Unit, Viewport},
    storage::StoreError,
};

fn id(id: &str) -> Identity {
    Identity {
        id: id.into(),
        label: id.into(),
    }
}
fn viewport(id: &str, width: u32, height: u32, dpr: f64) -> Viewport {
    Viewport {
        id: id.into(),
        width,
        height,
        dpr,
        full_page: false,
    }
}
fn unit(id_: &str) -> Unit {
    Unit {
        identity: id(id_),
        states: vec![State {
            identity: id("empty"),
            steps: vec![id("initial")],
        }],
        viewports: vec![viewport("mobile", 390, 844, 1.0)],
        themes: vec![id("light")],
    }
}
fn invalid(result: Result<FrozenMatrix, StoreError>) {
    assert_eq!(result.unwrap_err().code, 2);
}

#[test]
fn cartesian_matrix_keeps_each_states_own_interaction_steps() {
    let mut u = unit("U1");
    u.states.push(State {
        identity: id("flow"),
        steps: vec![id("01"), id("03")],
    });
    u.viewports.push(viewport("desktop", 1440, 900, 2.0));
    u.themes.push(id("dark"));
    let m = freeze(vec![u], None).unwrap();
    assert_eq!(m.cells.len(), 12);
    assert_eq!(
        m.cells.iter().filter(|c| c.key.state_id == "empty").count(),
        4
    );
    assert_eq!(
        m.cells.iter().filter(|c| c.key.state_id == "flow").count(),
        8
    );
    assert!(m.full);
    m.validate().unwrap();
}

#[test]
fn explicit_scope_never_claims_a_full_round_even_when_it_lists_all_units() {
    let units = vec![unit("U1"), unit("U2")];
    let all = freeze(units.clone(), None).unwrap();
    assert!(all.full);
    assert_eq!(all.selected_units(), ["U1", "U2"]);
    for scope in [vec!["U1".into()], vec!["U2".into(), "U1".into()]] {
        let m = freeze(units.clone(), Some(scope.clone())).unwrap();
        assert!(!m.full);
        assert_eq!(
            m.selected_units(),
            scope.iter().map(String::as_str).collect::<Vec<_>>()
        );
        assert_eq!(
            m.cells.len(),
            2,
            "capture matrix retains excluded units for regression"
        );
    }
}

#[test]
fn scope_must_be_nonempty_unique_and_known() {
    for scope in [vec![], vec!["U9".into()], vec!["U1".into(), "U1".into()]] {
        invalid(freeze(vec![unit("U1")], Some(scope)));
    }
}

#[test]
fn original_filenames_and_interaction_frame_names_are_preserved() {
    let mut u = unit("U1");
    u.states.push(State {
        identity: id("flow"),
        steps: vec![id("03")],
    });
    let m = freeze(vec![u], None).unwrap();
    assert_eq!(m.cells[0].filename, "U1-empty-390-light.png");
    assert_eq!(m.cells[1].filename, "U1-flow-390-light-03.png");
    assert_eq!(
        m.screenshot_name(&m.cells[0].key, 1).unwrap(),
        m.cells[0].filename
    );
    assert_eq!(
        m.screenshot_name(&m.cells[0].key, 2).unwrap(),
        "U1-empty-390-light-a2.png"
    );
    assert_eq!(
        m.screenshot_name(&m.cells[1].key, 3).unwrap(),
        "U1-flow-390-light-03-a3.png"
    );
    assert_eq!(m.screenshot_name(&m.cells[0].key, 0).unwrap_err().code, 2);
}

#[test]
fn same_width_viewports_bind_height_and_dpr_and_preallocate_distinct_names() {
    let mut u = unit("U1");
    u.viewports.extend([
        viewport("short", 390, 700, 1.0),
        viewport("retina", 390, 844, 2.0),
    ]);
    let m = freeze(vec![u], None).unwrap();
    let names = m
        .cells
        .iter()
        .map(|c| c.filename.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "U1-empty-390-h844-dpr1-light.png",
            "U1-empty-390-h700-dpr1-light.png",
            "U1-empty-390-h844-dpr2-light.png"
        ]
    );
    assert_eq!(m.cells[2].viewport.dpr, 2.0);
    m.validate().unwrap();
}

#[test]
fn a_viewport_id_has_one_binding_across_units() {
    let a = unit("U1");
    let mut b = unit("U2");
    b.viewports[0].height = 900;
    invalid(freeze(vec![a, b], None));
}

#[test]
fn duplicate_axes_and_windows_case_collisions_are_rejected() {
    invalid(freeze(vec![unit("U1"), unit("u1")], None));
    let u = unit("U1");
    for kind in 0..4 {
        let mut changed = u.clone();
        match kind {
            0 => changed.states.push(changed.states[0].clone()),
            1 => changed.viewports.push(changed.viewports[0].clone()),
            2 => changed.themes.push(id("LIGHT")),
            _ => changed.states[0].steps.push(id("initial")),
        }
        invalid(freeze(vec![changed], None));
    }
}

#[test]
fn empty_axes_cannot_erase_required_cells() {
    invalid(freeze(vec![], None));
    for kind in 0..4 {
        let mut u = unit("U1");
        match kind {
            0 => u.states.clear(),
            1 => u.viewports.clear(),
            2 => u.themes.clear(),
            _ => u.states[0].steps.clear(),
        }
        invalid(freeze(vec![u], None));
    }
}

#[test]
fn malformed_dimensions_and_unbounded_pixel_sizes_are_rejected() {
    for dpr in [0.0, -1.0, f64::INFINITY, f64::NAN, f64::MAX] {
        let mut u = unit("U1");
        u.viewports[0].dpr = dpr;
        invalid(freeze(vec![u], None));
    }
    for axis in [0, 1] {
        let mut u = unit("U1");
        if axis == 0 {
            u.viewports[0].width = 0;
        } else {
            u.viewports[0].height = 0;
        }
        invalid(freeze(vec![u], None));
    }
    let mut u = unit("U1");
    u.viewports[0].dpr = 1.25;
    assert_eq!(freeze(vec![u], None).unwrap().cells[0].viewport.dpr, 1.25);
}

#[test]
fn path_like_identifiers_cannot_escape_or_inject_names() {
    for bad in ["", "../U1", "U1/a", "U1\\a", "U1:", " U1", "U1."] {
        let mut u = unit("U1");
        u.identity.id = bad.into();
        invalid(freeze(vec![u], None));
    }
    let mut u = unit("U1");
    u.identity.label = " ".into();
    invalid(freeze(vec![u], None));
}

#[test]
fn display_labels_do_not_change_cell_identity_or_filenames() {
    let u = unit("U1");
    let before = freeze(vec![u.clone()], None).unwrap();
    let mut u = u;
    u.identity.label = "订单列表".into();
    u.states[0].identity.label = "空数据".into();
    u.states[0].steps[0].label = "等待完成".into();
    u.themes[0].label = "浅色".into();
    assert_eq!(before.cells, freeze(vec![u], None).unwrap().cells);
}

#[test]
fn persisted_matrix_recomputes_full_coverage_names_and_viewport_bindings() {
    let m = freeze(vec![unit("U1")], Some(vec!["U1".into()])).unwrap();
    let bytes = serde_json::to_vec(&m).unwrap();
    serde_json::from_slice::<FrozenMatrix>(&bytes)
        .unwrap()
        .validate()
        .unwrap();
    for kind in 0..5 {
        let mut changed = m.clone();
        match kind {
            0 => changed.full = true,
            1 => changed.cells.clear(),
            2 => changed.cells[0].viewport.dpr = 2.0,
            3 => changed.cells[0].filename = "other.png".into(),
            _ => changed.schema_version = 2,
        }
        assert_eq!(changed.validate().unwrap_err().code, 2);
    }
}

#[test]
fn ambiguous_hyphenated_ids_cannot_overwrite_other_cells() {
    let mut a = unit("U1-a");
    a.states[0].identity.id = "empty".into();
    let mut b = unit("U1");
    b.states[0].identity.id = "a-empty".into();
    let m = freeze(vec![a, b], None).unwrap();
    assert_ne!(m.cells[0].filename, m.cells[1].filename);
    m.validate().unwrap();
}

#[test]
fn unknown_json_fields_cannot_supply_full_or_a_different_key() {
    let mut v = serde_json::to_value(unit("U1")).unwrap();
    v["full"] = true.into();
    assert!(serde_json::from_value::<Unit>(v).is_err());
    let m = freeze(vec![unit("U1")], None).unwrap();
    let mut v = serde_json::to_value(&m.cells[0].key).unwrap();
    v["filename"] = "old.png".into();
    assert!(serde_json::from_value::<phoenix_ui_review::capture::CellKey>(v).is_err());
}

#[test]
fn disambiguated_long_names_remain_writable_as_real_files() {
    let stem = "U".repeat(62);
    let state = "s".repeat(32);
    let mut a = unit(&format!("{stem}-a"));
    a.states[0].identity.id = state.clone();
    a.states[0].steps[0].id = "k".repeat(24);
    a.themes[0].id = "t".repeat(64);
    let mut b = a.clone();
    b.identity.id = stem;
    b.states[0].identity.id = format!("a-{state}");
    let m = freeze(vec![a, b], None).unwrap();
    let dir = std::env::temp_dir().join(format!("phoenix-matrix-names-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let results = m
        .cells
        .iter()
        .map(|cell| {
            std::fs::write(
                dir.join(m.screenshot_name(&cell.key, u32::MAX).unwrap()),
                b"capture input",
            )
        })
        .collect::<Vec<_>>();
    std::fs::remove_dir_all(&dir).unwrap();
    for result in results {
        result.unwrap();
    }
}

#[test]
fn expanding_huge_axes_is_rejected_before_allocating_the_cartesian_product() {
    let mut u = unit("U1");
    u.states[0].steps = (0..1001).map(|n| id(&format!("s{n}"))).collect();
    u.viewports = (0..1001)
        .map(|n| viewport(&format!("v{n}"), 390, 844, 1.0))
        .collect();
    let error = freeze(vec![u], None).unwrap_err();
    assert_eq!(error.code, 2);
    assert!(error.message.contains("resource limit"));
}
