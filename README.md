# XGENT Skills

XGENT 的 [Agent Skills](https://skills.sh) 集合,可安装到 Claude Code、Cursor、Codex 等支持 skill 的 Agent 中。

## 安装

列出并选择安装公开的 skills (`xgent-init` 是内部初始化 skill,不在默认列表中):

```bash
npx skills add XGENT-ai/skills
```

只安装指定 skill:

```bash
npx skills add XGENT-ai/skills --skill dev-plan
```

## 项目辅助工具（Claude Code / Codex）

本仓库同时以 [`@xgent-ai/skills`](https://www.npmjs.com/package/@xgent-ai/skills) 发布到 npm,自带 `xgent-skills` 命令,可为任意项目安装 XGENT 的 Claude Code hooks(当前包含 statusline)、在项目 `.claude/settings.json` 中启用对应配置,装上随包的 [Phoenix UI](#phoenix-ui) 设计工具与 UI 验收 skills,并可选创建通用的 `AGENTS.md`、安装 Codex 或 Claude Code 的 goal 上下文收尾提醒。当前开发入口还默认接入独立 [Phoenix Diagram 候选](docs/phoenix-diagram.md),其最终包与四平台交接尚在验收:

```bash
# 在目标项目根目录执行(也可显式传目录:npx @xgent-ai/skills install <dir>)
npx @xgent-ai/skills install
```

0.7.3 补齐了 npx 默认入口。旧版 0.7.2 会报 `could not determine executable to run`,使用该版本时请显式指定安装器:

```bash
npx --package=@xgent-ai/skills@0.7.2 xgent-skills install
```

选项:

| 选项 | 说明 |
| --- | --- |
| `--no-phoenix-ui` | 跳过 Phoenix UI,仍安装 XGENT 的 hooks 与 settings;不删除已装版本 |
| `--no-impeccable` | 已弃用,等同 `--no-phoenix-ui`,保留一个迁移主版本 |
| `--replace-impeccable` | 删除所选工具目录中的项目级及个人级 impeccable 入口和 hooks,不备份 |
| `--keep-impeccable` | 保留 impeccable,与 Phoenix UI 共存;无交互环境默认采用 |
| `--no-phoenix-diagram` | 跳过独立 Phoenix Diagram 候选,不删除已装版本 |
| `--migrate-archify` | 仅迁移所选项目/provider 的已知原版 Archify;展示目标后确认删除,跳过 XGENT/UI 阶段,须已有匹配引擎 |
| `--confirm-remove-archify` | 与迁移配用的显式确认;旧 skill 不备份,回滚不重建,非交互迁移必须提供 |
| `--release-dir=DIR` | 离线安装 Phoenix UI engine:使用已核验的 release 目录,不联网 |
| `--diagram-release-dir=DIR` | 只为 Phoenix Diagram 使用匹配的完整离线目录 |
| `--xgent-init` | 直接创建仓根 `AGENTS.md` (已有则跳过),不再询问 |
| `--no-xgent-init` | 跳过创建 `AGENTS.md`,不再询问 |
| `--providers=a,b` | 指定两个 Phoenix 工具的 harness 目录(如 `--providers=.claude,.cursor`);`codex` 识别 `.agents/.codex`,diagram skill 只写 `.agents`;默认按已有目录选择,空项目用 `.claude` |

安装是幂等的:hook 文件按内容比对,只在有变化时覆盖;`settings.json` 按顶层 key 合并。全部只读预检通过后才按 XGENT、Phoenix UI、Phoenix Diagram 顺序写项目,两个 Phoenix 工具分别由 Rust 在各自项目锁内重读并事务化提交。后段失败保留前段结果,按各自日志重跑恢复。Diagram 候选的独立安装与验收范围见[包内说明](docs/phoenix-diagram.md)。

普通安装不移除 Archify。`--migrate-archify` 使用独立的 Diagram 迁移分支,仅处理精确项目/provider 的完整已知来源;自定义、链接、外部管理与个人入口保留。TTY 拒绝或 EOF 不迁移;`--no-phoenix-diagram` 仍跳过并保留。旧入口移除后需要时按原来源重装,新 Diagram 入口的恢复/回滚见[迁移说明](docs/phoenix-diagram.md#旧入口迁移与安装恢复)。仓库只保留新活跃入口,不代表其他项目或个人旧安装已被处理。

`install` 会询问是否同时创建仓根 `AGENTS.md`,输入 `y` 或 `yes` 确认,回车默认跳过。确认后直接复制 [通用模板](skills/xgent-init/references/external-app-AGENTS.template.md),只创建 `AGENTS.md`,不安装 `xgent-init` skill,不生成 `CLAUDE.md` / `PRODUCT.md` / `DESIGN.md`。已有 `AGENTS.md` 时直接跳过并保留原文。

非交互环境默认跳过,可用 `--xgent-init` 显式创建,或用 `--no-xgent-init` 跳过询问。文件位于仓根,不受 `--providers` 影响。模板没有待填占位,通用编码准则支持不同 coding agent;Portal App 的 onebox 联调与有前端改动时的真实浏览器验收分别按适用条件执行。

Portal 的产品和设计规范仍保留在 [PRODUCT 模板](skills/xgent-init/references/external-app-PRODUCT.template.md) 与 [DESIGN 模板](skills/xgent-init/references/external-app-DESIGN.template.md) 中。需要生成项目上下文时,显式安装并在 Agent 对话中使用 [xgent-init](skills/xgent-init/SKILL.md),它读取清单与代码事实,补齐 PRODUCT.md / DESIGN.md (无前端的 service 型不新生成 DESIGN.md),已有文件保留:

```bash
npx skills add XGENT-ai/skills --skill xgent-init
```

## Phoenix UI

Phoenix UI 是 XGENT 自主维护的设计工具:从 [impeccable](https://github.com/pbakaus/impeccable) 4.5.0 / engine 0.1.11 的固定源码承接(Apache-2.0,来源与修改见 `dist/phoenix-ui/NOTICE.md`),版本、规则与发布由本仓控制,不再查询或下载上游 latest。它只随 npm 安装器分发:`install` 装入 `phoenix-ui` skill、对应 hooks,以及 design-md、design-reference、ui-pattern-research、review-ui、apply-ui-review 五个辅助 skill。对话里用 `/phoenix-ui`(Codex 用 `$phoenix-ui`);终端命令为 `npm exec --package=@xgent-ai/skills -- phoenix-ui --help`。

Phoenix UI 适用于独立网站、应用和嵌入式界面等 UI 项目。`phoenix-ui init` 可独立完成产品上下文初始化,不依赖 `xgent-init`。subagent 与并行策略由 coding agent 自主安排,Phoenix 不额外要求询问或等待授权。

- **engine**:按包内 `dist/phoenix-ui/VERSION.json` 的摘要校验后放进 `~/.phoenix-ui/bin/<版本>/<平台>/<engineSha256>/`(`PHOENIX_UI_HOME` 可改),新旧项目的引擎可共存,本期支持 darwin-arm64、darwin-x64、linux-x64、linux-arm64,Windows 延期。不可用时整个安装在写项目前停止,可改用 `--release-dir=DIR` 离线安装或 `--no-phoenix-ui`。hook 与 context 运行时不联网补下载。
- **已有工具**:安装时发现 impeccable 会询问是否替换。选择替换则直接删除所选 provider 的项目级及个人级旧入口和旧 hooks,不备份、不留转跳入口;选择保留则共存,不阻止 Phoenix 安装,无交互环境默认保留。没有安装事务状态的 Phoenix 源码副本直接删除后重装。业务源码、旧工具的数据和文档不改动;共享配置中的其他 hooks 保留。
- **回滚**:安装输出给出事务 ID、备份位置与完整命令,例如 `npm exec --package=@xgent-ai/skills@<版本> -- phoenix-ui migrate --rollback <事务ID> --project <项目>`;回滚只撤销本次安装的条目,后来新增的配置保留。

安装、迁移、回滚、支持矩阵与渠道说明见 [docs/phoenix-ui.md](docs/phoenix-ui.md)。原 `vendor/impeccable/` 副本及其 vendor/R2 脚本已删除,需要时从 git 历史查看。

managed UI 循环先核对固定包 engine，再用 `phoenix-ui review --help` 和 `phoenix-ui review schema --json` 发现 verbs 与输入形状；后者是 typed examples，样例不是真实证据。具体调用与恢复边界见 [Phoenix 评审协议](skills/review-ui/references/phoenix-review.md)。

## Skills 列表

以下按用途分组列出 `skills/` 中的 skill。点击名称查看使用条件与完整流程。

`review-*`、`apply-*`，以及 `agi-mode`、`arena`、`reflect` 等入口设了 `disable-model-invocation`，只能由你显式调用：Claude Code 用 `/名称`，Codex 用 `$名称`；模型不会自行发起。其余 skill 仍按 `description` 自动匹配。

### 需求、计划与评审

| Skill | 说明 |
| --- | --- |
| [prd](skills/prd/SKILL.md) | 撰写或优化 PRD，明确产品目标、FR/NFR、UAT、分期与终验，可按授权录入 SPMS |
| [crd](skills/crd/SKILL.md) | 固定由提出方使用：起草或修订需求、整合接收方回复、答复反问并维护需求与 Q&A 当前视图 |
| [crd-respond](skills/crd-respond/SKILL.md) | 固定由接收方评估、答复 CRD 或回告实现结果；需改码时生成带来源 CR 与完成回写约定的计划，不实施。仅原样追加/收录不重评或生成计划 |
| [dev-plan](skills/dev-plan/SKILL.md) | 基于需求、仓库规则与代码事实撰写或评审可执行计划，明确开发对象、排除与外部前置；CR 来源可追溯，对应请求完成后回写答复，支持跨对话续做 |
| [spec-human-decision](skills/spec-human-decision/SKILL.md) | 逐项讲清 PRD 或开发计划的功能与价值，计划另述做法；交互选择确认、驳回、修改并打磨文档，修改须附说明 |
| [test-plan](skills/test-plan/SKILL.md) | 从需求与验收标准生成测试用例，覆盖正常、边界、权限和并发场景，可按授权写入 SPMS |
| [story-points](skills/story-points/SKILL.md) | 按客观因子估算故事点，回写 SPMS 规划点数并复核 Sprint 容量 |
| [review-prd](skills/review-prd/SKILL.md) | 核实代码现状，评审产品价值、体验、AI、SMAR、UAT 与分期终验，默认将完整报告落盘并回读核验 |
| [review-dev-plan](skills/review-dev-plan/SKILL.md) | 评审开发计划的代码事实、技术方案、需求映射、依赖编排与交付可用性，输出评审报告 |
| [review-code](skills/review-code/SKILL.md) | 评审未提交变更、commit 或分支差异，可对照开发计划，输出有证据的代码评审报告 |
| [review-prd-dev-gaps](skills/review-prd-dev-gaps/SKILL.md) | 核对 PRD 与开发交付，将遗漏、合理变更和待拍板差异分流，交互 Triage 后按需调用 dev-plan 生成 gaps plan |
| [apply-code-review](skills/apply-code-review/SKILL.md) | 核实代码评审意见，实施合理修复、验证结果并回写实现记录与必要进度 |
| [review-ui](skills/review-ui/SKILL.md) | 代码评审后可选的 UI/UX 验收：只认运行中界面的实拍截图与实测，不凭代码推断效果；由全新上下文的独立评审按冻结的目标包打分并列出可执行差距，默认每个单元 ≥ 8/10 且没有未关闭的 P0/P1 才通过 |
| [apply-ui-review](skills/apply-ui-review/SKILL.md) | 按 UI 验收报告对照截图与目标核实并修复界面差距、自检；停滞预警时重构思路，提交登记后交回复审 |
| [apply-doc-review](skills/apply-doc-review/SKILL.md) | 可只传评审报告，沿报告的原文档引用定位并修订 PRD 或开发计划，复核并记录逐条处置结果 |
| [resolve-blocked](skills/resolve-blocked/SKILL.md) | 查证与交互推进计划解阻，外部依赖通过 CRD 交接，存 decisions 并回改计划；最终集中列全剩余阻塞、各方交付和用户具体动作 |

把上述 skill 串成完整交付流水线（想法 → PRD → 分期 → 每期开发与并行测试 → 交付）的方法见 [AI Native SDLC](docs/ai-native-sdlc.md)，含各环节产物路径约定与测试线待补环节的现状核对。

**手动闭环**：各 SDLC skill 结束时先给明确判定，再给可直接复制的下一条命令（Codex 写作 `$<skill>`，Claude Code 写作 `/<skill>`），由你决定是否执行。`P1`、`P2` 表示期次（Phase）。

需要逐项确定功能取舍时，可在 PRD 或开发计划成稿后使用 `spec-human-decision <文档>`：每项简述功能与价值，开发计划另述做法；选择确认、驳回或附说明修改，修订版再确认，并同步文档与决定记录。这是可选的用户决策环节，技术评审与开工条件仍按下述回路核对。

1. （可选）`prd` → `review-prd <PRD>`：需要修改时 `apply-doc-review <报告>`；可交接后 `dev-plan <PRD> P1`。临时或小任务可直接从 `dev-plan` 开始。
2. `dev-plan` → `review-dev-plan <计划>`：需要修改时 `apply-doc-review <报告>`；之后 `Blocked` 用 `resolve-blocked <计划>`，自行查证、交互补齐前置，收到答复后继续处理，外部缺口实际用 `crd` 创建/续写本地请求，不自动发送；最终开头列全剩余阻塞、各方需提供什么、用户具体动作和解除条件，不只给重跑命令。先推进具体 CR 或本方验证，解除条件满足后再运行以重判就绪，`Ready` 用 `/goal 按 <计划> 开发`。从 PRD 派生的计划在顶部「需求来源」记录 PRD 与期次，开发范围区分本方改动与外部前置，不因端到端目标代建依赖系统。
3. `/goal` 开发：按计划的提交点分阶段提交，全部里程碑完成后最终提交，SHA 记入完成记录，结束时提示 `review-code <计划>`。来自 CR 的计划按顶部合同，在对应请求或明确子范围实现且约定检查通过后追加实现结果答复，保留历史、余项及回告链接；不等同发布或对方验收，未能回写时保留恢复动作。
4. `review-code <计划>` 从完成记录登记的提交与未提交改动确定范围：需要修改时 `apply-code-review <报告>`（按计划约定提交修复并登记 SHA），再回到 `review-code <计划>` 复审，直到通过验收或你明确本期放过。
5. （可选）计划涉及界面时，`review-ui <计划>` 做 UI/UX 验收：每轮按目标包截图，由全新上下文的独立评审打分。需要修改时 `apply-ui-review <报告>`（按计划约定提交并登记 `UI修复` SHA），再回到 `review-ui <计划>` 复审，直到通过（默认每个单元 ≥ 8/10 且没有未关闭的 P0/P1）、停滞后交你裁定，或你明确放过。UI 修复带来代码改动时，再用 `review-code <计划>` 复审这些改动。

   UI 循环可沿用人工 Markdown，或通过匹配的 Phoenix engine 管理证据与轮次。已有 managed 索引时从原轮与 revision 续接；engine 缺失、版本不匹配或状态损坏时保持只读，恢复固定版本后再继续。人工历史只经明确的 `review import` 接管，不能清零未知次数；CLI 操作成功不代表 UI 通过，真实取证和独立评审仍是判定依据。
6. 通过后，计划来自 PRD 时可选 `review-prd-dev-gaps <计划>`；生成 gaps plan 时从第 2 步的 `review-dev-plan <gaps 计划>` 继续。

**跨项目入口按角色固定**：提出方始终用 `crd` 起草/修订需求、整合接收方返回稿、答复反问；接收方始终用 `crd-respond <CRD>` 评估/答复需求、续接提出方澄清或回告实现结果。双方收到文件或追加消息都不切换角色，归属不明先核实。接收方默认在原文末追加有证据的评估答复，需改本方代码时实际调用 `dev-plan`（未决契约按就绪门标 `Blocked`），计划记录来源 CR、承接请求/子范围、回写文件与顶部完成回写约定，生成计划从第 2 步继续；完成回告核对适用证据后追加 RESP，不重开规划或改写历史。仅原样追加/收录不重新评估或生成计划，仅评估不落盘，仅答复不生成计划，不自动发送或实施。

在任意阶段通过 `$agi-mode sdlc <问题>` 或自然语言获取 [SDLC 指引](skills/agi-mode/playbooks/sdlc.md)：解释 PRD、CRD、计划与测试的分工，判断交接条件，选择下一步。手册随 agi-mode 分发；咨询默认只读，涉及项目现状时复用 what-next 核对证据，实际能力以当前项目和已安装技能为准。

这 15 个 SDLC skill 及 `agi-mode` 均在 `SKILL.md` 的 `argument-hint` 中提供参数提示。`<…>` 表示任务所需输入，可沿已有上下文或文档引用定位；`[…]` 是可选补充，`/` 分隔同一位置的备选，不是要求照抄的参数或新增命令解析器。例如：

```text
$crd-respond <接收方收到的 CRD 路径/链接/内容> [本方项目/仓库] [需求ID/范围] [仅评估/仅答复] [计划输出路径]
$review-prd <PRD路径/链接/内容> [上轮报告] [复审/完整评审] [问题ID/范围] [报告路径] [仅聊天]
$review-code [开发计划] [未提交/commit/A..B/基准分支] [上轮报告] [复审/完整评审] [问题ID/文件范围] [报告路径] [仅聊天]
$apply-doc-review <文档评审报告> [原文路径] [上轮报告/处置记录] [修复/仅复核] [问题ID/范围]
$spec-human-decision <PRD/Dev-plan路径/链接/内容> [功能ID/期次/范围] [输出路径] [仅讨论]
$review-ui [开发计划] [页面URL/路由] [设计稿/目标图] [上轮报告] [复审/重建基线] [单元/范围] [门槛分] [报告路径]
$agi-mode <任务描述> 或 sdlc [流程/阶段/问题] 或 init-sdlc [PRD/计划/范围] 或 what-next [查询/更新] [PRD/计划/范围]
```

提示的展示取决于客户端：[Claude Code](https://code.claude.com/docs/en/skills#frontmatter-reference) 在 `/skill` 补全中支持 `argument-hint`；Codex 的 `$skill` 参数预览仍有[未关闭的功能请求](https://github.com/openai/codex/issues/31014)，不能仅凭新增字段保证弹出提示。未展示时可直接参考上述用法或各技能的 frontmatter。

5 个 `review-*` 支持“对照上轮报告复审修复效果”：逐项保留来源 ID，区分已解决、仍存在、部分解决、回归、待核实、不再适用和本轮未复核；新增问题单列，默认另存报告。代码旧问题的复核不受本次修复 diff 限制。`review-prd-dev-gaps` 会先核对已有 gaps plan，支持“仅报告”，避免重复规划。

`apply-doc-review` 与 `apply-code-review` 支持“继续修复”与“仅复核”：前者处理剩余问题，后者保留目标产物并保存必要处置记录；完全只读则仅聊天交付。重复执行先核对已有证据，无实质变化不改文件或时间；apply 的复核不自动成为新一轮独立 review。

项目状态与下一步能力已合并到 `agi-mode` 的 [what-next 流程](skills/agi-mode/playbooks/what-next.md)，不再单独发布 `what-next` skill。查询默认只读；初始化/更新才保存，既有执行授权继续有效。

```text
使用 $agi-mode sdlc，解释从需求、CRD 往返到开发、验收和发布的交接。
使用 $agi-mode init-sdlc 初始化项目状态跟进。
使用 $agi-mode what-next 看当前项目状态和下一步。
使用 $agi-mode what-next 更新状态，核对并行计划和最近几轮评审。
```

`init-sdlc` 按需创建状态目录和索引，核实并登记现有材料；可重复运行，沿用已有台账并补齐缺项，不重置历史。兼容原有 `what-next 初始化` 写法。新台账使用协议 v4，日常只更新对象页中本次触及的 `facet + scope` 分段，不为每次进展或来源版本变化新增事件文件。初始化和状态查询的聊天交付按“业务主题 / 文档类型 / 原文文件路径”列出工作项，用通俗语言说明进展、具体阻塞及下一步的完成条件，并保留状态与证据码；写入的索引只留身份与入口，状态和下一步保存在对象分段。其他 SDLC skill 在台账启用后按仓内协议维护相关状态。

[状态目录约定](skills/agi-mode/references/workspace-layout.md)使用仓根 `.xgent-ai/sdlc/` **只保存开发状态**。PRD、开发计划、评审报告、实施/验收记录和证据继续按用户习惯存放，台账通过链接引用，不改变其路径或复制正文。

```text
.xgent-ai/
  index.md                          工作状态总入口
  sdlc/
    index.md                        本层导航与工作项快捷链接
    protocol.md                     状态回写和索引维护协议
    state-model.md                  固定状态、证据门槛和自愈规则
    status/
      index.md                      类型入口
      <prd|plan|review>/
        index.md                    对象目录
        <对象UUID>.md               各 PRD、计划、评审的状态
```

只有项目明确要求不可变审计，或并发/纠错事实无法由原产物及可取回版本保留时，才按协议新增 `events/<类型>/<对象UUID>/<事件UUID>.md`；事件模板和目录索引在首次需要时创建。旧事件保留原路径，状态页只链接历史入口和已核对末端，不抄全量 ID；未解冲突、待决和授权依据必须保留。

开发过程直接复用原计划的恢复快照/完成记录、里程碑实现与验收记录、评审及处置章节；台账保存当前判断和来源链接。里程碑推进、最终提交和整份计划完成均不另生成 event 总结。新建事件前先核对原记录，写明审计要求出处或无法由来源及当前页保留的具体事实，已有过程只引用。

每个实际创建的状态目录都有 `index.md`，非开发任务的 `tasks/` 及任务子目录也一样；不预建空目录。索引只列本层身份与入口，数量增多后可按领域/年份分片。已有对象只改正文时无需改索引，工作项快捷链接仅随对象选择或入口变化维护，不复制进度、证据、阻塞、下一步或事件计数。写入者按[索引维护规范](skills/agi-mode/assets/sdlc/protocol.md#目录索引维护)同步受影响入口，多机合并保留双方事实并核对来源，不能按机器时间覆盖；查询只报告缺项。

状态按对象拆分、页内按维度分段、历史按需读取。各段独立保存核查时间、来源版本、基线与适用范围，只更新本次触及的段落；页头只留共同身份，不写跨维度摘要。PRD 与计划保持多对多关系，每轮评审有独立状态 ID，原报告文件名不必改变；同轮评审结论与处置在同页分段，不再分别建事件。计划的里程碑完成数只在实施维度按原计划保留，事件总数与历史覆盖条数不手工维护。

[固定状态模型](skills/agi-mode/assets/sdlc/state-model.md) v3 保留既有状态与证据门槛：`dev-plan-ready` 表示可开工，`dev-plan-completed` 表示实施范围和规定验证已完成，保存粒度由仓内协议决定。what-next 会从 `.review.md`、`.codereview.md`、`.code-review.md` 等候选补查漏记评审，并从报告处置节、`.applied.md` 或实施记录核实整改。状态取原文记录支持的最强状态，证据质量单独标注：只有自述为 `provisional`，之后相关文件又有改动为 `stale`，依据矛盾为 `conflict`；只有没有可读依据才用 `unknown`。原计划状态滞后时按记录纠正并保留原文自述。查询只读重算，更新才保存修复；独立验收线枚举暂为 TBD。

相关撰写、评审、修复技能和计划实施模板遵循仓内回写协议，原产物落盘方式保持原有约定。已有台账不自动迁移或双写；明确要求精简/升级时可切换 v4，保留项目原则、授权、审计要求和旧证据，不批量删除历史。状态协议随仓同步，另一台机器无需安装所有技能也能回写；状态已同步不代表原文和证据已同步。安装 `agi-mode`：`npx skills add XGENT-ai/skills --skill agi-mode`。

### 开发、调试与质量

| Skill | 说明 |
| --- | --- |
| [architect](skills/architect/SKILL.md) | 实现前先设计类型、签名与模块结构，并在开发过程中持续校准 |
| [debugging](skills/debugging/SKILL.md) | 按四阶段流程调查根因、定位问题并验证修复 |
| [tdd](skills/tdd/SKILL.md) | 先用失败测试复现缺陷再改实现，适用于有明确廉价测试路径的 TDD 与回归测试 |
| [create-verification-skill](skills/create-verification-skill/SKILL.md) | 按应用或场景创建、维护 verify-*，复用现有测试并验证真实入口与交接；不依赖编辑器，可供 agi-mode 复用 |
| [mcp-release-qa](skills/mcp-release-qa/SKILL.md) | 发布前验证 MCP server：用真实协议会话对照源码与文档核能力，测失败路径并留下可复现证据 |
| [clean-artifacts](skills/clean-artifacts/SKILL.md) | 清理评审、测试日志和截图等过程文件，处理误入 Git 的文件并补齐忽略规则；保留必要记录，按单独授权执行历史瘦身 |
| [react](skills/react/SKILL.md) | 编写和评审 React 组件，处理 Hooks、Effects、渲染性能、数据请求与 React 19 迁移 |
| [rust](skills/rust/SKILL.md) | 编写、评审、重构、测试和调试 Rust 代码，按所有权、API、错误、性能、Tokio、unsafe/FFI、Cargo、宏、行为模式及 Bevy 等场景提供指引 |
| [sql-optimization](skills/sql-optimization/SKILL.md) | 分析 SQL 执行计划，优化查询、索引、分页与批量操作 |
| [security-audit](skills/security-audit/SKILL.md) | 沿数据流与组件交互审查注入、认证授权、密钥泄露、依赖及业务逻辑漏洞 |
| [gdpr-compliant](skills/gdpr-compliant/SKILL.md) | 将 GDPR 隐私工程要求应用到个人数据处理、权限、日志、保留与删除流程 |
| [working-with-mbx](skills/working-with-mbx/SKILL.md) | 安装和排查 Cargo 共享编译缓存 mbx，核对命中、绕过与磁盘使用情况 |
| [working-with-mise](skills/working-with-mise/SKILL.md) | 配置和排查 mise 管理的工具、配置文件、PATH 与激活问题 |

### Agent 协作与实验

| Skill | 说明 |
| --- | --- |
| [agi-mode](skills/agi-mode/SKILL.md) | 通用问题助手，不知道如何推进时从这里开始；核心是自进化 loop，SDLC 与项目状态为按需能力 |
| [optimize-agents-md](skills/optimize-agents-md/SKILL.md) | 精简所在代码仓库 AGENTS.md；用 use-prompt-dsl 形式化原则并补齐缺项，保留项目事实与作用域，缺 DSL skill 时停止并建议完整安装 |
| [arena](skills/arena/SKILL.md) | agi-mode 的多候选比较兼容入口，选择基础版本并重新验证合成结果 |
| [swarm](skills/swarm/SKILL.md) | 为任务选择多 agent 协作机制，设计角色、分工、执行与收尾流程 |
| [figure-it-out](skills/figure-it-out/SKILL.md) | 任务没有匹配的既有 playbook 时设计可审计的工作流：阶段、实验与决策记录 |
| [automate-me](skills/automate-me/SKILL.md) | 把个人偏好和工作习惯固化成专属 `-mode` skill，按引导生成并试跑 |
| [autoresearch](skills/autoresearch/SKILL.md) | agi-mode 的 Hillclimb 兼容入口，在固定协议下搜索并确认改进 |
| [use-prompt-dsl](skills/use-prompt-dsl/SKILL.md) | 编写、改写、解释或按明确要求执行 Odyssey（ODC）Prompt DSL；用目标、本体、上下文与规则表达要求，LOOP 明确重复工作，片段可省略 PROMPT，普通开发不自动套用 |
| [reflect](skills/reflect/SKILL.md) | agi-mode 的 Reflection 兼容入口，从会话证据提炼并验证改进假设 |
| [experience-capture](skills/experience-capture/SKILL.md) | 开发中轻量收集踩坑、纠正及自动化线索，供用户随后发起复盘 |

`agi-mode` 的自进化 loop：真实任务反馈 → 改进假设 → 评测与搜索 → 最终确认 → 采用与沉淀 → 后续反馈。[Reflection](skills/agi-mode/playbooks/reflection.md) 提炼假设，[Eval](skills/agi-mode/playbooks/eval.md) 建立可信判定，[Hillclimb](skills/agi-mode/playbooks/hillclimb.md) 管理搜索，[Arena](skills/agi-mode/playbooks/arena.md) 按需提供不同候选。协作与记录约定见 [技能演化](skills/agi-mode/references/skill-evolution.md)。

只需安装 `agi-mode` 即可使用上述流程；保留的 `autoresearch`、`arena`、`reflect` 名称是兼容入口，需要同时安装 `agi-mode`。它们不再独立维护执行规则。

明确要求优化仓库指令时，可使用 `$optimize-agents-md [repo 路径] [文件范围] [仅诊断]`，默认处理当前代码仓库的根 AGENTS.md。它默认可发现，普通编码、初始化或全局规则修改不进入该流程；仅诊断保持只读。优化时使用已安装的 `use-prompt-dsl` 将原则压缩为 Odyssey RULE 片段（嵌入 AGENTS.md 时省略 PROMPT），按语义去重并补齐 Think Before Coding 与八项 Coding Principles，保留项目事实、授权和作用域。缺少 DSL skill 时停止，建议完整安装本仓 skills；安装检查和优化流程只属于 skill，不写进目标 AGENTS.md。无需同仓评测任务或远端配置，文件精简不等于已证明行为质量提升。

独立的 agi-mode [远端执行工具](skills/agi-mode/references/remote-evolution.md) 提供 `validate / run / resume / apply`，优化器、被测 agent 与固定验收使用 SSH 主机上的独立环境。默认搜索 keep 只更新探索最佳，独立确认和原则核对通过后才在已有写回授权内按 CAS 采用；显式获准的 v2 `bounded_exploration_v1` 仅允许质量门通过、来源精度不足时探索一个候选，不晋级最佳、不确认/采用，整体 inconclusive。不自动提交、发布或安装全局配置。首期v1已在指定SSH/Docker组合完成真实隔离、任务对照/搜索、自然请求/CAS与故障恢复验收，但独立确认样本不足，未证明候选优于基线。v2现按已有结果阶段交付：工程和已测子范围可用，完整skill三条件及TypeSafe反馈进入真实optimizer尚未验收，没有已证实的指令收益；资源到位后再按3分目标续跑，详见[M4阶段记录](docs/plan/agent-instruction-evolution-v2.records/M4.md#2026-10-22-阶段交付)。

自然发现验收可用 `source.fixed_skill_roots` 显式提供完整固定背景目录，目标缺席/候选切换保持其他技能字节；实际目录、读取与任务行为分别记录，单独的目标技能执行不代表完整目录发现已通过。

v2运行质量项可显式声明`verifier/`公开观测文件，按白名单接收固定grader的实际运行事实；验收分与运行证据分开，缺证据保持unknown，独立观测不替代新增回归测试。
新百分表保留其他已知分项，不因单项unknown全表清空；已知硬失败控制可显式声明不可观察的非gate项并要求双评一致未知，成功控制、底线、排序及辅助资格保持严格。
新百分rubric还可显式让程序从已校验逐项判断派生评审状态，避免把产物失败误作测量无效；旧协议默认保持，新格式需重新校准。
固定预算项可显式要求双评数值及校准固定参考完全相同，分歧保持unknown及原分；缺省程度容差保持，同分仍须核对依据，详见远端手册。

v2自有仓任务可显式使用`case.platform=linux/arm64`和匹配的固定镜像；默认及v1/Pro V2仍amd64。平台改变后重新校准，不能迁移旧分或资格；详见[远端手册](skills/agi-mode/references/remote-evolution.md)。

安装、有限运行与恢复入口见 [使用说明](docs/agent-instruction-evolution.md)。

`experience-capture` 可独立安装，默认允许自动选择；本仓 `AGENTS.md` 也提供触发约定。它只在开发的自然间隙，把当前上下文已有的线索用 3–4 行保存到 `.xgent-ai/experience/`，不打断主任务，不额外检索取证或回读完整记录；没有合适间隙则跳过，纯咨询和只读任务不落盘。自动选择不等于可靠的后台事件监听；其他项目可在自己的 `AGENTS.md` 中约定出现这些信号时读取已安装的 skill。任务结束后用 `$agi-mode 复盘` 或 `$agi-mode reflect` 深入挖掘、查找和核对证据；默认给出建议并回写已审条目的处置，明确授权沉淀时落实到 know-how、playbook、principle 或自动化工具。

整体方法见 [agi-mode 自进化方法论](docs/agi-mode-evolution-methodology.md)，说明样本沉淀、重放、进化实践和能力评估，以及通用方法与业务实践案例的边界。

### 文档、表达与图示

| Skill | 说明 |
| --- | --- |
| [documentation-writer](skills/documentation-writer/SKILL.md) | 基于代码事实编写和更新 README、API、架构、代码库说明与用户手册 |
| [phoenix-diagram](skills/phoenix-diagram/SKILL.md) | 十类 Rust 图解、Mermaid 语义转换、仓库来源与五语言离线阅读；明确请求时提供品牌资源、本地预览、架构比较；实际图像审核与[候选安装/迁移边界](docs/phoenix-diagram.md) |
| [elintp](skills/elintp/SKILL.md) | 将技术主题或文档改写成面向非研发读者、带图示的通俗 HTML 说明 |
| [summarize](skills/summarize/SKILL.md) | 将需求、计划、Issue、文档或主题整理为 HTML 报告，并提交到指定 SPMS 项目的报告区 |
| [humanize-writing](skills/humanize-writing/SKILL.md) | 改写机械、夸大或模板化的文字，使表达自然、直接 |
| [meeting-minutes](skills/meeting-minutes/SKILL.md) | 整理会议议题、决定、负责人、截止日期与后续行动，生成会议纪要 |
| [pardon](skills/pardon/SKILL.md) | 补充必要上下文，用当前会话语言把上一条回答重新讲清楚 |
| [how](skills/how/SKILL.md) | 用代码证据讲清模块如何工作、分层与归属；设计理由用 `why` |
| [why](skills/why/SKILL.md) | 追查设计取舍、回归或事故背后的原因；运行时行为用 `how` |
| [teach](skills/teach/SKILL.md) | 由一个具体改动或子系统切入，讲清是什么、怎么运转、为什么这样设计 |

### 设计与转化

| Skill | 说明 |
| --- | --- |
| [ui-loop](skills/ui-loop/SKILL.md) | 按获奖级视觉标准设计或打磨页面，逐项自检、对标、修补并循环复验，直到所有检查维度无可见改进 |
| [design-reference](skills/design-reference/SKILL.md) | 寻找视觉参考并形成有来源的 brief；可读 Phoenix context，已接受方向按授权保存原文和路径/hash 映射供恢复 |
| [design-md](skills/design-md/SKILL.md) | 提炼和维护 DESIGN.md，保留规范并报告偏差；可读 Phoenix 原文/解析覆盖，写入仍按用户决定与证据 |
| [ui-pattern-research](skills/ui-pattern-research/SKILL.md) | 比较交互方案并交付状态与验收条件；可读 Phoenix 产品/已接受方向，业务能力仍核实实际接口 |
| [kpi-dashboard-design](skills/kpi-dashboard-design/SKILL.md) | 设计 KPI 仪表盘的指标选择、信息层级与数据可视化 |
| [landing-page-conversion-audit](skills/landing-page-conversion-audit/SKILL.md) | 审查落地页、销售页或结账流程的转化障碍，按预期收入影响排序修复建议 |

### XGENT Portal 接入与发布

| Skill | 说明 |
| --- | --- |
| [xgent-init](skills/xgent-init/SKILL.md) | 内部初始化 skill,补齐 `AGENTS.md` / `PRODUCT.md` / `DESIGN.md`,保留 Portal 产品边界与设计规范;`install` 仅直接创建 AGENTS.md |
| [portal-dev-setup](skills/portal-dev-setup/SKILL.md) | 在 App 仓库中启动、体检和排查 Docker 一盒门户，完成本地联调 |
| [portal-external-app](skills/portal-external-app/SKILL.md) | 接入以 Docker 镜像交付的外部服务，核对 manifest、注册、路由、权限与交付契约 |
| [portal-micro-app](skills/portal-micro-app/SKILL.md) | 开发嵌入门户的微应用前端，处理 SDK 握手、导航、授权与 iframe 交互 |
| [portal-app-exchange](skills/portal-app-exchange/SKILL.md) | 配置和排查跨应用 OAuth Token Exchange、交换白名单、授权与 scope |
| [portal-logging](skills/portal-logging/SKILL.md) | 将应用日志与遥测接入门户，配置采集器、写入身份、日志流和字段口径 |
| [xgent-image-push](skills/xgent-image-push/SKILL.md) | 构建并推送 App 镜像到私有 Harbor，预检链路、镜像架构、tag 与保留策略 |
| [xgent-app-release](skills/xgent-app-release/SKILL.md) | 使用发布令牌提交前端产物、镜像和 manifest 变更，处理发布提案与发布故障 |
| [use-llm-gateway](skills/use-llm-gateway/SKILL.md) | 为 App 接入 LLM Gateway 统一 `/v1` 调用面，迁移旧调用并排查凭证、幂等与媒体任务 |
| [use-task-gateway](skills/use-task-gateway/SKILL.md) | App 接入 Task Gateway 跑异步任务，处理鉴权、幂等、进度与结果回收 |
| [use-workflow-service](skills/use-workflow-service/SKILL.md) | 判断 workflow 与 Agent + Skill 的分工，并按 Portal 接入与编排 |

## 目录结构

每个 skill 位于 `skills/<name>/` 目录下,包含一个带 `name` 与 `description` frontmatter 的 `SKILL.md`:

`description` 用于加载前的技能选择,按「触发条件与关键排除条件 → 核心产出」组织。整条建议不超过 250 字符,中文不超过 200 字符并尽量在 100 字符左右;前 100 字符应能独立表达适用边界。这是本仓库的编辑目标,不是所有 coding agent 的统一截断上限。

`when_to_use` 补充 `description` 没覆盖的同义说法、触发短语或示例请求,不重复正文条件;客户端把它拼在描述后参与选择,不读取该字段的客户端仍按 `description` 触发。

容易混淆的限制要紧跟触发条件,例如「仅写/评审计划,不用于按计划实施」,不能放在长段落末尾或只留在正文。操作步骤、完整错误码清单、工具细节和重复的双语说明放入正文或 references;精简后检查前缀是否仍表达原意,不要直接机械截字。

```
skills/
└── dev-plan/
    ├── SKILL.md          # skill 主体(必需)
    ├── references/       # 补充参考文档
    ├── scripts/          # 辅助脚本
    └── evals/            # 评测用例
```

## License

[MIT](LICENSE)
