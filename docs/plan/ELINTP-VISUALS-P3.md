# ELINTP-VISUALS-P3 · XGENT-ai Skills · 静态图与看图审核减量（Phoenix Diagram 引擎增量 · Rust 端生成 · elintp 内联嵌图）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-12 · `53d35750c97f865c5b739ee467eccd686c653ba7` · dirty（当前工作树含 P3 计划、评审及状态登记；代码事实按本工作树核对）。
>
> 需求来源：[elintp 模板化图文说明与 Phoenix Diagram 图解补充 PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P3（第 3 期）。本期承接 WORK-F9/AC-1 至 AC-4、WORK-F10/AC-1 至 AC-5、WORK-F5/AC-2、WORK-N3/AC-3；WORK-F5、WORK-F9、WORK-N3 的终验期是 P5，本期完成后这三条停在已提交 / 可测试。
>
> 本期交付：**phoenix-diagram 新增 `static` 命令，为十一类图型输出可内联的静态 SVG 并走亮、暗各一张的看图审核；单图审核截图由四张减为两张，架构比较由 12 张减为 6 张；elintp 在明确要求内联图时用静态图代替页内嵌入阅读器。**
> **开发范围：`tools/phoenix-diagram`（引擎与静态样式派生）、`skills/phoenix-diagram`（随工具附带的 skill 及 `dist/phoenix-diagram/bundle` 副本）、`skills/elintp`（图来源与出图规则）、复述审核截图数的现行文档；依据见 §0。**
> 本期独特职责：阅读器 HTML 是带脚本的独立页面；静态图是没有脚本、跟随宿主亮暗、可直接放进别的 HTML 的单个 SVG。
> **顶层排除：数值图表、变更视图、逐词标注及它们的静态图（P4、P5）；summarize skill 不改；不向 SPMS 提交报告；不发布 npm 包或引擎。**

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

- 最近更新：2026-11-12T20:43:01+08:00
- 当前进度：0/5 个里程碑完成（M1–M3 自动门、命令级验收与文档同步已完成，实际看图项环境缺失）
- 当前状态：active；M4 实施中
- 最近完成：M1–M3 引擎批次（提交待回填）
- 下一步：按 M3 提交点提交引擎与文档；M4 用真实引擎产物验证 `static:` 内联并执行 V-3/V-4 elintp 部分
- 当前阻塞：图像能力缺失（主模型与 scout/worker 子代理读图均返回 IMAGE_UNSUPPORTED）；实际看图项挂起，不阻塞自动门与命令级工作
- 代码基线：`53d35750`；开发引擎 `local/phoenix-diagram/elintp-p3`

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 进行中 | 2026-11-12T20:43+08:00 | 静态 SVG、两字体策略与 `static` 命令完成；V-1 自动门与无效输入、V-4 引擎测试通过；实际看图项环境缺失 | [M1 记录](ELINTP-VISUALS-P3.records/M1.md) | 待回填 |
| M2 | 进行中 | 2026-11-12T20:43+08:00 | 宿主页双覆盖浏览器检查与登记完成；V-2 恢复提示、缺观察拒绝、10 条覆盖证据齐备 | [M2 记录](ELINTP-VISUALS-P3.records/M2.md) | 待回填 |
| M3 | 进行中 | 2026-11-12T20:43+08:00 | Reader 两张/比较六张减量、§5.7 文档与 bundle 同步完成；Reader 字节与基线一致 | [M3 记录](ELINTP-VISUALS-P3.records/M3.md) | 待回填 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID，验收原文见 PRD §3，不在此重抄；「内容」列只写本期承接的部分。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F9/AC-1 | 功能 | PRD §3 WORK-F9 · P3 | 十一类任一 candidate 可取得静态图；内联进无脚本的离线 HTML 后，元素、关系、标签、图例与阅读器图面一致 | §5.1–§5.3；V-1、V-5 | 已确认 |
| WORK-F9/AC-2 | 功能 | 同上 | 静态图无脚本、不请求外部资源；宿主切换亮暗时随之切换 | §5.2、§5.3、§5.4；V-1 | 已确认 |
| WORK-F9/AC-3 | 功能 | 同上 | 静态图字节数 ≤ 同一 candidate 阅读器 HTML 的 50% | §5.1；V-1、V-5 | 已确认 |
| WORK-F9/AC-4 | 功能 | 同上 | 校验未过不产出静态图；取静态图不改变阅读器 HTML 及其交付状态 | §5.4；V-1 | 已确认 |
| WORK-F10/AC-1 | 功能 | PRD §3 WORK-F10 · P3 | 单图待审截图为 1440×900 亮、暗各一张 | §5.5；V-2 | 已确认 |
| WORK-F10/AC-2 | 功能 | 同上 | 自动检查仍覆盖四个桌面窗口与窄屏，溢出、裁剪、遮挡即失败 | §5.5（不改测量范围）；V-2、V-4 | 已确认 |
| WORK-F10/AC-3 | 功能 | 同上 | 架构比较待审截图为三视图各亮、暗一张；缺任一张观察不能登记通过 | §5.5；V-2 | 已确认 |
| WORK-F10/AC-4 | 功能 | 同上 | 只有截图、只有自动检查或没看图时保持未完成 | §5.4、§5.5（登记规则不改）；V-1、V-2 | 已确认 |
| WORK-F10/AC-5 | 功能 | 同上 | 静态图审核以亮、暗各一张为依据，登记规则与单图相同 | §5.4；V-1 | 已确认 |
| WORK-F5/AC-2 | 功能 | PRD §3 WORK-F5 · 跨期，终验 P5 | 调用方或作者明确要求内联图时，页面每张图都是静态图，没有页内嵌入阅读器；summarize 的调用满足单文件、零外部资源、图为内联及系统字体栈 | §6；V-3 | 已确认 |
| WORK-N3/AC-3 | 兼容 | PRD §3 WORK-N3 · 跨期，终验 P5 | 十一类有效输入、既有命令、已交付 HTML 保持有效；同一 candidate 的阅读器 HTML 字节不变 | §5.6；V-4、V-5 | 已确认 |
| C-1 | 约束 | PRD §7「已交付的部分持续遵守对应的质量门槛」 | elintp 的静态嵌图同样满足 WORK-N1（窄屏在自身区域横向滚动、打印可读）与 WORK-N2/AC-1、AC-2（单文件、零网络、内容不执行） | §6.2；V-3 | 已确认 |
| C-2 | 约束 | [AGENTS.md](../../AGENTS.md) `@skill_tool_independence` | elintp 不以 phoenix-diagram 的安装或引擎为执行前提；`static` 不可用时走既有降级（自绘 `svg:` 并说明） | §6.3 | 已确认 |
| C-3 | 约束 | AGENTS.md §7；PRD §5.3 的 C6、C11、C12 | 修改两个 skill 前后按整合清单核对：elintp 触发与授权不变、默认审核与完整交付含义不变、独立 HTML 不变 | §6.3、§5.7；V-5 第 4 步 | 已确认 |
| C-4 | 约束 | AGENTS.md「Working in This Repository」接口复述条款 | 截图数量与新命令的全部现行复述在同一改动内更新，历史记录不改 | §1.2 文档清单；V-5 第 4 步 | 已确认 |
| C-5 | 约束 | AGENTS.md §6；`test/phoenix-diagram/skill.test.js` L15–L27 | phoenix-diagram 描述不超过 200 字，前 100 字保留现有图型与排除词，`version: "0.2.0"` 不变；elintp 描述不改 | §5.7 | 已确认 |
| C-6 | 约束 | AGENTS.md「Working in This Repository」；`scripts/test-phoenix-diagram-native.mjs` | 引擎检查用 mise 固定的 Rust 与 `mbx`，离线、锁定依赖；工具缺失即失败，不跳过 | §9 环境 | 已确认 |
| C-7 | 约束 | [docs/phoenix-ui.md](../phoenix-ui.md) L253 | 改过 `skills/phoenix-diagram` 后运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle` 并提交 bundle；`VERSION.json`、`ENGINE.json` 不提交 | M3；V-5 | 已确认 |
| C-8 | 约束 | AGENTS.md `@simplicity`、`@failure_handling` | 工具保持直接实现：整文件覆盖写出，不加事务、备份、并发检查或回滚 | §5.4、§8 | 已确认 |
| C-9 | 约束 | PRD §4「既有决策约束」 | 旧 PRD 的 D5 与 WORK-N1/AC-2 保持：自动检查通过不等于审核通过 | §5.4、§5.5 | 已确认 |
| A-1 | 待核实 | [summarize](../../skills/summarize/SKILL.md) §3；SPMS 报告阅读面未实测 | 系统字体栈是已确认硬要求，按 D4 落实；仅真实门户对内联静态 SVG 的显示尚缺证 | 非阻塞。用户下一次真实 `/summarize` 提交后在门户查看；对外宣称 S4 实际可用之前补证，不代替本期本地验收 | 开放 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 静态图由谁生成 | 引擎在 Rust 端生成，新增 `static` 命令 | 不需要浏览器即可写出文件；Reader 分支输出字节不变 | ADR-1 |
| D2 | 静态图的样式来源 | Rust 共享静态样式函数处理 `viewer/viewer.css`、节点卡及六类 `TypeRenderer.defs` 的 CSS | 所有规则采用同一作用域与主题/媒体策略；各维护源、`template.html` 与 Reader 分支输出不变；不新增 CSS 生成物或装配转换器 | ADR-1；§5.2 |
| D3 | 怎样跟随宿主亮暗 | 宿主 `<html data-theme="light\|dark">` 优先；没有该属性时跟随 `prefers-color-scheme`；打印为亮色 | 与 elintp 页面（`skills/elintp/assets/page.css` L14–L30、L134）和阅读器的约定一致；宿主不需要脚本 | ADR-2 |
| D4 | 字体 | `static --font-policy embedded\|system`，默认 `embedded`；summarize 或明确要求系统字体时必须选 `system` | `embedded` 嵌六段 JetBrains Mono；`system` 仅用 `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`，无 JetBrains 与 `@font-face`。两者复用既有等宽布局估计，并在实际输出字体下检查文字边界与看图；不能拿 Reader 字体下的校验代替系统字体检查。内嵌字体重复是已接受代价，未来出现体积问题再评估 | summarize §3；WORK-F9/AC-1、AC-3；§5.3、§5.4 |
| D5 | 静态图包含什么 | 只含阅读器的主图 SVG（节点、关系、标签、图内图例）；不含页面标题、摘要卡片、节点索引与任何控件；静态上下文不生成 Reader 控件、按钮角色、Tab 停靠点与事件钩子 | 标题与说明由宿主页面提供；`<title>`、`<desc>` 仍在 SVG 内。重新评估：用户要求静态图自带摘要卡片 | PRD WORK-F9 边界；阅读器的完整 SVG 导出同样只导出主图（`viewer/export.js` L246–L247） |
| D6 | 多张静态图同页的 ID 冲突 | 引擎用 candidate 摘要前 8 位及字体策略给全部元素 ID 加前缀 `pd<8hex>-<policy>-` | 宿主不必改写静态图；同一 candidate 同策略放两次时 ID 相同但定义相同；不同策略不混用定义 | 复用 `render.rs` `namespace_svg`（L670） |
| D7 | 静态图的交付与审核记录 | 复用 `FinalizeReceipt` 结构，summary 写在 SVG 旁的 `NAME.static-summary.json`；`review-record` 按 `.svg` 后缀找到它 | 登记校验与单图相同；恢复提示按 summary.command 分派到 `static` 或 `finalize`，不新增审核实现 | WORK-F10/AC-5；`delivery/review.rs` |
| D8 | 静态图的自动检查 | 引擎把静态图内联进一份无脚本宿主页，在 1440×900 亮、暗各测一次并截图 | 检查独立显示、主题、无外部请求及实际字体下的文字边界；candidate 校验复用，主题/打印变体并入 V-1/V-3，不增加待审截图 | WORK-F9/AC-1、AC-2；§5.4 |
| D9 | 审核减量的实现 | 只改截图采集条件与数量断言；测量覆盖、登记校验不改；已有 summary 不迁移 | 旧 summary 列四张仍按四张登记 | WORK-F10/AC-1 至 AC-4；WORK-N3/AC-3 |
| D10 | elintp 怎样接收静态图 | `:::figure` 新增 `static:` 来源，解析 SVG 与 CSS 并通过静态安全白名单后内联 | 标记只负责来源分派；不重跑 candidate/图型语义校验或审核，不改引擎 ID；安全检查失败不输出页面，自绘图仍走 `svg:` | §6.1；WORK-N2/AC-1、AC-2 |
| D11 | 工具版本号 | 保持 0.2.0 | statechart 新增图型时也未改版本（`6bb21b6b`）；npm 版本在发布时定 | `Cargo.toml`、`skill.test.js` L25 |
| D12 | elintp 何时改用静态图 | 仅调用方或作者明确要求内联图或零网络阅读面时；默认仍嵌阅读器 | 不改变 P1 的默认行为；独立识别系统字体要求并传 `--font-policy system`，summarize 的三项硬要求全部生效 | PRD D4、WORK-F5 行为；summarize §3 |

### 0.3 ADR-lite

#### ADR-1：静态图在 Rust 端生成，全部样式在共享渲染层派生

- 状态：Accepted
- 背景与驱动：WORK-F9 要求图面完整、离线、体积减半；WORK-N3/AC-3 要求 Reader HTML 字节不变。现有主图来自共享渲染器，样式同时来自 `viewer.css`、`node-card-style.svg` 和六类 `TypeRenderer.defs`，后者包含按场景生成的 CSS。
- 备选：A. Rust 渲染层用同一静态样式函数派生全部来源；B. 构建脚本只派生固定样式，动态类型 CSS 再由 Rust 转换，会重复维护主题/作用域策略；C. 浏览器导出，增加取图依赖且仍要修正宿主作用域；D. 另写静态渲染器或 CSS，产生双份维护源。
- 决策：A。保留 Rust 场景、几何与 ID 命名空间复用，在样式与属性生成点引入静态上下文。Reader 上下文保持原始输出。无需新增装配生成物，构建脚本仍只管理现有 Reader 产物。
- 正面后果：固定与动态 CSS 共用一份静态转换策略，原样式与 Reader 模板不改；取图不依赖浏览器，交付检查仍需真实浏览器。
- 负面/中性后果：Rust 需要覆盖当前选择器和媒体规则的有界解析；未知的图面规则不能静默遗漏。两种字体策略都须按实际输出检查，系统字体跨机器的具体字形可能不同。
- 既有探索证据：原方案实验记录 Reader HTML 967–989 KB、内嵌字体静态图 157–177 KB（16.3%–17.9%）；只作为体积可行性线索，不能证明本修订方案或系统字体已经通过。两策略的十一类体积与图面按 V-5 实测。
- 重新评估触发：维护源改为当前解析不支持的 CSS 语法，或静态图需要 Reader 运行时才有的图元。

#### ADR-2：静态图用宿主 `data-theme` 与系统偏好决定亮暗

- 状态：Accepted
- 背景与驱动：WORK-F9/AC-2。静态图没有脚本，只能靠 CSS 感知宿主主题。
- 备选：
  - A. 选择器上下文：`:root[data-theme]` 优先，否则 `prefers-color-scheme`。
  - B. 变量值写成 `light-dark()`，跟随宿主的 `color-scheme`。只对颜色值有效；阅读器有按主题切换的非颜色声明（例如 classic 亮色两层、暗色一层的节点阴影，`viewer.css` L3978–L3985），仍需选择器上下文，两套机制并存。
- 决策：A。全部静态样式来源统一执行：有有效 `data-theme` 时优先；无有效属性时按系统偏好；单独打开 SVG 也按系统偏好。所有显式暗色规则仅在 `screen` 生效，打印独立选择亮色规则，不受宿主已保存暗色影响。
- 正面后果：一套机制覆盖变量与非变量声明；与本仓两个宿主一致。
- 负面/中性后果：每条按主题区分的规则输出多份选择器；主题属性不在 `<html>` 上的宿主只能跟随系统偏好。
- 重新评估触发：出现用其他属性或类名表示主题、且不能改的宿主。

### 0.4 职责与事实所有权

|  | phoenix-diagram 引擎与 skill | elintp | summarize（范围外） |
| --- | --- | --- | --- |
| 拥有 | 静态图的内容、样式、ID 前缀、自动检查、截图与审核登记；阅读器 HTML | 何时用静态图；把静态图放进页面；页面的窄屏、打印与最终页自检 | 对 elintp 提出阅读面要求；提交报告 |
| 不拥有 | 宿主页面的版式与主题切换 | **candidate/图型语义校验与看图审核**；进入主文档前的 SVG/CSS 安全解析归 elintp，不重绘、不重命名 ID | elintp 的嵌图实现 |

一句话：静态图画得对不对归 phoenix-diagram，页面里放得对不对归 elintp；`static` 失败时引擎只报错，elintp 按既有规则降级并如实说明。

### 0.5 明确不在本期

- **数值图表、变更视图、逐词标注及其静态图**——P4、P5（WORK-F9/AC-5、AC-6，WORK-F5/AC-3、AC-6）。
- **架构比较的静态图**——比较不是图型，PRD 未要求；`static` 只接受十一类单图。
- **按实际用到的字符裁剪字体、多图共用一份字体**——后续可选；本期仅 `embedded` 策略按 D4 整段嵌入，`system` 不带字体资源。
- **summarize skill 的文字与调用方式**——不改（WORK-N3/AC-2）；它写的「字体用系统字体栈」由引擎 `system` 策略与 elintp 分派满足（D4），不作为开放假设。
- **向 SPMS 提交报告、验证门户阅读面**——范围外系统；本期只验证页面本身的离线性质。
- **npm 发布、引擎四平台构建、CHANGELOG 条目**——发布流程，见 §8。
- **PHOENIX-DIAGRAM-P3 计划尚未完成的终验里程碑**——不处理，见 §12。

## 1. 当前事实与改动面

### 1.1 现状与缺口

下文 `src/…`、`viewer/…`、`assets/…` 相对 `tools/phoenix-diagram/`，`references/…` 相对所属 skill 目录。

- **已核实·足够**：主图 SVG 完全在 Rust 端生成，树默认全部展开，折叠只由阅读器脚本施加（`tools/phoenix-diagram/src/render.rs` `render_svg` L821、`render_with_layers` L1408；`viewer/tree-branches.js` L126）。
- **已核实·足够**：`namespace_svg(svg, prefix)` 改写 `id`、`href`、`aria-labelledby` 与属性里的 `url(#…)`（`render.rs` L670–L772）。`viewer.css` 与各类型渲染器的 `<style>` 文本里没有 `url(#`。
- **已核实·足够**：阅读器 SVG 导出按选择器前缀收集规则：`svg`、`:root`、`[data-theme`、`[data-preset`、`.c-`、`.t-`、`.a-`、`.m-`，并把 `.diagram-container > svg` 改写为 `svg`（`viewer/export.js` L275–L305）。`viewer.css` 对此有明文约定（L4000–L4002）。
- **已核实·足够**：阅读器模板 959,181 字节，其中字体样式块 95,564 字节（`viewer/template.source.html` L61–L194）。
- **已核实·足够**：浏览器检查对四个桌面窗口与 390×844 窄屏各测亮、暗，共 10 次；`coverage.len() == 10` 才通过（`src/browser/mod.rs` L66–L87、L280、L369）。
- **已核实·缺口**：截图采集条件写死为最小与最大桌面窗口，数量断言为 4 与 12 → 改 `src/browser/evidence.rs` L9–L11、`mod.rs` L343–L344、`comparison.rs` L676 → 漏改则回执仍要求旧数量（WORK-F10/AC-1、AC-3）。
- **已核实·足够**：审核登记只比对「记录列出的截图」与「summary 列出的截图」是否一一对应，不关心数量（`src/delivery/review.rs` `validate` L83–L112）；比较的登记复用同一函数（`src/compare/review.rs` L115）。
- **已核实·缺口**：没有静态输出命令；`summary_path` 只认 `.html`（`src/delivery/support.rs` L92）→ §5.4 → 漏做则 `review-record` 找不到静态图的 summary。
- **已核实·缺口**：elintp 的 `svg:` 白名单拒绝 `<style>`、`style` 属性、`data-*` 与白名单外元素（`skills/elintp/scripts/draft.mjs` L7–L8、L44、L57）→ 静态图不能走 `svg:`，需新增来源 → 漏做则 summarize 场景只能自绘。
- **已核实·足够**：elintp 的资源快照按路径存原始字节，与来源类型无关（`draft.mjs` `resource` L90–L97）；审阅页、局部重讲与终稿对新来源无需改动。
- **已核实·足够**：elintp 宿主页用 `:root[data-theme]` 与 `@media screen and (prefers-color-scheme: dark)` 切换主题，打印强制亮色（`skills/elintp/assets/page.css` L14–L30、L134）；页面没有 CSP；页面类名与静态图的 `.c-`、`.t-`、`.a-`、`.m-` 不冲突。
- **已核实·足够**：launcher 不限制子命令，除 `engine` 与 `install` 外的参数原样传给引擎（`skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs` `main` L197–L225）。`skill.test.js` L82–L93 要求文档里出现的 `"$PD" <命令>` 在 `cli.rs` 存在。
- **已核实·足够**：前置期 P1、P2 已完成（[P1 计划](ELINTP-VISUALS-P1.md)恢复快照 6/6，[P2 计划](ELINTP-VISUALS-P2.md) 4/4），`:::figure`、审阅与终稿链路在当前 HEAD 可用。

### 1.2 拓扑与文件清单

**引擎（`tools/phoenix-diagram`）**

- `tools/phoenix-diagram/src/render.rs`：新增 `render_static`、`RenderContext` / `FontPolicy`，共享属性生成点按上下文输出。
- `tools/phoenix-diagram/src/render/static_style.rs` ★：统一派生 viewer、节点卡与类型 CSS。
- `tools/phoenix-diagram/src/render/{erd,class,tree,timeline,waterfall,statechart}.rs`：`TypeRenderer` 的 defs/body/legend 入口接收上下文；静态属性、控件与样式就地分派，Reader 分支保留原字节。
- `tools/phoenix-diagram/src/delivery.rs`、`delivery/finalize.rs`、`delivery/support.rs`、`delivery/review.rs`：`static_figure` 入口、`summary_path` 认 `.svg`、审核恢复提示分派。
- `tools/phoenix-diagram/src/browser/static_figure.rs` ★、`static-probe.js` ★、`browser/mod.rs`、`browser/cdp.rs`、`browser/evidence.rs`、`browser/comparison.rs`：静态图检查与截图；采集条件与数量。
- `tools/phoenix-diagram/src/cli.rs`：`static` 命令、帮助文本。
- `tools/phoenix-diagram/tests/{browser_contract,delivery,compare,compare_browser}.rs`：数量断言随行为更新。
- `tools/phoenix-diagram/tests/delivery.rs`：静态样式派生与字体策略的外部产物用例（V-4）。
- 不改：`viewer/viewer.css`、`viewer/*.js`、`viewer/template.source.html`、`assets/template.html`、`assets/node-card-style.svg`、装配脚本、`skills/phoenix-diagram/schemas`、各类型布局与既有检查。

**phoenix-diagram skill 与复述**

- `skills/phoenix-diagram/SKILL.md`、`references/delivery-contract.md`、`references/architecture-compare.md`、`references/preview.md`、`references/viewer-runtime.md`；`references/static-figure.md` ★。
- `dist/phoenix-diagram/bundle/**`：由 `--sync-bundle` 重新生成。
- `docs/phoenix-diagram.md`、`README.md` L216、`skills/documentation-writer/SKILL.md` L151。
- 不改：`docs/PRD-PHOENIX-DIAGRAM.md`、`docs/plan/**` 既有计划与记录、`.xgent-ai/**` 既有记录、`docs/diagrams/**`。

**elintp**

- `skills/elintp/scripts/draft.mjs`、`skills/elintp/assets/page.css`、`skills/elintp/references/draft-format.md`、`skills/elintp/SKILL.md`。
- `test/elintp/render.test.js`：`static:` 的用例（V-4）。
- 不改：`skills/summarize/SKILL.md`、`review.mjs`、`render.mjs`、`templates.mjs`、elintp 的描述与触发。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `render::static_style` | 共享渲染器的静态分支 | 输入各来源 CSS 与静态根作用域，统一输出 scoped、按主题/媒体分派的 CSS；同输入同输出 | 无 | 当前选择器与媒体规则解析（§5.2） | 进程内 | CLI 写出的 SVG；V-4 |
| `render::render_static` | `delivery::static_figure` | 输入已编译场景、ID 前缀与 `FontPolicy`，输出 SVG 字节；几何与内容复用，样式及交互语义按静态上下文生成 | 无 | 根属性、样式与底色矩形的插入位置 | 进程内 | 写出的 `.svg` |
| `delivery::static_figure` | `cli.rs` | 校验 → 写出 → 浏览器检查，返回并落盘 `FinalizeReceipt` 形态的 summary | 无 | 阶段顺序与失败语义（§5.4） | 本地文件系统 | 回执、summary、文件 |
| `browser::check_static` | `delivery::static_figure` | 输入 `.svg` 路径，返回 `BrowserReceipt`（2 条覆盖、至多 2 张截图） | 无 | 宿主页生成、主题切换、离线判定 | 本机 Chrome（既有发现逻辑） | 回执与截图 |
| `RenderContext` | `render_svg_layers`、`write_nodes` 与六类 `TypeRenderer` 的 defs/body/legend | `Reader` 保持原字节；`Static(FontPolicy)` 在生成点省略交互并统一派生样式 | 无 | 几何/绘制属性与 Reader 钩子的区分 | 进程内 | 无失效按钮语义；V-1/V-3/V-5 |
| elintp `static:` 来源 | `parseDraft` 的 `figure` 分支 | 根标记用于分派，全部 SVG/CSS 必须通过静态安全解析；保留 ID，安全重序列化后内联 | 无 | 元素/属性、CSS 作用域与 URL/MIME 位置白名单（§6.1） | 本地文件（不可信边界） | 生成页面与拒绝诊断；V-3/V-4 |

不新增接缝或适配器：静态样式函数由多个样式生成点共享，静态交付与浏览器检查各有单一实现。删除测试：去掉 `static_style`，同一作用域/主题策略会散进 viewer、节点卡与六类渲染器，故保留；去掉 `browser::check_static`，宿主页与两主题的采图逻辑会塞进 `check_browser` 的十次循环，故单列。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| elintp → phoenix-diagram 静态图文件 | 已核实缺口 | elintp 现有来源只有 `reader:`、`svg:`（`draft.mjs` L265–L267） | 范围内：§5.3 定义文件契约，§6.1 新增来源 | 文件读不了、没有标记或安全解析不通过是内容稿格式错误（退出 2），给行号与改法，不覆盖旧页 |
| elintp skill → phoenix-diagram skill | 已核实足够 | elintp 自行查找并读取 `phoenix-diagram/SKILL.md`，不自动安装（`skills/elintp/SKILL.md`「出图」） | 范围内：两处文字同步静态图流程 | 未安装、引擎没有 `static` 或流程失败 → 自绘 `svg:` 并在交付说明写明原因 |
| summarize → elintp | 已核实足够 | summarize §3 以文字提出「单文件、不引用外部资源、图用内联 SVG，字体用系统字体栈」 | 零改动；elintp 识别内联及系统字体要求，传 `--font-policy system` | — |
| SPMS 报告阅读面 | 未核实，不阻塞 | 仅有 summarize §3 的描述；本期不提交报告 | 范围外。提供方：SPMS 门户。A-1 仅记录真实显示未核实项 | 见 A-1 |
| documentation-writer skill | 已核实足够 | L151 复述「四张暗亮截图」，L161 从 summary 取亮色截图 | 只改 L151 的数量文字；流程不变 | — |
| `review-record` → 静态/HTML summary | 已核实缺口 | `delivery/review.rs` 的两条诊断只写 `finalize` 与 HTML | M2 按 summary.command 选择恢复命令，复用原校验和诊断码 | `review/automated-gates` 提示先重跑 `static` 直到自动门通过；`review/artifact-changed` 提示 SVG 已变，重跑 `static` 后重新看图登记；HTML 仍提示 `finalize` |
| 本机 Chrome | 已核实足够 | `browser::discovery::find`；缺失时回执 `unavailable`、退出 5 | 复用 | 静态图已写出，summary 记 `unavailable`，不能登记审核 |

登记位置：新命令在 `cli.rs` 的 `HELP` 与命令匹配表；新诊断码 `static/output-extension`；新来源在 `draft-format.md` 的图来源一节与「常见报错」表。没有新的配置、环境变量或回执 schema 版本。

## 5. 核心机制（引擎）

### 5.1 `static` 命令

```text
static <type> <candidate.json> <output.svg> [--font-policy embedded|system] [--quality standard|showcase] [--repo-root DIR] [--brand-cache DIR] [--out-dir DIR] --json
```

- 除新增 `--font-policy embedded|system`（默认 `embedded`，其他值按 CLI 参数错误退出 2）外，参数与 `finalize` 相同；`<type>` 是十一类之一，其他值按现有逻辑报 `Unsupported diagram type`。
- `<output.svg>` 必须以 `.svg` 结尾，否则退出 2、诊断码 `static/output-extension`、不写任何文件。这是新增输出边界校验：summary 与截图的命名、`review-record` 的分流都依赖后缀。
- stdout 是一个 JSON（`FinalizeReceipt`，`command: "static"`），同时写入 SVG 旁的 `NAME.static-summary.json`。下文 `NAME` 都指输出文件名去掉 `.svg` 后的部分。
- `cli.rs` 落点：`dispatch` 的命令白名单（L364–L376）与 `validate | deliver | finalize` 分支（L460–L518）并入 `static`，选项在 `finalize` 基础上增加仅 `static` 接受的 `--font-policy`；`HELP` 增加一行，`review-record` 一行改为 `review-record <output.html|output.svg> --record <review.json> --json`。

### 5.2 全部静态样式在 Rust 共享层派生

`render/static_style.rs` ★ 提供一个静态 CSS 派生函数；只在 `RenderContext::Static` 中调用，Reader 分支仍逐字写原样式。不新增依赖，不在 elintp 改 CSS，也不复制维护源。

**来源与接入点**：

| 样式来源 | 静态分支处理 | Reader 分支 |
| --- | --- | --- |
| `viewer/viewer.css` | 用 `include_str!` 读取维护源，按下表筛选图面规则并派生 | 模板装配不变 |
| `assets/node-card-style.svg` 的 `<style>` | 在当前插入点取 CSS，经同一函数处理；识别七类节点的 `:is([data-theme="dark"] svg[…], svg[…][data-theme="dark"])` 为暗色规则 | 保留原片段 |
| `TypeRenderer.defs`：erd、class、tree、timeline、waterfall、statechart | defs 的几何定义原样复用；每个 CSS 块在生成点统一经同一函数处理，包括场景生成的 hue 规则、类型选择器与嵌套媒体规则 | 保留原 defs 的字节与顺序 |
| `viewer/template.source.html` 的 `phoenix-diagram-fonts` 样式块 | 仅 `embedded` 取六段字体；`system` 不取此块 | 模板不变 |

`ROOT` 为 `svg[data-phoenix-static]`。解析当前维护源的规则块、声明、选择器列表及媒体块（括号/引号内逗号不能拆）；不是正则全局替换，也不实现通用 CSS 引擎。输出规则：

| 输入形态 | 静态规则 |
| --- | --- |
| `svg[…]`、`.diagram-container > svg` 及 `.c-` / `.t-` / `.a-` / `.m-` 图面类 | 都锚定 `ROOT`；保留图型、预设条件与后代选择器 |
| `html` / `:root` / 祖先或 SVG 上的 `data-theme`，含节点卡的 `:is` 两分支 | 抽出主题条件后统一按 ADR-2 展开，不能留下另一套未限定媒体的主题选择器 |
| `:root`、`data-preset`、`data-theme` 变量块 | 变量只定义在 `ROOT`（有预设则加预设条件），不能覆盖宿主根变量 |
| 无主题的图面规则及类型 CSS | 锚定静态根并保留声明/必要媒体条件；Reader 的 hover/focus、动画、折叠等交互规则不进入静态图 |
| 页面选择器、Reader 控件与运行时状态规则 | 不进入静态图；不得据此前缀丢弃节点卡或类型图面规则 |

主题展开对所有来源一致：亮色屏幕规则只在显式 light 或无有效主题且系统亮色时生效；暗色屏幕规则只在显式 dark 或无有效主题且系统暗色时生效；独立 SVG 根同样支持系统偏好。打印独立输出亮色规则，不让显式 dark 的屏幕规则参与；已有类型 `@media print` 规则保留并加静态作用域。at 规则按上述图面用途处理；无关动画可删除，遇到无法处理的图面语法须报渲染失败并定位源，不能静默缺图。

字体规则按 `data-font-policy` 作用于对应静态根及文字：`embedded` 嵌字体并用原等宽栈，`system` 严格使用 D4 的系统栈；源内任何字体声明或简写均不能重新引入 JetBrains 到 system 分支。无主题、亮色、暗色、打印的顺序与 specificity 必须确保 ADR-2 的优先级。背景矩形用 `ROOT .c-bg-rect { fill: var(--bg); }`。

同页的不同图及 Reader SVG 不受彼此样式影响：类型/预设条件保留；场景生成规则（如 hue）锚定当前图的根 ID，根 ID 使用 §5.3 的前缀。公共静态规则只作用于静态图；两字体策略的选择器互不覆盖。唯一宿主全局例外为 embedded 策略的内嵌字体族声明。

### 5.3 静态图文件的内容

共享渲染器引入显式上下文：

```rust
pub enum FontPolicy { Embedded, System }
pub enum RenderContext { Reader, Static(FontPolicy) }
pub fn render_static(scene: &CompiledDiagram, id_prefix: &str, font: FontPolicy)
    -> Result<Vec<u8>, Diagnostics>
```

`render_svg_layers`、普通节点属性生成点，以及六类 `TypeRenderer` 的 root/defs/body/legend 路径接收上下文。Reader 现有入口始终传 `Reader`，字节不变；静态模式共用几何、内容、定义与绘制流程，不复制渲染器，不在输出后正则删属性。

1. 静态分支在属性生成点保留几何、绘制、节点/关系身份、必要图型与预设属性、`<title>` / `<desc>` 和根 `role="img"`、`aria-labelledby`、语言；这些是图面与无障碍名称，不是按钮。
2. 普通节点、erd/class 共用节点属性、tree/timeline/waterfall/statechart 的各自入口与图例均省略 `tabindex`、`role="button"`、`aria-pressed`、控件 `aria-expanded` / `aria-controls` 及 Reader 事件/动画钩子（如 `data-animate`、`data-motion-*`、`data-animation`）。保留用于绘制的 `data-node-id`、`data-graph-role` 等属性。tree 不生成 toggle / expand-all 控件，全部展开；不靠隐藏控件实现无交互。
3. 全部 CSS 按 §5.2 在生成点派生；defs 中的 marker、pattern、clipPath 等非样式定义保留。根追加 `data-phoenix-static=""`、`data-font-policy="embedded|system"`、`width`、`height`、唯一根 ID（前缀 + `figure`），在图元前插入背景矩形，四个值取 `scene.view_box`。
4. 通过既有 `namespace_svg` 处理 ID / 本地引用，不在 CSS 中产生未命名空间化的 `url(#…)`；场景专属样式根锚点使用同一前缀。UTF-8，无 XML 声明。

文件契约（elintp 与其他宿主依赖的全部内容）：

- 单个带 `data-phoenix-static` 的 `<svg>`，有 `viewBox`、尺寸、字体策略、图型/预设及根可访问名称。
- 无脚本、事件属性、`foreignObject`、外部资源或交互语义/控件；资源仅允许文件内字体和安全品牌图片，具体 MIME/位置边界见 §6.1。`system` 不含字体资源、JetBrains 与 `@font-face`。
- 所有 `<style>` 来源已在 §5.2 枚举并统一处理；内联 style 属性只保留绘制声明，同样受安全解析。除 embedded 字体族声明外均只影响静态图，场景专属规则不影响别的图。
- 独立打开、无主题属性的系统亮/暗、显式亮/暗、显式暗色下打印及多图同页都遵守 ADR-2。无搜索、缩放、Route / Reach、播放、导出和键盘空按钮。
- ID 前缀：`pd` + candidate 的 SHA-256 前 8 位 + `-embedded-` 或 `-system-`，candidate 摘要取自 `ValidationReceipt.candidate.identity.sha256`；两策略同页时定义不冲突，同 candidate 同策略重复时定义一致。
- 布局复用当前等宽宽度估计（`geometry.rs` 的 `text_units` / `text_glyph_box`），不把它说成指定字体的真实字宽。浏览器必须在本次策略输出的 SVG 上等待字体就绪、测文字/节点边界并看图；system 字形可随平台变化，但文字内容、几何关系和可辨认性不能缺失。不为通过验收而改 Reader HTML。

### 5.4 静态图的交付流程

`delivery.rs` 新增 `pub fn static_figure(request, output, browser, font_policy) -> FinalizeReceipt`，实现放 `delivery/finalize.rs`，照 `finalize`（L79–L157）写，差异如下：

1. **validate**：`prepare::prepare(request)`，与 `finalize` 相同。未通过时后两个阶段 `not-run`，不写 SVG；已存在的同名旧文件保持原样（与 `deliver` 一致）。
2. **static**（阶段名 `static`，取代 `deliver`）：`render::render_static(&compiled.scene, &prefix, font_policy)`，渲染失败则该阶段 failed、浏览器 not-run，不覆盖旧 SVG；成功后用 `support::write` 写到 `output`；`summary.artifact` 记录该 SVG 的路径与身份。输出路径与 candidate 是同一文件时沿用 `output/input-alias` 失败。
3. **browser-check**：`browser::check_static(output, &options)`，`options.capture = true`。
4. summary 写到 `summary_path(output)`。`support::summary_path` 增加：文件名以 `.svg` 结尾时返回 `NAME.static-summary.json`。
5. 状态字段语义与 `finalize` 相同：三阶段通过后 `automatedStatus: "pass"`、`visualReview.status: "required-pending"`、`deliveryStatus: "incomplete"`；浏览器不可用时 `status: "unavailable"`，SVG 已写出。

`review-record figure.svg --record r.json`：`cli.rs` 现有分流不改（`.svg` 不是比较产物，落到 `delivery::record_review`）；`review::record` 经 `summary_path` 自动读到静态 summary，「自动门已通过、文件未变、记录逐一列出当前截图」三项校验原样生效（WORK-F10/AC-4、AC-5）。`delivery/review.rs` 依据 `summary.command` 分派两条恢复信息：`static` 的自动门失败提示重跑 `static` 直到通过，产物变化提示 SVG 已变化、重跑 `static` 再看图登记；原 `finalize` 的 HTML 提示保持不变。诊断码、身份/截图校验和记录结构不变。

`static` 只写 `.svg`、`NAME.static-summary.json` 与 `NAME.static.1440x900.{light,dark}.png`，不读写同名 `.html`、`.finalize-summary.json` 及其截图（WORK-F9/AC-4）。

**`browser::check_static`**（`src/browser/static_figure.rs` ★，在 `mod.rs` 以 `pub use` 导出）：

- 启动、发现、版本、关闭、失败回执照 `check_browser`（`mod.rs` L200–L379）；返回同一 `BrowserReceipt` 类型，`command: "browser-check"`。
- 宿主页：把 `.svg` 全文放进下面的 HTML，写到截图目录下的 `NAME.static-host.html`，检查结束后删除。

  ```html
  <!doctype html><html lang="en" data-theme="THEME"><head><meta charset="utf-8"><title>static figure</title>
  <style>body{margin:24px}svg{display:block;max-width:100%;height:auto}</style></head><body>SVG</body></html>
  ```

- 对 `Theme::Light`、`Theme::Dark` 各一次：把宿主页的 `THEME` 写成对应值，按 `DESKTOP_VIEWPORTS[0]` 导航并执行 `static-probe.js` ★，取得度量后截图。`cdp.rs` 增加 `inspect_static`（照 `inspect` L661–L671，换探针）与 `screenshot_static`（照 `screenshot` L718，稳定条件改为等待 `document.fonts.ready` 与两帧）。
- 探针返回：`documentUrl`、`innerWidth`、`scrollWidth`、`scripts`（`document.scripts.length`）、`figures`（`svg[data-phoenix-static]` 的个数）、`bounds`（该 SVG 的 `getBoundingClientRect`）、`background`（`.c-bg-rect` 的计算 `fill`）、`fontsStatus`、所有文字的实际字体族/边界、节点边界与可见性、`resources`（`performance.getEntriesByType('resource')` 的名称）。
- 每个主题四项检查，名称沿用 `BROWSER_MEASUREMENT_CHECKS` 中已有的：

  | 检查 | 通过条件 |
  | --- | --- |
  | `rendered-dom` | `figures == 1`，`scripts == 0`，`bounds` 宽高大于 0；按实际字体测量，文字非空、可见且无节点/画布裁剪（复用现有 probe 的文字边界算法），无按钮、Tab 停靠点或 Reader 事件钩子 |
  | `viewport-containment` | `scrollWidth <= innerWidth` |
  | `offline-resources` | 事件流中没有 `observation::unexpected_resource` 判为意外的请求，没有 `Network.loadingFailed`，`fontsStatus == "loaded"`；system 无字体资源与 `@font-face`，计算字体族符合 D4 |
  | `theme-state` | 两个主题都测完后比较：亮、暗的 `background` 不同；结果写进两条覆盖 |

- 汇总：两条覆盖全部通过、`browser-cleanup` 通过、`screenshot-capture` 通过（`required: 2`）才是 `Pass`。
- 截图：`Evidence::new(artifact, out_dir, Some("static"))`；`evidence.rs` 取文件名主干时在去掉 `.html` 之外也去掉 `.svg`，得到 `NAME.static.1440x900.light.png`。

### 5.5 审核截图减量

- `src/browser/evidence.rs` `requires_capture`：只在 `viewport == DESKTOP_VIEWPORTS[0]` 时采集；注释改为当前事实。
- `src/browser/mod.rs` L343–L344：`captured` 改为 `== 2`；`scope` 的 `required` 为 2，`viewports` 只列 `DESKTOP_VIEWPORTS[0]`。
- `src/browser/comparison.rs` L676：`count == 6`，`required` 为 6，`viewports` 同上。
- 不改：十次（比较为每视图十次）测量循环、`coverage.len()` 断言、`observation::classify`、`review::validate`。
- 已有 summary 不迁移：它列几张截图，登记时就要求几张。

### 5.6 兼容

- 阅读器 HTML：`render_with_layers` 只向共享渲染器传 Reader 上下文；其装配、`template.html`、`viewer/**` 与 Reader 分支生成字节不改，同一 candidate 的 HTML 字节不变。V-5 用改动前后的 SHA-256 对比证明。
- 既有命令的参数、回执字段与退出码不变；回执里只有 `screenshots` 的条数与 `screenshot-capture` 的 `scope` 数值变化。
- `build.rs` 的 `INPUTS` 已包含 `assets` 目录，新生成物自动计入 `sourceInputSha256`，不需要改 `build.rs`。

### 5.7 phoenix-diagram skill 与文档的文字

- `SKILL.md`：
  - 描述：把「齐备后交付经 Rust、浏览器与实际看图核对的独立 HTML」改为「齐备后交付经 Rust、浏览器与实际看图核对的独立 HTML，可另取可内联的静态 SVG」；其余不动。改后读全文与前 100 字，确认不超过 200 字（当前 151 字）。
  - 「以下能力按用户请求分别启用」列表增加一项：明确要求可内联的静态图时读 `references/static-figure.md`。
  - 「最短创作路径」第 4 步的「四张暗/亮截图」改为「1440×900 亮、暗两张截图」。
- `references/static-figure.md` ★：何时取静态图；命令与字体策略入口/默认值（系统字体要求必须选 `system`）；产物三类文件；宿主约定（ADR-2 与 §5.3 文件契约）；看两张截图、写审核记录、`review-record figure.svg`；完成条件；与阅读器交付相互独立；`static` 不存在或失败时如实报告。
- `references/delivery-contract.md`：首段与「浏览器与实际看图」「审核记录」「架构比较的收口」中的四张、12 张改为两张、六张，并写明截图为 1440×900；末尾加一句指向 `static-figure.md`。
- `references/architecture-compare.md` L34、L40：12 张改为 6 张（每个视图 1440×900 亮、暗各一）。
- `references/preview.md` L43：「四张截图」改为「两张截图」。
- `references/viewer-runtime.md`：加一句区分阅读器内的 SVG 导出（读者触发）与作者命令 `static`。
- `docs/phoenix-diagram.md`：首段能力列表加静态图；「公开命令」加 `static` 一行并更新 `review-record`；「看图后收口」「按请求启用的作者工具」中的四张、12 张按上面口径改；新增一小节「静态图」摘要并链接 `static-figure.md`。
- `README.md` L216：phoenix-diagram 一行加「可内联静态图」。
- `skills/documentation-writer/SKILL.md` L151：「四张暗亮截图」改为「两张暗亮截图」。
- 完成后运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle`（C-7）。

## 6. elintp 嵌图

### 6.1 `static:` 来源与宿主安全边界（`skills/elintp/scripts/draft.mjs`）

- `figure` 的来源匹配由 `reader|svg` 扩为 `reader|svg|static`，格式提示同步。资源快照、图注、外壳和审阅标注沿用现有流程。
- 在 `draft.mjs` 复用现有 SVG token、实体解码、结构/属性检查与报错方式，为静态模式增加显式白名单；不放宽自绘 `svg:` 的规则。解析成功后安全重序列化至 `.image-scroll` 容器，保留引擎 ID 与本地引用；不走原样拼接或 iframe。
- 根须为一个带标记及 §5.3 必要属性的 SVG；标记只用于分派，不证明可信来源。BOM、XML 声明与注释可去掉；拒绝 DOCTYPE/实体声明、无效结构、命名空间切换、脚本、事件属性、`foreignObject`、嵌套浏览上下文与可执行/交互元素。普通自绘图无标记仍报「不是 phoenix-diagram 静态图」，伪造标记也必须经过全部安全检查。
- 静态元素在现有绘制白名单上按实际引擎产物补 `style`、`image`、`pattern`、`filter` 及所用滤镜原语等静态定义；属性按实际绘制需求补 `data-*` 与绘制 style。`data-*` 仅是数据，不授权事件执行；拒绝 `tabindex`、按钮/按压语义与 Reader 事件钩子。保留 `role="img"` 及可访问名称。
- CSS 必须结构化解析选择器、声明、at 规则与 URL；只允许 §5.2 产物所需的 scoped 绘制规则、主题/打印媒体及 embedded 字体声明。每个选择器的目标必须在静态 SVG 内，允许 `:root[data-theme]` 作为只读祖先条件，不允许匹配宿主元素、跳出 SVG 的组合器或全局变量改写。禁止 `@import`、脚本表达式、外部 URL；实体与 CSS 转义须在 URL/属性判断前解码或对不支持的转义明确拒绝，不能只扫描危险字符串。声明白名单包含实际颜色/变量/字体/绘制属性，不能接受任意 CSS。
- URL 按位置校验：绘制引用（`fill` / `stroke` / marker / filter / clip / mask / `use`）只允许本文件 `#片段`；`@font-face src` 仅允许 base64 `data:font/woff2`，不允许 `local()` 或网络字体；`image href` 仅允许 base64 的 PNG、JPEG、WebP、ICO（`image/png`、`image/jpeg`、`image/webp`、`image/x-icon`、`image/vnd.microsoft.icon`），与当前品牌捕获支持的 MIME 一致；校验编码及图片类型，拒绝 `data:image/svg+xml`（当前引擎品牌捕获也拒绝 SVG），无需新增递归 SVG 解析。其他 `data:` MIME/位置、http(s)、协议相对 URL 和文件路径均拒绝。system 分支连字体 data 与 `@font-face` 也不允许。
- 文件或 SVG/CSS/资源不合约时退出 2，给出内容稿行号、内部 SVG 位置、问题与改法；不覆盖旧页面，不修改原 SVG。
- 职责：这里检查进入宿主 DOM 的可执行性、网络与样式影响边界，不重跑 candidate、图型几何语义或实际看图审核；这些仍由引擎拥有。页面最终自检不能替代输入安全解析。

### 6.2 页面样式（`skills/elintp/assets/page.css`）

- 窄屏媒体块（现有 `figure img { max-width: none; }` 旁，L131）加：`figure svg[data-phoenix-static] { width: auto; max-width: none; }`。静态图按自身 `width`、`height` 显示，在 `.image-scroll` 内横向滚动。
- 桌面沿用 L111 的 `figure svg`（铺满宽度、等比缩放）；打印块（L141 旁）加 `figure svg[data-phoenix-static] { width: 100%; max-width: 100%; }`。
- 主题不需要任何样式：静态图自己跟随 `:root[data-theme]` 与系统偏好，打印为亮色。

### 6.3 skill 文字

- `references/draft-format.md`：图来源示例加一段 `static:`；说明句改为「首个非空行只能是一个 `reader:`、`svg:` 或 `static:` 来源」；写明 `static:` 只引用 phoenix-diagram `static` 命令的产物，先经宿主安全解析再内联；标记不代表安全；跟随页面亮暗、不修改源文件、不替图工具做语义校验或审核；列明两字体策略及安全错误的改法；「常见报错」加一行「不是 phoenix-diagram 静态图」。
- `SKILL.md`「出图」一节：
  - 新增一段：调用方或作者明确要求内联图、内联 SVG 或零网络阅读面时，页面里每张图都用 phoenix-diagram 的静态图，以 `:::figure` 的 `static:` 引用，不用 `reader:`；静态图的生成、看图与审核登记按 phoenix-diagram 的流程，这里不复述。没有这类要求时默认仍用 `reader:`。字体要求独立识别：summarize 或作者明确要求系统字体时，取图必须传 `--font-policy system`；其他静态图默认 `embedded`。系统字体产物必须在该策略下完成浏览器检查与实际看图，不能在 elintp 删除字体后冒用 embedded 产物的审核记录。
  - 降级句补一种情形：已安装的 phoenix-diagram 没有静态图命令或按其流程无法交付时，同样改用自绘 `svg:` 并在交付说明写明原因。
  - 「合并后仍是一个离线 HTML」一段补一句 `static:` 的嵌入方式。
- 「交付前自检」第 9 项与交付说明的「图由 phoenix-diagram 交付还是自绘」一项补充图的形态（页内嵌入阅读器 / 静态图 / 自绘）。
- 不改：描述、触发、语言选择、红线、写作准则、转述硬规则、模板与审阅流程（C-3，PRD §5.3 的 C6、C7）。

### 6.4 整合清单在本期的落点

PRD §5.3 的清单是基线；本期只触及其中三项。实施 diff 逐项对照，出现清单外的行为变化先停下修订本节。

| PRD 清单项 | 目标文件 | 现有行为 | 本期改动 | 对原则、触发与授权的影响 | 核对 |
| --- | --- | --- | --- | --- | --- |
| C6 · elintp「出图」 | `skills/elintp/SKILL.md`、`references/draft-format.md` | 已安装就调用 phoenix-diagram，以 `reader:` 嵌阅读器；否则自绘 | 调整：明确要求内联时改用经安全解析的 `static:`；系统字体要求传给引擎；默认与降级规则不变 | 不自动安装、不新增检测脚本、触发与描述不变 | V-3；V-5 第 4 步读 diff |
| C11 · 交付合同的审核截图 | `skills/phoenix-diagram/references/delivery-contract.md`、`architecture-compare.md` 及 §5.7 所列复述 | 单图四张，比较 12 张 | 调整为两张、六张 | 默认审核与「三门加看图才算完整交付」不变 | V-2；V-5 第 4 步 |
| C12 · 输出形态 | `skills/phoenix-diagram/SKILL.md`、`references/static-figure.md` ★ | 只有独立 HTML | 新增静态图；独立 HTML 及其流程不变 | 静态图同样默认要求看图审核；不扩大自动触发 | V-1；V-5 第 3 步 |

## 8. 失败模式、发布与回滚

运行期失败沿用单次命令语义：`static` 是单次本地命令，整文件覆盖写出（`output::write` 先写临时文件再改名），失败时返回带阶段的诊断，由作者重跑原 `static` 命令（HTML 交付仍重跑 `finalize`）；没有并发写、后台任务或跨资源提交。浏览器不可用沿用既有 `unavailable` 语义。

- 发布顺序：引擎与 skill 随同一个 npm 包发布；本计划不发布。发布时按 [docs/phoenix-ui.md](../phoenix-ui.md) 的 `release:phoenix` 流程，源码输入已变，不能用 `--reuse-engines`。CHANGELOG 在发布时记录 `static` 命令与审核截图减量。
- 放量/切换门：无。
- 回滚门：回退提交即可；旧引擎不认识 `static`，elintp 按 §6.3 降级。已生成的静态图是普通文件，不受影响。

## 9. 验证

### 9.1 行为与接口验证

**环境**

- 在仓根执行；工具已按 `node scripts/prepare-phoenix-diagram.mjs --prepare` 准备好，缺失即失败（C-6）。
- `CARGO` 表示 `mise exec --no-deps -- mbx`，`MF` 表示 `--manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline`。
- 开发引擎：`mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/elintp-p3 --sync-dist`，引擎为 `E=local/phoenix-diagram/elintp-p3/phoenix-diagram-darwin-arm64`。输出目录须为新目录或空目录；耗时未知。
- 产物目录：`OUT=local/phoenix-diagram/elintp-p3-out`（`local/` 不入库）。
- 看图由具备图像能力的 Agent 实际打开截图完成；没有图像能力时如实保留未完成。

**基线（M1 动工前执行一次，结果写进 M3 记录）**

用改动前的引擎（本机缓存中 `sourceInputSha256` 为 `cff3ac11…` 的引擎，或在基线提交上按上面命令构建到 `local/phoenix-diagram/elintp-p3-base`）：

1. 对十一类最小示例各执行 `deliver <type> skills/phoenix-diagram/examples/minimal.<type>.json local/phoenix-diagram/elintp-p3-base-out/<type>.html --json`，记录每个 HTML 的 SHA-256。
2. 对 workflow 执行一次 `finalize`（保留它的四张截图与 summary，不登记审核），供 V-5 验证旧交付。

**批内最小反馈**

- 单个 Rust 测试：`$CARGO test $MF --test <文件> <过滤词>`；只编译：`$CARGO test $MF --no-run`。
- Node 单文件：`mise exec --no-deps -- node --test <文件>`。
- 开发构建只在批末或收口；整 crate 测试与 clippy 只在 V-5 收口运行。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **图解作者取一张能放进别的文档的图，并完成审核。** 输入：作者按本计划 §5.4 的交付流程写的 workflow candidate，另加 `minimal.tree.json`、`minimal.erd.json`。覆盖 WORK-F9/AC-1 至 AC-4、WORK-F10/AC-4、AC-5。结果：<br>• 得到 `.svg`、两张截图与 summary，自动阶段全过、审核待办<br>• 把多张 SVG 放进空白无脚本 HTML（另放未带静态标记的 SVG 对照），两字体策略各自的亮、暗下节点、关系、标签、图例齐全；字体允许按策略变化，文字无缺失或裁剪，内容与几何关系对照同一 candidate 的阅读器截图一致；树全部展开且无折叠按钮；普通节点与类型节点/图例无 Tab 停靠点、按钮/按压语义或 Reader 事件钩子<br>• 无主题属性时系统亮/暗、独立打开 SVG、宿主显式 dark 时打印均完整可辨；节点文字、线条、填充随主题变化，图间及对照 SVG 无样式污染<br>• 两字体策略各自的 SVG 字节数不超过阅读器 HTML 的 50%<br>• 只列一张截图的审核记录被拒；两张都写观察后 `deliveryStatus: complete`<br>• candidate 缺必填字段时不产出 SVG<br>• 同名阅读器 HTML 与其 summary 前后 SHA-256 相同 | 1. `"$E" finalize workflow $OUT/flow.json $OUT/flow.html --json`<br>2. `"$E" static workflow $OUT/flow.json $OUT/flow.svg --json`，读 `$OUT/flow.static-summary.json`<br>3. 写 `$OUT/host.html`（`<html data-theme="light">` 内联 `flow.svg`，无脚本），用 Chrome 打开看图；改为 `dark` 再看；对照第 1 步的 1440×900 截图<br>4. `wc -c $OUT/flow.svg $OUT/flow.html`<br>5. 打开两张 `flow.static.1440x900.*.png`；先用只列一张的记录运行 `"$E" review-record $OUT/flow.svg --record … --json`（应失败），再用完整记录登记<br>6. 删去 candidate 一个必填字段后运行第 2 步到新路径，确认退出非 0 且文件不存在<br>7. 第 2 步前后对 `flow.html`、`flow.finalize-summary.json` 取 `shasum -a 256`<br>8. 对 tree、erd 重复第 2、3 步；workflow/tree/erd 均以 `--font-policy embedded` 与 `--font-policy system` 写入不同路径检查，后者实际字体族/文字边界合约通过；另用 DevTools Emulation 模拟无主题属性的系统亮/暗与显式 dark 的 print，并单独打开 SVG。主题/打印变体只看现有场景，不增加待登记截图 | M1–M3 交付齐备、开发引擎构建后 |
| V-2 | 场景 E2E | **图解作者照常交付一张图和一份架构比较，审核只看两张与六张。** 输入：`minimal.erd.json`；作者按 `architecture-compare.md` 写的两份 architecture（head 比 base 多一个组件、改一个标签，连接带 `id`）。覆盖 WORK-F10/AC-1 至 AC-4。结果：<br>• 单图 summary 列两张 1440×900 截图；浏览器回执的 `coverage` 仍是四个桌面窗口加窄屏各亮、暗共 10 条<br>• 比较 summary 列六张；只写五张观察的记录被拒，六张齐全后完成<br>• 静态自动门未过与 SVG 改变分别保留原诊断码并提示重跑 `static`；HTML 对应失败仍提示 `finalize`<br>• 不看图直接读 summary，状态仍是 `required-pending` / `incomplete` | 1. `"$E" finalize erd skills/phoenix-diagram/examples/minimal.erd.json $OUT/erd.html --json`，读 summary 的 `screenshots` 与 `stages[2].receipt.coverage`<br>2. 看两张图，`review-record` 登记<br>3. `"$E" compare architecture $OUT/base.json $OUT/head.json $OUT/cmp.html --json`<br>4. `"$E" visual-check $OUT/cmp.html --out-dir $OUT/cmp-evidence --json`<br>5. 先用五张的记录登记（应失败），再看六张并登记<br>6. 扩展 `delivery` 已有自动门失败与产物变化用例，在公开 delivery 接口断言两个来源的恢复信息；场景中用真实 `review-record` 入口验证静态 SVG 变化路径，断言诊断消息中的恢复命令；静态 SVG 写出后修改再登记，应被拒绝并要求重跑 `static` | 与 V-1 同批 |
| V-3 | 场景 E2E | **文档作者按 summarize 的阅读面要求生成带图页面。** 角色：只读 `skills/elintp` 与 `skills/phoenix-diagram` 的 skill 文件及其引用文档，不读实现源码。输入：`docs/PRD-ELINTP-VISUALS.md`，附 summarize §3 的三条硬要求原文，页面需要 §2.3 主路径的流程图。覆盖 WORK-F5/AC-2、C-1。不调用 `report_submit`。结果：<br>• 终稿是单个 HTML；图是内联静态图，没有 `iframe`；产物使用 `system`，无 JetBrains / `@font-face` / 字体 data，计算字体族符合 D4，实际字体下文字无缺失/裁剪<br>• 断网打开无任何网络请求，图完整<br>• 切换页面亮暗，图随之切换；清除保存主题后模拟系统亮/暗，节点文字/线条/填充均可辨认，类型样式不污染其他图<br>• 360 px 宽下页面不横向滚动，图在自己的区域内横向滚动<br>• 显式 dark 下打印预览中图完整、为亮色，节点填充/描边也转亮色；SVG 内无 `tabindex`、按钮/按压语义或 Reader 事件钩子（滚动容器的键盘入口仍保留）<br>• 交付说明写明图的形态与实际做过的检查<br>常见变体与误操作：同一材料不带内联要求再生成一次，图为页内嵌入阅读器（默认不变）；把一份自绘 SVG 写到 `static:` 下，得到带行号的格式错误且旧页面未被覆盖 | 1. 按 `skills/elintp/SKILL.md` 执行，phoenix-diagram 的命令用 `$E`（launcher 只负责定位引擎）<br>2. 用 Chrome 打开终稿：DevTools Network 设为 Offline 后刷新，确认请求列表只有文件本身；点主题按钮；把窗口设为 360 px 宽；打开打印预览<br>3. `grep -c '<iframe' 终稿`（应为 0）、`grep -c 'data-phoenix-static' 终稿`（等于图数）<br>4. 两个变体各运行一次 `review.mjs` 或 `render.mjs`；用 DevTools 检查静态 SVG 内字体与语义属性，清除主题存储后分别模拟系统亮/暗，再于显式 dark 下查看 print | M4 交付齐备后（依赖 V-1 已通过的开发引擎） |
| V-4 | 局部 | 准入依据：分支多的 CSS/SVG 安全解析，以及既有溢出与恢复路径。<br>• 通过真实 `static` CLI 输出观察 §5.2 各类选择器/主题/媒体与动态类型样式的作用域；两字体策略不交叉覆盖，错误策略退出 2（`tests/delivery.rs`）；断言运行产物，不测试静态源码文字<br>• `static:`：安全的两策略产物通过并内联，保留 ID 与局部片段；无标记退出 2；在现有 SVG 安全用例加伪造标记的脚本/事件/foreignObject、外部资源、CSS URL 绕过及不合约 data MIME/位置，均退出 2 且旧页字节不变（`test/elintp/render.test.js`）<br>• 既有浏览器合同测试随数量更新：任一窗口溢出仍失败（WORK-F10/AC-2），截图数为 2 与 6；审核失败消息承接 V-2 | `mise exec --no-deps -- node --test test/elintp/render.test.js`<br>`$CARGO test $MF --test browser_contract --test delivery --test compare --test compare_browser` | 引擎部分在 M1–M3 批末；elintp 安全解析部分在 M4 批末 |
| V-5 | 阶段收口 | 全量回归与兼容（WORK-N3/AC-3、WORK-F9/AC-1、AC-3、C-3、C-4、C-7）：<br>• 十一类最小示例的阅读器 HTML 与基线 SHA-256 逐一相同<br>• 十一类在两字体策略下都能取得静态图，自动阶段全过，体积比均 ≤ 50%<br>• 基线引擎产出的旧交付（四张截图的 summary）用新引擎仍可按四张登记<br>• 现行文档中不再有旧数量的复述；skill 描述与引用测试通过；bundle 与源一致 | 1. `mise exec --no-deps -- npm run test:diagram`（fmt、clippy、全部 Rust / Node / Reader 测试；耗时未知）<br>2. `mise exec --no-deps -- node --test test/elintp/render.test.js test/elintp/review.test.js`<br>3. 最终代码上重新开发构建（新输出目录），对十一类各运行 `deliver` 与两策略的 `static`（分别输出），比对 SHA-256 与 `wc -c`，并逐类实际打开每个策略亮、暗两张静态截图（每件产物仍只登记两张）；复核类型节点/图例无失效交互语义，system 文字完整且无嵌入字体；对基线的 workflow 旧交付运行 `review-record`（四张）<br>4. `git grep -n -e '四张' -e '12 张' -e '2048×1320' -- skills/phoenix-diagram docs/phoenix-diagram.md README.md skills/documentation-writer skills/elintp`，逐条确认不是现行截图数量的复述；对照 PRD §5.3 的 C6、C11、C12 读一遍两个 skill 的 diff<br>5. `node scripts/build-phoenix-diagram.mjs --sync-bundle` 后 `git status --short dist/phoenix-diagram/bundle` 无未提交差异 | M5，代码改完后 |
| V-6 | 真实使用验收 | 从干净的用户状态连续完成三件事，亲自看产物（WORK-F9、WORK-F10、WORK-F5/AC-2 的本期部分）：<br>1. 新项目按文档离线安装后，图解作者用安装后的 skill 入口，为一份自己写的 sequence candidate 交付阅读器 HTML（看两张、登记）并取静态图（看两张、登记）<br>2. 文档作者在同一项目里用 elintp 把 `docs/phoenix-diagram.md` 讲成一页，明确要求零网络阅读面，页面带上一步的静态图<br>3. 读者断网打开终稿，切换亮暗，在窄屏下阅读<br>发现的用户可见问题修复后复验，或列给用户确认延期 | 1. 新建临时目录作项目，按 `docs/phoenix-diagram.md`「离线安装」：`node bin/xgent-skills.js install "$PROJECT" --no-phoenix-ui --providers=claude --diagram-release-dir="<V-5 第 3 步的开发构建目录>"`<br>2. `PD="$PROJECT/.claude/skills/phoenix-diagram/scripts/phoenix-diagram"`，只凭安装后的 `SKILL.md` 与 `references/` 操作<br>3. elintp 用仓内 `skills/elintp`；角色只读两个 skill 的文件与公开文档<br>4. 浏览器操作同 V-3 第 2 步 | M5，V-5 通过后 |

V-3、V-6 是 Agent 角色模拟，不代表真人可用性结论。门户报告阅读面的实际显示未覆盖（A-1）。

## 10. 里程碑与验收安排

M1–M3 同批：都在同一个开发引擎上验证，之间没有风险门，连续实施，批末构建一次、运行 V-1、V-2 与 V-4 的引擎部分。M4 一批。M5 收口。M4 的代码可在 §5.3 文件契约确定后与 M1–M3 并行编写（只写 `skills/elintp` 与 `test/elintp`，与引擎无共享文件）；它的 V-3 需要 V-1 通过后的开发引擎。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 引擎写出静态 SVG | 基线已记录（§9.1） | **落点**：§5.2 `render/static_style.rs` ★ 的统一样式策略；§5.3 `render.rs` `render_static`、`RenderContext` / `FontPolicy` 及六类 `TypeRenderer` 的属性/defs/body/legend 入口；§5.1 `cli.rs`；§5.4 第 1、2、4、5 步（`delivery.rs`、`finalize.rs`、`support.rs`）<br>**同构**：`finalize.rs` `finalize`；当前 defs、节点/图例属性生成点；`viewer/export.js` L275–L305 的选择器约定<br>**文档**：`references/static-figure.md` ★ 的命令、两字体策略与文件契约部分 | 与 M3 同批：V-1、V-4；回写「实施进度」 |
| M2 | 静态图的浏览器检查、截图与审核登记 | M1 的 `.svg` 输出 | **落点**：§5.4 `browser/static_figure.rs` ★、`static-probe.js` ★、`cdp.rs` `inspect_static` / `screenshot_static`、`evidence.rs` 主干命名、`delivery/review.rs` 恢复提示分派；实际输出策略下文字边界测量<br>**同构**：`browser/mod.rs` `check_browser`；`cdp.rs` `inspect`、`screenshot`<br>**文档**：`static-figure.md` 的看图与登记部分 | 与 M3 同批：V-1、V-2 恢复路径；回写「实施进度」 |
| M3 | 单图与架构比较审核减量，接口文字同步 | 无（与 M1、M2 改同一 crate，同一人顺序修改 `evidence.rs`、`mod.rs`） | **落点**：§5.5 三处代码与四个测试文件的数量断言；§5.7 全部文字；`--sync-bundle`<br>**同构**：各文件现有写法<br>**注意**：`SKILL.md` 描述改后跑 `skill.test.js` | 批末（M1–M3 交付齐备、开发引擎构建后）：V-1、V-2、V-4 的引擎部分；回写「实施进度」 |
| M4 | elintp 用静态图内联嵌图 | §5.3 文件契约；V-3 需 V-1 已通过 | **落点**：§6.1 `draft.mjs` 静态 SVG/CSS 安全解析；§6.2 `page.css`；§6.3 `draft-format.md`、`SKILL.md` 的字体分派/安全边界说明；`test/elintp/render.test.js`<br>**同构**：`draft.mjs` L244 的截图容器、L263–L277 的 `figure` 分支 | V-3、V-4 的 elintp 部分；回写「实施进度」 |
| M5 | 阶段收口 | M1–M4 完成 | 全量回归、兼容比对、接口复述核对、bundle 同步、真实使用验收；发现的问题修复后只复验失败项与波及项 | V-5、V-6；回写「实施进度」 |

提交点：M3 批末通过后提交一次（引擎、phoenix-diagram skill 与文档、bundle，覆盖 M1–M3）；M4 通过后提交一次（elintp）；M5 完成后最终提交。跨期需求 WORK-F5、WORK-F9、WORK-N3 在 M5 后仍停在已提交 / 可测试，终验在 P5。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1：真实门户内联静态 SVG 显示未实测 | 尚不能宣称 S4 在门户实际可用；不影响本地实现与离线验收，summarize 系统字体硬要求已在 D4 确定 | 用户下次真实 `/summarize` 提交后查看，不以本地通过代替门户证据 | 对外宣称 S4 实际可用之前 | 否 |
| D5：静态图不含摘要卡片与页面标题 | candidate 里写在卡片中的说明不会出现在静态图里，需由宿主页面正文承担 | 用户如要求自带卡片，另立需求 | — | 否 |
| `viewer.css` 的预设（signal-flow、blueprint、editorial）静态图未逐一看图 | 预设专属规则若有未遵守选择器约定的，静态图会缺该样式 | V-5 第 3 步对十一类最小示例取静态图；V-1 用作者 candidate。发现缺失时修共享 `static_style` 的规则处理，不能通过改 Reader 维护源破坏字节不变约束；做不到则按偏差处理 | M3 批末 | 否 |
| 开发构建目录能否直接用作离线安装的 release-dir | 影响 V-6 第 1 步的安装方式 | 构建脚本写出引擎、`VERSION.json`、`bundle/`、许可文件与 `build-receipt.json`（`scripts/build-phoenix-diagram.mjs` L436–L441），`--sync-dist` 使包内清单一致；若安装器拒绝，按其报错处理并记录，不绕过安装直接用 `$E` 冒充安装验收 | M5 | 否 |

- 最终状态：**Ready**。
- 定级理由：范围、文件契约、命令、检查与里程碑已定；修订路线基于当前共享渲染能力，旧实验仅支持体积可行性，新增策略的实现验证已分配至既有检查；前置期 P1、P2 已完成；A-1 仅为范围外阅读面实测，不改变本期任何 AC 的判定。

## 12. 已知坑与历史教训

- `assets/template.html` 是生成物，直接改会被 `--check` 判为过期；本期不应出现它的 diff，出现即说明误改了阅读器维护源（`tools/phoenix-diagram/viewer/README.md`）。
- `build.rs` 的 `collect` 遇到缺失输入会 panic，`include_str!` 指向不存在的文件时整个 crate 编译失败；静态样式直接读取已有维护源，不引用尚未生成的资产（[PHOENIX-DIAGRAM-STATECHART 计划](PHOENIX-DIAGRAM-STATECHART.md) §12）。
- `finalize` 的 stdout 约 600–730 KB，宿主直接读取会被截断；读 summary 文件，不读 stdout（[STATECHART 计划收口记录](PHOENIX-DIAGRAM-STATECHART.records/M6.md)「已知项」）。`static` 的回执同样带完整校验回执。
- 内联 SVG 共用页面的 ID 空间，未加前缀时 `#引用` 会指向别的图（[P1 计划](ELINTP-VISUALS-P1.md)完成记录中的评审修复 F-01）；静态图由引擎加前缀，elintp 不要再改写。
- `skill.test.js` 固定了描述长度、前 100 字的词与 `version: "0.2.0"`；改描述后先跑这个文件。
- [PHOENIX-DIAGRAM-P3 计划](PHOENIX-DIAGRAM-P3.md)的终验里程碑仍记为阻塞，其待取的宿主原件按四张截图的审核合同设计。本计划不处理它；它若恢复，需先按两张、六张的现行合同修订。
