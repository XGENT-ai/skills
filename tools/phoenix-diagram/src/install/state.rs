// Apache-2.0 transaction ordering adapted from Phoenix UI install/transaction.rs.
// Phoenix diagram records physical before/after bindings and only compensates
// files that still match this transaction; unrelated edits are preserved.
use super::bundle::{self, parse};
use super::disk::{self, Directory, Physical, Snapshot};
use super::{InstallEngine, InstallResult};
use crate::output::FileIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) const STATE: &str = ".phoenix-diagram/install";
pub(super) const ACTIVE: &str = ".phoenix-diagram/install/active.json";
pub(super) const JOURNALS: &str = ".phoenix-diagram/install/journals";
pub(super) const TRANSACTIONS: &str = ".phoenix-diagram/install/transactions";

pub(super) fn valid_id(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(super) fn managed(path: &str) -> bool {
    let mut parts = path.splitn(4, '/');
    let Some(provider) = parts.next() else {
        return false;
    };
    super::plan::PROVIDERS.contains(&provider)
        && provider != ".codex"
        && parts.next() == Some("skills")
        && parts.next() == Some("phoenix-diagram")
        && parts.next().is_some_and(|path| bundle::allowed(path, true))
}
pub(super) fn encoded<T: Serialize>(value: &T) -> InstallResult<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|e| disk::conflict("install/state-serialization", e.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Owned {
    pub path: String,
    pub identity: FileIdentity,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Active {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub npm_package_version: String,
    pub engine: InstallEngine,
    pub bundle_manifest_sha256: String,
    pub engine_manifest_sha256: String,
    pub providers: Vec<String>,
    pub transaction_id: String,
    pub journal: String,
    pub backup: String,
    pub project: Physical,
    pub ownership: Vec<Owned>,
    pub plan_sha256: String,
    pub checksum: String,
}
impl Active {
    pub fn seal(&mut self) -> InstallResult<()> {
        self.checksum.clear();
        self.checksum = bundle::hash(&encoded(self)?);
        Ok(())
    }
    pub fn validate(&self, project: &Arc<Directory>) -> InstallResult<()> {
        let mut value = self.clone();
        value.seal()?;
        let mut paths = BTreeSet::new();
        let plan_sha256 = bundle::hash(&encoded(&(
            &self.bundle_manifest_sha256,
            &self.engine_manifest_sha256,
            &self.providers,
            &self.ownership,
        ))?);
        if ![1, 2].contains(&self.schema_version)
            || self.tool != "phoenix-diagram"
            || self.tool_version != crate::TOOL_VERSION
            || !valid_id(&self.transaction_id)
            || self.project != project.physical
            || self.journal != format!("{JOURNALS}/{}.json", self.transaction_id)
            || self.backup != format!("{TRANSACTIONS}/{}/before", self.transaction_id)
            || !bundle::valid_hash(&self.bundle_manifest_sha256)
            || !bundle::valid_hash(&self.engine_manifest_sha256)
            || !bundle::valid_hash(&self.plan_sha256)
            || self.plan_sha256 != plan_sha256
            || !bundle::valid_hash(&self.engine.source_input_sha256)
            || self.engine.input_contract_schema != crate::INPUT_CONTRACT_SCHEMA
            || self.engine.receipt_schema != crate::RECEIPT_SCHEMA
            || self.engine.viewer_revision != crate::VIEWER_REVISION
            || !bundle::valid_hash(&self.engine.binary.sha256)
            || self.engine.binary.bytes == 0
            || self.checksum != value.checksum
            || self.providers.is_empty()
            || self
                .providers
                .iter()
                .any(|p| !super::plan::PROVIDERS.contains(&p.as_str()))
            || self.ownership.is_empty()
            || self.ownership.len() > 200_000
            || self.ownership.iter().any(|owned| {
                !managed(&owned.path)
                    || !valid_identity(&owned.identity, false)
                    || !paths.insert(&owned.path)
            })
        {
            return Err(disk::conflict(
                "install/invalid-active",
                "Active receipt is not a valid diagram ownership record",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Phase {
    Applying,
    Committed,
    Recovering,
    Recovered,
    RollingBack,
    RolledBack,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Operation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub path: String,
    pub stage_path: String,
    pub backup_path: Option<String>,
    pub retired_path: String,
    pub undo_path: String,
    pub before: Option<FileIdentity>,
    pub after: FileIdentity,
    pub backup: Option<FileIdentity>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Journal {
    pub schema_version: u32,
    pub tool: String,
    pub transaction_id: String,
    pub project: Physical,
    pub transaction: Physical,
    pub phase: Phase,
    pub completed: usize,
    pub pending: Option<usize>,
    pub operations: Vec<Operation>,
    pub active: Active,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migration: Option<super::migration::Migration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback: Option<super::migration::Rollback>,
    pub checksum: String,
}
pub(super) fn valid_identity(identity: &FileIdentity, private: bool) -> bool {
    let decimal = |v: &str| v.parse::<u64>().is_ok_and(|n| n.to_string() == v);
    decimal(&identity.device)
        && decimal(&identity.inode)
        && identity.inode != "0"
        && identity.uid == disk::uid()
        && identity.links == 1
        && identity.bytes <= disk::FILE_LIMIT
        && bundle::valid_hash(&identity.sha256)
        && if private {
            identity.mode == 0o600
        } else {
            [0o644, 0o755].contains(&identity.mode)
        }
}
impl Journal {
    pub fn seal(&mut self) -> InstallResult<()> {
        self.checksum.clear();
        self.checksum = bundle::hash(&encoded(self)?);
        Ok(())
    }
    pub fn validate(
        &self,
        project: &Arc<Directory>,
        transaction: &Arc<Directory>,
    ) -> InstallResult<()> {
        let mut value = self.clone();
        value.seal()?;
        let mut paths = BTreeSet::new();
        self.active.validate(project)?;
        if ![1, 2].contains(&self.schema_version)
            || self.tool != "phoenix-diagram"
            || !valid_id(&self.transaction_id)
            || self.project != project.physical
            || self.transaction != transaction.physical
            || self.active.transaction_id != self.transaction_id
            || self.checksum != value.checksum
            || self.operations.is_empty()
            || self.operations.len() > 200_001
            || (matches!(
                self.phase,
                Phase::Committed | Phase::RollingBack | Phase::RolledBack
            ) && (self.completed != self.operations.len() || self.pending.is_some()))
            || self.completed > self.operations.len()
            || self.pending.is_some_and(|i| i >= self.operations.len())
        {
            return Err(disk::conflict(
                "install/invalid-journal",
                "Unknown diagram installation journal",
            ));
        }
        if self.schema_version == 1 && (self.migration.is_some() || self.rollback.is_some()) {
            return Err(disk::conflict("install/invalid-journal", "v1 extension"));
        }
        if let Some(migration) = &self.migration {
            migration.validate(project, self)?;
        }
        if let Some(rollback) = &self.rollback {
            rollback.validate(project, self)?;
        }
        for (index, operation) in self.operations.iter().enumerate() {
            let active = index + 1 == self.operations.len();
            if (self.schema_version == 2 && operation.kind.as_deref() != Some("file-write"))
                || (self.schema_version == 1 && operation.kind.is_some())
                || (active && operation.path != ACTIVE)
                || (!active && !managed(&operation.path))
                || !paths.insert(&operation.path)
                || operation.stage_path != format!("after/{index}")
                || operation.retired_path != format!("retired/{index}")
                || operation.undo_path != format!("undo/{index}")
                || operation.backup_path
                    != operation.before.as_ref().map(|_| format!("before/{index}"))
                || !valid_identity(&operation.after, active)
                || operation
                    .before
                    .as_ref()
                    .is_some_and(|i| !valid_identity(i, active))
                || operation.before.is_some() != operation.backup.is_some()
                || operation
                    .backup
                    .as_ref()
                    .is_some_and(|i| !valid_identity(i, true))
            {
                return Err(disk::conflict("install/invalid-journal", &operation.path));
            }
            if let Some(backup) = &operation.backup {
                let before = operation.before.as_ref().expect("validated before");
                let actual = transaction
                    .descend("before")?
                    .ok_or_else(|| disk::conflict("install/missing-backup", &operation.path))?;
                disk::expected(&actual, &index.to_string(), backup, &[1])?;
                if before.sha256 != backup.sha256 || before.bytes != backup.bytes {
                    return Err(disk::conflict("install/backup-mismatch", &operation.path));
                }
            }
        }
        let active_bytes = encoded(&self.active)?;
        let active_operation = self.operations.last().expect("validated operations");
        if active_operation.after.sha256 != bundle::hash(&active_bytes)
            || active_operation.after.bytes != active_bytes.len() as u64
        {
            return Err(disk::conflict("install/active-journal-mismatch", ACTIVE));
        }
        let ownership: BTreeMap<_, _> = self
            .active
            .ownership
            .iter()
            .map(|v| (&v.path, &v.identity))
            .collect();
        for operation in self.operations.iter().take(self.operations.len() - 1) {
            if ownership
                .get(&operation.path)
                .is_none_or(|identity| *identity != &operation.after)
            {
                return Err(disk::conflict(
                    "install/ownership-journal-mismatch",
                    &operation.path,
                ));
            }
        }
        Ok(())
    }
}
#[derive(Default)]
pub(super) struct Inventory {
    pub active: Option<(Active, Snapshot)>,
    pub pending: Vec<Journal>,
    pub publications: Vec<crate::output::PendingPublication>,
}
// A rolled-back last journal is archived for the actual v1 reader. Its
// now-lockless cleanup directory may remain, but only the exact empty owner
// namespace can be recognized without acquiring or creating an output lock.
fn empty_cleanup(parent: &Arc<Directory>) -> InstallResult<()> {
    #[derive(Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Owner {
        schema_version: u32,
        tool: String,
        kind: String,
        parent_path: std::path::PathBuf,
        parent: Physical,
        directory: Physical,
        record_identity: Physical,
        checksum: String,
    }
    let directory = parent
        .child(".phoenix-diagram-output-cleanup")?
        .ok_or_else(|| disk::conflict("install/unverified-output-namespace", "cleanup"))?;
    directory.private()?;
    if directory.list()? != vec!["owner-v1.json".to_owned()] {
        return Err(disk::conflict(
            "install/unverified-output-namespace",
            directory.path.display().to_string(),
        ));
    }
    let snapshot = directory
        .read("owner-v1.json", &[1])?
        .ok_or_else(|| disk::conflict("install/unverified-output-namespace", "cleanup owner"))?;
    let owner: Owner = parse(&snapshot.bytes, "install/unverified-output-namespace")?;
    let mut value = owner.clone();
    value.checksum.clear();
    let digest = bundle::hash(
        &serde_json::to_vec(&value)
            .map_err(|e| disk::conflict("install/unverified-output-namespace", e.to_string()))?,
    );
    if owner.schema_version != 1
        || owner.tool != "phoenix-diagram"
        || owner.kind != "output-cleanup-directory"
        || owner.parent_path != parent.path
        || owner.parent != parent.physical
        || owner.directory != directory.physical
        || owner.record_identity.device != snapshot.identity.device
        || owner.record_identity.inode != snapshot.identity.inode
        || owner.checksum != digest
        || snapshot.identity.mode != 0o600
        || encoded(&owner)? != snapshot.bytes
    {
        return Err(disk::conflict(
            "install/unverified-output-namespace",
            directory.path.display().to_string(),
        ));
    }
    Ok(())
}
fn publications(
    directory: &Arc<Directory>,
    results: &mut Vec<crate::output::PendingPublication>,
    count: &mut usize,
) -> InstallResult<()> {
    const CLEANUP: &str = ".phoenix-diagram-output-cleanup";
    const RECOVERY: &str = ".phoenix-diagram-recovery-";
    let names = directory.list()?;
    let mut local = Vec::new();
    let mut queried = false;
    for name in names
        .iter()
        .filter(|name| name.starts_with(".phoenix-diagram-lock-"))
    {
        let lock = directory
            .read(name, &[1])?
            .ok_or_else(|| disk::conflict("install/output-lock-changed", name))?;
        let header: serde_json::Value = parse(&lock.bytes, "install/unknown-output-lock")?;
        let target = header
            .get("name")
            .and_then(|value| value.as_str())
            .filter(|value| !value.contains('/') && disk::relative(value).is_ok())
            .ok_or_else(|| disk::conflict("install/unknown-output-lock", name))?;
        if *name
            != format!(
                ".phoenix-diagram-lock-{}.lock",
                bundle::hash(target.as_bytes())
            )
        {
            return Err(disk::conflict("install/unknown-output-lock", name));
        }
        directory.verify()?;
        local.extend(
            crate::output::pending_existing_publications(&directory.path.join(target))
                .map_err(super::transaction::from_output)?,
        );
        directory.verify()?;
        queried = true;
    }
    let mut ids = BTreeSet::new();
    for name in &names {
        if let Some(suffix) = name.strip_prefix(RECOVERY) {
            let id = suffix.strip_suffix(".cleanup.json").unwrap_or(suffix);
            if !valid_id(id) {
                return Err(disk::conflict("install/unverified-output-namespace", name));
            }
            ids.insert(id.to_owned());
        }
    }
    if names.iter().any(|name| name == CLEANUP) {
        if !queried {
            empty_cleanup(directory)?;
        }
        // The shared query has verified the owner checksum, physical directory,
        // parent, and every record. This listing only checks complete coverage.
        let cleanup = directory
            .child(CLEANUP)?
            .ok_or_else(|| disk::conflict("install/output-namespace-changed", CLEANUP))?;
        for name in cleanup.list()? {
            if name == "owner-v1.json" {
                continue;
            }
            let id = name
                .strip_suffix(".json")
                .filter(|id| valid_id(id))
                .ok_or_else(|| disk::conflict("install/unverified-output-namespace", &name))?;
            ids.insert(id.to_owned());
        }
    }
    let observed: BTreeSet<_> = local
        .iter()
        .map(|pending| {
            if pending.recovery_directory.parent() != Some(directory.path.as_path()) {
                return Err(disk::conflict(
                    "install/foreign-output-namespace",
                    pending.recovery_directory.display().to_string(),
                ));
            }
            pending
                .recovery_directory
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_prefix(RECOVERY))
                .filter(|id| valid_id(id))
                .map(str::to_owned)
                .ok_or_else(|| {
                    disk::conflict(
                        "install/unverified-output-namespace",
                        pending.recovery_directory.display().to_string(),
                    )
                })
        })
        .collect::<InstallResult<_>>()?;
    if ids != observed {
        return Err(disk::conflict(
            "install/unverified-output-namespace",
            directory.path.display().to_string(),
        ));
    }
    results.extend(local);
    for name in names {
        *count += 1;
        if *count > 500_000 {
            return Err(disk::conflict(
                "install/state-inventory-incomplete",
                "Installation state inventory exceeds its bound",
            ));
        }
        if name == CLEANUP || name.starts_with(RECOVERY) {
            continue;
        }
        if let Some(child) = directory.child_if_directory(&name)? {
            publications(&child, results, count)?;
        }
    }
    Ok(())
}
pub(super) fn inspect(project: &Arc<Directory>) -> InstallResult<Inventory> {
    let mut result = Inventory::default();
    let Some(state) = project.descend(STATE)? else {
        return Ok(result);
    };
    state.private()?;
    for name in state.list()? {
        if !["active.json", "install.lock", "journals", "transactions"].contains(&name.as_str()) {
            return Err(disk::conflict(
                "install/unknown-state-entry",
                state.path.join(name).display().to_string(),
            ));
        }
    }
    if let Some(lock) = state.read("install.lock", &[1])? {
        if lock.identity.mode != 0o600 || (!lock.bytes.is_empty() && lock.bytes != disk::LOCK_BYTES)
        {
            return Err(disk::conflict("install/unknown-lock", "install.lock"));
        }
    }
    publications(&state, &mut result.publications, &mut 0)?;
    result
        .publications
        .sort_by(|a, b| a.recovery_directory.cmp(&b.recovery_directory));
    result
        .publications
        .dedup_by(|a, b| a.recovery_directory == b.recovery_directory);
    // Recovery locators can describe an interrupted atomic journal update.
    // Do not interpret the temporarily missing/old journal until explicit apply.
    if !result.publications.is_empty() {
        return Ok(result);
    }
    let mut journals = BTreeMap::new();
    if let Some(directory) = project.descend(JOURNALS)? {
        directory.private()?;
        for name in directory.list()? {
            if name == ".phoenix-diagram-output-cleanup" {
                continue;
            }
            if name.starts_with(".phoenix-diagram-lock-") {
                continue;
            }
            let Some(id) = name.strip_suffix(".json").filter(|id| valid_id(id)) else {
                return Err(disk::conflict("install/unknown-journal", name));
            };
            let snapshot = directory
                .read(&name, &[1])?
                .ok_or_else(|| disk::conflict("install/missing-journal", &name))?;
            if snapshot.identity.mode != 0o600 {
                return Err(disk::conflict("install/state-not-private", &name));
            }
            let journal: Journal = parse(&snapshot.bytes, "install/invalid-journal")
                .map_err(|e| disk::conflict(&e.code, e.message))?;
            let transaction = project
                .descend(&format!("{TRANSACTIONS}/{id}"))?
                .ok_or_else(|| disk::conflict("install/missing-transaction", id))?;
            transaction.private()?;
            journal.validate(project, &transaction)?;
            if journal.transaction_id != id {
                return Err(disk::conflict("install/journal-id-mismatch", id));
            }
            if matches!(
                journal.phase,
                Phase::Applying | Phase::Recovering | Phase::RollingBack
            ) {
                result.pending.push(journal.clone());
            }
            journals.insert(id.to_owned(), journal);
        }
    }
    if let Some(transactions) = project.descend(TRANSACTIONS)? {
        transactions.private()?;
        for id in transactions.list()? {
            if !valid_id(&id) {
                return Err(disk::conflict("install/unknown-transaction", id));
            }
            if !journals.contains_key(&id) {
                let transaction = transactions
                    .child(&id)?
                    .ok_or_else(|| disk::conflict("install/unknown-transaction", &id))?;
                transaction.private()?;
                let marker = transaction.read("preparing.json", &[1])?;
                let Some(marker) = marker else {
                    let names = transaction.list()?;
                    let lock_name = format!(
                        ".phoenix-diagram-lock-{}.lock",
                        bundle::hash(b"preparing.json")
                    );
                    if names.is_empty() {
                        continue;
                    }
                    if names == [lock_name.clone()] {
                        let lock = transaction
                            .read(&lock_name, &[1])?
                            .ok_or_else(|| disk::conflict("install/unknown-transaction", &id))?;
                        let expected = serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"tool":"phoenix-diagram","kind":"output-lock","name":"preparing.json","parentDevice":transaction.physical.device,"parentInode":transaction.physical.inode})).map_err(|e| disk::conflict("install/invalid-lock", e.to_string()))?;
                        if lock.identity.mode == 0o600 && lock.bytes == expected {
                            continue;
                        }
                    }
                    return Err(disk::conflict("install/unknown-transaction", id));
                };
                let marker: Preparation = parse(&marker.bytes, "install/unknown-transaction")?;
                if marker.schema_version != 1
                    || marker.tool != "phoenix-diagram"
                    || marker.transaction_id != id
                    || marker.project != project.physical
                    || marker.transaction != transaction.physical
                {
                    return Err(disk::conflict("install/unknown-transaction", id));
                }
                // Preparation alone cannot have changed any public file. Keep
                // this private, identifiable audit directory without adopting it.
            }
        }
    }
    if result.pending.len() > 1 {
        return Err(disk::conflict(
            "install/multiple-pending-transactions",
            "More than one installation needs recovery",
        ));
    }
    if !result.pending.is_empty() {
        return Ok(result);
    }
    if let Some(snapshot) = project.read_path(ACTIVE)? {
        if snapshot.identity.mode != 0o600 {
            return Err(disk::conflict("install/state-not-private", ACTIVE));
        }
        let active: Active = parse(&snapshot.bytes, "install/invalid-active")
            .map_err(|e| disk::conflict(&e.code, e.message))?;
        active.validate(project)?;
        let journal = journals.get(&active.transaction_id).ok_or_else(|| {
            disk::conflict(
                "install/unverified-ownership",
                "Active receipt has no verified installation journal",
            )
        })?;
        if !disk::matches(
            &journal
                .operations
                .last()
                .expect("validated operations")
                .after,
            &snapshot.identity,
        ) || encoded(&active)? != encoded(&journal.active)?
            || journal.phase == Phase::Recovered
        {
            return Err(disk::conflict(
                "install/unverified-ownership",
                "Active receipt does not match the installation journal",
            ));
        }
        result.active = Some((active, snapshot));
    }
    Ok(result)
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Preparation {
    pub schema_version: u32,
    pub tool: String,
    pub transaction_id: String,
    pub project: Physical,
    pub transaction: Physical,
}
