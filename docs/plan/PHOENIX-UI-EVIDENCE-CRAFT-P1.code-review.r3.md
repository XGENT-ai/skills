# PHOENIX-UI-EVIDENCE-CRAFT-P1 · R3 代码复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3 · 修复效果复审 | [Phoenix UI 多状态取证与盲测走查 P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md) | `42a00a1e5daf4448c4440181cba1f410ddf446de..92ec5fc8fb35c2e182ea9a79fc5f920a66a45e46` 中本计划修复 `1a58e6a3b2ed32d22eb9927ed3e82a201a13caa4`；无本计划未提交实现；文件与排除项见 §4 | [R2 代码复审](PHOENIX-UI-EVIDENCE-CRAFT-P1.code-review.r2.md) |

本轮意图：核对旧报告部分重取后是否保留沿用图的原采集工具身份，复核 R2 旧问题与待核实项，并检查修复直接引入的回归。用户确认“只审本计划（推荐）”，不纳入其他合入，不重做整期初审。

R2 目标为上述 base，报告 SHA-256 为 `cb7c965eac6b0a5650d3f3672841699b76417936cde31e43c5987a42052d90e3`。处置记录：[M2 评审修复与 R2-5a/5b](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M2.md#评审修复)、[M5 证据限制](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M5.md#r2-复核与证据限制)、[M6 证据限制](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md#r2-复核与证据限制)。本轮独立读取源码并复验，不以实施者的完成记录替代行为证据。

## 1. 确认问题

未发现可确认的新增问题。R2 尚未关闭的 **F-05 已解决**；F-01–F-04 的已解决结论保持。没有剩余已确认的 P0–P2 代码问题。

此结论限于本轮修复及旧问题，不表示整期真实使用验收已通过；未关闭的证据项见 §3。

## 2. 上轮问题复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01，经 R2 §2 | P1；Service Worker 写入绕过拦截；R2 已解决 | `e374f79`；本轮该实现未变 | **已解决** | 重读 [cdp.rs:1103–1133](../../tools/phoenix-ui/crates/browser/src/cdp.rs#L1103)，仍在恢复子会话前启用 Fetch；复用 R2 T-1 真浏览器证据 | 本机原反例保持关闭；V-8 限制另列 |
| R1 / F-02，经 R2 §2 | P2；静态根外符号链接可读取；R2 已解决 | `e374f79`；本轮该实现未变 | **已解决** | 重读 [serve.rs:149–161](../../tools/phoenix-ui/crates/cli/src/capture/serve.rs#L149)，canonical 根边界保留；复用 R2 T-4/E-2 | 无 |
| R1 / F-03，经 R2 §2 | P2；派生图覆盖同名主图；R2 已解决 | `e374f79`；本轮该实现未变 | **已解决** | 重读 [mod.rs:698–735](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L698)，主图、无效图和请求派生图仍共用启动前冲突检查；复用 R2 T-2 | 无 |
| R1 / F-04，经 R2 §2 | P2；窄元素标注 panic；R2 已解决 | `e374f79`；本轮该实现未变 | **已解决** | 重读 [derived.rs:295–307](../../tools/phoenix-ui/crates/cli/src/capture/derived.rs#L295) 与 [mod.rs:1050–1064](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L1050)，错误仍进入派生问题，主图与报告保留；复用 R2 T-3 | 无 |
| R1 / F-05，经 R2 §1 | P2；新式逐图身份已保留，但旧格式部分重取仍丢失原身份；R2 部分解决 | `1a58e6a`；M2 R2-5a/5b | **已解决** | [report.rs:37–65](../../tools/phoenix-ui/crates/cli/src/capture/report.rs#L37) 先保留旧顶层 tool，只补无逐图 tool 的沿用条目；本轮 T-1 和真实双引擎 E-1 均通过 | 无；关闭的是原始出处仍在旧报告中的升级路径，不能恢复此前已被覆盖丢失的数据 |
| R1 / §2 V-6、V-9，经 R2 §2–3 | 交接、走查、修复复走与最终交付原始材料缺失；待核实 | M5/M6 新增明确缺证说明，没有新增原始材料 | **待核实** | 重读完整 M5/M6；`local/phoenix-ui/v6-head`、`local/phoenix-ui/v9-src` 当前均不存在；未取得原会话与产物 | 补原始材料及适用版本，或按计划缺证场景重跑 |
| R1 / §2 V-8，经 R2 §2–3 | 旧候选四平台及 Node18 job 已查询；当前修复版缺证 | R2 的 `d5904e1` / run `42753553795` 证据；M6 保留版本边界 | **部分解决** | 复用 R2 已核实的旧 run 元数据；本轮未取得覆盖 `e374f79` 与 `1a58e6a` 的新候选平台证据 | 当前候选补四平台及 Node18 结果、capture 实际截图记录 |
| R1 / §2 生成物与范围限制，经 R2 §2 | 未逐字节重审全部 provider/live 生成物；无关 hook/安装改动排除 | 本轮没有相关增量 | **不再适用（作为本轮修复增量检查）** | 本轮实现仅 report.rs；原报告限制不扩大成“已全部验收” | 保留原限制，不重做整期或范围外评审 |

五项代码问题统计：**5 项已解决，0 项仍存在，0 项回归**；其中 F-05 本轮新增关闭，F-01–F-04 复用仍有效的 R2 验证。证据缺口未算作已解决。

## 3. 待核实与覆盖限制

1. **V-6/V-9**：需要与适用候选关联的原会话、交接输入、工具调用、截图/状态文件、走查报告、主代理处置及复走结果、最终交付。尤其要证明可复现的走查问题进入修复批次并沿原路径复验。M6 对 A2、a2v、a2v2 有叙述，但叙述不能替代原材料；本轮没有重新启动整组 Claude/Codex 宿主验收，也不把合成 CLI 实验当作宿主验收。指定目录不存在不等于断言所有材料已丢失。
2. **V-8**：旧候选 `d5904e146588766dda1b425bc6f927a0b861984f` 的 [CI run](https://github.com/XGENT-ai/skills/actions/runs/42753553795) 证据由 R2 已查证，本轮没有重复查询或下载制品。它早于 R1/R2 修复；尤其 R1 改了共享 CDP 写拦截，当前 darwin-arm64 成功不能代替其他平台。需按计划取得当前候选的四个 UI Native、Node18 消费者和实际截图记录。本轮没有推送、触发 CI 或发布。
3. 不重新审查 P1 全部原始实现，不覆盖 P2–P4、独立 UI 评分或用户排除的其他 skill 改动。不因缺证假定产品存在新缺陷，也不因本轮没有 findings 宣布整期通过。

## 4. 覆盖与验证

### 4.1 固定版本与文件范围

- 仓库 `/Users/rockie/Documents/GitHub/xgent/skills`；分支 `main`；日期 2026-11-10（Asia/Shanghai）。原输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md`。
- R2 基线与当前 target 见报告头。完整读取本计划修复增量；唯一实现变更为 `tools/phoenix-ui/crates/cli/src/capture/report.rs`（合并逻辑及一个原缺陷回归用例）。计划及 M2/M3/M5/M6 改动只作契约、处置和缺证上下文；`2a15f53dd0435569a1a16c2b2e19251f3244f544` 仅回填计划 SHA。
- 已解析并确认计划登记的全部 12 个不同提交均为 HEAD 祖先；历史登记提交由 R1/R2 覆盖，本轮新增实现仅 `1a58e6a3b2ed32d22eb9927ed3e82a201a13caa4`，其直接父提交为 `ba5ede6a6b3166803b0420143e979c85b0bea470`。本轮 diff 使用 R2 target 到当前 HEAD 的本计划路径净差异，不混入无关合并内容。
- 用户明确排除：`e201aa91d1605d73db0f67afc8bf843136bce6f7`、`ba5ede6a6b3166803b0420143e979c85b0bea470`、`92ec5fc8fb35c2e182ea9a79fc5f920a66a45e46` 的 software-copyright-docs/README 改动，以及 `f43af93b4b591d3239fe073e4840ab8503ebf882` 的 review-prd-dev-gaps/经验记录。相关 merge 提交不另扩大范围。
- 开评 index 无差异；tracked dirty 为五份 `.xgent-ai/` 索引，untracked 为经验/状态记录、PRD/决定/评审文档，无 capture 或相关 skill 未提交实现。它们已枚举并纳入一致性指纹，属于上下文或记录，不作为实现 findings；本轮报告不反向纳入输入。
- 计划 SHA-256：`edb9c7f37e7a8c1ec593d731a09ce478584ffec997a19068b5d56470004ca770`。对 Phoenix 源码/skill、相关调用方、计划记录和开评 dirty 文件的 2213 个文件建立排序 `路径→SHA-256` JSON，整体摘要 `0bfc67e5646285b36dcd96356c4448de7ec17f7fe358349d99a99995b74f9bb6`；写报告前逐项一致，HEAD 未变。该集合用于检测并发变化，不表示逐文件全面评审。
- 由当前主 agent 独立完成；无子代理。没有修改实现、测试、计划或历史报告。

### 4.2 行为链与检查映射

| 行为 | 检查与本轮范围 |
| --- | --- |
| 旧 schema 报告读取 → 顶层 tool → 沿用条目补充 → 当前头部写入 | 完整读取 report.rs；追踪 mod.rs 的旧报告读取、merge、redact 与写出；T-1、E-1 |
| 已有逐图身份、重取成功/失败、其他 schema、脱敏 | 阅读 merge 分支及本模块所有测试；T-1；本轮没有为静态文字新增测试 |
| 消费者登记出处 | 核对 review-ui `target-and-capture.md:128` 与计划 §5.8、§6.4；沿用图现在有自身 tool，不再误用当前顶层身份 |
| 其余四项旧缺陷 | 当前代码定位复核，并用 Git 比较确认实现、相关测试与配置未变，复用 R2 实测 |
| 宿主/跨平台承诺 | 对照完整计划及 M5/M6、R2 限制；缺证单列，不用局部检查替代 |

检查只选报告合并行为及直接调用路径；未运行聚合 `npm test`、`test:phoenix`、安装器、发布或不相关模块测试。

### 4.3 本轮实际执行

环境：darwin-arm64、mise Rust `1.99.1`，实测 Chrome `157.0.8103.40`。以下 Cargo 命令在 `tools/phoenix-ui` 运行，源码对应报告 target，无相关 dirty 实现。

| ID | 命令 | 实际结果 |
| --- | --- | --- |
| T-1 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib capture::report --locked --offline` | 退出 0；5 passed、0 failed、35 filtered；覆盖旧格式补身份、已有身份保留、重取替换/失败移除、不同 schema 和脱敏 |
| B-1 | `mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` | 退出 0；当前 debug 引擎用于 E-1，不用旧候选证明修后行为 |
| E-1 | 旧引擎 `capture <临时合成页面> --viewport 600x400 --states <states.json> --out <mixed>`，再用当前引擎同参数加 `--only again` | 两次退出 0；旧报告两张均无逐图 tool；修后沿用图身份为旧引擎、重取图及顶层为新引擎；两张均 valid |

E-1 从 `capture --help` 的公开参数构造仓外合成 HTML：初始标题 `Initial state`，点击 Continue 后变为 `Again state` / Done；状态输入为：

```json
{"states":[{"name":"again","steps":[{"do":"click","target":{"css":"button"}}]}]}
```

旧引擎来自 `local/phoenix-ui/development-release/phoenix-ui-darwin-arm64`，SHA-256 为 `7b64a66d178c4384ca85f753717f8acc435433c189719cc4508fdac8b3feb1a1`；本轮源码构建的 debug 引擎为 `0e0148f303cea61b9f98dd5b6be96aee944ef273ca6171cdf880b590bb64c10b`。没有手工修改报告制造旧格式，也没有修改版本号制造不同引擎。

- 沿用 `600x400.png` 的 SHA-256 前后均为 `e026b8161fa4fe9f2d66ff3755b8f893b5a85bc81909bd5a46962f0f1b12401e`；`capturedAt` 保持 `2026-11-10T08:50:13.903Z`。
- 沿用条目除新增原顶层 `tool` 外，所有字段与旧报告逐项相等；原工具摘要仍可查询。重取 again 的时间为 `2026-11-10T08:50:28.432Z`，其 tool 与当前二进制摘要一致。
- 亲自打开两张 600×400 主图：分别显示 Initial state / Continue 与 Again state / Done，内容与状态名一致。机器 findings 为页面规则发现，不是本轮代码缺陷或视觉评分。
- 实验只使用合成页面、独立临时目录和 capture 的临时浏览器资料目录，不访问真实业务数据。临时材料取证后清理；没有新增仓内测试或运行后台宿主会话。

## 5. 计划符合性

| 需求 / 承诺 | 本次任务 | 实现及证据 | 状态 |
| --- | --- | --- | --- |
| WORK-F1 AC-9、§5.8：逐图原始出处与拍摄时间 | M2 / R2 F-05 | report.rs 只为缺少逐图身份的沿用条目补原 tool；T-1/E-1 | **已核实** |
| WORK-F3 AC-3：部分重取且沿用可追溯；§6.4 消费者出处抄录 | M2/M5 直接调用链 | 当前 mod.rs 与消费者合同未变，E-1 的沿用图身份、字段和像素均正确 | **已核实本轮影响范围** |
| F-01–F-04 的安全边界、输出名和派生错误语义 | R1 已修复路径 | 当前源码复核及 R2 有效实测复用 | **已核实原反例保持关闭** |
| V-6/V-9：真实宿主交接、走查、处置与复走 | M5/M6 | 仅有文字记录，缺原始材料 | **待核实** |
| V-8 / WORK-N2：修复版四平台及 Node18 | M6 | 旧候选结果可复用作历史证据，不能覆盖当前原生修复 | **待补证** |

P2–P4 及发布不是本轮验收项。计划的实施完成记录不等于独立验收通过；本报告不回写其进度。

## 6. 总体结论与下一步

验收判定：**需补证**。

F-01–F-05 的代码问题修复结论为 **已解决**；本次 `1a58e6a` 修复增量的正确性结论为 **patch is correct**，未发现新增阻塞回归。包含旧待核实项的整体复审仍为 **无法判定（inconclusive）**：V-6/V-9 原始宿主材料和当前修复版 V-8 尚未补齐，不能据局部修复通过宣布本计划通过验收。

下一步先补齐 §3 的原始材料及版本关联；无法取回的宿主材料按计划缺证场景重跑，跨平台证据在获授权的候选推送/CI 流程中取得。对应代码与条件未变时复用已通过的 T-1/E-1 和 R2 检查，不要求再修改已关闭的 F-05，也不自动启动修复、UI 验收、推送或发布。

补证后独立复审：

```text
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md
```
