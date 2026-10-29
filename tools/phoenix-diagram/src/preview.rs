//! Explicit foreground preview. Only the controller owns the public output.
mod controller;
mod http;
pub mod worker;

use crate::{
    delivery::CompileRequest,
    diagnostic::{Diagnostic, Diagnostics},
    output::OutputError,
};
use std::path::Path;

pub struct PreviewRequest<'a> {
    pub compile: CompileRequest<'a>,
    pub output: &'a Path,
    pub port: u16,
    pub open: bool,
}
#[derive(Debug)]
pub struct PreviewFailure {
    pub diagnostics: Diagnostics,
    pub exit: u8,
}
pub type PreviewResult<T> = Result<T, PreviewFailure>;
impl PreviewFailure {
    pub(crate) fn new(code: &str, message: impl Into<String>, exit: u8) -> Self {
        let mut diagnostic = Diagnostic::error(None, code, "", message);
        diagnostic.stage = "preview".into();
        Self {
            diagnostics: vec![diagnostic],
            exit,
        }
    }
    pub(crate) fn io(code: &str, error: std::io::Error) -> Self {
        Self::new(code, error.to_string(), 4)
    }
}
impl From<OutputError> for PreviewFailure {
    fn from(error: OutputError) -> Self {
        let mut failure = Self::new(&error.code, &error.message, error.exit_code());
        failure.diagnostics[0].evidence =
            serde_json::to_value(error).expect("output error serializes");
        failure
    }
}
pub fn run(request: &PreviewRequest<'_>) -> u8 {
    controller::run(request)
}
