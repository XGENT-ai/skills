//! Compile, write the HTML, check it in a browser, and record the image review.

mod finalize;
pub(crate) mod metadata;
pub(crate) mod prepare;
mod receipts;
pub(crate) mod review;
mod support;
pub use receipts::*;

use crate::browser::{BrowserOptions, BrowserReceipt};
use crate::input::{CompileOptions, DiagramType};
use crate::render::EmbeddedAssets;
use std::path::{Path, PathBuf};

pub struct CompileRequest<'a> {
    pub kind: DiagramType,
    pub candidate: &'a Path,
    pub options: &'a CompileOptions,
    pub assets: &'a EmbeddedAssets<'a>,
    /// The local checkout cited sources are checked against, if given.
    pub repo_root: Option<&'a Path>,
    pub brand_cache: Option<&'a Path>,
}

pub fn validate(request: &CompileRequest<'_>) -> ValidationReceipt {
    prepare::prepare(request).receipt
}

pub fn deliver(request: &CompileRequest<'_>, output: &Path) -> DeliveryReceipt {
    finalize::deliver(request, output)
}

pub fn check(output: &Path) -> CheckReceipt {
    finalize::check(output)
}

pub fn browser_check(output: &Path, options: &BrowserOptions, visual: bool) -> BrowserReceipt {
    finalize::browser_check(output, options, visual)
}

pub fn finalize(
    request: &CompileRequest<'_>,
    output: &Path,
    browser: &BrowserOptions,
) -> FinalizeReceipt {
    finalize::finalize(request, output, browser)
}

pub fn record_review(output: &Path, record: &Path) -> ReviewReceipt {
    review::record(output, record)
}

/// Where finalize writes the summary for `output`.
pub fn finalize_summary_path(output: &Path) -> PathBuf {
    support::summary_path(output)
}
