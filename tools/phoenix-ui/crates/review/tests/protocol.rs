use phoenix_ui_review::{
    capture_evidence::{
        CaptureInput, FileEvidence, HumanCheck, Observation, Origin, TestedVersion, WorkingTree,
    },
    protocol::{self, BeginOptions, Cycle},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const PREFIX: &str = "docs/orders";
const TARGET: &str = "docs/orders.ui-target.md";
const TEXT: &str = "# Orders\n\n## 单元\n\n| 单元 | 必需状态 | 视口 | 主题 | 来源 |\n| --- | --- | --- | --- | --- |\n| U1 Orders | default | 2×2 | light | [Design](../DESIGN.md) |\n\n## 规范要点\n\nUse the project rules.\n\n## 采集\n\nOpen the real app.\n";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "phoenix-review-protocol-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join(TARGET), TEXT).unwrap();
        fs::write(root.join("DESIGN.md"), "Accepted design").unwrap();
        Self(root)
    }
    fn options(&self) -> BeginOptions {
        BeginOptions {
            prefix: PREFIX.into(),
            target: TARGET.into(),
            plan: None,
            scope: None,
            threshold: None,
            version: TestedVersion {
                head: None,
                working_tree: WorkingTree::Unknown,
                build_id: Some("protocol-fixture-v1".into()),
            },
        }
    }
    fn round(&self) -> PathBuf {
        protocol::round_path(PREFIX, 1)
    }
    fn input(&self, attempt: u32, missing: bool) -> CaptureInput {
        self.input_with_options(attempt, missing, self.options())
    }
    fn input_with_options(
        &self,
        attempt: u32,
        missing: bool,
        options: BeginOptions,
    ) -> CaptureInput {
        let round = protocol::begin(&self.0, options).unwrap();
        let cell = round.matrix.cells[0].clone();
        let filename = round.matrix.screenshot_name(&cell.key, attempt).unwrap();
        let image_path = self.round().parent().unwrap().join(filename);
        let bytes = png();
        fs::write(self.0.join(&image_path), &bytes).unwrap();
        CaptureInput {
            schema_version: 1,
            round_id: "r1".into(),
            cell: cell.key,
            attempt,
            observation: if missing {
                Observation::Missing {
                    reason: "App unavailable".into(),
                }
            } else {
                Observation::Captured {
                    image: FileEvidence {
                        path: image_path.to_str().unwrap().into(),
                        sha256: hash(&bytes),
                    },
                    origin: Origin::Browser,
                    tool: "test fixture (not product capture)".into(),
                    address: "http://localhost:8080/orders?token=private".into(),
                    taken_at: "2026-10-10T04:00:00Z".into(),
                    viewport: Box::new(cell.viewport),
                    tested_version: Box::new(round.tested_version),
                    interactions: vec![],
                    human_check: HumanCheck {
                        opened: true,
                        current_product: true,
                        current_version: true,
                        notes: "PNG structural fixture, not acceptance".into(),
                    },
                }
            },
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn png() -> Vec<u8> {
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255; 16]).unwrap();
    }
    output
}

const JUDGE_RAW: &[u8] = "评审原文\nU1 3 / 2 / 3 / 1；视觉失分 1；新-1 的图标偏大。\n".as_bytes();

fn judge_add(
    project: &std::path::Path,
    path: &std::path::Path,
    input: protocol::judgement::JudgeInput,
    raw: &[u8],
    mappings: Vec<protocol::judgement::GapMapping>,
) -> Result<protocol::judgement::JudgeReceipt, phoenix_ui_review::storage::StoreError> {
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(project.join(path)).unwrap()).unwrap();
    protocol::judgement::add(project, path, round.revision, input, raw, mappings)
}

fn adjudication(f: &Fixture) -> protocol::finalize::Adjudication {
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    protocol::finalize::Adjudication {
        schema_version: 1,
        cycle_id: round.cycle_id,
        round_id: round.round_id,
        machine_checks: vec![],
        main_checks: vec![],
        decisions: vec![],
        applied: vec![],
        notes: "Protocol fixture; does not prove product acceptance".into(),
    }
}

fn runtime_check(f: &Fixture, kind: protocol::finalize::CheckKind) -> FileEvidence {
    use protocol::finalize::*;
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let check = RuntimeCheck {
        schema_version: 1,
        kind,
        tool: "Explicit structural fixture, not a product measurement".into(),
        taken_at: "2026-10-10T04:05:00Z".into(),
        tested_version: round.tested_version.clone(),
        cell: round.matrix.cells[0].key.clone(),
        image: round.captures[0].frozen_image.clone().unwrap(),
        status: CheckStatus::Failed,
        notes: "Human observation fixture".into(),
        findings: vec![Finding {
            key: "page-1".into(),
            existing_issue: None,
            severity: phoenix_ui_review::policy::Severity::P0,
            description: "Save deletes another item".into(),
            expected: "Save preserves neighbors".into(),
            basis: round.target_snapshot.expectations,
            fix: "Correct the save request and verify the page".into(),
        }],
        reassessed: vec![],
    };
    let bytes = serde_json::to_vec_pretty(&check).unwrap();
    let path = "docs/runtime-check.json";
    fs::write(f.0.join(path), &bytes).unwrap();
    FileEvidence {
        path: path.into(),
        sha256: hash(&bytes),
    }
}

fn finalize(f: &Fixture) -> protocol::finalize::Response {
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    protocol::finalize::run(&f.0, &f.round(), round.revision, adjudication(f)).unwrap()
}

fn applied_fixture() -> (Fixture, protocol::applied::Input) {
    applied_fixture_with(false, false)
}

fn applied_fixture_with(plan: bool, two_gaps: bool) -> (Fixture, protocol::applied::Input) {
    use protocol::applied::*;
    let f = Fixture::new();
    let mut options = f.options();
    if plan {
        fs::write(f.0.join("docs/plan.md"), "Existing implementation plan").unwrap();
        options.plan = Some("docs/plan.md".into());
    }
    let capture = f.input_with_options(1, false, options);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let mut judge = judge_input(&f, "reviewer-a");
    let mut raw = JUDGE_RAW.to_vec();
    if two_gaps {
        let mut gap = judge.units[0].gaps[0].clone();
        gap.key = "新-2".into();
        gap.location = "Footer icon".into();
        gap.recoverable = "0.5".parse().unwrap();
        judge.units[0].gaps[0].recoverable = "0.5".parse().unwrap();
        judge.units[0].gaps.push(gap);
        raw.extend_from_slice("新-2 的页尾图标也偏大，两处各恢复 0.5 分。\n".as_bytes());
        judge.raw_sha256 = hash(&raw);
    }
    judge_add(&f.0, &f.round(), judge, &raw, vec![]).unwrap();
    finalize(&f);
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let mut capture = round.captures[0].input.clone();
    let version = TestedVersion {
        head: None,
        working_tree: WorkingTree::Unknown,
        build_id: Some("apply-structural-fixture-v2".into()),
    };
    let image_directory = PathBuf::from(format!("{PREFIX}.apply.ui-captures/r1"));
    fs::create_dir_all(f.0.join(&image_directory)).unwrap();
    if let Observation::Captured {
        image,
        tested_version,
        taken_at,
        ..
    } = &mut capture.observation
    {
        image.path = image_directory
            .join(round.matrix.screenshot_name(&capture.cell, 1).unwrap())
            .to_string_lossy()
            .into_owned();
        fs::write(f.0.join(&image.path), png()).unwrap();
        **tested_version = version.clone();
        *taken_at = "2026-10-10T05:00:00Z".into();
    }
    let code = b"// Explicit apply protocol fixture, not product UI.\n";
    fs::write(f.0.join("card.tsx"), code).unwrap();
    let input = Input {
        schema_version: 1,
        cycle_id: round.cycle_id,
        round_id: round.round_id,
        record_path: None,
        at: "2026-10-10T05:00:00Z".into(),
        tested_version: version,
        items: vec![Item {
            issue_id: "UI-01".into(),
            verification: "Page observation fixture; not acceptance evidence".into(),
            disposition: phoenix_ui_review::policy::Disposition::Modified,
            changes: vec![FileEvidence {
                path: "card.tsx".into(),
                sha256: hash(code),
            }],
            basis: vec![],
            self_check: SelfCheck::Page {
                capture: Box::new(capture),
                notes: "Explicit PNG structural fixture, not actual UI verification".into(),
            },
            notes: "Adjusted the fixture; next independent review determines resolution".into(),
        }],
        restructure_scopes: vec![],
        decisions: vec![],
        checks: vec![],
        unfinished: vec![],
    };
    (f, input)
}

#[test]
fn applied_records_changes_without_modifying_the_sealed_judgement() {
    let (f, input) = applied_fixture();
    let round_before = fs::read(f.0.join(f.round())).unwrap();
    let report = f.0.join(format!("{PREFIX}.ui-review.md"));
    let report_before = fs::read(&report).unwrap();
    let receipt = protocol::applied::record(&f.0, &f.round(), 0, input).unwrap();
    assert_eq!(receipt.revision, 1);
    assert_eq!(
        receipt.record_path,
        format!("{PREFIX}.ui-review.applied.md")
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), round_before);
    assert_eq!(fs::read(&report).unwrap(), report_before);
    let body = fs::read_to_string(f.0.join(&receipt.record_path)).unwrap();
    assert!(body.contains("UI 验收修复") && body.contains("已修改待评审"));
    assert!(!body.contains("已修复并验证"));
    let record: protocol::applied::Record =
        serde_json::from_slice(&fs::read(f.0.join(&receipt.record.path)).unwrap()).unwrap();
    assert_eq!(record.input.items[0].issue_id, "UI-01");
    assert_eq!(record.page_evidence.len(), 1);
    assert_eq!(
        record.page_evidence[0].frozen_image.sha256,
        hash(&fs::read(f.0.join(&record.page_evidence[0].frozen_image.path)).unwrap())
    );
}

#[test]
fn applied_revision_cas_preserves_unrelated_record_text_and_immutable_inputs() {
    let (f, mut input) = applied_fixture();
    let first = protocol::applied::record(&f.0, &f.round(), 0, input.clone()).unwrap();
    let original = fs::read(f.0.join(&first.record.path)).unwrap();
    let ledger = f.0.join(format!("{PREFIX}.ui-review.applied.json"));
    let before = fs::read(&ledger).unwrap();
    assert_eq!(
        protocol::applied::record(&f.0, &f.round(), 0, input.clone())
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(&ledger).unwrap(), before);
    let record_path = f.0.join(&first.record_path);
    let mut bytes = fs::read(&record_path).unwrap();
    bytes.extend_from_slice(b"\nUnrelated milestone note.\n");
    fs::write(&record_path, bytes).unwrap();
    input.items[0].notes.push_str(" Added a factual follow-up.");
    let second = protocol::applied::record(&f.0, &f.round(), 1, input).unwrap();
    assert_eq!(second.revision, 2);
    assert_eq!(fs::read(f.0.join(&first.record.path)).unwrap(), original);
    let body = fs::read_to_string(&record_path).unwrap();
    assert!(body.contains("Unrelated milestone note."));
    assert_eq!(body.matches("### r1 → 修复").count(), 1);
}

#[test]
fn applied_rejects_generated_block_edits_and_old_capture_reuse() {
    let (f, mut input) = applied_fixture();
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    input.items[0].self_check = protocol::applied::SelfCheck::Page {
        capture: Box::new(round.captures[0].input.clone()),
        notes: "Reusing the sealed old screenshot is not current fix evidence".into(),
    };
    assert_eq!(
        protocol::applied::record(&f.0, &f.round(), 0, input)
            .unwrap_err()
            .code,
        2
    );
    let (f, input) = applied_fixture();
    let receipt = protocol::applied::record(&f.0, &f.round(), 0, input.clone()).unwrap();
    let file = f.0.join(&receipt.record_path);
    let edited = fs::read_to_string(&file)
        .unwrap()
        .replace("已修改待评审", "已解决");
    fs::write(&file, &edited).unwrap();
    let ledger = f.0.join(format!("{PREFIX}.ui-review.applied.json"));
    let before = fs::read(&ledger).unwrap();
    assert_eq!(
        protocol::applied::record(&f.0, &f.round(), 1, input)
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(&ledger).unwrap(), before);
    assert_eq!(fs::read_to_string(&file).unwrap(), edited);
}

#[test]
fn applied_records_one_refactor_per_scope_and_rejects_resolved_self_grades() {
    let (f, mut input) = applied_fixture();
    input.restructure_scopes.push(protocol::applied::Refactor {
        scope: protocol::applied::RefactorScope::Unit {
            unit_id: "U1".into(),
        },
        hypothesis: "The fixture layout rule caused the repeated gap".into(),
        change: "Changed the fixture layout rule".into(),
        expected: "U1 should improve on the next independent review".into(),
    });
    protocol::applied::record(&f.0, &f.round(), 0, input.clone()).unwrap();
    assert_eq!(
        protocol::applied::record(&f.0, &f.round(), 1, input.clone())
            .unwrap_err()
            .code,
        3
    );
    let mut json = serde_json::to_value(&input).unwrap();
    json["items"][0]["disposition"] = "已修复并验证".into();
    assert!(serde_json::from_value::<protocol::applied::Input>(json).is_err());
}

#[test]
fn applied_multiple_dispositions_remain_one_markdown_table_with_evidence_links() {
    let (f, mut input) = applied_fixture_with(false, true);
    let mut second = input.items[0].clone();
    second.issue_id = "UI-02".into();
    second.notes = "The footer uses the same corrected token".into();
    input.items.push(second);
    let receipt = protocol::applied::record(&f.0, &f.round(), 0, input).unwrap();
    let body = fs::read_to_string(f.0.join(&receipt.record_path)).unwrap();
    let rows: Vec<_> = body.lines().collect();
    let first = rows
        .iter()
        .position(|row| row.starts_with("| UI-01 |"))
        .unwrap();
    assert!(
        rows[first + 1].starts_with("| UI-02 |"),
        "Notes must follow the complete table"
    );
    assert!(body.contains("[r1 报告]"));
    assert!(body.contains("[UI-01 自检]"));
    assert!(body.contains("- 用户方向：无"));
    assert!(body.contains("- 未完项：无"));
}

#[test]
fn applied_plan_record_location_is_selected_once_without_replacing_plan_text() {
    let (f, mut input) = applied_fixture_with(true, false);
    assert_eq!(
        protocol::applied::record(&f.0, &f.round(), 0, input.clone())
            .unwrap_err()
            .code,
        2
    );
    fs::create_dir_all(f.0.join("docs/plan.records")).unwrap();
    let record_path = "docs/plan.records/M2.md";
    fs::write(
        f.0.join(record_path),
        "# M2\n\nExisting milestone evidence.\n",
    )
    .unwrap();
    input.record_path = Some(record_path.into());
    let first = protocol::applied::record(&f.0, &f.round(), 0, input.clone()).unwrap();
    assert_eq!(first.record_path, record_path);
    let index_path = f.0.join(format!("{PREFIX}.ui-review.json"));
    let before = fs::read(&index_path).unwrap();
    input.record_path = Some("docs/plan.records/M3.md".into());
    assert_eq!(
        protocol::applied::record(&f.0, &f.round(), 1, input.clone())
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(&index_path).unwrap(), before);
    input.record_path = None;
    protocol::applied::record(&f.0, &f.round(), 1, input).unwrap();
    let body = fs::read_to_string(f.0.join(record_path)).unwrap();
    assert!(body.starts_with("# M2\n\nExisting milestone evidence."));
    assert_eq!(body.matches("### r1 → 修复").count(), 1);
    assert!(body.contains("(../orders.ui-review.md)"));
    assert!(!f.0.join("docs/plan.records/M3.md").exists());
}

#[test]
fn applied_not_run_preserves_the_reason_and_declined_needs_frozen_counterevidence() {
    use phoenix_ui_review::policy::Disposition;
    let (f, mut input) = applied_fixture();
    input.items[0].self_check = protocol::applied::SelfCheck::NotRun {
        reason: "The real app is unavailable; no UI result is claimed".into(),
    };
    input.items[0].changes.clear();
    input.items[0].disposition = Disposition::Declined;
    assert_eq!(
        protocol::applied::record(&f.0, &f.round(), 0, input.clone())
            .unwrap_err()
            .code,
        2
    );
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    input.items[0]
        .basis
        .push(round.target_snapshot.expectations);
    let first = protocol::applied::record(&f.0, &f.round(), 0, input.clone()).unwrap();
    let body = fs::read_to_string(f.0.join(&first.record_path)).unwrap();
    assert!(body.contains("未运行：The real app is unavailable"));
    assert!(body.contains("不采纳"));
    let original = fs::read(f.0.join(&first.record.path)).unwrap();
    input.items[0].disposition = Disposition::AwaitingDecision;
    input.items[0].basis.clear();
    protocol::applied::record(&f.0, &f.round(), 1, input).unwrap();
    assert_eq!(fs::read(f.0.join(&first.record.path)).unwrap(), original);
}

#[test]
fn finalize_preserves_whole_reviewer_score_and_seals_typed_results() {
    use protocol::finalize::*;
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    judge_add(&f.0, &f.round(), input.clone(), JUDGE_RAW, vec![]).unwrap();
    let Response::Finalized {
        result,
        report,
        outcome,
        ..
    } = finalize(&f)
    else {
        panic!("Expected a finalized result")
    };
    assert_eq!(
        outcome.verdict,
        Some(phoenix_ui_review::policy::Verdict::Passed)
    );
    let result: ResultRecord =
        serde_json::from_slice(&fs::read(f.0.join(result.path)).unwrap()).unwrap();
    assert_eq!(
        result.units[0].selected.as_ref().unwrap().scores,
        input.units[0].scores
    );
    assert_eq!(result.units[0].gaps[0].id, "UI-01");
    assert_eq!(result.units[0].score, Some("9".parse().unwrap()));
    assert!(fs::read_to_string(f.0.join(report.path))
        .unwrap()
        .contains("通过验收"));
    let original = fs::read(f.0.join(f.round())).unwrap();
    assert_eq!(
        protocol::finalize::run(&f.0, &f.round(), 4, adjudication(&f))
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), original);
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap().round_id, "r2");
}

#[test]
fn finalize_missing_evidence_is_sealed_without_inventing_a_score() {
    use protocol::finalize::*;
    let f = Fixture::new();
    protocol::begin(&f.0, f.options()).unwrap();
    let Response::Finalized {
        result, outcome, ..
    } = finalize(&f)
    else {
        panic!("Missing evidence can be sealed")
    };
    assert_eq!(
        outcome.verdict,
        Some(phoenix_ui_review::policy::Verdict::NeedEvidence)
    );
    assert_eq!(outcome.scoring.score, None);
    let result: ResultRecord =
        serde_json::from_slice(&fs::read(f.0.join(result.path)).unwrap()).unwrap();
    assert!(result.units[0].selected.is_none());
    assert!(result.units[0].unreviewed_reason.is_some());
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap().round_id, "r2");
}

#[test]
fn finalize_confirmation_keeps_draft_then_selects_the_lower_whole_group() {
    use protocol::finalize::*;
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let mut a = judge_input(&f, "reviewer-a");
    a.units[0].scores = ["2.8", "1.6", "3", "1"].map(|s| s.parse().unwrap());
    a.units[0].gaps[0].recoverable = "1.6".parse().unwrap();
    judge_add(&f.0, &f.round(), a, JUDGE_RAW, vec![]).unwrap();
    let before = fs::read(f.0.join(f.round())).unwrap();
    assert!(matches!(
        finalize(&f),
        Response::ConfirmationRequired { .. }
    ));
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), before);
    assert!(!f
        .0
        .join(f.round().parent().unwrap().join("result.json"))
        .exists());
    let mut b = judge_input(&f, "reviewer-b");
    b.units[0].scores = ["3", "1.2", "3", "1"].map(|s| s.parse().unwrap());
    b.units[0].gaps[0].recoverable = "1.8".parse().unwrap();
    judge_add(
        &f.0,
        &f.round(),
        b.clone(),
        JUDGE_RAW,
        vec![protocol::judgement::GapMapping {
            key: "新-1".into(),
            id: "UI-01".into(),
            basis: "Same observed icon".into(),
        }],
    )
    .unwrap();
    let Response::Finalized {
        result, outcome, ..
    } = finalize(&f)
    else {
        panic!("Two independent reviewers confirm")
    };
    assert_eq!(outcome.scoring.score, Some("8.2".parse().unwrap()));
    let result: ResultRecord =
        serde_json::from_slice(&fs::read(f.0.join(result.path)).unwrap()).unwrap();
    assert_eq!(
        result.units[0].selected_reviewer.as_deref(),
        Some("reviewer-b")
    );
    assert_eq!(
        result.units[0].selected.as_ref().unwrap().scores,
        b.units[0].scores
    );
    assert_eq!(result.units[0].gaps.len(), 1);
    assert_eq!(result.units[0].gaps[0].details.len(), 2);
}

#[test]
fn finalize_rejects_revision_and_adjudication_binding_without_writes() {
    let f = Fixture::new();
    protocol::begin(&f.0, f.options()).unwrap();
    let before = fs::read(f.0.join(f.round())).unwrap();
    assert_eq!(
        protocol::finalize::run(&f.0, &f.round(), 0, adjudication(&f))
            .unwrap_err()
            .code,
        3
    );
    let mut input = adjudication(&f);
    input.round_id = "r99".into();
    assert_eq!(
        protocol::finalize::run(&f.0, &f.round(), 1, input)
            .unwrap_err()
            .code,
        2
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), before);
}

#[test]
fn finalize_cannot_use_a_changed_raw_judgement() {
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let receipt = judge_add(&f.0, &f.round(), input, JUDGE_RAW, vec![]).unwrap();
    let record: protocol::judgement::JudgeRecord =
        serde_json::from_slice(&fs::read(f.0.join(receipt.file.path)).unwrap()).unwrap();
    fs::write(f.0.join(record.raw.path), "changed").unwrap();
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    assert_eq!(
        protocol::finalize::run(&f.0, &f.round(), round.revision, adjudication(&f))
            .unwrap_err()
            .code,
        3
    );
}

#[test]
fn finalize_page_measurement_gates_acceptance_without_changing_scores() {
    use protocol::finalize::*;
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    judge_add(&f.0, &f.round(), input, JUDGE_RAW, vec![]).unwrap();
    let mut input = adjudication(&f);
    input
        .machine_checks
        .push(runtime_check(&f, CheckKind::Page));
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let Response::Finalized {
        result, outcome, ..
    } = run(&f.0, &f.round(), round.revision, input).unwrap()
    else {
        panic!("P0 blocks preliminary pass")
    };
    assert_eq!(
        outcome.verdict,
        Some(phoenix_ui_review::policy::Verdict::NeedModification)
    );
    assert_eq!(outcome.scoring.score, Some("9".parse().unwrap()));
    let result: ResultRecord =
        serde_json::from_slice(&fs::read(f.0.join(result.path)).unwrap()).unwrap();
    assert_eq!(result.units[0].gaps.len(), 2);
    assert_eq!(result.units[0].gaps[1].id, "UI-02");
    assert!(result.units[0].gaps[1].details.is_empty());
}

#[test]
fn finalize_source_finding_is_retained_without_becoming_a_page_gap() {
    use protocol::finalize::*;
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    judge_add(&f.0, &f.round(), input, JUDGE_RAW, vec![]).unwrap();
    let mut input = adjudication(&f);
    input
        .machine_checks
        .push(runtime_check(&f, CheckKind::Source));
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let Response::Finalized {
        result, outcome, ..
    } = run(&f.0, &f.round(), round.revision, input).unwrap()
    else {
        panic!("Source findings cannot impose visual acceptance")
    };
    assert_eq!(
        outcome.verdict,
        Some(phoenix_ui_review::policy::Verdict::Passed)
    );
    let result: ResultRecord =
        serde_json::from_slice(&fs::read(f.0.join(result.path)).unwrap()).unwrap();
    assert_eq!(result.units[0].gaps.len(), 1);
    assert_eq!(result.adjudication.machine_checks.len(), 1);
}

#[test]
fn finalize_current_rereview_keeps_lower_whole_group_and_gap_union() {
    use protocol::{finalize::*, judgement::*};
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let mut a = judge_input(&f, "reviewer-a");
    let cell = a.units[0].gaps[0].cell.clone();
    a.units[0].unseen_cells.push(UnseenCell {
        cell,
        reason: "Small icon could not be read".into(),
    });
    judge_add(&f.0, &f.round(), a, JUDGE_RAW, vec![]).unwrap();
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let mut capture = round.captures[0].input.clone();
    capture.attempt = 2;
    if let Observation::Captured { image, .. } = &mut capture.observation {
        image.path = f
            .round()
            .parent()
            .unwrap()
            .join(round.matrix.screenshot_name(&capture.cell, 2).unwrap())
            .to_string_lossy()
            .into_owned();
        fs::write(f.0.join(&image.path), png()).unwrap();
    }
    protocol::add_capture(&f.0, &f.round(), round.revision, capture).unwrap();
    let mut b = judge_input(&f, "reviewer-b");
    b.units[0].scores[1] = "1.7".parse().unwrap();
    b.units[0].gaps[0].recoverable = "1.3".parse().unwrap();
    judge_add(
        &f.0,
        &f.round(),
        b,
        JUDGE_RAW,
        vec![GapMapping {
            key: "新-1".into(),
            id: "UI-01".into(),
            basis: "Same icon, now readable".into(),
        }],
    )
    .unwrap();
    let Response::Finalized {
        result, outcome, ..
    } = finalize(&f)
    else {
        panic!("Current rereview can complete a draft")
    };
    assert_eq!(outcome.scoring.score, Some("8.7".parse().unwrap()));
    assert_eq!(
        outcome.verdict,
        Some(phoenix_ui_review::policy::Verdict::Passed)
    );
    let result: ResultRecord =
        serde_json::from_slice(&fs::read(f.0.join(result.path)).unwrap()).unwrap();
    assert_eq!(result.units[0].gaps[0].details.len(), 2);
    assert_eq!(
        result.units[0].selected_reviewer.as_deref(),
        Some("reviewer-b")
    );
    assert!(protocol::begin(&f.0, f.options()).is_ok());
}

#[test]
fn finalize_unknown_typed_result_cannot_open_the_next_round() {
    use protocol::finalize::*;
    let f = Fixture::new();
    protocol::begin(&f.0, f.options()).unwrap();
    let Response::Finalized { result, .. } = finalize(&f) else {
        panic!()
    };
    let path = f.0.join(&result.path);
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["futureAuthority"] = "passed".into();
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(path, &bytes).unwrap();
    let mut round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    round.finalization.as_mut().unwrap().result.sha256 = hash(&bytes);
    fs::write(
        f.0.join(f.round()),
        serde_json::to_vec_pretty(&round).unwrap(),
    )
    .unwrap();
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap_err().code, 4);
    assert!(!f.0.join(protocol::round_path(PREFIX, 2)).exists());
}

fn judge_input(f: &Fixture, reviewer: &str) -> protocol::judgement::JudgeInput {
    use protocol::judgement::*;
    let packet = protocol::packet::create(&f.0, &f.round(), None).unwrap();
    let manifest: protocol::packet::Manifest =
        serde_json::from_slice(&fs::read(f.0.join(&packet.manifest.path)).unwrap()).unwrap();
    let image = manifest
        .captures
        .iter()
        .find_map(|capture| match &capture.input.observation {
            Observation::Captured { image, .. } if capture.current => Some(image.clone()),
            _ => None,
        })
        .unwrap();
    let basis = manifest
        .files
        .iter()
        .find(|file| file.path == "expectations.txt")
        .unwrap()
        .clone();
    JudgeInput {
        schema_version: 1,
        cycle_id: manifest.cycle_id,
        round_id: "r1".into(),
        packet_hash: packet.packet_hash,
        reviewer_id: reviewer.into(),
        raw_sha256: hash(JUDGE_RAW),
        provenance: Provenance {
            fresh_context: true,
            read_only: true,
            inherited_model: true,
            opened_images: manifest
                .files
                .into_iter()
                .filter(|file| file.path.ends_with(".png"))
                .collect(),
            notes: "Declared fixture provenance, not visual acceptance".into(),
        },
        conversion: Conversion::ReviewerStructured,
        units: vec![UnitInput {
            unit_id: "U1".into(),
            scores: ["3", "2", "3", "1"].map(|score| score.parse().unwrap()),
            reasons: [
                "Layout matches",
                "Icon scale differs",
                "State matches",
                "Copy matches",
            ]
            .map(String::from),
            gaps: vec![GapInput {
                key: "新-1".into(),
                cell: manifest.captures[0].input.cell.clone(),
                image,
                dimension: "视觉".into(),
                location: "Header icon".into(),
                current: "Icon is 32px".into(),
                expected: "Icon is 24px".into(),
                basis,
                basis_location: "Accepted icon scale".into(),
                fix: "Use the accepted icon token".into(),
                severity: phoenix_ui_review::policy::Severity::P2,
                recoverable: "1".parse().unwrap(),
            }],
            suggestions: vec!["Optional copy alternative, no deduction".into()],
            unseen_cells: vec![],
        }],
        unreviewed_units: vec![],
        previous: vec![],
        summary: "U1 is the lowest unit; align the icon scale".into(),
    }
}

#[test]
fn judge_preserves_raw_scores_and_allocates_issue_ids_transactionally() {
    let f = Fixture::new();
    let input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let result = judge_add(&f.0, &f.round(), input.clone(), JUDGE_RAW, vec![]).unwrap();
    assert_eq!(result.assignments[0].id, "UI-01");
    let record: protocol::judgement::JudgeRecord =
        serde_json::from_slice(&fs::read(f.0.join(&result.file.path)).unwrap()).unwrap();
    assert_eq!(fs::read(f.0.join(&record.raw.path)).unwrap(), JUDGE_RAW);
    assert_eq!(record.raw.sha256, hash(JUDGE_RAW));
    assert_eq!(record.input.units[0].scores, input.units[0].scores);
    let cycle: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.join(format!("{PREFIX}.ui-review.json"))).unwrap())
            .unwrap();
    assert_eq!(cycle["nextIssueId"], 2);
    assert_eq!(cycle["issues"][0]["id"], "UI-01");
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    assert_eq!(round.judges.len(), 1);
}

#[test]
fn judge_rejects_wrong_packet_images_cells_basis_or_provenance_without_writes() {
    let f = Fixture::new();
    let input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let before = fs::read(f.0.join(f.round())).unwrap();
    for n in 0..12 {
        let mut bad = input.clone();
        match n {
            0 => bad.packet_hash = "0".repeat(64),
            1 => bad.units[0].unit_id = "U999".into(),
            2 => bad.units[0].gaps[0].image.sha256 = "0".repeat(64),
            3 => bad.units[0].gaps[0].cell.state_id = "missing".into(),
            4 => bad.units[0].gaps[0].basis = bad.units[0].gaps[0].image.clone(),
            5 => bad.units[0].gaps[0].dimension = "invented".into(),
            6 => bad.provenance.fresh_context = false,
            7 => bad.provenance.opened_images.clear(),
            8 => bad.reviewer_id = "../outside".into(),
            9 => bad.raw_sha256 = "0".repeat(64),
            10 => bad.units[0].reasons[2].clear(),
            _ => bad.previous.push(protocol::judgement::OldAssessment {
                id: "UI-99".into(),
                status: phoenix_ui_review::policy::GapStatus::Resolved,
                basis: "Invented previous issue".into(),
            }),
        }
        assert!(
            judge_add(&f.0, &f.round(), bad, JUDGE_RAW, vec![]).is_err(),
            "invalid input {n}"
        );
        assert_eq!(fs::read(f.0.join(f.round())).unwrap(), before);
    }
}

#[test]
fn judge_uses_exact_loss_accounting_and_dimension_limits() {
    let f = Fixture::new();
    let input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let mut bad = input.clone();
    bad.units[0].gaps[0].recoverable = "0.499999999999999999999999999999999999".parse().unwrap();
    assert_eq!(
        judge_add(&f.0, &f.round(), bad, JUDGE_RAW, vec![])
            .unwrap_err()
            .code,
        2
    );
    let mut bad = input.clone();
    bad.units[0].scores[0] = "3.01".parse().unwrap();
    assert_eq!(
        judge_add(&f.0, &f.round(), bad, JUDGE_RAW, vec![])
            .unwrap_err()
            .code,
        2
    );
    let mut boundary = input;
    boundary.units[0].gaps[0].recoverable = "0.5".parse().unwrap();
    judge_add(&f.0, &f.round(), boundary, JUDGE_RAW, vec![]).unwrap();
}

#[test]
fn judge_host_conversion_requires_each_unit_gap_and_original_raw_hash() {
    use protocol::judgement::Conversion;
    let f = Fixture::new();
    let input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input).unwrap();
    let mut input = judge_input(&f, "reviewer-a");
    input.conversion = Conversion::HostVerified {
        raw_sha256: hash(JUDGE_RAW),
        checked_units: vec!["U1".into()],
        checked_gaps: vec![],
        notes: "Checked original prose".into(),
    };
    assert_eq!(
        judge_add(&f.0, &f.round(), input.clone(), JUDGE_RAW, vec![])
            .unwrap_err()
            .code,
        2
    );
    input.conversion = Conversion::HostVerified {
        raw_sha256: hash(JUDGE_RAW),
        checked_units: vec!["U1".into()],
        checked_gaps: vec!["新-1".into()],
        notes: "Each raw score, reason and gap checked".into(),
    };
    judge_add(&f.0, &f.round(), input, JUDGE_RAW, vec![]).unwrap();
}

#[test]
fn judge_exact_replay_is_idempotent_and_raw_drift_blocks_reuse() {
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let result = judge_add(&f.0, &f.round(), input.clone(), JUDGE_RAW, vec![]).unwrap();
    let before = fs::read(f.0.join(f.round())).unwrap();
    let replay = judge_add(&f.0, &f.round(), input.clone(), JUDGE_RAW, vec![]).unwrap();
    assert_eq!(replay.file, result.file);
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), before);
    let record: protocol::judgement::JudgeRecord =
        serde_json::from_slice(&fs::read(f.0.join(&result.file.path)).unwrap()).unwrap();
    fs::write(f.0.join(record.raw.path), "Changed raw conclusion").unwrap();
    assert_eq!(
        judge_add(&f.0, &f.round(), input, JUDGE_RAW, vec![])
            .unwrap_err()
            .code,
        3
    );
}

#[test]
fn judge_confirmation_keeps_both_inputs_and_explicit_gap_mapping() {
    use protocol::judgement::*;
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let a = judge_input(&f, "reviewer-a");
    let first = judge_add(&f.0, &f.round(), a, JUDGE_RAW, vec![]).unwrap();
    let first_bytes = fs::read(f.0.join(&first.file.path)).unwrap();
    let mut b = judge_input(&f, "reviewer-b");
    b.units[0].gaps[0].recoverable = "0.6".parse().unwrap();
    let mut extra = b.units[0].gaps[0].clone();
    extra.key = "新-2".into();
    extra.location = "Footer icon".into();
    extra.recoverable = "0.4".parse().unwrap();
    b.units[0].gaps.push(extra);
    let mapping = GapMapping {
        key: "新-1".into(),
        id: "UI-01".into(),
        basis: "Same header icon in the current screenshot".into(),
    };
    let result = judge_add(&f.0, &f.round(), b, JUDGE_RAW, vec![mapping]).unwrap();
    assert_eq!(
        result
            .assignments
            .iter()
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>(),
        ["UI-01", "UI-02"]
    );
    assert_eq!(fs::read(f.0.join(&first.file.path)).unwrap(), first_bytes);
}

#[test]
fn judge_missing_unit_is_rejected_and_registered_units_cannot_be_rescored() {
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let mut partial = input.clone();
    partial.units.clear();
    partial.unreviewed_units = vec![protocol::packet::UnreviewedUnit {
        unit_id: "U1".into(),
        reason: "Response stopped before this unit".into(),
    }];
    let before = fs::read(f.0.join(f.round())).unwrap();
    assert_eq!(
        judge_add(&f.0, &f.round(), partial, JUDGE_RAW, vec![])
            .unwrap_err()
            .code,
        2
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), before);
    judge_add(&f.0, &f.round(), input.clone(), JUDGE_RAW, vec![]).unwrap();
    let mut changed = input;
    changed.units[0].scores[1] = "1.9".parse().unwrap();
    assert_eq!(
        judge_add(&f.0, &f.round(), changed, JUDGE_RAW, vec![])
            .unwrap_err()
            .code,
        3
    );
}

#[test]
fn judge_requires_the_callers_exact_round_revision() {
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let before = fs::read(f.0.join(f.round())).unwrap();
    assert_eq!(
        protocol::judgement::add(
            &f.0,
            &f.round(),
            round.revision - 1,
            input,
            JUDGE_RAW,
            vec![]
        )
        .unwrap_err()
        .code,
        3
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), before);
}

#[test]
fn judge_rejects_stale_packet_after_current_capture_changes() {
    let f = Fixture::new();
    let capture = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, capture).unwrap();
    let input = judge_input(&f, "reviewer-a");
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let mut capture = round.captures[0].input.clone();
    capture.attempt = 2;
    if let Observation::Captured { image, .. } = &mut capture.observation {
        image.path = f
            .round()
            .parent()
            .unwrap()
            .join(round.matrix.screenshot_name(&capture.cell, 2).unwrap())
            .to_string_lossy()
            .into_owned();
        fs::write(f.0.join(&image.path), png()).unwrap();
    }
    protocol::add_capture(&f.0, &f.round(), round.revision, capture).unwrap();
    assert_eq!(
        judge_add(&f.0, &f.round(), input, JUDGE_RAW, vec![])
            .unwrap_err()
            .code,
        3
    );
}

fn packet_text(f: &Fixture, packet: &protocol::packet::PacketRef) -> String {
    let directory =
        f.0.join(&packet.manifest.path)
            .parent()
            .unwrap()
            .to_path_buf();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.join(&packet.manifest.path)).unwrap()).unwrap();
    let mut output = serde_json::to_string(&manifest).unwrap();
    for file in manifest["files"].as_array().unwrap() {
        let bytes = fs::read(directory.join(file["path"].as_str().unwrap())).unwrap();
        output.push_str(&String::from_utf8_lossy(&bytes));
    }
    output
}

#[test]
fn packet_whitelist_preserves_expectations_images_and_provenance_without_private_inputs() {
    let f = Fixture::new();
    fs::create_dir_all(f.0.join("src")).unwrap();
    fs::write(f.0.join("src/private.rs"), "PRIVATE_SOURCE_BODY").unwrap();
    fs::write(f.0.join("target.png"), png()).unwrap();
    fs::write(f.0.join("DESIGN.md"), "Accepted type scale 16/24. [private](src/private.rs)\n<script>PRIVATE_HTML</script>\n```rust\nPRIVATE_CODE_EXAMPLE\n```\n").unwrap();
    fs::write(f.0.join(TARGET), TEXT.replace("Use the project rules.", "Use the project rules. ![Goal](../target.png) [Code](../src/private.rs) [Site](https://private.example/report)\n")).unwrap();
    let input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input).unwrap();
    let packet = protocol::packet::create(&f.0, &f.round(), None).unwrap();
    let text = packet_text(&f, &packet);
    assert!(text.contains("Accepted type scale 16/24"));
    assert!(text.contains("currentProduct"));
    assert!(text.contains("localhost:8080/orders"));
    for forbidden in [
        "PRIVATE_SOURCE_BODY",
        "PRIVATE_HTML",
        "PRIVATE_CODE_EXAMPLE",
        "private.example",
        "src/private.rs",
        "threshold",
        "machineChecks",
        "judge-pack/",
        "token=",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    let directory =
        f.0.join(&packet.manifest.path)
            .parent()
            .unwrap()
            .to_path_buf();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.join(&packet.manifest.path)).unwrap()).unwrap();
    assert_eq!(manifest["units"], serde_json::json!(["U1"]));
    let files = manifest["files"].as_array().unwrap();
    assert_eq!(
        files
            .iter()
            .filter(|file| file["path"].as_str().unwrap().ends_with(".png"))
            .count(),
        2
    );
    for file in files {
        let p = file["path"].as_str().unwrap();
        assert!(!p.contains("..") && !p.starts_with('/'));
        assert_eq!(
            hash(&fs::read(directory.join(p)).unwrap()),
            file["sha256"].as_str().unwrap()
        );
    }
}

#[test]
fn packet_partial_evidence_lists_unreviewed_units_without_invented_scores() {
    let f = Fixture::new();
    fs::write(
        f.0.join(TARGET),
        TEXT.replace(
            "| U1 Orders",
            "| U2 Summary | default | 2×2 | light | none |\n| U1 Orders",
        ),
    )
    .unwrap();
    let mut input = f.input(1, false);
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    input.cell = round
        .matrix
        .cells
        .iter()
        .find(|cell| cell.key.unit_id == "U1")
        .unwrap()
        .key
        .clone();
    if let Observation::Captured { image, .. } = &mut input.observation {
        let filename = round.matrix.screenshot_name(&input.cell, 1).unwrap();
        image.path = f
            .round()
            .parent()
            .unwrap()
            .join(filename)
            .to_string_lossy()
            .into_owned();
        fs::write(f.0.join(&image.path), png()).unwrap();
    }
    protocol::add_capture(&f.0, &f.round(), 1, input).unwrap();
    let packet = protocol::packet::create(&f.0, &f.round(), None).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(f.0.join(&packet.manifest.path)).unwrap()).unwrap();
    assert_eq!(manifest["units"], serde_json::json!(["U1"]));
    assert_eq!(manifest["unreviewedUnits"][0]["unitId"], "U2");
    assert!(!serde_json::to_string(&manifest).unwrap().contains("scores"));
    assert_eq!(
        protocol::packet::create(&f.0, &f.round(), Some(vec!["U2".into()]))
            .unwrap_err()
            .code,
        2
    );
    assert_eq!(
        protocol::packet::create(&f.0, &f.round(), Some(vec!["unknown".into()]))
            .unwrap_err()
            .code,
        2
    );
}

#[test]
fn packet_recapture_uses_current_attempt_and_retains_the_previous_fixed_packet() {
    let f = Fixture::new();
    let input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input.clone()).unwrap();
    let old = protocol::packet::create(&f.0, &f.round(), None).unwrap();
    let old_bytes = fs::read(f.0.join(&old.manifest.path)).unwrap();
    let round: protocol::Round =
        serde_json::from_slice(&fs::read(f.0.join(f.round())).unwrap()).unwrap();
    let mut missing = input.clone();
    missing.attempt = 2;
    missing.observation = Observation::Missing {
        reason: "Blurred; recapture required".into(),
    };
    let round = protocol::add_capture(&f.0, &f.round(), round.revision, missing).unwrap();
    assert_eq!(
        protocol::packet::create(&f.0, &f.round(), None)
            .unwrap_err()
            .code,
        2
    );
    let mut recapture = input;
    recapture.attempt = 3;
    if let Observation::Captured { image, .. } = &mut recapture.observation {
        image.path = f
            .round()
            .parent()
            .unwrap()
            .join(round.matrix.screenshot_name(&recapture.cell, 3).unwrap())
            .to_string_lossy()
            .into_owned();
        fs::write(f.0.join(&image.path), png()).unwrap();
    }
    protocol::add_capture(&f.0, &f.round(), round.revision, recapture).unwrap();
    let new = protocol::packet::create(&f.0, &f.round(), None).unwrap();
    assert_ne!(old.packet_hash, new.packet_hash);
    assert_eq!(fs::read(f.0.join(&old.manifest.path)).unwrap(), old_bytes);
    let text = packet_text(&f, &new);
    assert!(text.contains("Blurred; recapture required"));
    assert!(text.contains("\"current\":false") && text.contains("\"current\":true"));
}

#[test]
fn packet_creation_is_idempotent_but_detects_modified_exports() {
    let f = Fixture::new();
    let input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input).unwrap();
    let first = protocol::packet::create(&f.0, &f.round(), None).unwrap();
    let state = fs::read(f.0.join(f.round())).unwrap();
    let repeat = protocol::packet::create(&f.0, &f.round(), None).unwrap();
    assert_eq!(first.packet_hash, repeat.packet_hash);
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), state);
    let directory =
        f.0.join(&first.manifest.path)
            .parent()
            .unwrap()
            .to_path_buf();
    fs::write(directory.join("expectations.txt"), "tampered").unwrap();
    assert_eq!(
        protocol::packet::create(&f.0, &f.round(), None)
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), state);
}

#[test]
fn begin_freezes_expectations_assets_and_matrix_without_editing_sources() {
    let f = Fixture::new();
    let round = protocol::begin(&f.0, f.options()).unwrap();
    assert_eq!(round.round_id, "r1");
    assert_eq!(round.revision, 1);
    assert!(round.full);
    assert_eq!(round.threshold.to_string(), "8");
    assert_eq!(round.matrix.cells.len(), 1);
    assert_eq!(fs::read_to_string(f.0.join(TARGET)).unwrap(), TEXT);
    assert_eq!(
        fs::read_to_string(f.0.join("DESIGN.md")).unwrap(),
        "Accepted design"
    );
    let frozen = fs::read_to_string(f.0.join(&round.target_snapshot.expectations.path)).unwrap();
    assert!(frozen.contains("规范要点"));
    assert!(!frozen.contains("Open the real app"));
    let attachment = round.target_snapshot.attachments[0]
        .frozen
        .as_ref()
        .unwrap();
    assert_eq!(
        fs::read_to_string(f.0.join(&attachment.path)).unwrap(),
        "Accepted design"
    );
    let cycle: Cycle =
        serde_json::from_slice(&fs::read(f.0.join(format!("{PREFIX}.ui-review.json"))).unwrap())
            .unwrap();
    assert_eq!(cycle.mode, "managed");
    assert_eq!(cycle.rounds[0].path, f.round().to_str().unwrap());
    assert_eq!(cycle.next_issue_id, 1);
    assert_eq!(
        cycle.record_path.as_deref(),
        Some("docs/orders.ui-review.applied.md")
    );
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap_err().code, 3);
}

#[test]
fn explicit_scope_never_self_promotes_to_full_and_plan_record_is_selected_later() {
    let f = Fixture::new();
    fs::write(
        f.0.join("docs/plan.md"),
        "A plan with existing record policy",
    )
    .unwrap();
    let mut options = f.options();
    options.scope = Some(vec!["U1".into()]);
    options.plan = Some("docs/plan.md".into());
    options.threshold = Some("8.000000000000000000000000000000000001".parse().unwrap());
    let round = protocol::begin(&f.0, options).unwrap();
    assert!(!round.full);
    assert_eq!(
        round.threshold.to_string(),
        "8.000000000000000000000000000000000001"
    );
    let cycle: Cycle =
        serde_json::from_slice(&fs::read(f.0.join(format!("{PREFIX}.ui-review.json"))).unwrap())
            .unwrap();
    assert!(cycle.record_path.is_none());
}

#[test]
fn begin_rejects_manual_history_unknown_schema_and_paths_without_resetting_them() {
    let f = Fixture::new();
    let manual = f.0.join(format!("{PREFIX}.ui-review.md"));
    fs::write(&manual, "Manual historical report").unwrap();
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap_err().code, 3);
    assert_eq!(
        fs::read_to_string(&manual).unwrap(),
        "Manual historical report"
    );
    assert!(!f.0.join(f.round()).exists());
    fs::remove_file(manual).unwrap();
    let index = f.0.join(format!("{PREFIX}.ui-review.json"));
    fs::write(&index, br#"{"schemaVersion":99,"revision":1}"#).unwrap();
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap_err().code, 4);
    assert_eq!(
        fs::read(&index).unwrap(),
        br#"{"schemaVersion":99,"revision":1}"#
    );
    let mut opts = f.options();
    opts.prefix = "../outside".into();
    assert_eq!(protocol::begin(&f.0, opts).unwrap_err().code, 2);
}

#[test]
fn capture_persistence_is_idempotent_freezes_png_and_redacts_query_credentials() {
    let f = Fixture::new();
    let input = f.input(1, false);
    let round = protocol::add_capture(&f.0, &f.round(), 1, input.clone()).unwrap();
    assert_eq!(round.revision, 2);
    assert_eq!(round.captures.len(), 1);
    let frozen = round.captures[0].frozen_image.as_ref().unwrap();
    assert_eq!(
        hash(&fs::read(f.0.join(&frozen.path)).unwrap()),
        frozen.sha256
    );
    let original = fs::read(f.0.join(f.round())).unwrap();
    let text = String::from_utf8(original.clone()).unwrap();
    assert!(!text.contains("private"));
    assert!(!text.contains("token="));
    let capture_md = f.round().parent().unwrap().join("capture.md");
    let md = fs::read_to_string(f.0.join(capture_md)).unwrap();
    assert!(md.contains("localhost:8080/orders"));
    assert!(!md.contains("private"));
    let repeat = protocol::add_capture(&f.0, &f.round(), 2, input).unwrap();
    assert_eq!(repeat.revision, 2);
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), original);
}

#[test]
fn missing_can_be_recaptured_and_original_attempts_are_preserved() {
    let f = Fixture::new();
    let mut input = f.input(1, true);
    let round = protocol::add_capture(&f.0, &f.round(), 1, input.clone()).unwrap();
    assert_eq!(round.revision, 2);
    input.attempt = 2;
    let filename = round.matrix.screenshot_name(&input.cell, 2).unwrap();
    let path = f.round().parent().unwrap().join(filename);
    let bytes = png();
    fs::write(f.0.join(&path), &bytes).unwrap();
    input.observation = Observation::Captured {
        image: FileEvidence {
            path: path.to_str().unwrap().into(),
            sha256: hash(&bytes),
        },
        origin: Origin::Browser,
        tool: "fixture".into(),
        address: "http://localhost/orders".into(),
        taken_at: "2026-10-10T04:00:00Z".into(),
        viewport: Box::new(round.matrix.cells[0].viewport.clone()),
        tested_version: Box::new(round.tested_version),
        interactions: vec![],
        human_check: HumanCheck {
            opened: true,
            current_product: true,
            current_version: true,
            notes: "Fixture only".into(),
        },
    };
    let round = protocol::add_capture(&f.0, &f.round(), 2, input).unwrap();
    assert_eq!(round.revision, 3);
    assert_eq!(round.captures.len(), 2);
    assert!(matches!(
        round.captures[0].input.observation,
        Observation::Missing { .. }
    ));
}

#[test]
fn capture_rejects_stale_revision_frozen_drift_and_projection_edits() {
    let f = Fixture::new();
    let input = f.input(1, false);
    let round = protocol::add_capture(&f.0, &f.round(), 1, input.clone()).unwrap();
    let before = fs::read(f.0.join(f.round())).unwrap();
    assert_eq!(
        protocol::add_capture(&f.0, &f.round(), 1, input.clone())
            .unwrap_err()
            .code,
        3
    );
    let frozen =
        f.0.join(&round.captures[0].frozen_image.as_ref().unwrap().path);
    fs::write(&frozen, b"drift").unwrap();
    assert_eq!(
        protocol::add_capture(&f.0, &f.round(), 2, input.clone())
            .unwrap_err()
            .code,
        3
    );
    fs::write(&frozen, png()).unwrap();
    let md = f.0.join(f.round().parent().unwrap().join("capture.md"));
    fs::write(&md, "manual generation edit").unwrap();
    assert_eq!(
        protocol::add_capture(&f.0, &f.round(), 2, input)
            .unwrap_err()
            .code,
        3
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), before);
    assert_eq!(fs::read_to_string(md).unwrap(), "manual generation edit");
}

#[test]
fn capture_step_edits_keep_the_fingerprint_but_expectation_changes_conflict() {
    let f = Fixture::new();
    let input = f.input(1, false);
    fs::write(
        f.0.join(TARGET),
        TEXT.replace("Open the real app.", "Wait for stable fonts."),
    )
    .unwrap();
    let round = protocol::add_capture(&f.0, &f.round(), 1, input.clone()).unwrap();
    assert_eq!(round.revision, 2);
    fs::write(f.0.join("DESIGN.md"), "Changed design").unwrap();
    assert_eq!(
        protocol::add_capture(&f.0, &f.round(), 2, input)
            .unwrap_err()
            .code,
        3
    );
}

#[test]
fn later_manual_report_cannot_be_silently_reset_without_an_index() {
    let f = Fixture::new();
    let report = f.0.join(format!("{PREFIX}.ui-review.r2.md"));
    fs::write(&report, "Existing manual round two").unwrap();
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap_err().code, 3);
    assert_eq!(
        fs::read_to_string(report).unwrap(),
        "Existing manual round two"
    );
    assert!(!f.0.join(f.round()).exists());
}

#[test]
fn changing_a_frozen_copy_and_its_recorded_hash_cannot_change_the_target() {
    for attachment in [false, true] {
        let f = Fixture::new();
        let input = f.input(1, false);
        let state = f.0.join(f.round());
        let mut round: protocol::Round =
            serde_json::from_slice(&fs::read(&state).unwrap()).unwrap();
        let file = if attachment {
            round.target_snapshot.attachments[0]
                .frozen
                .as_mut()
                .unwrap()
        } else {
            &mut round.target_snapshot.expectations
        };
        fs::write(f.0.join(&file.path), "Different expectations").unwrap();
        file.sha256 = hash(b"Different expectations");
        let changed = serde_json::to_vec_pretty(&round).unwrap();
        fs::write(&state, &changed).unwrap();
        assert_eq!(
            protocol::add_capture(&f.0, &f.round(), 1, input)
                .unwrap_err()
                .code,
            4
        );
        assert_eq!(fs::read(&state).unwrap(), changed);
        assert_eq!(
            fs::read_to_string(f.0.join("DESIGN.md")).unwrap(),
            "Accepted design"
        );
    }
}

#[test]
fn capture_snapshot_must_use_the_exact_preallocated_attempt_name() {
    let f = Fixture::new();
    let input = f.input(1, false);
    let mut round = protocol::add_capture(&f.0, &f.round(), 1, input.clone()).unwrap();
    let frozen = round.captures[0].frozen_image.as_mut().unwrap();
    let alternate = f
        .round()
        .parent()
        .unwrap()
        .join("evidence/another-attempt.png");
    fs::copy(f.0.join(&frozen.path), f.0.join(&alternate)).unwrap();
    frozen.path = alternate.to_string_lossy().into_owned();
    let changed = serde_json::to_vec_pretty(&round).unwrap();
    fs::write(f.0.join(f.round()), &changed).unwrap();
    assert_eq!(
        protocol::add_capture(&f.0, &f.round(), 2, input)
            .unwrap_err()
            .code,
        4
    );
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), changed);
}

#[test]
fn a_latest_missing_attempt_is_current_and_the_older_image_is_history() {
    let f = Fixture::new();
    let mut input = f.input(1, false);
    protocol::add_capture(&f.0, &f.round(), 1, input.clone()).unwrap();
    input.attempt = 2;
    input.observation = Observation::Missing {
        reason: "Latest state could not be captured".into(),
    };
    let round = protocol::add_capture(&f.0, &f.round(), 2, input).unwrap();
    assert_eq!(round.captures.len(), 2);
    let md = fs::read_to_string(f.0.join(f.round().parent().unwrap().join("capture.md"))).unwrap();
    assert!(md.contains("| 1 | 历史 |"));
    assert!(md.contains("| 2 | 当前 | missing | Latest state could not be captured"));
}

#[test]
fn unknown_finalization_cannot_authorize_skipping_a_draft() {
    let f = Fixture::new();
    let mut round = protocol::begin(&f.0, f.options()).unwrap();
    round.finalization = Some(protocol::Finalization {
        status: "future-status".into(),
        result: FileEvidence {
            path: "docs/future.json".into(),
            sha256: hash(b"unknown"),
        },
    });
    let changed = serde_json::to_vec_pretty(&round).unwrap();
    fs::write(f.0.join(f.round()), &changed).unwrap();
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap_err().code, 4);
    assert_eq!(fs::read(f.0.join(f.round())).unwrap(), changed);
    assert!(!f.0.join(protocol::round_path(PREFIX, 2)).exists());
}

#[cfg(unix)]
#[test]
fn begin_cannot_follow_a_target_symlink_or_create_state_outside_the_project() {
    let f = Fixture::new();
    let outside = Fixture::new();
    fs::remove_file(f.0.join(TARGET)).unwrap();
    std::os::unix::fs::symlink(outside.0.join(TARGET), f.0.join(TARGET)).unwrap();
    assert_eq!(protocol::begin(&f.0, f.options()).unwrap_err().code, 2);
    assert!(!f.0.join(f.round()).exists());
    assert!(!outside.0.join(f.round()).exists());
}
