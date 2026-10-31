# PHOENIX-DIAGRAM-P3 · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-DIAGRAM-P3](PHOENIX-DIAGRAM-P3.md) | SHA256 `faa0f3683e7178e6bc24633d193e45d3aad2269d225700c0142bf48d52840ca0` | 全文、M1–M8、16 条 P3 AC 与五条跨期需求终验链 |

评审日期：2026-10-28（Asia/Shanghai）。本轮为首次完整评审；原计划及产品代码保持只读。相关 PRD 是背景和需求依据，不是本报告的修订对象。

## 1. 确认问题

### [P2] F-01 · 冻结比较成果的审核与完整交付合同

- **定位**：[计划 §3.3、§5.2](PHOENIX-DIAGRAM-P3.md)，344–346、385–386 行；关联 WORK-F9、WORK-N4，以及继承的默认视觉审核要求；M2、V-2、V-4、compare-valid。
- **问题**：计划已明确比较使用独立 artifactKind、专用检查分派及 Before/Delta/After 审核范围，但公开操作链仍缺一段：`compare` 只提交 HTML/CompareReceipt，后续 `visual-check → 实际看三个视图 → review-record` 由哪个命令建立审核 summary、如何定位、如何绑定双输入及三视图、以什么状态宣布完整交付，均未冻结。若让成对提交的 compare JSON 兼任可变 summary，会改变原比较回执的身份；若另存 summary，则还需决定创建者、路径和更新规则。这是持久化及调用方协议，不能只靠“精确分派”交给实施者重新设计。
- **证据**：
  - [review.rs](../../tools/phoenix-diagram/src/delivery/review.rs) 73–82 行要求已有 summary，并按 `FinalizeReceipt` 读取；265–313 行进一步核对 `command=finalize`、单一 candidate 和四个固定自动阶段的真实回执。
  - [receipts.rs](../../tools/phoenix-diagram/src/delivery/receipts.rs) 216–235 行的 `VisualReviewRecord` 拒绝未知字段，固定为单 candidate/config 身份；当前格式不能直接承载新比较合同。
  - [cli.rs](../../tools/phoenix-diagram/src/cli.rs) 461–466 行的 `visual-check` 只分派浏览器检查；[delivery-contract.md](../../skills/phoenix-diagram/references/delivery-contract.md) 28–41、87–95 行说明单独检查不替代 summary，最终须读取同交付的完成状态。
  - [PRD §5.3 C3](../PRD-PHOENIX-DIAGRAM.md) 392 行要求新命令保持默认实际审核及真实结论含义。
- **最小修订**：在 §3.3/§5.2 定义比较专用 summary/evidence/review 的创建入口、路径获取、字段和完成判据，并同步 delivery-contract。可保留 HTML/原 CompareReceipt 为不可变确定性锚点，由比较分支的浏览器检查创建独立 summary，`review-record` 仅更新该 summary；记录绑定 base/head、配置/资源/引擎、IR、HTML、原 compare 回执以及三个视图的截图身份。给出可直接执行的公开命令顺序及缺证、失败、失效语义；不必增加新的 finalize 命令。
- **复核**：改稿后仅凭公开说明即可定位审核材料、创建记录并取得完整交付状态；V-2 验证不重生成原比较成果即可登记审核，缺任一输入/视图、混入旧审核或身份变化均拒绝；V-4 实际查看三个视图后完成同一交付收口。现有单图命令及其完成含义保持有效。
- **置信度**：高；主 agent 已阅读上述原始代码并与 architecture、delivery 交叉核对。**阻塞范围**：M2 的公开审核协议实现；不阻塞已独立确定的 M1。

### [P2] F-02 · 将阶段 A 所需验收入口和真实资源安排到 M4 之前

- **定位**：[计划 §4、V-4、§9.2、M4/M7](PHOENIX-DIAGRAM-P3.md)，355、464、483–487、533、536 行。
- **问题**：M4/V-4 必须运行新增 `p3-acceptance.mjs --stage author-tools`，并通过同 run 的 review-file/resume 完成真实品牌、三视图、预览及故障收口；但 M7 才整批交付 P3 验收入口。真实采集所用的唯一明确 HTTPS 测试资源还被指定为 M7 固定提交中的新 PNG。按既定 M4→M5→M6→M7 顺序，阶段 A 退出所需工具和来源尚未交付，实施者必须先改排计划才能继续。
- **证据**：计划 464 行已调用该新增脚本；483 行定义其新增维护合同；355、487 行定义固定提交的真实 HTTPS 资源；536 行把验收入口/profile/题面/CI 整批安排到 M7，533 行则要求 M4 先通过 V-4。路径检查也确认该脚本和 PNG 是新增产物，而非可直接复用的现有入口。适用的[依赖编排合同](../../skills/dev-plan/references/execution-workflow.md)要求实际前置成果在使用前就绪。
- **最小修订**：将 author-tools 及 M4 所需审核/恢复/故障支撑明确分配给 M4 退出前的交付，可随 M1–M3 稳定接口准备；同时给出 M4 前可用、已授权且可取回的固定 HTTPS 资源和提交/内容身份。M7 只补最终 candidate/providers/profile/CI 接线。若资源必须先提交，明确对应准备提交与 M4 验收的先后关系，不用验收通过后的提交反向提供其前置。
- **复核**：沿 M1→M4 走查，V-4 每个新增命令和外部资源均能追到执行前的交付及就绪证据，不再依赖 M7；M7 继续负责最终候选扩展。无需新增里程碑或提前执行阶段 B 终验。
- **置信度**：高；四个视角独立识别后，主 agent 复读冲突段落确认。**阻塞范围**：M4 退出，不阻塞 M1–M3 的独立实现。

### [P2] F-03 · 补齐删除旧入口后的 legacy 消费样本分发合同

- **定位**：[计划 §1.2、§5.5、§9.2](PHOENIX-DIAGRAM-P3.md)，296–298、408、483–485 行；M6/M7、V-7/V-8/V-11，关联 WORK-F10/AC-1、WORK-N3 的双合同兼容与跨期终验。
- **问题**：M6 删除整个 `skills/archify` 后，最终候选仍须真实消费五类 legacy 输入。当前消费器直接从旧活跃目录取得这五份样本；计划虽把 candidate 脚本列入改动面，但未分配样本搬迁交付，也未规定新包内位置、原字节身份与历史候选布局分派。只保留仓内 oracle 或增加 P3 profile 不会补齐包内输入，最终候选会在进入消费检查前失败；保留旧活跃目录或删掉旧合同消费又会违背本期验收。
- **证据**：
  - [check-phoenix-diagram-candidate.mjs](../../scripts/check-phoenix-diagram-candidate.mjs) 30、119 行强制检查旧目录内五份指定 JSON；201 行 `caseManifest()` 再从该目录构造 legacy 消费路径。
  - [package.json](../../package.json) 28–47 行的发布允许清单包含 `skills`/指定 dist，不包含 `tools/upstream`；[pack-phoenix-diagram-candidate.mjs](../../scripts/pack-phoenix-diagram-candidate.mjs) 123–136 行还明确禁止把 tools 树直接放入候选包。保留冻结来源不等于发行包已携带输入。
  - 五份原例在 [legacy examples](../../tools/phoenix-diagram/upstream/legacy-2.17/examples) 均有冻结副本。本轮逐份比较确认 `web-app.architecture.json`、`agent-tool-call.workflow.json`、`async-job-roundtrip.sequence.json`、`product-analytics.dataflow.json`、`deployment-release.lifecycle.json` 与活跃旧目录中的对应文件字节一致。
- **最小修订**：在 §1.2/§5.5/§9.2 指定这五份样本的非活跃、可随包分发的新位置及来源/原字节要求，分配给 M6/M7；同步 candidate 必需成员检查、`caseManifest` 及适用的包/bundle 允许清单。按候选自身可辨认布局保留旧候选验证，不放宽样本身份或减少 legacy case；无需保留旧 Node 引擎或将完整 oracle 装进运行包。
- **复核**：改稿可从五份冻结原例追到最终 tgz 和消费路径；实现后 tgz 不含旧活跃入口，五份样本仍与固定来源同字节，双合同消费齐备；历史候选按原布局、新候选按新布局分别获得适用验证。
- **置信度**：中高；删除后的实际路径断链已确认，需修订的是其迁移/分发合同，而非指定某个偏好的目录名称。**阻塞范围**：M6 旧目录删除及 M7/M8 候选消费接线；不影响阶段 A。

## 2. 待核实与待决策

| 项目 | 已有登记与性质 | 影响及所需证据 | 责任/最晚确认点 |
| --- | --- | --- | --- |
| A-1：无历史所有权回执的旧副本是否可被显式认领 | 计划 §0.3、§3.2、§11 已如实登记；不是本轮新增 finding | 用户选择 A/B 后收敛迁移范围、V-5、migrate-known 题面。B 路线须真实报告 unowned，不能以虚构回执验收自动迁移成功 | 用户决策，主实现者同步；M5 开工前 |
| 实际平台/网站/宿主资源 | 执行期证据缺口；计划已区分入口可用与验收通过 | M7 核届时模型/端点类别、collector、GitHub/浏览器入口；M8 取得真实 HTTPS、四平台同包消费及两宿主各 18 会话原件。当前评审未重新检查登录、配额或远端状态 | 执行者；相应检查前。资源不可用时据实阻塞该终验，不新增无证据的当前环境阻塞 |

P3 尚未实现，故目前没有相应原型、浏览器、迁移或 provider 通过记录；这是实施后的验收工作，不因“尚无运行结果”另报计划缺陷。P2 G-07/G-08 的既有失败处置与延期由原记录保留，未扩大为 P3 豁免。

## 3. 覆盖、基线与验证记录

### 3.1 调查基线

- 仓库：`XGENT-ai/skills`；HEAD `dce8662270f993cae3521b59ceff8ac6f22e1b63`。
- 事实来自当前工作树；调查开始即为 dirty。原有改动为 `.xgent-ai/experience/index.md`、`.xgent-ai/sdlc/status/plan/index.md`、`.xgent-ai/sdlc/status/prd/06d03c42-b8de-4f89-bddc-e20f595b5fd2.md`，以及未跟踪的经验记录、P3 计划状态页和本计划。相关产品源码/脚本/skill/dist 没有相对 HEAD 的工作树改动。
- 计划是未跟踪文档，版本以报告头 SHA256 固定；结束前再次核对 HEAD、计划 hash 和相关源码，没有并发变化。报告及本轮 SDLC 索引写入不改变原计划自报状态。
- 需求基线：[PHOENIX-DIAGRAM PRD](../PRD-PHOENIX-DIAGRAM.md) §3、§5.3、§7.2/§7.3；前期边界：[P2 覆盖复审](PHOENIX-DIAGRAM-P2.prd-dev-gaps.2026-10-28.md)，由计划 C-4/§0.5/§11 承接。

### 3.2 四视角任务台账

| 维度 | 实际负责人 | 覆盖与结论 | 状态/证据缺口 |
| --- | --- | --- | --- |
| facts | 独立 subagent `/root/facts` | 核代码/安装/来源/候选包依赖；发现 F-03 | 已完成；未来 P3 行为未运行 |
| architecture | 独立 subagent `/root/architecture` | 核 brands、compare、preview、migration 的接口/所有权/失败恢复；发现 F-01，关联 F-02 | 已完成；固定依赖源码可读，真实 HTTP/TLS 与平台行为待实施 |
| schedule | 独立 subagent `/root/schedule` | 核 16 AC、五条跨期终验、依赖/并行/局部检查与集中收口；发现 F-02 | 已完成；A-1 已登记，未增加真人招募或重复验收门 |
| delivery | 独立 subagent `/root/delivery` | 从安装、创作到审核/文档/失败恢复检查主路径；发现 F-01，关联 F-02/F-03 | 已完成；UI 为文档推演 |
| 综合裁定 | 主 agent `/root` | 亲读每项支持段落与原始证据，跨视角复核、合并同根因、运行脚本并单写报告 | 已完成；报告写入后回读及链接核验 |

四位 reviewer 初轮独立只读调查，之后仅就受影响链路交换证据；没有以一致意见代替原始证据。未委派实施、未改稿，全部 reviewer 已交回结果。

### 3.3 实际检查

| 检查 | 本轮结果与解释 |
| --- | --- |
| `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P3.md <仓根>` | exit 0；检查 87 个路径 token，33 个 MISSING。逐项均属 §1.2 新交付或 §9.1 明示的新输出目录，没有据此把未实现报成缺陷；其中 F-02 审的是交付时点倒置 |
| `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P3.md <仓根>` | exit 0；0/8，8 行里程碑、0 条完成记录，ERROR 0 / WARN 1。告警仅由已有 6 项文档 dirty 和无已完成记录触发；没有产品实现或完成声明矛盾 |
| `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P3.md` | exit 0；ERROR 0 / WARN 9，均为 WORK 自定义 ID 人工核对提示。已对照 PRD 分期矩阵核对九个本期需求 ID |
| `mise exec --no-deps -- npm test` | exit 0；Node 479 项中 449 pass、30 skipped、0 fail；Python 三组 24/2/13 项全部通过。跳过项不记为真实原生验收通过 |
| `npm run test:diagram` / `npm run test:phoenix`、产品浏览器/四平台/provider 检查 | 本轮未运行；只做计划评审，无 Phoenix runtime 修改。轻量测试不证明这些检查通过 |
| 外部版本能力 | 已读固定本地 `ureq-2.12.1/src/agent.rs` 的 resolver、代理、重定向及慢 DNS 超时语义，并与本仓 UI 锁文件核对版本。官方 docs.rs 页面访问失败，未将其作为已读网页证据；未据此提出版本升级或支持周期结论 |

### 3.4 用户主路径与 UI 证据等级

| 主路径 | 文档推演结果 |
| --- | --- |
| 获得工具/安装/恢复 | §3.2、§5.4、V-5/V-8/V-11 定义固定版本、provider、离线、skip、pending 与条件回滚；A-1 决定旧副本归属 |
| 品牌查询/采集/离线交付 | §3.1/§5.1 区分查询、显式 capture 和离线消费，含固定摘要、缺失/变更/脱敏/职责保留；阶段 A 资源前置须按 F-02 调整 |
| 预览更新/出错/停止 | §5.3/§6 定义 waiting/error、last-good、当前 generation、源/目标冲突及端口停止；不把预览当视觉审核 |
| 比较/查看变化/确认交付 | 双输入与成对恢复有设计，三视图/变化定位有交互定义；公开审核收口按 F-01 补齐 |
| 文档调用/最终文档检查 | §0.6/§5.5、V-9/V-10 保留原探测、fallback、不自动安装、语言、三类成果、离线 srcdoc 与最终文档独立检查 |
| 唯一入口/旧内容兼容 | 删除、备份及保留历史范围明确；五份 legacy 消费样本的分发链按 F-03 补齐 |

**界面证据等级：文档推演。** P3 品牌标题轨、比较页及预览 shell 尚未实现；本轮无 P3 静态截图、原型或实际浏览器通过结论。计划 §6/§9.4 已覆盖暗亮主题、五语言、四桌面/390×844、键盘/焦点、空集合、未变事实和三个视图；未确认额外 UI 设计缺陷。按需读取 Phoenix UI 的项目事实优先和规划边界，未启动其设计/安装/hook 工作流。

## 4. 需求映射与实施编排

### 4.1 映射核对

16 条 P3 AC 均能从需求账本追到设计与检查，未发现整条需求遗漏；确认问题是已映射链路中的协议、准备时点和包消费断点。

| PRD 范围 | 计划设计/交付 | 实施与收口 | 本轮缺口 |
| --- | --- | --- | --- |
| F7/AC-1、AC-2 | §3.1/§5.1/§6 品牌 | M1；V-1/V-4/V-8/V-9 | F-02：M4 真实来源/工具准备 |
| F8/AC-1、AC-2 | §5.3/§6 预览 | M3；V-3/V-4/V-8/V-9 | F-02：阶段 A 验收入口 |
| F9/AC-1、AC-2 | §3.3/§5.2 比较及成对恢复 | M2；V-2/V-4/V-8/V-9 | F-01、F-02 |
| F10/AC-1–AC-3 | §0.6/§5.5 唯一入口与调用方 | M6/M7；V-6/V-7/V-9/V-10 | F-03：删除后消费样本 |
| F11/AC-3、AC-4 | §3.2/§5.4 安装/迁移/回滚 | M5；V-5/V-8/V-9/V-11 | A-1 已登记待决 |
| N2/AC-3 | §4/§7 来源/版本/许可 | M7；V-7/V-8/V-11 | F-03 的样本分发身份须同步 |
| N3/AC-3、AC-4 | §3.1/§5.5 兼容处置/扩展 | M1/M6；V-1/V-6/V-8/V-9 | F-03：双合同真实包消费 |
| N4/AC-3 | §3/§8 恢复边界 | M2/M3/M5；V-2–V-5/V-8/V-11 | F-01 的审核身份与确定性成果关系 |
| N5/AC-2 | §5.1/§5.3/§7 安全/显式访问 | M1/M3；V-1/V-4/V-8/V-9 | F-02：保持授权且提前可用的真实采集资源 |

WORK-F11/N2/N3/N4/N5 的前期 AC 已由 V-8/V-11 纳入跨期终验，M8 为统一收口负责人所在里程碑；M7 源码冻结不等于阶段 B 或整期完成。保留现有集中验收，不要求另造重复映射表、子任务文档或逐里程碑全量走查。

### 4.2 依赖与并行波次

建议保留主链 `M1 → M2 → M3 → M4 → M5 → M6 → M7 → M8`；A-1 是 M5 开发门，M4 是既定阶段顺序而非旧来源调查前置。修订下表中的实际产物边即可，无需新建里程碑。

| 波次/任务 | 前置及解锁证据 | 可并行工作 | 共享负责人/运行资源 | 汇合验收 |
| --- | --- | --- | --- | --- |
| M1 | 现有 compile/evidence/output 合同 | 品牌固定数据、独立 fixture；提前准备阶段 A 入口及授权资源 | 主实现者单写 input/scene/render/prepare/CLI、锁文件及身份资源 | V-1 |
| M2→M3 | M1 资源/快照合同及 V-1；M3 再依赖 V-2 | 独立 fixture；比较审核合同按 F-01 先冻结 | cli/delivery/check/prepare 单一集成人；不强行并行改共享文件 | V-2、V-3 |
| M4 | V-1–V-3；F-02 的 author-tools/审核恢复入口及冻结 HTTPS 来源已可用 | 无资源冲突的证据整理 | 本机端口/浏览器/实际图像审核由主集成人串行操作 | V-4，阶段 A 收口 |
| M5→M6 | A-1 已决，完整旧来源已冻结；V-5 解锁 M6 | 旧来源冻结、调用方草案可提前；F-03 五份样本迁移随删除批交付 | install/bootstrap/CLI 单负责人；临时项目隔离；共享文档主实现者集成 | V-5、V-6 |
| M7 | V-5/V-6；阶段 B 产品及样本分发完成 | 独立题面/fixture 准备 | CI/构建/profile/聚合/来源清单单负责人；仅扩最终候选与 providers | V-7、源码冻结提交 |
| M8 | 可取回源码、唯一认证同包、四平台/宿主/浏览器资源 | 隔离的平台消费与全新宿主会话可并行 | 候选服务器端口、浏览器、证据输出隔离；主集成人统一记录与最终状态 | V-8–V-11、16 AC 及五条跨期终验 |

M1–M3 共用 cli/prepare/render，原计划的顺序安排有依据；不同目录并不等于可安全并行。实际验收仍按影响复用证据，不因写报告、回写、提交或续做重复跑全量。

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

```text
$apply-doc-review docs/plan/PHOENIX-DIAGRAM-P3.review.md
```

**建议状态：Blocked。** 与原计划自报状态一致。A-1 仍是已正确登记的政策门；另有三项 P2 计划缺口须修订。主要路线、16 AC 覆盖、两阶段收口及原有授权边界总体成立，修订不需要扩范围或重写整份计划。

1. 在 M2 开工前补 F-01：固定比较审核 summary/record、双输入三视图身份及完成状态，维持原默认实际审核要求。
2. 在 M4 执行前补 F-02：前移阶段 A 所需维护入口和冻结 HTTPS 资源，消除对 M7 的反向依赖。
3. 在 M6 删除前补 F-03：明确五份 legacy 消费样本的新分发位置、原字节及历史候选分派，接入 V-7/V-8/V-11。
4. 用户在 M5 前决定 A-1，主实现者收敛迁移路线和相关验收；不将已有决定门重复列为新缺陷。

解除上述计划缺口并取得 A-1 决策后，再依据受影响链路复核就绪状态；本报告不自动改计划状态，也不启动修订或实施。独立 M1 工作仍可按原授权范围安排；M2 先补公开审核合同。计划评审结论不表示产品已实现、真实平台/provider 已通过或已公开发布。

## 修订处置与复核

- 来源：以上为原始完整评审，原问题、证据、所审基线及结论未改。追加前本报告 SHA256 为 `8bade5d7c1155edded1214cfc4faa7bbc13cb9ddf75c132c1d414e8071ece52d`；本节不另立一轮独立评审。
- 目标：[当前 P3 计划](PHOENIX-DIAGRAM-P3.md)；修前 SHA256 `faa0f3683e7178e6bc24633d193e45d3aad2269d225700c0142bf48d52840ca0` 与原所审版本一致；修后 SHA256 `5e7aa60f8838f0681f54dddb9fe730215bf0b25c2339df167af478fd0b9f3414`。
- 复核：2026-10-28（Asia/Shanghai）；main / `dce8662270f993cae3521b59ceff8ac6f22e1b63`，工作树 dirty。原有计划/经验/SDLC 改动保留；本次只改目标计划、此处置节及既有计划/评审状态页，产品代码、PRD、skill 合同与实施记录未改，未提交或推送。
- 范围：本报告 F-01–F-03 全部条目，以及 §2 已有 A-1/执行资源的影响；按文档层面的修后合同复核，不执行 P3 产品、不回填实施完成数或验收通过。

| 问题/报告章节 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。重读原 review.rs 的 summary/四门绑定、receipts.rs 的单 candidate 严格记录、cli.rs 的浏览器分派及 delivery-contract：它们不能直接承载比较交付；原计划未固定独立审核协议。 | 已修订并复核 | [§3.3](PHOENIX-DIAGRAM-P3.md#33-比较文件)/§5.2 冻结不可变 HTML/CompareReceipt 与独立 ComparisonSummary、创建入口/路径定位、双输入/配置/资源/引擎/IR/原回执绑定、三视图 12 图记录、完整状态及拒绝/幂等重试；V-2/V-4/M2 明确实现、公开说明和验证责任。M2 将同一比较合同及完整草稿示例同步 delivery-contract/architecture-compare，未提前改尚无实现的公开 skill。 | **已解决（文档层）**。沿 compare → check → visual-check → 实际逐张观察 → review-record → 读取同一 summary 可确定完整收口；不再以原 compare JSON 兼任可变状态，不复用单图审核，身份失效及缺输入/视图/旧记录均有明确断言。真实浏览器/看图/实现仍待 M2/M4。 |
| F-02 | 成立。M4 原依赖 M7 才交付的脚本及新 PNG。曾尝试复用当前提交的既有 PNG 公共 URL，但匿名读取为 HTTP 404，不能据本地文件/commit 冒称可取回；未采用该 URL。 | 已修订并复核 | [§9.2](PHOENIX-DIAGRAM-P3.md#92-p3-维护入口与候选身份)/§4/V-3/V-4/M3–M4/M7/提交点：最晚 M3 完成 author-tools、同 run 审核/恢复/故障支撑及测试；给定合成 PNG 的确切 Base64、125 bytes、SHA 和尺寸；M3 准备提交、授权推送、匿名 HTTPS 的完整提交/内容核验必须先于 M4。M7 只补 all/providers/最终候选/profile/题面/CI 接线。 | **已解决（前置编排）**。M1→M4 每个新增命令/资源均有使用前的交付和就绪判据，不再反向依赖 M7。PNG 定义已静态校验，但尚未生成文件、提交、发布或成功真实采集；完整准备 SHA/实际 URL 在 M3 记录，未获授权/不可取回则不进入依赖采集的 V-4，不换图或公开私有仓库。 |
| F-03 | 成立。candidate 的必需成员与 caseManifest 硬编码旧活跃目录；发行允许清单不携带 tools/upstream。五份原例与固定 legacy-2.17 副本逐份 cmp/摘要一致；现有包及 bundle/native 允许平层 examples JSON，可复用边界而不放宽目录。 | 已修订并复核 | [§5.5](PHOENIX-DIAGRAM-P3.md#55-唯一入口和扩展约定)/§1.2/§9.2/V-6/V-7/M6–M7：列出唯一新 skill 下五个 legacy.*.json 路径及固定原字节 SHA；M6 先复制并迁移必需成员/caseManifest/新旧布局测试，再删旧入口；M7 核 sourceInputs→bundle manifest→tgz→消费路径。 | **已解决（分发合同）**。可从五个固定源文件追到新包及消费者；历史 tgz 按原布局/原身份，新 tgz 按新布局，混合/缺失/篡改/未知布局拒绝，不读工作树补样本，不减少五类 legacy 消费。真实打包及同包消费待实施，不称新样本已随包分发。 |

### 决策与未完项

1. **A-1 保持待用户决定，未代选。** 原 ADR-2 的 A 是仅在显式 migrate 时按完整来源/外部管理检查/物理快照严格认领并备份替换；B 是无历史所有权回执均报 unowned、保留并给手动指引。两者都保留新安装、受管升级与回滚。用户在 M5 前选定后，主实现者收敛 §0.3/§3.2/V-5/migrate-known，不得保留两条未决实施路线或伪造历史回执。它不是本轮新增 finding，也不计为三项处置中的未关闭缺陷。
2. **准备工作和执行证据不冒充当前事实。** M3 的准备提交/资源发布授权/匿名 HTTPS 身份尚待执行；未授权时须由用户提供授权发布位置。M4/V-4 与 M8/V-8 的实际采集、四平台、真实 provider 和最终文档验收均仍待实施。HTTP/TLS fixture 或文档内 PNG 解码不能替代这些证据；P2 的原失败/延期边界未改。
3. **整体就绪仍为 Blocked。** 三条计划缺口已闭合，但 A-1 的归属/范围决策尚缺；原计划状态和实施进度 0/8 保持。独立 M1–M3 只能在已有或后续实施授权内推进；本次命令不启动实施、迁移、发布或独立全量复审。

### 修订后结论与验证

- 处置计数：**3 项已修订并复核；0 项无需修改；0 项不采纳；F-01–F-03 无未完处置。** 效果判定均为文档层面的已解决；A-1 及执行资源按上节保留。原报告“需要修改/建议 Blocked”的历史结论不改写；当前计划仍 Blocked，不宣称完整独立评审通过。
- `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P3.md`：ERROR 0、WARN 9；均为原有 WORK-F*/WORK-N* 自定义 ID，已对原账本/PRD 编号复核，本次未改其定义或范围。
- `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P3.md .`：0/8、ERROR 0、WARN 1。警告来自计划尚未纳入 Git 跟踪、工作树已有其他文档改动；核对确认没有实施记录/完成行，不以此虚增进度。恢复快照、完成记录及 ADR-2 与修前逐字一致。
- `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P3.md .`：96 个路径 token、38 个 MISSING。相对修前仅新增五个明确标 ★ 的 M6 legacy 样本，其余为原定新增交付或 M4 输出；复用路径无新增缺失，不把未来文件存在当作当前事实。
- 内存校验：PNG Base64 可严格解码为 125 bytes，SHA/32×32 RGBA/IHDR-IDAT-IEND CRC/解压尺寸匹配；五份 legacy 固定源与活跃原例同字节，计划路径和 SHA 表逐项一致。没有生成产品 fixture 或分发包。
- `npm test`：exit 0；Node 479 项中 449 pass、30 skipped、0 fail，Python 三组共 39 项通过。`git diff --check` 通过；计划未跟踪，另以修前副本对本次文档 diff/空白复核。内部只读复核仅覆盖三条修订链，未发现本范围剩余合同缺口；主处置者另核最终相对回执定位/避免循环摘要的澄清。
- 未运行 `npm run test:diagram`、`npm run test:phoenix` 或 P3 原生/浏览器/provider 验收：没有产品/runtime 修改，本轮只修文档。轻量测试及静态复核不证明产品已实现、测试资源已可访问或已公开发布。
- 既有 [计划状态页](../../.xgent-ai/sdlc/status/plan/b41a85fb-9c61-4d64-a2ad-58483ebc7aa9.md) 与 [评审状态页](../../.xgent-ai/sdlc/status/review/152a12f9-044f-4063-91bc-0dbb63c482ad.md) 接续原 ID 更新；只推进本评审的 disposition，保留原 review 及 Blocked/0/8。原导航路径未变，无需重写索引、PRD 状态或另造事件。
- 推荐下一步：`$resolve-blocked docs/plan/PHOENIX-DIAGRAM-P3.md`，由用户确认 A-1 后收敛迁移/验收；本次仅推荐，不启动。
