# ELINTP-VISUALS-P4 · XGENT-ai Skills · 数值图表（Phoenix Diagram 新增 chart 图型 · 柱、折线、饼、限额四种形态 · 数据表随图交付）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-14 · `3889a9ae16396d710f637583721b12a73d3eb897` · dirty（未提交改动是 use-llm-gateway skill、状态台账与 P3 评审文档，不在本计划改动面；`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 相对 HEAD 无改动，代码事实按当前工作树核对）。
>
> 需求来源：[elintp 模板化图文说明与 Phoenix Diagram 图解补充 PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P4（第 4 期）。本期承接 WORK-F6/AC-1 至 AC-6、WORK-F5/AC-3、WORK-F9/AC-5、WORK-N3/AC-4；WORK-F5、WORK-F9、WORK-N3 的终验期是 P5，本期完成后这三条停在已提交 / 可测试。
>
> 本期交付：**phoenix-diagram 新增 `chart` 图型，用同一套交付流程画柱状图（竖向或横向）、折线图、饼图和限额对比图；每张图在阅读器里附可展开的数据表，静态图把数据表作为数据随 SVG 交付；elintp 在材料含需要比较的真实数值时选用数值图表，并在页面里显示数据表。**
> **开发范围：`tools/phoenix-diagram`（引擎与阅读器维护源）、`skills/phoenix-diagram`（随工具附带的 skill 及 `dist/phoenix-diagram/bundle` 副本）、`skills/elintp`（选图规则与静态图数据表的显示）、复述图型数量与排除范围的现行文档；依据见 §0。**
> 本期独特职责：既有十一类画的是实体与关系；`chart` 画的是数值，图形的长度、位置与角度由数值决定，每个数值都要在图面和数据表里原样可读。
> **顶层排除：变更视图、逐词标注及其静态图（P5）；仪表盘、多图联动、实时数据与其他 infographic；summarize skill 不改；不向 SPMS 提交报告；不发布 npm 包或引擎。**

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

- 最近更新：2026-11-14 16:29 +0800
- 当前进度：4/5 个里程碑完成
- 当前状态：M5 实施中
- 最近完成：M4 · elintp 选用数值图表并显示数据表（V-3 浏览器五查 13/13 PASS 与 V-4 elintp 部分通过）
- 下一步：M5 · 阶段收口（V-5 全量回归与兼容 + V-6 真实使用验收）
- 当前阻塞：无
- 代码基线：`3889a9ae`（工作区 dirty，未提交改动不在本计划改动面）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-14 14:27 +0800 | chart 图型与两种柱状图打通：基线十一类逐一相同，V-1 三份交付 complete，V-4 全绿；guide 场景词按 @implementation_route 修订 | [M1 记录](ELINTP-VISUALS-P4.records/M1.md) | 1d9e3f57 |
| M2 | 已完成 | 2026-11-14 15:21 +0800 | 折线、饼、限额三形态：批末 V-2 全部结果项与 V-4 其余引擎部分通过（三形态 finalize complete、四形态双策略静态图 complete、体积比 ≤50%、data-phoenix-table 逐格一致、错误输入 ×4 诊断正确） | [M2 记录](ELINTP-VISUALS-P4.records/M2.md) | 1310cb63 |
| M3 | 已完成 | 2026-11-14 15:21 +0800 | 静态图 data-phoenix-table 数据（write_common）与 §5.10 其余接口文字同步（九处），bundle 已同步；批末共用检查引用 M2 记录 | [M3 记录](ELINTP-VISUALS-P4.records/M3.md) | 1310cb63 |
| M4 | 已完成 | 2026-11-14 16:29 +0800 | elintp 选用数值图表并显示数据表：V-3 浏览器五查 13/13 PASS（Offline/360px/键盘/打印/禁 JS + 宽表区域滚动 + 损坏退 2 不覆盖）与 V-4 elintp 部分全过；elintp frontmatter 无变化 | [M4 记录](ELINTP-VISUALS-P4.records/M4.md) | 待提交 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID，验收原文见 PRD §3，不在此重抄；「内容」列只写本期承接的部分。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F6/AC-1 | 功能 | PRD §3 WORK-F6 · P4 | 四种形态的图里，每个标签、系列、数值与单位与输入一致，没有未说明的丢失、合并或四舍五入 | §5.3、§5.4；V-1、V-2 | 已确认 |
| WORK-F6/AC-2 | 功能 | 同上 | 柱状图数值轴含 0，负值在零线另一侧；多系列带图例，系列之间有颜色之外的区分 | §5.3（系列键）、§5.4；V-1 | 已确认 |
| WORK-F6/AC-3 | 功能 | 同上 | 饼图含负值或全为 0、数值不是数字、行内数值个数与系列数不符时，失败信息指向具体行，不产出通过的图 | §5.2；V-1、V-2、V-4 | 已确认 |
| WORK-F6/AC-4 | 功能 | 同上 | 每张图附带含全部数值的数据表，可展开阅读与复制；限额对比图标出超限项 | §5.4（limits）、§5.7、§5.8、§6；V-1、V-2、V-3 | 已确认 |
| WORK-F6/AC-5 | 功能 | 同上 | 没有真实数值时 Agent 不画数值图表；作者明确要求示意图时，图上标明数值为示意 | §5.1（`meta.evidence`）、§5.3（徽标）、§5.10、§6.3；V-1、V-3 | 已确认 |
| WORK-F6/AC-6 | 功能 | 同上 | 与既有图型相同的交付入口、阅读器主题与导出；未知形态被显式拒绝 | §5.1、§5.9；V-1 | 已确认 |
| WORK-F5/AC-3 | 功能 | PRD §3 WORK-F5 · 跨期，终验 P5 | 材料含需要比较的真实数值且 phoenix-diagram 可用时 elintp 选用数值图表；没有真实数值时不画 | §6.3；V-3 | 已确认 |
| WORK-F9/AC-5 | 功能 | PRD §3 WORK-F9 · 跨期，终验 P5 | 数值图表可取得静态图，完整性、亮暗与体积要求同十一类 | §5.8；V-2、V-5 | 已确认 |
| WORK-N3/AC-4 | 兼容 | PRD §3 WORK-N3 · 跨期，终验 P5 | 十一类有效输入结果不变；skill 描述、README、文档与 elintp 出图对照同步说明数值图表，仪表盘及其他 infographic 仍排除 | §5.9（判定口径）、§5.10、§6.3；V-5 | 已确认 |
| C-1 | 约束 | PRD §7「已交付的部分持续遵守对应的质量门槛」 | elintp 页面里的数据表同样满足 WORK-N1（窄屏在自身区域横向滚动、键盘可达、打印与禁用脚本时可读）与 WORK-N2/AC-1、AC-2（单文件、零网络、表格文字只当内容显示） | §6.1、§6.2；V-3、V-4 | 已确认 |
| C-2 | 约束 | [AGENTS.md](../../AGENTS.md) `@skill_tool_independence` | elintp 不以 phoenix-diagram 的安装或引擎版本为执行前提；引擎没有 `chart` 时走既有降级 | §6.3 | 已确认 |
| C-3 | 约束 | AGENTS.md §7；PRD §5.3 的 C6、C10 | 修改两个 skill 前后按整合清单核对：elintp 触发与授权不变；phoenix-diagram 只新增数值图表并改写描述边界，不引入固定数量上限 | §5.10、§6.4；V-5 第 4 步 | 已确认 |
| C-4 | 约束 | AGENTS.md「Working in This Repository」接口复述条款 | 「十一类」「不用于数值图表」的全部现行复述在同一改动内更新；历史计划、记录与 CHANGELOG 既有条目不改 | §1.2、§5.10；V-5 第 4 步 | 已确认 |
| C-5 | 约束 | AGENTS.md §6；`test/phoenix-diagram/skill.test.js` L15–L27 | phoenix-diagram 描述不超过 200 字，前 100 字含现有图型词与「数值图表」「仪表盘」「infographic」，`version: "0.2.0"` 不变；elintp 描述不改 | §5.10 | 已确认 |
| C-6 | 约束 | AGENTS.md「Working in This Repository」；`scripts/test-phoenix-diagram-native.mjs` | 引擎检查用 mise 固定的 Rust 与 `mbx`，离线、锁定依赖；工具缺失即失败，不跳过 | §9 环境 | 已确认 |
| C-7 | 约束 | [docs/phoenix-ui.md](../phoenix-ui.md) L253；`tools/phoenix-diagram/viewer/README.md` | 改过 `skills/phoenix-diagram` 后运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle` 并提交 bundle；改过阅读器维护源后运行装配脚本，`tools/phoenix-diagram/assets/template.html`、`tools/phoenix-diagram/assets/catalogs.json` 是生成物不手改 | M1、M3；V-4、V-5 | 已确认 |
| C-8 | 约束 | AGENTS.md `@simplicity`、`@failure_handling` | 工具保持直接实现：只在输入边界校验，不加事务、备份、兜底或兼容层 | §5.2、§8 | 已确认 |
| C-9 | 约束 | `skills/phoenix-diagram/references/extending-types.md` | 新图型同时交付输入与语义、发现与创作、布局渲染与检查、Reader 与导出、交付与分发、回归六条路径；不加内容配额，不靠裁切、缩字、隐藏过检 | §5、§9 | 已确认 |
| C-10 | 约束 | PRD §5.1 D10、WORK-F6 边界 | 不设固定的行数、系列数或点数上限；读不清时由校验失败并指出对象 | §5.4、§5.6 | 已确认 |
| C-11 | 约束 | PRD §4「参考基线」 | 复用参考工具代码须保留 MIT 许可与来源；本期只借鉴行为、不复制代码，因此不新增许可条目 | D12 | 已确认 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 图型身份 | 一个新图型 `chart`，`form` 取 `bar`、`line`、`pie`、`limits`；柱状图另有 `orientation` | 图型由十一类变为十二类；命令、回执、静态图、预览都按 `<type>` 通用分派，不新增命令或参数 | ADR-1 |
| D2 | 输入结构 | 顶层 `series`（可省）与 `rows`；柱、折线、饼的行写 `values` 数组，限额行写 `value`（可省）与 `limit` | 一份 schema；形态与字段的搭配由语义校验给出指向行的错误 | §5.1、§5.2 |
| D3 | 数值来源声明 | `meta.evidence` 必填，取 `sourced` 或 `illustrative`；后者在图上画「示意数值」徽标 | 作者必须表态数值是否来自材料；与 waterfall、timeline 的 `meta.evidence` 同构 | WORK-F6/AC-5；`skills/phoenix-diagram/schemas/waterfall.schema.json` |
| D4 | 什么是节点 | 一行数据一个节点；节点框是该行的行带，柱、点、扇区画在行的 `<g>` 内 | 满足通用检查「每节点恰一个节点框、节点框间距 ≥ 8」；搜索、详情、聚焦按行工作 | ADR-2 |
| D5 | 数值标注 | 每个数值都在图面上标出，不按数量阈值省略；放不下时画布变宽，过宽由既有可读性检查失败 | 稠密序列会校验失败并指出对象，作者改为拆图或减少点数 | WORK-F6/AC-1；C-9、C-10 |
| D6 | 系列的非颜色区分 | 两个及以上系列时，每个系列有系列键 `A`、`B`…：图例写「A · 系列名」，该系列每个数值标注以键开头（`A 12`） | 不依赖颜色即可把标注、图形与系列对应；数量不受色板或线型种类限制 | WORK-F6/AC-2；PRD §2.4 |
| D7 | 数值与单位的写法 | 数值用 `js_to_string` 原样写出（不加千分位、不舍入）；单位写在刻度、限额与饼图的数值文字和数据表表头 | 派生量（刻度、饼图占比）可取整，占比列明确标为「占比」 | WORK-F6/AC-1；`tools/phoenix-diagram/src/format/number.rs` |
| D8 | 数值轴 | 柱状图轴必含 0；折线轴覆盖数据并向外取整到刻度，不强制含 0；刻度步长复用 waterfall 的 `tick_step` | 不新增刻度算法；折线不因基线被压平 | WORK-F6/AC-2；`tools/phoenix-diagram/src/layout/waterfall.rs` L162 |
| D9 | 阅读器里的数据表 | `.cards` 容器末尾的 `<details class="chart-data">`，默认收起 | 不新增模板插槽；随卡片进入侧栏、展示模式隐藏等既有布局 | §5.7；`tools/phoenix-diagram/src/render.rs` L1440–L1455 |
| D10 | 静态图里的数据表 | 表格数据写进静态 SVG 根下的 `<metadata data-phoenix-table="">`，由宿主渲染；elintp 渲染成图下的 `<details>` 表格 | 与卡片数据同一做法；图面、截图、体积不变 | ADR-3 |
| D11 | 来源、品牌与比较 | `chart` 不接受 `sources`、`meta.repository`、品牌与架构比较 | 与 statechart 相同；数值来源由 D3 表态 | PRD WORK-F6 边界；`tools/phoenix-diagram/src/evidence.rs` L133–L144 |
| D12 | 参考工具 | 只借鉴形态与错误语义；不复制其代码、刻度算法或样式 | NOTICE、许可清单与构建脚本的许可条目不变 | C-11 |
| D13 | 工具版本号 | 保持 0.2.0 | statechart 新增图型时未改版本；npm 版本在发布时定 | `tools/phoenix-diagram/Cargo.toml`；`skill.test.js` L25 |
| D14 | 「十一类结果不变」的判定 | 同一 candidate 的阅读器主图 SVG、内嵌元数据与静态 SVG 字节不变；阅读器 HTML 整文件因模板新增目录文案与数据表样式而变化 | 与 statechart 新增图型时一致：模板内嵌五语言目录，新增文案必然改变模板 | WORK-N3/AC-4；`tools/phoenix-diagram/viewer/README.md` |
| D15 | elintp 何时画数值图表 | 材料给出需要比较的真实数值时用 `chart`；没有真实数值不画；作者明确要求示意图时用 `illustrative` | 只改选图规则与对照表，不改触发、授权与默认嵌图方式 | WORK-F5/AC-3、WORK-F6/AC-5 |

### 0.3 ADR-lite

#### ADR-1：一个 `chart` 图型承载四种形态

- 状态：Accepted
- 背景与驱动：WORK-F6 称四者为「形态」，AC-6 要求相同交付入口并显式拒绝未知形态。每个图型在引擎里有约二十处登记点（见 §5.9）。
- 备选：
  - A. 一个图型 `chart`，`form` 区分形态。
  - B. 四个图型 `bar`、`line`、`pie`、`limits`。登记点、schema、示例、文案各四份；`guide` 要在四者间再选一次。
  - C. 两个图型 `chart` 与 `limits`（参考工具的切法）。限额与其余三者共用行、标注、数据表和示意徽标，拆开只多一套登记。
- 决策：A。未知形态由 schema 的枚举拒绝（`input/schema`，路径 `/form`）。
- 正面后果：登记一次；四种形态共用行节点、数值写法、系列键、数据表与静态数据。
- 负面/中性后果：schema 不能单独表达「形态与字段的搭配」，由语义校验承担（§5.2）；文档里的图型数是十二，形态数另述。
- 重新评估触发：某形态需要独立的输入契约版本或独立的阅读器交互。

#### ADR-2：一行数据是一个节点，行带是节点框

- 状态：Accepted
- 背景与驱动：通用产物检查要求每个 `g[data-node-id]` 恰有一个 `rect[data-graph-role="node-body"]`，任意两个节点框间距不小于 8（`tools/phoenix-diagram/src/check.rs` L537–L562、L636–L672）；浏览器探针要求节点标签落在节点框内（`tools/phoenix-diagram/src/browser/probe.js` L217）。
- 备选：
  - A. 行是节点，节点框是覆盖该行标签、图形与数值标注的行带。
  - B. 每个数值（行 × 系列）是节点，柱或点是节点框。同组柱间距要 ≥ 8；折线上数值接近的两个系列的点必然被判为节点重叠。
  - C. 为 chart 豁免通用检查。等于另写一套检查。
- 决策：A。行带之间留 10 的间距；柱、点、扇区是行内图形，带 `data-chart-series`，由 chart 自己的内容检查核对位置（§5.6）。
- 正面后果：通用检查、探针、搜索、详情、聚焦与节点大纲不改；节点数等于行数。
- 负面/中性后果：阅读器的最小选择单位是行，不能单独选中某个系列的一根柱；行的详情里列出该行全部数值。
- 重新评估触发：出现按系列筛选或单点下钻的需求。

#### ADR-3：数据表在阅读器里是 HTML，在静态图里是数据

- 状态：Accepted
- 背景与驱动：WORK-F6/AC-4 要求每张图附带可展开、可复制的数据表。阅读器主图是 SVG，表格适合用 HTML；静态图只有一个 SVG，是为不能嵌阅读器的阅读面准备的。卡片已按「引擎交数据、宿主渲染」交付（[P3 补缺计划](ELINTP-VISUALS-P3-gaps.md) ADR-1）。
- 备选：
  - A. 阅读器：`.cards` 内的 `<details>` 表格。静态图：`<metadata data-phoenix-table="">` JSON，宿主渲染。
  - B. 把表格画进 SVG。图面变高，默认展开，不能收起；截图与体积随之变化。
  - C. 静态图不带数据表。elintp 的零网络页面里 AC-4 不成立。
- 决策：A。表格内容在布局阶段算一次（`DiagramFacts::Chart.table`），阅读器、静态数据与内容检查读同一份。
- 正面后果：图面与看图审核不受表格影响；宿主用自己的表格样式，亮暗与窄屏跟随宿主。
- 负面/中性后果：其他宿主要自己读 `<metadata>` 才有数据表；旧 elintp 不认识第二个 `<metadata>`（§8）。
- 重新评估触发：出现只能渲染纯 SVG、又要求数据表随图显示的宿主。

### 0.4 职责与事实所有权

|  | phoenix-diagram 引擎与 skill | elintp | 宿主 Agent（范围外） |
| --- | --- | --- | --- |
| 拥有 | `chart` 的输入契约、校验、布局、图面、数据表内容、静态数据格式、交付与审核 | 何时画数值图表；把静态图的数据表放进页面；页面的窄屏、打印与最终页自检 | 从授权材料取数；判断材料有没有真实数值 |
| 不拥有 | 数值是否真实；宿主怎么排版数据表 | **数值的校验、图面与看图审核**；不改写、不补造表格内容 | 引擎的校验与审核结论 |

一句话：数画得对不对归 phoenix-diagram，数从哪来归作者与宿主 Agent，页面里放得对不对归 elintp；数据不合约时各自报格式错误，不猜。

### 0.5 明确不在本期

- **变更视图、逐词标注及其静态图，elintp 对这两者的选用**——P5（WORK-F7、WORK-F8、WORK-F9/AC-6、WORK-F5/AC-6）。
- **仪表盘、多图联动、实时数据、统计计算与其他 infographic**——PRD NG7，明确不做；描述里继续排除。
- **堆叠柱、面积、散点、双轴、对数轴、误差线、数值型 X 轴**——PRD 只定了四种形态；出现具体需求时另立需求。
- **稠密序列的抽稀标注（只标重点值）**——D5 的后果；出现具体需求时另立需求。
- **`chart` 的仓库来源、品牌标识、架构比较**——D11。
- **summarize skill 的文字与调用方式、向 SPMS 提交报告**——不改（WORK-N3/AC-2）；范围外系统。
- **渲染性能基线与压测**——后续可选；本期布局是行数的线性计算，没有专项工作。
- **npm 发布、引擎四平台构建、CHANGELOG 条目**——发布流程，见 §8。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **已核实·足够**：前置期 P3 及其补缺已完成（[P3 计划](ELINTP-VISUALS-P3.md)恢复快照 5/5，[P3 补缺计划](ELINTP-VISUALS-P3-gaps.md) 2/2）；`static` 命令、两种字体策略、卡片数据与 elintp `static:` 来源在当前 HEAD 可用。
- **已核实·缺口**：引擎只有十一类，`DiagramType::ALL` 长度 11，未知类型报 `input/unsupported-type`（`tools/phoenix-diagram/src/input.rs` L10–L82）；skill 描述与 `SKILL.md` L40 明确排除数值图表 → §5 新增 `chart` → 漏做则 WORK-F6 全部不成立。
- **已核实·足够**：新增图型的登记点可由 statechart 的新增提交 `6bb21b6b` 逐项对照（§5.9）；`static`、`finalize`、`validate`、`deliver`、`preview` 都按 `<type>` 解析 `DiagramType`，没有图型白名单。
- **已核实·足够**：最接近的同构实现是 waterfall：线性刻度上的条形、刻度与网格、行容器携带绘制事实、图例换行、`meta.evidence` 徽标（`tools/phoenix-diagram/src/layout/waterfall.rs` `compile` L185、`tick_step` L162；`tools/phoenix-diagram/src/render/waterfall.rs` `write_row` L72、`body` L161；`tools/phoenix-diagram/src/check/waterfall.rs`）。
- **已核实·足够**：`TypeRenderer` 给图型四个钩子：根属性、`defs`、主体、图例符号（`tools/phoenix-diagram/src/render.rs` L39–L56）；阅读器脚本没有按图型分支（`tools/phoenix-diagram/viewer/*.js` 中无 `data-diagram-type` 判断），tree、erd、class 的根属性不带 `data-motion-choreography`。
- **已核实·足够**：通用产物检查对节点的要求见 ADR-2；节点事实里的 `label` 必须出现在该节点的文字中（`check.rs` L959–L986）；节点数须等于元数据 `graph.nodes` 数（L948–L957）；节点文字投影字号低于 6 px 时 `composition/desktop-readability` 失败（L604–L630）。
- **已核实·足够**：schema 错误带实例路径（`tools/phoenix-diagram/src/input.rs` L165–L177），数值不是数字时路径形如 `/rows/2/values/1`；行与系列的重复 `id` 由 `Checks::unique` 报 `input/duplicate-id`（`tools/phoenix-diagram/src/input/semantics.rs` L60–L82）。
- **已核实·足够**：阅读器卡片是 `{{PHOENIX:CARDS}}` 插槽里的 `<div class="cards">`，位于侧栏 `aside.reader-rail`（`tools/phoenix-diagram/viewer/template.source.html` L547–L552）；卡片检查只数 `.cards > .card`（`check.rs` L906），`.cards` 里的其他子元素不计入。
- **已核实·足够**：静态样式派生对 `viewer.css` 中既不引用 SVG 根、也不以 `.c-`、`.t-`、`.a-`、`.m-` 开头的选择器直接丢弃；图型 `defs` 里的规则必须引用 SVG 根，声明须在支持清单内（`tools/phoenix-diagram/src/render/static_style.rs` `scope_selector` L242–L268、`supported_declaration` L629–L680）。在 `viewer.css` 新增 `.chart-data` 规则不改变其他图型的静态 SVG。
- **已核实·缺口**：静态 SVG 只携带卡片数据（`static_style.rs` `write_common` L1300–L1326）→ §5.8 → 漏做则静态图没有数据表（WORK-F6/AC-4）。
- **已核实·缺口**：elintp 静态模式只接受根下唯一的 `metadata[data-phoenix-cards]`（`skills/elintp/scripts/draft.mjs` L426–L431）→ §6.1 → 漏做则带数据表的静态图被判为格式错误。
- **已核实·足够**：elintp 已有表格样式与三语界面文字 `ui.table`（`skills/elintp/assets/page.css` L81–L84；`skills/elintp/scripts/templates.mjs` L19、L27、L35）；`page.js` 已有 `beforeprint` 钩子（L23）。
- **已核实·足够**：五语言目录是唯一文案来源，装配脚本生成 `tools/phoenix-diagram/assets/catalogs.json` 并把同一对象嵌进模板，`--check` 要求五份键集合与占位符一致（`tools/phoenix-diagram/viewer/README.md`）。
- **已核实·足够**：`tools/phoenix-diagram`、`skills/phoenix-diagram` 自 `13435a65` 之后没有提交，当前工作树相对 HEAD 无改动。

### 1.2 拓扑与文件清单

**引擎（`tools/phoenix-diagram`）**

- `tools/phoenix-diagram/src/layout/chart.rs` ★、`tools/phoenix-diagram/src/render/chart.rs` ★、`tools/phoenix-diagram/src/check/chart.rs` ★：四种形态的布局、渲染与内容检查。
- `tools/phoenix-diagram/src/layout/waterfall.rs`：`tick_step` 改为 `pub(crate)` 并增加最小刻度间距参数。
- `tools/phoenix-diagram/src/` 下的 `input.rs`、`schema.rs`、`input/semantics.rs`、`layout/mod.rs`、`render.rs`、`check.rs`、`scene.rs`、`evidence.rs`、`cli.rs`：登记点（§5.9）。
- `tools/phoenix-diagram/src/render/static_style.rs`：`write_common` 写表格数据（§5.8）。
- `tools/phoenix-diagram/viewer/viewer.css`、`tools/phoenix-diagram/viewer/locales/{en,zh-CN,zh-TW,es,ko}.json`；`tools/phoenix-diagram/assets/template.html`、`tools/phoenix-diagram/assets/catalogs.json` 由装配脚本重新生成。
- `tools/phoenix-diagram/tests/chart.rs` ★；`tools/phoenix-diagram/tests/cli.rs`、`tools/phoenix-diagram/tests/check_anchors.rs`、`tools/phoenix-diagram/tests/support/input_corpus.rs` 的图型清单。
- 不改：其余十一类的布局、渲染、检查与 schema；`tools/phoenix-diagram/viewer/` 下的脚本与 `tools/phoenix-diagram/viewer/template.source.html`，`tools/phoenix-diagram/src/` 下的 `browser/`、`delivery/`、`compare/`、`brands/`，以及 `tools/phoenix-diagram/build.rs`、`tools/phoenix-diagram/Cargo.toml`、`tools/phoenix-diagram/NOTICE.md` 与许可清单。

**phoenix-diagram skill 与复述**

- `skills/phoenix-diagram/schemas/chart.schema.json` ★、`skills/phoenix-diagram/examples/minimal.chart.json` ★、`skills/phoenix-diagram/references/chart.md` ★。
- `skills/phoenix-diagram/SKILL.md`、`skills/phoenix-diagram/schemas/README.md`、`skills/phoenix-diagram/references/extending-types.md`、`skills/phoenix-diagram/references/preview.md`、`skills/phoenix-diagram/references/viewer-runtime.md`、`skills/phoenix-diagram/references/static-figure.md`、`skills/phoenix-diagram/references/delivery-contract.md`。
- `scripts/build-phoenix-diagram.mjs` L106 的 schema 清单；`test/phoenix-diagram/skill.test.js`、`test/phoenix-diagram/build.test.js` L126。
- `dist/phoenix-diagram/bundle/**`：由 `--sync-bundle` 重新生成。
- `docs/phoenix-diagram.md` L3、L115，`README.md` L216，`bin/xgent-skills.js` L301。
- 不改：`docs/PRD-PHOENIX-DIAGRAM.md`、`docs/plan/**` 既有计划与记录、`.xgent-ai/**` 既有记录、`CHANGELOG.md`。

**elintp**

- `skills/elintp/scripts/draft.mjs`、`skills/elintp/assets/page.css`、`skills/elintp/assets/page.js`、`skills/elintp/SKILL.md`、`skills/elintp/references/draft-format.md`。
- `test/elintp/render.test.js`。
- 不改：`skills/summarize/SKILL.md`、`skills/elintp/scripts/review.mjs`、`render.mjs`、`templates.mjs`、elintp 的描述与触发。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `input::semantics::chart_checks` | `semantics::validate` | 输入已过 schema 的文档；输出 `chart/*` 诊断，路径指到行或字段 | 无 | 形态与字段的搭配（§5.2） | 进程内 | `validate` 回执；V-4 |
| `layout::chart::compile` | `layout::compile` | 输入 `DiagramInput`，输出 `CompiledDiagram`；每行一个节点，`facts.table` 含全部数值；同输入同输出 | 无 | 四种形态的几何、刻度、标注避让、系列键（§5.3、§5.4） | 进程内 | `validate --layout-json`、写出的 HTML；V-1、V-2、V-4 |
| `render::chart::RENDERER` | `render::type_renderer` | 四个钩子；只读场景，不再计算数值 | 无 | 行内图形与属性（§5.5） | 进程内 | 写出的 SVG |
| `check::chart::{content, table}` | `check::check_html` | 从产物与内嵌元数据重新核对标签、数值、位置比例与数据表 | 无 | 各形态的比例核对（§5.6） | 进程内 | `validate` / `finalize` 回执的 `semantic_content` |
| 阅读器数据表 | `render::render_with_layers` | 图型为 chart 时在 `.cards` 末尾写一个 `details.chart-data`；其他图型的插槽字节不变 | 无 | — | 进程内 | 写出的 HTML；V-1 |
| 静态表格数据 | `static_style::SvgConversion::write_common` | 图型为 chart 时在卡片数据之后写一个 `metadata[data-phoenix-table]`；其他图型输出不变 | 无 | — | 进程内 | 写出的 SVG；V-2 |
| elintp 静态表格解析 | `inlineSVG`（静态模式） | 根下至多一个表格 `metadata`，只含文本；结构不合约即报错；不进入输出 SVG | 无 | 结构校验与转义（§6.1） | 本地文件（不可信边界） | 生成页面与拒绝诊断；V-3、V-4 |

不新增接缝或适配器：四种形态只有一个调用方（`layout::compile`），在 `layout/chart.rs` 内按 `form` 分支，文件过长时按形态拆成 `layout/chart/` 下的子模块。删除测试：去掉 `check/chart.rs`，数值与图形的对应只能靠看图发现，故保留；表格数据只有一个写出方与两个读取方（阅读器、静态数据），不为它建独立模块。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| elintp → 静态 SVG 表格数据 | 已核实缺口 | 静态模式只接受卡片 `metadata`（`draft.mjs` L426–L431） | 范围内：§5.8 定义数据格式，§6.1 解析与渲染 | 数据不合约是内容稿格式错误（退出 2），给内容稿行号与 SVG 位置，不覆盖旧页 |
| elintp skill → phoenix-diagram skill | 已核实足够 | elintp 自行查找并读取 `phoenix-diagram/SKILL.md`，不自动安装（`skills/elintp/SKILL.md`「出图」） | 范围内：两处文字同步数值图表 | 未安装、引擎没有 `chart` 或流程失败 → 按既有规则降级（自绘 `svg:` 或 `:::metrics`、表格）并在交付说明写明原因 |
| 其他内联静态 SVG 的宿主 | 已核实足够 | `<metadata>` 不渲染，图面不变 | `static-figure.md` 写明表格数据格式 | — |
| 本机 Chrome | 已核实足够 | `browser::discovery::find`；缺失时回执 `unavailable`、退出 5 | 复用 | 与十一类相同 |
| summarize → elintp | 已核实足够 | summarize §3 以文字提出阅读面要求 | 零改动 | — |

登记位置：新诊断码 `chart/form-fields`、`chart/series-count`、`chart/pie-value`（输入阶段）与 `chart/invalid-scale`、`chart/label-room`（布局阶段）；新文案键见 §5.5；新静态数据属性 `data-phoenix-table`。没有新的命令、参数、配置、环境变量或回执字段。

## 5. 核心机制（引擎）

### 5.1 输入契约（`skills/phoenix-diagram/schemas/chart.schema.json` ★）

照 `waterfall.schema.json` 的写法：顶层与 `meta`、行对象均 `additionalProperties: false`。

| 字段 | 约束 | 含义 |
| --- | --- | --- |
| `schema_version` | `const 1` | — |
| `diagram_type` | `const "chart"` | — |
| `meta` | 必填 `title`、`evidence`；可选 `locale`、`translations`、`subtitle`、`output`、`animation`、`visual_preset`、`quality_profile`（引用 common 的定义，与 waterfall 相同）；没有 `repository` | — |
| `meta.evidence` | `enum ["sourced", "illustrative"]` | 数值来自授权材料，或作者明确要求的示意 |
| `meta.unit` | 非空字符串，可省 | 柱、折线、饼的数值单位；限额行没写 `unit` 时的缺省 |
| `form` | 必填，`enum ["bar", "line", "pie", "limits"]` | 形态 |
| `orientation` | `enum ["vertical", "horizontal"]`，可省，缺省 `vertical` | 只用于 `bar` |
| `series` | 可省；非空数组，每项必填 `id`（common 的 `id`）与非空 `name` | 系列，顺序即图例与数值顺序 |
| `rows` | 必填，非空数组；每行必填 `id`、非空 `label` | 一行一个比较项、时间点、扇区或限额项 |
| `rows[].values` | 非空的 `number` 数组 | `bar`、`line`、`pie` 的数值，逐个对应系列 |
| `rows[].value` | `number`，`minimum 0` | `limits` 的当前值，可省（只有上限） |
| `rows[].limit` | `number`，`exclusiveMinimum 0` | `limits` 的上限 |
| `rows[].unit` | 非空字符串，可省 | `limits` 该行的单位 |
| `rows[].note` | 字符串 | `limits` 该行的说明 |
| `rows[].highlight` | 布尔 | 高亮该行 |
| `cards` | common 的 `cards` | 与其他图型相同 |

最小示例 `skills/phoenix-diagram/examples/minimal.chart.json` ★：`form: "bar"`、两个系列、三行、其中一行 `highlight`，`meta` 为 `{"title": …, "output": "minimal.chart.html", "quality_profile": "showcase", "locale": "en", "evidence": "illustrative", "unit": "ms"}`。

### 5.2 语义校验（`tools/phoenix-diagram/src/input/semantics.rs`）

`validate` 的集合表增加 `DiagramType::Chart => ("rows", "relationships")`，行 `id` 走通用的 `checks.unique`；末尾的图型分派增加 `chart_checks(document, nodes, &mut checks)`，照 `waterfall_checks`（L818）写。系列 `id` 用一次 `checks.unique(array(document, "series"), "/series", …)`。

| 诊断码 | 触发 | 路径 |
| --- | --- | --- |
| `chart/form-fields` | `bar`、`line`、`pie` 的行缺 `values`，或带 `value`、`limit`、行级 `unit`、`note`；`limits` 的行缺 `limit` 或带 `values`；`orientation` 出现在非 `bar`；`series` 出现在 `limits` | 出错的字段；缺字段时指到行 `/rows/<i>` |
| `chart/series-count` | 行的 `values` 个数不等于系列数（没写 `series` 时为 1） | `/rows/<i>/values` |
| `chart/pie-value` | `pie` 的系列数大于 1；某行数值为负；全部数值为 0 | `/series`；`/rows/<i>/values/0`；`/rows` |
| `input/schema`（既有） | 数值不是数字、未知 `form`、缺 `meta.evidence` 等 | schema 实例路径，如 `/rows/2/values/1`、`/form` |
| `input/duplicate-id`（既有） | 行或系列 `id` 重复 | `/rows/<i>/id`、`/series/<i>/id` |

每条消息写出期望与实际（例如「第 3 行有 1 个数值，系列有 2 个」）。校验未过时不进入布局，不产出 HTML 或静态图。

### 5.3 场景映射与共同规则（`tools/phoenix-diagram/src/layout/chart.rs` ★）

入口 `pub fn compile(input: &DiagramInput, _options: &CompileOptions) -> Result<CompiledDiagram, Diagnostics>`，照 `layout/waterfall.rs` 的 `compile` 组织：先把输入读成内部行结构，再算几何，最后组装场景。

**场景**

- `nodes`：每行一个。`authored` 为 `{id: 行 id, source_path: "/rows/<i>", kind: "chart-row", facts: 原始行}`；`bounds` 是行带；`label` 是行标签；`text` 依次是标签（`/rows/<i>/label`）与该行各数值标注（`/rows/<i>/values/<k>`，限额为 `/rows/<i>/limit`）。
- `containers`（`authored.kind`）：
  - `chart-row`：每行一个，`id` 与 `members` 指向该行节点；`facts` 带绘制所需的全部事实：`tone`、`highlight`、`dim`、`sublabel`、`marks`（每个系列一项：`series`、`tone`、矩形或点坐标；饼图为起始角与扫过角；限额为轨道、填充、上限线位置与 `over`）。照 waterfall 的 `waterfall-row`。
  - `chart-axis`、`chart-tick`：数值轴与刻度，`chart-tick` 的 `facts` 为 `{"value": 刻度值, "zero": 是否零线}`。照 `waterfall-axis`、`waterfall-tick`。
  - `chart-series`：折线每个系列一条折线，`facts` 为 `{"series": k, "tone": …, "points": [[x, y], …]}`。
  - `chart-evidence`：仅 `illustrative` 时一个徽标。照 `waterfall-evidence`。
  - `chart-caption`：饼图的顺序说明。
- `legends`：`legend-title` 加条目。柱与折线在系列数 ≥ 2 时每系列一项（`kind` 为 `series:<id>`，文字「A · 系列名」，`facts` 为 `{"tone", "key"}`）；限额固定有「当前值」「上限」，存在超限行时加「超限」；饼图没有图例（右侧列表即图例）。条目换行照 waterfall 的 `legend_rows`；画布宽度取内容与最宽图例条目的较大者。
- `relations` 为空；`cards` 用 `super::tree::cards(document)`；`reader_fit` 为 `"width-first"`。
- `facts`：`scene.rs` 新增

  ```rust
  Chart {
      form: String,
      orientation: String,
      unit: String,
      evidence: String,
      series: Vec<Value>,   // {"id", "name", "key"}
      rows: Vec<Value>,     // {"id", "values"} 或 {"id", "value", "limit", "unit", "over"}
      table: Value,         // {"columns": [...], "rows": [[...], ...]}
  }
  ```

**共同规则**

- 数值文字：`js_to_string(v)`。系列数 ≥ 2 时标注为「键 + 空格 + 数值」。系列键按序取 `A`…`Z`、`AA`、`AB`…。
- 单位后缀：没有单位为空；单位是 `%` 时直接相接；其余前置一个空格。
- 色槽：系列 k 取 `["frontend", "backend", "database", "cloud", "messagebus"]` 的第 `k % 5` 个；单系列全部取第一个。
- 高亮：存在 `highlight` 行时，高亮行标签字重 750，其余行的图形与数值标注加 `ch-dim`；没有高亮行时不加。
- 字号与宽度估计沿用 waterfall：标签 12.5、数值 11、刻度 10.5、图例 13；文字宽度按 `text_units(s) × 字号 × 0.6` 估计。
- 刻度：`waterfall::tick_step(span, length, min_tick_px)`（本期给它加第三个参数，waterfall 传原来的 80）。轴的下限、上限向外取整到步长的整数倍；刻度文字用 `waterfall::number(t)` 加单位后缀。范围算不出有限比例时报 `chart/invalid-scale`（路径 `/rows`，阶段 `layout`），照 `timing_failure`；这一步要在调用 `tick_step` 之前判断，极大数值的差为无穷时 `tick_step` 内的解析会 panic。
- 行带：相邻行带间距 10；标签、图形与数值标注都落在行带内。
- 页眉：`illustrative` 时在画布左上画徽标（文案 `chart.evidence.illustrative`），其下留出与 waterfall 相同的页眉高度；`sourced` 时不画徽标也不留页眉带。
- 数据表 `table`：
  - 柱、折线：列为「项目」加各系列名（没写 `series` 时为「数值」），有单位时列名后加「（单位）」；每行为标签与各数值文字。
  - 饼：列为「项目」「数值」「占比」；单位是 `%` 时没有「占比」列。
  - 限额：列为「项目」「当前值」「上限」「单位」「状态」，任一行有 `note` 时加「备注」；没有当前值写「—」，状态写「超限」「未超限」或「—」。
  - 表头与状态文字取自目录文案，数值与数值标注同一写法。

### 5.4 四种形态的布局

**柱状图，竖向**（`form: "bar"`，缺省方向）

- 每行一个等宽的槽。柱宽取「22」与「最宽数值标注 + 6」的较大者；槽宽取「该行全部柱加柱间 4」与「最宽行标签」的较大者再加 24。行带是槽内缩 5 的竖条，从绘图区上方的标注空间到标签下沿。
- 绘图区高 240。轴范围是 `[min(0, 最小值), max(0, 最大值)]` 向外取整；最小刻度间距 40。全部为 0 时轴为 `[0, 1]`。
- 柱从零线画到数值所在位置：正值向上，负值向下；数值不为 0 时柱长至少 1，数值为 0 时不画柱。数值标注在柱外端的外侧（正值在上，负值在下），与柱同中线。
- 行标签在绘图区下方居中。左侧是刻度文字与水平网格线，零线加重。

**柱状图，横向**（`orientation: "horizontal"`）

- 行自上而下。左列是标签，列宽取最宽标签（至少 140）；右侧是数值轴，轴长 560，最小刻度间距 80。
- 行高是「系列数 × 14 + (系列数 − 1) × 4 + 12」；同一行各系列的柱自上而下排列。柱从零线向右（正）或向左（负）；数值标注在柱外端的外侧。画布在轴两端为最宽的标注留出空间，做法同 waterfall 的 `label_room`。
- 刻度与竖直网格线在行的上方，零线加重。

**折线图**（`form: "line"`）

- 行是横轴上的等距位置。槽宽取「最宽行标签」与「最宽数值标注」的较大者加 16；点在槽的中线。
- 绘图区高 240。轴范围是 `[最小值, 最大值]` 向外取整，不强制含 0；全部相等时上限加一个步长。
- 每个系列一条折线（按行序连接各点，只有一行时没有线段）与每个点一个圆点（半径 3.5）。
- 数值标注默认在点的上方 8。同一行内按点的纵坐标排序后自上而下排开，相邻标注基线间距至少 13；排到行带底部以下时整体上移。该行所需高度超过行带时报 `chart/label-room`（路径 `/rows/<i>`，阶段 `layout`，修法写「减少系列，或拆成多张图」）。
- 数值标注带与底色同色的描边（`paint-order: stroke`），压在折线上仍可读。

**饼图**（`form: "pie"`）

- 左侧是环形图：外半径 110，内半径 62，从 12 点方向起顺时针，按行序排列；扇区角度是「数值 ÷ 合计 × 360」。数值为 0 的行不画扇区；只有一个非 0 行时画整环。
- 右侧是列表，每行一项，行距 28，行带高 18：序号、色块、标签、数值加单位、占比。占比是「数值 ÷ 合计 × 100」保留至多一位小数加 `%`；单位是 `%` 时不写占比。
- 扇区色槽按行序循环；行数大于 1 且除以 5 余 1 时，最后一行改用 `external`，避免首尾相邻同色。
- 环形图下方一行说明（文案 `chart.pie.order`）：扇区自 12 点方向顺时针排列，顺序与列表相同。列表序号与这句说明是颜色之外的对应方式。

**限额对比图**（`form: "limits"`）

- 行自上而下，行距 44。左列是标签（至少 140），`note` 作为标签下的第二行小字；右侧是长 420 的轨道。
- 每行自己的比例：轨道满长对应「上限与当前值的较大者 × 1.25」，从 0 开始。填充条画到当前值；没有当前值时画到上限。上限处一条竖线，上下各超出轨道 4。
- 轨道右侧的文字：有当前值时用 `chart.limit.pair`（「13 / 上限 20」加单位），只有上限时用 `chart.limit.only`。
- 当前值大于上限即超限：填充条用 `security` 色槽，文字后加「· 超限」并加粗。

### 5.5 渲染（`tools/phoenix-diagram/src/render/chart.rs` ★）

照 `render/waterfall.rs`：`pub(super) const RENDERER: TypeRenderer`，四个钩子。

- 根属性：` data-chart-ui="" data-chart-form="<form>" data-reader-min-text="7.5" data-node-kind-derived=""`；柱状图另加 `data-chart-orientation`。不设 `data-motion-choreography`。
- `defs`：一个 `<style>`，每条规则以 `svg[data-chart-ui]` 开头，只用 `static_style::supported_declaration` 清单内的声明；颜色全部取既有主题变量（`var(--<tone>-stroke)`、`var(--lane-stroke)`、`var(--text-dim)`、`var(--bg)`）。类名用 `ch-` 前缀：`ch-band`、`ch-band-alt`、`ch-bar`、`ch-point`、`ch-line`、`ch-slice`、`ch-track`、`ch-limit`、`ch-grid`、`ch-zero`、`ch-axis`、`ch-num`、`ch-dim`。照 waterfall 加一条去掉节点框阴影的规则。
- 主体绘制顺序：示意徽标 → 轴与刻度 → 折线 → 各行。
- 每行的 `<g>` 照 `write_row`（L72）：`id="node-<id>"`、`data-node-id`、`data-node-label`、`tabindex="0"`、`role="button"`、`aria-label`（文案 `node.focus.detail`）、`aria-pressed="false"`、`data-node-kind`（该行色槽）、`data-node-sublabel`（该行全部数值，如「p50 12 ms · p95 40 ms」）、`data-source-path`，再加 `data-chart-values="12,40"`（限额为 `data-chart-value`、`data-chart-limit`、`data-chart-over`），内含 `<title>`。
- `<g>` 内依次是：行带 `<rect data-graph-role="node-body" class="ch-band…">`；图形（柱 `<rect class="c-<tone> ch-bar" data-chart-series="k">`、点 `<circle class="t-<tone> ch-point" data-chart-series="k">`、扇区 `<path class="c-<tone> ch-slice" data-chart-sweep="角度">`、限额的轨道、填充与上限线）；标签 `<text data-node-label="">`；数值标注 `<text class="t-primary ch-num" data-chart-value-label="k">`。
- 刻度线带 `data-chart-tick="<刻度值>"`；示意徽标的 `<g>` 带 `data-chart-evidence="illustrative"`。
- 图例符号：系列为色块加系列键；限额的「上限」为一条竖线。

五语言目录新增的键（`tools/phoenix-diagram/viewer/locales/*.json`，五份都写译文）：`diagram.description.chart`、`chart.evidence.illustrative`、`chart.pie.order`、`chart.limit.pair`（占位符 `{value}`、`{limit}`）、`chart.limit.only`（`{limit}`）、`chart.limit.over`、`chart.limit.within`、`chart.table.summary`、`chart.table.item`、`chart.table.value`、`chart.table.share`、`chart.table.current`、`chart.table.limit`、`chart.table.unit`、`chart.table.status`、`chart.table.note`、`legend.chart.current`、`legend.chart.limit`、`legend.chart.over`。写完运行装配脚本重新生成 `tools/phoenix-diagram/assets/template.html` 与 `tools/phoenix-diagram/assets/catalogs.json`。

### 5.6 内容检查（`tools/phoenix-diagram/src/check/chart.rs` ★）

`pub(super) fn content(svg, metadata) -> Vec<ContentFinding>`，在 `check.rs` L1042 的分派里登记；照 `check/waterfall.rs`，从元数据 `facts` 与产物重新核对，失败码沿用 `artifact/semantic-text`、`artifact/semantic-relation`，路径指到 `/metadata/facts/rows/<i>`。

- 全部形态：每行的节点存在；标签文字相同；每个数值标注的文字等于按 §5.3 规则由 `facts` 得到的文字（含系列键）；`data-chart-values`（或限额的三个属性）与 `facts` 相同。
- 柱：存在 `data-chart-tick="0"` 的刻度；由两条刻度线的位置与刻度值得到线性比例；每根柱一端在零线、另一端在数值对应的位置（误差 0.05 内），负值在零线另一侧。
- 折线：每个点的纵坐标在同一线性比例上。
- 限额：填充条长度与轨道长度之比等于「显示值 ÷ 该行满长对应值」；上限线在上限对应的位置；「超限」文字当且仅当当前值大于上限时出现。
- 饼：各扇区 `data-chart-sweep` 等于「数值 ÷ 合计 × 360」，合计为 360；列表里的数值与占比文字相同。
- 示意徽标当且仅当 `facts.evidence` 为 `illustrative` 时存在；系列数 ≥ 2 时图例每系列一项，文字为「键 · 系列名」。

`pub(super) fn table(document, metadata) -> Vec<ContentFinding>`，在 `check.rs` 卡片检查（L905–L946）之后调用：`details.chart-data` 恰有一个，表头与每个单元格的文字与 `facts.table` 逐格相同。

读不清的情形由既有检查承担，不为 chart 另设数量上限：节点文字投影字号低于 6 px（`composition/desktop-readability`）、节点越出画布、图例与内容相撞，以及浏览器检查里文字越出节点框或画布。

### 5.7 阅读器里的数据表（`tools/phoenix-diagram/src/render.rs`、`tools/phoenix-diagram/viewer/viewer.css`）

`render_with_layers` 在卡片循环之后、`cards.push_str("</div>")`（L1455）之前：`scene.facts` 是 `Chart` 时写入

```html
<details class="chart-data"><summary>数据表</summary><div class="chart-data-scroll"><table><thead><tr><th scope="col">项目</th>…</tr></thead><tbody><tr><th scope="row">行标签</th><td>12</td>…</tr></tbody></table></div></details>
```

`<summary>` 文字取 `chart.table.summary`；全部文字经 `escape_text`。其他图型不经过这个分支。

`viewer.css` 在 `.card` 规则（L2377）之后新增 `.chart-data` 一组规则：面板底色与边框同 `.card`；`.chart-data-scroll` 横向滚动；表格数字用等宽数字（`font-variant-numeric: tabular-nums`）；在网格布局里占满整行（`grid-column: 1 / -1`）。只用 `.chart-data` 开头的 HTML 选择器，不写引用 SVG 根的规则。

### 5.8 静态图（`tools/phoenix-diagram/src/render/static_style.rs`）

`chart` 的静态图走既有的 `static` 命令与转换，不新增代码路径：根属性、`defs` 样式与图形按 §5.5 的写法即可通过转换。

`write_common`（L1300）在卡片数据之后、公共 `<style>` 之前：`scene.facts` 是 `Chart` 时写入

```text
<metadata data-phoenix-table="">{"columns":["项目","p50（ms）","p95（ms）"],"rows":[["Redis","12","40"],…]}</metadata>
```

内容是 `facts.table` 的 `serde_json::to_string` 经 `escape_text`；每个单元格都是字符串，每行长度等于列数。其他图型不写。

### 5.9 登记点与兼容

照 statechart 的新增提交 `6bb21b6b` 逐项补 `chart`：

| 文件 | 改动 |
| --- | --- |
| `tools/phoenix-diagram/src/input.rs` | `DiagramType::Chart`；`ALL` 长度 12；`as_str` 返回 `"chart"` |
| `tools/phoenix-diagram/src/schema.rs` | `PHOENIX` 长度 12，`include_str!` 新 schema |
| `tools/phoenix-diagram/src/input/semantics.rs` | §5.2 |
| `tools/phoenix-diagram/src/layout/mod.rs` | `pub mod chart;` 与 `compile` 分派 |
| `tools/phoenix-diagram/src/render.rs` | `mod chart;`、`type_renderer` 分派、§5.7 |
| `tools/phoenix-diagram/src/check.rs` | `mod chart;`、内容与数据表两处调用 |
| `tools/phoenix-diagram/src/scene.rs` | `DiagramFacts::Chart` |
| `tools/phoenix-diagram/src/evidence.rs` | `collection` 对 `Chart` 返回 `None`（与 statechart 同分支） |
| `tools/phoenix-diagram/src/cli.rs` | `HELP` 的 `Types:` 行加 `chart`；`guide` 的提示词加一组：`chart`、`bar chart`、`line chart`、`pie`、`quota`、`limit`、`柱状图`、`折线图`、`饼图`、`占比`、`限额`、`配额`、`数值对比`、`趋势` |
| 其余对 `DiagramType` 穷举的 `match` | 按编译错误逐处补齐，`chart` 与 waterfall 取同一分支（没有关系、没有品牌） |
| `tools/phoenix-diagram/tests/cli.rs` L123、L145 附近 | `Types:` 行断言；`guide` 用例加一条数值图表场景 |
| `tools/phoenix-diagram/tests/check_anchors.rs` L138 附近 | `example(kind)` 加最小示例 |
| `tools/phoenix-diagram/tests/support/input_corpus.rs` | `minimal` 给一份两行单系列的 `bar`；`collections` 返回 `("rows", "relationships")` |
| `scripts/build-phoenix-diagram.mjs` L106、`test/phoenix-diagram/build.test.js` L126 | schema 清单加 `chart` |
| `test/phoenix-diagram/skill.test.js` | `types` 与 `references` 清单加 `chart`；`collections` 加 `chart: ['rows']`；`chart` 与 timeline、waterfall 一样不要求关系；两个用例名改为十二类 |

兼容（WORK-N3/AC-4，判定口径见 D14）：十一类的布局、渲染与检查代码不改；`waterfall::tick_step` 只加参数，waterfall 传原值，既有 waterfall 基准用例覆盖。模板因目录文案与 `.chart-data` 样式变化，这是唯一会改变十一类 HTML 字节的来源。

### 5.10 phoenix-diagram skill 与文档的文字

- `skills/phoenix-diagram/SKILL.md`：
  - 描述改为：`用于架构、工作流、时序、数据流、状态、状态机模型检查、ER、类、树、时间线、耗时瀑布图、数值图表及 Mermaid 转换；不用于仪表盘、多图联动或其他 infographic。缺必需事实时仅提必要问题，没有真实数值不画数值图表；齐备后交付经 Rust、浏览器与实际看图核对的独立 HTML，可另取可内联的静态 SVG。品牌采集、本地预览和架构比较仅在明确请求时启用。`（183 字；前 100 字含 `skill.test.js` L24 要求的全部词）。
  - 「选择图型」表加一行 `chart`：「数值的比较、趋势、占比与限额：柱状图、折线图、饼图、限额对比图」，示例链到 `examples/minimal.chart.json`。
  - L40 的排除句改为：仪表盘、多图联动、实时数据和其他 infographic 不在范围内，不能改称上述图型来绕过。
  - 「最短创作路径」第 1 步加一句：`chart` 另读[数值图表](references/chart.md)；它没有 `sources`、品牌或架构比较。
- `skills/phoenix-diagram/references/chart.md` ★（照 `skills/phoenix-diagram/references/statechart.md` 的体例，只写创作规则）：
  - 数值只取自授权材料；材料没有真实数值时不画数值图表，也不为画图去运行项目或补造数值；作者明确要求示意图时写 `meta.evidence: "illustrative"`。
  - 形态怎么选：比较各项用柱状图，标签长或项多时用横向；随时间或顺序的变化用折线图；占一个整体的份额用饼图；每项一个当前值对一个上限用限额对比图。
  - 数值写纯数字，单位写在 `meta.unit`（限额可逐行写 `unit`）；多个系列时写 `series`，每行的数值个数与系列数相同；`highlight` 标出要读者先看的行。
  - 每个数值都会标在图上；图太宽、标注放不下而校验失败时，减少点数或系列、改用横向柱状图，或拆成多张图，不删改数值来过检。
  - 交付答复说明形态、数值是否为示意，以及数据表在哪里展开。
- `skills/phoenix-diagram/schemas/README.md` L3、L5、L7：类型清单加 `chart`，改为十二类；语义门一句加「数值图表的形态与字段搭配、系列数与饼图数值（`chart/*`）」；「其余九类为 v1」改为十类。
- `skills/phoenix-diagram/references/extending-types.md` L3、L7、L12，`skills/phoenix-diagram/references/preview.md` L15，`skills/phoenix-diagram/references/viewer-runtime.md` L18：十一类改为十二类。
- `skills/phoenix-diagram/references/static-figure.md` 与 `skills/phoenix-diagram/references/delivery-contract.md` 描述静态产物的一句各补：数值图表另带 `<metadata data-phoenix-table="">` JSON（`columns` 与 `rows`，单元格均为字符串），由宿主显示成数据表。
- `docs/phoenix-diagram.md` L3：类型清单加 `chart`、改为十二类，句末排除改为「仪表盘、多图联动及其他 infographic 不在范围内」，并加一句数值图表的四种形态与数据表；L115：十一类改为十二类，「有关系的九类」不变。
- `README.md` L216：「十一类 Rust 图解（含状态机模型检查）」改为「十二类 Rust 图解（含状态机模型检查与数值图表）」。`bin/xgent-skills.js` L301：「十一类图解」改为「十二类图解」。
- 运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle`（C-7）。

## 6. elintp

### 6.1 解析与渲染静态图的数据表（`skills/elintp/scripts/draft.mjs`）

- 静态模式的 `metadata` 分支（L426–L431）改为：必须是根的直接子元素，恰有一个属性，属性是 `data-phoenix-cards=""` 或 `data-phoenix-table=""`，两种各至多一个；其间只允许文本。不符时报「静态图数据须为根下的 metadata[data-phoenix-cards] 或 metadata[data-phoenix-table]，各至多一个」。
- 闭合时按属性分别解析：卡片沿用 `parseCards`（L315）；表格 `JSON.parse` 后要求是恰有 `columns`、`rows` 两个键的对象，`columns` 是非空的字符串数组，`rows` 是数组，每行是与 `columns` 等长的字符串数组。不符即报「数据表结构不合约」。
- `inlineSVG` 的回调改为一次交回两者：`onData({ cards, table })`；`figure` 分支（L629–L636）相应接收。
- `figure` 分支在 `static` 来源且有表格时，在图与卡片区之间插入：

  ```html
  <details class="figure-data"><summary>数据表</summary><div class="table-scroll" role="region" tabindex="0" aria-label="数据表"><table><thead><tr><th scope="col">…</th></tr></thead><tbody><tr><th scope="row">…</th><td>…</td></tr></tbody></table></div></details>
  ```

  `<summary>` 与 `aria-label` 用已有的 `ui.table`；单元格一律 `escapeHTML`，不走行内 Markdown。表格不带标题元素，不进入大纲。

### 6.2 页面样式与打印（`skills/elintp/assets/page.css`、`page.js`）

- `page.css` 在 `.figure-cards`（L116）旁新增 `.figure-data`：`margin-top: 12px`；`summary` 字重 650、`cursor: pointer`；内部 `.table-scroll` 的上外边距改为 8px。颜色沿用页面变量。
- `page.js` 已有的 `beforeprint` 钩子（L23）里加一句：把页面上所有 `details.figure-data` 设为展开。禁用脚本时 `<details>` 由浏览器原生展开与收起。

### 6.3 skill 文字（`skills/elintp/SKILL.md`、`skills/elintp/references/draft-format.md`）

- `SKILL.md`「出图」对照表加一行：「需要比较的真实数值：各项对比、随时间的变化、占比、当前值对上限」→ `chart`。
- 表后加一段：材料给出需要比较的真实数值且 phoenix-diagram 可用时，用数值图表；数值、标签与单位原样取自材料。材料没有真实数值时不画数值图表，也不估数；作者明确要求示意图时按 phoenix-diagram 的规则标为示意。单个关键数字仍用 `:::metrics`。已安装的 phoenix-diagram 没有 `chart` 图型或按其流程无法交付时，改用 `:::metrics` 或表格呈现这些数值，并在交付说明写明原因。
- 「合并后仍是一个离线 HTML」一段在卡片一句之后补：数值图表的静态图带数据表数据时，数据表随图显示为可展开的表格。
- `skills/elintp/references/draft-format.md` L179 的 `static:` 一段补同一句；「常见报错」表加一行：「数据表结构不合约」→ 重新用 phoenix-diagram `static` 生成，不手改 SVG。

### 6.4 整合清单在本期的落点

| PRD §5.3 条目 | 本期改动 | 对原则的影响 | 核对 |
| --- | --- | --- | --- |
| C6 · elintp「出图」 | 对照表加数值图表；补「没有真实数值不画」 | 不自动安装、不新增安装检测脚本、默认嵌阅读器不变 | V-3；V-5 第 4 步读 diff |
| C7 · elintp 的描述与触发 | 不改 | — | `git diff` 中 elintp frontmatter 无变化 |
| C10 · phoenix-diagram 的图型与描述边界 | 新增 `chart`、改写描述边界；拒绝固定数量上限；本期不含变更标记与逐词标注 | 仪表盘与其他 infographic 仍排除；不设节点、行、系列配额 | `skill.test.js`；V-5 第 4 步读描述全文与前 100 字 |
| C12 · phoenix-diagram 的输出形态 | 静态图多一种随图数据（数据表） | 独立 HTML 的交付不变 | V-2、V-5 |

## 8. 失败模式、发布与回滚

运行期失败沿用单次命令语义：校验或布局失败时返回带阶段与路径的诊断，不写产物，由作者改 candidate 后重跑；没有并发写、后台任务或跨资源提交。

- 新增失败模式：**新引擎 + 旧 elintp**。旧 elintp 只认识卡片 `metadata`，数值图表的静态 SVG 会被判为格式错误（退出 2，不覆盖旧页）。elintp 作为普通 skill 独立更新，phoenix-diagram 随 npm 安装器更新，两者不保证同时更新。恢复：更新 elintp；无法更新时改用 `reader:` 嵌入，或按 §6.3 的降级呈现数值。其余十一类的静态图不写表格数据，不受影响。
- 发布顺序：引擎与 skill 随同一个 npm 包发布；本计划不发布。发布时按 [docs/phoenix-ui.md](../phoenix-ui.md) 的 `release:phoenix` 流程，源码输入已变，不能用 `--reuse-engines`。CHANGELOG 在发布时记录数值图表与图型增至十二类。
- 放量/切换门：无。
- 回滚门：回退提交即可；旧引擎不认识 `chart`，elintp 按 §6.3 降级。已生成的 HTML 与静态图是普通文件，不受影响。

## 9. 验证

### 9.1 行为与接口验证

**环境**

- 在仓根执行；工具已按 `node scripts/prepare-phoenix-diagram.mjs --prepare` 准备好，缺失即失败（C-6）。
- `CARGO` 表示 `mise exec --no-deps -- mbx`，`MF` 表示 `--manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline`。
- 开发引擎：`mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/elintp-p4 --sync-dist`，引擎为 `E=local/phoenix-diagram/elintp-p4/phoenix-diagram-darwin-arm64`。输出目录须为新目录或空目录，每次构建换一个目录名后缀；耗时未知。
- 产物目录：`OUT=local/phoenix-diagram/elintp-p4-out`（`local/` 不入库）。
- 看图由具备图像能力的 Agent 实际打开截图完成；没有图像能力时如实保留未完成。
- 阅读器 HTML 的图面指纹（D14）用下面的短命令取，输出每个文件的主图 SVG 加内嵌元数据的 SHA-256：

  ```sh
  node -e 'const fs=require("fs"),c=require("crypto");for(const f of process.argv.slice(1)){const h=fs.readFileSync(f,"utf8"),a=h.indexOf("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox="),b=h.indexOf("</svg>",a)+6,m=/<script[^>]*id="phoenix-diagram-metadata"[^>]*>([\s\S]*?)<\/script>/.exec(h);console.log(c.createHash("sha256").update(h.slice(a,b)).update(m[1]).digest("hex"),f)}' "$@"
  ```

**基线（M1 动工前执行一次，结果写进 M1 记录）**

在未改动的 HEAD 上按上面的命令构建到 `local/phoenix-diagram/elintp-p4-base`，对十一类最小示例各执行：

1. `deliver <type> skills/phoenix-diagram/examples/minimal.<type>.json local/phoenix-diagram/elintp-p4-base-out/<type>.html --json`，记录图面指纹。
2. `static <type> … <type>.embedded.svg --json` 与 `static <type> … <type>.system.svg --font-policy system --json`，记录两份 SVG 的 SHA-256。

**批内最小反馈**

- 单个 Rust 测试：`$CARGO test $MF --test chart <过滤词>`；只编译：`$CARGO test $MF --no-run`。
- 单个 candidate：`$CARGO run $MF --bin phoenix-diagram -- validate chart <candidate.json> --layout-json --json`（缓存已热时约 2 秒）。
- Node 单文件：`mise exec --no-deps -- node --test <文件>`。
- 开发构建只在批末或收口；整 crate 测试与 clippy 只在 V-5 收口运行。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **图解作者把三组真实耗时画成对比图并交付。** 输入：作者自己写的三份 candidate——单系列竖向柱状图，数值取自 PRD §1.1（手写 HTML 平均 4,893 token，内容稿平均 612 token）；两个系列的竖向柱状图，数值取自 [P3 计划](ELINTP-VISUALS-P3.md) ADR-1 的体积记录（阅读器 HTML 967–989 KB，内嵌字体静态图 157–177 KB，系列为最小与最大）；作者明确要求示意的横向柱状图，含负值与一行高亮，`meta.evidence` 为 `illustrative`。覆盖 WORK-F6/AC-1、AC-2、AC-3（数值与个数部分）、AC-4（数据表）、AC-5（示意标示）、AC-6。结果：<br>• 得到 HTML、两张截图与 summary，自动阶段全过；看两张图：每个标签、系列、数值与单位都在图上且与 candidate 一致，轴含 0，负值在零线另一侧，图例写「A · 系列名」，数值标注带系列键，高亮行可辨；示意那份图上有「示意数值」徽标，另两份没有<br>• 在 Chrome 打开 HTML：展开数据表，全部数值在表里并可选中复制；切换亮暗与预设，图与表都可读；搜索行标签可定位，行详情列出该行数值；导出 SVG 与 PNG 得到完整的图<br>• 两张图都写观察后 `deliveryStatus: complete`<br>容易犯的错：`form` 写成 `column`；某行少写一个数值；数值写成字符串 `"12ms"`。三者都退出非 0，诊断路径分别指到 `/form`、该行的 `values`、该数值，且没有产出 HTML | 1. `"$E" guide "三种写法的数值对比" --json`，确认返回 `chart` 及其 schema、示例路径<br>2. `"$E" finalize chart $OUT/cost.json $OUT/cost.html --json`，读 `$OUT/cost.finalize-summary.json`<br>3. 打开两张 1440×900 截图，写审核记录，`"$E" review-record $OUT/cost.html --record … --json`<br>4. 用 Chrome 打开 `$OUT/cost.html` 做上述操作<br>5. 对另两份重复第 2、3 步<br>6. 三份错误输入各运行 `"$E" finalize chart … $OUT/bad-<n>.html --json`，确认退出码、诊断码与路径，且输出文件不存在 | M1 交付齐备、开发引擎构建后 |
| V-2 | 场景 E2E | **图解作者画趋势、占比与限额，并取能放进别的文档的图。** 输入：作者自己写的三份 candidate——两个系列六个位置的折线图；四项份额的饼图；三项配额的限额对比图（一项超限、一项只有上限、带 `note`；其中一行省略行级 `unit`、只依赖 `meta.unit`，其余行写行级单位）。数值取自本仓可核对的事实（例如各 skill 目录的文件数、近几次提交的改动行数），取不到真实数值的那份标为 `illustrative`。覆盖 WORK-F6/AC-1、AC-3（饼图部分）、AC-4、AC-6，WORK-F9/AC-5。结果：<br>• 三份都得到 HTML 并看图登记：折线的每个点有带系列键的数值，接近的数值标注不相互遮挡；饼图列表的数值、占比与扇区顺序对得上，有顺序说明；限额图的超限项有「超限」文字，只有上限的项画到上限，省略行级 `unit` 的项在图面与数据表中均使用 `meta.unit`<br>• 四种形态（含 V-1 的柱状图）各取 `embedded` 与 `system` 两份静态图，自动阶段全过；内联进空白无脚本 HTML 后亮、暗下图面与阅读器截图一致；每份 SVG 字节数不超过同一 candidate 阅读器 HTML 的 50%；SVG 里有一个 `data-phoenix-table`，其 JSON 与阅读器数据表逐格相同<br>• 静态图看两张、登记后 `deliveryStatus: complete`<br>容易犯的错：饼图某行写负值；饼图全写 0；饼图写两个系列；折线某行多写一个数值。都退出非 0，诊断指到具体行或 `/series` | 1. `"$E" finalize chart $OUT/<name>.json $OUT/<name>.html --json`，看图并登记<br>2. `"$E" static chart $OUT/<name>.json $OUT/<name>.embedded.svg --json` 与 `--font-policy system` 写到 `<name>.system.svg`<br>3. 写 `$OUT/host.html`（`<html data-theme="light">` 内联各静态图，无脚本），用 Chrome 打开；改为 `dark` 再看；对照第 1 步的截图<br>4. `wc -c $OUT/<name>.*.svg $OUT/<name>.html`；`grep -c 'data-phoenix-table' $OUT/<name>.embedded.svg`<br>5. 打开静态图的两张截图，`review-record` 登记<br>6. 四份错误输入各运行第 1 步到新路径 | M2–M3 交付齐备、开发引擎构建后 |
| V-3 | 场景 E2E | **文档作者把一份带数字的材料讲成一页，另一份没有数字的材料照常讲。** 角色：只读 `skills/elintp` 与 `skills/phoenix-diagram` 的 skill 文件及其引用文档，不读实现源码。输入：`docs/PRD-ELINTP-VISUALS.md`（§1.1 有可比较的真实数值）；`skills/phoenix-diagram/references/extending-types.md`（没有可比较的数值）。覆盖 WORK-F5/AC-3、WORK-F6/AC-5（不画的一半）、WORK-F6/AC-4、C-1。结果：<br>• 第一份的页面里有一张数值图表，数值、标签与单位和原文一致；默认是页内嵌入阅读器<br>• 同一材料加上「零网络阅读面、图用内联 SVG」的要求再生成：图是静态图，图下有可展开的数据表，全部数值在表里；终稿是单个 HTML，断网打开没有网络请求<br>• 360 px 宽下页面不横向滚动，数据表在自己的区域内横向滚动；只用键盘可以展开数据表；打印预览里数据表是展开的；禁用脚本时仍可展开<br>• 第二份的页面没有数值图表；交付说明不声称画过<br>• 交付说明写明图的形态与实际做过的检查<br>容易犯的错：把手改过表格数据的静态 SVG 交给 `static:`，得到带行号的格式错误且旧页面未被覆盖 | 1. 按 `skills/elintp/SKILL.md` 执行，phoenix-diagram 的命令用 `$E`<br>2. 用 Chrome 打开终稿：DevTools Network 设为 Offline 后刷新；把窗口设为 360 px 宽；用 Tab 与回车展开数据表；打开打印预览；在 DevTools 禁用 JavaScript 后刷新再展开<br>3. `grep -c '<iframe' 终稿`、`grep -c 'figure-data' 终稿`<br>4. 误操作变体运行一次 `render.mjs` | M4 交付齐备后（依赖 V-2 已通过的开发引擎） |
| V-4 | 局部 | 准入依据：分支多的纯逻辑（形态 × 非法输入、刻度与负值、标注避让、系列键），以及不可信输入的解析。<br>• `tools/phoenix-diagram/tests/chart.rs` ★，经公开的校验与编译入口断言回执与场景：§5.2 每个诊断码的触发与路径；全正、跨 0、全 0 三种数值下柱状图的刻度与柱的方向；折线轴不强制含 0；`12`、`4.5`、`0.1`、`1234567`、`-3` 原样写出；同一行数值相同的两个系列标注不重叠；标注放不下时报 `chart/label-room`；第 27 个系列的键是 `AA`；四种形态最小输入的产物检查通过<br>• `test/elintp/render.test.js`：合约的表格数据渲染成转义后的表格；非对象、缺键、行列不等长、单元格不是字符串、两个表格 `metadata`、多余属性、其间有标签，均退出 2 且旧页字节不变；没有表格数据的静态图不出现 `figure-data`<br>• 随清单更新的既有用例：`skill.test.js`、`build.test.js`、`tests/cli.rs`、`tests/check_anchors.rs`、输入语料；阅读器装配 `--check` 与目录用例 | `$CARGO test $MF --test chart --test cli --test check_anchors --test input`<br>`mise exec --no-deps -- node --test test/phoenix-diagram/skill.test.js test/phoenix-diagram/build.test.js tools/phoenix-diagram/tests/viewer/catalogs.test.mjs`<br>`mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check`<br>`mise exec --no-deps -- node --test test/elintp/render.test.js` | 柱状图与清单部分在 M1 批末；其余引擎部分在 M2–M3 批末；elintp 部分在 M4 批末 |
| V-5 | 阶段收口 | 全量回归与兼容（WORK-N3/AC-4、WORK-F9/AC-5、C-3、C-4、C-7）：<br>• 十一类最小示例的图面指纹、两种策略的静态 SVG SHA-256 与基线逐一相同<br>• 十二类最小示例在两种策略下都能取得静态图，自动阶段全过，体积比均不超过 50%<br>• 现行文档中不再有「十一类」与「不用于数值图表」的复述，仪表盘与其他 infographic 仍被排除；skill 描述与引用测试通过；bundle 与源一致 | 1. `mise exec --no-deps -- npm run test:diagram`（fmt、clippy、全部 Rust / Node / Reader 测试；耗时未知）<br>2. `mise exec --no-deps -- node --test test/elintp/render.test.js test/elintp/review.test.js`<br>3. 最终代码上重新开发构建（新输出目录），对十二类各运行 `deliver` 与两策略的 `static`，按 §9.1 的命令比对指纹、SHA-256 与 `wc -c`<br>4. `git grep -n -e '十一类' -e '十一种' -e '数值图表' -- skills README.md docs/phoenix-diagram.md bin ':!dist'`，逐条确认是现行、正确的说法；对照 §6.4 读两个 skill 的 diff<br>5. `node scripts/build-phoenix-diagram.mjs --sync-bundle` 后 `git status --short dist/phoenix-diagram/bundle` 无未提交差异 | M5，代码改完后 |
| V-6 | 真实使用验收 | 从干净的用户状态连续完成三件事，亲自看产物（WORK-F6、WORK-F5/AC-3、WORK-F9/AC-5 的本期部分）：<br>1. 新项目按文档离线安装后，图解作者只凭安装后的 skill 文件，把自己手头的一组数（例如本仓各 skill 目录的文件数）画成横向柱状图，交付阅读器 HTML（看两张、登记），再取静态图（看两张、登记）<br>2. 文档作者在同一项目里用 elintp 把 `docs/phoenix-diagram.md` 加上一步的数讲成一页，明确要求零网络阅读面，页面带上一步的静态图与数据表<br>3. 读者断网打开终稿，切换亮暗，在窄屏下阅读，展开数据表并复制一行数值<br>发现的用户可见问题修复后复验，或列给用户确认延期 | 1. 新建临时目录作项目，按 `docs/phoenix-diagram.md`「离线安装」：`node bin/xgent-skills.js install "$PROJECT" --no-phoenix-ui --providers=claude --diagram-release-dir="<V-5 第 3 步的开发构建目录>"`<br>2. `PD="$PROJECT/.claude/skills/phoenix-diagram/scripts/phoenix-diagram"`，只凭安装后的 `SKILL.md` 与 `references/` 操作<br>3. elintp 用仓内 `skills/elintp`；角色只读两个 skill 的文件与公开文档<br>4. 浏览器操作同 V-3 第 2 步 | M5，V-5 通过后 |

V-3、V-6 是 Agent 角色模拟，不代表真人可用性结论。

## 10. 里程碑与验收安排

M1 一批：它打通新图型的整条链路（输入、布局、渲染、通用检查、阅读器、浏览器检查、看图），是 M2 动工前要先验证的前置——ADR-2 的节点定义是否与通用检查和探针相容，只有跑过真实交付才知道。M2–M3 同批：都在同一个开发引擎上验证，之间没有风险门，连续实施，批末构建一次。M4 一批。M5 收口。M4 的代码可在 §5.8 的数据格式确定后与 M2–M3 并行编写（只写 `skills/elintp` 与 `test/elintp`，与引擎无共享文件）；它的 V-3 需要 V-2 通过后的开发引擎。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | `chart` 图型与柱状图打通 | 基线已记录（§9.1） | **落点**：§5.1 schema ★ 与最小示例 ★；§5.2 语义校验；§5.3 场景与共同规则、§5.4 两种柱状图（`layout/chart.rs` ★）；§5.5 渲染与五语言文案（`render/chart.rs` ★）；§5.6 柱状图与数据表的内容检查（`check/chart.rs` ★）；§5.7 阅读器数据表与 `viewer.css`；§5.9 全部登记点；装配阅读器<br>**同构**：`layout/waterfall.rs` `compile`、`render/waterfall.rs`、`check/waterfall.rs`；登记点照 `6bb21b6b`<br>**文档**：§5.10 中 `skills/phoenix-diagram/SKILL.md` 的四处与 `references/chart.md` ★（`skill.test.js` 要求引用存在）；`--sync-bundle` | V-1、V-4 的柱状图与清单部分；回写「实施进度」 |
| M2 | 折线图、饼图、限额对比图 | M1 的行节点、数值写法与数据表（V-1 已通过） | **落点**：§5.4 其余三种形态、§5.3 的 `chart-series` 与 `chart-caption`；§5.5 对应图形与图例符号；§5.6 对应检查<br>**同构**：M1 的柱状图实现<br>连续实施，中途不跑批末检查 | 与 M3 同批：V-2、V-4；回写「实施进度」 |
| M3 | 静态图的数据表数据，接口文字同步 | M1 的 `facts.table` | **落点**：§5.8 `static_style.rs` `write_common`；§5.10 其余文字（`SKILL.md` 与 `chart.md` 已在 M1）；`--sync-bundle`<br>**同构**：`write_common` 写卡片数据的一段（L1301–L1318）<br>**注意**：`SKILL.md` 描述改后跑 `skill.test.js` | 批末（M2–M3 交付齐备、开发引擎构建后）：V-2、V-4 的其余引擎部分；回写「实施进度」 |
| M4 | elintp 选用数值图表并显示数据表 | §5.8 数据格式；V-3 需 V-2 已通过 | **落点**：§6.1 `draft.mjs`；§6.2 `page.css`、`page.js`；§6.3 `SKILL.md`、`draft-format.md`；`test/elintp/render.test.js`<br>**同构**：`draft.mjs` 的卡片解析（L315–L325、L426–L431）与卡片渲染（L635） | V-3、V-4 的 elintp 部分；回写「实施进度」 |
| M5 | 阶段收口 | M1–M4 完成 | 全量回归、兼容比对、接口复述核对、bundle 同步、真实使用验收；发现的问题修复后只复验失败项与波及项 | V-5、V-6；回写「实施进度」 |

提交点：M1 通过后提交一次（引擎、schema、示例、`chart.md`、图型表与描述、bundle）；M3 批末通过后提交一次（覆盖 M2–M3）；M4 通过后提交一次（elintp）；M5 完成后最终提交。跨期需求 WORK-F5、WORK-F9、WORK-N3 在 M5 后仍停在已提交 / 可测试，终验在 P5。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 行带作节点框与通用检查、浏览器探针的相容性只经代码阅读确认，未跑过真实交付 | 不相容时柱状图过不了自动阶段 | M1 的 V-1 是第一道真实验证；不相容时先调整行带与间距，仍不行则按偏差修订 ADR-2，不为 chart 豁免通用检查 | M1 批末 | 否 |
| D14：十一类阅读器 HTML 整文件字节会变（模板新增文案与样式） | 有人按「整文件字节相同」理解 WORK-N3/AC-4 时会判为不符 | 判定口径已写明并与 statechart 新增时一致；用户如要求整文件字节不变，需另议（目录内嵌模板，新增任何文案都会改变它） | M5 | 否 |
| D5：稠密序列校验失败 | 点数很多的折线图画不出来 | PRD D10 的既定后果；`chart.md` 写明修法。用户要抽稀标注时另立需求 | — | 否 |
| 阅读器侧栏里数据表的观感与窄屏表现未实测 | 表格过宽时可能挤压侧栏 | V-1 第 4 步实际打开查看；问题在 `viewer.css` 的 `.chart-data` 规则内修 | M1 批末 | 否 |
| 新引擎 + 旧 elintp（§8） | 数值图表的静态图被旧 elintp 拒收 | 文档已写恢复办法；发布说明提示一并更新 elintp | 发布时 | 否 |
| 开发构建目录作离线安装的 release-dir | 影响 V-6 第 1 步 | P3 收口已按同一方式安装通过：包侧不入库的 `dist/phoenix-diagram/VERSION.json` 与 `dist/phoenix-diagram/bundle/ENGINE.json` 要取自同一次构建，才能通过安装器的逐字节预检（[P3 M5 记录](ELINTP-VISUALS-P3.records/M5.md)「V-6 真实使用」）；若安装器拒绝，按其报错处理并记录，不用 `$E` 冒充安装验收 | M5 | 否 |

- 最终状态：**Ready**。
- 定级理由：范围、输入契约、诊断、场景映射、四种形态的布局规则、数据表格式、登记点、检查与里程碑已定；前置期 P3 已完成；没有未决的范围、数据、外部契约或明确要求的 NFR 问题。

## 12. 已知坑与历史教训

- `tools/phoenix-diagram/assets/template.html` 与 `tools/phoenix-diagram/assets/catalogs.json` 是生成物，直接改会被 `--check` 判为过期；改 `tools/phoenix-diagram/viewer/viewer.css` 或目录后运行装配脚本（`tools/phoenix-diagram/viewer/README.md`）。
- `build.rs` 的 `collect` 遇到缺失输入会 panic，`include_str!` 指向不存在的文件时整个 crate 编译失败；先建 schema 与示例文件，再加 `include_str!`（[PHOENIX-DIAGRAM-STATECHART 计划](PHOENIX-DIAGRAM-STATECHART.md) §12）。
- `finalize` 与 `static` 的 stdout 很大，宿主直接读取会被截断；读 summary 文件，不读 stdout（[STATECHART 计划收口记录](PHOENIX-DIAGRAM-STATECHART.records/M6.md)「已知项」）。
- 图型 `defs` 里的样式会被静态转换重新解析：规则不引用 SVG 根、用了支持清单外的声明或 `:has()` 时，`static` 报 `static/style` 而阅读器正常（`tools/phoenix-diagram/src/render/static_style.rs` L266–L274）。M1 写样式时就按 §5.5 的约束写，不等到 M3 才发现。
- `viewer.css` 里以 `.c-`、`.t-`、`.a-`、`.m-` 开头的规则会进入每一类的静态图；数据表样式用 `.chart-data` 前缀，不用这四个前缀。
- `skill.test.js` 固定了描述长度、前 100 字的词与 `version: "0.2.0"`；改描述后先跑这个文件。
- 内联 SVG 共用页面的 ID 空间；静态图由引擎加前缀，elintp 不要再改写（[P1 计划](ELINTP-VISUALS-P1.md)完成记录中的评审修复 F-01）。
- 节点事实里的 `label` 会被通用检查拿去和节点文字比对（`check.rs` L959–L986）：行标签必须原样写进该行的 `<g>`，不要拆行、截断或加前后缀。
