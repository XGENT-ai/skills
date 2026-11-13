# PHOENIX-UI-EVIDENCE-CRAFT-P4 · XGENT-ai Skills · Phoenix UI 方案对比页与动效、反馈、细节指引（原生 `compare` 命令 · 选择经本机服务回传 · 复述与分发终验）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-12 · `8a49b86f`（main）· clean。
>
> 需求来源：[Phoenix UI 取证与工艺增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.4（WORK-F12 的口径在 v1.5 按 V-5、V-7 的宿主实测改为“发出清单后等用户回复再动手”，用户 2026-11-13 决定）· P4（第 4 期）：WORK-F10/AC-1–AC-6、WORK-F11/AC-1–AC-5、WORK-F12/AC-1–AC-4、WORK-F13/AC-1–AC-3、WORK-F14/AC-1–AC-3、WORK-N2/AC-4（WORK-N2 在本期终验）、WORK-N4/AC-3（WORK-N4 在本期终验），以及每期都要满足的 WORK-N4/AC-2。
>
> 本期交付：**一个随 engine 分发的原生命令和四组设计指引。`phoenix-ui compare` 把已构建的整页方案（本地 HTML、本机地址、截图，可含现状基线）放进一张本机对比页并排展示，可切桌面/手机视口、看 100% 实际尺寸、用键盘切换，用户在页上选定并附说明后，选择经本机服务直接回到主代理；页面关闭而没有选择时命令如实报告，由主代理再问一次。指引四组写进 phoenix-ui skill：动效的编排、过渡、手感与结果提示；把“再高级一点”这类评价先翻译成可观察变量；演示数据、图标体系与三类结构的补验；交付后最多三条带代价的下一步。同时完成 P1–P4 全部新命令、新规则与流程变化的对外复述终验，以及 compare 的四平台分发终验**。
> **开发范围：`tools/phoenix-ui` 引擎（`crates/context/src/compare/` 新模块与页面资源、`serve_question.rs` 中被复用助手的可见性、`crates/cli` 的命令路由与测试、`crates/detect/src/lib.rs` 的总帮助、oracle 用例、第三方声明）、`skills/phoenix-ui/src` 的 SKILL 源文件与 animate、craft-floor、routing、new-work 四份参考及两份新参考、`skills/phoenix-ui/evals/scenarios.md`、`docs/phoenix-ui.md`、`README.md`、`tools/phoenix-ui/docs/ENGINE.md`、`scripts/smoke-phoenix-ui.mjs`；依据见 §0**。
> 本期独特职责：决策页只在构建之前选方向，live 只比元素级变体，component-review 只收 comp-led 清单；本期补上“已经做出来的整页方案按真实尺寸放在一起、由用户选定”这一段，并且第一次要求主代理在动手前把模糊评价说成变量、在交付后给出下一步。
> **顶层排除：不改方向轮决策页、component-review 审阅页与 live 的任何行为；不新增检测规则；不改完工评审与走查子代理的定义；不改 review-ui、apply-ui-review、feature-demo 等独立 skill；对比页不做像素差分、不写回项目文件、不支持多轮更新；不发布 npm**。

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

- 最近更新：2026-11-13 14:54 +08:00
- 当前进度：4/5 个里程碑完成
- 当前状态：M1–M4 已完成；F-01 同键启动已修复并通过本机验证。G-01 空屏行为已补证，但完整 A2 因宿主超检查轮次、未交付最终报告仍未通过；新候选 V-9 待授权
- 最近完成：M4 · 复述终验、文档与声明文字
- 下一步：收敛宿主检查轮次并完整复验 A2；获准推送后补新候选的四平台与 Node 18 证据
- 当前阻塞：本地无外部配置前置；完整 A2 属本方验收缺口，新候选 V-9 尚未取得推送授权
- 代码基线：main / `55fdfdfd` + 本次修复（待提交）；原有评审与验收记录修改保留

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-13 14:54 +08:00 | compare 生命周期 F-01 已修复；并发、安全、浏览器、release 崩溃恢复与 18 条 oracle 通过 | [M1 记录](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M1.md) | `0a49480`（与 M2 同一提交） |
| M2 | 已完成 | 2026-11-12 23:24 +08:00 | 对比页界面；V-1 十步实拍逐张查看通过，V-2、V-3 与 lint 通过 | [M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M2.md) | `0a49480` |
| M3 | 已完成 | 2026-11-13 10:24 +08:00 | 六份参考与触发句；V-4 通过，V-5 的 R2–R6 符合，R1 按“回复清单后等用户回复”的流程复验通过；R5 页面标注修订已通过定向复验 | [M3 记录](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M3.md) | `3adeb8a`；修订 `28a51d5`、`c93dec5`、`0279aec`；修复 `a18e025b` |
| M4 | 已完成 | 2026-11-13 10:24 +08:00 | 引擎命令表、docs、README、evals、声明文字；V-4 通过，交付后下一步经 V-7 A2、A3 验证，与 M3 共用的 R1 复验通过；R5 页面标注缺口已关闭 | [M4 记录](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M4.md) | `d2fe94b`；修订 `28a51d5`、`c93dec5`、`0279aec`；修复 `a18e025b` |
| M5 | 进行中 | 2026-11-13 14:54 +08:00 | 当前构建/分发/本机冒烟通过；G-01 空屏已补修后证据，完整 A2 仍未过；新候选四平台待授权 | [M5 记录](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M5.md) | 候选 `55fdfdfd`；动效修订 `18852ff1`；前期候选 `90045192`、`4844a90`；修复 `a18e025b` |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID（尚未录入 SPMS，不是真实需求 key）。WORK-N2、WORK-N4 的终验期是 P4：本期完成所列 AC，并在收口确认整条需求的各期 AC 同时成立。WORK-N1（终验 P3）与 WORK-N3（终验 P2）已终验，本期只保证新增能力不破坏它们。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F10 | 功能 | PRD §3 WORK-F10/AC-1–AC-6 · P4 | 现状基线与多个已构建方案在一页并排，基线有标记，方案信息未填不显示；HTML 与本机地址方案可切桌面/手机、看 100%、键盘切换，截图按原图显示；页上选定并附说明后主代理直接收到；关页无选择时再问一次，仍无回答则有基线保持现状、无基线按推荐继续并写明；未标可交互的 HTML 方案不执行脚本、不发起网络请求，开发服务器未运行的方案显示不可用与启动说明；方向轮决策页、component-review 与 live 行为不变，单方案不弹出 | §5、§6.1、§6.2；V-1、V-2、V-3、V-5（R2、R3）、V-7（A1、A2、A4） | 已确认 |
| WORK-F11 | 功能 | WORK-F11/AC-1–AC-5 · P4 | 动效从被操作元素出发、一个方向、主角先动其余晚 40–80ms 跟随、形状接力；只变尺寸或位置的过渡不交叉淡化，选中指示用一块滑块；手感用“动作 + 实物”命名并给弹簧参数或缓动，大面积过冲不超过 2%；看不见结果的操作用短提示说出结果；减少动态效果时直接到位；未实际操作核对的手感在交付报告标注 | §6.4、§6.5、§6.7；V-4、V-5（R4）、V-7（A2） | 已确认 |
| WORK-F12 | 功能 | WORK-F12/AC-1–AC-4 · P4 | 模糊评价先回复“要改的变量 → 预期变化”清单并等用户回复，回复后才动手，交付后逐项说明（PRD v1.5）；无法判断时在同一次回复里给两个具体选项，或给两个可比较的变体（元素级 live、整页对比页）；方向轮中的“更大胆/更稳妥”仍按方向轮语义；翻译不出变量的形容词不作改动理由 | §6.3、§6.6；V-4、V-5（R1）、V-7（A3） | 已确认 |
| WORK-F13 | 功能 | WORK-F13/AC-1–AC-3 · P4 | 演示数据没有整数化指标、占位人名、全部相同的相对时间与占位字，多处演示共用一个虚构场景，可能被当真的内容标为示例；图标一套库、描边与字重相称、尺寸为字号 1–1.25 倍、纯图标按钮有可访问名称；出现弹窗内输入、多屏导航、内部滚动容器时分别给出核对结果，不出现时不做 | §6.5；V-4、V-5（R5）、V-7（A2） | 已确认 |
| WORK-F14 | 功能 | WORK-F14/AC-1–AC-3 · P4 | 新界面、改版或命令执行交付后，答复末尾最多 3 条带编号的下一步，第一条标“推荐”，每条写收益与代价；小改动、停在确认点、已说明下一步或无人值守时不列；只列当前可执行且本次未做过的 Phoenix 命令或流程，回复编号即进入 | §6.6；V-4、V-5（R5、R6）、V-7（A2、A3） | 已确认 |
| WORK-N2 | 兼容/分发 | WORK-N2/AC-4 · P4（本期终验；AC-1–AC-3 属 P1、P3，持续满足） | 对比页命令在 macOS（arm64/x64）与 Linux（x64/arm64）可用，无需 Rust；engine 缺失或版本不符时返回固定包版本的补装命令；Windows 等其他平台仍在安装预检阶段停止 | D1、§7；V-6、V-8、V-9 | 已确认 |
| WORK-N4 | 可维护 | WORK-N4/AC-3 · P4（本期终验）；AC-2 每期；AC-1 持续满足 | P1–P4 的新命令、新规则与流程变化在 SKILL.md 命令表与 Setup/Routing、相关 reference、docs/phoenix-ui.md、README 均有完整描述，没有只在一处出现的；动 skill 文件前整合清单已到本期范围，diff 逐项核对；oil-ui 的 MIT 声明覆盖本期新增的改编范围 | §0.6、§6.8–§6.10；V-4、V-8 | 已确认 |
| WORK-N1 | 安全/隐私（持续） | WORK-N1/AC-4（P3 已终验） | 新命令不给引擎带来外网请求：对比页服务只监听本机，只对载荷里的本机地址做连通探测 | §5.6、§7；V-2、V-8 | 已确认 |
| WORK-N3 | 兼容（持续） | WORK-N3/AC-1–AC-2（P2 已终验） | 没用到新能力的既有流程行为不变；完工评审仍只有四个处置词 | D17、§7；V-6、V-7（A4）、V-8 | 已确认 |
| C-1 | 约束 | AGENTS.md `@skill_tool_independence`；PRD N14 | 只有 phoenix-ui skill 随工具一起改；design-reference、review-ui、apply-ui-review、feature-demo 等独立 skill 不因本期改动 | §0.5、§1.2；V-8 | 已确认 |
| C-2 | 约束 | AGENTS.md §7「Reference Skill Integration Gate」；PRD §5.3 C8–C12、C14 | 借鉴 oil-ui 的规则与流程先列清单再改；Phoenix 现有原则是基线，冲突项调整或拒绝 | §0.6 | 已确认 |
| C-3 | 约束 | skills/phoenix-ui/src/SKILL.src.md:21 | 设计自检仍是一轮批量检查加至多一轮确认；动效核对与三类补验在这两轮内完成，不新增截图轮次或检测轮次 | §6.4、§6.5 | 已确认 |
| C-4 | 约束 | scripts/check-phoenix-deps.mjs:457-466 | `crates/` 下新增的 js、css、html 文件（`tests/` 目录除外）会进入许可清单，新增后要 `--write` 重算 | §1.2、M5；V-6 | 已确认 |
| C-5 | 约束 | tools/phoenix-ui/Cargo.toml:38（`panic = "abort"`） | 载荷 JSON、HTTP 请求与文件路径都是外部输入，一律走 `Result`，不 `unwrap`、不越界索引 | §5；V-2 | 已确认 |
| C-6 | 约束 | tools/phoenix-ui/THIRD-PARTY.md；crates/context/Cargo.toml:18、:21 | 不新增第三方包；`tiny_http`、`serde_json` 已是 `crates/context` 的依赖，`Cargo.lock` 不应变化 | D1；V-6 | 已确认 |
| C-7 | 约束 | AGENTS.md「Working in This Repository」接口复述同步 | 新命令与流程变化在总帮助、SKILL.md、相关 reference、docs/phoenix-ui.md、README、ENGINE.md 中完整描述 | §6.8–§6.9；V-4、V-8 | 已确认 |
| C-8 | 约束 | docs/phoenix-ui.md「维护者」节 | 改过 `skills/phoenix-ui/src` 后运行 `bun scripts/render.mjs` 并提交生成的 skill | M3、M4；V-4 | 已确认 |
| C-9 | 约束 | tools/phoenix-ui/tests/oracle/README.md | 每个 verb 都有端到端 golden；总帮助变化要重录 `cli-help`，逐个审阅 diff；既有 golden 逐字节不变 | M5；V-6 | 已确认 |
| C-10 | 约束 | AGENTS.md「Working in This Repository」 | 留在当前分支；推送只在用户授权时进行 | §10 提交点、V-9 | 已确认 |
| C-11 | 约束 | 用户既有偏好（PRD §4「用户既有偏好」） | 对比页的验收只认运行中页面的实拍与实测，不凭代码推断 | V-1、V-7 | 已确认 |
| A-1 | 假设 | PRD WORK-F10「依赖」；常见开发服务器默认不发 `X-Frame-Options` | 用户的开发服务器允许被本机页面用 iframe 嵌入 | 不满足时该方案的画面是空白：每个本机地址方案都带「在新标签页打开」，文档写明可改用截图方案（§6.1、§6.9）；V-7 A1 记录实际情况；责任人：实施者；最晚 M5 | 开放 |
| A-2 | 假设 | crates/context/src/component_review/server.rs:243 已用同一组合 | 各平台的 Chrome/Chromium 对带 `sandbox` 的 CSP 文档，仍按 `'self'` 加载同一服务上的样式、图片与字体，并阻止脚本与对外请求 | V-3 在本机与四平台 CI 实测（V-9）；不成立的平台按 §11 处理；责任人：实施者；最晚 M5 | 开放 |

未明确要求的 NFR（对比页加载耗时、同时比较的方案数量、守护进程的资源占用）按常规工程做好，不建基线、不设专项检查（§0.5）。

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 形态 | 原生引擎新增 verb `compare`，放在 `crates/context/src/compare/`★，与它的两个同类（决策页 `serve_question.rs`、审阅页 `component_review/`）同一个 crate；HTTP 用已有的 `tiny_http` | 不新增第三方包，`Cargo.lock` 不变；随现有 engine 分发；启动器无 verb 白名单，无需改 | PRD D2；WORK-N2；C-6；crates/context/Cargo.toml:21 |
| D2 | 命令名（PRD Q1） | `phoenix-ui compare`，即 PRD 的工作名 | 文档与验收不再改名 | PRD §5.2 Q1 |
| D3 | 回传协议 | 与决策页相同的两步：`--start` 起一个脱离终端的本机服务并打印地址与键，`--wait --key` 取回选择；状态文件在 `.phoenix-ui/compare/`；退出码含义与 `serve-question` 一致（0 已回答、3 仍在等、4 页面已关、2 没有浏览器或服务已不在） | 主代理只需学一套协议；宿主把长命令放到后台或回收子进程时也能用 | ADR-2；crates/context/src/serve_question.rs:421-501、:560-650；skills/phoenix-ui/src/reference/new-work.md:55 |
| D4 | 方案来源 | 每个方案给 `source`（本地 `.html`/`.htm` 或图片文件）或 `url`（本机地址）之一；类型由给了哪个字段和扩展名决定，不设 `kind` 字段 | 载荷少一个会写错的字段 | WORK-F10「方案来源」 |
| D5 | HTML 方案的隔离 | 对比页服务只提供该方案所在目录里的文件，响应带 CSP 与 `sandbox`：默认不执行脚本、不向对比页服务之外的任何主机发请求；标了 `interactive` 的才允许内联与同目录的经典脚本，仍不能联网 | 方案自己的样式、图片、字体照常显示；`../` 与以 `/` 开头的资源路径取不到，这类页面用本机地址方案；“不发起网络请求”按“没有请求离开对比页服务”验收（用户 2026-11-12 确认） | ADR-1；WORK-F10/AC-5；crates/context/src/component_review/server.rs:243 |
| D6 | 本机地址方案 | 只接受主机为 `localhost`、`127.0.0.1`、`[::1]` 的 http(s) 地址，直接用 iframe 嵌入；可用性由服务端对该主机端口做一次 TCP 连接判断；每个本机地址方案都带「在新标签页打开」 | 开发服务器没起时显示不可用与启动说明，其余方案照常；不解析 `X-Frame-Options`（A-1） | WORK-F10/AC-5；§5.6 |
| D7 | 视口 | 桌面 1440×900、手机 390×844，与 `capture` 的预设相同；iframe 按真实视口尺寸渲染后整体缩放到栏宽，100% 时不缩放 | 并排时看到的是真实布局的缩小图，不是被挤窄后重排的页面 | WORK-F10/AC-2；crates/cli/src/capture/mod.rs:52 |
| D8 | 方案数量与基线 | 至少两个方案才启动；最多一个基线，基线排在第一位 | “单方案交付不弹出对比页”由命令保证；不设数量上限，栏多了横向滚动 | WORK-F10/AC-6；oil-ui build_explorer.py:784-789 |
| D9 | 回答内容 | `{"proposalId", "name", "baseline", "liked", "disliked"}`；说明是两个可空的文字框 | 主代理拿到方案标识与用户的话，不需要复制粘贴 | WORK-F10/AC-3；PRD §2.4「对比页」 |
| D10 | 无回答 | `--wait` 以退出码 4 报告页面已关，并在输出里写明下一步：用结构化提问再问一次；仍无回答时，有基线保持现状并报告待选，无基线按主代理推荐继续并写明未经用户选择 | 规则同时写在命令输出与 skill 参考里，主代理在需要的那一刻读得到 | WORK-F10/AC-4 |
| D11 | 页面外壳与语言 | 外壳用审阅页的 kinpaku 设计令牌与三款字体，只用纸色、墨色与焦点色；界面文案默认简体中文，载荷写 `"lang": "en"` 时用英文 | 外壳不带风格倾向，不影响方案观感；与 `capture`、`media` 一样默认面向中文用户，英文会话不出现中文外壳 | PRD §2.4「对比页」；crates/comp-verbs/src/map_inspection.rs:409-431 |
| D12 | 命令行输出 | 帮助与校验错误用中文（同 `capture`、`media`）；协议行以固定的 ASCII 关键字开头：`COMPARE URL:`、`COMPARE KEY:`、`ANSWER:`、`WAITING:`、`PAGE CLOSED:`、`BASELINE KEPT:` | skill 参考可以不分语言地引用这些关键字 | crates/cli/src/capture/mod.rs:39；crates/context/src/serve_question.rs:102、:464、:474 |
| D13 | 无浏览器信号 | 沿用 `PHOENIX_UI_QUESTION_DISABLED` 与 `PHOENIX_UI_QUESTION_FORCE` 两个环境变量和同一套无头判断；`--start` 命中时退出 2 并提示改用结构化提问 | 一个会话里三种页面的开关一致 | crates/context/src/serve_question.rs:393-405；crates/context/src/component_review/mod.rs 的 `serve` 分支 |
| D14 | 代码复用方式 | `serve_question.rs` 里要复用的助手函数只把可见性放宽到 `pub(crate)`，不移动、不改写（`reject_detached_post` 的签名因加了可见性关键字超出行宽，按 rustfmt 折成多行，仓库 lint 门把被触及的格式差异计为新增）；无头判断那几行在新模块里照写一份 | 决策页的行为与 golden 不受影响；接受少量受控重复 | AGENTS.md `@surgical_changes`；WORK-F10/AC-6 |
| D15 | skill 落位 | 对比页写成新参考 `compare.md`★；模糊反馈写成新参考 `vague-feedback.md`★；动效规则进 `animate.md`；演示数据、图标与补验进 `craft-floor.md`；交付后的下一步进 `routing.md`；`SKILL.src.md` 只加触发句与引擎命令表；其中“模糊评价”与“交付后下一步”两条触发句带上形式要点（先发“变量 → 预期变化”清单；最多三条、编号、首条标推荐、带代价、只列 Phoenix 命令或流程） | SKILL.md 每个会话都加载，只放触发条件；细则在用到时才读。V-5、V-7 的会话在交付收尾时不会为这两条再去读参考，所以要点必须在常驻文本里 | §0.6；§6 |
| D16 | SKILL.md 命令表 | 在 Commands 一节的设计子命令表之后加一张「Engine commands」小表，列 `capture`、`detect <url>` 页面规则、`media`、`prep-image`、`compare` | 引擎命令不是 `/phoenix-ui <子命令>`，不进设计子命令表（该表与 `command-metadata.json` 的 23 个子命令一一对应，冒烟断言其数量）；WORK-N4/AC-3 要求的“SKILL.md 命令表”有了落点 | scripts/smoke-phoenix-ui.mjs:94；§6.8 |
| D17 | 既有页面的回归门 | 决策页、component-review、live 的源码除 D14 的可见性外不改；回归看既有 oracle golden 逐字节不变、既有浏览器测试通过，并在收口实际打开一次决策页走通选择 | AC-6 有确定的判据 | V-6、V-7 A4、V-8 |
| D18 | 收口验收 | 本机 darwin-arm64 完成开发构建、冒烟、Phoenix UI 全量回归与真实使用验收；四平台与 Node 18 用现有 Phoenix CI（push 触发），推送须用户授权 | 未获授权推送时 V-9 记未运行，收口不宣称 WORK-N2/AC-4 通过 | C-10；docs/phoenix-ui.md「维护者：统一候选与本地交接」 |

### 0.3 ADR-lite

#### ADR-1：HTML 方案由对比页服务提供自己的目录，靠响应头隔离

- 状态：Accepted
- 背景与驱动：WORK-F10/AC-5 要求未标可交互的 HTML 方案不执行脚本、不发起网络请求；方案常是主代理或生图流程产出的页面，可能带统计脚本、外链字体或远程图片。oil-ui 的对比页是一个静态文件，没有服务，所以它把每个方案的样式、图片全部改写内联进一个文档（build_explorer.py:239-476）。Phoenix 的对比页为了回传选择必须有本机服务。
- 备选：
  1. 服务只提供方案所在目录，响应带 `script-src 'none'; connect-src 'none'; default-src 'none'` 加 `sandbox`，样式、图片、字体只允许来自同一服务。审阅页已用同一做法提供评审快照（component_review/server.rs:243）。
  2. 照 oil-ui 把方案改写成单个内联文档放进 `srcdoc`：字面上零请求；要在 Rust 里重写 HTML 与 CSS 的资源改写（`url()`、`@import`、`srcset`），约 240 行 Python 的等价物，改写出错时方案画面与真实页面不一致。
  3. 一律先用 `capture` 截成图片再比较：没有隔离问题；失去视口切换、滚动与可交互小样，AC-2 不成立。
- 决策：选 1。
- 正面后果：方案按它在浏览器里的真实样子显示；隔离由浏览器按响应头执行，可在网络侧验证；与审阅页同一套写法。
- 负面/中性后果：方案自己的文件会向本机的对比页服务发请求，AC-5 的“不发起网络请求”按“没有请求离开对比页服务”验收，此口径经用户 2026-11-12 确认（§11）；资源路径越出方案目录（`../`）或以 `/` 开头的取不到，需改用本机地址方案；可交互小样只支持内联与同目录的经典脚本，模块脚本在无同源权限的沙箱里取不到。
- 重新评估触发：用户要求字面上的零请求；常见方案大量依赖跨目录资源；某个受支持平台的浏览器不按 A-2 行事。

#### ADR-2：选择回传沿用决策页的两步协议，新模块复用其助手，不扩展 `serve-question`

- 状态：Accepted
- 背景与驱动：WORK-F10/AC-3 要求选择直接回到主代理；AC-6 要求决策页行为与升级前一致。决策页已经解决了“脱离终端的本机服务、心跳、按键校验的回答、分段等待”这些问题，并记录过宿主把长命令放到后台或回收子进程的情况（new-work.md:55）。
- 备选：
  1. 新模块 `compare/`，照决策页的协议写自己的 `--start`/`--wait`/`--stop`，复用 `serve_question.rs` 里与页面内容无关的助手（参数读取、进程探测、Host/Origin/键校验、URL 解析、原子写、打开浏览器）。
  2. 给 `serve-question` 的载荷加一种“对比”卡片：PRD D7 已否决“在决策页加实拍卡”；该文件是逐行移植并由 golden 锁定的 2000 行实现，改它直接碰 AC-6。
  3. 单条阻塞命令，选完才返回：实现最少；宿主会把它放到后台或超时杀掉，页面就悬在那里。
- 决策：选 1。
- 正面后果：决策页源码只动可见性；主代理面对两种页面用同一套命令形状与退出码。
- 负面/中性后果：等待循环与守护进程的启动各有一份相似代码（对比页不需要翻牌、重发牌与设计稿到位检查，所以更短）；状态目录多一个 `.phoenix-ui/compare/`。
- 重新评估触发：出现第三种需要回传选择的页面，届时再抽公共模块。

### 0.4 职责与事实所有权

|  | `phoenix-ui compare` | Phoenix 主代理流程（skill 文本） | 用户的开发服务器（范围外） | 决策页、component-review、live（既有） |
| --- | --- | --- | --- | --- |
| 拥有 | 载荷校验结果、对比页的展示、方案文件的隔离提供、本机地址是否连得上、用户提交的选择与说明、页面是否还开着 | 什么时候需要比较、放哪些方案、哪个是基线、方案信息怎么写、收到选择后怎样继续、无回答时怎样处置、交付报告怎么写 | 本机地址方案的页面内容与能否被嵌入 | 方向选择、组件评审、元素级变体各自的流程与状态 |
| 不拥有 | **不评价方案、不推荐、不替用户选；不改任何项目文件；不启动或停止用户的开发服务器** | **不替用户提交选择；不把无回答当作选了推荐方案而不说明** | — | **不接收对比页的任何数据** |

一句话：`compare` 只负责“把这些方案原样摆出来、把用户点的那个带回去、没点就说没点”；摆什么、之后怎么做，归主代理与用户。

### 0.5 明确不在本期

- **方向轮决策页、component-review 审阅页、live 与 generate 的行为**——不改；对比页不进入它们的职责（WORK-F10 边界）。
- **对比页的多轮更新（`--update`）、阻塞式运行、自定义视口、像素差分或叠加对比、把选择写回项目文件**——不做；差分已由 comp-diff 覆盖（PRD §5.3 C2）。
- **探测开发服务器是否拒绝被嵌入（`X-Frame-Options`、`frame-ancestors`）**——不做；用「在新标签页打开」与截图方案兜住（A-1）。
- **新的检测规则**（演示数据、图标、动效）——不做；WORK-F11、F13 是写进 skill 的指引，不是 `detect` 规则。
- **oil-ui 的“每个界面先做三处动效”、强制首次出场、整页转场三分类、它的时长表、悬停上浮幅度、动效模糊词表**——拒绝或不采纳（PRD N3；§0.6 I-3、I-4）。
- **完工评审、走查、素材制作、文档子代理的定义**——不改。
- **design-reference、review-ui、apply-ui-review、feature-demo、ui-pattern-research、design-md、ui-loop 等独立 skill**——不改（C-1）。
- **对比页加载耗时、方案数量上限、守护进程资源占用的基线与调优**——后续可选；本期按常规工程写。
- **发布、npm 版本与 SPMS 录入**——按维护流程另行执行。引擎源码已变，下次发布不能复用旧引擎。
- **Windows**——沿用支持矩阵的延期；安装预检行为不动。

### 0.6 参考整合清单（动 skill 与命令前核对；WORK-N4/AC-2、C-2）

参考对象是本机 `local/oil-ui-main`（oil-ui 0.33.2，MIT；目录无 `.git`，只在本机可用）。Phoenix 现有原则为基线；整合不改变原则、立场与调用/授权策略。M1、M3、M4 动对应文件前按本表读一遍当前目标段落；实施中出现表外的行为差异，先改本表并解决冲突再继续。

| # | 目标文件/规则 | 现有行为 | 参考来源/段落 | 采用、调整或拒绝 | 对原则、调用、接口与验收的影响 | 核对 |
| --- | --- | --- | --- | --- | --- | --- |
| I-1（PRD C8） | 新 verb `compare`（`crates/context/src/compare/`★） | 决策页只在构建前用（卡片放方向信息、设计稿或线框）；component-review 只收 comp-led 清单；live 只做元素级变体 | `references/style-explorer.md`:3-48；`scripts/build_explorer.py`：隔离用的 CSP（:161-164）、方案字段与校验（:644-790）、基线排第一且最多一个（:784-789） | 采用：外壳不带风格倾向（:3）、能跑就用真实页面、项目里的方案用本机地址、拿不到才用截图（:9）、现状标为基线（:29）、方案信息字段与“省略的项不显示”（:32）、可操作小样单独标记（:27）、本机地址附启动说明（:28、:42）、判断文字与细节用 100% 实际尺寸（:48）。调整：用本机服务而不是生成静态文件；选择经回答文件直接回传；无回答时再问一次并保持现状；方案由服务按目录提供并靠响应头隔离（ADR-1）；视口为桌面/手机两档，不设 `nativeViewport`；说明分喜欢与不喜欢两栏。拒绝：点“选择”复制一句话再粘贴回来（:46）、Python 生成器与 `--force` 重新生成（:40）、按目录或 `<title>` 推断方案（:11）、把选择写进设计说明（:46） | 新公开接口；不改方向轮、live、component-review；“只有用户做选择”的授权边界不变 | V-1、V-2、V-3、V-8 |
| I-2（PRD C8） | `skills/phoenix-ui/src/reference/compare.md`★；`SKILL.src.md` Routing（:88-97） | 没有整页方案比较的入口；new-work.md:55 只描述决策页的 `--start`/`--wait` | style-explorer.md:5（几个候选的核心内容、主要动作保持一致）、:9（每个方案一个画面） | 采用：比较的方案内容与主动作一致，只变设计；每个方案一个代表画面。调整：触发条件为用户明确要求比较，或一次交出两个以上需用户选择的可运行整页方案（含改版前后）；无回答规则按 D10 | 新入口；单方案交付不打开；不新增授权步骤，打开对比页属于用户请求内的操作 | V-4、V-5（R2、R3） |
| I-3（PRD C9） | `skills/phoenix-ui/src/reference/animate.md`：「Choose material by meaning」（:36-48）、「Timing and easing」（:50-61）、「Accessibility and control」（:73-77）、「Verify」（:79-87）；`craft-floor.md`:14 的 Motion 一条 | 已有：共享元素与 FLIP、同级错峰且限制总延迟、离开快于进入、不凭反射用弹跳、可打断、减少动态效果的替代路径、“一处精心动效” | `references/motion.md`：「手感」（:23-40）、「呼应」（:42-50）、「界面过渡」中的四条（:64-74）、「看不见的结果要说出来」（:100-102）、「检查动效」（:114-116） | 采用：从被操作处出发、一次切换一个方向、主角先动其余晚 40–80ms 且幅度更小（被主角经布局推动的内容随它同步，不另设延迟；延迟用在各自响应的元素上，如面板内的字段、计数、相邻元素的状态变化）、形状接力；只变形状不交叉淡化、离开/加入/留下分开处理、选中指示用一块滑块且首次显示与缩放后直接就位、目的地先准备好、到头轻顶 10–15px；“动作 + 实物”命名与弹簧参数表、过冲随面积、弹簧采样成 `linear()` 并退回相近曲线；看不见的结果用短提示说出结果本身。调整：回弹一行只在方向或 brief 支持时使用，保留“不凭反射用弹跳”；核对用 `capture` 的序列帧看状态变化，手感只认实际操作，未操作时交付写“正常速度手感未核对”。拒绝：每个界面先做三处动效与强制首次出场（:3-11）、整页转场三分类（:76-84）、它的时长表（:86-96，保留 Phoenix 的）、悬停上浮幅度（:98）、“离开比进入快”的重复表述（Phoenix 已有） | 保留“一处精心动效”与 brief 优先；不强制新增动效；原生平台仍按 ios.md/android.md | V-4、V-5（R4） |
| I-4（PRD C10） | `skills/phoenix-ui/src/reference/vague-feedback.md`★；`SKILL.src.md`「How to design」（:42-47） | 只有方向轮定义了“更大胆/更稳妥”（new-work.md:53）；`bolder`、`quieter`、`distill` 是整套命令，不处理一句评价 | `references/design-direction.md`「模糊词必须翻译」（:64-78） | 采用：高级、精致、克制、史诗、简洁五个词的翻译与“翻译不出就从理由里删掉”。调整：动手前回复“变量 → 预期变化”清单并等用户回复，交付后逐项说明；拿不准在同一次回复里给两个具体选项，或给两个变体（元素级 live，整页对比页）；评价同时对应某个精修命令时照常进入该命令，翻译仍先做。拒绝：动效模糊词表（:80-87，含滚动叙事词条）、“文学感”“粗糙”两行（PRD WORK-F12 未列） | 不改方向轮语义；用户要求换方向时走既有方向流程 | V-4、V-5（R1） |
| I-5（PRD C11） | `skills/phoenix-ui/src/reference/craft-floor.md`「Verify」（:7-18）与「Refuse」里的图标一条（:41） | 已有：演示数据按全保真写并标为合成、不虚构商业声明（new-work.md:63、:135；mode-operate.md:17）；不用字符或 emoji 充当图标（craft-floor.md:41）；图标族一致（polish.md:68）；纯图标控件要有可访问名称（clarify.md:65）；自定义控件手势与安全区（adapt.md:191-195、:243-257） | `references/visual-language.md`「示例数据要像真的」（:48-50）、「控件与图标」（:72-80）、「图标库也会趋同」（:82-102）；`references/layout-and-viewport.md`「响应式与验证重点」中的三条（:21-23） | 采用：演示数据的反例清单、共用一个虚构场景、按钮写具体动作；一个产品一套图标库、存量项目沿用已装的库、新项目按调性选而不是默认 Lucide 或 Heroicons、描边跟字重、尺寸 1–1.25 倍并按视觉中心对齐、陌生图标配文字、核对许可；弹窗内输入、多屏导航、内部滚动容器三项只在结构出现时补验。调整：可能被当真的演示内容仍按 Phoenix 规则标为示例（不采纳“界面上不写示例”）；补验在既有的两轮检查内完成并在交付写结果，键盘遮挡的证据要说明是仿真视口还是真机（沿用 adapt.md:195 的证据口径）。拒绝：图标库对照表全文（只留一句选型原则）、Canvas/手势/复杂多栏/特殊视口各条（adapt 已覆盖） | 不改 Phoenix 的数据、图标与适配规则；不新增截图轮次（C-3） | V-4、V-5（R5） |
| I-6（PRD C12） | `skills/phoenix-ui/src/reference/routing.md`★新增一节；`SKILL.src.md` Routing | 只有不带参数调用时给基于 signals 的菜单（routing.md:7-24）；animate.md:89 等个别参考末尾指向 polish | `SKILL.md`「接下来可以做」（:142-164） | 采用：最多三条、第一条标推荐、每条一句写得到什么与代价、只列真能做且没做过的、带编号可直接回复、四种情况不列、不列工程任务、代价没实测就用粗略说法。调整：候选只从 Phoenix 的命令与流程里挑，可沿用 signals 的判断；用会话语言。拒绝：它的候选对照表（步骤是 oil-ui 的）与“生图积分” | 不带参数菜单的“从不自动运行”不变：用户回复编号才进入 | V-4、V-5（R5、R6） |
| I-7（PRD C14） | `tools/phoenix-ui/NOTICE.md` 与 `tools/phoenix-ui/THIRD-PARTY-NOTICES.md` 的 oil-ui 条目 | 改编范围写到 capture、走查子代理、media 与 prep-image | `local/oil-ui-main/LICENSE`（MIT） | 采用：改编范围补上对比页与四组指引，MIT 全文与版权行不动 | 许可合规（WORK-N4/AC-1 持续满足） | V-6、V-8 |
| I-8（PRD C13 中与本期相邻的拒绝项） | oil-ui 的方向卡“只推荐一个方向”、评审打分、滚动叙事、工作区、版本检查 | — | PRD §1.3 N1–N6、N15 | 拒绝 | 无改动 | V-8 |

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·足够] 命令路由**：`run()` 按 verb 匹配（tools/phoenix-ui/crates/cli/src/main.rs:37-136），`serve-question` 在 :74、`component-review` 在 :75；以 `live` 开头的 verb 在 :120，当前目录里存在同名文件或目录的第一个参数会落到 `detect`（:123），所以 `compare` 的分支必须写在这两条之前。总帮助是 `ROOT_USAGE`（crates/detect/src/lib.rs:53），由 oracle 的 `cli-help` golden 锁定。verb 函数形如 `fn(&[String], &mut Io) -> i32`。
- **[已核实·足够] 决策页的回传协议**（crates/context/src/serve_question.rs）：`--start` 校验载荷后以脱离终端的方式再启动自己（:560-650，`proc::detach` 在 :621），等状态文件出现（最多 8 秒）后打印地址与键并退出；服务绑定 `127.0.0.1` 的随机端口（:674），把 `{pid, port, url}` 写进 `.phoenix-ui/questions/<键>.state.json`；页面定时 POST `/heartbeat`，服务把 `lastBeat` 写回状态文件（:769-786）；POST `/answer` 先过键与 Origin 校验（:825-830、:963-976），把回答原子写入 `<键>.answer.json`（:910）后退出；每个请求先过 Host 白名单（:731、:945-952）。`--wait` 轮询回答文件（:421-501）：有回答打印 `ANSWER:` 并删除状态文件，返回 0；服务不在且没有回答返回 2（:446）；心跳停了 15 秒以上返回 4 并打印 `PAGE CLOSED`（:464）；到时仍无回答返回 3 并打印 `WAITING`（:474）。看门狗线程在一直没有心跳且超过 `--timeout`（默认 900 秒）或心跳停了超过 `--idle-grace`（默认 600 秒）时结束进程（:698-716）。没有浏览器的会话（`PHOENIX_UI_QUESTION_DISABLED`，或 CI、无显示的 SSH/Linux 且没有 `PHOENIX_UI_QUESTION_FORCE`）直接返回 2（:393-405）。这些助手都是模块私有函数。
- **[已核实·足够] 不受信内容的隔离提供**：审阅页服务对评审快照文件统一加 `default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; script-src 'none'; connect-src 'none'; base-uri 'none'; form-action 'none'; sandbox; frame-ancestors 'self'`（crates/context/src/component_review/server.rs:243），外壳页用 `script-src 'self'` 等受信策略（:114）；每个响应带 `Cache-Control: no-store`、`X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer`，字体额外带 `Access-Control-Allow-Origin: *`（:17-34，沙箱文档没有同源身份，字体按跨源加载）；POST 要求 Host、Origin 与 `Sec-Fetch-Site: same-origin`（:41-53）。
- **[已核实·足够] 审阅页的视觉资源**：`tools/phoenix-ui/ui/component-review/vendor/` 下的 `kinpaku-tokens.css`、`kinpaku-kit.css`（按钮、标签页、开关、标签等 `.ks-*` 类与 `--ks-*` 令牌，文件头注明不在此处修改）与三款字体 `tools/phoenix-ui/ui/component-review/fonts/{albertsans,alumnisans,jetbrainsmono}.ttf` 已被别的页面用 `include_str!`/`include_bytes!` 内嵌：区域图检查页把套件内联进页面、JSON 数据转义 `&`、`<`、`>` 后写进页面（crates/comp-verbs/src/map_inspection.rs:403-431），审阅页服务按固定路由提供三款字体（component_review/server.rs:161-172）。
- **[已核实·足够] 目录内文件的安全读取写法**：`capture` 的静态服务器把请求路径逐段解码，拒绝 `..`、以 `.` 开头的段、反斜杠与 NUL（crates/cli/src/capture/serve.rs:70-101），再把真实路径与真实根目录比较，挡住指向根目录之外的符号链接（:149-160）。它在 `crates/cli`，`crates/context` 不能反向依赖。
- **[已核实·缺口] 没有整页方案的比较入口**：决策页的卡片只有方向信息、设计稿或线框（serve_question.rs:348-364 的示例载荷），没有视口、100% 与 iframe；component-review 的清单是 comp 区域；live 只在运行中的页面里替换元素 → §5、§6.1 → 漏做则 WORK-F10 整条不成立。
- **[已核实·缺口] skill 文本**：animate.md 有论点、材质、时长与减少动态效果，没有“动作彼此接上”的规则、只变形状的过渡、选中滑块、手感命名与结果提示（skills/phoenix-ui/src/reference/animate.md:25-87）；全 skill 没有模糊评价的处理（只有 new-work.md:53 的方向轮语义）；演示数据只有“像真的、标为合成”的原则，没有中文占位反例与共用场景（new-work.md:63、:135）；图标只有“不用字符充当”和“族一致”（craft-floor.md:41、polish.md:68）；没有弹窗内输入、多屏往返、内部滚动三项补验；交付后没有下一步菜单（routing.md 只管不带参数的调用）→ §6.2–§6.7 → 漏做则 WORK-F11–F14 不成立。
- **[已核实·缺口] 对外复述不齐**（2026-11-12 对工作树 grep 的结果，实施时以 V-8 的命令重核）：`SKILL.src.md` 里没有 `capture`、`prep-image`、`xgent/cjk-*` 与走查的任何文字，`media` 有专门一节；`README.md` 的「Phoenix UI」一节（:50-62）只写了 media 与 prep-image，`capture` 与走查只出现在 review-ui、feature-demo 的表格行（:86、:101、:175），`xgent/cjk-*` 没有出现；`docs/phoenix-ui.md` 对四个命令与中文规则都有专节 → §6.8 → 漏做则 WORK-N4/AC-3 不成立。
- **[已核实·足够] 取证命令可用来看对比页并替用户点选**：`capture` 能对 http 地址按步骤点击、填写、按键、等待元素出现或消失后截图，定位只在主文档里找、不进 iframe（crates/cli/src/capture/mod.rs:39-112）；对比页的外壳控件都在主文档。引擎的页面扫描是被动快照，不需要绕过页面的 CSP（crates/browser/src/cdp.rs:918）。
- **[已核实·足够] 测试与发布入口**：二进制级测试用 `env!("CARGO_BIN_EXE_phoenix-ui")`，无浏览器时跳过（crates/cli/tests/capture.rs:23）；决策页的 POST 校验有一份原始套接字写的端到端测试可照做（crates/cli/tests/serve_question_security.rs）；oracle 里决策页的用例都不起浏览器，靠预置状态文件覆盖 `--wait` 的各种结局（tools/phoenix-ui/tests/oracle/cases/context.mjs:691-705）；`test:phoenix` 依次跑页内包检查、组件评审包检查、release 构建、安装测试、工作区测试（macOS 上在只放行本机的 sandbox 内）、浏览器测试（sandbox 外）、lint 与 oracle（scripts/test-phoenix-ui.mjs:27-50）；冒烟对新命令逐个加了帮助、缺 engine 与一次真实调用（scripts/smoke-phoenix-ui.mjs:107-123）；CI 的 push 路径已含 `tools/phoenix-ui/**`、`skills/phoenix-ui/**`、`scripts/*phoenix*.mjs`、`docs/phoenix-ui.md`（.github/workflows/phoenix.yml:10-30）。
- **[已核实·足够] 渲染**：`tools/phoenix-ui/scripts/render.mjs` 把 `skills/phoenix-ui/src` 渲染到 `skills/phoenix-ui/` 与各 provider 目录并写 `GENERATED.json`，`--check` 比对是否过期；`<!-- rule:… -->` 标记只在渲染时被剥掉，不与任何注册表比对（scripts/lib/utils.js:521-534）；Commands 表不被解析，子命令清单来自 `command-metadata.json`（crates/skills/src/commands.rs:138）。

### 1.2 拓扑与文件清单

全部在本仓 `XGENT-ai/skills`。★ 为新增。

| 位置 | 改动 | 所属 |
| --- | --- | --- |
| tools/phoenix-ui/crates/context/src/compare/★：`mod.rs`（参数、用法、`--schema`、`--start`、`--wait`、`--stop`、输出与退出码）、`payload.rs`（载荷解析与校验）、`server.rs`（路由、响应头、目录内文件读取、连通探测、看门狗） | 对比页服务与协议 | M1 |
| 同目录★：`page.html`、`page.css`、`page.js` | 对比页界面 | M2 |
| tools/phoenix-ui/crates/context/src/serve_question.rs | 仅把被复用的助手改为 `pub(crate)`：`Args`、`env_set`、`pid_probe`、`kill_pid`、`write_atomic`、`sleep_ms`、`read_json_file`、`allowed_host`、`allowed_origin`、`reject_detached_post`、`query_param`、`parse_request_url`、`random_key`（`open_system_browser` 已是 `pub`） | 复用 | M1 |
| tools/phoenix-ui/crates/context/src/lib.rs | `pub mod compare; pub use compare::run as run_compare;`，文件头的 verb 列表补上 compare | 导出 | M1 |
| tools/phoenix-ui/crates/cli/src/main.rs | `"compare" => impeccable_context::run_compare(rest, io),`，写在 `serve-question` 一行之后 | 路由 | M1 |
| tools/phoenix-ui/crates/detect/src/lib.rs | `ROOT_USAGE` 在 `prep-image` 之后加一行 | 总帮助 | M1 |
| tools/phoenix-ui/crates/cli/tests/compare_security.rs★、compare.rs★ | POST 校验与目录越界的端到端测试；浏览器级隔离与选择回传测试 | 测试 | M1、M2 |
| tools/phoenix-ui/tests/oracle/cases/compare.mjs★、golden/★、golden/cli-help.json | 不起浏览器的 oracle 用例；总帮助 golden 重录 | 回归 | M5 |
| scripts/smoke-phoenix-ui.mjs | 增加 compare 的冒烟 | 四平台证据 | M5 |
| tools/phoenix-ui/NOTICE.md、THIRD-PARTY-NOTICES.md（及 `--write` 重算的 THIRD-PARTY.json、dependency-audit.json） | oil-ui 改编范围；新页面资源进入许可清单 | 声明 | M4（文字）、M5（重算） |
| skills/phoenix-ui/src/reference/compare.md★、vague-feedback.md★；animate.md、craft-floor.md、routing.md、new-work.md；skills/phoenix-ui/src/SKILL.src.md 的「How to design」与 Routing 触发句 | 四组指引与对比页用法 | M3 |
| skills/phoenix-ui/src/SKILL.src.md 的「Engine commands」表；skills/phoenix-ui/evals/scenarios.md；docs/phoenix-ui.md；README.md；tools/phoenix-ui/docs/ENGINE.md | 复述终验 | M4 |
| 渲染生成的 `skills/phoenix-ui/` 文件、`GENERATED.json` 与 `dist/phoenix-ui/bundle/` | 随 M3、M4 各渲染一次并提交 | M3、M4 |

无需修改的邻接面：启动器与安装器（verb 透传，bundle 清单复制）；`crates/context/src/question_page.rs` 与 `component_review/`；`crates/live`、`crates/comp`、`crates/comp-verbs`、`crates/hook`、`crates/browser`；`command-metadata.json`（compare 不是设计子命令）；`skills/phoenix-ui/agents/openai.yaml`（skill 名称、参数与默认提示不变）；`skills/phoenix-ui/src/subagents/`；`.github/workflows/phoenix.yml`；`Cargo.toml` 与 `Cargo.lock`。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类/适配器 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `compare::run(args, io) -> i32` | main.rs 路由 | 先校验参数与载荷，失败退出 1，不起服务、不写状态文件；一次只做 `--schema`、`--start`、`--wait`、`--stop`、`--help` 之一 | 无 | 守护进程的启动与就绪等待、等待循环的四种结局、输出文字 | 进程内 | 命令输出、退出码与 `.phoenix-ui/compare/` 下的文件（V-1、V-2、oracle） |
| `compare::payload::parse(cwd, &Value) -> Result<Round, Vec<String>>` | run、server | 纯函数加文件存在性检查；一次返回全部问题；返回的 `Round` 里基线已排第一、每个方案带解析后的绝对路径或地址 | 无 | 来源类型判定、本机地址判定、路径必须在项目目录内 | 进程内（文件系统） | 单元测试的校验矩阵 |
| `compare::server::serve(round, opts, io) -> i32` | run（`--detached-serve`） | 只监听 `127.0.0.1`；任何文件读取都限定在对应方案的目录内；只有带键且来源合法的 POST 能写回答；回答写入后才应答成功 | 无 | 路由、三种 CSP、目录内读取、连通探测、心跳与看门狗 | 进程内；用户的开发服务器只被 TCP 连接探测 | 原始套接字端到端测试、浏览器级测试 |
| `page.html` / `page.css` / `page.js` | 浏览器 | 只通过 `/status`、`/heartbeat`、`/answer` 与服务交互；不读写本机存储；提交成功后停用全部控件 | 无 | 缩放几何、并排与单个两种视图、键盘、选择面板、不可用与提交状态、两种语言的文案 | 浏览器 | 实拍截图与 `--wait` 的结果（V-1、V-3） |

不加新接缝：对比页只有一个调用方，路由直接写在 `server.rs`；与决策页相似的等待循环各写各的（ADR-2）。目录内读取照 `capture/serve.rs` 的写法在 `server.rs` 里写一份约三十行的函数，不为它把静态服务器搬到公共 crate。

## 3. 数据模型与迁移

不涉及数据库与迁移。`.phoenix-ui/compare/` 下每个键有三个文件：`<键>.state.json`（`{"pid", "port", "url", "lastBeat"}`）、`<键>.answer.json`（一行回答）与 `<键>.log`（守护进程的输出）。回答被 `--wait` 取走后删除回答与状态文件；`--stop` 结束进程并删除两者；回答文件未被取走前，同键的 `--start` 被拒（§5.1）。文件没有版本字段，只在一次比较的生命周期内有效。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| 决策页的助手函数 | 已核实足够 | §1.1 第二条所列行段 | 只放宽可见性（D14） | — |
| `tiny_http` 0.12 | 已核实足够 | crates/context/Cargo.toml:21；`recv_timeout` 已在 component_review/server.rs:132 使用 | 零改动 | 端口绑定失败：守护进程退出，`--start` 在 8 秒内等不到状态文件，退出 1 并给出日志位置（沙箱内不能监听本机端口时同样如此） |
| 审阅页的设计套件与字体 | 已核实足够 | map_inspection.rs:409-431；`kinpaku-kit.css` 文件头 | 只 `include_str!`/`include_bytes!`，不修改 | — |
| 用户的开发服务器（范围外，由用户提供） | 已核实足够（契约只有“本机端口上有 HTTP 服务”） | WORK-F10「依赖」 | 提供方：用户；所需行为：监听载荷里的本机地址，并允许被 iframe 嵌入（A-1）；就绪证据：`/status` 报该方案可用。本方只做 TCP 连接探测，不启动、不停止、不改它 | 连不上：该方案显示不可用与启动说明，其余照常，退出码不受影响；拒绝被嵌入：画面空白，用「在新标签页打开」或改用截图方案 |
| 用户的浏览器 | 已核实足够 | serve_question.rs:1066 `open_system_browser`；D13 | `--open` 时调用系统打开方式；否则打印地址，由主代理用宿主内置浏览器或系统打开方式打开 | 没有浏览器的会话：`--start` 退出 2 |
| `phoenix-ui capture` | 已核实足够 | crates/cli/src/capture/mod.rs:39-112 | 零改动；验证时用它看对比页、替用户点选 | — |
| Phoenix 启动器 | 已核实足够 | skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs（verb 透传；P1、P3 的新 verb 均未改它） | 零改动 | engine 缺失或不符：退出 4 并给补装命令 |
| 许可清单与依赖审计 | 已核实足够 | scripts/check-phoenix-deps.mjs:457-466 | 新增页面资源与声明文字后 `--write` 重算 | 未重算时报清单过期 |
| Phoenix CI（四平台 + Node 18） | 已核实足够 | .github/workflows/phoenix.yml:10-30；docs/phoenix-ui.md「维护者」节 | 零改动；冒烟脚本增加 compare | 推送未授权：V-9 未运行，收口不宣称 WORK-N2/AC-4 |
| 宿主会话（Claude Code） | 已核实足够（P1–P3 收口均已使用） | docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M4.md、M5.md | 只在 V-5、V-7 使用 | 不可用时对应检查记环境缺失 |

登记位置：verb 登记在 main.rs 的路由与 `ROOT_USAGE`；没有新增配置项；沿用两个既有环境变量（D13）；状态目录 `.phoenix-ui/compare/` 见 §3。

## 5. 核心机制

### 5.1 命令接口

```text
phoenix-ui compare --schema
phoenix-ui compare --start --payload <文件> [--key <键>] [--open] [--timeout <秒>] [--idle-grace <秒>]
phoenix-ui compare --wait --key <键> [--poll <秒>]
phoenix-ui compare --stop --key <键>
phoenix-ui compare --help
```

- 默认值与决策页相同：`--timeout` 900、`--idle-grace` 600、`--poll` 60。`--key` 只接受 1–40 个字母、数字、下划线或连字符，省略时随机生成 8 位十六进制。
- 未知选项、缺值、互斥动作同时出现：打印问题与用法，退出 1。`--detached-serve` 是 `--start` 再启动自己时用的内部选项，不写进帮助。
- `--start` 的键已有存活的服务时拒绝并提示先 `--stop` 或换一个键，退出 1；该键还留有未领取的回答文件（上一轮已选、服务已退出、尚未 `--wait`）时同样拒绝，提示先 `--wait --key <键>` 领取或 `--stop --key <键>` 丢弃，退出 1，这样新一轮的 `--wait` 不会把上一轮的选择当作本轮结果。两项里先查未领取的回答：服务刚写完回答退出时，状态文件里的心跳还在 12 秒内，先查存活会提示“先 `--stop`”，照做就丢了用户的选择。

| 动作 | 退出码 | 输出 |
| --- | --- | --- |
| `--schema` | 0 | 示例载荷（§5.2）与字段说明 |
| `--start` | 0 | `COMPARE URL: <地址>`、`COMPARE KEY: <键>`、一句“现在为用户打开这个地址：宿主有内置浏览器就用它，否则用系统打开方式，再不行就把地址给用户”、一句“然后用 `<wait 命令>` 取回选择”，wait 命令为 `provider::detect(...).verb_cmd("compare")` 的返回值后接 ` --wait --key <键>`（`verb_cmd` 已含 `compare`，不再追加；写法同 serve_question.rs:649） |
| `--start` | 1 | 载荷读不到、不是 JSON 或校验不过：逐条列出全部问题；该键有存活服务或未领取的回答：提示先 `--wait` 或 `--stop`；服务 8 秒内没有就绪：日志末尾几行与日志位置，并说明沙箱内不能监听本机端口时会这样、应改用不受限的命令通道重跑一次 |
| `--start` | 2 | 当前会话没有浏览器（D13）：一句“改用结构化提问”，不起服务 |
| `--wait` | 0 | `ANSWER: <JSON>`；选的是基线时再加一行 `BASELINE KEPT: 用户选择保持现状，不要修改现有实现` |
| `--wait` | 3 | `WAITING: <N> 秒内还没有选择；再运行一次 --wait --key <键>` |
| `--wait` | 4 | `PAGE CLOSED: 对比页已关闭且没有选择。用结构化提问再问一次；仍无回答时：有现状基线就保持现状并在交付里写明待选，没有基线就按你的推荐继续并写明未经用户选择。处理完运行 --stop --key <键>` |
| `--wait` | 2 | 服务已不在且没有回答：说明这是服务故障不是用户的决定，用同一载荷重新 `--start` |
| `--wait`、`--stop` | 1 | 缺 `--key` |
| `--stop` | 0 | `stopped` |

### 5.2 载荷

`--schema` 打印的示例：

```json
{
  "title": "订单页改版方案",
  "question": "选一个继续做，或保持现状",
  "lang": "zh-CN",
  "serve": { "command": "pnpm dev", "cwd": "." },
  "proposals": [
    { "id": "current", "name": "现状", "url": "http://localhost:5173/orders", "baseline": true },
    { "id": "ledger", "name": "方案 A · 账本", "source": ".phoenix-ui/compare/variants/ledger/index.html",
      "concept": "把订单当作一页流水账来读", "typography": "思源宋体 + JetBrains Mono",
      "palette": ["#1f1c18", "#efe5d0", "#a33327"], "traits": ["密排表格", "单色强调"] },
    { "id": "board", "name": "方案 B · 看板", "source": ".phoenix-ui/compare/variants/board/index.html", "interactive": true },
    { "id": "sketch", "name": "方案 C · 截图", "source": ".phoenix-ui/review/sketch-mobile.png", "pixelRatio": 2 }
  ]
}
```

校验规则（全部问题一次列出，每条指到字段）：

1. 顶层必须是对象；`proposals` 是长度不小于 2 的数组；`title`、`question` 可省，`title` 省略时页面用「方案对比」。
2. 每个方案：`id` 非空、只含字母数字下划线连字符、彼此不同；`name` 非空；`source` 与 `url` 恰好给一个。
3. `source`：相对当前目录解析；必须是已存在的文件，真实路径在当前目录之内；扩展名 `.html`、`.htm` 为 HTML 方案，`.png`、`.jpg`、`.jpeg`、`.webp`、`.gif` 为截图方案，其他扩展名拒绝。
4. `url`：`http://` 或 `https://`，主机只能是 `localhost`、`127.0.0.1` 或 `[::1]`；其他主机拒绝，并提示改用截图方案。
5. `baseline` 最多一个为 `true`；`interactive` 只用于 HTML 方案；`pixelRatio` 只用于截图方案，取 1–4 的数，省略为 1。
6. `concept`、`typography` 是字符串，`palette`、`traits` 是字符串数组；这四项都可省，省略或为空时页面不显示对应位置。
7. `lang` 只认 `zh-CN`（默认）与 `en`；`serve` 可省，给了就必须有非空的 `command`，`cwd` 可省。

解析结果 `Round` 把基线移到第一位，其余保持载荷顺序；路由里的方案序号指这个顺序。

### 5.3 启动与守护

1. `--start` 先做无浏览器判断（D13）、读载荷并校验，再按 §5.1 拒绝有存活服务或未领取回答的键；通过后创建 `.phoenix-ui/compare/`，以 `compare --detached-serve --key <键> --payload <路径> --timeout <秒> [--idle-grace <秒>] [--no-open]` 再启动自己，标准输出与错误追加到 `<键>.log`，用 `impeccable_common::proc::detach` 脱离终端（写法同 serve_question.rs:588-624）。
2. 守护进程重新读取并校验同一份载荷，在 `<键>.lock` 上取得非阻塞排他文件锁，再检查存活服务与未领取回答；持锁至服务退出，进程死亡由操作系统释放锁，锁文件保留不删除。同键并发启动只允许一份服务发布状态，避免双服务覆盖状态与回答。取得启动权后绑定 `127.0.0.1:0`，把 `{pid, port, url}` 写进状态文件；带 `--open` 启动时调用 `open_system_browser`。
3. `--start` 每 100 毫秒看一次状态文件，最多 8 秒；只接受 PID 属于本次子进程的状态，再打印 §5.1 的四行并退出 0。子进程失败或超时则报错并收回该子进程，避免报错后留下迟到的服务。
4. 看门狗线程每 2 秒检查一次：一直没有心跳且超过 `--timeout`，或心跳停了超过 `--idle-grace`，进程以 2 退出。
5. 写入回答并应答成功后 150 毫秒，守护进程以 0 退出。

### 5.4 路由与响应头

每个请求先过 Host 白名单（`allowed_host`），不合格回 403；路径无法解析回 400。所有响应带 `Cache-Control: no-store`、`X-Content-Type-Options: nosniff`、`Referrer-Policy: no-referrer`。

| 方法与路径 | 行为 | CSP |
| --- | --- | --- |
| `GET /` | 外壳页：`page.html`，其中的数据占位换成转义后的 JSON（方案的 `id`、`name`、类型、展示用地址、`baseline`、`interactive`、`pixelRatio`、四项方案信息，以及 `title`、`question`、`lang`、`serve`、键）；转义写法同 map_inspection.rs:403-407 | 外壳策略 |
| `GET /compare.css`、`GET /compare.js` | 内嵌的样式（设计令牌与套件在前，`page.css` 在后）与脚本 | 外壳策略 |
| `GET /fonts/albertsans.ttf` 等三款 | 内嵌字体 | 外壳策略 |
| `GET /p/<序号>/<路径>` | 只对 HTML 方案：从该方案文件所在目录读取；按 §1.1 的写法拒绝越界、以 `.` 开头的路径段与指向目录外的符号链接，不存在回 404；内容类型按扩展名（html、css、js、json、png、jpg、jpeg、webp、gif、svg、woff、woff2、ttf、otf、mp4、webm，其余为 `application/octet-stream`）；字体额外带 `Access-Control-Allow-Origin: *` | 方案策略，按该方案是否 `interactive` 取其一 |
| `GET /img/<序号>` | 只对截图方案：返回该文件 | 外壳策略 |
| `GET /status?key=<键>` | 对每个本机地址方案做一次连通探测（§5.6），返回 `{"proposals":[{"id":…,"available":true\|false,"reason":…}]}`；键不对回 401 | 外壳策略 |
| `POST /heartbeat?key=<键>` | 过 `reject_detached_post`（键不对 401，Origin 不在白名单 403）；回 204；记下心跳时间，每 4 秒把 `lastBeat` 写回状态文件一次 | — |
| `POST /answer?key=<键>` | 同样的校验；请求体最多 64 KiB，必须是 JSON 对象；`proposalId` 必须是这次比较里的某个方案，否则 400；`liked`、`disliked` 取字符串并去掉首尾空白，缺省为空串；按 D9 组装回答，原子写入回答文件，写成功后才回 `{"ok":true}`，写失败回 500 | — |
| 其他 | 404 | 外壳策略 |

三种 CSP：

- 外壳策略：`default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-src 'self' <各本机地址方案的源，去重>; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`。CSP 的主机源写不了 IPv6 字面量，所以主机是 `[::1]` 的方案改用它的协议（`http:` 或 `https:`）放行，否则它的画面会被外壳自己的策略拦掉。
- 方案策略（默认）：`default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; media-src 'self'; script-src 'none'; connect-src 'none'; base-uri 'none'; form-action 'none'; sandbox; frame-ancestors 'self'`。
- 方案策略（`interactive`）：把上一条的 `script-src 'none'` 换成 `script-src 'self' 'unsafe-inline' 'unsafe-eval'`，`sandbox` 换成 `sandbox allow-scripts`；`connect-src 'none'` 不变。

外壳页里：HTML 方案的 iframe 带 `sandbox=""`（可交互的为 `sandbox="allow-scripts"`），地址是 `/p/<序号>/<文件名>`；本机地址方案的 iframe 不带 `sandbox`，带 `referrerpolicy="no-referrer"`。方案页面拿不到键：它在另一个源或没有同源身份的沙箱里，读不到外壳的文档，它发出的 POST 也过不了 Origin 校验。

### 5.5 等待

`--wait` 在 `--poll` 秒内每秒检查一次，顺序如下：

1. 回答文件存在：打印 `ANSWER:` 与可能的 `BASELINE KEPT:`，删除回答与状态文件，返回 0。
2. 状态文件里的进程已不存在且最近 12 秒没有心跳：返回 2。
3. 见过心跳且最后一次心跳在 15 秒之前：返回 4。
4. 到时：返回 3。

页面每 4 秒发一次心跳。页面被关掉后服务还会留到 `--idle-grace`，用户重新打开同一地址可以继续选；主代理按 D10 处理完后运行 `--stop`。

### 5.6 连通探测

对本机地址方案，`/status` 用 `TcpStream::connect_timeout` 连它的主机与端口（没写端口时 http 为 80、https 为 443），超时 800 毫秒：连上即可用；被拒绝或超时即不可用，`reason` 写成“localhost:5173 没有响应”这样的一句（主机与端口取该方案的地址）。不发 HTTP 请求、不读响应头。这是引擎在本命令里唯一主动发起的连接，目标只能是载荷校验过的回环地址。

## 6. 对比页界面、Skill 与文档增量

skill 源文件用英文写，与现文一致；下列为要表达的内容与落点。动这些文件前先核对 §0.6 对应行，改完运行渲染（C-8）。给用户的话术随会话语言。

### 6.1 对比页界面（`page.html`、`page.css`、`page.js`）

- **结构**：顶栏放标题、问题与一排工具：视图（并排 / 单个）、视口（桌面 1440×900 / 手机 390×844）、缩放（适应 / 100%，只在单个视图可用）；主体是方案区；底部一行键盘提示。视图、视口、缩放三组开关各是一个 `radiogroup`，写法照决策页的构建路径开关。
- **并排视图**（默认）：每个方案一栏，基线在最左；栏头是序号与名称，基线的序号位置显示「现状」标签；画面区；信息区依次是概念一句、字体、色板色块、特征标签，没填的不占位置；栏底是「选择」按钮，基线栏写「保持现状」；按钮的可访问名称带上方案名（「选择：方案 A」），多个按钮才分得开。每栏最小宽 400px，栏宽不足时方案区横向滚动。
- **单个视图**：一次只显示当前方案，上方一排方案标签用来切换；「适应」把画面缩到不超过舞台宽，「100%」不缩放，舞台出现滚动条。
- **画面与缩放**：HTML 与本机地址方案的 iframe 按所选视口的宽高设定元素尺寸，再用 `transform: scale(min(1, 栏宽 ÷ 视口宽))`、`transform-origin: 0 0` 缩到栏内（栏比视口宽时按原尺寸居中，不放大），外层容器的高度取“视口高 × 缩放比”；窗口尺寸变化时重算。截图方案并排时宽度撑满栏，100% 时宽度取“原图像素宽 ÷ pixelRatio”；截图不随视口开关变化，栏头注明「截图 · 按原图显示」。
- **状态**：iframe 加载完成前画面区显示占位底纹。本机地址方案先看 `/status`：不可用时不创建 iframe，显示「不可用：」加 `/status` 给的原因，有 `serve` 时再写「在（`serve.cwd`）运行（`serve.command`）后重试」，没有时写「启动你的开发服务器后重试」，并给「重试」按钮；只要还有方案不可用，页面每 5 秒再查一次，可用后自动载入。每个本机地址方案的栏头有「在新标签页打开」。提交后显示「已选择：」加方案名称与「可以关闭此页，回到对话。」并停用全部控件；提交失败显示「提交失败：对比页服务已停止，请回到对话重新打开」。
- **选择**：点「选择」打开一个模态面板：标题是所选方案的名称，两个可空的多行输入框「喜欢它什么」「不喜欢什么」，按钮「确认选择」与「取消」。确认后 POST `/answer`。
- **键盘**：焦点不在输入框时，左右方向键切换当前方案（并排视图里把它滚到可见并加焦点框，单个视图里换显示的方案），数字键 1–9 直接跳到对应方案，回车打开当前方案的选择面板，Esc 关闭面板。焦点落进某个方案的 iframe 后按键到不了外壳，底部提示写明“点一下页面空白处再用键盘”。
- **可访问性**：控件都是原生 `button`、`textarea`；当前方案带 `aria-current`；面板是 `role="dialog"`、`aria-modal="true"`，打开时焦点进第一个输入框，关闭时回到触发按钮；焦点框用 `--ks-focus-ring`；`prefers-reduced-motion` 下外壳没有过渡。
- **外壳的视觉**：只用套件里的纸色、墨色、分隔线与焦点色，字体为 Albert Sans（界面）与 JetBrains Mono（序号、尺寸），不用强调色块、渐变与装饰；色板色块用 CSSOM 给元素设背景色，浏览器不认识的值自然被忽略。
- **数据与通信**：页面数据放在 `<script type="application/json" id="compare-data">` 里，脚本从 `/compare.js` 加载；每 4 秒 POST `/heartbeat`；不使用本机存储。
- **文案**：一张中英两列的字符串表，按 `lang` 取用。

### 6.2 `reference/compare.md`★ 与 Routing 触发句

`SKILL.src.md` Routing 加一条：用户要求比较已做出来的方案，或一次交出两个以上需要用户选择的可运行整页方案（含改版前后）时，读 `reference/compare.md`；只有一个方案时不打开对比页。

`compare.md` 的内容：

- **什么时候用、什么时候不用**：整页、已构建、要用户选。构建之前选方向用方向轮的决策页；元素级变体用 live 或 generate；comp-led 的组件评审用 component-review。三者照旧。
- **准备方案**：各方案的内容、主动作与目标用户保持一致，变的是设计；每个方案一个代表画面。能跑就用真实页面：项目里的页面用本机地址；单独写的小样用 HTML 文件，各自放在 `.phoenix-ui/compare/variants/<id>/` 这样自成一体的目录里，资源用相对路径；拿不到可运行页面时才用截图（用 `capture` 截的图写上它的像素比）。改已有页面时把现状也放进来并标 `baseline`。需要脚本才能显示的小样标 `interactive`，它仍然不能联网；需要构建产物、模块脚本或以 `/` 开头的资源路径的页面用本机地址。有本机地址方案时写上 `serve`，并让开发服务器开着。
- **方案信息**：`concept` 写一句、`typography`、`palette`、`traits` 按实际填写，没有就不填。
- **命令**：`{{scripts_path}}/phoenix-ui compare --schema` 看载荷；`--start --payload <file>` 后为用户打开地址（宿主内置浏览器优先，其次系统打开方式，再次给出地址）；`--wait --key <key>` 取回选择，退出 3 时重复。绝不自己向页面提交选择。
- **读结果**：`ANSWER` 里的 `proposalId` 是用户选的方案，`liked`、`disliked` 是用户的原话，照此继续；`BASELINE KEPT` 表示保持现状，不改现有实现。
- **没有回答**：退出 4 时用结构化提问再问一次；仍无回答，有基线就保持现状并在交付里写明“待选”，没有基线就按你的推荐继续并写明“未经用户选择”；之后 `--stop`。退出 2 来自 `--start` 时（没有浏览器）直接走结构化提问，同样的兜底规则；来自 `--wait` 时用同一载荷重新 `--start`。
- **交付报告**：写明比较了哪些方案、用户选了哪个及其说明，或写明待选 / 未经用户选择。

### 6.3 `reference/vague-feedback.md`★ 与「How to design」

`SKILL.src.md`「How to design」加一条：用户对已有界面给出模糊评价（“再高级一点”“不够好看”“简洁些”）时，先读 `reference/vague-feedback.md`，回复“变量 → 预期变化”清单，用户回复后才动手。

`vague-feedback.md` 的内容：

- **先翻译**：动手前回复“要改的变量 → 预期变化”清单（固定格式，一行一个）并结束这一轮，用户回复（确认、删改或补充）后才动手；无人值守的运行只交付清单、不改界面；交付后逐项说明实际改了什么。评价同时对应某个精修命令（如“安静一点”对应 `quieter`）时，用户回复后进入该命令。宿主实测表明，不停下的话这份清单只会出现在可见思考摘要里（V-5 R1、V-7 A3 共五次），所以改为等回复。
- **翻译依据**（一张五行的表）：高级——先说清是哪一种，再定字体对比、分隔方式、间距尺度、色相数量；精致——公差：字距、网格对齐到几像素、阴影级数、色相上限；克制——饱和度预算（如强调色占面积比例）、动效幅度上限、色相上限；史诗——尺度：展示字大小、主图面积、明暗对比；简洁——密度档位，以及删什么、为什么删。
- **翻译不出来的词**：不作为改动理由，也不出现在交付报告里。
- **拿不准时**：同一次回复给两个具体的可选方向（宿主有结构化提问工具就用它），不再追问第二轮；或者做两个可比较的变体，元素级用 live，整页用对比页（指向 `compare.md`）。不自行随机换风格。
- **边界**：方向轮进行中用户说“更大胆”“更稳妥”，仍是方向轮的档位（new-work.md 现有规则），不进入这里；用户要求换方向时走既有的方向流程。

### 6.4 `reference/animate.md`

- 在「Choose material by meaning」之后新增 **Choreograph the change**：变化从被操作的元素出发；一次切换只朝一个方向；主角先动，被影响的元素晚 40–80ms 开始、幅度更小（被主角经布局推动的内容随它同步移动，延迟只用于各自响应的元素）；上一状态的元素变成下一状态的元素（按钮展开成面板、图标拉长成输入框）。
- 新增 **Transitions that keep the same content**：只改变尺寸或位置时（展开、筛选、切换视口、缩放）外框直接变形，内容立即显示新状态，不做交叉淡化；离开的淡出、加入的淡入、留下的滑到新位置，三者分开处理；选中指示用同一块滑块移到新位置，选项宽度不同时宽度一起过渡，首次显示、窗口缩放与字体加载完成后直接就位；目的地内容先加载或预渲染好再过渡；已是第一项或最后一项时轻顶 10–15px。
- 「Timing and easing」末尾新增 **Feel**：用“动作 + 实物”命名并给弹簧刚度/阻尼（六行表：咔嗒 350–400 / 10–15，滑入 250–300 / 20–25，流淌 150–200 / 30–40，砸落 400–500 / 5–10，回弹 300 / 8，漂浮 100 / 25，各附实物参照与适用处）；整页、面板等大面积元素过冲不超过 1–2%，徽章、勾选这类小物件可到 10–20%；弹簧可按刚度和阻尼采样 20–30 个点写成 `linear()`，不支持时退回相近的 `cubic-bezier`；回弹一行只在方向或 brief 支持时使用，现有的“不凭反射用弹跳或弹性曲线”一句保留。
- 新增 **Say the invisible result**：复制、后台保存、发送等看不见结果的操作完成后，用一条短提示说出结果本身（如“已复制：”后面跟被复制的那段文字），不只说“成功”。
- 「Accessibility and control」补一句：减少动态效果时各状态直接到位、不走位移，确认操作的反馈仍然可辨认。
- 「Verify」补三条：Web 上用 `{{scripts_path}}/phoenix-ui capture` 给该操作的状态加 `"frames": 8`，从序列图核对被影响元素晚于主角起动且方向一致、只变形状的过渡没有新旧两层内容叠在一起；序列帧只说明状态怎样变化，手感要按正常速度实际操作一遍、往回操作一次、快速连按几次来判断；没有实际操作核对时，交付报告写 `正常速度手感未核对`（英文会话写 `normal-speed feel not checked`）。这些核对在既有的两轮检查内完成。
- 文末新增 **Report**：交付报告为每处会改变状态的动效写明起点（被操作的元素）、方向与跟随顺序，以及手感的“动作 + 实物”名称和对应的弹簧参数或缓动。
- `craft-floor.md`:14 的 Motion 一条末尾加一句：给状态变化做动效时，按 animate.md 的编排与过渡规则。`new-work.md`:141「Author motion as material」一条末尾加同样的指向，并写明交付按 animate.md 的 Report 写。
- 「Implement to the runtime」补充状态与临时动画效果的收尾：完成/中断时恢复目标可见性并移除填充效果后再丢弃句柄，旧回调不能覆盖最新选择，减少动效切换时仍保持计数与内容一致。
- 不改：Visitor mode、Find the job、Set the motion thesis 各节，时长表，原生平台的指向。

### 6.5 `reference/craft-floor.md`

「Verify」在 Copy 一条之后加三条：

- **Demo data**：演示数据不出现整数化指标（1,000 位用户、+10%）、占位人名与公司（张三、李四、John Doe、示例公司、Acme）、每条都相同的相对时间、“标题”“描述”“Title”“Description”这类占位字、反复出现的“了解更多”“Learn more”；一个界面上的多处演示共用同一个虚构场景（同一个产品、同一批人、同一组数字）；按钮写出具体动作。可能被访客当真的内容仍标为示例，不虚构商业声明（new-work.md 的现有规则不变）。
- **Icons**：一个产品只用一套图标库；存量项目沿用已安装的库，新项目按这个世界的调性选，不默认 Lucide 或 Heroicons；描边粗细与同尺寸正文的字重相称，尺寸为字号的 1–1.25 倍并按视觉中心与文字对齐；不熟悉的图标配可见文字，纯图标按钮有可访问名称；核对许可（如 SF Symbols 不用于网页）。
- **Structure checks**：只在界面里出现对应结构时做，并在交付报告里分别写结果；没有的结构不做也不提。弹窗内有输入框：在手机视口聚焦它，确认关闭方式与主操作没有被键盘或固定栏挡住，并说明证据来自缩短高度的仿真视口还是真机。多屏导航：沿主导航连续走两屏再返回，确认位置与状态正确。内部滚动容器：分别滚动页面与容器，确认滚动落在该滚的区域，没有双滚动条或卡死。这些在既有的批量检查轮里完成。

「Refuse」里“Unicode glyphs or emoji standing in for an icon system”一条不动。

### 6.6 `reference/routing.md` 与 Routing 触发句

`SKILL.src.md` Routing 加一条：交付一个新界面、一次改版或一次命令执行之后，按 `reference/routing.md` 的「Next steps after a delivery」在答复末尾列下一步；小改动、停在用户确认点、用户已说明下一步或无人值守时不列。

`routing.md` 新增一节 **Next steps after a delivery**：

- 最多 3 条，带编号，第一条标“推荐”（英文会话标 Recommended）；用会话语言和用户的说法。
- 每条一句话：做完得到什么，以及代价——多花的时间或 Token、会新建或修改哪些文件、是否使用生图。代价没有实测就用粗略的说法，不编数字。
- 只列当前真能执行、且这次会话还没做过的 Phoenix 命令或流程（如 critique、polish、animate、adapt、把这一页的系统扩展到其他页面、再做一个方案用对比页比较）；可以沿用上文 signals 的判断（需要时运行一次 `{{scripts_path}}/phoenix-ui signals`，不为此再跑检测）；接后端、接真实服务等工程任务不列。
- 用户回复编号即进入对应流程，按该命令的参考执行；菜单本身不自动运行任何命令。
- 不列的四种情况同触发句。

### 6.7 `reference/new-work.md`

除 §6.4 的一句指向外，再改两处，其余不动：

- §3 讲决策页协议的一段不改；在 §7 交付要求的末尾（documenter 一段之后）加一句：交付按 routing.md 的「Next steps after a delivery」收尾。
- 「Both paths」的“Prove, don't claim”一条末尾加一句：演示数据按 craft-floor.md 的 Demo data 核对。

### 6.8 `SKILL.src.md` 的「Engine commands」表与复述终验

Commands 一节的设计子命令表之后加一张表，说明这些是流程内部调用的引擎命令，不是 `{{command_prefix}}phoenix-ui <子命令>`：

| Engine command | 用途 | 细则 |
| --- | --- | --- |
| `capture <page>` | 按步骤取多视口、多状态的截图并自检有效性；Web 完工轮的证据来源，有主要操作流程时同轮派出盲测走查 | new-work.md §7 |
| `detect <url>` 的页面规则 | 浏览器渲染后的页面检测，含以汉字为主文本的五条 `xgent/cjk-*` 规则 | craft-floor.md、typeset.md |
| `media <search\|preview\|fetch\|audit>` | 为设计稿与预览检索、取用开放授权素材，只进 `.phoenix-ui/media/` | 「Media requests」 |
| `prep-image <files>` | 按显示宽度的 2 倍导出交付图片并给出 `srcset` 片段 | optimize.md |
| `compare --start` / `--wait` | 并排比较已构建的整页方案，取回用户的选择 | compare.md |

复述终验矩阵（WORK-N4/AC-3）。“现状”列是 2026-11-12 的 grep 结果；“本期补”列是 M3、M4 要写的内容：

| 新命令 / 规则 / 流程变化（期次） | 总帮助 | SKILL.md | reference | docs/phoenix-ui.md | README「Phoenix UI」节 | 本期补 |
| --- | --- | --- | --- | --- | --- | --- |
| `capture`（P1） | 有 | 无 | new-work.md | 「取证命令」 | 无 | SKILL 引擎命令表；README 一条 |
| 完工取证默认用 capture、同轮盲测走查（P1） | — | 无 | new-work.md §7 | 「取证命令」内 | 无 | SKILL 引擎命令表的 capture 行；README 同一条 |
| `xgent/cjk-*` 五条规则与中文排版口径（P2） | — | 无 | typeset.md、craft-floor.md | 「XGENT 自有检测规则」 | 无 | SKILL 引擎命令表的 detect 行；README 一条 |
| `media`（P3） | 有 | 「Media requests」 | new-work.md、visualize.md | 「素材命令」 | 有 | SKILL 引擎命令表一行 |
| `prep-image`（P3） | 有 | 无 | optimize.md、visualize.md | 「交付图片处理」 | 有 | SKILL 引擎命令表一行 |
| `compare`（P4） | 本期加 | 本期加 | compare.md★ | 本期加 | 本期加 | 全部 |
| 模糊反馈翻译（P4） | — | 本期加 | vague-feedback.md★ | 本期加 | 本期加 | 全部 |
| 动效编排、过渡、手感与结果提示（P4） | — | Commands 表已有 `animate` | animate.md | 本期加 | 本期加 | docs、README |
| 演示数据、图标与三类补验（P4） | — | Setup 第 3 步已要求读 craft-floor | craft-floor.md | 本期加 | 本期加 | docs、README |
| 交付后的下一步（P4） | — | 本期加 | routing.md | 本期加 | 本期加 | 全部 |

### 6.9 `docs/phoenix-ui.md`、`README.md`、`ENGINE.md`

- `docs/phoenix-ui.md` 在「交付图片处理（prep-image）」之后新增「方案对比页（compare）」：用途与边界（已构建的整页方案；不替代方向轮决策页、live 与 component-review；不评价、不写项目文件）；三种方案来源与各自的限制（HTML 方案只能用自己目录里的相对资源，默认不执行脚本、不向对比页服务之外发请求，`interactive` 允许内联与同目录的经典脚本但仍不能联网；本机地址只接受回环主机，开发服务器要开着，拒绝被嵌入的页面用「在新标签页打开」或改用截图；截图按原图显示，可写像素比）；载荷字段与一个示例；四个动作与退出码；页面上能做什么（并排与单个、桌面与手机、100%、键盘、选择与说明）；无回答时的处置；状态目录；没有浏览器的会话。
- 同文件新增「设计指引增补」：模糊评价先翻译成变量、动效的编排与核对口径、演示数据与图标规则、三类结构的补验、交付后的下一步，各一两句并指向 skill 里对应的参考。
- 同文件「联网边界」表加一行：`compare` 只监听 `127.0.0.1`，只对载荷里的本机地址做 TCP 连通探测，不发起外网请求；方案页面自身的对外请求被浏览器按响应头拦下，标了可交互的也一样；本机地址方案是用户自己的开发服务器页面，它加载什么由它自己决定。
- `README.md` 的「Phoenix UI」一节在「素材与图片」一条之后加三条：**取证与走查**（`phoenix-ui capture` 与 `phoenix-ui-ux-walker`，Web 完工轮的默认证据与同轮盲测走查）、**中文排版**（`detect <url>` 的五条 `xgent/cjk-*` 规则与 typeset 的中文口径）、**方案对比与设计指引**（`phoenix-ui compare`；模糊评价翻译、动效编排、演示数据与图标、交付后的下一步），细节指向 docs/phoenix-ui.md 的对应小节。
- `tools/phoenix-ui/docs/ENGINE.md` 的 crate 说明里 `context` 一行补上 serve-question 与 compare。

### 6.10 第三方声明与 evals

- `tools/phoenix-ui/NOTICE.md` 的 oil-ui 段补一句：`phoenix-ui compare`（`crates/context/src/compare/`）改编其对比页的方案字段、基线标记与查看方式；phoenix-ui skill 的 animate、craft-floor、routing 与 vague-feedback 参考改编其动效编排与手感、演示数据与图标、交付后下一步与模糊词翻译的方法，均为重写。`tools/phoenix-ui/THIRD-PARTY-NOTICES.md` 的 oil-ui 条目把改编范围写全，MIT 全文与 “Copyright (c) 2026 oil-oil” 不动。重算在 M5。
- `skills/phoenix-ui/evals/scenarios.md` 在现有 24 条之后追加 6 条（写法照第 15–24 条，含失败信号）：
  - 两个整页改版方案让用户选：应把现状标为基线，用 `compare --start` 打开对比页并用 `--wait` 取回选择，按用户选的方案继续；失败信号：自己替用户选、只有一个方案也打开对比页、让用户复制粘贴选择。
  - 对比页没有回答：应再用结构化提问问一次，仍无回答时有基线保持现状并写明待选；失败信号：当作选了推荐方案而不说明。
  - 用户只说“再高级一点”：应先回复要改的变量与预期变化清单，等用户回复后再动手，交付逐项说明；拿不准时在同一次回复里给两个具体选项或两个变体；失败信号：回复前直接改动、直接换一套风格、交付里用“更高级”作理由。
  - 给面板展开加动效：应从被操作的元素出发、其余晚一拍跟随，只变尺寸的过渡不交叉淡化，取序列帧核对，交付写起点、方向、跟随顺序与手感名称及参数，未实际操作时标“正常速度手感未核对”；失败信号：所有元素同时同幅度起动、新旧内容重影、凭序列帧宣称手感已核对。
  - 新建带弹窗表单与内部滚动列表的页面：演示数据不出现占位反例且共用一个场景，可能被当真的内容在页面标为示例，图标来自同一套库，交付分别给出键盘遮挡与滚动落点的核对结果，不出现两屏往返检查；失败信号：张三李四与整数指标、混用图标库、没有该结构却写了核对结果。
  - 一次改版交付之后与一次小改动之后：前者答复末尾有不超过三条带编号的下一步，第一条标推荐，每条带代价；后者不列；失败信号：列出接后端等工程任务、编造耗时数字、小改动也列菜单。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N2 | PRD WORK-N2/AC-4（AC-1–AC-3 已在 P1、P3 交付） | `compare` 在四平台可用；缺 engine 给补装命令；其他平台仍在安装预检停止 | engine 缺失退出 4 | D1；启动器与安装器零改动 | V-6（本机冒烟）、V-8（安装器无 diff）、V-9（四平台） |
| WORK-N4 | PRD WORK-N4/AC-1–AC-3 | 复述齐全；改 skill 与命令前清单到位、diff 对得上；声明覆盖新的改编范围 | 出现清单外的行为差异时先更新 §0.6 再继续 | §0.6、§6.8–§6.10 | V-4、V-8 |
| WORK-N1 | PRD WORK-N1/AC-4（已终验，持续） | 新命令不给引擎带来外网请求 | — | §5.6：只连回环地址；服务只监听 `127.0.0.1` | V-2（非回环地址被拒）、V-8（`compare/` 里没有 HTTP 客户端调用） |
| WORK-N3 | PRD WORK-N3/AC-1–AC-2（已终验，持续） | 既有流程与四个处置词不变 | — | D14、D17 | V-6、V-7 A4、V-8 |

安全项：

- **谁能提交选择**：只有带这次比较的键、Host 与 Origin 都是本服务的 POST；方案页面拿不到键（§5.4）。
- **文件提供范围**：HTML 方案只提供其所在目录内的文件，拒绝越界、点文件与出界的符号链接；截图方案只提供那一个文件；载荷里的路径必须在项目目录内。
- **方案页面的能力**：默认不执行脚本、不能向对比页服务之外发请求；可交互的可以执行脚本，仍不能发请求、不能读外壳。
- **监听范围**：只绑 `127.0.0.1`；Host 白名单挡住 DNS 重绑定。
- **展示的数据**：方案名称与信息作为文本插入页面（`textContent`），色值经 CSSOM 赋值；页面数据以转义后的 JSON 数据块给出。

可观测性：命令的全部事实在输出行、退出码与 `.phoenix-ui/compare/<键>.log` 里；不新增日志通道。

## 8. 失败模式、发布与回滚

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现/降级 | 检测 | 恢复/补偿 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| 别的页面或进程向本机服务伪造一次选择（用户浏览器里任何网页都能向 `127.0.0.1` 发不读响应的 POST） | 当前这轮比较 | 主代理按不是用户做的选择改代码 | 请求被拒，页面仍在等用户 | 键、Origin、Host 三道校验 | §5.4；沿用决策页的同一组函数 | V-2 |
| HTML 方案里的脚本或外链在对比页里执行、对外发请求（生成的小样带统计脚本、外链字体、远程图片是常态） | 当前这轮比较 | 无数据后果；用户只是想看方案，却触发了对外请求与脚本 | 方案照常显示，脚本不跑，外链资源不加载 | 浏览器按响应头拦截；网络侧可见 | §5.4 的方案策略与 iframe 的 `sandbox` | V-3 |
| 载荷或请求路径指向方案目录之外的文件（`../`、编码后的点段、指向外面的符号链接） | 当前项目所在的机器 | 项目外的文件被本机服务提供出去 | 载荷校验失败退出 1，或该请求 404 | 校验与目录内读取 | §5.2 第 3 条、§5.4 | V-2 |
| 页面被关掉或服务中途退出，没有选择（用户关了标签页；宿主回收了后台进程） | 当前这轮比较 | 无；风险是主代理把“没选”当成“选了” | `--wait` 以 4 或 2 返回并写明下一步 | 心跳与进程探测 | §5.5、D10 | V-1 |

- 没有列入的情况按最简行为处理：端口绑定失败时 `--start` 报错并给日志位置，由主代理换不受限的命令通道重跑；同一键的并发 `--wait` 互不协调，先到的取走回答；开发服务器在比较过程中停掉时，已载入的画面保持原样，下次探测把它标为不可用。
- 发布顺序：本期只到开发候选（本机开发构建 + 四平台 CI 制品），不发 npm/R2。引擎源码已变，发布须走 CI 构建，不能用 `--reuse-engines`。
- 放量/切换门：无；新命令只在调用时生效，skill 文本随安装生效。
- 回滚：回退对应提交并重新渲染 skill；`.phoenix-ui/compare/` 只有一次比较的临时文件，没有数据或配置迁移。

## 9. 验证

### 9.1 行为与接口验证

共同环境：仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；Cargo 命令在 `tools/phoenix-ui` 下运行；本机 macOS 有可发现的 Google Chrome（可用 `PHOENIX_UI_BROWSER` 指定）；浏览器测试不放进 `sandbox-exec`；场景 E2E 的项目目录建在仓库树之外的临时目录并 `git init`。看对比页一律用 `phoenix-ui capture <对比页地址>` 截图后由 Agent 开图查看，替用户点选也用 capture 的步骤（外壳控件都在主文档里）；宿主有浏览器工具时再在真实窗口里操作一遍。

批内最小反馈：`mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` 构建后，直接运行 `target/debug/phoenix-ui compare …` 复现单个问题；纯逻辑用 `mise exec --no-deps -- cargo test -p impeccable-context --lib compare::payload --locked --offline`。两者都只在有具体失败或疑问时运行。`--locked` 构建失败说明锁文件要变，违反 C-6，停下核对。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **设计负责人要在现状和三个改版方案里选一个。** 角色：只读过 `phoenix-ui compare --help` 与 `--schema`，不读源码。夹具（临时项目）：`site/` 是现状页面，由 `python3 -m http.server 4173 --bind 127.0.0.1 --directory site` 提供；`variants/a/` 是带样式表与本地图片的静态小样，页面里另有一段会改标题与底色的内联脚本和一张 `https://example.com/x.png`；`variants/b/` 是靠内联脚本渲染列表的小样；`shots/c.png` 是用 `capture … --viewport mobile --dpr 2` 截的图。步骤与结果：①按示例写载荷（现状为基线、A 不标可交互、B 标可交互、C 写 `pixelRatio: 2`，A 填全四项方案信息、B 只填概念），`compare --start`：打印地址与键（WORK-F10/AC-1）；②对地址截 1920×1080 的图：四栏并排、现状在最左并有「现状」标签、A 的四项信息齐全、B 只显示概念、A 的样式与本地图片在、脚本没有改它的底色、外链图片位置为空、B 的列表已渲染（AC-1、AC-5）；③加步骤点「手机」后截图：A、B 与现状变为 390 宽的布局，C 不变（AC-2）；④加步骤点「单个」「100%」后截图：当前方案按 1440 宽不缩放显示、舞台可滚动；再按两次右方向键截图：换到了第三个方案（AC-2）；⑤加步骤点 B 的「选择」、在「喜欢它什么」里填一句话、点「确认选择」后截图：页面显示已选择；随后 `compare --wait --key <键>` 退出 0，`ANSWER` 里的 `proposalId` 是 B、`liked` 是填的那句话；状态与回答文件已删除，地址不再可达（AC-3）；⑥重新 `--start` 后只截一张初始图（页面打开后即关闭），`compare --wait --key <键> --poll 30` 退出 4，输出含再问一次与保持现状的规则；`--stop`（AC-4）；⑦停掉 `http.server` 后 `--start` 并截图：现状栏显示不可用与载荷 `serve` 里的启动说明，其余三栏照常；重新启动它后用一个等「不可用」消失的步骤截图：现状已载入（AC-5）；⑧选基线：`--wait` 的输出多一行 `BASELINE KEPT`；⑨错误输入各试一次：只有一个方案、两个基线、`url` 为 `http://example.com/`、`source` 不存在、`source` 为 `../outside.html`——一次运行列出全部问题，退出 1，没有起服务（AC-6 的“单方案不弹出”）；⑩`CI=1 compare --start …` 与 `PHOENIX_UI_QUESTION_DISABLED=1 compare --start …` 都退出 2 并提示改用结构化提问。Agent 逐张打开截图查看，并核对外壳没有抢方案的视觉 | `target/debug/phoenix-ui compare …` 与 `target/debug/phoenix-ui capture <地址> --viewport wide=1920x1080 --states <状态文件> --out <目录>`（debug 二进制的绝对路径），在临时项目目录里运行 | M1–M2 交付齐备后（批 1）；M3 的 V-5 的前置 |
| V-2 | 局部 | 准入：载荷校验是分支多的纯逻辑；伪造选择与目录越界是 §8 列出的高后果路径，照用户操作的 E2E 触发不到。内容：①`compare::payload` 的校验矩阵（§5.2 七条各一正一反，含 `[::1]`、带端口与不带端口的地址、大写扩展名、真实路径在项目外的符号链接、基线被排到第一）；②`crates/cli/tests/compare_security.rs`★：直接以 `--detached-serve` 起服务，用原始套接字断言：不带键的 `/answer` 回 401、Origin 是别的站点回 403、Host 是别的域名回 403、带键且来源合法回 200 并写出回答文件、`proposalId` 不是这次比较里的方案回 400、`/p/0/../x`、`/p/0/%2e%2e/x`、`/p/0/.env` 回 404、截图方案的序号请求 `/p/` 回 404、`/status` 不带键回 401 | `mise exec --no-deps -- cargo test -p impeccable-context --lib compare --locked --offline`；`mise exec --no-deps -- cargo test -p phoenix-ui --test compare_security --locked --offline -- --test-threads=1`；仓库根 `node scripts/check-phoenix-lints.mjs` | M1–M2 交付齐备后（批 1） |
| V-3 | 局部 | 准入：WORK-F10/AC-5 是明示的隔离要求，“脚本没有执行、请求没有发出”从页面外观看不出来，而且各平台浏览器的行为是 A-2 的假设。`crates/cli/tests/compare.rs`★（无浏览器时跳过，写法同 capture 测试）：测试里起一个只记录首行的本机 TCP 监听作为探针；两个 HTML 方案的页面底色都是纯红，都带一段把底色改成纯绿、并向探针发 `fetch` 与图片请求的内联脚本，第二个标 `interactive`。用 `capture` 以单个视图 100% 各截一张：第一张的画面中心是红色（脚本没跑），第二张是绿色（脚本跑了）；探针全程没有收到任何请求；再用 capture 的步骤选第二个方案并确认，`compare --wait` 退出 0 且 `proposalId` 正确 | `mise exec --no-deps -- cargo test -p phoenix-ui --test compare --locked --offline -- --test-threads=1` | M2 交付齐备后（批 1） |
| V-4 | 局部（静态） | §6.2–§6.10 的文本落位与 §0.6 对得上（WORK-N4/AC-2、AC-3，C-7、C-8）：渲染为 `fresh`；生成的 compare.md、vague-feedback.md、animate.md、craft-floor.md、routing.md、new-work.md 与 SKILL.md 里有对应内容；`git diff --stat` 的 skill 源改动只有 §1.2 列出的文件；子代理定义无 diff；docs/phoenix-ui.md 有两节新内容与联网边界的新行；README 有三条新内容；`phoenix-ui --help` 列出 compare；§6.8 矩阵的每一格都能在对应文件里找到；phoenix-ui skill 的元数据测试通过，其他 skill 的既有失败只报告 | `tools/phoenix-ui` 下 `mise exec --no-deps -- bun scripts/render.mjs`，再 `mise exec --no-deps -- bun scripts/render.mjs --check`；仓库根 `grep -n 'compare\|capture\|prep-image\|cjk\|vague-feedback' skills/phoenix-ui/SKILL.md README.md docs/phoenix-ui.md`、`grep -n 'Choreograph\|Feel\|invisible result' skills/phoenix-ui/reference/animate.md`、`grep -n 'Demo data\|Icons\|Structure checks' skills/phoenix-ui/reference/craft-floor.md`、`grep -n 'Next steps after a delivery' skills/phoenix-ui/reference/routing.md skills/phoenix-ui/SKILL.md`、`git diff --stat -- skills/phoenix-ui/src docs README.md tools/phoenix-ui/docs`、`git diff -- skills/phoenix-ui/src/subagents`；`python3 -m unittest discover -s test/skills -p 'test_skill_metadata.py'`，只按 phoenix-ui 的子测试判定 | M3–M4 交付齐备后（批 2） |
| V-5 | 场景 E2E（有界演练） | **用户在一个已有的中文订单管理页面上连续提几种要求。** 角色：Claude Code 新会话，只有项目文件与已安装的 Phoenix skill。项目：仓库树外的临时目录，含 PRODUCT.md、DESIGN.md、一个带筛选面板、“复制订单号”按钮与订单表格的静态页面（由 `python3 -m http.server` 提供）；`.claude/` 用渲染产物 `tools/phoenix-ui/dist/providers/claude-code/.claude/` 的副本，其中 `skills/phoenix-ui/scripts/phoenix-ui` 换成调用本地 debug 二进制的垫片（仅演练）。逐个请求各一次调用，按会话记录、最终答复与文件判定。**R1**“这个订单页再高级一点”：会话先回复要改的变量与预期变化的清单并结束这一轮，此时界面文件没有改动；以 `--resume` 回复确认后才修改，交付逐项说明实际改动，没有换一套风格，“更高级”没有作为理由出现（WORK-F12/AC-1、AC-4）。**R2**“给订单页出两个整页改版方案，让我比较后选一个”：写了含现状基线的载荷并 `compare --start`，没有自己提交选择；操作者（执行本检查的 Agent）从 `.phoenix-ui/compare/<键>.state.json` 读到地址，用 capture 的步骤替用户选第二个方案并附一句说明；会话的 `--wait` 取到 `ANSWER` 后按所选方案继续，交付写明用户的选择与说明（WORK-F10/AC-3、WORK-F12/AC-2）。**R3** 同 R2 的请求，但会话环境带 `PHOENIX_UI_QUESTION_DISABLED=1`：没有起对比页，转为结构化提问；得不到回答时保持现状，交付写明待选，没有自行挑一个方案实现（WORK-F10/AC-4）。**R4**“给筛选面板的展开加上动效，复制订单号要有反馈”：交付写明起点、方向与跟随顺序和手感的“动作 + 实物”名称及参数；状态文件里该操作带 `"frames": 8` 并有序列图；序列图里面板从被操作的按钮处展开、面板内各自响应的内容不早于面板起动、被面板推动的表格随它同向移动、没有重影；复制后的提示说出了被复制的内容；有减少动态效果的路径；没有实际操作时交付带“正常速度手感未核对”（WORK-F11/AC-1–AC-5）。**R5**“新建一个客户列表页，带新建客户的弹窗表单”：演示数据没有占位反例且共用一个场景，可能被当真的内容已在页面标为示例（不留给用户决定），图标来自同一套库且纯图标按钮有可访问名称，交付分别给出键盘遮挡与内部滚动的核对结果、没有两屏往返一项，答复末尾有不超过三条带编号的下一步、第一条标推荐、每条带代价、没有工程任务（WORK-F13/AC-1–AC-3、WORK-F14/AC-1、AC-3）。**R6**“把保存按钮的文字改成‘保存客户’”：答复末尾没有下一步菜单（WORK-F14/AC-2）。某个请求不符合时，先核对 skill 文本是否有歧义并修订、重新渲染，再只重跑这个请求 | `claude -p "<请求>" --output-format stream-json --verbose`（每个请求一次，同一项目目录），权限参数按 `claude --help` 实际选择并只授予本场景所需；R2 的会话放在后台运行，操作者轮询状态文件后执行 `target/debug/phoenix-ui capture <地址> --states <选择步骤> …` | M3–M4 交付齐备且 V-1 通过后（批 2） |
| V-6 | 阶段收口 | 构建、许可与 Phoenix UI 全量回归：`Cargo.lock` 无变化；`--write` 后清单检查通过，新页面资源在清单里，NOTICE 与 THIRD-PARTY-NOTICES 的 oil-ui 条目含本期改编范围并进入 `dist/phoenix-ui`（WORK-N4/AC-1）；oracle 新增 compare 的不起浏览器用例（`--help`、`--schema`、缺 `--payload`、未知选项、校验失败的载荷、无头环境下的 `--start`、`--wait` 缺键、`--wait` 时服务不在、预置回答文件的 `--wait`、预置未领取回答时同键 `--start` 被拒、预置过期心跳的 `--wait`、选基线的回答）并重录 `cli-help`，其余 golden 逐字节不变（决策页、component-review、live 的用例不变，证明 D14 没有改变它们的行为，WORK-F10/AC-6、WORK-N3）；开发构建与冒烟通过，冒烟新增：经启动器 `compare --help` 退出 0；空 `PHOENIX_UI_HOME` 下 `compare --help` 退出 4 并给补装命令；带 `PHOENIX_UI_QUESTION_FORCE=1` 对两个内嵌小 HTML 文件 `compare --start`，用 Node 的 `http` 取到外壳页，再带键与 Origin POST `/answer`，`compare --wait` 退出 0 且回答正确（记录为 `answered`）；只有一个方案的载荷被拒（WORK-N2/AC-4 的开发候选证据）；`test/phoenix-ui` 下的 Node 测试与 `test:phoenix` 全绿；render `--check` 为 `fresh`。不以整仓 `npm test` 为门 | `tools/phoenix-ui` 下：`mise exec --no-deps -- node ../../scripts/prepare-phoenix-ui.mjs --prepare` 并按回执导出环境变量；`mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --write`，再不带参数运行一次；重建 debug 二进制后在仓库根运行 `node tools/phoenix-ui/tests/oracle/record.mjs --bin tools/phoenix-ui/target/debug/phoenix-ui compare`、`… cli-help`（逐个审阅 golden diff）；清空 `../../local/phoenix-ui/development-release` 后 `mise exec --no-deps -- node ../../scripts/build-phoenix-ui.mjs --development --output-dir ../../local/phoenix-ui/development-release`；`mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --distribution-dir ../../local/phoenix-ui/development-release`；`mise exec --no-deps -- node ../../scripts/smoke-phoenix-ui.mjs --release-dir ../../local/phoenix-ui/development-release --target darwin-arm64 --stage-runtime`（`--target` 填本机平台）；仓库根 `mise exec --no-deps -- node --test 'test/phoenix-ui/*.test.js'`、`mise exec --no-deps -- npm run test:phoenix`；`mise exec --no-deps -- bun scripts/render.mjs --check`。`test:phoenix` 含 release 构建与全量 oracle，耗时未知，只在收口运行一次 | M5，M1–M4 完成后 |
| V-7 | 真实使用验收 | 干净的夹具项目（仓库树外）、只凭公开入口与文档、用 V-6 的开发构建安装，逐项保存输出并亲自查看产物。**A1 直接使用 compare**（WORK-F10/AC-1–AC-5；PRD UAT-12）：以开发者身份只读 docs/phoenix-ui.md 的「方案对比页」一节，经安装后的启动器比较一个真实的小项目（自己准备：一个用 Vite 或同类开发服务器跑着的现状页面、两份自成目录的改版 HTML、一张截图）；带 `--open` 在系统浏览器里打开，宿主有浏览器工具时在真实窗口里切视口、看 100%、用键盘切换并选定，没有时用 capture 完成同样的操作并开图查看；另做一次关页不选；记录开发服务器是否能被嵌入（A-1）。判据同 UAT-12：收到的标识与所选一致，关页无选择时没有代码被改。**A2 宿主会话的完整改版**（WORK-F10/AC-3、WORK-F11、WORK-F13、WORK-F14；PRD UAT-12、UAT-15、UAT-16、UAT-18 的 P4 部分）：Claude Code 新会话，请求“把这个任务列表页改版，出两个方案让我比较后选一个，选定的方案把列表筛选做成带动效的”；操作者在对比页上选一个；判据：选择直接回到会话、没有复制粘贴；交付含动效的起点、方向、跟随顺序与手感标注；演示数据与图标符合 §6.5；答复末尾的下一步符合 §6.6。**A3 宿主会话的模糊评价与小改动**（WORK-F12、WORK-F14/AC-2；PRD UAT-13、UAT-16）：同一项目的新会话先说“再精致一点”，再说一处文字小改；判据同 V-5 的 R1、R6。**A4 既有页面不受影响**（WORK-F10/AC-6、WORK-N3）：用 `serve-question --schema` 的示例载荷 `--start` 一次决策页，截图查看并用 capture 的步骤选一张卡，`--wait` 取到 `ANSWER`。发现的用户可见问题修复后只复跑受影响的场景；改了引擎或 skill 源须从 V-6 的构建重做再复验受影响项 | 安装：在夹具项目里用仓库的安装入口加 `--release-dir=<仓库>/local/phoenix-ui/development-release`（docs/phoenix-ui.md「安装」选项表）；A1、A4 用 `npm exec` 形式或安装后的 `skills/phoenix-ui/scripts/phoenix-ui`；A2、A3 用 `claude -p … --output-format stream-json --verbose` | M5，V-6 通过后 |
| V-8 | 阶段收口 | 整合清单、复述终验与跨期约束核对（WORK-N4/AC-2、AC-3，C-1，WORK-N1、WORK-N3 持续）：§0.6 每个采用或调整项都有落点、拒绝项无改动；独立 skill、子代理定义、`crates/context/src/question_page.rs`、`crates/context/src/component_review`、`crates/live/src`、`crates/comp`、`crates/comp-verbs`、`crates/hook`、`crates/browser`、`crates/install`、`crates/skills`、启动器脚本无 diff；`serve_question.rs` 的 diff 只有可见性关键字；`compare/` 里没有 `ureq` 或其他 HTTP 客户端调用，唯一的出站连接是 §5.6 的回环探测；§6.8 矩阵逐格核对：每个新命令在总帮助、SKILL.md、对应 reference、docs/phoenix-ui.md、README 都能找到，每条流程变化在 SKILL.md 或其指向的 reference、docs 与 README 都能找到；第三方声明写全改编范围 | `git diff 8a49b86f --stat -- skills/review-ui skills/apply-ui-review skills/feature-demo skills/design-reference skills/ui-pattern-research skills/design-md skills/ui-loop skills/phoenix-ui/src/subagents skills/phoenix-ui/src/scripts tools/phoenix-ui/crates/context/src/question_page.rs tools/phoenix-ui/crates/context/src/component_review tools/phoenix-ui/crates/live/src tools/phoenix-ui/crates/comp tools/phoenix-ui/crates/comp-verbs tools/phoenix-ui/crates/hook tools/phoenix-ui/crates/browser tools/phoenix-ui/crates/install tools/phoenix-ui/crates/skills tools/phoenix-ui/Cargo.lock bin` 为空（开工时 HEAD 不是 `8a49b86f` 的，以实际开工基线替换并记入 M1 记录）；`git diff 8a49b86f -- tools/phoenix-ui/crates/context/src/serve_question.rs` 逐行查看；`grep -rn 'ureq\|agent_builder' tools/phoenix-ui/crates/context/src/compare` 无结果；`grep -n 'capture\|cjk\|media\|prep-image\|compare' skills/phoenix-ui/SKILL.md README.md docs/phoenix-ui.md tools/phoenix-ui/crates/detect/src/lib.rs tools/phoenix-ui/NOTICE.md tools/phoenix-ui/THIRD-PARTY-NOTICES.md` 对照 §6.8；`git diff 8a49b86f -- skills tools/phoenix-ui docs README.md scripts` 逐项对照 §0.6 与 §1.2 | M5，V-7 之后 |
| V-9 | 阶段收口 | 四平台开发候选（WORK-N2/AC-4、A-2）：同一候选提交上 Phoenix CI 的 UI 四个平台任务与 Node 18 冒烟全部成功（CI 记录为 `distribution=development`，证明四平台构建、暂存安装与运行；最终 npm 包安装与旧版升级由既有发布检查覆盖，本期不执行也不宣称）；四个平台的冒烟记录里 compare 为 `answered`；`compare_security` 与 `compare` 两个测试在四个平台运行，浏览器级测试在哪个平台被跳过就把该平台的 A-2 记为未验证并在收口注明。Diagram 任务的失败与本期无关时只报告 | 经用户授权推送 main 后：设 `candidate_sha=<M5 候选提交的完整 SHA>`，运行 `gh run list --workflow phoenix.yml --branch main -L 100 --json databaseId,headSha,status,conclusion --jq "map(select(.headSha == \"$candidate_sha\"))"`，逐条确认 `headSha` 相同后对匹配的 `databaseId` 执行 `gh run watch <run-id>`，下载各平台的 `phoenix-ui-<platform>` 制品并读取其中的 `runtime-smoke-node*.json` 核对冒烟记录，Node 18 读 `phoenix-ui-node18-consumer-evidence` 里的 `runtime-smoke-node18.json`（`phoenix-ui-<platform>-evidence` 只含准备与阶段结果，不能替代冒烟记录）；记录候选 SHA、run ID 与四平台结果 | M5，V-6–V-8 通过并完成候选提交后；未获授权时记入当前阻塞，不记完成 |

### 9.2 集成与回归

- `test:phoenix` 只在 V-6 运行一次；它的工作区测试段与浏览器测试段覆盖决策页、component-review、live 与 capture 的既有测试，oracle 段逐字节比对既有 golden，是 D14 可见性改动的回归门。
- `skills/phoenix-ui/src` 的改动在 M3、M4 各渲染一次并提交生成物，render `--check` 在 V-4 与 V-6 各看一次（V-6 是开发构建之后）。
- oracle 的 `context.mjs` 里决策页的用例（:691-705）在 V-6 里应逐字节不变；有差异即是回归。

### 9.3 真实环境

V-1 在本机由 Agent 执行并亲自看图；V-5、V-7 的 A2、A3 需要已登录的 Claude Code；V-7 的 A1 需要一个能跑开发服务器的小项目（Node 已是既有前置）；V-9 需要用户授权推送。缺任一环境时对应检查记“环境缺失/未运行”，收口不宣称相应 AC 通过。V-5、V-7 里由 Agent 充当操作者替用户点选，记录为角色模拟，不代表真实用户的选择偏好。

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 对比页服务与回传协议 | §0.6 I-1 已核对 | 批 1 前半。`crates/context/src/compare/` 的 `mod.rs`（§5.1、§5.3、§5.5）、`payload.rs`（§5.2）、`server.rs`（§5.4、§5.6，先用一张只显示方案名称与提交按钮的最简页面打通）★；`serve_question.rs` 的可见性（§1.2 所列函数）；`lib.rs` 导出；main.rs 的 `compare` 分支（写在 :74 之后）；`ROOT_USAGE` 一行；`crates/cli/tests/compare_security.rs`★。同构：参数读取、守护进程启动、等待循环与回答写入照 crates/context/src/serve_question.rs:389-650、:674-786、:825-932；响应头与三种策略的写法照 crates/context/src/component_review/server.rs:17-34、:114、:243；目录内读取照 crates/cli/src/capture/serve.rs:70-101、:149-160；原始套接字测试照 crates/cli/tests/serve_question_security.rs。`serve_question.rs`、`lib.rs`、main.rs、`ROOT_USAGE` 由主 agent 单人修改 | 与 M2 同批，V-1、V-2、V-3；回写「实施进度」 |
| M2 | 对比页界面 | M1 的路由与页面数据格式（§5.4 的 `GET /`） | 批 1 后半，连续实施。`page.html`、`page.css`、`page.js`★（§6.1）与 `server.rs` 里对它们和套件、字体的内嵌；`crates/cli/tests/compare.rs`★。同构：套件与字体的内嵌、数据块的转义照 crates/comp-verbs/src/map_inspection.rs:403-431 与 `map_inspection.html` 的页头写法；开关组与心跳照决策页模板（crates/context/src/question_page.rs 的 `bp-switch` 与 `beat()`）；浏览器级测试的跳过与二进制调用照 crates/cli/tests/capture.rs:23、:68-88。页面三个文件的可写路径与 M1 的 Rust 文件不重叠，可交给 subagent 与 M1 并行开发，接口以 §5.4 与 §6.1 为准 | 批末 V-1、V-2、V-3 通过；回写「实施进度」 |
| M3 | 四组指引与对比页用法（skill） | §0.6 I-2–I-6 已核对；命令接口以 §5 为准，文本开发不依赖批 1 完成；V-5 需 V-1 通过 | 批 2 前半。`reference/compare.md`★、`reference/vague-feedback.md`★、`animate.md`、`craft-floor.md`、`routing.md`、`new-work.md` 与 `SKILL.src.md` 的「How to design」一条和 Routing 两条（§6.2–§6.7）；运行渲染并提交生成物。同构：new-work.md:55 对决策页命令的写法、:147 对 capture 命令行的写法；typeset.md 的「Chinese copy」一节作为“在现有参考里加一组规则”的样子。可写路径 `skills/phoenix-ui/src/reference`，可交给 subagent；`SKILL.src.md` 由主 agent 单人修改 | 与 M4 同批，V-4、V-5；回写「实施进度」 |
| M4 | 复述终验、文档与声明文字 | §0.6 I-7 已核对；M3 的参考文件名已定 | 批 2 后半，连续实施。`SKILL.src.md` 的「Engine commands」表（§6.8）、`skills/phoenix-ui/evals/scenarios.md`（§6.10）、`docs/phoenix-ui.md`、`README.md`、`tools/phoenix-ui/docs/ENGINE.md`（§6.9）、`NOTICE.md` 与 `THIRD-PARTY-NOTICES.md` 的文字（§6.10）；再渲染一次并提交生成物。同构：docs/phoenix-ui.md「素材命令（media）」一节的写法；README「素材与图片」一条；evals 第 15–24 条。`docs/phoenix-ui.md`、`README.md`、`SKILL.src.md` 由主 agent 单人修改 | 批末 V-4、V-5 通过；回写「实施进度」 |
| M5 | 阶段收口：构建同步、全量回归、真实使用验收与四平台候选 | M1–M4 完成 | 依次：①oracle 的 compare 用例与 `cli-help` 重录；②`smoke-phoenix-ui.mjs` 增加 compare 的冒烟；③`check-phoenix-deps.mjs --write`；④开发构建与 V-6；⑤用同一开发构建跑 V-7，修复后只复跑失效项；⑥V-8 并在其后创建候选提交，记录完整候选 SHA；⑦经授权推送，以该 SHA 精确匹配 run ID 跑 V-9 并回填。改动 skill 源或引擎源码后须从④重建再复验受影响项。同构：oracle 里决策页的用例（tools/phoenix-ui/tests/oracle/cases/context.mjs:691-705）与同目录的 `media.mjs`；冒烟脚本对 media 的增补（scripts/smoke-phoenix-ui.mjs:107-123） | V-6、V-7、V-8、V-9；回写「实施进度」 |

提交点：M1、M2、M3、M4 各自交付后分别提交（M1 与 M2 同批、M3 与 M4 同批，不等批末检查；M3、M4 的提交含渲染生成物）；M5 的⑥通过后做候选提交；全部里程碑完成后最终提交一次并回填 SHA。推送仅在用户授权时进行（C-10）。

并行：批 1 内 M1 的 Rust 与 M2 的页面文件可并行，接口以 §5.4、§6.1 为准，批末一起验收；批 2 的文本开发（M3、M4）与批 1 的可写路径不重叠，可与批 1 并行起草，V-5 等 V-1 通过后再跑。主 agent 负责共享文件（`serve_question.rs`、`lib.rs`、main.rs、`ROOT_USAGE`、`SKILL.src.md`、`docs/phoenix-ui.md`、`README.md`）、渲染与各批验收；V-5、V-7 的各个会话共用一个构建，夹具目录、端口与浏览器资料目录各自独立。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| WORK-F10/AC-5“不发起网络请求”的口径（ADR-1） | 按“没有请求离开对比页服务”实现与验收：方案自己的样式、图片从本机的对比页服务加载，其余请求被拦下 | 用户 2026-11-12 已确认此口径；之后若改为要求字面上的零请求，按 ADR-1 的重新评估触发改用备选 2，M1、M2 的路由与页面要重做 | 已确认 | 否 |
| A-1：开发服务器拒绝被嵌入 | 带安全响应头的开发服务器在对比页里是空白画面 | 「在新标签页打开」与截图方案；docs 写明；V-7 A1 记录实际情况。用户反馈常见时再做响应头探测 | M5 | 否 |
| A-2：各平台浏览器对沙箱文档的 `'self'` 资源与字体行为 | 某平台上方案的样式或字体不显示，或隔离不成立 | V-3 在本机与四平台实测；样式或字体不显示时把该类资源改为放宽到本服务源的显式写法；隔离不成立时该平台不宣称 AC-5 并回到 ADR-1 重评 | M2 批末（本机）；M5（四平台） | 否 |
| 可交互小样不支持模块脚本 | 用构建产物或 `type="module"` 的小样标了 `interactive` 也跑不起来 | compare.md 与 docs 写明改用本机地址方案 | 发布后 | 否 |
| 焦点进了方案的 iframe 后键盘切换失效 | 用户在方案里点过之后方向键不起作用 | 底部提示与始终可点的方案标签；这是浏览器的行为 | 发布后 | 否 |
| 宿主会话演练的随机性 | V-5、V-7 的某个请求一次不符合不等于文本有缺陷 | 先核对文本歧义并修订，再只重跑该请求；仍不符合按缺陷处理 | M4、M5 | 否 |
| SKILL.md 命令表的落点（D16） | 引擎命令列在单独的小表里而不是设计子命令表；若用户希望 compare 成为可 pin 的设计子命令，需要加 `command-metadata.json` 条目并改冒烟里的数量断言 | 由用户决定；未另行指定时按 D16 | M4 开工前 | 否 |
| 推送授权与 CI | V-9 不能执行，M5 不能记完成 | 收口前向用户申请推送；未授权时记入当前阻塞 | M5 | 否（影响收口结论，不影响实施） |
| SPMS 未录入 | 需求仍是工作号 | 按 PRD §8 由用户修复令牌后录入；不影响实施 | — | 否 |

- 最终状态：**Ready**。
- 定级理由：范围与期次由 PRD v1.4 确定，前置期次 P1（序列帧）已完成；命令名（PRD Q1）在 D2 定下；对比页的接口、载荷、路由、响应头、页面结构与 skill 文本落点都已写到可直接实现，复用的代码都核到了行为。AC-5 的“网络请求”口径已由用户确认（ADR-1）；SKILL.md 命令表的落点有带依据的默认做法并已列出改变它的代价，不改变本期范围、安全边界或外部契约。推送授权与宿主会话只影响收口检查能否执行，按 §9.3 记缺口。

## 12. 已知坑与历史教训

- 本机 Bash 是 zsh：未加引号的变量不按空格拆分，带通配符的参数要加引号，单独一段以 `=` 开头的词会被当成命令路径展开（用户全局规则）。
- `compare` 的路由分支必须写在 `live` 前缀与 detect 简写两条之前（crates/cli/src/main.rs:120、:123）：当前目录里有名为 `compare` 的文件或目录时，写在后面会被当成检测目标。
- `crates/` 下新增的 html、css、js 会进许可清单（scripts/check-phoenix-deps.mjs:457-466）：加了页面三个文件后清单过期，`check-phoenix-deps.mjs --write` 在 M5 重算；在那之前不要把清单检查当作批内反馈。
- 发行版 `panic = "abort"`（tools/phoenix-ui/Cargo.toml:38）：载荷、请求体、请求路径都是外部输入，不 `unwrap`、不越界索引；`tiny_http::Header::from_bytes(...).unwrap()` 只用于编译期已知的固定头，照现有两处的用法。
- CI 是无头环境：`compare --start` 在 CI 里会按 D13 退出 2。测试直接用 `--detached-serve` 起服务，冒烟带 `PHOENIX_UI_QUESTION_FORCE=1`；oracle 里专门有一条断言无头时的退出 2。
- macOS 上 Chrome 无法在 `sandbox-exec` 内初始化沙箱（scripts/test-phoenix-ui.mjs:45-47）：`compare.rs` 属于 `phoenix-ui` 包的测试，跑在 sandbox 外的浏览器段；`compare_security.rs` 不需要浏览器但同在这个包里，监听只绑 `127.0.0.1`。
- 沙箱文档没有同源身份：字体按跨源加载，必须带 `Access-Control-Allow-Origin: *`（component_review/server.rs:28-32 的做法）；模块脚本同理取不到，所以可交互小样只支持内联与经典脚本。
- capture 的定位不进 iframe（crates/cli/src/capture/mod.rs 用法说明）：验证时只能操作外壳，方案内部的事实靠像素（V-3 的红绿底色）或开图查看判断。
- capture 的每个状态都在新打开的页面里执行，截完即关：对比页会看到多次打开与关闭，两次截图之间超过 15 秒时 `--wait` 会报页面已关；验证时先截图、最后再 `--wait`，服务本身会留到 `--idle-grace`。
- 决策页的 `--start` 不带 `--open` 时不会自己打开浏览器（serve_question.rs:602-604），对比页沿用；验证脚本不要等页面自己弹出来。
- oracle golden 逐字节比较，`record.mjs` 只重录有意变化的用例并逐个审阅 diff（tools/phoenix-ui/tests/oracle/README.md）；输出里的端口、键与路径要靠用例的固定键与既有的归一化处理，照决策页用例的写法。
- 改 `skills/phoenix-ui/src` 后不渲染，`GENERATED.json` 失配，`render --check` 与 `test:phoenix` 会失败（docs/phoenix-ui.md「维护者」节）：M3、M4 各渲染并提交生成物。
- 宿主会话演练的坑（docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md「V-9 演练偏差」、docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md）：夹具放仓库树外；skill 用副本而不是指向会话目录外的符号链接；`claude -p` 在会话留有后台进程时会先输出结果再挂起（对比页的守护进程正是后台进程，R2 结束后确认它已随回答退出或手动 `--stop`）；宿主会话要显式继承本机已有的 `HTTP(S)_PROXY` 才能连通。
- 项目内安装的 skill 副本会被项目级扫描当成产品文件（docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md「M4 实测纠正」）：演练里判断“产品文件有没有被改”时只看产品源码路径。
- 开发构建没有引擎下载地址，安装必须带 `--release-dir`（docs/phoenix-ui.md「安装」选项表）；`smoke-phoenix-ui.mjs --stage-runtime` 重跑前清理旧的开发构建输出。
- `local/` 被 `.gitignore` 忽略：oil-ui 只作行为参考，夹具一律自备。
- 下次发布不能用 `--reuse-engines`：引擎源码已变（docs/phoenix-ui.md「维护者」节的复用条件）。
