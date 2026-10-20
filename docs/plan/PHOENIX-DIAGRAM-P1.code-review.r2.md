# Phoenix Diagram P1 · R2 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2（修复效果复审） | [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md) | 修复差异：51c637997c221fe99765159dce9e6a8571ff9b10..1e27a5ac4a6f7c7004ea0af7a402170e6130a23f，其中代码只来自完成记录登记的「修复 6904c9a」；无未提交代码 | [R1](PHOENIX-DIAGRAM-P1.code-review.md) |

## 复审记录

- 方式：修复效果复审，不是完整重审。逐项核对 R1 的 F-01–F-05 和三项待核实，并检查修复差异是否引入新问题。
- 上轮：[R1 报告](PHOENIX-DIAGRAM-P1.code-review.md)，SHA-256 `ec05fcbc19ed2c338c311a8bdff9da33846ef83a065685ef6a125be666798d35`，与 M5/M6/M7 修复记录登记的值一致，内容未改。R1 审查范围为 `4a65a15..51c6379`，结论是需要修改（2 P2 / 3 P3）。该报告仍是未跟踪文件。
- 修复差异：从 R1 的目标 51c6379 到当前 HEAD 1e27a5a，共 4 个提交：00f25d3 只改计划，5bb5668 只改台账，6904c9a 为登记的「修复」提交（26 个文件），1e27a5a 只改计划。排除计划、记录、台账和 `.xgent-ai/experience` 后，代码与随包文档共 19 个文件，+236/−42。完成记录中没有 R1 未覆盖的新实现提交；M1–M7 的原 SHA 都在 R1 范围内。
- 当前核查版本：HEAD 1e27a5a，分支 `codex/phoenix-diagram-reader-facts-20261017`。计划文件与 HEAD 一致（SHA-256 `7359bdc1244cdbe2a5dc756e30f0f57770527c5d85f59731a516279fc1032d88`）。工作树只有 R1 报告和三个台账文件的未提交改动（`git diff HEAD` SHA-256 `8da45d7a…`，另有两个未跟踪文件），没有代码改动。
- 已有处置记录：[M5 评审修复](PHOENIX-DIAGRAM-P1.records/M5.md#r1-评审修复)、[M6 评审修复](PHOENIX-DIAGRAM-P1.records/M6.md#r1-评审修复)、[M7 当前缺口](PHOENIX-DIAGRAM-P1.records/M7.md#r1-修复与当前验收缺口)、[M4 doctor 接缝](PHOENIX-DIAGRAM-P1.records/M4.md)。下表结论都是本轮独立核对的结果，没有直接照搬记录。

## 上轮问题复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01 | [P2] 每次 push/PR，四个 consume job 都要等 85 分钟人工审核，超时后失败 | 6904c9a：新增 `workflow_dispatch.inputs.await_review`（boolean，默认 false）。审核续接条件加上 `github.event_name == 'workflow_dispatch' && inputs.await_review == true`。阶段回执的 state 路径在审核跳过时回退到 consume 的 state | **已解决** | [phoenix-diagram.yml:7–11](../../.github/workflows/phoenix-diagram.yml#L7)、[497](../../.github/workflows/phoenix-diagram.yml#L497)、[512](../../.github/workflows/phoenix-diagram.yml#L512)。push/PR 下条件为 false，该步骤 skipped；之后两个 `if: always()` 步骤照常写回执、上传，`ci-prepare/` 目录总会写入，所以 job 不会因审核门失败。GitHub 文档说明：`inputs` 只在 reusable workflow 和 `workflow_dispatch` 中可用，且“preserves Boolean values as Booleans”。[workflow.test.js:176–187](../../test/phoenix-diagram/workflow.test.js#L176) 用实际条件字符串跑真值表（push/PR/dispatch × true/false，以及前置步骤失败），在 npm test 中通过 | 6904c9a 未推送，远端分支仍在 7f3136c，没有新 run。普通 push 应 success，`await_review=true` 的 dispatch 应在原 job 收口；这两项属 M7 的 V-14 验收证据，不影响本条代码关闭 |
| R1 / F-02 | [P2] 缺引擎时的恢复命令指向 registry 上已发布、不含 phoenix-diagram 的 0.7.4，执行后 `command not found` | 6904c9a：`setupCommand` 只在 distribution=release 时输出 registry 命令；candidate/development 输出当前 node + 本地包 bin 或已装 skill launcher + `--release-dir MATCHING_RELEASE_DIR`。bootstrap 用 `PHOENIX_DIAGRAM_SETUP_COMMAND` 把同一指引传给原生 doctor。安装事务回执的 `recoveryCommand` 有同一根因，也一并修正。development 离线目录只校验清单声明的平台 | **已解决** | 定位见 [phoenix-bootstrap.cjs:485–495](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L485)、[537](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L537)，[cli.rs:288–295](../../tools/phoenix-diagram/src/cli.rs#L288)，[install/plan.rs:149–161](../../tools/phoenix-diagram/src/install/plan.rs#L149)。本轮在隔离项目和空 `PHOENIX_DIAGRAM_HOME` 下实测三条路径：① 仓内 `bin/phoenix-diagram.js doctor --json` 退出 4，`setupCommand` 指向本地 bin；替换目录后执行退出 0；再跑 doctor 退出 0，`recovery.engineInstall` 与 setupCommand 完全相同，项目目录只剩 `.git`。② 用 `xgent-skills install --diagram-release-dir` 装入 `.claude` 后删除 cache；已装 launcher 给出指向自身的指引，替换目录后执行退出 0，doctor 退出 0。③ 安装回执的 `recoveryCommand` 是缓存引擎的绝对路径，原样执行退出 0，状态 `unchanged`。candidate 分支由真实引擎下的 bootstrap 原生测试覆盖（20/20） | 新 tgz 的 V-14/V-15 归 M7。宿主 skill 的说明没有同步，见本轮 F-06 |
| R1 / F-03 | [P3] 三份 development 清单的 bundleSha256 过期，检出内的普通命令报 bundle 摘要不符 | 6904c9a：三份清单按实际构建的 development 制品同步（bundle `10dec879…`、引擎 `a1932f1b…`、源码输入 `807e97d8…`）；build.test 增加摘要绑定断言和三份清单逐字相同的断言 | **已解决** | doctor 现在报缺引擎，不再报摘要不符。用 build 脚本的 `sourceInputs()` 重算当前树的 146 项输入，SHA-256 为 `807e97d8…`，与制品的 `source-inputs.json` 逐字相同。制品引擎的 SHA-256 与清单一致。[build.test.js:205–209](../../test/phoenix-diagram/build.test.js#L205) 通过 | 无 |
| R1 / F-04 | [P3] 不支持的平台上，默认启用的 diagram 分支阻断 `--no-phoenix-ui` 安装 | 6904c9a：未显式要求 diagram 资源时跳过 diagram 并说明；显式传 `--diagram-release-dir` 时仍在写项目前失败 | **已解决** | 定位 [xgent-skills.js:468–472](../../bin/xgent-skills.js#L468)。本轮模拟 win32：`--no-phoenix-ui --no-xgent-init` 退出 0，打印跳过说明，只写入 `.claude` 的 hooks 和 settings；加上 `--diagram-release-dir` 时退出 4，报 Unsupported platform，目录为空，也不再给出无效的离线目录建议。[install.test.js:24–40](../../test/phoenix-diagram/install.test.js#L24) 的两条回归通过 | 未在真实 Windows 上运行 |
| R1 / F-05 | [P3] 通过符号链接路径调用发布 guard 时，入口判断失败，guard 静默放行 | 6904c9a：入口判断改为比较 `fs.realpathSync(argv[1])` 与 `fileURLToPath(import.meta.url)` | **已解决** | 定位 [require-release-package.mjs:85](../../scripts/require-release-package.mjs#L85)。本轮经 scratchpad 符号链接调用：退出 1，拒绝 development 清单；用相对路径调用也退出 1。[publish.test.js:39](../../test/phoenix-diagram/publish.test.js#L39) 通过。导入方 `release-phoenix-ui.mjs` 的 argv[1] 是它自身，不会误触发 CLI | 无 |
| R1 / 待核实 1 | 真实宿主、图像审核和 23 AC 的证据在本机路径，无法复核 | M7 已从完成重开为进行中 | **不再适用** | 修复改变了 binary、bundle 和包内说明。M7 记录明确写明：旧 292386b、CI 39097200153 和 tgz 2fe5980d 只能证明旧候选。旧证据已降为历史，计划不再据此宣称通过 | 新包的 V-13–V-15 仍缺，转入下方计划符合性 |
| R1 / 待核实 2 | 联合发行合同建立前，任何 npm 发布都会被阻断（决策提醒，不计缺陷） | M7 记录：维持 §8/§11 已接受的阻断 | **仍存在**（决策项，不是缺陷） | 代码保持原设计，guard 行为未变 | 合并到 main 前仍需维护者确认 |
| R1 / 待核实 3 | F-02 需要维护者选定候选的版本策略 | 计划 D7 已改为“按 distribution 区分恢复指引”，并记录用户在 R1 修复中确认 | **已解决** | [计划 D7](PHOENIX-DIAGRAM-P1.md) 与 §5.1（[计划:303](PHOENIX-DIAGRAM-P1.md)）的写法与实现一致 | 无 |

## 确认问题（本轮新增）

### F-06 · [P3] 同步宿主 skill 中缺引擎时的恢复说明

- 定位：[phoenix-bootstrap.cjs:491–494](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L491)（target 1e27a5a，修复新增的非 release 指引）；过时的宿主说明在 [SKILL.md:67](../../skills/phoenix-diagram/SKILL.md#L67)。
- 问题：修复后，candidate/development 的 `setupCommand` 不再带固定包版本，而是本地入口加 `--release-dir 'MATCHING_RELEASE_DIR'` 占位符。计划 §5.1 要求接收者把占位符换成该候选配套完整资源的实际绝对路径。但已装 skill 中，宿主唯一能读到的说明 SKILL.md:67 仍是“缺引擎时给出错误回执中固定包版本的 `setupCommand`；只有用户已明确请求引擎安装才执行该命令”：既没提到占位符，也没要求先向用户取得配套目录。包内 [docs/phoenix-diagram.md:21](../../docs/phoenix-diagram.md#L21) 有说明，但这份文档不会随 skill 装入项目。宿主照 SKILL.md 原样执行时，命令以退出码 4 失败，stderr 又打印同一条命令，恢复无法完成。失败不写入任何文件，所以定为 P3。M7 重新取得 V-13/V-15 证据时，宿主读到的就是这份契约，应在同一变更中同步。
- 证据：在已装 `.claude/skills/phoenix-diagram` 的临时项目中删除 cache，再原样执行 setupCommand：退出 4，报 `ENOENT … <项目>/MATCHING_RELEASE_DIR/VERSION.json`，cache 目录没有创建。占位符的约定见计划 §5.1（[PHOENIX-DIAGRAM-P1.md:303](PHOENIX-DIAGRAM-P1.md)）。仓库规则 [AGENTS.md:103](../../AGENTS.md#L103) 要求：改变 skill 的 next-step 命令时，在同一变更中更新所有复述它的地方，判断标准是“each place still fully describes the new behavior”。
- 修复与验证：在 SKILL.md:67 按 distribution 分开写：release 直接执行回执给出的命令；candidate/development 先向用户取得配套完整 release-dir 的绝对路径，替换 `MATCHING_RELEASE_DIR` 后再执行，不猜测路径，也不改用 registry。SKILL.md 既属于 bundle，也是 146 项构建源码输入之一。修改后须按 R1 修复的做法重新生成 development 制品，再同步 bundle、`manifest.json` 和三份清单（bundleSha256 由 [build.test.js:205](../../test/phoenix-diagram/build.test.js#L205) 检查；sourceInputSha256 没有测试检查，须手工核对）。验证：跑 npm test；可加一条断言，检查 SKILL.md 提到 `MATCHING_RELEASE_DIR`。
- 置信度：高。说明文本与实测结果一致；实际影响取决于宿主能否自行推断占位符的含义。

## 待核实 / 待决策项

1. **联合发行阻断**（沿用 R1 待核实 2）：合并到 main 前需维护者确认。不属于代码缺陷。
2. **M7 新候选证据**：F-01、F-02 的真实效果要等新提交推送后的 CI 证明：普通 push 应以 success 结束，且 handoff 保持 pending-review；`await_review=true` 的 dispatch 应在原 job 内收口。此外，同一个新 tgz 下的四平台、Node 18.20.8、宿主与图像审核（V-13–V-15）都要重新取得。这些属计划验收，不是本报告的代码缺陷。
3. **本轮未重跑的验证**：完整 `test:diagram`、`test:phoenix`、真实 Windows。修复记录写明已运行 `test:phoenix`，本轮没有复核。

## 覆盖与验证

- 变更意图：修复 R1 的 F-01–F-05；按用户确认的 distribution 策略修正恢复指引。
- 仓根 `/Users/rockie/Documents/GitHub/xgent/skills`，日期 2026-10-18。输入为 `/review-code docs/plan/PHOENIX-DIAGRAM-P1.md`。开发计划模式下本计划已有 R1 代码评审报告，因此默认做修复效果复审。范围和指纹见「复审记录」。
- 已逐行读完全部修复差异：workflow、`bin/xgent-skills.js`、bootstrap（bundle 内副本与源文件逐字相同）、`cli.rs`、`install/plan.rs`、`require-release-package.mjs`、三份清单和 bundle manifest、`docs/phoenix-diagram.md`，以及全部新增测试。
- 上下游：
  - bootstrap 的 `manifestAt`、`verifyOffline`、`installEngine`、`preflight`、`main`、`help`，以及两个 launcher 传入的 options。
  - Rust install 请求的构造（`engine = current_exe`）。
  - xgent-skills 的 `prepareDiagram` 和顶层错误处理。
  - workflow 的触发路径，以及 consume、review、record、upload 各步骤。
  - guard 的导入方；SKILL.md；README 中的安装说明。
  - 全仓检索 `npm exec`：phoenix-diagram 范围内只剩两处 release 分支；候选 harness 不断言恢复命令的格式。
- 未覆盖：修复未触及的 layout、Reader、渲染、交付代码。R1 已把它们列为限制，本轮没有重审。
- 实际执行（都在 HEAD 1e27a5a，代码树与 6904c9a 相同）：
  - `npm test`：exit 0。Node 共 474 项，451 pass、23 skip、0 fail；Python 四组 22/24/2/13 全部 OK。修复新增的回归中，除两条 bootstrap 原生用例因需要显式 `PHOENIX_DIAGRAM_BIN` 而跳过（已另行补跑），其余都实际运行并通过。
  - `PHOENIX_DIAGRAM_BIN=<制品引擎> node --test test/phoenix-diagram/bootstrap.test.js`：20/20，0 skip。其中包括 candidate/development 空 cache 恢复，以及已装 launcher 丢失 cache 后的恢复。
  - `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test cli`：7/7。同一工具链加 `--test install install_dry_run_has_no_project_writes_and_uses_real_engine`：1/1。
  - `mbx fmt -- --check` 和 `mbx clippy --locked --offline --all-targets -- -D warnings`：均 exit 0。
  - 复现：F-02 的三条路径；F-03 的源码输入重算与制品引擎摘要；F-04 的两种模拟 win32 调用；F-05 的符号链接与相对路径调用；F-06 的原样执行。
  - `gh run list --workflow=phoenix-diagram.yml`：最新 run 仍是 292386b（39097200153），6904c9a 没有 run。`git ls-remote`：远端分支停在 7f3136c。
  - 外部资料：GitHub Actions 的 [workflow syntax（on.workflow_dispatch.inputs）](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax) 和 [contexts（inputs）](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts)。
- 证据限制：
  - 实测用的 development 制品 `local/phoenix-diagram/r1-recovery-development-2` 由修复者生成，位于 gitignored 目录，只能在本机取回；其引擎 SHA-256 与清单一致。
  - 临时项目和脚本都放在会话 scratchpad，没有写入仓库。
  - 由单个 agent 完成，没有使用并行 reviewer。

## 计划符合性

计划为 [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md)（HEAD 1e27a5a，SHA-256 见上），当前 6/7，M7 进行中。需求来源是 PRD P1 的 23 条 AC。下表只列与本轮修复相关的项。

| 需求 / 章节 | 本次任务 | 实现证据 | 验证状态 |
| --- | --- | --- | --- |
| WORK-F11 AC-5 / D7、§5.1：缺引擎时给出可执行、身份固定的恢复指引 | M4 doctor、M5 bootstrap | bootstrap:485–495、cli.rs:288–295、install/plan.rs:149–161 | 已核实（本地实测三条路径）；宿主说明缺口见 F-06；新 tgz 下的证据未取得 |
| §1.2：保留 UI 与 XGENT 现有行为 | M5 | xgent-skills.js:468–472 | 已核实（模拟 win32） |
| §5.5：离线目录完整性，candidate/release 必须含四平台 | M5 | bootstrap:185–192 | 已核实；development 单平台的放宽与计划 §5.5 修订一致，candidate 的反例测试保留并通过 |
| §5.6 / V-14：常规 CI 不等待人工审核，显式 dispatch 收口 | M6、M7 | workflow:7–11、497、512 | 代码已核实；真实 run 未取得（M7） |
| §8：发布保护 | M6 | require-release-package.mjs:85 | 已核实 |
| V-12：development 清单与 bundle 一致 | M6 | 三份清单、build.test.js:205–209 | 已核实 |
| V-13–V-15：同一新 tgz 的四平台、宿主、图像审核与 23 AC | M7 | — | 未核实；M7 进行中，属后续范围，不计本轮缺陷 |
| V-16：轻量回归、diagram 与 Phoenix runtime | M7 | — | 部分核实：npm test、受影响的 bootstrap/CLI/install 测试和 fmt/clippy 通过；完整 test:diagram 与 test:phoenix 本轮未重跑 |

## 总体结论与修复顺序

**验收判定：通过验收**

修复差异 `51c6379..1e27a5a` 为 `patch is correct`。修复结论为**已解决**：R1 的 F-01–F-05 五项都已实际解决；待核实 1 不再适用，待核实 3 已解决；待核实 2 是仍然有效的决策提醒，不计缺陷。本轮新增 F-06（P3），不阻断验收。

限制：本判定只覆盖修复差异和 R1 列出的问题，不代表 P1 整期验收。M7 仍在进行中，新候选的 V-13–V-15 和真实 CI 证据尚未取得；本结论也不构成合并或发布授权。

修复顺序：

1. 可选：修 F-06，更新 SKILL.md 的恢复说明，重新生成 development 制品，并同步 bundle 与三份清单。
2. 推送修复后完成 M7：取得普通 push 的 success 和 `await_review=true` dispatch 的收口证据，并在同一新 tgz 下重新取得 V-13–V-15。
3. 合并到 main 前，确认联合发行阻断的决策。
