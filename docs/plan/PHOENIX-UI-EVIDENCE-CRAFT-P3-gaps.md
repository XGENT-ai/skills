# PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps · XGENT-ai Skills · Phoenix UI 素材结果行补许可、Linux 浏览器残余关闭尝试与 PRD 同步（引擎增量 · 无新增文件）

```formal
INTENT:
  GOAL: "直接依据本计划、仓库规则和当前代码实施或续做"
  SUCCESS: "全部里程碑退出检查通过，按‘实施进度’完成最终提交、SHA 回填与自查，并提示用户发起代码评审"

RULE @implementation_entry:
  MUST: "实施须知：先读下方‘实施者定位’‘实施进度’，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；goal 执行也遵循这些约定"
```

> **计划状态：Ready**
>
> 调查基线：2026-11-12 · `3094bbeda67dbdbb72806c1c82f27bc2fbb5a403`（main）· dirty：未提交内容只有 `.xgent-ai/sdlc/` 台账与 `docs/plan/` 下的评审、差异报告和本计划；引擎、skill 源码、docs 与 PRD 无未提交改动，下文事实取自当前工作树。
>
> 需求来源：[Phoenix UI 取证与工艺增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.3 · P3（第 3 期）：WORK-F7/AC-1、WORK-N1/AC-4（WORK-N1 在本期终验）、WORK-F8/AC-4 与 WORK-N2/AC-3 的文字同步。补齐 [PHOENIX-UI-EVIDENCE-CRAFT-P3](PHOENIX-UI-EVIDENCE-CRAFT-P3.md) 交付后经 [PRD 差异核查](PHOENIX-UI-EVIDENCE-CRAFT-P3.prd-dev-gaps.md) 确认的明确遗漏 G-01，以及用户 2026-11-12 对 G-06、G-07 的决定 D-01、D-02（连带 G-05 的一句文字）。
>
> 本期交付：**`phoenix-ui media` 不加 `--json` 时，检索与预览的每条结果在分级旁带上许可文字；浏览器启动参数加两个把组件更新检查与 GCM checkin 改发到本机丢弃端口的开关，经四平台 CI 判定后清空或收窄 Linux 的残余清单与文档；PRD 写明视频取用的浏览器前置并写准一处许可口径。**
> **开发范围：`tools/phoenix-ui/crates/cli/src/media/mod.rs`（`print_result` 与一条既有用例）、`tools/phoenix-ui/crates/browser/src/cdp.rs`（`default_chrome_args` 与 `default_args_shape`）、`tools/phoenix-ui/crates/cli/tests/network_boundary.rs`（残余清单）、`docs/phoenix-ui.md`、`docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md`；依据见 §0.1。**
> 本期独特职责：原计划把联网残余如实登记、把许可放在 `--json` 里；本计划把许可补到终端输出，并对 Linux 上的两项残余做一次有判据的关闭尝试。
> **顶层排除：不改 `--json` 输出、取用行、联系表、`--help` 与 oracle golden；不改 skill 文本与生成物；不改视频取用的实现；不屏蔽主机、不加未经实测的其他开关；不改原计划的正文与完成记录；不发布 npm/R2。**

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

- 最近更新：2026-11-12 17:10（UTC+8）
- 当前进度：0/2 个里程碑完成
- 当前状态：M1 已交付（mod.rs、docs、PRD v1.4 与既有用例新断言，media::tests 批内绿），提交见完成记录 M1 行；批末检查未运行
- 最近完成：无
- 下一步：M2 · §5.2 第一步候选（cdp.rs 两个改发开关与 default_args_shape 断言，network_boundary.rs 不动）交付提交；随后批末 V-1–V-3
- 当前阻塞：无
- 代码基线：`3094bbeda67dbdbb72806c1c82f27bc2fbb5a403`（main）

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 进行中 | 2026-11-12 17:10 | §5.1 print_result 条目行分级旁括注许可原文（空许可不带括号）＋既有用例新增不带 --json 断言；docs「素材命令」补结果行字段说明；§5.3 PRD 四处与 v1.4。批内 media::tests 绿；批末 V-1–V-3 未运行 | [M1.md](PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.records/M1.md) | 待提交 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

需求 ID 沿用 PRD 工作号（尚未录入 SPMS）。G-ID、D-01、D-02 是[差异报告](PHOENIX-UI-EVIDENCE-CRAFT-P3.prd-dev-gaps.md)的编号。WORK-N1 的终验期是 P3，本计划完成后它的 AC-4 才有最终结论；WORK-N2 的终验期是 P4，本计划只同步文字，不转“已上线”。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F7 | 功能 | PRD §3 WORK-F7/AC-1 · P3；差异报告 G-01（明确遗漏）、G-05 | 不加 `--json` 的检索与预览结果，每条在分级旁带上条目的许可文字；PRD 第 318 行把荷兰国立博物馆的许可口径写准 | §5.1、§5.3；V-1、V-2、V-3 | 已确认 |
| WORK-N1 | 安全/隐私 | PRD WORK-N1/AC-4 · P3（本期终验）；差异报告 G-06、D-01（用户 2026-11-12 决定） | 对 Linux 上浏览器自身的组件更新检查与 checkin 请求做一轮关闭尝试；清零则清空残余清单与文档，没清零则保留登记并写明试过什么 | §5.2、§7；V-2、V-4 | 已确认 |
| WORK-F8 | 功能（文档同步） | PRD WORK-F8/AC-4 · P3；差异报告 G-07、D-02（用户 2026-11-12 决定） | PRD 写明取用视频需要本机浏览器且支持该视频编码，否则该视频取用失败；实现不变 | §5.3；V-3 | 已确认 |
| WORK-N2 | 兼容/分发（文档同步） | PRD WORK-N2/AC-3 · P3（终验期 P4）；差异报告 G-07、D-02 | AC-3 注明视频取用以上述前置为前提；实现不变 | §5.3；V-3 | 已确认 |
| C-1 | 约束 | AGENTS.md「Working in This Repository」 | 留在当前分支；推送只在用户授权时进行 | §10 提交点、V-4 | 已确认 |
| C-2 | 约束 | 原计划 D15 与 §5.16（用户 2026-11-11 确认）；差异报告 D-01 | 启动参数只保留实测有效的开关；不用 `--host-resolver-rules` 屏蔽主机；仍有残余时如实登记在测试与 docs | §5.2 | 已确认 |
| C-3 | 约束 | tools/phoenix-ui/tests/oracle/README.md | oracle golden 逐字节比较；本计划不改 `--help` 与任何离线用例的输出，golden 不应变化 | §5.1；V-4 | 已确认 |
| C-4 | 约束 | AGENTS.md `@skill_tool_independence`、`@surgical_changes` | 不改 skill 文本、独立 skill、安装器与发布流程 | §0.5 | 已确认 |
| A-1 | 假设 | Chromium 源码（2026-11-12 读取，未在浏览器上运行） | Linux 的 Google Chrome 156 遵守 `--component-updater=url-source=`，Chromium 147 遵守 `--gcm-checkin-url=`，改发后原主机不再出现，也不引出新的外发主机 | V-4 在四平台 CI 上判定；不成立时按 §5.2 的第二条路线处理；责任人：实施者；最晚 M2 | 开放 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 许可文字放在哪 | 终端结果行写成 `<编号>  <分级>（<许可>）  <宽>x<高>  <信源>  <条目标题>`，许可取条目的 `license` 原文 | 分级与它所依据的许可挨在一起；`--json` 输出、联系表与取用行不变 | WORK-F7；tools/phoenix-ui/crates/cli/src/media/mod.rs:427-438 |
| D2 | 关闭办法 | 把两条请求改发到本机丢弃端口：`--component-updater=url-source=http://127.0.0.1:9/`、`--gcm-checkin-url=http://127.0.0.1:9/` | 与现有的 `--gaia-url=http://127.0.0.1:9` 同一做法；不屏蔽主机，目标页面自己的资源不受影响 | WORK-N1；C-2；tools/phoenix-ui/crates/browser/src/cdp.rs:62-112 |
| D3 | 由什么判定 | 第一版候选只加两个开关，联网边界测试的 Linux 残余清单先不动，让测试在四平台跑完四个场景；由候选那一轮的完整 netlog 逐个主机定开关、清单与 docs 的去留 | 清单留着不是开关有效的证据，候选那一轮测试通过只说明没有清单外的主机；先清空的话，开关无效时测试在第一个越界场景中止，拿不齐四个场景的 netlog。判定后的处置另行提交再跑一轮，以这一轮全绿收口 | A-1；差异报告 D-01；tools/phoenix-ui/crates/cli/tests/network_boundary.rs:18-22、:336-441 |
| D4 | 视频取用 | 实现不变，只同步 PRD | linux-arm64 上不带 H.264 的 Chromium 取不了视频，属写明的前置限制 | 差异报告 D-02 |

没有高影响、难回退的决策，不设 ADR。

### 0.5 明确不在本期

- **差异报告的 G-02、G-03、G-04**——保留现状，无动作。
- **差异报告 §5 的四项待核实**（国会图书馆真实响应样本、最终 skill 文本的宿主实跑、发布包安装与升级、Edge/Brave 的后台请求）——不纳入；各自的补证时机见差异报告 §5。
- **skill 文本要求向用户展示许可**——不改；本计划只补命令输出，skill 源文件、渲染生成物与 evals 不动（C-4）。
- **取用行、联系表与 `--json` 的格式**——不改。
- **视频取用的替代解码手段或缩略图退路**——用户已选择不做（D-02）。
- **再加其他关闭开关、屏蔽主机、为 Edge/Brave 做验证**——不做（C-2）。
- **原计划 PHOENIX-UI-EVIDENCE-CRAFT-P3 的正文与完成记录**——不改；它是已完成的历史记录。
- **发布、npm 版本与 SPMS 录入**——按维护流程另行执行。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·缺口] 终端结果行没有许可**：`print_result` 对每个条目输出 `ref`、分级中文名、`originalWidth`x`originalHeight`、`provider`、`title`（tools/phoenix-ui/crates/cli/src/media/mod.rs:427-438），`search` 与 `preview` 共用这一段；条目的 `license` 只在 `--json` 里。实测一条 CC BY-SA 4.0 的条目输出为 `C1  可上线  4000x3000  commons  Carte marine 1650` → 在分级后加 `（<license>）` → 漏做则 WORK-F7/AC-1 在终端输出上不成立。
- **[已核实·足够] 分级中文名与许可字段**：`rights::use_label` 给出“可上线 / 需核对 / 仅参考”；每个信源的映射都给条目写入非空的 `license`（tools/phoenix-ui/crates/cli/src/media/sources.rs 的各 `parse_*`，如第 741-750 行）。
- **[已核实·足够] 可扩展的既有用例**：`search_isolates_sources_keeps_refs_and_never_serves_cached_results`（mod.rs:535 起）经 `run_with` 把 Commons 指向本机 `tiny_http`，两条样本的许可分别是 `CC0` 与 `CC BY-NC`，目前只用 `--json` 调用。
- **[已核实·足够] 离线 oracle 与帮助不含结果行**：`tools/phoenix-ui/tests/oracle/golden/` 里 media 的用例只有帮助、参数错误、越界与空核对，没有条目行；`--help` 没有描述结果行格式（C-3）。
- **[已核实·缺口] Linux 上的浏览器残余**：`default_chrome_args` 已有 `--disable-component-update` 与 `--gaia-url`、`--google-apis-url`、`--lso-url` 三个改发到 `http://127.0.0.1:9` 的开关（tools/phoenix-ui/crates/browser/src/cdp.rs:62-112）。四平台 CI（run 43141743915）的 netlog：Linux x64 的 Google Chrome 156.0.8078.79 在 `detect <url>` 与 `capture` 的 4 个场景都向 `update.googleapis.com` 发 `POST /service/update2/json`；Linux arm64 的 Chromium 147.0.7727.0 在 `capture` 的 2 个场景向 `android.clients.google.com` 发 `POST /checkin`；macOS 的 Chrome 157 无残余（[原计划收口记录的联网边界排查](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5.md#v-11-联网边界排查)）→ §5.2 → 漏做则 WORK-N1 的“不自动更新或下载”在 Linux 上只能靠登记。
- **[已核实·足够] 两个改发开关存在于对应版本**：Chrome 156.0.8078.79 的组件更新器读取 `--component-updater` 的逗号分隔值，其中 `url-source=<地址>` 覆盖更新检查的地址（[component_updater_command_line_config_policy.cc](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.79/components/component_updater/component_updater_command_line_config_policy.cc)）；Chromium 147.0.7727.0 定义了 `gcm-checkin-url`，“设置 GCM checkin 使用的服务地址”（[gservices_switches.cc](https://raw.githubusercontent.com/chromium/chromium/147.0.7727.0/google_apis/gcm/engine/gservices_switches.cc)）。是否在运行中生效见 A-1。
- **[已核实·足够] 联网边界测试与证据通道**：`browser_commands_only_request_page_resources_and_documented_residuals` 对两种页面各跑 `detect` 与 `capture`，要求代理记录的主机是“残余清单加页面外部资源”的子集；残余清单在 Linux 上是两个主机、其他平台为空（tools/phoenix-ui/crates/cli/tests/network_boundary.rs:18-22、:324 起）；测试打印浏览器版本，并把每个场景的 netlog 复制到 `local/phoenix-ui/ci-prepare/network-<verb>-<page>.netlog.json`，随 CI 的 `phoenix-ui-<平台>-evidence` 制品上传（.github/workflows/phoenix.yml:171-176）。四个场景在同一个测试里依次运行，断言在循环内：某个场景出现清单外的主机时测试当即中止，后面的场景不运行也不留 netlog（:336-441；原计划收口首轮清单为空时，Linux x64 在第 1 个场景、arm64 在第 3 个场景中止）。测试给浏览器加了 `--proxy-server`，本机回环地址不走代理，所以改发到 `127.0.0.1:9` 的请求不会出现在代理记录里，要看 netlog 确认原主机确实不再被请求。
- **[已核实·足够] 启动参数的单元测试**：`default_args_shape` 断言列表里有 `--disable-component-update` 与 `--gaia-url=http://127.0.0.1:9`（cdp.rs:2292-2310）。
- **[已核实·足够] CI 触发**：Phoenix 工作流在 push 到 main 且改动落在 `tools/phoenix-ui/**` 等路径时运行四平台 UI 任务与 Node 18 冒烟，UI 任务里跑 `npm run test:phoenix`（.github/workflows/phoenix.yml:10-31、:138-143）；`docs/phoenix-ui.md` 也在触发路径里，PRD 与 `docs/plan/` 不在。
- **[已核实·缺口] 文档与 PRD 的四处文字**：docs/phoenix-ui.md 第 130 行写“Linux Chrome/Chromium 仍有下文列明的后台请求”，第 200-207 行是残余表，第 159 行附近没有说明终端结果行的字段；PRD 第 318 行写“美术馆只取公有领域或 CC0”，第 341 行 WORK-F8 的依赖只有 WORK-F7，第 574 行 WORK-N2/AC-3 没有视频取用的前提，第 631 行的前置依赖表没有把浏览器列给 WORK-F8 → §5.3。

### 1.2 拓扑与文件清单

全部在本仓 `XGENT-ai/skills`，没有新增文件。

| 位置 | 改动 | 所属 |
| --- | --- | --- |
| tools/phoenix-ui/crates/cli/src/media/mod.rs | `print_result` 的条目行加许可；既有检索用例加一次不带 `--json` 的断言 | M1 |
| docs/phoenix-ui.md | 「素材命令」一节补一句终端结果行的字段（M1）；「浏览器与隐私」一条与「联网边界」的残余表按 CI 结果改写（M2） | M1、M2 |
| docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md | §5.3 所列四处文字与版本号 | M1 |
| tools/phoenix-ui/crates/browser/src/cdp.rs | `default_chrome_args` 加两个开关；`default_args_shape` 加两条断言 | M2 |
| tools/phoenix-ui/crates/cli/tests/network_boundary.rs | Linux 残余清单按 CI 结果清空或收窄 | M2 |

无需修改的邻接面：`media` 的检索、取用、联系表与 `--json` 输出；`--help` 与 oracle golden；`skills/phoenix-ui/` 的源文件与生成物；`scripts/`、安装器与 `.github/workflows/phoenix.yml`；`Cargo.toml` 与 `Cargo.lock`（没有依赖变化，许可清单不用重算）。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| Linux 上的 Google Chrome 156 与 Chromium 147（CI 镜像自带） | 已核实缺口（开关存在，运行效果未验证） | §1.1 的源码链接与 CI netlog | 加两个改发开关，由 V-4 判定；提供方是浏览器自身，本方不改浏览器 | 开关不被遵守：该主机继续出现在该平台的 netlog 里（候选的清单未动，测试不因此失败），按 §5.2 第二条路线处理 |
| Phoenix CI（四平台 + Node 18） | 已核实足够 | .github/workflows/phoenix.yml；原计划收口时的两次运行 | 零改动；推送候选后按完整 SHA 匹配运行并取回证据制品 | 推送未获授权：V-4 记未运行，M2 不记完成，WORK-N1 不下终验结论 |
| 素材信源（外部） | 已核实足够（原计划 V-1、A1，2026-11-12） | 原计划 §5.2 | 零改动；V-1 的检索走真实信源 | 单个信源不可用按既有行为报出；全部不可达时 V-1 记环境缺失 |

没有新增配置项、环境变量或对外字段。

## 5. 核心机制

### 5.1 终端结果行带许可

`print_result`（tools/phoenix-ui/crates/cli/src/media/mod.rs:427-438）的条目行由

```text
<编号>  <分级>  <宽>x<高>  <信源>  <条目标题>
```

改为

```text
<编号>  <分级>（<许可>）  <宽>x<高>  <信源>  <条目标题>
```

- `<许可>` 取条目 `license` 的原文，不截断、不翻译；例：`C1  可上线（CC BY-SA 4.0）  4000x3000  commons  Carte marine 1650`。
- `license` 为空串时只写分级，不带空括号（各信源的映射都会写入许可，这是索引被手工改过时的表现）。
- `search` 与 `preview` 共用这段输出，一并生效；取用行、汇总行、`UNAVAILABLE` 行与 `--json` 不变。
- docs/phoenix-ui.md 第 159 行所在段前加一句：不加 `--json` 时，检索与预览每条一行，依次是编号、分级及其后括号里的条目许可、原图尺寸、信源、标题。

### 5.2 Linux 浏览器残余的关闭尝试

第一步（候选）：

1. `default_chrome_args`（tools/phoenix-ui/crates/browser/src/cdp.rs:62-112）在 `--lso-url=http://127.0.0.1:9` 之后加两项：`--component-updater=url-source=http://127.0.0.1:9/`、`--gcm-checkin-url=http://127.0.0.1:9/`。`default_args_shape` 加两条包含断言。
2. `tools/phoenix-ui/crates/cli/tests/network_boundary.rs` 在候选里不动，Linux 残余清单照旧：清单留着只为让测试在 Linux 上跑完四个场景、留下完整的 netlog（§1.1），候选那一轮测试通过不说明开关有效。
3. 本机 V-2 通过后提交为候选，经用户授权推送，运行 V-4。

第二步取回候选那一轮的 netlog，逐个主机判定，再按下表处置。清零的判据：两个 Linux 任务都跑完四个场景，各 4 份 netlog 里都没有发往该主机的请求；某个任务没跑完四个场景时，不从它得出清零结论。处置另行提交并再推送一次，以这一轮四平台与 Node 18 全绿收口：

| 结果 | 处理 |
| --- | --- |
| 两个主机都清零，候选那一轮四平台与 Node 18 全绿，没有新的外发主机 | `network_boundary.rs` 删掉 `BROWSER_RESIDUAL_HOSTS` 两个按平台的定义及其注释，`allowed` 从空集合开始（只在外部资源页面加入 `boundary-probe.invalid`），测试函数名不改。docs/phoenix-ui.md 第 130 行改为：在 macOS 的 Google Chrome 157、Linux 的 Google Chrome 156 与 Chromium 147 上未观察到目标页面及其资源之外的请求，Edge、Brave 及其他版本未逐一验证；第 200-207 行的残余表换成一句话：组件更新检查与 GCM checkin 已由启动参数改发到本机丢弃端口，注明验证的版本与 CI 运行 |
| 某个主机仍出现 | 去掉对它无效的那个开关（C-2）；残余清单只留仍出现的主机，另一个主机若已清零，从清单与 docs 表里删去；docs 保留仍出现主机的行，并写明试过的开关、浏览器版本与 netlog 看到的请求 |
| 出现原先没有的外发主机，或其他浏览器测试因新开关失败 | 去掉引发它的开关，它针对的主机按上一行“仍出现”处理；不登记新的残余主机。这次修订仍保留残余清单，推送后取到四个场景的完整 netlog，再按前两行判定另一个主机 |

两条路线都不改 PRD 的 WORK-N1：v1.3 的 AC-4 以 docs 列明的清单为限，清单为空时即“目标页面之外无请求”。

### 5.3 PRD 同步

`docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md` 改四处，页头版本号由 v1.3 改为 v1.4，其他内容不动：

1. 第 318 行「许可分级沿用参考规则」一句里的“美术馆只取公有领域或 CC0”改为“大都会、芝加哥、克利夫兰三家美术馆只取公有领域或 CC0；荷兰国立博物馆按条目记录的许可分级，记录没有许可信息时为需核对”。
2. 第 341 行 WORK-F8 的依赖改为“WORK-F7；取用视频另需本机浏览器且支持该视频编码（§4），否则该视频取用失败”。
3. 第 574 行 WORK-N2/AC-3 末尾加一句：“取用视频以本机浏览器能解码该视频为前提（§4）；不满足时该视频取用失败并说明原因，其余素材不受影响。”
4. 第 631 行所在的前置依赖表加一行：提供方“用户本机”；所需能力“Chrome / Chromium / Edge，且能解码所取视频的编码”；成立条件与依据“只在取用视频素材时需要，首帧由本机浏览器解码；不引入 ffmpeg（D9）”；影响需求“WORK-F8（视频）、WORK-N2/AC-3（视频取用）”；未就绪时的产品状态“该视频取用失败并说明原因，视频与首帧都不落位；图片与贴图取用照常”。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N1 | PRD WORK-N1/AC-4；差异报告 D-01 | 页面检测与取证期间，Linux 上浏览器不再向 `update.googleapis.com`、`android.clients.google.com` 发请求；能否做到取决于 A-1 | 开关无效的主机保留在残余清单与 docs，写明试过什么 | §5.2 的两个改发开关 | V-2（本机 macOS 维持零残余）、V-4（四平台 CI 与 netlog） |

外发信息没有变化：两个开关只改变浏览器自身后台请求的去向，引擎进程仍不发请求。

## 8. 失败模式、发布与回滚

不引入新的运行期失败模式：终端多一段文字；两个启动开关不被浏览器识别时会被忽略，改发后的请求连到本机丢弃端口即被拒绝。新开关若影响页面渲染或浏览器启动，会在 V-4 的既有浏览器测试里暴露，按 §5.2 去掉该开关。

- 发布顺序：本计划只到四平台开发候选，不发 npm/R2。引擎源码已变，下次发布须走 CI 构建。
- 回滚：回退对应提交；没有数据或配置迁移。

## 9. 验证

共同环境：仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；Cargo 命令在 `tools/phoenix-ui` 下运行；本机 macOS 有可发现的 Google Chrome；浏览器测试不放进 `sandbox-exec`。批内最小反馈：`mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` 后直接运行 `target/debug/phoenix-ui media preview <编号>` 看输出，只在有具体疑问时运行。

| 检查 ID | 层级 | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- | --- |
| V-1 | 真实使用验收 | **设计者在终端里给一张航海史专题页的预览稿挑一张古地图，看清每张候选的许可后取用，放进预览页并实拍。** 角色：只读过 `phoenix-ui media --help` 与 docs/phoenix-ui.md 的「素材命令」一节，不读源码。干净的临时项目（仓库树外，`git init`），真实网络、真实信源。步骤与结果：①`media search "nautical chart\|old world map" --kind map --sheet`（不加 `--json`）：每条在分级后的括号里有许可文字，打开联系表对照编号选图（WORK-F7/AC-1）；②同一检索加 `--json`，逐条核对终端行的许可与 `license` 字段相同；③`media preview <两条编号>`：结果行同样带许可；④`media fetch <一条编号>`，把文件放进 `.phoenix-ui/preview.html` 引用（`media/<文件>`）；⑤`capture .phoenix-ui/preview.html --viewport desktop --out .phoenix-ui/check`：截图里能看到地图，报告的 `failedImages` 与 `failures` 为空（新启动参数下取证照常，WORK-N1）；⑥`media audit`：0 项。Agent 亲自打开联系表与截图查看 | `tools/phoenix-ui/target/debug/phoenix-ui`（绝对路径）在临时项目里运行；本机已有的 `HTTPS_PROXY` 等代理变量照常继承。安装器与启动器本计划未改，由 V-4 的 CI 冒烟覆盖，不另做安装 | M1–M2 交付齐备后；V-4 之前。V-4 之后若按 §5.2 第二条路线改了 `cdp.rs`，只复跑第⑤步 |
| V-2 | 局部 | 准入：G-01 出在所有既有检查都用 `--json`、终端输出没人看，窄断言防复发；WORK-N1 是明示的安全要求，“没有发出请求”只能在网络侧观察。内容：①既有用例 `search_isolates_sources_keeps_refs_and_never_serves_cached_results` 里加一次不带 `--json` 的检索，断言输出里有 `可上线（CC0）` 与 `仅参考（CC BY-NC）`；②`default_args_shape` 断言两个新开关在列表里；③本机联网边界测试 3 项通过，macOS 上清单为空仍成立 | `env -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u https_proxy -u http_proxy -u all_proxy mise exec --no-deps -- cargo test -p phoenix-ui --lib media::tests --locked --offline -- --test-threads=1`；`mise exec --no-deps -- cargo test -p impeccable-browser --lib default_args_shape --locked --offline`；`mise exec --no-deps -- cargo test -p phoenix-ui --test network_boundary --locked --offline -- --test-threads=1`；仓库根 `node scripts/check-phoenix-lints.mjs` | M1–M2 交付齐备后；候选提交之前 |
| V-3 | 局部（静态） | 文字同步对得上 §5.1、§5.3（WORK-F7、WORK-F8、WORK-N2）：PRD 只改四处与版本号；docs 的「素材命令」一节有终端结果行的说明；skill 源文件、生成物与 `--help` 无 diff | 仓库根 `git diff 3094bbed -- docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md docs/phoenix-ui.md`；`git diff --stat HEAD -- skills tools/phoenix-ui/tests/oracle scripts` 为空（基线注 2026-11-12：main 在 3094bbed 后落入无关的 feature-demo 修复，排除检查改对当期 HEAD；docs 那条仍对 3094bbed） | M1–M2 交付齐备后；M2 改完 docs 的残余段落后再看一次该段 |
| V-4 | 阶段收口 | 四平台候选与残余判定（WORK-N1/AC-4，A-1，C-3）：候选提交（两个开关已加，Linux 残余清单未动）上 Phoenix CI 的四个 UI 任务与 Node 18 冒烟的结果；从证据制品取回 4 个平台各 4 份 `network-<verb>-<page>.netlog.json`，记录各平台实际浏览器版本，逐个主机核对两个 Linux 任务里是否还有发往 `update.googleapis.com`、`android.clients.google.com` 的请求，并确认没有其他新的外发主机；按 §5.2 的表处置开关、清单与 docs，把所走路线、试过的开关与 netlog 事实写进记录与 docs。退出条件（两条路线都适用）：判为清零的主机在完整的四场景 netlog 里没有请求，且已从清单与 docs 表删去；没清零的主机只出现在最终的清单与 docs 里，对它无效的开关已去掉；没有登记新的外发主机；处置后的提交四平台与 Node 18 全绿。CI 里的 `test:phoenix` 同时是启动参数与 oracle golden 的全量回归，本机不另跑。Diagram 任务的失败与本计划无关时只报告 | 经用户授权推送 main 后：设 `candidate_sha=<候选提交的完整 SHA>`，运行 `gh run list --workflow phoenix.yml --branch main -L 100 --json databaseId,headSha,status,conclusion --jq "map(select(.headSha == \"$candidate_sha\"))"`，确认 `headSha` 相同后 `gh run watch <run-id>`；`gh run download <run-id> -n phoenix-ui-<平台>-evidence -D <临时目录>` 取回 netlog 与阶段回执 | M2，V-1–V-3 通过并完成候选提交后；按 §5.2 处置后的提交再运行一次（两条路线都有）。未获授权时记入当前阻塞，不记完成 |

### 9.3 真实环境

V-1 需要外网与本机浏览器；信源整体不可达时记环境缺失，不以本机样本代替。V-4 需要用户授权推送。缺任一环境时对应检查记“环境缺失/未运行”，M2 不记完成。

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 检索结果行带许可，PRD 同步 | 无 | 与 M2 同批。§5.1：`tools/phoenix-ui/crates/cli/src/media/mod.rs` 的 `print_result` 条目行与既有检索用例的一条断言（同构：同函数里取用行对 `use_label` 的用法，第 441-450 行）；`docs/phoenix-ui.md`「素材命令」一节一句。§5.3：`docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md` 四处与版本号（同构：原计划收口时对 WORK-N1 的两处改写，`git show 622cbfcf -- docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md`） | 与 M2 同批，V-1、V-2、V-3；回写「实施进度」 |
| M2 | Linux 浏览器残余的关闭尝试与收口 | M1 代码（同一候选提交一起上 CI） | §5.2 第一步：`tools/phoenix-ui/crates/browser/src/cdp.rs` 两个开关与 `default_args_shape`（同构：同列表里的 `--gaia-url` 一行及其断言）；`tools/phoenix-ui/crates/cli/tests/network_boundary.rs` 候选阶段不动。本机 V-1–V-3 通过后做候选提交，申请推送授权，运行 V-4；取齐四个场景的 netlog 后按 §5.2 的表处置残余清单、无效的开关与 `docs/phoenix-ui.md` 第 130 行及残余表，提交后再跑一轮确认全绿。两个里程碑除 `docs/phoenix-ui.md`（各改不同段落）外改动的文件不重叠，由主 agent 连续完成，不委派 | V-1、V-2、V-3、V-4；回写「实施进度」 |

提交点：M1、M2 各自交付后分别提交（不等批末检查）；V-1–V-3 通过时的提交即 CI 候选，记录完整候选 SHA；判定后的处置（残余清单、无效开关的去留与 docs 的收口文字）另行提交，推送后再跑一轮 CI，以这一轮四平台与 Node 18 全绿收口；全部里程碑完成后最终提交一次并回填 SHA。推送仅在用户授权时进行（C-1）。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1：两个改发开关在 Linux 的实际浏览器上是否生效 | 决定残余清单能否清空；不生效时 WORK-N1 在 Linux 上仍按登记口径验收 | V-4 判定；两种结果在 §5.2 各有路线，不需要再做决定 | M2 | 否 |
| 推送授权与 CI | V-4 不能执行，M2 不能记完成，WORK-N1 的终验结论下不了 | 候选提交后向用户申请推送 | M2 | 否（影响收口，不影响实施） |
| 新开关引出新的外发主机，或影响既有浏览器测试 | 候选那一轮在该平台失败，直到去掉该开关的修订推送；开关只是无效时清单未动，CI 不因此变红 | 按 §5.2 表的第三行当即修订并复验，不长时间留红 | M2 | 否 |
| CI 镜像的浏览器版本变化 | 判定时的 Chrome 可能已不是 156（镜像升级），结论只对实际版本成立 | 记录测试打印的实际版本，docs 写实际验证的版本 | M2 | 否 |
| 许可文字较长 | V&A 的条款文字有一百多个字符，终端行变长 | 按原文输出，不截断；需要紧凑视图时看联系表 | 发布后 | 否 |
| 真实信源在验收时不可达 | V-1 的检索步骤无法执行 | 按 §9.3 记环境缺失，恢复后补跑 | M2 | 否 |

- 最终状态：**Ready**。
- 定级理由：范围来自差异报告的一项明确遗漏与用户 2026-11-12 的两项决定，没有待决的范围、安全、数据或外部契约问题。唯一的技术未知（A-1）依赖 Linux CI 环境，已排为 M2 的判定步骤，两种结果各有写定的路线。推送授权只影响收口检查能否执行。

## 12. 已知坑与历史教训

- 本机 Bash 是 zsh：带通配符的参数要加引号，未加引号的变量不按空格拆分（用户全局规则）。
- 本机代理存活时，media 的检索用例里“死端口”会经代理变成 502，断言按环境失败：跑 `media::tests` 前移除代理变量（[原计划评审修复验收记录](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M1.md#评审修复验收)）。V-1 的真实检索则要继承代理。
- 联网边界测试给浏览器加了 `--proxy-server`，回环地址不走代理：改发到 `127.0.0.1:9` 的请求不在代理记录里，判定开关是否生效要看 netlog 里原主机是否消失（§1.1）。
- 只回 502 的记录代理下看不出更新检查之后会不会下载；本计划的判据是请求不再发往原主机，不对“是否下载”下结论。
- 推送新候选会按工作流的 `cancel-in-progress` 取消同分支上还在跑的旧运行；等上一轮跑完或确认不再需要它的结果再推（原计划收口记录「当前候选」）。
- 预览页放在 `.phoenix-ui/preview.html`、用 `media/<文件>` 引用：放在子目录里用 `../media` 引用时，取证的本机文件服务器加载不到图片（原计划收口的宿主验收记录）。
- 处置提交改的 `network_boundary.rs`、`cdp.rs` 与 `docs/phoenix-ui.md` 都在 Phoenix CI 的触发路径里：推送后会再跑一轮，这一轮确认按最终清单全绿；主机是否清零以取齐四个场景 netlog 的那一轮为准。
- 原计划收口时记录过两次 Chrome 调试端点启动超时造成的测试中断：遇到同类失败只复跑该项，不改启动超时或断言。
