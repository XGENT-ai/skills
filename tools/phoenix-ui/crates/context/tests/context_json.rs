use impeccable_common::Io;
use impeccable_context::{run_context, run_surface_brief};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-context-json-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let fixture = Self(root);
        fixture.write("package.json", "{\"name\":\"context-fixture\"}\n");
        fixture.write("src/page.tsx", "export default () => <main />;\n");
        fixture
    }

    fn write(&self, relative: &str, body: &str) {
        let file = self.0.join(relative);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, body).unwrap();
    }

    fn run(&self, args: &[&str]) -> (i32, Value, String) {
        self.run_at(self.0.clone(), args)
    }

    fn run_at(&self, cwd: PathBuf, args: &[&str]) -> (i32, Value, String) {
        let args = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        let home = self.0.join("home").to_string_lossy().into_owned();
        let env = HashMap::from([("HOME".into(), home.clone()), ("USERPROFILE".into(), home)]);
        let (mut io, capture) = Io::captured("", cwd, env);
        let code = run_context(&args, &mut io);
        let stdout = String::from_utf8(capture.stdout.borrow().clone()).unwrap();
        let stderr = String::from_utf8(capture.stderr.borrow().clone()).unwrap();
        let value =
            serde_json::from_str(&stdout).expect("--json must emit exactly one JSON object");
        (code, value, stderr)
    }

    fn link(&self, path: &str, decision: &str) -> (i32, Value, String) {
        self.link_to("src/page.tsx", path, decision)
    }

    fn link_to(&self, target: &str, path: &str, decision: &str) -> (i32, Value, String) {
        let args = ["link", target, path, "--decision", decision].map(String::from);
        let (mut io, capture) = Io::captured("", self.0.clone(), HashMap::new());
        let code = run_surface_brief(&args, &mut io);
        let stdout = String::from_utf8(capture.stdout.borrow().clone()).unwrap();
        let stderr = String::from_utf8(capture.stderr.borrow().clone()).unwrap();
        let value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
        (code, value, stderr)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn context_json_preserves_documents_and_records_their_sources_without_writing() {
    let fixture = Fixture::new();
    let product = "# 产品\r\n\r\n运营工作区。\r\n";
    let design = "# 设计\n\n## 品牌与密度\n保留项目原有字体，不使用默认颜色。\n";
    fixture.write("PRODUCT.md", product);
    fixture.write("DESIGN.md", design);
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.is_empty());
    for key in [
        "schemaVersion",
        "projectRoot",
        "target",
        "product",
        "design",
        "surfaceBrief",
        "profile",
        "provenance",
        "parseCoverage",
        "conflicts",
        "capabilities",
    ] {
        assert!(value.get(key).is_some(), "missing public field {key}");
    }
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["product"]["body"], product);
    assert_eq!(value["design"]["body"], design);
    assert_eq!(
        value["design"]["sha256"],
        format!("{:x}", Sha256::digest(design.as_bytes()))
    );
    assert_eq!(value["design"]["status"], "present");
    assert_eq!(value["parseCoverage"][1]["format"], "markdown");
    assert_eq!(value["parseCoverage"][1]["coverage"], "partial");
    assert!(value["provenance"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source["artifact"] == "design" && source["relationship"] == "explicit"));
    assert!(!fixture.0.join(".phoenix-ui").exists());
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("PRODUCT.md")).unwrap(),
        product
    );
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("DESIGN.md")).unwrap(),
        design
    );
}

#[test]
fn missing_documents_and_disabled_profiles_do_not_inject_template_values() {
    let fixture = Fixture::new();
    let (code, value, stderr) = fixture.run(&["--json", "--target=src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    for name in ["product", "design", "surfaceBrief"] {
        assert_eq!(value[name]["status"], "missing");
        assert!(value[name]["body"].is_null());
    }
    assert_eq!(value["profile"]["status"], "disabled");
    assert_eq!(value["parseCoverage"], serde_json::json!([]));
    assert_eq!(value["conflicts"], serde_json::json!([]));
    assert!(!value.to_string().contains("#2563EB"));
}

#[test]
fn accepted_brief_links_the_original_document_and_restores_it_in_a_fresh_context() {
    let fixture = Fixture::new();
    let body = "# 已接受方向\r\n用户决定：保留现有字体，采用宽行布局。\r\n";
    fixture.write("docs/design/orders.md", body);
    let (code, linked, stderr) = fixture.link(
        "docs/design/orders.md",
        "request-42: 用户接受方向A并继续实现",
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(linked["status"], "linked");
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("docs/design/orders.md")).unwrap(),
        body
    );
    let index = std::fs::read_to_string(fixture.0.join(".phoenix-ui/briefs.json")).unwrap();
    assert!(
        !index.contains("宽行布局"),
        "the index must not duplicate editable brief content"
    );
    assert!(!fixture.0.join(".phoenix-ui/surfaces").exists());
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["surfaceBrief"]["status"], "accepted");
    assert_eq!(value["surfaceBrief"]["body"], body);
    assert_eq!(
        value["surfaceBrief"]["acceptedDecision"],
        "request-42: 用户接受方向A并继续实现"
    );
    assert_eq!(
        value["surfaceBrief"]["sha256"],
        format!("{:x}", Sha256::digest(body.as_bytes()))
    );
    assert_eq!(value["conflicts"], serde_json::json!([]));
}

#[test]
fn accepted_brief_target_is_relative_to_the_selected_monorepo_project() {
    let fixture = Fixture::new();
    fixture.write(
        "package.json",
        r#"{"private":true,"workspaces":["apps/*"]}"#,
    );
    fixture.write("apps/admin/package.json", r#"{"name":"admin"}"#);
    fixture.write(
        "apps/admin/src/page.tsx",
        "export default () => <main />;\n",
    );
    let body = "# Admin direction\nKeep the accepted table layout.\n";
    fixture.write("apps/admin/docs/design/orders.md", body);
    let target = "apps/admin/src/page.tsx";
    let (code, _, stderr) = fixture.link_to(
        target,
        "apps/admin/docs/design/orders.md",
        "request-3: accept admin direction",
    );
    assert_eq!(code, 0, "{stderr}");
    let index: Value = serde_json::from_slice(
        &std::fs::read(fixture.0.join("apps/admin/.phoenix-ui/briefs.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["briefs"][0]["target"], "src/page.tsx");
    assert_eq!(index["briefs"][0]["path"], "docs/design/orders.md");
    for target in [
        target.to_owned(),
        fixture.0.join(target).to_string_lossy().into_owned(),
    ] {
        let (code, value, stderr) = fixture.run(&["--json", "--target", &target]);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(value["surfaceBrief"]["status"], "accepted");
        assert_eq!(value["surfaceBrief"]["body"], body);
    }
}

#[test]
fn accepted_monorepo_brief_restores_the_shared_original_without_copying() {
    let fixture = Fixture::new();
    fixture.write(
        "package.json",
        r#"{"private":true,"workspaces":["apps/*"]}"#,
    );
    fixture.write("apps/admin/package.json", r#"{"name":"admin"}"#);
    fixture.write(
        "apps/admin/src/page.tsx",
        "export default () => <main />;\n",
    );
    let body = "# Shared direction\nKeep the accepted navigation and table layout.\n";
    fixture.write("docs/design/orders.md", body);
    let (code, _, stderr) = fixture.link_to(
        "apps/admin/src/page.tsx",
        "docs/design/orders.md",
        "request-4: accept shared direction",
    );
    assert_eq!(code, 0, "{stderr}");
    let index: Value = serde_json::from_slice(
        &std::fs::read(fixture.0.join("apps/admin/.phoenix-ui/briefs.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["briefs"][0]["target"], "src/page.tsx");
    assert_eq!(index["briefs"][0]["path"], "docs/design/orders.md");
    assert_eq!(index["briefs"][0]["sourceRoot"], "repository");
    for (cwd, target) in [
        (fixture.0.clone(), "apps/admin/src/page.tsx"),
        (fixture.0.join("apps/admin"), "src/page.tsx"),
    ] {
        let (code, value, stderr) = fixture.run_at(cwd, &["--json", "--target", target]);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(value["surfaceBrief"]["status"], "accepted");
        assert_eq!(value["surfaceBrief"]["body"], body);
        assert_eq!(
            value["surfaceBrief"]["path"],
            fixture
                .0
                .join("docs/design/orders.md")
                .to_string_lossy()
                .as_ref()
        );
        assert!(value["provenance"]
            .as_array()
            .unwrap()
            .iter()
            .any(|source| source["artifact"] == "surfaceBrief"
                && source["relationship"] == "inherited"));
    }
    assert!(!fixture.0.join("apps/admin/docs/design/orders.md").exists());
    assert!(!fixture.0.join(".phoenix-ui").exists());
    fixture.write("docs/design/orders.md", "# Changed without acceptance\n");
    let (_, value, _) = fixture.run(&["--json", "--target", "apps/admin/src/page.tsx"]);
    assert_eq!(value["surfaceBrief"]["status"], "stale");
    let outside = Fixture::new();
    outside.write("docs/design/outside.md", "# Unrelated repository\n");
    let (code, _, stderr) = fixture.link_to(
        "apps/admin/src/page.tsx",
        outside.0.join("docs/design/outside.md").to_str().unwrap(),
        "request-5: accept outside",
    );
    assert_eq!(code, 2, "{stderr}");
    fixture.write(
        "apps/admin/docs/design/orders.md",
        "# A competing local direction\n",
    );
    let (code, _, stderr) = fixture.link_to(
        "apps/admin/src/page.tsx",
        "apps/admin/docs/design/orders.md",
        "request-6: accept local direction",
    );
    assert_eq!(code, 0, "{stderr}");
    let (_, value, _) = fixture.run(&["--json", "--target", "apps/admin/src/page.tsx"]);
    assert_eq!(value["surfaceBrief"]["status"], "ambiguous");
    assert_eq!(
        value["surfaceBrief"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn changed_or_competing_accepted_briefs_are_conflicts_and_are_not_silently_reaccepted() {
    let fixture = Fixture::new();
    fixture.write("docs/design/a.md", "# Direction A\nAccepted original.\n");
    assert_eq!(fixture.link("docs/design/a.md", "request-1: accept A").0, 0);
    let original_index = std::fs::read(fixture.0.join(".phoenix-ui/briefs.json")).unwrap();
    fixture.write(
        "docs/design/a.md",
        "# Direction A\nChanged without another acceptance.\n",
    );
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["surfaceBrief"]["status"], "stale");
    assert_eq!(value["conflicts"][0]["kind"], "accepted_brief_changed");
    assert_eq!(
        std::fs::read(fixture.0.join(".phoenix-ui/briefs.json")).unwrap(),
        original_index
    );
    fixture.write(
        "docs/design/b.md",
        "# Direction B\nA competing accepted direction.\n",
    );
    assert_eq!(fixture.link("docs/design/b.md", "request-2: accept B").0, 0);
    let (_, value, _) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(value["surfaceBrief"]["status"], "ambiguous");
    assert!(value["surfaceBrief"]["body"].is_null());
    assert_eq!(
        value["surfaceBrief"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(value["conflicts"][0]["kind"], "ambiguous_accepted_brief");
}

#[test]
fn accepted_brief_link_requires_acceptance_and_preserves_a_corrupt_index() {
    let fixture = Fixture::new();
    fixture.write("docs/design/a.md", "# Brief\n");
    assert_eq!(fixture.link("docs/design/a.md", "").0, 2);
    assert!(!fixture.0.join(".phoenix-ui").exists());
    fixture.write(".phoenix-ui/briefs.json", "{corrupt index");
    assert_eq!(fixture.link("docs/design/a.md", "request-1: accept A").0, 4);
    assert_eq!(
        std::fs::read_to_string(fixture.0.join(".phoenix-ui/briefs.json")).unwrap(),
        "{corrupt index"
    );
    assert_eq!(fixture.run(&["--json", "--target", "src/page.tsx"]).0, 4);
}

#[test]
fn accepted_brief_link_obeys_the_os_lock_and_releases_it_after_the_owner_exits() {
    let fixture = Fixture::new();
    fixture.write("docs/design/a.md", "# Brief\n");
    fixture.write(".phoenix-ui/briefs.lock", "");
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.0.join(".phoenix-ui/briefs.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert_eq!(fixture.link("docs/design/a.md", "request-1: accept A").0, 3);
    assert!(!fixture.0.join(".phoenix-ui/briefs.json").exists());
    drop(lock);
    assert_eq!(fixture.link("docs/design/a.md", "request-1: accept A").0, 0);
}

#[cfg(unix)]
#[test]
fn accepted_brief_link_rejects_symlinked_sources_and_state_directories() {
    let fixture = Fixture::new();
    fixture.write("docs/design/a.md", "# Brief\n");
    std::os::unix::fs::symlink("a.md", fixture.0.join("docs/design/link.md")).unwrap();
    assert_eq!(
        fixture.link("docs/design/link.md", "request-1: accept A").0,
        2
    );
    assert!(!fixture.0.join(".phoenix-ui").exists());
    fixture.write("other/keep.txt", "unrelated bytes");
    std::os::unix::fs::symlink(fixture.0.join("other"), fixture.0.join(".phoenix-ui")).unwrap();
    assert_eq!(fixture.link("docs/design/a.md", "request-1: accept A").0, 2);
    assert!(!fixture.0.join("other/briefs.json").exists());
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("other/keep.txt")).unwrap(),
        "unrelated bytes"
    );
}

#[test]
fn monorepo_context_identifies_local_product_and_inherited_design() {
    let fixture = Fixture::new();
    fixture.write("package.json", "{\"workspaces\":[\"apps/*\"]}\n");
    fixture.write("pnpm-workspace.yaml", "packages:\n  - apps/*\n");
    fixture.write(
        "DESIGN.md",
        "# Shared design\nKeep the accepted shared values.\n",
    );
    fixture.write("apps/admin/package.json", "{\"name\":\"admin\"}\n");
    fixture.write(
        "apps/admin/PRODUCT.md",
        "# Admin product\nOperate existing records.\n",
    );
    fixture.write(
        "apps/admin/src/page.tsx",
        "export default () => <main />;\n",
    );
    let (code, value, stderr) = fixture.run(&["--json", "--target", "apps/admin/src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        value["projectRoot"],
        fixture.0.join("apps/admin").to_string_lossy().as_ref()
    );
    assert_eq!(
        value["product"]["body"],
        "# Admin product\nOperate existing records.\n"
    );
    assert!(value["provenance"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source["artifact"] == "design" && source["relationship"] == "inherited"));
}

#[test]
fn yaml_tokens_are_reported_with_missing_and_cyclic_reference_conflicts() {
    let fixture = Fixture::new();
    fixture.write("DESIGN.md", "---\ncolors:\n  primary: '#123456'\n  broken: '{colors.missing}'\n  loop-a: '{colors.loop-b}'\n  loop-b: '{colors.loop-a}'\n---\n\n## Colors\nUse accepted tokens.\n");
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["design"]["tokens"]["colors"]["primary"], "#123456");
    assert_eq!(value["parseCoverage"][0]["format"], "markdown-yaml-subset");
    let conflicts = value["conflicts"].as_array().unwrap();
    assert!(
        conflicts
            .iter()
            .any(|item| item["kind"] == "invalid_token_reference"
                && item["token"] == "colors.missing")
    );
    assert!(conflicts
        .iter()
        .any(|item| item["kind"] == "cyclic_token_reference"));
}

#[test]
fn unsupported_yaml_remains_readable_with_an_explicit_coverage_limit() {
    let fixture = Fixture::new();
    let body = "---\ncolors:\n  modes: [light, dark]\ndescription: |\n  Multi-line narrative\n---\n\n## 主题\n这仍然是已接受的项目规范。\n";
    fixture.write("DESIGN.md", body);
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["design"]["body"], body);
    assert!(value["design"]["tokens"].is_null());
    assert_eq!(value["parseCoverage"][0]["coverage"], "partial");
    assert!(!value["parseCoverage"][0]["unsupported"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("DESIGN.md")).unwrap(),
        body
    );
}

#[test]
fn json_parameter_errors_are_separate_from_stdout_and_keep_the_json_contract() {
    let fixture = Fixture::new();
    let (code, value, stderr) = fixture.run(&["--json", "--target"]);
    assert_eq!(code, 2);
    assert_eq!(value["status"], "invalid_input");
    assert!(!stderr.is_empty());
}

#[test]
fn profile_generic_is_explicit_versioned_and_does_not_replace_project_rules() {
    let fixture = Fixture::new();
    let config = r#"{"profile":{"enabled":true,"id":"generic","version":"1.0.0"}}"#;
    let design = "# Design\nUse the accepted project font and spacing.\n";
    fixture.write(".phoenix-ui/config.json", config);
    fixture.write("DESIGN.md", design);
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["profile"]["status"], "enabled");
    assert_eq!(value["profile"]["id"], "generic");
    assert_eq!(value["profile"]["version"], "1.0.0");
    assert_eq!(value["profile"]["authority"], "advisory");
    assert_eq!(value["design"]["body"], design);
    assert!(!value.to_string().contains("#2563EB"));
    assert_eq!(
        std::fs::read_to_string(fixture.0.join(".phoenix-ui/config.json")).unwrap(),
        config
    );
    assert!(value["provenance"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source["artifact"] == "profile"
            && source["path"].as_str().unwrap().ends_with("config.json")));
}

#[test]
fn profile_portal_binds_only_accepted_tokens_and_keeps_template_values_out() {
    let fixture = Fixture::new();
    let design = "---\ncolors:\n  app-identity: '#654321'\n  app-identity-dark: '#FEDCBA'\n---\n\n# Design\nKeep the project's chosen font.\n";
    fixture.write("DESIGN.md", design);
    fixture.write(
        ".phoenix-ui/config.json",
        r#"{"profile":{"enabled":true,"id":"portal","version":"1.0.0"}}"#,
    );
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["profile"]["id"], "portal");
    assert_eq!(
        value["profile"]["tokenBindings"]["colors.app-identity"]["value"],
        "#654321"
    );
    assert_eq!(
        value["profile"]["tokenBindings"]["colors.app-identity"]["sha256"],
        value["design"]["sha256"]
    );
    assert_eq!(value["profile"]["scope"], "portal-content-area");
    assert_eq!(value["design"]["body"], design);
    assert!(!value["profile"].to_string().contains("#2563EB"));
    assert!(!value["profile"].to_string().contains("Space Grotesk"));
    assert!(value["profile"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source["path"]
            == "skills/xgent-init/references/external-app-DESIGN.template.md"));
}

#[test]
fn profile_local_disable_and_unsupported_versions_are_read_without_upgrading() {
    let fixture = Fixture::new();
    fixture.write(
        ".phoenix-ui/config.json",
        r#"{"profile":{"enabled":true,"id":"generic","version":"1.0.0"}}"#,
    );
    let disabled = r#"{"profile":{"enabled":false}}"#;
    fixture.write(".phoenix-ui/config.local.json", disabled);
    let (code, value, stderr) = fixture.run(&["--json"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["profile"]["status"], "disabled");
    assert!(value["profile"]["source"]["path"]
        .as_str()
        .unwrap()
        .ends_with("config.local.json"));
    for (id, version, kind) in [
        ("unknown", "1.0.0", "unknown_profile"),
        ("portal", "0.9.0", "profile_version_mismatch"),
    ] {
        let config =
            serde_json::json!({"profile":{"enabled":true,"id":id,"version":version}}).to_string();
        fixture.write(".phoenix-ui/config.local.json", &config);
        let (code, value, stderr) = fixture.run(&["--json"]);
        assert_eq!(code, 0, "{stderr}");
        assert_ne!(value["profile"]["status"], "enabled");
        assert!(value["conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["kind"] == kind));
        assert_eq!(
            std::fs::read_to_string(fixture.0.join(".phoenix-ui/config.local.json")).unwrap(),
            config
        );
    }
}

#[test]
fn profile_config_corruption_is_unavailable_instead_of_silently_disabled() {
    let fixture = Fixture::new();
    fixture.write(".phoenix-ui/config.json", "{bad JSON");
    let (code, value, stderr) = fixture.run(&["--json"]);
    assert_eq!(code, 4);
    assert_eq!(value["status"], "unavailable");
    assert!(stderr.contains("config.json"));
    assert_eq!(
        std::fs::read_to_string(fixture.0.join(".phoenix-ui/config.json")).unwrap(),
        "{bad JSON"
    );
}

#[test]
fn nested_cwd_recovers_project_norms_profile_and_the_accepted_original() {
    for git in [false, true] {
        let fixture = Fixture::new();
        if git {
            fixture.write(".git", "gitdir: ignored-fixture\n");
        }
        let design = "# Accepted project design\nUse our established font.\n";
        let brief = "# Accepted direction\nPreserve the wide table layout.\n";
        fixture.write("PRODUCT.md", "# Product\nOperate existing records.\n");
        fixture.write("DESIGN.md", design);
        fixture.write("docs/design/table.md", brief);
        fixture.write(
            ".phoenix-ui/config.json",
            r#"{"profile":{"enabled":true,"id":"generic","version":"1.0.0"}}"#,
        );
        assert_eq!(
            fixture
                .link("docs/design/table.md", "request-7: accept table")
                .0,
            0
        );
        let before = std::fs::read(fixture.0.join(".phoenix-ui/briefs.json")).unwrap();
        for args in [vec!["--json", "--target", "page.tsx"], vec!["--json"]] {
            let (code, value, stderr) = fixture.run_at(fixture.0.join("src"), &args);
            assert_eq!(code, 0, "{stderr}");
            assert_eq!(value["projectRoot"], fixture.0.to_string_lossy().as_ref());
            assert_eq!(value["repoRoot"], fixture.0.to_string_lossy().as_ref());
            assert_eq!(value["design"]["body"], design);
            assert_eq!(value["profile"]["status"], "enabled");
            assert_eq!(value["surfaceBrief"]["status"], "accepted");
            assert_eq!(value["surfaceBrief"]["body"], brief);
        }
        assert_eq!(
            std::fs::read(fixture.0.join(".phoenix-ui/briefs.json")).unwrap(),
            before
        );
        assert!(!fixture.0.join("src/.phoenix-ui").exists());
    }
}

#[test]
fn product_and_design_resolve_independently_across_document_locations() {
    let fixture = Fixture::new();
    let product = "# Root product\nExisting operations.\n";
    let design = "# Design in docs\nKeep the accepted typography.\n";
    fixture.write("PRODUCT.md", product);
    fixture.write("docs/DESIGN.md", design);
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["product"]["body"], product);
    assert_eq!(value["design"]["body"], design);
    fixture.write("DESIGN.md", "# Explicit root design\n");
    let (_, value, _) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(value["design"]["body"], "# Explicit root design\n");
    assert_eq!(
        std::fs::read_to_string(fixture.0.join("docs/DESIGN.md")).unwrap(),
        design
    );
    assert!(!fixture.0.join(".phoenix-ui").exists());
}

#[test]
fn a_nested_repository_does_not_inherit_outer_norms_or_profile() {
    let fixture = Fixture::new();
    fixture.write("DESIGN.md", "# Outer design\n");
    fixture.write(
        ".phoenix-ui/config.json",
        r#"{"profile":{"enabled":true,"id":"portal","version":"1.0.0"}}"#,
    );
    fixture.write("independent/.git", "gitdir: isolated-fixture\n");
    fixture.write(
        "independent/src/page.tsx",
        "export default () => <main />;\n",
    );
    let (code, value, stderr) = fixture.run_at(
        fixture.0.join("independent/src"),
        &["--json", "--target", "page.tsx"],
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        value["projectRoot"],
        fixture.0.join("independent").to_string_lossy().as_ref()
    );
    assert_eq!(value["design"]["status"], "missing");
    assert_eq!(value["profile"]["status"], "disabled");
    assert_eq!(value["conflicts"], serde_json::json!([]));
}

#[test]
fn logical_route_and_url_targets_restore_the_original_without_filesystem_resolution() {
    for (target, normalized) in [
        ("route:/orders/?filter=active#table", "route:/orders"),
        (
            "http://localhost:3000/orders/?filter=active#table",
            "http://localhost:3000/orders",
        ),
        (
            "https://example.test/team/@user",
            "https://example.test/team/@user",
        ),
    ] {
        let fixture = Fixture::new();
        let body = "# Accepted operations direction\nKeep the established filters.\n";
        fixture.write("PRODUCT.md", "# Product\nOperations.\n");
        fixture.write("docs/design/orders.md", body);
        let (code, _, stderr) =
            fixture.link_to(target, "docs/design/orders.md", "request-8: accept orders");
        assert_eq!(code, 0, "{target}: {stderr}");
        assert!(fixture.0.join(".phoenix-ui/briefs.json").exists());
        let (code, value, stderr) =
            fixture.run_at(fixture.0.join("src"), &["--json", "--target", target]);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(value["projectRoot"], fixture.0.to_string_lossy().as_ref());
        assert_eq!(value["target"]["path"], normalized);
        assert!(value["target"]["exists"].is_null());
        assert_eq!(value["surfaceBrief"]["status"], "accepted");
        assert_eq!(value["surfaceBrief"]["body"], body);
    }
}

#[test]
fn portal_profile_does_not_bind_null_empty_or_mapping_tokens_as_values() {
    for invalid in ["null", "''", "\n    nested: '#123456'"] {
        let fixture = Fixture::new();
        let design = format!("---\ncolors:\n  app-identity: {invalid}\n  app-identity-dark: '#FEDCBA'\n---\n# Design\n");
        fixture.write("DESIGN.md", &design);
        fixture.write(
            ".phoenix-ui/config.json",
            r#"{"profile":{"enabled":true,"id":"portal","version":"1.0.0"}}"#,
        );
        let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(
            value["profile"]["tokenBindings"]["colors.app-identity"]["status"],
            "unresolved"
        );
        assert!(value["conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["kind"] == "unresolved_profile_token"
                && item["token"] == "colors.app-identity"));
        assert_eq!(value["design"]["body"], design);
    }
}

#[test]
fn generic_default_advice_is_separate_from_explicit_profile_and_target_authority() {
    let fixture = Fixture::new();
    fixture.write(
        "DESIGN.md",
        "# Accepted norm\nKeep the existing typeface.\n",
    );
    fixture.write(
        "docs/design/a.md",
        "# Accepted direction\nKeep the table.\n",
    );
    assert_eq!(
        fixture
            .link("docs/design/a.md", "request-9: accept table")
            .0,
        0
    );
    let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["profile"]["status"], "disabled");
    assert_eq!(value["defaultAdvice"]["id"], "generic");
    assert_eq!(value["defaultAdvice"]["authority"], "advisory");
    assert!(!value["defaultAdvice"]["rules"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        value["precedence"]["order"],
        serde_json::json!([
            "current_user_decision",
            "frozen_target_or_accepted_project_norm",
            "accepted_project_profile",
            "tool_default_advice"
        ])
    );
    assert_eq!(
        value["precedence"]["conflictPolicy"],
        "require_user_decision"
    );
    assert_eq!(
        value["precedence"]["profileRulesBecomeTargets"],
        "only_after_acceptance_as_project_norm"
    );
    assert_eq!(value["design"]["authority"], "project_norm");
    assert_eq!(value["surfaceBrief"]["authority"], "accepted_direction");
    assert_eq!(value["surfaceBrief"]["usable"], true);
    fixture.write("docs/design/a.md", "# Changed without acceptance\n");
    let (_, changed, _) = fixture.run(&["--json", "--target", "src/page.tsx"]);
    assert_eq!(changed["surfaceBrief"]["usable"], false);
    assert_eq!(changed["surfaceBrief"]["authority"], "unresolved_direction");
}

#[test]
fn profile_definition_pin_and_upgrade_advice_never_apply_a_new_definition() {
    let fixture = Fixture::new();
    fixture.write("docs/design/a.md", "# Frozen accepted direction\n");
    assert_eq!(
        fixture
            .link("docs/design/a.md", "request-10: accept direction")
            .0,
        0
    );
    let before = std::fs::read(fixture.0.join(".phoenix-ui/briefs.json")).unwrap();
    for (version, pin, status) in [
        ("0.9.0", None, "version_mismatch"),
        ("1.0.0", Some("0".repeat(64)), "definition_mismatch"),
    ] {
        let mut selection = serde_json::json!({"enabled":true,"id":"generic","version":version});
        if let Some(pin) = pin {
            selection["definitionSha256"] = Value::String(pin);
        }
        let config = serde_json::json!({"profile":selection}).to_string();
        fixture.write(".phoenix-ui/config.json", &config);
        let (code, value, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(value["profile"]["status"], status);
        assert!(value["profile"]["rules"].is_null());
        assert_eq!(value["profile"]["upgrade"]["applied"], false);
        assert_eq!(value["profile"]["upgrade"]["comparison"], "unavailable");
        assert_eq!(
            value["profile"]["upgrade"]["requiresExplicitAcceptance"],
            true
        );
        assert!(!value["profile"]["upgrade"]["candidate"]["rules"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(value["surfaceBrief"]["status"], "accepted");
        assert_eq!(
            std::fs::read(fixture.0.join(".phoenix-ui/briefs.json")).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read_to_string(fixture.0.join(".phoenix-ui/config.json")).unwrap(),
            config
        );
    }
}

#[test]
fn accepted_brief_link_rejects_url_credentials_and_non_text_sources_before_writing() {
    let fixture = Fixture::new();
    fixture.write("docs/design/a.md", "# Brief\n");
    let (code, _, stderr) = fixture.link_to(
        "HTTPS://user:secret@example.test/orders",
        "docs/design/a.md",
        "request-11: accept",
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(!fixture.0.join(".phoenix-ui").exists());
    std::fs::write(fixture.0.join("docs/design/a.md"), [0xff, 0xfe]).unwrap();
    let (code, _, stderr) = fixture.link("docs/design/a.md", "request-11: accept");
    assert_eq!(code, 2, "{stderr}");
    assert!(!fixture.0.join(".phoenix-ui").exists());
}

#[test]
fn accepted_brief_index_validation_rejects_credentials_and_preserves_the_index() {
    let fixture = Fixture::new();
    fixture.write("docs/design/a.md", "# Brief\n");
    for target in [
        "../page.tsx",
        "route:",
        "https://user:secret@example.test/orders",
    ] {
        let index = serde_json::json!({"schemaVersion":1,"briefs":[{
            "target":target,"path":"docs/design/a.md","sha256":"0".repeat(64),"acceptedDecision":"request-12: accept"
        }]}).to_string();
        fixture.write(".phoenix-ui/briefs.json", &index);
        let (code, _, stderr) = fixture.run(&["--json", "--target", "src/page.tsx"]);
        assert_eq!(code, 4, "{target}: {stderr}");
        assert_eq!(fixture.link("docs/design/a.md", "request-12: accept").0, 4);
        assert_eq!(
            std::fs::read_to_string(fixture.0.join(".phoenix-ui/briefs.json")).unwrap(),
            index
        );
    }
}

#[test]
fn monorepo_selection_reports_actual_document_sources_and_keeps_profile_scope() {
    let fixture = Fixture::new();
    fixture.write("package.json", r#"{"workspaces":["apps/*"]}"#);
    fixture.write("docs/DESIGN.md", "# Shared norm\n");
    fixture.write("apps/admin/package.json", r#"{"name":"admin"}"#);
    fixture.write("apps/admin/PRODUCT.md", "# Admin product\n");
    fixture.write("apps/admin/docs/DESIGN.md", "# Admin norm\n");
    fixture.write(
        "apps/admin/src/page.tsx",
        "export default () => <main />;\n",
    );
    fixture.write("apps/site/package.json", r#"{"name":"site"}"#);
    fixture.write("apps/site/PRODUCT.md", "# Site product\n");
    fixture.write("apps/site/src/page.tsx", "export default () => <main />;\n");
    fixture.write(
        ".phoenix-ui/config.json",
        r#"{"profile":{"enabled":true,"id":"generic","version":"1.0.0"}}"#,
    );
    let (code, value, stderr) = fixture.run(&["--json"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["status"], "selection_required");
    assert_eq!(value["projectRoot"], fixture.0.to_string_lossy().as_ref());
    assert_eq!(value["product"]["status"], "missing");
    assert_eq!(value["design"]["body"], "# Shared norm\n");
    assert_eq!(value["target"]["candidates"][0]["designStatus"], "fallback");
    assert_eq!(
        value["target"]["candidates"][0]["designPath"],
        "apps/admin/docs/DESIGN.md"
    );
    assert_eq!(
        value["target"]["candidates"][1]["designStatus"],
        "inherited"
    );
    assert_eq!(
        value["target"]["candidates"][1]["designPath"],
        "docs/DESIGN.md"
    );
    let (code, selected, stderr) = fixture.run(&["--json", "--target", "apps/admin/src/page.tsx"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(selected["design"]["body"], "# Admin norm\n");
    assert_eq!(selected["profile"]["status"], "disabled");
    assert!(!fixture.0.join("apps/admin/.phoenix-ui").exists());
}
