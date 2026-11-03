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
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn own_data() -> [Value; 4] {
    [
        CONCEPTS_JSON,
        CONCEPT_REVIEWS_JSON,
        COMPOSITIONS_JSON,
        COMPOSITION_REVIEWS_JSON,
    ]
    .map(|text| serde_json::from_str(text).unwrap())
}

fn catalog(data: [Value; 4]) -> Catalog {
    let [a, b, c, d] = data;
    catalog_from_values(a, b, c, d)
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
    let catalog = &*BUILTIN;
    assert_eq!(catalog.concepts.len(), 24);
    assert_eq!(catalog.compositions.len(), 8);
    let env = Env::from([("PHOENIX_UI_COMPOSITIONS".into(), "1".into())]);
    for scope in ["direction", "surface"] {
        for mode in SEED_MODES {
            let mut input = args(scope, mode);
            let first = render_concept_seed(&env, project.cwd(), &input, catalog).unwrap();
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
                render_concept_seed(&env, project.cwd(), &input, catalog).unwrap()
            );
            input.reroll = 1.0;
            let reroll = render_concept_seed(&env, project.cwd(), &input, catalog).unwrap();
            assert_eq!(
                reroll,
                render_concept_seed(&env, project.cwd(), &input, catalog).unwrap()
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
    let small = catalog(data);
    let env = Env::new();
    let one =
        render_concept_seed(&env, project.cwd(), &args("direction", "operate"), &small).unwrap();
    assert!(
        one.contains("status=insufficient; requested=6; available=1"),
        "{one}"
    );
    assert!(one.contains("catalog=phoenix-2026-01; seed=stable-local-seed"));
    let mut input = args("direction", "read");
    input.reroll = 1.0;
    input.register = Some(Some("bolder".into()));
    let zero = render_concept_seed(&env, project.cwd(), &input, &small).unwrap();
    assert!(zero.contains("available=0"), "{zero}");
    assert!(zero.contains("No eligible challenger leads this round"));
    assert!(zero.contains("PRODUCT.md, DESIGN.md"));
    assert!(!zero.contains("ASSIGNED INDEX"));
}

#[test]
fn an_empty_catalog_succeeds_without_invented_candidates() {
    let project = Project::new();
    let mut data = own_data();
    data[0]["families"] = json!([]);
    data[1]["reviews"] = json!({});
    data[2]["compositions"] = json!([]);
    data[3]["reviews"] = json!({});
    let text = render_concept_seed(
        &Env::new(),
        project.cwd(),
        &args("surface", "operate"),
        &catalog(data),
    )
    .unwrap();
    assert!(
        text.contains("status=insufficient; requested=6; available=0"),
        "{text}"
    );
}

#[test]
fn invalid_arguments_fail_with_the_argument_diagnostic() {
    let project = Project::new();
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
        let (mut io, capture) = Io::captured("", project.0.clone(), Env::new());
        let argv = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        assert_eq!(run(&argv, &mut io), 1, "{args:?}");
        assert!(capture.stdout.borrow().is_empty());
        let diagnostic = String::from_utf8(capture.stderr.borrow().clone()).unwrap();
        assert!(diagnostic.starts_with("concept-seed: --"), "{diagnostic}");
    }
}
