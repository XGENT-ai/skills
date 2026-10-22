//! `phoenix-ui install --project` and `phoenix-ui migrate`: the transactional
//! project installer. `--json` writes exactly one object to stdout; diagnostics
//! go to stderr. Exit 0 = success or valid state, 2 = argument/path input,
//! 3 = conflict or recovery required, 4 = file, dependency or environment failure.
use crate::{
    installer::{self, Report, Request, Status},
    ErrorKind,
};
use impeccable_common::Io;
use std::path::{Path, PathBuf};

const PROVIDERS: [&str; 19] = [
    ".agent",
    ".agents",
    ".claude",
    ".codex",
    ".cursor",
    ".dsh",
    ".gemini",
    ".github",
    ".grok",
    ".hermes",
    ".kiro",
    ".opencode",
    ".pi",
    ".qoder",
    ".rovodev",
    ".trae",
    ".trae-cn",
    ".veto",
    ".vibe",
];
const DEFAULT_PROVIDERS: [&str; 1] = [".claude"];

pub const INSTALL_USAGE: &str = "Usage: phoenix-ui install [--project [DIR]] --bundle-root DIR [--auxiliary-root DIR] [--providers a,b] [--keep-impeccable] [--dry-run] [--json] [--transaction-id ID]\n";
pub const MIGRATE_USAGE: &str = "Usage: phoenix-ui migrate --from impeccable --project DIR --bundle-root DIR [--auxiliary-root DIR] [--providers a,b] [--dry-run] [--json]\n       phoenix-ui migrate --rollback TRANSACTION --project DIR [--json]\n       phoenix-ui migrate --recover --project DIR [--json]\n";

#[derive(Default)]
struct Args {
    project: Option<String>,
    bundle_root: Option<String>,
    auxiliary_root: Option<String>,
    providers: Option<String>,
    transaction_id: Option<String>,
    from: Option<String>,
    rollback: Option<String>,
    recover: bool,
    dry_run: bool,
    json: bool,
    keep_impeccable: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_string())),
            _ => (arg, None),
        };
        let mut value = || -> Result<String, String> {
            if let Some(value) = inline.clone() {
                return Ok(value);
            }
            i += 1;
            args.get(i)
                .filter(|v| !v.starts_with("--"))
                .cloned()
                .ok_or_else(|| format!("{name} requires a value"))
        };
        match name {
            // The inherited flag is boolean (the current project); a following
            // non-option argument names another project.
            "--project" if inline.is_none() => {
                parsed.project = match args.get(i + 1).filter(|v| !v.starts_with('-')) {
                    Some(dir) => {
                        i += 1;
                        Some(dir.clone())
                    }
                    None => Some(".".into()),
                }
            }
            "--project" => parsed.project = Some(value()?),
            "--scope" if value()? != "project" => {
                return Err("project installs are transactional; use --user or --global for the user level".into())
            }
            "--scope" => {}
            "--force" | "--no-hooks" if inline.is_none() => {
                return Err(format!("{name} applies only to --user/--global installs; a project install keeps conflicts and user changes for you to resolve"))
            }
            "--bundle-root" => parsed.bundle_root = Some(value()?),
            "--auxiliary-root" => parsed.auxiliary_root = Some(value()?),
            "--providers" => parsed.providers = Some(value()?),
            "--transaction-id" => parsed.transaction_id = Some(value()?),
            "--from" => parsed.from = Some(value()?),
            "--rollback" => parsed.rollback = Some(value()?),
            "--recover" if inline.is_none() => parsed.recover = true,
            "--dry-run" if inline.is_none() => parsed.dry_run = true,
            "--json" if inline.is_none() => parsed.json = true,
            "--keep-impeccable" if inline.is_none() => parsed.keep_impeccable = true,
            "--yes" | "-y" if inline.is_none() => {}
            _ => return Err(format!("unknown argument: {arg}")),
        }
        i += 1;
    }
    Ok(parsed)
}

fn resolve(io: &Io, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        io.cwd.join(path)
    }
}

/// Explicit names (`claude`, `codex`) or directories (`.claude`). Codex skills
/// live in `.agents` and its hooks in `.codex`, so either selects both.
fn providers(requested: Option<&str>, project: &Path) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = match requested {
        Some(list) => {
            let mut out = Vec::new();
            for token in list.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                let dir = if token == "codex" {
                    ".agents".to_string()
                } else if token.starts_with('.') {
                    token.to_string()
                } else {
                    format!(".{token}")
                };
                if !PROVIDERS.contains(&dir.as_str()) {
                    return Err(format!(
                        "unknown provider: {token}; choose from {}",
                        PROVIDERS.join(", ")
                    ));
                }
                out.push(dir);
            }
            if out.is_empty() {
                return Err("--providers needs at least one provider".into());
            }
            out
        }
        None => {
            let found: Vec<String> = PROVIDERS
                .iter()
                .filter(|p| project.join(p).is_dir())
                .map(|p| {
                    if *p == ".codex" {
                        ".agents".to_string()
                    } else {
                        p.to_string()
                    }
                })
                .collect();
            if found.is_empty() {
                DEFAULT_PROVIDERS.iter().map(|p| p.to_string()).collect()
            } else {
                found
            }
        }
    };
    if out.iter().any(|p| p == ".agents" || p == ".codex") {
        out.push(".agents".into());
        out.push(".codex".into());
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn transaction_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "phoenix-{}-{:09}-{}",
        now.as_secs(),
        now.subsec_nanos(),
        std::process::id()
    )
}

fn quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./:@".contains(&b))
    {
        value.to_string()
    } else if cfg!(windows) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn emit(io: &mut Io, report: &Report, json: bool) -> i32 {
    let code = report.status.exit_code();
    if json {
        io.out(&format!(
            "{}\n",
            serde_json::to_string_pretty(report).unwrap_or_default()
        ));
        return code;
    }
    let status = serde_json::to_value(report.status)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default();
    io.out(&format!(
        "Phoenix UI {} ({}): {status}\n",
        report.mode, report.project
    ));
    if let Some(recovered) = &report.recovered {
        io.out(&format!("  recovered  {recovered}\n"));
    }
    // Group per skill/agent/command entry so the summary stays readable.
    for (label, paths) in [
        ("write", &report.writes),
        ("merge", &report.shared_json),
        ("remove", &report.removals),
    ] {
        let mut groups: std::collections::BTreeMap<String, usize> = Default::default();
        for path in paths {
            let parts: Vec<&str> = path.split('/').collect();
            let key = if parts.len() > 3 && matches!(parts[1], "skills" | "agents" | "commands") {
                parts[..3].join("/") + "/"
            } else {
                path.clone()
            };
            *groups.entry(key).or_default() += 1;
        }
        for (key, count) in groups {
            let files = if count > 1 {
                format!(" ({count} files)")
            } else {
                String::new()
            };
            io.out(&format!("  {label:<7}{key}{files}\n"));
        }
    }
    for path in &report.transition_entries {
        io.out(&format!(
            "  hint   {path} (old command points to phoenix-ui)\n"
        ));
    }
    for conflict in &report.conflicts {
        io.out(&format!(
            "  conflict  {}: {}\n",
            conflict.path, conflict.message
        ));
    }
    for finding in &report.findings {
        io.out(&format!(
            "  {}  {}: {}\n",
            if finding.blocking_integration {
                "pending"
            } else {
                "note"
            },
            finding.path,
            finding.message
        ));
    }
    for item in &report.unconsumed {
        io.out(&format!(
            "  unconsumed  {} {}: {}\n",
            item.path, item.field, item.reason
        ));
    }
    if let (Some(id), true) = (
        &report.transaction_id,
        matches!(
            report.status,
            Status::Installed | Status::IntegrationPending
        ),
    ) {
        if let Some(backups) = &report.backups {
            io.out(&format!("  backups  {backups}\n"));
        }
        let version = report
            .package_version
            .as_deref()
            .unwrap_or("<package-version>");
        io.out(&format!(
            "  rollback  npm exec --package=@xgent-ai/skills@{version} -- phoenix-ui migrate --rollback {id} --project {}\n",
            quote(&report.project)
        ));
    }
    code
}

fn fail(io: &mut Io, kind: ErrorKind, message: &str, usage: &str) -> i32 {
    io.err(&format!("phoenix-ui: {message}\n"));
    match kind {
        ErrorKind::InvalidPlan => {
            io.err(usage);
            2
        }
        ErrorKind::Conflict | ErrorKind::Busy | ErrorKind::RecoveryRequired => 3,
        ErrorKind::Io => 4,
    }
}

/// Project-scope `install` (the default); `--user`/`--global` stay with the inherited installer.
pub fn run_install(args: &[String], io: &mut Io) -> i32 {
    run(args, io, false)
}

pub fn run_migrate(args: &[String], io: &mut Io) -> i32 {
    run(args, io, true)
}

fn run(args: &[String], io: &mut Io, migrate: bool) -> i32 {
    let usage = if migrate {
        MIGRATE_USAGE
    } else {
        INSTALL_USAGE
    };
    if args.iter().any(|a| a == "--help" || a == "-h") {
        io.out(usage);
        return 0;
    }
    let parsed = match parse(args) {
        Ok(parsed) => parsed,
        Err(message) => return fail(io, ErrorKind::InvalidPlan, &message, usage),
    };
    if migrate && parsed.keep_impeccable {
        return fail(
            io,
            ErrorKind::InvalidPlan,
            "--keep-impeccable belongs to install",
            usage,
        );
    }
    // `install` defaults to the current project; `migrate` names it explicitly.
    let project = parsed.project.as_deref().or((!migrate).then_some("."));
    let Some(project) = project.map(|p| resolve(io, p)) else {
        return fail(
            io,
            ErrorKind::InvalidPlan,
            "--project DIR is required",
            usage,
        );
    };
    if !project.is_dir() {
        return fail(
            io,
            ErrorKind::InvalidPlan,
            &format!("project directory does not exist: {}", project.display()),
            usage,
        );
    }
    let modes = [
        parsed.rollback.is_some(),
        parsed.recover,
        parsed.from.is_some(),
    ];
    let result = if migrate && modes.iter().filter(|m| **m).count() != 1 {
        return fail(
            io,
            ErrorKind::InvalidPlan,
            "choose exactly one of --from impeccable, --rollback TRANSACTION or --recover",
            usage,
        );
    } else if !migrate && (parsed.rollback.is_some() || parsed.recover || parsed.from.is_some()) {
        return fail(
            io,
            ErrorKind::InvalidPlan,
            "--rollback, --recover and --from belong to `phoenix-ui migrate`",
            usage,
        );
    } else if let Some(id) = &parsed.rollback {
        installer::rollback(&project, id)
    } else if parsed.recover {
        installer::recover(&project)
    } else {
        if migrate && parsed.from.as_deref() != Some("impeccable") {
            return fail(
                io,
                ErrorKind::InvalidPlan,
                "only --from impeccable is supported",
                usage,
            );
        }
        let bundle_root = parsed
            .bundle_root
            .as_deref()
            .or(io.env("PHOENIX_UI_BUNDLE_PATH").filter(|v| !v.is_empty()));
        let Some(bundle_root) = bundle_root.map(|p| resolve(io, p)) else {
            return fail(
                io,
                ErrorKind::InvalidPlan,
                "--bundle-root DIR is required; Phoenix does not download bundles",
                usage,
            );
        };
        let providers = match providers(parsed.providers.as_deref(), &project) {
            Ok(providers) => providers,
            Err(message) => return fail(io, ErrorKind::InvalidPlan, &message, usage),
        };
        let auxiliary_root = parsed.auxiliary_root.as_deref().map(|p| resolve(io, p));
        let request = Request {
            project: &project,
            bundle_root: &bundle_root,
            auxiliary_root: auxiliary_root.as_deref(),
            providers,
            home: io.home(),
            scan_sources: migrate,
            keep_impeccable: parsed.keep_impeccable,
            dry_run: parsed.dry_run,
            transaction_id: parsed.transaction_id.clone().unwrap_or_else(transaction_id),
        };
        installer::install(&request)
    };
    match result {
        Ok(report) => emit(io, &report, parsed.json),
        Err(error) => fail(io, error.kind, &error.message, usage),
    }
}
