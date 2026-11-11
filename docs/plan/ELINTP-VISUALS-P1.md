# ELINTP-VISUALS-P1 · XGENT-ai Skills · elintp 固定模板图文说明（内容稿 + 随 skill 分发的 Node 渲染脚本 · 四种模板 · 单文件离线页面）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-11 · `0b35470b` · dirty（PRD、其评审与决定记录尚未提交；本计划引用的 PRD 事实来自当前工作树的 v1.2）。
>
> 需求来源：[ELINTP-VISUALS PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P1（第 1 期）。
>
> 本期交付：**elintp 改为「模型写内容稿，随 skill 分发的 Node 脚本按四种固定模板生成单文件离线页面」；页面带亮暗切换与大纲，格式错误不出半成品，脚本不可用时退回手写**。
> **开发范围：`skills/elintp/`（SKILL.md、references、scripts、assets）与 `test/elintp/`；依据 PRD §7.2 的 P1 列与 §7.4**。
> 本期独特职责：版式、配色与组件外观从模型现写改为随 skill 固定；模型只负责判型、内容稿与对最终页面的自检。
> **分期边界：审阅标注与提交、页面自带内容稿在 P2；phoenix-diagram 的静态图、新图型与审核减量在 P3–P5。Phoenix Diagram 属于本期 PRD 的修改对象；本计划遇到其直接依赖缺陷时可就地修复并记录影响，不把分期安排解释为禁止修改。** summarize、phoenix-ui 本计划无需改动。

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

- 最近更新：2026-11-11T11:49:32+08:00
- 当前进度：4/6 个里程碑完成
- 当前状态：进行中；M2–M4 浏览器批次通过，进入阶段收口
- 最近完成：M4 · 模板视觉设计
- 下一步：按 M4 提交点保存 M1–M4；完成 M6 的 V-5 浏览器通读、主题、大纲与旧页打开，再最终提交
- 当前阻塞：无；屏幕解锁后 Chrome 已可控制
- 代码基线：`dirty@0b35470b`（`skills/elintp/`、`test/elintp/`、本计划与记录；M4 批次通过，待提交）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M5 | 进行中 | 2026-11-11T11:49:32+08:00 | 六项全新上下文作者演练已取证；M4 前置已通过，随最终文案收口 | [M5 记录](ELINTP-VISUALS-P1.records/M5.md) | — |
| M6 | 进行中 | 2026-11-11T11:49:32+08:00 | 干净副本已完成两任务与第二节重讲；静态收口通过，浏览器通读待验 | [M6 记录](ELINTP-VISUALS-P1.records/M6.md) | — |
| M1 | 已完成 | 2026-11-11T10:09:34+08:00 | 通用模板生成、两处错误一次报告且不覆盖旧页，Chrome 首屏结论可读 | [M1 记录](ELINTP-VISUALS-P1.records/M1.md) | 待提交 |
| M2 | 已完成 | 2026-11-11T11:49:32+08:00 | 四模板和全部组件；V-2、V-3 通过 | [M4 共同验收](ELINTP-VISUALS-P1.records/M4.md) | 待提交 |
| M3 | 已完成 | 2026-11-11T11:49:32+08:00 | 主题、大纲、键盘、无脚本及打印行为通过 | [M4 共同验收](ELINTP-VISUALS-P1.records/M4.md) | 待提交 |
| M4 | 已完成 | 2026-11-11T11:49:32+08:00 | 360/1440 亮暗阅读与真实阅读器交互通过 | [M4 共同验收](ELINTP-VISUALS-P1.records/M4.md) | 待提交 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 的工作号；「来源」列的 AC 即本期承接范围，验收原文以 PRD §3 为准。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F1 | 功能 | PRD §3 WORK-F1/AC-1 至 AC-6 · P1，终验 P1 | 判型后选四种模板之一，只写内容稿；脚本生成单文件页面；格式错误不出页并给位置与改法；脚本不可用时手写降级 | §5.1–§5.6、§6.1、§6.3；V-1、V-2、V-3、V-4 | 已确认 |
| WORK-F2 | 功能 | PRD §3 WORK-F2/AC-1 至 AC-4 · P1，终验 P1 | 九项自检的对象改为最终页面；有修改就改内容稿重新生成；交付说明如实列出做过与没做的检查 | §6.3；V-4、V-5 | 已确认 |
| WORK-F3 | 功能 | PRD §3 WORK-F3/AC-1 至 AC-3 · P1，终验 P1 | 页面按系统偏好显示亮暗并可切换、选择可保留；三个及以上小节带大纲并标出当前小节 | §6.1、§6.2；V-2 | 已确认 |
| WORK-F5 | 功能 | PRD §3 WORK-F5/AC-1、AC-4、AC-5 · 本期只承接这三行；AC-2 在 P3、AC-3 在 P4、AC-6 在 P5，终验 P5 | phoenix-diagram 可用时默认页内嵌入完整阅读器；不可用时用自绘内联图并说明；合并后检查最终页面的图与截图 | §5.4、§6.3；V-2（AC-1、AC-5）、V-4（AC-4） | 已确认 |
| WORK-N1 | 可用性 | PRD §3 WORK-N1/AC-1 至 AC-4 · P1，终验 P1 | 360 CSS px 与 1440×900 无页面级水平滚动；全部交互可用键盘；打印或禁用脚本时内容完整；界面文字三种语言 | §6.1、§6.2、§7；V-2 | 已知 |
| WORK-N2 | 安全 | PRD §3 WORK-N2/AC-1、AC-2、AC-4 · 本期只承接这三行；AC-3、AC-5 在 P2，终验 P2 | 页面单文件、打开与阅读零网络请求；内容稿与原文中的脚本或标记不执行；图片只取作者给出的本地文件，缺失如实标注 | §5.3、§5.4、§7；V-2、V-3 | 已知 |
| WORK-N3 | 兼容 | PRD §3 WORK-N3/AC-1、AC-2 · 本期只承接这两行；AC-3 至 AC-5 在 P3–P5，终验 P5 | elintp 的触发、参数、语言选择、红线、写作准则、转述硬规则、判型与过程记录要求保持有效；调用方不改调用方式；输出位置、命名与出处说明沿用；旧页面仍可打开 | §0.6、§6.3；V-4、V-5 | 已知 |
| C-1 | 约束 | [AGENTS.md](../../AGENTS.md) `@skill_tool_independence` | elintp 独立于 Phoenix 工具：渲染脚本与页面运行不以 phoenix-ui、phoenix-diagram 的安装、引擎或协议为前提 | D5、§4 | 已确认 |
| C-2 | 约束 | [AGENTS.md](../../AGENTS.md) §7、PRD §5.3 | 借鉴参考 skill 的规则前出示整合清单，按清单核对 diff；不改变既有原则、触发与授权策略 | §0.6、M5 | 已确认 |
| C-3 | 约束 | [summarize](../../skills/summarize/SKILL.md) §3 | 调用方的硬要求：单文件、样式与脚本内联、不引用任何外部资源、字体用系统字体栈。模板因此不嵌入字体文件 | §6.1；V-4 | 已确认 |
| C-4 | 约束 | PRD §4 用户约束、PRD D6 | 填充程序是脚本；红线、写作准则与转述保真只靠模型自检，脚本不做词表、句长或数字比对 | D6、§5.5 | 已确认 |
| C-5 | 约束 | PRD §4 参考基线 | 参考工具为 MIT 许可；本期只借鉴行为，不复制其代码、样式或词表，因此不新增第三方声明 | D7 | 已确认 |
| C-6 | 约束 | [test_skill_metadata.py](../../test/skills/test_skill_metadata.py) `FRONTMATTER_KEYS` | SKILL.md frontmatter 只允许白名单字段；本期不新增字段、不改 `description` | §6.3；V-5 | 已确认 |
| A-1 | 假设 | 安装缓存缺少匹配引擎，但工具源码在本仓 | 验证环境能取得一份由匹配引擎交付的 phoenix-diagram 阅读器 HTML，用于 WORK-F5/AC-1 的真实嵌入验证 | §4、§11；从本仓按固定工具链构建，通过 finalize 与 review-record 取得真实阅读器 | 已解除；嵌入验收仍属 V-2 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 脚本语言与运行时 | Node 18+ 的 ESM 脚本（`.mjs`），不引第三方依赖 | 与仓库 `engines.node >= 18` 和已有脚本一致；没有 Node 时走手写降级。未选 Python：仓库经 npm 分发，Node 是已有前提，Python 在部分宿主不保证存在 | [package.json](../../package.json) `engines`；[decision-form.mjs](../../skills/spec-human-decision/scripts/decision-form.mjs) |
| D2 | 内容稿格式 | 受限 Markdown：frontmatter、`##` 小节、`:::` 容器，自写解析器 | 见 ADR-1 | WORK-F1/AC-2、AC-4；WORK-N2/AC-2 |
| D3 | 模板的实现形态 | 一份共享外壳与样式，加每个模板一张 slot 表（顺序、是否必填、界面标签） | 四种模板的组件外观完全相同，差异只在页头标识与小节顺序；模板的版式一致由同一份 CSS 保证。代价：某个模板将来要独有组件时须先改共享样式 | WORK-F1/AC-2、AC-3、AC-6 |
| D4 | 第一屏的结论 | 内容稿第一个 `##` 之前的引言必填，渲染为页面顶部的结论块 | 四种模板的第一屏都先出现结论；通用模板的「结论是什么」由引言承担，不再设同名 slot | WORK-F1/AC-3 |
| D5 | 嵌图方式 | `reader:` 把阅读器 HTML 转义后放进 `iframe srcdoc`；`svg:` 把自绘 SVG 经白名单校验后内联 | 沿用现行 `iframe srcdoc` 做法，不加 `sandbox`，不传阅读器的 `?embed=1`（`srcdoc` 无法带查询参数）。阅读器保留自己的主题控件 | [elintp](../../skills/elintp/SKILL.md)「出图」；PRD WORK-F3 边界；[阅读器模板](../../tools/phoenix-diagram/assets/template.html) 第 44 行 |
| D6 | 脚本校验的范围 | 只校验格式：结构、slot、表格列数、链接协议、文件是否存在、SVG 白名单 | 语义问题留给模型对最终页面的自检；脚本通过不代表写得对 | C-4 |
| D7 | 是否复用参考工具代码 | 不复用，只借鉴「内容稿加脚本」的行为 | 不带入 `marked`、`dagre` 等依赖和 1MB 的打包脚本；不需要许可声明 | C-5；参考包 `local/answer-me-with-html-main/skills/answer-me-with-html/scripts/am.mjs` 1,067,843 字节 |
| D8 | 页面是否自带内容稿 | 本期不带；内容稿是临时文件 | 页面自带内容稿属于 WORK-F4，由 P2 加入。P1 生成的页面进入 P2 审阅前要重新生成一次 | PRD §7.2：WORK-F4 全部在 P2 |
| D9 | 不执行内容的保证方式 | 所有文字在输出时 HTML 转义；格式不支持原样 HTML；自绘 SVG 走元素与属性白名单，URL 型资源引用只允许本文件片段 | 不加 CSP 和 iframe `sandbox`：`srcdoc` 文档会继承宿主页面的 CSP，可能挡掉阅读器自己的内联脚本，而转义与白名单已经关闭了注入入口 | WORK-N2/AC-2；重新评估条件：P2 引入本机审阅连接时 |
| D10 | 不覆盖旧页面的保证方式 | 全部校验与拼装在内存里完成，通过后对输出路径只写一次 | 有任何格式错误就不碰输出文件。不做临时文件加重命名：崩溃在单次写入中途属于假想故障 | WORK-F1/AC-4 |
| D11 | 界面语言 | 内容稿的 `lang` 必填，脚本按标签映射简体中文、繁体中文、英文三套界面文字，其余语言用英文界面 | 不做语言检测。`<html lang>` 原样使用内容稿给出的标签 | WORK-N1/AC-4；[elintp](../../skills/elintp/SKILL.md)「语言选择」 |
| D12 | 模板里可省略的小节 | 通用模板只有 `why` 必填，`scale`、`risks` 在材料确实没有时可省略；计划模板的 `records` 可省略；其余模板表列出的小节全部必填，没有内容时写明「原文没有说」 | 避免为凑小节编造量级或风险；也让「少于三个小节不显示大纲」有真实场景。文档模式下原文的风险与数字是否都在，仍由自检第 7 项把关 | WORK-F1/AC-3、WORK-F3/AC-2；[elintp](../../skills/elintp/SKILL.md)「转述硬规则」4 |

### 0.3 ADR-lite

#### ADR-1：内容稿用受限 Markdown，解析器自写

- 状态：Accepted
- 背景与驱动：WORK-F1/AC-2 要求内容稿不含样式、版式与图形坐标；AC-4 要求每处格式错误给位置与改法；WORK-N2/AC-2 要求内容稿里的脚本或标记不执行。P2 的局部重讲要按小节替换内容稿。格式一旦写进 SKILL.md 并被模型使用，换格式要同时改 skill 文案与所有在用内容稿。
- 备选：
  - A（选中）：受限 Markdown。frontmatter 加 `##` 小节加 `:::` 容器，只支持 §5.3 列出的块与行内标记，其余写法报错。
  - B：JSON 内容稿（与 [decision-form.mjs](../../skills/spec-human-decision/scripts/decision-form.mjs) 的 `form.json` 同类）。校验最简单，但正文是长段落和表格，JSON 转义让模型多写字符且易错；错误只能定位到字段路径，定位不到行。
  - C：完整 Markdown，内置 `marked`（参考工具的做法）。写法最自由，但要引入并随 skill 分发第三方代码，原样 HTML 默认透传，需要另加清洗才能满足 WORK-N2/AC-2。
- 决策：选 A。受限语法让「不支持的写法报错并给改法」本身成为 AC-4 的实现；所有文字只有转义一条输出路径，WORK-N2/AC-2 由构造保证。
- 正面后果：零依赖；错误有行号；小节边界明确，P2 可按 `##` 替换。
- 负面/中性后果：模型习惯的部分 Markdown 写法（引用块、三层以上嵌套列表、原样 HTML）不可用，要靠 [内容稿格式参考](../../skills/elintp/references/draft-format.md) 和报错的改法引导；解析器由本仓维护。
- 重新评估触发：V-4 的演练里同一种不支持的写法反复出现且有真实表达需求时，把它加进语法；需要第三类以上的嵌套结构时，重新比较 C。

### 0.4 职责与事实所有权

|  | 宿主 Agent（执行 elintp） | elintp 脚本与模板（本期） | phoenix-diagram | 调用方 skill（summarize） |
| --- | --- | --- | --- | --- |
| 拥有 | 判型、选模板、写内容稿、对最终页面自检、交付说明、手写降级 | 内容稿的格式校验、四种模板的版式与组件、页面的亮暗与大纲行为、文字转义与 SVG 白名单 | 图的生成、校验、看图审核与阅读器 HTML | 阅读面的硬要求、报告的查重与提交 |
| 不拥有 | 版式、配色、图形坐标 | **语义判断（红线、套话、转述保真）；图的校验与审核；安装任何工具** | elintp 页面的最终显示检查 | 页面的版式与内容 |

一句话：脚本只管格式与外观，内容对不对由模型自检负责，图对不对由 phoenix-diagram 自己的流程负责。

### 0.5 明确不在本期

- **审阅标注、主动提交、局部重讲、终稿与审阅稿的区分、页面自带内容稿**——P2（WORK-F4、WORK-N2/AC-3、AC-5）。
- **要求内联图时改用 phoenix-diagram 静态图**——P3（WORK-F5/AC-2、WORK-F9）。本期不为「调用方要求内联图」新增规则；[summarize](../../skills/summarize/SKILL.md) §3 与 elintp「出图」之间的口径差异维持现状，由 P3 收口。
- **数值图表、变更视图、逐词标注的选用**——P4、P5（WORK-F5/AC-3、AC-6）。elintp 的出图对照表本期不增行。
- **phoenix-ui 工具与 skill、summarize**——本计划无需改动。Phoenix Diagram 的静态图、新图型与审核减量按 P3–P5 推进；P1 嵌入验收发现的直接依赖缺陷，经独立阅读器与嵌入页对照归因后在对应源码修复，记录变更与最小检查。
- **主题库、用户自定义模板、主动触发与常开模式、词表与句长检查、导出 PDF / PNG**——PRD NG1、NG3、NG5、NG8 明确不做。
- **README 的 elintp 一行、`description` 与 `when_to_use`**——名称、参数与触发未变，不改。
- **渲染耗时与页面体积的基线、压测与调优**——后续可选；本期按常规工程做，不设专门检查。

### 0.6 参考整合清单（动 skill 文件前核对）

PRD §5.3 是清单的事实源。下表只列本期触及的条目及其在 [elintp](../../skills/elintp/SKILL.md) 的落点；M5 改文件前逐项核对，改完后对照 `git diff -- skills/elintp/SKILL.md` 再核一遍，出现表外的行为变更先更新本表。

| PRD §5.3 条目 | 本期落点 | 采用、调整或拒绝 | 核对 |
| --- | --- | --- | --- |
| C1 产出形态 | 「产出形态」改为内容稿加脚本加四种模板；新增「选模板」；原有外观要求保留为手写降级的依据 | 调整；拒绝多面板网格、手工列宽、默认输出目录 | 输出位置与命名一段原文不变；WORK-F1/AC-1 至 AC-5 |
| C2 交付前自检 | 自检前加一句「对象是生成后的页面」；写作准则补三条：同一事实只说一次、列表列全不以「等」收尾、不描述自己怎么做的 | 采用这四处；拒绝机器词表与句长检查 | 九项自检编号与含义不变；红线与转述硬规则原文不变；WORK-F2 |
| C5 页面要求 | 结论块、提醒块、状态标记、大纲、亮暗由模板提供；大图示、指标块、表头释义的要求保留 | 调整；拒绝主题下拉与主题库 | WORK-F1/AC-6、WORK-F3、WORK-N1 |
| C6 出图（本期部分） | 图仍由 phoenix-diagram 画，改为通过内容稿的 `:::figure` 嵌入；自绘降级改为独立 SVG 文件 | 调整；对照表不增行（三种新图在 P4、P5） | 「不新增安装检测脚本、不自动安装」原文不变；WORK-F5/AC-1、AC-4、AC-5 |
| C7 描述与触发 | 不改 | 拒绝主动触发与常开 | frontmatter 的 `git diff` 为空；WORK-N3/AC-1 |
| C8 语言选择 | 规则不变；内容稿加 `lang`，界面文字三种语言 | 调整 | 「语言选择」一节原文不变；WORK-N1/AC-4 |
| C9 截图规则 | 截图用 `![说明](本地路径)`，脚本嵌入；缺失如实标注；不生成图片 | 采用，与现有规则一致 | WORK-N2/AC-4 |

C3、C4（标注与局部重讲）属 P2，C10 至 C12 属 P3–P5，C13 已拒绝，本期都不触及。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·足够]** elintp 只有一个 [SKILL.md](../../skills/elintp/SKILL.md)，没有 `scripts/`、`assets/`、`references/`、`agents/openai.yaml`。触发是 `/elintp` 或明确要求；交付前自检共九项。
- **[已核实·足够]** 随 skill 分发脚本与静态资源的范式已存在：[decision-form.mjs](../../skills/spec-human-decision/scripts/decision-form.mjs) 用 `join(dirname(fileURLToPath(import.meta.url)), '..', 'assets')` 定位资源，用 `fill(html, '/*__FORM_CSS__*/', …)` 把 CSS 与 JS 填进 HTML 外壳，校验失败 `fail(lines, 2)`，成功打印带前缀的结果行。测试在 [decision-form.test.js](../../test/spec-human-decision/decision-form.test.js)，以子进程运行脚本并断言输出。
- **[已核实·足够]** `skills/` 整个目录随 npm 包分发（[package.json](../../package.json) `files`）；[run-tests.mjs](../../scripts/run-tests.mjs) 自动收集 `test/<目录>/*.test.js`，新测试文件无需登记。
- **[已核实·足够]** phoenix-diagram 阅读器对存储与地址栏写入的失败有保护：[阅读器模板](../../tools/phoenix-diagram/assets/template.html) 中 `localStorage` 与 8 处 `history.replaceState` 都在 `try/catch` 内；[阅读器运行约定](../../skills/phoenix-diagram/references/viewer-runtime.md) 规定主体是静态 SVG、文字在 runtime 失败时仍可读，并有 `@media print` 规则。
- **[已核实·缺口]** 仓内两份现有 elintp 页面（`docs/ai-native-sdlc.summary.html`、`docs/ai-native-sdlc.summary.2026-11-10.html`）都没有 `iframe`，没有「阅读器放进 `iframe srcdoc` 后能正常显示与交互」的实例 → 由 V-2 用真实阅读器 HTML 验证 → 漏做则 WORK-F5/AC-1 无证据。
- **[已核实·足够]** 本机安装缓存没有 [ENGINE.json](../../skills/phoenix-diagram/ENGINE.json) 指定的引擎目录；已从本仓固定工具链离线构建匹配引擎，完成 workflow 的 finalize、四张实际截图检查与 review-record，交付状态为 complete → A-1 已解除。
- **[已核实·缺口]** 源码检出下 phoenix-ui 的启动器不可用（`skills/phoenix-ui/scripts/ENGINE.json` 不存在），仓根也没有 `PRODUCT.md`、`DESIGN.md` → 模板设计按 [phoenix-ui](../../skills/phoenix-ui/SKILL.md)「Launcher unavailable」分支进行，设计依据取 §6.1、§6.4 → 不影响交付，见 §6.4。
- **[已核实·足够]** 仓内没有 CRD 实例文档；V-2 的计划输入采用本计划已完成 M1 与含真实截图的 M4 记录，按实际进度 partial。V-2 的 CRD 输入用 [crd skill](../../skills/crd/SKILL.md) 的结构写一份有代表性的稿件。

### 1.2 拓扑与文件清单

```text
作者材料 ──> 宿主 Agent（判型、写内容稿）──> render.mjs ──> 单文件 HTML ──> 读者浏览器
                    │                          ▲
                    │ 可用时调用               │ 读取阅读器 HTML / 自绘 SVG / 截图
                    └──> phoenix-diagram ──────┘
```

| 路径 | 改动 | 说明 |
| --- | --- | --- |
| `skills/elintp/SKILL.md` | 修改 | §6.3 |
| `skills/elintp/references/draft-format.md` ★ | 新增 | 内容稿格式参考，写内容稿前读；含四种模板各一份最小示例 |
| `skills/elintp/scripts/render.mjs` ★ | 新增 | 命令入口：读参数、调用解析与拼装、写文件、退出码 |
| `skills/elintp/scripts/draft.mjs` ★ | 新增 | 内容稿解析与格式校验，产出块结构与错误列表 |
| `skills/elintp/scripts/templates.mjs` ★ | 新增 | 四种模板的 slot 表与三套界面文字 |
| `skills/elintp/assets/page.html` ★ | 新增 | 页面外壳，含占位符 |
| `skills/elintp/assets/page.css` ★ | 新增 | 全部样式：亮暗两套变量、组件、打印 |
| `skills/elintp/assets/page.js` ★ | 新增 | 亮暗切换、大纲当前小节、表头释义折叠 |
| `test/elintp/render.test.js` ★ | 新增 | V-3 的局部测试 |

当前无预定修改的邻接面：`skills/summarize/`（调用方式不变）、`skills/phoenix-diagram/`、`skills/phoenix-ui/`、`README.md`、`bin/`、`scripts/`、`package.json`。若真实嵌入验收定位到 Phoenix Diagram 的直接依赖缺陷，按 §4 纳入对应源码与最小检查，不以本清单限制已授权修复。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `render.mjs` | 宿主 Agent（命令行） | §5.1 的命令、输出行与退出码；有格式错误时不写输出文件 | 无 | 资源定位、占位符填充、一次写入 | 进程内（Node 标准库） | 子进程运行，断言 stdout、stderr、退出码与输出文件 |
| `draft.mjs` | `render.mjs` | `parseDraft(text, { cwd }) → { meta, lead, sections, errors, warnings }`；`errors` 非空时其余字段不用于输出 | 无 | 受限 Markdown 语法、slot 顺序、文件读取、SVG 白名单 | 进程内 | 经 `render.mjs` 观察 |
| `templates.mjs` | `draft.mjs`、`render.mjs` | 导出 `TEMPLATES`（§5.2 的 slot 表）与 `UI`（§6.2 的界面文字）；纯数据 | 无 | 无 | 进程内 | 经页面观察 |
| `page.js` | 读者浏览器 | §6.2 的行为；脚本不运行时页面内容完整 | 无 | 存储失败的回退、当前小节判定 | 浏览器 | 真实浏览器走查 |

三个脚本模块只有 `render.mjs` 一个入口，分文件是为了让语法、模板数据与拼装各自可读，不是为了替换；不设适配层。

## 3. 数据模型与迁移

没有持久化数据。内容稿是 Agent 放在临时目录或项目约定工作目录的文件，不进文档目录；页面是一次性生成的产物。旧的手写页面不迁移，保持原样可读。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| Node 运行时 | 已核实足够 | 仓库要求 Node 18+（[package.json](../../package.json) `engines`）；本机 v25.2.1 | 脚本只用标准库（`node:fs`、`node:path`、`node:url`） | `node` 不存在、版本低于 18，或脚本以 0、2 之外的退出码结束：elintp 手写降级并在交付说明写明原因 |
| phoenix-diagram 阅读器 HTML | 真实产物已就绪，嵌入待验 | 本仓源码引擎 finalize 与 review-record 交付 complete；阅读器主体为静态 SVG | 由本仓构建真实引擎，无需用户另交 release-dir。嵌入失败先对照独立阅读器定位；本方拼装与尺寸问题在 elintp 修，阅读器直接依赖缺陷在 Phoenix Diagram 修并记录受影响检查 | 阅读器文件不存在：格式错误，退出码 2。运行时不可交付：Agent 用 `svg:` 自绘并说明（WORK-F5/AC-4） |
| summarize | 已核实足够 | 它只说「调用 elintp skill，把素材交给它」并列出三条硬要求（[summarize](../../skills/summarize/SKILL.md) §3） | 零改动。模板用系统字体栈、零外部资源，天然满足前两条；「图用内联 SVG」见 §0.5 | 不适用 |
| phoenix-ui（设计期） | 已核实缺口 | 启动器在源码检出下不可用；skill 规定此时直接读已有上下文并继续（[phoenix-ui](../../skills/phoenix-ui/SKILL.md) Setup） | 只在 M4 作为设计方法使用；elintp 运行时不依赖它 | 启动器不可用不阻塞设计 |
| 真实浏览器 | 已核实足够 | 本机有 `/Applications/Google Chrome.app` | V-2、V-5 的走查由 Agent 操作 | 没有浏览器或图像能力时，对应检查记为未运行，不记通过 |

## 5. 核心机制

### 5.1 命令接口

```sh
node <skill 目录>/scripts/render.mjs <内容稿.md> <输出.html>
```

| 结果 | 退出码 | 输出 |
| --- | --- | --- |
| 成功 | 0 | stdout 一行 `PAGE: <输出文件的绝对路径>`；有图片缺失时 stderr 另有 `WARN L<行号>: …` |
| 内容稿有格式错误 | 2 | stderr 每处错误一行 `L<行号>: <问题>。改法：<怎么改>`，按行号排序，一次列全；不创建、不修改输出文件 |
| 参数不对、内容稿读不到、输出写不了 | 1 | stderr 一行说明，参数不对时附用法 |

- 输出文件的父目录不存在时创建。输出路径已有文件时，成功才整体覆盖。
- 同一份内容稿渲染两次，输出逐字节相同：页面里不写时间戳、随机标识或本机绝对路径（图片缺失提示里的路径按内容稿原样显示）。
- 相对路径（图片、阅读器、SVG）相对运行命令时的当前目录解析。

### 5.2 frontmatter 与模板

内容稿以 `---` 包起来的 frontmatter 开头，每行一个 `键: 值`。

| 键 | 必填 | 取值与用途 |
| --- | --- | --- |
| `template` | 是 | `general`、`prd`、`crd`、`plan` |
| `title` | 是 | 页面标题，也是 `<title>` |
| `lang` | 是 | 内容语言的标签，如 `zh-CN`、`zh-TW`、`en`、`ja`；原样写入 `<html lang>` |
| `subtitle` | 否 | 标题下的一行说明 |
| `source` | 否，可重复 | 文档模式的出处与读到的版本；每个值在页面末尾的出处区占一行 |
| `progress` | 仅 `plan` 必填 | `todo`（未完成）、`partial`（做了一半）、`done`（已完成）；其他模板出现此键报错 |

frontmatter 之后、第一个 `##` 之前是引言，只能是段落，至少一段，渲染为结论块。

小节标题写作 slot 名、竖线、标题，例如 `## gain | 审阅意见可以直接提交`。slot 决定小节在页面上方显示的固定标签；标题由模型写，是这一节的结论。slot 必须按下表顺序出现，必填的不能缺；除 `more` 外每个 slot 至多出现一次。

| 模板 | slot 顺序（★ 必填） |
| --- | --- |
| `general` | `why`★ → `scale` → `risks` → `more`（可重复） |
| `prd` | `gain`★ → `delta`★ → `rollout`★ → `more`（可重复） |
| `crd` | `ask`★ → `stance`★ → `agreed`★ → `open`★ → `more`（可重复） |
| `plan`，`progress: todo` | `build`★ → `route`★ → `more`（可重复） |
| `plan`，`progress: done` | `built`★ → `verify`★ → `gaps`★ → `records` → `more`（可重复） |
| `plan`，`progress: partial` | `built`★ → `verify`★ → `build`★ → `route`★ → `gaps` → `records` → `more`（可重复） |

`more` 用来放重心之外但不能丢的内容（原文的其余警告、数字与前提），不显示固定标签。

### 5.3 块与行内语法

小节内只支持下列写法；不属于下列写法的行按段落处理，文字一律转义后输出。

| 写法 | 渲染 | 校验 |
| --- | --- | --- |
| 连续的非空行 | 段落 | — |
| `### 小标题` | 小节内的三级标题 | 一级标题、四级及更深的标题报错 |
| `- 项`，子项缩进两个空格 | 无序列表，至多两层 | 第三层报错，改法：拆成两个列表或改用表格 |
| `1. 项` | 步骤：带序号的步骤组件 | 序号必须从 1 连续递增 |
| 管道表格，第二行是 `---` 分隔行 | 表格，外包可横向滚动的区域 | 每行列数与表头一致 |
| 表头单元格 `列名 :: 释义` | 列名加表头释义 | 释义不能为空 |
| 单元格以 `[ok] `、`[no] `、`[warn] ` 开头 | 状态标记：符号 ✓、✗、! 加后面的文字 | 标记后必须有文字 |
| 三个反引号围起的代码块，可带语言名 | 等宽代码块，内容逐字输出 | 必须闭合 |
| 单独一行的 `![说明](本地路径)` | 图片，说明同时是图注与替代文字 | §5.4 |
| `:::conclusion [标题]` … `:::` | 结论块 | 内部只能有段落与列表 |
| `:::reminder [标题]` … `:::` | 提醒块 | 同上 |
| `:::metrics` … `:::` | 指标块，每行写数值、名称、含义三段，用竖线分隔 | 每行恰好三段且都不为空 |
| `:::figure` 后接图的标题 … `:::` | 图，标题必填 | §5.4 |
| 行首 `>` | 不支持 | 报错，改法：改用 `:::reminder` 或普通段落 |

行内只识别三种标记：`` `代码` ``、`**强调**`、`[文字](链接)`。链接只允许 `http://`、`https://`、`mailto:` 与以 `#` 开头的页内锚点，其他协议报错。内容稿里的 `<script>`、`<img onerror=…>` 这类文字不报错，按原样显示为文字。

### 5.4 图片与图

- **图片**：路径是本地文件，扩展名限 `png`、`jpg`、`jpeg`、`gif`、`webp`、`avif`、`svg`，读入后以 data URI 放进 `<img>`。`http://`、`https://` 开头的路径是格式错误（改法：先存成本地文件）。文件不存在或读不了不算格式错误：页面在该位置显示「图片缺失：」加内容稿里写的路径，并保留图注，stderr 输出一行 `WARN`，退出码仍为 0。
- **`:::figure` 的内容**：恰好一行来源，后面可跟若干行图注文字。
  - `reader: <路径.html>`：phoenix-diagram 交付的阅读器 HTML。读入全文，转义 `&` 与 `"` 后写进带 `srcdoc`、`loading="lazy"` 的 `iframe`，`title` 取图的标题。文件不存在或读不了是格式错误。
  - `svg: <路径.svg>`：Agent 自绘的 SVG 文件。通过白名单校验后去掉 XML 声明与注释，根元素加 `role="img"`，`aria-label` 取图的标题，内联进页面。
- **SVG 白名单**：逐个标签检查，任何一处不符就是格式错误，报出 SVG 文件里的行号与标签，不做静默清洗。
  - 允许的元素：`svg`、`g`、`defs`、`title`、`desc`、`path`、`rect`、`circle`、`ellipse`、`line`、`polyline`、`polygon`、`text`、`tspan`、`marker`、`linearGradient`、`radialGradient`、`stop`、`clipPath`、`use`。
  - 禁止的属性：以 `on` 开头的属性、`style`。
  - 资源引用：`href` 与 `xlink:href` 的值必须以 `#` 开头；`fill`、`stroke`、`marker-*`、`clip-path`、`mask`、`filter` 等 URL 型属性中的 `url(...)` 只能写 `url(#...)`，外部 URL 与其他外部资源引用按格式错误处理。
  - 禁止 `<!DOCTYPE`、`<![CDATA[`。根元素必须有 `viewBox`。
  - 触发与后果：自绘 SVG 是页面里唯一不经转义直接进入 DOM 的作者内容，原文中的注入文字可能被带进去；页面常作为附件转发，脚本在收件人的浏览器里执行不可接受。
- **自绘 SVG 跟随亮暗**：线条与文字用 `currentColor`，或用模板提供的类名 `f-fg`、`f-muted`、`f-accent`、`f-surface`、`f-ok`、`f-warn`、`f-no`（填充）与对应的 `s-` 前缀（描边）。这是写法约定，写在格式参考里；脚本不检查颜色。

### 5.5 格式校验清单

脚本报告的错误只有下列几类，每条都带行号和改法；红线句、套话、数字是否与原文一致不在其中（C-4）。

| 类别 | 触发 |
| --- | --- |
| frontmatter | 缺少开头的 `---` 块；必填键缺失；出现未知键；`template` 或 `progress` 取值不在表内；非 `plan` 模板出现 `progress` |
| 引言 | 第一个 `##` 之前没有段落，或出现段落以外的块 |
| 小节 | 标题不是「slot 名、竖线、标题」的形式；slot 不属于该模板；顺序不对；必填 slot 缺失；非 `more` 的 slot 重复；小节没有内容 |
| 块 | §5.3「校验」列的各项；未知的 `:::` 容器名；容器或代码块没有闭合 |
| 行内 | 链接协议不在允许范围 |
| 图片与图 | 远程图片路径；不支持的图片扩展名；`:::figure` 缺标题、来源行不是恰好一行；阅读器或 SVG 文件读不了；SVG 白名单不通过 |

### 5.6 拼装

1. 读内容稿，`parseDraft` 得到结构、错误与警告；有错误就输出并以 2 退出。
2. 按 `lang` 选界面文字（§6.2），按模板与 `progress` 取 slot 标签。
3. 读 `assets/page.html`，把 `page.css`、`page.js` 与渲染出的页头、结论块、大纲、小节、出处区填进占位符。
4. 把完整字符串一次写入输出路径，打印 `PAGE:` 行与警告。

## 6. 页面与 skill 增量

### 6.1 页面结构

```html
<html lang="<内容稿的 lang>">
<head>
  meta charset、viewport、<title>
  <style>page.css</style>
  <script>读取保存的亮暗选择并设置 data-theme；给 <html> 加 class="js"</script>
</head>
<body class="tpl-<template>">
  <a class="skip-link" href="#main">跳到正文</a>
  <header class="page-head">模板标识（plan 另有进度标记）、<h1>、副标题、亮暗切换按钮</header>
  <div class="lead">结论块</div>
  <nav class="outline" aria-label="大纲">小节链接的有序列表</nav>   <!-- 仅当小节数 ≥ 3 -->
  <main id="main">
    <section id="s1" data-slot="gain"><p class="slot-label">会多出什么</p><h2>标题</h2>…</section>
    …
  </main>
  <footer class="sources">出处</footer>   <!-- 仅当有 source -->
  <script>page.js</script>
</body>
```

- 样式全部在 `page.css`，字体用系统字体栈，不嵌入字体文件，不引用任何外部地址（C-3）。
- 颜色用 CSS 变量定义亮、暗两套：没有 `data-theme` 时由 `prefers-color-scheme` 决定，`data-theme="light"` 或 `"dark"` 时覆盖系统偏好。
- 表格外包 `<div class="table-scroll" role="region" tabindex="0" aria-label="…">`，宽表在区域内横向滚动；`iframe` 与图片宽度不超过容器。页面本身在 360 CSS px 与 1440×900 下都不出现水平滚动条。
- 阅读器 `iframe` 宽 100%，高度 `min(82vh, 760px)`；窄屏下由阅读器自己的窄屏布局处理宽内容。
- 模板标识与进度标记都是「符号加文字」：进度标记写作「计划记录：未完成 / 做了一半 / 已完成」，与「和原文一样真」的转述规则一致。

### 6.2 页面行为

| 行为 | 有脚本时 | 脚本不运行或打印时 |
| --- | --- | --- |
| 亮暗 | 页头按钮切换 `data-theme`，选择存进 `localStorage` 的 `elintp-theme`；读写都包在 `try/catch` 里，存不了时仍可切换 | 按钮默认带 `hidden`，由脚本显示；页面跟随系统偏好。打印固定用亮色 |
| 大纲 | 链接跳到小节；`IntersectionObserver` 给当前小节的链接加 `aria-current="true"`，样式上除颜色外另有粗细或标记区分 | 链接照常可跳；打印时隐藏大纲 |
| 表头释义 | `.js` 下默认折叠在表格下方的「表头释义」按钮后（`aria-expanded`），按钮可用键盘操作 | 释义列表直接显示在表格下方 |
| 键盘 | 跳到正文链接、亮暗按钮、大纲链接、释义按钮、可滚动表格区域都能用 Tab 到达并操作，焦点样式可见 | — |

界面文字随 `lang` 选择：小写后为 `zh`、`zh-cn`、`zh-sg` 或以 `zh-hans` 开头的用简体中文；`zh-tw`、`zh-hk`、`zh-mo` 或以 `zh-hant` 开头的用繁体中文；以 `en` 开头的用英文；其余语言用英文界面，`<html lang>` 仍是内容稿给出的标签。

| 键 | 简体中文 | 繁体中文 | 英文 |
| --- | --- | --- | --- |
| 模板 `general` / `prd` / `crd` / `plan` | 技术说明 / 产品需求 / 跨项目需求 / 开发计划 | 技術說明 / 產品需求 / 跨專案需求 / 開發計畫 | Explainer / Product requirements / Cross-project request / Development plan |
| 进度 `todo` / `partial` / `done` | 计划记录：未完成 / 做了一半 / 已完成 | 計畫記錄：未完成 / 做了一半 / 已完成 | Plan record: not done / partly done / done |
| `why` / `scale` / `risks` | 为什么成立 / 量级 / 风险与前提 | 為什麼成立 / 量級 / 風險與前提 | Why it holds / Scale / Risks and conditions |
| `gain` / `delta` / `rollout` | 会多出什么 / 和现在差在哪 / 建议怎么实施 | 會多出什麼 / 和現在差在哪 / 建議怎麼實施 | What is new / What changes / How to roll out |
| `ask` / `stance` / `agreed` / `open` | 谁向谁要什么 / 对方的立场与答复 / 已经谈定 / 还没谈定与下一步 | 誰向誰要什麼 / 對方的立場與答覆 / 已經談定 / 還沒談定與下一步 | Who asks whom for what / The other side's position / Agreed / Still open and next step |
| `build` / `route` | 要开发什么 / 技术路线与代价 | 要開發什麼 / 技術路線與代價 | What will be built / Approach and cost |
| `built` / `verify` / `gaps` / `records` | 做出了什么 / 怎么自己验证 / 遗漏与后续 / 过程记录 | 做出了什麼 / 怎麼自己驗證 / 遺漏與後續 / 過程記錄 | What was built / How to check it yourself / Gaps and follow-ups / Process records |
| 控件 | 大纲 / 切换亮暗 / 表头释义 / 出处 / 跳到正文 / 图片缺失 | 大綱 / 切換亮暗 / 表頭釋義 / 出處 / 跳到正文 / 圖片缺失 | Outline / Switch theme / Column notes / Sources / Skip to content / Image missing |

### 6.3 SKILL.md 增量

frontmatter、「两种输入」「语言选择」「红线」「转述硬规则」「常见坑」的原文不动。其余按下表改，§0.6 是核对依据。

| 小节 | 改动 |
| --- | --- |
| 写作准则 | 末尾补三条：同一事实只说一次，图或表已经表达的不在正文重复；列表列全，不以「等」收尾，列不全就写完整清单在哪；不描述自己怎么做的，只给结果与依据 |
| 文档模式第一步 | 新增「### CRD」：读者要能回答谁向谁要什么、为什么要，对方的立场与答复，已经谈定的约定，还没谈定的问题与下一步由谁做。「其他类型」保留，并写明它们用通用模板 |
| 新增「选模板，写内容稿」 | 判型结果到模板的对应：PRD → `prd`；CRD 及其往返答复 → `crd`；开发计划 → `plan`，并按原文记录填 `progress`；主题与其余文档 → `general`。写内容稿前读 [内容稿格式参考](../../skills/elintp/references/draft-format.md)；内容稿放临时目录或项目约定的工作目录，不放文档目录；内容稿里不写样式、版式与图形坐标 |
| 产出形态 | 改为：运行 §5.1 的命令生成页面；退出码 2 时按每行的位置与改法修内容稿再跑，不改脚本或模板绕过；输出路径与命名规则、Artifact 发布条件、出处写在末尾这三段原文保留。原「这份文档要靠看」的要求保留，改为说明它们由哪种内容稿写法表达（指标块、表头释义、结论块、提醒块、状态标记、步骤） |
| 新增「脚本不可用时」 | `node` 不存在、版本低于 18，或脚本以 0、2 之外的退出码结束时，手写单文件 HTML，外观要求沿用原「产出形态」的各条；交付说明写明未使用模板及原因。不为此安装 Node |
| 出图 | 对照表与「不新增安装检测脚本、不自动安装」原文不动。「合并后仍是一个离线 HTML」改为：阅读器 HTML 用 `:::figure` 的 `reader:` 引用，由脚本嵌入；没有安装 phoenix-diagram 或按其流程无法交付时，把自绘图存成 SVG 文件用 `svg:` 引用；截图用 `![说明](本地路径)`，只用已存在的本地文件，不生成图片。「原图的校验结果不能代替最终页面检查」保留 |
| 交付前自检 | 开头加一句：自检对象是生成后的页面，不是内容稿；发现问题改内容稿并重新生成。九项的编号与含义不变。末尾新增交付说明的固定内容：页面路径；所用模板，或未使用模板及原因；实际做过的自检项；没做或做不了的自检项；仍未解决的问题；图由 phoenix-diagram 交付还是自绘及原因；脚本输出的 `WARN` |

`references/draft-format.md` 的内容：§5.2 至 §5.4 的写法（面向写内容稿的模型重写，不照抄本计划），四种模板各一份最小示例，自绘 SVG 的写法约定，常见报错与改法。

### 6.4 模板的视觉设计

M4 用 phoenix-ui 的设计流程完成 `page.css`，页面属于它的 Read 模式（读者是来理解一件事的）。启动器在源码检出下不可用，按其 skill 的「Launcher unavailable」分支：先单独说明上下文加载未运行，再读 `skills/phoenix-ui/reference/new-work.md` 与 `skills/phoenix-ui/reference/craft-floor.md` 后动手。仓内没有 `PRODUCT.md` 与 `DESIGN.md`，设计依据是下面这些已定事实，不另造品牌设定：

- 第一屏是结论块；指标块与大图优先于长段落（PRD §2.4）。
- 只有亮、暗两套外观，四种模板的组件外观完全相同（D3）。
- 系统字体栈，零外部资源（C-3）。
- 状态与类型的区分不只靠颜色（WORK-N1/AC-2、WORK-F1/AC-6）。
- 360 CSS px 可读，宽内容在自身区域滚动（WORK-N1/AC-1）。

设计只改 `assets/page.css`，必要时调整 `page.html` 与渲染输出里的类名；§6.1 的结构与 §6.2 的行为是设计的固定前提。设计用的样例页面由 V-2 的四份内容稿渲染得到。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N1 | PRD §3 WORK-N1；360 CSS px 是 PRD 提出的门槛，未经实测 | AC-1 至 AC-4 | 不达标即 V-2 不通过，修样式或结构 | §6.1 的滚动区域与宽度约束；§6.2 的渐进增强与键盘可达；三套界面文字 | V-2 |
| WORK-N2 | PRD §3 WORK-N2 | AC-1、AC-2、AC-4 | 格式错误不出页；图片缺失在页面与 `WARN` 中标出 | 模板零外部地址、图片与图全部内嵌；文字转义、不支持原样 HTML、SVG 白名单（D9） | V-2（零网络、文字不执行、缺图标注）、V-3（转义与白名单） |
| WORK-N3 | PRD §3 WORK-N3 | AC-1、AC-2 | 出现表外的 skill 行为变更时先更新 §0.6 | §6.3 逐节保留原文；frontmatter 不改；summarize 不改 | V-4、V-5 |

可观测性没有新增入口：脚本的 `PAGE:`、`L<行号>:`、`WARN` 行就是全部运行反馈。

## 8. 失败模式、发布与回滚

本期不引入需要专门恢复机制的运行期失败：脚本是一次性的本地命令，没有并发写、网络调用或持久状态；失败时退出码与 stderr 说明原因，输出文件未被触碰（D10），由 Agent 修内容稿后重跑或走手写降级。

- 发布顺序：skill 文件随 npm 包的 `skills/` 目录分发，没有迁移与开关；脚本、资源、参考与 SKILL.md 在同一次提交内一致即可。
- 回滚：回退提交即恢复手写流程；已生成的页面是独立文件，不受影响。

## 9. 验证

### 9.1 行为与接口验证

批内遇到具体问题时的最小反馈：`node skills/elintp/scripts/render.mjs <一份内容稿> <临时输出>` 看输出，或 `node --test --test-name-pattern '<用例名>' test/elintp/render.test.js` 跑单个用例。浏览器走查只在批末做。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | 作者把一个真实主题（取 [交付合同](../../skills/phoenix-diagram/references/delivery-contract.md) 的「三门」）写成通用模板内容稿并生成页面，中途写错两处再改正。WORK-F1/AC-1（通用模板部分）、AC-4。结果：首次生成得到 `PAGE:` 行，浏览器打开第一屏是结论；把一个 slot 改成不存在的名字、把一行表格多写一列后，退出码 2，stderr 恰好两行且各带行号与改法，输出文件的 SHA-256 与改错前相同；改正后重新生成成功 | `node skills/elintp/scripts/render.mjs <内容稿> <输出>`；`shasum -a 256 <输出>`；Chrome 打开输出文件 | M1 交付齐备后 |
| V-2 | 场景 E2E | 读者在各种阅读条件下读四份由真实文档写成的页面。输入与走查步骤见 §9.4。WORK-F1/AC-2、AC-3、AC-6；WORK-F3/AC-1 至 AC-3；WORK-F5/AC-1、AC-5；WORK-N1/AC-1 至 AC-4；WORK-N2/AC-1、AC-2、AC-4 | §9.4 的命令与 Chrome；WORK-F5/AC-1 一项需要 A-1 就绪 | M2–M4 交付齐备后 |
| V-3 | 局部 | 准入依据：语法与 slot 校验是分支多的纯逻辑；转义与 SVG 白名单是后果不可接受且 E2E 难以逐一触发的路径。断言全部通过命令行观察：§5.5 各类别各取一个代表输入，得到对应行号的错误且退出码 2；含 `<script>`、`onerror` 的文字在输出中是转义后的文字；含 `<script>`、`onload`、`<foreignObject>`、外部 `href`、外部 `url(...)` 资源引用、`style` 的 SVG 各被拒绝；有错误时已存在的输出文件不变；同一内容稿两次输出逐字节相同。WORK-F1/AC-4；WORK-N2/AC-2 | `node --test test/elintp/render.test.js` | M2–M4 交付齐备后 |
| V-4 | 场景 E2E | 作者在全新上下文里按改后的 skill 调用 elintp，完成六个任务，见 §9.4。WORK-F1/AC-1、AC-3、AC-5；WORK-F2/AC-1 至 AC-4；WORK-F5/AC-4；WORK-N3/AC-1、AC-2 | 每个任务由一个全新上下文的 Agent 执行，只给用户会说的那句话与材料路径，让它读 `skills/elintp/SKILL.md`；主 Agent 对照验收行读页面与交付说明 | M5 交付齐备后 |
| V-5 | 真实使用验收（阶段收口） | 从干净的用户状态连续完成两个完整任务并亲自读完页面；另跑本期全部回归。步骤见 §9.2、§9.4。覆盖本期全部需求 | 临时项目目录里的 skill 副本；Chrome；§9.2 的命令 | M6，全部代码与文案改完后 |

### 9.2 集成与回归

阶段收口时各跑一次：

```sh
node --test test/elintp/render.test.js
uv run --no-project --with pyyaml python - <<'PY'
from pathlib import Path
import runpy

checks = runpy.run_path("test/skills/test_skill_metadata.py")
path = Path("skills/elintp/SKILL.md")
match = checks["FRONTMATTER"].match(path.read_text(encoding="utf-8"))
assert match, "SKILL.md 须以 YAML frontmatter 开头"
data = checks["yaml"].safe_load(match.group(1))
assert isinstance(data, dict)
assert data.get("name") == "elintp"
assert isinstance(data.get("description"), str) and data["description"].strip()
assert not set(data) - checks["FRONTMATTER_KEYS"], "frontmatter 含不允许的字段"
PY
git diff --stat 0b35470b -- skills/summarize skills/phoenix-diagram skills/phoenix-ui README.md
git status --short docs/*.html
npm pack --dry-run 2>&1 | grep 'skills/elintp/'
```

预期：渲染测试与 elintp 单文件元数据检查通过；第三条核对邻接面改动；无直接依赖修复时应为空，有修复则逐项对应 §4 归因与最小检查；第四条不出现本期造成的旧页面改动（WORK-N3/AC-2 的「此前生成的页面仍可打开」）；第五条列出 `SKILL.md`、`references/`、`scripts/`、`assets/` 下的全部文件。不跑全仓元数据测试或 `npm test`；未改 Phoenix 代码时不跑其运行时检查，直接依赖修复仅运行能发现对应错误的最小检查。

### 9.4 真实环境与页面走查

**V-2 的输入**（内容稿放临时目录，由实施者按本计划 §5 的制稿规则写）：

| 稿件 | 来源 | 模板与语言 | 特意包含 |
| --- | --- | --- | --- |
| A | [本期 PRD](../PRD-ELINTP-VISUALS.md) | `prd`，`zh-CN` | 带状态标记与表头释义的表格、指标块（参考工具自测的 4,893 与 612 token）、提醒块、步骤、一张已有截图（`docs/diagrams/agi-mode-evolution-loop.visual-check.1440x900.light.png`）、一个 `reader:` 图（PRD §2.3 主路径的 workflow，需要 A-1） |
| B | [本计划](ELINTP-VISUALS-P1.md) 与 M1、M4 记录；M1 已完成，整份计划部分完成 | `plan`，`progress` 按其顶部记录，`zh-CN` | `records` 小节与 M4 中真实 Chrome 截图 |
| C | 按 [crd skill](../../skills/crd/SKILL.md) 的结构写的一份有请求、答复、约定与未决项的 CRD | `crd`，`zh-TW` | 四个必填小节 |
| D | 一个英文主题 | `general`，`en`；另存一份只把 `lang` 改成 `ja` | 只有 `why` 一个小节；一个 `svg:` 自绘图；一个路径不存在的图片；一段含 `<script>alert(1)</script>` 与 `<img src=x onerror=alert(1)>` 的文字 |

**V-2 的走查**（Agent 在 Chrome 中逐页操作并亲眼看页面，不只看命令输出）：

1. 每页第一屏先出现结论；小节顺序与 §5.2 一致；A 页的各组件都显示，状态标记有符号也有文字（WORK-F1/AC-3、AC-6）。
2. 再写一份内容不同的 `prd` 稿件，与 A 的输出比较：两个文件里 `<style>` 与两段 `<script>` 的内容逐字节相同，并排看版式与配色一致（WORK-F1/AC-2）。
3. 系统偏好分别设为亮与暗打开页面，外观跟随；点切换按钮后正文、表格、指标块与自绘图的文字和线条都看得清；刷新后选择保留（WORK-F3/AC-1、AC-3）。
4. A、B、C 有大纲，点任一项跳到对应小节，滚动时当前项有标记；D 没有大纲（WORK-F3/AC-2）。
5. 窗口宽 360 CSS px 与 1440×900 各看一遍：页面没有水平滚动条，宽表格在自己的区域里滚动，文字不被裁剪（WORK-N1/AC-1）。
6. 只用键盘完成切换亮暗、经大纲跳转、展开表头释义、横向滚动宽表格（WORK-N1/AC-2）。
7. 禁用脚本打开，以及打印成 PDF：正文、表格、图与表头释义都在，控件不遮挡内容。可用 `"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --print-to-pdf=<输出.pdf> file://<页面>`；禁用脚本用浏览器设置或 DevTools（WORK-N1/AC-3）。
8. C 的界面文字是繁体，D 是英文，`ja` 版是英文界面且 `<html lang="ja">`（WORK-N1/AC-4）。
9. 打开 DevTools 的 Network 面板后加载 A 与 D：除页面自身外没有任何请求（WORK-N2/AC-1）。
10. D 页里那段文字按原样显示，没有弹窗，DOM 里没有多出来的 `script` 或 `img` 元素；缺失的图片位置显示「Image missing」与路径，命令有 `WARN` 行（WORK-N2/AC-2、AC-4）。
11. A 页的阅读器在 `iframe` 里完整显示图、标签与图例，能平移缩放与切换主题；360 CSS px 下宽内容在 `iframe` 区域内滚动；截图完整显示（WORK-F5/AC-1、AC-5）。A-1 未就绪时本项记为环境缺失，不记通过。

第 11 项同时回答一个只能在真实浏览器里确认的问题：阅读器放进 `iframe srcdoc` 后是否正常。判据是上面的显示与交互。结果正常则维持 D5；阅读器出现脚本错误或交互失效时，记录控制台报错并停下，先打开同一 HTML 的独立阅读器与嵌入页对照，检查本期的 `srcdoc` 拼装、转义与 `iframe` 尺寸。确认属于嵌入实现或阅读器直接依赖的缺陷时，在对应源码修复并复验本项；同仓 Phoenix Diagram 不作为需用户另行交付的外部系统。页面保持 `iframe` 嵌入，修复涉及契约变化时同步本计划相关正文。

**V-4 的六个任务**（每个任务一个全新上下文的 Agent；提示里只有用户会说的话，不提示模板、格式或本计划）：

| # | 用户说的话与材料 | 判据 |
| --- | --- | --- |
| 1 | `/elintp docs/PRD-ELINTP-VISUALS.md` | 用了 `prd` 模板；第一句是主题本身的事实，没有描述读者的句子；抽查原文五条警告或数字都在且一致；交付说明含路径、模板、做过与没做的自检（WORK-F1/AC-1、AC-3；WORK-F2/AC-1 至 AC-4） |
| 2 | `/elintp docs/plan/ELINTP-VISUALS-P1.md`（此时本计划做了一半） | 用了 `plan` 模板且 `progress` 与计划顶部记录一致；未在本次会话验证的完成状态写作「计划记录为…」（WORK-F1/AC-3；WORK-F2/AC-3） |
| 3 | `/elintp 讲讲 phoenix-diagram 的 finalize 为什么分三门` | 用了通用模板；没有主动触发以外的行为变化；默认输出位置与命名符合原规则（WORK-F1/AC-1；WORK-N3/AC-1、AC-2） |
| 4 | 同任务 3，但运行环境的 `PATH` 里没有 `node` | 交付手写的单文件 HTML，交付说明写明未使用模板及原因（WORK-F1/AC-5） |
| 5 | `/elintp` 一份需要流程图的文档，环境里 phoenix-diagram 不可交付（A-1 未就绪时即当前环境） | 页面用自绘内联图完成；交付说明写明未使用 phoenix-diagram 的原因，没有声称其校验或审核通过（WORK-F5/AC-4） |
| 6 | 按 [summarize](../../skills/summarize/SKILL.md) §3 的三条硬要求原话调用 elintp，材料同任务 1；只做到生成，不调用 SPMS | 不改调用方式就得到模板化页面；页面是单文件，没有任何外部地址的引用（WORK-N3/AC-2；C-3） |

这些是角色模拟，结果按模拟记录；某个任务不达标时改 skill 文案或参考，只重跑不达标的任务。

**V-5 的真实使用验收**：在系统临时目录建一个空项目，把 `skills/elintp/` 整个复制到其中的 `.claude/skills/elintp/`（证明脚本、资源与参考离开本仓后仍自洽）。Agent 以文档作者身份，只凭 SKILL.md 与格式参考，连续完成两个任务：把一份真实的技术方案或复盘文档讲成页面；把一个主题讲成页面并在看完后要求「第二节没看懂，重讲」。每个页面都在 Chrome 里从头读到尾，切一次亮暗，走一遍大纲。发现的用户可见问题修复后重验该任务，或列给用户确认延期。

### 9.5 静态校验

仓库没有针对 `skills/` 下脚本的 lint 或类型检查入口，本期不新增。frontmatter 由 §9.2 的 elintp 单文件检查覆盖，允许字段复用 `test/skills/test_skill_metadata.py`；frontmatter 未改的 diff 核对按 §0.6 C7 执行。

## 10. 里程碑与验收安排

M1 单独一批，打通最薄的真实链路。M2、M3、M4 同批：它们在同一组页面上用同一次浏览器走查验证，之间没有风险门。M5 一批，M6 收口。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 最薄链路：通用模板内容稿生成页面 | 无 | ★`scripts/render.mjs`（§5.1、§5.6）、★`scripts/draft.mjs`（frontmatter、引言、小节、段落、列表、表格、代码块、行内标记，§5.2、§5.3、§5.5 的对应部分）、★`scripts/templates.mjs`（四张 slot 表，§5.2）、★`assets/page.html` 与能读的基础版 ★`assets/page.css`。同构实现：[decision-form.mjs](../../skills/spec-human-decision/scripts/decision-form.mjs) 的资源定位、`fill()` 占位符填充、`fail(lines, 2)` 与带前缀的结果行 | V-1；回写「实施进度」 |
| M2 | 全部组件与四种模板 | M1 的命令接口 | `draft.mjs` 补齐 `:::conclusion`、`:::reminder`、`:::metrics`、`:::figure`、状态标记、表头释义、步骤、图片嵌入与缺失标注、SVG 白名单（§5.3、§5.4）；`templates.mjs` 补三套界面文字（§6.2）；渲染输出页头标识、进度标记、出处区（§6.1）；★`test/elintp/render.test.js`，同构实现：[decision-form.test.js](../../test/spec-human-decision/decision-form.test.js) 的子进程断言 | 与 M4 同批，V-2、V-3；回写「实施进度」 |
| M3 | 页面行为与各阅读条件 | M1 的页面结构 | ★`assets/page.js` 与 `page.html` 头部的预置脚本（§6.2）；`page.css` 的亮暗变量、滚动区域、焦点样式、打印规则与无脚本表现（§6.1）。同构实现：[form-core.js](../../skills/spec-human-decision/assets/form-core.js) 作为内联进单文件页面的浏览器脚本。可与 M2 并行：M2 只动 `scripts/` 与测试，M3 只动 `assets/`；`page.css` 由 M3 的执行者单独负责 | 与 M4 同批，V-2；回写「实施进度」 |
| M4 | 模板视觉设计 | M2、M3 的结构与类名稳定 | 按 §6.4 用 phoenix-ui 的设计流程完成 `assets/page.css` 的亮、暗两套外观与全部组件；只在必要时调整类名。样例页面用 §9.4 的四份稿件渲染 | V-2、V-3（本批交付齐备后各运行一次）；回写「实施进度」 |
| M5 | skill 文案与格式参考 | M4 批次通过（命令与语法不再变） | 改动前按 §0.6 核对；`skills/elintp/SKILL.md` 按 §6.3 逐节修改；★`skills/elintp/references/draft-format.md`。同构实现：[spec-human-decision](../../skills/spec-human-decision/SKILL.md) §2 指示运行脚本并按退出码处理的写法，及其 [表单数据格式](../../skills/spec-human-decision/references/form-data.md) 作为按需读取的格式参考。文案起草可在 §5 的语法稳定后与 M2–M4 并行；脚本完成后可在等待 UI 环境时先跑作者演练，M2–M4 退出仍须 V-2 全部完成 | V-4；改完后对照 §0.6 核对 `git diff -- skills/elintp/SKILL.md`；回写「实施进度」 |
| M6 | 阶段收口 | M1–M5 交付齐备 | 不新增功能；修复验收发现的问题。跨期需求 WORK-F5、WORK-N2、WORK-N3 在本期完成后停在已提交 / 可测试，不标记为整条验收通过（PRD §7.3） | V-5 与 §9.2 的回归；回写「实施进度」 |

提交点：M4（M2–M4 同批通过后，连同 M1 一并提交）、M6（全部完成）。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1：匹配引擎与真实阅读器 | 真实产物已就绪；V-2 第 11 项仍需实际嵌入检查 | 本仓源码按固定 mise/mbx 工具链构建，finalize 与 review-record 已完成；证据在 M4 记录 | M2–M4 批末 | 否；已解除 |
| 阅读器在 `iframe srcdoc` 中的真实表现未实测 | 可能暴露本期嵌入实现或上游产物/契约缺陷，须先归因再确定修复去向 | V-2 第 11 项对照独立阅读器与嵌入页；本方问题本期修，确认上游缺口再登记（§9.4） | 同上 | 否 |
| 模型写出受限语法之外的 Markdown | 多一轮「报错、修改、重跑」 | 报错自带改法；V-4 观察实际发生的写法，高频且有真实表达需求的再按 ADR-1 的触发条件扩语法 | M5 | 否 |
| D12 把通用模板的 `scale`、`risks` 定为可省略 | 对 WORK-F1/AC-3「按顺序回答对应问题」的一种读法；另一种读法是全部必填、没有内容时写明没有 | 用户可在评审时改选；改选只影响 `templates.mjs` 的必填标记与格式参考的一句话 | M2 | 否 |
| 每个 `reader:` 图把整份阅读器 HTML 放进页面，前身产物约 0.8MB 一份 | 图多的页面可能超过 summarize 的 2MB 单份上限 | 调用方已有「超了就报错并收敛内容」的规则；静态图在 P3 提供 | — | 否 |

- 最终状态：**Ready**。
- 定级理由：范围、内容稿格式、命令接口、页面结构与验证办法都已确定，没有影响范围、安全、数据或外部契约的未决项。A-1 已通过本仓构建解除；WORK-F5/AC-1 的嵌入表现由 V-2 取证。

## 12. 已知坑与历史教训

- 本仓的 `.claude/skills/elintp` 是指向 `skills/elintp` 的符号链接，演练读到的就是源码；其他安装方式是副本，[AGENTS.md](../../AGENTS.md)「Working in This Repository」要求行为检查指向 `skills/<name>/...`。V-4 让演练 Agent 直接读 `skills/elintp/SKILL.md`，V-5 才用副本。
- [AGENTS.md](../../AGENTS.md) `@automated_test_scope` 不允许为模型输出或静态源文本写自动化测试：`test/elintp/render.test.js` 只测脚本的确定性行为，SKILL.md 的效果靠 V-4 的演练判断，不写成测试代码。
- [run-tests.mjs](../../scripts/run-tests.mjs) 会自动把 `test/elintp/render.test.js` 纳入 `npm test`：这份测试必须不依赖浏览器、网络与 phoenix-diagram 引擎。
- [AGENTS.md](../../AGENTS.md) §6 要求改过 `description` 后重读全文与前 100 字；本期不改它，若实施中发现必须改，先更新 §0.6 的 C7 一行。
- [Skill 编写与修订](../../skills/agi-mode/playbooks/authoring-a-skill.md) 第 6 步：宣称规则生效需要全新上下文按原请求重放。V-4 因此每个任务用一个全新上下文，且提示里不给任何关于改动的线索。
