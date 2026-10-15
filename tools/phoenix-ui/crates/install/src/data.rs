//! Read-only preparation of explicitly supported legacy project data and pins.
//! Copies retain the old source and use Retain rollback; only complete fixed pin
//! templates may be regenerated with Restore rollback. This does not activate an
//! install or discover personal stores, processes, or external review receipts.
//!
//! The allowlist is config/config.local, design sidecar schema 2, surface brief
//! version 1, and stamped critique history. Known input references are mapped
//! only to validated copied artifacts; hook.auditLog is a registered output
//! path. Unknown fields and prose retain their original bytes. Build/accepted
//! families without complete producer mappings block preparation. The bounded
//! text-source scan is opt-in for explicit migration and excludes .git,
//! node_modules, vendor, target, provider roots and the separately inspected state
//! namespaces. Caller discovery must cover external stores, overridden live
//! roots, and arbitrary capture ready files.
use crate::{
    classify::{Conflict, ConflictKind, EntryKind, Inventory, Preserved},
    disk, json, sha256, Change, Error, ExpectedFile, JsonPatch, JsonSlot, JsonStep, Payload,
    Permissions, Project, Result, RollbackPolicy,
};
use serde::Serialize;
use serde_json::{Map, Value};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

const FILE_LIMIT: u64 = 4 * 1024 * 1024;
const ENTRY_LIMIT: usize = 10_000;
const TOTAL_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct DataSource {
    pub source_path: String,
    pub source_sha256: String,
    pub target_path: String,
    pub target_sha256: String,
    pub permissions: Permissions,
}

#[derive(Clone, Debug, Serialize)]
pub struct PathMapping {
    pub source_path: String,
    pub field: Option<String>,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Unconsumed {
    pub path: String,
    pub field: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedPin {
    pub old_path: String,
    pub new_path: String,
    pub old_sha256: String,
    pub payload: Payload,
}

#[derive(Debug, Default, Serialize)]
pub struct PreparedData {
    pub changes: Vec<Change>,
    pub sources: Vec<DataSource>,
    pub mappings: Vec<PathMapping>,
    pub unconsumed: Vec<Unconsumed>,
    pub conflicts: Vec<Conflict>,
    pub preserved: Vec<Preserved>,
    /// Only these exact old paths have a validated replacement payload. A caller
    /// must not use this list to dismiss any other migration conflict.
    pub resolved_pin_paths: Vec<ResolvedPin>,
}

/// Prepare migration of persistent project data and selected fixed pins without
/// writing state. The caller merges these changes with the bundle plan, preserves
/// source snapshots, and repeats preparation if an old source changes before apply.
/// Ordinary installation checks fixed legacy paths; explicit migration may also
/// scan business sources for orphaned injection markers.
pub fn prepare_legacy_data(
    project: &Project,
    inventory: &Inventory,
    scan_sources: bool,
) -> Result<PreparedData> {
    let mut result = PreparedData {
        conflicts: inventory.conflicts.clone(),
        preserved: inventory.preserved.clone(),
        ..PreparedData::default()
    };
    let mut paths = Vec::new();
    state_paths(project.root(), ".impeccable", &mut paths, &mut result)?;
    let mut candidates = Vec::new();
    let mut data_bytes = 0;
    for path in paths {
        let Some(file) = read(project.root(), &path, &mut result) else {
            continue;
        };
        data_bytes += file.bytes.len() as u64;
        if data_bytes > TOTAL_LIMIT {
            return Err(Error::conflict(
                "persistent data byte budget exceeded; no writes are prepared",
            ));
        }
        let processed = if matches!(
            path.as_str(),
            ".impeccable/config.json" | ".impeccable/config.local.json"
        ) {
            config(project.root(), &path, &file.bytes, &mut result)
        } else if path == ".impeccable/design.json" {
            design(&path, &file.bytes, &mut result)
        } else if path.starts_with(".impeccable/surfaces/")
            || path.starts_with(".impeccable/critique/")
        {
            markdown(&path, &file.bytes, &mut result)
        } else {
            unreachable!("state_paths returns only explicit allowlist families")
        };
        match processed {
            Ok(bytes) => candidates.push((path, file, bytes)),
            Err(message) => conflict(&mut result, &path, ConflictKind::InvalidJson, &message),
        }
    }
    let copied: BTreeSet<_> = candidates.iter().map(|c| c.0.clone()).collect();
    for (path, file, bytes) in candidates {
        let bytes = if path.starts_with(".impeccable/surfaces/")
            || path.starts_with(".impeccable/critique/")
        {
            match map_markdown(project.root(), &path, &bytes, &copied, &mut result) {
                Ok(bytes) => bytes,
                Err(message) => {
                    conflict(
                        &mut result,
                        &path,
                        ConflictKind::UnknownLegacyEntry,
                        &message,
                    );
                    continue;
                }
            }
        } else {
            bytes
        };
        copy(project.root(), &path, file, bytes, &mut result);
    }
    pins(project.root(), inventory, &mut result)?;
    for path in [".impeccable-live.json", ".impeccable-live"] {
        match disk::checked(project.root(), path).and_then(|full| {
            match fs::symlink_metadata(&full) {
                Ok(_) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(e) => Err(Error::io(&full, e)),
            }
        }) {
            Ok(true) => live(&mut result, path),
            Ok(false) => {}
            Err(error) => conflict(&mut result, path, ConflictKind::Symlink, &error.message),
        }
    }
    if scan_sources {
        let providers = provider_ids()?;
        let mut budget = (0, 0);
        source_residue(project.root(), "", &providers, &mut budget, &mut result)?;
    }
    result.changes.sort_by(|a, b| a.path().cmp(b.path()));
    result
        .conflicts
        .sort_by(|a, b| (&a.path, &a.message).cmp(&(&b.path, &b.message)));
    result.conflicts.dedup_by(|a, b| {
        a.path == b.path && a.provider == b.provider && a.kind == b.kind && a.message == b.message
    });
    result.preserved.sort_by(|a, b| a.path.cmp(&b.path));
    result
        .preserved
        .dedup_by(|a, b| a.path == b.path && a.reason == b.reason);
    if !result.conflicts.is_empty() {
        result.changes.clear();
        result.resolved_pin_paths.clear();
    }
    Ok(result)
}

fn conflict(result: &mut PreparedData, path: &str, kind: ConflictKind, message: &str) {
    result.conflicts.push(Conflict {
        path: path.into(),
        provider: None,
        kind,
        message: message.into(),
    });
}
fn unconsumed(result: &mut PreparedData, path: &str, field: &str, reason: &str) {
    result.unconsumed.push(Unconsumed {
        path: path.into(),
        field: field.into(),
        reason: reason.into(),
    });
}
fn live(result: &mut PreparedData, path: &str) {
    let message = if path.starts_with(".impeccable/questions") {
        "question state remains; finish or close the old question page, then use the old project launcher with serve-question --stop --key <the recorded question key>; verify the launcher/session before stopping and rerun dry-run. No PID is probed or signalled by this preparer"
    } else if path.contains("capture") {
        "capture state remains; stop the original capture-service launcher normally, preserve accepted artifacts using the trusted old host, then rerun dry-run; arbitrary external ready-file discovery belongs to the caller"
    } else {
        "live/injection/variant residue remains; verify the original live launcher/session, then use its live-server stop and live-inject --remove; resolve wrappers with live-accept --id <verified session ID> --variant N or --discard before repeating dry-run; never delete source markers or kill a guessed PID"
    };
    conflict(result, path, ConflictKind::LiveResidue, message);
}

// No following references, symlinks or FIFO opens. The bounds are a conservative
// preflight boundary, not a claim that an incomplete scan is clean.
fn bounded_read(root: &Path, path: &str) -> Result<Option<disk::Snapshot>> {
    let full = disk::checked(root, path)?;
    let before = match fs::symlink_metadata(&full) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::io(&full, e)),
    };
    if !before.is_file() || before.len() > FILE_LIMIT {
        return Err(Error::conflict(
            "not a bounded regular file; inspect it manually",
        ));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(&full).map_err(|e| Error::io(&full, e))?;
    let meta = file.metadata().map_err(|e| Error::io(&full, e))?;
    if !meta.is_file() || meta.len() > FILE_LIMIT {
        return Err(Error::conflict(
            "source changed type or exceeded the byte limit",
        ));
    }
    let mut bytes = Vec::new();
    file.take(FILE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(&full, e))?;
    if bytes.len() as u64 != meta.len()
        || meta.len() != before.len()
        || disk::permissions(&meta) != disk::permissions(&before)
    {
        return Err(Error::conflict(
            "source changed during read; repeat preparation",
        ));
    }
    Ok(Some(disk::Snapshot {
        state: disk::State {
            hash: sha256(&bytes),
            permissions: disk::permissions(&meta),
        },
        bytes,
    }))
}
fn read(root: &Path, path: &str, result: &mut PreparedData) -> Option<disk::Snapshot> {
    match bounded_read(root, path) {
        Ok(file) => file,
        Err(error) => {
            conflict(result, path, ConflictKind::Unreadable, &error.message);
            None
        }
    }
}

fn state_paths(
    root: &Path,
    at: &str,
    paths: &mut Vec<String>,
    result: &mut PreparedData,
) -> Result<()> {
    let full = match disk::checked(root, at) {
        Ok(full) => full,
        Err(error) => {
            conflict(result, at, ConflictKind::Symlink, &error.message);
            return Ok(());
        }
    };
    let entries = match fs::read_dir(&full) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            conflict(result, at, ConflictKind::Unreadable, &e.to_string());
            return Ok(());
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        names.push(entry.map_err(|e| Error::io(&full, e))?.file_name());
        if names.len() > ENTRY_LIMIT {
            return Err(Error::conflict(
                "legacy data entry limit exceeded; inspect/reduce the scope manually",
            ));
        }
    }
    names.sort();
    for name in names {
        let path = format!("{at}/{}", name.to_string_lossy());
        let checked = match disk::checked(root, &path) {
            Ok(full) => full,
            Err(error) => {
                conflict(result, &path, ConflictKind::Symlink, &error.message);
                continue;
            }
        };
        let meta = fs::symlink_metadata(&checked).map_err(|e| Error::io(&checked, e))?;
        if at == ".impeccable"
            && matches!(name.to_str(), Some("surfaces" | "critique"))
            && meta.is_dir()
        {
            state_paths(root, &path, paths, result)?;
        } else if matches!(
            path.as_str(),
            ".impeccable/config.json" | ".impeccable/config.local.json" | ".impeccable/design.json"
        ) || (at != ".impeccable" && meta.is_file())
        {
            paths.push(path);
        } else {
            result.preserved.push(Preserved {
                path: path.clone(),
                reason:
                    "outside the explicit persistent-data allowlist; original state is retained"
                        .into(),
            });
            match path.as_str() {
                ".impeccable/hook.cache.json"
                | ".impeccable/hook.pending.json"
                | ".impeccable/cache"
                | ".impeccable/pending"
                | ".impeccable/dedup" => unconsumed(
                    result,
                    &path,
                    "*",
                    "disposable cache/queue/dedup state is excluded, without reading its content",
                ),
                ".impeccable/questions" => {
                    live(result, &path);
                    question_keys(root, &path, result)?;
                }
                ".impeccable/live" | ".impeccable/capture" | ".impeccable/captures" => {
                    live(result, &path)
                }
                _ => {
                    unconsumed(
                        result,
                        &path,
                        "*",
                        "no complete supported producer schema/path mapping; no copy is authorized",
                    );
                    conflict(
                        result,
                        &path,
                        ConflictKind::UnknownLegacyEntry,
                        "persistent family is unsupported by this preparer; preserve it and arrange explicit old-producer schema/accepted-artifact mapping before activation",
                    );
                }
            }
        }
    }
    if paths.len() > ENTRY_LIMIT {
        return Err(Error::conflict(
            "persistent data count exceeds the preflight limit",
        ));
    }
    Ok(())
}

fn copy(root: &Path, path: &str, file: disk::Snapshot, bytes: Vec<u8>, result: &mut PreparedData) {
    let target = path.replacen(".impeccable/", ".phoenix-ui/", 1);
    let existing = match bounded_read(root, &target) {
        Ok(existing) => existing,
        Err(error) => {
            conflict(result, &target, ConflictKind::Unreadable, &error.message);
            return;
        }
    };
    let (expected, target_permissions) = match existing {
        Some(existing) if existing.bytes == bytes => (
            ExpectedFile::Hash(existing.state.hash),
            existing.state.permissions,
        ),
        Some(_) => {
            conflict(
                result,
                &target,
                ConflictKind::CustomizedFile,
                "new Phoenix data already differs; preserve both versions and resolve the explicit conflict without automatic merging",
            );
            return;
        }
        None => (ExpectedFile::Absent, file.state.permissions.clone()),
    };
    let payload = Payload::new(bytes, target_permissions);
    result.sources.push(DataSource {
        source_path: path.into(),
        source_sha256: file.state.hash,
        target_path: target.clone(),
        target_sha256: payload.sha256.clone(),
        permissions: file.state.permissions,
    });
    result.mappings.push(PathMapping {
        source_path: path.into(),
        field: None,
        before: path.into(),
        after: target.clone(),
    });
    result.changes.push(Change::Exclusive {
        path: target,
        expected,
        after: Some(payload),
        rollback: RollbackPolicy::Retain,
    });
}

fn object<'a>(
    value: &'a Value,
    field: &str,
) -> std::result::Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{field} requires an object"))
}
fn check(ok: bool, field: &str, expected: &str) -> std::result::Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!(
            "{field} requires {expected}; original bytes are retained"
        ))
    }
}
fn strings(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|a| a.iter().all(Value::is_string))
}
fn unknown(
    map: &Map<String, Value>,
    known: &[&str],
    path: &str,
    prefix: &str,
    result: &mut PreparedData,
) {
    for key in map.keys().filter(|key| !known.contains(&key.as_str())) {
        unconsumed(
            result,
            path,
            &format!("{prefix}{key}"),
            "unknown field is preserved byte-for-byte and is not consumed by this preparer",
        );
    }
}
fn transient_key(key: &str) -> bool {
    matches!(
        key,
        "pid" | "port" | "token" | "pending" | "cache" | "dedup" | "live" | "sessionId" | "capture"
    )
}
fn reject_transient(value: &Value, field: &str) -> std::result::Result<(), String> {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if transient_key(key) {
                    return Err(format!(
                        "{field}{key} is ambiguous runtime/session state; no persistent copy is authorized; stop the old launcher normally and recover before retrying"
                    ));
                }
                reject_transient(value, &format!("{field}{key}."))?;
            }
        }
        Value::Array(a) => {
            for value in a {
                reject_transient(value, field)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn detector(
    map: &Map<String, Value>,
    path: &str,
    prefix: &str,
    result: &mut PreparedData,
) -> std::result::Result<(), String> {
    for (key, value) in map {
        let field = format!("{prefix}{key}");
        match key.as_str() {
            "ignoreRules" | "ignoreFiles" => check(strings(value), &field, "an array of strings")?,
            "advisoryRules" => check(
                matches!(value.as_str(), Some("include" | "exclude")),
                &field,
                "include or exclude",
            )?,
            "designSystem" => {
                let m = object(value, &field)?;
                if let Some(v) = m.get("enabled") {
                    check(v.is_boolean(), &format!("{field}.enabled"), "a boolean")?;
                }
                unknown(m, &["enabled"], path, &format!("{field}."), result);
            }
            "extensions" => {
                let list = value
                    .as_array()
                    .ok_or_else(|| format!("{field} requires an array"))?;
                for (i, v) in list.iter().enumerate() {
                    if v.is_string() {
                        continue;
                    }
                    let m = object(v, &field)?;
                    check(
                        m.get("ext").is_some_and(Value::is_string),
                        &field,
                        "a string or an object with string ext",
                    )?;
                    unknown(m, &["ext"], path, &format!("{field}[{i}]."), result);
                }
            }
            "ignoreValues" => {
                let list = value
                    .as_array()
                    .ok_or_else(|| format!("{field} requires an array"))?;
                for (i, v) in list.iter().enumerate() {
                    let m = object(v, &field)?;
                    for required in ["rule", "value"] {
                        check(
                            m.get(required).is_some_and(Value::is_string),
                            &format!("{field}[{i}].{required}"),
                            "a string",
                        )?;
                    }
                    for (k, v) in m {
                        match k.as_str() {
                            "files" => check(
                                v.is_null() || strings(v),
                                &field,
                                "an optional string array",
                            )?,
                            "createdAt" | "reason" => check(v.is_string(), &field, "a string")?,
                            _ => {}
                        }
                    }
                    unknown(
                        m,
                        &["rule", "value", "files", "createdAt", "reason"],
                        path,
                        &format!("{field}[{i}]."),
                        result,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(())
}
/// Phoenix reads `.phoenix-ui` configuration and sidecars as strict JSON. A file
/// only the JSONC editor accepts would migrate and then be unreadable or ignored.
fn strict_json(bytes: &[u8]) -> std::result::Result<Value, String> {
    serde_json::from_slice::<Value>(bytes).map_err(|e| {
        format!("Phoenix reads this file as strict JSON ({e}); remove comments or trailing commas, then rerun; original bytes are retained")
    })?;
    json::parse_value(bytes).map_err(|e| e.message)
}
fn config(
    root: &Path,
    path: &str,
    bytes: &[u8],
    result: &mut PreparedData,
) -> std::result::Result<Vec<u8>, String> {
    let value = strict_json(bytes)?;
    let map = object(&value, "config")?;
    reject_transient(&value, "")?;
    let known = [
        "hook",
        "detector",
        "updateCheck",
        "projectRoots",
        "buildPath",
        "browser",
    ];
    unknown(map, &known, path, "", result);
    check(
        map.is_empty() || map.keys().any(|k| known.contains(&k.as_str())),
        "config",
        "at least one supported persistent field or an empty object",
    )?;
    let mut patches = Vec::new();
    for (key, value) in map {
        match key.as_str() {
            "updateCheck" => check(value.is_boolean(), key, "a boolean")?,
            "projectRoots" => check(strings(value), key, "an array of strings")?,
            "buildPath" => check(
                matches!(value.as_str(), Some("comp" | "code")),
                key,
                "comp or code",
            )?,
            "browser" => check(
                value.as_str().is_some_and(|s| !s.trim().is_empty()),
                key,
                "a nonempty browser name/path string",
            )?,
            "hook" | "detector" => {
                let section = object(value, key)?;
                detector(section, path, &format!("{key}."), result)?;
                let detector_keys = [
                    "ignoreRules",
                    "ignoreFiles",
                    "ignoreValues",
                    "designSystem",
                    "advisoryRules",
                    "extensions",
                ];
                let mut known = detector_keys.to_vec();
                if key == "hook" {
                    known.extend([
                        "enabled",
                        "quiet",
                        "perEditRules",
                        "auditLog",
                        "limits",
                        "consent",
                    ]);
                    for (k, v) in section {
                        match k.as_str() {
                            "enabled" | "quiet" => {
                                check(v.is_boolean(), &format!("hook.{k}"), "a boolean")?
                            }
                            "perEditRules" => check(
                                matches!(v.as_str(), Some("all" | "immediate")),
                                "hook.perEditRules",
                                "all or immediate",
                            )?,
                            "consent" => check(
                                matches!(v.as_str(), Some("accepted" | "declined")),
                                "hook.consent",
                                "accepted or declined",
                            )?,
                            "auditLog" => {
                                let text = v
                                    .as_str()
                                    .filter(|s| !s.trim().is_empty())
                                    .ok_or("hook.auditLog requires a nonempty string")?;
                                let normal = if cfg!(windows) {
                                    text.replace('\\', "/")
                                } else {
                                    text.to_string()
                                };
                                let project_prefix =
                                    format!("{}/", root.to_string_lossy().replace('\\', "/"));
                                let (prefix, relative) =
                                    if let Some(relative) = normal.strip_prefix(&project_prefix) {
                                        (project_prefix.as_str(), relative)
                                    } else {
                                        ("", normal.as_str())
                                    };
                                if let Some(rest) = relative.strip_prefix(".impeccable/") {
                                    disk::relative(relative).map_err(|e| e.message)?;
                                    check(
                                        !matches!(
                                            rest.split('/').next(),
                                            Some(
                                                "install"
                                                    | "migrations"
                                                    | "live"
                                                    | "questions"
                                                    | "capture"
                                                    | "cache"
                                                    | "pending"
                                                    | "dedup"
                                                    | "hook.cache.json"
                                                    | "hook.pending.json"
                                            )
                                        ),
                                        "hook.auditLog",
                                        "a persistent output path outside runtime/reserved directories",
                                    )?;
                                    let after = format!("{prefix}.phoenix-ui/{rest}");
                                    patches.push(JsonPatch {
                                        parent: vec![JsonStep::Key("hook".into())],
                                        slot: JsonSlot::Member("auditLog".into()),
                                        before: Some(v.clone()),
                                        after: Some(Value::String(after.clone())),
                                    });
                                    result.mappings.push(PathMapping {
                                        source_path: path.into(),
                                        field: Some("hook.auditLog".into()),
                                        before: text.into(),
                                        after,
                                    });
                                }
                            }
                            "limits" => {
                                let limits = object(v, "hook.limits")?;
                                for (name, v) in limits {
                                    if ["maxFindings", "maxChars", "maxFileBytes"]
                                        .contains(&name.as_str())
                                    {
                                        check(
                                            v.as_f64().is_some_and(|n| n.is_finite() && n > 0.0),
                                            &format!("hook.limits.{name}"),
                                            "a positive finite number",
                                        )?;
                                    }
                                }
                                unknown(
                                    limits,
                                    &["maxFindings", "maxChars", "maxFileBytes"],
                                    path,
                                    "hook.limits.",
                                    result,
                                );
                            }
                            _ => {}
                        }
                    }
                }
                unknown(section, &known, path, &format!("{key}."), result);
            }
            _ => {}
        }
    }
    json::prepare(bytes, &patches)
        .map(|p| p.0)
        .map_err(|e| e.message)
}

fn design(
    path: &str,
    bytes: &[u8],
    result: &mut PreparedData,
) -> std::result::Result<Vec<u8>, String> {
    let value = strict_json(bytes)?;
    let map = object(&value, "design")?;
    check(
        !map.keys().any(|key| transient_key(key)),
        "design",
        "persistent sidecar fields without root PID/port/token/session state; recover the old launcher before retrying",
    )?;
    check(
        map.get("schemaVersion").and_then(Value::as_u64) == Some(2),
        "schemaVersion",
        "supported design sidecar schema 2; schema 1 needs explicit regeneration from its old producer",
    )?;
    for key in ["generatedAt", "title"] {
        check(map.get(key).is_some_and(Value::is_string), key, "a string")?;
    }
    let ext = object(
        map.get("extensions").ok_or("missing extensions")?,
        "extensions",
    )?;
    for (key, v) in ext {
        match key.as_str() {
            "colorMeta" | "typographyMeta" => {
                for (name, v) in object(v, key)? {
                    let m = object(v, &format!("extensions.{key}.{name}"))?;
                    let keys = if key == "colorMeta" {
                        vec!["role", "displayName", "canonical", "tonalRamp"]
                    } else {
                        vec!["displayName", "purpose"]
                    };
                    for (k, v) in m {
                        if keys.contains(&k.as_str()) {
                            check(
                                if k == "tonalRamp" {
                                    strings(v)
                                } else {
                                    v.is_string()
                                },
                                &format!("extensions.{key}.{name}.{k}"),
                                "the supported string/string-array shape",
                            )?;
                        }
                    }
                    unknown(m, &keys, path, &format!("extensions.{key}.{name}."), result);
                }
            }
            "shadows" | "motion" | "breakpoints" => records(
                v,
                if key == "breakpoints" {
                    &["name", "value"]
                } else {
                    &["name", "value", "purpose"]
                },
                path,
                &format!("extensions.{key}"),
                result,
            )?,
            _ => {}
        }
    }
    unknown(
        ext,
        &[
            "colorMeta",
            "typographyMeta",
            "shadows",
            "motion",
            "breakpoints",
        ],
        path,
        "extensions.",
        result,
    );
    records(
        map.get("components").ok_or("missing components")?,
        &["name", "kind", "refersTo", "description", "html", "css"],
        path,
        "components",
        result,
    )?;
    let narrative = object(
        map.get("narrative").ok_or("missing narrative")?,
        "narrative",
    )?;
    for (key, v) in narrative {
        match key.as_str() {
            "northStar" | "overview" => {
                check(v.is_string(), &format!("narrative.{key}"), "a string")?
            }
            "keyCharacteristics" | "dos" | "donts" => {
                check(strings(v), &format!("narrative.{key}"), "a string array")?
            }
            "rules" => records(
                v,
                &["name", "body", "section"],
                path,
                "narrative.rules",
                result,
            )?,
            _ => {}
        }
    }
    unknown(
        narrative,
        &[
            "northStar",
            "overview",
            "keyCharacteristics",
            "rules",
            "dos",
            "donts",
        ],
        path,
        "narrative.",
        result,
    );
    unknown(
        map,
        &[
            "schemaVersion",
            "generatedAt",
            "title",
            "extensions",
            "components",
            "narrative",
        ],
        path,
        "",
        result,
    );
    Ok(bytes.to_vec())
}
fn records(
    value: &Value,
    keys: &[&str],
    path: &str,
    prefix: &str,
    result: &mut PreparedData,
) -> std::result::Result<(), String> {
    let list = value
        .as_array()
        .ok_or_else(|| format!("{prefix} requires an array"))?;
    for (i, v) in list.iter().enumerate() {
        let m = object(v, prefix)?;
        for key in keys {
            check(
                m.get(*key).is_some_and(Value::is_string),
                &format!("{prefix}[{i}].{key}"),
                "a string",
            )?;
        }
        unknown(m, keys, path, &format!("{prefix}[{i}]."), result);
    }
    Ok(())
}

// Only the flat frontmatter actually emitted by surface_briefs and
// critique_storage is accepted; this is not a general YAML parser.
fn frontmatter(text: &str) -> std::result::Result<Map<String, Value>, String> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
        .ok_or("missing producer frontmatter")?;
    let mut map = Map::new();
    for line in rest.lines() {
        if line == "---" {
            return Ok(map);
        }
        if line.trim().is_empty() {
            continue;
        }
        let (key, raw) = line
            .split_once(':')
            .ok_or("unsupported non-flat frontmatter")?;
        check(
            !key.is_empty() && key.trim() == key && !map.contains_key(key),
            "frontmatter",
            "unique flat keys",
        )?;
        let raw = raw.trim();
        let value = if raw.starts_with(['"', '[', '{'])
            || matches!(raw, "true" | "false")
            || raw.parse::<i64>().is_ok()
        {
            match serde_json::from_str(raw) {
                Ok(value) => value,
                Err(_)
                    if matches!(
                        key,
                        "version"
                            | "slug"
                            | "primary_target"
                            | "related_targets"
                            | "timestamp"
                            | "closed"
                            | "target_identity"
                            | "target_path"
                            | "target_fingerprint"
                    ) =>
                {
                    return Err("invalid known frontmatter JSON scalar/array".into());
                }
                // critique_storage serializes caller arrays/objects with JS
                // String(), and plain strings need not be JSON literals.
                Err(_) => Value::String(raw.into()),
            }
        } else {
            Value::String(raw.into())
        };
        map.insert(key.into(), value);
    }
    Err("unterminated frontmatter".into())
}
fn markdown(
    path: &str,
    bytes: &[u8],
    result: &mut PreparedData,
) -> std::result::Result<Vec<u8>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "persistent Markdown must be UTF-8")?;
    let map = frontmatter(text)?;
    reject_transient(&Value::Object(map.clone()), "")?;
    let name = path.rsplit('/').next().unwrap_or_default();
    if path.starts_with(".impeccable/surfaces/") {
        check(
            name.ends_with(".md") && map.get("version").and_then(Value::as_u64) == Some(1),
            "version",
            "surface brief version 1 in a .md file",
        )?;
        check(
            map.get("slug").and_then(Value::as_str) == name.strip_suffix(".md"),
            "slug",
            "the producer filename slug",
        )?;
        check(
            map.get("primary_target").is_some_and(Value::is_string)
                && map.get("related_targets").is_some_and(strings),
            "targets",
            "string primary_target and array of string related_targets",
        )?;
        unknown(
            &map,
            &["version", "slug", "primary_target", "related_targets"],
            path,
            "",
            result,
        );
    } else {
        let stamp = b"0000-00-00T00-00-00Z";
        let b = name.as_bytes();
        check(
            b.len() > 25
                && stamp.iter().enumerate().all(|(i, p)| {
                    b.get(i).is_some_and(|x| {
                        if *p == b'0' {
                            x.is_ascii_digit()
                        } else {
                            x == p
                        }
                    })
                }),
            "filename",
            "a producer critique UTC timestamp",
        )?;
        let rest = &name[20..];
        let rest = if let Some(rest) = rest.strip_prefix('~') {
            check(
                rest.len() > 4 && rest.as_bytes()[..4].iter().all(u8::is_ascii_digit),
                "filename",
                "a four-digit collision suffix",
            )?;
            &rest[4..]
        } else {
            rest
        };
        let slug = rest
            .strip_prefix("__")
            .and_then(|s| s.strip_suffix(".md"))
            .filter(|s| !s.is_empty())
            .ok_or("critique requires the stamped __slug.md producer filename")?;
        check(
            map.get("timestamp").and_then(Value::as_str) == Some(&name[..20])
                && map.get("slug").and_then(Value::as_str) == Some(slug),
            "identity",
            "timestamp and slug matching the producer filename",
        )?;
        for (key, v) in &map {
            match key.as_str() {
                "closed" => check(
                    v.is_boolean(),
                    key,
                    "a historical boolean (not an accepted-review receipt)",
                )?,
                "target_fingerprint" => check(
                    v.as_str().is_some_and(|s| {
                        s.strip_prefix("sha256:").is_some_and(|h| {
                            h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())
                        })
                    }),
                    key,
                    "a sha256:<64 hex> string",
                )?,
                "target_path" | "target_identity" => check(v.is_string(), key, "a string")?,
                _ => {}
            }
        }
        unknown(
            &map,
            &[
                "timestamp",
                "slug",
                "closed",
                "target_identity",
                "target_path",
                "target_fingerprint",
            ],
            path,
            "",
            result,
        );
    }
    Ok(bytes.to_vec())
}

fn question_keys(root: &Path, path: &str, result: &mut PreparedData) -> Result<()> {
    let full = disk::checked(root, path)?;
    if !fs::symlink_metadata(&full)
        .map_err(|e| Error::io(&full, e))?
        .is_dir()
    {
        return Ok(());
    }
    for (i, entry) in fs::read_dir(&full)
        .map_err(|e| Error::io(&full, e))?
        .enumerate()
    {
        if i >= ENTRY_LIMIT {
            return Err(Error::conflict(
                "question state count exceeds the preflight limit",
            ));
        }
        let entry = entry.map_err(|e| Error::io(&full, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(key) = name.strip_suffix(".state.json") else {
            continue;
        };
        let path = format!("{path}/{name}");
        if key.is_empty() || key.contains(['\r', '\n', '\0']) {
            conflict(
                result,
                &path,
                ConflictKind::LiveResidue,
                "question key cannot be safely reported; inspect the old question launcher before recovery",
            );
            continue;
        }
        let quoted = format!("'{}'", key.replace('\'', "'\\''"));
        conflict(
            result,
            &path,
            ConflictKind::LiveResidue,
            &format!(
                "verify the original question launcher/session, then use that launcher with serve-question --stop --key {quoted}; preserve any selected result first and repeat dry-run. This preparation never reads or signals the recorded PID"
            ),
        );
    }
    Ok(())
}

fn map_markdown(
    root: &Path,
    path: &str,
    bytes: &[u8],
    copied: &BTreeSet<String>,
    result: &mut PreparedData,
) -> std::result::Result<Vec<u8>, String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Markdown requires UTF-8")?;
    let mut values = frontmatter(text)?;
    let mut replaced = BTreeSet::new();
    for key in [
        "primary_target",
        "related_targets",
        "target_path",
        "target_identity",
    ] {
        let Some(value) = values.get_mut(key) else {
            continue;
        };
        let indexed = value.is_array();
        let values = match value {
            Value::Array(a) => a.iter_mut().collect::<Vec<_>>(),
            _ => vec![value],
        };
        for (i, v) in values.into_iter().enumerate() {
            let Some(before) = v.as_str() else { continue };
            let (identity, reference) = if let Some(s) = before.strip_prefix("file:") {
                ("file:", s)
            } else {
                ("", before)
            };
            let absolute = root.to_string_lossy().replace('\\', "/");
            let normal = reference.replace('\\', "/");
            let (prefix, relative) = if let Some(s) = normal.strip_prefix(&format!("{absolute}/")) {
                (format!("{absolute}/"), s)
            } else {
                (String::new(), normal.as_str())
            };
            if !relative.starts_with(".impeccable/") {
                continue;
            }
            disk::relative(relative).map_err(|e| e.message)?;
            check(
                copied.contains(relative),
                key,
                "an explicitly validated persistent artifact with a prepared new payload; unknown/temporary references require manual recovery",
            )?;
            let after = format!(
                "{identity}{prefix}{}",
                relative.replacen(".impeccable/", ".phoenix-ui/", 1)
            );
            result.mappings.push(PathMapping {
                source_path: path.into(),
                field: Some(if indexed {
                    format!("{key}[{i}]")
                } else {
                    key.into()
                }),
                before: before.into(),
                after: after.clone(),
            });
            *v = Value::String(after);
            replaced.insert(key);
        }
    }
    if replaced.is_empty() {
        return Ok(bytes.to_vec());
    }
    if values.contains_key("target_fingerprint") {
        unconsumed(
            result,
            path,
            "target_fingerprint",
            "historical old-target fingerprint is preserved; a copied history entry confers no new acceptance or capture trust",
        );
    }
    let mut output = String::new();
    let mut in_front = false;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if i == 0 {
            in_front = true;
            output.push_str(line);
            continue;
        }
        if in_front && line.trim_end_matches(['\r', '\n']) == "---" {
            in_front = false;
            output.push_str(line);
            continue;
        }
        if in_front {
            if let Some((key, raw)) = line.split_once(':') {
                if replaced.contains(key) {
                    let leading = raw.len() - raw.trim_start().len();
                    let trailing = raw.trim_end().len();
                    output.push_str(key);
                    output.push(':');
                    output.push_str(&raw[..leading]);
                    output
                        .push_str(&serde_json::to_string(&values[key]).map_err(|e| e.to_string())?);
                    output.push_str(&raw[trailing..]);
                    continue;
                }
            }
        }
        output.push_str(line);
    }
    Ok(output.into_bytes())
}

fn provider_ids() -> Result<BTreeSet<String>> {
    let value: Value = serde_json::from_str(include_str!("../../../migration-sources/0.6.0.json"))
        .map_err(|e| Error::conflict(format!("fixed provider catalog: {e}")))?;
    Ok(value["providers"]
        .as_object()
        .ok_or_else(|| Error::conflict("fixed provider catalog missing"))?
        .keys()
        .cloned()
        .collect())
}
fn pins(root: &Path, inventory: &Inventory, result: &mut PreparedData) -> Result<()> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../test/phoenix-ui/fixtures/migration/pins.json"
    ))
    .map_err(|e| Error::conflict(format!("fixed pin templates: {e}")))?;
    let providers = provider_ids()?;
    for old in inventory
        .exclusive
        .iter()
        .filter(|f| f.kind == EntryKind::Pin)
    {
        if !old.selected {
            continue;
        }
        let key = if old.provider == ".opencode" {
            "opencodeCommand"
        } else if matches!(old.provider.as_str(), ".agents" | ".codex") {
            "codexSkill"
        } else {
            "skill"
        };
        let expected_path = if key == "opencodeCommand" {
            ".opencode/commands/impeccable-polish.md".into()
        } else {
            format!("{}/skills/polish/SKILL.md", old.provider)
        };
        let template = fixture["templates"][key]
            .as_str()
            .ok_or_else(|| Error::conflict("fixed pin template missing"))?;
        let Some(file) = read(root, &old.path, result) else {
            conflict(
                result,
                &old.path,
                ConflictKind::CustomizedFile,
                "classified pin disappeared; repeat classification",
            );
            continue;
        };
        if !providers.contains(&old.provider)
            || old.path != expected_path
            || old.sha256 != file.state.hash
            || file.bytes != template.as_bytes()
        {
            conflict(
                result,
                &old.path,
                ConflictKind::CustomizedFile,
                "pin path/parameter/current bytes differ from the complete fixed M1 template; retain the custom pin and repeat classification or resolve manually",
            );
            continue;
        }
        let new_path = if key == "opencodeCommand" {
            ".opencode/commands/phoenix-ui-polish.md".into()
        } else {
            old.path.clone()
        };
        if new_path != old.path {
            match bounded_read(root, &new_path) {
                Ok(None) => {}
                Ok(Some(_)) => {
                    conflict(
                        result,
                        &new_path,
                        ConflictKind::CustomizedFile,
                        "new pin destination already exists without ownership; preserve it and resolve manually",
                    );
                    continue;
                }
                Err(e) => {
                    conflict(result, &new_path, ConflictKind::Unreadable, &e.message);
                    continue;
                }
            }
        }
        // Branding substitution is only a generation step AFTER full-byte and
        // producer path/parameter verification, never an ownership heuristic.
        let payload = Payload::new(
            template.replace("impeccable", "phoenix-ui").into_bytes(),
            file.state.permissions,
        );
        if new_path != old.path {
            result.changes.push(Change::Exclusive {
                path: old.path.clone(),
                expected: ExpectedFile::Hash(old.sha256.clone()),
                after: None,
                rollback: RollbackPolicy::Restore,
            });
        }
        result.changes.push(Change::Exclusive {
            path: new_path.clone(),
            expected: if new_path == old.path {
                ExpectedFile::Hash(old.sha256.clone())
            } else {
                ExpectedFile::Absent
            },
            after: Some(payload.clone()),
            rollback: RollbackPolicy::Restore,
        });
        result.resolved_pin_paths.push(ResolvedPin {
            old_path: old.path.clone(),
            new_path,
            old_sha256: old.sha256.clone(),
            payload,
        });
    }
    Ok(())
}

fn source_residue(
    root: &Path,
    at: &str,
    providers: &BTreeSet<String>,
    budget: &mut (usize, u64),
    result: &mut PreparedData,
) -> Result<()> {
    let full = if at.is_empty() {
        root.to_path_buf()
    } else {
        disk::checked(root, at)?
    };
    for entry in fs::read_dir(&full).map_err(|e| Error::io(&full, e))? {
        let entry = entry.map_err(|e| Error::io(&full, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if at.is_empty() {
            name.clone()
        } else {
            format!("{at}/{name}")
        };
        budget.0 += 1;
        if budget.0 > ENTRY_LIMIT || path.split('/').count() > 32 {
            return Err(Error::conflict(
                "source-residue scan limit exceeded; no clean activation claim is available",
            ));
        }
        if matches!(
            name.as_str(),
            ".git"
                | "node_modules"
                | "vendor"
                | "target"
                | ".phoenix-ui"
                | ".impeccable"
                | ".impeccable-live"
        ) || (at.is_empty() && providers.contains(&name))
        {
            continue;
        }
        let meta = fs::symlink_metadata(entry.path()).map_err(|e| Error::io(&entry.path(), e))?;
        if meta.is_symlink() {
            result.preserved.push(Preserved {path:path.clone(),reason:"source symlink is never followed; caller must inspect any external active source".into()});
            conflict(
                result,
                &path,
                ConflictKind::Symlink,
                "source scan cannot establish absence of legacy injections through a symlink; resolve the project boundary before activation",
            );
            continue;
        }
        if meta.is_dir() {
            source_residue(root, &path, providers, budget, result)?;
            continue;
        }
        if !matches!(
            Path::new(&path).extension().and_then(|s| s.to_str()),
            Some(
                "html"
                    | "htm"
                    | "js"
                    | "mjs"
                    | "cjs"
                    | "jsx"
                    | "ts"
                    | "tsx"
                    | "svelte"
                    | "vue"
                    | "astro"
                    | "css"
            )
        ) {
            continue;
        }
        let Some(file) = read(root, &path, result) else {
            continue;
        };
        budget.1 += file.bytes.len() as u64;
        if budget.1 > TOTAL_LIMIT {
            return Err(Error::conflict(
                "source-residue byte budget exceeded; inspect/reduce the scope before activation",
            ));
        }
        let text = String::from_utf8_lossy(&file.bytes);
        if [
            "impeccable-live-start",
            "impeccable-live-end",
            "data-impeccable-csp-original",
            "impeccable-variants-start",
            "impeccable-variants-end",
            "data-impeccable-variants",
            "impeccable-carbonize-start",
            "impeccable-carbonize-end",
            "impeccable-param-values",
        ]
        .iter()
        .any(|marker| text.contains(marker))
        {
            live(result, &path);
        }
    }
    Ok(())
}
