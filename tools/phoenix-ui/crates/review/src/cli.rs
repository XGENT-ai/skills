//! `phoenix-ui review <verb>` entry. With `--json`, stdout carries exactly one JSON
//! object and diagnostics go to stderr. Exit 0 means the operation succeeded or a
//! valid state was returned; the UI verdict is a field, never the exit code.
//! 2 = argument/schema/path input, 3 = revision/lock/history conflict,
//! 4 = file/dependency/environment failure, including `status` on corrupt,
//! future-schema or drifted managed state (it still prints the unknown object).
use crate::{
    capture_evidence::{CaptureInput, TestedVersion, WorkingTree},
    protocol::{self, applied, finalize, judgement, packet, BeginOptions, Round},
    storage::{error, StoreError},
    Decimal,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

type Outcome = Result<Value, StoreError>;

mod schema;

const HELP: &str = "Usage: phoenix-ui review <command> [flags] [--json]

Commands and flags:
  begin --prefix <prefix> --target <target.md>
        [--plan <plan.md>] [--scope <unit IDs>] [--threshold <decimal>]
        [--tested-version <tested-version.json>]
  capture add --round <round> --input <capture.json> --expect-revision <revision>
  judge-pack --round <round> [--units <unit IDs>]
  judge add --round <round> --input <judge.json> --raw <reviewer.txt>
        --packet <packet hash or manifest path> --expect-revision <revision>
        [--mappings <mappings.json>]
  finalize --round <round> --input <adjudication.json>
        --expect-revision <revision>
        (append a whole-UI waiver: use --expect-cycle-revision <revision> instead)
  apply record --round <round> --input <applied.json> --expect-revision <revision>
  status --prefix <prefix>
  import --prefix <prefix> --input <legacy-map-or-rebase.json>
  schema --json [--kind <kind>]
  help | --help | -h

Input discovery kinds: tested-version, capture, judge, mappings, adjudication,
runtime-check, applied, import. schema returns versioned typed examples and enum
variants, not JSON Schema or real evidence. Replace all example identities,
paths, hashes, versions and revisions with actual operation/evidence values.

Help and schema are read-only and do not open a cycle. Lists use comma-separated
IDs; --round accepts a round directory or its round.json. --json writes exactly
one object to stdout; diagnostics go to stderr. Exit 0 means a successful query
or operation, never UI acceptance. Errors: 2 input, 3 conflict, 4 environment or
unknown managed state. Finalization may return confirmation_required and remain
a draft. Use the fixed package's matching engine before querying or writing.";

pub fn run(args: &[String], cwd: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let json_mode = args.iter().any(|arg| arg == "--json");
    let (command, result) = dispatch(args, cwd);
    match result {
        Ok(mut value) => {
            // Unknown managed state is reported in full but is not a valid state.
            let unknown = command == "status" && value["state"] == "unknown";
            value["ok"] = json!(!unknown);
            value["command"] = json!(command);
            if unknown {
                value["exitCode"] = json!(4);
                let _ = writeln!(
                    stderr,
                    "phoenix-ui review status: managed state is unknown: {}",
                    value["problem"]["message"].as_str().unwrap_or_default()
                );
            }
            if json_mode {
                let _ = writeln!(stdout, "{value}");
            } else if command == "help" {
                let _ = writeln!(stdout, "{HELP}");
            } else {
                let status = if unknown { "unknown" } else { "ok" };
                let _ = writeln!(stdout, "phoenix-ui review {command}: {status}");
                for (key, field) in value.as_object().into_iter().flatten() {
                    if let Some(text) = field.as_str() {
                        let _ = writeln!(stdout, "{key}: {text}");
                    } else if field.is_number() || field.is_boolean() {
                        let _ = writeln!(stdout, "{key}: {field}");
                    }
                }
            }
            if unknown {
                4
            } else {
                0
            }
        }
        Err(failure) => {
            let code = if matches!(failure.code, 2..=4) {
                failure.code
            } else {
                4
            };
            let _ = writeln!(stderr, "phoenix-ui review {command}: {}", failure.message);
            if json_mode {
                let _ = writeln!(
                    stdout,
                    "{}",
                    json!({"ok": false, "command": command, "exitCode": code, "error": failure.message})
                );
            }
            i32::from(code)
        }
    }
}

fn dispatch(args: &[String], cwd: &Path) -> (String, Outcome) {
    let word = |n: usize| args.get(n).map(String::as_str);
    let (command, rest) =
        match (word(0), word(1)) {
            (Some("help" | "--help" | "-h"), _) => ("help", &args[1..]),
            (Some("capture"), Some("add")) => ("capture add", &args[2..]),
            (Some("judge"), Some("add")) => ("judge add", &args[2..]),
            (Some("apply"), Some("record")) => ("apply record", &args[2..]),
            (
                Some(verb @ ("begin" | "judge-pack" | "finalize" | "status" | "import" | "schema")),
                _,
            ) => (verb, &args[1..]),
            _ => return (
                args.first().cloned().unwrap_or_default(),
                Err(error(
                    2,
                    "Unknown review command; use phoenix-ui review --help for supported commands",
                )),
            ),
        };
    let result = match command {
        "help" => Flags::parse(rest, &[])
            .map(|_| json!({"reviewSchema": 1, "readOnly": true, "help": HELP})),
        "schema" => {
            Flags::parse(rest, &["kind"]).and_then(|flags| schema::query(flags.get("kind")))
        }
        "begin" => begin(cwd, rest),
        "capture add" => capture_add(cwd, rest),
        "judge-pack" => judge_pack(cwd, rest),
        "judge add" => judge_add(cwd, rest),
        "finalize" => finalize_round(cwd, rest),
        "apply record" => apply_record(cwd, rest),
        "status" => status(cwd, rest),
        _ => import(cwd, rest),
    };
    (command.to_owned(), result)
}

struct Flags(BTreeMap<String, String>);

impl Flags {
    fn parse(args: &[String], allowed: &[&str]) -> Result<Self, StoreError> {
        let mut values = BTreeMap::new();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if arg == "--json" {
                continue;
            }
            let name = arg
                .strip_prefix("--")
                .filter(|name| allowed.contains(name))
                .ok_or_else(|| error(2, format!("Unexpected argument {arg}")))?;
            let value = iter
                .next()
                .filter(|value| !value.starts_with("--"))
                .ok_or_else(|| error(2, format!("--{name} needs a value")))?;
            if values.insert(name.to_owned(), value.clone()).is_some() {
                return Err(error(2, format!("--{name} was given more than once")));
            }
        }
        Ok(Self(values))
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    fn required(&self, name: &str) -> Result<&str, StoreError> {
        self.get(name)
            .ok_or_else(|| error(2, format!("--{name} is required")))
    }

    fn revision(&self) -> Result<u64, StoreError> {
        self.required("expect-revision")?
            .parse()
            .map_err(|_| error(2, "--expect-revision must be a non-negative integer"))
    }

    fn list(&self, name: &str) -> Result<Option<Vec<String>>, StoreError> {
        self.get(name)
            .map(|value| {
                let items: Vec<String> = value.split(',').map(|s| s.trim().to_owned()).collect();
                if items.iter().any(String::is_empty) {
                    return Err(error(2, format!("--{name} must list comma-separated IDs")));
                }
                Ok(items)
            })
            .transpose()
    }
}

fn file_bytes(cwd: &Path, value: &str) -> Result<Vec<u8>, StoreError> {
    std::fs::read(cwd.join(value)).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => error(2, format!("Input file {value} does not exist")),
        _ => error(4, format!("Cannot read {value}: {e}")),
    })
}

fn input<T: DeserializeOwned>(cwd: &Path, value: &str) -> Result<T, StoreError> {
    serde_json::from_slice(&file_bytes(cwd, value)?)
        .map_err(|e| error(2, format!("{value} does not match the review schema: {e}")))
}

fn round_path(value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.file_name().and_then(|name| name.to_str()) == Some("round.json") {
        path
    } else {
        path.join("round.json")
    }
}

fn to_value(value: &impl serde::Serialize) -> Result<Value, StoreError> {
    serde_json::to_value(value).map_err(|e| error(4, e))
}

fn read_round(cwd: &Path, path: &Path) -> Result<Round, StoreError> {
    serde_json::from_slice(&file_bytes(cwd, &path.to_string_lossy())?).map_err(|e| error(4, e))
}

fn begin(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(
        args,
        &[
            "prefix",
            "target",
            "plan",
            "scope",
            "threshold",
            "tested-version",
        ],
    )?;
    let threshold = flags
        .get("threshold")
        .map(|value| {
            value
                .parse::<Decimal>()
                .map_err(|e| error(2, format!("--threshold: {e}")))
        })
        .transpose()?;
    let version = match flags.get("tested-version") {
        Some(path) => input::<TestedVersion>(cwd, path)?,
        None => TestedVersion {
            head: None,
            working_tree: WorkingTree::Unknown,
            build_id: None,
        },
    };
    let round = protocol::begin(
        cwd,
        BeginOptions {
            prefix: flags.required("prefix")?.to_owned(),
            target: flags.required("target")?.to_owned(),
            plan: flags.get("plan").map(str::to_owned),
            scope: flags.list("scope")?,
            threshold,
            version,
        },
    )?;
    let path = protocol::round_path(&round.prefix, round.round_id[1..].parse().unwrap());
    let directory = path.parent().unwrap();
    let cells: Vec<Value> = round
        .matrix
        .cells
        .iter()
        .map(|cell| {
            json!({"cell": cell.key, "viewport": cell.viewport,
                "path": directory.join(&cell.filename).to_string_lossy()})
        })
        .collect();
    Ok(json!({
        "roundId": round.round_id, "round": path.to_string_lossy(), "revision": round.revision,
        "baseline": round.baseline,
        "baselineStarted": round.baseline.starts_with(&format!("{}:", round.round_id)),
        "targetFingerprint": round.target_fingerprint, "full": round.full, "scope": round.scope,
        "threshold": round.threshold, "testedVersion": round.tested_version,
        "testedVersionKnown": round.tested_version.known(), "cells": cells,
        "targetMissing": round.target_snapshot.missing,
    }))
}

fn capture_add(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(args, &["round", "input", "expect-revision"])?;
    let path = round_path(flags.required("round")?);
    let expected = flags.revision()?;
    let capture: CaptureInput = input(cwd, flags.required("input")?)?;
    let round = protocol::add_capture(cwd, &path, expected, capture)?;
    Ok(json!({
        "roundId": round.round_id, "round": path.to_string_lossy(), "revision": round.revision,
        "changed": round.revision != expected, "missingCells": protocol::missing_cells(&round),
        "captureList": path.parent().unwrap().join("capture.md").to_string_lossy(),
    }))
}

fn judge_pack(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(args, &["round", "units"])?;
    let path = round_path(flags.required("round")?);
    let reference = packet::create_derived(cwd, &path, flags.list("units")?)?;
    let manifest: packet::Manifest = input(cwd, &reference.manifest.path)?;
    let directory = Path::new(&reference.manifest.path).parent().unwrap();
    let round = read_round(cwd, &path)?;
    Ok(json!({
        "roundId": round.round_id, "revision": round.revision, "packetHash": reference.packet_hash,
        "manifest": reference.manifest.path, "directory": directory.to_string_lossy(),
        "absoluteDirectory": cwd.join(directory).to_string_lossy(), "units": manifest.units,
        "unreviewedUnits": manifest.unreviewed_units,
    }))
}

fn judge_add(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(
        args,
        &[
            "round",
            "input",
            "raw",
            "packet",
            "expect-revision",
            "mappings",
        ],
    )?;
    let path = round_path(flags.required("round")?);
    let expected = flags.revision()?;
    let judge: judgement::JudgeInput = input(cwd, flags.required("input")?)?;
    let packet = flags.required("packet")?;
    let packet = packet
        .strip_suffix("/manifest.json")
        .and_then(|dir| Path::new(dir).file_name())
        .and_then(|name| name.to_str())
        .unwrap_or(packet);
    if packet != judge.packet_hash {
        return Err(error(
            2,
            "--packet does not match the judge input packetHash",
        ));
    }
    let raw = file_bytes(cwd, flags.required("raw")?)?;
    let mappings = match flags.get("mappings") {
        Some(file) => input(cwd, file)?,
        None => Vec::new(),
    };
    let receipt = judgement::add(cwd, &path, expected, judge, &raw, mappings)?;
    let round = read_round(cwd, &path)?;
    Ok(json!({
        "roundId": round.round_id, "revision": round.revision,
        "changed": round.revision != expected, "record": receipt.file,
        "assignments": receipt.assignments,
    }))
}

fn finalize_round(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(
        args,
        &["round", "input", "expect-revision", "expect-cycle-revision"],
    )?;
    let path = round_path(flags.required("round")?);
    let adjudication: finalize::Adjudication = input(cwd, flags.required("input")?)?;
    // A draft round compares its own revision; appending a whole-UI waiver to the
    // latest sealed round compares the cycle revision.
    let (expected, response) = match flags.get("expect-cycle-revision") {
        Some(value) => {
            if flags.get("expect-revision").is_some() {
                return Err(error(
                    2,
                    "Pass either --expect-revision or --expect-cycle-revision, not both",
                ));
            }
            let revision = value
                .parse()
                .map_err(|_| error(2, "--expect-cycle-revision must be a non-negative integer"))?;
            (
                revision,
                finalize::append_waiver(cwd, &path, revision, adjudication)?,
            )
        }
        None => {
            let revision = flags.revision()?;
            (revision, finalize::run(cwd, &path, revision, adjudication)?)
        }
    };
    let (mut value, outcome) = match response {
        finalize::Response::ConfirmationRequired { outcome } => (
            json!({"status": "confirmation_required", "revision": expected}),
            outcome,
        ),
        finalize::Response::Finalized {
            round_id,
            revision,
            result,
            report,
            outcome,
        } => (
            json!({"status": "finalized", "roundId": round_id, "revision": revision,
                "result": result, "report": report}),
            outcome,
        ),
        finalize::Response::Waived {
            round_id,
            revision,
            outcome,
            waivers,
        } => (
            json!({"status": "waived", "roundId": round_id, "cycleRevision": revision,
                "revision": revision, "waivers": waivers}),
            outcome,
        ),
    };
    for (key, field) in to_value(&outcome)?.as_object().into_iter().flatten() {
        value[key] = field.clone();
    }
    Ok(value)
}

fn apply_record(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(args, &["round", "input", "expect-revision"])?;
    let path = round_path(flags.required("round")?);
    let expected = flags.revision()?;
    let record: applied::Input = input(cwd, flags.required("input")?)?;
    to_value(&applied::record(cwd, &path, expected, record)?)
}

fn status(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(args, &["prefix"])?;
    protocol::status::inspect(cwd, flags.required("prefix")?)
}

fn import(cwd: &Path, args: &[String]) -> Outcome {
    let flags = Flags::parse(args, &["prefix", "input"])?;
    let input = flags.required("input")?;
    let bytes = file_bytes(cwd, input)?;
    protocol::legacy::run(cwd, flags.required("prefix")?, input, &bytes)
}
