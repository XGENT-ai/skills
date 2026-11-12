# AI Native SDLC：用 XGENT Skills 串起从想法到交付的流水线

本文是这套流水线的操作说明：人提出 idea、确认 spec、完成最终 UAT，并手动启动各个 skill；agent 完成需求推演、技术规划、实施、测试、评审、修复和交付核查。下文说明每个 skill 何时启动、读什么、完成什么、写出什么、什么时候停。

流水线的源在本仓 `skills/`。安装到目标项目用 `npx skills add XGENT-ai/skills`（[README.md](../README.md)）。下一环节以落盘文件或 SPMS 实体及可追溯决定为依据，不依赖上一轮对话才能理解。

## 快照信息

| 项 | 值 |
| --- | --- |
| 仓库 | https://github.com/XGENT-ai/skills.git |
| 分支 / 提交 | `main` / `f281e765`（2026-11-10） |
| 版本 | `0.7.12`（[package.json](../package.json)） |
| 许可证 | MIT（[LICENSE](../LICENSE)，[package.json](../package.json)） |
| 调查范围 | 以本仓 `skills/` 源文件为准，核对需求、spec 确认、计划、实施、测试、代码/UI 评审与差距回补；SPMS 字段与状态单列为集成约束 |

`skills/` 是源文件；`.agents/skills/`、`.claude/skills/` 等是本地安装，不以安装目录数量判断仓库能力。

## 1. 这是什么

从一个想法出发，人手动启动当前需要的 skill，由 agent 把这一环节做完并交出下一步入口。coding agent 的 goal 机制负责按计划持续实施。人的实质参与集中在 idea、spec 确认和最终 UAT（用户验收测试）；开发过程中的调查、测试、独立评审、修复、补证和进度回写由 agent 执行。

这套 SDLC 可以只用目标仓文件运行，也可以接入 SPMS（研发项目管理）。接入后，SPMS 保存需求、用例和项目报告；开发计划目前以目标仓里的 Markdown 为事实源。SPMS 的字段权限与状态流转约束不定义本仓 SDLC 的人机分工，详见第 10 节。

## 2. 方法论

下列原则按当前 skill 合同说明；第 4 节是本文选定的开发动线，不代表每个 skill 都会自动启动下一步。

### 2.1 产物是合同，对话不是

`prd` 的产出有两个去处：SPMS 里的 `FR-N` / `NFR-N`，以及 `docs/PRD-<代号>.md`（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。计划、评审报告、完成记录、用例 key、项目报告也一样。新会话从这些产物恢复，不从聊天记录恢复。

### 2.2 写、审、改分开

| 动作 | skill | 默认不做什么 |
| --- | --- | --- |
| 写需求 | `prd` | 不替人决定 spec，不写实现方案和测试步骤（[skills/prd/SKILL.md](../skills/prd/SKILL.md)） |
| 写计划 | `dev-plan` | 实施、goal 执行、续做、回写进度时不加载本 skill（[skills/dev-plan/SKILL.md](../skills/dev-plan/SKILL.md)） |
| 审 | `review-prd`、`review-dev-plan`、`review-code`、`review-ui` | 不改原文，不实施，不写 SPMS |
| 改文档 | `apply-doc-review` | 不重跑评审，不修代码，不写 SPMS（[skills/apply-doc-review/SKILL.md](../skills/apply-doc-review/SKILL.md)） |
| 改代码 | `apply-code-review` | 报告是待核实主张，不是补丁指令 |
| 人确认取舍 | `spec-human-decision` | 不代替质量评审，不更新 SPMS，不实施 |
| UI 修复与补证 | `apply-ui-review` | 不自行评分或宣称验收通过 |
| 解计划阻塞 | `resolve-blocked` | 不实施，不重跑评审，不自动发送跨项目请求（[skills/resolve-blocked/SKILL.md](../skills/resolve-blocked/SKILL.md)） |

同一条回路用两次：需求走 `review-prd`，计划走 `review-dev-plan`，两次都由 `apply-doc-review` 按报告来源改对应原文（[skills/apply-doc-review/SKILL.md](../skills/apply-doc-review/SKILL.md)）。

文档评审无需修改时可直接交接，不空跑 apply。需要改稿时，`apply-doc-review` 核实意见、修订并复核；PRD 可交接后进入分期计划，计划处置后按真实前置判 `Ready` 或 `Blocked`。只有仍在等用户指定的其他非阻塞评审时保留 `Proposed`。需要独立复核或用户要求再评时，才再次调用对应 review；处置复核不能冒称独立评审通过。计划仍 `Blocked` 时由 `resolve-blocked` 解阻，见 7.3。

### 2.3 人给方向、确认 spec 和最终 UAT，agent 完成交付过程

| 参与方式 | 做什么 |
| --- | --- |
| 人提出 idea | 给出目标、问题、约束和希望产生的结果 |
| 人确认 spec | 确认功能、范围、验收及重要架构取舍；已有决定仍有效时不重复确认 |
| 人手动启动 skill | 选择当前环节，交给相应 skill；调用后由 agent 完成该环节的工作 |
| agent 推进交付 | 调查、成文、设计、编码、运行测试、审查、修复、采证、查差异与回写产物 |
| 人最终 UAT | 在 agent 提交交付结果与证据后，判断产品是否满足实际业务需要 |

开发验收默认由 agent 从真实入口执行。它可以按用户角色的知识、资料和权限约束操作真实环境，不需要为了开发退出另行招募真人。agent 的开发验收、代码/UI 评审通过，与人的最终 UAT 分别记录（[skills/dev-plan/SKILL.md](../skills/dev-plan/SKILL.md)「开发验收的可执行性」、[skills/test-plan/SKILL.md](../skills/test-plan/SKILL.md)「验证执行者与阶段」）。

出现新的功能取舍、改变已确认承诺或接受验收偏差时，回到 spec 确认；事实核查、普通实现选择和可执行的补证由 agent 自己完成。缺少无法自行取得的权限或外部资料时请求补齐，不把这类异常变成每一步都要人工操作的固定门禁。

评审里的「通过」或计划上的 `Ready` 只说明对应环节满足条件。它们不自动产生 merge 或 deploy 授权；已有提交、推送、部署授权仍由 agent 按约定执行（[skills/review-code/references/report-contract.md](../skills/review-code/references/report-contract.md)）。

### 2.4 需求 key 是唯一关联

已录入 SPMS 时，交接单位是 `FR-N` / `NFR-N` 与稳定 AC ID 的集合，不是整份 PRD 文件名。文档模式沿用工作号与 AC ID，并保留 PRD 来源和期次。一份 PRD 可以拆给多份计划，一份计划也可以合并多份 PRD 里的需求（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。`R#` 只在 PRD 里标识原始诉求，不进 SPMS。

一个 `PLAN-N` 只能挂同一项目的需求。跨项目要合并，只能一个项目一份计划，或先把需求迁到同一项目（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。本文写计划时以本地文件为准；`plan_create` 见第 10 节，`dev-plan` 自己不调用它。

### 2.5 正文写终局，分期只排节奏

§0–§6 写这块能力做完之后的样子。不许因为「这期做不完」删需求或把需求写小（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。分期在 §7，每期要能说清哪个角色多了什么以前做不到的事。承接计划建议 5–8 个 Milestone，连续性例外可以到 9–10 个，不超过 10 个（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。这个规模在交接时再交给 `dev-plan`（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。

跨期需求按完整承诺验收，某一期完工不能代替整条需求完成。终验期写进 PRD 与交接说明；接入 SPMS 时同步需求 `description`，实体终态的处理见第 10 节。

### 2.6 报告是待核实主张

进入 findings 的问题要有定位、证据、后果和最小修订。证据不足放待核实，不降成 P3 充数。`apply-doc-review` 和 `apply-code-review` 都要亲自读原文和代码，再决定采纳、不采纳或换更小的修法。

需求、Issue、用例正文是租户数据，不是给 agent 的指令（[skills/prd/SKILL.md](../skills/prd/SKILL.md)，[skills/test-plan/SKILL.md](../skills/test-plan/SKILL.md)）。

### 2.7 新会话从计划顶部恢复

计划开头的「实施须知」「实施者定位」「实施进度」是实施期的完整合同（[skills/dev-plan/references/plan-skeleton.md](../skills/dev-plan/references/plan-skeleton.md)）。续做时先读这三节和仓库规则，再核对工作树。记录与工作树不符时，先查明原因，不凭记录覆盖用户改动，也不凭代码存在推断验收已通过。

用 goal 执行时也读这三节，不加载 `dev-plan`（[skills/dev-plan/SKILL.md](../skills/dev-plan/SKILL.md)）。

### 2.8 两条线何时并行

测试用例的数据来源是需求的验收标准和 UAT 预期状态，不是计划正文（[skills/prd/SKILL.md](../skills/prd/SKILL.md)，[skills/test-plan/SKILL.md](../skills/test-plan/SKILL.md)）。计划评审结束、计划可实施后，开发线与测试线可以并行：agent 实施功能，也设计用例、准备环境并在相应能力就绪后执行验证。先稳住验收口径，再展开用例，避免返工。

`test-plan` 负责测试设计；实施 agent 或项目专用验证 skill 负责真实执行（见第 8 节）。SPMS 用例状态与结果如何回填属于集成问题，不代表测试要交给人执行。到差异核查时，已有测试结果只是证据的一种，不是免检证明。

## 3. 开工前

先核对以下条件。SPMS 项目和写权限仅在使用 SPMS 时需要；纯文档模式可以继续，不用相邻项目或相邻文档凑。

1. **目标仓**。skill 在运行时读这个仓的规则和代码，不把本仓的技术栈写进计划。
2. **agent 能看见的 skill**。按 [README.md](../README.md) 安装。本仓里直接读 `skills/<name>/SKILL.md` 也可以，但不要假设 `.agents/skills/` 是全量副本。
3. **SPMS 项目**。需求挂在项目上。MCP 没有 `project_create`，项目要人在 Web 建（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。令牌白名单里找不到对应项目就问，不塞进相近项目（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。
4. **写权限**。只出文档时可以不写 SPMS。要录入需求或用例时，工具必须可用，并且人已经确认可以写。`requirement_create` 不幂等，不查重就会造出重复需求。

用户只要文档、或 SPMS 工具不可用时，`prd` 用工作号 `WORK-F1` / `WORK-N1`，不虚构 `FR-N`（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。有落库需求且工具可用时，再走下面的写入步骤。

## 4. 从想法到一期收口

以下主路径展示 agent 如何交付，测试线在第 8 节展开。人手动启动每个所需的 skill，skill 内部的调查、执行和产物回写由 agent 完成；不用人工逐条落实报告意见。方括号表示可选或按条件进入；apply 只在有修订、修复或补证工作时执行。

```text
人提出 idea
→ prd
→ [spec-human-decision：人确认功能取舍]
→ review-prd → [apply-doc-review：修订至可交接]
→ dev-plan <PRD> P<n>：按 PRD 分期
→ [spec-human-decision：人确认架构决策与功能取舍]
→ review-dev-plan → [apply-doc-review]
→ [resolve-blocked：计划仍 Blocked 时解阻]
→ goal：coding agent 按 Ready 计划完成实施与验证（非 skill）
→ review-code ⇄ apply-code-review，修复后复审直至通过验收
→ [review-ui ⇄ apply-ui-review：有 UI 且选择 UI 验收时]
→ [review-code ⇄ apply-code-review：UI 循环改过代码时]
→ review-prd-dev-gaps
→ 有确定的回补工作：复用已有 gaps plan 或生成新计划，再走完整计划流程
→ 回补完成后再次 review-prd-dev-gaps，核实原缺口及受影响范围
→ agent 完成本期覆盖核查与交付汇总；继续 PRD 下一期
→ 全部交付范围完成后，由人最终 UAT
→ [agi-mode 复盘]
```

- `spec-human-decision` 可选的是确认表单这一步。`prd` 自身要求的共同理解确认，以及会改变范围、权限、验收的关键决定，不能因此跳过。
- 文档改稿不强制每次独立复审；需要独立复核时回到相应 review。代码和 UI 修复后必须独立复审，apply 自检不能作为通过依据。
- UI 达标后可选择不再打磨；未达标时，只有人明确接受整体界面并登记为「用户豁免通过」才可按豁免收口。单纯暂停仍保留未完成状态。
- UI 循环只要改过代码，收口前就复审这些改动，不以改动幅度大小判断。未改代码时可跳过这一轮；复审修复又影响 UI 时，复查受影响的 UI 范围。
- 各 skill 默认交出结果和下一步调用，由人手动启动下一环节。单个 skill 完成自身任务不需要人逐条指挥；已有更广的执行授权时按其范围推进。SPMS 状态流转另按第 10 节处理。

## 5. 需求阶段

### `prd`：把 idea 推演成可确认、可验收的产品定义

有新想法、需要补齐需求或修订 PRD 时启动。输入是目标、问题、约束和已有材料；接入 SPMS 时再给项目。agent 以产品设计者的角色推演功能、用户旅程、信息层级、交互反馈、视觉与 AI 行为，核实现状，提出真正影响 spec 的问题，再把确认结果写成 PRD。它负责形成方案和材料，由人确认产品取舍，不要求人代写需求。

产物是 `docs/PRD-<代号>.md`，包含完整 FR/NFR、场景、稳定 AC ID、UAT 预期状态、分期和终验关系；获授权时同步 SPMS 需求。退出时，原始诉求都有明确去向，验收可断言，阻塞决定已解决，才交给 `review-prd` 或按已确认安排进入计划。仅需要审已有 PRD 时直接用 `review-prd`，不用重新跑需求发现。

调用示例：

```text
使用 prd，把这个想法写成 PRD。项目是 <项目名>。先澄清会改变范围或验收的问题，确认后再写入 SPMS draft。
```

### 5.1 锚定项目

agent 先从目标仓与已有材料确定需求范围。使用 SPMS 时再执行下面的项目定位；纯文档模式按第 3 节继续，不需要补建项目。

| | |
| --- | --- |
| 进入 | 有想法，或有一份要改的 PRD |
| agent | `project_list` / `project_get`。读该项目 2–3 条已有需求，以及目标仓里已有的 CONTEXT.md、ADR。这些文件不存在就跳过，不因为写 PRD 去创建它们 |
| 人 | 项目对不上时指定项目。项目还没有就先在 Web 建 |
| 退出 | `projectId` 唯一确定。文档模式则明确标未核实 |
| 回路 | 项目不确定就停。key 一旦建错，排期和权限都会错 |

### 5.2 澄清到共同理解

agent 按 `prd` 的澄清规则提问，不问已经能从代码或 SPMS 查到的事实。会改变范围、语义或验收的问题未答完，不定稿，不写 SPMS（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。

非阻塞问题可以带默认、理由、负责人和解决节点留下来。解决节点写成「进入哪期计划前」，不为了凑日期编一个截止日期。

### 5.3 成文

正文写终局。每条需求要能回答：谁在什么条件下做什么，应到达什么状态。写不出可断言形式的，标开放问题，不用「体验流畅」「性能良好」占位。

不写表结构、端点、库选型和 Milestone 拆分。不写测试步骤。用户已经给定的平台约束写进「约束与前提」并标来源。

原始诉求每一条要么成为需求，要么被显式排除并写出去向。没有第三种下场。

### 5.4 确认后写入 draft

共同理解已确认、具体写入清单已获授权且查重完成后，agent 才 `requirement_create`，`status` 固定 `draft`（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。写之前用 `pms_search` 和 `requirement_list` 全量查重；命中已有需求时更新，不另建并行条目。

跨期需求要在 `description` 里写终验期，并在验收标准行上加 `[P1]` / `[P2]` 前缀。单期需求不加前缀（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。

写完把真实 `FR-N` / `NFR-N` 回填进 PRD。未录入的条目保持工作号，不编 key。

项目基本信息七段可以在共同理解确认后 `project_update`。这个调用是整段覆盖，写之前先 `project_get`，拿不准就把七段摊给人看（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。

SPMS 当前创建需求使用 `draft`，版本与附件等字段的接入限制见第 10 节；这些字段不决定 PRD 是否已经成文。排期和点数不在这一步做（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。

### 5.5 人逐项确认，可选

```text
$spec-human-decision docs/PRD-<代号>.md
```

需要把已有 PRD 的功能点逐项交给人确认、驳回或修改时使用。它生成交互式 HTML 表单，按实际提交修订指定文档，并在文档旁保存 `<文档名>.decisions.md`。选「修改」的条目及受影响条目需用修订稿再次确认；未答保持待决。已有明确且仍有效的答复不重复求批。

`spec-human-decision` 的输入是已有 PRD、CRD 或开发计划及本次确认范围。agent 抽取能够独立决定的能力和重要取舍，呈现价值、备选、代价与依赖；人选择后，agent 处理连带影响、修订正文、保存决定并复核一致性。开发计划先确认架构决策，再确认功能、里程碑与验证安排。

表单在本机 `127.0.0.1` 运行，需要 Node 18 及以上和浏览器；只把实际提交结果视为答复。未提交、关闭页面或超时不算确认。退出条件是范围内条目都有最终决定或明确待决原因，文档和决定记录对应；没有可确认条目就结束，不造空表单。这个 skill 准备并落实 spec 确认，不替代质量评审。

它不代替 `review-prd`，也不更新 SPMS。已经录入的需求受修订影响时，列出同步事项，按授权另行同步。确认某期不等于删除终局需求；计划状态也不能仅凭人确认改成 `Ready`。

### 5.6 评审和改稿

`review-*` 与 `apply-*` 设了 `disable-model-invocation`，必须显式发起：Claude Code 用 `/名称`，Codex 用 `$名称`。本文用这些写法表示显式调用，不把报告中的建议当作自动执行授权。

```text
/review-prd docs/PRD-<代号>.md
/apply-doc-review docs/PRD-<代号>.review.md docs/PRD-<代号>.md
```

#### `review-prd`：判断产品定义能否交给规划与测试

输入是既有 PRD、原始诉求和约束；复审时附上轮报告。agent 不只检查格式，而是核对代码现状与真实增量，审产品选择、用户旅程、体验、验收和分期。新功能尚未实现不算 PRD 缺陷，现有实现也不限制产品应有的目标。可用且获授权时由不同 reviewer 覆盖审查视角，主 agent 交叉核实后形成一份报告。

`review-prd` 默认把完整报告写到 PRD 同目录的 `<名>.review.md`（[skills/review-prd/references/report-contract.md](../skills/review-prd/references/report-contract.md)）。四个视角是现状、产品、体验、验收。报告先列问题，再给「通过 / 需修订 / 需补证」。

报告写入并回读后才完成这一环节。通过时进入分期计划；需要修订时手动启动 `apply-doc-review`；只缺外部证据且不需改正文时，补齐后复审。它默认不改 PRD、不实施、不写 SPMS。复审核对旧问题及直接受影响的回归，沿用问题 ID，保留历史。

#### `apply-doc-review`：由 agent 核实意见并完成改稿

输入是 `review-prd` 或 `review-dev-plan` 的报告。只给报告也可以，agent 从报告链接定位并核实原文；不能只凭报告文件名猜文档类型。调用即包含修订指定原文和记录处置，不需要核实后再问是否开始。

`apply-doc-review` 逐条核实。意见不成立就不改原文，只留处置。改稿会改变已确认目标、权限或验收时，先问人。SPMS 同步、发评论、修代码都记为另需处理，不借改稿算完成。

它交付修订后的 PRD 或计划，以及原报告中独立的「修订处置与复核」节。报告原问题、原结论和所审基线保留，正文只写当前产品或技术结论。再次 apply 先读已有处置，只处理未解决、缺验证或回归项；明确指定「仅复核」时不改正文。退出时逐项有依据和结果，未解决项不伪装成关闭。

退出：PRD 无阻塞、可交接。评审「无需修改」可直接进入分期计划；需要修订时，由 apply 核实并处置至可交接。仍需事实或决策时保留未决，补齐后续接 apply；需要独立复核时再运行 `review-prd`。不把 apply 处置改写成原报告已通过。

PRD 的交接依据是当前内容、spec 决定及评审/处置证据；SPMS 卡片状态与此分开，不作为纯文档模式的前置。

### 5.7 给人读的报告，可选

需要把需求、计划或其他材料解读成独立项目报告时，启动 `summarize`：

```text
使用 summarize FR-231 --project <项目>
```

链路是定位项目、取素材、调用 `elintp`、查重、`report_submit`（[skills/summarize/SKILL.md](../skills/summarize/SKILL.md)）。需求的 `kind` 是 `prd_interpretation`，计划是 `dev_plan_summary`（[skills/summarize/SKILL.md](../skills/summarize/SKILL.md)）。

`summarize` 接收实体 key、本地文档或主题，并把完整材料交给 `elintp` 解读。agent 完成读取、生成、查重和提交，产物是本地 HTML 与项目「报告」Tab 的 `RPT-N`，不是侧栏的团队统计总结。输入不是需求或计划时，`kind` 为 `other`；单份上限 2MB，超限不截断、不拆份绕过。

没给项目，或项目不在令牌白名单里，就停下来问。同题报告已存在时问人：新增，还是先删旧的。服务端没有幂等键（[skills/summarize/SKILL.md](../skills/summarize/SKILL.md)）。提交失败不要把 HTML 粘进对话当交付。

这一步不改 PRD，也不改计划。它可以和评审并行，但不是批准门禁。

#### `elintp`：把材料解释成一份能独立阅读的 HTML

需要本地解读时可直接启动 `elintp`；需要项目报告时由 `summarize` 调用。它接收主题或完整文档，先判断材料类型与重心，再围绕机制、操作、结果和限制组织图文，不照原目录逐段换词。产物是单文件离线 HTML，样式和脚本内联，图用可离线显示的内容；SPMS 报告不引用任何外部资源。

agent 必须保留原文警告、风险、数字与前提，并区分原文自述和本次验证。已完成计划引用的相关过程记录要逐份摘要，可读取截图要在相应位置嵌入。交付前检查内容、图示和最终页面，并在末尾标注原文与所读版本。它不修改素材；默认只保存本地文件，不发布。

## 6. 切期

agent 从已确认 PRD §7 提取本期 key 与 AC 集合，依据用户价值、前置和终验期出计划，不按「先做后端」切期。已有分期不要求人重新手选一次；改变已确认的范围或终验关系时再进入 spec 确认。

交给 `dev-plan` 的材料：

- 这一期的 `FR-N` / `NFR-N`（文档模式用工作号）及 AC ID 清单
- 每条的验收标准，跨期的要能看出终验期
- 明确排除项
- 一句终验声明，例如「本计划完成后 FR-38 仍停在交付段，终验在 P2」

一份计划可以少于 PRD 建议的 5–8 个 Milestone，只要这一期的用户价值闭合。9–10 个需说明连续性例外，超过 10 个必须反馈拆期。PRD 的 §7 不是技术任务表，Milestone 怎么拆由 `dev-plan` 判断（[skills/prd/SKILL.md](../skills/prd/SKILL.md)）。

## 7. 每期开发线

### 7.1 写计划

`dev-plan` 在产品范围明确、需要落地技术路线时启动。输入是 PRD 及期次，或已确认的直接需求；修订时再给既有计划与范围。agent 以架构师角色核查目标仓，明确本方开发对象与外部依赖，比较重要方案，把接口、数据、失败语义、迁移、验证与 Milestone 写到后续实现者能直接执行的程度。

```text
$dev-plan docs/PRD-<代号>.md P1，排除 <范围>。
```

agent 沿用上游需求 ID 建立追踪账本，对照目标仓代码核实现状，再写计划。未证实的主张标成假设。影响范围、安全、数据、外部契约或明确要求的 NFR 的问题未决时，状态必须是 `Blocked`，不能标 `Ready`（[skills/dev-plan/SKILL.md](../skills/dev-plan/SKILL.md)）。

落盘顺序：用户指定路径，否则目标仓根目录已有 goal 目录时写 `goal/<代号>.md`，没有则写 `docs/plan/<代号>.md`。文件已存在先问再覆盖（[skills/dev-plan/SKILL.md](../skills/dev-plan/SKILL.md)）。

计划里不出现 skill 名称、骨架路径或写作经过。实现者要的是当前结论（[skills/dev-plan/references/plan-skeleton.md](../skills/dev-plan/references/plan-skeleton.md)）。

输出的计划顶部包含需求来源、实施合同、恢复快照和进度索引；正文包含改动落点、依赖、连续实施批次、真实入口验证及退出条件。agent 在写作期完成适用的路径、结构和进度检查，自定义模板核对等价内容。验证默认由 agent 执行，开发收尾要从用户入口完成真实任务；不能只凭测试全绿或代码评审通过宣称能用。

它交付 `Ready / Proposed / Blocked` 的计划和下一步入口，默认接 `review-dev-plan`。直接实现、按计划续做或 goal 执行时不使用这个写作 skill，不重走规划。

`dev-plan` 不调用 `plan_create`，也不改需求状态。本开发线没有承接 SPMS `PLAN-N` 写入的步骤，见第 10 节。

### 7.2 人确认计划取舍，可选

```text
$spec-human-decision docs/plan/<代号>.md
```

逐项确认技术架构决策、备选及代价，再确认功能取舍、里程碑与验证安排。处理方式与 5.5 相同，决定写入计划旁的 `<文档名>.decisions.md`，与已有解阻记录分节共存。未纳入修订范围的 PRD 保持只读，涉及上游承诺的变化列出同步需求，不以改计划替代产品决定。

### 7.3 计划评审回路

#### `review-dev-plan`：判断计划是否贴合代码事实、路线是否站得住

输入是已有计划、需求来源和重要决定；复审附上轮报告。agent 核对计划中的复用、契约和依赖是否真实，架构与技术路线是否先进、能支撑后续演进，有没有绕开根因的补丁或为省事回避正确做法，实施者是否还被迫重做关键设计。也会指出没有真实风险依据、只增加实现和验证成本的防御或测试安排。

```text
/review-dev-plan docs/plan/<代号>.md
/apply-doc-review docs/plan/<代号>.review.md docs/plan/<代号>.md
```

`review-dev-plan` 重点看两件事：计划与代码事实是否吻合，架构与技术路线是否先进、可持续而非补丁凑合；范围越界、发布写进里程碑、需求无落点、交付断点等大问题由主 agent 顺带检查。建议状态沿用 `Ready` / `Proposed` / `Blocked`（[skills/review-dev-plan/references/report-contract.md](../skills/review-dev-plan/references/report-contract.md)）。`Ready` 不是实现完成，也不是上线许可。

报告默认在计划同目录：`docs/plan/<代号>.review.md`（[skills/review-dev-plan/references/report-contract.md](../skills/review-dev-plan/references/report-contract.md)）。

评审输出独立问题报告、两个重点与硬检查的覆盖、关键决策评估，默认不改计划和进度、不实施。报告建议状态与计划自报状态分开。需要改稿时交 `apply-doc-review`；无需改稿但仍有真实阻塞时交 `resolve-blocked`；无需修改且 Ready 时交 goal。缺关键前提证据不能因 findings 为空而判 Ready。

#### `resolve-blocked`：查证并解除真正阻止开工的事项

apply 之后计划仍是 `Blocked`，不要再跑一遍评审或 apply，改用：

```text
使用 resolve-blocked 解除 docs/plan/<代号>.md 的阻塞。
```

`resolve-blocked` 的输入是当前计划，可附报告、阻塞 ID 或外部答复。它以计划为事实源列出全部阻塞项，区分可查事实、spec 决策、本方验证/访问前置与外部依赖。agent 自行查证能查的事实、完成边界内可做的验证；需人决定的附上下文、推荐和代价，通过交互问答获取答复。外部项目需补能力、契约或交付时，准备或续写本地跨项目请求，不以一句“找负责人”代替交接。

问答、理由与各项结果存进 `docs/plan/<代号>.decisions.md`（[skills/resolve-blocked/references/decisions-contract.md](../skills/resolve-blocked/references/decisions-contract.md)），计划正文只写最终结论并重判状态。它不实施功能，也不自动发送跨项目请求。尚缺无法自行取得的访问条件、外部交付或实测证据时，写清缺口、执行入口、解除判据和下一步，计划保持 `Blocked`；其余独立事项继续处理。

退出：前置均已解决时计划变为 `Ready`，给出 goal 实施入口；否则交付完整的剩余阻塞清单和恢复条件。`Proposed` 只表示还在等非阻塞评审，不表示可实施。人手动启动实施即交给 agent 按 Ready 计划推进，不再另设一次“人工接受开工”。`Blocked` 时只做计划单列且不依赖阻塞的前期工作。

### 7.4 goal 按计划实施

goal 是 coding agent 的持续执行机制，不是本仓 skill。显式启动时把可执行计划作为输入，例如支持该命令的客户端使用：

```text
/goal 按 docs/plan/<代号>.md 开发，完成全部里程碑、约定验证与记录回写。
```

同一对话里写完计划接着做，也按计划顶部实施合同执行，不再读 `dev-plan`（[skills/dev-plan/SKILL.md](../skills/dev-plan/SKILL.md)）。

实施者做这些事：

1. 读仓库规则及计划的「实施须知」「实施者定位」「实施进度」，核对 `git status` 和相关 diff。
2. 按计划的前置和连续实施批次推进；批内运行必要的最小反馈，批末统一验证受影响场景，复用未失效的证据。
3. 按计划退出条件与提交点完成验证、提交和进度回写，证据写入 `<计划名>.records/M<n>.md`，全部完成后交代码评审。
4. 未运行、失败、环境缺失分开记。局部检查通过不等于整期验收通过。

独立任务可以交给 subagent，模型默认继承。共享文件一个人改。边界变了先回报，不悄悄扩大范围。

专项 skill（`architect`、`tdd`、`debugging` 等）只在这个 Milestone 的任务需要时使用。它们不改计划的退出条件。

### 7.5 代码评审和修复

#### `review-code`：审查真实变更，并独立判定开发验收

输入可以是开发计划、未提交改动、某次提交、提交区间或分支差异。agent 先冻结审查范围和版本，再沿变更追踪调用方、依赖及用户可见行为，核实可行动缺陷、需求符合性与验证证据；不按问题数量衡量质量，也不默认普查全仓。

本期计划实施完成后，对照计划及所审提交/工作树做代码验收；用户明确要求时也可审阶段增量：

```text
/review-code docs/plan/<代号>.md
/apply-code-review docs/plan/<代号>.code-review.md docs/plan/<代号>.md
```

`review-code` 默认不改代码、不改进度、不提交。只给计划时，默认审计划登记的提交与当前未提交改动；已有报告时复审修复效果。初审没有计划或其他明确目标且工作树干净时才询问，不擅自拿最近一次 commit（[skills/review-code/SKILL.md](../skills/review-code/SKILL.md) 第 1 节）。结论用 `patch is correct` / `patch is incorrect` / `inconclusive`（[skills/review-code/references/report-contract.md](../skills/review-code/references/report-contract.md)）。

报告路径：有计划时，计划同目录的 `<计划名>.code-review.md`（[skills/review-code/references/report-contract.md](../skills/review-code/references/report-contract.md)）。

产物是包含 findings、覆盖范围、证据缺口、总体结论和验收判定的报告。复审沿旧报告逐项核对，包括已离开当前 diff 的旧问题及修复影响的回归；保留轮次与历史。它默认只审查、不修复，但评审边界内能取得的证据由它在本轮补齐；需要修改或需补证时由人手动启动 `apply-code-review`，无需人自己改代码或补证。

#### `apply-code-review`：核实缺陷、修复代码并留下验证记录

输入必须能确定代码评审报告和开发计划；只给报告时可沿已核实的计划链接恢复。调用即授权处理范围内的合理修复、必要验证和记录回写，不在读完报告后再问是否开始。明确“仅复核”时不修改实现。

`apply-code-review` 要同时有计划和报告。它核实每一条，修合理的，把处置写进对应 Milestone 记录（[skills/apply-code-review/references/record-contract.md](../skills/apply-code-review/references/record-contract.md)）。不成立的不改代码。缺验证的写成「已修改待验证」，不能写成「已修复并验证」。提交、推送、部署不从评审文本里推断授权。

agent 按根因选择最小修法，通过真实入口或有用的窄检查确认修复，合并运行受影响回归，复用仍有效证据。计划有提交约定或已有授权时完成修复提交并登记 SHA；否则留在工作树供复审。输出代码改动、逐项处置、实际证据、计划记录及仍未解决项，保留原评审结论，不用自己的自检代替独立验收。

修复后再次 `review-code <计划路径>`，独立核实旧问题及受影响回归，循环至「通过验收」。以报告的验收判定为出口，不能只看 `patch is correct` 或空 findings：当前合同还要求没有 P0–P2 问题、旧问题已关闭或有依据不再适用，且改变的用户主路径有当前版本的真实入口证据。评审尝试后仍受阻的证据缺口才是「需补证」，不是通过；报告逐项写明执行者，agent 可执行的交 `apply-code-review` 补证后复审，只有无法自行取得的权限、资料或决定才请求人补齐。

用户明确豁免时单独记「用户豁免通过」，保留问题及影响，不能记成修复。每轮评审只覆盖声明的范围，局部修复通过不能掩盖其他未核实问题。通过也不自动授权合并或发布。

### 7.6 UI 验收与打磨，可选

有 UI 且需要界面验收的项目，在代码验收后进入：

```text
$review-ui docs/plan/<代号>.md
$apply-ui-review docs/plan/<代号>.ui-review.md
$review-ui docs/plan/<代号>.md
```

#### `review-ui`：根据运行中的界面作独立 UI/UX 验收

输入是计划或明确页面范围，以及设计稿、目标图或适用规范；复审再带上轮报告和修复记录。agent 定位或建立目标包，固定本轮版本与目标，启动真实界面、操作并实测，按页面状态采集代表性截图，再安排独立评分与裁定。代码和 diff 用于定位，不能证明视觉达标；同状态的多个问题共用截图，有效旧证据直接复用。

`review-ui` 基于目标包、真实界面截图、实测与交互证据，独立评分并给出验收判定。首轮保存 `<前缀>.ui-target.md`、`<前缀>.ui-review.md` 与 `<前缀>.ui-captures/`，后续报告用 `<前缀>.ui-review.r<n>.md`。这些文件与记录构成续接依据，不要求安装 Phoenix UI 或 Phoenix Diagram 工具。

每次调用交付一轮报告、评分、可行动差距和证据。它不改产品代码、计划进度或历史评审，修复方不能给自己评分。首轮新建目标包时列出预期来源与假设供人确认，属于 spec 确认；已有目标和决定仍有效时沿用。纯 UI 设计、交互选型和代码 diff 审查不属于这一环节。

| 判定 | 下一步 |
| --- | --- |
| 需要修改 | `apply-ui-review` 核实并修复差距，再 `review-ui` 独立复审 |
| 需补证 | apply 补齐可执行的环境、账号、数据或采集工作，再复审；只缺独立评分且材料齐备时直接续接 review |
| 需用户裁定 | 人决定接受整体、接受局部偏差/调整目标，或提供方向继续修复；未决保持未决 |
| 通过验收 / 用户豁免通过 | 给出最新截图，由人决定是否继续打磨；继续则 apply 后再 review |

#### `apply-ui-review`：由 agent 修界面、补证并解除可处理的阻塞

输入是 UI 报告及其目标包、截图和关联计划，可限定问题或给出已确认方向。调用覆盖修复、环境准备、补采证据、必要检查及按计划回写，不要求人手工照报告改页面。需补证时，agent 先恢复可用环境、测试账号与数据，运行缺失交互；只有无法自行取得的权限、素材或 spec 决定才请求人补齐。

agent 在运行中的界面核实差距，优先修共同根因，再按受影响状态自检；缺陷、证据不足与标准争议分别处理。产物是必要的代码或样式改动、修复/补证记录及可供复审的真实材料。仅缺证时完成采集即可，不为体现 apply 而改代码。已有有效证据不重复采集，不覆盖评审截图或原报告。

`apply-ui-review` 的修复最多记「已修改待评审」，自检不能代替验收。环境或证据阻塞不等于界面缺陷；没有新改动、新证据或阻塞解除时续接已有记录，不原地空跑新轮次。停滞时按报告请求裁定，不无限微调。

未达标而决定接受整体界面时，由 `review-ui` 追加用户豁免记录；只接受局部偏差或改变目标时，更新目标包并在下一轮重建基线。仅决定暂停不算豁免通过。

结束打磨后，只要本轮循环修复改过代码，就先 `review-code <计划路径>` 复审这些提交或未提交改动；有问题仍走 apply → review 的代码回路。代码修复若改变 UI 行为或外观，复核受影响的 UI 范围。没有代码改动则直接进入差异核查。

### 7.7 差异核查

`review-prd-dev-gaps` 用于开发完成后的覆盖核查。输入是已完成计划及其 PRD、记录与证据；只给计划时沿顶部需求来源定位 PRD 和期次。它回答本期要求实际交付了什么，而不是重新审 PRD 质量、计划设计或单次 diff。

本期计划的 Milestone 都有完成记录，或人明确要求对照已完成部分时：

```text
/review-prd-dev-gaps docs/plan/<代号>.md docs/PRD-<代号>.md
```

它从 PRD 往下追，不从「计划里已有的条目」往上猜。差异分三类（[skills/review-prd-dev-gaps/SKILL.md](../skills/review-prd-dev-gaps/SKILL.md)）：

| 类 | 含义 | 默认动作 |
| --- | --- | --- |
| A 明确遗漏 | 本期要求明确，证据说明没做到，补齐不需要新的产品取舍 | 进入 follow-up，不再问要不要修 |
| B 合理变更 | 有约束、替代和证据，目标或已批准的承诺还在 | 保留，并单列还要同步的文档或验收 |
| C 需人决定 | 范围、体验、权限、数据、外部契约或验收口径变了，现有决定不够 | 问人。没回答就保持待决 |

「实现更方便」或「工期不够」不能单独把一项定成 B。

有 C 时先出问题卡再问。默认选项、超时、空回答都不是批准。未决项不能写进计划当已选方案。

agent 建立完整覆盖账本，交叉核对 PRD、计划、各 Milestone 记录与真实行为。原计划尚未完成时明确前提不成立，继续核对可确认部分；证据不足先补查，不能一概归为遗漏。C 涉及新的 spec 取舍才交给人；已明确要求的 A 和有充分依据的 B 由 agent 判断并形成后续动作。

先核对已有 gaps plan 是否承接 follow-up，已有承接则沿用并链接真实进度；只有尚未承接、范围已确定的增量，且用户未限定「仅报告」时，才由本 skill 实际调用 `dev-plan` 生成独立 `<原计划名>-gaps` 计划。它不修改原 PRD、原计划或历史完成记录，也不开始执行 gaps plan。全部无 follow-up 时明确「无需 gaps plan」，不建空计划。

gaps plan 与普通计划走同一流程：可选的人确认（7.2）→ 计划评审/处置/解阻（7.3）→ goal 实施（7.4）→ 代码验收（7.5）→ 适用的 UI 回路（7.6）→ 再次差异核查。复审沿用上轮 G-ID、D-ID 和仍有效的决定，核对补齐结果与直接受影响的回归；其余范围标本轮未复核，避免反复创建相同计划或把局部通过写成整份 PRD 交付。

报告默认在原计划旁边：`<原计划名>.prd-dev-gaps.md`（[skills/review-prd-dev-gaps/references/report-contract.md](../skills/review-prd-dev-gaps/references/report-contract.md)）。

退出时交付覆盖账本、A/B/C、待核实与待决项，以及实际生成或沿用的 gaps plan 链接。无后续工作时说明无需计划；有未决或缺证时不能宣称 PRD 已完整交付。此 skill 不改原 PRD、原计划、历史记录或代码，也不开始实施回补计划；下一次由人手动启动相应环节。

### 7.8 agent 汇总交付，人做最终 UAT

本期差异核查没有未决 C、没有未核实的主路径缺口时，agent 汇总本期产物、实际验证、已确认限制和下一期入口。按已确认 PRD 继续后续期次不需要新增一轮人工评审；需要改变 spec 时再请人确认。

全部约定交付范围完成后，agent 提供可操作的产品入口、验收路径、证据与剩余限制，由人做最终 UAT，判断业务目标和使用效果是否满足。UAT 不通过时保留具体差距，按其性质返回需求确认、计划、实现或修复环节，不把人的接受写成问题已修复。

agent 的开发验收和独立评审是最终 UAT 的输入，不代替最终 UAT。有未决或缺证据时，不说整份 PRD 已经交付。SPMS 的「已上线」是另一个系统的状态字段，不能据此定义本期 SDLC 是否完成。

### 7.9 复盘，可选

```text
$agi-mode 复盘
```

回顾当前会话及已授权经验记录，提炼有证据的教训与改进建议。也可以指定本期计划、评审报告和修复记录作为材料。已有修改授权直接落实；只要求复盘不自动修改全局规则、发布 skill 或创建外部工单。复盘不替代尚未完成的开发或验收。

`agi-mode` 是通用的问题推进与持续改进入口。用于复盘时，输入是会话或指定材料，agent 从真实失败和纠正提炼机制假设，区分证据、建议与已获授权的修改；需要证明改进有效时再进入相应评测与迭代流程，不能把改了规则当作效果已证实。

在交付中也可用 `$agi-mode sdlc <问题>` 查流程，或用 `$agi-mode what-next <范围>` 核实当前状态和下一步。`what-next` 默认只读，`init-sdlc` 初始化状态台账，`what-next 更新` 保存变化。agent 依据原产物与证据登记状态，不从文件名推测已完成；普通查询不自动进入开发或长期执行。它帮助选择和推进工作，不替代各专项 skill 的产物合同，也不替代 goal 机制。

## 8. 每期测试线

### `test-plan`：把验收标准展开成可执行测试设计

计划评审结束、计划 `Ready` 后可以启动。输入是本期需求 key 或 PRD、验收范围与 UAT 预期状态；写入 SPMS 时从需求实体取得 projectId。agent 设计测试方法、数据、步骤与预期结果，不在这里发明产品行为：

```text
使用 test-plan，为 FR-18 和 NFR-3 写出测试用例草稿并写入 SPMS。
```

agent 做的事：

1. `requirement_get` 读取正文和验收标准，按行编号 AC1…ACn。
2. `testcase_list(requirementKey)` 翻到 `hasMore=false`。已有用例不覆盖、不删除、不改写。
3. 按正常路径、边界、权限、并发设计用例。推不出步骤的验收标准报出来，不编步骤。
4. `testcase_create`，带 `requirementKey`，`status='draft'`，不传 `result`。服务端默认 `untested`（[skills/test-plan/SKILL.md](../skills/test-plan/SKILL.md)）。

退出时给人一张映射表：每条验收标准指向新建的 `TC-N`、已有 key，或「无法成例」。无法成例的条目回到需求阶段改验收标准，不在用例里替产品做决定。

它交付 AC/UAT 到用例的映射、正常/边界/权限/并发覆盖、已有与新建用例清单、无法成例和未写入项。当前 skill 要求四类各至少一条，超过每需求 12 条的软上限时报告范围并请求确认，不能静默截断。服务端字面同名幂等不能代替语义查重，已有用例不覆盖、不删除、不改写。

`test-plan` 到测试设计与草稿交付为止，不在本次起草调用里执行测试。开发用例默认由后续实施 agent 或项目验证 skill 执行；真实页面、接口、账号和环境可用时直接操作。角色模拟要遵守该角色的知识、资料与权限限制，不能把结果外推为真人满意度或成功率。最终业务 UAT 由人完成。

### `create-e2e-verification-skill`：把真实用户旅程固化为可复跑能力

需要可复用的项目验证能力时，使用 [create-e2e-verification-skill](../skills/create-e2e-verification-skill/SKILL.md) 为用户选定的旅程创建、维护 `e2e-<area>`（未指定时由它提名候选），由它驱动真实入口、保存场景与运行证据。已有验证能力直接复用。agi-mode 将验收项映射到场景和入口，只执行所选范围及必要依赖，再核对完整覆盖；生成器的单场景冒烟不代表整期通过，也不自动回填 SPMS。

输入是要保护的用户旅程或产品承诺、目标仓与环境；已有 `e2e-*` 时先按覆盖路径和最后确认提交核对有效性，不重复生成。agent 发现真实启动和驱动方式、确定可观察判据，写出可单独选择的场景，生成或原位维护项目 skill。默认位置按用户指定、项目约定，再到 `.agents/skills/e2e-<area>/` 选择。

交付前，agent 按生成结果实际跑通，并在隔离副本临时破坏目标行为，证明同一入口能发现故障；当前已有缺陷时用隔离修复对照验证。产物包括 skill、运行证据、故障检测结果、清理结果及未覆盖范围。没跑过只算草稿，无法安全注入故障时明确未证明检测能力。它不替代一次性验收，也不因生成验证能力而获得修改产品代码的授权。

### 项目 `e2e-*`：由 agent 执行本次选定场景

生成后的项目 skill 接收场景范围、目标版本/环境、隔离条件与证据位置，只运行所选场景及必要依赖。agent 从真实用户入口操作，观察用户实际得到的结果，输出 `passed / failed / blocked / not-run`、实际观察和证据，清理本轮资源。环境受阻、跳过和未执行不算通过；一个入口或一个场景通过不代表整期覆盖。

测试结果若要作为差异核查的证据，把命令、环境和结论写进 Milestone 记录，或在运行 `/review-prd-dev-gaps` 时指出结果所在的位置。没有结果就如实标未运行，不把用例草稿当成已经测过。

## 9. 产物怎么交接

原产物继续按用户习惯和项目约定保存。`agi-mode what-next` 使用 `.xgent-ai/sdlc/` 仅保存按类型、对象拆分的状态页、索引和变化记录，通过链接引用下面的原文档与证据，不改变其落盘规则。

| 环节 | 产物 | 下一环节读取什么 |
| --- | --- | --- |
| `prd` | `docs/PRD-<代号>.md`；确认后的 `FR-N` / `NFR-N`（`draft`） | key、验收标准、终验期、排除项 |
| `review-prd` | `<PRD名>.review.md` | findings、待核实、建议结论 |
| `apply-doc-review` | 修订后的 PRD 或计划，加报告里的处置节 | 修订后的产品或技术结论，不是处置经过 |
| `spec-human-decision` | 修订后的 PRD/计划、`<文档名>.decisions.md` | 最终取舍、仍待决项及关联同步需求 |
| `summarize` | 项目报告 `RPT-N`，本地 HTML 在 `docs/` | 不作为下游合同 |
| `elintp` | 离线 HTML 解读 | 原文出处与版本、解释、限制；不作为开发合同 |
| `dev-plan` | `goal/<代号>.md` 或 `docs/plan/<代号>.md` | 实施须知、实施者定位、实施进度、Milestone |
| `review-dev-plan` | `<计划名>.review.md` | 建议状态和阻塞范围 |
| `resolve-blocked` | `<计划名>.decisions.md`，加修订后的计划 | 修订后的计划结论；剩余阻塞的解除条件 |
| goal（coding agent 机制） | 代码，以及 `<计划名>.records/M<n>.md` | 完成记录和验证基线 |
| `review-code` | `<计划名>.code-review.md` | 问题 ID、所审基线 |
| `apply-code-review` | 修复和记录里的处置表 | 当前行为是否仍满足计划 |
| `review-ui` | `<前缀>.ui-target.md`、`<前缀>.ui-review[.r<n>].md`、采集证据 | 目标、判定、差距和有效证据 |
| `apply-ui-review` | UI 修复与补证、计划记录中的 UI 验收修复节 | 已修改待评审项、剩余阻塞与影响范围 |
| `agi-mode` | 按模式产出的调查/复盘结论，或显式初始化、更新的状态台账 | 原产物、证据和下一步；查询默认只读 |
| `review-prd-dev-gaps` | `<计划名>.prd-dev-gaps.md`；有未承接的确定 follow-up 时另有 gaps plan | G-ID、D-ID、确认过的增量范围 |
| `test-plan` | 测试设计、AC/UAT 映射；接入 SPMS 时有 `TC-N` 草稿 | 步骤、数据、预期结果与覆盖缺口，供后续 agent 执行 |
| `create-e2e-verification-skill` | 项目 `e2e-<area>` skill、运行与故障检测证据 | 选定场景、真实入口、覆盖路径与最后确认提交 |
| 项目 `e2e-*` / 实施 agent | 实际测试结果、环境/版本与证据 | `passed / failed / blocked / not-run`，以及未覆盖范围 |
| 人最终 UAT | 对业务结果的验收结论及具体差距 | 接受结果，或回到相应环节继续处理 |

同名报告默认不覆盖。PRD/计划评审按各自报告合同另存带日期的文件；附计划的代码评审与 UI 评审按轮次 `.r<n>.md` 另存，保留历史。指定路径冲突也保留旧文件。不要把报告写进计划正文，也不要写进 `plans.content`。

## 10. SPMS 可选集成：字段、状态与衔接限制

本节仅在落库 SPMS 时适用。SDLC 的计划就绪、实施进度、独立评审和最终 UAT 以对应产物与证据为依据；不能把 SPMS 卡片的 `draft`、`approved` 或「已上线」当成本仓全流程的分工规则。纯文档模式跳过这些系统操作。

| 集成点 | 当前约束与衔接 |
| --- | --- |
| 需求写入 | `prd` 新建时使用 `status=draft`；其 SPMS 合同仍规定 `reviewing / approved` 由人流转，禁止 agent 写 `shipped / rejected`，平台错误为 `FINAL_STATE_FORBIDDEN`。这是需求实体写面的限制，不代表 agent 只做草稿或不能完成开发验收。 |
| 版本与附件 | 当前 `prd` 合同列为 Web 补录；跨期版本挂终验期。它们属于项目管理字段，不新增一轮 SDLC spec 确认。 |
| 计划实体 | `prd` 的交接指向 `plan_create` → `PLAN-N` 和 `plan_update({content})`；`dev-plan` 本身只交付本地 Markdown。本开发线没有统一承接实体写入，不能宣称本地计划已自动同步。 |
| 用例草稿 | `test-plan` 创建 `draft`，不传 `result`，服务端默认 `untested`；起草调用不激活用例、不执行或回填。这是该 skill 的职责边界，后续执行者可以是 agent。 |
| 测试结果同步 | 实施 agent、项目 `e2e-*` 与本地完成记录可以提供真实测试证据。SPMS 用例激活和 `result` 回填需由获授权、工具允许的后续流程承接；当前这套技能没有统一绑定该写回步骤，未写回须说明，不能因此把测试执行整体交给人。 |
| 跨期需求终态 | SPMS 的「已上线」表示需求级验收。首期 Issue 全部完成可能使按钮亮起，仍要按终验期处理；单期计划通过不能提前结束跨期需求。 |

字段和写面依据：[prd](../skills/prd/SKILL.md)、[SPMS 映射](../skills/prd/references/spms-mapping.md)、[test-plan](../skills/test-plan/SKILL.md)。本说明不修改这些 skill 的 SPMS 合同，也不把其限制扩展成 SDLC 的人工工作清单。

## 11. 按需使用的专项 skill

这些能力在对应问题出现时启动，不增加固定的人工审批环节，也不要求每个 Milestone 都跑一遍。

### `story-points`：用固定因子估点，按要求写回

在要估算需求/工单或核对 Sprint 容量时启动。输入包含实体 key、验收标准、技术影响说明及代码基线；影响说明来自本次读取的计划或明确给出的材料，不能从需求正文猜技术方案。非代码交付物按其专用因子处理，不强求开发计划。

agent 沿领域、技术栈、复杂程度和验收复杂度四轴判定，输出 `points / matchedFactors / unknown / evidence`。同一输入应得到逐字段一致的结果；缺技术影响说明时不估点，未知因子如实标出，不用历史点数改变本条判定。

默认只估算；明确要求写回时才更新 `plannedPoints` 并回读核对容量。已有点数默认跳过，写回带读取的 `expectedPoints` 防止覆盖并发修改。超容量时报告差额，不擅自移项或改点数。这是可选的 SPMS 排期辅助，不是需求或计划进入下一阶段的必经门（[skills/story-points/SKILL.md](../skills/story-points/SKILL.md)）。

### `architect`：为复杂实现形成可编码的结构草图

复杂设计需要先明确类型、签名和模块边界时使用；已有可执行计划不因含架构内容就重新规划。输入是具体设计问题、相关系统与约束。agent 追踪既有行为与设计依据，比较结构上不同的候选，再形成调用示例、类型/接口草图、模块图和选择理由。

调用包含实施范围时，agent 按选定草图完成实现；只有显式要求 checkpoint 才停下展示草图求确认。草图持续容纳不了真实约束时重新调查和设计，不叠加变通。它解决当前复杂实现问题，不替代 PRD、整期开发计划或代码验收（[skills/architect/SKILL.md](../skills/architect/SKILL.md)）。

### `debugging`：先证明故障原因，再修复

遇到 bug、测试失败、构建或集成异常时使用。输入是症状、复现路径、错误与相关环境。agent 先读完整错误、复现并检查近期变化，沿组件边界收集证据，再与可工作的路径比较，提出能被最小实验区分的根因假设。

假设得到证据支持后才实施修复并验证。产物是根因、复现与修后证据、必要代码改动，以及未解决的环境或外部限制；不能靠不断试补丁代替调查。它可以服务于 goal 或 apply 的当前任务，不改变原计划退出条件（[skills/debugging/SKILL.md](../skills/debugging/SKILL.md)）。

### `tdd`：在成本合适时把 bug 变成回归检查

用户要求 TDD/回归测试，或 bug 有清晰、便宜的测试路径时使用。agent 先写最小检查，确认它因目标问题失败，再修改实现并确认同一检查通过。产物是针对实际缺陷的回归检查、最小修复及红绿证据。

需要大量新 harness、脆弱 mock、昂贵基础设施或仍不清楚复现步骤时，不为形式强加测试；改用最接近真实问题的可执行验证，并说明限制。测试断言期望行为，不能为了通过改成错误实现的现状。它是局部验证方法，不是每个函数必须配测试的覆盖率要求（[skills/tdd/SKILL.md](../skills/tdd/SKILL.md)）。

目标环境要能定位本文使用的 skill；本地安装可能滞后，按第 3 节核对源文件与安装。是否另增 `review-test-plan`、是否把估点纳入固定排期流程，仍属编排选择；当前不把它们增设为必经环节，也不影响 agent 执行开发测试。

## 依据文件

| 文件 | 本文用它核对的内容 |
| --- | --- |
| [README.md](../README.md) | 安装命令与本文入口 |
| [package.json](../package.json) | 版本 `0.7.12` |
| [skills/prd/SKILL.md](../skills/prd/SKILL.md) | 产品推演、内容边界、终验、多对多交接与 SPMS 写面 |
| [skills/review-prd/SKILL.md](../skills/review-prd/SKILL.md) | 评审通过不等于可上线 |
| [skills/review-prd/references/report-contract.md](../skills/review-prd/references/report-contract.md) | PRD 评审报告路径 |
| [skills/apply-doc-review/SKILL.md](../skills/apply-doc-review/SKILL.md) | 按报告来源改 PRD 或计划 |
| [skills/summarize/SKILL.md](../skills/summarize/SKILL.md) | 项目报告链路、查重、kind |
| [skills/elintp/SKILL.md](../skills/elintp/SKILL.md) | 文档解读、离线 HTML、转述与过程材料要求 |
| [skills/dev-plan/SKILL.md](../skills/dev-plan/SKILL.md) | 实施时不加载写作 skill |
| [skills/dev-plan/references/plan-skeleton.md](../skills/dev-plan/references/plan-skeleton.md) | 实施合同在计划顶部 |
| [skills/review-dev-plan/references/report-contract.md](../skills/review-dev-plan/references/report-contract.md) | Ready / Proposed / Blocked |
| [skills/resolve-blocked/SKILL.md](../skills/resolve-blocked/SKILL.md) | 解除计划阻塞的边界、决策记录与回改计划 |
| [skills/review-code/SKILL.md](../skills/review-code/SKILL.md)、[skills/review-code/references/report-contract.md](../skills/review-code/references/report-contract.md) | 计划提交范围、复审、验收判定及轮次命名 |
| [skills/apply-code-review/references/record-contract.md](../skills/apply-code-review/references/record-contract.md) | 修复处置写入 Milestone 记录 |
| [skills/review-prd-dev-gaps/SKILL.md](../skills/review-prd-dev-gaps/SKILL.md) | A/B/C 与 gaps plan |
| [skills/test-plan/SKILL.md](../skills/test-plan/SKILL.md) | 测试设计、用例草稿、agent 执行与 SPMS 写回边界 |
| [skills/create-e2e-verification-skill/SKILL.md](../skills/create-e2e-verification-skill/SKILL.md) | 项目验证 skill 的生成、维护、实跑与故障检测 |
| [skills/story-points/SKILL.md](../skills/story-points/SKILL.md) | 估点输入、因子、只估算/写回与容量边界 |
| [skills/architect/SKILL.md](../skills/architect/SKILL.md)、[skills/debugging/SKILL.md](../skills/debugging/SKILL.md)、[skills/tdd/SKILL.md](../skills/tdd/SKILL.md) | 复杂设计、根因调查与有用的回归验证 |
| [skills/spec-human-decision/SKILL.md](../skills/spec-human-decision/SKILL.md) | 两次可选确认、决定记录、文档修订与边界 |
| [skills/review-ui/SKILL.md](../skills/review-ui/SKILL.md)、[skills/review-ui/references/report-contract.md](../skills/review-ui/references/report-contract.md) | UI 判定、豁免、补证与代码复审出口 |
| [skills/apply-ui-review/SKILL.md](../skills/apply-ui-review/SKILL.md) | 修复/补证、自检、记录与独立复审 |
| [skills/agi-mode/SKILL.md](../skills/agi-mode/SKILL.md)、[skills/agi-mode/playbooks/reflection.md](../skills/agi-mode/playbooks/reflection.md) | 可选复盘入口及授权范围 |
