//! Public `review` CLI regressions. Generated 2×2 PNGs and declared provenance are
//! structural fixtures: they prove bindings and state transitions, not UI truth.
use phoenix_ui_review::{
    capture_evidence::{CaptureInput, TestedVersion},
    cli,
    protocol::{applied, finalize, judgement, legacy},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const PREFIX: &str = "docs/orders";
const TARGET: &str = "docs/orders.ui-target.md";
const RAW: &str = "Independent reviewer raw conclusion (structural fixture).\n";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn png(shade: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[shade; 16]).unwrap();
    }
    bytes
}

fn target_text(units: &[&str], rule: &str, steps: &str) -> String {
    let rows: String = units
        .iter()
        .map(|unit| format!("| {unit} Screen | default | 2×2 | light | [Design](../DESIGN.md) |\n"))
        .collect();
    format!("# Orders\n\n## 单元\n\n| 单元 | 必需状态 | 视口 | 主题 | 来源 |\n| --- | --- | --- | --- | --- |\n{rows}\n## 规范要点\n\n{rule} ![Target](orders.ui-target/orders.png)\n\n## 采集\n\n{steps}\n\n## 变更记录\n\n- 2026-10-10 R1 established.\n")
}

struct Project {
    root: PathBuf,
    units: Vec<&'static str>,
}

impl Project {
    fn new(units: &[&'static str]) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-review-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("docs/orders.ui-target")).unwrap();
        fs::create_dir_all(root.join("inputs")).unwrap();
        fs::write(root.join("DESIGN.md"), "Accepted icon scale 24px.").unwrap();
        fs::write(root.join("docs/orders.ui-target/orders.png"), png(10)).unwrap();
        let project = Self {
            root,
            units: units.to_vec(),
        };
        project.set_target("Accepted icon scale 24px.", "Open the real app.");
        project
    }

    fn set_target(&self, rule: &str, steps: &str) {
        fs::write(
            self.root.join(TARGET),
            target_text(&self.units, rule, steps),
        )
        .unwrap();
    }

    /// Runs the public entry with `--json` and requires exactly one JSON object on stdout.
    fn run(&self, args: &[&str]) -> (i32, Value) {
        let mut argv: Vec<String> = args.iter().map(|s| (*s).to_owned()).collect();
        argv.push("--json".into());
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = cli::run(&argv, &self.root, &mut out, &mut err);
        let text = String::from_utf8(out).unwrap();
        let mut stream = serde_json::Deserializer::from_str(&text).into_iter::<Value>();
        let value = stream
            .next()
            .unwrap_or_else(|| {
                panic!(
                    "no JSON on stdout for {args:?}; stderr: {}",
                    String::from_utf8_lossy(&err)
                )
            })
            .unwrap();
        assert!(stream.next().is_none(), "more than one JSON value: {text}");
        assert!(value.is_object(), "stdout must be one object: {text}");
        assert_eq!(value["ok"], code == 0, "{text}");
        if code != 0 {
            assert_eq!(value["exitCode"], code, "{text}");
            assert!(!err.is_empty(), "diagnostics belong on stderr");
        }
        (code, value)
    }

    fn ok(&self, args: &[&str]) -> Value {
        let (code, value) = self.run(args);
        assert_eq!(code, 0, "{args:?} -> {value}");
        value
    }

    fn write(&self, name: &str, value: &Value) -> String {
        let path = format!("inputs/{name}.json");
        fs::write(
            self.root.join(&path),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
        path
    }

    fn read(&self, path: &str) -> Value {
        serde_json::from_slice(&fs::read(self.root.join(path)).unwrap()).unwrap()
    }

    fn version() -> Value {
        json!({"head": null, "workingTree": {"state": "unknown"}, "buildId": "cli-structural-fixture"})
    }

    fn begin_with(&self, extra: &[&str]) -> (i32, Value) {
        let version = self.write("version", &Self::version());
        let mut args = vec![
            "begin",
            "--prefix",
            PREFIX,
            "--target",
            TARGET,
            "--tested-version",
            &version,
        ];
        args.extend_from_slice(extra);
        self.run(&args)
    }

    fn begin(&self, extra: &[&str]) -> String {
        let (code, value) = self.begin_with(extra);
        assert_eq!(code, 0, "{value}");
        value["round"].as_str().unwrap().to_owned()
    }

    fn round(&self, round: &str) -> Value {
        self.read(round)
    }

    fn rev(&self, round: &str) -> String {
        self.round(round)["revision"].to_string()
    }

    fn capture_input(&self, round: &str, unit: &str, attempt: u32, shot: Shot) -> Value {
        let state = self.round(round);
        let cell = state["matrix"]["cells"]
            .as_array()
            .unwrap()
            .iter()
            .find(|cell| cell["key"]["unitId"] == unit)
            .unwrap()
            .clone();
        let filename = cell["filename"].as_str().unwrap();
        let name = if attempt == 1 {
            filename.to_owned()
        } else {
            format!("{}-a{attempt}.png", filename.strip_suffix(".png").unwrap())
        };
        let image = Path::new(round).parent().unwrap().join(name);
        let observation = match shot {
            Shot::Missing(reason) => json!({"status": "missing", "reason": reason}),
            Shot::Image(shade) | Shot::Unverified(shade) => {
                let bytes = png(shade);
                fs::write(self.root.join(&image), &bytes).unwrap();
                json!({"status": "captured",
                    "image": {"path": image.to_string_lossy(), "sha256": hash(&bytes)},
                    "origin": "browser", "tool": "structural fixture, not a product capture",
                    "address": "http://localhost:8080/orders?session=private",
                    "takenAt": "2026-10-10T04:00:00Z", "viewport": cell["viewport"],
                    "testedVersion": state["testedVersion"], "interactions": [],
                    "humanCheck": {"opened": true, "currentProduct": !matches!(shot, Shot::Unverified(_)),
                        "currentVersion": true, "notes": "Declared per-image check of a generated fixture"}})
            }
        };
        json!({"schemaVersion": 1, "roundId": state["roundId"], "cell": cell["key"],
            "attempt": attempt, "observation": observation})
    }

    fn capture_at(
        &self,
        round: &str,
        unit: &str,
        attempt: u32,
        shot: Shot,
        revision: &str,
    ) -> (i32, Value) {
        let input = self.capture_input(round, unit, attempt, shot);
        let path = self.write(&format!("capture-{unit}-{attempt}"), &input);
        self.run(&[
            "capture",
            "add",
            "--round",
            round,
            "--input",
            &path,
            "--expect-revision",
            revision,
        ])
    }

    fn capture(&self, round: &str, unit: &str, attempt: u32, shot: Shot) -> (i32, Value) {
        let revision = self.rev(round);
        self.capture_at(round, unit, attempt, shot, &revision)
    }

    fn capture_all(&self, round: &str) {
        let state = self.round(round);
        let units: Vec<String> = match state["scope"].as_array() {
            Some(scope) => scope
                .iter()
                .map(|u| u.as_str().unwrap().to_owned())
                .collect(),
            None => self.units.iter().map(|u| (*u).to_owned()).collect(),
        };
        for unit in units {
            let (code, value) = self.capture(round, &unit, 1, Shot::Image(255));
            assert_eq!(code, 0, "{value}");
        }
    }

    fn pack(&self, round: &str, units: Option<&str>) -> (Value, Value) {
        let mut args = vec!["judge-pack", "--round", round];
        if let Some(units) = units {
            args.extend(["--units", units]);
        }
        let packet = self.ok(&args);
        let manifest = self.read(packet["manifest"].as_str().unwrap());
        (packet, manifest)
    }

    /// Builds one reviewer's structured input. Each listed gap uses the unit's current image
    /// and the frozen expectation basis; unlisted assigned previous gaps default to 仍存在.
    fn judge_input(&self, manifest: &Value, packet: &Value, reviewer: &str, spec: &Judge) -> Value {
        let current = |unit: &str| {
            manifest["captures"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["current"] == true && c["input"]["cell"]["unitId"] == unit)
                .unwrap()
                .clone()
        };
        let basis = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["path"] == "expectations.txt")
            .unwrap()
            .clone();
        let mut next = 0;
        let mut units = Vec::new();
        for unit in manifest["units"].as_array().unwrap() {
            let unit = unit.as_str().unwrap();
            let (scores, gaps) = spec
                .units
                .iter()
                .find(|(name, _, _)| *name == unit)
                .map(|(_, scores, gaps)| (*scores, gaps.clone()))
                .unwrap_or((["3", "3", "3", "1"], vec![]));
            let capture = current(unit);
            let gaps: Vec<Value> = gaps
                .iter()
                .map(|(key, severity, recoverable)| {
                    let key = if key.is_empty() {
                        next += 1;
                        format!("新-{next}")
                    } else {
                        (*key).to_owned()
                    };
                    let mut gap = json!({"key": key, "cell": capture["input"]["cell"],
                        "image": capture["input"]["observation"]["image"], "dimension": "视觉",
                        "location": "Header icon", "current": "Icon is 32px", "expected": "Icon is 24px",
                        "basis": basis, "basisLocation": "Accepted icon scale", "fix": "Use the 24px icon token",
                        "severity": severity, "recoverable": recoverable});
                    if let Some(summary) = spec.summary {
                        gap["summary"] = summary.into();
                    }
                    gap
                })
                .collect();
            units.push(json!({"unitId": unit, "scores": scores,
                "reasons": ["Layout reason", "Visual reason", "State reason", "Detail reason"],
                "gaps": gaps, "suggestions": [], "unseenCells": []}));
        }
        let previous: Vec<Value> = manifest["previousUnits"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|unit| {
                let capture = current(unit["unitId"].as_str().unwrap());
                unit["gaps"].as_array().unwrap().iter().map(move |gap| {
                    let id = gap["id"].as_str().unwrap();
                    let status = spec
                        .previous
                        .iter()
                        .find(|(old, _)| *old == id)
                        .map_or("仍存在", |(_, status)| *status);
                    json!({"id": id, "status": status, "basis": {"cell": capture["input"]["cell"],
                        "image": capture["input"]["observation"]["image"],
                        "notes": "Current image reassessed against the previous conclusion"}})
                })
            })
            .collect();
        json!({"schemaVersion": 1, "cycleId": manifest["cycleId"], "roundId": manifest["roundId"],
            "packetHash": packet["packetHash"], "reviewerId": reviewer, "rawSha256": hash(RAW.as_bytes()),
            "provenance": {"freshContext": true, "readOnly": true, "inheritedModel": true,
                "openedImages": manifest["files"].as_array().unwrap().iter()
                    .filter(|f| f["path"].as_str().unwrap().ends_with(".png")).collect::<Vec<_>>(),
                "notes": "Declared fixture provenance; not a visual acceptance"},
            "conversion": {"kind": "reviewerStructured"}, "units": units, "unreviewedUnits": [],
            "previous": previous, "summary": "Lowest unit and main gaps"})
    }

    fn judge(&self, round: &str, reviewer: &str, spec: &Judge) -> (i32, Value) {
        let (packet, manifest) = self.pack(round, spec.packet_units);
        let input = self.judge_input(&manifest, &packet, reviewer, spec);
        let path = self.write(&format!("judge-{reviewer}"), &input);
        fs::write(self.root.join("inputs/raw.md"), RAW).unwrap();
        let revision = self.rev(round);
        let mut args = vec![
            "judge",
            "add",
            "--round",
            round,
            "--input",
            &path,
            "--raw",
            "inputs/raw.md",
            "--packet",
            packet["packetHash"].as_str().unwrap(),
            "--expect-revision",
            &revision,
        ];
        let mappings;
        if !spec.mappings.is_empty() {
            mappings = self.write(&format!("mappings-{reviewer}"), &json!(spec.mappings));
            args.extend(["--mappings", &mappings]);
        }
        self.run(&args)
    }

    fn adjudication(&self, round: &str, extra: Value) -> Value {
        let state = self.round(round);
        let mut value = json!({"schemaVersion": 1, "cycleId": state["cycleId"], "roundId": state["roundId"],
            "machineChecks": [], "mainChecks": [], "decisions": [], "applied": [],
            "notes": "Host observation notes for a structural fixture"});
        for (key, field) in extra.as_object().unwrap() {
            value[key] = field.clone();
        }
        value
    }

    fn finalize_at(&self, round: &str, extra: Value, revision: &str) -> (i32, Value) {
        let input = self.adjudication(round, extra);
        let path = self.write("adjudication", &input);
        self.run(&[
            "finalize",
            "--round",
            round,
            "--input",
            &path,
            "--expect-revision",
            revision,
        ])
    }

    fn finalize(&self, round: &str, extra: Value) -> (i32, Value) {
        let revision = self.rev(round);
        self.finalize_at(round, extra, &revision)
    }

    fn applied_revision(&self) -> String {
        let path = self.root.join(format!("{PREFIX}.ui-review.applied.json"));
        if path.exists() {
            self.read(&format!("{PREFIX}.ui-review.applied.json"))["revision"].to_string()
        } else {
            "0".into()
        }
    }

    /// Records dispositions; `Modified` items change a product file with its real hash.
    fn apply(&self, round: &str, items: &[(&str, &str)], refactors: Value) -> (i32, Value) {
        let state = self.round(round);
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let items: Vec<Value> = items
            .iter()
            .map(|(id, disposition)| {
                let modified = *disposition == "已修改待评审";
                let changes = if modified {
                    let body = format!("// change {}\n", NEXT.fetch_add(1, Ordering::Relaxed));
                    fs::write(self.root.join("card.tsx"), &body).unwrap();
                    json!([{"path": "card.tsx", "sha256": hash(body.as_bytes())}])
                } else {
                    json!([])
                };
                let basis = if *disposition == "不采纳" {
                    json!([state["targetSnapshot"]["expectations"]])
                } else {
                    json!([])
                };
                json!({"issueId": id, "verification": "Observed on the running page fixture",
                    "disposition": disposition, "changes": changes, "basis": basis,
                    "selfCheck": {"kind": "not_run", "reason": "Structural fixture has no running app"},
                    "notes": "Concrete repair notes"})
            })
            .collect();
        let input = json!({"schemaVersion": 1, "cycleId": state["cycleId"], "roundId": state["roundId"],
            "recordPath": null, "at": "2026-10-10T06:00:00Z",
            "testedVersion": {"head": null, "workingTree": {"state": "unknown"}, "buildId": "fix-build"},
            "items": items, "restructureScopes": refactors, "decisions": [], "checks": [], "unfinished": []});
        let path = self.write("applied", &input);
        let revision = self.applied_revision();
        self.run(&[
            "apply",
            "record",
            "--round",
            round,
            "--input",
            &path,
            "--expect-revision",
            &revision,
        ])
    }

    fn status(&self) -> Value {
        self.ok(&["status", "--prefix", PREFIX])
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Clone, Copy)]
enum Shot {
    Image(u8),
    Unverified(u8),
    Missing(&'static str),
}

type GapSpec = (&'static str, &'static str, &'static str);

#[derive(Default)]
struct Judge {
    units: Vec<(&'static str, [&'static str; 4], Vec<GapSpec>)>,
    previous: Vec<(&'static str, &'static str)>,
    mappings: Vec<Value>,
    packet_units: Option<&'static str>,
    summary: Option<&'static str>,
}

fn unit(name: &'static str, scores: [&'static str; 4], gaps: &[GapSpec]) -> Judge {
    Judge {
        units: vec![(name, scores, gaps.to_vec())],
        ..Judge::default()
    }
}

impl Judge {
    fn and(mut self, name: &'static str, scores: [&'static str; 4], gaps: &[GapSpec]) -> Self {
        self.units.push((name, scores, gaps.to_vec()));
        self
    }
    fn prev(mut self, id: &'static str, status: &'static str) -> Self {
        self.previous.push((id, status));
        self
    }
    fn titled(mut self, summary: &'static str) -> Self {
        self.summary = Some(summary);
        self
    }
}

/// A complete reviewed round through the public CLI; returns (round path, finalize output).
fn reviewed(p: &Project, scope: Option<&str>, judge: &Judge) -> (String, Value) {
    let round = match scope {
        Some(scope) => p.begin(&["--scope", scope]),
        None => p.begin(&[]),
    };
    p.capture_all(&round);
    let (code, value) = p.judge(&round, "reviewer-a", judge);
    assert_eq!(code, 0, "{value}");
    let (code, value) = p.finalize(&round, json!({}));
    assert_eq!(code, 0, "{value}");
    (round, value)
}

// ── V-7: arguments, identity, CAS, recovery and projections ────────────────

#[test]
fn discovery_help_lists_the_public_verbs_and_input_discovery_path() {
    let p = Project::new(&["U1"]);
    for verb in ["--help", "-h", "help"] {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(cli::run(&[verb.into()], &p.root, &mut out, &mut err), 0);
        assert!(err.is_empty());
        let text = String::from_utf8(out).unwrap();
        for entry in [
            "begin",
            "capture add",
            "judge-pack",
            "judge add",
            "finalize",
            "apply record",
            "status",
            "import",
            "schema --json",
            "--tested-version",
            "--mappings",
            "--expect-cycle-revision",
            "--kind",
        ] {
            assert!(text.contains(entry), "missing {entry} in {text}");
        }
        let help = p.ok(&[verb]);
        assert_eq!(help["command"], "help");
        assert_eq!(help["readOnly"], true);
    }
}

#[test]
fn discovery_schema_exposes_typed_input_roots_and_variants() {
    fn parses<T: serde::de::DeserializeOwned>(inputs: &Value, kind: &str) {
        for sample in inputs[kind]["examples"].as_array().unwrap() {
            serde_json::from_str::<T>(&sample["value"].to_string()).unwrap();
        }
    }
    let p = Project::new(&["U1"]);
    let schema = p.ok(&["schema"]);
    assert_eq!(schema["command"], "schema");
    assert_eq!(schema["reviewSchema"], 1);
    assert_eq!(schema["discoveryVersion"], 1);
    assert_eq!(schema["format"], "typed-examples");
    assert_eq!(schema["readOnly"], true);
    assert_eq!(schema["examplesAreEvidence"], false);
    let inputs = &schema["inputs"];
    parses::<TestedVersion>(inputs, "tested-version");
    parses::<CaptureInput>(inputs, "capture");
    parses::<judgement::JudgeInput>(inputs, "judge");
    parses::<Vec<judgement::GapMapping>>(inputs, "mappings");
    parses::<finalize::Adjudication>(inputs, "adjudication");
    for sample in inputs["adjudication"]["examples"].as_array().unwrap() {
        assert_eq!(sample["value"]["applied"], json!([]));
    }
    parses::<finalize::RuntimeCheck>(inputs, "runtime-check");
    parses::<applied::Input>(inputs, "applied");
    let imported = inputs["import"]["examples"].as_array().unwrap();
    assert_eq!(imported.len(), 2);
    serde_json::from_str::<legacy::LegacyMap>(&imported[0]["value"].to_string()).unwrap();
    serde_json::from_str::<legacy::Rebase>(&imported[1]["value"].to_string()).unwrap();
    assert_eq!(imported[0]["value"]["kind"], "legacyMarkdown");
    assert_eq!(imported[1]["value"]["kind"], "projectionRebase");
    assert_eq!(
        inputs["capture"]["examples"][0]["value"]["observation"]["status"],
        "missing"
    );
    assert_eq!(
        inputs["capture"]["examples"][1]["value"]["observation"]["status"],
        "captured"
    );
    assert_eq!(
        inputs["judge"]["examples"][0]["value"]["conversion"]["kind"],
        "reviewerStructured"
    );
    assert_eq!(
        inputs["judge"]["examples"][1]["value"]["conversion"]["kind"],
        "hostVerified"
    );
    assert_eq!(
        inputs["applied"]["examples"][0]["value"]["items"][0]["selfCheck"]["kind"],
        "not_run"
    );
    assert_eq!(
        inputs["applied"]["examples"][1]["value"]["items"][0]["selfCheck"]["kind"],
        "page"
    );
    assert_eq!(
        schema["variants"]["Origin"],
        json!(["browser", "simulator", "native"])
    );
    assert_eq!(
        schema["variants"]["Disposition"],
        json!(["已修改待评审", "不采纳", "待决定", "阻塞", "未处理"])
    );
    for kind in schema["kinds"].as_array().unwrap() {
        let kind = kind.as_str().unwrap();
        let selected = p.ok(&["schema", "--kind", kind]);
        assert_eq!(selected["inputs"].as_object().unwrap().len(), 1);
        assert_eq!(selected["inputs"][kind], inputs[kind]);
    }
}

#[test]
fn discovery_is_read_only_and_unknown_kinds_remain_input_errors() {
    let p = Project::new(&["U1"]);
    p.begin(&[]);
    fs::write(
        p.root.join(format!("{PREFIX}.ui-review.json")),
        "do not read this cycle",
    )
    .unwrap();
    let before = tree(&p.root);
    p.ok(&["--help"]);
    p.ok(&["schema"]);
    let (code, value) = p.run(&["schema", "--kind", "not-a-kind"]);
    assert_eq!(code, 2);
    assert_eq!(value["command"], "schema");
    assert!(value["error"].as_str().unwrap().contains("not-a-kind"));
    assert_eq!(
        tree(&p.root),
        before,
        "discovery must not create or change files"
    );
    let absent = p.root.join("does-not-exist");
    for args in [vec!["--help", "--json"], vec!["schema", "--json"]] {
        let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(cli::run(&args, &absent, &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&out).unwrap()["readOnly"],
            true
        );
    }
    assert!(
        !absent.exists(),
        "discovery must work without opening a project"
    );
}

#[test]
fn arguments_schema_and_paths_fail_with_exit_2_and_one_json_object() {
    let p = Project::new(&["U1"]);
    for args in [
        vec!["unknown"],
        vec!["begin", "--prefix", PREFIX],
        vec![
            "begin", "--prefix", PREFIX, "--target", TARGET, "--bogus", "x",
        ],
        vec!["begin", "--prefix", "../escape", "--target", TARGET],
        vec![
            "begin",
            "--prefix",
            PREFIX,
            "--target",
            TARGET,
            "--threshold",
            "10.5",
        ],
        vec![
            "begin",
            "--prefix",
            PREFIX,
            "--target",
            TARGET,
            "--threshold",
            "NaN",
        ],
        vec![
            "capture",
            "add",
            "--round",
            "docs/orders.ui-captures/r1/round.json",
        ],
        vec!["status"],
    ] {
        let (code, value) = p.run(&args);
        assert_eq!(code, 2, "{args:?} -> {value}");
    }
    let round = p.begin(&[]);
    let (code, _) = p.run(&[
        "capture",
        "add",
        "--round",
        &round,
        "--input",
        "inputs/missing.json",
        "--expect-revision",
        "1",
    ]);
    assert_eq!(code, 2);
    fs::write(p.root.join("inputs/broken.json"), "{\"schemaVersion\": 1,").unwrap();
    for revision in ["1", "one"] {
        let (code, _) = p.run(&[
            "capture",
            "add",
            "--round",
            &round,
            "--input",
            "inputs/broken.json",
            "--expect-revision",
            revision,
        ]);
        assert_eq!(code, 2);
    }
    assert_eq!(p.rev(&round), "1");
}

#[test]
fn begin_freezes_target_attachments_and_capture_steps_do_not_change_the_baseline() {
    let p = Project::new(&["U1"]);
    let (code, begin) = p.begin_with(&[]);
    assert_eq!(code, 0, "{begin}");
    let round = begin["round"].as_str().unwrap().to_owned();
    assert_eq!(begin["roundId"], "r1");
    assert_eq!(begin["revision"], 1);
    assert_eq!(begin["full"], true);
    assert_eq!(begin["threshold"], "8");
    assert_eq!(begin["baselineStarted"], true);
    let fingerprint = begin["targetFingerprint"].as_str().unwrap().to_owned();
    let frozen = p.round(&round)["targetSnapshot"]["attachments"].clone();
    assert!(frozen.to_string().contains("orders.ui-target/orders.png"));
    // Only the capture section changes: same fingerprint, capture still accepted.
    p.set_target(
        "Accepted icon scale 24px.",
        "Wait for fonts, then open the app.",
    );
    let (code, value) = p.capture(&round, "U1", 1, Shot::Image(255));
    assert_eq!(code, 0, "{value}");
    // Same-path target image replacement drifts the frozen target: conflict, no write.
    fs::write(p.root.join("docs/orders.ui-target/orders.png"), png(99)).unwrap();
    let before = fs::read(p.root.join(&round)).unwrap();
    let (code, _) = p.run(&["judge-pack", "--round", &round]);
    assert_eq!(code, 3);
    assert_eq!(fs::read(p.root.join(&round)).unwrap(), before);
    // A draft cannot be skipped by beginning again.
    let (code, value) = p.begin_with(&[]);
    assert_eq!(code, 3, "{value}");
    assert!(value["error"].as_str().unwrap().contains("r1"));
    assert_eq!(fingerprint.len(), 64);
}

#[test]
fn cell_key_attempt_identity_duplicates_and_stale_revisions() {
    let p = Project::new(&["U1"]);
    let round = p.begin(&[]);
    let (code, first) = p.capture(&round, "U1", 1, Shot::Image(255));
    assert_eq!(code, 0, "{first}");
    assert_eq!(first["revision"], 2);
    assert_eq!(first["changed"], true);
    let stored = fs::read(p.root.join(&round)).unwrap();
    assert!(!String::from_utf8_lossy(&stored).contains("session=private"));
    // Identical cell+attempt+content at the current revision is idempotent.
    let (code, again) = p.capture_at(&round, "U1", 1, Shot::Image(255), "2");
    assert_eq!(code, 0, "{again}");
    assert_eq!(again["changed"], false);
    assert_eq!(again["revision"], 2);
    // Same cell+attempt with different registered content is rejected, never overwritten.
    let mut changed = p.capture_input(&round, "U1", 1, Shot::Image(255));
    changed["observation"]["tool"] = "a different tool claim".into();
    let path = p.write("capture-changed", &changed);
    let (code, _) = p.run(&[
        "capture",
        "add",
        "--round",
        &round,
        "--input",
        &path,
        "--expect-revision",
        "2",
    ]);
    assert_eq!(code, 2);
    // Rewriting the registered attempt file itself is evidence drift (conflict).
    let (code, _) = p.capture_at(&round, "U1", 1, Shot::Image(7), "2");
    assert_eq!(code, 3);
    p.capture_input(&round, "U1", 1, Shot::Image(255));
    // Stale revision is a CAS conflict.
    let (code, _) = p.capture_at(&round, "U1", 2, Shot::Image(255), "1");
    assert_eq!(code, 3);
    // A capture whose host check says it is not the current product is not registrable.
    let (code, _) = p.capture_at(&round, "U1", 2, Shot::Unverified(255), "2");
    assert_eq!(code, 2);
    assert_eq!(fs::read(p.root.join(&round)).unwrap(), stored);
}

#[test]
fn missing_capture_then_recapture_appends_attempts_and_keeps_history() {
    let p = Project::new(&["U1"]);
    let round = p.begin(&[]);
    let (code, missing) = p.capture(&round, "U1", 1, Shot::Missing("App returned a login page"));
    assert_eq!(code, 0, "{missing}");
    assert_eq!(missing["missingCells"].as_array().unwrap().len(), 1);
    let (code, recaptured) = p.capture(&round, "U1", 2, Shot::Image(255));
    assert_eq!(code, 0, "{recaptured}");
    assert_eq!(recaptured["missingCells"], json!([]));
    let state = p.round(&round);
    assert_eq!(
        state["captures"][0]["input"]["observation"]["status"],
        "missing"
    );
    assert_eq!(state["captures"][1]["input"]["attempt"], 2);
    assert!(state["captures"][1]["frozenImage"]["path"]
        .as_str()
        .unwrap()
        .ends_with("-a2.png"));
    let list = fs::read_to_string(
        p.root
            .join(Path::new(&round).parent().unwrap().join("capture.md")),
    )
    .unwrap();
    assert!(list.contains("历史") && list.contains("当前"));
}

#[test]
fn every_writer_rejects_a_stale_revision_with_exit_3_and_no_writes() {
    let p = Project::new(&["U1"]);
    let round = p.begin(&[]);
    p.capture_all(&round);
    let (code, _) = p.capture_at(&round, "U1", 2, Shot::Image(255), "1");
    assert_eq!(code, 3);
    let (packet, manifest) = p.pack(&round, None);
    let input = p.judge_input(
        &manifest,
        &packet,
        "reviewer-a",
        &unit("U1", ["3", "2", "3", "1"], &[("", "P2", "1")]),
    );
    let path = p.write("judge-stale", &input);
    fs::write(p.root.join("inputs/raw.md"), RAW).unwrap();
    let before = fs::read(p.root.join(&round)).unwrap();
    let (code, _) = p.run(&[
        "judge",
        "add",
        "--round",
        &round,
        "--input",
        &path,
        "--raw",
        "inputs/raw.md",
        "--packet",
        packet["packetHash"].as_str().unwrap(),
        "--expect-revision",
        "2",
    ]);
    assert_eq!(code, 3);
    // A packet argument that differs from the judge input is an input error.
    let (code, _) = p.run(&[
        "judge",
        "add",
        "--round",
        &round,
        "--input",
        &path,
        "--raw",
        "inputs/raw.md",
        "--packet",
        &"0".repeat(64),
        "--expect-revision",
        &p.rev(&round),
    ]);
    assert_eq!(code, 2);
    assert_eq!(fs::read(p.root.join(&round)).unwrap(), before);
    let (code, value) = p.judge(
        &round,
        "reviewer-a",
        &unit("U1", ["3", "2", "3", "1"], &[("", "P2", "1")]),
    );
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["assignments"][0]["id"], "UI-01");
    let (code, _) = p.finalize_at(&round, json!({}), "1");
    assert_eq!(code, 3);
    let (code, done) = p.finalize(&round, json!({}));
    assert_eq!(code, 0, "{done}");
    assert_eq!(done["status"], "finalized");
    assert_eq!(done["verdict"], "通过验收");
    let (code, _) = p.apply(&round, &[("UI-01", "已修改待评审")], json!([]));
    assert_eq!(code, 0);
    let (code, _) = p.run(&[
        "apply",
        "record",
        "--round",
        &round,
        "--input",
        "inputs/applied.json",
        "--expect-revision",
        "0",
    ]);
    assert_eq!(code, 3);
}

#[cfg(unix)]
#[test]
fn interrupted_report_publication_is_ignored_then_recovered_by_the_same_command() {
    use std::os::unix::fs::PermissionsExt;
    let p = Project::new(&["U1"]);
    let round = p.begin(&[]);
    p.capture_all(&round);
    let (code, value) = p.judge(
        &round,
        "reviewer-a",
        &unit("U1", ["3", "2", "3", "1"], &[("", "P2", "1")]),
    );
    assert_eq!(code, 0, "{value}");
    let revision = p.rev(&round);
    let directory = p.root.join(Path::new(&round).parent().unwrap());
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o555)).unwrap();
    let (code, value) = p.finalize_at(&round, json!({}), &revision);
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, 4, "{value}");
    assert!(p
        .root
        .join(format!("{PREFIX}.ui-review.publish.json"))
        .exists());
    assert!(p.round(&round)["finalization"].is_null());
    let (code, value) = p.finalize_at(&round, json!({}), &revision);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["status"], "finalized");
    assert!(!p
        .root
        .join(format!("{PREFIX}.ui-review.publish.json"))
        .exists());
    assert_eq!(p.round(&round)["finalization"]["status"], "finalized");
    let report = fs::read_to_string(p.root.join(format!("{PREFIX}.ui-review.md"))).unwrap();
    assert!(report.contains("通过验收"));
}

#[test]
fn round_reports_keep_the_markdown_names_without_a_latest_block() {
    let p = Project::new(&["U1"]);
    let first = format!("{PREFIX}.ui-review.md");
    let (_, out) = reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
    );
    assert_eq!(out["report"]["path"], first, "{out}");
    let r1 = fs::read(p.root.join(&first)).unwrap();
    let text = String::from_utf8(r1.clone()).unwrap();
    assert!(text.starts_with("# R1 · UI 验收报告\n"), "{text}");
    assert!(!text.contains("phoenix-ui:"), "no generated latest block");
    assert!(!p.root.join(format!("{PREFIX}.ui-review.r1.md")).exists());
    let (_, out) = reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]),
    );
    let second = format!("{PREFIX}.ui-review.r2.md");
    assert_eq!(out["report"]["path"], second, "{out}");
    assert!(fs::read_to_string(p.root.join(&second))
        .unwrap()
        .starts_with("# R2 · UI 验收报告\n"));
    assert_eq!(
        fs::read(p.root.join(&first)).unwrap(),
        r1,
        "a later round never rewrites the first round's report"
    );
}

#[test]
fn generated_section_edits_conflict_while_unrelated_text_survives() {
    let p = Project::new(&["U1", "U2"]);
    let round = p.begin(&[]);
    let (code, _) = p.capture(&round, "U1", 1, Shot::Image(255));
    assert_eq!(code, 0);
    let list = p
        .root
        .join(Path::new(&round).parent().unwrap().join("capture.md"));
    let original = fs::read_to_string(&list).unwrap();
    fs::write(
        &list,
        format!("Manual preface kept by the host.\n\n{original}"),
    )
    .unwrap();
    let (code, value) = p.capture(&round, "U2", 1, Shot::Image(255));
    assert_eq!(code, 0, "{value}");
    let updated = fs::read_to_string(&list).unwrap();
    assert!(updated.starts_with("Manual preface kept by the host.\n\n"));
    assert!(updated.contains("U2"));
    let edited = updated.replace("| U1 |", "| U1 (hand edited) |");
    fs::write(&list, &edited).unwrap();
    let before = fs::read(p.root.join(&round)).unwrap();
    let (code, value) = p.capture(&round, "U1", 2, Shot::Image(255));
    assert_eq!(code, 3, "{value}");
    assert_eq!(fs::read_to_string(&list).unwrap(), edited);
    assert_eq!(fs::read(p.root.join(&round)).unwrap(), before);
}

/// Positions of `needles` in `text`, asserting each exists and they appear in order.
fn in_order(text: &str, needles: &[&str]) {
    let mut at = 0;
    for needle in needles {
        let found = text[at..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing or out of order: {needle}\n{text}"));
        at += found + needle.len();
    }
}

#[test]
fn round_projections_follow_the_markdown_contracts() {
    let p = Project::new(&["U1", "U2"]);
    let (r1, out) = reviewed(
        &p,
        None,
        &unit(
            "U1",
            ["2", "2", "2", "1"],
            &[("", "P1", "1.5"), ("", "P2", "1.5")],
        )
        .and("U2", ["3", "3", "3", "1"], &[])
        .titled("顶栏图标偏大"),
    );
    assert_eq!(out["verdict"], "需要修改", "{out}");
    let (code, value) = p.apply(
        &r1,
        &[("UI-01", "已修改待评审"), ("UI-02", "不采纳")],
        json!([]),
    );
    assert_eq!(code, 0, "{value}");
    let (r2, out) = reviewed(
        &p,
        None,
        &unit(
            "U1",
            ["2", "2", "2", "1"],
            &[("UI-01", "P1", "1.5"), ("UI-02", "P2", "1.5")],
        ),
    );
    assert_eq!(out["verdict"], "需要修改", "{out}");
    let fingerprint = p.round(&r2)["targetFingerprint"].as_str().unwrap()[..12].to_owned();
    let report = fs::read_to_string(p.root.join(format!("{PREFIX}.ui-review.r2.md"))).unwrap();
    assert!(report.starts_with("# R2 · UI 验收报告\n\n## 评审对象\n\n| 轮次 | 开发计划 | 目标包 | 被测版本 | 截图 | 上轮报告 |\n"), "{report}");
    in_order(
        &report,
        &[
            &format!("| R2 | 无 | [orders.ui-target.md](orders.ui-target.md) · {fingerprint} | HEAD 未知；构建 cli-structural-fixture；http://localhost:8080/orders | [r2](orders.ui-captures/r2/) | [orders.ui-review.md](orders.ui-review.md) |"),
            "## 验收判定\n\n判定：需要修改 · 停滞预警（UI-01 所在 U1） · 待裁定（UI-02）\n",
            "本轮分数 7（U1）；门槛 8；历史最佳 7。",
            "## 评分\n\n| 单元 | 布局 | 视觉 | 交互与状态 | 细节 | 单元分 | 达标 | 采用评审 |",
            "| U1 | 2 | 2 | 2 | 1 | 7 | 否 | reviewer-a |",
            "| U2 | 3 | 3 | 3 | 1 | 10 | 是 | reviewer-a |",
            "- 本轮分数：7（U1）；历史最佳：7；门槛：8",
            "- 布局：Layout reason",
            "## 轮次轨迹\n\n| 轮次 | 本轮分数（最低单元） | 历史最佳 | 未关闭 P0/P1 | 重构轮 | 备注 |",
            "| R1 | 7（U1） | 7 | 1 | — | 首轮 · 需要修改 |",
            "| R2 | 7（U1） | 7 | 1 | — | 需要修改 · 停滞预警（UI-01 所在 U1） · 待裁定（UI-02） |",
            "## 上轮差距复核\n\n| 编号 | 上轮严重度与摘要 | 上轮处置（取自修复记录） | 本轮结果 | 依据 |",
            "| UI-01 | P1 · 顶栏图标偏大 | 已修改待评审 | 仍存在 | 评审 reviewer-a：Current image reassessed against the previous conclusion（[U1-default-2-light.png](orders.ui-captures/r2/evidence/U1-default-2-light.png)） |",
            "| UI-02 | P2 · 顶栏图标偏大 | 不采纳 | 仍存在 |",
            "## 差距清单\n\n### UI-01 · [P1] 顶栏图标偏大（U1 · 视觉 · 可挽回 1.5）\n",
            "- 本轮结果：仍存在（沿用 R1 编号）",
            "- 位置：[U1-default-2-light.png](orders.ui-captures/r2/evidence/U1-default-2-light.png) Header icon",
            "- 现状 → 期望：Icon is 32px → Icon is 24px（依据：[目标包](orders.ui-target.md) · Accepted icon scale）",
            "- 修法：Use the 24px icon token",
            "- 来源：评审 R2（reviewer-a）",
            "### UI-02 · [P2] 顶栏图标偏大（U1 · 视觉 · 可挽回 1.5） · 待裁定\n",
            "## 待裁定事项\n\n### UI-02",
            "修复记录最近处置为不采纳，本轮评审仍判仍存在",
            "R1 → 修复：不采纳",
            "## 建议（不扣分）",
            "## 机器检查\n\n未检查",
            "## 采集与覆盖",
            "已采 2 / 2",
            "Host observation notes for a structural fixture",
        ],
    );
    for leak in ["\"workingTree\"", "Present", "session=private", "{   "] {
        assert!(
            !report.contains(leak),
            "{leak} leaked into the report\n{report}"
        );
    }
    // The first report has no previous-gap review and names no previous report.
    let first = fs::read_to_string(p.root.join(format!("{PREFIX}.ui-review.md"))).unwrap();
    assert!(first.starts_with("# R1 · UI 验收报告\n"), "{first}");
    assert!(!first.contains("## 上轮差距复核"), "{first}");
    assert!(
        first.contains("| [r1](orders.ui-captures/r1/) | 无 |"),
        "{first}"
    );
    assert!(
        first.contains(
            "### UI-01 · [P1] 顶栏图标偏大（U1 · 视觉 · 可挽回 1.5）\n\n- 本轮结果：本轮新增"
        ),
        "{first}"
    );
    // judge.md keeps only reviewer conclusions: scores, open reviewer gaps and suggestions.
    let round_dir = Path::new(&r2).parent().unwrap();
    let judge = fs::read_to_string(p.root.join(round_dir.join("judge.md"))).unwrap();
    in_order(
        &judge,
        &[
            "# R2 · 评审结论\n",
            "## U1\n\n采用评审：reviewer-a\n\n| 布局 | 视觉 | 交互与状态 | 细节 | 单元分 |\n| --- | --- | --- | --- | --- |\n| 2 | 2 | 2 | 1 | 7 |\n",
            "- 布局：Layout reason",
            "### 未关闭差距\n\n#### UI-01 · [P1] 顶栏图标偏大（U1 · 视觉 · 可挽回 1.5）\n",
            "- 本轮结论：仍存在",
            "- 位置：[U1-default-2-light.png](evidence/U1-default-2-light.png) Header icon",
            "### 建议（不扣分）\n\n无",
            "## U2",
        ],
    );
    for excluded in ["门槛", "判定", "不采纳", "Host observation", "Present"] {
        assert!(
            !judge.contains(excluded),
            "{excluded} leaked into judge.md\n{judge}"
        );
    }
    // capture.md lists readable versions, viewports and per-image provenance.
    let capture = fs::read_to_string(p.root.join(round_dir.join("capture.md"))).unwrap();
    in_order(
        &capture,
        &[
            "# R2 · 采集清单",
            "- 被测版本：HEAD 未知；构建 cli-structural-fixture",
            &format!("- 目标指纹：phoenix-ui-target-v1 · {fingerprint}"),
            "| 单元 | 状态 | 步骤 | 视口 | 主题 | attempt | 有效 | 截图或缺项 | 访问地址 | 拍摄时间 | 工具 |",
            "[U1-default-2-light.png](evidence/U1-default-2-light.png) | http://localhost:8080/orders | 2026-10-10T04:00:00Z | structural fixture, not a product capture |",
            "## 出处核对与操作记录",
            "- 主 agent 核对：Declared per-image check of a generated fixture",
        ],
    );
    assert!(capture.contains("2×2"), "{capture}");
    assert!(!capture.contains("\"workingTree\""), "{capture}");
    // The repair record names the report verdict, target fingerprint and readable version.
    let applied =
        fs::read_to_string(p.root.join(format!("{PREFIX}.ui-review.applied.md"))).unwrap();
    in_order(
        &applied,
        &[
            "### R1 → 修复 · 2026-10-10T06:00:00Z",
            &format!("- 来源：[R1 报告](orders.ui-review.md)（判定：需要修改）；目标包 [orders.ui-target.md](orders.ui-target.md) · 目标指纹 {fingerprint}"),
            "- 修复版本：HEAD 未知；构建 fix-build；改动 card.tsx",
            "- 重构轮：无",
            "| UI-01 | Observed on the running page fixture | 已修改待评审 | card.tsx | 未运行：Structural fixture has no running app |",
        ],
    );
    assert!(!applied.contains("\"workingTree\""), "{applied}");
}

// ── V-8 / §9.2: table-driven CLI sequences ─────────────────────────────────

struct Expect {
    status: &'static str,
    verdict: Option<&'static str>,
    score: Option<&'static str>,
    best: Option<&'static str>,
    warnings: &'static [&'static str],
    pending: &'static [&'static str],
}

const fn expect(
    verdict: &'static str,
    score: Option<&'static str>,
    best: Option<&'static str>,
    warnings: &'static [&'static str],
) -> Expect {
    Expect {
        status: "finalized",
        verdict: Some(verdict),
        score,
        best,
        warnings,
        pending: &[],
    }
}

enum Step {
    Round(Option<&'static str>, Judge, Expect),
    Apply(Vec<(&'static str, &'static str)>, Value),
    Retarget(&'static str),
    Check(fn(&Project, &str, &Value)),
}

fn warnings(value: &Value) -> Vec<String> {
    value["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            let scope = match w["scope"]["kind"].as_str().unwrap() {
                "global" => "global".to_owned(),
                _ => w["scope"]["gapId"].as_str().unwrap().to_owned(),
            };
            format!("{scope}:{}", w["state"].as_str().unwrap())
        })
        .collect()
}

fn play(name: &str, units: &[&'static str], steps: Vec<Step>) {
    let p = Project::new(units);
    let mut last = String::new();
    let mut output = Value::Null;
    for (n, step) in steps.into_iter().enumerate() {
        match step {
            Step::Round(scope, judge, want) => {
                let round = match scope {
                    Some(scope) => p.begin(&["--scope", scope]),
                    None => p.begin(&[]),
                };
                p.capture_all(&round);
                let (code, value) = p.judge(&round, "reviewer-a", &judge);
                assert_eq!(code, 0, "{name} step {n}: {value}");
                let (code, value) = p.finalize(&round, json!({}));
                assert_eq!(code, 0, "{name} step {n}: {value}");
                let context = format!("{name} step {n}: {value}");
                assert_eq!(value["status"], want.status, "{context}");
                assert_eq!(value["verdict"].as_str(), want.verdict, "{context}");
                assert_eq!(value["scoring"]["score"].as_str(), want.score, "{context}");
                assert_eq!(
                    value["scoring"]["historicalBest"].as_str(),
                    want.best,
                    "{context}"
                );
                assert_eq!(warnings(&value), want.warnings, "{context}");
                let pending: Vec<&str> = value["pendingDecisions"]
                    .as_array()
                    .map(|items| items.iter().map(|v| v.as_str().unwrap()).collect())
                    .unwrap_or_default();
                assert_eq!(pending, want.pending, "{context}");
                last = round;
                output = value;
            }
            Step::Apply(items, refactors) => {
                let (code, value) = p.apply(&last, &items, refactors);
                assert_eq!(code, 0, "{name} step {n}: {value}");
            }
            Step::Retarget(rule) => p.set_target(rule, "Open the real app."),
            Step::Check(check) => check(&p, &last, &output),
        }
    }
}

fn result_gap(p: &Project, round: &str, id: &str) -> Value {
    let state = p.round(round);
    let result = p.read(state["finalization"]["result"]["path"].as_str().unwrap());
    result["units"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|unit| unit["gaps"].as_array().unwrap().iter())
        .chain(result["carried"].as_array().into_iter().flatten())
        .find(|gap| gap["id"] == id)
        .unwrap_or_else(|| panic!("{id} missing from {round} result"))
        .clone()
}

const NONE: &[&str] = &[];

#[test]
fn score_sequences_follow_the_shared_contract_rows() {
    let six = || unit("U1", ["2", "2", "1", "1"], &[("", "P2", "4")]);
    let rows: Vec<(&str, Vec<&'static str>, Vec<Step>)> = vec![
        (
            "minimum-not-average",
            vec!["U1", "U2"],
            vec![Step::Round(
                None,
                unit("U1", ["3", "3", "2", "1"], &[("", "P2", "1")]).and(
                    "U2",
                    ["2", "2", "2", "1"],
                    &[("", "P2", "3")],
                ),
                expect("需要修改", Some("7"), Some("7"), NONE),
            )],
        ),
        (
            "open-upper-confirmation-boundary",
            vec!["U1"],
            vec![Step::Round(
                None,
                unit("U1", ["3", "2.5", "2", "1"], &[("", "P2", "1.5")]),
                expect("通过验收", Some("8.5"), Some("8.5"), NONE),
            )],
        ),
        (
            "exact-decimal-sum-needs-confirmation",
            vec!["U1"],
            vec![Step::Round(
                None,
                unit("U1", ["2.4", "2.3", "2.6", "0.7"], &[("", "P2", "2")]),
                Expect {
                    status: "confirmation_required",
                    verdict: None,
                    score: Some("8"),
                    best: Some("8"),
                    warnings: NONE,
                    pending: NONE,
                },
            )],
        ),
        (
            "plateau-6.0-6.5-6.8",
            vec!["U1"],
            vec![
                Step::Round(None, six(), expect("需要修改", Some("6"), Some("6"), NONE)),
                Step::Round(
                    None,
                    unit("U1", ["2", "2", "1.5", "1"], &[("", "P2", "3.5")]),
                    expect("需要修改", Some("6.5"), Some("6.5"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["2", "2", "1.8", "1"], &[("", "P2", "3.2")]),
                    expect("需要修改", Some("6.8"), Some("6.8"), &["global:停滞预警"]),
                ),
            ],
        ),
        (
            "scoped-round-does-not-advance-window",
            vec!["U1"],
            vec![
                Step::Round(None, six(), expect("需要修改", Some("6"), Some("6"), NONE)),
                Step::Round(
                    Some("U1"),
                    unit("U1", ["2", "2", "1.2", "1"], &[("", "P2", "3.8")]),
                    expect("需要修改", None, Some("6"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["2", "2", "1.5", "1"], &[("", "P2", "3.5")]),
                    expect("需要修改", Some("6.5"), Some("6.5"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["2", "2", "1.8", "1"], &[("", "P2", "3.2")]),
                    expect("需要修改", Some("6.8"), Some("6.8"), &["global:停滞预警"]),
                ),
            ],
        ),
        (
            "growth-exactly-one",
            vec!["U1"],
            vec![
                Step::Round(None, six(), expect("需要修改", Some("6"), Some("6"), NONE)),
                Step::Round(
                    None,
                    unit("U1", ["2", "2", "1.5", "1"], &[("", "P2", "3.5")]),
                    expect("需要修改", Some("6.5"), Some("6.5"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
                    expect("需要修改", Some("7"), Some("7"), NONE),
                ),
            ],
        ),
        (
            "decimal-growth-exactly-one",
            vec!["U1"],
            vec![
                Step::Round(
                    None,
                    unit("U1", ["1", "1", "0.1", "1"], &[("", "P2", "6.9")]),
                    expect("需要修改", Some("3.1"), Some("3.1"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["1", "1", "0.6", "1"], &[("", "P2", "6.4")]),
                    expect("需要修改", Some("3.6"), Some("3.6"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["1", "1", "1.1", "1"], &[("", "P2", "5.9")]),
                    expect("需要修改", Some("4.1"), Some("4.1"), NONE),
                ),
            ],
        ),
        (
            "prior-high-best-does-not-exempt",
            vec!["U1"],
            vec![
                Step::Round(
                    None,
                    unit("U1", ["3", "2.2", "2", "1"], &[("", "P1", "1.8")]),
                    expect("需要修改", Some("8.2"), Some("8.2"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["3", "1.5", "2", "1"], &[("", "P2", "2.5")]),
                    expect("需要修改", Some("7.5"), Some("8.2"), NONE),
                ),
                Step::Round(
                    None,
                    unit("U1", ["3", "1.6", "2", "1"], &[("", "P2", "2.4")]),
                    expect("需要修改", Some("7.6"), Some("8.2"), &["global:停滞预警"]),
                ),
            ],
        ),
        (
            "scoped-high-score-has-no-global-score",
            vec!["U1", "U2"],
            vec![Step::Round(
                Some("U1"),
                unit("U1", ["3", "3", "2", "1"], &[("", "P2", "1")]),
                expect("需补证", None, None, NONE),
            )],
        ),
    ];
    for (name, units, steps) in rows {
        play(name, &units, steps);
    }
}

#[test]
fn attempts_come_only_from_actual_modifications_and_refactors_turn_warnings_into_stalls() {
    let seven = || unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]);
    let first = || unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]);
    play(
        "modified-then-present-warns-then-stalls",
        &["U1"],
        vec![
            Step::Round(
                None,
                first(),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Round(
                None,
                seven(),
                expect("需要修改", Some("7"), Some("7"), &["UI-01:停滞预警"]),
            ),
            Step::Check(|p, round, _| {
                let gap = result_gap(p, round, "UI-01");
                assert_eq!(gap["history"].as_array().unwrap().len(), 2);
                assert_eq!(
                    gap["history"][1]["priorDisposition"],
                    json!({"attempted": true, "latest": "已修改待评审"})
                );
            }),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Round(
                None,
                seven(),
                expect(
                    "需要修改",
                    Some("7"),
                    Some("7"),
                    &["UI-01:停滞预警", "global:停滞预警"],
                ),
            ),
            Step::Apply(
                vec![("UI-01", "已修改待评审")],
                json!([{"scope": {"kind": "unit", "unitId": "U1"},
            "hypothesis": "Flex wrapping drives the icon size", "change": "Two-column grid", "expected": "Visual 3"}]),
            ),
            Step::Round(
                None,
                seven(),
                Expect {
                    status: "finalized",
                    verdict: Some("需用户裁定"),
                    score: Some("7"),
                    best: Some("7"),
                    warnings: &["UI-01:停滞", "global:停滞预警"],
                    pending: &["UI-01"],
                },
            ),
        ],
    );
    play(
        "p3-repeated-modification-is-not-stubborn",
        &["U1"],
        vec![
            Step::Round(
                None,
                unit("U1", ["2", "2", "2", "1"], &[("", "P3", "3")]),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Round(
                None,
                unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P3", "3")]),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Round(
                None,
                unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P3", "3")]),
                expect("需要修改", Some("7"), Some("7"), &["global:停滞预警"]),
            ),
        ],
    );
    play(
        "untreated-is-not-an-attempt",
        &["U1"],
        vec![
            Step::Round(
                None,
                first(),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "未处理")], json!([])),
            Step::Round(
                None,
                seven(),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
        ],
    );
    play(
        "two-modified-partials-warn",
        &["U1"],
        vec![
            Step::Round(
                None,
                first(),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Round(
                None,
                seven().prev("UI-01", "部分解决"),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Round(
                None,
                seven().prev("UI-01", "部分解决"),
                expect(
                    "需要修改",
                    Some("7"),
                    Some("7"),
                    &["UI-01:停滞预警", "global:停滞预警"],
                ),
            ),
        ],
    );
    play(
        "declined-but-present-stays-pending",
        &["U1"],
        vec![
            Step::Round(
                None,
                first(),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "不采纳")], json!([])),
            Step::Round(
                None,
                seven(),
                Expect {
                    status: "finalized",
                    verdict: Some("需用户裁定"),
                    score: Some("7"),
                    best: Some("7"),
                    warnings: NONE,
                    pending: &["UI-01"],
                },
            ),
            Step::Round(
                None,
                seven(),
                Expect {
                    status: "finalized",
                    verdict: Some("需用户裁定"),
                    score: Some("7"),
                    best: Some("7"),
                    warnings: &["global:停滞预警"],
                    pending: &["UI-01"],
                },
            ),
        ],
    );
}

#[test]
fn a_later_record_in_the_same_round_replaces_the_earlier_disposition() {
    // record-contract: a rerun updates that round's section, so 不采纳 replaces
    // the earlier 已修改待评审: no attempt is counted and the decision stays pending.
    play(
        "same-round-replacement",
        &["U1"],
        vec![
            Step::Round(
                None,
                unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Apply(vec![("UI-01", "不采纳")], json!([])),
            Step::Round(
                None,
                unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]),
                Expect {
                    status: "finalized",
                    verdict: Some("需用户裁定"),
                    score: Some("7"),
                    best: Some("7"),
                    warnings: NONE,
                    pending: &["UI-01"],
                },
            ),
            Step::Check(|p, round, _| {
                assert_eq!(
                    result_gap(p, round, "UI-01")["history"][1]["priorDisposition"],
                    json!({"attempted": false, "latest": "不采纳"})
                );
            }),
        ],
    );
    // Across rounds both facts hold independently: an earlier round's final
    // 已修改待评审 is an attempt, and the latest round's final 不采纳 keeps it pending.
    play(
        "attempt-and-later-decline",
        &["U1", "U2"],
        vec![
            Step::Round(
                None,
                unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]).and(
                    "U2",
                    ["3", "3", "3", "1"],
                    &[],
                ),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "已修改待评审")], json!([])),
            Step::Round(
                Some("U2"),
                unit("U2", ["3", "3", "3", "1"], &[]),
                expect("需补证", None, Some("7"), NONE),
            ),
            Step::Apply(vec![("UI-01", "不采纳")], json!([])),
            Step::Round(
                None,
                unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]).and(
                    "U2",
                    ["3", "3", "3", "1"],
                    &[],
                ),
                Expect {
                    status: "finalized",
                    verdict: Some("需用户裁定"),
                    score: Some("7"),
                    best: Some("7"),
                    warnings: &["UI-01:停滞预警"],
                    pending: &["UI-01"],
                },
            ),
        ],
    );
}

#[test]
fn reviewers_of_one_packet_keep_the_more_serious_old_gap_status() {
    // 回归 > 仍存在 > 部分解决 > 待核实 > 已解决 > 不再适用 (rules.v1.json).
    let p = Project::new(&["U1"]);
    let (r1, _) = reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
    );
    p.apply(&r1, &[("UI-01", "已修改待评审")], json!([]));
    let r2 = p.begin(&[]);
    p.capture_all(&r2);
    let partial =
        unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]).prev("UI-01", "部分解决");
    let present = unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]);
    assert_eq!(p.judge(&r2, "reviewer-a", &partial).0, 0);
    assert_eq!(p.judge(&r2, "reviewer-b", &present).0, 0);
    let (code, out) = p.finalize(&r2, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(result_gap(&p, &r2, "UI-01")["status"], "仍存在");
    assert_eq!(warnings(&out), ["UI-01:停滞预警"]);
    // A recaptured P1 judged 待核实 by one reviewer and 部分解决 by another is
    // partial, so it is repairable rather than missing evidence.
    let p = Project::new(&["U1"]);
    reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("", "P1", "3")]),
    );
    let r2 = p.begin(&[]);
    p.capture_all(&r2);
    assert_eq!(p.capture(&r2, "U1", 2, Shot::Image(254)).0, 0);
    let unseen = unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P1", "3")]).prev("UI-01", "待核实");
    let partial =
        unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P1", "3")]).prev("UI-01", "部分解决");
    assert_eq!(p.judge(&r2, "reviewer-a", &unseen).0, 0);
    assert_eq!(p.judge(&r2, "reviewer-b", &partial).0, 0);
    let (code, out) = p.finalize(&r2, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(result_gap(&p, &r2, "UI-01")["status"], "部分解决");
    assert_eq!(out["verdict"], "需要修改");
}

#[test]
fn scope_exclusion_saves_not_reviewed_without_counting_an_attempt() {
    play(
        "scope-excluded-gap",
        &["U1", "U2"],
        vec![
            Step::Round(
                None,
                unit("U1", ["3", "3", "2", "1"], &[("", "P2", "1")]).and(
                    "U2",
                    ["2", "2", "2", "1"],
                    &[("", "P2", "3")],
                ),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Round(
                Some("U1"),
                unit("U1", ["3", "3", "2", "1"], &[("UI-01", "P2", "1")]),
                expect("需补证", None, Some("7"), NONE),
            ),
            Step::Check(|p, round, _| {
                let gap = result_gap(p, round, "UI-02");
                assert_eq!(gap["status"], "本轮未复核");
                assert_eq!(gap["history"].as_array().unwrap().len(), 2);
            }),
            Step::Apply(vec![("UI-02", "未处理")], json!([])),
            Step::Round(
                None,
                unit("U1", ["3", "3", "2", "1"], &[("UI-01", "P2", "1")]).and(
                    "U2",
                    ["2", "2", "2", "1"],
                    &[("UI-02", "P2", "3")],
                ),
                expect("需要修改", Some("7"), Some("7"), NONE),
            ),
            Step::Check(|p, round, _| {
                let gap = result_gap(p, round, "UI-02");
                assert_eq!(gap["status"], "仍存在");
                assert_eq!(gap["history"].as_array().unwrap().len(), 3);
            }),
        ],
    );
}

#[test]
fn target_change_starts_a_new_baseline_and_keeps_gap_ids() {
    let p = Project::new(&["U1"]);
    let (r1, out) = reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
    );
    assert_eq!(out["scoring"]["historicalBest"], "7");
    let (code, value) = p.apply(
        &r1,
        &[("UI-01", "已修改待评审")],
        json!([{"scope": {"kind": "unit", "unitId": "U1"},
        "hypothesis": "h", "change": "c", "expected": "e"}]),
    );
    assert_eq!(code, 0, "{value}");
    let (r2, out) = reviewed(
        &p,
        None,
        &unit("U1", ["3", "2.5", "2", "1"], &[("UI-01", "P2", "1.5")]),
    );
    assert_eq!(out["scoring"]["historicalBest"], "8.5");
    // Same-path target image replacement changes the fingerprint: new baseline.
    fs::write(p.root.join("docs/orders.ui-target/orders.png"), png(77)).unwrap();
    let (code, begin) = p.begin_with(&[]);
    assert_eq!(code, 0, "{begin}");
    assert_eq!(begin["baselineStarted"], true);
    let r3 = begin["round"].as_str().unwrap().to_owned();
    p.capture_all(&r3);
    let (packet, manifest) = p.pack(&r3, None);
    let previous = &manifest["previousUnits"][0];
    assert_eq!(previous["gaps"][0]["id"], "UI-01");
    assert_eq!(previous["targetChanges"].as_array().unwrap().len(), 1);
    let spec = unit(
        "U1",
        ["2", "2", "1", "1"],
        &[("UI-01", "P2", "2"), ("", "P2", "2")],
    );
    let input = p.judge_input(&manifest, &packet, "reviewer-a", &spec);
    let path = p.write("judge-r3", &input);
    fs::write(p.root.join("inputs/raw.md"), RAW).unwrap();
    let (code, value) = p.run(&[
        "judge",
        "add",
        "--round",
        &r3,
        "--input",
        &path,
        "--raw",
        "inputs/raw.md",
        "--packet",
        packet["packetHash"].as_str().unwrap(),
        "--expect-revision",
        &p.rev(&r3),
    ]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["assignments"][1]["id"], "UI-02");
    let (code, out) = p.finalize(&r3, json!({}));
    assert_eq!(code, 0, "{out}");
    // Best and plateau window restart; the earlier 8 is not this baseline's best.
    assert_eq!(out["scoring"]["historicalBest"], "6");
    assert_eq!(warnings(&out), NONE);
    // The refactor opportunity resets with the baseline.
    let (code, value) = p.apply(
        &r3,
        &[("UI-01", "已修改待评审")],
        json!([{"scope": {"kind": "unit", "unitId": "U1"},
        "hypothesis": "h", "change": "c", "expected": "e"}]),
    );
    assert_eq!(code, 0, "{value}");
    assert_ne!(p.round(&r2)["baseline"], p.round(&r3)["baseline"]);
}

fn runtime_check(
    p: &Project,
    round: &str,
    status: &str,
    findings: Value,
    reassessed: Value,
) -> Value {
    let state = p.round(round);
    let capture = state["captures"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    let check = json!({"schemaVersion": 1, "kind": "page", "tool": "page measurement fixture",
        "takenAt": "2026-10-10T05:00:00Z", "testedVersion": state["testedVersion"],
        "cell": capture["input"]["cell"], "image": capture["frozenImage"], "status": status,
        "notes": "Measured on the running page fixture", "findings": findings, "reassessed": reassessed});
    let bytes = serde_json::to_vec_pretty(&check).unwrap();
    let name = format!("inputs/check-{}.json", hash(&bytes));
    fs::write(p.root.join(&name), &bytes).unwrap();
    json!({"path": name, "sha256": hash(&bytes)})
}

#[test]
fn host_measured_gaps_are_rechecked_by_the_host_and_never_silently_dropped() {
    let p = Project::new(&["U1"]);
    let r1 = p.begin(&[]);
    p.capture_all(&r1);
    p.judge(
        &r1,
        "reviewer-a",
        &unit("U1", ["3", "2", "3", "1"], &[("", "P2", "1")]),
    );
    let basis = p.round(&r1)["targetSnapshot"]["expectations"].clone();
    let finding = json!([{"key": "page-1", "existingIssue": null, "severity": "P1",
        "description": "Save button overlaps the table", "expected": "No overlap", "basis": basis,
        "fix": "Move the button into the toolbar"}]);
    let check = runtime_check(&p, &r1, "failed", finding, json!([]));
    let (code, out) = p.finalize(&r1, json!({"machineChecks": [check]}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需要修改");
    // Reviewers only reassess their own gaps; the host rechecks the measured one.
    let r2 = p.begin(&[]);
    p.capture_all(&r2);
    let (_, manifest) = p.pack(&r2, None);
    assert_eq!(
        manifest["previousUnits"][0]["gaps"],
        json!([{"id": "UI-01", "unitId": "U1"}])
    );
    p.judge(
        &r2,
        "reviewer-a",
        &unit("U1", ["3", "2", "3", "1"], &[("UI-01", "P2", "1")]),
    );
    let (code, out) = p.finalize(&r2, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需要修改");
    assert_eq!(result_gap(&p, &r2, "UI-02")["status"], "本轮未复核");
    let r3 = p.begin(&[]);
    p.capture_all(&r3);
    p.judge(
        &r3,
        "reviewer-a",
        &unit("U1", ["3", "2", "3", "1"], &[("UI-01", "P2", "1")]),
    );
    let check = runtime_check(
        &p,
        &r3,
        "passed",
        json!([]),
        json!([{"issueId": "UI-02", "status": "已解决"}]),
    );
    let (code, out) = p.finalize(&r3, json!({"mainChecks": [check]}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "通过验收");
    assert_eq!(result_gap(&p, &r3, "UI-02")["status"], "已解决");
    // Source-only findings stay leads and never become measured gating gaps.
    let p = Project::new(&["U1"]);
    let r1 = p.begin(&[]);
    p.capture_all(&r1);
    p.judge(
        &r1,
        "reviewer-a",
        &unit("U1", ["3", "2", "3", "1"], &[("", "P2", "1")]),
    );
    let mut check = runtime_check(
        &p,
        &r1,
        "failed",
        json!([{"key": "src-1", "existingIssue": null, "severity": "P1",
        "description": "Container width is fixed in CSS", "expected": "No overflow",
        "basis": p.round(&r1)["targetSnapshot"]["expectations"], "fix": "Measure on the page"}]),
        json!([]),
    );
    let mut body = p.read(check["path"].as_str().unwrap());
    body["kind"] = "source".into();
    let bytes = serde_json::to_vec_pretty(&body).unwrap();
    fs::write(p.root.join(check["path"].as_str().unwrap()), &bytes).unwrap();
    check["sha256"] = hash(&bytes).into();
    let (code, out) = p.finalize(&r1, json!({"machineChecks": [check]}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "通过验收");
}

#[test]
fn evidence_gaps_unknown_versions_and_wrong_pages_need_evidence() {
    // Missing required screenshot is machine-detected.
    let p = Project::new(&["U1", "U2"]);
    let round = p.begin(&[]);
    let (code, _) = p.capture(&round, "U1", 1, Shot::Image(255));
    assert_eq!(code, 0);
    let (code, _) = p.capture(&round, "U2", 1, Shot::Missing("State unreachable"));
    assert_eq!(code, 0);
    p.judge(&round, "reviewer-a", &unit("U1", ["3", "3", "3", "1"], &[]));
    let (code, out) = p.finalize(&round, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需补证");
    assert!(out["scoring"]["score"].is_null());
    // A login/error page cannot be registered as the product; it is recorded as missing.
    let p = Project::new(&["U1"]);
    let round = p.begin(&[]);
    let (code, _) = p.capture(&round, "U1", 1, Shot::Unverified(255));
    assert_eq!(code, 2);
    let (code, _) = p.capture(
        &round,
        "U1",
        1,
        Shot::Missing("Captured page was the login error page"),
    );
    assert_eq!(code, 0);
    let (code, out) = p.finalize(&round, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需补证");
    // An unknown tested version never announces acceptance of the current code.
    let p = Project::new(&["U1"]);
    let (code, begin) = p.run(&["begin", "--prefix", PREFIX, "--target", TARGET]);
    assert_eq!(code, 0, "{begin}");
    assert_eq!(begin["testedVersionKnown"], false);
    let round = begin["round"].as_str().unwrap().to_owned();
    p.capture_all(&round);
    p.judge(&round, "reviewer-a", &unit("U1", ["3", "3", "3", "1"], &[]));
    let (code, out) = p.finalize(&round, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需补证");
}

#[test]
fn confirmation_keeps_the_draft_then_the_lower_whole_review_decides() {
    let p = Project::new(&["U1"]);
    let round = p.begin(&[]);
    p.capture_all(&round);
    p.judge(
        &round,
        "reviewer-a",
        &unit("U1", ["3", "2.2", "2", "1"], &[("", "P2", "1.8")]),
    );
    let before = fs::read(p.root.join(&round)).unwrap();
    let (code, out) = p.finalize(&round, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["status"], "confirmation_required");
    assert!(out["verdict"].is_null());
    assert_eq!(fs::read(p.root.join(&round)).unwrap(), before);
    assert!(!p.root.join(format!("{PREFIX}.ui-review.md")).exists());
    let mut confirm = unit("U1", ["3", "1.8", "2", "1"], &[("", "P2", "2.2")]);
    confirm.mappings = vec![json!({"key": "新-1", "id": "UI-01", "basis": "Same icon"})];
    let (code, value) = p.judge(&round, "reviewer-b", &confirm);
    assert_eq!(code, 0, "{value}");
    let (code, out) = p.finalize(&round, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需要修改");
    assert_eq!(out["scoring"]["score"], "7.8");
    let gap = result_gap(&p, &round, "UI-01");
    assert_eq!(gap["details"].as_array().unwrap().len(), 2);
}

#[test]
fn hand_edited_reports_and_raw_judgements_are_conflicts_not_higher_scores() {
    let p = Project::new(&["U1"]);
    let (r1, _) = reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
    );
    let r2 = p.begin(&[]);
    p.capture_all(&r2);
    p.judge(
        &r2,
        "reviewer-a",
        &unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]),
    );
    // Changing a sealed round report or raw judgement is evidence drift.
    let report = p.root.join(format!("{PREFIX}.ui-review.md"));
    let sealed = fs::read_to_string(&report).unwrap();
    fs::write(&report, sealed.replace("本轮分数：7", "本轮分数：9")).unwrap();
    let before = fs::read(p.root.join(&r2)).unwrap();
    let (code, out) = p.finalize(&r2, json!({}));
    assert_eq!(code, 3, "{out}");
    assert_eq!(fs::read(p.root.join(&r2)).unwrap(), before);
    fs::write(&report, sealed).unwrap();
    let record = p.round(&r1)["judges"][0]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let raw = Path::new(&record).parent().unwrap().join("raw.md");
    fs::write(p.root.join(&raw), "edited raw").unwrap();
    let (code, _) = p.finalize(&r2, json!({}));
    assert_eq!(code, 3);
    fs::write(p.root.join(&raw), RAW).unwrap();
    let (code, out) = p.finalize(&r2, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["scoring"]["score"], "7");
}

// ── Waivers are append-only user decisions, never resolutions ──────────────

fn decision(p: &Project, round: &str, ids: &[&str], kind: &str) -> Value {
    let quote = "用户：接受当前整体界面，不再继续打磨。";
    let text = format!("Session excerpt\n{quote}\n");
    fs::write(p.root.join("inputs/session.md"), &text).unwrap();
    json!({"source": {"path": "inputs/session.md", "sha256": hash(text.as_bytes())},
        "quote": quote, "date": "2026-10-10", "testedVersion": p.round(round)["testedVersion"],
        "issueIds": ids, "impact": "Remaining icon size difference stays visible", "kind": kind})
}

fn cycle_revision(p: &Project) -> String {
    p.read(&format!("{PREFIX}.ui-review.json"))["revision"].to_string()
}

/// Appends to a sealed round; the compare-and-swap token is the cycle revision.
fn waive_at(p: &Project, round: &str, extra: Value, cycle_revision: &str) -> (i32, Value) {
    let input = p.adjudication(round, extra);
    let path = p.write("waiver", &input);
    p.run(&[
        "finalize",
        "--round",
        round,
        "--input",
        &path,
        "--expect-cycle-revision",
        cycle_revision,
    ])
}

#[test]
fn whole_ui_waiver_is_appended_and_keeps_scores_and_gaps_unresolved() {
    let p = Project::new(&["U1"]);
    let (r1, _) = reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
    );
    p.apply(&r1, &[("UI-01", "不采纳")], json!([]));
    let r2 = p.begin(&[]);
    p.capture_all(&r2);
    p.judge(
        &r2,
        "reviewer-a",
        &unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]),
    );
    let (code, out) = p.finalize(&r2, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需用户裁定");
    let sealed = fs::read(p.root.join(&r2)).unwrap();
    let result_path = p.round(&r2)["finalization"]["result"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let result = fs::read(p.root.join(&result_path)).unwrap();
    let first = p.root.join(format!("{PREFIX}.ui-review.md"));
    let first_report = fs::read(&first).unwrap();
    let report_path = format!("{PREFIX}.ui-review.r2.md");
    let report = p.root.join(&report_path);
    let sealed_report = fs::read_to_string(&report).unwrap();
    // Partial acceptance is not a whole-UI waiver.
    let p2 = Project::new(&["U1"]);
    let (q1, _) = reviewed(
        &p2,
        None,
        &unit(
            "U1",
            ["2", "2", "2", "1"],
            &[("", "P2", "1.5"), ("", "P2", "1.5")],
        ),
    );
    let partial = decision(&p2, &q1, &["UI-01"], "wholeUiWaiver");
    let (code, value) = waive_at(
        &p2,
        &q1,
        json!({"decisions": [partial]}),
        &cycle_revision(&p2),
    );
    assert_eq!(code, 2, "{value}");
    // The whole-UI waiver after sealing appends a decision; it does not rewrite history.
    let waiver = decision(&p, &r2, &["UI-01"], "wholeUiWaiver");
    let stale = cycle_revision(&p);
    // The round's --expect-revision is not the token for appending to a sealed round.
    let (code, value) = p.finalize_at(&r2, json!({"decisions": [waiver.clone()]}), &stale);
    assert_eq!(code, 2, "{value}");
    let (code, value) = waive_at(&p, &r2, json!({"decisions": [waiver.clone()]}), "999");
    assert_eq!(code, 3, "{value}");
    let mut wrong = waiver.clone();
    wrong["quote"] = "用户说可以".into();
    let (code, _) = waive_at(&p, &r2, json!({"decisions": [wrong]}), &stale);
    assert_eq!(code, 2);
    let (code, out) = waive_at(&p, &r2, json!({"decisions": [waiver.clone()]}), &stale);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["status"], "waived");
    assert_eq!(out["verdict"], "需用户裁定");
    assert_eq!(out["waivers"][0]["decision"]["quote"], waiver["quote"]);
    assert_eq!(fs::read(p.root.join(&r2)).unwrap(), sealed);
    assert_eq!(fs::read(p.root.join(&result_path)).unwrap(), result);
    assert_eq!(result_gap(&p, &r2, "UI-01")["status"], "仍存在");
    // The waiver is a generated block after the verdict inside that round's report;
    // removing the block gives back the sealed bytes, and no other report changes.
    let text = fs::read_to_string(&report).unwrap();
    let verdict = text.find("判定：需用户裁定").unwrap();
    let begin = text.find("<!-- phoenix-ui:waiver-").unwrap();
    let waived = text.find("用户豁免通过").unwrap();
    let end = text.find(":end -->\n\n").unwrap() + ":end -->\n\n".len();
    assert!(verdict < begin && begin < waived && waived < end, "{text}");
    assert!(text[end..].starts_with("## 评分\n"), "{text}");
    assert!(text.contains("接受当前整体界面"));
    assert_eq!(format!("{}{}", &text[..begin], &text[end..]), sealed_report);
    assert_eq!(fs::read(&first).unwrap(), first_report);
    // Replaying the same waiver is idempotent.
    let revision = cycle_revision(&p);
    let (code, again) = waive_at(&p, &r2, json!({"decisions": [waiver.clone()]}), &revision);
    assert_eq!(code, 0, "{again}");
    assert_eq!(cycle_revision(&p), revision);
    assert_eq!(fs::read_to_string(&report).unwrap(), text);
    // A hand-edited waiver block is a projection conflict that stops the next append;
    // a projection rebase restores it from the cycle facts after the edit is saved.
    let edited = text.replace("剩余影响：", "剩余影响（手改）：");
    fs::write(&report, &edited).unwrap();
    assert_eq!(
        read_only_status(&p)["projectionConflicts"],
        json!([report_path])
    );
    let (code, value) = waive_at(&p, &r2, json!({"decisions": [waiver.clone()]}), &revision);
    assert_eq!(code, 3, "{value}");
    assert_eq!(fs::read_to_string(&report).unwrap(), edited);
    fs::write(p.root.join("inputs/saved-report.md"), &edited).unwrap();
    let rebase = json!({"kind": "projectionRebase", "schemaVersion": 1, "prefix": PREFIX,
        "cycleRevision": revision.parse::<u64>().unwrap(), "file": report_path,
        "currentSha256": hash(edited.as_bytes()),
        "savedCopy": {"path": "inputs/saved-report.md", "sha256": hash(edited.as_bytes())}});
    let (code, out) = import(&p, &rebase);
    assert_eq!(code, 0, "{out}");
    assert_eq!(fs::read_to_string(&report).unwrap(), text);
    assert!(read_only_status(&p)["projectionConflicts"]
        .as_array()
        .unwrap()
        .is_empty());
    // The gap is still reassessed next round; only the latest sealed round takes a waiver.
    let r3 = p.begin(&[]);
    p.capture_all(&r3);
    let (_, manifest) = p.pack(&r3, None);
    assert_eq!(manifest["previousUnits"][0]["gaps"][0]["id"], "UI-01");
    let waiver = decision(&p, &r2, &["UI-01"], "wholeUiWaiver");
    let (code, _) = waive_at(&p, &r2, json!({"decisions": [waiver]}), &cycle_revision(&p));
    assert_eq!(code, 3, "only the latest sealed round can receive a waiver");
    // The cycle revision flag never finalizes a draft.
    let (code, value) = waive_at(&p, &r3, json!({}), &cycle_revision(&p));
    assert_eq!(code, 2, "{value}");
}

#[test]
fn waiver_at_finalize_is_recorded_after_the_verdict_without_changing_it() {
    let p = Project::new(&["U1"]);
    let round = p.begin(&[]);
    p.capture_all(&round);
    p.judge(
        &round,
        "reviewer-a",
        &unit("U1", ["2", "2", "2", "1"], &[("", "P2", "3")]),
    );
    // Gap IDs are allocated at judge add, so the waiver can name every open gap.
    let waiver = decision(&p, &round, &["UI-01"], "wholeUiWaiver");
    let (code, out) = p.finalize(&round, json!({"decisions": [waiver]}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需要修改");
    assert_eq!(out["scoring"]["score"], "7");
    let report = fs::read_to_string(p.root.join(format!("{PREFIX}.ui-review.md"))).unwrap();
    let verdict = report.find("判定：需要修改").unwrap();
    let waived = report.find("用户豁免通过").unwrap();
    assert!(verdict < waived);
    assert_eq!(result_gap(&p, &round, "UI-01")["status"], "仍存在");
    // A later waiver on the same sealed round goes after the sealed waiver section.
    let mut later = decision(&p, &round, &["UI-01"], "wholeUiWaiver");
    later["impact"] = "Second acceptance after the sealed one".into();
    let (code, out) = waive_at(
        &p,
        &round,
        json!({"decisions": [later]}),
        &cycle_revision(&p),
    );
    assert_eq!(code, 0, "{out}");
    let text = fs::read_to_string(p.root.join(format!("{PREFIX}.ui-review.md"))).unwrap();
    let begin = text.find("<!-- phoenix-ui:waiver-").unwrap();
    let end = text.find(":end -->\n\n").unwrap() + ":end -->\n\n".len();
    assert!(
        waived < begin && text[begin..end].contains("Second acceptance"),
        "{text}"
    );
    assert!(text[end..].starts_with("## 评分\n"), "{text}");
    assert_eq!(format!("{}{}", &text[..begin], &text[end..]), report);
}

#[test]
fn partial_acceptance_changes_the_target_and_starts_a_new_baseline() {
    play("partial-acceptance", &["U1"], vec![
        Step::Round(None, unit("U1", ["2", "2", "2", "1"], &[("", "P2", "1.5"), ("", "P2", "1.5")]),
            expect("需要修改", Some("7"), Some("7"), NONE)),
        Step::Retarget("Accepted icon scale 24px; the user accepted the 28px header icon (2026-10-10, original quote kept)."),
        Step::Round(None, unit("U1", ["3", "2", "2.5", "1"], &[("UI-02", "P2", "1.5")]).prev("UI-01", "不再适用"),
            expect("通过验收", Some("8.5"), Some("8.5"), NONE)),
        Step::Check(|p, round, _| {
            assert!(p.round(round)["baseline"].as_str().unwrap().starts_with("r2:"));
            assert_eq!(result_gap(p, round, "UI-01")["status"], "不再适用");
            assert_eq!(result_gap(p, round, "UI-02")["status"], "仍存在");
        }),
    ]);
}

// ── status is read-only and never defaults to pass ─────────────────────────

fn tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push((path.clone(), fs::read(&path).unwrap()));
            }
        }
    }
    files.sort();
    files
}

fn read_only_status(p: &Project) -> Value {
    let before = tree(&p.root);
    let value = p.status();
    assert_eq!(
        tree(&p.root),
        before,
        "status must not create or change files"
    );
    value
}

/// Corrupt, future-schema or drifted managed state: the unknown object, ok:false, exit 4.
fn unknown_status(p: &Project) -> Value {
    let before = tree(&p.root);
    let (code, value) = p.run(&["status", "--prefix", PREFIX]);
    assert_eq!(code, 4, "{value}");
    assert_eq!(value["state"], "unknown", "{value}");
    assert!(value["verdict"].is_null());
    assert_eq!(
        tree(&p.root),
        before,
        "status must not create or change files"
    );
    value
}

#[test]
fn status_nested_prefix_without_parent_is_absent_and_does_not_create_it() {
    let p = Project::new(&["U1"]);
    let before = tree(&p.root);
    for prefix in ["docs/ui/new/orders", "new-project/ui/orders"] {
        let value = p.ok(&["status", "--prefix", prefix]);
        assert_eq!(value["state"], "absent");
        assert!(value["verdict"].is_null());
        assert_eq!(value["next"], json!(["begin"]));
        assert_eq!(tree(&p.root), before);
    }
    assert!(!p.root.join("docs/ui").exists());
    assert!(!p.root.join("new-project").exists());
}

#[test]
fn status_reports_states_units_and_next_steps_without_writing() {
    let p = Project::new(&["U1", "U2"]);
    let absent = read_only_status(&p);
    assert_eq!(absent["state"], "absent");
    assert!(absent["verdict"].is_null());
    fs::write(
        p.root.join(format!("{PREFIX}.ui-review.md")),
        "# Manual R1\n",
    )
    .unwrap();
    let manual = read_only_status(&p);
    assert_eq!(manual["state"], "manual");
    assert_eq!(manual["next"][0], "import");
    fs::remove_file(p.root.join(format!("{PREFIX}.ui-review.md"))).unwrap();
    let round = p.begin(&[]);
    let draft = read_only_status(&p);
    assert_eq!(draft["state"], "managed");
    assert_eq!(draft["current"]["roundId"], "r1");
    assert_eq!(draft["current"]["state"], "draft");
    assert_eq!(
        draft["current"]["missingCells"].as_array().unwrap().len(),
        2
    );
    assert_eq!(draft["next"][0], "capture add");
    p.capture_all(&round);
    assert_eq!(read_only_status(&p)["next"][0], "judge-pack");
    p.judge(
        &round,
        "reviewer-a",
        &unit("U1", ["3", "3", "2", "1"], &[("", "P2", "1")]).and(
            "U2",
            ["2", "2", "2", "1"],
            &[("", "P2", "3")],
        ),
    );
    assert_eq!(read_only_status(&p)["next"][0], "finalize");
    p.finalize(&round, json!({}));
    let sealed = read_only_status(&p);
    assert!(sealed["current"].is_null());
    assert_eq!(sealed["verdict"], "需要修改");
    assert_eq!(sealed["rounds"][0]["state"], "finalized");
    assert_eq!(sealed["rounds"][0]["score"], "7");
    assert_eq!(sealed["units"][1]["unitId"], "U2");
    assert_eq!(sealed["units"][1]["lastCoveredRound"], "r1");
    assert_eq!(sealed["units"][1]["score"], "7");
    assert_eq!(sealed["openGaps"].as_array().unwrap().len(), 2);
    assert_eq!(sealed["next"][0], "apply record");
    assert!(sealed["baseline"].as_str().unwrap().starts_with("r1:"));
    // A scoped round keeps each unit's nearest covering round.
    let r2 = p.begin(&["--scope", "U1"]);
    p.capture_all(&r2);
    p.judge(
        &r2,
        "reviewer-a",
        &unit("U1", ["3", "3", "2.5", "1"], &[("UI-01", "P2", "0.5")]),
    );
    p.finalize(&r2, json!({}));
    let scoped = read_only_status(&p);
    assert_eq!(scoped["units"][0]["lastCoveredRound"], "r2");
    assert_eq!(scoped["units"][1]["lastCoveredRound"], "r1");
    let carried = scoped["openGaps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == "UI-02")
        .unwrap()
        .clone();
    assert_eq!(carried["status"], "本轮未复核");
}

#[test]
fn damaged_future_drifted_or_publishing_state_is_unknown_never_pass() {
    let p = Project::new(&["U1"]);
    let (r1, out) = reviewed(
        &p,
        None,
        &unit("U1", ["3", "2.5", "2", "1"], &[("", "P2", "1.5")]),
    );
    assert_eq!(out["verdict"], "通过验收");
    assert_eq!(read_only_status(&p)["verdict"], "通过验收");
    let index = p.root.join(format!("{PREFIX}.ui-review.json"));
    let original = fs::read(&index).unwrap();
    for broken in [
        b"{\"schemaVersion\": 1,".to_vec(),
        String::from_utf8(original.clone())
            .unwrap()
            .replacen("\"schemaVersion\": 1", "\"schemaVersion\": 2", 1)
            .into_bytes(),
    ] {
        fs::write(&index, &broken).unwrap();
        let unknown = unknown_status(&p);
        assert!(unknown["problem"]["message"].is_string());
        // Text mode reports the same object and exit code, never "ok".
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let args = ["status", "--prefix", PREFIX].map(String::from);
        assert_eq!(cli::run(&args, &p.root, &mut out, &mut err), 4);
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("state: unknown") && text.contains("ok: false"),
            "{text}"
        );
        assert!(!text.contains("status: ok"), "{text}");
        assert!(!err.is_empty());
    }
    fs::write(&index, &original).unwrap();
    // A hand-edited sealed report is drift, not a higher score.
    let report = p.root.join(format!("{PREFIX}.ui-review.md"));
    let sealed = fs::read(&report).unwrap();
    fs::write(&report, b"edited").unwrap();
    let drift = unknown_status(&p);
    assert_eq!(drift["problem"]["exitCode"], 3);
    fs::write(&report, &sealed).unwrap();
    // An unfinished publication is visible and its round is not treated as sealed.
    let r2 = p.begin(&[]);
    p.capture_all(&r2);
    p.judge(
        &r2,
        "reviewer-a",
        &unit("U1", ["3", "2.5", "2", "1"], &[("UI-01", "P2", "1.5")]),
    );
    let revision = p.rev(&r2);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let directory = p.root.join(Path::new(&r2).parent().unwrap());
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o555)).unwrap();
        let (code, _) = p.finalize_at(&r2, json!({}), &revision);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(code, 4);
        let pending = read_only_status(&p);
        assert_eq!(pending["publicationPending"], true);
        assert_eq!(pending["rounds"][1]["state"], "publishing");
        assert_eq!(pending["verdict"], "通过验收");
        assert_eq!(pending["latestFinalizedRound"], "r1");
        assert!(p
            .root
            .join(format!("{PREFIX}.ui-review.publish.json"))
            .exists());
    }
    assert_eq!(p.round(&r1)["finalization"]["status"], "finalized");
}

// ── import: explicit legacy mapping, unknowns stay unknown ─────────────────

fn evidence(p: &Project, path: &str, bytes: &[u8]) -> Value {
    let full = p.root.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, bytes).unwrap();
    json!({"path": path, "sha256": hash(bytes)})
}

struct Legacy {
    u1_since: Value,
    u2_since: Value,
    u1_refactor: &'static str,
    u2_refactor: &'static str,
}

fn known(disposition: &str) -> Value {
    json!({"state": "known", "disposition": disposition})
}

fn legacy_project(legacy: Legacy) -> (Project, Value) {
    let p = Project::new(&["U1", "U2"]);
    let fingerprint = phoenix_ui_review::target::freeze(&p.root.join(TARGET), &p.root, &[])
        .unwrap()
        .fingerprint;
    let r1 = evidence(
        &p,
        &format!("{PREFIX}.ui-review.md"),
        "# R1 manual report\n本轮分数：6\n".as_bytes(),
    );
    let r2 = evidence(
        &p,
        &format!("{PREFIX}.ui-review.r2.md"),
        "# R2 manual report\n本轮分数：6.5\n".as_bytes(),
    );
    let mut units = Vec::new();
    for unit in ["U1", "U2"] {
        let image = evidence(
            &p,
            &format!("{PREFIX}.ui-captures/r2/{unit}-default-2-light.png"),
            &png(200),
        );
        let judge = evidence(&p, &format!("{PREFIX}.ui-captures/r2/judge-{unit}.md"),
            format!("## {unit}\n\nHeader icon is 32px, target is 24px. [full report](../../orders.ui-review.r2.md)\n").as_bytes());
        units.push(json!({"unitId": unit, "roundId": "r2", "judge": judge, "images": [image]}));
    }
    let applied = evidence(
        &p,
        &format!("{PREFIX}.ui-review.applied.md"),
        "## UI 验收修复\n\n### R2 → 修复\n\n| UI-01 | 成立 | 已修改待评审 | card.tsx | 未运行 |\n"
            .as_bytes(),
    );
    let map = json!({"kind": "legacyMarkdown", "schemaVersion": 1, "prefix": PREFIX, "target": TARGET,
        "plan": null, "recordPath": format!("{PREFIX}.ui-review.applied.md"), "threshold": "8",
        "rounds": [
            {"roundId": "r1", "report": r1, "targetFingerprint": null, "fullScore": {"state": "known", "value": "6"}},
            {"roundId": "r2", "report": r2, "targetFingerprint": {"algorithm": "phoenix-ui-target-v1", "value": fingerprint},
                "fullScore": {"state": "known", "value": "6.5"}}],
        "gaps": [
            {"id": "UI-01", "unitId": "U1", "severity": "P2", "origin": "reviewer",
                "assessments": [{"roundId": "r1", "status": "仍存在", "priorDisposition": {"state": "absent"}},
                    {"roundId": "r2", "status": "仍存在", "priorDisposition": known("已修改待评审")}],
                "sinceLastAssessment": legacy.u1_since, "pendingDecision": false},
            {"id": "UI-02", "unitId": "U2", "severity": "P2", "origin": "reviewer",
                "assessments": [{"roundId": "r2", "status": "仍存在", "priorDisposition": {"state": "absent"}}],
                "sinceLastAssessment": legacy.u2_since, "pendingDecision": false}],
        "refactors": {"global": "not_recorded", "units": [
            {"unitId": "U1", "refactor": legacy.u1_refactor}, {"unitId": "U2", "refactor": legacy.u2_refactor}]},
        "units": units, "sources": [applied]});
    (p, map)
}

fn import(p: &Project, map: &Value) -> (i32, Value) {
    let path = p.write("legacy-map", map);
    p.run(&["import", "--prefix", PREFIX, "--input", &path])
}

#[test]
fn legacy_markdown_import_keeps_hashes_and_continues_as_a_managed_cycle() {
    let (p, map) = legacy_project(Legacy {
        u1_since: known("已修改待评审"),
        u2_since: json!({"state": "unknown"}),
        u1_refactor: "not_recorded",
        u2_refactor: "unknown",
    });
    assert_eq!(
        p.begin_with(&[]).0,
        3,
        "manual history needs an explicit import"
    );
    let legacy_files: Vec<_> = tree(&p.root)
        .into_iter()
        .filter(|(path, _)| !path.starts_with(p.root.join("inputs")))
        .collect();
    let (code, out) = import(&p, &map);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["kind"], "legacyMarkdown");
    for (path, bytes) in &legacy_files {
        assert_eq!(
            &fs::read(path).unwrap(),
            bytes,
            "import never rewrites legacy files"
        );
    }
    let index = p.read(&format!("{PREFIX}.ui-review.json"));
    assert_eq!(index["legacy"]["marker"], "legacy-markdown");
    assert_eq!(
        index["legacy"]["input"]["sha256"],
        hash(&fs::read(p.root.join("inputs/legacy-map.json")).unwrap())
    );
    assert_eq!(index["nextIssueId"], 3);
    let status = read_only_status(&p);
    assert_eq!(status["state"], "managed");
    let unknowns = status["legacy"]["unknowns"].to_string();
    assert!(
        unknowns.contains("UI-02") && unknowns.contains("U2"),
        "{unknowns}"
    );
    let (code, again) = import(&p, &map);
    assert_eq!(code, 3, "{again}");
    // Managed continuation: next round number, same legacy baseline, previous material.
    let (code, begin) = p.begin_with(&[]);
    assert_eq!(code, 0, "{begin}");
    assert_eq!(begin["roundId"], "r3");
    assert_eq!(begin["baselineStarted"], false);
    assert!(begin["baseline"].as_str().unwrap().starts_with("r2:"));
    let r3 = begin["round"].as_str().unwrap().to_owned();
    p.capture_all(&r3);
    let (packet, manifest) = p.pack(&r3, None);
    let previous = &manifest["previousUnits"];
    assert_eq!(previous[0]["roundId"], "r2");
    assert_eq!(
        previous[0]["gaps"],
        json!([{"id": "UI-01", "unitId": "U1"}])
    );
    let text = fs::read_to_string(
        p.root
            .join(packet["directory"].as_str().unwrap())
            .join(previous[0]["legacy"]["judge"]["path"].as_str().unwrap()),
    )
    .unwrap();
    assert!(text.contains("Header icon is 32px") && !text.contains("orders.ui-review.r2.md"));
    let spec = unit(
        "U1",
        ["2", "2", "1", "1"],
        &[("UI-01", "P2", "3"), ("", "P2", "1")],
    )
    .and("U2", ["3", "3", "2", "1"], &[("", "P2", "1")])
    .prev("UI-02", "已解决");
    let input = p.judge_input(&manifest, &packet, "reviewer-a", &spec);
    let path = p.write("judge-legacy", &input);
    fs::write(p.root.join("inputs/raw.md"), RAW).unwrap();
    let (code, value) = p.run(&[
        "judge",
        "add",
        "--round",
        &r3,
        "--input",
        &path,
        "--raw",
        "inputs/raw.md",
        "--packet",
        packet["packetHash"].as_str().unwrap(),
        "--expect-revision",
        &p.rev(&r3),
    ]);
    assert_eq!(code, 0, "{value}");
    assert_eq!(value["assignments"][1]["id"], "UI-03");
    let (code, out) = p.finalize(&r3, json!({}));
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["verdict"], "需要修改");
    assert_eq!(out["scoring"]["historicalBest"], "6.5");
    // The unknown-fingerprint legacy R1 is not in this baseline, so no plateau.
    assert_eq!(warnings(&out), ["UI-01:停滞预警"]);
    assert_eq!(result_gap(&p, &r3, "UI-02")["status"], "已解决");
    // The unknown legacy U2 refactor count cannot be claimed again.
    let (code, value) = p.apply(
        &r3,
        &[("UI-03", "已修改待评审")],
        json!([{"scope": {"kind": "global"},
        "hypothesis": "h", "change": "c", "expected": "e"}]),
    );
    assert_eq!(code, 0, "{value}");
    let u2_refactor = json!([{"scope": {"kind": "unit", "unitId": "U2"},
        "hypothesis": "h", "change": "c", "expected": "e"}]);
    // The resolved UI-02 takes no disposition, so it cannot open a U2 refactor either.
    let (code, value) = p.apply(&r3, &[("UI-02", "已修改待评审")], u2_refactor.clone());
    assert_eq!(code, 2, "{value}");
    let (code, value) = p.apply(&r3, &[("UI-04", "已修改待评审")], u2_refactor);
    assert_eq!(code, 3, "{value}");
    let (code, value) = p.apply(
        &r3,
        &[("UI-01", "已修改待评审")],
        json!([{"scope": {"kind": "unit", "unitId": "U1"},
        "hypothesis": "h", "change": "c", "expected": "e"}]),
    );
    assert_eq!(code, 0, "{value}");
}

#[test]
fn unknown_legacy_attempt_or_refactor_history_is_never_counted_as_zero() {
    // Unknown attempts: a reassessed 仍存在 cannot be classified as stubborn or not.
    let (p, map) = legacy_project(Legacy {
        u1_since: json!({"state": "absent"}),
        u2_since: json!({"state": "unknown"}),
        u1_refactor: "not_recorded",
        u2_refactor: "not_recorded",
    });
    assert_eq!(import(&p, &map).0, 0);
    let (r3, before) = {
        let r3 = p.begin(&[]);
        p.capture_all(&r3);
        p.judge(
            &r3,
            "reviewer-a",
            &unit("U1", ["3", "3", "3", "1"], &[])
                .prev("UI-01", "已解决")
                .and("U2", ["3", "3", "2", "1"], &[("UI-02", "P2", "1")]),
        );
        let before = fs::read(p.root.join(&r3)).unwrap();
        (r3, before)
    };
    let (code, out) = p.finalize(&r3, json!({}));
    assert_eq!(code, 3, "{out}");
    assert!(out["error"].as_str().unwrap().contains("unknown"));
    assert_eq!(fs::read(p.root.join(&r3)).unwrap(), before);
    // Unknown refactor count: a stubborn warning cannot become a warning or a stall.
    let (p, map) = legacy_project(Legacy {
        u1_since: known("已修改待评审"),
        u2_since: json!({"state": "absent"}),
        u1_refactor: "unknown",
        u2_refactor: "not_recorded",
    });
    assert_eq!(import(&p, &map).0, 0);
    let r3 = p.begin(&[]);
    p.capture_all(&r3);
    p.judge(
        &r3,
        "reviewer-a",
        &unit("U1", ["3", "3", "2", "1"], &[("UI-01", "P2", "1")])
            .and("U2", ["3", "3", "3", "1"], &[])
            .prev("UI-02", "已解决"),
    );
    let (code, out) = p.finalize(&r3, json!({}));
    assert_eq!(code, 3, "{out}");
    assert!(out["error"].as_str().unwrap().contains("refactor"));
    // After a user-confirmed target change the legacy baseline no longer applies.
    p.set_target("Accepted icon scale 20px.", "Open the real app.");
    let (code, _) = p.begin_with(&[]);
    assert_eq!(code, 3, "the current draft still has to be finished first");
}

#[test]
fn import_rejects_ambiguous_or_unverifiable_maps_without_writing() {
    let base = || {
        legacy_project(Legacy {
            u1_since: known("已修改待评审"),
            u2_since: json!({"state": "absent"}),
            u1_refactor: "not_recorded",
            u2_refactor: "not_recorded",
        })
    };
    type MapCase = (&'static str, fn(&mut Value), i32);
    let cases: Vec<MapCase> = vec![
        (
            "report hash drift",
            |m| m["rounds"][0]["report"]["sha256"] = "0".repeat(64).into(),
            3,
        ),
        (
            "missing judge file",
            |m| m["units"][0]["judge"]["path"] = "docs/missing.md".into(),
            2,
        ),
        (
            "duplicate gap id",
            |m| {
                let gap = m["gaps"][0].clone();
                m["gaps"].as_array_mut().unwrap().push(gap);
            },
            2,
        ),
        (
            "open gap without material",
            |m| {
                m["units"].as_array_mut().unwrap().remove(1);
            },
            2,
        ),
        (
            "refactor count omitted",
            |m| {
                m["refactors"]["units"].as_array_mut().unwrap().remove(1);
            },
            2,
        ),
        ("prefix mismatch", |m| m["prefix"] = "docs/other".into(), 2),
        (
            "non-contiguous rounds",
            |m| m["rounds"][1]["roundId"] = "r3".into(),
            2,
        ),
        (
            "unknown fingerprint algorithm",
            |m| m["rounds"][1]["targetFingerprint"]["algorithm"] = "md5".into(),
            2,
        ),
        (
            "unknown full score in the current baseline",
            |m| m["rounds"][1]["fullScore"] = json!({"state": "unknown"}),
            3,
        ),
    ];
    for (name, mutate, expected) in cases {
        let (p, mut map) = base();
        mutate(&mut map);
        let before = tree(&p.root);
        let (code, out) = import(&p, &map);
        assert_eq!(code, expected, "{name}: {out}");
        let data = |(path, _): &(PathBuf, Vec<u8>)| {
            !path.ends_with("legacy-map.json") && path.extension().is_none_or(|ext| ext != "lock")
        };
        let after: Vec<_> = tree(&p.root).into_iter().filter(data).collect();
        let before: Vec<_> = before.into_iter().filter(data).collect();
        assert_eq!(after.len(), before.len(), "{name}: import wrote files");
    }
}

#[test]
fn unknown_full_score_in_the_current_legacy_baseline_keeps_the_cycle_manual() {
    let legacy = || Legacy {
        u1_since: known("已修改待评审"),
        u2_since: json!({"state": "absent"}),
        u1_refactor: "not_recorded",
        u2_refactor: "not_recorded",
    };
    let (p, mut map) = legacy_project(legacy());
    map["rounds"][1]["fullScore"] = json!({"state": "unknown"});
    let (code, out) = import(&p, &map);
    assert_eq!(code, 3, "{out}");
    assert!(!p.root.join(format!("{PREFIX}.ui-review.json")).exists());
    assert_eq!(read_only_status(&p)["state"], "manual");
    // R1 has an unknown fingerprint, so its unknown score is outside the current baseline.
    let (p, mut map) = legacy_project(legacy());
    map["rounds"][0]["fullScore"] = json!({"state": "unknown"});
    let (code, out) = import(&p, &map);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn dispositions_only_target_open_gaps_of_that_round_report() {
    let p = Project::new(&["U1"]);
    reviewed(
        &p,
        None,
        &unit(
            "U1",
            ["2", "2", "1", "1"],
            &[("", "P2", "2"), ("", "P2", "2")],
        ),
    );
    let (r2, _) = reviewed(
        &p,
        None,
        &unit("U1", ["2", "2", "2", "1"], &[("UI-01", "P2", "3")]).prev("UI-02", "已解决"),
    );
    assert_eq!(result_gap(&p, &r2, "UI-02")["status"], "已解决");
    let refactor = json!([{"scope": {"kind": "unit", "unitId": "U1"},
        "hypothesis": "Flex wrapping drives the icon size", "change": "Two-column grid",
        "expected": "Visual 3"}]);
    let applied = p.root.join(format!("{PREFIX}.ui-review.applied.json"));
    // A resolved gap takes no disposition and so cannot satisfy the refactor precondition.
    let (code, out) = p.apply(&r2, &[("UI-02", "已修改待评审")], refactor.clone());
    assert_eq!(code, 2, "{out}");
    let (code, out) = p.apply(&r2, &[("UI-99", "未处理")], json!([]));
    assert_eq!(code, 2, "{out}");
    assert!(!applied.exists());
    let (code, out) = p.apply(&r2, &[("UI-01", "已修改待评审")], refactor);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn projection_rebase_restores_a_generated_block_only_after_the_manual_text_is_saved() {
    let p = Project::new(&["U1", "U2"]);
    let round = p.begin(&[]);
    assert_eq!(p.capture(&round, "U1", 1, Shot::Image(255)).0, 0);
    let list = Path::new(&round).parent().unwrap().join("capture.md");
    let original = fs::read_to_string(p.root.join(&list)).unwrap();
    let edited = format!(
        "Manual preface.\n\n{}",
        original.replace("| U1 |", "| U1 (hand edited) |")
    );
    fs::write(p.root.join(&list), &edited).unwrap();
    assert_eq!(p.capture(&round, "U2", 1, Shot::Image(255)).0, 3);
    let rebase = |saved: Value, revision: &str| {
        json!({"kind": "projectionRebase", "schemaVersion": 1, "prefix": PREFIX,
            "cycleRevision": revision.parse::<u64>().unwrap(), "file": list.to_string_lossy(),
            "currentSha256": hash(edited.as_bytes()), "savedCopy": saved})
    };
    let unsaved = json!({"path": "inputs/saved-capture.md", "sha256": hash(edited.as_bytes())});
    let revision = cycle_revision(&p);
    let (code, out) = import(&p, &rebase(unsaved.clone(), &revision));
    assert_eq!(code, 2, "{out}");
    fs::write(p.root.join("inputs/saved-capture.md"), &edited).unwrap();
    let (code, _) = import(&p, &rebase(unsaved.clone(), "99"));
    assert_eq!(code, 3);
    let (code, out) = import(&p, &rebase(unsaved, &revision));
    assert_eq!(code, 0, "{out}");
    let restored = fs::read_to_string(p.root.join(&list)).unwrap();
    assert!(restored.starts_with("Manual preface.\n\n") && !restored.contains("hand edited"));
    assert_eq!(
        fs::read_to_string(p.root.join("inputs/saved-capture.md")).unwrap(),
        edited
    );
    assert_eq!(p.capture(&round, "U2", 1, Shot::Image(255)).0, 0);
    // Sealed history is never re-based.
    p.judge(
        &round,
        "reviewer-a",
        &unit("U1", ["3", "3", "2", "1"], &[("", "P2", "1")]),
    );
    p.finalize(&round, json!({}));
    let report = format!("{PREFIX}.ui-review.md");
    let bytes = fs::read(p.root.join(&report)).unwrap();
    fs::write(p.root.join("inputs/saved-report.md"), &bytes).unwrap();
    let sealed = json!({"kind": "projectionRebase", "schemaVersion": 1, "prefix": PREFIX,
        "cycleRevision": cycle_revision(&p).parse::<u64>().unwrap(), "file": report,
        "currentSha256": hash(&bytes), "savedCopy": {"path": "inputs/saved-report.md", "sha256": hash(&bytes)}});
    assert_eq!(import(&p, &sealed).0, 3);
}
