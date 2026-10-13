//! Read-only classification of fixed, published project-local migration artifacts.
//! Names and command markers may flag a conflict; ownership requires a complete
//! supported file digest or complete supported handler structure.
use crate::{disk, json, sha256, Error, ErrorKind, Identity, JsonSlot, JsonStep, Project, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Source {
    pub package_version: String,
    pub channel: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    File,
    Pin,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyFile {
    pub path: String,
    pub provider: String,
    pub sha256: String,
    pub selected: bool,
    pub kind: EntryKind,
    pub sources: Vec<Source>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegacyHandler {
    pub provider: String,
    pub selected: bool,
    pub parent: Vec<JsonStep>,
    pub slot: JsonSlot,
    pub value: Value,
    pub sources: Vec<Source>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharedFile {
    pub path: String,
    pub sha256: String,
    pub handlers: Vec<LegacyHandler>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConflictKind {
    CustomizedFile,
    UnknownLegacyEntry,
    CustomizedHandler,
    AmbiguousHandler,
    OutsideSelection,
    InvalidJson,
    Symlink,
    LiveResidue,
    Unreadable,
    /// A personal (user-level) old entry that would run beside Phoenix.
    PersonalEntry,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conflict {
    pub path: String,
    pub provider: Option<String>,
    pub kind: ConflictKind,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preserved {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub exclusive: Vec<LegacyFile>,
    pub shared: Vec<SharedFile>,
    pub conflicts: Vec<Conflict>,
    pub preserved: Vec<Preserved>,
}

/// Classify all fixed project provider paths, including active legacy entries
/// outside the caller's migration selection. No write or process lookup occurs.
pub fn classify_project(
    root: impl AsRef<Path>,
    migration_providers: &[String],
) -> Result<Inventory> {
    classify_project_with(root, migration_providers, &BTreeMap::new())
}

/// `owned` maps exclusive paths of a verified active receipt to their recorded
/// hashes. A file there with the same bytes is Phoenix's, not a legacy entry.
pub fn classify_project_with(
    root: impl AsRef<Path>,
    migration_providers: &[String],
    owned: &BTreeMap<String, String>,
) -> Result<Inventory> {
    let catalog = Catalog::fixed()?;
    let selected: BTreeSet<_> = migration_providers.iter().cloned().collect();
    if selected.is_empty() || selected.iter().any(|p| !catalog.providers.contains(p)) {
        return Err(Error::new(
            ErrorKind::InvalidPlan,
            "migration selection requires known project providers",
        ));
    }
    let project = Project::open(root)?;
    let root = project.root();
    let mut result = Inventory::default();
    for (path, rule) in &catalog.files {
        let Some(file) = read(root, path, Some(&rule.provider), &mut result) else {
            continue;
        };
        if owned.get(path) == Some(&file.state.hash) {
            continue;
        }
        if let Some(sources) = rule.hashes.get(&file.state.hash) {
            result.exclusive.push(LegacyFile {
                path: path.clone(),
                provider: rule.provider.clone(),
                sha256: file.state.hash,
                selected: selected.contains(&rule.provider),
                kind: EntryKind::File,
                sources: sources.iter().cloned().collect(),
            });
        } else {
            conflict(
                &mut result,
                path,
                Some(&rule.provider),
                ConflictKind::CustomizedFile,
                "file bytes do not match any fixed published source; retain and resolve manually",
            );
        }
    }
    for provider in &catalog.providers {
        let old_skill = format!("{provider}/skills/impeccable");
        inspect_old_tree(
            root,
            &old_skill,
            provider,
            &catalog.files,
            owned,
            &mut result,
        );
        inspect_shortcuts(root, provider, &catalog, &selected, &mut result);
    }
    for (path, patterns) in &catalog.hooks {
        let Some(file) = read(root, path, None, &mut result) else {
            continue;
        };
        let value = match json::parse_value(&file.bytes) {
            Ok(value) => value,
            Err(_) => {
                conflict(&mut result, path, None, ConflictKind::InvalidJson, "shared JSON/JSONC is invalid or cannot be compared exactly; retain original bytes");
                continue;
            }
        };
        let handlers = classify_hooks(path, &value, patterns, &selected, &mut result);
        result.shared.push(SharedFile {
            path: path.clone(),
            sha256: file.state.hash,
            handlers,
        });
        result.preserved.push(Preserved {
            path: path.clone(),
            reason: "unowned members, matcher fields and neighboring handlers remain unchanged"
                .into(),
        });
    }
    inspect_live(root, &mut result);
    for path in [
        "AGENTS.md",
        "CLAUDE.md",
        "DESIGN.md",
        "PRODUCT.md",
        ".impeccable/config.json",
        ".impeccable/config.local.json",
        ".impeccable/design.json",
        ".impeccable/brief.json",
        ".impeccable/ui",
        ".claude/hooks",
    ] {
        if disk::checked(root, path).is_ok_and(|full| fs::symlink_metadata(full).is_ok()) {
            result.preserved.push(Preserved { path: path.into(), reason: "project documents, old data and independent XGENT functionality are outside entry retirement".into() });
        }
    }
    let outside: BTreeMap<_, _> = result
        .exclusive
        .iter()
        .filter(|f| !f.selected)
        .map(|f| (f.provider.clone(), f.path.clone()))
        .chain(result.shared.iter().flat_map(|s| {
            s.handlers
                .iter()
                .filter(|h| !h.selected)
                .map(move |h| (h.provider.clone(), s.path.clone()))
        }))
        .collect();
    for (provider, path) in outside {
        conflict(&mut result, &path, Some(&provider), ConflictKind::OutsideSelection, "active legacy entry is outside the migration selection; expand the selection or disable it before activation");
    }
    result.exclusive.sort_by(|a, b| a.path.cmp(&b.path));
    result
        .conflicts
        .sort_by(|a, b| (&a.path, &a.message).cmp(&(&b.path, &b.message)));
    result.conflicts.dedup_by(|a, b| {
        a.path == b.path && a.kind == b.kind && a.provider == b.provider && a.message == b.message
    });
    result.preserved.sort_by(|a, b| a.path.cmp(&b.path));
    result.preserved.dedup_by(|a, b| a.path == b.path);
    Ok(result)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestSource {
    schema_version: u32,
    package_version: String,
    providers: BTreeMap<String, Provider>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Provider {
    files: BTreeMap<String, String>,
    hook_artifact: Option<Artifact>,
    hooks: Vec<Hook>,
}
#[derive(Deserialize)]
struct Artifact {
    source: String,
    destination: String,
    shared: Option<String>,
}
#[derive(Deserialize)]
struct Hook {
    shape: String,
    value: Value,
}
struct FileRule {
    provider: String,
    hashes: BTreeMap<String, BTreeSet<Source>>,
}
struct Pattern {
    provider: String,
    event: String,
    identity: Option<Identity>,
    value: Value,
    sources: BTreeSet<Source>,
}
struct Catalog {
    providers: BTreeSet<String>,
    files: BTreeMap<String, FileRule>,
    hooks: BTreeMap<String, Vec<Pattern>>,
    pins: BTreeMap<String, String>,
}

// Published bundle contains this Gemini file although M1's installer-derived
// hookArtifact/hooks metadata omits it. Keep the full verified blob and compare
// handlers exactly; the shared file is never retired as an exclusive payload.
const GEMINI_BUNDLE: &str = "{\n  \"hooks\": {\n    \"BeforeTool\": [\n      {\n        \"matcher\": \"^run_shell_command$\",\n        \"hooks\": [\n          {\n            \"name\": \"impeccable-session\",\n            \"type\": \"command\",\n            \"command\": \"[ ! -f $GEMINI_PROJECT_DIR/.gemini/skills/impeccable/scripts/impeccable ] || $GEMINI_PROJECT_DIR/.gemini/skills/impeccable/scripts/impeccable hook\",\n            \"timeout\": 5000\n          }\n        ]\n      }\n    ],\n    \"AfterAgent\": [\n      {\n        \"hooks\": [\n          {\n            \"name\": \"impeccable-completion\",\n            \"type\": \"command\",\n            \"command\": \"[ ! -f $GEMINI_PROJECT_DIR/.gemini/skills/impeccable/scripts/impeccable ] || $GEMINI_PROJECT_DIR/.gemini/skills/impeccable/scripts/impeccable hook\",\n            \"timeout\": 30000\n          }\n        ]\n      }\n    ]\n  }\n}\n";

impl Catalog {
    fn fixed() -> Result<Self> {
        let mut catalog = Self {
            providers: BTreeSet::new(),
            files: BTreeMap::new(),
            hooks: BTreeMap::new(),
            pins: BTreeMap::new(),
        };
        for text in [
            include_str!("../../../migration-sources/0.3.0.json"),
            include_str!("../../../migration-sources/0.4.0.json"),
            include_str!("../../../migration-sources/0.5.0.json"),
            include_str!("../../../migration-sources/0.6.0.json"),
        ] {
            let source: ManifestSource = serde_json::from_str(text)
                .map_err(|e| Error::new(ErrorKind::InvalidPlan, e.to_string()))?;
            if source.schema_version != 1 {
                return Err(Error::new(
                    ErrorKind::InvalidPlan,
                    "unsupported fixed migration schema",
                ));
            }
            for (provider, entry) in source.providers {
                catalog.providers.insert(provider.clone());
                for (relative, hash) in &entry.files {
                    if relative.starts_with("skills/impeccable/")
                        || relative.starts_with("agents/impeccable-")
                        || relative == "commands/impeccable.md"
                    {
                        let path = format!("{provider}/{relative}");
                        let rule = catalog.files.entry(path).or_insert_with(|| FileRule {
                            provider: provider.clone(),
                            hashes: BTreeMap::new(),
                        });
                        rule.hashes.entry(hash.clone()).or_default().insert(Source {
                            package_version: source.package_version.clone(),
                            channel: "published-file".into(),
                        });
                    }
                }
                if let Some(artifact) = &entry.hook_artifact {
                    for hook in &entry.hooks {
                        // M1's .agents bundle hook is a reference to .codex's
                        // artifact, not an installed .agents bundle file.
                        if hook.shape == "bundle"
                            && !artifact.source.starts_with(&format!("{provider}/"))
                        {
                            continue;
                        }
                        let mut paths = BTreeSet::from([if hook.shape == "bundle" {
                            artifact.source.clone()
                        } else {
                            artifact.destination.clone()
                        }]);
                        if let Some(shared) = &artifact.shared {
                            paths.insert(shared.clone());
                        }
                        for path in paths {
                            catalog.add_hooks(
                                &path,
                                &provider,
                                &hook.value,
                                &source.package_version,
                                &hook.shape,
                                true,
                            )?;
                        }
                    }
                }
                if provider == ".gemini" {
                    if let Some(hash) = entry.files.get("settings.json") {
                        if sha256(GEMINI_BUNDLE.as_bytes()) != *hash {
                            return Err(Error::new(
                                ErrorKind::InvalidPlan,
                                "fixed Gemini bundle bytes differ from M1 blob digest",
                            ));
                        }
                        let value: Value = serde_json::from_str(GEMINI_BUNDLE)
                            .map_err(|e| Error::new(ErrorKind::InvalidPlan, e.to_string()))?;
                        catalog.add_hooks(
                            ".gemini/settings.json",
                            &provider,
                            &value,
                            &source.package_version,
                            "bundle",
                            false,
                        )?;
                    }
                }
            }
        }
        let pins: Value = serde_json::from_str(include_str!(
            "../../../../../test/phoenix-ui/fixtures/migration/pins.json"
        ))
        .map_err(|e| Error::new(ErrorKind::InvalidPlan, e.to_string()))?;
        for value in pins["templates"]
            .as_object()
            .ok_or_else(|| Error::new(ErrorKind::InvalidPlan, "fixed pin templates missing"))?
            .values()
        {
            let raw = value.as_str().ok_or_else(|| {
                Error::new(ErrorKind::InvalidPlan, "fixed pin template is not text")
            })?;
            catalog.pins.insert(
                sha256(raw.as_bytes()),
                pins["sourceCommit"].as_str().unwrap_or_default().into(),
            );
        }
        Ok(catalog)
    }

    fn add_hooks(
        &mut self,
        path: &str,
        provider: &str,
        value: &Value,
        version: &str,
        channel: &str,
        legacy_js: bool,
    ) -> Result<()> {
        let events = value["hooks"]
            .as_object()
            .ok_or_else(|| Error::new(ErrorKind::InvalidPlan, "fixed hooks missing"))?;
        for (event, entries) in events {
            for outer in entries.as_array().ok_or_else(|| {
                Error::new(ErrorKind::InvalidPlan, "fixed hook event is not array")
            })? {
                let (identity, values) = if let Some(hooks) = outer.get("hooks") {
                    let mut fields: BTreeMap<_, _> = outer
                        .as_object()
                        .ok_or_else(|| {
                            Error::new(ErrorKind::InvalidPlan, "fixed matcher is not object")
                        })?
                        .iter()
                        .filter(|(key, _)| key.as_str() != "hooks")
                        .map(|(key, value)| (key.clone(), Some(value.clone())))
                        .collect();
                    fields.entry("matcher".into()).or_insert(None);
                    (
                        Some(Identity {
                            fields,
                            required_keys: vec!["hooks".into()],
                        }),
                        hooks
                            .as_array()
                            .ok_or_else(|| {
                                Error::new(ErrorKind::InvalidPlan, "fixed nested hook is not array")
                            })?
                            .clone(),
                    )
                } else {
                    (None, vec![outer.clone()])
                };
                for handler in values {
                    self.add_pattern(
                        path,
                        provider,
                        event,
                        identity.clone(),
                        handler.clone(),
                        Source {
                            package_version: version.into(),
                            channel: channel.into(),
                        },
                    );
                    if legacy_js {
                        let mut old = handler;
                        for key in ["command", "commandWindows", "bash", "powershell"] {
                            if let Some(command) = old.get(key).and_then(Value::as_str) {
                                let actual_provider = if command.contains(".agents/skills/") {
                                    ".agents"
                                } else {
                                    path.split('/').next().unwrap_or(provider)
                                };
                                let prefix = match actual_provider {
                                    ".claude" => "${CLAUDE_PROJECT_DIR}/",
                                    ".github" => "$(git rev-parse --show-toplevel)/",
                                    _ => "",
                                };
                                let script = match event.as_str() {
                                    "SessionStart" => "hook-probe.mjs",
                                    "Stop" => "hook-stop.mjs",
                                    "preToolUse" => "hook-before-edit.mjs",
                                    "PostToolUse" => "hook-after-edit.mjs",
                                    _ => "hook.mjs",
                                };
                                old[key] = Value::String(format!("node \"{prefix}{actual_provider}/skills/impeccable/scripts/{script}\""));
                            }
                        }
                        self.add_pattern(
                            path,
                            provider,
                            event,
                            identity.clone(),
                            old,
                            Source {
                                package_version: version.into(),
                                channel: "legacy-js-fixture".into(),
                            },
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn add_pattern(
        &mut self,
        path: &str,
        provider: &str,
        event: &str,
        identity: Option<Identity>,
        value: Value,
        source: Source,
    ) {
        let patterns = self.hooks.entry(path.into()).or_default();
        if let Some(pattern) = patterns.iter_mut().find(|p| {
            p.provider == provider && p.event == event && p.identity == identity && p.value == value
        }) {
            pattern.sources.insert(source);
        } else {
            patterns.push(Pattern {
                provider: provider.into(),
                event: event.into(),
                identity,
                value,
                sources: BTreeSet::from([source]),
            });
        }
    }
}

fn conflict(
    result: &mut Inventory,
    path: &str,
    provider: Option<&str>,
    kind: ConflictKind,
    message: &str,
) {
    result.conflicts.push(Conflict {
        path: path.into(),
        provider: provider.map(str::to_string),
        kind,
        message: message.into(),
    });
}

fn read(
    root: &Path,
    path: &str,
    provider: Option<&str>,
    result: &mut Inventory,
) -> Option<disk::Snapshot> {
    // Reject special files before opening: read-only classification must not
    // block on a FIFO or access a device through a legacy artifact path.
    match disk::checked(root, path).and_then(|full| match fs::symlink_metadata(&full) {
        Ok(meta) if !meta.is_file() => Err(Error::conflict(format!(
            "not a regular file: {}",
            full.display()
        ))),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(&full, e)),
    }) {
        Ok(()) => {}
        Err(error) => {
            read_error(root, path, provider, &error, result);
            return None;
        }
    }
    match disk::read(root, path) {
        Ok(value) => value,
        Err(error) => {
            read_error(root, path, provider, &error, result);
            None
        }
    }
}

fn read_error(
    root: &Path,
    path: &str,
    provider: Option<&str>,
    error: &Error,
    result: &mut Inventory,
) {
    let mut prefix = String::new();
    for part in path.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(part);
        if fs::symlink_metadata(root.join(&prefix)).is_ok_and(|m| m.file_type().is_symlink()) {
            conflict(
                result,
                &prefix,
                provider,
                ConflictKind::Symlink,
                "project entry is a symlink; ownership cannot follow it or retire its target",
            );
            return;
        }
    }
    conflict(
        result,
        path,
        provider,
        ConflictKind::Unreadable,
        &error.message,
    );
}

fn inspect_old_tree(
    root: &Path,
    path: &str,
    provider: &str,
    rules: &BTreeMap<String, FileRule>,
    owned: &BTreeMap<String, String>,
    result: &mut Inventory,
) {
    let full = match disk::checked(root, path) {
        Ok(full) => full,
        Err(error) => {
            read_error(root, path, Some(provider), &error, result);
            return;
        }
    };
    let meta = match fs::symlink_metadata(&full) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            read_error(root, path, Some(provider), &Error::io(&full, e), result);
            return;
        }
    };
    if !meta.is_dir() {
        if !rules.contains_key(path) && !owned.contains_key(path) {
            conflict(
                result,
                path,
                Some(provider),
                ConflictKind::UnknownLegacyEntry,
                "unrecognized entry in the old skill; keep it and resolve manually",
            );
        }
        return;
    }
    let entries = match fs::read_dir(&full) {
        Ok(entries) => entries,
        Err(e) => {
            read_error(root, path, Some(provider), &Error::io(&full, e), result);
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                read_error(root, path, Some(provider), &Error::io(&full, e), result);
                continue;
            }
        };
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            conflict(
                result,
                path,
                Some(provider),
                ConflictKind::UnknownLegacyEntry,
                "non-UTF-8 old skill entry requires manual handling",
            );
            continue;
        };
        inspect_old_tree(
            root,
            &format!("{path}/{name}"),
            provider,
            rules,
            owned,
            result,
        );
    }
}

fn inspect_shortcuts(
    root: &Path,
    provider: &str,
    catalog: &Catalog,
    selected: &BTreeSet<String>,
    result: &mut Inventory,
) {
    for directory in [
        format!("{provider}/skills"),
        format!("{provider}/commands"),
        format!("{provider}/agents"),
    ] {
        let full = match disk::checked(root, &directory) {
            Ok(full) => full,
            Err(error) => {
                read_error(root, &directory, Some(provider), &error, result);
                continue;
            }
        };
        let entries = match fs::read_dir(&full) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                read_error(
                    root,
                    &directory,
                    Some(provider),
                    &Error::io(&full, e),
                    result,
                );
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    read_error(
                        root,
                        &directory,
                        Some(provider),
                        &Error::io(&full, e),
                        result,
                    );
                    continue;
                }
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{directory}/{name}");
            if (directory.ends_with("/skills") && name == "impeccable")
                || catalog.files.contains_key(&path)
            {
                continue;
            }
            let meta = match fs::symlink_metadata(entry.path()) {
                Ok(meta) => meta,
                Err(e) => {
                    read_error(
                        root,
                        &path,
                        Some(provider),
                        &Error::io(&entry.path(), e),
                        result,
                    );
                    continue;
                }
            };
            if meta.file_type().is_symlink() {
                if name.starts_with("impeccable") {
                    conflict(result, &path, Some(provider), ConflictKind::Symlink, "legacy-looking shortcut is a symlink; keep its target and resolve the project entry manually");
                } else {
                    result.preserved.push(Preserved { path, reason: "unowned symlink; external copies and auxiliary skill discovery remain caller responsibilities".into() });
                }
                continue;
            }
            let pin = if directory.ends_with("/skills") && meta.is_dir() {
                format!("{path}/SKILL.md")
            } else if meta.is_file() {
                path.clone()
            } else {
                result.preserved.push(Preserved {
                    path,
                    reason: "unowned project entry".into(),
                });
                continue;
            };
            let Some(file) = read(root, &pin, Some(provider), result) else {
                result.preserved.push(Preserved {
                    path,
                    reason: "unowned project entry".into(),
                });
                continue;
            };
            if let Some(commit) = catalog.pins.get(&file.state.hash) {
                result.exclusive.push(LegacyFile {
                    path: pin,
                    provider: provider.into(),
                    sha256: file.state.hash,
                    selected: selected.contains(provider),
                    kind: EntryKind::Pin,
                    sources: vec![Source {
                        package_version: format!("pin:{commit}"),
                        channel: "fixed-pin-template".into(),
                    }],
                });
            } else if String::from_utf8_lossy(&file.bytes).contains("<!-- impeccable-pinned-")
                || name.starts_with("impeccable")
            {
                conflict(result,&pin,Some(provider),ConflictKind::UnknownLegacyEntry,"legacy-looking shortcut/agent/command is not a complete fixed artifact; keep it and resolve manually");
            } else {
                result.preserved.push(Preserved {
                    path,
                    reason: "unowned skill, agent or command".into(),
                });
            }
        }
    }
}

fn identity_matches(value: &Value, identity: &Identity) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    identity
        .required_keys
        .iter()
        .all(|key| object.contains_key(key))
        && identity
            .fields
            .iter()
            .all(|(key, expected)| match (object.get(key), expected) {
                (None, None) => true,
                (Some(actual), Some(expected)) => actual == expected,
                _ => false,
            })
}

// Detection may conservatively flag a legacy script path, but it never grants
// ownership. Bare mentions such as `echo "impeccable notes"` remain unowned.
fn legacy_command(value: &Value) -> bool {
    ["command", "commandWindows", "bash", "powershell"]
        .iter()
        .any(|key| {
            value.get(*key).and_then(Value::as_str).is_some_and(|text| {
                text.contains("skills/impeccable/scripts/")
                    || text.contains(".impeccable/bin/")
                    || text.starts_with("impeccable ")
            })
        })
}

fn classify_hooks(
    path: &str,
    value: &Value,
    patterns: &[Pattern],
    selected: &BTreeSet<String>,
    result: &mut Inventory,
) -> Vec<LegacyHandler> {
    let Some(hooks) = value.get("hooks") else {
        return Vec::new();
    };
    let Some(events) = hooks.as_object() else {
        conflict(
            result,
            path,
            None,
            ConflictKind::InvalidJson,
            "hooks must be an object of event arrays",
        );
        return Vec::new();
    };
    let mut owned = Vec::new();
    for (event, entries) in events {
        let Some(entries) = entries.as_array() else {
            conflict(
                result,
                path,
                None,
                ConflictKind::InvalidJson,
                &format!("event {event} must contain an array"),
            );
            continue;
        };
        for (outer_index, outer) in entries.iter().enumerate() {
            let nested = outer.get("hooks");
            let values: Vec<(usize, &Value)> = if let Some(nested) = nested {
                let Some(values) = nested.as_array() else {
                    conflict(
                        result,
                        path,
                        None,
                        ConflictKind::InvalidJson,
                        &format!("event {event} nested hooks must contain an array"),
                    );
                    continue;
                };
                values.iter().enumerate().collect()
            } else {
                vec![(outer_index, outer)]
            };
            for (index, handler) in &values {
                let candidates: Vec<_> = patterns
                    .iter()
                    .filter(|p| {
                        p.event == *event
                            && p.value == **handler
                            && match (&p.identity, nested) {
                                (Some(identity), Some(_)) => identity_matches(outer, identity),
                                (None, None) => true,
                                _ => false,
                            }
                    })
                    .collect();
                if candidates.is_empty() {
                    let legacy_name =
                        handler
                            .get("name")
                            .and_then(Value::as_str)
                            .is_some_and(|name| {
                                patterns.iter().any(|p| {
                                    p.value.get("name").and_then(Value::as_str) == Some(name)
                                })
                            });
                    if legacy_command(handler) || legacy_name {
                        conflict(result,path,None,ConflictKind::CustomizedHandler,&format!("event {event}, outer {outer_index}, handler {index} differs from every complete supported structure; keep wrappers and unknown fields"));
                    }
                    continue;
                }
                let pattern = candidates[0];
                // A group holding just one claimed handler is the old bundle's own
                // group. Retire it whole by exact value: a neighbor group with the
                // same (often absent) matcher must not make it ambiguous.
                let whole = nested.is_some()
                    && values.len() == 1
                    && pattern.identity.as_ref().is_some_and(|identity| {
                        let mut group: serde_json::Map<String, Value> = identity
                            .fields
                            .iter()
                            .filter_map(|(key, value)| Some((key.clone(), value.clone()?)))
                            .collect();
                        group.insert("hooks".into(), Value::Array(vec![pattern.value.clone()]));
                        Value::Object(group) == *outer
                    });
                if whole {
                    if candidates
                        .iter()
                        .any(|p| p.provider != pattern.provider || p.identity != pattern.identity)
                        || entries.iter().filter(|entry| *entry == outer).count() > 1
                    {
                        conflict(
                            result,
                            path,
                            Some(&pattern.provider),
                            ConflictKind::AmbiguousHandler,
                            &format!("event {event}, outer {outer_index} has duplicate values"),
                        );
                        continue;
                    }
                    owned.push(LegacyHandler {
                        provider: pattern.provider.clone(),
                        selected: selected.contains(&pattern.provider),
                        parent: vec![JsonStep::Key("hooks".into()), JsonStep::Key(event.clone())],
                        slot: JsonSlot::Element(outer_index),
                        value: outer.clone(),
                        sources: candidates
                            .iter()
                            .flat_map(|p| p.sources.iter().cloned())
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect(),
                    });
                    continue;
                }
                let duplicates = if nested.is_some() {
                    values
                        .iter()
                        .filter(|(_, value)| *value == *handler)
                        .count()
                } else {
                    entries.iter().filter(|value| *value == *handler).count()
                };
                if candidates
                    .iter()
                    .any(|p| p.provider != pattern.provider || p.identity != pattern.identity)
                    || duplicates > 1
                    || pattern.identity.as_ref().is_some_and(|id| {
                        entries
                            .iter()
                            .filter(|entry| identity_matches(entry, id))
                            .count()
                            > 1
                    })
                {
                    conflict(result,path,Some(&pattern.provider),ConflictKind::AmbiguousHandler,&format!("event {event}, outer {outer_index}, handler {index} has duplicate values or ambiguous matcher identity"));
                    continue;
                }
                let mut parent = vec![JsonStep::Key("hooks".into()), JsonStep::Key(event.clone())];
                if let Some(identity) = &pattern.identity {
                    parent.push(JsonStep::Match(identity.clone()));
                    parent.push(JsonStep::Key("hooks".into()));
                }
                owned.push(LegacyHandler {
                    provider: pattern.provider.clone(),
                    selected: selected.contains(&pattern.provider),
                    parent,
                    slot: JsonSlot::Element(*index),
                    value: pattern.value.clone(),
                    sources: candidates
                        .iter()
                        .flat_map(|p| p.sources.iter().cloned())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                });
            }
        }
    }
    owned
}

fn inspect_live(root: &Path, result: &mut Inventory) {
    for path in [
        ".impeccable-live.json",
        ".impeccable/live/config.json",
        ".impeccable/live/inject-journal.json",
        ".impeccable/live/server.json",
    ] {
        let Some(file) = read(root, path, None, result) else {
            continue;
        };
        conflict(result,path,None,ConflictKind::LiveResidue,"old session/injection state remains; use the old project launcher to stop, remove injections, and accept/discard variants before retrying; do not kill by PID");
        if path.ends_with("inject-journal.json") {
            if let Ok(value) = json::parse_value(&file.bytes) {
                for artifact in value["artifacts"].as_array().into_iter().flatten() {
                    if let Some(target) = artifact["path"].as_str() {
                        if disk::relative(target).is_err() {
                            conflict(result,path,None,ConflictKind::LiveResidue,"injection journal contains an unsafe project reference; manual recovery is required");
                        } else if read(root, target, None, result).is_some() {
                            conflict(result,target,None,ConflictKind::LiveResidue,"source referenced by the old injection journal must be restored using live-inject --remove; never delete its markers to bypass recovery");
                        }
                    }
                }
            }
        }
    }
}
