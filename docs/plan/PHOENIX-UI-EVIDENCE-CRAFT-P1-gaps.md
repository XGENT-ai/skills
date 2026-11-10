# PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps · XGENT-ai Skills · capture 步骤失败提示补齐（页内定位脚本增量 · 报告结构不变）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-10 · `0509ba6c9012f5eb5b6a51507a28d10ad7a66601`（main）· dirty：`.xgent-ai/` 台账、原计划及其里程碑记录已修改，PRD、决定记录、评审与差异报告未跟踪；`tools/phoenix-ui` 与 HEAD 一致，代码事实来自 HEAD。
>
> 需求来源：[Phoenix UI 取证、中文排版与素材细节增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.2（`sha256:09b19f65…`，未提交）· **P1（第 1 期）**；补齐 [PHOENIX-UI-EVIDENCE-CRAFT-P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md) 交付后经 [PRD 差异核查](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md) 确认的明确遗漏 G-01、G-02。
>
> 本期交付：**`phoenix-ui capture` 在目标隐藏、被遮挡或禁用时给出状态文件格式的可见元素定位写法；`waitFor` 超时与其他找不到目标的失败一样，报出“选择器只作用于主文档”的 iframe / Shadow DOM 限制。**
> **开发范围：`tools/phoenix-ui/crates/cli/src/capture/page_js.rs`（`LOCATE`）、`steps.rs`（`locate`、`LocateError`、`waitFor` 超时）、`tools/phoenix-ui/crates/cli/tests/capture.rs` 的 `step_failures_and_warnings` 与夹具 `tools/phoenix-ui/tests/fixtures/capture/pages/failures.html`、`tools/phoenix-ui/tests/fixtures/capture/pages/failures.json`。依据见 §0.1。**
> 本期独特职责：原计划只为“找不到”生成候选写法和 iframe 说明；本计划把同一套提示延伸到隐藏、遮挡、禁用与等待超时。
> **顶层排除：不改步骤判定（隐藏仍判失败；遮挡、禁用仍记警告并照常执行；`waitFor hidden` 语义不变）；不改报告 schema、终端行格式、`--help`、docs、skill 文本与 oracle golden；不改 `cdp.rs`、detect 与其他 verb；不发布 npm/R2。**

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

- 最近更新：2026-11-10T23:48+08:00
- 当前进度：1/1 个里程碑完成
- 当前状态：M1 已完成并提交（`c84f6a4`）
- 最近完成：M1
- 下一步：代码评审——由用户以本计划路径为参数发起 review-code
- 当前阻塞：无
- 代码基线：`0509ba6`

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-10T23:48+08:00 | 隐藏给出可见元素写法，遮挡、禁用警告追加写法，`waitFor` 超时保留 iframe / Shadow DOM 限制说明；V-1、V-2 通过 | [M1 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.records/M1.md) | c84f6a4 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID；G-ID 来自 [PRD 差异核查](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md)。其余 P1 需求已由原计划交付，本计划不重做。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F1/AC-3 | 功能 | PRD §3 WORK-F1 AC-3 后半句 · P1；差异 G-01 | 目标被遮挡、禁用或隐藏时，报告除提示外还给出可见元素的定位写法（状态文件 JSON 格式） | §5.1–§5.3；V-1、V-2 | 已确认 |
| WORK-F1/边界 | 功能 | PRD §3 WORK-F1「边界」与 AC-7 · P1；差异 G-02 | 目标在 iframe 或 Shadow DOM 内时报出“只作用于主文档”的限制，包括 `waitFor` 等待超时 | §5.4；V-1、V-2 | 已确认 |
| C-1 | 约束 | 原计划 §5.4；PRD WORK-F1/AC-3 前半句 | 判定语义不变：隐藏 → 该状态失败且不截图；遮挡、禁用 → 步骤 `warning` 并照常执行；“找不到”的候选生成与已有提示逐字不变 | §5、§0.5；V-2 | 已确认 |
| C-2 | 约束 | 原计划 §3「报告带 schema 版本，后续扩展字段只增不改」；`tools/phoenix-ui/crates/cli/src/capture/report.rs:163-179` | 报告 `phoenix-ui-capture-v1` 不增删字段：隐藏的写法进已有 `failures[].hints`（终端已有“可改为：”行），遮挡、禁用的写法追加在已有 `warnings` 文字中 | D1；V-2 | 已确认 |
| C-3 | 约束 | 原计划决策表中页内脚本存放的决定；`scripts/check-phoenix-deps.mjs:457-472` | 页内脚本只改 Rust 常量 `page_js.rs`，不在 `crates/` 下新增 .js 文件 | §1.2 | 已确认 |
| C-4 | 约束 | AGENTS.md「Working in This Repository」及首条分支规则；`mise.toml` | Cargo 经 mise（rust 1.99.1）以 `--locked --offline` 运行，工具缺失记失败；浏览器测试不放进 macOS `sandbox-exec`；留在 main，推送只在用户授权时进行 | §9 | 已确认 |
| C-5 | 约束 | `tools/phoenix-ui/Cargo.toml` `[profile.release] panic = "abort"`；原计划 §5.10 | 处理页内脚本返回值时不 `unwrap`，缺字段按空处理 | §5 | 已确认 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 写法放在哪里 | 隐藏：写进该失败的 `hints`；遮挡、禁用：在原警告句末追加写法，不新增步骤级字段 | 报告结构、终端格式、review-ui/feature-demo/走查者的读取方式都不变；代价是遮挡、禁用的写法以文字形式出现，不是单独数组 | C-2；`report.rs:163-179` 已打印 `hints`，`mod.rs:1263-1271` 只登记 `warnings` |
| D2 | 候选从哪来 | 把 `LOCATE` 中“找不到”分支的相关文字候选提成页内函数 `suggest(wanted, like, exclude)`；先取相关文字，再在 `like` 非空且不足 3 个时按同标签（参照元素有 role 时同 role）补足，排除目标自身。新增路径两阶段统一排除隐藏及三种禁用状态；实际 `innerText` 非空且 ≤60 字时输出 `text`，否则输出按 D3 转义的 `css`，细则见 §5.1。`like=null` 的 missing 分支保持原过滤、顺序与输出。角色定位沿用 Rust 端 `role_hints` | 隐藏、离屏、遮挡、禁用共用新增路径；候选供调用方判断，不保证选择器唯一；保留 missing 旧行为 | `page_js.rs:57-72`、`steps.rs:115-157` |
| D3 | 遮挡物与替代写法 | 选择器 = `localName` + `#CSS.escape(id)`（有 id 时）+ 前两个 `.CSS.escape(class)`；遮挡警告保留原句，追加遮挡物 CSS，再附 §5.1 的至多 3 个替代候选（排除目标自身），无候选则只给遮挡物 | CSS 转义规则也供 D2 的候选使用；不保证唯一，写法用于指认，由调用方判断；遮挡仍警告并执行 | `page_js.rs:96-102`；G-01 Follow-up |
| D4 | `waitFor` 的限制说明 | `LocateError` 增加 `scope_note: Option<String>`：`missing` 分支把已追加到原因里的那句同时存入此字段；`waitFor` 超时拼原因时，若最后一次探测带 `scope_note` 则追加同一句 | 与 click 等找不到时的说明逐字一致；`until: hidden` 超时（目标在主文档可见）不带说明 | `steps.rs:115-137`、`:322-362` |

### 0.4 职责与事实所有权

capture 仍只报告页面在这些步骤后的事实与可照做的写法，不判断哪个候选“对”；改不改、怎么改归调用方（主代理、走查者、feature-demo、review-ui）。其余同原计划 §0.4。

### 0.5 明确不在本期

- **其他 P1 需求与原计划已交付的行为**——不重做；原计划 §9 各检查的已有证据沿用。
- **进入 iframe / Shadow DOM 执行步骤（PRD N13、Q4）**——仍不支持，只报限制。
- **“找不到”时的候选规则调整、CSS 定位找不到时也给候选**——PRD AC-3 未要求，现状不变。
- **步骤级结构化 `hints` 字段**——调用方需要机器可读的遮挡/禁用写法时再评估（D1）。
- **四平台 CI 复跑与发布**——本改动只在页内脚本与 Rust 错误拼接，不涉及平台与分发；下次经授权推送 main 时 `.github/workflows/phoenix.yml` 会自动运行，不作为本计划退出门。发布须走 CI 构建、不能用 `--reuse-engines`（原计划 §0.5）。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·缺口] 隐藏目标无写法**：`LOCATE` 在有匹配但都不可见时返回 `{status:'hidden', count, reason}`（`page_js.rs:51-56`），`steps.rs:110-114` 生成 `LocateError { reason: "<原因>；先执行让它出现的那一步", hints: Vec::new() }` → 按 D2 生成候选 → 否则调用方只能猜（WORK-F1/AC-3）。实测：`failures.html` 的 `hidden-target` 报告 `hints=[]`。
- **[已核实·缺口] 遮挡、禁用只有提示**：遮挡警告用 `localName + #id + .class` 拼名（`page_js.rs:96-102`，未转义），禁用警告只列原因（`:103-110`）→ D3、D2 → 否则 AC-3 后半句不满足。
- **[已核实·缺口] `waitFor` 超时丢掉限制说明**：`missing` 分支把“选择器只作用于主文档：页面有 N 个 iframe、M 个开放 shadow root，可直接打开被嵌入的页面”拼进原因（`steps.rs:121-130`）；`waitFor` 超时重新拼原因，只沿用 `hints`（`steps.rs:348-358`）→ D4 → 否则嵌入页面上的等待只显示超时。实测：对 `failures.html` 执行 `{"do":"waitFor","target":{"text":"框内按钮"},"timeoutMs":300}`，原因无 iframe 说明。
- **[已核实·足够] 其他调用点**：`steps::locate` 还被元素图与 `mark`（`mod.rs:1292`、`:1357`）调用，它们只用成功结果或失败原因；隐藏分支多出的 `hints` 不改变它们的行为。终端在 `FAILED` 行下逐条打印 `hints` 为“可改为：…”（`report.rs:170-179`）。
- **[已核实·足够] 夹具**：`tools/phoenix-ui/tests/fixtures/capture/pages/failures.html` 已有隐藏按钮“隐藏按钮”、被 `span.veil` 遮挡的“被盖住”、禁用的“禁用按钮”与内含“框内按钮”的 iframe；`failures.json` 已有对应状态；V-1 应用夹具的“应用筛选”在登录后仍位于 `hidden` 的 `#filter-panel` 内（`tools/phoenix-ui/tests/fixtures/capture/app/index.html:31-40`）。

### 1.2 拓扑与文件清单

| 位置 | 改动 |
| --- | --- |
| `tools/phoenix-ui/crates/cli/src/capture/page_js.rs` | `LOCATE`：提出 `suggest`；`hidden` 返回带 `hints`；遮挡名转义并追加遮挡物写法与替代候选；禁用警告追加候选 |
| `tools/phoenix-ui/crates/cli/src/capture/steps.rs` | `LocateError.scope_note`；`locate` 隐藏分支取 `hints`（角色定位用 `role_hints`）；`waitFor` 超时追加 `scope_note` |
| `tools/phoenix-ui/crates/cli/tests/capture.rs` | `step_failures_and_warnings` 增加断言 |
| `tools/phoenix-ui/tests/fixtures/capture/pages/failures.html` | 保留已有元素顺序；追加一个可见可交互的替代按钮（如“替代按钮”），供遮挡等新路径补足；增加开放 Shadow Root，内部放“影内按钮” |
| `tools/phoenix-ui/tests/fixtures/capture/pages/failures.json` | 新增 `wait-framed`、`wait-shadow` 状态，分别 `waitFor` “框内按钮”与“影内按钮”出现，`timeoutMs:300` |

无需修改：`report.rs`（已打印 `hints`）、`mod.rs`、`cdp.rs`、`--help`、`docs/phoenix-ui.md`（只写“只查主文档”，未描述提示细节）、skill 文本、oracle（capture 用例均为不启动浏览器的离线路径）、生成物（原计划 `1a58e6a` 只改引擎源码即通过 CI）。

## 5. 核心机制

### 5.1 候选函数 `suggest`（`LOCATE` 内）

```js
// wanted: 文字；like: 参照元素或 null；exclude: 要排除的元素或 null（避开已有的 skip 函数名）
const suggest = (wanted, like, exclude) => { /* 返回至多 3 个 JSON 字符串 */ };
```

1. 相关文字：沿用现有 `related` 判定与文档遍历（`document.body.querySelectorAll(INTERACTIVE)`），跳过 `skip(e)`、`hidden(e)`、`exclude`、空文字、超过 60 字与重复文字；相关性仍以 `textOf(e)` 判断。`like` 非空时还排除 `:disabled`、`aria-disabled`（转小写后为 `true`）、计算样式 `pointer-events:none`，与现有目标禁用定义一致。`wanted` 为空（如无文字的目标）时跳过本阶段，避免空串让所有元素都算“相关”。
2. 补足：结果少于 3 个且 `like` 非空时，按文档顺序取 `localName === like.localName`（`like` 有 `role` 属性时改为同 `role` 属性）的可交互元素；同样跳过 `skip(e)`、`hidden(e)`、上述三种禁用状态与 `exclude`。不因空文字或长文字丢掉同类元素，改按第 3 条输出 CSS；跳过已入选元素与重复写法，至多 3 项。
3. 序列化：`like` 非空的新路径仅在 `norm(e.innerText)` 非空且长度 ≤60 时输出 `JSON.stringify({ text: norm(e.innerText) })`；否则按 D3 的标签、转义 id/class 规则输出 `JSON.stringify({ css: sel })`。不得把 `aria-label`、`value` 或空串写成 `text`。`like=null` 保持原 `JSON.stringify({ text: textOf(e) })` 输出与去重，不应用新增禁用过滤或 CSS 回退。
4. “找不到”分支改为 `hints = loc.text !== undefined ? suggest(norm(loc.text), null, null) : []`，与现状等价；隐藏、滚动后仍离屏、遮挡、禁用都以非空 `like` 调用，排除目标自身。CSS 只保证合法写法、不保证唯一，沿用 D3 的指认定位边界。

### 5.2 隐藏

`hidden` 返回对象加 `hints: suggest(loc.text !== undefined ? norm(loc.text) : textOf(all[0]), all[0], all[0])`。`steps.rs` 隐藏分支：`hints = strings(result.get("hints"))`，角色定位时改用 `role_hints(page, role, name)`（与 `missing` 分支相同）；原因文字与 `not_visible: true` 不变（`waitFor` 仍据此轮询）。“滚动后仍不在视口内”的 `hidden` 返回同样带 `hints`，参数改用该元素 `el`。

### 5.3 遮挡与禁用

- 遮挡：`const sel = top.localName + (top.id ? '#' + CSS.escape(top.id) : '') + [...top.classList].slice(0, 2).map((c) => '.' + CSS.escape(c)).join('')`；警告：`'目标中心被 ' + sel + ' 遮挡，动作照常执行；遮挡物：' + JSON.stringify({ css: sel })`；再取 `alt = suggest(textOf(el), el, el)`，非空时追加 `'；可用的可见元素：' + alt.join('、')`。替代候选遵守 §5.1 的序列化、过滤与排除自身规则，遮挡仍警告并执行。
- 禁用：`const alt = suggest(textOf(el), el, el)`；`alt` 非空时警告追加 `'；可用的可见元素：' + alt.join('、')`，否则保持原句；“可用”采用 §5.1 两阶段相同的三种禁用过滤口径。

### 5.4 `waitFor` 超时的限制说明

- `LocateError` 新增 `pub scope_note: Option<String>`；`plain` 与隐藏分支为 `None`。
- `missing` 分支：`frames + shadows > 0` 时生成 `note = format!("选择器只作用于主文档：页面有 {frames} 个 iframe、{shadows} 个开放 shadow root，可直接打开被嵌入的页面")`，原因仍为 `找不到 <定位>；<note>`，并存入 `scope_note`。
- iframe 与开放 Shadow DOM 均复用现有 `iframes` / `shadowRoots` 探测和此字段；§1.2 的两份夹具通过 M1 → V-2 分别验证两种等待超时，不进入嵌入文档定位。
- `waitFor` 超时：原因照旧拼出后，若 `probe` 为 `Err(e)` 且 `e.scope_note` 有值，追加 `；<note>`；`hints` 照旧取自 `probe`，该错误自身的 `scope_note` 为 `None`。

## 9. 验证

共同环境：仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；Cargo 命令在 `tools/phoenix-ui` 下运行；需可发现的 Chrome（可用 `PHOENIX_UI_BROWSER` 指定）；浏览器测试不放进 macOS `sandbox-exec`。批内最小反馈：`mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` 后直接运行 `target/debug/phoenix-ui capture tools/phoenix-ui/tests/fixtures/capture/pages/failures.html --viewport 800x600 --states tools/phoenix-ui/tests/fixtures/capture/pages/failures.json --out "$tmp/out"`（仓库根相对路径；在 `tools/phoenix-ui` 下运行时去掉前缀） 查看报告。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E（真实使用验收） | **调用方按报告提示修好过早点击的状态文件。** 角色：只读过 `phoenix-ui capture --help` 与页面可见内容，不读 capture 源码与夹具源码。输入：原计划取证场景所用的应用夹具 `tools/phoenix-ui/tests/fixtures/capture/app/`，状态 `apply-early` = 原 `open-filter` 的登录三步 + `{"do":"waitFor","target":{"text":"任务"}}` + `{"do":"click","target":{"css":"#apply-filter"}}`。结果：①该状态 `FAILED … step 5 (click)`，原因含“先执行让它出现的那一步”，下一行有 `可改为：{"text":"筛选"}`（WORK-F1/AC-3）；②按提示在第 5 步前插入 `{"do":"click","target":{"text":"筛选"}}` 后重跑退出 0，Agent 打开截图确认筛选面板已展开；③对 `failures.html` 运行只含 `{"do":"waitFor","target":{"text":"框内按钮"},"timeoutMs":300}` 的状态，原因含“选择器只作用于主文档：页面有 1 个 iframe”（WORK-F1/边界）。 | `tools/phoenix-ui` 下构建 debug 二进制；`DEMO_PASSWORD=<测试值> target/debug/phoenix-ui capture tests/fixtures/capture/app --viewport desktop --states "$tmp/apply.json" --out "$tmp/apply"`；③为 `target/debug/phoenix-ui capture tests/fixtures/capture/pages/failures.html --viewport 800x600 --states "$tmp/wait.json" --out "$tmp/wait"` | M1 交付齐备后 |
| V-2 | 局部 | 准入：扩展已有 `step_failures_and_warnings`，定向验证本期提示增量。`hidden-target` 的 hints 含 `{"text":"被盖住"}` 且 stdout 在其 FAILED 行后有对应“可改为：”；`covered` 警告同时含 `遮挡物：{"css":"span.veil"}` 与可见替代按钮的候选写法，且不含目标自身；`disabled` 警告含可用候选。三类新增路径的候选总数至多 3 个，沿 §5.1 核对可用性与状态文件 JSON 写法，不再固定为单项数组。新增 `wait-framed`、`wait-shadow` 的失败原因均含“选择器只作用于主文档”、`1 个 iframe` 与 `1 个开放 shadow root`；分别证明 iframe 与开放 Shadow DOM 内目标等待出现超时的说明保留。原有断言（含 `missing-text` 的 `hints[0]`、`wait-hidden` 的消失超时）不变（C-1）。 | `mise exec --no-deps -- cargo test -p phoenix-ui --test capture --locked --offline step_failures_and_warnings -- --test-threads=1`；`mise exec --no-deps -- cargo fmt -p phoenix-ui --check`；`mise exec --no-deps -- cargo clippy -p phoenix-ui --all-targets --no-deps --locked --offline`（只检查本包，编译其必要依赖；既有诊断按 `tools/phoenix-ui/lint-baseline.json` 区分，不将无关存量诊断升级为本期缺陷） | M1 交付齐备后 |

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 隐藏、遮挡、禁用的写法与 `waitFor` 限制说明 | 无 | 一批完成 §5.1–§5.4 与 §1.2 全部文件；同构：现有 `missing` 分支的候选与 `role_hints`（`page_js.rs:57-81`、`steps.rs:115-157`）。主 agent 单人修改，不并行 | V-1、V-2；回写「实施进度」 |

提交点：M1 完成后按 `@commit_protocol` 提交并回填 SHA。推送仅在用户授权时进行（C-4）。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 同标签补足的候选与调用方意图不符 | 提示不准，调用方仍需判断 | 候选只作写法提示，不改判定（C-1）；V-1 按真实修正任务检验有用性，结果不符时就地修订 §5.1 | M1 | 否 |

- 最终状态：**Ready**。
- 定级理由：两项差异的需求原文明确，判定语义、报告结构与对外接口都不变；改动落点、同构实现与检查命令已核到代码；无外部依赖与未决项。

## 12. 已知坑与历史教训

- `crates/` 下新增 .js/.html/.css 会被自动登记为 Impeccable Apache-2.0（`scripts/check-phoenix-deps.mjs:457-472`）：页内脚本只改 `page_js.rs`，夹具只改 `tests/` 下文件。
- macOS 上 Chrome 无法在 `sandbox-exec` 内初始化沙箱（`scripts/test-phoenix-ui.mjs:46-48`）：capture 测试直接用上面的 cargo 命令在 sandbox 外跑。
- 发行版 `panic = "abort"`：页内脚本返回值缺字段时按空处理，不 `unwrap`。
