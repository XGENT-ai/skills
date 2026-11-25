# PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps · R1 代码评审

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1（初审） | [Phoenix UI P4 文档补遗与 M5 定义复原](PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md)（`sha256:5d4400aa…`，工作树版本） | 登记提交 `6a6bfb8957a7ecaf82adae8323f1f65011c192ae` 的父提交 `2f64baa6` → 该提交树 `d28891d3…`，限本期交付的 7 组文件；观察 checkout `41374ef0f88e957b657f8a43c652df3653a1230f`（main，clean） | 无 |

## 1. 确认问题

未发现可确认的问题。

本次交付是两处纯文本补遗与一行定义复原：`compare` 无回答分支在「宿主没有结构化提问工具」时的回退写法（skill 参考两个 bullet + `docs/phoenix-ui.md` 一句），以及原计划 §10 被进度行覆盖的 M5 定义行复原。四处文本口径一致、与 PRD `WORK-F10/AC-4` 的承诺一致、与 `vague-feedback.md` 的既有同口径句一致；复原行与 `0eda22cd^` 逐字节相同；渲染生成物与源一致。无 P0–P3 缺陷。

## 2. 待核实/待决策项

1. **Claude Code 宿主按计划原文复跑 V-4 第④步 —— 执行者：用户；阻塞：否（本轮已用同类宿主覆盖该分支）。**
   - 缺什么：计划 §9.1 V-4 指定的宿主是 Claude Code（读 `.env.local` 后启动）。M1 记录该步为「环境缺失/未运行」；本轮复核再次受阻：`.env.local` 把 CLI 固定在 DeepSeek（`ANTHROPIC_BASE_URL=api.deepseek.com`、`ANTHROPIC_MODEL=deepseek-flash`），`claude -p 'Reply with exactly: PONG'` 两次返回 `API Error: 402 Insufficient Balance`（request_id `f6cf5487-…`）。
   - 已尝试：载入 `.env.local` 后直接探针（同上）；随后改为在同类宿主（`codex exec`，同样没有结构化提问工具）上按 V-4 第④步的步骤实测，结果通过（见 §3 第 7–8 项）。受影响结论：无——被验证的是「宿主无结构化提问工具时按新文本在答复里再问一次并标待选」，这一分支与具体 CLI 无关。
   - 补证动作与完成条件：用户为一个可用的 Anthropic 兼容 provider 充值或另给凭据后，按 §3 第 6–8 项的命令在 Claude Code 里复跑一次，确认答复里出现同一问题、交付标 `待选`、`src/` 无改动。
2. **范围外观察：`compare --start` 退出 2 的提示语会让无工具的宿主自称「已通过结构化提问」——执行者：用户/后续需求（不在本计划范围）；阻塞：否。**
   - 证据：本轮实测中 `compare --start` 退出 2 打印「…本会话已停用浏览器页面（PHOENIX_UI_QUESTION_DISABLED），没有启动对比页；改用结构化提问。」（`/tmp/v4b-run.log:551-553`）；替代宿主据此在交付里写「我已通过结构化提问提供两个选项，尚未收到选择」（同日志 `:598`），而它并没有这样的工具，用户看不到任何「结构化」界面。
   - 影响：用户面措辞与实际机制不符，可能让用户去找不存在的控件。它由引擎提示语（`crates/context/src/compare/mod.rs` 的退出码文案）与宿主能力共同产生，**本次改动既未引入也未激活**（退出码与文案在候选 `968b6539` 起未变），且计划 §0.5 明确「引擎与页面行为本期不动」，故不计入本轮 findings；修复方向如需采纳，是让退出 2 的文案同时点明「宿主没有该工具时写进答复」，属独立需求。

## 3. 覆盖与验证

**意图一句话**：把已实测的「宿主无结构化提问工具」回退写进 `compare` 的两处权威文本，并复原被进度行覆盖的原计划 M5 定义行，不改任何运行时行为。

**仓库与环境**：`/Users/rockie/Documents/GitHub/xgent/skills`（main）；评审日期 2026-11-24；macOS arm64，mise 固定 Node 25.2.1 / Bun 1.4.4 / Rust 1.99.1。

**范围取证**：计划模式初审。登记提交只有 `6a6bfb89`，已用 `git merge-base --is-ancestor` 确认在当前分支；审查 `<该提交>^1 → <该提交>` 净差异，即 `2f64baa6..6a6bfb89`。工作树 `41374ef0`、`git status --porcelain` 为空，`git diff --check` 干净，`6a6bfb89..HEAD` 对本范围文件零改动（`git diff --stat` 仅计划文档的进度回写）。

**纳入文件（7 组，blob 取自 `6a6bfb89`）**：`skills/phoenix-ui/src/reference/compare.md`（`28067e1f`）、`skills/phoenix-ui/reference/compare.md`（`15f0f0d9`）、`skills/phoenix-ui/SKILL.md`（`8e211ea0`，仅 `source_sha256`）、`skills/phoenix-ui/GENERATED.json`（`2a5f8acd`）、`docs/phoenix-ui.md`（`d1ac2d76`）、`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md`（`c7b7201a`，仅 §10 一行）、`dist/phoenix-ui/bundle/manifest.json`（`8b1802b8`）及 34 个 `bundle/blobs/*`（渲染输出）。numstat 与声明一致：`1/1`（docs）、`1/1`（§10）、`2/2`（src）、`2/2`（生成物）、`1/1`（SKILL 元数据）、`75/75`（`GENERATED.json`：19 provider × (SKILL.md + compare.md) 输出哈希 + 输入与 bundle 哈希）、`34/34`（manifest）。

**排除项**：`6a6bfb89` 另外携带约 47 个与本期无关的文件（`skills/review-ui/**`、`.xgent-ai/sdlc/status/**` 等，属其他工作流，M1 记录已划界）；`6a6bfb89..HEAD` 的 `8b04f938`（计划进度与记录）、`5fa5c3c7`（记录）、`41374ef0`（`skills/review-code/**` 等 8 个文件，未登记、与本计划无关）均按 `git-scopes` 的进度回写/未登记口径排除。**`41374ef0` 如需纳入，另起一轮单独审查**（它改的是 review-code skill 及其复述，与本计划无共同行为链）。

**实际执行（本轮亲自运行）**：

| # | 命令 / 入口 | 目标版本 | 结果 |
| --- | --- | --- | --- |
| 1 | `cd tools/phoenix-ui && mise exec --no-deps -- bun scripts/render.mjs --check` | 工作树（= `6a6bfb89` 内容） | `status: fresh`，`providers: 19`，`inputSha256 0c362cf6…`、`bundleSha256 885801bf…`，与 `GENERATED.json`、`SKILL.md` 的 `source_sha256` 及 M1 记录一致 → 生成物与源成对，无手改 |
| 2 | `rg -n 'structured question' skills/phoenix-ui/src docs skills/phoenix-ui/reference`；回读三处 | 同上 | `compare.md:61`（`--wait` 退出 4）、`:62`（`--start` 退出 2）均含「Where the host has no structured question tool, put that same question in the reply and end the round」；`docs/phoenix-ui.md:229` 用中文同一前提；`vague-feedback.md:43` 的「where the host has one」同口径；原有「有基线标 `待选` / 无基线标 `未经用户选择`」与「No answer is never a choice」未改 |
| 3 | `python3` 逐字节比对 `git show 0eda22cd^:docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` 与当前 M5 行 | `6a6bfb89` | 两条 M5 行：§10 定义行 `equal=True`（与历史原文逐字节相同）；完成记录行仍是六列、状态「已完成」、SHA `6a6bfb89`，未被 §10 复原改动触及 |
| 4 | `sed -n '/^## 10/,/^## 11/p'` 原计划 | 同上 | M1–M5 五行均为 5 列，与五列表头对齐；M5 的 ①–⑦ 步骤与「V-6…V-9；回写『实施进度』」可读 |
| 5 | `git diff --check`；`git status --porcelain` | 工作树 | 全部干净，无临时文件混入 |
| 6 | 夹具：`node bin/xgent-skills.js install /tmp/v4b --release-dir=$PWD/local/phoenix-ui/development-release --no-phoenix-diagram --providers=.agents` | 开发构建（`sourceCommit 6a6bfb89`，`bundleSha256 885801bf…`） | 安装成功，`.agents/skills/phoenix-ui/` 73 个文件；夹具在仓库树外，`git status` 仍为空 |
| 7 | `cd /tmp/v4b && PHOENIX_UI_QUESTION_DISABLED=1 ./.agents/skills/phoenix-ui/scripts/phoenix-ui compare --start --payload .phoenix-ui/compare/two.json` | 同上 | 退出码 **2**，输出「…没有启动对比页；改用结构化提问。」；安装副本的 `reference/compare.md` 命中本轮新增句 1 次 → 复现 M1 的 V-4 ③，且证明夹具装的是本轮渲染文本 |
| 8 | 新会话只读公开文档与参考走无回答分支（替代宿主：`codex exec -C /tmp/v4b --skip-git-repo-check -s workspace-write`，同样没有结构化提问工具；日志 `/tmp/v4b-run.log`，session `01a20d8c-…`） | 同上 | 会话读到 `docs/phoenix-ui.md:253` 的新句与参考 `:360` 的新 bullet，执行 `compare --start` 得退出 2（`:551-553`），随后在答复里把同一个问题以两个选项写出（`:560-562`）、最终交付「订单页改版 **待选**，当前维持现状」（`:596`）、未实现任何方案、夹具内未新增或改动业务文件（`.phoenix-ui/` 只有原有 4 个文件，无 `*.state.json`）→ V-4 第④步的分支行为成立 |
| 9 | `claude -p 'Reply with exactly: PONG'`（载入 `.env.local`） | — | `API Error: 402 Insufficient Balance`（两次）；计划指定的 Claude Code 宿主仍不可用，见 §2 第 1 项 |

**证据分层**：第 1–5 项为静态/局部证据；第 6–8 项为真实入口证据（安装后的启动器 + 独立宿主会话），其中第 8 项是**替代宿主**的一次采样，不是计划原文指定的 Claude Code 会话。**未运行**：`test:phoenix`、四平台 CI、Rust 单测——本期不触 `tools/phoenix-ui` 任何源码（第 5 项的 diff 清单可证），与计划 §9.2 的取舍一致；跨会话遵循率统计属差异报告待核实②，计划已排除。**评审边界**：本轮只读——未修改代码、计划、记录，未提交、未推送、未跑测试写回。

**能力边界说明**：`6a6bfb89` 里约 47 个他方文件未审（§3 排除项）；`dist/phoenix-ui/bundle/blobs/*` 按增量核对（只确认它们是渲染产物、与 manifest 和 `--check` 一致），未逐字节解析其内容。

## 4. 计划符合性

计划 `docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md`，工作树版本 `sha256:5d4400aa11820b2a24803a90e8040f494cf4034257194a01ff3d41a41bb3fae9`；需求来源 PRD v1.5 · P4 · G-01/G-02。

**本次任务（M1 · 两处文档补遗）逐项核对**：

| 承诺 | 证据 | 结论 |
| --- | --- | --- |
| R-1：`compare.md`「No answer」两个 bullet + `docs/phoenix-ui.md` 无回答句 + 渲染生成物 | §3 第 2、1 项（`:61`、`:62`、`docs:229`，`render --check` fresh） | 达成 |
| R-1：与 `vague-feedback.md` 同口径、docs 只指回参考不重复整段流程 | `vague-feedback.md:43`；`docs:229` 保留「skill 的 `reference/compare.md` 同此」 | 达成 |
| R-1：与 PRD `WORK-F10/AC-4`（`docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md:391`）不冲突 | AC-4 只要求「通过结构化提问再问一次，仍无回答按基线/推荐处理」；新句是宿主缺该工具时的等价机制，`待选`/`未经用户选择` 与「不替用户提交」原样保留 | 达成（属差异报告 G-01 已批准的 B 类变更） |
| R-2 / C-2：复原 §10 该行，其余定义行、完成记录表、恢复快照与历史 SHA 不动 | §3 第 3、4 项；`git diff` 对该文件仅 `1/1` | 达成 |
| C-1：`src/` 与生成物同提交，`render --check` fresh，不手改生成物 | §3 第 1、5 项 | 达成 |

**整份计划完成核对**（用户未限定里程碑，按 `@plan_completion_gate` 覆盖全部里程碑）：

- 里程碑只有 M1（§10），退出条件是 V-1、V-2、V-3、V-4。完成记录 1/1 行状态「已完成」，提交列 `6a6bfb89` 可解析且在当前分支可达，无「待提交」行。
- V-1、V-2、V-3：本轮亲自复跑通过（§3 第 1–5 项），不是引用记录。
- V-4：第①②③步有 M1 记录且第③步本轮亲自复现（§3 第 7 项）；第④步 M1 记为「环境缺失/未运行」（计划 §11 对它预授权「记录 + 用既有证据作退路，是否阻塞：否」），本轮按 `@evidence_ownership` 在同类宿主上补做了该步（§3 第 8 项）并通过。
- §11 的另外两项风险（生成物漏提交、复原连带改其他行）由 §3 第 1、5 项与第 3、4 项排除。
- 结论：**整份计划 1/1 里程碑完成，无未完成、阻塞、待提交项**；唯一残留是 §2 第 1 项的宿主来源偏差。

**已登记偏差（非缺陷）**：计划 §10 的提交点要求一次 `docs(phoenix-ui): …` 提交且不推送；实际内容随用户侧的 `6a6bfb89`（消息 `feat: refine skills`）落盘并已推送 `origin/main`。M1 记录如实登记了来源、划了同提交内的范围边界、并说明改写已推送历史属破坏性操作且未获授权。C-1「推送只在用户授权时进行」约束的是实施者的推送动作，该提交与推送由用户自己完成，不构成规则违反。

**非阻塞观察**：计划 §0.1 账本的状态列仍为「待实施」（R-1、R-2、C-1–C-3）。该列在 `skills/dev-plan/references/plan-skeleton.md` 的取值是「已确认 / 已知 / 未知 / 阻塞 / 开放」，同类的 P3-gaps 计划用「已确认」，本计划用了「待实施」这一进度味的值；因计划自身 `@progress_writeback` 规定回写只进「恢复快照 / 完成记录 / 记录文件」，实施方未动它算守规，但读起来像未开工。不影响本轮判定，留给计划维护者决定。

**未在本轮范围**：差异报告中 G-01/G-02 的关闭（计划 C-3 明写「由 reviewer 另起复审关闭，本计划不自改其状态」）；`evals/scenarios.md` 第 26 条的判据扩写（计划 §0.5 明确排除，理由成立）；发布期安装验证与跨会话遵循率（§0.5 排除）。

## 5. 总体结论与修复顺序

**通过验收**

`patch is correct`：在明确范围内未发现影响正确性或形成阻塞的缺陷，证据足以支撑判断。四处文本口径一致并与 PRD `AC-4` 一致，复原行与历史原文逐字节相同，生成物与源成对（`render --check` fresh），`compare` 无回答分支在当前版本的安装件上按公开文档实走通过（退出 2 → 答复里再问一次 → 交付标 `待选`、保持现状、零源码改动）。整份计划 1/1 里程碑完成。

覆盖限制：`6a6bfb89` 内约 47 个他方文件、`6a6bfb89..HEAD` 的未登记提交 `41374ef0` 未审；替代宿主只有一次采样（跨会话遵循率不在本期）；计划指定的 Claude Code 宿主仍因 provider 余额受阻，见 §2 第 1 项。本结论覆盖所审提交与工作树，不是合并、发布或需求验收授权。

修复顺序：无确认缺陷可修。若要继续闭环，先由用户处理 §2 第 1 项的宿主凭据；§2 第 2 项是否立项由用户决定，不属本期。
