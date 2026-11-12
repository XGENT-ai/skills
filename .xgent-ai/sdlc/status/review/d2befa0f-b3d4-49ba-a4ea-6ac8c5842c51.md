# Phoenix UI 中文排版与口径 P2 · PRD 交付覆盖核查状态

- 对象 ID / 类型：`d2befa0f-b3d4-49ba-a4ea-6ac8c5842c51` / `review`；review_type: `other`（PRD 与开发交付差异核查，首次完整 P2 核查）。
- 状态模型：协议 v4 / state-model v3。
- 来源：[完整差异报告](../../../../docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.prd-dev-gaps.md)。
- 导航分类：活动。

## review / report

- 状态：`review-recorded`；来源自述：本期全部需求与验收条件已覆盖，A=0、B=3、C=1（已决 1 / 待决 0），待核实 0，follow-up 4（1 项已定增量并入下期、3 项发布后观察）；证据质量：`confirmed`。
- 核查：2026-11-11T21:51+08:00；观察 checkout：main / `9b07ef066c9ebd00eea83587756f44532d1d9d75`，dirty（台账、评审与差异报告未跟踪或已修改；`tools/phoenix-ui/crates/` 与 HEAD 无差异）。
- 所审范围：PRD v1.2 `sha256:09b19f65376834f46480a3dec8394e8ab1a18ba2707f0399da0e9be0caad1e25` P2 的 WORK-F5/AC-1–AC-7、WORK-F6/AC-1–AC-4、WORK-N3/AC-3（含 AC-1/AC-2 持续满足与 **P2 终验**）、WORK-N4/AC-2、UAT-7/8/17/14，及 WORK-N1/N2 对本期新代码的持续约束；原计划 `sha256:84e2aa2a429d08d4c036ec89db3ca326fed51226575d08e24d1467c0dcab46db`（已提交，4/4 里程碑，Ready）。
- 依据：报告 `sha256:a683752b09132a31b44b9a080062d1b24609a5911446f25c1afc6d3f926568b9`。本轮独立实测：`local/phoenix-ui/r3-development-release`（引擎 `sha256:530ddf6661f4aff5…`，构建自 dirty@`2518626`，与 CI 候选 `cd60330` 引擎代码一致）对 10 个夹具页运行 `detect`（含 `--no-advisory`、`detector.ignoreRules` 各一次），静态核实规则/让位/备注通道与全部文档落点；CI run `43007878500` 与 R6 结论在输入未变下复用，未重跑。
- 当前差异：G-01 按意群断句 span 使文字退出候选范围（C，已决 D-01：范围不变、typeset 补一句指引、增量并入下期）；G-02 字体名按声明名匹配的本地化误报风险（B，发布后观察）；G-03 `cjk-split-unit` 封闭单位词表（B，按反馈增补）；G-04 Linux 两类系统字体判定仅 macOS 自动化证据（B，M4 A-1 收口为发布后观察）。
- 关系：`reviews` → [PRD d6f09ace-6158-4eb2-a8e3-465807145888](../prd/d6f09ace-6158-4eb2-a8e3-465807145888.md)，scope=P2；`reviews` → [P2 计划 ddfc2db0-1081-4c9e-8cee-8aecf3179c7e](../plan/ddfc2db0-1081-4c9e-8cee-8aecf3179c7e.md)。前轮 [P1 覆盖核查 87e10b05-b870-415b-97a0-64298c33c5e3](87e10b05-b870-415b-97a0-64298c33c5e3.md) 为独立对象，本轮不复审其 G-ID；R1–R6 代码评审与计划评审原判定不变。
- 限制：未重跑 `test:phoenix`、CI 或宿主会话；报告与台账未提交。
- 下一步：F-01（typeset「Chinese copy」断行补一句手法提示 + 渲染生成物）并入 P3 计划或下次动 skill 时带上；无需 gaps plan。

## disposition / report

- 状态：`review-recorded`；来源自述：无待决 Triage；D-01 已登记并确定去向；证据质量：`confirmed`。
- 核查：2026-11-11T21:51+08:00；基线同上。
- 去向：G-01 → F-01 并入下期（用户 2026-11-11 交互答复选择「补指引提示」）；G-02/G-03/G-04 → 发布后观察（计划 §11 对应行维持）。WORK-N3 终验条件在本报告基线内成立，「已上线」流转待 SPMS 录入（外部依赖，PRD §8）。
