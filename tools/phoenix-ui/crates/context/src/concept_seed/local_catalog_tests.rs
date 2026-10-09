use super::*;
use serde_json::json;
use std::path::PathBuf;

struct Project(PathBuf);

impl Project {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "phoenix-catalog-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("PRODUCT.md"),
            "# Product\n\nAn operations workspace.\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("DESIGN.md"),
            "# Design\n\nKeep standard navigation.\n",
        )
        .unwrap();
        Self(dir)
    }

    fn cwd(&self) -> &str {
        self.0.to_str().unwrap()
    }

    fn catalog(&self, data: &[Value; 4]) -> Env {
        let dir = self.0.join("catalog");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, value) in [
            "concept-ingredients.json",
            "concept-reviews.json",
            "composition-ingredients.json",
            "composition-reviews.json",
        ]
        .into_iter()
        .zip(data)
        {
            std::fs::write(dir.join(name), serde_json::to_vec(value).unwrap()).unwrap();
        }
        Env::from([(
            "PHOENIX_UI_CATALOG_DIR".into(),
            dir.to_str().unwrap().into(),
        )])
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn own_data() -> [Value; 4] {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../skills/phoenix-ui/src/scripts/data/catalog");
    [
        "concept-ingredients.json",
        "concept-reviews.json",
        "composition-ingredients.json",
        "composition-reviews.json",
    ]
    .map(|name| serde_json::from_slice(&std::fs::read(source.join(name)).unwrap()).unwrap())
}

fn args(scope: &str, mode: &str) -> SeedArgs {
    SeedArgs {
        scope: Some(scope.into()),
        key: "stable-local-seed".into(),
        reroll: 0.0,
        register: None,
        mode: Some(Some(mode.into())),
        grain: None,
        platform: None,
        candidate_count: 7.0,
    }
}

#[test]
fn builtin_catalog_supports_both_scopes_and_all_four_modes_deterministically() {
    let project = Project::new();
    let catalog = builtin_catalog().unwrap();
    assert_eq!(catalog.concepts.len(), 24);
    assert_eq!(catalog.compositions.len(), 8);
    let env = Env::from([("PHOENIX_UI_COMPOSITIONS".into(), "1".into())]);
    for scope in ["direction", "surface"] {
        for mode in SEED_MODES {
            let mut input = args(scope, mode);
            let first = render_concept_seed(&env, project.cwd(), &input).unwrap();
            assert!(!first.contains("INSUFFICIENT"), "{scope}/{mode}: {first}");
            assert!(first.contains("phoenix-2026-01"));
            assert!(first.contains("text-only candidate"));
            assert!(!first.contains("impeccable.style"));
            for composition in &catalog.compositions {
                let id = composition["id"].as_str().unwrap();
                assert_eq!(
                    first.contains(id),
                    composition["surface"] == mode,
                    "{scope}/{mode}: {id}"
                );
            }
            assert_eq!(
                first,
                render_concept_seed(&env, project.cwd(), &input).unwrap()
            );
            input.reroll = 1.0;
            let reroll = render_concept_seed(&env, project.cwd(), &input).unwrap();
            assert_eq!(
                reroll,
                render_concept_seed(&env, project.cwd(), &input).unwrap()
            );
        }
    }
}

#[test]
fn a_small_catalog_returns_actual_count_and_never_borrows_another_mode() {
    let project = Project::new();
    let mut data = own_data();
    let id = data[1]["reviews"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, review)| review["allowedModes"] == json!(["operate"]))
        .unwrap()
        .0
        .clone();
    for family in data[0]["families"].as_array_mut().unwrap() {
        family["concepts"]
            .as_array_mut()
            .unwrap()
            .retain(|entry| entry["id"] == id);
    }
    data[1]["reviews"]
        .as_object_mut()
        .unwrap()
        .retain(|key, _| key == &id);
    let env = project.catalog(&data);
    let one = render_concept_seed(&env, project.cwd(), &args("direction", "operate")).unwrap();
    assert!(
        one.contains("status=insufficient; requested=6; available=1"),
        "{one}"
    );
    assert!(one.contains("catalog=phoenix-2026-01; seed=stable-local-seed"));
    let mut input = args("direction", "read");
    input.reroll = 1.0;
    input.register = Some(Some("bolder".into()));
    let zero = render_concept_seed(&env, project.cwd(), &input).unwrap();
    assert!(zero.contains("available=0"), "{zero}");
    assert!(zero.contains("No eligible challenger leads this round"));
    assert!(zero.contains("PRODUCT.md, DESIGN.md"));
    assert!(!zero.contains("ASSIGNED INDEX"));
}

#[test]
fn an_empty_valid_catalog_is_successful_and_has_no_invented_candidates() {
    let project = Project::new();
    let mut data = own_data();
    data[0]["families"] = json!([]);
    data[1]["reviews"] = json!({});
    data[2]["compositions"] = json!([]);
    data[3]["reviews"] = json!({});
    let env = project.catalog(&data);
    let (mut io, capture) = Io::captured("", project.0.clone(), env);
    assert_eq!(run(&["--mode".into(), "operate".into()], &mut io), 0);
    let text = String::from_utf8(capture.stdout.borrow().clone()).unwrap();
    assert!(text.contains("status=insufficient; requested=6; available=0"));
}

#[test]
fn invalid_arguments_keep_the_original_exit_code_before_catalog_loading() {
    let project = Project::new();
    let missing = project.0.join("missing");
    let env = Env::from([(
        "PHOENIX_UI_CATALOG_DIR".into(),
        missing.to_str().unwrap().into(),
    )]);
    for args in [
        vec!["--scope", "invalid"],
        vec!["--reroll", "-1"],
        vec!["--register", "invalid"],
        vec!["--register", "bolder"],
        vec![
            "--scope",
            "surface",
            "--reroll",
            "1",
            "--register",
            "bolder",
        ],
        vec!["--mode", "invalid"],
        vec!["--grain", "invalid"],
        vec!["--platform", "invalid"],
        vec!["--candidate-count", "4"],
    ] {
        let (mut io, capture) = Io::captured("", project.0.clone(), env.clone());
        let argv = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        assert_eq!(run(&argv, &mut io), 1, "{args:?}");
        assert!(capture.stdout.borrow().is_empty());
        let diagnostic = String::from_utf8(capture.stderr.borrow().clone()).unwrap();
        assert!(diagnostic.starts_with("concept-seed: --"), "{diagnostic}");
        assert!(!diagnostic.contains("catalog"), "{diagnostic}");
    }
}

#[test]
fn explicit_missing_or_invalid_catalogs_are_input_errors() {
    let project = Project::new();
    let missing = project.0.join("missing");
    let env = Env::from([(
        "PHOENIX_UI_CATALOG_DIR".into(),
        missing.to_str().unwrap().into(),
    )]);
    let (mut io, capture) = Io::captured("", project.0.clone(), env);
    assert_eq!(run(&[], &mut io), 2);
    assert!(String::from_utf8(capture.stderr.borrow().clone())
        .unwrap()
        .contains("missing or unreadable"));
    let mut data = own_data();
    data[0]["families"][0]["concepts"][0]["cardBoard"] = json!("https://example.test/card.webp");
    let env = project.catalog(&data);
    assert!(
        render_concept_seed(&env, project.cwd(), &args("direction", "operate"))
            .unwrap_err()
            .1
            .contains("relative local asset path")
    );
    data[0]["families"][0]["concepts"][0]["source"]["license"] = json!(null);
    let env = project.catalog(&data);
    assert!(
        render_concept_seed(&env, project.cwd(), &args("direction", "operate"))
            .unwrap_err()
            .1
            .contains("source.license")
    );
}

#[test]
fn malformed_reviews_and_invalid_dates_are_input_errors() {
    let project = Project::new();
    for (index, key, value) in [
        (1, "reviews", json!([])),
        (3, "reviews", json!([])),
        (1, "reviewedAt", json!("2026-99-99")),
        (3, "reviewedAt", json!("2026-02-30T12:00:00Z")),
        (1, "reviewedAt", json!("review-1-q")),
        (3, "rating", json!(4)),
        (3, "breadth", json!("unknown")),
        (3, "allowedModes", json!(["unknown"])),
        (3, "note", json!("")),
    ] {
        let mut data = own_data();
        if key == "reviews" {
            data[index][key] = value;
        } else {
            data[index]["reviews"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .next()
                .unwrap()[key] = value;
        }
        let env = project.catalog(&data);
        let (mut io, capture) = Io::captured("", project.0.clone(), env);
        assert_eq!(run(&[], &mut io), 2, "{index}/{key}");
        assert!(!capture.stderr.borrow().is_empty(), "{index}/{key}");
    }
}

#[test]
fn portal_dashboard_forms_are_not_rejected_by_aesthetic_words() {
    let mut data = own_data();
    let entry = &mut data[0]["families"][0]["concepts"][0];
    entry["form"] = json!("a Portal operations dashboard, where a stable sidebar and panel index keep ownership, current filters and the selected work item visible");
    assert!(
        crate::catalog::validate_concept_entry(entry, &std::collections::HashMap::new()).is_empty()
    );
}

#[test]
fn stale_composition_reviews_and_mixed_catalog_versions_are_rejected() {
    let mut data = own_data();
    data[2]["compositions"][0]["form"] =
        json!("changed composition, with a different spatial hierarchy");
    assert!(local_from_values(
        data[0].clone(),
        data[1].clone(),
        data[2].clone(),
        data[3].clone()
    )
    .err()
    .unwrap()
    .contains("current formHash"));
    let mut data = own_data();
    data[2]["catalogVersion"] = json!("another-version");
    assert!(local_from_values(
        data[0].clone(),
        data[1].clone(),
        data[2].clone(),
        data[3].clone()
    )
    .err()
    .unwrap()
    .contains("catalogVersion must match"));
}

#[test]
fn chosen_compatibility_never_contacts_the_legacy_service() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/api", listener.local_addr().unwrap());
    let env = Env::from([
        ("IMPECCABLE_API_URL".into(), url.clone()),
        ("PHOENIX_UI_API_URL".into(), url),
    ]);
    let project = Project::new();
    let (mut io, capture) = Io::captured("", project.0.clone(), env);
    assert_eq!(run(&["--chosen".into(), "local-choice".into()], &mut io), 0);
    assert!(String::from_utf8(capture.stdout.borrow().clone())
        .unwrap()
        .contains("telemetry disabled"));
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(!project.0.join(".phoenix-ui").exists());
}
