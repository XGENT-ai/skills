# PHOENIX-UI-EVIDENCE-CRAFT-P2 · XGENT-ai Skills · Phoenix UI 中文排版检测与口径（五条 `xgent/cjk-*` 页面规则 · 原生排版探针 · skill 中文口径）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-11 · `45e3844`（main）· dirty：包含本计划、评审报告、`.xgent-ai/sdlc/` 台账及 P1 gaps 评审报告；相关引擎与 skill 源码无未提交改动；下文事实取自当前工作树。
>
> 需求来源：[Phoenix UI 取证与工艺增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.2 · P2（第 2 期）：WORK-F5/AC-1–AC-7、WORK-F6/AC-1–AC-4、WORK-N3/AC-3，以及每期都要满足的 WORK-N4/AC-2。
>
> 本期交付：**`detect <url>` 与 `capture` 的页面规则新增 5 条 `xgent/cjk-*`，检出中文正文的紧行、长行、孤字、数字单位拆行与汉字字体意外回退；Phoenix skill 给出 zh-CN / zh-TW 排版口径，并把设计稿里的伪汉字按稿件缺陷处理**。
> **开发范围：`tools/phoenix-ui` 引擎（`crates/rules`、`crates/browser`、`crates/foundation`、`crates/core` 的规则包钩子、`crates/detect` 与 `crates/cli` 的备注输出、测试夹具）、`skills/phoenix-ui/src` 中的 typeset、craft-floor、new-work 与完工评审子代理、`docs/phoenix-ui.md` 与 `tools/phoenix-ui/docs/ENGINE.md`；依据见 §0**。
> 本期独特职责：内置 `line-length`、`tight-leading` 只按拉丁口径判定；本期让以汉字为主的正文改由中文规则判定，并第一次让规则读到“每一行实际排了哪些字”和“汉字实际由哪个字体渲染”。
> **顶层排除：live 叠层、浏览器扩展与设计 hook 不运行这 5 条规则；不判日文、韩文；不改拉丁文本的判定；不改 review-ui 等独立 skill；发布与 SPMS 录入不在本计划内**。

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

- 最近更新：2026-11-11T15:02+08:00
- 当前进度：4/4 个里程碑完成
- 当前状态：全部里程碑完成。批 1、批 2 与阶段收口的检查 V-1–V-9 均通过；验收中发现并修复两处问题（探针读字体前未排版 `532f8d8`；macOS 断言写死兜底字体的系统名 `3d764e1`）
- 最近完成：M4
- 下一步：代码评审。由用户发起 `review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md`；评审从完成记录定位提交
- 当前阻塞：无
- 代码基线：`3d764e1`（main，已推送至 `f6cc578`；其后只有计划与记录文档的提交）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-11-11T11:24+08:00 | 排版探针、`Dom` 两个事实方法、内置规则让位钩子与备注通道；页内包已重新生成。批末检查与 M2 共用。V-7 发现读字体前未排版的缺陷，已修。 | [M1 记录](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md) | 70fa92e 修复 532f8d8 |
| M2 | 已完成 | 2026-11-11T11:24+08:00 | 五条 `xgent/cjk-*` 规则与夹具；真实页面上的发现、开关、非中文对照、capture 与延迟字体备注均符合 V-1，V-2、V-3 通过。V-9 暴露 macOS 断言写死了兜底字体的系统名，已改。 | [M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M2.md) | e69c600 修复 3d764e1 |
| M3 | 已完成 | 2026-11-11T11:31+08:00 | typeset 中文口径、伪汉字条款、`findingsNotes` 的消费与披露、规则文档与渲染生成物；V-4 通过，V-5 演练会话按中文口径整理页面并跑了页面扫描，复查无发现。 | [M3 记录](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M3.md) | 76814c1 |
| M4 | 已完成 | 2026-11-11T15:02+08:00 | 真实使用验收四个场景通过：新建中文更新日志页排版符合口径、开发者直接用 detect 的发现与 UAT 一致、设计稿伪汉字被逐条替换并列出、字体备注被人工核对并在交付披露。开发构建、全量回归、清单核对通过；四平台 CI 与 Node 18 冒烟在候选 `f6cc578` 上全部成功，Linux 上探针取数正常（A-1 成立）。 | [M4 记录](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md) | f6cc578 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号与 AC ID（尚未录入 SPMS，不是真实需求 key）。WORK-N3 的终验期是 P2：AC-1、AC-2 已在 P1 交付，本期完成 AC-3 并在收口确认三条同时成立。WORK-N4 的终验期是 P4，本期只承接 AC-2，完成后不转“已上线”。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F5 | 功能 | PRD §3 WORK-F5/AC-1–AC-7 · P2 | 页面规则新增 `xgent/cjk-tight-leading`、`xgent/cjk-line-length`（warning）与 `xgent/cjk-orphan`、`xgent/cjk-split-unit`、`xgent/cjk-font-fallback`（advisory）；只作用于以汉字为主的文本；同一段文本不再出现内置 `line-length`、`tight-leading`；默认启用、可单独关闭；字体来源无法确认时不报并注明 | §5.1–§5.9；V-1、V-2、V-7（A2） | 已确认 |
| WORK-F6 | 功能 | WORK-F6/AC-1–AC-4 · P2 | zh-CN / zh-TW 界面的字号起点、行高、行长、断行、中英混排、窄幅展示字与字体回退口径，DESIGN.md 另有规定的从其规定；comp-led 中伪汉字、乱码与繁简不符的字按稿件缺陷替换并逐条列出，其余文案逐字沿用 | §6.1–§6.3；V-4、V-5、V-7（A1、A3） | 已确认 |
| WORK-N3 | 兼容 | WORK-N3/AC-3 · P2（AC-1、AC-2 属 P1，本期持续满足并终验） | 中文规则与既有 xgent 页面规则一样默认启用、可单独关闭；hook 不产生中文规则发现；live 叠层与浏览器扩展不运行；规则表如实写明 | §5.4、§5.9、§6.4、§7；V-1、V-2、V-4、V-6、V-8 | 已确认 |
| WORK-N4 | 可维护 | WORK-N4/AC-2（每期；AC-3 属 P4） | 动 skill 文件前 §0.6 的整合清单已到本期范围；本期 diff 与清单逐项核对，新冲突在继续改行为前解决 | §0.6；V-4、V-8 | 已确认 |
| C-1 | 约束 | AGENTS.md `@skill_tool_independence` | 只有 phoenix-ui skill 随工具一起改；review-ui、feature-demo、design-md 等独立 skill 不因本期改动 | §0.5、§1.2 | 已确认 |
| C-2 | 约束 | AGENTS.md §7「Reference Skill Integration Gate」；PRD §5.3 C4、C5 | 借鉴 oil-ui 的规则先列清单再改；Phoenix 现有原则是基线，冲突项调整或拒绝 | §0.6 | 已确认 |
| C-3 | 约束 | docs/phoenix-ui.md「XGENT 自有检测规则」；tools/phoenix-ui/docs/ENGINE.md「Rule packs」 | xgent 规则包只进原生二进制；页面规则只在 `detect <url>` 与 `capture` 的页面扫描中出现，hook 与页内 WASM 检测不运行它们 | §5.4、§5.5 | 已确认 |
| C-4 | 约束 | tools/phoenix-ui/docs/ENGINE.md「The browser bundle」 | 改过 `crates/core`、`crates/foundation` 后必须 `cargo xtask bundle` 重新生成并提交页内包，否则 `bundle --check` 失败 | M1；V-3 | 已确认 |
| C-5 | 约束 | scripts/check-phoenix-deps.mjs:457-466 | `crates/`、`browser-bundle/` 下新增的 js、css、html、字体文件（`tests/` 目录除外）会进入许可清单 | §5.5（探针脚本写成 Rust 字符串常量）、§9（夹具放 `tools/phoenix-ui/tests/fixtures/`） | 已确认 |
| C-6 | 约束 | tools/phoenix-ui/Cargo.toml:38（`panic = "abort"`） | 规则与探针遇到缺失事实、零尺寸、空文本一律按“无发现”处理，不 `unwrap`、不越界索引 | §5.3；V-2 | 已确认 |
| C-7 | 约束 | AGENTS.md「Working in This Repository」接口复述同步 | 新规则在 docs/phoenix-ui.md 规则表、typeset 指引与 ENGINE.md 中完整描述 | §6.4、§6.5；V-4、V-8 | 已确认 |
| C-8 | 约束 | docs/phoenix-ui.md「维护者」节 | 改过 `skills/phoenix-ui/src` 后运行 `bun scripts/render.mjs` 并提交生成的 skill | M3；V-4 | 已确认 |
| C-9 | 约束 | AGENTS.md「Working in This Repository」 | 留在当前分支；推送只在用户授权时进行 | §10 提交点、V-9 | 已确认 |
| A-1 | 假设 | macOS Chrome 上已实测（§1.1），Linux 未在本机验证 | Linux 上的 Chromium 对探针节点同样返回渲染汉字的平台字体；没有中文字体的 CI 机器上页面扫描不报错 | V-9 四平台 CI；失败时按 §11 的路线处理；责任人：实施者；最晚 M4 | 已核实（V-9：Linux 两个平台的通用字体族夹具无未核实备注，夹具表通过） |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 形态 | 既有模块增量：5 条规则写进 `crates/rules`，经规则包的元素钩子与页面钩子运行 | 只有原生二进制的页面扫描会跑；`detect <url>` 与 `capture` 共用 `scan_loaded_page`，两者自动一致；源码文件扫描、静态 HTML 引擎、hook、live、扩展不变 | C-3；crates/browser/src/lib.rs:353；crates/html/src/engine.rs:268-272（静态引擎只调 `check_text`） |
| D2 | 规则名 | PRD 的工作名即最终名 | 5 个 ID 见 §5.3；PRD Q1 就此关闭 | PRD §5.2 Q1 |
| D3 | “以汉字为主”的判定 | 按 PRD：去掉空白与标点后，汉字数 × 2 > 可见字符数 | 普通中文文案与少量品牌词的句子都算中文；标识符或路径密集的中文技术段落不算，仍由内置规则判定（与升级前相同）。是否改判定见 §11 | PRD §2.1、§5.2 Q5；样本结果见 §1.1 |
| D4 | 内置规则让位 | 规则包新增 `replaces_builtin` 钩子，驱动循环按元素丢掉被接替的内置发现；只对中文正文候选生效，与规则是否被 `ignoreRules` 关闭无关 | 关掉 `xgent/cjk-tight-leading` 后，该段中文正文也不再有内置 `tight-leading`；非 QUALITY 标签（如 `div`）上的中文文本仍由内置 `tight-leading` 判定 | WORK-F5/AC-4；ADR-1 |
| D5 | 行长怎么量 | 每行渲染宽度 ÷（字号 + 字距），四舍五入后 > 40 计为超长，至少两行超长才报 | 量的是“这一行有多少个汉字宽”，中英混排的行按实际宽度折算；不逐字计数，不受本机字体影响 | WORK-F5/AC-2“渲染长度”；内置 `line-length` 同样按渲染行宽判定（crates/core/src/browser/quality.rs:291-317） |
| D6 | 行高取值 | 计算样式的 `lineHeight` 换算成 px 后除以字号；`normal` 不判定 | 与内置 `tight-leading` 一致；`line-height: normal` 的页面不会因本机字体不同得到不同结果 | quality.rs:624-636；crates/foundation/src/css/measures.rs:242 |
| D7 | 逐行文本与字体事实从哪来 | 原生引擎在快照之后跑一次排版探针，把事实挂到快照 DOM 上 | 不改共享的页面快照脚本；探针会在页面里临时插入并移除一个不可见节点 | ADR-2 |
| D8 | 字体意外回退的判定 | 只报三种情况：与当前文字 run 的 weight/style/stretch 匹配的 Web 字体面加载失败、范围覆盖该 run 汉字，且有实际证据确认因此回退；汉字由两个以上字体渲染且其中有声明的具名字体；没有任何声明字体渲染汉字且列表里没有通用字体族 | “声明的中文 Web 字体”落成“加载失败且范围覆盖这些汉字的 Web 字体”；范围不含汉字的拉丁子集失败不报 | PRD §2.1「汉字字体意外回退」；§5.3 |
| D9 | 字体回退发现的粒度 | 先按元素及其文字 run 独立判定；仅把已命中的同一字体栈、同一种情况合并成一条，挂在首个命中元素并写出其余命中元素数 | 一个失败的全站字体不会刷出几十条相同发现 | §2.4“与现有发现同一格式”；`xgent/font-family-count` 同为页面级一条（crates/rules/src/dom.rs:463-514） |
| D10 | “未核实”怎么注明 | 页面扫描新增备注通道：`detect` 在 stderr 输出 `note:` 行，`capture` 报告在该图条目写 `findingsNotes` | 备注不是发现，不进 JSON 发现数组，不影响退出码 | WORK-F5/AC-7；§5.7 |
| D11 | 发现文案 | 英文，写出实测值与门槛，与注册表的名称、描述同一语言 | 例：`line-height 1.40x on Chinese body text (need >=1.5)` | PRD §2.4；现有 xgent 规则文案（crates/rules/src/lib.rs） |
| D12 | 元素范围 | 正文候选与内置 `line-length` 相同（`p li td th dd blockquote figcaption`）；`h1`–`h6` 只进入孤字、数字单位拆行与字体回退 | 行高与行长两条只判正文，多行标题的行高由 skill 口径约束 | WORK-F5 行为段与规则表；crates/foundation/src/rules/text.rs:43 |
| D13 | skill 口径落位 | 主体写进 typeset.md，craft-floor.md 的 Type 一条加中文口径，new-work.md 与完工评审子代理加伪汉字条款及字体未核实备注的交接/披露；不改 operate.md | operate.md 的 `65–75ch` 是 CSS 宽度，折合约 32–37 个汉字，已在 25–40 字内 | §0.6；skills/phoenix-ui/src/reference/operate.md:16 |
| D14 | 跨平台测试 | 不依赖本机字体的夹具四个平台都断言；依赖系统字体的三类字体夹具只在 macOS 断言 | Linux 上“混用”和“无通用族”两类字体判定没有自动化证据，见 §11 | CI 不安装中文字体（.github/workflows/phoenix.yml 无字体安装步骤） |

### 0.3 ADR-lite

#### ADR-1：内置规则通过规则包钩子让位

- 状态：Accepted
- 背景与驱动：WORK-F5/AC-4 要求以汉字为主的文本不再出现内置 `line-length`、`tight-leading`；WORK-N3 与 PRD N11 要求 live 叠层和扩展的行为不变。内置规则在 `crates/core`，同一份代码编译成页内 WASM。
- 备选：
  1. 在 `check_quality` 里直接跳过以汉字为主的文本。页内 WASM 也会跳过，而它不运行中文规则，中文页面在叠层与扩展里反而少了两类发现。
  2. 页面扫描结束后在 `crates/browser` 里按结果过滤。过滤需要知道“哪些元素是中文正文”，这个判断属于规则包，仍然要新增一个钩子，且过滤点离规则执行点更远。
  3. 规则包新增 `replaces_builtin(dom, el)`，驱动循环在同一元素上先丢掉被接替的内置发现，再追加规则包发现。
- 决策：选 3。钩子默认返回空，没有规则包时驱动行为逐字节不变；页内 WASM 不带规则包，行为不变。
- 正面后果：让位与中文规则的候选判断出自同一个函数，不会出现两边口径不一；改动集中在 `driver.rs` 一处。
- 负面/中性后果：改了 `crates/foundation` 与 `crates/core`，页内包必须重新生成（C-4），WASM 字节会变但行为不变；关闭某条中文规则不会让对应内置规则回来。
- 重新评估触发：要在 live 叠层或扩展里运行中文规则（PRD N11 另立项）；或用户反馈关闭中文规则后希望内置规则恢复。

#### ADR-2：逐行文本与实际字体由原生排版探针提供

- 状态：Accepted
- 背景与驱动：孤字与数字单位拆行要知道每一行的文字；字体回退要知道汉字实际由哪个字体渲染。快照只有每行的矩形（`text_line_rects`），没有行内文字，也没有实际字体。
- 备选：
  1. 扩展共享快照脚本 `browser-bundle/15-snapshot.js`。实际字体只能经 CDP 取得，页内脚本拿不到；逐行文本会让扩展的快照体积与耗时一并增长，而扩展用不上。
  2. 只用行宽估算末行字数。分不清“好了。”与“三个字”，孤字与拆行无法判定。
  3. 对现有元素直接调用 `CSS.getPlatformFontsForNode`。结果按字体给出字形数，拉丁与汉字混在一起：`"Helvetica Neue", "Songti SC"` 的正常结果是两个字体，与“汉字混用两个字体”无法区分。
  4. 原生引擎在快照之后跑一次排版探针：页内脚本逐字取矩形得到每行文字；同时在页面里临时插入一个不可见容器，每个元素按文字 run 分别放只含该 run 汉字的探针节点，不跨元素合并字符或结果，经 CDP 读出渲染这些汉字的平台字体后移除容器。
- 决策：选 4。实测（2026-11-11，macOS，Chrome 157.0.8103.40）：只含汉字的探针节点能把字体区分开，例如 `"Helvetica Neue", "Songti SC"` 得到 `Songti SC`，`"Hiragino Mincho ProN", "Songti SC"` 得到两个字体；`visibility: hidden` 的节点有结果，`display: none` 的没有；逐字取矩形 1.2 万字约 40ms。
- 正面后果：共享快照合同不动，扩展与 live 不受影响；字体判定以浏览器的真实排版结果为准。
- 负面/中性后果：扫描期间页面 DOM 短暂多出一个节点，监听 `<html>` 子树变化的页面脚本能观察到；需要启用 CDP 的 CSS 域。引擎已有同类先例：对比度回退向 `document.head` 注入样式（crates/browser/src/screenshot_contrast.rs:265-277），`create_capture_stylesheet` 启用 DOM 与 CSS 域（crates/browser/src/cdp.rs:1889-1895）。
- 重新评估触发：真实页面因探针节点出现可复现的行为变化；或 A-1 在 Linux 上不成立。

### 0.4 职责与事实所有权

|  | 内置规则与驱动（`crates/core`、`crates/foundation`） | 排版探针（`crates/browser`） | 中文规则（`crates/rules`） | Phoenix skill 文本 |
| --- | --- | --- | --- | --- |
| 拥有 | DOM 探针接口、规则包钩子的调用时机、内置拉丁规则 | 测量：每行文字、渲染汉字的字体、Web 字体状态、无法核实的原因 | 文本分类、候选范围、五条规则的判定与文案、接替哪些内置规则 | 设计时的中文口径、伪汉字处理、什么时候跑页面扫描 |
| 不拥有 | 中文判定、任何 `xgent/` 规则 | **规则判定**（探针只测量，不出发现） | 测量手段、输出与过滤 | 检测门槛的实现 |

一句话：探针只量不判，规则只判不量；内置规则不知道中文规则的存在，只响应规则包声明的接替关系。

### 0.5 明确不在本期

- **live 叠层、浏览器扩展、设计 hook 运行中文规则**——PRD N11，另立项；本期不改 `browser-bundle/15-snapshot.js` 的行为与 WASM 侧规则。
- **日文、韩文排版规则**——PRD N10。汉字过半的日文标题会被当作中文判定孤字与拆行，属已知边界。
- **`line-height: normal` 的判定**——与内置规则一致不判；结果随本机字体变化，不作为门槛。
- **拉丁单位（如 `8 GB`、`16 px`）被拆行**——本期只认汉字单位与 `% ‰ ℃ °` 等符号。
- **源码文件扫描与静态 HTML 引擎上的中文规则**——页面规则只在浏览器渲染后判定。
- **review-ui、apply-ui-review、feature-demo、design-md 等独立 skill**——不改（C-1）。
- **P1、P3、P4 的需求**——各自的计划承接；WORK-N1、WORK-N2 本期没有 AC。
- **发布、npm 版本与 SPMS 录入**——按维护流程另行执行。引擎源码已变，下次发布不能复用旧引擎。
- **检测耗时基线、压测与调优**——后续可选；本期只保证探针与规则不做明显低效的事（只测含汉字的候选元素）。

### 0.6 参考整合清单（动 skill 与规则前核对；WORK-N4/AC-2、C-2）

参考对象是本机 `local/oil-ui-main`（oil-ui 0.33.2，MIT；许可声明已在 P1 进入发布包）。本期只采用方法与数值，不复制代码。

| # | 目标文件/规则 | 现有行为 | 参考来源/段落 | 采用、调整或拒绝 | 对原则、调用、接口与验收的影响 | 核对 |
| --- | --- | --- | --- | --- | --- | --- |
| I-1 | 页面规则 `xgent/cjk-*` 与内置规则让位（PRD C4） | 行高 ≥1.3、行长 80 字符，不分文字（quality.rs:291-317、:624-636） | `scripts/shoot.mjs`:891-894（中文行高门槛 1.4，`normal` 按 1.2 计）；`references/visual-language.md`「可读性是交付门槛」 | 调整：门槛取 PRD 提案 1.5 与 40 字；拒绝把 `normal` 当 1.2 计（D6） | 只改变原生页面扫描中以汉字为主的正文的发现集合；hook、live、扩展与拉丁文本不变；warning 会影响退出码 | V-1、V-2 |
| I-2 | `skills/phoenix-ui/src/reference/craft-floor.md` Type 一条（PRD C5） | `body measure 65–75ch`，无中文口径 | visual-language.md:38-46 | 调整：追加一句中文口径并指向 typeset.md；文件开头“pinned brief 与已定视觉世界优先”的原则不动 | 完工检查对中文正文按 25–40 字、1.6–1.75 行高核对；DESIGN.md 仍优先 | V-4、V-7（A1） |
| I-3 | `skills/phoenix-ui/src/reference/typeset.md`（PRD C5） | 正文 `45–75 character measure`（:21、:47）；行高“按语言调”但无数值 | visual-language.md:31、:38-46 | 采用字号起点、行高、行长、断行、中英混排、窄幅展示字与字体回退核对；调整为“起点而非硬规定，DESIGN.md 另有规定的从其规定”；机械扫描补页面扫描 | 不改字体选择与方向规则；typeset 多一次带 `--viewport` 的页面扫描，仍在 craft-floor 规定的同一轮内 | V-4、V-5 |
| I-4 | `skills/phoenix-ui/src/reference/new-work.md` 首屏文案条款（:123）与 `src/subagents/phoenix-ui-finish-reviewer.md` Fidelity 检查（PRD C5） | 设计稿文字逐字沿用；偏离须引用用户答复、surface brief、无障碍需要或产品事实，否则算缺陷 | `references/image-ideation.md`:127-128 | 调整：只有伪汉字、乱码、与界面语言不符的繁简字可替换，替换为产品事实或用户确认的文案并逐条列出；拒绝“允许自由微调、润色和补全”与“繁体一律改简体” | comp 的测量合同、门禁与四个处置词不变；评审把列明的替换归为可接受的改编 | V-4、V-7（A3） |
| I-5 | oil-ui 的截图可读性检查脚本、生图提示词模板、截图还原时的自由润色 | — | shoot.mjs:763-900；image-ideation.md | 拒绝：可读性由 detect 页面规则承担；其余见 PRD §1.3 N4、N5 | 无改动 | V-8 |

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·足够]** `detect <url>` 与 `capture` 共用 `scan_loaded_page`：注入快照脚本、捕获一次快照、在原生侧对 `SnapshotDom` 运行 `collect_browser_findings`（crates/browser/src/lib.rs:353-378；capture 的调用在 crates/cli/src/capture/mod.rs:1441-1457）。规则包经 `ScanOptions.rule_pack` 传入（crates/detect/src/engines.rs:15-27）。
- **[已核实·足够]** 驱动循环按元素先跑全部内置元素规则，再追加 `pack.check_element_dom`，随后统一过滤（crates/core/src/browser/driver.rs:1366-1406）；页面级规则包钩子在所有内置页面检查之后运行（:1478-1480）。原生页面扫描的 `disabled_rules` 为空（crates/browser/src/snapshot_engine.rs:185-202），`detector.ignoreRules` 在扫描结束后由 `filter_detection_findings` 过滤（crates/detect/src/config.rs:732；crates/detect/src/cli.rs:604），`--no-advisory` 在其后生效（cli.rs:607-609）。
- **[已核实·足够]** 内置 `line-length`：`QUALITY_TEXT_TAGS` 内、直接文本超过 10 个 UTF-16 单位、至少两行按行宽折算超过 85 字符时报（quality.rs:291-317）；`tight-leading`：直接文本、全文超过 50 单位、非标题、行高/字号 < 1.3 时报，不限标签（:624-636）。
- **[已核实·足够]** `Dom::text_line_rects` 给出元素全部渲染文字（含后代）按行合并后的矩形，拿不到时返回 `None`（crates/foundation/src/browser/dom.rs:190-192、:214；快照实现在 snapshot.rs:910-924）。`SnapshotDom::add_facts(&self, …)` 用内部可变性追加事实（snapshot.rs:584）。`FakeDom::set_text_lines` 供单元测试设行（fake_dom.rs:174）。
- **[已核实·足够]** 快照在页面里保留元素表：`window.__impCap.elements[id]` 即快照 id 对应的元素（browser-bundle/15-snapshot.js:703-708；crates/browser/src/snapshot_engine.rs:56-67）。
- **[已核实·足够]** 既有页面规则的真实浏览器测试：`crates/cli/tests/xgent_rules.rs` 用发布形态的二进制对 `tools/phoenix-ui/tests/fixtures/xgent/*.html` 在 375x812 与 1280x800 两个视口跑 `detect --json`，按表断言 `xgent/` 发现（:262-319）；`crates/rules/tests/dom.rs` 用 `FakeDom` 测规则逻辑；`crates/rules/tests/text.rs`:574-588 写死了注册表的 ID 列表；`crates/core/tests/rule_pack.rs` 保证没有规则包时输出不变。
- **[已核实·足够]** 实测（2026-11-11，macOS，Chrome 157.0.8103.40；一次性脚本未入库，结论由 V-1、V-2 的夹具复现）：
  - `CSS.getPlatformFontsForNode` 必须先 `CSS.enable`；对只含汉字的探针节点返回渲染这些汉字的字体及字形数。各声明下的结果：`sans-serif`、`system-ui`、`"Noto Sans SC", "Microsoft YaHei", sans-serif`（两个具名字体本机未装）均为 `PingFang SC`；`"Songti SC", serif` 与 `"Helvetica Neue", "Songti SC", sans-serif` 为 `Songti SC`；`"Helvetica Neue"`（无通用族）与不存在的字体名为 `PingFang SC`；`"Hiragino Mincho ProN", "Songti SC"` 配简体文本为两个字体（25 与 12 个字形）。
  - `document.fonts` 给出每个 `@font-face` 的 `family`、`status`、`unicodeRange`：加载失败为 `error`（http 与 file:// 下的缺失文件都是），默认范围为 `U+0-10FFFF`，声明了范围的原样返回（如 `U+0-FF`）。
  - 对文本节点逐字取 `Range` 矩形能还原每行文字；行尾的空格留在上一行末尾。
  - `width: min-content; word-break: keep-all` 的段落只在空格处换行，与字体无关：`…中文正文内容 了。` 稳定排成两行且末行为“了。”，`…总共耗时38 秒才最终全部完成` 稳定在“38”与“秒”之间换行。宽 52em 的中文段落整行宽度为 51–52em。
- **[已核实·足够]** “以汉字为主”按过半数判定的样本结果：产品文案（如“提交后我们会在 3 个工作日内完成审核…”）汉字占 97%，繁体句 100%，带品牌词的句子（“Phoenix UI 是 XGENT 自主维护的设计工具…”去掉路径后）约 66%；假名为主的日文 38%、韩文 0%、英文夹中文名 5%，都不触发。标识符密集的技术句（“使用 React 18 的 useEffect 处理 API 请求…”）只有 25%；本仓 `docs/phoenix-ui.md` 的 79 个中文段落里有 58 个不过半。汉字过半的日文标题（“東京都知事選挙結果速報…”）95%，会被当作中文。
- **[已核实·缺口]** 规则读不到每行文字与实际字体 → `Dom` 增加两个默认返回 `None` 的方法，由排版探针提供事实（§5.5、§5.6）→ 漏做则孤字、拆行、字体回退三条无法判定。
- **[已核实·缺口]** 规则包不能让内置发现让位 → `RulePack::replaces_builtin` 与驱动循环一处过滤（§5.4）→ 漏做则中文正文同时出现两套行高/行长发现，违反 WORK-F5/AC-4。
- **[已核实·缺口]** 页面扫描只返回发现数组，没有“未核实”的出口（`UrlEngine::detect_url` 返回 `Vec<Finding>`，engines.rs:61-71）→ `ScanOptions.notes` 备注通道（§5.7）→ 漏做则 WORK-F5/AC-7 的“注明未核实”无处呈现。
- **[已核实·缺口]** skill 的排版口径只有拉丁数值：typeset.md:21、:47 为 45–75 字符；craft-floor.md:12 为 65–75ch；new-work.md:123 要求设计稿文字逐字沿用且没有伪汉字例外 → §6 的文本增量 → 漏做则 WORK-F6 四条 AC 无依据。
- **[已核实·足够]** skill 源在 `skills/phoenix-ui/src`，`bun scripts/render.mjs` 生成 `skills/phoenix-ui/{SKILL.md,reference,subagents,…}`、`GENERATED.json`、入库的 `dist/phoenix-ui/bundle/`（blob 与 `manifest.json`，随 skill 一并提交）与（不入库的）`tools/phoenix-ui/dist/providers/`（tools/phoenix-ui/scripts/render.mjs:106-131；`.gitignore`:53）；源文件里的 `<!-- rule:… -->` 标记在渲染时剥除，新增文字不需要登记标记（scripts/lib/utils.js:521-534）。
- **[已核实·足够]** 四个处置词与完工评审的输出合同在 `phoenix-ui-finish-reviewer.md` 的 Disposition 段，本期增加 Fidelity 的伪汉字条款及现有取证包中字体未核实备注的消费语义，不动处置词或输出结构。

### 1.2 拓扑与文件清单

全部在本仓 `XGENT-ai/skills`。★ 为新增。

- 引擎 `tools/phoenix-ui/`
  - `crates/foundation/src/browser/dom.rs`：`RenderedFont`★、`FailedWebFont`★、`HanFontRun`★、`HanFontFacts`★、`Dom::text_line_texts`★、`Dom::han_font_facts`★
  - `crates/foundation/src/browser/snapshot.rs`：`SnapshotDom::set_typography`★ 与两个方法的实现
  - `crates/foundation/src/browser/fake_dom.rs`：`set_text_line_texts`★、`set_han_fonts`★
  - `crates/foundation/src/rule_pack.rs`：`RulePack::replaces_builtin`★
  - `crates/core/src/browser/driver.rs`：元素循环里的让位过滤（:1400-1406）
  - `crates/detect/src/engines.rs`：`ScanOptions.notes`★；`crates/detect/src/cli.rs`：输出 `note:` 行
  - `crates/browser/src/typography.rs`★、`crates/browser/src/cdp.rs`（`Page::platform_fonts`★）、`crates/browser/src/lib.rs`（`scan_loaded_page` 接入、`pub mod typography`）
  - `crates/cli/src/capture/mod.rs`、`report.rs`：`findingsNotes` 与终端 `NOTE` 行
  - `crates/rules/src/cjk.rs`★、`crates/rules/src/lib.rs`（5 行注册、钩子接线）
  - `crates/live/assets/detect-antipatterns-browser.js`、`bundle-inputs.json`：`cargo xtask bundle` 重新生成（C-4）
  - 测试：`crates/rules/tests/cjk.rs`★、`crates/rules/tests/text.rs`（ID 列表）、`crates/core/tests/rule_pack.rs`（让位用例）、`crates/cli/tests/xgent_rules.rs`（夹具表）、`tests/fixtures/xgent/cjk/*.html`★
  - `docs/ENGINE.md`「Rule packs」段
- skill 与文档
  - `skills/phoenix-ui/src/reference/typeset.md`、`craft-floor.md`、`new-work.md`、`src/subagents/phoenix-ui-finish-reviewer.md`、`skills/phoenix-ui/evals/scenarios.md`；渲染生成的 `skills/phoenix-ui/` 文件与 `GENERATED.json`
  - `docs/phoenix-ui.md`「XGENT 自有检测规则」「取证命令」两节
- 无需修改的邻接面：`browser-bundle/*.js`（快照脚本与页内探针）、`crates/wasm`、`crates/html`（静态引擎只调文本钩子）、`crates/hook`、`crates/comp-verbs`（comp 门禁不读文字内容，替换伪汉字不影响门禁）、`skills/phoenix-ui/src/reference/operate.md`、`live.md`、`critique.md`（:69 已说明叠层不含 xgent 规则）、README.md（未列举规则）。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类/适配器 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| 排版探针 `crates/browser/src/typography.rs`★ | `scan_loaded_page`，仅在带规则包时调用 | `collect(page, url, notes) -> CdpResult<TypographyFacts>`；调用前页面已有 `window.__impCap`；返回前探针容器必已移除；只测量，不出发现 | 无新接缝；事实经既有 `Dom` 接口读出 | 逐字分行、元素内文字 run、探针节点的插入与移除、Web 字体状态与范围解析、无法核实的原因（组装为纯函数） | 进程内 + 本机浏览器（CDP），用真实浏览器测 | `detect --json` 对夹具的发现与 `note:` 行；组装函数的单元测试 |
| 中文规则 `crates/rules/src/cjk.rs`★ | 规则包的 `check_element_dom`、`check_page_dom`、`replaces_builtin` | 入参只有 `&dyn Dom` 与元素；事实缺失即无发现；不 panic | 无 | 文本分类、候选范围、五条判定、单位词表、字体三类情况 | 进程内 | 经规则包公开钩子，用 `FakeDom` 断言发现 ID 与文案 |
| 驱动让位 `driver.rs` | `collect_browser_findings` | 每个元素：内置发现 → 丢掉 `replaces_builtin` 列出的 ID → 追加规则包发现；无规则包时路径不变 | `RulePack` trait（既有接缝，新增一个默认方法） | — | 进程内 | `crates/core/tests/rule_pack.rs` |

不新增适配器：探针只有一个调用方，直接用 `Page` 的 CDP 方法；`Dom` 的两个新方法沿用 `text_line_rects` 已有的“拿不到就返回 `None`”形态，页内 WASM 的实现不需要改。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| Chromium CDP `CSS.getPlatformFontsForNode`、`DOM.querySelectorAll` | 已核实足够（macOS Chrome 157）；Linux 见 A-1 | §1.1 实测；需先 `DOM.enable`、`CSS.enable` | 新增 `Page::platform_fonts` | 调用报错：该次扫描的字体事实全部缺失并写一条备注，其余规则照常 |
| 页面 API `document.fonts`、`Range.getClientRects`、`TreeWalker` | 已核实足够 | §1.1 实测 | 探针脚本使用 | 脚本抛错按既有页面脚本错误处理：本次扫描失败并报错，不吞掉 |
| 规则包钩子 `RulePack`（`crates/hook`、`crates/html`、`crates/detect` 也持有规则包引用） | 已核实足够 | 新方法有默认实现；其他引擎不调用它 | trait 加一个默认方法 | — |
| `capture` 报告 schema `phoenix-ui-capture-v1` | 已核实足够 | 每张图条目已有 `findings`、`findingsError`（crates/cli/src/capture/report.rs:118-146） | 新增可选数组 `findingsNotes`，为空时不写 | — |
| feature-demo、review-ui 对 `capture` 的可选接入（P1） | 已核实足够 | 只消费 `capture` 的报告与截图（docs/phoenix-ui.md「渠道」「取证命令」） | 零改动；新增的规则发现随报告的 `findings` 出现，怎么用由各 skill 现有规则决定 | — |

登记位置：5 条规则登记在 `crates/rules/src/lib.rs` 的 `ROWS`；没有新增配置项、环境变量、权限或对外命令。

## 5. 核心机制

### 5.1 文本分类

`han_dominant(text)`：遍历 `text` 的字符，跳过空白（`char::is_whitespace`）与标点（Unicode 类别 P，用 `regex` 的 `\p{P}`）；其余为可见字符，其中匹配 `\p{Han}` 的为汉字；`汉字数 × 2 > 可见字符数` 时为真，没有可见字符时为假。`text` 取 `dom.text_content(el)`，与内置规则统计长度用的是同一段文字。

### 5.2 候选范围

- **正文候选**：标签在 `QUALITY_TEXT_TAGS`（`p li td th dd blockquote figcaption`）内；`has_direct_text_longer_than(el, 10)`；`is_rendered_for_browser_rule`；`han_dominant`。
- **标题候选**：`h1`–`h6`；有非空白直接文本；`is_rendered_for_browser_rule`；`han_dominant`。
- 驱动循环已跳过 `claude-`、`cic-` 前缀与叠层节点，规则不再重复判断。

### 5.3 五条规则

注册行的 `category` 均为 `quality`；`scopes`：行长为 `["type", "layout"]`，其余为 `["type"]`；两条 warning 的 `severity` 为 `None`，三条 advisory 为 `Some("advisory")`。名称与描述用英文，写法照 crates/rules/src/lib.rs 现有各行。

| 规则 | 钩子 | 候选 | 判定 | 发现文案 |
| --- | --- | --- | --- | --- |
| `xgent/cjk-tight-leading` | 元素 | 正文 | `text_line_rects` 中宽高为正的行 ≥ 2；`resolve_length_px(lineHeight, fontSize)` 有值；比值保留两位小数后 < 1.50 | `line-height 1.40x on Chinese body text (need >=1.5)` |
| `xgent/cjk-line-length` | 元素 | 正文 | 行 ≥ 2；每行字数 = 行宽 ÷（字号 + 字距，字距取不到按 0）；四舍五入后 > 40 的行 ≥ 2 | `~52 Han-width chars on 3 of 4 rendered lines (aim for <=40)`，数字为最长行、超长行数、总行数 |
| `xgent/cjk-orphan` | 元素 | 正文与标题 | `text_line_texts` ≥ 2 行；末行去掉空白与标点后剩 1–2 个字符且全是汉字 | `last line holds only "了。"`（末行原文去首尾空白，最多 10 个字符） |
| `xgent/cjk-split-unit` | 元素 | 正文与标题 | `text_line_texts` ≥ 2 行；存在相邻两行：上一行去尾部空白后以数字串结尾（半角或全角数字，中间可含 `.`、`,`），下一行去首部空白后以单位开头；每个元素只报第一处 | `number "38" and its unit "秒" sit on different lines` |
| `xgent/cjk-font-fallback` | 页面 | 正文与标题中 `han_font_facts` 有值的元素 | 每个元素的各文字 run 按下文三种情况独立判断；只对命中结果按字体栈及情况去重，不合并未命中元素的事实 | 见下 |

**单位**（`cjk.rs` 内一处常量）：单字 `年月日号时分秒天周岁个位名人次项条件张页行列元角块万亿千百倍折成度点米克吨升斤层期届版代款台部辆套组份步级章节篇种类家只本册场轮双对瓶杯盒箱包首颗`；多字前缀 `小时`、`公里`、`公斤`、`毫秒`、`毫米`、`厘米`、`像素`、`美元`、`字节`、`英寸`、`星期`、`季度`、`平方`、`立方`、`工作日`；符号 `%`、`％`、`‰`、`℃`、`°`。

**字体回退的三种情况**。逐元素、逐文字 run 判定，不把内联后代主动选择不同字体算成缺字混用。记 `D` 为该 run 的声明列表（`fontFamily` 按逗号拆开、去引号、转小写，写法同 dom.rs:516 的 `family_name`），其中通用族指 `serif`、`sans-serif`、`monospace`、`cursive`、`fantasy`、`system-ui`、`-apple-system`、`blinkmacsystemfont`、`ui-sans-serif`、`ui-serif`、`ui-monospace`、`ui-rounded`、`math`、`emoji`、`fangsong`；`R` 为该 run 的非空实际汉字字体集合；“声明的具名字体”指 `R` 中名称等于 `D` 里某个非通用名的字体，或 `custom` 为真的字体（经 `@font-face` 加载，平台名可能与 CSS 别名不同）。

1. **Web 字体加载失败**：该 run 存在匹配的失败字体面，其 `earlier_declared_rendered` 为 false。失败面只有在所在字体族已不再渲染它覆盖的汉字时才进入事实（§5.5），所以这些字必然改由声明列表中更靠后的字体或兜底字体渲染。失败面须保留 family、weight、style、stretch、unicode-range 及声明位置；700 面失败不能污染使用正常 400 面的 run。声明中更早的通用族按浏览器解析后的实际字体判断，不能拿通用族名与平台字体名直接比较。归属取证方式见 §5.5；证据不足只备注，不报此情况。文案：`web font "Fixture Han" failed to load; Han text renders in PingFang SC`。
2. **混用**：同一 run 的 `R` 有两个以上字体，且其中至少一个是声明的具名字体。文案：`Han text renders in 2 fonts: Hiragino Mincho ProN (25 glyphs), Songti SC (12 glyphs)`。
3. **无字体可用**：`R` 中没有声明的具名字体，且 `D` 里没有通用族。文案：`no listed font renders Han and the list has no generic family; Han text renders in PingFang SC`。

其余都是正常结果，不报：只声明通用族；声明的具名字体渲染了全部汉字；前面的拉丁字体不含汉字或本机没装、汉字落到列表后面的具名字体或通用族。事实缺失（包括空 `R`）不进入以上判定。独立判定后，同栈同情况不止一个命中元素时文案末尾加 `(+N more elements with this font stack)`，N 只计其余命中元素。

### 5.4 内置规则让位

- 接口（crates/foundation/src/rule_pack.rs）：

  ```rust
  /// Built-in rule ids this pack takes over on `el`. The browser driver drops
  /// built-in findings with these ids on that element before it appends the
  /// pack's own. Default: none.
  fn replaces_builtin(&self, dom: &dyn Dom, el: ElId) -> &'static [&'static str] {
      let _ = (dom, el);
      &[]
  }
  ```

- 驱动（driver.rs:1400-1404 的规则包分支）：先取 `pack.replaces_builtin(dom, el)`，非空时 `findings.retain(|f| !ids.contains(&f.type_.as_str()))`，再 `extend(pack.check_element_dom(dom, el))`。此时 `findings` 里只有该元素的内置发现。
- XGENT 规则包的实现：元素是正文候选时返回 `&["line-length", "tight-leading"]`，否则为空。与行数无关：单行的中文正文也不再报内置 `tight-leading`。

### 5.5 排版探针

`crates/browser/src/typography.rs`★。探针脚本是文件内的 Rust 字符串常量（C-5），不新增 `.js` 文件。

```rust
pub struct TypographyFacts {
    pub line_texts: HashMap<ElId, Vec<String>>,
    pub han_fonts: HashMap<ElId, HanFontFacts>,
}
pub fn collect(page: &mut Page<'_>, url: &str, notes: &mut Vec<String>) -> CdpResult<TypographyFacts>
```

执行顺序：

1. **页内测量**（一次 `page.evaluate_value`）。元素集合与 §5.2 的标签、直接文本长度/非空、可渲染及汉字占比条件一致；在其子树存在可见汉字时测量，不能要求直接文本含汉字：
   - 行文字：`TreeWalker` 按文档顺序走直接文本与行内后代的文本节点；嵌套在元素里的块级后代（如列表项里的子列表）是另一段文字，不计入本元素的行，否则子列表的短项会被当成上级列表项的末行孤字。逐字符取 `Range.getClientRects()` 中正宽高的矩形并归属渲染行；任何字符（包括非空白字符）没有正尺寸矩形时跳过，不拼入前后行。行归属沿用 `text_line_rects` 的行带重叠口径，内联 `<strong>/<a>` 的片段归入同一渲染行。结果为 `lines[id] = [每行可见文字]`，规则取首尾空白时沿用原去空格规则；`<br>` 只造成真实断行，不豁免孤字和拆单元。
   - 字体文字 run：每个候选元素独立记录直接及行内后代的可见汉字（范围同行文字），按文本节点的计算样式 `fontFamily/fontWeight/fontStyle/fontStretch` 与最近祖先 `lang` 分 run。相同属性可在该元素内部合并字符；不同元素不得合并取数或相互回填。主动设置不同字体的内联 run 分别判定，不能把它们的字体并集当成缺字混用。
   - Web 字体状态：从 `document.fonts` 保留 family、weight、style、stretch、unicodeRange、status；按 CSS 字体面选择语义（含描述符范围）匹配当前 run，只有其范围覆盖可见汉字的面才参与。`unicodeRange` 解析 `U+X`、`U+X-Y` 与 `?` 通配；匹配面仍 `loading` 则该 run 未核实。`failed` 保存匹配的 error 面及其声明位置，不仅保存族名。
   - 探针容器：在 `document.documentElement` 末尾追加 `div[data-phoenix-ui-font-probe][aria-hidden="true"]`，样式 `position:fixed; left:-99999px; top:0; visibility:hidden; pointer-events:none; white-space:nowrap`；每个元素的每个 run 一个 span，复制上述字体属性与 lang，文本只含该 run 的可见汉字。批量读取仍保留 `(元素 id, run)` 对应关系。
   - 失败面归属：仅对匹配的失败面，用该面覆盖的汉字、同 run 的字体属性追加至多三个探针。①只声明失败面所在的字体族：结果里仍有 Web 字体（`isCustomFont`）说明该族另一个面仍在渲染这些字（如 700 失败而 run 用的 400 正常），这个失败面不记入事实。②失败面之前的声明前缀（失败面排在首位时不需要）。③一个不存在的字体族，作为“什么都没声明”时的兜底结果。前缀结果里有 Web 字体、有名称与前缀中具名字体相同的字体，或与兜底结果不同，即更早声明已承接这些字，不归因于失败面；前缀结果与兜底结果相同且前缀里没有通用族，即更早声明没有承接，失败面是来源；前缀结果与兜底结果相同而前缀里有通用族时，通用族可能正好解析到兜底字体，两者无法区分，记为归属未核实，不推定失败导致回退。任一探针读不到字体同样记为未核实。
2. **读字体**：`page.platform_fonts("[data-phoenix-ui-font-probe] > span")` 按文档顺序读取字体，并以第 1 步保留的身份映射回元素/run。`CSS.getPlatformFontsForNode` 不会先排版：第 1 步插入容器后立即读取其尺寸以强制同步排版；读到的结果里有空列表时（页面在两步之间又使排版失效），再强制排版并重读一次。
3. **清理**：再次 `evaluate_value` 移除容器；第 2 步出错也先清理再处理错误。
4. **组装**：
   - 第 2 步调用报错：所有字体事实缺失，备注 `font sources not verified at <URL> (whole page): <错误信息>`。
   - run 标为 loading、实际字体 `R` 为空（无论 failed 是否非空）或失败归属仍未核实：该 run 返回 `None`，备注 `font source not verified at <URL> element <id> "<文本样本>": <原因> (font-family: <字体栈>)`。样本取可见文字前 40 个字符；URL + 当前快照元素 id 稳定对应本次扫描段落；末尾的字体栈辅助人工核对。
   - 只有该元素所有需测 run 均已核实，才生成其 `HanFontFacts { runs }`；否则该元素无字体事实，已核实 run 不掩盖未核实部分。其他元素仍保留独立事实。不生成空 R 的可触发事实。

   组装是纯函数，输入包含 URL、元素定位、run 测量、实际字体及错误，输出事实与可定位备注。字体面归属证据与规则条件使用同一 run 和同一失败面。

`Page::platform_fonts`（cdp.rs，写法照 :1889-1895 的 `create_capture_stylesheet`）：`DOM.enable`、`CSS.enable`、`DOM.getDocument`（`depth: 0`）、`DOM.querySelectorAll`，对每个节点调 `CSS.getPlatformFontsForNode`，取 `familyName`、`isCustomFont`、`glyphCount`。

探针不发起网络请求：探针文字全部来自页面已经显示的汉字，所需字体页面已请求过。

### 5.6 事实接口

crates/foundation/src/browser/dom.rs：

```rust
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct RenderedFont { pub family: String, pub custom: bool, pub glyphs: u32 }

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct FailedWebFont {
    pub family: String,
    pub weight: String,
    pub style: String,
    pub stretch: String,
    pub unicode_range: String,
    pub declaration_index: usize,
    /// Whether earlier declared families rendered the Han covered by this face.
    pub earlier_declared_rendered: bool,
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct HanFontRun {
    pub declared_families: Vec<String>,
    pub weight: String,
    pub style: String,
    pub stretch: String,
    pub lang: String,
    pub rendered: Vec<RenderedFont>,
    pub failed_web_fonts: Vec<FailedWebFont>,
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct HanFontFacts { pub runs: Vec<HanFontRun> }

// trait Dom
/// The text on each rendered line of the element (descendants included), top to
/// bottom. `None` when this DOM did not record it.
fn text_line_texts(&self, _el: ElId) -> Option<Vec<String>> { None }
/// Which fonts rendered the element's Han characters. `None` when unverified.
fn han_font_facts(&self, _el: ElId) -> Option<HanFontFacts> { None }
```

- 同文件另有 `GENERIC_FONT_FAMILIES` 常量（§5.3 所列通用族），探针判断前缀与规则判断声明列表共用这一份。
- `SnapshotDom::set_typography(&self, line_texts, han_fonts)`：与 `add_facts`（snapshot.rs:584）相同的内部可变性写法；两个 trait 方法按元素 id 查表。
- `FakeDom::set_text_line_texts(id, &[&str])`、`set_han_fonts(id, HanFontFacts)`：照 `set_text_lines`（fake_dom.rs:174）。
- 页内 WASM 的 `Dom` 实现（crates/wasm）不改，沿用默认的 `None`。

### 5.7 备注通道

- `ScanOptions`（crates/detect/src/engines.rs）新增 `pub notes: Option<Rc<RefCell<Vec<String>>>>`，默认 `None`。
- `scan_loaded_page`（crates/browser/src/lib.rs）在第一次 `capture_snapshot`（:372）之后、`resolve_needs` 之前：仅当 `options.rule_pack.is_some()` 时调用 `typography::collect(page, url, notes)`，把事实交给 `dom.set_typography`，以当前页面 URL 和元素定位生成的备注追加到 `options.notes`（为 `None` 时丢弃）。
- `detect`：`detect_cli` 为本次运行建一个备注容器放进基础 `ScanOptions`；发现输出之后，按完整 URL、元素定位与原因去重（不能跨 URL/段落吞并），逐行写到 stderr，格式 `note: <文本>`。JSON 模式下 stdout 仍只有发现数组；退出码不变。
- `capture`：`scan`（crates/cli/src/capture/mod.rs:1441-1457）每次调用建一个容器，返回发现与备注；该图条目在备注非空时写与 detect 相同可定位文本的 `findingsNotes` 数组，终端在该图的行后输出 `NOTE <状态> <视口> <文件>: <文本>`（写法照 report.rs:144-149 的 `SCAN-ERROR`）。

### 5.8 开关、级别与输出

全部沿用既有机制，不新增代码路径：`detector.ignoreRules` 与 `hooks ignore-rule` 经 `filter_detection_findings` 生效；`--no-advisory` 隐藏三条 advisory；两条 warning 计入失败数，使 `detect` 退出 2；页面规则没有 inline 豁免（docs/phoenix-ui.md「关闭方式」）。

### 5.9 不运行中文规则的地方

- 设计 hook：只经文本引擎与静态 HTML 引擎运行，二者只调规则包的 `check_text`。
- live 叠层与浏览器扩展：页内 WASM 不带规则包；`replaces_builtin` 不会被调用，内置 `line-length`、`tight-leading` 照旧。
- `detect` 的源码文件目标与 `.html` 文件目标：不经浏览器，没有行与字体事实。

## 6. Skill 与文档增量

skill 源文件用英文写，下列为要表达的内容与落点；数值以本节为准。改完运行渲染（C-8）。

### 6.1 `skills/phoenix-ui/src/reference/typeset.md`

- 「Two isolated assessments」的 Reading 一问（:21）：拉丁正文按 45–75 个字符评估；以汉字为主的正文按约 25–40 个字评估，不拿 45–75 字符当中文的依据。
- 「Mechanical scan」（:24-28）：目标有可访问的 URL 时，在同一轮里按 craft-floor 的宽度口径加跑 `{{scripts_path}}/phoenix-ui detect --json --scope type --viewport <W>x<H> <url>`；说明 `xgent/cjk-*` 五条只出现在页面扫描里，stderr 的 `note:` 行表示所定位段落字体来源未核实；按现有轮次重取或人工核对，仍未解除则在最终交付列入未验证项，不以无发现声称已核实。
- 「Apply」（:47 一条补一句中文例外），并新增一节 `Chinese copy (zh-CN / zh-TW)`，开头写明“以下是起点；项目 DESIGN.md 另有规定的从其规定，并在交付里写出所依据的条目”：
  - 字号起点：阅读型页面正文 16–18px；产品界面正文 14–16px；长时间使用的高密度工具 13–14px；不靠 9–12px 的说明文字营造精致。
  - 行高：正文 1.6–1.75；多行中文标题从 1.2–1.35 起，逐视口核对字形不碰撞；标题高度不固定为单行，不用负外边距挤压后续内容。
  - 行长：中文正文约 25–40 字。
  - 断行：数字与单位、不可拆的词用不换行包住；首屏说明、图注这类短段落按语义手动断句；行末只剩 1–2 个字或词被拆到两行算缺陷。
  - 中英混排：核对基线与视觉字重；窄幅展示字只用于展示标题与极短标签。
  - 字体回退：核对汉字的实际渲染字体；`xgent/cjk-font-fallback` 报出的情况要处理，正常的通用族解析不算问题。
- 「Verify」加两条：每个检查视口下多行中文正文行高在 1.6–1.75，否则交付写出所依据的 DESIGN.md 条目；页面扫描没有未解释的 `xgent/cjk-*` 发现。

### 6.2 `skills/phoenix-ui/src/reference/craft-floor.md`

Type 一条（:12）末尾加一句：zh-CN / zh-TW 文案的行长是 25–40 个汉字、正文行高 1.6–1.75、数字不与单位分行、末行不只剩一两个字，细则见 typeset.md。文件开头“pinned brief 与已定视觉世界优先”的总则不动，DESIGN.md 因此仍然优先。

### 6.3 伪汉字条款与完工备注

- `skills/phoenix-ui/src/reference/new-work.md` 首屏一步（:123）“the comp's words copied verbatim (…)”之后加例外：设计稿里的伪汉字、乱码、与界面语言不符的繁简字是稿件缺陷，不照抄；替换为产品事实或用户确认的真实文案，逐条记下“原位置 → 替换后的文字 → 依据”；能辨认的真实文案仍逐字沿用。
- 同文件 capture 检查段消费 `findingsNotes`：逐段识别字体未核实状态，按既有轮次重取或人工核对并留依据；无字体发现不能作为已核实证据。备注及其解除依据随原取证包交给完工评审，仍未解除的 URL/段落/原因列入最终交付未验证项，不另设处置词、阻塞门或 detector 轮次。
- 同文件派出完工评审的输入清单（“Then spawn the shipped finish reviewer…”一段）加一项：这份替换清单；交付报告逐条列出替换。
- `skills/phoenix-ui/src/subagents/phoenix-ui-finish-reviewer.md` Fidelity 检查（:31）加一句：输入里逐条列明并写了依据的伪汉字、乱码或繁简不符替换，归为 acceptable adaptation；未列明的文案改动仍按原规则判。输入合同同时接收 capture 的 `findingsNotes` 及核对依据，在现有 evidence 检查中核对仍未解除的备注并交回主代理披露；备注自身不新增阻塞门。Disposition 段、输出合同与四个处置词不动。

### 6.4 `docs/phoenix-ui.md`

- 「XGENT 自有检测规则」表追加 5 行：检测方式均为“页面（`detect <url>`，浏览器渲染）”；`xgent/cjk-tight-leading`、`xgent/cjk-line-length` 为 warning，关闭方式 `detector.ignoreRules`、`hooks ignore-rule`；其余三条为 advisory，另可 `--no-advisory`。
- 表后补一段：这 5 条只作用于以汉字为主的正文与标题（判定见 §5.1 的定义，用一句话写出）；在这些正文上内置 `line-length`、`tight-leading` 由对应中文规则接替，关闭中文规则不会让内置规则回来；hook 不产生它们，live 叠层与扩展不运行它们，在那两处中文正文仍只有内置规则；字体来源无法核实时不报 `xgent/cjk-font-fallback`，`detect` 在 stderr 输出 `note:` 行。
- 「取证命令」报告字段说明补 `findingsNotes`：它表示该图相应 URL/段落的字体来源尚未核实，不是发现；无字体回退发现不等于已核实。按现有重取轮次等字体加载稳定后重取，或人工核对该段实际字体并记录依据；仍未解除则在最终交付的未验证项披露 URL、段落与原因，不另加阻塞门或 detector 轮次。

### 6.5 `tools/phoenix-ui/docs/ENGINE.md`

「Rule packs」段补两句：`replaces_builtin` 的作用与调用时机；URL 引擎在带规则包时运行排版探针，经 `Dom::text_line_texts`、`Dom::han_font_facts` 提供事实，页内 WASM 不提供。

### 6.6 `skills/phoenix-ui/evals/scenarios.md`

在现有 18 条之后追加 3 条待执行用例（写法照第 13–18 条，含失败信号）：给中文页面做 typeset（按 25–40 字与 1.6–1.75 调整并跑页面扫描；失败信号：按 45–75 字符评估中文正文、只跑文件扫描）；zh-CN 新页面完工（交付截图中多行正文行高在口径内或写出 DESIGN.md 条目；失败信号：出现末行孤字或数字单位分行而未处理）；comp-led 设计稿含伪汉字（替换并逐条列出，其余逐字沿用；失败信号：照抄乱码，或顺手润色可辨认的文案）。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N3 | PRD WORK-N3/AC-1–AC-3 | 没用到中文规则的流程行为不变；四个处置词不变；中文规则默认启用、可单独关闭，hook、live、扩展不运行 | — | 规则只挂在规则包钩子上；让位钩子默认空；不改共享快照脚本与评审处置段 | V-2（无规则包输出不变、拉丁夹具对照）、V-6（oracle golden 与全量回归）、V-8 |
| WORK-N4 | PRD WORK-N4/AC-2 | 改 skill 前清单到位，diff 逐项对得上 | 出现清单外的行为差异时先更新 §0.6 再继续 | §0.6 | V-4、V-8 |

安全与隐私：没有新入口、新凭据或新的外发请求。探针只读页面已显示的文字，不读源码与存储；探针容器不进入截图（不可见且在截图前后已移除）。既有“页面扫描只访问目标页面及其自身资源”的边界不变。

可观测性：探针无法核实的情况全部经备注通道写出；没有静默跳过。

## 8. 失败模式、发布与回滚

本期不引入需要专门防御的运行期失败模式：规则与探针都是只读测量，失败的后果是少报发现或本次扫描报错，不涉及数据、权限或不可逆操作。两处行为写死即可：

- CDP 读字体失败：字体事实缺失并写备注，其余规则照常（§5.5 第 4 步）。
- 探针页内脚本抛错：按既有页面脚本错误处理，本次扫描失败并给出错误，`capture` 记为该图的 `findingsError`。

发布顺序：引擎与 skill 生成物在同一提交序列里；引擎源码已变，发布走 CI 构建，不能用 `--reuse-engines`（docs/phoenix-ui.md「维护者」节的复用条件）。回滚即回退对应提交并重新生成页内包与 skill；没有数据或配置迁移。

对存量项目的可见变化：升级后中文页面的 `detect <url>` 可能新增两类 warning 并使退出码变为 2；项目可用 `hooks ignore-rule` 逐条关闭。这是 PRD D5 接受的升级成本，写进 docs/phoenix-ui.md（§6.4）。

## 9. 验证

### 9.1 行为与接口验证

共同环境：仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；Cargo 命令在 `tools/phoenix-ui` 下运行；本机 macOS 有 PingFang SC、Songti SC、Hiragino Mincho ProN 与可发现的 Chrome（可用 `PHOENIX_UI_BROWSER` 指定）；浏览器测试不放进 `sandbox-exec`。

批内最小反馈：`mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` 后直接运行 `target/debug/phoenix-ui detect --json --viewport 1280x800 file://<夹具绝对路径>` 复现单个问题；纯逻辑用 `mise exec --no-deps -- cargo test -p phoenix-ui-rules --test cjk <用例名> --locked --offline`。两者都只在有具体失败或疑问时运行。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 场景 E2E | **一位开发者用 `detect` 检查一篇中文文章页，修好后复查，再确认英文页不受影响。** 角色：只读过 `phoenix-ui detect --help` 与 docs/phoenix-ui.md 规则表，不读规则源码。输入：`tests/fixtures/xgent/cjk/article.html`（zh-CN 文章：正文 `sans-serif`、行高 1.4、容器 52em；两行标题末行只剩一个字；一段“耗时 38 秒”在数字后换行；一个引用块用加载失败的 Web 字体）、`article-fixed.html`（行高 1.6、容器 38em、数字单位包不换行、标题手动断句、字体栈正常）、`latin.html`（英文长行、行高 1.2）、`mixed.html`（假名为主的日文段与英文夹少量中文的段，行高 1.2）。结果：①文章页报两条 warning（写出 `1.40x` 与约 52 字）、孤字、拆行、字体回退各一处，这些元素上没有内置 `line-length`、`tight-leading`，退出 2（WORK-F5/AC-1–AC-4、AC-7）；②`--no-advisory` 只剩两条 warning（AC-5）；③在临时项目里 `phoenix-ui hooks ignore-rule xgent/cjk-line-length` 后该规则消失、其余照旧，移除后恢复（AC-5、WORK-N3/AC-3）；④修好的页面没有任何 `xgent/cjk-*`（AC-1 后半、AC-7）；⑤`latin.html`、`mixed.html` 的发现与 §9.2 的起始对照逐条相同且没有 `xgent/cjk-*`（AC-4、AC-6）；⑥对文章页运行 `capture`，报告该图的 `findings` 含同样的中文规则发现，没有 `findingsNotes`；⑦`slow-font.html` 在 load 事件里才插入一段用 Web 字体的中文，本地脚本把该字体的响应延迟 10 秒：两个 URL 各有自己的延迟字体文本段，`detect` 同时给这两个 URL（共享浏览器只等到 load）时，stderr 的两条 `note:` 各带正确 URL、元素 id 与样本（同栈也不互相去重）、各段不报字体回退、退出码不因备注改变；`capture` 的一个点击后状态（`slow-font.html?case=click`，点击后才插入用该字体的段落）同理在该图条目写出含相同 URL/段落定位的 `findingsNotes`（AC-7、§5.7）。Agent 逐项阅读输出 | `target/debug/phoenix-ui detect --json --viewport 1280x800 file://…/article.html`，同一页再跑 `--viewport 375x812`、`--no-advisory`；`target/debug/phoenix-ui capture file://…/article.html --viewport desktop --out "$tmp/cap"`；第⑦项用一个几行的本地 HTTP 脚本提供夹具目录并延迟字体响应，命令为 `target/debug/phoenix-ui detect --json http://127.0.0.1:<端口>/slow-font.html "http://127.0.0.1:<端口>/slow-font.html?case=2"` 与带 `--states` 的 `capture` | M1–M2 交付齐备后（批 1） |
| V-2 | 局部 | 准入：文本分类、单位识别、字体三类情况是分支多的纯逻辑；拉丁不变与无规则包不变是明示的逐项等价要求；四平台结果一致需要不依赖字体的断言。内容：①`crates/rules/tests/cjk.rs`★（`FakeDom`）：`han_dominant` 在恰好一半、标点与空白不计、假名/谚文/拉丁为主时的结果；五条规则各自的正例、门槛两侧（1.49 与 1.50，40 与 41）、只有一行超长、末行 1/2/3 个字、末行含拉丁、`<br>` 造成孤字/数字单位拆行仍报、数字单位的几种写法、字体三种情况与四类正常结果、事实为 `None` 时无发现；`replaces_builtin` 只对正文候选返回两条 ID；零尺寸与无父元素不 panic（C-6）；②`crates/core/tests/rule_pack.rs`：带让位的规则包使对应内置发现消失，无规则包时既有断言不变；③`crates/rules/tests/text.rs` 的注册 ID 列表含新 5 条；④`crates/cli/tests/xgent_rules.rs`：夹具表追加不依赖字体的行，四平台同表：`tight.html`（紧行）、`long.html`（长行）、`clean.html`（行高合格、只声明 `sans-serif`、`display:none` 文字、嵌套列表，无发现）、`breaks.html`（`min-content` 孤字与拆行、手工 `<br>` 孤字与拆行）、`webfont.html`（Web 字体失败、汉字只在 `<strong>` 里、通用族排在失败字体之前）、`mixed.html`（假名段）、`latin.html`（拉丁段）；同类情形合在一页以控制浏览器测试耗时。依赖系统字体的情形（同栈三段只有一段混用、无通用族、拉丁字体在前而具名字体正常、同族 700 失败而 400 正常）合为 `fonts-macos.html`，只在 macOS 断言；另断言中文夹具上没有内置 `line-length`、`tight-leading`，`latin.html` 在 1280x800 下两条内置规则都在；⑤`capture` 与 `detect` 的初始发现一致的既有用例仍通过；⑥`typography.rs` 的组装函数：读字体报错、字体仍在加载、返回空列表且同时存在失败 Web 字体声明三种情况都不产生字体事实并各有一条备注，正常情况产生事实且无备注；⑦在既有字体/行文字组补：同栈两段分别完整用 A/B 均不报，只有一段混用时仅归属该段；父元素直接文本无汉字、汉字仅在 `<strong>/<a>` 后代仍有行文字及字体事实；隐藏前缀、后缀和数字/单位片段不进任何行也不引发发现；同族 700 失败/400 正常及更早通用族已渲染均不报。真实 DOM 范围与行文字用本表既有浏览器夹具入口核对 | `mise exec --no-deps -- cargo test -p phoenix-ui-rules --locked --offline`；`mise exec --no-deps -- cargo test -p impeccable-core --test rule_pack --locked --offline`；`mise exec --no-deps -- cargo test -p phoenix-ui --test xgent_rules --locked --offline -- --test-threads=1`；`mise exec --no-deps -- cargo test -p phoenix-ui --test capture initial_findings_match_detect --locked --offline`；`mise exec --no-deps -- cargo test -p impeccable-browser --lib typography --locked --offline` | M1–M2 交付齐备后（批 1） |
| V-3 | 局部（静态） | 文档回读 §5.7、§6 的 detect/capture 备注含义、定位、消费与披露一致（skill 生成物在 V-4 核对）；C-4：页内包与输入一致；改动范围内无新增 lint 诊断 | `tools/phoenix-ui` 下 `mise exec --no-deps -- cargo xtask bundle --check`；仓库根 `node scripts/check-phoenix-lints.mjs` | M1–M2 交付齐备后（批 1） |
| V-4 | 局部（静态） | §6 的文本落位与 §0.6 对得上（WORK-N4/AC-2、C-7、C-8）：渲染为 `fresh`；生成的 typeset、craft-floor、new-work 与完工评审子代理里有 §6 的内容；`git diff --stat` 的 skill 源改动只有 §6 列出的文件；完工评审子代理的 Disposition 段与输出合同无 diff；docs/phoenix-ui.md 规则表有新 5 行与说明段；phoenix-ui skill 的元数据测试通过，其他 skill 的既有失败只报告 | `tools/phoenix-ui` 下 `mise exec --no-deps -- bun scripts/render.mjs`，再 `mise exec --no-deps -- bun scripts/render.mjs --check`；仓库根 `grep -n 'cjk' skills/phoenix-ui/reference/typeset.md docs/phoenix-ui.md`、`git diff --stat -- skills/phoenix-ui/src docs tools/phoenix-ui/docs`、`git diff -- skills/phoenix-ui/src/subagents/phoenix-ui-finish-reviewer.md`；`python3 -m unittest discover -s test/skills -p 'test_skill_metadata.py'`，只按 phoenix-ui 的子测试判定 | M3 交付齐备后（批 2） |
| V-5 | 场景 E2E（有界演练） | **设计负责人让 agent 给一篇现成的中文文章页做排版整理。** 角色：Claude Code 新会话，只有项目文件与已安装的 Phoenix skill。项目：仓库树外的临时目录，放 V-1 的 `article.html` 作为页面并用本地 HTTP 脚本提供；`.claude/` 用渲染产物 `tools/phoenix-ui/dist/providers/claude-code/.claude/` 的副本，其中 `skills/phoenix-ui/scripts/phoenix-ui` 换成调用本地 debug 二进制的垫片（仅演练）。请求：“用 Phoenix typeset 整理这个页面的排版”。结果（看会话记录与页面 diff）：评估中文正文时用 25–40 字而不是 45–75 字符；跑了带 `--viewport` 的页面扫描；正文行高改到 1.6–1.75、行长收进 40 字、数字单位不再分行、标题孤字消失；改后 `detect` 没有 `xgent/cjk-*`；交付没有无依据的偏离说明（WORK-F6/AC-1、AC-2） | `claude -p "<请求>" --output-format stream-json --verbose`，权限参数按 `claude --help` 实际选择并只授予本场景所需；保存输出后按工具调用、最终答复与 `detect` 复查判定 | M3 交付齐备且 V-1、V-2 通过后（批 2） |
| V-6 | 阶段收口 | 构建同步与 Phoenix UI 全量回归：开发构建与冒烟通过；`test:phoenix` 全绿（页内包检查、工作区测试、浏览器测试、lint、oracle golden 逐字节不变）；render `--check` 为 `fresh`（WORK-N3/AC-1–AC-3 在本期终验时同时成立）。不以整仓 `npm test` 为门，其 Diagram、statusline 等检查与本期无关 | `tools/phoenix-ui` 下：`mise exec --no-deps -- node ../../scripts/prepare-phoenix-ui.mjs --prepare` 并按回执导出环境变量；`mise exec --no-deps -- node ../../scripts/build-phoenix-ui.mjs --development --output-dir ../../local/phoenix-ui/development-release`（先清空输出目录）；`mise exec --no-deps -- node ../../scripts/smoke-phoenix-ui.mjs --release-dir ../../local/phoenix-ui/development-release --target darwin-arm64 --stage-runtime`（`--target` 填本机平台）；`mise exec --no-deps -- npm --prefix ../.. run test:phoenix`；`mise exec --no-deps -- bun scripts/render.mjs --check`。耗时未知，只在收口运行一次 | M4，M1–M3 完成后 |
| V-7 | 真实使用验收 | 干净的夹具项目（仓库树外）、只凭公开入口与文档、用 V-6 的开发构建安装，逐项保存输出并亲自查看产物。**A1 新建中文页面**（WORK-F6/AC-1、AC-2、AC-4；PRD UAT-7 的设计侧）：Claude Code 新会话，请求“用 Phoenix 给我们的产品做一个中文的版本更新日志页面”；判据：各检查视口的截图里多行正文行高在 1.6–1.75 或交付写出 DESIGN.md 条目，行长在 40 字内，没有末行孤字与数字单位分行，中英混排基线对齐、字重接近，窄幅展示字只在标题与短标签；完工取证报告里没有未处理的 `xgent/cjk-*`；复用 V-1 的 slow-font 状态，确认仍未解除的 `findingsNotes` 随取证包进入完工评审并在最终交付未验证项列明 URL/段落/原因，没有额外 detector 轮次。**A2 直接使用 detect**（WORK-F5/AC-1–AC-7、WORK-N3/AC-3；PRD UAT-7、UAT-17）：以开发者身份经安装后的启动器，对一个自己写的中文页面（行高 1.4、每行约 52 字、段末孤字、只声明 `sans-serif`）与一个英文页面运行 `detect`，改到 1.6 与 40 字内后复查，再试 `--no-advisory` 与 `hooks ignore-rule`；判据：发现集合与 UAT-7、UAT-17 的预期一致，英文页没有 `xgent/cjk-*`。**A3 设计稿含伪汉字**（WORK-F6/AC-3；PRD UAT-8）：夹具项目带一张已批准的设计稿（小字区域是伪汉字，标题与按钮是真实中文），Claude Code 新会话从 `build-phase start --comp` 起做首屏；判据：伪汉字被换成产品事实或向用户确认的文案并在答复里逐条列出，标题与按钮文字逐字沿用；首屏门禁是否通过不计入本项。发现的用户可见问题修复后只复跑受影响的场景；改了引擎或 skill 源须从 V-6 的构建重做再复验受影响项 | 安装：在夹具项目里用仓库的安装入口加 `--release-dir=<仓库>/local/phoenix-ui/development-release`（docs/phoenix-ui.md「安装」选项表）；会话用 `claude -p … --output-format stream-json --verbose`；A2 直接用 `npm exec` 形式或安装后的 `skills/phoenix-ui/scripts/phoenix-ui`。A3 耗时未知 | M4，V-6 通过后 |
| V-8 | 阶段收口 | 整合清单与复述核对（WORK-N4/AC-2、C-1、C-7、WORK-N3/AC-2）：§0.6 每个采用或调整项都有落点、拒绝项无改动；独立 skill 与 `browser-bundle/`、`crates/wasm`、`crates/comp-verbs` 无 diff；新规则在 docs/phoenix-ui.md、typeset 指引、ENGINE.md 三处都有完整描述 | `git diff 45e3844 -- skills tools/phoenix-ui docs README.md` 逐项对照 §0.6 与 §1.2；`git diff --stat 45e3844 -- skills/review-ui skills/apply-ui-review skills/feature-demo tools/phoenix-ui/browser-bundle tools/phoenix-ui/crates/wasm tools/phoenix-ui/crates/comp-verbs` 为空；`grep -rn 'xgent/cjk' docs/phoenix-ui.md skills/phoenix-ui/reference tools/phoenix-ui/docs` | M4，V-7 之后 |
| V-9 | 阶段收口 | 四平台候选（A-1）：同一候选提交上 Phoenix CI 的 UI 四个平台任务与 Node 18 冒烟全部成功，其中 `xgent_rules` 的夹具表在 Linux 两个平台通过；既有中文 clean/通用族控件还须保留 stderr 并断言没有字体未核实备注（不能用丢弃 stderr 的 helper 证明）。分别记录 Linux 两平台此断言的结果；空 R 将产生备注而失败，转 A-1 的不可核实路线，不能以无发现冒充正常取数。Diagram 任务的失败与本期无关时只报告 | 经用户授权推送 main 后：先设 `candidate_sha=<M4③之后候选提交的完整SHA>`，运行 `gh run list --workflow phoenix.yml --branch main -L 100 --json databaseId,headSha,status,conclusion --jq "map(select(.headSha == \"$candidate_sha\"))"`，逐条精确核对 `headSha == candidate_sha` 后才对匹配的 `databaseId` 执行 `gh run watch <run-id>`；未找到则记证据缺失并等待/查明原因，错误 SHA 的 run 不回填通过。记录候选完整 SHA、匹配的 headSha、run ID 与四平台结果 | M4，V-6–V-8 通过并完成候选提交后；未获授权时记入当前阻塞，不记完成 |

### 9.2 集成与回归

- 批 1 动引擎前，先写好 `latin.html`、`mixed.html` 两个非中文夹具，用起始代码的 debug 二进制各跑一次 `detect --json --viewport 1280x800`，输出存进 M2 记录，作为 V-1 第⑤项的对照。
- `test:phoenix` 的第一步就是 `cargo xtask bundle --check`（scripts/test-phoenix-ui.mjs:27），随后是 release 构建、安装测试、工作区测试、浏览器测试、lint 与 oracle；它只在 V-6 运行一次。
- 页面扫描的规则发现不在 oracle golden 里（golden 只含源码文件扫描与离线命令），本期不需要重录；V-6 里 golden 应逐字节不变，有差异即是回归。

### 9.3 真实环境

V-1 在本机 Chrome 上由 Agent 执行并阅读输出；V-5、V-7 需要已登录的 Claude Code；V-9 需要用户授权推送。缺任一环境时对应检查记“环境缺失/未运行”，收口不宣称相应 AC 通过。

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 排版事实、让位钩子与备注通道 | 无 | 批 1 前半。§5.4–§5.7：`dom.rs` 的字体/run 事实类型与两个 trait 方法（同构：`text_line_rects`，dom.rs:190）；`snapshot.rs` 的 `set_typography`（同构：`add_facts`，:584）；`fake_dom.rs` 的两个设置方法（同构：`set_text_lines`，:174）；`rule_pack.rs` 的 `replaces_builtin`；`driver.rs`:1400-1406 的过滤；`engines.rs` 的 `ScanOptions.notes`；`typography.rs`★ 与 `cdp.rs` 的 `platform_fonts`（同构：`create_capture_stylesheet`，cdp.rs:1889-1895；页内脚本以字符串常量内嵌的写法同 `screenshot_contrast.rs`）；`lib.rs` 的 `scan_loaded_page` 接入；`detect/src/cli.rs` 的 `note:` 输出；`capture/mod.rs`、`report.rs` 的 `findingsNotes`（同构：`findingsError`）；`cargo xtask bundle` 重新生成页内包；ENGINE.md（§6.5）。先按 §9.2 录非中文夹具的对照。`cdp.rs`、`lib.rs`、`driver.rs` 由主 agent 单人修改 | 与 M2 同批，V-1、V-2、V-3；回写「实施进度」 |
| M2 | 五条中文规则与夹具 | M1 代码 | 批 1 后半，连续实施。`crates/rules/src/cjk.rs`★（§5.1–§5.3；同构：dom.rs 中 `wrapped_control_label` 的元素规则写法与 `font_family_count` 的页面规则写法）；`lib.rs` 的 5 行注册、`ROWS`、`replaces_builtin` 与两个钩子的接线；`tests/fixtures/xgent/cjk/*.html`★（构造见 §1.1 的 `min-content` 与 52em 结论）；`crates/rules/tests/cjk.rs`★（同构：tests/dom.rs）；`crates/rules/tests/text.rs`、`crates/core/tests/rule_pack.rs`、`crates/cli/tests/xgent_rules.rs` 的增量 | 批末 V-1、V-2、V-3 通过；回写「实施进度」 |
| M3 | skill 中文口径与文档复述 | §0.6 I-1–I-4 已核对；规则名与行为以 §5 为准，开发不依赖批 1 完成；V-5 需 V-1、V-2 通过 | 批 2。§6.1–§6.4、§6.6：`typeset.md`、`craft-floor.md`、`new-work.md`、`phoenix-ui-finish-reviewer.md`、`evals/scenarios.md`、`docs/phoenix-ui.md`；运行渲染并提交生成物。同构：`craft-floor.md` 的 Widths 一条对页面扫描的写法、`new-work.md`:148 对取证有效性的条款写法。可写路径 `skills/phoenix-ui/src`、`skills/phoenix-ui/evals`、`docs/phoenix-ui.md`，与批 1 的文件不重叠，可交给 subagent 与批 1 并行开发；`docs/phoenix-ui.md` 由主 agent 单人修改 | V-4、V-5；回写「实施进度」 |
| M4 | 阶段收口：构建同步、全量回归、真实使用验收与四平台候选 | M1–M3 完成 | 依次：①开发构建与 V-6；②用同一开发构建跑 V-7，修复后只复跑失效项；③V-8 并在其后创建候选提交，记录完整候选 SHA；④经授权推送，以该 SHA 精确匹配 run ID 跑 V-9 并回填二者及四平台结果。改动 skill 源或引擎源码后须从①重建再复验受影响项 | V-6、V-7、V-8、V-9；回写「实施进度」 |

提交点：M1、M2、M3 各自交付后分别提交（M1 与 M2 同批，不等批末检查；M3 的提交含渲染生成物）；M4 的③通过后做候选提交；全部里程碑完成后最终提交一次并回填 SHA。推送仅在用户授权时进行（C-9）。

并行：M1 与 M2 共用规则包接口，串行；M3 可在批 1 进行时由 subagent 并行开发，主 agent 负责 `docs/phoenix-ui.md`、渲染与批 2 验收；V-7 三个场景共用一个开发构建，夹具目录、端口与浏览器资料目录各自独立，可顺序或并行。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| “以汉字为主”按过半数判定（D3） | 标识符或路径密集的中文技术段落不触发中文规则，继续按内置拉丁规则判定（与升级前相同，不是回归）。可选改法：改为“汉字多于假名与谚文之和，且占可见字符 ≥ 20%”，这类段落也会按中文口径判定；代价是需要修订 PRD §2.1 的定义与 WORK-F5/AC-6 的措辞，且汉字占比 20%–50% 的混排段落会多出中文规则发现 | 用户选择；改法只动 `han_dominant` 一个函数与对应用例，本期任何时点都能换 | M2 批末（V-2 之前定下最省事；之后改只需重跑 V-2 与 V-1 第①⑤项） | 否 |
| A-1：Linux 上探针与字体判定的行为 | 探针在 Linux 两个平台正常取数已由 V-9 核实（通用字体族夹具无未核实备注、夹具表通过）。混用、无通用族两类依赖系统字体的判定在 Linux 仍只有 macOS 上的自动化证据（D14） | 已收口；Linux 上这两类判定若有误报，维护者按反馈处理 | M4（已核实） | 否 |
| 探针临时插入节点 | 监听 `<html>` 子树变化的页面可能出现可观察的副作用 | V-1、V-7 的真实页面观察；出现可复现问题时按 ADR-2 的重新评估处理 | M4 | 否 |
| 字体名匹配的局限 | 用本地化字体名（如“微软雅黑”）且不写通用族的声明，可能被报为第三种情况；反过来系统也可能用本地化名报告字体（V-9 的 macOS runner 把 PingFang SC 报为“蘋方-簡”），声明英文名且不写通用族时同样会被报为第三种情况。这类声明补一个通用族即可消除，文案只陈述事实，docs/phoenix-ui.md 已写明 | 维护者按 PRD Q2 的方式观察误报并调整 | 发布后 | 否 |
| 单位词表覆盖不全 | 少见量词前的拆行不报（advisory 漏报） | 维护者按反馈增补 §5.3 的常量 | 发布后 | 否 |
| 按意群拆成 span 的文字不在候选范围内 | 候选沿用内置 `line-length` 的范围，要求元素有直接文本（§5.2、PRD WORK-F5 行为段）。V-5 与 V-7 的会话都按 typeset 的“按意群断句”把标题或短段落整段放进 `inline-block` 的 span，元素从此没有直接文本，五条规则与字体备注都不再覆盖它；修掉孤字的写法同时让这段文字退出了检测 | 用户决定是否把候选放宽到“直接文本或行内后代的文本”；这会改 PRD 的元素范围与 D12，另需核对对内置规则让位的影响，不在本期擅自扩大 | 发布后（或 P3 立项时） | 否 |
| 中文门槛 1.5 / 40 字的误报率（PRD Q2） | 存量中文项目升级后 warning 增多 | 按 PRD：先按提案上线，项目可逐条关闭；维护者据反馈调整并对外说明 | 发布后 | 否 |
| 推送授权与 CI | V-9 不能执行，M4 不能记完成 | 收口前向用户申请推送；未授权时记入当前阻塞 | M4 | 否（影响收口结论，不影响实施） |
| 宿主会话与 comp-led 演练成本 | V-5、V-7 无法执行或 A3 耗时过长 | 运行前用 `claude --help` 核对参数并用最小会话确认 skill 可加载；A3 只判文案处理，首屏门禁结果不计入 | M3、M4 | 否 |

- 最终状态：**Ready**。
- 定级理由：范围与期次由 PRD v1.2 确定，P2 与 P1 没有前置关系；规则名（PRD Q1）与判定方式（PRD Q5 的默认）已在 D2、D3 定下，换判定方式是不阻塞的可选项；逐行文本与实际字体的基础取数有 §1.1 所载 macOS 一次性实测（原始脚本未入库，本次未独立复现），元素/run 与失败归属增量按 V-1/V-2 验证，Linux 按 A-1/V-9 收口；没有影响范围、安全、数据或外部契约的未决项。推送授权与宿主会话只影响收口检查能否执行，按 §9.3 记缺口。

## 12. 已知坑与历史教训

- 改过 `crates/core`、`crates/foundation` 不重新生成页内包，`test:phoenix` 第一步就失败（tools/phoenix-ui/docs/ENGINE.md「The browser bundle」；scripts/test-phoenix-ui.mjs:27）：M1 里生成并随 M1 提交。
- `crates/`、`browser-bundle/` 下新增的 js、css、html 与字体文件会进许可清单（scripts/check-phoenix-deps.mjs:457-466）：探针脚本写成 Rust 字符串常量，夹具放 `tools/phoenix-ui/tests/fixtures/`，夹具不带字体文件。
- `crates/rules/tests/text.rs`:574-588 写死注册 ID 列表，新增规则不改这里会失败。
- macOS 上 Chrome 无法在 `sandbox-exec` 内初始化沙箱，浏览器测试须在 sandbox 外跑（scripts/test-phoenix-ui.mjs:45-47）。
- 发行版 `panic = "abort"`（tools/phoenix-ui/Cargo.toml:38）：规则与探针对外部输入不 `unwrap`、不越界索引。
- `CSS.getPlatformFontsForNode` 不强制排版：刚插入、尚未排版的节点返回空列表。静态页面上下一帧通常先到所以看不出来，点击后带过渡动画的状态取证里约半数会读空，表现为整页“未核实”的虚假备注（V-7 A1 发现，探针已在插入后强制排版并在读空时重读）。
- `CSS.getPlatformFontsForNode` 未先 `CSS.enable` 会报 `CSS agent was not enabled`；`display: none` 的节点返回空列表，探针容器必须用 `visibility: hidden`（§1.1 实测）。
- 逐字分行得到的行文字，行尾空格留在上一行（§1.1 实测）：规则判定前先去首尾空白。
- 依赖系统字体的断言在没有中文字体的 CI 机器上不成立：四平台夹具只用 `min-content` + `keep-all`、em 宽度与 `@font-face` 失败这类与字体无关的构造（D14）。
- 改 `skills/phoenix-ui/src` 后不渲染，`GENERATED.json` 失配，`render --check` 与 `test:phoenix` 会失败（docs/phoenix-ui.md「维护者」节）：M3 渲染并提交生成物。
- 宿主会话演练的三个坑（docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md「V-9 演练偏差」）：夹具放仓库树外；skill 用副本而不是指向会话目录外的符号链接；`claude -p` 在会话留有后台进程时会先输出结果再挂起。
- 开发构建没有引擎下载地址，安装必须带 `--release-dir`（docs/phoenix-ui.md「安装」选项表）；`smoke-phoenix-ui.mjs --stage-runtime` 重跑前清理旧的开发构建输出。
- 下次发布不能用 `--reuse-engines`：引擎源码已变（docs/phoenix-ui.md「维护者」节的复用条件）。
