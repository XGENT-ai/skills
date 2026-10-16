//! One-snapshot delivery and current, independently checked evidence.

mod metadata;
mod prepare;
mod provenance;
mod receipts;
mod review;
mod support;
mod transaction;
pub use receipts::*;

use crate::browser::{BoundArtifact, BrowserOptions};
use crate::input::{CompileOptions, DiagramType};
use crate::render::EmbeddedAssets;
use std::path::Path;

pub struct CompileRequest<'a> {
    pub kind: DiagramType,
    pub candidate: &'a Path,
    pub options: &'a CompileOptions,
    pub engine: &'a EngineIdentity,
    pub assets: &'a EmbeddedAssets<'a>,
}

pub fn validate(request: &CompileRequest<'_>) -> ValidationReceipt {
    prepare::prepare(request).receipt
}

pub fn deliver(request: &CompileRequest<'_>, output: &Path) -> DeliveryReceipt {
    transaction::deliver(request, output)
}

pub fn check(output: &Path, strict: bool, engine: &EngineIdentity) -> CheckReceipt {
    provenance::inspect(output, strict, engine)
}

pub fn bound_artifact(
    output: &Path,
    engine: &EngineIdentity,
) -> Result<BoundArtifact, CheckReceipt> {
    bound_artifact_observed(output, engine, || {})
}

fn bound_artifact_observed(
    output: &Path,
    engine: &EngineIdentity,
    after_failure: impl FnOnce(),
) -> Result<BoundArtifact, CheckReceipt> {
    match provenance::current(output, engine) {
        Ok(current) => Ok(current.bound_artifact()),
        Err(failure) => {
            after_failure();
            Err(provenance::failed(engine, true, failure))
        }
    }
}

pub fn browser_check(
    output: &Path,
    engine: &EngineIdentity,
    options: &BrowserOptions,
    visual: bool,
) -> Result<BrowserCommandReceipt, CheckReceipt> {
    transaction::browser_check(output, engine, options, visual)
}

pub fn finalize(
    request: &CompileRequest<'_>,
    output: &Path,
    browser: &BrowserOptions,
) -> FinalizeReceipt {
    transaction::finalize(request, output, browser)
}

pub fn record_review(output: &Path, record: &Path, engine: &EngineIdentity) -> ReviewReceipt {
    review::record(output, record, engine)
}

/// Stable namespace shared by receipts, browser captures, and review records.
pub fn bounded_sidecar_stem(output: &Path) -> String {
    support::bounded_stem(output)
}

pub fn sidecar_paths(output: &Path, out_dir: Option<&Path>) -> SidecarPaths {
    support::paths(output, out_dir)
}

#[cfg(test)]
mod regression_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod test_support;
