---
name: review-prd
argument-hint: "<PRD路径/链接/内容> [上轮报告] [复审/完整评审] [问题ID/范围] [报告路径] [仅聊天]"
description: 用于评审既有 PRD 的合理性、产品质量与可验收性；不用于从零写 PRD、评审技术计划或代码 diff。核实代码现状、需求、体验、UAT 与分期，默认落盘报告。
disable-model-invocation: true
---

# review-prd · 基于代码事实的产品需求评审

```formal
INTENT:
  GOAL: "判断 PRD 在用户约束内是否充分设计值得做、可用且体验好的产品，能否交接做什么/怎样算完成，以及代码现状与前置认知是否真实"
  SUCCESS: ["错误、缺口及有价值的创新/AI 遗漏均按证据裁定，不只审格式",
            four_review_lenses_covered, complete_report_with_verified_delivery]

RULE @prd_default_delivery:
  UNLESS: explicit_chat_only_or_no_file_request
  MUST: "将完整报告写入文件并回读核验，聊天给摘要与实际链接；落盘属于评审，不另问是否保存"
  MUST_NOT: claim_completion_before_report_is_saved
```

`references/` 相对本文件解析。仓库规则、产品上下文、代码位置、工具和外部依赖在运行时读取，不硬编码目标仓的业务约定。回复语言跟随用户。

## 边界与参考

```formal
RULE @prd_review_scope:
  MUST: ["默认审整份 PRD；指定差异/局部时追踪受影响的场景、需求、验收与分期，默认只写独立报告及已启用台账的事实索引",
         =>[explicit_prd_revision_requested, "先形成可追踪问题，再统一修订并复核；报告留处置记录，PRD 正文只留修订后的产品结论"]]
  MUST_NOT: ["未经明确修订要求改写 PRD", modify_code_or_implement_features,
             automatic_spms_writes_or_external_comments]

RULE @prd_claim_evidence:
  MUST: ["区分现状事实、目标能力、用户/平台约束、未证实假设、验收门槛及观察/PoC 基线",
         "指出真实增量与前置，不把当前实现固化为需求"]
  MUST_NOT: ["因新能力尚未实现报缺陷", "因代码当前行为禁止产品演进"]

RULE @prd_product_boundary:
  MUST: ["审 what / why / 怎样算完成，以及旅程、信息层级、视觉与 AI 产品行为；内部架构、表结构、端点、模型/算法选型、技术里程碑交开发计划",
         "UAT 定角色/场景/预期状态/判据，测试脚本、造数、故障注入、工具和 TC 创建交测试计划",
         "保留已给定技术约束、外部产品契约与作为产品能力的评测台，不按技术词机械判越界"]
```

- 在可用时读取 `prd/references/product-quality.md` 校准产品质量规则；采用该模板的文档再按需读 `prd/references/prd-skeleton.md`。找不到 `prd` 时继续按本技能的实质标准评审，不要求安装依赖。用户自定义模板或历史 PRD 不因章节名、编号、缺少 TC 或未录入 SPMS 被判不合格。
- UI 评审读取目标仓实际适用的产品/设计规范；只有能看到的材料才支持视觉结论。源码、静态稿、原型与浏览器实测分别标注，不把静态推演写成已验证体验。

## 1. 锚定输入与审查基线

开始时读取 [报告合同](references/report-contract.md)，按其落盘、引用及模式约定执行。

```formal
WORKFLOW @prd_review_baseline:
  DO: ["定位用户指定 PRD、原始诉求、已确认约束、关联 PRD 和必要决策；目标不唯一只问文件/范围，可先查规则，不自选相邻文档",
       "读适用根/子目录规则及产品/设计上下文，记 PRD 路径/版本或 hash、日期、代码仓路径、commit/dirty，明确当前工作树；无 Git 用可用版本，缺源码留证据缺口并继续文档审查",
       "提取目标/非目标、角色/权限、状态、旅程、FR/NFR、UAT/验收、依赖、决策与分期/终验；无 ID 用稳定章节，不造 SPMS key；只读工具可用时核对关联及重复立项，录入不等于实现",
       "将影响产品选择的已支持/未支持/复用/零改动/兼容/已验证/性能等主张与前置列清单，区分事实和未来目标",
       "确定报告路径，记录四维审查、代码核实、报告写入/回读状态；可查事实先查，只问证据不能推出的产品决定，缺输入继续独立部分"]
```

## 修复效果复审

```formal
RULE @prd_re_review:
  WHEN: "用户要求复审 / re-review / 检查修复效果"
  MUST: ["按报告合同复审；明确要求完整重审时执行完整流程",
         "按用户指定、当前上下文或明确记录链接找上轮报告，候选不唯一只澄清相关范围，不按文件时间猜选",
         "默认覆盖上轮全部问题、待核实项和修复直接影响的回归；指定 ID/范围时其余标未复核；缺上轮报告可审当前对象，不声称旧项逐条关闭",
         "原文可由上轮评审对象引用定位，按报告目录解相对路径并核实身份，唯一明确时可省原文参数；读处置节/applied 文档，独立核对问题与修订关联内容，不以已 applied 判通过",
         "逐项复审表沿用来源 ID 与上轮链接/版本，默认另存新报告保留历史；启用台账时登记独立评审并关联前轮，局部通过不覆盖未核查范围"]
```

```text
$review-prd docs/PRD-login.md，对照 docs/PRD-login.review.md 复审修复效果，逐项记录结果。
```

## 2. 编排四个审查视角

先读 [references/review-lenses.md](references/review-lenses.md) 的对应节，再分派。各视角都以用户约束为边界，不为凑问题扩大产品范围。

| 角色 | 主责 | 必须交回的专属结果 |
| --- | --- | --- |
| `facts` | 代码现状、依赖契约、主张真伪与内部冲突 | 现状核实矩阵：已有能力、真实增量、行为冲突、未核实项及证据 |
| `product` | 创新、先进性、AI 价值与合理范围 | 关键产品选择的价值/取舍；有依据的遗漏机会、AI 闭环与前置 |
| `experience` | 可用性、便捷旅程、美观 UI 与状态表达 | 各角色任务闭环；体验/视觉缺口；界面证据等级 |
| `acceptance` | SMAR、UAT、需求追踪、分期与终验 | 目标/场景/需求/验收/UAT 的断链；独立用户价值、计划规模与终验一致性 |

### 调度规则

```formal
RULE @prd_review_scheduling:
  MUST: ["主 agent 在多 agent 工具可用且当前授权允许时实际启动只读 subagents，综合裁定并单写报告；简单局部文档可用两位 reviewer 各覆盖两个视角，分别回报覆盖与结论",
         "按实际剩余槽位安排，主 agent 也占槽；总槽位为 4 时先运行 3 个视角，再复用空闲 agent 完成第 4 个，更少则分批",
         "工具/授权不允许或用户要求串行时，主 agent 顺序审四视角并如实记录执行方式，不漏维度",
         "模型继承主 agent；初轮各 reviewer 获相同基线、用户约束与原始材料，独立调查，不用预设缺陷或别人结论作答案"]
  MUST_NOT: [simulate_independent_agents_in_one_response, agent_initiated_model_or_reasoning_override,
             "reviewer 改 PRD/代码、继续派生 agent 或写最终报告"]

RULE @prd_review_resources:
  MUST: ["代码可并行只读，共用浏览器/外部环境由一个操作者采证；必要验证按现有授权，无法执行记缺口",
         =>[reviewer_failed, "重派一次或主 agent 接管，无法核实部分列待核实，不无限重试"]]
  MUST_NOT: "自动运行迁移、种子、部署、收费模型评测或有副作用的业务操作"
```

### Reviewer 派发合同

```text
你负责 {facts/product/experience/acceptance}，只读评审，不实现、不改稿、不继续派生 agent。
用户目标及范围：{原始要求、排除、已有产品决定}
PRD 与基线：{PRD 绝对路径及版本；代码仓根、commit/dirty、日期}
必读：{本 SKILL.md、report-contract.md、review-lenses.md 对应节的绝对路径}
相关材料：{规则、产品/设计上下文、已知代码入口、关联 PRD/需求的实际位置}
资源边界：{可读目录、可运行检查、外部查询与浏览器授权}
核实产品主张依赖的实际行为；新功能未实现不算缺陷，现有代码也不是永久产品约束。
交回：范围与基线；候选问题（PRD 定位、场景/触发、影响、证据、最小产品修订、
复核判据、严重度与置信度）；本视角专属结果；待核实/待决定项及所需证据。
没有问题可以返回空 findings。机会提案须说明用户价值与约束，不强制增加功能或实现方案。
```

## 3. 主 agent 交叉复核与裁定

```formal
WORKFLOW @prd_adjudication:
  DO: ["逐条读候选问题的 PRD 段落与原始证据，核对角色/作用域/版本/触发；文件/函数存在或 reviewer 一致不等于行为事实",
       "追到入口/权限、业务逻辑、结果/状态等用户或外部系统可见行为；按需求核对租户、分页、重复/冲突、数据保护、恢复、AI 权限/预算及依赖，部分链路只记部分核实",
       "将事实影响传回产品/体验/验收等相关视角，只复核受影响链路，不设计具体调用方案",
       "按根因及修订动作去重并保留所有实质问题，确认项进 findings，低证据项待核实，纯偏好排除；非必要高价值机会单列，不凑数或用总分抵消阻塞",
       "核对目标 ↔ 场景 ↔ FR/NFR ↔ 验收 ↔ UAT 状态 ↔ 分期/终验；每期独立价值，计划尽量 5–8 个 Milestone，9–10 个连续性例外有理由、最多 10 个，不要求 PRD 已拆技术任务 M1…Mn",
       "交付前核对 PRD/代码并发变化，只重审受影响结论，无法收敛标待核实；收集或停止仍运行的 reviewer"]
```

## 4. 输出评审结果

使用 [报告合同](references/report-contract.md) 的完整顺序与字段：先问题，后总体结论和修订顺序。报告头引用原 PRD；代码矩阵、四维/六维覆盖、SMAR/UAT 与分期核查、非阻塞机会均按合同保留。

```formal
WORKFLOW @prd_report_delivery:
  DO: ["主 agent 单写完整报告，空 findings、缺源码或需补证也按 @prd_default_delivery 处理",
       =>[report_saved, "回读实际文件，沿评审对象链接核对原 PRD 身份/版本/范围及实际报告目录下的相对路径，检查全部字段和链接"],
       "按第 5 节给是否需要修改与下一步；已保存则给实际链接、通过/需修订/需补证及关键摘要，仅聊天则交完整报告，保存失败按 @prd_report_failure 交付未完成结果"]

RULE @prd_report_limits:
  MUST: "通过仅表示已核实范围可交给开发/测试计划，不表示功能已实现、性能实测、UI 验证或可直接上线"

RULE @prd_report_failure:
  WHEN: report_write_tool_unavailable_or_write_failed
  MUST: "明确报告未落盘、原因与目标路径，聊天保留完整内容，将保存标未完成，不链接不存在产物"

REVIEW:
  CHECK: [@prd_default_delivery, @prd_review_scope, @prd_claim_evidence, @prd_product_boundary,
          @prd_re_review, @prd_review_scheduling, @prd_review_resources, @prd_adjudication,
          @prd_report_delivery, @prd_report_limits, @prd_report_failure]
  ALSO: ["结论和证据等级一致，报告合同字段齐全；自定义章节、无 TC 或未录入 SPMS 未被误判为缺陷"]
  EXPOSE: on_issue
```

## 5. 是否需要修改与下一步

报告总体结论与最终回复都先给一行明确判定，再给可直接复制的下一步命令：按当前客户端的技能调用写法（如 Codex `$apply-doc-review`、Claude Code `/apply-doc-review`），路径填实际文件。

| 判定 | 条件 | 下一步 |
| --- | --- | --- |
| **需要修改** | 结论为「需修订」，或「需补证」且所缺事实/产品决定要写回 PRD | `apply-doc-review <本报告路径>`：核实意见、补查事实、交互确认决定后修订 PRD |
| **无需修改** | 结论为「通过」，只剩 P3 或产品增强机会 | 按 PRD 期次出开发计划：`dev-plan <PRD 路径> P1`（PRD 未分期时省略期次；已知某期已有计划时给下一期）。想采纳 P3 或增强机会时，可选 `apply-doc-review <本报告路径>` |

所缺证据只能由外部提供、PRD 本身不需改写时，写明缺什么、由谁提供，补齐后用 `review-prd <PRD 路径>` 复审。仅聊天模式没有报告文件时，说明需先落盘报告才能交给 apply-doc-review。只给出建议命令，不自行启动下一步。

## 项目状态回写

若仓库已登记状态台账，或存在仓根 `.xgent-ai/sdlc/protocol.md`（兼容已有 `docs/project-status/protocol.md`；两套并存且未登记切换时先核对，不能双写），交付原产物后读取台账的回写规则（默认目录读上述协议），按仓内协议更新受影响对象的当前状态与来源链接，必要时维护索引；v4 日常直接更新当前页，事件留存条件或旧版事件要求以仓内协议为准；未启用时不自动建表，不依赖另装技能。只读/仅聊天/不保存要求优先，登记失败须说明，不影响如实交付已完成的原产物。

每次实际评审登记独立评审 ID、PRD 版本、覆盖范围、原结论与报告路径；复审关联前轮，保留历史，不因通过改写 PRD 实施状态。台账更新属于已有跟进协议的产物索引，不修改原 PRD。
