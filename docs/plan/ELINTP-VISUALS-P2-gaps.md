# ELINTP-VISUALS-P2-gaps · XGENT-ai Skills · elintp P2 覆盖补齐（无图注图片可标注 · 终稿不内嵌资源快照）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-11 · 0d097e947ae07bec13002ddc4c51a4964e8e96ea · dirty；`skills/elintp/`、`test/elintp/` 相对 HEAD 无改动，dirty 仅为其他文档、经验与状态文件。
>
> 需求来源：[ELINTP-VISUALS PRD v1.2](../PRD-ELINTP-VISUALS.md) · P2（第 2 期）；补齐原计划 [ELINTP-VISUALS-P2](ELINTP-VISUALS-P2.md)，差异与决定见 [P2 差异核查](ELINTP-VISUALS-P2.prd-dev-gaps.md) 的 G-01、G-02 / D-01。
>
> 本期交付：**审阅页中没有图注的截图也能被标注；作者确认后的离线终稿只内嵌内容稿，不再内嵌第二份资源字节**。
> **开发范围：仅 `skills/elintp/` 及 `test/elintp/`；依据 PRD P2 的 `WORK-F4/AC-1、AC-6`、`WORK-N3/AC-2` 与用户 2026-11-11 的 D-01 决定**。
> 本期独特职责：只做原 P2 交付后的两处增量，审阅、局部重讲、跨会话续接与 loopback 提交契约保持不变。
> **顶层排除：不改审阅页与 result.json 的资源快照，不迁移已生成的 P2 终稿，不改内容稿语法、frontmatter、summarize、Phoenix 或其他 skill。**

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

- 最近更新：2026-11-11T23:47:22+08:00
- 当前进度：1/1 个里程碑完成
- 当前状态：已完成
- 最近完成：M1 · 无图注图片可标注、终稿不内嵌资源快照
- 下一步：M1 · 提交本计划改动并回填 SHA，随后由用户以 `docs/plan/ELINTP-VISUALS-P2-gaps.md` 发起代码评审
- 当前阻塞：无
- 代码基线：0d097e94

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-11T23:47:22+08:00 | 两项增量及文档已实现，V-1～V-4 全部通过 | [M1 记录](ELINTP-VISUALS-P2-gaps.records/M1.md) | 待提交 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

计划类型：既有模块增量；无持久化迁移。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F4/AC-1 | 功能 | PRD §3 WORK-F4 · P2；差异核查 G-01 | 任一内容块可标注；补齐空图注图片（含缺失图片）这一类块 | §5.1、§6；V-2、V-3 | 已确认 |
| WORK-F4/AC-6 | 功能 | PRD §3 WORK-F4 · P2；差异核查 G-02、D-01 | 终稿保留内容稿，不再内嵌资源快照 | §3、§5.2；V-1、V-2、V-3 | 已确认 |
| WORK-N3/AC-2 | 兼容性 | PRD §3 WORK-N3（P1 已交付，P2 不得退化）；summarize §5 的 2MB 上限 | 调用方得到的终稿体积回到 P1 水平；两参数 `render.mjs` 不变 | §5.2；V-1、V-2 | 已确认 |
| C-1 | 技术约束 | `skills/elintp/SKILL.md`「产出形态」 | Node 18+、仅标准库、单文件内联 | §5 | 已确认 |
| C-2 | 范围约束 | `AGENTS.md` skill 独立规则；本计划顶部 | 只改 elintp 与其定向测试；frontmatter、description 不变 | §1.2；V-4 | 已确认 |
| C-3 | 兼容约束 | D-01 新验收边界；WORK-F4/AC-3、AC-5 | 审阅页与 result.json 的 `elintp-assets`/`assets`、`--previous` 续接、`--assets-from <审阅页/result.json>` 行为不变；同一未改小节先按旧规则为既有可登记块编号，再给新增空图注图片追加 ID，旧结果与同轮浏览器草稿仍附着原块；已生成且带快照的 P2 终稿不迁移，仍可作 `--assets-from` 来源 | §3.1、§5.1～5.2；V-1、V-2 | 已确认 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 终稿资源快照 | 终稿只注入 `elintp-draft`；`elintp-assets` 只进审阅页 | 终稿体积回到 P1 量级；只拿终稿再次开审时需要原资源目录，或最近的审阅页 / result.json | 用户 2026-11-11 确认（差异核查 D-01） |
| D2 | 无图注图片的引文与编号 | 图注为空时，图片 `figure` 带 `data-ref="<原引用>"`，`quote` 取 `element.dataset.ref` 原值，不折叠空白、不截断；每小节先按旧规则编号既有可登记块，再按文档顺序为新增图片追加 ID | 引文精确等于内容稿中的原引用；升级续接不改变既有块 ID；有图注的图片输出字节不变，其他块引文规则不变 | WORK-F4/AC-1、AC-3、AC-5；`skills/elintp/assets/review.js` 的 `register` |
| D3 | 用终稿作 `--assets-from` 来源 | 页面没有 `elintp-assets` 时退出 1，提示改用审阅页或 result.json，输出文件不动 | 不含快照的新终稿给出清晰错误，不静默生成缺资源的页面；带快照的旧 P2 终稿仍可读取 | C-3；`render.mjs` 现有退出码约定 |

### 0.4 职责与事实所有权

|  | 审阅者浏览器 | elintp 脚本与页面 | 当前 Agent |
| --- | --- | --- | --- |
| 拥有 | 当前轮未提交标注 | 内容块目录与引文、审阅页资源快照、终稿构造 | 何时生成终稿、终稿交付说明 |
| 不拥有 | 终稿内容 | 不替作者确认、不迁移旧终稿 | 不手工删 HTML 节点做终稿 |

一句话：资源快照只属于审阅轮次的交接材料，终稿只是阅读产物。

### 0.5 明确不在本期

- **终稿资源去重或从显示内容还原资源**——D-01 未选；需要时另立需求。
- **迁移或重写已生成的 P2 终稿**——不处理；它们仍可读，也仍可作 `--assets-from` 来源。
- **summarize 的上限、调用方式或文档**——范围外零改动；本期只让 elintp 不再翻倍体积。
- **内容稿语法变化（如强制图注）**——不做；空图注仍是合法内容稿。
- **终稿体积基线或压测**——后续可选；本期只按 V-2 的代表性输入核对不再存第二份资源。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·缺口]** 图片 `figure` 在图注为空时输出空 `<figcaption>`（`skills/elintp/scripts/draft.mjs` 图片分支，`![...](...)` 正则允许空说明）；`review.js` 的 `register` 对 `figure` 只读图注文字，空引文直接不登记，因此该图没有标注按钮。缺失图片同样只有 `.missing-image` 文字和空图注 → 给空图注图片一个非空引文 → 否则 AC-1 对这类块不成立。
- **[已核实·缺口]** `renderPage` 无论模式都注入 `elintp-assets`（`skills/elintp/scripts/render.mjs` `parts.DATA`），终稿再存一份资源 base64。实测 4 张截图的终稿 2,469,998 字节，P1 渲染器 1,241,228 字节 → 终稿不注入 → 否则 summarize 2MB 上限下 P1 能提交的报告现在失败。
- **[已核实·足够]** `readAssetSnapshot` 对 JSON 读 `.assets`，对 HTML 用正则取 `elintp-assets`，取不到时 `validateAssets(null)` 抛出「资源快照须为…」并经 `main` 退出 1、不写输出。只需把 HTML 无快照的报错换成可操作的说明。
- **[已核实·足够]** `review.mjs` 只用 `renderPage` 的返回值 `assets` 写结果与下一轮，审阅模式继续注入快照，因此续接链路不受 D1 影响（`skills/elintp/scripts/review.mjs` `nextRound`、提交 `record`）。
- **[已核实·足够]** 服务端只要求 `quote` 为非空文字（`review.mjs` `validateItems`），无需改服务端。

### 1.2 拓扑与文件清单

| 路径 | 改动 |
| --- | --- |
| `skills/elintp/scripts/render.mjs` | final 模式不注入 `elintp-assets`；`readAssetSnapshot` 对 HTML 无快照给出 D3 报错；用法字符串写明「含快照的审阅页或旧 P2 终稿 HTML / result.json」 |
| `skills/elintp/scripts/draft.mjs` | 图片分支在图注为空（`caption.trim()` 为空）时给 `figure` 加 `data-ref` |
| `skills/elintp/assets/review.js` | 两遍登记：先按旧规则为既有块编号，再为新增空图注图片追加编号；新增图片的 `quote` 取 `figure.dataset.ref` 原值 |
| `skills/elintp/references/draft-format.md` | 第 5 行说明、「两类页面 `elintp-assets`」行、提取脚本、`--assets-from` 新旧终稿来源、终稿保留内容、§5.1 的 `quote` 例外定义同步 |
| `skills/elintp/SKILL.md` | 仅当正文仍写终稿带资源快照时同步一句；frontmatter 不动 |
| `test/elintp/render.test.js` | 按新契约改三个依赖终稿快照的用例，并在 round-trip 用例里加不含快照的新终稿作来源的退出 1 断言 |

不修改 `review.mjs`、`page.*`、`review.css`、`templates.mjs`、`test/elintp/review.test.js`、README 与其他 skill。

## 3. 数据模型与迁移

| 对象 | 关键字段与约束 | 生命周期/权威来源 | 查询与索引 |
| --- | --- | --- | --- |
| `elintp-assets` | 不变的 JSON 形态；**只出现在审阅页** | 每次 review 渲染创建 | `--assets-from <审阅页>`、提取脚本 |
| `elintp-draft` | 不变；终稿唯一的内嵌数据 | 每次 final 渲染创建 | 新会话取稿 |
| `result.json` `assets` | 不变 | 每次有效提交 | `--previous`、`--assets-from <result.json>` |
| 图片 `figure[data-ref]` ★ | 值为内容稿中的原引用字符串，经 HTML 转义；仅图注为空时出现 | 每次渲染 | 审阅脚本取引文 |

### 3.1 兼容与迁移

1. 已生成的 P2 终稿仍带快照，不迁移；`readAssetSnapshot` 的正则照旧能读，作 `--assets-from` 来源时行为不变。
2. 新终稿没有快照；只拿新终稿再次开审时，从终稿取内容稿，资源用原目录或最近的审阅页 / result.json。
3. 有图注的图片 HTML 字节不变；空图注图片多一个 `data-ref` 属性，对阅读、打印与无脚本显示无影响。
4. 回滚代码即恢复原行为，已生成页面不受影响。
5. 同一未改小节内，既有可登记块的 `blockId` 与旧 P2 完全一致；新增空图注图片只使用旧编号之后的 ID。旧 `result.json --previous` 与相同 `pageKey`/轮次的浏览器草稿均无需重映射；已改小节仍沿用现有清除旧标注的规则。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| `review.mjs` | 已核实足够 | 只经 `renderPage(..., { review })` 取审阅页与 `assets` | 零改动 | 不变 |
| summarize 等调用方 | 已核实足够 | 只读生成的 HTML 文件全文提交（`skills/summarize/SKILL.md` §5） | 零改动；终稿变小 | 不变 |
| 新会话恢复 | 已核实足够 | 审阅页 + result.json 已覆盖续接（`draft-format.md`「审阅、恢复与终稿」） | 文档改为从终稿只取内容稿 | 提取脚本在终稿上只写内容稿并提示资源来源 |

## 5. 核心机制

### 5.1 无图注图片的引文（G-01）

1. `draft.mjs` 图片分支：`caption.trim()` 为空时输出 `<figure data-ref="${escapeHTML(file)}">`，否则保持 `<figure>`；图片存在与缺失两种分支一致。
2. `review.js` 按原选择器与遍历顺序分两遍登记。第一遍完全沿用旧 `register` 判据：`figure` 取 `figcaption.innerText`，其余块取可见文字，折叠空白、trim 后截取开头 80 个 Unicode 字符；引文为空的块不占号。每个 section（含 `lead`）的既有可登记块仍从 `b1` 顺序编号。
3. 第二遍仅登记第一遍未登记且带 `data-ref` 的空图注图片；每小节从第一遍末号之后按图片文档顺序追加编号。新增图片的 `quote` 直接取 `element.dataset.ref` 原值，不折叠空白、trim 或截断，完整保留长路径和连续空白。例如同小节原有“空图注图片 → 段落 → 带图注图片”，旧两块仍为 `s1:b1`、`s1:b2`，新增图片为 `s1:b3`；不得在第一遍按回退引文占用旧编号。完成两遍目录登记后再创建控件、读取和展示旧标注，避免控件文字影响第一遍引文。
4. `draft-format.md` 的 `quote` 定义同步为：普通块是可见文字折叠空白后开头 80 个 Unicode 字符；空图注图片例外，使用内容稿中的完整原引用，不做文字规范化或截断。服务端、`pageKey`/轮次、`blockId` 格式与结果 schema 不变，无需迁移旧标注。

### 5.2 终稿只内嵌内容稿（G-02 / D-01）

1. `renderPage`：review 模式注入 `elintp-assets` 与 `elintp-review`；final 模式只注入 `elintp-draft`。返回值 `assets` 不变。
2. `readAssetSnapshot`：HTML 中没有 `elintp-assets` 时抛出「`<文件>` 没有资源快照；新终稿不含快照，请改用当前审阅页或 result.json。」，经 `main` 退出 1，输出文件不写。JSON 分支与快照校验不变。
3. `draft-format.md` 的提取脚本：`elintp-review` 存在时照旧写内容稿与资源快照；只有 `elintp-draft` 时只写内容稿，并在 stderr 说明资源需来自原目录或最近审阅页 / result.json。
4. CLI 用法与公开文档的 `--assets-from` 来源写明「含快照的审阅页或旧 P2 终稿 HTML / result.json」；带 `elintp-assets` 的旧 P2 终稿仍可读取，不含快照的新终稿才进入第 2 步错误分支。

## 6. 前端与交互

空图注图片在开启「标注内容」后与其他块一样出现「＋ 添加标注」，按钮 `aria-label` 带引文；其余控件、键盘、亮暗、移动端与打印规则不变。

## 8. 失败模式、发布与回滚

不引入新的运行期失败模式：变化只有输出内容与一条输入报错。发布只随 skill 分发源码与文档，无部署或开关；回滚即回退本计划提交。

## 9. 验证

### 9.1 行为与接口验证

批内最小反馈：`node --test --test-name-pattern="<用例名>" test/elintp/render.test.js`，或单次 `node skills/elintp/scripts/render.mjs <临时稿> <临时输出>`。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 局部 | `WORK-F4/AC-6`、C-3：新终稿不含 `id="elintp-assets"`，仍含精确 `elintp-draft` 与审阅快照中的图片/阅读器；用不含快照的新终稿作 `--assets-from` 退出 1、stderr 指向审阅页或 result.json、输出不变；带快照的旧 P2 终稿仍可读取；`--assets-from` 读审阅页与 result.json 的确定性及符号链接入口不变。准入：既有终稿快照用例因行为有意改变而失效，按 `@delivery_first` 改到当前行为 | `node --test test/elintp/render.test.js test/elintp/review.test.js` | M1 代码齐备后 |
| V-2 | 场景 E2E | `WORK-F4/AC-1、AC-3、AC-5、AC-6`、`WORK-N3/AC-2`、C-3：作者把含 4 张截图（其一无图注、另有一张空图注缺失图片）的已完成计划总结做成审阅页；缺失图片的原引用超过 80 个 Unicode 字符且含连续空格。无图注图片放在同小节既有可标注块之前，用旧 P2 建立原块标注；升级后分别验证旧 `result.json --previous` 续接与同轮未提交草稿重开（相同 Chrome origin/端口、内容稿字节、输出路径、round/pageKey，沿用该轮原 `--previous` 参数）：旧标注仍在原块，可编辑提交，新图片可独立标注且 ID 不冲突。真实 Chrome 提交两张空图注图片的标注，result.json 的 `quote` 与各自内容稿原引用逐字相等，长路径和连续空白完整保留；作者确认后用 `--assets-from <审阅页>` 在不含原图的空 cwd 生成终稿；终稿 `file://` 离线打开，已有截图完整、缺失图片仍显示缺失、无审阅残留，字节数与同稿两参数 P1 渲染器输出同量级（不再约两倍），可从中取回精确内容稿 | 临时目录；截图用 `docs/diagrams/agi-mode-evolution-loop.visual-check.*.png`；`node skills/elintp/scripts/review.mjs … --no-open` 后在 Chrome 打开实际 `REVIEW URL:`；升级前产物用 `git archive 0d097e94 skills/elintp` 解出的旧 P2 脚本生成；同轮重开前停掉自己启动的旧服务并用原实际 `--port`；P1 对照用 `git archive 1ee47f4b skills/elintp` 解出的 `render.mjs` | M1 代码齐备、V-1 通过后 |
| V-3 | 真实使用验收 | `WORK-F4/AC-1、AC-6`：全新上下文作者只读公开 `SKILL.md` 与格式参考，从审阅页恢复并交付终稿，再只凭新终稿按文档取回内容稿，说明资源需原目录或审阅页；不把终稿当快照来源 | 干净项目只复制当前 `skills/elintp/` 到 `.claude/skills/elintp/`；主 Agent 在真实 Chrome 扮演审阅者并读页；标为 Agent 角色模拟，不冒充真人 UAT | M1 最后执行，所有代码与文档改完后 |
| V-4 | 静态 | C-2：elintp frontmatter 字节不变；本计划 scoped diff 仅含 §1.2 路径；无空白错误；保留既有无关 dirty，不回退、不纳入提交 | `git diff --check -- skills/elintp test/elintp`；`git diff 0d097e94 -- skills/elintp/SKILL.md` 不含 frontmatter 行；`git diff --stat 0d097e94 -- skills/elintp test/elintp` 核对路径 | M1 提交前 |

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 无图注图片可标注、终稿不内嵌资源快照（唯一收口） | 无；原 P2 已提交（`924f8793`、`f2b205bd`） | 单批、单人。先改 `render.test.js` 三个依赖终稿快照的用例与不含快照的新终稿来源报错断言使其失败；再改 `render.mjs`（§5.2）、`draft.mjs` 与 `review.js`（§5.1，既有块先编号、新图片追加编号且保留完整原引用）、`draft-format.md` 的引文例外与新旧终稿来源说明、必要的 `SKILL.md` 句子；同构参照现有 `renderPage` 的 review/final 分支与 `register` | V-1、V-2、V-3、V-4 通过，用户可见问题已修复或经用户确认延期；回写「实施进度」 |

提交点：M1（全部完成）。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 已安装的旧版 SKILL 副本仍指导 Agent 用不含快照的新终稿作 `--assets-from` | 该调用退出 1 并给出改用说明，不产生错误页面；带快照的旧 P2 终稿仍可读取 | D3 报错文字；V-1 断言 | M1 | 否 |
| 开放问题 | 无 | D-01 已由用户确认 | 不适用 | 否 |

- 最终状态：**Ready**。
- 定级理由：两处增量的范围、验收边界与用户决定均已确定，改动落点与同构代码已核实，没有影响范围、安全、数据或外部契约的未决项。

## 12. 已知坑与历史教训

- `AGENTS.md` skill 独立规则：本计划只改 elintp，不改 summarize 或 Phoenix；summarize 的 2MB 上限只作验收参照。
- `AGENTS.md` 验证范围规则：只跑 elintp 两个测试文件、元数据核对与 §9 场景，不跑 `npm test` 或无关全套。
- `docs/plan/ELINTP-VISUALS-P2.code-review.md` F-01：CLI 主模块判断曾在符号链接路径下静默不执行；V-1 保留符号链接用例，改测试时不能删掉它，只换快照来源。
- `docs/plan/ELINTP-VISUALS-P1.records/M4.md`：审阅走 `REVIEW URL:`，终稿走 `file://`；V-2 不能用 curl 或退出码代替看图。
- `skills/elintp/references/draft-format.md` 多处写终稿带 `elintp-assets`、可作 `--assets-from` 来源；行为改变后必须同批改完，避免新会话按旧说明操作。
