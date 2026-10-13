//! Phoenix local skill installation, update, link, help and consistency checks.
//! Operational entry points accept only an explicitly selected, verified local
//! distribution. They never resolve upstream/latest assets or fetch an engine.
//! Imported archive/signature parsers remain tested with deterministic inputs.

pub mod bundle;
#[cfg(test)]
mod bundle_signature;
pub mod commands;
pub mod engine_binary;
pub mod hook_manifest;
pub mod local_bundle;
pub mod prompt;
pub mod providers;
pub mod util;

use impeccable_common::Io;

/// How a JS code path leaves the verb: `process.exit(code)`, a thrown
/// `PromptAbortError` (cli.js prints `\nAborted.` and exits 130), or any
/// other uncaught throw (cli.js prints the message to stderr and exits 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Flow {
    Exit(i32),
    Abort,
    Throw(String),
}

pub type R<T> = Result<T, Flow>;

/// JS: skills.mjs#run, wrapped in cli.js's `main().catch(...)`.
pub fn run(args: &[String], io: &mut Io) -> i32 {
    match commands::run(args, io) {
        Ok(()) => 0,
        Err(Flow::Exit(code)) => code,
        Err(Flow::Abort) => {
            io.out("\nAborted.\n");
            130
        }
        Err(Flow::Throw(msg)) => {
            io.err(&format!("{msg}\n"));
            1
        }
    }
}
