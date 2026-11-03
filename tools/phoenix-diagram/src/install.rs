//! Project-local diagram installation, isolated from the Phoenix UI installer.
//!
//! Provider selection is adapted from tools/phoenix-ui/crates/install/src/cli.rs
//! (Apache-2.0). Phoenix changes: diagram-only file allowlist, no hooks,
//! settings, personal installation, or cache management.

use crate::diagnostic::Diagnostics;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[path = "install/bundle.rs"]
mod bundle;
#[path = "install/plan.rs"]
mod plan;

pub type InstallResult<T> = Result<T, InstallError>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallRequest {
    pub project: PathBuf,
    /// Skill bundle directory containing manifest.json and ENGINE.json.
    pub bundle_root: PathBuf,
    pub engine: PathBuf,
    pub providers: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallStatus {
    Planned,
    Installed,
    Unchanged,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: InstallStatus,
    pub diagnostics: Diagnostics,
    pub engine: InstallEngine,
    pub npm_package_version: String,
    pub project: PathBuf,
    pub providers: Vec<String>,
    pub bundle_manifest_sha256: String,
    pub version_manifest_sha256: String,
    pub writes: Vec<InstallWrite>,
    pub unchanged: Vec<String>,
    pub removals: Vec<String>,
    pub reload_steps: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallErrorKind {
    Invalid,
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

/// Read-only plan of the tool files an install would write.
pub fn plan_install(request: &InstallRequest) -> InstallResult<InstallReceipt> {
    plan::install(request, false)
}

/// Write the verified tool files directly; rerun to repair an interrupted install.
pub fn apply_install(request: &InstallRequest) -> InstallResult<InstallReceipt> {
    plan::install(request, true)
}

fn invalid(code: &str, message: impl Into<String>) -> InstallError {
    InstallError {
        kind: InstallErrorKind::Invalid,
        code: code.into(),
        message: message.into(),
    }
}
fn io(error: std::io::Error) -> InstallError {
    InstallError {
        kind: InstallErrorKind::Io,
        code: "install/write".into(),
        message: error.to_string(),
    }
}
