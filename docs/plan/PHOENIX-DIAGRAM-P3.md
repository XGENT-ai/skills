# PHOENIX-DIAGRAM-P3 · 品牌、预览、架构比较与唯一入口迁移

```text
::OBJECTIVE{implementation}
  target: 直接依据本计划、仓库规则和当前代码实施或续做
  ACCEPT: 全部里程碑退出检查通过，按「实施进度」完成最终提交、SHA 回填与自查，并提示用户发起代码评审
  实施须知: 先读下方「实施者定位」「实施进度」，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；使用 goal 执行时也遵循这些约定。
```

> **计划状态：Ready**
>
> 调查基线：2026-10-28 · main / dce8662270f993cae3521b59ceff8ac6f22e1b63 · dirty（计划、评审、决策及经验/SDLC 文档）；产品代码与该合并提交一致。
>
> 需求来源：[PHOENIX-DIAGRAM PRD v1.0](../PRD-PHOENIX-DIAGRAM.md) · **P3（第 3 期）**；承接 §7.2 的 16 条 P3 AC，跨期终验取 §7.3。WORK-F11/AC-3 的旧入口备份要求由用户 2026-10-28 的直接移除决定替代，执行与验收口径见 C-6。
>
> 本期交付：**品牌资源、显式本地预览、双输入架构比较，以及用户确认后直接移除旧 skill 的入口迁移候选；新入口安装可恢复和回滚，旧 skill 不备份。**
> **开发范围：本仓 tools/phoenix-diagram、skills/phoenix-diagram、diagram 启动/安装/构建/验收链及 dist；删除活跃 skills/archify；同步 README、elintp、documentation-writer 及其当前出图参考。依据见 §0.1。**
> 本期独特职责：完成十类图工具的接替和项目级迁移，保留旧输入、旧 HTML、后来修改和 Phoenix UI 的独立状态。
> **顶层排除：公开 npm/GitHub Release、用户活跃项目及个人安装的实际迁移、Phoenix UI 产品或 hooks、未知 infographic、其他图型比较、旧 Guided Views/Story/普通 Share Card 的重建。**

## 实施者定位

```text
::CONSTRAINT{implementer}
  角色: 执行本计划的 agent 是资深软件工程师，依据本计划、仓库规则和当前代码实施。

::CONSTRAINT{development_scope}
  规则: 开发边界
  范围: 仅实施本计划明确纳入的系统与改动；范围外依赖按 §4 的契约、提供方与解除条件跟进，不代建、改造或部署。源码可访问、自有服务或同仓目录不扩大开发范围。
  [IF] 外部缺口阻塞 => [DO] 回写影响与继续条件，推进不依赖它的已授权工作
  [IF] 依赖到位 => [DO] 从当前阻塞移除，剩余项按本方未实现或验收未运行重新归因
  变更: 就地修订偏差和委派不扩大范围，新增开发对象须有用户明确依据。
  fixture: 只覆盖约定契约的必要行为，不能成为替代外部系统的产品，也不能冒充真实集成验收。

::CONSTRAINT{communication_and_source}
  规则: 表达与源码
  报告: 用路径、需求/检查/里程碑 ID 定位，不复述计划。
  源码: 不放需求编号、agent 标记或规划元数据，注释解释必要原因；追溯放记录或提交说明。

::CONSTRAINT{identifiers_and_tasks}
  规则: 编号与任务
  定义: 需求/约束/假设 ID 只在需求账本登记一次，上游 ID 保留原编号及来源；检查、里程碑、任务 ID 分别由验证表、里程碑表、按需内部任务表定义，正文、NFR 展开、映射和进度记录只引用。
  修订: 保留已有 ID，删除/拆分时同步引用。
  [IF] 任务表存在 => [DO] 按交付物、前置成果与检查 ID 推进，任务进展及证据写入所属里程碑记录，不另设任务状态表

::CONSTRAINT{milestone_completion}
  规则: 完成的定义
  完成: 本里程碑退出检查全部通过才能记完成；未运行、失败、环境缺失分别记缺口，不冒充通过。
  终验: 集中到阶段收口的终验由收口里程碑负责，通过前不宣称整期或跨期需求已验收。

::CONSTRAINT{role_acceptance}
  规则: 角色验收
  模拟: 上手与业务场景由 Agent 按 §9 指定的知识、资料及权限约束模拟，不能借内部源码或实现者补步骤；记录实际结果与卡点，不把模拟结果写成真人数据。
  真实检查: 有工具和权限的真实环境检查由 Agent 执行；真人公测指标按其指定阶段另验，不冒充本阶段已通过。

::CONSTRAINT{implementation_route}
  规则: 按计划路线实现
  路线: 计划已定的技术路线、接口、复用锚点和检查命令直接照做，不重新调研，也不为已定决策另起方案设计、比较或评判；计划没写的实现细节按最近的同构代码就地决定。
  调查: 只读本里程碑的改动落点及其直接调用方和依赖。
  [IF] 代码事实推翻计划前提或路线走不通 => [DO] 停下，就地修订对应正文并在记录写明偏差；仅此条件触发路线重审

::FLOW{implementation_sequence|trigger:on_start}
  规则: 工作顺序
  [READ] 按 @resume_protocol 恢复，核对仓库规则、当前任务相关正文与依赖、工作树
  [DO] 按 §10 的连续实施批次推进，默认一个实现里程碑为一批，不按文件、函数、一次编辑或一轮 goal 对话切批
  [IF] 需要行为测试 => [DO] 用当前行为的最小用例先红、再绿、再重构，红绿循环不附带整组回归
  [DO] 完成本批相关实现和测试代码，再做局部退出检查
  [VERIFY] 下游所需前置、数据、安全、不可逆操作、发布及用户分期门及时验证，不等无关终验；稳定契约下可用 fixture 开发，不能替代真实集成
  [DO] 按 @verification_rhythm 与 @reverification 验证；批次通过后继续下一可执行批次，里程碑完成按 @progress_writeback 回写，汇报或回答状态询问后继续实施
  [IF] 用户明确暂停、取消或切换任务 => [DO] 按 @writeback_timing 保存后停止

::CONSTRAINT{verification_rhythm}
  规则: 验证节奏
  批内: 只为具体失败、技术疑问或必要前置运行能回答它的最小测试/编译检查；没有这些触发就继续实现。
  局部退出: §9 中整组局部检查（含受影响模块的回归和静态检查）等本批交付物齐备后统一运行，不能因命令叫“局部/快速”就每改一点重跑。
  收口: 全量测试、端到端、真实环境与界面走查、故障注入、评测、干净构建等只在指定阶段收口运行，同环境走查合并。
  成本: 优先复用增量构建、缓存和已就绪环境，不无故清缓存、重启栈或重复启动尚在运行的检查；调优用缩小版本迭代，达标后再跑完整检查。
  检查门: 仓库或用户明确指定的检查门仍须遵守。

::CONSTRAINT{reverification}
  规则: 复验时机
  失效: 证据仅在实际覆盖的代码、契约、配置、依赖或数据改变时失效；将受影响检查合并到下一适用批次退出或阶段收口，不因失效立即停工。
  合并: 同批连续改动只复验一次，未补证不能宣称新版通过。
  [IF] 下游要依赖该证据或即将跨风险门 => [VERIFY] 先补必要检查
  [IF] 检查失败 => [DO] 先定位原因，完成针对同一原因的修复 => [VERIFY] 失败项与修复波及的检查；未影响的已通过项直接引用
  [IF] 共享契约、公共入口、构建或依赖变化 => [DO] 核对波及面，必要时扩大复验；不能把所有局部修复都升级成整组或全量重跑
  复用: 续做、汇报、回写和提交本身不使证据失效。

::CONSTRAINT{parallel_delivery}
  规则: 并行交付
  条件: 独立任务在收益超过委派/集成成本且工具可用时使用 subagents，模型默认继承。
  任务说明: 只给目标/ID、稳定契约、可写与禁改路径、运行资源和检查；共享文件单人负责，数据库、端口、浏览器等隔离或串行。
  交付: 子 agent 返回结果、路径、验证证据与缺口，默认不另写任务文档；主 agent 审查并集成验收。
  异常: 边界变化先回报，失败只阻塞依赖任务。
  结束: 结束对话前收集或停止仍在运行的任务。
```

## 实施进度（实施期持续更新）

```text
::CONSTRAINT{progress_entry}
  入口: 本节是本计划跨对话恢复的唯一汇总入口。
  固定协议: 协议文字不随实施改写，实施期只更新「恢复快照」和「完成记录」。

::CONSTRAINT{resume_protocol}
  顺序: 续做时先读仓库规则、本节、当前任务相关正文与依赖，再核对 git status 和相关 diff。
  [IF] 记录与工作树不符 => [DO] 先查明原因，不凭记录覆盖用户改动，也不凭代码存在推断验收已通过
  按需读取: 只有继续未完任务、核验前置验收、排错或审查时，才按链接读取对应记录的相关部分；不要默认加载全部记录或原始日志。
  恢复: 从未完交付继续，不先例行重跑回归。
  检查前: 核对当前批次、触发原因与记录中的证据；未到验证点且无具体失败/疑问或必要前置就继续编码，已通过且覆盖范围未变的检查直接引用。

::CONSTRAINT{writeback_timing}
  规则: 回写时机
  [IF] 里程碑完成 => [DO] 立即按 @progress_writeback 回写，再报告完成或提交
  [IF] 下游只需部分成果 => [DO] 所需前置检查通过并在记录保存证据即可推进
  [IF] 实质进展后受阻、暂停、交接、发现偏差或结束对话 => [DO] 保存成果与缺口
  排除: 普通子任务切换不单独回写。

::CONSTRAINT{progress_writeback}
  规则: 一次回写
  顺序: 主 agent 更新对应记录，再同步快照变化字段和完成记录一行，不重写无变化内容或追加流水。
  快照: 写当前任务、进展与下一步，命令及证据只进记录。
  完成记录: 首次回写删占位行，每个已记进展的里程碑恰好一行，状态仅「进行中 / 阻塞 / 已完成」；完成时移至已完成行末，最近完成取该行。
  格式: 更新时间带时区，摘要 1–2 句，记录填相对计划的链接（如 [M1 记录](FEATURE.records/M1.md)）。
  定位: 回写在本节的恢复快照、完成记录及对应记录文件内定位，不按里程碑 ID、「状态」等同名字样对全文件查找替换；里程碑表、检查表、ADR 等定义行不随进度改动。
  [IF] 范围、决策、接口、数据、风险或退出条件有偏差 => [PATCH] 就地修订对应正文，不追加勘误历史
  [IF] 来源 CR 的承接请求或明确子范围已完成 => [DO] 按 @source_cr_writeback 核对并回写实现结果
  [DO] 回写后按 @project_index 同步已启用的台账
  [VERIFY] 每次回写后直接执行 @progress_consistency

::CONSTRAINT{source_cr_writeback}
  规则: 来源 CR 完成回写
  入口: 仅处理「需求来源」明确由本计划承接的 CR 请求；保留 CR/原 R/Q/RESP 语境，以既有需求/验证映射核对承接范围与收口检查，不因文档提及 CR 就改写。
  时点: 对应请求或明确子范围实现且约定检查通过后，在本次进度回写时追加结果，不必等无关里程碑；最后一个里程碑完成时再次核对有无漏回告。
  核对: 通读来源 CR，核对最新有效请求与往返答复、计划范围及适用验证证据；本计划仅承接部分或仍需其他计划交付时只回告已完成部分，列出剩余项，不关闭整条请求或整份 CR。
  写入: 复核实际回写文件的最新内容、保留并发增量，在文末追加实现结果 RESP，沿用编号并引用原请求和被更新的旧 RESP，说明哪些旧「未实现」判断被本轮取代；附实现行为、计划/里程碑、代码版本、验证结果、使用条件与剩余配合，不改正文或历史。
  边界: 本方实现、发布生效与对方验收分别陈述；无证据不标已发布或已获对方验收，不自动发送、部署或改平台状态，不外写敏感内部证据。
  去重: 同一请求范围、实现版本和验证结论已有实质等价回告时跳过，不重复追加或更新时间；有变化只追加受影响范围，不因无关 HEAD 变化记新轮。
  异常: 来源缺失、目标不唯一、只读或无法保真写回时，在对应记录保存可追加答复、原因和实际目标，恢复快照的下一步保留恢复动作；仅更新工作副本须说明原件未改，不宣称回写完成。
  留证: 写入后回读，确认范围、编号及历史完整，在对应里程碑记录保存实际 CR/RESP 链接；无需重新加载规划或答复技能。

::CONSTRAINT{milestone_records}
  规则: 独立记录
  路径: 按需建 <计划名>.records/M<n>.md。
  头部: 写计划链接、更新时区、状态及验证基线（commit SHA；未提交写 dirty@起始SHA 与关键改动路径）。
  正文: 只写交付行为/路径、必要偏差、未完项与继续动作；未完项含当前批次剩余交付、受影响而需复验的检查及下次执行点，按回写时机保存，不每改一处记一笔。
  验收: 按「检查 ID/覆盖范围 | 命令或步骤 | 结果/必要证据」记录，共同环境和基线在头部写一次，有差异再单列。
  追溯: 同一测试组可覆盖多条退出条件，须逐条可追溯；引用原证据与复用依据，不复制断言全文、成功日志或调试流水。
  证据: 命令、结果和必要的少量输出，不为留证另写一次性脚本、哈希清单或大体量证据目录。
  缺口: 未运行、失败、环境缺失明确区分；只有复杂交接、排错或证据需独立保存时才另建任务记录/日志/截图并链接。

::CONSTRAINT{project_index}
  规则: 项目跟进索引
  启用: 仓库已登记状态台账，或存在仓根 .xgent-ai/sdlc/protocol.md 时启用；兼容已有 docs/project-status/protocol.md，两套并存未切换时先核对，不双写；未启用则跳过。
  同步: 上述回写完成后读取台账协议，只同步实际改变的当前状态和来源链接，必要时更新索引；默认直接更新本计划状态页。
  来源复用: 依据直接链接本计划的恢复快照/完成记录、相关里程碑记录中的验收或未完项章节、已有评审/处置记录及其版本；不重写成 SDLC event、过程总结或证据副本，不先造 event 再替代原记录作为来源。
  去重: 里程碑推进、验证结果更新、来源版本变化、最终提交和整份计划完成都不单独构成新建事件的理由。
  留存门槛: 新建前先核对原记录及其可取回版本；仅仓内明确要求不可变审计，或仍有无法由原记录与当前页保留的并发/纠错事实时独立留存，说明依据与必须保留的差异，其余直接引用。
  旧协议: 仓内旧协议强制事件时遵守其要求，只保留最小必要内容与原记录链接，不复述实施和验收过程；不自行升级协议。
  范围: 复用已有计划 ID，只更新本次触及的范围，保留其他计划和各轮评审，不复制本节及里程碑记录；台账仅作项目索引，不替代本节的恢复快照。
  跨机器: 使用仓根相对路径与明确基线，未提交成果标明同步限制；登记失败说明缺口，不将原成果回退。

::CONSTRAINT{commit_protocol}
  规则: 提交
  提交点: 在里程碑表后的「提交点」提交；里程碑较多时在标出的阶段收口后提交，全部里程碑完成后必须提交一次。
  优先约定: 仓库规则或用户对分支、提交信息、签名或是否由 agent 提交另有要求时从其要求。
  提交前: 先回写本次进度；提交点要求的检查就是本次覆盖的里程碑或阶段收口的检查，已通过且未失效的证据直接引用，不为提交重跑。
  暂存与钩子: 只暂存本计划相关改动，不纳入、不回退他人改动，不跳过提交钩子。
  提交后: 把短 SHA 写入完成记录中本次覆盖的各里程碑行的「提交」列，再单独提交这次只改计划文件的更新，该提交不回填自己的 SHA。
  修复提交: 代码评审修复的提交写作「修复」加短 SHA，UI 验收修复的提交写作「UI修复」加短 SHA，均追加在原 SHA 之后。
  [IF] 已完成但未到提交点 => [DO] 对应行写「待提交」
  [IF] 无法提交 => [DO] 写明原因并记入当前阻塞，不伪造 SHA

::CONSTRAINT{completion_handoff}
  规则: 全部完成
  [IF] 最后一个里程碑完成 => [DO] 按 @source_cr_writeback 核对来源 CR 回告 => [DO] 按 @commit_protocol 完成最终提交并回写 SHA，确认每个已完成行的「提交」都有 SHA => [DO] 恢复快照的「下一步」写代码评审；仍有 CR 回写缺口时一并保留
  报告: 随后向用户报告完成，并提示下一步用 review-code 以本计划路径为参数评审本计划；它从完成记录找到要审的提交，并纳入未提交改动。
  发起: 评审由用户发起，实施中不自行启动评审或修复评审意见。

::CHECK{progress_consistency}
  规则: 回写后自查
  OBSERVE: 每次回写后核对恢复快照、完成记录、记录链接及本计划文件的 git diff；非 Git 仓库对照回写前副本。
  PASS: 当前进度 n/N 的 N 等于里程碑总数，n 只计状态为「已完成」的行；最近完成对应最后一条已完成记录，不按编号推算；链接指向已落盘文件；已完成行的「提交」是短 SHA 或「待提交」，全部完成时每行都有 SHA 且没有「待提交」；本节以外只有有意修订的正文，定义行没有被进度文字改写。
  边界: 上述结构、计数与链接检查不证明验收真实通过。
```

### 恢复快照

- 最近更新：2026-10-31T12:08:36+08:00（Asia/Shanghai）
- 当前进度：7/8 个里程碑完成
- 当前状态：M1–M7已完成；M8终验未完成。本地修复7c89d08已保留；此前将旧候选分支删除列为阻塞是错误判断，CI可沿当前main认证，不需要新建分支
- 最近完成：M7 · 最终候选接线、完整来源许可及V-7通过
- 下一步：当前按用户要求保留本地；后续远端验证沿main执行并绑定实际提交SHA，再完成四平台/Node18、同包all与36会话、补充视觉/V-10/全量门/聚合。Claude及固定HTTPS资源分别待恢复
- 当前阻塞：Claude尚未恢复，阻塞其18题；固定HTTPS测试资源最后采集超时，品牌采集未验收。7c89d08尚未运行远端认证属于待验证，不以缺少新分支归因
- 代码基线：dce8662270f993cae3521b59ceff8ac6f22e1b63

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M8 | 阻塞 | 2026-10-31T12:08:36+08:00 | a2 core/31迁移及品牌内置/比较视觉已闭合；C5异步证明/PTY回显修复与npm轻量门通过，已提交7c89d08并按用户选择保留本地。已撤销分支阻塞判断，后续CI沿main；Claude/HTTPS资源待就绪，终验范围保持。 | [M8 记录](PHOENIX-DIAGRAM-P3.records/M8.md) | 待终验收口；ec6ec88 / 321a322 / 77e3006 / 926cb18 / 9d9fe10 / 114da76 / 41ad54c / 1ed79b3 / 52e3dee / adc3d4b / c72133d / 7c89d08 |
| M1 | 已完成 | 2026-10-29T00:02:52+08:00 | 品牌/安全采集/缓存/五类标题轨及终局兼容已交付，V-1 定向退出通过。 | [M1 记录](PHOENIX-DIAGRAM-P3.records/M1.md) | d60efaf |
| M2 | 已完成 | 2026-10-29T01:41:16+08:00 | 比较/三视图/成对恢复/独立审核及公开链路交付，V-2 与静态、轻量门通过。 | [M2 记录](PHOENIX-DIAGRAM-P3.records/M2.md) | d60efaf |
| M3 | 已完成 | 2026-10-29T10:27:10+08:00 | V-3/正式引擎故障支撑、准备提交/限定分支推送及固定 PNG 匿名 HTTPS 就绪通过。 | [M3 记录](PHOENIX-DIAGRAM-P3.records/M3.md) | d60efaf / 859db51 |
| M4 | 已完成 | 2026-10-29T11:32:40+08:00 | 当前作者 run 的真实采集、导出、比较12图与两品牌8图实际审核后 complete；故障/preview视觉与维护证据闭合，保留已知 Reader 轻微限制。 | [M4 记录](PHOENIX-DIAGRAM-P3.records/M4.md) | 8f6547a |
| M5 | 已完成 | 2026-10-29T14:58:03+08:00 | v1/v2迁移与回滚、真实旧包桥接、CLI故障门及V-5整组/受影响复验组合通过；首次失败和证据边界保留。 | [M5 记录](PHOENIX-DIAGRAM-P3.records/M5.md) | b9a63ec / 665e5e8 |
| M6 | 已完成 | 2026-10-29T15:14:00+08:00 | 唯一入口/两调用方/兼容与扩展文档、五份原例及认证布局齐备；V-6为19/19，重建分发后npm测试退出0。 | [M6 记录](PHOENIX-DIAGRAM-P3.records/M6.md) | b9a63ec / 665e5e8 |
| M7 | 已完成 | 2026-10-29T16:39:56+08:00 | all/providers/31迁移场景、两宿主18题/认证CI与聚合、完整来源许可和同字节分发齐备；V-7及必需npm门通过，真实同包终验留M8。 | [M7 记录](PHOENIX-DIAGRAM-P3.records/M7.md) | b9a63ec / 673b4c5 / 665e5e8 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

类型：既有本地工具增量、文件生命周期变更和入口迁移。WORK 号尚未转成 SPMS key，沿用原号；本表唯一登记需求与约束。P1/P2 已交付 AC 在 §9 回归，不重复定义。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F7 | 功能 | PRD §3 F7、§7.2，AC-1、AC-2 | 内置品牌查询、显式官方来源采集、固定摘要 | §3.1、§5.1、§6；V-1、V-4、V-8、V-9 | 本期终验 |
| WORK-F8 | 功能 | PRD §3 F8、§7.2，AC-1、AC-2 | 显式预览、当前有效版本推进、last-good 与可靠停止 | §5.3、§6；V-3、V-4、V-8、V-9 | 本期终验 |
| WORK-F9 | 功能 | PRD §3 F9、§7.2，AC-1、AC-2 | 双 architecture 的有据差异和成对成果恢复 | §3.3、§5.2；V-2、V-4、V-8、V-9 | 本期终验 |
| WORK-F10 | 功能 | PRD §3 F10、§5.3、§7.2，AC-1–AC-3 | 唯一入口、原调用授权/fallback/语言/嵌入验收、十类描述与许可 | §0.6、§5.5；V-6、V-7、V-9、V-10 | 本期终验 |
| WORK-F11 | 功能 | PRD §3 F11、§7.2，AC-3、AC-4；§7.3；C-6 | 确认后直接移除旧 skill、不备份；新入口安装中断恢复/回滚；合并 P1 AC-1、AC-2、AC-5 终验 | §3.2、§5.4；V-5、V-8、V-9、V-11；旧入口验收按 C-6 | 政策已确认；终验 P3 |
| WORK-N2 | 可维护性 | PRD §3 N2、§7.2，AC-3；§7.3 | 固定来源、Phoenix 修改、完整许可、匹配身份；合并 P1/P2 原生核心证据 | §4、§7；V-7、V-8、V-11 | 本期整条终验 |
| WORK-N3 | 兼容性 | PRD §3 N3、§7.2，AC-3、AC-4；§7.3 | 品牌/来源/废弃展示字段终局处置；扩展合同；旧 JSON/HTML 保留 | §3.1、§5.5；V-1、V-6、V-8、V-9 | 本期整条终验 |
| WORK-N4 | 可靠性 | PRD §3 N4、§7.2，AC-3；§7.3；C-6 | 比较/预览/导出失败与取消可恢复，新入口安装条件恢复；旧 skill 移除不回滚，后来修改不覆盖 | §3、§8；V-2–V-5、V-8、V-11 | 本期整条终验 |
| WORK-N5 | 安全性 | PRD §3 N5、§7.2，AC-2；§7.3 | 正常作图零网络，外部访问显式且固定身份，公开信息不泄密 | §5.1、§5.3、§7；V-1、V-4、V-8、V-9 | 本期整条终验 |
| C-1 | 技术约束 | PRD §4、D1–D5 | Rust 核心、npm-only、现有四平台；无旧命令 shim、Node renderer 或新设计 hook | §0.2、§1.2、§4；V-7、V-8、V-11 | 已确认 |
| C-2 | 验证约束 | AGENTS.md「Working in This Repository」 | skills/ 为真值，源码同步 bundle/dist；轻量、原生、真实平台/provider 验证分开 | §9–§10；V-7、V-8、V-11 | 已确认 |
| C-3 | 接口约束 | AGENTS.md §6/§7；PRD §5.3 | 描述边界前置并查前 100 字；借鉴先列清单；同步接口复述，保留历史与许可 | §0.6、§5.5；V-6、V-7 | 已确认 |
| C-4 | 前置约束 | [P2 覆盖复审](PHOENIX-DIAGRAM-P2.prd-dev-gaps.2026-10-28.md) §1、G-07/G-08；[gaps 快照](PHOENIX-DIAGRAM-P2-gaps.md#恢复快照) | 十类核心、兼容、四平台已有候选证据；Claude 5 项失败的处置仅限 P2 M4，人员图例指引延期 | §0.5、§4、§9.3；V-8、V-9 | 按已确认限制可用 |
| C-5 | 范围约束 | PRD §2.2、§7；用户 2026-10-28 先合并 main | 基于合并后 main，默认沿当前分支开发和构建，仅在临时项目验收；不要求新建分支，不改活跃/个人安装、不发布、不接管外部管理入口 | §0.4、§3.2、§10；V-5、V-8 | 已确认 |
| C-6 | 迁移政策约束 | 用户 2026-10-28 确认；替代 PRD WORK-F11/AC-3 的旧入口备份要求 | 展示所选项目/provider 的旧 skill，用户明确确认后直接移除，不备份、不承诺自动恢复旧 skill；恢复旧入口须重装。自定义/未知来源/外部管理冲突、个人范围及后来修改保护不变 | ADR-2、§3.2、§5.4；V-5、V-8、V-9；通过仅代表替代口径，不声称满足原备份条款 | 已确认 |
| A-1 | 归属假设 | PRD F11/AC-3；`tools/phoenix-diagram/src/install/plan.rs`；`bin/xgent-skills.js` | 无历史回执的旧入口不转成受管归属；改为按 C-6 识别、确认后直接移除 | ADR-2、§3.2；V-5 | 已解除，转入 C-6 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 运行边界 | Rust 增加 brands/compare/preview/migrate，Node 仍只管固定启动和获取 | 不把旧 Node 引擎放回普通路径；网络只由显式 capture 使用 | C-1；`tools/phoenix-diagram/src/cli.rs`、`tools/phoenix-diagram/src/delivery/prepare.rs` |
| D2 | 版本身份 | 沿用当前输入/交付身份机制，新命令分 kind/command，安装状态单独 v2 | 现有 0.2.0 引擎系列以 sourceInputSha256、bundle、二进制、VERSION 区分候选；不预定公开 npm 版本 | `skills/phoenix-diagram/ENGINE.json`、`tools/phoenix-diagram/src/install/state.rs`、bootstrap |
| D3 | 品牌解析 | 显式 capture；编译只读固定目录和本地缓存 | 离线确定性，缓存缺失/变更失败；代价是 JSON 跨机需携带资源 | WORK-F7、WORK-N5；ADR-1 |
| D4 | 比较输出 | 双输入各自编译检查，专用 IR/HTML 与成对 journal | 不把比较伪装成普通单图，也不以两个 rename 充当成对成功 | WORK-F9；冻结 comparator、`tools/phoenix-diagram/src/output.rs` |
| D5 | 预览 | 前台 loopback、原生轮询、单 worker、最新待处理槽 | 无 daemon/新 watcher 框架；会话 shell 与正式 HTML 分离 | WORK-F8；冻结 preview.mjs |
| D6 | 迁移 | 用户确认后直接移除旧 skill、不备份；journal 记录实际删除，新入口事务 active 最后提交 | 旧 skill 需要重装；新入口受管文件继续条件恢复/回滚，不把旧目录认领为受管文件 | C-6、WORK-F11；`tools/phoenix-diagram/src/install/disk.rs`、`tools/phoenix-diagram/src/install/state.rs`、`tools/phoenix-diagram/src/install/transaction.rs` |
| D7 | 验收 | 两阶段各一次收口；最终同包、四平台、本机视觉、两宿主定界题 | 不照搬 P2 过程、不改判残余失败；仅受影响证据失效 | C-2、C-4；§9–§10 |

### 0.3 ADR-lite

**ADR-1 · 普通编译只消费已固定的本地品牌资源。** 状态 Accepted（设计选择，不授予自动采集权限）。候选 A 是每次生成重新访问网站再核摘要，省去缓存携带，但普通生成依赖远端并增加授权/离线成本；候选 B 是显式 capture 固定资源后只读消费，网站变化不影响已固定成果，但跨机需同步缓存。选择 B，保留冻结参考的 URL 安全、摘要、来源和内嵌规则，拒绝 prepareDiagramBrandMarks 在编译中自动访问远程。出现已确认的跨项目分发需求时再评估显式资源打包，不预建资产服务。

**ADR-2 · 确认后移除旧 skill。** 状态 Accepted（用户 2026-10-28 确认，见 C-6）。选择 C：完整来源匹配只识别可处理的原版目录，不推定所有权；展示精确项目/provider/目录及“不备份、旧入口需重装”后，用户确认才直接移除。候选 A 的认领、备份及旧目录回滚增加了用户明确不需要的恢复机制；候选 B 只认历史回执，使现有无回执副本不能迁移，均不采用。代价是删除后的旧 skill 不由安装恢复或回滚重建；新入口的受管写入、后来修改与无关配置继续受原事务保护。自定义/未知来源/外部管理冲突和个人范围不因本决定放宽。若以后明确要求恢复旧 skill 的确切字节，再评估备份政策，不从当前 journal 虚构恢复能力。

### 0.4 职责与事实所有权

| 责任 | 作者/宿主 | Phoenix Diagram | 外部依赖/维护者 |
| --- | --- | --- | --- |
| 拥有 | 品牌与官方 URL、两份材料、是否采集/预览/迁移的明确请求 | 资源/输入核对、确定性生成、版本门、受管事务、回执和条件恢复 | 网站返回字节；provider 原生会话；GitHub 构建/认证制品 |
| 不拥有 | 不从品牌/布局补业务事实，不以自动检查代看图 | 不发现未授权网站、不代建服务、不接管个人/外部管理/Phoenix UI | 本期不改造网站、provider、Actions 平台或 UI 产品 |

### 0.5 明确不在本期

- PRD 排除的 infographic、Windows、移动编辑、云端品牌库、其他图型比较，另立范围。
- 旧 HTML 不批量改写；旧 Story/Guided Views/普通 Share Card 不重建，逐字段交代移除和现有 Reader 替代。
- G-08 人员 workflow 图例指引保持延期，不借入口迁移加人员类别或改变图例。G-07 原失败保留，P3 新同包实际结果独立判断。
- 只开发迁移能力并在临时项目验收，不实际迁移用户活跃/个人安装；外部管理旧入口由原管理器处理。
- 不更新 SPMS、不公开发布，不把候选通过当作上线。

### 0.6 参考整合审阅清单

清单先于 M6 的目标 skill 修改；固定来源是 `tools/phoenix-diagram/upstream/SOURCE.json` 的 3.1.0 归档，相关本地副本已与归档逐文件比对。M6 对照 diff 核查，新行为差异先更新本表。

| 目标文件/规则 | 当前行为 | 已读来源/段落 | 采用/调整/拒绝 | 原则、调用/授权、接口与验收影响；验证 |
| --- | --- | --- | --- | --- |
| phoenix-diagram/SKILL.md 描述/路由 | 十类、排除数值图表/仪表盘/未知 infographic | PRD C1；冻结 SKILL Type router/Mermaid input；现行选择图型 | 保留十类与排除，补精确品牌/预览/比较入口 | 不改事实、语言、停止阈值；完整描述/前 100 字 V-6 |
| 品牌参考 ★ | brand 标为 P3 deferred | 冻结 brand-marks.md；findBrandMark/captureBrandReference/prepareDiagramBrandMarks | 采用固定目录/安全/来源，调整为显式采集+本地消费，拒绝生成中联网 | 不猜品牌/URL，不替代 type；V-1、V-9 |
| 预览参考 ★ | 不自动启动服务/窗口 | 冻结 preview.mjs generation/last-good/stop | 采用版本与恢复；原生轮询；--open 才开窗口 | 仅明确请求启动，不充当视觉验收；V-3、V-4 |
| 迁移帮助/条件参考 ★ | 普通安装/作图不移除旧入口，尚无公开迁移路由 | 现行 install/bootstrap 与 §3.2/§5.4 的原生 migrate 合同 | 补 migration.md 与显式维护请求路由，优先使用真实回执给出的固定引擎恢复命令 | 沿用 C-6 确认、不备份/不恢复旧 skill、精确项目/provider 及后来修改保护，不新增自动迁移或安装授权；V-5、V-6、V-9 |
| 比较参考 ★、delivery-contract | M2 已提供比较命令及专用审核参考，主 skill 尚无条件路由 | 冻结 canonicalArchitecture/compareArchitecture/buildDeltaSvg/renderArchitectureDeltaHtml；M2 现行 architecture-compare/delivery-contract | 保留已实现的稳定身份/字段分类/三视图及成对事务，M6 补明确请求路由 | 无因果/风险/可合并性推断；保留默认实际审核，V-2、V-4、V-6 |
| 兼容回执/交付答复 | brand/views/旧 animation 均 deferred phase=P3 | `tools/phoenix-diagram/src/input.rs`::compatibility；PRD C5/NG5；冻结 Reader | 品牌实现，来源沿 P2，废弃展示改终局 dispositions | 不声称复刻移除展示，不改原 JSON；V-1、V-8 |
| elintp/SKILL.md 出图/嵌入 | archify 可用则调用，缺失用内联 SVG/HTML/CSS，最终离线 srcdoc 检查 | 现行出图/自检；PRD C7 | 仅换入口、能力映射和回执 | 原触发/不自动安装/语言/fallback/最终嵌入检查保留；V-6、V-9、V-10 |
| documentation-writer 及 architecture/user-manual 参考 | archify/Mermaid fallback，三类成果，旧截图命名与证据清理 | 现行出图/交付；现行 delivery-contract | finalize→实际看图→review-record→同交付 summary，从真实回执取截图 | 原 fallback/语言/成果数量保留；额外证据置本机 out-dir 至交接，不删身份链冒充通过；V-6、V-10 |
| README、旧源码、许可、扩展约定 ★ | 新旧并列，旧 Node 引擎活跃 | PRD C6/C8；SOURCE.json/许可链 | 删除旧活跃入口，保留冻结历史/第三方文本，补扩展约定 | 无旧命令 shim，不改 UI，不新建原本没有的 agents/openai.yaml；V-6–V-8 |

## 1. 当前事实与改动面

### 1.1 现状与缺口

- [已核实·足够] `tools/phoenix-diagram/src/cli.rs`、`tools/phoenix-diagram/src/delivery/prepare.rs` 已有十类输入/来源/布局/产物检查和独立视觉门；`tools/phoenix-diagram/src/output.rs` 的 FileBinding/OutputSlot 保护物理身份、输入和后来输出。
- [已核实·足够] `tools/phoenix-diagram/src/install/plan.rs` 包含 19 个 provider 名称；codex 选择 .agents/.codex、skill 只写 .agents，空项目默认 .claude；未知文件/链接和受管修改保持冲突。bootstrap 核对包/引擎/bundle/所有权。
- [已核实·缺口] cli 无 brands/preview/compare/migrate，`tools/phoenix-diagram/src/input.rs` 的品牌/展示仍延期；单文件 output 无成对提交，安装无旧目录移除或公开回滚 → §3/§5 的最小原生增量，否则 P3 不能交付。
- [已核实·缺口] README/调用方仍用旧入口，documentation-writer 假定旧截图/清理方式 → 按 §0.6 同步，不改调用方立场。
- [已核实·缺口] legacy-2.17 的 70 文件表只覆盖 oracle 闭包；当前活跃旧目录有 219 个文件 → 删除前另存完整迁移来源，不能用部分清单授权整目录。
- [已核实·缺口] `tools/phoenix-diagram/src/install/plan.rs:199`–225 保留无归属或后来替换的文件，`state.rs:23`–32 的受管范围只含新 skill；`bin/xgent-skills.js:62`–164 的旧工具清理针对 impeccable，没有 archify 所有权记录。按 C-6 新增确认后移除，不把内容一致转成所有权。

### 1.2 拓扑与文件清单

★ 为新增。所有路径相对仓根；归档只用于来源/测试，不回到普通执行路径。

| 切片 | 改动落点与同构实现 |
| --- | --- |
| 品牌/兼容 | `tools/phoenix-diagram/src/brands.rs` ★、`tools/phoenix-diagram/src/brands/capture.rs`、`tools/phoenix-diagram/src/brands/cache.rs`、`tools/phoenix-diagram/src/brands/network.rs` ★；`tools/phoenix-diagram/src/input.rs`、`tools/phoenix-diagram/src/scene.rs`、`tools/phoenix-diagram/src/render.rs`、`tools/phoenix-diagram/src/delivery/prepare.rs`、`tools/phoenix-diagram/src/delivery/receipts.rs`、原五类 layout 的容量/标题轨；`tools/phoenix-diagram/assets/brand-marks.json` ★、Cargo.toml/Cargo.lock/build.rs；照 evidence 绑定与冻结品牌解析 |
| 比较/恢复 | `tools/phoenix-diagram/src/compare.rs` ★、`tools/phoenix-diagram/src/compare/canonical.rs`、`tools/phoenix-diagram/src/compare/render.rs`、`tools/phoenix-diagram/src/compare/transaction.rs` ★；`tools/phoenix-diagram/assets/compare-template.html` ★、`tools/phoenix-diagram/viewer/compare.js` ★；`tools/phoenix-diagram/src/cli.rs`、`tools/phoenix-diagram/src/check.rs`、`tools/phoenix-diagram/src/browser/mod.rs`、`tools/phoenix-diagram/src/browser/probe.js`、`tools/phoenix-diagram/src/browser/evidence.rs`、`tools/phoenix-diagram/src/delivery/provenance.rs`、`tools/phoenix-diagram/src/delivery/review.rs`、`tools/phoenix-diagram/src/delivery/receipts.rs`；照冻结 comparator 和 output 的描述符/恢复合同 |
| 预览 | `tools/phoenix-diagram/src/preview.rs` ★、`tools/phoenix-diagram/assets/preview-shell.html` ★、`tools/phoenix-diagram/src/cli.rs`、`tools/phoenix-diagram/src/delivery/prepare.rs`；照冻结 generation/last-good，复用原生准备/发布 |
| 安装迁移 | `tools/phoenix-diagram/src/install.rs`、`tools/phoenix-diagram/src/install/plan.rs`、`tools/phoenix-diagram/src/install/state.rs`、`tools/phoenix-diagram/src/install/transaction.rs`、`tools/phoenix-diagram/src/install/disk.rs`、`tools/phoenix-diagram/src/install/bundle.rs`、`tools/phoenix-diagram/src/install/migration.rs` ★；`tools/phoenix-diagram/migration-sources/archify-v1.json` ★、`tools/phoenix-diagram/upstream/archify-2.17-full-source.tar.gz` ★；`scripts/collect-phoenix-diagram-migration-sources.mjs` ★；照项目锁、active-last、条件补偿 |
| 身份/分发 | `skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs`、`bin/phoenix-diagram.js`、`bin/xgent-skills.js`；现有 `scripts/build-phoenix-diagram.mjs`、`scripts/check-phoenix-diagram-source.mjs`、`scripts/pack-phoenix-diagram-candidate.mjs`、`scripts/check-phoenix-diagram-candidate.mjs`、`scripts/aggregate-phoenix-diagram-candidate.mjs`、`scripts/test-phoenix-diagram-native.mjs`；`.github/workflows/phoenix-diagram.yml`；ENGINE、dist、来源/许可清单；`.gitattributes`仅对六个固定官方许可副本保留原文EOF空行 |
| 兼容消费样本 | `skills/phoenix-diagram/examples/legacy.web-app.architecture.json` ★、`skills/phoenix-diagram/examples/legacy.agent-tool-call.workflow.json` ★、`skills/phoenix-diagram/examples/legacy.async-job-roundtrip.sequence.json` ★、`skills/phoenix-diagram/examples/legacy.product-analytics.dataflow.json` ★、`skills/phoenix-diagram/examples/legacy.deployment-release.lifecycle.json` ★；M6 从固定 legacy-2.17 原例逐字节复制，M7 核 sourceInputs/bundle/tgz 与消费路径，详见 §5.5 |
| 唯一 skill/调用方 | `skills/phoenix-diagram/SKILL.md` 及现行四份参考；`skills/phoenix-diagram/references/brand-marks.md`、`skills/phoenix-diagram/references/preview.md`、`skills/phoenix-diagram/references/architecture-compare.md`、`skills/phoenix-diagram/references/migration.md`、`skills/phoenix-diagram/references/compatibility.md`、`skills/phoenix-diagram/references/extending-types.md` ★；删除 `skills/archify/`；README.md、`docs/phoenix-diagram.md`、`skills/elintp/SKILL.md`、`skills/documentation-writer/SKILL.md` 及 `skills/documentation-writer/references/architecture.md`、`skills/documentation-writer/references/user-manual.md` |
| 测试 | `tools/phoenix-diagram/tests/brands.rs`、`tools/phoenix-diagram/tests/compare.rs`、`tools/phoenix-diagram/tests/preview.rs`、`tools/phoenix-diagram/tests/migration.rs` ★、现行 input_contract/render_check/install/cli/delivery/output；`tools/phoenix-diagram/tests/p3-acceptance.mjs` ★、`tools/phoenix-diagram/tests/provider/p3-scenarios.json` ★、`tools/phoenix-diagram/tests/fixtures/brands/official-test.png` ★；`test/phoenix-diagram/migration.test.js`、`test/phoenix-diagram/p3-acceptance.test.js` ★ 及现行 bootstrap/install/candidate/aggregate/provider-acceptance/skill |

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类/适配器 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| brands | 显式 CLI、prepare | list/capture/prepare；普通 prepare 零网络 | 仅内部 DNS/请求测试接缝 | URL/IP/大小/摘要、安全缓存、内嵌 | 外部网站；ureq 生产、本地 HTTP/TLS fixture | 回执、缓存、产物字节与网络记录 |
| compare | CLI、专用 check/browser/review | 两个 CompileRequest + 双输出；全部检查后提交 | FileBinding/OutputSlot、pair journal | 稳定身份、分类、差异图、双成果恢复 | 进程内+真实文件系统 | IR、双输入绑定、失败后保留/恢复 |
| preview | 显式 CLI 前台 | run(request)；最新版本门、127.0.0.1 | 同一编译准备/发布边界 | 半写/删除/generation/last-good/停止 | 本地文件/TCP，真实回环 | HTTP 版本、正式字节、关闭端口 |
| migration | 显式 migrate | dry-run 只读；apply/recover/rollback 锁内重读 | disk 描述符、事务 observer | 原版识别/确认删除、新入口归属/条件回滚 | 本地真实文件系统 | 实际删除/余项、事务结果、后续修改/无关配置保持 |
| 调用方 | 宿主原触发 | 新 skill 可用才调用，否则原 fallback，最终文档另验 | 已有 skill 探测 | 回执路径/状态和嵌入边界 | 本地安装+真实会话 | 加载路径、工具调用、最终成果 |

入口签名固定为 brands::list(&str) -> CatalogReceipt、brands::capture(&CaptureRequest) -> CaptureReceipt、brands::prepare(&DiagramInput, &Path) -> Result<ResolvedBrands, Diagnostics>；compare::run(&CompareRequest) -> CompareReceipt、compare::recover(&Path) -> RecoveryReceipt；preview::run(&PreviewRequest) -> u8；install::migrate(&MigrationRequest)、recover_migration(project, id)、rollback(project, id, dry_run) 均返回 InstallResult<InstallReceipt>。请求结构只含 §3/§5 定义的输入、配置和输出，不引入未请求选项。

新回执沿 schemaVersion/tool/toolVersion/command/status/diagnostics 公共头：CatalogReceipt 增 catalogVersion/query/matches（每项 id/title/category/aliases/domains/provenance）；CaptureReceipt 增 brand/resource/cachePath（resource 为 §3.1 manifest 身份，失败时不返回已激活资源）；RecoveryReceipt 增 transactionId/recoveryDirectory/publicStates/preserved/status。PreviewRequest 为 kind/input/output/compileOptions/brandCache/port/open；PreviewEvent 增 event/sessionId/generation/lastSuccessfulGeneration/lastAttemptStatus/shutdownComplete（只在 stopped 给真实值）。MigrationRequest 为 mode/project/providers/bundleRoot/transactionId/dryRun/confirmRemoveArchify，确认字段仅用于 --from-archify。所有未执行阶段写 not-run，不由顶层 pass 推导视觉或兼容成功。

这些模块各自集中安全、版本或事务策略，删除会将复杂规则散回调用者。不新增通用资产服务、HTTP 框架、通用多资源事务或 provider adapter。新增公开 JSON 为 camelCase，诊断沿 stage/code/subject/evidence/supportedFixes，不改变原命令含义。

## 3. 文件模型与兼容迁移

### 3.1 品牌和旧输入

- 原五类已有 brand 字段：architecture.components、workflow.nodes、sequence.participants、dataflow.nodes、lifecycle.states；另五类的冻结 schema 没有 brand，不凭空扩字段。
- 从固定 3.1.0 generated-brand-marks.mjs 导入完整向量和 provenance 到 `tools/phoenix-diagram/assets/brand-marks.json`，核 ID/alias/domain 冲突、viewBox/颜色/SVG path；运行时不安装 simple-icons。原数据/商标/第三方说明随许可链保存。
- 远程缓存默认当前工作目录 .phoenix-diagram/brands/v1，可用 --brand-cache DIR。每个资源为单一私有 JSON envelope，文件名由 URL 身份摘要和内容 SHA 组成；字段为 schemaVersion、urlSha256、脱敏 sourceUrl/resolvedUrl、mediaType、bytes、sha256、dataBase64。用 publish_bound_with_mode 以 0600 原子发布，已有同身份只校验不覆盖；避免 blob/manifest 半套。
- brands::prepare(kind,document,cache) 返回 ResolvedBrands，按输入 JSON pointer 索引，含资源身份、公共来源和安全内嵌数据。DiagramInput 增加资源事实与 bind_brands；configSha256 绑定排序后的资源 ID/摘要/目录版本，不含机器缓存路径。CompileRequest 增可选 brand_cache；scene 节点增可选品牌矩形，render 不再读磁盘/联网。
- 兼容对象保留 status/coreCompatibility/deferredFields，增 dispositions=[{path,disposition,reason}]，disposition 为 supported/ignored-display/unsupported-display。品牌成功为 supported；meta.views 明确不重建旧 Guided Views，旧 meta.animation 不覆盖首次 Still；均不再写“延期 P3”。有展示缺口则 status=partial，核心 pass；空 deferredFields 不声称完整复现。
- 原 JSON 不重写；来源沿 P2 已支持行为。未知品牌/未固定 URL/缺缓存/摘要或类型不符均失败，不能静默省略已著品牌；旧独立 HTML 原字节保留。

### 3.2 安装 v1 → v2、旧入口和回滚

旧 skill 按 C-6 确认后直接移除；不发生旧目录所有权认领，不保存该目录的目标备份。固定历史来源归档是识别/重装依据，不是用户旧安装的备份。

1. **Expand**：native/bootstrap 同时读 diagram v1/v2，先核原 checksum、物理项目和文件归属，新 journal/active 写 v2；不能只按路径采用 v1。
2. **固定来源**：删除 `skills/archify` 前，保存 Git 固定 2.17 完整目录、当前仅增加 when_to_use 的已知变体，以及固定 3.1.0 归档的 archify 子树。表中包含完整文件集合/字节/mode；完整归档可取回。现有 70 文件 oracle 表不充当完整安装识别或目录所有权。archify-v1.json 通过 include_str! 编入原生 migration 模块，并纳入 sourceInputs；完整旧源归档只供构建/来源核对，运行期不需要 Git、旧 Node 或另取源码包。
3. **识别与确认**：显式 migrate --from-archify 只识别全匹配来源、无外部管理标记且物理快照稳定的项目副本，不要求虚构历史回执。dry-run 展示来源/provider/精确旧目录/新入口、legacyBackupPolicy=none 和旧入口重装指引，零写入。实际 apply 必须带 --confirm-remove-archify；缺确认返回 install/confirmation-required、exit 3，删除和新入口写入均不开始。该标志确认本次展示的所选范围，不授权清理其他项目或个人目录。
4. **冲突**：skills-lock.json 等已识别的外部管理归属、额外/缺失文件、自定义字节、链接、未知来源、跨设备父目录或个人安装不移除。个人范围只报告精确已知入口和指引，不递归扫个人空间。
5. **Apply**：项目锁内重读所有绑定，先准备新文件，再持久化 v2 意图、已确认范围和旧目录当前快照；不保存旧 skill 文件内容。按描述符逐项核对后直接移除旧目录，记录实际已删/剩余项，再发布新 skill，active 最后提交。原版识别、确认、路径和后来修改保护全部通过才能跨删除门；不用跟随未知路径的 rm -rf。普通 install 不自动移除 archify；联合安装新增显式 --migrate-archify，透传同一原生 dry-run/apply 与确认标志。
6. **Recover**：原 install 的 pending 恢复保持；migrate --recover ID 仅处理指定事务。已确认且仍匹配该事务快照的剩余旧文件可继续移除，不重复索取同一范围的确认；新文件和父目录继续核物理身份/摘要。未知 claimant、删除后重建或后来新增内容保留并返回 recovery-required。中断后从 durable journal 核对已删项与当前目录，不凭 stdout 推断；没有旧 skill 备份，不重建已删字节，回执给重装指引。
7. **Rollback**：仅回滚当前 committed 事务的新入口受管写入，旧 skill 删除不在逆向恢复范围。预检发现后来修改则先返回冲突、零变更；实际逆序恢复仍用条件身份。恢复旧 diagram active 语义时重算恢复文件的新物理身份/checksum，不能原样拷贝引用旧 inode 的 active。成功记录 rolled-back，并列出未恢复的旧入口及重装指引；重复同 ID 幂等，不跨过后来安装。
8. **Contract**：本期不删除 v1 reader、新入口受管写入的备份或恢复记录；旧 Node 引擎只退出活跃 skill，冻结 oracle 保留。原 JSON、旧独立 HTML 及目录外内容不因旧入口移除而被改写。外部/个人旧入口未处理时，不声称唯一加载已生效。

v2 Operation 用带标签的 file-write/legacy-root-remove，后者仅允许精确 provider/skills/archify、完整来源 manifest、确认范围及目录/文件快照，不把 state::managed 放宽到任意路径。Journal 保存 applying/committed/recovering/recovered/rolling-back/rolled-back、正向/逆向游标、删除进度和 checksum；legacy 删除只有正向进度，不创建 before 文件内容或退役目录备份。kill 发生在删除与游标写入之间时，以当前项是否缺失、余项快照及父目录身份核对，不覆盖重建目录。active 只拥有新 diagram 文件。共享 settings/hooks、.phoenix-ui 和个人文件不纳入归属。

### 3.3 比较文件

CompareReceipt（schemaVersion=1、kind=architecture-comparison、command=compare）包含 comparatorVersion/canonicalVersion、base/head 的原始/语义 SHA 与字节数、各自 config/engine/evidence、资源身份、comparisonIr 的 SHA/字节数、changes/summary、artifact 身份、receiptId、transaction/recovery、checks、limitations。公开 HTML 不嵌入输入绝对路径或仓库凭据。HTML 与原 CompareReceipt 是确定性锚点；后续浏览器检查和审核不得改写两者，或把 CompareReceipt 兼作可变完成状态。

默认 JSON 为 output.html.compare.json，须与 HTML 同物理父目录。compare 专用 0700 journal 持久保存两输出 before/after、备份和阶段；两个输出及两输入禁止同路径、符号/硬链接别名。临时文件只清理匹配本事务者，保留后来修改。

**比较专用审核合同（M2 交付）**：

- `visual-check` 的比较分支先重核原 CompareReceipt/双输入/资源/IR/HTML 并执行专用 strict check，再实测三个视图，创建独立 `output.html.compare-summary.json`；不要求已有普通 finalize summary。输出的 `evidence.summaryReceipt` 和 `evidence.browserReceipt` 均为 `{path,identity:{sha256,bytes}}`，调用方按返回路径读取，不猜 out-dir 内的文件名。`--receipt PATH` 仍限同一物理父目录；HTML 只保存安全的相对回执文件名与 receiptId 供定位，CompareReceipt 绑定 HTML 摘要，summary 再绑定两者，避免互嵌摘要的循环。审核不误用默认 compare JSON；summary/浏览器证据/观察记录禁止与输入或两份不可变成果成为同路径、符号/硬链接别名。
- summary 使用独立 ComparisonSummary（schemaVersion=1、kind=architecture-comparison-summary），包含 `comparison`、`stages`、`automatedStatus`、`visualReview`、`deliveryStatus` 与 `evidence`。`comparison` 是完整 ComparisonBinding：原 compare 回执的路径/摘要及 receiptId，base/head 各自的原文件路径/摘要/字节数、语义 SHA、configSha256、engine、资源身份，comparisonIr 的 SHA/字节数、HTML 路径/摘要。只在本地审核材料保留源路径；公开 HTML 仍脱敏。stages 仅记录本次真实的 strict `check` 与 `visual-check` 回执，不伪造四个 finalize 阶段；各门 pass/failed/unavailable/not-run 分开，automatedStatus 仅两门均 pass 才 pass。自动通过后 visualReview 为 required-pending、deliveryStatus 为 incomplete。
- 浏览器回执绑定同一 comparison，按 Before/Delta/After 分别标记 `viewId=before|delta|after`。每视图沿单图四桌面尺寸实测，采集 1440×900、2048×1320 各 light/dark，合计 12 张当前截图；每条含 viewId、viewport、theme 和原文件身份。不能用父页或一个视图的通过覆盖其余视图。
- ComparisonVisualReviewRecord 独立于单图 VisualReviewRecord，严格拒未知字段：schemaVersion=1、kind=architecture-comparison-review、command=review-record、comparison（原样复制上述 binding）、browserReceipt（路径/身份）、screenshotIdentities（上述 12 条及逐张 observed/observations）、reviewer、scope（diagram/labels/cards/relationship-occlusion/hierarchy/controls）、verdict、findings。三个 viewId 均须被实际观察；草稿 observed=false、reviewer/observations 为空、verdict=unavailable，不代写 passed。已看且无缺陷才 passed；有缺陷为 failed；无法看图为 unavailable 并写限制。
- `review-record` 按 artifactKind 和已核原回执精确分派比较合同，重读当前身份及两自动阶段，只封存该观察记录并原子更新独立 summary，返回其真实路径/身份；不重生成 HTML/CompareReceipt、不重跑 compare/visual-check。缺 summary、漏任一输入/视图/截图、单图记录、旧浏览器记录或任何绑定漂移均拒绝且不能 complete。完成须 automatedStatus=pass、12 张当前图片实际审核通过、visualReview.status=passed、deliveryStatus=complete；failed/unavailable 保持 incomplete。封存后 summary 更新失败只以同一记录字节幂等重试；新交付/新浏览器证据不继承旧审核，不覆盖已封存的不同记录。

M2 同步 `skills/phoenix-diagram/references/delivery-contract.md` 与 architecture-compare.md 的比较专节：给出下列公开顺序、从 summary.comparison/browser 回执复制上述完整草稿的 Node 18+ 示例及逐张填写说明；普通单图 finalize/VisualReviewRecord/四门完成含义不变。以下是待实现的公开合同，不声称当前二进制已支持：

```sh
"$PD" compare architecture base.json head.json comparison.html --json
"$PD" check comparison.html --strict-provenance --json
"$PD" visual-check comparison.html --out-dir compare-evidence-01 --json
# 按 evidence.summaryReceipt / evidence.browserReceipt 读取，复制完整草稿并实际观察 12 张图。
"$PD" review-record comparison.html --record comparison-review-01.json --json
# 读取返回的 summaryReceipt；核同一 comparison、两自动门、visualReview 与 deliveryStatus。
```

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| 固定参考/oracle | 已核实足够 | `tools/phoenix-diagram/upstream/SOURCE.json` 的归档/2.17 闭包；已核相关品牌/预览/比较文件与本地副本一致 | 原生移植；完整迁移来源独立登记，不改 oracle 固定摘要 | 来源/许可不完整禁止打包，不取 latest |
| 编译/发布 | 已核实足够 | prepare 捕获源、核 engine/evidence、compile/check、再次 verify；OutputSlot 保护输入及后来输出 | 品牌加入身份链；共享持有快照的准备函数；compare 专用 pair journal | 输入/目标变化冲突，旧成果保留 |
| HTTP/TLS | 已核实缺口 | diagram 无 HTTP；本仓 UI 已用 ureq 2.12.1；本机依赖源码有 Resolver、redirect/timeout 配置 | diagram 精确新增 ureq =2.12.1、default-features=false、tls；补锁和许可，不改 UI | 每跳重新验地址，受限读取；网络/安全错误分开 |
| 官方网站 | 已核实缺口 | 字节可变，固定摘要是本方合同，无需外部新增 API；未把本地 commit 或文件存在当作公网可取回 | 仅题面/用户给 URL 才采集；§9.2 的确定字节 official-test.png 在 M1–M3 准备、M3 准备提交并在授权范围推送，匿名 HTTPS 核 URL/提交/SHA 后才可进入 M4；M8 复用同一固定资源，不等 M7 | 未获远端发布授权、不可达/不安全/变更均不换图或泄凭据；先补资源就绪，阻塞相应 V-4/V-8 真实采集验收 |
| 四平台/Chrome | 已核实足够 | 当前 workflow 的四 runner、Node 25.2.1、浏览器准备和候选认证；P2 四平台真实证据 | 扩 P3 scope；最终四平台真实消费，本机 darwin-arm64 实际看图 | 缺环境/证据保持 incomplete，跨编译不替消费 |
| 两宿主 | 已核实足够（入口） | 本机 codex/claude 已安装，auth status 均 exit 0；原生 JSONL/capture-stdio 和归档器存在 | M7 记录届时实际模型/端点类别/版本和审核资源后冻结题面；M8 全新项目会话 | 登录不等于通过；套餐/配额问题单列，失败角色不改判 |
| GitHub 制品 | 已核实足够（入口） | gh auth status exit 0；现有 workflow/pack/aggregate 核 SHA、job/artifact 来源 | M7 源码提交，M8 沿当前分支（当前为 main）运行候选 CI，不发 release；执行时核实际 API 可用性 | 令牌/runner 不可用阻塞相应终验，不代建平台 |
| 文档调用方 | 已核实缺口 | 两 skill 和两参考仍用 archify、旧截图路径 | 按 §0.6 同步，保留原触发/授权/fallback | 缺工具走原 fallback，不自动安装，不借独立图检查替最终文档 |

## 5. 核心机制与命令

### 5.1 品牌

新增 brands [query] --json 与 brands capture URL [--brand-cache DIR] --json；普通 validate/deliver/finalize 增 --brand-cache DIR。

1. list 只读固定目录，返回 canonical ID/alias/category/domain/provenance；已知域名 URL 用内置向量，零网络。无匹配给空集合，不按相似品牌自动代用。
2. capture 才可访问 HTTP(S)。拒绝 userinfo、非标准端口、私有/回环/链路本地/保留地址及 IPv4 映射绕过；校验全部 DNS 结果，把同一结果交给 ureq Resolver 固定连接 IP，保留原 hostname/SNI。禁系统代理/自动 redirect，不提供生产 ALLOW_PRIVATE。
3. 最多 3 跳手动 redirect，每跳重验 URL/DNS/IP。只读用户站点的 head 图标链接及 favicon，不执行页面脚本，不发现其他品牌网站。沿冻结参考：总截止 8000ms、HTML 256KiB、图像 1MiB、并发至多 3。
4. DNS 用剩余期限的独立解析任务及限时接收，不能误认 ureq timeout 包含慢 DNS；采集进程退出不留常驻线程池。只收 MIME/真实签名/边界匹配的 PNG/JPEG/WebP/ICO，拒绝 SVG、HTML 冒图、非 identity 内容编码、大图和畸形 ICO。
5. 成功返回 brand={url,sha256}、脱敏来源、缓存位置和资源身份。普通 compile 只读并重核缓存，解析结果随 configSha256 绑定；HTML/导出只含 data URI 或可信内置 path。
6. 品牌在节点标题轨，标签、职责、type 语义标识保留。自动布局计入图标容量，固定几何不够则诊断，不改 pin/缩字。metadata 里的节点 brand 同样脱敏，不让原 URL query/fragment、缓存路径或会话信息经 facts 重新泄露。

错误在 brands 模块登记：brand/unknown、brand/unpinned-url、brand/cache-missing、brand/digest-mismatch、brand/blocked、brand/capture-unavailable、brand/invalid-image。业务输入失败 exit 1，参数 2，归属/并发冲突 3，I/O 4，资源/网络不可获得 5；既有 bootstrap 失败保持原语义。

### 5.2 架构比较

入口：compare architecture BASE HEAD OUTPUT [--receipt PATH] [--base-contract legacy-2.17|phoenix-v1] [--head-contract legacy-2.17|phoenix-v1] [--quality standard|showcase] [--repo-root DIR] [--brand-cache DIR] --json；恢复为 recover-compare PRIVATE_DIRECTORY --json。

- CompareRequest 持两份分别选合同的 CompileRequest 和双输出。先稳定捕获双输入，双输出锁按路径稳定顺序获取；共享 prepare 的快照函数，不二次读取冒充同版。两侧各自通过 schema/语义/evidence/layout/render/check 后计算 IR，提交前再核源绑定。
- 采用冻结 canonicalArchitecture/compareArchitecture：组件按 id，连接必须有 authored id，边界 kind+label 唯一键，至少一个共同组件 ID；两侧都给仓库时须同一身份。缺连接 ID 只比较拒绝，不收紧普通 architecture 合同。
- 对象键排序，组件/连接按稳定 ID，sources 按规范 JSON，wraps 排序，忽略 meta.output；字段组照冻结 COMPONENT_FIELDS/CONNECTION_FIELDS/BOUNDARY_FIELDS。分类 semantic/evidence/geometry/topology/scope，状态 added/removed/changed/evidence-changed/moved/rerouted/geometry-changed/same；presentationChanged、provenanceChanged 单列。不把数组排列/输出位置推断成业务变化。
- proofLevel 仅两侧来源 verified、同仓且固定提交时为 revision-pinned，否则 authored；不推断因果、风险、运行影响或可合并性。
- 按 §3.3 准备全部字节，先 journal 后逐文件 publish_bound，回读双成果匹配后 committed/返回 pass。短暂单文件可见不等于成对成功。提交失败恢复旧对；recover-compare 对 applying 回滚，对 committed 只清理自有临时物；后来的修改/父目录变化/篡改备份保留并报 recovery-required，重复恢复幂等。
- HTML 增独立 artifactKind。check/browser-check/visual-check/review-record 精确分派比较检查器，核双快照原检查、IR/双输入/双文件摘要、差异标记全集、命名空间和安全内嵌；不能把普通 single_svg 套到父比较页，或把基础检查称 showcase。
- compare 成功只表示确定性比较和成对提交；浏览器 not-run、视觉 required-pending 分列。默认交付按 §3.3 的公开链路，由 visual-check 建比较专用 summary，实际看三个视图的 12 张图，再由 review-record 封存记录/更新 summary；不能改原 compare JSON 或复用普通单图审核。最终从同一 summary 取得 complete；变更任一输入/配置/引擎/资源/IR/产物或浏览器证据会使对应审核失效。

### 5.3 显式本地预览

入口：preview TYPE INPUT OUTPUT [compile options] [--brand-cache DIR] [--port 0..65535] [--open] --json。仅新命令为前台 NDJSON 流：started/checking/updated/failed/stopped，含 sessionId/generation/lastSuccessfulGeneration；普通命令仍是原单回执接口。

- 固定 127.0.0.1，默认端口 0；--open 才开系统浏览器，否则返回 URL。单 worker、一个最新待处理槽；轮询 800ms、稳定窗口 400ms，取冻结参考默认。内容 SHA/删除状态产生 generation，同字节不重复推进。
- controller 持 FileBinding，向本次同引擎 worker 的 stdin 发送 schemaVersion=1 的私有 envelope（generation、原始输入 bytes、合同/配置、repo-root、brand-cache、engine 身份）；worker 调共享 prepare_bytes 管线，仅返回 generation/inputSha256/ValidationReceipt/HTML 字节，不写正式输出。controller 核 worker 退出和摘要，保留原输入身份再提交；发布前再验 generation 与源绑定。半写/非法/删除/被替代结果不覆盖 last-good。成功发布后 HTTP 才引用相同字节；初次无成功图为 waiting/error，不伪造空成功；不自动创建浏览器/视觉通过记录。
- 固定 GET/HEAD shell/status/artifact 路由，随机会话路径令牌，Host 必须匹配监听地址/端口，拒绝跨源 Origin，不开 CORS，不提供任意文件服务；请求头上限 8KiB、连接读取截止。shell iframe 显示 last-good，错误用 textContent，no-store/no-referrer；正式 HTML/导出不加入源路径、端口、令牌或错误栈。
- Ctrl+C/SIGTERM 停止接受与排队并结束 worker/连接，监听确已关闭才输出 stopped/shutdownComplete；沿参考 3000ms grace、随后 750ms 强制停止本次编译子任务。为可取消性，preview controller 调起同引擎的私有 compile worker 子进程，只传绑定快照/配置，不调用 Node；worker 结果仍受版本/输入门。SIGKILL 后 OS 释放端口，未清理发布记录按原入口恢复，不杀未知 PID。
- 输出被后来替换则停止发布并报告冲突，不能以内存旧图当正式文件的新证据；停止保留最后成功文件。

### 5.4 安装、迁移和回滚

入口：migrate --from-archify --project DIR --bundle-root DIR [--providers claude,codex] [--confirm-remove-archify] [--dry-run] --json；migrate --recover ID --project DIR --json；migrate --rollback ID --project DIR [--dry-run] --json。三者互斥，provider 默认沿 plan.rs；确认标志仅用于 from-archify，实际 apply 必需、dry-run 不必需。恢复/回滚只用已固定正确引擎和本地事务，不自动下载或重装旧 skill。

bootstrap 的 migrate 分支先严格解析操作互斥、--project 和标志，再按目标项目执行固定 VERSION/bundle/engine-probe 预检；与现有 install 一样使用 checkReceipt=false，使损坏/pending 的安装可到达原生恢复入口，普通命令仍核 active。仅 --from-archify 自动补包内 bundle-root；recover/rollback 不要求当前 active 已完整提交，不因此放宽 native 的项目锁、事务 checksum 或物理归属检查。引擎仍须已存在，缺失只给固定包的显式安装指引。

InstallReceipt 保留现有字段，增 transactionSchemaVersion/migrationSources/removals/preserved/rollbackCommand、legacyBackupPolicy=none、legacyRestoration=not-supported、reinstallInstructions；removals 逐项区分 planned/removed/preserved/remaining，rollbackCommand 只表示新入口回滚。现有 backup 字段仅指 diagram 受管文件，不充当旧 skill 备份。状态增 rolled-back、cancelled（仅真实取消），recovery-required 保留。dry-run 遇 pending 只报告，不恢复；首次写后取消不能说零变更，必须列真实事务、已删/剩余项和重装指引。

联合 --migrate-archify 使用独立的 Diagram 迁移分支，跳过 XGENT hooks/settings/AGENTS 和 Phoenix UI 阶段，以满足迁移入口的共享配置零变化边界；普通联合 install 保持原有职责。该分支先展示同一原生 dry-run 的精确目标及不备份后果；TTY 用户肯定答复后透传 --confirm-remove-archify，拒绝/EOF 时保留旧目录且不启动迁移。非交互调用须显式提供该确认标志，不能以超时或默认值代替。单独确认标志没有迁移请求时为参数错误；--no-phoenix-diagram 继续跳过并保留，不因确认标志删除旧入口。普通 install 不迁移；任何 Diagram 迁移入口都不写 UI、共享配置、其他 provider 或个人目录。

### 5.5 唯一入口和扩展约定

- 先固定完整迁移来源，并交付下述五份包内兼容样本及消费者路径，再删 `skills/archify`；README、当前调用方和包内列表只指新入口。upstream/oracle/迁移来源/许可及历史 PRD/plan/记录保留 Archify，不全仓字符串替换，不留旧命令 shim。
- M6 把 `tools/phoenix-diagram/upstream/legacy-2.17/examples/` 的五份固定原例复制到唯一新 skill 的 examples 平层，文件名只加 `legacy.` 前缀，不规范化 JSON/换行或重写字段。它们是输入数据，不含 SKILL.md、Node 引擎或旧命令入口；不是把完整 oracle/旧来源归档装进运行包。固定字节如下：

| legacy-2.17 源文件 | `skills/phoenix-diagram/examples/` 内目标文件 | SHA256（原字节） |
| --- | --- | --- |
| web-app.architecture.json | legacy.web-app.architecture.json | 483350f5297df682aba4e4a0fa491307ce3d3abd725ae4df07fab177490752cf |
| agent-tool-call.workflow.json | legacy.agent-tool-call.workflow.json | 4d3f6c23de2ca7d306e05d0041a9506833bb5a164a2bddc2d1984cf4da3ee90c |
| async-job-roundtrip.sequence.json | legacy.async-job-roundtrip.sequence.json | 545f20920c89b69c2bb1f15508d5543fddbeb15cffba61e102206e04bd5a5cd9 |
| product-analytics.dataflow.json | legacy.product-analytics.dataflow.json | 91411404fb4828a3b7ea358f3a7b9877781c9e9dc9c04b42b6eb4a757281b59f |
| deployment-release.lifecycle.json | legacy.deployment-release.lifecycle.json | 735c2c5819266e6ea2a71775b6eefce98b53030ce90cb721dc8e1d022ce5be71 |

- M6 同步 `check-phoenix-diagram-candidate.mjs` 的必需成员和 caseManifest：历史候选按五个 `skills/archify/examples/` 原路径，新候选按上表五个新路径。选择依据是认证 tgz 自身完整布局及来源身份，不读当前工作树补文件、不以 profile 回退绕过缺项；新旧混合、缺一份、摘要变化或未知布局均拒绝。P3 必须新布局且包内无旧活跃入口，仍消费全部十类 phoenix-v1 和五类 legacy-2.17，不减 case。
- 新路径已落在 package.json 的 `skills` 及构建/native 安装器的平层 `examples/*.json` 允许边界内，无须放宽任意目录。M6 同步相关清单/断言；M7 将每份纳入 sourceInputs、bundle manifest 和最终 tgz，核仓内源→新 skill→bundle→tgz→caseManifest 的五组同字节关系及完整许可。历史候选仍验证原 tgz/原 sourceInputs，禁止用重打包或新引擎冒充旧身份；测试同时覆盖旧、新及拒绝混合/缺失/篡改，不保留旧活跃 skill 来规避失败。
- 新描述十类/主要 Mermaid 触发和数值图表/仪表盘/其他 infographic 排除前置，核前 100 字；普通作图、capture、preview、compare 的授权分别说明，事实/语言/停止 2 阈值保持，不新增原本没有的 agents/openai.yaml。
- description 采用：“用于架构、工作流、时序、数据流、状态、ER、类、树、时间线、耗时瀑布图及 Mermaid 转换；不用于数值图表、仪表盘或其他 infographic。缺必需事实时仅提必要问题；齐备后交付经 Rust、浏览器与实际看图核对的独立 HTML。品牌采集、本地预览和架构比较仅在明确请求时启用。”
- elintp 保留列表/同级/项目/个人探测、内联 SVG/HTML/CSS fallback、离线 srcdoc 与最终 HTML 检查。documentation-writer 保留先判断是否需图、Mermaid fallback、文档同语言及 diagrams 源/成果。
- documentation-writer 从真实 summary 截图条目取亮色正文图，不假定旧命名；先实际审核并 review-record，再复制正文图。仓库仍只留源 JSON、HTML、正文截图，额外身份/审核证据放本机 out-dir 至交接，说明同步限制；不能删除当前声明所依赖的证据。独立图检查不替最终 Markdown/嵌入文档检查。
- extending-types.md 定义新类型同时具备输入/schema/语义/guide/示例/layout/render/check、适用 Reader graph/导出/i18n、CLI/skill/来源分发与旧行为回归；既有十类/命令/独立 HTML 有效，未知类型继续拒绝。

## 6. 前端与交互

品牌只补身份，暗亮两主题保留职责标签；详情可查公共来源/摘要。缺资源诊断提供显式补齐的动作，工具不擅自删品牌。

预览 shell 在画布外显示最新尝试错误与“仍显示第 N 个成功版本”；会话信息与正式 HTML 分离。连接断开不是视觉通过。

比较页采用 Before/Delta/After、变化列表和逐项定位，默认静态 Delta。Rust 从结构化图元直接组装：head 当前层，base 的 removed/moved-from 节点/关系/边界为幽灵层；base/head/delta SVG ID/defs 命名空间隔离，不依赖旧 renderer 注释字符串插入。Before/After 以 srcdoc 内嵌完整 HTML，变化详情显示字段前后值，不丢未变事实。

不采用参考的宣传标题、写死英文、720/820 最小宽度或内部横向滚动。标题取输入，UI 沿现有五语言/主题回退；四桌面与 390×844 下完整可达，必要时声明页面纵向阅读，不裁剪/缩字伪通过。tab、变化行、关闭/主题有键盘与焦点/选中状态，空集合明确。实际审核覆盖三个视图和变化定位；预览与正式 HTML 分别验证。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N2 | 现有 SOURCE/ENGINE/固定资源链 | Rust、固定来源/完整许可、确切资源与引擎 | 来源/许可/清单不符拒绝打包或激活 | sourceInputs 纳新源码/资源/迁移来源，完整许可随包 | V-7、V-8、V-11 |
| WORK-N3 | 两套输入合同、P2 兼容证据 | 品牌/来源/展示终局说明，旧文件保留 | 缺支持事实不算兼容通过 | dispositions、冻结 JSON/HTML 对照 | V-1、V-8、V-9 |
| WORK-N4 | 描述符发布、项目锁；C-6 | 成对提交、latest-only、新入口条件恢复/回滚；旧 skill 确认后删除不回滚 | recovery-required 列实际保留资源和旧 skill 已删/剩余项 | §3、§5 的意图先落盘、active-last、版本门及删除进度 | V-2–V-5、V-8、V-11 |
| WORK-N5 | 本地处理/显式外部访问 | 无上传/跟踪/隐式发现/自动更新、无凭据泄露 | blocked/missing/unavailable 分列，不换字节 | 固定 DNS/IP、受限读取、缓存只读、公开脱敏、loopback | V-1、V-4、V-8、V-9 |

不编造吞吐/p95/SLA/RPO/RTO。单 worker 和最新待处理槽限制预览积压，品牌大小/截止取固定参考，实际延迟由收口记录。观测沿 CLI JSON/NDJSON、delivery summary、浏览器/审核回执、私有 journal，不加遥测服务。公开图/来源不含凭据、缓存路径或会话令牌。

## 8. 失败模式、发布与回滚

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现/降级 | 检测 | 恢复/补偿 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| DNS/redirect 私网、超时/大图/畸形图 | 一次 capture | 不激活缓存 | blocked/unavailable/invalid-image，原图保留 | diagnostic/连接记录 | 用户修正 URL 或显式补固定资源，不换字节 | V-1、V-4、V-8 |
| 缓存缺失/篡改 | 当前输入 | 旧成果不变 | 精确资源/摘要错误 | prepare/config 摘要 | 显式重采相同身份，新字节须作者采用 | V-1、V-8 |
| 比较第二文件失败/kill | 一对成果 | pending 或恢复旧对 | 不报告 pass，列备份/保留位置 | journal/双身份回读 | recover-compare 条件恢复，保留后来修改 | V-2、V-4、V-8 |
| 半写/删除/旧 worker 后完成 | 当前预览 | last-good 不变 | error+最近成功版本 | NDJSON/HTTP/产物 SHA | 合法新版本推进，同字节不推进 | V-3、V-4 |
| 预览停止/后来输出占用 | 当前会话/目标 | 无关文件保留 | stopped 或冲突 | 端口/OutputSlot | 只关自有 listener/worker，走原发布恢复 | V-3、V-4、V-8 |
| 迁移/回滚中断、旧目录重建或新入口备份变化 | 所选项目/provider | 旧 skill 可能已部分/全部删除且无备份；新入口仅匹配归属可恢复 | conflict/recovery-required/事务 ID、实际已删/剩余项 | v2 journal/物理身份/删除进度 | 显式继续已确认删除或回滚新入口，不清未知 claimant；旧 skill 按指引重装 | V-5、V-8、V-11 |
| 旧入口遮蔽/未重载 | 当前 provider | 不删个人/外部入口 | 报实际加载路径和冲突 | 全新会话/安装回执 | 事务完成后重载，个人由维护者处理 | V-6、V-9、V-10 |
| 混入旧审核/旧包 | 当前结论 | 文件保留，证据失效 | incomplete | 输入/资源/引擎/HTML/截图绑定 | 只补实际失效范围 | V-7–V-11 |

顺序：M4 核心阶段提交 → M5/M6 迁移/调用方 → M7 完整源码冻结提交 → M8 四平台、唯一候选包、真实消费和终验。无公开 release/registry 上传。

弃用失败候选时保留旧候选和真实失败记录；临时项目按事务验证回滚。源码回退不删用户内容，安装回滚依 §3.2。浏览器/视觉失败不否认已真实提交的确定性成果，也不能借旧审核说当前完整通过。

## 9. 验证

### 9.1 检查表与执行时点

Node 命令在仓根通过 mise exec --no-deps -- 执行，固定 Node 25.2.1；Rust 命令工作目录 `tools/phoenix-diagram`，通过 mise exec --no-deps -- mbx 执行。表中的新增测试/验收入口均为 §1.2 的正式维护交付，不是一次性留证脚本。局部组仅在整批实现和测试齐备后跑；批内只为具体失败/技术疑问使用单个测试过滤器或 cargo check，不逐次编辑重跑整组。

| 检查 ID | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | WORK-F7/AC-1、AC-2；WORK-N3/AC-3；WORK-N5/AC-2：目录匹配、五类布局/内嵌、资源与展示处置、缓存缺失/篡改/未知品牌、DNS/redirect/类型边界、普通生成零网络；旧无品牌图回归 | mbx test --locked --offline --test brands --test input_contract --test render_check --test architecture --test workflow --test remaining_types；HTTP/TLS fixture、内部 resolver/dial 测试接缝；禁止生产私网开关 | M1：品牌/缓存/布局/回执和全部定向测试齐备后；地址/解析安全门在 capture 合入前 |
| V-2 | WORK-F9 两 AC、WORK-N4/AC-3：字段分类/稳定身份/无变更/双输入来源、HTML/JSON 绑定；第二提交失败和后来 claimant 保留；按 §3.3 独立 summary 收口、不重生成原比较，缺输入/视图、旧审核、任一身份漂移拒绝；单图原合同不变 | mbx test --locked --offline --test compare --test output --test delivery；真实临时文件、事务 observer；公开 review-record/summary 与封存后重试测试，不在局部跑整套 SIGKILL 矩阵 | M2：比较 IR/三视图/审核 summary 与公开说明/检查分派/成对恢复交付齐备后 |
| V-3 | WORK-F8 两 AC、WORK-N4/AC-3、WORK-N5/AC-2：半写/删除/乱序/同字节、源/输出替换、Host/Origin/路径拒绝、信号停止后端口关闭，正式 HTML 不含会话信息；author-tools 的状态/审核恢复支撑可用 | mbx test --locked --offline --test preview；node --test `test/phoenix-diagram/p3-acceptance.test.js`（届时阶段 A 部分）；真实 loopback、测试进程与临时输入，端口由 OS 分配 | M3：controller/worker/shell/停止、author-tools 脚本/审核恢复/故障支撑和定向测试齐备后；真实资源按 §9.2 另核就绪 |
| V-4 | 阶段 A 收口：WORK-F7/F8/F9 全部 AC；真实品牌离线导出、预览错误恢复、比较三个视图的 12 张图实际审核及同一 summary complete；比较/预览进程崩溃与后来的修改 | mise exec --no-deps -- npm run test:diagram；node `scripts/build-phoenix-diagram.mjs` --development --output-dir `local/phoenix-diagram/p3-stage-a`；node `tools/phoenix-diagram/tests/p3-acceptance.mjs` --stage author-tools --release-dir `local/phoenix-diagram/p3-stage-a` --output-dir `local/phoenix-diagram/p3-stage-a-acceptance`；实际查看截图后按同 run 的 review-file/resume 入口登记并核 §3.3 完成状态 | M4：V-1–V-3、author-tools 及资源准备提交/HTTPS URL/内容身份均已就绪，不依赖 M7；阶段 A 统一环境仅收口一次，不能当最终分发验收 |
| V-5 | WORK-F11/AC-3 按 C-6 替代口径、AC-4、WORK-N4/AC-3：所选 provider、完整原版识别、无历史回执可确认后移除；未确认/拒绝/非交互无标志零迁移，旧 skill 不备份/回滚不重建且给重装指引；外部管理/修改/额外文件/链接/个人保护、v1 升级、新入口回滚/取消/中断、共享配置零变化 | mbx test --locked --offline --test migration --test install；node --test `test/phoenix-diagram/migration.test.js` `test/phoenix-diagram/install.test.js` `test/phoenix-diagram/bootstrap.test.js` `test/phoenix-diagram/joint-install.test.js`；临时项目，按 C-6 核真实删除/余项/journal，不能以原备份条款宣称通过 | M5：v2/确认移除/新入口恢复回滚/启动链整批交付齐备后；来源识别/显式确认/路径安全门在任何删除前 |
| V-6 | WORK-F10 三 AC、WORK-N3/AC-4：旧活跃目录/命令/调用消失，语义/授权/fallback/嵌入约定及描述前 100 字/许可保留；扩展合同完整，五份 legacy 样本及新旧布局消费分派已迁移 | node --test `test/phoenix-diagram/skill.test.js` `test/phoenix-diagram/candidate.test.js`；rg -n 'archify' README.md `skills/elintp` `skills/documentation-writer` `skills/phoenix-diagram`；人工逐条对 §0.6/§5.5 和活跃接口清单 | M6：唯一入口、样本/必需成员/caseManifest、全部调用方/参考、兼容/扩展文档和测试齐备后；历史/许可命中须逐项分类 |
| V-7 | WORK-F10/AC-3、WORK-N2/AC-3：源→bundle→dist/tgz 的五份 legacy 样本同字节，包无旧活跃入口、完整迁移来源/许可、P3 profile/必需场景与拒绝假证据；历史候选原布局/身份及 p1/p2 默认合同不变 | node --test `test/phoenix-diagram/build.test.js` `test/phoenix-diagram/candidate.test.js` `test/phoenix-diagram/aggregate.test.js` `test/phoenix-diagram/provider-acceptance.test.js` `test/phoenix-diagram/p3-acceptance.test.js`；node `scripts/assemble-phoenix-diagram-reader.mjs` --check；核 §5.5 样本 sourceInputs/manifest/tgz 与 caseManifest、拒绝混合/缺失/篡改 | M7：all/providers 扩展/题面/profile/CI 接线及最终来源/分发资源齐备后，冻结前只做该批定向检查；不再首次交付 author-tools 或 PNG |
| V-8 | 阶段 B 最终同包收口：P3 全部核心 AC，以及 P1/P2 十类、双合同、来源、五语言/导出/兼容的受影响回归；四平台/Node18 真实包消费；品牌/预览/比较/迁移的故障、离线和安全边界 | M7 源码提交在当前分支可取回后，不要求新建分支；当前用 gh workflow run phoenix-diagram.yml --ref main -f profile=p3 -f await_review=false；按 run ID 下载并认证现有 candidate/harness 制品；从认证harness运行node `tools/phoenix-diagram/tests/p3-acceptance.mjs` --stage all --candidate-dir DIR --legacy-candidate-dir LEGACY --evidence-repo REPO --output-dir NEW；本机同一 tgz 完整消费并实际看图；node `scripts/aggregate-phoenix-diagram-candidate.mjs` --profile p3 --input INPUT --output NEW.json | M8：M5–M7 完成、唯一候选可认证后；四平台各跑一次本阶段完整消费，本机统一视觉/故障收口 |
| V-9 | WORK-F7–F11、WORK-N3/N5：§9.3 两宿主各 18 个真实新会话，实际加载路径/工具/原始材料/语义/审核和授权正确；非支持资源不伪通过 | 用 authenticated harness中的capture-p3-provider/capture-p3-stdio原生 JSONL 与 p3-scenarios.json；原始 transcript/调用结果、candidate/回执/截图由独立审核者核对；node `tools/phoenix-diagram/tests/p3-acceptance.mjs` --stage providers --candidate-dir DIR --provider-manifest FILE --output-dir NEW 只验证/归档真实原件，不合成会话 | M8：最终 tgz/VERSION/题面/collector 冻结并安装完成后；先记录实际模型/端点类别，无额外重复预检阶段 |
| V-10 | WORK-F10/AC-2：两个文档调用方已装/缺失路径，Markdown 正文截图/链接和离线 srcdoc 最终页面都完整；独立图通过不冒充最终页通过 | 复用 V-9 的 elintp/documentation-writer 四类会话产物；Agent 仅凭调用方文档、授权材料、公开说明操作，不读实现或口头补步骤；HTML 直接真实浏览器打开；Markdown 在文档目录运行 pandoc DOCUMENT.md --standalone --embed-resources --resource-path=. --output=PREVIEW.html（本机已安装 pandoc），再用真实浏览器断网检查嵌入/截图和全部标签 | M8：对应调用方会话产物就绪后，与同一环境的 V-8/V-9 合并走查，不再启动第二轮作者 |
| V-11 | 阶段 B 全量与跨期退出：WORK-F11/N2/N3/N4/N5 的 P1/P2 已交付部分仍成立；包/引擎身份、原生/Node18、恢复和导出回归；Phoenix UI 隔离 | mise exec --no-deps -- npm test；mise exec --no-deps -- npm run test:diagram；准备 UI 固定工具后 mise exec --no-deps -- npm run test:phoenix；git diff --check；各门复用 V-8–V-10 已通过且未失效的证据 | M8：本阶段所有实现、候选及验收辅助修改结束后的最终收口；不因回写/提交再跑 |

local/phoenix-diagram/p3-stage-a 与 local/phoenix-diagram/p3-stage-a-acceptance 为 M4 新建的验收输出（★），不是现有复用锚点。

BRANCH/DIR/LEGACY/REPO/NEW/INPUT/FILE 是运行时参数：源码分支、已认证候选目录、独立认证的历史v1前置、明确的来源验收仓库、新证据位置、按 §9.2 生成的清单。开始验收时在 M8 记录一次实际值，后续命令原样引用，不猜包目录、不拼接旧版本证据。所有输出父目录须符合现有维护脚本的自有普通目录要求。

**成本和最小反馈**：调查中的未优化原生回归含大型引擎身份散列、安装及 kill 矩阵，耗时已超过十分钟，UI 回归更长；不得当每次编辑反馈。维护测试可设 CARGO_PROFILE_DEV_OPT_LEVEL=2、CARGO_PROFILE_TEST_OPT_LEVEL=2，并显式保持两 profile 的 DEBUG_ASSERTIONS=true、OVERFLOW_CHECKS=true；这些测试设置不改变发行 release profile。批内仅运行具体失败用例（如 mbx test --test brands CASE_NAME）或 mbx check --locked --offline；未知耗时如实记录。M4/M8 构建保留增量缓存，不无故 clean。

UI 本机已存在 `local/phoenix-ui/build-tools/darwin-arm64/binaryen-version_117/bin/wasm-opt`，运行 V-11 时以其实际绝对路径设置 WASM_OPT；工具锁定仍由 preflight 核对，其他平台使用自己的准备回执，不能照搬该路径。普通 npm test 不证明真实浏览器、原生四平台或 provider 通过。

### 9.2 P3 维护入口与候选身份

新增 `tools/phoenix-diagram/tests/p3-acceptance.mjs`，复用现有 core-runner、candidate 验证及 capture-stdio。**M1–M3 随稳定接口准备，最晚 M3 交付可运行的 author-tools 全链**：--release-dir、--output-dir、--resume、--review-file、state.json、品牌真实采集/离线导出、比较专用 summary/12 图审核与预览/比较故障支撑；同时交付 `test/phoenix-diagram/p3-acceptance.test.js` 的阶段 A 测试。不等 M7 才有脚本、审核文件或同 run 恢复入口。resume/review-file 必须引用已有 run，不能重新生成/重跑已通过交付；比较记录遵守 §3.3。state.json 记录 profile=p3、实际 source/bundle/engine/VERSION、检查与图像审核、资源位置和未运行项，阶段 A 的 tgz/provider 为不适用，不伪填最终候选身份；缺真实原件为 incomplete，不由检查器代写 passed 审核。

**阶段 A 真实资源准备（M3 准备提交先于 M4）**：

1. M1–M3 将下列固定 Base64 解码为 `tools/phoenix-diagram/tests/fixtures/brands/official-test.png`（★）：125 bytes、SHA256 `8a1035d9172bf23b4444d8409418141e7bb39271deb17786c3860ef1defd494e`，32×32 RGBA 蓝/琥珀棋盘。它是本计划明确提供的合成测试图，不代表第三方商标或用户已授权访问任意网站，不借用未知品牌。

   ```text
   iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAARElEQVR42mNQCZnxHx/+ssEZL6ZUP8OoA0YdMOqAAXcArS0gpH/UAaMOGHXAwDtgtCQcdcCoA0YdMFoSjjpg1AEj3gEA1jDqiHXnBKIAAAAASUVORK5CYII=
   ```

2. V-1–V-3/阶段 A 支撑定向检查通过后，将已完成 M1–M3 源码、该 PNG 和 author-tools 支撑作 **M3 准备提交**，按顶部协议回填覆盖行及记录。M3 记录保存准备提交的完整 SHA、上述文件路径/字节数/SHA、资源来源及实际 URL。仅在用户现有远端/可见性授权内推送，不发 npm/GitHub Release，不擅自公开私有仓库、另建网站或携凭据到公网。
3. 资源 URL 必须含该完整提交 SHA，而非 main/latest：`https://raw.githubusercontent.com/XGENT-ai/skills/<准备提交完整SHA>/tools/phoenix-diagram/tests/fixtures/brands/official-test.png`。准备者在 M4 前匿名 HTTPS GET，核每跳安全、最终 URL、MIME/PNG 签名、125 bytes 和固定 SHA，并保存就绪证据；本地 commit/文件存在、gh 登录或 URL 字符串均不算资源可取回。未获推送授权或 URL 不可达时，先请求用户提供该测试资源的授权发布位置，不扩大可见性或悄换图片；记录未就绪，不进入依赖真实采集的 V-4。
4. M4 的 author-tools 以该已核 URL/提交/内容身份作为题面明确提供的单个测试资源完成真实 HTTPS capture；M8/V-8 复用同一资源并重核，不依赖 M7 首次提交 PNG。上述 Base64 的本地解码或 HTTP/TLS fixture 不能替代真实采集。DNS/redirect/压缩/畸形响应攻击矩阵仍用本地接缝，生产代码没有私网旁路；普通生成监测零外联。

M7 只补 --stage all|providers、--candidate-dir、--provider-manifest、必要的显式 --legacy-candidate-dir/--evidence-repo 前置、最终候选 state/认证与 profile/题面/CI/aggregate 接线，复用已有 author-tools/review/resume。默认 p1/p2 和历史输入继续原行为。p3 固定需要：四个平台同源码/同 bundle 引擎、唯一同包及认证 run/job/artifact、精确 Node 18.20.8 消费、本机完整视觉记录、两宿主各 18 题与各自真实模型/collector。按 §5.5 的认证 tgz 布局选择 legacy 路径，不从 tools/upstream 或当前工作树补消费输入；新布局五份同字节样本必须随包，历史包按旧布局核验。扩展 artifact-kind 和新命令结果的身份核对；文件存在不算能力通过，p3 不冒充 p2 的旧 64 角色聚合。

完整迁移消费另需旧 skill 文件字节，不能由仅含摘要的运行期 manifest 重建。M7 在装配端从已核固定归档生成一份完整、固定摘要的旧迁移测试数据，随认证维护 harness 转移；其中 manifest 原字节须匹配同候选 sourceInputs，三来源的完整文件集合/bytes/mode/SHA 均逐项核对。它只作为新建隔离项目的旧目标输入，不执行其中 Node、renderer 或命令；不进入 npm tgz、生产 bundle 或引擎依赖。source-less 检查仅为这一份精确路径/身份的数据开放例外，其他源码归档、Rust/Cargo、活跃 renderer 和未知文件继续拒绝，并如实说明这一测试数据边界。消费者不得从当前工作树或网络补旧文件；缺失/篡改时迁移范围保持未运行或失败，不缩减 V-8。

旧 v1 journal 的兼容终态发布只会由真实 v1 fresh install 后的新引擎直接回滚触发。它另用固定、已认证的历史 P2 候选作为安装前置，通过 `--legacy-candidate-dir DIR` 显式给出；这不是第二个最终候选，不进入新 tgz 或 harness，不执行惰性 Archify 测试数据中的旧程序，也不手造 v1 journal。M7 冻结其来源/run/artifact/原包摘要描述符；CI 从该历史制品独立取回并核摘要，本机重用已认证原字节。缺失、过期或不符时该兼容检查保持未运行/失败，不能以其他 12 个 checkpoint 通过代替。所有 P3 产品结论仍绑定唯一最终候选，聚合分别核历史前置与本期来源。

### 9.3 两宿主冻结角色题面

每行在 codex、claude-code 各开一个全新中性项目会话，共 36 个。知识/资料限于已安装 skill 的必要公开 schema/示例/参考、题面材料及固定命令的原始回执；安装题额外给确切 release-dir、临时项目和操作授权。不得读工具实现、场景答案/隐藏验收或个人缓存；缺工具题从可用 skill 列表和约定目录确认为缺失，不“假装找过”。

| 场景 ID | 任务与判据 |
| --- | --- |
| brand-builtin | 按给定身份选择内置品牌，保留职责/标签/来源并完成真实交付 |
| brand-no-url | 无匹配且无官方 URL，省略品牌，不猜站点、不下载 |
| brand-capture | 题面明确授权固定官方测试 URL；capture 后使用该摘要，导出/离线可读 |
| brand-missing | 固定资源缺失/变更时如实失败，不联网补齐或换字节 |
| preview-explicit | 明确启动后有效修改推进，半写/非法保留 last-good，停止可确认 |
| normal-no-preview | 普通作图不启动服务、不打开窗口、不采集品牌 |
| compare-valid | 双架构变化可回指输入，三个视图/未变事实可读，回执与默认审核真实 |
| compare-invalid | 无效输入或成对提交失败不报成功，保留旧对/恢复位置 |
| migrate-known | 展示已知项目副本与不备份后果，用户确认后直接移除并装新入口；无历史回执不阻塞原版识别，未确认不删除，不声称认领或生成旧 skill 备份 |
| migrate-conflict | 自定义/外部管理/个人入口保留，只报告范围和指引 |
| migrate-recover | 已确认事务的中断恢复/新入口回滚；如实列已删/剩余旧项，不自动还原旧 skill，给重装指引；后来修改边界真实 |
| legacy-dispositions | 品牌、仓库来源、旧 views/animation 各有真实处置，不复刻声明、不改原 JSON |
| caller-elintp | 已装新入口按原触发出图，离线 srcdoc，最终文档另外检查 |
| caller-documentation | 新入口出图、实际审核、正文截图/交互链接、三类成果及证据边界准确 |
| fallback-elintp | 工具缺失用原内联 SVG/HTML/CSS，不自动安装或声称 native 通过 |
| fallback-documentation | 工具缺失用 Mermaid，不自动安装或声称 native 通过 |
| workflow-facts | 给定完整系统步骤材料，保留条件/执行者/分支与关系事实，不补标签或先后 |
| erd-missing-facts | 题面缺必需参与数/可空性，只问必要开放问题并停止，不写 candidate |

角色模拟不推导真人成功率。每题原件/语义/图像独立审核，全部预期行为通过才记该宿主 pass；P2 D28 不延伸到 P3。不为模型行为失败随机重试；修复已证实产品/skill 缺陷后，按实际失效范围冻结新版本，保留原失败。外部中断可恢复未完成工作但不能把旧会话当新身份；bundle 变化后两宿主题面所依赖的规则证据须新同包重取，未变原生行为可按最终引擎复测/复用，记录明确范围。

### 9.4 故障、浏览器与证据复用

V-4/V-8 包含 compare 在双发布前后 SIGKILL、预览 worker 停止/强杀、latest-only、安装/回滚各落盘边界及旧目录逐项删除/进度写入间隙的真实中断，含 stdout 丢弃后按 durable journal 定位；源/输出/父目录/新入口备份被替换、旧目录重建、未知 claimant、同内容不同 inode、重复恢复均需结果与保留路径。旧 skill 没有备份，取消/恢复/回滚须列实际已删/剩余项与重装指引；不能把部分删除报成零变更或已还原旧入口。局部 observer 检查不充当真实 kill。

本机在四桌面窗口和 390×844、暗亮主题检查品牌标题轨、比较三个视图/变化定位、预览 shell/正式图；五语言沿现有 scope。实际查看原截图后才登记审核。默认导出格式、canonical/范围卡片边界及不支持/执行失败沿原生检查，不因 P3 缩减。V-8 与 V-10 在同一候选部署合并走查，各产物/角色 scope 仍可追溯。

同一候选先保存命令/原件，再登记实际图像观察；仅在内容/契约/配置/依赖改变时失效。M8 失败修复只重跑失败项和波及项；若共享 CLI/安装状态/构建/资源身份改变，扩大到对应平台/consumer；若 skill bundle 变化，则重新取得实际受其影响的宿主行为证据。过程记录链接原件，不复制成功日志或重建一次性证据脚本。

## 10. 里程碑与验收安排

阶段 A 是授权材料 → 品牌/比较/显式预览的完整作者路径（M1–M4）；阶段 B 是旧项目 → 唯一新入口 → 确切发行候选的维护者路径（M5–M8）。只在 M4/M8 收口全量/真实环境/故障/评测；数据和安全门保持在相应实现批内。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 品牌和输入终局兼容 | 既有 compile/evidence/output 合同可用 | 整批完成 §3.1/§5.1：brands 模块/固定目录/安全 capture/缓存/布局/回执及测试；照 evidence 绑定、冻结品牌语义。主实现者拥有 input/scene/render/prepare/CLI；目录数据可独立准备后统一集成 | 交付齐备后 V-1 通过；回写「实施进度」 |
| M2 | 可追溯架构比较与成对恢复 | M1 的资源模型和 CompileRequest 稳定，V-1 通过；不依赖新安装器 | 整批完成 compare 模块、三视图/专用 check/browser/review 分派与 pair journal；按 §3.3 冻结不可变 compare 与独立 summary/记录/公开链路，同步 delivery-contract 比较专节。共享 cli/delivery/check 由主实现者单写 | 交付齐备后 V-2 通过；单图合同保持；回写「实施进度」 |
| M3 | 显式可恢复本地预览 | 同一快照准备/发布边界已稳定，V-1/V-2 通过 | 整批完成 preview controller/同引擎可取消 worker/loopback shell/版本门/停止及测试；一并收齐 §9.2 的 author-tools/同 run 审核恢复/故障支撑与固定 PNG，完成准备提交、授权推送和匿名 HTTPS 身份核验。无后台 daemon，浏览器/端口隔离 | V-3 及阶段 A 支撑定向检查通过，§9.2 准备提交/资源就绪可取回；回写「实施进度」；不提前声称 V-4 通过 |
| M4 | 阶段 A 核心收口 | V-1–V-3 通过，三项作者能力/author-tools 支撑齐备，M3 准备提交与真实 HTTPS 资源已核就绪；不依赖 M7 | 主实现者统一本机原生构建、真实采集/浏览器/12 图审核/导出、故障与维护回归；按 §3.3 收口比较 summary，只保存阶段 A 证据，不冒充最终包消费 | V-4 通过，阶段 A 完整用户路径可验；回写「实施进度」 |
| M5 | 确认移除旧 skill、新入口恢复与回滚 | C-6 已确认；既有安装 v1 合同及固定来源可核；M4 仅为阶段顺序，不是来源调查前置 | 整批完成完整旧来源识别、安装 v2、确认后直接删除/进度恢复、新入口回滚、bootstrap/联合确认标志及测试，按 §3.2/§5.4；旧 skill 不备份。固定来源先于旧目录删除；install/CLI/bootstrap 单人写入，临时项目隔离 | 交付齐备后 V-5 通过；无范围外写入；回写「实施进度」 |
| M6 | 唯一入口与文档调用闭环 | 命令/回执合同稳定，V-5 和完整迁移来源可用 | 按 §0.6/§5.5 整批更新新 skill/参考、README/两调用方/两参考、兼容/扩展约定；先复制五份 legacy 原例并同步必需成员/caseManifest/新旧布局测试，再删旧活跃目录。主实现者单写共享文档，旧记录/许可不替换 | 交付齐备后 V-6 通过；五份样本不依赖已删除目录；回写「实施进度」 |
| M7 | 最终候选接线和源码冻结 | V-5/V-6 通过；author-tools 已在 M3 交付、M4 验收，本阶段产品修改齐备 | 只扩展 acceptance 的 all/providers 及最终候选/profile/18 题/CI/身份/来源/许可接线；核五份样本 sourceInputs/bundle/tgz 路径与固定字节，核两 CLI/浏览器/GitHub 入口和届时实际资源。主实现者拥有 CI/构建/聚合/台账 | V-7 通过；源码可提交并作为唯一候选来源；不首次提供 M4 前置；回写「实施进度」 |
| M8 | 阶段 B 同包与跨期终验 | M7 源码提交、当前分支可构建（无需新建分支）；四平台/浏览器/provider 资源到位 | 主实现者统一认证候选、四平台/Node18/本机消费、两宿主 36 会话、文档最终页和全量/故障收口；独立平台/会话可在隔离资源上调度，所有结论/记录/计划仅主实现者写回 | V-8、V-9、V-10、V-11 通过；16 条 P3 AC 按 §0.1/C-6 的替代口径及本期五条跨期需求终验均有证据，不宣称原旧入口备份条款通过；回写「实施进度」 |

提交点：M3（M1–M3/author-tools 与固定 PNG 准备提交，先于 M4，按 §9.2 授权推送并核匿名 HTTPS 身份；不等于阶段 A 收口）；M4（阶段 A 收口）；M7（M5–M7 源码冻结，四平台 CI 需要可取回提交）；M8（阶段 B 全部完成后的最终提交）。各次按顶部协议回填已覆盖里程碑 SHA 并单独提交只改计划的 SHA 回填；M7 不是阶段 B 终验。完成 M8 之前不宣称整期验收。

M1–M3 默认顺序，避免三批共同修改 cli/prepare/render 的集成成本；品牌固定数据/测试 fixture、迁移来源冻结、调用方文档草案可以在稳定合同下独立准备，不能提前把未实现接口写成已核实。内部任务无需另建状态表；完整批次内连续实现，测试失败只反馈相应任务。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 确认移除旧 skill 的不可逆部分 | 按 C-6 不备份，删除后的旧 skill 不由安装恢复或回滚重建 | 展示精确目标和重装指引，肯定确认后删除；保留自定义/外部管理/个人范围及后来修改保护；V-5/V-8/V-9 核实际已删/剩余项 | M5 删除门前 | 否，政策已确认；删除门由实施验证 |
| P2 G-07/G-08 的既有边界 | Claude 宿主残余失败、人员图例指引延期 | 保留原结论；P3 只验 §9.3 新范围，不自行将旧豁免推广或扩大产品修复 | P3 题面冻结前 | 否，非本期默认新增开发 |
| 外部网站、GitHub runner/凭据、宿主配额执行时变化 | 阶段 A 真实采集、平台或宿主终验可能 incomplete | 执行者按 §4/§9.2 记录实际缺口/责任方；M3 核资源发布授权及匿名 HTTPS 的提交/内容身份，未就绪不进入依赖采集的 M4；先推进独立实现，不伪造替身通过 | 资源在 M4 前；最终资源在相应 M8 检查前 | 尚无准备提交/真实采集通过证据，不预判就绪；实际不可用时阻塞对应检查 |
| 新安装状态与旧 bootstrap 交叉读取 | 升级/回滚后误判归属或普通命令不可用 | v1/v2 双读、active-last、恢复身份重算；V-5/V-8 必须覆盖旧→新→旧 | M5 局部退出前 | 实施风险，由既定验证解除 |
| 未公开发布/未操作活跃安装 | 候选通过不代表已向用户部署 | 保留 candidate/release 区分，最终报告真实范围；实际发布另按用户授权 | 最终交接 | 否 |

- 最终状态：**Ready**。
- 定级理由：旧入口政策已按用户 2026-10-28 的 C-6 确认为展示目标、确认后直接移除、不备份；ADR-2、接口、恢复边界、迁移题面与替代验收口径一致，无关键开工决策未定。真实 HTTPS 资源由 M1–M3 准备，发布授权及可取回证据仍是 M4 前的执行门；四平台/provider 证据在 M8 收口，不预填通过。
- 已完成调查为静态代码/固定来源比对及本机工具入口核对；P3 产品、真实品牌采集、P3 四平台/provider、比较/预览/迁移和最终文档均尚未实现或验收。本机登录可用不是这些检查通过的证据。

## 12. 已知坑与历史教训

- 用户 2026-10-28 的 C-6 明确替代 PRD F11 的旧入口备份要求；可复用 UI 的展示/确认形态，但不照搬其个人目录和 hooks 清理范围。旧 skill 删除不回滚，diagram 新入口的受管事务保护仍成立（ADR-2；`bin/xgent-skills.js:410`–412；`tools/phoenix-diagram/src/install/transaction.rs`）。
- 70 文件 legacy oracle 闭包不等于完整旧安装；全目录来源先固定再删活跃目录，仅用于识别原版和保留固定来源，不推定归属或冒充目标备份；额外/自定义内容仍冲突（`tools/phoenix-diagram/upstream/SOURCE.json`；C-6）。
- `tools/phoenix-diagram/src/input.rs`::compatibility 仍把旧展示字段延期 P3；必须改为终局 dispositions，不把“不复刻”说成全部兼容，也不继续虚构下一期。
- `tools/phoenix-diagram/src/render.rs` metadata 会序列化节点 facts；仅脱敏图标旁的 source 属性仍可能经原 brand 对象泄露 query，必须同时处理 metadata。
- capture 的慢 DNS 不受 ureq 普通请求 timeout 完整约束；截止要在解析层生效，连接使用已核地址，不二次解析（固定参考 brand-marks.mjs；ureq Resolver/AgentBuilder 本机源码）。
- 比较参考依赖旧 renderer 注释插入幽灵层；本期按结构化图元装配，不把注释字符串当稳定 Rust 接口（固定 buildDeltaSvg；`tools/phoenix-diagram/src/render.rs`）。
- documentation-writer 的旧截图命名/证据删除规则与当前绑定式交付不同；从真实回执取路径，本机证据与三个入仓成果分别处理，最终文档仍独立检查（§0.6）。
- P2 的两宿主失败与延期已有明确处置，不能因计划完成行或 Codex 通过就说全部宿主通过；新同包证据独立记录（P2 覆盖复审 G-07/G-08）。
- 本机默认 Node 不是固定 Reader 构建版本；维护入口通过 mise 的 Node 25.2.1 运行，消费最低 Node18 的证据另验。UI 的 wasm-opt 位于带平台层的缓存目录，设置实际 WASM_OPT，不修改源配置绕过预检。
