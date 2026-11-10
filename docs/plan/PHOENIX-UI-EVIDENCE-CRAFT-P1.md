# PHOENIX-UI-EVIDENCE-CRAFT-P1 · XGENT-ai Skills · Phoenix UI 多状态取证与盲测走查（原生 `capture` 命令 · 走查子代理 · 三个独立 skill 的可选接入）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-09 · `4c2d1117f83b7af3292a846aa71a14fe4df8d990`（main）· dirty：`.xgent-ai/` 台账已修改，PRD、决定记录与评审报告未跟踪；PRD 相关事实来自当前工作树。参考源 `local/oil-ui-main`（oil-ui 0.33.2，MIT，无 `.git`，被 `.gitignore` 忽略，只在本机可读）。
>
> 需求来源：[Phoenix UI 取证、中文排版与素材细节增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.2（`sha256:09b19f65…`，未提交）· **P1（第 1 期）**；范围依据为 PRD §7.2 分期矩阵 P1 列、[逐项决定记录](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.decisions.md)（用户 2026-11-09 确认）与 [PRD 评审](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.review.md) 的修订处置。
>
> 本期交付：**Phoenix 原生引擎新增 `phoenix-ui capture` 取证命令（多视口、命名状态组、三种截图类型、有效性自检、写请求拦截、出处报告、逐张页面规则发现，以及序列帧、遮字版、扫视缩略图、标注图、状态并排图和修改前后拼图）；Phoenix 完工轮默认用它取证，并同轮派出新的盲测走查子代理 `phoenix-ui-ux-walker`；feature-demo、review-ui、apply-ui-review 在检测到 Phoenix 时可选借用取证与走查。**
> **开发范围：`tools/phoenix-ui`（`crates/cli` 新增 capture 模块，`crates/browser` 增补 CDP 能力并抽出页面扫描函数，`crates/detect` 增加选项解析函数与总帮助一行，测试、夹具、oracle 用例，`scripts/render.mjs` 子代理计数，第三方声明）；`scripts/smoke-phoenix-ui.mjs` 的 capture 冒烟；`skills/phoenix-ui/src` 与经开发构建同步的生成物、`skills/phoenix-ui/evals/scenarios.md`；`skills/feature-demo`、`skills/review-ui`、`skills/apply-ui-review` 的可选接入；`docs/phoenix-ui.md`、`README.md` 的对应复述。依据见 §0.1。**
> 本期独特职责：现有截图只服务 comp-led 门禁（`build-phase`、`component-review capture`）与 live 批注；本期提供面向任意调用方的通用“按步骤操作后多状态取证”命令，以及一个不拿任务的独立走查者。
> **顶层排除：不发布 npm/R2 新版本；不改 comp-led 门禁、`build_phase.rs` 文案、component-review、live、critique、方向轮与 hook 的行为；不在 iframe/Shadow DOM 内执行步骤；不录视频；不改 review-ui 的计分、门槛与判定；P2–P4 的中文规则、素材、图片处理、对比页与细节指引不在本期。**

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

- 最近更新：2026-11-10T16:30+08:00
- 当前进度：6/6 个里程碑完成
- 当前状态：全部里程碑实施记录完成；R2 F-05 旧报告合并缺口已修复并通过本地真实 E1。V-7/V-10 保留原证据，V-6/V-9 待核实，V-8 只覆盖 `d5904e1`，不覆盖当前修复版
- 最近完成：M6 · 阶段收口
- 下一步：代码评审——`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md`
- 当前阻塞：无实现阻塞；代码验收待独立复审，V-6/V-9 原始材料与当前修复版 V-8 待补
- 代码基线：`55aa061`（origin main 为合并后的 `d5904e1` 及其后的计划提交；本地 main 待用户处理未跟踪的 `skills/software-copyright-docs/` 后再同步，见 [M6 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md#未完项与续做动作)）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-10T15:25+08:00 | capture 主链路：目标打开、多视口、报告与逐张页面规则发现；与 M2 同批验收；R1 评审 F-02 修复 | [M1–M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M2.md) | c6ce151；修复 e374f79 |
| M2 | 已完成 | 2026-11-10T16:30+08:00 | 状态组与步骤、三种截图类型、有效性自检、写拦截与凭据脱敏；V-1、V-2 通过；R1/R2 F-01/F-03/F-05 修复，旧报告沿用图已恢复原工具身份 | [M1–M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M2.md) | c6ce151；修复 e374f79 |
| M3 | 已完成 | 2026-11-10T15:25+08:00 | 序列帧、遮字版、缩略图、标注图、并排图与 capture stitch；V-3、V-4 通过；R1 评审 F-04 修复，R2 复核已解决 | [M3 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M3.md) | 9203484；修复 e374f79 |
| M4 | 已完成 | 2026-11-09T23:47+08:00 | Phoenix Web 完工轮用 capture 取证并同轮派出盲测走查子代理；与 M5 同批验收 | [M4–M5 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M5.md) | a3177e5 |
| M5 | 已完成 | 2026-11-10T16:30+08:00 | feature-demo、review-ui、apply-ui-review 检测到 Phoenix 时的可选接入与复述；V-5 通过，V-6 原演练有记录但原始宿主材料待核 | [M4–M5 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M5.md) | a3177e5 |
| M6 | 已完成 | 2026-11-10T16:30+08:00 | 声明、oracle、冒烟与开发候选；V-7/V-10 保留原证据，V-9 原始宿主材料待核，V-8 仅覆盖 `d5904e1`，当前 R2 修复版待补四平台结果 | [M6 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md) | 226c8c2、86d8576、21d12ea、ce45063、2a4b99e、55aa061、00878b4 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID（尚未录入 SPMS，不是真实需求 key）。跨期 NFR 只承接 P1 列的 AC；P1 完成后 WORK-N1 待 P3 终验，WORK-N2、WORK-N4 待 P4 终验，WORK-N3 待 P2 终验（PRD §7.3），均不转“已上线”。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F1 | 功能 | PRD §3 WORK-F1/AC-1–AC-9 · P1 | 取证命令：本地 HTML、静态目录或 http(s) 目标；多视口、像素比、浅/深配色；命名状态组从重新打开的页面执行步骤；视口图/整页图/元素图；写请求拦截（默认关）；调用方指定输出目录与文件名；报告逐张登记出处、控制台错误、失败请求与图片、横向溢出及该状态的页面规则发现 | §5.1–§5.5、§5.7–§5.10；V-1、V-2、V-7 | 已确认 |
| WORK-F2 | 功能 | WORK-F2/AC-1–AC-5 · P1 | 派生证据：序列帧（含首次进入）、遮字版、扫视缩略图（480/180）、问题标注图、状态并排图、修改前后拼图；不替代状态截图 | §5.6；V-3、V-4 | 已确认 |
| WORK-F3 | 功能 | WORK-F3/AC-1–AC-7 · P1 | Web 完工评审默认用取证命令写入 `.phoenix-ui/review/` 的证据并注明截图类型；无效图不交评审；修复轮同路径重取并注明沿用；comp-led 门禁不变；宿主与 Phoenix 都无浏览器时写“未取证”；自检轮次不变、不另跑页面 detect；自检对任意调用方与目录生效 | §5.5、§6.1；V-1、V-2、V-9（A1–A3、A5）、V-10 | 已确认 |
| WORK-F4 | 功能 | WORK-F4/AC-1–AC-8 · P1 | 盲测走查子代理随 Phoenix 安装；有主要操作流程的 Web 界面完工时同轮派出；交接限制、报告格式、可复现卡住/走错修复并由主代理复验、交付单列走查且不改处置词、无法走查写“未完成 UX 走查”、记录隔离标识、不派条件；其他 skill 派出时同交接与格式 | §6.1、§6.2；V-6、V-9（A1–A3、A5、A6） | 已确认 |
| WORK-F15 | 功能 | WORK-F15/AC-1–AC-5 · P1 | feature-demo 检测到可用取证命令时，录制前开拦截按演示步骤核对每个待录功能，拦截记录按现有写入规则处理，失败按报告归因；未检测到照原流程；录制、视频检查与报告模板不变 | §6.3；V-5、V-9（A4） | 已确认 |
| WORK-F16 | 功能 | WORK-F16/AC-1–AC-6 · P1 | review-ui 检测到取证命令时对适用单元采集，命名与 `capture.md` 出处符合原合同；无效图按「没采到」；机器发现只进机器检查；主 agent 仍逐张开图；不适用单元改用现有采集方式 | §6.4；V-5、V-9（A5） | 已确认 |
| WORK-F17 | 功能 | WORK-F17/AC-1–AC-5 · P1 | review-ui 检测到走查子代理时对有主要操作流程的单元派出走查；报告存本轮目录；可复现卡住/走错以来源「盲测走查」列差距，犹豫列建议，走查材料不进评审；apply-ui-review 复走同一路径并记录处置 | §6.4、§6.5；V-5、V-6、V-9（A5） | 已确认 |
| WORK-N1 | 安全/隐私 | WORK-N1/AC-1–AC-3（P1 部分；AC-4 属 P3） | 取证用临时浏览器资料目录并在结束后删除；凭据以引用提供，不以明文进入报告、日志与输出；走查只拿测试凭据，不对真实业务数据写入或破坏，交接不含真实业务账号 | §5.7、§6.2、§7；V-1、V-2、V-6、V-9 | 已确认 |
| WORK-N2 | 兼容/分发 | WORK-N2/AC-1–AC-2（P1 部分；AC-3、AC-4 属 P3、P4） | 新命令只随 engine 分发；macOS（arm64/x64）与 Linux（x64/arm64）无需 Rust 即可运行；engine 缺失或版本不符返回固定包版本补装命令；无浏览器时明确报不可用且其他命令照常；Node ≥18、不要求 Python | §5.1、§5.8、§7；V-1、V-7、V-8 | 已确认 |
| WORK-N3 | 兼容 | WORK-N3/AC-1–AC-2（P1 部分；AC-3 属 P2） | 没用到新命令的既有流程（方向轮、comp-led 门禁、component-review、live、critique、hook）行为不变；完工评审仍只有 recapture/rebuild/fix/ship 四个处置词，走查不产生新处置词 | §0.4、§5.10、§6.1；V-2、V-7、V-10 | 已确认 |
| WORK-N4 | 可维护性 | WORK-N4/AC-1–AC-2（P1 部分；AC-3 属 P4） | 发布包第三方声明保留 oil-ui 的 MIT 许可全文与版权行 “Copyright (c) 2026 oil-oil”；动 skill 文件前 §0.6 整合清单已更新到本期范围，diff 逐项核对 | §0.6、§6.7；V-7、V-10 | 已确认 |
| C-1 | 约束 | AGENTS.md `@skill_tool_independence`；PRD D11、N14 | feature-demo、review-ui、apply-ui-review 的接入只在检测到 Phoenix 能力可用且适用于当前目标时启用；检测不到或不适用照原流程，不安装、不下载、不提示安装、不阻塞；不改它们的录制、计分、门槛与判定 | §6.3–§6.5；V-5、V-9（A4b、A5b） | 已确认 |
| C-2 | 约束 | AGENTS.md「Working in This Repository」接口复述与 UI 评审合同一致 | 新命令、新差距来源与流程变化在同一改动内完整更新全部复述处；review-ui 与 apply-ui-review 及其引用合同同步 | §6.4–§6.6；V-5、V-10 | 已确认 |
| C-3 | 约束 | AGENTS.md §7「Reference Skill Integration Gate」；PRD §5.3、§7.4 | 把 oil-ui 规则/流程或 Phoenix 能力借入既有 skill 前，先按 §0.6 清单逐项给出采纳/调整/拒绝；整合不授权改原则、立场与调用/授权策略 | §0.6；V-10 | 已确认 |
| C-4 | 约束 | skills/phoenix-ui/src/SKILL.src.md:15、:21；[phoenix-ui 计划](phoenix-ui.md) D7 | engine 在 context、检测、hook 中不更新不下载；设计自检一轮批量检查加至多一轮确认；完工评审不关闭独立验收；CLI 只产出证据，不出评审结论 | §5、§6.1 | 已确认 |
| C-5 | 约束 | skills/phoenix-ui/src/subagents/phoenix-ui-finish-reviewer.md:41；tools/phoenix-ui/crates/comp-verbs/src/build_phase.rs:2255-2275、:2663 | 四个处置词及其派生规则不变；comp-led 门禁读取 `.phoenix-ui/review/desktop.png`（1440 宽整页）与 `mobile.png`（390 宽整页）的行为与提示文案不变 | §6.1；V-7、V-10 | 已确认 |
| C-6 | 约束 | tools/phoenix-ui/scripts/render.mjs:112-122、:150-178；scripts/release-phoenix-ui.mjs:153-163 | Phoenix skill 只改 `skills/phoenix-ui/src/`，生成物与 GENERATED.json 由 render/开发构建同步并提交；子代理数量断言（render.mjs:121）随新增子代理改为 5 | §1.2、§6.1；V-5、V-7 | 已确认 |
| C-7 | 约束 | AGENTS.md「Working in This Repository」；mise.toml | 维护工具经 mise（node 25.2.1、bun 1.4.4、rust 1.99.1），Cargo 用 `--locked --offline`；工具缺失记失败不记跳过；浏览器、原生平台与真实宿主会话分别取证，不互相推断 | §9 | 已确认 |
| C-8 | 约束 | docs/phoenix-ui.md:44-53、:124；package.json `engines` | macOS/Linux 四平台；Node ≥18；不新增 Python 运行依赖；Windows 延期 | §7；V-8 | 已确认 |
| C-9 | 约束 | AGENTS.md「Working in This Repository」首条；用户全局规则 | 留在当前分支 main；提交按 §10 提交点；推送只在用户明确授权时进行 | §9 V-8、§10 | 已确认 |
| C-10 | 约束 | scripts/check-phoenix-deps.mjs:457-472 | `crates/` 等目录下新增的 .js/.html/.css 会被自动登记为 Impeccable Apache-2.0 来源；.rs 与 `tests/` 下文件不登记 | D12、§1.2 | 已确认 |
| A-1 | 假设 | 这些 CDP 方法仓内尚未使用，CI runner 浏览器上未实测；旁证：puppeteer-core 25.13.0 的 ARIA 查询就用 `Accessibility.queryAXTree`（tools/phoenix-ui/node_modules/puppeteer-core/lib/puppeteer/cdp/ElementHandle.js:174-187） | 取证所依赖的 `Accessibility.queryAXTree`、`Page.startScreencast`、`Fetch`（含子会话）与 `Input` 域在本机 Chrome 与四平台 CI 浏览器上行为一致 | 本机由 V-2 验证，四平台由 V-8 的 UI Native 浏览器测试验证；实施者负责；最晚 M2 批末（本机）、M6（四平台）。不成立时改用页内 JS 实现对应能力并修订 §5.4/§5.6 | 开放 |

未明确要求的 NFR（取证耗时、走查耗时等）按常规工程做好，不建基线、不设专项检查（§0.5）。

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 形态 | 原生引擎新增 `capture` verb，代码在 `crates/cli/src/capture/`★，复用 `impeccable-browser` 的浏览器发现与 CDP 连接，复用 detect 的页面扫描与项目配置过滤 | 不新增运行依赖与第三方 crate；随现有 engine 分发；启动器无 verb 白名单（phoenix-bootstrap.cjs:309-315），无需改启动器 | ADR-1；WORK-N2；C-8 |
| D2 | 命令名（PRD Q1） | `phoenix-ui capture`；拼图为子命令 `phoenix-ui capture stitch` | 保留 PRD 工作名，避免文档与验收再改名；与内部 verb `capture-server`、comp-led 的 `component-review capture` 在 docs 与 `--help` 中明确区分 | PRD §5.2 Q1；main.rs:91-97；component-review.md:11 |
| D3 | 状态与步骤输入 | 状态组只用 JSON 文件（`--states <file>`），步骤为统一的 `{ "do": …, "target": … }` 对象；不另做行内 DSL | 一种写法，选择器含分号或引号时无歧义，可在启动浏览器前完整校验并指出 `states[i].steps[j]` | §5.2；oil-ui shoot.mjs:168-232 的 DSL 解析分支多、需转义 |
| D4 | 本地目标 | HTML 文件或目录由内置静态服务器经 `http://127.0.0.1:<随机端口>` 提供（根为该目录或文件所在目录）；`http(s)://`、`file://` 原样打开 | 绝对路径资源与模块脚本可用；DESIGN.md 仍按本地路径解析，使初始状态发现与 `detect file://…` 一致 | §5.9；detect cli.rs:663-667；HtmlSnapshot 只服务冻结清单与白名单类型（html_snapshot.rs:85-125、:238-256），不适用任意静态目录 |
| D5 | 设备仿真 | 与 `detect` 相同（`mobile:false`、不开触摸）；像素比用新增 setter；`prefers-color-scheme` 每次显式仿真（默认 light）；`--reduced-motion` 可选 | 初始状态发现可与 `detect --viewport` 对齐；手机视口不仿真触摸与移动 UA | cdp.rs:994-1010；WORK-F1/AC-4 |
| D6 | 页面规则发现 | 把 `scan_page_inner` 导航之后的扫描抽成 `pub fn scan_loaded_page`，detect 与 capture 共用；发现经 `filter_detection_findings` 按项目配置过滤。初始状态按 detect 单目标的时序另开扫描页：视口、像素比、媒体特性、basic-auth 与写拦截同截图页，`goto(url, "networkidle0", 30s)` 后立即扫描，不等动画稳定、不截图；其余状态在截图页上于主截图与派生截图之后扫描。本地目标的发现 `file` 记为该目标的 `file://` 地址，不写临时服务器地址 | 初始状态发现与 `detect --json --viewport` 共用扫描函数与时序，有入场动画的页面也一致；扫描页的滚动揭示不影响截图与 entry 序列帧，代价是每个视口多一次页面加载；非初始状态扫描放在最后，不改变截图 | lib.rs:87、:253-442；config.rs:732；WORK-F1/AC-4、WORK-F3/AC-6 |
| D7 | 有效性自检 | 确定性规则：步骤全部执行、有限动画在预算内结束、截图区域内文字未大面积透明、非整张单色、无异常全黑带、画布已绘出、尺寸与取景符合截图类型；无效图写成 `<名>.invalid.png` 并删除同名旧图 | “内容与文件名所称状态一致”仍由调用方开图核对；旧图不会冒充本轮状态 | §5.5；PRD §2.1「有效证据」 |
| D8 | 报告 | 每个输出目录一份 `capture-report.json`（schema `phoenix-ui-capture-v1`），按文件名合并更新；退出码 0/1/2；终端逐张一行 | 修复轮只重取部分状态时，沿用图的出处仍在同一清单中可查 | §5.8；WORK-F3/AC-3 |
| D9 | 写拦截与凭据 | `Fetch` 在页面会话及自动附加的 iframe/worker 会话上拦截非 GET，并绕过 Service Worker；步骤用 `env` 引用凭据，报告与输出写入前统一脱敏 | 共享实例上不会因核对写入业务数据；密码不进证据文件 | §5.7、§8；WORK-F1/AC-8、WORK-N1/AC-2 |
| D10 | 派生图合成 | 在同一无头浏览器的合成页里用 `<img>`/SVG/文字排版后截图；缩放、裁切、标注、标签都在合成页完成 | 不引入图像与字体 crate；拼图仍需本机浏览器（与 WORK-F2 依赖一致） | ADR-1；shoot.mjs:1333-1429 同类做法 |
| D11 | 走查者操作页面 | 子代理工具为 `Read, Write, Bash, Glob`，通过 `phoenix-ui capture` 每次从新页面重放累积步骤、读截图推进；正文只许在交接的输出目录写状态文件与取证输出，不编辑项目源码和其他文件 | 所有装有 Phoenix 的宿主都能走查；声明 Write 使 Cursor 不生成只读角色（只读模式禁止编辑文件与改变状态的 shell 命令，而走查须写状态文件并运行 capture）；不用宿主浏览器 MCP | ADR-2；factory.js:128-178；[Cursor Subagents](https://cursor.com/docs/subagents) Configuration fields（2026-11-09 查询） |
| D12 | 页内脚本存放 | 新增页内 JS 写成 Rust 常量（`crates/cli/src/capture/page_js.rs`★），夹具放 `tools/phoenix-ui/tests/fixtures/capture/`★ | 不被许可清单误登记为 Impeccable 来源；oil-ui 借鉴部分在 NOTICE 与 THIRD-PARTY-NOTICES 声明 | C-10；WORK-N4/AC-1 |
| D13 | Phoenix 完工接入 | §7 的 Web 取证默认用 capture（两条构建路径共用 §7）；capture 不可用时宿主浏览器按原有效性规则补位并注明；两者都不行时不派评审、交付写“未取证”；走查与评审同轮；评审输入加取证报告与截图类型 | 不改四处置词、轮次上限、comp-led 门禁与 `build_phase.rs` | §6.1；C-4、C-5 |
| D14 | 独立 skill 的检测 | 取证命令：在宿主技能目录找到 phoenix-ui 的 `scripts/phoenix-ui` 且 `capture --help` 退出 0；走查：宿主可派出 `phoenix-ui-ux-walker`，或 phoenix-ui 技能目录有 `reference/degraded/ux-walker.md` 可交给全新上下文子代理；其他结果一律视为未检测到 | 引擎缺失时启动器退出 4 并给补装命令（phoenix-bootstrap.cjs:346-349），不下载；版本不支持时为 `Unknown command`，均按未检测到处理 | §6.3–§6.4；C-1 |
| D15 | 收口验收 | 本机 darwin-arm64 完成开发构建、冒烟、Phoenix UI 全量回归与真实宿主会话；四平台与 Node18 消费者用现有 `.github/workflows/phoenix.yml`（push 触发），推送须用户授权；Claude Code 跑 A1–A5，Codex 跑 A6 | 未获授权推送时 V-8 记未运行，收口不宣称 WORK-N2/AC-1 通过 | §9；C-9；docs/phoenix-ui.md「维护者：统一候选与本地交接」 |

### 0.3 ADR-lite

#### ADR-1：取证命令用原生 Rust 实现在现有 CDP 层之上

- 状态：Accepted
- 背景与驱动：WORK-F1/AC-4 要求每个状态的页面规则发现与 detect 一致；WORK-N2 要求随 engine 分发、Node ≥18、无 Python；PRD D2 已定“做成 `phoenix-ui` 命令”；C-4 要求不下载。
- 备选：
  - ① **原生 verb，扩展 `impeccable-browser` 的 CDP 能力**：非初始状态的发现与截图在同一页面状态上完成，初始状态与 detect 共用扫描函数与时序（D6），天然一致。
  - ② 移植 shoot.mjs 为随 skill 分发的 Node 脚本：它要求 Node 22（shoot.mjs:476）且自带浏览器启动；非初始状态的规则发现只能另起 detect 重放步骤，无法保证同一状态，淘汰。
  - ③ 引入 Playwright/Puppeteer：会下载浏览器或新增大依赖，违背 C-4 与 PRD D2，淘汰。
- 决策：①。输入事件、无障碍查询、屏幕录制帧、媒体仿真、网络日志、写拦截作为 `Page` 的增量方法加入 `crates/browser/src/cdp.rs`；编排、校验、报告与合成放在 `crates/cli/src/capture/`。
- 正面后果：一个二进制、一套浏览器发现与配置过滤；四平台随现有 CI 构建。
- 负面/中性后果：步骤执行、定位提示与合成都要在 Rust 中重写，不能直接复用 shoot.mjs 代码；`cdp.rs` 是 detect 与 comp 门禁共用层，改动需回归（V-2 定向、V-7 全量）。
- 重新评估触发：上述 CDP 方法在某平台浏览器上不可用（A-1 不成立）；用户要求 iframe/Shadow DOM 内步骤（PRD Q4）；需要跨调用保持会话的交互式浏览。

#### ADR-2：走查子代理通过 capture 重放操作页面

- 状态：Accepted
- 背景与驱动：WORK-F4 要求走查者只操作真实页面、不读源码与任务、不编辑文件，且“在任何宿主上”可用；依赖写明“宿主浏览器或 WORK-F1 的步骤操作”。
- 备选：
  - ① **`tools: Read, Write, Bash, Glob`，用 capture 每次从新页面重放累积步骤并读截图**：所有安装 Phoenix 的宿主一致可用；声明 Write 使 Cursor 不生成只读角色（factory.js:172-175），否则无法写状态文件与运行 capture；写入由正文限定在交接的输出目录；每步都有截图文件可作证据。
  - ② 省略 `tools` 继承宿主全部工具以使用浏览器 MCP：交互更顺手，但可用性随宿主而变，且授予 Edit 在内的全部工具，没有统一证据文件。
  - ③ 主代理操作、走查者只看截图：PRD 拒绝静态走查顶替（§5.3 C3），淘汰。
- 决策：①。走查者只在交接给的输出目录里写步骤文件与截图，不编辑项目源码和其他文件；最终报告作为返回值交给主 agent 保存。
- 正面后果：可复现（报告里的步骤文件就是复验脚本）；不需要宿主浏览器。
- 负面/中性后果：每次探查都从新页面重放，较慢；登录与安全边界内可重复的测试操作随累积路径重放，不能安全重复的步骤只作为最后一次运行的最后一步执行一次，此后的路径列为未验证（§6.2）。
- 重新评估触发：重放造成测试数据重复且影响判断，或用户要求跨调用保持会话。

### 0.4 职责与事实所有权

|  | `phoenix-ui capture`（本期） | Phoenix 主代理流程（本期改 §7） | 完工评审 / 走查子代理 | feature-demo / review-ui / apply-ui-review（本期可选接入） |
| --- | --- | --- | --- | --- |
| 拥有 | 浏览器中实际执行的步骤与结果、截图像素、有效性自检标记、出处、控制台/网络/溢出记录、页面规则发现、拦截记录 | 声明哪些状态、交给谁、修复与复验、交付报告中处置词与走查结论的呈现 | 评审：处置词与修复清单；走查：产品理解、实际路径、≤3 条问题（事实/推断/建议） | 各自的录制、计分、门槛、判定、差距清单与处置 |
| 不拥有 | **不判断内容是否就是所称状态，不打分，不出处置词，不读写项目源码** | **不改处置词，不把自走当独立走查，不把走查结论并入处置词** | 评审不浏览页面；走查不读源码、任务、设计说明，不编辑项目文件（只在交接的输出目录写取证材料），只用测试凭据 | **不以 Phoenix 为执行前提，不安装或下载 Phoenix，不把 capture 自检当作开图核对** |

一句话：capture 只证明“页面在这些步骤后渲染成这样、证据本身没坏”；状态是否对、好不好、要不要修，分别归调用方、评审与用户。

### 0.5 明确不在本期

- **P2–P4 需求**——WORK-F5–F14、WORK-N1/AC-4、WORK-N2/AC-3–AC-4、WORK-N3/AC-3、WORK-N4/AC-3 由后续期计划承接；本期不改 detect 规则、typeset、素材、图片处理、对比页、动效与细节指引。
- **npm 发布与 R2 上传**——属发布流程（docs/phoenix-ui.md「维护者」节），需用户另行授权；注意新增 verb 改动了 `tools/phoenix-ui`，下次发布不能用 `--reuse-engines`，须走 CI 构建（release-phoenix-ui.mjs:139、:147）。
- **iframe / Shadow DOM 内执行步骤（PRD N13、Q4）**——命令报出限制并提示直接打开被嵌入页面；独立 skill 对这类目标沿用自身流程。
- **录屏视频（N12）、原生 iOS/Android 取证（N8）、Windows（N9）**。
- **comp-led 门禁、`build_phase.rs` 提示文案、component-review、live、critique、方向轮与 hook**——行为不变（WORK-N3/AC-1）；hallmark 计划延期的规则不在本期（PRD §1.1 关联）。
- **触摸仿真与移动 UA**——与 detect 保持同一设备仿真（D5）。
- **review-ui 的计分维度、门槛、判定规则与 feature-demo 的录制、视频检查、写入规则和报告模板**——只增加可选接入（C-1）。
- **SPMS 录入与查重**——PRD §8 待办，不在开发计划内。
- **取证耗时、走查耗时的基线与调优**——后续可选；本期按常规做法（单浏览器复用、按需扫描）。

### 0.6 参考整合清单（动 skill 文件前核对；N4/AC-2、C-3）

Phoenix 与各独立 skill 的现有原则为基线；整合不改变它们的原则、立场与调用/授权策略。M4、M5 开始改 skill 文件前，按本表逐项读一遍当前目标段落；实施中出现表外行为差异，先改本表并解决冲突再继续。

| ID | 目标文件 / 规则 | 现有行为 | 参考来源 / 段落 | 采用、调整或拒绝 | 对原则、调用/授权、接口与验收的影响 | 核对 |
| --- | --- | --- | --- | --- | --- | --- |
| I-1（PRD C1） | `skills/phoenix-ui/src/reference/new-work.md` §7（:146–:162，含 `rule:skill-capture-validity`、派出评审、`fix` 段）；`src/subagents/phoenix-ui-finish-reviewer.md` Input Contract（:25）与检查 0（:29） | 宿主截图、主代理开图自验；评审按文件名推断尺寸 | oil-ui `references/tools.md:15-24`「什么时候截，截多少」 | 调整：capture 为 Web 默认证据源；保留文件名、两轮上限、四处置词；评审按报告中的截图类型判尺寸；拒绝为截图开发 `?state=` 路由（tools.md:19） | 不改轮次上限与处置词；评审输入新增取证报告 | V-5、V-10 |
| I-2（C2） | 新 verb `capture`（`crates/cli/src/capture/`★） | 无 | `scripts/shoot.mjs` `--help`（:14-127）、步骤与定位（:164-232、:1056-1257）、settle（:927-942）、mask（:987-990）、mark（:992-1029）、sheet（:1415-1429）、stitch（:1333-1412）、frames（:1557-1609） | 采用：步骤操作、多视口、失败提示、mask/mark/sheet/stitch/frames 方法；调整：步骤改 JSON（D3），可读性检查改用 detect 页面规则，新增角色定位、等待消失、配色、写拦截、凭据引用、输出路径与出处；拒绝 `--compare` 像素差分（comp-diff 已覆盖）、`--record`、0.5 倍 `--thumb`（改为 480/180 扫视缩略图）、`--param`/`?state=` | 新公开接口；D7（CLI 不打分）不变 | V-1–V-4 |
| I-3（C3） | 新子代理 `src/subagents/phoenix-ui-ux-walker.md`★；new-work §7 派发与处理 | polish 主代理走查（polish.md:87）、critique persona（critique.md:165-167） | oil-ui `references/ux-review.md`（「默认什么时候派」「交接什么」「收到报告以后」）、`references/ux-reviewer.md` 第 1–4 条与输出段 | 采用：交接限制、报告格式、三类分诊、≤3 条问题、不代表真实用户；调整：与完工评审同轮、可复现卡住/走错必修并由主代理复验、处置词不变、走查单列、操作方式改为 capture 重放（ADR-2）、无隔离子代理时记“未完成 UX 走查”；拒绝静态 UX 走查顶替 | 不替代 review-ui；不改处置词 | V-6、V-10 |
| I-4（C14） | `tools/phoenix-ui/NOTICE.md`、`tools/phoenix-ui/THIRD-PARTY-NOTICES.md` | 无 oil-ui 条目 | `local/oil-ui-main/LICENSE`（MIT，第 3 行版权） | 采用：按 ehmo 先例（NOTICE.md:11-17）写来源与改编范围，THIRD-PARTY-NOTICES 附 MIT 全文 | 许可合规 | V-7 |
| I-5（C15） | `skills/feature-demo/SKILL.md`：`@write_demo`（:44-51）与 `@recording`（:53-56）之间新增录制前核对规则；`@failed_step`（:58-61） | 录制失败只留 `failed.png`；写拦截规则在 setup（:37） | PRD WORK-F15；I-2 接口 | 采用为可选增强，检测到才用；拒绝以 Phoenix 为前提 | 不改录制、视频检查、写入规则与报告模板；复制的脚本仍不依赖 skill（:22） | V-5、V-9 |
| I-6（C16） | `skills/review-ui/SKILL.md` 工作流（:59-72）；`references/target-and-capture.md` 采集与出处（:95-113） | 运行时发现采集工具（SKILL.md:42）；真实性与出处合同；机器检查不进评审 | PRD WORK-F16；I-2 | 采用为可选增强；调整：命令发现只进机器检查，命令不适用的单元回到现有采集方式；拒绝以命令自检替代主 agent 开图核对 | 不改计分、门槛与判定；`@tool_independence`（:53-56）保持 | V-5、V-9 |
| I-7（C17） | review-ui `report-contract.md`:37、:57、:62-68、:78、:94；`judge.md`:12、:79；`SKILL.md` `@independent_judge`（:204-212）与工作流；apply-ui-review `SKILL.md` §2（:42-50）、§4（:67）、`references/record-contract.md`:55 | 独立评审按目标包评分；无盲测走查；apply 无按来源的处理 | PRD WORK-F17；ux-review.md、ux-reviewer.md | 采用为可选增强；调整：差距来源「盲测走查」、主 agent 定严重度与复查、P0/P1 计入门槛项、走查材料不进评审；apply-ui-review 复走同一路径并记录 | 不改计分与判定；两份合同同步（C-2） | V-5、V-9 |

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·足够] 浏览器发现与临时资料目录**：`discovery::find_browser` 依次读 `PHOENIX_UI_BROWSER`、`PUPPETEER_EXECUTABLE_PATH`、`CHROME_PATH` 再查标准路径（crates/browser/src/discovery.rs:30-52）；`Browser::launch` 在系统临时目录建 `phoenix-ui-chrome-profile-<pid>-<n>` 并以 `--user-data-dir` 启动，`Drop` 删除（crates/browser/src/cdp.rs:123-232、:314-322）。WORK-N1/AC-1 直接复用，只需验证。
- **[已核实·足够] 页面规则扫描**：`detect_url_impl` → `scan_page_inner` 在导航后注入快照脚本、原生运行规则（含 `phoenix_ui_rules::PACK`）、执行滚动揭示、隐藏文字度量、脚本错误与视觉对比度兜底（crates/browser/src/lib.rs:253-442）；URL 目标的 DESIGN.md 只对 `file://` 解析（crates/detect/src/cli.rs:253-269、:663-667），结束时按项目配置过滤（crates/detect/src/config.rs:732）。
- **[已核实·缺口] 设备与媒体仿真**：`set_viewport` 固定 `deviceScaleFactor: 1`、`mobile: false`（cdp.rs:994-1010）；`set_reduced_motion` 只设一个媒体特性（cdp.rs:987-992）→ 新增像素比 setter 与同时设置配色和减少动态的媒体方法 → 否则无法满足 WORK-F1 的像素比与配色。
- **[已核实·缺口] 输入事件、无障碍查询、屏幕录制帧**：仓库内无 `Input.dispatch*`、`Accessibility.queryAXTree`、`Page.startScreencast` 调用（grep `crates`）→ 在 `Page` 增加对应方法 → 否则无法执行步骤、按角色定位与取序列帧。
- **[已核实·缺口] 控制台与网络记录**：`Page` 只收 `Runtime.exceptionThrown`（cdp.rs:837-856）；`ResponseCapture` 只服务主文档响应体证据，不记方法与失败原因（crates/browser/src/response_capture.rs:83-128）→ 新增 `console_errors` 收集与 `network_log.rs`★ → 否则报告缺控制台错误与失败请求。
- **[已核实·缺口] 请求拦截**：`Fetch.requestPaused` 只为同源 basic-auth 注入服务，按单一会话匹配（cdp.rs:368-420、:1016-1043）→ 改为按会话登记的拦截器，同时支持 auth 与写拦截，并在自动附加的 iframe/worker 会话上启用（cdp.rs:722-790 的 attach 处理）→ 否则无法拦截非 GET。
- **[已核实·缺口] 命令入口**：verb 路由在 crates/cli/src/main.rs:43-138，总帮助 `ROOT_USAGE` 在 crates/detect/src/lib.rs:53-67；无 `capture` → 新增路由、总帮助一行与 `capture --help`。
- **[已核实·足够] 启动器透传**：`scripts/phoenix-ui` → `phoenix-bootstrap.cjs` 校验 manifest、摘要与 `engine-probe` 后透传任意 verb；engine 缺失或不符时 stderr 给补装命令并退出 4，不下载（skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs:77-91、:309-349）。WORK-N2/AC-1 的缺 engine 行为无需改动。
- **[已核实·足够] 子代理生成**：`src/subagents/*.md` 自动生成 Claude/Cursor/Copilot/Grok 原生定义、Codex TOML 与 `reference/degraded/<名>.md`（tools/phoenix-ui/scripts/lib/transformers/factory.js:357-436，utils.js:184-213），项目安装按 bundle 清单复制（crates/install/src/lib.rs:234-300）；只有 render.mjs:121 断言子代理数为 4。
- **[已核实·缺口] Phoenix 完工取证**：new-work.md §7 要求宿主截图并自验（src/reference/new-work.md:146-150），评审“没有浏览器”且检查 0 只按文件名判尺寸（phoenix-ui-finish-reviewer.md:19、:29）；无走查子代理 → §6.1、§6.2。
- **[已核实·缺口] 三个独立 skill**：均未提及 phoenix-ui；feature-demo 只在录制失败时截 `failed.png`（scripts/record-demo.mjs:119-122）；review-ui 差距来源只有评审、机器检查、主 agent 核实（references/report-contract.md:78、:94）；apply-ui-review 不按来源处理（SKILL.md:42-50）→ §6.3–§6.5。
- **[已核实·足够] 测试基础**：二进制级测试用 `env!("CARGO_BIN_EXE_phoenix-ui")`，无浏览器时打印跳过（crates/cli/tests/xgent_rules.rs:15-17、:303）；`test:phoenix` 在 macOS 把浏览器测试放在 sandbox-exec 之外（scripts/test-phoenix-ui.mjs:44-48），并最后回放 oracle（:50）；oracle 约定“每个 verb 都有端到端 golden”，`record.mjs` 只重录有意变化的用例（tools/phoenix-ui/tests/oracle/README.md:1-20）。

### 1.2 拓扑与文件清单

范围内全部位于本仓。★ 为新增。

| 位置 | 改动 | 所属 |
| --- | --- | --- |
| tools/phoenix-ui/crates/cli/src/capture/★ `mod.rs`（参数、编排、`--help`）、`plan.rs`（状态文件校验）、`steps.rs`（定位、动作、失败提示）、`validity.rs`、`derived.rs`（帧采样、合成、拼图）、`report.rs`（schema、合并、脱敏、终端行）、`serve.rs`（静态服务器）、`page_js.rs`（页内脚本常量） | 取证命令实现 | M1–M3 |
| tools/phoenix-ui/crates/cli/src/main.rs、lib.rs | 路由 `capture`；`pub mod capture`。Cargo.toml 不变，Cargo.lock 只在 M6 经用户授权升级 rustls、rustls-webpki、webpki-roots（重新固定 RustSec 库后的漏洞修复）：PNG 解码用已依赖的 `impeccable_comp::png_io::decode_png`（crates/comp/src/png_io.rs:32），路径解码照 html_snapshot.rs:143-166 手写，SHA-256 用已依赖的 `sha2` | M1 |
| tools/phoenix-ui/crates/browser/src/cdp.rs | 新增 `Page` 方法与事件收集；拦截器按会话登记（§5.10） | M1–M3 |
| tools/phoenix-ui/crates/browser/src/network_log.rs★、lib.rs | 网络日志；抽出 `pub fn scan_loaded_page`，`reveal_sweep` 改为 `pub fn`，供 `crates/cli` 的整页图跨 crate 复用（保持一份实现） | M1–M2 |
| tools/phoenix-ui/crates/detect/src/cli.rs、lib.rs | 新增 `pub fn url_scan_options`；`ROOT_USAGE` 增加 capture 一行 | M1 |
| tools/phoenix-ui/crates/cli/tests/capture.rs★、tools/phoenix-ui/tests/fixtures/capture/★ | 二进制级测试与夹具（应用页、状态文件、有效性/写通道/脱敏夹具、测试服务器脚本 `serve.mjs`） | M1–M3 |
| tools/phoenix-ui/tests/oracle/cases/capture.mjs★、golden/capture-*.json★、golden/cli-help.json | 离线路径 oracle 用例；总帮助 golden 重录 | M6 |
| tools/phoenix-ui/scripts/render.mjs:121 | 子代理计数 4 → 5 | M4 |
| tools/phoenix-ui/NOTICE.md、THIRD-PARTY-NOTICES.md（及 `--write` 重算的 THIRD-PARTY.json、THIRD-PARTY-LICENSES/、dependency-audit.json） | oil-ui 署名与 MIT 全文 | M6 |
| scripts/smoke-phoenix-ui.mjs | 增加 capture 冒烟（§9 V-7） | M6 |
| skills/phoenix-ui/src/reference/new-work.md、src/subagents/phoenix-ui-finish-reviewer.md、src/subagents/phoenix-ui-ux-walker.md★、skills/phoenix-ui/evals/scenarios.md；生成物 skills/phoenix-ui/**、GENERATED.json、dist/phoenix-ui/**（开发构建同步） | 完工取证与走查 | M4、M6 |
| skills/feature-demo/SKILL.md | 录制前核对（可选） | M5 |
| skills/review-ui/SKILL.md、references/target-and-capture.md、references/report-contract.md、references/judge.md、evals/scenarios.md、evals/fresh-context.md | 采集与走查（可选） | M5 |
| skills/apply-ui-review/SKILL.md、references/record-contract.md、evals/scenarios.md | 「盲测走查」差距复走 | M5 |
| docs/phoenix-ui.md、README.md | 取证命令一节；辅助 skill 可选接入的复述 | M4、M5 |

无需修改的邻接面：启动器与安装器（verb 透传、bundle 清单复制，见 §1.1）；`crates/comp-verbs`（含 `build_phase.rs`）、`crates/live`、`crates/hook`、`crates/context`；`skills/phoenix-ui/src/reference/` 除 new-work.md 外的文件；`.github/workflows/phoenix.yml`（push 路径已含 `tools/phoenix-ui/**`、`skills/phoenix-ui/**`、`scripts/*phoenix*.mjs`）。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `capture::run(args, io) -> i32`（mod.rs） | main.rs 路由 | §5.1 参数；先完整校验参数与状态文件（失败退出 1，不启动浏览器）；浏览器启动失败退出 1 且不建输出目录 | 无 | 视口 × 状态编排、单浏览器复用、输出与报告的原子写入顺序 | 进程内 | 二进制级（V-1、V-2） |
| `capture::plan`（plan.rs） | run | `parse_states(json) -> Result<Plan, Vec<PlanError>>`，错误带 `states[i].steps[j]` 路径与就近建议 | 无 | 字段校验、未知键、保留名、命名模板冲突 | 进程内 | 单元（V-2） |
| `capture::steps`（steps.rs） | run | `resolve(page, &Locator) -> Resolved{rect, warnings} \| LocateError{reason, hints}`；`execute(page, &Step, secrets)` | 无 | 文字定位优先级、角色查询、可操作性判断、失败提示生成 | 本地可替代（真实 Chrome） | 二进制级（V-2） |
| `capture::validity`（validity.rs） | run | `check(png, &ShotMeasure, &ShotSpec) -> Vec<String>`（空即有效） | 无 | 像素规则与按截图类型的尺寸规则 | 进程内 | 单元（V-2） |
| `capture::derived`（derived.rs） | run、`capture stitch` | `sample_frames(frames, t0, t1, n)`、`compose(page, Layout) -> png`、`stitch(images, match, crop, labels)` | 无 | 帧采样、缩放裁切几何、合成页布局 | 本地可替代 | 单元（V-4）+ 场景（V-3） |
| `capture::report`（report.rs） | run | `merge(existing, run) -> Report`（按 `file` 替换）、`redact(value, secrets)`、`print_lines` | 无 | 合并、脱敏、退出码 | 进程内 | 单元（V-2） |
| `capture::serve`（serve.rs） | run | `serve(root) -> Server{url_for(entry)}`；仅 127.0.0.1、GET/HEAD | 无 | 路径规范化与拒绝规则 | 进程内 | 二进制级（V-1） |
| `impeccable_browser::scan_loaded_page(page, url, &ScanOptions, Viewport)` | detect_url_impl、capture | 与原 `scan_page_inner` 导航后部分逐行等价，返回映射后的 `Finding` | 无 | 规则扫描全流程 | 本地可替代 | V-2（xgent_rules 与一致性用例）、V-7 oracle |
| `impeccable_detect::cli::url_scan_options(cwd, home, local_path, rule_pack) -> (ScanOptions, DetectionConfig)` | capture | 与 `detect` 默认参数（未加 `--no-config` 等）时的选项一致 | 无 | 配置读取与 DESIGN.md 解析 | 进程内 | V-2 一致性用例 |

只有一个调用方的逻辑不另抽共享模块；`scan_loaded_page` 与 `url_scan_options` 是为了让 capture 与 detect 共用同一段扫描与选项代码（WORK-F1/AC-4），不是新的抽象层。

## 3. 数据模型与迁移

不涉及持久化与迁移。输入输出文件格式（状态文件、报告）在 §5.2、§5.8 定义；报告带 `schema` 版本，后续期扩展字段只增不改。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| 本机 Chrome/Chromium/Edge | 已核实足够 | discovery.rs:30-52；CI 在 Linux ARM 设 `PHOENIX_UI_BROWSER`（phoenix.yml 准备步骤） | 零改动；capture 自带不可用提示 | 退出 1：`浏览器不可用：<发现失败原因>`，不建输出目录（§5.8） |
| CDP 实验性域（A-1） | 未核实（本机与 CI 浏览器） | puppeteer-core ElementHandle.js:174-187 用 `Accessibility.queryAXTree` | V-2 本机、V-8 四平台验证 | 方法报错时该步骤失败并带原错误；不静默改用别的定位 |
| detect 扫描与配置 | 已核实足够 | lib.rs:253-442；cli.rs:253-269；config.rs:732 | 抽出共用函数（§5.10），detect 行为不变 | 扫描出错：该图 `findings` 记 `error` 字段，图本身有效性不受影响，退出码 2 |
| Phoenix 启动器 | 已核实足够 | phoenix-bootstrap.cjs:77-91、:309-349 | 零改动 | engine 缺失/不符：退出 4 + 补装命令 |
| render 与开发构建 | 已核实足够 | render.mjs:112-178；scripts/build-phoenix-ui.mjs:13-21、:99-123 | 计数断言改 5；收口时同步生成物 | 计数不符或占位残留时 render 断言失败 |
| 第三方许可清单 | 已核实足够 | check-phoenix-deps.mjs:457-523 | NOTICE/THIRD-PARTY-NOTICES 手写后 `--write` 重算 | 未 `--write` 时报 “Third-party inventory is stale” |
| 宿主子代理能力 | 已核实足够（finish-reviewer 同机制） | new-work.md:152 的各宿主派出写法；factory.js:385-436 | 新增走查子代理随同一机制生成 | 宿主不能派隔离子代理：交付写“未完成 UX 走查”（WORK-F4/AC-5） |
| Phoenix CI（四平台 + Node18） | 已核实足够 | .github/workflows/phoenix.yml:1-60（push 路径、四平台矩阵）、:181-205（Node 18 消费者） | 零改动；冒烟脚本增加 capture | 推送未授权：V-8 未运行，收口不宣称 WORK-N2/AC-1 |
| feature-demo / review-ui / apply-ui-review | 已核实缺口 | §1.1 | 文本增量（§6.3–§6.5），只作可选接入 | 检测不到即照原流程 |

## 5. 核心机制

### 5.1 命令接口

```text
phoenix-ui capture <target> [options]
phoenix-ui capture stitch <image> <image>... --match <px,px,...> [--crop X,Y,W,H] [--label a,b,...] [--columns N] --out <file.png>
```

- `<target>`：`http://`、`https://`、`file://` URL 原样打开（URL 中的 basic-auth 用户信息按 `split_scan_url` 剥离并以同源注入，lib.rs:166-183）；本地 `.html`/`.htm` 文件或目录由内置静态服务器提供（§5.9）。
- `--viewport <列表>`：逗号分隔，元素为 `desktop`（1440x900）、`mobile`（390x844）、`<W>x<H>` 或 `<标签>=<W>x<H>`（标签 `[a-z0-9-]+`）；默认 `desktop,mobile`；`<W>x<H>` 的标签为 `<W>x<H>`。宽高 100–4096。
- `--dpr <n>`：1–4，默认 1。`--color-scheme light|dark`：默认 light。`--reduced-motion`：仿真 `prefers-reduced-motion: reduce`。
- `--states <file>`：状态文件（§5.2）；不给时只取初始状态。`--only <名,…>`：只取列出的状态，`initial` 指初始状态。
- `--type viewport|full`：未在状态里声明类型时的默认截图类型，默认 `viewport`。
- `--out <dir>`：默认 `.phoenix-ui/capture`（相对当前目录）。`--name <模板>`：默认 `{viewport}{-state}.png`；占位符 `{viewport}`、`{state}`（初始状态为 `initial`）、`{-state}`（初始状态为空，其他为 `-<名>`）、`{width}`、`{height}`、`{dpr}`、`{scheme}`；展开后重名在启动浏览器前报错。`--report <file>`：默认 `<out>/capture-report.json`。展开后重名在启动浏览器前报错；主图、无效图与本次请求的派生图（序列帧/遮字图/缩略图/标注图/并排图）同名也在此报错，一个文件名只对应一张图。
- `--block-writes`：中止非 GET 请求并记录；`--allow-write <路径>` 可重复，按 URL pathname 精确匹配，末尾 `*` 表示前缀。
- `--settle-ms <ms>`：动画稳定预算，默认 2000；`--timeout-ms <ms>`：`waitFor` 默认超时，默认 5000（两者沿用 shoot.mjs:1921-1927 的 2000ms 动画预算与 :1267-1273 的 5s 等待）。本节其余上限（视口 100–4096、像素比 1–4、帧数 2–60、`wait` ≤10000ms）是本计划为参数校验定的取值。
- 派生：`--mask`、`--thumbs`、`--sheet`；序列帧与标注在状态里声明（§5.2、§5.6）。
- `--json`：把报告 JSON 打到 stdout，代替逐行输出。`--help`：用法（列出以上参数、状态文件示例、退出码，并注明与 `capture-server`、`component-review capture` 无关）。

### 5.2 状态文件

```json
{
  "initial": { "type": "full", "frames": { "count": 8, "from": "entry" }, "mark": [{ "css": "header nav" }] },
  "states": [
    {
      "name": "open-filter",
      "steps": [
        { "do": "fill", "target": { "role": "textbox", "name": "用户名" }, "value": "demo" },
        { "do": "fill", "target": { "css": "#password" }, "env": "DEMO_PASSWORD" },
        { "do": "click", "target": { "role": "button", "name": "登录" } },
        { "do": "waitFor", "target": { "text": "任务" } },
        { "do": "click", "target": { "role": "button", "name": "筛选" } }
      ],
      "type": "viewport",
      "frames": 8,
      "mark": [{ "role": "button", "name": "应用筛选" }]
    }
  ]
}
```

- 顶层只允许 `initial`、`states`；未知键、重复状态名、名为 `initial` 的状态、空 `steps` 均为错误。状态名 `[a-z0-9][a-z0-9-]*`。
- `type`：`"viewport"`、`"full"` 或 `{ "element": <定位> }`。`frames`：整数 2–60，或 `{ "count": n, "from": "last-step" | "entry" }`（`entry` 只允许在 `initial`）。`mark`：定位数组，编号按数组顺序从 1 开始。
- 定位：`{ "css": "…" }`、`{ "text": "…" }`、`{ "role": "…", "name": "…" }`（`name` 可省），均可加 `"nth": n`（从 1 起，按文档顺序在可见匹配中取第 n 个；默认取第一个可见匹配）。
- 动作 `do`：

| do | 必填 | 可选 | 行为 |
| --- | --- | --- | --- |
| `click` / `dblclick` / `hover` | `target` | — | 滚入视口，按元素中心派发真实鼠标事件（`dblclick` 为 clickCount 2） |
| `fill` | `target`，`value` 或 `env` | — | 点击聚焦，全选后 `Input.insertText` 写入 |
| `type` | `value` 或 `env` | `target` | 有 `target` 先点击聚焦，否则写入当前焦点元素 |
| `select` | `target`（原生 `<select>`），`value` | — | 按 option 的 value 或可见文字选中，派发 input/change |
| `press` | `key` | — | 如 `Enter`、`Tab`、`Escape`、`ArrowDown`、`Control+A`、`Meta+K` |
| `drag` | `target`，`dx`，`dy` | — | 从元素中心按 24 步移动后松开 |
| `scroll` | `dy` | `dx`，`target`（滚动容器） | 默认滚动窗口 |
| `wait` | `ms`（≤10000） | — | 固定等待 |
| `waitFor` | `target` | `until`：`visible`（默认）/ `hidden`；`timeoutMs` | `hidden` 在没有可见匹配时成立 |

- `env` 读取同名环境变量；未设置时该状态在启动前即判失败（“环境变量 DEMO_PASSWORD 未设置”）。所有 `env` 值进入本次运行的脱敏集合（§5.7）。

### 5.3 每个状态 × 视口的执行顺序

1. 启动一次浏览器（发现失败 → 退出 1）；本地目标先起静态服务器。
2. 对每个视口、每个状态（初始状态在前，其余按文件顺序，受 `--only` 过滤）在独立浏览器上下文中新开页面（不共享 cookie、存储与缓存，前一状态的登录或写入不带入下一状态）：设置像素比与视口、媒体特性；`--block-writes` 时启用写拦截并绕过 Service Worker；初始状态声明 `from: entry` 的序列帧时先在 `about:blank` 开始录帧，收到基准帧后再导航（§5.6）。
3. `goto(url, "networkidle0", 30s)`，与 detect 单目标一致（lib.rs:87、:361）。
4. 依次执行步骤；某步失败 → 该状态 × 视口记失败（§5.4），不截图，关闭页面，继续下一个。`frames` 为 `last-step` 时在最后一步动作前开始录帧，收到基准帧后再派发该动作（§5.6）。
5. 稳定：在 `--settle-ms` 预算内轮询 `document.getAnimations()`，直到没有运行中的有限时长动画，再等两帧；整页类型先执行与 detect 相同的滚动揭示（lib.rs:447-464）后再稳定一次。
6. 度量（同一时刻）：剩余有限动画数、截图区域内文字字符数与累积不透明度 < 0.05 的字符数、区域内 ≥32×32 的 `<canvas>` 盒子、文档宽高、元素类型的盒子、横向溢出（`scrollWidth > innerWidth + 1`）、加载失败的图片（`complete && naturalWidth == 0` 且有 src）、`mark` 各定位的盒子。
7. 主截图：视口图 = `captureBeyondViewport:false` 的当前视口；整页图 = 文档顶部起、宽为视口宽、高为文档高的裁切；元素图 = 滚入视口后的元素边框盒。像素均为 CSS 尺寸 × 像素比。
8. 派生截图（同一状态）：`--mask` 时注入遮字样式表截同类型图后移除；停止录帧。
9. 页面规则发现（非初始状态）：`scan_loaded_page`，再按项目配置过滤（D6）。
10. 收集该页面的控制台错误、网络记录与拦截记录，关闭页面。初始状态的截图页关闭后，另开扫描页取初始状态发现（D6）：像素比、视口、媒体特性、basic-auth 与写拦截同截图页，`goto(url, "networkidle0", 30s)` 后立即 `scan_loaded_page` 并按项目配置过滤，不执行第 5–8 步；该页的控制台、网络与拦截记录不重复登记。
11. 全部完成后统一合成派生图（§5.6）、逐张有效性判定（§5.5）、写文件与报告（§5.8）。

### 5.4 定位、动作与失败提示

- 只在主文档查找：CSS 用 `document.querySelectorAll`；文字先找 innerText（去首尾空白、合并空白）完全相等的可见元素，没有再找包含者，取最深层，优先可交互元素（a、button、input、select、textarea、summary、label、带 role 或 tabindex 者）；角色用 `Accessibility.queryAXTree`（对 document 节点，给 `role` 与可选 `accessibleName`），过滤 ignored 与非元素角色后，只保留 `getRootNode() === document` 的节点。
- 可见：有非零盒子、非 `display:none`/`visibility:hidden`、累积不透明度 > 0。
- 可操作性：目标找到但不可见 → 失败，原因写明（零尺寸 / 隐藏 / 透明 / 在视口外不可滚入），提示“先执行让它出现的那一步”；被遮挡（中心点 `elementFromPoint` 不是目标或其后代）或禁用（`disabled`、`aria-disabled="true"`、`pointer-events:none`）→ 记为警告并照常执行（真实点击会落在遮挡物上）。
- 找不到：失败，提示最多 3 个可见候选的定位写法（同角色的元素，或可见文字包含所给文字任一词的可交互元素），格式就是状态文件里的 JSON，如 `{"role":"button","name":"筛选"}`；页面里有 iframe 或开放 shadow root 时追加“选择器只作用于主文档：页面有 N 个 iframe、M 个开放 shadow root，可直接打开被嵌入的页面”。
- `waitFor` 超时：失败，写明等待的条件、超时毫秒数与当前可见匹配数。
- 动作后：若主框架开始新导航则等 `load`（≤30s），然后等两帧。每步在报告中记 `{index, do, target(展示用), result: ok|warning|failed, observed}`，`observed` 只写实际观察（如“导航到 /tasks”“出现 3 个匹配，取第 1 个”），不写预期。

### 5.5 有效性自检

一张主截图在以下任一条件成立时无效，原因逐条写入报告（WORK-F3/AC-2、AC-7；PRD §2.1）：

1. 状态的声明步骤没有全部执行（此时不产出截图，记在 `failures`）。
2. 稳定预算结束时仍有运行中的有限时长动画：“动画未稳定（N 个仍在运行）；加大 --settle-ms 或用 --reduced-motion”。无限循环动画不计。
3. 截图区域内可见文字 ≥20 字符且其中 ≥50% 的累积不透明度 < 0.05：“主要内容透明，入场动效未稳定或停在隐藏态”。
4. 整张图所有像素在 ±2 色差内为同一颜色：“整张为单一颜色，页面未绘出”。
5. 存在连续纯黑行（各通道 ≤8）高度 ≥ 图高 25%，且页面根背景色亮度 ≥ 0.2：“出现大面积全黑区域”。
6. 区域内 ≥32×32 的 `<canvas>` 在截图中对应像素全部同色：“画布未绘出：<选择器>”。
7. 尺寸与取景不符：视口图像素 ≠ 视口宽高 × 像素比；整页图宽 ≠ 视口宽 × 像素比，或高 ≠ 文档高 × 像素比（四舍五入），或裁切不是从文档顶部开始；元素图 ≠ 元素盒 × 像素比。整页图高于视口、元素图小于视口本身不算无效。

第 3–6 条的阈值（20 字符、50%、±2、≤8、25%、亮度 0.2、32×32）是本计划定的判定值，取保守一侧以避免把正常页面判无效；V-2 第⑤项按这些值验证，实测误判时就地修订本节。无效图写成 `<名>.invalid.png` 供调用方查看；同名的 `<名>` 旧文件在本次运行中删除，失败状态同样删除同名旧文件，避免旧图冒充本轮状态（§8）。规则 4–7 是纯像素/几何逻辑，在 `validity.rs` 单元测试覆盖；“内容与文件名所称状态一致”不由命令判断。

### 5.6 派生证据

| 证据 | 声明方式 | 输出 | 判定要点 |
| --- | --- | --- | --- |
| 序列帧 | 状态 `frames`；初始状态可 `from: entry` | `<名去扩展>.frames.png` | `Page.startScreencast`（jpeg，质量 90，尺寸为视口 × 像素比）收帧并逐帧 ack；开始录帧后先等第一帧（基准帧，最多等 1000ms，本计划定的上限）到达，再派发动作或开始导航；窗口 = 动作派发（或导航开始）到有限动画全部结束后 100ms，最短 300ms、最长 3000ms；在窗口内等距取 n 个时刻，每个时刻取时间戳不晚于它的最新帧，绝不取未来帧；没有更早帧的时刻（基准帧未在等待内到达）该格标“无帧”，报告 `derived.frames.missing` 记格数；网格最多 4 列，每格标 `+<ms> ms`；报告写 `note: "序列图只表示状态变化，不代表正常速度手感已核对"`；`entry` 的基准帧为导航前的空白页；窗口内未观察到动画时写 `未检测到出场动画` |
| 遮字版 | `--mask` | `<名>.mask.png` | 用 inspector 样式表（cdp.rs:1206-1225）对 `*, ::before, ::after` 设 `-webkit-text-fill-color: transparent`、`caret-color: transparent`、`text-shadow: none`，`::placeholder` 设 `color: transparent`，SVG `text, tspan` 的 fill 与 stroke 透明；**不改 `color`**，使用 currentColor 的图标、边框与装饰保持可见（同 shoot.mjs:987-989 的做法与注释）；截图类型与尺寸同主图 |
| 扫视缩略图 | `--thumbs` | `<名>.thumb.png`；同时有 `--mask` 时另出 `<名>.mask.thumb.png` | 取主图的首个视口区域；视口 CSS 宽 ≥600 缩到 480px 宽，否则 180px 宽 |
| 问题标注图 | 状态 `mark` | `<名>.mark.png` | 在主图上画 2px 框与编号牌；编号与报告 `derived.mark.items[]` 一一对应；某定位找不到时报告列入 `unmarked`；主图任一边小于 4 像素（画不下框）时记「标注图未产出」（派生问题，退出 2），主图不受影响 |
| 状态并排图 | `--sheet` | `sheet-<viewport>.png` | 该视口全部成功主图按初始在前、文件顺序排列，取各图首个视口区域，单格宽 480（视口 ≥600）或 240，格上标状态名 |
| 修改前后拼图 | `capture stitch` | `--out` 指定文件 | 第 i 张缩放比 = `match[0] / match[i]`；`--crop` 在缩放后坐标（即首图像素空间）中取，超出任一张缩放后图的范围即退出 1 且不写文件；未给 `--crop` 时取各图缩放后的公共左上区域；标签默认 `1,2,…`，默认 2 列 |

合成统一在同一浏览器中新开合成页（像素比 1），用 data URL 的 `<img>`、SVG 框与文字排版，等待图片解码后对容器截图。为让合成图保持单次截图，本计划把合成尺寸上限定为每边 16384px；超出时该派生图不产出，报告写明原因，退出码 2。派生图不计入、也不替代主截图（PRD §2.1）。

### 5.7 写拦截、凭据与隐私

- 写拦截（`--block-writes`）：页面会话 `Fetch.enable({patterns:[{urlPattern:"*", requestStage:"Request"}]})` 并 `Network.setBypassServiceWorker({bypass:true})`；拦截期间新附加的 iframe、worker 与 service worker 会话在 `Runtime.runIfWaitingForDebugger` 之前同样启用 `Fetch`（附加时已暂停，见 cdp.rs:735-790；`setBypassServiceWorker` 只覆盖页面自身加载，service worker 自己的写入经它自己的会话拦截）。暂停的请求：方法为 GET → 继续（需要时附 basic-auth 头）；pathname 命中 `--allow-write` → 继续并记 `allowed`；否则 `Fetch.failRequest(errorReason: "BlockedByClient")` 并记 `{state, viewport, method, url（origin + path，去掉查询串）, resourceType, at}`。被拦截的请求不再计入失败请求列表。
- 触发与后果：feature-demo 在共享实例上做录制前核对、review-ui 在真实产品上采集时，页面操作可能提交业务写入；后果是共享数据被改且不可撤回；现有能力只有 feature-demo 在 Playwright setup 中自写的 route（feature-demo SKILL.md:37），capture 自身没有。故按上面最小机制实现，并用服务端观察验证（V-2）。
- 凭据：步骤用 `env` 引用；URL 中的 basic-auth 用户信息按现有逻辑剥离。报告、终端输出与错误信息在写出前统一把每个 `env` 值与 basic-auth 密码的所有出现替换为 `***`（覆盖控制台消息、URL、观察文字）。截图像素中页面自己显示的内容不在脱敏范围。
- 资料目录：沿用 `Browser::launch` 的临时 profile 与 `Drop` 删除（§1.1）；不读取用户日常浏览器数据。进程被强杀时 profile 可能残留，与 detect 现状相同。
- 网络：只请求目标页面及其自身加载的资源；内置静态服务器只监听 127.0.0.1。

### 5.8 报告、终端输出与退出码

```json
{
  "schema": "phoenix-ui-capture-v1",
  "tool": { "name": "phoenix-ui capture", "version": "0.1.0", "binarySha256": "<当前可执行文件 SHA-256>" },
  "updatedAt": "2026-11-09T21:00:00+08:00",
  "shots": [{
    "file": "desktop-open-filter.png", "valid": true, "invalidReasons": [],
    "tool": { "name": "phoenix-ui capture", "version": "0.1.0", "binarySha256": "<采集该图的引擎>" },
    "target": "<调用方给的目标，已脱敏>", "url": "<实际打开的地址>",
    "state": "open-filter", "viewport": { "label": "desktop", "width": 1440, "height": 900 },
    "dpr": 1, "colorScheme": "light", "reducedMotion": false, "type": "viewport",
    "pixels": { "width": 1440, "height": 900 }, "capturedAt": "<ISO 8601 带时区>",
    "browser": { "product": "<Browser.getVersion product>" },
    "steps": [{ "index": 1, "do": "click", "target": "role=button[name=\"筛选\"]", "result": "ok", "observed": "…" }],
    "consoleErrors": ["…"], "failedRequests": [{ "method": "GET", "url": "…", "status": 404 }], "failedImages": ["…"],
    "horizontalOverflow": { "overflow": false, "scrollWidth": 1440, "viewportWidth": 1440 },
    "blockedWrites": [], "findings": [ /* detect Finding 对象 */ ],
    "derived": { "frames": { "file": "…", "count": 8, "missing": 0, "windowMs": 640, "note": "…" }, "mask": "…", "thumb": "…", "mark": { "file": "…", "items": [], "unmarked": [] } }
  }],
  "failures": [{ "state": "submit-empty", "viewport": "mobile", "file": "mobile-submit-empty.png", "step": 3, "do": "click", "reason": "…", "hints": ["{\"role\":\"button\",\"name\":\"保存\"}"] }],
  "sheets": [{ "viewport": "desktop", "file": "sheet-desktop.png" }]
}
```

- 其余字段：顶层 `outDir` 为报告所在目录到输出目录的相对路径；无效图条目带 `invalidFile`（实际写出的 `<名>.invalid.png`）；规则扫描出错时 `findingsError` 记原因、`findings` 为空数组；`allowedWrites` 记 `--allow-write` 放行的写请求；有警告的步骤带 `warnings`。浏览器自身探测的 `/favicon.ico` 不计入失败请求，同一失败请求只记一次。
- 合并：报告已存在且 schema 相同时，按 `file` 替换本次重取的条目，未重取的条目原样保留（含原 `capturedAt` 与该条目自己的 `tool`）；每张条目带采集它的 `tool`，顶层 `tool` 记最后写入报告的引擎，沿用图不被部分重取改写出处；`failures` 按 `(state, viewport)` 替换，本次成功即移除；`updatedAt` 为本次写入时间。
- 终端（非 `--json`）：每张图一行 `<state> <viewport> <file> findings=<n>`；无效图 `INVALID <state> <viewport> <file>.invalid.png: <原因>`；失败 `FAILED <state> <viewport> step <k> (<do>): <原因>`，下一行缩进给提示；末行 `capture: <n> shots, <k> invalid, <m> failed → <report>`。
- 退出码：0 全部状态完成且主截图全部有效；1 无法运行（参数或状态文件错误、浏览器不可用、目标无法打开、静态服务器无法启动），不写报告；2 已运行但有失败状态、无效图、派生图未产出或扫描出错，报告已写。

### 5.9 内置静态服务器

`serve.rs` 按 html_snapshot.rs:179-230 的线程与非阻塞 accept 写法：绑定 `127.0.0.1:0`；只接受 GET/HEAD，其他方法 405；`Host` 必须等于绑定地址；路径解码后拒绝 `..`、以 `.` 开头的段、NUL 与反斜杠；符号链接（文件或目录）canonical 解析后必须仍在根目录内，越出即 404；目录请求映射 `index.html`；按扩展名给 `Content-Type`（html、css、js/mjs、json、svg、png、jpg/jpeg、webp、gif、avif、ico、woff/woff2/ttf/otf、wasm、txt、xml、mp4、webm、map，其余 `application/octet-stream`），附 `Cache-Control: no-store`、`X-Content-Type-Options: nosniff`；不存在 404。HTML 文件目标的入口是该文件名，目录目标的入口是 `index.html`（不存在则退出 1）。

### 5.10 CDP 增量与 detect 抽取

- `Page` 新增：`set_device_metrics(w, h, dpr)`（`mobile:false`、关触摸）；`set_media(color_scheme, reduced_motion)`（一次设置两个特性）；`mouse(type, x, y, click_count)`；`key(&KeyStroke)`（`KeyStroke::parse` 同时供状态文件校验）；`insert_text`；`query_ax(role, name) -> Vec<(backend_node_id, name)>`；`call_with_nodes(function, args, nodes)`（角色定位的匹配元素交给页内定位脚本，滚入视口、取盒、遮挡与禁用判断都在 `page_js::LOCATE` 中完成，不另设 `scroll_into_view`/`box_of`）；`idle(duration)`（等待期间继续处理事件与录帧确认）；`watch_navigation`/`settle_navigation`（动作前记主框架 loader，动作后等加载停止）；`Browser::new_isolated_page()`（§5.3 第 2 步）；`layout_metrics()`；`screenshot(clip?, beyond_viewport)`（clip scale 1，输出为设备像素）；`start_screencast/stop_screencast -> Vec<(ts, jpeg_base64)>`；`console_errors()`（`Runtime.exceptionThrown`、`Runtime.consoleAPICalled` 的 error/assert、`Log.entryAdded` 的 error 级且 source 非 network，去重，单条 ≤500 字，最多 50 条）；`begin_network_log()` / `network_log()`（`network_log.rs`：requestWillBeSent 的方法与 URL、responseReceived 的状态、loadingFailed 的 errorText/canceled/blockedReason，最多 500 条）；`enable_write_block(allow)` / `blocked_writes()`。
- `Connection`：把单一 `auth: Option<OriginAuth>` 改为按会话登记的拦截器（auth 与写拦截可同时存在，子会话指向所属页面）；`maybe_handle_request_paused` 按会话查找后分别处理；`Page::close` 移除本页及子会话的登记（同 cdp.rs:1424-1434 的清理位置）。
- `lib.rs`：`scan_page_inner` 在 `goto`/settle 之后的部分移入 `pub fn scan_loaded_page`，`detect_url_impl` 改为调用它并保持结果映射（lib.rs:300-318）不变；capture 的初始状态扫描页按 detect 单目标的 `networkidle0`、不额外等待的时序调用它（lib.rs:87；D6）；`reveal_sweep` 改为 `pub fn`，`crates/cli` 的整页图经 `impeccable_browser::reveal_sweep` 复用同一实现。
- `detect`：新增 `pub fn url_scan_options(cwd: &str, home: &str, local_path: Option<&str>, rule_pack) -> (ScanOptions, DetectionConfig)`，取与 `detect_cli` 默认参数相同的 `inline_ignores`、`design_system`（仅 `local_path` 时，经 `load_design_system_for_target`，需 `home`）、`rule_pack`（`phoenix_ui_rules::PACK` 由调用方传入）与 `read_detection_config(cwd)`；`file_url_to_local_path` 改为 `pub` 供 `file://` 目标查找 DESIGN.md。`impeccable_browser::BrowserEngine::launch` 改为 `pub`，capture 与 detect 用同一套浏览器发现与沙箱参数。
- 发行版 `panic = "abort"`（Cargo.toml `[profile.release]`）：外部输入（状态文件、CDP 返回、图片解码）一律走 `Result`，不 `unwrap`。

## 6. Skill 与文档增量

所有 skill 改动先核对 §0.6 对应行。Phoenix skill 只改 `skills/phoenix-ui/src/`（英文行文，与现文一致）；交付给用户的话术随用户语言，中文会话中“未取证”“未完成 UX 走查”按原词出现。

### 6.1 Phoenix 完工取证（new-work.md §7、finish-reviewer）

1. **批量检查轮**（:146、:148、:150 段）：Web 上这一轮就是一次 capture 运行：`{{scripts_path}}/phoenix-ui capture <artifact URL 或路径> --viewport desktop,mobile[,user-<w>=<w>x<h>] --type full --states .phoenix-ui/review/states.json --out .phoenix-ui/review`。`states.json` 声明方向合同的 signature interaction 与主要流程产生的状态（展开/收起、错误、成功等），每组从新页面开始；signature interaction 的状态加 `"frames": 8`；Persuade surface 加 `--mask --thumbs`，用遮字缩略图做“几秒内看懂”的检查（现有 :146 句）。界面为 app-shell/100vh 工作台、整页拼接失真时，该状态改 `"type": "viewport"`。文件名沿用 `desktop.png`、`mobile.png`、`user-<width>.png`（默认模板即此），状态图为 `<viewport>-<state>.png`。
2. **有效性**（`rule:skill-capture-validity` 段）：命令已做 §5.5 自检；报告标为无效的图、失败的状态一律不交评审，按原因修正（`--settle-ms`/`--reduced-motion`、修步骤、改截图类型）后用 `--only` 只重取它们；仍须逐个开图确认内容就是文件名所称状态。
3. **页面检测**（:150 段）：capture 每张图的 `findings` 就是本轮页面检测，不再另跑 `detect <url>`；无 hook 宿主对改动源文件的 `detect --json` 保持现状。
4. **补位与未取证**：capture 退出 1（浏览器不可用、启动器不可用）时，用宿主浏览器按原有效性规则截同名文件，并在评审输入与交付中注明“宿主截图”；宿主也无法渲染时不派评审，交付写“未取证”及原因，不报告评审通过或 ship（WORK-F3/AC-5）。宿主浏览器仍可用于边看边改。
5. **评审输入**（:152 段）：输入包加 `.phoenix-ui/review/capture-report.json` 路径，并逐个列出交评审的文件及其截图类型；派生图（序列图、遮字版、缩略图、标注图、并排图）标明“派生证据，不替代状态截图”。
6. **修复轮**（:154 `fix` 段）：只用 `--only` 重取本批修复影响到的状态，覆盖同一路径；其余沿用，并在 verdict 输入中逐个写明沿用的文件（WORK-F3/AC-3）。
7. **盲测走查**（新段，紧接派出评审）：有主要操作流程（填表提交、多步流程、会保存数据的操作）的 Web 界面，在派出评审的同一轮，以全新上下文（不 fork）派出 `phoenix-ui-ux-walker`（Codex `phoenix_ui_ux_walker`；Cursor `/phoenix-ui-ux-walker`；Copilot 说 “Use the phoenix-ui-ux-walker agent”）。交接只含：页面入口、主视口、测试凭据（环境变量名）或测试文件、安全边界（只用测试账号；不对真实数据写入或破坏；交接不含真实业务账号）、输出目录 `.phoenix-ui/review/walk/`。不给：原始请求、用户案例、预期路径、源码路径、设计说明、方向合同、历史问题。纯视觉微调、方向首屏、无主要操作流程的界面不派，交付中不出现走查结论（WORK-F4/AC-7）。宿主不能派隔离子代理、或走查者报告无法操作页面时，交付写“未完成 UX 走查”及原因；主代理自己走不算独立走查（AC-5）。
8. **处理走查结果**：把每条问题分为可复现的产品问题、工具或数据问题、待复现推测；可复现的卡住/走错并入评审的修复批次，修后主代理在真实页面走同一路径（可直接用走查者留下的状态文件跑 capture，或用宿主浏览器）并记录结果，在此之前不宣称完工；犹豫作为建议。不强制再派走查。用户明确选择带问题交付时列出未修问题（AC-3）。
9. **交付报告**：原样给出评审处置词；单列走查：完成情况、问题、分类与处置、隔离标识（子代理 ID、会话 ID 或运行目录）与证据位置（AC-4、AC-6）。走查结果不改变处置词，四个处置词及含义不变（WORK-N3/AC-2）。
10. **finish-reviewer**：Input Contract 增加“取证报告 `capture-report.json`（父代理用 capture 时提供）：每个文件的截图类型、视口、像素比与有效性；派生图只是派生证据”。检查 0 中 “dimensions that make sense for the named viewport” 改为按报告的截图类型判定：视口图 = 视口 × 像素比；整页图 = 视口宽 × 像素比、从文档顶部开始、可以高于视口；元素图 = 该元素盒；报告标为无效的文件按畸形处理。处置词段、输出合同与 Verdict Pass 不改。

### 6.2 走查子代理 `phoenix-ui-ux-walker`（新文件）

- frontmatter：`name: phoenix-ui-ux-walker`；`codex-name: phoenix_ui_ux_walker`；`description` 写“以不知道任务的首次使用者身份走一条主要路径，报告卡住、走错与犹豫”；`tools: Read, Write, Bash, Glob`（Write 只用于输出目录，见正文第 1 条；D11）；`model: inherit`；`effort: high`；`max-turns: 40`；`nickname-candidates` 三个。
- 正文要点（改编自 ux-reviewer.md 与 ux-review.md，见 §0.6 I-3）：
  1. 输入只有入口、主视口、测试凭据环境变量名或测试文件、安全边界与输出目录；不读源码、开发者工具、原始任务、设计说明与项目文档，不猜测制作意图；只在输出目录写状态文件与取证输出，不编辑项目源码和其他任何文件。
  2. 操作方式：在输出目录写状态文件，运行 `{{scripts_path}}/phoenix-ui capture <入口> --viewport <主视口> --states <文件> --out <输出目录> --name 'step-{state}-{viewport}.png'`，读截图与报告里的步骤结果后再追加下一步；每次运行都从新页面重放当前累积路径（登录等前置步骤每次照做）。登录与安全边界内可重复的测试操作（测试账号登录、对测试数据的保存或提交）可随路径重放；会产生不能安全重复的副作用的步骤，只作为最后一次运行的最后一步执行一次，此后须经它才能到达的路径列为未验证。只用测试凭据，不做破坏性操作。capture 不可用、入口打不开或只能看到静态材料时，返回“未完成 UX 走查”及原因，不改报静态走查。
  3. 先看页面，用自己的话写出这是什么产品、谁会用、最可能来做什么；从可见内容选一条最重要的路径走到可观察结果。每步记“看到什么 → 为何这样操作 → 实际发生什么”，标出卡住、走错、犹豫，并记录操作后能否看出正在等待、已完成或失败，反馈是否在动作附近。
  4. 报告（用交接的语言）：产品理解、所选任务、完成 / 部分完成 / 未完成与实际路径（引用截图文件与状态文件）、最多 3 条关键问题（各写观察事实、原因推断、建议达到的结果；分不清事实与工具限制时标“待复现”）、未验证项（声音、触感、正常速度手感）。不打审美分，不代表真实用户，不给成功率、满意度或耗时结论。
- render.mjs:121 的计数改为 5；生成物中出现 `.claude/agents/phoenix-ui-ux-walker.md`、`subagents/codex/phoenix_ui_ux_walker.toml`、`reference/degraded/ux-walker.md`（V-5 核对）。
- evals/scenarios.md 新增 15–18：①code-led 带表单页面完工：用 capture 取证、评审与走查同轮、交付四项齐全；②宿主与 Phoenix 都无浏览器：交付“未取证”，不出现 ship；③走查报告可复现卡住：修复后主代理复验才完工，用户选择带问题交付时列出未修问题；④无主要操作流程的页面：不派走查、无走查结论。每条写失败信号。

### 6.3 feature-demo（`skills/feature-demo/SKILL.md`）

在 `@write_demo` 与 `@recording` 之间新增规则 `@precheck`，并在 `@failed_step`、`@report` 各补一句：

- 检测（D14）：在宿主技能目录（如 `.claude/skills/phoenix-ui/`、`.agents/skills/phoenix-ui/` 或用户级同名目录）找到 `scripts/phoenix-ui` 且 `<它> capture --help` 退出 0 才算可用；找不到、退出 4（engine 缺失或版本不符）或未知命令都按不可用处理，不安装、不下载、不提示安装（WORK-F15/AC-4）。应用在 iframe 内或步骤目标在 Shadow DOM 内时该功能不适用，沿用现有流程。
- 可用时，录制前对每个待录功能把 demo 步骤写成 capture 状态文件（角色与可见名称定位与 demo 一致），对运行中的系统执行 `capture <入口+path> --viewport <demo 视口> --states <文件> --block-writes --allow-write <已核实必要路径> --out <批次>/precheck/<slug>/`，查看关键状态截图、控制台错误与失败请求（AC-1）。
- 报告有拦截记录时按 `@environment` 的写入规则处理（缺授权先问或只录不产生副作用的部分），不算核对通过（AC-2）；核对失败按报告的失败步骤与原因归为产品缺陷、脚本漂移或环境问题（AC-3）。
- 在 report.json 的 `meta.environment` 写明“录制前核对用了 phoenix-ui capture <版本>”；预检截图不替代视频、不算验收；`precheck/` 不进交付清单；录制、视频检查、写入规则与报告模板不变；复制到演示目录的脚本仍不依赖任何 skill（AC-5；SKILL.md:22）。

### 6.4 review-ui

- **采集**（SKILL.md 工作流 :64 一步与 `target-and-capture.md` :95-113）：检测到可用 capture（D14）且单元适用时，用 `capture <实际地址> --viewport <宽>x<高> --dpr <像素比> --color-scheme <主题> --states <单元状态文件> --name '<命名模板>' --out <前缀>.ui-captures/r<n>/ --report <前缀>.ui-captures/r<n>/logs/capture-report.json` 采集。命名模板由 review-ui 按 `target-and-capture.md` 命名合同（:106-112）生成后传入，不扩展 CLI：首采为 `<单元>-{state}-{width}-{scheme}.png`；同单元另有同宽但高度或 DPR 不同的视口时加确定性后缀（如 `<单元>-{state}-{width}-{scheme}-h{height}-dpr{dpr}.png`）；同轮补采追加 `-a2`、`-a3` 等 attempt 后缀，原图及报告中的原条目保留不覆盖。`capture.md` 的地址、视口宽高、DPR、主题、拍摄时间、所用工具（`phoenix-ui capture <该图条目 tool.version>`，附该条目 `binarySha256` 前 12 位；旧报告无逐张 `tool` 时回落顶层）从报告抄录；报告标无效的图按「没采到」处理（重拍或记缺证），不交评审（WORK-F16/AC-1、AC-2）。控制台错误、失败请求、横向溢出与 `findings` 只进机器检查（报告 §9），不进评审材料（AC-3）。主 agent 仍逐张开图核对（AC-4）。到达目标状态所需交互或前置超出命令能力（iframe、Shadow DOM 等）时，该单元改用现有其他采集方式并满足原真实性与出处要求，`capture.md` 记实际工具；只有现有方式也取不到真实状态才记缺证（AC-6）。未检测到时沿用现有方式（AC-5）。
- **走查**（SKILL.md 工作流在采集之后、独立评审之外新增一步；`@independent_judge` :210-212 与 `judge.md`:12 的禁给材料加“走查材料”）：检测到走查子代理（D14）时，对有主要操作流程的单元派出 `phoenix-ui-ux-walker`，交接只含入口、视口、测试凭据环境变量名与安全边界、输出目录 `<前缀>.ui-captures/r<n>/walk-<单元>/`；不给目标包、期望、计划、代码、修复记录与历史报告（WORK-F17/AC-1）。主 agent 把返回的报告存为 `<前缀>.ui-captures/r<n>/walk-<单元>.md`（随 Git 跟踪，截图按现有忽略规则不入库），格式同 WORK-F4/AC-2（AC-2）。
- **差距**（`report-contract.md`）：:78 来源列加「盲测走查」；:94 门槛项列表加「盲测走查」；:57 复查分工写明「盲测走查」差距由主 agent 复查；:37 `judge.md` 不写走查差距；§10「采集与覆盖」记走查状态、隔离标识与报告链接。可复现的卡住/走错由主 agent 按 `judge.md`:59-63 的严重度定义定级列入差距，犹豫列为建议（AC-3）。走查者无法操作时报告写“未完成 UX 走查”及原因；未安装或单元无主要操作流程时不派、报告与原来一致；计分与判定规则不变（AC-5）。
- **复述**：`evals/scenarios.md`、`evals/fresh-context.md` 中“Phoenix 不在场”行保持，新增“已装 Phoenix：适用单元用 capture、iframe 单元回到现有方式；派出走查、差距来源与评审隔离”两行；`agents/openai.yaml` 不含相关描述，不改。

### 6.5 apply-ui-review

- `SKILL.md` §2（:42-50）新增一条：来源为「盲测走查」的差距不是评审看图得出的，先在真实页面按走查报告的状态文件或路径复走一遍确认，修复后再复走同一路径并记录结果；处置最多登记「已修改待评审」，是否到位由下一轮 review-ui 判定（WORK-F17/AC-4）。§4（:67）自检要求同步提到复走记录。
- `references/record-contract.md`:55：「盲测走查」差距的“核实”“自检”列写复走的步骤与结果。
- `evals/scenarios.md` 增加一行对应场景；`SKILL.md:10` 的“评审看过目标和截图”补一句说明走查来源例外。

### 6.6 文档与复述

- `docs/phoenix-ui.md`：在「XGENT 自有检测规则」之后新增「取证命令（capture）」一节：用途与 D7 边界、命令与主要参数、状态文件示例、输出命名与报告、有效性与退出码、浏览器要求、网络与隐私边界（临时资料目录、只访问目标及其资源、写拦截、`env` 凭据）、与 `capture-server`/`component-review capture` 的区别、随 Phoenix 安装的走查子代理。:70 末句与 :120 改为“这些 skill 不依赖 Phoenix；检测到 Phoenix 时 review-ui 可借用取证命令与盲测走查”。
- `README.md`：review-ui（:85）、apply-ui-review（:86）、feature-demo（:174）三行各补一句可选接入；UI 评审循环（:100）补“装有 Phoenix 时可加盲测走查”。
- `skills/agi-mode/` 下的 sdlc/what-next/evals 只引用 review-ui/apply-ui-review 的阶段与下一步命令，名称与命令不变，不改（V-5 grep 复核）。

### 6.7 第三方声明

- `tools/phoenix-ui/NOTICE.md`：按 ehmo 段落格式新增 oil-ui 段：Original work oil-ui 0.33.2（本机副本，上游地址未从副本核实）、License MIT、Author oil-oil；写明改编范围：`crates/cli/src/capture/` 的步骤模型、文字定位与失败提示、settle、mask/mark/sheet/stitch/frames 方法，`skills/phoenix-ui/src/subagents/phoenix-ui-ux-walker.md` 的交接与报告格式。
- `tools/phoenix-ui/THIRD-PARTY-NOTICES.md`：新增 oil-ui 条目，附 `local/oil-ui-main/LICENSE` 的 MIT 全文（含 “Copyright (c) 2026 oil-oil”）。随后 `check-phoenix-deps.mjs --write` 重算清单；构建把 NOTICE 与 THIRD-PARTY 三件套复制进 `dist/phoenix-ui`（build-phoenix-ui.mjs:107-112）。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N1 | PRD WORK-N1/AC-1–AC-3 | 临时 profile 用后删除；凭据不入报告/日志/输出；走查只用测试凭据 | 进程被强杀时 profile 可能残留（与 detect 相同） | §5.7；§6.1 第 7 条、§6.2 | V-2（profile、脱敏）、V-1、V-6、V-9 交接核对 |
| WORK-N2 | PRD WORK-N2/AC-1–AC-2；docs/phoenix-ui.md:44-53 | 四平台无需 Rust；缺 engine 给补装命令；无浏览器明确报错，其余命令照常；Node ≥18、无 Python | 浏览器不可用退出 1；engine 缺失退出 4 | D1、§5.8；启动器零改动 | V-1、V-7（冒烟）、V-8（四平台、Node18 消费者） |
| WORK-N3 | PRD WORK-N3/AC-1–AC-2 | 既有流程与四处置词不变 | — | D6 抽取等价；§6.1 只改 §7 与评审输入 | V-2（xgent_rules）、V-7（全量与 oracle）、V-10 |
| WORK-N4 | PRD WORK-N4/AC-1–AC-2 | 声明含 oil-ui MIT 全文与版权行；§0.6 先于 skill 改动 | — | §0.6、§6.7 | V-7、V-10 |

安全项：写拦截（§5.7）；凭据脱敏（§5.7）；静态服务器只绑回环、拒绝路径穿越与点文件（§5.9）；走查交接不含真实账号（§6.1、§6.2）。可观测性：命令的全部事实进报告文件与退出码，不新增日志通道。

## 8. 失败模式、发布与回滚

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现/降级 | 检测 | 恢复/补偿 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| 写请求经 iframe、worker、Service Worker 或表单导航绕过拦截（feature-demo/review-ui 在共享实例上取证） | 被取证的共享实例 | 业务数据被写入，不可撤回 | 无提示的副作用 | 测试服务器侧记录收到的非 GET；报告 `blockedWrites` | 页面与子会话 `Fetch` 拦截 + 绕过 SW（§5.7） | V-2 写通道用例 |
| 失败或无效时旧图仍在原路径（修复轮同路径重取） | 单次评审 | 评审依据旧状态给出 ship | 错误放行 | 报告 `failures`/`invalid`；原路径文件不存在 | 删除同名旧图，无效图另名（§5.5） | V-2 |
| 凭据经控制台、URL 或观察文字进入报告/输出 | 报告与终端日志的读者 | 测试账号泄露 | 无 | 对报告、stdout、stderr 搜索密钥值 | 统一脱敏（§5.7） | V-2 |
| 抽取 `scan_loaded_page` 改变 detect URL 扫描结果 | 所有 `detect <url>` 用户 | 无 | 发现集合变化 | xgent_rules 与 oracle 逐字节对比 | 恢复等价抽取 | V-2、V-7 |

- 发布顺序：本期只到开发候选（本机开发构建 + 四平台 CI 制品），不发 npm/R2（§0.5）。
- 放量/切换门：无；新命令只在调用时生效，既有流程不调用它。
- 回滚：`git revert` 对应提交；生成物与 GENERATED.json 随源码一起回退后重跑开发构建。

## 9. 验证

### 9.1 检查表

共同环境：仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；Cargo 命令在 `tools/phoenix-ui` 下运行；需可发现的 Chrome（可用 `PHOENIX_UI_BROWSER` 指定）；浏览器测试不放进 macOS `sandbox-exec`。批内最小反馈：`mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` 后直接运行 `target/debug/phoenix-ui capture …` 复现单个问题；纯逻辑用 `mise exec --no-deps -- cargo test -p phoenix-ui --lib capture::<模块> --locked --offline`。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **主代理为一个带登录、筛选面板与新建表单的 code-led 页面取完工证据。** 角色：只读过 `phoenix-ui capture --help` 与夹具页面，不读 capture 源码。输入：夹具应用 `tests/fixtures/capture/app/`（登录页 → 任务列表；“筛选”打开面板；空表单保存出错误提示；保存走 POST `/api/tasks`；首屏 600ms 淡入；一张图片缺失；手机宽有一处横向溢出；`console.error` 一次）与 `states.json`（open-filter、submit-empty、save-task，都含 `env` 密码登录）。结果：①`desktop,mobile` × 初始 + 3 状态共 8 张整页图，全部有效，文件名为 `desktop.png`、`mobile-open-filter.png` 等；报告逐张有目标、视口、像素比、配色、类型、拍摄时间、命令版本、步骤与观察、控制台错误、失败请求与图片、横向溢出、findings（WORK-F1/AC-1、AC-2、AC-5、AC-9）；②save-task 的 POST 被拦截并记录，其余放行（AC-8）；③输出目录与终端搜不到密码（WORK-N1/AC-2）；④把 open-filter 的定位改成 `{"role":"button","name":"筛选器"}` 重跑：该状态 FAILED 并给出 `{"role":"button","name":"筛选"}` 写法，其余照常，退出 2，上一轮的 `desktop-open-filter.png` 已删除（AC-3、AC-7）；⑤以 `--viewport 1280x800 --dpr 2 --color-scheme dark --name 'U1-{state}-{width}-{scheme}.png' --out <另一目录>` 采集，文件名与 2560×1600 像素正确，报告同样有有效性标记（WORK-F3/AC-7）；⑥`PHOENIX_UI_BROWSER=/nonexistent` 运行：退出 1、提示浏览器不可用、未建输出目录（WORK-F1/AC-6、WORK-N2/AC-2）；⑦同一应用由 `node tests/fixtures/capture/serve.mjs` 以 http 提供时结果同①。Agent 逐张打开图片确认内容与文件名所称状态一致 | `tools/phoenix-ui` 下构建 debug 二进制；`DEMO_PASSWORD=<测试值> target/debug/phoenix-ui capture tests/fixtures/capture/app --type full --states tests/fixtures/capture/app/states.json --out "$tmp/review" --block-writes`；其余变体按结果描述调整参数 | M1–M2 交付齐备后（批 1）；M4、M5 的前置 |
| V-2 | 局部 | 准入：AC-4 一致性为明示的逐项等价要求；写拦截、凭据、资料目录为高后果安全路径且 E2E 难逐一触发；有效性与失败语义为明示规则，错判会让错误证据进评审；解析、像素、合并为分支多的纯逻辑。内容：①初始状态 `findings` 与 `detect --json --viewport <WxH>` 相同：静止的 http 目标；有入场动画的 http 目标（夹具让 detect 的扫描时点稳定落在动画进行中，如带延迟的长淡入，避免时序抖动使两次运行不可比）；以及本地 `tests/fixtures/xgent/controls.html` 对比 `detect file://…`，含 `file` 字段（WORK-F1/AC-4）；②写通道页（fetch POST、表单 POST、sendBeacon、XHR PUT、跨源 iframe POST、dedicated worker POST、service worker 安装期 POST、允许路径 `/api/allowed`）：测试服务器只收到允许路径的非 GET，报告逐条列出其余（AC-8、§8）；③密钥经 console、URL 查询、页面文字回显后，报告/stdout/stderr 无明文；④运行后系统临时目录无该子进程的 `phoenix-ui-chrome-profile-<pid>-*`（WORK-N1/AC-1）；⑤有效性夹具：6s 淡入 + `--settle-ms 300` 判无效并写 `.invalid.png`、删同名旧图；纯白页判单色；未绘制 canvas 判画布未绘出；高页整页图有效且高于视口；dpr 2 元素图尺寸 = 盒 × 2；窄于 4 像素的元素图标注记「标注图未产出」（派生问题，退出 2），主图不受影响（WORK-F3/AC-2、WORK-F1/AC-5）；⑥失败语义：隐藏目标失败带原因、被遮挡与禁用目标记警告并执行、找不到给候选、`waitFor hidden` 超时、目标只在 iframe 中时提示 iframe 数（AC-3、AC-7）；⑦单元：状态文件校验、命名模板冲突、像素规则 4–7、报告合并与脱敏；⑧回归：detect URL 路径与 CDP 共用层 | `mise exec --no-deps -- cargo test -p phoenix-ui --test capture --locked --offline -- --test-threads=1`；`mise exec --no-deps -- cargo test -p phoenix-ui --lib capture --locked --offline`；`mise exec --no-deps -- cargo test -p phoenix-ui --test xgent_rules --locked --offline`；`mise exec --no-deps -- cargo test -p impeccable-browser --locked --offline -- --test-threads=1`（cdp.rs 为 detect 与 comp 门禁共用层，改动后定向回归）；仓库根 `node scripts/check-phoenix-lints.mjs` | M1–M2 交付齐备后（批 1） |
| V-3 | 场景 E2E | **评审者要从证据看出面板如何展开、首屏块面分布，以及修复前后差别。** 用 V-1 夹具：open-filter 加 `"frames": 8`、`mark` 两项（一项不存在）；`initial` 加 `{"frames":{"count":8,"from":"entry"}}`；`--mask --thumbs --sheet`；另取一个无动画静态页的 entry 帧；再对修复前后两张 `desktop-open-filter.png`（第二轮把面板宽度改窄）运行 `capture stitch`，以及一次越界 `--crop`。结果：序列图 8 格按时间递增、标注时间、能看出面板展开，报告带“不代表正常速度手感”说明（WORK-F2/AC-1）；entry 首帧为空白、能看到出场（AC-2）；静态页报告“未检测到出场动画”；遮字版文字全不可见、图片与图形可见，缩略图 480/180 宽（AC-3）；标注图 #1 框在“应用筛选”，#2 列为未标注（AC-4）；并排图按状态排列带名称；拼图按共同元素宽度统一比例、同区域、带标签，越界时退出 1 且无文件（AC-5）。Agent 逐张打开查看 | `target/debug/phoenix-ui capture … --mask --thumbs --sheet`；`target/debug/phoenix-ui capture stitch a.png b.png --match 320,320 --crop 0,0,1440,600 --label 修改前,修改后 --out "$tmp/stitch.png"` | M3 交付齐备后（批 2）；M4 的前置 |
| V-4 | 局部 | 准入：帧采样与拼图几何为分支多的纯逻辑。内容：等距采样、不取未来帧、早于首个可用帧的时刻标“无帧”并计入 `missing`（不用后来的帧补位）；缩放比、裁切换算、越界判定、缩略图宽度选择 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib capture::derived --locked --offline`；仓库根 `node scripts/check-phoenix-lints.mjs` | M3 交付齐备后（批 2） |
| V-5 | 局部（静态） | C-1、C-2、C-6、WORK-F3–F4、F15–F17 文本落位：render 预览通过（子代理数 5、无占位残留），预览中有走查子代理的 Claude/Codex/degraded 三种产物，Cursor 产物不含 `readonly: true`，§7 含 capture 命令行、走查派发与“未取证”“未完成 UX 走查”语义，评审输入含取证报告；三个独立 skill 中每处提到 Phoenix 的地方都是“检测到才用”；「盲测走查」出现在 §6.4、§6.5 列出的全部位置；改动未触及 live.md、critique.md、component-review.md、visualize.md、build_phase.rs；四个改动 skill（phoenix-ui、feature-demo、review-ui、apply-ui-review）的元数据通过，其他 skill 的既有失败只报告 | `tools/phoenix-ui` 下 `mise exec --no-deps -- bun scripts/render.mjs --without-engine --output-root ../../local/phoenix-ui/render-preview`，在预览中检索上述文件与关键词；仓库根 `grep -rn '盲测走查' skills/review-ui skills/apply-ui-review README.md`；`grep -rni 'phoenix' skills/feature-demo skills/review-ui skills/apply-ui-review` 逐条看条件；`git diff --stat` 核对范围；`python3 -m unittest discover -s test/skills -p 'test_skill_metadata.py'`，只按上述四个 skill 的子测试判定 | M4–M5 交付齐备后（批 3） |
| V-6 | 场景 E2E（有界演练） | **一个不知任务的走查者走一遍提交后没有反馈的页面。** 角色：Claude Code 新会话中的主 agent 只按交接派出走查者。夹具：V-1 应用变体 `app-nofeedback/`（保存成功但界面无任何变化），由 `serve.mjs` 以 http 提供；演练项目放入 V-5 预览中 claude-code 产物的 `.claude/`，`skills/phoenix-ui/scripts/phoenix-ui` 换成调用本地 debug 二进制的演练垫片（仅演练，收口用开发构建正式安装）。结果：走查者的工具调用只有 capture 运行与读取其输出（无读取夹具源码）；报告有产品理解、所选任务、完成情况与路径、≤3 条问题（含保存后无反馈的卡住或犹豫，分开写事实/推断/建议），无审美分与真实用户结论（WORK-F4/AC-2、WORK-N1/AC-3）。变体：入口不可达时返回“未完成 UX 走查”及原因 | `claude -p "<只含入口、主视口、凭据变量名、安全边界与输出目录的派发请求>" --output-format stream-json --verbose`，权限参数按 `claude --help` 实际选择并只授予本场景所需；保存输出并按工具调用与最终回复判定；没有生效的拒绝调用记未运行 | M4–M5 交付齐备后（批 3） |
| V-7 | 阶段收口 | 构建、许可与 Phoenix UI 全量回归：NOTICE/THIRD-PARTY 含 oil-ui MIT 全文与版权行并进入 `dist/phoenix-ui`（WORK-N4/AC-1）；oracle 新增 capture 离线用例（`--help`、未知参数、坏视口、状态文件错误、浏览器不可用）并重录 `cli-help`，其余 golden 逐字节不变（WORK-N3/AC-1）；开发构建与冒烟通过，冒烟新增：经启动器 `capture --help` 退出 0；对冒烟夹具 `capture --viewport 390x844` 得 390×844 有效图，或在无浏览器时退出 1 且提示不可用（记录是哪一种）；空 `PHOENIX_UI_HOME` 下 `capture --help` 退出 4 并给补装命令（WORK-N2/AC-1–AC-2）；`test/phoenix-ui` 下全部 Node 测试（启动器、构建、许可审计、安装、发布等）与 `test:phoenix`（含共享浏览器层、全部 UI 原生测试与 oracle，WORK-N3）全绿；render `--check` 为 fresh。不以整仓 `npm test` 为门：其 Diagram、statusline、dev-plan、agi-mode 等检查与本期行为无直接关系，失败只报告、不扩围修复 | `tools/phoenix-ui` 下：`mise exec --no-deps -- node ../../scripts/prepare-phoenix-ui.mjs --prepare` 并按回执导出环境变量；`mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --write`；重建 debug 二进制后在仓库根运行 `node tools/phoenix-ui/tests/oracle/record.mjs --bin tools/phoenix-ui/target/debug/phoenix-ui capture` 与 `… cli-help`（逐个审阅 golden diff）；清理旧 `../../local/phoenix-ui/development-release` 后 `mise exec --no-deps -- node ../../scripts/build-phoenix-ui.mjs --development --output-dir ../../local/phoenix-ui/development-release`；`mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --distribution-dir ../../local/phoenix-ui/development-release`；`mise exec --no-deps -- node ../../scripts/smoke-phoenix-ui.mjs --release-dir ../../local/phoenix-ui/development-release --target darwin-arm64 --stage-runtime`；`mise exec --no-deps -- bun scripts/render.mjs --check`；仓库根 `mise exec --no-deps -- node --test 'test/phoenix-ui/*.test.js'`、`mise exec --no-deps -- npm run test:phoenix`（含 release 构建 lto=fat 与全量 oracle，耗时未知） | M6（阶段收口），代码与 skill 定稿后 |
| V-8 | 阶段收口 | 四平台候选：同一候选提交上 `UI Native darwin-arm64/darwin-x64/linux-x64/linux-arm64` 与 `UI Node 18 portable consumer smoke` 全部成功；四个平台的冒烟记录都是“已截图”（WORK-N2/AC-1）；capture 浏览器测试在各平台运行（A-1）。Diagram job 的失败与本期无关时只报告 | 经用户授权推送 main 后：`gh run list --workflow phoenix.yml --branch main -L 1`、`gh run watch <run-id>`，下载各平台 evidence 制品核对冒烟记录；记录 run ID、提交与制品摘要 | M6，V-7 通过并完成候选提交后；未获授权推送记未运行 |
| V-9 | 真实使用验收 | 干净夹具项目、只凭公开入口与文档、Claude Code 新会话（A6 用 Codex），逐项保存会话输出并亲自查看产物。**A1** 新页面完工（UAT-1）：请求“用 Phoenix 做一个带筛选和新建表单的任务列表页面”；判据：capture 取证 desktop/mobile 与声明状态、无效图未交评审、评审输入含取证报告与截图类型、设计自检截图不超过一轮批量检查加一轮确认且没有另跑 `detect <url>`、走查与评审同轮且交接只含允许材料、交付有原样处置词与单列走查及隔离标识（WORK-F3/AC-1、AC-6，WORK-F4/AC-1、AC-2、AC-4、AC-6，WORK-N1/AC-3）。**A2** 修复复验（UAT-5）：在 A1 产物上去掉保存反馈后请求“继续完工检查”；判据：走查的可复现问题进入修复批次、修后主代理复走并记录、修复轮只重取受影响状态并注明沿用（WORK-F3/AC-3、WORK-F4/AC-3）；走查未报出该问题时如实记录并据此修订走查子代理。**A3** 无浏览器（UAT-2、UAT-6）：`PHOENIX_UI_BROWSER=/nonexistent`、会话无浏览器工具；判据：交付写“未取证”与“未完成 UX 走查”及原因，无 ship 或“评审通过”（WORK-F3/AC-5、WORK-F4/AC-5）。**A4** feature-demo（UAT-20、UAT-23）：带写接口的夹具应用，请求“这次加了筛选，录个演示”；判据：录制前用 capture 开拦截核对、拦截记录按写入规则处理、`meta.environment` 写明、视频与报告模板不变（WORK-F15/AC-1–AC-3、AC-5）；A4b 同项目未装 Phoenix：流程与升级前一致、无安装提示（AC-4）。**A5** review-ui + apply-ui-review（UAT-21、UAT-22、UAT-23）：两个单元（一个含提交流程，一个在 iframe 内）；判据：适用单元用 capture 且 `capture.md` 出处齐全、iframe 单元用现有方式并记实际工具、评审材料无机器发现与走查内容、主 agent 开图记录在、走查报告在本轮目录、可复现卡住以「盲测走查」列差距；随后 `apply-ui-review <报告>` 复走同一路径并记录处置（WORK-F16/AC-1–AC-4、AC-6，WORK-F17/AC-1–AC-4，WORK-F4/AC-8）；A5b 未装 Phoenix：报告与升级前一致、无走查结论（WORK-F16/AC-5、WORK-F17/AC-5）。**A6**（Codex）无主要操作流程的信息页新建完工：capture 取证与评审正常、不派走查、交付无走查结论（WORK-F4/AC-7）。发现的用户可见问题修复或列给用户确认延期 | 夹具放在仓库树之外的临时目录（`local/` 下有其他项目的 `CLAUDE.md`，Claude Code 会向上加载，夹具就不是干净的用户状态），夹具源存 `local/phoenix-ui/v9-src/`；Phoenix 用 `node bin/xgent-skills.js install <夹具> --release-dir=local/phoenix-ui/development-release --no-phoenix-diagram --providers=claude`（A6 用 `--providers=codex`）安装 V-7 的开发候选；feature-demo/review-ui/apply-ui-review 以符号链接指向仓库 `skills/` 对应目录，先用最小会话确认能被加载，不行改复制并记录；A4b/A5b 不安装 Phoenix；`claude -p "<请求>" --output-format stream-json --verbose`、Codex 用 `codex exec` 的非交互模式，权限按 CLI 实际参数只授予场景所需；需已登录的 CLI；权限拒绝使调用未发生时该场景记未运行 | M6，V-7 通过后，用同一开发候选 |
| V-10 | 阶段收口 | 参考整合与接口复述（WORK-N4/AC-2、C-2、C-3、WORK-N3/AC-1–AC-2）：§0.6 每个采纳/调整项都有落点、拒绝项无改动；finish-reviewer 的 Disposition、Output Contract、Verdict Pass 段与 build_phase.rs 无 diff；§6.4–§6.6 列出的复述处全部更新；`skills/agi-mode/` 无需改动的判断复核 | `git diff 4c2d111 -- skills tools/phoenix-ui docs README.md scripts` 逐项对照 §0.6；`git diff 4c2d111 -- tools/phoenix-ui/crates/comp-verbs skills/phoenix-ui/src/reference/live.md skills/phoenix-ui/src/reference/critique.md skills/phoenix-ui/src/reference/component-review.md` 为空；`grep -rn 'review-ui\|apply-ui-review\|feature-demo' skills/agi-mode README.md docs/phoenix-ui.md` | M6，V-9 之后 |

### 9.2 集成与回归

- 批 1 开工前，在起始 SHA 跑一次 `cargo test -p phoenix-ui --test xgent_rules` 与 `-p impeccable-browser`，把既有失败分型写入 M1 记录，作为 V-2 第⑧项的对照。
- `test:phoenix` 依次执行 bundle 检查、component-review 检查、release 构建、安装测试、工作区单测（macOS 在 sandbox 内）、浏览器测试（sandbox 外）、lint 与 oracle（scripts/test-phoenix-ui.mjs:27-50）；它只在 V-7 运行一次。
- `skills/phoenix-ui/src` 改动会让 GENERATED.json 失配，M4 之后到 V-7 开发构建之前依赖生成物的检查（如 render `--check`）预期失败，不作为中间里程碑的检查。

### 9.3 真实环境

V-1、V-3 在本机 Chrome 上由 Agent 执行并亲自看图；V-6、V-9 需要已登录的 Claude Code（`claude` 2.1.319 已安装）与 Codex（`codex-cli` 0.166.0 已安装）；V-8 需要用户授权推送。缺任一环境时对应检查记“环境缺失/未运行”，收口不宣称相应 AC 通过。

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 取证主链路（初始状态、多视口、报告、页面规则发现） | 无 | 批 1 前半。`crates/cli/src/capture/` 的 mod.rs、serve.rs、report.rs、validity.rs（规则 4–7）★；main.rs 路由、lib.rs；cdp.rs 的 `set_device_metrics`、`set_media`、`console_errors`、`network_log`、`layout_metrics`、`screenshot`；`network_log.rs`★；lib.rs 抽出 `scan_loaded_page`；detect 的 `url_scan_options` 与 `ROOT_USAGE`；夹具与 `crates/cli/tests/capture.rs`★ 初版。同构：`crates/cli/src/component_capture.rs:58-130` 的页面生命周期、`html_snapshot.rs:179-230` 的服务器、`xgent_rules.rs` 的二进制测试与跳过写法。cdp.rs 与 lib.rs 由主 agent 单人修改 | 与 M2 同批，V-1、V-2；回写「实施进度」 |
| M2 | 状态组、步骤、截图类型、有效性、写拦截与凭据 | M1 代码 | 批 1 后半，连续实施。plan.rs、steps.rs、page_js.rs★；cdp.rs 的 `mouse`、`key`、`insert_text`、`scroll_into_view`、`box_of`、`query_ax`、按会话拦截器与 `enable_write_block`；validity 规则 1–3 与 7 的整页/元素分支；`--only`、无效图命名与旧图删除、报告合并与脱敏。同构：oil-ui shoot.mjs:1056-1257 的定位与提示（行为参考，代码重写） | 批末 V-1、V-2 通过；回写「实施进度」 |
| M3 | 派生证据 | V-1、V-2 通过 | 批 2。单独成批的原因：批 1 的 V-1、V-2 证明命令接口可用，是 M5 中 capture 接入并行开工的前置，且批 1 已含两个里程碑的引擎工作。derived.rs★；cdp.rs 的 `start_screencast/stop_screencast`；mask 复用 `create_capture_stylesheet`/`set_capture_stylesheet`（cdp.rs:1206-1225）；`capture stitch` 子命令。同构：shoot.mjs:1333-1429、:1557-1609 的合成与采样做法（行为参考）、本期 M1 的页面生命周期 | V-3、V-4；回写「实施进度」 |
| M4 | Phoenix 完工取证与走查子代理 | V-1、V-3 通过（命令接口与派生参数已定）；§0.6 I-1、I-3 已核对 | 批 3 前半。§6.1、§6.2：new-work.md、finish-reviewer.md、phoenix-ui-ux-walker.md★、evals/scenarios.md、render.mjs:121；docs/phoenix-ui.md 取证一节。同构：新子代理照 `src/subagents/phoenix-ui-finish-reviewer.md` 的 frontmatter 与“Input Contract / 输出合同”结构，§7 派发写法照 new-work.md:152 派出评审的一段。只渲染到预览目录，不改生成物 | 与 M5 同批，V-5、V-6；回写「实施进度」 |
| M5 | 独立 skill 可选接入 | capture 接入（§6.3、§6.4「采集」）：V-1、V-2 通过，可在 M3 进行时由 subagent 并行；走查接入（§6.4「走查」「差距」、§6.5 及走查相关复述）：M4 的走查子代理定义完成（同批内先完成）；§0.6 I-5–I-7 已核对 | 批 3 后半。§6.3–§6.5 与 README、docs/phoenix-ui.md 的复述行。同构：feature-demo `@charts`（SKILL.md:69-75）“已安装 phoenix-diagram 时才用、不可用照常完成”的可选接入写法；review-ui 新规则沿用该文件现有 ODC 规则格式。可写路径：skills/feature-demo、skills/review-ui、skills/apply-ui-review；README.md 与 docs/phoenix-ui.md 由主 agent 单人修改 | 批末 V-5、V-6 通过；回写「实施进度」 |
| M6 | 阶段收口：许可、构建、四平台候选与真实使用验收 | M1–M5 完成 | 依次：①§6.7 声明 → `--write`；②oracle capture 用例与 cli-help 重录；③smoke-phoenix-ui.mjs 增加 capture 冒烟；④开发构建同步生成物（GENERATED.json、dist/phoenix-ui）并跑 V-7；⑤候选提交，经授权推送跑 V-8；⑥用同一开发候选跑 V-9；⑦V-10。V-9 或 V-8 暴露的问题修复后只重跑失效项；改动 skill 源或引擎源码后须从④重建再复验受影响项 | V-7、V-8、V-9、V-10；回写「实施进度」 |

提交点：M1、M2、M3、M4、M5 各自交付后分别提交（同批的 M1/M2、M4/M5 不等批末检查）；M6 的④完成后提交构建同步与生成物，作为 V-8 的候选提交；全部里程碑完成后最终提交一次并回填 SHA。推送仅在用户授权时进行（C-9）。

并行：M1–M3 共用 cdp.rs，串行；M5 文件与 M3 不重叠，其 capture 接入在 V-1、V-2 通过后可交给 subagent 与 M3 并行，走查接入等 M4 的走查子代理定义完成后再做（可交给同一 subagent）；主 agent 负责 README.md、docs/phoenix-ui.md 与批 3 验收；V-9 各场景共用一个开发候选但夹具目录、端口与浏览器 profile 各自独立，可顺序或并行运行。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1 CDP 实验性方法在 CI 浏览器上不可用或行为不同 | 角色定位、序列帧或子会话拦截在某平台失效 | 实施者：V-2 本机先证；V-8 失败时按 A-1 的替代路线改页内实现并修订 §5.4/§5.6 | M2 批末、M6 | 否 |
| 走查质量（可能漏报或误报） | WORK-F4 价值 | V-6 演练后按实际输出修订走查子代理正文；A2 中未报出植入问题时如实记录并修订 | M5 批末、M6 | 否 |
| 推送授权与 CI runner 可用性 | V-8 不能执行，WORK-N2/AC-1 不能宣称通过 | 收口前向用户申请推送；未授权记未运行 | M6 | 否（影响收口结论，不影响实施） |
| 宿主 CLI 登录与权限参数（`claude -p`、`codex exec`） | V-6、V-9 无法执行 | 运行前用 `--help` 核对参数，最小会话确认 skill 与子代理能被加载；不行记未运行 | M5、M6 | 否 |
| 超长页面整页截图的像素高度与文档高度不符 | 该整页图无效 | 由 §5.5 第 7 条按实测尺寸判无效并提示改用视口或元素图；不预设浏览器上限数值 | — | 否 |

- 最终状态：**Ready**。
- 定级理由：范围、期次与可选接入方式由用户 2026-11-09 逐项确认（决定记录），PRD 评审 F-01–F-05 已在 PRD 内修订；PRD Q1（命名）在 D2 决定；没有影响范围、安全、数据或外部契约的未决项；四平台 CI 推送授权与宿主会话登录只影响收口能否执行，按 §9.3 记缺口。

## 12. 已知坑与历史教训

- 改 `skills/phoenix-ui/src` 会使 GENERATED.json 失配，`npm test` 在开发构建前必失败（docs/plan/phoenix-ui-hallmark.md §12）：中间批次只用 `render --without-engine` 预览，生成物在 M6 统一同步。
- 许可清单只由 `check-phoenix-deps.mjs --write` 生成，且须在构建之前；构建把三件套复制进 `dist/phoenix-ui`（docs/phoenix-ui.md「维护者」节）。
- `smoke-phoenix-ui.mjs --stage-runtime` 要求 release 目录下尚无 `runtime/`，重跑前清理旧开发候选。
- 开发构建没有下载 URL，引擎安装必须带 `--release-dir`（phoenix-bootstrap.cjs:243）。
- `crates/` 下新增 .js/.html/.css 会被自动登记为 Impeccable Apache-2.0（check-phoenix-deps.mjs:457-472）：页内脚本写进 `page_js.rs`，夹具放 `tests/`。
- oracle golden 逐字节比较，`record.mjs` 只重录有意变化的用例并逐个审阅 diff（tests/oracle/README.md:15-20）。
- macOS 上 Chrome 无法在 `sandbox-exec` 内初始化沙箱，浏览器测试须在 sandbox 外跑（scripts/test-phoenix-ui.mjs:46-48）。
- 发行版 `panic = "abort"`：外部输入路径禁止 `unwrap` 与越界索引。
- `live.md`:92-94 明确禁止无批注时自行截图：本期不在 live 中加入 capture。
- 下次发布不能用 `--reuse-engines`（引擎源码已变），须走 CI 构建（release-phoenix-ui.mjs:139、:147）。
- review-ui 轮次目录只忽略 `*.png`、`*.log` 与 `logs/`（review-ui SKILL.md:96-99）：走查报告 `.md` 会入库，capture 报告放 `logs/`。
- feature-demo 复制到演示目录的脚本不得依赖 skill（feature-demo SKILL.md:22）：录制前核对只由执行 agent 调用 capture，不写进 demo 脚本或录制器。
- `local/` 被 `.gitignore` 忽略：oil-ui 只作行为参考，夹具一律自写在 `tools/phoenix-ui/tests/fixtures/capture/`。
