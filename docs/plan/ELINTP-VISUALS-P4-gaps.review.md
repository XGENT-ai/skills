# ELINTP-VISUALS-P4-gaps · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [ELINTP-VISUALS-P4-gaps](ELINTP-VISUALS-P4-gaps.md) | SHA-256 `ef1571abd216f9f8971d8c28c3dd8cae6904ce305b35a123a51ca30dc41c4bd9`；未跟踪工作树文件 | 全文，四项补缺与 M1–M2 |

评审日期：2026-11-15（+0800）。本轮为首次完整计划评审，不是原 P4 计划评审的修复效果复审。只交付报告及已启用台账的评审事实，不修改计划、代码或实施进度。

## 1. 确认问题

### [P2] F-01 · 饼图占比单元格仍带百分号，与已决纯数字契约冲突

- **定位**：[计划 §5.3，L321](ELINTP-VISUALS-P4-gaps.md#L321) 明确保留占比单元格 `35%`；与 [D-01，L237](ELINTP-VISUALS-P4-gaps.md#L237) 及 [V-2，L356](ELINTP-VISUALS-P4-gaps.md#L356) 的纯数字口径冲突。关联 G-05/D-01、M1。
- **问题**：对 V-1 已安排的 `files` 单位饼图，按设计修订后原值列变成数字，但派生占比列仍复制出 `35%`。已确认的“数值单元格只放数字、单位只在列头”没有统一落实，设计与检查也给出不同预期。这里不主张电子表格必然无法识别百分比；确认问题是输出契约不一致。
- **证据**：[来源报告已决 D-01，L133](ELINTP-VISUALS-P4.prd-dev-gaps.md#L133) 明确选择所有形态的数值单元格只放原值，未记录派生占比列例外。当前 [layout/chart.rs，L1492–1518](../../tools/phoenix-diagram/src/layout/chart.rs#L1492) 在单位不是 `%` 时增加占比列，并把百分号拼入单元格。阅读器 [data_table](../../tools/phoenix-diagram/src/render/chart.rs#L315)、静态 [data-phoenix-table，L1321–1325](../../tools/phoenix-diagram/src/render/static_style.rs#L1321) 与 [elintp 表格输出，L653](../../skills/elintp/scripts/draft.mjs#L653) 都直接消费该字符串，不会在下游去掉百分号。
- **修订**：仍在 `facts.table` 生成处处理；占比列头复用拟新增的 `chart.table.withUnit`，写成 `Share (%)` / `占比（%）`，单元格保留现有派生百分数与舍入规则，只输出 `35`。图面列表继续显示 `35%`，不改变输入值、图面标注或 elintp。
- **复核**：核对 §5.3、D-01 与 V-2 不再对占比单元格给出相反指令。实现验证由现有 V-1 饼图场景及 V-2 表格检查承接，不增加检查组或退出门。
- **置信度：高**，来源决定、计划冲突和生成/消费链均已核实。**阻塞：否**，M1 可直接局部修订，无需重新拍板。

### [P2] F-02 · 将局部图表修正绑定到含无关发布、安装和维护面的全量回归

- **定位**：[计划 V-3，L357](ELINTP-VISUALS-P4-gaps.md#L357) 要求 `npm run test:diagram` 全绿，[M2，L369](ELINTP-VISUALS-P4-gaps.md#L369) 将其作为退出检查；而 [§1.2，L285–292](ELINTP-VISUALS-P4-gaps.md#L285) 将改动限定在 chart 源码、目录及 chart 测试，排除 schema、其他图类实现、阅读器脚本、skill 和分发逻辑。
- **问题**：四项局部修正及相关检查通过后，M2 仍须执行与它们隔离的发布、安装和维护测试；任一无关失败都会阻止退出，并引入额外执行和排错成本。十一类兼容要求有明确来源，但不能由此推出所有维护测试都必须重跑。
- **证据**：[聚合器 L24–55](../../scripts/test-phoenix-diagram-native.mjs#L24) 除 fmt/clippy 外，运行全部 Rust 测试、`test/phoenix-diagram` 下全部测试以及准备工具、许可来源、阅读器交互等测试。其中 [publish.test.js，L15–68](../../test/phoenix-diagram/publish.test.js#L15) 核查两套工具的发行清单、四平台 URL 和发布分支，不读取本次图表结果或新增目录键；[prepare.test.mjs](../../tools/phoenix-diagram/tests/maintenance/prepare.test.mjs) 核查工具准备命令。根 [AGENTS.md 的 `@verification_scope_and_cost`](../../AGENTS.md) 要求检查实际聚合内容并选择相关子命令，不能按工具名默认全套。
- **修订**：从 V-3 移除整个聚合命令的全绿要求，改用变更范围所需的直接静态检查及相关子命令；M1 已通过且条件未变的 chart/目录检查复用，不重复运行。**保留**明确要求的 WORK-N3 十一类兼容比对、WORK-F9 静态图及体积检查，以及 V-4 真实使用验收。不修改聚合器或既有测试，不借本问题删减明确验收要求。
- **复核**：V-3、M2 及收口描述不再以无关发布/维护测试全绿为退出前提；留下的检查均能对应本次改动或明确验收条款。由既有检查覆盖，不新增验收门。
- **置信度：高**，聚合器和无关测试的实际行为已核实。**阻塞：否**，属于本计划可直接修订的验证范围问题。

## 2. 待核实 / 待决策

**没有影响实施就绪、需要用户或外部提供方补答的关键前提。** 以下是已安排的实施后验收，不是本轮确认缺陷，也不代表现在已经通过：

| 项目 | 本轮证据与缺口 | 提供者 / 最晚确认点 |
| --- | --- | --- |
| 强调色、虚线框在亮暗主题及静态图中实际可辨 | 颜色变量、渲染和静态 CSS 支持已读；未生成改后截图 | 实施者，既有 M1/V-1 |
| 十一类兼容及十二类静态结果、体积 | 已核实 chart 分支隔离；本地原基线目录存在，但未重算其指纹或执行新旧比较 | 实施者，既有 M2/V-3；缺失时按计划既有重建路径处理 |
| 离线安装、浏览器复制、elintp 零网络阅读 | 表格字符串契约及同源传播成立；未执行安装或浏览器验收 | 实施者，既有 M2/V-4 |

D-01/D-02 的用户决定依据为来源差异报告的已决表；未取得原始问答会话。本轮未将历史报告中的运行结论冒充当前实测。

## 3. 覆盖摘要

### 3.1 基线与分工

- 仓库：`XGENT-ai/skills`，当前分支 `main`，HEAD `b4c7b568539e59f62fc6756945b30686a70c1480`。
- 当前工作树 dirty 参与调查；目标计划未跟踪，另有台账、其他报告及 `use-llm-gateway` 修改。本轮不处理这些既有变更。`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 相对 HEAD 无 tracked diff。
- 需求基线：[PRD v1.2](../PRD-ELINTP-VISUALS.md) 的 P4 范围及 [差异报告 G-01/G-02/G-05/D-01/G-06/D-02](ELINTP-VISUALS-P4.prd-dev-gaps.md)；[原 P4 计划](ELINTP-VISUALS-P4.md) 与 [M5 记录](ELINTP-VISUALS-P4.records/M5.md) 仅作背景及既有契约来源，不是本轮修订对象。
- 实际并行启动两位只读 reviewer：facts（`7393d8b0`）及 architecture（`55552ac3`），继承主模型；均未改稿、实现或派生 agent。主 agent 同期执行硬检查，重新读取候选问题的源证据，合并同根因问题，单独撰写本报告。两位均已完成，无遗留运行命令。
- 写报告前复核 HEAD、目标计划 SHA-256 及相关代码 diff，均与调查基线一致。

### 3.2 两个重点与硬检查

| 覆盖项 | 核实内容与结果 | 证据等级 |
| --- | --- | --- |
| facts：图例与强调色 | `legend_glyph` 当前使用系列 id；布局已携带 key。chart 实色规则遗漏 `security`/`external`，但主题变量存在；计划修法对应真实根因 | 当前源码直接证据 |
| facts：表格传播 | 布局生成 `facts.table`，阅读器、静态 SVG 与 elintp 直接消费；无需宿主改写。发现 F-01 | 源码及已决契约直接证据 |
| facts：仅上限行 | 现有布局用限额值生成 fill；计划保留长度、换为 span 并相应修改检查，符合 D-02 | 当前源码与已决契约直接证据 |
| architecture | 已评估数据归属、本地化、图形语义、静态转换和验证成本；四项根因层修法成立，F-02 要求减少无关回归 | 源码/契约评估；无运行通过声明 |
| 硬检查：范围 | 修改属于本计划明确的引擎增量；无擅自改 elintp、其他图类或外部系统交付 | 全文及接口核对 |
| 硬检查：发布与评审 | 未将生产发布或代码评审安排为实施里程碑；完成后提示评审不等于评审门 | 全文核对 |
| 硬检查：需求落点 | G-01/G-02/D-01/D-02 均有设计及既有检查；跨期终验仍归 P5。F-01 是已有落点的契约冲突，而非另增验收项 | 需求、设计及检查映射 |
| 硬检查：交付断点 | 只修改输出内容，输入/schema/命令不变；静态传播、目录装配和安装后真实使用路径有落点，未发现必须新增的环节 | 本地契约核对；安装未实跑 |

### 3.3 检查执行与限制

本轮只检查计划引用及源码证据，不实施计划，也不为评审运行功能全套：

```sh
bash skills/dev-plan/scripts/check_paths.sh docs/plan/ELINTP-VISUALS-P4-gaps.md /Users/rockie/Documents/gh-xgent/skills
```

实际完成，退出码 0：检查 19 个路径样式 token，1 个 `MISSING: bundle/ENGINE.json`。人工核实它是计划 §12 所指的**构建输出目录相对路径**；[构建器 L438–440](../../scripts/build-phoenix-diagram.mjs#L438) 确实生成该文件，不是错误复用路径。路径脚本不证明行为正确。

未运行 Cargo/Node 功能测试、全量回归、开发构建、安装、`finalize`、`static` 或浏览器；未创建临时构建目录、worktree 或容器。没有新增依赖或依赖最新版本的主张，本轮无需联网查询支持周期或外部服务能力。

## 4. 关键决策评估

| 选择 | 驱动需求 | 收益 / 代价与持续性判断 | 裁定 |
| --- | --- | --- | --- |
| 表格只在引擎 `facts.table` 生成，宿主只显示 | D-01、WORK-F9/AC-5 | 一处修改覆盖阅读器和静态宿主，避免宿主清洗与重复事实；沿用字符串结构，无迁移 | 保留；按 F-01 去掉占比列例外 |
| 五语言目录新增 `chart.table.withUnit` | D-01、C-1 | [Locale::text](../../tools/phoenix-diagram/src/i18n.rs#L103) 已支持占位符；[装配器](../../scripts/assemble-phoenix-diagram-reader.mjs#L48) 检查目录键和占位符一致；无须硬编码语言分支或新库 | 保留 |
| span mark、虚线框与「仅上限」图例 | D-02 | 在图形语义层区分缺当前值，保留用户明确选择的长度表达；[静态转换声明支持](../../tools/phoenix-diagram/src/render/static_style.rs#L629) 已覆盖所需 CSS | 保留，不改成“不画填充” |
| 补 chart 的 security/external 规则，保留系列循环色 | G-02 | 在规则生成处同时修填充和图例，不新增全局主题或特判 | 保留 |
| 保持 schema、命令和表格数据结构不变 | 局部输出内容修正 | 现有边界足够；没有证据要求协议升级、兼容层或范围外 skill 修改 | 保留 |
| 全量维护回归作为收口要求 | V-3 | 包含与本次变化隔离的发布/维护检查，无对应交付收益；明确兼容与真实使用验收仍必要 | 按 F-02 缩小 |

这里的先进性指正确的数据归属、局部演进和可维护性，而非换用更新技术；未发现必须引入新依赖或推翻当前架构的依据。

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

- **评审建议状态：Ready（按上述明确修订落实后）**。原计划自报状态为 Ready；没有需要改判 Blocked 的用户决策或外部前提。两条 P2 由“需要修改”承接，不借 Proposed 收纳缺陷，也不把报告理解为无条件通过。
- 共确认 **2 条 P2，0 条 P0/P1/P3**。没有待补的关键技术证据；运行与视觉目标仍由既有实施验收证明，Ready 不表示已实现或可上线。
- **修订顺序**：① 修 §5.3 饼图占比的列头/单元格契约，统一 D-01 与 V-2；② 缩小 V-3 及 M2 对聚合回归的依赖，保留明确兼容门及真实使用验收。其余正确设计不重写。
- 不增加新里程碑、防御机制、专项验证或收口门；不修改源代码、原计划状态或进度。

下一步建议（仅建议，不自动执行）：

```text
$apply-doc-review docs/plan/ELINTP-VISUALS-P4-gaps.review.md
```

核实并修订计划，记录两项处置后重新确认 Ready；本报告不启动实施或发布。

## 修订处置与复核

- 来源：本报告原 §1–§5，所审计划 SHA-256 `ef1571ab…c4bd9`，报告追加本节前 SHA-256 `504a84f8…c401b`；F-01、F-02 编号不变。
- 目标：[ELINTP-VISUALS-P4-gaps](ELINTP-VISUALS-P4-gaps.md)；修前 `ef1571abd216f9f8971d8c28c3dd8cae6904ce305b35a123a51ca30dc41c4bd9`（与所审版本相同），修后 `bba671e0dfcf72749715927c58f6052cd107a6b7190c52b775b250444be8f87e`；仍为未跟踪文件。
- 复核日期与代码基线：2026-11-15（+0800）；main / `b4c7b568`，dirty；`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 相对 HEAD 无改动。
- 本次范围：全部确认问题（F-01、F-02）；§2 的实施后验收项不属于文档修订。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。[layout/chart.rs L1512–L1517](../../tools/phoenix-diagram/src/layout/chart.rs#L1512) 用 `format!("{}%", …)` 生成占比单元格；[差异报告 D-01](ELINTP-VISUALS-P4.prd-dev-gaps.md#L133) 写“所有形态数值单元格写原值”，没有派生列例外。无单位饼图（如 `pie_sectors_and_list_match_facts`）同样有占比列，不只影响 `files` 单位 | 已修订并复核 | §5.3 饼行：占比列头经 `chart.table.withUnit`（`{unit}` 为 `%`），单元格只写数字、保留舍入；表后补“占比是派生数值，同守 D-01”，图面列表仍为 `35%`。V-1 结果补饼图表头 `Value (files)`/`Share (%)`；V-2 补 `pie_sectors_and_list_match_facts` 断言（列头带 `%`，单元格 `50`）。D-01 账本行、elintp 与 schema 未改 | §5.3、D-01、V-1、V-2 对占比单元格口径一致，全文无 `35%` 单元格指令；实现效果留待 M1 的 V-1/V-2 |
| F-02 | 成立。[聚合器 L28–L54](../../scripts/test-phoenix-diagram-native.mjs#L28) 在 fmt/clippy 外运行全部 Rust 测试、`test/phoenix-diagram/*.test.js`（含发布、安装、打包）及准备、许可来源测试。直接读取 chart 产物或目录的既有测试已核实：`tests/i18n.rs`（`catalogs.json`）、`tests/check_anchors.rs`（`minimal.chart.json`）、`tests/browser_contract.rs`（单行柱图）、`test/phoenix-diagram/reader-assets.test.js`（`template.html`） | 已修订并复核 | V-3 首条改为“改动 crate fmt/clippy + 直接消费方测试通过，V-2 未失效时引用”；步骤 1 换成对应四条命令，并注明聚合命令不作退出前提。M2 内容“全量回归”改为“改动范围回归”。十一类兼容比对、静态体积、`dist/` 还原与 V-4 不变；聚合器与既有测试未改 | V-3、M2 不再以无关发布/维护测试为前提，留下的每条检查都对应改动面或 WORK-N3/WORK-F9 条款；`@verification_rhythm` 是固定协议文字，未改 |

### 决策与未完项

无新增用户决策；D-01/D-02 沿用差异报告已决表。F-02 选出的 Rust/Node 子命令是本次修订的计划内容，尚未运行；在 M2 执行时由实施者记录结果。

### 修订后结论与验证

- 已修订并复核 2 条，不采纳 0，未完 0。计划状态行保持 **Ready**：两条 P2 已落到正文，没有需要用户或外部提供的前提；Ready 不代表已实现或已验收。
- `bash skills/dev-plan/scripts/check_paths.sh docs/plan/ELINTP-VISUALS-P4-gaps.md <仓根>`：退出 0，19 个 token，仍只有 `bundle/ENGINE.json` MISSING（构建输出相对路径，同评审时判定）；新引用的四个测试路径均存在。
- `bash skills/dev-plan/scripts/check_progress.sh docs/plan/ELINTP-VISUALS-P4-gaps.md <仓根>`：退出 0，进度 0/2，ERROR 0；1 个 WARN 来自工作树里其他计划和台账的未提交改动，本计划没有开始实施，不适用。
- 未运行 Cargo/Node 功能测试、构建、安装或浏览器；本次只修订文档。本次是修订复核，不代表又做了一轮独立评审。
