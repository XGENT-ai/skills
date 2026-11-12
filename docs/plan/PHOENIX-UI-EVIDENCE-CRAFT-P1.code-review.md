# PHOENIX-UI-EVIDENCE-CRAFT-P1 · R1 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 · 初审 | [Phoenix UI 多状态取证与盲测走查 P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md) | 下表登记提交各自相对第一父提交的 diff；核对当前 `c76838ad46249963aca2d2f6a3642c6d8fb1cab9` 中的累计行为；本计划实现无未提交差异 | 无；同名 `.review.md` 是计划文档评审，不是代码评审 |

本轮意图：核对通用 capture、多状态与派生证据、完工走查及三个独立 skill 的可选接入，是否兑现 P1 已完成里程碑的契约。用户本轮只要求评审；功能与验收要求来自计划和其明确引用的 PRD，不另增需求。

## 1. 确认问题

### F-01 · [P1] 阻止新注册的 Service Worker 绕过写拦截

- 定位：[cdp.rs:1100–1105](../../tools/phoenix-ui/crates/browser/src/cdp.rs#L1100)，当前目标侧，`c6ce151` 新增。
- 问题：页面注册一个在 install 事件中执行 POST 的 Service Worker 时，`--block-writes` 仍允许该请求到达服务器。这里仅为 iframe 和普通 worker 登记 Fetch 拦截，页面上的 `Network.setBypassServiceWorker` 也没有阻止新 Service Worker 自身发请求。实测 capture 退出 0、主图有效、`blockedWrites` 为空，服务器却收到两次 POST（初始截图页和独立扫描页各一次）。feature-demo 在共享实例上依赖此开关做预检时，会发生未授权写入，违背 WORK-F1/AC-8 与计划 §5.7、§8 的安全边界。
- 证据：本轮 E-1，见 §3.3；主页面注册 `sw.js`，其中 `install` 的 `waitUntil` 执行 `fetch('/api/sw', {method:'POST'})`。没有配置任何 allow 路径。普通嵌套 worker 的对照请求全部被拦截，故不将普通 worker 泛化为失败。
- 修复与验证：在写拦截启用时，覆盖新 Service Worker 的写通道，或阻止其启动；不能把“绕过页面的 Service Worker 响应”当作“禁止 Service Worker 写入”。扩展现有真实浏览器写通道场景，以服务器实际收到的请求和报告记录共同验证。
- 置信度：高，真实 Chrome 与合成 HTTP 服务端观测到写入。

### F-02 · [P2] 在静态服务器读取文件前检查符号链接的真实边界

- 定位：[serve.rs:178–179](../../tools/phoenix-ui/crates/cli/src/capture/serve.rs#L178)，当前目标侧，`c6ce151` 新增。
- 问题：本地目标目录里存在指向目录外的符号链接时，`resolve` 只检查请求路径的字面段，随后 `std::fs::read` 跟随链接读取目录外文件。实测 `app/public.txt -> ../private.txt` 后，目标页面对 `/public.txt` 的 fetch 成功读到外部合成标记，并将它写入 capture 的控制台记录。用户打开一个含这类链接的本地项目时，页面脚本便能读取本未打算经静态根目录公开的文件；只绑定回环和检查 Host 不会限制同源页面的读取，违背该服务器声明的目录边界。
- 证据：本轮 E-2，输出 `consoleErrors: ["OUTSIDE_ROOT=synthetic-private-marker"]`，退出 0；[serve.rs:94–100](../../tools/phoenix-ui/crates/cli/src/capture/serve.rs#L94) 返回拼接路径，未检查解析链接后的路径。计划 §5.9、§7 规定静态根与路径边界。
- 修复与验证：在文件读取边界核对 canonical 根和最终文件路径，拒绝越出根目录的链接；保留根目录内正常资源访问。用合成文件检查文件链接和目录链接，不访问真实私密文件。
- 置信度：高，真实 CLI 复现；输入控制条件是本地目标树中有越界链接。

### F-03 · [P2] 将派生图名称纳入输出重名检查

- 定位：[mod.rs:688–692](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L688)，当前目标侧，`c6ce151` 新增的命名检查；覆盖行为由 `9203484` 的派生输出激活。
- 问题：`--name 'sheet-{viewport}.png' --sheet` 是当前接受的参数，但重名检查仅比较主图之间的名称。后续并排图也写到 `sheet-<viewport>.png`，在同一 `files` 队列里覆盖主图。实测 400×300 视口运行退出 0，报告主图 `valid:true`、`pixels:400×300`，该路径实际却是 272×234、带 initial 标签的派生图；调用方会把已被替换的图当作通过有效性检查的主证据。这违反 WORK-F1/AC-9、WORK-F2 的“派生图不替代主截图”与计划 §5.1 的重名检查承诺。
- 证据：本轮 E-3，读取 PNG IHDR 并亲自开图确认；[mod.rs:1050–1052](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L1050) 追加并排图，[mod.rs:853–855](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L853) 按队列顺序写入同一路径。
- 修复与验证：把本次主图、无效图及请求的派生图输出放入同一名称冲突检查，冲突时在写入前明确报错。保留自定义文件名接口；用上述真实 CLI 参数验证不会再覆盖主图并返回成功。
- 置信度：高，文件尺寸、图像内容与报告三方直接矛盾。

### F-04 · [P2] 处理不足 4 像素的元素标注区域

- 定位：[derived.rs:301–304](../../tools/phoenix-ui/crates/cli/src/capture/derived.rs#L301)，当前目标侧，`9203484` 新增。
- 问题：元素截图任一边小于 4 像素且声明了可定位的 mark 时，`clamp(0.0, size - 4.0)` 的上限小于下限，会直接 panic。实测对 2×100 CSS 像素的渐变分隔线取元素图并标注，debug 二进制退出 101，报 `min = 0.0, max = -2.0`，不写主图和报告；普通有效输入因此使整次取证丢失，未走承诺的 0/1/2 结果。发行配置为 `panic = "abort"`，不能依赖发行版回收这一失败。
- 证据：本轮 E-4；元素 `#tiny` 为 `width:2px;height:100px`，状态文件见 §3.3。计划 §5.2 未禁止这类元素尺寸，§5.5 允许元素图，§5.10 要求外部输入不触发 panic；发行配置见 [Cargo.toml:33–38](../../tools/phoenix-ui/Cargo.toml#L33)。
- 修复与验证：按实际图幅计算合法的标注范围；无法绘制时返回可报告的派生失败，保住主图和报告。用同一 CLI 场景验证窄分隔线和薄横线不会终止进程。
- 置信度：高，debug 实测 panic；发行版 abort 后果来自已读配置，未另行运行崩溃的发行进程。

### F-05 · [P2] 为沿用截图保留原采集工具身份

- 定位：[mod.rs:862–865](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L862)，当前目标侧，`c6ce151` 新增。
- 问题：修复轮换用另一份引擎后通过 `--only` 部分重取，旧图及其条目原样保留，但整份报告唯一的 `tool.version/binarySha256` 被替换为新引擎，旧图没有自己的工具身份。实测先用已有发行候选采集 initial 和 again，再用本轮 debug 引擎只重取 again：initial 的文件和条目均未变化，原 `7b64a66d178c…` 工具摘要却从报告消失，唯一摘要变成 `e4fe7a8caa10…`。review-ui 按新合同从报告抄录出处时，会把沿用图错误归到新引擎，无法再核对它究竟由哪份二进制生成。
- 证据：本轮 E-5；[report.rs:51–56](../../tools/phoenix-ui/crates/cli/src/capture/report.rs#L51) 保留未重取条目，组装新顶层时不保留旧工具信息。WORK-F1/AC-9 要求逐张命令版本，计划 §5.8 允许保留旧图；[review-ui 出处合同:128](../../skills/review-ui/references/target-and-capture.md#L128) 明确抄录工具版本及二进制摘要。两份实测二进制版本号都为 0.1.0，更说明只靠版本号无法替代被丢弃的摘要。
- 修复与验证：为每张图保留其生成时的工具身份，或保留可关联到每张图的运行记录；部分重取只更新本次条目。用两份不同摘要的引擎连续采集后，核对两类图片都能解析出各自的原始出处，并同步报告消费者的读取约定。
- 置信度：高，使用两份真实二进制观察到旧图工具身份丢失。

## 2. 待核实与覆盖限制

- V-6、V-9：已读 [M5](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M5.md#验收)、[M6](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md#验收) 的真实宿主演练记录；本轮未取得其中 Claude/Codex 会话的原始输出和走查产物，不能独立确认 A1–A6 全部细节，也未重新运行收费宿主会话。修后复审应提供或重新生成与最终候选匹配的交接、走查及交付材料，特别是保存反馈问题进入修复批次后的复走证据。
- V-8：本轮未重新查询远程 CI。M6 记载最终候选 `d5904e146588766dda1b425bc6f927a0b861984f`、run `42753553795` 四平台与 Node18 通过；已在本地确认它与本轮 HEAD 的 Phoenix UI 源码、相关 skill、冒烟脚本无差异。这只核对了版本适用性，不冒充重新检查 CI 日志与制品。
- 生成物通过当前版本 render freshness 检查；未逐字节人工审阅所有 provider blob、许可清单和约 2 MB 的 live 生成脚本。live 生成脚本的输入/输出记录增量已检查，不据此宣称重新验收 live；未登记的 hook/安装与 manifest 重构按用户要求排除。
- 全期可用性已被 F-01–F-05 阻断；以上未独立复验项不被测试通过数或里程碑自述替代，也没有被另报成代码缺陷。

## 3. 覆盖与验证

### 3.1 版本、范围与快照

- 仓库：`/Users/rockie/Documents/GitHub/xgent/skills`；日期：2026-11-10（Asia/Shanghai）；分支 main。
- 原输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md`。没有本计划的旧代码报告，本轮为初审，不复审计划文档报告中的 F-ID。
- 冻结 HEAD：`c76838ad46249963aca2d2f6a3642c6d8fb1cab9`。因登记提交间夹有其他工作，各登记提交按 `git diff --no-ext-diff --no-textconv <第一父SHA> <登记SHA> --` 分别归因；当前树用于核对最终净行为，不将中途已修复问题当作仍存缺陷。

| 登记提交（完整 SHA） | 第一父提交（比较基线） | 本轮处理 |
| --- | --- | --- |
| `c6ce1518f6b33c52a89f4619e0935a82915adbf7` | `4c2d1117f83b7af3292a846aa71a14fe4df8d990` | M1–M2 |
| `920348421f4955f4386a4578adac9a83cd381523` | `b5faa8f1185c713c62555750f03408ac1cfbd785` | M3 |
| `a3177e5a88678a13c8f590217b1cdcb636f1b55d` | `f7e98443fbe74f3d9c29ae509d5b5c0109cf2ec8` | M4–M5 |
| `226c8c2c6c7929303c87a992df9dbcde9e44ff3e` | `306d90fba089b7f3ec8ac74bd2ea47d661aa27b9` | M6 审计 pin 与锁文件 |
| `86d8576e5052c26bc17bccafd44e66474611067d` | `226c8c2c6c7929303c87a992df9dbcde9e44ff3e` | M6 构建、许可、生成物、oracle、冒烟 |
| `21d12ea40bcb1cfed3e2ab4b62a0ee07fe91471d` | `9e27d9c990243e6bc9ad42cad4121ed4707d8dac` | 未取证与未走查原词及生成物 |
| `ce45063cd84ff09994785e1ac5125d845e12630d` | `a92c97f8d0ea8ba725942d389274d5a1f40b0dd4` | 仅登记的 hook 测试格式差异 |
| `2a4b99e376a574619f25ec872c2d79e62dccb851` | `762da0ac31c7bf13dab40c44d876d4f5426ae0a7` | 日期输入、走查交接和降级措辞 |
| `55aa061e997d125c240c176b9d4ba75c879437db` | `2a4b99e376a574619f25ec872c2d79e62dccb851` | 交付位置的处置词与走查要求 |
| `00878b4329252a9fecb009902e9bee8e09cf2181` | `55aa061e997d125c240c176b9d4ba75c879437db` | 仅计划与记录，契约材料，不作实现 finding 对象 |

全部登记提交已解析并确认是当前 HEAD 的祖先。排除：进度回写 `b5faa8f`、`f7e9844`、`306d90f`、`762da0a`、`c76838a`；用户明确只审登记提交，因此另排除未登记的 `59d87db`、`5d2ca87`、`daac434`、`9e27d9c`、`b157be7`、`a92c97f`。其中合并带入的代码可作当前上下文，不能据其独立改动产生本轮 finding。

工作树/index：开评时无 staged 差异；tracked dirty 仅五份 `.xgent-ai/` 索引。未跟踪的 PRD、决定记录、文档评审报告和状态资料仅作相关契约/记录上下文；`skills/software-copyright-docs/` 经用户明确排除。本计划没有未提交实现变更。计划 SHA-256：`593983d7e2697b031f9c72e23d8eac6b9d2c73361837e2b2f9fc67a234e0489e`；未提交 PRD：`09b19f65376834f46480a3dec8394e8ab1a18ba2707f0399da0e9be0caad1e25`；决定记录：`4cc32092cb792cc33e95be0215cbbf951a9a4c5b4a2849e44cec7f5e4cd9ddc0`。未提交材料仍仅原工作树可取回。

开评对登记 diff 涉及文件及当时 untracked 文件做内容快照指纹（排序的 `路径→SHA-256或不存在` JSON，整体 SHA-256 `a12c10581836991b31e5c8301694e3c8212e451595d1d13803061c94fe9184db`）；写报告前逐项对比无变化。该初始集合包含后经用户排除的文件，指纹不表示它们已经受审。实现的可取回版本由上述提交及 HEAD 定位；本报告和随后台账写入均不作为本轮输入。

### 3.2 文件与行为覆盖清单

| 文件/直接依赖 | 已审行为与检查 | 限制 |
| --- | --- | --- |
| `crates/cli/src/capture/{mod,plan,steps,page_js,report,serve,validity,derived}.rs` | 完整新增源码；参数/JSON 边界、命名、独立状态、输入、动画稳定、三类截图、无效/失败清理、脱敏、合并、派生图与 stitch；E-1–E-5 | 未穷举所有外部输入 |
| `crates/browser/src/{cdp,lib,network_log}.rs` | 完整相关 diff，导航/请求拦截与子会话、profile、录帧、网络记录、detect 共用扫描；真实 worker/SW 对照 | 未重跑共享浏览器层全部测试 |
| `crates/detect/src/{cli,lib}.rs`，`crates/cli/src/{main,lib}.rs` | 路由、帮助、扫描选项与过滤调用链 | 初始 findings 对 detect 的全场景等价沿用 M2 记录，未重新全跑 |
| `crates/cli/tests/capture.rs`，`tests/fixtures/capture/**`，`tests/oracle/cases/capture.mjs` 与 capture/cli-help golden | 读取新增测试与夹具；当前真实写通道用例、capture oracle | 既有用例未覆盖本轮确认的边界，不能证明这些边界正确 |
| Phoenix `src/reference/new-work.md`、`src/subagents/{phoenix-ui-finish-reviewer,phoenix-ui-ux-walker}.md`、`evals/scenarios.md` | capture 默认取证、截图类型、两轮自检、修复沿用、同轮盲测、交接、隔离标识、未运行语义、四处置词 | 主要为指令一致性；宿主表现限制见 §2 |
| `skills/feature-demo/SKILL.md`；`skills/review-ui/{SKILL.md,references/{target-and-capture,report-contract,judge}.md,evals/{scenarios,fresh-context}.md}`；`skills/apply-ui-review/{SKILL.md,references/record-contract.md,evals/scenarios.md}` | 可选发现、不可用不安装、不适用单元回原方式、出处、评审材料隔离、走查差距与复走合同 | 未派出额外 reviewer；本轮全部审查由当前 agent 完成 |
| `README.md`、`docs/phoenix-ui.md`；`scripts/render.mjs` 的登记计数改动；`skills/phoenix-ui` 生成物与 `dist/phoenix-ui/bundle/**` | 接口复述、五个子代理、19 provider 的 freshness 和源/生成一致性 | 未登记的生成器重构只作直接依赖上下文 |
| `Cargo.lock`、`scripts/check-phoenix-deps.mjs`、NOTICE/THIRD-PARTY、`scripts/smoke-phoenix-ui.mjs`、live 生成输入记录、`hook_tests.rs` 格式 hunk | 锁文件/pin/署名增量、capture 冒烟路径及不相关检查边界 | 不重新审计所有第三方库，不扩大到安装器、live 或 hook 重构 |

### 3.3 本轮实际执行

执行前的对应关系：写拦截承诺→真实 HTTP 服务端接收记录→E-1 与现有单个写通道用例；目录边界→目标页同源读取→E-2；证据文件完整性→CLI 输出及 PNG→E-3/E-4；合并出处→两份真实二进制的部分重取→E-5；技能与生成物→既有 render freshness 和仅四个技能的元数据检查。未运行聚合 `npm test` 或全量 `test:phoenix`。

环境为本机 macOS arm64、可发现的 Chrome，Rust 1.99.1、Node 25.2.1、Bun 1.4.4（均经 mise）。先在 `tools/phoenix-ui` 执行 `mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline`，成功；当前 debug 二进制 SHA-256 `e4fe7a8caa1028aca65fce07bb59c03ba1f0b86760f9913eee7174325ce389ab`。之后所有 E-1–E-4 都从该真实 CLI 入口运行，非模拟内部函数。

| 检查 | 实际步骤/最小调用 | 结果 |
| --- | --- | --- |
| E-1 · SW 写入 | 合成 `sw.html` 注册 `sw.js`；后者 `self.addEventListener('install',e=>e.waitUntil(fetch('/api/sw',{method:'POST'})))`。Python `ThreadingHTTPServer` 服务夹具并记录 POST；`capture <http入口>/sw.html --viewport 400x300 --block-writes --out <tmp>/sw` | 退出 0；服务端两次 `/api/sw`；报告 `blockedWrites:[]`，主图有效。**复现 F-01** |
| E-1 对照 | 普通 parent worker 创建 child worker；两者各 POST；同样开启 block | 服务端无写入；报告包含 parent 与 nested 两条拦截。该候选不成立 |
| E-2 · 目录边界 | 只在临时目录创建合成 `private.txt`，`app/public.txt` 为其链接；页面 fetch `/public.txt` 后 `console.error('OUTSIDE_ROOT='+text)`；`capture <tmp>/app/link.html --viewport 400x300 --out <tmp>/symlink` | 退出 0；控制台记录目录外合成标记。**复现 F-02** |
| E-3 · 输出冲突 | `capture <合成页面> --viewport 400x300 --name 'sheet-{viewport}.png' --sheet --out <tmp>/name-collision` | 退出 0；报告主图 400×300、有效；PNG IHDR 实际 272×234；亲自开图看到带 initial 标签的并排图。**复现 F-03** |
| E-4 · 窄元素 | 页面有 `#tiny{width:2px;height:100px;background:linear-gradient(red,blue)}`；状态 `{"initial":{"type":{"element":{"css":"#tiny"}},"mark":[{"css":"#tiny"}]}}`；`capture <页面> --viewport 400x300 --states <文件> --out <tmp>/tiny` | 退出 101；`clamp` panic；无报告。**复现 F-04** |
| E-5 · 工具身份 | 用已有 release 二进制采 initial/again（again 仅 wait 0），再用当前 debug 二进制对相同输出 `--only again` | 两次退出 0；initial 条目及 PNG 不变；旧工具 SHA `7b64a66d178c4384ca85f753717f8acc435433c189719cc4508fdac8b3feb1a1` 被新 SHA 替换且无逐图原身份。**复现 F-05**。旧二进制 SHA 与 M6 记录对应；本实验只验证合并身份，不用旧二进制证明新源码行为 |
| 既有真实写通道用例 | `mise exec --no-deps -- cargo test -p phoenix-ui --test capture block_writes_covers_every_channel --locked --offline -- --exact --test-threads=1` | 1 pass，5 filtered out；实际浏览器运行，未跳过 |
| capture 离线 oracle | 仓根 `mise exec --no-deps -- node tools/phoenix-ui/tests/oracle/run.mjs --bin tools/phoenix-ui/target/debug/phoenix-ui capture` | 6 pass，0 fail，0 missing；未更新 golden |
| 生成物 | `tools/phoenix-ui` 下 `mise exec --no-deps -- bun scripts/render.mjs --check` | fresh；19 providers；input `43c5a442b5a0205dad1eea1ff1dc88e86e51983ac499f2367f7c298d7cb2bbc8`；bundle `9acc60a83e47b73e2989d033c9898ea33c81dc7f1cd9ccacfe2d9d55f500270d` |
| 四个技能元数据 | 读取现有 `test/skills/test_skill_metadata.py`；在临时目录仅链接 phoenix-ui、feature-demo、review-ui、apply-ui-review，替换其 `SKILLS` 根后执行原 `SkillMetadataTest` | frontmatter 与 openai.yaml 两项通过；未纳入其他技能 |

所有复现仅使用合成页面、文件和回环服务，不访问真实业务数据。复现写入都在仓外临时目录；服务已关闭；临时脚本、截图和快照在报告留存必要复现方法后清理，不添加到实现或测试目录。没有执行修复、提交、推送、发布或 UI 评分。

## 4. 计划符合性

仅核对 P1 M1–M6；P2–P4、发布 npm/R2、comp-led 门禁与独立 UI 评分不在本轮承诺中。

| 需求/本期承诺 | 实现/证据 | 本轮判断 |
| --- | --- | --- |
| WORK-F1、WORK-N1：输入、状态、写拦截、出处、安全边界 | capture/CDP/serve/report；真实 CLI E-1、E-2、E-3、E-5 | **存在缺陷 F-01、F-02、F-03、F-05**；其余已读链路未发现可确认缺陷，不据此替代全部 AC 实测 |
| WORK-F2、WORK-F3：三类主图、派生图、有效性与修复沿用 | derived、validity、mod；E-3/E-4/E-5 | **存在缺陷 F-03–F-05**；正常派生演示沿用 M3 记录，本轮未重跑全部 V-3 |
| WORK-F4：盲测走查、受限交接、可复现问题处置、单列交付 | 新 walker、new-work、finish-reviewer；M5/M6 记录 | 指令落点已核对；真实宿主材料限制见 §2；不把文字落位当成每次运行必然遵守 |
| WORK-F15–F17、C-1/C-2：独立技能可选接入与合同同步 | feature-demo、review-ui、apply-ui-review、README/docs | 可选条件与接口复述已核对，未发现额外确认问题；借用 capture 时仍受 F-01–F-05 影响 |
| WORK-N2/N3：四平台、启动器与既有行为 | oracle、render、M6 四平台记录；本地目标候选对比 | 本轮只实测本机；远程及完整回归限制见 §2，不扩围重跑 |
| WORK-N4、C-3：署名、整合边界 | NOTICE/THIRD-PARTY 增量、计划 §0.6 与技能 diff | 署名与采纳落点已核对；未重跑全部许可审计 |

## 5. 总体结论与修复顺序

验收判定：**需要修改**。

正确性结论：**patch is incorrect**。确认 1 个 P1、4 个 P2，均由登记提交引入或激活；F-01 可使预检真实写入共享实例，F-02 破坏本地静态根边界，F-03–F-05 破坏证据内容、运行结果或出处。已有离线与正常写通道检查通过不覆盖这些反例。

优先修 F-01/F-02 的外部边界，再修 F-03/F-04 的输出完整性，随后修 F-05 并同步出处消费者。完成后用相同合成反例复验：服务端无未允许写入、链接不能读出根外、主图不被派生图覆盖、窄元素不 panic、沿用图保留原工具身份；再按受影响范围补齐真实宿主材料并独立复审。本报告只评审所列版本与范围，不授权合并或发布。

下一步：

```text
$apply-code-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.code-review.md
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md
```
