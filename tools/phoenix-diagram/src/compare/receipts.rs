use crate::{
    browser::{BoundFile, BrowserCoverage, BrowserStatus, Screenshot, Theme, Viewport},
    check::{ArtifactCheckEntry, FileIdentity},
    delivery::{
        EffectiveConfig, EngineIdentity, FileReference, Stage, Status, VisualReview, VisualStatus,
    },
    diagnostic::Diagnostics,
    input::Compatibility,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompareSide {
    pub candidate: FileReference,
    pub raw: FileIdentity,
    pub semantic_sha256: String,
    pub config: EffectiveConfig,
    pub config_sha256: String,
    pub engine: EngineIdentity,
    pub evidence: Value,
    pub compatibility: Compatibility,
    pub rendered: FileIdentity,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonIr {
    pub identity: FileIdentity,
    pub document: Value,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompareReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub kind: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub comparator_version: u32,
    pub canonical_version: u32,
    pub base: Option<CompareSide>,
    pub head: Option<CompareSide>,
    pub comparison_ir: Option<ComparisonIr>,
    pub changes: Value,
    pub summary: Value,
    pub artifact: Option<BoundFile>,
    pub receipt_id: String,
    pub transaction: Value,
    pub checks: Vec<ArtifactCheckEntry>,
    pub browser_status: Status,
    pub visual_review: VisualReview,
    pub delivery_status: String,
    pub limitations: Vec<String>,
    #[serde(skip)]
    pub(crate) exit: u8,
}
impl CompareReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonSideBinding {
    pub candidate: BoundFile,
    pub semantic_sha256: String,
    pub config_sha256: String,
    pub engine: EngineIdentity,
    pub resources: BTreeMap<String, crate::brands::ResourceIdentity>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonBinding {
    pub receipt: BoundFile,
    pub receipt_id: String,
    pub base: ComparisonSideBinding,
    pub head: ComparisonSideBinding,
    pub comparison_ir: FileIdentity,
    pub artifact: BoundFile,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonCheckReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub kind: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub strict_provenance: bool,
    pub comparison: Option<ComparisonBinding>,
    pub checks: Vec<ArtifactCheckEntry>,
    #[serde(skip)]
    pub(crate) exit: u8,
}
impl ComparisonCheckReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonCoverage {
    pub view_id: String,
    #[serde(flatten)]
    pub coverage: BrowserCoverage,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonScreenshot {
    pub view_id: String,
    #[serde(flatten)]
    pub screenshot: Screenshot,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonBrowserReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub kind: String,
    pub status: BrowserStatus,
    pub diagnostics: Diagnostics,
    pub comparison: ComparisonBinding,
    pub browser_version: Option<Value>,
    pub coverage: Vec<ComparisonCoverage>,
    pub checks: Vec<crate::browser::BrowserCheck>,
    pub screenshots: Vec<ComparisonScreenshot>,
    pub evidence: Value,
}
impl ComparisonBrowserReceipt {
    pub fn exit_code(&self) -> u8 {
        if self.status == BrowserStatus::Pass {
            return 0;
        }
        if self.status == BrowserStatus::Unavailable {
            return 5;
        }
        if self.diagnostics.iter().any(|d| d.code == "browser/options") {
            return 2;
        }
        if self.diagnostics.iter().any(|d| {
            matches!(
                d.code.as_str(),
                "browser/evidence-conflict"
                    | "browser/comparison-receipt-changed"
                    | "browser/artifact-changed"
                    | "browser/candidate-changed"
                    | "browser/evidence-changed"
            )
        }) {
            return 3;
        }
        if self.diagnostics.iter().any(|d| {
            !d.code.strip_prefix("browser/").is_some_and(|code| {
                crate::browser::BROWSER_MEASUREMENT_CHECKS.contains(&code)
                    || matches!(
                        code,
                        "comparison-parent" | "comparison-coverage" | "comparison-delta-paint"
                    )
            })
        }) {
            4
        } else {
            1
        }
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonSummary {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub kind: String,
    pub command: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub comparison: ComparisonBinding,
    pub stages: Vec<Stage>,
    pub automated_status: Status,
    pub visual_review: VisualReview,
    pub delivery_status: String,
    pub evidence: Value,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonObservedScreenshot {
    pub view_id: String,
    pub file: BoundFile,
    pub viewport: Viewport,
    pub theme: Theme,
    pub observed: bool,
    pub observations: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonVisualReviewRecord {
    pub schema_version: u32,
    pub kind: String,
    pub command: String,
    pub comparison: ComparisonBinding,
    pub browser_receipt: BoundFile,
    pub screenshot_identities: Vec<ComparisonObservedScreenshot>,
    pub reviewer: String,
    pub scope: Vec<String>,
    pub verdict: VisualStatus,
    pub findings: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonReviewReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub kind: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub comparison: Option<ComparisonBinding>,
    pub automated_status: Status,
    pub visual_review: VisualReview,
    pub delivery_status: String,
    pub evidence: Value,
    #[serde(skip)]
    pub(crate) exit: u8,
}
impl ComparisonReviewReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparisonPaths {
    pub summary: PathBuf,
    pub browser: PathBuf,
    pub visual: PathBuf,
    pub review: PathBuf,
}
