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
> 调查基线：2026-11-12 · `53d35750c97f865c5b739ee467eccd686c653ba7` · clean。
>
> 需求来源：[elintp 模板化图文说明与 Phoenix Diagram 图解补充 PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P3（第 3 期）。本期承接 WORK-F9/AC-1 至 AC-4、WORK-F10/AC-1 至 AC-5、WORK-F5/AC-2、WORK-N3/AC-3；WORK-F5、WORK-F9、WORK-N3 的终验期是 P5，本期完成后这三条停在已提交 / 可测试。
>
> 本期交付：**phoenix-diagram 新增 `static` 命令，为十一类图型输出可内联的静态 SVG 并走亮、暗各一张的看图审核；单图审核截图由四张减为两张，架构比较由 12 张减为 6 张；elintp 在明确要求内联图时用静态图代替页内嵌入阅读器。**
> **开发范围：`tools/phoenix-diagram`（引擎与阅读器维护源的装配脚本）、`skills/phoenix-diagram`（随工具附带的 skill 及 `dist/phoenix-diagram/bundle` 副本）、`skills/elintp`（图来源与出图规则）、复述审核截图数的现行文档；依据见 §0。**
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

- 最近更新：尚未开始
- 当前进度：0/5 个里程碑完成
- 当前状态：尚未开始
- 最近完成：无
- 下一步：M1 · 装配脚本生成 `assets/static-figure.css`，引擎 `static` 命令写出静态 SVG；与 M2、M3 同批，批末运行 V-1、V-2、V-4
- 当前阻塞：无
- 代码基线：`53d35750`

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| — | — | — | 尚未开始任何里程碑 | — | — |

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
| WORK-F5/AC-2 | 功能 | PRD §3 WORK-F5 · 跨期，终验 P5 | 调用方或作者明确要求内联图时，页面每张图都是静态图，没有页内嵌入阅读器；summarize 的调用满足单文件、零外部资源、图为内联 | §6；V-3 | 已确认 |
| WORK-N3/AC-3 | 兼容 | PRD §3 WORK-N3 · 跨期，终验 P5 | 十一类有效输入、既有命令、已交付 HTML 保持有效；同一 candidate 的阅读器 HTML 字节不变 | §5.6；V-4、V-5 | 已确认 |
| C-1 | 约束 | PRD §7「已交付的部分持续遵守对应的质量门槛」 | elintp 的静态嵌图同样满足 WORK-N1（窄屏在自身区域横向滚动、打印可读）与 WORK-N2/AC-1（单文件、零网络） | §6.2；V-3 | 已确认 |
| C-2 | 约束 | [AGENTS.md](../../AGENTS.md) `@skill_tool_independence` | elintp 不以 phoenix-diagram 的安装或引擎为执行前提；`static` 不可用时走既有降级（自绘 `svg:` 并说明） | §6.3 | 已确认 |
| C-3 | 约束 | AGENTS.md §7；PRD §5.3 的 C6、C11、C12 | 修改两个 skill 前后按整合清单核对：elintp 触发与授权不变、默认审核与完整交付含义不变、独立 HTML 不变 | §6.3、§5.7；V-5 第 4 步 | 已确认 |
| C-4 | 约束 | AGENTS.md「Working in This Repository」接口复述条款 | 截图数量与新命令的全部现行复述在同一改动内更新，历史记录不改 | §1.2 文档清单；V-5 第 4 步 | 已确认 |
| C-5 | 约束 | AGENTS.md §6；`test/phoenix-diagram/skill.test.js` L15–L27 | phoenix-diagram 描述不超过 200 字，前 100 字保留现有图型与排除词，`version: "0.2.0"` 不变；elintp 描述不改 | §5.7 | 已确认 |
| C-6 | 约束 | AGENTS.md「Working in This Repository」；`scripts/test-phoenix-diagram-native.mjs` | 引擎检查用 mise 固定的 Rust 与 `mbx`，离线、锁定依赖；工具缺失即失败，不跳过 | §9 环境 | 已确认 |
| C-7 | 约束 | [docs/phoenix-ui.md](../phoenix-ui.md) L253 | 改过 `skills/phoenix-diagram` 后运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle` 并提交 bundle；`VERSION.json`、`ENGINE.json` 不提交 | M3；V-5 | 已确认 |
| C-8 | 约束 | AGENTS.md `@simplicity`、`@failure_handling` | 工具保持直接实现：整文件覆盖写出，不加事务、备份、并发检查或回滚 | §5.4、§8 | 已确认 |
| C-9 | 约束 | PRD §4「既有决策约束」 | 旧 PRD 的 D5 与 WORK-N1/AC-2 保持：自动检查通过不等于审核通过 | §5.4、§5.5 | 已确认 |
| A-1 | 假设 | [summarize](../../skills/summarize/SKILL.md) §3 写「字体用系统字体栈」；SPMS 报告阅读面未实测 | 阅读面接受页内 `data:` 字体；若被剥离，静态图回退到样式里的等宽系统字体栈，文字仍显示 | 不阻塞。验证办法：用户下一次真实 `/summarize` 提交后在门户查看；结果不同则按 D4 的重新评估条件处理。责任人：用户 | 开放 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 静态图由谁生成 | 引擎在 Rust 端生成，新增 `static` 命令 | 不需要浏览器即可写出文件；阅读器渲染路径零改动 | ADR-1 |
| D2 | 静态图的样式来源 | 装配脚本从 `viewer/viewer.css` 按选择器约定生成 `assets/static-figure.css`，Rust 原样嵌入 | `viewer.css`、`template.html` 不改，阅读器 HTML 字节不变；样式只有一份维护源 | ADR-1；`viewer/export.js` L275–L305 已有同一约定 |
| D3 | 怎样跟随宿主亮暗 | 宿主 `<html data-theme="light\|dark">` 优先；没有该属性时跟随 `prefers-color-scheme`；打印为亮色 | 与 elintp 页面（`skills/elintp/assets/page.css` L14–L30、L134）和阅读器的约定一致；宿主不需要脚本 | ADR-2 |
| D4 | 字体 | 嵌入与阅读器相同的六段 JetBrains Mono 子集（约 95 KB） | 文字宽度与布局测量一致；每张静态图多约 95 KB，仍为阅读器的 16–18%。代价：一页多图时字体重复。重新评估：单页静态图的字体重复成为体积问题，或 A-1 证实阅读面剥离 `data:` 字体 | WORK-F9/AC-1、WORK-N2/AC-1「字体……都在文件内」；`viewer/export.js` L340–L342 的导出也嵌同一份字体 |
| D5 | 静态图包含什么 | 只含阅读器的主图 SVG（节点、关系、标签、图内图例）；不含页面标题、摘要卡片、节点索引与任何控件；树的折叠按钮隐藏 | 标题与说明由宿主页面提供；`<title>`、`<desc>` 仍在 SVG 内。重新评估：用户要求静态图自带摘要卡片 | PRD WORK-F9 边界；阅读器的完整 SVG 导出同样只导出主图（`viewer/export.js` L246–L247） |
| D6 | 多张静态图同页的 ID 冲突 | 引擎用 candidate 摘要前 8 位给全部元素 ID 加前缀 `pd<8hex>-` | 宿主不必改写静态图；同一 candidate 放两次时 ID 相同但定义相同 | 复用 `render.rs` `namespace_svg`（L670） |
| D7 | 静态图的交付与审核记录 | 复用 `FinalizeReceipt` 结构，summary 写在 SVG 旁的 `NAME.static-summary.json`；`review-record` 按 `.svg` 后缀找到它 | 登记规则与单图逐字相同，不新增审核实现 | WORK-F10/AC-5；`delivery/review.rs` |
| D8 | 静态图的自动检查 | 引擎把静态图内联进一份无脚本宿主页，在 1440×900 亮、暗各测一次并截图 | 只证明能独立显示、随主题切换、无外部请求；文字与几何的完整检查已在同一 candidate 的校验阶段完成 | WORK-F9/AC-1、AC-2；§5.4 |
| D9 | 审核减量的实现 | 只改截图采集条件与数量断言；测量覆盖、登记校验不改；已有 summary 不迁移 | 旧 summary 列四张仍按四张登记 | WORK-F10/AC-1 至 AC-4；WORK-N3/AC-3 |
| D10 | elintp 怎样接收静态图 | `:::figure` 新增 `static:` 来源，校验根元素带 `data-phoenix-static` 后原样内联 | 不经自绘 SVG 白名单（静态图必然含 `<style>`）；自绘图仍只能走 `svg:` | §6.1；与 `reader:` 同为工具交付物 |
| D11 | 工具版本号 | 保持 0.2.0 | statechart 新增图型时也未改版本（`6bb21b6b`）；npm 版本在发布时定 | `Cargo.toml`、`skill.test.js` L25 |
| D12 | elintp 何时改用静态图 | 仅调用方或作者明确要求内联图或零网络阅读面时；默认仍嵌阅读器 | 不改变 P1 的默认行为 | PRD D4、WORK-F5 行为 |

### 0.3 ADR-lite

#### ADR-1：静态图在 Rust 端生成，样式由装配脚本从阅读器样式派生

- 状态：Accepted
- 背景与驱动：WORK-F9/AC-1 要求图面与阅读器一致，WORK-N3/AC-3 要求阅读器 HTML 不变，WORK-F9/AC-4 要求只依赖输入校验。阅读器的主图 SVG 已由 `render.rs` `render_svg` 在 Rust 端完整生成，着色靠 `viewer.css` 里的主题变量与 SVG 语义类。
- 备选：
  - A. Rust 端输出 `render_svg` 的结果，加一份构建期从 `viewer.css` 派生的样式。
  - B. 用浏览器打开阅读器 HTML，调用其 SVG 导出（`serializeSvg`）取结果。导出逻辑现成，但取图需要浏览器，结果随浏览器的 CSS 序列化变化，且导出样式用 `:root` 定义变量，内联进宿主会覆盖宿主的同名变量，仍要再加工。
  - C. 手写一份静态图专用样式。与 `viewer.css` 重复约 54 KB 规则，两处会漂移。
- 决策：A。实验（本基线引擎对十一类最小示例）：阅读器 HTML 967–989 KB，静态图 157–177 KB，占 16.3%–17.9%；erd、tree、workflow 的静态图在无脚本宿主页亮、暗两种主题下与阅读器图面一致。
- 正面后果：取图不依赖浏览器，输出确定；`viewer.css` 与模板不改；样式规则只有一份维护源，`--check` 能发现派生物过期。
- 负面/中性后果：装配脚本多一个选择器改写函数；阅读器新增的 SVG 规则必须继续遵守「以 `svg` 或语义类开头」的约定才会进入静态图，这与现有 SVG 导出的要求相同。
- 重新评估触发：`viewer.css` 改为预处理器或嵌套语法，使顶层规则切分失效；或静态图需要阅读器运行时才有的图元。

#### ADR-2：静态图用宿主 `data-theme` 与系统偏好决定亮暗

- 状态：Accepted
- 背景与驱动：WORK-F9/AC-2。静态图没有脚本，只能靠 CSS 感知宿主主题。
- 备选：
  - A. 选择器上下文：`:root[data-theme]` 优先，否则 `prefers-color-scheme`。
  - B. 变量值写成 `light-dark()`，跟随宿主的 `color-scheme`。只对颜色值有效；阅读器有按主题切换的非颜色声明（例如 classic 亮色两层、暗色一层的节点阴影，`viewer.css` L3978–L3985），仍需选择器上下文，两套机制并存。
- 决策：A。elintp 页面、阅读器本身都用 `<html data-theme>`；没有该属性的宿主按系统偏好显示。
- 正面后果：一套机制覆盖变量与非变量声明；与本仓两个宿主一致。
- 负面/中性后果：每条按主题区分的规则输出多份选择器；主题属性不在 `<html>` 上的宿主只能跟随系统偏好。
- 重新评估触发：出现用其他属性或类名表示主题、且不能改的宿主。

### 0.4 职责与事实所有权

|  | phoenix-diagram 引擎与 skill | elintp | summarize（范围外） |
| --- | --- | --- | --- |
| 拥有 | 静态图的内容、样式、ID 前缀、自动检查、截图与审核登记；阅读器 HTML | 何时用静态图；把静态图放进页面；页面的窄屏、打印与最终页自检 | 对 elintp 提出阅读面要求；提交报告 |
| 不拥有 | 宿主页面的版式与主题切换 | **静态图的校验与审核；不改写静态图内容** | elintp 的嵌图实现 |

一句话：静态图画得对不对归 phoenix-diagram，页面里放得对不对归 elintp；`static` 失败时引擎只报错，elintp 按既有规则降级并如实说明。

### 0.5 明确不在本期

- **数值图表、变更视图、逐词标注及其静态图**——P4、P5（WORK-F9/AC-5、AC-6，WORK-F5/AC-3、AC-6）。
- **架构比较的静态图**——比较不是图型，PRD 未要求；`static` 只接受十一类单图。
- **按实际用到的字符裁剪字体、多图共用一份字体**——后续可选；本期按 D4 整段嵌入。
- **summarize skill 的文字与调用方式**——不改（WORK-N3/AC-2）；它写的「字体用系统字体栈」见 A-1。
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

**引擎与装配（`tools/phoenix-diagram`、`scripts/`）**

- `scripts/assemble-phoenix-diagram-reader.mjs`：新增 `staticFigureCss` 并写出、核对第三个生成物。
- `tools/phoenix-diagram/assets/static-figure.css` ★：生成物，不手改。
- `tools/phoenix-diagram/src/render.rs`：新增 `render_static`。
- `tools/phoenix-diagram/src/delivery.rs`、`delivery/finalize.rs`、`delivery/support.rs`：`static_figure` 入口、`summary_path` 认 `.svg`。
- `tools/phoenix-diagram/src/browser/static_figure.rs` ★、`static-probe.js` ★、`browser/mod.rs`、`browser/cdp.rs`、`browser/evidence.rs`、`browser/comparison.rs`：静态图检查与截图；采集条件与数量。
- `tools/phoenix-diagram/src/cli.rs`：`static` 命令、帮助文本。
- `tools/phoenix-diagram/tests/{browser_contract,delivery,compare,compare_browser}.rs`：数量断言随行为更新。
- `test/phoenix-diagram/reader-assets.test.js`：选择器改写的用例（V-4）。
- 不改：`viewer/viewer.css`、`viewer/*.js`、`viewer/template.source.html`、`assets/template.html`、`skills/phoenix-diagram/schemas`、各类型布局与检查。

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
| `staticFigureCss`（装配脚本） | 装配脚本 `main` | 输入 `viewer.css` 文本与字体样式块，输出一段 CSS 文本；同输入同输出 | 无 | 选择器改写与主题上下文展开（§5.2） | 进程内 | 生成物内容；V-4 用例 |
| `render::render_static` | `delivery::static_figure` | 输入已编译场景与 ID 前缀，输出 SVG 字节；主图内容与阅读器 SVG 插槽相同 | 无 | 根属性、样式与底色矩形的插入位置 | 进程内 | 写出的 `.svg` |
| `delivery::static_figure` | `cli.rs` | 校验 → 写出 → 浏览器检查，返回并落盘 `FinalizeReceipt` 形态的 summary | 无 | 阶段顺序与失败语义（§5.4） | 本地文件系统 | 回执、summary、文件 |
| `browser::check_static` | `delivery::static_figure` | 输入 `.svg` 路径，返回 `BrowserReceipt`（2 条覆盖、至多 2 张截图） | 无 | 宿主页生成、主题切换、离线判定 | 本机 Chrome（既有发现逻辑） | 回执与截图 |
| elintp `static:` 来源 | `parseDraft` 的 `figure` 分支 | 文件须是带标记的静态图；原样内联 | 无 | 无 | 本地文件 | 生成的页面 |

不新增接缝或适配器：每个模块只有一个调用者与一个实现。删除测试：去掉 `staticFigureCss`，选择器改写会散进 Rust 或手写样式，故保留；去掉 `browser::check_static`，宿主页与两主题的采图逻辑会塞进 `check_browser` 的十次循环，故单列。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| elintp → phoenix-diagram 静态图文件 | 已核实缺口 | elintp 现有来源只有 `reader:`、`svg:`（`draft.mjs` L265–L267） | 范围内：§5.3 定义文件契约，§6.1 新增来源 | 文件读不了或没有标记是内容稿格式错误（退出 2），给行号与改法 |
| elintp skill → phoenix-diagram skill | 已核实足够 | elintp 自行查找并读取 `phoenix-diagram/SKILL.md`，不自动安装（`skills/elintp/SKILL.md`「出图」） | 范围内：两处文字同步静态图流程 | 未安装、引擎没有 `static` 或流程失败 → 自绘 `svg:` 并在交付说明写明原因 |
| summarize → elintp | 已核实足够 | summarize §3 以文字提出「单文件、不引用外部资源、图用内联 SVG」 | 零改动；elintp 把这类文字识别为明确的内联要求 | — |
| SPMS 报告阅读面 | 未核实，不阻塞 | 仅有 summarize §3 的描述；本期不提交报告 | 范围外。提供方：SPMS 门户。A-1 记录未核实项与回退行为 | 见 A-1 |
| documentation-writer skill | 已核实足够 | L151 复述「四张暗亮截图」，L161 从 summary 取亮色截图 | 只改 L151 的数量文字；流程不变 | — |
| 本机 Chrome | 已核实足够 | `browser::discovery::find`；缺失时回执 `unavailable`、退出 5 | 复用 | 静态图已写出，summary 记 `unavailable`，不能登记审核 |

登记位置：新命令在 `cli.rs` 的 `HELP` 与命令匹配表；新诊断码 `static/output-extension`；新来源在 `draft-format.md` 的图来源一节与「常见报错」表。没有新的配置、环境变量或回执 schema 版本。

## 5. 核心机制（引擎）

### 5.1 `static` 命令

```text
static <type> <candidate.json> <output.svg> [--quality standard|showcase] [--repo-root DIR] [--brand-cache DIR] [--out-dir DIR] --json
```

- 参数与 `finalize` 相同；`<type>` 是十一类之一，其他值按现有逻辑报 `Unsupported diagram type`。
- `<output.svg>` 必须以 `.svg` 结尾，否则退出 2、诊断码 `static/output-extension`、不写任何文件。这是唯一新增校验：summary 与截图的命名、`review-record` 的分流都依赖后缀。
- stdout 是一个 JSON（`FinalizeReceipt`，`command: "static"`），同时写入 SVG 旁的 `NAME.static-summary.json`。下文 `NAME` 都指输出文件名去掉 `.svg` 后的部分。
- `cli.rs` 落点：`dispatch` 的命令白名单（L364–L376）与 `validate | deliver | finalize` 分支（L460–L518）并入 `static`，选项集合同 `finalize`；`HELP` 增加一行，`review-record` 一行改为 `review-record <output.html|output.svg> --record <review.json> --json`。

### 5.2 `assets/static-figure.css` 的生成

装配脚本新增 `export function staticFigureCss(viewerCss, fontsCss)`，`main` 在写 `template.html` 的同一处写出 `assets/static-figure.css`，`--check` 同样核对它。`fontsCss` 取 `template.source.html` 中 `<style id="phoenix-diagram-fonts">` 的文本。同构实现：本脚本现有的片段读取与 `--check` 比对；选择器约定照 `viewer/export.js` L275–L305。

记 `ROOT` = `svg[data-phoenix-static]`。

1. **切分**：去掉注释后按顶层花括号切出规则；跳过全部 at 规则（`@media`、`@keyframes`、`@supports`、`@font-face`）。
2. **逐个选择器分支改写**（按顶层逗号拆分，括号内的逗号不拆）：

   | 分支形态 | 改写结果 | 主题 |
   | --- | --- | --- |
   | 前缀 `html` 或 `:root`（可省略）后接若干 `[data-preset="P"]`、`[data-theme="T"]`，再接空格与剩余部分；或没有前缀 | 剩余部分按下三行处理；有 `P` 时 `ROOT` 后追加 `[data-preset="P"]` | 有 `T` 则为该主题，否则无 |
   | 剩余部分以 `svg` 开头 | `ROOT` + 去掉 `svg` 后的部分 | 同上 |
   | 剩余部分以 `.diagram-container > svg` 开头 | `ROOT` + 其后的部分 | 同上 |
   | 剩余部分以 `.c-`、`.t-`、`.a-`、`.m-` 开头 | `ROOT` + 空格 + 剩余部分 | 同上 |
   | 没有剩余部分（变量块，如 `[data-theme="light"]`、`[data-preset="blueprint"][data-theme="dark"]`） | `ROOT`（有 `P` 时带预设属性） | 同上 |
   | 只有 `:root`，且同一规则另有带主题的分支（`:root, [data-theme="dark"]`） | 丢弃该分支 | — |
   | 其余（前缀含其他属性或伪类，如 `html[data-motion=…]`、`html[data-preset="classic"]:not(…)`；剩余部分是页面元素） | 丢弃该分支 | — |

   一条规则的全部分支都被丢弃时丢弃整条规则。声明块原样保留。

3. **按主题分组输出**，`S` 表示改写后的选择器：
   - 无主题：`S { … }`。
   - 亮色，三处：
     - `@media print { S { … } }`
     - `:root[data-theme="light"] S { … }`
     - `@media not (prefers-color-scheme: dark) { :root:not([data-theme="dark"]) S, S:root { … } }`（`S:root` 指把 `S` 开头的 `ROOT` 换成 `ROOT:root`，供单独打开 `.svg` 文件时使用）
   - 暗色，两处：
     - `@media screen { :root[data-theme="dark"] S { … } }`
     - `@media screen and (prefers-color-scheme: dark) { :root:not([data-theme="light"]) S, S:root { … } }`
4. **文件内容顺序**：字体样式块 → 字体族规则 → 无主题规则 → 亮色组 → 暗色组 → 固定规则。
   - 字体族规则：`ROOT { font-family: … }`，字体栈照 `viewer/export.js` L363。
   - 固定规则两条：`ROOT .c-bg-rect { fill: var(--bg); }`；`ROOT [data-tree-toggle], ROOT [data-tree-expand-all] { display: none; }`。

本基线的实验值，供核对：对当前 `viewer.css`，保留 254 个选择器分支、丢弃 810 个；无主题规则 177 条，亮、暗各 6 条；生成物约 150 KB，其中字体 95.6 KB。

### 5.3 静态图文件的内容

`render.rs` 新增：

```rust
/// The diagram's SVG for inlining into another page: no reader, no script.
pub fn render_static(scene: &CompiledDiagram, id_prefix: &str) -> Vec<u8>
```

步骤：

1. `let svg = namespace_svg(&render_svg(scene), id_prefix);`
2. 在根 `<svg` 标签内追加 `data-phoenix-static=""`、`width="<viewBox 宽>"`、`height="<viewBox 高>"`（用 `geometry::format_number`）。
3. 紧接 `</desc>` 之后插入 `<style>` + `include_str!("../assets/static-figure.css")` + `</style>`，再插入 `<rect class="c-bg-rect" x y width height/>`，四个值取 `scene.view_box`。
4. 不输出 XML 声明；UTF-8。

文件契约（elintp 与其他宿主依赖的全部内容）：

- 根元素是带 `data-phoenix-static` 属性的 `<svg>`，有 `viewBox`、`width`、`height`，保留 `data-diagram-type`、`data-preset`、`role="img"`、`<title>`、`<desc>`。
- 没有 `<script>`、事件属性、`foreignObject`；资源只有文件内的 `data:` 字体与品牌图。
- 样式只作用于 `svg[data-phoenix-static]` 之内；除 `@font-face` 声明的 `JetBrains Mono` 字体族外不影响宿主。
- 亮暗按 ADR-2；宿主不设主题属性时跟随系统偏好。
- 没有交互：无搜索、缩放、Route / Reach、播放与导出；树全部展开且不显示折叠按钮。
- ID 前缀：`pd` + candidate 的 SHA-256 前 8 位十六进制 + `-`，取自 `ValidationReceipt.candidate.identity.sha256`。

### 5.4 静态图的交付流程

`delivery.rs` 新增 `pub fn static_figure(request, output, browser) -> FinalizeReceipt`，实现放 `delivery/finalize.rs`，照 `finalize`（L79–L157）写，差异如下：

1. **validate**：`prepare::prepare(request)`，与 `finalize` 相同。未通过时后两个阶段 `not-run`，不写 SVG；已存在的同名旧文件保持原样（与 `deliver` 一致）。
2. **static**（阶段名 `static`，取代 `deliver`）：`render::render_static(&compiled.scene, &prefix)`，用 `support::write` 写到 `output`；`summary.artifact` 记录该 SVG 的路径与身份。输出路径与 candidate 是同一文件时沿用 `output/input-alias` 失败。
3. **browser-check**：`browser::check_static(output, &options)`，`options.capture = true`。
4. summary 写到 `summary_path(output)`。`support::summary_path` 增加：文件名以 `.svg` 结尾时返回 `NAME.static-summary.json`。
5. 状态字段语义与 `finalize` 相同：三阶段通过后 `automatedStatus: "pass"`、`visualReview.status: "required-pending"`、`deliveryStatus: "incomplete"`；浏览器不可用时 `status: "unavailable"`，SVG 已写出。

`review-record figure.svg --record r.json`：`cli.rs` 现有分流不改（`.svg` 不是比较产物，落到 `delivery::record_review`）；`review::record` 经 `summary_path` 自动读到静态 summary，「自动门已通过、文件未变、记录逐一列出当前截图」三项校验原样生效（WORK-F10/AC-4、AC-5）。

`static` 只写 `.svg`、`NAME.static-summary.json` 与 `NAME.static.1440x900.{light,dark}.png`，不读写同名 `.html`、`.finalize-summary.json` 及其截图（WORK-F9/AC-4）。

**`browser::check_static`**（`src/browser/static_figure.rs` ★，在 `mod.rs` 以 `pub use` 导出）：

- 启动、发现、版本、关闭、失败回执照 `check_browser`（`mod.rs` L200–L379）；返回同一 `BrowserReceipt` 类型，`command: "browser-check"`。
- 宿主页：把 `.svg` 全文放进下面的 HTML，写到截图目录下的 `NAME.static-host.html`，检查结束后删除。

  ```html
  <!doctype html><html lang="en" data-theme="THEME"><head><meta charset="utf-8"><title>static figure</title>
  <style>body{margin:24px}svg{display:block;max-width:100%;height:auto}</style></head><body>SVG</body></html>
  ```

- 对 `Theme::Light`、`Theme::Dark` 各一次：把宿主页的 `THEME` 写成对应值，按 `DESKTOP_VIEWPORTS[0]` 导航并执行 `static-probe.js` ★，取得度量后截图。`cdp.rs` 增加 `inspect_static`（照 `inspect` L661–L671，换探针）与 `screenshot_static`（照 `screenshot` L718，稳定条件改为等待 `document.fonts.ready` 与两帧）。
- 探针返回：`documentUrl`、`innerWidth`、`scrollWidth`、`scripts`（`document.scripts.length`）、`figures`（`svg[data-phoenix-static]` 的个数）、`bounds`（该 SVG 的 `getBoundingClientRect`）、`background`（`.c-bg-rect` 的计算 `fill`）、`fontsStatus`、`resources`（`performance.getEntriesByType('resource')` 的名称）。
- 每个主题四项检查，名称沿用 `BROWSER_MEASUREMENT_CHECKS` 中已有的：

  | 检查 | 通过条件 |
  | --- | --- |
  | `rendered-dom` | `figures == 1`，`scripts == 0`，`bounds` 宽高大于 0 |
  | `viewport-containment` | `scrollWidth <= innerWidth` |
  | `offline-resources` | 事件流中没有 `observation::unexpected_resource` 判为意外的请求，没有 `Network.loadingFailed`，`fontsStatus == "loaded"` |
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

- 阅读器 HTML：`render_with_layers`、`template.html`、`viewer/**` 不改，同一 candidate 的 HTML 字节不变。V-5 用改动前后的 SHA-256 对比证明。
- 既有命令的参数、回执字段与退出码不变；回执里只有 `screenshots` 的条数与 `screenshot-capture` 的 `scope` 数值变化。
- `build.rs` 的 `INPUTS` 已包含 `assets` 目录，新生成物自动计入 `sourceInputSha256`，不需要改 `build.rs`。

### 5.7 phoenix-diagram skill 与文档的文字

- `SKILL.md`：
  - 描述：把「齐备后交付经 Rust、浏览器与实际看图核对的独立 HTML」改为「齐备后交付经 Rust、浏览器与实际看图核对的独立 HTML，可另取可内联的静态 SVG」；其余不动。改后读全文与前 100 字，确认不超过 200 字（当前 151 字）。
  - 「以下能力按用户请求分别启用」列表增加一项：明确要求可内联的静态图时读 `references/static-figure.md`。
  - 「最短创作路径」第 4 步的「四张暗/亮截图」改为「1440×900 亮、暗两张截图」。
- `references/static-figure.md` ★：何时取静态图；命令；产物三类文件；宿主约定（ADR-2 与 §5.3 文件契约）；看两张截图、写审核记录、`review-record figure.svg`；完成条件；与阅读器交付相互独立；`static` 不存在或失败时如实报告。
- `references/delivery-contract.md`：首段与「浏览器与实际看图」「审核记录」「架构比较的收口」中的四张、12 张改为两张、六张，并写明截图为 1440×900；末尾加一句指向 `static-figure.md`。
- `references/architecture-compare.md` L34、L40：12 张改为 6 张（每个视图 1440×900 亮、暗各一）。
- `references/preview.md` L43：「四张截图」改为「两张截图」。
- `references/viewer-runtime.md`：加一句区分阅读器内的 SVG 导出（读者触发）与作者命令 `static`。
- `docs/phoenix-diagram.md`：首段能力列表加静态图；「公开命令」加 `static` 一行并更新 `review-record`；「看图后收口」「按请求启用的作者工具」中的四张、12 张按上面口径改；新增一小节「静态图」摘要并链接 `static-figure.md`。
- `README.md` L216：phoenix-diagram 一行加「可内联静态图」。
- `skills/documentation-writer/SKILL.md` L151：「四张暗亮截图」改为「两张暗亮截图」。
- 完成后运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle`（C-7）。

## 6. elintp 嵌图

### 6.1 `static:` 来源（`skills/elintp/scripts/draft.mjs`）

- `figure` 分支（L263–L277）的来源匹配由 `reader|svg` 扩为 `reader|svg|static`；错误提示的改法文字加上 `static: 路径.svg`。
- `static` 分支：
  1. 去掉文件开头可能存在的 BOM、XML 声明与空白。
  2. 用 `/^<svg\b[^>]*\sdata-phoenix-static(=|\s|>)/` 核对根标签；不匹配时报格式错误：问题「不是 phoenix-diagram 静态图」，改法「用 phoenix-diagram 的 static 命令生成；自绘图用 svg:」。
  3. 原样内联：`<div class="image-scroll" role="region" tabindex="0" aria-label="标题">` + 文件内容 + `</div>`。容器照 L244 的截图写法。不改写 ID（D6），不经 `inlineSVG`。
- 取舍：只核对标记，不重新解析内容。触发：作者把自绘 SVG 写到 `static:` 下；后果：绕过自绘白名单。静态图本身由引擎保证无脚本（§5.3），与 `reader:` 一样按工具交付物对待。
- 图注、`<figure class="diagram">` 外壳、资源快照与审阅标注沿用现有代码，不分支。

### 6.2 页面样式（`skills/elintp/assets/page.css`）

- 窄屏媒体块（现有 `figure img { max-width: none; }` 旁，L131）加：`figure svg[data-phoenix-static] { width: auto; max-width: none; }`。静态图按自身 `width`、`height` 显示，在 `.image-scroll` 内横向滚动。
- 桌面沿用 L111 的 `figure svg`（铺满宽度、等比缩放）；打印块（L141 旁）加 `figure svg[data-phoenix-static] { width: 100%; max-width: 100%; }`。
- 主题不需要任何样式：静态图自己跟随 `:root[data-theme]` 与系统偏好，打印为亮色。

### 6.3 skill 文字

- `references/draft-format.md`：图来源示例加一段 `static:`；说明句改为「首个非空行只能是一个 `reader:`、`svg:` 或 `static:` 来源」；写明 `static:` 只引用 phoenix-diagram `static` 命令的产物、原样内联、跟随页面亮暗、不修改源文件、不替图工具做审核；「常见报错」加一行「不是 phoenix-diagram 静态图」。
- `SKILL.md`「出图」一节：
  - 新增一段：调用方或作者明确要求内联图、内联 SVG 或零网络阅读面时，页面里每张图都用 phoenix-diagram 的静态图，以 `:::figure` 的 `static:` 引用，不用 `reader:`；静态图的生成、看图与审核登记按 phoenix-diagram 的流程，这里不复述。没有这类要求时默认仍用 `reader:`。
  - 降级句补一种情形：已安装的 phoenix-diagram 没有静态图命令或按其流程无法交付时，同样改用自绘 `svg:` 并在交付说明写明原因。
  - 「合并后仍是一个离线 HTML」一段补一句 `static:` 的嵌入方式。
- 「交付前自检」第 9 项与交付说明的「图由 phoenix-diagram 交付还是自绘」一项补充图的形态（页内嵌入阅读器 / 静态图 / 自绘）。
- 不改：描述、触发、语言选择、红线、写作准则、转述硬规则、模板与审阅流程（C-3，PRD §5.3 的 C6、C7）。

### 6.4 整合清单在本期的落点

PRD §5.3 的清单是基线；本期只触及其中三项。实施 diff 逐项对照，出现清单外的行为变化先停下修订本节。

| PRD 清单项 | 目标文件 | 现有行为 | 本期改动 | 对原则、触发与授权的影响 | 核对 |
| --- | --- | --- | --- | --- | --- |
| C6 · elintp「出图」 | `skills/elintp/SKILL.md`、`references/draft-format.md` | 已安装就调用 phoenix-diagram，以 `reader:` 嵌阅读器；否则自绘 | 调整：明确要求内联时改用 `static:`；默认与降级规则不变 | 不自动安装、不新增检测脚本、触发与描述不变 | V-3；V-5 第 4 步读 diff |
| C11 · 交付合同的审核截图 | `skills/phoenix-diagram/references/delivery-contract.md`、`architecture-compare.md` 及 §5.7 所列复述 | 单图四张，比较 12 张 | 调整为两张、六张 | 默认审核与「三门加看图才算完整交付」不变 | V-2；V-5 第 4 步 |
| C12 · 输出形态 | `skills/phoenix-diagram/SKILL.md`、`references/static-figure.md` ★ | 只有独立 HTML | 新增静态图；独立 HTML 及其流程不变 | 静态图同样默认要求看图审核；不扩大自动触发 | V-1；V-5 第 3 步 |

## 8. 失败模式、发布与回滚

不引入新的运行期失败模式：`static` 是单次本地命令，整文件覆盖写出（`output::write` 先写临时文件再改名），失败时返回带阶段的诊断，由作者重跑；没有并发写、后台任务或跨资源提交。浏览器不可用沿用既有 `unavailable` 语义。

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
- 装配：`mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs`，核对用 `--check`。
- Node 单文件：`mise exec --no-deps -- node --test <文件>`。
- 整 crate 测试、clippy 与开发构建只在批末或收口运行。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **图解作者取一张能放进别的文档的图，并完成审核。** 输入：作者按本计划 §5.4 的交付流程写的 workflow candidate，另加 `minimal.tree.json`、`minimal.erd.json`。覆盖 WORK-F9/AC-1 至 AC-4、WORK-F10/AC-4、AC-5。结果：<br>• 得到 `.svg`、两张截图与 summary，自动阶段全过、审核待办<br>• 把 SVG 放进空白无脚本 HTML，亮、暗下节点、关系、标签、图例齐全，与同一 candidate 的阅读器截图一致；树全部展开且无折叠按钮<br>• SVG 字节数不超过阅读器 HTML 的 50%<br>• 只列一张截图的审核记录被拒；两张都写观察后 `deliveryStatus: complete`<br>• candidate 缺必填字段时不产出 SVG<br>• 同名阅读器 HTML 与其 summary 前后 SHA-256 相同 | 1. `"$E" finalize workflow $OUT/flow.json $OUT/flow.html --json`<br>2. `"$E" static workflow $OUT/flow.json $OUT/flow.svg --json`，读 `$OUT/flow.static-summary.json`<br>3. 写 `$OUT/host.html`（`<html data-theme="light">` 内联 `flow.svg`，无脚本），用 Chrome 打开看图；改为 `dark` 再看；对照第 1 步的 1440×900 截图<br>4. `wc -c $OUT/flow.svg $OUT/flow.html`<br>5. 打开两张 `flow.static.1440x900.*.png`；先用只列一张的记录运行 `"$E" review-record $OUT/flow.svg --record … --json`（应失败），再用完整记录登记<br>6. 删去 candidate 一个必填字段后运行第 2 步到新路径，确认退出非 0 且文件不存在<br>7. 第 2 步前后对 `flow.html`、`flow.finalize-summary.json` 取 `shasum -a 256`<br>8. 对 tree、erd 重复第 2、3 步 | M1–M3 交付齐备、开发引擎构建后 |
| V-2 | 场景 E2E | **图解作者照常交付一张图和一份架构比较，审核只看两张与六张。** 输入：`minimal.erd.json`；作者按 `architecture-compare.md` 写的两份 architecture（head 比 base 多一个组件、改一个标签，连接带 `id`）。覆盖 WORK-F10/AC-1 至 AC-4。结果：<br>• 单图 summary 列两张 1440×900 截图；浏览器回执的 `coverage` 仍是四个桌面窗口加窄屏各亮、暗共 10 条<br>• 比较 summary 列六张；只写五张观察的记录被拒，六张齐全后完成<br>• 不看图直接读 summary，状态仍是 `required-pending` / `incomplete` | 1. `"$E" finalize erd skills/phoenix-diagram/examples/minimal.erd.json $OUT/erd.html --json`，读 summary 的 `screenshots` 与 `stages[2].receipt.coverage`<br>2. 看两张图，`review-record` 登记<br>3. `"$E" compare architecture $OUT/base.json $OUT/head.json $OUT/cmp.html --json`<br>4. `"$E" visual-check $OUT/cmp.html --out-dir $OUT/cmp-evidence --json`<br>5. 先用五张的记录登记（应失败），再看六张并登记 | 与 V-1 同批 |
| V-3 | 场景 E2E | **文档作者按 summarize 的阅读面要求生成带图页面。** 角色：只读 `skills/elintp` 与 `skills/phoenix-diagram` 的 skill 文件，不读实现源码。输入：`docs/PRD-ELINTP-VISUALS.md`，附 summarize §3 的三条硬要求原文，页面需要 §2.3 主路径的流程图。覆盖 WORK-F5/AC-2、C-1。不调用 `report_submit`。结果：<br>• 终稿是单个 HTML；图是内联静态图，没有 `iframe`<br>• 断网打开无任何网络请求，图完整<br>• 切换页面亮暗，图随之切换<br>• 360 px 宽下页面不横向滚动，图在自己的区域内横向滚动<br>• 打印预览中图完整、为亮色<br>• 交付说明写明图的形态与实际做过的检查<br>常见变体与误操作：同一材料不带内联要求再生成一次，图为页内嵌入阅读器（默认不变）；把一份自绘 SVG 写到 `static:` 下，得到带行号的格式错误且旧页面未被覆盖 | 1. 按 `skills/elintp/SKILL.md` 执行，phoenix-diagram 的命令用 `$E`（launcher 只负责定位引擎）<br>2. 用 Chrome 打开终稿：DevTools Network 设为 Offline 后刷新，确认请求列表只有文件本身；点主题按钮；把窗口设为 360 px 宽；打开打印预览<br>3. `grep -c '<iframe' 终稿`（应为 0）、`grep -c 'data-phoenix-static' 终稿`（等于图数）<br>4. 两个变体各运行一次 `review.mjs` 或 `render.mjs` | M4 交付齐备后（依赖 V-1 已通过的开发引擎） |
| V-4 | 局部 | 准入依据：分支多的纯解析逻辑，以及 E2E 难以触发的溢出失败路径。<br>• `staticFigureCss` 的选择器改写：§5.2 表格每种分支形态各一例，加一条带主题规则的五处输出；生成物过期时 `--check` 失败（新增，放 `reader-assets.test.js`）<br>• `static:` 来源：带标记的文件原样内联且 `iframe` 为 0；无标记文件退出 2 并给行号与改法（新增，放 `test/elintp/render.test.js`）<br>• 既有浏览器合同测试随数量更新：任一窗口溢出仍判失败（WORK-F10/AC-2），截图数为 2 与 6 | `mise exec --no-deps -- node --test test/phoenix-diagram/reader-assets.test.js`<br>`mise exec --no-deps -- node --test test/elintp/render.test.js`<br>`$CARGO test $MF --test browser_contract --test delivery --test compare --test compare_browser` | Rust 与装配部分在 M1–M3 批末；elintp 部分在 M4 批末 |
| V-5 | 阶段收口 | 全量回归与兼容（WORK-N3/AC-3、WORK-F9/AC-1、AC-3、C-3、C-4、C-7）：<br>• 十一类最小示例的阅读器 HTML 与基线 SHA-256 逐一相同<br>• 十一类都能取得静态图，自动阶段全过，体积比均 ≤ 50%<br>• 基线引擎产出的旧交付（四张截图的 summary）用新引擎仍可按四张登记<br>• 现行文档中不再有旧数量的复述；skill 描述与引用测试通过；bundle 与源一致 | 1. `mise exec --no-deps -- npm run test:diagram`（fmt、clippy、全部 Rust / Node / Reader 测试；耗时未知）<br>2. `mise exec --no-deps -- node --test test/elintp/render.test.js test/elintp/review.test.js`<br>3. 最终代码上重新开发构建（新输出目录），对十一类各运行 `deliver` 与 `static`，比对 SHA-256 与 `wc -c`，并逐类实际打开亮、暗两张静态截图；对基线的 workflow 旧交付运行 `review-record`（四张）<br>4. `git grep -n -e '四张' -e '12 张' -e '2048×1320' -- skills/phoenix-diagram docs/phoenix-diagram.md README.md skills/documentation-writer skills/elintp`，逐条确认不是现行截图数量的复述；对照 PRD §5.3 的 C6、C11、C12 读一遍两个 skill 的 diff<br>5. `node scripts/build-phoenix-diagram.mjs --sync-bundle` 后 `git status --short dist/phoenix-diagram/bundle` 无未提交差异 | M5，代码改完后 |
| V-6 | 真实使用验收 | 从干净的用户状态连续完成三件事，亲自看产物（WORK-F9、WORK-F10、WORK-F5/AC-2 的本期部分）：<br>1. 新项目按文档离线安装后，图解作者用安装后的 skill 入口，为一份自己写的 sequence candidate 交付阅读器 HTML（看两张、登记）并取静态图（看两张、登记）<br>2. 文档作者在同一项目里用 elintp 把 `docs/phoenix-diagram.md` 讲成一页，明确要求零网络阅读面，页面带上一步的静态图<br>3. 读者断网打开终稿，切换亮暗，在窄屏下阅读<br>发现的用户可见问题修复后复验，或列给用户确认延期 | 1. 新建临时目录作项目，按 `docs/phoenix-diagram.md`「离线安装」：`node bin/xgent-skills.js install "$PROJECT" --no-phoenix-ui --providers=claude --diagram-release-dir="<V-5 第 3 步的开发构建目录>"`<br>2. `PD="$PROJECT/.claude/skills/phoenix-diagram/scripts/phoenix-diagram"`，只凭安装后的 `SKILL.md` 与 `references/` 操作<br>3. elintp 用仓内 `skills/elintp`；角色只读两个 skill 的文件与公开文档<br>4. 浏览器操作同 V-3 第 2 步 | M5，V-5 通过后 |

V-3、V-6 是 Agent 角色模拟，不代表真人可用性结论。门户报告阅读面的实际显示未覆盖（A-1）。

## 10. 里程碑与验收安排

M1–M3 同批：都在同一个开发引擎上验证，之间没有风险门，连续实施，批末构建一次、运行 V-1、V-2 与 V-4 的引擎部分。M4 一批。M5 收口。M4 的代码可在 §5.3 文件契约确定后与 M1–M3 并行编写（只写 `skills/elintp` 与 `test/elintp`，与引擎无共享文件）；它的 V-3 需要 V-1 通过后的开发引擎。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 引擎写出静态 SVG | 基线已记录（§9.1） | **落点**：§5.2 装配脚本 `staticFigureCss` 与生成物 ★；§5.3 `render.rs` `render_static`；§5.1 `cli.rs`；§5.4 第 1、2、4、5 步（`delivery.rs`、`finalize.rs`、`support.rs`）<br>**同构**：`finalize.rs` `finalize`；装配脚本的片段读取与 `--check`；`viewer/export.js` L275–L305 的选择器约定<br>**文档**：`references/static-figure.md` ★ 的命令与文件契约部分 | 与 M3 同批：V-1、V-4；回写「实施进度」 |
| M2 | 静态图的浏览器检查、截图与审核登记 | M1 的 `.svg` 输出 | **落点**：§5.4 `browser/static_figure.rs` ★、`static-probe.js` ★、`cdp.rs` `inspect_static` / `screenshot_static`、`evidence.rs` 主干命名<br>**同构**：`browser/mod.rs` `check_browser`；`cdp.rs` `inspect`、`screenshot`<br>**文档**：`static-figure.md` 的看图与登记部分 | 与 M3 同批：V-1；回写「实施进度」 |
| M3 | 单图与架构比较审核减量，接口文字同步 | 无（与 M1、M2 改同一 crate，同一人顺序修改 `evidence.rs`、`mod.rs`） | **落点**：§5.5 三处代码与四个测试文件的数量断言；§5.7 全部文字；`--sync-bundle`<br>**同构**：各文件现有写法<br>**注意**：`SKILL.md` 描述改后跑 `skill.test.js` | 批末（M1–M3 交付齐备、开发引擎构建后）：V-1、V-2、V-4 的引擎与装配部分；回写「实施进度」 |
| M4 | elintp 用静态图内联嵌图 | §5.3 文件契约；V-3 需 V-1 已通过 | **落点**：§6.1 `draft.mjs`；§6.2 `page.css`；§6.3 `draft-format.md`、`SKILL.md`；`test/elintp/render.test.js`<br>**同构**：`draft.mjs` L244 的截图容器、L263–L277 的 `figure` 分支 | V-3、V-4 的 elintp 部分；回写「实施进度」 |
| M5 | 阶段收口 | M1–M4 完成 | 全量回归、兼容比对、接口复述核对、bundle 同步、真实使用验收；发现的问题修复后只复验失败项与波及项 | V-5、V-6；回写「实施进度」 |

提交点：M3 批末通过后提交一次（引擎、装配脚本、phoenix-diagram skill 与文档、bundle，覆盖 M1–M3）；M4 通过后提交一次（elintp）；M5 完成后最终提交。跨期需求 WORK-F5、WORK-F9、WORK-N3 在 M5 后仍停在已提交 / 可测试，终验在 P5。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1：门户报告阅读面对 `data:` 字体的处理未实测 | 最坏情况是静态图用系统等宽字体显示，文字宽度与布局测量略有出入 | 用户下次真实 `/summarize` 后查看；必要时按 D4 重新评估 | P3 完成后 | 否 |
| D5：静态图不含摘要卡片与页面标题 | candidate 里写在卡片中的说明不会出现在静态图里，需由宿主页面正文承担 | 用户如要求自带卡片，另立需求 | — | 否 |
| `viewer.css` 的预设（signal-flow、blueprint、editorial）静态图未逐一看图 | 预设专属规则若有未遵守选择器约定的，静态图会缺该样式 | V-5 第 3 步对十一类最小示例取静态图；V-1 用作者 candidate。发现缺失时修 `staticFigureCss` 的改写或把规则改成约定前缀（后者会改变阅读器 HTML，须先停下按偏差处理） | M3 批末 | 否 |
| 开发构建目录能否直接用作离线安装的 release-dir | 影响 V-6 第 1 步的安装方式 | 构建脚本写出引擎、`VERSION.json`、`bundle/`、许可文件与 `build-receipt.json`（`scripts/build-phoenix-diagram.mjs` L436–L441），`--sync-dist` 使包内清单一致；若安装器拒绝，按其报错处理并记录，不绕过安装直接用 `$E` 冒充安装验收 | M5 | 否 |

- 最终状态：**Ready**。
- 定级理由：范围、文件契约、命令、检查与里程碑已定；技术路线经实验确认；前置期 P1、P2 已完成；A-1 属范围外阅读面且有回退行为，不改变本期任何 AC 的判定。

## 12. 已知坑与历史教训

- `assets/template.html` 是生成物，直接改会被 `--check` 判为过期；本期不应出现它的 diff，出现即说明误改了阅读器维护源（`tools/phoenix-diagram/viewer/README.md`）。
- `build.rs` 的 `collect` 遇到缺失输入会 panic，`include_str!` 指向不存在的文件时整个 crate 编译失败：先运行装配脚本生成 `static-figure.css`，再编译（[PHOENIX-DIAGRAM-STATECHART 计划](PHOENIX-DIAGRAM-STATECHART.md) §12）。
- `finalize` 的 stdout 约 600–730 KB，宿主直接读取会被截断；读 summary 文件，不读 stdout（[STATECHART 计划收口记录](PHOENIX-DIAGRAM-STATECHART.records/M6.md)「已知项」）。`static` 的回执同样带完整校验回执。
- 内联 SVG 共用页面的 ID 空间，未加前缀时 `#引用` 会指向别的图（[P1 计划](ELINTP-VISUALS-P1.md)完成记录中的评审修复 F-01）；静态图由引擎加前缀，elintp 不要再改写。
- `skill.test.js` 固定了描述长度、前 100 字的词与 `version: "0.2.0"`；改描述后先跑这个文件。
- [PHOENIX-DIAGRAM-P3 计划](PHOENIX-DIAGRAM-P3.md)的终验里程碑仍记为阻塞，其待取的宿主原件按四张截图的审核合同设计。本计划不处理它；它若恢复，需先按两张、六张的现行合同修订。
