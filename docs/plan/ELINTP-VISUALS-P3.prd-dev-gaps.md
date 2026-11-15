# ELINTP-VISUALS-P3 · PRD 与开发交付差异核查

- 审查对象：[ELINTP-VISUALS-P3 开发计划](ELINTP-VISUALS-P3.md)（SHA-256 `2fdb14f89ba17d8a4cdd16dfb235476c8b0fd8500f903f2d398adb85d50f9101`）对照 [PRD-ELINTP-VISUALS](../PRD-ELINTP-VISUALS.md) v1.2（SHA-256 `ff8353d0d1026e2a9e2dd1d884e12ab2708f33fc4f2897cddb097e15373e0d3b`）P3。
- 审查日期：2026-11-13；类型：首次完整核查（非复审）。
- 代码基线：main / `29d692703cfca0e81aa9f7cd5d171ec517f10926`，dirty（仅台账状态页、R1/R2 评审报告与本报告；`tools/`、`skills/`、`test/`、`dist/` 无未提交改动）。
- 输入记录：[M1](ELINTP-VISUALS-P3.records/M1.md)–[M5](ELINTP-VISUALS-P3.records/M5.md)（最后修改于 `cdc6ea52`）、[R1 代码评审](ELINTP-VISUALS-P3.code-review.md)、[R2 修复效果复审](ELINTP-VISUALS-P3.code-review.r2.md)。

## 1. 结论与范围

本期范围：WORK-F9/AC-1 至 AC-4、WORK-F10/AC-1 至 AC-5、WORK-F5/AC-2、WORK-N3/AC-3，以及 PRD §7 要求已交付部分继续满足的 WORK-N1、WORK-N2（计划 C-1）。WORK-F5、WORK-F9、WORK-N3 的终验期是 P5，本期完成后应停在已提交 / 可测试。

| 项 | 数量 |
| --- | --- |
| A · 明确遗漏 | 0 |
| B · 合理变更 | 2（G-02、G-03） |
| C · 需 Triage | 1（G-01；已决 1 / 待决 0） |
| 范围内待核实 | 0 |
| 需跟进 | 1（G-01 → D-01，由 gaps plan 承接） |

整体结论：**存在已决待补齐的差异，存在合理变更。** 本期 13 条 AC 已逐条核实，在下文注明的基线和证据范围内全部有可观察证据。WORK-F9/AC-1 有一处缺口：Reader 图下的卡片不进入静态图，而 Workflow 的文字容量修法正是把超出的文字移进卡片（G-01）。用户已决定卡片以页面内联 HTML 随图交付（D-01），由 [gaps plan](ELINTP-VISUALS-P3-gaps.md) 承接。本报告完成不代表 P3 需求已验收，跨期需求仍在 P5 终验。

## 2. 三类差异

### C · 需 Triage

#### G-01 · 静态图不含 Reader 图下卡片，文字容量修法移入卡片的内容在静态图中丢失 · C

- PRD 期望：WORK-F9/AC-1：内联后“图中全部元素、关系、标签与图例完整显示，与同一 candidate 的完整阅读器图面一致”（[PRD §3 WORK-F9](../PRD-ELINTP-VISUALS.md#work-f9-输出可嵌入宿主文档的静态图)）。WORK-F9 的边界只排除搜索、缩放、Route / Reach、播放与导出控件，没有提到卡片。
- 实际交付：计划 D5 规定静态图只含主图 SVG，不含页面标题、摘要卡片和节点索引（[计划 §0.2 D5](ELINTP-VISUALS-P3.md#02-决策表)），[static-figure.md「产物」](../../skills/phoenix-diagram/references/static-figure.md#产物)也是这样写的。与此同时，[authoring-defaults.md「事实与修复」](../../skills/phoenix-diagram/references/authoring-defaults.md#事实与修复)的 Workflow 修法要求把超出文字容量的内容“移入卡片条目而不删除”。按文档取静态图时，被移走的文字既不在节点里，也不在静态图里；只有 Reader 能看到。
- 证据链：计划 D5、§11 风险行（“用户如要求自带卡片，另立需求”）→ M1/M5 记录未涉及卡片 → 本次 V-3 角色演练（见 §5）：作者第一次 `static` 因 `/nodes/3`、`/nodes/5` 的 sublabel 超出 29 单位失败，按 `supportedFixes` 把六步原文移入卡片后通过；静态 SVG 中没有这段文字，作者自行把它补进了 elintp 图注。statechart 的规则判定卡片同样不进入静态图（[statechart.md](../../skills/phoenix-diagram/references/statechart.md) 说明这些卡片在“图下”）。
- 影响：图解作者与 elintp / summarize 页面读者。主路径不会失败，但内容会静默缺失：卡片里的说明（含修法移入的原文、statechart 规则结论）不进静态图，现有文档也没有提醒宿主补写。这次演练中作者是自己察觉后补上的。
- 分类依据：D5 由计划自行确定，P3 计划评审只讨论了交互属性（[P3 计划评审 F-04](ELINTP-VISUALS-P3.review.md)），没有用户确认卡片的取舍记录。PRD §5.2 只把“静态图的取得方式”交给计划，并写明“不改变本 PRD 的验收含义”。卡片算不算 AC-1 里的“图面”，属于产品范围取舍，不是纯技术替换，所以归 C。
- 处置：补齐，按 D-01 的替代方案（卡片不画进 SVG，以页面 HTML 卡片随图交付）。
- Follow-up：引擎在静态 SVG 中携带卡片数据，elintp `static:` 把它渲染成图下的页面卡片；验收要求卡片标题与条目和 Reader 逐项相同，且满足零网络、亮暗、窄屏与打印要求。由 [ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md) M1、M2 承接。

### B · 合理变更

#### G-02 · 宿主亮暗契约限定为 `<html data-theme>` 或系统偏好 · B

- PRD 期望：WORK-F9/AC-2：“宿主页面切换亮暗时，静态图的文字、线条与背景随之切换并保持可辨认。”
- 实际交付：静态图以 `<html data-theme="light|dark">` 为准，没有该属性时跟随 `prefers-color-scheme`，打印为亮色（[计划 ADR-2](ELINTP-VISUALS-P3.md#adr-2静态图用宿主-data-theme-与系统偏好决定亮暗)）。用其他属性或类名切换主题的宿主只能跟随系统偏好。
- 证据链：ADR-2 → M2 双主题浏览器检查 → M4 修复验收（架构 / ER 两种策略：显式亮暗、无属性时跟随系统、显式暗色下打印）→ R1 实测“无属性时跟随系统偏好，显式属性优先”。
- 影响：本仓两个宿主（elintp 页面、Reader）都用该约定，PRD 场景 S4 / UAT-4 已满足；外部宿主须按文档约定设置主题属性。
- 分类依据：静态图不能带脚本，只能用 CSS 感知宿主主题，必须选定一个约定的信号。计划比较过 `light-dark()` 方案并说明了取舍；约定已写入 [static-figure.md](../../skills/phoenix-diagram/references/static-figure.md#产物)「宿主约定」，证据充分。
- 处置：保留。Follow-up：无。重新评估条件：出现用其他方式表示主题且无法修改的宿主（ADR-2 已写明）。

#### G-03 · 图例计数徽标不进入静态图 · B

- PRD 期望：WORK-F9/AC-1 图例完整，图面与 Reader 一致。
- 实际交付：Reader 图例项旁的数量徽标（如“Processing logic 2”）由阅读器脚本 `viewer/semantic-lens.js` 在运行时添加，静态图中没有。图例项本身、色块和文字都完整。
- 证据链：本次对 workflow 的 blueprint、signal-flow、editorial 三种预设逐一对照 Reader 与静态图截图，只有计数徽标不同；`viewer/export-cleanup.js` L60、L179 显示 Reader 自己的 SVG 导出同样清除 `data-legend-count`。
- 影响：数量是对图面节点的重复计数，不增加新信息，读者可以从图面直接数出来。
- 分类依据：徽标属于阅读器的语义镜头交互层，与 Reader 自带的“完整 SVG 导出”边界一致；用户可见的图例语义不变。
- 处置：保留。Follow-up：无。

## 3. 完整需求覆盖账本

证据等级：文档自述 / 静态代码核实 / 已有运行记录 / **本次实测**。

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F9/AC-1 十一类静态图内联后完整、与 Reader 图面一致（PRD L286） | P3 | §5.1–§5.3；M1、M5 | M1 V-1、M5 十一类表、44 张看图 | 已有运行记录：11 类 × 2 策略三门 pass，44 张经子代理看图。本次实测：workflow 三种预设与 Reader 对照（样式生效，仅计数徽标差异）；tree 的 `collapsed` 分支在静态图中展开（Refunds 可见）；erd、statechart 截图完整；22 份产物无 `tabindex` / 按钮 / 脚本 / `foreignObject` | 部分满足（卡片缺口已决，待 gaps plan 补齐） | G-01（C → D-01）、G-03（B） |
| WORK-F9/AC-2 无脚本、无外部资源，随宿主亮暗切换（PRD L287） | P3 | §5.2–§5.4；M1、M2 | M2 四项检查；M4 修复验收 | 已有运行记录：`offline-resources`、`theme-state` 两条覆盖 pass；M4 显式 / 系统亮暗、打印亮色、请求 0。R1 实测 | 已满足（宿主约定见 G-02） | G-02（B） |
| WORK-F9/AC-3 静态图 ≤ Reader HTML 的 50%（PRD L288） | P3 | §5.1；V-1、V-5 | M5 十一类表 | 已有运行记录：embedded 13.3%–14.7%，system 3.9%–5.6%。本次实测：角色演练图 system 54,221 B 对 Reader 982,202 B（约 5.5%，子代理据 validate 回执推算） | 已满足 | — |
| WORK-F9/AC-4 校验失败不出图；不改 Reader HTML 及其交付状态（PRD L289） | P3 | §5.4；M1 | M1 无效输入 | 已有运行记录：坏 candidate 退出 1、不写 SVG、summary 记 not-run；Reader HTML SHA-256 前后一致。静态代码核实：`static` 不读写同名 `.html` 与 finalize summary | 已满足 | — |
| WORK-F10/AC-1 单图待审为 1440×900 亮、暗各一张（PRD L304） | P3 | §5.5；M3 | M3 截图减量 | 静态代码核实：`evidence.rs` `requires_capture` 只采 `DESKTOP_VIEWPORTS[0]`，`mod.rs` `required: 2`。本次实测：三份预设 workflow 的 `finalize` 各列 2 张 | 已满足 | — |
| WORK-F10/AC-2 自动检查仍覆盖四个桌面窗口加窄屏，溢出即失败（PRD L305） | P3 | §5.5；M3 | M3、M5 全量回归 | 静态代码核实：测量循环 `DESKTOP_VIEWPORTS + NARROW_VIEWPORT` 未改，`summarize_named` 默认 10 条，任一失败即 Failed；`browser_contract.rs` 溢出变体随 M5 全量通过。本次实测：比较 visual-check 每视图 10 条 | 已满足 | — |
| WORK-F10/AC-3 架构比较三视图各亮暗一张，缺观察不能登记（PRD L306） | P3 | §5.5；M3、V-2 | M3 只记代码与测试；记录中无真实比较 E2E | **本次实测**：真实 `compare` + `visual-check` 得 6 张截图、30 条覆盖全部 pass；只列 5 张的记录被拒（`review/record`），一张观察为空被拒；6 张齐全按实际观察以 `failed` 登记，`deliveryStatus: incomplete` | 已满足 | — |
| WORK-F10/AC-4 只有截图 / 自动检查 / 没看图时保持未完成（PRD L307） | P3 | §5.4、§5.5；V-1、V-2 | M2 V-2 第 4 项 | 已有运行记录：空观察登记被拒；自动门通过后仍为 `required-pending` / `incomplete`。本次实测同上 | 已满足 | — |
| WORK-F10/AC-5 静态图审核亮暗两张，规则同单图（PRD L308） | P3 | §5.4；M2 | M2 V-2 | 已有运行记录：`review-record OUTPUT.svg` 自动门、身份、截图一一对应校验；22 份 + V-6 登记 complete。本次角色演练登记 complete | 已满足 | — |
| WORK-F5/AC-2 明确要求内联时每张图为静态图、无页内阅读器；summarize 调用满足单文件、零外部资源、图为内联（PRD L215） | P3（跨期，终验 P5） | §6；M4；V-3 | M4 V-3 为命令级拼装，未按计划做角色模拟；R1 F-01/F-02 已在 `cdc6ea52` 修复，R2 关闭 | **本次实测**：只读 skill 文档的子代理按 summarize §3 三条硬要求执行 elintp，依据 [elintp SKILL.md](../../skills/elintp/SKILL.md#出图已安装-phoenix-diagram-就调用) L200 选 `static:` 与 `--font-policy system`；终稿单文件，`<iframe` 0，`data-font-policy="system"`，`@font-face` / `data:font` / JetBrains 均为 0，无外部 `src` / `href` / `@import`；断网打开请求 0，1440 / 360 无页面横向滚动，亮暗切换生效。打印未在本次演练中测，沿用 M4 修复验收中两种策略的打印证据 | 已满足（本期部分；终验 P5） | 演练暴露 G-01 |
| WORK-N3/AC-3 十一类有效输入、命令、已交付 HTML 有效，Reader HTML 不变（PRD L357） | P3（跨期，终验 P5） | §5.6；M5 | M5 十一类 SHA、旧四张登记 | 已有运行记录：11 类 Reader HTML 与基线 SHA-256 逐一相同；旧四张 summary 按四张登记 complete；`npm run test:diagram` pass。静态代码核实：现行复述已改为两张、六张（剩余“四张”只出现在旧 summary 兼容句中），documentation-writer 按 `theme: light` 与 `viewport` 选图，不依赖张数 | 已满足（本期部分；终验 P5） | — |
| C-1：WORK-N1 窄屏 / 打印、WORK-N2/AC-1、AC-2 对静态嵌图持续有效（PRD §7 L470） | P3 质量门槛 | §6.1–§6.2；M4 | M4 修复验收 | 已有运行记录：360px 下 `innerWidth = scrollWidth`，图在自身区域滚动；显式暗色打印为亮色；离线请求 0；伪造标记、CSS 双重编码等攻击退出 2、旧页不变（R2 18/18）。本次实测：角色演练 360 px 结果一致 | 已满足 | — |
| UAT-4 summarize 零网络调用（PRD L458） | P3 部分 | V-3 | — | 见 WORK-F5/AC-2 行 | 已满足（本地页面；门户阅读面另见 A-1） | — |
| UAT-7 审核两张，1920×1080 溢出被自动检查拦下（PRD L461） | P3 | V-2、V-4 | M3、M5 | 见 WORK-F10/AC-1、AC-2 行；没有单独构造 1920×1080 溢出的实测，依据是逐视口判定逻辑不变及合同测试 | 已满足 | — |
| WORK-F9/AC-5、AC-6；WORK-F5/AC-3、AC-6；WORK-N3/AC-4、AC-5 | P4 / P5 | — | — | 不在本期 | 范围外 | 按 PRD §7.3 在 P4 / P5 承接 |

## 4. Triage 与决策账本

### 待决

无。

### 已决

| 决策 ID | 关联 G-ID | 用户选择与理由 | 确认日期与来源 | 新目标/验收边界 | 跟进/延期/豁免去向 |
| --- | --- | --- | --- | --- | --- |
| D-01 | G-01 | 从真实需求出发：静态图是为了不支持 iframe 的阅读面，这类页面可以直接放内联 HTML，所以卡片用页面 HTML 卡片比画进 SVG 更好。卡片内容由引擎以数据形式输出，宿主（elintp）用自己的页面组件渲染，不扩大 HTML 安全白名单 | 用户，2026-11-13，本次核查对话（先说明需求意图，再在交互问题中选「引擎输出卡片数据」） | 静态 SVG 图面、截图与审核不变；取静态图并内联后，Reader 图下全部卡片的标题与条目在宿主页面可读、顺序一致；页面仍单文件、零外部资源、无 iframe，跟随亮暗、窄屏、打印 | [ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md) M1、M2 |

被取代的选项：之前问题卡里的「接受现状并补文档」「引擎把卡片渲染进静态 SVG」「延期」都没有采用。用户提出的替代方向是让卡片作为页面 HTML 随图交付。

## 5. 待核实项与审查基线

### 范围内待核实

无。下列证据缺口已在本次核查中补证：

| 缺口 | 原状态 | 本次补证 |
| --- | --- | --- |
| 计划 §11 风险：signal-flow / blueprint / editorial 预设的静态图未看图；计划把解除放在 M3 批末，但 V-5 用的最小示例都没有预设 | 记录无证据 | 用最终开发引擎对 workflow 加三种预设分别 `deliver`、`finalize` 和 `static --font-policy system`，全部退出 0、三门 pass；逐张对照 Reader 与静态截图，预设样式生效 |
| WORK-F10/AC-3 架构比较的真实 E2E（计划 V-2 第 3–5 步） | M2 / M3 只有代码与 Rust 测试 | 真实 compare → visual-check → review-record 三种记录（见 §3） |
| V-3 角色模拟（只读 skill 文档、summarize 三条硬要求、system 字体） | M4 只有命令级拼装证据 | Claude 子代理只读 `skills/elintp`、`skills/phoenix-diagram` 的文档，用最终开发引擎完整走一遍；产物在 `/private/tmp/claude-501/-Users-rockie-Documents-gh-xgent-skills/1f4510f8-a42f-4ec7-979b-0696018a4012/scratchpad/v3/`。这是 Agent 角色模拟，不是真人数据 |

### 计划层面的偏差（不构成 PRD 差异）

- V-4 引擎部分：计划要求的 `tests/delivery.rs` 静态 CLI 自动用例和 V-2 第 6 步的 Rust 恢复信息断言没有新增，M1 / M2 记录已如实写明，并以公开 CLI 证据替代（R1 判定不构成缺陷，R2 维持）。PRD 不要求特定测试形式，本报告不列为差异；如需防回归，可作为后续工程改进另行安排。
- M4 将“V-3”记为通过，但实际做的是命令级拼装，没有按计划的角色、输入和变体执行。本次补证后结论成立，记录描述仍偏宽，不改写历史记录。

### 范围外观察

- 架构比较 Delta 视图（亮、暗）中，新增关系标签 “Persist” 被新增标记 “+” 部分压住。P3 没有改比较渲染（`tools/phoenix-diagram/src/compare` 无差异），属于既有问题，不归入本期差异；需要时另行立项。
- A-1：SPMS 门户对内联静态 SVG 的实际显示仍未实测（计划 §0.1 A-1，范围外，非阻塞）。
- 角色演练的其他反馈不属于 P3：作者确认后自检又补了内容，是否允许改稿属于 P2 审阅流程；开发引擎与 `skills/phoenix-diagram/ENGINE.json` 不一致，后者是另一构建的元数据，开发引擎的 `source-inputs.json` 与当前树一致由 R2 核对。
- 文档清晰度（非 PRD 差异）：`static-figure.md`「命令」写有“同一 candidate 的静态 SVG 字节数不得超过其 Reader HTML 的 50%”，作者会以为自己要核对，但 summary 里没有比例字段。该比例是引擎属性，已在 V-5 回归中验证。可在以后修改该文件时把这句改为回归保证，不单独立项。

### 基线与验证方式

- 引擎：`local/phoenix-diagram/elintp-p3-final/phoenix-diagram-darwin-arm64`。R2 已逐项核对其 191 个源输入与当前树一致，`29d69270` 之后 `tools/` 未改动。
- 本次实测产物：`/private/tmp/claude-501/-Users-rockie-Documents-gh-xgent-skills/1f4510f8-a42f-4ec7-979b-0696018a4012/scratchpad/gaps/`（预设与比较）和 `.../scratchpad/v3/`（角色演练）。
- 沿用的历史证据：M5 全量回归、十一类 Reader SHA 比对、44 张看图与 V-6 安装验收；R1 / R2 的 elintp 修复复验。相关代码未变，证据仍然适用。
- 未运行：Rust 测试套件、`npm run test:diagram`、安装器（相关代码未变，没有重跑的理由）。

## 6. Follow-up 与 gaps plan

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| G-01 / D-01 / WORK-F9/AC-1、WORK-F5/AC-2、WORK-N3/AC-3 | 引擎在静态 SVG 根下写 `<metadata data-phoenix-cards="">` 卡片数据；elintp `static:` 解析后渲染 `.figure-cards`；同步「静态图不含摘要卡片」的现行复述与 bundle | 卡片标题与条目和同一 candidate 的 Reader `.cards` 逐项相同；页面零网络、无 iframe，亮暗、360 px、打印可读；无卡片数据的旧 SVG 照常内联；Reader HTML 字节不变 | 无 | [gaps plan](ELINTP-VISUALS-P3-gaps.md) M1（V-1、V-2）、M2（V-3、V-4） |
| G-02、G-03 | 无 | — | — | 保留，无需计划 |

gaps plan：[ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md)，生成时状态 `Ready`；实施进度以该计划的[恢复快照](ELINTP-VISUALS-P3-gaps.md#恢复快照)为准。可顺带处理的文档清晰度项（`static-figure.md` 中 50% 体积句的表述）没有纳入计划，因为它不是 PRD 差异。
