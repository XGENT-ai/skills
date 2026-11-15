# ELINTP-VISUALS-P4-gaps · XGENT-ai Skills · 数值图表补缺（图例键、强调色槽、纯数字数据表、仅上限行样式）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-15 · `b4c7b568539e59f62fc6756945b30686a70c1480` · dirty（未提交改动是 use-llm-gateway skill、`.xgent-ai/**` 台账、其他计划的评审文档与本计划的差异报告，不在本计划改动面；`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp` 相对 HEAD 无改动）。
>
> 需求来源：[elintp 模板化图文说明与 Phoenix Diagram 图解补充 PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P4（第 4 期）；补齐原计划 [ELINTP-VISUALS-P4](ELINTP-VISUALS-P4.md)，差异与用户决定见 [P4 差异核查报告](ELINTP-VISUALS-P4.prd-dev-gaps.md) G-01、G-02、G-05/D-01、G-06/D-02。本计划承接 WORK-F6/AC-1、AC-2、AC-4 与 §2.4 数值图表 UI 要求的未满足部分；WORK-F9/AC-5 随静态图同源修复；WORK-F5、WORK-F9、WORK-N3 的终验期仍是 P5。
>
> 本期交付：**数值图表的四处图面与数据表修正：多系列图例色块写系列键；超限填充与饼图收尾扇区使用醒目色；阅读器与静态图的数据表单元格只放数字、单位进列头且括号随语言；只有上限的限额行改用虚线框样式并在图例写「仅上限」。**
> **开发范围：`tools/phoenix-diagram` 的 chart 布局、渲染、内容检查、五语言目录与生成的阅读器模板，以及 `tools/phoenix-diagram/tests/chart.rs`；依据见 §0。**
> 本期独特职责：只改 `chart` 的既有行为，不加形态、字段、命令或静态数据格式。
> **顶层排除：elintp 与 phoenix-diagram skill 文字不改；十一类图型不改；不改 PRD 与原计划；不发布。**

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

- 最近更新：2026-11-15 13:05 CST
- 当前进度：1/2 个里程碑完成
- 当前状态：进行中
- 最近完成：M1 · 图例键、强调色槽、纯数字数据表、仅上限样式
- 下一步：M2 · 按 §9 跑 V-3/V-4 并收口阶段
- 当前阻塞：无
- 代码基线：`b4c7b568`

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-15 13:05 CST | 四处 chart 修正落地，V-1/V-2 通过 | [M1 记录](ELINTP-VISUALS-P4-gaps.records/M1.md) | 待提交 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

PRD 需求沿用工作号与 AC ID，差异沿用报告的 G-ID 与 D-ID，原文见来源，不重抄。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| G-01 | 差异（A） | 报告 G-01；WORK-F6/AC-2、§2.4 | 多系列图例色块写系列键，不写系列 id | §5.1；V-1、V-2 | 已确认 |
| G-02 | 差异（A） | 报告 G-02；WORK-F6/AC-4、§2.4 | `security`（超限）与 `external`（饼图收尾扇区）获得与其他色槽相同的醒目填充 | §5.2；V-1 | 已确认 |
| G-05 | 差异（C，已决 D-01） | 报告 G-05；WORK-F6/AC-4 | 数据表单元格带系列键、饼图单元格带单位、非中文列头用全角括号 | 按 D-01 处理 | 已决 |
| G-06 | 差异（C，已决 D-02） | 报告 G-06；WORK-F6 行为、§2.4 | 只有上限的限额行用当前值色填到上限 | 按 D-02 处理 | 已决 |
| G-03 | 差异（B） | 报告 G-03 | 静态图数据表由宿主读取 `data-phoenix-table` 显示 | §0.5 保留，无动作 | 排除 |
| G-04 | 差异（B） | 报告 G-04 | 十一类结果按主图、元数据与静态 SVG 判定 | §5.5 沿用；V-3 | 已确认 |
| D-01 | 用户决定 | 报告 D-01（用户 2026-11-15 确认） | 数据表数值单元格只写数字；单位只在列头（饼图「数值（单位）」，限额沿用单位列）；列头括号随语言 | §5.3；V-1、V-2 | 已确认 |
| D-02 | 用户决定 | 报告 D-02（用户 2026-11-15 确认） | 只有上限的限额行仍画到上限，但样式区别于当前值填充；图例在有这类行时加「仅上限」 | §5.4；V-1、V-2 | 已确认 |
| WORK-F9/AC-5 | 功能（跨期，终验 P5） | PRD §3 WORK-F9 | 静态图与阅读器同源，上述修正同样进入静态图与其 `data-phoenix-table` | §5；V-1、V-3 | 已确认 |
| WORK-N3/AC-4 | 兼容（跨期，终验 P5） | PRD §3 WORK-N3；原计划 D14 | 十一类主图 SVG、内嵌元数据与两策略静态 SVG 不变；新增目录文案只改变模板 | §5.5；V-3 | 已确认 |
| C-1 | 约束 | `tools/phoenix-diagram/viewer/README.md` | 五语言目录是唯一文案来源；改目录后运行装配脚本，`tools/phoenix-diagram/assets/template.html` 与 `tools/phoenix-diagram/assets/catalogs.json` 不手改 | §5.3、§5.4；V-2 | 已确认 |
| C-2 | 约束 | `tools/phoenix-diagram/src/render/static_style.rs` `scope_selector`、`supported_declaration` | chart `defs` 规则以 `svg[data-chart-ui]` 开头，只用支持清单内的声明，否则 `static` 报 `static/style` | §5.2、§5.4 | 已确认 |
| C-3 | 约束 | AGENTS.md `@skill_tool_independence`、「Working in This Repository」 | elintp 不改；elintp 只按字符串渲染表格，单元格写法变化对其透明 | §0.5 | 已确认 |
| C-4 | 约束 | AGENTS.md；`scripts/test-phoenix-diagram-native.mjs` | 引擎检查用 mise 固定的 Rust 与 `mbx`，离线、锁定依赖；工具缺失即失败 | §9 环境 | 已确认 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 列头括号随语言的实现 | 新目录键 `chart.table.withUnit`，占位符 `{name}`、`{unit}`；zh-CN、zh-TW 写 `{name}（{unit}）`，en、es、ko 写 `{name} ({unit})` | 代码不按语言分支；装配 `--check` 保证五份占位符一致 | D-01；C-1 |
| D2 | 仅上限行的样式 | 新图形类 `ch-span`：无填充、`var(--text-dim)` 描边 1.5、`stroke-dasharray: 4 3`，长度到上限；图例符号为同样式的 14×10 框 | 与当前值的实心填充一眼可分，亮暗都用既有主题变量；声明全在静态支持清单内 | D-02；C-2 |
| D3 | 强调色槽的样式来源 | 把 `security`、`external` 加进 `render/chart.rs` 生成色槽规则的循环；系列循环色 `TONES` 不变 | 只补规则，不改哪一行用哪个色槽 | G-02 |
| D4 | 版本号 | 保持 `0.2.0`，phoenix-diagram skill 文件与 bundle 不变 | 本计划只改引擎；静态数据格式（字符串单元格）不变 | 原计划 D13 |

### 0.4 职责与事实所有权

|  | phoenix-diagram 引擎（本期） | elintp | 宿主 Agent |
| --- | --- | --- | --- |
| 拥有 | 图例、色槽、表格单元格与仅上限样式；数据表与图面的一致性检查 | 把 `data-phoenix-table` 的字符串单元格渲染成表格 | 数值来源 |
| 不拥有 | 宿主怎么排版表格 | **单元格内容的改写** | 引擎的检查结论 |

一句话：数据表写什么归引擎，页面怎么放归 elintp，两者之间的契约（字符串单元格、`columns`/`rows`）不变。

### 0.5 明确不在本期

- **elintp 与 phoenix-diagram skill 文字**——不改（C-3）；`static-figure.md`、`delivery-contract.md` 描述的「单元格均为字符串」仍成立。
- **原计划与 PRD 的文字**——只读；原计划 §5.3、§5.4、§5.8 与本计划不同之处由本计划 §5 取代，记录在本计划。
- **其他差异报告结论**：G-03（静态图数据表由宿主显示）、G-04（D14 判定口径）保留现状，无动作。
- **新形态、字段、图例交互或数据表复制按钮**——PRD 未要求。
- **发布、CHANGELOG**——发布流程处理。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **已核实·缺口**：`tools/phoenix-diagram/src/render/chart.rs` `legend_glyph`（L284–L293）用 `legend.authored.kind` 去掉 `series:` 前缀后的系列 id 作色块文字；`facts.key`（`layout/chart.rs` L1469 写入）未使用 → §5.1 → 漏做则 id 溢出色块、压住图例文字。
- **已核实·缺口**：`check/chart.rs` L165–L185 只要求图例文字「包含」`键 · 系列名`，色块写错仍通过 → §5.1。
- **已核实·缺口**：`render/chart.rs` L58–L66 只为 `TONES = ["frontend","backend","database","cloud","messagebus"]`（L16）生成 `.ch-bar/.ch-point/.ch-line/.ch-slice/.ch-fill` 的色槽规则；`layout/chart.rs` L819 的超限填充 `security`、L562 的饼图收尾 `external`、L1389 的「超限」图例落到通用浅色 `.c-security/.c-external` → §5.2。`--security-stroke`、`--external-stroke` 在 `tools/phoenix-diagram/viewer/viewer.css` L42/L46、L83/L87 有亮暗定义。
- **已核实·缺口**：表格在 `layout/chart.rs` L1492–L1593 生成：柱、折线单元格用闭包 `number_text`（L280，多系列加键）；饼图数值单元格带单位后缀；列头用硬编码全角括号 `\u{ff08}…\u{ff09}`（L1571、L1578） → §5.3。
- **已核实·缺口**：限额行 marks（`layout/chart.rs` L812–L840）对没有当前值的行仍发 `kind: "fill"`、色槽为第一个系列色，渲染为 `c-frontend ch-fill`（`render/chart.rs` L230）；图例只有「当前值」「上限」「超限」（`layout/chart.rs` L1370–L1392） → §5.4。
- **已核实·足够**：阅读器表格（`render/chart.rs` `data_table`）、静态数据（`static_style.rs` `write_common`）与表格检查（`check/chart.rs` `table`，L589）都读同一份 `facts.table`，改一处即三处一致。
- **已核实·足够**：`check/chart.rs` L533–L560 的限额检查取 `rect.ch-fill` 计算长度比；`tests/chart.rs` 有 `limits_fill_ratio_limit_line_and_over_mark`（L638）、`pie_sectors_and_list_match_facts`（L560）、`equal_values_of_two_series_keep_separate_annotations`（L358）、`three_forms_pass_the_artifact_check`（L726），经 `check::check_html`（`src/check.rs` L352）断言产物。
- **已核实·足够**：elintp `draft.mjs` 只校验单元格为字符串并 `escapeHTML`，不解析数字（`test/elintp/render.test.js` L153–L156 用任意字符串）。

### 1.2 拓扑与文件清单

- `tools/phoenix-diagram/src/layout/chart.rs`：表格单元格与列头（§5.3）；仅上限行的 mark 与图例条目（§5.4）。
- `tools/phoenix-diagram/src/render/chart.rs`：图例色块文字（§5.1）；色槽规则循环（§5.2）；`ch-span` 样式、mark 分支与图例符号（§5.4）。
- `tools/phoenix-diagram/src/check/chart.rs`：图例色块文字（§5.1）；限额检查区分 `ch-fill` 与 `ch-span`、「仅上限」图例（§5.4）。
- `tools/phoenix-diagram/viewer/locales/{en,zh-CN,zh-TW,es,ko}.json`：`chart.table.withUnit`、`legend.chart.limitOnly`；装配生成 `tools/phoenix-diagram/assets/template.html`、`tools/phoenix-diagram/assets/catalogs.json`。
- `tools/phoenix-diagram/tests/chart.rs`：§9 V-2。
- 不改：十一类的布局、渲染、检查与 schema；`skills/phoenix-diagram/schemas/chart.schema.json`；`static_style.rs`；阅读器脚本与 `template.source.html`；`skills/**`、`dist/**`、`test/**`。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| elintp → 静态 SVG `data-phoenix-table` | 已核实足够 | `columns` 为非空字符串数组、每行等长字符串数组（`skills/elintp/scripts/draft.mjs` `parseTable`） | 零改动；单元格仍是字符串 | 不变 |
| 静态样式转换 | 已核实足够 | `supported_declaration` 含 `fill`、`stroke`、`stroke-width`、`stroke-dasharray` | 新规则遵守 C-2 | 违规时 `static` 报 `static/style`，修规则 |

没有新的命令、参数、诊断码、回执字段或静态数据属性；新增目录键 `chart.table.withUnit`、`legend.chart.limitOnly`，新增图形类 `ch-span`。

## 5. 核心机制（引擎）

### 5.1 图例色块写系列键（G-01）

- `render/chart.rs` `legend_glyph` 的 `series:` 分支：色块内文字改为 `facts["key"]`（已由布局写入），其余几何不变。
- `check/chart.rs` 系列图例检查（L165 起）：在现有「包含 `键 · 系列名`」之外，要求该图例组内第一个 `<text>`（色块文字）等于系列键；不符报 `artifact/semantic-text`，路径 `/metadata/facts/series`。

### 5.2 强调色槽规则（G-02）

`render/chart.rs` 生成色槽规则的循环（L58）改为遍历 `TONES` 加 `security`、`external`；生成的五类规则（`.ch-bar`、`.ch-point`、`.ch-line`、`.ch-slice`、`.ch-fill`）与既有色槽相同。`TONES` 本身（系列与饼图循环色）不变。效果：超限填充与「超限」图例色块用 `var(--security-stroke)`，饼图收尾扇区与其列表色块用 `var(--external-stroke)`。

### 5.3 数据表单元格（D-01）

`layout/chart.rs` 表格段（L1492–L1593）：

| 形态 | 列头 | 数值单元格 |
| --- | --- | --- |
| 柱、折线 | 「项目」加各系列名（无 `series` 时「数值」）；有单位时用 `chart.table.withUnit`（`{name}` 为列名，`{unit}` 为 `meta.unit`） | `js_to_string(v)`，不带系列键 |
| 饼 | 「项目」、「数值」（有单位时经 `chart.table.withUnit`）、单位不是 `%` 时加占比列，列头经 `chart.table.withUnit`（`{name}` 为 `chart.table.share`，`{unit}` 为 `%`），如 `Share (%)`、`占比（%）` | 数值 `js_to_string(v)`，不带单位；占比沿用现有派生值与一位小数舍入，只写数字（`35`），不带 `%` |
| 限额 | 不变 | 不变（已是纯数字与单位列） |

占比是派生数值，同样遵守 D-01 的「单元格只写数字」。图面数值标注不变（多系列仍「A 12」，饼图列表仍带单位与 `35%`）。`chart.table.withUnit` 五份译文：zh-CN、zh-TW `{name}（{unit}）`；en、es、ko `{name} ({unit})`。写完运行装配脚本（C-1）。

### 5.4 仅上限的限额行（D-02）

- 布局：没有 `value` 的行，第二个 mark 改为 `{"kind": "span", "x", "y", "width", "height"}`，宽度仍按「上限 ÷ 满长」计算（与现有填充长度相同），不带色槽；有 `value` 的行保持 `kind: "fill"`。任一行没有 `value` 时，图例在「上限」之后加条目 `kind: "limits-limit-only"`，文字 `legend.chart.limitOnly`（zh-CN「仅上限」、zh-TW「僅上限」、en「Limit only」、es「Solo límite」、ko「한도만」）。
- 渲染：`render/chart.rs` mark 分支加 `"span"` → `<rect … class="ch-span{dim_class}"/>`；`defs` 加规则 `svg[data-chart-ui] .ch-span { fill: none; stroke: var(--text-dim); stroke-width: 1.5; stroke-dasharray: 4 3; }`；`legend_glyph` 对 `limits-limit-only` 画同类 14×10 的 `rect.ch-span`。右侧文字（`chart.limit.only`）与行属性（`data-chart-value` 缺省、`data-chart-limit`）不变。
- 检查：`check/chart.rs` 限额检查取「`rect.ch-fill` 或 `rect.ch-span`」计算长度比；另要求没有当前值的行恰有 `rect.ch-span` 且没有 `rect.ch-fill`，有当前值的行反之；「仅上限」图例条目当且仅当存在这类行时出现。失败码沿用 `artifact/semantic-relation`，路径 `/metadata/facts/rows/<i>`。

### 5.5 兼容

十一类不经过上述代码；新增目录键只改变模板，十一类的主图 SVG、内嵌元数据与两策略静态 SVG 应与原计划基线逐一相同（原计划 D14 口径）。chart 产物（含 `minimal.chart.json`）的图例、样式与表格按本节改变，属预期。

## 8. 失败模式、发布与回滚

不引入新的运行期失败模式：改动只影响 chart 的图面文字、样式与表格内容，校验与交付流程不变。发布随 npm 包，按原计划 §8；旧 elintp 读取新表格无差别（单元格仍为字符串）。回滚即回退提交。

## 9. 验证

### 9.1 行为与接口验证

**环境**

- 仓根执行；工具已按 `node scripts/prepare-phoenix-diagram.mjs --prepare` 准备，缺失即失败（C-4）。`CARGO` 表示 `mise exec --no-deps -- mbx`，`MF` 表示 `--manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline`。
- 批次引擎：`$CARGO build $MF --bin phoenix-diagram`，`E=tools/phoenix-diagram/target/debug/phoenix-diagram`。收口开发构建：`mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/elintp-p4-gaps-<后缀> --sync-dist`（新目录，耗时未知）。
- 产物：`OUT=local/phoenix-diagram/elintp-p4-gaps-out`（`local/` 不入库）。`finalize`/`static` 的结果读 summary 文件，不读 stdout。
- 看图由具备图像能力的 Agent 实际打开截图；没有图像能力时如实保留未完成。

**批内最小反馈**：`$CARGO test $MF --test chart <过滤词>`；单个 candidate `$CARGO run $MF --bin phoenix-diagram -- validate chart <candidate.json> --layout-json --json`。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **图解作者画两组前后对比与一份配额图，交给读者看图和复制数据。** 输入：作者写的三份 candidate——两系列竖向柱（系列 id `before`/`after`，单位 `ms`，en）；两系列折线（id `p50`/`p95`，含 `0.1` 与 `1234567`，zh-CN）；限额图（一行超限、一行未超、一行只有上限且省略行级单位，zh-CN）；另加 6 行单系列饼图（en，单位 `files`）。覆盖 G-01、G-02、D-01、D-02、WORK-F9/AC-5。结果：<br>• 四份 `finalize` 自动阶段全过，看亮暗两张后登记 `deliveryStatus: complete`：柱与折线图例为色块内单个键字母加「A · 名称」，不重叠；超限填充与「超限」图例色块比正常填充更醒目（亮暗都是）；仅上限行是虚线框，图例有「仅上限」；饼图第 6 扇区与列表色块清晰可辨<br>• 在 Chrome 打开柱状图阅读器，展开数据表，选中一行复制，得到「标签⇥4200⇥2600」这样的纯数字；英文列头为「Before (ms)」；折线表头为「p50（ms）」，单元格无键；饼图表头为「Value (files)」「Share (%)」，单元格无单位与 `%`<br>• 四份各取 `embedded` 静态图，自动阶段全过、看两张登记 complete；SVG 中 `data-phoenix-table` 与阅读器表格逐格相同<br>容易犯的错：作者照旧把单位写进数值（`"12ms"`），仍按原规则 `input/schema` 退出非 0、无 HTML | 1. `"$E" finalize chart $OUT/<name>.json $OUT/<name>.html --json`，读 `<name>.finalize-summary.json`<br>2. 打开两张 1440×900 截图，写审核记录，`"$E" review-record $OUT/<name>.html --record … --json`<br>3. Chrome 打开柱状图 HTML，展开「Data table」，选中第一行复制（CDP 短脚本或浏览器操作），记录剪贴板文本<br>4. `"$E" static chart $OUT/<name>.json $OUT/<name>.embedded.svg --json`，看图并 `review-record`<br>5. 取 SVG 中 `data-phoenix-table` 的 JSON，与阅读器 `details.chart-data` 单元格逐格比对<br>6. 错误输入运行第 1 步到新路径 | M1 交付齐备、批次引擎构建后 |
| V-2 | 局部 | 准入：E2E 暴露的缺陷防复发（G-01 的图例检查漏判）与限额分支规则。`tools/phoenix-diagram/tests/chart.rs` 经 `compile` 与 `check::check_html`：<br>• 多字符 id 两系列柱：图例色块文字等于键；把产物中色块文字改回 id 后 `check_html` 报 `artifact/semantic-text`（扩展 `equal_values_of_two_series_keep_separate_annotations` 或新增一例）<br>• 两系列柱与饼图的 `facts.table`：单元格为纯数字字符串；en 列头 `Before (ms)`、zh-CN 列头 `p50（ms）`；`pie_sectors_and_list_match_facts`（无单位，有占比列）补断言占比列头经 `chart.table.withUnit` 带 `%`、单元格为 `50` 而非 `50%`；`pie_percent_unit_omits_share` 补断言表格只有两列、数值列头经 `chart.table.withUnit` 带 `%`、单元格为 `45`（图面「45%」不变）<br>• 限额：仅上限行有 `rect.ch-span`、无 `rect.ch-fill`，长度比不变；「仅上限」图例当且仅当存在这类行；把 `ch-span` 改成 `ch-fill` 后检查失败（改 `limits_fill_ratio_limit_line_and_over_mark`）<br>• 既有用例中受本计划改变的期望按新行为更新<br>阅读器目录：五份键集合与占位符一致 | `$CARGO test $MF --test chart`<br>`mise exec --no-deps -- node scripts/assemble-phoenix-diagram-reader.mjs --check`<br>`mise exec --no-deps -- node --test tools/phoenix-diagram/tests/viewer/catalogs.test.mjs` | M1 交付齐备后 |
| V-3 | 阶段收口 | 兼容与改动范围回归（WORK-N3/AC-4、C-1、C-2）：<br>• 改动 crate 的 fmt、clippy 无告警；直接读取 chart 产物或目录的既有测试通过；M1 的 V-2 已通过且覆盖未变时直接引用，不重跑<br>• 十一类最小示例的图面指纹与两策略静态 SVG SHA-256 与原计划基线产物逐一相同<br>• 十二类最小示例两策略 `static` 自动阶段全过、体积比 ≤50% | 1. `$CARGO fmt --manifest-path tools/phoenix-diagram/Cargo.toml -- --check`；`$CARGO clippy $MF --all-targets -- -D warnings`；`$CARGO test $MF --test i18n --test check_anchors --test browser_contract`；`mise exec --no-deps -- node --test test/phoenix-diagram/reader-assets.test.js`。聚合命令 `npm run test:diagram` 另含发布、安装、准备与许可等和本次改动隔离的维护测试，不作退出前提<br>2. 收口开发构建；对十二类最小示例各运行 `deliver` 与两策略 `static`；用原计划 §9.1 的指纹命令与 `shasum -a 256` 对照 `local/phoenix-diagram/elintp-p4-base-out` 的基线产物；`wc -c` 求体积比<br>3. 构建后按 §12 还原 `dist/` 根部被改写的文件，`git status --short dist` 无本计划外差异 | M2，代码改完后 |
| V-4 | 真实使用验收 | 从干净用户状态连续完成（G-01、G-02、D-01、D-02 与 WORK-F9/AC-5 的本期部分）：<br>1. 新项目按 `docs/phoenix-diagram.md`「离线安装」装上收口构建；图解作者只凭安装后的 skill 文件，把自己手头两组可核对的数（如本仓两次提交各 skill 目录的文件数，系列 id 用英文单词）画成两系列横向柱状图，交付阅读器（看两张、登记）并取静态图（看两张、登记）；再画一份含仅上限行的限额图并交付<br>2. 文档作者用仓内 `skills/elintp` 把这两张静态图讲成一页零网络页面<br>3. 读者断网打开终稿，切换亮暗，展开数据表复制一行，粘贴结果为纯数字；图例键、超限色、仅上限样式一眼可辨<br>发现的用户可见问题修复后复验，或列给用户确认延期 | 1. 临时目录作项目：`node bin/xgent-skills.js install "$PROJECT" --no-phoenix-ui --providers=claude --diagram-release-dir="<V-3 的开发构建目录>"`<br>2. `PD="$PROJECT/.claude/skills/phoenix-diagram/scripts/phoenix-diagram"`，只读安装后的 `SKILL.md` 与 `references/`<br>3. elintp 用仓内 `skills/elintp`，角色只读两个 skill 的文件与公开文档<br>4. Chrome：DevTools Offline 后刷新、切换主题、展开表格、选中一行复制 | M2，V-3 通过后 |

V-1、V-4 中的作者与读者为 Agent 角色模拟，不代表真人可用性结论。

## 10. 里程碑与验收安排

M1 一批：四处修正共用同一引擎、同一组 candidate，批末一次构建、一次 V-1/V-2。M2 是收口。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 图例键、强调色槽、纯数字数据表、仅上限样式 | 无 | **落点**：§5.1–§5.4 的 `layout/chart.rs`、`render/chart.rs`、`check/chart.rs`；五语言目录两键与装配；`tests/chart.rs`<br>**同构**：色槽规则循环（`render/chart.rs` L58）；限额图例条目（`layout/chart.rs` L1370–L1392）与其 `legend_glyph` 分支；表格段的现有列头写法<br>单人连续实施，三份源码共享，不拆并行 | V-1、V-2；回写「实施进度」 |
| M2 | 阶段收口 | M1 完成 | 改动范围回归、十一类兼容比对、离线安装后的真实使用验收；发现的问题修复后只复验失败项与波及项 | V-3、V-4；回写「实施进度」 |

提交点：M1 通过后提交一次（引擎源码、目录、生成的模板与目录 JSON、测试、M1 记录）；M2 完成后最终提交。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 虚线框在暗色主题下对比度不足 | 仅上限行不易看见 | V-1 看暗色截图；不足时在 `.ch-span` 规则内调整描边变量或宽度，仍用支持清单内声明 | M1 批末 | 否 |
| 原计划基线产物 `local/phoenix-diagram/elintp-p4-base-out` 不入库，可能已被清理 | V-3 第 2 步无基线可比 | 缺失时在 `b4c7b568` 的干净 worktree 按原计划 §9.1「基线」重建到新目录，再比对 | M2 | 否 |
| 原计划 §5.3/§5.4/§5.8 的旧写法与本计划不同 | 读原计划的人按旧示例理解表格 | 本计划 §5 为现行规则；原计划只读，差异由报告与本计划追溯 | — | 否 |

- 最终状态：**Ready**。
- 定级理由：四项修正的行为、落点、样式、文案与检查已定，用户决定已登记；没有外部依赖或未决的范围、数据与契约问题。

## 12. 已知坑与历史教训

- `tools/phoenix-diagram/assets/template.html`、`tools/phoenix-diagram/assets/catalogs.json` 是生成物，改目录后运行装配脚本，`--check` 判过期（`tools/phoenix-diagram/viewer/README.md`）。
- chart `defs` 的规则会被静态转换重新解析，不引用 SVG 根或用清单外声明时 `static` 报 `static/style` 而阅读器正常（`render/static_style.rs` L242–L268、L629–L680）。
- 原 P4 看图时把色块里的系列 id 当作正常内容记录（[P4 M2 记录](ELINTP-VISUALS-P4.records/M2.md)「ins A · 新增行数」）：看图要核对色块内只有一个键字母，测试 candidate 必须用多字符 id。
- 饼图只有行数为 6、11、16… 时才用 `external`，四项饼图测不到 G-02 的收尾扇区。
- `--sync-dist` 会改写 `dist/` 根部三方许可文件，构建后 `git checkout --` 还原；离线安装需要同一次构建目录里的 `bundle/ENGINE.json`（包侧对应文件不入库），从构建目录原样复制、安装后删除（[P4 收口记录](ELINTP-VISUALS-P4.records/M5.md)「事 1」）。
- `finalize` 与 `static` 的 stdout 很大，读 summary 文件。
