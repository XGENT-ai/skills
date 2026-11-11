# PHOENIX-UI-EVIDENCE-CRAFT-P3 · XGENT-ai Skills · Phoenix UI 预览素材与交付图片处理（原生 `media`、`prep-image` 命令 · 素材只进 `.phoenix-ui/media/` · 联网边界终验）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-11 · `9b07ef0`（main）· dirty：未提交内容只有 `.xgent-ai/sdlc/` 台账与 `docs/plan/` 下的评审报告；引擎与 skill 源码无未提交改动，下文事实取自当前工作树。
>
> 修订基线：2026-11-11 · `b60ee18e`（main）· 按评审报告处置 F-01–F-14（见该报告「修订处置与复核」节）；处置引用的代码事实在 `b60ee18e` 逐一复核未变。
>
> 需求来源：[Phoenix UI 取证与工艺增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.2 · P3（第 3 期）：WORK-F7/AC-1–AC-4、WORK-F8/AC-1–AC-4、WORK-F9/AC-1–AC-5、WORK-N1/AC-4（WORK-N1 在本期终验）、WORK-N2/AC-3，以及每期都要满足的 WORK-N4/AC-2；另承接 [P2 需求与开发差距报告](PHOENIX-UI-EVIDENCE-CRAFT-P2.prd-dev-gaps.md) 的 D-01（typeset 断行指引补一句，用户 2026-11-11 决定并入下期计划）。
>
> 本期交付：**两个随 engine 分发的原生命令。`phoenix-ui media` 从 11 个公开信源检索开放授权素材，给出带编号、许可分级与尺寸的联系表，把选中的条目取到 `.phoenix-ui/media/` 并留下来源清单，交付前核对产品没有带走素材；`phoenix-ui prep-image` 把交付图片按显示宽度的 2 倍批量导出、生成 `srcset`/`sizes` 片段并保留内嵌来源。同时把页面检测与取证时浏览器自身的后台联网按启动参数关掉，完成联网边界终验**。
> **开发范围：`tools/phoenix-ui` 引擎（`crates/cli` 新增 `media/`、`prep_image.rs` 与命令路由；`crates/context/src/embed_prompt.rs` 公开来源读写函数；`crates/browser/src/cdp.rs` 的浏览器启动参数；`crates/detect/src/lib.rs` 的总帮助；测试、oracle 用例与夹具）、`skills/phoenix-ui/src` 中的 new-work、visualize、optimize、typeset、SKILL 源文件与 evals、`docs/phoenix-ui.md`、`README.md`、`tools/phoenix-ui` 的第三方声明、`scripts/smoke-phoenix-ui.mjs`，以及 PRD 中 WORK-N1 的一句澄清；依据见 §0**。
> 本期独特职责：Phoenix 现在只要求“用真实、核实过的图片”，没有检索工具也没有许可分级；交付图片只有原则没有命令。本期第一次让引擎主动访问第三方素材接口，因此同时把“哪些命令会联网、联到哪里”落成可验证的边界。
> **顶层排除：素材不进入产品——不写项目资产目录、不进产品代码、许可清单与署名不带进产品文件；不输出有损 WebP 与 AVIF；不引入 ffmpeg、libwebp 或任何新的第三方包；不改 feature-demo、review-ui 等独立 skill**。

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

- 最近更新：2026-11-12 01:35 +08:00
- 当前进度：3/5 个里程碑完成
- 当前状态：M1–M3 实现与批末检查完成；M4 已渲染且静态检查通过，宿主演练进行中
- 最近完成：M2 · 素材取用与联网边界
- 下一步：继续 M4 V-7，完成后保存 skill 与生成物提交；继续 M5 阶段收口
- 当前阻塞：无（V-11 推送授权在候选就绪后申请）
- 代码基线：`ea0e5a3`（main）；M1–M3已提交，M4文本与生成物尚未提交

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M4 | 进行中 | 2026-11-12 01:35 +08:00 | skill六文件与公开命令文档已写；待渲染、V-6/V-7 | [M4 记录](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M4.md) | 待提交 |
| M1 | 已完成 | 2026-11-12 01:35 +08:00 | 11源检索、稳定编号、联系表与真实V-1/V-2通过 | [M1 记录](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M1.md) | `2d5deab` |
| M3 | 已完成 | 2026-11-12 01:35 +08:00 | 批量图片导出与来源保留，V-4目视及V-5、lint通过 | [M3 记录](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M3.md) | `ea0e5a3` |
| M2 | 已完成 | 2026-11-12 01:40 +08:00 | V-1/V-2/V-3及最终lint通过；Chrome后台请求清零 | [M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M2.md) | `49be9b9` |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID（尚未录入 SPMS，不是真实需求 key）。WORK-N1 的终验期是 P3：AC-1–AC-3 已在 P1 交付，本期完成 AC-4 并在收口确认四条同时成立。WORK-N2、WORK-N4 的终验期是 P4，本期只承接所列 AC，完成后不转“已上线”。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F7 | 功能 | PRD §3 WORK-F7/AC-1–AC-4 · P3 | 按关键词与类别检索 11 个信源；每条带编号、标题、信源、原图尺寸、许可与使用分级，编号在同一素材区内跨次检索不变；可只要可上线条目；联系表带编号、分级、尺寸，并给出四边主色、是否浅色纸底、可压字区域；单个信源不可用时注明、其余照常，全部不可达时明确报网络不可用 | §5.1–§5.7、§5.12；V-1、V-2、V-9（A1、A3） | 已确认 |
| WORK-F8 | 功能 | WORK-F8/AC-1–AC-4 · P3 | 按编号取到 `.phoenix-ui/` 素材区，核对真实宽高与解码完整性；来源清单逐条记录文件名、标题、来源页、许可、署名要求与取用日期；目标在 `.phoenix-ui/` 以外时拒绝；交付前核对产品不引用素材区文件、产品文件里没有素材的许可清单或署名；贴图可一并取颜色、粗糙度、法线；视频同时得到首帧静态图 | §5.8–§5.11、§6.1；V-1、V-2、V-7、V-9（A1、A3） | 已确认 |
| WORK-F9 | 功能 | WORK-F9/AC-1–AC-5 · P3 | 按显示宽度导出 2 倍宽，不足不放大并注明；不透明图出 JPEG，带透明通道的图保留透明通道；多宽度版本与 `srcset`/`sizes` 片段；原图字节不变，输出带相同的内嵌来源且 `embed-prompt --scan` 不报缺失；按比例裁切、裁透明边；不可解码的文件单独失败 | §5.13–§5.15、§6.3；V-4、V-5、V-9（A2） | 已确认 |
| WORK-N1 | 安全/隐私 | WORK-N1/AC-4 · P3（AC-1–AC-3 属 P1，本期持续满足并终验） | context、hook 与源码文件检测不发起外网请求；页面检测与取证只请求目标页面及其自身资源，不发起素材检索；这些命令运行中不自动更新或下载；素材检索与取用只在命令被显式调用时联网，对话中可见调用记录 | §5.16、§6.1、§7；V-3、V-8、V-9（A3、A4）、V-10 | 已确认 |
| WORK-N2 | 兼容/分发 | WORK-N2/AC-3 · P3（AC-1–AC-2 属 P1，AC-4 属 P4） | 素材检索、取用与图片处理命令在 macOS（arm64/x64）与 Linux（x64/arm64）可用，无需 Rust；engine 缺失或版本不符时返回固定包版本的补装命令 | D1、§7；V-8、V-11 | 已确认 |
| WORK-N4 | 可维护 | WORK-N4/AC-2（每期；AC-3 属 P4） | 动 skill 文件前 §0.6 的整合清单已到本期范围；本期 diff 与清单逐项核对，新冲突在继续改行为前解决；oil-ui 的 MIT 声明覆盖本期新增的改编范围 | §0.6、§6.6；V-6、V-10 | 已确认 |
| P2-D01 | 功能（承接） | [P2 差距报告](PHOENIX-UI-EVIDENCE-CRAFT-P2.prd-dev-gaps.md) D-01、F-01；用户 2026-11-11 决定 | typeset 中文断行指引补一句：按语义断句优先用 `<br>`，整段包进 inline-block 的 span 会让该段退出 `xgent/cjk-*` 检测与字体备注 | §6.4；V-6 | 已确认 |
| C-1 | 约束 | AGENTS.md `@skill_tool_independence`；PRD N14 | 只有 phoenix-ui skill 随工具一起改；design-reference、review-ui、feature-demo 等独立 skill 不因本期改动 | §0.5、§1.2；V-10 | 已确认 |
| C-2 | 约束 | AGENTS.md §7「Reference Skill Integration Gate」；PRD §5.3 C6、C7、C14 | 借鉴 oil-ui 的规则与流程先列清单再改；Phoenix 现有原则是基线，冲突项调整或拒绝 | §0.6 | 已确认 |
| C-3 | 约束 | PRD D10、N16；用户 2026-11-09 决定 | 素材只用于设计稿与方案预览；只进 `.phoenix-ui/` 素材区，许可与来源记录只留在设计阶段 | D3、§5.10、§5.11、§6.1 | 已确认 |
| C-4 | 约束 | skills/phoenix-ui/src/SKILL.src.md:15；docs/phoenix-ui.md:28 | engine 在 context、检测与 hook 中不更新、不下载；新命令不得给这些路径带来联网 | §5.16；V-3 | 已确认 |
| C-5 | 约束 | scripts/check-phoenix-deps.mjs:457-466 | `crates/`、`browser-bundle/` 下新增的 js、css、html、字体文件（`tests/` 目录除外）会进入许可清单 | 首帧脚本写成 Rust 字符串常量（§5.9）；夹具放 `tools/phoenix-ui/tests/fixtures/media/` | 已确认 |
| C-6 | 约束 | tools/phoenix-ui/Cargo.toml:38（`panic = "abort"`） | 外部输入（接口 JSON、下载内容、图片解码、索引文件）一律走 `Result`，不 `unwrap`、不越界索引 | §5；V-2、V-5 | 已确认 |
| C-7 | 约束 | tools/phoenix-ui/THIRD-PARTY.md；scripts/check-phoenix-deps.mjs:518-540 | 不新增第三方包；`Cargo.lock` 只新增依赖边，改后用 `check-phoenix-deps.mjs --write` 重算清单与审计结果 | D1、M5；V-8 | 已确认 |
| C-8 | 约束 | AGENTS.md「Working in This Repository」接口复述同步 | 新命令在总帮助、docs/phoenix-ui.md、相关 reference 与 README 中完整描述 | §6；V-6、V-10 | 已确认 |
| C-9 | 约束 | docs/phoenix-ui.md「维护者」节 | 改过 `skills/phoenix-ui/src` 后运行 `bun scripts/render.mjs` 并提交生成的 skill | M4；V-6 | 已确认 |
| C-10 | 约束 | tools/phoenix-ui/tests/oracle/README.md:1-20 | 每个 verb 都有端到端 golden；总帮助变化要重录 `cli-help`，逐个审阅 diff | M5；V-8 | 已确认 |
| C-11 | 约束 | AGENTS.md「Working in This Repository」 | 留在当前分支；推送只在用户授权时进行 | §10 提交点、V-11 | 已确认 |
| A-1 | 假设 | 2026-11-11 本机实测（§1.1）；信源无 SLA（PRD §4） | 10 个信源的接口按 §5.2 所列写法可用；美国国会图书馆（LoC）在本机（经代理与直连）对非浏览器客户端返回 403，其他网络下是否可用未知 | V-1 实测并记录每个信源的结果；不可用的信源按 WORK-F7/AC-4 报出，不阻塞；责任人：实施者；最晚 M2 批末 | 开放 |
| A-2 | 假设 | macOS Google Chrome 157 上实测（§1.1）；Linux 与其他浏览器未验证 | 无头浏览器能解码素材视频并经 canvas 取出首帧；§5.16 的启动参数在 Chromium、Edge、Brave 上同样关闭后台联网 | V-2 的 WebM 夹具在四平台运行（V-11）；H.264 视频在不带该解码器的 Chromium 上按 §5.9 报首帧未产出；其他浏览器的后台联网按 §11 披露；责任人：实施者；最晚 M5 | 开放 |

未明确要求的 NFR（检索耗时、下载速度、输出体积上限等）按常规工程做好，不建基线、不设专项检查（§0.5）。

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 形态 | 原生引擎新增两个 verb：`media`（`crates/cli/src/media/`★）与 `prep-image`（`crates/cli/src/prep_image.rs`★）。HTTPS 用已有的 `impeccable_context::http::agent_builder`，图片编解码用已在依赖树里的 `image` 0.25，拼图与标注用 `impeccable_comp::raster`，视频首帧借 capture 的浏览器与静态服务器 | 不新增第三方包，`Cargo.lock` 只多依赖边；随现有 engine 分发；启动器无 verb 白名单，无需改 | PRD D2；WORK-N2；C-7；crates/context/src/http.rs:38；crates/comp/Cargo.toml:24 |
| D2 | 命令名（PRD Q1） | `phoenix-ui media`（子命令 `search`、`preview`、`fetch`、`audit`）与 `phoenix-ui prep-image`，即 PRD 的工作名 | 文档与验收不再改名 | PRD §5.2 Q1 |
| D3 | 素材区 | 默认 `.phoenix-ui/media/`（相对当前目录）；`--dir` 只能落在当前目录的 `.phoenix-ui/` 之内；首次创建素材区时写入内容为 `*` 的 `.gitignore` | 素材、编号索引与来源清单只留在本机，不会被 `git add -A` 带进仓库；换机器后按来源清单重新取用；项目要入库可自行删掉该文件 | WORK-F8/AC-2；C-3；用户 2026-11-11 确认 |
| D4 | 编号 | 每个信源一个字母前缀加递增序号（如 `C12`），登记在素材区的 `index.json`；同一条目（信源 + 信源内 ID）再次出现沿用原编号；读改写在文件锁内完成 | 编号短，能标在联系表上；清空素材区后序号重新开始 | WORK-F7/AC-1；oil-ui curate-media.mjs:415-423 |
| D5 | 使用分级 | 字段 `use` 取 `page`（可上线）、`check`（需核对）、`reference`（仅参考），由条目自带的许可文字按 §5.5 的规则得出；分级只说明条目自身的许可，不决定素材能进产品 | 与 oil-ui 的取值一致，文档与终端用中文名 | PRD §2.1「使用分级」；curate-media.mjs:162-168 |
| D6 | 信源与类别 | PRD 所列 11 个信源全部保留；类别到信源的映射见 §5.3，其中「测绘图纸」在 LoC 之外加查 Wikimedia Commons | LoC 被拦的网络下该类别仍有结果；LoC 不可用时照常报出 | PRD WORK-F7；§1.1 实测；用户 2026-11-11 确认 |
| D7 | 联系表与观感数据 | 用 `impeccable_comp::raster` 在 Rust 里拼成一张 PNG：每格一张缩略图、编号、分级色块与尺寸；四边主色、纸底与可压字区域由缩略图像素算出 | 不需要浏览器和 ffmpeg；格内文字只有 ASCII（内置 5×7 点阵字体），标题看 JSON 与终端输出 | WORK-F7/AC-2；crates/comp/src/raster.rs:224-265；ADR-2 |
| D8 | 取用校验 | 图片整张解码成功才算可用，宽高取解码结果；信源给了字节数的（Poly Haven）核对字节数；内容类型是 HTML 或 JSON、文件签名不是已知图片或视频的一律拒收 | 不校验 MD5（依赖树里没有 MD5 实现，解码完整性已覆盖损坏与截断） | WORK-F8/AC-1；C-7 |
| D9 | 视频首帧 | 用本机无头浏览器打开素材区里的视频，暂停在第 0 帧画到 canvas 导出 PNG；浏览器不可用或不支持该编码时该视频取用失败，视频与首帧均不落位、不登记，退出码 2 | 不引入 ffmpeg；首帧就是视频的第一帧，开头是黑场的视频得到黑图 | WORK-F8/AC-4；PRD D9；ADR-2 |
| D10 | 来源清单 | 素材区的 `sources.json`，每个取用的文件一条；清单不写到素材区之外 | 清单同时是交付前核对的依据（文件摘要、来源页、署名文字） | WORK-F8/AC-1；C-3 |
| D11 | 交付前核对 | 新增 `media audit`：在项目文件里查三件事——引用素材区路径、与素材文件内容相同的副本、来源页链接或署名文字；有发现退出码 2 | WORK-F8/AC-3 有确定性的检查，不靠 agent 手工 grep；副本按 SHA-256 比对 | WORK-F8/AC-3；PRD D2（确定性能力做成命令） |
| D12 | 输出格式 | 不透明图输出 JPEG（质量默认 82），带透明通道的图输出无损 WebP；不提供格式选项 | 现有依赖只有 JPEG 编码与无损 WebP 编码；不透明图比有损 WebP 大约两到三成 | WORK-F9/AC-1；ADR-1；用户 2026-11-11 确认 |
| D13 | 输出带来源 | PNG 的 tEXt、JPEG 的 COM 段、其他格式的 `<文件>.json` 旁注，全部沿用 `embed-prompt` 现有写法；本期只有 JPEG 与 WebP 两种输出，WebP 用旁注 | `embed-prompt --read` 与 `--scan` 对输出直接可用；WebP 的旁注文件要随图片一起移动 | WORK-F9/AC-3；crates/context/src/embed_prompt.rs:116-140、:317-323 |
| D14 | 宽度写法 | `--width <显示宽>` 导出 2 倍宽；`--widths <像素宽,…>` 列出要额外导出的像素宽；两者可同给，至少给一个；任何目标宽都不超过原图宽 | 与 PRD S11 一致：显示宽 600 得到 1200 宽的主输出 | WORK-F9/AC-1、AC-2；oil-ui media.md:156 |
| D15 | 联网边界 | 引擎里只有 `media` 访问素材信源；浏览器启动参数补上已验证的关闭项，把 Chrome 自身的后台请求从 6 个主机压到 1 个并继续尝试清零；仍有残余时逐条写进 docs 与验收记录，并在 PRD 的 WORK-N1 补一句澄清 | 页面检测与取证期间不再触发 Chrome 的组件更新下载、账号与搜索请求；不屏蔽主机，目标页面自己的资源照常加载 | WORK-N1/AC-4；§1.1 实测；用户 2026-11-11 确认 |
| D16 | 测试替身的位置 | 信源是真正的外部依赖：真实检索由 Agent 在批末与收口实跑；CI 里的测试把信源地址指向本机 `tiny_http`，返回录下来的真实响应样本 | CI 不依赖第三方接口的可用性；地址注入只存在于代码内部的 `Endpoints` 参数，没有对外的开关或环境变量 | crates/context/src/generate_image.rs:24（`run_with_api_base` 同一做法） |
| D17 | skill 落位 | 素材写进 new-work.md 的真实图片条款与完工段、visualize.md 的设计稿参考图条款；图片处理写进 optimize.md 与 visualize.md 的转换一句；SKILL 源文件的联网一句补上 media | 不改方向流程、comp 合同、评审处置词与 `context` 的 IMAGE_TOOLS 指令 | §0.6；§6 |
| D18 | 收口验收 | 本机 darwin-arm64 完成开发构建、冒烟、Phoenix UI 全量回归与真实使用验收；四平台与 Node 18 用现有 Phoenix CI（push 触发），推送须用户授权 | 未获授权推送时 V-11 记未运行，收口不宣称 WORK-N2/AC-3 通过 | C-11；docs/phoenix-ui.md「维护者：统一候选与本地交接」 |

### 0.3 ADR-lite

#### ADR-1：交付图片用现有依赖在 Rust 里处理，输出 JPEG 与无损 WebP

- 状态：Accepted（用户 2026-11-11 确认）
- 背景与驱动：WORK-F9 要求批量缩放、裁切并保留来源；WORK-N2 要求四平台无需 Rust、不新增 Python 依赖；PRD §4 的前置表没有把浏览器列为 WORK-F9 的依赖。引擎已有 `image` 0.25.10（JPEG、GIF、WebP 解码，JPEG 编码，WebP 只有无损编码）与 `png` 0.18。
- 备选：
  1. 现有依赖：不透明图出 JPEG，透明图出无损 WebP。不加包，不需要浏览器。
  2. 引入 libwebp（C 库）出有损 WebP：体积最小；四个平台的 CI 都要能编译 C 依赖，许可清单与依赖审计多一个包。
  3. 照 oil-ui 用无头浏览器的 canvas 编码（prep-image.mjs:187-195）：不加包也能出有损 WebP；命令从此依赖本机浏览器，色彩管理与元数据由浏览器决定。
  4. 调用 `context` 探测到的系统转换器（cwebp、sips、magick）：不同机器结果不同，没装就不能用。
- 决策：选 1。
- 正面后果：命令离线、确定、四平台一致；EXIF 方向与 ICC 色彩配置都能在同一条解码路径上处理。
- 负面/中性后果：不透明图比有损 WebP 大约两到三成；`image` 的 JPEG 编码器是基线编码，没有 mozjpeg 级别的优化；`crates/cli` 直接依赖 `image` 并打开 `png` 特性，经特性合并 `crates/comp` 里的 `image` 也带上 PNG 解码，但 comp 对 PNG 一律先走自己的 `decode_png`（crates/comp/src/png_io.rs:128-131、:161-168），行为不变。
- 重新评估触发：`image-webp` 提供有损编码；用户要求有损 WebP 或 AVIF；实际交付体积成为问题。

#### ADR-2：联系表、观感数据与完整性校验不用 ffmpeg；视频首帧用无头浏览器

- 状态：Accepted
- 背景与驱动：oil-ui 的联系表、观感数据、宽高探测、完整解码与视频首帧全部调用 ffmpeg/ffprobe（curate-media.mjs:506-553、:604-611、:687-697），没装时这些能力缺失；PRD D9 已定不新增 ffmpeg 依赖。
- 备选：
  1. 联系表与观感数据用 `raster` 与 `image` 在 Rust 里做；视频首帧用本机无头浏览器。
  2. 联系表也在无头浏览器里排版截图（capture 派生图的做法，crates/cli/src/capture/derived.rs:155-226）：标签能写中文标题；检索从此依赖浏览器，而 PRD 没有把浏览器列为 WORK-F7 的前置。
  3. 视频首帧改用信源自带的缩略图：NASA 的 `~medium_1.jpg` 等是按间隔抽的帧，不是首帧，其他信源没有视频。
- 决策：选 1。实测（2026-11-11，macOS，Google Chrome 157）：对 NASA 的 H.264 视频（480×270，由不支持 Range 的本机 HTTP 服务器提供），页面里的 `<video muted preload="auto">` 在 `loadeddata` 后画到 canvas 并 `toDataURL` 成功，得到的画面与 `ffmpeg -frames:v 1` 的首帧一致（该视频首帧是黑场）。
- 正面后果：检索、联系表、取用图片与贴图都不需要浏览器；只有视频首帧需要。
- 负面/中性后果：联系表格内文字只有 ASCII；不带 H.264 解码器的 Chromium 取不出 MP4 首帧（按 D9 报出）；`file://` 页面里 canvas 会被污染，必须经本机 HTTP 提供视频（§5.9）。
- 重新评估触发：需要在联系表上显示中文标题；视频首帧在常见环境里大面积取不出。

### 0.4 职责与事实所有权

|  | `phoenix-ui media` | `phoenix-ui prep-image` | Phoenix 主代理流程（skill 文本） | 素材信源（外部） |
| --- | --- | --- | --- | --- |
| 拥有 | 检索到的条目与其许可文字、分级、编号、联系表与观感数据、取到素材区的文件及其校验结果、来源清单、交付前核对的发现 | 输出文件的像素、格式、尺寸与随图来源、处理报告与 `srcset` 片段 | 什么时候需要真实素材、选哪张、怎样用在设计稿与预览里、产品用什么图、发现后怎样清理、向用户说明分级 | 条目内容、许可声明与接口可用性 |
| 不拥有 | **不判断肖像权、商标或具体商业用途；不把任何文件写到 `.phoenix-ui/` 之外；不改产品文件** | **不改原图；不判断图片该不该上线；不做性能诊断** | **不把素材区文件放进产品，不把分级当作可以上线的许可** | — |

一句话：`media` 只把“这条素材从哪来、许可怎么写、文件是不是好的”说清楚并把它关在素材区里；用不用、产品放什么图，归主代理与用户。

### 0.5 明确不在本期

- **素材用于产品时的许可核对、付费或需要密钥的图库、肖像权与商标判断**——PRD N16 与 WORK-F7 边界，明确不做。
- **有损 WebP、AVIF 输出与格式选项**——ADR-1，本期放弃；触发条件见该 ADR。
- **绕过信源的访问限制**（伪装浏览器、经浏览器抓取 LoC）——不做；被拦的信源按不可用报出。
- **直接按 URL 取用未检索过的素材**——不做；取用只认索引里的编号。
- **Poly Haven 模型文件（glTF）下载**——只取渲染预览图。
- **信源目录缓存、检索并发与耗时调优、输出体积上限**——后续可选；本期每次检索直接请求信源，响应体读取上限 16 MB。
- **`embed-prompt` 把来源写进 WebP 文件内部**——不做，WebP 沿用旁注文件（D13）。
- **`context` 的 IMAGE_TOOLS 指令、`generate-image`、方向流程、comp 门禁、完工评审的处置词与输出合同**——不改。
- **P4 的需求**（WORK-F10–F14、WORK-N2/AC-4、WORK-N4/AC-3）——由 P4 计划承接；README 里的完整命令清单属 WORK-N4/AC-3。
- **design-reference、review-ui、apply-ui-review、feature-demo 等独立 skill**——不改（C-1）。
- **发布、npm 版本与 SPMS 录入**——按维护流程另行执行。引擎源码已变，下次发布不能复用旧引擎。
- **Windows**——沿用支持矩阵的延期。

### 0.6 参考整合清单（动 skill 与命令前核对；WORK-N4/AC-2、C-2）

参考对象是本机 `local/oil-ui-main`（oil-ui 0.33.2，MIT；目录无 `.git`，只在本机可用）。Phoenix 现有原则为基线；整合不改变原则、立场与调用/授权策略。M1、M3、M4 动对应文件前按本表读一遍当前目标段落；实施中出现表外的行为差异，先改本表并解决冲突再继续。

| # | 目标文件/规则 | 现有行为 | 参考来源/段落 | 采用、调整或拒绝 | 对原则、调用、接口与验收的影响 | 核对 |
| --- | --- | --- | --- | --- | --- | --- |
| I-1（PRD C6） | 新 verb `media`（`crates/cli/src/media/`★） | 无检索工具 | `scripts/curate-media.mjs`：信源请求与映射（:216-401）、类别任务（:424-440）、相关性与混排（:150-160、:402-414、:452-464）、分级（:162-168）、编号（:415-423）、观感数据（:509-536）、下载校验（:576-603）；`references/media.md`:126、:130-132；`references/tools.md`:48-65 | 采用：信源写法、类别、分级规则、相关性过滤与混排、稳定编号、观感数据的算法与阈值、贴图三种图、单信源失败隔离、下载的体积上限与签名检查。调整：素材只进 `.phoenix-ui/media/` 并自忽略（D3）；来源清单为 `sources.json` 且不出素材区（D10）；联系表只出 PNG、在 Rust 里拼（D7）；完整性用解码判定（D8），视频的完整性以首帧解码为准（§5.8）；原图尺寸在检索阶段取得（§5.2 的字段、跟随请求或头部读取；oil-ui 只对部分信源带尺寸并允许缺省）；视频首帧用无头浏览器（D9）；测绘图纸加查 Commons（D6）；不缓存 Poly Haven 目录；新增 `audit`（D11）。拒绝：`--out` 任意路径与按 URL 直取（curate-media.mjs:471-492）、把素材存进上线的 `assets/` 并在页脚署名（media.md:37、:132 末句）、oiloil 积分生图 | 新公开接口；联网边界见 §5.16；不改“brief 优先”与方向流程 | V-1、V-2、V-10 |
| I-2（PRD C6） | `skills/phoenix-ui/src/reference/new-work.md`:139（真实图片条款）与 :158 一段（完工前的来源扫描）；`visualize.md`:11（设计稿的参考图） | “Use real, verified imagery when the brief implies it… Verify stock URLs resolve.”；设计稿用现有页面截图作参考图；上线图片都要有来源 | media.md:37「复用、生成，还是交给代码」；tools.md:48-65「检索和下载真实素材」 | 调整：设计阶段需要真实图像时可调用 `media`，每次调用在对话里说明在找什么；取到的素材只作设计稿的参考图或放进 `.phoenix-ui/` 下的预览页；交付前跑 `media audit` 并清掉发现。拒绝：让素材进入产品、把分级当作上线许可 | 不改“真实、核实过的图片”对产品的要求，也不改来源（provenance）规则：产品里的图仍须是项目拥有或获得的图片或生成的 plate，并带来源 | V-6、V-7、V-9（A3） |
| I-3（PRD C7） | 新 verb `prep-image`（`crates/cli/src/prep_image.rs`★）；`skills/phoenix-ui/src/reference/optimize.md`:28-42；`visualize.md`:44 | optimize 只有原则（现代格式、合适尺寸、`srcset`、80–85% 质量）；转换靠 `context` 报告的系统转换器 | `scripts/prep-image.mjs`：处理顺序与不放大（:22、:179-193）、透明判定（:170-175）、片段格式（:269-271）、单文件失败隔离（:216-224、:282）；media.md:156-157 | 采用：先裁透明边、再按比例裁切、再缩放的顺序，不放大，按像素判断有无透明，片段格式，失败不拖累整批。调整：`--width` 为显示宽、导出 2 倍（D14）；在 Rust 里处理、不需要浏览器，输出 JPEG 与无损 WebP（ADR-1）；输出带与原图相同的来源（D13）；应用 EXIF 方向并保留 ICC；重跑覆盖自己上次的输出。拒绝：`--format`、`--focus`、编码结果不比原图小时改为复制原图（prep-image.mjs:252-266） | 新公开接口；optimize 的性能诊断流程与 IMAGE_TOOLS 指令不变 | V-4、V-5、V-6 |
| I-4（PRD C14） | `tools/phoenix-ui/NOTICE.md`:19-25、`tools/phoenix-ui/THIRD-PARTY-NOTICES.md`:59-87 | oil-ui 的改编范围只写了 capture 与走查子代理 | `local/oil-ui-main/LICENSE`（MIT，第 3 行版权） | 采用：改编范围补上 `media` 与 `prep-image`，MIT 全文与版权行不动 | 许可合规（WORK-N4/AC-1 持续满足） | V-8、V-10 |
| I-5（PRD C13 中与本期相邻的拒绝项） | oil-ui 的 `SOURCES.md` 入库与页脚署名、生图积分、把参考素材放进上线目录 | — | media.md:37、:132、:157 | 拒绝，见 PRD §1.3 N6、N16 | 无改动 | V-10 |

P2-D01 不是参考整合项，随 M4 的 skill 改动一并带上（§6.4）。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·足够] 命令路由**：`run()` 是一个按 verb 匹配的分支（tools/phoenix-ui/crates/cli/src/main.rs:37-134），`capture` 在 :54；路径形状的第一个参数会落到 `detect`（:121），新 verb 的分支要排在它之前。总帮助是 `ROOT_USAGE`（crates/detect/src/lib.rs:53-68），由 oracle 的 `cli-help` golden 锁定。verb 函数形如 `fn(&[String], &mut Io) -> i32`，只有 `main()` 退出进程。
- **[已核实·足够] HTTPS 客户端**：`impeccable_context::http::agent_builder()` 返回装好信任库的 `ureq::AgentBuilder`：系统证书库与内置 Mozilla 根证书的并集，并读取 `ALL_PROXY`、`HTTPS_PROXY`、`HTTP_PROXY`（crates/context/src/http.rs:38-42、:62-68）。目前唯一的调用方是 `generate-image`（crates/context/src/generate_image.rs:75），它用 `run_with_api_base` 把接口地址作为参数传入以便测试（:20-24）。`crates/live` 里的 `ureq` 只访问本机。锁文件里已有 ureq 2.12.1、rustls 0.23.46、ring 0.17.14，没有其他 HTTP 客户端。
- **[已核实·足够] 图片能力**：`crates/comp` 依赖 `image = { version = "0.25", default-features = false, features = ["jpeg", "gif", "webp"] }` 与 `png = "0.18"`（crates/comp/Cargo.toml:20、:24）；锁定版本 image 0.25.10、image-webp 0.2.4。本机 registry 源码核对：`JpegEncoder::new_with_quality`（image-0.25.10 src/codecs/jpeg/encoder.rs:398）、JPEG/PNG/WebP 编码器的 `set_icc_profile`（jpeg/encoder.rs:752、png.rs:780、webp/encoder.rs:96）、解码器的 `orientation()` 与 `icc_profile()`（io/decoder.rs:53；jpeg/decoder.rs:86、:140）、`DynamicImage::apply_orientation`（images/dynimage.rs:1161）、默认解码上限 512 MB（io/limits.rs:54）都在；WebP 编码只有 `new_lossless`；`image` 的 `png` 特性依赖 `png 0.18.0`（Cargo.toml:195-197），与锁定版本同一个包。
- **[已核实·足够] 拼图与标注**：`impeccable_comp::raster` 提供 `Image`（RGBA8）、`create_image`、`crop`、`resize`（缩小按面积平均）、`fit`、`blit`、`fill_rect`、`stroke_rect` 与内置 5×7 点阵字体的 `draw_text`、`draw_label`（crates/comp/src/raster.rs:11-265；字体只有 ASCII，小写转大写，未知字符画成 `?`）。comp-diff 用它拼带标签的并排图（crates/comp-verbs/src/comp_diff.rs:455-475）。
- **[已核实·缺口] 图片来源的读写没有库接口**：`embed_prompt.rs` 的函数全是私有的（crates/context/src/embed_prompt.rs:11-168）；PNG 写 tEXt（:264-299）、JPEG 写 COM 段且超过 65535 字节报错（:300-316）、其他格式写 `<文件>.json` 旁注（:317-323）；读取先看文件内再看旁注（:116-126）；`--scan` 对既无内嵌也无旁注的图报 `MISSING` 并退出 3（:181-219）→ 公开一对读写函数供 `prep-image` 调用（§5.15）→ 漏做则 WORK-F9/AC-3 只能靠重复实现三种格式的写法。
- **[已核实·缺口] 没有“必须在 `.phoenix-ui/` 之内”的检查**：capture 的 `--out` 只做词法规范化（crates/cli/src/capture/mod.rs:270-271、:481）；可照做的是静态服务器的 `read_within`：规范化真实路径后要求以根目录开头，能挡住符号链接（crates/cli/src/capture/serve.rs:149-160）→ §5.10 → 漏做则 WORK-F8/AC-2 不成立。
- **[已核实·足够] `.phoenix-ui/` 下已有的目录**：`config.json`、`build/`、`review/`、`mocks/`、`questions/`、`capture/` 等，没有素材目录；命令一律以 `io.cwd` 为项目位置。
- **[已核实·足够] 浏览器与静态服务器**：`BrowserEngine::new(env).launch()`、`new_isolated_page()`、`page.goto(url, wait, timeout)`、`page.evaluate_value(js)`（crates/cli/src/capture/mod.rs:768-770、:817、:1237、:1131）；`serve::serve(root)` 绑定 127.0.0.1 的随机端口、只服务根目录内的文件、拒绝点文件、按扩展名给出包括 `video/mp4`、`video/webm` 在内的类型（crates/cli/src/capture/serve.rs:20、:34、:128-129）。
- **[已核实·缺口] skill 文本**：new-work.md:139 只说“用真实、核实过的图片，确认图库链接可用”；visualize.md:42 规定产品里的每张图都带来源，:44 让 agent 用 `context` 报告的系统转换器转换图片；optimize.md:28-42 只有图片优化原则 → §6 → 漏做则 agent 不知道有这两个命令，也不知道素材不能进产品。
- **[已核实·缺口] 浏览器自身的后台请求**：2026-11-11 在 macOS、Google Chrome 157 上，用已安装的引擎经一个转发代理运行 `detect <本机页面>` 与 `capture <本机页面>`，除页面自己引用的资源外，代理还看到 Chrome 发往 `update.googleapis.com` 与 `dl.google.com`（组件更新与下载）、`clients2.google.com`（网络时间）、`accounts.google.com`（账号列表）、`www.google.com`（搜索预连接与 `/async/folae`）、`www.gstatic.com`（`/ohttp_gateway/hpke_public_keys/v1/sbc_prod`）的请求。现有启动参数（crates/browser/src/cdp.rs:64-91）里的 `--disable-background-networking` 挡不住它们。同机实验：加上 §5.16 所列参数后只剩 gstatic 的那一个请求 → §5.16 → 漏做则 WORK-N1/AC-4“只请求目标页面及其自身资源”不成立，docs/phoenix-ui.md:130 的同一句话也不属实。引擎进程自身在这两个命令里没有任何 HTTP 调用（`agent_builder` 只在 generate_image.rs 被调用）。
- **[已核实·足够] 信源接口**（2026-11-11 本机逐个请求，经本机已配置的 `HTTPS_PROXY`）：

  | 信源 | 结果 | 备注 |
  | --- | --- | --- |
  | Poly Haven | 200 | 目录 869 条约 855 KB；`files/<slug>` 含 `Diffuse`、`Rough`、`nor_gl` 及各分辨率的 `url`、`size`、`md5` |
  | V&A | 200 | `_images._iiif_image_base_url` 在；IIIF `info.json` 200 |
  | Wellcome | 200 | `locations[].locationType.id == "iiif-image"`，许可在 `license.label` |
  | NASA | 200 | 图片与视频检索、资源清单（含 `~orig/~medium/~small/~preview.mp4`）均可用 |
  | Wikimedia Commons | 200 | `extmetadata.LicenseShortName` 在；1920 宽缩略图可下载；「测绘图纸」加查的写法返回 HAER 图纸（TIFF 原件，取缩略图） |
  | 芝加哥艺术博物馆 | 200 | 必须带 `AIC-User-Agent` 头，否则返回 Cloudflare 校验页（403） |
  | 克利夫兰艺术博物馆 | 200 | `images.web/print/full` 在 |
  | 大都会博物馆 | 200 | 检索必须用 `v1.1/search`；`v1/search` 已于 2026-10-01 下线（410）；对象仍取 `v1/objects/<id>` |
  | Japan Search | 200 | `common.contentsRightsType`、`database == "cobas"` 在 |
  | 荷兰国立博物馆 | 200 | 检索后三跳（对象 → `shows[0]` → `digitally_shown_by[0]` → `access_point[0]`）均 200 |
  | 美国国会图书馆 | **403** | 照片、地图、HABS/HAER 三个入口对 curl 与 Node fetch 都返回 Cloudflare 校验页；经本机代理与直连两条路径、换 UA、改 HTTP/1.1 结果相同 |

- **[已核实·足够] 测试与发布入口**：二进制级测试用 `env!("CARGO_BIN_EXE_phoenix-ui")`，无浏览器时跳过（crates/cli/tests/capture.rs:23、:68-88）；单元测试用 `Io::captured` 收集输出（crates/common/src/lib.rs:174）；`tiny_http` 已是 `crates/context` 的依赖。`test:phoenix` 依次跑页内包检查、release 构建、安装测试、工作区测试（macOS 上在只放行本机的 sandbox 内）、浏览器测试（sandbox 外）、lint 与 oracle（scripts/test-phoenix-ui.mjs:27-50）。冒烟把所有代理变量指向 `http://127.0.0.1:9`（scripts/smoke-phoenix-ui.mjs:81），任何外发请求在冒烟里必然失败。oracle 的 capture 用例只覆盖不启动浏览器的路径（tools/phoenix-ui/tests/oracle/cases/capture.mjs）。
- **[已核实·足够] 第三方声明**：oil-ui 的条目已在 NOTICE.md:19-25 与 THIRD-PARTY-NOTICES.md:59-87，改编范围只写了 capture 与走查子代理。
- **无关发现（只报告，不在本期处理）**：visualize.md:9 写 `phoenix-ui generate-image` 在 `build-phase start` 之前拒绝写入 `.phoenix-ui/mocks/`，`generate_image.rs` 里没有这道检查。

### 1.2 拓扑与文件清单

全部在本仓 `XGENT-ai/skills`。★ 为新增。

| 位置 | 改动 | 所属 |
| --- | --- | --- |
| tools/phoenix-ui/crates/cli/src/media/★：`mod.rs`（参数、编排、用法、输出与退出码）、`sources.rs`（各信源的请求与响应映射、`Endpoints`）、`rights.rs`（分级）、`relevance.rs`（相关性与混排）、`index.rs`（编号索引与锁、素材区边界、`.gitignore`）、`look.rs`（观感数据）、`sheet.rs`（联系表） | 检索与联系表 | M1 |
| 同目录：`fetch.rs`（下载、校验、命名、来源清单）、`poster.rs`（视频首帧）、`audit.rs`（交付前核对） | 取用与核对 | M2 |
| tools/phoenix-ui/crates/cli/src/prep_image.rs★ | 交付图片处理 | M3 |
| tools/phoenix-ui/crates/cli/src/main.rs、lib.rs；crates/cli/Cargo.toml（新增 `image`（`default-features = false`，写法同 comp）、`ureq`、`regex`，dev 依赖 `tiny_http`）；Cargo.lock（只多依赖边） | 路由与依赖 | M1、M3 |
| tools/phoenix-ui/crates/detect/src/lib.rs | `ROOT_USAGE` 加两行 | M1、M3 |
| tools/phoenix-ui/crates/context/src/embed_prompt.rs、lib.rs | 公开 `read_provenance`、`write_provenance` | M3 |
| tools/phoenix-ui/crates/browser/src/cdp.rs | 启动参数补关闭后台联网的开关与特性（:64-91 的列表及其单元测试 :2287 附近） | M2 |
| tools/phoenix-ui/crates/cli/src/capture/mod.rs | `mod serve;` 收窄为 `pub(crate) mod serve;`，供 `media/poster.rs` 复用静态服务器（`serve.rs` 本体不改） | M2 |
| tools/phoenix-ui/crates/cli/tests/network_boundary.rs★；tools/phoenix-ui/tests/fixtures/media/★（接口响应样本、小图片、WebM 短片、带外部资源的页面） | 联网边界测试与夹具 | M1–M3 |
| tools/phoenix-ui/tests/oracle/cases/media.mjs★、prep-image.mjs★、golden/★、golden/cli-help.json | 离线路径 oracle 用例；总帮助 golden 重录 | M5 |
| tools/phoenix-ui/NOTICE.md、THIRD-PARTY-NOTICES.md（及 `--write` 重算的 THIRD-PARTY.json、dependency-audit.json） | oil-ui 改编范围 | M5 |
| scripts/smoke-phoenix-ui.mjs | 增加 media 与 prep-image 冒烟 | M5 |
| skills/phoenix-ui/src/SKILL.src.md、src/reference/new-work.md、visualize.md、optimize.md、typeset.md、skills/phoenix-ui/evals/scenarios.md；渲染生成的 `skills/phoenix-ui/` 文件、`GENERATED.json` 与 `dist/phoenix-ui/bundle/` | 素材与图片处理的用法、断行一句 | M4 |
| docs/phoenix-ui.md、README.md、tools/phoenix-ui/docs/ENGINE.md | 两个命令与联网边界的说明 | M2（docs/phoenix-ui.md 的「素材命令」一节）、M4（其余） |
| docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md | WORK-N1 约束段与 AC-4 按浏览器后台联网的例外补写（D15，§6.8） | M5 |

无需修改的邻接面：启动器与安装器（verb 透传，bundle 清单复制）；`crates/comp`、`crates/comp-verbs`、`crates/hook`、`crates/live`、`crates/detect` 的检测逻辑；`context` 的 IMAGE_TOOLS 指令（crates/context/src/context_cli.rs:207-229）；`generate_image.rs`；完工评审子代理与素材制作子代理；`.github/workflows/phoenix.yml`（push 路径已含 `tools/phoenix-ui/**`、`skills/phoenix-ui/**`、`scripts/*phoenix*.mjs`）。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类/适配器 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `media::run(args, io) -> i32` / `run_with(args, io, &Endpoints)` | main.rs 路由；单元测试经 `run_with` | 先校验参数与素材区位置（失败退出 1，不联网、不写文件）；任何文件只写在素材区内 | `Endpoints`：各信源的基地址，默认是真实地址 | 子命令编排、并发检索、输出与退出码 | 信源为真正的外部依赖，测试时指向本机 `tiny_http` | 命令的 JSON 输出、退出码与素材区文件（V-1、V-2） |
| `media::sources` | run | 每个信源一个 `search(agent, endpoints, query, limit) -> Result<Vec<Item>, String>` 与一个纯函数 `parse_*(&Value) -> Vec<Item>`；单个信源失败不影响其他 | 无 | 11 套响应形状、多跳请求、图片地址规则 | 外部（HTTPS） | 对录下的响应样本断言映射结果 |
| `media::rights`、`relevance`、`look` | sources、run、sheet | 纯函数：`use_grade(license, license_url) -> Use`；`match_count`、`interleave`；`describe(&Image) -> Look` | 无 | 分级规则、分词与混排、像素统计阈值 | 进程内 | 单元测试 |
| `media::index` | run、fetch、audit | `resolve_area(cwd, dir) -> Result<PathBuf, String>`（素材区必须在 `.phoenix-ui/` 内）；`remember(area, items) -> Result<Vec<Item>, String>`（锁内分配编号并原子写回） | 无 | 路径规范化、锁、编号 | 进程内（文件系统） | 命令行为（越界被拒、编号稳定、并行不重号） |
| `media::fetch`、`poster` | run | 下载到素材区内的临时文件，校验通过才改名落位；失败不留半成品；视频与首帧成对成功或成对失败（§5.8） | 无 | 下载候选、体积上限、签名与解码校验、命名 | 外部（HTTPS）；本机浏览器 | 素材区文件与 `sources.json` |
| `media::audit` | run | 只读；不改任何文件 | 无 | 遍历范围、三类比对 | 进程内 | 命令输出与退出码 |
| `prep_image::run(args, io) -> i32` | main.rs 路由 | 不写任何输入文件；单个文件失败不影响其余 | 无 | 方向、裁切几何、预乘缩放、格式选择、片段生成 | 进程内 | 输出文件、报告与 `embed-prompt` 的读取结果（V-4、V-5） |
| `embed_prompt::{read_provenance, write_provenance}` | prep_image；`embed-prompt` verb 自身 | 写法与 `embed-prompt` 完全相同；verb 的输出与退出码逐字节不变 | 无 | 三种载体 | 进程内 | oracle 的 embed-prompt 用例；V-5 |

`Endpoints` 是本期唯一的新接缝：信源是真正的外部依赖，CI 不能依赖它们的可用性，`generate-image` 已用同样的做法。除此之外不加适配器：十一个信源各写各的函数，不抽“信源 trait”；联系表只有 `media` 一个调用方，不并入 capture 的合成页。

## 3. 数据模型与迁移

不涉及数据库与迁移。素材区的 `index.json` 与 `sources.json` 是首版格式，各带 `schema` 字段（§5.6、§5.8）；它们是被忽略的本机文件，读到不认识的 schema 时命令退出并说明，不做转换。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| 10 个可用信源的公开接口 | 已核实足够（2026-11-11，本机） | §1.1 的表；请求写法见 §5.2 | 只读 GET；不带密钥；UA 为 `phoenix-ui-media/<版本> (+https://github.com/XGENT-ai/skills)` | 单个信源超时、连接失败或非 2xx：结果的 `errors` 记该信源与原因，其余照常，退出码 2 |
| 美国国会图书馆 | 已核实缺口（本机 403） | §1.1 | 提供方：LoC；所需行为：对非浏览器客户端返回 JSON；就绪证据：V-1 中该信源返回条目。本方不做绕过；不可用时按上一行处理。影响的只是 LoC 条目的检索，不阻塞开发与终验 | 同上 |
| 本机 Chrome/Chromium/Edge（仅视频首帧） | 已核实足够（Chrome 157，macOS）；其他见 A-2 | §0.3 ADR-2 的实测 | 复用 capture 的浏览器发现与静态服务器 | 浏览器不可用或不支持该编码：该视频取用失败（不落位、不登记，§5.8），退出码 2；错误信息写明浏览器或编解码原因 |
| `impeccable_context::http::agent_builder` | 已核实足够 | http.rs:38-42 | 零改动 | 代理变量指向不可达地址时所有信源连接失败，命令报网络不可用 |
| `embed-prompt` 的三种载体 | 已核实足够 | embed_prompt.rs:116-140、:264-323 | 抽出读写函数，verb 行为不变 | JPEG 的来源文字超过一个 COM 段时 `write_provenance` 改写旁注 |
| Phoenix 启动器 | 已核实足够 | skills/phoenix-ui/src/scripts/phoenix-bootstrap.cjs:77-91、:309-349 | 零改动 | engine 缺失或不符：退出 4 并给补装命令 |
| 许可清单与依赖审计 | 已核实足够 | check-phoenix-deps.mjs:518-540 | 锁文件变化后 `--write` 重算 | 未重算时报 “Third-party inventory is stale” |
| Phoenix CI（四平台 + Node 18） | 已核实足够 | .github/workflows/phoenix.yml:42-205 | 零改动；冒烟脚本增加两个命令 | 推送未授权：V-11 未运行，收口不宣称 WORK-N2/AC-3 |

登记位置：两个 verb 登记在 main.rs 的路由与 `ROOT_USAGE`；没有新增配置项与环境变量；素材区的三个文件（`index.json`、`sources.json`、`.gitignore`）格式见 §5.6、§5.8、§5.10。

## 5. 核心机制

### 5.1 `media` 命令接口

```text
phoenix-ui media search "<词>[|<词>…]" [--kind <类别>] [--limit <n>] [--use all|page] [--sheet] [--dir <素材区>] [--timeout-ms <ms>] [--json]
phoenix-ui media preview <编号>… [--dir <素材区>] [--timeout-ms <ms>] [--json]
phoenix-ui media fetch <编号>… [--maps <列表>] [--resolution 1k|2k|4k|8k] [--video-size medium|small|preview|orig] [--max-mb <n>] [--dir <素材区>] [--timeout-ms <ms>] [--download-timeout-ms <ms>] [--json]
phoenix-ui media audit [<路径>…] [--dir <素材区>] [--json]
```

- 参数解析照 capture 的写法：开关表、带值表、`--k=v` 与 `--k v` 两种形式、未知选项给最接近的写法、重复给值报错（crates/cli/src/capture/mod.rs:155-232）。`--help` 输出中文用法，写明：素材只用于设计稿与方案预览、只进 `.phoenix-ui/` 素材区；各类别；三个分级的含义；退出码；`search` 与 `fetch` 会联网，`audit` 不联网。
- `search`：多个词用 `|` 分开，每个词各查一遍再合并。`--kind` 取 `all`（默认）、`texture`（材质贴图）、`design`（设计器物）、`photo`（摄影）、`blueprint`（测绘图纸）、`map`（地图）、`plate`（版画）、`science`（科学）、`video`（视频）。`--limit` 1–30，`all` 默认 12，其余默认 6。`--use page` 只留可上线条目。`--sheet` 另出联系表与观感数据（§5.7）。`--timeout-ms` 每个请求的超时，默认 12000，范围 1000–120000。
- `preview`：给已在索引里的编号出联系表与观感数据。
- `fetch`：`--maps` 取 `color`、`roughness`、`normal` 的逗号列表或 `all`，默认 `color`，只对贴图有效；`--resolution` 默认 `1k`；`--video-size` 默认 `medium`；`--max-mb` 单个文件上限，默认 256，范围 1–2048；`--download-timeout-ms` 默认 180000。
- `audit`：路径默认是当前目录。
- `--dir` 默认 `.phoenix-ui/media`，四个子命令都受 §5.10 约束。`--json` 把结果 JSON 打到 stdout，代替逐行输出。

### 5.2 信源

请求都是 GET，头为 `User-Agent: phoenix-ui-media/<CARGO_PKG_VERSION> (+https://github.com/XGENT-ai/skills)` 与 `Accept: application/json`；不重试、不翻页；响应体最多读 16 MB。`Endpoints` 有 11 个基地址字段，默认值即下表的主机。`<n>` 是该信源本次要取的条数（§5.3）。

| 信源 id / 前缀 | 检索请求 | 映射要点 | 许可文字 |
| --- | --- | --- | --- |
| `polyhaven` / T | `https://api.polyhaven.com/assets?t=textures`（模型为 `t=models`），本地按词对 slug、名称、分类、标签打分取前 `<n>` 条；每条再取 `…/files/<slug>` | 预览与图片 `https://cdn.polyhaven.com/asset_img/thumbs/<slug>.png?width=768`；贴图的颜色、粗糙度、法线依次取 `Diffuse`、`Rough`、`nor_gl` 下 `<分辨率>.jpg` 的 `url` 与 `size`（没有所要分辨率时取不超过它的最大一档，再没有取最小一档）；来源页 `https://polyhaven.com/a/<slug>` | `CC0`；署名为作者名加 `Poly Haven` |
| `vam` / V | `https://api.vam.ac.uk/v2/objects/search?q=&images_exist=1&page_size=<n>` | IIIF 基址取 `_images._iiif_image_base_url`；图片 `<基址>/full/!1600,1600/0/default.jpg`，预览 `!400,400`；检索时读 `<基址>/info.json` 得原图宽高（§5.2 尺寸规则，取用时复用）；来源页 `https://collections.vam.ac.uk/item/<systemNumber>` | 固定为 `V&A terms: permitted non-commercial uses only; commercial/home-page/high-resolution publication requires licensing`（分级结果为仅参考） |
| `wellcome` / W | `https://api.wellcomecollection.org/catalogue/v2/images?query=&pageSize=<n>&include=source.contributors` | 取 `locationType.id == "iiif-image"` 的位置，IIIF 规则同上；来源页 `https://wellcomecollection.org/works/<source.id>` | 条目的 `license.label`；没有则 `Rights not specified; check item` |
| `loc` / L | `https://www.loc.gov/photos/?q=&fo=json&c=<2n，≤60>`；地图为 `/maps/`；图纸为 `/collections/historic-american-buildings-landscapes-and-engineering-records/?q=<词> drawings&fo=json&at=results&c=<3n，≤60>` | 丢掉 `access_restricted` 的行；图片取 `image_url` 里最后一个 jpg/png，预览取第二个；图纸只留图片地址含 `sheet` 的资源 | 条目的 `rights_advisory` 或 `rights_information`；没有则 `Rights not specified in search; check Rights & Access on item page` |
| `nasa` / N | `https://images-api.nasa.gov/search?q=&media_type=image\|video&page_size=<n>`（视频取 `3n`，≤100） | 预览取 `render == "image"` 的链接；资源清单为条目的 `href`；取用时读清单选文件：图片按 `~orig`、`~large`、`~medium`、`~small` 的顺序，视频先取 `--video-size` 所指的 mp4，再依次 `~medium`、`~small`、`~mobile`、`~preview`、`~orig`；标题或关键词含 animation、visualization、simulation、computer generated、rendering 的视频不要；来源页 `https://images.nasa.gov/details/<nasa_id>` | 描述或署名里出现 copyright、© 或 all rights reserved 时为 `Third-party rights mentioned; check item credits`，否则 `NASA media usage guidelines; generally not subject to US copyright; verify third-party credits` |
| `commons` / C | `https://commons.wikimedia.org/w/api.php?action=query&format=json&formatversion=2&generator=search&gsrsearch=<词> filetype:bitmap&gsrnamespace=6&gsrlimit=<2n，≤40>&prop=imageinfo&iiprop=url\|size\|mime\|extmetadata&iiurlwidth=1920&iiextmetadatafilter=LicenseShortName\|License\|LicenseUrl\|Artist\|ObjectName\|ImageDescription\|AttributionRequired` | 原图宽超过 1920 或类型不是 jpeg/png/webp/gif 时图片取 1920 宽缩略图，否则取原图；预览把缩略图地址里的宽度换成 500；原图尺寸取 `width`、`height` | `LicenseShortName`，否则 `License`，都没有则 `License on file page` |
| `aic` / A | `https://api.artic.edu/api/v1/artworks/search?q=&limit=<2n，≤40>&fields=id,title,artist_display,date_display,image_id,is_public_domain,thumbnail&query[term][is_public_domain]=true`，另带 `AIC-User-Agent` 头（值同 UA） | IIIF 基址取响应的 `config.iiif_url`；图片 `<基址>/<image_id>/full/<min(宽,1686)>,/0/default.jpg`，预览 `full/400,` | `CC0 (public-domain artwork, Art Institute of Chicago)` |
| `cleveland` / K | `https://openaccess-api.clevelandart.org/api/artworks/?q=&has_image=1&cc0=1&limit=<2n，≤40>` | 图片取 `images.print.url`（没有取 `web`），预览取 `web` | `CC0` |
| `met` / M | `https://collectionapi.metmuseum.org/public/collection/v1.1/search?q=&hasImages=true&isPublicDomain=true&limit=<2n，≤12>`，再逐个取 `…/public/collection/v1/objects/<id>` | 要求 `isPublicDomain` 且有 `primaryImage`；预览取 `primaryImageSmall` | `CC0 (Met Open Access, public domain)` |
| `japan` / J | `https://jpsearch.go.jp/api/item/search/jps-cross?keyword=&f-contents=image&size=<3n，≤60>&f-rights=+cc0&f-rights=+pdm&f-rights=+ccby&f-rights=+ccbysa` | 图片取 `contentsUrl[0]`，地址里的 `/slideshow_s/` 换成 `/slideshow/`，原地址作为下载候选 | `contentsRightsType` 映射为 `CC0`、`Public Domain Mark`、`CC BY 4.0`、`CC BY-SA 4.0`，其他值的条目不要；`database == "cobas"` 的署名以 `出典：ColBase（https://colbase.nich.go.jp/）` 开头 |
| `rijks` / R | `https://data.rijksmuseum.nl/search/collection?description=<词>&imageAvailable=true`，取前 `min(2n,10)` 个对象，每个三跳：对象 → `shows[0].id` → `digitally_shown_by[0].id` → `access_point[0].id`（请求头 `Accept: application/ld+json`） | IIIF 服务为 `access_point` 去掉 `/full/…` 之后的部分；图片 `full/!2400,2400/0/default.jpg`，预览 `!500,500` | `subject_of` 里第一个 creativecommons 链接：含 `/zero/` 为 `CC0`，含 `/mark/` 为 `Public Domain Mark`，否则原链接；没有则 `Rights not specified in record; check item page` |

每个条目（`Item`）的字段：`ref`、`provider`、`sourceId`、`kind`、`title`、`detail`、`pageUrl`、`imageUrl`、`previewUrl`、`license`、`licenseUrl`、`attribution`、`use`、`originalWidth`、`originalHeight`，以及按信源才有的 `textureMaps`、`manifestUrl`、`iiifInfoUrl`、`downloadCandidates`、`video`。只接受不带用户信息的 http(s) 地址；`//` 开头的补成 `https://`，`http://` 开头的非回环地址改写成 `https://`。

**原图尺寸的取得（WORK-F7/AC-1）**：条目在返回与生成联系表前必须已有 `originalWidth`/`originalHeight`；取不到尺寸的条目丢弃并记入该信源的提示，某信源全部条目都取不到时按该信源不可用处理。逐信源来源（2026-11-11 本机实测；LoC 因 403 只核到接口形状，无法核实字段）：

- `commons`：`imageinfo` 的 `width`、`height`（即上表已写的字段）。
- `aic`：检索响应的 `thumbnail.width/height`（即取图规则 `min(宽,1686)` 使用的原图宽）。
- `cleveland`：`images.full.width/height`，缺 `full` 时取 `print`（值为字符串，转数字）。
- `polyhaven`：`/assets` 元数据的 `max_resolution`（最高可下载/渲染档的宽高；贴图与模型条目同此字段。`dimensions` 是采集或物理尺寸，不作条目尺寸）。
- `vam`、`wellcome`：检索时对每条再取一次 IIIF `info.json`（wellcome 的位置 URL 即 info.json），读 `width`、`height`；取用时复用该结果，不再重读。
- `rijks`：第三跳得到 IIIF 服务后，检索时对每条再取一次 `info.json` 读宽高（即第四跳）。
- `nasa`：检索时对每条取资产清单旁的 `metadata.json`，图片读 `File:ImageWidth`、`File:ImageHeight`（原图的 EXIF；实测样本 PIA03647 为 1237×2737）；视频读 `QuickTime:ImageWidth`、`QuickTime:ImageHeight`（实测 TROPICS 视频为 1920×1080，metadata.json 没有图片的 File 字段）。
- `met`、`japan`、`loc`：接口不提供像素尺寸（Met 对象 JSON 的 `dimensions`/`measurements` 是实物尺寸；Japan Search 行内无宽高字段；LoC 不可达无法核实），检索时对条目将取用的原图地址（Met 的 `primaryImage`、Japan 的原始 `contentsUrl` 地址、LoC 的 `image_url` 原图）发 `Range: bytes=0-65535` 请求读头部尺寸（PNG/JPEG/WebP）；服务器忽略 Range 时读到 64 KB 即断开；读不出的条目按上一段丢弃。

### 5.3 类别到任务

| 类别 | 任务（信源 / 该任务产出的条目类别） |
| --- | --- |
| `texture` | polyhaven 贴图 |
| `design` | met、aic、cleveland、vam、polyhaven 模型预览、japan、rijks |
| `photo` | commons、loc 照片、nasa 图片 |
| `blueprint` | loc 图纸；commons，检索词为 `<词> (HABS OR HAER OR "measured drawing" OR blueprint)`，只留标题或描述匹配 `habs\|haer\|measured drawing\|blueprint\|elevation\|floor plan\|section` 的条目 |
| `map` | loc 地图；commons，检索词为 `<词> (map OR chart OR plan)` 并按 map/chart/plan 过滤 |
| `plate` | wellcome；japan；commons，检索词为 `<词> engraving` 并按 engraving/etching/woodcut/lithograph/plate 过滤 |
| `science` | nasa 图片、wellcome |
| `video` | nasa 视频 |
| `all` | 每个信源入口各跑一次，共 15 个任务：commons（原词）、japan、polyhaven 贴图、aic、loc 照片、rijks、cleveland、wellcome、met、nasa 图片、loc 图纸、loc 地图、vam、nasa 视频、polyhaven 模型预览 |

- 每个任务要取的条数：`all` 时为 `max(2, ceil(limit / 15))`，其余为 `limit`。
- 任务用 `std::thread::scope` 并发，每个任务自己的错误收进 `errors`，不影响其他任务。

### 5.4 相关性与混排

- 检索词拆成词项：去掉两个字母以内的拉丁词与连接词（and、or、of、the、with、for 等），数字与非拉丁文字保留。
- 条目得分为词项在 `title`、`detail` 中的命中数：转小写、去掉 HTML 标签；拉丁词考虑复数与五个字母以上的前缀匹配；中日韩文字按子串匹配。
- 词项全是拉丁文字或数字时，得分为 0 的条目丢掉并计入 `filtered`。
- 排序：全部词项命中的在前，其余在后；每组内按信源轮转，一轮每个信源出一条；同一 `provider:sourceId` 只留一条；最后按分级排序（可上线、需核对、仅参考）并截到 `limit`。

### 5.5 使用分级

对 `license` 与 `licenseUrl` 拼接后的小写文字依次判断，先命中先得：

1. 含 `non-commercial`/`noncommercial`、`by-nc`、单独的 `nc`、`no deriv`/`noderiv`、`by-nd`、单独的 `nd`、`requires licensing` → `reference`。
2. 含 `third-party rights`、`not verified`、`not specified`、`in copyright` → `check`。
3. 含 `cc0`、`publicdomain`、`public domain`、单独的 `pd` 或 `pdm`、`no known restrictions`、`not subject to us copyright`、`creativecommons.org/licenses/by/` 或 `/by-sa/`、`cc by`/`cc-by`（可带 `-sa`）、`attribution` 后接版本号（可带 `-sharealike`）→ `page`。
4. 其余 → `check`。

规则是 curate-media.mjs:162-168 的同一张表，写成一组 `regex` 常量。结果里另有 `useLabel`（可上线 / 需核对 / 仅参考）。

### 5.6 编号、索引与锁

- `index.json`：`{ "schema": "phoenix-ui-media-index-v1", "items": { "<编号>": { …条目字段, "cachedAt": "<ISO 时间>" } } }`。schema 不同或 JSON 损坏时命令退出 1 并说明，不覆盖。
- 前缀：polyhaven `T`、vam `V`、loc `L`、wellcome `W`、nasa `N`、commons `C`、met `M`、aic `A`、cleveland `K`、japan `J`、rijks `R`。
- `remember`：索引里已有同一 `provider` + `sourceId` 的条目时沿用其编号并更新字段；否则编号为前缀加（该前缀现有最大序号 + 1）。
- 锁：读改写期间持有 `index.lock`（`OpenOptions::create_new`），拿不到时每 50 ms 重试，5 秒后放弃并退出 1，提示“另一个 media 命令正在写索引；确认没有命令在运行后删除 `<路径>`”。写回用同目录临时文件加 `rename`。触发与后果：agent 常把几个不同关键词的检索并行发出，它们写同一个索引；不加锁时两个进程会把同一个编号分给不同条目，之后按编号取用会取错素材。锁只包住本地读改写，不包住联网。

### 5.7 观感数据与联系表

- 观感数据（`look`）：把预览图缩到 48×48（`raster::resize`），亮度取 `(0.2126R + 0.7152G + 0.0722B)/255`。每个区域算亮度均值、标准差、饱和度均值（`(max−min)/max`）与梯度（与右、下邻点亮度差的绝对值均值）。字段：
  - `tone`：整图亮度均值 < 0.33 为 `dark`，> 0.66 为 `light`，否则 `mid`；`mono`：整图饱和度 < 0.1。
  - `edges`：上、右、下、左四条 3 像素宽边带各自的平均色（`#rrggbb`）；`edge`：四条边带平均色的平均。
  - `edgeFlat`：四条边带都满足标准差 < 0.06 且梯度 < 0.02。
  - `paper`：`edgeFlat` 且四条边带的亮度均值 > 0.75、饱和度均值 < 0.2。
  - `calm`：在上、下、左、右各三分之一与中间方块五个区域里，标准差 < 0.09 且梯度 < 0.02 的区域名（`top`、`bottom`、`left`、`right`、`center`）。
  - `ratio`：预览图宽高比，保留两位小数。
- 联系表：`--sheet`（或 `preview`）时并发下载各条的 `previewUrl`（超时用 `--timeout-ms`，单张上限 8 MB），用 `image` 解码。写 `<素材区>/candidates.png`：最多 4 列，每格 360×300、底色 `#EDEDED`；缩略图 `raster::fit` 到 344×238 居中；格底一条分级色带（可上线 `#1E7B46`、需核对 `#B26A00`、仅参考 `#A8322D`），色带上用 `draw_label` 写 `<编号>  <PAGE|CHECK|REF>  <宽>X<高>`（宽高为原图尺寸，按 §5.2 在联系表前已取得，不出现 `?`），下一行写信源 id。预览下载或解码失败的格子画占位底色并写 `NO PREVIEW`，该条的 `look` 为 `null`。视频条目用其预览图。
- 结果 JSON 的 `sheet` 为 `{ "file": "<相对当前目录的路径>", "columns": n, "cells": ["<编号>", …] }`；每个条目带 `look`。没有 `--sheet` 时不下载预览，条目没有 `look`。

### 5.8 取用

对每个编号（不在索引里的编号记入 `errors`；全部不在时退出 1）：

1. **定下载地址**：贴图按 `--maps` 与 `--resolution` 重新读 `files/<slug>`，每种图一个任务，带期望字节数；NASA 读资源清单；有 `iiifInfoUrl` 的用检索时已读到的原图宽高（条目的 `originalWidth`/`originalHeight`，§5.2），长边 ≤ 1600 时取 `full/full`，否则 `full/!1600,1600`，不重读 `info.json`；其余取 `imageUrl`，`downloadCandidates` 作为备选（最多试 3 个，用了备选记一条提示）。
2. **下载**：写到素材区内的 `.<随机名>.part`；`Content-Length` 超过 `--max-mb`、内容类型是 `text/html` 或 `application/json`、实际字节数超过上限、与期望字节数不符、长度为 0，任一情况即失败并删除临时文件。边下边算 SHA-256。
3. **校验**：按文件签名识别 jpg、png、gif、webp、mp4/mov（`ftyp`）、webm，其余拒收。图片用 `image::ImageReader::with_guessed_format()` 在默认上限内整张解码，成功则 `integrity` 为 `decoded`，宽高取解码结果；解码失败或超出解码上限即该文件失败（WORK-F8/AC-1 的解码完整性核对，没有「只读头部尺寸」的成功路径）。视频在签名检查后按 §5.9 取首帧：首帧解码成功即 `integrity` 为 `decoded`，宽高取首帧的 `videoWidth`/`videoHeight`；取不出首帧即该文件失败。
4. **落位**：文件名 `<编号>-<安全化的 sourceId>[-<图种>-<分辨率>].<按签名定的扩展名>`（安全化：`[^A-Za-z0-9_-]+` 换成 `-`，最长 60 个字符）；同名文件直接覆盖。长边不足 1000 像素、`use` 为 `reference` 的各记一条提示（后者写明“仅参考：只作生成或设计参考”）。
5. **首帧**：视频按 §5.9 取首帧，存为 `<同名>-poster.png`；取不出首帧（浏览器不可用、编码不支持、超时）时该视频取用失败，视频与首帧都不落位、不登记（WORK-F8/AC-4：取用视频时同时得到首帧静态图）。
6. **登记**：写 `sources.json`：`{ "schema": "phoenix-ui-media-sources-v1", "files": [ … ] }`，按 `file` 替换或追加，写回同样用临时文件加 `rename`，并在 §5.6 的同一把锁内完成。每条：`file`、`ref`、`title`、`provider`、`pageUrl`、`downloadUrl`、`license`、`licenseUrl`、`attribution`、`use`、`fetchedAt`（`YYYY-MM-DD`）、`width`、`height`、`bytes`、`sha256`、`integrity`，贴图另有 `map`、`resolution`，首帧另有 `derivedFrom`（许可与署名同原视频）。

下载并发最多 3 个。单个任务失败不影响其他任务。第 2、3 步的触发与后果：信源的图片地址会返回错误页、被代理截断或给出损坏的文件；不校验时坏文件会被登记成素材并进入设计稿。

### 5.9 视频首帧

`poster::first_frame(env, area, file) -> Result<Vec<u8>, String>`：

1. `BrowserEngine::new(env).launch()`；失败返回“浏览器不可用：<原因>”。
2. 下载的点文件不能被静态服务器读取，先将视频移到素材区内新建的临时子目录，使用普通文件名供解码；成功后才移到正式文件名，失败删除暂存目录。`serve::serve(area)` 提供素材区；`new_isolated_page()`，`goto(server.url_for(file), "load", 30s)`。必须经这个本机 HTTP 地址打开：`file://` 下 canvas 会被污染，导不出像素。
3. `evaluate_value` 一段写成 Rust 字符串常量的脚本（C-5）：清空文档，新建 `<video muted preload="auto">`，`src` 为当前地址；等 `loadeddata`（最长 20 秒）；出现 `error` 事件时返回 `{ error: "<video.error.code 与 message>" }`；成功后建 `videoWidth × videoHeight` 的 canvas，`drawImage(video, 0, 0)`，返回 `{ width, height, png: canvas.toDataURL("image/png") }`。
4. 解出 base64 得到 PNG 字节；关闭页面与浏览器。

一次 `fetch` 里有多个视频时共用一个浏览器与服务器。首帧失败（浏览器不可用、编码不支持、超时）即该视频取用失败：原因写进该条的失败信息，视频与首帧都不落位、不登记，命令退出码 2；首帧解码成功同时是视频的解码完整性核对（§5.8 第 3 步）。首帧就是第 0 帧，开头是黑场的视频得到黑图，输出里不做替换。

### 5.10 素材区边界

`index::resolve_area(cwd, dir)`：

1. `base = cwd/.phoenix-ui`；`area = cwd.join(dir)` 做词法规范化（去掉 `.`、`..`）；`area` 不以 `base` 开头时拒绝。
2. `create_dir_all(area)` 后对 `base` 与 `area` 各取 `canonicalize`；规范化后的 `area` 不以规范化后的 `base` 开头时拒绝（符号链接指到外面）。
3. 拒绝的文案：`素材只取到 .phoenix-ui/ 下的素材区，只用于设计稿与方案预览，不进入产品：<路径> 在 .phoenix-ui/ 之外`，退出 1；拒绝发生在第 1 步时不创建任何目录，也不联网。
4. `area/.gitignore` 不存在时写入一行 `*`；已存在则不动。

四个子命令都先校验词法与真实路径边界；创建前还核对已有父目录的真实路径，避免越界符号链接导致外部目录被创建。`audit` 只做边界检查，不创建目录或 `.gitignore`，保持 §5.11 的只读合同。其余命令通过检查后按上述步骤初始化。命令写出的所有文件（索引、锁、临时文件、联系表、素材、首帧、来源清单）都在 `area` 之内。触发与后果：agent 为了省一步会想把素材直接取到 `public/` 之类的产品目录；素材一旦落在那里，它的许可就跟着进了产品（C-3）。

### 5.11 交付前核对

`media audit [路径…]`：读 `<素材区>/sources.json`；没有清单或清单为空时输出“素材区没有取用记录，无可核对”，退出 0。否则遍历各路径（默认当前目录），跳过 `.git`、`node_modules` 与当前目录的 `.phoenix-ui`，不跟随符号链接，查三类发现：

1. `reference`：不超过 2 MB、是合法 UTF-8 的文件里出现素材区的相对路径（如 `.phoenix-ui/media`）。
2. `copy`：文件字节数等于清单里某条的 `bytes` 且 SHA-256 相同。
3. `credit`：上述文本文件里出现清单里某条的 `pageUrl`、`downloadUrl`、`licenseUrl`、`license` 原文，或任意非空 `attribution` 原文（不设长度下限；`license` 等短文字可能误报，沿用主代理判断的既有约定）。

输出每条发现的类别、文件、行号（文本类）与对应的清单条目；末行汇总。有发现退出 2，没有退出 0。命令只读。`credit` 是提示：署名文字碰巧出现在产品文案里时由主代理判断，判断依据写进交付报告。

### 5.12 `media` 的输出与退出码

- 结果 JSON：`search` 为 `{ "schema": "phoenix-ui-media-v1", "command": "search", "query", "queries"?, "kind", "count", "filtered", "area", "items": […], "errors": [{ "source", "message" }], "sheet"? }`；`preview` 为 `{ …, "command": "preview", "items", "sheet" }`；`fetch` 为 `{ …, "command": "fetch", "files": [清单条目外加 "warnings"], "errors": [{ "ref", "map"?, "message" }] }`；`audit` 为 `{ …, "command": "audit", "findings": [{ "kind", "file", "line"?, "ref", "match" }] }`。
- 终端（非 `--json`）：检索每条一行 `<编号>  <可上线|需核对|仅参考>  <宽>x<高>  <信源>  <条目标题>`；不可用的信源一行 `UNAVAILABLE <信源>: <原因>`；取用每个文件一行 `<文件>  <宽>x<高>  <字节数>  <分级>`，失败 `FAILED <编号>: <原因>`；末行汇总并给出联系表、来源清单的路径。
- 退出码：0 完成且没有错误（检索到 0 条也是 0）；1 无法运行：参数错误、素材区在 `.phoenix-ui/` 之外、索引无法读写、给的编号都不在索引里，或 `search` 的全部任务都失败——此时不写索引，文案在全部是连接或超时错误时为 `网络不可用：<n> 个信源均无法连接`，否则为 `全部信源不可用` 并逐个列出原因；2 已运行但有部分失败：个别信源不可用、个别文件下载或校验失败（含视频取不出首帧）、联系表有格子没有预览，或 `audit` 有发现。
- 不返回缓存：检索结果只来自本次请求；索引只用来保持编号。

### 5.13 `prep-image` 命令接口

```text
phoenix-ui prep-image <图片或目录>… (--width <显示宽> | --widths <像素宽,…>) [--sizes <值>] [--crop <宽:高>] [--trim] [--quality <1-100>] [--out <目录>] [--json]
```

- 输入：PNG、JPEG、WebP 文件，或目录（不递归，取其中这三种扩展名的文件）。输入里有目录时必须给 `--out`，且 `--out` 不能是这些目录之一。
- 输入文件没有内嵌来源（`read_provenance` 为空）时该输入失败并写明「先嵌入来源再处理」，不写任何输出（WORK-F9/AC-3）。
- `--width`：显示宽度（CSS 像素），导出宽为它的 2 倍。`--widths`：额外要导出的像素宽。两者可同给，至少给一个。取值 1–16384。
- `--sizes`：片段里的 `sizes`；默认在给了 `--width W` 时为 `(max-width: Wpx) 100vw, Wpx`，否则 `100vw`。
- `--crop`：`宽:高`（也接受 `宽/高`、`宽x高` 或一个小数），比例 0.01–100，居中裁切。`--trim`：裁掉全透明的边。
- `--quality`：JPEG 质量，默认 82。
- `--out`：输出目录，默认是各输入文件所在的目录。
- 参数错误、没有任何输入时退出 1，不写文件。

### 5.14 `prep-image` 处理流程

每个输入文件：

1. 读入字节，算 SHA-256；按签名识别类型；APNG（有 `acTL` 块）与动画 WebP 报“动画图片不支持”。用 `image::ImageReader::with_guessed_format()` 在默认上限内解码为 RGBA8，取出 ICC 配置与方向，`apply_orientation`。解码失败即该文件失败，写明原因（WORK-F9/AC-5）。
2. `--trim`：取 alpha > 0 的像素的外接矩形裁切；整张全透明即该文件失败；没有透明像素时不裁并记一条说明。
3. `--crop`：`cw = min(w, round(h × r))`，`ch = min(h, round(w / r))`，四舍五入后各至少取 1（`w`、`h` 本身 ≥ 1，`min` 不会再引入 0），居中。
4. 定输出宽：`{2 × width} ∪ widths` 里每个值取 `min(值, 当前宽)` 后去重；被压到原图宽的记一条 `原图宽 <w>，未放大`（WORK-F9/AC-1）。
5. 有无透明：当前像素里存在 alpha < 255 即有。
6. 缩放到每个输出宽，高按比例四舍五入且至少取 1；`image::imageops::resize` 用 `Lanczos3`；有透明的图先把 RGB 乘以 alpha 再缩放、缩放后还原，避免透明边缘出现暗边。
7. 编码：没有透明用 `JpegEncoder::new_with_quality`（丢掉 alpha），有透明用 `WebPEncoder::new_lossless`；原图有 ICC 配置时 `set_icc_profile`。
8. 写出 `<输出目录>/<原文件名去扩展名>-<宽>.<jpg|webp>`。处理前先算出本批全部输出路径：某路径与本次任何一个输入文件相同，或同一输出路径被两个不同输入命中（如不同目录的同名输入加共享 `--out`）时，相关输入失败并写明可定位到输入的原因，不写这些文件；已存在的无关同名文件仍直接覆盖。触发与后果：输入名本身形如 `hero-1200.jpg` 且又请求 1200 宽时，输出会落在输入上，原图被覆盖后无法恢复；同名输入相互覆盖会让用户拿不到每个输入对应的交付文件。
9. 来源：先 `read_provenance(输入)`；为空时该输入失败并写明「先嵌入来源」，不写任何输出（WORK-F9/AC-3）；有值时对每个输出 `write_provenance`。
10. 处理完再读一次输入文件，SHA-256 与第 1 步不同则报错（WORK-F9/AC-3 的直接证据写进报告）。

- 报告 JSON：`{ "schema": "phoenix-ui-prep-image-v1", "files": [{ "input", "original": { "width", "height", "bytes", "alpha", "sha256", "provenance": bool }, "crop"?: { "x", "y", "width", "height" }, "outputs": [{ "file", "width", "height", "bytes", "format", "provenance": "jpeg-com"|"sidecar" }], "notes": […], "html"? }], "errors": [{ "input", "message" }] }`。
- 片段（`html`）：某个输入得到两个及以上输出时生成 `<img src="<最大的一张>" srcset="<a> <宽>w, <b> <宽>w" sizes="<sizes>" width="<最大宽>" height="<对应高>" alt="">`，按宽度升序；路径相对当前目录，逐段做 URL 编码。只有一个输出时不生成片段。
- 终端（非 `--json`）：每个输出一行 `<文件>  <宽>x<高>  <字节数>`，说明与失败各一行，片段原样输出。
- 退出码：0 全部成功；1 无法运行；2 有文件失败，其余已处理。

### 5.15 来源读写函数

crates/context/src/embed_prompt.rs，并在 crates/context/src/lib.rs 导出：

```rust
/// How `write_provenance` stored the text.
pub enum Carried { PngText, JpegCom, Sidecar }

/// The provenance embedded in the image at `path`, or in its `<path>.json`
/// sidecar: the same lookup `embed-prompt --read` performs.
pub fn read_provenance(path: &str) -> Option<String>

/// Stores `text` the way `embed-prompt` does for this file type. A JPEG whose
/// text does not fit one COM segment gets the sidecar instead.
pub fn write_provenance(path: &str, text: &str) -> Result<Carried, String>
```

`run()` 的写入分支改为调用同一段内部实现；`embed-prompt` verb 自己遇到 JPEG 文字过长时仍照原样报错退出 1，输出文字与退出码逐字节不变。

### 5.16 联网边界

| 命令 | 允许的联网 | 机制 |
| --- | --- | --- |
| `context`、`hook`、`detect <源码文件>` | 无外网请求（只有既有的本机端口探测） | 这些路径不调用 `agent_builder`；V-3 用记录代理验证 |
| `detect <url>`、`capture` | 目标页面及其自身加载的资源 | 引擎进程不发请求；浏览器按下列参数关闭后台联网 |
| `media search/preview/fetch` | §5.2 的信源与条目给出的图片地址 | 只在命令被调用时发生；`audit` 不联网 |
| `prep-image` | 无 | 纯本地处理 |
| 以上全部 | 不更新、不下载 engine 或组件 | 启动器行为不变；浏览器的组件更新被关闭 |

浏览器启动参数（crates/browser/src/cdp.rs `default_chrome_args`）在现有列表上追加，2026-11-11 在 Google Chrome 157 上验证过效果：

- 开关：`--disable-component-update`（组件更新与下载）、`--disable-domain-reliability`、`--no-pings`、`--no-default-browser-check`、`--gaia-url=http://127.0.0.1:9`、`--google-apis-url=http://127.0.0.1:9`、`--lso-url=http://127.0.0.1:9`（账号列表请求改发本机的丢弃端口）。
- `--disable-features` 追加：`NetworkTimeServiceQuerying`、`PreconnectToSearch`、`AimEnabled`、`AiModeOmniboxEntryPoint`、`AimServerEligibilityEnabled`、`AimEntrypointEligibility`、`OptimizationGuideModelDownloading`、`OptimizationHintsFetching`、`OptimizationTargetPrediction`、`CertificateTransparencyComponentUpdater`、`AutofillServerCommunication`、`InterestFeedContentSuggestions`、`HashPrefixRealTimeLookups`、`SafeBrowsingHashPrefixRealTimeLookups`、`SafeBrowsingHashRealTimeOverOhttp`。
- 这组参数下剩余的只有 `https://www.gstatic.com/ohttp_gateway/hpke_public_keys/v1/sbc_prod`。M2 里先做一次限时排查（用 `--log-net-log` 看发起方，再试对应的特性开关）：
  - 找到能关掉它的参数 → 加进列表，测试里的残余清单为空。
  - 排查后仍在 → 列入测试里的残余清单与 docs（§6.5），不用 `--host-resolver-rules` 屏蔽主机（目标页面自己引用该主机的资源会因此加载失败）。
  - 两种结果都在 PRD 的 WORK-N1 约束段补一句（§6.8）。
- 追加的参数不改变页面渲染；既有浏览器测试与 oracle 在 V-8 里作为回归。

## 6. Skill 与文档增量

skill 源文件用英文写，与现文一致；下列为要表达的内容与落点。动这些文件前先核对 §0.6 对应行，改完运行渲染（C-9）。给用户的话术随用户语言，中文会话里分级写“可上线 / 需核对 / 仅参考”。

### 6.1 `skills/phoenix-ui/src/reference/new-work.md`

- 真实图片条款（:139）之后加一条：
  - 设计阶段（方向卡、设计稿、`.phoenix-ui/` 下的预览页）需要真实图像时，用 `{{scripts_path}}/phoenix-ui media search "<词>" --kind <类别> --sheet` 检索，打开联系表再选，随后 `media fetch <编号>`。每次调用前在对话里说明在找什么；这是 Phoenix 里唯一会访问素材信源的命令。
  - 取到的素材留在 `.phoenix-ui/media/`，只是预览材料：作为设计稿的参考图（`generate-image --ref` 或宿主生图工具的输入图），或放进 `.phoenix-ui/` 下的预览页。不复制进项目资产目录，不从产品代码引用，许可清单与署名不写进产品文件。
  - 分级只说明条目自身的许可：向用户展示候选时带上分级；“可上线”不表示可以放进这个产品。
  - 产品里的图仍按现有规则：项目拥有或获得的真实图片，或生成的 plate，并带来源。
- 完工段里来源扫描的一段（:158，`embed-prompt --scan`）之后加一句：存在 `.phoenix-ui/media/sources.json` 时，在结果送评审或向用户报告之前运行 `{{scripts_path}}/phoenix-ui media audit`，清掉每条发现（把副本换成可以上线的图片，删掉引用与署名），并在交付报告里写明核对结果；`credit` 类发现经判断保留的，写出依据。
- 不改 :109-123 的 comp 合同、:146-156 的取证与走查、处置词相关段落。

### 6.2 `skills/phoenix-ui/src/reference/visualize.md`

- 「Generate three compositional options」里讲参考图的一段（:11）补一句：brief 需要真实的物件、地图、材质或历史图像时，可以把 `media fetch` 取到的素材作为参考图传入；写明它贡献什么（题材、年代、构图、材质线索），生成结果不照搬原图；素材本身不成为 plate。
- 转换一句（:44）改为：交付图片的缩放与格式转换用 `{{scripts_path}}/phoenix-ui prep-image`（输出带与原图相同的来源）；它不产出的格式才用 `context` 的 IMAGE_TOOLS 行报告的转换器。
- :42 的来源规则不动。

### 6.3 `skills/phoenix-ui/src/reference/optimize.md`

「Optimize Images」（:28-42）在原则之后加用法：`{{scripts_path}}/phoenix-ui prep-image <文件…> --width <显示宽> [--widths <像素宽,…>] [--crop <宽:高>] [--trim]`——按显示宽度的 2 倍导出且不放大，不透明图出 JPEG、带透明通道的图出无损 WebP，多个宽度时给出可直接使用的 `srcset`/`sizes` 片段，原图不动，输出带原图的来源；需要有损 WebP 或 AVIF 时另用项目自己的工具链，并自行补记来源。原有的原则与示例保留。

### 6.4 `skills/phoenix-ui/src/reference/typeset.md`

「Chinese copy」的 Line breaks 一条（:74）末尾加一句（P2-D01）：按语义断句优先用 `<br>`；把整段包进 inline-block 的 span 会让这段文字退出 `xgent/cjk-*` 页面规则与字体备注的检查范围。

### 6.5 `skills/phoenix-ui/src/SKILL.src.md` 与 `docs/phoenix-ui.md`

（docs/phoenix-ui.md 的「素材命令」一节在 M2 随批 1 定稿，作为 V-1 首次上手的公开资料前置；本节其余改动在 M4。）

- SKILL.src.md:15 “The engine does not update or download during context, detection, or hooks.” 之后加一句：`media` 是唯一访问素材信源的命令，只在被调用时联网。Commands 表列的是设计子命令，不加行。
- docs/phoenix-ui.md 在「取证命令（capture）」之后新增两节：
  - 「素材命令（media）」：用途与边界（只用于设计稿与方案预览，不进入产品）；四个子命令与主要参数；类别与 11 个信源；三个分级的含义与各信源的许可口径（Poly Haven 为 CC0；美术馆只取公有领域或 CC0；Japan Search 只取 CC0、公有领域、CC BY、CC BY-SA，ColBase 须署“出典：ColBase”；Commons 按单个文件的许可，BY-SA 改色或合成后须同许可共享；V&A 为仅参考；美国国会图书馆不推定无版权限制；Wellcome 保留条目许可；NASA 保留第三方署名与限制）；素材区的三个文件与自忽略；联系表与观感数据字段；取用校验（图片整张解码为成功条件、视频以首帧解码为准）、贴图三种图、视频首帧及其对浏览器与编码的依赖（取不出首帧即视频取用失败）；`audit` 的三类发现；退出码；信源不可用与离线时的表现；美国国会图书馆在部分网络下对非浏览器客户端返回 403，届时报为不可用。
  - 「交付图片处理（prep-image）」：参数、处理顺序、格式规则（JPEG 与无损 WebP，不出有损 WebP 与 AVIF）、不放大、片段、原图不变、来源的三种载体（WebP 为旁注文件，须随图片一起移动）、输入无内嵌来源时被拒、退出码。
- 同文件「取证命令」的「浏览器与隐私」一条（:130）改写：引擎自身不发请求；浏览器以启动参数关闭组件更新、账号、时间、搜索等后台联网；列出 §5.16 排查后仍存在的浏览器自身请求（没有则写“在 Google Chrome 上未观察到其他请求”）；写明这一结论在 Google Chrome 上验证，Edge、Brave 等其他浏览器的后台联网未逐一验证。
- 同文件新增一小段「联网边界」，用一张表复述 §5.16 的前四行。

### 6.6 第三方声明

- `tools/phoenix-ui/NOTICE.md` 的 oil-ui 段补一句：`phoenix-ui media`（`crates/cli/src/media/`）改编其素材检索的信源映射、许可分级、相关性过滤、稳定编号与观感数据算法，`phoenix-ui prep-image`（`crates/cli/src/prep_image.rs`）改编其处理顺序与片段格式，均以 Rust 重写。
- `tools/phoenix-ui/THIRD-PARTY-NOTICES.md` 的 oil-ui 条目把改编范围写全，MIT 全文与 “Copyright (c) 2026 oil-oil” 不动。随后 `check-phoenix-deps.mjs --write`。

### 6.7 其余复述

- `ROOT_USAGE`（crates/detect/src/lib.rs:53-68）在 capture 之后加两行：`media <search|preview|fetch|audit>   Find open-license reference media for comps and previews`、`prep-image <files...> [options]     Export delivery images at 2x display width with srcset`。
- `README.md` 的「Phoenix UI」一节加一条：`phoenix-ui media` 检索并取用开放授权素材（只进 `.phoenix-ui/media/`，仅用于设计稿与方案预览），`phoenix-ui prep-image` 批量导出交付图片；细节见 docs/phoenix-ui.md。该节 engine 一条的“hook 与 context 运行时不联网补下载”不动。
- `tools/phoenix-ui/docs/ENGINE.md` 的 crate 说明（:15 `cli` 一行）补上 media 与 prep-image。
- `skills/phoenix-ui/evals/scenarios.md` 在现有 21 条之后追加 3 条（写法照第 15–21 条，含失败信号）：
  - 方向稿需要真实素材：应先说明在找什么，用 `media search --sheet` 并打开联系表，展示候选时带分级，取到 `.phoenix-ui/media/` 后只作参考图或放进预览页；失败信号：从网上随手取一张没有许可信息的图、把素材复制进 `public/` 或 `src/`、把署名写进产品页面。
  - 素材区有取用记录的界面完工：应运行 `media audit` 并清掉发现后再交付；失败信号：不跑核对，或带着副本交付。
  - 把大图处理成上线尺寸：应使用 `prep-image --width <显示宽> --widths <像素宽,…>`（如 `--width 600 --widths 600,1200`；两个及以上输出才给片段）并采用它给出的片段，`embed-prompt --scan` 保持干净；失败信号：手工调用系统转换器而丢失来源、放大原图、覆盖原图。

### 6.8 PRD 澄清

`docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md` 的 WORK-N1 改两处（用户 2026-11-11 确认的口径，见 D15）：

- 约束段「目标页面渲染」一条末尾补一句：引擎自身不发起其他请求，并以启动参数关闭浏览器的后台联网；浏览器自身仍发出的请求如实列在 docs/phoenix-ui.md，属浏览器行为。
- AC-4 的「页面检测与取证命令只请求调用方指定的目标页面及该页面自身加载的资源」之后插入限定：「引擎自身不发起其他请求；以启动参数关闭浏览器后台联网后，浏览器自身仍发出的请求以 docs/phoenix-ui.md 列明的清单为限」；「不发起素材检索、不自动更新或下载」与其余文字不动。

页头版本号加一，其他内容不动。V-3 的记录按同一口径分开报告引擎请求与已登记的浏览器残余，不把例外扩成任意外发。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N1 | PRD WORK-N1/AC-1–AC-4 | 静态运行不外发；页面检测与取证只访问目标页面及其资源；素材检索只在显式调用时发生；都不自动更新或下载。AC-1–AC-3（临时资料目录、凭据不落证据、走查只用测试凭据）保持 P1 的实现 | 浏览器自身的残余请求按 D15 记录 | §5.16；素材相关代码只在 `media` 模块；skill 要求调用前说明 | V-3（记录代理）、V-8（capture 既有测试覆盖 AC-1、AC-2）、V-9 A3（调用可见）、A4（sandbox 下的静态运行）、V-10（走查交接段无 diff，AC-3） |
| WORK-N2 | PRD WORK-N2/AC-3 | 三个新入口在四平台可用；缺 engine 给补装命令 | engine 缺失退出 4 | D1；启动器零改动 | V-8（本机冒烟）、V-11（四平台） |
| WORK-N4 | PRD WORK-N4/AC-2（AC-1 持续满足） | 改 skill 与命令前清单到位，diff 逐项对得上；声明覆盖新的改编范围 | 出现清单外的行为差异时先更新 §0.6 再继续 | §0.6、§6.6 | V-6、V-10 |

安全项：

- **写入范围**：`media` 的所有写入限定在 `.phoenix-ui/` 内的素材区（§5.10）；`prep-image` 不写输入文件（§5.14 第 8、10 步）。
- **下载内容**：只接受 http(s) 地址；体积上限、内容类型与签名检查、图片整张解码（§5.8）；视频只在本机无头浏览器的临时资料目录里打开。
- **外发信息**：请求里只有检索词与固定 UA，不带项目路径、用户信息或密钥。
- **文件即数据**：信源返回的标题、描述、署名只作为数据写进 JSON 与清单；联系表上的文字限定为编号、分级、尺寸与信源 id。

可观测性：命令的全部事实在结果 JSON、终端行与退出码里；不新增日志通道。

## 8. 失败模式、发布与回滚

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现/降级 | 检测 | 恢复/补偿 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| `--dir` 指向 `.phoenix-ui/` 之外，或素材区是指向外面的符号链接（agent 想把素材直接放进 `public/`） | 当前项目 | 第三方素材进入产品目录，许可随之带入产品 | 命令拒绝并说明，退出 1 | 命令自身；`media audit` | §5.10 两步检查 | V-1、V-2 |
| 两个 `media` 命令同时写索引（agent 并行发出多次检索） | 当前素材区 | 同一编号指向不同条目，之后按编号取错素材；或索引写坏 | 后到的命令等待，超过 5 秒退出 1 并提示 | 命令自身 | §5.6 的锁与原子写回 | V-2 |
| 下载到的不是素材：错误页、截断或损坏的文件 | 单个条目 | 设计稿里放进坏图，清单登记了不存在的内容 | 该条 `FAILED`，不落位、不登记，其余照常 | 内容类型、签名、字节数、整张解码 | §5.8 第 2–3 步 | V-2 |
| 页面检测或取证时浏览器自己联网（组件更新下载、账号、搜索） | 每次 `detect <url>`、`capture` | 无数据后果；违反联网边界，且在目标环境里产生与页面无关的外发请求 | 无提示 | 记录代理看到非目标主机 | §5.16 的启动参数；残余如实记录 | V-3 |
| `prep-image` 的输出路径落在某个输入文件上，或两个输入命中同一输出路径（同名输入加共享 `--out`） | 单个原图或单个交付文件 | 原图被覆盖无法恢复，或同名交付相互覆盖、拿不到每个输入对应的文件 | 相关输入失败并写明可定位的原因，不写这些文件 | 写前比对路径（本批输出全集对输入全集与本批内部）；处理后复核输入的 SHA-256 | §5.14 第 8、10 步 | V-5 |

- 没有列入的情况按最简行为处理：信源请求不重试，失败即报该信源不可用；索引损坏时退出并说明，由用户删除素材区重来；下载中断留下的 `.part` 文件在下次取用同名条目时被覆盖。
- 发布顺序：本期只到开发候选（本机开发构建 + 四平台 CI 制品），不发 npm/R2。引擎源码已变，发布须走 CI 构建，不能用 `--reuse-engines`。
- 放量/切换门：无；新命令只在调用时生效。浏览器启动参数对 `detect <url>`、`capture`、comp 门禁与组件评审同时生效，以 V-8 的既有浏览器测试与 oracle 为回归门。
- 回滚：回退对应提交并重新渲染 skill；素材区是被忽略的本机目录，没有数据或配置迁移。

## 9. 验证

### 9.1 行为与接口验证

共同环境：仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；Cargo 命令在 `tools/phoenix-ui` 下运行；本机 macOS 有可发现的 Google Chrome（可用 `PHOENIX_UI_BROWSER` 指定）；浏览器测试不放进 `sandbox-exec`；场景 E2E 的项目目录建在仓库树之外的临时目录并 `git init`。V-1、V-9 需要能访问各信源的网络（本机已有的 `HTTPS_PROXY` 等代理变量照常继承）。

批内最小反馈：改过 `crates/cli/Cargo.toml` 后先在 `tools/phoenix-ui` 下运行一次 `mise exec --no-deps -- cargo check -p phoenix-ui --offline` 让锁文件补上依赖边（只应多出 `phoenix-ui` 对 `image`、`ureq`、`regex`、`tiny_http` 与 `image` 对 `png` 等所选格式的边；出现 `rayon`、AVIF/EXR/TIFF 相关的包说明 `image` 没关默认特性，违反 C-5；出现其他新包或版本同样停下核对）。之后用 `mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` 构建，直接运行 `target/debug/phoenix-ui media …` 或 `prep-image …` 复现单个问题；纯逻辑用 `mise exec --no-deps -- cargo test -p phoenix-ui --lib media::<模块> --locked --offline`。两者都只在有具体失败或疑问时运行。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **主代理为一张航海史专题页的设计稿找一张古地图和一块纸张贴图，再取一段发射视频做预览素材。** 角色：只读过 `phoenix-ui media --help` 与 docs/phoenix-ui.md 的素材一节，不读源码。真实网络、真实信源。步骤与结果：①`media search "old world map\|nautical chart" --kind map --sheet`：结果来自至少一个信源，每条有编号、标题、信源、原图尺寸（按 §5.2 的尺寸来源取得，不是预览尺寸，也不是 `?`）、许可与分级；`candidates.png` 里每格有缩略图、编号、分级色带与尺寸，每条有 `look`（四边主色、`paper`、`calm`）；记录每个信源是返回条目还是被报为不可用及原因（WORK-F7/AC-1、AC-2、AC-4，A-1）；②原样再查一次，已出现条目的编号不变（AC-1）；③加 `--use page`，结果里没有需核对与仅参考的条目（AC-3）；④`media search "paper" --kind texture` 后 `media fetch <地图编号> <贴图编号> --maps all`：素材区出现地图与颜色、粗糙度、法线三张贴图，报告的宽高与用系统看图工具看到的一致，`sources.json` 逐条有文件名、标题、来源页、许可、署名与取用日期，素材区有 `.gitignore`，项目里 `git status --short` 不出现素材区的任何文件（WORK-F8/AC-1、AC-4，D3）；⑤`media fetch <编号> --dir public/img` 与素材区是指向 `../public` 的符号链接两种情况：都被拒绝并说明原因，退出 1，`public/` 下没有新文件（WORK-F8/AC-2）；⑥`media search "rocket launch" --kind video` 后取用一条：得到 mp4 与 `-poster.png`，首帧图能打开（WORK-F8/AC-4）；⑦把代理变量都指向 `http://127.0.0.1:9` 再检索：输出“网络不可用”，退出 1，没有条目，`index.json` 未变（WORK-F7/AC-4）；⑧`media search "bridge" --kind blueprint`：有来自 Commons 的图纸条目，LoC 返回条目或被报为不可用（D6）；⑨把一张取到的图复制为 `public/hero.jpg`、在 `index.html` 里写上素材区路径、在 `about.html` 里贴上来源页链接后运行 `media audit`：列出 `copy`、`reference`、`credit` 三条发现并退出 2；清掉后退出 0（WORK-F8/AC-3）。Agent 逐个打开联系表、取到的图片与首帧查看 | `target/debug/phoenix-ui media …`（debug 二进制的绝对路径），在临时项目目录里运行；第⑦步为 `HTTPS_PROXY=http://127.0.0.1:9 HTTP_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9 … media search …` | M1–M2 交付齐备后（批 1）；M4 的 V-7 的前置 |
| V-2 | 局部 | 准入：各信源的响应映射与分级是分支多的纯逻辑，且分级错了会把受限素材标成可上线；CI 不能依赖第三方接口；素材区边界与索引并发是 §8 列出的高后果路径，E2E 难以稳定触发。内容：①`rights::use_grade` 的对照表（CC0、PDM、CC BY、BY-SA、BY-NC、BY-ND、V&A 条款、NASA 两种文字、未注明、空串）；`relevance` 的词项拆分、复数与前缀、中日韩子串、得分 0 的过滤与按信源轮转；`look::describe` 对合成图的结果（纯浅灰图为 `paper`、五个区域都平静；上半平静下半噪声的图只有 `top` 平静且 `edgeFlat` 为假）；编号前缀与沿用；文件名安全化；②11 个信源的 `parse_*` 对 `tests/fixtures/media/api/*.json` 的映射结果（样本取自 2026-11-11 及之后的真实响应并裁剪；LoC 的样本本机录不到，按 oil-ui 测试夹具的响应形状自写并在文件头注明未经真实响应核对）；Met 用 v1.1 的形状；AIC 的请求带 `AIC-User-Agent`；尺寸跟随请求（vam/wellcome/rijks 的 `info.json`、nasa 的 `metadata.json`、met/japan/loc 的部分下载头部读取）各有样本，断言 `originalWidth/Height` 的映射或丢弃；③经 `run_with` 把 `Endpoints` 指向本机 `tiny_http` 的整条路径（样本里的图片地址在测试里换成本机服务器的地址）：两个信源一个正常一个指向已关闭的端口时，有条目、`errors` 里有该信源、退出 2；全部指向已关闭端口时退出 1、文案为网络不可用、索引文件不存在；第二次检索编号不变；四个线程同时检索不同词后索引里没有重复编号、JSON 完好；`--use page` 只留可上线；`--sheet` 写出 PNG 且尺寸与列数对应、预览 404 的格子 `look` 为 `null` 且退出 2；取用：正常图片落位并登记 SHA-256 与解码宽高；截断的 JPEG、返回 `text/html` 的地址、超过 `--max-mb` 的文件都被拒收且素材区没有残留的 `.part`；贴图 `--maps all` 得到三张且字节数不符的被拒；`--dir ../assets` 与符号链接越界退出 1 且未创建目录；`.gitignore` 首次写入、已存在时不改；④首帧：对夹具里的 WebM 短片取首帧，视频与 `-poster.png` 成对登记且宽高一致，`integrity` 为 `decoded`（无浏览器时跳过，写法同 capture 测试）；`PHOENIX_UI_BROWSER=/nonexistent` 时该视频不落位、不登记，失败信息写明浏览器不可用，退出 2；⑤`audit`：三类发现各一例（`credit` 的断言含短署名与仅出现 `license`/`licenseUrl` 的文本）与全清后的结果；没有清单时退出 0 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib media --locked --offline -- --test-threads=1`；仓库根 `node scripts/check-phoenix-lints.mjs` | M1–M2 交付齐备后（批 1） |
| V-3 | 局部 | 准入：WORK-N1/AC-4 是明示的安全要求，“没有发出请求”从命令输出里看不出来，必须在网络侧观察。`crates/cli/tests/network_boundary.rs`★：测试里起一个只记录首行并回 502 的 TCP 监听作为代理。①把 `ALL_PROXY`、`HTTPS_PROXY`、`HTTP_PROXY` 指向它，运行 `context --target <夹具项目>`、`hook`（SessionStart 负载，同冒烟脚本的调用）、`detect --no-config --json page.html`：代理记录为空，退出码与不设代理时相同；②用一个包一层的脚本作 `PHOENIX_UI_BROWSER`（在真实浏览器参数前加 `--proxy-server=<监听地址>`；本机回环地址默认不走代理），对本机夹具服务器上的页面运行 `detect <url>` 与 `capture <url>`：自包含的页面下代理记录按两类分开记录且只含两类：引擎自身的请求（应为空）与 §5.16 已登记的浏览器残余主机（清单为空时记录为空），不得出现清单外的其他主机；页面引用 `http://boundary-probe.invalid/x.png` 时记录里多出且只多出这个主机；③同样的代理变量下运行 `media search map`：代理记录里出现信源主机的 CONNECT，命令退出 1 并报网络不可用。无浏览器时②跳过并打印跳过原因 | `mise exec --no-deps -- cargo test -p phoenix-ui --test network_boundary --locked --offline -- --test-threads=1` | M2 交付齐备后（批 1） |
| V-4 | 场景 E2E | **前端开发者把设计产出的大图处理成可以上线的尺寸。** 角色：只读过 `phoenix-ui prep-image --help`。输入（临时项目的 `src/assets/`）：3000×2000 的不透明 PNG 主图（已用 `embed-prompt` 写入来源）、1200×1200 带透明底的 PNG 图标（已写入来源）、一张 EXIF 方向为 6 的 JPEG 照片（已写入来源）、一张 800 宽的 PNG（已写入来源）、一个内容损坏的 `.png`、一张四周留有透明边的 PNG（已写入来源）。步骤与结果：①`prep-image src/assets/hero.png src/assets/logo.png src/assets/photo.jpg src/assets/small.png src/assets/broken.png --width 600 --widths 600,1200 --out public/img`：主图得到 600 与 1200 宽的 JPEG，图标得到保留透明的 WebP，照片方向正确，800 宽的图没有被放大且报告注明，损坏的文件失败并写明原因、其余照常，退出 2（WORK-F9/AC-1、AC-5）；②终端里的片段贴进 `index.html` 后图片路径都能解析（AC-2）；③处理前后原图的 `shasum -a 256` 相同；对输出运行 `embed-prompt --read` 得到与原图相同的文字，`embed-prompt --scan public/img` 退出 0（AC-3）；④`prep-image src/assets/hero.png --width 600 --crop 16:9 --out public/img` 与 `prep-image src/assets/padded.png --width 300 --trim --out public/img`：输出符合 16:9、透明边被裁掉，报告列出每个输出的像素尺寸与字节数（AC-4）；⑤把图标的输出放在浅色与深色底上查看，边缘没有暗边。Agent 逐个打开输出查看 | `target/debug/phoenix-ui prep-image …` 与 `target/debug/phoenix-ui embed-prompt …`，在临时项目目录里运行 | M3 交付齐备后（批 2）；M4 的 V-7 的前置 |
| V-5 | 局部 | 准入：宽度集合、裁切矩形、透明边外接矩形与命名是分支多的纯逻辑；“不写输入文件”是 §8 列出的不可恢复路径；来源的三种载体是明示的等价要求。内容：输出宽集合（2 倍、去重、不超过原图宽并出说明）；`--crop` 的矩形（横图、竖图、比例等于原图；0.01 与 100 等会产生零几何的极端比例下 `cw`/`ch` 与输出高四舍五入后仍 ≥1）；`--trim` 的外接矩形、整张全透明报错、没有透明时不裁；按像素判定透明；输出路径落在输入上时拒绝且不写文件；两个不同目录的同名输入加共享 `--out` 时按冲突失败，不发生覆盖；片段的升序、`sizes` 默认值与路径编码；目录输入缺 `--out` 报错；APNG 与动画 WebP 被拒；`read_provenance` 为空的输入被拒绝且无新输出；`read_provenance`/`write_provenance` 对 PNG、JPEG、WebP 的往返，以及 JPEG 文字超过一个 COM 段时改写旁注 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib prep_image --locked --offline`；`mise exec --no-deps -- cargo test -p impeccable-context --lib embed_prompt --locked --offline`；仓库根 `node scripts/check-phoenix-lints.mjs` | M3 交付齐备后（批 2） |
| V-6 | 局部（静态） | §6 的文本落位与 §0.6 对得上（WORK-N4/AC-2、C-8、C-9、P2-D01）：渲染为 `fresh`；生成的 new-work、visualize、optimize、typeset 与 SKILL.md 里有 §6.1–§6.5 的内容；`git diff --stat` 的 skill 源改动只有 §6 列出的文件；完工评审与素材制作子代理无 diff；docs/phoenix-ui.md 有两节新内容与联网边界表；`phoenix-ui --help` 列出两个新命令；phoenix-ui skill 的元数据测试通过，其他 skill 的既有失败只报告 | `tools/phoenix-ui` 下 `mise exec --no-deps -- bun scripts/render.mjs`，再 `mise exec --no-deps -- bun scripts/render.mjs --check`；仓库根 `grep -n 'media\|prep-image' skills/phoenix-ui/reference/new-work.md skills/phoenix-ui/reference/visualize.md skills/phoenix-ui/reference/optimize.md skills/phoenix-ui/SKILL.md docs/phoenix-ui.md README.md`、`grep -n 'inline-block' skills/phoenix-ui/reference/typeset.md`、`git diff --stat -- skills/phoenix-ui/src docs README.md tools/phoenix-ui/docs`、`git diff -- skills/phoenix-ui/src/subagents`；`python3 -m unittest discover -s test/skills -p 'test_skill_metadata.py'`，只按 phoenix-ui 的子测试判定 | M4 交付齐备后（批 3） |
| V-7 | 场景 E2E（有界演练） | **设计负责人让 agent 给一张品牌专题页的预览稿配一张真实的古地图，再把一张大图处理成上线尺寸。** 角色：Claude Code 新会话，只有项目文件与已安装的 Phoenix skill。项目：仓库树外的临时目录，含 PRODUCT.md、DESIGN.md、一个 `.phoenix-ui/mocks/preview.html` 预览页、`src/` 下的产品页面与一张带来源的 3000 宽 PNG；`.claude/` 用渲染产物 `tools/phoenix-ui/dist/providers/claude-code/.claude/` 的副本，其中 `skills/phoenix-ui/scripts/phoenix-ui` 换成调用本地 debug 二进制的垫片（仅演练）。回合一：“用 Phoenix 给预览稿找几张真实的古地图候选让我挑”。结果（看会话记录与文件）：调用前说明了在找什么；用了 `media search … --sheet` 并打开联系表；向用户展示候选时带分级；本回合没有取用。回合二（同一项目目录、另一次调用，把用户实际选定的编号发给宿主，如“就选 M3，放进预览稿”）：`media fetch <选定编号>` 把文件取到 `.phoenix-ui/media/`；素材只出现在 `.phoenix-ui/` 下的预览页；`src/`、`public/` 没有素材的副本、引用或署名；结束前运行了 `media audit` 且无发现（WORK-F8/AC-3、WORK-N1/AC-4 的调用可见）。单次非交互调用接收不到用户的编号选择，不得用一次输出代替选择回合。请求二：“把 src/assets/hero.png 按 600 的显示宽度处理成上线尺寸，给我 img 片段”。结果：用了 `prep-image --width 600 --widths 600,1200`（两个及以上输出才给片段，§5.14），采用了它给出的片段，没有放大或覆盖原图，`embed-prompt --scan` 干净 | `claude -p "<回合请求>" --output-format stream-json --verbose`（回合一、回合二、请求二各一次，同一项目目录），权限参数按 `claude --help` 实际选择并只授予本场景所需；保存输出后按工具调用、最终答复与文件判定 | M4 交付齐备且 V-1、V-4 通过后（批 3） |
| V-8 | 阶段收口 | 构建、许可与 Phoenix UI 全量回归：锁文件只多依赖边，`--write` 后清单检查通过，NOTICE 与 THIRD-PARTY-NOTICES 的 oil-ui 条目含本期改编范围并进入 `dist/phoenix-ui`（WORK-N4）；oracle 新增 media 与 prep-image 的离线用例（`--help`、未知选项、缺参数、`fetch --dir` 越界被拒、`audit` 无清单、`prep-image` 缺宽度）并重录 `cli-help`，其余 golden 逐字节不变（embed-prompt 的用例不变，证明 §5.15 没有改变 verb 行为）；开发构建与冒烟通过，冒烟新增：经启动器 `media --help`、`prep-image --help` 退出 0；空 `PHOENIX_UI_HOME` 下 `media --help` 退出 4 并给补装命令；在冒烟的黑洞代理下 `media search map` 退出 1 并报网络不可用（记录为 `network-unavailable`）；`media fetch T1 --dir out` 被拒；对冒烟脚本内嵌的一张小 PNG 运行 `prep-image --width 100`，得到 JPEG 输出与“未放大”的说明（记录为 `processed`）（WORK-N2/AC-3 的开发候选证据：开发构建与暂存新装；不证明最终 npm 包安装或旧版升级，那由既有发布检查在发布时覆盖，本期不执行）；`test/phoenix-ui` 下的 Node 测试与 `test:phoenix` 全绿（含 capture 的既有浏览器测试，覆盖 WORK-N1/AC-1、AC-2 与启动参数的回归）；render `--check` 为 `fresh`。不以整仓 `npm test` 为门，其 Diagram、statusline 等检查与本期无关 | `tools/phoenix-ui` 下：`mise exec --no-deps -- node ../../scripts/prepare-phoenix-ui.mjs --prepare` 并按回执导出环境变量；`mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --write`，再不带参数运行一次；重建 debug 二进制后在仓库根运行 `node tools/phoenix-ui/tests/oracle/record.mjs --bin tools/phoenix-ui/target/debug/phoenix-ui media`、`… prep-image`、`… cli-help`（逐个审阅 golden diff）；清空 `../../local/phoenix-ui/development-release` 后 `mise exec --no-deps -- node ../../scripts/build-phoenix-ui.mjs --development --output-dir ../../local/phoenix-ui/development-release`；`mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --distribution-dir ../../local/phoenix-ui/development-release`；`mise exec --no-deps -- node ../../scripts/smoke-phoenix-ui.mjs --release-dir ../../local/phoenix-ui/development-release --target darwin-arm64 --stage-runtime`（`--target` 填本机平台）；仓库根 `mise exec --no-deps -- node --test 'test/phoenix-ui/*.test.js'`、`mise exec --no-deps -- npm run test:phoenix`；`mise exec --no-deps -- bun scripts/render.mjs --check`。`test:phoenix` 含 release 构建与全量 oracle，耗时未知，只在收口运行一次 | M5，M1–M4 完成后 |
| V-9 | 真实使用验收 | 干净的夹具项目（仓库树外）、只凭公开入口与文档、用 V-8 的开发构建安装，逐项保存输出并亲自查看产物。**A1 直接使用 media**（WORK-F7/AC-1–AC-4、WORK-F8/AC-1–AC-4；PRD UAT-9、UAT-10）：以开发者身份经安装后的启动器，为“一张茶叶品牌页的设计稿”找一张浮世绘或植物版画、一块布料贴图：检索（含中文或日文关键词一次）、看联系表、取用、看来源清单；试一次把素材取到 `public/`；断网（代理变量指向 `http://127.0.0.1:9`）检索一次；记录各信源的可用情况。判据同 UAT-9、UAT-10。**A2 直接使用 prep-image**（WORK-F9/AC-1–AC-5；PRD UAT-11）：对一张自己准备的 3000 宽不透明 PNG（先用 `embed-prompt` 写入来源；显示宽 600，请求多宽度）、一张透明 PNG（已写入来源）与一个坏文件运行；判据同 UAT-11，另看输出体积是否明显小于原图。**A3 宿主会话**（WORK-F8/AC-3、WORK-N1/AC-4 的“对话中可见调用记录”；PRD UAT-9 的流程侧）：Claude Code 新会话，请求“用 Phoenix 给这个专题页出一个方向预览，需要真实的历史地图质感”；判据：素材的每次检索与取用都是会话里可见的命令调用且事先说明；素材没有进入产品文件；交付前有 `media audit` 的结果；整个会话里除 `media` 外没有别的命令访问素材信源。能生图时素材作为参考图传入，不能生图时放进 `.phoenix-ui/` 下的预览页，两种都算通过，记录是哪一种。**A4 联网边界**（WORK-N1/AC-4 终验）：在 macOS 上用 `sandbox-exec` 的只放行本机的配置（scripts/phoenix-build-tools.mjs 的 `testNetworkProfile`）运行安装后的 `context`、`hook`（SessionStart 负载）与 `detect <源码文件>`，输出与不加 sandbox 时相同；同一配置下 `media search map` 报网络不可用；再用转发代理加 `--proxy-server` 包装脚本对 `https://example.com/`（页面不加载其他主机的资源）运行一次 `capture`，代理记录的主机只有 `example.com` 与 docs 里列明的残余。发现的用户可见问题修复后只复跑受影响的场景；改了引擎或 skill 源须从 V-8 的构建重做再复验受影响项 | 安装：在夹具项目里用仓库的安装入口加 `--release-dir=<仓库>/local/phoenix-ui/development-release`（docs/phoenix-ui.md「安装」选项表）；A1、A2、A4 用 `npm exec` 形式或安装后的 `skills/phoenix-ui/scripts/phoenix-ui`；A3 用 `claude -p … --output-format stream-json --verbose`；A4 的转发代理用一个几十行的 Node 脚本（转发 HTTP、隧道 CONNECT、记录主机） | M5，V-8 通过后 |
| V-10 | 阶段收口 | 整合清单、复述与跨期约束核对（WORK-N4/AC-2、C-1、C-8、WORK-N1/AC-3）：§0.6 每个采用或调整项都有落点、拒绝项无改动；独立 skill、完工评审与走查子代理、`crates/comp`、`crates/comp-verbs`、`crates/hook`、`crates/live` 无 diff，new-work.md 里派出走查的一段没有改动；两个新命令在总帮助、docs/phoenix-ui.md、对应 reference、README 都有描述；第三方声明写全改编范围；PRD 的 WORK-N1 约束段与 AC-4 已按 §6.8 补写，且只改了这两处与版本号 | `git diff 9b07ef0 -- skills tools/phoenix-ui docs README.md scripts` 逐项对照 §0.6 与 §1.2；`git diff --stat 9b07ef0 -- skills/review-ui skills/apply-ui-review skills/feature-demo skills/design-reference skills/phoenix-ui/src/subagents tools/phoenix-ui/crates/comp tools/phoenix-ui/crates/comp-verbs tools/phoenix-ui/crates/hook tools/phoenix-ui/crates/live` 为空；`grep -rn 'prep-image\|phoenix-ui media' docs/phoenix-ui.md README.md skills/phoenix-ui/reference tools/phoenix-ui/NOTICE.md tools/phoenix-ui/THIRD-PARTY-NOTICES.md`；`git diff 9b07ef0 -- docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md` | M5，V-9 之后 |
| V-11 | 阶段收口 | 四平台开发候选（WORK-N2/AC-3 的开发候选证据、A-2）：同一候选提交上 Phoenix CI 的 UI 四个平台任务与 Node 18 冒烟全部成功（CI 记录为 `distribution=development`，证明四平台构建、暂存安装与运行；最终 npm 包安装与旧版升级由既有发布检查覆盖，本期不执行也不宣称）；四个平台的冒烟记录里 media 为 `network-unavailable`、prep-image 为 `processed`；`media` 的库测试与 `network_boundary` 在四个平台运行，四平台对本期声明支持的真实 MP4/H.264 首帧用例与浏览器段须实际执行并记录，无法执行的平台记「未验证」并在收口注明，不记通过（A-2）。Diagram 任务的失败与本期无关时只报告 | 经用户授权推送 main 后：设 `candidate_sha=<M5 候选提交的完整 SHA>`，运行 `gh run list --workflow phoenix.yml --branch main -L 100 --json databaseId,headSha,status,conclusion --jq "map(select(.headSha == \"$candidate_sha\"))"`，逐条确认 `headSha` 相同后对匹配的 `databaseId` 执行 `gh run watch <run-id>`，下载各平台 evidence 制品核对冒烟记录；记录候选 SHA、run ID 与四平台结果 | M5，V-8–V-10 通过并完成候选提交后；未获授权时记入当前阻塞，不记完成 |

### 9.2 集成与回归

- 改动浏览器启动参数前，在起始代码上跑一次 `mise exec --no-deps -- cargo test -p impeccable-browser --locked --offline -- --test-threads=1`，把既有失败分型写入 M2 记录，作为 V-8 的对照。
- `test:phoenix` 只在 V-8 运行一次；它的浏览器测试段覆盖 `detect <url>`、`capture`、comp 门禁与组件评审，是启动参数改动的回归门。
- `skills/phoenix-ui/src` 的改动在 M4 当批渲染并提交生成物，render `--check` 在 V-6 与 V-8 各看一次（V-8 是开发构建之后）。
- oracle 的 `context.mjs` 里已有 embed-prompt 用例，V-8 里应逐字节不变；有差异即是 §5.15 的回归。

### 9.3 真实环境

V-1、V-4 在本机由 Agent 执行并亲自看图；V-1 与 V-9 的 A1、A3 需要外网；V-7、V-9 的 A3 需要已登录的 Claude Code；V-11 需要用户授权推送。缺任一环境时对应检查记“环境缺失/未运行”，收口不宣称相应 AC 通过。信源本身不可用（如 LoC 被拦）不算环境缺失：按 WORK-F7/AC-4 的表现判定并记录。

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 素材检索与联系表 | §0.6 I-1 已核对 | 批 1 前半。`crates/cli/src/media/` 的 `mod.rs`（`search`、`preview`、用法与输出，§5.1、§5.12）、`sources.rs`（§5.2、§5.3）、`rights.rs`（§5.5）、`relevance.rs`（§5.4）、`index.rs`（§5.6、§5.10）、`look.rs`、`sheet.rs`（§5.7）★；main.rs 的 `media` 分支（排在 :121 之前）、lib.rs 的 `pub mod media`；crates/cli/Cargo.toml 加 `image`（`default-features = false` 加 `jpeg`、`gif`、`webp`、`png`，写法同 crates/comp/Cargo.toml:24）、`ureq`（`tls`、`json`）、`regex`（workspace）与 dev 依赖 `tiny_http`；`ROOT_USAGE` 加 media 一行；`tests/fixtures/media/api/` 的响应样本。同构：参数解析与用法照 crates/cli/src/capture/mod.rs:39-306；HTTPS 调用与测试时注入地址照 crates/context/src/generate_image.rs:20-24、:75、:233-310；拼图照 crates/comp-verbs/src/comp_diff.rs:455-475；越界检查照 crates/cli/src/capture/serve.rs:149-160；各信源的行为参考 `local/oil-ui-main/scripts/curate-media.mjs`（§0.6 I-1 所列行段，Rust 重写）。main.rs、lib.rs、Cargo.toml、`ROOT_USAGE` 由主 agent 单人修改 | 与 M2 同批，V-1、V-2；回写「实施进度」 |
| M2 | 素材取用、视频首帧、交付前核对与联网边界 | M1 代码 | 批 1 后半，连续实施。`fetch.rs`（§5.8）、`poster.rs`（§5.9）、`audit.rs`（§5.11）★ 与 `mod.rs` 的 `fetch`、`audit` 子命令；`tests/fixtures/media/` 的小图片、截断图片与 WebM 短片（短片用本机 ffmpeg 的 `testsrc` 一次性生成后入库，不成为运行或测试依赖）；crates/browser/src/cdp.rs 的启动参数与其单元测试（§5.16），先按 §9.2 录对照，再做残余请求的限时排查并定下残余清单；`crates/cli/tests/network_boundary.rs`★；`crates/cli/src/capture/mod.rs` 的 `mod serve;` 收窄为 `pub(crate) mod serve;`（供 `poster.rs` 复用静态服务器，`serve.rs` 本体不改）；docs/phoenix-ui.md 的「素材命令」一节按 §6.5 定稿（V-1 的公开资料前置到批 1；该文件由主 agent 单人修改，M4 再补其余小节）。同构：浏览器页面的打开与求值照 crates/cli/src/capture/mod.rs:768-830、:1131、:1237；页内脚本写成字符串常量照 `crates/cli/src/capture/page_js.rs`；二进制级测试与无浏览器跳过照 crates/cli/tests/capture.rs:23、:68-88；代理监听的写法照 crates/context/src/http.rs:152-229 的测试。cdp.rs 由主 agent 单人修改 | 批末 V-1、V-2、V-3 通过；回写「实施进度」 |
| M3 | 交付图片处理 | §0.6 I-3 已核对；开发不依赖批 1 | 批 2。`crates/cli/src/prep_image.rs`★（§5.13、§5.14）；crates/context/src/embed_prompt.rs 的 `read_provenance`、`write_provenance` 与 lib.rs 导出（§5.15）；main.rs 的 `prep-image` 分支与 `ROOT_USAGE` 一行。同构：参数解析同 M1；解码与尺寸上限的写法照 crates/comp/src/png_io.rs:160-183；来源的三种写法即 embed_prompt.rs:264-323 本身。可写路径 `crates/cli/src/prep_image.rs`、`crates/context/src/embed_prompt.rs`、`crates/context/src/lib.rs`，与批 1 的文件不重叠，可交给 subagent 与批 1 并行开发；main.rs、`ROOT_USAGE` 与 Cargo.toml 的改动由主 agent 合入 | V-4、V-5；回写「实施进度」 |
| M4 | skill 与文档 | §0.6 I-2、I-3 已核对；命令接口以 §5 为准，文本开发不依赖批 1、批 2 完成；V-7 需 V-1、V-4 通过 | 批 3。§6.1–§6.5、§6.7：`new-work.md`、`visualize.md`、`optimize.md`、`typeset.md`、`SKILL.src.md`、`evals/scenarios.md`、`docs/phoenix-ui.md`（「素材命令」一节已在 M2 定稿，本批补「交付图片处理」、浏览器与隐私改写与联网边界表）、`README.md`、`tools/phoenix-ui/docs/ENGINE.md`；运行渲染并提交生成物。docs/phoenix-ui.md 里浏览器残余请求的清单以 M2 的排查结果为准。同构：docs/phoenix-ui.md「取证命令（capture）」一节的写法；new-work.md:146 对 capture 命令行的写法；evals 第 15–21 条。可写路径 `skills/phoenix-ui/src`、`skills/phoenix-ui/evals`，可交给 subagent 并行；`docs/phoenix-ui.md`、`README.md` 由主 agent 单人修改 | V-6、V-7；回写「实施进度」 |
| M5 | 阶段收口：声明、构建同步、全量回归、真实使用验收与四平台候选 | M1–M4 完成 | 依次：①§6.6 的声明与 `--write`；②oracle 的 media、prep-image 用例与 `cli-help` 重录；③`smoke-phoenix-ui.mjs` 增加两个命令的冒烟；④开发构建与 V-8；⑤用同一开发构建跑 V-9，修复后只复跑失效项；⑥§6.8 的 PRD 两处（约束段与 AC-4，按最终的残余清单措辞）；⑦V-10 并在其后创建候选提交，记录完整候选 SHA；⑧经授权推送，以该 SHA 精确匹配 run ID 跑 V-11 并回填。改动 skill 源或引擎源码后须从④重建再复验受影响项。同构：P1 收口对冒烟脚本的增补（scripts/smoke-phoenix-ui.mjs:100-123）、oracle 的 capture 用例（tools/phoenix-ui/tests/oracle/cases/capture.mjs） | V-8、V-9、V-10、V-11；回写「实施进度」 |

提交点：M1、M2、M3、M4 各自交付后分别提交（M1 与 M2 同批，不等批末检查；M4 的提交含渲染生成物）；M5 的⑦通过后做候选提交；全部里程碑完成后最终提交一次并回填 SHA。推送仅在用户授权时进行（C-11）。

并行：M1 与 M2 共用 `media/mod.rs` 与素材区接口，串行；M3 与 M4 的可写路径和批 1 不重叠，可由 subagent 并行开发，主 agent 负责共享文件（main.rs、lib.rs、Cargo.toml、Cargo.lock、`ROOT_USAGE`、cdp.rs、docs/phoenix-ui.md、README.md）、渲染与各批验收；V-9 的四个场景共用一个开发构建，夹具目录、端口与浏览器资料目录各自独立，可顺序或并行。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1：美国国会图书馆对非浏览器客户端返回 403 | 本机网络下 LoC 的照片、地图、图纸三个任务恒为不可用；`blueprint` 靠 Commons、`photo` 与 `map` 靠 Commons 与 NASA 仍有结果。LoC 的响应映射只有自写样本的测试，没有真实响应的证据 | 按 WORK-F7/AC-4 报不可用，不做绕过；V-1、V-9 记录实际结果。LoC 恢复可用时由维护者用真实响应补一份样本 | M2 批末（记录）；发布后（补样本） | 否 |
| 信源接口变更或下线（PRD Q3） | 个别信源突然不可用或字段变化，表现为该信源报错或条目缺字段 | 报不可用不影响其他信源；维护者更新 §5.2 的写法与样本。本期已遇到两例并已按现状写入：大都会的 `v1/search` 下线、芝加哥艺术博物馆要求专用请求头 | 发布后 | 否 |
| 浏览器后台请求的残余（D15） | Google Chrome 上经参数压到 1 个请求，能否清零待 M2 排查；Edge、Brave、Chromium 的后台联网没有验证，同一组参数未必对它们有效 | M2 排查并定残余清单；docs 写明验证范围；用户已确认残余如实记录、PRD 补一句 | M2 批末；M5 写入 PRD | 否 |
| A-2：视频首帧依赖浏览器的解码器 | 不带 H.264 的 Chromium（常见于 Linux）取不出 NASA 的 MP4 首帧，该视频取用失败（§5.8），该平台视频能力记「未验证」、不记通过 | 按 D9 报出；V-11 实际执行并记录各平台首帧用例的结果，无法执行时记「未验证」 | M5 | 否 |
| 首帧可能是黑场 | 以淡入开头的视频得到黑图，作预览用处有限 | 按 PRD 的“首帧”字面实现；docs 写明。用户反馈需要有内容的一帧时再议取帧位置 | 发布后 | 否 |
| 联系表文字只有 ASCII | 格内不显示中文标题，要对照终端或 JSON 的标题看 | 编号、分级与尺寸满足 WORK-F7/AC-2；需要标题时按 ADR-2 的触发条件重评 | 发布后 | 否 |
| `audit` 的 `credit` 类发现可能误报 | 署名文字（如机构名）碰巧出现在产品文案里 | 发现只作提示，由主代理判断并在交付报告写依据（§5.11、§6.1） | 发布后 | 否 |
| JPEG 体积 | `image` 的编码器是基线编码，输出比 mozjpeg 或有损 WebP 大 | V-4、V-9 A2 记录实际体积；成为问题时按 ADR-1 重评 | M5 | 否 |
| 推送授权与 CI | V-11 不能执行，M5 不能记完成 | 收口前向用户申请推送；未授权时记入当前阻塞 | M5 | 否（影响收口结论，不影响实施） |
| 宿主会话与外网 | V-7、V-9 的 A3 无法执行，或信源在验收时整体不可达 | 运行前用 `claude --help` 核对参数并用最小会话确认 skill 可加载；外网不可达时按 §9.3 记未运行 | M4、M5 | 否 |

- 最终状态：**Ready**。
- 定级理由：范围与期次由 PRD v1.2 确定，P3 没有前置期次；命令名（PRD Q1）在 D2 定下，信源现状（PRD Q3）已逐个实测并写入 §5.2；四项需要用户决定的取舍（测绘图纸加查 Commons、素材区自忽略、输出格式、浏览器后台请求的收口口径）已由用户 2026-11-11 确认。视频首帧与浏览器启动参数的效果有本机实测；残余请求的排查有两种结果各自对应的路线，不需要再做决定。没有影响范围、安全、数据或外部契约的未决项。推送授权、宿主会话与外网只影响收口检查能否执行，按 §9.3 记缺口。

## 12. 已知坑与历史教训

- 本机 Bash 是 zsh：未加引号的变量不按空格拆分，带通配符的参数要加引号（用户全局规则）。拼浏览器参数或 `cargo` 参数时用数组，否则整串会被当成一个参数。
- 改 `crates/cli/Cargo.toml` 后 `--locked` 的构建会失败，先不带 `--locked` 跑一次 `cargo check --offline` 补依赖边；锁文件变了，`dependency-audit.json` 里的摘要也要用 `check-phoenix-deps.mjs --write` 重算（tools/phoenix-ui/THIRD-PARTY.md）。
- `crates/`、`browser-bundle/` 下新增的 js、css、html 与字体文件会进许可清单（scripts/check-phoenix-deps.mjs:457-466）：首帧脚本写成 Rust 字符串常量，夹具放 `tools/phoenix-ui/tests/fixtures/`。
- 发行版 `panic = "abort"`（tools/phoenix-ui/Cargo.toml:38）：接口 JSON、下载内容、图片与索引文件都是外部输入，不 `unwrap`、不越界索引。
- 冒烟把代理变量指向 `http://127.0.0.1:9`（scripts/smoke-phoenix-ui.mjs:81）：冒烟里的 `media search` 只能断言“网络不可用”，不能断言有结果。
- macOS 上 Chrome 无法在 `sandbox-exec` 内初始化沙箱（scripts/test-phoenix-ui.mjs:45-47）：首帧与联网边界的浏览器段要在 sandbox 外跑；`cargo test --workspace` 段在只放行本机的 sandbox 内，测试里的服务器只能绑 127.0.0.1。
- `file://` 页面里的 canvas 画过视频后会被污染，`toDataURL` 抛错：首帧必须经本机 HTTP 地址打开视频（§5.9）。静态服务器不支持 Range，实测不影响取第 0 帧；它拒绝点文件，素材区的 `.gitignore`、`.part` 临时文件不会被页面读到。
- 静态服务器把整个文件读进内存（crates/cli/src/capture/serve.rs:159）：取首帧时视频最大就是 `--max-mb` 的值。
- Chrome 的后台请求要在“健康”的网络下观察：用只回 502 的代理也能看到主机，但排查发起方时用转发代理加 `--log-net-log` 才能看到完整 URL（§1.1 的实测做法）。
- 大都会博物馆的 `v1/search` 已下线，芝加哥艺术博物馆不带 `AIC-User-Agent` 会返回校验页，LoC 在本机返回 403（§1.1）：样本与测试按现状写，不照 oil-ui 测试夹具里的旧地址。
- `image` 的默认解码上限是 512 MB（io/limits.rs:54）：超大原图解码失败是预期行为，按 §5.8 该文件取用失败并写明原因；不要去掉上限，也不要恢复「只读头部尺寸」的登记路径。
- `crates/cli` 打开 `image` 的 `png` 特性后，经特性合并 `crates/comp` 里的 `image` 也能解 PNG；comp 的代码先走自己的 `decode_png`，不要顺手改它。
- oracle golden 逐字节比较，`record.mjs` 只重录有意变化的用例并逐个审阅 diff（tools/phoenix-ui/tests/oracle/README.md:15-20）；`embed-prompt` 的 golden 不应变化。
- 改 `skills/phoenix-ui/src` 后不渲染，`GENERATED.json` 失配，`render --check` 与 `test:phoenix` 会失败（docs/phoenix-ui.md「维护者」节）：M4 渲染并提交生成物。
- 宿主会话演练的三个坑（docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md「V-9 演练偏差」）：夹具放仓库树外；skill 用副本而不是指向会话目录外的符号链接；`claude -p` 在会话留有后台进程时会先输出结果再挂起。P2 收口时宿主会话要显式继承本机已有的 `HTTP(S)_PROXY` 才能连通（docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md）。
- 开发构建没有引擎下载地址，安装必须带 `--release-dir`（docs/phoenix-ui.md「安装」选项表）；`smoke-phoenix-ui.mjs --stage-runtime` 重跑前清理旧的开发构建输出。
- `local/` 被 `.gitignore` 忽略：oil-ui 只作行为参考，夹具与样本一律自备在 `tools/phoenix-ui/tests/fixtures/media/`。
- 下次发布不能用 `--reuse-engines`：引擎源码已变（docs/phoenix-ui.md「维护者」节的复用条件）。
