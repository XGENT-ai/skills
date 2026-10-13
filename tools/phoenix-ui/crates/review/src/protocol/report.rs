//! Markdown projections of round facts. The round report and the merged judge
//! list follow the review-ui report contract; the JSON facts stay authoritative
//! and these texts never add a judgement of their own.
use super::*;
use crate::{
    capture::CellKey,
    capture_evidence::WorkingTree,
    policy::{Disposition, GapStatus, Outcome, Scope, Severity, Verdict, WarningState},
};
use finalize::{
    Adjudication, CheckKind, CheckStatus, DecisionKind, Gap, GapOrigin, ResultRecord, RuntimeCheck,
    UnitConclusion,
};
use std::collections::BTreeSet;

const DIMENSIONS: [&str; 4] = ["布局", "视觉", "交互与状态", "细节"];

pub(super) fn round_label(round_id: &str) -> String {
    round_id.to_uppercase()
}

fn short(text: &str, length: usize) -> &str {
    text.char_indices()
        .nth(length)
        .map_or(text, |(at, _)| &text[..at])
}

/// HEAD short SHA (with the dirty fingerprint) and build ID, never raw JSON.
pub(super) fn version_text(version: &TestedVersion) -> String {
    let mut parts = Vec::new();
    match (&version.head, &version.working_tree) {
        (Some(head), WorkingTree::Clean) => parts.push(format!("`{}`", short(head, 7))),
        (Some(head), WorkingTree::Dirty { sha256 }) => parts.push(format!(
            "dirty@`{}` · 未提交改动指纹 {}",
            short(head, 7),
            short(sha256, 12)
        )),
        (Some(head), WorkingTree::Unknown) => {
            parts.push(format!("`{}`（工作树状态未知）", short(head, 7)))
        }
        (None, WorkingTree::Dirty { sha256 }) => {
            parts.push(format!("HEAD 未知 · 未提交改动指纹 {}", short(sha256, 12)))
        }
        (None, _) => parts.push("HEAD 未知".into()),
    }
    if let Some(id) = &version.build_id {
        parts.push(format!("构建 {}", table_text(id)));
    }
    parts.join("；")
}

pub(super) fn status_text(status: GapStatus) -> &'static str {
    match status {
        GapStatus::Resolved => "已解决",
        GapStatus::Partial => "部分解决",
        GapStatus::Present => "仍存在",
        GapStatus::Regressed => "回归",
        GapStatus::Unverified => "待核实",
        GapStatus::NotApplicable => "不再适用",
        GapStatus::NotReviewed => "本轮未复核",
    }
}

pub(super) fn disposition_text(disposition: Disposition) -> &'static str {
    match disposition {
        Disposition::Modified => "已修改待评审",
        Disposition::Declined => "不采纳",
        Disposition::AwaitingDecision => "待决定",
        Disposition::Blocked => "阻塞",
        Disposition::Untreated => "未处理",
    }
}

fn severity_text(severity: Severity) -> &'static str {
    match severity {
        Severity::P0 => "P0",
        Severity::P1 => "P1",
        Severity::P2 => "P2",
        Severity::P3 => "P3",
    }
}

fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::P0 => 0,
        Severity::P1 => 1,
        Severity::P2 => 2,
        Severity::P3 => 3,
    }
}

fn gating(gap: &Gap) -> bool {
    matches!(gap.severity, Severity::P0 | Severity::P1)
}

/// The verdict with its stall and pending-decision markers, as the report's first line.
pub(super) fn verdict_text(outcome: &Outcome) -> String {
    let mut text = match &outcome.verdict {
        Some(Verdict::NeedEvidence) => "需补证",
        Some(Verdict::Passed) => "通过验收",
        Some(Verdict::NeedDecision) => "需用户裁定",
        Some(Verdict::NeedModification) => "需要修改",
        None => "待确认评审",
    }
    .to_owned();
    for warning in &outcome.warnings {
        let state = match warning.state {
            WarningState::Warning => "停滞预警",
            WarningState::Stalled => "停滞",
        };
        let scope = match &warning.scope {
            Scope::Global => "全局".to_owned(),
            Scope::Gap { gap_id, unit_id } => format!("{gap_id} 所在 {unit_id}"),
        };
        text.push_str(&format!(" · {state}（{scope}）"));
    }
    if !outcome.pending_decisions.is_empty() {
        text.push_str(&format!(
            " · 待裁定（{}）",
            outcome.pending_decisions.join("、")
        ));
    }
    text
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn directory_link(label: &str, directory: &str, from: &str) -> Result<String, StoreError> {
    let link = applied::link(label, directory, from)?;
    Ok(format!("{}/)", &link[..link.len() - 1]))
}

fn cell_text(cell: &CellKey) -> String {
    format!(
        "{} / {} / {} / {} / {}",
        cell.unit_id, cell.state_id, cell.viewport_id, cell.theme_id, cell.step_id
    )
}

fn dpr(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

pub(super) fn viewport_text(viewport: &crate::matrix::Viewport) -> String {
    format!(
        "{} {}×{} @{}x",
        viewport.id,
        viewport.width,
        viewport.height,
        dpr(viewport.dpr)
    )
}

/// A link to the round's frozen copy of the current capture of `cell`.
fn image_link(round: &Round, cell: &CellKey, from: &str) -> Result<Option<String>, StoreError> {
    finalize::current_capture(round, cell)
        .and_then(|capture| capture.frozen_image.as_ref())
        .map(|file| applied::link(file_name(&file.path), &file.path, from))
        .transpose()
}

/// Reviewers cite the packet's derived `expectations.txt` and copied target images;
/// host findings cite the frozen expectations. Both resolve to the project's target.
fn basis_text(
    cycle: &Cycle,
    round: &Round,
    basis: &FileEvidence,
    location: &str,
    from: &str,
) -> Result<String, StoreError> {
    let source = if basis.path == "expectations.txt"
        || basis.sha256 == round.target_snapshot.expectations.sha256
    {
        applied::link("目标包", &cycle.target, from)?
    } else if let Some((attachment, frozen)) =
        round
            .target_snapshot
            .attachments
            .iter()
            .find_map(|attachment| {
                attachment
                    .frozen
                    .as_ref()
                    .filter(|frozen| frozen.sha256 == basis.sha256)
                    .map(|frozen| (attachment, frozen))
            })
    {
        applied::link(&attachment.logical_path, &frozen.path, from)?
    } else {
        table_text(&basis.path)
    };
    Ok(if location.trim().is_empty() {
        source
    } else {
        format!("{source} · {}", table_text(location))
    })
}

fn result_gaps(result: &ResultRecord) -> impl Iterator<Item = &Gap> {
    result
        .units
        .iter()
        .flat_map(|unit| &unit.gaps)
        .chain(&result.carried)
}

/// The latest description of a gap: this round's own, else the most recent earlier
/// round that described it (with that round's ID).
fn description<'a>(gap: &'a Gap, prior: &'a [ResultRecord]) -> Option<(&'a Gap, Option<&'a str>)> {
    let described = |gap: &Gap| !gap.details.is_empty() || !gap.runtime_findings.is_empty();
    if described(gap) {
        return Some((gap, None));
    }
    prior.iter().rev().find_map(|result| {
        result_gaps(result)
            .find(|old| old.id == gap.id && described(old))
            .map(|old| (old, Some(result.round_id.as_str())))
    })
}

fn clip(text: &str) -> String {
    let first = text
        .split(['。', '；', ';', '\n'])
        .map(str::trim)
        .find(|part| !part.is_empty())
        .unwrap_or("");
    let mut clipped = short(first, 40).to_owned();
    if clipped.len() < first.len() {
        clipped.push('…');
    }
    table_text(&clipped)
}

fn summary(gap: &Gap, prior: &[ResultRecord]) -> String {
    let explicit = std::iter::once(gap)
        .chain(
            prior
                .iter()
                .rev()
                .flat_map(|result| result_gaps(result).filter(|old| old.id == gap.id)),
        )
        .flat_map(|gap| &gap.details)
        .find_map(|detail| detail.gap.summary.as_deref());
    if let Some(summary) = explicit {
        return table_text(summary.trim());
    }
    match description(gap, prior) {
        Some((gap, _)) => gap
            .details
            .first()
            .map(|detail| clip(&detail.gap.current))
            .or_else(|| {
                gap.runtime_findings
                    .first()
                    .map(|(_, finding)| clip(&finding.description))
            })
            .unwrap_or_default(),
        None => "人工历史导入的差距，描述见旧报告".into(),
    }
}

fn recoverable(gap: &Gap, prior: &[ResultRecord]) -> Option<Decimal> {
    description(gap, prior).and_then(|(gap, _)| {
        gap.details
            .iter()
            .map(|detail| detail.gap.recoverable)
            .max()
    })
}

fn title(gap: &Gap, prior: &[ResultRecord], pending: bool) -> String {
    let mut parts = vec![gap.unit_id.clone()];
    if let Some((described, _)) = description(gap, prior) {
        let mut dimensions: Vec<&str> = Vec::new();
        for detail in &described.details {
            if !dimensions.contains(&detail.gap.dimension.as_str()) {
                dimensions.push(&detail.gap.dimension);
            }
        }
        if !dimensions.is_empty() {
            parts.push(dimensions.join("、"));
        }
    }
    if let Some(recoverable) = recoverable(gap, prior) {
        parts.push(format!("可挽回 {recoverable}"));
    }
    format!(
        "{} · [{}] {}（{}）{}",
        gap.id,
        severity_text(gap.severity),
        summary(gap, prior),
        parts.join(" · "),
        if pending { " · 待裁定" } else { "" }
    )
}

fn sorted<'a>(gaps: impl Iterator<Item = &'a Gap>, prior: &[ResultRecord]) -> Vec<&'a Gap> {
    let mut gaps: Vec<_> = gaps.filter(|gap| !finalize::closed(gap.status)).collect();
    gaps.sort_by(|a, b| {
        rank(a.severity)
            .cmp(&rank(b.severity))
            .then_with(|| recoverable(b, prior).cmp(&recoverable(a, prior)))
            .then_with(|| a.id.cmp(&b.id))
    });
    gaps
}

fn first_round<'a>(cycle: &'a Cycle, id: &str) -> Option<&'a str> {
    cycle
        .issues
        .iter()
        .find(|issue| issue.id == id)
        .map(|issue| issue.first_round.as_str())
}

/// Current judges as (file, record, still bound to the current evidence).
pub(super) type Judges = [(FileEvidence, judgement::JudgeRecord, bool)];

fn reassessments<'a>(
    judges: &'a Judges,
    id: &'a str,
) -> impl Iterator<Item = (&'a str, &'a judgement::OldAssessment)> {
    judges
        .iter()
        .filter(|(_, _, current)| *current)
        .flat_map(move |(_, record, _)| {
            record
                .input
                .previous
                .iter()
                .filter(move |old| old.id == id)
                .map(move |old| (record.input.reviewer_id.as_str(), old))
        })
}

fn reassessment_text(
    round: &Round,
    reviewer: &str,
    old: &judgement::OldAssessment,
    from: &str,
) -> Result<String, StoreError> {
    Ok(match &old.basis {
        judgement::AssessmentBasis::Current(evidence) => {
            let image = image_link(round, &evidence.cell, from)?
                .map(|link| format!("（{link}）"))
                .unwrap_or_default();
            format!("评审 {reviewer}：{}{image}", table_text(&evidence.notes))
        }
        judgement::AssessmentBasis::LegacyText(text) => {
            format!("评审 {reviewer}：{}", table_text(text))
        }
    })
}

struct GapContext<'a> {
    cycle: &'a Cycle,
    round: &'a Round,
    prior: &'a [ResultRecord],
    judges: &'a Judges,
    machine: &'a [FileEvidence],
    from: &'a str,
}

/// The bullets of one gap: this round's description, or the earlier description
/// with this round's reassessment. `host` adds host-measured page findings.
fn gap_lines(context: &GapContext<'_>, gap: &Gap, host: bool) -> Result<String, StoreError> {
    let GapContext {
        cycle,
        round,
        prior,
        judges,
        machine,
        from,
    } = *context;
    let mut text = String::new();
    let details: Vec<_> = gap.details.iter().collect();
    if let Some((first, others)) = details.split_first() {
        let image = image_link(round, &first.gap.cell, from)?.unwrap_or_default();
        text.push_str(&format!(
            "- 位置：{image} {}\n- 现状 → 期望：{} → {}（依据：{}）\n- 修法：{}\n",
            table_text(&first.gap.location),
            table_text(&first.gap.current),
            table_text(&first.gap.expected),
            basis_text(
                cycle,
                round,
                &first.gap.basis,
                &first.gap.basis_location,
                from
            )?,
            table_text(&first.gap.fix)
        ));
        for other in others {
            text.push_str(&format!(
                "- 另一评审 {}：{}：{} → {}；修法：{}\n",
                other.reviewer_id,
                table_text(&other.gap.location),
                table_text(&other.gap.current),
                table_text(&other.gap.expected),
                table_text(&other.gap.fix)
            ));
        }
    } else if let Some((described, Some(round_id))) = description(gap, prior) {
        for detail in &described.details {
            text.push_str(&format!(
                "- 描述（沿用 {} 评审 {}）：{}：{} → {}；修法：{}\n",
                round_label(round_id),
                detail.reviewer_id,
                table_text(&detail.gap.location),
                table_text(&detail.gap.current),
                table_text(&detail.gap.expected),
                table_text(&detail.gap.fix)
            ));
        }
        if host {
            for (_, finding) in &described.runtime_findings {
                text.push_str(&format!(
                    "- 描述（沿用 {} 页面实测）：{} → {}；修法：{}\n",
                    round_label(round_id),
                    table_text(&finding.description),
                    table_text(&finding.expected),
                    table_text(&finding.fix)
                ));
            }
        }
    } else if description(gap, prior).is_none() {
        text.push_str("- 描述：人工历史导入的差距，见旧报告\n");
    }
    for (reviewer, old) in reassessments(judges, &gap.id) {
        text.push_str(&format!(
            "- 本轮复核：{}\n",
            reassessment_text(round, reviewer, old, from)?
        ));
    }
    let mut sources = Vec::new();
    let reviewers: BTreeSet<&str> = gap
        .details
        .iter()
        .map(|detail| detail.reviewer_id.as_str())
        .collect();
    if !reviewers.is_empty() {
        sources.push(format!(
            "评审 {}（{}）",
            round_label(&round.round_id),
            reviewers.into_iter().collect::<Vec<_>>().join("、")
        ));
    }
    if host {
        for (file, finding) in &gap.runtime_findings {
            let record = applied::link("记录", &file.path, from)?;
            text.push_str(&format!(
                "- 页面实测：{} → {}（依据：{}）；修法：{}（{record}）\n",
                table_text(&finding.description),
                table_text(&finding.expected),
                basis_text(cycle, round, &finding.basis, "", from)?,
                table_text(&finding.fix)
            ));
            let label = if machine.contains(file) {
                "机器检查"
            } else {
                "主 agent 核实"
            };
            let source = format!("{label} {}", round_label(&round.round_id));
            if !sources.contains(&source) {
                sources.push(source);
            }
        }
    }
    if sources.is_empty() {
        if let Some((described, round_id)) = description(gap, prior) {
            let reviewers: BTreeSet<&str> = described
                .details
                .iter()
                .map(|detail| detail.reviewer_id.as_str())
                .collect();
            let round_id = round_label(round_id.unwrap_or(&round.round_id));
            sources.push(if reviewers.is_empty() {
                format!("页面实测 {round_id}")
            } else {
                format!(
                    "评审 {round_id}（{}）",
                    reviewers.into_iter().collect::<Vec<_>>().join("、")
                )
            });
        } else {
            sources.push("人工历史导入".into());
        }
    }
    text.push_str(&format!("- 来源：{}\n", sources.join("；")));
    Ok(text)
}

fn conclusion_text(cycle: &Cycle, round: &Round, gap: &Gap) -> String {
    match first_round(cycle, &gap.id) {
        Some(first) if first == round.round_id => "本轮新增".into(),
        Some(first) => format!(
            "{}（沿用 {} 编号）",
            status_text(gap.status),
            round_label(first)
        ),
        None => status_text(gap.status).into(),
    }
}

fn minimum(units: &[UnitConclusion]) -> Option<(&str, Decimal)> {
    units
        .iter()
        .filter_map(|unit| unit.score.map(|score| (unit.unit_id.as_str(), score)))
        .min_by_key(|(_, score)| *score)
}

fn scoped_round(full: bool, units: &[UnitConclusion]) -> String {
    if full {
        String::new()
    } else {
        format!(
            "限定范围（{}）",
            units
                .iter()
                .map(|unit| unit.unit_id.as_str())
                .collect::<Vec<_>>()
                .join("、")
        )
    }
}

pub(super) struct ReportInput<'a> {
    pub cycle: &'a Cycle,
    pub round: &'a Round,
    pub units: &'a [UnitConclusion],
    pub carried: &'a [Gap],
    pub outcome: &'a Outcome,
    pub adjudication: &'a Adjudication,
    pub prior: &'a [ResultRecord],
    pub records: &'a [applied::Record],
    pub legacy: Option<&'a legacy::LegacyMap>,
    pub judges: &'a Judges,
    pub checks: &'a [(FileEvidence, RuntimeCheck)],
    /// Open gaps of the most recent rounds, as reassessed now.
    pub previous_open: &'a [Gap],
    /// What is missing when the evidence is incomplete.
    pub evidence_gaps: &'a [String],
}

impl ReportInput<'_> {
    fn previous_round(&self) -> Option<&str> {
        self.prior.last().map(|result| result.round_id.as_str())
    }

    fn rebuilt(&self) -> bool {
        (self.previous_round().is_some() || self.legacy.is_some())
            && self
                .round
                .baseline
                .starts_with(&format!("{}:", self.round.round_id))
    }

    fn addresses(&self) -> Vec<String> {
        let mut addresses = Vec::new();
        for cell in &self.round.matrix.cells {
            if let Some(Observation::Captured { address, .. }) =
                finalize::current_capture(self.round, &cell.key).map(|c| &c.input.observation)
            {
                if !addresses.contains(address) {
                    addresses.push(address.clone());
                }
            }
        }
        addresses
    }

    fn current_gap(&self, id: &str) -> Option<&Gap> {
        self.units
            .iter()
            .flat_map(|unit| &unit.gaps)
            .chain(self.carried)
            .find(|gap| gap.id == id)
    }

    fn gap_context<'b>(&'b self, from: &'b str) -> GapContext<'b> {
        GapContext {
            cycle: self.cycle,
            round: self.round,
            prior: self.prior,
            judges: self.judges,
            machine: &self.adjudication.machine_checks,
            from,
        }
    }
}

fn subject(input: &ReportInput<'_>, from: &str) -> Result<String, StoreError> {
    let round = input.round;
    let mut label = round_label(&round.round_id);
    if input.rebuilt() {
        label.push_str("；重建基线");
    }
    if !round.full {
        label.push_str(&format!("；{}", scoped_round(false, input.units)));
    }
    let plan = match &input.cycle.plan {
        Some(plan) => applied::link(file_name(plan), plan, from)?,
        None => "无".into(),
    };
    let target = format!(
        "{} · {}",
        applied::link(file_name(&input.cycle.target), &input.cycle.target, from)?,
        short(&round.target_fingerprint, 12)
    );
    let mut version = vec![version_text(&round.tested_version)];
    version.extend(input.addresses().iter().map(|address| table_text(address)));
    let directory = round_path(&round.prefix, ordinal(&round.round_id)?);
    let shots = directory_link(
        &round.round_id,
        &directory.parent().unwrap().to_string_lossy(),
        from,
    )?;
    let previous = match (input.previous_round(), input.legacy) {
        (Some(id), _) => {
            let path = report_path(&round.prefix, id);
            applied::link(file_name(&path), &path, from)?
        }
        (None, Some(map)) => {
            let path = &map.last().report.path;
            applied::link(file_name(path), path, from)?
        }
        (None, None) => "无".into(),
    };
    Ok(format!(
        "## 评审对象\n\n| 轮次 | 开发计划 | 目标包 | 被测版本 | 截图 | 上轮报告 |\n| --- | --- | --- | --- | --- | --- |\n| {label} | {plan} | {target} | {} | {shots} | {previous} |\n\n",
        version.join("；")
    ))
}

fn reasons(input: &ReportInput<'_>) -> String {
    let outcome = input.outcome;
    let threshold = input.round.threshold;
    let open = sorted(
        input
            .units
            .iter()
            .flat_map(|unit| &unit.gaps)
            .chain(input.carried),
        input.prior,
    );
    let ids = |gaps: &[&Gap]| {
        gaps.iter()
            .map(|gap| gap.id.as_str())
            .collect::<Vec<_>>()
            .join("、")
    };
    let gating: Vec<&Gap> = open.iter().copied().filter(|gap| gating(gap)).collect();
    match &outcome.verdict {
        Some(Verdict::NeedEvidence) if input.evidence_gaps.is_empty() => {
            "限定范围内各单元已达标，需要一轮全量评审才能判通过。".into()
        }
        Some(Verdict::NeedEvidence) => format!(
            "缺少证据：{}。补齐后重新采集与评审；已评单元的分数和差距照常列出，可以先修。",
            input.evidence_gaps.join("；")
        ),
        Some(Verdict::Passed) => {
            let mut text = match minimum(input.units) {
                Some((unit, score)) => format!(
                    "全量轮各单元均达标：最低 {unit} {score} ≥ 门槛 {threshold}，没有未关闭的 P0/P1。"
                ),
                None => "全量轮各单元均达标，没有未关闭的 P0/P1。".into(),
            };
            if !open.is_empty() {
                text.push_str(&format!("剩余 {} 条 P2/P3 差距为可选打磨项。", open.len()));
            }
            text
        }
        Some(Verdict::NeedDecision) => {
            if outcome.warnings.iter().any(|warning| {
                warning.scope == Scope::Global && warning.state == WarningState::Stalled
            }) {
                "全局停滞：已做过全局重构轮，本轮仍低于门槛，且历史最佳提高不到 1.0。".into()
            } else if open
                .iter()
                .all(|gap| outcome.pending_decisions.contains(&gap.id))
            {
                format!(
                    "尚未通过，且未关闭的差距都是待裁定事项（{}），继续修改无法推进。",
                    ids(&open)
                )
            } else {
                format!(
                    "分数已达门槛，剩余门槛项都是待裁定事项（{}）；继续修 P2/P3 也无法通过。",
                    ids(&gating)
                )
            }
        }
        Some(Verdict::NeedModification) | None => {
            let mut parts = Vec::new();
            if !input.round.full {
                parts.push("限定范围轮没有本轮分数".to_owned());
            }
            if let Some((unit, score)) =
                minimum(input.units).filter(|(_, score)| *score < threshold)
            {
                parts.push(format!("最低单元 {unit} {score} 低于门槛 {threshold}"));
            }
            if !gating.is_empty() {
                parts.push(format!(
                    "未关闭门槛项 {} 条（{}）",
                    gating.len(),
                    ids(&gating)
                ));
            }
            if parts.is_empty() {
                "仍有可修复的差距。".into()
            } else {
                format!("{}。", parts.join("；"))
            }
        }
    }
}

fn verdict_section(input: &ReportInput<'_>) -> Result<String, StoreError> {
    let scoring = &input.outcome.scoring;
    let score = match (scoring.score, minimum(input.units)) {
        (Some(score), Some((unit, _))) => format!("{score}（{unit}）"),
        (Some(score), None) => score.to_string(),
        (None, _) if !input.round.full => "无（限定范围）".into(),
        (None, _) => "无（取证或评分不完整）".into(),
    };
    let best = scoring
        .historical_best
        .map_or_else(|| "无".into(), |best| best.to_string());
    let mut text = format!(
        "## 验收判定\n\n判定：{}\n\n本轮分数 {score}；门槛 {}；历史最佳 {best}。{}\n\n",
        verdict_text(input.outcome),
        input.round.threshold,
        reasons(input)
    );
    let waivers: Vec<_> = input
        .adjudication
        .decisions
        .iter()
        .filter(|decision| decision.kind == DecisionKind::WholeUiWaiver)
        .map(|decision| finalize::Waiver {
            round_id: input.round.round_id.clone(),
            decision: decision.clone(),
        })
        .collect();
    if !waivers.is_empty() {
        text.push_str(&String::from_utf8(finalize::waiver_body(&waivers)?).unwrap());
        text.push('\n');
    }
    Ok(text)
}

fn scoring_section(input: &ReportInput<'_>) -> String {
    let threshold = input.round.threshold;
    let mut text = "## 评分\n\n| 单元 | 布局 | 视觉 | 交互与状态 | 细节 | 单元分 | 达标 | 采用评审 |\n| --- | --- | --- | --- | --- | --- | --- | --- |\n".to_owned();
    for unit in input.units {
        match (&unit.selected, unit.score) {
            (Some(selected), Some(score)) => {
                let passed = score >= threshold
                    && !unit
                        .gaps
                        .iter()
                        .any(|gap| gating(gap) && !finalize::closed(gap.status));
                text.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {score} | {} | {} |\n",
                    unit.unit_id,
                    selected.scores[0],
                    selected.scores[1],
                    selected.scores[2],
                    selected.scores[3],
                    if passed { "是" } else { "否" },
                    unit.selected_reviewer.as_deref().unwrap_or_default()
                ));
            }
            _ => text.push_str(&format!(
                "| {} | — | — | — | — | — | — | 未评：{} |\n",
                unit.unit_id,
                table_text(unit.unreviewed_reason.as_deref().unwrap_or_default())
            )),
        }
    }
    let scoring = &input.outcome.scoring;
    text.push_str(&format!(
        "\n- 本轮分数：{}；历史最佳：{}；门槛：{threshold}\n",
        match (scoring.score, minimum(input.units)) {
            (Some(score), Some((unit, _))) => format!("{score}（{unit}）"),
            (Some(score), None) => score.to_string(),
            (None, _) => "无".into(),
        },
        scoring
            .historical_best
            .map_or_else(|| "无".into(), |best| best.to_string())
    ));
    for unit in input.units {
        let scores: Vec<String> = input
            .judges
            .iter()
            .filter(|(_, _, current)| *current)
            .filter_map(|(_, record, _)| {
                record
                    .input
                    .units
                    .iter()
                    .find(|scored| scored.unit_id == unit.unit_id)
                    .and_then(|scored| crate::sum_dimensions(scored.scores).ok())
                    .map(|score| format!("{} {score}", record.input.reviewer_id))
            })
            .collect();
        if scores.len() > 1 {
            text.push_str(&format!(
                "- {} 各评审单元分：{}；取较低的一整组 {}（{}）\n",
                unit.unit_id,
                scores.join("、"),
                unit.score.map(|s| s.to_string()).unwrap_or_default(),
                unit.selected_reviewer.as_deref().unwrap_or_default()
            ));
        }
    }
    for unit in input.units {
        if let Some(selected) = &unit.selected {
            text.push_str(&format!(
                "\n### {} 维度理由（{}）\n\n",
                unit.unit_id,
                unit.selected_reviewer.as_deref().unwrap_or_default()
            ));
            for (dimension, reason) in DIMENSIONS.iter().zip(&selected.reasons) {
                text.push_str(&format!("- {dimension}：{}\n", table_text(reason)));
            }
        }
    }
    text.push('\n');
    text
}

#[allow(clippy::too_many_arguments)]
fn trajectory_row(
    round_id: &str,
    first: bool,
    baseline: &str,
    full: bool,
    units: &[UnitConclusion],
    carried: &[Gap],
    outcome: &Outcome,
    refactors: &[&applied::Refactor],
) -> String {
    let score = match (outcome.scoring.score, minimum(units)) {
        (Some(score), Some((unit, _))) => format!("{score}（{unit}）"),
        _ => "无".into(),
    };
    let best = outcome
        .scoring
        .historical_best
        .map_or_else(|| "—".into(), |best| best.to_string());
    let open = units
        .iter()
        .flat_map(|unit| &unit.gaps)
        .chain(carried)
        .filter(|gap| gating(gap) && !finalize::closed(gap.status))
        .count();
    let refactor = if refactors.is_empty() {
        "—".into()
    } else {
        refactors
            .iter()
            .map(|refactor| refactor_scope(&refactor.scope))
            .collect::<Vec<_>>()
            .join("、")
    };
    let mut remarks = Vec::new();
    if first {
        remarks.push("首轮".to_owned());
    } else if baseline.starts_with(&format!("{round_id}:")) {
        remarks.push("重建基线".to_owned());
    }
    if !full {
        remarks.push(scoped_round(full, units));
    }
    remarks.push(verdict_text(outcome));
    format!(
        "| {} | {score} | {best} | {open} | {refactor} | {} |\n",
        round_label(round_id),
        remarks.join(" · ")
    )
}

pub(super) fn refactor_scope(scope: &applied::RefactorScope) -> String {
    match scope {
        applied::RefactorScope::Global => "全局".into(),
        applied::RefactorScope::Unit { unit_id } => unit_id.clone(),
    }
}

fn trajectory_section(input: &ReportInput<'_>) -> String {
    let mut text = "## 轮次轨迹\n\n| 轮次 | 本轮分数（最低单元） | 历史最佳 | 未关闭 P0/P1 | 重构轮 | 备注 |\n| --- | --- | --- | --- | --- | --- |\n".to_owned();
    if let Some(map) = input.legacy {
        for round in &map.rounds {
            let score = match (&round.full_score.state, round.full_score.value) {
                (legacy::Knowledge::Known, Some(score)) => score.to_string(),
                (legacy::Knowledge::Absent, _) => "无".into(),
                _ => "未知".into(),
            };
            text.push_str(&format!(
                "| {} | {score} | — | — | — | 人工历史导入 |\n",
                round_label(&round.round_id)
            ));
        }
    }
    let refactors = |previous: Option<&str>| -> Vec<&applied::Refactor> {
        input
            .records
            .iter()
            .filter(|record| Some(record.round_id.as_str()) == previous)
            .flat_map(|record| &record.input.restructure_scopes)
            .collect()
    };
    let mut previous: Option<&str> = None;
    for (index, result) in input.prior.iter().enumerate() {
        text.push_str(&trajectory_row(
            &result.round_id,
            index == 0 && input.legacy.is_none(),
            &result.baseline,
            result.full,
            &result.units,
            &result.carried,
            &result.outcome,
            &refactors(previous),
        ));
        previous = Some(&result.round_id);
    }
    text.push_str(&trajectory_row(
        &input.round.round_id,
        input.prior.is_empty() && input.legacy.is_none(),
        &input.round.baseline,
        input.round.full,
        input.units,
        input.carried,
        input.outcome,
        &refactors(previous),
    ));
    text.push('\n');
    text
}

fn previous_disposition(gap: &Gap) -> String {
    let Some(last) = gap.history.last() else {
        return "无记录".into();
    };
    if last.disposition_unknown {
        return "未知（导入历史）".into();
    }
    match (
        last.prior_disposition.attempted,
        last.prior_disposition.latest,
    ) {
        (true, Some(latest)) if latest != Disposition::Modified => {
            format!("已修改待评审；最近：{}", disposition_text(latest))
        }
        (_, Some(latest)) => disposition_text(latest).into(),
        (_, None) => "无记录".into(),
    }
}

fn review_section(input: &ReportInput<'_>, from: &str) -> Result<String, StoreError> {
    if input.prior.is_empty() && input.legacy.is_none() {
        return Ok(String::new());
    }
    let mut text = "## 上轮差距复核\n\n".to_owned();
    if input.previous_open.is_empty() {
        text.push_str("上一轮没有未关闭的差距。\n\n");
        return Ok(text);
    }
    text.push_str("| 编号 | 上轮严重度与摘要 | 上轮处置（取自修复记录） | 本轮结果 | 依据 |\n| --- | --- | --- | --- | --- |\n");
    let mut old: Vec<&Gap> = input.previous_open.iter().collect();
    old.sort_by(|a, b| a.id.cmp(&b.id));
    for previous in old {
        let Some(current) = input.current_gap(&previous.id) else {
            continue;
        };
        let mut basis = Vec::new();
        for (reviewer, assessment) in reassessments(input.judges, &previous.id) {
            basis.push(reassessment_text(input.round, reviewer, assessment, from)?);
        }
        for (file, check) in input.checks {
            if check
                .reassessed
                .iter()
                .any(|reassessed| reassessed.issue_id == previous.id)
            {
                basis.push(format!(
                    "主 agent 页面复核（{}）",
                    applied::link("记录", &file.path, from)?
                ));
            }
        }
        if basis.is_empty() {
            basis.push(if input.carried.iter().any(|gap| gap.id == previous.id) {
                "不在本轮范围".into()
            } else {
                "本轮没有有效复核".into()
            });
        }
        text.push_str(&format!(
            "| {} | {} · {} | {} | {} | {} |\n",
            previous.id,
            severity_text(previous.severity),
            summary(previous, input.prior),
            previous_disposition(current),
            status_text(current.status),
            basis.join("；")
        ));
    }
    text.push('\n');
    Ok(text)
}

fn gap_section(input: &ReportInput<'_>, from: &str) -> Result<String, StoreError> {
    let gaps = sorted(
        input
            .units
            .iter()
            .flat_map(|unit| &unit.gaps)
            .chain(input.carried),
        input.prior,
    );
    let mut text = "## 差距清单\n\n".to_owned();
    if gaps.is_empty() {
        text.push_str("没有未关闭的差距。\n\n");
        return Ok(text);
    }
    let context = input.gap_context(from);
    for gap in gaps {
        let pending = input.outcome.pending_decisions.contains(&gap.id);
        text.push_str(&format!(
            "### {}\n\n- 本轮结果：{}\n",
            title(gap, input.prior, pending),
            if input.carried.iter().any(|carried| carried.id == gap.id) {
                "本轮未复核（不在本轮范围）".into()
            } else {
                conclusion_text(input.cycle, input.round, gap)
            }
        ));
        text.push_str(&gap_lines(&context, gap, true)?);
        text.push('\n');
    }
    Ok(text)
}

fn attempts(input: &ReportInput<'_>, id: &str) -> Vec<String> {
    let mut rounds: Vec<(&str, &applied::Item)> = Vec::new();
    for record in input.records {
        for item in record.input.items.iter().filter(|item| item.issue_id == id) {
            rounds.retain(|(round, _)| *round != record.round_id);
            rounds.push((&record.round_id, item));
        }
    }
    rounds
        .into_iter()
        .map(|(round, item)| {
            format!(
                "{} → 修复：{}（核实：{}）",
                round_label(round),
                disposition_text(item.disposition),
                table_text(&item.verification)
            )
        })
        .collect()
}

fn pending_section(input: &ReportInput<'_>) -> String {
    let mut text = "## 待裁定事项\n\n".to_owned();
    let options = "- 可选项：给出修复方向后继续修改；接受当前偏差，写入目标包比较口径并在下一轮重建基线；或接受当前整体界面（用户豁免，需原话）\n\n";
    let global =
        input.outcome.warnings.iter().any(|warning| {
            warning.scope == Scope::Global && warning.state == WarningState::Stalled
        });
    if global {
        let refactors: Vec<_> = input
            .records
            .iter()
            .filter(|record| record.baseline == input.round.baseline)
            .flat_map(|record| &record.input.restructure_scopes)
            .filter(|refactor| refactor.scope == applied::RefactorScope::Global)
            .map(|refactor| table_text(&refactor.change))
            .collect();
        text.push_str(&format!(
            "### 全局停滞\n\n- 原因：本轮分数低于门槛，近三个有本轮分数的轮次历史最佳提高不到 1.0，且已做过全局重构轮\n- 已有的尝试：{}\n{options}",
            if refactors.is_empty() {
                "导入历史中的全局重构".to_owned()
            } else {
                format!("全局重构：{}", refactors.join("；"))
            }
        ));
    }
    if input.outcome.pending_decisions.is_empty() && !global {
        text.push_str("无。\n\n");
        return text;
    }
    for id in &input.outcome.pending_decisions {
        let Some(gap) = input.current_gap(id) else {
            continue;
        };
        let mut reasons = Vec::new();
        if input.outcome.warnings.iter().any(|warning| {
            warning.state == WarningState::Stalled
                && matches!(&warning.scope, Scope::Gap { gap_id, .. } if gap_id == id)
        }) {
            reasons.push(format!(
                "{} 所在 {} 已重构过，仍满足顽固差距条件（停滞）",
                id, gap.unit_id
            ));
        }
        let last = gap.history.last();
        match last.and_then(|last| last.prior_disposition.latest) {
            Some(Disposition::Declined)
                if matches!(
                    gap.status,
                    GapStatus::Present | GapStatus::Regressed | GapStatus::Partial
                ) =>
            {
                reasons.push(format!(
                    "修复记录最近处置为不采纳，本轮评审仍判{}",
                    status_text(gap.status)
                ));
            }
            Some(disposition @ (Disposition::AwaitingDecision | Disposition::Blocked)) => {
                reasons.push(format!(
                    "修复记录最近处置为{}，尚未解除",
                    disposition_text(disposition)
                ));
            }
            _ => (),
        }
        if gap.pending_decision {
            reasons.push("此前已列为待裁定事项，用户尚未给出方向".into());
        }
        if reasons.is_empty() {
            reasons.push("需要用户决定".into());
        }
        let tried = attempts(input, id);
        text.push_str(&format!(
            "### {} · [{}] {}\n\n- 原因：{}\n- 相关差距：{}\n- 已有的尝试：{}\n{options}",
            id,
            severity_text(gap.severity),
            summary(gap, input.prior),
            reasons.join("；"),
            id,
            if tried.is_empty() {
                "无".into()
            } else {
                tried.join("；")
            }
        ));
    }
    text
}

fn suggestion_section(input: &ReportInput<'_>) -> String {
    let mut text = "## 建议（不扣分）\n\n".to_owned();
    let mut any = false;
    for unit in input.units {
        let mut seen = BTreeSet::new();
        for (_, record, current) in input.judges {
            if !current {
                continue;
            }
            for scored in record
                .input
                .units
                .iter()
                .filter(|scored| scored.unit_id == unit.unit_id)
            {
                for suggestion in &scored.suggestions {
                    if seen.insert(suggestion.as_str()) {
                        any = true;
                        text.push_str(&format!(
                            "- {}（{}）：{}\n",
                            unit.unit_id,
                            record.input.reviewer_id,
                            table_text(suggestion)
                        ));
                    }
                }
            }
        }
    }
    if !any {
        text.push_str("无。\n");
    }
    text.push('\n');
    text
}

fn check_section(input: &ReportInput<'_>, from: &str) -> Result<String, StoreError> {
    let mut text = "## 机器检查\n\n".to_owned();
    if input.checks.is_empty() {
        text.push_str(
            "未检查：本轮没有登记运行页面上的机器检查或主 agent 实测；未检查不等于通过。\n\n",
        );
        return Ok(text);
    }
    text.push_str("| 检查项 | 工具与版本 | 结果 | 对应差距 |\n| --- | --- | --- | --- |\n");
    for (file, check) in input.checks {
        let kind = match (input.adjudication.machine_checks.contains(file), check.kind) {
            (_, CheckKind::Source) => "源码检查（仅作补测线索）",
            (true, CheckKind::Page) => "机器检查",
            (false, CheckKind::Page) => "主 agent 实测",
        };
        let status = match check.status {
            CheckStatus::Passed => "通过",
            CheckStatus::Failed => "失败",
            CheckStatus::Unavailable => "未检查（工具不可用）",
        };
        let mut gaps: Vec<String> = input
            .units
            .iter()
            .flat_map(|unit| &unit.gaps)
            .filter(|gap| gap.runtime_findings.iter().any(|(f, _)| f == file))
            .map(|gap| gap.id.clone())
            .collect();
        for reassessed in &check.reassessed {
            gaps.push(format!(
                "{}（复核：{}）",
                reassessed.issue_id,
                status_text(reassessed.status)
            ));
        }
        text.push_str(&format!(
            "| {kind}：{}（{}） | {} · {} | {status}：{} | {} |\n",
            cell_text(&check.cell),
            applied::link("记录", &file.path, from)?,
            table_text(&check.tool),
            table_text(&check.taken_at),
            table_text(&check.notes),
            if gaps.is_empty() {
                "—".into()
            } else {
                gaps.join("、")
            }
        ));
    }
    text.push('\n');
    Ok(text)
}

fn coverage_section(input: &ReportInput<'_>, from: &str) -> Result<String, StoreError> {
    let round = input.round;
    let mut tools = Vec::new();
    let mut captured = 0;
    // Missing cells of the covered units grouped by their recorded reason, in matrix order.
    let mut missing: Vec<(String, Vec<String>)> = Vec::new();
    for cell in round.matrix.selected_cells() {
        let reason = match finalize::current_capture(round, &cell.key).map(|c| &c.input.observation)
        {
            Some(Observation::Captured { tool, .. }) => {
                captured += 1;
                if !tools.contains(tool) {
                    tools.push(tool.clone());
                }
                continue;
            }
            Some(Observation::Missing { reason }) => table_text(reason),
            None => "未登记".to_owned(),
        };
        match missing.iter_mut().find(|(known, _)| *known == reason) {
            Some((_, cells)) => cells.push(cell_text(&cell.key)),
            None => missing.push((reason, vec![cell_text(&cell.key)])),
        }
    }
    let missing_count: usize = missing.iter().map(|(_, cells)| cells.len()).sum();
    let recaptured: Vec<String> = round
        .captures
        .iter()
        .filter(|capture| capture.input.attempt > 1)
        .map(|capture| {
            format!(
                "{} attempt {}",
                cell_text(&capture.input.cell),
                capture.input.attempt
            )
        })
        .collect();
    let mut viewports = Vec::new();
    let mut themes = Vec::new();
    for unit in &round.matrix.units {
        for viewport in &unit.viewports {
            let text = viewport_text(viewport);
            if !viewports.contains(&text) {
                viewports.push(text);
            }
        }
        for theme in &unit.themes {
            if !themes.contains(&theme.id) {
                themes.push(theme.id.clone());
            }
        }
    }
    let directory = round_path(&round.prefix, ordinal(&round.round_id)?);
    let capture_list = directory.parent().unwrap().join("capture.md");
    let mut text = format!(
        "## 采集与覆盖\n\n- 环境与被测版本：{}；访问地址 {}\n- 采集工具与条件：{}；视口 {}；主题 {}；逐图出处见 {}\n- 目标指纹：{} · {}；目标基线 {}\n- 目标文件：期望节 sha256 {}",
        version_text(&round.tested_version),
        {
            let addresses = input.addresses();
            if addresses.is_empty() {
                "无".to_owned()
            } else {
                addresses.iter().map(|a| table_text(a)).collect::<Vec<_>>().join("、")
            }
        },
        if tools.is_empty() {
            "无".to_owned()
        } else {
            tools.iter().map(|t| table_text(t)).collect::<Vec<_>>().join("、")
        },
        viewports.join("、"),
        themes.join("、"),
        applied::link("capture.md", &capture_list.to_string_lossy(), from)?,
        round.target_snapshot.algorithm,
        round.target_fingerprint,
        round.baseline,
        short(&round.target_snapshot.expectations.sha256, 12)
    );
    for attachment in &round.target_snapshot.attachments {
        text.push_str(&format!(
            "；{} {}",
            table_text(&attachment.logical_path),
            attachment.frozen.as_ref().map_or_else(
                || "缺失".to_owned(),
                |frozen| format!("sha256 {}", short(&frozen.sha256, 12))
            )
        ));
    }
    if !round.target_snapshot.missing.is_empty() {
        text.push_str(&format!(
            "；目标缺项 {}",
            round
                .target_snapshot
                .missing
                .iter()
                .map(|m| table_text(m))
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    text.push_str(&format!(
        "\n- 格子：已采 {captured} / {}；{}；补采：{}\n- 环境噪声：未单独登记，如有见核对说明\n",
        round.matrix.selected_cells().count(),
        if missing.is_empty() {
            "未采：无".to_owned()
        } else {
            format!(
                "未采 {missing_count} 格：{}",
                missing
                    .iter()
                    .map(|(reason, cells)| format!("{reason}（{}）", cells.join("、")))
                    .collect::<Vec<_>>()
                    .join("；")
            )
        },
        if recaptured.is_empty() {
            "无".to_owned()
        } else {
            recaptured.join("、")
        }
    ));
    let current: Vec<_> = input
        .judges
        .iter()
        .filter(|(_, _, current)| *current)
        .collect();
    let mut groups = Vec::new();
    for (_, record, _) in &current {
        let conversion = match record.input.conversion {
            judgement::Conversion::ReviewerStructured => "评审本人结构化输出",
            judgement::Conversion::HostVerified { .. } => "主 agent 依原文忠实转换",
        };
        groups.push(format!(
            "{}：{}（packet {}；{}{}）",
            record.input.reviewer_id,
            record
                .input
                .units
                .iter()
                .map(|unit| unit.unit_id.as_str())
                .collect::<Vec<_>>()
                .join("、"),
            short(&record.input.packet_hash, 12),
            conversion,
            if record.input.provenance.fresh_context {
                "；全新上下文"
            } else {
                "；非全新上下文"
            }
        ));
    }
    let confirmed: Vec<&str> = input
        .units
        .iter()
        .filter(|unit| {
            current
                .iter()
                .filter(|(_, record, _)| {
                    record
                        .input
                        .units
                        .iter()
                        .any(|scored| scored.unit_id == unit.unit_id)
                })
                .count()
                > 1
        })
        .map(|unit| unit.unit_id.as_str())
        .collect();
    let stale: Vec<&str> = input
        .judges
        .iter()
        .filter(|(_, _, current)| !current)
        .map(|(_, record, _)| record.input.reviewer_id.as_str())
        .collect();
    text.push_str(&format!(
        "- 评审方式：{} 位独立评审；{}；确认评审或重评：{}{}\n- 主 agent 核对说明：{}\n",
        current
            .iter()
            .map(|(_, record, _)| record.input.reviewer_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        if groups.is_empty() {
            "无".to_owned()
        } else {
            groups.join("；")
        },
        if confirmed.is_empty() {
            "无".to_owned()
        } else {
            format!(
                "{}（各取总分较低的一整组，差距取并集）",
                confirmed.join("、")
            )
        },
        if stale.is_empty() {
            String::new()
        } else {
            format!("；材料已变更未计入：{}", stale.join("、"))
        },
        table_text(&input.adjudication.notes)
    ));
    for decision in &input.adjudication.decisions {
        if decision.kind == DecisionKind::Direction {
            text.push_str(&format!(
                "- 用户决定：{} · 原话：{} · 影响：{} · 差距：{} · 来源：{}\n",
                table_text(&decision.date),
                table_text(&decision.quote),
                table_text(&decision.impact),
                decision.issue_ids.join("、"),
                applied::link(
                    file_name(&decision.source.path),
                    &decision.source.path,
                    from
                )?
            ));
        }
    }
    Ok(text)
}

/// The round report in the report contract's section order.
pub(super) fn report(input: &ReportInput<'_>) -> Result<Vec<u8>, StoreError> {
    let round = input.round;
    let from = report_path(&round.prefix, &round.round_id);
    let mut text = format!("# {} · UI 验收报告\n\n", round_label(&round.round_id));
    text.push_str(&subject(input, &from)?);
    text.push_str(&verdict_section(input)?);
    text.push_str(&scoring_section(input));
    text.push_str(&trajectory_section(input));
    text.push_str(&review_section(input, &from)?);
    text.push_str(&gap_section(input, &from)?);
    text.push_str(&pending_section(input));
    text.push_str(&suggestion_section(input));
    text.push_str(&check_section(input, &from)?);
    text.push_str(&coverage_section(input, &from)?);
    Ok(text.into_bytes())
}

/// The merged reviewer conclusions (`judge.md`): per unit the selected whole review
/// group, the open reviewer gaps and suggestions. Thresholds, verdicts, checks,
/// host findings and repair facts stay out.
pub(super) fn judge_list(
    cycle: &Cycle,
    round: &Round,
    units: &[UnitConclusion],
    prior: &[ResultRecord],
    judges: &Judges,
) -> Result<Vec<u8>, StoreError> {
    let directory = round_path(&round.prefix, ordinal(&round.round_id)?);
    let from = directory
        .parent()
        .unwrap()
        .join("judge.md")
        .to_string_lossy()
        .into_owned();
    let context = GapContext {
        cycle,
        round,
        prior,
        judges,
        machine: &[],
        from: &from,
    };
    let mut text = format!("# {} · 评审结论\n\n", round_label(&round.round_id));
    for unit in units {
        text.push_str(&format!("## {}\n\n", unit.unit_id));
        match (&unit.selected, unit.score) {
            (Some(selected), Some(score)) => {
                text.push_str(&format!(
                    "采用评审：{}\n\n| 布局 | 视觉 | 交互与状态 | 细节 | 单元分 |\n| --- | --- | --- | --- | --- |\n| {} | {} | {} | {} | {score} |\n\n",
                    unit.selected_reviewer.as_deref().unwrap_or_default(),
                    selected.scores[0],
                    selected.scores[1],
                    selected.scores[2],
                    selected.scores[3],
                ));
                for (dimension, reason) in DIMENSIONS.iter().zip(&selected.reasons) {
                    text.push_str(&format!("- {dimension}：{}\n", table_text(reason)));
                }
            }
            _ => text.push_str("本轮未评；没有单元分。\n"),
        }
        text.push_str("\n### 未关闭差距\n\n");
        let gaps = sorted(
            unit.gaps
                .iter()
                .filter(|gap| gap.origin == GapOrigin::Reviewer),
            prior,
        );
        if gaps.is_empty() {
            text.push_str("无\n");
        }
        for gap in gaps {
            text.push_str(&format!(
                "#### {}\n\n- 本轮结论：{}\n",
                title(gap, prior, false),
                conclusion_text(cycle, round, gap)
            ));
            text.push_str(&gap_lines(&context, gap, false)?);
            text.push('\n');
        }
        if !text.ends_with("\n\n") {
            text.push('\n');
        }
        text.push_str("### 建议（不扣分）\n\n");
        let mut seen = BTreeSet::new();
        for (_, record, current) in judges {
            if !current {
                continue;
            }
            for scored in record
                .input
                .units
                .iter()
                .filter(|scored| scored.unit_id == unit.unit_id)
            {
                for suggestion in &scored.suggestions {
                    if seen.insert(suggestion.as_str()) {
                        text.push_str(&format!("- {}\n", table_text(suggestion)));
                    }
                }
            }
        }
        if seen.is_empty() {
            text.push_str("无\n");
        }
        text.push('\n');
    }
    Ok(text.into_bytes())
}
