//! `phoenix-ui install` for a project. `--json` writes exactly one object to
//! stdout; diagnostics go to stderr. Exit 0 = success, 2 = argument or path
//! input, 4 = bundle, project file or environment failure.
use crate::{install, Error, Report};
use impeccable_common::Io;
use std::{collections::BTreeMap, path::Path};

pub const INSTALL_USAGE: &str = "Usage: phoenix-ui install [--project [DIR]] --bundle-root DIR [--providers a,b] [--dry-run] [--json]\n";

#[derive(Default)]
struct Args {
    project: Option<String>,
    bundle_root: Option<String>,
    providers: Option<String>,
    dry_run: bool,
    json: bool,
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
            // A bare flag means the current project; a following non-option
            // argument names another project.
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
            "--bundle-root" => parsed.bundle_root = Some(value()?),
            "--providers" => parsed.providers = Some(value()?),
            "--dry-run" if inline.is_none() => parsed.dry_run = true,
            "--json" if inline.is_none() => parsed.json = true,
            _ => return Err(format!("unknown argument: {arg}")),
        }
        i += 1;
    }
    Ok(parsed)
}

fn emit(io: &mut Io, report: &Report) {
    let status = serde_json::to_value(report.status).unwrap_or_default();
    io.out(&format!(
        "Phoenix UI {} ({}): {}\n",
        report.mode,
        report.project,
        status.as_str().unwrap_or_default()
    ));
    // Group per skill/agent/command entry so the summary stays readable.
    for (label, paths) in [
        ("write", &report.writes),
        ("merge", &report.shared_json),
        ("remove", &report.removals),
    ] {
        let mut groups: BTreeMap<String, usize> = BTreeMap::new();
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
}

/// Project-scope `install`/`update`; `--user`/`--global` stay with the
/// inherited installer in `impeccable-skills`.
pub fn run_install(args: &[String], io: &mut Io) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        io.out(INSTALL_USAGE);
        return 0;
    }
    let result = parse(args).map_err(Error::Usage).and_then(|parsed| {
        let resolve = |path: &str| io.cwd.join(Path::new(path));
        let project = resolve(parsed.project.as_deref().unwrap_or("."));
        let project = project
            .canonicalize()
            .ok()
            .filter(|p| p.is_dir())
            .ok_or_else(|| {
                Error::Usage(format!(
                    "project directory does not exist: {}",
                    project.display()
                ))
            })?;
        let bundle_root = parsed
            .bundle_root
            .as_deref()
            .or(io.env("PHOENIX_UI_BUNDLE_PATH").filter(|v| !v.is_empty()))
            .map(resolve)
            .ok_or_else(|| {
                Error::Usage(
                    "--bundle-root DIR is required; Phoenix does not download bundles".into(),
                )
            })?;
        install(
            &project,
            &bundle_root,
            parsed.providers.as_deref(),
            parsed.dry_run,
        )
        .map(|report| (report, parsed.json))
    });
    match result {
        Ok((report, true)) => {
            io.out(&format!(
                "{}\n",
                serde_json::to_string_pretty(&report).unwrap_or_default()
            ));
            0
        }
        Ok((report, false)) => {
            emit(io, &report);
            0
        }
        Err(Error::Usage(message)) => {
            io.err(&format!("phoenix-ui: {message}\n{INSTALL_USAGE}"));
            2
        }
        Err(Error::Failed(message)) => {
            io.err(&format!("phoenix-ui: {message}\n"));
            4
        }
    }
}
