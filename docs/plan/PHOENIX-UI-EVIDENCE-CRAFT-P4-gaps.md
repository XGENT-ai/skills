# PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps · XGENT-ai Skills · 对比页无回答的宿主回退文本与原计划收口里程碑定义复原（文档补遗 · 无运行时改动）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-24 · `2f64baa6`（main）· clean。
>
> 需求来源：[Phoenix UI 取证与工艺增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.5 · P4（第 4 期）· 补齐已完成的 [P4 计划](PHOENIX-UI-EVIDENCE-CRAFT-P4.md)；缺口来源见 [P4 PRD 与开发交付差异核查](PHOENIX-UI-EVIDENCE-CRAFT-P4.prd-dev-gaps.md) 的 G-01（B，WORK-F10/AC-4）与 G-02（记录缺陷），对应需求 ID 保留 PRD 原文编号。
>
> 本期交付：**两处纯文档补遗——`compare` 无回答分支在「宿主没有结构化提问工具」时的回退写法（skill 参考 + docs），以及原计划 §10 被进度行覆盖的收口里程碑定义复原**。
> **开发范围：`skills/phoenix-ui/src/reference/compare.md` 与其渲染生成物、`docs/phoenix-ui.md` 的「方案对比页」无回答句、`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` §10 的收口里程碑单行；依据见 §0.1**。
> 本期独特职责：只把已经实测通过的行为写进文本、把被覆盖的定义行复原；不改变任何产品行为、命令、校验或验收判据。
> **顶层排除：不改引擎（`tools/phoenix-ui` 任何源码、命令、退出码、CSP、页面资源）；不改 `skills/phoenix-ui/evals/scenarios.md` 的判据与失败信号；不改其他 reference、其他 skill、README 与 `ENGINE.md`；不改原计划已完成里程碑的定义行、完成记录与恢复快照（唯一例外：§10 收口里程碑的定义行按 R-2 复原；其余里程碑的定义行、完成记录表、恢复快照与历史 SHA 仍只读）；不重做已完成里程碑；不发布 npm；不动 P5 计划**。

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
         =>[closure_or_high_cost_check_failed, "先看全这一轮暴露的全部失败并排查同类问题，集中修完，每处用局部检查或缩小版本确认，再统一重验一次；不每修一处就从头重跑整轮"],
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

- 最近更新：尚未开始
- 当前进度：0/1 个里程碑完成
- 当前状态：尚未开始
- 最近完成：无
- 下一步：M1 · 先补 R-1 的两处文本并渲染，再复原 R-2 的收口里程碑定义行，批末跑 V-1–V-4
- 当前阻塞：无
- 代码基线：`2f64baa6`（clean）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| — | — | — | 尚未开始任何里程碑 | — | — |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

| ID | 类型 | 来源 | 内容 | 状态 |
| --- | --- | --- | --- | --- |
| R-1 | 需求 | 差异报告 G-01（B）· PRD WORK-F10/AC-4 `:391` | `compare` 无回答分支写明：宿主没有结构化提问工具时，把同一个问题写进答复并结束该轮；无回答仍按「有基线保持现状标待选 / 无基线按推荐标未经用户选择」处理。四处文本同步：`skills/phoenix-ui/src/reference/compare.md` 的「No answer」两个 bullet、`docs/phoenix-ui.md` 的无回答句、渲染生成物 | 待实施 |
| R-2 | 需求 | 差异报告 G-02（记录缺陷）· 原计划 §10 | 从 `0eda22cd^` 取回该里程碑的定义行，复原到原计划 §10；进度只保留在「完成记录」表，最终 SHA 不动 | 待实施 |
| C-1 | 约束 | AGENTS.md「Working in This Repository」接口复述同步 | `src/` 与渲染生成物同一次提交；改完必须 `render --check` 为 fresh，不手改生成物 | 待实施 |
| C-2 | 约束 | 原计划 §0 进度回写规则、差异报告第 6 节 | 原计划只改 §10 的收口里程碑一行（该行即已完成里程碑的定义行，按 R-2 复原是唯一例外；其余里程碑的定义行、完成记录表、恢复快照与历史 SHA 仍只读） | 待实施 |
| C-3 | 约束 | PRD 与差异报告均为输入 | PRD、差异报告、完成记录只读；repair 后差异报告的 G-01/G-02 由 reviewer 另起复审关闭，本计划不自改其状态 | 待实施 |
| A-1 | 假设 | 本轮调查 | `0eda22cd^` 中 §10 的该定义行可取回（`git show 0eda22cd^:docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` 已核实） | 已核实 |

### 0.2 决策表

| ID | 决策 | 备选与取舍 | 依据 |
| --- | --- | --- | --- |
| D1 | 回退写法落 `compare.md`，docs 只留一句并指回参考 | 备选：只在 docs 写全流程——两处口径会再次不一致，且 docs 一直声明「skill 的 `skills/phoenix-ui/src/reference/compare.md` 同此」 | `docs/phoenix-ui.md` `:229` 现状；`vague-feedback.md`「where the host has one」已有同口径 |
| D2 | 复原该定义行而不是补写新定义 | 备选：重写一版更细的定义——会与已完成的实情不符并等于改写历史 | `git show 0eda22cd^` 的原文即该里程碑实际执行的步骤 |

### 0.5 明确不在本期

- **引擎与页面行为**——`compare` 的退出码、提示语、CSP、页面资源已实测符合 PRD，本期不动。
- **宿主遵循稳定性加强（差异报告待核实②）**——是否多起会话统计遵循率需用户决定投入，不纳入本期。
- **发布期安装验证（差异报告待核实①）**——属发布授权动作，不纳入本期。
- **evals 判据扩写**——`scenarios.md` 第 26 条的失败信号已覆盖「把无回答当成选了推荐」，不因一句回退再加判据。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- `compare.md` 的「No answer」两个 bullet（`--wait` 退出 4、`--start` 退出 2）只写「through the structured question tool / go straight to the structured question tool」，未写宿主没有该工具时的做法；`docs/phoenix-ui.md` `:229` 同此，且以「skill 的 `skills/phoenix-ui/src/reference/compare.md` 同此」把 docs 绑在参考文本上。
- 实际行为已实测通过：复述终验记录的 R3 轮在无该工具的会话里以 `PHOENIX_UI_QUESTION_DISABLED=1` 让 `--start` 退出 2，会话在答复里再问一次、交付标「待选」、`src/` 零改动。本轮再次独立复跑该命令，退出码与提示一致。
- 原计划 §10 第 736 行现在是六列进度行，五列表头下该里程碑的定义（①–⑦ 步骤）不可读；`0eda22cd^` 仍可取回原文。

### 1.2 拓扑与文件清单

| 路径 | 角色 | 本期动作 |
| --- | --- | --- |
| `skills/phoenix-ui/src/reference/compare.md` | skill 参考源（「No answer」段） | 加一句宿主回退写法（R-1） |
| `skills/phoenix-ui/{reference,SKILL.md,GENERATED.json}` 及各 provider 生成物 | `tools/phoenix-ui/scripts/render.mjs` 生成 | 由渲染产生，不手改（C-1） |
| `docs/phoenix-ui.md` | 对外文档（「方案对比页」无回答句） | 加同一前提的简短一句（R-1） |
| `docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` | 已完成的原计划（只读，§10 例外） | 复原 §10 的该行（R-2） |
| `docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.records/`（★ 新增，实施后建 M1.md） | 本计划完成记录 | 实施后新建 |

## 9. 验证

### 9.1 行为与接口验证

共同环境：仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；渲染命令在 `tools/phoenix-ui` 下运行；V-4 需要一次 provider 会话（Claude Code，读 `.env.local` 后启动）。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 局部（静态） | R-1、C-1：`compare.md`、`docs/phoenix-ui.md` 两处都写了宿主无工具时的回退与「待选/未经用户选择」的处置 | `tools/phoenix-ui` 下 `mise exec --no-deps -- bun scripts/render.mjs --check`（脚本在 `tools/phoenix-ui/scripts/render.mjs`）；`git diff --stat` 确认 `src/` 的改动对应生成物 | 批末 |
| V-2 | 局部（静态） | R-1：文本可回读且与 `vague-feedback.md` 同口径；docs 仍指回参考而不重复整段流程 | 读 `skills/phoenix-ui/src/reference/compare.md`「No answer」、`docs/phoenix-ui.md`「方案对比页」；`rg -n "structured question" skills/phoenix-ui docs` | 批末 |
| V-3 | 局部（静态） | R-2、C-2：§10 五行与五列表头对齐、该里程碑定义可读；完成记录表与恢复快照的 SHA、进度未变；`git diff` 只触及该行 | `sed -n '/^## 10/,/^## 11/p' docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md`；`git diff -- docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` | 批末 |
| V-4 | 真实使用验收（有界） | R-1：只凭公开文档与参考走一次无回答分支——`--start` 退出 2 后会话在答复里再问一次、交付标「待选」，无源码改动 | 仓库树外夹具 + 开发构建安装（先按 `docs/phoenix-ui.md` `:291` 在 `tools/phoenix-ui` 下跑一次开发构建，产出 `<仓库>/local/phoenix-ui/development-release` 并同步 `dist/phoenix-ui/VERSION.json`；当前工作树无该目录且 `ENGINE.json` 仍记 0.7.12，直接安装会在预检阶段因清单不符失败。随后仓根 `bin/xgent-skills.js install <夹具目录> --release-dir=<仓库>/local/phoenix-ui/development-release`）+ 安装后的启动器 `PHOENIX_UI_QUESTION_DISABLED=1 <夹具目录>/.claude/skills/phoenix-ui/scripts/phoenix-ui compare --start --payload two.json`；新会话只读 `docs/phoenix-ui.md` 与参考 | 收口 |

### 9.2 集成与回归

不跑 `test:phoenix`、不重跑四平台 CI：本期不触引擎源码、命令与退出码，`compare` 的实现与验收已在候选 `968b6539` 上通过；V-1 的 `fresh` 是生成物与源一致的门。

### 9.5 静态校验

`git diff --check`；`git status --porcelain` 确认没有夹具或临时文件混入提交。

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 两处文档补遗 | 无 | 单批。先改 `skills/phoenix-ui/src/reference/compare.md` 并渲染，再改 `docs/phoenix-ui.md` 一句，最后复原 §10 的该行；三项互不依赖，可一次编辑完成 | V-1、V-2、V-3、V-4；回写「实施进度」 |

提交点：M1 通过后一次提交 `docs(phoenix-ui): document compare fallback without a question tool and restore the P4 closing milestone definition`，并在完成记录与恢复快照回填 SHA；不推送（推送需用户授权）。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 渲染生成物漏提交 | 分发包与源不一致，`render --check` 会报错 | V-1 在提交前跑；`git status` 确认生成物已加入 | M1 提交前 | 否 |
| 复原该行时连带改到其他行 | 破坏原计划的历史事实 | 只从 `0eda22cd^` 复制该行；V-3 用 `git diff` 验证只有一行变化 | M1 提交前 | 否 |
| V-4 无可用 provider 会话 | 回退写法的可遵循性只有静态与旧证据 | 记录「环境缺失/未运行」，用该轮既有宿主证据与 V-2 的文本回读作为退路，并如实标注 | M1 收口 | 否 |

就绪状态：`Ready` —— 范围、文本、验收与排除均已确定，无影响范围、安全、数据或外部契约的未决项。

## 12. 已知坑与历史教训

- 渲染必须在 `tools/phoenix-ui` 下跑 `bun scripts/render.mjs`（`tools/phoenix-ui/scripts/render.mjs`），在仓根找不到脚本；不手改 `SKILL.md`/`reference/` 生成物（原计划 §12）。
- `docs/phoenix-ui.md` `:229` 已把 docs 的处置口径绑在 `compare.md` 上，改一处必须核对另一处，否则两处再次分叉。
- 原计划 §10 是定义行表，进度回写只进「完成记录」；上一轮已在同一处犯过错（G-02），复原后不要再把进度写进去。
- 宿主会话启动前先载入根 `.env.local`；未载入会落到订阅认证或 400，属于环境问题而非产品缺陷（原计划 §12）。
