# ELINTP-VISUALS-P4 · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [ELINTP-VISUALS-P4 · 数值图表（Phoenix Diagram 新增 chart 图型）](ELINTP-VISUALS-P4.md) | SHA-256 `5955236687f108e2635983a91dd77f567657ed502b75a531dbdb8f3340b9003b` | 全文；PRD P4（WORK-F6/AC-1–6、WORK-F5/AC-3、WORK-F9/AC-5、WORK-N3/AC-4、C-1–C-11）、D1–D15、ADR-1–3、M1–M5、V-1–V-6 |

## 1. 确认问题

1. [P2] F-01 · §5.1 字段表 `rows[].unit` 未标「可省」，与 `meta.unit` 缺省语义自相矛盾
   - 定位：[计划 §5.1 输入契约表](ELINTP-VISUALS-P4.md#L417-L425)（L417 `meta.unit` 行、L425 `rows[].unit` 行）；关联 [§5.2 诊断表 L438](ELINTP-VISUALS-P4.md#L438)、[§5.10 `chart.md` 文字 L615](ELINTP-VISUALS-P4.md#L615)、D2、M1、V-2、WORK-F6/AC-1。
   - 问题：§5.1 字段表是编写 `chart.schema.json` 的规范来源。`rows[].unit` 的约束列只写「非空字符串」，而同表 `meta.unit`（L417）的含义列明确写「限额行没写 `unit` 时的缺省」，§5.2 的 `chart/form-fields` 对 limits 行缺 `unit` 不报错，§5.10 `chart.md` 也写「限额可逐行写 `unit`」——三处均要求行级单位可选。照字段表机械实施会把 `unit` 写进 limits 行的 `required`，只写 `meta.unit` 的合法 candidate 在输入闸被拒，`meta.unit` 的缺省分支成为死代码。V-2 的限额 candidate 明确带「逐行单位」，验收发现不了该偏差。
   - 证据：计划 L417/L425/L438/L615 原文互斥；同表其余可选字段（`orientation`、`series`、`rows[].value`）均按表格惯例显式标注「可省」；[`waterfall.schema.json`](../../skills/phoenix-diagram/schemas/waterfall.schema.json) 的可选字段同样显式标注。
   - 修订：§5.1 `rows[].unit` 的约束列补「可省」，与 L417、§5.2、§5.10 三处既有口径一致。一行改动，不影响其余正文、账本与检查。
   - 复核：改稿后核对 §5.1 表内两行语义一致，且与 §5.2、§5.10 不再互斥。本期 V-2 对「只写 `meta.unit` 的限额图」没有任何检查能发现该偏差，故在 V-2 既有限额 candidate 的描述中把其单位写法改为「其中一行省略行级 `unit`、只依赖 `meta.unit`」，不另建检查。
   - 置信度：高；四处原文与同构 schema 惯例均已核对。阻塞：否。

2. [P3] F-02 · §5.10 对 `schemas/README.md` 的行号锚点只覆盖三处改动之一
   - 定位：[计划 §5.10 L618](ELINTP-VISUALS-P4.md#L618)；关联 M3、C-4、V-5 第 4 步。
   - 问题：计划把三处编辑（类型清单加 `chart`、「其余九类为 v1」改为十类、语义门句加 `chart/*`）全部锚到该文件 L3；实际分别位于 L3（类型清单「十一类」）、L5（「其余九类为 v1」）与 L7（语义门句）。按单行定位时实施者需自行另寻两处。
   - 证据：[`skills/phoenix-diagram/schemas/README.md`](../../skills/phoenix-diagram/schemas/README.md) L3/L5/L7 实文；三句表述唯一，能立即找到。
   - 修订：锚点改为「L3、L5、L7」，或去掉行号只留文件名与三处原句。
   - 复核：改稿后按锚点逐行可命中三处目标文字。
   - 置信度：高；阻塞：否。

## 2. 待核实 / 待决策

无待用户或外部决策项。以下为实施期验证，计划已排期，不是评审缺口：

- ADR-2 行带作节点框与通用检查、浏览器探针的运行期相容性只经代码阅读确认；第一道真实验证是 M1 的 V-1（计划 §11 首行已如实登记该风险与偏差处理路径）。最晚确认点：M1 批末。
- §9.1 指纹 one-liner 在真实阅读器 HTML 上的取段正确性（首个 `<svg xmlns=…viewBox=` 是否恒为主图）未实跑；M1 记录基线时一步即可校准。最晚确认点：M1 基线。
- 参考工具（`local/answer-me-with-html-main`）侧的形态与错误语义未核对；本期 D12/C-11 只借鉴行为、不复制代码，仓内交付不依赖其实现。

## 3. 覆盖摘要

- 计划与代码基线：计划 749 行（SHA-256 见评审对象）；仓库 HEAD `3889a9ae16396d710f637583721b12a73d3eb897`（2026-11-14），工作树 dirty 仅 use-llm-gateway skill、`.xgent-ai` 台账与 `docs/plan` 的 P3 评审文档；`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 相对 HEAD 无改动，代码事实按当前工作树核对。
- 实际分工：两位只读 subagent 并行独立评审（`facts` / `architecture`），主 agent 做 C 节硬检查、交叉裁定与脚本检查；报告由主 agent 单写。
- facts 覆盖：§1.1 全部「已核实·足够/缺口」条目逐条核到行为级（`DiagramType::ALL` 长 11、`input/unsupported-type`、`tick_step` 对无穷差 panic、通用检查行号组、`TypeRenderer` 四钩子与 cards 插入点、`scope_selector` 丢弃逻辑、`write_common` 只写卡片 metadata、elintp `draft.mjs` 边界、`typed_identity` 与通用 unique 的关系、探针 `:scope` 选择器）；§5.9 登记点对照 statechart 提交 `6bb21b6b` 逐项吻合；测试/脚本/文档行号锚点逐行命中；数字与跨文档引用（PRD §1.1 的 4,893/612、P3 计划 ADR-1 的 967–989 KB/157–177 KB、`13435a65` 后两目录零提交、STATECHART/P3 记录、V-6 安装命令与选项）全部核到原文；schema↔语义校验↔渲染↔检查↔数据表↔elintp 解析的内部一致性交叉比对，除 F-01/F-02 外无互斥陈述。
- architecture 覆盖：ADR-1/2/3 与 D1–D15 逐项评估（见 §4）；§5.3–5.8 编码就绪度（签名、结构体、常量、错误码、路径俱全，无「实施时确定」）；§8 失败模式与回滚；§10 编排（PRD §7.1 建议 P4 为 5–8 个里程碑，计划 5 个，均在建议区间且每步有独立退出条件）；C-9 六条路径无遗漏；`@plan_proportionality` 双向核对未见无真实触发的防御、兜底或验收门（五语言文案、M1 基线与 V-5 指纹比对均有仓库机制或验收行背书），也未见该做而被砍的正确工作。
- C 节硬检查（主 agent）：范围越界——无（§1.2 全部落在声明的开发范围内，范围外依赖无缺口）；发布与评审——无（§8 明确本计划不发布，里程碑无代码评审，收口后仅提示用户发起）；需求落点——账本全部 ID 有设计/检查落点，延期项有 P5 去向；交付断点——无（入口 `guide`、安装路径 V-6、新引擎+旧 elintp 降级 §8、宿主数据契约文档化 §5.10 齐备）。
- `check_paths.sh`：实际运行，72 个路径 token、12 个 MISSING；人工分类——9 个为本计划新增（★）文件，1 个为 M1 构建输出目录（`local/phoenix-diagram/elintp-p4-base`），1 个为 gitignored 构建产物（`dist/phoenix-diagram/bundle/ENGINE.json`，V-5/V-6 时随构建生成，计划 §11 已登记），1 个为 §4 中 skill 相对名（`phoenix-diagram/SKILL.md`，仓内实际路径为 `skills/phoenix-diagram/SKILL.md`）。无「复用/已核实」类路径缺失。

## 4. 关键决策评估

| 关键选择 | 驱动需求 | 结论 |
| --- | --- | --- |
| ADR-1 一个 `chart` 图型 + `form` 枚举 | WORK-F6/AC-6 同入口交付、显式拒绝未知形态；登记点约 20 处/图型 | 保留。schema 管形状、语义层管形态×字段搭配是 JSON Schema 的标准分层而非补丁；备选 B/C 只增登记成本无契约收益，淘汰理由成立。 |
| ADR-2 一行一节点、行带为节点框 | 通用检查「每节点恰一节点框、间距 ≥ 8」、探针标签包含、搜索/详情/聚焦按节点 | 保留。不是绕开通用检查（备选 C 被明确拒绝），而是让 chart 适配引擎节点契约；备选 B 的死穴（同组柱间距、折线近值点判重叠）经几何推导成立。运行期风险以 M1 V-1 最薄链路先行验证，偏差处理路径已定义（调行带或修订 ADR-2，不为 chart 豁免检查）。 |
| ADR-3 阅读器 HTML `<details>` 表 + 静态 SVG `<metadata data-phoenix-table>` 由宿主渲染 | WORK-F6/AC-4 可展开可复制；WORK-F9/AC-5 体积 ≤ 50%；C-1 | 保留。与 P3 卡片数据「引擎交数据、宿主渲染」同一架构方向；把表格画进 SVG 会破坏体积约束且不可收起，是更差做法。新引擎+旧 elintp 拒收是诚实失败语义（退出 2、不覆盖旧页），非兼容层。 |
| D5 全数值标注 + 画布加宽，读不清由既有检查失败 | WORK-F6/AC-1、C-9、C-10 | 保留。让约束显式失败而非静默抽稀或隐藏配额；稠密序列失败是 PRD D10 既定后果，修法写入 `chart.md`。 |
| D6 系列键 A/B…（非颜色区分） | WORK-F6/AC-2、PRD §2.4 | 保留。任意系列数都有非颜色区分，不受色槽/线型种类限制；标注变长已计入宽度估计。 |
| D7 `js_to_string` 原样数值；派生量可取整并标「占比」 | WORK-F6/AC-1 | 保留。与引擎既有数值写法单一归属一致。 |
| D8 复用 `tick_step` 并加第三参数（waterfall 传原值 80） | WORK-F6/AC-2；折线不强制含 0 | 保留。对共享模块的最小正确扩展，不是外围复制刻度逻辑；`tick_step` 对无穷差会 panic（已核到 `expect` 语句），计划把有限性判断放在调用之前是正确且必要的。 |
| D10/D14 静态表格数据格式与「十一类不变」判定口径 | WORK-F9/AC-5、WORK-N3/AC-4 | 保留。判定精确到字节级可比（主图 SVG + 内嵌元数据 + 静态 SVG），模板整文件允许变化与 statechart 先例一致；`.chart-data` 规则经 `scope_selector` 不进静态图（已核实），静态 SVG 字节不变有代码依据。 |
| D15 elintp 选图规则只改文字与对照表 | WORK-F5/AC-3、C-2 | 保留。遵守 `@skill_tool_independence`：elintp 不以引擎版本为前提，无 `chart` 时走既有降级。 |
| §10 编排：M1 最薄链路 → M2–M3 同批 → M4 可并行 → M5 收口 | PRD §7.1 建议 5–8 个里程碑 | 保留。M1 先烧最大架构风险；M4 与引擎无共享文件（已核对落点清单）；提交点与回滚闭环。 |
| §5.6 内容检查从产物反推线性比例 | C-9「检查」路径、AC-1/2/4 | 保留。waterfall 已有同构先例（反推比例、容差核对）；深度与既有图型一致，非加码。 |

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

- 建议状态：**Ready**（应用 F-01 后；F-02 顺手采纳）。无 P0/P1、无阻塞项、无待用户决策；两项修订均为一行级文档改动，不影响账本、里程碑与检查编排。
- 与自报状态差异：计划自报 Ready；本评审不推翻该判断，但 F-01 属确认的契约文本矛盾，须先修订再开工，故当前记 `dev-plan-reviewed`，处置后转 Ready。
- 修订顺序：① §5.1 `rows[].unit` 约束列补「可省」（F-01），同步把 V-2 限额 candidate 的单位写法改为「其中一行只依赖 `meta.unit`」；② §5.10 `schemas/README.md` 锚点改「L3、L5、L7」（F-02）。两处均为就地修订，无需重排其他正文。

## 下一步

- 核实并修订本报告所列问题：`/apply-doc-review docs/plan/ELINTP-VISUALS-P4.review.md`（Codex 客户端：`$apply-doc-review docs/plan/ELINTP-VISUALS-P4.review.md`）。处置完 F-01 后判定 Ready，即可 `/goal 按 docs/plan/ELINTP-VISUALS-P4.md 开发`。

## 修订处置与复核

- 来源：本报告原始 SHA-256 `10f00a7cfd21ffc3ac49b3c48ba0a7b54ca33e1648d4bc0335d328e7b1ff37ff`；评审 ID `79c4f547-0cf5-40a1-8d08-4952084ee1ea`。以上原问题、所审基线与总体结论保留。
- 目标：[ELINTP-VISUALS-P4 开发计划](ELINTP-VISUALS-P4.md)；修前 SHA-256 `5955236687f108e2635983a91dd77f567657ed502b75a531dbdb8f3340b9003b`，与报告所审版本一致；修后 SHA-256 `b4d6f75c8ecafe6caa5852d2964b39e3897e79e2723ef45ca0e5f704aee0390f`。
- 复核日期与代码基线：2026-11-14（Asia/Shanghai）；main / `3889a9ae16396d710f637583721b12a73d3eb897`，工作树 dirty。修前计划与报告均为未跟踪文件；其余已有改动涉及 use-llm-gateway、状态台账、经验记录及 P3 评审材料，均未覆盖。`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 相对 HEAD 无改动。
- 本次范围：F-01、F-02 全部核实并就地修订，同时核对 §2 的待验证事项与就绪判断；这是文档修订复核，不是新一轮独立评审或实施验收。

| 问题/报告章节 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。亲自核对计划 §5.1：`meta.unit` 已定义限额行缺省单位，`rows[].unit` 却未注明可省；§5.2 不拒绝限额行缺 `unit`，§5.10 明确可逐行写单位。PRD WORK-F6/AC-1、AC-4 要求图面与数据表忠实呈现数值及单位，没有要求每行重复单位。 | 已修订并复核 | §5.1 L425 补「可省」；V-2 L704 的既有限额 candidate 改为一行省略行级 `unit`、依赖 `meta.unit`，其余行保留行级单位，并明确缺省单位须在图面与数据表正确显示。未另建检查。 | **已解决（文档层面）**。字段表、语义门与创作文字一致，既有 V-2 同时覆盖缺省与逐行单位。真实输入校验与产物验收仍在 M2–M3 批末执行。 |
| F-02 | 成立。亲自读取 `skills/phoenix-diagram/schemas/README.md`：类型清单位于 L3、版本句位于 L5、语义门句位于 L7，原计划只列 L3。 | 已修订并复核 | §5.10 L618 的锚点改为 L3、L5、L7；三项编辑内容不变。 | **已解决（文档层面）**。按补齐后的行号可分别命中三处目标，M3 与 V-5 第 4 步无需调整。 |
| §2 待核实 / 待决策 | 没有待用户或外部决策。ADR-2 的运行期相容性、§9.1 指纹命令的真实取段效果属于已安排的 M1 验证；参考工具实现不是本期交付依赖。 | 已解决无需修改 | 沿用 §9.1、M1、§11 的验证与偏差处理路径，不改写为已验证。 | 无新增开工阻塞；这些实施期检查本次未运行。 |

### 决策与未完项

- 两项建议均采纳，无不采纳项、待用户选择或未落实的文档修订；没有新增产品承诺、范围或架构取舍。
- 本次没有修改 PRD、skill、引擎、实施记录或外部系统；没有需要同步修订的关联文档。计划仍为 0/5，实施验收、构建与浏览器检查均未执行。
- 原始报告完整保留在本节之前；修前计划可由修后文件仅逆转 L425、L618、L704 三行修订还原，并以修前 hash 校验。计划、报告及台账均未提交，尚未共享到其他工作树。

### 修订后结论与验证

- F-01、F-02：**2 项已修订并复核，0 项未采纳，0 项待决或待修订**。本次仅关闭确认的文档问题，不声称重新完成全文独立评审。
- 实际运行 `bash skills/dev-plan/scripts/check_paths.sh docs/plan/ELINTP-VISUALS-P4.md .`：72 个路径 token、12 个 MISSING。人工分类为 7 个计划新增文件及其中 2 个 skill 相对写法、1 个 M1 基线构建目录、1 个随构建生成的 `dist/phoenix-diagram/bundle/ENGINE.json`、1 个既有 skill 相对名 `phoenix-diagram/SKILL.md`；没有错误复用路径。
- 实际运行 `bash skills/dev-plan/scripts/check_progress.sh docs/plan/ELINTP-VISUALS-P4.md .`：0/5，5 个里程碑、0 条实施记录，ERROR 0、WARN 1。警告称工作树有改动而计划未改，是脚本对未跟踪计划的提示；该文件原先即未跟踪，当前只有三行文档修订，相关实现目录无 diff，未发现完成事实漏记。
- 人工回读 §5.1、§5.2、§5.10 与 V-2，核对 F-01 的单位缺省口径和可观察验收结果；对照 schema README 的 L3、L5、L7 核验 F-02；比对修前/修后内容确认仅三行受影响，未改里程碑、需求映射或进度。路径、版本与本节新增本地链接均已核对。
- **当前判定：Ready。** 评审意见已全部处置，没有关键开工阻塞；保留原计划 Ready，台账 readiness 更新为 `dev-plan-ready`，原评审对象的 disposition 更新为 `review-applied`，实施状态不变。
- 下一步（仅建议，不自动启动）：`/goal 按 docs/plan/ELINTP-VISUALS-P4.md 开发`。
