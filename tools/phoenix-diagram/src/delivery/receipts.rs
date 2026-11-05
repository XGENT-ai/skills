use crate::check::{ArtifactCheckEntry, FileIdentity};
use crate::diagnostic::Diagnostics;
use crate::input::{DiagramType, Quality};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EngineIdentity {
    pub source_input_sha256: String,
    pub receipt_schema: u32,
    pub viewer_revision: String,
}
impl EngineIdentity {
    pub fn current() -> Self {
        Self {
            source_input_sha256: env!("PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256").into(),
            receipt_schema: crate::RECEIPT_SCHEMA,
            viewer_revision: crate::VIEWER_REVISION.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveConfig {
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub brand_resources: std::collections::BTreeMap<String, crate::brands::ResourceIdentity>,
    pub diagram_type: DiagramType,
    pub quality: Quality,
    pub viewer_revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileReference {
    pub path: PathBuf,
    pub identity: FileIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Pass,
    Failed,
    Unavailable,
    NotRun,
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
    pub checks: Vec<ArtifactCheckEntry>,
    pub composition: Value,
    pub evidence: Value,
    /// The statechart model verdict; absent for every other type.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub model: Value,
    #[serde(skip)]
    pub layout: Option<Value>,
    #[serde(skip)]
    pub(crate) exit: u8,
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
    pub candidate: Option<FileReference>,
    pub artifact: Option<FileReference>,
    pub config: Option<EffectiveConfig>,
    pub config_sha256: Option<String>,
    pub engine: EngineIdentity,
    pub quality: Option<Quality>,
    pub checks: Vec<ArtifactCheckEntry>,
    pub composition: Value,
    pub evidence: Value,
    /// The statechart model verdict; absent for every other type.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub model: Value,
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
    pub artifact: Option<FileReference>,
    pub engine: EngineIdentity,
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

/// What a reviewer saw in each listed screenshot, written by the reviewer.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenshotReview {
    pub path: PathBuf,
    pub observations: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewRecord {
    pub reviewer: String,
    pub verdict: VisualStatus,
    pub screenshots: Vec<ScreenshotReview>,
    pub findings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualReview {
    pub status: VisualStatus,
    pub record: Option<ReviewRecord>,
}
impl VisualReview {
    pub fn pending() -> Self {
        Self {
            status: VisualStatus::RequiredPending,
            record: None,
        }
    }
}

/// The single summary JSON finalize writes beside the HTML; review-record
/// adds the visual review to it.
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
    pub automated_status: Status,
    pub screenshots: Vec<PathBuf>,
    pub visual_review: VisualReview,
    pub delivery_status: String,
    pub evidence: Value,
    /// The statechart model verdict; absent for every other type.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub model: Value,
    pub locale: Value,
    #[serde(skip)]
    pub(super) exit: u8,
}
impl FinalizeReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
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
    pub summary: PathBuf,
    pub automated_status: Status,
    pub visual_review: VisualReview,
    pub delivery_status: String,
    #[serde(skip)]
    pub(crate) exit: u8,
}
impl ReviewReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}
