use crate::browser::{BoundFile, Theme, Viewport};
use crate::check::{ArtifactCheckEntry, FileIdentity};
use crate::diagnostic::Diagnostics;
use crate::input::{Compatibility, DiagramType, InputContract, Quality};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineIdentity {
    pub source_input_sha256: String,
    pub input_contract_schema: u32,
    pub receipt_schema: u32,
    pub viewer_revision: String,
    pub binary: FileIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveConfig {
    pub diagram_type: DiagramType,
    pub input_contract: InputContract,
    pub quality: Quality,
    pub viewer_revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileReference {
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub original_binding: crate::output::FileIdentity,
}

impl FileReference {
    pub fn bound_file(&self) -> BoundFile {
        BoundFile {
            path: self.path.clone(),
            identity: self.identity.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Pass,
    Failed,
    Unavailable,
    NotRun,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommitState {
    NotCommitted,
    Committed,
    CommittedUnreceipted,
    RecoveryRequired,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub candidate: Option<FileReference>,
    pub artifact: Option<FileIdentity>,
    pub config: Option<EffectiveConfig>,
    pub config_sha256: Option<String>,
    pub engine: EngineIdentity,
    pub compatibility: Option<Compatibility>,
    pub checks: Vec<ArtifactCheckEntry>,
    pub composition: Value,
    #[serde(skip)]
    pub layout: Option<Value>,
    #[serde(skip)]
    pub(super) exit: u8,
}
impl ValidationReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub receipt_id: String,
    pub candidate: Option<FileReference>,
    pub artifact: Option<FileReference>,
    pub config: Option<EffectiveConfig>,
    pub config_sha256: Option<String>,
    pub engine: EngineIdentity,
    pub input_contract: Option<InputContract>,
    pub quality: Option<Quality>,
    pub compatibility: Option<Compatibility>,
    pub checks: Vec<ArtifactCheckEntry>,
    pub composition: Value,
    pub commit_state: CommitState,
    pub evidence: Value,
    #[serde(skip)]
    pub(super) exit: u8,
}
impl DeliveryReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub provenance: String,
    pub scope: String,
    pub candidate: Option<FileReference>,
    pub artifact: Option<FileReference>,
    pub config_sha256: Option<String>,
    pub engine: EngineIdentity,
    pub delivery_receipt_id: Option<String>,
    pub delivery_receipt: Option<BoundFile>,
    pub checks: Vec<ArtifactCheckEntry>,
    pub composition: Value,
    #[serde(skip)]
    pub(super) exit: u8,
}
impl CheckReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Stage {
    pub name: String,
    pub status: Status,
    pub receipt: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisualStatus {
    RequiredPending,
    Passed,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualReview {
    pub status: VisualStatus,
    pub record: Option<BoundFile>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub stages: Vec<Stage>,
    pub candidate: Option<FileReference>,
    pub artifact: Option<FileReference>,
    pub config_sha256: Option<String>,
    pub engine: EngineIdentity,
    pub delivery_receipt_id: Option<String>,
    pub browser_receipt_identity: Option<FileIdentity>,
    pub automated_status: Status,
    pub visual_review: VisualReview,
    pub delivery_status: String,
    pub produced_by_this_run: bool,
    pub evidence: Value,
    #[serde(skip)]
    pub(super) exit: u8,
}
impl FinalizeReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedScreenshot {
    pub file: BoundFile,
    pub viewport: Viewport,
    pub theme: Theme,
    pub observed: bool,
    pub observations: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualReviewRecord {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub artifact: BoundFile,
    pub candidate: BoundFile,
    pub config_sha256: String,
    pub engine: EngineIdentity,
    pub delivery_receipt_id: String,
    pub browser_receipt_identity: FileIdentity,
    pub screenshot_identities: Vec<ObservedScreenshot>,
    pub reviewer: String,
    pub scope: Vec<String>,
    pub verdict: VisualStatus,
    pub findings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub delivery_receipt_id: Option<String>,
    pub browser_receipt_identity: Option<FileIdentity>,
    pub automated_status: Status,
    pub visual_review: VisualReview,
    pub delivery_status: String,
    pub evidence: Value,
    #[serde(skip)]
    pub(super) exit: u8,
}
impl ReviewReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCommandReceipt {
    #[serde(flatten)]
    pub receipt: crate::browser::BrowserReceipt,
    pub evidence: Value,
    #[serde(skip)]
    pub(super) exit: u8,
}
impl BrowserCommandReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SidecarPaths {
    pub delivery: PathBuf,
    pub summary: PathBuf,
    pub browser: PathBuf,
    pub visual: PathBuf,
    pub review: PathBuf,
}
