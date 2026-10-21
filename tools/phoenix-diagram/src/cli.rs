use phoenix_diagram::{
    browser::{self, BrowserOptions},
    check::FileIdentity,
    delivery::{self, CompileRequest, EngineIdentity},
    diagnostic::Diagnostic,
    input::{CompileOptions, DiagramType, InputContract, Quality},
    install::{self, InstallRequest},
    output,
    render::EmbeddedAssets,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const HELP: &str = "phoenix-diagram 0.2.0\n\nengine-probe\ninstall --project DIR --bundle-root DIR [--providers claude,codex] [--dry-run] --json\ndoctor --json\nguide <scenario> --json\nvalidate <type> <candidate.json> [--quality standard|showcase] [--input-contract NAME] [--repo-root DIR] [--layout-json] --json\ndeliver <type> <candidate.json> <output.html> [compile options] [--repo-root DIR] --json\ncheck <output.html> [--strict-provenance] --json\nbrowser-check <output.html> [--out-dir DIR] --json\nvisual-check <output.html> [--out-dir DIR] --json\nfinalize <type> <candidate.json> <output.html> [compile options] [--repo-root DIR] [--out-dir DIR] --json\nreview-record <output.html> --record <review.json> --json\nrecover-output <private-recovery-directory> --json\n\nTypes: architecture, workflow, sequence, dataflow, lifecycle, erd, class, tree, timeline, waterfall.\nUnmarked input of the first five types uses legacy-2.17; new input declares meta.input_contract=phoenix-v1.\nerd, class, tree, timeline and waterfall accept only phoenix-v1.\n--repo-root names the local Git checkout that cited sources are checked against; read evidence.status, exit 0 does not mean sources were verified.\nFinal delivery requires automatic checks and an actual image review.\n";
const RECOVERY_HELP: &str = "recover-output <private-recovery-directory> --json\n\nAfter an interrupted publication, locate its durable record in the original output parent:\n  ls -la \"<original output parent>\"\nRead .phoenix-diagram-recovery-<id>/publication-recovery-v1.json and identify its target.\nCleanup can retain the same record at .phoenix-diagram-recovery-<id>.cleanup.json or .phoenix-diagram-output-cleanup/<id>.json.\nAlways pass the original .phoenix-diagram-recovery-<id> directory, even if it is now absent.\nThe fixed cleanup directory retains its bound owner-v1.json. Unknown entries or ambiguous records are preserved and rejected.\nMultiple records require an explicit choice. The command preserves later claimants and changed bindings.\nRecovery restores file bytes; delivery checks and image review remain separate.\n";

struct Arguments {
    positional: Vec<OsString>,
    options: BTreeMap<String, Option<OsString>>,
}
impl Arguments {
    fn parse(args: Vec<OsString>) -> Result<Self, String> {
        let mut result = Self {
            positional: Vec::new(),
            options: BTreeMap::new(),
        };
        let mut args = args.into_iter();
        let mut literal = false;
        while let Some(arg) = args.next() {
            let text = arg.to_str();
            if !literal && text == Some("--") {
                literal = true;
                continue;
            }
            if !literal && text.is_some_and(|v| v.starts_with('-')) {
                let text = text.unwrap();
                let raw = match text {
                    "-h" => "help",
                    "-V" => "version",
                    _ => text
                        .strip_prefix("--")
                        .ok_or_else(|| format!("Unknown option {text}"))?,
                };
                let (key, inline) = raw
                    .split_once('=')
                    .map_or((raw, None), |(k, v)| (k, Some(OsString::from(v))));
                let value = match key {
                    "json" | "help" | "version" | "strict-provenance" | "layout-json"
                    | "dry-run" => {
                        if inline.is_some() {
                            return Err(format!("--{key} has no value"));
                        }
                        None
                    }
                    "quality" | "input-contract" | "out-dir" | "record" | "project"
                    | "bundle-root" | "providers" | "repo-root" => Some(
                        inline
                            .or_else(|| args.next())
                            .ok_or_else(|| format!("--{key} needs a value"))?,
                    ),
                    _ => return Err(format!("Unknown option --{key}")),
                };
                if result.options.insert(key.into(), value).is_some() {
                    return Err(format!("Duplicate option --{key}"));
                }
            } else {
                result.positional.push(arg);
            }
        }
        Ok(result)
    }
    fn has(&self, name: &str) -> bool {
        self.options.contains_key(name)
    }
    fn value(&self, name: &str) -> Option<&OsStr> {
        self.options.get(name).and_then(|v| v.as_deref())
    }
    fn allow(&self, names: &[&str]) -> Result<(), String> {
        for name in self.options.keys() {
            if !matches!(name.as_str(), "json" | "help") && !names.contains(&name.as_str()) {
                return Err(format!("--{name} is not valid for this command"));
            }
        }
        Ok(())
    }
    fn compile_options(&self) -> Result<CompileOptions, String> {
        let quality = self
            .value("quality")
            .map(|v| match v.to_str() {
                Some("standard") => Ok(Quality::Standard),
                Some("showcase") => Ok(Quality::Showcase),
                _ => Err("Quality must be standard or showcase".to_owned()),
            })
            .transpose()?;
        let input_contract = self
            .value("input-contract")
            .map(|v| match v.to_str() {
                Some("legacy-2.17") => Ok(InputContract::Legacy217),
                Some("phoenix-v1") => Ok(InputContract::PhoenixV1),
                _ => Err("Input contract must be legacy-2.17 or phoenix-v1".to_owned()),
            })
            .transpose()?;
        Ok(CompileOptions {
            quality,
            input_contract,
            ..Default::default()
        })
    }
}

fn error(command: &str, code: &str, message: impl Into<String>, exit: u8) -> (Value, u8) {
    let mut diagnostic = Diagnostic::error(None, code, "", message);
    diagnostic.stage = command.into();
    (
        json!({"schemaVersion":1,"tool":"phoenix-diagram","toolVersion":phoenix_diagram::TOOL_VERSION,"command":command,"status":"failed","diagnostics":[diagnostic]}),
        exit,
    )
}
fn encoded<T: Serialize>(value: &T, exit: u8) -> (Value, u8) {
    serde_json::to_value(value)
        .map(|v| (v, exit))
        .unwrap_or_else(|e| error("output", "output/json", e.to_string(), 4))
}
fn engine() -> Result<EngineIdentity, String> {
    let path = std::env::current_exe().map_err(|e| e.to_string())?;
    let binding = output::FileBinding::capture(&path).map_err(|e| e.message)?;
    Ok(EngineIdentity {
        source_input_sha256: env!("PHOENIX_DIAGRAM_SOURCE_INPUT_SHA256").into(),
        input_contract_schema: phoenix_diagram::INPUT_CONTRACT_SCHEMA,
        receipt_schema: phoenix_diagram::RECEIPT_SCHEMA,
        viewer_revision: phoenix_diagram::VIEWER_REVISION.into(),
        binary: FileIdentity::of(binding.bytes()),
    })
}
fn browser_options(args: &Arguments, capture: bool) -> BrowserOptions {
    BrowserOptions {
        capture,
        out_dir: args.value("out-dir").map(PathBuf::from),
        ..Default::default()
    }
}
fn guide(scenario: &str) -> Value {
    let lower = scenario.to_lowercase();
    let hints = [
        (
            DiagramType::Architecture,
            &["architecture", "component", "架构", "组件"][..],
        ),
        (
            DiagramType::Workflow,
            &["workflow", "flowchart", "审批", "工作流"][..],
        ),
        (
            DiagramType::Sequence,
            &["sequence", "调用", "时序", "消息"][..],
        ),
        (
            DiagramType::Dataflow,
            &["dataflow", "数据流", "数据管道"][..],
        ),
        (
            DiagramType::Lifecycle,
            &["lifecycle", "state", "生命周期", "状态"][..],
        ),
        (
            DiagramType::Erd,
            &[
                "erd",
                "erdiagram",
                "entity",
                "table",
                "primary key",
                "foreign key",
                "cardinality",
                "实体",
                "表结构",
                "数据表",
                "数据库表",
                "主键",
                "外键",
                "基数",
            ][..],
        ),
        (
            DiagramType::Class,
            &[
                "class",
                "interface",
                "uml",
                "inheritance",
                "realization",
                "composition",
                "类图",
                "接口",
                "继承",
                "实现关系",
                "组合",
            ][..],
        ),
        (
            DiagramType::Tree,
            &[
                "tree",
                "hierarchy",
                "breakdown",
                "repository layout",
                "org chart",
                "module tree",
                "树",
                "层级",
                "层次",
                "目录结构",
                "组织结构",
                "分解",
                "模块树",
            ][..],
        ),
        (
            DiagramType::Timeline,
            &[
                "timeline",
                "incident",
                "release history",
                "timestamp",
                "chronology",
                "时间线",
                "时间轴",
                "事故",
                "复盘",
                "发布历史",
                "时间戳",
            ][..],
        ),
        (
            DiagramType::Waterfall,
            &[
                "waterfall",
                "span",
                "trace",
                "latency",
                "duration",
                "where the time went",
                "瀑布",
                "耗时",
                "链路",
                "延迟",
                "调用跨度",
            ][..],
        ),
    ];
    let mut matches: Vec<_> = hints
        .iter()
        .filter(|(_, terms)| terms.iter().any(|term| lower.contains(term)))
        .map(|(kind, _)| *kind)
        .collect();
    if matches.is_empty() {
        matches = DiagramType::ALL.to_vec();
    }
    json!({"schemaVersion":1,"tool":"phoenix-diagram","toolVersion":phoenix_diagram::TOOL_VERSION,"command":"guide","status":"pass","diagnostics":[],"scenario":scenario,"resourceRoot":std::env::var_os("PHOENIX_DIAGRAM_BUNDLE_ROOT").map(PathBuf::from),"relativeTo":"phoenix-diagram skill root","matches":matches.iter().map(|kind|json!({"type":kind,"schema":format!("schemas/{}.schema.json",kind.as_str()),"commonSchema":"schemas/common.schema.json","example":format!("examples/minimal.{}.json",kind.as_str())})).collect::<Vec<_>>()})
}

fn dispatch(args: &Arguments) -> Result<(Value, u8), String> {
    let command = args
        .positional
        .first()
        .and_then(|v| v.to_str())
        .ok_or("A command is required")?;
    let positional = &args.positional[1..];
    if command == "install" {
        args.allow(&["project", "bundle-root", "providers", "dry-run"])?;
        if !positional.is_empty() {
            return Err(
                "install takes --project and --bundle-root, without positional arguments".into(),
            );
        }
        let project = args
            .value("project")
            .ok_or("install requires --project DIR")?;
        let bundle_root = args
            .value("bundle-root")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("PHOENIX_DIAGRAM_BUNDLE_ROOT").map(PathBuf::from))
            .ok_or("install requires --bundle-root DIR")?;
        let providers = args
            .value("providers")
            .map(|v| {
                v.to_str()
                    .ok_or("Provider names must be UTF-8")
                    .map(|v| v.split(',').map(str::to_owned).collect::<Vec<_>>())
            })
            .transpose()?;
        let request = InstallRequest {
            project: project.into(),
            version_manifest: bundle_root.join("ENGINE.json"),
            bundle_root,
            engine: std::env::current_exe().map_err(|e| e.to_string())?,
            providers,
        };
        return Ok(
            match if args.has("dry-run") {
                install::plan_install(&request)
            } else {
                install::apply_install(&request)
            } {
                Ok(receipt) => encoded(&receipt, receipt.exit_code()),
                Err(failure) => error(
                    "install",
                    &failure.code,
                    &failure.message,
                    failure.exit_code(),
                ),
            },
        );
    }
    if command == "guide" {
        args.allow(&[])?;
        if positional.len() != 1 {
            return Err("guide requires one scenario".into());
        }
        return Ok((guide(&positional[0].to_string_lossy()), 0));
    }
    if command == "recover-output" {
        args.allow(&[])?;
        if positional.len() != 1 {
            return Err("recover-output requires one private directory".into());
        }
        let receipt = output::recover_publication(Path::new(&positional[0]));
        let code = receipt.exit_code();
        let diagnostics = if code == 0 {
            vec![]
        } else {
            let mut d = Diagnostic::error(None, &receipt.code, "", &receipt.message);
            d.stage = "recover-output".into();
            d.evidence = json!({"publicState":receipt.public_state,"preserved":receipt.preserved});
            vec![d]
        };
        return Ok((
            json!({"schemaVersion":1,"tool":"phoenix-diagram","toolVersion":phoenix_diagram::TOOL_VERSION,"command":"recover-output","status":if code==0 {"pass"} else {"failed"},"recoveryStatus":receipt.status,"recoveryDirectory":receipt.recovery_directory,"target":receipt.target,"publicState":receipt.public_state,"preserved":receipt.preserved,"diagnostics":diagnostics}),
            code,
        ));
    }
    if !matches!(
        command,
        "doctor"
            | "validate"
            | "deliver"
            | "finalize"
            | "check"
            | "browser-check"
            | "visual-check"
            | "review-record"
    ) {
        return Err(format!("Unknown command {command}"));
    }
    let engine = match engine() {
        Ok(e) => e,
        Err(e) => return Ok(error(command, "engine/current-binary", e, 4)),
    };
    if command == "doctor" {
        args.allow(&[])?;
        if !positional.is_empty() {
            return Err("doctor takes no positional arguments".into());
        }
        let browser = browser::doctor_browser();
        let ready = browser["status"] == "available";
        let recovery = std::env::var("PHOENIX_DIAGRAM_SETUP_COMMAND").unwrap_or_else(|_| {
            format!(
                "Use the matching local @xgent-ai/skills@{} package bootstrap: phoenix-diagram engine install --project . --release-dir MATCHING_RELEASE_DIR",
                env!("PHOENIX_DIAGRAM_NPM_PACKAGE_VERSION")
            )
        });
        return Ok((
            json!({"schemaVersion":1,"tool":"phoenix-diagram","toolVersion":phoenix_diagram::TOOL_VERSION,"command":"doctor","status":if ready {"pass"} else {"failed"},"ready":ready,"engine":engine,"platform":{"os":std::env::consts::OS,"architecture":std::env::consts::ARCH},"resources":{"status":"available","readerBytes":EmbeddedAssets::builtin().template.len()},"browser":browser,"supportedTypes":DiagramType::ALL,"diagnostics":[],"recovery":{"engineInstall":recovery}}),
            if ready { 0 } else { 4 },
        ));
    }
    if matches!(command, "validate" | "deliver" | "finalize") {
        args.allow(if command == "validate" {
            &["quality", "input-contract", "layout-json", "repo-root"][..]
        } else if command == "finalize" {
            &["quality", "input-contract", "out-dir", "repo-root"][..]
        } else {
            &["quality", "input-contract", "repo-root"][..]
        })?;
        let repo_root = args.value("repo-root").map(Path::new);
        if repo_root.is_some_and(|root| !root.is_dir()) {
            return Err("--repo-root must be an existing directory".into());
        }
        let count = if command == "validate" { 2 } else { 3 };
        if positional.len() != count {
            return Err(format!(
                "{command} requires a type, candidate{}",
                if count == 3 { " and output" } else { "" }
            ));
        }
        let kind: DiagramType = positional[0]
            .to_str()
            .ok_or("Diagram type must be UTF-8")?
            .parse()
            .map_err(|_| "Unsupported diagram type".to_owned())?;
        let options = args.compile_options()?;
        let assets = EmbeddedAssets::builtin();
        let request = CompileRequest {
            kind,
            candidate: Path::new(&positional[1]),
            options: &options,
            engine: &engine,
            assets: &assets,
            repo_root,
        };
        return Ok(match command {
            "validate" => {
                let receipt = delivery::validate(&request);
                let code = receipt.exit_code();
                let layout = receipt.layout.clone();
                let (mut value, code) = encoded(&receipt, code);
                if args.has("layout-json") {
                    value["layout"] = layout.unwrap_or(Value::Null);
                }
                (value, code)
            }
            "deliver" => {
                let receipt = delivery::deliver(&request, Path::new(&positional[2]));
                encoded(&receipt, receipt.exit_code())
            }
            _ => {
                let receipt = delivery::finalize(
                    &request,
                    Path::new(&positional[2]),
                    &browser_options(args, true),
                );
                encoded(&receipt, receipt.exit_code())
            }
        });
    }
    if positional.len() != 1 {
        return Err(format!("{command} requires one output HTML path"));
    }
    let output = Path::new(&positional[0]);
    Ok(match command {
        "check" => {
            args.allow(&["strict-provenance"])?;
            let receipt = delivery::check(output, args.has("strict-provenance"), &engine);
            encoded(&receipt, receipt.exit_code())
        }
        "review-record" => {
            args.allow(&["record"])?;
            let record = args
                .value("record")
                .ok_or("review-record requires --record FILE")?;
            let receipt = delivery::record_review(output, Path::new(record), &engine);
            encoded(&receipt, receipt.exit_code())
        }
        _ => {
            args.allow(&["out-dir"])?;
            let visual = command == "visual-check";
            match delivery::browser_check(output, &engine, &browser_options(args, visual), visual) {
                Ok(receipt) => encoded(&receipt, receipt.exit_code()),
                Err(receipt) => encoded(&receipt, receipt.exit_code()),
            }
        }
    })
}

pub fn run(raw: Vec<OsString>) -> ExitCode {
    let raw_command = raw
        .first()
        .and_then(|v| v.to_str())
        .unwrap_or("arguments")
        .to_owned();
    let args = match Arguments::parse(raw) {
        Ok(a) => a,
        Err(e) => return print(error(&raw_command, "arguments/invalid", e, 2)),
    };
    let command = args
        .positional
        .first()
        .and_then(|v| v.to_str())
        .unwrap_or("arguments");
    if args.has("version") {
        println!("{}", phoenix_diagram::TOOL_VERSION);
        return ExitCode::SUCCESS;
    }
    if args.has("help") || args.positional.is_empty() {
        println!(
            "{}",
            if command == "recover-output" {
                RECOVERY_HELP
            } else {
                HELP
            }
        );
        return ExitCode::SUCCESS;
    }
    if command == "engine-probe" {
        if args.positional.len() != 1 || !args.options.is_empty() {
            return print(error(
                command,
                "arguments/invalid",
                "engine-probe takes no options",
                2,
            ));
        }
        println!(
            "phoenix-diagram-engine {} input={} receipt={} viewer={}",
            phoenix_diagram::TOOL_VERSION,
            phoenix_diagram::INPUT_CONTRACT_SCHEMA,
            phoenix_diagram::RECEIPT_SCHEMA,
            phoenix_diagram::VIEWER_REVISION
        );
        return ExitCode::SUCCESS;
    }
    print(dispatch(&args).unwrap_or_else(|e| error(command, "arguments/invalid", e, 2)))
}
fn print((value, exit): (Value, u8)) -> ExitCode {
    if writeln!(io::stdout().lock(), "{value}").is_err() {
        return ExitCode::from(4);
    }
    ExitCode::from(exit)
}
