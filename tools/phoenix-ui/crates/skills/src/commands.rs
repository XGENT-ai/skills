//! The verbs: `help`, `check`, `install`, `link`, `update`, and the
//! `skills` router. JS: skills.mjs `showHelp` / `check` / `install` / `link`
//! / `update` / `run` plus the install-plan prompts
//! (`chooseInstallPlan` and friends) and `decideHookInstall`.

use impeccable_common::Io;
use serde_json::Value;

use crate::bundle;
use crate::engine_binary::install_engine_binaries;
use crate::hook_manifest::{
    self, copy_provider_hooks, hook_installed_for_provider, HOOK_EXPLAINER,
};
use crate::local_bundle::LocalBundle;
use crate::prompt::{CheckboxOption, Prompt, RadioOption};
use crate::providers::*;
use crate::util::{self, pad_end, utf16_len, utf16_prefix};
use crate::{Flow, R};

/// JS: skills.mjs#run(args)
/// JS: skills.mjs#SUBCOMMAND_HELP (#708). Static help for a sub-command, so
/// `install --help` never enters an operational path.
const SUBCOMMAND_HELP: &[(&str, &str)] = &[
    (
        "install",
        "Usage: phoenix-ui install [options]\n\nInstall compiled Phoenix UI skills into project or user-level harness folders.\n\nOptions:\n  -y, --yes              Accept detected defaults without prompting\n  --providers=<names>    Comma-separated harnesses to install\n  --scope=<scope>        Install scope: project or global\n  --project              Install into the current project\n  --user, --global       Install at the user level\n  --no-hooks             Install skills without provider hook manifests\n  --force                Replace an existing installation\n  --bundle-root=<path>   Explicit local bundle; also PHOENIX_UI_BUNDLE_PATH\n  --json                 Report unavailable/invalid input as JSON\n  -h, --help             Show this help message",
    ),
    (
        "link",
        "Usage: phoenix-ui link [options]\n\nLink Phoenix UI skills from a verified local distribution.\n\nOptions:\n  --source=<path>        Alias for --bundle-root\n  --providers=<names>    Comma-separated harnesses to link\n  -y, --yes              Accept detected defaults without prompting\n  --force                Replace existing skill folders with links\n  --bundle-root=<path>   Explicit local bundle; also PHOENIX_UI_BUNDLE_PATH\n  --json                 Report unavailable/invalid input as JSON\n  -h, --help             Show this help message",
    ),
    (
        "update",
        "Usage: phoenix-ui update [options]\n\nUpdate an existing Phoenix UI skill installation.\n\nOptions:\n  -y, --yes              Accept detected defaults without prompting\n  --scope=<scope>        Update scope: project or global\n  --project              Update the current project installation\n  --user, --global       Update the user-level installation\n  --no-hooks             Update skills without changing hook manifests\n  --force                Replace installed skill files\n  --bundle-root=<path>   Explicit local bundle; also PHOENIX_UI_BUNDLE_PATH\n  --json                 Report unavailable/invalid input as JSON\n  -h, --help             Show this help message",
    ),
    (
        "check",
        "Usage: phoenix-ui check [options]\n\nCheck whether installed Phoenix UI skills are up to date.\n\nOptions:\n  --bundle-root=<path>   Explicit local bundle; also PHOENIX_UI_BUNDLE_PATH\n  --json                 Report unavailable/invalid input as JSON\n  -h, --help             Show this help message",
    ),
];

fn subcommand_help(sub: &str) -> Option<&'static str> {
    SUBCOMMAND_HELP
        .iter()
        .find(|(k, _)| *k == sub)
        .map(|(_, v)| *v)
}

pub fn run(args: &[String], io: &mut Io) -> R<()> {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest: Vec<String> = args.iter().skip(1).cloned().collect();
    if let Some(help) = subcommand_help(sub) {
        if rest.iter().any(|a| a == "--help" || a == "-h") {
            out(io, help);
            return Ok(());
        }
    }
    match sub {
        "" | "help" | "--help" | "-h" => return show_help(&rest, io),
        "install" | "link" | "update" | "check" => {}
        other => {
            err(io, &format!("Unknown skills command: {other}"));
            return Err(Flow::Exit(1));
        }
    }
    let (mut sys, _) = ctx(io);
    if let Some(value) = get_flag_value(&rest, "--bundle-root").or_else(|| {
        if sub == "link" {
            get_flag_value(&rest, "--source")
        } else {
            None
        }
    }) {
        sys.env
            .insert("PHOENIX_UI_BUNDLE_PATH".into(), value.into());
    }
    if sub == "check" && crate::local_bundle::source(&sys).is_none() {
        return check_local_contract(&sys, &rest, io);
    }
    let bundle = match LocalBundle::load(&sys) {
        Ok(bundle) => bundle,
        Err(error) => {
            local_error(
                &rest,
                io,
                if crate::local_bundle::source(&sys).is_some() {
                    "invalid"
                } else {
                    "unavailable"
                },
                &error,
            );
            return Err(Flow::Exit(4));
        }
    };
    match sub {
        "install" => install(&rest, io, &bundle),
        "link" => link(&rest, io, &bundle),
        "update" => update(&rest, io, &bundle),
        "check" => check(&rest, io, &bundle),
        _ => unreachable!(),
    }
}

fn local_error(flags: &[String], io: &mut Io, status: &str, message: &str) {
    if has_flag(flags, "--json") {
        out(
            io,
            &serde_json::json!({"status": status, "error": message}).to_string(),
        );
    } else {
        err(io, message);
    }
}

fn ctx(io: &Io) -> (Sys, Prompt) {
    let sys = Sys::new(io.env.clone(), io.cwd.to_string_lossy().into_owned());
    let prompt = Prompt::new(io);
    (sys, prompt)
}

fn has_flag(flags: &[String], name: &str) -> bool {
    flags.iter().any(|f| f == name)
}

fn out(io: &mut Io, line: &str) {
    io.out(&format!("{line}\n"));
}

fn err(io: &mut Io, line: &str) {
    io.err(&format!("{line}\n"));
}

// ─── help ────────────────────────────────────────────────────────────────────

/// JS: showHelp()
fn show_help(flags: &[String], io: &mut Io) -> R<()> {
    let metadata: Value = serde_json::from_str(include_str!(
        "../../../../../skills/phoenix-ui/src/scripts/command-metadata.json"
    ))
    .map_err(|e| Flow::Throw(e.to_string()))?;
    let commands = metadata
        .as_object()
        .ok_or_else(|| Flow::Throw("Invalid compiled command metadata".into()))?;
    if has_flag(flags, "--json") {
        out(
            io,
            &serde_json::json!({"status":"available", "commands": commands}).to_string(),
        );
        return Ok(());
    }
    out(io, "\n  Phoenix UI Skills & Commands\n");
    out(io, "  Install: phoenix-ui install --bundle-root DIR");
    out(io, "  Link:    phoenix-ui link --bundle-root DIR");
    out(io, "  Update:  phoenix-ui update --bundle-root DIR\n");
    let mut sorted: Vec<(&String, &Value)> = commands.iter().collect();
    sorted.sort_by(|a, b| locale_compare(a.0, b.0));
    for (id, entry) in sorted {
        let description = entry
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .replace("Impeccable", "Phoenix UI")
            .replace("impeccable", "phoenix-ui");
        let description = if utf16_len(&description) > 72 {
            format!("{}...", utf16_prefix(&description, 69))
        } else {
            description
        };
        out(
            io,
            &format!(
                "  {} {description}",
                pad_end(&format!("/phoenix-ui {id}"), 26)
            ),
        );
    }
    out(
        io,
        &format!(
            "\n  {} commands available. Run /phoenix-ui <command> in your AI harness.\n",
            commands.len()
        ),
    );
    Ok(())
}

/// `a.localeCompare(b)` for the ASCII command ids: case-insensitive first,
/// then code point order as the tiebreak.
fn locale_compare(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

// ─── check ───────────────────────────────────────────────────────────────────

fn check_target(sys: &Sys) -> (String, Option<Scope>, Vec<&'static str>) {
    let root = sys.find_project_root();
    let scope = if sys.is_home_dir(&root) {
        Some(
            if sys.is_already_installed(&root, Some(Scope::User)).is_some() {
                Scope::User
            } else {
                Scope::Project
            },
        )
    } else {
        None
    };
    let providers = sys.find_impeccable_providers(&root, scope);
    (root, scope, providers)
}

fn check_local_contract(sys: &Sys, flags: &[String], io: &mut Io) -> R<()> {
    let (root, scope, providers) = check_target(sys);
    if providers.is_empty() {
        local_error(flags, io, "unavailable", "Local Phoenix skill contract unavailable. Pass --bundle-root DIR; Phoenix does not check upstream/latest or download engines.");
        return Err(Flow::Exit(4));
    }
    let result: Result<(), String> = (|| {
        for (_, skills) in sys.deduplicate_providers(&root, &providers, scope) {
            let skill = util::jsp::join(&[&skills, "phoenix-ui"]);
            let contract = util::jsp::join(&[&skill, "scripts", "ENGINE.json"]);
            if util::is_symlink(&contract) {
                return Err("Local runtime contract is a symlink".into());
            }
            let version: Value = serde_json::from_slice(&util::read_bytes(&contract)?)
                .map_err(|e| format!("Invalid local ENGINE.json: {e}"))?;
            if version.get("toolVersion").and_then(Value::as_str) != Some(env!("CARGO_PKG_VERSION"))
                || version.get("bundleSchema").and_then(Value::as_u64) != Some(1)
                || version.get("reviewSchema").and_then(Value::as_u64) != Some(1)
            {
                return Err("Local ENGINE.json does not match this Phoenix runtime".into());
            }
            let md = util::read_text(&util::jsp::join(&[&skill, "SKILL.md"]))?;
            if !md.lines().any(|line| line == "name: phoenix-ui")
                || !util::is_file(&util::jsp::join(&[&skill, "scripts", "phoenix-ui"]))
                || !util::is_file(&util::jsp::join(&[
                    &skill,
                    "scripts",
                    "phoenix-launcher.cjs",
                ]))
            {
                return Err("Local Phoenix skill/launcher contract is incomplete".into());
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        local_error(flags, io, "invalid", &error);
        return Err(Flow::Exit(4));
    }
    if has_flag(flags, "--json") {
        out(io, &serde_json::json!({"status":"local", "providers":providers, "bundleVerified":false, "engineVerified":false}).to_string());
    } else {
        out(io, "Local Phoenix skill/launcher contract is present. Pass --bundle-root DIR to compare all files; engine verification belongs to the Node launcher.");
    }
    Ok(())
}

fn check(flags: &[String], io: &mut Io, bundle: &LocalBundle) -> R<()> {
    let (sys, _) = ctx(io);
    let (root, scope, providers) = check_target(&sys);
    if providers.is_empty() {
        if has_flag(flags, "--json") {
            out(
                io,
                &serde_json::json!({"status":"not-installed"}).to_string(),
            );
        } else {
            out(io, "Phoenix UI is not installed in this project.");
        }
        return Ok(());
    }
    let staging = bundle.stage(&sys).map_err(Flow::Throw)?;
    let result = bundle::is_up_to_date(&sys, &root, &providers, &staging, scope, scope);
    util::rm_rf(&staging);
    let current = result.map_err(Flow::Throw)?;
    if has_flag(flags, "--json") {
        out(io, &serde_json::json!({"status": if current {"current"} else {"drift"}, "bundleSha256":bundle.digest, "engineVerified":false}).to_string());
    } else if current {
        out(
            io,
            &format!(
                "Skills are up to date{} against the explicit local bundle.",
                version_suffix(&sys.get_skills_version(&root, scope))
            ),
        );
    } else {
        out(io, "Updates available in the explicit local bundle.");
        out(io, "Run `phoenix-ui update --bundle-root DIR` to apply it.");
    }
    Ok(())
}

fn version_suffix(v: &Option<String>) -> String {
    match v {
        Some(v) => format!(" (v{v})"),
        None => String::new(),
    }
}

fn to_version_suffix(v: &Option<String>) -> String {
    match v {
        Some(v) => format!(" to v{v}"),
        None => String::new(),
    }
}

// ─── install plan (prompts) ──────────────────────────────────────────────────

/// JS: printInstallIntro()
fn print_install_intro(prompt: &Prompt, io: &mut Io) {
    if !prompt.interactive() {
        return;
    }
    out(
        io,
        &format!(
            "{} {}",
            prompt.accent(&prompt.bold("Phoenix UI")),
            prompt.dim("install")
        ),
    );
    out(io, "");
}

/// JS: formatInstallDetectionLines(projectRoot, detections, home, {styled})
pub fn format_install_detection_lines(
    sys: &Sys,
    prompt: &Prompt,
    project_root: &str,
    detections: &[Detection],
    styled: bool,
) -> Vec<String> {
    if detections.is_empty() {
        let message = format!(
            "No harnesses detected under {} or {}.",
            sys.format_path_for_display(project_root),
            sys.format_path_for_display(&sys.home)
        );
        return if styled {
            vec![
                format!(
                    "{} {}",
                    prompt.accent("◇"),
                    prompt.bold("Detected harnesses")
                ),
                format!("  {}", prompt.dim(&message)),
            ]
        } else {
            vec![message]
        };
    }
    let names: Vec<String> = detections
        .iter()
        .map(|d| provider_display_name(d.provider))
        .collect();
    let paths: Vec<String> = detections
        .iter()
        .map(|d| sys.format_path_for_display(&d.found_path))
        .collect();
    let name_width = names.iter().map(|n| utf16_len(n)).max().unwrap_or(0);
    let heading = if styled {
        format!(
            "{} {}",
            prompt.accent("◇"),
            prompt.bold("Detected harnesses")
        )
    } else {
        "Detected harnesses:".to_string()
    };
    let mut lines = vec![heading];
    for (index, _) in detections.iter().enumerate() {
        let raw_name = pad_end(&names[index], name_width);
        let raw_found = &paths[index];
        let name = if styled {
            prompt.bold(&raw_name)
        } else {
            raw_name.clone()
        };
        let found = if styled {
            prompt.dim(raw_found)
        } else {
            raw_found.clone()
        };
        lines.push(format!("  {name}  {found}"));
    }
    lines
}

/// JS: printInstallDetections(projectRoot, detections)
fn print_install_detections(
    sys: &Sys,
    prompt: &Prompt,
    io: &mut Io,
    project_root: &str,
    detections: &[Detection],
) {
    for line in
        format_install_detection_lines(sys, prompt, project_root, detections, prompt.interactive())
    {
        out(io, &line);
    }
    out(io, "");
}

/// JS: providerPromptOptions()
fn provider_prompt_options() -> Vec<(&'static str, CheckboxOption)> {
    PROVIDER_INPUT_ORDER
        .iter()
        .map(|input| {
            let provider = normalize_provider_name(input).unwrap_or(input);
            let label = provider_display_name(provider);
            let hint = format!("({provider}/skills)");
            let search_text = format!("{label} {input} {provider} {hint}");
            (
                provider,
                CheckboxOption {
                    label,
                    hint: Some(hint),
                    search_text,
                },
            )
        })
        .collect()
}

/// JS: promptForProviders(defaultProviders)
fn prompt_for_providers(
    prompt: &mut Prompt,
    io: &mut Io,
    default_providers: &[&'static str],
) -> R<Vec<&'static str>> {
    if prompt.interactive() {
        let options = provider_prompt_options();
        let selected: Vec<usize> = options
            .iter()
            .enumerate()
            .filter(|(_, (p, _))| default_providers.contains(p))
            .map(|(i, _)| i)
            .collect();
        let boxes: Vec<CheckboxOption> = options
            .iter()
            .map(|(_, o)| CheckboxOption {
                label: o.label.clone(),
                hint: o.hint.clone(),
                search_text: o.search_text.clone(),
            })
            .collect();
        let chosen = prompt.checkbox(io, "Select harnesses", &boxes, &selected)?;
        return Ok(chosen.into_iter().map(|i| options[i].0).collect());
    }
    let choices = PROVIDER_INPUT_ORDER.join(", ");
    let suffix = if !default_providers.is_empty() {
        format!(" [blank keeps {}]", format_provider_list(default_providers))
    } else {
        String::new()
    };
    loop {
        let answer = prompt.ask(
            io,
            &format!("Select harnesses (comma-separated: {choices}){suffix}: "),
        )?;
        if answer.is_empty() && !default_providers.is_empty() {
            return Ok(default_providers.to_vec());
        }
        let (providers, invalid) = parse_provider_list(&answer);
        if !invalid.is_empty() {
            out(io, &format!("Unknown provider(s): {}", invalid.join(", ")));
            continue;
        }
        if !providers.is_empty() {
            return Ok(providers);
        }
        out(io, "Choose at least one provider.");
    }
}

#[derive(PartialEq, Eq)]
enum DetectedMode {
    Detected,
    Add,
}

/// JS: promptDetectedInstallMode(detectedProviders)
fn prompt_detected_install_mode(
    prompt: &mut Prompt,
    io: &mut Io,
    detected: &[&'static str],
) -> R<DetectedMode> {
    if prompt.interactive() {
        let options = [
            RadioOption {
                label: "Detected only".to_string(),
                hint: Some(format!("({})", format_provider_list(detected))),
            },
            RadioOption {
                label: "Customize...".to_string(),
                hint: None,
            },
        ];
        let index = prompt.radio(
            io,
            "Install for detected harnesses only, or add more?",
            &options,
            0,
        )?;
        return Ok(if index == 0 {
            DetectedMode::Detected
        } else {
            DetectedMode::Add
        });
    }
    loop {
        let answer = prompt.ask(
            io,
            &format!(
                "Install target: [1] Detected only ({})  [2] Customize [1]: ",
                format_provider_list(detected)
            ),
        )?;
        if answer.is_empty()
            || ["1", "detected", "detected only", "only", "d"].contains(&answer.as_str())
        {
            return Ok(DetectedMode::Detected);
        }
        if [
            "2",
            "customize",
            "customise",
            "add",
            "add more",
            "more",
            "a",
            "n",
            "no",
        ]
        .contains(&answer.as_str())
        {
            return Ok(DetectedMode::Add);
        }
        out(io, "Choose 1 for detected only, or 2 to customize.");
    }
}

struct ProviderChoice {
    targets: Vec<&'static str>,
    detections: Vec<Detection>,
    explicit: bool,
}

/// JS: chooseInstallProviders(projectRoot, providersValue, {yes}). A thrown
/// `Error` surfaces as `Err(Flow::Throw)`.
fn choose_install_providers(
    sys: &Sys,
    prompt: &mut Prompt,
    io: &mut Io,
    project_root: &str,
    providers_value: Option<&str>,
    yes: bool,
) -> R<ProviderChoice> {
    let detections = sys.collect_install_detections(project_root);
    if let Some(value) = providers_value {
        let (providers, invalid) = parse_provider_list(value);
        if !invalid.is_empty() {
            return Err(Flow::Throw(format!(
                "Unknown provider(s): {}",
                invalid.join(", ")
            )));
        }
        return Ok(ProviderChoice {
            targets: providers,
            detections,
            explicit: true,
        });
    }
    if yes {
        return Ok(ProviderChoice {
            targets: sys.resolve_install_targets(project_root, None),
            detections,
            explicit: false,
        });
    }
    print_install_detections(sys, prompt, io, project_root, &detections);
    let detected = default_detected_providers(&detections);
    if detected.is_empty() {
        let targets = prompt_for_providers(prompt, io, &[])?;
        return Ok(ProviderChoice {
            targets,
            detections,
            explicit: false,
        });
    }
    let mode = prompt_detected_install_mode(prompt, io, &detected)?;
    if mode == DetectedMode::Add {
        let targets = prompt_for_providers(prompt, io, &detected)?;
        return Ok(ProviderChoice {
            targets,
            detections,
            explicit: false,
        });
    }
    Ok(ProviderChoice {
        targets: detected,
        detections,
        explicit: false,
    })
}

/// JS: chooseInstallScope(projectRoot, targets, detections, {yes, scopeValue})
fn choose_install_scope(
    sys: &Sys,
    prompt: &mut Prompt,
    io: &mut Io,
    project_root: &str,
    targets: &[&'static str],
    detections: &[Detection],
    yes: bool,
    scope_value: Option<&str>,
) -> R<Scope> {
    let explicit = scope_value.and_then(normalize_install_scope);
    if let Some(v) = scope_value {
        if explicit.is_none() {
            return Err(Flow::Throw(format!(
                "Unknown install scope: {v}. Use --scope=project or --scope=global."
            )));
        }
    }
    if let Some(s) = explicit {
        return Ok(s);
    }
    if yes {
        return Ok(Scope::Project);
    }
    let fallback = default_install_scope(detections, targets);
    if prompt.interactive() {
        let options = [
            RadioOption {
                label: "Project".to_string(),
                hint: Some(format!("({})", sys.format_path_for_display(project_root))),
            },
            RadioOption {
                label: "Global".to_string(),
                hint: Some(format!("({})", sys.format_path_for_display(&sys.home))),
            },
        ];
        let index = prompt.radio(
            io,
            "Install location",
            &options,
            if fallback == Scope::User { 1 } else { 0 },
        )?;
        return Ok(if index == 1 {
            Scope::User
        } else {
            Scope::Project
        });
    }
    let fallback_label = if fallback == Scope::User {
        "global"
    } else {
        fallback.as_str()
    };
    let answer = prompt.ask(
        io,
        &format!(
            "Install location: project ({}) or global ({})? [{fallback_label}] ",
            sys.format_path_for_display(project_root),
            sys.format_path_for_display(&sys.home)
        ),
    )?;
    if answer.is_empty() {
        return Ok(fallback);
    }
    match normalize_install_scope(&answer) {
        Some(s) => Ok(s),
        None => {
            out(
                io,
                &format!(
                    "Unknown install location \"{answer}\", using {}.",
                    fallback.as_str()
                ),
            );
            Ok(fallback)
        }
    }
}

struct InstallPlan {
    targets: Vec<&'static str>,
    scope: Scope,
    install_root: String,
    hook_root: String,
    explicit: bool,
}

/// JS: chooseInstallPlan(projectRoot, flags, {yes})
fn choose_install_plan(
    sys: &Sys,
    prompt: &mut Prompt,
    io: &mut Io,
    project_root: &str,
    flags: &[String],
    yes: bool,
) -> R<InstallPlan> {
    let providers_value = get_flag_value(flags, "--providers");
    let scope_value = get_install_scope_value(flags);
    let choice = choose_install_providers(sys, prompt, io, project_root, providers_value, yes)?;
    if choice.targets.is_empty() {
        return Err(Flow::Throw(
            "Could not determine a target harness folder.".to_string(),
        ));
    }
    let scope = choose_install_scope(
        sys,
        prompt,
        io,
        project_root,
        &choice.targets,
        &choice.detections,
        yes,
        scope_value.as_deref(),
    )?;
    let install_root = if scope == Scope::User {
        sys.home.clone()
    } else {
        project_root.to_string()
    };
    Ok(InstallPlan {
        targets: choice.targets,
        scope,
        install_root,
        hook_root: project_root.to_string(),
        explicit: choice.explicit,
    })
}

/// JS: decideHookInstall(root, targets, {yes})
fn decide_hook_install(
    prompt: &mut Prompt,
    io: &mut Io,
    root: &str,
    targets: &[&'static str],
    yes: bool,
) -> R<bool> {
    if targets.is_empty() {
        return Ok(false);
    }
    match hook_manifest::get_hook_consent(root).as_deref() {
        Some("declined") => return Ok(false),
        Some("accepted") => return Ok(true),
        _ => {}
    }
    if targets.iter().all(|p| hook_installed_for_provider(root, p)) {
        return Ok(true);
    }
    if yes || !prompt.stdin_tty {
        return Ok(true);
    }
    io.out(HOOK_EXPLAINER);
    let ans = prompt.ask(io, "Install the design hook? (Y/n) ")?;
    let accepted = !(ans == "n" || ans == "no");
    hook_manifest::set_hook_consent(root, if accepted { "accepted" } else { "declined" })
        .map_err(Flow::Throw)?;
    Ok(accepted)
}

// ─── link ────────────────────────────────────────────────────────────────────

/// JS: link(flags)
fn link(flags: &[String], io: &mut Io, bundle: &LocalBundle) -> R<()> {
    let (sys, mut prompt) = ctx(io);
    let force = has_flag(flags, "--force");
    let yes = has_flag(flags, "-y") || has_flag(flags, "--yes");
    let providers_value = get_flag_value(flags, "--providers");
    let root = sys.find_project_root();

    let targets = sys.resolve_install_targets(&root, providers_value);
    if targets.is_empty() {
        err(io, "Could not determine a target harness folder.");
        err(io, "Pass one explicitly, e.g. --providers=claude,cursor");
        return Err(Flow::Exit(1));
    }
    if !yes {
        out(io, &format!("Local bundle: {}", bundle.source.display()));
        out(
            io,
            &format!("Target harness folder(s): {}", targets.join(", ")),
        );
        let ans = prompt.ask(
            io,
            &format!(
                "Link Phoenix UI skills into {} folder(s)? (Y/n) ",
                targets.len()
            ),
        )?;
        if ans == "n" || ans == "no" {
            out(io, "Aborted. Re-run with --providers=<names> to choose explicitly (e.g. --providers=claude,cursor).");
            return Err(Flow::Exit(0));
        }
    }
    let bundle_root = bundle.stage_linked(&root).map_err(Flow::Throw)?;
    let result = bundle::link_provider_skills(io, &bundle_root, &root, &targets, force)
        .map_err(Flow::Throw)?;
    // Linked installs are excluded from install/update refreshes (overwriting
    // a symlink would destroy the link), so this is the only path that can
    // deliver the OpenCode command bridge to them. A copy, not a symlink: the
    // bridge is static and OpenCode scans the real commands dir. No-ops when
    // the source checkout has no built commands (#483).
    bundle::copy_provider_commands(&sys, &bundle_root, &root, &targets, Some(Scope::Project));
    if result.linked == 0 && result.already == 0 {
        if result.skipped > 0 {
            err(
                io,
                "Nothing was linked because matching skill folders already exist.",
            );
            err(io, "Existing skills were left untouched. Re-run with --force to replace them with links.");
        } else {
            err(
                io,
                &format!(
                    "Nothing was linked: {} had no variants for {}.",
                    bundle_root,
                    targets.join(", ")
                ),
            );
        }
        return Err(Flow::Exit(1));
    }
    // Linked skill dirs resolve into the source checkout; repair the
    // launcher/binary executable bit there too (through the symlink), so a
    // checkout whose copier dropped modes still runs.
    for provider in &targets {
        let skills_dir = util::jsp::join(&[&root, provider, "skills"]);
        for skill in util::read_dir_names(&skills_dir).unwrap_or_default() {
            let dir = util::jsp::join(&[&skills_dir, &skill]);
            if util::is_dir(&dir) {
                crate::engine_binary::ensure_executable_scripts(&dir);
            }
        }
    }
    let mut parts = Vec::new();
    if result.linked > 0 {
        parts.push(format!("{} linked", result.linked));
    }
    if result.already > 0 {
        parts.push(format!("{} already linked", result.already));
    }
    if result.skipped > 0 {
        parts.push(format!("{} skipped", result.skipped));
    }
    out(
        io,
        &format!(
            "Linked Phoenix UI into: {} ({}).",
            targets.join(", "),
            parts.join(", ")
        ),
    );
    out(
        io,
        "Rerun this command with the explicit local bundle when its contents change.\n",
    );
    Ok(())
}

// ─── install ─────────────────────────────────────────────────────────────────

/// Sentinel for the JS `process.exit(1)` inside install's already-installed
/// `try` block (an exit, not an error, so no "Install check failed" line).
const EXIT_1: &str = "\u{0}exit1";

/// JS: install(flags)
fn install(flags: &[String], io: &mut Io, bundle: &LocalBundle) -> R<()> {
    let (sys, mut prompt) = ctx(io);
    let force = has_flag(flags, "--force");
    let yes = has_flag(flags, "-y") || has_flag(flags, "--yes");
    let install_hooks = !has_flag(flags, "--no-hooks");
    let project_root = sys.find_project_root();
    if !yes {
        print_install_intro(&prompt, io);
    }
    let plan = match choose_install_plan(&sys, &mut prompt, io, &project_root, flags, yes) {
        Ok(p) => p,
        Err(Flow::Throw(msg)) => {
            err(io, &msg);
            err(
                io,
                "Pass providers explicitly, e.g. --providers=claude,cursor",
            );
            return Err(Flow::Exit(1));
        }
        Err(other) => return Err(other),
    };
    let InstallPlan {
        targets,
        scope,
        install_root,
        hook_root,
        explicit,
    } = plan;
    let scope_opt = Some(scope);
    let existing = sys.is_already_installed(&install_root, scope_opt);
    let installed_targets: Vec<&'static str> = if existing.is_some() {
        sys.find_installed_providers(&install_root, scope_opt)
    } else {
        Vec::new()
    };
    let missing_selected_targets: Vec<&'static str> = if existing.is_some() && !force && explicit {
        targets
            .iter()
            .copied()
            .filter(|p| !installed_targets.contains(p))
            .collect()
    } else {
        Vec::new()
    };

    if let (Some(existing), true) = (
        existing,
        !force && missing_selected_targets.len() < targets.len(),
    ) {
        out(
            io,
            &format!("Phoenix UI skills are already installed (found in {existing}/)."),
        );
        let selected_installed: Vec<&'static str> = targets
            .iter()
            .copied()
            .filter(|p| installed_targets.contains(p))
            .collect();
        let linked_targets =
            sys.find_linked_providers(&install_root, &selected_installed, scope_opt);
        let copy_targets: Vec<&'static str> = selected_installed
            .iter()
            .copied()
            .filter(|p| !linked_targets.contains(p))
            .collect();
        let hook_targets: Vec<&'static str> = selected_installed
            .iter()
            .chain(missing_selected_targets.iter())
            .copied()
            .collect();
        let want_hooks =
            install_hooks && decide_hook_install(&mut prompt, io, &hook_root, &hook_targets, yes)?;
        let mut bundle_dir: Option<String> = None;
        let outcome: Result<(), String> = (|| {
            if !linked_targets.is_empty() {
                out(
                    io,
                    &format!("Linked skills found in: {}", linked_targets.join(", ")),
                );
                out(io, "Rerun `phoenix-ui link --bundle-root DIR` with the explicit local bundle to replace or add links.");
                if !copy_targets.is_empty() {
                    out(
                        io,
                        &format!(
                            "Continuing with copied installs in: {}\n",
                            copy_targets.join(", ")
                        ),
                    );
                }
            }
            let mut updated = 0usize;
            let missing_hook_targets: Vec<&'static str> = if want_hooks {
                hook_targets
                    .iter()
                    .copied()
                    .filter(|p| !hook_installed_for_provider(&hook_root, p))
                    .collect()
            } else {
                Vec::new()
            };
            let mut update_check_skipped = false;
            if !copy_targets.is_empty()
                || !missing_hook_targets.is_empty()
                || !missing_selected_targets.is_empty()
            {
                match bundle.stage(&sys) {
                    Ok(dir) => bundle_dir = Some(dir),
                    Err(e) => {
                        if !missing_hook_targets.is_empty() || !missing_selected_targets.is_empty()
                        {
                            return Err(e);
                        }
                        update_check_skipped = true;
                        out(io, &format!("Could not check for skill updates: {e}"));
                    }
                }
            }
            let bdir = bundle_dir.clone().unwrap_or_default();

            if !update_check_skipped
                && !copy_targets.is_empty()
                && !bundle::is_up_to_date(
                    &sys,
                    &install_root,
                    &copy_targets,
                    &bdir,
                    scope_opt,
                    scope_opt,
                )?
            {
                let refreshed = bundle::refresh_provider_skills(
                    &sys,
                    &bdir,
                    &install_root,
                    &copy_targets,
                    scope_opt,
                )?;
                updated = refreshed.len();
                let agents = bundle::copy_provider_agents(
                    &sys,
                    &bdir,
                    &install_root,
                    &copy_targets,
                    scope_opt,
                )?;
                bundle::report_provider_agents(&sys, io, &agents);
                bundle::copy_provider_commands(
                    &sys,
                    &bdir,
                    &install_root,
                    &copy_targets,
                    scope_opt,
                );
                let v = sys.get_skills_version(&install_root, scope_opt);
                out(
                    io,
                    &format!("Updated {updated} skill(s){}.", to_version_suffix(&v)),
                );
                install_engine_binaries(&sys, io, &refreshed);
            }

            let mut fresh_written = 0usize;
            if !update_check_skipped && !missing_selected_targets.is_empty() {
                let written = bundle::copy_provider_skills(
                    &sys,
                    &bdir,
                    &install_root,
                    &missing_selected_targets,
                    scope_opt,
                )?;
                fresh_written = written.len();
                if fresh_written == 0 {
                    err(
                        io,
                        &format!(
                            "Nothing was installed: the bundle had no variants for {}.",
                            missing_selected_targets.join(", ")
                        ),
                    );
                    return Err(EXIT_1.to_string());
                }
                out(
                    io,
                    &format!(
                        "Installed Phoenix UI into: {} ({})",
                        missing_selected_targets.join(", "),
                        if scope == Scope::User {
                            "global"
                        } else {
                            "project"
                        }
                    ),
                );
                install_engine_binaries(&sys, io, &written);
                let agents = bundle::copy_provider_agents(
                    &sys,
                    &bdir,
                    &install_root,
                    &missing_selected_targets,
                    scope_opt,
                )?;
                bundle::report_provider_agents(&sys, io, &agents);
            }

            let written_hook_targets = if !missing_hook_targets.is_empty() {
                copy_provider_hooks(
                    &sys,
                    &bdir,
                    &hook_root,
                    &missing_hook_targets,
                    false,
                    Some(&install_root),
                )?
            } else {
                Vec::new()
            };
            if !written_hook_targets.is_empty() {
                out(
                    io,
                    &format!("Installed hooks into: {}", written_hook_targets.join(", ")),
                );
            }

            if update_check_skipped {
                out(io, "Existing skills were left unchanged.");
                out(io, "Run with --force to reinstall.\n");
            } else if updated == 0 && written_hook_targets.is_empty() && fresh_written == 0 {
                let v = sys.get_skills_version(&install_root, scope_opt);
                out(io, &format!("Skills are up to date{}.", version_suffix(&v)));
                out(io, "Run with --force to reinstall.\n");
            } else {
                out(io, "Done!\n");
            }
            Ok(())
        })();
        if let Some(dir) = &bundle_dir {
            util::rm_rf(dir);
        }
        return match outcome {
            Ok(()) => Err(Flow::Exit(0)),
            Err(e) if e == EXIT_1 => Err(Flow::Exit(1)),
            Err(e) => {
                err(io, &format!("Install check failed: {e}"));
                Err(Flow::Exit(1))
            }
        };
    }

    if targets.is_empty() {
        err(io, "Could not determine a target harness folder.");
        err(io, "Pass one explicitly, e.g. --providers=.claude,.cursor");
        return Err(Flow::Exit(1));
    }

    let want_hooks =
        install_hooks && decide_hook_install(&mut prompt, io, &hook_root, &targets, yes)?;

    out(io, "\nPreparing verified local Phoenix UI skills...");
    let bundle_dir = match bundle.stage(&sys) {
        Ok(d) => d,
        Err(e) => {
            err(io, &format!("Local bundle staging failed: {e}"));
            return Err(Flow::Exit(1));
        }
    };

    let outcome: Result<(Vec<String>, Vec<bundle::AgentResult>, Vec<&'static str>), String> =
        (|| {
            let written = bundle::copy_provider_skills(
                &sys,
                &bundle_dir,
                &install_root,
                &targets,
                scope_opt,
            )?;
            let agents = bundle::copy_provider_agents(
                &sys,
                &bundle_dir,
                &install_root,
                &targets,
                scope_opt,
            )?;
            bundle::copy_provider_commands(&sys, &bundle_dir, &install_root, &targets, scope_opt);
            let hooks = if want_hooks {
                copy_provider_hooks(
                    &sys,
                    &bundle_dir,
                    &hook_root,
                    &targets,
                    force,
                    Some(&install_root),
                )?
            } else {
                Vec::new()
            };
            Ok((written, agents, hooks))
        })();
    let (written, agent_results, hook_targets) = match outcome {
        Ok(v) => v,
        Err(e) => {
            util::rm_rf(&bundle_dir);
            err(io, &format!("Install failed: {e}"));
            return Err(Flow::Exit(1));
        }
    };
    util::rm_rf(&bundle_dir);

    if written.is_empty() {
        err(
            io,
            &format!(
                "Nothing was installed: the bundle had no variants for {}.",
                targets.join(", ")
            ),
        );
        return Err(Flow::Exit(1));
    }
    out(
        io,
        &format!(
            "Installed Phoenix UI into: {} ({})",
            targets.join(", "),
            if scope == Scope::User {
                "global"
            } else {
                "project"
            }
        ),
    );
    install_engine_binaries(&sys, io, &written);
    bundle::report_provider_agents(&sys, io, &agent_results);
    if !hook_targets.is_empty() {
        out(
            io,
            &format!("Installed hooks into: {}", hook_targets.join(", ")),
        );
    }
    out(io, "\nDone! Now type /phoenix-ui init in your AI coding agent's chat (not in this terminal) to set up design context.\n");
    Ok(())
}

// ─── update ──────────────────────────────────────────────────────────────────

/// JS: update(flags)
fn update(flags: &[String], io: &mut Io, bundle: &LocalBundle) -> R<()> {
    let (sys, mut prompt) = ctx(io);
    let yes = has_flag(flags, "-y") || has_flag(flags, "--yes");
    let force = has_flag(flags, "--force");
    let install_hooks = !has_flag(flags, "--no-hooks");
    let scope_value = get_install_scope_value(flags);
    let explicit_scope = scope_value.as_deref().and_then(normalize_install_scope);
    if let Some(v) = &scope_value {
        if explicit_scope.is_none() {
            err(
                io,
                &format!("Unknown update scope: {v}. Use --project or --user."),
            );
            return Err(Flow::Exit(1));
        }
    }

    let project_root = sys.find_project_root();
    let home = sys.home.clone();

    let target = sys.resolve_update_target(&project_root, explicit_scope);
    let Some(target) = target else {
        match explicit_scope {
            Some(scope) => {
                let where_ = if scope == Scope::User {
                    format!("user level ({})", sys.format_path_for_display(&home))
                } else {
                    format!("this project ({project_root})")
                };
                out(
                    io,
                    &format!("No Phoenix UI skill folders found at the {where_}."),
                );
            }
            None => out(
                io,
                "No Phoenix UI skill folders found in this project or at the user level.",
            ),
        }
        out(io, "Run `phoenix-ui install` to install first.");
        return Err(Flow::Exit(1));
    };

    let (root, scope, agent_scope, providers, scope_label) = match target {
        UpdateTarget::Resolved {
            root,
            scope,
            agent_scope,
            providers,
            scope_label,
        } => (root, scope, agent_scope, providers, scope_label),
        UpdateTarget::Ambiguous {
            project_providers,
            user_providers,
        } => {
            out(
                io,
                "Phoenix UI is installed both here and at the user level:",
            );
            out(
                io,
                &format!(
                    "  project    {project_root}  ({})",
                    project_providers.join(", ")
                ),
            );
            out(
                io,
                &format!(
                    "  user level {}  ({})",
                    sys.format_path_for_display(&home),
                    user_providers.join(", ")
                ),
            );
            let mut pick_user = false;
            if !yes && prompt.stdin_tty {
                let ans = prompt.ask(io, "Update which? [project]/user: ")?;
                pick_user = ["user", "u", "global", "home"].contains(&ans.as_str());
            } else {
                out(io, "Defaulting to the project. Re-run with --user to update the user-level install instead.");
            }
            if pick_user {
                (
                    home.clone(),
                    Some(Scope::User),
                    Some(Scope::User),
                    user_providers,
                    "user level",
                )
            } else {
                (
                    project_root.clone(),
                    Some(Scope::Project),
                    Some(Scope::Project),
                    project_providers,
                    "this project",
                )
            }
        }
    };

    out(
        io,
        &format!(
            "Updating the {scope_label} install: {} ({})",
            sys.format_path_for_display(&root),
            providers.join(", ")
        ),
    );
    let linked_providers = sys.find_linked_providers(&root, &providers, scope);
    let copy_providers: Vec<&'static str> = providers
        .iter()
        .copied()
        .filter(|p| !linked_providers.contains(p))
        .collect();

    if !linked_providers.is_empty() {
        out(
            io,
            &format!("Linked skills found in: {}", linked_providers.join(", ")),
        );
        out(io, "Rerun `phoenix-ui link --bundle-root DIR` with the explicit local bundle to replace or add links.");
        if copy_providers.is_empty() {
            return Err(Flow::Exit(0));
        }
        out(
            io,
            &format!(
                "Continuing with copied installs in: {}\n",
                copy_providers.join(", ")
            ),
        );
    }

    out(io, "Checking against the explicit local bundle...");

    let tmp_dir = match bundle.stage(&sys) {
        Ok(d) => d,
        Err(e) => {
            err(io, &format!("Local bundle staging failed: {e}"));
            return Err(Flow::Exit(1));
        }
    };

    let up_to_date =
        match bundle::is_up_to_date(&sys, &root, &copy_providers, &tmp_dir, scope, agent_scope) {
            Ok(v) => v,
            Err(e) => {
                // JS: an fs error inside isUpToDate is uncaught (main's catch).
                return Err(Flow::Throw(e));
            }
        };
    if up_to_date {
        let outcome: R<Vec<&'static str>> = (|| {
            let want_hooks =
                install_hooks && decide_hook_install(&mut prompt, io, &root, &copy_providers, yes)?;
            let hook_targets = if want_hooks {
                copy_provider_hooks(&sys, &tmp_dir, &root, &copy_providers, force, None)
                    .map_err(Flow::Throw)?
            } else {
                Vec::new()
            };
            Ok(hook_targets)
        })();
        util::rm_rf(&tmp_dir);
        return match outcome {
            Ok(hook_targets) => {
                let v = sys.get_skills_version(&root, scope);
                out(io, &format!("Skills are up to date{}.", version_suffix(&v)));
                if !hook_targets.is_empty() {
                    out(
                        io,
                        &format!("Installed hooks into: {}", hook_targets.join(", ")),
                    );
                }
                out(io, "Nothing else to do.");
                Err(Flow::Exit(0))
            }
            Err(Flow::Throw(e)) => {
                err(io, &format!("Update failed: {e}"));
                Err(Flow::Exit(1))
            }
            // JS: the try/catch around decideHookInstall swallows the prompt
            // abort too, printing its message.
            Err(Flow::Abort) => {
                err(io, "Update failed: Aborted.");
                Err(Flow::Exit(1))
            }
            Err(other) => Err(other),
        };
    }

    out(
        io,
        &format!("Found skills in: {}", copy_providers.join(", ")),
    );

    if !yes {
        let ans = prompt.ask(
            io,
            &format!(
                "Update skills in {} provider folder(s)? (Y/n) ",
                copy_providers.len()
            ),
        )?;
        if ans == "n" || ans == "no" {
            util::rm_rf(&tmp_dir);
            out(io, "Aborted.");
            return Err(Flow::Exit(0));
        }
    }

    let outcome: R<()> = (|| {
        let refreshed =
            bundle::refresh_provider_skills(&sys, &tmp_dir, &root, &copy_providers, scope)
                .map_err(Flow::Throw)?;
        let agents =
            bundle::copy_provider_agents(&sys, &tmp_dir, &root, &copy_providers, agent_scope)
                .map_err(Flow::Throw)?;
        bundle::report_provider_agents(&sys, io, &agents);
        bundle::copy_provider_commands(&sys, &tmp_dir, &root, &copy_providers, scope);
        let want_hooks =
            install_hooks && decide_hook_install(&mut prompt, io, &root, &providers, yes)?;
        let hook_targets = if want_hooks {
            copy_provider_hooks(&sys, &tmp_dir, &root, &providers, force, None)
                .map_err(Flow::Throw)?
        } else {
            Vec::new()
        };
        util::rm_rf(&tmp_dir);
        let v = sys.get_skills_version(&root, scope);
        out(
            io,
            &format!(
                "Updated {} skill(s){}.",
                refreshed.len(),
                to_version_suffix(&v)
            ),
        );
        install_engine_binaries(&sys, io, &refreshed);
        if !hook_targets.is_empty() {
            out(
                io,
                &format!("Installed hooks into: {}", hook_targets.join(", ")),
            );
        }
        out(io, "Done!\n");
        Ok(())
    })();
    match outcome {
        Ok(()) => Ok(()),
        Err(Flow::Throw(e)) => {
            err(io, &format!("Update failed: {e}"));
            util::rm_rf(&tmp_dir);
            Err(Flow::Exit(1))
        }
        Err(Flow::Abort) => {
            err(io, "Update failed: Aborted.");
            util::rm_rf(&tmp_dir);
            Err(Flow::Exit(1))
        }
        Err(other) => Err(other),
    }
}
