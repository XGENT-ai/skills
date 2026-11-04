# PHOENIX-DIAGRAM-STATECHART · XGENT-ai Skills · Phoenix Diagram 状态图模型检查与上游解耦（原生 Rust · 有限模型穷举 · 嵌套/并行状态图）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-04 · main / `ca6ab799406f81acb9aed38e34e1a90a9be997fd` · clean；本计划及其 SDLC 状态页是新增的未提交文件。
>
> 需求来源：用户需求，无 PRD。2026-11-04 用户在对话中要求：参考 `local/plugins-main/pstack/skills/statechart`，给 `tools/phoenix-diagram` 加类似功能，并去掉 phoenix-diagram 与 upstream 的关联；同日确认了 R-1–R-7 的形态与范围。按用户决定，本计划替代 [Phoenix Diagram PRD](../PRD-PHOENIX-DIAGRAM.md) 中两处口径：WORK-N3 AC-1 的 legacy-2.17 兼容，以及 WORK-N2 AC-3 的「固定参考源码可追溯」。PRD 原文不改，见 C-9。
>
> 本期交付：**两项。(1) Phoenix Diagram 第 11 类图 `statechart`：作者用 JSON 声明有限状态模型，Rust 引擎穷举可达状态，判定每条规则与死锁，给出每条被违反规则的最短路径。结果画成扁平、父状态或并行区域状态图，交付为独立 HTML，带 breaks / mayBreak / 出错路径标记和报告卡片。(2) 删除 `tools/phoenix-diagram/upstream/` 冻结来源和 legacy-2.17 输入合同，标识改为 Phoenix 自有，许可署名保留。**
> **开发范围：本仓 `tools/phoenix-diagram`（引擎、viewer 源、assets、tests）、`skills/phoenix-diagram`、`scripts/*phoenix-diagram*`、`test/phoenix-diagram`；开发构建同步生成的 `dist/phoenix-diagram` 与 `skills/phoenix-diagram/ENGINE.json`；`README.md`、`docs/phoenix-diagram.md`、`bin/xgent-skills.js` 中复述图型的文字。依据见 §0.1。**
> 本期独特职责：lifecycle 画的是作者给定的状态流；statechart 从模型推导出图面，并判定规则是否成立。
> **顶层排除：TLA+/TLC 与时序 PROPERTY；对 statechart 参考实现的任何运行期或测试期依赖；版本号提升与 npm/GitHub 发布；改写 PRD、历史计划或记录；Phoenix UI。**

## 实施者定位

```formal
ROLE @implementer:
  AS: senior_software_engineer
  MISSION: "依据本计划、仓库规则和当前代码实施"

RULE @development_scope:
  MUST: ["仅实施本计划明确纳入的系统与改动；范围外依赖按 §4 的契约、提供方与解除条件跟进",
         =>[external_gap_blocks, "回写影响与继续条件，推进不依赖它的已授权工作"],
         =>[dependency_ready, "从当前阻塞移除，剩余项按本方未实现或验收未运行重新归因"],
         "新增开发对象须有用户明确依据；fixture 只覆盖约定契约的必要行为"]
  MUST_NOT: ["因源码可访问、自有服务、同仓目录、就地修订或委派扩大开发范围",
             "代建、改造或部署范围外依赖；将 fixture 做成替代外部系统的产品或冒充真实集成验收"]

RULE @communication_and_source:
  MUST: ["报告用路径、需求/检查/里程碑 ID 定位，不复述计划",
         "源码注释解释必要原因；追溯放记录或提交说明"]
  MUST_NOT: "在源码放需求编号、agent 标记或规划元数据"

RULE @identifiers_and_tasks:
  MUST: ["需求/约束/假设 ID 只在账本登记一次，上游 ID 保留原编号及来源；检查、里程碑、任务 ID 分别在验证表、里程碑表、按需内部任务表定义，其他位置仅引用或展开",
         "修订保留已有 ID，删除/拆分同步引用",
         =>[task_table_present, "按交付物、前置成果与检查 ID 推进，进展及证据写入所属里程碑记录，不另设任务状态表"]]

RULE @milestone_completion:
  MUST: ["本里程碑退出检查全部通过才能记完成；未运行、失败、环境缺失分别记缺口",
         "阶段终验由收口里程碑负责，通过前不宣称整期或跨期需求已验收"]

RULE @role_acceptance:
  MUST: ["Agent 按 §9 指定的知识、资料及权限约束模拟上手与业务场景，记录实际结果与卡点",
         "有工具和权限的真实环境检查由 Agent 执行；真人公测指标按指定阶段另验"]
  MUST_NOT: ["角色模拟借内部源码或实现者补步骤", "将模拟结果写成真人数据或冒充本阶段真人指标已通过"]

RULE @implementation_route:
  MUST: ["直接遵循已定技术路线、接口、复用锚点和检查命令；未写明的实现细节按最近的同构代码就地决定",
         "调查只读本里程碑改动落点及其直接调用方和依赖",
         =>[plan_premise_refuted_or_route_infeasible, "停下，就地修订对应正文并在记录写明偏差；仅此条件触发路线重审"]]
  MUST_NOT: "无上述偏差而重新调研已定决策，或另起方案设计、比较、评判"

RULE @delivery_first:
  MUST: ["优先交付里程碑的用户可用行为；事务、锁、重试、幂等、降级、补偿、自愈、沙箱/隔离、额外校验与告警只实现正文已定义或仓库规则要求的",
         "实现以性能等 NFR 为目标：同一问题开销相近时选更优写法，避免明显低效",
         =>[unplanned_failure_path_found, "仅当本期真实路径会触发、后果不可接受且现有框架/约束不足时用最小机制处理，就地补正文触发与后果并在记录写明偏差；否则失败时返回清晰错误"],
         "测试围绕 §9 检查对应的行为、关键边界与实施中发现的真实缺陷，每个行为用能捕获回归的最少用例，优先扩展既有测试"]
  MUST_NOT: ["为假想故障、内部契约或框架已保证的行为加防御、回退或测试",
             "为正文未列为本期的 NFR 做基线、压测、专项验证、调优迭代或引入专门基础设施",
             "用 fallback 或静默恢复掩盖编程错误",
             "为覆盖率、逐个函数或穷举非法输入批量写测试"]

WORKFLOW @implementation_sequence:
  TRIGGER: on_implementation_start_or_resume
  DO: ["按 @resume_protocol 恢复，核对仓库规则、当前任务相关正文与依赖、工作树",
       "按 §10 连续实施批次推进，默认一个实现里程碑为一批，不按文件、函数、一次编辑或一轮 goal 对话切批",
       =>[behavior_tests_needed, "用当前行为的最小用例先红、再绿、再重构，红绿循环不附带整组回归"],
       "完成本批相关实现和测试代码，再按 @verification_rhythm 与 @reverification 做局部退出检查",
       "批次通过后继续下一可执行批次，里程碑完成按 @progress_writeback 回写，汇报或回答状态询问后继续实施"]

RULE @implementation_stop:
  WHEN: user_explicitly_pauses_cancels_or_switches_task
  MUST: "按 @writeback_timing 保存后停止"

RULE @verification_rhythm:
  MUST: ["批内只为具体失败、技术疑问或必要前置运行能回答它的最小测试/编译检查，无触发则继续实现",
         "§9 整组局部检查（含受影响模块回归与静态检查）等本批交付物齐备后统一运行，不因命令叫‘局部/快速’就逐次编辑重跑",
         "全量测试、端到端、真实环境与界面走查、故障注入、评测、干净构建仅在指定阶段收口运行，同环境走查合并",
         "下游前置、数据、安全、不可逆操作、发布及用户分期门及时验证，不等无关终验；稳定契约下可用 fixture 开发，不能替代真实集成",
         "优先复用增量构建、缓存和已就绪环境；调优用缩小版本迭代，达标后再跑完整检查",
         "遵守仓库或用户明确指定的检查门"]
  MUST_NOT: "无故清缓存、重启栈或重复启动尚在运行的检查"

RULE @reverification:
  MUST: ["证据仅在实际覆盖的代码、契约、配置、依赖或数据改变时失效；受影响检查合并到下一适用批次退出或阶段收口，不因失效立即停工",
         "同批连续改动只复验一次，未补证不能宣称新版通过",
         =>[downstream_needs_evidence_or_risk_gate_imminent, "先补必要检查"],
         =>[check_failed, "先定位原因并完成同因修复，再验失败项与修复波及项；未影响的已通过项直接引用"],
         =>[shared_contract_public_entry_build_or_dependency_changed, "核对波及面，必要时扩大复验，不把所有局部修复升级成整组或全量重跑"]]
  MUST_NOT: "仅因续做、汇报、回写或提交使证据失效"

RULE @parallel_delivery:
  MUST: [=>[&&[tasks_independent, "收益超过委派及集成成本", subagents_available], "使用 subagents，模型默认继承"],
         "任务说明只给目标/ID、稳定契约、可写与禁改路径、运行资源和检查",
         "共享文件单人负责，数据库、端口、浏览器等隔离或串行",
         "子 agent 返回结果、路径、验证证据与缺口，默认不另写任务文档；主 agent 审查并集成验收",
         "边界变化先回报，失败只阻塞依赖任务；结束对话前收集或停止仍在运行的任务"]
```

## 实施进度（实施期持续更新）

```formal
RULE @progress_entry:
  MUST: "本节是跨对话恢复的唯一汇总入口；协议文字固定，实施期只更新‘恢复快照’和‘完成记录’"

RULE @resume_protocol:
  MUST: ["依次读仓库规则、本节、当前任务相关正文与依赖，再核对 git status 和相关 diff",
         =>[records_conflict_with_worktree, "先查明原因，不凭记录覆盖用户改动，不凭代码存在推断验收已通过"],
         "仅在继续未完任务、核验前置验收、排错或审查时，按链接读对应记录的相关部分，不默认加载全部记录或原始日志",
         "从未完交付继续，不先例行重跑回归；检查前核对当前批次、触发原因与已有证据，未到验证点且无具体失败/疑问或必要前置则继续编码，已通过且覆盖未变的检查直接引用"]

RULE @writeback_timing:
  MUST: [=>[milestone_completed, "立即按 @progress_writeback 回写，再报告完成或提交"],
         =>[downstream_needs_partial_delivery, "所需前置检查通过并在记录保存证据即可推进"],
         =>["实质进展后受阻、暂停、交接、发现偏差或结束对话", "保存成果与缺口"]]
  MUST_NOT: "仅因普通子任务切换单独回写"

WORKFLOW @progress_writeback:
  DO: ["主 agent 更新对应记录，再同步快照变化字段和完成记录一行，不重写无变化内容或追加流水",
       =>["范围、决策、接口、数据、风险或退出条件有偏差", "就地修订对应正文，不追加勘误历史"],
       =>[source_cr_request_or_explicit_subscope_completed, "按 @source_cr_writeback 核对并回写实现结果"],
       "按 @project_index 同步已启用台账", "执行 @progress_consistency 自查"]

RULE @progress_format:
  MUST: ["快照写当前任务、进展与下一步，命令及证据只进记录",
         "首次回写删占位行，每个已记进展的里程碑恰好一行，状态仅‘进行中 / 阻塞 / 已完成’；完成时移至已完成行末，最近完成取该行",
         "更新时间带时区，摘要 1–2 句，记录填相对计划的链接，如 [M1 记录](FEATURE.records/M1.md)",
         "仅在本节快照、完成记录及对应记录文件内定位回写；里程碑表、检查表、ADR 等定义行不随进度改动"]
  MUST_NOT: "按里程碑 ID、‘状态’等同名字样对全文件查找替换"

RULE @source_cr_writeback:
  WHEN: requirement_source_explicitly_assigns_cr_requests_to_this_plan
  MUST: ["保留 CR/原 R/Q/RESP 语境，以既有需求/验证映射核对承接范围与收口检查，不因文档提及 CR 就改写",
         "对应请求或明确子范围实现且约定检查通过后，在本次进度回写追加结果，不等无关里程碑；最后一个里程碑完成时再核对漏回告",
         "通读来源 CR，核对最新有效请求、往返答复、计划范围与适用证据；部分承接或仍需其他计划时只回告已完成部分并列余项，不关闭整条请求或整份 CR",
         "复核实际回写文件最新内容，保留并发增量；文末追加实现结果 RESP，沿用编号并引用原请求、被更新的旧 RESP，说明被取代的旧‘未实现’判断；附实现行为、计划/里程碑、代码版本、验证结果、使用条件与剩余配合，不改正文或历史",
         "本方实现、发布生效、对方验收分别陈述，无证据不标已发布或已获对方验收",
         "同请求范围、实现版本、验证结论已有实质等价回告则跳过，不重复追加或更新时间；有变化只追加受影响范围，不因无关 HEAD 变化记新轮",
         "来源缺失、目标不唯一、只读或无法保真写回时，在对应记录保存可追加答复、原因与实际目标，快照下一步保留恢复动作；仅改工作副本须说明原件未改，不宣称回写完成",
         "写入后回读范围、编号及历史完整性，在里程碑记录保存实际 CR/RESP 链接，无需重新加载规划或答复技能"]
  MUST_NOT: [automatic_sending_deployment_or_platform_state_changes, disclose_sensitive_internal_evidence_externally]

RULE @milestone_records:
  MUST: ["按需建 <计划名>.records/M<n>.md；头部写计划链接、更新时区、状态与验证基线（commit SHA；未提交写 dirty@起始SHA 与关键改动路径）",
         "正文只写交付行为/路径、必要偏差、未完项与继续动作；未完项含当前批次剩余交付、需复验检查及下次执行点，按回写时机保存，不每改一处记一笔",
         "按‘检查 ID/覆盖范围 | 命令或步骤 | 结果/必要证据’记录验收，共同环境与基线头部只写一次，有差异再单列",
         "同一测试组可覆盖多条退出条件，须逐条可追溯；引用原证据及复用依据，不复制断言全文、成功日志或调试流水",
         "证据只留命令、结果与必要少量输出，不为留证另写一次性脚本、哈希清单或大体量证据目录",
         "明确区分未运行、失败、环境缺失；仅复杂交接、排错或证据需独立保存时另建任务记录/日志/截图并链接"]

RULE @project_index:
  WHEN: "仓库已登记状态台账，或存在仓根 .xgent-ai/sdlc/protocol.md；兼容已有 docs/project-status/protocol.md，未启用则跳过"
  MUST: ["两套协议并存未切换时先核对，不双写；上述回写后读协议，只同步实际改变的当前状态与来源链接，必要时维护索引，默认直接更新本计划状态页",
         "来源直接链接计划快照/完成记录、里程碑记录的验收或未完项章节、已有评审/处置记录及其版本，不重写为 SDLC event、过程总结或证据副本，不先造 event 替代原记录",
         "里程碑推进、验证结果更新、来源版本变化、最终提交、整份计划完成均不单独构成新建事件理由",
         "新建前核对原记录及可取回版本；仅仓内明确要求不可变审计，或原记录与当前页无法保留并发/纠错事实时独立留存，说明依据与必留差异，其余直接引用",
         "旧协议强制事件时只留最小必要内容与原记录链接，不复述实施验收过程，不自行升级协议",
         "复用已有计划 ID，只更新本次范围，保留其他计划与各轮评审，不复制本节或里程碑记录；台账仅作索引，不替代恢复快照",
         "使用仓根相对路径与明确基线，未提交成果标明同步限制；登记失败说明缺口，不回退原成果"]

RULE @commit_protocol:
  MUST: ["在 §10 里程碑表后的‘提交点’提交，里程碑较多时在标出的阶段收口后提交，全部里程碑完成后必须提交一次",
         "仓库或用户对分支、提交信息、签名、是否由 agent 提交另有要求时从其要求",
         "提交前先回写进度；仅要求本次覆盖里程碑或阶段收口的检查，已通过且未失效的证据直接引用，不为提交重跑",
         "只暂存本计划相关改动，不纳入、不回退他人改动，不跳过提交钩子",
         "提交后把短 SHA 回填本次覆盖的各里程碑行‘提交’列，再单独提交只改计划文件的更新，该提交不回填自己的 SHA",
         "代码评审修复写‘修复’加短 SHA，UI 验收修复写‘UI修复’加短 SHA，均追加在原 SHA 后",
         =>[completed_before_commit_point, "对应行写‘待提交’"],
         =>[commit_unavailable, "写明原因并记入当前阻塞，不伪造 SHA"]]

RULE @completion_handoff:
  WHEN: last_milestone_completed
  MUST: ["依次按 @source_cr_writeback 核对 CR 回告、按 @commit_protocol 完成最终提交并回写 SHA，确认每个已完成行都有 SHA，再将快照‘下一步’写为代码评审；仍有 CR 回写缺口时一并保留",
         "随后报告完成，提示用户用 review-code 以本计划路径为参数评审；它从完成记录定位提交，并纳入未提交改动；评审由用户发起"]
  MUST_NOT: "实施中自行启动评审或修复评审意见"

REVIEW @progress_consistency:
  CHECK: ["每次回写后核对快照、完成记录、记录链接及本计划文件 git diff；非 Git 仓库对照回写前副本",
          "进度 n/N 的 N 等于里程碑总数，n 只计‘已完成’行；最近完成取最后一条已完成记录，不按编号推算",
          "链接指向已落盘文件；已完成行‘提交’为短 SHA 或‘待提交’，全部完成时每行有 SHA 且无‘待提交’",
          "本节外只有有意修订的正文，定义行未被进度文字改写"]
  EXPOSE: on_issue
  # 结构、计数与链接检查不证明验收真实通过。
```

### 恢复快照

- 最近更新：2026-11-04T15:30+08:00
- 当前进度：2/6 个里程碑完成
- 当前状态：进行中
- 最近完成：M2 · 删除 upstream 与 Archify 技术耦合，改用 Phoenix 标识（阶段 A 收口）
- 下一步：M3 · statechart 合同、表达式与穷举；退出条件 V-4
- 当前阻塞：无
- 代码基线：`ca6ab799406f81acb9aed38e34e1a90a9be997fd`（M1、M2 待本次提交）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-04T14:20+08:00 | 删除 legacy-2.17 合同、分支、fixture 与示例；未标记的原五类报 `input/contract-required`；V-1 通过，dist 一致性用例在 V-3 复验通过 | [M1 记录](PHOENIX-DIAGRAM-STATECHART.records/M1.md) | 待提交 |
| M2 | 已完成 | 2026-11-04T15:30+08:00 | 删除 upstream 与归档耦合，组装器/来源检查改为自有来源，标识改为 Phoenix；V-2、V-3（构建同步、十类真实浏览器交付与看图）通过 | [M2 记录](PHOENIX-DIAGRAM-STATECHART.records/M2.md) | 待提交 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| R-1 | 功能 | 用户 2026-11-04：参考 statechart 加类似功能；同日选定「原生 Rust 模型检查」 | 新图型 `statechart`。JSON 有限模型（变量、初值、动作、规则、死锁终态）由 Rust 穷举，得出每条规则的 holds / breaks / not-checked 和最短出错路径。消费端不需要 Java、TLC 或网络 | §5.1–§5.3；V-4 | 已确认 |
| R-2 | 功能 | 用户 2026-11-04 选定「本期含父状态与并行区域」 | 声明式 view 把模型状态投影为扁平、父状态（嵌套）或并行区域图。包括：合并同名状态与同名箭头、隐藏步骤、区域自动 `[条件]`、区域失真时回退为父状态、父状态提升、breaks / mayBreak / 出错路径标记、阶段着色 | §5.4；V-5 | 已确认 |
| R-3 | 功能 | 同 R-1（statechart 的报告与交付） | statechart 经 finalize 三门与实际看图，交付独立 HTML。包括：嵌套容器布局、Reader 中可选中父状态、标记图例、报告卡片（概况、每条违反规则的最短路径、成立/未检查的规则）；回执带 `model` 判定 | §5.5、§6；V-6、V-8 | 已确认 |
| R-4 | 功能 | 同 R-1；工具随附的 skill（`AGENTS.md` @skill_tool_independence） | 宿主能发现并正确创作 statechart：guide、SKILL.md 图型表与创作/交接规则、schema、最小示例、新参考文档，并写清与 lifecycle 的边界 | §5.8；V-7、V-8 | 已确认 |
| R-5 | 功能 | 用户 2026-11-04：「将 phoenix-diagram 的 upstream 相关的去掉，不要保持关联了」 | 删除 `tools/phoenix-diagram/upstream/` 及其全部技术耦合：冻结归档与 legacy 快照的校验、从归档组装 Reader、对归档的 oracle 生成器、build.rs 与脚本中的 upstream 输入。与 Archify 无关的 ICU/tz/crate 许可/品牌来源检查继续可用 | §5.7；V-2、V-3 | 已确认 |
| R-6 | 功能 | 用户 2026-11-04 选定「一并移除」 | 删除 legacy-2.17 输入合同、legacy schema、五份 legacy 示例和所有 legacy 分支。未标记合同的 architecture/workflow/sequence/dataflow/lifecycle 输入报 `input/contract-required`；显式 `phoenix-v1`（文件标记或 `--input-contract`）照常 | §5.6；V-1 | 已确认 |
| R-7 | 功能 | 用户 2026-11-04 选定「改成 Phoenix 标识，保留许可署名」 | schema `$id`/`title`、SKILL `based_on`、源码与文档里的「frozen Archify/快照 SHA」字样、品牌目录版本等改为 Phoenix 自有。NOTICE、Archify MIT 全文、JetBrains Mono OFL 与版权行保留 | §5.7；V-2 | 已确认 |
| C-1 | 约束 | `skills/phoenix-diagram/SKILL.md`「环境与停止」；PRD WORK-N2 AC-1/AC-2 | 生成、布局、校验、交付都在 Rust 内完成；消费端普通命令不下载、不安装；Reader 可以使用浏览器 JS | ADR-1；§2 | 已确认 |
| C-2 | 约束 | `AGENTS.md`「Working in This Repository」；`tools/phoenix-diagram/build-tools.lock.json` | 实际运行的检查经 mise，固定 Rust 1.99.1、Node 25.2.1、mr-boxington 1.28.0；工具缺失算失败，不算跳过 | §9 | 已确认 |
| C-3 | 约束 | `AGENTS.md` @verification_scope_and_cost | `npm run test:diagram` 是聚合入口，只在阶段收口运行；局部退出用 `--test` 或文件级命令 | §9 | 已确认 |
| C-4 | 约束 | `AGENTS.md`：「When skill names, arguments … change, update all interface restatements」 | 图型、合同、命令参数的复述同步到 README、docs、skill references、CLI HELP/guide 与 bin 文案 | §5.8；V-7 | 已确认 |
| C-5 | 约束 | `AGENTS.md` §6；`test/phoenix-diagram/skill.test.js:10-28` | SKILL.md description 中文 ≤200 字（测试限 200 字符，且前 100 字须含 14 个指定词），触发与排除放在前面；只校验所改 skill 的 YAML | §5.8；V-7 | 已确认 |
| C-6 | 约束 | Archify MIT（`tools/phoenix-diagram/viewer/LICENSE.txt`）、JetBrains Mono OFL（`viewer/JetBrainsMono-OFL.txt`）、pstack MIT（`local/plugins-main/pstack/LICENSE`，Copyright (c) 2026 Lauren Tan） | 派生代码与随附资源保留版权行和许可全文；移植 statechart 语义要新增 pstack 的 MIT 署名与全文 | §5.7、§5.9；V-2、V-8 | 已确认 |
| C-7 | 约束 | `AGENTS.md`（Preserve historical records/snapshots） | 不改写 `docs/plan/PHOENIX-DIAGRAM-*`、`*.records/`、`docs/PRD-*`、`.xgent-ai/experience/*`、`docs/diagrams/*` | §0.5 | 已确认 |
| C-8 | 约束 | `AGENTS.md` @skill_tool_independence | 只有 phoenix-diagram skill 随工具同步修改；不改其他 skill，也不改 Phoenix UI | §0.4 | 已确认 |
| C-9 | 约束 | 用户 2026-11-04 决定；PRD WORK-N3 AC-1、WORK-N2 AC-3 | 本计划完成后，legacy-2.17 旧稿不再被接收，发行资源也不再追溯到冻结参考快照。PRD 原文保留，替代关系只记在本计划和 SDLC 状态页 | §5.6、§5.7；§11 | 已确认 |
| C-10 | 约束 | `AGENTS.md`：「Stay on the current branch」 | 在 main 上实施，不建分支 | §10 提交点 | 已确认 |
| A-1 | 假设 | 收口需要真实浏览器；本机尚未核实 | 本机有 `src/browser/discovery.rs` 能发现的 Chrome/Chromium，或可设置 `PHOENIX_DIAGRAM_BROWSER` | M2 收口前由 Agent 跑一次 `finalize` 确认；缺失时把 V-3/V-8 的浏览器部分记为环境缺失并告知用户，不冒充通过 | 开放 |
| A-2 | 假设 | 通用 router 目前只服务调用方摆好的盒子，以及 architecture 的 boundary 框（`src/layout/router.rs:376-379`、`src/layout/architecture.rs:252-271`） | router 在多层嵌套结构框中走线，能通过现有构图门（重叠、贴边、交叉、标签放置） | 由 M5 的 T-1 验证：先只做摆放 + 走线，对 F-3、F-4 跑 `check_html`；各结果对应的路线见 §6.1 | 开放 |
| A-3 | 假设 | JSON 结构化表达式的可写性还没有实测 | 只读 skill 文档的宿主能写出通过校验的 statechart 模型 | V-8 角色模拟；失败时按 ADR-2 的重新评估触发处理 | 开放 |

用户没有明确要求的 NFR，本表不登记。性能按常规工程处理，见 §5.3 的状态上限与线性数据结构。

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | statechart 形态 | 新增第 11 类 `statechart`（phoenix-v1 合同）；lifecycle 不变 | lifecycle 继续画作者给定的状态流；Mermaid `stateDiagram` 仍转为 lifecycle | R-1、R-4；`SKILL.md` 中的 Mermaid 段 |
| D2 | 路径模式 | 不移植参考实现「超过 500 个模型状态时只画出错路径」的规则 | 图面始终是完整投影；图过大时由布局门给出可操作的诊断（合并标签、隐藏步骤）。参考实现设这个上限，是因为要逐状态调用 JS view 且 Mermaid 文本体量大；声明式 view 在 Rust 内求值，没有这项成本 | 参考 `scripts/ir.ts:139`、`scripts/run.ts:412-477` |
| D3 | 探索上限 | 最多 100,000 个不同模型状态，到达即停：未见违反的规则与死锁记为 `not-checked`，`complete:false`，并发 `statechart/state-limit` 警告；图面只画已到达的部分 | 防止无界模型挂死；数值沿用参考实现的复跑上限 | 参考 `scripts/ir.ts:137` RERUN_STATES |
| D4 | 死锁 | 默认检查；用 `model.deadlock.ends` 声明可以停下的终态；规则 id `deadlock` 保留，作者规则不得使用 | 不需要参考实现的「Done 空动作」惯用法，也不需要 CHECK_DEADLOCK 改名逻辑 | 参考 `SKILL.md` 的「Write the spec」段；`scripts/ir.ts:529-547` |
| D5 | 标记图例 | 图例固定四项（breaks、mayBreak、出错路径、起始），外加阶段颜色；每个被标记的状态在节点 `tag` 里写明它违反、可能违反的规则 id | 参考实现的 breaks 图例会把偶发违反的规则也混列进去；改后不再需要交接时口头补「哪个状态违反哪条规则」 | 参考 `passes/regions.ts:270-278`、`backends/mermaid.ts:141-151` |
| D6 | 阶段颜色词表 | `cyan, emerald, violet, amber, orange, slate`，依次映射 tone `frontend, backend, database, cloud, messagebus, external`；`rose` 留给 breaks | 复用 common cards 的 `dot` 词表和现有 tone 变量，主题与暗色自动跟随 | `skills/phoenix-diagram/schemas/common.schema.json` `$defs/cards`；`src/render.rs:98-108`；`assets/template.html:231-284` |
| D7 | 合同枚举 | `InputContract` 只保留 `PhoenixV1`，仍序列化为 `phoenix-v1`；只为 legacy 存在的 `contract` 参数随分支一起删除 | 回执、`data-input-contract`、`meta.input_contract` 的形状不变；clippy `-D warnings` 下不留未用参数 | `src/input.rs:79-86`；`scripts/test-phoenix-diagram-native.mjs:30` |
| D8 | 显式合同 | 以下两者都视同标记：CLI `--input-contract phoenix-v1`（含 compare 的 `--base-contract`/`--head-contract`），以及 `CompileOptions.input_contract` | 25 份未标记的 phoenix-v1 fixture 继续可用，「Archify 3.1 稿加参数读取」的用法也继续可用 | `tests/support/input_corpus.rs:57`；`src/compare/render.rs:446`；`references/compatibility.md:9` |
| D9 | Reader 组装 | `scripts/assemble-phoenix-diagram-reader.mjs` 保留拼接、内嵌目录、界面本地化与 `--check`；删掉对归档的读取和逐 hunk 审计；`svg-definitions.svg`、`node-card-style.svg` 改为直接维护的资源 | `viewer/` 片段和 `viewer/locales/` 成为唯一来源；`assets/template.html`、`assets/catalogs.json` 仍是生成物 | `assemble-phoenix-diagram-reader.mjs:13-29,148-161,235-246,279-307` |
| D10 | 目录适配 | `viewer/catalog-adaptations.json` 一次性并入 `viewer/locales/*.json`，同时把 locale 中的「Archify」改为「Phoenix Diagram」；组装器保留「不得含 archify」守卫 | 目录只剩一处来源；新增的 statechart 文案直接写进 locale | `assemble-phoenix-diagram-reader.mjs:177-208` |
| D11 | 归档派生的生成器 | 删除读取归档或 `local/phoenix-diagram/reference` 的 oracle 生成器（§5.7 第 6 项列出）。它们已产出的 phoenix-v1 oracle JSON 保留，作为 Phoenix 自有的回归基线，并删去其中的 legacy 段。`format/generate.mjs` 删除，`src/format/time/system_v.rs` 改为直接维护（来源仍由 `system-v-sources.json` 校验）。`text-segmentation/generate.mjs` 保留，只删去读取 Archify 示例的段 | 回归覆盖不降，也不再有任何指向归档的生成路径 | `tests/fixtures/format/generate.mjs:7-15`；`tests/fixtures/text-segmentation/generate.mjs:119-129` |
| D12 | 来源身份 | 删除 `upstream/SOURCE.json`。ICU 文本、SystemV 时区、Rust 许可、品牌目录这四项身份只以 `scripts/check-phoenix-diagram-source.mjs` 内的常量为准；`verifyFrozenSources` 改名为 `verifySources`，不再读归档 | 四项检查都不再依赖归档；品牌目录只少了「与归档原文件逐字节相同」这一项比对 | `check-phoenix-diagram-source.mjs:17-78,166-326` |
| D13 | 品牌来源字段 | `brand-marks.json` 与 `brand-license-sources.json` 删去 `archive`/`archiveSha256`，保留原文件的 `path`/`sha256` 和 notice 字段作为来源记录；`catalogVersion` 由 `archify-3.1.0` 改为 `phoenix-1` | 品牌回执里的版本串改为 Phoenix。两个文件的字节变化，需要级联更新 `brandSourceIdentity`、`brand-license-sources.json` 的 catalog 块和 `brand-legal.test.js` | `assets/brand-marks.json:1-11`；`assets/brand-license-sources.json:1-18`；`src/brands.rs:136,180` |
| D14 | 版本 | 工具版本 0.2.0 不变；`INPUT_CONTRACT_SCHEMA`、`RECEIPT_SCHEMA`、`VIEWER_REVISION` 不变 | engine 身份的变化由 `sourceInputSha256` 承担；版本是否提升留给下次发布决定 | `src/lib.rs:21-23`；`scripts/build-phoenix-diagram.mjs:22-23,335-341` |
| D15 | statechart 来源证据 | 本期不支持 `sources`/`meta.repository` | 图面状态是推导出来的，没有作者集合可以挂来源；`evidence.status` 恒为 `none` | `src/evidence.rs:132-142` |

### 0.3 ADR-lite

#### ADR-1：状态图检查在 Rust 内用有限 JSON 模型完成

- 状态：Accepted（用户 2026-11-04 选定）
- 背景与驱动：R-1、C-1。参考实现用 TLC（Java）检查 TLA+；Phoenix 消费端只有 Rust 引擎，普通命令不安装任何东西。
- 备选：
  - (a) 接入 TLA+/TLC：保留 TLA+ 的表达力，但消费端要装 Java 与 tla2tools、首次使用需要联网，安装器与许可链都要扩展。
  - (b) 只渲染不检查：最省事，但丢掉了自动找出违规路径的能力。
  - (c) 本方案。
- 决策：(c)。变量限于 bool、字符串枚举、整数区间；动作写成守卫 + 同时赋值；参数从有限字面量中取值；用 BFS 穷举。
- 正面后果：离线、确定，没有新依赖；判定与图面在同一引擎内完成，回执可以直接带上判定。
- 负面/中性后果：表达力小于 TLA+（没有集合、函数、序列、时序性质）；多进程模型要按进程写重复的动作。
- 重新评估触发：反复出现非要数组或集合才能建模的真实需求，或用户要求读入 TLA+。

#### ADR-2：表达式用 JSON 结构（单键运算对象），不引入文本表达式语言

- 状态：Accepted
- 背景与驱动：R-1、A-3。Phoenix 的所有诊断都用 JSON pointer 定位，输入由宿主写成 JSON。
- 备选：
  - (a) 文本小语言（如 `phase = "paid" && charges <= 1`）：更易读，但要写词法/语法分析，诊断也要按字符位置给出。
  - (b) 单键 JSON 运算对象（与 JSONLogic 同类）。
- 决策：(b)。schema 校验结构与元数，语义层做类型检查。
- 正面后果：不需要解析器；错误精确到 JSON pointer；schema 可以直接给宿主看。
- 负面/中性后果：模型 JSON 更冗长。
- 重新评估触发：在 V-8 角色模拟或后续使用中，表达式结构错误占 statechart 失败的多数。

#### ADR-3：嵌套/并行图用新的复合分层摆放 + 通用 router + 结构框，父状态以标题节点参与交互

- 状态：Accepted（A-2 由 M5 的 T-1 验证）
- 背景与驱动：R-2、R-3。现有十类都没有嵌套容器或并行区域；把父状态画成包住子节点的 `g[data-node-id]`，会触发 `composition/node-overlap`（`src/check.rs:672-680`）。
- 备选：
  - (a) 扩展 lifecycle v3：它按主路径分行排布、模块私有、直接读 `states/transitions/mainPath`（`src/layout/lifecycle/v3.rs:212-214`），没有容器模型；主路径约 7 个状态就会报 `lifecycle/too-wide`。加嵌套等于重写它的行模型。
  - (b) 把父状态画成节点并放宽重叠检查，同时让 router、锚点检查和 Reader 都支持「容器端点」：改动要穿过三处共享门。
  - (c) 新的摆放模块输出叶子盒与结构框；每个父状态在框头放一个普通节点（标题节点），作为可选中、可搜索、可作箭头起点的实体；走线交给 router（框可以穿越，但不可沿边走）。
- 决策：(c)。叶子与标题节点都是普通 `g[data-node-id]`，父状态与区域是 `data-graph-role="structural-frame"`；现有的重叠、边框、锚点检查和 Reader 关系机制原样适用。
- 正面后果：不改变共享检查门的语义；Reader 的选择、搜索和 Route/Reach 自动覆盖父状态。
- 负面/中性后果：父状态提升出的箭头从框头的标题节点出发，而不是从框边任意位置出发；摆放模块是新代码，规模约千行级。
- 重新评估触发：T-1 证明 router 在嵌套框中无法稳定通过构图门（分支路线见 §6.1）。

#### ADR-4：删除 legacy-2.17 合同后的输入处理

- 状态：Accepted（用户 2026-11-04 选定「一并移除」）
- 背景与驱动：R-6、C-9。今天未标记合同的五类文件都按 legacy-2.17 解读（`src/input.rs:183-191`）。
- 备选：
  - (a) 未标记的一律按 phoenix-v1 解读：同一份旧字节会被静默换一种语义，可能通过 schema 却画成另一张图。
  - (b) 原五类未标记时拒绝并给出改法，显式 phoenix-v1 照常；其余六类未标记仍按 phoenix-v1。
- 决策：(b)，诊断码 `input/contract-required`。文件写 `legacy-2.17` 时，合同枚举反序列化失败，给出既有的 `input/unknown-contract`；CLI 参数写 `legacy-2.17` 时，沿用参数校验的 `arguments/invalid`，消息写明该合同已不受支持。
- 正面后果：不会静默改变语义；现有 phoenix-v1 用法零改动。
- 负面/中性后果：老用户未标记的旧稿要先人工转换；旧的 legacy HTML 不能再通过 `check`（`src/check.rs:396-418` 改为拒绝未知合同）。
- 重新评估触发：确认仓外已没有未标记的五类旧稿后，可以把「未标记 = phoenix-v1」统一到全部类型。

### 0.4 职责与事实所有权

|  | statechart 参考实现（pstack，仓外只读） | Phoenix Diagram（本期） | Archify（原上游） |
| --- | --- | --- | --- |
| 拥有 | 自身的 TLC 管线与 Mermaid 输出 | 模型语义、穷举、投影、布局、渲染、检查、Reader、回执与 skill 文档 | 其原始代码与许可 |
| 不拥有 | **本期不运行、不打包、不比对它的代码或测试** | **不代建 TLC/Java 能力；不改任何其他 skill 或 Phoenix UI** | 本期完成后与 Phoenix 没有任何同步、校验或生成关系，只保留许可署名 |

一句话：statechart 与 Archify 都只是许可来源；判定正确性、回归基线和失败恢复全部由 Phoenix 自有的代码与 fixture 负责。

### 0.5 明确不在本期

- **TLA+/TLC、时序 PROPERTY（lasso / repeats），以及 CONSTRAINT / VIEW / SYMMETRY 语义**——明确放弃（ADR-1）。
- **集合、函数、序列、记录类型的变量，以及文本表达式语言**——达到 ADR-1/ADR-2 的重新评估触发时另立项。
- **自动「只画出错路径」模式**——见 D2；需要时另立项。
- **statechart 的 `sources`/仓库来源、品牌、架构比较**——见 D15。preview 走通用编译，不做专门适配。
- **版本号提升、CI 多平台 engine 构建、npm / GitHub 发布**——随下次发布另行操作（D14）。
- **修改 PRD、P1–P3 计划与记录、经验与历史图解**——见 C-7、C-9；本计划只在 SDLC 状态页登记替代关系。
- **转换 `docs/diagrams/*.json` 等仓内未标记的旧稿**——没有调用方会渲染它们（`git grep` 未找到），保持原样。
- **未明确要求的 NFR 系统性工作（性能基线、压测、调优）**——后续可选；本期按 §5.3 的数据结构做到线性开销。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **已核实**：十类图共用同一条管线（`src/delivery/prepare.rs:67-148`），依次是：
  1. `input::parse_input`：JSON → 合同 → schema → `meta.output` → `semantics::validate`（`src/input.rs:155-288`）
  2. 品牌、证据、`i18n::resolve`
  3. `layout::compile`（`src/layout/mod.rs:19-52`）
  4. `render::render_with_evidence`
  5. `check::check_html`

  finalize、deliver、review-record 没有类型分支（`src/delivery/receipts.rs:32`）。
- **已核实**：新五类通过 `TypeRenderer { root_attributes, defs, body, legend_glyph }` 接入渲染（`src/render.rs:22-38,885-909`）。最近的同构：
  - tree：关系由引擎推导，有自有 CSS 与内容检查（`src/layout/tree.rs:866-872`、`src/render/tree.rs`、`src/check/tree.rs`）。
  - waterfall：按状态着色，图例由目录生成（`src/layout/waterfall.rs:73-79,363-372`、`src/render/waterfall.rs:66-103,190-193`）。
- **已核实**：通用 router 接收调用方摆好的 `scene::Node` 盒子、`router::Edge` 和 `router::Frame`（框可以穿越，但不可沿边走）；`route_with` 接受 `RouterOptions`（`src/layout/router.rs:376-470,489`）。
  - architecture 先由 boundary 生成 Frame 再路由（`src/layout/architecture.rs:252-271`）。
  - erd 用 `route_with` 加自定义候选（`src/layout/erd.rs:746-756`）。
  - router 不画自环（`src/input/semantics.rs:439`、`src/layout/architecture.rs:818`）。
- **已核实**：Reader 从 SVG DOM 建图，依据 `g[data-node-id]`、`path[data-edge-from][data-edge-to][data-edge-key]` 和图例 `g[data-legend]`（`viewer/focus.js:60-131`、`viewer/node-finder.js:64-90`、`viewer/semantic-lens.js:45-68`）；元数据 JSON 只读取 `meta` 与 `sourceEvidence`（`viewer/template.source.html:566-620,890-947`）。
- **已核实**：`i18n::Locale::text` 缺 key 直接 panic（`src/i18n.rs:103-108`）。目录由组装器把 `viewer/locales/*.json` 与 `viewer/catalog-adaptations.json` 合成 `assets/catalogs.json`；五种 locale 的键集与占位符必须一致（`assemble-phoenix-diagram-reader.mjs:177-208`）。
- **已核实**：upstream 在仓库中的耦合点：
  - `upstream/` 共 81 个 tracked 文件，7.4 MB。
  - Rust 在 `src/schema.rs:6-28` 用 `include_str!` 读 legacy schema。
  - `build.rs:17-24` 把 upstream 与 legacy 示例列为构建输入。
  - `scripts/assemble-phoenix-diagram-reader.mjs:148-161` 解压归档来生成 Reader。
  - `scripts/check-phoenix-diagram-source.mjs:328-366` 校验归档。
  - `scripts/test-phoenix-diagram-native.mjs:25,45-47` 调用该校验，并运行 upstream 下的三份测试。
  - CI `.github/workflows/phoenix.yml:284` 运行 `check-phoenix-diagram-source.mjs --check`。
- **已核实**：legacy-2.17 分支遍布以下文件：`input.rs`、`schema.rs`、`cli.rs`、`check.rs`、`check/anchors.rs`、`render.rs`、`geometry.rs`、`geometry/composition.rs`、`layout/{lifecycle,lifecycle/presentation,architecture,architecture/presentation,router,sequence*,dataflow*,workflow*}.rs`，另有 `layout/legacy.rs`（683 行）。多数是 `if legacy {A} else {B}` 的形式，保留 B 即可。
- **已核实**：Archify MIT 全文与 JetBrains Mono OFL 在 `upstream/` 与 `viewer/` 各有一份字节相同的副本（`viewer/LICENSE.txt`、`viewer/JetBrainsMono-OFL.txt`）；`scripts/build-phoenix-diagram.mjs:256-261` 和 `test/phoenix-diagram/build.test.js:38-39` 读的是 upstream 那份。
- **缺口**：没有任何类型支持嵌套容器、并行区域或从模型推导图面 → 新增 `statechart` 全链路（§5、§6）→ 不做则 R-1–R-4 没有落点。
- **缺口**：直接删除 upstream 会导致三处失败：`build.rs collect()` panic、`include_str!` 编译失败、`reader-layout.test.mjs` 在模块加载时就失败（L30-42）→ 按 §5.6/§5.7 逐项改到不依赖 upstream → 漏做则构建与回归全部失败。

### 1.2 拓扑与文件清单

★ = 新增，✕ = 删除；其余为修改。

**引擎 `tools/phoenix-diagram/`**

- `src/input.rs`、`src/schema.rs`、`src/input/semantics.rs`、`src/cli.rs`：M1 删除 legacy；M3 接入 `Statechart`。
- `src/statechart/{mod,expr,model,explore}.rs` ★（M3）；`src/statechart/{chart,regions,report}.rs` ★（M4）。
- `src/layout/statechart.rs`、`src/layout/statechart/place.rs`、`src/render/statechart.rs`、`src/check/statechart.rs` ★（M5）。
- M3/M5 修改：`src/scene.rs`（`DiagramFacts::Statechart`）、`src/render.rs`、`src/check.rs`、`src/check/anchors.rs`、`src/evidence.rs`、`src/brands.rs`、`src/layout/mod.rs`、`src/delivery/receipts.rs`、`src/delivery/prepare.rs`。
- M1：删除 legacy 涉及的其余 src 文件（见 §5.6）；`src/layout/legacy.rs` ✕。
- `build.rs`（M1/M2）；`NOTICE.md`（M2/M5）。
- `assets/{brand-marks,brand-license-sources}.json`、`assets/{svg-definitions,node-card-style}.svg`（M2）；生成物 `assets/{template.html,catalogs.json}`（M2/M5 重新生成）；`assets/licenses/pstack-statechart-MIT.txt` ★（M5）。
- `viewer/`：以下 7 个文件 ✕——`{reader-layout,chrome-layout,css,export,focus}-adaptations.json`、`source-manifest.json`、`catalog-adaptations.json`。修改：`locales/*.json`、各片段头注释、`README.md`、`NOTICE.md`、`reader-layout.js`、`viewer.css`（M2/M5）。
- `upstream/` ✕（M2）。
- `tests/`：
  - legacy 用例与 fixture ✕/改（M1）；归档生成器 ✕（M2）。
  - `tests/maintenance/{source-check,prepare,license-sources}.test.mjs` ★（M2，从 upstream 迁入）。
  - `tests/statechart_model.rs`、`tests/statechart_chart.rs`、`tests/statechart.rs`、`tests/fixtures/statechart/*.json` ★（M3–M5）。
  - 枚举类型的测试（M3）。

**脚本与 Node 测试**：`scripts/{check-phoenix-diagram-source,assemble-phoenix-diagram-reader,build-phoenix-diagram,prepare-phoenix-diagram,test-phoenix-diagram-native}.mjs`；`test/phoenix-diagram/{brand-legal,build,reader-assets,skill}.test.js`。

**skill 与文档**

- 修改：`skills/phoenix-diagram/{SKILL.md,schemas/*.json,schemas/README.md,references/*.md}`。
- 新增：`schemas/statechart.schema.json` ★、`examples/minimal.statechart.json` ★（M3）、`references/statechart.md` ★（M6）。
- 删除：`examples/legacy.*.json` ✕。
- 复述同步：`README.md:228`、`docs/phoenix-diagram.md:3,99`、`bin/xgent-skills.js:243`。

**分发**：`dist/phoenix-diagram/**` 与 `skills/phoenix-diagram/ENGINE.json` 只通过 `build-phoenix-diagram.mjs --development --sync-dist` 重新生成（M2、M6），不手改。

**无需修改**：
- `bin/phoenix-diagram.js`、`package.json`、`scripts/run-tests.mjs`、`mise.toml`。
- 安装器白名单：`src/install/bundle.rs:79-99` 按模式匹配。
- `.github/workflows/phoenix.yml`：它调用的 `check-phoenix-diagram-source.mjs --check` CLI 保持不变。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类/适配器 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `statechart` ★（`src/statechart/mod.rs`） | `input/semantics.rs`（静态检查）、`layout/statechart.rs`、测试 | 两个入口：<br>`pub fn check_static(document: &Value) -> Diagnostics`<br>`pub fn analyze(document: &Value, locale: &Locale) -> Result<Analysis, Diagnostics>`<br>`Analysis { model: ModelResult, chart: Chart, stories: Vec<Story>, cards: Vec<ReportCard> }`<br>同一输入的输出确定 | 无 | 表达式编译与类型检查、穷举、规则判定、投影、条件文字、失真回退、提升、标记、叙述 | 进程内 | `tests/statechart_model.rs`、`tests/statechart_chart.rs`：经 `parse_input` + `analyze` 观察判定、路径与图结构 |
| `layout::statechart` ★ | `layout::compile` | `pub(super) fn compile(input, options) -> Result<CompiledDiagram, Diagnostics>`<br>输出节点不重叠、框正确嵌套、每条箭头两端落在节点边上 | 无 | 复合分层摆放、自环、起始标记、图例与报告卡片 | 进程内（复用 `layout::router`） | `tests/statechart.rs`：经 `compile` + `render` + `check_html` |
| `render::statechart` ★ | `render::type_renderer` | 一个 `TypeRenderer`；标记写成 `data-statechart-mark`、`data-statechart-bug-path` 属性与 class | 无 | 标记 CSS、图例字形 | 进程内 | 同上，用 HTML/DOM 断言 |
| `check::statechart` ★ | `check.rs:1049-1060` 分派 | 由 `facts` 重新推导标记、箭头与节点集合，并与 SVG 比较 | 无 | 防篡改 | 进程内 | 篡改 HTML 后 `check_html` 报错 |

不新增接缝或适配器：statechart 只有一个实现，测试直接经公开函数观察行为。`analyze` 是深模块，调用者只给 document 与 locale，就能得到判定、图与叙述。

## 3. 输入与产物兼容

1. **Contract（一次性，不可逆）**：本计划提交后，以下三类输入都会被拒绝，不提供转换器（用户决定「一并移除」）：
   - 未标记合同的五类旧稿；
   - 写着 `legacy-2.17` 的稿件；
   - 带 `data-input-contract="legacy-2.17"` 的旧 HTML。
2. 已有 phoenix-v1 稿件、HTML 和回执的形状都不变（D7、D8）；statechart 是新增类型，不影响旧输入。
3. **回滚**：代码级回滚即 `git revert` 本计划的提交，没有数据迁移。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| statechart 参考实现（`local/plugins-main/pstack/skills/statechart`，gitignored） | 已核实·足够 | 许可为 MIT（`local/plugins-main/pstack/LICENSE`）；语义要点已写进 §5.3–§5.5，实施时不需要访问它 | 只读参考；不复制其测试，不做差分；署名见 §5.9 | 其他机器上没有这个目录，不影响实施与验收 |
| 本机 Chrome/Chromium | 未核实·阻塞（仅收口证据）→ A-1 | `src/browser/discovery.rs:19-40`；`PHOENIX_DIAGRAM_BROWSER` | 收口前确认 | 缺失时记为环境缺失，不冒充通过 |
| CI `.github/workflows/phoenix.yml` | 已核实·足够 | L268 prepare、L284 `check-phoenix-diagram-source.mjs --check`、L287 开发构建、L300 `mbx test` | 不改 workflow；保持这几个 CLI 可用 | — |
| 发布脚本 `scripts/release-phoenix-ui.mjs` | 已核实·缺口（范围外） | `sourceInputSha256` 变化后，L149 拒绝 `--reuse-engines`；L163 要求重新生成 bundle | 不在本期；下次发布时由 CI 重建 engine | — |

## 5. 核心机制

### 5.1 statechart 输入合同（`skills/phoenix-diagram/schemas/statechart.schema.json` ★）

draft 2020-12。`$id` 用 M2 定下的 Phoenix 基址（§5.7 第 7 项）；`$ref` 复用 `common.schema.json#/$defs/{id,locale,translations,portableOutputPath,visualPreset,qualityProfile,cards}`。

```json
{
  "schema_version": 1,
  "diagram_type": "statechart",
  "meta": { "title": "订单支付", "output": "checkout.html", "input_contract": "phoenix-v1", "quality_profile": "showcase", "locale": "zh-CN" },
  "model": {
    "variables": [
      { "id": "phase", "values": ["cart", "paying", "paid", "failed"] },
      { "id": "charges", "range": [0, 2] },
      { "id": "gatewayUp", "values": [true, false] }
    ],
    "init": { "phase": "cart", "charges": 0, "gatewayUp": [true, false] },
    "actions": [
      { "id": "pay", "when": { "eq": [{ "var": "phase" }, "cart"] }, "set": { "phase": "paying" } },
      { "id": "charge", "when": { "and": [{ "eq": [{ "var": "phase" }, "paying"] }, { "var": "gatewayUp" }, { "lt": [{ "var": "charges" }, 2] }] },
        "set": { "charges": { "add": [{ "var": "charges" }, 1] } } },
      { "id": "confirm", "pick": { "result": ["paid", "failed"] }, "when": { "eq": [{ "var": "phase" }, "paying"] },
        "set": { "phase": { "param": "result" } } }
    ],
    "rules": [{ "id": "chargedOnce", "text": "一笔订单最多扣一次款", "holds": { "le": [{ "var": "charges" }, 1] } }],
    "deadlock": { "ends": { "in": [{ "var": "phase" }, ["paid", "failed"]] } }
  },
  "view": {
    "state": [
      { "when": { "eq": [{ "var": "phase" }, "cart"] }, "label": "购物车" },
      { "when": { "eq": [{ "var": "phase" }, "paying"] }, "label": ["结账", "支付中，已扣 {charges} 次"] },
      { "label": ["结账", "{phase}"] }
    ],
    "steps": [
      { "action": "charge", "when": { "eq": [{ "next": "charges" }, 2] }, "label": "网关又扣了一次" },
      { "action": "charge", "label": "网关扣款" },
      { "action": "confirm", "label": "支付结果返回：{result}" }
    ],
    "groups": [{ "when": { "ne": [{ "var": "phase" }, "cart"] }, "group": "结账" }],
    "colors": { "结账": "cyan" }
  }
}
```

字段规则。schema 管结构，`statechart::check_static` 管其余：

| 路径 | 规则 |
| --- | --- |
| `schema_version` | const 1 |
| `meta` | 必填 `title`、`output`；可选 `subtitle`、`locale`、`translations`、`visual_preset`、`quality_profile`、`input_contract`（const `phoenix-v1`）；不接受 `repository`、`views`、`animation`、`legend` |
| `model.variables` | ≥1 项；`id` 唯一（common id）。以下二选一：<br>• `values`：≥1 个互不相同的标量，且全为字符串、全为布尔或全为整数；<br>• `range: [lo, hi]`：整数，lo ≤ hi。<br>声明顺序即变量顺序 |
| `model.init` | 对象，键恰好是全部变量 id；每个值是域内标量，或域内标量的非空数组（表示可选的初值）。初始状态 = 按变量顺序、数组顺序的笛卡尔积，去重 |
| `model.actions` | ≥1 项；`id` 唯一。<br>• `pick` 可选：参数名 → 非空、同类型的标量数组；<br>• `when` 可选：布尔表达式，缺省为 true；<br>• `set` 可选：变量 id → 表达式，缺省 `{}`，即原地不动的一步 |
| `model.rules` | 可选；`id` 唯一且不等于 `deadlock`；`text` 必填非空，写成读者能认同的一句话；`holds` 是布尔表达式 |
| `model.deadlock` | 可选。`check` 缺省 true；`ends` 是布尔表达式，缺省 false；`text` 可选，缺省取目录 `statechart.rule.deadlock` |
| `view` | 可选；`state` 与 `regions` 至多出现其一；都缺省时，以每个模型状态的默认名（§5.3）构成扁平图 |
| `view.state` | 标签规则数组 `{when?, label}`，按顺序取首个命中。`label` 是字符串模板，或非空的字符串模板数组（数组 = 父状态路径） |
| `view.regions` | ≥2 项 `{name, state}`；`name` 非空且互不相同；`state` 规则同 `view.state` |
| `view.steps` | 可选数组 `{action, when?, label}`；`action` 必须存在；`label` 是字符串模板，或 `null`（隐藏该步）；没有命中的步骤用动作 id 作标签 |
| `view.groups` / `view.colors` | `groups`：`{when?, group}`，取首个命中。`colors`：group → D6 词表中的颜色。`groups` 用到的每个名字都必须出现在 `colors` 里，否则报 `statechart/group-color` |
| `cards` | 可选，common cards；排在引擎生成的报告卡片之后 |

模板规则：
- `{name}` 取同名参数（仅 steps 可用）或同名变量的显示值：字符串原样输出，整数为十进制，布尔为 `true`/`false`。
- 状态标签里的变量取该状态的值；步骤标签里的变量取步前值。
- 未知占位符报 `statechart/unknown-placeholder`；结果为空串报 `statechart/empty-label`。

### 5.2 表达式（`src/statechart/expr.rs` ★）

JSON 形状在 schema 的 `$defs/expr` 中递归定义；每个运算对象恰好一个键。

| 形状 | 结果类型 | 说明 |
| --- | --- | --- |
| `"s"` / `3` / `true` | 字符串 / 整数 / 布尔 | 字面量 |
| `{"var": id}` | 变量类型 | 当前状态的值；在 `set` 右侧即步前值 |
| `{"next": id}` | 变量类型 | 步后值；只能用在 `view.steps[].when` |
| `{"param": name}` | 参数类型 | 只能用在所属动作的 `when`/`set`，以及同一动作的 `view.steps` 条目 |
| `{"not": e}`、`{"and": [e, …≥1]}`、`{"or": [e, …≥1]}`、`{"implies": [a, b]}` | 布尔 | — |
| `{"eq"/"ne": [a, b]}` | 布尔 | 两侧同类型 |
| `{"lt"/"le"/"gt"/"ge": [a, b]}` | 布尔 | 两侧为整数 |
| `{"in": [e, [标量, …≥1]]}` | 布尔 | 列表元素与 e 同类型 |
| `{"add"/"sub": [a, b]}` | 整数 | 有符号 64 位，带溢出检查 |
| `{"if": [c, a, b]}` | a/b 的类型 | c 为布尔，a 与 b 同类型 |

- **编译期**：`check_static` 把 JSON 编译成 `enum Expr` 并做类型推导。诊断都带 JSON pointer：
  - `statechart/unknown-variable`、`statechart/unknown-param`
  - `statechart/next-outside-step`
  - `statechart/expression-type`：evidence 写期望类型与实际类型
  - `statechart/value-not-in-domain`：变量或参数与不在其域内的字面量比较、出现在 `in` 列表中，或被 `set` 赋为这样的字面量
  - `statechart/init-incomplete`、`statechart/reserved-rule-id`
  - `variables`/`actions`/`rules` 内重复 id 沿用 `input/duplicate-id`
- **运行期**：值用 `enum Val { Bool(bool), Int(i64), Str(u32) }`，字符串全局驻留为 u32。
  - `set` 的结果不在变量域内，报 `statechart/value-out-of-domain`。
  - `add`/`sub` 溢出，报 `statechart/arithmetic-overflow`。
  - 两者的 evidence 都包含：动作 id、参数值、变量、值，以及到达该状态的最短路径（用默认名表示）。
  - 按 BFS 顺序只报第一个，并停止分析。

### 5.3 穷举与判定（`src/statechart/explore.rs` ★）

1. **默认名**：
   - 单变量时，为该值的显示文本；多变量时，为 `id=值`，按变量顺序以空格连接。
   - 字符串值写成 JSON 字符串字面量（带引号），整数为十进制，布尔为 `true`/`false`。
   - `name_rank` = 默认名按 Rust `str` 字节序的名次，是所有并列的最终裁决。
2. **初始状态**：按 §5.1 做笛卡尔积并去重，按 `name_rank` 排序，深度为 0。
3. **BFS**：
   - 状态存为 `Box<[Val]>`，用 `HashMap<Box<[Val]>, u32>` 去重。
   - 每个出队状态，先按动作 id 的字节序、再按参数组合（按 `pick` 键的声明顺序与值顺序取字典序）枚举；`when` 为真即产生一步，自环也算一步。
   - 同一状态的出步再按（动作 id、目标 `name_rank`、生成序）稳定排序。
   - 目标首次被发现时，记录其 `via` 步与深度。违反规则的状态照常展开。
4. **上限**：已发现状态数达到 100,000 时（D3），停止发现新状态，`complete=false`，并发 `statechart/state-limit` 警告；evidence 带上限与已展开的状态数。
5. **规则与死锁**：
   - 规则：对每个被发现的状态（含未展开的边界状态），按声明顺序求每条 `holds`；为假即该状态违反该规则。
   - 死锁：已展开、零出步、且 `ends` 为假的状态，违反 `deadlock`；`deadlock.check=false` 时不判。
6. **最短出错路径**：
   - 按 BFS 发现序遍历状态，再按（作者规则的声明序，`deadlock` 排最后）遍历它违反的规则。
   - 每条规则取它遇到的第一个状态，路径是沿 `via` 链回到起点。
   - 所有 bug 按路径步数稳定排序。
7. **判定**：有 bug 的规则为 `breaks`；没有 bug 且 `complete` 为 `holds`；没有 bug 且未完成为 `not-checked`。死锁同理。
8. **复杂度**：每个状态的每个动作只求值一次；`state`、`steps`、`groups` 标签规则，对每个模型状态或步骤也只求一次。

### 5.4 图投影（`src/statechart/chart.rs`、`src/statechart/regions.rs` ★）

术语：
- **区域（region）**：`view.regions` 中的一项；没有 `regions` 时，只有一个无名区域。
- **路径（path）**：标签数组；字符串标签即单元素路径。

1. **标签**：每个模型状态在每个区域求一次首个命中的标签规则；没有命中时报 `statechart/unlabeled-state`（evidence：默认名与最短路径）。每个模型步骤求一次步骤标签。
2. **图状态**：
   - 每个区域内，以路径为键合并模型状态，按绘制序（深度、`name_rank`）创建；任一成员是起点，该图状态即为起点。
   - 某条路径既是叶子、又是另一条路径的前缀时，报 `statechart/leaf-is-parent`。
   - 父状态 = 叶子路径的所有真前缀；子项按首次出现的顺序归入父状态。
3. **箭头**：
   - 步骤按（起点绘制名次、终点绘制名次、动作名次）遍历。对每个区域，起止落在不同叶子时，产生该区域的一条候选箭头。
   - 按（起、止、标签）去重：不同动作只要标签相同，就共用一条箭头。
   - 起止相同的步骤：若在单区域图里且该步在出错路径上，画成自环；否则计入 `inside`，不画。
4. **隐藏**：
   - 标签为 `null` 的步骤不画，计入 `hidden`。
   - 该步若在任一出错路径上，报 `statechart/hidden-bug-step`（error），消息说明它在哪条规则的路径上、应当给出标签。
   - 不另外强制可达性或出边覆盖，与参考实现一致；相应规则写进 `references/statechart.md`。
5. **区域条件**（仅 ≥2 个区域时）：对每条箭头，取与起点共现的其他区域组合集合 `seen`；`takes(t)` 表示在组合 t 下存在同标签的这一步。
   - **and**：对每个其他区域 j 取已取值集。若「每个区域都在其已取值集内」恰好等于 `takes`，则对每个存在未取值的区域写一段：已取数 ≤ 未取数时写「是 X」，否则写「不是 Y」。两个区域时只有一段，更多区域时各段以「且」连接。
   - **or**：对每个区域，取「所有共现组合都已取」的值集。若「某区域在其值集内」恰好等于 `takes`，各段「是 X」以「或」连接。
   - 两者都不满足时记为 tuples（没有文字），触发第 7 步的回退。
   - 措辞（目录键见 §6.4）：
     - 两个区域：`[当 X 或 Y 时]`、`[除非 X]`。
     - 更多区域：`区域 是 X`、`是 X、Y 之一`、`不是 X`、`不是 X、Y 中任何一个`。
     - 值写成路径，以 ` / ` 连接并排序。
   - 作者标签已以 `]` 结尾的，不再加自动条件。
6. **区域说明**（≥2 个区域时）：按标签首次出现的顺序逐个判断：
   - 该标签从不在一步里同时改动 2 个以上区域 →「{step} 每次只改变 {regions} 中的一个」；
   - 否则 →「{step} 会同时改变 {pairs}」。
   - 同时改动过多个区域的标签，组成集合 `together`。
7. **区域失真检查**：依次做以下三项，取第一个失败作为原因：
   - (a) 各区域起点的笛卡尔积，每个组合都必须是真实存在的起点组合；
   - (b) 不得有 tuples 条件；
   - (c) 模拟：对每个真实组合，启用 `from` 匹配且条件成立的箭头；`together` 中的标签取各区域可走箭头的乘积，其他标签只移动单个区域。出现模型从未有过的移动，或模型的某个已绘制移动在图上走不出来，都算失败。

   失败时，每个状态改用单区域的父状态路径 `["区域名 标签", …]` 重新投影（不再有区域说明），并在概况卡片写明原因。**叙述（stories）仍用区域标注。**
8. **父状态提升**：
   - 父状态由外向内处理。离开某父状态的箭头按（标签、条件、终点）分组；若该父状态的叶子数 ≥2，且组的大小等于其叶子数，就把这组箭头换成一条从该父状态出发的箭头，追加在末尾。
   - 提升出的箭头不再提升。
9. **合并**：起、止、条件都相同的箭头合并为一条，标签按出现顺序用目录 `statechart.join.or` 连接。
10. **标记**：
    - 组合 = 一个模型状态在各区域所处图状态的元组。组合内只要有成员违反任何规则，该组合即为「脏」。
    - 区域图状态「可行」= 存在包含它的干净组合。
    - 多区域图中，以下两种情况会标记：状态有违反规则的成员且不可行；或某个脏组合的所有区域状态都可行，此时该组合的全部状态都标记。
    - 单区域图中，有违反规则的成员即标记。
    - 标记类：存在某条规则被**全部**成员违反，为 `breaks`；否则为 `may-break`。
    - 节点 `tag` 先列始终违反的规则 id，再列可能违反的规则 id（D5）。
    - 出错路径上每个模型状态所在的图状态，加 `bug-path`。
11. **阶段颜色**：图状态的全部成员恰好属于同一个 group 时才上色；成员 group 混合、或部分成员没有 group 时，不上色。
12. **叙述**：每个 bug 的路径逐步写成「{i}. {步骤标签}：{状态}」。状态文本 = 各区域的「名称 a / b」以「，」连接；单区域时没有名称前缀。
13. **确定性**：所有集合都按上述绘制序或 `name_rank` 排序。结果与 `actions` 的声明顺序无关；规则声明顺序只影响同长度 bug 的先后。

### 5.5 报告卡片与回执

**卡片**（`src/statechart/report.rs` ★）：由引擎生成，排在作者 `cards` 之前，文字全部取自目录（§6.4）：

1. **概况**（dot `slate`），依次为：
   - 「检查了 {states} 个状态、{steps} 步；图中 {chartStates} 个状态、{arrows} 条箭头。」
   - 未完成时：「到达 {limit} 个状态的上限后停止，未见违反的规则未检查。」
   - `inside>0` 时：「{count} 步停留在同一图状态内，图中不画。」
   - `hidden>0` 时：「view 隐藏了 {count} 步。」
   - 发生回退时：回退原因 +「因此图中用父状态代替区域。」
   - 区域说明（§5.4 第 6 步）。
2. **每个 bug 一张**（dot `rose`）：标题「违反：{rule text}（{rule id}）」；条目依次为「最短路径 {n} 步」、「0. {state}」、「{i}. {step}：{state}」。
3. **成立的规则**（dot `emerald`）：列出成立规则的 text；没有则省略这张卡。
4. **未检查的规则**（dot `amber`）：列出 not-checked 的规则；没有则省略这张卡。

**回执**（`src/delivery/receipts.rs`、`src/delivery/prepare.rs`）：
- `ValidationReceipt` 新增字段 `model: Value`，标注 `#[serde(default, skip_serializing_if = "Value::is_null")]`。
- 只有 statechart 会填写，值取自 `DiagramFacts::Statechart.model`。

```json
"model": {
  "states": 12, "steps": 29, "complete": true, "stateLimit": 100000,
  "chart": { "states": 7, "arrows": 10, "regions": ["会话", "错误次数"], "regionsFallback": null, "inside": 3, "hidden": 0 },
  "rules": [
    { "id": "signedInWhileLocked", "text": "锁定后不能登录", "outcome": "breaks",
      "path": [{ "state": "会话 未登录，错误次数 0" }, { "step": "密码错误", "state": "会话 未登录，错误次数 1" }] },
    { "id": "deadlock", "text": "运行不会卡住", "outcome": "holds" }
  ]
}
```

- `outcome` 取值为 `breaks | holds | not-checked`。
- finalize 的退出码与交付状态不受规则判定影响：图上显示出错路径本身就是一次成功的交付。

### 5.6 删除 legacy-2.17（M1）

1. **`src/input.rs`**：
   - 删除 `InputContract::Legacy217`（D7）。
   - `has_legacy_contract` 改名为 `is_original_five`：`check.rs:1195`、`semantics.rs:140` 仍需要判断「原五类」。
   - 合同解析顺序改为：显式参数 → 文件标记 → `is_original_five()` 为真时报 `input/contract-required` → 其余类型取 phoenix-v1。该诊断的 path 为 `/meta/input_contract`；message 说明 legacy-2.17 已不受支持，需要转成 phoenix-v1 并写上 `meta.input_contract`，若 JSON 已经是 phoenix-v1，则传 `--input-contract phoenix-v1`。
   - 删除 L238-244 的 legacy 去标记逻辑；L261 的 `if PhoenixV1` 改为无条件执行。
   - 合同只剩一个取值后，显式参数与文件标记不可能冲突，`input/contract-conflict` 检查随之删除。
   - `compatibility()`（L398-446）删除 `/meta/animation` 的 `ignored-display`，保留 `brand` 与 `/meta/views`。
2. **`src/schema.rs`**：删除 `LEGACY_COMMON`/`LEGACY` 及其分派（L6-28、74-79、88、130-135）。
3. **`src/cli.rs`**：HELP（L21）、`--input-contract`（L101-108）以及 compare 的 `--base/--head-contract`（L444-451）都只接受 `phoenix-v1`。
4. **`src/check.rs`**：L396-399 删去「未知值回退为 Legacy217」，只映射 `phoenix-v1`；未知值已由 L409-418 报 `artifact/input-contract`，该处删除 `"legacy-2.17"` 即可。
5. **渲染、几何、布局**：各文件删除 legacy 分支、保留 phoenix-v1 分支：
   - `render.rs`：L111-120、128-141、856-873、1034-1066、1173。
   - `check/anchors.rs`：L335、341、530、560。
   - `geometry.rs`：L26、116-120。
   - `geometry/composition.rs`：L281-310、1373、1397。
   - `layout/lifecycle.rs`：L11-12、178-181，连同只被 legacy 使用的 `point()`（L27-32）。
   - `layout/lifecycle/presentation.rs`：L45-158。
   - `input/semantics.rs`：L177、236-245。
   - `layout/architecture.rs` 及其 `presentation.rs`。
   - `layout/router.rs`：`legacy` 字段（L448）、`default_sides`（L254-259）、L498-559、629-640、1370-1378 等。
   - `layout/sequence*.rs`；`layout/dataflow*.rs`（含 `Context::legacy()`，L231-233）；`layout/workflow*.rs` 中的 `!= PhoenixV1` 分支。
   - `layout/legacy.rs` ✕；`layout/mod.rs::clear_sigil_labels` 删除合同参数。
   - 只为 legacy 存在的参数与函数一并删除；`*_for_contract` 函数并回它们的 phoenix-v1 版本，若已有同名的无合同函数则合并。
6. **保留**（这些不是 legacy 合同代码）：
   - `segments.rs::legacy_label`：两条路径共用。
   - phoenix-v1 非自动 sequence 输出的 readerFit `"legacy"`，及其在 `browser/observation.rs:81,385`、`viewer/reader-layout.js:40` 中的处理。
   - workflow 的 `LEGACY_COLUMNS`；erd/class 的 `legacy_side`。
7. **Reader 中的 legacy 代码**：`viewer/reader-layout.js:39` 对 `data-input-contract === 'legacy-2.17'` 的判断，以及 `viewer/viewer.css:2428-2434` 中只作用于 legacy 的规则，都放到 **M2** 删除。在 M1 时这些片段仍受组装器的逐 hunk 审计约束。
8. **测试与 fixture**：
   - 删除 `tests/fixtures/input/legacy-2.17/`（14 个）、`tests/fixtures/{architecture,lifecycle}/legacy-2.17-*.svg`（19 个），以及只测 legacy 的测试函数。
   - 共享的 oracle JSON 用一次性命令删去 legacy 段：architecture、architecture/presentation、composition、dataflow、lifecycle、sequence、workflow、workflow/pin、input/schema-oracle。`tests/support/input_corpus.rs:99` 中 schema-oracle 的摘要随之更新。
   - `input_contract.rs` L141 改写为 V-1 的拒绝用例。
   - 用 `include_bytes!`/`include_str!` 指向 upstream legacy 的测试，改为指向 phoenix-v1 的等价物，或直接删除：`composition.rs:232-233`、`remaining_lifecycle.rs:296-298`、`architecture.rs:763`、`brands.rs:7`、`input_contract.rs:9,291`、`render_check.rs:590,594`、`render_remaining.rs:213`。
9. **skill 与构建**：
   - 删除 `examples/legacy.*.json`（5 个）；`build.rs` 删除 L18 与 L20-24。
   - 修改以下文档：
     - `SKILL.md:75`
     - `references/compatibility.md`：表格改为「全部类型都是 phoenix-v1；未标记的原五类报 `input/contract-required`」，并删除 `/meta/animation` 一项
     - `references/authoring-defaults.md:15,17,25`
     - `references/architecture-compare.md:12`
     - `references/preview.md:15`
     - `references/extending-types.md:7,12`
   - 删除 `test/phoenix-diagram/skill.test.js:77-91`、`build.test.js:28-29,72` 中的 legacy 断言。

### 5.7 删除 upstream 与改用 Phoenix 标识（M2）

1. **删除目录**：`tools/phoenix-diagram/upstream/` ✕（81 个文件）。其中三份测试里与归档无关的用例，迁到 `tools/phoenix-diagram/tests/maintenance/` ★：
   - `prepare.test.mjs`：`preparationCommands`；
   - `source-check.test.mjs` L97-173：ICU 文本与时间来源；
   - `license-sources.test.mjs`：Rust 许可。
2. **`scripts/check-phoenix-diagram-source.mjs`**：
   - 删除 `gunzipSync`、L12-16 的常量、`referenceRoots`、`treeManifest`、`readManifest`、`archiveManifest`、`verifyFrozenSources`。
   - 新增 `export function verifySources()`：依次调用 `verifyNativeTextSources(identity)`、`verifyNativeTimeSources(identity)`、`verifyRustLicenseSources()`、`verifyBrandLicenseSources()`；项目根固定为 `workspace`，身份取文件内的常量。
   - `verifyBrandLicenseSources` 删除 L310 默认读 SOURCE.json 的逻辑，以及 L313-324 对归档的比对。
   - `--check` CLI 的输出 `{status:'verified', sources, tools}` 保持不变（CI L284 依赖它）。
3. **其余脚本**：
   - `scripts/prepare-phoenix-diagram.mjs` L10/102/105：改用 `verifySources`。
   - `scripts/build-phoenix-diagram.mjs`：
     - L13/414/432：改用 `verifySources`；构建回执字段 `frozenSources` 改名为 `sources`。
     - L227：品牌 notice 位置改为 `tools/phoenix-diagram/assets/${row.path}`。
     - L230：`source` 取更新后的 `catalog.source`。
     - L256-261：Archify 与 JetBrains Mono 的许可来源改为 `tools/phoenix-diagram/viewer/LICENSE.txt` 和 `viewer/JetBrainsMono-OFL.txt`。
     - L315：删除 `upstream/SOURCE.json`。
     - L332-333：`viewerIdentity` 删除 `sourceManifest`。
   - `scripts/test-phoenix-diagram-native.mjs`：L10/25 改用 `verifySources`；L45-47 改为运行 `tests/maintenance/*.test.mjs`。
   - `build.rs` 删除 L17，并改写 L7-9 的注释。
4. **`scripts/assemble-phoenix-diagram-reader.mjs`**（D9/D10）：
   - 删除 `archiveMembers`、`verifyReaderLayout`/`verifyChromeLayout`/`verifyReaderCss`、export/focus 的审计、`definitionsFrom`、`nodeCardStyleFrom` 和 `namespaceSource`。
   - `catalogsAt` 只读 `viewer/locales/manifest.json` 和各 locale；保留键集/占位符一致性检查和 archify 守卫，删除 L184-185 的替换与 adaptations 合并。
   - 输出只剩 `template.html` 与 `catalogs.json`。
   - 迁移顺序：
     1. 先用旧流程生成一次资产；
     2. 把 `catalog-adaptations.json` 并入各 locale，并按 L184-185 的同一规则把 locale 中的「Archify」「ARCHIFY」改写；
     3. 确认新流程生成的两份资产与旧流程逐字节相同；
     4. 删除旧文件；
     5. 执行 §5.6 第 7 项。
5. **viewer**：
   - ✕：5 个 `*-adaptations.json`、`source-manifest.json`、`catalog-adaptations.json`。
   - 头注释：14 个 `viewer/*.js`、`viewer.css`、`template.source.html`、`assets/svg-definitions.svg`、`assets/node-card-style.svg`。把「Derived from Archify 3.1.0 … frozen content SHA-256 …/adaptations.json」改为「Derived from Archify 3.1.0 viewer/focus.js (MIT); Phoenix Diagram modifications.」这一格式（文件名换成各片段对应的原文件），保留版权行与「Full license: LICENSE.txt」。
   - 改写 `viewer/README.md`，使其描述当前的组装方式；`viewer/NOTICE.md` 删去对快照与审计的描述。
   - 最后重新生成资产。
6. **`tests/`**：
   - 删除归档生成器：
     - `tests/generate-input-oracle.mjs`
     - `tests/fixtures/{architecture,class,composition,dataflow,erd,geometry-quality,lifecycle,svg-path,timeline,tree,waterfall,workflow}/generate.mjs`
     - `tests/fixtures/workflow/{generate-pins.mjs,pin-cases.mjs}`
     - `tests/fixtures/sequence/generate-oracles.py`
     - `tests/fixtures/format/generate.mjs`
   - `tests/fixtures/text-segmentation/generate.mjs` 删去 L119-129；`src/format/time/system_v.rs` L4 的注释改为数据来源说明。
   - `tests/viewer/reader-layout.test.mjs`：去掉 L30-42 的归档读取和 L164-178，删除 legacy 用例。
   - `test/phoenix-diagram/{brand-legal,build,reader-assets}.test.js`：按研究清单删去归档与 SOURCE.json 的夹具和断言。`reader-assets` 中依赖组装审计的用例，改为对 `--check` 输出与生成资产做断言。
7. **身份（R-7）**：
   - **schema**：11 份 schema 的 `$id` 改为 `https://github.com/XGENT-ai/skills/schemas/phoenix-diagram/<文件名>`，`title` 改为「Phoenix Diagram …」。`$ref` 是相对引用，统一基址即可；`src/schema.rs:101-123` 按 `$id` 注册 schema。
   - **skill**：`schemas/README.md` 改写来源段，删去 L19-23 的 oracle 命令；`SKILL.md` frontmatter 删除 `metadata.based_on`。
   - **用户可见文本**：`src/schema.rs:91,93` 的错误文本；`src/compare/canonical.rs:626` 中的「Archify cannot prove」改为「Phoenix Diagram cannot prove」。
   - **源码首行注释**：「Ported from the frozen Archify …; see NOTICE.md」改为「Derived from Archify (MIT); see NOTICE.md」。涉及：`check/path.rs`、`evidence/location.rs`、`format.rs`、`geometry*.rs`、`i18n.rs`、`input/semantics.rs`、`layout/{class,class/members,dataflow,erd,erd/*,lifecycle,timeline,tree,waterfall,router}.rs`、`render/{class,erd}.rs`、`tests/support/svg_oracle.rs`。
   - `input.rs:53,239,320` 的注释随 §5.6 一起改写；`src/browser/probe.js:1-2` 保持不变。
   - 品牌 JSON 按 D13 修改，并级联更新摘要。
   - **保留不改**：`references/viewer-runtime.md:5,27` 中「不读取旧 Archify 偏好」「不使用 Archify fallback」两句。它们描述的是行为，不构成关联。
8. **`tools/phoenix-diagram/NOTICE.md`**：
   - 删除涉及 SOURCE.json、快照、legacy oracle、归档校验的句子（L3、16、18、22、28、30、32）。
   - 改为一段说明：「Phoenix Diagram 的部分源码、schema 与阅读器派生自 Archify（MIT），保留以下署名」，后接版权块，并写明许可全文位于 `viewer/LICENSE.txt`、`viewer/JetBrainsMono-OFL.txt`。
   - ICU、SystemV、Rust 许可、品牌目录各段保留；其中「登记在 upstream/SOURCE.json」改为「由 `scripts/check-phoenix-diagram-source.mjs` 校验」。

### 5.8 skill、guide 与复述

M3 负责 `cli.rs`、schema 与最小示例；其余在 M6 完成。

- **`src/cli.rs`**：
  - HELP 列入 statechart。
  - `guide()` 增加 `(DiagramType::Statechart, ["statechart", "state machine", "model check", "invariant", "counterexample", "deadlock", "状态机", "模型检查", "不变量", "违规路径", "死锁", "反例"])`。lifecycle 的 `"state"`、`"状态"` 不动；两者同时命中时，由 SKILL 中写明的边界区分。
  - `doctor` 自动包含新类型。
- **`SKILL.md`**：
  - description 的前 100 字内加入「状态机模型检查」作触发词，同时满足 C-5 的字数与 14 词要求，逐字核对。
  - 图型表增加 `statechart` 一行和示例链接。
  - 写明与 lifecycle 的边界：「要判断流程会不会违反规则、找出错路径」用 statechart，「画出已知的状态流」用 lifecycle。
  - 在只支持 phoenix-v1 的类型列表中加入 statechart。
  - 交付答复段增加：
    - 逐条列出 `model.rules` 中 breaks 的规则、作者写的句子与路径；
    - 说明 not-checked 与 `complete:false`、隐藏了哪些步骤、模型省略了什么；
    - 模型描述的是代码时，按步复现每条出错路径；复现之前，先假定模型有误。
- **`references/statechart.md`** ★：
  - 建模与 view 写法：
    - 规则写成读者认同的句子，不为让检查通过而放宽规则；
    - 用 `ends` 声明可以停下的终态，域保持小；
    - 箭头控制在约 20 条以内；
    - 状态标签写读者看到的，步骤标签写用户经历的；
    - `[条件]` 的写法，以及区域图中的限制；
    - 隐藏步骤的三条守则：每个图状态都能从起点经箭头到达；模型有出步的图状态至少保留一条出箭头；一条箭头的步骤要么全隐藏、要么全保留；
    - 合并标签时，不得把违反不同规则的状态合成一个。
  - 诊断码对照表。
  - 回执 `model` 字段说明。
- **`examples/minimal.statechart.json`** ★（M3）：两个变量、一条会被违反的规则、`ends`，带一个父状态。
- **`schemas/README.md`**：改为十一类，并说明 statechart 没有外部来源。
- **复述同步（C-4）**：
  - `references/{compatibility.md,authoring-defaults.md,preview.md,viewer-runtime.md,brand-marks.md,extending-types.md}`：「十类」改为「十一类」，并补上 statechart 的例外（无品牌、无来源、无比较）。
  - `extending-types.md`：删除「保留冻结 oracle」，改为「新类型的 fixture 由 Phoenix 自有」。
  - `docs/phoenix-diagram.md:3,99`、`README.md:228`、`bin/xgent-skills.js:243`。
- **`scripts/build-phoenix-diagram.mjs:105-108`**：必备 schema 列表加入 `statechart.schema.json`。
- **Node 测试的类型枚举**：`test/phoenix-diagram/skill.test.js:10` 的 `types` 与 L47-71 的每类集合/`schema_version` 断言、`test/phoenix-diagram/build.test.js:132` 加入 statechart；statechart 的最小示例以 `model.variables`/`model.actions` 作为集合，不套用节点/关系断言。

### 5.9 statechart 署名（C-6，M5）

- 新增 `assets/licenses/pstack-statechart-MIT.txt` ★，内容为 `local/plugins-main/pstack/LICENSE` 原文。
- `NOTICE.md` 增加一段：statechart 的投影、区域条件、失真回退、父状态提升与报告结构，派生自 pstack 的 `skills/statechart`（MIT，Copyright (c) 2026 Lauren Tan）。
- `src/statechart/mod.rs` 首行注释：`Chart semantics derived from pstack skills/statechart (MIT); see NOTICE.md.`
- `build-phoenix-diagram.mjs` 运行期许可列表（与 L256-261 同处）加入该条，使其进入 `THIRD-PARTY.json` 和 `THIRD-PARTY-LICENSES/`。

## 6. 布局、渲染与 Reader

### 6.1 复合分层摆放（`src/layout/statechart/place.rs` ★）

输入：`Chart`（区域 → 叶子、父状态树、箭头）。输出：叶子与标题节点的 `Rect`、父状态框与区域框的 `Rect`、起始叶子列表。

1. **尺寸**：
   - 叶子沿用 lifecycle 的状态尺寸规则：宽 140–220、高 64，文字超过 212px 报错（`src/layout/lifecycle/v3.rs:182-209`），诊断码为 `statechart/state-text-too-long`。
   - 被标记的叶子在第二行显示 `tag`；父状态内的叶子只显示路径的末段。
   - 标题节点：高 28，宽 = 文字宽 + 24，至少 72。
2. **自底向上**：
   - 每个容器（父状态；区域是顶层容器）的直接子项，是叶子或子父状态。
   - 对所在区域的每条箭头，求起点与终点的最近公共祖先；祖先等于本容器时，产生一条「子项 → 子项」的诱导边。
   - 起点是被提升的父状态时，子项取该父状态；起点就是本容器标题节点的箭头，不参与排层。
3. **排层**：子项的层号 = 它所含叶子的最小 BFS 深度（§5.3），在容器内压缩为连续的列号；回边自然是从右向左。
4. **列内排序**：初始顺序为绘制序；按诱导边做 4 轮中位数（median）扫描，左右交替，并列时按绘制序，结果确定。
5. **坐标**：
   - 列从左到右排；列宽 = 列内最宽的子项。
   - 列间距 = max(88, 相邻两列之间诱导边的最长标签宽 + 40)。
   - 列内子项间距 40，垂直居中。
   - 容器尺寸 = 内容包围盒 + 内边距 20 + 顶部标题带 44；首列含起点叶子时，左内边距改为 44，以容纳起始标记。
   - 标题节点放在 (框.x+14, 框.y+8)。
6. **区域**：
   - ≥2 个区域时，各区域框等宽（取最宽者），自上而下堆叠，间距 28；框头写区域名（用 `Container.label_plate`，不是节点）。
   - 单区域时不画区域框。
7. **自顶向下**：把相对坐标累加成绝对坐标；画布 = 全部框与节点的包围盒 + 外边距 24。

**走线**（`src/layout/statechart.rs` ★）：
- 调用 `router::route_with(&nodes, &edges, &frames, false, RouterOptions::default())`（M1 删除 legacy 后 router 不再接收合同参数）：
  - `nodes` = 叶子 + 标题节点；
  - `edges` = 非自环箭头，`raw` 合成为 `{"id","label","variant"}`；出错路径上的箭头用 `variant:"emphasis"`；
  - `frames` = 父状态框（radius 12）与区域框（radius 8）。
- 标签用 `router::Label::new` 与 `place_labels` 放置。
- 自环（只出现在单区域图的出错路径上）不交给 router，作为带显式点的关系画出，`independent_ports: true`。路线：从叶子上边距右端 24 处出发，向上 20；向右越过右边 20；向下到上边以下 16；再回到右边。
- 起始叶子左侧用 `render.rs::initial_marker` 画起始标记。

**A-2 / T-1 的分支路线**：T-1 用 F-3（父状态 + 提升）与 F-4（区域），只完成摆放 + 走线，然后运行 `check_html`。

- 全部通过 → 按本节继续实施。
- 只有个别构图门失败（标签放不下、交叉、贴边）→ 调整列/行间距或 `RouterOptions`（`component_gap_px`、`crossing_free_grid_first`、`label_reservations`），并在记录中写明所用参数。
- 框内路由结构性失败（router 无法在嵌套框之间找到路线）→ 停下，就地修订本节，改为另一条路线：每个容器单独路由其内部箭头；跨容器的箭头经容器边界上的端口分两段路由。在记录中写明偏差（@implementation_route）。

### 6.2 渲染（`src/render/statechart.rs` ★）

- **`root_attributes`**：` data-statechart-ui="" data-node-kind-derived=""`；readerFit 沿用 erd 对自动画布的处理。
- **`defs`**：写一段 `<style>`：
  - `breaks`：`[data-statechart-mark="breaks"] rect[data-graph-role=node-body]` 设 `stroke: var(--security-stroke); stroke-width: 2.5px`，文字设 `fill: var(--security-stroke)`。
  - `may-break`：同色，加 `stroke-dasharray: 6 3`。
  - 出错路径：用 `var(--arrow-emphasis)` 画 2.5px 外环。外环画成第二个圆角矩形，不用 CSS outline，以便导出时保留。
- **`body`**，依次写入：
  1. `write_container_frames`：父状态框 `data-composition-frame-kind="statechart-parent"`，区域框 `statechart-region`；
  2. `write_relation_paths`；
  3. 自写的 `write_node`（照 `render/tree.rs::write_node`）：节点的 `data-node-kind` 为 `state` 或 `parent`；`data-node-context` 用目录 `node.context.statechart.*` 生成，形如「{n} 个模型状态 · 违反 R1；可能违反 R2」；
  4. 起始标记；
  5. `write_container_labels`、`write_relation_labels`。

  `render.rs` 中这几个通用写入函数按需改为 `pub(super)`。
- **`legend_glyph`**：按 facts 中图例条目的 `{mark}` 或 `{tone}` 画小方块（照 `render/waterfall.rs:190-193`）。
- **分派**：
  - `render.rs::type_renderer` 增加 `DiagramType::Statechart => Some(statechart::RENDERER)`。
  - 以下几处照 `DiagramType::Erd` 的分支处理：`render.rs:874` 的 root 属性、`brands.rs:188`（无品牌集合）、`check.rs:262`（折线）、`check/anchors.rs:258,332,338`。

### 6.3 检查（`src/check/statechart.rs` ★）

- **数据来源**：由 `DiagramFacts::Statechart` 重新推导期望值，facts 包括：
  - `regions`
  - `states`：id、路径、kind、mark、bug_path、tone、start
  - `arrows`：id、from、to、label
  - `model`
- **比较内容**：推导出的节点集合与 kind、`data-statechart-mark`/`data-statechart-bug-path`、起始标记，以及箭头的 `data-edge-from/to/label`，与 SVG 逐项比较。
- **报错**：不一致时报 `statechart/artifact-mismatch`，evidence 写出节点或箭头的 id。
- **接入**：挂到 `check.rs:1049-1060` 的分派。

### 6.4 Reader 与 i18n

- 不新增 Reader 模块。叶子与标题节点是普通节点，选择、搜索、详情、Route/Reach、导出都照常工作；框不可选，与 architecture 的 boundary 一致。
- 新增目录键，写进 `viewer/locales/{en,zh-CN,zh-TW,es,ko}.json`。五种语言都写真实译文，占位符保持一致：
  - 图例与节点：`legend.statechart.{breaks,may-break,bug-path,start}`、`node.context.statechart.{members,breaks,may-break}`
  - 规则与卡片：`statechart.rule.deadlock`、`statechart.card.{summary,rule-breaks,rules-hold,rules-not-checked}`
  - 概况与回退：`statechart.summary.{counts,incomplete,inside,hidden,fallback}`、`statechart.fallback.{starts,condition,move-extra,move-missing}`
  - 区域说明与路径：`statechart.note.{one-at-a-time,together}`、`statechart.path.{length,start,step}`
  - 条件与连接词：`statechart.when.{either,unless,is,one-of,is-not,none-of,and,or}`、`statechart.join.{or,list}`
  - 图型描述：`diagram.description.statechart`

  中文句式以 §5.4–§5.5 为准，英文句式以参考实现为准。
- `assets/catalogs.json` 与 `assets/template.html` 经组装器重新生成。

## 7. NFR、安全与运行保障

用户没有明确要求的 NFR，本节不建专项验证。statechart 只处理输入文档本身，不访问其他资源，也不联网；模型规模由 D3 的上限约束，内存与时间随已发现的状态数线性增长。

## 8. 失败模式、发布与回滚

不引入新的高影响运行期失败模式。statechart 是纯计算：失败以诊断返回（§5.2、§5.4）；状态爆炸由 D3 的上限截断，并在回执与卡片中如实标注。

发布与回滚：本计划只做本地开发构建并同步 dist（D14）；回滚即 revert 本计划的提交。

## 9. 验证

### 9.1 行为与接口验证

**环境**：
- 在仓根执行；工具已按 `node scripts/prepare-phoenix-diagram.mjs --prepare` 准备好（缺失即失败，C-2）。
- 下文 `CARGO` 表示 `mise exec --no-deps -- mbx`，`MF` 表示 `--manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline`。

**批内最小反馈**：
- 单个测试：`$CARGO test $MF --test <文件> <过滤词>`。
- 只编译不运行：`$CARGO test $MF --no-run`，用于删文件、改引用之后。
- Node 单文件：`mise exec --no-deps -- node --test <文件>`。
- 整 crate 测试和 clippy 的耗时未知，只在批次退出时运行。

**statechart fixture**：位于 `tools/phoenix-diagram/tests/fixtures/statechart/`，是手写的模型，期望值在测试中直接断言。

| Fixture | 模型与覆盖点 | 期望结果 |
| --- | --- | --- |
| F-1 `diehard.json` | 大桶 0..5、小桶 0..3，六个动作（装满/倒空/互倒），规则「大桶不会正好 4 升」；无 view，使用默认名 | breaks，最短路径 6 步 |
| F-2 `checkout.json` | 扁平图；`ends` 为已付/失败；两个阶段颜色；一个隐藏步骤；出错路径上有一个自环 | `chargedOnce` breaks |
| F-3 `login-parents.json` | 父状态「已登录」含三个子状态，每个子状态都能「退出」 | 「退出」提升为父状态的箭头 |
| F-4 `login-regions.json` | 区域「会话」「错误次数」 | 自动条件 `[除非 已锁定]`；规则「锁定后不能登录」breaks；两句区域说明 |
| F-5 `two-phase.json` | 三区域两阶段提交 | 触发区域失真，回退为父状态，回退原因可读；叙述仍用区域标注 |
| F-6 `deadlock.json` | 一个未声明 `ends` 的停止状态；再加 `ends` 跑一次同一模型 | 不加 `ends` 时 `deadlock` breaks；加上后 holds |
| F-7 `limit.json` | 三个 0..99 的整数变量自由加减 | `complete:false`；未见违反的规则为 not-checked；发出 `statechart/state-limit` 警告 |
| F-8 `mixed-groups.json` | 合并后成员 group 混合的图状态 | 该图状态不上色；出现 `may-break` 标记 |

| 检查 ID | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | R-6，四类输入：<br>• 未标记的原五类，各报 `input/contract-required`<br>• 文件标记或 `--input-contract phoenix-v1`，照常解析<br>• `legacy-2.17` 标记报 `input/unknown-contract`；CLI 参数报 `arguments/invalid`<br>• 未标记的 erd，仍按 phoenix-v1 解析<br>另外：含 legacy 合同的 HTML 被 `check` 拒绝；其余既有测试全部通过（`build.test.js` 的 dist 一致性用例依赖 M2 的 `--sync-dist`，在 V-3 复验） | `$CARGO test $MF --test input_contract`<br>`$CARGO test $MF`<br>`$CARGO fmt --manifest-path tools/phoenix-diagram/Cargo.toml -- --check`<br>`$CARGO clippy $MF --all-targets -- -D warnings`<br>`mise exec --no-deps -- node --test test/phoenix-diagram/skill.test.js test/phoenix-diagram/build.test.js` | M1：src、tests、skill 中的 legacy 删除全部完成后 |
| V-2 | R-5、R-7、C-6：<br>• 没有 upstream 时，来源检查、组装检查、构建输入与 Node 维护测试都通过<br>• 组装产物与删除前相比，只差 §5.6 第 7 项<br>• 许可全文齐全<br>• `git ls-files tools/phoenix-diagram/upstream` 输出为空<br>• 下列 grep 只剩：许可署名、来源说明，以及 §5.7 第 7 项中标明保留的行为说明 | `mise exec --no-deps -- node scripts/check-phoenix-diagram-source.mjs --check`<br>`mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check`<br>`mise exec --no-deps -- node --test tools/phoenix-diagram/tests/maintenance/source-check.test.mjs tools/phoenix-diagram/tests/maintenance/prepare.test.mjs tools/phoenix-diagram/tests/maintenance/license-sources.test.mjs tools/phoenix-diagram/tests/viewer/reader-layout.test.mjs test/phoenix-diagram/brand-legal.test.js test/phoenix-diagram/build.test.js test/phoenix-diagram/reader-assets.test.js test/phoenix-diagram/skill.test.js`<br>`$CARGO test $MF`<br>clippy（同 V-1）<br>`git grep -n -i -e archify -e 'upstream/' -- tools/phoenix-diagram skills/phoenix-diagram scripts test/phoenix-diagram README.md docs/phoenix-diagram.md` | M2：§5.7 第 1–8 项齐备后 |
| V-3 | 阶段 A 收口（R-5、R-6、R-7）：<br>• 全量回归通过<br>• 开发构建完成并同步 dist<br>• 十类最小示例都能在真实浏览器中交付，并实际看图 | 1. `npm run test:diagram`：聚合，含 fmt、clippy 与全部 Rust/Node/Reader 测试，耗时未知<br>2. `mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/statechart-a --sync-dist`<br>3. `E=local/phoenix-diagram/statechart-a/phoenix-diagram-<platform>-<arch>`（本机为 `darwin-arm64`）<br>4. 对每一类运行 `"$E" finalize <type> skills/phoenix-diagram/examples/minimal.<type>.json local/phoenix-diagram/statechart-a-out/<type>.html --json`<br>5. 打开回执列出的四张截图看图，写好审核记录后运行 `"$E" review-record <html> --record <审核记录> --json`<br>6. 确认 `dist/phoenix-diagram` 中没有 legacy 示例，`THIRD-PARTY.json` 中没有 `upstream/` 位置 | M2 收口（阶段 A）；先确认 A-1 |
| V-4 | R-1：<br>• F-1 路径为 6 步；F-6 的死锁与 `ends`；F-7 的上限与 not-checked<br>• 打乱 `actions` 的声明顺序，结果逐字节相同<br>• §5.2 的每个编译期诊断各有一例（含拼错的字面量触发 `value-not-in-domain`）；运行期诊断 `value-out-of-domain` 带路径<br>• 初值数组与 `pick` 参数产生预期的状态数<br>• `minimal.statechart.json` 能通过 schema 与静态检查 | `$CARGO test $MF --test statechart_model`<br>`$CARGO test $MF --test input_contract --test cli`（覆盖新类型枚举、guide 与 HELP）<br>clippy | M3：schema、示例、静态检查、穷举与判定齐备后 |
| V-5 | R-2：<br>• F-2：合并、去重、「或」合并；`inside`/`hidden` 计数；隐藏出错步骤时报错；单区域自环<br>• F-3：父状态归组与提升<br>• F-4：条件文字与区域说明<br>• F-5：失真回退的原因与叙述<br>• F-8：混合 group 不上色，以及 may-break<br>• `leaf-is-parent`、`unlabeled-state`、`group-color` 各有一例 | `$CARGO test $MF --test statechart_chart`<br>clippy | M4：投影的全部步骤齐备后 |
| V-6 | R-3：<br>• F-1–F-8 经 `compile` + `render` + `check_html` 全部通过（showcase）<br>• 节点不重叠、框正确嵌套、标记属性与 facts 一致<br>• 篡改一个 `data-statechart-mark` 或删掉一条箭头后，`check_html` 报 `statechart/artifact-mismatch`<br>• 报告卡片的内容正确<br>• 用 `validate statechart` 检查最小示例，回执含 `model`<br>• 五种语言的目录键集一致 | `$CARGO test $MF --test statechart`<br>`$CARGO test $MF --test cli --test check_anchors --test evidence --test render_check`<br>`mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check`<br>`mise exec --no-deps -- node --test tools/phoenix-diagram/tests/viewer/catalogs.test.mjs tools/phoenix-diagram/tests/viewer/explore.test.mjs`<br>clippy | M5：T-1 结论已记录，T-2、T-3 交付齐备后 |
| V-7 | R-4、C-4、C-5：<br>• skill 元数据与各处复述已同步<br>• 人工通读 description 全文与前 100 字<br>• 所改 SKILL.md 的 YAML 只用仓库允许的字段<br>• bundle 必备 schema 列表包含 statechart<br>• 下列 grep 没有过期的复述 | `mise exec --no-deps -- node --test test/phoenix-diagram/skill.test.js test/phoenix-diagram/build.test.js`<br>`git grep -n -e '十类' -e '十种' -e 'legacy-2.17' -- README.md docs/phoenix-diagram.md skills/phoenix-diagram bin tools/phoenix-diagram/src` | M6：§5.8 的文档与示例齐备后 |
| V-8 | 阶段 B 收口（R-1–R-4）：<br>• 全量回归通过<br>• 开发构建完成并同步 dist<br>• statechart 在真实浏览器中交付并实际看图<br>• 十类回归通过<br>• 角色模拟完成 | 1. `npm run test:diagram`<br>2. `mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/statechart-b --sync-dist`<br>3. 用新 engine 对 F-2、F-3、F-4、F-5 与 `minimal.statechart.json` 分别运行 `finalize statechart … --json`；查看四张暗/亮截图，核对标记、框、条件标签与卡片；用 `review-record` 登记<br>4. 对十类最小示例各 finalize 一次，作回归<br>5. 角色模拟：Agent 扮演首次使用者，只读安装后的 skill 目录（`SKILL.md`、`references/statechart.md`、`schemas/statechart.schema.json`、`schemas/common.schema.json`、最小示例），不看引擎源码，也不看本计划。为「订单支付：超时可以重试；规则：一笔订单最多扣一次款」建模，直到 finalize 完成交付，并在答复中按 SKILL 交接 breaks 的规则与路径。记录尝试轮次、遇到的诊断与卡点 | M6 收口（阶段 B） |

§9.2–§9.5 的内容已在表中定义：全量回归见 V-3、V-8 的 `npm run test:diagram`；真实浏览器与看图见 V-3、V-8；静态检查见各局部退出中的 fmt、clippy 与组装器 `--check`。

## 10. 里程碑与验收安排

默认串行实施：M1–M3 共用 `src/input.rs`、`src/schema.rs`、`scripts/test-phoenix-diagram-native.mjs`，M3–M5 共用 `src/statechart/`。可以并行的只有 M5 内部的 T-3（见 §10.1）。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 删除 legacy-2.17 合同 | 无 | **内容**：§5.6 第 1–6、8、9 项<br>**同构**：各文件中既有的 `== PhoenixV1` 分支<br>**注意**：删文件与改引用放在同一批，批内用 `--no-run` 确认能编译 | V-1；回写「实施进度」 |
| M2 | 删除 upstream 与 Archify 技术耦合，改用 Phoenix 标识（阶段 A 收口） | M1 的 V-1 | **内容**：§5.6 第 7 项与 §5.7 全部<br>**顺序**：先按 §5.7 第 4 项完成组装器的等价生成，再删除旧文件<br>**同构**：`catalogsAt` 中现有的校验、`verifyNativeTextSources` 的现有签名<br>V-2 通过后再跑 V-3 | V-2、V-3；回写「实施进度」 |
| M3 | statechart 合同、表达式与穷举 | M2 的 V-2（schema `$id` 基址与 `input.rs` 已稳定） | **接入 `DiagramType::Statechart`**：<br>• `input.rs:9-59`：`ALL` 改为 11 项，补 `as_str`<br>• `schema.rs` 的 PHOENIX 表<br>• `semantics.rs:106,249`：调用 `statechart::check_static`<br>• `layout/mod.rs:37-48`：在 M5 之前返回 `statechart/layout-unavailable` 诊断<br>• `evidence.rs:132-142`：无来源集合<br>• `scene.rs`：增加 `DiagramFacts::Statechart`<br>**新增**：`src/statechart/{mod,expr,model,explore}.rs` ★；schema 与 `examples/minimal.statechart.json` ★<br>**CLI**：`cli.rs` 的 HELP 与 guide<br>**测试枚举**：`tests/support/input_corpus.rs:232,304`、`tests/check_anchors.rs:109`、`tests/evidence.rs:94`、`tests/cli.rs:123,139-155`（guide 的模糊用例由 10 改为 11）<br>**fixture**：F-1、F-6、F-7<br>**注意**：遍历 `ALL` 编译最小示例的测试（`check_anchors.rs:181`、`evidence.rs:194`）要到 M5 才会通过，所以 M3、M4 不跑整 crate 测试 | V-4；回写「实施进度」 |
| M4 | 图投影 | M3 的 V-4 | **新增**：`src/statechart/{chart,regions,report}.rs` ★，实现 §5.4 与判定汇总；卡片文字在 M5 接入目录<br>**fixture**：F-2–F-5、F-8 | V-5；回写「实施进度」 |
| M5 | 嵌套布局、渲染、检查、Reader、报告与回执 | M4 的 V-5 | **任务**：见 §10.1 的 T-1–T-3<br>**同构**：<br>• `layout/architecture.rs:252-271`：Frame + router<br>• `layout/erd.rs:746`：`route_with`<br>• `render/tree.rs`：TypeRenderer 与自写节点<br>• `check/tree.rs`：由 facts 推导后比对<br>• `layout/waterfall.rs:363-372`：由目录生成图例<br>**其余**：`layout/mod.rs` 去掉占位诊断；完成 §5.9 署名 | V-6；回写「实施进度」 |
| M6 | skill、文档、分发与阶段 B 收口 | M5 的 V-6 | **内容**：§5.8 中 M6 的部分，以及 `build-phoenix-diagram.mjs:105-108`<br>V-7 通过后再跑 V-8 | V-7、V-8；回写「实施进度」 |

提交点：M2（阶段 A 收口）、M6（全部完成）。

### 10.1 内部任务

| 任务 ID | 所属里程碑 | 交付物/影响路径 | 前置成果 | 检查 ID |
| --- | --- | --- | --- | --- |
| T-1 | M5 | `layout/statechart/place.rs` 与 `layout/statechart.rs` 的摆放 + 走线（不含标记与卡片）；F-3、F-4 通过 `check_html`；在 M5 记录中写明 A-2 的结论与所用参数（§6.1） | M4 的 V-5 | V-6（F-3、F-4 的 `check_html` 部分） |
| T-2 | M5 | `render/statechart.rs`、`check/statechart.rs`、`scene.rs` 中的 facts 字段；`render.rs`/`check.rs`/`check/anchors.rs`/`brands.rs` 的分派；自环与起始标记。`scene.rs` 由 T-2 单人负责 | T-1 | V-6 |
| T-3 | M5 | 五种语言的目录键、报告卡片、图例条目、`receipts.rs`/`prepare.rs` 中的 `model` 字段、§5.9 署名；只写 `viewer/locales/`、`src/statechart/report.rs`、`src/delivery/`、`NOTICE.md`、`assets/licenses/`。可以与 T-1 并行 | M4 的 V-5 | V-6 |

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1 浏览器 | V-3、V-8 的浏览器门 | Agent 在 M2 收口前跑一次 finalize；缺失时请用户安装浏览器，或设置 `PHOENIX_DIAGRAM_BROWSER` | M2 收口 | 否：只影响收口证据，不影响开发 |
| A-2 嵌套走线 | M5 的路线 | T-1 按 §6.1 的分支处理 | M5 T-1 | 否 |
| A-3 表达式可写性 | statechart 的可用性 | V-8 角色模拟；失败时按 ADR-2 的触发重新评估 | M6 收口 | 否 |
| P3 计划第 8 个里程碑仍处于阻塞（`.xgent-ai/sdlc/status/plan/b41a85fb-9c61-4d64-a2ad-58483ebc7aa9.md`） | P3 待验收证据中依赖 legacy 兼容的部分，在本计划完成后失效 | 用户：续做 P3 时，按 C-9 重新界定其中与 legacy 相关的 AC；本计划不改 P3 | 续做 P3 之前 | 否 |
| 下次发布 | 不能复用旧 engine | 发布时由 CI 重建 engine（见 §4） | 下次发布时 | 否 |
| 已交付的 legacy HTML | 只能阅读，不能再通过 `check` | ADR-4 已接受 | — | 否 |

- 最终状态：**Ready**。
- 定级理由：范围、合同、身份与许可的处理，用户已于 2026-11-04 决定并写入账本。剩余三项假设只影响收口证据或 M5 的局部路线，且各有判据与分支，不改变范围、安全、数据或外部契约。

## 12. 已知坑与历史教训

- **删文件导致编译失败**：`build.rs collect()` 遇到缺失的输入会 panic；`include_str!`/`include_bytes!` 指向被删文件时，整个测试二进制都编译不过。删除与改引用放在同一批，批内先跑 `$CARGO test $MF --no-run`。
- **clippy 以 `-D warnings` 运行**（`scripts/test-phoenix-diagram-native.mjs:30`）：删除 legacy 后未使用的字段、参数与函数必须一并删除，例如 `Router.legacy`、`Context::legacy`、`lifecycle.rs::point`、`default_sides` 的 legacy 参数。不能用 `_` 前缀绕过（`AGENTS.md` @surgical_changes）。
- **`has_legacy_contract` 实际表示「原五类」**：phoenix-v1 代码也在用它（`check.rs:1195`、`semantics.rs:140`），只改名，不删除。
- **readerFit 字符串 `"legacy"` 并非 legacy 合同专用**（`layout/sequence.rs:775-779`）：它的处理逻辑要保留。
- **`Locale::text` 缺 key 直接 panic**（`src/i18n.rs:103-108`）：新键要在同一批里写齐五种 locale。
- **品牌 JSON 的摘要级联四处**：`brandSourceIdentity`、`brand-license-sources.json` 的 catalog 块、dist 中的许可副本、`brand-legal.test.js`。
- **测试文件是显式列出的**（`scripts/test-phoenix-diagram-native.mjs:44-54`）：迁移或删除测试时，同步修改这份列表。
- **zsh 下 glob 参数要加引号**（`~/.claude/CLAUDE.md`）：§9 的命令都写成显式文件列表。
- **`viewer/` 片段在 M1 仍受逐 hunk 审计约束**（`assemble-phoenix-diagram-reader.mjs:47-146`）：Reader 源码的改动只在 M2 简化组装器之后进行。
- **oracle JSON 不再能重新生成**：tests 中现有的 oracle JSON 由 Archify 渲染器生成。删除生成器后，它们是固定的回归基线；需要更新时手工修改，并在记录中说明原因。
