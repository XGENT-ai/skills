# ELINTP-VISUALS-P5 · XGENT-ai Skills · 变更视图与逐词标注（Phoenix Diagram 四类结构图的单输入变更标记 · 新增 annotation 图型 · 静态图与 elintp 选图收口）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-16 · `84f91b5e1e24a9a13b9ac5e370142fbb8e6bc0ac` · dirty（未提交改动是本计划及其评审报告、P4 补缺计划的评审文档与 `.xgent-ai/**` 台账，不在本计划改动面；`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 相对 HEAD 无改动）。
>
> 需求来源：[elintp 模板化图文说明与 Phoenix Diagram 图解补充 PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P5（第 5 期）。本期承接 WORK-F7/AC-1 至 AC-6、WORK-F8/AC-1 至 AC-4、WORK-F5/AC-6、WORK-F9/AC-6、WORK-N3/AC-5；P5 是 WORK-F5、WORK-F9、WORK-N3 的终验期，这三条的终验证据在本期收口时汇齐。
>
> 本期交付：**phoenix-diagram 可在 workflow、lifecycle、tree、erd 的一份输入里把元素标为新增、删除、变更，把关系标为新增、删除；读者看到带徽标与计数的变更视图，并可切到变更前、变更后，导出与打印为当前视图。新增 `annotation` 图型，对一句或几句原文逐词点评。两者都可取静态图；elintp 在材料讲结构改动或含逐词点评时选用它们。**
> **开发范围：`tools/phoenix-diagram`（引擎与阅读器维护源）、`skills/phoenix-diagram`（随工具附带的 skill 及 `dist/phoenix-diagram/bundle` 副本）、`skills/elintp/SKILL.md`（选图规则）、复述图型数量的现行文档；依据见 §0。**
> 本期独特职责：架构比较要两份输入、交付三个并排的阅读器；本期的变更视图只要一份输入、仍是一张图，变更前后由同一张图切换得到。`annotation` 画的是一句话里的词段与对它们的点评，不是实体与关系。
> **顶层排除：architecture 之外的两份输入比较；其他图型的变更标记；变更前、变更后的重新排版；整篇文档的批注；elintp 脚本与 summarize skill 不改；不向 SPMS 提交报告；不发布 npm 包或引擎。**

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
  MUST: ["需求/约束/假设 ID 只在账本登记一次，上游 ID 保留原编号及来源；检查、里程碑、任务 ID 分别在验证表、里程碑表、按需内部任务表定义，其他位置仅引用或展开；单处定义不限制复用，同一检查可被多个里程碑引用、批末只运行一次",
         "修订保留已有 ID，删除/拆分/合并同步引用",
         =>[task_table_present, "按交付物、前置成果与检查 ID 推进，进展及证据写入所属里程碑记录，不另设任务状态表"]]

RULE @milestone_completion:
  MUST: ["本里程碑退出检查全部通过才能记完成；未运行、失败、环境缺失分别记缺口",
         "同批里程碑共用 §9 检查：批末运行一次，通过后一并记完成；批末前已交付的里程碑保持进行中，不单独补验；里程碑和 commit 仍各自独立",
         "最后的收口须通过 §9 真实使用验收；测试全过或代码评审通过不等于能用，汇报进度先说真实场景结果，不以测试通过数衡量",
         "阶段终验由收口里程碑负责，通过前不宣称整期或跨期需求已验收"]

RULE @role_acceptance:
  MUST: ["Agent 按 §9 指定的知识、资料及权限约束模拟上手与业务场景，记录实际结果与卡点",
         "真实使用验收从干净的用户状态出发，只凭公开入口与文档，用真实或有代表性的输入连续完成完整任务，亲自查看产物，不只看退出码、回执或断言",
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
         "优先扩展 §9 的场景 E2E，用真实入口和真实运行栈证明行为；局部测试只写 §9 已列的，或 E2E 暴露的缺陷确需窄测试防复发时补最少用例",
         =>[existing_local_test_fails_after_feature_change, "先判断是否真实回归；行为有意改变或断言绑定内部结构时删除或改到当前行为，在记录写一句"],
         =>[user_visible_problem_found, "修复，或列给用户确认延期并记入当前阻塞；不记为观察后继续完成"]]
  MUST_NOT: ["为假想故障、内部契约或框架已保证的行为加防御、回退或测试",
             "为正文未列为本期的 NFR 做基线、压测、专项验证、调优迭代或引入专门基础设施",
             "用 fallback 或静默恢复掩盖编程错误",
             "为覆盖率、逐个模块、逐个函数或穷举非法输入批量写测试",
             "为本方代码新造 fixture、mock 或内存替身；为保住旧测试保留兼容层或绕开设计",
             "把已知产品缺陷写成测试预期或例外",
             "另建验收框架、认证流水线或证据体系；E2E 用项目已有工具或短脚本照用户操作跑"]

WORKFLOW @implementation_sequence:
  TRIGGER: on_implementation_start_or_resume
  DO: ["按 @resume_protocol 恢复，核对仓库规则、当前任务相关正文与依赖、工作树",
       "按 §10 连续实施批次推进，批次可含多个里程碑，不按文件、函数、一次编辑或一轮 goal 对话切批",
       =>[local_test_admitted, "用当前行为的最小用例先红、再绿、再重构，红绿循环不附带整组回归"],
       "完成本批相关实现，再按 @verification_rhythm 与 @reverification 跑批末检查；场景 E2E 失败就是红灯，定位修复后只复跑失败场景及波及项",
       "批次通过后继续下一可执行批次，里程碑完成按 @progress_writeback 回写，汇报或回答状态询问后继续实施"]

RULE @implementation_stop:
  WHEN: user_explicitly_pauses_cancels_or_switches_task
  MUST: "按 @writeback_timing 保存后停止"

RULE @verification_rhythm:
  MUST: ["批内只为具体失败、技术疑问或必要前置运行能回答它的最小测试/编译检查，无触发则继续实现",
         "批末运行本批场景 E2E、§9 准入的局部测试和改动范围的静态检查；已通过且未受影响的场景不复跑，改动共享入口时加跑主路径",
         "不跑受影响模块或整包的全部已有测试，不因命令叫‘局部/快速’就逐次编辑重跑",
         "全量回归及跨平台矩阵、发布包与干净构建、故障注入、评测、成批真实会话仅在指定阶段收口运行，同环境走查合并",
         "下游前置、数据、安全、不可逆操作、发布及用户分期门及时验证，不等无关终验；范围外依赖未就绪时可在外部边界用 fixture 开发，不能替代真实集成",
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
  MUST: ["按需建 <计划名>.records/M<n>.md，同批共用检查的证据只写批末里程碑记录，其他里程碑链接引用；头部写计划链接、更新时区、状态与验证基线（commit SHA；未提交写 dirty@起始SHA 与关键改动路径）",
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
         "同批里程碑可在各自交付后分别提交并回填各自行，不等批末检查；提交只保存成果，不代表验收通过，批末失败用后续提交修复",
         "仓库或用户对分支、提交信息、签名、是否由 agent 提交另有要求时从其要求",
         "提交前先回写进度；仅要求本次覆盖里程碑或阶段收口的检查，已通过且未失效的证据直接引用，不为提交重跑",
         "只暂存本计划相关改动，不纳入、不回退他人改动，不跳过提交钩子",
         "提交后把短 SHA 回填本次覆盖的各里程碑行‘提交’列，再单独提交只改计划文件的更新，该提交不回填自己的 SHA",
         "代码评审修复写‘修复’加短 SHA，UI 验收修复写‘UI修复’加短 SHA，均追加在原 SHA 后",
         =>[completed_before_commit_point, "对应行写‘待提交’"],
         =>[commit_unavailable, "写明原因并记入当前阻塞，不伪造 SHA"]]

RULE @completion_handoff:
  WHEN: last_milestone_completed
  MUST: ["依次按 @source_cr_writeback 核对 CR 回告、按 @commit_protocol 完成最终提交并回写 SHA，确认每个已完成行都有 SHA；快照‘下一步’只保留仍有的 CR 回写缺口，没有则写‘无’",
         "随后报告完成，提示用户可用 review-code 以本计划路径为参数评审；它从完成记录定位提交，并纳入未提交改动"]
  MUST_NOT: ["把代码评审写成里程碑、退出条件、阻塞项或快照‘下一步’；评审由用户独立发起，不属于本计划",
             "实施中自行启动评审或修复评审意见"]

REVIEW @progress_consistency:
  CHECK: ["每次回写后核对快照、完成记录、记录链接及本计划文件 git diff；非 Git 仓库对照回写前副本",
          "进度 n/N 的 N 等于里程碑总数，n 只计‘已完成’行；最近完成取最后一条已完成记录，不按编号推算",
          "链接指向已落盘文件；已完成行‘提交’为短 SHA 或‘待提交’，全部完成时每行有 SHA 且无‘待提交’",
          "本节外只有有意修订的正文，定义行未被进度文字改写"]
  EXPOSE: on_issue
  # 结构、计数与链接检查不证明验收真实通过。
```

### 恢复快照

- 最近更新：2026-11-17 · M4 已验收完成；M5 未开始
- 当前进度：4/5 个里程碑完成
- 当前状态：M4 完成——描述改为 §6.2 的 199 字文本，五份 references、schema 清单、`docs/phoenix-diagram.md`、README、`bin/xgent-skills.js` 与 elintp「出图」同步到十三类与变更视图/逐词标注两项能力；V-4 三份材料 17 项页面检查（页内三视图、断网、360 px、打印、禁用脚本、grep）全过，V-5 的 `skill.test.js` 6 项与 elintp 21 项通过（`31bb3ff4`），见 [M4 记录](ELINTP-VISUALS-P5.records/M4.md)
- 最近完成：M4 · 接口文字同步与 elintp 选图（记录 [M4](ELINTP-VISUALS-P5.records/M4.md)）
- 下一步：M5 · 阶段收口与跨期终验（V-6 聚合回归与十二类字节比对、§9.2 终验证据表、V-7 从干净状态的真实使用、最终提交与 SHA 回填）；本地辅助物在 `local/phoenix-diagram/`（均不入库）
- 当前阻塞：无
- 代码基线：`84f91b5e`（批内实现提交：`b7fab31a` M1、`55135ea3` M2、`9217a694` M3；批末修复 `6330c17b`、`679e2d08`；M4 `31bb3ff4`）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-17 | 十二类基线与指纹已记录；变更标记在 workflow 全链路打通，阅读器三视图切换/导出/打印/探索回退与错误输入诊断均通过 | [M1 记录](ELINTP-VISUALS-P5.records/M1.md) | `b7fab31a` |
| M2 | 已完成 | 2026-11-17 | lifecycle、tree、erd 的变更视图、静态图、三视图与错误输入均通过 V-2；批末修复同级换父连线绕行与静态图样式残留 | [M2–M3 批末记录](ELINTP-VISUALS-P5.records/M2M3.md) | `55135ea3`；修复 `6330c17b` |
| M3 | 已完成 | 2026-11-17 | annotation 的看图、搜索与他人详情、主题/预设、导出与错误输入均通过 V-3；批末修复节点身份缺失 | [M3 记录](ELINTP-VISUALS-P5.records/M3.md) | `9217a694`；修复 `679e2d08` |
| M4 | 已完成 | 2026-11-17 | 十三类与两项能力的接口文字同步；V-4 三份材料 17 项页面检查与 V-5 接口测试通过 | [M4 记录](ELINTP-VISUALS-P5.records/M4.md) | `31bb3ff4` |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID，验收原文见 PRD §3，不在此重抄；「内容」列只写本期承接的部分。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F7/AC-1 | 功能 | PRD §3 WORK-F7 · P5 | workflow、lifecycle、tree、erd 中被标记的元素与关系在变更视图带对应标记，标记有颜色之外的符号；图上给出三类变更的数量 | §5.1–§5.4；V-1、V-2、V-5 | 已确认 |
| WORK-F7/AC-2 | 功能 | 同上 | 变更前不含新增、保留删除，变更后相反；两者不带标记，未标记内容三处一致；标为变更的项三处显示同一描述，前后视图在图上注明它不代表变更前的取值 | §5.3（说明带）、§5.4（视图规则）、§6.1；V-1、V-2 | 已确认 |
| WORK-F7/AC-3 | 功能 | 同上 | erd 被标记实体的全部字段带同一标记；只标字段时实体不带标记 | §5.2（有效标记）、§5.4（erd）；V-2、V-5 | 已确认 |
| WORK-F7/AC-4 | 功能 | 同上 | 矛盾标记、关系标为变更、删除元素却保留指向它的未删除关系时，失败信息指向具体对象。标记写在元素自身，不存在「指向不存在元素的标记」；关系端点或 tree 的 `parent_before` 不存在沿用既有 `input/unknown-reference` | §5.1、§5.2；V-1、V-2、V-5 | 已确认 |
| WORK-F7/AC-5 | 功能 | 同上 | 没有任何标记的输入与现有行为一致，不显示视图切换 | §5.3（无标记时场景不变）、§6.1；V-1、V-6 | 已确认 |
| WORK-F7/AC-6 | 功能 | 同上 | 导出或打印得到当前所选视图，图上标明是变更视图、变更前还是变更后 | §5.3（说明带）、§6.1；V-1 | 已确认 |
| WORK-F8/AC-1 | 功能 | PRD §3 WORK-F8 · P5 | 原文逐字出现且顺序不变，每条标注对应确切词段，读者能分辨对应关系 | §5.6–§5.9；V-3、V-5 | 已确认 |
| WORK-F8/AC-2 | 功能 | 同上 | 问题词段与普通标注有颜色之外的区分；改法与该词段相邻显示 | §5.8（卡片）、§5.9；V-3 | 已确认 |
| WORK-F8/AC-3 | 功能 | 同上 | 词段在原文中不存在或两条标注的词段重叠时，失败信息指向具体标注 | §5.7；V-3、V-5 | 已确认 |
| WORK-F8/AC-4 | 功能 | 同上 | 原文换行后标注仍与词段对应，没有文字被裁剪或互相遮挡 | §5.8（换行）；V-3、V-5 | 已确认 |
| WORK-F9/AC-6 | 功能 | PRD §3 WORK-F9 · 跨期，终验 P5 | 变更视图与逐词标注可取得静态图，完整性、亮暗与体积要求同既有图型；带标记输入的静态图呈现变更视图 | §5.5、§5.9；V-2、V-3、V-6 | 已确认 |
| WORK-F5/AC-6 | 功能 | PRD §3 WORK-F5 · 跨期，终验 P5 | 材料描述结构改动或含需要逐词点评的句子且 phoenix-diagram 可用时，elintp 分别选用变更视图与逐词标注 | §6.3；V-4 | 已确认 |
| WORK-N3/AC-5 | 兼容 | PRD §3 WORK-N3 · 跨期，终验 P5 | 既有图型未带标记的有效输入结果不变；skill 描述、README、文档与 elintp 出图对照同步说明两项能力及适用图型 | §5.10（判定口径）、§6.2、§6.3；V-6 | 已确认 |
| C-1 | 约束 | PRD §7.3、§7.4 | P5 是 WORK-F5、WORK-F9、WORK-N3 的终验期：收口逐条给出这三条全部 AC 的证据（本期新证据，或前期记录加未失效依据） | §9.2；V-6、V-7 | 已确认 |
| C-2 | 约束 | PRD §7「已交付的部分持续遵守对应的质量门槛」 | elintp 页面里新图的嵌入仍满足 WORK-N1 与 WORK-N2/AC-1、AC-2：窄屏不出页面级横向滚动、键盘可达、打印与禁用脚本可读、单文件零网络 | §6.3；V-4 | 已确认 |
| C-3 | 约束 | [AGENTS.md](../../AGENTS.md) `@skill_tool_independence` | elintp 不以 phoenix-diagram 的安装或引擎版本为执行前提；引擎不支持变更标记或 `annotation` 时走降级 | §6.3 | 已确认 |
| C-4 | 约束 | AGENTS.md §7；PRD §5.3 的 C6、C10、C12 | 修改两个 skill 前后按整合清单核对：elintp 触发与授权不变；phoenix-diagram 只新增两项能力并改写描述边界，不引入固定数量上限 | §6.4；V-6 第 4 步 | 已确认 |
| C-5 | 约束 | AGENTS.md「Working in This Repository」接口复述条款 | 「十二类」的全部现行复述在同一改动内更新；历史计划、记录与 CHANGELOG 既有条目不改 | §1.2、§6.2；V-6 第 4 步 | 已确认 |
| C-6 | 约束 | AGENTS.md §6；`test/phoenix-diagram/skill.test.js` L15–L27 | phoenix-diagram 描述不超过 200 字，前 100 字含既有词与「逐词标注」，`version: "0.2.0"` 不变；elintp 描述不改 | §6.2 | 已确认 |
| C-7 | 约束 | AGENTS.md「Working in This Repository」；`scripts/test-phoenix-diagram-native.mjs` | 引擎检查用 mise 固定的 Rust 与 `mbx`，离线、锁定依赖；工具缺失即失败，不跳过 | §9 环境 | 已确认 |
| C-8 | 约束 | `tools/phoenix-diagram/viewer/README.md`；[docs/phoenix-ui.md](../phoenix-ui.md) | 改过阅读器维护源后运行装配脚本，`tools/phoenix-diagram/assets/template.html`、`tools/phoenix-diagram/assets/catalogs.json` 是生成物不手改；改过 `skills/phoenix-diagram` 后运行 `--sync-bundle` 并提交 bundle | M1–M4；V-5、V-6 | 已确认 |
| C-9 | 约束 | AGENTS.md `@simplicity`、`@failure_handling` | 工具保持直接实现：只在输入边界校验，不加事务、备份、兜底或兼容层 | §5、§8 | 已确认 |
| C-10 | 约束 | `skills/phoenix-diagram/references/extending-types.md` | 新图型同时交付输入与语义、发现与创作、布局渲染与检查、Reader 与导出、交付与分发、回归六条路径；不加内容配额，不靠裁切、缩字、隐藏过检 | §5.6–§5.10、§9 | 已确认 |
| C-11 | 约束 | PRD §1.3 NG9、§5.1 D8、D12 | 单输入变更标记只给 workflow、lifecycle、tree、erd；两份输入的比较仍只有 architecture；不推断变更的原因、风险或影响；「变更」不携带旧值 | §5.1、§0.5 | 已确认 |
| C-12 | 约束 | PRD §4「参考基线」 | 复用参考工具代码须保留 MIT 许可与来源；本期只借鉴行为、不复制代码，不新增许可条目 | D11 | 已确认 |
| C-13 | 约束 | `skills/elintp/scripts/draft.mjs` L8–L13（静态 SVG 白名单） | 静态图只用 elintp 已接受的元素、属性与 CSS 属性，本期不改 elintp 脚本：不用 `text-decoration`、`xml:space`、`foreignObject` | §5.4、§5.9、§12 | 已确认 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 变更标记写在哪 | 元素与关系自身的可选字段 `change` | 四份 schema 各加字段；没有 `change` 的输入完全走原路径；其他图型的 schema 不认识该字段，按 `input/schema` 拒绝 | WORK-F7 行为；C-11；关系 `id` 是可选的，按引用标记不可靠（`skills/phoenix-diagram/schemas/workflow.schema.json`） |
| D2 | 三种视图怎么产生 | 一次布局（全部元素），SVG 根属性 `data-change-view` 切换，CSS 隐藏当前视图不存在的内容 | 未标记内容在三种视图位置完全相同；变更前、变更后不重新排版，被隐藏元素的位置留空 | ADR-1 |
| D3 | 标记的图面形式 | 节点右上角内侧的圆徽标，内画 `+`、`−`、`~` 线条符号；关系起点旁同样的小徽标；erd 字段行左侧留白内画符号；另加描边色与线型 | 符号用路径画，不是文字，不受字体与字号检查影响；颜色取既有主题变量 | WORK-F7/AC-1；同构 `render.rs` `delta_node_marker` L639 |
| D4 | 计数、视图名与「变更」说明放哪 | 画布底部新增一条说明带，三种视图各一组文字，高度按三者最高的一组预留 | 视图名与说明随 SVG 进入导出、打印与静态图；切换视图时画布尺寸不变 | WORK-F7/AC-1、AC-2、AC-6 |
| D5 | 计数口径 | 每类变更的数量 = 图上带该类徽标的对象数：节点、作者写出的关系、erd 字段（含从实体继承的字段）、tree 换父的连线（D19）；随子节点新增或删除的 tree 父子连线不单独计数 | 读者数图上的徽标能对上数字；内容检查按同一口径核对 | WORK-F7/AC-1、AC-3 |
| D6 | 关系与端点的一致性 | 关系的某端被删除时关系必须标为删除；某端是新增时关系必须标为新增。tree 的父子连线按 D19 取得有效标记后适用同一规则：父节点被删除时，子节点须标为删除或用 `parent_before` 换父；父节点是新增时同理 | 前一半是 AC-4 原文；后一半由 AC-2 推出：变更前不能画一条连到不存在节点的线。tree 不做隐式继承，逐个节点显式标记 | WORK-F7/AC-2、AC-4 |
| D7 | 不可标记的对象 | workflow 的 lane、phase、group，lifecycle 的泳道，erd 的域带不接受 `change` | 只含被删除节点的分组在变更后是空框；需要时作者拆成两张图 | PRD WORK-F7 行为只列元素与关系 |
| D8 | 阅读器探索功能与视图 | 搜索、大纲、Route、Reach 按全部元素工作；选中当前视图不存在的节点，或在变更前、变更后启动 Route / Reach 时，回到变更视图并提示 | 不改四个探索脚本的遍历逻辑；变更前、变更后是只读的普通图 | 同构 `tools/phoenix-diagram/viewer/tree-branches.js` 的 `reveal`；`tools/phoenix-diagram/viewer/focus.js` L1401 |
| D9 | 看图审核 | 仍是 1440×900 亮、暗两张，画面是默认的变更视图；变更前、变更后不另出截图 | 与 WORK-F10 的单图口径一致；前后视图由内容检查与场景走查覆盖 | PRD WORK-F10/AC-1 |
| D10 | 逐词标注的图型身份 | 新图型 `annotation`，图型由十二类变为十三类 | 命令、回执、静态图、预览按 `<type>` 通用分派，不新增命令或参数 | WORK-F8；`tools/phoenix-diagram/src/input.rs` L11–L40 |
| D11 | 参考工具 | 只借鉴变更标记与逐词标注的语义；不复制其代码与样式 | NOTICE、许可清单与构建脚本的许可条目不变 | C-12 |
| D12 | 词段怎么定位 | 标注写词段原文 `span`，同一词段出现多次时用 `occurrence`（从 1 起）指明第几处 | 作者不数字符偏移；找不到或越界时报错 | WORK-F8/AC-3 |
| D13 | 词段重叠 | 任意重叠都拒绝：部分重叠、包含与完全相同 | 比 AC-3 的「部分重叠」多拒绝包含与相同两种情形；嵌套词段的徽标顺序无法无歧义地表达 | WORK-F8/AC-3；ADR-2 |
| D14 | 逐词标注的版式 | 句子行内给词段加粗并在词段后放编号；标注逐条列在句子下方的卡片里，卡片带同一编号并引述词段 | 对应关系靠编号与引述，不靠位置对齐；长原文、长点评都不会互相遮挡 | ADR-2 |
| D15 | 什么是节点 | 一条标注一个节点，节点框是标注卡片 | 通用检查、探针、搜索、详情按既有规则工作；句子是容器，不是节点 | 通用检查要求每节点恰一个节点框（`tools/phoenix-diagram/src/check.rs` L538–L566） |
| D16 | 工具版本号 | 保持 0.2.0 | 新增图型时未改版本的既有做法；npm 版本在发布时定 | `tools/phoenix-diagram/Cargo.toml`；`skill.test.js` L25 |
| D17 | 「未带标记的输入结果不变」的判定 | 十二类最小示例的阅读器主图 SVG、内嵌元数据与两种字体策略的静态 SVG 字节不变；阅读器 HTML 整文件因模板新增视图控件、脚本片段与目录文案而变化 | 与 P4 新增图型时的口径相同 | WORK-N3/AC-5；[P4 计划](ELINTP-VISUALS-P4.md) D14 |
| D18 | elintp 何时用两项能力 | 材料描述结构的改动时用对应图型的变更标记画一张图，不画前后两张；材料含需要逐词点评的句子时用 `annotation` | 只改选图规则与对照表，不改触发、授权、默认嵌图方式与脚本 | WORK-F5/AC-6 |
| D19 | tree 怎样表达保留节点的关系替换 | 节点可选字段 `parent_before`：该节点在变更前的父节点。`parent` 仍是布局用的父节点（变更后的父节点；删除节点是它变更前的父节点）。写了 `parent_before` 的节点产生两条连线：`parent_before → 节点` 为删除，`parent → 节点` 为新增；节点自身不带标记或标为变更，三种视图都在 | 节点 C 从 A 移到 B 写成 `{"id":"C","parent":"B","parent_before":"A"}`，不复制节点身份。布局仍按 `parent` 一次完成，旧父连线是追加的一条关系（§5.3）。树根在变更前后不变：根不能写 `parent_before`，在旧根之上新增根或删除根只留其子树都被拒绝 | WORK-F7 行为「改变一条关系表达为删除旧关系并新增新关系」；`parent` 是单个 id（`skills/phoenix-diagram/schemas/tree.schema.json` L119–L120）；树须单根（`input/semantics.rs` L691） |

### 0.3 ADR-lite

#### ADR-1：一次布局，三种视图由根属性切换

- 状态：Accepted
- 背景与驱动：WORK-F7/AC-2 要求未标记内容在三种视图中一致；AC-5 要求无标记输入不变；AC-6 要求导出、打印得到当前视图；WORK-F9 要求静态图呈现变更视图。阅读器假定页面只有一个主图 SVG（`check.rs` L375–L393 的 `artifact/single-svg`）。
- 备选：
  - A. 对全部元素布局一次；元素带 `data-change`，SVG 根带 `data-change-view`，随 SVG 输出的样式按视图隐藏元素与标记。
  - B. 从一份输入派生变更前、变更后两份输入，三次布局，三个 SVG。前后视图排版紧凑，但未标记内容的位置在三处不同；阅读器、检查、探针、导出、静态图都要改成多图。
  - C. 复用架构比较的三个 iframe 外壳（`tools/phoenix-diagram/src/compare/render.rs`）。交付物变成另一种合同（比较 summary、6 张截图），与「单图交付」冲突。
- 决策：A。树的折叠已按同一做法实现：布局一次，按属性隐藏，可见节点不移动（`tools/phoenix-diagram/viewer/tree-branches.js`、`tools/phoenix-diagram/src/render/tree.rs` L61–L63）。
- 正面后果：只有一个 SVG、一份元数据、一次检查；导出与打印天然是当前视图；静态图把根属性固定为变更视图即可。
- 负面/中性后果：变更前、变更后在被隐藏元素处留空，不是重新排版的图；关系线的路线按全部元素算出，隐藏一条线后其余线不回到最短路线。tree 的换父节点只排在变更后的父节点下，变更前视图里它的旧父连线绕外侧通道到达（D19、§5.3）。
- 重新评估触发：出现「变更前、变更后要各自紧凑排版」或「变更前、变更后要各自通过看图审核」的需求。

#### ADR-2：逐词标注用「行内编号 + 下方卡片」，不用按位置对齐的批注线

- 状态：Accepted
- 背景与驱动：WORK-F8/AC-1、AC-4 要求标注与词段对应，换行后不裁剪、不遮挡。引擎没有字形测量，文字宽度按「字符单位 × 字号 × 0.6」估计（`tools/phoenix-diagram/src/geometry.rs` `text_units` L12）；CJK 字符的实际宽度小于估计。elintp 的静态图白名单没有 `text-decoration`（C-13）。
- 备选：
  - A. 句子是一行行 `<text>`，词段是加粗的 `<tspan>`，词段后跟一个行内编号；标注在句子下方逐条成卡片，带同一编号并引述词段。位置由浏览器排字决定，引擎只需要估计在哪里换行。
  - B. 参考工具的做法：词段下画括线，批注按词段的横坐标悬挂在句子下方，重叠时换行排。需要每个词段的精确横坐标；估计有偏差时括线与词段错位，长批注要做碰撞处理。
  - C. 每段文字用 `textLength` 强制等于估计宽度后再按 B 画。CJK 文字会被拉开间距。
- 决策：A。问题词段的区分：行内编号后加 `!`，卡片带「问题」字样与虚线边框；改法写在卡片里引述词段的下一行。
- 正面后果：对应关系不依赖坐标估计；任意长度的原文与点评都能排下；不用新的 SVG 属性。
- 负面/中性后果：批注不紧贴词段，读者靠编号对照；行内编号是画在原文之间的附加字符（内容检查核对原文时排除它们）。
- 重新评估触发：引擎有了字形测量，或用户要求批注线式的版面。

### 0.4 职责与事实所有权

|  | phoenix-diagram 引擎与 skill | elintp | 宿主 Agent（范围外） |
| --- | --- | --- | --- |
| 拥有 | `change` 字段的契约、校验、有效标记、三种视图、说明带、导出与静态图；`annotation` 的输入契约、校验、版式；交付与审核 | 何时用变更视图与逐词标注；把图放进页面；最终页自检 | 从授权材料判断哪些内容是新增、删除、变更；点评内容 |
| 不拥有 | 改动是否属实；改动的原因、风险与影响；哪里写得不好 | **图的校验、图面与看图审核**；不改写标记与点评 | 引擎的校验与审核结论 |

一句话：标得对不对、画得对不对归 phoenix-diagram，改了什么、哪里该点评归作者与宿主 Agent，页面里放得对不对归 elintp；输入不合约时各自报格式错误，不猜。

### 0.5 明确不在本期

- **architecture 之外的两份输入比较；workflow、lifecycle、tree、erd 之外的单输入变更标记**——PRD NG9；出现具体需求时另立需求。
- **lane、phase、group、域带的变更标记；「变更」项的旧值；变更原因、风险与影响的推断**——D7、C-11。
- **tree 根节点在变更前后的更换**——D19；需要时画成两张树。
- **变更前、变更后的重新排版与各自的看图审核截图**——ADR-1、D9。
- **变更前、变更后视图里的探索功能**——D8；这两个视图里树的折叠计数、大纲与搜索列表仍按全部元素计。
- **嵌套或重叠的词段；整篇文档的批注或审校；按位置对齐的批注线**——D13、ADR-2、PRD WORK-F8 边界。
- **`annotation` 的仓库来源、品牌标识、架构比较**——与 `chart`、`statechart` 相同。
- **elintp 脚本、模板与描述；summarize skill**——不改（WORK-N3/AC-1、AC-2）。
- **渲染性能基线与压测**——后续可选；本期新增的是元素数量的线性计算。
- **npm 发布、引擎四平台构建、CHANGELOG 条目、向 SPMS 提交报告**——发布流程，见 §8。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **已核实·足够**：前置期已完成。[P3 计划](ELINTP-VISUALS-P3.md)与[补缺计划](ELINTP-VISUALS-P3-gaps.md)交付了 `static` 命令与 elintp `static:` 来源；[P4 计划](ELINTP-VISUALS-P4.md)恢复快照 5/5、[P4 补缺计划](ELINTP-VISUALS-P4-gaps.md) 2/2，`chart` 图型在当前 HEAD 可用。
- **已核实·缺口**：四份 schema 的节点与关系对象都是 `additionalProperties: false`，没有 `change` 字段（`skills/phoenix-diagram/schemas/{workflow,lifecycle,tree,erd}.schema.json`）→ §5.1 → 漏做则 WORK-F7 全部不成立。
- **已核实·缺口**：引擎只有十二类，`DiagramType::ALL` 长度 12（`tools/phoenix-diagram/src/input.rs` L11–L40）→ §5.6–§5.10 新增 `annotation` → 漏做则 WORK-F8 不成立。
- **已核实·足够**：`layout::compile` 在各图型布局之后有统一的后处理位置（`clear_sigil_labels`、`attach_brands`，`tools/phoenix-diagram/src/layout/mod.rs` L50–L51）；`CompiledDiagram` 可序列化，新增可选字段在缺省时不输出即可保持旧输入的 `--layout-json` 与元数据不变（`tools/phoenix-diagram/src/scene.rs` L246–L263）。
- **已核实·足够**：节点与关系的原始 JSON 原样保留在 `authored.facts`，并进入内嵌元数据的 `graph.nodes`、`graph.relations`（`tools/phoenix-diagram/src/layout/workflow.rs` L59、`tools/phoenix-diagram/src/layout/erd.rs` L1122、L1341；`tools/phoenix-diagram/src/render.rs` L1464–L1465）；tree 的连线由 `parent` 派生，`source_path` 为 `/nodes/<i>/parent`（`tools/phoenix-diagram/src/layout/tree.rs` L863–L880）。
- **已核实·缺口**：tree 的关系模型只有单个 `parent`，且三处假定每个节点恰有一条入线——布局按 `parent` 建单根层级（`layout/tree.rs` `structure` L222–L254），渲染从关系表反查父节点（`tools/phoenix-diagram/src/render/tree.rs` `Hierarchy::of` L95–L100），内容检查要求元数据关系与画出的连线都等于「每个 `parent` 一条」（`tools/phoenix-diagram/src/check/tree.rs` L45–L94）→ D19、§5.3、§5.4 → 漏做则保留节点的换父无法表达。
- **已核实·足够**：workflow 与 lifecycle 走通用渲染（`render.rs` `write_nodes` L1104、`write_relation_paths` L999、`write_relation_labels` L1253）；tree 与 erd 走各自的 `TypeRenderer`（`tools/phoenix-diagram/src/render/tree.rs` `write_node` L196、`body` L291；`tools/phoenix-diagram/src/render/erd.rs` `body` L292，字段行 `g[data-er-row]` L395–L449）。带交叉衬底的关系外面包一层 `<g data-graph-role="automatic-crossover">`（`render.rs` L1052–L1054）。
- **已核实·足够**：最接近的同构实现有两处。架构比较的增量视图在节点右上角画圆徽标、在关系起点旁画符号、按状态改描边（`render.rs` `delta_node_marker` L639、`write_delta_markers` L649、`DELTA_STYLE` L825）；树的折叠用属性加样式隐藏子树，导出时清掉折叠状态（`tools/phoenix-diagram/viewer/tree-branches.js`；`tools/phoenix-diagram/viewer/export-cleanup.js` L150–L158）。
- **已核实·足够**：导出克隆整个主图 SVG，保留根属性与 SVG 内的 `<style>`，只清除运行期状态属性（`tools/phoenix-diagram/viewer/export.js` `serializeSvg` L236–L270；`export-cleanup.js` 的清除清单不含自定义的 `data-change-*`）；打印不改 DOM。带根属性的视图状态会随导出与打印保留。
- **已核实·足够**：静态转换对 SVG 内 `<style>` 的规则重新解析：选择器必须引用 SVG 根，声明须在支持清单内，`display`、`opacity`、`stroke-dasharray`、`font-weight` 在清单内，`white-space`、`text-decoration` 不在（`tools/phoenix-diagram/src/render/static_style.rs` `scope_selector` L242、`supported_declaration` L629–L680）；运行期选择器按子串清单丢弃（`interactive_selector` L327–L383），带特定属性的元素整体丢弃（`drop_element` L1555）。
- **已核实·足够**：`viewer.css` 中既不引用 SVG 根、也不以 `.c-`、`.t-`、`.a-`、`.m-` 开头的规则不进入静态图（`static_style.rs` L255–L265），因此在 `viewer.css` 加 HTML 控件样式不改变任何图型的静态 SVG。
- **已核实·足够**：通用产物检查要求节点数、关系数与元数据相等（`check.rs` L955–L964），节点 `<g>` 内可以有额外图元；节点内文字的投影字号低于 6 px 时失败，但只看带 `font-size` 属性的 `<text>`（L613–L630）。浏览器探针要求默认状态下每个节点、关系与带 `data-source-path` 的文字都可见，全部可见文字落在 SVG 内（`tools/phoenix-diagram/src/browser/probe.js` L113–L126、L180–L186、L223–L237）；探针固定核对 7 个控件（`tools/phoenix-diagram/src/browser/observation.rs` L176–L177）。
- **已核实·足够**：阅读器由 14 个脚本片段按固定顺序装配（`scripts/assemble-phoenix-diagram-reader.mjs` L11–L26），片段在模板里用 `/* PHOENIX:NAME */` 标出（`tools/phoenix-diagram/viewer/template.source.html` L949、L951）；`focus.js` 在聚焦节点时调用 `PhoenixDiagram.treeBranches.reveal(id)`（`tools/phoenix-diagram/viewer/focus.js` L1401–L1402）；Route 与 Reach 生效时在主图根上写 `data-route-active`、`data-reach-active`（`tools/phoenix-diagram/viewer/route-probe.js` L750、`focus.js` L196）。
- **已核实·足够**：lifecycle 的合流点是场景里的一组坐标，没有归属关系（`scene.rs` `LifecyclePresentation.junctions` L141；`render.rs` L913–L917）；erd 的共用干线是 `erd-trunk` 容器，成员关系写在 `geometry_provenance` 的 `/relationships/<i>`（`layout/erd.rs` L1529–L1542）。
- **已核实·足够**：新增图型的登记点可由 `chart` 的新增提交 `1d9e3f57` 逐项对照（§5.10）；`static`、`finalize`、`validate`、`deliver`、`preview` 按 `<type>` 解析 `DiagramType`，没有图型白名单。
- **已核实·足够**：换行与断词已有实现：`layout/tree.rs` 的 `tokens`（L54，西文按词连同其后空白、CJK 逐字）与 `wrap_text`（L80）。两者目前是文件内私有函数。
- **已核实·足够**：elintp 的静态图校验接受 `data-*` 属性、`tspan`、`dy`、`font-weight`、`font-size`，CSS 属性含 `display`、`opacity`、`white-space`，不含 `text-decoration`；选择器须锚定静态根，可带 `data-*` 属性条件（`skills/elintp/scripts/draft.mjs` L7–L13、L141–L232、L393–L394）。
- **已核实·足够**：elintp「出图」对照表与降级规则在 `skills/elintp/SKILL.md` L181–L205；phoenix-diagram 描述长 183 字（`skills/phoenix-diagram/SKILL.md` L3）。
- **已核实·足够**：`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 在调查基线相对 HEAD 无改动。

### 1.2 拓扑与文件清单

**引擎（`tools/phoenix-diagram`）**

- `tools/phoenix-diagram/src/changes.rs` ★：标记解析、校验、有效标记、计数与说明带（§5.2、§5.3）；`tools/phoenix-diagram/src/lib.rs` 登记模块。
- `tools/phoenix-diagram/src/render/changes.rs` ★、`tools/phoenix-diagram/src/check/change.rs` ★：标记的图面与内容检查（§5.4）。
- `tools/phoenix-diagram/src/layout/annotation.rs` ★、`tools/phoenix-diagram/src/render/annotation.rs` ★、`tools/phoenix-diagram/src/check/annotation.rs` ★：新图型（§5.6–§5.9）。
- `tools/phoenix-diagram/src/` 下的 `input.rs`、`schema.rs`、`input/semantics.rs`、`scene.rs`、`layout/mod.rs`、`layout/tree.rs`（两个函数改为 `pub(crate)`；换父连线与堆叠排除，§5.3）、`render.rs`、`render/tree.rs`、`render/erd.rs`、`render/static_style.rs`、`check.rs`、`check/tree.rs`（换父连线计入预期关系，§5.4）、`evidence.rs`、`cli.rs`。
- `tools/phoenix-diagram/viewer/change-view.js` ★、`tools/phoenix-diagram/viewer/template.source.html`、`tools/phoenix-diagram/viewer/focus.js`（一处调用）、`tools/phoenix-diagram/viewer/viewer.css`、`tools/phoenix-diagram/viewer/locales/{en,zh-CN,zh-TW,es,ko}.json`、`tools/phoenix-diagram/viewer/README.md`；`scripts/assemble-phoenix-diagram-reader.mjs` 的片段清单；`tools/phoenix-diagram/assets/template.html`、`tools/phoenix-diagram/assets/catalogs.json` 由装配脚本重新生成。
- `tools/phoenix-diagram/tests/change.rs` ★、`tools/phoenix-diagram/tests/annotation.rs` ★；`tools/phoenix-diagram/tests/cli.rs`、`tools/phoenix-diagram/tests/check_anchors.rs`、`tools/phoenix-diagram/tests/support/input_corpus.rs` 的图型清单；`test/phoenix-diagram/reader-assets.test.js` 中随片段数变化的断言。
- 不改：workflow、lifecycle、erd 的布局算法，tree 布局对没有 `parent_before` 的输入的结果，其余八类的布局、渲染、检查；`tools/phoenix-diagram/src/` 下的 `browser/`、`delivery/`、`compare/`、`brands/`；`viewer/` 下除上述以外的脚本；`tools/phoenix-diagram/build.rs`、`tools/phoenix-diagram/Cargo.toml`、`tools/phoenix-diagram/NOTICE.md` 与许可清单。

**phoenix-diagram skill 与复述**

- `skills/phoenix-diagram/schemas/annotation.schema.json` ★、`skills/phoenix-diagram/examples/minimal.annotation.json` ★、`skills/phoenix-diagram/references/annotation.md` ★、`skills/phoenix-diagram/references/change-view.md` ★。
- `skills/phoenix-diagram/schemas/common.schema.json`（两个定义）、`skills/phoenix-diagram/schemas/{workflow,lifecycle,tree,erd}.schema.json`、`skills/phoenix-diagram/schemas/README.md`。
- `skills/phoenix-diagram/SKILL.md`、`skills/phoenix-diagram/references/{extending-types,preview,viewer-runtime,static-figure,delivery-contract}.md`。
- `scripts/build-phoenix-diagram.mjs` L106 的 schema 清单；`test/phoenix-diagram/skill.test.js`、`test/phoenix-diagram/build.test.js` L126。
- `dist/phoenix-diagram/bundle/**`：由 `--sync-bundle` 重新生成。
- `docs/phoenix-diagram.md` L3、L115，`README.md` L216，`bin/xgent-skills.js` L301。
- 不改：`docs/PRD-PHOENIX-DIAGRAM.md`、`docs/plan/**` 既有计划与记录、`.xgent-ai/**` 既有记录、`CHANGELOG.md`。

**elintp**

- `skills/elintp/SKILL.md`「出图」一节。
- 不改：`skills/elintp/scripts/**`、`skills/elintp/assets/**`、`skills/elintp/references/draft-format.md`、elintp 的描述与触发、`skills/summarize/SKILL.md`。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `changes::resolve` | `input::semantics::validate`、`changes::attach` | 输入已过 schema 的文档与图型；没有任何 `change` 与 `parent_before` 时返回 `None`；否则返回有效标记或诊断 | 无 | 四类图型的端点规则、tree 换父与两个视图的树约束、erd 字段继承、计数口径（§5.2） | 进程内 | `validate` 回执；V-5 |
| `changes::attach` | `layout::compile` | 输入已布局的场景；`resolve` 为 `None` 时场景不变；否则写入 `scene.changes` 并把画布向下加高一条说明带 | 无 | 场景关系与输入下标的对应、派生图元的隐藏、说明带文字与换行（§5.3） | 进程内 | `validate --layout-json`、写出的 HTML；V-1、V-2、V-5 |
| `render::changes` | `render.rs` 通用渲染、`render/tree.rs`、`render/erd.rs` | 只读 `scene.changes`；`None` 时不输出任何字节 | 无 | 属性、徽标、样式、说明带（§5.4） | 进程内 | 写出的 SVG |
| `check::change::content` | `check::check_html` | 元数据有 `changes` 时核对每个元素的标记、计数与说明带；SVG 带 `data-change-ui` 当且仅当元数据有 `changes` | 无 | — | 进程内 | 回执的 `semantic_content` |
| 阅读器 `PhoenixDiagram.changeView` | 页面控件、`focus.js` | `active`、`view()`、`set(view)`、`reveal(id)`；主图没有 `data-change-ui` 时 `active` 为假、控件保持隐藏 | 无 | 选中被隐藏节点与启动 Route / Reach 时回到变更视图（§6.1） | 进程内（浏览器） | V-1 |
| `layout::annotation::compile` | `layout::compile` | 每条标注一个节点；`facts.sentences` 含原文、每行的片段与标注的字符区间；同输入同输出 | 无 | 词段定位、换行、编号、卡片排布（§5.8） | 进程内 | `validate --layout-json`、写出的 HTML；V-3、V-5 |
| `check::annotation::content` | `check::check_html` | 从产物重新拼出每句原文并与元数据逐字比较；核对词段、编号与卡片 | 无 | — | 进程内 | 回执的 `semantic_content` |

不新增接缝或适配器。`changes` 是一个模块而不是分散在四个布局里：四类图型用同一套规则，校验与场景标注读同一份结果（删除测试：去掉它，端点规则与计数口径要在语义校验和四个渲染路径里各写一遍）。`render::changes` 有三个调用方（通用渲染、tree、erd），提供属性与图元两种小函数，不做成渲染钩子。`annotation` 只有一个调用方，在各自文件内实现。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| elintp `static:` → 变更视图与 `annotation` 的静态 SVG | 已核实足够 | 静态校验接受 `data-*` 属性、`tspan`、`dy`、`font-weight`、`display`、`opacity`、`white-space`（`draft.mjs` L7–L13、L393–L394） | 范围内：静态产物只用这些构件（C-13）；elintp 脚本零改动，V-4 实测 | 不合约时 elintp 报内容稿格式错误（退出 2），不覆盖旧页 |
| elintp skill → phoenix-diagram skill | 已核实足够 | elintp 自行查找并读取 phoenix-diagram 的 `SKILL.md`，不自动安装（`skills/elintp/SKILL.md` L183） | 范围内：两处文字同步两项能力 | 未安装、引擎不支持或流程失败 → 按 §6.3 降级并在交付说明写明原因 |
| 本机 Chrome | 已核实足够 | `browser::discovery::find`；缺失时回执 `unavailable`、退出 5 | 复用 | 与十二类相同 |
| summarize → elintp | 已核实足够 | summarize 以文字提出阅读面要求 | 零改动 | — |

登记位置：新诊断码 `change/conflict`、`change/dangling-relation`、`annotation/span-missing`、`annotation/span-overlap`（输入阶段）与 `artifact/change-mark`（产物检查）；新文案键见 §5.4、§5.9、§6.1；新元数据键 `changes`；新 SVG 属性 `data-change-ui`、`data-change-view`、`data-change`、`data-change-mark`、`data-change-when`、`data-change-hidden-in`、`data-change-count` 与 `data-annotation-*`。没有新的命令、参数、配置、环境变量或回执字段。

## 5. 核心机制（引擎）

### 5.1 变更标记的输入契约

`skills/phoenix-diagram/schemas/common.schema.json` 的 `$defs` 新增：

```json
"elementChange": { "enum": ["added", "removed", "changed"] },
"relationChange": { "enum": ["added", "removed"] }
```

四份 schema 各加一个可选属性 `change`：

| schema | 用 `elementChange` 的对象 | 用 `relationChange` 的对象 |
| --- | --- | --- |
| `workflow.schema.json` | `nodes[]` | `edges[]` |
| `lifecycle.schema.json` | `states[]` | `transitions[]` |
| `tree.schema.json` | `nodes[]` | —（连线由 `parent` 派生；换父用下面的 `parent_before`） |
| `erd.schema.json` | `entities[]`、`entities[].attributes[]` | `relationships[]` |

`tree.schema.json` 的节点另加可选属性 `parent_before`（引用 common 的 `id`），并加 `"dependentRequired": {"parent_before": ["parent"]}`：根节点不能写它（D19）。

其余对象（lane、phase、group、card、`meta`）与其余九份 schema 不加。关系写 `"change": "changed"`、未知取值、在不支持的对象上写 `change`，都由 schema 拒绝（`input/schema`，路径指到该字段）。「变更」只表示该项有变化：三种视图都显示输入里的同一份描述，不接受旧值字段。

### 5.2 有效标记与校验（`tools/phoenix-diagram/src/changes.rs` ★）

```rust
pub(crate) fn resolve(document: &Value, kind: DiagramType) -> Result<Option<Resolved>, Diagnostics>
```

`kind` 不是四类之一，或文档里没有任何 `change` 与 `parent_before` 时返回 `Ok(None)`。`input::semantics::validate` 在各图型检查之后调用它，把诊断并入返回（照 `chart_checks` 的调用位置，`tools/phoenix-diagram/src/input/semantics.rs` L228–L236）。

**有效标记**

- 节点：自身的 `change`。
- 关系（workflow 的 edge、lifecycle 的 transition、erd 的 relationship）：自身的 `change`。
- tree 的父子连线：`parent → 节点` 在子节点标为新增或删除时取同一标记；子节点写了 `parent_before` 时为新增；否则没有。`parent_before → 节点` 为删除。
- erd 字段：实体带标记（新增、删除或变更）时，全部字段取实体的标记；否则取字段自身的 `change`。

**tree 两个视图的树约束**：变更前由未标为新增的节点组成，父节点取 `parent_before`（有的话）否则 `parent`；变更后由未标为删除的节点组成，父节点取 `parent`。两者都须是以同一根为根、无环的树。根相同由「根不能写 `parent_before`」保证；每条连线的两端在它出现的视图里都存在，由下表的端点规则保证；变更后的无环由既有的 `tree/cycle` 检查（它检查的就是 `parent`）保证；变更前另做一次环检查。

**诊断**（阶段 `input`，每条消息写出涉及的对象与它会在哪个视图悬空）

| 诊断码 | 触发 | 路径 |
| --- | --- | --- |
| `change/dangling-relation` | 关系的某端被标为删除而关系没有标为删除；某端被标为新增而关系没有标为新增（两端一个新增一个删除时必然触发） | `/edges/<i>`、`/transitions/<i>`、`/relationships/<i>` |
| `change/dangling-relation` | tree：按上面的有效标记，父子连线的某端在该连线出现的视图里不存在（如父节点被标为删除，而子节点既没有标为删除也没有换父；`parent_before` 指向新增节点） | `/nodes/<i>/parent`、`/nodes/<i>/parent_before` |
| `change/conflict` | tree：标为新增或删除的节点写了 `parent_before`；`parent_before` 与 `parent` 相同 | `/nodes/<i>/parent_before` |
| `tree/cycle`（既有码） | tree：变更前视图的父子关系成环 | 环上第一个写了 `parent_before` 的节点的 `/nodes/<i>/parent_before` |
| `change/conflict` | erd：实体带标记，而它的某个字段写了不同的 `change` | `/entities/<i>/attributes/<j>/change` |
| `input/schema`（既有） | 关系标为变更、未知取值、不支持的对象、根节点写了 `parent_before` | schema 实例路径 |
| `input/unknown-reference`（既有） | 关系端点或 `parent_before` 不存在 | 该端点 |

`supportedFixes` 给出可直接采用的改法，例如「把 `/edges/3/change` 设为 `removed`」；tree 父节点被删除而子节点保留时，同时给出「标为删除」与「写 `parent_before` 并改 `parent`」两种。校验未过时不进入布局。

**计数**（D5）：三类各自的数量 = 有效标记为该类的节点数 + 作者写出的关系数 + erd 字段数 + tree 由 `parent_before` 产生的连线数（每个换父节点新增、删除各一）。

### 5.3 场景标注与说明带

`tools/phoenix-diagram/src/scene.rs` 的 `CompiledDiagram` 新增字段：

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub changes: Option<ChangeFacts>,

#[serde(rename_all = "camelCase")]
pub struct ChangeFacts {
    pub counts: ChangeCounts,                 // added / removed / changed
    pub nodes: BTreeMap<String, String>,      // 节点 id → 标记
    pub relations: Vec<Option<String>>,       // 与 scene.relations 同序
    pub fields: BTreeMap<String, String>,     // erd 字段的 source_path → 标记
    pub trunks: BTreeMap<String, String>,     // erd 干线容器 id → 该干线不存在的视图："before" 或 "after"
    pub junctions: Vec<Vec<String>>,          // 与 lifecycle 合流点同序，每项是该点不显示的视图
    pub band: ChangeBand,
}
pub struct ChangeBand { pub bounds: Rect, pub changes: Vec<Text>, pub before: Vec<Text>, pub after: Vec<Text> }
```

`layout::compile` 在 `attach_brands` 之后调用 `changes::attach(&mut diagram, input)`（`tools/phoenix-diagram/src/layout/mod.rs` L51 之后）：

1. `resolve` 返回 `None` 时直接返回，场景不变。
2. 把有效标记对到场景：节点按 `authored.id`；workflow、lifecycle、erd 的关系按 `authored.source_path` 末尾的下标；tree 的关系按 `source_path`（`/nodes/<i>/parent` 或 `/nodes/<i>/parent_before`）取 §5.2 的连线标记。
3. 派生图元。erd 干线：成员关系（`geometry_provenance` 里的 `/relationships/<i>`）全部为新增时记 `before`，全部为删除时记 `after`。lifecycle 合流点：经过该点（到折线的距离不超过 0.5）的关系里，在某个视图中可见的少于两条时，该点在这个视图不显示。
4. 说明带。位置在原画布下沿之下：左右各留 24，文字宽度上限为画布宽减 48，超出时用 `layout::tree::wrap_text` 换行（该函数与 `tokens` 改为 `pub(crate)`）。字号按画布宽度放大，系数 `k = max(1, 画布宽 ÷ 930)`（930 是通用可读性检查换算的阅读宽度，`check.rs` L617），使说明带在整图适配时仍可读。三组文字各自排版，带高取三组中行数最多的一组：`(16 + 行数 × 18 + 12) × k`；`view_box.height` 加上带高。文字不带 `data-source-path`。

| 视图 | 第一行（字号 12.5 × k，字重 700） | 其后（字号 11.5 × k） |
| --- | --- | --- |
| 变更视图 | 文案 `change.view.changes` | 计数：只列数量大于 0 的类别，每项是符号、文案与数量，如「+ 新增 2 · − 删除 1 · ~ 变更 1」 |
| 变更前 | `change.view.before` | 有标为变更的项时：文案 `change.note.changed`（「标为变更的 {count} 项显示的是输入中的描述，不代表变更前的取值。」） |
| 变更后 | `change.view.after` | 同上 |

**tree 的换父连线**（`layout/tree.rs`，只在有节点写了 `parent_before` 时生效，没有时布局结果逐字节不变）：

- 层级、深度与位置仍只按 `parent` 求解，换父节点排在变更后的父节点下。
- 写了 `parent_before` 的节点与被 `parent_before` 引用的节点不进入叶子堆叠的连续段（`structure` L282–L289），按普通子节点排，使换父连线的出入口都在节点正面，不穿过堆叠里的兄弟节点。
- 在按 `parent` 生成的关系之后，每个换父节点追加一条关系：`from` 为 `parent_before`，`to` 为该节点，`source_path` 为 `/nodes/<i>/parent_before`，`kind` 为 `tree-edge-before`，`facts` 为 `{"from","to"}`。路线只走不放节点的地方：从旧父节点的出线点进入它下一层之前的层间空隙，沿空隙走到画布外侧新增的一条通道（向下生长时在右侧，向右生长时在下方；`view_box` 相应加宽，只在有换父连线时加），沿通道到换父节点所在层之前的空隙，再进入节点正面的四分之三处，不与 `parent` 连线的入口重合。层间空隙里的横段与普通连线的横段错开 6。
- 渲染（§5.4）与检查按 `kind` 区分：`render/tree.rs` 的 `Hierarchy::of` 只用 `tree-edge`，换父连线沿用子节点的 `data-tree-ancestors` 等属性，随子节点一起折叠；`check/tree.rs` 把每个 `parent_before` 计为一条预期关系与一条画出的连线。

`render_with_layers` 在组装内嵌元数据后，`scene.changes` 为 `Some` 时加一个键 `"changes"`（`render.rs` L1464 之后）；为 `None` 时元数据与现在逐字节相同。

### 5.4 标记的图面（`tools/phoenix-diagram/src/render/changes.rs` ★）

对外的函数如下，`scene.changes` 为 `None` 时全部返回空：

| 函数 | 输出 | 调用处 |
| --- | --- | --- |
| `root_attributes(scene)` | ` data-change-ui="" data-change-view="changes"` | `render_svg_layers` 写根标签处（`render.rs` L874–L878） |
| `defs(scene)` | 一个 `<style>` | 同函数，在图型自己的 `defs` 之后（L884–L886） |
| `attribute(mark)`、`hidden_attribute(views)` | ` data-change="added"`、` data-change-hidden-in="before"` | 各渲染路径写元素的开始标签时 |
| `node_mark(scene, node)` | 节点徽标 | 通用渲染、tree、erd 各自在节点 `<g>` 结束之前 |
| `edge_marks(scene)`、`band(scene)` | 关系徽标与说明带 | `render_svg_layers` 在两条路径调用 `write_legends` 之前（`render.rs` L896、L936） |

**哪些图元带 `data-change`**：节点的 `<g>`；关系的路径、它外面的交叉衬底 `<g>`（有的话）与标签 `<g>`；tree 的连线与分支开关 `g[data-tree-toggle]`；erd 的字段行 `g[data-er-row]`。erd 干线与 lifecycle 合流点带 `data-change-hidden-in`。

**徽标**（都带 `data-change-mark="<标记>"`、`role="img"` 与取自文案 `change.mark.<标记>` 的 `aria-label`，内含同文的 `<title>`）：

- 节点：圆心在 `(bounds.x + bounds.width − 9, bounds.y + 9)`，半径 8，写在节点 `<g>` 的末尾（位置同 `delta_node_marker`）。符号是路径：`+` 为臂长 4 的十字，`−` 为长 8 的横线，`~` 为宽 8 的一段波浪线。
- 关系：圆心在路线第一个点偏移 `(9, −7)` 处，半径 6，臂长 3（位置同 `write_delta_markers`）。tree 随子节点新增或删除的连线不画徽标（子节点已有）；换父节点的两条连线各画一个关系徽标，位置在路线最后一段的中点旁偏移 9（同一父节点的连线共用起点，放在起点会分不清是哪条）。
- erd 字段行：符号中心在 `(实体 bounds.x + 5, 行中线)`，臂长 3；另画一条覆盖整行的底色带（照 `render/erd.rs` L399–L402 斑马纹的位置）。字段行左侧到键徽标之间有 10 的留白（`layout/erd.rs` `PAD_X` 12、`KEY_BADGE_INSET` 2）。

**说明带** `band(scene)`：`<g data-change-band="">`，内含三个 `<g data-change-when="changes|before|after">`，各自是 §5.3 的文字。计数项的 `<tspan>` 带 `data-change-count="<标记>"`。

**样式**（每条规则以 `svg[data-change-ui]` 开头，只用 `static_style::supported_declaration` 清单内的声明，不用 `:has()`、`:not()` 与 `!important`；颜色取既有主题变量）：

- 变量：新增 `var(--backend-stroke)`，删除 `var(--security-stroke)`，变更 `var(--cloud-stroke)`。
- `[data-change-view="changes"]` 下：新增的节点框与关系线用新增色、线宽 2.5；删除的用删除色、虚线 `6 4`，所在 `<g>` 的 `opacity` 为 0.62；变更的节点框用变更色、点线 `2 3`、线宽 2.5。被既有预设规则压过时提高选择器特异性。
- `[data-change-view="before"]` 下：`[data-change="added"]`、`[data-change-hidden-in~="before"]`、`[data-change-mark]`、`[data-change-when="changes"]`、`[data-change-when="after"]` 为 `display: none`。
- `[data-change-view="after"]` 下：`[data-change="removed"]`、`[data-change-hidden-in~="after"]`、`[data-change-mark]`、`[data-change-when="changes"]`、`[data-change-when="before"]` 为 `display: none`。
- `[data-change-view="changes"]` 下：`[data-change-when="before"]`、`[data-change-when="after"]` 为 `display: none`。
- 变更前、变更后没有描边与透明度的规则，被保留的元素是普通外观。

**内容检查**（`tools/phoenix-diagram/src/check/change.rs` ★）：`pub(super) fn content(svg, metadata) -> Vec<ContentFinding>`，在 `check.rs` L1042–L1056 的图型内容检查之后调用，失败码 `artifact/change-mark`，路径指到 `/metadata/changes/...`：

- SVG 根带 `data-change-ui` 当且仅当元数据有 `changes`。
- 每个节点 `<g>` 的 `data-change` 等于 `changes.nodes[id]`；每条 `path[data-edge-key]` 的等于 `changes.relations[key]`；erd 每个字段行的等于 `changes.fields[source_path]`。
- 三类徽标 `[data-change-mark]` 的个数分别等于 `changes.counts`；说明带里 `[data-change-count]` 的数字与之相同。
- 说明带恰有一个，含三组；`changes.counts.changed` 大于 0 时变更前、变更后两组各多一段说明。

### 5.5 变更视图的静态图（`tools/phoenix-diagram/src/render/static_style.rs`）

带标记输入的静态图走既有 `static` 命令，根属性保持 `data-change-view="changes"`。三处小改动，都不影响没有这些属性的图：

- `interactive_selector` 的 `RUNTIME` 清单（L341–L381）加 `data-change-view="before"`、`data-change-view="after"`：这两个视图的规则不进入静态图。
- `drop_element`（L1555）：带 `data-change-when` 且值为 `before` 或 `after` 的元素整体丢弃。
- `supported_declaration`（L629）加 `white-space`（§5.9 使用）。

静态图因此只含变更视图的图元、徽标、计数与视图名，没有另外两组文字。tree 在静态图里全部展开的既有行为不变。

### 5.6 `annotation` 的输入契约（`skills/phoenix-diagram/schemas/annotation.schema.json` ★）

照 `chart.schema.json` 的写法：顶层、`meta`、句子与标注对象均 `additionalProperties: false`。

| 字段 | 约束 | 含义 |
| --- | --- | --- |
| `schema_version` | `const 1` | — |
| `diagram_type` | `const "annotation"` | — |
| `meta` | 必填 `title`；可选 `locale`、`translations`、`subtitle`、`output`、`animation`、`visual_preset`、`quality_profile`（引用 common 的定义）；没有 `repository` | — |
| `sentences` | 必填，非空数组 | 一句或几句原文，自上而下排列 |
| `sentences[].id` | common 的 `id` | 句子标识 |
| `sentences[].text` | 非空字符串 | 原文，逐字照录 |
| `sentences[].title` | 非空字符串，可省 | 这一句上方的标题 |
| `sentences[].aside` | 非空字符串，可省 | 标题行右侧的旁注 |
| `sentences[].footnote` | 非空字符串，可省 | 这一句全部标注之后的脚注 |
| `sentences[].annotations` | 必填，非空数组 | 这一句的标注 |
| `annotations[].span` | 非空字符串 | 被标注的词段，须是 `text` 的子串 |
| `annotations[].occurrence` | 整数，`minimum 1`，可省，缺省 1 | 词段在 `text` 中多次出现时指明第几处 |
| `annotations[].note` | 非空字符串 | 点评 |
| `annotations[].issue` | 布尔 | 把这处标为问题 |
| `annotations[].fix` | 非空字符串 | 改法；只能与 `issue: true` 同时出现（`if`/`then`） |
| 每条标注 | `anyOf`：有 `note`，或有 `fix` | 至少有点评或改法 |
| `cards` | common 的 `cards` | 与其他图型相同 |

最小示例 `skills/phoenix-diagram/examples/minimal.annotation.json` ★：两句；第一句三条标注，其中一条 `issue: true` 带 `fix`；第二句带 `title`、`aside`、`footnote` 与一条标注；`meta` 为 `{"title": …, "output": "minimal.annotation.html", "quality_profile": "showcase", "locale": "en"}`。

### 5.7 `annotation` 的语义校验（`tools/phoenix-diagram/src/input/semantics.rs`）

`validate` 的集合表增加 `DiagramType::Annotation => ("sentences", "relationships")`，句子 `id` 走通用的 `checks.unique`；图型分派增加 `annotation_checks(nodes, &mut checks)`，照 `chart_checks` 写。

词段定位：在句子的 `text` 里从左到右找 `span` 的不相交出现，取第 `occurrence` 处，得到字符区间。

| 诊断码 | 触发 | 路径 |
| --- | --- | --- |
| `annotation/span-missing` | `span` 不在 `text` 中；或 `occurrence` 大于出现次数（消息写出实际出现次数） | `/sentences/<i>/annotations/<j>/span`；`/sentences/<i>/annotations/<j>/occurrence` |
| `annotation/span-overlap` | 同一句里两条标注的字符区间相交（部分重叠、包含、相同）；报在输入顺序靠后的一条，`evidence` 给出另一条的下标与两个区间 | `/sentences/<i>/annotations/<j>` |
| `input/schema`（既有） | 缺 `note` 与 `fix`、`fix` 没有 `issue: true`、空字符串等 | schema 实例路径 |
| `input/duplicate-id`（既有） | 句子 `id` 重复 | `/sentences/<i>/id` |

### 5.8 `annotation` 的布局（`tools/phoenix-diagram/src/layout/annotation.rs` ★）

入口 `pub fn compile(input: &DiagramInput, _options: &CompileOptions) -> Result<CompiledDiagram, Diagnostics>`，照 `layout/chart.rs` 的 `compile` 组织：读输入、定位词段、排版、组装场景。

**编号**：全图统一编号，按句子顺序、句内按词段起点从 1 起。行内编号的文字是编号本身，问题标注在其后加 `!`（如 `2!`）。

**版面**（画布宽 760，左右边距 28，正文栏宽 704；自上而下）

- 标题行（有 `title` 或 `aside` 时）：标题字号 13、字重 700，左对齐；旁注字号 11、右对齐，与标题同一基线；两者估计宽度之和超过栏宽时旁注另起一行。行高 22。
- 原文行：字号 15，行高 26。把原文按 `layout::tree::tokens` 切成不可断开的片，行内编号紧跟所属词段的最后一片；逐片累加估计宽度（原文按 `text_units × 15 × 0.6`，编号按 `text_units × 10 × 0.6 + 4`），超过栏宽时在该片之前换行；单片超过栏宽时按字符切开。行尾空白留在行尾，不删除。每行记录它包含的片段：`plain`、`span`（带编号）、`marker`（带编号与是否问题），以及在原文中的字符起止。
- 标注卡片：每条标注一张，按编号顺序纵向排列，卡片间距 10。卡片左缩进 16，宽为栏宽减 16，内边距 10。卡片内依次是：编号（字号 11、字重 700）与引述的词段（字号 12.5、字重 650，用 `tree::wrap_text` 换行）；有 `fix` 时紧接一行「→ 改为」加改法（文案 `annotation.fix`）；有 `note` 时是点评（字号 11.5）。行高 17。问题标注的卡片右上角有文案 `annotation.issue`（「问题」，字号 10、字重 700）。
- 脚注（有 `footnote` 时）：字号 11，卡片之后间距 10，换行同上。
- 句与句之间间距 30。画布高为内容总高加上下边距 28。

**场景**

- `nodes`：每条标注一个，顺序同编号。`authored` 为 `{id: "<句子 id>-<句内序号>", source_path: "/sentences/<i>/annotations/<j>", kind: "annotation-issue" 或 "annotation-note", facts: 原始标注}`；`bounds` 是卡片；`label` 是词段；`text` 依次是引述的词段（`…/span`）、改法（`…/fix`）、点评（`…/note`）。
- `containers`：每句一个 `annotation-sentence`，`id` 为句子 `id`，`members` 是该句的节点，`bounds` 是这一句的整块，`text` 是标题、旁注与脚注。
- `relations`、`legends` 为空；`cards` 用 `super::tree::cards(document)`；`reader_fit` 为 `"width-first"`。
- `facts`：`scene.rs` 新增

  ```rust
  Annotation {
      sentences: Vec<Value>,  // {"id", "text", "lines": [{"baseline", "runs": [{"kind", "start", "end", "number"?, "issue"?}]}],
                              //  "annotations": [{"number", "node", "start", "end", "issue"}]}
  }
  ```

### 5.9 `annotation` 的渲染与检查

**渲染**（`tools/phoenix-diagram/src/render/annotation.rs` ★），照 `render/chart.rs`：`pub(super) const RENDERER: TypeRenderer`，四个钩子。

- 根属性：` data-annotation-ui="" data-reader-min-text="7.5" data-node-kind-derived=""`。
- `defs`：一个 `<style>`，每条规则以 `svg[data-annotation-ui]` 开头，只用支持清单内的声明，类名用 `an-` 前缀。其中 `.an-sentence { white-space: pre; }` 让原文里的连续空格按原样显示。照 `render/chart.rs` 加一条去掉节点框阴影的规则。
- 原文：每行一个 `<text class="t-primary an-sentence" data-annotation-line="<句子 id>" data-source-path="/sentences/<i>/text" x y font-size="15">`，行内片段依次是 `<tspan>`：
  - 普通文字：不带属性。
  - 词段：`class="an-span"`（问题为 `an-span an-issue`）、`data-annotation-ref="<编号>"`、`font-weight="700"`；问题词段用 `security` 色槽，其余用 `backend` 色槽。一个词段跨行时在两行各有一个 `<tspan>`。
  - 行内编号：`class="an-marker"`、`data-annotation-marker="<编号>"`、`font-size="10"`、`dy="-5"`；它之后的第一个 `<tspan>` 带 `dy="5"` 回到基线。
- 标题、旁注、脚注：`<text class="t-primary">` 与 `<text class="t-muted">`，带各自的 `data-source-path`。
- 每条标注的 `<g>` 照 `render/chart.rs` 行节点的写法：`id="node-<id>"`、`data-node-id`、`data-node-label`（词段）、`tabindex="0"`、`role="button"`、`aria-label`（文案 `node.focus.detail`）、`aria-pressed="false"`、`data-node-kind`（问题为 `security`，其余 `backend`）、`data-node-sublabel`（点评，或「改为 …」）、`data-node-context`（文案 `node.context.annotation.note` 或 `node.context.annotation.issue`）、`data-source-path`、`data-annotation-number`，问题另加 `data-annotation-issue=""`，内含 `<title>`。
- `<g>` 内：卡片 `<rect data-graph-role="node-body" class="c-<色槽> an-card">`（问题卡片加 `an-card-issue`，虚线边框）；编号 `<text class="an-number">`；引述的词段 `<text data-node-label="">`；改法 `<text class="an-fix">`；点评 `<text data-detail="context" class="t-muted">`；问题字样 `<text class="an-issue-tag">`。
- 图例符号：空函数（没有图例）。

静态图：走既有 `static` 命令与转换，不新增代码路径；`white-space` 已在 §5.5 加入支持清单。

五语言目录新增的键（五份都写译文）：`diagram.description.annotation`、`annotation.issue`、`annotation.fix`、`node.context.annotation.note`、`node.context.annotation.issue`。

**内容检查**（`tools/phoenix-diagram/src/check/annotation.rs` ★）：`pub(super) fn content(svg, metadata) -> Vec<ContentFinding>`，在 `check.rs` L1043 的分派里登记；失败码沿用 `artifact/semantic-text`，路径指到 `/metadata/facts/sentences/<i>` 或其下的标注。

- 每句：按文档顺序拼接该句各行里除 `[data-annotation-marker]` 之外的全部 `<tspan>` 文字，结果与 `facts.sentences[i].text` 逐字相同。
- 每条标注：`[data-annotation-ref="<编号>"]` 的文字拼起来等于词段，它在句中的字符起点等于 `facts` 里的 `start`；恰有一个同编号的行内编号，紧跟在该词段的最后一个 `<tspan>` 之后；问题标注的行内编号以 `!` 结尾。
- 每张卡片：`data-annotation-number` 与 `facts` 相同；卡片文字含词段，有改法、点评时含改法、点评；`data-annotation-issue` 与问题字样当且仅当 `issue` 为真时存在。
- 编号从 1 起连续，卡片顺序与编号一致。

### 5.10 登记点与兼容

照 `chart` 的新增提交 `1d9e3f57` 逐项补 `annotation`：

| 文件 | 改动 |
| --- | --- |
| `tools/phoenix-diagram/src/input.rs` | `DiagramType::Annotation`；`ALL` 长度 13；`as_str` 返回 `"annotation"` |
| `tools/phoenix-diagram/src/schema.rs` | `include_str!` 新 schema（照 L53–L56 的 `chart` 一项） |
| `tools/phoenix-diagram/src/input/semantics.rs` | §5.7；§5.2 的 `resolve` 调用 |
| `tools/phoenix-diagram/src/layout/mod.rs` | `pub mod annotation;` 与 `compile` 分派；`changes::attach` 调用 |
| `tools/phoenix-diagram/src/render.rs` | `mod annotation;`、`mod changes;`、`type_renderer` 分派；§5.3、§5.4 的调用 |
| `tools/phoenix-diagram/src/check.rs` | `mod annotation;`、`mod change;`、两处调用 |
| `tools/phoenix-diagram/src/scene.rs` | `DiagramFacts::Annotation`；`CompiledDiagram.changes` 与 `ChangeFacts` |
| `tools/phoenix-diagram/src/evidence.rs` | `collection` 对 `Annotation` 返回 `None`（与 `Statechart`、`Chart` 同分支，L143） |
| `tools/phoenix-diagram/src/cli.rs` | `HELP` 的 `Types:` 行加 `annotation`（L21）；`guide` 的提示词加一组：`annotation`、`annotate`、`wording`、`word by word`、`逐词`、`标注`、`点评`、`用词`、`措辞` |
| 其余对 `DiagramType` 穷举的 `match` | 按编译错误逐处补齐，`annotation` 与 `chart` 取同一分支（没有关系、没有品牌） |
| `tools/phoenix-diagram/tests/cli.rs` L123、L145 附近 | `Types:` 行断言；`guide` 用例加一条逐词点评场景 |
| `tools/phoenix-diagram/tests/check_anchors.rs` L141 附近 | `example(kind)` 加最小示例 |
| `tools/phoenix-diagram/tests/support/input_corpus.rs` | `minimal` 给一份两句的 `annotation`；`collections` 返回 `("sentences", "relationships")` |
| `scripts/build-phoenix-diagram.mjs` L106、`test/phoenix-diagram/build.test.js` L126 | schema 清单加 `annotation` |
| `test/phoenix-diagram/skill.test.js` | `types` 加 `annotation`；`references` 加 `annotation`、`change-view`；`collections` 加 `annotation: ['sentences']`；`annotation` 与 `chart` 一样不要求关系；描述前 100 字的词表加「逐词标注」；两个用例名改为十三类 |

兼容（WORK-N3/AC-5，判定口径见 D17）：没有 `change` 与 `parent_before` 的输入在 `resolve` 处返回 `None`，tree 布局的两处改动不生效，场景、元数据与 SVG 不经过任何新分支；`scene.changes` 缺省时不序列化。`static_style.rs` 的三处改动只对带新属性的图元与新声明生效。模板因视图控件、脚本片段与目录文案变化，这是唯一会改变十二类 HTML 字节的来源。

## 6. 阅读器、skill 文字与 elintp

### 6.1 视图切换（`tools/phoenix-diagram/viewer/`）

**页面结构**（`template.source.html`）：在 `.header` 内、`{{PHOENIX:SUBTITLE}}`（L370）之后加一组默认隐藏的控件，随页头在文档流里换行，不占固定工具栏的位置：

```html
<div class="change-view-switch no-print" id="change-view-switch" role="group" aria-label="{{i18n:change.view.label}}" hidden>
  <button type="button" data-change-view-select="changes" aria-pressed="true">{{i18n:change.view.changes}}</button>
  <button type="button" data-change-view-select="before" aria-pressed="false">{{i18n:change.view.before}}</button>
  <button type="button" data-change-view-select="after" aria-pressed="false">{{i18n:change.view.after}}</button>
  <span class="change-view-status" id="change-view-status" role="status" aria-live="polite"></span>
</div>
```

**脚本片段** `change-view.js` ★：在 `scripts/assemble-phoenix-diagram-reader.mjs` 的 `fragments` 中排在 `tree-branches.js` 之后、`focus.js` 之前，标记 `/* PHOENIX:CHANGE_VIEW */`；模板在 L949 与 L951 之间加同名标记。照 `tree-branches.js` 的结构写：

- 主图没有 `data-change-ui` 时返回 `{ active: false }`，控件保持隐藏。
- 否则显示控件。点击或用键盘激活按钮时调用 `set(view)`：把主图根的 `data-change-view` 设为该值，更新三个按钮的 `aria-pressed`。视图不写入本地存储，也不进入分享链接。
- `reveal(id)`：该节点带 `data-change`，且在当前视图被隐藏（变更前的新增、变更后的删除）时，`set('changes')` 并在状态区写文案 `change.view.returned`，返回真。
- 用 `MutationObserver` 监听主图根的 `data-route-active`、`data-reach-active`：当前不是变更视图而其中之一出现时，`set('changes')` 并写同一文案。
- 对外 `PhoenixDiagram.changeView = { active, view, set, reveal }`。

`focus.js` 在 L1401–L1402 调用 `treeBranches.reveal` 的同一处，加一句对 `PhoenixDiagram.changeView.reveal(id)` 的调用。导出与打印不需要改：根属性与 SVG 内的样式随克隆保留。

**样式**（`viewer.css`）：`.change-view-switch` 一组规则——横向排列、可换行，按钮外观沿用工具栏按钮的变量，`[aria-pressed="true"]` 有明显的选中态与可见的键盘焦点环。选择器只用 `.change-view-` 开头的类名，不引用 SVG 根。

**文案键**（五份目录都写译文）：`change.view.label`、`change.view.changes`、`change.view.before`、`change.view.after`、`change.view.returned`、`change.count.added`、`change.count.removed`、`change.count.changed`、`change.note.changed`（占位符 `{count}`）、`change.mark.added`、`change.mark.removed`、`change.mark.changed`。写完运行装配脚本。`tools/phoenix-diagram/viewer/README.md` 的「14 个 classic JavaScript 片段」改为 15 个。

### 6.2 phoenix-diagram skill 与文档的文字

- `skills/phoenix-diagram/SKILL.md`：
  - 描述改为：`用于架构、工作流、时序、数据流、状态、状态机模型检查、ER、类、树、时间线、耗时瀑布图、数值图表、逐词标注及 Mermaid 转换；不用于仪表盘、多图联动或其他 infographic。缺必需事实时仅提必要问题，没有真实数值不画数值图表；齐备后交付经 Rust、浏览器与实际看图核对的独立 HTML，可另取可内联的静态 SVG。四类结构图可标记变更。品牌采集、本地预览和架构比较仅在明确请求时启用。`（199 字；前 100 字含 `skill.test.js` 要求的全部词与「逐词标注」）。
  - 「选择图型」表加一行 `annotation`：「对一句或几句原文逐词点评：词段、点评、问题与改法」，示例链到同目录下 `examples/` 里的 `minimal.annotation.json`。
  - 表后的说明段加一句：要讲「改完以后和现在差在哪」时，在 workflow、lifecycle、tree、erd 的一份输入里标记新增、删除、变更，读[变更视图](references/change-view.md)；不画前后两张图，也不改用架构比较。
  - 「最短创作路径」第 1 步加一句：`annotation` 另读[逐词标注](references/annotation.md)；它没有 `sources`、品牌或架构比较。
  - 第 4 步看图一句之后加：带变更标记的图，两张截图是变更视图；另核对徽标、计数与图下方的视图说明是否与输入相符。
- `skills/phoenix-diagram/references/change-view.md` ★（照 `skills/phoenix-diagram/references/chart.md` 的体例，只写创作与交付规则）：
  - 适用图型与字段：四类图型里哪些对象能写 `change`、可取的值；关系只能是新增或删除，改变一条关系写成删除旧的、新增新的。
  - 标记只取自授权材料；不推断改动的原因、风险或影响；「变更」只表示有变化，三种视图显示同一份描述，不写旧值。
  - 端点规则（D6）与 erd 的继承规则（实体的三类标记都带给全部字段，字段写了不同标记会被拒绝）；tree 的节点换父写 `parent_before`（D19），树根不能在变更前后更换；容器不能标记（D7）；`mainPath`、`semanticChecks` 按全部元素书写。
  - 读者看到什么：变更视图的徽标与计数；可切到变更前、变更后；这两个视图在被隐藏元素处留空；导出与打印是当前所选视图；静态图是变更视图。
  - 交付答复说明三类变更各有多少、有没有标为变更的项。
- `skills/phoenix-diagram/references/annotation.md` ★：
  - 原文逐字照录，不改写、不纠错；点评与改法取自材料，工具不判断哪里写得不好。
  - `span` 写原文里的确切词段；同一词段出现多次时写 `occurrence`；词段之间不能重叠或嵌套。
  - 把某处标为问题时写 `issue: true`，改法写 `fix`；标题、旁注、脚注各自的用途。
  - 不用于整篇文档的批注或审校。
- `skills/phoenix-diagram/schemas/README.md` L3、L5、L7：类型清单加 `annotation`，改为十三类；「其余十类为 v1」改为十一类；语义门一句加「变更标记的端点一致与 erd 字段继承（`change/*`），逐词标注的词段定位与重叠（`annotation/*`）」。
- `skills/phoenix-diagram/references/extending-types.md` L3、L7、L12，`skills/phoenix-diagram/references/preview.md` L15，`skills/phoenix-diagram/references/viewer-runtime.md` L18：十二类改为十三类。`viewer-runtime.md` 另加一段视图切换：三种视图、键盘操作、探索功能回到变更视图的规则（D8）、导出与打印为当前视图。
- `skills/phoenix-diagram/references/static-figure.md` 与 `skills/phoenix-diagram/references/delivery-contract.md` 描述静态产物的一句各补：带变更标记的输入，静态图呈现变更视图，不含变更前、变更后。`delivery-contract.md` L22 一段补：带变更标记的图，两张截图是变更视图。
- `docs/phoenix-diagram.md` L3：类型清单加 `annotation`、改为十三类，并加两句——`annotation` 对一句或几句原文逐词点评；workflow、lifecycle、tree、erd 可在一份输入里标记新增、删除、变更。L115：十二类改为十三类，「有关系的九类」不变；在树的折叠一句后加「带变更标记的图可在变更视图、变更前、变更后之间切换，导出与打印为当前所选视图」。
- `README.md` L216：「十二类 Rust 图解（含状态机模型检查与数值图表）」改为「十三类 Rust 图解（含状态机模型检查、数值图表与逐词标注），四类结构图可标记变更」。`bin/xgent-skills.js` L301：「十二类图解」改为「十三类图解」。
- 运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle`（C-8）。

### 6.3 elintp 的选图规则（`skills/elintp/SKILL.md`「出图」）

- 对照表（L185–L197）加两行：
  - 「结构的改动：流程、状态流转、目录层级或数据表在改动前后的差异」→ `workflow`、`lifecycle`、`tree`、`erd` 的变更标记。
  - 「对一句或几句原文的逐词点评」→ `annotation`。
- 数值图表一段（L201）之后加一段：材料描述结构的改动——新增、删除、变更了哪些步骤、状态、目录项、表或字段——且 phoenix-diagram 可用时，按其规则在对应图型的一份输入里标记变更，画一张变更视图，不画前后两张；标记只取自材料，不推断改动的原因或影响。材料含需要逐词点评的句子时用 `annotation`，原文逐字照录，点评取自材料。已安装的 phoenix-diagram 不支持变更标记或 `annotation`，或按其流程无法交付时：结构改动改用三列表格（新增、删除、变更）或自绘 `svg:`，逐词点评改用表格（词段、点评、改法），并在交付说明写明原因。
- 内联图一段（L203）补一句：变更视图的静态图只有变更视图，没有变更前、变更后；读者需要切换时用默认的 `reader:`。
- 交付前自检第 9 条与交付说明的措辞不改：两项能力属于「上表的图」。

### 6.4 整合清单在本期的落点

| PRD §5.3 条目 | 本期改动 | 对原则的影响 | 核对 |
| --- | --- | --- | --- |
| C6 · elintp「出图」 | 对照表加变更视图与逐词标注；补降级写法 | 不自动安装、不新增安装检测脚本、默认嵌阅读器不变 | V-4；V-6 第 4 步读 diff |
| C7 · elintp 的描述与触发 | 不改 | — | `git diff` 中 elintp frontmatter 无变化 |
| C10 · phoenix-diagram 的图型与描述边界 | 新增 `annotation` 与四类图型的变更标记；改写描述；拒绝参考工具的固定数量上限与 `api` | 仪表盘与其他 infographic 仍排除；不设节点、词段、标注配额 | `skill.test.js`；V-6 第 4 步读描述全文与前 100 字 |
| C12 · phoenix-diagram 的输出形态 | 静态图覆盖变更视图与逐词标注 | 独立 HTML 的交付不变 | V-2、V-3、V-6 |

## 8. 失败模式、发布与回滚

运行期失败沿用单次命令语义：校验或布局失败时返回带阶段与路径的诊断，不写产物，由作者改 candidate 后重跑；没有并发写、后台任务或跨资源提交。本期不引入新的运行期失败模式：elintp 脚本不改，静态图只用它已接受的构件；旧引擎不认识 `change` 字段与 `annotation` 时按 schema 或类型错误拒绝，elintp 按 §6.3 降级。

- 发布顺序：引擎与 skill 随同一个 npm 包发布；本计划不发布。发布时按 [docs/phoenix-ui.md](../phoenix-ui.md) 的 `release:phoenix` 流程，源码输入已变，不能用 `--reuse-engines`。CHANGELOG 在发布时记录变更视图、逐词标注与图型增至十三类。
- 放量/切换门：无。
- 回滚门：回退提交即可。已生成的 HTML 与静态图是普通文件，不受影响。

## 9. 验证

### 9.1 行为与接口验证

**环境**

- 在仓根执行；工具已按 `node scripts/prepare-phoenix-diagram.mjs --prepare` 准备好，缺失即失败（C-7）。
- `CARGO` 表示 `mise exec --no-deps -- mbx`，`MF` 表示 `--manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline`。
- 开发引擎：`mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/elintp-p5 --sync-dist`，引擎为 `E=local/phoenix-diagram/elintp-p5/phoenix-diagram-darwin-arm64`。输出目录须为新目录或空目录，每次构建换一个目录名后缀；耗时未知。构建后用 `git checkout --` 还原被 `--sync-dist` 改写的 `dist/` 根部许可文件。
- 产物目录：`OUT=local/phoenix-diagram/elintp-p5-out`（`local/` 不入库）。
- 看图由具备图像能力的 Agent 实际打开截图完成；没有图像能力时如实保留未完成。
- 阅读器 HTML 的图面指纹（D17）用下面的短命令取，输出每个文件的主图 SVG 加内嵌元数据的 SHA-256：

  ```sh
  node -e 'const fs=require("fs"),c=require("crypto");for(const f of process.argv.slice(1)){const h=fs.readFileSync(f,"utf8"),a=h.indexOf("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox="),b=h.indexOf("</svg>",a)+6,m=/<script[^>]*id="phoenix-diagram-metadata"[^>]*>([\s\S]*?)<\/script>/.exec(h);console.log(c.createHash("sha256").update(h.slice(a,b)).update(m[1]).digest("hex"),f)}' "$@"
  ```

**基线（M1 动工前执行一次，结果写进 M1 记录）**

在未改动的 HEAD 上按上面的命令构建到 `local/phoenix-diagram/elintp-p5-base`，对十二类最小示例各执行：

1. `deliver <type> skills/phoenix-diagram/examples/minimal.<type>.json local/phoenix-diagram/elintp-p5-base-out/<type>.html --json`，记录图面指纹。
2. `static <type> … <type>.embedded.svg --json` 与 `static <type> … <type>.system.svg --font-policy system --json`，记录两份 SVG 的 SHA-256。

**批内最小反馈**

- 单个 Rust 测试：`$CARGO test $MF --test change <过滤词>`、`--test annotation <过滤词>`；只编译：`$CARGO test $MF --no-run`。
- 单个 candidate：`$CARGO run $MF --bin phoenix-diagram -- validate <type> <candidate.json> --layout-json --json`。
- Node 单文件：`mise exec --no-deps -- node --test <文件>`；阅读器装配核对：`mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check`。
- 开发构建只在批末或收口；整 crate 测试与 clippy 只在 V-6 收口运行。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **图解作者把一条流程的改动讲成一张图并交付。** 输入：作者自己写的 workflow candidate，内容取自 PRD §1.1 与 §2.3 的 elintp 主路径——在既有「生成页面 → 自检 → 交付」上，去掉「读者回到对话描述是哪一段」一步，新增「在页面标注并提交」「确认审阅完成」两步及其连线，把「重讲」一步标为变更；至少一处删除、两处新增、一处变更，关系的改动写成删旧加新。覆盖 WORK-F7/AC-1、AC-2、AC-4、AC-5、AC-6。结果：<br>• 得到 HTML、两张截图与 summary，自动阶段全过；看两张图：每个被标记的步骤与连线带 `+`、`−`、`~` 徽标，图下方写着「变更视图」与三类数量，数量与 candidate 一致；写观察并登记后 `deliveryStatus: complete`<br>• 在 Chrome 打开 HTML：标题下有三个视图按钮；切到变更前，新增的两步与连线不在图上，被删除的一步还在，没有徽标，图下方写「变更前」与「标为变更的 1 项显示的是输入中的描述…」；切到变更后相反；未标记的步骤在三种视图里位置不变；只用键盘能切换并看到焦点<br>• 在变更前导出 SVG 与 PNG，打开导出文件：是变更前的图，带「变更前」字样；打印预览同样是当前视图<br>• 在变更前搜索一个新增的步骤并选中：页面回到变更视图并提示；在变更后启动 Route：同样回到变更视图<br>• 把同一 candidate 去掉全部 `change` 再交付：页面没有视图按钮，图下方没有说明带<br>容易犯的错：把一条连线标成 `changed`；删掉一步却保留连到它的连线；新增一步但连线没标新增。三者都退出非 0，诊断码分别是 `input/schema`、`change/dangling-relation`、`change/dangling-relation`，路径指到那条连线，且没有产出 HTML | 1. `"$E" finalize workflow $OUT/review-flow.json $OUT/review-flow.html --json`，读 `$OUT/review-flow.finalize-summary.json`<br>2. 打开两张 1440×900 截图，写审核记录，`"$E" review-record $OUT/review-flow.html --record … --json`<br>3. 用 Chrome 打开 `$OUT/review-flow.html` 做上述操作；窗口另设为 360 px 宽看一次视图按钮是否可见、可点<br>4. 无标记版本运行第 1 步到新路径并打开<br>5. 三份错误输入各运行第 1 步到新路径，确认退出码、诊断码与路径，且输出文件不存在 | M1 交付齐备、开发引擎构建后 |
| V-2 | 场景 E2E | **图解作者讲状态、目录与数据表的改动，并取能放进别的文档的图。** 输入：作者自己写的三份 candidate——lifecycle：一条审阅稿的状态流转，新增一个状态及进出它的流转、删除一条流转；tree：`skills/phoenix-diagram` 目录在本期的变化（新增 `skills/phoenix-diagram/references/change-view.md`、`skills/phoenix-diagram/references/annotation.md` 等，取自本计划 §1.2），含一个新增目录及其全部子项，以及一个从一处目录移到另一处的既有文件（写 `parent_before`）；erd：两张表，一张新增（含三个字段），另一张新增一个字段、删除一个字段、一个字段标为变更，外加一条新增关系。覆盖 WORK-F7/AC-1 至 AC-4、WORK-F9/AC-6。结果：<br>• 三份都得到 HTML 并看图登记：徽标与数量对得上；erd 新增表的每个字段行都有 `+`，只标字段的那张表表头没有徽标；tree 新增目录下的每一项都有 `+`，被移动文件的旧、新两条连线各带 `−`、`+`，该文件本身没有徽标<br>• 在 Chrome 逐份切换三种视图：变更前没有新增内容，变更后没有删除内容；lifecycle 没有悬空的线头或孤立的合流点；erd 没有只剩一截的干线；tree 被隐藏分支的开关一并消失；被移动的文件三种视图都在，旧父连线只在变更视图与变更前出现、新父连线只在变更视图与变更后出现<br>• 四类（含 V-1 的 workflow）各取 `embedded` 与 `system` 两份静态图，自动阶段全过；内联进空白无脚本 HTML 后亮、暗下是变更视图，带徽标、数量与「变更视图」字样，与阅读器截图一致；文件里没有变更前、变更后的文字；每份 SVG 字节数不超过同一 candidate 阅读器 HTML 的 50%；静态图看两张、登记后 `deliveryStatus: complete`<br>容易犯的错：tree 把父目录标为删除却漏标一个子项；erd 在新增的表里把一个字段标为删除；lifecycle 把流转标成 `changed`。分别得到 `change/dangling-relation`（指到该子项的 `parent`）、`change/conflict`（指到该字段）、`input/schema` | 1. `"$E" finalize <type> $OUT/<name>.json $OUT/<name>.html --json`，看图并登记<br>2. 用 Chrome 打开各 HTML 切换视图<br>3. `"$E" static <type> $OUT/<name>.json $OUT/<name>.embedded.svg --json` 与 `--font-policy system` 写到 `<name>.system.svg`<br>4. 写 `$OUT/host.html`（`<html data-theme="light">` 内联各静态图，无脚本），用 Chrome 打开；改为 `dark` 再看<br>5. `wc -c $OUT/<name>.*.svg $OUT/<name>.html`；`grep -c 'data-change-when="before"' $OUT/<name>.embedded.svg` 应为 0<br>6. 打开静态图的两张截图，`review-record` 登记<br>7. 三份错误输入各运行第 1 步到新路径 | M2–M3 交付齐备、开发引擎构建后 |
| V-3 | 场景 E2E | **图解作者点评一句文案的三处用词。** 输入：作者自己写的 `annotation` candidate——第一句取 `skills/phoenix-diagram/SKILL.md` 的描述句（中英混排、超过一行），三条标注，其中一条标为问题并给改法；第二句是一句英文，带标题、旁注与脚注，同一个词出现两次、用 `occurrence` 标注第二处，原文里有一处连续两个空格。覆盖 WORK-F8/AC-1 至 AC-4、WORK-F9/AC-6。结果：<br>• 得到 HTML 并看图登记：原文逐字、顺序不变，换行后没有字被裁掉或叠在一起；三处词段加粗并各跟一个编号，问题那处的编号带 `!`；下方卡片按编号排列，每张引述词段并写点评；问题卡片有「问题」字样与虚线边框，改法紧跟在引述的词段下面；第二句标注的是第二处出现；连续空格按原样显示<br>• 在 Chrome：搜索一个词段能定位到它的卡片，选中卡片后详情里是点评；切换亮暗与预设后原文、编号、卡片都可读；导出 SVG 与 PNG 得到完整的图<br>• 取 `embedded` 与 `system` 两份静态图，自动阶段全过；内联进无脚本 HTML 后亮、暗下与阅读器截图一致；字节数不超过阅读器 HTML 的 50%；看两张、登记后 `deliveryStatus: complete`<br>容易犯的错：词段与原文差一个字；两条标注的词段有一部分重合；`occurrence` 写成 3 而该词只出现两次。分别得到 `annotation/span-missing`、`annotation/span-overlap`、`annotation/span-missing`，路径指到那条标注，且没有产出 HTML | 1. `"$E" guide "逐词点评一句文案的用词" --json`，确认返回 `annotation` 及其 schema、示例路径<br>2. `"$E" finalize annotation $OUT/wording.json $OUT/wording.html --json`，看图并登记<br>3. 用 Chrome 打开 `$OUT/wording.html` 做上述操作<br>4. `"$E" static annotation …` 两种策略；内联进 `$OUT/host.html` 查看；`wc -c`；看图登记<br>5. 三份错误输入各运行第 2 步到新路径 | M2–M3 交付齐备、开发引擎构建后（与 V-2 同一次构建） |
| V-4 | 场景 E2E | **文档作者把一份讲改动的材料讲成一页，把一段用词点评讲成一页，另一份不涉及两者的材料照常讲。** 角色：只读 `skills/elintp` 与 `skills/phoenix-diagram` 的 skill 文件及其引用文档，不读实现源码。输入：`docs/plan/ELINTP-VISUALS-P4.md`（§1.2 列出了新增与改动的文件，是结构的改动）；一份作者写的短材料（一句 skill 描述与对其中三处用词的点评）；`skills/phoenix-diagram/references/extending-types.md`（没有结构改动，也没有逐词点评）。覆盖 WORK-F5/AC-6、C-2。结果：<br>• 第一份的页面里有一张变更视图（tree 或其他合适的图型），标记与原文一致；默认是页内嵌入阅读器，读者能在页面里切换三种视图<br>• 第二份的页面里有一张逐词标注，原文逐字、点评与材料一致<br>• 两份各加上「零网络阅读面、图用内联 SVG」的要求再生成：图是静态图，变更视图那张带徽标、数量与「变更视图」字样；终稿是单个 HTML，断网打开没有网络请求；360 px 宽下页面不横向滚动，图在自己的区域内横向滚动；打印预览与禁用脚本时图完整可见<br>• 第三份的页面没有变更视图，也没有逐词标注；交付说明不声称画过<br>• 交付说明写明图的形态与实际做过的检查 | 1. 按 `skills/elintp/SKILL.md` 执行，phoenix-diagram 的命令用 `$E`<br>2. 用 Chrome 打开各终稿：DevTools Network 设为 Offline 后刷新；把窗口设为 360 px 宽；打开打印预览；在 DevTools 禁用 JavaScript 后刷新<br>3. `grep -c '<iframe' 终稿`、`grep -c 'data-change-ui' 终稿`、`grep -c 'data-annotation-ui' 终稿` | M4 交付齐备后（依赖 V-2、V-3 已通过的开发引擎） |
| V-5 | 局部 | 准入依据：分支多的纯逻辑（四类图型 × 标记组合的端点规则、erd 继承、计数；词段定位、重叠判定、换行切分），E2E 逐一覆盖不经济。<br>• `tools/phoenix-diagram/tests/change.rs` ★，经公开的校验与编译入口断言回执与场景：§5.2 每个诊断码在四类图型上的触发与路径；erd 实体的新增、删除、变更三类标记都带给全部字段、只标字段时实体无标记；tree 换父的两条连线标记与两个视图的树约束；三类计数；没有标为变更的项时说明带没有说明段；没有任何 `change` 的输入，场景与去掉 `changes` 字段前逐字段相同；四类最小带标记输入的产物检查通过<br>• `tools/phoenix-diagram/tests/annotation.rs` ★：词段首次与第 N 次出现的定位；找不到、越界、部分重叠、包含、相同五种拒绝；纯西文、纯中文、中英混排三种原文换行后各行片段拼回原文逐字相同；词段跨行；编号跨句连续；最小示例的产物检查通过<br>• 随清单更新的既有用例：`skill.test.js`、`build.test.js`、`reader-assets.test.js`、`tests/cli.rs`、`tests/check_anchors.rs`、输入语料；阅读器装配 `--check` 与目录用例 | `$CARGO test $MF --test change --test annotation --test cli --test check_anchors --test input`<br>`mise exec --no-deps -- node --test tools/phoenix-diagram/tests/viewer/catalogs.test.mjs test/phoenix-diagram/skill.test.js test/phoenix-diagram/build.test.js test/phoenix-diagram/reader-assets.test.js`<br>`mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check` | workflow 的变更标记与阅读器部分在 M1 批末；其余在 M2–M3 批末；`skill.test.js` 在 M4 批末再跑一次 |
| V-6 | 阶段收口 | 全量回归、兼容与跨期终验（WORK-N3/AC-5、WORK-F9/AC-6、C-1、C-4、C-5、C-8）：<br>• 十二类最小示例的图面指纹、两种策略的静态 SVG SHA-256 与基线逐一相同<br>• 十三类最小示例在两种策略下都能取得静态图，自动阶段全过，体积比均不超过 50%<br>• 现行文档中不再有「十二类」的复述，仪表盘与其他 infographic 仍被排除；skill 描述与引用测试通过；bundle 与源一致<br>• 终验证据表（§9.2）逐行填齐 | 1. `mise exec --no-deps -- npm run test:diagram`（fmt、clippy、全部 Rust / Node / Reader 测试；耗时未知；必须实际运行这条聚合命令）<br>2. `mise exec --no-deps -- node --test test/elintp/render.test.js test/elintp/review.test.js`<br>3. 最终代码上重新开发构建（新输出目录），对十三类各运行 `deliver` 与两策略的 `static`，按 §9.1 的命令比对指纹、SHA-256 与 `wc -c`<br>4. `git grep -n -e '十二类' -e '十二种' -e '十三类' -e '逐词标注' -e '变更视图' -- skills README.md docs/phoenix-diagram.md bin ':!dist'`，逐条确认是现行、正确的说法；对照 §6.4 读两个 skill 的 diff<br>5. `node scripts/build-phoenix-diagram.mjs --sync-bundle` 后 `git status --short dist/phoenix-diagram/bundle` 无未提交差异<br>6. 按 §9.2 填终验证据表 | M5，代码改完后 |
| V-7 | 真实使用验收 | 从干净的用户状态连续完成三件事，亲自看产物（WORK-F7、WORK-F8、WORK-F5/AC-6、WORK-F9/AC-6）：<br>1. 新项目按文档离线安装后，图解作者只凭安装后的 skill 文件，把自己手头的一处改动（例如本仓某个目录在最近几次提交里的增删，取自 `git log --stat`）画成 tree 变更视图，交付阅读器 HTML（看两张、登记），切到变更前导出一张 PNG，再取静态图（看两张、登记）<br>2. 同一位作者把一句自己写的话连同三处点评画成逐词标注并交付<br>3. 文档作者在同一项目里用 elintp 把一份讲这处改动的短材料讲成一页，明确要求零网络阅读面，页面带第 1 步的静态图；读者断网打开终稿，切换亮暗，在窄屏下阅读<br>发现的用户可见问题修复后复验，或列给用户确认延期 | 1. 新建临时目录作项目，按 `docs/phoenix-diagram.md`「离线安装」：`node bin/xgent-skills.js install "$PROJECT" --no-phoenix-ui --providers=claude --diagram-release-dir="<V-6 第 3 步的开发构建目录>"`<br>2. `PD="$PROJECT/.claude/skills/phoenix-diagram/scripts/phoenix-diagram"`，只凭安装后的 `SKILL.md` 与 `references/` 操作<br>3. elintp 用仓内 `skills/elintp`；角色只读两个 skill 的文件与公开文档<br>4. 浏览器操作同 V-4 第 2 步 | M5，V-6 通过后 |

V-4、V-7 是 Agent 角色模拟，不代表真人可用性结论。

### 9.2 跨期需求的终验证据（V-6 第 6 步）

P5 是 WORK-F5、WORK-F9、WORK-N3 的终验期。收口时在 M5 记录里列一张表，每个 AC 一行，写证据来源与它仍然有效的依据：

| 需求 | 本期新证据 | 引用前期记录的 AC | 前期证据仍有效的依据 |
| --- | --- | --- | --- |
| WORK-F5 | AC-6：V-4 | AC-1、AC-4、AC-5：[P1 计划](ELINTP-VISUALS-P1.md)完成记录；AC-2：[P3 计划](ELINTP-VISUALS-P3.md)完成记录；AC-3：[P4 计划](ELINTP-VISUALS-P4.md) M4 记录 | `skills/elintp/scripts`、`skills/elintp/assets` 本期无改动（`git diff --stat <基线>.. -- skills/elintp`）；V-4 的两份页面再次覆盖默认嵌入与静态两种形态 |
| WORK-F9 | AC-6：V-2、V-3 | AC-1 至 AC-4：P3 计划及其补缺计划的完成记录；AC-5：P4 计划 M2、M3 记录 | V-6 第 3 步：十二类两种策略的静态 SVG 与基线逐字节相同 |
| WORK-N3 | AC-5：V-6 | AC-1、AC-2：P1 计划完成记录；AC-3：P3 计划完成记录；AC-4：P4 计划 M5 记录 | elintp 的 frontmatter 与脚本无改动；V-6 第 1、3 步 |

引用的前期记录缺少某个 AC 的证据，或依据不成立时，在本期补做对应检查并写进 M5 记录，不把缺口记为通过。

## 10. 里程碑与验收安排

M1 一批：它打通变更标记的整条链路（输入、校验、场景标注、说明带、图面、内容检查、阅读器切换、导出、浏览器检查、看图），是 M2 动工前要先验证的前置——说明带加高画布、徽标落在节点内角、页头里的视图按钮这三点与通用检查、探针和窄屏的相容性，只有跑过真实交付才知道。M2–M3 同批：都在同一个开发引擎上验证，之间没有风险门，连续实施，批末构建一次。M4 一批。M5 收口。

M3 与 M2 可并行编写：M3 的新文件（`layout/annotation.rs`、`render/annotation.rs`、`check/annotation.rs`、schema、示例、`tests/annotation.rs`）与 M2 没有重叠；共享文件 `render.rs`、`check.rs`、`scene.rs`、`render/static_style.rs`、五份目录与 `skill.test.js` 由主 agent 一人修改并集成。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 变更标记在 workflow 打通 | 基线已记录（§9.1） | **落点**：§5.1 `common.schema.json` 两个定义与 `workflow.schema.json`；§5.2 `changes.rs` ★ 的 `resolve`（四类规则一次写完）；§5.3 `scene.rs`、`changes::attach`、`layout/mod.rs`、`layout/tree.rs` 两个函数改 `pub(crate)`；§5.4 `render/changes.rs` ★ 与通用渲染三处调用（`write_nodes`、`write_relation_paths`、`write_relation_labels`）、`check/change.rs` ★；§6.1 阅读器控件、`change-view.js` ★、`focus.js` 一处、`viewer.css`、五语言文案、装配脚本与 README<br>**同构**：`render.rs` `delta_node_marker`、`write_delta_markers`；`tools/phoenix-diagram/viewer/tree-branches.js`；`input/semantics.rs` `chart_checks` 的接入方式<br>**文档**：`skills/phoenix-diagram/references/change-view.md` ★ 与 `SKILL.md` 的变更视图一句（作者要凭它写 candidate）；`--sync-bundle` | V-1、V-5 的 workflow 与阅读器部分；回写「实施进度」 |
| M2 | lifecycle、tree、erd 的变更标记与变更视图的静态图 | M1 的 `changes` 模块、说明带与视图切换（V-1 已通过） | **落点**：§5.1 其余三份 schema；§5.3 第 3 步的派生图元（erd 干线、lifecycle 合流点）与 tree 的换父连线（`layout/tree.rs`、`check/tree.rs`）；§5.4 在 `render/tree.rs`（节点、连线、换父连线的徽标、分支开关）、`render/erd.rs`（实体、字段行、关系、标签、干线）与通用渲染的合流点处接入；§5.5 `static_style.rs` 两处<br>**同构**：M1 的 workflow 接入；`render/erd.rs` L399–L402 的斑马纹<br>连续实施，中途不跑批末检查 | 与 M3 同批：V-2、V-5；回写「实施进度」 |
| M3 | `annotation` 图型 | 无（与 M2 并行；共享文件由主 agent 集成） | **落点**：§5.6 schema ★ 与最小示例 ★；§5.7 语义校验；§5.8 `layout/annotation.rs` ★；§5.9 `render/annotation.rs` ★、`check/annotation.rs` ★、五语言文案、`static_style.rs` 的 `white-space`；§5.10 全部登记点；`skills/phoenix-diagram/references/annotation.md` ★ 与 `SKILL.md` 的图型表一行、创作路径一句、描述（`skill.test.js` 要求引用与词表）；`--sync-bundle`<br>**同构**：`layout/chart.rs` `compile`、`render/chart.rs`、`check/chart.rs`；登记点照 `1d9e3f57`<br>**注意**：先建 schema 与示例文件，再加 `include_str!` | 批末（M2–M3 交付齐备、开发引擎构建后）：V-2、V-3、V-5 的其余部分；回写「实施进度」 |
| M4 | 接口文字同步与 elintp 选图 | M1–M3 的行为已定（V-2、V-3 已通过） | **落点**：§6.2 其余文字（`skills/phoenix-diagram/SKILL.md` 第 4 步一句、`skills/phoenix-diagram/schemas/README.md`、五份 references、`docs/phoenix-diagram.md`、`README.md`、`bin/xgent-skills.js`）；§6.3 `skills/elintp/SKILL.md`；`--sync-bundle`<br>**同构**：P4 提交 `1310cb63`、`5f40e694` 中的同类文字改动 | V-4；V-5 的 `skill.test.js`；回写「实施进度」 |
| M5 | 阶段收口与跨期终验 | M1–M4 完成 | 全量回归、兼容比对、接口复述核对、bundle 同步、终验证据表、真实使用验收；发现的问题修复后只复验失败项与波及项 | V-6、V-7；回写「实施进度」 |

提交点：M1 通过后提交一次（引擎、阅读器、schema、`change-view.md`、bundle）；M2 与 M3 各自交付后分别提交，批末检查通过后一并记完成；M4 通过后提交一次（文字与 elintp）；M5 完成后最终提交。M5 通过后 WORK-F5、WORK-F9、WORK-N3 的本方实施与终验证据齐备；需求状态是否转「已上线」随发布流程决定，不在本计划内。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 说明带加高画布、文字放在图例之下，与通用检查、浏览器探针和阅读器图例迁移的相容性只经代码阅读确认 | 不相容时带标记的图过不了自动阶段 | M1 的 V-1 是第一道真实验证。不相容时先调整带的位置与间距；仍不行则把说明带改放画布顶部（`view_box.y` 上移），按偏差修订 §5.3，不为它豁免通用检查 | M1 批末 | 否 |
| 节点徽标放在右上角内侧，可能压到长标签或品牌标识；关系徽标可能压到相邻标签 | 看图审核不通过 | V-1、V-2 看图时逐个核对；冲突时节点徽标改为骑在右上角的边框上，关系徽标沿第一段路线后移，按偏差修订 §5.4 | M2 批末 | 否 |
| 视图按钮放在页头里，展示模式与窄屏下的表现未实测 | 窄屏可能换行过多；展示模式下页头隐藏时不能切换 | V-1 第 3 步在 360 px 宽下查看；展示模式下不能切换属于可接受的限制（图上有视图名），发现后写进 `viewer-runtime.md` | M1 批末 | 否 |
| `white-space: pre` 用在 SVG 文字上，在 Chrome 与 elintp 静态校验里的实际效果未实测 | 连续空格可能被合并显示，或静态图被拒 | V-3、V-4 实测。若 Chrome 不生效：去掉该规则与 §5.5 的清单项，连续空格合并显示，在 `annotation.md` 写明；若 elintp 拒绝：同样去掉。两种情况原文在 DOM 中仍逐字保留，内容检查不受影响 | M3 批末 | 否 |
| 原文换行按估计宽度切分，CJK 的估计偏宽 | 中文行会比栏宽短一些，不会溢出 | V-3 看图确认可接受；溢出由浏览器检查拦下 | M3 批末 | 否 |
| D6、D13、D19 比 PRD 的字面多拒绝几种输入：新增端点的关系须同为新增；包含与相同的词段也算重叠；tree 的根不能在变更前后更换 | 作者会在这些输入上得到失败 | 理由写在 D6、D13、D19、ADR-2；用户不同意时改为放行需要另定这些情形的图面（换根还需另定两个根的联合布局），在 M1（D6、D19）或 M3（D13）动工前提出 | M1、M3 动工前 | 否 |
| tree 的旧父连线绕外侧通道，在大树里线长、与普通连线交叉多 | 变更视图与变更前视图不易读，看图审核可能不通过 | V-2 看图时核对；不通过时先调整通道位置（换到另一侧、贴近两端所在的子树），按偏差修订 §5.3；不改用复制节点 | M2 批末 | 否 |
| 变更前、变更后在被隐藏元素处留空；这两个视图里树的折叠计数、大纲与搜索列表按全部元素计 | 读者可能以为排版有缺口，或计数与所见不符 | ADR-1 的既定后果，写进 `change-view.md` 与 `viewer-runtime.md`；用户要求紧凑排版或逐视图计数时另立需求 | — | 否 |
| 开发构建目录作离线安装的 release-dir | 影响 V-7 第 1 步 | P4 收口已按同一方式安装通过：包侧不入库的 `dist/phoenix-diagram/VERSION.json` 与 `dist/phoenix-diagram/bundle/ENGINE.json` 要取自同一次构建（[P4 M5 记录](ELINTP-VISUALS-P4.records/M5.md)）；安装器拒绝时按其报错处理并记录，不用 `$E` 冒充安装验收 | M5 | 否 |

- 最终状态：**Ready**。
- 定级理由：范围、输入契约（含 tree 的换父与 erd 三类标记的字段继承）、诊断、有效标记规则、场景字段、图面与样式规则、阅读器行为、新图型的版式与登记点、检查与里程碑已定；前置期 P3、P4 已完成；没有未决的范围、数据、外部契约或明确要求的 NFR 问题。表中各项都有实测时点与每种结果对应的路线。

## 12. 已知坑与历史教训

- `tools/phoenix-diagram/assets/template.html` 与 `tools/phoenix-diagram/assets/catalogs.json` 是生成物，直接改会被 `--check` 判为过期；改 `viewer/` 下的源或目录后运行装配脚本（`tools/phoenix-diagram/viewer/README.md`）。
- `build.rs` 的 `collect` 遇到缺失输入会 panic，`include_str!` 指向不存在的文件时整个 crate 编译失败；先建 schema 与示例文件，再加 `include_str!`（[PHOENIX-DIAGRAM-STATECHART 计划](PHOENIX-DIAGRAM-STATECHART.md) §12）。
- `finalize` 与 `static` 的 stdout 很大，宿主直接读取会被截断；读 summary 文件，不读 stdout。
- SVG 内 `<style>` 的规则会被静态转换重新解析：不引用 SVG 根、用了支持清单外的声明或 `:has()` 时，`static` 报 `static/style` 而阅读器正常（`tools/phoenix-diagram/src/render/static_style.rs` L266–L274）。M1 写样式时就按 §5.4 的约束写，不等到 M2 才发现。
- `viewer.css` 里引用 `svg` 或以 `.c-`、`.t-`、`.a-`、`.m-` 开头的规则会进入每一类的静态图，改变十二类的静态 SVG 字节；视图按钮的样式只用 `.change-view-` 前缀，变更标记的样式写在 SVG 内的 `<style>`，不写进 `viewer.css`。
- elintp 的静态图白名单没有 `text-decoration` 与 `xml:space`（`skills/elintp/scripts/draft.mjs` L8–L12）：删除项不用删除线，词段不用下划线。
- 浏览器探针要求带 `data-source-path` 的文字在默认状态下可见（`tools/phoenix-diagram/src/browser/probe.js` L110、L180–L182）：说明带里默认隐藏的两组文字不带 `data-source-path`。
- 探针固定核对 7 个控件（`tools/phoenix-diagram/src/browser/observation.rs` L176–L177）：不要把视图按钮加进探针的控件清单。
- `skill.test.js` 固定了描述长度、前 100 字的词与 `version: "0.2.0"`，并要求每个最小示例至少两个节点级对象：`minimal.annotation.json` 写两句；改描述后先跑这个文件。
- 节点事实里的 `label`、`sublabel`、`tag` 会被通用检查拿去和节点文字比对（`check.rs` L973–L991）：`annotation` 的节点事实是原始标注，没有这些字段，词段与点评由 `check/annotation.rs` 核对。
- `--sync-dist` 会改写 `dist/` 根部的三方许可文件，构建后 `git checkout --` 还原（[P4 补缺计划](ELINTP-VISUALS-P4-gaps.md) §12）。
- P4 收口曾把子集测试结果记成 `npm run test:diagram` 通过、把退出码检查记成真实使用验收，后来返工（[P4 M5 记录](ELINTP-VISUALS-P4.records/M5.md)「记录修正」）：V-6 第 1 步必须实际运行聚合命令，V-7 必须按步骤安装并亲自看产物。
