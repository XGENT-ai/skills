//! Explicit foreground preview: recompile on input edits and serve the result.
mod controller;
mod http;

use crate::{
    delivery::CompileRequest,
    diagnostic::{Diagnostic, Diagnostics},
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
pub fn run(request: &PreviewRequest<'_>) -> u8 {
    controller::run(request)
}
