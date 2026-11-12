# PHOENIX-UI-EVIDENCE-CRAFT-P1 · R2 代码复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2 · 修复效果复审 | [Phoenix UI 多状态取证与盲测走查 P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md) | `c76838ad46249963aca2d2f6a3642c6d8fb1cab9..42a00a1e5daf4448c4440181cba1f410ddf446de`；实现修复为 `e374f794b6ab4f598f59717e24b244fbdcd7c80a`；无本计划未提交实现 | [R1 代码评审](PHOENIX-UI-EVIDENCE-CRAFT-P1.code-review.md) |

本轮意图：核对 R1 的五项修复是否关闭原问题，并检查修复增量及其直接调用链的回归。用户明确要求代码评审；行为要求来自计划、PRD 与原报告，不另增需求，不重做整期初审。

R1 所审最终版本为 `c76838ad46249963aca2d2f6a3642c6d8fb1cab9`，报告 SHA-256 为 `e9ae23bc1864f679ee808b46a3847503937768604cf68f7f34840cf0d6b9565f`。本轮当前版本为上表 target；处置依据见 [M2 评审修复](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M2.md#评审修复)、[M3 评审修复](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M3.md#评审修复) 和 [M5 同步记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M5.md#评审修复同步)。以下结果来自独立核对，未以处置记录的“已修复”代替验证。

## 1. 确认问题

### F-05 · [P2] 部分重取旧报告前保留沿用图的原工具身份（部分解决）

- 来源：[R1 / F-05](PHOENIX-UI-EVIDENCE-CRAFT-P1.code-review.md#f-05--p2-为沿用截图保留原采集工具身份)，沿用旧编号，不计作本轮新增问题。
- 定位：[report.rs:52–56](../../tools/phoenix-ui/crates/cli/src/capture/report.rs#L52)，当前 target 侧；[mod.rs:901–905](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L901) 读取旧报告后仍用当前工具替换顶层身份。
- 问题：从修复前引擎生成的同 schema 报告开始，升级后通过 `--only again` 部分重取时，未重取条目仍没有 `tool`，合并又丢弃它原来依赖的顶层 `tool`。实测沿用的 initial 图片与条目均未变化，但原引擎摘要从整个报告消失；[review-ui:128](../../skills/review-ui/references/target-and-capture.md#L128) 新增的“旧报告无逐张 tool 时回落顶层”会把这张旧图归到新引擎。修复对已经带逐图身份的新报告有效，却未关闭原问题覆盖的旧引擎采图后部分重取路径；计划 [§5.8](PHOENIX-UI-EVIDENCE-CRAFT-P1.md#58-报告终端输出与退出码) 与 PRD [WORK-F1/AC-9](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md#work-f1-按步骤取得多状态界面证据可供其他-skill-调用) 的逐图出处承诺仍未满足。
- 证据：本轮 E-1 从真实旧开发候选采集 initial/again，再用当前源码构建的引擎只重取 again；两次均退出 0、主图均有效。沿用图 `tool` 仍缺失，旧顶层摘要 `7b64a66d178c4384ca85f753717f8acc435433c189719cc4508fdac8b3feb1a1` 不再存在；顶层和重取图均为 `1e139f0a370cf9b1ec79084bc9dc58c7e34957aa8816c1282b309996313123f5`。现有 `an_untouched_shot_keeps_its_own_tool_identity` 用例从已含逐图 `tool` 的报告开始，其通过不能反驳此路径。
- 修复与验证：合并同 schema 旧报告时，将旧顶层工具身份补到缺少逐图身份的沿用条目，保留已有逐图身份，再写入当前运行信息。用修复前引擎的真实输出复验部分重取，确认沿用图解析为原引擎、重取图解析为当前引擎，图片与原拍摄时间保持不变；不能依赖已被替换的顶层身份补录出处。
- 置信度：高；真实双引擎 CLI、文件哈希、报告内容及消费者合同相互印证。

本轮未发现可确认的新增问题。F-05 是旧问题的未关闭部分。

## 2. 上轮问题复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01 | P1；新注册 Service Worker 的 POST 绕过拦截 | M2 / R1-1；`e374f79` 扩大子会话 Fetch 拦截范围 | **已解决** | 已读 `cdp.rs:1103–1133`：恢复前配置拦截；T-1 真浏览器用例通过，服务端仅收到允许路径，报告含 `POST sw`，原 fetch/form/XHR/beacon/iframe/worker 断言也通过 | 无；本机关闭原反例，跨平台限制见 §3 |
| R1 / F-02 | P2；静态根内链接可读取根外文件 | M2 / R1-2；`serve.rs` 增加 canonical 边界 | **已解决** | T-4 通过；E-2 真实 CLI：文件链接、目录链接均返回 404，根内链接返回 200 及合成内容 | 无 |
| R1 / F-03 | P2；并排图覆盖同名主图而仍报告有效 | M2 / R1-3；启动前统一检查主图、无效图及请求派生图名 | **已解决** | 已核对 `mod.rs:698–735` 与实际派生命名；T-2 原 `--name 'sheet-{viewport}.png' --sheet` 反例退出 1、明确冲突、不建输出目录，检查先于浏览器发现 | 无 |
| R1 / F-04 | P2；不足 4 像素元素标注导致 panic、无主图或报告 | M3 / R1-4；布局返回派生错误，调用方保留主图 | **已解决** | `derived.rs:294–307` 与 `mod.rs:1050–1064`；T-3 真浏览器对 2×100 与 100×2 元素退出 2，主图和报告存在、记录标注失败，无 panic | 无；不将派生失败称为全部成功 |
| R1 / F-05 | P2；部分重取丢失沿用图工具身份 | M2 / R1-5、M5；新图加 `tool`，消费者同步 | **部分解决** | T-5 证明已有逐图身份可保留；E-1 证明修复前报告升级后仍丢失原身份，详见 §1 | 补全旧报告沿用条目的出处后，复验 E-1 |
| R1 / §2 V-6、V-9 | 宿主交接、走查与交付原始材料未独立核实 | M5、M6 既有演练叙述；本轮无新增宿主原始材料 | **待核实** | 已重读 M5/M6，包括 A2 植入问题未由走查先发现的实际偏差及 a2v/a2v2 后续记录；本轮未取得原会话、交接与复走材料 | 提供与最终适用版本可关联的原始材料，或按 V-9 重现缺证路径 |
| R1 / §2 V-8 | 未独立查询远程四平台与 Node18 CI | 本轮通过 GitHub 连接器查询 run `42753553795` 的 jobs 与 artifacts | **部分解决** | 旧候选 `d5904e1` 的四个 UI Native 和 UI Node18 job 均 success；制品明确关联该完整 SHA。该候选早于修复，不证明本轮新二进制四平台通过 | 保留旧候选证据的版本边界；最终候选按受影响范围补证，不为此次只读评审自动推送或启动全量构建 |
| R1 / §2 生成物与范围限制 | 未逐字节审查全部 provider/live 生成物；无关 hook/安装重构排除 | 本轮没有这些实现或生成物增量 | **不再适用（作为本轮修复增量检查）** | `c76838a..42a00a1` 文件清单无这些改动，R1 freshness 结论在其原范围内保留 | 不扩大为整期生成物、live 或安装器验收；原限制仍有效 |

旧问题统计：五项代码问题中 **4 项已解决、1 项部分解决**，无“不再适用”的代码问题；另逐项处理上轮证据限制，未把缺证项记为关闭。

## 3. 待核实与覆盖限制

- **V-6/V-9 宿主材料**：M5/M6 的文字记录不能替代原会话及产物。本轮复验了实际 capture CLI 主路径，但未重新运行 Claude/Codex 收费宿主会话，也未独立验收 A1–A6 的全部交接、走查与交付行为。计划指定的 `local/phoenix-ui/v9-src/` 当前不存在；不据此断言所有原材料均已丢失。需补足保存反馈问题或其他可复现走查问题进入修复批次、主代理沿路径复走及最终交付的材料，并声明与最终版本的适用关系。
- **V-8 版本边界**：独立查证了[旧候选 CI run](https://github.com/XGENT-ai/skills/actions/runs/42753553795) 的 job 结果和制品元数据，没有下载并逐一审查各平台制品内容。本轮修复改变了原生源码和共享 CDP 路径，旧候选成功不能直接替代新版本跨平台证据；本轮本机实际运行覆盖 darwin-arm64。未执行 push、CI rerun 或发布。
- 本轮没有重做 R1 的整期初审；未覆盖 P2–P4、独立 UI 评分或无关未登记改动。未穷举 shared worker 等所有嵌套组合；不把未复验的组合报告为缺陷。

## 4. 覆盖与验证

### 4.1 固定版本与范围

- 仓库：`/Users/rockie/Documents/GitHub/xgent/skills`；日期：2026-11-10（Asia/Shanghai）；分支 `main`。
- 原输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md`。已有 R1，按默认规则进行修复效果复审。R1 原始登记提交清单和各自基线见原报告，不在本轮重新引入其历史间隔中的无关提交。
- 基线：`c76838ad46249963aca2d2f6a3642c6d8fb1cab9`；目标：`42a00a1e5daf4448c4440181cba1f410ddf446de`。区间仅有登记修复 `e374f794b6ab4f598f59717e24b244fbdcd7c80a` 和计划 SHA 回填 `42a00a1e5daf4448c4440181cba1f410ddf446de`；后者只作记录上下文。已验证修复提交可解析且为当前 HEAD 祖先。
- 使用 `git diff --no-ext-diff --no-textconv c76838a HEAD --` 核对完整修复增量。实现与消费者覆盖下表 10 个文件；另读改动的计划及 M2/M3/M5 记录作契约和处置依据。
- 开评时 index 无差异；tracked dirty 为五份 `.xgent-ai/` 索引，未提交材料还包括 PRD、决定记录、报告及状态记录，均不作为本计划实现 findings 对象。无关未跟踪 `skills/software-copyright-docs/` 沿用 R1 已记载的排除范围。本轮无未提交 capture 或 skill 实现增量，不把台账与新报告当作新修复输入。
- 计划 SHA-256：`39e47cc8dff6b0d0bb7c8b3ce9ade44783d249046df86824a4ba58c2df6c48b8`；R1 哈希见报告头。开评固定修复文件、相关上下游、计划/PRD/处置记录等 30 个文件的排序 `路径→SHA-256` JSON，整体 SHA-256 为 `d247e0c6a242d837582bb3888e8e6ab0d947814e7a5af571db79e69f11fb500e`；写报告前逐项比对无变化，HEAD 未变化。该集合用于一致性检查，不表示其中每个文件都完整重审。
- 全部复核由当前主 agent 完成，未派独立 reviewer。

### 4.2 文件与行为清单

| 文件（相对仓根） | 本轮覆盖 |
| --- | --- |
| `tools/phoenix-ui/crates/browser/src/cdp.rs` | 子目标附加、恢复前写拦截、owner 关联及日志；F-01 原场景与其他既有写通道 |
| `tools/phoenix-ui/crates/cli/src/capture/serve.rs` | 请求解析、canonical 根边界、实际文件响应；根内/根外文件及目录链接 |
| `tools/phoenix-ui/crates/cli/src/capture/derived.rs` | 小图布局的错误返回、合法图幅沿用原布局 |
| `tools/phoenix-ui/crates/cli/src/capture/mod.rs` | 计划输出名与实际主图/派生命名、错误传播与退出码、逐图工具写入、旧报告读取和合并调用 |
| `tools/phoenix-ui/crates/cli/src/capture/report.rs` | 未重取条目、顶层替换、同 schema 合并和新增测试的输入前提 |
| `tools/phoenix-ui/crates/cli/tests/capture.rs` | 完整新增 diff；真实调用与服务器侧写入观察、输出冲突、窄元素及出处断言 |
| `tools/phoenix-ui/tests/fixtures/capture/pages/{sw.js,tiny.html,writes.html}` | 完整新增/修改内容，实际测试入口与可达输入 |
| `skills/review-ui/references/target-and-capture.md` | 新旧报告出处读取约定及其与 merge 的组合后果；独立 skill 的可选接入边界未改 |

未改动的报告合并与其他必要代码用于证明旧问题的剩余影响；没有把范围外旧代码问题新增为 finding。新增用例保护原报告的明确反例，未发现应单独报告的测试维护负担。

### 4.3 检查选择及实际结果

执行前映射：写拦截 → 真实服务端接收与拦截日志 → T-1；输出冲突 → CLI 错误及无输出 → T-2；窄元素 → 主图、报告及退出码 → T-3；静态根 → 响应码与合成内容 → T-4/E-2；逐图身份 → 合并结果及升级后的真实 CLI → T-5/E-1。仅运行相关用例；没有执行聚合 `npm test`、`test:phoenix`、安装器或发布检查。元数据与 Phoenix 生成物未改，不重复其既有检查。

环境：darwin-arm64，Rust `1.99.1`、Node `v25.2.1`（mise），实测报告浏览器 `Chrome/157.0.8103.40`。先在 `tools/phoenix-ui` 成功执行：

```bash
mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline
```

当前源码 debug 二进制 SHA-256：`1e139f0a370cf9b1ec79084bc9dc58c7e34957aa8816c1282b309996313123f5`。以下 Cargo 命令在 `tools/phoenix-ui` 执行，使用当前 target 源码；均实际退出 0，不是从实现记录推定。

| 检查 | 最小命令 | 实际结果 |
| --- | --- | --- |
| T-1 · 写通道 | `mise exec --no-deps -- cargo test -p phoenix-ui --test capture block_writes_covers_every_channel --locked --offline -- --exact --test-threads=1` | 1 pass / 7 filtered，20.04s；真实浏览器运行，未跳过；服务端仅允许路径，SW 拦截条目存在 |
| T-2 · 重名 | `mise exec --no-deps -- cargo test -p phoenix-ui --test capture a_name_used_by_a_derived_file_is_rejected_before_the_browser --locked --offline -- --exact --test-threads=1` | 1 pass / 7 filtered，1.06s；子 CLI 按预期退出 1，重名先于无浏览器错误，无输出目录 |
| T-3 · 窄元素 | `mise exec --no-deps -- cargo test -p phoenix-ui --test capture marks_on_sub_4px_elements_report_a_problem_instead_of_panicking --locked --offline -- --exact --test-threads=1` | 1 pass / 7 filtered，21.48s；真实浏览器运行，主图与派生错误报告保留，子 CLI 退出 2 |
| T-4 · 链接 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib capture::serve::tests::symlinks_stay_inside_the_root --locked --offline -- --exact` | 1 pass / 38 filtered；仅合成文件，含根内链接与根外文件/目录链接 |
| T-5 · 新式条目合并 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib capture::report::tests::an_untouched_shot_keeps_its_own_tool_identity --locked --offline -- --exact` | 1 pass / 38 filtered；只证明起始报告已有逐图身份时保留成功 |

**E-1 · 旧引擎到新引擎的真实部分重取。** 在仓外临时目录写合成 HTML（渐变页头、标题和正文）及状态文件：

```json
{"states":[{"name":"again","steps":[{"do":"wait","ms":0}]}]}
```

先用现有开发候选 `local/phoenix-ui/development-release/phoenix-ui-darwin-arm64` 执行 `capture <app> --viewport 400x300 --states <states.json> --out <mixed>`，保存报告与 initial PNG 哈希；再用当前 debug 二进制执行同参数加 `--only again`。旧候选摘要为 `7b64a66d178c4384ca85f753717f8acc435433c189719cc4508fdac8b3feb1a1`，与 M6 最终候选及 R1 E-5 所记一致。结果如下：

| 可观察项 | 实测 |
| --- | --- |
| 两次 CLI 退出码 | 均为 0 |
| initial 与 again 主图 | 均 `valid:true`、400×300；亲自开图确认合成页实际内容 |
| 沿用 initial 的 PNG/条目 | 两者均完全未变 |
| 沿用 initial 的 `tool` | 缺失 |
| 重取 again 和新顶层 `tool.binarySha256` | 均为当前 debug 摘要 `1e139f0a370c…` |
| 旧摘要是否仍在合并报告 | 否 |

旧二进制只用于生成真实旧输入，不用于证明当前源码的其他行为。本反例没有修改报告制造旧格式，也没有修改源码版本号。

**E-2 · 静态根的真实页面读取。** 同一临时合成页面 fetch 三条路径并把状态码与响应写到控制台；`file-link.txt` 指向根外合成文件，`dir-link` 指向根外合成目录，`inside-link.txt` 指向根内文件。当前 CLI `capture <app> --viewport 400x300 --out <links>` 退出 0、主图有效；报告分别为 `404 Not found`、`404 Not found`、`200 synthetic-inside-marker`，未泄露根外合成标记。

**远程既有证据。** GitHub 连接器实际查询 run `42753553795` 的 jobs/artifacts：`UI Native darwin-x64`、`darwin-arm64`、`linux-arm64`、`linux-x64` 和 `UI Node 18 portable consumer smoke` 均 completed/success；制品关联 SHA 均为 `d5904e146588766dda1b425bc6f927a0b861984f`。这补足了 R1 未查询 job 状态的部分缺口，版本限制见 §3，不冒充本轮新候选的跨平台结果。

所有实验只用合成文件、测试服务与独立浏览器资料目录，不访问真实业务数据。复现脚本和输出均在仓外临时目录，完成取证后清理；没有添加仓内测试或修复。M2 记录的其他回归命令仅作为已有证据引用，未标成本轮执行。

## 5. 计划符合性

| 当前修复承诺 | 实现与验证 | 本轮结论 |
| --- | --- | --- |
| M2 / WORK-F1 AC-8、§5.7、V-2②：SW 写拦截 | CDP attach 增量；T-1 | 已核实本机原反例关闭 |
| M1 / §5.9：根外链接拒绝，根内链接可读 | `read_within`；T-4/E-2 | 已核实 |
| M1–M2 / §5.1、WORK-F2：派生图不覆盖主图 | 输出名预检查；T-2 | 已核实原冲突被明确拒绝 |
| M3 / §5.6、V-2⑤：窄元素标注失败保留主图与报告 | 布局错误返回及派生问题；T-3 | 已核实，不再 panic |
| M2/M5 / §5.8、§6.4、WORK-F1 AC-9：沿用图出处 | 新条目 `tool` 与消费者同步；T-5/E-1 | **未完全兑现，F-05 部分解决** |
| V-6/V-9：真实宿主交接、走查、处置及复走材料 | M5/M6 叙述；本轮未重跑宿主 | 待核实，不能宣布整期真实使用验收通过 |
| V-8 / WORK-N2：四平台与 Node18 | 已独立查询旧候选 job/制品元数据 | 旧候选已核实；修复版的相应证据不由此推定 |

P2–P4 与原计划排除项不作为本轮缺陷；本报告不更新计划里程碑或替代 UI 验收。

## 6. 总体结论与修复顺序

验收判定：**需要修改**。

修复结论：**未完全解决**。R1 五项代码问题中四项已解决，F-05 仍有确定的 P2 剩余路径；本轮未确认新增问题。正确性结论：**patch is incorrect**，原因是修复增量尚未兑现沿用图原始出处的承诺，而非已解决问题再次失败。另有宿主材料和新候选跨平台证据限制，不能用五项测试通过扩大为整期验收通过。

先在报告合并边界保留旧顶层身份到相应沿用条目，再用 E-1 从真实旧候选输出复验；关闭条件是沿用图和重取图都能解析到各自原采集引擎，且沿用图像与拍摄时间不变。其余已通过检查在对应代码与条件未变时复用；按受影响范围补齐上述证据后独立复审，不自动启动修复、UI 评分、推送或发布。

```text
$apply-code-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.code-review.r2.md
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md
```
