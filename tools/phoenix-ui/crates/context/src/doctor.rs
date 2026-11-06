//! JS: doctor.mjs -> `impeccable doctor`

use crate::context::*;
use crate::jsp;
use crate::staleness::*;
use crate::staleness_deep::*;
use crate::target_args::{parse_target_options, TargetOptions};
use crate::util::{json_pretty, opt_string, Env};
use impeccable_common::Io;
use serde_json::{Map, Value};

struct Flags {
    json: bool,
    help: bool,
}

fn parse_args(argv: &[String]) -> Result<(Flags, TargetOptions), String> {
    let mut flags = Flags {
        json: false,
        help: false,
    };
    let mut passthrough: Vec<String> = Vec::new();
    for a in argv {
        match a.as_str() {
            "--json" => flags.json = true,
            "--help" | "-h" => flags.help = true,
            _ => passthrough.push(a.clone()),
        }
    }
    let t = parse_target_options(&passthrough, true)?;
    Ok((flags, t))
}

fn usage() -> String {
    [
        "Usage: phoenix-ui doctor [--json] [--target <path>]",
        "",
        "Report drift between this project's Phoenix UI artifacts and what the",
        "installed version reads: PRODUCT.md, DESIGN.md and its sidecar,",
        ".phoenix-ui/config.json, surface briefs, and the design hook.",
        "",
        "  --json           Emit findings as JSON.",
        "  --target <path>  Select a workspace in a monorepo.",
    ]
    .join("\n")
}

struct Report {
    ctx: Ctx,
    project_root: String,
    findings: Vec<Finding>,
    workspaces: Vec<WorkspaceRow>,
    rule_registry_available: bool,
}

fn read_project_root_patterns(repo_root: &str) -> Vec<String> {
    if repo_root.is_empty() {
        return vec![];
    }
    read_impeccable_project_roots(repo_root)
}

fn collect(cwd: &str, target: &TargetOptions, env: &Env, provider_id: &str) -> Report {
    let ctx = load_context(cwd, target, env);
    let project_root = if ctx.project_root.is_empty() { cwd.to_string() } else { ctx.project_root.clone() };
    let abs_design_path = ctx.design_path.as_deref().map(|p| jsp::resolve(cwd, &[p]));
    let known = load_known_rule_ids();
    let selection = resolve_target_selection(cwd, target, env);
    let workspace_candidates: Vec<TargetCandidate> = selection.map(|s| s.target_candidates).unwrap_or_default();
    let (ws_findings, workspaces) = check_workspaces(&ctx.repo_root, &workspace_candidates);

    // JS: doctor builds on collectBootFindingGroups, interleaving only its
    // deep checks while preserving the established finding order (upstream
    // 80997663).
    let boot = collect_boot_finding_groups(
        &ctx,
        cwd,
        &BootExtras {
            abs_design_path: abs_design_path.clone(),
            sidecar_path: design_sidecar_path(&project_root),
            project_root_patterns: Some(read_project_root_patterns(&ctx.repo_root)),
            target_candidates: workspace_candidates,
        },
    );
    let mut findings: Vec<Finding> = Vec::new();
    findings.extend(boot.native_platform);
    findings.extend(boot.design_sidecar);
    findings.extend(check_design_drift(abs_design_path.as_deref(), &project_root, 25));
    findings.extend(check_design_coverage(ctx.design.as_deref(), ctx.design_path.as_deref()));
    findings.extend(boot.config);
    findings.extend(boot.build_path);
    findings.extend(check_detector_ignores(&project_root, known.as_deref()));
    findings.extend(boot.surface_briefs);
    findings.extend(check_hook_installation(&project_root, Some(&ctx.repo_root), provider_id));
    findings.extend(boot.project_roots);
    findings.extend(ws_findings);

    Report {
        ctx,
        project_root,
        findings,
        workspaces,
        rule_registry_available: known.is_some(),
    }
}

fn rel(file_path: &str, root: &str) -> String {
    let v = jsp::relative("/", root, file_path);
    if !v.is_empty() && !v.starts_with("..") {
        jsp::to_posix(&v)
    } else {
        file_path.to_string()
    }
}

fn render_text(report: &Report, cwd: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let pr = rel(&report.project_root, cwd);
    lines.push(format!(
        "Phoenix UI doctor: {}",
        if pr.is_empty() { "." } else { &pr }
    ));
    if report.ctx.is_monorepo {
        let rr = rel(&report.ctx.repo_root, cwd);
        lines.push(format!(
            "Monorepo, repo root {}.",
            if rr.is_empty() { "." } else { &rr }
        ));
    }
    lines.push(String::new());
    if report.findings.is_empty() {
        lines.push("No drift found. Every artifact matches what this version reads.".to_string());
    } else {
        for (severity, label) in [("route", "needs a command"), ("mention", "worth saying")] {
            let group: Vec<&Finding> = report
                .findings
                .iter()
                .filter(|f| f.severity == severity)
                .collect();
            if group.is_empty() {
                continue;
            }
            lines.push(format!("{} ({}):", label, group.len()));
            for f in group {
                lines.push(format!("  {}{}", f.id, f.path.as_ref().filter(|p| !p.is_empty()).map(|p| format!("  [{}]", p)).unwrap_or_default()));
                lines.push(format!("    {}", f.summary));
                lines.push(format!("    → {}", f.fix));
            }
            lines.push(String::new());
        }
    }
    if !report.workspaces.is_empty() {
        lines.push("Workspaces:".to_string());
        for w in &report.workspaces {
            lines.push(format!(
                "  {}  product: {}  design: {}{}",
                w.path,
                w.product_status,
                w.design_status,
                w.platform.as_ref().filter(|p| !p.is_empty()).map(|p| format!("  platform: {}", p)).unwrap_or_default()
            ));
        }
        lines.push(String::new());
    }
    if !report.rule_registry_available {
        lines.push("Note: the bundled detector could not be resolved, so ignored rule ids were not validated.".to_string());
        lines.push(String::new());
    }
    lines.join("\n")
}

pub fn run(args: &[String], io: &mut Io) -> i32 {
    let cwd = io.cwd.to_string_lossy().into_owned();
    let env = io.env.clone();
    let provider = crate::provider::detect(&env, &cwd);
    let (flags, target) = match parse_args(args) {
        Ok(v) => v,
        Err(msg) => {
            io.err(&format!("{}\n", msg));
            return 1;
        }
    };
    if flags.help {
        io.out(&format!("{}\n", usage()));
        return 0;
    }
    let report = collect(&cwd, &target, &env, &provider.id);
    if flags.json {
        let mut m = Map::new();
        m.insert(
            "projectRoot".into(),
            Value::String(report.project_root.clone()),
        );
        m.insert(
            "repoRoot".into(),
            Value::String(report.ctx.repo_root.clone()),
        );
        m.insert("isMonorepo".into(), Value::Bool(report.ctx.is_monorepo));
        m.insert("productPath".into(), opt_string(&report.ctx.product_path));
        m.insert("designPath".into(), opt_string(&report.ctx.design_path));
        m.insert("platform".into(), opt_string(&report.ctx.platform));
        m.insert(
            "ruleRegistryAvailable".into(),
            Value::Bool(report.rule_registry_available),
        );
        m.insert(
            "findings".into(),
            Value::Array(report.findings.iter().map(|f| f.to_value()).collect()),
        );
        m.insert(
            "workspaces".into(),
            Value::Array(report.workspaces.iter().map(|w| w.to_value()).collect()),
        );
        io.out(&format!("{}\n", json_pretty(&Value::Object(m))));
        return 0;
    }
    io.out(&format!("{}\n", render_text(&report, &cwd)));
    0
}
