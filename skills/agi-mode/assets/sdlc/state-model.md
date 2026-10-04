# 固定状态与证据校准 · v1

本文件随状态协议保存到项目，不依赖安装 agi-mode。状态是基于当前可见证据的判断，不是从原文标题或状态行直接复制的事实。PRD、计划、评审、处置和实现记录仍在用户约定的位置。

## 状态字段

每条事件声明 `schema_version: 3`，`state` 从下面对应 `kind + facet` 的集合取值；原文自述另存 `source_state`（无状态文字用 null）。正文解释来源、版本、覆盖范围、判断依据及差异。各维度独立，不把整个项目强制压成一个阶段，也不要求补造中间事件。

- `evidence_status` 固定为 `confirmed`（该范围可核实）、`provisional`（仅有线索或覆盖不足）、`stale`（旧证据不适用于当前相关版本）、`conflict`（有效证据矛盾未解）、`unknown`（无可用依据）。这是判断依据的质量，不是百分比置信度。
- 无法确定当前状态用 `state: unknown`；可选 `candidate_state` 记录待验证候选，仍必须属于对应状态集合。旧的确定状态保留在历史事件，不作为当前默认答案。`ready`、`completed`、`applied` 只有依据达到 `confirmed` 才能作为当前肯定结论，否则用 unknown 与候选。
- 例如计划仍写“未开始”，但完整实施证据已核实，可记录 `state: dev-plan-completed`、`source_state: 未开始`、`evidence_status: confirmed`，正文说明原状态滞后及可追溯依据。自报完成但缺证则写 unknown / provisional，候选为 dev-plan-completed。

## 开发线：文档就绪

| kind / facet | 固定 state | 判定条件 |
| --- | --- | --- |
| prd / document | `prd-drafted` | 已有 PRD，当前可见范围内尚未发现可确认关联的评审；不得宣称其他机器也从未评审 |
| prd / document | `prd-reviewed` | 确认存在针对该 PRD 的评审，尚无充分证据满足 ready；分别列出各轮处置，不把“没找到 applied”说成“确定未处理” |
| prd / document | `prd-blocked` | 有当前有效的关键范围、产品语义、验收或决策阻塞；可在评审前或处理后出现 |
| prd / document | `prd-ready` | 当前文档可交接，所需评审意见已核实处置或确认无需处理，关键阻塞已解除 |
| plan / readiness | `dev-plan-drafted` | 已有开发计划，当前可见范围内尚未发现可确认关联的计划文档评审 |
| plan / readiness | `dev-plan-reviewed` | 确认存在计划文档评审，尚无充分证据满足 ready；不表示代码已评审 |
| plan / readiness | `dev-plan-blocked` | 当前开工条件存在已证实的关键阻塞，列解除条件，不限于 review applied 之后 |
| plan / readiness | `dev-plan-ready` | 设计与前置可执行，所需评审意见已核实处置或确认无需处理，无关键开工阻塞；不是实施完成 |

上述两类也允许 `unknown`。判断时先核对当前版本及范围；有已证实阻塞用 blocked，满足就绪条件用 ready，只有评审事实则 reviewed，仅有原文则 drafted。仅有过期评审时可保留 reviewed 这一历史发生事实，但标 stale 并说明当前需复核，不能因此断言当前 ready。局部评审的 reviewed 只代表该 scope；对象汇总必须说明其余覆盖缺口。

项目明确不需要某项评审时，引用项目规则或用户决定作为豁免依据，可在其他就绪条件满足时判 ready；不伪造评审对象或 applied 历史。没有项目豁免时，不仅凭原文自报 Ready 跳过评审/处置核对。

## 开发线：计划实施

| kind / facet | 固定 state | 判定条件 |
| --- | --- | --- |
| plan / implementation | `dev-plan-not-started` | 可核实尚未开始；仅未发现代码或记录不能证明未开始 |
| plan / implementation | `dev-plan-in-progress` | 有实际实施证据，当前范围仍有必需工作未完成 |
| plan / implementation | `dev-plan-implementation-blocked` | 实施存在已证实阻塞；与设计未就绪的 dev-plan-blocked 分开 |
| plan / implementation | `dev-plan-completed` | 当前 scope 内全部必需交付及计划规定的验证门槛均有适用证据，未决缺口已解除或有明确范围变更依据 |
| plan / implementation | `unknown` | 实施进度不可核实、证据过期或相互矛盾，保留候选及缺什么 |

scope 为 `plan` 才表示整份计划完成；M1 完成不代表 M2 完成。不得仅因末个里程碑打勾、代码目录存在、一次局部测试成功、有 code review 报告或所有计划自报完成而判整体交付。计划实施完成可与某轮未处理的非阻塞评审并存；计划明确要求的评审门槛尚未满足时不能判 completed。

不强制经历 drafted → reviewed → ready → in-progress → completed：可首次导入已完成计划，也可因真实回归退回 in-progress/blocked，说明依据；不要虚构历史阶段。

## 每轮评审与处置

评审对象另存 `review_type: prd | dev-plan | code | other`，类型必须由报告内容确认。所有值均可按单条问题或整轮 scope 记录。

| facet | 固定 state | 判定条件 |
| --- | --- | --- |
| review | `review-recorded` | 报告确为已执行的评审，有目标、实际范围及结论/发现；缺基线时保留事实并说明当前适用性未知 |
| review | `unknown` | 只有文件名、空文件、模板、无对应目标或内容不足以证明发生过评审 |
| disposition | `review-pending` | 有明确待处理项且尚无处理证据；“没找到记录”但是否处理不可知则用 unknown |
| disposition | `review-partially-applied` | 部分意见有处置，其他条目未关闭或修改后验证未完成 |
| disposition | `review-applied` | 本 scope 内各条意见均有可追踪的最终处置与必要复核；可包括有依据的不采纳/已解决无需改，不能把仍成立但延期的问题算关闭 |
| disposition | `review-apply-blocked` | 本 scope 的处置被明确依赖或决定阻塞，可另列已处理条目 |
| disposition | `review-not-required` | 报告完整、相关前提已核实且没有需处理意见；这是“无需 apply”，不是伪造运行过 apply |
| disposition | `unknown` | 缺失、过期或冲突的处置证据，无法确认处理状态 |

`review-recorded` 不代表通过。报告原结论保留在来源/摘要中，不将风险高低硬映射成通过。review-applied 也不自动使 PRD/计划 ready：原意见处置后可能仍有其他阻塞。多轮按评审 ID、目标版本及 scope 并列；新一轮通过不能自动关闭前轮未覆盖问题。只处理 F-01 的 applied 不可提升成整轮 applied。

## 发现与自愈

1. **补发现**：先沿原文显式链接、项目约定和台账关系，再搜索同目录及项目评审目录。将 `<stem>.review.md`、`<原文件名>.review.md` 作为文档评审候选，将 `.codereview.md`、`.code-review.md` 及项目已有命名（含日期/轮次）作为代码评审候选；也读取报告内「修订处置与复核」、关联 `.applied.md`、实施记录中的问题处置。其他命名同样可以有效，不能仅按后缀筛掉真实记录。
2. **确认关联**：打开候选，核对目标路径/文档 ID、报告类型、所审版本与范围。明确指向原对象的实际报告可补 `reviews` 关系；版本缺失时仍可登记发生过评审，但当前适用性单列未知。仅同名不足，空模板、目标不明、文件找不到或未同步时保留候选与限制，不造关联或重复评审。改名通过内容、稳定 ID、Git 历史等确认后沿用对象 ID；同一路径的新一轮评审仍需独立 ID。
3. **发现处置**：核对逐项修改位置、最终处置和复核证据；不要求必须调用某个 skill 或存在独立 applied 文件。代码看似修复但缺少逐项确认/验证时只标待核实；记录只有部分关闭时保留剩余范围。无需处理的完整报告用 review-not-required，不为了凑流程推荐空跑 apply。
4. **纠正滞后**：计划状态行、复选框、状态页和事件都可能过期。对计划规定的每项必需交付/里程碑，核对实现记录、实际代码/提交、必要验证结果及未关闭事项；按相关版本和范围判断，不按文件更新时间选赢家。不默认重跑测试或整轮评审。已有证据足以覆盖全部门槛时可在 SDLC 确认 completed，并指出原计划状态滞后；覆盖不足用 unknown + candidate_state，列补证动作，不能只凭文案重开开发或反过来虚报完成。
5. **有限修复**：查询时在回答中重算、解释冲突与建议修复，不写文件。初始化/更新时自动补登记可确认的既有对象/评审、关系和必要索引；状态纠错追加事件并接续相关前序，刷新受影响视图，保留旧证据。只修派生视图时不需新造事实事件。原计划/报告改稿、执行 apply、测试或评审须在已有任务授权内，不从“自愈”推导这些授权。
6. **容错与幂等**：单个坏文件、缺来源、损坏 frontmatter、缺前序、断链或同 ID 冲突标记到对应对象，继续盘点其他计划；保留损坏原文件，不覆盖或静默丢弃。来源缺失不等于未开始，哈希存在不等于内容可取回。新证据/关联、固定状态、证据质量或适用范围实质改变才追加记录；重复更新不因时间/无关 HEAD 改变重记事件。修复失败保留成功的事件并报告待修复范围。

## 验收线与扩展

独立验收线的固定状态暂为 **TBD**，不虚构 passed、accepted、released 等默认阶段。已有验收/集成/发布事实可保留来源与范围；没有项目已登记枚举时新 v3 事件使用 `state: unknown` 并在 `source_state` 保留原结论，明确“未定义状态映射”，不是判定验收失败。已有项目枚举沿用其版本，不抹除历史。开发完成不自动说明产品验收、集成或发布完成。

自定义 kind/facet 或新增状态必须先在项目的本文件登记含义、证据条件和版本，再写事件；不能每个 agent 临时造同义词。

## 旧记录兼容

无 schema_version 的旧事件按原协议解释，不将其自由文本当 v3 枚举，也不回写修改。维护任务明确包含协议升级时保留原规则的兼容说明，新增本文件并升级协议/模板；有证据才追加 v3 规范化事件，接续同对象、维度和 scope 的旧前序。原文字段保留在 source_state，无可靠映射用 unknown。只读盘点可在回答中给固定状态映射，不静默升级仓内文件；历史事件/对象 ID 不变。
