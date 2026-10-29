//! Project-local diagram installation, isolated from the Phoenix UI installer.
//!
//! Provider selection and journal ordering are adapted from
//! tools/phoenix-ui/crates/install/src/{cli,transaction,disk}.rs (Apache-2.0).
//! Phoenix changes: diagram-only file allowlist, independent state, no hooks,
//! settings, personal installation, or cache management.

use crate::diagnostic::Diagnostics;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[path = "install/bundle.rs"]
mod bundle;
#[path = "install/disk.rs"]
mod disk;
#[path = "install/migration.rs"]
mod migration;
#[path = "install/plan.rs"]
mod plan;
#[path = "install/state.rs"]
mod state;
#[path = "install/transaction.rs"]
mod transaction;

pub type InstallResult<T> = Result<T, InstallError>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallRequest {
    pub project: PathBuf,
    pub bundle_root: PathBuf,
    pub version_manifest: PathBuf,
    pub engine: PathBuf,
    pub providers: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationMode {
    FromArchify,
    Recover,
    Rollback,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigrationRequest {
    pub mode: MigrationMode,
    pub project: PathBuf,
    pub providers: Option<Vec<String>>,
    pub bundle_root: Option<PathBuf>,
    pub transaction_id: Option<String>,
    pub dry_run: bool,
    pub confirm_remove_archify: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyRemoval {
    pub path: String,
    pub source: String,
    pub status: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallStatus {
    Planned,
    Installed,
    Unchanged,
    Conflict,
    RecoveryRequired,
    Recovered,
    RolledBack,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileDigest {
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallEngine {
    pub source_input_sha256: String,
    pub input_contract_schema: u32,
    pub receipt_schema: u32,
    pub viewer_revision: String,
    pub binary: FileDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallWrite {
    pub path: String,
    pub after: FileDigest,
    pub mode: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallConflict {
    pub path: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: InstallStatus,
    pub diagnostics: Diagnostics,
    pub engine: Option<InstallEngine>,
    pub npm_package_version: Option<String>,
    pub project: PathBuf,
    pub providers: Vec<String>,
    pub bundle_manifest_sha256: Option<String>,
    pub version_manifest_sha256: Option<String>,
    pub writes: Vec<InstallWrite>,
    pub unchanged: Vec<String>,
    pub conflicts: Vec<InstallConflict>,
    pub transaction_id: Option<String>,
    pub journal: Option<PathBuf>,
    pub backup: Option<PathBuf>,
    pub recovered: Vec<String>,
    pub reload_steps: Vec<String>,
    pub recovery_command: Option<String>,
    pub local_ignore: Vec<String>,
    pub transaction_schema_version: u32,
    pub migration_sources: Vec<String>,
    pub removals: Vec<LegacyRemoval>,
    pub preserved: Vec<String>,
    pub rollback_command: Option<String>,
    pub legacy_backup_policy: String,
    pub legacy_restoration: String,
    pub reinstall_instructions: Vec<String>,
}
impl InstallReceipt {
    pub fn exit_code(&self) -> u8 {
        match self.status {
            InstallStatus::Conflict | InstallStatus::RecoveryRequired => 3,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallErrorKind {
    Invalid,
    Conflict,
    Busy,
    RecoveryRequired,
    Io,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallError {
    pub kind: InstallErrorKind,
    pub code: String,
    pub message: String,
}
impl InstallError {
    pub fn exit_code(&self) -> u8 {
        match self.kind {
            InstallErrorKind::Invalid => 2,
            InstallErrorKind::Conflict
            | InstallErrorKind::Busy
            | InstallErrorKind::RecoveryRequired => 3,
            InstallErrorKind::Io => 4,
        }
    }
}
impl std::fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for InstallError {}

/// Read-only: never creates the state directory, an OS lock, or a journal.
pub fn plan_install(request: &InstallRequest) -> InstallResult<InstallReceipt> {
    plan::prepare(request).map(|prepared| prepared.receipt)
}

/// An explicit installation recovers pending work, then replans under the lock.
pub fn apply_install(request: &InstallRequest) -> InstallResult<InstallReceipt> {
    transaction::apply(request, &mut |_, _| Ok(()))
}

/// Conditional compensation only; this is not a user-selected rollback API.
pub fn recover_install(project: &Path) -> InstallResult<InstallReceipt> {
    transaction::recover(project, &mut |_, _| Ok(()))
}

pub fn migrate(request: &MigrationRequest) -> InstallResult<InstallReceipt> {
    migration::migrate(request, &mut |_, _| Ok(()))
}
pub fn recover_migration(project: &Path, id: &str) -> InstallResult<InstallReceipt> {
    migration::recover(project, id, &mut |_, _| Ok(()))
}
pub fn rollback(project: &Path, id: &str, dry_run: bool) -> InstallResult<InstallReceipt> {
    migration::rollback(project, id, dry_run, &mut |_, _| Ok(()))
}

fn invalid(code: &str, message: impl Into<String>) -> InstallError {
    InstallError {
        kind: InstallErrorKind::Invalid,
        code: code.into(),
        message: message.into(),
    }
}
fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\"'\"'"))
}
fn receipt(command: &str, project: &Path) -> InstallReceipt {
    InstallReceipt {
        schema_version: 1, tool: "phoenix-diagram".into(), tool_version: crate::TOOL_VERSION.into(), command: command.into(), status: InstallStatus::Planned,
        diagnostics: vec![], engine: None, npm_package_version: None, project: project.into(), providers: vec![],
        bundle_manifest_sha256: None, version_manifest_sha256: None, writes: vec![], unchanged: vec![], conflicts: vec![], transaction_id: None,
        journal: None, backup: None, recovered: vec![], reload_steps: vec!["Reload the project in a new provider session and verify the loaded phoenix-diagram skill path.".into()],
        transaction_schema_version: 2, migration_sources: vec![], removals: vec![], preserved: vec![], rollback_command: None,
        legacy_backup_policy: "none".into(), legacy_restoration: "not-supported".into(),
        reinstall_instructions: vec![],
        recovery_command: None, local_ignore: vec!["Keep .phoenix-diagram/install/ machine-local in your local ignore configuration; the installer does not edit shared ignore files.".into()],
    }
}
fn diagnostics(receipt: &mut InstallReceipt) {
    receipt.diagnostics = receipt
        .conflicts
        .iter()
        .map(|conflict| {
            let mut diagnostic = crate::diagnostic::Diagnostic::error(
                None,
                &conflict.code,
                &conflict.path,
                &conflict.message,
            );
            diagnostic.stage = "install".into();
            diagnostic
        })
        .collect();
    if receipt.status == InstallStatus::RecoveryRequired && receipt.diagnostics.is_empty() {
        let mut diagnostic=crate::diagnostic::Diagnostic::error(None,"install/recovery-required",&receipt.project.display().to_string(),"A pending diagram installation must be recovered by the next explicit install; dry-run does not recover it");
        diagnostic.stage = "install".into();
        receipt.diagnostics.push(diagnostic);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Checkpoint {
    LockAcquired,
    JournalReady,
    BeforePublish(String),
    AfterRetire(String),
    AfterLink(String),
    AfterPublish(String),
    BeforeActive,
    AfterActive,
    Committed,
    BeforeLegacyRemove(String),
    AfterLegacyRemove(String),
    BeforeRollback(String),
    AfterRollback(String),
    AfterRollbackJournalRetire,
    AfterRollbackJournalPublish,
    RollbackAuditArchived,
    RollbackCompatibilityPublished,
    BeforeRecover(String),
    AfterRecoverRetire(String),
    AfterRecoverLink(String),
    AfterRecover(String),
}
type Observer<'a> = &'a mut dyn FnMut(Checkpoint, &Path) -> std::io::Result<()>;

#[cfg(test)]
// Library-only unit targets leave these fault hooks unused; the integration
// harness includes this module as source and exercises them on temporary files.
#[allow(dead_code, unused_imports)]
pub(crate) mod test_support {
    pub(crate) use super::Checkpoint;
    pub(crate) fn apply(
        request: &super::InstallRequest,
        observer: super::Observer<'_>,
    ) -> super::InstallResult<super::InstallReceipt> {
        super::transaction::apply(request, observer)
    }
    pub(crate) fn recover(
        project: &std::path::Path,
        observer: super::Observer<'_>,
    ) -> super::InstallResult<super::InstallReceipt> {
        super::transaction::recover(project, observer)
    }
    pub(crate) fn migrate(
        request: &super::InstallRequest,
        confirm: bool,
        dry_run: bool,
        observer: super::Observer<'_>,
    ) -> super::InstallResult<super::InstallReceipt> {
        super::migration::from_install(request, confirm, dry_run, observer)
    }
    pub(crate) fn recover_migration(
        project: &std::path::Path,
        id: &str,
        observer: super::Observer<'_>,
    ) -> super::InstallResult<super::InstallReceipt> {
        super::migration::recover(project, id, observer)
    }
    pub(crate) fn rollback(
        project: &std::path::Path,
        id: &str,
        dry_run: bool,
        observer: super::Observer<'_>,
    ) -> super::InstallResult<super::InstallReceipt> {
        super::migration::rollback(project, id, dry_run, observer)
    }
    pub(crate) fn v1_fixture(project: &std::path::Path) {
        super::migration::v1_fixture(project).unwrap();
    }
    pub(crate) fn hold_lock(
        project: &std::path::Path,
        observer: super::Observer<'_>,
    ) -> super::InstallResult<()> {
        let project = super::disk::Directory::open(project)?;
        let state = project.ensure(super::state::STATE, true)?;
        let _lock = state.lock()?;
        observer(super::Checkpoint::LockAcquired, &project.path).map_err(|error| {
            super::InstallError {
                kind: super::InstallErrorKind::Io,
                code: "install/observer-interruption".into(),
                message: error.to_string(),
            }
        })
    }
}
