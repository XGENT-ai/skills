# Phoenix Diagram P1 · R4 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R4（修复效果复审） | [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md) | 修复差异：e0c68b49682d0dfc1efbc26938c94b9df9656054..6b90f881985b01ead331ef1c107bb1d3652dde23（7 个提交）。代码改动只来自两个登记的修复提交：a0e79aa（M5「修复」）和 8222b79（M7「修复」）。当前核查版本为 HEAD 6b90f88；工作树没有代码改动，`git diff HEAD` 的 SHA-256 为 `3d5ff072…` | [R3](PHOENIX-DIAGRAM-P1.code-review.r3.md) |

## 复审记录

- 方式：修复效果复审，不是完整重审。逐项核对 R3 列出的全部旧问题（R2 的 F-06、R3 的三项待核实或待决策、R2 的待核实 2 和 3、R1 的 F-01–F-05），再检查修复差异有没有引入新问题。
- 上轮：[R3 报告](PHOENIX-DIAGRAM-P1.code-review.r3.md)，SHA-256 `50e2b975…`，与 M5 处置记录登记的值一致，仍是未跟踪文件。R3 审查 `1e27a5a..e0c68b4`，验收判定为需要修改，修复结论为未完全解决，唯一的阻断项是 F-06（P3）。R1 的 SHA-256 为 `ec05fcbc…`，R2 为 `d4eef6be…`，都与之前登记的值一致。
- 修复差异：e0c68b4 之后的 7 个提交如下。
  - **a0e79aa**（修复 F-06）：改 `skills/phoenix-diagram/SKILL.md`，同步 bundle 副本、`manifest.json` 和三份 development 清单。
  - **8222b79**（修复）：M7 宿主恢复专项中，Claude 在没有资源路径时先去搜索资源或试装。这次修复把规则改成“未提供路径就只问路径并停止安装”，同时更新 SKILL.md、包内 `docs/phoenix-diagram.md`:21、计划 §5.1，并重建派生清单。
  - **5305796、aa77bf5、c48d008、72bedfb、6b90f88**：只改计划、M5/M7 记录、计划台账和经验记录。其中 c48d008 恢复了 aa77bf5 回填 SHA 时误改的 §10 M7 定义行。本轮核对过：8222b79 之后计划只改了恢复快照和 M7 完成记录两处。
  - 核对结论：`git diff c48d008 HEAD` 不含代码。代码、bundle 和随包文档在 HEAD 与最终验收源码 c48d008 上逐字相同；`tools bin scripts test package.json .github` 以及 skill 的 scripts、references、schemas、examples 自 R2 目标 1e27a5a 起没有变化。
- 当前核查版本：分支 `codex/phoenix-diagram-reader-facts-20261017`，HEAD 6b90f88，tree `9034855a`。计划与 HEAD 一致（blob `43801b19`，SHA-256 `29d9770a…`）。index 为空。tracked 改动只有两个台账文件，未跟踪文件是 R1–R3 报告和三个评审台账页，没有代码。`git status --porcelain -z` 的 SHA-256 为 `f57999eb…`。
- 已有处置记录：
  - [M5 · R3 / F-06 评审修复](PHOENIX-DIAGRAM-P1.records/M5.md#r3--f-06-评审修复)。
  - [M7 · 缺路径执行停止修正](PHOENIX-DIAGRAM-P1.records/M7.md#缺路径执行停止修正当前)。
  - [M7 · 当前收口检查点](PHOENIX-DIAGRAM-P1.records/M7.md#当前收口检查点)。
  - 下表的结论都由本轮独立核对得出，没有直接沿用记录里的说法。

## 上轮问题复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| [R2 / F-06](PHOENIX-DIAGRAM-P1.code-review.r2.md#f-06--p3-同步宿主-skill-中缺引擎时的恢复说明)（R3 续接） | [P3] SKILL.md 仍写“固定包版本的 `setupCommand`”，没有说明 candidate/development 指引中的 `MATCHING_RELEASE_DIR` 要替换。R3 判仍存在 | a0e79aa 按 distribution 分开写；8222b79 增加“未提供路径即停止”；最终包为 c48d008 / tgz `6544a703…` | **已解决** | ① 文本：[SKILL.md:69–74](../../skills/phoenix-diagram/SKILL.md#L69) 已按 R2 的修复方向分开写。release 执行回执中的固定版本命令；candidate/development 先核对是否有配套 release-dir 的绝对路径，没有就只询问并停止，有就替换 `MATCHING_RELEASE_DIR` 后执行，不改用 registry。占位符与 [bootstrap:494](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L494) 的实际输出一致。[docs:21](../../docs/phoenix-diagram.md#L21) 和计划 [§5.1](PHOENIX-DIAGRAM-P1.md#L303) 已同步；frontmatter 和描述没有变化。② 派生物：`diff -rq` 显示 skill 与 bundle 只差 `manifest.json`。manifest 中 SKILL.md 为 9504B / `5b21f455…`，与源文件一致。bundleSha256 `4805c3a4…` 等于 manifest.json 原字节的摘要。三份清单逐字相同。R2 要求手工核对的 sourceInputSha256，本轮用 `sourceInputs()` 重算：146 项，`336ed1d3…`，与清单一致。③ 运行：用留存的 development 引擎（14314240B / `b84b210a…`，与清单一致）跑 bootstrap，20/20 通过。④ 宿主行为：最终 tgz 上的六场恢复专项全部通过（`recovery-specialty.review.json`，SHA-256 `76cd3056…`）。本轮抽查了全部 typed calls：两宿主在缺路径场景都只读已装资料、跑 doctor、问路径，`installCalls=[]`；在已提供路径的场景只替换路径后安装。最终 AC trace 中 WORK-F11/AC-5 的证据已加入宿主恢复组 K（同一 SHA），R3 指出的“AC-5 证据缺宿主组”的缺口也随之补上 | 无 |
| R3 / 待决策 1 | F-06 是现在修还是本期豁免；修复会改变 bundle 和 tgz 身份，需要重新取得受影响的包证据 | 用户选择修复，不豁免（M5 记录）；M7 重开后在新源码上重新收口 | **已解决** | 按计划 [§10](PHOENIX-DIAGRAM-P1.md#L541) 重新取得了证据：源码为 c48d008，dispatch run 39353915898 产出唯一 tgz `6544a703…`，在此之上完成了两宿主各 22 场景、六场恢复专项和生产聚合。旧 cohort（1e27a5a/607735ed、5305796/7b2c5ec3）的结果没有迁入 | 无 |
| R2 / 待核实 2 | M7 新候选证据：普通 push 应 success，`await_review=true` 的 dispatch 应在原 job 内收口；同一新 tgz 的四平台、Node 18.20.8、宿主与图像审核（V-13–V-15）都要重新取得。R3 判部分解决 | M7 当前收口检查点 | **已解决**（限于下述独立核对范围） | ① Actions API：run 39353915898 的 event=workflow_dispatch，head c48d008，attempt 1，10/10 job 为 success；四个平台 consume job 的 “Resume the same delivery after actual image review” 步骤都是 success，即在原 job 内收口；Node 18 job 为 success。② 本轮下载四个 Native job 的日志重新统计：每个平台 21 组，267 pass / 0 fail / 2 ignored。③ 生产聚合 output（8452B，SHA-256 `d8ff0cfd…`）为 status=pass、handoff=passed、diagnostics=[]，四平台、node18 和两宿主均为 pass，publication=not-requested。AC trace（99486B，`462376aa…`）为 23/23 AC、46/46 子条件，pending 和 failed 均为 0，source c48d008，tgz `6544a703…`。39216914041 与 39353915898 两次 run 的 `authenticated-transfer.json` 记录的是同一个 tgz。④ 普通 push：最终源码没有成功的 push run（8222b79 的 39216737699 因同分支 dispatch 的 concurrency 被取消）。由于自 1e27a5a 起 workflow 和 scripts 都没有变化，R3 已核的 push run 39142463831 结论（审核续接步骤被跳过）仍然适用 | 未独立复核的部分：44 场选中宿主会话的语义判定、160 张原图审核、聚合内部回放。这些证据在 gitignored 的 `local/` 和 `~/.local/share/xgent/` 下，只能在本机取回。这是计划验收证据，不是代码缺陷 |
| R2 / 待核实 3（R3 续接） | 评审者未重跑完整 test:diagram、test:phoenix，也没有在真实 Windows 上运行。R3 判部分解决 | — | **部分解决** | Rust 原生测试已在最终源码的四平台 CI 中跑过（见上一行）；本轮在 HEAD 跑了完整 `npm test` 和真实引擎下的 bootstrap，结果见「覆盖与验证」。修复差异只改说明文本和派生清单，不触及这两套测试覆盖的代码 | 本地 test:diagram、test:phoenix 和真实 Windows 仍未由评审者运行；本轮不要求补跑 |
| R3 / 待核实 2 | core-runner 计时用例偶发失败（观察，不计 finding） | 测试没有改 | **待核实** | 本轮完整 `npm test` 跑 1 次，0 fail；M5、M7 记录中的多次运行也都是 0 fail。一次通过不能排除偶发失败，也没有新的复现。该用例在 R1 范围引入，本轮差异没有触及 | 如果要把 npm test 当作稳定门禁，可以另行放宽期限或改为等待子进程事件 |
| R3 / 待核实 3（= R2 / 待核实 1） | 联合发行合同建立前，任何 npm 发布都会被阻断（决策提醒，不计缺陷） | 维持计划 §8 的设计 | **仍存在**（决策项，不是缺陷） | 发布 guard 代码自 1e27a5a 起没有变化 | 合并到 main 或公开发布前，由维护者确认 |
| [R1 / F-01–F-05](PHOENIX-DIAGRAM-P1.code-review.md)（R2 已判关闭，R3 维持） | 见 R1 | 6904c9a | **已解决**（维持） | 相关代码自 1e27a5a 起没有变化。F-01 另有最终源码 dispatch 在原 job 内收口的证据；F-02 另有最终 tgz 上宿主按回执恢复的证据（4→0→0，见 F-06 一行的 ④） | 无 |

## 确认问题（本轮新增）

未发现可确认的问题。

修复差异里的代码只有 SKILL.md 中一段说明文本、包内文档的一句话，以及据此重建的 bundle 和三份清单。本轮已核对：文本与 bootstrap 的实际输出、计划 §5.1 一致；派生身份与当前源码一致；frontmatter 没变；恢复说明在仓内的其他复述（native doctor 的 [cli.rs:290](../../tools/phoenix-diagram/src/cli.rs#L290)、bootstrap 的 help 输出）与新规则不冲突。

## 待核实 / 待决策项

1. **联合发行阻断（待决策）**：沿用 R2 / R3。合并到 main 或公开发布前需要维护者确认，不是代码缺陷。
2. **core-runner 计时偶发失败（观察）**：沿用 R3 待核实 2。本轮没有复现，也不能证伪。这是 R1 范围内的既有测试，不在本轮差异中。

## 覆盖与验证

- 变更意图：关闭 R2 / F-06，并按 M7 宿主实测把“缺资源路径时先停止”写成明确规则；用新的 clean 候选重新完成 P1 收口。产品 runtime 不变。
- 仓根 `/Users/rockie/Documents/GitHub/xgent/skills`，日期 2026-10-20。输入为 `/review-code docs/plan/PHOENIX-DIAGRAM-P1.md`。开发计划模式下本计划已有 R1–R3，按轮次最大的 R3 做修复效果复审。修复差异从 R3 的目标 e0c68b4 算到 HEAD；登记为「修复」的提交是 a0e79aa（M5 行）和 8222b79（M7 行），72bedfb 是 M7 的普通登记提交，三者都在 HEAD 的祖先链上。没有未登记的代码提交。
- 已读：
  - e0c68b4..HEAD 的完整代码差异：SKILL.md、包内 docs、三份清单和 bundle manifest。
  - 计划 §5.1、§0.6、§10 的相关行，以及 8222b79 之后计划的全部差异。
  - M5 的 R3 处置节；M7 中 R3 修复后的各节、缺路径停止修正和当前收口检查点。
  - bootstrap 的 `setupCommand` 与 help。
  - `build-phoenix-diagram.mjs` 中 `sourceInputs`、`bundleFiles` 和 sourceCommit/sourceState 的生成逻辑。development 清单的 sourceCommit 记的是构建时的 HEAD，dirty 状态写在 build-receipt 里，这是既有设计，不是本轮引入。
  - 台账协议和相关状态页。
- 实际执行（HEAD 6b90f88，Node 25.2.1）：
  - `npm test`：exit 0，474 项，451 pass / 0 fail / 23 skip；Python 四组 22/24/2/13 全部 OK。
  - `PHOENIX_DIAGRAM_BIN=<local/…/m7-explicit-recovery-stop-development/phoenix-diagram-darwin-arm64> node --test test/phoenix-diagram/bootstrap.test.js`：20/20，0 skip。
  - `node --input-type=module` 调用 `sourceInputs()` 和 `bundleFiles()`：146 项输入 `336ed1d3…`，19 个 bundle 文件 `4805c3a4…`，manifest 原字节相同。
  - `diff -rq skills/phoenix-diagram dist/phoenix-diagram/bundle`、`cmp` 三份清单。
  - `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P1.md`：7/7，ERROR 0 / WARN 0。
  - `gh api`（经本机代理）：读取 run 39353915898 及其 jobs 和 steps，下载四个 Native job 的日志并汇总 `test result` 行；列出 workflow 最近 12 次 run。
  - 本机证据：计算聚合 output、AC trace 和恢复专项审核的 SHA-256，读取其顶层字段；逐条读取六场恢复会话的 typed calls、installCalls 和最终响应摘要；核对 AC trace 中 AC-5 的证据组 K。
- 未运行：本地 test:diagram、test:phoenix、真实浏览器、真实 Windows 和宿主会话；没有重放生产聚合。
- 残余风险：仓内没有静态测试锁定 SKILL.md 中这段恢复说明（R2 当时列为可选断言），目前只靠 M7 的宿主恢复场景保护。以后改 SKILL.md 时需要留意。
- 证据限制：M7 原件都在 gitignored 的 `local/` 和 `~/.local/share/xgent/` 下，只能在本机取回。CI 日志受 GitHub 保留期限制。临时日志写在会话 scratchpad，没有写入仓库。本轮由单个 agent 完成，没有使用并行 reviewer。

## 计划符合性

计划为 [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md)（HEAD 6b90f88，blob `43801b19`），7/7 完成。需求来源为 PRD P1 的 23 条 AC。下表只列与本轮相关的项。

| 需求 / 章节 | 本次任务 | 实现证据 | 验证状态 |
| --- | --- | --- | --- |
| WORK-F11 AC-5 / D7、§5.1：缺引擎时给出可执行、身份固定的恢复指引 | M5 修复 a0e79aa、M7 修复 8222b79 | [SKILL.md:69–74](../../skills/phoenix-diagram/SKILL.md#L69)、[docs:21](../../docs/phoenix-diagram.md#L21)、[bootstrap:485–495](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L485) | 已核实：bootstrap 20/20；最终 tgz 六场宿主恢复通过，AC trace 中 AC-5 含宿主组 K |
| §10：最终验收修复产生新 SHA 时重新取得受影响的原生/包证据 | M7 | CI 39353915898、tgz `6544a703…`、生产聚合 | 已核实：新源码重新收口，旧 cohort 没有迁入 |
| V-14：同一 tgz 的四平台原生构建与消费、显式审核收口 | M7 | [phoenix-diagram.yml](../../.github/workflows/phoenix-diagram.yml)、run 39353915898 | 已核实：10/10 job success，4 × 267/0/2，四个审核续接步骤 success |
| V-13、V-15：两宿主、图像审核与 23 AC | M7 | 生产聚合、AC trace、恢复专项 | 部分核实：顶层结论、摘要与恢复专项 typed calls 已核；会话语义与原图审核未独立复核 |
| V-16：轻量回归 | M7 | npm test、bootstrap | 已核实 |
| 提交点：修复与回填分开提交，§10 定义不被回填改写 | M5、M7 | a0e79aa、8222b79；5305796、aa77bf5、c48d008、6b90f88 | 已核实：aa77bf5 误改的 §10 M7 行已由 c48d008 恢复；此后计划只改进度 |
| 计划 [§0.6 bootstrap/setup 行](PHOENIX-DIAGRAM-P1.md#L159) | — | 该行仍写“缺引擎给固定包版本指引” | 文本残留，不阻断：该行早于 D7 的 R1 修订，与 D7、§5.1 的按 distribution 区分不一致，也是 F-06 原文的来源。下次修订计划时可以同步，避免再被照抄。PRD AC-5 的同类措辞由 D7 经用户确认后适配，不算缺口 |
| P2/P3 的新增类型、迁移与公开发行 | — | — | 后续范围，不计缺陷 |

## 总体结论与修复顺序

**验收判定：通过验收**

修复差异 e0c68b4..HEAD 的结论为 **patch is correct**。修复结论为**已解决**：

- **旧问题**：R1 的 F-01–F-05 维持已解决；R2 的 F-06 已按原修复方向关闭，有文本、派生身份、真实引擎和最终包宿主恢复四方面证据。R3 的待决策 1 已按用户选择执行并重新收口。
- **新问题**：修复差异没有引入新问题。
- **未覆盖范围**：宿主会话语义、原图审核和聚合内部回放没有独立复核；本地 test:diagram、test:phoenix 和真实 Windows 未运行。

没有需要修复的项。仍需留意的两件事：

1. 合并到 main 或公开发布前，由维护者确认联合发行阻断。
2. 如果 npm test 要作为稳定门禁，可以另行处理 core-runner 计时用例。

本结论只覆盖所审提交与工作树，不构成合并、发布或需求验收的授权。
