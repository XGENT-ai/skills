# Phoenix Diagram P1 · R3 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3（修复效果复审） | [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md) | 修复差异：1e27a5ac4a6f7c7004ea0af7a402170e6130a23f..e0c68b49682d0dfc1efbc26938c94b9df9656054（6b980c8、e0c68b4），不含代码；当前核查版本 HEAD e0c68b4，工作树只有评审报告和台账的改动 | [R2](PHOENIX-DIAGRAM-P1.code-review.r2.md) |

## 复审记录

- 方式：修复效果复审，不是完整重审。逐项核对 R2 的 F-06 和三项待核实，确认 R2 关闭的 R1 F-01–F-05 是否仍然成立，并检查修复差异是否引入新问题。
- 上轮：[R2 报告](PHOENIX-DIAGRAM-P1.code-review.r2.md)，SHA-256 `d4eef6be003dfa84eddbdc03993fa5038dcc542f6fd52a1cf50526790a4c6021`，仍是未跟踪文件。R2 审查 `51c6379..1e27a5a`，验收判定为通过验收，修复差异 patch is correct，新增 F-06（P3）。更早的 [R1 报告](PHOENIX-DIAGRAM-P1.code-review.md) SHA-256 为 `ec05fcbc…`，与 R2 登记的值一致。
- 修复差异：从 R2 的目标 1e27a5a 到 HEAD e0c68b4，共 2 个提交。6b980c8 是 M7 完成记录登记的提交，改了 4 个文件：计划、M7 记录、计划台账页、`.xgent-ai/experience` 经验记录。e0c68b4 只回填计划中的 SHA。`git diff 1e27a5a HEAD -- skills tools bin scripts dist test .github package.json docs/phoenix-diagram.md` 输出为空：代码、bundle、清单和随包文档与 R2 所审目标逐字相同。完成记录中没有新的实现提交或「修复」提交。
- 当前核查版本：HEAD e0c68b4（tree `5dd20c8e`），分支 `codex/phoenix-diagram-reader-facts-20261017`。计划文件与 HEAD 一致（blob `d2c8ef14`，SHA-256 `8d0602b6…`）。工作树中 tracked 改动只有两个台账文件（`git diff HEAD` 的 SHA-256 为 `9858e3a2…`），index 为空；未跟踪文件是 R1、R2 报告和两个评审台账页。没有代码改动。
- 已有处置记录：M7 的[修后候选续验（当前）](PHOENIX-DIAGRAM-P1.records/M7.md#修后候选续验当前)和[修后候选最终23AC与P1收口](PHOENIX-DIAGRAM-P1.records/M7.md#修后候选最终23ac与p1收口)。前一节写明“未自行发起代码评审或处理可选 F-06”。下表结论都由本轮独立核对得出。

## 上轮问题复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| [R2 / F-06](PHOENIX-DIAGRAM-P1.code-review.r2.md#f-06--p3-同步宿主-skill-中缺引擎时的恢复说明) | [P3] [SKILL.md:67](../../skills/phoenix-diagram/SKILL.md#L67) 仍写“固定包版本的 `setupCommand`”，没有说明 candidate/development 指引中的 `MATCHING_RELEASE_DIR` 需要替换 | 未处理。M7 记录写明未处理可选 F-06；修复差异没有触及 `skills/` | **仍存在** | SKILL.md:67 在 HEAD 与 1e27a5a 逐字相同，bundle 副本与源文件逐字相同（`cmp`）；[bootstrap:494](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L494) 仍输出占位符。M7 的最终 tgz 恢复证明 `~/.local/share/xgent/phoenix-diagram-acceptance/recovery-1e27a5a-xTrpi7/recovery-proof.json` 中，同一 tgz 给出的 setupCommand 以 `--release-dir 'MATCHING_RELEASE_DIR'` 结尾，验收者把它换成配套目录后才执行成功。最终 AC trace 中 WORK-F11/AC-5 的证据组为 C+K+N+F+L5，没有宿主组 H，实现定位也不含 SKILL.md。也就是说，两宿主的 44 个入选场景都没有覆盖“宿主按 SKILL.md 转述恢复命令”这条路径，R2 的影响判断仍成立 | 按 R2 的修复方向处理，或由用户明确豁免。修复会改变 bundle 和最终 tgz 的身份，代价见待决策 1 |
| R2 / 待核实 1 | 联合发行合同建立前，任何 npm 发布都会被阻断（决策提醒，不计缺陷） | 维持计划 §8 的设计。M7 最终收口写明公开 npm/R2 发行保持原授权边界 | **仍存在**（决策项，不是缺陷） | guard 代码未变 | 合并到 main 前由维护者确认 |
| R2 / 待核实 2 | M7 新候选证据：普通 push 应 success、`await_review=true` 的 dispatch 应在原 job 收口；同一新 tgz 的四平台、Node 18.20.8、宿主与图像审核（V-13–V-15） | M7 已标完成：push run 39142463831、dispatch run 39142536978、唯一 tgz `607735ed…` | **部分解决** | 本轮独立核实了四项。① `gh run view` 与 jobs API：39142463831 的 event=push、head 1e27a5a，10 个 job 全部 success，四个 consume job 的 “Resume the same delivery after actual image review” 步骤均为 skipped，这是 F-01 修复的真实效果；39142536978 的 event=workflow_dispatch、head 1e27a5a，10 个 job 全部 success，四平台的同一步骤均为 success，耗时 11–39 分钟，即显式 dispatch 在原 job 内收口。② 四个 Native job 的日志各有 21 组 `test result`，每个平台都是 267 pass / 0 fail / 2 ignored。③ F-02 的最终 tgz 恢复证明：source 1e27a5a、tgz `607735ed…`，doctor 退出 4，恢复退出 0，再跑 doctor 退出 0，项目目录仍为空，引擎 `da454867…` 与记录一致。④ 最终生产聚合 output（8452B，SHA-256 `5eb5964b…`，与记录一致）为 status=pass、handoff=passed、diagnostics=[]；最终 AC trace（450635B，SHA-256 `9554b933…`，与记录一致）为 23/23 AC、46/46 条件通过，pending 为 0。未独立复核的部分：57 场真实宿主会话的语义判定、160 张原图审核和聚合内部的回放；这些只核到文件存在、摘要和顶层结论 | 证据都在 gitignored 的 `local/` 和用户目录下，只能在本机取回。这些属于计划验收证据，不是代码缺陷，不阻断本轮判定 |
| R2 / 待核实 3 | 未重跑完整 test:diagram、test:phoenix 和真实 Windows | — | **部分解决** | CI 39142536978 已在 1e27a5a 上跑完四平台原生 Rust 测试（见上一行）。本轮在 HEAD 跑了三次完整 `npm test`，代码树与 1e27a5a 相同：两次 exit 0（474 项，451 pass / 23 skip / 0 fail；Python 四组 22/24/2/13 全部 OK），一次 exit 1，失败在 core-runner 计时用例，见待核实 2 | 本地完整 test:diagram、test:phoenix 和真实 Windows 仍未由评审者运行。本轮没有代码改动，不强制补跑 |
| [R1 / F-01–F-05 及待核实 1、3](PHOENIX-DIAGRAM-P1.code-review.md)（R2 已判关闭） | 见 R1；R2 判 F-01–F-05 已解决，待核实 3 已解决，待核实 1 不再适用 | 6904c9a（R2 已复核） | **已解决**（维持 R2 结论） | 相关代码自 R2 的目标版本以来没有变化，R2 的关闭证据仍然适用。F-01、F-02 另外有了真实 CI 和最终 tgz 的证据（见「R2 / 待核实 2」一行） | 无 |

## 确认问题（本轮新增）

未发现可确认的问题。本轮差异只有计划、M7 记录、计划台账和经验记录，没有代码、bundle 或随包文档的改动。

## 待核实 / 待决策项

1. **F-06 的处理时机（待决策）**：SKILL.md 是 bundle 的 19 个文件之一，也是 146 项构建源码输入之一。改它会改变 development 清单、bundle manifest 和最终 tgz 的身份。计划 [§10 里程碑与验收安排](PHOENIX-DIAGRAM-P1.md#L541) 规定“最终验收修复产生新 SHA 时重新取得受影响原生/包证据”。所以现在修 F-06，就要用新 tgz 重新取得受影响的 V-14/V-15 证据；宿主读到的正是 SKILL.md，V-13 证据是否仍适用也需要重新判断。另一种做法是本期豁免 F-06，留到下一期或首次公开发行前处理，按报告合同记为“用户豁免通过”。需要维护者选择。
2. **npm test 的计时偶发失败（观察，不计 finding）**：第一次完整 `npm test` 中，[core-runner.test.js:32](../../test/phoenix-diagram/core-runner.test.js#L32) 失败，用时 4216ms，期望错误码 `core/checkpoint-timeout`，实际为 `ERR_ASSERTION`。onSpawn 在 [第 20 行](../../test/phoenix-diagram/core-runner.test.js#L20)给复制出的 Node 子进程 3 秒期限写 ready 标记，断言在 onSpawn 内抛出，被当作 rejection 收集。单独运行这个测试文件 3 次都通过，该用例用时 1.8–2.2s；另两次完整 `npm test` 也通过，用时 2.4–2.7s。推断是负载下复制的 Node 启动接近 3 秒期限，偶尔超时；这一点没有进一步证实。该测试在 R1 范围内引入（3c8c3ac、2c133da），本轮差异没有改它，因此不作为本轮 finding。R1、R2 和 M7 记录的 npm test 都是 0 fail。如果要把 npm test 当作稳定门禁，可以另行放宽期限，或改为等待子进程事件。
3. **联合发行阻断**：沿用 R2 待核实 1，合并到 main 前需要维护者确认，不属于代码缺陷。

## 覆盖与验证

- 变更意图：记录修后候选 1e27a5a 的 M7 最终验收（同一 tgz 的四平台、Node 18.20.8、两宿主各 22 场景、生产聚合和 23 AC），把计划标为 7/7 完成；不改产品代码。
- 仓根 `/Users/rockie/Documents/GitHub/xgent/skills`，日期 2026-10-19。输入为 `/review-code docs/plan/PHOENIX-DIAGRAM-P1.md`。开发计划模式下本计划已有 R1、R2 两份代码评审报告，按轮次最大的 R2 做修复效果复审。范围和指纹见「复审记录」。
- 已读：
  - 6b980c8、e0c68b4 的完整 diff：计划的恢复快照与完成记录，M7 新增的两节，计划台账页，经验记录 E003–E009。
  - HEAD 的 SKILL.md:67 与 bootstrap `setupCommand`。
  - 工作树中的台账改动。
- 实际执行（HEAD e0c68b4，代码树与 1e27a5a 相同）：
  - `npm test` 三次：第 1 次 exit 1（474 项，450 pass / 1 fail / 23 skip，Python 未运行）；第 2、3 次 exit 0（451 pass / 0 fail / 23 skip，Python 22/24/2/13 OK）。
  - `node --test test/phoenix-diagram/core-runner.test.js` 三次：均为 24/24。
  - `gh run view` 与 jobs API 查询 39142463831、39142536978 的 run、job 和审核续接步骤结论；用 `gh api .../actions/jobs/<id>/logs` 取四个 Native job 的日志，汇总 `test result` 行。
  - 本机证据：读取恢复证明的 `recovery-proof.json` 和 `recovery-command.json`；计算最终生产聚合 output 和最终 AC trace 的 SHA-256，并读取其顶层字段。
  - `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P1.md`：7/7，里程碑表 7 行，ERROR 0 / WARN 0。
- 未运行：本地 test:diagram、test:phoenix、真实浏览器、真实 Windows 和宿主会话；没有重放生产聚合。
- 证据限制：
  - M7 证据（CI 下载、宿主原档、聚合 output、AC trace、恢复证明）都在 gitignored 的 `local/` 和 `~/.local/share/xgent/` 下，只能在本机取回。本轮只核到文件存在、摘要和顶层结论。
  - CI 日志受 GitHub 保留期限制。
  - 临时日志放在会话 scratchpad，没有写入仓库。
  - 由单个 agent 完成，没有使用并行 reviewer。

## 计划符合性

计划为 [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md)（HEAD e0c68b4，blob `d2c8ef14`），7/7 完成。需求来源为 PRD P1 的 23 条 AC。下表只列与本轮相关的项。

| 需求 / 章节 | 本次任务 | 实现证据 | 验证状态 |
| --- | --- | --- | --- |
| WORK-F11 AC-5 / D7、§5.1：缺引擎时给出可执行、身份固定的恢复指引 | M4、M5；M7 的 K 组 | [bootstrap:485–495](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L485)、[cli.rs:288](../../tools/phoenix-diagram/src/cli.rs#L288)、[install/plan.rs:149](../../tools/phoenix-diagram/src/install/plan.rs#L149) | 已核实：最终 tgz 恢复为 4→0→0。宿主说明的缺口 F-06 仍存在 |
| §5.6 / V-14：常规 CI 不等待人工审核，显式 dispatch 收口 | M6、M7 | [phoenix-diagram.yml](../../.github/workflows/phoenix-diagram.yml) | 已核实：两个 run 的 job 与步骤结论 |
| V-14：同一 tgz 的四平台原生构建与测试 | M7 | CI 39142536978 的 Native job | 已核实：4 × 267 pass / 0 fail / 2 ignored |
| V-13、V-15：两宿主、图像审核与 23 AC | M7 | 最终生产聚合、最终 AC trace | 部分核实：摘要和顶层结论与记录一致；会话语义与原图审核未独立复核 |
| V-16：轻量回归 | M7 | npm test | 已核实，其中一次计时偶发失败（待核实 2） |
| §10 提交点：最终提交只整理记录、回填 SHA | M7 | 6b980c8、e0c68b4 | 已核实：两个提交都没有代码改动，最终代码树与验收源码 1e27a5a 相同 |
| P2/P3 的新增类型、迁移与公开发行 | — | — | 属后续范围，不计缺陷 |

## 总体结论与修复顺序

**验收判定：需要修改**

本轮差异不含代码，不给 patch 结论。修复结论为**未完全解决**：R1 的 F-01–F-05 维持已解决，其中 F-01、F-02 新增了真实 CI 和最终 tgz 证据；R2 的 F-06（P3）仍然存在。报告合同要求复审选定的旧问题须关闭或明确豁免，所以本轮不能判通过。本轮没有 P0–P2 问题，也没有新增问题。

唯一阻止通过的是 F-06（P3）。它只影响宿主读到的恢复说明，不影响已验收的代码行为。两种走法：

1. 修 F-06：执行 `/apply-code-review docs/plan/PHOENIX-DIAGRAM-P1.code-review.r3.md`，按 R2 的方向更新 SKILL.md，重新生成 development 制品，并同步 bundle 与三份清单；再按计划 §10 用新 tgz 重新取得受影响的包证据，然后复审。
2. 本期放过 F-06：用户明确表示后，在本报告追加豁免记录，验收判定改为“用户豁免通过”，F-06 留到下一期或首次公开发行前处理。

另外，合并到 main 前需确认联合发行阻断（待核实 3）。本结论只覆盖所审提交与工作树，不构成合并或发布授权。
