# PHOENIX-DIAGRAM-P1 · 五类 Rust 图解候选（原生核心 · 独立阅读器 · npm 分发）

> **实施须知：直接依据本计划、仓库规则和当前代码实施或续做。** 先读下方「实施者定位」「实施进度」，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；使用 goal 执行时也遵循这些约定。
>
> **计划状态：Ready**
>
> 调查基线：2026-10-16 · `4a65a1525f9ed03c775a0e18935901e1edbb773f` · dirty；事实以当前工作树为准，PRD、其评审及部分状态台账尚未提交，相关运行源码与 HEAD 一致。
>
> 需求来源：[PHOENIX-DIAGRAM PRD v1.0](../PRD-PHOENIX-DIAGRAM.md) · **P1（第 1 期）**，范围以 §7.2 的 23 条 AC 为准；上游工作号尚未转为 SPMS 实体。
>
> 本期交付：**architecture、workflow、sequence、dataflow、lifecycle 的 Rust CLI、离线 HTML 阅读与导出、确切产物证据，以及四平台 npm 安装候选**。
> **开发范围：本仓新增 `tools/phoenix-diagram/`、`skills/phoenix-diagram/`、独立 diagram 发行资源及测试；修改共同 npm 安装、打包与发布保护的必要接入点，见 §1.2**。
> 本期独特职责：将五类生成、布局、校验及交付迁为 Rust；保留浏览器 JS 阅读器，并建立旧输入的显式兼容合同。
> **顶层排除：P2 的新增五类与仓库依据；P3 的旧入口替换、品牌、预览、比较和迁移回滚；公开发布与现有活跃安装变更**。

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

- 最近更新：2026-10-17T16:18:55+08:00
- 当前进度：6/7 个里程碑完成
- 当前状态：进行中
- 最近完成：M6 · 四平台构建与唯一候选装配能力
- 下一步：冻结clean source并取得新同包CI/真实host各22；完成四平台实图、strict聚合/23AC后最终提交与单独SHA回填
- 当前阻塞：无
- 代码基线：Waiting自循环修复与指南补充已冻结：渲染/严格路径绑定共用canonical圆角生成，原points/语义/合格路径d及legacy规则保留，冻结oracle不改、仅两条错误微线段精确投影适配。先红后绿与完整维护检查已证；Node25轻量447pass/22原skip/0fail，Diagram Rust264pass/2原ignored、Node/Reader通过。新input146/b5a5f2f4、bundle19/15aaeacd；开发引擎已构建，两原候选和最小例各validate/finalize自然0、四门全pass，完整scene与原baseline相等，12PNG实际查看。当前仅开发回归，视觉正式状态仍pending；待clean source提交与新最终CI/真实host复验。af2已封存为历史，不得冒用为新包最终证据；M7仍待同包四平台、各22、strict聚合和23AC。

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-10-16T10:28:31+08:00 | 冻结来源、工具与双输入合同；V-1/V-2 通过，布局/浏览器尚未验收 | [M1 记录](PHOENIX-DIAGRAM-P1.records/M1.md) | 98dfb34 |
| M2 | 已完成 | 2026-10-16T13:19:48+08:00 | V-3/V-4 通过；两类实际 SVG、独立检查与 pin 修复闭环已有定向证据 | [M2 记录](PHOENIX-DIAGRAM-P1.records/M2.md) | 98dfb34 |
| M3 | 已完成 | 2026-10-16T13:19:48+08:00 | V-5 通过；remaining_types 18/18、文本 7/7，已知 CJK/SEA 差异已清 | [M3 记录](PHOENIX-DIAGRAM-P1.records/M3.md) | 98dfb34 |
| M5 | 已完成 | 2026-10-16T16:13:58+08:00 | V-10/V-11 指定范围通过；native20/Node18×两版/共装5/npm入口71，原UI7案同binary下以局部复验收口 | [M5 记录](PHOENIX-DIAGRAM-P1.records/M5.md) | 98dfb34 |
| M4 | 已完成 | 2026-10-16T17:32:02+08:00 | V-6–V-9通过；10例原身份complete，全部Reader/导出/离线、8公开恢复、审核篡改与summary失败重试通过 | [M4 记录](PHOENIX-DIAGRAM-P1.records/M4.md) | 98dfb34 |
| M6 | 已完成 | 2026-10-16T17:51:30+08:00 | V-12通过；build/pack/dist/21文件harness及严格聚合、stdio采集齐备，影响组72×两版绿；真实平台留M7 | [M6 记录](PHOENIX-DIAGRAM-P1.records/M6.md) | d9e83cd |
| M7 | 进行中 | 2026-10-17T16:18:55+08:00 | 自循环与指南修复本地检查已过：Rust264/Node447及既有skip；开发3例四门与12实图已复验，同包CI/各22/最终聚合待齐，af2已历史封存 | [M7 记录](PHOENIX-DIAGRAM-P1.records/M7.md) | 3c8c3ac、c629a04、3f97df8、55ba990、2c133da、dbbd728、2b59b3a、0c5829c、7e37f2a、7f3136c、90bc8b5、9fea3fa、af2b770、20c7f80 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

本计划属于新模块及本地工具链集成。下列工作号沿用 [PRD §3、§7.2](../PRD-PHOENIX-DIAGRAM.md#72-分期矩阵)，AC 原文只在 PRD 定义；这里登记本期边界及完整设计/验证去向。全部 11 条跨期需求在 P1 后仍停在交付段，不能据此标成整条已上线。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F1 | 功能 | PRD §3 同名条目，P1 AC-1、AC-3；终验 P2 | 五类语义完整、未知图型与非法数据失败 | §2、§3、§5.1；V-2、V-3、V-4、V-5、V-9、V-15 | 已确认，本期部分交付 |
| WORK-F2 | 功能 | PRD §3 同名条目，P1 AC-1、AC-2、AC-4；终验 P2 | 宿主 Agent 整理事实、转换三类 Mermaid、定向修复并识别停滞 | §0.6、§6.1；V-13 | 已确认，本期部分交付 |
| WORK-F4 | 功能 | PRD §3 同名条目，P1 AC-1、AC-3、AC-4；终验 P2 | 四道自动门、独立视觉结论、当前输入/产物绑定与诊断 | §3.2、§5.2–§5.4；V-6、V-7、V-9、V-15 | 已确认，本期部分交付 |
| WORK-F5 | 功能 | PRD §3 同名条目，P1 AC-1、AC-3；终验 P2 | 主题/风格、缩放、搜索/详情，首次 Still 与主动 Live | §6.2；V-8、V-9、V-15 | 已确认，本期部分交付 |
| WORK-F6 | 功能 | PRD §3 同名条目，P1 AC-1、AC-3、AC-4；终验 P2 | 图像、双主题 SVG、WebM、PNG 复制及准确失败 | §6.3；V-8、V-9、V-15 | 已确认，本期部分交付 |
| WORK-F11 | 功能 | PRD §3 同名条目，P1 AC-1、AC-2、AC-5；终验 P3 | 四平台、固定身份、在线/离线、幂等、provider 及缺依赖反馈 | §3.3、§5.5、§5.6；V-10、V-11、V-12、V-14、V-15 | 已确认，本期部分交付 |
| WORK-N1 | 可用性 | PRD §3 同名条目，P1 AC-1、AC-2；终验 P2 | 四桌面窗口、可读纵向例外、对确切产物的默认图像审核 | §6.2、§7；V-7、V-9、V-15 | 已确认，本期部分交付 |
| WORK-N2 | 可维护性 | PRD §3 同名条目，P1 AC-1；终验 P3 | 五类核心全部原生；固定源码、许可是 P1 发行前提 | §1.3、§2、§5.6；V-1、V-9、V-12、V-14 | 已确认，本期部分交付 |
| WORK-N3 | 兼容性 | PRD §3 同名条目，P1 AC-1；终验 P3 | 五类旧有效 JSON 核心及固定几何不丢失；原文件不覆盖 | §3.1；V-2、V-3、V-4、V-5、V-9、V-15 | 已确认，本期部分交付 |
| WORK-N4 | 可靠性 | PRD §3 同名条目，P1 AC-1、AC-2；终验 P3 | 确定性、输入输出不别名、提交失败保留旧成果、提交后失败不补旧证据 | §3.2、§5.2–§5.4、§8；V-6、V-9、V-15 | 已确认，本期部分交付 |
| WORK-N5 | 安全 | PRD §3 同名条目，P1 AC-1；终验 P3 | 内容不能执行，核心生成/检查/阅读可离线，字体和必要资源内置 | §5.1、§6.2、§7；V-2、V-7、V-9、V-12、V-15 | 已确认，本期部分交付 |
| C-1 | 技术约束 | PRD §4、§5.1；`package.json`、`rust-toolchain.toml`、`mise.toml` | npm-only；消费 Node ≥18；维护 Rust 1.99.0、Node 25.2.1、mr-boxington 1.21.1；四个平台 | §2.2、§5.5–§5.6；V-1、V-12、V-14 | 已确认 |
| C-2 | 授权与范围 | PRD §2.2、§7 首段、§5.3；`AGENTS.md` §7 | 不自动安装、打开/预览、公开发布或改变相邻 skill 的授权；参考整合先有具体清单 | §0.4–§0.6、§8；V-11、V-13、V-14 | 已确认 |
| C-3 | 仓库验证与接口 | `AGENTS.md` Working in This Repository、§6；`skills/agi-mode/playbooks/authoring-a-skill.md` | 源文件在 skills；描述完整/前 100 字核对；接口复述同步；轻量、Phoenix runtime、真实平台/provider 分别留证 | §1.2、§6.1、§9；V-11、V-13、V-14、V-15、V-16 | 已确认 |
| C-4 | 跨期合同 | PRD §7.1–§7.4 | P1 为可交付候选，建议 5–8 个里程碑；入口替换在 P3，跨期需求不得提前终验 | §0.5、§10；V-13、V-15 | 已确认 |

没有依赖未证实产品语义的假设。吞吐、p95、最大输入规模、安装体积预算未知；本期不据此引入服务、队列或节点数裁剪。它们只影响后续性能优化；性能声明必须另有测量证据，见 §7。

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | Rust 边界 | 一个独立 Cargo package `phoenix-diagram`，库及同名 binary；模块内部按能力拆分 | 编译/布局/检查/交付/项目安装在 Rust；Node 仅作 npm bootstrap、维护构建和测试；浏览器 JS 只操作已生成 DOM | WORK-N2；现有 Phoenix UI 的 npm/native 分工 |
| D2 | 图解算法 | 按固定参考的五类布局及共享几何规则移植；旧几何分支显式保留 | 不引入 ELK/Graphviz、JS VM、WASM 或通用图插件注册器；对已有单模块能力不另造适配层 | WORK-F1、WORK-N3；§1.1 的五类实现 |
| D3 | Reader | 保留固定参考的原生 HTML/CSS/classic JS 模块和装配次序；改成 Phoenix 运行身份 | 字节由维护阶段确定性装配后嵌进 binary；用户不运行模板生成器；原 HTML 不批量重写 | WORK-F5、WORK-F6、WORK-N5；`local/archify-main/viewer/README.md` |
| D4 | 产品期次 | P1 提供只声明五类能力的新 skill，与 archify 并存 | 不修改旧 skill、README 旧路由或文档调用方；新增说明明确候选边界；P3 再原位接管 | C-4；PRD 的 WORK-F10 全部属于 P3 |
| D5 | 自动与视觉门 | `finalize` 完成四道自动门并默认采图；实际审核后由 `review-record` 原位收口，不再 deliver | 在原 delivery/browser 身份下刷新 summary 并返回自动、视觉及完整状态；不另设汇总命令，不由 CLI 代评图像 | WORK-F4、WORK-N1；PRD §5.3 C3/C4；§3.2、§5.3 |
| D6 | 安装隔离 | 单独 bootstrap、manifest、cache、项目回执和最小 Rust 安装器 | 允许复制已核对的底层范式；不抽象/参数化整个 UI 安装器，不带入 hooks、review policy、impeccable 迁移或 shim | WORK-F11、C-2；UI 安装器硬编码命名空间且承担很多本期不需要的职责 |
| D7 | 版本 | tool 首版为 `0.1.0`；npmPackageVersion 始终读取该候选的根 `package.json` | engine probe 同时校对工具/输入/阅读器合同；不把当前包版本或参考 3.1.0 当成新工具版本；构建不自行 bump/publish | WORK-F11、WORK-N2；§3.3 |
| D8 | 输出发布恢复 | 提供显式 `recover-output <private-recovery-directory> --json`，由 CLI 调用 output 的条件恢复 | 不在重跑生成时隐式选择旧事务；用户按包内说明定位并指名一次事务，仅恢复该次发布，不承接安装迁移或跨版本 rollback | WORK-N4；参考 `local/archify-main/archify/bin/recover-output.mjs` / `atomic-output.mjs::recoverRetiredPublication`；§5.2 |

### 0.3 ADR-lite

#### ADR-1：用显式输入合同保存旧几何

- 状态：Accepted。
- 驱动：WORK-N3。旧 sequence 与参考版都使用 schema v1，省略列适配字段时几何不同；只看 `schema_version` 无法判断作者原意。
- 备选：① 每类只采用新布局，导入时尽量猜旧意图；② 保留 legacy-2.17 合同，新稿通过可持久标识选择 phoenix-v1。① 实现较少，但有效旧输入可能静默改排，无法满足兼容门。另一个可行办法是强制迁移全部旧输入，它增加每次接手的操作和文件管理成本。
- 决策：选②，按 §3.1 分派。无标识的旧五类输入默认 legacy-2.17；新候选写 `meta.input_contract: "phoenix-v1"`。两者共用已编译场景及 Reader，不用旧 Node 兜底。
- 正面后果：旧省略值、固定节点和 route pins 的含义有可测试的归属；未来参考输入可显式迁移，不靠猜测。
- 代价：维护受控的 legacy 布局分支与语料；P2 接收无标识的 3.1.0 同形输入时仍须显式选择来源合同，不能凭字段相同宣称两种默认值同时成立。
- 重评触发：用户明确允许改变旧几何，或有覆盖全部省略值/显式 pins 的无损迁移证明；不能用新图好看来替代旧输入验收。

#### ADR-2：保留 Reader，原生核心与 UI 安装器各自独立

- 状态：Accepted。
- 驱动：WORK-N2、WORK-F11，避免把 Phoenix UI 的设计/评审/安装授权带入图解产品。
- 备选：① 在 UI workspace 增加 crate，直接使用完整 browser/install crates；② 独立 package，移植必要的几何、CDP 和文件保护代码。① 能少写一些底层代码，但 browser crate 拉入设计检测依赖，install 的状态目录和迁移策略专属 UI；为适配它们而改造公共框架会扩大改动面。另一个可行 Reader 方案是重建精简阅读器，包更小，但需重做全部导出与状态生命周期，验收成本明显增加。
- 决策：选②，并保留参考 Reader。共享只限真实的五类几何/渲染/输出规则；npm 公共入口只增加一次有边界的安装分支。
- 正面后果：图解不依赖 UI 引擎、hooks 或运行数据；图像/视频导出沿已建立的浏览器合同移植。
- 代价：CDP/bootstrap/文件保护存在受控重复，须注明复制的源版本与许可证；两个工具同包发行时要分别核对资源。
- 重评触发：第三个工具需要相同安装内核，且提取可以保留全部现有行为测试；届时另立公共化变更，不在本期预建框架。

### 0.4 职责与事实所有权

| 责任 | 宿主 Agent / 作者 | phoenix-diagram | 浏览器 / 分发平台 |
| --- | --- | --- | --- |
| 拥有 | 请求事实、授权材料、图型选择、candidate、实际图像审核与修复决定 | 输入边界、布局及安全序列化、确定性产物、诊断、证据身份、受管安装事务 | DOM/字体/媒体 API 的实际结果；CI 的原生构建/运行；npm 与资源服务器的取回结果 |
| 不拥有 | 不以推测补事实，不用删必需内容消除错误 | 不解释私有业务、不运行模型、不把浏览器测量判为美观通过、不接管 UI 或旧 archify | 不授权写用户配置、不因文件可下载就证明 host 加载或视觉审核通过 |

恢复归所属能力：确定性提交由 output/delivery 负责，浏览器资源由 browser 负责，Reader 导出由 export closure 负责，受管文件由 install 负责；宿主只消费这些明确状态。

### 0.5 明确不在本期

- P2：erd/class/tree/timeline/waterfall 的 Rust 编译与验收、仓库证据核对、完整 Route/Reach/概览/深链接验收、五种界面语言质量。参考 Reader 中保留的相关代码不计这些能力交付；CLI 对新增五类返回不支持。
- P3：删除/替换 `skills/archify`，重路由 README、elintp 和 documentation-writer；官方品牌查询/采集、预览服务、architecture compare、旧安装认领、跨版本迁移与用户主动回滚。P1 的安装只实现本次新安装的中断恢复和同版本幂等，不能借此认领旧目录；§5.2 的输出恢复仅处理本次输出发布事务，不扩展为跨版本回滚。
- 品牌、repository sources、旧 guided views 等字段不属于 P1 核心兼容结论。导入时逐字段列出处理结果和所属后续期次，保留原 JSON；不能把有这些字段的整份输入描述为全量兼容。必要文字、实体、关系、标签、固定几何不允许用这个排除项删掉。
- 不新建账号/云存储/模型服务、Windows 发行、数值图表或其他 infographic；不修改 Phoenix UI 功能、review 合同或 hook 策略。
- 不进行 npm/R2 公开写入，不安装到当前用户的真实项目或个人 skill 目录。P1 验收使用专用临时项目、隔离 cache 和同包候选；公开发布所需真实资源回读门保留在 §8，不能以候选结果冒称上线。

### 0.6 参考整合清单（先于目标 skill 修改）

承接 PRD §5.3，以下清单是新入口规则与资源移植的实施前对照；不改变其他 skill 的触发或授权。实现 diff 必须逐项核对，增量行为先更新此表；原则冲突没有用户依据时暂停该项。

| 目标文件/规则 | 当前行为 | 参考来源/段落 | P1 采用、适配或拒绝 | 原则、调用/授权、接口与验收影响 / 核对 |
| --- | --- | --- | --- | --- |
| ★ `skills/phoenix-diagram/SKILL.md` 适用范围 | `skills/archify/SKILL.md` Type router 为五类 | `local/archify-main/archify/SKILL.md` Type router / Mermaid input | 本期仅五类及 flowchart/graph/sequenceDiagram/stateDiagram 语义转换；十类终局留给 P2/P3 | 自动选用策略保持；排除数值图表/仪表盘和未知 infographic；完整描述与前 100 字均核对，V-11、V-13 |
| 同上，创作/修复 | 旧 Fast authoring path 有 12 主节点启发、自动路由与两轮停滞停止 | 参考 `local/archify-main/archify/references/authoring-defaults.md` Composition and meaning；参考 SKILL Fast authoring path | 拒绝节点数裁剪；保留事实完整、先自动路线、按诊断修复与历史最佳错误数；不引入参考版额外的强制摆放轮次 | 示例只教结构，缺个人事实须询问；不能删除语义来通过；V-13 |
| ★ `skills/phoenix-diagram/references/delivery-contract.md` / 最终答复 | 旧 Delivery 分开确定性、浏览器与默认视觉结论 | 参考 `local/archify-main/archify/bin/finalize.mjs` FINALIZE_STAGES；delivery-contract Automated browser evidence / Perceptual review | 采用四门、严格身份与可读纵向例外；实际审核后按 §5.3 由 review-record 原身份收口；拒绝默认 `not-requested` 视觉策略 | 默认需实际图像审核；安装/preview/open 不自动触发；接口用 §3.2，V-6、V-7、V-9、V-13、V-15 |
| ★ Viewer / `skills/phoenix-diagram/references/viewer-runtime.md` | 旧 Reader 静态默认；旧桌面首屏要求 | 参考 `local/archify-main/viewer/motion-governor.js` readStored / readerPaused；参考交付合同纵向例外 | 保留首次 Still、主动 Live；采用受约束页面滚动、参考导出清理 | 用户已确认该验收差异；拒绝内部滚动/裁剪/缩字伪通过；V-8、V-9 |
| ★ 输入 schema / 导入说明 | 旧 lifecycle v1 固定 lanes/col，sequence 默认 fixed | 参考 lifecycle README v3、sequence README Column fit；旧同名 renderer | 以 §3.1 适配，旧核心不强制转 v3；显式选择新合同 | 不改变稳定身份、用户 pins 或原字节；废弃展示字段逐项报告；V-2–V-5 |
| ★ bootstrap / setup | 旧 skill 的 update awareness 查询上游，Node renderers 不需另装 | 参考 SKILL Update awareness；`skills/phoenix-ui/scripts/phoenix-bootstrap.cjs` manifestAt / verifyEngine / installEngine | 拒绝 Archify 更新判定；采用固定 Phoenix 身份与显式 engine install；缺引擎给固定包版本指引 | 普通生成无下载/更新/安装；无 shell 时交付未验证说明，不用手写 SVG 冒充 Rust 通过；V-10、V-11、V-13 |
| README 与文档调用方 | 当前调用 archify，保持各自 fallback、语言与嵌入验收 | PRD §5.3 C7；参考 Viewer 与 Delivery 合同 | 本期不迁移现有调用；只增加独立候选说明和新 API 文档 | P3 按真实新回执适配，旧嵌入验收与授权保持；V-11 |
| ★ 来源 / 许可与元数据 | 旧 MIT、tt-a1i / Cocoon AI 来源 | 参考根 LICENSE、THIRD_PARTY_NOTICES.md、Viewer README | 保留来源与全文许可；Phoenix 修改单独标明；字体 OFL 随独立 HTML/SVG；不打包本期未使用的品牌数据 | 新 skill 不冒称原创；无现有 `agents/openai.yaml` 的目录不为填表新建；V-1、V-12 |

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·足够]** 当前 `skills/archify/package.json` 为 2.17.0-dev.1；五类 CLI、schema、examples 及 Node renderer 是旧输入依据。`local/archify-main/archify/package.json` 为 3.1.0，其十类注册见 `local/archify-main/archify/bin/archify.mjs`；两个源树都不等于 Phoenix 已实现。
- **[已核实·缺口]** 当前没有 diagram Rust package、npm bin 或 dist 清单 → 按 §1.2 新增 → 仅换 skill 名不能满足原生核心/安装验收（`package.json`、`bin/phoenix-ui.js`）。
- **[已核实·足够]** 参考 architecture 用 `gridLayout` / `resolveComponentPos` / `createRouter`；workflow 与 dataflow 有 `compileWorkflow` / `compileDataflow`，各自拥有测量、route/label repair；sequence、lifecycle renderer 内完成结构与布局校验。最同构移植锚点为其 `renderers/` 文件，不以通用新算法替换。
- **[已核实·缺口]** 旧 lifecycle schema v1 要求 lanes；参考 schema v3 要求 mainPath 且不接收旧几何。旧 sequence renderer 默认 fixed、participantH=54；参考 README 明确默认 spread，卡片高度 60 → 需要 §3.1 的独立旧合同 → 否则同一个 schema_version 会静默改变几何。
- **[已核实·足够]** 参考 `local/archify-main/viewer/README.md` 规定 classic-script 装配次序、字体、canonical cleanup、编码失败及资源清理；`local/archify-main/scripts/generate-viewer.mjs` 用唯一 marker 插入片段，Export 唯一嵌套 Cleanup。`local/archify-main/viewer/motion-governor.js` 实际以 `readStored() !== 'live'` 初始化暂停，不能照搬“Live 默认”的说明文字。
- **[已核实·足够]** 参考 `local/archify-main/archify/bin/finalize.mjs` 固定 validate/deliver/check/browser-check，按 digest、bytes 和 receiptId 校验；`local/archify-main/archify/scripts/check-render-output.mjs` 检查实际 HTML/SVG 而不是只信编译器场景。`local/archify-main/archify/renderers/shared/atomic-output.mjs` 以文件身份、备份、no-clobber link 与恢复记录保护后来的修改。
- **[已核实·足够]** UI bootstrap 校对大小、SHA-256、probe，并允许本机 HTTP 测下载；普通命令只校验已有 cache。UI Rust `install/src/cli.rs::providers` 固定 19 个 provider 目录、默认检测现有目录、空项目用 .claude；Codex 展开为 .agents/.codex。`transaction.rs::preflight` 零写入，apply 锁内重读、备份/日志先行、active receipt 最后写。
- **[已核实·缺口]** `dist/phoenix-ui/VERSION.json` 当前是 darwin-arm64 单平台 development 清单 → diagram 不能借它声称四平台交付；现有 `.github/workflows/phoenix-ui.yml` 提供四个原生 runner 的项目内范式，diagram 必须产生自己的四平台运行证据。
- **[已核实·足够]** 本机有 Rust 1.99.0、mise、gh、claude、codex 和 Chrome 可执行文件；gh 账号可访问 GitHub。这里只证明入口存在，不证明未来候选、provider 会话、四平台或浏览器验收通过。

### 1.2 文件清单与拓扑

★ 为新增；目录行约束该目录内的新增文件，未列为范围内的邻接系统不修改。

| 范围内路径 | 交付/改动 | 同构实现与边界 |
| --- | --- | --- |
| ★ `tools/phoenix-diagram/` | Cargo.toml/lock、build-tools.lock.json、src、viewer、assets、upstream、tests、NOTICE 与第三方许可；具体模块见 §2 | 单 package；不把图解塞进 UI workspace |
| ★ `skills/phoenix-diagram/` | SKILL、schemas、五类 examples、references、scripts/phoenix-diagram、phoenix-launcher.cjs、phoenix-bootstrap.cjs、ENGINE.json | §0.6 清单约束；无旧 Node renderer 或生成时 update checker |
| ★ `bin/phoenix-diagram.js`；`bin/xgent-skills.js` | 独立 npm bin；总安装器新增 diagram 预检/写入/状态与 skip/release 参数 | 参照 `bin/phoenix-ui.js` 和 preparePhoenix，保留 UI 与 XGENT 现有行为 |
| ★ `dist/phoenix-diagram/` | bundle/manifest.json、VERSION.json、LICENSE、NOTICE、THIRD-PARTY 清单/全文许可 | bundle 的源为新 skill；引擎不直接塞进 npm tgz，仍由固定 manifest 获取 |
| ★ `scripts/prepare-phoenix-diagram.mjs`、`scripts/build-phoenix-diagram.mjs`、`scripts/test-phoenix-diagram.mjs`、`scripts/check-phoenix-diagram-source.mjs`、`scripts/pack-phoenix-diagram-candidate.mjs`、`scripts/check-phoenix-diagram-candidate.mjs` | 显式准备、离线构建、定向/收口测试、来源校验、四平台候选装配与验收 | prepare/build 分离；candidate pack 不修改 UI 发行资源，不调用公开发布 |
| ★ `.github/workflows/phoenix-diagram.yml`；★ `test/phoenix-diagram/` | 原生构建、同包四平台消费及 Node18；bootstrap/package/skill/installer 测试 | runner 与工具版本沿现有 Phoenix workflow；Rust/browser 测试不伪装为 npm test 已覆盖 |
| `package.json`、`.gitignore`、`scripts/require-release-package.mjs`、`scripts/release-phoenix-ui.mjs`、`test/phoenix-ui/release.test.js` | 添加 bin、test:diagram、候选命令、dist files 及包内 docs/phoenix-diagram.md；忽略新构建输出；发布 guard 和现有 release 入口必须检查 diagram 就绪 | `release:phoenix` 保留 UI 发行职责；携带 diagram 而联合发行尚未就绪时，在任何公开写入前停止，不遗漏资源 |
| `test/xgent-skills.test.js`、`test/npm-entrypoint.test.js`、`test/phoenix-ui/install.test.js` | 原 hooks/UI 专项 fixture 与调用增加明确的 diagram skip；UI fixture 包装器按需携带新 bootstrap 模块 | 只保持原测试目标与故障闸；diagram 正向/共同安装由新测试实际验证，不因 skip 掩盖新链路 |
| ★ `docs/phoenix-diagram.md` | 消费命令、输入/证据合同、审核后状态收口、候选安装、引擎资源及输出事务恢复、平台与授权边界 | 包内说明及 --help 包含 kill 后持久记录的只读定位与 recover-output 调用；只记录 P1 能力；不改 README 的 archify 路由 |

数据流：宿主写 candidate → Rust input/五类 layout → 原生 SVG+Reader → output 提交 → strict check → Chrome 自动证据 → 实际图像审核 → review-record 原身份汇总 → HTML 链接。Node bootstrap 只定位并校验 Rust；npm 安装器向独立 install 模块交接。没有常驻服务或 HTTP 业务 API。

### 1.3 固定参考与来源就绪

参考目录没有 .git，使用内容地址而非虚构上游 commit。读取根集合 `archify/`、`viewer/`、`scripts/`、`test/`、根 package.json/package-lock.json/LICENSE/THIRD_PARTY_NOTICES.md，递归只接受普通文件。每行 `[POSIX相对路径, 字节数, sha256]`，按路径 UTF-8 字节升序；数组使用 UTF-8、无空白 JSON、无结尾换行计算 SHA-256。本工作树的结果为 **`4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395`**。

M1 的首个交付是把这些确切字节压成 ★ `tools/phoenix-diagram/upstream/archify-3.1.0-source.tar.gz`，随源树清单及来源标识提交；解包后重算上述摘要必须一致。归档元数据固定 mtime=0、uid/gid=0、空 uname/gname，归档本身另记 digest。来源注明“用户提供的本地 3.1.0 快照，未核对上游提交”，不 fetch latest。源码归档不进入消费者 npm 包；其中第三方标识数据只保留为参考，不能进入 P1 运行资源。

旧核心依据取上述调查 SHA 下的 `skills/archify/`，并将相关 JSON、schema、必要 Node oracle 源码/许可冻结在同一 upstream 目录的 legacy 子集；不依赖以后会删除的活跃目录。两份冻结来源均先通过 V-1，后续移植与 oracle 才开始；若本机参考字节改变或其他机器拿不到归档，阻止该移植并取回此摘要的来源，不能静默换基线。

许可最小闭环：参考 MIT 全文保留 tt-a1i 与 Cocoon AI；从 UI 复制的 CDP/安装文件片段保留 Apache-2.0 来源和修改标识；JetBrains Mono OFL 全文随嵌入字体及 SVG；Rust 依赖按 Cargo.lock 收集全文。P1 不携带 Simple Icons/官方品牌数据，P3 再完成品牌资源许可与验收；这不豁免所实际分发代码/字体的许可。

## 2. 模块、接口与依赖

### 2.1 边界与可直接调用的接口

以下路径均位于 ★ `tools/phoenix-diagram/`；所有路径是本期交付，函数名为本计划确定的接口。类型仅公开给同一 package 的库/bin/测试，不建立跨工具 SDK。

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类/适配器 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `src/input.rs` / `src/diagnostic.rs` | compile、CLI | `parse_input(bytes: &[u8], kind: DiagramType, options: &CompileOptions) -> Result<DiagramInput, Diagnostics>`；schema、跨集合身份/引用先于布局 | 无公共 adapter | 输入合同分派、路径注释、错误定位与后续字段处置 | 进程内；只允许内嵌 schema registry | V-2 |
| `src/layout/`、`src/geometry.rs` | compile | `compile(input: &DiagramInput, options: &CompileOptions) -> Result<CompiledDiagram, Diagnostics>`；一份 resolved scene 同时驱动布局回执与 SVG | 五类直接 enum dispatch | 旧几何、新自动路线/标签/边界/图例、失败修复建议 | 进程内；无 JS VM/布局外部进程 | V-3、V-4、V-5 |
| `src/render.rs` / `src/check.rs` | validate、deliver、check | `render(scene: &CompiledDiagram, assets: &EmbeddedAssets) -> Result<Vec<u8>, Diagnostics>`；`check_html(bytes: &[u8], quality: Quality) -> ArtifactCheck` | 固定模板资源；独立 HTML parser | 安全序列化、已渲染几何的独立检查 | 进程内；HTML parser 用本仓已用的 scraper 0.27 系列，锁定精确版本 | V-3–V-6 |
| `src/output.rs` | delivery、install、证据写入、CLI recover-output | `capture_slot` / `publish_bound` / `recover_publication` / `OutputSlot::pending_publications`；公开恢复命令直接调用后者，按 §5.2 验证私有记录、父目录、备份与 public 状态；无法证明所有权即保留并报错 | 私有文件操作 observer 仅用于竞争/故障测试 | 锁、独占暂存、持久记录、备份、no-clobber 发布及条件恢复 | 真实本地文件系统；Unix libc；不以 in-memory FS 代替关键验收 | V-6、V-9、V-10、V-15 |
| `src/delivery.rs` | CLI | `validate`、`deliver`、`check`、`finalize` 共用编译/检查逻辑；`record_review` 按 §5.3 在输出锁内登记审核并刷新/返回当前 summary，不重新交付 | 不 spawn 自身或 Node 串门 | 当前版本绑定、阶段短路、sidecar 生命周期与审核后收口 | 进程内 + 本地文件 | V-6、V-9、V-15 |
| `src/browser/mod.rs`、`src/browser/cdp.rs`、`src/browser/probe.js` | browser-check、visual-check | `check_browser(artifact: &BoundArtifact, options: &BrowserOptions) -> BrowserReceipt`；真实 Chrome、文件只读、前后重新核对摘要 | 私有 CDP transport；测试可注入缺席/协议错误，终验走真实 Chrome | 字体/页面稳定、viewport、主题、DOM/截图、超时与清理 | 本地可替代浏览器；复制 UI CDP 必要子集，保留 license；无 UI detect 依赖 | V-7、V-9 |
| `src/install.rs` | npm 总安装器、CLI install | `plan_install` / `apply_install` / `recover_install`；文件白名单、预检无项目写、active 最后提交 | 私有 observer | provider、ownership、日志、并发与中断恢复 | 真实项目目录、OS lock；不加载 UI install crate | V-10、V-11 |
| `viewer/` / `assets/template.html` | render、读者 | 保留固定片段初始化顺序与语义 DOM；生成后 include_bytes 嵌入 binary | 浏览器能力探测；不把导出移到 Node | 阅读状态、播放、canonical clone、媒体事件终态与 URL 回收 | HTML/JS 浏览器端；无网络资源 | V-8、V-9 |

删除测试：input 隐藏兼容与验证策略，output 隐藏两个真实调用者共有的文件所有权规则，geometry 服务五类；删掉它们会把规则散回调用方。没有额外 facade、插件总线或远程 service adapter。

### 2.2 依赖选择与构建约束

沿本仓锁定的 serde/serde_json、sha2、libc、tungstenite、url、base64 和 scraper 范式，不链接整个 UI crates。新引入 `jsonschema = { version = "=0.58.6", default-features = false }`：已有 serde 不能等价执行这些 Draft 2020-12 schema，手写第二套校验器代价更高。使用 `draft202012::options().offline().with_registry(&registry).build(...)`，旧、新输入合同分别构造 registry（两份来源的 $id 相同，不能混装覆盖），各自仅预登记随包 schema；调用 `iter_errors` 转为诊断。该版本的 [官方 API](https://docs.rs/jsonschema/latest/jsonschema/) 提供离线选项；已核对下载到本机的 0.58.6 源码 options.rs 与 Cargo.toml，关闭默认 features 可禁用 HTTP/file 解析。输入适配规则如下，V-2 固定实际边界。

参考 common schema 的 portableOutputPath 包含 `not.pattern: "[\\uD800-\\uDFFF]"`。jsonschema 0.58.6 会在构建 registry 时拒绝该 surrogate-range regex；Phoenix registry 的内存副本仅在 `common.schema.json#/$defs/portableOutputPath/allOf/2/not/pattern` 把 pattern 改为永不匹配的 `(?!)`，不修改冻结原 schema，也不关闭其他 schema/meta/format 校验。原因是 Rust JSON 字符串只能含合法 Unicode scalar，serde_json 已拒绝未配对 surrogate，合法 surrogate pair 解码为完整码点；在这个输入域里原 pattern 本来也永不匹配。离线小实验已核对旧五类的 14 份及参考五类的 15 份示例全部 schema-valid，并核对成对/未配对 surrogate；这不是布局或全量兼容结论。V-2 将这些样本、非法输出路径和 registry 外 ref 的拒绝固化为回归，保留精确适配位置，禁止递归泛改任意 regex。

新 package 的 Cargo.lock 固定完整依赖；C-1 版本及 fmt/clippy 由 mise 预检，mbx 管理构建目录。prepare 是唯一可显式下载依赖/浏览器的维护命令，build/test 均 `--locked --offline` 且禁隐式安装。diagram 不需要 UI 的 Bun、WASM、wasm-bindgen 或 Binaryen，不能直接调用要求这些依赖的 UI preflight。

移植时核实 sequence 的 `wrapNoteText` 与 dataflow 的 `label-capacity.mjs` / stage 标题使用 Intl.Segmenter，空格或 Unicode scalar 切分不等价。原生固定 ICU4X 2.0 compiled data（Unicode 16 / CLDR 47，[官方来源](https://github.com/unicode-org/icu4x/releases/tag/icu%402.0.0)），维护 Node 25.2.1 实测 ICU 77.1 / Unicode 16.0 / en-US。实际回归证明同代数据仍有 CJK/SEA 算法差异，因此按固定 ICU 77 源移植 CJK 全候选成本/NFKC 与四类 SEA 三词前瞻，复用完整 baked dictionaries；Han/Mark/JS 空白表从固定 Node 生成。来源 SHA 和 Unicode 许可随源码保存，不调用系统 ICU、JS VM 或网络，不做语料特判。

依赖精确锁定 icu_segmenter/icu_segmenter_data/icu_collections/icu_normalizer/icu_normalizer_data 2.0.0、icu_properties 2.0.2、icu_provider 2.3.1（sync）、zerofrom 0.1.8（alloc），关闭各无关默认 feature；同版数据不当等价验收，V-5 比较实际边界/完整词典/属性。变更依赖后显式 `mise exec --no-deps -- node scripts/prepare-phoenix-diagram.mjs --prepare --update-lock`，普通 prepare 保护已有 lock。

## 3. 数据模型与兼容

没有数据库。权威输入是确切 JSON 字节；scene、HTML、自动/视觉回执均为绑定版本的派生物，不能反写输入。

### 3.1 输入合同与五类场景

`DiagramType` 只列五种。`CompileOptions` 包括 `input_contract`、`quality`、`viewer_revision`；质量优先级为显式 `--quality` → meta.quality_profile → showcase。显式参数与已有 meta.quality_profile 冲突返回 `input/quality-conflict`，不默默切换；options 规范化后记录 configSha256。环境变量不改变布局。输入带未知图型、版本或合同标识时直接失败。

新 schema 在 meta 增加 `input_contract`，唯一新稿值 `phoenix-v1`。未标识的五类 v1 与 workflow v2 默认 legacy-2.17；CLI 可显式传 `--input-contract legacy-2.17` 或 `phoenix-v1`，与输入已有值冲突为 `input/contract-conflict`。新稿一律写标识，避免日后接手失去默认值语义。schema 校验用对应合同版本，不把新字段送进旧 additionalProperties 闸。

| 图型 | legacy-2.17 合同 | phoenix-v1 合同与移植锚点 | 固定语义与几何 |
| --- | --- | --- | --- |
| architecture | 旧 schema v1 与旧 grid/free-coordinate 行为 | 参考 `render-architecture.mjs`、`grid.mjs`、`routing.mjs::createRouter` | components 身份/职责、boundaries 归属及连接 from/to/label/variant；pos/size/viewBox/side/via/labelAt 原样约束布局 |
| workflow | v1 固定 layout；v2 按旧 compiler 解释，不能悄悄改成参考新策略 | 新稿 v2；参考 `workflow-compiler.mjs::compileWorkflow`，保留 readable-v2、pins 与 measured scene | lanes/phases/groups、mainPath、条件、回路；节点 col/yOffset、绝对 pins 和 viewBox；v1 不自动升级 |
| sequence | v1，省略 column_fit 时保留旧 fixed、卡片高度/默认画布/消息 y | 参考 `render-sequence.mjs`，默认 spread、内容高度与 notes 包裹 | participants 次序；messages.from/to/y/variant/label/note；activations/segments 范围，不能排序改向或删除 note |
| dataflow | v1，旧 stage/row 默认与显式宽高/路线 | 参考 `dataflow-compiler.mjs::compileDataflow`、joint-label-routing、label-capacity | stages、nodes、flows、classification；每条数据流有 label；显式 stage/row/yOffset/width/height/viewBox 与 route pins 优先 |
| lifecycle | v1 独立 legacy 分支，直接移植旧 `bandFor`、`measureState`、`pathFor` | 新稿 v3；参考 lifecycle renderer 的结构→主轨/支线/arc→标签重试 | v1 lanes/main/terminal、col/yOffset/尺寸/viewBox/route pins 全保留；v3 mainPath 必须有真实转换；等待、终止、可恢复失败的返回边不能伪造 |

内部 `CompiledDiagram` = `diagram_type + input_contract + view_box + reader_fit + nodes + relations + containers + text + legends + cards + diagnostics`。Node/Relation 保留稳定 authored id、源 JSON pointer、完整文案和种类；Relation 同时带 from/to、variant、note、折线 points、label plate；Sequence 的 participant/activation/segment、Lifecycle 状态类型与 Workflow 主路径/泳道保留为带类型事实，不能压成丢语义的通用无向图。已著显式几何单独保留 provenance，用于断言编译后未漂移。

共享几何直接移植 `geometry.mjs`、automatic-labels、route-quality、text-fit 的被用路径：文字宽度按 Unicode scalar 迭代，CJK/variation selector 与现有 textUnits 一致；数值有限、稳定数组顺序、相同 tie-break、固定精度的数值输出。自动路线只改变没有 pin 的对象；节点/关系没有新增人为数量上限。旧 schema 本身的版本/字段约束仍保留，不能以“取消限制”为名扩大合法数据后再猜语义。

品牌、sources、meta.repository、旧 views/animation 等被延期字段先做类型与语义边界识别，回执 `compatibility.deferredFields` 列 JSON pointer、原因和去向；被核心语义引用的字段不能归入延期。原始字节保留，完整兼容状态为 partial，coreCompatibility 可独立为 pass；showcase 的 composition 仍必须零错误/零警告。兼容处置说明不是可以忽略的 composition warning。

P1 不提供批量迁移/覆盖输入命令，也不自动迁移 lifecycle v1。需要作者改成新合同只能另存新 candidate，再验证语义与固定几何；无法保持时明确缺口，不将失败输入算通过。旧 HTML 是独立文件，入口或包变化不触碰它。

### 3.2 回执与失效合同

所有 JSON 回执使用 `schemaVersion: 1, tool: "phoenix-diagram", toolVersion: "0.1.0", command, status, diagnostics`；`engine` 包含 sourceInputSha256、inputContractSchema=1、viewerRevision、receiptSchema=1。文件身份统一 `{sha256: hex64, bytes: u64}`。路径、时刻和 receiptId 只进 sidecar，不进确定性 HTML。

| 对象 | 字段与约束 | 生命周期/权威来源 | 读取与失效 |
| --- | --- | --- | --- |
| Diagnostics | `stage, code, severity, message, subject:{diagramType,path,identity?}, evidence, supportedFixes[]` | 来自真实闸；JSON pointer 精确到对象；修复只包含已验证动作 | stdout 单一 JSON，普通说明去 stderr；未执行的门不能生成 pass |
| DeliveryReceipt | `receiptId, candidate, artifact, configSha256, engine, inputContract, quality, compatibility, checks, composition, commitState` | input snapshot 与原生实际检查；随机 receiptId 不影响 HTML 字节 | ★ `<output-stem>.delivery.json`；只有 artifact/config/candidate/engine 全部匹配才 current |
| FinalizeReceipt | `stages:[{name,status,receipt?}], candidate, artifact?, deliveryReceiptId?, automatedStatus, visualReview, deliveryStatus, evidence, diagnostics` | finalize 建立四门汇总；review-record 只在相同交付/浏览器身份下更新视觉记录链接与完整状态 | ★ `<output-stem>.finalize-summary.json`；阶段状态 pass/failed/unavailable/not-run；顶层身份失配必须失败；deliveryStatus=complete/incomplete |
| BrowserReceipt | `artifact, candidate, configSha256, engine, deliveryReceiptId, browserVersion, coverage, checks, screenshots?` | 真实浏览器；每个 viewport/theme 的事实单列 | ★ `<output-stem>.browser-check.json` 或 visual-check.json；取证前后校对当前 HTML；截图亦有 digest/bytes |
| VisualReviewRecord | `artifact,candidate,configSha256,deliveryReceiptId,browserReceiptIdentity,screenshotIdentities,reviewer,scope,verdict,findings` | 有图像能力的实际审核者；verdict=passed/failed/unavailable，包含所看文件 | ★ `<output-stem>.visual-review.json`；普通 JSON 不能自行证明审核发生，交接仍需实际观察证据 |
| RecoveryReceipt | `recoveryDirectory, target, recoveryStatus, publicState, diagnostics` | recover-output 对真实事务的恢复/保留事实；沿用公共回执头 | 仅命令 JSON；状态及退出码见 §5.2，不生成 delivery/browser/visual 通过回执 |

视觉默认 `required-pending`；自动 pass 不更改它。无图像能力记 unavailable；审核失败记 failed。实际审核后用 `review-record <html> --record <json> --json` 验证、封存审核记录，并按 §5.3 条件刷新同一份当前 summary，返回 `deliveryReceiptId, browserReceiptIdentity, automatedStatus, visualReview, deliveryStatus, evidence` 及 summary 的路径/身份；它不重渲染、不 deliver、不重采浏览器或截图，不更换两类 receipt 身份。只有当前四门全部 pass、审核范围覆盖要求且视觉 passed 才 `deliveryStatus: complete`，其余为 incomplete。候选/HTML/config/engine、delivery/browser receipt、截图身份或实际范围变化都会使相关证据非 current；相同 HTML 摘要不能豁免运行身份。再次 finalize 是新交付，视觉从 required-pending 开始，不能继承上一交付的审核。finalize 自动通过、review-record 接收 failed/unavailable 记录均可退出 0，宿主必须读独立字段而不能把退出码解释为完整交付。

### 3.3 安装/资源格式

VERSION.json 包含 `schemaVersion, distribution:development|candidate|release, tool, toolVersion, npmPackageVersion, sourceCommit, sourceInputSha256, inputContractSchema, receiptSchema, viewerRevision, bundleSha256, engines`。每个平台 engine = `{asset, size, sha256, url?}`；asset 固定 `phoenix-diagram-<platform>`，四平台键与 C-1 相同。probe 精确打印 `phoenix-diagram-engine 0.1.0 input=1 receipt=1 viewer=<viewerRevision>`。大小、摘要或任一身份不同都拒绝。

bundle manifest 用 `schemaVersion:1, tool, version, files:[{path,size,sha256,mode}]`，只允许新 skill 的相对普通文件；不收绝对路径、..、symlink、hooks/settings 或其他工具路径。ENGINE.json 是同一 VERSION 的逐字副本，已装回执绑定其 digest。为避免 VERSION.bundleSha256 与 manifest 中 ENGINE digest 的循环依赖，manifest.files 明确排除派生 ENGINE.json；bundle 根仍必须额外提供该文件，native 逐字核对后单独纳入 ownership。bundleSha256 是 manifest.json 原字节的 SHA-256，不是另造的归一化投影。

项目状态位于 ★ `.phoenix-diagram/install/`：active.json 记 package/tool/bundle/manifest 身份、providers、transactionId 和每个受管文件摘要；journals 记 before/after、进度、备份位置及 pending 操作。阶段为 applying/committed/recovering/recovered；只有 active 提交且全部文件复核通过才 installed。cache 位于 `~/.phoenix-diagram/bin/<toolVersion>/<platform>/<engineSha256>/phoenix-diagram`，可用 `PHOENIX_DIAGRAM_HOME` 隔离；内容地址避免同工具版本的两个构建互相替换。安装器、launcher 及 shell 脚本路径只能由 manifest/安装回执导出，不能信来自 candidate 的执行路径。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| 固定参考 / 旧输入 | 已核实足够 | §1.3 内容摘要；旧五类 schema/renderer 与参考对应文件 | M1 先归档；后续只读冻结来源；旧引擎只作开发 oracle | 来源缺失/摘要变化阻止相关工作，不换 latest |
| UI npm 总安装器 | 已核实缺口 | preparePhoenix 先 cache/probe/dry-run，全部预检先于任何项目写；UI manifest 不能表示 diagram | 新增并列 prepareDiagram / apply；独立错误前缀与状态；不调用 UI engine 作图 | diagram 预检失败时整个总安装尚未写项目；写入期失败逐工具报告，不能宣称总成功 |
| provider 目录与 host | 已核实足够（目录合同）；会话验收未执行 | UI `cli.rs::providers` 的 19 目录与默认展开；本机 claude/codex 入口存在 | 同选择规则；diagram 只写 `.agents/skills/phoenix-diagram` 等实际 skill 位置，Codex 的 .codex 不写 hooks；真实加载由 V-13 验证 | 路径存在不能代替加载；个人副本冲突只报告，不改个人配置 |
| Chrome/Chromium | 已核实足够（API/发现范式）；新工具集成待实现 | UI `browser/src/cdp.rs` launch/new_page/set_viewport/goto/evaluate/screenshot；本机 Chrome 路径存在 | 私有 CDP 子集与 §5.4；`PHOENIX_DIAGRAM_BROWSER` 显式覆盖，不探测后自动下载 | 未找到=unavailable；显式坏路径、启动/协议/检查失败=failed；都非浏览器通过 |
| Reader 媒体/剪贴板 | 已核实足够（源合同）；产物效果待验收 | 参考 Viewer README Export contract、export.js / export-cleanup.js | 保留能力探测、canonical clone、失败终态；真实浏览器测实际 Blob/文件 | 不支持与执行失败分开；只禁对应动作，其余可用 |
| 四平台 CI | 已核实足够（项目内范式）；diagram 制品尚不存在 | 既有 workflow 的 macos-15、macos-15-intel、ubuntu-24.04、ubuntu-24.04-arm | 新 workflow 原生构建、同包消费；M6/M7 取得新 run/commit/digest，不能引用 UI 制品 | 任一平台缺失不得写 P1 完成；权限/runner 不可用记受影响验收阻塞 |
| npm / 外部资产托管 | 已核实缺口（尚无 diagram 资源）；不阻塞本地开发 | UI bootstrap HTTPS/loopback 规则；既有 release 脚本在上传 R2 与发布 npm 前各有确认 | 本期产出同包候选与真实 loopback 在线传输/离线目录验收；未来真实公开地址由维护者在获授权发行时冻结并回读 | 候选不声称公开可安装；缺真实 URL 时发布 guard 阻止整个包，见 §8 |

这些依赖不要求新建外部服务、招募真人或调用付费模型 API。宿主会话本身仍使用既有账号/工具；如其不可用，推进其他开发但 V-13 保留未完成。依赖环境的实际候选与 CI run 只能在产物存在后取证，属于验收前置，不是把设计推迟到实施。

## 5. 核心机制与本地接口

### 5.1 CLI、输入闸与布局

外部命令均为 `phoenix-diagram`；npm 调用形态为 `npm exec --package=@xgent-ai/skills@<候选包版本> -- phoenix-diagram ...`，本仓对应 `node bin/phoenix-diagram.js ...`。参数用数组传给 native binary，不经 shell 拼接。`--help` 和 launcher 自身的版本/恢复指引可在缺引擎时读取，不下载。

| 命令 | 行为与必需参数 | 写入范围 / 成功含义 |
| --- | --- | --- |
| `engine-probe` | binary 输出 §3.3 固定身份；launcher 不让普通内容改变它 | 不写入；供 bootstrap 验证 |
| `doctor --json` | 报引擎/资源/平台/浏览器实际状态、支持图型与固定包恢复动作 | 不安装、不修复、不启动预览；缺任一必要项不是 ready |
| `guide <scenario> --json` | 本地五类路由说明与 schema/example 路径；模糊时列备选 | 不生成虚构事实，不调用模型/网络 |
| `validate <type> <candidate> [--quality standard\|showcase] [--input-contract NAME] [--layout-json] --json` | 输入→结构→场景→HTML 序列化→独立 artifact checks；layout-json 是同一 scene 的稳定可见诊断 | 不提交 HTML；基本 schema 成功不等于 showcase 成功 |
| `deliver <type> <candidate> <output.html> [相同编译选项] --json` | 按 §5.2 freeze/检查/提交，返回 DeliveryReceipt | 仅输出及绑定 sidecar；不含浏览器/视觉通过 |
| `check <output.html> --strict-provenance --json` | 读取当前 HTML、delivery sidecar、原 candidate 路径字节，重算身份并独立检查 | 不重渲染、不改 HTML；缺原输入或回执无法判 current；无 strict 只做基础检查并明示 scope |
| `browser-check <output.html> [--out-dir DIR] --json` | strict 闸后真实浏览器测量 | 写当前绑定的浏览器证据；不生成新 HTML |
| `visual-check <output.html> [--out-dir DIR] --json` | 同 browser-check，并采集固定暗亮截图与索引 | 只供实际图像审核，不产生视觉通过 |
| `finalize <type> <candidate> <output.html> [编译选项] [--out-dir DIR] --json` | validate→deliver→strict check→browser-check；最后一道默认含截图，每次产生新的交付身份 | 汇总自动结果，视觉初始 required-pending、完整状态 incomplete；停在第一道非 pass；返回实际提交事实 |
| `review-record <output.html> --record <record.json> --json` | 按 §5.3 在输出锁内验证当前四门及图像审核绑定、封存记录、条件刷新 summary 并返回完整状态 | 只写本次审核证据及当前 summary；不重新交付或换 delivery/browser 身份，不自动认可截图 |
| `recover-output <private-recovery-directory> --json` | CLI 调用 output::recover_publication，显式恢复 §5.2 的单次输出发布事务 | 只写经验证的原输出槽位并清理本次所有物；不扫描后自动恢复、不改输入或安装，不补通过回执 |
| `engine install --project DIR [--release-dir DIR] --json` | 显式下载/离线取出当前包清单指定的引擎进独立 cache | 不写项目，不推断安装 skill 授权 |
| `install --project DIR --bundle-root DIR [--providers a,b] [--dry-run] --json` | native 只处理匹配 bundle 的项目安装，参见 §5.5 | 受管新 skill 与 diagram 状态；P1 不提供旧安装 migrate 或用户主动 rollback 命令 |

外部退出码：0=该命令的机器结果成功（须回读 scope/自动/视觉字段）；1=输入、语义、artifact/browser 断言失败；2=参数、图型或路径用法无效；3=冲突、busy 或 recovery-required；4=I/O、引擎/资源/浏览器启动与执行环境故障；5=浏览器未安装。机器 JSON 必须包含对应阶段与 diagnostics；不能用捕获异常后空对象或未执行的数组宣称通过。review-record 退出 0 表示记录已验证接收且当前 summary 已刷新，verdict 可以为 failed/unavailable；是否完整仍看 deliveryStatus。恢复的具体结果按 §5.2 映射上述退出码。

输入闸顺序固定：参数/类型和合同→读取确切 UTF-8 JSON→schema→跨集合唯一身份/引用及图型语义→固定几何/结构布局→实际 HTML/SVG 检查。重复身份不能先装进 Map 再被覆盖；实体与关系的显式 id 共用身份域，须检查跨集合冲突。lanes、phases、groups、views 各自保持集合内唯一，引用按所属集合解析；冻结的两版 incident-response 样例允许 phase/node 同名 recover，release-delivery 允许 lane/node 同名 approval，不能把这些合法容器身份与实体混装。unknown endpoints、方向矛盾、mainPath 缺边或失序、无效 activation/segment 范围均指向原 JSON pointer。source fields 的后续期次处置在 schema 后、编译前完成，不绕过核心语义闸。

布局移植的执行顺序为：测量文本/节点及显式 pins→类型专属容器/主路径→自动 ports/routes→labels 与有界 repair→图例和 canvas→完整几何检查→一次 SVG 序列化。workflow/dataflow 的同一 resolved scene 被 validate、layout-json 与 render 共用；architecture 的障碍及边界归属、sequence 的参与者/消息顺序、lifecycle 的主路径/共享出口规则保持源实现的判据和 tie-break。所有 repair 保留节点、关系、文本、note 和来源字段，不用节点数限、重新排序或缩字越过读取下限消除错误。

`check_html` 独立解析实际文档：只允许一个主 SVG，数值有限、路径可解析、正交/显式直连合同、非目标节点穿越、箭头端点方向、相关/无关 corridor、标签/路由/边界/图例遮挡、路由节奏、canvas 与声明的阅读合同。showcase 要求适用的组成检查全部执行且零错误/零警告；standard 保留源合同中的警告和检查范围。回执列具名 checks，不把旧版“固定 4/9 项”的数字当成通过捷径；缺任一适用检查为失败。

序列化严格区分文本、属性和 script JSON：HTML/SVG 文本及属性转义 &、<、>、引号；嵌入 JSON 把 <、>、& 变为 Unicode escape，避免 `</script>`；用户内容不进入 innerHTML、事件属性、CSS 源或可执行 URL。模板 slot 使用字面拼接，不能把 `$&` 等文案解释为替换模式。raw HTML/SVG 与远程 asset 注入不在输入能力内，检测后拒绝。

### 5.2 确定性提交与恢复

1. 在任何输出写入前解析输入与输出位置，比较 real parent、设备/inode 与 hardlink 身份；同文件、symlink 指向输入、硬链接输入或被换父目录均拒绝。新输出允许 native 绝对路径；candidate 的 meta.output 按对应旧/新 schema 校验，但不改写它。输出与 sidecars 不得互相别名。
2. 对候选通过打开的普通文件描述符读取一次，内存字节及 digest 成为本次 snapshot；validate/deliver 共享编译结果，不重新读取一个可能半写的新版本。记录原 candidate binding，提交前再核对；它发生变化则 `input/changed`，不提交。临时源 snapshot 如需落盘仅创建在私有目录，结束按所有权清理。
3. 同目录建立独占 output 锁，锁绑定文件身份并用 OS lock；锁内重读 output/sidecar 快照。把完整 HTML 写入独占临时普通文件，sync、独立检查；HTML 不含时间、随机 ID、绝对源/输出路径或浏览器状态。相同引擎、输入字节、有效配置得到相同 HTML；回执可以有不同 receiptId。
4. 参照 `atomic-output.mjs::publishRegularFileBinding` 实现保守发布：以 fd、device/inode、链接数、mode、digest 绑定旧文件和 candidate；先写恢复记录、核验备份并退役旧文件，再以 no-clobber hardlink 发布新文件，验证 public/stage 身份后只清理本次绑定。后来的目标占位获胜，不使用会无条件覆盖它的 rename。旧文件退役后到新名字可见之间存在短窗口；这是可恢复事务，不承诺多文件原子性或掉电零丢失。
5. HTML 提交成功后再发布 DeliveryReceipt。提交前失败旧文件原字节不变；提交窗口中失败则从已验证备份条件恢复。若新占位/身份变化妨碍恢复，保留用户文件和本次备份，明确 `recovery-required`、实际 public 状态及恢复位置，不能声称旧文件已恢复。receipt 写入失败时新 HTML 可已存在，commitState 必须如实为 committed-unreceipted，strict check 不通过。
6. 浏览器/视觉发生在提交之后：失败不把新 HTML 换回旧版本；旧成功回执因身份不匹配自然失效。故障资源路径只出现在 sidecar/终端，不进入正式 HTML、导出或 Reader。清理失败单列，不用“生成通过”掩盖待恢复状态。

sidecar 以输出名派生，超文件系统组件长度时按完整 basename 的 SHA-256 生成有界 stem，并在回执保留实际输出绑定。delivery.json 与 finalize-summary.json 是当前运行指针，在输出锁与原文件 binding 校对下条件更新；browser/截图/视觉审核为已封存证据，其默认位置被不同运行占用时不覆盖，返回 conflict 并要求新 `--out-dir`。该参数移动浏览器、截图与视觉审核证据根目录，当前 summary 保存其确切链接；同摘要的普通 artifact 也不能替另一份 candidate 的证据充数。陌生 symlink、目录、硬链接及未知恢复记录全部保留，不递归清空用户目录。

**中断后的公开恢复路径：**退役旧 public 前，在其真实父目录独占创建权限 0700 的 `.phoenix-diagram-recovery-<transactionId>/`；其中 `publication-recovery-v1.json` 为 0600 普通文件，记录 `schemaVersion:1, tool:"phoenix-diagram", kind:"output-publication", transactionId`、请求路径与原槽位、父目录及恢复目录的 device/inode、备份 `previous` 的 device/inode/mode/SHA-256/bytes。记录、目录项及已核验备份必须按平台同步后才允许退役旧文件；日志输出不是唯一定位来源。包内文档和 `recover-output --help` 明示：kill 后先用 `ls -la "<输出原父目录>"` 显示这些隐藏目录，再只读查看其中的记录，按 target 路径定位并显式传入完整私有目录；多个候选不自动挑选，未知记录不执行。该定位不依赖 Rust、源码、已丢失的终端输出或成功 sidecar，--out-dir 也不移动恢复记录。

`recover-output` 只接受上述 kind 的输出记录，不接受 install journal；验证目录/记录为本工具支持的私有普通对象及当前用户所有物，核对原父目录/请求路径/槽位、备份内容和物理身份；取得同一 output 锁后重读，链接前后再验证。只在 public 缺失时用 no-clobber link 恢复 `previous`；若 public 已与本备份的物理身份和原字节完全一致，只补完本次恢复的清理，便于恢复过程再次被 kill 后重试。新 claimant、用户改动、备份/记录绑定不一致、替换父目录、未知格式或不明所有权均保留现有内容并停止，不用 rename 覆盖；仅按所有权清理自己的备份/记录。恢复只报告原字节及实际 public 状态，不重建通过回执；任何交付结论仍须严格当前性检查。

清理 record 的最后阶段采用确定的 sibling `.phoenix-diagram-recovery-<transactionId>.cleanup.json` witness：已核验 record 原字节以 no-clobber move 退役到该位置，清空原目录后再将同一 inode 移入固定私有 `.phoenix-diagram-output-cleanup/<transactionId>.json`，核原 fd 后 unlink，目录项每步同步。recordIdentity 绑定原文件 device/inode，checksum 绑定 checksum 置空后的紧凑 JSON。固定目录为当前用户 0700，只有独占创建时可写入 0600、单链接的 `owner-v1.json`；owner 绑定 canonical parent、parent/dir/self device/inode 及 checksum。既有空目录不认领，陌生条目/owner 或跨位置歧义保留拒绝。有效 owner 的空壳保留，不递归删除。

显式 recover 参数始终为原 `.phoenix-diagram-recovery-<transactionId>`，即便它已不存在；仅由该路径导出 sibling witness 与固定 quarantine 中同 ID 的记录，不扫描选择事务。原目录不存在但任一可信记录尚在时继续验证与清理，不能直接返回 absent；未知、替换或不一致对象保持原样。固定目录中除 owner 外只允许 32 位小写 hex ID 的 `.json`，每份仍须核验自身/parent/owner/权限/单链接/内容绑定；仅剩有效 owner 不算 pending。包内说明与 help 同时说明这三种残留位置。

`OutputSlot::pending_publications` 在原 output 锁内只读检测 bound parent 中的恢复 record/witness：可信且属于该槽位的未完发布阻止 strict pass，可信不同目标不阻塞；未知标准前缀对象 fail-closed 并保留。HTML、delivery receipt 与当前 summary 的发布/清理状态均必须 settled，不能因新 receipt 字节已可见就补造通过；查询本身不执行恢复。

| recoveryStatus | 结果 / 退出码 |
| --- | --- |
| recovered | 原字节已恢复或已核对为本次恢复结果，且本次清理完成；0 |
| absent | 指定恢复目录及其对应可信 witness/quarantine 记录均不存在；重复恢复可幂等退出 0，不据此声称当前 HTML 已通过 |
| preserved / recovery-required | 存在后来占位、身份/记录不可信、busy 或恢复/清理未完成；3，列 publicState、保留文件和恢复位置 |
| invalid | 参数或不支持的记录格式；2，不执行不明恢复 |
| failed | I/O 或执行环境失败；4，保留不确定文件并列实际状态 |

### 5.3 四门与证据传播

`finalize` 只使用内部 Rust 调用，不通过四次进程读取不同 candidate。validate 返回当前 snapshot 的 checks；deliver 提交相同 HTML 与新 receiptId；strict check 复核 candidate/config/engine/artifact 和具名检查；browser-check 接收这份 BoundArtifact。每道门开始与结束都核对实际文件，检查范围/identity/receiptId 也必须与预期一致。伪造、缺失、旧 sidecar、仅基础检查或数组少一项全部阻止 automated pass。

审核后的收口固定为 `finalize → 实际查看当前截图 → review-record → 读取其返回值及当前 summary`，不再次调用 finalize：

1. `record_review` 取得该 output 锁，重读当前 summary、delivery、browser、candidate/HTML/config/engine、截图及原文件 binding，核对原四门的结果、具名检查与范围，审核记录必须精确指向同一 deliveryReceiptId 和 browserReceiptIdentity。缺失/漂移/不完整范围不能接收为当前通过，也不能补写 not-run 门。
2. 校验审核者、实际观察记录与所看文件，按封存规则登记；同路径同身份的同一记录可幂等重试，其他内容返回 conflict。在写 summary 前再核对绑定，只更新视觉结果/记录链接、deliveryStatus 和对应诊断，保持原四门证据与 delivery/browser 身份。返回 §3.2 的完整状态及写后 summary 身份。
3. 登记成功但 summary 写入失败时如实非零并保留已封存记录；恢复相应 I/O/冲突后可重试同一记录完成汇总，不能改称完整通过。任何输入、产物或证据变动均须取得适用的新证据；新 finalize 不继承旧审核。

第一道失败后，其余阶段为 not-run。如果失败时旧输出仍存在，摘要明确其身份和 `producedByThisRun:false`；不对它补做浏览器检查。已提交后 browser 缺席、执行失败和视觉未审核分别列出。最终答复保留 HTML 可用性、自动/浏览器/视觉结果及必要限制，详细摘要和修复线索由回执承载。

### 5.4 浏览器测量与资源

发现顺序为 `PHOENIX_DIAGRAM_BROWSER` 显式路径→Chrome 常规路径；显式路径失效是配置失败，不回退成未安装。Chrome 使用独立临时 profile、headless、临时 DevTools 端口，默认保留 sandbox，不访问现有登录 profile、不下载浏览器。启动与 CDP deadline 沿源合同上限记录在 BrowserOptions，不能无限等待；新脚本的默认上限固定为启动 90 秒、单条 CDP 15 秒，错误包含具体阶段。退出或超时后关页/杀本次 child、清理本次 profile；无所有权证据不清理其他 Chrome 或目录。

通过 file URL 读取确切 HTML，在 fonts.ready 与两次 animation frame 后执行内嵌 probe。覆盖四个规定桌面尺寸、light/dark 两主题；测量 document 溢出、SVG/panel/canvas、投影文本、label/卡片遮挡、主控件 hit rect、JS errors、资源请求及媒体状态。截图默认覆盖 1440×900、2048×1320 两端的暗亮主题；可读纵向图使用完整页面截图，不能只截首屏隐藏下半图。另以 390×844 作为本期选定的窄屏检查样本，不宣称所有移动设备兼容。

水平溢出总失败；纵向例外严格沿参考 delivery-contract 的 reader-fit 声明和测量：intrinsic-height/width-first 自动 canvas 及 architecture 的 authored-height，各自要求内容完整、可读、无内部滚动/裁剪且页面正常流动。回执分别写 verticalScrollAccepted 与 overflowDisposition；不能给任意显式 viewBox 自动加白名单。测量不能证明美观，截图不能证明已被审核。

### 5.5 npm 获取与项目安装

总安装器新增默认启用 diagram 的分支，提供 `--no-phoenix-diagram` 与 `--diagram-release-dir=DIR`；`--no-phoenix-ui` 和原 `--release-dir` 继续只控制 UI。`--providers` 共同适用但每个工具独立报告。选项、guard/XGENT 配置、UI 和 diagram 的全部预检完成后才执行任何项目写入；diagram 被跳过时仍校验 provider 参数，且不删除有效安装。

bootstrap 按 §3.3 校验 manifest、平台、bundle、大小、SHA-256 与 engine probe；普通命令仅查 cache/active，缺失时退出 4 并给固定 npmPackageVersion 的 `phoenix-diagram engine install` 指引。显式 engine install 才允许 HTTPS 下载；loopback HTTP 只作候选传输验证；不接受凭据 URL、非本机明文 HTTP 或不受限重定向。复制 UI bootstrap 已有的代理、超时、重定向和有界下载行为，换独立命名空间；新文件头标来源与修改，保留其测试边界，不抽取公用下载框架。

`--diagram-release-dir` 下 VERSION.json 必须与包内清单逐字一致，并含完整四平台资源/许可/清单；当前平台引擎仍重算大小、digest、probe。目录缺资源失败且零网络，不回退在线。先 staging 到内容地址 cache、核验后可见；包资源和 cache 均验证，不能仅因 cache 已有文件就信任。Node 18 的 smoke 必须实际调用这些消费文件。

Rust `plan_install` 解析 §3.3 白名单并确认项目/provider；默认规则复制 UI 的 19 项列表，不重扫业务源码。`codex` / .agents / .codex 的选择语义沿 UI，实际 diagram skill 只进 .agents，.codex 不新增 hook。项目相同 managed bytes 返回 unchanged 的前提是原 ownership 的物理身份仍 current（device/inode/links/mode/uid 与 digest/bytes 均匹配）；同字节的原子重写、新 inode 或无有效 diagram ownership 的同名文件均不认领。用户修改、symlink、未知父目录和额外文件返回 conflict，预检零写入；Node launcher 与 native 使用同一判定。

`apply_install` 持本项目 diagram OS 锁后重做 plan，备份与 journal 先写，逐文件条件替换，active 最后写；不改 settings/hooks/AGENTS/个人安装或 UI 状态。新安装中断后的下一次显式 install 先做条件恢复：只撤销与 journal after 摘要/文件身份仍匹配的本次写入；后来修改一律保留并报冲突，不能用 before 盲覆盖。active 已落地且全部文件匹配时补全 committed 状态。只读 dry-run 遇未恢复事务返回 recovery-required，不自动写日志。候选附本地忽略说明，不在本期新写共享 .git/info/exclude。

总安装器仍分段提交：XGENT、UI、diagram 按序执行，diagram 写入失败时前段可能已成功，必须逐项报告已完成/待恢复，不反向回滚其他工具。新安装报告含 tool/package/目标/provider、writes、conflicts、transactionId、journal/backup、重载步骤和重跑恢复命令；不能给尚未实现的 P3 rollback 命令。P1 不检测后自动迁移/删除 archify；只提示它仍为旧入口。

### 5.6 构建、四平台候选与同包消费

维护路径分为 prepare、build、assemble candidate、consume；不存在隐式联网的 build。`prepare-phoenix-diagram.mjs --prepare` 预检 C-1 及 Cargo.lock，显式 fetch 并记录依赖/浏览器来源，之后冻结准备回执。浏览器 pin 采用本仓 UI 的固定维护来源与校验方法，diagram 的实际 browser binary/version/digest 单独记录；不会把 UI 运行证据当 diagram 已通过。

`build-phoenix-diagram.mjs --development --output-dir DIR` 在目标原生 host 使用 mbx release、`--locked --offline --message-format=json`，从 compiler-artifact 读取真实 binary；检查编译前后源码输入摘要未变，记录 sourceCommit/tree/state、全部输入摘要、Rust/Node/mbx、目标、binary/bundle/viewer 身份。禁止把 dirty build 记成已提交来源。生成新技能 ENGINE、bundle 与 VERSION，单平台清单只能为 development；不改 UI dist。

浏览器准备来源具体沿 `tools/phoenix-ui/build-tool-assets.json` 的 linuxArmBrowser 与现有 Chrome 发现/版本记录方法；读取固定 pin，diagram prepare 独立校验，不运行要求 WASM 的 UI preflight。

新 workflow 使用与现有 UI 相同的四个原生 runner：macos-15/aarch64-apple-darwin、macos-15-intel/x86_64-apple-darwin、ubuntu-24.04/x86_64-unknown-linux-gnu、ubuntu-24.04-arm/aarch64-unknown-linux-gnu。每个实际 native build/run/upload，禁止交叉编译加文件存在性替代运行。Action 版本先复用当前 UI workflow 的固定 SHA，不加未核对 latest；上传独立 diagram artifacts 和真实 phase 结果。

四份相同 HEAD/tree/源码输入及 bundle 的 native artifact 齐备后，assemble job 调用 `pack-phoenix-diagram-candidate.mjs --artifacts-dir DIR --output-dir DIR` 生成唯一 tgz 与完整 release-dir，distribution=candidate，四平台清单齐全。包版本取当前 package.json，不在本期决定公开 release 版本。使用 staging tree 生成 ENGINE/VERSION，只把 diagram 派生字段写进暂存包；UI 及其他工具的包文件按输入原字节保留，不能借 candidate pack 改它们的安装身份。源码 archive、Node oracle、Rust target、node_modules 和凭据不进消费包。

候选的在线路径用真实本地 HTTP 传输，不 mock 网络库、不换假引擎：assemble 时仅为该候选选定一个 loopback 测试端口，冻结四个本机 URL 和 manifest，写入唯一 tgz。各平台 consume job 下载同一 tgz/release-dir，在 manifest 指定端口提供其真实 engine 字节，空 cache 在线安装；离线场景关闭该服务、禁止 IP 网络并提供相同清单的完整 release-dir。端口冲突为环境失败，不偷偷改 URL/manifest。该 URL 仅证明候选在线获取路径，不能当公开托管验收；发行时必须用真实 HTTPS URL 重新冻结/验收。

consume jobs 四个平台都从 tgz 中运行 npm bin→bootstrap→native install→五图 finalize/浏览器/导出，不从检出源码执行消费路径。四平台图解候选安装统一显式传 `--no-phoenix-ui`，在线入口为 `node <包目录>/bin/xgent-skills.js install <临时项目> --no-phoenix-ui --no-xgent-init`，离线再加 `--diagram-release-dir=<完整发行目录>`；docs 的候选快速入门使用同一命令。原因是当前随包 UI 清单只有一个 development 平台，不能把保持原字节的 UI 资源宣称为新的四平台统一发行。这个 skip 只限定候选验收对象，不改变总安装器的默认选项；默认两工具共装的接缝在 V-11 使用匹配的真实 UI engine fixture 单独验，正式联合公开发行仍须 §8 的两个工具同包门。开发机本地脚本可在空临时前缀解包同一 tgz 并执行 `node <解包目录>/bin/xgent-skills.js install <临时项目>`；测试需验证包内 bin 映射与 `npm exec --package=<tgz> -- phoenix-diagram`，npm 安装的 registry 访问与已装工具零网络运行分别记录。Node18 consumer 按现有 UI workflow 的 18.20.8 消费基线另验。原生无 Rust 消费由隔离 PATH/环境证明，而不是只看 README。最终 tgz 必须包含 docs/phoenix-diagram.md；V-15 仅凭包内说明/--help 经 npm 入口完成 §5.3 的审核后收口和 §5.2 的 kill 后定位/恢复，不能调用检出源码或私有 Rust helper。

为保实际消费环境无检出源码，consume 与 Node18 jobs 不执行 checkout；assemble 从同一源码生成单独的维护 harness 并绑定其真实 tar SHA/GitHub artifact digest。harness 只含 Node 验收控制器、浏览器准备 helper/pin 和必要许可，不含 Rust/src/Cargo/target、Node renderer 或上游 archive，不进入 npm 包。candidate 根另附四原生构建共有的 canonical source-inputs.json 并纳入 files；完整 frozen archive/source gate 在 native/assemble 执行，consumer 仅核 canonical tuples、四 build receipts、assembly 的实际 artifact/归档/tgz 摘要和 commit。实际应用命令只用 tgz 的 npm bin/已装 launcher，项目/cache/home/PATH 独立，PATH 无 rustc/cargo；Chrome 保自身 sandbox，网络禁用与 CDP 事实单独记录，不为拒读源码给 macOS Chrome 叠加不兼容的 sandbox-exec。

四 native consumer 的自动步骤完成后先上传 review-request 中间证据，handoff 保持 pending-review，原 job 与文件系统继续存活。维护者实际读取每张绑定图片后，将 JSON 记录通过本仓专用 `phoenix-diagram-review/<runId>` 分支的 `reviews/<target>.json` 提供；这只是验收数据通道，不发布 npm/R2 或产品页面。外层 checker 以只读 gh API 最多等待 90 分钟，核 repository/runId/target/nonce/state SHA/candidate SHA/sourceCommit 和每条原绑定，写本地独占记录，经包内公开 review-record 在同一 delivery/browser 身份收口。不能结束 job 后复制文件到新机重建物理身份、不能再次 finalize 原图来补审核；超时/缺审核保留未完成。Node18 job 单独报告自动范围，不能据此宣称视觉/provider 或整期通过；凭据不进入状态、归档或应用子进程。

## 6. 作者与阅读器交互

### 6.1 宿主创作与新 skill

新 skill 的描述按 C-3 先写五类触发及数值图表/仪表盘排除，再写 HTML/检查/导出结果。body 的最短主路径为：选择图型→读匹配 schema/common/example→从授权材料写完整 candidate（新稿写 input_contract）→一次 finalize→按诊断修复→实际图像审核→review-record 并回读同一交付的当前 summary→简短交付。只为登记审核/取得完整状态不得再次 finalize；候选修复后则是新交付，重新取得适用证据。已给 candidate 先 finalize；不要求读 renderer 或扫描全仓。无 Agent 的用户可直接走 JSON CLI。遇输出 recovery-required 时说明实际 public 状态、私有记录定位与显式 recover-output 命令，不自动选事务恢复，也不把文件恢复当交付通过。

Mermaid 是宿主的语义输入，不新增 Rust Mermaid parser：flowchart/graph 映射 workflow 或组件 architecture；sequenceDiagram 保留参与者、消息方向及同步/异步；stateDiagram 映射 lifecycle 并保留主路径、等待与恢复边。schema 不支持或事实不明时报告缺口，不造参与者、条件、日期或金额。示例结构不能成为私有事实。保留用户主要语言和代码/产品标识；P1 不声称五种界面语言已验收。

修复依据 diagnostic subject/evidence/supportedFixes，只编辑相关布局邻域，语义清单、来源与必要文本保持。维护历史最佳客观错误数；连续两轮不优于历史最佳即停止并列未解决项。同一失败门保留可比较的历史；前门实际通过后首次进入后门属于进展，但重建交付、返回已记录门或改变错误种类均不重置该门历史，不给环境失败无限重试。视觉纠正按实际观察，改 candidate 后旧证据失效，重跑受影响检查并采集新证据。缺工具/图像能力时只如实交付可用文件与未完成门，不安装、不手工造一份被称为 Rust 校验通过的替代图。

### 6.2 阅读界面与状态

沿参考 Reader 的标题+主图、按需菜单/图例、可关闭详情；不添加默认 subtitle、填空卡片或帮助用户作图时才需要的内部实现说明。Reader 保留主题、classic/其他已有风格、pan/zoom、按标签/稳定 id 搜索及焦点详情。切换不改变已著坐标/关系，empty/no-match/无选中状态有准确反馈；键盘可到达主控件并退出详情。P1 沿用 en/zh-CN 可用界面文本和明确英文 fallback；打包时不破坏参考的其他 catalogs，但完整语言质量留 P2。

初次没有保存播放偏好为 Still，用户点击 Live 才保存选择；localStorage 使用 Phoenix 命名空间，不读取 Archify 偏好。prefers-reduced-motion、hidden、print/export 只暂停临时动作，静态图必须保留全部含义；返回页面按当前偏好恢复。字体从参考内嵌 OFL 子集供给，未覆盖字符使用系统 fallback，不为 CJK 字体触发网络。旧 HTML 的原偏好和阅读器原样保留。

页面失败包括未初始化、资源异常和不支持动作；任何情况下主体静态 SVG 与文字仍可读。视觉审核要实际查看暗亮截图中的主图/标签/卡片、关系遮挡、层级与控件；不能把 DOM 测量、截图存在或测试输出当成审核结论。

### 6.3 canonical 导出与失败终态

移植 `local/archify-main/viewer/export.js` 与 `export-cleanup.js`；维持 classic-script closure 和初始化次序。导出用私有 SVG clone，包含全部节点、关系、完整 notes、图例及选定视觉风格；清除相机 transform、搜索/焦点/路线/Reach/Lens/临时播放叠层与 runtime controls，不能直接导出被当前 filter 隐藏的残图。双主题 SVG 内嵌两主题 CSS/字体；PNG/JPEG/WebP 锁当前主题/风格；WebM 使用完整静态背景，动画不改变拓扑。录制完成或失败后恢复原探索/暂停状态。

每个动作先 capability probe，再执行；实际结果以 Blob 字节/MIME/签名/尺寸与可解码性核对，不能把文件后缀当格式。PNG 复制只在真实 user gesture 内创建 ClipboardItem/write；不支持或权限拒绝提示可下载 PNG，不伪造复制成功。编码异常、toBlob=null、空 chunks、MediaRecorder error/超时/取消均进入独立终态 failed/unavailable/cancelled；后来 stop/dataavailable 不能覆盖失败终态，也不能再下载空文件。只有非空且格式真实的结果才 success。

每次动作清理自己的 Blob URL、anchor、image handlers、tracks、timer 与 recorder listener；有 finally，但不能关掉其他用户播放或删除作者源文件。保留参考既有真实导出接口，Route/Reach 卡片不是 P1 验收对象；其存在不扩大为 canonical 完整图。状态由 toast/live region 和对应按钮呈现，只禁用不支持格式，其他格式继续可用。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N1 | PRD 的四桌面尺寸与默认真实审核；参考阅读合同 | 四尺寸内容完整、无水平溢出/遮挡；纵向例外必须有声明和可读测量 | 精确指出尺寸、主题和对象；无法审核保留未完成 | §5.4、§6.2；不缩字/隐藏溢出求通过 | V-7、V-9、V-15 |
| WORK-N2 | PRD Rust 核心；当前包/模板版本 | normal path 不运行 Node 图解引擎；来源和实际许可可追溯 | 身份不匹配拒绝；许可缺失不生成可交接候选 | 编译期内嵌资源、manifest/probe/来源清单，§1.3 | V-1、V-9、V-12、V-14 |
| WORK-N3 | 两份实际 schema/renderer 的默认值差异 | 五类旧核心、稳定 id/方向/标签及显式几何保留；旧 JSON/HTML 原字节不变 | 缺口明确且不能计兼容通过 | legacy 分派、字段处置、几何/语义 oracle 与固定 pins | V-2–V-5、V-9、V-15 |
| WORK-N4 | PRD 确定性与旧成果保护 | 固定引擎/输入/配置的 HTML 相同；失败及恢复不覆盖后来的修改 | failed/conflict/recovery-required 分开，列真实保留文件 | §5.2 no-clobber、file binding、持久记录与公开 recover-output；§5.3 原身份收口 | V-6、V-9、V-10、V-15 |
| WORK-N5 | PRD 离线、安全内容；参考 esc/serializeScriptJson | 用户内容只作内容；无网络生成/检查/阅读，无跟踪/上游更新 | 不支持/恶意字段拒绝；外部请求作为测试失败 | schema 资源关闭解析、文本/script JSON 转义、内嵌 Reader/字体、deny-network | V-2、V-7、V-9、V-12、V-15 |

诊断与可观测性止于本地结构化回执、stderr、安装 journal 和浏览器证据；不新增日志服务、遥测或后台轮询。回执不输出账号凭据或完整私有源码；路径按本地证据所需最小范围记录，不嵌入分享 HTML。

性能没有已确认的延迟、容量和包体积 SLO。沿参考的有界路由/label repair 和缓存上限，不增加 speculative parallelism；先用五类代表小图及密集/长标签 case 反复测量，测试报告实际 elapsed/峰值资源与输入规模，不编造 p95。若有界搜索失败，保留完整数据并给布局诊断；不得截断节点、终止后假称通过或为不可读图强上新基础设施。

## 8. 失败模式、候选交接与公开发布门

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现/降级 | 检测 | 恢复/补偿 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| schema/引用/布局失败 | 当前 candidate | 不写输出；旧 HTML 保留 | 指明对象/支持修复；后续门 not-run | input/layout/artifact diagnostics | 作者另存或定向修复，不删事实 | V-2–V-6、V-9 |
| 输出别名、竞争或提交中断 | 单个输出及其证据 | 新稿不发布或留下可定位的绑定备份；后来的修改保留 | conflict/recovery-required，明确当前 public identity | inode/digest/slot/持久记录复核 | 按 §5.2 包内说明定位私有目录，经公开 recover-output 条件恢复；不确定即保留备份和用户文件，不盲覆盖 | V-6、V-9、V-15 |
| HTML 已提交，receipt/浏览器/视觉或审核后 summary 写入失败 | 当前已提交图 | 新 HTML 可能可读；缺完整证据；已封存记录可仍存在 | committed-unreceipted、browser failed/unavailable、visual failed/pending 或 summary 更新失败分列 | strict provenance、真实 browser、实际图像观察及 summary 写后身份 | 修复对应门；仅 summary 写入失败时可重试同记录收口；不拿旧通过回执补证，不擅自退回旧图 | V-6、V-7、V-9、V-15 |
| cache/manifest/平台不匹配 | 当前机器当前工具 | 项目预检不写；UI 与旧 diagram 安装保留 | 固定包恢复指引，普通命令非零 | size/SHA/probe/bundle/schema 校对 | 用户显式匹配版本 engine install；离线缺资源不回退网络 | V-10、V-11、V-12、V-14 |
| 安装中断、受管文件漂移、并发 install | 一个项目的 diagram | active 最后写；中断可部分落盘；其他工具不回退 | recovery-required/conflict 并列已完成工具 | OS lock、before/after、active/journal 校验 | 下次显式 install 条件恢复，后来修改保持，未知文件保留 | V-10、V-14、V-15 |
| 格式不支持、权限拒绝、编码/录制后到事件 | 单次读者导出 | 不改源图；可能只缺该格式 | unavailable/failed/cancelled 与可用替代动作 | capability、实际 Blob/签名、终态 guard | 清理本动作资源、恢复探索，允许其他格式再试 | V-8、V-9、V-15 |
| 缺原生 job、provider 未加载、来源许可不全 | 候选交接 | 不损坏旧包，不产生虚假完成 | 该验收未完成，P1 不得完成 | CI 原始结果、真实会话、源码/许可清单 | 补实际资源/证据，不以文件存在、stub 或其他平台代验 | V-12–V-16 |

候选交接顺序：冻结参考/许可→完成核心收口→安装/包资源接入→同 HEAD 四平台 native artifacts→唯一 npm tgz/release-dir→四平台消费、真实 host 与图像审核→最终进度/提交。先前证据只有在其实际覆盖的字节/契约未变且绑定仍 current 时可复用；不能把 M4 的开发 binary 回执直接算成最终包验收。

公开发布属于后续显式授权动作。P1 的 candidate 交接不运行 `npm publish`、R2 上传或当前全局安装；未来公开发行仍须：固定非 candidate 清单及真实 HTTPS URL，上传后逐资源读回摘要，使用最终同一 npm tgz 在四平台在线/离线消费，验证 registry/tarball/bin 与两个 Phoenix 工具的资源身份，最后才批准 npm 发布。不得把 loopback 传输或 development 单平台声明为这个门通过。

共同 `prepublishOnly` 必须从实际 package.files/bin 检查每个随包 Phoenix 工具：只要 diagram 存在，就要求其 release 清单四平台齐全、所有公开 URL 为 HTTPS、bundle/包版本一致；缺清单或 candidate/development 均拒绝，UI-only 的旧 fixture 补齐真实 package.json 后继续按其原范围验。`npm publish <tgz>` 不执行 checkout 的 prepublishOnly，所以 `scripts/release-phoenix-ui.mjs` 还须在 Checkout 后、创建发行状态与任何 R2/npm 写入前显式检查：同包含 diagram 且尚无联合发行合同即停止，给出候选与公开发行的区别。保留现有 UI-only 路线，不尝试替 diagram 上传或发布；真正的联合发行流程另获授权后扩展，不能用候选 tarball 绕过它。

本期的回退是撤销仓库内新工具接入、恢复共同 npm 文件到原行为并保留独立生成物；临时项目只按本次 ownership 清理。本期无 schema 数据迁移或线上切流。候选不覆盖旧 JSON/HTML；项目的跨版本迁移和用户主动 rollback 在 P3 定义，不能拿删除整个 .phoenix-diagram 目录作为恢复步骤。

## 9. 验证

所有命令从仓根执行。★ 脚本/测试由对应里程碑交付；本节给出它们的必需 CLI 合同，不能换成“人工看一下”。Rust 测试通过 native lib/bin 的公开行为与真实临时文件观察结果；oracle 只在开发测试使用冻结 Node 来源，不进包、不成为 normal path。正例按语义/几何/可读性验证，不要求新旧 HTML 像素或字节一致。

### 9.1 检查定义与唯一执行时点

| 检查 ID | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | WORK-N2、C-1：冻结源码可从归档恢复并重算 §1.3 摘要；许可、依赖锁、维护工具与离线构建前置就绪 | `node scripts/check-phoenix-diagram-source.mjs --check`；先显式 `node scripts/prepare-phoenix-diagram.mjs --prepare`；检查模式不写源码、不换来源 | M1 局部；解除后续移植前置 |
| V-2 | WORK-F1/AC-3、WORK-N3/AC-1、WORK-N5/AC-1：五类旧/新 schema、合同选择、重复身份/跨集合冲突/非法端点/未知类型、Unicode/注入及延期字段均有真实结果，输入不改 | `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test input_contract`；语料=冻结旧 examples + 每个支持字段/省略值/边界组合；旧 schema oracle 预生成可核对断言 | M1 局部；输入合同稳定后解锁布局 |
| V-3 | WORK-F1/AC-1、WORK-N3/AC-1 的 architecture：边界、职责/方向/标签与显式几何保持；showcase/standard checks 有实际范围 | `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test architecture`；对照旧/参考具体合同，含 free/grid/pins/长标签/反向边界 case | M2 局部 |
| V-4 | 同上 workflow：v1 固定/v2 可读、泳道/条件/回路/mainPath、route/label pins、失败布局回执不改事实 | `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test workflow`；冻结旧/参考 compiler fixtures，按合同分组 | M2 局部 |
| V-5 | 同上 sequence/dataflow/lifecycle：次序/同步异步/note/数据标签/恢复边、旧默认 fixed/v1 几何、新 v3；全部语义与 pins 可追溯，合法旧核心不被新 schema 拒绝 | `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test remaining_types`；包含未显式 column_fit 的旧 sequence 和旧 deployment-release.lifecycle | M3 局部 |
| V-6 | WORK-F4/AC-1、AC-3、AC-4、WORK-N4/AC-1、AC-2：阶段顺序/短路、确定性、alias/receipt 失配、提交/清理结果；review-record 原身份汇总、幂等重试与 summary 写失败分支；recover-output 参数/状态/退出码及绑定拒绝 | `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test delivery`；真实临时 FS，经公开 CLI/模块行为观察；实际审核及竞争/kill 的收口演练另见 V-9 | M4 局部 |
| V-7 | WORK-F4、WORK-N1、WORK-N5：测量/capture/不可用/执行失败与身份失配不混淆；无意外网络；只清理本次浏览器 | `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test browser_contract`；私有 CDP fixture 测分支，真实效果由 V-9 验 | M4 局部 |
| V-8 | WORK-F5、WORK-F6：Still/Live 状态机、canonical clone、MIME/空 Blob/MediaRecorder 后到事件终态、资源释放 | `node --test tools/phoenix-diagram/tests/viewer/unit.test.mjs`；在原 JS API 上做有区分力的边界检查，不用模拟 DOM 宣称真实导出 | M4 局部 |
| V-9 | 核心阶段收口：五类真实生成→四门→四桌面/窄屏→实际暗亮审核→review-record 原身份 complete→全部导出；旧核心不丢失；确定性/竞争/kill 后经公开 recover-output 恢复/恶意文本/离线故障演练 | `node scripts/test-phoenix-diagram.mjs --stage core`；真实 Rust+Chrome，脚本产出每图证据索引；Agent 逐张调用图像读取工具查看四张暗亮截图及必要长图，经 review-record 登记并核对 §9.3 身份/状态；恢复同按 §9.3。测实际 PNG/JPEG/WebP/SVG/WebM 文件、剪贴板成功及能力/权限失败；离线用 OS 网络禁用与 CDP 请求记录 | M4，阶段 A 收口；不以 V-7/V-8 代验 |
| V-10 | WORK-F11/AC-1、AC-2、AC-5：bootstrap manifest/size/SHA/probe、四平台选择、缓存隔离；install provider/幂等/冲突/预检零写/中断恢复 | `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test install`；`node --test test/phoenix-diagram/bootstrap.test.js test/phoenix-diagram/install.test.js`；临时项目/cache，fake probe 只测拒绝分支，成功路径用真实引擎 | M5 局部 |
| V-11 | C-2、C-3、WORK-F11/AC-2、AC-5：两工具选项互不混用、缺 diagram 不写项目、skip 不删除；新 skill schema/examples/描述边界与调用一致；archify/调用方未被提前替换 | `node --test test/phoenix-diagram/package.test.js test/phoenix-diagram/skill.test.js test/xgent-skills.test.js test/npm-entrypoint.test.js`；`node scripts/test-phoenix-diagram.mjs --stage install` 按下文运行 UI 安装局部组；原专项测试只加 scope 必需的 skip/bootstrap fixture 适配；核对 §0.6 全项 diff | M5 局部 |
| V-12 | WORK-F11、WORK-N2、WORK-N5：独立 build/pack 的来源/四平台完整性/许可证/包内文件、Node18 消费入口、错误资源拒绝和 publish guard | `node --test test/phoenix-diagram/build.test.js test/phoenix-diagram/candidate.test.js`；`node --test test/phoenix-ui/release.test.js`；固定 artifact fixtures 测少平台、异 commit/bundle、坏 digest/许可/候选 URL，不能据此宣称平台通过 | M6 局部 |
| V-13 | WORK-F2 全部 P1 AC、WORK-F11 provider、C-2：真实新会话加载且自然意图/三类 Mermaid/旧 JSON/缺事实/修复停滞与默认审核行为正确 | 临时无 Rust 项目用最终 tgz 安装；分别在已登录 Claude Code、Codex 的全新项目会话发起 §9.2 场景。仅提供包内 skill/schema/example 和题目材料；Agent 记录真实会话入口、工具版本、实际候选/产物/缺口；不从历史上下文提示答案 | M7，阶段 B 收口；普通 provider 目录测试不替代 |
| V-14 | WORK-F11/AC-1、AC-2、AC-5：同一最终 tgz 四平台原生消费，在线真实传输/缓存/零网络离线、坏引擎/不支持平台/预检失败，消费环境无 Rust；UI 和 diagram 状态互不改写 | `.github/workflows/phoenix-diagram.yml` 的四个 consume jobs 与 Node18 job；统一入口 `node scripts/check-phoenix-diagram-candidate.mjs --candidate-dir DIR --target PLATFORM`，DIR/PLATFORM 使用 workflow 当前真实值；验证步骤见 §5.6、§9.3；保存 run/commit 与 artifact digest 链接 | M7，阶段 B 收口；M6 只交付构建/装配能力，不提前宣称四平台通过 |
| V-15 | P1 全部 23 条 AC 的最终包业务链路：所有五类从 npm 入口生成、审核后原 delivery/browser 身份的 complete 汇总、读者操作和真实导出、双主题Still/Live下reduce/hidden八样本的真实暂停/恢复与静态事实保留、内容完整/离线/XSS/故障结果；无 Rust/源码时依包内说明完成 kill 后公开恢复与幂等/保护断言 | `node scripts/test-phoenix-diagram.mjs --stage distribution --candidate-dir DIR`；从最终 tgz 运行，Agent 读图后调用 review-record；依 §9.3 经 npm 入口 recover-output，验证说明文件实际在包内；原语义检查引用 V-2–V-5 的未失效断言。旧/new JSON 输入摘要、实际文件格式和各阶段状态逐项核对 | M7，阶段 B 收口；final binary/bundle/入口已改变，开发制品证据不得直接替代 |
| V-16 | C-3：同包接入未破坏仓库 Node/Python/metadata 与 Phoenix UI runtime；fmt/clippy、锁定依赖和干净构建实际成功 | `npm test`；`npm run test:phoenix`；`npm run test:diagram`（新增脚本执行全量 diagram Rust/静态检查，真实 browser/export 用 V-14/V-15 的当前证据）；`mise exec --no-deps -- mbx fmt --manifest-path tools/phoenix-diagram/Cargo.toml -- --check`；`mise exec --no-deps -- mbx clippy --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --all-targets -- -D warnings` | M7，阶段 B 收口；全量仅在此收口执行 |

V-11 的 install stage 仅构建并测试受共同入口影响的 UI 安装组：先复用既有 UI prepare 环境；以 `mise exec --no-deps -- mbx build --manifest-path tools/phoenix-ui/Cargo.toml -p phoenix-ui --bin phoenix-ui --locked --offline --message-format=json` 读取 compiler-artifact 的真实 executable，临时复制固定 binary，将该路径传为 `PHOENIX_UI_BIN` 后执行 `node --test test/phoenix-ui/install.test.js`。不改 UI manifest/源码、不运行全量 UI runtime；环境未准备时显式运行既有 `scripts/prepare-phoenix-ui.mjs --prepare`，不能把缺 engine 的跳过记为通过。共同入口的 diagram bootstrap 必须在启用分支才读取 diagram manifest，既有 UI-only fixture 在明确 skip 后仍可运行。

### 9.2 角色模拟与图像审核

角色模拟由 Agent 执行，结果标明“角色模拟”，不推断真人成功率。首次作者只知道自己的请求和提供材料，允许读已安装 skill/schema/example/快速入门，不允许读 renderer、测试断言或实现者补充说明；安装使用者只知道平台和候选安装说明，不具备 Rust 知识。

真实 host 场景至少包括：自然语言组件/审批/调用/数据/状态五种意图；带分支/回路的 flowchart 与 graph、含 return/async 的 sequenceDiagram、带 waiting/recovery 的 stateDiagram；五类旧 JSON；缺日期/金额/条件材料；可修复诊断及连续两轮无改善；浏览器 unavailable/failed 与视觉未完成。期望行为沿 WORK-F2、§6.1，不用对话里透露评分词。发现缺事实的提问是正确结果，不能为跑通流程补造事实。stall fixture 使用预设候选与真实诊断结果驱动，不把假的成功回执给宿主。

图像审核者必须实际打开当前截图并检查 §5.4/§6.2 范围，记录看到的缺陷或 passed；截图缺失/身份错/图像工具不可用为未完成，不能从 JS 数值推测通过。修复导致绑定变化时只重做受影响图的检查/图像审核。授权范围是本次临时测试项目和已提供资料，不安装个人全局 skill，不启动 preview。

### 9.3 实际环境、故障与证据

四平台记录分别包含 native host/target、同一 tgz SHA、引擎 probe/摘要、npm bin→安装器路径、五类 HTML 和 stage results、网络条件、浏览器版本、实际导出文件及 phase outcomes。UI 的现有四平台 workflow 不是替代证据。最终 candidate 脚本必须校验全部平台、成功结果和来源一致后才给 handoff=passed，任何 not-run/unavailable 都列缺口。

离线验证在工具准备/包获取完成后执行：macOS 沿既有 Phoenix 的 sandbox-exec 禁 IP 范式，Chrome 的 sandbox 限制按当前工作流单列；Linux 使用 CI 的网络 namespace/firewall 隔离并允许该进程所需的本地 CDP，记录实际配置与请求。不能仅凭 CARGO_NET_OFFLINE 声称整个系统零网络。无需远程资源的 HTML 在浏览器断网模式下阅读/导出，并断言无远程字体、脚本、跟踪或更新请求。

故障演练覆盖：candidate/HTML 在门间变化；output/sidecar 是 symlink/hardlink/新 claimant；提交各步骤 I/O 失败及 kill 后恢复；引擎 digest/probe/version/不支持平台；离线少一个资源；安装各 journal checkpoint 中断及后来的用户修改；Chrome 未安装/坏路径/启动失败；恶意文本；toBlob 空/错误、录制 error 后到 stop/dataavailable、权限拒绝和导出后继续阅读。错误分支可定向注入，正向必须真实 binary/browser/文件，且不以 fault fake 当真实平台/导出证据。

V-9/V-15 的审核闭环须保存首次 finalize 的 deliveryReceiptId、browser receipt 身份与截图摘要；实际看图后调用 review-record，比较其返回值和写后 summary：四门 pass、视觉 passed、deliveryStatus=complete，且上述身份均未改变。重复登记同一记录仍可收口；分别改变 candidate/HTML/config/engine、browser/截图/审核绑定或范围必须拒绝当前通过。再次 finalize 使用新证据目录仍应取得新 receipt，旧审核不得继承。另覆盖记录已封存但 summary 写失败后的同记录重试；不能靠再次 deliver 达成 complete。

输出恢复在真实文件系统中，于旧 public 已退役、新 public 尚未可见的 checkpoint kill 进程，丢弃终端输出后仅按 §5.2 包内定位说明找到持久目录，经公开 recover-output 恢复并核对原字节，重复调用结果幂等。V-15 用最终 tgz/npm 入口及无 Rust、无检出源码的消费环境。分别插入新 claimant、改备份/记录绑定、替换父目录，应保留现有内容并非零停止；恢复过程中再次 kill 后重试不得覆盖后来修改。核对返回的 publicState、恢复/保留状态、退出码和未补造通过回执；仅 Rust 私有函数通过不算消费恢复验收。

为确定性命中公开 CLI 的短暂中断窗口，维护验收仅在 `PHOENIX_DIAGRAM_ACCEPTANCE=1`、`PHOENIX_DIAGRAM_CHECKPOINT_DIR` 和 `PHOENIX_DIAGRAM_CHECKPOINT_PHASE` 同时显式配置时启用 checkpoint。目录须为验收进程预先创建的当前用户 0700 普通目录；phase 只允许 after-retire、after-recovery-link、after-witness-quarantine。引擎以独占 0600 `checkpoint.json` 同步实际 pid/phase/target/recoveryDirectory 后最多等待 30 秒，由控制器 kill 本次 child。该标记只确认中断时机，恢复定位须丢弃其路径内容并按上段重新查持久记录。普通消费未启用时不读写这些标记、不延迟、不改变布局或发布合同；这不是面向用户的恢复命令，也不授权选取未知事务。

成功日志不倾倒进计划；里程碑记录按检查组保存命令、实际结果、少量身份/阶段输出与必要链接。四平台原始 CI artifacts 属于可复现分发成果，按交接需要保存，不另造重复的本地证据目录。未准备环境、运行失败、断言失败和无法图像审核分别记缺口。

## 10. 里程碑与验收安排

阶段 A 完成原生五类图的作者→阅读/导出/审核路径；阶段 B 完成四平台同包消费与真实 host 路径。每阶段收口集中执行端到端、浏览器、故障与视觉；其余只跑局部检查。主 agent 统一维护计划与共享契约、安装总入口、package/发布保护及集成资源；独立布局模块在输入/scene 合同及 V-2 稳定后可按实际收益并行，不能同时改 geometry 或共享 schema。浏览器、临时项目和同包端口串行或按进程隔离。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 固定来源、输入与可恢复的编译边界 | 本工作树 §1.3 的确切来源；无前置里程碑 | ★ upstream 源码/许可/旧语料、Cargo package/lock、input/diagnostic/scene、schemas、prepare/source-check；按旧/新 validator 和 UI 工具锁范式落实 §1.3、§2、§3.1。先固化并校验来源，再写移植代码；共享合同由主 agent 单写 | V-1、V-2 通过，冻结来源可取回，输入/scene 合同具备布局开发所需字段；回写「实施进度」 |
| M2 | architecture 与 workflow 的完整语义布局 | M1 的输入/scene 合同及 V-1、V-2；不等其他图型 | ★ layout/architecture.rs、workflow.rs、geometry.rs、render/check 的所需增量与定向 fixtures；照参考 createRouter/compileWorkflow 和旧合同；按 §3.1、§5.1 实现。两类型文件可独立开发，共享 geometry/render/check 单人集成 | V-3、V-4 通过；本阶段未做整体浏览器终验；回写「实施进度」 |
| M3 | sequence、dataflow 与双合同 lifecycle | M1 已稳定的输入/scene 及 V-2；M2 所需共享几何可按 V-3/V-4 中已验证成果复用 | ★ layout/sequence.rs、dataflow.rs、lifecycle.rs、legacy.rs 及 tests；移植具名 renderer/compiler、旧 bandFor/measureState/pathFor；每类型独立文件，共享 helper 由主 agent 集成，三类内部不相互等待 | V-5 通过；固定旧默认/几何与新主路径均保留，兼容缺口为零才计对应核心通过；回写「实施进度」 |
| M4 | 连续交付、阅读/导出与原生阶段收口 | 五类编译/检查的 V-3、V-4、V-5；可提前开发稳定输出与 Reader 接缝，终验须五类齐备 | ★ output/delivery/browser/CLI、viewer/assets 装配、§3.2 回执及 core 测试入口；按参考 atomic-output/finalize/recover-output/check-render-output、UI CDP 子集与 Viewer README；落实 §5.1–§5.4、§6，含审核后同身份 summary 与公开输出恢复/定位说明。输出状态机与共享 Viewer 单人负责，可并行定向测试；主 agent 收口 | V-6、V-7、V-8、V-9 全通过，真实审核后 complete 可达、公开输出恢复及保护断言通过，阶段 A 当前产物可交付；回写「实施进度」 |
| M5 | 新 skill 与隔离 npm/项目安装 | §3.2/§3.3 已定合同，M1 schema；正向安装需 M4 的真实 native binary，不要求等 CI | ★ 新 skill/resources/launchers、bin/phoenix-diagram.js、install.rs；修改 bin/xgent-skills.js、package 与说明；按 UI bootstrap/preparePhoenix/providers/transaction 必要范式落实 §5.5。主 agent 单写总安装器/package，skill 与 Rust installer 可分文件开发 | V-10、V-11 通过，P1 五类声明准确且旧入口/调用方未被替换；回写「实施进度」 |
| M6 | 四平台构建与唯一候选装配能力 | M4 的 native 核心、M5 的包/安装接口；source/bundle 输入已固定 | ★ build/pack/candidate-check 脚本、workflow、dist 与许可，发布 guard 最小接入，按现有 UI build/native workflow 与 release 测试的来源/完整性范式落实 §5.6/§8。主 agent 固定共享包输入，四平台 build 各用独立 host，不合并相近 commit 的资源 | V-12 通过；构建、装配与证据校验入口齐备；平台真实消费及 host 终验留 M7；回写「实施进度」 |
| M7 | 同包四平台、真实 host 与 P1 收口 | 最终同 HEAD/tree/input 的四平台 engines、唯一 candidate tgz/release-dir、M5 安装能力；V-10–V-12 的局部证据 | 主 agent 统一候选与实际 run；执行 §9 的最终包、原生、Node18、Claude/Codex 新会话、图像审核后收口、无 Rust 消费者输出恢复、回归/离线与失败路径；必要修复只重跑失效证据。包内 docs/phoenix-diagram.md 写真实范围、恢复说明、结果与公开发行缺口，不触发公开发布 | V-13、V-14、V-15、V-16 全通过且 23 条 P1 AC 可追溯；审核/恢复经最终包公开入口验证；其余期次未冒充终验；回写「实施进度」 |

提交点：M4（阶段 A 收口）提交该阶段实现与记录；M7（阶段 B / 全部完成）最终提交。每个提交点按顶部协议回填 SHA 并单独提交计划更新；仅暂存本计划相关文件。阶段 B 中为获得同 SHA CI 制品可先提交必要候选代码，但不能在 M7 验收前将其记完成；最终验收修复产生新 SHA 时重新取得受影响原生/包证据。

7 个里程碑位于 PRD 的 5–8 个建议规模内。没有按工种拆成十几段；新增五类、迁移或发布不是隐藏的第八项。没有需要另设状态的内部任务表，依赖和可并行落点已在上表明确。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 本地参考尚未成为可跨机器取回的归档 | 其他机器不能仅凭目录名复现移植依据 | 实施者先完成 §1.3 固化与 V-1；摘要不符即停止相关移植并取回指定字节 | M1 开始，移植代码之前 | 当前来源存在且摘要已确定，不阻塞启动；缺失时阻塞相关移植 |
| 原生 schema 的 Unicode regex 差异 | 直接编译参考 common 会失败 | 已按 §2.2 确定唯一等价适配并实测 29 份五类示例；实施者用 V-2 固化边界及离线 ref 拒绝，不改冻结源 | M1 退出 | 已解决，无待选库或开放适配路线 |
| 四平台 CI、真实 provider 与图像证据尚未产生 | 不能判 P1 完成；单平台和源码检查不替代 | 实施者通过新 workflow 与本机真实会话/图像工具完成 V-13–V-15；环境缺失按具体检查回写，推进独立工作 | M7 退出 | 不阻塞开发；缺实际结果阻塞最终验收 |
| P1/P2 的同形 schema 默认值有歧义 | 无法从相同 JSON 自动判断 2.17 与 3.1 意图 | 按 ADR-1 显式合同，P1 新稿持久标识；P2 沿该接缝扩展 reference 输入，不静默猜测 | 输入消费时 | 已解决，无待拍板项 |
| 无已确认性能/输入规模/包体积 SLO | 不能声称性能提升或任意大图都能快速通过 | 实施者记录真实测量；有界失败给诊断；有用户目标后再另定优化，不引入新服务 | 出现性能承诺前 | 否，不改变本期同步本地工具路线 |
| 联合公开发行尚未授权，也没有 diagram 真实公开资源 | 不能用现有 UI-only release 流程漏发 diagram，不能宣称在线公开可安装 | 本期发布 guard 阻止遗漏；维护者后续显式授权后补真实 HTTPS/同包四平台/registry 验收及联合发行入口 | 任何公开发布前 | 不阻塞本期候选；阻塞公开发布 |

- 最终状态：**Ready**。
- 定级理由：P1 范围、产品原则、五类输入差异、核心/Reader/分发职责、外部接口与验证路线已明确；审核后由 review-record 在原交付身份收口，输出发布中断由显式 recover-output 及持久定位记录闭合，均有 M4/M7 检查落点，不存在要用户代选的安全、数据或产品语义问题。未来产物的实测结果是退出条件，未被当成既有事实。来源先固化、再移植；四平台/host/图像及公开恢复门未通过时不能完成 M7。正常计划评审不列为阻塞等待项。
- 待拍板问题：无。P1 候选不授权公开发布或活跃安装变更；以后扩大该范围时只处理新增授权。

## 12. 已知坑与历史教训

- 参考 portableOutputPath 的 surrogate-range regex 不能直接交给 Rust JSON Schema；按 §2.2 的精确内存适配处理，不能关闭整套 schema 验证来绕过（参考 common schema 的 portableOutputPath、jsonschema 0.58.6）。
- 同为 sequence v1，旧/new 默认列适配及卡片高度不同；分别按输入合同测试，不用 schema_version 猜来源（`skills/archify/renderers/sequence/render-sequence.mjs`、`local/archify-main/archify/renderers/sequence/render-sequence.mjs` 与其 README）。
- 参考 lifecycle v3 明确移除了 lanes/显式路由，直接换 validator 会拒绝有效旧输入；保留独立 legacy 分支，不强制升级（两版 lifecycle schema、旧 `measureState`）。
- 参考说明声称 Live 默认，但源码首次仍 Still；用行为测试和 `readerPaused` 初始化决定实际产品含义（`local/archify-main/viewer/motion-governor.js`）。
- 参考视觉审核可选，与本产品已确认原则不同；保留默认审核与三种真实证据，不能照抄 not-requested（PRD §5.3 C3/C4）。
- 原生 Rust 要求不意味着重写浏览器导出；保留 Export→Cleanup 的私有装配与事件终态，避免清错 live DOM（`local/archify-main/viewer/README.md`）。
- UI install 的 ACTIVE/JOURNALS/provider hooks 语义专用；复制必要保护逻辑并隔离命名空间，不直接换字符串后复用整个迁移器（`tools/phoenix-ui/crates/install/src/transaction.rs`、`installer.rs`）。
- package.files 广泛包含 skills，新增入口会进入同一个 npm 包；公开发布保护必须认识 diagram，不能只检查 UI 清单（`package.json`、`scripts/require-release-package.mjs`）。
- `npm test` 不执行真实原生四平台/provider/浏览器验收；按 C-3 分别保存结果，Node/Python 通过不能推导运行时通过（`AGENTS.md` Working in This Repository）。

