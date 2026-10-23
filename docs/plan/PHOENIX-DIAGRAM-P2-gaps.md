# PHOENIX-DIAGRAM-P2-gaps · 有效时间线兼容、失败表达与 ERD 事实核对

> **实施须知：直接依据本计划、仓库规则和当前代码实施或续做。** 先读下方「实施者定位」「实施进度」，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；使用 goal 执行时也遵循这些约定。
>
> **计划状态：Ready**
>
> 调查基线：2026-10-23 · main / 93c4842ca2b2191fe13acc6b09472c8b5244c88f · dirty。当前产品代码与候选 c1703a9 同源；未提交内容为既有评审/状态记录及本计划关联的差异报告，不作为已实施修复。
>
> 需求来源：[PHOENIX-DIAGRAM PRD v1.0](../PRD-PHOENIX-DIAGRAM.md) · **P2（第2期）补齐**；原计划：[PHOENIX-DIAGRAM-P2](PHOENIX-DIAGRAM-P2.md)；范围由[差异报告](PHOENIX-DIAGRAM-P2.prd-dev-gaps.md)的 G-01/G-02/G-03/G-06、FU-1–FU-4 确定。原报告 D-01/D-02 的既有决定继续有效。
>
> 本期交付：**六个有效参考时区的原生兼容、准确浏览器失败回执、关系事实边界，以及与既定 Reader 行为一致的维护测试**。M4 实际验收补入类图不猜数量下界；用户另行授权修复架构图自动 wrap 标题遮挡既有连线，边界见 D6/D7。
> **开发范围：本仓 tools/phoenix-diagram/ 的时间、浏览器/交付及测试；skills/phoenix-diagram/ 的既有 ERD 创作规则与 Timeline 时区说明；diagram 来源/构建/验收脚本与 dist；docs/phoenix-diagram.md 的受影响说明。**
> 本期独特职责：只补已交付 P2 的确认缺口，不重做原计划八个里程碑；新里程碑和验收记录保持独立。
> **顶层排除：接受或延期这些遗漏、改变语义/质量/视觉门、增加时区库或 Node 运行期依赖、P3 功能、公开发行、活跃安装和 Phoenix UI 产品修改。**

## 实施者定位

执行本计划的 agent 是**资深软件工程师**，依据本计划、仓库规则和当前代码实施。

- **开发边界**：仅实施本计划明确纳入的系统与改动；范围外依赖按 §4 的契约、提供方与解除条件跟进，不代建、改造或部署。源码可访问、自有服务或同仓目录不扩大开发范围。外部缺口阻塞时回写影响与继续条件，推进不依赖它的已授权工作；就地修订偏差和委派不扩大范围，新增开发对象须有用户明确依据。fixture 只覆盖约定契约的必要行为，不能成为替代外部系统的产品，也不能冒充真实集成验收。
- **表达与源码**：报告用路径、需求/检查/里程碑 ID 定位，不复述计划。源码不放需求编号、agent 标记或规划元数据，注释解释必要原因；追溯放记录或提交说明。
- **编号与任务**：需求/约束/假设 ID 只在需求账本登记一次，上游 ID 保留原编号及来源；检查、里程碑、任务 ID 分别由验证表、里程碑表、按需内部任务表定义，正文、NFR 展开、映射和进度记录只引用。修订保留已有 ID，删除/拆分时同步引用；任务表若存在，按交付物、前置成果与检查 ID 推进，任务进展及证据写入所属里程碑记录，不另设任务状态表。
- **完成的定义**：本里程碑退出检查全部通过才能记完成；未运行、失败、环境缺失分别记缺口，不冒充通过。集中到阶段收口的终验由收口里程碑负责，通过前不宣称整期或跨期需求已验收。
- **角色验收**：上手与业务场景由 Agent 按 §9 指定的知识、资料及权限约束模拟，不能借内部源码或实现者补步骤；记录实际结果与卡点，不把模拟结果写成真人数据。有工具和权限的真实环境检查由 Agent 执行；真人公测指标按其指定阶段另验，不冒充本阶段已通过。
- **按计划路线实现**：计划已定的技术路线、接口、复用锚点和检查命令直接照做，不重新调研，也不为已定决策另起方案设计、比较或评判；计划没写的实现细节按最近的同构代码就地决定。调查只读本里程碑的改动落点及其直接调用方和依赖。只有代码事实推翻计划前提或路线走不通时才停下，就地修订对应正文并在记录写明偏差。
- **工作顺序**：需要行为测试的改动先红、再绿、再重构。按实际前置成果及检查推进，不等无关终验，数据、安全、发布及用户分期门不得绕过；稳定契约下可用 fixture 开发，不能替代真实集成。
- **验证节奏**：开发中和里程碑退出只跑 §9 中排在本里程碑的局部检查，即新增或改动行为的测试、直接受影响模块的已有测试和快速静态检查。全量测试、端到端、真实环境与界面走查、故障注入、评测等收口检查只在 §9 指定的阶段收口跑一次，同环境走查合并，不在中间里程碑或小改之后提前跑。已通过的证据只在它实际覆盖的代码、契约、配置、依赖或数据被改动时失效，失效就重跑那几项，不扩成全量。收口失败修复后重跑失败项和修复波及的局部检查；修复改动共享契约、公共入口、构建或依赖时，再重跑受影响的收口检查。调优、容量这类要反复尝试的工作，先用几分钟内能复测的缩小版本迭代，达标后再跑完整检查。
- **并行交付**：独立任务在收益超过委派/集成成本且工具可用时使用 subagents，模型默认继承。任务说明只给目标/ID、稳定契约、可写与禁改路径、运行资源和检查；共享文件单人负责，数据库、端口、浏览器等隔离或串行。子 agent 返回结果、路径、验证证据与缺口，默认不另写任务文档；主 agent 审查并集成验收。边界变化先回报，失败只阻塞依赖任务；结束对话前收集或停止仍在运行的任务。

## 实施进度（实施期持续更新）

本节是本计划跨对话恢复的**唯一汇总入口**，协议文字不随实施改写，实施期只更新「恢复快照」和「完成记录」。续做时先读仓库规则、本节、当前任务相关正文与依赖，再核对 `git status` 和相关 diff；记录与工作树不符时先查明原因，不凭记录覆盖用户改动，也不凭代码存在推断验收已通过。只有继续未完任务、核验前置验收、排错或审查时，才按链接读取对应记录的相关部分；不要默认加载全部记录或原始日志。

**回写时机**：里程碑完成后立即回写，再报告完成或提交；下游只需部分成果时，所需前置检查通过并在记录保存证据即可推进。实质进展后受阻、暂停、交接、发现偏差或结束对话时也保存成果与缺口；普通子任务切换不单独回写。

**一次回写**：主 agent 更新对应记录，再同步快照变化字段和完成记录一行，不重写无变化内容或追加流水。快照写当前任务、进展与下一步，命令及证据只进记录。首次回写删占位行，每个已记进展的里程碑恰好一行，状态仅「进行中 / 阻塞 / 已完成」；完成时移至已完成行末，最近完成取该行。更新时间带时区，摘要 1–2 句，记录填相对计划的链接（如 `[M1 记录](FEATURE.records/M1.md)`）。范围、决策、接口、数据、风险或退出条件有偏差时，就地修订对应正文，不追加勘误历史。

**独立记录**：按需建 `<计划名>.records/M<n>.md`，头部写计划链接、更新时区、状态及验证基线（commit SHA；未提交写 dirty@起始SHA 与关键改动路径）。正文只写交付行为/路径、必要偏差、未完项与继续动作；验收按 `检查 ID/覆盖范围 | 命令或步骤 | 结果/必要证据` 记录，共同环境和基线在头部写一次，有差异再单列。同一测试组可覆盖多条退出条件，须逐条可追溯；引用原证据与复用依据，不复制断言全文、成功日志或调试流水。证据是命令、结果和必要的少量输出，不为留证另写一次性脚本、哈希清单或大体量证据目录。未运行、失败、环境缺失明确区分；只有复杂交接、排错或证据需独立保存时才另建任务记录/日志/截图并链接。

**项目跟进索引**：仓库已登记状态台账，或存在仓根 `.xgent-ai/sdlc/protocol.md` 时（兼容已有 `docs/project-status/protocol.md`；两套并存未切换时先核对，不双写），上述回写完成后读取台账协议，只同步实际改变的当前状态和来源链接，必要时更新索引；默认直接更新本计划状态页，不因普通进展或来源版本变化另建事件，独立留存按仓内协议；未启用则跳过。复用已有计划 ID，只更新本次触及的范围，保留其他计划和各轮评审，不复制本节及里程碑记录；台账仅作项目索引，不替代本节的恢复快照。跨机器使用仓根相对路径与明确基线，未提交成果标明同步限制；登记失败说明缺口，不将原成果回退。

**提交**：在里程碑表后的「提交点」提交：里程碑较多时在标出的阶段收口后提交，全部里程碑完成后必须提交一次；仓库规则或用户对分支、提交信息、签名或是否由 agent 提交另有要求时从其要求。提交前先回写本次进度；提交点要求的检查就是本次覆盖的里程碑或阶段收口的检查，已通过且未失效的证据直接引用，不为提交重跑；只暂存本计划相关改动，不纳入、不回退他人改动，不跳过提交钩子。提交后把短 SHA 写入完成记录中本次覆盖的各里程碑行的「提交」列（代码评审修复的提交写作「修复」加短 SHA，UI 验收修复的提交写作「UI修复」加短 SHA，均追加在原 SHA 之后），再单独提交这次只改计划文件的更新，该提交不回填自己的 SHA。已完成但未到提交点的行写「待提交」；无法提交时写明原因并记入当前阻塞，不伪造 SHA。

**全部完成**：最后一个里程碑完成后，按上条完成最终提交并回写 SHA，确认每个已完成行的「提交」都有 SHA；恢复快照的「下一步」写代码评审。随后向用户报告完成，并提示下一步用 review-code 以本计划路径为参数评审本计划：它从完成记录找到要审的提交，并纳入未提交改动。评审由用户发起，实施中不自行启动评审或修复评审意见。

**回写后自查**：每次回写后直接核对以下条目：「当前进度 n/N」的 N 等于里程碑总数，n 只计状态为「已完成」的行；「最近完成」对应最后一条已完成记录，不按编号推算；链接指向已落盘文件；已完成行的「提交」是短 SHA 或「待提交」，全部完成时每行都有 SHA 且没有「待提交」。上述结构、计数与链接检查不证明验收真实通过。

### 恢复快照

- 最近更新：2026-10-24T03:40:58+08:00
- 当前进度：3/4 个里程碑完成
- 当前状态：实施中；同包固定后端12必需预检覆盖均通过，原GLM标准11过1失败封存；准备正式62并冻结候选源码
- 最近完成：M2 · 准确浏览器失败状态与回执
- 下一步：冻结提交/计划SHA回填，固定路线新31×2取证与同源CI并行；随后五ZIP/本机同tgz/双宿主归档和聚合
- 当前阻塞：本机可用空间约2.6GiB，低于正式安装3GiB门；新62运行根尚未创建，先保存源码并核可重建构建缓存，远程CI准备可继续
- 代码基线：候选源码6571b83（此前63f076b/9abb063/f04d4c9/8b5c66b、原实现fd0be93）；r6/r7指引与生成物已冻结提交，r7源/dist同步与轻量已过、实际宿主待新取证，M4未收口

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M4 | 进行中 | 2026-10-24T03:40:58+08:00 | 固定后端12角色预检覆盖全过（原GLM标准11过1失败保留）；准备正式62/冻结候选，终验未收口 | [M4 记录](PHOENIX-DIAGRAM-P2-gaps.records/M4.md) | 待提交 |
| M3 | 已完成 | 2026-10-23T13:51:25+08:00 | ERD事实边界、六区说明与Reader维护接线完成；V-3修前红、修后14/14通过 | [M3 记录](PHOENIX-DIAGRAM-P2-gaps.records/M3.md) | fd0be93；修复8b5c66b、f04d4c9、9abb063、63f076b、6571b83 |
| M1 | 已完成 | 2026-10-23T13:57:00+08:00 | 六区原生规则、固定来源/许可与oracle完成；V-1原生13/13及来源、派生物检查通过 | [M1 记录](PHOENIX-DIAGRAM-P2-gaps.records/M1.md) | fd0be93 |
| M2 | 已完成 | 2026-10-23T14:00:00+08:00 | 回执目录先准备、summary先保留浏览器首因；V-2协议19、CLI10、delivery10项通过 | [M2 记录](PHOENIX-DIAGRAM-P2-gaps.records/M2.md) | fd0be93 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

本计划为既有模块增量与验收同步。源 AC 以 PRD 为准；下表只写本次增量，G-ID/FU-ID 以差异报告为准，不重定义。无未证实产品假设。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F1 | 功能 | PRD §3 F1/AC-2；G-01/FU-1 | 六区有效 timeline 可生成并保留时间语义 | §5.1；V-1/V-4 | 已确认，本期补齐 |
| WORK-N3 | 兼容性 | PRD §3 N3/AC-2；G-01/FU-1、G-06/FU-4 | 不以依赖缺名称拒绝有效参考JSON；既定固定画布适配的测试同步 | §3/5.1/6.2；V-1/V-3/V-4/V-5 | 已确认，本期部分交付，终验P3 |
| WORK-F4 | 功能 | PRD §3 F4/AC-3；G-02/FU-2 | fresh out-dir 下 unavailable/failed/证据写入错误准确分列 | §5.2；V-2/V-4/V-6 | 已确认，本期终验补齐 |
| WORK-F2 | 功能 | PRD §3 F2/AC-3及事实边界；G-03/FU-3 | ERD 影响含义的最小基数/FK可空性缺失先问，不先交付假设 | §6.1；V-3/V-6 | 已确认，本期终验补齐 |
| WORK-N1 | 可用性 | PRD §3 N1/AC-3；G-06/FU-4 | 同步既定 Reader 高度适配测试；不回退已通过的真实阅读行为 | §6.2；V-3/V-5 | 已确认，本期验证补齐 |
| WORK-N2 | 可维护性 | PRD §3 N2/AC-2；原计划D1/D4/D8 | 运行期只用原生Rust时间规则；新增固定数据来源与许可可取回 | §2/3/5.1；V-1/V-7/V-8 | 已确认，终验P3 |
| WORK-N4 | 可靠性 | PRD §3 N4/AC-1、AC-2，保持P1交付 | 新证据目录不覆盖旧产物/证据；确定性、身份与失败保护保持 | §5.2/8；V-2/V-4/V-5 | 已确认，终验P3 |
| WORK-N5 | 安全 | PRD §3 N5/AC-1，保持P1交付 | 不隐式取回来源、调用Node或跟随不可信目录写证据 | §3/5.2/8；V-1/V-2/V-5 | 已确认，终验P3 |
| C-1 | 技术约束 | tools/phoenix-diagram/Cargo.toml、rust-toolchain.toml、mise.toml；AGENTS.md | Rust1.99.0、Node25.2.1维护；固定Node18.20.8消费；不新增crate/模型服务，完整回归与真实环境分别留证 | §2/9；V-1–V-8 | 已确认 |
| C-2 | 范围与授权 | PRD §2.2/§7；原计划§0.5/§8；AGENTS.md | 只做本计划路径；不改archify/调用方、活跃安装、全局设置，不公开发布 | §0.4/0.5/1.2/8；V-7/V-8 | 已确认 |
| C-3 | 证据合同 | 原计划D12/D13、§9.3；报告G-04/G-05及D-01/D-02 | 保留workflow适配和严格身份；bundle改变不得复用旧宿主记录；不改写历史完成记录 | §3/9；V-5–V-8 | 已确认 |
| C-4 | Skill修订 | AGENTS.md、现行SKILL澄清原则、skills/agi-mode/playbooks/authoring-a-skill.md | 仅把已存在事实原则具体化，不融合其他skill、不改触发/授权/描述；全新上下文验证自然请求 | §6.1；V-3/V-6 | 已确认 |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 六区原生时间 | 在现有时间模块增加专用SystemV变体，使用固定历史表和明确最终规则 | 不映射成近似城市时区，不新加ICU库；现有IANA/固定偏移路径不变 | §1.1的29,544个Node对照样本；WORK-N2/N3 |
| D2 | 失败目录生命周期 | 在浏览器发现/启动前准备普通回执目录；截图暂存仅清理自己的内容，保留最终回执容器；原诊断先进入summary | 不再依赖调用者mkdir，不为记录缺席启动Chrome；真实目录错误保留独立诊断 | browser/evidence.rs、delivery/transaction.rs；G-02 |
| D3 | ERD缺事实 | 具体检查两端最大/最小参与数和FK可空依据；缺必需事实按现行开放问题即停 | 不增加审批或默认值，不改已有JSON/schema；明确材料/显式Mermaid不多问 | SKILL.md首部，报告G-03 |
| D4 | 过期Reader断言 | 按现行Reader条件分组测行为，加入原生维护入口 | 保留真实适配，不通过回退实现迎合旧断言；既定排除与源几何不变仍须验证 | reader-layout.js:36–42，reader-layout.test.mjs:164；G-06 |
| D5 | 重新取证范围 | 引擎与bundle冻结后，一次阶段收口：本机、四平台、两宿主全量及新增缺事实反例 | C-3要求相同bundle，D3改变bundle后原宿主记录不能复用；旧证据只作对照 | 原计划§9.3、D10/D13，原计划候选收口中的完整31行×两宿主 |
| D6 | 类图数量事实 | 未给下界不补0..*/1..*；逐条保留“一个/多个”等已给数量，组合/生命周期不能替代数量；明确数值和端点照用，可选数值不触发多余提问 | 具体化现有“不补私有事实”原则，不修改schema、触发/授权或验收门；bundle变化后重新取得宿主证据 | M4原生case-020实际输出无依据0..*；原SKILL事实原则；class-facts.json |
| D7 | 自动wrap标题遮挡 | 保留显式orthogonal-v及固定节点/原路径，自动wrap标题在顶轨无解时避让既有线与marker，必要时按实测扩展派生底部空间 | 不改变原JSON、用户固定几何、显式路线/标题、质量要求；只修架构标题布局及直接回归 | 用户2026-10-23明确“允许最小修复”；production-deployment两主题实际图、原SVG与红测试 |
| D8 | 宿主事实核对具体化 | 资格条件与批准结果分别保留；节点tag/sublabel、卡片同样不能从组件名称补任务执行方式，可省略标注不追加问题 | 沿用现有路径等价/不补事实原则，不改调用授权、schema或验收；新bundle先做两宿主各5小门再正式31行 | r3 Claude001 Worker异步任务、002低风险路径漏负责人批准的原件及独立审核；原SKILL第62行 |
| D9 | Workflow修复与必需读取 | 依实测整组路线优先调整未固定共享目标的列；自加端口冲突先撤掉；宿主读取截断补齐后再创作 | 具体化现有局部布局/完整读取原则，不加固定摆放、重试轮次或放宽停止/质量；renderer/schema不改 | r4 Code004正确停但未交付；原候选副本只改一列及三端口约束后四门/四原图/原位登记完整通过；common宿主返回截断证据 |
| D10 | 未结束跨度的测量边界 | 请求/父跨度结束、trace采集截止与显示窗口分别取证；未给截止不补精确时刻或精确已观测量，保留incomplete及有据时长下界 | 具体化原“不补事实”原则，不新增可选追问或修改schema/质量；每宿主增原waterfall小门，r6共12小门再正式62行 | r5 Code054原411字节题只给请求end180，候选detail/卡片却断言trace截止180；独立failed账本 |
| D11 | 写前断言与答复事实核对 | 明确ERD省略optional仍断言最小值1，未知不能靠默认字段绕过；澄清直接从问题列表起；条件检查阶段不是瞬时起始跳转；Waterfall尾段解释也核未结束子跨度 | 沿用宿主拥有事实、引擎验证输入的分工，不新增事实推断服务或改变schema/授权/质量/停滞；r7同12小门全部重取，旧failed与同包诊断保留 | r6 GLM workflow检查阶段缺撤回、waterfall final零覆盖错句、ERD列表前言；Flash额外ERD实际补两端最小值并先图后交付；独立原会话账本 |
| D12 | 实际验收后端与派生索引 | 同包预检固定Claude自然架构用deepseek/deepseek-flash，其余Claude用用户选定glm-5.3-flash，Code原gpt-6.1-sol/max；私有v14只补唯一current合法回退及已有封存native副本 | 原GLM架构failed不改，固定路线在正式31行前登记；不随机重试、不改原题/规则/质量/停止/原件或建模型服务，全部模型逐会话留档；13预检项目含1额外对照再加62正式，实为75项目/150安装回执 | r7标准11过1失败（P3视图未披露）；同包Flash自然架构新会话独立passed；A/B/A真实Edit与12原PNG、Code sealed-native四像素证明及14项私有索引预检 |

D1 是局部可逆选择：直接六区专用历史表与规则比引入完整ICU时间栈局部，且不改变已有API/依赖。两者均能精确实现；本次只缺六区，后者会扩大数据、构建与许可面。未来确有更多ICU专属规则时再评估完整库。D2保留现有 Evidence/OutputSlot，不引入新的目录管理框架。

### 0.4 职责与事实所有权

| 责任 | 作者/宿主 | 原生工具与本期 | 验收执行者 |
| --- | --- | --- | --- |
| 拥有 | 真实关系约束、来源与对缺口的回答 | 接受有效输入、准确时间/目录/诊断、确定性及证据身份 | 固定材料、原生会话、实际截图与语义逐项核对 |
| 不拥有 | 不以习惯替用户决定NULL/最小基数 | 不推断数据库业务约束、不取回来源、不调用参考Node运行期 | 不因自动/视觉通过批准补造事实，不改历史原件 |

作者确认事实后才创作；工具验证输入；审核者另核材料与候选是否等价。

### 0.5 明确不在本期

- 不接受六区限制、近似时区或先图后确认作为替代；原PRD承诺保持。
- 不改输入schema、质量阈值、默认视觉/停滞原则、skill触发/调用/安装授权。
- 不建设全量时区库、数据库DDL解析器、LLM业务事实判定服务或新UI。
- P3的品牌/预览/比较/替换archify、调用方迁移、正式升级回滚和公开发行不纳入。
- 原计划/完成记录、各轮评审和旧验收运行目录保持只读；新里程碑及新候选另存。

## 1. 当前事实与改动面

### 1.1 已核实事实

1. **已核实·缺口**：最终候选对六个SystemV/*DT均报timeline/invalid-timezone；同字节参考renderer均生成成功。format测试豁免SystemV/*DT类；现行oracle仅含SystemV/EST5与SystemV/EST5EDT，实际命中豁免的只有EST5EDT，其余五个夏令时区尚未纳入。→ 原生规则与真实一致性断言（format/time.rs、tests/format.rs；报告E4）。
2. **已核实·足够**：ICU77.1官方zoneinfo64.txt为六区各列150次历史转换，最终规则从1977年起；六区将每个UTC转换秒加各自raw offset后得到相同150项表，typeMap均从DST开始交替。raw分别为-4/-6/-5/-7/-8/-9小时。最后规则为四月最后一个周日02:00标准时开始、十月最后一个周日02:00夏令时结束。报告E7对每区4,924个样本（历史转换±1ms、400年历法循环及远期/边界）与固定Node零差异；不是城市时区别名猜测。
3. **已核实·足够**：该官方文件头标tz2025a；Node标tz2025b。只采用已逐点对齐的六区定义，不替换现有chrono-tz全库。原始文本SHA-256 c0c34daadb59d2552252405b9be08894f83d2cd8145d0f6eb35e1557721dd55f；[官方来源](https://github.com/unicode-org/icu/blob/release-77-1/icu4c/source/data/misc/zoneinfo64.txt)和报告E7可取回。现有assets/icu77-LICENSE.txt及来源校验/许可构建已支持Unicode-3.0。
4. **已核实·缺口**：browser discovery早退，或launch失败后Evidence::cleanup删除刚建目录，随后seal_browser在该目录写回执而报ENOENT；finalize在seal之后才extend浏览器诊断。→ 让目录在浏览器门前就作为最终回执容器存在，并先保留原诊断（browser/mod.rs:275–308、browser/evidence.rs:149–225/390–458、delivery/transaction.rs:333–345）。
5. **已核实·缺口**：两宿主v11h原题缺两项ERD约束，却都先画成确定事实；Claude明确承认按常规理解定的。现行SKILL已有先问原则，但ERD小节只要求列、键、FK、功能域，没有把这两个易误推点具体化。→ 最小提示及能识别缺陷的验收（SKILL.md:11–23、references/authoring-defaults.md:28；报告E3/G-03）。
6. **已核实·缺口**：Reader固定画布适配是原计划第七里程碑为通过有效参考JSON的已验证行为；旧测试仍要求contract=null/phoenix-v1等与冻结版完全一致。原计划第二里程碑的R2局部回归已记录此失败并未处理；本轮定向114项113pass/1fail，单跑同一断言失败，确认原生维护入口未包含该套件。→ 更新行为断言并纳入入口（报告G-06/E6）。
7. **已核实·足够**：现有四平台候选、真实宿主账号/采集工具、Chrome、OS拒网与聚合可用。报告E1本轮生产聚合pass；不把这些旧证据记成本计划完成。

### 1.2 文件清单

★ 为新增。以tools/、scripts/、skills/、test/或docs/开头的路径相对仓根；同行其余短路径按括号指定目录解析。

| 范围内路径 | 增量与复用锚点 |
| --- | --- |
| tools/phoenix-diagram/src/format/time.rs；★ src/format/time/system_v.rs（未带tools/前缀的短路径相对tools/phoenix-diagram/） | Zone新增SystemV(i32)，六个名称解析和offset_seconds分派；专用子模块拥有历史表/最终规则，复用chrono日期计算 |
| tools/phoenix-diagram/tests/format.rs、tests/timeline.rs、tests/support/input_corpus.rs、tests/fixtures/format/generate.mjs、tests/fixtures/format/oracle.json、tests/fixtures/timeline/generate.mjs、tests/fixtures/timeline/oracle.json（未带tools/前缀的短路径相对tools/phoenix-diagram/） | 去除拒绝例外、补六区与端到端反例及直接输入门回归；沿现有固定Node oracle，六区端到端复用现有timeline oracle生成器 |
| ★ tools/phoenix-diagram/tests/fixtures/format/icu77-zoneinfo64.txt、★ system-v-sources.json（后者位于同目录） | 原始官方文本与来源/size/SHA/规则选择清单，开发期使用 |
| tools/phoenix-diagram/upstream/SOURCE.json、upstream/source-check.test.mjs、upstream/license-sources.test.mjs；scripts/check-phoenix-diagram-source.mjs；tools/phoenix-diagram/NOTICE.md；scripts/build-phoenix-diagram.mjs；test/phoenix-diagram/build.test.js | 仿nativeText添加独立nativeTime固定身份、离线验证与篡改拒绝；复用现有Unicode全文，发行说明包含SystemV适配；同步直接调用方的来源fixture及许可名称断言 |
| tools/phoenix-diagram/src/browser/mod.rs、src/browser/evidence.rs、src/delivery/transaction.rs；tests/browser_contract.rs、tests/delivery.rs、tests/cli.rs（未带tools/前缀的短路径相对tools/phoenix-diagram/） | 目录生命周期与首因传播；沿Evidence目录/临时文件保护和OutputSlot回执提交，不改输出事务语义 |
| skills/phoenix-diagram/SKILL.md、references/authoring-defaults.md（后者相对skill目录）；test/phoenix-diagram/skill.test.js；docs/phoenix-diagram.md | ERD事实检查及澄清规则同步；M3一并同步authoring-defaults的Timeline时区接受范围（见§5.1）；描述/名称/metadata/其他原则保留 |
| tools/phoenix-diagram/tests/viewer/reader-layout.test.mjs；scripts/test-phoenix-diagram-native.mjs | 既定高度适配的断言与日常接线；不改reader-layout.js/viewer.css |
| ★ tools/phoenix-diagram/tests/gaps-acceptance.mjs；★ tests/support/gaps-browser.mjs；★ tests/provider/erd-facts.json；★ tests/provider/class-facts.json（后三者相对工具目录） | 有界真实回归入口、12组浏览器失败辅助片段及关系事实原题/明确正例；审核字段与作者题面分离，接线见§9 |
| tools/phoenix-diagram/src/layout/architecture/presentation.rs及必要直接架构调用方；tests/architecture.rs、tests/render_source.rs（后两者相对工具目录） | D7获授权的自动wrap标题避让；锁原节点与显式路径，原参考JSON/oracle只读，只替换该已修标题的旧坐标断言，其余原回归保持 |
| skills/phoenix-diagram/ENGINE.json；dist/phoenix-diagram/ | 通过既有build --sync-dist生成，不手写身份 |

不修改Cargo依赖、输入schema、D7以外的其他renderer、Phoenix UI、archify、elintp、documentation-writer及全局安装器。共享文件、构建与浏览器由主实施者统一负责；M1/M2可在稳定边界下独立推进，不强制委派。

## 2. 模块与接口

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类/适配器 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| format/time | timeline输入门、Clock、产物检查 | zone/wall/offset_label签名不变；Zone增加SystemV(i32)，无全局状态 | 无 | ICU专属历史/最终偏移规则 | 进程内，chrono已存在 | V-1/V-4 |
| format/time/system_v ★ | time.rs | pub(super) fn raw(name: &str) -> Option<i32>；pub(super) fn offset(ms: i64, raw: i32) -> i64；名称大小写不敏感，其他名称不冒领 | 无 | 150次历史转换与1977起的规则 | 进程内；只服务六区，不造通用时区接口 | V-1 |
| browser/evidence | check_browser、Evidence::reserve | pub(super) fn prepare_receipt_directory(artifact: &Path, out_dir: Option<&Path>) -> Result<(), Error>；单层普通目录、0700新建、不跟随叶symlink，不覆盖已有证据 | 现有文件系统保护 | 回执容器与截图暂存生命周期 | 本地真实FS，无新adapter | V-2/V-4 |
| delivery/transaction | finalize、browser-check、visual-check | 状态/退出码/回执schema不变；原浏览器诊断先保存，记录失败再追加不覆盖首因 | 现有OutputSlot | 四门与sidecar身份 | 进程内/本地FS | V-2/V-4 |
| skill与审核材料 | 宿主作者、语义审核者 | 影响含义的缺事实先问；完整输入照常创作；完整图与业务事实通过分别取证 | 真实新会话 | 是否拥有足够关系事实 | 既有外部宿主能力，不建设模型服务 | V-3/V-6 |

单调用方的SystemV子模块集中真实历史规则，删除后同样复杂度会回到time.rs；不增加facade/注册器。browser目录帮助函数复用原reserve的相同普通目录检查，防止两套规则漂移。

## 3. 固定数据、身份与兼容

- 原始zoneinfo64.txt逐字节保存到§1.2新fixture，来源清单保存官方URL/release、大小、SHA、六区选择、最终规则与现有Unicode许可位置。upstream/SOURCE.json增加nativeTime，来源检查器仿nativeText固定常量核对清单/原始字节/许可；缺失或篡改时离线失败，不取回或自我刷新身份。
- 维护生成器读取固定文本并断言六区归一化转换表相等、150项严格递增、typeMap交替；生成专用Rust常量和oracle。生成器显式要求Node25.2.1/ICU77.1/tz2025b。原始fixture不进入消费包；派生常量作为Rust源进入已有构建输入。
- 六区历史表使用“转换UTC秒 + raw offset秒”的标准墙钟epoch值；不是本地平台时区。1977年前按归一化瞬时做二分，转换瞬时起采用新类型；表前标准时，交替DST/标准时。1977及以后按当地标准时间所在年计算四月/十月最后周日：开始UTC=标准墙钟02:00−raw；结束UTC=夏令墙钟02:00−(raw+3600)。整个历法域由chrono公历算术承担，包含0000及偏移跨年。
- 不改变输入JSON、时间戳语法、作者时区名称、schema/receipt版本或已有IANA/固定偏移行为。旧独立HTML不重写。
- 工具仍为0.2.0/phoenix-v2；构建内容变化必须产生新sourceInputSha256、引擎/候选摘要。SKILL变化产生新bundle，三份ENGINE/VERSION由 --sync-dist同步。
- 现有宿主复用要求相同bundle，本次不满足；不得只更新VERSION或拼接旧passed。新候选按V-7绑定新证据。D12的workflow修复保持，D13没有被扩大。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| ICU77.1文本/现有Unicode许可 | 已核实足够 | §1.1与报告E7；许可已在SOURCE/nativeText及构建中固定 | 固定独立时间数据身份，复用全文；不更新其他tzdb | 缺源/改字节即离线失败 |
| chrono/chrono-tz | 已核实足够 | time.rs现有Named/Fixed与日期运算；报告E7路线 | 不新增依赖；仅新增专用变体 | 未知名称仍timeline/invalid-timezone |
| Browser Evidence/OutputSlot | 已核实缺口 | 报告E4四组对照、evidence.rs创建/清理、transaction先seal后diagnostics | §5.2精确增量，输出事务本身不变 | 不掩盖浏览器首因；目录真实失败独立保留 |
| Claude Code、Codex及Chrome | 三者真实入口已运行；Claude官网额度不可用，用户指定TokenHub已成功 | 新M4原生会话及用户2026-10-23授权；仓根.env.local已就位，额外原题与正式001均退出0 | 临时项目/新包/新会话；Claude仅子进程设置指定BASE_URL、MODEL及AUTH_TOKEN，不改全局配置，不将token写回执 | 超时/配额/图像不可用记未完成，不能代验 |
| 四平台CI/同包harness | 已核实足够 | run39816459249及报告E1 | 源码冻结后按原计划候选收口独立候选流程重取；不修改CI审核政策 | 平台缺结果不收口 |
| npm/R2公开发布、活跃安装 | 范围外 | 原计划§8、C-2 | 零写入 | 继续保留发行guard |

## 5. 核心机制

### 5.1 六区时间

zone依次匹配既有Named/ICU别名，再匹配SystemV六个名称，最后保持Fixed解析。新变体的offset_seconds调用专用规则；wall和offset_label沿原路径，因而布局/详情/产物检查共用一套结果。

历史数据不能用城市IANA别名替代；保留标准时/DST起止的边界包含规则。format测试去掉starts_with(SystemV)特判，不改oracle接受标记。至少覆盖全部历史转换±1ms、1977切换前后、400年历法循环、0000/0001与9999、大小写、不支持名称及固定偏移回归。timeline端到端使用冻结payment-incident输入的六个副本，仅改timezone，原件摘要不变；检查可见时间标签、轴说明、详情和比例位置。

M3统一维护authoring-defaults.md：Timeline行说明既有IANA名、固定偏移及ICU兼容别名，并明确本期新增接受的SystemV/AST4ADT、SystemV/CST6CDT、SystemV/EST5EDT、SystemV/MST7MDT、SystemV/PST8PDT、SystemV/YST9YDT；曾因不支持而拒绝的这六区输入可直接重试，不改成近似城市时区。沿§0.5不改输入schema，包含timeline.schema.json的description；该说明不作为名称白名单，实际接受范围以创作默认值和原生输入门为准。M3按本节固定名称更新文本，不等待M1代码完成；M4核对文实一致。

### 5.2 浏览器失败与回执目录

1. 确切输入/HTML绑定验证之后，capture=true或显式指定out_dir时先准备最终回执目录，再发现/启动浏览器。把Evidence::reserve现有普通目录创建段提为同文件私有帮助函数，由prepare_receipt_directory与reserve复用。只创建请求的单层leaf，不递归造父目录；已存在目录保持内容，文件/symlink拒绝。
2. 目录先作为回执容器存在，Evidence::reserve会认作已有目录；截图失败时cleanup只删自己创建的stage/已发布且身份仍匹配的截图，不删这个最终容器。standalone browser-check不生成截图；显式out-dir仍是获准写回执的目标，须准备目录。未请求capture且无out-dir时不新增目录；finalize/visual-check沿同一路径。
3. unavailable仍返回BrowserStatus::Unavailable，无截图；配置错误/launch/协议/断言仍Failed。fresh与预建目录必须等价。保持receipt/summary实际路径与OutputSlot保护，不覆盖已封存回执。
4. finalize在stage写入后立即收集browser.diagnostics，再执行seal_browser；seal成功不重复extend，失败追加记录错误，不能抹去原诊断。真正目录不可写/冲突的情况下，不伪装成已封存、完整或浏览器通过。
5. 回归覆盖HTML已提交后的失败、无out-dir、fresh/precreated、已存在证据冲突、symlink/替换目录、只删除本次stage和后续独立新目录重试。不得因为G-02改输出恢复语义或释放他人的锁。

## 6. 作者与Reader验收

### 6.1 ERD事实

在SKILL创作第一步及authoring-defaults的ERD小节，用短规则具体化既有原则：

- 分开核对关系每端最大基数、最小参与数与引用列可空性；FK只证明引用目标，不自动证明NOT NULL；“可有多个”不单独证明最小数为零。
- 从已给SQL约束、联合PK、显式文字或Mermaid端点取得事实；材料已明确时直接用，不重复询问、不要求作者另选表现形式。
- 需要表示而材料没有的最小数/可空性，先按现行开放问题规则询问并停止；不生成candidate、不先画0..N/1再加“待确认”，不以卡片兜底。
- 用户明确让作者拟定规则时仍沿现行建议分支，不改变授权原则。

不从其他skill借规则、不修改description/metadata/触发/安装政策。docs/phoenix-diagram.md同步唯一行为说明，skill静态测试只能证明规则/资源可加载，不能代替V-6。

★ tests/provider/erd-facts.json保存：报告E3的原题原字节、完整正例（独立题面明确customer_id非空、商品可无明细）、显式Mermaid邻接正例及预期缺口的独立审核字段。给宿主的题面不含判据/评分词。现有intent-erd正例使用完整材料；原题另作缺事实反例，不改历史会话。审核先逐条列“材料依据→候选端点/卡片断言”，再判事实是否通过；自动/视觉complete不得覆盖语义失败。

M4实际class原题出现无依据的0..*，按D6具体化同一事实边界：类图的关系种类、生命周期与两端数量分别取证。成员可独立存在不证明整体可以没有成员；保留已给“多个”等数量含义，不补未给下界，可选数值不阻止已明确关系的创作。class-facts.json固定484字节原题，审核字段分别登记明确的单个Cart与未给LineItem下界，另有明确两端数量的Mermaid正例，作者只读prompt。静态检查只保护材料身份/隔离；新bundle须在全新上下文先以额外小项目核普通类图、明确数量输入及ERD原题（每宿主各3项），通过后再准备正式各31行；这些小门不替代正式行，预计68项目/136安装回执。旧failed不回判passed，bundle改变也不复用旧passed。 r3正式材料又暴露节点执行方式与批准结果遗漏，按D8把原有事实核对覆盖到tag/sublabel、卡片，并分开资格与批准结果；r4在上述每宿主3小门外，原样增加architecture/workflow各1小门（每宿主5项、合10项目/20回执），全部通过再准备正式62项目，合计72项目/144回执。新增小门不替代任何正式行。 r4九小门通过，Code workflow原题正确停止但无完整交付；按D9最小细化现有布局修复与读取返回规则。r5仍同十小门、先两宿主workflow再其余八项；不得将root私有几何副本或旧passed代替新作者行为。r5正式waterfall又把父请求end180当作trace精确截止180，按D10分开测量事实与显示推导，保留未结束跨度及有据下界，不为可省略截止说明追加问题。r6每宿主原样增加waterfall一小门，合12项目/24回执；全部通过后再正式62项目，总74项目/148回执。 r6标准12项实际9过3失败；同包Flash额外workflow/waterfall通过，但ERD缺事实仍写candidate并交付，原件保留。按D11只具体化已有写前事实与答复规则，不靠模型轮换或重判旧结果收口；r7原标准沿用户选定glm-5.3-flash及Codex原配置，使用新bundle/原题/全新上下文重取同12项。标准11过1失败保留；按D12同包新会话用先前获授权Flash对照自然架构并独立通过，派生索引补实合法回退与原始像素来源后，固定后端的12必需角色覆盖均passed。正式Claude31行仅intent-architecture用deepseek/deepseek-flash，其余30行用glm-5.3-flash；Codex原配置。路线在正式创建/运行前登记，不按正式结果随机轮换或回判原failed。额外1诊断项目不代正式行，计入本轮75项目/150安装回执；旧bundle诊断另存不混用。

### 6.2 Reader既定行为测试同步

reader-layout.test.mjs只修改与原计划固定画布适配冲突的测试分组：

- 对宽图、intrinsic-height/width-first等实际未变路径，继续与冻结版逐项对照。
- 对非宽固定画布contain/legacy/空fit，包括缺contract和phoenix-v1，断言已进入高度适配、源SVG/viewBox/点/文字完全不变、可读下限和必要authored溢出标识正确。
- 保留narrow/embed/present/print排除；契约未知但fit为固定画布时按实际条件验证，不以猜测合同改变已有代码。
- 加入scripts/test-phoenix-diagram-native.mjs的常规Node测试列表。不会改Reader运行代码或为过测降低断言。

### 6.3 已授权的架构标题遮挡修复

production-deployment的postgres→replica为显式orthogonal-v，保留原四点903,467→903,553.5→1103,553.5→1103,640及固定节点。现有两层wrap标题在顶轨占满宽度，最终遮住WAL末段/marker；不把这条线改称auto或改端口。只对无显式标题位置的自动wrap增加合法标题轨道，按实际标题高度/间距分配必要派生底部空间；标题不能压节点、另一标题、连线/marker、legend或超出已固定画布。冻结参考输入/oracle不改；仅该已证实错误的phoenix-v1逐字标题坐标预期改为更强的可见性、原路径/节点/事实不变断言，其他逐字回归保持。用户已明确授权最小修复，不改变一般显式几何政策。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N2 | Rust核心及现有许可 | 六区原生、固定来源/许可，不加运行期Node/网络 | 缺源/失配失败 | §2/3/5.1 | V-1/V-7/V-8 |
| WORK-N3 | 固定参考输入和原文件 | 六区同语义，其他类型/合同保持 | 未知类型/时区明确拒绝 | §3/5.1 | V-1/V-4/V-5 |
| WORK-N1 | 原四桌面+390、可读下限/默认视觉 | 新六区可读，既定Reader行为有维护测试；无新性能SLO | 无法看图保持pending | §6.2、原Reader/审核门 | V-3/V-4/V-5 |
| WORK-N4 | 四门/确定性/输出保护 | 浏览器首因可见，目录失败不损坏原成果 | incomplete及具体原因 | §5.2/8 | V-2/V-4/V-5 |
| WORK-N5 | 本地默认/不跟随不可信内容 | 来源离线核对，目录冲突不覆盖，不执行内容 | 结构/证据路径失败 | 固定源、普通文件/目录与既有OutputSlot | V-1/V-2/V-5 |

不增加遥测、服务、常驻进程或新的吞吐/时延承诺。SystemV每次偏移查找对固定150项历史表二分，最终规则用有界历法运算；不扩大布局搜索。

## 8. 失败模式、候选与回滚

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现/降级 | 检测 | 恢复/补偿 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| 数据缺失/改字节或Node版本不符 | 本次维护生成/构建 | 不更新派生物 | 明确来源/版本失败 | 固定身份及生成前置 | 显式取回已钉字节，不自动刷新基线 | V-1 |
| 浏览器缺席/启动失败 | 当前交付 | 已生成HTML保留；无虚假截图 | unavailable或failed及原诊断、incomplete | 状态/receipt/summary交叉 | 环境补齐后新证据目录，仍须真实审核 | V-2/V-4 |
| 目录不可写/symlink/冲突/被替换 | 当前证据提交 | 不覆盖旧证据/无关文件 | 路径/身份失败且保留原浏览器信息 | Evidence/OutputSlot | 选择真实普通新目录；不自动删除冲突 | V-2/V-4 |
| 宿主补规则或先图后问 | 当前作者交付 | 保留原会话，事实验收失败 | 不记完整语义通过 | 材料→断言账本 | 修规则后全新会话，不回写历史passed | V-6 |
| 新bundle沿用旧宿主记录 | 新候选 | 无产品数据变化 | 聚合不得收口 | bundle/引擎/会话身份 | 新包重新采集，不扩D13 | V-7 |

发布顺序：M1–M3局部通过→冻结引擎/skill→M4开发包与本机收口/宿主→同源四平台候选→本机同tgz闭环/聚合。沿原计划候选收口独立候选分支和既有授权范围，不推main、不运行npm/R2发布、不改活跃安装。

回滚是撤销本计划代码/生成文件，保留旧候选及独立HTML；没有数据库迁移。新证据/旧证据均独立留存，不通过删除旧文件伪造恢复。回退后六区缺口与旧失败行为重新存在，不能记本计划验收完成。

## 9. 验证

命令从仓根执行，Node维护命令经 MISE_ENV_CACHE=0 mise exec --no-deps --；Rust命令用同前缀的 mbx，manifest固定 tools/phoenix-diagram/Cargo.toml、--locked --offline。未运行/失败/环境缺失分列。检查ID在本表唯一定义。

| 检查 ID | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | F1/N3六区兼容、N2来源；去例外后修前红，修后全部Node边界/墙钟/标签一致；数据篡改失败且不自修 | node tools/phoenix-diagram/tests/fixtures/format/generate.mjs --check；mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test format --test timeline；node --test tools/phoenix-diagram/upstream/source-check.test.mjs；node scripts/check-phoenix-diagram-source.mjs --check | M1局部，解锁新时区真实收口 |
| V-2 | F4/N4/N5失败目录：fresh/precreated、缺席/配置/launch/协议失败，首因/阶段/无截图/保护准确；修前红 | mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test delivery --test browser_contract --test cli；沿现有ProtocolFixture和真实Temp目录，配置缺失不冒充unavailable，真实unavailable归V-4 | M2局部 |
| V-3 | F2关系事实提示、Class材料身份与明确数量相邻正例，N1/N3固定画布断言与维护接线；名称/描述/原原则不漂移；按§5.1核对Timeline说明含六个名称及既有接受范围，schema保持不变；Reader失败断言先红后绿 | node --test test/phoenix-diagram/skill.test.js tools/phoenix-diagram/tests/viewer/reader-layout.test.mjs；静态核对native runner确实列入该套件；完整frontmatter由V-8的npm test验证 | M3局部 |
| V-4 | F1/F4/N3/N4真实新增路径：六区原参考副本finalize/strict/可见时间/导出；12种浏览器命令×目录×环境对照，原文件/证据保护 | ★ node tools/phoenix-diagram/tests/gaps-acceptance.mjs --engine <新开发引擎绝对路径> --output-dir <新运行目录>；本机darwin-arm64、Chrome和sandbox-exec；入口实现见§9.1；六个新时区交付各自四张截图（2视口×暗亮主题，共24张）均逐张实际查看，在本运行新产生且已finalize的各delivery上按现行review-record分别原位登记 | M4唯一阶段收口 |
| V-5 | 十类已有路径及FU-4回归：core十五例、Reader/导出/恢复/拒网/explore/corpus/evidence/locale、真实看图 | 沿原计划core收口第1–5步，目录使用本计划独立代号；node scripts/test-phoenix-diagram.mjs --stage core --engine <BIN> --output-dir <DIR>，再按最新index依次--resume --reader --recovery --offline --explore --corpus --evidence --locale，最后--review-file <实际记录map>；不得为取得complete重做原delivery | M4唯一阶段收口 |
| V-6 | F2全部原宿主范围及G-03反例；F4失败答复；真实新包读取后发生行为，不能带修订提示重判 | 用冻结后development包安装临时项目，沿原计划宿主/候选收口的factory/capture/index/review/archive和现有采集器，Claude/Codex各31行；intent-erd用完整正例；额外在每宿主新会话原样投放erd-facts原题，先问且无candidate/HTML/补造选项才过；所问须覆盖erd-facts.json预期缺口（商品最小参与数、orders.customer_id可空性），不重复询问材料已给事实，按独立审核字段逐条核对并留记录。原始会话/安装摘要/加载读取/语义账本/截图归档 | M4唯一阶段收口；仅角色模拟，不招募真人 |
| V-7 | N2/C-2/C-3：新同包四平台与Node18、本机distribution、宿主/额外反例绑定；不混旧候选 | 原workflow_dispatch不设await_review，四平台自动与Node18按原计划四平台检查；同tgz本机--stage distribution，按原计划distribution检查公开入口/审核/恢复/来源/语言/explore；node scripts/aggregate-phoenix-diagram-candidate.mjs --profile p2 --input <新input> --output <新result>；新反例另链实际原档/版本，不冒充聚合内置行 | M4唯一阶段收口 |
| V-8 | 仓库轻量/原生维护、依赖/许可/静态与scope不回归；Reader套件实际进入常规日志 | npm test；npm run test:diagram；npm run test:phoenix；mbx fmt --manifest-path tools/phoenix-diagram/Cargo.toml -- --check；mbx clippy --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --all-targets -- -D warnings；沿既有prepare工具环境，不跳检查 | M4唯一阶段收口 |

### 9.1 有界真实回归入口

新gaps-acceptance.mjs只用于测试，不进入消费端：

- 参数只接--engine、--output-dir；要求新的普通运行目录和实际darwin-arm64/Chrome/sandbox-exec，缺工具或断言失败返回非零、写明确not-run/failed，不静默跳过。
- 六区输入从已校验冻结payment-incident例复制，仅改timezone；逐个用指定引擎finalize、check --strict-provenance；复用export-acceptance.mjs的runPassportAcceptance、runExportAcceptance核对时间详情/真实格式，保留每个交付的输入与四张截图身份（六个交付共24张）；逐张实际查看并在本运行各新delivery分别登记review-record。主入口写索引，实际看图和review-record由实施者按产品合同完成；截图生成不代判视觉。
- 对minimal.tree做finalize、browser-check、visual-check三个公开命令 × fresh/precreated out-dir × unavailable/launch-failed，共12组。macOS在临时profile中隐藏discovery.rs列出的系统/当前用户Chrome/Canary/Chromium路径模拟缺席，清除子进程override；启动失败设子进程PHOENIX_DIAGRAM_BROWSER=/usr/bin/false。不删除或改名用户浏览器，不影响其他进程。
- 每组保留真实argv/退出码/stdout/stderr/receipt；缺席为unavailable/exit5、启动为failed/exit4，无截图；finalize前三门及三种命令的HTML/输入摘要和真实原诊断均可核对。另用既有单元反例验证冲突/替换，不为测试破坏用户目录。
- 测试脚本复用当前capture/导出/临时profile帮助函数与--use-mock-keychain，不绕Chrome沙箱。额外模型反例按V-6单独执行，不由脚本假造宿主答复。

### 9.2 取证安排

M1–M3只运行各自局部检查；全量、模型、四平台和真实浏览器集中M4。准备脚本、fixture和契约随所属代码里程碑交付，但真实入口只在M4运行。M4失败只重跑失败与受修复影响的证据；源码/bundle再次变化时按实际摘要作废受影响结果，不机械全量重跑，也不借旧bundle复用新文本行为。

V-7沿用既有聚合器的完整ZIP身份验证：`ArchiveEvidence`先核对本机ZIP摘要，再按原路径提取所需成员；只有远端索引或选取文件不能满足该接口。本次据此限定原计划§9.3留存例外：冻结后锁定一个新cohort，仅取回该次四平台consume evidence及Node18 evidence共五个必需ZIP，生成后先核对API体积/摘要与本机可用空间，不取native目标包或整套日志ZIP。只按需提取回执和待看截图，不将完整消费目录解包；聚合的身份、语义及视觉门不变。candidate/harness仍按既有同包转移流程取回。run39911661359的五份原档已留存，只作前一候选基线，不拼接到新cohort；新run/commit及逐artifact不可变API绑定写M4记录。

原计划候选收口的技术账号、候选workflow、OS拒网和工具环境为执行资源；保留其实际获取方式与已授权范围。配额/CI/图像工具临时不可用时记录未完成并推进不依赖项，不降低本计划退出门。

## 10. 里程碑与验收安排

四个里程碑只有一个阶段收口；独立能力局部验收后汇合。不预建记录，不将原P2完成记录复制为本计划完成。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 六区有效时间线的原生兼容 | 现有固定参考与§1.1已核路线 | §1.2时间模块、固定来源/校验/许可、oracle及format/timeline反例，照Named/Fixed与nativeText范式；真实入口中六区片段可并行准备，不提前跑收口 | V-1；回写「实施进度」 |
| M2 | 准确浏览器失败状态与回执 | 当前Evidence/OutputSlot契约，不依赖M1完成 | §5.2的browser/evidence/transaction及真实FS/协议/CLI回归；真实入口12组片段随此完成，共享脚本由主实施者集成 | V-2；回写「实施进度」 |
| M3 | ERD事实澄清与维护测试同步 | 现有SKILL事实原则；G-03原件与G-06失败已固定 | §6的SKILL/authoring-defaults/docs与erd-facts，并按§5.1同步Timeline时区说明；修reader-layout过期断言、纳入native runner；不改Reader产品代码，与M1/M2仅在构建集成时汇合 | V-3；回写「实施进度」 |
| M4 | 新同包P2增量终验与回归收口 | M1–M3的V-1/V-2/V-3通过，源码/skill冻结 | §9真实入口、本机core、全新宿主与原题反例、四平台新候选/同tgz本机/聚合、仓库全量；主实施者统一浏览器/候选/证据身份，按原授权使用独立临时项目 | V-4–V-8全部通过；四项follow-up均有真实结果；回写「实施进度」 |

提交点：M4（全部里程碑完成后提交一次，并按顶部协议回填各完成记录SHA，再单独提交计划SHA回填）。为取得同SHA四平台制品，可按原计划候选收口先提交冻结候选源码，但所有终验未过前不记M4完成。若提前冻结提交，其SHA回填M1–M3行；M4最终提交覆盖收口验收记录及后续必要修复/生成物，SHA回填M4行。后续修复若影响M1–M3，按顶部协议在对应行追加修复SHA；源码/bundle改变按§9.2作废并重取受影响证据。不提前冻结提交时，最终提交SHA回填M1–M4各行。各次SHA回填均按顶部协议单独提交，回填提交不回填自己的SHA。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 六区历史/最终边界 | 错误时间事实 | 实施者按固定数据+Node±1ms/400年循环，不近似IANA城市 | M1退出 | 路线已核，不阻塞实施 |
| 证据目录生命周期与并发 | 可能误删/误写 | 实施者保留OutputSlot/Evidence身份保护，真实FS反例；不改事务模块 | M2退出 | 不阻塞实施，局部门必过 |
| 文本修订不能保证宿主正确 | 事实验收仍可能失败 | V-6原题全新上下文对照，不通过就继续最小修订并作废相关bundle证据 | M4退出 | 不把未来结果当已过 |
| bundle变化使旧宿主证据不适用 | 收口成本 | D5一次集中各31行并保留原题反例，不擅自放宽D13 | M4 | 已确定范围，无待决 |
| CPU争用/Chrome/CI/配额 | 取证中断 | 沿原计划本机/候选收口负载门、串行浏览器与按失效范围重试；缺资源记录未完成 | M4 | 终验执行风险 |
| 公开联合发行未授权 | 无法宣称公开可安装 | C-2持续排除，保留guard | 任何公开写入前 | 不阻塞候选 |

- 最终状态：**Ready**。
- 定级理由：四项follow-up均有PRD或已实现行为依据；六区路线已与固定Node实测，目录/诊断根因和最小接口已追通，ERD规则只具体化现有原则，Reader只补验证。没有产品、权限、数据归属或关键NFR待决。0/4尚未实施，Ready不表示缺口已关闭。

## 12. 已知坑与历史教训

- tests/format.rs的例外会把已知不兼容变成绿灯；本计划V-1删除例外并对齐外部可观察行为，不改oracle“accepted”事实。
- 原计划宿主验收曾把browser回执写入ENOENT当成产品观察，宿主因此建目录重试；本计划按首因/目录生命周期修，不能仅增加mkdir使用说明。
- E3的原review把“交付后请确认假设”判passed；V-6先核材料再看自动/视觉状态，保留原会话，不回写历史评价。
- reader-layout.test.mjs曾未随原计划固定画布适配行为变化同步且未进入native runner；FU-4同时修断言与接线，不只让单跑转绿。
- 原计划D13仅允许同bundle复用；改SKILL后不得只重跑intent-erd或替换ENGINE摘要，M4按新bundle取证。
- 原计划候选验收隔离HOME下Chrome曾触发钥匙串弹窗；真实入口沿两个现有启动器的--use-mock-keychain。
- 仓库AGENTS.md要求轻量、Phoenix runtime、浏览器、原生平台和真实宿主分别留证；V-8不能代替V-4/V-6/V-7。
