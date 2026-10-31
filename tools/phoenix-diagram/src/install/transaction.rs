// Apache-2.0 ordering adapted from Phoenix UI install/transaction.rs.
// Phoenix diagram stages physical candidates before intent, conditionally
// compensates them after interruption, and commits active.json last.
use super::bundle::parse;
use super::disk::{self, Directory, Snapshot};
use super::plan::{self, Prepared};
use super::state::{self, Active, Journal, Operation, Phase, Preparation};
use super::{
    Checkpoint, InstallConflict, InstallError, InstallErrorKind, InstallReceipt, InstallRequest,
    InstallResult, InstallStatus, Observer,
};
use crate::output::{self, FileIdentity};
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

fn checkpoint_error(error: std::io::Error) -> InstallError {
    InstallError {
        kind: if error.kind() == std::io::ErrorKind::InvalidInput {
            InstallErrorKind::Invalid
        } else {
            InstallErrorKind::Io
        },
        code: "install/acceptance-checkpoint".into(),
        message: error.to_string(),
    }
}
fn observe(
    observer: Observer<'_>,
    checkpoint: Checkpoint,
    project: &Directory,
) -> InstallResult<()> {
    observer(checkpoint, &project.path).map_err(|error| InstallError {
        kind: InstallErrorKind::Io,
        code: "install/observer-interruption".into(),
        message: error.to_string(),
    })
}
pub(super) fn emit(
    observer: Observer<'_>,
    checkpoint: Checkpoint,
    project: &Directory,
) -> InstallResult<()> {
    observe(observer, checkpoint, project)?;
    output::acceptance_checkpoint("", &project.path, &project.path).map_err(checkpoint_error)
}
fn checkpoint_selected(phase: &str) -> bool {
    use std::ffi::OsStr;
    std::env::var_os("PHOENIX_DIAGRAM_ACCEPTANCE").as_deref() == Some(OsStr::new("1"))
        && std::env::var_os("PHOENIX_DIAGRAM_CHECKPOINT_DIR").is_some_and(|value| !value.is_empty())
        && std::env::var_os("PHOENIX_DIAGRAM_CHECKPOINT_PHASE").as_deref()
            == Some(OsStr::new(phase))
}
pub(super) fn emit_transaction(
    observer: Observer<'_>,
    checkpoint: Checkpoint,
    project: &Directory,
    transaction: &Directory,
    target: &std::path::Path,
) -> InstallResult<()> {
    let phase = match &checkpoint {
        Checkpoint::JournalReady => Some("install-journal-ready"),
        Checkpoint::AfterRetire(_) => Some("install-after-retire"),
        Checkpoint::AfterLink(_) => Some("install-after-link"),
        Checkpoint::BeforeActive => Some("install-before-active"),
        Checkpoint::AfterActive => Some("install-after-active"),
        Checkpoint::Committed => Some("install-committed"),
        Checkpoint::BeforeLegacyRemove(_) => Some("migrate-before-legacy-remove"),
        Checkpoint::AfterLegacyRemove(_) => Some("migrate-after-legacy-remove"),
        Checkpoint::AfterRollback(_) => Some("rollback-after-file"),
        Checkpoint::AfterRollbackJournalRetire => Some("rollback-old-journal-retired"),
        Checkpoint::AfterRollbackJournalPublish => Some("rollback-old-journal-published"),
        Checkpoint::RollbackAuditArchived => Some("rollback-audit-archived"),
        Checkpoint::RollbackCompatibilityPublished => Some("rollback-compat-published"),
        _ => None,
    };
    observe(observer, checkpoint, project)?;
    if phase.is_some_and(checkpoint_selected) {
        transaction.private()?;
    }
    output::acceptance_checkpoint(phase.unwrap_or(""), target, &transaction.path)
        .map_err(checkpoint_error)
}
pub(super) fn from_output(error: output::OutputError) -> InstallError {
    InstallError {
        kind: match error.kind {
            output::ErrorKind::Invalid => InstallErrorKind::Invalid,
            output::ErrorKind::Conflict => InstallErrorKind::Conflict,
            output::ErrorKind::Busy => InstallErrorKind::Busy,
            output::ErrorKind::Io => InstallErrorKind::Io,
            output::ErrorKind::RecoveryRequired => InstallErrorKind::RecoveryRequired,
        },
        code: format!("install/{}", error.code),
        message: error.message,
    }
}
pub(super) fn persist(
    directory: &Arc<Directory>,
    name: &str,
    bytes: &[u8],
    mode: u32,
    previous: Option<&FileIdentity>,
) -> InstallResult<FileIdentity> {
    directory.verify()?;
    let slot = output::capture_slot(&directory.path.join(name), &[]).map_err(from_output)?;
    let matches = match (previous, slot.previous()) {
        (None, None) => true,
        (Some(expected), Some(current)) => disk::matches(expected, current.identity()),
        _ => false,
    };
    if !matches {
        return Err(disk::conflict(
            "install/state-changed",
            directory.path.join(name).display().to_string(),
        ));
    }
    directory.verify()?;
    let publication =
        output::publish_bound_with_mode(&slot, bytes, mode, &[]).map_err(from_output)?;
    if !publication.cleanup_complete {
        return Err(InstallError {
            kind: InstallErrorKind::RecoveryRequired,
            code: "install/state-publication-pending".into(),
            message: format!(
                "Private installation-state publication needs recovery: {:?}",
                publication.recovery_directory
            ),
        });
    }
    directory.verify()?;
    let actual = directory
        .read(name, &[1])?
        .ok_or_else(|| disk::conflict("install/state-absent", name))?;
    if !disk::matches(&publication.identity, &actual.identity) {
        return Err(disk::conflict("install/state-changed", name));
    }
    Ok(actual.identity)
}
pub(super) fn save(
    project: &Arc<Directory>,
    journal: &mut Journal,
    expected: &mut Option<FileIdentity>,
) -> InstallResult<()> {
    journal.seal()?;
    let directory = project
        .descend(state::JOURNALS)?
        .ok_or_else(|| disk::conflict("install/missing-journals", state::JOURNALS))?;
    *expected = Some(persist(
        &directory,
        &format!("{}.json", journal.transaction_id),
        &state::encoded(journal)?,
        0o600,
        expected.as_ref(),
    )?);
    Ok(())
}
pub(super) fn id() -> InstallResult<String> {
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|e| InstallError {
            kind: InstallErrorKind::Io,
            code: "install/random-transaction-id".into(),
            message: e.to_string(),
        })?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub(super) fn directory(project: &Arc<Directory>, id: &str) -> InstallResult<Arc<Directory>> {
    project
        .descend(&format!("{}/{id}", state::TRANSACTIONS))?
        .ok_or_else(|| disk::conflict("install/missing-transaction", id))
}
pub(super) fn subdirectory(
    transaction: &Arc<Directory>,
    name: &str,
) -> InstallResult<Arc<Directory>> {
    transaction
        .descend(name)?
        .ok_or_else(|| disk::conflict("install/missing-transaction-directory", name))
}
pub(super) fn verify_files(project: &Arc<Directory>, active: &Active) -> InstallResult<()> {
    for owned in &active.ownership {
        let actual = project
            .read_path(&owned.path)?
            .ok_or_else(|| disk::conflict("install/managed-file-missing", &owned.path))?;
        if !disk::matches(&owned.identity, &actual.identity) {
            return Err(disk::conflict("install/managed-file-modified", &owned.path));
        }
    }
    plan::verify_inventory(project, active)
}
pub(super) fn retire(
    source: &Arc<Directory>,
    name: &str,
    target: &Arc<Directory>,
    retired: &str,
    expected: &FileIdentity,
    links: &[u64],
) -> InstallResult<()> {
    disk::expected(source, name, expected, links)?;
    disk::rename(source, name, target, retired)?;
    if let Err(error) = disk::expected(target, retired, expected, links) {
        let restored = disk::rename(target, retired, source, name);
        return Err(if restored.is_ok() {
            error
        } else {
            disk::conflict(
                "install/displaced-claimant-preserved",
                target.path.join(retired).display().to_string(),
            )
        });
    }
    Ok(())
}
pub(super) fn target_parent(
    project: &Arc<Directory>,
    path: &str,
    known: &mut BTreeMap<String, Arc<Directory>>,
) -> InstallResult<Arc<Directory>> {
    let mut parent = project.clone();
    let mut relative = String::new();
    for part in path.split('/') {
        if !relative.is_empty() {
            relative.push('/');
        }
        relative.push_str(part);
        parent = if let Some(directory) = known.get(&relative) {
            directory.verify()?;
            directory.clone()
        } else {
            let directory = parent.create_child(part, false)?;
            known.insert(relative.clone(), directory.clone());
            directory
        };
    }
    Ok(parent)
}
pub(super) fn publish(
    project: &Arc<Directory>,
    transaction: &Arc<Directory>,
    operation: &Operation,
    index: usize,
    known: &mut BTreeMap<String, Arc<Directory>>,
    observer: Observer<'_>,
) -> InstallResult<()> {
    let (parent_path, name) = operation
        .path
        .rsplit_once('/')
        .expect("validated project path");
    let parent = if operation.path == state::ACTIVE {
        project
            .descend(parent_path)?
            .ok_or_else(|| disk::conflict("install/parent-changed", parent_path))?
    } else {
        target_parent(project, parent_path, known)?
    };
    let after = subdirectory(transaction, "after")?;
    let retired = subdirectory(transaction, "retired")?;
    let index = index.to_string();
    match &operation.before {
        Some(before) => retire(&parent, name, &retired, &index, before, &[1])?,
        None if parent.entry(name)?.is_some() => {
            return Err(disk::conflict("install/later-claimant", &operation.path))
        }
        None => {}
    }
    emit_transaction(
        observer,
        Checkpoint::AfterRetire(operation.path.clone()),
        project,
        transaction,
        &project.path.join(&operation.path),
    )?;
    disk::expected(&after, &index, &operation.after, &[1])?;
    disk::link(&after, &index, &parent, name)?;
    disk::expected(&parent, name, &operation.after, &[2])?;
    emit_transaction(
        observer,
        Checkpoint::AfterLink(operation.path.clone()),
        project,
        transaction,
        &project.path.join(&operation.path),
    )?;
    disk::remove_known(
        &after,
        &index,
        &format!("{index}.discard"),
        &operation.after,
        &[2],
    )?;
    disk::expected(&parent, name, &operation.after, &[1])?;
    Ok(())
}
pub(super) fn stage(
    transaction: &Arc<Directory>,
    index: usize,
    path: String,
    bytes: &[u8],
    mode: u32,
    before: Option<&Snapshot>,
) -> InstallResult<Operation> {
    let after = subdirectory(transaction, "after")?;
    let backups = subdirectory(transaction, "before")?;
    let name = index.to_string();
    let after = persist(&after, &name, bytes, mode, None)?;
    let backup = before
        .map(|snapshot| persist(&backups, &name, &snapshot.bytes, 0o600, None))
        .transpose()?;
    Ok(Operation {
        kind: Some("file-write".into()),
        path,
        stage_path: format!("after/{index}"),
        backup_path: before.map(|_| format!("before/{index}")),
        retired_path: format!("retired/{index}"),
        undo_path: format!("undo/{index}"),
        before: before.map(|s| s.identity.clone()),
        after,
        backup,
    })
}
pub(super) fn run(
    prepared: &mut Prepared,
    lock: &disk::Lock,
    mut migration: Option<super::migration::Migration>,
    observer: Observer<'_>,
) -> InstallResult<()> {
    lock.verify()?;
    prepared.bundle.verify()?;
    let id = id()?;
    prepared.project.ensure(state::JOURNALS, true)?;
    let transactions = prepared.project.ensure(state::TRANSACTIONS, true)?;
    let transaction = transactions.create_child(&id, true)?;
    prepared.receipt.transaction_id = Some(id.clone());
    prepared.receipt.journal = Some(
        prepared
            .project
            .path
            .join(format!("{}/{id}.json", state::JOURNALS)),
    );
    prepared.receipt.backup = Some(transaction.path.join("before"));
    let marker = Preparation {
        schema_version: 1,
        tool: "phoenix-diagram".into(),
        transaction_id: id.clone(),
        project: prepared.project.physical.clone(),
        transaction: transaction.physical.clone(),
    };
    persist(
        &transaction,
        "preparing.json",
        &state::encoded(&marker)?,
        0o600,
        None,
    )?;
    for name in ["after", "before", "retired", "undo"] {
        transaction.ensure(name, true)?;
    }
    let mut operations = vec![];
    let mut ownership = prepared.carried.clone();
    for change in &prepared.changes {
        let operation = stage(
            &transaction,
            operations.len(),
            change.path.clone(),
            &change.bytes,
            change.mode,
            change.before.as_ref(),
        )?;
        ownership.insert(change.path.clone(), operation.after.clone());
        operations.push(operation);
    }
    let active = plan::active(prepared, &id, ownership)?;
    let active_before = prepared
        .inventory
        .active
        .as_ref()
        .map(|(_, snapshot)| snapshot);
    operations.push(stage(
        &transaction,
        operations.len(),
        state::ACTIVE.into(),
        &state::encoded(&active)?,
        0o600,
        active_before,
    )?);
    if let Some(migration) = &mut migration {
        super::migration::bind_parents(prepared, migration, &operations)?;
    }
    let mut journal = Journal {
        schema_version: 2,
        tool: "phoenix-diagram".into(),
        transaction_id: id,
        project: prepared.project.physical.clone(),
        transaction: transaction.physical.clone(),
        phase: Phase::Applying,
        completed: 0,
        pending: None,
        operations,
        migration,
        rollback: None,
        active,
        checksum: String::new(),
    };
    journal.seal()?;
    journal.validate(&prepared.project, &transaction)?;
    let mut journal_identity = None;
    save(&prepared.project, &mut journal, &mut journal_identity)?;
    emit_transaction(
        observer,
        Checkpoint::JournalReady,
        &prepared.project,
        &transaction,
        &prepared.project.path.join(&journal.active.journal),
    )?;
    if journal.migration.is_some() {
        super::migration::remove_legacy(
            &prepared.project,
            &transaction,
            &mut journal,
            &mut journal_identity,
            lock,
            observer,
        )?;
    }
    for index in 0..journal.operations.len() {
        lock.verify()?;
        journal.pending = Some(index);
        save(&prepared.project, &mut journal, &mut journal_identity)?;
        let operation = &journal.operations[index];
        emit(
            observer,
            Checkpoint::BeforePublish(operation.path.clone()),
            &prepared.project,
        )?;
        if operation.path == state::ACTIVE {
            emit_transaction(
                observer,
                Checkpoint::BeforeActive,
                &prepared.project,
                &transaction,
                &prepared.project.path.join(state::ACTIVE),
            )?;
        }
        publish(
            &prepared.project,
            &transaction,
            operation,
            index,
            &mut prepared.directories,
            observer,
        )?;
        emit(
            observer,
            Checkpoint::AfterPublish(operation.path.clone()),
            &prepared.project,
        )?;
        if operation.path == state::ACTIVE {
            emit_transaction(
                observer,
                Checkpoint::AfterActive,
                &prepared.project,
                &transaction,
                &prepared.project.path.join(state::ACTIVE),
            )?;
        }
        journal.completed = index + 1;
        journal.pending = None;
        save(&prepared.project, &mut journal, &mut journal_identity)?;
    }
    lock.verify()?;
    prepared.bundle.verify()?;
    verify_files(&prepared.project, &journal.active)?;
    super::migration::verify_removed(&prepared.project, &journal)?;
    journal.phase = Phase::Committed;
    save(&prepared.project, &mut journal, &mut journal_identity)?;
    prepared.receipt.status = InstallStatus::Installed;
    emit_transaction(
        observer,
        Checkpoint::Committed,
        &prepared.project,
        &transaction,
        &prepared.project.path.join(state::ACTIVE),
    )?;
    Ok(())
}

pub(super) fn apply(
    request: &InstallRequest,
    observer: Observer<'_>,
) -> InstallResult<InstallReceipt> {
    let prepared = plan::prepare(request)?;
    if matches!(
        prepared.receipt.status,
        InstallStatus::Conflict | InstallStatus::Unchanged
    ) {
        return Ok(prepared.receipt);
    }
    // This first complete preflight is read-only. Conflict never creates a lock
    // or state directory. The second preflight runs while holding the OS lock.
    let state = prepared.project.ensure(state::STATE, true)?;
    let lock = state.lock()?;
    emit(observer, Checkpoint::LockAcquired, &prepared.project)?;
    let mut recovery = recover_locked(&prepared.project, &lock, observer)?;
    recovery.command = "install".into();
    recovery.recovery_command = prepared.receipt.recovery_command.clone();
    if matches!(
        recovery.status,
        InstallStatus::Conflict | InstallStatus::RecoveryRequired
    ) {
        return Ok(recovery);
    }
    lock.verify()?;
    let mut prepared = plan::prepare(request)?;
    prepared.receipt.recovered = recovery.recovered;
    if prepared.receipt.status != InstallStatus::Planned {
        return Ok(prepared.receipt);
    }
    if let Err(error) = run(&mut prepared, &lock, None, observer) {
        prepared.receipt.status = InstallStatus::RecoveryRequired;
        prepared.receipt.conflicts.push(InstallConflict {
            path: prepared.receipt.journal.as_ref().map_or_else(
                || state.path.display().to_string(),
                |p| p.display().to_string(),
            ),
            code: error.code,
            message: error.message,
        });
        super::diagnostics(&mut prepared.receipt);
    }
    Ok(prepared.receipt)
}

fn recover_operation(
    project: &Arc<Directory>,
    transaction: &Arc<Directory>,
    operation: &Operation,
    index: usize,
    observer: Observer<'_>,
) -> InstallResult<()> {
    let name = index.to_string();
    let after = subdirectory(transaction, "after")?;
    let retired = subdirectory(transaction, "retired")?;
    let undo = subdirectory(transaction, "undo")?;
    let Some((parent, target)) = project.parent(&operation.path)? else {
        if operation.before.is_some() {
            return Err(disk::conflict(
                "install/recovery-parent-missing",
                &operation.path,
            ));
        }
        disk::remove_known(
            &after,
            &name,
            &format!("{name}.discard"),
            &operation.after,
            &[1],
        )?;
        return Ok(());
    };
    let current = parent.read(&target, &[1, 2])?;
    let before_matches = current
        .as_ref()
        .zip(operation.before.as_ref())
        .is_some_and(|(current, before)| disk::matches(before, &current.identity));
    let after_matches = current
        .as_ref()
        .is_some_and(|current| disk::matches(&operation.after, &current.identity));
    if let Some(current) = &current {
        if current.identity.links == 2 {
            let (known, identity) = if after_matches {
                (&after, &operation.after)
            } else if before_matches {
                (&retired, operation.before.as_ref().expect("matched before"))
            } else {
                return Err(disk::conflict(
                    "install/recovery-later-modification",
                    &operation.path,
                ));
            };
            let private = known
                .read(&name, &[2])?
                .or(known.read(&format!("{name}.discard"), &[2])?);
            if private
                .as_ref()
                .is_none_or(|snapshot| !disk::matches(identity, &snapshot.identity))
            {
                return Err(disk::conflict(
                    "install/recovery-unknown-hardlink",
                    &operation.path,
                ));
            }
        }
    }
    if current.is_some() && !before_matches && !after_matches {
        return Err(disk::conflict(
            "install/recovery-later-modification",
            &operation.path,
        ));
    }
    emit(
        observer,
        Checkpoint::BeforeRecover(operation.path.clone()),
        project,
    )?;
    if after_matches {
        let links = current.as_ref().expect("matched current").identity.links;
        retire(&parent, &target, &undo, &name, &operation.after, &[links])?;
        emit(
            observer,
            Checkpoint::AfterRecoverRetire(operation.path.clone()),
            project,
        )?;
    }
    disk::remove_known(
        &undo,
        &name,
        &format!("{name}.discard"),
        &operation.after,
        &[1, 2],
    )?;
    if !before_matches {
        if let Some(before) = &operation.before {
            disk::expected(&retired, &name, before, &[1])?;
            disk::link(&retired, &name, &parent, &target)?;
            disk::expected(&parent, &target, before, &[2])?;
            emit(
                observer,
                Checkpoint::AfterRecoverLink(operation.path.clone()),
                project,
            )?;
        }
    }
    if let Some(before) = &operation.before {
        // A killed restore may already have linked the original before inode.
        // Retire its private link only after proving that public is that inode.
        let restored = disk::expected(&parent, &target, before, &[1, 2])?;
        if retired.entry(&name)?.is_some() || retired.entry(&format!("{name}.discard"))?.is_some() {
            disk::remove_known(
                &retired,
                &name,
                &format!("{name}.discard"),
                before,
                &[restored.identity.links],
            )?;
        }
    }
    disk::remove_known(
        &after,
        &name,
        &format!("{name}.discard"),
        &operation.after,
        &[1],
    )?;
    emit(
        observer,
        Checkpoint::AfterRecover(operation.path.clone()),
        project,
    )?;
    Ok(())
}

fn output_target_allowed(project: &Directory, value: &serde_json::Value) -> bool {
    let Some(path) = value.pointer("/target/commitPath").and_then(|v| v.as_str()) else {
        return false;
    };
    let Ok(relative) = std::path::Path::new(path).strip_prefix(project.path.join(state::STATE))
    else {
        return false;
    };
    let Some(relative) = relative.to_str() else {
        return false;
    };
    let parts: Vec<_> = relative.split('/').collect();
    match parts.as_slice() {
        ["journals", name] => name.strip_suffix(".json").is_some_and(state::valid_id),
        ["transactions", id, "preparing.json"] => state::valid_id(id),
        ["transactions", id, directory, index] => {
            state::valid_id(id)
                && (matches!(*directory, "before" | "after" | "undo")
                    || directory
                        .strip_prefix("rollback-")
                        .is_some_and(state::valid_id))
                && index
                    .parse::<usize>()
                    .is_ok_and(|n| n.to_string() == *index)
        }
        _ => false,
    }
}
pub(super) fn recover_publications(
    project: &Arc<Directory>,
    inventory: &state::Inventory,
    receipt: &mut InstallReceipt,
) -> InstallResult<()> {
    for pending in &inventory.publications {
        let locator = &pending.recovery_directory;
        let record_path = pending
            .witness
            .clone()
            .unwrap_or_else(|| locator.join("publication-recovery-v1.json"));
        let relative = record_path
            .strip_prefix(&project.path)
            .ok()
            .and_then(|path| path.to_str())
            .ok_or_else(|| {
                disk::conflict(
                    "install/foreign-publication-record",
                    record_path.display().to_string(),
                )
            })?;
        let record = project.read_path(relative)?.ok_or_else(|| {
            disk::conflict(
                "install/unknown-publication-record",
                locator.display().to_string(),
            )
        })?;
        let value: serde_json::Value = parse(&record.bytes, "install/unknown-publication-record")?;
        if !output_target_allowed(project, &value) {
            return Err(disk::conflict(
                "install/foreign-publication-record",
                locator.display().to_string(),
            ));
        }
        let result = output::recover_publication(locator);
        if result.exit_code() != 0 {
            return Err(disk::conflict(
                "install/state-recovery-required",
                format!("{}: {}", locator.display(), result.message),
            ));
        }
        receipt
            .recovered
            .push(format!("private-publication:{}", locator.display()));
    }
    Ok(())
}
fn recover_locked(
    project: &Arc<Directory>,
    lock: &disk::Lock,
    observer: Observer<'_>,
) -> InstallResult<InstallReceipt> {
    lock.verify()?;
    let mut receipt = super::receipt("install-recovery", &project.path);
    receipt.status = InstallStatus::Unchanged;
    let initial = state::inspect(project)?;
    recover_publications(project, &initial, &mut receipt)?;
    let inventory = state::inspect(project)?;
    for mut journal in inventory.pending {
        if journal.migration.is_some() || journal.phase == Phase::RollingBack {
            receipt.status = InstallStatus::RecoveryRequired;
            receipt.transaction_id = Some(journal.transaction_id.clone());
            receipt.recovery_command = Some(format!(
                "{} migrate --recover {} --project {} --json",
                std::env::current_exe()
                    .map(|p| super::quote(&p))
                    .unwrap_or_else(|_| "phoenix-diagram".into()),
                journal.transaction_id,
                super::quote(&project.path)
            ));
            return Ok(receipt);
        }
        lock.verify()?;
        let transaction = directory(project, &journal.transaction_id)?;
        journal.validate(project, &transaction)?;
        receipt.transaction_id = Some(journal.transaction_id.clone());
        receipt.journal = Some(project.path.join(&journal.active.journal));
        receipt.backup = Some(project.path.join(&journal.active.backup));
        receipt.engine = Some(journal.active.engine.clone());
        receipt.npm_package_version = Some(journal.active.npm_package_version.clone());
        receipt.providers = journal.active.providers.clone();
        receipt.bundle_manifest_sha256 = Some(journal.active.bundle_manifest_sha256.clone());
        receipt.version_manifest_sha256 = Some(journal.active.engine_manifest_sha256.clone());
        let snapshot = project
            .read_path(&journal.active.journal)?
            .ok_or_else(|| disk::conflict("install/missing-journal", &journal.transaction_id))?;
        let mut journal_identity = Some(snapshot.identity);
        let active_after = &journal.operations.last().expect("validated active").after;
        let active_parent = project
            .descend(state::STATE)?
            .ok_or_else(|| disk::conflict("install/parent-changed", state::STATE))?;
        let active_snapshot = active_parent.read("active.json", &[1, 2])?;
        let completed = active_snapshot
            .is_some_and(|current| disk::matches(active_after, &current.identity))
            && verify_files(project, &journal.active).is_ok();
        if completed {
            let after = subdirectory(&transaction, "after")?;
            let name = (journal.operations.len() - 1).to_string();
            if after.entry(&name)?.is_some() || after.entry(&format!("{name}.discard"))?.is_some() {
                disk::remove_known(
                    &after,
                    &name,
                    &format!("{name}.discard"),
                    active_after,
                    &[2],
                )?;
            }
            disk::expected(&active_parent, "active.json", active_after, &[1])?;
            journal.phase = Phase::Committed;
            journal.completed = journal.operations.len();
            journal.pending = None;
            save(project, &mut journal, &mut journal_identity)?;
            receipt
                .recovered
                .push(format!("activated:{}", journal.transaction_id));
            receipt.status = InstallStatus::Recovered;
            continue;
        }
        journal.phase = Phase::Recovering;
        save(project, &mut journal, &mut journal_identity)?;
        for index in (0..journal.operations.len()).rev() {
            lock.verify()?;
            journal.pending = Some(index);
            save(project, &mut journal, &mut journal_identity)?;
            match recover_operation(
                project,
                &transaction,
                &journal.operations[index],
                index,
                observer,
            ) {
                Ok(()) => {}
                Err(error) => receipt.conflicts.push(InstallConflict {
                    path: journal.operations[index].path.clone(),
                    code: error.code,
                    message: error.message,
                }),
            }
            journal.pending = None;
            save(project, &mut journal, &mut journal_identity)?;
        }
        if receipt.conflicts.is_empty() {
            journal.phase = Phase::Recovered;
            save(project, &mut journal, &mut journal_identity)?;
            receipt
                .recovered
                .push(format!("rolled-back:{}", journal.transaction_id));
            receipt.status = InstallStatus::Recovered;
        } else {
            receipt.status = InstallStatus::Conflict;
        }
    }
    if receipt.status == InstallStatus::Unchanged && !receipt.recovered.is_empty() {
        receipt.status = InstallStatus::Recovered;
    }
    super::diagnostics(&mut receipt);
    Ok(receipt)
}
pub(super) fn recover(
    project: &std::path::Path,
    observer: Observer<'_>,
) -> InstallResult<InstallReceipt> {
    let project = Directory::open(project)?;
    let initial = state::inspect(&project)?;
    if initial.pending.is_empty() && initial.publications.is_empty() {
        let mut receipt = super::receipt("install-recovery", &project.path);
        receipt.status = InstallStatus::Unchanged;
        return Ok(receipt);
    }
    let state = project
        .descend(state::STATE)?
        .ok_or_else(|| disk::conflict("install/parent-changed", state::STATE))?;
    let lock = state.lock()?;
    recover_locked(&project, &lock, observer)
}
