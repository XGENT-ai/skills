//! Public protocol regression fixtures. Generated PNGs test structure, not UI truth.
use phoenix_ui_review::{
    capture_evidence::{
        CaptureInput, FileEvidence, HumanCheck, Observation, Origin, TestedVersion, WorkingTree,
    },
    protocol::{self, BeginOptions, Round},
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
const TEXT: &str = "# Orders\n\n## 单元\n\n| 单元 | 必需状态 | 视口 | 主题 | 来源 |\n| --- | --- | --- | --- | --- |\n| U1 Orders | default | 2×2 | light | [Design](../DESIGN.md) |\n| U2 Editor | default | 2×2 | light | [Design](../DESIGN.md) |\n\n## 规范要点\n\nAccepted icon scale 24px.\n\n## 采集\n\nOpen the real app.\n\n## 变更记录\n\nR1 established.\n";
const RAW: &[u8] = b"Independent structural fixture. Full raw multi-unit conclusion stays private.\nPRIVATE_RAW_BODY\n";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255; 16]).unwrap();
    }
    bytes
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-review-previous-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join(TARGET), TEXT).unwrap();
        fs::write(root.join("DESIGN.md"), "Accepted icon scale 24px.").unwrap();
        Self(root)
    }

    fn begin(&self, units: Option<&[&str]>) -> PathBuf {
        let round = protocol::begin(
            &self.0,
            BeginOptions {
                prefix: PREFIX.into(),
                target: TARGET.into(),
                plan: None,
                scope: units.map(|units| units.iter().map(|s| (*s).into()).collect()),
                threshold: None,
                version: TestedVersion {
                    head: None,
                    working_tree: WorkingTree::Unknown,
                    build_id: Some("previous-structural-fixture".into()),
                },
            },
        )
        .unwrap();
        protocol::round_path(PREFIX, round.round_id[1..].parse().unwrap())
    }

    fn round(&self, path: &Path) -> Round {
        serde_json::from_slice(&fs::read(self.0.join(path)).unwrap()).unwrap()
    }

    fn capture(&self, path: &Path, unit: &str, attempt: u32, missing: bool) {
        let round = self.round(path);
        let cell = round
            .matrix
            .cells
            .iter()
            .find(|cell| cell.key.unit_id == unit)
            .unwrap();
        let image_path = path
            .parent()
            .unwrap()
            .join(round.matrix.screenshot_name(&cell.key, attempt).unwrap());
        let bytes = png();
        let observation = if missing {
            Observation::Missing {
                reason: "Fixture app unavailable on this attempt".into(),
            }
        } else {
            fs::write(self.0.join(&image_path), &bytes).unwrap();
            Observation::Captured {
                image: FileEvidence {
                    path: image_path.to_string_lossy().into_owned(),
                    sha256: hash(&bytes),
                },
                origin: Origin::Browser,
                tool: "Generated PNG structural fixture, not product capture".into(),
                address: "http://localhost:8080/orders".into(),
                taken_at: "2026-10-10T04:00:00Z".into(),
                viewport: Box::new(cell.viewport.clone()),
                tested_version: Box::new(round.tested_version.clone()),
                interactions: vec![],
                human_check: HumanCheck {
                    opened: true,
                    current_product: true,
                    current_version: true,
                    notes: "Declared structural provenance, not visual acceptance".into(),
                },
            }
        };
        protocol::add_capture(
            &self.0,
            path,
            round.revision,
            CaptureInput {
                schema_version: 1,
                round_id: round.round_id,
                cell: cell.key.clone(),
                attempt,
                observation,
            },
        )
        .unwrap();
    }

    fn capture_all(&self, path: &Path) {
        for unit in self.round(path).matrix.selected_units() {
            self.capture(path, unit, 1, false);
        }
    }

    fn packet(&self, path: &Path) -> (protocol::packet::PacketRef, Value) {
        let packet = protocol::packet::create(&self.0, path, None).unwrap();
        let manifest =
            serde_json::from_slice(&fs::read(self.0.join(&packet.manifest.path)).unwrap()).unwrap();
        (packet, manifest)
    }

    fn input(&self, path: &Path, reviewer: &str) -> Value {
        let (packet, manifest) = self.packet(path);
        let units: Vec<_> = manifest["units"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(n, unit)| {
                let capture = current(&manifest, unit.as_str().unwrap());
                let basis = manifest["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|file| file["path"] == "expectations.txt")
                    .unwrap();
                json!({
                    "unitId": unit, "scores": ["3", "2", "3", "1"],
                    "reasons": ["Layout matches", "Icon is too large", "State matches", "Copy matches"],
                    "gaps": [{"key": format!("新-{}", n + 1), "cell": capture["input"]["cell"],
                        "image": capture["input"]["observation"]["image"], "dimension": "视觉",
                        "location": "Header icon", "current": "32px", "expected": "24px",
                        "basis": basis, "basisLocation": "Accepted icon scale", "fix": "Use 24px icon",
                        "severity": "P2", "recoverable": "1"}],
                    "suggestions": ["Optional copy change; no deduction"], "unseenCells": []
                })
            })
            .collect();
        let previous: Vec<Value> = manifest["previousUnits"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|unit| {
                let capture = current(&manifest, unit["unitId"].as_str().unwrap());
                unit["gaps"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(move |gap| {
                        json!({"id": gap["id"], "status": "仍存在", "basis": {
                            "cell": capture["input"]["cell"], "image": capture["input"]["observation"]["image"],
                            "notes": "Current image still shows the same oversized icon"}})
                    })
            })
            .collect();
        json!({
            "schemaVersion": 1, "cycleId": manifest["cycleId"], "roundId": manifest["roundId"],
            "packetHash": packet.packet_hash, "reviewerId": reviewer, "rawSha256": hash(RAW),
            "provenance": {"freshContext": true, "readOnly": true, "inheritedModel": true,
                "openedImages": manifest["files"].as_array().unwrap().iter().filter(|file| file["path"].as_str().unwrap().ends_with(".png")).collect::<Vec<_>>(),
                "notes": "Declared fixture provenance; no real UI acceptance"},
            "conversion": {"kind": "reviewerStructured"}, "units": units,
            "unreviewedUnits": [], "previous": previous, "summary": "Per-unit independent conclusion"
        })
    }

    fn add(
        &self,
        path: &Path,
        input: Value,
    ) -> Result<protocol::judgement::JudgeReceipt, phoenix_ui_review::storage::StoreError> {
        protocol::judgement::add(
            &self.0,
            path,
            self.round(path).revision,
            serde_json::from_value(input).unwrap(),
            RAW,
            vec![],
        )
    }

    fn seal(&self, path: &Path) {
        let round = self.round(path);
        let response = protocol::finalize::run(
            &self.0,
            path,
            round.revision,
            protocol::finalize::Adjudication {
                schema_version: 1,
                cycle_id: round.cycle_id,
                round_id: round.round_id,
                machine_checks: vec![],
                main_checks: vec![],
                decisions: vec![],
                applied: vec![],
                notes: "PRIVATE_HOST_OPINION; structural fixture, not acceptance".into(),
            },
        )
        .unwrap();
        assert!(matches!(
            response,
            protocol::finalize::Response::Finalized { .. }
        ));
    }

    fn reviewed(&self, units: Option<&[&str]>) -> PathBuf {
        let path = self.begin(units);
        self.capture_all(&path);
        self.add(&path, self.input(&path, "fixture-reviewer"))
            .unwrap();
        self.seal(&path);
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn current<'a>(manifest: &'a Value, unit: &str) -> &'a Value {
    manifest["captures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|capture| capture["current"] == true && capture["input"]["cell"]["unitId"] == unit)
        .unwrap()
}

#[test]
fn packet_uses_nearest_covered_round_for_each_assigned_unit() {
    let f = Fixture::new();
    f.reviewed(None);
    f.reviewed(Some(&["U1"]));
    f.reviewed(Some(&["U2"]));
    let path = f.begin(None);
    f.capture_all(&path);
    let (_, manifest) = f.packet(&path);
    let previous = manifest["previousUnits"]
        .as_array()
        .expect("Missing per-unit previous materials");
    assert_eq!(previous.len(), 2);
    assert_eq!(previous[0]["unitId"], "U1");
    assert_eq!(previous[0]["roundId"], "r2");
    assert_eq!(previous[1]["unitId"], "U2");
    assert_eq!(previous[1]["roundId"], "r3");
    assert!(!previous[0]["judges"].as_array().unwrap().is_empty());
    assert!(!previous[1]["judges"].as_array().unwrap().is_empty());
}

#[test]
fn judge_cannot_omit_assigned_previous_gap_assessments() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let mut input = f.input(&path, "current-reviewer");
    input["previous"] = json!([]);
    assert_eq!(
        f.add(&path, input)
            .expect_err("Missing old gap conclusion was accepted")
            .code,
        2
    );
}

#[test]
fn packet_preserves_historical_missing_and_recapture_attempts() {
    let f = Fixture::new();
    let old = f.begin(Some(&["U1"]));
    f.capture(&old, "U1", 1, true);
    f.capture(&old, "U1", 2, false);
    f.add(&old, f.input(&old, "previous-reviewer")).unwrap();
    f.seal(&old);
    let path = f.begin(Some(&["U1"]));
    f.capture(&path, "U1", 1, true);
    f.capture(&path, "U1", 2, false);
    let (_, manifest) = f.packet(&path);
    let captures = manifest["previousUnits"][0]["captures"]
        .as_array()
        .expect("Missing historical attempts");
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0]["current"], false);
    assert_eq!(captures[0]["input"]["observation"]["status"], "missing");
    assert_eq!(captures[1]["current"], true);
    assert_eq!(captures[1]["input"]["attempt"], 2);
    assert!(captures[1]["input"]["observation"]["image"]["path"]
        .as_str()
        .unwrap()
        .starts_with("previous/r1/"));
    assert!(
        current(&manifest, "U1")["input"]["observation"]["image"]["path"]
            .as_str()
            .unwrap()
            .starts_with("current/")
    );
}

#[test]
fn first_round_has_no_previous_material_and_packet_creation_is_idempotent() {
    let f = Fixture::new();
    let path = f.begin(None);
    f.capture_all(&path);
    let (first, manifest) = f.packet(&path);
    assert_eq!(manifest["previousUnits"], json!([]));
    let revision = f.round(&path).revision;
    let (again, _) = f.packet(&path);
    assert_eq!(first.packet_hash, again.packet_hash);
    assert_eq!(f.round(&path).revision, revision);
}

#[test]
fn previous_packet_excludes_other_units_raw_report_source_and_host_opinion() {
    let f = Fixture::new();
    f.reviewed(None);
    fs::create_dir_all(f.0.join("src")).unwrap();
    fs::write(f.0.join("src/private.rs"), "PRIVATE_SOURCE").unwrap();
    fs::write(f.0.join("docs/fixes.md"), "PRIVATE_FIXES").unwrap();
    fs::write(f.0.join("docs/machine.json"), "PRIVATE_MACHINE").unwrap();
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let (packet, manifest) = f.packet(&path);
    assert_eq!(manifest["previousUnits"].as_array().unwrap().len(), 1);
    assert_eq!(manifest["previousUnits"][0]["unitId"], "U1");
    let previous = &manifest["previousUnits"][0];
    assert_eq!(
        previous["judges"][0]["unit"]["scores"],
        json!(["3", "2", "3", "1"])
    );
    assert_eq!(previous["judges"][0]["rawSha256"], hash(RAW));
    assert_eq!(previous["judges"][0]["unit"]["gaps"][0]["key"], "UI-01");
    let directory =
        f.0.join(packet.manifest.path)
            .parent()
            .unwrap()
            .to_path_buf();
    let mut all_text = serde_json::to_string(&manifest).unwrap();
    for file in manifest["files"].as_array().unwrap() {
        let name = file["path"].as_str().unwrap();
        if !name.ends_with(".png") {
            all_text.push_str(&fs::read_to_string(directory.join(name)).unwrap());
        }
    }
    for private in [
        "PRIVATE_RAW_BODY",
        "PRIVATE_SOURCE",
        "PRIVATE_FIXES",
        "PRIVATE_MACHINE",
        "PRIVATE_HOST_OPINION",
        "threshold",
        "machineChecks",
        "orders.ui-review.md",
        "raw.md",
        "record.json",
        "judge-pack/",
    ] {
        assert!(
            !all_text.contains(private),
            "Unexpected private material: {private}"
        );
    }
    assert!(!manifest["previousUnits"][0]["captures"]
        .to_string()
        .contains("U2"));
}

#[test]
fn nearest_sealed_missing_unit_is_explicit_and_does_not_fabricate_a_judge() {
    let f = Fixture::new();
    let old = f.begin(Some(&["U1"]));
    f.capture(&old, "U1", 1, true);
    f.seal(&old);
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let (_, manifest) = f.packet(&path);
    let previous = &manifest["previousUnits"][0];
    assert_eq!(previous["roundId"], "r1");
    assert_eq!(previous["judges"], json!([]));
    assert_eq!(previous["gaps"], json!([]));
    assert!(previous["unreviewedReason"].is_string());
    assert_eq!(
        previous["captures"][0]["input"]["observation"]["status"],
        "missing"
    );
    f.add(&path, f.input(&path, "current-reviewer")).unwrap();
}

#[test]
fn retained_old_id_keeps_counter_and_exact_raw_and_is_idempotent() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let mut input = f.input(&path, "current-reviewer");
    input["units"][0]["gaps"][0]["key"] = "UI-01".into();
    let receipt = f.add(&path, input.clone()).unwrap();
    assert_eq!(receipt.assignments[0].id, "UI-01");
    assert_eq!(receipt.assignments[0].key, "UI-01");
    let index: Value =
        serde_json::from_slice(&fs::read(f.0.join(format!("{PREFIX}.ui-review.json"))).unwrap())
            .unwrap();
    assert_eq!(index["nextIssueId"], 2);
    let record: protocol::judgement::JudgeRecord =
        serde_json::from_slice(&fs::read(f.0.join(&receipt.file.path)).unwrap()).unwrap();
    assert_eq!(fs::read(f.0.join(record.raw.path)).unwrap(), RAW);
    assert_eq!(record.input.raw_sha256, hash(RAW));
    assert_eq!(
        record.input.units[0].scores.map(|s| s.to_string()),
        ["3", "2", "3", "1"]
    );
    let revision = f.round(&path).revision;
    let again = f.add(&path, input).unwrap();
    assert_eq!(receipt.file, again.file);
    assert_eq!(f.round(&path).revision, revision);
    let (packet, _) = f.packet(&path);
    assert_eq!(packet.packet_hash, record.input.packet_hash);
}

#[test]
fn explicit_mapping_reuses_the_assigned_previous_id_without_allocation() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let input = f.input(&path, "current-reviewer");
    let receipt = protocol::judgement::add(
        &f.0,
        &path,
        f.round(&path).revision,
        serde_json::from_value(input).unwrap(),
        RAW,
        vec![protocol::judgement::GapMapping {
            key: "新-1".into(),
            id: "UI-01".into(),
            basis: "Same header icon, verified against previous unit material".into(),
        }],
    )
    .unwrap();
    assert_eq!(receipt.assignments[0].id, "UI-01");
    let index: Value =
        serde_json::from_slice(&fs::read(f.0.join(format!("{PREFIX}.ui-review.json"))).unwrap())
            .unwrap();
    assert_eq!(index["nextIssueId"], 2);
}

#[test]
fn all_six_previous_states_require_typed_current_evidence() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    for (n, status) in ["已解决", "部分解决", "仍存在", "回归", "待核实", "不再适用"]
        .iter()
        .enumerate()
    {
        let mut input = f.input(&path, &format!("reviewer-{n}"));
        input["previous"][0]["status"] = (*status).into();
        f.add(&path, input).unwrap();
    }
}

#[test]
fn previous_assessment_rejects_unknown_id_cross_unit_old_image_duplicate_and_no_basis() {
    let f = Fixture::new();
    f.reviewed(None);
    let path = f.begin(None);
    f.capture_all(&path);
    let good = f.input(&path, "current-reviewer");
    let (_, manifest) = f.packet(&path);
    let old_image =
        manifest["previousUnits"][0]["captures"][0]["input"]["observation"]["image"].clone();
    for case in 0..8 {
        let mut bad = good.clone();
        match case {
            0 => bad["previous"][0]["id"] = "UI-999".into(),
            1 => bad["previous"][0]["basis"]["cell"] = good["previous"][1]["basis"]["cell"].clone(),
            2 => bad["previous"][0]["basis"]["image"] = old_image.clone(),
            3 => {
                let duplicate = bad["previous"][0].clone();
                bad["previous"].as_array_mut().unwrap().push(duplicate);
            }
            4 => bad["previous"][0]["basis"]["notes"] = " ".into(),
            5 => bad["previous"][0]["basis"] = "Only an unbound legacy assertion".into(),
            6 => bad["previous"][0]["status"] = "本轮未复核".into(),
            _ => bad["previous"][0]["basis"]["image"]["sha256"] = "0".repeat(64).into(),
        }
        assert_eq!(f.add(&path, bad).unwrap_err().code, 2, "case {case}");
    }
    let mut bad = good.clone();
    bad["units"][0]["gaps"][0]["image"] = old_image;
    assert_eq!(f.add(&path, bad).unwrap_err().code, 2);
    // Equal PNG hashes across genuine rounds are allowed; the evidence path/round matters.
    assert_eq!(
        good["previous"][0]["basis"]["image"]["sha256"],
        manifest["previousUnits"][0]["captures"][0]["input"]["observation"]["image"]["sha256"]
    );
    f.add(&path, good).unwrap();
}

#[test]
fn packet_unit_selection_cannot_assess_or_retain_another_units_old_id() {
    let f = Fixture::new();
    f.reviewed(None);
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let mut bad = f.input(&path, "current-reviewer");
    bad["previous"][0]["id"] = "UI-02".into();
    assert_eq!(f.add(&path, bad).unwrap_err().code, 2);
    let mut bad = f.input(&path, "current-reviewer");
    bad["units"][0]["gaps"][0]["key"] = "UI-02".into();
    assert_eq!(f.add(&path, bad).unwrap_err().code, 2);
}

#[test]
fn host_conversion_must_attest_previous_assessments_too() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let mut input = f.input(&path, "current-reviewer");
    input["conversion"] = json!({"kind": "hostVerified", "rawSha256": hash(RAW), "checkedUnits": ["U1"], "checkedGaps": ["新-1"], "notes": "Faithful checked transcription"});
    assert_eq!(f.add(&path, input.clone()).unwrap_err().code, 2);
    input["conversion"]["checkedGaps"] = json!(["新-1", "UI-01"]);
    f.add(&path, input).unwrap();
}

#[test]
fn old_frozen_image_and_judge_drift_reject_packet_and_judge_registration() {
    for image in [true, false] {
        let f = Fixture::new();
        let old = f.reviewed(Some(&["U1"]));
        let path = f.begin(Some(&["U1"]));
        f.capture_all(&path);
        let input = f.input(&path, "current-reviewer");
        let old = f.round(&old);
        let evidence = if image {
            old.captures[0].frozen_image.as_ref().unwrap()
        } else {
            &old.judges[0]
        };
        fs::write(f.0.join(&evidence.path), b"tampered evidence").unwrap();
        assert_eq!(
            protocol::packet::create(&f.0, &path, None)
                .unwrap_err()
                .code,
            3
        );
        assert_eq!(f.add(&path, input).unwrap_err().code, 3);
    }
}

#[test]
fn historical_images_come_from_frozen_copies_even_if_original_capture_is_gone() {
    let f = Fixture::new();
    let old = f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let old = f.round(&old);
    let Observation::Captured { image, .. } = &old.captures[0].input.observation else {
        panic!()
    };
    fs::remove_file(f.0.join(&image.path)).unwrap();
    let (_, manifest) = f.packet(&path);
    assert_eq!(
        manifest["previousUnits"][0]["captures"][0]["input"]["observation"]["image"]["sha256"],
        image.sha256
    );
    f.add(&path, f.input(&path, "current-reviewer")).unwrap();
}

#[test]
fn previous_image_cannot_be_registered_as_a_current_capture() {
    let f = Fixture::new();
    let old = f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    let mut input = f.round(&old).captures[0].input.clone();
    input.round_id = "r2".into();
    assert_eq!(
        protocol::add_capture(&f.0, &path, f.round(&path).revision, input)
            .unwrap_err()
            .code,
        2
    );
    f.capture_all(&path);
}

#[test]
fn unrelated_unlinked_round_is_not_history_even_with_a_newer_name() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let orphan = protocol::round_path(PREFIX, 99);
    fs::create_dir_all(f.0.join(orphan.parent().unwrap())).unwrap();
    fs::write(f.0.join(orphan), b"unlinked future file").unwrap();
    let (_, manifest) = f.packet(&path);
    assert_eq!(manifest["previousUnits"][0]["roundId"], "r1");
}

#[test]
fn changed_expectations_require_real_bound_change_record_and_export_no_links() {
    let f = Fixture::new();
    let old = f.reviewed(Some(&["U1"]));
    let from = f.round(&old).target_fingerprint;
    let record = "- First: user accepted icon scale 20px. [Decision](private-report.md)\n- Second: comparison uses the accepted token.\n<script>PRIVATE_HTML</script>\n```\nPRIVATE_CODE\n```\n";
    let target = TEXT
        .replace("Accepted icon scale 24px.", "Accepted icon scale 20px.")
        .replace("R1 established.", record);
    fs::write(f.0.join(TARGET), &target).unwrap();
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let to = f.round(&path).target_fingerprint;
    assert_eq!(
        protocol::packet::create(&f.0, &path, None)
            .unwrap_err()
            .code,
        2
    );
    let good = protocol::history::TargetChange {
        from_fingerprint: from,
        to_fingerprint: to,
        units: vec!["U1".into()],
        source: FileEvidence {
            path: TARGET.into(),
            sha256: hash(target.as_bytes()),
        },
    };
    for case in 0..5 {
        let mut bad = good.clone();
        let expected = match case {
            0 => {
                bad.source.sha256 = "0".repeat(64);
                3
            }
            1 => {
                bad.from_fingerprint = "0".repeat(64);
                2
            }
            2 => {
                bad.to_fingerprint = "0".repeat(64);
                2
            }
            3 => {
                bad.units = vec!["U2".into()];
                2
            }
            _ => {
                bad.units.push("U1".into());
                2
            }
        };
        assert_eq!(
            protocol::packet::create_with_changes(&f.0, &path, None, vec![bad])
                .unwrap_err()
                .code,
            expected
        );
    }
    let packet = protocol::packet::create_with_changes(&f.0, &path, None, vec![good]).unwrap();
    let manifest: Value =
        serde_json::from_slice(&fs::read(f.0.join(&packet.manifest.path)).unwrap()).unwrap();
    let change = &manifest["previousUnits"][0]["targetChanges"][0];
    assert_eq!(change["sourceSha256"], hash(target.as_bytes()));
    let directory =
        f.0.join(packet.manifest.path)
            .parent()
            .unwrap()
            .to_path_buf();
    let text =
        fs::read_to_string(directory.join(change["record"]["path"].as_str().unwrap())).unwrap();
    assert!(text.contains("user accepted icon scale 20px"));
    assert!(text.find("First:").unwrap() < text.find("Second:").unwrap());
    for private in [
        "private-report.md",
        "PRIVATE_HTML",
        "PRIVATE_CODE",
        "Open the real app",
        "Accepted icon scale 20px.",
        "U2 Editor",
    ] {
        assert!(!text.contains(private));
    }
}

fn reject_wrong_change_source(source: &str) {
    let f = Fixture::new();
    let old = f.reviewed(Some(&["U1"]));
    let from = f.round(&old).target_fingerprint;
    fs::write(
        f.0.join(TARGET),
        TEXT.replace("Accepted icon scale 24px.", "Accepted icon scale 20px.")
            .replace("R1 established.", "User accepted the new icon scale."),
    )
    .unwrap();
    if source != "docs/orders.ui-review.md" {
        fs::create_dir_all(f.0.join(source).parent().unwrap()).unwrap();
        fs::write(
            f.0.join(source),
            b"PRIVATE_MATERIAL masquerading as a target change",
        )
        .unwrap();
    }
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let change = protocol::history::TargetChange {
        from_fingerprint: from,
        to_fingerprint: f.round(&path).target_fingerprint,
        units: vec!["U1".into()],
        source: FileEvidence {
            path: source.into(),
            sha256: hash(&fs::read(f.0.join(source)).unwrap()),
        },
    };
    assert_eq!(
        protocol::packet::create_with_changes(&f.0, &path, None, vec![change])
            .expect_err("Private change source was exported")
            .code,
        2
    );
}

#[test]
fn target_change_source_cannot_be_a_full_report() {
    reject_wrong_change_source("docs/orders.ui-review.md");
}

#[test]
fn target_change_source_cannot_be_a_fix_record() {
    reject_wrong_change_source("docs/orders.ui-review.applied.md");
}

#[test]
fn target_change_source_cannot_be_source_code() {
    reject_wrong_change_source("src/private.rs");
}

#[test]
fn target_change_requires_a_unique_nonempty_real_second_level_section() {
    for suffix in [
        "",
        "## 变更记录\n\n",
        "## 变更记录\nFirst\n\n## 变更记录\nSecond\n",
        "```md\n## 变更记录\nFake\n```\n> ## 变更记录\n> Fake\n",
        "## 变更记录\n```\nOnly code\n```\n",
    ] {
        let f = Fixture::new();
        let old = f.reviewed(Some(&["U1"]));
        let from = f.round(&old).target_fingerprint;
        let target = TEXT
            .split("## 变更记录")
            .next()
            .unwrap()
            .replace("Accepted icon scale 24px.", "Accepted icon scale 20px.")
            + suffix;
        fs::write(f.0.join(TARGET), &target).unwrap();
        let path = f.begin(Some(&["U1"]));
        f.capture_all(&path);
        let change = protocol::history::TargetChange {
            from_fingerprint: from,
            to_fingerprint: f.round(&path).target_fingerprint,
            units: vec!["U1".into()],
            source: f.round(&path).target_snapshot.source,
        };
        assert_eq!(
            protocol::packet::create_with_changes(&f.0, &path, None, vec![change])
                .expect_err("Missing/ambiguous change section was exported")
                .code,
            2
        );
    }
}

#[test]
fn previous_unit_metadata_drift_is_rejected_even_when_packet_hash_is_recomputed() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let mut input = f.input(&path, "current-reviewer");
    let (packet, mut manifest) = f.packet(&path);
    manifest["previousUnits"][0]["gaps"][0]["id"] = "UI-999".into();
    let bytes = serde_json::to_vec_pretty(&manifest).unwrap();
    let digest = hash(&bytes);
    let old_directory =
        f.0.join(&packet.manifest.path)
            .parent()
            .unwrap()
            .to_path_buf();
    let directory = path.parent().unwrap().join("judge-pack").join(&digest);
    for file in manifest["files"].as_array().unwrap() {
        let name = file["path"].as_str().unwrap();
        let destination = f.0.join(directory.join(name));
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(old_directory.join(name), destination).unwrap();
    }
    fs::write(f.0.join(directory.join("manifest.json")), bytes).unwrap();
    let mut round = f.round(&path);
    round.packets.push(protocol::packet::PacketRef {
        packet_hash: digest.clone(),
        manifest: FileEvidence {
            path: directory
                .join("manifest.json")
                .to_string_lossy()
                .into_owned(),
            sha256: digest.clone(),
        },
    });
    fs::write(f.0.join(&path), serde_json::to_vec_pretty(&round).unwrap()).unwrap();
    input["packetHash"] = digest.into();
    assert_eq!(f.add(&path, input).unwrap_err().code, 4);
}

#[test]
fn old_attempt_in_a_fixed_packet_cannot_be_used_after_current_recapture() {
    let f = Fixture::new();
    f.reviewed(Some(&["U1"]));
    let path = f.begin(Some(&["U1"]));
    f.capture_all(&path);
    let old_input = f.input(&path, "current-reviewer");
    f.capture(&path, "U1", 2, false);
    assert_eq!(f.add(&path, old_input).unwrap_err().code, 3);
    let good = f.input(&path, "new-reviewer");
    assert_eq!(
        good["previous"][0]["basis"]["image"]["path"],
        "current/U1-default-2-light-a2.png"
    );
    f.add(&path, good).unwrap();
}
