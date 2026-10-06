# 固定状态与证据校准 · v2

本文件随状态协议保存到项目，不依赖安装 agi-mode。状态是基于当前可见证据的判断，不是从原文标题或状态行直接复制的事实。PRD、计划、评审、处置和实现记录仍在用户约定的位置。

## 状态字段

每条事件声明 `schema_version: 3`，`state` 从下面对应 `kind + facet` 的集合取值；原文自述另存 `source_state`（无状态文字用 null）。正文解释来源、版本、覆盖范围、判断依据及差异。各维度独立，不把整个项目强制压成一个阶段，也不要求补造中间事件。

- **state 取最强可支撑状态**：按下文各节的判定顺序，取原文记录与可核验物（记录文件、登记的提交、报告判定行、处置节、决策记录）支持的最强状态。之后相关文件又有改动时状态不降级，改由证据质量表达；只有完全没有可读依据时才用 `unknown`。
- `evidence_status` 表达依据质量与当前适用性，不是百分比置信度，固定为：
  - `confirmed`：原文记录与可核验物一致，覆盖该状态的判定条件；
  - `provisional`：只有原文自述（状态行、勾选、快照计数），缺少支撑记录；
  - `stale`：依据在其记录基线上成立，但之后相关文件另有改动未被覆盖，正文列出未覆盖的提交或文件及复核动作；
  - `conflict`：有效依据相互矛盾，state 取矛盾双方中较弱的一方，正文写明两处出处；
  - `unknown`：没有可读依据。
- `state: unknown` 只用于原文缺失、不可读或没有任何状态/进度信息；`candidate_state` 仅在此时记录线索指向的候选，必须属于对应状态集合。不用 unknown 或候选代替判断。
- **归档对象**：原文明确标为历史、废弃或已被取代的 PRD/计划，导航标归档，不套用下文判定顺序，也不给下一步。状态页保留最后登记的状态；首次登记写 `state: unknown`，`source_state` 保留原文标记，正文注明归档不判定。原文或用户明确重新启用后才恢复判定。
- 例：计划仍写“未开始”，但完成记录与实现证据覆盖全部里程碑 → `dev-plan-completed`、`source_state: 未开始`、`confirmed`，正文说明原文滞后。只有快照写 3/3、没有任何记录 → `dev-plan-completed`、`provisional`，下一步补证。完成后相关代码又有提交 → 仍是 `dev-plan-completed`、`stale`，列出这些提交。

## 开发线：文档就绪

| kind / facet | 固定 state | 判定条件 |
| --- | --- | --- |
| prd / document | `prd-drafted` | 已有 PRD，未发现可确认关联的评审，也尚未被计划承接；不得宣称其他机器也从未评审 |
| prd / document | `prd-reviewed` | 有针对该 PRD 的评审，判定需要修改且处置未完成 |
| prd / document | `prd-blocked` | 有当前有效的关键范围、产品语义、验收或决策阻塞；可在评审前或处理后出现 |
| prd / document | `prd-ready` | 当前文档可交接：最新评审无需修改或须改项已处置，关键阻塞已解除 |
| plan / readiness | `dev-plan-drafted` | 已有开发计划，未发现可确认关联的计划文档评审，且实施尚未开始 |
| plan / readiness | `dev-plan-reviewed` | 有计划文档评审，判定需要修改且处置未完成；不表示代码已评审 |
| plan / readiness | `dev-plan-blocked` | 当前开工条件存在已证实的关键阻塞，列解除条件，不限于 review applied 之后 |
| plan / readiness | `dev-plan-ready` | 设计与前置可执行，最新评审无需修改或须改项已处置，无关键开工阻塞；不是实施完成 |

判定顺序，取第一个成立的状态：

- **PRD**：文档有未解的阻塞问题 → `prd-blocked`；最新一轮评审判定无需修改，或处置记录显示须改项均已关闭 → `prd-ready`；有评审但须改项未处置完 → `prd-reviewed`；未发现评审但已有计划承接它（计划的需求来源指向该 PRD）→ `prd-ready`，证据 provisional 并注明未发现评审；其余 → `prd-drafted`。
- **计划**：计划状态行为 `Blocked`，或决策记录最新一轮结论为 Blocked → `dev-plan-blocked`；最新一轮计划评审判定无需修改，或处置/决策记录显示须改项与阻塞均已解除 → `dev-plan-ready`；有评审但须改项未处置完 → `dev-plan-reviewed`；未发现评审但实施已开始或完成 → `dev-plan-ready`，证据 provisional 并注明未发现评审；其余 → `dev-plan-drafted`。

最新评审或处置之后原文又有改动时，状态按评审与处置结果判定，证据记 stale 并说明改动范围；改动引入新阻塞时以阻塞为准。局部评审只代表其 scope，对象汇总说明其余覆盖缺口。项目或用户明确豁免评审时，引用豁免依据，在其他条件满足时判 ready；不伪造评审对象或 applied 历史。

## 开发线：计划实施

| kind / facet | 固定 state | 判定条件 |
| --- | --- | --- |
| plan / implementation | `dev-plan-not-started` | 可核实尚未开始；仅未发现代码或记录不能证明未开始 |
| plan / implementation | `dev-plan-in-progress` | 有实际实施证据，当前范围仍有必需工作未完成 |
| plan / implementation | `dev-plan-implementation-blocked` | 实施存在已证实阻塞；与设计未就绪的 dev-plan-blocked 分开 |
| plan / implementation | `dev-plan-completed` | 当前 scope 内全部必需交付及计划规定的验证门槛均有记录，未决缺口已解除或有明确范围变更依据 |
| plan / implementation | `unknown` | 计划没有可读的进度信息 |

判定顺序，取第一个成立的状态：

1. 恢复快照的当前阻塞非空，或完成记录有「阻塞」行 → `dev-plan-implementation-blocked`。
2. 完成记录中每个里程碑都是「已完成」，且与「当前进度」n/N 一致 → `dev-plan-completed`。计划有「提交」列时每行还须有提交 SHA，仍有「待提交」说明最终提交未做，按进行中。证据：每行的记录链接存在、登记的提交在当前分支可达 → confirmed；只有快照或行内自述、缺记录或 SHA → provisional；这些提交或记录之后相关文件又有改动 → stale。
3. 有「已完成」或「进行中」行、实施提交或工作树中的实施改动 → `dev-plan-in-progress`。
4. 快照为尚未开始、完成记录只有占位行，且当前分支与工作树都没有该计划的实施痕迹 → `dev-plan-not-started`；看不到完整历史时证据 provisional。
5. 没有可读的实施进度节，或快照与里程碑无法对应 → `unknown`。

scope 为 `plan` 才表示整份计划完成；M1 完成不代表 M2 完成。整体完成看完成记录的全部里程碑，不看末个里程碑打勾、代码目录存在、一次局部测试或有 code review 报告。计划实施完成可与某轮未处理的非阻塞评审并存；计划明确要求的评审门槛尚未满足时不能判 completed。

不强制经历 drafted → reviewed → ready → in-progress → completed：可首次导入已完成计划，也可因真实回归退回 in-progress/blocked，说明依据；不要虚构历史阶段。

## 每轮评审与处置

评审对象另存 `review_type: prd | dev-plan | code | other`，类型必须由报告内容确认。所有值均可按单条问题或整轮 scope 记录。

| facet | 固定 state | 判定条件 |
| --- | --- | --- |
| review | `review-recorded` | 报告确为已执行的评审，有目标、实际范围及结论/发现；缺基线时保留事实并说明当前适用性未知 |
| review | `unknown` | 只有文件名、空文件、模板、无对应目标或内容不足以证明发生过评审 |
| disposition | `review-pending` | 有明确待处理项且尚无处理证据 |
| disposition | `review-partially-applied` | 部分意见有处置，其他条目未关闭或修改后验证未完成 |
| disposition | `review-applied` | 本 scope 内各条意见均有可追踪的最终处置与必要复核；可包括有依据的不采纳/已解决无需改，不能把仍成立但延期的问题算关闭 |
| disposition | `review-apply-blocked` | 本 scope 的处置被明确依赖或决定阻塞，可另列已处理条目 |
| disposition | `review-not-required` | 报告完整、相关前提已核实且没有需处理意见；这是“无需 apply”，不是伪造运行过 apply |
| disposition | `unknown` | 报告不可读，或无法判断有没有须改项 |

处置判定顺序，取第一个成立的状态：报告判定无需修改或通过验收（旧报告：没有 findings 且结论为通过，如 `patch is correct`）→ `review-not-required`；处置节、applied 文档或实现记录显示本轮须改项均有最终处置 → `review-applied`；处置被明确的依赖或决定阻塞 → `review-apply-blocked`；部分须改项有最终处置 → `review-partially-applied`；有须改项但没有任何处置记录 → `review-pending`。

- 报告结论首行的判定（文档评审「是否需要修改」、代码评审「验收判定」）是该轮原结论，保存在来源/摘要。
- 处置之后目标又有改动：处置状态不变，证据记 stale；需要对新改动下结论时另起一轮评审。
- 用户豁免通过：豁免项按仍成立的延期问题处理，处置取 partially-applied 或 pending 并列出豁免项；下一步按通过处理，不再推荐 apply。

`review-recorded` 不代表通过。review-applied 也不自动使 PRD/计划 ready：原意见处置后可能仍有其他阻塞。多轮按评审 ID、目标版本及 scope 并列，当前路由看最新一轮；新一轮通过不能自动关闭前轮未覆盖问题。只处理 F-01 的 applied 不可提升成整轮 applied。

## 发现与自愈

1. **补发现**：先沿原文显式链接、项目约定和台账关系，再搜索同目录及项目评审目录。将 `<stem>.review.md`、`<原文件名>.review.md` 作为文档评审候选，将 `.codereview.md`、`.code-review.md` 及项目已有命名（含日期/轮次）作为代码评审候选；也读取报告内「修订处置与复核」、关联 `.applied.md`、实施记录中的问题处置。其他命名同样可以有效，不能仅按后缀筛掉真实记录。
2. **确认关联**：打开候选，核对目标路径/文档 ID、报告类型、所审版本与范围。明确指向原对象的实际报告可补 `reviews` 关系；版本缺失时仍可登记发生过评审，但当前适用性单列未知。仅同名不足，空模板、目标不明、文件找不到或未同步时保留候选与限制，不造关联或重复评审。改名通过内容、稳定 ID、Git 历史等确认后沿用对象 ID；同一路径的新一轮评审仍需独立 ID。
3. **发现处置**：核对逐项修改位置、最终处置和复核证据；不要求必须调用某个 skill 或存在独立 applied 文件。代码看似修复但缺少逐项确认/验证时只标待核实；记录只有部分关闭时保留剩余范围。无需处理的完整报告用 review-not-required，不为了凑流程推荐空跑 apply。
4. **纠正滞后**：计划状态行、复选框、状态页和事件都可能过期。对计划规定的每项必需交付/里程碑，核对实现记录、完成记录登记的提交、必要验证结果及未关闭事项；按相关版本和范围判断，不按文件更新时间选赢家。不默认重跑测试或整轮评审。记录覆盖全部门槛时在 SDLC 确认 completed，并指出原计划状态滞后；覆盖不足时按判定顺序取可支撑的状态（如 in-progress，或证据 provisional 的 completed）并列补证动作，不能只凭文案重开开发或反过来虚报完成。
5. **有限修复**：查询时在回答中重算、解释冲突与建议修复，不写文件。初始化/更新时自动补登记可确认的既有对象/评审、关系和必要索引；状态纠错追加事件并接续相关前序，刷新受影响视图，保留旧证据。只修派生视图时不需新造事实事件。原计划/报告改稿、执行 apply、测试或评审须在已有任务授权内，不从“自愈”推导这些授权。
6. **容错与幂等**：单个坏文件、缺来源、损坏 frontmatter、缺前序、断链或同 ID 冲突标记到对应对象，继续盘点其他计划；保留损坏原文件，不覆盖或静默丢弃。来源缺失不等于未开始，哈希存在不等于内容可取回。新证据/关联、固定状态、证据质量或适用范围实质改变才追加记录；重复更新不因时间/无关 HEAD 改变重记事件。修复失败保留成功的事件并报告待修复范围。

## 验收线与扩展

独立验收线的固定状态暂为 **TBD**，不虚构 passed、accepted、released 等默认阶段。已有验收/集成/发布事实可保留来源与范围；没有项目已登记枚举时新 v3 事件使用 `state: unknown` 并在 `source_state` 保留原结论，明确“未定义状态映射”，不是判定验收失败。已有项目枚举沿用其版本，不抹除历史。开发完成不自动说明产品验收、集成或发布完成。

自定义 kind/facet 或新增状态必须先在项目的本文件登记含义、证据条件和版本，再写事件；不能每个 agent 临时造同义词。

## 旧记录兼容

无 schema_version 的旧事件按原协议解释，不将其自由文本当 v3 枚举，也不回写修改。维护任务明确包含协议升级时保留原规则的兼容说明，新增本文件并升级协议/模板；有证据才追加 v3 规范化事件，接续同对象、维度和 scope 的旧前序。原文字段保留在 source_state，无可靠映射用 unknown。只读盘点可在回答中给固定状态映射，不静默升级仓内文件；历史事件/对象 ID 不变。

**v1 → v2**：v1 要求 ready、completed、applied 的依据必须 confirmed，否则记 unknown 加候选；v2 改为取最强可支撑状态，证据质量单独标注。状态集合、事件字段与 `schema_version` 不变。

- 仓内副本与 v1 原样一致时（`state-model.md` 的 SHA-256 为 `35a5abdf71d59bd1b4de16b09c6efcb8f127bd08e9e7af413599a5f90ffe772e`，`event-template.md` 为 `d5a418d1e47d53a92e7bce50636c642c4fb142a42cc1cb1e9aaddd3b7f795cf6`），初始化或更新直接替换为新版，并在台账导航登记升级时间与版本。副本有项目改动时保留，列出差异请用户决定，不覆盖。
- v1 事件不改写。升级后首次核对按 v2 重判，状态或证据质量有变化时追加接续事件（`predecessors` 指向原事件），正文注明口径升级；无变化不追加。查询模式按 v2 在回答中给状态，并说明台账仍是旧口径。
