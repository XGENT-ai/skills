# ELINTP-VISUALS-P3-gaps · XGENT-ai Skills · 静态图随图交付卡片（引擎写卡片数据 · elintp 渲染为页面 HTML 卡片）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-13 · `29d692703cfca0e81aa9f7cd5d171ec517f10926` · dirty（未提交的只有台账状态页、P3 的 R1/R2 评审报告、P3 差异核查报告和本计划；`tools/`、`skills/`、`test/`、`dist/` 无改动，代码事实以该工作树为准）。
>
> 需求来源：[elintp 模板化图文说明与 Phoenix Diagram 图解补充 PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P3（第 3 期）。补齐 [P3 计划](ELINTP-VISUALS-P3.md)；差异与决定见 [P3 差异核查报告](ELINTP-VISUALS-P3.prd-dev-gaps.md) G-01 / D-01。WORK-F9、WORK-F5、WORK-N3 的终验期是 P5，本计划完成后这三条停在已提交 / 可测试。
>
> 本期交付：**`static` 产出的 SVG 带上与 Reader 相同的卡片数据；elintp 用 `static:` 内联时，把卡片渲染成页面 HTML 卡片，放在图与图注之间。读者在零网络页面里能看到 Reader 图下的全部卡片内容。**
> **开发范围：`tools/phoenix-diagram/src/render/static_style.rs`（写卡片数据）、`skills/phoenix-diagram` 中复述静态图内容的文字及 `dist/phoenix-diagram/bundle` 副本、`docs/phoenix-diagram.md`、`skills/elintp`（解析卡片数据并渲染、样式、文字）、`test/elintp/render.test.js`；依据见 §0。**
> 本期独特职责：卡片在 Reader 里本来就是 HTML，不进 SVG 图面；这里只把卡片的数据交给宿主，由宿主用自己的组件显示，静态图的截图与图面不变。
> **顶层排除：不把卡片画进 SVG；不改 Reader HTML、静态图截图与审核流程；不重做 P3 已完成的里程碑；不发布 npm 包或引擎。**

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

- 最近更新：2026-11-13 22:18 +08:00
- 当前进度：2/2 个里程碑完成
- 当前状态：M1、M2 均已完成（V-1–V-4 通过）
- 最近完成：M2 · 收口（V-3 回归兼容、V-4 真实使用验收）
- 下一步：无
- 当前阻塞：无
- 代码基线：`13435a65`（M1 提交；M2 无实现代码改动，最终提交见完成记录）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 · 静态图卡片数据与 elintp 卡片区 | 已完成 | 2026-11-13 21:57 +08:00 | 引擎在静态 SVG 写卡片数据，elintp 渲染页面卡片区；V-1、V-2 通过 | [M1.md](ELINTP-VISUALS-P3-gaps.records/M1.md) | `13435a65` |
| M2 · 收口（V-3 回归兼容、V-4 真实使用验收） | 已完成 | 2026-11-13 22:18 +08:00 | V-3 全过（SHA/体积/内联/bundle/测试），V-4 角色模拟终稿单文件零网络、卡片出现 | [M2.md](ELINTP-VISUALS-P3-gaps.records/M2.md) | `3bf1ff09`；UI修复 `b0e182ff` |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID，验收原文见 PRD §3；「内容」列只写本计划补齐的部分。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F9/AC-1 | 功能 | PRD §3 WORK-F9 · P3；差异核查 G-01 / D-01 | 取静态图并内联到宿主后，Reader 图下的全部卡片（作者 `cards` 和 statechart 规则判定卡片）都能在宿主页面读到，标题和条目与 Reader 一致；静态 SVG 图面本身不变 | §5.1、§5.2；V-1、V-3 | 已确认 |
| WORK-F5/AC-2 | 功能 | PRD §3 WORK-F5 · 跨期，终验 P5；D-01 | elintp 以 `static:` 内联时，卡片作为页面内联 HTML 随图出现，页面仍是单文件、零外部资源、无 iframe | §5.2；V-1、V-4 | 已确认 |
| WORK-N3/AC-3 | 兼容 | PRD §3 WORK-N3 · 跨期，终验 P5 | 十一类 Reader HTML 字节不变；P3 已交付、不带卡片数据的静态 SVG 仍能被 `static:` 内联 | §5.3；V-1、V-3 | 已确认 |
| C-1 | 约束 | PRD §7「已交付的部分持续遵守对应的质量门槛」 | 卡片区满足 WORK-N1（360 px 无页面横向滚动、打印可读、状态不只靠颜色区分）与 WORK-N2/AC-1、AC-2（卡片文字只当内容显示，不执行） | §5.2；V-1、V-2 | 已确认 |
| C-2 | 约束 | [AGENTS.md](../../AGENTS.md) `@skill_tool_independence` | elintp 不以 phoenix-diagram 的安装为前提；SVG 没有卡片数据时照常内联，不报错 | §5.2 | 已确认 |
| C-3 | 约束 | AGENTS.md「Working in This Repository」接口复述条款 | 「静态图不含摘要卡片」的现行复述在同一改动内全部更新；P3 计划及其记录是历史，不改 | §5.3；V-3 | 已确认 |
| C-4 | 约束 | [docs/phoenix-ui.md](../phoenix-ui.md) L253 | 改过 `skills/phoenix-diagram` 后运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle` 并提交 bundle | M1；V-3 | 已确认 |
| C-5 | 约束 | AGENTS.md「Working in This Repository」；`scripts/test-phoenix-diagram-native.mjs` | 引擎检查用 mise 固定的 Rust 与 `mbx`，离线、锁定依赖；工具缺失即失败 | §9 环境 | 已确认 |
| C-6 | 约束 | AGENTS.md `@simplicity`、`@failure_handling` | 不新增命令、参数、回执字段或 summary 字段；数据不合约时报格式错误，不做兜底 | §5.1、§5.2 | 已确认 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 卡片以什么形态到达宿主 | 由宿主渲染成 HTML 卡片；引擎只交卡片数据，不画进 SVG | 截图、看图审核、体积与图面都不变；宿主用自己的组件，亮暗与窄屏跟随宿主 | 用户 2026-11-13 确认（D-01） |
| D2 | 卡片数据放在哪里 | 静态 SVG 根下的 `<metadata data-phoenix-cards="">`，内容是 JSON 数组 | 一个文件就是全部交付，`static:` 只引用这一份；数据随 SVG 的身份与审核登记一起变化，不需要宿主再找 summary | ADR-1 |
| D3 | 卡片数据含什么 | 每张卡片只含 `dot`、`title`、`items`，顺序与 Reader `.cards` 相同（statechart 规则判定卡片在前，作者卡片在后） | 与 Reader 的唯一来源 `scene.cards` 一致；没有卡片时不写 `<metadata>` | `render.rs` `render_with_layers` L1440–L1455；`layout/statechart.rs` L972–L987 |
| D4 | elintp 怎么显示 | 在 `<figure>` 内图与 `<figcaption>` 之间插入 `.figure-cards`，每张卡片是标题加列表；`dot` 只校验，不显示颜色 | 卡片含义在标题与条目文字里，不依赖颜色（WORK-N1/AC-2）；不为七种 dot 颜色新增页面色板 | `skills/elintp/scripts/draft.mjs` `figure` 分支 L593–L609；`page.css` 色板 L1–L40 |
| D5 | 旧产物兼容 | `<metadata>` 可选：没有时不输出卡片区 | P3 已交付的静态 SVG 和其他宿主照常可用 | WORK-N3/AC-3；C-2 |

### 0.3 ADR-lite

#### ADR-1：卡片数据随静态 SVG 交付，宿主渲染

- 状态：Accepted
- 背景与驱动：G-01。Reader 的卡片是 SVG 外的 HTML（`render.rs` L1440），静态图只取主图 SVG，于是卡片丢失。Workflow 的文字容量修法又会把超出的文字移进卡片（`skills/phoenix-diagram/references/authoring-defaults.md`「事实与修复」）。用户决定卡片以页面内联 HTML 呈现，由引擎提供数据（D-01）。
- 备选：
  - A. SVG 内 `<metadata>` 携带 JSON。单文件，`static:` 不变；elintp 已有的 SVG 解析器顺带取出。
  - B. 在 `NAME.static-summary.json` 加 `cards` 字段。宿主要按命名约定再找一份含本机路径和审核状态的 summary，`static:` 也要多一个隐式依赖。
  - C. 另写 `NAME.static-cards.json`。多一个文件要传递，内容也不在 SVG 身份与审核登记的覆盖范围内。
- 决策：A。
- 正面后果：交付物仍是一个 SVG；`<metadata>` 不渲染，单独打开 SVG 或其他宿主内联时图面不变；卡片数据进入 SVG 的 SHA-256 身份，改卡片会使原审核登记失效，与改图一致。
- 负面/中性后果：其他宿主要自己读 `<metadata>` 才能显示卡片，在 `static-figure.md` 中写明；数据格式成为静态图文件契约的一部分。
- 重新评估触发：出现需要卡片保留 Reader 原样外观、不能解析 SVG 内数据，或只能渲染图片/纯 SVG（没有可承载 HTML 卡片的页面结构、要求卡片随图一起显示）的宿主。

### 0.4 职责与事实所有权

|  | phoenix-diagram 引擎与 skill | elintp |
| --- | --- | --- |
| 拥有 | 卡片内容与顺序（与 Reader 同源）、`<metadata>` 的写出格式 | 卡片数据的安全解析、页面卡片的结构与样式、最终页面自检 |
| 不拥有 | 宿主怎么排版卡片 | 卡片内容是否正确；不改写、不补造卡片 |

一句话：卡片写什么归 phoenix-diagram，页面上怎么放归 elintp；数据不合约时 elintp 报格式错误，不猜。

### 0.5 明确不在本期

- **把卡片画进 SVG、卡片的 dot 颜色**——D1、D4 已排除；有宿主要 Reader 原样外观时按 ADR-1 重新评估。
- **静态图截图与看图审核覆盖卡片**——截图仍只有主图；卡片在宿主页面由 elintp 的最终页面自检负责。
- **Delta 视图中新增关系标签被「+」标记遮住**——P3 差异核查报告「范围外观察」，比较渲染不在本计划内。
- **SPMS 门户阅读面实测**——P3 计划 §0.1 登记的门户显示待核实项，范围外。
- **npm 发布、引擎四平台构建、CHANGELOG**——发布流程。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **已核实·足够**：Reader 卡片来自 `scene.cards`，每张卡片的 `authored.facts` 含 `dot`、`title`、`items`，渲染成 SVG 外的 `<div class="cards">`（`tools/phoenix-diagram/src/render.rs` `render_with_layers` L1440–L1455）；statechart 的规则判定卡片在编译时放在作者卡片之前（`src/layout/statechart.rs` L972–L987）。
- **已核实·缺口**：静态 SVG 只由 `render_svg` 转换而来，没有卡片（`src/render/static_style.rs` `render` L15–L47）→ §5.1 在 `SvgConversion::write_common` 旁写入卡片数据 → 漏做则卡片内容继续丢失（G-01）。
- **已核实·足够**：`write_common` 在根下 `</desc>` 之后写出公共样式（`static_style.rs` `convert` 的 `name == "desc" && depth == 2` 分支，L1180–L1182），是插入 `<metadata>` 的唯一稳定位置；`render.rs` `escape_text`（L81）转义 `& < > " '`。
- **已核实·缺口**：elintp 静态模式的元素白名单不含 `metadata`（`skills/elintp/scripts/draft.mjs` `STATIC_SVG_ELEMENTS` L9），`inlineSVG` 只返回 SVG 字符串（L306–L414）；`figure` 分支直接把图和 `figcaption` 拼进 `<figure>`（L593–L609，拼接的 `result.push` 在 L609）→ §5.2 → 漏做则带卡片数据的 SVG 被拒。
- **已核实·足够**：浏览器检查只量 `text` 元素（`tools/phoenix-diagram/src/browser/static-probe.js` L89），`<metadata>` 不渲染，不影响 `rendered-dom` 与截图。
- **已核实·足够**：现行文字中写「不含摘要卡片」的是 `skills/phoenix-diagram/references/static-figure.md` L26、`skills/phoenix-diagram/references/delivery-contract.md` L86、`docs/phoenix-diagram.md` L101；elintp 的 `static:` 说明在 `skills/elintp/SKILL.md` L202 与 `skills/elintp/references/draft-format.md` L179。
- **已核实·足够**：P3 最终开发引擎与输出在 `local/phoenix-diagram/elintp-p3-final`、`elintp-p3-final-out`；十一类 Reader HTML 基线 SHA-256 在 `local/phoenix-diagram/elintp-p3-base-out/baseline-sha256.txt`。

### 1.2 拓扑与文件清单

- `tools/phoenix-diagram/src/render/static_style.rs`：`SvgConversion::write_common` 之后写卡片数据（§5.1）。
- `skills/phoenix-diagram/references/static-figure.md`、`skills/phoenix-diagram/references/delivery-contract.md`、`docs/phoenix-diagram.md`：改写「不含摘要卡片」，写明卡片数据契约。
- `dist/phoenix-diagram/bundle/**`：由 `--sync-bundle` 重新生成。
- `skills/elintp/scripts/draft.mjs`：解析 `<metadata data-phoenix-cards>` 并输出 `.figure-cards`（§5.2）。
- `skills/elintp/assets/page.css`：`.figure-cards` 样式。
- `skills/elintp/SKILL.md`、`skills/elintp/references/draft-format.md`：`static:` 一段加卡片说明。
- `test/elintp/render.test.js`：卡片数据安全解析用例（V-2）。
- 不改：Reader 渲染（`render.rs` 及 `viewer/**`）、`static-probe.js`、`browser/**`、`delivery/**`、`cli.rs`、summary 与回执结构、`skills/phoenix-diagram/SKILL.md`、`authoring-defaults.md`、P3 计划及其记录。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `static_style::SvgConversion::write_common` | `SvgConversion::convert` | `scene.cards` 非空时，在公共 `<style>` 之前写出一个 `<metadata data-phoenix-cards="">`；JSON 文本经 `escape_text` 转义；空时不写 | 无 | 只取三字段、顺序不变 | 进程内 | CLI 写出的 SVG；V-1 |
| elintp `inlineSVG`（静态模式） | `parseDraft` 的 `figure` 分支 | 根的直接子元素中最多一个 `metadata`，只带 `data-phoenix-cards=""`，只含文本；文本解码后 `JSON.parse`，结构不合约即报错；`metadata` 不进入输出 SVG，解析出的卡片交回调用方 | 无 | 结构校验与转义 | 本地文件（不可信边界） | 生成页面与拒绝诊断；V-1、V-2 |

不新增接缝：卡片数据只有一个写出方和一个读取方。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| elintp → 静态 SVG 卡片数据 | 已核实缺口 | `STATIC_SVG_ELEMENTS` 无 `metadata`（`draft.mjs` L9） | 范围内：§5.2 | 数据不合约是内容稿格式错误（退出 2），给内容稿行号与 SVG 位置，不覆盖旧页 |
| 其他内联静态 SVG 的宿主 | 已核实足够 | `<metadata>` 不渲染，图面不变 | `static-figure.md` 写明数据格式，由宿主自行显示 | — |
| elintp 审阅页（`review.mjs`） | 已核实足够 | 审阅页与终稿共用 `parseDraft` | 零改动；卡片随图出现在审阅页与终稿 | 同上 |

没有新的命令、参数、配置、环境变量、诊断码或回执 schema 版本。

## 5. 核心机制

### 5.1 引擎写出卡片数据（`static_style.rs`）

- `write_common` 先写卡片数据，再写原有公共 `<style>` 与字体块：

  ```text
  <metadata data-phoenix-cards="">[{"dot":"emerald","title":"…","items":["…"]}]</metadata>
  ```

- JSON 由 `scene.cards` 逐张取 `authored.facts` 的 `dot`、`title`、`items` 组成，`serde_json::to_string` 后经 `super::escape_text` 写出；没有卡片时整段不写。
- 写入位置因此固定为根下 `</desc>` 之后，`<style>` 之前；不改 `render_svg`、命名空间或几何。Reader 分支不经过这里，字节不变。

### 5.2 elintp 解析与渲染（`draft.mjs`、`page.css`）

- `STATIC_SVG_ELEMENTS` 增加 `metadata`。静态模式遇到 `metadata` 时：
  - 必须是根的直接子元素（`stack.length === 1`），全文最多一个，唯一属性为 `data-phoenix-cards=""`，否则报「卡片数据须为根下唯一的 metadata[data-phoenix-cards]」；
  - 只收集其文本（经现有 `decode`），其间出现任何标签即报错；开闭标签与文本都不写入输出 SVG。
- 闭合时 `JSON.parse` 文本，要求是数组，每项恰有 `dot`、`title`、`items` 三个键：`dot` 属于 `cyan emerald violet amber rose orange slate`，`title` 是非空字符串，`items` 是字符串数组。任何一项不符即报「卡片数据结构不合约」。
- `inlineSVG(source, title, prefix, report, staticMode = false, onCards)`：静态模式解析成功时调用 `onCards(cards)`；其他调用方不传。
- `figure` 分支在 `static` 来源下，卡片非空时在图与 `<figcaption>` 之间插入：

  ```html
  <div class="figure-cards"><section class="figure-card"><p class="figure-card-title">标题</p><ul><li>条目</li></ul></section>…</div>
  ```

  标题与条目一律 `escapeHTML`，不走行内 Markdown。卡片区没有标题元素，不进入大纲。
- `page.css`：`.figure-cards` 用 `display:grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 12px; margin-top: 12px`，卡片用 `--surface` 底色与 `--line` 边框，标题 `font-weight: 650`，条目沿用正文颜色；窄屏自动单列，打印块把 `.figure-card` 设为 `break-inside: avoid`。颜色全部取页面已有变量，亮暗与打印自动跟随。

### 5.3 文字同步

- `skills/phoenix-diagram/references/static-figure.md` L26 与 `skills/phoenix-diagram/references/delivery-contract.md` L86：把「不含 Reader 页面标题、摘要卡片、节点索引或交互控件」改为「不含 Reader 页面标题、节点索引或交互控件；卡片以 `<metadata data-phoenix-cards="">` JSON（每项 `dot`、`title`、`items`，顺序同 Reader）随 SVG 交付，不画进图面，由宿主显示」，`static-figure.md` 的「宿主约定」补一句：要显示卡片的宿主读这段数据。
- `docs/phoenix-diagram.md` L101 同样改写，并把「只含主图、关系、标签与图内图例」改为「图面只含主图、关系、标签与图内图例」，使图面与文件内的卡片数据分开表述。
- `skills/elintp/references/draft-format.md` L179：补「SVG 带卡片数据时，脚本把卡片显示在图与图注之间；没有时不显示」；`skills/elintp/SKILL.md` L202 的 `static:` 一句补「卡片随图显示为页面卡片」。
- 运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle`（C-4）。

## 8. 失败模式、发布与回滚

- 新 elintp 内部：引擎多写一段由同一场景数据得到的文本；elintp 新增的只是对不可信输入的格式校验，失败语义沿用现有「格式错误、退出 2、不覆盖旧页」。
- 新增失败模式：**新引擎 + 旧 elintp**。旧 elintp 的静态白名单没有 `metadata`，带卡片数据的新 SVG 被判为格式错误（诊断「SVG L… metadata：元素不在白名单」，退出 2，不覆盖旧页）。elintp 作为普通 skill 独立安装与更新，phoenix-diagram 随 npm 安装器更新（[README.md](../../README.md)「Phoenix UI 和 Phoenix Diagram skill 均由 npm 安装器随匹配引擎安装与更新」），两者不保证同时更新。恢复：更新 elintp 到含本计划改动的版本；无法更新时用旧引擎重新取图（新引擎重取仍会写 `<metadata>`）。没有卡片的图不写 `<metadata>`，不受影响。
- 回滚：回退提交只恢复代码；已生成的带卡片 SVG 不会因此变化，在回退后的 elintp 下同样被拒，需用回退后的引擎重新取图。

## 9. 验证

### 9.1 行为与接口验证

**环境**

- 在仓根执行；工具已按 `node scripts/prepare-phoenix-diagram.mjs --prepare` 准备好（C-5）。
- `CARGO` 表示 `mise exec --no-deps -- mbx`，`MF` 表示 `--manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline`。
- 开发引擎：`mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/elintp-p3-gaps`，`E=local/phoenix-diagram/elintp-p3-gaps/phoenix-diagram-darwin-arm64`；输出目录须为新目录或空目录，耗时未知。不加 `--sync-dist`：本计划不做安装验收，`--sync-dist` 会删除并重写 git 跟踪的 `dist/phoenix-diagram` 许可文件（`scripts/build-phoenix-diagram.mjs` `syncDist`），超出 §1.2；bundle 按 C-4 只用 `--sync-bundle` 生成。
- 产物目录：`OUT=local/phoenix-diagram/elintp-p3-gaps-out`（`local/` 不入库）。
- 看图由具备图像能力的 Agent 实际打开截图或页面截图完成。

**批内最小反馈**

- 只编译：`$CARGO build $MF`；单个 elintp 用例：`mise exec --no-deps -- node --test --test-name-pattern='<过滤词>' test/elintp/render.test.js`。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **图解作者取一张带卡片的静态图，文档作者把它放进零网络页面。** 输入：`minimal.statechart.json`（自带规则判定卡片），以及作者在 `minimal.workflow.json` 上加两张卡片（一张条目含 `<b>` 与 `&` 字样）的 candidate。覆盖 WORK-F9/AC-1、WORK-F5/AC-2、WORK-N3/AC-3、C-1、C-2。结果：<br>• 两种字体策略的 `static` 三门全过，SVG 内有且仅有一个 `metadata[data-phoenix-cards]`，卡片标题与条目和同一 candidate 的 Reader HTML `.cards` 逐项相同、顺序相同；每份仍是亮、暗两张截图，图面与 P3 同类同策略产物一致；4 份产物按静态图流程看图登记后 `visualReview.status: "passed"`、`deliveryStatus: "complete"`<br>• elintp 页面的 `.figure-cards` 显示全部卡片，`<b>` 与 `&` 按原文显示为文字；`<iframe` 为 0，system 无字体资源<br>• 断网打开请求为 0；切换亮暗时卡片底色与文字随页面变化；360 px 无页面横向滚动，卡片单列；显式暗色下打印卡片为亮色且完整<br>• 引用 P3 已交付的无卡片 SVG（`local/phoenix-diagram/elintp-p3-final-out/workflow.system.svg`）的页面照常生成，没有卡片区 | 1. `"$E" static statechart skills/phoenix-diagram/examples/minimal.statechart.json $OUT/sc.system.svg --font-policy system --json`，embedded 同理；workflow 卡片 candidate 同样两种策略<br>2. `"$E" deliver` 同两份 candidate，对照 Reader `.cards` 与 SVG 内 JSON<br>3. 4 份 static 产物（statechart、workflow × 两种策略）是本场景作者的交付：逐份打开 summary 列出的亮、暗两张截图（共 8 张），写非空 `observations` 后运行 `"$E" review-record <svg> --record <review.json> --json`，读各自 summary<br>4. 写 general 内容稿，以 `static:` 引用上述 SVG 与 P3 旧 SVG，`node skills/elintp/scripts/render.mjs <draft.md> <page.html>`<br>5. Chrome 打开页面：DevTools Network Offline 后刷新；切换主题；窗口 360 px；显式 dark 下打印预览；`grep -c '<iframe'` | M1 交付齐备、开发引擎构建后 |
| V-2 | 局部 | 准入依据：不可信输入的安全解析（WORK-N2/AC-2、C-1）。在 `render.test.js` 静态 SVG 攻击表中增加：合法卡片数据正确显示且条目中的 `<script>` 被转义；`metadata` 不在根的直接子级、出现两个、带其他属性、内含元素、JSON 无效、多余键、`dot` 不在枚举、`items` 含非字符串，均退出 2 且旧页字节不变；无卡片数据的静态 SVG 照常通过 | `mise exec --no-deps -- node --test test/elintp/render.test.js test/elintp/review.test.js` | M1 交付齐备后 |
| V-3 | 阶段收口 | 回归与兼容（WORK-N3/AC-3、WORK-F9/AC-1、C-3、C-4）：<br>• 十一类最小示例的 Reader HTML 与 `baseline-sha256.txt` 逐一相同<br>• 十一类两种策略的 `static` 全部三门通过、体积 ≤ Reader 的 50%，22 份 SVG 经 `static:` 内联全部退出 0；带卡片的类型页面出现卡片区<br>• 现行文字中不再有「静态图不含摘要卡片」的复述；bundle 与源一致 | 1. 在 `tools/phoenix-diagram` 下：`mise exec --no-deps -- mbx fmt -- --check` 与 `mise exec --no-deps -- mbx clippy --locked --offline --all-targets -- -D warnings`<br>2. 用 `$E` 对十一类各运行 `deliver` 与两种策略的 `static`，比对 SHA-256 与 `wc -c`，逐份写最小内容稿运行 `render.mjs`<br>3. `git grep -n '摘要卡片' -- skills/phoenix-diagram docs/phoenix-diagram.md skills/elintp`，逐条确认<br>4. `node scripts/build-phoenix-diagram.mjs --sync-bundle` 后 `git status --short dist/phoenix-diagram` 无未提交差异，`dist/phoenix-diagram` 下 bundle 以外的文件（`LICENSE`、`NOTICE.md`、`THIRD-PARTY*`）自基线起无改动；`mise exec --no-deps -- node --test test/phoenix-diagram/skill.test.js` | M2 |
| V-4 | 真实使用验收 | **文档作者按 summarize 的阅读面要求，把一份需要带卡片流程图的 PRD 讲成一页。** 角色只读 `skills/elintp`、`skills/phoenix-diagram` 的 skill 文件及其引用文档，不读实现源码；输入 `docs/PRD-ELINTP-VISUALS.md` §2.3 主路径，附 summarize §3 三条硬要求；不调用 `report_submit`。结果：终稿单文件、无 iframe、system 字体；按 Workflow 修法移进卡片的文字出现在页面卡片区，作者不需要手工补进图注；交付说明写明图与卡片的形态 | 按 `skills/elintp/SKILL.md` 执行，phoenix-diagram 命令用 `$E`；浏览器检查同 V-1 第 5 步 | M2，V-3 通过后 |

V-4 是 Agent 角色模拟，不代表真人可用性结论。

## 10. 里程碑与验收安排

M1 一批：引擎与 elintp 改动在同一场景下验证。M2 收口。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 静态图卡片数据与 elintp 卡片区 | 无 | **落点**：§5.1 `static_style.rs` `write_common`；§5.2 `draft.mjs` `STATIC_SVG_ELEMENTS`、`inlineSVG`、`figure` 分支，`page.css`；§5.3 全部文字与 `--sync-bundle`；`test/elintp/render.test.js`<br>**同构**：`write_common` 现有写法；`draft.mjs` 中 `style` 元素的文本收集分支（L335–L337）与 `metrics` 块的 HTML 拼接（L586–L592）<br>**并行**：引擎与 elintp 文件不共享，可由两人分别编写；§5.1 的数据格式先定 | V-1、V-2；回写「实施进度」 |
| M2 | 收口 | M1 完成 | 回归、兼容、接口复述、bundle 同步、真实使用验收；发现的问题修复后只复验失败项与波及项 | V-3、V-4；回写「实施进度」 |

提交点：M1 完成后提交一次（引擎、两份 skill、文档、bundle、测试）；M2 完成后最终提交。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 带卡片数据的新 SVG 在旧 elintp 下被拒（§8） | 只影响已更新 phoenix-diagram、未更新 elintp 的环境；旧 elintp 报格式错误、不覆盖旧页，不是 SVG 缺陷 | 本计划：引擎与 elintp 改动在 M1 同一提交，推送 main 后 elintp 经 skill 更新即可拿到，早于引擎随 npm 发布；恢复方法写在 §8。范围外（§0.5 发布流程）：执行下一次 `npm run release:phoenix` 的发布者在 `CHANGELOG.md` 该版本条目写明「带卡片的静态图需同时更新 elintp」 | 下一次 phoenix-diagram npm 发布写 CHANGELOG 时 | 否 |
| 卡片条目很长或数量很多时页面变长 | 页面高度增加，不影响可读性 | 网格自动换行；没有配额限制，与 Reader 一致 | V-1 | 否 |

- 最终状态：**Ready**。
- 定级理由：用户已决定卡片形态与归属（D-01）；数据格式、写入位置、解析规则、页面结构与验证都已确定，没有影响范围、安全或外部契约的未决项。

## 12. 已知坑与历史教训

- elintp 的静态 CSS 会拒绝残留 `&` 与字符引用（[P3 R1 F-02](ELINTP-VISUALS-P3.code-review.md)）；卡片 JSON 在 `<metadata>` 文本里，不经过 CSS 解析，但输出时仍须 `escapeHTML`，不能原样拼接。
- 静态字体块的许可注释曾含原始 `&`，使独立 SVG 成为无效 XML（[P3 M1 记录](ELINTP-VISUALS-P3.records/M1.md)「提交后修正」）；卡片 JSON 必须经 `escape_text`，写完用 `xmllint --noout` 检查一份带卡片的 SVG。
- `finalize` / `static` 的 stdout 很大，读 summary 文件，不读 stdout（P3 计划 §12）。
