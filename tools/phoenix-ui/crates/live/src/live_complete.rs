//! `phoenix-ui live-complete`: tell the live helper a session is finished,
//! gated on the accepted source being clean.

use crate::accept_verify::verify_accepted_file;
use crate::paths::read_live_server_info;
use crate::roots::enter_live_root;
use crate::server::{fetch_status, post_poll};
use crate::util::{json_pretty, jsp, println};
use impeccable_common::Io;
use serde_json::{json, Value};

const USAGE: &str = "Usage: phoenix-ui live-complete --id SESSION_ID [--discarded|--error MESSAGE] [--force]\n\nMark a session finished in the running live server. Use after accept/discard cleanup is verified.\nCompletion is refused while the session's source file still carries live-mode leftovers\n(markers, data-p-* attributes, unbaked --p-* vars); fix the file or pass --force.";

struct Args {
    id: Option<String>,
    status: &'static str,
    message: Option<String>,
    force: bool,
    help: bool,
}

fn parse_args(argv: &[String]) -> Args {
    let mut out = Args {
        id: None,
        status: "complete",
        message: None,
        force: false,
        help: false,
    };
    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        if a == "--id" {
            i += 1;
            out.id = argv.get(i).cloned();
        } else if let Some(v) = a.strip_prefix("--id=") {
            out.id = Some(v.to_string());
        } else if a == "--discarded" || a == "--discard" {
            out.status = "discarded";
        } else if a == "--error" {
            out.status = "agent_error";
            i += 1;
            out.message = Some(
                argv.get(i)
                    .cloned()
                    .filter(|m| !m.is_empty())
                    .unwrap_or_else(|| "unknown error".to_string()),
            );
        } else if let Some(v) = a.strip_prefix("--error=") {
            out.status = "agent_error";
            out.message = Some(v.to_string());
        } else if a == "--force" {
            out.force = true;
        } else if a == "--help" || a == "-h" {
            out.help = true;
        }
        i += 1;
    }
    out
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    let mut argv: Vec<String> = args.to_vec();
    if let Err(code) = enter_live_root(&mut argv, io) {
        return code;
    }
    complete_cli(&argv, io)
}

fn complete_cli(argv: &[String], io: &mut Io) -> i32 {
    let args = parse_args(argv);
    let id = args.id.clone().filter(|s| !s.is_empty());
    if args.help || id.is_none() {
        println(io, USAGE);
        return if args.help { 0 } else { 1 };
    }
    let id = id.unwrap();
    let cwd = io.cwd.to_string_lossy().into_owned();
    let env = io.env.clone();
    let server = read_live_server_info(&cwd, &env)
        .map(|(info, _)| info)
        .and_then(|info| Some((info.port?, info.token.unwrap_or_default())));
    let Some((port, token)) = server else {
        crate::util::eprintln(
            io,
            "Error: no running live server; the session lived in it.",
        );
        return 1;
    };

    if args.status == "complete" && !args.force {
        let source_file = fetch_status(port, &token)
            .and_then(|status| {
                status
                    .get("activeSessions")?
                    .as_array()?
                    .iter()
                    .find(|s| s.get("id").and_then(Value::as_str) == Some(id.as_str()))?
                    .get("sourceFile")?
                    .as_str()
                    .map(String::from)
            })
            .filter(|s| !s.is_empty());
        if let Some(sf) = &source_file {
            let abs = jsp::resolve(&cwd, &[sf]);
            let rel = jsp::relative("/", &cwd, &abs);
            let inside = !rel.is_empty() && !rel.starts_with("..") && !jsp::is_absolute(&rel);
            if inside
                && !rel.starts_with(&format!("node_modules{}", jsp::SEP))
                && !rel.starts_with("node_modules/")
            {
                let (clean, findings, _missing) = verify_accepted_file(&abs);
                if !clean {
                    println(
                        io,
                        &json_pretty(&json!({
                            "ok": false,
                            "error": "source_dirty",
                            "id": id,
                            "file": sf,
                            "findings": findings,
                            "hint": "The accepted source still carries live-mode leftovers. Finish the carbonize cleanup (bake params, remove markers and data-p-* attributes), then run live-complete again. Use --force only if a finding is a false positive.",
                        })),
                    );
                    return 1;
                }
            }
        }
    }

    let ty = match args.status {
        "discarded" => "discarded",
        "agent_error" => "error",
        _ => "complete",
    };
    let mut body = json!({ "token": token, "id": id, "type": ty });
    if let Some(m) = &args.message {
        body["message"] = json!(m);
    }
    let acknowledged = post_poll(port, &body)
        .and_then(|r| r.get("ok").cloned())
        .is_some_and(|ok| ok == Value::Bool(true));
    if !acknowledged {
        crate::util::eprintln(
            io,
            &format!(
                "Error: the live server did not accept the completion of {}",
                id
            ),
        );
        return 1;
    }
    let phase = match args.status {
        "complete" => "completed",
        other => other,
    };
    println(
        io,
        &json_pretty(&json!({ "ok": true, "id": id, "phase": phase })),
    );
    0
}
