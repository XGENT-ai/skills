use phoenix_ui_review::target::{freeze, ExternalSource, TargetSnapshot};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const TARGET: &str = "# Review\n\n## 单元\nU1\n\n## 规范要点\nKeep the accepted design.\n\n## 采集\nOpen the page.\n\n## 变更记录\nInitial.\n";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "phoenix-target-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("project")).unwrap();
        Self(path)
    }
    fn root(&self) -> PathBuf {
        self.0.join("project")
    }
    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        let path = self.root().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn target(&self, text: &str) -> TargetSnapshot {
        self.write("review.ui-target.md", text);
        freeze(&self.root().join("review.ui-target.md"), &self.root(), &[]).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn capture_changes_do_not_change_expectations_but_raw_source_hash_changes() {
    let f = Fixture::new();
    let first = f.target(TARGET);
    let next = f.target(
        &TARGET
            .replace("Open the page.", "Open a new page.")
            .replace("Initial.", "More changes."),
    );
    assert_eq!(first.fingerprint, next.fingerprint);
    assert_ne!(first.source_hash, next.source_hash);
    assert_eq!(first.algorithm, "phoenix-ui-target-v1");
    assert_eq!(
        first.fingerprint,
        "dcd090e55da0cd7540590e4f4867b698daec6aef42e986f548eb8e4239295bca"
    );
    assert!(first.missing.is_empty());
    assert_eq!(first.sections.len(), 2);
}

#[test]
fn lf_normalization_keeps_every_other_byte_and_section_order() {
    let f = Fixture::new();
    let first = f.target(TARGET);
    assert_eq!(
        first.fingerprint,
        f.target(&TARGET.replace('\n', "\r\n")).fingerprint
    );
    assert_ne!(
        first.fingerprint,
        f.target(&TARGET.replace("U1\n", "U1  \n")).fingerprint
    );
    let swapped =
        "## 规范要点\nKeep the accepted design.\n\n## 单元\nU1\n\n## 采集\nOpen the page.\n";
    assert_ne!(first.fingerprint, f.target(swapped).fingerprint);
    assert!(first.sections[0].text.starts_with("## 单元\n"));
}

#[test]
fn ast_ignores_quoted_and_fenced_fake_headings_and_links() {
    let f = Fixture::new();
    let target = TARGET.replace(
        "U1\n",
        "U1\n```md\n## 单元\n![fake](not-a-file.png)\n```\n> ## 单元\n> quoted\n",
    );
    let result = f.target(&target);
    assert_eq!(result.sections.len(), 2);
    assert!(result.attachments.is_empty());
    assert!(result.missing.is_empty());
}

#[test]
fn duplicate_real_sections_are_input_errors_and_missing_sections_are_explicit() {
    let f = Fixture::new();
    f.write("target.md", format!("{TARGET}\n## 单元\nAgain\n"));
    assert_eq!(
        freeze(&f.root().join("target.md"), &f.root(), &[])
            .unwrap_err()
            .code,
        2
    );
    let missing = f.target("## 单元\nU1\n");
    assert!(missing.missing.iter().any(|m| m.contains("规范要点")));
}

#[test]
fn replacing_an_attachment_at_the_same_path_changes_the_fingerprint() {
    let f = Fixture::new();
    f.write("design.png", b"first");
    let text = TARGET.replace("U1\n", "U1 ![target](design.png)\n");
    let first = f.target(&text);
    f.write("design.png", b"second");
    let second = f.target(&text);
    assert_ne!(first.fingerprint, second.fingerprint);
    assert_ne!(first.attachments[0].sha256, second.attachments[0].sha256);
    assert_eq!(
        second.attachments[0].bytes.as_deref(),
        Some(b"second".as_slice())
    );
}

#[test]
fn references_resolve_globally_and_logical_aliases_deduplicate_and_sort() {
    let f = Fixture::new();
    f.write("a.md", b"a");
    f.write("z.png", b"z");
    let text = TARGET.replace(
        "U1\n",
        "U1 ![target][PIC] [token](./a.md#rule) [again](a.md)\n",
    ) + "\n[pic]: z.png\n";
    let result = f.target(&text);
    assert_eq!(
        result
            .attachments
            .iter()
            .map(|a| a.logical_path.as_str())
            .collect::<Vec<_>>(),
        ["a.md", "z.png"]
    );
    assert!(result.missing.is_empty());
}

#[test]
fn file_fragments_hash_the_whole_attachment_without_recursive_links() {
    let f = Fixture::new();
    f.write("DESIGN.md", "# Rule\nKeep it\n[other](ignored.md)\n");
    let text = TARGET.replace("U1\n", "U1 [rule](DESIGN.md#rule)\n");
    let first = f.target(&text);
    assert_eq!(first.attachments.len(), 1);
    assert!(first.missing.is_empty());
    f.write(
        "DESIGN.md",
        "# Rule\nKeep it\n[other](ignored.md)\n# Unused\nChanged\n",
    );
    assert_ne!(first.fingerprint, f.target(&text).fingerprint);
}

#[test]
fn only_expectation_links_count_and_plain_web_sources_need_no_export() {
    let f = Fixture::new();
    let text = TARGET
        .replace("U1\n", "U1 [source](https://example.com/design)\n")
        .replace("Open the page.", "![capture](not-yet.png)");
    let result = f.target(&text);
    assert!(result.attachments.is_empty());
    assert!(result.missing.is_empty());
}

#[test]
fn missing_local_files_remote_images_and_broken_references_are_missing_evidence() {
    let f = Fixture::new();
    let text = TARGET.replace(
        "U1\n",
        "U1 ![missing](no.png) ![remote](https://example.com/image.png) ![broken][unknown]\n",
    );
    let result = f.target(&text);
    assert_eq!(result.missing.len(), 3);
    assert_eq!(result.attachments.len(), 1);
    assert!(result.attachments[0].sha256.is_none());
    f.write("no.png", b"new");
    assert_ne!(result.fingerprint, f.target(&text).fingerprint);
}

#[test]
fn encoded_paths_and_file_urls_preserve_spaces_and_reserved_characters() {
    let f = Fixture::new();
    f.write("design space #.md", b"rule");
    let url = url::Url::from_file_path(f.root().join("design space #.md")).unwrap();
    let text = TARGET.replace(
        "U1\n",
        &format!("U1 [first](design%20space%20%23.md) [same]({url}#rule)\n"),
    );
    let result = f.target(&text);
    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].logical_path, "design space #.md");
    assert!(result.missing.is_empty());
}

#[test]
fn equivalent_projects_have_identical_fingerprints() {
    let a = Fixture::new();
    let b = Fixture::new();
    a.write("design.md", b"same");
    b.write("design.md", b"same");
    let text = TARGET.replace("U1\n", "U1 [design](design.md)\n");
    assert_eq!(a.target(&text).fingerprint, b.target(&text).fingerprint);
}

#[test]
fn external_attachments_require_explicit_logical_source_registration() {
    let f = Fixture::new();
    fs::write(f.0.join("outside.md"), b"accepted external source").unwrap();
    let text = TARGET.replace("U1\n", "U1 [external](../outside.md)\n");
    f.write("target.md", &text);
    assert_eq!(
        freeze(&f.root().join("target.md"), &f.root(), &[])
            .unwrap_err()
            .code,
        2
    );
    let external = [ExternalSource {
        path: f.0.join("outside.md"),
        logical_path: "external/design.md".into(),
    }];
    let result = freeze(&f.root().join("target.md"), &f.root(), &external).unwrap();
    assert_eq!(result.attachments[0].logical_path, "external/design.md");
    assert_eq!(
        result.attachments[0].bytes.as_deref(),
        Some(b"accepted external source".as_slice())
    );
}

#[test]
fn external_logical_names_cannot_escape_or_alias_project_files() {
    let f = Fixture::new();
    fs::write(f.0.join("outside.md"), b"source").unwrap();
    f.write("target.md", TARGET);
    for logical in ["../escape", "/absolute", ""] {
        let sources = [ExternalSource {
            path: f.0.join("outside.md"),
            logical_path: logical.into(),
        }];
        assert_eq!(
            freeze(&f.root().join("target.md"), &f.root(), &sources)
                .unwrap_err()
                .code,
            2
        );
    }
    let sources = [
        ExternalSource {
            path: f.0.join("outside.md"),
            logical_path: "same.md".into(),
        },
        ExternalSource {
            path: f.0.join("outside.md"),
            logical_path: "same.md".into(),
        },
    ];
    assert_eq!(
        freeze(&f.root().join("target.md"), &f.root(), &sources)
            .unwrap_err()
            .code,
        2
    );
    f.write("same.md", b"project source");
    assert_eq!(
        freeze(&f.root().join("target.md"), &f.root(), &sources[..1])
            .unwrap_err()
            .code,
        2
    );
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_borrow_an_unregistered_outside_source() {
    let f = Fixture::new();
    fs::write(f.0.join("outside.md"), b"outside").unwrap();
    std::os::unix::fs::symlink(f.0.join("outside.md"), f.root().join("linked.md")).unwrap();
    f.write(
        "target.md",
        TARGET.replace("U1\n", "U1 [design](linked.md)\n"),
    );
    assert_eq!(
        freeze(&f.root().join("target.md"), &f.root(), &[])
            .unwrap_err()
            .code,
        2
    );
}

#[test]
fn outside_targets_and_invalid_text_are_rejected_without_writes() {
    let f = Fixture::new();
    fs::write(f.0.join("outside.md"), TARGET).unwrap();
    assert_eq!(
        freeze(&f.0.join("outside.md"), &f.root(), &[])
            .unwrap_err()
            .code,
        2
    );
    f.write("invalid.md", b"\xff");
    assert_eq!(
        freeze(&f.root().join("invalid.md"), &f.root(), &[])
            .unwrap_err()
            .code,
        2
    );
}

#[test]
fn reference_definitions_bind_web_sources_and_fragments_without_hashing_unused_definitions() {
    let f = Fixture::new();
    f.write("design.md", b"same complete file");
    let text = TARGET.replace("U1\n", "U1 [design][rule] [source][web]\n")
        + "\n[rule]: design.md#first\n[web]: https://example.com/one\n[unused]: no.png\n";
    let first = f.target(&text);
    assert_ne!(
        first.fingerprint,
        f.target(&text.replace("design.md#first", "design.md#second"))
            .fingerprint
    );
    assert_ne!(
        first.fingerprint,
        f.target(&text.replace("example.com/one", "example.com/two"))
            .fingerprint
    );
    assert_eq!(
        first.fingerprint,
        f.target(&text.replace("no.png", "other.png")).fingerprint
    );
    assert!(first.missing.is_empty());
    assert_eq!(first.sections[0].references.len(), 2);
}

#[test]
fn ordinary_brackets_are_text_but_broken_shortcut_images_are_missing() {
    let f = Fixture::new();
    let text = TARGET.replace("U1\n", "U1 [clarification] ![unavailable]\n");
    let result = f.target(&text);
    assert_eq!(result.missing.len(), 1);
    assert!(result.missing[0].contains("unavailable"));
}

#[test]
fn missing_encoded_file_url_stays_inside_its_real_project_ancestor() {
    let f = Fixture::new();
    let url = url::Url::from_file_path(f.root().join("not-created/target space.png")).unwrap();
    let result = f.target(&TARGET.replace("U1\n", &format!("U1 ![target]({url})\n")));
    assert_eq!(
        result.attachments[0].logical_path,
        "not-created/target space.png"
    );
    assert_eq!(result.missing.len(), 1);
}

#[test]
fn a_new_top_level_heading_ends_an_expectation_section() {
    let f = Fixture::new();
    let text = "## 单元\nU1\n## 规范要点\nKeep it\n# Notes\n![unused](not-a-target.png)\n";
    let result = f.target(text);
    assert!(result.attachments.is_empty());
    assert!(result.missing.is_empty());
    assert!(!result.sections[1].text.contains("Notes"));
}
