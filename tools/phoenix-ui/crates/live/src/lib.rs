//! Live mode: the helper server and its browser overlay, the agent poll
//! protocol, framework injection, and the wrap/insert/accept verbs that edit
//! source for variant sessions and copy edits.

pub mod accept_css;
pub mod accept_verify;
pub mod browser_assets;
pub mod browser_open;
pub mod config;
pub mod copy_edit_agent;
pub mod dev_url;
pub mod event_validation;
pub mod gitignore;
pub mod inject;
pub mod instructions;
pub mod live_http;
pub mod manifests;
pub mod manual_edits;
pub mod paths;
pub mod pending_edits;
pub mod preflight;
pub mod project_ignores;
pub mod random;
pub mod roots;
pub mod server;
pub mod server_state;
pub mod session;
pub mod source_search;
pub mod svelte_ast;
pub mod svelte_bridge;
pub mod svelte_component;
pub mod util;
pub mod vocabulary;
pub mod wrap_common;

pub mod live_accept;
pub mod live_boot;
pub mod live_commit_manual_edits;
pub mod live_complete;
pub mod live_discard_manual_edits;
pub mod live_generate;
pub mod live_inject;
pub mod live_insert;
pub mod live_poll;
pub mod live_server;
pub mod live_target;
pub mod live_wrap;

use impeccable_common::Io;

/// Verb dispatcher for every `live*` verb; an unknown one exits 70.
pub fn run(verb: &str, args: &[String], io: &mut Io) -> i32 {
    match verb {
        "live" => live_boot::run(args, io),
        "live-complete" => live_complete::run(args, io),
        "live-inject" => live_inject::run(args, io),
        "live-wrap" => live_wrap::run(args, io),
        "live-insert" => live_insert::run(args, io),
        "live-accept" => live_accept::run(args, io),
        "live-server" => live_server::run(args, io),
        "live-generate" => live_generate::run(args, io),
        "live-poll" => live_poll::run(args, io),
        "live-commit-manual-edits" => live_commit_manual_edits::run(args, io),
        "live-discard-manual-edits" => live_discard_manual_edits::run(args, io),
        other => {
            io.err(&format!("phoenix-ui: unknown live verb '{}'\n", other));
            70
        }
    }
}
