use crate::{
    browser::{BoundFile, BrowserCoverage, BrowserStatus, Screenshot},
    check::{ArtifactCheckEntry, FileIdentity},
    delivery::{EffectiveConfig, EngineIdentity, FileReference, Status, VisualReview},
    diagnostic::Diagnostics,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub rendered: FileIdentity,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComparisonIr {
    pub identity: FileIdentity,
    pub document: Value,
}
/// The comparison summary: written by compare beside the HTML, then updated
/// by visual-check and review-record.
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
    pub checks: Vec<ArtifactCheckEntry>,
    pub browser_status: Status,
    pub browser: Option<ComparisonBrowserReceipt>,
    pub screenshots: Vec<std::path::PathBuf>,
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
    pub artifact: BoundFile,
    pub browser_version: Option<Value>,
    pub coverage: Vec<ComparisonCoverage>,
    pub checks: Vec<crate::browser::BrowserCheck>,
    pub screenshots: Vec<ComparisonScreenshot>,
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
#[serde(rename_all = "camelCase")]
pub struct ComparisonCheckReceipt {
    pub schema_version: u32,
    pub tool: String,
    pub tool_version: String,
    pub command: String,
    pub kind: String,
    pub status: Status,
    pub diagnostics: Diagnostics,
    pub checks: Vec<ArtifactCheckEntry>,
    #[serde(skip)]
    pub(crate) exit: u8,
}
impl ComparisonCheckReceipt {
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}
