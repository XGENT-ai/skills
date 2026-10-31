//! Authored architecture comparison; publication and review are separate gates.
pub mod canonical;
pub mod receipts;
pub use receipts::*;
mod pipeline;
pub mod render;
mod review;
pub mod transaction;
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
fn from_output(error: crate::output::OutputError) -> Failure {
    let exit = error.exit_code();
    let mut failure = fail(&error.code, &error.message, exit);
    failure.diagnostics[0].evidence = serde_json::to_value(error).expect("output diagnostic");
    failure
}

#[cfg(test)]
mod review_tests;
