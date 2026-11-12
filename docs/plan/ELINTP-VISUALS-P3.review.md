# ELINTP-VISUALS-P3 · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [ELINTP-VISUALS-P3 · 静态图与看图审核减量](ELINTP-VISUALS-P3.md) | SHA-256 `9ca5325a5ff0042df863892d8843a3f3416175a71abc7caa72928391d6316f2d` | 全文；PRD P3、D1–D12、ADR-1/2、M1–M5、V-1–V-6 |

## 1. 确认问题

1. [P1] F-01 · `static:` 标记不能替代终稿的安全边界
   - 定位：[计划 §6.1「静态图内联」D10、§7.1「静态图可信边界」](ELINTP-VISUALS-P3.md#L512-L539)；关联 M4、V-2、WORK-N2。
   - 问题：按计划，任何带 `data-phoenix-static` 的本地 SVG 都会跳过 `inlineSVG` 并原样进入 elintp 主文档；标记可由任意文件伪造，不能证明引擎来源。作者引用含 `<script>`、事件属性、`foreignObject` 或外部资源的 SVG 时，终稿可在主文档上下文执行代码或发起网络请求。现有 `reader:` 信任面不能证明新增的主文档内联信任面安全，反而使影响从 iframe 扩大到宿主 DOM 与存储。
   - 证据：当前 [`draft.mjs:5–99`](../../skills/elintp/scripts/draft.mjs#L5-L99) 的元素、属性、URL 白名单和 [`draft.mjs:221–230`](../../skills/elintp/scripts/draft.mjs#L221-L230) 的 `svg:` 调用是现有安全闸；[`render.test.js:54–66`](../../test/elintp/render.test.js#L54-L66) 明确覆盖脚本、事件、`foreignObject`、外部 URL 与实体编码绕过。PRD [`WORK-N2/AC-1、AC-2`](../PRD-ELINTP-VISUALS.md#L94-L103) 要求终稿内容不执行、打开与阅读不发起网络请求。计划 §7.1 已承认自绘 SVG 会绕过白名单，但仅以“按工具交付物对待”接受，未建立可信来源。
   - 修订：把“来源分派”与“宿主安全解析”分开：`static:` 仍不重跑引擎的 candidate/图型语义校验，但 elintp 必须对即将进入主文档的静态 SVG 做显式安全解析，允许静态图所需的 `<style>`、`data-*`、`@font-face`、限定字体/图片 MIME 与使用位置的 `data:` 及本地片段，同时拒绝脚本、事件、`foreignObject` 和外部资源；保留引擎 ID 前缀，不在 elintp 重命名。不能用字符串查找标记后原样拼接，也不能把 iframe 作为符合“内联 SVG”的替代。同步修改 D10、§7.1、M4 文件契约和审核说明，保持 summarize 零网络要求。
   - 复核：改稿后，计划应能回答“伪造 `data-phoenix-static` 的恶意本地 SVG 在哪一层被拒绝、允许哪些静态专用元素/资源、为何不重跑引擎语义校验”。本期现有 V-2 没有任何检查能发现该回归，故在 V-2 既有安全用例中加入一个伪造静态标记的脚本/事件/外部资源断言，不另建检查组。
   - 置信度：高；调用链、现有安全测试与 PRD 硬约束均已核对。阻塞：否；安全契约明确，修订应先于 M1 文件契约与 M4 集成。

2. [P1] F-02 · summarize 的“系统字体栈”硬要求被误改成门户接受度假设
   - 定位：[计划 A-1、D4、§0.5、§6.3](ELINTP-VISUALS-P3.md#L251-L260)、[D12](ELINTP-VISUALS-P3.md#L311-L333)；关联 M1、M4、V-3、WORK-F5/AC-2、WORK-N3/AC-2。
   - 问题：summarize 明确把“图用内联 SVG，字体用系统字体栈”列为传给 elintp 的硬要求；计划却让每张静态 SVG 内嵌六段 JetBrains Mono，并把问题改成“门户是否剥离 `data:` 字体”。即使门户完整保留字体，产物仍违反调用方文字契约。按 V-3 把三项硬要求原样传给 elintp 后，当前 D4 与 §6.3 的静态图路径必然选中违规分支，S4 主场景不能按承诺交付。
   - 证据：[summarize §3:37–41](../../skills/summarize/SKILL.md#L37-L41) 的原文是“写进 `elintp` 的硬要求”及“字体用系统字体栈”；计划 D4 指定 JetBrains Mono，§6.3 对 summarize 只按零网络/内联识别，没有系统字体策略。PRD [`S4、WORK-F5/AC-2、WORK-N3/AC-2`](../PRD-ELINTP-VISUALS.md#L110-L120) 要求满足调用方明确阅读面限制，且本计划明确不得修改 summarize。
   - 修订：把字体要求纳入引擎静态输出与 elintp 调用契约，而不是依赖门户不剥离来兜底。正确做法是在引擎层定义并生成系统字体策略，使布局、浏览器检查和实际看图使用同一字体环境；若其他宿主仍保留内嵌 JetBrains 的保真策略，须明确两种输出策略的选择入口、默认值和 elintp 分派，summarize 只能走不嵌入 JetBrains、使用系统/等宽字体栈的分支。不得修改 summarize，也不得把该冲突留成 A-1 的开放假设。同步更新 D4、D12、A-1、静态文件契约、M1/M4 与体积/图面复核口径。
   - 复核：沿“调用方三项硬要求 → elintp 识别 → 引擎字体策略 → SVG `<style>`/字体资源”逐跳核对；计划须证明系统字体策略下文字仍完整且检查使用同一策略。现有 V-3 没有字体断言，故把“summarize 分支不含内嵌 JetBrains/`@font-face`，字体族符合其系统栈要求”并入 V-3，不新增验收门。
   - 置信度：高；这是当前文本与调用方硬约束的直接冲突，不依赖未实测的门户行为。阻塞：否；可在本期引擎与 elintp 边界内修订，无须用户另作取舍。

3. [P2] F-03 · 静态样式派生遗漏主图内嵌 `<style>`，系统暗色、打印和作用域合同不成立
   - 定位：[计划 §5.2](ELINTP-VISUALS-P3.md#L401-L433)、[§5.3 步骤 1–3 与文件契约](ELINTP-VISUALS-P3.md#L446-L459)、ADR-2；关联 M1/M4、V-1/V-3、WORK-F9/AC-1/AC-2、C-1。
   - 问题：`staticFigureCss` 只转换 `viewer.css`，而 `render_static` 复用的 `render_svg` 还嵌入节点卡和图类型自己的样式。节点卡暗色填充/描边仅在祖先或 SVG 自带 `data-theme="dark"` 时生效；elintp 没有保存主题时根元素没有该属性，所以系统暗色只切换生成器管理的变量，不切换这些节点卡规则。显式暗色后打印时，这些未限定 `screen` 的暗色规则也仍然生效。与此同时，类型样式使用 `svg[data-tree-ui]` 等选择器，并非计划承诺的只作用于 `svg[data-phoenix-static]`。
   - 证据：[`render.rs:842–844`](../../tools/phoenix-diagram/src/render.rs#L842-L844) 在主图插槽内加入 `svg-definitions.svg` 与 `node-card-style.svg`；[`node-card-style.svg:27–33`](../../tools/phoenix-diagram/assets/node-card-style.svg#L27-L33) 的七类暗色规则依赖 `[data-theme="dark"]`，没有 `prefers-color-scheme` 或打印分支。[`page.html:9–16`](../../skills/elintp/assets/page.html#L9-L16) 只在有有效保存主题时设置 `data-theme`，[`page.css:133–136`](../../skills/elintp/assets/page.css#L133-L136) 打印时不会移除该属性。计划 §5.4 的静态宿主始终显式设置主题，`theme-state` 又只比较根背景计算色，不能覆盖上述节点规则。
   - 修订：在引擎样式拥有层统一处理 `viewer.css`、节点卡和类型内嵌样式：复用同一静态作用域与主题/媒体策略，或把主题差异重构为静态生成器同样管理的变量；保持单一维护源及 Reader `template.html` 字节不变，不在 elintp 字符串补丁式修图。修订后的文件契约须枚举所有 `<style>` 来源，并证明独立打开、无 `data-theme` 的系统亮暗、显式暗色打印及多图同页均满足作用域要求。
   - 复核：检查 §5.2/§5.3 是否覆盖 `node-card-style.svg` 与各 `TypeRenderer.defs` 的实际规则，而非只验证生成 CSS 的单元测试。现有检查未覆盖该条件，故在 V-1/V-3 既有看图与主题断言中分别加入“无主题属性 + 系统暗色”和“显式暗色打印”的节点文字/线条/填充可辨认断言，不新增检查组。
   - 置信度：高；样式来源、选择器及宿主主题属性均已逐跳核对，未把未运行的像素实验冒充证据。阻塞：否；M1 样式契约修订后由既有批次验证。

4. [P2] F-04 · 直接复用 Reader SVG 会留下无功能的按钮语义和键盘停靠点
   - 定位：[计划 §5.3 步骤 1 与“没有交互”契约](ELINTP-VISUALS-P3.md#L446-L459)、D5；关联 M1、V-1/V-3、WORK-F9 边界。
   - 问题：`render_static` 直接命名空间化 `render_svg(scene)`，只隐藏树的折叠控件；普通节点仍带 `tabindex="0" role="button" aria-pressed="false"`，其他图型也有相同交互语义。静态产物没有对应事件处理器，键盘用户会遇到不能激活的“按钮”，与静态图无交互及根 `role="img"` 的阅读合同冲突。
   - 证据：[`render.rs:1086–1109`](../../tools/phoenix-diagram/src/render.rs#L1086-L1109) 对非 ghost 节点固定生成上述三个属性；[`erd.rs:105–131`](../../tools/phoenix-diagram/src/render/erd.rs#L105-L131) 明确称其为 Reader focus hooks，[`tree.rs:224`](../../tools/phoenix-diagram/src/render/tree.rs#L224)、[`waterfall.rs:95`](../../tools/phoenix-diagram/src/render/waterfall.rs#L95) 也生成按钮和 Tab 停靠点。PRD [`WORK-F9 行为与边界`](../PRD-ELINTP-VISUALS.md#L276-L291) 把静态图定义为无脚本、无交互控件的图面。
   - 修订：在共享渲染器引入明确的静态渲染上下文/模式，由各节点属性生成点省略 Reader 专用角色、Tab 顺序、按压状态及事件钩子，同时保留几何、绘制、`<title>`/`<desc>` 和根 `role="img"` 的可访问名称；不要输出后正则删属性，也不要复制一份渲染器。同步更新 §3 接口表、§5.3 与 M1 落点。
   - 复核：计划须列出静态模式保留/移除的语义属性，并覆盖普通节点与各类型 `TypeRenderer`。现有 V-1/V-3 没有该断言，故在既有内联静态图检查中加入“无 `tabindex`、按钮/按压语义和 Reader 事件钩子”的断言，不新增专项验收。
   - 置信度：高；实际属性生成路径和静态无脚本契约一致指向该结果。阻塞：否；属于 M1 可就地修正的共享渲染上下文设计。

5. [P2] F-05 · 静态审核失败的恢复提示仍要求重跑 `finalize`
   - 定位：[计划 D7](ELINTP-VISUALS-P3.md#L263)、[§5.4 `review-record figure.svg`](ELINTP-VISUALS-P3.md#L472)；关联 M2、V-2、WORK-F10/AC-4/AC-5。
   - 问题：计划复用 `review::record` 的三项登记校验是正确的，但同时保留的 HTML/`finalize` 专用诊断不适用于 `.svg`。静态 summary 自动门未通过或 SVG 在交付后变化时，用户会得到“Run finalize”/“run finalize again”；该命令生成 HTML，不能重新生成需要审核的静态 SVG，恢复指引把用户带离本计划主路径。
   - 证据：[`delivery/review.rs:28–42`](../../tools/phoenix-diagram/src/delivery/review.rs#L28-L42) 硬编码“Run finalize until its automated gates pass”和“The HTML differs from the finalized one; run finalize again”。计划 §5.4 只扩展 `summary_path`，§1.2 的改动文件清单没有 `delivery/review.rs`；现有测试只断言诊断码，不会发现错误恢复命令。
   - 修订：保留现有诊断码与身份/截图校验，把消息改为按 summary 的来源命令分派，或改为产物中性的“重新运行原静态/完整交付命令”；将 `delivery/review.rs` 纳入 M2 文件清单和 §4 失败语义。不要为静态图另造一套审核记录实现。
   - 复核：分别沿静态自动门未通过、SVG 已变化两条路径核对计划，诊断应指向 `static`，HTML 原有恢复语义保持不变。现有 V-2 没有覆盖此恢复信息，故把这两种静态拒绝路径的正确命令提示并入 V-2 既有失败用例。
   - 置信度：高；错误字符串、调用入口和计划改动面完全对应。阻塞：否；M2 局部修订，不设置里程碑门。

## 2. 待核实 / 待决策

| 项目 | 缺少的证据 | 影响 | 责任方与最晚确认点 | 当前定级 |
| --- | --- | --- | --- | --- |
| SPMS 实际阅读面 | 未通过真实 `report_submit` 后检查门户对内联静态 SVG 的显示；本评审未执行提交或浏览器验收。F-02 修订后，summarize 不应再依赖门户接受内嵌 JetBrains。 | 只影响真实门户显示的最终确认，不影响本地引擎、elintp 和离线 HTML 的实施路线；本计划本来就不包含提交。 | 用户在下一次真实 `/summarize` 提交后确认；对外宣称 S4 已实际可用之前。 | 非阻塞；保留为实施后的真实环境证据，不以 A-1 代替调用方字体契约。 |

无须用户拍板的技术决策：F-01–F-05 均有当前代码与已确认需求作为依据，可在原计划职责范围内修订。

## 3. 覆盖摘要

### 基线与实际分工

- 调查日期：2026-11-12；仓库 HEAD `53d35750c97f865c5b739ee467eccd686c653ba7`，当前工作树参与调查。
- 计划为未跟踪文件，所审 SHA-256 为 `9ca5325a5ff0042df863892d8843a3f3416175a71abc7caa72928391d6316f2d`；交付前复核 hash 未变。工作树原有 `.xgent-ai/sdlc/status/plan/index.md`、PRD 状态页修改及 P3 计划状态页新增，评审未还原或覆盖原计划。
- 需求基线：[PRD v1.2](../PRD-ELINTP-VISUALS.md) P3；对照 §7 分期矩阵，WORK-F9/AC-1–4、WORK-F10/AC-1–5、WORK-F5/AC-2、WORK-N3/AC-3 均有设计与验证落点，跨期终验归属一致。
- `architecture` 只读 reviewer（run `ebe5d1b4`）独立完成架构审查并交回证据。`facts` 只读 reviewer 首次运行 `3ae531d8` 与重试 `a4fb038b` 均因上游 429 未交回结论；主 agent 按约定接管事实核验，未伪称第二次独立事实复核成功。主 agent 同期完成 C 节硬检查、交叉复核和最终裁定。
- 未实施代码、未修改原计划、未运行浏览器/迁移/部署/提交，也未把静态源码推导表述为像素级或门户实测。结论不依赖外部版本、弃用周期或服务限制，无须外部官方资料补证。

### 事实核验与硬检查

| 核验主张 | 结果 | 对结论的影响 |
| --- | --- | --- |
| `prepare`、编译场景、命名空间和独立 summary 可复用 | 已核实其入口与行为；Reader HTML 保持不变可行 | 总体 Rust 路线保留；F-03/F-04 限定不能直接继承的样式与交互语义 |
| `static:` 原样内联与现有 `svg:` 安全等级相同 | 已核实不成立：新路径跳过现有白名单且进入主文档 | F-01 |
| summarize 只存在门户是否接受 `data:` 字体的未知 | 已核实不成立：系统字体栈是调用方明示硬要求 | F-02 |
| 静态样式只来自 `viewer.css` 且全部 scoped | 已核实不成立：主图还含节点卡与类型样式 | F-03 |
| `review-record` 可完全不改地服务静态图 | 身份与截图校验可复用；恢复诊断不可原样复用 | F-05 |
| 范围、发布/评审、需求落点、交付闭环 | 未发现越界开发或发布/代码评审里程碑；主路径问题已并入 F-01–F-05 | 无额外硬检查 finding |

### 路径脚本

实际执行：

```bash
skills/dev-plan/scripts/check_paths.sh docs/plan/ELINTP-VISUALS-P3.md /Users/rockie/Documents/gh-xgent/skills
```

脚本检查 45 个路径样式 token，报 16 个 `MISSING`。逐项人工分类如下；脚本结果未替代行为核实：

- **未来新增或输出目录**：`tools/phoenix-diagram/assets/static-figure.css`、`tools/phoenix-diagram/src/browser/static_figure.rs` 为计划标记 ★ 的新文件；`local/phoenix-diagram/elintp-p3-base` 是 M5 才生成的测试夹具目录，不是当前复用事实。
- **§1.1 已声明的相对目录**：`assets/template.html`、`assets/static-figure.css`、`viewer/export.js`、`viewer/template.source.html`、`viewer/tree-branches.js`、`viewer/viewer.css` 均相对 `tools/phoenix-diagram/`；除 ★ 静态 CSS 外均实际存在。
- **skill/引用相对路径**：`phoenix-diagram/SKILL.md` 对应 `skills/phoenix-diagram/SKILL.md`；`references/architecture-compare.md`、`delivery-contract.md`、`preview.md`、`viewer-runtime.md` 相对该 skill，`references/draft-format.md` 相对 `skills/elintp/`，均实际存在。

未发现把不存在的路径声称为当前“复用/已核实”事实的错误；也未把脚本通过等同于设计正确。

## 4. 关键决策评估

| 选择 | 驱动需求与收益/代价 | 先进性与持续性判断 | 处置 |
| --- | --- | --- | --- |
| Rust 复用 `prepare`、编译场景和 `namespace_svg` 生成静态图 | 保持十一类输入、几何与 Reader 一致，避免浏览器导出和复制渲染器；代价是共享渲染上下文必须区分交互语义 | 方向可持续，正确复杂度在引擎拥有层 | 保留，按 F-03/F-04 补全静态上下文与样式归属 |
| 从 Reader 维护源派生静态 CSS | 避免双份样式漂移，保持 Reader HTML 字节不变；生成器需要覆盖真实选择器和全部样式来源 | 单一事实源方向先进，但当前覆盖不完整 | 保留方向，按 F-03 修订 |
| 内嵌 JetBrains 保真 | 有利于文字宽度和 Reader 图面一致；增加约 95 KB，并与系统字体调用方冲突 | 不能作为所有调用方的唯一策略 | 按 F-02 增加明确字体契约与分派 |
| 静态 summary 复用 `FinalizeReceipt` 和审核登记 | 保留产物身份、自动门与实际看图闭环，避免第二套审核协议 | 复用合理，接口错误语义也需适配新调用者 | 保留，按 F-05 修诊断 |
| `static:` 只认标记并原样内联 | 实现短，但把可伪造标记误作信任凭证 | 不符合终稿安全和零网络边界，不可持续 | 按 F-01 改为分派后安全解析 |
| 单图 4→2、架构比较 12→6 | 只减少截图采集与人工看图重复，十次测量、全部覆盖断言和登记门不变 | 与 PRD 的减量不放宽目标一致，成本与风险相称 | 保留，不追加截图或验收门 |
| M1–M3 同批、M4/M5 分批及 V-1–V-6 | 先形成可审核产物，再接入宿主和全类型回归；现有检查可承载上述修订 | 编排与真实依赖一致，无需新增里程碑 | 保留，只把报告指出的断言并入既有 V-1/V-2/V-3 |

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

**建议状态：Ready（完成 F-01–F-05 的计划修订与文本复核后）。** 当前计划虽自报 `Ready`，但两项 P1 会使终稿安全边界和 summarize 主场景无法按已确认要求交付，三项 P2 会在主题/打印、键盘阅读和审核恢复路径留下可证实缺口，因此当前文本不应直接进入实施。修后建议与原自报状态一致；没有必须等待用户或外部答案的阻塞项，故不建议 `Blocked`。Ready 不表示已实现、浏览器验收通过或门户上线可用。

修订顺序：

1. **先修安全与调用方契约**：处理 F-01 的静态 SVG 安全边界、F-02 的字体策略，更新 A-1/D4/D10/D12 及 §6，确保 summarize 原文和 PRD 范围不变。
2. **再修引擎设计与接口**：处理 F-03 的全部静态样式来源、F-04 的静态渲染上下文，回填 §3/§5、文件清单和 M1/M4 落点。
3. **再修恢复语义与验证映射**：处理 F-05，并把必要的少量断言并入既有 V-1/V-2/V-3，不新增里程碑或专项验收门。
4. **最后做文档一致性复核**：沿安全解析、字体分派、主题/打印、静态语义和 `review-record` 五条链路复核计划；保持 P3/P4/P5 边界、Reader HTML 零改动及 4→2、12→6 的减量决定不变。

下一步：`/apply-doc-review docs/plan/ELINTP-VISUALS-P3.review.md`，核实并修订计划；处置后判定 `Blocked` 或 `Ready`。本评审不自动改原计划或启动实施。

## 6. 修订处置与复核

- 来源：本报告原始 SHA-256 `f9ada269ae27917cde7feb7c25669301a57a71c55604c92df8341dffcefc5f56`；评审 ID `3d94f0f4-d8d2-49e2-abf1-601f909d6521`。原问题、原结论与所审基线完整保留，本节不是新一轮独立评审。
- 目标：[ELINTP-VISUALS-P3](ELINTP-VISUALS-P3.md)；修前 SHA-256 `9ca5325a5ff0042df863892d8843a3f3416175a71abc7caa72928391d6316f2d`，与报告所审版本一致，因原计划未跟踪，保留[所审基线快照（仅供追溯）](ELINTP-VISUALS-P3.review-baseline.md)，当前实施目标仍为前述计划；修后 SHA-256 `ba9a28f6950a742efe431390dc352be1a021d338eed3d8ecb75816fbb53e8f46`。
- 复核日期：2026-11-12；代码基线：main / `53d35750c97f865c5b739ee467eccd686c653ba7`，dirty。开始时已有 plan/review 索引和 PRD 状态页修改、P3 计划/评审及两份状态页未跟踪；未回退用户改动。对应代码只读；本次仅修计划、追加本节、保留所审基线快照并回写原对象的状态维度，成果未提交，仅当前工作树可取回。
- 本次范围：F-01–F-05 全部、影响修订的 SPMS 阅读面待核实项与任务/验证映射；无上轮处置。修前按标题和符号重新定位：报告提及的“§7.1”在目标中不存在，安全边界实际在 §6.1；所称“§3 接口表”实际在 §2；不按旧行号改稿。

### 逐项处置

| 问题 | 核实依据与修订前行为 | 最小修订及关联同步 | 当前处置 | 复核结果 / 适用范围 / 剩余动作 |
| --- | --- | --- | --- | --- |
| F-01 | 已读 `draft.mjs` 的 `inlineSVG`、figure 分派及 `render.test.js` 的现有安全用例；标记确实可伪造，新增原样拼接绕过当前主文档安全闸，与 WORK-N2/AC-1、AC-2 冲突 | D10、§0.4、§2/§4、§5.3、§6.1/§6.3、C-1、M4：标记仅分派，解析 SVG/CSS/URL 后安全序列化；限定静态元素、作用域、woff2 字体及品牌图片 MIME/位置，拒绝可执行内容与外链；保留引擎 ID，不重跑语义校验或审核 | 已修订并复核 | 已解决（文档层）。恶意伪标记明确由 `draft.mjs` 拒绝、退出 2 且不覆盖旧页；安全断言并入已有 V-4 的 elintp 用例，由 M4 运行。实现与运行验收待实施 |
| F-02 | 已读 summarize §3 的三项硬要求及 PRD S4、WORK-F5/AC-2、WORK-N3/AC-2；系统字体要求不能依赖门户剥离字体。`geometry.rs` 当前是等宽字宽估计，`prepare.rs` 校验的是 Reader HTML，不能冒称已验证系统字体 | D4/D6/D12、A-1、§0.5、§4、§5.1–§5.4、§6.3、M1/M2/M4、V-1/V-3/V-5：新增 `--font-policy embedded/system`，默认 embedded，summarize 明确选 system；文件记录策略，system 无 JetBrains、@font-face 与字体 data；实际产物下测文字边界并看图，两策略分别验证体积与完整性 | 已修订并复核 | 已解决（文档层）。文字要求 → elintp 分派 → 引擎策略 → CSS/字体 → 实际检查已逐跳映射；复用既有布局估计但不冒充真实字体测量。summarize 原文零改动；系统策略运行结果待实施 |
| F-03 | 已读 `render_svg_layers`、节点卡七类暗色规则、六类 `TypeRenderer.defs` 及 elintp 的 `page.html`/`page.css`；样式并非仅来自 viewer.css，无主题属性与暗色打印条件确实遗漏 | D2/D3、ADR-1/2、§1.2/§2、§5.2–§5.4、§5.6、M1/M2/M4、V-1/V-3/V-4/V-5：枚举全部来源，由同一 Rust 函数派生作用域/主题/媒体；动态场景样式锚定图根，公共规则限静态图；Reader 分支及维护源输出不变 | 已修订并复核 | 已解决（文档层）。独立打开、无主题系统亮/暗、显式暗色打印、多图同页与对照 SVG 均有检查；不只比较根背景。新增方案尚未运行，旧体积实验仅作探索线索 |
| F-04 | 已读普通 `write_nodes`、erd/class 共用 `node_attributes`、tree/timeline/waterfall/statechart 的生成点与图例；直接复用会留下无处理器的按钮与 Tab 停靠点 | D5、§1.2/§2、§5.3、M1、V-1/V-3/V-5：`RenderContext` 传至普通及类型渲染器的 root/defs/body/legend，在属性生成点省略 Tab、按钮、按压/展开状态与事件/动画钩子；保留绘制、标题/描述及根图像名称，不生成树控件 | 已修订并复核 | 已解决（文档层）。明确覆盖普通节点、六类类型节点和图例，禁用输出后正则删除/复制渲染器；elintp 滚动容器的键盘入口仍保留。产物语义待实施检查 |
| F-05 | 已读 `delivery/review.rs` 的 `review/automated-gates` / `review/artifact-changed` 两条消息及 delivery 既有失败用例，确实只提示 finalize/HTML | D7、§1.2/§4、§5.4、§8、M2、V-2/V-4：按 summary.command 分派消息；静态失败提示重跑 static，HTML 保持 finalize；原诊断码、身份/截图校验、记录结构不变 | 已修订并复核 | 已解决（文档层）。自动门失败、产物变化两条恢复路径均已映射到既有失败检查，不新增审核实现或里程碑；消息实测待实施 |
| §2 · SPMS 实际阅读面 | 本次未提交报告或检查门户；没有真实显示证据 | A-1 与 §4/§11 只保留门户显示待核实，不再假设接受 data 字体或靠字体剥离回退满足调用方 | 待核实（非阻塞） | 尚不能宣称 S4 已在真实门户可用；用户下一次实际 summarize 提交后核对，最晚在对外宣称 S4 实际可用前补证。本次本地实现路线不依赖此结果 |

### 建议适配与未完项

- 五项问题均成立，无不采纳项、无待用户决定的产品或范围取舍。原有 Rust 生成、Reader HTML 字节不变、4→2 / 12→6 截图减量、M1–M5 与 P3/P4/P5 边界保留，实施仍为 0/5。
- F-03 的实现建议作就地适配：CSS 继续从原维护源派生，但统一放在 Rust 样式拥有层；按场景生成的 `TypeRenderer.defs` 无法只靠构建时 viewer.css 转换覆盖。移除原计划的 static-figure.css 生成物与装配转换步骤，避免 JS/Rust 各维护一套策略；不改变产品承诺或授权方式。
- F-01 报告指向“V-2 既有安全用例”，实际 V-2 是截图审核场景，安全局部检查原本在 V-4。伪造标记的断言放回 V-4 现有 `test/elintp/render.test.js`，M4 批末执行；不增加新检查组或把尚未接入 elintp 的 M1–M3 批末设为安全用例完成点。接口表已同步此映射。
- 读取当前 `brands/capture.rs::validate_image` 后，资源白名单与其 PNG/JPEG/WebP/ICO 支持对齐，拒绝 SVG data 图片；不为引擎不产出的 SVG 品牌图片新增递归解析。字体仅 embedded 的 woff2 使用位置可通过。
- PRD 与 summarize 均只读且现有承诺已足够，无需另改上游。其余代码/skill/现行文档变更仍属于未来实施任务，本次没有执行。未新增完成记录、刷新历史测试或登记产品交付完成。

### 修订后结论与验证

**判定：Ready。** F-01–F-05 共 5 项已修订并通过文档复核；0 项不采纳、0 项待决、0 项未修订缺陷。真实 SPMS 阅读面 1 项非阻塞待核实。此结论只表示计划可实施，不代表实现、浏览器检查、门户验收或独立复审通过。

| 本次改变 / 复核目标 | 实际检查与结果 |
| --- | --- |
| 计划引用与未来路径的真假 | 已读脚本后运行 `skills/dev-plan/scripts/check_paths.sh docs/plan/ELINTP-VISUALS-P3.md /Users/rockie/Documents/gh-xgent/skills`：44 个 token、16 个 MISSING；逐项分类后未发现错误复用路径。新增为 `src/render/static_style.rs`、`src/browser/static_figure.rs`、skill 的 `references/static-figure.md`；输出目录为 `local/phoenix-diagram/elintp-p3-base`；其余 12 项为 §1.1 已声明的引擎/skill 相对路径，实际文件存在。脚本结果不是实现验收 |
| 修订不冒领实施进度 | `skills/dev-plan/scripts/check_progress.sh docs/plan/ELINTP-VISUALS-P3.md /Users/rockie/Documents/gh-xgent/skills`：0/5、定义 5 行、完成记录 0 行、ERROR 0、WARN 1。警告因计划未跟踪而无法识别其修改；人工对照修前快照确认无实施、无记录迁移或旧证据改写 |
| 五条契约无断链与相互矛盾 | 回读 D1–D12、ADR、职责、文件/接口/失败契约、§5/§6、V-1–V-6、M1–M5 及风险；搜索并移除旧原样内联信任、字体门户兜底、viewer-only 样式与未适配恢复指引，核对安全/字体/主题/语义/恢复均有设计、任务和验证落点 |
| 写入、来源与范围 | 对照修前文件检查完整 diff；原报告在本节前逐字保留；实际本地文档链接与状态页来源可达；进度协议内的 `FEATURE.records/M1.md` 是原有格式示例，不是已落盘记录，保持原样；只改计划、报告与其两个既有状态对象，另保存未经改写的所审基线快照，不覆盖原有索引/PRD 状态页改动 |

未运行 npm/Rust/浏览器/安装/发布检查：本次只修文档，未来实现的 V-1–V-6 尚未执行；没有代码与 skill 元数据变更，无须运行其测试。状态台账接续原评审的 disposition/report，计划只更新 readiness/plan，不产生新独立评审或重复事件。

下一步：`/goal 按 docs/plan/ELINTP-VISUALS-P3.md 开发`。本次不自动启动。
