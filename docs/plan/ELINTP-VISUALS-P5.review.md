# ELINTP-VISUALS-P5 · 开发计划完整评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [ELINTP-VISUALS-P5](ELINTP-VISUALS-P5.md) | SHA-256 `9fba8c1e8a0f263bf65306bb10fb09d56e69cc3890b443b687f0061164077316`；未提交工作树原文 | 全文；WORK-F7、WORK-F8、本期 WORK-F5/F9/N3，D1–D18、ADR-1/2、M1–M5、V-1–V-7 |

评审日期：2026-11-16（Asia/Shanghai）。首次完整评审；原计划与实现代码均未修改。

## 1. 确认问题

### [P1] F-01 · tree 需要独立表达父子关系的增删，不能只继承子节点状态

- 定位：[计划](ELINTP-VISUALS-P5.md) §5.1 L422、§5.2 L439，关联 D5/D6、§1.2 的布局不改约束、M2、WORK-F7/AC-1/AC-2。
- 问题：已有节点 C 从 A 移到 B 时，作者需要保留 C，删除旧关系 A→C，并新增关系 B→C。计划只给 tree 节点增加 `change`，父子连线仅随子节点的新增/删除；一个 `parent` 无法同时承载旧边与新边。C 不标记或标为 changed 都只会产生一条普通父子线；把 C 标为新增或删除又会在一个视图里错误地隐藏 C。照此实现，tree 无法交付已承诺的关系变更能力。
- 证据：[PRD](../PRD-ELINTP-VISUALS.md) L244 的 WORK-F7 行为明确四类图型支持关系新增、删除，并要求改变关系用删旧加新表达；L250 的 AC-1 要求关系带标记。[tree schema](../../skills/phoenix-diagram/schemas/tree.schema.json) L119–L120 的 `parent` 是单个 id；[输入校验](../../tools/phoenix-diagram/src/input/semantics.rs) `parent_checks` L661–L665 拒绝重复节点 id；[tree 布局](../../tools/phoenix-diagram/src/layout/tree.rs) L859–L885 每节点只从唯一 parent 生成一条关系。复制同 id 节点或只在渲染层补徽标均不能补齐输入缺失的旧关系。用户的 [决定记录](../PRD-ELINTP-VISUALS.decisions.md) L38 也确认 tree 的关系变更，没有排除换父关系。
- 修订：在 tree 输入与有效标记契约层定义独立的新增、删除父子关系表示，明确它与已有 `parent` 的归属和组合规则；无标记输入继续保持原行为。相应定义两个普通视图各自的树约束，以及联合布局如何容纳旧、新父边，更新受影响的 schema、校验、场景、布局与渲染任务。联合图不能未经处理就继续按单父树布局。保留一次布局与单 SVG 方向，不靠复制节点身份绕过关系模型。
- 复核：以“C 保留，A→C 删除、B→C 新增”逐段核对输入到场景的合同，确保实施者无需另定关键模型。现有 V-2 仅用新增目录覆盖 tree，未覆盖独立关系替换；并入 V-2 一句：同一保留节点的旧父边只在变更前出现、新父边只在变更后出现，节点三处都保留。其余实现验证由既有 V-5 承担，不新增检查组或退出门。
- 置信度：高；PRD、输入模型及实际关系生成代码一致证明缺口。阻塞：否；修订属于已确认功能范围内的设计工作，无须新增用户或外部决定，但该契约应在对应实现前修订。

### [P2] F-02 · erd 的 changed 实体也应向全部字段传递标记

- 定位：[计划](ELINTP-VISUALS-P5.md) §5.2 L440、L448；与 §0.1 WORK-F7/AC-3、V-5 的“实体标记带给全部字段”口径冲突，影响 M2。
- 问题：实体写 `change: "changed"`、字段不单独写标记时，计划明确规定字段不继承。结果只有实体表头带变更徽标，字段没有同类标记且不计入 changed 数量，缩减已确认需求。
- 证据：[PRD](../PRD-ELINTP-VISUALS.md) L244 的行为及 L252 的 WORK-F7/AC-3 均要求被标记实体把同一标记带给全部字段，没有 changed 例外；[决定记录](../PRD-ELINTP-VISUALS.decisions.md) L39 确认沿用 erd 行为。计划全文及该决定记录没有授权此例外。[字段渲染](../../tools/phoenix-diagram/src/render/erd.rs) L395–L449 已逐行输出字段图元，也不存在字段不可承载标记的技术前提。
- 修订：有效标记继承覆盖 added、removed、changed 三类实体标记，同步确定与字段显式不同标记的冲突规则，并统一计数、图面及创作说明口径；移除 changed 不继承的例外。
- 复核：核对 §5.2 的有效标记与冲突表、§6.2 的引用文档要求及计数口径一致。实现验证由既有 V-5 的实体标记继承检查承担。
- 置信度：高；直接违反已确认的行为与 AC。阻塞：否，可直接按现有需求修订。

## 2. 待核实 / 待决策

没有需要用户或外部提供答案的关键前置缺口。

说明带与探针相容性、徽标在预设下的可读性、annotation 实际换行与连续空格显示、静态图体积及离线安装结果，均未在本次运行验证。这些是未来实现的验收目标，计划已安排 V-1 至 V-4、V-6、V-7；由实施者在对应批次取得证据，不因尚未实现报为缺陷或增加门槛。D6 对新增端点的约束及 D13 对重叠词段的限制有功能一致性与版式依据，本轮未将其另列为待用户批准的阻塞。

## 3. 覆盖摘要

- 代码基线：main / `8f3e66650c6ce97f673e8695f201e8e0b810b284`。当前工作树参与调查；开始时 dirty 项为 P5 计划及其计划/PRD 台账，`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 无实现改动。计划顶部的 clean 描述是其调查基线，不冒充本次工作树状态。
- 需求基线：[PRD v1.2](../PRD-ELINTP-VISUALS.md)，SHA-256 `ff8353d0d1026e2a9e2dd1d884e12ab2708f33fc4f2897cddb097e15373e0d3b`；结合其决定记录核对本期范围与产品语义。
- 实际分工：`facts` 与 `architecture` 两位只读 subagent 分别独立调查事实与路线；主 agent 通读计划、执行 C 节硬检查，并回读候选问题的原始证据，合并两位 reviewer 重复发现的 ERD 问题。最终报告由主 agent 单写。
- 硬检查：没有确认范围越界、生产发布或独立评审被夹进里程碑、需求完全无落点、安装交付路径缺失。发布留在 §8；跨期终验证据有 §9.2 的引用及失效补证安排。
- 路径检查：实际运行 `bash skills/dev-plan/scripts/check_paths.sh docs/plan/ELINTP-VISUALS-P5.md .`，退出 0；检查 84 个路径 token，提示 14 个 MISSING。人工逐项核对：13 个是计划明确新增的 annotation/change 模块、schema、示例、引用与测试；另一个是待构建的 `local/phoenix-diagram/elintp-p5-base`。没有确认虚构复用路径。脚本结果不证明行为正确。
- 验证范围：只做源码、契约与相关引用核对；未构建引擎、运行产品测试、浏览器或安装验收。本次没有需要外部最新版本资料才能成立的结论，未使用外部服务。

| 关键主张 | 类型 | 原始证据 | 核实结果与影响 |
| --- | --- | --- | --- |
| 可在统一布局后附加 changes | 现状事实 / 未来设计 | [layout/mod.rs](../../tools/phoenix-diagram/src/layout/mod.rs) L36–L52 | 已核实足够，规则无需复制进四类布局 |
| 导出能保留新视图根属性 | 现状事实 | [export.js](../../tools/phoenix-diagram/viewer/export.js) L236–L270；[export-cleanup.js](../../tools/phoenix-diagram/viewer/export-cleanup.js) L150–L158 | 克隆 SVG 且清除列表不含新属性；设计可成立，实际结果仍待 V-1 |
| 静态转换可保留 SVG 根属性条件 | 现状事实 | [static_style.rs](../../tools/phoenix-diagram/src/render/static_style.rs) `scope_selector` L274–L324 | 已核实足够，根属性切换的路线可用 |
| elintp 接受新文字构件 | 现状事实 | [draft.mjs](../../skills/elintp/scripts/draft.mjs) L7–L13 | 已包含 tspan、dy、font-weight、white-space，无须扩大到 elintp 脚本修改 |
| tree tokens 可保留原文字符 | 现状事实 | [layout/tree.rs](../../tools/phoenix-diagram/src/layout/tree.rs) L54–L77 | tokens 保留字符与空白；trim 发生在 wrap_text。计划原文使用 tokens 的选择合理 |
| tree 独立关系变更 / ERD 三类继承 | 未来设计与需求一致性 | F-01、F-02 的原始定位 | 已核实缺口，需修订合同 |

## 4. 关键决策评估

| 选择 | 需求驱动与收益 / 代价 | 先进性与持续性判断 |
| --- | --- | --- |
| changes 模块集中有效标记、计数与派生隐藏信息 | 四类规则复用，避免校验与渲染各维护一份语义 | 保留，有实际复杂度归属；按 F-01/F-02 补齐模型 |
| 一次布局，根属性切换三视图 | 保持未标记内容位置、单 SVG 与现有导出合同；代价是隐藏位置留空 | 保留，符合已确认边界；tree 需要先解决联合边与单父布局的关系 |
| tree 关系跟随节点状态 | 增删子树简单，但无法表达保留节点的关系替换 | 按 F-01 修订输入模型，避免把局部省事固化成对外能力缺口 |
| annotation 行内编号加下方卡片 | 不依赖精确字形测量，换行仍可通过编号与引述对应；读者需上下对照 | 保留，复杂度与已知能力相称，无须额外批注线布局基础设施 |
| 扩展既有静态转换器 | 新属性限定新行为，无标记路径保持原输出 | 保留；实际样式、字体与体积由原有验收验证 |
| elintp 只改选图文字并保留降级 | 保持技能独立、默认阅读器及零网络内联形态 | 保留，范围清楚，无须引擎安装器成为 elintp 的执行前提 |

## 5. 总体结论与修订顺序

是否需要修改：**需要**。

建议状态：**Ready（F-01、F-02 修订后）**。原计划自报 Ready；本轮发现一项 P1 输入契约缺口、一项 P2 需求收窄，尚不能把当前原文当作可直接执行的最终合同。两项都有明确范围内修订方向，无须等待用户或外部决定，因此不据此判为 Blocked。Ready 建议不表示实现、验收或上线完成。

先修 tree 的独立关系模型及联合布局合同，再统一 ERD 三类继承与冲突规则，最后核对受影响的设计、引用文档要求与既有检查描述；其余路线保留。不自动改写原计划或开始实施。

下一步：

```text
$apply-doc-review docs/plan/ELINTP-VISUALS-P5.review.md
```

## 修订处置与复核

- 来源：本报告原文 SHA-256 `661d292abb1134c11f4296de4a27df0d0be64c4b1ee6fc884bb24e78945b33bc`；F-01、F-02 编号不变。
- 目标：[ELINTP-VISUALS-P5](ELINTP-VISUALS-P5.md)；修前 SHA-256 `9fba8c1e8a0f263bf65306bb10fb09d56e69cc3890b443b687f0061164077316`（与所审版本相同），修后 `c5b9abf55176214bcfcc5462254c03d8024b8c46b20839948764442948962b04`。
- 复核日期与代码基线：2026-11-16（Asia/Shanghai）；main / `8f3e66650c6ce97f673e8695f201e8e0b810b284`，dirty 仅为计划、本报告与 SDLC 台账；`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 无改动。
- 本次范围：报告全部确认问题（F-01、F-02）；§2 无待决项。这是修订复核，不是重跑的独立评审。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。PRD L244 要求四类图型都能「删除旧关系并新增新关系」；`tree.schema.json` L119–L120 的 `parent` 是单个 id。另核实三处单入线假定：`layout/tree.rs` `structure` L222–L254 按 `parent` 建单根层级、`render/tree.rs` `Hierarchy::of` L95–L100 从关系反查父节点、`check/tree.rs` L45–L94 要求关系与画出的连线都等于每个 `parent` 一条——后者报告未列，计划原改动清单也未含 | 已修订并复核 | 新增 D19：节点可选字段 `parent_before`，`parent` 仍是布局父节点，换父产生「删除旧父连线 + 新增新父连线」，节点保留；树根不可更换。§5.1 schema（`dependentRequired`）；§5.2 连线有效标记、两个视图的树约束、诊断（`change/dangling-relation` 推广、`change/conflict`、变更前 `tree/cycle`、`unknown-reference`）、计数；§5.3 换父连线的布局（不改既有层级、堆叠排除、外侧通道路线、`tree-edge-before`）及渲染/检查区分；§5.4 换父连线徽标；同步 D5、D6、ADR-1 后果、§0.1 AC-4、§0.5、§1.1 新缺口、§1.2（`check/tree.rs`、不改范围）、§2、§5.10 兼容、§6.2 `change-view.md`、M2 落点、V-2 一句、V-5、§11 两行 | 以「C 保留，A→C 删除、B→C 新增」回读：输入 `{"id":"C","parent":"B","parent_before":"A"}` → 两条连线标记 → 变更前父 A、变更后父 B → 布局按 B 排、追加 A→C 关系 → 徽标与计数 2 → `Hierarchy`/`check/tree.rs` 计入；实施者无需另定模型。V-2 已加移动文件的三视图断言；未新增检查组或退出门 |
| F-02 | 成立。PRD L244、L252 要求被标记实体把同一标记带给全部字段，无 changed 例外；决定记录 L39 未授权例外；计划 §5.2 原文明确「实体标为变更时字段不继承」 | 已修订并复核 | §5.2 有效标记改为三类都继承；`change/conflict` 改为「实体带标记而字段写了不同 `change`」；§6.2 `change-view.md` 写明三类继承与冲突；V-5 改为三类继承 | §5.2、§6.2、V-5 与 D5「含从实体继承的字段」计数口径一致；全文已无「不继承」残留 |

### 决策与未完项

- D19 是在已确认功能范围内的设计选择，未改变产品承诺；新增的限制「tree 根不能在变更前后更换」与 D6、D13 同列 §11 风险行，用户不同意时在 M1 动工前提出。F-02 的冲突规则按 PRD「全部字段带同一标记」字面落实：字段写了与实体不同的标记即拒绝，不做字段覆盖。
- 换父连线绕外侧通道的可读性未实测，已列 §11 风险行，由 V-2 看图在 M2 批末验证；未来实现验收（V-1 至 V-7）均未运行。

### 修订后结论与验证

- 2 项已修订并复核，0 项不采纳、0 项待决。计划状态维持 **Ready**：两项合同缺口已补齐，无关键范围、数据、外部契约或 NFR 阻塞。
- 实际运行：`bash skills/dev-plan/scripts/check_paths.sh docs/plan/ELINTP-VISUALS-P5.md .` 退出 0，86 个 token、14 个 MISSING，与评审时同为计划新增文件与待构建的 `local/phoenix-diagram/elintp-p5-base`，新引用的 `check/tree.rs`、`render/tree.rs` 存在；`bash skills/dev-plan/scripts/check_progress.sh docs/plan/ELINTP-VISUALS-P5.md .` ERROR 0、WARN 1（计划文件未入库导致「未被改过」的误报，实施进度 0/5 未改）。未构建引擎或运行产品测试。
- 关联文档：PRD 与决定记录只读，无需同步。
