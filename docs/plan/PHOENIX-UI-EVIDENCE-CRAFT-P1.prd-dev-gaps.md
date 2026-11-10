# PHOENIX-UI-EVIDENCE-CRAFT-P1 · PRD 与开发交付差异核查

- 审查对象：[Phoenix UI 取证、中文排版与素材细节增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.2（`sha256:09b19f65376834f46480a3dec8394e8ab1a18ba2707f0399da0e9be0caad1e25`，未跟踪）· **P1** × [开发计划 PHOENIX-UI-EVIDENCE-CRAFT-P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md)（工作树版本 `sha256:b366b163d0df80776e9664dd61c9c1fcc5d435feb0127bc35f762ac9519c5a7d`，已修改未提交，6/6 里程碑完成）
- 审查方式：完整核查（首轮，无上轮差异报告）
- 审查日期：2026-11-10（Asia/Shanghai）

## 1. 结论与范围

**整体结论：存在明确遗漏 · 存在合理变更。** 本期 PRD 范围内 54 条 AC 逐行建账：53 条已满足（其中 WORK-N2/AC-1 为已确认变更），1 条部分满足（WORK-F1/AC-3）；另有 WORK-F1 边界中“报出 iframe / Shadow DOM 限制”这一项部分满足。

| 统计 | 数量 |
| --- | --- |
| A · 明确遗漏 | 2（G-01、G-02） |
| B · 合理变更 | 1（G-03） |
| C · 需 Triage | 0（已决 0 / 待决 0） |
| 证据不足（待核实） | 0 |
| 需跟进 | 2（G-01、G-02 → gaps plan） |

- 两项 A 都在 `capture` 步骤失败时的报告文字里（WORK-F1/AC-3 与 WORK-F1 边界），不阻塞主路径与本期出口判据 WORK-F3/AC-1、WORK-F4/AC-3；但 PRD §7.1 的 P1 出口还要求“本期全部 AC”，所以补齐前不能说 P1 全部 AC 都已交付。
- 计划已通过 [R4 代码复审](PHOENIX-UI-EVIDENCE-CRAFT-P1.code-review.r4.md)（通过验收），V-1–V-10 的记录与本次抽查一致；两项遗漏是计划 §5.4 在设计阶段把 AC 收窄造成的，计划评审和代码评审都没有指出。
- 跨期 NFR（WORK-N1–N4）只核对 P1 列的 AC；终验期 P2/P3/P4 不变，不能转为“已上线”（PRD §7.3）。P2–P4 需求（WORK-F5–F14 等）不在本期。
- 本报告只说明覆盖核查已完成；产品交付完成要等 gaps plan 实施并验收后再判断。

## 2. 三类差异

### A · 明确遗漏

#### G-01 · 目标隐藏、被遮挡或禁用时，报告不给可见元素的定位写法 · A

- PRD 期望：WORK-F1/AC-3 后半句（[PRD §3 WORK-F1](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md#work-f1-按步骤取得多状态界面证据可供其他-skill-调用) AC 表 AC-3）：“目标被遮挡、处于禁用或隐藏状态时，报告给出提示与可见元素的写法。”§2.4 也要求失败时给出“可照做的修正写法”。
- 实际交付：
  - 隐藏（display:none、visibility、opacity:0、零尺寸、滚入后仍在视口外）：状态失败，原因写“找到 1 个匹配，但都不可见：隐藏（display:none）；先执行让它出现的那一步”，`hints` 为空数组。
  - 被遮挡：步骤记 `warning`，文字为“目标中心被 span.veil 遮挡，动作照常执行”；遮挡物名称是 `标签#id.class` 拼接，不是状态文件里的定位 JSON。
  - 禁用：步骤记 `warning`，文字为“目标处于禁用状态（disabled），动作照常执行”，不附任何定位写法。
  - 只有“找不到”时才输出至多 3 个候选写法（终端显示“可改为：{"text":"被盖住"}”）。
- 证据链：计划 [§5.4](PHOENIX-UI-EVIDENCE-CRAFT-P1.md#54-定位动作与失败提示) 只给“找不到”设计候选写法，隐藏只给“先执行让它出现的那一步” → [M1–M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M2.md#验收) V-2⑥ 只核对了原因与警告文字 → 代码 `tools/phoenix-ui/crates/cli/src/capture/steps.rs` 的 `locate`（`Some("hidden")` 分支 `hints: Vec::new()`）、`page_js.rs` `LOCATE`（遮挡、禁用只推入警告字符串）→ 测试 `crates/cli/tests/capture.rs` `step_failures_and_warnings` 只断言原因含 `display:none`、警告含“遮挡”“禁用”→ **本次实测**：当前 debug 二进制（`binarySha256 0e0148f303ce…`；`crates/` 与 HEAD 无差异）对 `tests/fixtures/capture/pages/failures.html` + `failures.json` 运行，`hidden-target` 的 `hints=[]`，`covered`/`disabled` 的警告如上。
- 影响：主代理、走查子代理、feature-demo 与 review-ui 按报告修状态文件时，只知道“看不见/被挡/禁用”，得自己猜该改成哪个元素，或读页面源码才能修正（走查者被禁止读源码）。不阻塞主路径，只是多花修正轮次。
- 分类依据：AC 文字明确，属于本期；计划把它收窄后没有任何已批准变更或用户决定（[决定记录](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.decisions.md)、[计划评审](PHOENIX-UI-EVIDENCE-CRAFT-P1.review.md)都未涉及）；补齐只需复用已有的候选生成逻辑，不涉及新的产品取舍。隐藏仍判失败、遮挡与禁用仍记警告并照常执行，这一已定语义保持不变。
- 处置：补齐。
- Follow-up：见 §6 与 gaps plan。验收：对 `failures.html` 的隐藏、遮挡、禁用三种目标，报告分别在 `failures[].hints` 或该步骤 `warnings`/`hints` 中给出提示，以及状态文件格式的可见元素定位写法（遮挡时含遮挡物的定位；页面上有同角色或相关文字的可见可交互元素时列出至多 3 个候选）；终端照常显示“可改为：”。

#### G-02 · `waitFor` 超时不报 iframe / Shadow DOM 限制 · A

- PRD 期望：WORK-F1 边界（[PRD §3 WORK-F1](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md#work-f1-按步骤取得多状态界面证据可供其他-skill-调用)“边界”）：“选择器只作用于主文档，目标在 iframe 或 Shadow DOM 内时报出该限制”；AC-7：“可等待元素消失；条件不成立时按 AC-3 处理”。
- 实际交付：`click` 等动作找不到目标时，原因追加“选择器只作用于主文档：页面有 N 个 iframe、M 个开放 shadow root，可直接打开被嵌入的页面”；而 `waitFor` 超时的原因只写“等待 text="框内按钮" 出现超时（300 ms），当前可见匹配 0 个”，不提这一限制。
- 证据链：计划 §5.4 只为“找不到”设计了这条提示 → 代码 `steps.rs` `WaitFor` 分支超时时重新拼 `reason`，丢掉了 `locate` 在 `missing` 时追加的说明 → **本次实测**：同一二进制、同一 `failures.html`（内含 1 个 iframe），状态文件 `{"do":"waitFor","target":{"text":"框内按钮"},"timeoutMs":300}`，退出 2，失败原因如上，没有 iframe 说明。
- 影响：先等待异步内容出现，是嵌入式应用（如门户微应用，PRD Q4）最常见的写法；调用方只看到超时，会误判成加载慢或步骤错，不知道应改为直接打开被嵌入页面，或按 WORK-F15/F16 回到原流程。
- 分类依据：PRD 边界明确要求“报出该限制”，没有已批准的收窄；补齐属于同一失败语义，不涉及产品取舍。
- 处置：补齐。
- Follow-up：见 §6 与 gaps plan。验收：`waitFor`（visible）在目标只存在于 iframe/开放 shadow root 的页面上超时时，原因同样带“选择器只作用于主文档：页面有 N 个 iframe、M 个开放 shadow root…”；`until: hidden` 超时语义不变。

### B · 合理变更

#### G-03 · WORK-N2/AC-1 用同一安装器与四平台 CI 候选验证，未经 npm 发布 · B

- PRD 期望：WORK-N2/AC-1：“用户经 `npx @xgent-ai/skills install` 安装或升级后，在 macOS（arm64/x64）与 Linux（x64/arm64）上无需 Rust 编译器即可运行取证命令；engine 缺失或版本不符时返回固定包版本的补装命令，运行中不自动下载。”
- 实际交付：未发布 npm/R2。四平台由 [Phoenix run 42808136237](https://github.com/XGENT-ai/skills/actions/runs/42808136237)（候选 `0509ba6`）的 UI Native 构建与冒烟（`capture=screenshot`）证明，Node 18 消费者冒烟消费 linux-x64 制品，本机 V-9 用同一安装器 `bin/xgent-skills.js install --release-dir` 安装开发候选；缺 engine 时退出 4 并给补装命令（V-7 冒烟）。
- 证据链：计划顶层排除与 [§0.5](PHOENIX-UI-EVIDENCE-CRAFT-P1.md#05-明确不在本期)“npm 发布与 R2 上传…需用户另行授权”、D15 → [M6 记录](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md#当前候选-v-8-ci) → R4 §4.3 核对制品摘要；`.github/workflows/phoenix.yml` 的 Node 18 job 运行 `smoke-phoenix-ui.mjs --release-dir … --target linux-x64`（本地 release 目录安装，不经 npm 仓库）。
- 影响：“用户经 npx 安装后可用”这一终端用户效果，要等下一次发布才会出现；本期证明的是同一份候选在四平台可装、可跑。
- 分类依据：PRD 头部写明“安装与发布由后续开发计划承接”，§7.3 规定 P1 完成后停在“已提交 / 可测试”、不转“已上线”；替代验证覆盖了同一安装器、四平台与 Node 18 底线，产品承诺不变，只是用户可用的时间点推迟到发布。
- 处置：保留。
- Follow-up：本期无动作。重新评估条件：发布 Phoenix 时须走 CI 构建，不能用 `--reuse-engines`（计划 §0.5、§12），并在已发布包上复核 npx 安装路径；发布需用户另行授权，不进入本次 gaps plan。

### C · 需 Triage

无。

## 3. 完整需求覆盖账本

证据等级：**实测**＝本次运行；**记录**＝完成记录/评审中的运行证据（本次复核其命令、结果与适用基线）；**CI**＝Phoenix run 42808136237；**宿主**＝V-6/V-9 原始会话（M5/M6 宿主补证，R4 核读）；**静态**＝本次读当前源码或 skill 文本。

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F1/AC-1 默认视口图、像素=视口×像素比、报告逐张出处 | P1 | §5.1、§5.3、§5.8；M1 | M2 V-1①⑤ | 记录：视口图 2560×1600（1280×800、dpr 2），报告逐张含目标、视口、文件、控制台、失败请求与图片、横向溢出；静态：默认 `--type viewport` | 已满足 | — |
| WORK-F1/AC-2 命名状态组各自从新页面执行，逐步记录实际观察 | P1 | §5.2–§5.4；M2 | M2 V-1① | 记录：8 张图；每状态在独立浏览器上下文打开（偏差已就地修订）；`observed` 只写实际观察 | 已满足 | — |
| WORK-F1/AC-3 前半：找不到/等待超时→该状态失败、指出步骤与原因、无冒充截图、其他照常、非成功退出 | P1 | §5.4、§5.5；M2 | M2 V-1④、V-2⑥ | 实测：`FAILED … step 1 (click)`，退出 2，旧图删除；其余状态照常 | 已满足 | — |
| WORK-F1/AC-3 后半：遮挡/禁用/隐藏时给提示与可见元素的写法 | P1；无收窄批准 | §5.4（已收窄） | M2 V-2⑥ | 实测：只有提示文字，无定位写法 | **部分满足** | G-01 → gaps plan |
| WORK-F1/AC-4 每张图附页面规则发现；初始状态与 `detect --viewport` 一致 | P1 | D6、§5.10；M1 | M2 V-2① | 记录 + CI：`capture.rs` 一致性用例（静止 http、延迟淡入、手机宽、`file://`）四平台 8/0/0 | 已满足 | — |
| WORK-F1/AC-5 元素图/整页图尺寸与取景规则；不符判无效 | P1 | §5.5 第 7 条；M2 | M2 V-2⑤ | 记录：整页 400×3000 有效、dpr 2 元素图 240×160；静态：`validity.rs` 规则 7 | 已满足 | — |
| WORK-F1/AC-6 无浏览器→明确报不可用、非成功、无截图 | P1 | §5.8；M1 | M2 V-1⑥；M6 V-9 A3 | 记录 + 宿主：退出 1、“浏览器不可用…”、不建输出目录 | 已满足 | — |
| WORK-F1/AC-7 角色与可访问名称定位、等待消失；不成立按 AC-3 | P1 | §5.2、§5.4；M2 | M2 V-2⑥ | 实测：`wait-hidden` 超时失败并写条件、毫秒数、可见匹配数；角色定位由 V-1④ 提示 `{"role":"button","name":"筛选"}` | 已满足（隐藏/遮挡等提示缺口见 G-01） | — |
| WORK-F1/AC-8 写拦截：非 GET 中止并逐条记录，只放行指定路径 | P1 | D9、§5.7；M2 | M2 V-1②、V-2②、R1-1 | 记录 + CI：fetch/表单/beacon/XHR/跨站 iframe/worker/SW 写入被拦，服务端只收到 `/api/allowed` | 已满足 | — |
| WORK-F1/AC-9 调用方指定输出目录与文件名；报告记类型、像素比、配色、时间、命令版本 | P1 | §5.1、§5.8；M1–M2 | M2 V-1⑤、R2-5b | 记录：`--name 'U1-{state}-{width}-{scheme}.png'` 按模板写入；逐张 `tool` 身份在部分重取后保留（F-05 已解决） | 已满足 | — |
| WORK-F1 边界：iframe/Shadow DOM 内目标时报出限制 | P1；无收窄批准 | §5.4（只覆盖“找不到”） | M2 V-2⑥（iframe 提示） | 实测：click 找不到时有说明；`waitFor` 超时没有 | **部分满足** | G-02 → gaps plan |
| WORK-F1 边界：只做 Web、不录视频、不出评审结论、拦截默认关 | P1 | §0.4、§0.5、§5.1 | — | 静态：`--block-writes` 需显式开启；无录像参数；报告无打分字段 | 已满足 | — |
| WORK-F2/AC-1 8 帧序列图、时间标注、手感说明 | P1 | §5.6；M3 | M3 V-3 主场景 | 记录：8 格时间递增标 `+N ms`，`note` 含“不代表正常速度手感已核对” | 已满足 | — |
| WORK-F2/AC-2 从首次进入取帧；无出场动画时注明 | P1 | §5.6；M3 | M3 V-3、静态页 | 记录：entry 首格空白；静态页 `未检测到出场动画` | 已满足 | — |
| WORK-F2/AC-3 遮字版；缩略图桌面 480 / 手机 180 | P1 | §5.6；M3 | M3 V-3、V-4 | 记录：遮字版文字全无、图形保留；缩略图 480×300 与 180×390。按视口 CSS 宽 ≥600/＜600 选宽度，两种预设与 PRD 一致，自定义视口按同一阈值 | 已满足 | — |
| WORK-F2/AC-4 标注图框与编号、缺失元素列未标注 | P1 | §5.6；M3 | M3 V-3、R1-4 | 记录：#1 框在“应用筛选”，#2 进 `unmarked`；窄元素不再 panic | 已满足 | — |
| WORK-F2/AC-5 拼图统一比例、同区域、标签；越界明确失败无文件 | P1 | §5.6 `capture stitch`；M3 | M3 V-3 拼图 | 记录：2924×654 拼图；越界退出 1、未写文件 | 已满足 | — |
| WORK-F3/AC-1 code-led 完工图由 capture 生成于 `.phoenix-ui/review/`、沿用文件名、注明截图类型 | P1 | §6.1 第 1、5 条；M4 | M5 V-5；M6 V-9 A1、A6 | 静态：new-work.md §7（:146、:150、:152）；宿主：A1/A6 实际 capture 与评审包含报告及类型 | 已满足 | — |
| WORK-F3/AC-2 无效图标注原因、不交评审、重取后再交 | P1 | §5.5、§6.1 第 2 条；M2、M4 | M2 V-2⑤；M6 V-9 A1 | 记录 + 宿主：`.invalid.png` 与原因；A1 另一次 `--only` 为补无效图 | 已满足 | — |
| WORK-F3/AC-3 修复轮同路径重取、注明沿用 | P1 | §6.1 第 6 条；M4 | M6 V-9 A2 | 宿主：`7c047…` 第 326 行只重取受影响状态；静态：:156 `fix` 段 `--only` 与沿用说明 | 已满足 | — |
| WORK-F3/AC-4 comp-led 门禁与 component-review 不变 | P1 | §0.5、C-5；M6 | M6 V-7、V-10 | 记录 + CI：comp-verbs 与相关 reference 零 diff；test:phoenix 全绿 | 已满足 | — |
| WORK-F3/AC-5 宿主与 Phoenix 都无浏览器→“未取证”、无 ship | P1 | §6.1 第 4 条；M4 | M6 V-9 A3 | 宿主：`e21ee…` 第 275 行原样写“未取证”“未完成 UX 走查”，无 ship | 已满足 | — |
| WORK-F3/AC-6 截图轮次不超一轮批量加一轮确认；不另跑 detect | P1 | §6.1 第 3 条；M4 | M6 V-9 A1 | 宿主：批量一轮 + 确认一轮；静态：:150“do not run `detect <url>` on top of them” | 已满足 | — |
| WORK-F3/AC-7 其他调用方/其他目录同样有效性标记 | P1 | §5.5、§5.8 | M2 V-1⑤；M6 A4、A5 | 记录 + 宿主：另一目录与 review-ui/feature-demo 调用都有 `valid`/`invalidReasons` | 已满足 | — |
| WORK-F4/AC-1 同轮派出全新上下文走查；交接只含允许材料 | P1 | §6.1 第 7 条、§6.2；M4 | M5 V-6；M6 V-9 A1、A2 | 宿主：主会话 `4c83…` 第 27 行、`7c047…` 第 195/197 行 | 已满足 | — |
| WORK-F4/AC-2 报告格式：理解、任务、完成度与路径、≤3 条事实/推断/建议，无打分 | P1 | §6.2；M4 | M5 V-6 | 宿主：子会话 `aa66…` 第 165 行 | 已满足 | — |
| WORK-F4/AC-3 可复现卡住/走错进修复批次、修后主代理复验、此前不宣称完工；用户选择带问题交付时列未修问题 | P1（出口判据） | §6.1 第 8 条；M4 | M6 V-9 A2（a2v） | 宿主：walker 报出的滚动/标记问题并入修复批次、主代理复走后交付；植入的无反馈问题被主代理先发现，不能算 walker 发现（M6 已如实记录）。“用户选择带问题交付”分支：静态（:154 末句、evals 17） | 已满足 | — |
| WORK-F4/AC-4 交付原样给处置词、单列走查、走查不改处置词 | P1 | §6.1 第 9 条；M4、M6 | M6 V-9 A2（a2v2） | 宿主：`5f154…` 第 451 行首行 `ship` 原词与单列走查；静态：:162 documenter 段末 | 已满足 | — |
| WORK-F4/AC-5 无法操作页面或不能派隔离子代理→“未完成 UX 走查” | P1 | §6.1 第 7 条、§6.2 第 2 条；M4 | M5 V-6 变体；M6 A3 | 宿主：不可达入口返回“未完成 UX 走查”及原因；“不能派子代理”分支：静态（:154） | 已满足 | — |
| WORK-F4/AC-6 记录隔离标识与证据位置 | P1 | §6.1 第 9 条；M6 修订 | M6 V-9 A2 | 宿主：a2v2 交付含 `.phoenix-ui/review/walk/` 目录标识 | 已满足 | — |
| WORK-F4/AC-7 无主要操作流程不派走查、无走查结论 | P1 | §6.1 第 7 条；M4 | M6 V-9 A6 | 宿主：Codex `01a1c102…` 只派评审与文档代理，第 251/252 行无走查结论 | 已满足 | — |
| WORK-F4/AC-8 其他 skill 派出时交接与格式一致 | P1 | §6.4「走查」；M5 | M6 V-9 A5 | 宿主：A5 R1 交接带任务的缺陷已修，`58ce…` 第 184 行与当前 A5 补跑交接合规 | 已满足 | — |
| WORK-F15/AC-1 检测到 capture 时录制前逐功能核对，报告写明 | P1 | §6.3；M5 | M6 V-9 A4 | 宿主：`85f95…` 4 张有效图，`meta.environment` 写 capture 版本；静态：feature-demo `@precheck`、`@report` | 已满足 | — |
| WORK-F15/AC-2 核对开拦截；拦截记录按写入规则处理 | P1 | §6.3；M5 | M6 V-9 A4 | 宿主：先问后只录无副作用部分 | 已满足 | — |
| WORK-F15/AC-3 失败按报告归因 | P1 | §6.3；M5 | M6 V-9 A4 | 宿主：首录失败归为脚本定位漂移 | 已满足 | — |
| WORK-F15/AC-4 未装/engine 缺失/版本不支持→原流程，不装不下载不阻塞 | P1 | D14、C-1；M5 | M6 V-9 A4b | 宿主：`f6a9cf01…` 无 capture、无安装提示；静态：`@precheck` WHEN 与注释（退出 4、未知命令） | 已满足 | — |
| WORK-F15/AC-5 核对图不替代视频与验收；录制与模板不变 | P1 | §6.3；M5 | M5 V-5；M6 A4 | 静态：`@precheck` MUST_NOT；宿主：视频 23.36 s 正常 | 已满足 | — |
| WORK-F16/AC-1 适用单元用 capture，命名与 `capture.md` 出处合规 | P1 | §6.4「采集」；M5 | M6 V-9 A5、当前 A5 补跑 | 宿主：U1 6 张有效、逐张 `binarySha256` 登记 | 已满足 | — |
| WORK-F16/AC-2 无效图按“没采到” | P1 | §6.4；M5 | M5 V-5 | 静态：`@phoenix_capture`、target-and-capture.md | 已满足 | — |
| WORK-F16/AC-3 机器发现只进机器检查，不进评审材料 | P1 | §6.4；M5 | M6 当前 A5 | 宿主：评审实际只读目标包、`capture.md`、操作记录与图片 | 已满足 | — |
| WORK-F16/AC-4 主 agent 仍逐张开图 | P1 | §6.4；M5 | M6 V-9 A5 | 宿主：开图记录存在 | 已满足 | — |
| WORK-F16/AC-5 未装或不可用→现有方式，规则不变 | P1 | C-1；M5 | M6 V-9 A5b（a5c） | 宿主：`1b2e4f6a…` 报告不提 Phoenix/走查 | 已满足 | — |
| WORK-F16/AC-6 超出命令能力的单元（iframe 等）改用现有方式并记实际工具 | P1 | §6.4；M5 | M6 当前 A5 补跑 | 宿主：U2 用 playwright-core 1.59.1 进入 iframe，`capture.md` 登记实际工具 | 已满足 | — |
| WORK-F17/AC-1 检测到走查子代理时对有主流程单元派出，交接不含禁给材料 | P1 | §6.4「走查」；M5 | M6 V-9 A5 R2、当前 A5 | 宿主：两位 walker 交接无任务或期望 | 已满足 | — |
| WORK-F17/AC-2 报告存本轮目录，格式同 F4/AC-2 | P1 | §6.4；M5 | M6 当前 A5 | 宿主：`walk-U1.md`、`walk-U2.md` | 已满足 | — |
| WORK-F17/AC-3 可复现卡住/走错以「盲测走查」列差距、主 agent 定级；犹豫列建议；不进评审 | P1 | §6.4「差距」；M5 | M6 V-9 A5 | 宿主：UI-06 来源「盲测走查」；静态：report-contract.md :37、:57、:79、:95 | 已满足 | — |
| WORK-F17/AC-4 apply-ui-review 复走同一路径并记录处置 | P1 | §6.5；M5 | M6 V-9 A5/apply | 宿主：`5263…` 第 213/219 行按 walk.json 复走；静态：apply SKILL.md :49、:68，record-contract.md :55 | 已满足 | — |
| WORK-F17/AC-5 无法操作写“未完成 UX 走查”；未装/无主流程不派；计分不变 | P1 | §6.4；M5 | M6 V-9 A5b | 宿主：未装时报告不变；“无法操作”分支：静态（report-contract.md :68） | 已满足 | — |
| WORK-N1/AC-1 临时资料目录、结束删除、不读日常浏览器数据 | P1（终验 P3） | §5.7；M1 | M2 V-2④ | 记录 + CI：子进程 profile 目录已删 | 已满足（P1 部分） | 终验 P3 |
| WORK-N1/AC-2 凭据引用、报告/日志/输出无明文 | P1（终验 P3） | §5.7；M2 | M2 V-1③、V-2③ | 记录：报告/stdout/stderr 无明文，控制台与 URL 中为 `***` | 已满足（P1 部分） | 终验 P3 |
| WORK-N1/AC-3 走查只拿测试凭据、不写真实数据、交接无真实账号 | P1（终验 P3） | §6.1 第 7 条、§6.2；M4 | M5 V-6；M6 V-9 | 宿主：交接只含凭据变量名与安全边界 | 已满足（P1 部分） | 终验 P3 |
| WORK-N1/AC-4 | P3 | — | — | — | 范围外 | P3 计划 |
| WORK-N2/AC-1 四平台无需 Rust、缺 engine 给补装命令 | P1（终验 P4） | D15、§0.5；M6 | M6 V-7、V-8 | CI：四平台 + Node 18 成功；未经 npm 发布 | 已确认变更 | G-03（B）；终验 P4 |
| WORK-N2/AC-2 无浏览器明确报错、其他命令照常；Node ≥18、无 Python | P1（终验 P4） | §5.8、C-8；M6 | M6 V-7、V-8 | CI + 记录：Node 18.20.8 消费者成功；冒烟记录 `browser-unavailable` 分支定义 | 已满足（P1 部分） | 终验 P4 |
| WORK-N2/AC-3、AC-4 | P3、P4 | — | — | — | 范围外 | P3、P4 计划 |
| WORK-N3/AC-1 未用新命令的流程行为不变 | P1（终验 P2） | §0.5、§5.10；M6 | M6 V-7、V-10 | CI + 记录：oracle 其余 golden 逐字节不变，comp-verbs/live/critique 零 diff，xgent_rules 与 impeccable-browser 回归通过 | 已满足（P1 部分） | 终验 P2 |
| WORK-N3/AC-2 只有四个处置词、走查不产生新词 | P1（终验 P2） | C-5、§6.1 第 9 条；M4 | M6 V-10 | 静态：finish-reviewer 处置词段零 diff；new-work.md :156 | 已满足（P1 部分） | 终验 P2 |
| WORK-N3/AC-3 | P2 | — | — | — | 范围外 | P2 计划 |
| WORK-N4/AC-1 发布包第三方声明含 oil-ui MIT 全文与版权行 | P1（终验 P4） | §6.7；M6 | M6 V-7 | 记录：`dist/phoenix-ui/NOTICE.md`、`THIRD-PARTY-NOTICES.md` 含 “Copyright (c) 2026 oil-oil” | 已满足（P1 部分） | 终验 P4 |
| WORK-N4/AC-2 动 skill 前整合清单已更新、diff 逐项核对 | P1（终验 P4） | §0.6；M4–M6 | M5（核对只在对话中列出）；M6 V-10 | 记录：动手前核对的证据只有文档自述；V-10 diff 对照 §0.6 通过 | 已满足（P1 部分） | 终验 P4 |
| WORK-N4/AC-3 | P4 | — | — | — | 范围外 | P4 计划 |
| WORK-F5–F14（P2–P4 功能） | P2–P4 | 计划 §0.5 | — | — | 范围外 | 各期计划 |

UAT 映射：UAT-1、2、3、4、5、6、14、20、21、22、23 的 P1 部分由上表对应 AC 覆盖；UAT-1 中的“下一步”菜单（WORK-F14）与 UAT-15 的 WORK-F11 判据属 P4。

## 4. Triage 与决策账本

本轮没有 C 类差异，没有待决问题，也没有新增决策。沿用的已有决定：[PRD 决定记录](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.decisions.md)（用户 2026-11-09 确认 WORK-F1、F3、F4 修订及 F15–F17 新增），计划 D1–D15。

## 5. 待核实项与审查基线

- 待核实：无。
- 代码基线：`0509ba6c9012f5eb5b6a51507a28d10ad7a66601`（main，已推送并通过 Phoenix run 42808136237）；工作树 dirty，只涉及 `.xgent-ai/` 台账、计划及 M5/M6 记录（已修改），以及 PRD、决定、评审报告（未跟踪）；`tools/phoenix-ui/crates` 与 HEAD 无差异。
- 本次实测：`tools/phoenix-ui/target/debug/phoenix-ui`（2026-11-10 16:49 构建，`binarySha256 0e0148f303cea61b9f98dd5b6be96aee944ef273ca6171cdf880b590bb64c10b`；构建后 `crates/` 未再改动），本机 Chrome，darwin-arm64；输出只在会话临时目录。G-01、G-02 另有源码静态核实，与实测一致。
- 历史证据：V-1–V-5、V-7、V-10 引用完成记录；V-6/V-9 引用 M5/M6 宿主补证及 R4 对原始 JSONL 的核读（证据包只在本机，未提交）；V-8 引用 CI run。A1–A6 用的是早于 F-05 修复的开发候选，本次没有重跑；R4 §3 的适用限制照旧有效。
- 未做：未重跑 test:phoenix、V-9 宿主会话或 CI；未审 P2–P4 及发布。

## 6. Follow-up 与 gaps plan

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| G-01 / — / WORK-F1/AC-3 | 隐藏目标的失败、遮挡与禁用的警告，补上状态文件格式的可见元素定位写法（复用“找不到”时的候选生成；遮挡时给出遮挡物的定位） | 见 G-01 Follow-up；终端照常显示“可改为：”；隐藏仍判失败、遮挡/禁用仍记警告并执行 | 无 | [gaps plan](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md) M1 |
| G-02 / — / WORK-F1 边界、AC-7 | `waitFor` 超时保留 iframe / 开放 shadow root 限制说明 | 见 G-02 Follow-up | 无 | [gaps plan](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md) M1 |
| G-03 / — / WORK-N2/AC-1 | 无（本期） | — | 发布需用户另行授权 | 排除：发布时按计划 §0.5、§12 走 CI 构建并复核 npx 路径 |

gaps plan：[PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md)，生成时状态 **Ready**；实施进度以其「恢复快照」为准。
