//! Read-only preparation of verified Phoenix bundle files and project-local hooks.
//! This scope excludes derived ENGINE metadata, old data, pins and runtime receipt
//! bootstrap. The caller must add those operations before activating an install.
use crate::{
    classify::{self, Conflict, ConflictKind, EntryKind, Inventory, Preserved},
    disk, json, sha256, Change, Error, ErrorKind, ExpectedFile, Identity, JsonPatch, JsonSlot,
    JsonStep, Manifest, Payload, Permissions, Plan, Project, Result, RollbackPolicy,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
};

const ACTIVE: &str = ".phoenix-ui/install/active.json";
const RUNTIME_MANIFEST: &str = "skills/phoenix-ui/scripts/ENGINE.json";
const PROVIDERS: [&str; 19] = [
    ".agent",
    ".agents",
    ".claude",
    ".codex",
    ".cursor",
    ".dsh",
    ".gemini",
    ".github",
    ".grok",
    ".hermes",
    ".kiro",
    ".opencode",
    ".pi",
    ".qoder",
    ".rovodev",
    ".trae",
    ".trae-cn",
    ".veto",
    ".vibe",
];
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_BLOB_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 50_000;

#[derive(Debug)]
pub struct PreparedInstall {
    /// None if there are conflicts, or a verified identical install is active.
    pub plan: Option<Plan>,
    pub inventory: Inventory,
    pub conflicts: Vec<Conflict>,
    pub preserved: Vec<Preserved>,
    pub already_installed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BundleManifest {
    schema: u64,
    #[serde(rename = "derivedRuntimeManifest")]
    derived_runtime_manifest: String,
    providers: BTreeMap<String, Provider>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provider {
    files: BTreeMap<String, String>,
    #[serde(default)]
    exec: Vec<String>,
}
struct Bundle {
    manifest: BundleManifest,
    blobs: BTreeMap<String, Vec<u8>>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidPlan, message)
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn plain_directory(path: &Path) -> Result<()> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| Error::io(path, e))?
            .join(path)
    };
    // Checking ancestors also rejects a bundle reached through a symlink.
    for ancestor in full.ancestors() {
        let meta = fs::symlink_metadata(ancestor).map_err(|e| Error::io(ancestor, e))?;
        if meta.is_symlink() || !meta.is_dir() {
            return Err(invalid(format!(
                "bundle requires plain directories: {}",
                ancestor.display()
            )));
        }
    }
    Ok(())
}
fn read_bundle_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let meta = fs::symlink_metadata(path).map_err(|e| Error::io(path, e))?;
    if meta.is_symlink() || !meta.is_file() || meta.len() > limit {
        return Err(invalid(format!(
            "bundle requires a bounded regular file: {}",
            path.display()
        )));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(|e| Error::io(path, e))?;
    let opened = file.metadata().map_err(|e| Error::io(path, e))?;
    if !opened.is_file() || opened.len() != meta.len() {
        return Err(invalid(format!("bundle file changed: {}", path.display())));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::io(path, e))?;
    if bytes.len() as u64 != meta.len() || bytes.len() as u64 > limit {
        return Err(invalid(format!(
            "bundle file size changed or exceeds limit: {}",
            path.display()
        )));
    }
    Ok(bytes)
}
impl Bundle {
    fn load(root: &Path, expected_sha: &str) -> Result<Self> {
        plain_directory(root)?;
        plain_directory(&root.join("blobs"))?;
        let bytes = read_bundle_file(&root.join("manifest.json"), MAX_MANIFEST_BYTES)?;
        if !valid_sha(expected_sha) || sha256(&bytes) != expected_sha {
            return Err(invalid("bundle manifest SHA-256 mismatch"));
        }
        // Validate duplicate keys before serde's map deserializer can erase them.
        json::parse_value(&bytes).map_err(|e| invalid(format!("bundle manifest: {e}")))?;
        let manifest: BundleManifest = serde_json::from_slice(&bytes)
            .map_err(|e| invalid(format!("invalid bundle manifest: {e}")))?;
        if manifest.schema != 1
            || manifest.derived_runtime_manifest != RUNTIME_MANIFEST
            || manifest.providers.is_empty()
        {
            return Err(invalid("unsupported bundle schema or runtime manifest"));
        }
        let mut referenced = BTreeSet::new();
        let mut files = 0;
        for (provider, entry) in &manifest.providers {
            if !PROVIDERS.contains(&provider.as_str()) || entry.files.is_empty() {
                return Err(invalid(format!("unsupported bundle provider: {provider}")));
            }
            let mut paths = BTreeSet::new();
            for (path, sha) in &entry.files {
                disk::relative(path)?;
                files += 1;
                if files > MAX_FILES
                    || !valid_sha(sha)
                    || path.eq_ignore_ascii_case(RUNTIME_MANIFEST)
                    || !paths.insert(path.to_lowercase())
                    || (provider == ".codex" && path.starts_with("skills/"))
                {
                    return Err(invalid(format!("unsafe bundle file: {provider}/{path}")));
                }
                referenced.insert(sha.clone());
            }
            for path in &paths {
                let parts: Vec<_> = path.split('/').collect();
                for count in 1..parts.len() {
                    if paths.contains(&parts[..count].join("/")) {
                        return Err(invalid(format!(
                            "bundle file/directory collision: {provider}/{path}"
                        )));
                    }
                }
            }
            let mut exec = BTreeSet::new();
            if entry
                .exec
                .iter()
                .any(|path| !entry.files.contains_key(path) || !exec.insert(path))
            {
                return Err(invalid(format!("invalid executable list: {provider}")));
            }
        }
        // Schema 1 has no declared lengths: verify the read against metadata,
        // bound the total, and hash every blob, including unreferenced entries.
        for entry in fs::read_dir(root.join("blobs")).map_err(|e| Error::io(root, e))? {
            let path = entry.map_err(|e| Error::io(root, e))?.path();
            let name = path
                .file_name()
                .and_then(|v| v.to_str())
                .ok_or_else(|| invalid("invalid bundle blob filename"))?;
            if !valid_sha(name) {
                return Err(invalid(format!("invalid bundle blob filename: {name}")));
            }
            referenced.insert(name.into());
        }
        if referenced.len() > MAX_FILES {
            return Err(invalid("too many bundle blobs"));
        }
        let mut total = 0u64;
        let mut blobs = BTreeMap::new();
        for sha in referenced {
            let bytes = read_bundle_file(&root.join("blobs").join(&sha), MAX_BLOB_BYTES)?;
            total = total
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| invalid("bundle size overflow"))?;
            if total > MAX_TOTAL_BYTES || sha256(&bytes) != sha {
                return Err(invalid(format!("bundle blob hash/size mismatch: {sha}")));
            }
            blobs.insert(sha, bytes);
        }
        Ok(Self { manifest, blobs })
    }
}

fn shared_path(provider: &str, path: &str) -> bool {
    matches!(
        (provider, path),
        (".claude" | ".gemini", "settings.json")
            | (".codex" | ".cursor", "hooks.json")
            | (".github" | ".grok", "hooks/phoenix-ui.json")
    )
}
fn snapshot(project: &Project, path: &str) -> Result<Option<disk::Snapshot>> {
    let full = disk::checked(project.root(), path)?;
    match fs::symlink_metadata(&full) {
        Ok(meta) if !meta.is_file() => {
            return Err(Error::conflict(format!(
                "not a regular project file: {path}"
            )))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::io(&full, e)),
        _ => {}
    }
    disk::read(project.root(), path)
}
fn expected(current: Option<&disk::Snapshot>) -> ExpectedFile {
    current
        .map(|s| ExpectedFile::Hash(s.state.hash.clone()))
        .unwrap_or(ExpectedFile::Absent)
}
fn report(prepared: &mut PreparedInstall, path: &str, provider: Option<&str>, error: Error) {
    prepared.conflicts.push(Conflict {
        path: path.into(),
        provider: provider.map(str::to_string),
        kind: ConflictKind::CustomizedFile,
        message: error.message,
    });
}
fn identity_matches(value: &Value, identity: &Identity) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    identity
        .required_keys
        .iter()
        .all(|k| object.contains_key(k))
        && identity
            .fields
            .iter()
            .all(|(key, expected)| match (object.get(key), expected) {
                (None, None) => true,
                (Some(actual), Some(expected)) => actual == expected,
                _ => false,
            })
}

#[derive(Clone)]
struct Handler {
    parent: Vec<JsonStep>,
    value: Value,
}
fn bundle_handlers(value: &Value) -> Result<Vec<Handler>> {
    let events = value
        .get("hooks")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("bundle hooks must be an object"))?;
    let mut handlers = Vec::new();
    for (event, entries) in events {
        let entries = entries
            .as_array()
            .ok_or_else(|| invalid("bundle hook event must be an array"))?;
        for outer in entries {
            let outer_object = outer
                .as_object()
                .ok_or_else(|| invalid("bundle hook must be an object"))?;
            if let Some(nested) = outer_object.get("hooks") {
                let nested = nested
                    .as_array()
                    .ok_or_else(|| invalid("bundle nested hooks must be an array"))?;
                if !nested.iter().all(Value::is_object) {
                    return Err(invalid("bundle handler must be an object"));
                }
            }
            // Own complete groups, as the upstream merge appends them. Nesting into
            // a neighbor's group with the same matcher (often none) is ambiguous.
            handlers.push(Handler {
                parent: vec![JsonStep::Key("hooks".into()), JsonStep::Key(event.clone())],
                value: outer.clone(),
            });
        }
    }
    Ok(handlers)
}
fn handler_values<'a>(root: &'a Value, parent: &[JsonStep]) -> Result<Option<&'a Vec<Value>>> {
    let mut at = root;
    for step in parent {
        match step {
            JsonStep::Key(key) => {
                let object = at
                    .as_object()
                    .ok_or_else(|| Error::conflict("hook parent is not an object"))?;
                let Some(next) = object.get(key) else {
                    return Ok(None);
                };
                at = next;
            }
            JsonStep::Match(identity) => {
                let array = at
                    .as_array()
                    .ok_or_else(|| Error::conflict("hook event is not an array"))?;
                let mut matches = array.iter().filter(|v| identity_matches(v, identity));
                let Some(next) = matches.next() else {
                    return Ok(None);
                };
                if matches.next().is_some() {
                    return Err(Error::conflict("ambiguous hook matcher identity"));
                }
                at = next;
            }
        }
    }
    at.as_array()
        .map(Some)
        .ok_or_else(|| Error::conflict("hook handlers are not an array"))
}
fn phoenix_command(value: &Value) -> bool {
    ["command", "commandWindows", "bash", "powershell"]
        .iter()
        .any(|key| {
            value.get(*key).and_then(Value::as_str).is_some_and(|text| {
                text.contains("skills/phoenix-ui/scripts/")
                    || text.contains("skills\\phoenix-ui\\scripts\\")
                    || text.starts_with("phoenix-ui ")
            })
        })
}
fn phoenix_handler(value: &Value) -> bool {
    phoenix_command(value)
        || value
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|s| s.starts_with("phoenix-ui-"))
        || value
            .get("hooks")
            .and_then(Value::as_array)
            .is_some_and(|nested| nested.iter().any(phoenix_handler))
}
fn check_current_hooks(root: &Value, owned: &[JsonPatch]) -> Result<()> {
    let Some(hooks) = root.get("hooks") else {
        return Ok(());
    };
    let events = hooks
        .as_object()
        .ok_or_else(|| Error::conflict("shared hooks must be an object"))?;
    for (event, array) in events {
        let array = array
            .as_array()
            .ok_or_else(|| Error::conflict(format!("hook event {event} is not an array")))?;
        let base = vec![JsonStep::Key("hooks".into()), JsonStep::Key(event.clone())];
        for outer in array {
            if !outer.is_object() {
                return Err(Error::conflict("shared hook entry must be an object"));
            }
            if let Some(nested) = outer.get("hooks") {
                let nested = nested
                    .as_array()
                    .ok_or_else(|| Error::conflict("nested hook handlers must be an array"))?;
                if !nested.iter().all(Value::is_object) {
                    return Err(Error::conflict("shared handler must be an object"));
                }
            }
            if !phoenix_handler(outer) {
                continue;
            }
            // Phoenix owns whole groups; a Phoenix handler anywhere else, or a
            // Phoenix group someone edited, is not ours to change.
            let recognized = owned.iter().any(|patch| {
                patch.after.as_ref() == Some(outer)
                    && matches!(patch.slot, JsonSlot::Element(_))
                    && patch.parent == base
            });
            if !recognized {
                return Err(Error::conflict(format!("event {event}: Phoenix-looking handler lacks complete verified receipt ownership; preserve it")));
            }
        }
    }
    Ok(())
}

/// `manifest.sha256` is the digest of this bundle's manifest.json, not an npm
/// or engine release manifest. Providers are exact directory IDs; the caller
/// expands Codex into `.agents` and `.codex` and chooses shared/local routing.
/// Conflicts return an inventory but no executable plan. All reads and kernel
/// preflight finish before returning; no project staging or state is created.
pub fn build_install_plan(
    project: &Project,
    bundle_root: &Path,
    providers: &[String],
    manifest: Manifest,
    transaction_id: &str,
) -> Result<PreparedInstall> {
    build_install_plan_with(
        project,
        bundle_root,
        providers,
        manifest,
        transaction_id,
        false,
        false,
    )
}

/// With `keep_complete`, a verified identical bundle install still returns its
/// plan (and `already_installed`), so a caller can carry its own ownership forward.
pub fn build_install_plan_with(
    project: &Project,
    bundle_root: &Path,
    providers: &[String],
    mut manifest: Manifest,
    transaction_id: &str,
    keep_complete: bool,
    keep_impeccable: bool,
) -> Result<PreparedInstall> {
    if transaction_id.is_empty()
        || transaction_id.len() > 80
        || !transaction_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(invalid(
            "transaction ID must contain 1..80 ASCII letters, digits, '-' or '_'",
        ));
    }
    let selected: BTreeSet<_> = providers.iter().cloned().collect();
    let stated: BTreeSet<_> = manifest.providers.iter().cloned().collect();
    if selected.is_empty()
        || selected.len() != providers.len()
        || stated.len() != manifest.providers.len()
        || selected != stated
        || manifest.version.trim().is_empty()
    {
        return Err(invalid(
            "providers must be nonempty, unique and match the install manifest",
        ));
    }
    let bundle = Bundle::load(bundle_root, &manifest.sha256)?;
    if selected
        .iter()
        .any(|p| !bundle.manifest.providers.contains_key(p))
    {
        return Err(invalid("unknown selected bundle provider"));
    }
    let verified = project.verified_installation();
    let owned: BTreeMap<String, String> = match &verified {
        Ok(Some(active)) => active
            .receipt
            .ownership
            .iter()
            .filter(|o| !o.shared_entries)
            .filter_map(|o| o.sha256.clone().map(|sha| (o.path.clone(), sha)))
            .collect(),
        _ => BTreeMap::new(),
    };
    let inventory = if keep_impeccable {
        Inventory::default()
    } else {
        classify::classify_project_with(project.root(), providers, &owned)?
    };
    let mut prepared = PreparedInstall {
        conflicts: inventory.conflicts.clone(),
        preserved: inventory.preserved.clone(),
        inventory,
        plan: None,
        already_installed: false,
    };
    let active = match verified {
        Ok(value) => value,
        Err(error) if error.kind == ErrorKind::Conflict => {
            report(&mut prepared, ACTIVE, None, error);
            return Ok(prepared);
        }
        Err(error) => return Err(error),
    };
    if !prepared.conflicts.is_empty() {
        return Ok(prepared);
    }
    if let Some(active) = &active {
        for provider in active
            .receipt
            .manifest
            .providers
            .iter()
            .filter(|p| !selected.contains(*p))
        {
            prepared.conflicts.push(Conflict {
                path: provider.clone(),
                provider: Some(provider.clone()),
                kind: ConflictKind::OutsideSelection,
                message:
                    "include this active receipt provider so its installed ownership is retained"
                        .into(),
            });
        }
        if !prepared.conflicts.is_empty() {
            return Ok(prepared);
        }
    }
    manifest.providers = selected.iter().cloned().collect();
    let mut complete = active
        .as_ref()
        .is_some_and(|r| r.receipt.manifest == manifest);
    let mut changes = BTreeMap::new();
    for provider in &selected {
        let entry = &bundle.manifest.providers[provider];
        for (relative, sha) in &entry.files {
            let path = format!("{provider}/{relative}");
            let bytes = &bundle.blobs[sha];
            let current = match snapshot(project, &path) {
                Ok(current) => current,
                Err(error) => {
                    report(&mut prepared, &path, Some(provider), error);
                    continue;
                }
            };
            if shared_path(provider, relative) {
                match prepare_shared(
                    &path,
                    provider,
                    bytes,
                    current.as_ref(),
                    &prepared.inventory,
                    active
                        .as_ref()
                        .and_then(|a| a.json_patches.get(&path))
                        .map(Vec::as_slice),
                ) {
                    Ok(patches) => {
                        let prior = active.as_ref().and_then(|a| a.json_patches.get(&path));
                        complete &= current.is_some()
                            && patches.iter().filter(|p| p.after.is_some()).all(|patch| {
                                prior.is_some_and(|prior| {
                                    prior.iter().any(|p| {
                                        p.parent == patch.parent
                                            && p.after == patch.after
                                            && match (&p.slot, &patch.slot) {
                                                (JsonSlot::Member(a), JsonSlot::Member(b)) => {
                                                    a == b
                                                }
                                                (JsonSlot::Element(_), JsonSlot::Element(_)) => {
                                                    true
                                                }
                                                _ => false,
                                            }
                                    })
                                })
                            });
                        changes.insert(
                            path.clone(),
                            Change::SharedJson {
                                path,
                                expected: expected(current.as_ref()),
                                patches,
                            },
                        );
                    }
                    Err(error) => report(&mut prepared, &path, Some(provider), error),
                }
            } else {
                let owned = active.as_ref().is_some_and(|r| {
                    r.receipt.ownership.iter().any(|o| {
                        o.path == path
                            && !o.shared_entries
                            && o.sha256.is_some()
                            && o.rollback == RollbackPolicy::Restore
                    })
                });
                if current.is_some() && !owned {
                    report(&mut prepared, &path, Some(provider), Error::conflict("existing Phoenix target has no verified exclusive receipt ownership; preserve its bytes"));
                    continue;
                }
                let permissions = Permissions::file(if entry.exec.contains(relative) {
                    0o755
                } else {
                    0o644
                });
                complete &= owned
                    && current.as_ref().is_some_and(|s| {
                        &s.state.hash == sha && s.state.permissions == permissions
                    });
                changes.insert(
                    path.clone(),
                    Change::Exclusive {
                        path,
                        expected: expected(current.as_ref()),
                        after: Some(Payload {
                            bytes: bytes.clone(),
                            sha256: sha.clone(),
                            permissions,
                        }),
                        rollback: RollbackPolicy::Restore,
                    },
                );
            }
        }
    }
    // Retire only complete fixed old files. Pins require caller regeneration.
    for old in &prepared.inventory.exclusive {
        if old.kind == EntryKind::Pin {
            prepared.preserved.push(Preserved {
                path: old.path.clone(),
                reason: "fixed old pin needs caller's Phoenix pin reconstruction before activation"
                    .into(),
            });
        } else if old.selected {
            changes.insert(
                old.path.clone(),
                Change::Exclusive {
                    path: old.path.clone(),
                    expected: ExpectedFile::Hash(old.sha256.clone()),
                    after: None,
                    rollback: RollbackPolicy::Restore,
                },
            );
        }
    }
    // Legacy-only shared paths include .claude/settings.local.json and the old
    // per-tool hook files. They retain their outer containers and neighbors.
    for shared in &prepared.inventory.shared {
        if changes.contains_key(&shared.path) {
            continue;
        }
        let patches: Vec<_> = shared
            .handlers
            .iter()
            .filter(|h| h.selected)
            .map(|h| JsonPatch {
                parent: h.parent.clone(),
                slot: h.slot.clone(),
                before: Some(h.value.clone()),
                after: None,
            })
            .collect();
        if !patches.is_empty() {
            changes.insert(
                shared.path.clone(),
                Change::SharedJson {
                    path: shared.path.clone(),
                    expected: ExpectedFile::Hash(shared.sha256.clone()),
                    patches,
                },
            );
        }
    }
    if let Some(active) = &active {
        for ownership in active
            .receipt
            .ownership
            .iter()
            .filter(|o| !changes.contains_key(&o.path))
        {
            prepared.preserved.push(Preserved { path: ownership.path.clone(), reason: "active receipt ownership outside bundle scope needs caller's ownership carry-forward".into() });
        }
    }
    if !prepared.conflicts.is_empty() {
        return Ok(prepared);
    }
    if complete
        && !prepared
            .inventory
            .exclusive
            .iter()
            .any(|f| f.kind == EntryKind::File)
        && !prepared
            .inventory
            .shared
            .iter()
            .any(|s| !s.handlers.is_empty())
    {
        prepared.already_installed = true;
        if !keep_complete {
            return Ok(prepared);
        }
    }
    let plan = Plan {
        transaction_id: transaction_id.into(),
        manifest,
        expected_receipt: active
            .as_ref()
            .map(|a| ExpectedFile::Hash(a.receipt_sha256.clone()))
            .unwrap_or(ExpectedFile::Absent),
        changes: changes.into_values().collect(),
    };
    match project.preflight(&plan) {
        Ok(_) => prepared.plan = Some(plan),
        Err(error) if error.kind == ErrorKind::Conflict => {
            report(&mut prepared, "project preflight", None, error)
        }
        Err(error) => return Err(error),
    }
    Ok(prepared)
}

fn prepare_shared(
    path: &str,
    _provider: &str,
    bytes: &[u8],
    current: Option<&disk::Snapshot>,
    inventory: &Inventory,
    owned: Option<&[JsonPatch]>,
) -> Result<Vec<JsonPatch>> {
    let new = json::parse_value(bytes).map_err(|e| invalid(format!("bundle hooks {path}: {e}")))?;
    let old = current
        .map(|s| json::parse_value(&s.bytes))
        .transpose()?
        .unwrap_or_else(|| serde_json::json!({}));
    if !old.is_object() {
        return Err(Error::conflict("shared settings root must be an object"));
    }
    let handlers = bundle_handlers(&new)?;
    let owned = owned.unwrap_or_default();
    check_current_hooks(&old, owned)?;
    let mut patches: Vec<_> = inventory
        .shared
        .iter()
        .filter(|s| s.path == path)
        .flat_map(|s| s.handlers.iter().filter(|h| h.selected))
        .map(|h| JsonPatch {
            parent: h.parent.clone(),
            slot: h.slot.clone(),
            before: Some(h.value.clone()),
            after: None,
        })
        .collect();
    for (key, value) in new
        .as_object()
        .ok_or_else(|| invalid("bundle hook root is not an object"))?
    {
        if key == "hooks" {
            continue;
        }
        match old.get(key) {
            Some(existing)
                if owned.iter().any(|p| {
                    p.parent.is_empty()
                        && p.slot == JsonSlot::Member(key.clone())
                        && p.after.as_ref() == Some(existing)
                }) =>
            {
                patches.push(JsonPatch {
                    parent: Vec::new(),
                    slot: JsonSlot::Member(key.clone()),
                    before: Some(existing.clone()),
                    after: Some(value.clone()),
                })
            }
            None => patches.push(JsonPatch {
                parent: Vec::new(),
                slot: JsonSlot::Member(key.clone()),
                before: None,
                after: Some(value.clone()),
            }),
            Some(existing) if key == "version" && existing != value => {
                return Err(Error::conflict(
                    "shared hook version differs from bundle schema; preserve it",
                ))
            }
            _ => {}
        }
    }
    let prior: Vec<_> = owned
        .iter()
        .filter(|p| p.after.as_ref().is_some_and(phoenix_handler))
        .collect();
    for patch in &prior {
        if !matches!(patch.slot, JsonSlot::Element(_))
            || !matches!(patch.parent.first(),Some(JsonStep::Key(key)) if key=="hooks")
        {
            return Err(Error::conflict(
                "Phoenix entry has unsupported receipt patch shape",
            ));
        }
    }
    let mut used = BTreeSet::new();
    for handler in handlers {
        let candidates: Vec<_> = prior
            .iter()
            .enumerate()
            .filter(|(_, p)| p.parent == handler.parent)
            .collect();
        if candidates.len() > 1 {
            return Err(Error::conflict("multiple receipt-owned Phoenix entries share one new handler identity; retain and resolve explicitly"));
        }
        let (slot, before) = if let Some((index, previous)) = candidates.first() {
            if !used.insert(*index) {
                return Err(invalid(
                    "multiple bundle handlers share one receipt entry identity",
                ));
            }
            (previous.slot.clone(), previous.after.clone())
        } else {
            if handler_values(&old, &handler.parent)?
                .is_some_and(|values| values.contains(&handler.value))
            {
                return Err(Error::conflict(
                    "complete Phoenix handler already exists without matching verified ownership",
                ));
            }
            (JsonSlot::Element(usize::MAX), None)
        };
        // Equal complete values are verified ownership assertions. The kernel
        // retains their original text while carrying their inverse guard forward.
        patches.push(JsonPatch {
            parent: handler.parent,
            slot,
            before,
            after: Some(handler.value),
        });
    }
    for (_, previous) in prior.iter().enumerate().filter(|(i, _)| !used.contains(i)) {
        patches.push(JsonPatch {
            parent: previous.parent.clone(),
            slot: previous.slot.clone(),
            before: previous.after.clone(),
            after: None,
        });
    }
    Ok(patches)
}
