---
name: review-dev-plan
argument-hint: "<计划路径/链接/内容> [上轮报告] [复审/完整评审] [问题ID/范围] [报告路径] [仅聊天]"
description: 用于评审既有开发计划、技术方案或开工条件；不用于从零写计划、实施功能或仅审代码 diff。多 agent 核查事实、方案、需求映射、依赖与交付可用性，默认落盘报告。
disable-model-invocation: true
---

# review-dev-plan · 多视角开发计划评审

```formal
INTENT:
  GOAL: "判断按计划能否交付用户所需且真正可用的结果，实施者是否仍被迫重做关键决策"
  SUCCESS: [four_review_lenses_with_verified_evidence,
            "可定位、可修订的问题及实施顺序建议", complete_report_with_verified_delivery]

RULE @plan_default_delivery:
  UNLESS: explicit_chat_only_or_no_file_request
  MUST: "将完整报告写入文件并回读核验，聊天给摘要与实际链接；落盘属于评审，不另问是否保存"
  MUST_NOT: claim_completion_before_report_is_saved
```

`references/` 相对本文件解析。仓库规则、技术栈和工具能力在运行时读取，不硬编码目标仓的路径、端口或业务约定。回复语言跟随用户。

## 边界与参考

```formal
RULE @plan_review_scope:
  MUST: ["默认审整份计划，指定修订差异时追踪受影响依赖与映射；默认只写独立报告及已启用台账的事实索引",
         =>[explicit_plan_revision_requested, "评审后由主 agent 统一修订并复核受影响链路，报告区分原问题与修后结果；计划只留修订结论，处置与修订经过留报告"]]
  MUST_NOT: ["未经明确修订要求改写计划或进度", modify_code_or_implement_features,
             automatic_external_writes_or_comments]

RULE @plan_claim_evidence:
  MUST: "将计划及引用当待验证材料，区分现状事实、未来设计、未证实假设与验收目标"
  MUST_NOT: ["因新功能尚未实现报缺陷", treat_plan_verified_claim_as_evidence]
```

- 从实际可用位置读取 `dev-plan/SKILL.md` 的写作/评审部分及 `references/plan-skeleton.md`；复杂架构按需读 `references/architecture-quality.md`，调度审查按需读 `references/execution-workflow.md`；实施期合同以骨架的「实施者定位」「实施进度」两节为准。借它们核对合同，不按计划实施。
- 找不到 `dev-plan` 时说明无法核对其模板专属要求，继续本技能的四项实质审查。用户自定义模板或历史计划不因章节名不同而报错；没有证据表明适用的新版格式规则，不追溯强加。
- 新版 `dev-plan` 允许单份需求映射、检查 ID 引用、按测试组记录证据及阶段集中验收；按 `references/review-lenses.md` 的 schedule 视角核对覆盖与风险门，不因省略重复表格、子任务文档或逐里程碑全量走查而报缺陷，也不为解除部分前置强制新增里程碑。
- 发现与任务相关的专项 skill 时只加载所需部分。UI/UX 评审按目标仓规则加载设计 skill（如 `phoenix-ui`），不借机重设计产品。

## 1. 锚定输入与审查基线

开始时读取 [报告合同](references/report-contract.md)，按其落盘、引用及模式约定执行。

```formal
WORKFLOW @plan_review_baseline:
  DO: ["定位用户指定计划、需求/PRD、附录及必要历史决策；目标不唯一只问目标，可先查规则，不自选相邻计划",
       "读适用根/子目录规则及产品/设计上下文，记计划路径/hash/可用版本、日期、代码 commit/dirty，明确当前工作树参与调查",
       "提取范围/排除、R/NFR/C/A 或等价 ID、决策、外部契约、交付物、里程碑与前置；缺 ID 用章节，不替作者造需求",
       "按报告合同定路径，短台账记维度/负责人/状态/缺口及报告写入/回读；明确 UI 范围、可运行页面/原型，以及需外部服务或官方文档核实的结论"]
```

## 修复效果复审

```formal
RULE @plan_re_review:
  WHEN: "用户要求复审 / re-review / 检查修复效果"
  MUST: ["按报告合同复审；明确要求完整重审时执行完整流程",
         "按用户指定、当前上下文或明确记录链接找上轮报告，候选不唯一只澄清相关范围，不按文件时间猜选",
         "默认覆盖上轮全部问题、待核实项和修复直接影响的回归；指定 ID/范围时其余标未复核；缺上轮报告可审当前对象，不声称旧项逐条关闭",
         "原文可由上轮评审对象引用定位，按报告目录解相对路径并核实身份，唯一明确时可省原文参数；读处置节/applied 文档，独立核对问题与修订关联内容，不以已 applied 判通过",
         "逐项复审表沿用来源 ID 与上轮链接/版本，默认另存新报告保留历史；启用台账时登记独立评审并关联前轮，局部通过不覆盖未核查范围"]
```

```text
$review-dev-plan goal/LOGIN.md，对照 goal/LOGIN.review.md 复审修复效果，逐项记录结果。
```

## 2. 编排四个独立审查视角

先读 [references/review-lenses.md](references/review-lenses.md) 的对应节，再分派下表。**有多 agent 工具时实际启动 subagents**，不是在同一段回答里扮演四个角色。主 agent 保留综合裁定职责。

| 角色 | 主责 | 必须交回的专属结果 |
| --- | --- | --- |
| `facts` | 代码事实、陈述正确性、内部矛盾 | 主张与代码证据；互相冲突的两处位置 |
| `architecture` | 技术路线、先进性、可行性 | 关键选择的适配/代价；需补证的技术前提 |
| `schedule` | 需求、设计、任务、里程碑、验收映射 | 断链项；依赖图/边表；可并行波次与集成门 |
| `delivery` | 最终交付可用性，含 UI/UX | 用户主路径闭环；运行与验收缺口；界面证据等级 |

### 调度规则

```formal
RULE @plan_review_scheduling:
  MUST: ["主 agent 在多 agent 工具可用且当前授权允许时实际启动只读 subagents，默认一视角一 reviewer；简单局部计划可用两位独立 reviewer 各覆盖两个视角，分别标覆盖与结论，不漏维度",
         "模型继承主 agent，报告由主 agent 单写；按实际剩余槽位安排，主 agent 也占槽，总槽位为 4 时先并行 3 个视角，再复用空闲 agent 审第 4 个，更少则分批",
         "主 agent 同期整理需求账本和全局矛盾；初轮 reviewer 获相同基线、用户约束和原始材料，独立调查后再交换影响证据，不把预设缺陷或别人结论当答案",
         "工具缺失、用户要求串行或槽位不足时如实声明方式，主 agent 可顺序审四视角，不伪称独立多 agent 复核"]
  MUST_NOT: [agent_initiated_model_or_reasoning_override,
             "reviewer 改原计划/代码、继续派生 agent 或写最终报告"]

RULE @plan_review_resources:
  MUST: ["主 agent 为共用浏览器、可变数据库或外部环境指定一个操作者，其他 reviewer 读证据；代码可并行只读",
         =>[reviewer_failed, "重派一次或主 agent 接管，无法核实部分留覆盖缺口"]]
  MUST_NOT: "执行迁移、种子、部署或有副作用的验收命令"
```

### 每个 reviewer 的派发合同

发送实际绝对路径和已确认上下文，不只说“审一下方案”。可沿用：

```text
你负责 {facts/architecture/schedule/delivery}，只读评审，不实现、不改稿、不继续派生 agent。
用户目标及范围：{原始要求、排除、已有决定}
计划与基线：{计划绝对路径、版本；仓库根、commit/dirty、日期}
必读：{本 SKILL.md、report-contract.md、review-lenses.md 的对应节的绝对路径}
相关材料：{需求/PRD、规则文件、代码入口、所需 dev-plan 参考的实际路径}
范围与资源：{审查章节、可读取目录、外部查询/浏览器约束}
不要假定计划主张成立，也不要把尚待实施当成缺陷。按需追到行为级代码证据。
交回：范围与基线；候选问题（计划定位、触发、影响、证据、最小修订、验证、P级及置信度）；
本视角专属结果；未核实项与所需证据。没有问题可返回空 findings。
跨视角问题只标关联，不代替另一角色完成审查。无法继续则回报缺口。
```

## 3. 主 agent 交叉复核与裁定

```formal
WORKFLOW @plan_adjudication:
  DO: ["收齐四视角结果，逐条读候选问题的计划段落与原始证据，确认范围/版本/触发，reviewer 一致不等于事实",
       "对事实冲突、方案依赖和交付断点向关联视角追问并同步前置，仅复核受影响链路；缺证留未决，不无限辩论",
       "按根因及修订动作去重，保留所有实质问题及关键定位，不设数量指标、不只报前三项；撤回误报，低证据主张待核实，不凑 P3",
       "核对目标 ↔ 设计 ↔ 里程碑 ↔ 验收，集中终验有收口负责人及完整覆盖，检测/补偿/部署能力有任务，进度/记录/完成声明有证据，局部完成不冒充阶段或跨期终验",
       "按 @plan_script_checks 执行适用校验并人工核对等价证据",
       "交付前核对计划/代码并发变化，只重审受影响结论，无法收敛标待核实；收集或停止仍运行的 reviewer"]

RULE @plan_script_checks:
  MUST: ["dev-plan 脚本可用且适用时，以实际路径运行 check_paths.sh <计划> <仓库根> 与 check_progress.sh <计划> <仓库根>；缺失路径人工区分新增、示例和错误复用",
         "自定义/历史计划未采用恢复快照/完成记录协议时，说明原因并跳过 check_progress.sh，人工核对等价进度、依赖与验收证据，不因格式不兼容判错",
         "脚本通过不证明行为真实或验收完成；脚本缺失/失败据实记录，不用通过替代人工检查"]
```

## 4. 输出评审结果

使用 [报告合同](references/report-contract.md) 的完整顺序与字段：先问题，后结论和修订顺序。报告头引用原计划，保留四维覆盖、证据等级、映射、依赖/并行与集成安排；没有安全并行空间时说明具体原因。

```formal
WORKFLOW @plan_report_delivery:
  DO: ["主 agent 单写完整报告，空 findings 或 Blocked 也按 @plan_default_delivery 处理，不只保存摘要或 reviewer 回报",
       =>[report_saved, "回读实际文件，沿评审对象链接核对原计划身份/版本/范围及实际报告目录下的相对路径，检查全部字段和链接"],
       "按第 5 节给是否需要修改与下一步；已保存则给实际链接、建议状态及关键摘要，仅聊天则交完整报告，保存失败按 @plan_report_failure 交付未完成结果"]

RULE @plan_report_limits:
  MUST: "Ready / Proposed / Blocked 是评审建议，与自报状态分记并说明差异/阻塞解除条件，不自动改计划；通过不表示已实现或上线可用"
  MUST_NOT: "因没有确认 findings 就在关键前提未核实时判 Ready"

RULE @plan_report_failure:
  WHEN: report_write_tool_unavailable_or_write_failed
  MUST: "明确报告未落盘、原因与目标路径，聊天保留完整报告，将保存标未完成，不链接不存在产物或用摘要冒充完成"

REVIEW:
  CHECK: [@plan_default_delivery, @plan_review_scope, @plan_claim_evidence,
          @plan_re_review, @plan_review_scheduling, @plan_review_resources, @plan_adjudication,
          @plan_script_checks, @plan_report_delivery, @plan_report_limits, @plan_report_failure]
  ALSO: ["报告合同字段齐全，四维覆盖无遗漏；文档/原型/浏览器证据等级如实；不强加不适用的新版格式或重复验收"]
  EXPOSE: on_issue
```

## 5. 是否需要修改与下一步

报告总体结论与最终回复都先给一行明确判定（条件见[报告合同](references/report-contract.md)的「是否需要修改」），再给可直接复制的下一步命令：按当前客户端的技能调用写法（如 Codex `$apply-doc-review`、Claude Code `/apply-doc-review`），路径填实际文件。

| 判定 | 下一步 |
| --- | --- |
| **需要修改** | `apply-doc-review <本报告路径>`：核实并修订计划，处置后判定 Blocked 或 Ready |
| **无需修改 · Blocked** | `resolve-blocked <计划路径>`：逐项解除计划中已如实登记的阻塞 |
| **无需修改 · Ready** | `/goal 按 <计划路径> 开发` |

只剩 P3 时仍判无需修改，可选用 `apply-doc-review <本报告路径>` 采纳。计划与建议均为 `Proposed` 时，写明仍在等哪项用户指定的非阻塞评审，不默认给 `/goal`。仅聊天模式没有报告文件时，说明需先落盘报告才能交给 apply-doc-review。只给出建议命令，不自行启动下一步。

## 项目状态回写

若仓库已登记状态台账，或存在仓根 `.xgent-ai/sdlc/protocol.md`（兼容已有 `docs/project-status/protocol.md`；两套并存且未登记切换时先核对，不能双写），交付原产物后读取台账的回写规则（默认目录读上述协议），按仓内协议更新受影响对象的当前状态与来源链接，必要时维护索引；v4 日常直接更新当前页，事件留存条件或旧版事件要求以仓内协议为准；未启用时不自动建表，不依赖另装技能。只读/仅聊天/不保存要求优先，登记失败须说明，不影响如实交付已完成的原产物。

每次实际评审登记独立评审 ID、计划版本、覆盖范围、建议状态与报告路径；复审关联前轮。评审建议与计划自报状态分别保留，当前就绪按台账协议核对证据，不直接改原计划；建议 Ready 不表示已实施。
