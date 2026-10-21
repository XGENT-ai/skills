# PHOENIX-DIAGRAM-P2 · 开发计划评审

评审日期：2026-10-20（Asia/Shanghai）。评审 ID：`72bbedc0-7192-47a1-bbd3-dea11c9f11c3`。本轮为全文初审。

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-DIAGRAM-P2](PHOENIX-DIAGRAM-P2.md) | SHA256 `072ccd6fb815bb97239d057ea9e3b6e6eb3b15baba2512f7fe6fd1ebc3b0c02e`；新增、未提交文件 | 全文；PRD P2 的 14 条 AC、七条本期终验需求、M1–M8、V-1–V-14，以及被依赖的 P1 合同 |

## 1. 确认问题

### [P1] F-01 · 来源核对的 Git 防护不能保证零网络、只读

- **定位**：[计划 §4](PHOENIX-DIAGRAM-P2.md#4-集成与契约) 第 328 行、[§5.3](PHOENIX-DIAGRAM-P2.md#53-来源核对) 第 379–387 行；与 §0.4 第 143 行、§7 第 488 行的不 fetch / 核对零网络承诺冲突。关联 WORK-F3、WORK-N5、M4、V-5。
- **问题**：计划仅在 Git `<2.44` 且存在 `extensions.partialClone` 时停止读对象；清除继承的全部 `GIT_*` 后只恢复 `GIT_NO_LAZY_FETCH` 等四个变量。Git 2.44 尚不识别这项禁懒抓取变量，且 promisor remote 并不一定声明 `extensions.partialClone`。因此允许的部分克隆在缺 commit/tree/blob 时仍可能由 `cat-file` 隐式 fetch，联网并写入对象库。
- **证据**：冻结参考 [repository-evidence.mjs](../../local/phoenix-diagram/reference/archify/renderers/shared/repository-evidence.mjs#L23) 第 23–30、54–60 行同时设置 `GIT_NO_LAZY_FETCH=1` 与 `GIT_ALLOW_PROTOCOL=''`，计划遗漏了传输级兜底。[Git 2.45 发布说明](https://raw.githubusercontent.com/git/git/v2.45.0/Documentation/RelNotes/2.45.0.txt) 的 `--no-lazy-fetch` 条目及 [v2.45 environment.c](https://raw.githubusercontent.com/git/git/v2.45.0/environment.c) 的 `setup_git_env` 才加入相应处理；[v2.44 environment.c](https://raw.githubusercontent.com/git/git/v2.44.0/environment.c) 没有该处理。[v2.44 promisor-remote.c](https://raw.githubusercontent.com/git/git/v2.44.0/promisor-remote.c) 的 `promisor_remote_config` 可单凭 `remote.*.promisor` 或 `remote.*.partialclonefilter` 注册来源；[v2.44 object-file.c](https://raw.githubusercontent.com/git/git/v2.44.0/object-file.c) 的 `do_oid_object_info_extended` 会对缺对象调用 `promisor_remote_get_direct`。以上为本轮查阅的固定版本官方源码，查询日为 2026-10-20。
- **最小修订**：每个 Git 子进程在清理环境后显式恢复 `GIT_ALLOW_PROTOCOL=""`；能力门修正到实际支持边界 2.45。若保留旧 Git 的部分克隆识别，覆盖所有 promisor 配置，或明确采用统一保守降级；同步 §4、失败模式与 V-5，不能靠单一扩展键保证不 fetch。
- **复核**：改稿后所有对象读取路径均受禁传输保护；实现后以旧 Git、2.44、支持版本及仅 remote promisor/filter 配置的缺对象仓库验证，确认没有启动传输 helper、没有新增对象，返回适当的 unverified 原因。超时分支也不得遗留传输子进程。
- **置信度**：高；固定版本源码与参考防护直接支持。**阻塞**：M4 来源核对及依赖其安全承诺的集成收口；修订前整份计划不宜标 Ready。

### [P2] F-02 · 时区接口遗漏参考已接受的固定偏移输入

- **定位**：[计划 §2.1](PHOENIX-DIAGRAM-P2.md#21-模块与接口) 第 252 行、§2.2 第 266 行、[§5.5](PHOENIX-DIAGRAM-P2.md#55-时间与数字格式) 第 414、423 行。关联 WORK-N3/AC-2、M3、V-4。
- **问题**：`zone(&str) -> Option<Tz>` 及遍历 `chrono_tz::TZ_VARIANTS` 只能表达命名时区。冻结参考实际上接受 `meta.timezone: "+05:30"`、`"+01"`、`"-2359"` 等固定偏移，按当前接口将被当作无效时区，缩小了本期承诺消费的有效参考输入集合。
- **证据**：[参考 timeline schema](../../local/phoenix-diagram/reference/archify/schemas/timeline.schema.json) 对 timezone 仅限定非空字符串；[render-timeline.mjs](../../local/phoenix-diagram/reference/archify/renderers/timeline/render-timeline.mjs#L81) 第 81–86 行直接用 `Intl.DateTimeFormat` 校验，墙钟换算也沿用该接口。[ECMA-402 的 CreateDateTimeFormat](https://tc39.es/ecma402/#sec-createdatetimeformat) 明确区分固定偏移与命名时区。本轮在维护 Node **v25.2.1 / tz=2025b / ICU 77.1** 实测三种偏移均被接受，并将冻结 `payment-incident.timeline.json` 的 timezone 改为 `+05:30`、输出改到系统临时目录：冻结 renderer **exit 0、stderr 为空、HTML 811598 字节，含 `UTC+05:30`**。另直接读取本机锁定版本 `chrono-tz-0.10.4/src/prebuilt/timezones.rs`：`Tz` 是封闭枚举，`FromStr` 只查命名表，`TZ_VARIANTS` 的 597 项不含任意固定偏移。[PRD WORK-N3](../PRD-PHOENIX-DIAGRAM.md#work-n3-保留旧输入并允许兼容扩展) 第 318、323 行要求保留有效参考核心输入。
- **最小修订**：让时区类型同时表示命名时区与固定偏移，例如 `Zone::{Named(Tz), Fixed(FixedOffset)}`；同步 `wall`、`offset_label` 等签名，明确按冻结 Node 的偏移语法、规范化与拒绝边界处理，无须更换已选依赖。
- **复核**：改稿后合法偏移可沿全部时间接口传递；V-4 补上述三种偏移及非法边界的 oracle，核对接收结果、墙钟、刻度/标签与完整渲染，不能只复测四个 IANA 样例。
- **置信度**：高；已完整执行参考消费并核对所选 Rust 类型。**阻塞**：M3 timeline 格式与兼容验收前修订。

### [P2] F-03 · 工作树比较规则把正常中间目录也判为不一致

- **定位**：[计划 §5.3](PHOENIX-DIAGRAM-P2.md#53-来源核对) 第 389 行；与 V-9 第 533 行、repo-verified 第 558 行的正常仓库来源 verified 预期关联。影响 M4、V-5。
- **问题**：规则要求逐段 `lstat`，任一段是符号链接或“不是普通文件”就记 `worktree-differs`。按该算法，正常引用 `tools/phoenix-diagram/src/input.rs` 时，三个中间段都是目录、都不是普通文件，因而无法核对通过；这会影响一般嵌套源码路径，而非罕见边界。
- **证据**：本轮对上述四段直接执行只读 `os.lstat`：`tools`、`tools/phoenix-diagram`、`tools/phoenix-diagram/src` 均为 `S_ISDIR=true / S_ISREG=false / S_ISLNK=false`；只有末段 `input.rs` 为普通文件。计划目前没有区分中间段与末段。
- **最小修订**：明确中间段必须是非符号链接目录，末段必须是非符号链接普通文件，再比较 blob 字节；保留原有不跟随符号链接的原则。
- **复核**：改稿能直接推导正常嵌套路径通过；V-5 分别覆盖正常嵌套文件、中间段符号链接、末段符号链接及非普通末段，不能只测仓根单文件。
- **置信度**：高，问题来自明确的逐段判定条件；不推测未来实现一定会照错。**阻塞**：M4 工作树核对实现前修订。

### [P2] F-04 · M5 的真实导出退出条件与单测命令不匹配

- **定位**：[计划 V-6](PHOENIX-DIAGRAM-P2.md#91-检查定义与唯一执行时点) 第 530 行、M5 第 590 行；关联 V-7 第 531 行及顶部验证节奏第 27 行。影响 WORK-F6/AC-2。
- **问题**：V-6 将真实 Chrome 中范围卡片和新增五类五格式导出列为 M5 的局部退出断言，但对应命令只有 DOM/目录/控制器单测。照列出的命令通过后并没有真实导出证据；额外把真实导出加入 M5，又会与 M6 阶段收口的全量真实导出重复。
- **证据**：[export-acceptance.test.mjs](../../tools/phoenix-diagram/tests/export-acceptance.test.mjs#L1) 第 1–2 行明确其测试不启动 Chrome；第 29–37 行仅验证固定一像素 PNG 的签名；第 221–235 行的 `runExportAcceptance` 故意传入不存在的浏览器，并要求导出为 `not-run`。计划 §6.1 也明确新 explore 测试使用 DOM fixture，真实浏览器由 V-7 承担。
- **最小修订**：V-6 保留状态机、目录、卡片范围元数据及导出控制器单测，把真实五格式、范围卡片文件与浏览器解码统一归 M6/V-7；需求映射引用这两个层次，无须新增里程碑。
- **复核**：M5 退出断言均能由所列局部命令证明，真实导出在阶段 A 收口执行并留存实际文件/浏览器结果；单测通过不再被写成真实格式验收通过。
- **置信度**：高。**阻塞**：M5 退出判定前修订，不妨碍独立前期开发。

### [P2] F-05 · M6 的运行器前置倒置，收口命令也未启用所承诺范围

- **定位**：[计划 V-7–V-9](PHOENIX-DIAGRAM-P2.md#91-检查定义与唯一执行时点) 第 531–533 行、[M6/M7](PHOENIX-DIAGRAM-P2.md#10-里程碑与验收安排) 第 591–592 行。
- **问题**：M6 必须运行“原五类×两合同＋新五类×phoenix-v1”的 15 例并完成 Reader、恢复、离线、实际审核，但 M6 的交付只点名 core runner，runtime 的十类更新排在必须等 M6 通过的 M7。现有 core 入口实际委托 runtime；只扩 core 的 `TYPES` 会让 runtime 为新增五类也建立 legacy 用例，得到 20 例并读取 `undefined.<type>.json`。此外，V-7 所列的 `--stage core` 没有给出必需引擎或显式环境，也未启用 Reader/恢复/离线及审核续接；计划没有定义新的默认执行协议。实施者必须自行重排任务并补入口才能达到 M6 退出条件。
- **证据**：[core::runCore](../../scripts/test-phoenix-diagram-core.mjs#L205) 委托 `runCoreStage`；[runtime::DEFAULT_CASES](../../scripts/test-phoenix-diagram-runtime.mjs#L12) 第 12–18 行对每种 `TYPES` 无条件创建两合同用例。[core::parseOptions](../../scripts/test-phoenix-diagram-core.mjs#L33) 第 33–50 行默认 `reader/recovery/offline=false` 且要求 `--engine` 或 `PHOENIX_DIAGRAM_BIN`；本轮仅调用参数解析、清除该子进程的引擎环境后，原样参数返回 `arguments/missing-engine` / exit 2，没有启动验收或写运行目录。[runtime](../../scripts/test-phoenix-diagram-runtime.mjs#L456) 第 456–460 行逐项受选项控制；[HELP](../../scripts/test-phoenix-diagram-core.mjs#L188) 第 188–200 行说明初跑保留 required-pending，需原位 resume、review-file 及相应开关。
- **最小修订**：把 runtime 中 core 用例选择、15 例覆盖及执行逻辑的增量前移 M6；包消费/CI 专属增量仍可留 M7。或明确 M6 交付 15 例 manifest 并在命令引用。给出引擎获取/绑定、初跑、实际看图登记及原位 resume 的完整命令，启用 Reader/恢复/离线和新增 corpus/evidence/locale/explore 范围；如果要改默认值，写出新协议及相应检查，不能隐含假设。
- **复核**：在尚未实施 M7 时，M6 的改动落点和入口已经足够选取正确 15 例、启动全部收口范围；新五类不生成 legacy case。初跑与审核续接分清，所有未运行项保持未完成，不为取得 complete 再次 finalize。
- **置信度**：高；入口、默认集与选项行为已读到实现，倒置来自明确任务归属。**阻塞**：M6 收口前修订。

## 2. 待核实与覆盖限制

没有新增需要用户拍板的产品范围问题。以下是本轮没有执行的未来实现/验收证据，不因尚未实现单独报缺陷，也不能据本报告声称已通过：

| 项目 | 本轮已有证据 / 仍缺证据 | 责任与最晚时点 |
| --- | --- | --- |
| Git 零网络修订 | 已核对官方固定版本源码及冻结参考；未在本机执行旧 Git 的缺对象场景 | M4 实施者按修订后的 V-5 取证，M4 退出前 |
| 时间/数字全量一致性 | 已实测 Node 边界与固定偏移完整参考渲染；未验证未来 Rust 实现的全量 oracle | M3 实施者按 V-4 取证，M3 退出前 |
| 新五类、来源 UI、五语言与真实导出 | 目前是计划/源码推演；浏览器只走查了既有 P1 architecture，见 §3.3 | M6 主集成人按 V-7–V-9 收口 |
| 最终同包四平台、精确 Node 18、双宿主 | 只核对现有入口与计划覆盖；未启动 CI、安装、新宿主会话或最终聚合 | M7/M8 负责人按 V-11–V-13 取得并绑定证据；资源故障沿计划 §11 记录 |

这些实施退出证据不是当前安全设计缺口的替代物：F-01 必须先在计划中修正，再实现和验证。

## 3. 覆盖摘要与调查基线

### 3.1 版本与实际分工

- 代码 HEAD：`e29d4cafcada8809335e0a3dfaa939be95bb9643`，分支 `main`；调查事实源为当前工作树。
- 评审开始时为 dirty：已有 `.xgent-ai/sdlc/index.md`、`status/plan/index.md`、`status/prd/06d03c42-b8de-4f89-bddc-e20f595b5fd2.md` 修改；计划及其状态页 `status/plan/a2287695-163a-4569-ba21-0fff8bf83af5.md` 未跟踪。相应台账路径均相对 `.xgent-ai/sdlc/`。未发现 diagram 源码的既有未提交修改。
- 计划顶部记的是其编写时 clean 基线；本报告不据此把当前工作树描述为 clean。关联 [PRD](../PRD-PHOENIX-DIAGRAM.md) 的 SHA256 为 `e099e47858cc9efeecdda26e1786242a2393cf50d0198538fc0817b144e4263b`。
- 实际启动四位独立只读 reviewer：`/root/facts`、`/root/architecture`、`/root/schedule`、`/root/delivery`。主 agent 重新读取每项候选的原始证据、运行定向只读实验、裁定去重，并单独写入本报告。四位均已交回结果；没有实施委派。
- 结束前复核计划 hash、HEAD 与相关代码变更列表，所审基线未变化。报告和本轮状态登记尚未提交或推送；未提交计划目前仅此工作树可取回。

| 视角 | 覆盖与结论 | 剩余边界 |
| --- | --- | --- |
| facts | 输入合同、scene/check、来源休眠、build INPUTS、Reader 目录、安装邻接面；确认 F-01/F-03 | 未重跑 P1 历史平台统计；未来能力未冒充现状 |
| architecture | enum 分派、共享路由、Git CLI、时间/数字、摘要绑定、候选身份；确认 F-01/F-02 | 不声称未来 Rust 格式完全等价 |
| schedule | PRD 14 条 AC 与七条终验、依赖、局部/阶段门、0/8 进度；确认 F-04/F-05 | CI/真实宿主退出证据由实施取得 |
| delivery | 安装/宿主创作→四门→来源/时间→探索→导出→恢复，核对 UI 状态与材料权限 | 无额外确认 findings；新增 UI 为文档推演，P1 基线另见下表 |

### 3.2 已执行校验

| 检查 | 实际结果与解释 |
| --- | --- |
| `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P2.md <仓根>` | 检查 51 个路径样式 token，列出 28 个 MISSING。逐项按计划第 90 行约定展开：已有资源/参考/测试均在对应根下存在；catalogs、markers、explore/evidence 测试等为显式新增；`agents/openai.yaml` 是不新建的排除项；旧 protocol 是条件兼容路径。未确认虚构复用路径。脚本 exit 0 不代表这些自动通过。 |
| `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P2.md <仓根>` | ERROR 0 / WARN 1；0/8、8 行里程碑、0 条记录。WARN 来自未提交工作树和计划未跟踪，人工核对为计划/台账编写，不是遗漏实施回写。 |
| `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P2.md` | ERROR 0 / WARN 9；九个 WORK 上游工作号均在 PRD 与账本有定义及范围，非悬空 ID。 |
| 仓库轻量测试 | `npm test` 在系统 Node 19.3.0 下退出 0；发现与维护版本不同后，用 `mise exec --no-deps -- npm test` 在固定 Node 25.2.1 下复核，亦退出 0。两次 Node 顶层 34 组均通过，四组 Python 分别 22/24/2/13 项均 OK。它们不证明 Phoenix 原生或 P2 真实环境验收通过。 |
| Node 与参考 renderer | Node 25.2.1 实测固定偏移；冻结 renderer 在系统临时目录完成 `+05:30` 输入的完整消费。只读仓库，没有改参考源码或原示例。 |
| 参数解析 / 文件类型 | core 简写参数明确缺引擎；嵌套路径的中间目录不是普通文件，分别支撑 F-05/F-03。没有启动完整运行器。 |

本轮未运行 Phoenix runtime、全量 diagram 原生回归、四平台/真实 provider 验收、活跃项目安装、部署或公开发布；没有修改目标计划和代码。仓库轻量测试及报告/台账链接回读结果见文末的交付核验。

### 3.3 UI 证据等级与有限浏览器走查

目标新增五类、来源三态与五语言增量属于**文档推演＋源码核对**。PRD 为产品事实源；`phoenix-ui` 的适用边界排除独立 UI 验收，因此没有启动其设计、重设计或写上下文流程。

主 agent 使用隔离 Chrome 会话打开既有 [P1 architecture HTML](../../local/phoenix-diagram/m7-capture-native-claude-koytwsp1/order-architecture.html)，浏览器报告 Chrome 155.0.0.0，窗口请求为 1440×900，页面为 zh-CN：

| 操作 | 本轮观察 |
| --- | --- |
| 节点索引选 browser | 详情显示节点身份与关系；无上游操作明确禁用 |
| 点击下游 Reach | 显示 9 个下游节点、9 条连接、最深 5 跳，hash 为 `focus=browser&reach=downstream` |
| Route 选择 browser→email-service | 显示 browser→api-gateway→order-service→event-queue→worker→email-service，6 节点/5 步；实际查看了该状态的截图 |
| Route 深链接重载、清除 | 重载保留对应路线；清除后 hash 为空，主图仍有 10 个 SVG 节点 |
| 展示与键盘退出 | 进入后 query 为 `present=1`；Escape 退出并恢复入口 |

这只是既有 P1 产物的有限交互基线，不能替代 P2 V-6–V-9 或最终候选审核；没有在本轮执行五格式下载、剪贴板、范围卡片、暗亮全矩阵或新增树/时间图的 UI 验收。隔离页面已关闭，作者源文件未由浏览器操作改写。

### 3.4 已排除的候选与事实勘误

- **不把 ADR-2 判成降低产品默认审核门**：它明确保留另外三平台的 required-pending，只调整候选验证矩阵；本机最终同包仍须四门、实际看图与 review-record complete。PRD 未要求每个平台的回归 fixture 都各自成为 complete，不以 P1 更强的 CI 等待流程强加本期要求；报告也不能把三平台观察称为完整审核。
- **不把 custom translations 的措辞歧义升成缺陷**：已有 [Reader 回归](../../test/phoenix-diagram/reader-assets.test.js#L293) 覆盖 `fr` 加有效单键翻译；计划明确沿既有解析规则，custom locale 可以解释为受支持。实施宜让 Rust/Reader/回执共享同一判定，保留无覆盖、有效覆盖与无效占位符的边界用例，不据歧义制造产品阻塞。
- **现状表可顺手校正**：计划第 176 行把两份 ERD 示例误记为含 brand 且不含 views；已读冻结原件 [orders.erd.json](../../local/phoenix-diagram/reference/archify/examples/orders.erd.json#L9) 与 [subscription-billing.erd.json](../../local/phoenix-diagram/reference/archify/examples/subscription-billing.erd.json#L9)，实际均含 `meta.views`、不含 brand。品牌在 `brand-aware-delivery.architecture.json` 的组件上。V-8 白名单已经包含 views，未证明该误述单独影响交付，不另列 finding。

## 4. 需求映射与实施编排

### 4.1 覆盖范围

PRD P2 的 14 条 AC 均能追到计划的设计与检查定义，未发现新增需求断链：

| 本期验收 | 主要落点 | 本轮发现的修订关联 |
| --- | --- | --- |
| F1/AC-2；F4/AC-2；N2/AC-2 | D1、§3.1/§5.2/§5.4；V-1/V-3/V-4/V-7/V-13 | F-05 修正实际执行入口 |
| F2/AC-3 | §0.6/§6.5、§9.2；V-10/V-11 | 无独立断链 |
| F3/AC-1–AC-4 | ADR-1、D7、§3.3/§5.3/§6.4；V-5/V-9/V-11 | F-01/F-03 修正来源算法 |
| F5/AC-2、AC-4；F6/AC-2 | §6.1/§6.2；V-6/V-7/V-13 | F-04 区分单测与真实导出 |
| N1/AC-3、AC-4 | §6.3/§7；V-2/V-6/V-7/V-9/V-11/V-13 | F-04/F-05 修正验收证据链 |
| N3/AC-2 | D2、§3.1；V-1/V-8，格式边界由 V-4 支撑 | F-02 补完整有效时区集合 |

F1–F6、N1 的七条本期终验还包括已交付 P1 AC 的受影响回归；计划已在 M6/M8 收口安排。N2/N3 的终验仍在 P3，F11/N4/N5 的后续范围没有提前算成终验。品牌、预览、比较、旧入口替换、活跃安装与公开发布均按 §0.5 排除。无需再复制完整需求表或新增过程文档。

### 4.2 建议依赖边与并行波次

原计划大部分拓扑成立；只需修正 M6 的执行器前置和 V-6 的检查归属。关键边为：M1 输入/场景/目录→M2/M3/M4；T-1 路由→ERD/class，ERD grid→class；T-4 格式→timeline/waterfall；M2/M3 渲染→M4 新类型来源集成；Reader 属性＋来源显示→M5；V-1–V-6→M6；M6→M7；文本/源码冻结与宿主结果→M8。

| 波次 / 任务 | 前置与解锁证据 | 可同时推进 / 写入边界 | 资源与汇合负责人 |
| --- | --- | --- | --- |
| M1 | 无；完成 V-1 与实际被下游依赖的 V-2 子集 | 主集成人稳定 `input.rs`、`scene.rs`、目录/装配器和依赖锁 | 共享契约单写；本轮问题须在对应接口开工前修文 |
| M2/M3/M4 | 输入/场景合同稳定；共享路由与格式分别先通过对应局部检查 | 独立类型文件和来源算法可并行；class 等 ERD grid，timeline/waterfall 等格式；M4 新类型端到端断言等待渲染 | 主集成人统一 `render.rs`、`check.rs`、`delivery/*`，不让多人同时改共享文件 |
| M5 | V-3/V-4 的 Reader 属性与 V-5 来源显示 | 目录/状态机/控制器局部检查；真实导出按 F-04 留 M6 | 模板与公共导出逻辑统一集成；浏览器/临时项目隔离或串行 |
| M6 阶段 A | V-1–V-6；按 F-05 已交付 core 所需 runtime/manifest 与完整执行协议 | 一次本机集成，完成 V-7–V-9；失败只重跑失效范围 | 主集成人掌握浏览器、看图与原位审核续接；共享引擎/证据目录单一归属 |
| M7 | M6 行为稳定、参考整合清单已审阅 | 文本与包脚本可按路径分工；两宿主用不同临时项目 | skill、版本、共享脚本单写，V-10/V-11 汇合后冻结 |
| M8 阶段 B | 冻结源码与 V-10/V-11；最终候选证据绑定 | 四原生平台 CI 可并行，唯一 tgz 后进行本机 distribution 与聚合 | 主集成人核对同包/同源、证据复用及最终 V-12–V-14；不把待审平台记成 complete |

不存在可据此计算的工期数字；上表只给实际依赖和安全并行边界。修订期间已确定且不依赖问题合同的前期工作可推进，本报告不自动启动开发。

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

建议状态：**Blocked**。确认 **1 项 P1、4 项 P2**。与原计划顶部和 §11 自报的 Ready 不一致：来源核对目前不能保证其零网络/只读安全承诺；固定偏移兼容、工作树目录判定以及 M5/M6 的退出协议也需要先补正。Blocked 针对这些合同及其依赖路径，不表示所有独立前期工作都无法开始。

修订顺序：先处理 F-01 的 Git 能力边界与禁传输；再处理 F-02/F-03 的格式类型和工作树算法；随后处理 F-04/F-05 的检查归属、runtime 前置与完整执行命令。同步受影响的账本、风险、V-4/V-5/V-6/V-7–V-9、M3–M6 及 Ready 理由，不必重写整份计划。

解除条件：五项修订都有唯一、相互一致的接口/算法或执行归属；对应验证能观察所承诺结果，M6 不再依赖 M7 才具备的执行器。实现后的真实浏览器、宿主与四平台检查仍按原阶段门取证，本次计划复核不代替这些验收。原计划和目标代码未修改。

下一步：

```text
$apply-doc-review docs/plan/PHOENIX-DIAGRAM-P2.review.md
```

### 交付核验

完整报告由主 agent 写入后回读，沿“评审对象”引用核对计划身份与 SHA256，并检查本地引用存在。按仓内协议登记独立 review 对象及计划 readiness，保留计划自报 Ready 与实施 0/8 的事实；没有改原计划状态、实施进度或 PRD 结论。

交付前核验：报告 24 个本地引用均存在；本轮 review 状态页、计划状态页及 review 索引的链接均可解析，review ID 恰好登记一次，目录无漏项或失效条目。仓库固定版本的轻量测试退出 0；目标计划 SHA256 与评审开始时一致，相关源码无本轮改动。

## 修订处置与复核

- 来源：本报告，评审 ID `72bbedc0-7192-47a1-bbd3-dea11c9f11c3`，处置前报告 SHA256 `e46738bee30c91e5ed570cd0a2b1e5e258423d369b5bd7c056ab14d2deff93cc`。问题 ID F-01–F-05 不变，另处理 §3.4 的现状表勘误。
- 目标：[PHOENIX-DIAGRAM-P2](PHOENIX-DIAGRAM-P2.md)。修前 SHA256 `072ccd6fb815bb97239d057ea9e3b6e6eb3b15baba2512f7fe6fd1ebc3b0c02e`，即所审版本；修后 SHA256 `94a8c57e4531f1214c83aba6dafce1a57a7b9227ac77292c69b2c54c53c547ed`。计划仍未跟踪，修前版本只留有 hash，没有可取回的副本。
- 复核日期与代码基线：2026-10-20（Asia/Shanghai）；main / `e29d4cafcada8809335e0a3dfaa939be95bb9643`，工作树 dirty：计划、报告与台账未提交，diagram 源码无改动。
- 本次范围：全部五项与 §3.4 勘误，与原评审范围相同。只修订计划文档；没有修改代码，也没有运行实施任务。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 问题成立。参考 [repository-evidence.mjs](../../local/phoenix-diagram/reference/archify/renderers/shared/repository-evidence.mjs#L23) 第 23–30、54–60 行同时设置两个变量。Git 2.45.0 发布说明加入 `--no-lazy-fetch`，v2.45 `environment.c` 的 `setup_git_env` 处理 `NO_LAZY_FETCH_ENVIRONMENT`；v2.44 的同一文件没有该处理。v2.44 `promisor_remote_config` 单凭 `remote.<name>.promisor` 或 `partialclonefilter` 就能注册 promisor。本机 Git 2.53 在会话临时目录实测：`clone --filter=blob:none` 不写 `extensions.partialClone`；只设空 `GIT_ALLOW_PROTOCOL` 时，读取缺失 blob 报 `transport 'file' not allowed`，对象仍缺失；不设防护则同一对象被抓回 | 已修订并复核 | §1.1 参考算法与 git 事实；§4 git 行；§5.3 进程设置（加空 `GIT_ALLOW_PROTOCOL`，超时杀进程组）与第 4 步（2.45 边界、全部 promisor 键、保守处理）；§7 WORK-N5 与安全条目；V-5 零网络用例；§11 定级理由 | 所有 Git 子进程都受禁传输保护，能力门与 promisor 判定在文档中唯一；V-5 可观察到“缺失对象仍缺失”。真实旧版 Git 未在本机执行，按 V-5 在 M4 退出前取证，无真实旧版时记未运行 |
| F-02 | 问题成立。参考 [timeline schema](../../local/phoenix-diagram/reference/archify/schemas/timeline.schema.json) 只要求非空字符串；[render-timeline.mjs](../../local/phoenix-diagram/reference/archify/renderers/timeline/render-timeline.mjs#L81) 第 81–86 行用 `Intl.DateTimeFormat` 校验，第 427 行轴说明使用作者原串。Node v25.2.1（tz 2025b / ICU 77.1）实测接受 `±HH`、`±HHMM`、`±HH:MM` 与 U+2212，`-00`、`-00:00` 规范为 `+00:00`；带秒、`+24:00`、`+05:60`、`+5:30`、`Z`、`UTC+05:30`、首尾空白被拒。本机 `chrono-tz-0.10.4` 的 `Tz` 为 597 项封闭枚举，`FromStr` 只查命名表；chrono 0.4.45 自带 `FixedOffset`。完整参考渲染沿用报告的实测，本次未重跑 | 已修订并复核 | §1.1 时间事实；§2.1 `Zone` 接口与签名；§2.2 依赖说明（不新增依赖）；§3.1 timeline 行；§5.5 zone 语法、轴说明、`wall` 与 oracle；V-4；§11 | 合法偏移能沿 zone → wall → offset_label → 轴说明传递，接受与拒绝边界都进 oracle。Rust 一致性在 M3 按 V-4 取证 |
| F-03 | 问题成立。原规则对中间目录段同样要求“普通文件”，按文本推导，正常嵌套路径必然 `worktree-differs` | 已修订并复核 | §5.3 工作树比较：中间段须为目录，末段须为普通文件，均不跟随符号链接，缺失也记 differs；V-5 新增四类路径用例；§11 | 按新规则可推导正常嵌套路径为 verified；不跟随 symlink 的原则保留 |
| F-04 | 问题成立。[export-acceptance.test.mjs](../../tools/phoenix-diagram/tests/export-acceptance.test.mjs#L1) 第 1–2 行说明不启动 Chrome；第 29–37 行只测固定一像素 PNG 的签名；第 221–235 行以不存在的浏览器要求导出为 not-run | 已修订并复核 | V-6 末条改为导出控制器单测范围，注明真实导出归 V-7；V-7 写明新五类五格式与卡片文件都核对签名并经浏览器解码；M5 内容同步。WORK-F5/F6 的映射原本就同时引用 V-6、V-7，未改 | M5 的退出断言都能由所列 `node --test` 证明；真实导出只在 M6 收口执行一次 |
| F-05 | 问题成立。core 的 `runCore` 委托 runtime；runtime 的 `DEFAULT_CASES` 对每个 `TYPES` 建两合同用例；`parseOptions` 三个开关默认 false，且必须提供引擎；HELP 写明初跑保留 required-pending，需原位 resume 与 review-file。候选检查器自建十类 × 两合同的包内 manifest（`check-phoenix-diagram-candidate.mjs:195`），属于包消费，留在 M7。命令序列沿 [P1 M4 记录](PHOENIX-DIAGRAM-P1.records/M4.md#阶段-a-退出证据与继续动作) 的实际做法 | 已修订并复核 | M6 前移 core 覆盖、runtime `DEFAULT_CASES` 与收口段执行、四个新范围开关、explore-reader 和 core-runner 用例；V-7 命令改为先跑单测，再执行五步序列（构建引擎并显式绑定、初跑、范围续接、看图与 review map、原位登记），写明失败续接与 not-run 规则；V-8/V-9 改为引用对应开关与看图登记；M7 只留 runtime 包消费/CI 增量与 candidate 开关；§11 | M7 未实施时，M6 的落点与命令已足以选取 15 例、启动全部收口范围；新五类不建 legacy 用例；初跑与审核续接分开，未运行项不冒充完成 |
| §3.4 现状勘误 | 成立。冻结示例中 `brand` 只出现在 `brand-aware-delivery.architecture.json` 的六个组件上；两份 ERD 示例只含 `/meta/views` | 已修订并复核 | §1.1 冻结归档条目 | 与 V-8 延期字段白名单（brand/views/animation）一致 |

### 决策与未完项

- 本次没有需要用户拍板的新范围或产品语义；以下取舍都保持既定目标：
  - F-01 选择“覆盖全部 promisor 配置 + 所有版本禁传输”，没有选择“< 2.45 统一降级”。后者会让旧版 Git 下的普通完整克隆也全部 unverified，超出 ADR-1 定下的“旧版 git 遇部分克隆”降级范围。
  - F-05 新增四个独立开关，沿用现有开关“未启用即 not-run”的语义，不改默认值。V-8/V-9 只要求实际看图，所以片段截图按观察登记，不扩大 review-record 的适用范围。
- 未完项都是实施取证，不阻塞开工：V-5 的旧版 Git 用例、V-4 的 Rust 固定偏移一致性、V-6–V-9 的全部实测，均按原里程碑退出门取得。

### 修订后结论与验证

- 处置统计：五项均已修订并复核，§3.4 勘误也已修订；没有不采纳、待决或延期项。
- 就绪判断：**Ready**。五项的接口、算法和执行归属都已唯一且相互一致，M6 不再依赖 M7 才具备的执行器，也没有影响安全、数据或外部契约的未决项。原报告的 Blocked 结论和所审基线保留；本节只是修订复核，不是新一轮独立评审。
- 实际运行的文档检查：
  - `check_paths.sh`：52 个 token，MISSING 28 个，与评审时逐项判定的集合相同；新引用的 build、test、core-runner 路径均存在。exit 0。
  - `check_progress.sh`：ERROR 0 / WARN 1（工作树未提交），进度 0/8。exit 0。
  - `check_plan.py`：ERROR 0 / WARN 9，与评审时相同，都是 WORK 自定义 ID。exit 0。
  - 搜索旧口径：计划中已没有 `2.44`、`Option<Tz>`、`, Tz)`、“真实 Chrome”、“corpus 段”、“扩展后的”。
  - 定向事实实验：F-01 的 Git 2.53 临时仓库实验、F-02 的 Node Intl 偏移接收实验，都在会话临时目录进行，没有写入仓库。
- 未运行：`npm test`（未改 skill 或代码）、Phoenix runtime、任何浏览器、宿主或 CI 检查。
- 关联文档：PRD 无需同步；台账同步更新计划 readiness 与本评审的 disposition。
