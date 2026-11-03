//! Authored architecture comparison; browser checks and review update its summary.
pub mod canonical;
pub mod receipts;
pub use receipts::*;
mod pipeline;
pub mod render;
mod review;
mod verification;
use crate::diagnostic::{Diagnostic, Diagnostics};
pub use pipeline::{run, CompareRequest};
pub use review::{browser_check, record_review};
pub use verification::{check, is_comparison};
struct Failure {
    diagnostics: Diagnostics,
    exit: u8,
}
fn fail(code: &str, message: impl Into<String>, exit: u8) -> Failure {
    let mut d = Diagnostic::error(Some("architecture"), code, "", message);
    d.stage = "compare".into();
    Failure {
        diagnostics: vec![d],
        exit,
    }
}
fn json_bytes(value: &impl serde::Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("receipts serialize");
    bytes.push(b'\n');
    bytes
}
fn write(path: &std::path::Path, bytes: &[u8]) -> Result<(), Failure> {
    crate::output::write(path, bytes)
        .map_err(|e| fail("output/write", format!("{}: {e}", path.display()), 4))
}
