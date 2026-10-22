# Phoenix Diagram P1 · R1 代码变更评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1（初审） | [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md) | 累计净差异：4a65a1525f9ed03c775a0e18935901e1edbb773f..51c637997c221fe99765159dce9e6a8571ff9b10；无未提交代码 | 无 |

计划版本：HEAD 5bb5668813e92c31aeff859e0f0d2fd8aa388536，计划文件 SHA-256 fcb6cad30485b01f215fe547f6d9a34e8b50a870a3cc7f03bcc670c0263ec103。完成记录登记的 18 个提交（98dfb34 … 51c6379）都能解析，且都在当前分支上；中间另有 19 个提交只改计划、记录或台账，属进度回写，不纳入审查。因此按连续区间审查 `98dfb34^1..51c6379` 的净差异。51c6379 只改记录，所以目标代码树与通过最终 CI 的 292386b 相同。同目录的 `.review.md` 是计划文档评审，不是前轮代码评审。

下列主定位均为 target 侧 51c6379。当前检出 HEAD 为 5bb5668，它在 51c6379 之上只改了计划和台账，代码文件与目标一致，工作树干净。

## 确认问题

### F-01 · [P2] 不要让每次 push/PR 都等待人工图像审核

- 定位：[.github/workflows/phoenix-diagram.yml:490–497](../../.github/workflows/phoenix-diagram.yml#L490)。触发路径见 7–27 行，其中 12–14 行包含 `package.json`、`LICENSE` 和 `bin/**`。
- 问题：四个平台的 consume job 自动消费通过后，会无条件进入“Resume the same delivery after actual image review”。这一步用只读 gh API 轮询 `phoenix-diagram-review/<runId>` 分支，最长 85 分钟；超时就抛错，以 1 退出。workflow 对 push 和 pull_request 都会触发，路径又包含共享的 package.json、LICENSE 和 bin/**。合并后，凡是升级包版本、改 XGENT 安装器或 UI 入口的提交/PR，只要没有维护者专门推送审核记录，四个 consume job 都会各占一台 runner（其中两台是 macOS）约 85 分钟，然后失败，workflow 一直是红的。根因是本次新增的 workflow 把人工交接门放进了自动触发路径。
- 证据：[check-phoenix-diagram-candidate.mjs:26–27](../../scripts/check-phoenix-diagram-candidate.mjs#L26) 定义 `WAIT_MS` 和 `POLL_BUDGET_MS`；270 行超时后抛错；300–301 行状态为 failed 时返回 1。实际运行：run 38988801251（9fea3fa）Linux x64 job 的第 12 步“Resume…”从 02:24:34Z 跑到 03:49:37Z，正好 85 分钟，结果 failure；run 38946210226（0c5829c）和 38977256489（7f3136c）也都在这一步失败。只有维护者推送了审核分支的 run（如 39097200153）才成功。
- 修复与验证：把审核续接改为显式触发，例如只在 workflow_dispatch 带输入（如 `await_review: true`）时运行该步骤；push/PR 只跑到 automated pass，并上传 review-request。也可以拆成单独的 dispatch workflow，同时收窄共享路径触发。验证：普通 push 在自动门通过后以 success 结束，handoff 仍标 pending-review，不能用来证明 P1 完成；带输入的 dispatch 仍能在原 job 内用 review-record 收口。
- 置信度：高。代码路径与 3 次实际 run 的步骤结论、耗时一致。

### F-02 · [P2] 修正缺引擎恢复指引所指向的包版本

- 定位：[phoenix-bootstrap.cjs:485–489](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L485)（`setupCommand`）；[cli.rs:289](../../tools/phoenix-diagram/src/cli.rs#L289)（doctor 的 `recovery.engineInstall`）。
- 问题：两处指引都用 npmPackageVersion 生成 `npm exec --package=@xgent-ai/skills@0.7.4 -- phoenix-diagram engine install ...`。根 package.json 仍是 0.7.4，而 registry 上的 @xgent-ai/skills@0.7.4 已于 2026-10-15T13:39:21Z 发布，早于调查基线 4a65a15，其中的 bin 只有 skills、xgent-skills、phoenix-ui。候选使用者在引擎缓存丢失、换机器或换 `PHOENIX_DIAGRAM_HOME` 后，执行 [SKILL.md:67](../../skills/phoenix-diagram/SKILL.md#L67) 要求宿主转述的命令，npm 会取回另一个同名同版本的包，并报 `command not found: phoenix-diagram`，无法恢复。这违背了 PRD WORK-F11 AC-5“给出固定包版本的恢复指引”的目的。候选 tgz 与已发布包同名同版本，“固定包版本”也就不再唯一标识一份产物。
- 证据：`npm view @xgent-ai/skills@0.7.4 bin` 中没有 phoenix-diagram；`npm view @xgent-ai/skills time` 显示 0.7.4 的发布时间为 2026-10-15T13:39:21.505Z。在空目录实际执行 `npm exec --yes --package=@xgent-ai/skills@0.7.4 -- phoenix-diagram engine install --project .`，结果为 `command not found: phoenix-diagram`。本仓 `node bin/phoenix-diagram.js doctor --json` 返回的 `setupCommand` 就是这条命令。需求见 [PRD WORK-F11 AC-5](../PRD-PHOENIX-DIAGRAM.md#work-f11-按-phoenix-方式安装迁移与恢复)，版本来源见计划 §0.2 D7 和 [dist/phoenix-diagram/VERSION.json:6](../../dist/phoenix-diagram/VERSION.json#L6)。
- 修复与验证：两种方向，由维护者选择：①候选改用未在 registry 发布的独立包版本（如预发布号），再重新装配；②在 distribution 不是 release 时，给出候选可用的指引（同一 tgz 的 `npm exec --package=<tgz>` 加 `--release-dir`），只有 release 才输出 registry 版本。两者都会改变最终 tgz 身份，按计划须重新取得受影响的 V-14/V-15 证据。验证：对 candidate/development 清单断言恢复命令不引用 registry 上已存在、且不含 phoenix-diagram bin 的版本；空 cache 下按指引实际恢复成功。
- 置信度：高。registry 元数据与实际执行结果一致。

### F-03 · [P3] 同步仓内 development 清单的 bundleSha256

- 定位：[dist/phoenix-diagram/VERSION.json:12](../../dist/phoenix-diagram/VERSION.json#L12)；[skills/phoenix-diagram/ENGINE.json:12](../../skills/phoenix-diagram/ENGINE.json#L12) 和 [dist/phoenix-diagram/bundle/ENGINE.json:12](../../dist/phoenix-diagram/bundle/ENGINE.json#L12) 是同值副本；漂移测试见 [build.test.js:196–205](../../test/phoenix-diagram/build.test.js#L196)。
- 问题：三份清单的 bundleSha256 仍是 98dfb34 时的 `b55f3aa6…`，而 bundle/manifest.json 从 dbbd728 起已多次更新，当前 SHA-256 为 `15aaeacd…`。在检出中运行 `node bin/phoenix-diagram.js` 的任一普通命令，或默认的 `node bin/xgent-skills.js install`，verifyBundle 都会直接报 “Bundle manifest identity or digest differs from VERSION”，而不是给出缺少 development 引擎的预期指引。dbbd728 新增的漂移测试只比较 bundle 文件和 manifest，不检查 VERSION/ENGINE 中的 bundleSha256，所以 npm test 发现不了。候选 pack 会覆写这三份文件，因此不影响已验收的 tgz。
- 证据：本地 `verifyBundle('dist/phoenix-diagram/bundle', manifestAt('dist/phoenix-diagram/VERSION.json'))` 和 `doctor --json` 都返回上述错误；`shasum -a 256 dist/phoenix-diagram/bundle/manifest.json` 为 15aaeacdec500120908a4fe6acd923aa3dfd77270c49ac402515c885609f3534。覆写清单见 [pack-phoenix-diagram-candidate.mjs:20](../../scripts/pack-phoenix-diagram-candidate.mjs#L20)。
- 修复与验证：同步 bundle 时一并更新三份 development 清单的 bundleSha256，或者不再提交这类会过期的字段。在 build.test.js 中增加断言：`sha256(manifest.json) === VERSION.bundleSha256`，且三份清单逐字相同。
- 置信度：高。

### F-04 · [P3] 不支持的平台上，默认 diagram 分支不应阻断 `--no-phoenix-ui` 安装

- 定位：[bin/xgent-skills.js:462](../../bin/xgent-skills.js#L462)（默认启用）、[508](../../bin/xgent-skills.js#L508)（预检调用）；失败出口见 107–128 行的 `prepareDiagram`。
- 问题：diagram 默认启用，预检在 `platformTarget()` 遇到不支持的平台时会把整个安装判为失败。以前在 Windows 上执行 `xgent-skills install --no-phoenix-ui` 会安装 XGENT 的 hooks/settings 并退出 0；现在同一命令退出 4、不写入任何文件，必须再加 `--no-phoenix-diagram`。错误提示还建议使用 `--diagram-release-dir`，但这在该平台上无效。这不符合计划 §1.2 对 bin/xgent-skills.js 的要求“保留 UI 与 XGENT 现有行为”。已发布 0.7.4 的 UI 清单同样只有四个 Unix 平台，所以启用 UI 的默认命令在 Windows 上本来就会失败；本条只影响跳过 UI 的路径。
- 证据：在临时项目中以 `process.platform = 'win32'` 运行 `install <dir> --no-phoenix-ui --no-xgent-init`，输出“Phoenix Diagram 未就绪,未修改项目: Unsupported Phoenix Diagram platform win32/arm64…”，exit 4，目录中没有文件；加上 `--no-phoenix-diagram` 后写入 .claude/hooks 和 settings，exit 0。用 `git archive 4a65a15` 取出的基线安装器，在同样的模拟条件下执行同一命令，写入 hooks/settings，exit 0。平台判断见 [phoenix-bootstrap.cjs:33–36](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L33)。
- 修复与验证：在未显式要求 diagram 时，对不支持的平台跳过 diagram 并打印说明；显式传了 `--diagram-release-dir` 等参数时仍然失败。具体取舍由维护者确认。回归测试：模拟 win32 时，`--no-phoenix-ui` 成功，且只写 XGENT 文件。
- 置信度：中高。在模拟平台上复现，未在真实 Windows 上运行。

### F-05 · [P3] 发布保护的入口判断不能静默放行

- 定位：[scripts/require-release-package.mjs:85](../../scripts/require-release-package.mjs#L85)。
- 问题：旧 guard 在模块顶层无条件执行；本次为了导出函数，改为仅当 `pathToFileURL(path.resolve(argv[1])).href === import.meta.url` 时才运行。ESM 主模块的 import.meta.url 是解析符号链接后的真实路径，argv[1] 却保留调用时的路径。用经过符号链接的绝对路径调用时（如 macOS 上经 /tmp 访问的检出，或软链接工作区），比较失败，脚本不做任何检查就退出 0。npm 的 prepublishOnly 从物理 cwd 用相对路径调用，不受影响；但维护者或其他脚本直接调用 guard 做预检时，会得到错误的“可发布”结果。
- 证据：在 scratchpad 建立指向仓库的符号链接后，`node <link>/scripts/require-release-package.mjs` 无输出、exit 0；用真实路径调用，或 `cd <link> && node scripts/require-release-package.mjs`，都 exit 1 并拒绝；基线版本的 guard 对同一链接路径也 exit 1。同一判断写法在仓内其他脚本中早已存在，本条只针对从无条件执行改为条件执行的发布保护。
- 修复与验证：改为比较 `fs.realpathSync(process.argv[1])` 与 `fileURLToPath(import.meta.url)`，或把导出函数移到单独模块，让 CLI 文件继续无条件执行。回归测试：用符号链接路径调用 guard，仍应拒绝 development 清单。
- 置信度：高（已复现）。影响仅限直接调用。

## 待核实 / 待决策项

1. **真实宿主、图像审核和 23 AC 的证据（V-9、V-13、V-15）**：M7 记录中的会话、截图和聚合回执大多位于本机的 `local/phoenix-diagram/...` 和 `/tmp/...`，仓内无法取回，本轮也未复核。本轮只核实了 CI 39097200153 的 job 结论。补证方式：维护者提供可取回的归档，或在该 run 的 artifacts 中定位对应文件。
2. **合并后的发布阻断（决策提醒，不计缺陷）**：包中带上 diagram 后，prepublishOnly 和 `release:phoenix` 都会拒绝发布。在联合发行合同建立前，@xgent-ai/skills 的任何 npm 发布都会被阻止，包括与 diagram 无关的 skills/hooks 更新。计划 §8、§11 已接受这个代价，合并到 main 前请确认。
3. **F-02 的版本身份**：计划 D7 规定 npmPackageVersion 直接读取根 package.json，没有考虑这个版本已在 registry 上发布。修复需要维护者选定候选的版本策略。

## 覆盖与验证

- 变更意图：在本仓新增独立的五类 Rust 图解工具、离线 Reader、npm 安装与候选装配、发布保护，并接入共同安装器，作为 P1 候选。
- 仓根 `/Users/rockie/Documents/GitHub/xgent/skills`，评审日期 2026-10-18。输入为 `docs/plan/PHOENIX-DIAGRAM-P1.md`，按开发计划模式确定范围（见“评审对象”）。工作树干净，没有 untracked 代码。
- 规模：594 个文件，+941022/−32 行。排除计划、台账和 PRD 后有 572 个文件，其中 tools/phoenix-diagram 366 个（fixtures 131 个，约 76 万行；upstream 冻结源 70 个）。
- 深读范围：
  - 共同 npm 入口 `bin/xgent-skills.js`、package.json、README 改动，发布 guard 及其在 release-phoenix-ui 中的接入，原有测试的 skip 适配。
  - bootstrap 与 launcher、`cli.rs`、`input.rs`、`output.rs` 与 `output/unix.rs`（均全读）；`output/transaction.rs` 的 prepare、publish、rollback、pending 部分。
  - `render.rs` 的转义、METADATA 和模板替换；install 的 provider 解析与 publish/retire；`browser/cdp.rs` 的进程和 profile 生命周期；`delivery/review.rs` 前段。
  - CI workflow（全读）；`check-phoenix-diagram-candidate.mjs` 的审核轮询、pack 的覆写与一致性检查、prepare 的 npm pin。
  - Reader JS：检索 HTML sink（无 innerHTML、eval 等）、外部资源（只有 SVG 命名空间和许可 URL）、存储键（`phoenix-diagram-motion`）。
  - 最后一个验收期修复 292386b 的 geometry、check、routing 和 measure 差异，并确认 200px 上限只对 phoenix-v1 放开。
- 未逐行审：layout/*（约 1.9 万行）；geometry 的 composition、route_quality、labels、text；check 的 anchors 和 path；delivery 其余部分；install 的 disk、state、bundle；output 的 recover 路径；browser 的 evidence、observation、probe.js；viewer 各模块；tests 与 fixtures；upstream 冻结源；aggregate、assemble 和 test-* 脚本。这些部分依赖现有测试与 oracle，本轮不声称已逐行验证。
- 实际执行：
  - `npm test`（HEAD 5bb5668，代码树等于目标）：exit 0。Node 共 469 项：447 pass、22 skip、0 fail；跳过的 22 项需要显式的 `PHOENIX_UI_BIN`/`PHOENIX_DIAGRAM_BIN`。Python 四组 22/24/2/13 全部 OK。
  - `node scripts/test-phoenix-diagram-native.mjs`（即 `npm run test:diagram`，同一 HEAD）：exit 0，回执为 `status: pass`，并注明 realBrowser 和 providerSessions 均为 not-run。具体结果：Reader 装配检查通过；fmt 检查和 all-targets `clippy -D warnings` 通过；Rust 21 组共 266 pass、0 fail、2 ignored（真实 Chrome 专案）；test/phoenix-diagram 共 197 项，192 pass、5 skip；upstream 与 viewer 共 70/70。跳过的 5 项是 UI 与 diagram 共装用例，需要显式 `PHOENIX_UI_BIN`，本轮未提供，因此共同安装器的真实共装路径没有在本地复核。
  - CI：`gh run view 39097200153` 显示 headSha 292386bf…、event push、10 个 job 全部 success。另外查询了 3 次失败 run 的步骤结论和耗时（F-01）。
  - 复现：F-02 的 `npm exec`；F-03 的 verifyBundle/doctor；F-04 的模拟 win32 安装及基线对照；F-05 的符号链接调用，以及基线 guard 的对照。临时文件都在会话 scratchpad，已清理。
  - 未运行：`npm run test:phoenix`、真实浏览器与导出、四平台原生、Claude Code/Codex 会话。
- 由单个 agent 完成，未使用并行 reviewer。

## 计划符合性

计划为 [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md)（版本见上），需求来源是 PRD P1 的 23 条 AC。M1–M7 在完成记录中都标为已完成。下表只列与本轮发现或可核实证据相关的项。

| 需求 / 章节 | 本次任务 | 实现证据 | 验证状态 |
| --- | --- | --- | --- |
| WORK-F11 AC-5：缺引擎时给出固定包版本的恢复指引 | M4 doctor、M5 bootstrap | phoenix-bootstrap.cjs:485–489、cli.rs:289 | 存在缺陷（F-02） |
| §1.2：bin/xgent-skills.js 保留 UI 与 XGENT 现有行为 | M5 | xgent-skills.js:107–128、462、508 | 存在缺陷（F-04，仅限不支持的平台） |
| §8：prepublishOnly 和 release:phoenix 认识 diagram | M6 | require-release-package.mjs、release-phoenix-ui.mjs:142 | 已核实，npm test 中 publish/release 用例通过；入口判断问题见 F-05 |
| §5.6 / V-14：同一 tgz 四平台消费 | M6、M7 | workflow、pack 与 check 脚本 | 已核实 CI 39097200153 全部 success；常规触发下的审核等待见 F-01 |
| §5.1 / WORK-N5：序列化安全、离线 Reader | M2–M4 | `escape_text`、METADATA 转义、variant 白名单；Reader 无 HTML sink、无远程资源 | 静态已核实 |
| §3.1 / V-2–V-5：双输入合同与五类布局 | M1–M3 | input.rs 的合同和质量冲突、legacy 标识剥离、延期字段；layout/* | input 静态已核对；布局依赖 test:diagram（Rust 266/0 fail）验证 |
| §5.2 / V-6：输出事务与恢复 | M4 | output.rs、unix.rs、transaction.rs | 发布与回滚静态核对；recover 由 test:diagram 的 output/delivery 组覆盖并通过 |
| V-9、V-13、V-15：真实宿主、图像审核、23 AC | M4、M7 | M7 记录 | 未核实（证据在本机路径） |
| V-16：轻量、完整 diagram、Phoenix runtime | M7 | — | 部分核实：npm test、test:diagram 通过（5 项 UI 共装用例因缺 `PHOENIX_UI_BIN` 跳过）；test:phoenix 未运行 |
| P2/P3 的新增五类、迁移、公开发行 | — | — | 属后续范围，不计缺陷 |

## 总体结论与修复顺序

**验收判定：需要修改**

`patch is incorrect`。理由：F-02 已确认，缺引擎时给出的恢复命令无法执行，未满足 WORK-F11 AC-5 的目的；F-01 会让合并后的常规 CI 持续失败并长时间占用 runner。F-03、F-04、F-05 是低优先级问题，不单独阻断。覆盖限制：大量布局、Reader 移植代码依赖测试，未逐行审；真实宿主与图像审核证据未复核。

修复顺序：

1. 先修 F-02，确定候选版本身份，或按 distribution 区分恢复指引。这会改变最终 tgz 身份，按计划须重新装配，并重新取得受影响的 V-14/V-15 证据。
2. 修 F-01：调整 workflow 的审核门触发方式，避免常规 push/PR 必然失败。
3. 再处理 F-03、F-04、F-05，补上对应的回归断言。

解除阻塞的条件：恢复指引在空 cache 下能按原文执行成功；普通 push 的 workflow 能以 success 结束，且不冒充 P1 handoff；修复后的复审确认没有新的回归。本结论只覆盖所审提交，不构成合并或发布授权。
