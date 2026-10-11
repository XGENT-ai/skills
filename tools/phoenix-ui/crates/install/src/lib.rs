//! Read-only project migration classification and an installation transaction kernel.
//! The kernel accepts an explicit classified plan; it never infers ownership from
//! filenames or command substrings. `classify` reads fixed migration artifacts.
//!
//! `preflight` performs no writes. `apply` repeats it under an OS lock, records
//! verified backups and a journal before replacing files, and activates the receipt
//! last. `recover` compensates interrupted transactions; `rollback` restores entry
//! points while retaining files explicitly marked as new-tool data. Callers must
//! keep `.phoenix-ui/install` and `.phoenix-ui/migrations` locally ignored.
//! Personal/provider discovery, process checks, downloads and the Node handoff
//! remain caller responsibilities. The kernel writes only project-local paths.

pub mod classify;
pub mod cli;
pub mod data;
mod disk;
pub mod installer;
mod json;
pub mod plan;
mod transaction;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt, path::Path};

pub use transaction::{Ownership, Preflight, Project, Receipt, Recovery, VerifiedInstallation};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidPlan,
    Conflict,
    Busy,
    RecoveryRequired,
    Io,
}

#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(crate) fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    pub(crate) fn io(path: &Path, error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, format!("{}: {error}", path.display()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", content = "sha256", rename_all = "kebab-case")]
pub enum ExpectedFile {
    Absent,
    Hash(String),
}

/// Unix permission bits, or the read-only attribute on other platforms. Unix
/// bits are ignored on Windows; the attribute is ignored on Unix.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Permissions {
    pub unix_mode: u32,
    pub readonly: bool,
}

impl Permissions {
    pub fn file(mode: u32) -> Self {
        Self {
            unix_mode: if cfg!(unix) { mode } else { 0 },
            readonly: false,
        }
    }

    pub(crate) fn native(&self) -> Self {
        Self {
            unix_mode: if cfg!(unix) { self.unix_mode } else { 0 },
            readonly: !cfg!(unix) && self.readonly,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payload {
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub permissions: Permissions,
}

impl Payload {
    pub fn new(bytes: Vec<u8>, permissions: Permissions) -> Self {
        Self {
            sha256: sha256(&bytes),
            bytes,
            permissions,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RollbackPolicy {
    Restore,
    /// New configuration and UI evidence remain in Phoenix's namespace.
    Retain,
}

/// Exact known fields identify a surrounding matcher group, not its mutable
/// children. `None` requires a field to be absent. Required keys can distinguish
/// a nested `hooks` group from neighboring direct command handlers.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub fields: BTreeMap<String, Option<Value>>,
    pub required_keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum JsonStep {
    Key(String),
    Match(Identity),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum JsonSlot {
    Member(String),
    /// A requested insertion position. Existing entries are always found by
    /// exact value, never by their possibly shifted array index.
    Element(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JsonPatch {
    pub parent: Vec<JsonStep>,
    pub slot: JsonSlot,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Change {
    Exclusive {
        path: String,
        expected: ExpectedFile,
        after: Option<Payload>,
        rollback: RollbackPolicy,
    },
    SharedJson {
        path: String,
        expected: ExpectedFile,
        patches: Vec<JsonPatch>,
    },
}

impl Change {
    pub fn path(&self) -> &str {
        match self {
            Self::Exclusive { path, .. } | Self::SharedJson { path, .. } => path,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub version: String,
    pub sha256: String,
    pub providers: Vec<String>,
    pub migration_sources: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub transaction_id: String,
    pub manifest: Manifest,
    pub expected_receipt: ExpectedFile,
    pub changes: Vec<Change>,
}

/// Checkpoints run after the named durable operation, except BeforeReplace,
/// which runs just before the final compare-and-swap read. An observer may
/// abort to simulate a crash; the next invocation must call recover first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checkpoint {
    JournalReady,
    BeforeReplace(String),
    AfterReplace(String),
    BeforeReceipt,
    AfterReceipt,
    BeforeRollback(String),
    AfterRollback(String),
}
