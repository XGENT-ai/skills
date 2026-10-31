use phoenix_diagram::{
    check::FileIdentity,
    delivery::{self, CompileRequest, EngineIdentity, Status},
    evidence::{verify, verify_with, EvidenceStatus, GitRunner, SourceEvidence},
    input::{parse_input, sha256, CompileOptions, DiagramInput, DiagramType},
    render::EmbeddedAssets,
};
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

const REPOSITORY: &str = "https://github.com/example/repo";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phoenix-evidence-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let output = command
        .args([
            "-c",
            "user.name=Phoenix",
            "-c",
            "user.email=phoenix@example.invalid",
        ])
        .args([
            "-c",
            "init.defaultBranch=main",
            "-c",
            "protocol.file.allow=always",
        ])
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// A committed repository with the cited files and an https origin.
fn repository(origin: &str) -> (Temp, String) {
    let temp = Temp::new();
    let root = temp.join("repo");
    fs::create_dir_all(root.join("src/nested")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "fn a() {}\r\nfn b() {}\r\n\nfn d() {}",
    )
    .unwrap();
    fs::write(root.join("src/nested/deep.rs"), "one\ntwo\nthree\n").unwrap();
    fs::write(root.join("README.md"), "# Title\n\nBody\n").unwrap();
    fs::write(root.join("empty.txt"), "").unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["remote", "add", "origin", origin]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-q", "-m", "fixture"]);
    let revision = git(&root, &["rev-parse", "HEAD"]);
    (temp, revision)
}

fn document(kind: DiagramType) -> Value {
    let body = match kind {
        DiagramType::Architecture => {
            json!({"components":[{"id":"a","type":"frontend","label":"A"},{"id":"b","type":"backend","label":"B"}],"connections":[{"from":"a","to":"b","label":"call"}]})
        }
        DiagramType::Workflow => {
            json!({"lanes":[{"id":"l","label":"L"}],"nodes":[{"id":"a","lane":"l","col":0,"type":"frontend","label":"A"},{"id":"b","lane":"l","col":1,"type":"backend","label":"B"}],"edges":[{"from":"a","to":"b","label":"next"}]})
        }
        DiagramType::Sequence => {
            json!({"participants":[{"id":"a","type":"frontend","label":"A"},{"id":"b","type":"backend","label":"B"}],"messages":[{"from":"a","to":"b","y":160,"label":"call"}]})
        }
        DiagramType::Dataflow => {
            json!({"stages":[{"label":"In"},{"label":"Out"}],"nodes":[{"id":"a","type":"frontend","label":"A","stage":0,"row":0},{"id":"b","type":"backend","label":"B","stage":1,"row":0}],"flows":[{"from":"a","to":"b","label":"rows"}]})
        }
        DiagramType::Lifecycle => {
            json!({"schema_version":3,"mainPath":["a","b"],"states":[{"id":"a","type":"start","label":"A"},{"id":"b","type":"success","label":"B"}],"transitions":[{"from":"a","to":"b","label":"go"}]})
        }
        DiagramType::Erd => {
            json!({"entities":[{"id":"a","label":"A","row":0,"col":0,"attributes":[{"name":"id","key":"pk"}]},{"id":"b","label":"B","row":0,"col":1,"attributes":[{"name":"a_id","key":"fk","references":"a.id"}]}],"relationships":[{"from":"a","to":"b","fromCardinality":"one","toCardinality":"many"}]})
        }
        DiagramType::Class => {
            json!({"types":[{"id":"a","label":"A","kind":"interface","row":0,"col":0},{"id":"b","label":"B","kind":"class","row":1,"col":0}],"relationships":[{"from":"b","to":"a","kind":"realization"}]})
        }
        DiagramType::Tree => {
            json!({"nodes":[{"id":"a","label":"A"},{"id":"b","label":"B","parent":"a"}]})
        }
        DiagramType::Timeline => {
            json!({"events":[{"id":"a","at":"2026-01-01T10:00Z","title":"A"},{"id":"b","at":"2026-01-01T11:00Z","title":"B"}]})
        }
        DiagramType::Waterfall => {
            json!({"spans":[{"id":"a","name":"A","start":0,"end":10},{"id":"b","name":"B","start":1,"end":4,"parent":"a"}]})
        }
    };
    let mut document = json!({"schema_version":1,"diagram_type":kind.as_str(),"meta":{"title":"Evidence","output":"evidence.html","input_contract":"phoenix-v1"}});
    if kind == DiagramType::Workflow {
        document["schema_version"] = json!(2);
    }
    if matches!(kind, DiagramType::Timeline) {
        document["meta"]["evidence"] = json!("observed");
    }
    if matches!(kind, DiagramType::Waterfall) {
        document["meta"]["evidence"] = json!("measured");
    }
    for (key, value) in body.as_object().unwrap() {
        document[key] = value.clone();
    }
    document
}

fn collection(kind: DiagramType) -> &'static str {
    match kind {
        DiagramType::Architecture => "components",
        DiagramType::Sequence => "participants",
        DiagramType::Lifecycle => "states",
        DiagramType::Erd => "entities",
        DiagramType::Class => "types",
        DiagramType::Timeline => "events",
        DiagramType::Waterfall => "spans",
        _ => "nodes",
    }
}

fn cited(kind: DiagramType, revision: &str, sources: Value) -> Value {
    let mut document = document(kind);
    document["meta"]["repository"] = json!({"url":REPOSITORY,"revision":revision});
    document[collection(kind)][0]["sources"] = sources;
    document
}

fn input(kind: DiagramType, document: &Value) -> DiagramInput {
    parse_input(
        &serde_json::to_vec(document).unwrap(),
        kind,
        &CompileOptions::default(),
    )
    .unwrap_or_else(|errors| panic!("{}: {errors:?}", kind.as_str()))
}

fn observe(document: &Value, root: Option<&Path>) -> SourceEvidence {
    verify(&input(DiagramType::Architecture, document), root).unwrap()
}

fn reason(document: &Value, root: Option<&Path>, git: &GitRunner) -> Option<String> {
    let evidence = verify_with(&input(DiagramType::Architecture, document), root, git).unwrap();
    assert_eq!(evidence.references.len(), 1);
    evidence.references[0].reason.clone()
}

fn rejects(document: &Value, code: &str, path: &str) -> Value {
    let diagnostics = verify(&input(DiagramType::Architecture, document), None).unwrap_err();
    let found = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == code && diagnostic.subject.path == path)
        .unwrap_or_else(|| panic!("expected {code} at {path}, got {diagnostics:?}"));
    json!({"message":found.message,"evidence":found.evidence})
}

#[test]
fn every_type_binds_its_primary_elements_to_lines_of_the_pinned_commit() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let root = temp.join("repo");
    for kind in DiagramType::ALL {
        let source = json!([{"path":"src/lib.rs","line":1,"end_line":2,"label":"entry"}]);
        let mut document = cited(kind, &revision, source.clone());
        let evidence = verify(&input(kind, &document), Some(&root)).unwrap();
        assert_eq!(
            evidence.status,
            EvidenceStatus::Partial,
            "{}",
            kind.as_str()
        );
        assert_eq!(evidence.unsourced, ["b"], "{}", kind.as_str());
        let reference = &evidence.references[0];
        assert_eq!(
            reference.pointer,
            format!("/{}/0/sources/0", collection(kind))
        );
        assert_eq!(
            (reference.node_id.as_str(), reference.state.as_str()),
            ("a", "verified")
        );
        assert_eq!(
            reference.href.as_deref(),
            Some(format!("{REPOSITORY}/blob/{revision}/src/lib.rs#L1-L2").as_str())
        );
        document[collection(kind)][1]["sources"] = json!([{"path":"src/nested/deep.rs"}]);
        let evidence = verify(&input(kind, &document), Some(&root)).unwrap();
        assert_eq!(
            evidence.status,
            EvidenceStatus::Verified,
            "{}",
            kind.as_str()
        );
        let repository = evidence.repository.unwrap();
        assert_eq!(repository.label, "example/repo");
        assert_eq!(repository.short_revision, &revision[..7]);
        assert_eq!(
            repository.tree_href.unwrap(),
            format!("{REPOSITORY}/tree/{revision}")
        );
        let observation = evidence.observation.unwrap();
        assert_eq!(
            (
                observation.head_matches_revision,
                observation.worktree.as_str()
            ),
            (Some(true), "compared")
        );
    }
}

#[test]
fn local_only_ssh_origins_and_each_forge_link_format() {
    let (temp, revision) = repository("git@github.com:Example/Repo.git");
    let root = temp.join("repo");
    let mut document = cited(
        DiagramType::Architecture,
        &revision,
        json!([{"path":"README.md","line":2,"end_line":3}]),
    );
    let evidence = observe(&document, Some(&root));
    assert_eq!(
        evidence.references[0].state, "verified",
        "SSH origin names the same GitHub repository"
    );
    document["meta"]["repository"]["link_mode"] = json!("local-only");
    let local = observe(&document, Some(&root));
    assert_eq!(local.references[0].href, None);
    let repository = local.repository.unwrap();
    assert_eq!(
        (repository.link_mode.as_str(), repository.tree_href),
        ("local-only", None)
    );
    for (origin, url, provider, href) in [
        (
            "https://gitee.com/o/r.git",
            "https://gitee.com/o/r",
            None,
            "https://gitee.com/o/r/blob/{rev}/README.md#L2-3",
        ),
        (
            "https://gitlab.com/g/s/r.git",
            "https://gitlab.com/g/s/r",
            None,
            "https://gitlab.com/g/s/r/-/blob/{rev}/README.md?plain=1#L2-3",
        ),
        (
            "https://git.example.com/g/r.git",
            "https://git.example.com/g/r",
            Some("gitlab"),
            "https://git.example.com/g/r/-/blob/{rev}/README.md?plain=1#L2-3",
        ),
    ] {
        git(&root, &["remote", "set-url", "origin", origin]);
        let mut document = document.clone();
        document["meta"]["repository"] = json!({"url":url,"revision":revision});
        if let Some(provider) = provider {
            document["meta"]["repository"]["provider"] = json!(provider);
        }
        let evidence = observe(&document, Some(&root));
        assert_eq!(
            evidence.references[0].href.as_deref(),
            Some(href.replace("{rev}", &revision).as_str())
        );
    }
    // Without a forge, the repository path (including .git) is compared literally.
    git(
        &root,
        &[
            "remote",
            "set-url",
            "origin",
            "https://git.example.com/g/r.git",
        ],
    );
    document["meta"]["repository"] =
        json!({"url":"https://git.example.com/g/r.git","revision":revision});
    rejects(
        &document,
        "repository/links-unsupported",
        "/meta/repository/url",
    );
    document["meta"]["repository"]["link_mode"] = json!("local-only");
    assert_eq!(
        observe(&document, Some(&root)).references[0].state,
        "verified"
    );
}

#[test]
fn each_unverifiable_condition_is_named_and_never_fails_the_input() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let root = temp.join("repo");
    let at = |path: &str, line: Option<u64>| {
        let mut source = json!({"path":path});
        if let Some(line) = line {
            source["line"] = json!(line);
        }
        cited(DiagramType::Architecture, &revision, json!([source]))
    };
    let runner = GitRunner::default();
    assert_eq!(
        reason(&at("src/lib.rs", Some(1)), None, &runner).as_deref(),
        Some("repo-root-not-provided")
    );
    let missing = GitRunner {
        program: temp.join("no-git").into(),
        ..GitRunner::default()
    };
    assert_eq!(
        reason(&at("src/lib.rs", None), Some(&root), &missing).as_deref(),
        Some("git-unavailable")
    );
    let plain = temp.join("plain");
    fs::create_dir(&plain).unwrap();
    assert_eq!(
        reason(&at("src/lib.rs", None), Some(&plain), &runner).as_deref(),
        Some("repository-unreadable")
    );
    assert_eq!(
        reason(&at("src/lib.rs", None), Some(&root.join("src")), &runner).as_deref(),
        Some("root-not-top-level")
    );
    let mut foreign = at("src/lib.rs", None);
    foreign["meta"]["repository"]["url"] = json!("https://github.com/example/other");
    assert_eq!(
        reason(&foreign, Some(&root), &runner).as_deref(),
        Some("origin-mismatch")
    );
    let mut unknown = at("src/lib.rs", None);
    unknown["meta"]["repository"]["revision"] = json!("b".repeat(40));
    assert_eq!(
        reason(&unknown, Some(&root), &runner).as_deref(),
        Some("revision-unavailable")
    );
    for path in ["absent.rs", "src", "src/nested"] {
        assert_eq!(
            reason(&at(path, None), Some(&root), &runner).as_deref(),
            Some("file-missing"),
            "{path}"
        );
    }
    // CRLF, LF and a final line without a break: four lines.
    assert_eq!(
        reason(&at("src/lib.rs", Some(4)), Some(&root), &runner),
        None
    );
    assert_eq!(
        reason(&at("src/lib.rs", Some(5)), Some(&root), &runner).as_deref(),
        Some("line-out-of-range")
    );
    assert_eq!(
        reason(&at("empty.txt", Some(1)), Some(&root), &runner).as_deref(),
        Some("line-out-of-range")
    );
    assert_eq!(reason(&at("empty.txt", None), Some(&root), &runner), None);
    fs::write(root.join("src/lib.rs"), "changed").unwrap();
    assert_eq!(
        reason(&at("src/lib.rs", Some(1)), Some(&root), &runner).as_deref(),
        Some("worktree-differs")
    );
    let head = observe(&at("src/nested/deep.rs", None), Some(&root));
    assert_eq!(head.status, EvidenceStatus::Partial);
    git(&root, &["commit", "-q", "-am", "next"]);
    let moved = observe(&at("src/nested/deep.rs", None), Some(&root));
    assert_eq!(
        moved.observation.unwrap().head_matches_revision,
        Some(false)
    );
    assert_eq!(
        moved.references[0].state, "verified",
        "an unchanged file still matches the pinned commit"
    );
}

#[test]
fn worktree_paths_are_followed_segment_by_segment_without_links() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let root = temp.join("repo");
    let at = |path: &str| cited(DiagramType::Architecture, &revision, json!([{"path":path}]));
    let runner = GitRunner::default();
    assert_eq!(
        reason(&at("src/nested/deep.rs"), Some(&root), &runner),
        None
    );
    let elsewhere = temp.join("elsewhere");
    fs::rename(root.join("src/nested"), &elsewhere).unwrap();
    symlink(&elsewhere, root.join("src/nested")).unwrap();
    assert_eq!(
        reason(&at("src/nested/deep.rs"), Some(&root), &runner).as_deref(),
        Some("worktree-differs"),
        "intermediate link"
    );
    fs::remove_file(root.join("src/nested")).unwrap();
    fs::rename(&elsewhere, root.join("src/nested")).unwrap();
    let copy = temp.join("deep-copy.rs");
    fs::rename(root.join("src/nested/deep.rs"), &copy).unwrap();
    symlink(&copy, root.join("src/nested/deep.rs")).unwrap();
    assert_eq!(
        reason(&at("src/nested/deep.rs"), Some(&root), &runner).as_deref(),
        Some("worktree-differs"),
        "final link"
    );
    fs::remove_file(root.join("src/nested/deep.rs")).unwrap();
    fs::create_dir(root.join("src/nested/deep.rs")).unwrap();
    assert_eq!(
        reason(&at("src/nested/deep.rs"), Some(&root), &runner).as_deref(),
        Some("worktree-differs"),
        "final directory"
    );
    fs::remove_dir(root.join("src/nested/deep.rs")).unwrap();
    assert_eq!(
        reason(&at("src/nested/deep.rs"), Some(&root), &runner).as_deref(),
        Some("worktree-differs"),
        "final missing"
    );
}

fn wrapper(temp: &Temp, name: &str, body: &str) -> GitRunner {
    let real = String::from_utf8(
        Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let path = temp.join(name);
    fs::write(
        &path,
        format!(
            "#!/bin/sh\nREAL_GIT='{}'\n{body}\nexec \"$REAL_GIT\" \"$@\"\n",
            real.trim()
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    GitRunner {
        program: path.into(),
        ..GitRunner::default()
    }
}

fn objects(root: &Path) -> usize {
    walk(&root.join(".git/objects"))
}
fn walk(path: &Path) -> usize {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| {
                    if entry.path().is_dir() {
                        walk(&entry.path())
                    } else {
                        1
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

#[test]
fn partial_clones_never_fetch_missing_objects_under_any_git_capability() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let source = temp.join("repo");
    git(&source, &["config", "uploadpack.allowFilter", "true"]);
    git(
        &source,
        &["config", "uploadpack.allowAnySHA1InWant", "true"],
    );
    let clone = |name: &str| {
        let target = temp.join(name);
        git(
            &temp.0,
            &[
                "clone",
                "-q",
                "--no-checkout",
                "--filter=blob:none",
                &format!("file://{}", source.display()),
                name,
            ],
        );
        // The promisor stays reachable under another name; origin names the declared repository.
        git(&target, &["remote", "rename", "origin", "source"]);
        git(&target, &["remote", "add", "origin", REPOSITORY]);
        target
    };
    let control = clone("control");
    let blob = git(&source, &["rev-parse", &format!("{revision}:src/lib.rs")]);
    let before = objects(&control);
    git(&control, &["cat-file", "-e", &blob]);
    assert!(
        objects(&control) > before,
        "the fixture can fetch: any lazy fetch would write objects"
    );
    let document = cited(
        DiagramType::Architecture,
        &revision,
        json!([{"path":"src/lib.rs","line":1}]),
    );
    let cases = [
        ("native", GitRunner::default(), "object-unavailable"),
        ("old", wrapper(&temp, "old-git", "case \"$4\" in version) echo 'git version 2.44.0'; exit 0;; esac\nunset GIT_NO_LAZY_FETCH"), "partial-clone-unsafe-git"),
        ("unset", wrapper(&temp, "unset-git", "unset GIT_NO_LAZY_FETCH"), "object-unavailable"),
    ];
    for (name, runner, expected) in cases {
        let target = clone(name);
        let config = git(
            &target,
            &[
                "config",
                "--get-regexp",
                r"^(extensions\.partialclone|remote\..+\.(promisor|partialclonefilter))$",
            ],
        );
        assert!(config.contains("remote.source.promisor"), "{config}");
        let before = objects(&target);
        assert_eq!(
            reason(&document, Some(&target), &runner).as_deref(),
            Some(expected),
            "{name}"
        );
        assert_eq!(objects(&target), before, "{name}: no object was written");
    }
}

#[test]
fn a_git_timeout_ends_the_whole_process_group() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let root = temp.join("repo");
    let pid = temp.join("child.pid");
    let mut runner = wrapper(
        &temp,
        "slow-git",
        &format!(
            "case \"$4\" in cat-file) sleep 30 & echo $! > '{}'; wait;; esac",
            pid.display()
        ),
    );
    runner.timeout = Duration::from_millis(500);
    let document = cited(
        DiagramType::Architecture,
        &revision,
        json!([{"path":"src/lib.rs"}]),
    );
    assert_eq!(
        reason(&document, Some(&root), &runner).as_deref(),
        Some("git-timeout")
    );
    let child: i32 = fs::read_to_string(&pid).unwrap().trim().parse().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert_ne!(
        unsafe { libc::kill(child, 0) },
        0,
        "the grandchild was terminated with its group"
    );
}

#[test]
fn credentials_escapes_and_inconsistent_ranges_fail_the_input_with_pointers() {
    let revision = "a".repeat(40);
    let mut document = cited(
        DiagramType::Architecture,
        &revision,
        json!([{"path":"a.rs"}]),
    );
    document["meta"]["repository"]["url"] =
        json!("https://user:secret-token@github.com/example/repo");
    let found = rejects(&document, "repository/url-invalid", "/meta/repository/url");
    assert_eq!(found["evidence"]["reason"], "credentials");
    assert!(!found.to_string().contains("secret-token"));
    for url in [
        " https://user:secret-token@github.com/example/repo",
        "\thttps://user:secret-token@github.com/example/repo",
        "\u{1}https://user:secret-token@github.com/example/repo",
        "https://user:secret-token@github.com/example/repo\n",
        "git+https://user:secret-token@github.com/example/repo",
        "user:secret-token@github.com/example/repo",
    ] {
        document["meta"]["repository"]["url"] = json!(url);
        let found = rejects(&document, "repository/url-invalid", "/meta/repository/url");
        assert_eq!(found["evidence"]["reason"], "syntax", "{url:?}");
        assert!(!found.to_string().contains("secret-token"), "{url:?}");
    }
    for (url, reason) in [
        ("https://github.com/a/../b", "syntax"),
        ("https://github.com/a/b?x=1", "syntax"),
        ("ssh://root@github.com/a/b", "syntax"),
    ] {
        document["meta"]["repository"]["url"] = json!(url);
        assert_eq!(
            rejects(&document, "repository/url-invalid", "/meta/repository/url")["evidence"]
                ["reason"],
            reason
        );
    }
    document["meta"]["repository"]["url"] = json!(REPOSITORY);
    document["meta"]["repository"]["provider"] = json!("gitee");
    rejects(
        &document,
        "repository/provider-invalid",
        "/meta/repository/provider",
    );
    document["meta"]["repository"]
        .as_object_mut()
        .unwrap()
        .remove("provider");
    for (path, code) in [
        ("/abs.rs", "path-invalid"),
        ("a\\b.rs", "path-invalid"),
        ("a\nb.rs", "path-invalid"),
        ("../x.rs", "path-escape"),
        ("a//b.rs", "path-escape"),
        ("./a.rs", "path-escape"),
        (".git/config", "path-escape"),
    ] {
        document["components"][0]["sources"] = json!([{"path":path}]);
        rejects(
            &document,
            &format!("repository/{code}"),
            "/components/0/sources/0/path",
        );
    }
    document["components"][0]["sources"] = json!([{"path":"a.rs","end_line":2}]);
    rejects(
        &document,
        "repository/line-required",
        "/components/0/sources/0/end_line",
    );
    document["components"][0]["sources"] = json!([{"path":"a.rs","line":3,"end_line":2}]);
    rejects(
        &document,
        "repository/line-range-invalid",
        "/components/0/sources/0",
    );
    document["components"][0]["sources"] = json!([{"path":"a.rs"}]);
    document["meta"]
        .as_object_mut()
        .unwrap()
        .remove("repository");
    rejects(
        &document,
        "repository/repository-required",
        "/meta/repository",
    );
}

#[test]
fn status_counts_and_digest_follow_the_references_and_bind_the_config() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let root = temp.join("repo");
    let none = observe(&document(DiagramType::Architecture), Some(&root));
    assert_eq!((none.status, none.digest()), (EvidenceStatus::None, None));
    let mut declared = document(DiagramType::Architecture);
    declared["meta"]["repository"] = json!({"url":REPOSITORY,"revision":revision});
    let empty = observe(&declared, Some(&root));
    assert_eq!(
        (empty.status, empty.unsourced.clone()),
        (
            EvidenceStatus::Unverified,
            vec!["a".to_owned(), "b".to_owned()]
        )
    );
    let mut document = cited(
        DiagramType::Architecture,
        &revision,
        json!([{"path":"src/lib.rs"},{"path":"absent.rs"}]),
    );
    document["components"][1]["sources"] = json!([{"path":"README.md"}]);
    let partial = observe(&document, Some(&root));
    assert_eq!(partial.status, EvidenceStatus::Partial);
    let receipt = partial.receipt();
    assert_eq!(
        receipt["counts"],
        json!({"references":3,"verified":2,"unverified":1,"unsourced":0})
    );
    assert_eq!(
        receipt["unverified"],
        json!([{"pointer":"/components/0/sources/1","nodeId":"a","reason":"file-missing"}])
    );
    assert_eq!(
        partial.summary(),
        json!({"status":"partial","evidenceSha256":partial.digest()})
    );
    let unverified = observe(&document, None);
    assert_eq!(unverified.status, EvidenceStatus::Unverified);
    assert_eq!(
        observe(&document, Some(&root)),
        partial,
        "same commit and tree give the same evidence"
    );
    let serialized = serde_json::to_string(&partial).unwrap();
    assert!(
        !serialized.contains(&root.display().to_string()),
        "no local path is bound"
    );
    let mut first = input(DiagramType::Architecture, &document);
    let unbound = first.config_sha256.clone();
    first.bind_evidence(partial.digest().as_deref());
    assert_ne!(first.config_sha256, unbound);
    let mut second = input(DiagramType::Architecture, &document);
    second.bind_evidence(observe(&document, Some(&root)).digest().as_deref());
    assert_eq!(first.config_sha256, second.config_sha256);
    second.bind_evidence(unverified.digest().as_deref());
    assert_ne!(first.config_sha256, second.config_sha256);
}

fn engine() -> EngineIdentity {
    EngineIdentity {
        source_input_sha256: sha256(b"evidence test engine"),
        input_contract_schema: 1,
        receipt_schema: 1,
        viewer_revision: "phoenix-v2".into(),
        binary: FileIdentity::of(&fs::read(std::env::current_exe().unwrap()).unwrap()),
    }
}

#[test]
fn delivery_binds_evidence_and_strict_currency_rejects_tampering() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let root = temp.join("repo");
    let mut document = cited(
        DiagramType::Architecture,
        &revision,
        json!([{"path":"src/lib.rs","line":2}]),
    );
    document["components"][0]["pos"] = json!([80, 120]);
    document["components"][1]["pos"] = json!([380, 120]);
    document["meta"]["legend"] = json!({"mode":"hidden"});
    document["meta"]["quality_profile"] = json!("standard");
    document["meta"]["output"] = json!("report.html");
    let candidate = temp.join("candidate.json");
    fs::write(&candidate, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    let (options, engine) = (CompileOptions::default(), engine());
    let assets = EmbeddedAssets::builtin();
    let request = |repo_root| CompileRequest {
        kind: DiagramType::Architecture,
        candidate: &candidate,
        options: &options,
        engine: &engine,
        assets: &assets,
        repo_root,
        brand_cache: None,
    };
    let output = temp.join("report.html");
    let delivered = delivery::deliver(&request(Some(&root)), &output);
    assert_eq!(
        delivered.status,
        Status::Pass,
        "{:?}",
        delivered.diagnostics
    );
    assert_eq!(delivered.evidence["status"], "partial");
    assert_eq!(delivered.evidence["counts"]["verified"], 1);
    let html = fs::read_to_string(&output).unwrap();
    assert!(html.contains("\"sourceEvidence\":{\"schemaVersion\":1,\"status\":\"partial\""));
    assert_eq!(delivery::check(&output, true, &engine).status, Status::Pass);
    let unchecked = temp.join("unchecked.html");
    let without = delivery::deliver(&request(None), &unchecked);
    assert_eq!(without.status, Status::Pass);
    assert_eq!(without.evidence["status"], "unverified");
    assert_ne!(
        without.config_sha256, delivered.config_sha256,
        "the evidence digest is part of the config"
    );
    let sidecar = delivery::sidecar_paths(&output, None).delivery;
    let mut receipt: Value = serde_json::from_slice(&fs::read(&sidecar).unwrap()).unwrap();
    receipt["evidence"]["evidenceSha256"] = without.evidence["evidenceSha256"].clone();
    fs::write(&sidecar, serde_json::to_vec(&receipt).unwrap()).unwrap();
    let tampered = delivery::check(&output, true, &engine);
    assert!(
        tampered
            .diagnostics
            .iter()
            .any(|d| d.code == "provenance/evidence-mismatch"),
        "{:?}",
        tampered.diagnostics
    );
}

#[test]
fn embedded_evidence_must_cite_exactly_the_candidate_sources() {
    let (temp, revision) = repository("https://github.com/example/repo.git");
    let root = temp.join("repo");
    let document = cited(
        DiagramType::Architecture,
        &revision,
        json!([{"path":"src/lib.rs","line":1}]),
    );
    let parsed = input(DiagramType::Architecture, &document);
    let evidence = verify(&parsed, Some(&root)).unwrap();
    let metadata = |evidence: &SourceEvidence| json!({"sourceEvidence":evidence});
    assert_eq!(
        phoenix_diagram::evidence::bound(&parsed, &metadata(&evidence)).unwrap(),
        evidence
    );
    let mut moved = evidence.clone();
    moved.references[0].line = Some(2);
    let mut silent = evidence.clone();
    silent.references[0].state = "unverified".into();
    let mut promoted = evidence.clone();
    promoted.status = EvidenceStatus::Verified;
    let mut hidden = evidence.clone();
    hidden.unsourced.clear();
    for tampered in [moved, silent, promoted, hidden] {
        assert!(
            phoenix_diagram::evidence::bound(&parsed, &metadata(&tampered)).is_err(),
            "{tampered:?}"
        );
    }
    let plain = input(
        DiagramType::Architecture,
        &self::document(DiagramType::Architecture),
    );
    assert!(phoenix_diagram::evidence::bound(&plain, &metadata(&SourceEvidence::none())).is_ok());
    assert!(phoenix_diagram::evidence::bound(&plain, &metadata(&evidence)).is_err());
}
