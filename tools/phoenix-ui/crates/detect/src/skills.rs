//! Compatibility dispatch stub. The public CLI uses impeccable_skills for
//! Phoenix local-bundle installation and checks.

use impeccable_common::Io;

/// JS: skills.mjs#run
pub fn run(args: &[String], io: &mut Io) -> i32 {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "help" | "--help" | "-h" => not_implemented("help", io),
        "install" | "link" | "update" | "check" => not_implemented(sub, io),
        other => {
            io.err(&format!("Unknown skills command: {other}\n"));
            io.err("Run 'phoenix-ui --help' for available commands.\n");
            1
        }
    }
}

fn not_implemented(verb: &str, io: &mut Io) -> i32 {
    io.err(&format!(
        "phoenix-ui {verb}: unavailable through this compatibility dispatcher. Use the Phoenix CLI with a verified local bundle.\n"
    ));
    1
}
