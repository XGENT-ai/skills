# PHOENIX-DIAGRAM-P2 · 十类 Rust 图解与可追溯探索（新增五类原生移植 · 仓库来源核对 · 五语言阅读器）

> **实施须知：直接依据本计划、仓库规则和当前代码实施或续做。** 先读下方「实施者定位」「实施进度」，核对工作树，再按相关正文与里程碑推进、验证、回写进度并按约定提交；使用 goal 执行时也遵循这些约定。
>
> **计划状态：Ready**
>
> 调查基线：2026-10-20 · `e29d4cafcada8809335e0a3dfaa939be95bb9643` · clean。
>
> 需求来源：[PHOENIX-DIAGRAM PRD v1.0](../PRD-PHOENIX-DIAGRAM.md) · **P2（第 2 期）**，范围以 §7.2 中 P2 的 14 条 AC 为准；WORK-F1/F2/F3/F4/F5/F6/N1 的终验期为本期。上游工作号尚未转为 SPMS 实体。
>
> 本期交付：**erd、class、tree、timeline、waterfall 的原生 Rust 生成/检查/交付，十类阅读探索与 Route/Reach 范围卡片，对固定提交的仓库来源核对与时间依据表达，五种界面语言，以及同包四平台候选**。
> **开发范围：本仓 `tools/phoenix-diagram/`、`skills/phoenix-diagram/`、diagram 专用 scripts/测试/workflow/dist、`docs/phoenix-diagram.md` 与 README 的 phoenix-diagram 条目；见 §1.2**。
> 本期独特职责：在 P1 五类的原生核心上补齐其余五类；把 P1 延期的 `meta.repository` / `sources` 变为对固定提交的实际核对，未能核对的部分在图与回执中显式限定；把随包但未验收的 Reader 探索能力扩到十类并验收。
> **顶层排除：P3 的旧入口替换（`skills/archify`、README 的 archify 路由、elintp/documentation-writer）、品牌、预览、比较、旧安装迁移与回滚；公开发布；Windows**。

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

- 最近更新：2026-10-22T16:55:00+08:00
- 当前进度：6/8 个里程碑完成
- 当前状态：进行中。R3 复审确认 F-01–F-03 已解决；新增 F-04（窄屏长详情使导航条盖住 Reach）已在 Reader 修复，并经真实触摸与局部回归验证（[M2 记录](PHOENIX-DIAGRAM-P2.records/M2.md#评审修复)）。修复改变引擎源，最终候选 `a8f3ec4` 及 M6 收口证据不再对应当前源码，M6、M8 重开，重做范围见 [M8 记录](PHOENIX-DIAGRAM-P2.records/M8.md#评审修复后的重做范围)
- 最近完成：M7 · skill、说明、包脚本与宿主验收
- 下一步：用户发起 `/review-code docs/plan/PHOENIX-DIAGRAM-P2.md` 复审 R3 修复；通过后在新引擎上重做 M6 的 V-7 第 1–5 步，再按 M8 生成新候选（四平台 CI、§9.3 宿主受影响行、distribution 收口、P2 聚合、V-14）
- 当前阻塞：无（复审不受影响）；磁盘 16:51 约 45GiB 可用，被取代运行目录是否清理由用户决定；390×844 下 waterfall 详情行被截断（修前已有，非 F-04，见 M2 未完项），是否在重做前处理待用户决定
- 代码基线：main / 修复 `1b9f910`（R1 F-01–F-03）、`7842b95`（R2 F-03 续项）及 R3 F-04 修复（见完成记录 M2 行）；`skills/phoenix-diagram/ENGINE.json` 与 `dist/` 仍指向旧候选引擎 `7fcabd88…`

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-10-20T18:58:00+08:00 | 十类输入合同、语义门、场景类型、唯一目录与 i18n、0.2.0/phoenix-v2 身份、chrono 依赖与 `--sync-dist`；V-1、V-2 通过 | [M1 记录](PHOENIX-DIAGRAM-P2.records/M1.md) | 711a5b7 |
| M4 | 已完成 | 2026-10-22T11:00:00+08:00 | git 只读来源核对、`--repo-root`、回执/配置摘要/严格当前性绑定 `sourceEvidence`、Reader 三态；V-5 通过（真实旧版 git 未运行）；R1 F-01 修复：非法 URL 诊断不回显凭据 | [M4 记录](PHOENIX-DIAGRAM-P2.records/M4.md) | 711a5b7、修复 1b9f910 |
| M2 | 已完成 | 2026-10-22T16:55:00+08:00 | 共享路由、erd 与 class 布局/渲染/检查、关系 marker 资产与 oracle；V-3 通过，合并后全 crate 332 passed；R1 F-03 修复：表详情列出外键目标；R2 续项：详情 context 换行完整显示，V-8 增多外键详情检查；R3 F-04：窄屏详情限高可滚动，Reach 不被导航条遮挡，详情检查改为窄屏触摸并操作 Reach | [M2 记录](PHOENIX-DIAGRAM-P2.records/M2.md) | 711a5b7、修复 1b9f910、修复 7842b95 |
| M3 | 已完成 | 2026-10-22T11:00:00+08:00 | 数字与时区格式 oracle，tree/timeline/waterfall 布局/渲染/检查与日精度；V-4 通过，tree 恢复 10px 圆角后复验；R1 F-02 修复：跨年时间线显示年份 | [M3 记录](PHOENIX-DIAGRAM-P2.records/M3.md) | 711a5b7、修复 1b9f910 |
| M5 | 已完成 | 2026-10-20T21:08:00+08:00 | locale 回执与 `lang`、Reader explore/catalogs 测试、语义透镜与树分支修复；十类真实产物探索通过，V-6 102 pass | [M5 记录](PHOENIX-DIAGRAM-P2.records/M5.md) | 711a5b7 |
| M7 | 已完成 | 2026-10-22T03:30:00+08:00 | skill/说明/包脚本十类与 P2 聚合；按 D12 修 workflow 布局（修复轮、光晕、整图撤 pin、文字上限、主路径修法）；新引擎上 M6 收口重做通过，V-10 通过，V-11 第三轮 62 例全部 passed | [M7 记录](PHOENIX-DIAGRAM-P2.records/M7.md) | e09bc82 |
| M6 | 进行中 | 2026-10-22T11:00:00+08:00 | 原 V-7–V-9 在 `p2-core-9`（`p2-native-8`）通过、handoff=passed；R1 修复改变引擎源，ERD 产物随之改变，需在新引擎上重做第 1–5 步 | [M6 记录](PHOENIX-DIAGRAM-P2.records/M6.md) | 711a5b7、修复 e09bc82 |
| M8 | 进行中 | 2026-10-22T11:00:00+08:00 | 原最终候选 `a8f3ec4` 的 V-12–V-14 与 P2 聚合通过；R1 修复改变引擎源，需生成新候选并只重取受影响证据（四平台 CI、宿主受影响行、distribution、聚合、V-14） | [M8 记录](PHOENIX-DIAGRAM-P2.records/M8.md) | a8f3ec4、8539d18 |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

本计划属于既有模块增量（新增五类、来源核对、Reader 扩展）加分发收口。工作号与 AC 原文只在 [PRD §3、§7.2](../PRD-PHOENIX-DIAGRAM.md#72-分期矩阵) 定义。终验期为 P2 的七条需求，本期收口须覆盖其全部 AC；其中 P1 的 AC 已在 P1 候选上通过，但 P2 改变了引擎、Reader 与 skill 文本，相关证据按 §9 在最终候选上重取，不沿用 P1 结论。

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| WORK-F1 | 功能 | PRD §3 同名条目，P2 AC-2；终验 P2（复验 P1 AC-1、AC-3） | 新增五类保留语义表事实；十类的未知图型、非法引用、重复身份与矛盾数据具体失败 | D1、§3.1、§5.2、§5.4；V-1、V-3、V-4、V-7、V-8、V-13 | 已确认，本期终验 |
| WORK-F2 | 功能 | AC-3；终验 P2（复验 AC-1、AC-2、AC-4） | 宿主选择新增五类、转换 erDiagram/classDiagram；原有意图、Mermaid、修复停滞规则在新 skill 文本下保持 | §0.6、§6.5；V-10、V-11 | 已确认，本期终验 |
| WORK-F3 | 功能 | AC-1–AC-4；终验 P2 | 主要元素来源绑定固定提交与有效行；不能核对或工作树不同的部分显式限定；四类托管链接与仅本地标识；无记录时间显式为示意/日精度 | ADR-1、D7、§3.3、§5.3、§6.4；V-4、V-5、V-9、V-11、V-13 | 已确认，本期终验 |
| WORK-F4 | 功能 | AC-2；终验 P2（复验 AC-1、AC-3、AC-4） | 新增五类走同一四门，showcase 零错误/零警告；来源核对状态独立报告，不混入自动结论 | §3.3、§5.1、§5.4；V-3、V-4、V-5、V-7、V-13 | 已确认，本期终验 |
| WORK-F5 | 功能 | AC-2、AC-4；终验 P2（复验 AC-1、AC-3） | 十类筛选、Route、Reach、树折叠只反映已著事实且空/无路径/身份冲突有状态；概览、展示、深链接可回到完整图，键盘可达 | §6.1；V-6、V-7、V-13 | 已确认，本期终验 |
| WORK-F6 | 功能 | AC-2；终验 P2（复验 AC-1、AC-3、AC-4） | 新增五类全部格式；Route/Reach 范围卡片标明范围且不冒充 canonical | §6.2；V-6、V-7、V-13 | 已确认，本期终验 |
| WORK-N1 | 可用性 | AC-3、AC-4；终验 P2（复验 AC-1、AC-2） | 十类同一窗口/实际看图/操作门槛；Reader 支持 en、zh-CN、zh-TW、es、ko，未支持语言明确回退 | D6、§6.3、§7；V-2、V-6、V-7、V-9、V-11、V-13 | 已确认，本期终验 |
| WORK-N2 | 可维护性 | AC-2；终验 P3 | 新增五类的生成、布局、校验与交付在原生 Rust 完成，正常路径不调用 Node 图解引擎 | D1、D4、§2、§5.2；V-3、V-4、V-12、V-13 | 已确认，本期部分交付 |
| WORK-N3 | 兼容性 | AC-2；终验 P3（P1 AC-1 保持） | 固定 3.1.0 参考包十类有效 JSON 的核心数据可被消费；lifecycle v1 与 v3 分别核对；原 JSON 与旧独立 HTML 不被覆盖 | D2、§3.1；V-1、V-8 | 已确认，本期部分交付 |
| C-1 | 技术约束 | [P1 计划](PHOENIX-DIAGRAM-P1.md) C-1 所列 `package.json`、`rust-toolchain.toml`、`mise.toml`；`tools/phoenix-diagram/Cargo.toml` | npm-only；消费 Node ≥18；维护 Rust 1.99.0、Node 25.2.1、mbx 1.21.1；四平台；本期新依赖只允许 D4 的两项 | D4、§2.2；V-2、V-12、V-14 | 已确认 |
| C-2 | 授权与范围 | PRD §2.2、§5.3、§7 首段；`AGENTS.md` §7 | 不自动安装、打开、预览或公开发布；不改 archify 与其他 skill 的触发/授权；参考整合先按 §0.6 清单，再改目标 skill | §0.4–§0.6、§8；V-10、V-11 | 已确认 |
| C-3 | 仓库验证与接口 | `AGENTS.md` Working in This Repository、§6；`skills/agi-mode/playbooks/authoring-a-skill.md` | 轻量、Phoenix runtime、真实平台/浏览器/宿主分别留证；skill 描述完整文本与前 100 字核对；新增 `--repo-root` 等接口在所有复述处同步 | §6.5、§9；V-10、V-14 | 已确认 |
| C-4 | 跨期合同 | PRD §7 首段、§7.1–§7.4 | P2 交付可用候选；P1 已交付能力持续满足原门槛；N2/N3 及 F11/N4/N5 不得提前终验；建议 5–8 个里程碑 | §0.5、§10；V-7、V-13、V-14 | 已确认 |

没有依赖未证实产品语义的假设。吞吐、p95、最大输入规模、包体积预算仍未知；本期不据此引入服务、队列或节点数裁剪，性能只记录实测（§7）。

### 0.2 决策表

本计划中的参考路径（`renderers/…`、`render-*.mjs`、`references/…`、参考 `SKILL.md` 第 N 行）均相对冻结归档的已核对解包 `local/phoenix-diagram/reference/archify/`；Phoenix 路径相对仓根，`src/`、`tests/`、`assets/`、`viewer/` 开头者相对 `tools/phoenix-diagram/`。

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 新增类型形态 | `DiagramType` 增五个变体；每类独立 `layout/<type>.rs` 与 `render/<type>.rs`，经现有 enum dispatch 接入；`DiagramFacts` 增类型化变体 | 不建插件注册器或通用图类型 trait；原五类的 layout/render 路径不改；N2 由同一原生 binary 承担 | `tools/phoenix-diagram/src/input.rs:9-55`、`layout/mod.rs:30-36`、`render.rs:473-935` |
| D2 | 参考输入合同 | 不新增第三合同。新增五类只有 phoenix-v1：未标识即 phoenix-v1，显式或标识为 legacy-2.17 时报 `input/contract-unsupported`。未标识的 3.1.0 原五类沿 P1 ADR-1，由 `--input-contract phoenix-v1` 显式选择 | phoenix-v1 schema 与 3.1.0 逐类只差 `meta.input_contract` marker；第三合同须改动约 63 处合同比较与 5 处穷举 match。P1 对 phoenix-v1 的两项布局适配不改变语义，满足 N3/AC-2 的核心消费 | `input.rs:148-151`；`schema.rs:6-21`；P1 计划 ADR-1 |
| D3 | ERD/class 路由 | 把 `layout/architecture/router.rs` 提为 `layout/router.rs`，供 architecture/erd/class 共用；补参考 `createRouter` 的七项选项（sideFor、maxPortSpacing、preferredCandidates、compositionFloors、crossingFreeGridFirst、componentGapPx、minimumTerminalSegmentPx）。erd 的 banded grid 供 class 复用 | 第三个同构调用者出现，提取有真实用例；architecture 产物须与 V-3 既有 oracle 逐项一致，七项选项缺省时为原行为 | 参考 `renderers/architecture/routing.mjs:75`、`renderers/class/render-class.mjs:455`、`renderers/erd/grid.mjs:61`；Rust `router.rs` 为 `pub(super)` |
| D4 | 时间与数字格式 | timeline 用 `chrono =0.4.45`（default-features=false）+ `chrono-tz =0.10.4`（default-features=false，内置 tzdata 2025b，与维护 Node 25.2.1 的 `process.versions.tz` 相同）；数字格式自实现 ECMAScript Number→String、`toPrecision`，以及 ICU en-US 的“最短十进制 → half-expand → 千分位” | 不用系统 tz/ICU、JS VM 或 jiff（jiff-tzdb 0.1.8 为 2026c，与 oracle 不同代）。两 crate 与 phf 0.12.1 已在本机 registry，离线构建及 4 个时区样例已与 Node 一致 | §5.5；本机 probe |
| D5 | 仓库证据 | 见 ADR-1 | — | WORK-F3 |
| D6 | 界面语言与目录 | 阅读器装配脚本输出唯一规范化目录 ★ `tools/phoenix-diagram/assets/catalogs.json`（`\bArchify\b`、`\bARCHIFY\b` 改为 Phoenix 身份，补缺键与本期新键），模板与 Rust `i18n.rs` 读同一份；原五类既有图例表不动；SVG `lang` 与回执写解析后的语言 | 翻译只有一个来源，Rust 与 Reader 不分叉；修正卡片徽标“ARCHIFY”残留 | WORK-N1/AC-4；`scripts/assemble-phoenix-diagram-reader.mjs:181`、`render.rs:489` |
| D7 | 时间精度 | phoenix-v1 timeline 事件增可选 `precision: "day" \| "time"`，缺省 `time`（即参考行为）；规则见 §3.1 | 参考仅建议“日期按中午并在卡片说明”，不能满足“图中明确精度”。没有该字段的 3.1.0 输入仍与 oracle 一致 | WORK-F3/AC-4；参考 `render-timeline.mjs:61-150`、`references/repository-authoring.md` |
| D8 | 工具身份 | toolVersion `0.2.0`，viewerRevision `phoenix-v2`；inputContractSchema、receiptSchema 仍为 1（只做加法）；`build.rs` 的 INPUTS 由整个 `skills/phoenix-diagram` 收窄为实际嵌入的 `skills/phoenix-diagram/schemas` | 十类引擎可辨认；SKILL/示例/参考文本改动不再改变引擎身份，HTML 与引擎相关证据可按摘要复用；npm 包版本不变，公开版本由发行时另定 | WORK-N2；`build.rs:7-18`、`schema.rs:14-21`；P1 M5 记录 R3 行 |
| D9 | 收口拓扑 | 见 ADR-2 | — | C-4；P1 M7 成本 |
| D10 | 宿主覆盖 | Claude Code 跑 §9.2 全部 31 行；Codex 跑 P2 新增 13 行加 3 行回归抽样；任一 Codex 行与 Claude Code 的同行为不一致时，Codex 补跑全部回归行 | 两宿主都验证新增能力；回归行验证的是两宿主共用的同一 skill 文本。代价是 Codex 上 P1 行的覆盖从全量降为抽样 | §9.2；P1 M7 记录的会话成本 |
| D11 | 开发包同步 | `scripts/build-phoenix-diagram.mjs` 增 `--sync-dist`：把刚构建的 development VERSION、bundle 与 ENGINE 逐字同步到 `dist/phoenix-diagram/` 与 `skills/phoenix-diagram/ENGINE.json` | 去掉 P1 的手工三份清单同步（P1 曾因 dist 内 SKILL.md 未同步导致 pack 失败） | P1 M6 记录 F-03 行、M7 记录 :521 |
| D12 | workflow 布局停滞（V-11 暴露，用户裁定 2026-10-21） | phoenix-v1 schema 2 的 workflow 在基础布局失败后进入修复轮：按实际选用的边分散端口、外侧平行通道、对残余交叉启用已验收的交叉光晕、对错误对尝试强制边组合；pin 冲突与组合错误给出“撤掉全部路由 pin”的已验证修法；文字容量诊断给出经整图重编验证的长度上限 | 已通过的图几何不变，legacy-2.17 不变，不放宽检查阈值（布局期交叉判定与产物检查同样接受双方带光晕的交叉）；整图撤 pin 修法记为 `verifiedWorkflowRepair`，冻结 oracle 的最小修法与冲突分类不变；`input/main-path` 列倒退另给最少改列修法。偏差：冻结 oracle 中只因一处自动边交叉被拒的 `optional-placement-advice` 现以光晕通过、不再给摆放建议。引擎身份改变，M6 收口与 V-11 全部 62 行重跑 | M7 记录「继续动作」第 7 项；冻结 3.1.0 对同样首稿同样拒绝渲染 |

### 0.3 ADR-lite

#### ADR-1：仓库来源用 git CLI 只读核对，未核对部分标注后交付

- 状态：Accepted。
- 背景与驱动：WORK-F3 要求来源可定位到固定提交的文件与有效行，并要求“未成立时只可标为待核实或示意”（PRD §3 WORK-F3、S3、UAT-3）。参考 `renderers/shared/repository-evidence.mjs` 用 git CLI 核对，任何核对失败都会阻断渲染；它不比较工作树，也没有“待核实”表达。
- 备选：① **git CLI 子进程 + 逐条来源状态**，未核对条目标注后交付；② 进程内 `gix` crate；③ 照搬参考的 fail-closed，任一来源不能核对即失败；④ 不核对，只靠创作说明。
  - ② 免除外部命令，但引入数十个 crate 与许可闭环（P1 曾因 jsonschema 子包补全文许可），且部分克隆、replace objects 等语义要重新对齐。
  - ③ 实现最少，但作者只读到未提交修改或缺本地提交时只能删掉来源，读者反而看不到哪些断言缺依据，与 S3/UAT-3 相悖。
  - ④ 不满足 AC-1/AC-2。
- 决策：选①。
  - 结构与安全错误一律输入失败，不生成 HTML：带凭据的 URL、路径逃逸、非法行区间、provider 与主机不符、web 模式用了不支持的主机。
  - 可核对性问题逐条记录 `verified` / `unverified` 及原因，HTML 照常生成并在详情中显式限定：缺 `--repo-root`、git 不可用、提交或对象缺失、文件缺失、行越界、工作树与固定提交不同、旧版 git 遇部分克隆。
  - 来源状态是独立维度，不改变四门结论与退出码；回执与 summary 报 `evidence.status`，交付答复必须说明非 verified 的部分。
  - 布局与来源无关：不移植参考为未绘制的来源标记预留的 38px/18px 空间。
- 正面后果：AC-1–AC-3 可通过真实 git 仓库实测；工作树改动可被工具识别（参考仅靠创作纪律）；HTML 几何只取决于输入与配置。
- 负面/中性后果：新增“待核实/未提供来源”的 Reader 表达与五语言文案；宿主可能忽略 `evidence.status`，由 SKILL 交付答复规则与 V-11 场景约束；来源核对依赖用户机器上的 git，缺席时只能 unverified。
- 重新评估触发：V-11 或用户反馈显示宿主经常交付未说明的 unverified 图；或 git 版本差异导致误报 unverified 超过个别情形。届时评估 `--require-verified-evidence` 默认失败模式或进程内实现。

#### ADR-2：四平台只做自动消费，完整审核在本机原生同包闭环

- 状态：Accepted。
- 背景与驱动：P1 M7 让四个 CI 消费 job 在原文件系统内等待实际看图后再 `review-record`。最终涉及 24 个 CI run、9 个审核分支、多次审核窗口超时，本机证据约 14G；其中最贵的是宿主会话，以及“改 SKILL 即改变引擎身份”引发的全量重跑（P1 M7 记录）。P2 新增的 AC 不含平台矩阵；P1 四平台原生测试结果一致（各 267 pass / 0 fail / 2 ignored），记录中没有平台字体或文本差异。
- 备选：
  - ① 照 P1：四平台各自等待审核。最强，但成本与脆弱性最高。
  - ② **四平台自动消费，本机原生完整闭环**：四平台 CI 原生构建/测试/消费十类，自动四门全过，视觉保持 required-pending；在 darwin-arm64 上，用同一 tgz 完成实际看图、`review-record` 及全部 Reader/导出/证据/语言收口；另外三个平台的新增五类截图由 Agent 实际查看并记为观察。
  - ③ 只在本机验收，不跑 CI。不足以支撑“可交付候选”在四平台成立。
- 决策：选②，并配套 D8 的身份收窄与 §9.3 的证据复用规则：宿主证据先在 development 包上取得，对最终候选按 bundle 文件摘要（ENGINE.json 除外）和引擎 sourceInputSha256 相等复用；截图 sha256 与已实际查看者相同时，可复用该次观察登记 `review-record`。
- 正面后果：去掉“活 job 等审核”的窗口风险；skill 文本修订不再连带使引擎与 HTML 证据失效。
- 负面/中性后果：linux 与 Intel mac 不做 `review-record` 闭环，只有自动测量和截图观察；跨平台的视觉缺陷只能在观察中发现。
- 重新评估触发：任一平台的自动测量或截图观察出现平台特有缺陷，或宿主/用户报告平台渲染差异。对受影响平台与图型恢复 P1 的 await_review 流程。

### 0.4 职责与事实所有权

| 责任 | 宿主 Agent / 作者 | phoenix-diagram | Git / 浏览器 / CI |
| --- | --- | --- | --- |
| 拥有 | 请求事实、仓库切片阅读、来源选择与固定提交、时间依据声明、candidate、实际看图与修复决定 | 输入边界、十类布局与安全序列化、来源的结构校验与只读核对、确定性产物、诊断、证据身份 | 提交/对象/origin 的真实内容；DOM/字体/媒体结果；原生构建与运行 |
| 不拥有 | 不以推测补事实，不把工作树字节当作固定提交证据 | 不判断引用行是否“支持”断言的语义，不 fetch、不改仓库、不扫描全仓，不把来源核对并入四门结论 | 不授权写用户配置；文件可下载或链接可生成不等于来源已核对 |

一句话：工具只证明“该路径与行在固定提交里存在，且作者读到的工作树与之相同”；断言是否被这些行支持，由宿主创作纪律与 V-11 的角色审核负责。

### 0.5 明确不在本期

- **P3**：替换 `skills/archify`，以及 README 的 archify 路由、elintp、documentation-writer 等调用方（WORK-F10）；品牌（WORK-F7，`brand` 仍按 P3 延期字段报告）；预览（WORK-F8）；架构比较（WORK-F9）；旧安装认领、跨版本迁移与用户主动回滚（WORK-F11/AC-3、AC-4）；最终发行身份与许可（WORK-N2/AC-3）；延期字段最终处置与图型扩展约定（WORK-N3/AC-3、AC-4）；WORK-N4/AC-3；WORK-N5/AC-2。
- **guided views**：`meta.views` 在参考 3.1.0 已退役（“accepted and ignored”），PRD NG5 也不重建。本期继续接收并忽略，延期期次由 P2 改报 P3；`legacy-2.17` 的 `meta.animation` 同样改报 P3。
- **P1→P2 原地升级验收**：现有安装事务对受管且未修改的文件会备份后替换（`src/install/plan.rs:198-262`），本期不改它；正式升级与回滚验收在 P3。
- **公开发布与活跃安装**：不运行 npm/R2 写入，不安装到用户真实项目或个人 skill 目录；联合发行阻断（P1 §8）保持。
- **来源语义判定**：不 fetch 远端或调托管 API，不做全仓扫描，不判断引用内容是否支持断言（职责见 §0.4）。
- **Windows、数值图表、仪表盘与其他 infographic**：沿 PRD NG1/NG3。

### 0.6 参考整合清单（先于目标 skill 修改）

承接 PRD §5.3 与 `AGENTS.md` §7。下表是修改 `skills/phoenix-diagram` 之前的审阅清单。M7 的 diff 必须逐行对照；新增行为差异先更新本表，原则冲突没有用户依据时暂停该项。参考路径均相对 `local/phoenix-diagram/reference/archify/`，即冻结归档的已核对解包。

| 目标文件/规则 | 当前行为 | 参考来源/段落 | 采用、适配或拒绝 | 原则、调用/授权、接口与验收影响 / 核对 |
| --- | --- | --- | --- | --- |
| `skills/phoenix-diagram/SKILL.md` 描述与类型表 | 五类；明确写“P1 不支持 ERD…” | `SKILL.md` Type router 第 59–63 行 | 采用 erd/class/tree/timeline/waterfall 五行，并改为中文用途说明；保留数值图表/仪表盘/其他 infographic 的排除 | 触发范围扩为十类；描述按 C-3 核对完整文本与前 100 字；V-10、V-11 |
| 同上，Mermaid | flowchart、graph、sequenceDiagram、stateDiagram | `SKILL.md` 第 76–77 行 erDiagram→erd、classDiagram→class | 采用，仍只作语义材料，不运行 Mermaid、不复刻样式 | WORK-F2/AC-3；V-11 的 mermaid-er、mermaid-class 两行 |
| 同上，ERD 创作 | 无 | `SKILL.md` 第 30 行：表优先、列出全部物理列、类型与源注释、键角色列表、fk 必带 references、按功能域 tag 成实心矩形 | 采用事实完整与功能域规则。**拒绝**“约 8 表或 50 字段先用 standard”：PRD WORK-F4 默认 showcase，standard 只在作者明确要求时使用。showcase 停滞时按停滞规则报告，并可询问用户是否改用 standard | 不降低默认质量门；V-11 的 intent-erd 行 |
| ★ `references/authoring-defaults.md` 新五类小节 | 只有五类 | `references/authoring-contract.md`：ERD 约第 340 行、Tree 约 415、Class 约 433、Timeline 约 463、Waterfall 约 483 | 只适配影响有效性与语义的规则（单父层级、类成员不截断、事件时间须带偏移、跨度计时一致、无结束须 incomplete）；不引入节点/成员数量目标 | 不新增配额；V-10 静态核对 |
| ★ `references/repository-authoring.md` | 无；`authoring-defaults.md:17` 写“P1 不执行仓库来源自动核对” | `references/repository-authoring.md` 全文（Freeze identity、Map the slice、Trace ownership、Record evidence、Name uncertainty、Evidence by type） | 采用阅读与取证纪律。适配：①工作树改动由工具标 `worktree-differs`，作者仍须在答复列出改动路径；②带凭据 URL 由工具拒绝；③日期记录用 `precision: "day"` 取代“中午 + 卡片说明”；④无记录的 waterfall 用 `illustrative`，不为计时运行项目 | 授权仍限用户指定的仓库与切片；不全仓扫描、不 fetch；V-11 的 repo 三行与 waterfall 行 |
| `references/delivery-contract.md` / 交付答复 | 四门、实际看图、review-record | `SKILL.md` 第 37 行 `--repo-root`；第 120 行“示意而非实测、缺来源证据须说明” | 采用 `finalize ... --repo-root`；采用答复中说明 `evidence.status` 非 verified 的部分及示意时间。**拒绝**参考的可选视觉审核（`SKILL.md` 第 89 行） | 新增 CLI 参数属接口变更，同步 `docs/phoenix-diagram.md`；V-10 |
| `references/viewer-runtime.md` | 写明 Route/Reach 卡片、深链接未在 P1 验收 | 参考 `references/viewer-runtime.md` 与 `SKILL.md` 第 101 行 | 采用：改为本期已验收的能力说明（筛选、Route/Reach、范围卡片、概览、展示、深链接、树折叠、五语言）；仍只在用户请求 Reader 功能时读取 | 不扩大宿主自动行为；V-10 |
| 其余参考规则 | — | `SKILL.md` Update awareness、第 14 行 `.archify/` 输出目录约定、第 29 行品牌查询、第 85 行 Architecture 交叉摆放轮次 | 全部拒绝：更新判定与品牌分别违背 P1 已定分发原则和 P3 范围；输出目录与摆放轮次不在 Phoenix 基线 | 与 P1 §0.6 一致；V-10 核对未引入 |
| 来源与许可 | P1 已保留参考 MIT 与来源身份 | 参考 `LICENSE`、`THIRD_PARTY_NOTICES.md` | 新五类移植代码沿用同一 MIT 来源标注；chrono/chrono-tz/phf/siphasher 许可全文与 tzdata 公有领域声明随 Cargo.lock 收集 | 不冒称原创；无 `agents/openai.yaml` 不新建；V-2 |

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **[已核实·足够]** 冻结归档 `tools/phoenix-diagram/upstream/archify-3.1.0-source.tar.gz` 已含新增五类的 renderer、schema、每类 2 个示例与上游测试，本地已核对解包在 `local/phoenix-diagram/reference/archify/`（由 `scripts/check-phoenix-diagram-source.mjs` 的 `verifyFrozenSources` 守护）。参考 examples 共 25 份 JSON（原五类 15、新增五类 10）与 15 份独立 HTML；其中 `brand-aware-delivery.architecture.json` 的组件含 `brand`，`orders.erd.json`、`subscription-billing.erd.json` 含 `meta.views`；均不含 sources/repository。
- **[已核实·缺口]** `DiagramType` 只有五个变体（`input.rs:9-36`），新增五类在解析时报 `input/unsupported-type`（`tests/input_contract.rs:24-38`）；合同缺省一律 `Legacy217`（`input.rs:148-151`）→ 按 D1/D2 增变体并改缺省 → 否则新类型连 schema 门都进不去。
- **[已核实·足够]** phoenix-v1 的 common 与五类 schema 和 3.1.0 相比只多 `meta.input_contract` 一项；参考新增五类 schema 均为 `schema_version` 常量 1，共同引用 common 的 `repository`、`sourceReferences`、`translations` 等定义。
- **[已核实·缺口]** 场景与渲染不足以表达新增五类 → 按 §3.2、§5.4 补类型化事实、按需 marker 与类型专属 body → 否则只能压成丢语义的通用图：
  - `Relation` 恒画 `marker-end` 箭头（`render.rs:634`），`assets/svg-definitions.svg` 只有四种箭头；
  - 没有表格行/成员分区；
  - `DiagramFacts` 只有五个变体（`scene.rs:102-135`）。
- **[已核实·缺口]** architecture 路由器是 `pub(super)`，缺 erd/class 所需的七项选项（D3）→ 不补则 ERD 端口间距与 class 继承总线无法复现。
- **[已核实·缺口]** `check.rs` 的 `semantic_content` 只核 label/sublabel/tag 与关系文本（`check.rs:844-857`）→ 须扩到 ERD 列、类成员、树父子、事件标题/时间、跨度名称/时长 → 否则产物丢列或丢成员仍可能通过。
- **[已核实·缺口]** 来源相关字段目前只是被延期：
  - `sources` 与 `/meta/repository` 被标为 P2 延期字段（`input.rs:240-246`）；
  - `DeliveryReceipt.evidence` 恒为 `{}`（`delivery/transaction.rs:30,247,480`）；
  - Reader 的 `PhoenixDiagram.sourceEvidence` 要求 metadata 顶层 `verified===true`，按设计处于休眠（`viewer/template.source.html:826-866`）；
  - `delivery/metadata.rs:64` 要求 `metadata.meta` 与输入 meta 逐字相等。

  → 按 §5.3 新增核对与独立 `sourceEvidence` 字段 → 否则 AC-1–AC-3 无落点。
- **[已核实·足够]** 参考核对算法：`repository-evidence.mjs` 经 `runGit` 与批量读取（`--no-replace-objects`；环境同时设 `GIT_NO_LAZY_FETCH=1` 与 `GIT_ALLOW_PROTOCOL=''`，注释说明旧 Git 可能忽略前者）执行 rev-parse、remote get-url、cat-file、ls-tree；`repository-location.mjs` 负责解析 origin、拒绝 HTTP(S) userinfo、生成 GitHub/Gitee/GitLab 永久链接。它不检查工作树，也不检查 HEAD 是否等于 revision。
- **[已核实·足够]** Reader 模块已装配并可用：语义筛选（semantic-lens）、Route（route-probe）、Reach（focus.js）、概览（semantic-radar）、展示（`?present=1`）、深链接（focus/reach/relation/route/lens/camera、query 的 theme/embed/present）与键盘快捷键（`viewer/` 各文件）；范围卡片完整保留（`viewer/export.js` 的 `renderShareCard`、`runRouteShareCard`、`runReachShareCard`）。
- **[已核实·缺口]** Reader 仍有以下缺口：
  - `tree-branches.js` 在没有 `svg[data-tree-ui]` 时保持休眠；
  - 五个目录中 `viewer.preset.badge.editorialPlate`、`viewer.export.card.routeBadge`、`reachBadge` 仍是大写 “ARCHIFY”（装配正则区分大小写）；
  - zh-TW/es/ko 比 en/zh-CN 少 `viewer.export.downloadPngInstead`（486 对 487 键）；
  - SVG `lang` 直接写原始 `meta.locale`（`render.rs:489`）；
  - Reader 单测不覆盖 Route/Reach/筛选/概览/查找/深链接/展示/树/卡片（`tests/viewer/unit.test.mjs`）。
- **[已核实·足够]** 参考新增五类不调用 `Intl.Segmenter`；timeline 依赖 `Intl.DateTimeFormat`（IANA 时区或固定 UTC 偏移、`formatToParts`、BC 纪年）与 `Date.parse`，waterfall 依赖 `toLocaleString('en-US')` 与 `toPrecision`。维护 Node 实测：
  - `T24:00Z` 被接受，`T24:00:01Z`、`T23:60Z`、偏移 `+24:00` 为 NaN；
  - `2026-02-30` 能被解析，故参考另设 `validCalendarDate`；
  - `(1.005)` 与 `(2.675)` 保留两位小数为 `1.01` / `2.68`，而 `toFixed` 为 `1.00` / `2.67`；
  - 时区名不区分大小写且保留 `Asia/Calcutta` 原名；
  - 固定偏移 `+05:30`、`+0530`、`+01`、`-2359`、`−05:00`（U+2212）被接受并规范为 `±HH:MM`；`+24:00`、`+05:60`、`+5:30`、`+05:30:00`、`Z`、`UTC+05:30` 及首尾带空白者被拒。参考 schema 对 timezone 只限非空字符串，被接受的偏移都是有效参考输入。
- **[已核实·足够]** 新增五类参考 renderer 约 4,317 行，约为已移植五类（约 13,068 行）的三分之一；ERD 最大（`render-erd.mjs` 1,566 行 + `grid.mjs` 150 行）。参考 `check-render-output.mjs` 对新增五类没有专属检查。
- **[已核实·缺口]** 收口与包脚本写死五类：
  - `scripts/test-phoenix-diagram-core.mjs:7`（`TYPES`）与 :58-61（每类 × 两合同的覆盖要求）；
  - `test-phoenix-diagram-runtime.mjs:12-17`；`check-phoenix-diagram-candidate.mjs:30,113,195`；
  - `aggregate-phoenix-diagram-candidate.mjs`（`cases.length===10`，`PROVIDER_SCENARIOS` 22 行）；
  - `build-phoenix-diagram.mjs:154-155`；`.github/workflows/phoenix-diagram.yml:464`；
  - `test/phoenix-diagram/` 与 `tools/phoenix-diagram/tests/` 中的类型列表。

  另有十余处写死 toolVersion `0.1.0`、`viewerRevision === 'phoenix-v1'`（如 `pack-phoenix-diagram-candidate.mjs:64`）。
- **[已核实·缺口]** `build.rs` 的 INPUTS 含整个 `skills/phoenix-diagram`（`build.rs:7-18`），而引擎只 `include_str!` 其中的 schemas（`schema.rs:14-21`）；P1 因此“改 SKILL 必然改变输入、引擎和 bundle 身份”（P1 M5 记录 R3 行）→ 按 D8 收窄。
- **[已核实·足够]** 本仓 origin 为不含凭据的 `https://github.com/XGENT-ai/skills.git`，可作为真实 GitHub 来源核对与永久链接的验收仓库；本机 git 2.53.0，Apple git 2.54.0。本机 2.53 以 `clone --filter=blob:none` 得到的部分克隆只写 `remote.origin.promisor` 与 `partialclonefilter`，不写 `extensions.partialClone`；去掉 `GIT_NO_LAZY_FETCH`、只留空 `GIT_ALLOW_PROTOCOL` 时，读缺失对象报 `transport 'file' not allowed`，对象仍缺失。

### 1.2 文件清单与拓扑

★ 为新增；表内未列的邻接面不修改，包括 `skills/archify`、其他 skill、Phoenix UI、`src/output*`、`src/install*` 的事务语义、npm 总安装器的选项语义。

| 范围内路径 | 交付/改动 | 同构实现与边界 |
| --- | --- | --- |
| `tools/phoenix-diagram/src/input.rs`、`input/semantics.rs`、`schema.rs`、`cli.rs` | 十类、合同缺省、新五类语义门、延期字段改期、`--repo-root`、guide 关键词与 HELP | 沿现有 parse_input 闸序与诊断形状 |
| ★ `src/evidence.rs`、★ `src/evidence/location.rs`、★ `src/evidence/git.rs` | 来源结构校验、只读核对、永久链接 | 移植参考 `repository-location.mjs`、`repository-evidence.mjs` |
| ★ `src/format.rs`、★ `src/format/time.rs`、★ `src/format/number.rs` | V8 时间解析、时区墙钟、JS/ICU 数字格式 | §5.5；oracle 来自冻结 Node |
| ★ `src/i18n.rs` | 读取 `assets/catalogs.json`，解析语言、覆盖与占位符 | 解析规则移植参考 `renderers/shared/i18n.mjs:126-196` |
| `src/scene.rs`、`src/layout/mod.rs`、`src/layout/architecture.rs`；`src/layout/architecture/router.rs` → ★ `src/layout/router.rs` | 新事实变体、关系 marker、明细行；路由提取与七项选项 | D3；architecture 只改 import |
| ★ `src/layout/{erd.rs,erd/grid.rs,class.rs,tree.rs,timeline.rs,waterfall.rs}` | 新五类结构→布局→场景 | §5.2 的参考锚点 |
| `src/render.rs`；★ `src/render/{erd,class,tree,timeline,waterfall}.rs`；★ `assets/svg-relationship-markers.svg` | 新类型 body、metadata `sourceEvidence`、解析后的 `lang` | 原五类 body 不改；新 marker 只在 erd/class 嵌入 |
| `src/check.rs`、`src/check/anchors.rs` | `semantic_content` 扩到新五类；marker 适用性 | 沿现有具名检查，不放宽阈值 |
| `src/delivery/{prepare,transaction,receipts,provenance,metadata}.rs`、`src/browser/observation.rs` | 来源/语言进回执与 summary、严格当前性绑定来源摘要；waterfall 的 width-first 纵向例外 | 沿 P1 回执头与 binding 规则 |
| `build.rs`、`Cargo.toml`、`Cargo.lock`、`assets/license-sources.json`、`NOTICE.md`、许可目录 | D8 INPUTS；D4 依赖与许可 | 依赖变更走 `prepare-phoenix-diagram.mjs --prepare --update-lock` |
| `viewer/template.source.html`、`viewer/focus.js`、★ `viewer/catalog-adaptations.json`；`assets/template.html`、★ `assets/catalogs.json`（生成） | 来源三态显示、目录规范化与新键 | 沿现有 `*-adaptations.json` 与装配器 `--check` |
| `tools/phoenix-diagram/tests/`：★ `{erd,class,tree,timeline,waterfall,evidence,format,i18n}.rs`、★ `fixtures/{erd,class,tree,timeline,waterfall}/generate.mjs` 与 oracle、`fixtures/input/`（重生成）、`support/input_corpus.rs`、`check_anchors.rs`、★ `viewer/{explore,evidence,catalogs}.test.mjs`、`export-acceptance.mjs`、★ `support/explore-reader.mjs` | 局部检查与 oracle | 沿 `fixtures/architecture/generate.mjs`、`tests/support/export-reader.mjs` |
| `skills/phoenix-diagram/`：`SKILL.md`、`references/{authoring-defaults,delivery-contract,viewer-runtime}.md`、★ `references/repository-authoring.md`、★ `schemas/{erd,class,tree,timeline,waterfall}.schema.json`、`schemas/README.md`、★ `examples/minimal.{erd,class,tree,timeline,waterfall}.json`、`ENGINE.json`、`scripts/phoenix-bootstrap.cjs`、`scripts/phoenix-launcher.cjs`、`scripts/phoenix-diagram` | §0.6 清单与 D8 版本 | schemas 为参考原件加 marker（timeline 另加 D7 字段） |
| `scripts/{test-phoenix-diagram-core,test-phoenix-diagram-runtime,check-phoenix-diagram-candidate,aggregate-phoenix-diagram-candidate,build-phoenix-diagram,pack-phoenix-diagram-candidate,assemble-phoenix-diagram-reader,test-phoenix-diagram-native}.mjs`、`.github/workflows/phoenix-diagram.yml`、`test/phoenix-diagram/*.test.js`、`bin/phoenix-diagram.js`、`dist/phoenix-diagram/` | 十类覆盖、P2 聚合配置、版本、D11 同步 | 不改 UI 发行资源与 release guard 语义 |
| `docs/phoenix-diagram.md`；`README.md` 第 21、238 行 | 十类、来源核对、`--repo-root`、候选范围 | 不改 archify 行与 README 其他条目 |

数据流：宿主写 candidate（可带 `meta.repository` / `sources`）→ 输入门 → 来源结构校验 → git 只读核对得到 `SourceEvidence` → 类型布局 → SVG + Reader + metadata（含 `sourceEvidence`）→ 输出提交 → 严格当前性检查 → Chrome 证据 → 实际看图 → review-record。没有常驻服务；git 是按需调用的本地子进程。

## 2. 模块、接口与依赖

### 2.1 模块与接口

路径均位于 `tools/phoenix-diagram/`；接口仅限同一 package 内的库、binary 与测试。

| 模块 | 调用者 | 接口与不变量 | 接缝 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- | --- |
| `input.rs` / `input/semantics.rs` | compile、delivery、CLI | `DiagramType::{Erd,Class,Tree,Timeline,Waterfall}`；`fn has_legacy_contract(self) -> bool`（只有原五类为真）。合同解析为 显式 → marker → 原五类 Legacy217 / 新五类 PhoenixV1；新五类遇 legacy 报 `input/contract-unsupported` | 无 | 合同分派、新五类结构门（§3.1） | 进程内 | V-1 |
| `schema.rs` | input | `fn validator(kind, contract) -> &Validator`；PHOENIX 增五份，LEGACY 不变；按 `(kind, contract)` 查找，不再依赖 `ALL.position` 与数组同序 | 无 | 两个注册表 `$id` 相同，必须各自构建 | 进程内 | V-1 |
| `evidence.rs` 及子模块 | delivery/prepare | `fn verify(input: &DiagramInput, repo_root: Option<&Path>) -> Result<SourceEvidence, Diagnostics>`：结构错误返回 Err（阶段 `input`），可核对性结果总是 Ok | git 子进程只在 `evidence/git.rs` | URL 解析、路径/行规则、批量 blob 读取、工作树比较、部分克隆保护 | 本地可替代（真实 git，测试用临时仓库） | V-5 |
| `format/time.rs`、`format/number.rs` | timeline、waterfall | `parse_timestamp(&str) -> Option<i64>`、`valid_calendar_date(&str) -> bool`、`zone(&str) -> Option<Zone>`（`enum Zone { Named(Tz), Fixed(FixedOffset) }`）、`wall(ms, &Zone) -> Wall`、`offset_label(ms, &Zone) -> String`；`js_to_string(f64)`、`js_to_precision(f64, u32)`、`en_us_max_fraction(f64, u32)`、`en_us_max_significant(f64, u32)` | 无 | V8/ICU 边界 | 进程内（chrono-tz 数据编译期内置） | V-4 |
| `i18n.rs` | 新五类 layout/render、render 的 lang | `fn resolve(meta: &Value) -> Locale { requested, resolved, fallback, catalog }`；`fn text(&self, key, params) -> String`：显式覆盖 → 内置目录 → en；占位符集合须一致，否则忽略覆盖并沿用内置 | 无 | 区域/脚本标签区分，大小写不敏感 | 进程内（`include_str!("../assets/catalogs.json")`） | V-2 |
| `layout/router.rs` | architecture、erd、class | 原 `Edge`/`Route`/`Label`/`place_labels` 与路由器构造不变；新增 `RouterOptions { side_for, max_port_spacing, preferred_candidates, composition_floors, crossing_free_grid_first, component_gap_px, minimum_terminal_segment_px }`，缺省等于原行为 | 无 | 端口分配、候选、交叉/走廊代价 | 进程内 | V-3 |
| `layout/<新五类>.rs` | `layout/mod.rs` | `fn compile_<type>(input, options) -> Result<CompiledDiagram, Diagnostics>`；同一 resolved scene 驱动 `--layout-json`、render 与检查 | 无 | 各类布局与容量诊断 | 进程内 | V-3、V-4 |
| `render/<新五类>.rs` | `render.rs::render_svg` | `pub(crate) fn body(scene: &CompiledDiagram, out: &mut String)`；只写本类节点、关系与轴/行；根属性、defs、图例、metadata 仍由 `render.rs` 负责 | 无 | 类型专属 SVG 语法与 Reader 属性 | 进程内 | V-3、V-4 |
| `check.rs` | validate、deliver、check | 具名检查集合不变；`semantic_content` 按类型比较产物文本与 metadata 事实；只对实际带 marker 的端做箭头类检查 | 无 | 新类型事实覆盖 | 进程内 | V-3、V-4、V-7 |
| `delivery/*` | CLI | 回执与 summary 增 `evidence`、`locale`（§3.3）；严格当前性比较 HTML 内 `sourceEvidence` 摘要与回执；不重跑 git | 无 | 当前性与确定性 | 本地文件 | V-5、V-7 |
| Reader（`viewer/`） | 读者 | 保持参考模块与装配次序；消费 Rust 输出的 `data-tree-*`、`data-timeline-*`、`data-waterfall-*`、`data-er-*`、`data-class-*`、`data-detail` 与 `metadata.sourceEvidence` | 能力探测 | 探索状态、范围卡片、来源三态 | 浏览器 JS | V-6、V-7 |

删除测试：`evidence` 把 git 语义、URL 安全与链接规则集中在一处，删掉会散进 delivery 与 render；`format` 服务 timeline/waterfall 两个调用者，且承担须与 oracle 对齐的边界；`router` 有三个真实调用者。不新建 facade、插件总线或远程 adapter。

### 2.2 依赖

- 新增 `chrono = { version = "=0.4.45", default-features = false }`、`chrono-tz = { version = "=0.10.4", default-features = false }`。
  - 传递依赖 phf 0.12.1 等由 Cargo.lock 锁定；不启用 `case-insensitive`（它要求构建期 chrono-tz-build），大小写不敏感查找由 `format/time.rs` 遍历 `chrono_tz::TZ_VARIANTS` 比较 ASCII 小写名实现。
  - `Tz` 是只含 597 个命名时区的封闭枚举，固定偏移改用 chrono 自带的 `FixedOffset` 表示（§5.5），不新增依赖。
  - 变更后用离线 `mise exec --no-deps -- mbx fetch --offline` 只为新依赖补入锁条目（`--update-lock` 会经 `generate-lockfile` 重算全部传递版本，可能改动已固定许可清单的 crate，故不用）；许可全文由构建的收集流程从各 crate 取得，五个新 crate 均自带根级许可文件，无须新增 `assets/license-sources.json` 条目。
- git 是运行期的可选外部命令，不进入依赖锁，也不随包分发。
- 不新增 Reader 依赖，不引入 gix、jiff、JS VM、WASM。

## 3. 数据模型与兼容

没有数据库。权威输入仍是确切 JSON 字节；`SourceEvidence` 是对固定提交与工作树的交付时观察，随交付绑定，不反写输入。

### 3.1 新增五类输入合同

phoenix-v1 新 schema 等于冻结 3.1.0 原件加 `meta.properties.input_contract: {"const":"phoenix-v1"}`（与 P1 五类相同的唯一适配）；timeline 另加 D7 字段。冻结 validator 在 schema 之后对每类的 `meta.output` 程序化执行可移植输出路径检查（`portable-path.mjs` 的 output 档，不要求 `.html`），而新五类 schema 只写 `"type":"string"`；Phoenix 在 phoenix-v1 输入门的 schema 之后执行同一检查，失败仍报 `input/schema`（`/meta/output`）。语义门移植参考 renderer 的输入检查并保留其诊断码；容量类诊断属于布局（§5.2），不前移。显式 id 的身份域沿 P1 规则：实体与关系共用，容器各自唯一。

| 图型 | 关键事实 | 结构/语义门（参考锚点） |
| --- | --- | --- |
| erd | entities（id、label、tag、row/col、attributes[name,type,key 单值或列表,references]）、relationships（from、to、from/toCardinality、from/toOptional、identifying、variant） | 实体/属性重复；fk 必须有 `entity.attribute` 形式的 references，且目标实体与列存在；未知端点、自引用、网格占格冲突（`render-erd.mjs` `validateEr` :1052–1213、`grid.mjs` `validateErGridPlacement` :122） |
| class | types（id、label、kind、attributes/methods 及 visibility/static/abstract）、relationships（from、to、kind 六种） | 重复、半列占格、realization 目标须为 interface、inheritance 两端 kind 匹配、继承环（`render-class.mjs` `validateClassDiagram` :652、`generalizationCycle` :623） |
| tree | nodes（id、label、parent、collapsed、sublabel） | 恰一个根、父存在、无环、重复 id（`render-tree.mjs` `validateStructure` :55–112） |
| timeline | meta.evidence（observed\|illustrative，必填）、meta.timezone（IANA 名或固定 UTC 偏移，缺省 UTC）、lanes、events（id、at、lane、kind、★precision） | 时区可识别（§5.5 语法）、时间戳（§5.5 解析 + 日历有效）、lane 重复与引用、precision 规则（下文）；不校验输入顺序，按时间排序（`render-timeline.mjs` :61–109） |
| waterfall | meta.evidence（measured\|illustrative，必填）、meta.unit（us\|ms\|s）、spans（id、name、start、end/duration、parent、status、service） | 重复；end 早于 start；end 与 duration 不一致（容差 1e-9）；无 end/duration 时须为 `incomplete`；父存在且无环。沿参考**不**校验子跨度的时间包含，嵌套只表示归属（`render-waterfall.mjs` :49–104） |

timeline `precision`（仅 phoenix-v1）：
- `"time"`（缺省）沿用参考的显示规则，即任一事件有非零秒时全部显示秒。
- `"day"` 表示记录只有日期，要求 `at` 在作者给出的偏移下为 `12:00`（秒与毫秒为零），否则报 `timeline/day-precision-time`（指向 `/events/i/precision`）。
- 渲染时：事件仍按该瞬时定位；时间标签只显示作者原样的日期部分，加目录键 `timeline.precision.day`；轴说明追加披露键 `timeline.precision.dayDisclosure`；元素 metadata 记 `precision: "day"`。

`brand`、`meta.views` 与 legacy 的 `meta.animation` 仍进入 `compatibility.deferredFields`，期次改报 `P3`；`sources` 与 `/meta/repository` 不再延期。

### 3.2 场景扩展

- `DiagramFacts` 增五个变体，均为 camelCase serde，字段即 §3.1 的语义事实：`Erd { entities, relationships }`、`Class { types, relationships }`、`Tree { root, nodes, direction }`、`Timeline { timezone, evidence, lanes, events{id,lane,atMs,precision,kind}, breaks }`、`Waterfall { unit, evidence, spans{id,parent,start,end,status,service} }`。
- `Relation` 增 `marker_start: Option<Marker>`、`marker_end: Option<Marker>`。原五类保持现有箭头，新五类按参考：ERD 用鸦脚的一/多/可选组合，class 用空心三角、实心/空心菱形与开放箭头。
- `Node` 增 `detail_rows: Vec<DetailRow { source_path, bounds, text, role }>`，承载 ERD 列与类成员；输出 `data-detail="context"`，不带 `data-graph-role="node-body"`。
- 每个 `g[data-node-id]` 仍恰有一个 `node-body` 矩形：ERD 表、类框、树节点、时间线事件卡、瀑布跨度条都是节点。
- tree 的边由 parent 派生为关系（parent→child）。timeline/waterfall 没有关系数组，不画因果箭头。

### 3.3 回执、HTML 与身份

- `SourceEvidence`（canonical JSON，sha256 记为 `evidenceSha256`）：
  ```json
  {"schemaVersion":1,"status":"none|verified|partial|unverified",
   "repository":{"label":"","url":"","revision":"","shortRevision":"","linkMode":"web|local-only","treeHref":"?"},
   "observation":{"headMatchesRevision":true,"worktree":"compared|not-compared"},
   "references":[{"pointer":"/components/0/sources/0","nodeId":"","path":"","line":1,"endLine":3,"label":"?","state":"verified|unverified","reason":"?","href":"?"}],
   "unsourced":["<nodeId>"]}
  ```
  值为 `"?"` 的字段可选：`treeHref` 只在 web 模式出现；`label`、`reason`、`href` 按条目有无出现。
  - `status` 取值：无 repository 且无 sources 时为 `none`；全部 verified 且无 `unsourced` 时为 `verified`；至少一条 verified 但另有 unverified 或 unsourced 时为 `partial`；零条 verified 时为 `unverified`。
  - `unsourced` 只在声明了 `meta.repository` 时计算，范围是该类型主要元素中没有 sources 的那些（主要元素集合沿参考 `EVIDENCE_NODE_COLLECTIONS`）。
  - 路径、时刻与 repo-root 不进入该 JSON。
- HTML：metadata JSON 增 `sourceEvidence`（`meta` 仍与输入逐字相等）；SVG `lang` 写 `Locale.resolved`。
- `configSha256` 的规范化选项增 `evidenceSha256`（无来源时为 null）。因此“同一引擎、输入字节与有效配置得到相同 HTML”在含来源时依然成立；repo-root 路径本身不进入。
- `ValidationReceipt` / `DeliveryReceipt.evidence`：`{status, evidenceSha256, repository?, counts{references,verified,unverified,unsourced}, unverified[{pointer,nodeId,reason}], unsourced[]}`。
- `FinalizeReceipt` 与 summary 增 `evidence {status, evidenceSha256}` 与 `locale {requested, resolved, fallback}`；`review-record` 不改。
- 严格当前性另核两点：HTML 中 `sourceEvidence` 的 canonical 摘要等于回执；references 与 candidate 的 sources 一一对应。任一失配报 `provenance/evidence-mismatch`。
- 身份按 D8：`TOOL_VERSION` 改为 0.2.0；probe 精确打印 `phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2`；ENGINE/VERSION、bootstrap、launcher、脚本与测试同步。缓存路径本来按版本与摘要寻址，0.1.0 与 0.2.0 并存。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| 冻结参考 3.1.0 | 已核实足够 | §1.1；`verifyFrozenSources` | 只读；新增五类 oracle 生成器从已核对解包执行原 Node renderer，仅限开发测试 | 摘要变化即阻止相关移植，不换 latest |
| git CLI（用户机器） | 已核实足够（参考语义；本机 2.53/2.54） | `repository-evidence.mjs` 的命令集，及 `GIT_NO_LAZY_FETCH=1` 加空 `GIT_ALLOW_PROTOCOL` 的双重防护；Git 2.45 起才识别 `GIT_NO_LAZY_FETCH`（2.45.0 发布说明、`environment.c` 的 `setup_git_env`） | §5.3 只读命令集；每个子进程禁止全部传输；`git version` < 2.45 且存在任一 promisor 配置时不读对象 | 缺席、超时、仓库不可读都是 unverified 原因，不是 exit 4；从不 fetch，缺对象不触发传输、不写对象库 |
| chrono / chrono-tz | 已核实足够 | 离线 `mbx run` probe 与 Node 输出逐项相同（Asia/Shanghai、America/New_York、Europe/London 夏令、Asia/Calcutta） | D4 锁定版本 | 无效时区为输入错误 `timeline/invalid-timezone` |
| Chrome/Chromium | 已核实足够（P1 合同） | P1 `src/browser/*`、probe.js | 不变；新类型须输出 probe 依赖的 `node-body` 角色与边 key | 沿 P1 unavailable/failed 区分 |
| 四平台 CI | 已核实足够（P1 run 39353915898 十 jobs 成功） | `.github/workflows/phoenix-diagram.yml` | 默认 dispatch（不设 `await_review`）跑十类自动消费 | 任一平台缺结果即 M8 不完成；runner/权限故障记缺口 |
| Claude Code、Codex 宿主 | 已核实足够（P1 两宿主各 22 场景） | P1 会话台账格式与 `checkProviderCoverage` | §9.2 场景；只用临时项目与随包资料 | 配额/超时记未完成，不以单宿主或旧会话冒充 |
| GitHub 仓库 `XGENT-ai/skills` | 已核实足够（origin 无凭据，gh 已登录） | `git remote get-url origin` | 作为真实 web 来源核对与永久链接的验收仓库；只读 `gh api` 核对链接目标在该提交存在 | 网络不可用时链接存在性检查记未运行，不影响离线核对结论 |
| npm/R2 公开发行 | 范围外 | P1 §8 | 不变 | 联合发行仍被 guard 阻止 |

## 5. 核心机制

### 5.1 CLI 与输入闸序

- `validate`、`deliver`、`finalize` 增 `--repo-root DIR`，必须是已存在的目录，否则报 `arguments/invalid`（exit 2）。
- `check`、`browser-check`、`visual-check`、`review-record` 不接收该参数，以回执为准。
- HELP 与 `guide` 的类型列表扩到十类。guide 关键词按参考 Type router 的用途词加中文同义词，模糊时列备选。
- 闸序：参数/类型 → 读取确切 JSON → 合同解析 → schema → 语义 → **来源结构校验** → **来源核对** → 布局 → 渲染 → 产物检查。前两段失败停在 `input` 阶段并有具体 pointer。来源核对从不使命令失败，只写入 `SourceEvidence`。
- 退出码沿 P1；宿主须读 `evidence.status`，不能把 exit 0 理解为来源已核对。

### 5.2 新增五类布局

逐类按参考 renderer 的结构 → 测量 → 布局 → 路由/标签 → 视口顺序移植，同一 resolved scene 驱动 `--layout-json`、SVG 与检查。所有 repair 保留节点、列、成员、事件、跨度与文字，不设数量上限，不缩字越过读取下限。

| 图型 | 参考锚点 | 移植要点 | Reader 属性 / 阅读合同 |
| --- | --- | --- | --- |
| erd | `render-erd.mjs` `buildLayoutReport` :1337、`renderSvg` :1483、`labelColumnGaps` :170、`autoViewBox` :503（最多 4 次扩图例带）；`grid.mjs` `bandedLayout` :61 | 按列取最宽、按行取最高的 banded grid；共享路由带 erd 选项（端口间距 ≤24，否则 `layout/marker-capacity`）；按功能域画实心矩形带，否则 `erd/domain-not-drawn`；头与行字号走 `fitted_node_font_size`，容量不足报 `erd/header-text-capacity`、`erd/attribute-text-capacity`；鸦脚 marker（参考 :407）；不画品牌 | `data-er-row`、`data-er-badge`、行 `data-detail="context"`、图例 hover 样式（参考 :1456–1522）；表的 context、aria-label 与 `<title>` 在参考文案后以 ` › ` 列出“外键列 → 目标”（参考未显示，WORK-F1 要求外键目标可读；节点目录仍按首段分组），焦点详情中的 context 换行完整显示；无 `meta.viewBox` 时 `data-reader-fit="intrinsic-height"`、`data-reader-min-text="7.5"` |
| class | `render-class.mjs` `fillLines` :149、`wrapMember` :173、`ordinaryRouter` :455、继承总线 :416–449、冲突回退 :472–520、`NOTATION` :282 | 复用 erd grid，类型在行内垂直居中；成员按参考正则与悬挂缩进（参数 4、续行 2）折行，不截断；两个以上子类且间距 ≥40px 时成继承总线，冲突时整组回退到路由器；`class/title-text-capacity` | `data-class-*`、成员 `data-detail="context"`、总线 `data-motion-path`；阅读合同同 erd |
| tree | `render-tree.mjs` `wrapText` :123、`STACK_MIN` :160、`renderSvg` :497 | 按代分带，按子树跨度排布，父居中；向下布局时连续 3 个以上叶子堆成缩进列表；直角连线；CJK 按参考字符类切分；`collapsed` 只决定 Reader 初始折叠，静态 SVG 与导出始终是完整树 | `data-tree-ui`、`data-tree-toggle`、`-ancestors`、`-initially-collapsed`、`-expand-all`、`-label-expand/collapse`、`-count`（参考 :477、:500）；阅读合同同 erd |
| timeline | `render-timeline.mjs` 的断轴规则（间隔大于中位数 8 倍且大于跨度 10% 时断开，至多 8 处，`SEGMENT_FLOOR` 28）、`TICKS` :257、泳道卡片上下择位 :318–356、`renderHeader` :423 | 刻度按显示时区墙钟对齐；证据徽标 observed/illustrative；轴说明写时区、偏移、单日日期与压缩披露；D7 的日精度显示；事件跨墙钟年份时，刻度、卡片与详情的日期带年份（参考只显示月日） | `data-timeline-ui`、`-at`、`-ms`、`-x`、`-lane`、`-break`、`-evidence`、`-axis-caption`，编排 `sweep`；`intrinsic-height` + 7.5 |
| waterfall | `render-waterfall.mjs` 行序（深度优先，按 start 再按索引）、`NICE` 刻度 [1,2,2.5,5]、开放跨度画到 t1、`number` :131、证据徽标 :234–247 | 单一线性比例尺；红色保留给 error，服务配色至多五色循环；incomplete 显示“≥ 时长”下界；名称列宽取自 `text_units` | `data-waterfall-start`、`-end`、`-duration`、`-status`、`data-waterfall-evidence`，编排 `replay`；`data-reader-fit="width-first"` + 7.5，由 `browser/observation.rs` 既有 width-first 例外判定 |

oracle 沿 P1 方式：生成器在已核对解包上执行原 renderer，取场景几何、文本、关系点、Reader 属性集合与被拒用例的诊断；Rust 比较场景几何（容差 1e-8）与渲染几何（容差 1e-5）。含 `brand` 的参考输入先删去 brand 再生成几何 oracle；原件另测延期字段报告与核心事实。

若参考几何在 Phoenix 更严格的具名检查（如 `arrowhead_separation`、`label_canvas_containment`）下不过，沿 P1 workflow 适配的先例处理：只在 phoenix-v1 合同内做最小、保语义的布局修正，保留原候选集与搜索预算，在记录写明偏差；不放宽检查，不改输入。

### 5.3 来源核对

**结构校验**（`evidence/location.rs`，错误码前缀 `repository/`，后缀沿参考）：
- `repository-required`：有 sources 而无 `meta.repository`。
- `revision-invalid`：revision 不是完整的 40 位十六进制。
- `url-invalid`：
  - 含空白、`\`、控制字符、`?`、`#`，或含点段/空段；
  - SSH 用户不是 `git`，或带密码；
  - 任何 HTTP(S) userinfo（evidence 字段写 `credentials`）。
  - 诊断沿参考不回显作者给的 URL；畸形 URL 中的凭据无法可靠脱敏。
- `provider-invalid`：provider 与主机不符。
- `links-unsupported`：web 模式要求 GitHub/Gitee/gitlab.com，或显式 `provider: "gitlab"` 的自托管主机，且为 https、端口 443、`owner/repo` 路径（GitLab 允许嵌套组）。
- `path-invalid` / `path-escape`：前导 `/`、反斜杠、控制字符或换行；空段、`.`、`..`，或首段为 `.git`。
- `line-required`、`line-range-invalid`。

legacy-2.17 architecture 的 repository/sources（provider 仅 github|gitee）走同一校验。

**核对**（`evidence/git.rs`）：
- 进程设置：`Command::new("git")`，参数用数组、不经 shell；清除继承的全部 `GIT_*` 环境变量，再设 `GIT_NO_LAZY_FETCH=1`、`GIT_ALLOW_PROTOCOL=`（空值，禁止包括 file 在内的全部传输协议）、`GIT_TERMINAL_PROMPT=0`、`GIT_OPTIONAL_LOCKS=0`、`LC_ALL=C`；每条命令都带 `--no-replace-objects -C <root>`；每个 child 在独立进程组启动，单条命令上限 30 秒，超时杀掉整个进程组。
  - 两项防护分工：Git ≥ 2.45 由 `GIT_NO_LAZY_FETCH` 阻止对缺失对象发起懒抓取，更早版本不识别该变量；空 `GIT_ALLOW_PROTOCOL` 让懒抓取在传输层被拒、不写对象库（§1.1 本机实测），作为所有版本的兜底。
- 命令依次为：
  1. `version`
  2. `rev-parse --show-toplevel`：须与 repo-root 为同一目录（比较 device/inode），否则 `root-not-top-level`
  3. `remote get-url origin`：按参考身份规则比较，否则 `origin-mismatch`
  4. `config --get-regexp '^(extensions\.partialclone|remote\..+\.(promisor|partialclonefilter))$'`：版本 < 2.45 或无法解析，且存在任一匹配键（不解析取值，保守处理）时，全部记 `partial-clone-unsafe-git`，不读对象。只查 `extensions.partialClone` 不够：Git 可单凭 `remote.<name>.promisor` 或 `partialclonefilter` 注册 promisor（v2.44 `promisor-remote.c` 的 `promisor_remote_config`）
  5. `cat-file -e <rev>^{commit}`：失败为 `revision-unavailable`
  6. `rev-parse HEAD`：只用于 `headMatchesRevision`
  7. `cat-file --batch`：stdin 逐行写 `<rev>:<path>`，路径已排除换行与控制字符；缺失时用 `ls-tree -z <rev> -- <path>` 区分 `object-unavailable` 与 `file-missing`
- 行数按 CRLF/LF/CR 计，末尾换行不计一行（参考 `sourceLineCount`），超出为 `line-out-of-range`。
- 工作树比较：沿 `<path>` 逐段 `lstat`，不跟随符号链接。中间段须为目录、末段须为普通文件；任一段缺失、为 symlink 或类型不符即记 `worktree-differs`。全部满足时读末段字节与 blob 逐字节比较，不同也记 `worktree-differs`。不调用会读工作树的 git 命令（status/diff 会触发 fsmonitor 与 clean filter）。autocrlf/LFS 仓库因此可能被保守地判为 unverified。
- 未提供 `--repo-root` 时，所有来源记 `repo-root-not-provided`；git 不可执行时记 `git-unavailable`；单条命令超时记 `git-timeout`；`rev-parse --show-toplevel` 等基础命令失败（目录不是可读仓库）记 `repository-unreadable`。连同上文的 `root-not-top-level`、`origin-mismatch`、`partial-clone-unsafe-git`、`revision-unavailable`、`object-unavailable`、`file-missing`、`line-out-of-range`、`worktree-differs`，共 12 个原因码，各对应一条 `viewer.passport.evidence.reason.<code>` 文案。

**链接**：移植 `repositorySourceHref` / `repositoryTreeHref`：
- GitHub、Gitee：`{url}/blob/{sha}/{编码路径}#L{a}`；行区间 GitHub 写 `#La-Lb`，Gitee 写 `#La-b`。
- GitLab：`{url}/-/blob/{sha}/{路径}`，`.md` 带行号时加 `?plain=1`，区间写 `#La-b`。
- 只有 verified 且 web 模式才有 href；local-only 只显示路径与行。

### 5.4 渲染与产物检查

- `render_svg`：原五类走原路径。新五类先由 `render.rs` 写根属性（`data-diagram-type`、`data-reader-fit`、`data-input-contract`、类型编排）与 defs（erd/class 另嵌 `svg-relationship-markers.svg`），再调用对应 `body`，最后由 `render.rs` 写图例与 metadata。
- 图例与类型文案取 `i18n.rs`（键名沿参考 `legend.class.*`、`timeline.*`、`waterfall.*`、`tree.*`、`node.context.class.*`）。
- 用户文本、源路径、标签与 href 一律走现有转义；href 只来自 §5.3 的构造结果。
- `check_html` 继续用 22 个具名检查，按类型细化：
  - `semantic_content` 按 `metadata.facts` 比较可见文本：ERD 每列的名称、类型与键徽标；类的完整成员签名（跨行拼接后比较）；树节点标签与父子边数；事件标题、时间标签与泳道；跨度名称与时长标签。
  - 关系类检查覆盖 tree 直角边与 erd/class 关系；箭头类检查只作用于实际带 marker 的端。
  - timeline/waterfall 出现任何关系路径或 marker 即为 `artifact/semantic-count` 失败（“不显示为运行时因果”的机器断言）。
- showcase/standard 语义沿 P1：showcase 要求适用检查全部执行且零错误/零警告。

### 5.5 时间与数字格式

- `parse_timestamp` 复现 V8 对 schema 模式内字符串的行为：
  - 小时 0–24，24 只在其余字段全为零时有效，表示次日 00:00；分钟、秒 0–59；
  - 偏移小时 ≤23、分钟 ≤59；年份 0000–9999；
  - `valid_calendar_date` 独立判定日期，两者都通过才有效。
- `zone`：先按 ASCII 大小写不敏感匹配 IANA 名（含 `UTC`），得 `Zone::Named`；再查 ICU 77.1 额外接受而 tzdata 不列的标识（25 个 Java 三字母 ID、`US/Pacific-New`、`Canada/East-Saskatchewan` 及 6 个固定偏移的 `SystemV/*`），映射到 ICU 解析出的 tzdata 区或偏移完全相同的 `Etc/GMT±N`；带夏令时规则的 6 个 `SystemV/*DST` 只存在于 ICU，仍报无效时区；否则按 §1.1 实测的 Node 语法解析固定偏移，得 `Zone::Fixed`：
  - 符号为 `+`、`-` 或 U+2212，后接两位小时，可选两位分钟，分钟前冒号可有可无（`±HH`、`±HHMM`、`±HH:MM`）；小时 00–23、分钟 00–59；`-00`、`-00:00` 等于 `+00:00`；
  - 其他写法（带秒或小数、单位数小时、`Z`、`UTC+05:30`、首尾空白）报 `timeline/invalid-timezone`。
- 轴说明与 layout 报告沿参考使用作者原串：原串恰为 `UTC` 时只写 `UTC`，否则写原串加 `offset_label`，如 `+05:30 (UTC+05:30)`（`render-timeline.mjs:427`）。
- `wall`：按 `Zone` 由 chrono-tz 或 `FixedOffset` 计算瞬时的偏移与墙钟，年份 ≤0 按参考 `wall()` 换算 BC 纪年。
- 数字格式：
  - `en_us_max_fraction(v, 2)`、`en_us_max_significant(v, 3)`：取最短往返十进制串，在该十进制上做 half-expand 舍入，去尾零，整数部分千分位逗号，负号保留。
  - `js_to_precision(v, 3)` 按 ECMAScript 规则在精确值上舍入。
  - `js_to_string` 输出 ECMAScript 最短表示，|v|<1e-6 或 ≥1e21 用指数形式。
  - waterfall `number()` 按参考分支组合以上函数（`render-waterfall.mjs:131`）。
- oracle 至少包含：
  - 数字：`1.005`、`2.675`、`0.125`、`0.005`、`-0.5`、`1234567.891`、`0.001235`、`0.0012345`、`1.2345e-7`、`1.235e-7`；
  - 时间：`T24:00Z`、`T24:00:01Z`、`T23:60Z`、`+24:00`、`+23:59`、`2026-02-30`、`0000-01-01`、各时区夏令时切换边界；
  - 时区：大小写变体与 `Asia/Calcutta`；固定偏移 `+05:30`、`+0530`、`+01`、`-2359`、`−05:00`、`-00:00` 的接收、墙钟、刻度与轴说明；被拒的 `+24:00`、`+05:60`、`+5:30`、`+05:30:00`、`Z`、`UTC+05:30` 与带首尾空白者。

## 6. 阅读器与创作

### 6.1 十类探索（WORK-F5）

- 语义筛选、Route、Reach、查找、概览、展示、深链接与键盘沿已装配模块，Rust 为新五类输出它们依赖的节点 kind、边 from/to/key、显式 `data-edge-id` 与图例 kind。
- 树折叠只改阅读状态，导出与打印恢复完整树（参考 `export-cleanup.js:146-153`）。
- timeline/waterfall 没有已著关系：Route 显示 `viewer.route.noOutgoing`，Reach 按钮禁用并带“无上/下游”说明，不造关系。
- 深链接按现有 hash/query 恢复状态；清除焦点、退出展示或移除 hash 后回到完整图；`relation=` 只对有显式 id 的关系可用。
- 新增 ★ `tests/viewer/explore.test.mjs`，在现有 DOM fixture 上覆盖以下状态机：
  - 路由的无出边、同点、不可达；
  - Reach 空集；
  - 筛选第三类拒绝与非法 hash；
  - 身份冲突时快照 fail-closed；
  - 树展开/折叠与祖先揭示；
  - 深链接解析与清除；
  - 展示进出。

  真实浏览器走查由 V-7 承担。

### 6.2 范围卡片（WORK-F6/AC-2）

- 保留 `export.js` 的范围卡片实现。装配器把 `\bARCHIFY\b` 改为 `PHOENIX DIAGRAM`，使徽标为“PHOENIX DIAGRAM · ROUTE · {hops}”，`editorialPlate` 同步。
- 卡片回执须为 `data-last-export-canonical="false"`、variant 为 route/reach，文件名带 `-route-share-card` 或 `-{direction}-reach-share-card`。
- 没有有效快照时菜单项隐藏，导出返回 `routeRequired` / `reachRequired`。
- 导出卡片后，canonical 导出仍是完整图。

### 6.3 五种界面语言（WORK-N1/AC-4）

- `catalogs.json` 由装配器生成：读取参考五目录 → 品牌规范化 → 应用 `catalog-adaptations.json`。该文件补 zh-TW/es/ko 的 `viewer.export.downloadPngInstead`，并新增以下键的五语言文案：
  - `timeline.precision.day`、`timeline.precision.dayDisclosure`；
  - `viewer.passport.evidence.unverified`、`.unsourced`、`.localOnly`；
  - `viewer.passport.evidence.reason.<code>`（§5.3 每个原因一条）。
- 装配器 `--check` 断言：五目录键集合完全相同，占位符集合逐键一致。
- 运行时选择沿现有模板逻辑（`meta.locale` 大小写不敏感 → 覆盖 → en）。未支持语言回退 en，`html`/SVG `lang` 为 en，回执 `locale.fallback: true`。
- 作者内容不翻译；作者内容的语言由宿主按请求主要语言写入，属 V-11 范围。

### 6.4 来源显示（WORK-F3）

- `PhoenixDiagram.sourceEvidence` 改为读取 `metadata.sourceEvidence`，`focus.js` 的 `renderSourceEvidence` 按三态显示：
  - verified：“已核对来源 · label @ shortRevision”，web 显示 `rel="noopener noreferrer"` 链接，local-only 显示路径、行与“仅本地”；
  - unverified：“待核实”加原因文案，无链接；
  - unsourced：“未提供来源”。
- 节点 aria-label 追加来源数与状态；查找纳入来源路径。
- 导出清理移除来源属性（参考 `viewer/export-cleanup.js:76-84`，Phoenix 同名文件已移植该段）。
- 时间依据：observed/measured/illustrative 徽标与日精度标签在 SVG 内，打印/导出保留。

### 6.5 Skill 与说明（WORK-F2、C-2、C-3）

- 按 §0.6 修改，描述目标文本：“用于架构、工作流、时序、数据流、状态、ER、类、树、时间线、耗时瀑布图及 Mermaid 转换；不用于数值图表、仪表盘或其他 infographic。生成经 Rust 与浏览器检查并实际看图的独立交互 HTML。”实施时核对完整长度 ≤200 字符、前 100 字符能独立辨认边界，超限则在不丢边界的前提下精简。
- 正文的最短创作路径不变，新增以下规则：
  - 新五类的 schema/example 指引；
  - 仓库事实图先读 `repository-authoring.md`，首稿即带来源并用 `finalize ... --repo-root <仓库根>`；
  - 交付答复须说明 `evidence.status` 非 verified 的元素与原因、示意时间、工作树改动路径与语言回退。
- `docs/phoenix-diagram.md` 与 README 第 238 行改为十类候选说明，并写明 `--repo-root` 与来源三态；不动 archify 相关行。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| WORK-N1 | PRD 四桌面尺寸、默认实际看图；P1 实测 | 十类在 1440×900、1600×1000、1920×1080、2048×1320 无水平溢出/遮挡，纵向只按声明例外；390×844 窄屏完整；五语言界面无截断 | 指出尺寸、主题、语言与对象；无法审核保留未完成 | §5.2 阅读合同、§6.3 | V-7、V-9、V-13 |
| WORK-N2 | PRD Rust 核心 | 新五类正常路径无 Node 图解引擎 | 身份不匹配拒绝 | D1、§5.2；oracle 只在开发测试 | V-3、V-4、V-12、V-13 |
| WORK-N3 | 冻结 25 份参考 JSON、15 份 HTML | 核心事实全保留；原文件字节不变 | 缺口明确且不计兼容通过 | D2、§3.1 | V-1、V-8 |
| WORK-N4（P1 起，保持） | P1 确定性与提交保护 | 含来源时仍确定性；输出事务不变 | 沿 P1 | §3.3 evidenceSha256 | V-7 |
| WORK-N5（P1 AC-1 保持） | P1 内容安全与离线 | 新文本字段、来源路径与链接只作内容；核对零网络 | 恶意字段拒绝；任何外部请求即失败 | §5.3 结构门、转义、env 清理与禁传输、部分克隆保护 | V-5、V-7 |

安全逐项：
- 凭据 URL 在输入门拒绝，不进入 HTML、回执或诊断原文。
- git 调用不经 shell；路径进入 batch 协议前排除换行与控制字符；继承的 `GIT_*` 环境被清除，并以空 `GIT_ALLOW_PROTOCOL` 禁止全部传输。
- 工作树比较不跟随 symlink。
- href 只由已校验的 https 基址与百分号编码的路径构造。
- 新五类全部用户文本走 P1 的文本/属性/script JSON 转义；XSS 字面量用例扩到 ERD 类型、类签名、事件标题、跨度名与来源路径。

可观测性只有本地回执、stderr 与浏览器证据，不新增遥测。性能没有已确认 SLO；V-7 记录每类实际耗时与输入规模，大 ERD 的有界搜索失败时给布局诊断，不截断。

## 8. 失败模式、发布与回滚

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现/降级 | 检测 | 恢复/补偿 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| 凭据 URL、路径逃逸、非法行区间 | 当前 candidate | 不写输出 | input 失败并给出 pointer | 结构门 | 作者去凭据或修路径 | V-5 |
| 缺 repo-root、git 缺席/超时、提交或对象缺失、工作树不同、旧 git 遇部分克隆 | 当前交付的来源维度 | HTML 正常；来源为 unverified/partial | 详情显示“待核实”与原因，回执、summary 带 `evidence.status` | 核对结果 | 提供干净检出或正确根目录后重新 finalize | V-5、V-9、V-11 |
| 时区或时间戳无效、计时矛盾、树/继承成环 | 当前 candidate | 不写输出 | 具体诊断 | 语义门 | 作者按记录修正 | V-1、V-4 |
| 大 ERD/class 在 showcase 下布局失败 | 当前 candidate | 旧 HTML 保留 | 容量/布局诊断，按停滞规则停止；作者明确要求时可用 standard | layout 诊断 | 定向修复或用户选择 standard | V-3、V-11 |
| 未支持界面语言 | 当前产物 | 无 | en 界面，回执 `locale.fallback: true` | i18n 解析 | 宿主说明回退 | V-6、V-9 |
| 范围卡片无有效快照或身份冲突 | 单次导出 | 无 | 菜单隐藏或明确拒绝 | Route/Reach 快照校验 | 读者重选路线 | V-6、V-7 |
| CI 平台缺结果、宿主配额/超时 | 收口 | 无 | 对应检查未完成 | CI 原始结果、会话台账 | 补跑受影响项，不以其他平台/宿主代验 | V-11、V-12 |
| 候选后修改 skill 文本 | 候选身份 | bundle 与 tgz 改变，引擎不变（D8） | 需新候选 | bundle/引擎摘要 | 按 §9.3 规则只重跑受影响证据 | V-13 |

发布与回滚：
- 本期只产出 candidate，不公开发布；P1 的联合发行 guard 保持。
- 回滚即撤销本期仓库改动；P1 候选与已生成的 HTML 原样保留。
- 无 schema 数据迁移；旧 JSON/HTML 不被改写。

## 9. 验证

所有命令从仓根执行；★ 脚本、测试与参数由所属里程碑交付。Rust 测试通过 lib/bin 的公开行为与真实临时文件、真实 git 仓库观察结果；oracle 只在开发测试中使用冻结 Node 来源。正例按语义、几何与可读性断言，不要求与参考 HTML 字节一致。下表 `mbx test` 均为 `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline` 后接列出的 `--test`。

### 9.1 检查定义与唯一执行时点

| 检查 ID | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | WORK-F1/AC-3、WORK-N3/AC-2（输入层）：<br>- 十类 schema 有效性与冻结 3.1.0 生成的 validator 一致（含 D7 扩展字段）；<br>- 合同缺省与 `input/contract-unsupported`；<br>- 新五类语义门诊断码与 pointer；<br>- 25 份参考 JSON 在对应合同下 schema 与语义均有效；<br>- 延期字段改期 | 先按现有 `tools/phoenix-diagram/tests/generate-input-oracle.mjs` 扩到十类并重生成 `tools/phoenix-diagram/tests/fixtures/input/`；`mbx test` `--test input_contract` | M1 局部；解锁 M2–M4 |
| V-2 | D6、D8、C-1：<br>- probe 打印 0.2.0/phoenix-v2；<br>- 只改 SKILL.md 时 sourceInputSha256 不变；<br>- `catalogs.json` 与模板同步，五目录键集合与占位符一致、无 Archify/ARCHIFY 残留；<br>- i18n 解析与回退；<br>- 新依赖许可全文齐备 | `node scripts/assemble-phoenix-diagram-reader.mjs --check`；`mbx test` `--test i18n --test cli`；`node --test test/phoenix-diagram/build.test.js tools/phoenix-diagram/upstream/license-sources.test.mjs` | M1 局部 |
| V-3 | WORK-F1/AC-2、WORK-F4/AC-2、WORK-N2/AC-2（erd、class）：<br>- 列、键、references、基数、可选、identifying、类成员与六种关系语义保留；<br>- 几何与 Reader 属性与 oracle 一致；<br>- showcase/standard 检查实际范围；<br>- architecture 在共享路由下仍与既有 oracle 一致 | ★ `tools/phoenix-diagram/tests/fixtures/{erd,class}/generate.mjs` 生成 oracle；`mbx test` `--test erd --test class --test architecture --test check_anchors` | M2 局部 |
| V-4 | WORK-F1/AC-2、WORK-F4/AC-2、WORK-N2/AC-2、WORK-F3/AC-4（tree、timeline、waterfall）：<br>- 单父层级与折叠属性；<br>- 比例轴、断轴、时区说明、证据徽标、日精度；<br>- 跨度重叠/嵌套/incomplete 下界；<br>- 无关系路径；<br>- §5.5 全部边界与 Node 一致，固定偏移时区从接收、墙钟、刻度/轴说明到完整渲染均与 oracle 一致 | ★ `tools/phoenix-diagram/tests/fixtures/{tree,timeline,waterfall}/generate.mjs` 生成 oracle；`mbx test` `--test tree --test timeline --test waterfall --test format --test check_anchors` | M3 局部 |
| V-5 | WORK-F3/AC-1–AC-3、WORK-F4（来源维度）、WORK-N5（保持）：<br>- 真实临时 git 仓库覆盖十类各一条来源，以及 web、local-only、SSH origin、GitHub/Gitee/GitLab/自托管链接格式；<br>- 每个 unverified 原因（含 dirty 工作树、缺提交、缺文件、越界、无 git、无 repo-root）；<br>- 工作树路径：正常嵌套文件 verified；中间段 symlink、末段 symlink、末段非普通文件均为 `worktree-differs`；<br>- 零网络：用本地 `file://` 源 `clone --filter=blob:none` 造只带 `remote.<name>.promisor`/`partialclonefilter` 的缺对象仓库（源可达，任何懒抓取都会写入）。分别在本机 git、PATH 前置包装模拟 < 2.45（报旧版本号并删去 `GIT_NO_LAZY_FETCH` 后转发）、只删该变量三种条件下核对：原因码正确，缺失对象仍缺失；超时时整个进程组被终止。有真实旧版 git 时加跑并记版本，否则记未运行，不以模拟冒充；<br>- 凭据与路径逃逸拒绝；<br>- status 计算；evidenceSha256 进入 configSha256 并保持确定性；<br>- 严格当前性拒绝篡改的 `sourceEvidence`；<br>- 来源三态的 Reader 显示 | `mbx test` `--test evidence --test delivery`；`node --test tools/phoenix-diagram/tests/viewer/evidence.test.mjs` | M4 局部 |
| V-6 | WORK-F5/AC-2、AC-4，WORK-F6/AC-2，WORK-N1/AC-4（Reader 层）：<br>- §6.1 状态机；<br>- 范围卡片 canonical=false 与徽标；<br>- 五语言目录与回退；<br>- 树导出为完整树；<br>- 导出控制器对范围卡片与新五类的格式范围、文件名与 canonical 元数据（单测不启动 Chrome；真实格式文件与浏览器解码归 V-7） | `node --test tools/phoenix-diagram/tests/viewer/unit.test.mjs tools/phoenix-diagram/tests/viewer/explore.test.mjs tools/phoenix-diagram/tests/viewer/catalogs.test.mjs tools/phoenix-diagram/tests/export-acceptance.test.mjs` | M5 局部 |
| V-7 | 阶段 A 收口。WORK-F1、F4、F5、F6、N1 全部 AC，以及 N2/AC-2、N4/N5 保持：<br>- 原五类 × 两合同加新五类 × phoenix-v1，共 15 例，各跑真实生成 → 四门 → 五视口 × 两主题；<br>- Agent 逐张实际查看暗亮截图后 `review-record` → complete；<br>- 每例（含新五类的五种格式）20 次真实导出，核对文件签名并经浏览器解码；PNG 剪贴板与能力失败分支、八个 reduce/hidden 样本、打印；<br>- 每类一条探索剧本（★ `support/explore-reader.mjs`：筛选、Route 有路/无路、Reach 有/空、树折叠、概览定位、展示进出、深链接重载再清除、键盘可达、详情关闭），以及作者源文件 sha256 不变；<br>- 有关系的八类各导出一张 Route 卡片与一张 Reach 卡片，卡片文件同样核对签名与浏览器解码；<br>- 确定性重放、XSS、离线与 P1 八项输出恢复演练 | 先以 `node --test test/phoenix-diagram/core-runner.test.js` 锁定 M6 的 runner 增量（15 例选择、新五类无 legacy 用例、新开关缺省 not-run）。再在本机 darwin-arm64 用真实 Rust + Chrome 依次执行，命令均经 `mise exec --no-deps --`，`<n>` 为本轮序号：<br>1. 引擎：`node scripts/build-phoenix-diagram.mjs --development --output-dir local/phoenix-diagram/p2-native-<n>`；以下 BIN 为其中的 `phoenix-diagram-darwin-arm64`，各步显式传 `--engine BIN`，不依赖 `PHOENIX_DIAGRAM_BIN`；<br>2. 初跑：`node scripts/test-phoenix-diagram.mjs --stage core --engine BIN --output-dir local/phoenix-diagram/p2-core-<n>`，15 例真实 finalize，视觉保持 required-pending；<br>3. 收口范围：同入口加 `--resume <最新 index> --reader --recovery --offline` 与 ★ `--explore --corpus --evidence --locale`，不再 finalize 原例；<br>4. 看图：Agent 逐张打开最新 index 列出的 15 例暗亮截图，按 `review-record` 的记录格式写入运行目录的 `actual-reviews/`，再写 review map `{"records":{"<case-id>":"<记录绝对路径>"}}`；V-8/V-9 片段截图同样逐张查看，按截图 sha256 在 M6 记录登记观察，不进 review map；<br>5. 登记：同入口加 `--resume <最新 index> --review-file <review map>`，原位 review-record。<br>每次续接都用上一步产生的最新 index；失败只修受影响项再续接。未启用或未运行的范围保持 not-run，handoff 不得为 passed；不为取得 complete 重新 finalize | M6，阶段 A 收口 |
| V-8 | WORK-N3/AC-2、WORK-F1/AC-2（真实规模）：<br>- 25 份参考 JSON 中，原五类显式 `--input-contract phoenix-v1`，新五类缺省；<br>- deliver 与严格检查通过，核心事实（id、方向、标签、显式几何）逐项与输入一致；<br>- deferredFields 只含 brand/views/animation；<br>- lifecycle：`deployment-release` 按 legacy-2.17 v1 与参考 v3 两份分别核对；<br>- 原 JSON 与 15 份参考 HTML 前后 sha256 不变；<br>- 25 份参考 JSON（原五类 15 份与新五类 10 份）均按原样交付并经 browser-check，在 1440×900 暗亮下实际看图；<br>- 参考 ERD 中有多个外键的表，在 1440×900 与 390×844 打开详情，逐列显示“列 → 目标”且无截断，截图实际看图；<br>- 三个长外键的合成 ERD 在 390×844 暗亮打开详情，面板不压导航条，全部目标可读；<br>- 详情检查在 390×844 按触摸屏模拟，每次都以真实触摸（桌面为点击）启动一个可用的 Reach，按钮须命中自身 | V-7 第 3 步的 `--corpus`；看图按 V-7 第 4 步登记观察 | M6，阶段 A 收口 |
| V-9 | WORK-F3 全部 AC、WORK-N1/AC-4（端到端）：<br>- 本仓固定提交的 architecture 与 timeline 来源 verified，用 `gh api repos/XGENT-ai/skills/contents/<path>?ref=<sha>` 只读核对 href 指向的文件存在；<br>- 修改一个被引用文件后为 worktree-differs；<br>- local-only tree；<br>- illustrative waterfall 与日精度 timeline；<br>- 来源三态在详情中显示，并实际看图；<br>- 五语言各一张 timeline 加 `fr` 回退，界面无截断并实际看图 | V-7 第 3 步的 `--evidence`、`--locale`；看图按 V-7 第 4 步登记观察；`gh` 已登录，网络缺席时记链接存在性未运行 | M6，阶段 A 收口 |
| V-10 | C-2、C-3、§0.6：<br>- skill 描述与前 100 字；<br>- 类型表、schema 与示例一致且十类示例均能 finalize；<br>- 接口复述同步（`--repo-root`、十类）；<br>- 十类与 0.2.0 的脚本/测试列表；<br>- P2 聚合配置单测；<br>- §0.6 逐行对照 diff；<br>- archify 与调用方未被修改 | `node --test test/phoenix-diagram/skill.test.js test/phoenix-diagram/package.test.js test/phoenix-diagram/build.test.js test/phoenix-diagram/core-runner.test.js test/phoenix-diagram/aggregate.test.js test/phoenix-diagram/candidate.test.js test/phoenix-diagram/workflow.test.js`；`grep -rn 'phoenix-diagram' README.md docs skills --include='*.md'` 人工核对；`git diff --stat -- skills/archify skills/elintp skills/documentation-writer` 为空 | M7 局部 |
| V-11 | WORK-F2 全部 AC、WORK-F3（创作侧）、WORK-F4/AC-3（宿主报告）、WORK-N1/AC-4（作者语言）：§9.2 场景在真实新会话中行为正确 | development 包：`mise exec --no-deps -- node scripts/build-phoenix-diagram.mjs --development --output-dir DIR --sync-dist`（★ `--sync-dist`），再 `mise exec --no-deps -- npm pack --json --pack-destination DIR`；在临时无 Rust 项目安装后，分别在已登录的 Claude Code、Codex 全新会话中发起 | M7；结果按 §9.3 绑定规则供 M8 复用 |
| V-12 | WORK-N2/AC-2 与候选四平台：同一 tgz 的四原生构建与测试、在线/离线安装、十类 15 例自动四门全过（视觉 required-pending），精确 Node 18.20.8 自动范围；另外三平台新五类 1440×900 暗亮截图由 Agent 实际查看并记为观察 | `.github/workflows/phoenix-diagram.yml` 默认 `workflow_dispatch`（`await_review=false`）；记录 run/commit/artifact digest | M8，阶段 B 收口 |
| V-13 | 阶段 B 收口。P2 全部 14 条 AC 与七条终验需求在最终候选上可追溯：<br>- darwin-arm64 从最终 tgz 跑 distribution，完成 15 例 review-record complete、Reader/导出/卡片/来源/语言子集与 npm 入口输出恢复；<br>- 宿主证据按 §9.3 复用或重跑；<br>- P2 聚合 handoff=passed | `node scripts/test-phoenix-diagram.mjs --stage distribution --candidate-dir DIR`；`node scripts/aggregate-phoenix-diagram-candidate.mjs --profile p2 --input FILE --output FILE`（★ `--profile p2`） | M8，阶段 B 收口 |
| V-14 | C-3、C-4：仓库轻量测试、Phoenix UI runtime 与 diagram 全量、fmt/clippy、锁定依赖与干净构建 | `npm test`；`npm run test:phoenix`；`npm run test:diagram`；`mise exec --no-deps -- mbx fmt --manifest-path tools/phoenix-diagram/Cargo.toml -- --check`；`mise exec --no-deps -- mbx clippy --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --all-targets -- -D warnings` | M8，阶段 B 收口；全量只在此执行 |

### 9.2 宿主场景与角色约束

角色模拟由 Agent 执行，结果标“角色模拟”，不推断真人成功率。
- 首次作者只知道请求与题目材料，可读已安装 skill 的 schema/example/references 与快速入门；不读 renderer、测试断言或实现说明。
- 仓库场景的作者可读给定仓库切片。
- 审核者对照被引用行判断是否支持断言，记录不支持项。
- 不从历史会话提示答案，题面不透露评分词。

| 行 ID | 场景 | 期望 | Claude Code | Codex |
| --- | --- | --- | --- | --- |
| intent-{architecture,workflow,sequence,dataflow,lifecycle} | 五类自然意图 | 选型正确、只用给定事实 | 跑 | intent-workflow 抽样 |
| missing-{date,amount,condition} | 缺事实 | 只问开放问题后停止 | 跑 | — |
| mermaid-{flowchart,graph,sequence,state} | 四类 Mermaid | 保拓扑、方向、条件、消息含义 | 跑 | mermaid-sequence 抽样 |
| repairable-diagnostic、stalled-repair | 修复与停滞 | 定向修复；连续两轮不优于历史最佳即停 | 跑 | stalled-repair 抽样 |
| browser-unavailable、browser-failed、visual-pending | 失败报告 | 状态分列，不称完整 | 跑 | — |
| legacy-lifecycle | 旧 v1 JSON | 按 legacy-2.17、原字节不变 | 跑 | — |
| intent-{erd,class,tree,timeline,waterfall} | 新五类自然意图 | 选型正确；ERD 列出全部物理列与键；时间带偏移 | 跑 | 跑 |
| mermaid-er、mermaid-class | erDiagram、classDiagram | 保实体、键、基数、类型与关系种类 | 跑 | 跑 |
| repo-verified | 解释本仓一段真实流程（临时干净检出、固定提交） | 主要元素带来源，`--repo-root` 下 verified；审核者确认引用行支持断言；答复简短 | 跑 | 跑 |
| repo-dirty | 同上，但某被引用文件有未提交改动 | 答复列出改动路径并说明 unverified，不称已核对 | 跑 | 跑 |
| repo-timeline | 用本仓 CHANGELOG/提交时间作时间线，部分记录只有日期 | observed + 日精度，来源指向固定提交 | 跑 | 跑 |
| waterfall-illustrative | 无 trace 的请求耗时 | `illustrative`，答复说明示意 | 跑 | 跑 |
| reference-31-json | 用户说明 JSON 来自 Archify 3.1 的 workflow | 显式 `--input-contract phoenix-v1`，不改写原 JSON | 跑 | 跑 |
| locale-zh-tw | 繁体中文请求 | 内容为繁体，`meta.locale: "zh-TW"`，界面为繁体 | 跑 | 跑 |

Claude Code 共 31 行，Codex 共 16 行。一次真实会话可覆盖多行。任一 Codex 行与 Claude Code 同行行为不一致时，Codex 补跑全部回归行（D10）。图像审核者必须实际打开截图；截图缺失、身份错或图像工具不可用记为未完成。

### 9.3 收口环境、证据复用与留存

- V-7–V-9 在本机 darwin-arm64、隔离 cache/home/项目下运行。离线段沿 P1：macOS 用 sandbox-exec 禁 IP，Chrome sandbox 限制单列。
- V-12 的另外三个平台只做自动消费与截图观察，不做 `review-record`（ADR-2）。
- **宿主证据复用**：仅当 V-11 所用 development 包与最终 tgz 同时满足以下两条时才可复用；否则只重跑受影响行。
  - bundle 中除 `ENGINE.json` 外每个文件的 sha256 相同；
  - 引擎 `sourceInputSha256` 相同。
- **看图复用**：最终候选的截图 sha256 与 M6 已实际查看的截图相同时，记录可引用原观察并对新交付调用 `review-record`；不同则重新查看。产品工具“新交付不继承旧审核”的语义不变。
- **留存**：只保存证据索引、回执、截图、导出文件与必要日志，按 run 归档在 gitignored 的 `local/phoenix-diagram/p2-*` 下；不下载完整 CI ZIP 到本机，除非排错需要（P1 本机证据曾达 14G 并发生 ENOSPC）。
- 未准备环境、运行失败、断言失败与无法看图分别记缺口。

## 10. 里程碑与验收安排

- 阶段 A（M1–M6）完成新增五类、来源核对与十类 Reader 的本机原生闭环；阶段 B（M7–M8）完成 skill、宿主与四平台候选。
- 主 agent 统一维护以下共享契约：`input.rs`、`scene.rs`、`render.rs`、`check.rs`、`delivery/*`、目录与装配器、脚本与 workflow。
- M1 稳定后，M2、M3、M4 可并行；三者的类型文件互不重叠，共享文件的增量由主 agent 集成。浏览器、端口与临时项目串行或按进程隔离。

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 十类输入合同、身份与共享基础 | 无 | `input.rs`/`semantics.rs`/`schema.rs`/`cli.rs` 的 D1/D2 与 §3.1（含 D7 字段、延期改期、guide/HELP）；★ 五份 schema；`scene.rs` 的 §3.2 类型；★ `i18n.rs` 与装配器输出 `catalogs.json`（D6 品牌规范化、缺键与新键）；D8 版本与 INPUTS 收窄并同步脚本/测试常量；D4 依赖加入 Cargo.toml/lock 与许可；D11 `--sync-dist`（新 schema 进入 skill 后，V-2 的 `build.test.js` 即要求 dist 与 skill 一致，故提前到本里程碑）；timeline 输入门所需的 `format/time.rs` 解析部分（`parse_timestamp`、`valid_calendar_date`、`zone`）。照 P1 输入门与 `generate-input-oracle.mjs` 范式 | V-1、V-2 通过；回写「实施进度」 |
| M2 | erd 与 class | M1 的 V-1；共享路由先于两类 | 先把 `layout/architecture/router.rs` 提为 ★ `layout/router.rs` 并补七项选项（V-3 的 architecture 部分先绿）；再做 ★ `layout/erd.rs`、`erd/grid.rs`、`class.rs`，★ `render/{erd,class}.rs`、★ `svg-relationship-markers.svg`、`check.rs` 对应增量、★ `examples/minimal.{erd,class}.json`、oracle 生成器；erd 与 class 在 grid 稳定后可分人 | V-3 通过；回写「实施进度」 |
| M3 | tree、timeline 与 waterfall | M1 的 V-1；与 M2 并行 | ★ `format/{time,number}.rs`（先绿 `--test format`），★ `layout/{tree,timeline,waterfall}.rs`、★ `render/{tree,timeline,waterfall}.rs`、`check.rs` 对应增量（含无关系断言）、★ 三份 minimal 示例、oracle 生成器；三类互不等待 | V-4 通过；回写「实施进度」 |
| M4 | 仓库来源核对与显示 | M1 的 V-1；与 M2/M3 并行，新类型的端到端渲染断言待 M2/M3 | ★ `evidence.rs` 与子模块（§5.3）、CLI `--repo-root`、`delivery/*` 回执/当前性/配置摘要、`render.rs` 的 `sourceEvidence`、Reader 三态显示与 ★ `tests/viewer/evidence.test.mjs`；照参考两 `.mjs` 与 P1 delivery 回执范式 | V-5 通过；回写「实施进度」 |
| M5 | 十类 Reader 探索、范围卡片与五语言 | M2、M3 的 Reader 属性（V-3、V-4）；M4 的显示（V-5） | §6.1–§6.3：补齐 Rust 输出的探索属性、`lang` 与回执 `locale`、★ `explore.test.mjs`、★ `catalogs.test.mjs`、`export-acceptance` 的控制器单测增范围卡片与新五类格式范围（真实导出留 M6） | V-6 通过；回写「实施进度」 |
| M6 | 阶段 A 收口：本机十类原生交付、阅读与来源 | M1–M5 的 V-1–V-6 | 先交付 core 收口 runner：`test-phoenix-diagram-core.mjs` 的十类与 15 例覆盖（legacy-2.17 只要求原五类）及 ★ 四个范围开关的参数与 HELP；core 入口委托的 `test-phoenix-diagram-runtime.mjs` 中 `DEFAULT_CASES`（新五类只建 phoenix-v1 用例）与 corpus/evidence/locale/explore/卡片段的执行；★ `support/explore-reader.mjs`；`test/phoenix-diagram/core-runner.test.js` 对应用例。再按 V-7 的命令序列运行 core stage，实际看图与 review-record，失败只修受影响项并重跑失效检查；主 agent 负责 | V-7、V-8、V-9 通过，阶段 A 产物可交付；回写「实施进度」 |
| M7 | skill、说明、包脚本与宿主验收 | M6 通过（引擎行为稳定）；§0.6 清单 | 按 §0.6 与 §6.5 改 skill/references/docs/README；按 D12 改进 workflow 布局修复与可执行诊断（改后重做 M6 收口）；runtime 的包消费/CI 增量（core 用例与收口段已在 M6）、`candidate`（包内十类 manifest，并为 V-13 启用所需范围开关）/`aggregate --profile p2`/`pack`/workflow 与其余 `test/phoenix-diagram/*` 的十类与 0.2.0；development 包上跑 §9.2；skill 修订后只重跑受影响行 | V-10、V-11 通过，skill 文本冻结；回写「实施进度」 |
| M8 | 四平台候选与 P2 收口 | M7 冻结的源码；V-10、V-11 | 提交候选源码 → 四平台 CI → 唯一 tgz/release-dir → 本机 distribution 收口 → 宿主证据绑定核对 → P2 聚合 → 全量回归；必要修复只重跑失效证据 | V-12、V-13、V-14 全通过，14 条 P2 AC 与七条终验需求可追溯；N2/N3/F11/N4/N5 未冒充终验；回写「实施进度」 |

提交点：M6（阶段 A 收口）、M8（阶段 B / 全部完成）。M8 为取得同 SHA 的 CI 制品，可先提交候选代码，但验收通过前不记完成；最终修复产生新 SHA 时，按 §9.3 只重取受影响证据。

8 个里程碑在 PRD 建议的 5–8 范围内；新增五类按共享路由与是否依赖时间格式分两组，来源核对与 Reader 各有独立接口与风险，没有按工种拆分。

### 10.1 内部任务

| 任务 ID | 所属里程碑 | 交付物/影响路径 | 前置成果 | 检查 ID |
| --- | --- | --- | --- | --- |
| T-1 | M2 | ★ `layout/router.rs` 与七项选项；architecture 改用共享路由且结果不变 | M1 场景类型 | V-3（architecture 部分） |
| T-2 | M2 | erd 布局/渲染/检查/示例/oracle | T-1 | V-3（erd 部分） |
| T-3 | M2 | class 布局/渲染/检查/示例/oracle | T-1、T-2 的 `erd/grid.rs` | V-3（class 部分） |
| T-4 | M3 | `format/time.rs` 的 `wall`/`offset_label`、★ `format/number.rs` 与 oracle（解析部分已随 M1 输入门交付） | M1 依赖锁 | V-4（`--test format`） |
| T-5 | M3 | tree 全链路 | M1 | V-4（tree 部分） |
| T-6 | M3 | timeline 全链路（含 D7） | T-4 | V-4（timeline 部分） |
| T-7 | M3 | waterfall 全链路 | T-4 | V-4（waterfall 部分） |

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| 参考新五类几何在 Phoenix 更严格的具名检查下不过 | M2/M3 工作量上升 | 实施者按 §5.2 末段的规则适配并记录偏差，不放宽检查 | M2/M3 退出 | 否 |
| 共享路由提取改变 architecture 结果 | P1 已验收行为回退 | T-1 先以既有 architecture oracle 锁定，再接 erd/class | T-1 | 否 |
| 宿主配额、会话超时或 CI runner 不可用 | V-11/V-12 延后 | 实施者记录缺口、推进其他工作；按 §9.3 只补受影响行 | M7/M8 退出 | 不阻塞开发；阻塞终验 |
| 新增来源/精度文案的 es、ko、zh-TW 质量 | 界面语言质量 | Agent 撰写，V-9 实际看图核对截断与术语一致；真人语言质量不在本期声明 | M6 | 否 |
| ADR-2 对三平台只做截图观察 | 平台特有视觉缺陷可能晚发现 | 按 ADR-2 的重新评估触发恢复 await_review | M8 | 否 |
| 本机磁盘与代理（P1 曾 ENOSPC、DNS 需代理） | 收口中断 | §9.3 留存规则；沿 P1 已知代理配置 | M6/M8 | 否 |
| 联合公开发行仍未授权 | 不能宣称公开可安装 | 沿 P1 §8 guard | 任何公开发布前 | 不阻塞本期 |

- 最终状态：**Ready**。
- 定级理由：
  - 范围按 PRD P2 的 14 条 AC 与七条终验需求逐条落到设计与检查；
  - 新增五类的参考算法、诊断与 Reader 属性已核到冻结源码；
  - 时间与数字格式的 Node 边界（含固定偏移时区）已实测，依赖已离线构建验证；
  - 来源核对沿参考的禁懒抓取与禁传输双重防护，按 Git 2.45 能力边界和全部 promisor 配置判定旧版降级，并补齐按段类型的工作树比较与“待核实”表达；
  - 每个局部退出检查都能由所列命令证明；阶段 A 收口的 runner 增量与完整命令序列归 M6，不依赖 M7；
  - 收口成本相对 P1 的降低（ADR-2、D10）有明确代价与恢复触发。

  不存在影响安全、数据或外部契约的未决项；未来实测结果是退出条件，没有被当成既有事实。
- 待拍板问题：无。若希望恢复 P1 级的四平台逐图审核或 Codex 全量回归，修改 ADR-2/D10 并调整 V-12/V-11 即可，不影响其他设计。

## 12. 已知坑与历史教训

- 改 SKILL 文本曾连带改变引擎身份并迫使四平台与宿主全量重跑；D8 收窄 INPUTS 后，仍须在候选前冻结 skill（P1 M5 记录 R3 行、M7 记录）。
- dist 内 SKILL.md 未同步曾导致 pack 失败；用 D11 的 `--sync-dist`，不再手工复制（P1 M7 记录 :521）。
- ICU4X 与 Node Intl 即使数据同代，算法也有差异；本期新五类不用 Segmenter，但 timeline/waterfall 依赖 `Intl.DateTimeFormat` 与 ICU 数字格式，必须用冻结 Node 生成边界 oracle，不凭文档推断（P1 M3 记录 :7–24；本计划 §5.5）。
- 参考说明与源码可能不一致（P1：Live 默认 vs 实际 Still）；本期的 `meta.views` 以参考 schemas README 的“accepted and ignored”与 Reader 装配列表为准，不重建 guided views。
- 参考为未绘制的来源标记预留 38px（workflow/dataflow/lifecycle）与 18px（tree）空间，Reader 并不绘制；本期不移植，布局与来源无关（参考 `utils.mjs:101`、`render-tree.mjs:225`）。
- 装配器的品牌替换区分大小写，漏掉 “ARCHIFY”；D6 统一在 `catalogs.json` 处理，V-2 断言零残留（`scripts/assemble-phoenix-diagram-reader.mjs:181`）。
- CI 活 job 等待看图曾超时与取消，需重新 dispatch；本期不用 await_review（ADR-2；P1 M7 记录 :629–631）。
- 多重硬链接的 binary 会触发 doctor 的 `engine/current-binary`；Ubuntu 24.04 的 AppArmor 会挡 user namespace；npm 11.6.2 曾报 ECOMPROMISED，已固定 npm 10.5.0。沿用 P1 workflow 现有处理，不改动这些步骤（P1 M7 记录 :357–361、:447–475）。
- mise 可能静默选到系统 Node 19；维护命令一律经 `mise exec --no-deps`，必要时设 `MISE_ENV_CACHE=0`（P1 M5 记录 :76、M7 记录 :421）。
- `npm test` 不覆盖原生四平台、浏览器与宿主；按 C-3 分别留证（`AGENTS.md` Working in This Repository）。
