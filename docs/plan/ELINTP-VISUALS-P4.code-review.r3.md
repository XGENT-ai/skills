# ELINTP-VISUALS-P4 代码评审报告（R3 · 修复效果复审）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3 · 修复效果复审 | [ELINTP-VISUALS-P4](ELINTP-VISUALS-P4.md)，SHA-256 `2e86cd1042617f07a91d5fe7c9ff136bddaf9eb0b1146eba73fae6723e88c1e1` | `2beb7e0d55055fcde82c6ec8ce35247e3ed8606e..b4c7b568539e59f62fc6756945b30686a70c1480`；当前未提交文档及指纹见覆盖节 | [R2](ELINTP-VISUALS-P4.code-review.r2.md)，SHA-256 `d2d084e79ecb083260b13977d627ed5a594d6ac55d238f6709ce15de8da321fc` |

本轮默认按已有最高轮次 R2 进行修复效果复审，覆盖其全部旧问题、待核实与待决策项，以及修复增量的直接回归；不是对原始 P4 实现的完整重审。处置依据为 [M1 评审修复](ELINTP-VISUALS-P4.records/M1.md#评审修复)（A4–A6）及 [M2 评审修复](ELINTP-VISUALS-P4.records/M2.md#评审修复)。

R2 所审目标为 `2beb7e0d`；本轮实现修复为 `5eaf6b11351cf20fcbacb87c4f8d09963b53a14e`（父提交就是 R2 目标），`b4c7b568` 仅回填计划中的修复 SHA。不存在夹杂的未登记实现提交。此前登记的实现提交已由 R1/R2 覆盖，继承其历史范围，不重复宣称本轮完整审过。

## 上轮问题复审

| 来源 / ID | 上轮结论 | 当前证据与核实结果 | 本轮结果 | 剩余动作 |
| --- | --- | --- | --- | --- |
| R1 → R2 / F-01 · P1 | 已解决：饼图说明裁切 | `layout/chart.rs:614-620` 的说明宽度与 `:750-756` 的说明文字在本轮未变；复用 R2 多语言结果。本轮西语单行饼图 `finalize` 自动阶段通过（包含文本边界检查） | **已解决** | 无 |
| R1 → R2 / F-02 · P1 | 已解决：竖向轴刻度过密及比例误差 | 竖向柱/折线使用 `PLOT_H`、最远刻度反推比例的代码未变；本轮 `chart` 测试中的宽柱、20 点折线及间距断言通过，十二类最小示例中 chart 的真实浏览器检查通过 | **已解决** | 无 |
| R1 → R2 / F-03 · P2 | 部分解决：负值最短横柱仍失败 | `layout/chart.rs:1257-1259` 改为从 `zero_x - len` 起画、标注随外端移动，柱右端固定在零线。七组 `[v,1000]`（v 为 −0.334971、−0.5、−1、−1.786、−1.8、−2、−5）全部 `deliver`、`static` 通过；`chart` 测试包含正负、竖横方向及 4 px 错位反例，全部通过 | **已解决** | 无 |
| R1 → R2 / F-04 · P2 | 已解决：横向 0 值标注越界 | `layout/chart.rs:1243-1250` 的 `zero_x + 4`、`center_y + 3.8` 分支未变，本次修改只影响严格负值；复用 R2 含 0 样本的静态图与浏览器通过证据 | **已解决** | 无 |
| R1 → R2 / F-05 · P3 | 已解决：测试名称仍称 eleven | `tests/cli.rs:128` 仍是 twelve，整个 cli 测试文件与 R2 目标相同，复用 R2 11/11 结果 | **已解决** | 无 |
| R2 / F-06 · P2（V-N2） | 单行四形态阅读器交付失败 | `browser/probe.js:263-274` 纳入 `.rail-controls`，与阅读器 `belowFold` 量整个 rail 的口径相符。本轮单行竖柱、横柱、折线、西语饼图、限额、一行双系列柱全部 `finalize` 自动阶段通过；每份检查四个桌面视口和 390×844、亮暗两色 | **已解决** | 无 |
| R2 / 待核实 1 | 单行阅读器/探针根因未定位 | 当前 CSS 的底部网格为 cards/index 两行，controls 与 outline 共用 index；`node-outline.js:138` 在单节点时隐藏 outline。原探针只量 cards/outline，漏掉独占一行的 controls。本轮单行柱实际测得 controls 的 y=949..979、cards 的 y=884..923、diagram 底部 868；扣减高度为 111，`unclipped=true`，首屏高度 900，复现并验证了处置记录中的具体分支 | **已解决** | 无 |
| R2 / 待决策 2 | 需选择 F-06 处置方向 | M1/M2 处置记录明确记载用户选择 (a)，当前提交实际采用统一探针口径的方案，没有收紧 schema 或把缺陷延期。浏览器正例及裁切反例验证成立 | **已解决** | 无 |

旧问题：6/6 已解决（其中 4 项沿用未失效的 R2 关闭证据并核对当前源码；F-03 残留和 F-06 本轮重新实测）。两项待核实/待决策已关闭。未发现修复引入的新问题。

## 确认问题

未发现可确认的问题。没有需要保留的 P0–P3 findings。

## 待核实 / 待决策项

无阻断本轮修复结论的剩余缺口。下述未执行范围属于复审覆盖限制，不是尚未尝试的补证任务。

## 覆盖与验证

意图：使合法的混合量级负值横柱和单行 chart 可以通过既有交付流程，保持真实裁切检查有效。

- 仓库：`/Users/rockie/Documents/gh-xgent/skills`；日期：2026-11-15；分支 `main`；单 agent 审查。
- 已完整读取修复净 diff：两份实现文件 `src/layout/chart.rs`、`src/browser/probe.js`，两份测试 `tests/chart.rs`、`tests/browser_contract.rs`；另核对计划和 M1/M2 记录的差异。追踪了 `check/chart.rs`、浏览器 observation、CDP 的 probe 调用、Reader belowFold、底部网格、单节点隐藏目录，以及新增测试的正反例。
- index 无暂存变更。当前未提交代码文档为 `skills/use-llm-gateway/{SKILL.md,references/api.md,references/recovery.md,references/self-hosted.md}`，按计划模式一并读取净 diff 和全文：平台资格、价格、响应身份与恢复说明内部一致，新增本地链接与标题存在；frontmatter 未变。仅核对该文档增量及引用一致性，不对另一项目的网关实现、生产部署、账单或供应商作验收结论，不运行无关网关。
- 未提交 `.xgent-ai/**`、其他计划的评审报告与 UI 截图，以及本计划已有报告均是记录产物，已枚举，不作为新增代码 findings。P4 的引擎、附带 skill、elintp、测试、构建、bundle 没有未提交实现变更。
- 先将行为与检查对应：最短负值柱 → chart 行为测试及 `deliver/static`；图下控制行的首屏预算 → 六份单行 `finalize`；共享探针是否漏报裁切 → 现有真实 Chrome 正反例；其他图型口径回归 → 十二类最小示例 `deliver/browser-check`。未运行聚合全套测试、同步 bundle、安装、迁移或发布。

### 本轮实际执行

所有命令在本轮目标 HEAD、实现文件与提交一致的工作树上运行。`C` 表示 `mise exec --no-deps -- mbx`，`M` 表示 `--manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline`；`E` 为 `tools/phoenix-diagram/target/debug/phoenix-diagram`，由本轮 `C build M --bin phoenix-diagram` 成功确认匹配源码。

| 行为 | 命令 / 输入 | 实际结果 |
| --- | --- | --- |
| 最短柱、原有 chart 行为与错位拒绝 | `C test M --test chart` | 15/15，通过；新增负值方向与横向 4 px 错位反例均执行 |
| 共用探针的真实 DOM 正反例 | `C test M --test browser_contract real_dom_reader_legend_and_normal_flow_notes_are_measured_without_clip_exemptions -- --ignored --nocapture` | 1/1，通过；正常双节点与新增单节点 Pass；legend-outside-panel、legend-svg-ancestor-clip、legend-count-hidden、notes-document-clip 均按预期 Failed，说明没有把真实裁切视为可扣减尾部 |
| 旧失败输入走真实 CLI | 七份双行横柱 candidate，`E deliver chart c.json c.html --json` 与 `E static chart c.json c.svg --json` | 14 次命令全部退出 0；对应七组数值见 F-03 行。`static` 包含真实浏览器静态图检查 |
| 单行四形态及两个变体 | `E finalize chart c.json c.html --json`，六份 candidate 见 F-06 行 | 六份退出 0，validate/deliver/browser-check 均 pass，60 个视口/主题 coverage 通过。自动阶段 pass；`deliveryStatus: incomplete` 仅因未为临时产物执行完整看图登记，不冒称 complete |
| 共用探针跨图型回归 | 十二份 `skills/phoenix-diagram/examples/minimal.<type>.json`，各 `E deliver <type> … out.html --json` 后 `E browser-check out.html --json` | 十二类全部退出 0；120 个视口/主题 coverage 通过 |
| 修改格式 | `git diff --no-ext-diff --no-textconv --check 2beb7e0d HEAD` | 退出 0 |
| 实际看图 | 打开 −2 vs 1000 静态图亮/暗截图，以及单行柱阅读器亮/暗截图 | 负值最短柱位于零线左侧，−2/1000 数字完整；单行柱的 42、Throughput、示意徽标清楚，底部表格栏可见。只作本次修复的抽样视觉核对，不是完整 UI 验收 |

候选和产物全部写在仓库外的隔离临时目录。首次尝试 `python` 被未配置版本的 mise shim 拒绝，改用 `/usr/bin/python3` 执行临时编排脚本；未更改项目工具配置，此失败不归因于产品。临时脚本仅构造上述输入并调用真实 CLI，不新增仓库测试。

### 复用与限制

- F-01/F-02/F-04/F-05 的旧关闭证据来自 [R2](ELINTP-VISUALS-P4.code-review.r2.md#上轮问题复审)，已对照当前文件与修复 diff；本轮改动没有触及其根因分支。F-02 另由当前 chart 测试实际复核。
- 本轮没有重复 V-5 全量、十一类图面/静态 SVG 的逐字节基线比对、V-6 离线安装与 elintp 成页验收。当前修复未改模板、静态转换、安装链路或 elintp；probe 仅由 CDP 测量时注入，不进入 HTML/SVG，chart 布局修改仅作用于横向负值柱。跨类型测量变化由上述十二类 browser-check 覆盖，未把旧实施记录写成本轮重新执行。
- 六份单行测试均用 42（双系列用 42/60，限额为 42/30），证据类别为 illustrative；负值组亦为 illustrative。这些是有代表性的验收输入，不声称来自业务实测。当前版本的真实入口证据与局部测试、静态审查、抽样看图分别记录。
- 没有完整 UI 验收或 PRD 全量交付差异核查；跨期 WORK-F5/F9/N3 的终验仍在 P5。

### 固定快照与一致性

以下集合逐文件计算 SHA-256，按 `json.dumps(mapping, sort_keys=True)` 的 UTF-8 字节再取 SHA-256，范围指纹为 `8135295f0867c7560e25eb3de946ceeb7e185d3f55176a495e1b012ba501401b`。落盘前及结束前重核 HEAD 与集合内容；本轮报告和台账写入不成为本轮输入。代码部分可由完整提交 SHA 取回；四份未提交网关文档只有内容指纹，未复制为可同步快照，仅原工作树可复现。

| 文件 | SHA-256 |
| --- | --- |
| `docs/plan/ELINTP-VISUALS-P4.code-review.r2.md` | `d2d084e79ecb083260b13977d627ed5a594d6ac55d238f6709ce15de8da321fc` |
| `docs/plan/ELINTP-VISUALS-P4.md` | `2e86cd1042617f07a91d5fe7c9ff136bddaf9eb0b1146eba73fae6723e88c1e1` |
| `docs/plan/ELINTP-VISUALS-P4.records/M1.md` | `49fbaba8743a7edf0fc64eac9470929f95a0548bb0fb3d94df97e19a5a3b16c5` |
| `docs/plan/ELINTP-VISUALS-P4.records/M2.md` | `2b705fe78e0657a641183c339de0cdff76167e285e349105356d3bf08f226b35` |
| `skills/phoenix-diagram/schemas/chart.schema.json` | `3d4de7a8eb3c1599e58720f53aaf75b7c9a41852bfb9b82b60393b0712041108` |
| `skills/use-llm-gateway/SKILL.md` | `6228ab041474bd773f4e361413f487ae0723c62ee9a42e575576b1ab21e3d123` |
| `skills/use-llm-gateway/references/api.md` | `4f0307ca69a496bab3f47a478a0cd7d23eb60cf7136d9e782e8abb7213174547` |
| `skills/use-llm-gateway/references/recovery.md` | `7be20ac687f0d10c00d07b6f75f3c4cd714de2b0db8332b40405a73774debc41` |
| `skills/use-llm-gateway/references/self-hosted.md` | `dc94e696ec29a1cb0a95f6d333ce5a70310321b3fc57b087b2169da33cf1e40e` |
| `tools/phoenix-diagram/src/browser/observation.rs` | `d7106e0c11e3e7292ca79b3c016f4b2ca48435b0a958fb4b098bf0f4ec6538a5` |
| `tools/phoenix-diagram/src/browser/probe.js` | `3f5ef1665dea23afd4a2ab7d86e58c8b9cf58a9a1a61bdde0c1930821f14a4b7` |
| `tools/phoenix-diagram/src/check/chart.rs` | `28ec919b395bc4988a1a351a198a8a2b55cf52232bbb77b9b37cbb5c21796d99` |
| `tools/phoenix-diagram/src/layout/chart.rs` | `18a1dbde256a3458c3c4318cef8c69d8d765e925f03f146a576fbe61f75e6118` |
| `tools/phoenix-diagram/tests/browser_contract.rs` | `76098a644fc93db4d69df4e4aeacfcbda6a51710dabb42a44c35c09783f2aef8` |
| `tools/phoenix-diagram/tests/chart.rs` | `ea445a0d4da29912b9959ee6bc6af0d5e319b497a7b22899eb188cd1c2b7f4c3` |
| `tools/phoenix-diagram/tests/cli.rs` | `e2485003070744a8c6489a493ee1c11f5eb1b0888024eb2fd70df66f37ed14d4` |
| `tools/phoenix-diagram/viewer/reader-layout.js` | `200b17c099771be816343d89e3648a32cb10698890584ab67bab557a989a08e6` |
| `tools/phoenix-diagram/viewer/viewer.css` | `3a241f9ea535edcb73dfd0ceb4c096b3b7e8af21744afb724af0cdb0d4a54408` |

## 计划符合性

| 本轮承担的承诺 | 证据 | 结论 |
| --- | --- | --- |
| §5.4 非零柱至少 1 px、负值在零线另一侧 | `zero_x - len` 与对应标注；七组真实交付、静态图及错位拒绝 | 已核实 |
| §5.4 单行折线合法、schema rows.minItems=1 | 六份单行 finalize；未收紧 schema | 已核实 |
| V-N2 / F-06 采用方向 (a) 修复 | 探针纳入常规流控制行；下方真实裁切仍被拒绝 | 已核实 |
| §5.4 少行竖柱/折线槽宽下限 | 计划两处已就地同步 PLOT_H / 行数，和当前布局实现一致 | R2 的正文不一致已消除 |
| 其他图型兼容 | 十二类最小示例交付及浏览器检查全部通过，产物生成路径未改变 | 本轮直接影响已核实；未重复字节比对 |
| M3/M4/M5 原始交付与跨期终验 | 不在本轮修复增量内；保留 R1/R2 与原实施记录，终验期约束不变 | 未扩大复审结论 |

## 总体结论与修复顺序

**验收判定：通过验收。**

修复效果结论：**已解决**。F-01–F-06 均有关闭依据；R2 剩余的 F-03、F-06 和两项待核实/待决策已在当前代码与真实入口中核实关闭，没有不再适用、豁免或延期项。

正确性结论：**patch is correct**，仅覆盖本报告声明的修复增量及未提交文档的一致性检查。本轮没有发现新增 P0–P3 问题，没有必须执行的整改顺序；原始 P4 实现没有在本轮完整重审。六份单行 finalize 和十二类 browser-check 提供了当前版本真实入口证据，局部测试与裁切反例共同支持修复结论。

本轮代码验收完成，不代表合并、发布、完整 UI 验收或跨期需求终验。可选下一步（不自动执行）：

```text
$review-ui docs/plan/ELINTP-VISUALS-P4.md
$review-prd-dev-gaps docs/plan/ELINTP-VISUALS-P4.md
```
