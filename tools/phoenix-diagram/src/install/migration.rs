//! Explicit project-only Archify removal. Frozen source equality never grants
//! ownership; confirmation and durable physical snapshots authorize this scope.
use super::bundle::{self, parse};
use super::disk::{self, Directory, Physical};
use super::plan::{self, Prepared};
use super::state::{self, Active, Journal, Operation, Phase};
use super::transaction as tx;
use super::{
    Checkpoint, InstallConflict, InstallReceipt, InstallRequest, InstallResult, InstallStatus,
    LegacyRemoval, MigrationMode, MigrationRequest, Observer,
};
use crate::output::FileIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

const SOURCES: &str = include_str!("../../migration-sources/archify-v1.json");
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    schema_version: u32,
    sources: Vec<Source>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Source {
    id: String,
    file_count: usize,
    files: Vec<SourceFile>,
}
#[derive(Deserialize)]
struct SourceFile {
    path: String,
    bytes: u64,
    mode: u32,
    sha256: String,
}
fn catalog() -> InstallResult<Catalog> {
    let value: Catalog = parse(SOURCES.as_bytes(), "install/migration-source-invalid")?;
    let expected = [
        ("legacy-2.17", 219),
        ("legacy-2.17-when-to-use", 219),
        ("reference-3.1.0", 157),
    ];
    if value.schema_version != 1
        || value.sources.len() != expected.len()
        || value
            .sources
            .iter()
            .zip(expected)
            .any(|(source, (id, count))| {
                source.id != id
                    || source.file_count != count
                    || source.files.len() != count
                    || source.files.iter().any(|f| {
                        disk::relative(&f.path).is_err()
                            || ![0o644, 0o755].contains(&f.mode)
                            || !bundle::valid_hash(&f.sha256)
                    })
            })
    {
        return Err(super::invalid(
            "install/migration-source-invalid",
            "Frozen complete migration sources differ from the declared catalog",
        ));
    }
    Ok(value)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LegacyEntry {
    path: String,
    physical: Physical,
    mode: u32,
    file: Option<FileIdentity>,
    temporary: String,
    removed: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LegacyRoot {
    kind: String,
    path: String,
    source: String,
    parent: Physical,
    entries: Vec<LegacyEntry>,
    pending: Option<usize>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Migration {
    confirmed: bool,
    roots: Vec<LegacyRoot>,
    parents: BTreeMap<String, Physical>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Reverse {
    index: usize,
    restored: Option<FileIdentity>,
    done: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Compatibility {
    journal: Box<Journal>,
    after: FileIdentity,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rollback {
    completed: usize,
    stage_directory: String,
    stage_physical: Physical,
    journal_parent: Physical,
    operations: Vec<Reverse>,
    restored_active: Option<Active>,
    previous_journal: Option<Box<Journal>>,
    previous_journal_identity: Option<FileIdentity>,
    previous_journal_after: Option<FileIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    compatibility: Option<Compatibility>,
}
impl Rollback {
    pub fn validate(&self, project: &Arc<Directory>, journal: &Journal) -> InstallResult<()> {
        if !self
            .stage_directory
            .strip_prefix("rollback-")
            .is_some_and(state::valid_id)
            || self.stage_physical.device != project.physical.device
            || self.operations.len() != journal.operations.len()
            || self.completed > self.operations.len()
            || self.operations.iter().enumerate().any(|(i, r)| {
                r.index != i
                    || r.restored.as_ref().is_some_and(|id| {
                        !state::valid_identity(id, i + 1 == journal.operations.len())
                    })
            })
        {
            return Err(disk::conflict(
                "install/invalid-rollback",
                &journal.transaction_id,
            ));
        }
        if !matches!(journal.phase, Phase::RollingBack | Phase::RolledBack)
            || self.completed != self.operations.iter().filter(|r| r.done).count()
        {
            return Err(disk::conflict(
                "install/invalid-rollback",
                &journal.transaction_id,
            ));
        }
        for (i, (reverse, operation)) in self.operations.iter().zip(&journal.operations).enumerate()
        {
            if i + 1 != self.operations.len() {
                match (&reverse.restored, &operation.before) {
                    (Some(restored), Some(before))
                        if restored.bytes == before.bytes
                            && restored.sha256 == before.sha256
                            && restored.mode == before.mode => {}
                    (None, None) => {}
                    _ => return Err(disk::conflict("install/invalid-rollback", &operation.path)),
                }
            }
        }
        let transaction = tx::directory(project, &journal.transaction_id)?;
        let stage = tx::subdirectory(&transaction, &self.stage_directory)?;
        if stage.physical != self.stage_physical {
            return Err(disk::conflict(
                "install/rollback-stage-changed",
                &self.stage_directory,
            ));
        }
        let parent = project
            .descend(state::JOURNALS)?
            .ok_or_else(|| disk::conflict("install/missing-journals", state::JOURNALS))?;
        if parent.physical != self.journal_parent {
            return Err(disk::conflict(
                "install/rollback-journal-parent-changed",
                state::JOURNALS,
            ));
        }
        let last = journal.operations.len() - 1;
        if let Some(active) = &self.restored_active {
            active.validate(project)?;
            let before = tx::subdirectory(&transaction, "before")?;
            let bytes = disk::expected(
                &before,
                &last.to_string(),
                journal.operations[last]
                    .backup
                    .as_ref()
                    .ok_or_else(|| disk::conflict("install/invalid-rollback", state::ACTIVE))?,
                &[1],
            )?
            .bytes;
            let original: Active = parse(&bytes, "install/invalid-before-active")?;
            let mut semantics = active.clone();
            semantics.ownership = original.ownership.clone();
            semantics.plan_sha256 = original.plan_sha256.clone();
            semantics.seal()?;
            if state::encoded(&semantics)? != state::encoded(&original)? {
                return Err(disk::conflict(
                    "install/rollback-before-active-changed",
                    state::ACTIVE,
                ));
            }
            for owned in &active.ownership {
                let expected = journal
                    .operations
                    .iter()
                    .enumerate()
                    .find(|(_, op)| op.path == owned.path)
                    .map(|(i, _)| self.operations[i].restored.as_ref())
                    .unwrap_or_else(|| {
                        original
                            .ownership
                            .iter()
                            .find(|o| o.path == owned.path)
                            .map(|o| &o.identity)
                    });
                if expected != Some(&owned.identity) {
                    return Err(disk::conflict("install/rollback-ownership", &owned.path));
                }
            }
            let old = self
                .previous_journal
                .as_ref()
                .ok_or_else(|| disk::conflict("install/invalid-rollback", state::ACTIVE))?;
            if old.transaction_id != active.transaction_id
                || old.phase != Phase::Committed
                || old.rollback.is_some()
                || state::encoded(&old.active)? != state::encoded(active)?
                || self.previous_journal_identity.is_none()
            {
                return Err(disk::conflict("install/invalid-rollback", state::ACTIVE));
            }
            old.validate(project, &tx::directory(project, &old.transaction_id)?)?;
            let after = self
                .previous_journal_after
                .as_ref()
                .ok_or_else(|| disk::conflict("install/invalid-rollback", state::JOURNALS))?;
            let before = self
                .previous_journal_identity
                .as_ref()
                .ok_or_else(|| disk::conflict("install/invalid-rollback", state::JOURNALS))?;
            let bytes = state::encoded(old)?;
            if !state::valid_identity(after, true)
                || !state::valid_identity(before, true)
                || after.sha256 != bundle::hash(&bytes)
                || after.bytes != bytes.len() as u64
            {
                return Err(disk::conflict("install/invalid-rollback", state::JOURNALS));
            }
            let restored = self.operations[last]
                .restored
                .as_ref()
                .ok_or_else(|| disk::conflict("install/invalid-rollback", state::ACTIVE))?;
            let bytes = state::encoded(active)?;
            if restored.bytes != bytes.len() as u64 || restored.sha256 != bundle::hash(&bytes) {
                return Err(disk::conflict("install/invalid-rollback", state::ACTIVE));
            }
        } else if journal.operations[last].before.is_some()
            || self.operations[last].restored.is_some()
            || self.previous_journal.is_some()
            || self.previous_journal_identity.is_some()
            || self.previous_journal_after.is_some()
        {
            return Err(disk::conflict("install/invalid-rollback", state::ACTIVE));
        }
        if let Some(compatibility) = &self.compatibility {
            if self.restored_active.is_some() || journal.active.schema_version != 1 {
                return Err(disk::conflict(
                    "install/invalid-compatibility",
                    &journal.transaction_id,
                ));
            }
            let mut expected = journal.clone();
            expected.schema_version = 1;
            expected.phase = Phase::Recovered;
            expected.migration = None;
            expected.rollback = None;
            for operation in &mut expected.operations {
                operation.kind = None;
            }
            expected.seal()?;
            if state::encoded(&expected)? != state::encoded(&compatibility.journal)?
                || !state::valid_identity(&compatibility.after, true)
            {
                return Err(disk::conflict(
                    "install/invalid-compatibility",
                    &journal.transaction_id,
                ));
            }
            compatibility.journal.validate(project, &transaction)?;
            let bytes = state::encoded(&compatibility.journal)?;
            if compatibility.after.sha256 != bundle::hash(&bytes)
                || compatibility.after.bytes != bytes.len() as u64
            {
                return Err(disk::conflict(
                    "install/invalid-compatibility",
                    &journal.transaction_id,
                ));
            }
        }
        Ok(())
    }
}
impl Migration {
    pub fn validate(&self, _project: &Arc<Directory>, journal: &Journal) -> InstallResult<()> {
        let catalog = catalog()?;
        let mut paths = BTreeSet::new();
        let mut remaining = false;
        let mut pending = false;
        for root in &self.roots {
            for (i, entry) in root.entries.iter().enumerate() {
                let was_remaining = remaining;
                if entry.removed && remaining {
                    return Err(disk::conflict(
                        "install/invalid-legacy-progress",
                        &entry.path,
                    ));
                }
                if !entry.removed {
                    remaining = true;
                }
                if root.pending == Some(i) {
                    if pending
                        || was_remaining
                        || entry.removed
                        || root.entries.iter().take(i).any(|e| !e.removed)
                    {
                        return Err(disk::conflict(
                            "install/invalid-legacy-progress",
                            &entry.path,
                        ));
                    }
                    pending = true;
                }
            }
        }
        if matches!(
            journal.phase,
            Phase::Committed | Phase::RollingBack | Phase::RolledBack
        ) && self
            .roots
            .iter()
            .any(|r| r.pending.is_some() || r.entries.iter().any(|e| !e.removed))
        {
            return Err(disk::conflict(
                "install/incomplete-legacy-removal",
                &journal.transaction_id,
            ));
        }
        if journal.operations.iter().any(|op| {
            op.path
                .rsplit_once('/')
                .is_none_or(|(parent, _)| !self.parents.contains_key(parent))
        }) || self.parents.keys().any(|path| {
            disk::relative(path).is_err()
                || !(path == state::STATE
                    || path == ".phoenix-diagram"
                    || journal.active.providers.iter().any(|p| {
                        path == p
                            || path == &format!("{p}/skills")
                            || path == &format!("{p}/skills/phoenix-diagram")
                            || path.starts_with(&format!("{p}/skills/phoenix-diagram/"))
                    }))
        }) {
            return Err(disk::conflict(
                "install/invalid-parent-bindings",
                &journal.transaction_id,
            ));
        }
        if !self.confirmed {
            return Err(disk::conflict(
                "install/unconfirmed-journal",
                &journal.transaction_id,
            ));
        }
        for root in &self.roots {
            let provider = root.path.split('/').next().unwrap_or("");
            let source = catalog
                .sources
                .iter()
                .find(|s| s.id == root.source)
                .ok_or_else(|| disk::conflict("install/unknown-migration-source", &root.source))?;
            if root.kind != "legacy-root-remove"
                || !journal.active.providers.iter().any(|p| p == provider)
                || root.path != format!("{provider}/skills/archify")
                || !plan::PROVIDERS.contains(&provider)
                || !paths.insert(&root.path)
                || root.pending.is_some_and(|i| i >= root.entries.len())
            {
                return Err(disk::conflict("install/invalid-legacy-root", &root.path));
            }
            let files: BTreeMap<_, _> = root
                .entries
                .iter()
                .filter_map(|e| e.file.as_ref().map(|id| (&e.path, id)))
                .collect();
            let mut directories = BTreeSet::from([root.path.clone()]);
            for file in &source.files {
                let path = format!("{}/{}", root.path, file.path);
                let Some(id) = files.get(&path) else {
                    return Err(disk::conflict("install/incomplete-legacy-snapshot", path));
                };
                if !state::valid_identity(id, false)
                    || id.mode != file.mode
                    || id.bytes != file.bytes
                    || id.sha256 != file.sha256
                {
                    return Err(disk::conflict("install/invalid-legacy-snapshot", path));
                }
                let mut parent = path.as_str();
                while let Some((next, _)) = parent.rsplit_once('/') {
                    if next.len() < root.path.len() {
                        break;
                    }
                    directories.insert(next.into());
                    parent = next;
                }
            }
            if files.len() != source.files.len()
                || root.entries.len() != files.len() + directories.len()
            {
                return Err(disk::conflict(
                    "install/incomplete-legacy-snapshot",
                    &root.path,
                ));
            }
            let mut names = BTreeSet::new();
            for (index, entry) in root.entries.iter().enumerate() {
                if !names.insert(&entry.path)
                    || disk::relative(&entry.path).is_err()
                    || entry.physical.device != journal.project.device
                    || entry.temporary != temporary(&entry.physical, index)
                    || entry.file.as_ref().is_some_and(|id| {
                        id.device != entry.physical.device
                            || id.inode != entry.physical.inode
                            || id.mode != entry.mode
                    })
                    || (entry.file.is_none() && !directories.contains(&entry.path))
                {
                    return Err(disk::conflict(
                        "install/invalid-legacy-snapshot",
                        &entry.path,
                    ));
                }
            }
        }
        Ok(())
    }
}
fn temporary(physical: &Physical, index: usize) -> String {
    format!(".phoenix-diagram-remove-{}-{index}", physical.inode)
}
fn reinstall(source: &str) -> String {
    if source == "reference-3.1.0" {
        "reference-3.1.0: explicitly reinstall the complete archify subtree from archify-3.1.0-source.tar.gz (SHA256 6831ba7f7d9d5c0fd473e317bd9b513f2581b9d4d63bd157d1de4680af3b32a4); its upstream commit is unverified.".into()
    } else {
        format!("{source}: explicitly reinstall complete skills/archify from https://github.com/XGENT-ai/skills.git at 4a65a1525f9ed03c775a0e18935901e1edbb773f{}; use the frozen migration catalog to verify the complete 219-file source.",if source=="legacy-2.17-when-to-use" {" with the exact when_to_use patch recorded in the catalog"}else{""})
    }
}
fn engine_entry() -> String {
    std::env::current_exe()
        .map(|path| super::quote(&path))
        .unwrap_or_else(|_| "phoenix-diagram".into())
}
fn add(receipt: &mut InstallReceipt, error: super::InstallError, path: &str) {
    if error.kind == super::InstallErrorKind::Conflict {
        receipt
            .preserved
            .push(format!("{}: {}", error.code, error.message));
    }
    receipt.conflicts.push(InstallConflict {
        path: path.into(),
        code: error.code,
        message: error.message,
    });
}
fn protect_project(project: &Arc<Directory>) -> InstallResult<()> {
    if let Some(home) = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .and_then(|p| p.canonicalize().ok())
    {
        if project.path == home
            || plan::PROVIDERS
                .iter()
                .any(|p| project.path.starts_with(home.join(p)))
        {
            return Err(disk::conflict("install/personal-scope", "Migration is restricted to explicit project copies; personal provider installations are preserved"));
        }
    }
    Ok(())
}
fn external(project: &Arc<Directory>) -> InstallResult<()> {
    for path in ["skills-lock.json", ".agents/.skill-lock.json"] {
        if let Some(snapshot) = project.read_path(path)? {
            let value: serde_json::Value =
                parse(&snapshot.bytes, "install/external-manager-invalid")?;
            let skills = value
                .get("skills")
                .and_then(|v| v.as_object())
                .ok_or_else(|| disk::conflict("install/external-manager-invalid", path))?;
            if skills.iter().any(|(key, value)| {
                key == "archify"
                    || value
                        .get("skillPath")
                        .and_then(|v| v.as_str())
                        .is_some_and(|p| p.split('/').any(|s| s == "archify"))
            }) {
                return Err(disk::conflict(
                    "install/externally-managed",
                    format!("{path} records Archify ownership; remove it through its manager"),
                ));
            }
        }
    }
    Ok(())
}
fn scan(
    directory: &Arc<Directory>,
    path: &str,
    files: &mut BTreeMap<String, FileIdentity>,
    directories: &mut BTreeMap<String, (Physical, u32)>,
) -> InstallResult<()> {
    directory.verify()?;
    if directories.len() + files.len() > 20_000 {
        return Err(disk::conflict("install/legacy-inventory-incomplete", path));
    }
    for name in directory.list()? {
        let relative = format!("{path}/{name}");
        let entry = directory
            .entry(&name)?
            .ok_or_else(|| disk::conflict("install/legacy-changed", &relative))?;
        if entry.physical.device != directory.physical.device || entry.uid != disk::uid() {
            return Err(disk::conflict("install/unsafe-legacy-entry", &relative));
        }
        if entry.kind == libc::S_IFDIR {
            directories.insert(relative.clone(), (entry.physical, entry.mode));
            scan(
                &directory
                    .child(&name)?
                    .ok_or_else(|| disk::conflict("install/legacy-changed", &relative))?,
                &relative,
                files,
                directories,
            )?;
        } else if entry.kind == libc::S_IFREG {
            let current = directory
                .read(&name, &[1])?
                .ok_or_else(|| disk::conflict("install/legacy-changed", &relative))?;
            if ![0o644, 0o755].contains(&current.identity.mode) {
                return Err(disk::conflict("install/legacy-mode", &relative));
            }
            files.insert(relative, current.identity);
        } else {
            return Err(disk::conflict("install/unsafe-legacy-entry", &relative));
        }
    }
    Ok(())
}
fn recognize(prepared: &mut Prepared) -> InstallResult<Migration> {
    protect_project(&prepared.project)?;
    external(&prepared.project)?;
    let catalog = catalog()?;
    let mut roots = vec![];
    for provider in &prepared.receipt.providers {
        let path = format!("{provider}/skills/archify");
        let mut ancestor = prepared.project.clone();
        for part in path.split('/') {
            let Some(next) = ancestor.child(part)? else {
                break;
            };
            if next.physical.device != prepared.project.physical.device {
                return Err(disk::conflict("install/cross-device-parent", &path));
            }
            ancestor = next;
        }
        let Some(root) = prepared.project.descend(&path)? else {
            continue;
        };
        let (parent, name) = prepared
            .project
            .parent(&path)?
            .ok_or_else(|| disk::conflict("install/legacy-parent-changed", &path))?;
        if parent.physical.device != prepared.project.physical.device
            || root.physical.device != prepared.project.physical.device
        {
            return Err(disk::conflict("install/cross-device-parent", &path));
        }
        let entry = parent
            .entry(&name)?
            .ok_or_else(|| disk::conflict("install/legacy-changed", &path))?;
        let mut files = BTreeMap::new();
        let mut directories = BTreeMap::new();
        directories.insert(path.clone(), (entry.physical, entry.mode));
        scan(&root, &path, &mut files, &mut directories)?;
        let source = catalog
            .sources
            .iter()
            .find(|source| {
                source.files.len() == files.len()
                    && source.files.iter().all(|f| {
                        files.get(&format!("{path}/{}", f.path)).is_some_and(|id| {
                            id.bytes == f.bytes && id.mode == f.mode && id.sha256 == f.sha256
                        })
                    })
            })
            .ok_or_else(|| {
                disk::conflict(
                    "install/unknown-legacy-source",
                    format!(
                        "{path}: complete file set, bytes, and modes do not match a frozen source"
                    ),
                )
            })?;
        let expected_dirs: BTreeSet<_> = source
            .files
            .iter()
            .flat_map(|f| {
                let mut parts = vec![path.clone()];
                let mut parent = f.path.as_str();
                while let Some((next, _)) = parent.rsplit_once('/') {
                    parts.push(format!("{path}/{next}"));
                    parent = next;
                }
                parts
            })
            .collect();
        if directories.keys().collect::<BTreeSet<_>>() != expected_dirs.iter().collect() {
            return Err(disk::conflict("install/extra-legacy-directory", &path));
        }
        let mut entries = vec![];
        for (path, id) in files {
            entries.push(LegacyEntry {
                path,
                physical: Physical {
                    device: id.device.clone(),
                    inode: id.inode.clone(),
                },
                mode: id.mode,
                file: Some(id),
                temporary: String::new(),
                removed: false,
            });
        }
        let mut dirs: Vec<_> = directories.into_iter().collect();
        dirs.sort_by(|a, b| {
            b.0.split('/')
                .count()
                .cmp(&a.0.split('/').count())
                .then(a.0.cmp(&b.0))
        });
        for (path, (physical, mode)) in dirs {
            entries.push(LegacyEntry {
                path,
                physical,
                mode,
                file: None,
                temporary: String::new(),
                removed: false,
            });
        }
        for (i, entry) in entries.iter_mut().enumerate() {
            entry.temporary = temporary(&entry.physical, i);
        }
        prepared.receipt.migration_sources.push(source.id.clone());
        prepared
            .receipt
            .reinstall_instructions
            .push(reinstall(&source.id));
        for entry in &entries {
            prepared.receipt.removals.push(LegacyRemoval {
                path: entry.path.clone(),
                source: source.id.clone(),
                status: "planned".into(),
            });
        }
        roots.push(LegacyRoot {
            kind: "legacy-root-remove".into(),
            path,
            source: source.id.clone(),
            parent: parent.physical.clone(),
            entries,
            pending: None,
        });
    }
    if let Some(home) = std::env::var_os("HOME") {
        for provider in &prepared.receipt.providers {
            let path = std::path::PathBuf::from(&home).join(format!("{provider}/skills/archify"));
            if std::fs::symlink_metadata(&path).is_ok() {
                prepared
                    .receipt
                    .preserved
                    .push(format!("personal:{}", path.display()));
            }
        }
    }
    prepared.receipt.command = "migrate".into();
    Ok(Migration {
        confirmed: true,
        roots,
        parents: BTreeMap::new(),
    })
}
pub(super) fn bind_parents(
    prepared: &mut Prepared,
    migration: &mut Migration,
    operations: &[Operation],
) -> InstallResult<()> {
    let mut parents: Vec<_> = operations
        .iter()
        .map(|op| {
            op.path
                .rsplit_once('/')
                .expect("operation parent")
                .0
                .to_owned()
        })
        .collect();
    parents.extend(migration.roots.iter().map(|root| {
        root.path
            .rsplit_once('/')
            .expect("legacy parent")
            .0
            .to_owned()
    }));
    for parent in &parents {
        let mut relative = String::new();
        for part in parent.split('/') {
            if !relative.is_empty() {
                relative.push('/');
            }
            relative.push_str(part);
            if prepared.directories.contains_key(&relative) {
                continue;
            }
            if let Some(directory) = prepared.project.descend(&relative)? {
                if directory.physical.device != prepared.project.physical.device {
                    return Err(disk::conflict("install/cross-device-parent", &relative));
                }
                prepared.directories.insert(relative.clone(), directory);
            } else {
                break;
            }
        }
        tx::target_parent(&prepared.project, parent, &mut prepared.directories)?;
    }
    migration.parents = prepared
        .directories
        .iter()
        .map(|(p, d)| (p.clone(), d.physical.clone()))
        .collect();
    verify_parents(&prepared.project, migration)
}
fn verify_parents(project: &Arc<Directory>, migration: &Migration) -> InstallResult<()> {
    for (path, physical) in &migration.parents {
        let parent = project
            .descend(path)?
            .ok_or_else(|| disk::conflict("install/parent-changed", path))?;
        if parent.physical != *physical {
            return Err(disk::conflict("install/parent-changed", path));
        }
    }
    Ok(())
}
fn bound_parent(
    project: &Arc<Directory>,
    path: &str,
    migration: &Migration,
) -> InstallResult<Arc<Directory>> {
    let mut parent = project.clone();
    let mut relative = String::new();
    for part in path.split('/') {
        if !relative.is_empty() {
            relative.push('/');
        }
        relative.push_str(part);
        let expected = migration
            .parents
            .get(&relative)
            .or_else(|| {
                migration
                    .roots
                    .iter()
                    .flat_map(|r| &r.entries)
                    .find(|e| e.file.is_none() && e.path == relative)
                    .map(|e| &e.physical)
            })
            .ok_or_else(|| disk::conflict("install/parent-unbound", &relative))?;
        let child = parent
            .child(part)?
            .ok_or_else(|| disk::conflict("install/parent-changed", &relative))?;
        if child.physical != *expected {
            return Err(disk::conflict("install/parent-changed", &relative));
        }
        parent = child;
    }
    parent.verify()?;
    Ok(parent)
}
fn verify_remaining(project: &Arc<Directory>, migration: &Migration) -> InstallResult<()> {
    for root in &migration.roots {
        let (parent, name) = project
            .parent(&root.path)?
            .ok_or_else(|| disk::conflict("install/legacy-parent-changed", &root.path))?;
        if parent.physical != root.parent {
            return Err(disk::conflict("install/legacy-parent-changed", &root.path));
        }
        let last = root
            .entries
            .last()
            .ok_or_else(|| disk::conflict("install/invalid-legacy-root", &root.path))?;
        let original = parent.entry(&name)?;
        let temporary = parent.entry(&last.temporary)?;
        if original.is_some() && temporary.is_some() {
            return Err(disk::conflict("install/legacy-later-claimant", &root.path));
        }
        let mut actual = BTreeSet::new();
        fn visit(
            dir: &Arc<Directory>,
            logical: &str,
            root: &LegacyRoot,
            actual: &mut BTreeSet<String>,
        ) -> InstallResult<()> {
            for name in dir.list()? {
                let raw = format!("{logical}/{name}");
                let entry = root
                    .entries
                    .iter()
                    .enumerate()
                    .find(|(i, e)| {
                        e.path == raw
                            || (root.pending == Some(*i)
                                && e.path
                                    .rsplit_once('/')
                                    .is_some_and(|(parent, _)| parent == logical)
                                && e.temporary == name)
                    })
                    .map(|(_, e)| e)
                    .ok_or_else(|| disk::conflict("install/legacy-later-claimant", &raw))?;
                check_entry(dir, &name, entry)?;
                if entry.removed || !actual.insert(entry.path.clone()) {
                    return Err(disk::conflict("install/legacy-later-claimant", &raw));
                }
                if entry.file.is_none() {
                    visit(
                        &dir.child(&name)?
                            .ok_or_else(|| disk::conflict("install/legacy-changed", &raw))?,
                        &entry.path,
                        root,
                        actual,
                    )?;
                }
            }
            Ok(())
        }
        if original.is_some() || temporary.is_some() {
            let target = if original.is_some() {
                name.as_str()
            } else {
                last.temporary.as_str()
            };
            if last.removed || (temporary.is_some() && root.pending != Some(root.entries.len() - 1))
            {
                return Err(disk::conflict("install/legacy-later-claimant", &root.path));
            }
            check_entry(&parent, target, last)?;
            actual.insert(root.path.clone());
            visit(
                &parent
                    .child(target)?
                    .ok_or_else(|| disk::conflict("install/legacy-changed", &root.path))?,
                &root.path,
                root,
                &mut actual,
            )?;
        }
        for (index, entry) in root.entries.iter().enumerate() {
            if actual.contains(&entry.path) == entry.removed
                || (!entry.removed && !actual.contains(&entry.path) && root.pending != Some(index))
            {
                // Only the durable pending item may disappear between unlink and cursor save.
                if !(root.pending == Some(index) && !entry.removed && !actual.contains(&entry.path))
                {
                    return Err(disk::conflict(
                        "install/legacy-snapshot-changed",
                        &entry.path,
                    ));
                }
            }
        }
    }
    Ok(())
}
fn check_entry(parent: &Arc<Directory>, name: &str, entry: &LegacyEntry) -> InstallResult<()> {
    let actual = parent
        .entry(name)?
        .ok_or_else(|| disk::conflict("install/legacy-changed", &entry.path))?;
    if actual.physical != entry.physical
        || actual.uid != disk::uid()
        || actual.mode != entry.mode
        || actual.kind
            != if entry.file.is_some() {
                libc::S_IFREG
            } else {
                libc::S_IFDIR
            }
    {
        return Err(disk::conflict(
            "install/legacy-snapshot-changed",
            &entry.path,
        ));
    }
    if let Some(file) = &entry.file {
        disk::expected(parent, name, file, &[1])?;
    }
    Ok(())
}
pub(super) fn remove_legacy(
    project: &Arc<Directory>,
    transaction: &Arc<Directory>,
    journal: &mut Journal,
    identity: &mut Option<FileIdentity>,
    lock: &disk::Lock,
    observer: Observer<'_>,
) -> InstallResult<()> {
    external(project)?;
    verify_remaining(project, journal.migration.as_ref().expect("migration"))?;
    let root_count = journal.migration.as_ref().expect("migration").roots.len();
    for r in 0..root_count {
        let count = journal.migration.as_ref().expect("migration").roots[r]
            .entries
            .len();
        for i in 0..count {
            if journal.migration.as_ref().expect("migration").roots[r].entries[i].removed {
                continue;
            }
            lock.verify()?;
            journal.migration.as_mut().expect("migration").roots[r].pending = Some(i);
            tx::save(project, journal, identity)?;
            let root = &journal.migration.as_ref().expect("migration").roots[r];
            let entry = root.entries[i].clone();
            let (parent_path, name) = entry.path.rsplit_once('/').expect("legacy entry");
            let parent = bound_parent(
                project,
                parent_path,
                journal.migration.as_ref().expect("migration"),
            )?;
            let expected = if entry.path == root.path {
                &root.parent
            } else {
                &root
                    .entries
                    .iter()
                    .find(|e| e.path == parent_path && e.file.is_none())
                    .ok_or_else(|| disk::conflict("install/legacy-parent-changed", parent_path))?
                    .physical
            };
            if parent.physical != *expected {
                return Err(disk::conflict("install/legacy-parent-changed", parent_path));
            }
            tx::emit_transaction(
                observer,
                Checkpoint::BeforeLegacyRemove(entry.path.clone()),
                project,
                transaction,
                &project.path.join(&entry.path),
            )?;
            external(project)?;
            if let Some(file) = &entry.file {
                disk::remove_known(&parent, name, &entry.temporary, file, &[1])?;
            } else {
                disk::remove_empty_known(
                    &parent,
                    name,
                    &entry.temporary,
                    &entry.physical,
                    entry.mode,
                )?;
            }
            tx::emit_transaction(
                observer,
                Checkpoint::AfterLegacyRemove(entry.path.clone()),
                project,
                transaction,
                &project.path.join(&entry.path),
            )?;
            let root = &mut journal.migration.as_mut().expect("migration").roots[r];
            root.entries[i].removed = true;
            root.pending = None;
            tx::save(project, journal, identity)?;
        }
    }
    verify_remaining(project, journal.migration.as_ref().expect("migration"))
}
pub(super) fn verify_removed(project: &Arc<Directory>, journal: &Journal) -> InstallResult<()> {
    if let Some(migration) = &journal.migration {
        external(project)?;
        verify_remaining(project, migration)?;
        verify_parents(project, migration)?;
    }
    Ok(())
}
fn removal_state(
    project: &Arc<Directory>,
    migration: &Migration,
    root: &LegacyRoot,
    index: usize,
    entry: &LegacyEntry,
) -> (&'static str, Option<String>) {
    if entry.removed {
        return ("removed", None);
    }
    let (path, name) = entry.path.rsplit_once('/').expect("validated legacy path");
    let Ok(parent) = bound_parent(project, path, migration) else {
        return ("preserved", None);
    };
    let (Ok(original), Ok(temporary)) = (parent.entry(name), parent.entry(&entry.temporary)) else {
        return ("preserved", None);
    };
    let temporary_path = temporary
        .as_ref()
        .map(|_| parent.path.join(&entry.temporary).display().to_string());
    if original.is_some() && temporary.is_some() {
        return ("preserved", temporary_path);
    }
    let current = if original.is_some() {
        Some(name)
    } else if temporary.is_some() {
        Some(entry.temporary.as_str())
    } else {
        None
    };
    if let Some(name) = current {
        (
            if check_entry(&parent, name, entry).is_ok() {
                "remaining"
            } else {
                "preserved"
            },
            temporary_path,
        )
    } else {
        (
            if root.pending == Some(index) {
                "removed"
            } else {
                "remaining"
            },
            None,
        )
    }
}
fn decorate(receipt: &mut InstallReceipt, journal: &Journal, project: &Arc<Directory>) {
    receipt.command = "migrate".into();
    receipt.transaction_schema_version = journal.schema_version;
    receipt.transaction_id = Some(journal.transaction_id.clone());
    receipt.journal = Some(project.path.join(&journal.active.journal));
    receipt.backup = Some(project.path.join(&journal.active.backup));
    receipt.providers = journal.active.providers.clone();
    receipt.engine = Some(journal.active.engine.clone());
    receipt.npm_package_version = Some(journal.active.npm_package_version.clone());
    receipt.bundle_manifest_sha256 = Some(journal.active.bundle_manifest_sha256.clone());
    receipt.version_manifest_sha256 = Some(journal.active.engine_manifest_sha256.clone());
    receipt.recovery_command = Some(format!(
        "{} migrate --recover {} --project {} --json",
        engine_entry(),
        journal.transaction_id,
        super::quote(&project.path)
    ));
    receipt.rollback_command = Some(format!(
        "{} migrate --rollback {} --project {} --json",
        engine_entry(),
        journal.transaction_id,
        super::quote(&project.path)
    ));
    if let Some(migration) = &journal.migration {
        receipt.migration_sources = migration.roots.iter().map(|r| r.source.clone()).collect();
        receipt.reinstall_instructions=vec!["Removed Archify files have no target backup and are not restored by recovery or rollback.".into()];
        for source in receipt.migration_sources.iter().collect::<BTreeSet<_>>() {
            receipt.reinstall_instructions.push(reinstall(source));
        }
        let mut removals = vec![];
        for root in &migration.roots {
            for (index, entry) in root.entries.iter().enumerate() {
                let (status, temporary) = removal_state(project, migration, root, index, entry);
                if status == "preserved" {
                    receipt.preserved.push(entry.path.clone());
                }
                if let Some(path) = temporary {
                    receipt.preserved.push(path);
                }
                removals.push(LegacyRemoval {
                    path: entry.path.clone(),
                    source: root.source.clone(),
                    status: status.into(),
                });
            }
        }
        receipt.removals = removals;
    }
}
fn load(project: &Arc<Directory>, id: &str) -> InstallResult<(Journal, FileIdentity, bool)> {
    if !state::valid_id(id) {
        return Err(super::invalid("install/invalid-transaction-id", id));
    }
    let transaction = tx::directory(project, id)?;
    transaction.private()?;
    let path = format!("{}/{id}.json", state::JOURNALS);
    let public = project.read_path(&path)?;
    let (snapshot, archived) = if let Some(audit) = transaction.read("rolled-back-v2.json", &[1])? {
        let journal: Journal = parse(&audit.bytes, "install/invalid-journal")?;
        journal.validate(project, &transaction)?;
        if journal.phase != Phase::RolledBack || journal.transaction_id != id {
            return Err(disk::conflict("install/invalid-journal", id));
        }
        if let Some(public) = &public {
            let compatibility = journal
                .rollback
                .as_ref()
                .and_then(|r| r.compatibility.as_ref())
                .ok_or_else(|| disk::conflict("install/journal-claimant", &path))?;
            if !disk::matches(&compatibility.after, &public.identity)
                || public.bytes != state::encoded(&compatibility.journal)?
            {
                return Err(disk::conflict("install/journal-claimant", &path));
            }
        }
        (audit, true)
    } else {
        (
            public.ok_or_else(|| disk::conflict("install/missing-journal", id))?,
            false,
        )
    };
    if snapshot.identity.mode != 0o600 {
        return Err(disk::conflict("install/state-not-private", id));
    }
    let journal: Journal = parse(&snapshot.bytes, "install/invalid-journal")?;
    journal.validate(project, &transaction)?;
    if journal.transaction_id != id || (archived && journal.phase != Phase::RolledBack) {
        return Err(disk::conflict("install/journal-id-mismatch", id));
    }
    Ok((journal, snapshot.identity, archived))
}
pub(super) fn migrate(
    request: &MigrationRequest,
    observer: Observer<'_>,
) -> InstallResult<InstallReceipt> {
    match request.mode {
        MigrationMode::FromArchify => {
            if request.transaction_id.is_some() {
                return Err(super::invalid(
                    "install/migration-arguments",
                    "from-archify has no transaction ID",
                ));
            }
            let bundle = request.bundle_root.as_ref().ok_or_else(|| {
                super::invalid("install/missing-bundle", "from-archify requires bundleRoot")
            })?;
            if !request.dry_run && !request.confirm_remove_archify {
                return Err(disk::conflict(
                    "install/confirmation-required",
                    "Explicit --confirm-remove-archify is required; no migration has started",
                ));
            }
            from_install(
                &InstallRequest {
                    project: request.project.clone(),
                    bundle_root: bundle.clone(),
                    version_manifest: bundle.join("ENGINE.json"),
                    engine: std::env::current_exe()
                        .map_err(|e| super::invalid("install/current-engine", e.to_string()))?,
                    providers: request.providers.clone(),
                },
                request.confirm_remove_archify,
                request.dry_run,
                observer,
            )
        }
        MigrationMode::Recover | MigrationMode::Rollback => {
            if request.bundle_root.is_some()
                || request.providers.is_some()
                || request.confirm_remove_archify
                || (request.mode == MigrationMode::Recover && request.dry_run)
            {
                return Err(super::invalid(
                    "install/migration-arguments",
                    "recover/rollback only accept project, transaction ID and rollback dry-run",
                ));
            }
            let id = request.transaction_id.as_deref().ok_or_else(|| {
                super::invalid("install/missing-transaction-id", "Choose a transaction")
            })?;
            if request.mode == MigrationMode::Recover {
                recover(&request.project, id, observer)
            } else {
                rollback(&request.project, id, request.dry_run, observer)
            }
        }
    }
}
pub(super) fn from_install(
    request: &InstallRequest,
    confirm: bool,
    dry_run: bool,
    observer: Observer<'_>,
) -> InstallResult<InstallReceipt> {
    if !dry_run && !confirm {
        return Err(disk::conflict(
            "install/confirmation-required",
            "Explicit --confirm-remove-archify is required; no migration has started",
        ));
    }
    let mut prepared = plan::prepare(request)?;
    prepared.receipt.command = "migrate".into();
    if matches!(
        prepared.receipt.status,
        InstallStatus::Conflict | InstallStatus::RecoveryRequired
    ) {
        if let Some(journal) = prepared
            .inventory
            .pending
            .iter()
            .find(|j| j.migration.is_some())
        {
            decorate(&mut prepared.receipt, journal, &prepared.project);
        }
        return Ok(prepared.receipt);
    }
    let migration = match recognize(&mut prepared) {
        Ok(value) => value,
        Err(error) => {
            add(&mut prepared.receipt, error, "archify");
            prepared.receipt.status = InstallStatus::Conflict;
            prepared.receipt.writes.clear();
            super::diagnostics(&mut prepared.receipt);
            return Ok(prepared.receipt);
        }
    };
    // A migration with unchanged diagram files still commits an active-last audit.
    if !migration.roots.is_empty() {
        prepared.receipt.status = InstallStatus::Planned;
    }
    if dry_run || prepared.receipt.status == InstallStatus::Unchanged {
        return Ok(prepared.receipt);
    }
    let state = prepared.project.ensure(state::STATE, true)?;
    let lock = state.lock()?;
    tx::emit(observer, Checkpoint::LockAcquired, &prepared.project)?;
    let mut prepared = plan::prepare(request)?;
    prepared.receipt.command = "migrate".into();
    if matches!(
        prepared.receipt.status,
        InstallStatus::Conflict | InstallStatus::RecoveryRequired
    ) {
        if let Some(journal) = prepared
            .inventory
            .pending
            .iter()
            .find(|j| j.migration.is_some())
        {
            decorate(&mut prepared.receipt, journal, &prepared.project);
        }
        return Ok(prepared.receipt);
    }
    let migration = recognize(&mut prepared)?;
    if migration.roots.is_empty() && prepared.receipt.status == InstallStatus::Unchanged {
        return Ok(prepared.receipt);
    }
    let result = tx::run(&mut prepared, &lock, Some(migration), observer);
    if let Some(id) = prepared.receipt.transaction_id.clone() {
        if let Ok((journal, _, _)) = load(&prepared.project, &id) {
            decorate(&mut prepared.receipt, &journal, &prepared.project);
        }
    }
    if let Err(error) = result {
        prepared.receipt.status = InstallStatus::RecoveryRequired;
        add(&mut prepared.receipt, error, "migration");
        super::diagnostics(&mut prepared.receipt);
    }
    Ok(prepared.receipt)
}
fn forward(
    project: &Arc<Directory>,
    journal: &mut Journal,
    identity: &mut Option<FileIdentity>,
    lock: &disk::Lock,
    observer: Observer<'_>,
) -> InstallResult<()> {
    let transaction = tx::directory(project, &journal.transaction_id)?;
    verify_parents(project, journal.migration.as_ref().expect("migration"))?;
    remove_legacy(project, &transaction, journal, identity, lock, observer)?;
    let mut known = BTreeMap::new();
    for (path, physical) in &journal.migration.as_ref().expect("migration").parents {
        let directory = project
            .descend(path)?
            .ok_or_else(|| disk::conflict("install/parent-changed", path))?;
        if directory.physical != *physical {
            return Err(disk::conflict("install/parent-changed", path));
        }
        known.insert(path.clone(), directory);
    }
    for i in 0..journal.operations.len() {
        lock.verify()?;
        let op = journal.operations[i].clone();
        if i < journal.completed {
            disk::expected(
                &project
                    .parent(&op.path)?
                    .ok_or_else(|| disk::conflict("install/recovery-parent-missing", &op.path))?
                    .0,
                op.path.rsplit_once('/').expect("path").1,
                &op.after,
                &[1],
            )?;
            continue;
        }
        if journal.pending.is_some_and(|pending| pending != i) {
            return Err(disk::conflict(
                "install/invalid-cursor",
                &journal.transaction_id,
            ));
        }
        journal.pending = Some(i);
        tx::save(project, journal, identity)?;
        if op.path == state::ACTIVE {
            tx::emit_transaction(
                observer,
                Checkpoint::BeforeActive,
                project,
                &transaction,
                &project.path.join(state::ACTIVE),
            )?;
        }
        let (parent_path, name) = op.path.rsplit_once('/').expect("path");
        let parent = tx::target_parent(project, parent_path, &mut known)?;
        let after = tx::subdirectory(&transaction, "after")?;
        let retired = tx::subdirectory(&transaction, "retired")?;
        let current = parent.read(name, &[1, 2])?;
        if current
            .as_ref()
            .is_some_and(|s| disk::matches(&op.after, &s.identity))
        {
            if let Some(staged) = after.read(&i.to_string(), &[1, 2])? {
                if !disk::matches(&op.after, &staged.identity) || staged.identity.links != 2 {
                    return Err(disk::conflict(
                        "install/recovery-unknown-hardlink",
                        &op.path,
                    ));
                }
            }
            if after.entry(&i.to_string())?.is_some()
                || after.entry(&format!("{i}.discard"))?.is_some()
            {
                disk::remove_known(
                    &after,
                    &i.to_string(),
                    &format!("{i}.discard"),
                    &op.after,
                    &[2],
                )?;
            }
        } else {
            match (&op.before, &current) {
                (Some(before), Some(current)) if disk::matches(before, &current.identity) => {
                    tx::retire(&parent, name, &retired, &i.to_string(), before, &[1])?
                }
                (Some(before), None) => {
                    disk::expected(&retired, &i.to_string(), before, &[1])?;
                }
                (None, None) => {}
                _ => {
                    return Err(disk::conflict(
                        "install/recovery-later-modification",
                        &op.path,
                    ))
                }
            }
            tx::emit_transaction(
                observer,
                Checkpoint::AfterRetire(op.path.clone()),
                project,
                &transaction,
                &project.path.join(&op.path),
            )?;
            disk::expected(&after, &i.to_string(), &op.after, &[1])?;
            disk::link(&after, &i.to_string(), &parent, name)?;
            tx::emit_transaction(
                observer,
                Checkpoint::AfterLink(op.path.clone()),
                project,
                &transaction,
                &project.path.join(&op.path),
            )?;
            disk::remove_known(
                &after,
                &i.to_string(),
                &format!("{i}.discard"),
                &op.after,
                &[2],
            )?;
        }
        disk::expected(&parent, name, &op.after, &[1])?;
        tx::emit(observer, Checkpoint::AfterPublish(op.path.clone()), project)?;
        if op.path == state::ACTIVE {
            tx::emit_transaction(
                observer,
                Checkpoint::AfterActive,
                project,
                &transaction,
                &project.path.join(state::ACTIVE),
            )?;
        }
        journal.completed = i + 1;
        journal.pending = None;
        tx::save(project, journal, identity)?;
    }
    tx::verify_files(project, &journal.active)?;
    verify_removed(project, journal)?;
    journal.phase = Phase::Committed;
    tx::save(project, journal, identity)?;
    Ok(())
}
fn recover_selected_publications(
    project: &Arc<Directory>,
    id: &str,
    receipt: &mut InstallReceipt,
) -> InstallResult<()> {
    let inventory = state::inspect(project)?;
    let previous = load(project, id).ok().and_then(|(j, _, _)| {
        j.rollback
            .and_then(|r| r.previous_journal)
            .map(|j| j.transaction_id.clone())
    });
    for pending in &inventory.publications {
        let record_path = pending.witness.clone().unwrap_or_else(|| {
            pending
                .recovery_directory
                .join("publication-recovery-v1.json")
        });
        let relative = record_path
            .strip_prefix(&project.path)
            .ok()
            .and_then(|p| p.to_str())
            .ok_or_else(|| disk::conflict("install/foreign-publication-record", id))?;
        let record = project
            .read_path(relative)?
            .ok_or_else(|| disk::conflict("install/unknown-publication-record", id))?;
        let value: serde_json::Value = parse(&record.bytes, "install/unknown-publication-record")?;
        let target = value
            .pointer("/target/commitPath")
            .and_then(|v| v.as_str())
            .ok_or_else(|| disk::conflict("install/foreign-publication-record", id))?;
        let target = Path::new(target)
            .strip_prefix(&project.path)
            .ok()
            .and_then(|p| p.to_str())
            .ok_or_else(|| disk::conflict("install/foreign-publication-record", id))?;
        if target != format!("{}/{id}.json", state::JOURNALS)
            && !target.starts_with(&format!("{}/{id}/", state::TRANSACTIONS))
            && previous
                .as_ref()
                .is_none_or(|old| target != format!("{}/{old}.json", state::JOURNALS))
        {
            return Err(disk::conflict("install/other-transaction-pending", target));
        }
    }
    tx::recover_publications(project, &inventory, receipt)
}
pub(super) fn recover(
    path: &Path,
    id: &str,
    observer: Observer<'_>,
) -> InstallResult<InstallReceipt> {
    let project = Directory::open(path)?;
    protect_project(&project)?;
    if !state::valid_id(id) {
        return Err(super::invalid("install/invalid-transaction-id", id));
    }
    let state = project
        .descend(state::STATE)?
        .ok_or_else(|| disk::conflict("install/missing-state", id))?;
    let lock = state.lock()?;
    let mut receipt = super::receipt("migrate", &project.path);
    recover_selected_publications(&project, id, &mut receipt)?;
    let (mut journal, current, archived) = load(&project, id)?;
    decorate(&mut receipt, &journal, &project);
    if archived || journal.phase == Phase::RolledBack {
        drop(lock);
        return rollback(&project.path, id, false, observer);
    }
    if journal.phase == Phase::RollingBack {
        drop(lock);
        return rollback(&project.path, id, false, observer);
    }
    if journal.migration.is_none() {
        return Err(disk::conflict("install/not-migration", id));
    }
    if journal.phase == Phase::Committed {
        if let Some(migration) = &journal.migration {
            verify_remaining(&project, migration)?;
            verify_parents(&project, migration)?;
        }
        let inventory = state::inspect(&project)?;
        if inventory
            .active
            .as_ref()
            .is_none_or(|(active, _)| active.transaction_id != id)
        {
            return Err(disk::conflict("install/not-current-transaction", id));
        }
        tx::verify_files(&project, &journal.active)?;
        receipt.status = InstallStatus::Unchanged;
        return Ok(receipt);
    }
    let mut identity = Some(current);
    journal.phase = Phase::Recovering;
    tx::save(&project, &mut journal, &mut identity)?;
    let result = forward(&project, &mut journal, &mut identity, &lock, observer);
    decorate(&mut receipt, &journal, &project);
    if let Err(error) = result {
        receipt.status = InstallStatus::RecoveryRequired;
        add(&mut receipt, error, "migration");
        super::diagnostics(&mut receipt);
    } else {
        receipt.status = InstallStatus::Recovered;
        receipt.recovered.push(id.into());
    }
    Ok(receipt)
}
fn prepare_reverse(project: &Arc<Directory>, journal: &Journal) -> InstallResult<Rollback> {
    let transaction = tx::directory(project, &journal.transaction_id)?;
    let stage_directory = format!("rollback-{}", tx::id()?);
    let undo = transaction.create_child(&stage_directory, true)?;
    let before = tx::subdirectory(&transaction, "before")?;
    let last = journal.operations.len() - 1;
    let mut operations = vec![];
    for (i, op) in journal.operations.iter().enumerate() {
        let restored = if i != last {
            if let Some(backup) = &op.backup {
                let bytes = disk::expected(&before, &i.to_string(), backup, &[1])?.bytes;
                Some(tx::persist(
                    &undo,
                    &i.to_string(),
                    &bytes,
                    op.before.as_ref().expect("backup").mode,
                    None,
                )?)
            } else {
                None
            }
        } else {
            None
        };
        operations.push(Reverse {
            index: i,
            restored,
            done: false,
        });
    }
    let mut restored_active = None;
    let mut previous_journal = None;
    let mut previous_journal_identity = None;
    let mut previous_journal_after = None;
    if let Some(backup) = &journal.operations[last].backup {
        let bytes = disk::expected(&before, &last.to_string(), backup, &[1])?.bytes;
        let mut active: Active = parse(&bytes, "install/invalid-before-active")?;
        active.validate(project)?;
        for owned in &mut active.ownership {
            if let Some((i, _)) = journal
                .operations
                .iter()
                .enumerate()
                .find(|(_, op)| op.path == owned.path)
            {
                owned.identity = operations[i]
                    .restored
                    .clone()
                    .ok_or_else(|| disk::conflict("install/rollback-ownership", &owned.path))?;
            } else {
                let current = project
                    .read_path(&owned.path)?
                    .ok_or_else(|| disk::conflict("install/rollback-ownership", &owned.path))?;
                if !disk::matches(&owned.identity, &current.identity) {
                    return Err(disk::conflict(
                        "install/rollback-later-modification",
                        &owned.path,
                    ));
                }
            }
        }
        active.plan_sha256 = bundle::hash(&state::encoded(&(
            &active.bundle_manifest_sha256,
            &active.engine_manifest_sha256,
            &active.providers,
            &active.ownership,
        ))?);
        active.seal()?;
        let staged = tx::persist(
            &undo,
            &last.to_string(),
            &state::encoded(&active)?,
            0o600,
            None,
        )?;
        operations[last].restored = Some(staged.clone());
        let (mut old, old_identity, archived) = load(project, &active.transaction_id)?;
        if archived || old.phase != Phase::Committed {
            return Err(disk::conflict(
                "install/rollback-before-journal",
                &active.transaction_id,
            ));
        }
        for op in &mut old.operations {
            if op.path == state::ACTIVE {
                op.after = staged.clone();
            } else if let Some(owned) = active.ownership.iter().find(|o| o.path == op.path) {
                op.after = owned.identity.clone();
            }
        }
        old.active = active.clone();
        old.seal()?;
        previous_journal_after = Some(tx::persist(
            &undo,
            &journal.operations.len().to_string(),
            &state::encoded(&old)?,
            0o600,
            None,
        )?);
        previous_journal = Some(Box::new(old));
        previous_journal_identity = Some(old_identity);
        restored_active = Some(active);
    }
    let compatibility = if journal.schema_version == 1 && restored_active.is_none() {
        let mut terminal = journal.clone();
        terminal.phase = Phase::Recovered;
        terminal.seal()?;
        let after = tx::persist(
            &undo,
            &(journal.operations.len() + 1).to_string(),
            &state::encoded(&terminal)?,
            0o600,
            None,
        )?;
        Some(Compatibility {
            journal: Box::new(terminal),
            after,
        })
    } else {
        None
    };
    Ok(Rollback {
        completed: 0,
        stage_directory,
        stage_physical: undo.physical.clone(),
        journal_parent: project
            .descend(state::JOURNALS)?
            .ok_or_else(|| disk::conflict("install/missing-journals", state::JOURNALS))?
            .physical
            .clone(),
        operations,
        restored_active,
        previous_journal,
        previous_journal_identity,
        previous_journal_after,
        compatibility,
    })
}
fn reverse_one(
    project: &Arc<Directory>,
    journal: &Journal,
    i: usize,
    observer: Observer<'_>,
) -> InstallResult<()> {
    let op = &journal.operations[i];
    let reverse = &journal.rollback.as_ref().expect("rollback").operations[i];
    let transaction = tx::directory(project, &journal.transaction_id)?;
    let rollback = journal.rollback.as_ref().expect("rollback");
    let undo = tx::subdirectory(&transaction, &rollback.stage_directory)?;
    if undo.physical != rollback.stage_physical {
        return Err(disk::conflict(
            "install/rollback-stage-changed",
            &rollback.stage_directory,
        ));
    }
    let retired = tx::subdirectory(&transaction, "retired")?;
    let (parent_path, name) = op.path.rsplit_once('/').expect("managed rollback path");
    let name = name.to_owned();
    let parent = if let Some(migration) = &journal.migration {
        bound_parent(project, parent_path, migration)?
    } else {
        project
            .descend(parent_path)?
            .ok_or_else(|| disk::conflict("install/rollback-parent-missing", &op.path))?
    };
    if let Some(migration) = &journal.migration {
        let path = op.path.rsplit_once('/').expect("operation parent").0;
        if migration.parents.get(path) != Some(&parent.physical) {
            return Err(disk::conflict("install/parent-changed", path));
        }
    }
    let displaced = format!("rollback-{i}");
    let current = parent.read(&name, &[1, 2])?;
    let restored_matches = current.as_ref().is_some_and(|s| {
        reverse
            .restored
            .as_ref()
            .is_some_and(|id| disk::matches(id, &s.identity))
    });
    tx::emit(
        observer,
        Checkpoint::BeforeRollback(op.path.clone()),
        project,
    )?;
    if !restored_matches {
        if let Some(current) = &current {
            if !disk::matches(&op.after, &current.identity) {
                return Err(disk::conflict(
                    "install/rollback-later-modification",
                    &op.path,
                ));
            }
            tx::retire(&parent, &name, &retired, &displaced, &op.after, &[1])?;
        } else if retired.entry(&displaced)?.is_none() {
            return Err(disk::conflict(
                "install/rollback-later-modification",
                &op.path,
            ));
        }
        if let Some(restored) = &reverse.restored {
            disk::expected(&undo, &i.to_string(), restored, &[1])?;
            disk::link(&undo, &i.to_string(), &parent, &name)?;
            disk::expected(&parent, &name, restored, &[2])?;
        }
    }
    if let Some(restored) = &reverse.restored {
        if undo.entry(&i.to_string())?.is_some() || undo.entry(&format!("{i}.discard"))?.is_some() {
            disk::remove_known(
                &undo,
                &i.to_string(),
                &format!("{i}.discard"),
                restored,
                &[2],
            )?;
        }
        disk::expected(&parent, &name, restored, &[1])?;
    } else if parent.entry(&name)?.is_some() {
        return Err(disk::conflict("install/rollback-later-claimant", &op.path));
    }
    // Retain the original v2 after inode in this transaction as rollback audit;
    // it is never an Archify backup and is not blindly linked on later calls.
    if let Some(current) = retired.read(&displaced, &[1])? {
        if !disk::matches(&op.after, &current.identity) {
            return Err(disk::conflict("install/rollback-retired-changed", &op.path));
        }
    }
    tx::emit_transaction(
        observer,
        Checkpoint::AfterRollback(op.path.clone()),
        project,
        &transaction,
        &project.path.join(&op.path),
    )?;
    Ok(())
}
fn rebind_previous(
    project: &Arc<Directory>,
    journal: &Journal,
    observer: Observer<'_>,
) -> InstallResult<()> {
    let rollback = journal.rollback.as_ref().expect("rollback");
    let Some(previous) = &rollback.previous_journal else {
        return Ok(());
    };
    let parent = project
        .descend(state::JOURNALS)?
        .ok_or_else(|| disk::conflict("install/missing-journals", state::JOURNALS))?;
    if parent.physical != rollback.journal_parent {
        return Err(disk::conflict(
            "install/rollback-journal-parent-changed",
            state::JOURNALS,
        ));
    }
    let name = format!("{}.json", previous.transaction_id);
    let transaction = tx::directory(project, &journal.transaction_id)?;
    let stage = tx::subdirectory(&transaction, &rollback.stage_directory)?;
    let retired = tx::subdirectory(&transaction, "retired")?;
    let index = journal.operations.len().to_string();
    let before = rollback
        .previous_journal_identity
        .as_ref()
        .expect("validated before journal");
    let after = rollback
        .previous_journal_after
        .as_ref()
        .expect("validated after journal");
    match parent.read(&name, &[1])? {
        Some(current) if disk::matches(after, &current.identity) => {
            if stage.entry(&index)?.is_some() {
                return Err(disk::conflict(
                    "install/rollback-journal-stage-claimant",
                    stage.path.join(&index).display().to_string(),
                ));
            }
            return Ok(());
        }
        Some(current) if disk::matches(before, &current.identity) => {
            disk::expected(&stage, &index, after, &[1])?;
            tx::retire(&parent, &name, &retired, "rollback-journal", before, &[1])?;
            tx::emit_transaction(
                observer,
                Checkpoint::AfterRollbackJournalRetire,
                project,
                &transaction,
                &parent.path.join(&name),
            )?;
        }
        None => {
            disk::expected(&retired, "rollback-journal", before, &[1])?;
        }
        _ => {
            return Err(disk::conflict(
                "install/rollback-before-journal-changed",
                &previous.active.journal,
            ))
        }
    }
    disk::expected(&stage, &index, after, &[1])?;
    disk::rename(&stage, &index, &parent, &name)?;
    disk::expected(&parent, &name, after, &[1])?;
    tx::emit_transaction(
        observer,
        Checkpoint::AfterRollbackJournalPublish,
        project,
        &transaction,
        &parent.path.join(&name),
    )
}
fn verify_reversed(project: &Arc<Directory>, journal: &Journal) -> InstallResult<()> {
    let rollback = journal
        .rollback
        .as_ref()
        .ok_or_else(|| disk::conflict("install/missing-rollback", &journal.transaction_id))?;
    for (op, reverse) in journal.operations.iter().zip(&rollback.operations) {
        match (&reverse.restored, project.read_path(&op.path)?) {
            (Some(expected), Some(actual)) if disk::matches(expected, &actual.identity) => {}
            (None, None) => {}
            _ => {
                return Err(disk::conflict(
                    "install/rollback-later-modification",
                    &op.path,
                ))
            }
        }
    }
    let parent = project
        .descend(state::JOURNALS)?
        .ok_or_else(|| disk::conflict("install/missing-journals", state::JOURNALS))?;
    if parent.physical != rollback.journal_parent {
        return Err(disk::conflict(
            "install/rollback-journal-parent-changed",
            state::JOURNALS,
        ));
    }
    if let Some(previous) = &rollback.previous_journal {
        disk::expected(
            &parent,
            &format!("{}.json", previous.transaction_id),
            rollback
                .previous_journal_after
                .as_ref()
                .expect("validated previous journal"),
            &[1],
        )?;
    }
    if let Some(active) = &rollback.restored_active {
        tx::verify_files(project, active)?;
    }
    Ok(())
}
fn rollback_run(
    project: &Arc<Directory>,
    journal: &mut Journal,
    identity: &mut Option<FileIdentity>,
    lock: &disk::Lock,
    observer: Observer<'_>,
) -> InstallResult<()> {
    let last = journal.operations.len() - 1;
    // Restore diagram files before the re-bound previous active receipt.
    for i in (0..last).rev().chain(std::iter::once(last)) {
        lock.verify()?;
        let reverse = &journal.rollback.as_ref().expect("rollback").operations[i];
        if reverse.done {
            let op = &journal.operations[i];
            match (&reverse.restored, project.read_path(&op.path)?) {
                (Some(id), Some(s)) if disk::matches(id, &s.identity) => {}
                (None, None) => {}
                _ => {
                    return Err(disk::conflict(
                        "install/rollback-later-modification",
                        &op.path,
                    ))
                }
            }
            continue;
        }
        if i == last {
            rebind_previous(project, journal, observer)?;
        }
        reverse_one(project, journal, i, observer)?;
        let rollback = journal.rollback.as_mut().expect("rollback");
        rollback.operations[i].done = true;
        rollback.completed += 1;
        tx::save(project, journal, identity)?;
    }
    verify_reversed(project, journal)?;
    journal.phase = Phase::RolledBack;
    tx::save(project, journal, identity)?;
    archive(
        project,
        journal,
        identity.as_ref().expect("saved"),
        observer,
    )
}
fn archive(
    project: &Arc<Directory>,
    journal: &Journal,
    identity: &FileIdentity,
    observer: Observer<'_>,
) -> InstallResult<()> {
    let journals = project
        .descend(state::JOURNALS)?
        .ok_or_else(|| disk::conflict("install/missing-journals", state::JOURNALS))?;
    let transaction = tx::directory(project, &journal.transaction_id)?;
    let name = format!("{}.json", journal.transaction_id);
    disk::expected(&journals, &name, identity, &[1])?;
    let lock_name = format!(
        ".phoenix-diagram-lock-{}.lock",
        bundle::hash(name.as_bytes())
    );
    if journal
        .rollback
        .as_ref()
        .and_then(|r| r.compatibility.as_ref())
        .is_none()
    {
        if let Some(lock) = journals.read(&lock_name, &[1])? {
            let expected=serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"tool":"phoenix-diagram","kind":"output-lock","name":name,"parentDevice":journals.physical.device,"parentInode":journals.physical.inode})).map_err(|e|disk::conflict("install/invalid-lock",e.to_string()))?;
            if lock.identity.mode != 0o600 || lock.bytes != expected {
                return Err(disk::conflict("install/unknown-lock", &lock_name));
            }
            disk::rename(
                &journals,
                &lock_name,
                &transaction,
                "rolled-back-journal.lock",
            )?;
            disk::expected(
                &transaction,
                "rolled-back-journal.lock",
                &lock.identity,
                &[1],
            )?;
        }
    }
    disk::rename(&journals, &name, &transaction, "rolled-back-v2.json")?;
    disk::expected(&transaction, "rolled-back-v2.json", identity, &[1])?;
    tx::emit_transaction(
        observer,
        Checkpoint::RollbackAuditArchived,
        project,
        &transaction,
        &transaction.path.join("rolled-back-v2.json"),
    )?;
    finish_compatibility(project, journal, observer)
}
fn compatibility_complete(project: &Arc<Directory>, journal: &Journal) -> InstallResult<bool> {
    let Some(compatibility) = journal
        .rollback
        .as_ref()
        .and_then(|r| r.compatibility.as_ref())
    else {
        return Ok(true);
    };
    let path = format!("{}/{}.json", state::JOURNALS, journal.transaction_id);
    match project.read_path(&path)? {
        Some(current)
            if disk::matches(&compatibility.after, &current.identity)
                && current.bytes == state::encoded(&compatibility.journal)? =>
        {
            Ok(true)
        }
        None => Ok(false),
        _ => Err(disk::conflict("install/journal-claimant", path)),
    }
}
fn finish_compatibility(
    project: &Arc<Directory>,
    journal: &Journal,
    observer: Observer<'_>,
) -> InstallResult<()> {
    if compatibility_complete(project, journal)? {
        return Ok(());
    }
    let rollback = journal.rollback.as_ref().expect("rollback");
    let compatibility = rollback
        .compatibility
        .as_ref()
        .expect("pending compatibility");
    let transaction = tx::directory(project, &journal.transaction_id)?;
    let stage = tx::subdirectory(&transaction, &rollback.stage_directory)?;
    let parent = project
        .descend(state::JOURNALS)?
        .ok_or_else(|| disk::conflict("install/missing-journals", state::JOURNALS))?;
    if parent.physical != rollback.journal_parent {
        return Err(disk::conflict(
            "install/rollback-journal-parent-changed",
            state::JOURNALS,
        ));
    }
    let index = (journal.operations.len() + 1).to_string();
    let name = format!("{}.json", journal.transaction_id);
    disk::expected(&stage, &index, &compatibility.after, &[1])?;
    disk::rename(&stage, &index, &parent, &name)?;
    disk::expected(&parent, &name, &compatibility.after, &[1])?;
    tx::emit_transaction(
        observer,
        Checkpoint::RollbackCompatibilityPublished,
        project,
        &transaction,
        &parent.path.join(&name),
    )
}
pub(super) fn rollback(
    path: &Path,
    id: &str,
    dry_run: bool,
    observer: Observer<'_>,
) -> InstallResult<InstallReceipt> {
    let project = Directory::open(path)?;
    protect_project(&project)?;
    let (mut journal, _current, archived) = load(&project, id)?;
    let mut receipt = super::receipt("migrate", &project.path);
    decorate(&mut receipt, &journal, &project);
    if archived || journal.phase == Phase::RolledBack {
        verify_reversed(&project, &journal)?;
        let inventory = state::inspect(&project)?;
        let expected = journal
            .rollback
            .as_ref()
            .and_then(|r| r.restored_active.as_ref())
            .map(|a| a.transaction_id.as_str());
        if inventory
            .active
            .as_ref()
            .map(|(a, _)| a.transaction_id.as_str())
            != expected
        {
            return Err(disk::conflict("install/not-current-transaction", id));
        }
        if !dry_run {
            let state = project
                .descend(state::STATE)?
                .ok_or_else(|| disk::conflict("install/missing-state", id))?;
            let _lock = state.lock()?;
            let (latest, identity, archived) = load(&project, id)?;
            verify_reversed(&project, &latest)?;
            let inventory = state::inspect(&project)?;
            if inventory
                .active
                .as_ref()
                .map(|(a, _)| a.transaction_id.as_str())
                != expected
            {
                return Err(disk::conflict("install/not-current-transaction", id));
            }
            if archived {
                finish_compatibility(&project, &latest, observer)?;
            } else {
                archive(&project, &latest, &identity, observer)?;
            }
        } else if !compatibility_complete(&project, &journal)? {
            receipt.status = InstallStatus::RecoveryRequired;
            super::diagnostics(&mut receipt);
            return Ok(receipt);
        }
        receipt.status = InstallStatus::RolledBack;
        receipt.journal = Some(
            tx::directory(&project, id)?
                .path
                .join("rolled-back-v2.json"),
        );
        return Ok(receipt);
    }
    if !matches!(journal.phase, Phase::Committed | Phase::RollingBack) {
        return Err(disk::conflict("install/rollback-not-committed", id));
    }
    if journal.phase == Phase::Committed {
        if let Some(migration) = &journal.migration {
            verify_remaining(&project, migration)?;
            verify_parents(&project, migration)?;
        }
        let inventory = state::inspect(&project)?;
        if !inventory.pending.is_empty()
            || inventory
                .active
                .as_ref()
                .is_none_or(|(a, _)| a.transaction_id != id)
        {
            return Err(disk::conflict("install/not-current-transaction", id));
        }
        tx::verify_files(&project, &journal.active)?;
        let active = project
            .read_path(state::ACTIVE)?
            .ok_or_else(|| disk::conflict("install/missing-active", id))?;
        if !disk::matches(
            &journal.operations.last().expect("active").after,
            &active.identity,
        ) {
            return Err(disk::conflict("install/active-changed", id));
        }
    }
    if dry_run {
        receipt.status = InstallStatus::Planned;
        return Ok(receipt);
    }
    let state = project
        .descend(state::STATE)?
        .ok_or_else(|| disk::conflict("install/missing-state", id))?;
    let lock = state.lock()?;
    recover_selected_publications(&project, id, &mut receipt)?;
    let (current_journal, current, _) = load(&project, id)?;
    journal = current_journal;
    let mut identity = Some(current);
    if journal.phase == Phase::Committed {
        if let Some(migration) = &journal.migration {
            verify_remaining(&project, migration)?;
            verify_parents(&project, migration)?;
        }
        let inventory = state::inspect(&project)?;
        if inventory
            .active
            .as_ref()
            .is_none_or(|(a, _)| a.transaction_id != id)
        {
            return Err(disk::conflict("install/not-current-transaction", id));
        }
        tx::verify_files(&project, &journal.active)?;
        journal.rollback = Some(prepare_reverse(&project, &journal)?);
        journal.schema_version = 2;
        for operation in &mut journal.operations {
            operation.kind = Some("file-write".into());
        }
        journal.phase = Phase::RollingBack;
        tx::save(&project, &mut journal, &mut identity)?;
    }
    let result = rollback_run(&project, &mut journal, &mut identity, &lock, observer);
    decorate(&mut receipt, &journal, &project);
    if let Err(error) = result {
        receipt.status = InstallStatus::RecoveryRequired;
        add(&mut receipt, error, "rollback");
        super::diagnostics(&mut receipt);
    } else {
        receipt.status = InstallStatus::RolledBack;
        receipt.journal = Some(
            tx::directory(&project, id)?
                .path
                .join("rolled-back-v2.json"),
        );
    }
    Ok(receipt)
}

// Structural v1 fixture for deterministic Rust kill tests. Historical producer
// compatibility is tested separately with the retained real v1 bundle/engine.
#[cfg(test)]
pub(super) fn v1_fixture(path: &Path) -> InstallResult<()> {
    let project = Directory::open(path)?;
    let inventory = state::inspect(&project)?;
    let (active, snapshot) = inventory.active.as_ref().expect("installed fixture");
    let (mut journal, current, archived) = load(&project, &active.transaction_id)?;
    assert!(!archived && journal.migration.is_none() && journal.rollback.is_none());
    journal.schema_version = 1;
    journal.active.schema_version = 1;
    journal.active.seal()?;
    for operation in &mut journal.operations {
        operation.kind = None;
    }
    let state = project.descend(state::STATE)?.expect("fixture state");
    let transaction = tx::directory(&project, &active.transaction_id)?;
    let after = tx::subdirectory(&transaction, "after")?;
    let retired = tx::subdirectory(&transaction, "retired")?;
    let name = journal.operations.len().to_string();
    let identity = tx::persist(
        &after,
        &name,
        &state::encoded(&journal.active)?,
        0o600,
        None,
    )?;
    tx::retire(
        &state,
        "active.json",
        &retired,
        "v1-fixture-active",
        &snapshot.identity,
        &[1],
    )?;
    disk::rename(&after, &name, &state, "active.json")?;
    disk::expected(&state, "active.json", &identity, &[1])?;
    journal
        .operations
        .last_mut()
        .expect("active operation")
        .after = identity;
    tx::save(&project, &mut journal, &mut Some(current))
}
