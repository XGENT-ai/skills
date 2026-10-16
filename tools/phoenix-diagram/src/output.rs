// Conservative descriptor-bound publication, adapted from the frozen Archify
// atomic-output.mjs. Phoenix adds OS locks and a public, private recovery record.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[path = "acceptance.rs"]
mod acceptance;
#[path = "output/transaction.rs"]
mod transaction;
#[path = "output/unix.rs"]
mod unix;
use unix::{Directory, SlotLock};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileIdentity {
    pub device: String,
    pub inode: String,
    pub links: u64,
    pub mode: u32,
    pub uid: u32,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PublicState {
    Absent,
    File { identity: FileIdentity },
    Untrusted { entry_type: String },
    Unknown { reason: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorKind {
    Invalid,
    Conflict,
    Busy,
    Io,
    RecoveryRequired,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputError {
    pub kind: ErrorKind,
    pub code: String,
    pub message: String,
    pub public_state: PublicState,
    pub recovery_directory: Option<PathBuf>,
    pub preserved: Vec<PathBuf>,
    pub committed: bool,
}
impl OutputError {
    pub fn exit_code(&self) -> u8 {
        match self.kind {
            ErrorKind::Invalid => 2,
            ErrorKind::Conflict | ErrorKind::Busy | ErrorKind::RecoveryRequired => 3,
            ErrorKind::Io => 4,
        }
    }
    fn new(kind: ErrorKind, code: &str, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: code.into(),
            message: message.into(),
            public_state: PublicState::Unknown {
                reason: "not inspected".into(),
            },
            recovery_directory: None,
            preserved: vec![],
            committed: false,
        }
    }
    fn io(code: &str, error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, code, error.to_string())
    }
    fn conflict(code: &str) -> Self {
        Self::new(ErrorKind::Conflict, code, code)
    }
}
pub type OutputResult<T> = Result<T, OutputError>;

#[derive(Debug)]
pub struct FileBinding {
    file: File,
    parent: Arc<Directory>,
    name: String,
    path: PathBuf,
    identity: FileIdentity,
    content: Vec<u8>,
}
impl FileBinding {
    /// Read one stable, no-follow regular-file descriptor snapshot. The open
    /// descriptor and its original bytes remain owned by this binding.
    pub fn capture(path: &Path) -> OutputResult<Self> {
        let (parent, name) = Directory::for_path(path)?;
        Self::at(parent, &name, 1, false)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.content
    }
    pub fn identity(&self) -> &FileIdentity {
        &self.identity
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn verify(&self) -> OutputResult<()> {
        self.parent.verify()?;
        self.verify_at(&self.parent, &self.name, 1)
    }
    fn at(parent: Arc<Directory>, name: &str, links: u64, owner: bool) -> OutputResult<Self> {
        let file = parent.open_regular(name, false)?;
        Self::from_file(file, parent, name, links, owner)
    }
    fn from_file(
        file: File,
        parent: Arc<Directory>,
        name: &str,
        links: u64,
        owner: bool,
    ) -> OutputResult<Self> {
        let (identity, content) = unix::snapshot(&file, &parent, name, links)?;
        if owner && (identity.uid != unix::uid() || identity.mode > 0o777) {
            return Err(OutputError::conflict("output/foreign-owner"));
        }
        Ok(Self {
            file,
            parent: parent.clone(),
            name: name.into(),
            path: parent.real.join(name),
            identity,
            content,
        })
    }
    fn verify_at(&self, parent: &Directory, name: &str, links: u64) -> OutputResult<()> {
        let (current, _) = unix::snapshot(&self.file, parent, name, links)?;
        if !same_identity(&current, &self.identity) {
            return Err(OutputError::conflict("output/binding-changed"));
        }
        Ok(())
    }
}
fn same_identity(a: &FileIdentity, b: &FileIdentity) -> bool {
    a.device == b.device
        && a.inode == b.inode
        && a.mode == b.mode
        && a.uid == b.uid
        && a.sha256 == b.sha256
        && a.bytes == b.bytes
}

#[derive(Debug)]
pub struct OutputSlot {
    parent: Arc<Directory>,
    requested_path: PathBuf,
    name: String,
    target: Option<FileBinding>,
    lock: Arc<SlotLock>,
}
impl OutputSlot {
    /// Verify the original absent/file snapshot before this slot publishes.
    /// After publication, use verify_guard plus a fresh FileBinding instead.
    pub fn verify_snapshot(&self) -> OutputResult<()> {
        self.verify(&[])
    }
    /// Inspect publication records under this bound parent without recovering
    /// or choosing a transaction. Unknown namespace objects fail closed.
    pub fn pending_publications(&self) -> OutputResult<Vec<PendingPublication>> {
        transaction::pending(self)
    }
    pub fn verify_guard(&self) -> OutputResult<()> {
        self.lock.verify()?;
        self.parent.verify()
    }
    pub fn path(&self) -> PathBuf {
        self.parent.real.join(&self.name)
    }
    pub fn previous(&self) -> Option<&FileBinding> {
        self.target.as_ref()
    }
    /// Capture a sidecar while retaining this output's existing OS lock.
    pub fn capture_related(&self, path: &Path, protected: &[&FileBinding]) -> OutputResult<Self> {
        self.lock.verify()?;
        let (parent, name) = Directory::for_path(path)?;
        if parent.same_directory(&self.lock.parent) && name == self.lock.target_name {
            return Err(OutputError::conflict("output/slot-alias"));
        }
        capture(parent, name, path, protected, self.lock.clone())
    }
    fn verify(&self, protected: &[&FileBinding]) -> OutputResult<()> {
        self.lock.verify()?;
        self.parent.verify()?;
        for source in protected {
            source.verify().map_err(|mut error| {
                error.code = "input/changed".into();
                error
            })?;
        }
        match (&self.target, self.parent.entry(&self.name)?) {
            (None, None) => Ok(()),
            (Some(binding), Some(_)) => binding.verify_at(&self.parent, &self.name, 1),
            _ => Err(OutputError::conflict("output/target-changed")),
        }
    }
}

pub fn capture_slot(path: &Path, protected: &[&FileBinding]) -> OutputResult<OutputSlot> {
    let (parent, name) = Directory::for_path(path)?;
    reject_aliases(&parent, &name, protected)?;
    if parent.entry(&name)?.is_some() {
        FileBinding::at(parent.clone(), &name, 1, true)?;
    }
    let lock = Arc::new(SlotLock::acquire(parent.clone(), &name)?);
    capture(parent, name, path, protected, lock)
}

/// Query a target with an already existing lock, without creating files or
/// capturing a target snapshot that may be in an interrupted hardlink phase.
pub fn pending_existing_publications(target: &Path) -> OutputResult<Vec<PendingPublication>> {
    let (parent, name) = Directory::for_path(target)?;
    let lock = Arc::new(SlotLock::acquire_existing(parent.clone(), &name)?);
    let slot = OutputSlot {
        parent,
        requested_path: unix::absolute(target)?,
        name,
        target: None,
        lock,
    };
    transaction::pending(&slot)
}
fn reject_aliases(parent: &Directory, name: &str, protected: &[&FileBinding]) -> OutputResult<()> {
    for source in protected {
        source.verify().map_err(|mut error| {
            error.code = "input/changed".into();
            error
        })?;
        if parent.same_directory(&source.parent) && name == source.name {
            return Err(OutputError::conflict("output/input-alias"));
        }
        if let Some(entry) = parent.entry(name)? {
            if entry.device.to_string() == source.identity.device
                && entry.inode.to_string() == source.identity.inode
            {
                return Err(OutputError::conflict("output/input-alias"));
            }
        }
    }
    Ok(())
}
fn capture(
    parent: Arc<Directory>,
    name: String,
    path: &Path,
    protected: &[&FileBinding],
    lock: Arc<SlotLock>,
) -> OutputResult<OutputSlot> {
    parent.verify()?;
    reject_aliases(&parent, &name, protected)?;
    let target = if parent.entry(&name)?.is_some() {
        Some(FileBinding::at(parent.clone(), &name, 1, true)?)
    } else {
        None
    };
    let requested_path = unix::absolute(path)?;
    let slot = OutputSlot {
        parent,
        requested_path,
        name,
        target,
        lock,
    };
    slot.verify(protected)?;
    Ok(slot)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub cleanup_complete: bool,
    pub recovery_directory: Option<PathBuf>,
    pub public_state: PublicState,
    pub cleanup_errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPublication {
    pub target: PathBuf,
    pub recovery_directory: PathBuf,
    pub witness: Option<PathBuf>,
    pub phase: String,
}
impl Publication {
    pub fn exit_code(&self) -> u8 {
        if self.cleanup_complete { 0 } else { 3 }
    }
}

/// The caller must finish artifact validation before publishing these exact
/// bytes. This function stages, syncs and re-hashes them before retiring public.
pub fn publish_bound(
    slot: &OutputSlot,
    bytes: &[u8],
    protected: &[&FileBinding],
) -> OutputResult<Publication> {
    transaction::publish(slot, bytes, protected, &mut |_, _| Ok(()))
}

/// Publish with the requested permissions already applied to the private
/// candidate, before any public link is created.
pub fn publish_bound_with_mode(
    slot: &OutputSlot,
    bytes: &[u8],
    mode: u32,
    protected: &[&FileBinding],
) -> OutputResult<Publication> {
    transaction::publish_with_mode(slot, bytes, mode, protected, &mut |_, _| Ok(()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryStatus {
    Recovered,
    Absent,
    Preserved,
    RecoveryRequired,
    Failed,
    Invalid,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResult {
    pub status: RecoveryStatus,
    pub code: String,
    pub message: String,
    pub target: Option<PathBuf>,
    pub public_state: PublicState,
    pub recovery_directory: PathBuf,
    pub preserved: Vec<PathBuf>,
}
impl RecoveryResult {
    pub fn exit_code(&self) -> u8 {
        match self.status {
            RecoveryStatus::Recovered | RecoveryStatus::Absent => 0,
            RecoveryStatus::Preserved | RecoveryStatus::RecoveryRequired => 3,
            RecoveryStatus::Failed => 4,
            RecoveryStatus::Invalid => 2,
        }
    }
}
pub fn recover_publication(directory: &Path) -> RecoveryResult {
    transaction::recover(directory, &mut |_, _| Ok(()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    StageCreated,
    BeforeRecordWrite,
    StageSynced,
    RecoveryDurable,
    BeforeRetire,
    AfterRetire,
    BeforePublishLink,
    AfterPublishLink,
    BeforeStageCleanup,
    BeforeBackupCleanup,
    BeforeRecoveryLink,
    AfterRecoveryLink,
    AfterRecoveryBackupCleanup,
    BeforeRecordCleanup,
    AfterRecordCleanup,
    BeforeDirectoryCleanup,
    AfterDirectoryCleanup,
    BeforeWitnessDelete,
    AfterWitnessQuarantine,
}
#[derive(Clone, Debug)]
pub(crate) struct Observation {
    pub(crate) target: PathBuf,
    pub(crate) directory: PathBuf,
    #[cfg(test)]
    #[allow(dead_code)] // Read by the integration test's private module copy.
    pub(crate) stage: PathBuf,
    pub(crate) backup: PathBuf,
}
type Observer<'a> = &'a mut dyn FnMut(Phase, &Observation) -> std::io::Result<()>;

#[cfg(test)]
#[allow(dead_code, unused_imports)] // Hooks are used by the integration module copy.
pub(crate) mod test_support {
    pub(crate) use super::{Observation, Phase};
    pub(crate) fn publish(
        slot: &super::OutputSlot,
        bytes: &[u8],
        sources: &[&super::FileBinding],
        observer: super::Observer<'_>,
    ) -> super::OutputResult<super::Publication> {
        super::transaction::publish(slot, bytes, sources, observer)
    }
    pub(crate) fn recover(
        path: &std::path::Path,
        observer: super::Observer<'_>,
    ) -> super::RecoveryResult {
        super::transaction::recover(path, observer)
    }
    pub(crate) fn publish_with_mode(
        slot: &super::OutputSlot,
        bytes: &[u8],
        mode: u32,
        sources: &[&super::FileBinding],
        observer: super::Observer<'_>,
    ) -> super::OutputResult<super::Publication> {
        super::transaction::publish_with_mode(slot, bytes, mode, sources, observer)
    }
}
