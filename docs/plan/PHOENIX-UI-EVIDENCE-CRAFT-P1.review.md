# Phoenix UI 多状态取证与盲测走查 P1 · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-UI-EVIDENCE-CRAFT-P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md) | SHA-256 `694e815e9fabb66e3e5fac21200b64f74f13b7f1d65d402b4eb638d6f945ff78`；未提交 | 全文，P1 全部需求、D1–D15、M1–M6、V-1–V-10 |

评审日期：2026-11-09（Asia/Shanghai）。初次完整评审；关联 PRD、决定记录及 PRD 评审只作需求背景，不是本报告的修订目标。

## 1. 确认问题

共 8 项 P2；无 P0、P1、P3。以下问题都有明确的局部修订方向，默认不构成须等待用户或外部答案的阻塞；实现期沿用对应批次已有检查，不新增验收门。

### F-01 · [P2] 保留初始状态 findings 与 detect 一致的需求合同

- 定位：计划 L264（D6）、L709（§11）；关联 WORK-F1/AC-4、M1–M2。
- 问题：计划明确允许入场动画页面在 capture 等待稳定后少报发现，把初始状态一致性缩为仅对静止页面成立。按该方案实施，即使 V-2 的静止夹具通过，仍不能满足本期 AC-4；在对动效页面取证时，两条命令会给出不同的初始发现集合。
- 证据：[PRD WORK-F1/AC-4](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md#work-f1-按步骤取得多状态界面证据可供其他-skill-调用) L174 无条件要求初始状态发现与单独 detect 一致；[现有扫描链](../../tools/phoenix-ui/crates/browser/src/lib.rs#L355) L355–380 在导航及调用方可选 settle 后抓取首次快照。计划 L478–485 则先等待动画、截图，再扫描。[决定记录](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.decisions.md) 没有授予“仅静止页面”的豁免。
- 修订：在共享扫描与初始状态采集的时序设计中落实同一合同，保留既有 detect 行为约束；删除自行缩窄的静止页面豁免。若确需改变 AC，须显式登记为待确认的需求取舍，不能仅在使用文档写原因后当作覆盖。
- 复核：核对 D6、§5.3/§5.10、V-2 与 §11 对初始扫描语义一致，并仍承接 PRD AC-4。实现验证归现有 V-2/V-7。
- 置信度：高，需求原文与计划的例外直接冲突；阻塞：否，优先按原需求修订设计；若选择改变需求，才需用户确认。

### F-02 · [P2] 走查者需有写取证目录的权限，不能生成 Cursor 只读角色

- 定位：计划 L269（D11）、L295（ADR-2）、L593–596（§6.2）；关联 WORK-F4、WORK-F17、M4。
- 问题：工具声明 `Read, Bash, Glob` 会让现有转换器为 Cursor 生成 `readonly: true`，而同一走查流程必须写状态 JSON，并运行 capture 写截图及报告。该宿主的权限合同会阻止主路线要求的操作，只能落入“未完成 UX 走查”。
- 证据：[factory.js](../../tools/phoenix-ui/scripts/lib/transformers/factory.js#L158) L158–174 明确按工具清单派生只读标记；[Cursor 官方 Subagents 文档](https://cursor.com/docs/subagents) 的 Configuration fields（2026-11-09 查询）说明只读模式禁止文件编辑及改变状态的 shell 命令。计划 L595–596 同时要求写输出目录与运行 capture。
- 修订：为走查角色声明必要的 `Write` 能力，使现有转换器不再生成只读角色；正文继续严格限制只能写交接的取证输出目录，禁止编辑项目源码和其他文件。同步 D11、ADR-2 的 Cursor 只读主张，无需修改公共转换器。
- 复核：核对角色权限、目录边界与 D11/ADR-2 一致；现有 V-5 生成物核查足以承接。
- 置信度：高，转换分支及宿主权限语义均已核实；阻塞：否。

### F-03 · [P2] 允许安全的测试登录重放，避免登录后无法继续盲测探索

- 定位：计划 L300（ADR-2）、L596（操作方式），关联 L477、L530；WORK-F4、M4。
- 问题：“会提交或保存数据的步骤放在最后一次运行，不重复提交”覆盖了登录。走查者先看登录页，下一次提交登录才看到主界面；继续从新页面重放并探索任务列表时，却必须再次提交登录，违反该限制。中途保存后才出现下一步的流程同样受阻，无法完成计划要求的逐步发现主路径。
- 证据：计划 §5.3 每次 capture 启动浏览器，§5.7 复用临时 profile；[Browser::launch](../../tools/phoenix-ui/crates/browser/src/cdp.rs#L131) L131–139 创建新 profile，[Drop](../../tools/phoenix-ui/crates/browser/src/cdp.rs#L314) L314–320 删除它，跨调用不保留登录态。[PRD WORK-F4](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md#work-f4-完工轮派出盲测-ux-走查) L232–239 要求从真实页面探索多步路径；WORK-N1/AC-3 禁止真实业务数据写入，并未禁止安全边界内可重放的测试操作。
- 修订：删除一概“提交只能放最后运行”的限制。明确必要登录及安全边界内可重放的测试操作可以重放，每次只重放当前累积路径；确实不能安全重放的副作用才停止，并如实列为未验证。不必新增保持会话的运行时。
- 复核：ADR-2 与 §6.2 的重放规则应能支持“登录后看见新页面，再选择下一步”；沿用 V-6/V-9 的走查演练。
- 置信度：高，计划自选的临时浏览器生命周期与登录主路径直接冲突；阻塞：否。

### F-04 · [P2] review-ui 命名模板需保留高度、DPR 与补采 attempt 的区分

- 定位：计划 L613（§6.4），关联 L427、L558；WORK-F16/AC-1、M5。
- 问题：固定模板 `<单元>-{state}-{width}-{scheme}.png` 会让同宽不同高度/DPR 的采集，以及同轮补采写入同名文件。分次执行会覆盖原图，按 file 合并的报告也会替换原出处；同次命名冲突则在启动前失败。两种结果都不能遵循既有采集合同。
- 证据：[target-and-capture.md](../../skills/review-ui/references/target-and-capture.md#截图真实性与保存) L106–112 要求同宽不同高度/DPR 加确定性后缀，同轮补采追加 `-a2`、`-a3`，保留原图和出处；计划 L558 按 file 替换报告条目。PRD WORK-F16/AC-1 明确要求命名符合该合同。
- 修订：由 review-ui 调用方按原合同生成视口条件后缀和 attempt 后缀，再传入 `--name`；普通首采仍可使用现有简名，无需扩展 CLI。
- 复核：§6.4 的命名例子与调用责任应同时覆盖条件冲突和补采；既有 V-5/V-9 已承接合同核对，不增加检查组。
- 置信度：高，模板展开与覆盖行为可直接推导；阻塞：否。

### F-05 · [P2] 跨 crate 复用 reveal_sweep 不能只公开到 pub(crate)

- 定位：计划 L364（文件清单）、L481（§5.3 第 5 步）、L570（§5.10）；关联 M1–M2。
- 问题：文件清单要求将 browser 中的函数改为 `pub(crate)`，同时让 CLI 的 capture 复用它执行整页滚动揭示。两者属于不同 crate，该可见性仍禁止调用，照计划会遇到编译错误。
- 证据：[browser/lib.rs](../../tools/phoenix-ui/crates/browser/src/lib.rs#L447) L447 的 `reveal_sweep` 当前为私有函数；[cli/Cargo.toml](../../tools/phoenix-ui/crates/cli/Cargo.toml) 的 package 为 phoenix-ui、lib 为 impeccable，并通过 dependency 引用 impeccable-browser。
- 修订：把这个窄接口明确为跨 crate 可调用的 `pub` 函数，或等价的 Page 公共方法，保持一份实现。
- 复核：文件清单与 §5.10 的接口可见性一致；现有构建与 V-2 覆盖。
- 置信度：高，crate 边界与可见性冲突确定；阻塞：否。

### F-06 · [P2] 序列帧首帧规则不能同时禁止并使用未来帧

- 定位：计划 L516（§5.6 序列帧）、L670（V-4），关联 L478–480；WORK-F2、M3。
- 问题：§5.6 要求每个时刻只选不晚于它的最新帧，V-4 却要求采样时刻早于首个可用帧时取首帧。例如窗口起点为 0ms、首个可用帧为 40ms，按 V-4 实现就会把后来的画面放进标为 +0ms 的格子，失去序列证据的时间含义。
- 证据：计划两处定义互斥；§5.3 仅要求在最后动作前开始录帧，没有要求动作派发前已经收到基准帧。
- 修订：明确先收到动作前基准帧再启动动作窗口，entry 可使用导航前空白基准；或将没有较早帧的格子明确记缺失。统一 V-4，禁止把未来画面标作早期时刻。
- 复核：§5.3 录帧起点、§5.6 采样规则与 V-3/V-4 的时间预期一致；仍由 V-3/V-4 验证。
- 置信度：高，同一算法的设计与预期结果直接冲突；阻塞：否。

### F-07 · [P2] M5 的提前并行范围需排除尚未就绪的走查接入

- 定位：计划 L694–696（M3–M5）、L701（并行）；关联 V-3 的 L669。
- 问题：M5 一方面声明 V-1 后可与 M3 并行，另一方面要求先有 M4 的走查子代理定义；M4 又须等 M3 批末 V-3 通过才能开工。把完整 M5 提前委派，会让实施者在契约未就绪时改走查接入，或被迫重新拆分任务。
- 证据：M3 → V-3 → M4 走查定义 → M5 走查接入的前置链，与 M3/M5 完整并行的描述不能同时成立。
- 修订：将提前并行限定为 M5 的 capture 接入，走查接入等 M4 定义；或者删除提前并行建议。无需新增里程碑或检查。
- 复核：M3/M4/M5 行及并行段对同一产物的解锁条件一致。
- 置信度：高，依赖表内有明确反证；阻塞：否。

### F-08 · [P2] 本地收口应去掉无关的整仓 npm test 门

- 定位：计划 L673（V-7）；关联 L671 的全仓元数据检查、L95–100 的验证节奏。
- 问题：V-7 在 Phoenix UI 原生回归之外，要求整个 `npm test` 全绿，把 Diagram、statusline、dev-plan、agi-mode 等未改动行为纳入本期收口。无关既有失败会阻止 M6 完成，造成额外运行与调查成本；当前计划没有说明这些检查与本期变化的直接关系。
- 证据：[run-tests.mjs](../../scripts/run-tests.mjs#L8) L8–24 收集 test 下全部 JavaScript 测试，L35–36 再运行 dev-plan、全仓技能元数据和 agi-mode 评测；[AGENTS.md](../../AGENTS.md#4-goal-driven-execution) 的 `@verification_scope_and_cost` 明确要求聚合含无关检查时选择相关子命令。相反，[test-phoenix-ui.mjs](../../scripts/test-phoenix-ui.mjs#L27) L27–50 的 UI 构建、安装、共享 CDP、扫描与 oracle 回归与本期存在直接关联，不能仅因命令为聚合而整段删除。
- 修订：用受影响的 Phoenix UI、启动器及构建产物相关文件/子命令替代本地整仓 `npm test` 门；V-5 元数据仅核对四个改动 skill。保留共享浏览器层和 WORK-N3 明确要求的 UI 回归。无关失败只报告，不扩围修复。
- 复核：每条本地批末/收口命令均能对应本期行为或直接依赖；不改 CI workflow 来追求本地检查通过，也不新增替代验收框架。
- 置信度：高，已读取聚合实际步骤；阻塞：否。

## 2. 待核实与待决策

没有新增的、必须由用户或外部提供答案才能修订本计划的阻塞决定。下列均为计划已经登记的实施或收口证据前置，本轮不冒充验证通过。

| 项目 | 尚缺证据及影响 | 提供方 / 最晚确认点 |
| --- | --- | --- |
| A-1：新 CDP 能力 | 本机与四平台浏览器中的角色查询、录帧及子会话拦截实测；影响 M2/M3 能力与四平台终验。官方协议有相应方法定义，不等于各环境运行已通过 | 实施者，按现有 V-2 与 V-8；本机在 M2 批末，平台在 M6 |
| CI 推送与 runner | V-8 需同一候选提交的四平台截图及 Node18 消费者证据；本轮未推送、未触发 CI | 主实施者在 M6 提交具体候选后申请已有计划要求的推送授权；未取得则不得宣称该 AC 通过 |
| 宿主 CLI 会话 | V-6/V-9 需实际登录、必要权限与可加载的 skill/子代理；已安装 CLI 不等于会话可执行 | 实施者，在 M5/M6 对应演练前按计划核对 |

CI 的 UI Native job 自身也执行整仓测试（[phoenix.yml](../../.github/workflows/phoenix.yml#L116) L116–121）。这是既有流水线约束；本报告没有授权修改 workflow。若届时发生无关失败，应与本期 UI 证据分开记录，按仓库规则处理，不扩展本期修复范围。

## 3. 覆盖、基线与证据等级

### 基线

- 仓库：XGENT-ai Skills；分支 `main`；HEAD `4c2d1117f83b7af3292a846aa71a14fe4df8d990`。
- 当前 dirty 工作树参与调查。开始时已有 `.xgent-ai/experience/index.md`、SDLC 入口及分类索引改动；PRD、决定记录、PRD 评审、计划与对应状态页未跟踪。相关 Phoenix 及三个独立 skill 实现源码无工作树差异。
- 需求基线：[PRD v1.2](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)，SHA-256 `09b19f65376834f46480a3dec8394e8ab1a18ba2707f0399da0e9be0caad1e25`；[决定记录](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.decisions.md)，SHA-256 `4cc32092cb792cc33e95be0215cbbf951a9a4c5b4a2849e44cec7f5e4cd9ddc0`。
- 所审计划 hash 见报告头；交付前再次核对计划、需求及相关源码，基线未变。未提交材料当前只能在原工作树取回，hash 不能替代内容同步。

### 实际分工与覆盖

| 维度 | 实际 reviewer | 核查内容与结论 |
| --- | --- | --- |
| facts | `/root/facts` | 核实浏览器生命周期、扫描链、启动器、生成器、许可及 CI；确认 F-01、F-05、F-06 |
| architecture | `/root/architecture` | 核实原生 verb、共享扫描、派生合成与宿主权限；主路线适配现有栈，确认 F-02 |
| schedule | `/root/schedule` | 双向核对 P1 需求、检查与批次，读取聚合检查实际内容；确认 F-07、F-08 |
| delivery | `/root/delivery` | 推演从安装到取证、盲测、修复复验的完整路径；确认 F-03、F-04，并交叉验证 F-02 |
| 综合裁定 | `/root` | 独立回读每项原段与证据，核对 PRD、原生源码、UI 合同与官方权限文档，去重并单写报告 |

四位 reviewer 均只读，没有继续派生 agent。报告和台账仅由主 agent 写入。共用浏览器由主 agent 保留操作权，本轮未启动浏览器。

UI/UX 证据等级为**文档推演**：本期新增 capture 与 walker 尚未实现，没有相应可运行产物可走查。读取 Phoenix 的现有完工、证据有效性和 finish-reviewer 合同，以及 review-ui 的截图与评审隔离合同作为上下文；没有执行设计流程、重新设计产品或声称看图验收通过。入口、错误输出、部分失败、宿主补位、处置词与走查单列的方案均已审查；不因尚未实现而报缺陷。

### 已执行检查

| 改变/待核查行为 | 最小检查与实际结果 | 人工判读 |
| --- | --- | --- |
| 计划路径是否把新增/示例误写为已存在 | `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md /Users/rockie/Documents/GitHub/xgent/skills`：退出 0；47 个路径 token，24 个 MISSING | 逐类核对均为新增产物、运行期输出、workspace/skill/参考源相对路径；未据此确认独立路径缺陷。例如 capture 夹具与 walker 明确新增，`scripts/shoot.mjs` 属本机 oil-ui 参考，`tests/fixtures/xgent/controls.html` 属 tools/phoenix-ui。路径存在不证明接口可调用，F-05 另按 crate 边界确认 |
| 恢复快照、里程碑与记录一致 | `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md /Users/rockie/Documents/GitHub/xgent/skills`：退出 0；0/6，6 行里程碑、0 行实施记录、ERROR 0、WARN 1 | 告警称 dirty 工作树中计划未改；该计划尚未跟踪，且源码无实施差异，0/6 符合当前事实，无虚假完成问题 |
| 新 CDP 方法及宿主权限的外部契约 | 查阅 [CDP 官方 Page 定义](https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/pdl/domains/Page.pdl)、[Fetch 定义](https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/pdl/domains/Fetch.pdl) 与 [Cursor 官方子代理权限](https://cursor.com/docs/subagents)，查询日期 2026-11-09 | 只核对接口/权限定义；CDP 方法定义不代替 A-1 运行证据，Cursor 只读限制用于 F-02 |

未运行构建、Rust/Node/Python 测试套件、V-1–V-10、安装、迁移、种子、部署、真实宿主会话或 CI。没有本轮测试/浏览器通过的声明。报告写后回读、相对链接及所审版本核对属于报告交付检查。

候选“失败重取可能留下旧 shots.valid 条目”没有升级为确认缺陷：§5.8 的“替换本次重取条目”仍容许正确删除旧条目的解释；尚无实现证明采用错误解释。同一运行中新开页面会复用 profile 也不作为隔离缺陷，PRD 仅要求重新打开页面，没有独立 storage 要求。

## 4. 映射与实施编排

### 需求覆盖

| 本期范围 | 设计 / 交付 | 既有验证 | 核对结果 |
| --- | --- | --- | --- |
| WORK-F1/AC-1–9 | §5，M1–M2 | V-1、V-2、V-7 | 有落点；F-01/F-05 需修订设计契约 |
| WORK-F2/AC-1–5 | §5.6，M3 | V-3、V-4 | 有落点；F-06 统一时间语义 |
| WORK-F3/AC-1–7 | §5.5、§6.1，M4 | V-1、V-2、V-9、V-10 | 有落点；comp-led 门禁、截图轮次和四处置词约束保留 |
| WORK-F4/AC-1–8 | §6.1–6.2，M4–M5 | V-6、V-9 | 有落点；F-02/F-03 修复走查可执行性 |
| WORK-F15/AC-1–5 | §6.3，M5 | V-5、V-9 A4/A4b | 可选检测、写拦截、录制独立性均有落点 |
| WORK-F16/AC-1–6 | §6.4，M5 | V-5、V-9 A5/A5b | 有落点；F-04 保留命名与补采合同，iframe 回到原采集方式已安排 |
| WORK-F17/AC-1–5 | §6.4–6.5，M5 | V-5、V-6、V-9 A5/A5b | 差距来源、主 agent 复查、apply 复走及评审隔离均有落点 |
| WORK-N1/AC-1–3 | §5.7、§6.2、§7 | V-1、V-2、V-6、V-9 | 明确要求，保留临时 profile、凭据与测试数据边界 |
| WORK-N2/AC-1–2 | 原生分发/启动器，M6 | V-7、V-8 | 候选与四平台门已安排；运行证据尚未取得 |
| WORK-N3/AC-1–2 | 等价抽取、处置词保留 | V-2、V-7、V-10 | 有直接回归面，不能用 F-08 删掉相关 UI 回归 |
| WORK-N4/AC-1–2 | §0.6、§6.7，M6 | V-7、V-10 | 许可与整合清单在位，未把参考整合当作改变既有原则的授权 |

C-1–C-10 均能追到范围、设计或既有检查；A-1 的证据前置已登记。没有除上述问题外的独立需求断链。跨期 N1→P3、N2/N4→P4、N3→P2 的终验及不得提前宣称上线的边界正确保留；P2–P4 业务不进入本期。

### 依赖、并行与集成

依赖边为：M1 代码 → M2 → V-1/V-2 → M3 → V-3/V-4 → M4 的 walker 定义 → M5 的走查接入 → V-5/V-6 → M6。M5 的 capture 接入只依赖稳定的基础命令，可在 V-1/V-2 后先做；F-07 正是要求把这部分与完整 M5 区分。M6 继续按许可/候选构建 → CI 与宿主使用 → 接口复述收口，必要失败只复验受影响项。

| 波次/任务 | 前置与解锁证据 | 可同时做的工作 | 写入边界 / 共享负责人 | 资源与汇合验收 |
| --- | --- | --- | --- | --- |
| M1–M2 | 无，连续形成真实可运行 capture | 不为并行拆 CDP 共享层 | 主 agent 单人负责 cdp.rs、browser/lib.rs、入口与共享契约 | 本机真实 Chrome；批末 V-1/V-2 |
| M3 + M5 的 capture 接入 | V-1/V-2 已证基础命令可用 | 三个独立 skill 的 capture 文本可与派生图实现并行；走查部分等待 | 主 agent 做 M3；受委派者仅写三个独立 skill；README/docs 留主 agent | M3 批末 V-3/V-4；M5 尚不记完成 |
| M4 + M5 走查接入 | V-3；M4 先给稳定 walker 定义 | 定义稳定后，Phoenix 与独立 skill 文本可并行收敛 | Phoenix 源、render 计数、README/docs 由主 agent；三个独立 skill 由同一受委派者 | 预览目录与演练项目分离；主 agent 集成 V-5/V-6 |
| M6 | M1–M5 交付齐备 | 独立使用场景可隔离目录、端口/profile；无隔离则串行 | 主 agent 负责候选、台账和统一收口 | V-7–V-10，仅一处阶段收口；V-8 推送授权不由本评审代行 |

没有工期数据，不估算节省比例或日历关键路径。现有批次划分、按批末验证、收口单列高成本检查的总体安排可保留，只需 F-07/F-08 的局部修订。

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

建议状态：**Ready（落实本报告修订后）**。已确认问题都有局部修订方向，未发现必须由外部回答才能推进设计的新阻塞，故不因这些缺陷机械判 Blocked。当前原稿仍自报 Ready，尚未落实 F-01–F-08；台账应区分原文自述与“已评审、待处置”，不能据此声称现稿无需修改或已经实施。

原生 CDP、共用扫描、浏览器内派生合成、随 engine 分发，以及独立 skill 检测到才用的主路线可保留。修改依次为：

1. 修 F-01，保证需求合同没有被静止页面例外缩窄；若坚持改变需求，显式登记待确认并重新判断受影响任务的就绪状态。
2. 修 F-02/F-03，令走查权限与登录后的探索路径可执行，保持源码禁改及真实数据边界。
3. 修 F-04/F-05/F-06，统一命名、跨 crate 接口与帧时间语义。
4. 修 F-07/F-08，收敛并行前置与本地验证范围；更新受影响的整合清单/复述后，核对原有映射，无需增加里程碑或验收门。

V-8 的四平台与 V-6/V-9 的真实宿主证据仍按原计划实施时取得；没有这些证据不宣称本期终验完成。Ready 是计划设计就绪建议，不表示功能实现、发布或上线许可。

下一步（仅建议，本轮不启动）：

```text
$apply-doc-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.review.md
```

## 修订处置与复核

- 来源：本报告 §1 的 F-01–F-08，处置前报告 SHA-256 `19d83079b077f1975a49f2f2f32000535cc2cfeb7edbe7d71e2481254292fe69`；原问题 ID、证据、所审基线与 §5 原结论保持不变。
- 目标：[PHOENIX-UI-EVIDENCE-CRAFT-P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md)；修前 SHA-256 `694e815e9fabb66e3e5fac21200b64f74f13b7f1d65d402b4eb638d6f945ff78`（即所审版本，评审后无改动），修后 SHA-256 `395aaef7d76a36302735e6196d4b88e2eb6bf2e9b0df583f2fa724c7bd32501f`。
- 复核日期与代码基线：2026-11-09（Asia/Shanghai）；main / `4c2d1117f83b7af3292a846aa71a14fe4df8d990`，dirty 仅为 `.xgent-ai/` 台账与未跟踪的 PRD/计划/评审文档，`tools`、`skills`、`scripts` 无工作树差异。计划仍未跟踪，只能在本工作树取回。
- 本次范围：全部 F-01–F-08；§2 的 A-1、CI 推送与宿主会话属实施/收口证据前置，本次不处理也不宣称通过。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。PRD WORK-F1/AC-4 无条件要求初始状态发现与 `detect --viewport` 一致；`detect <url>` 单目标为 `networkidle0`、settle 0 后立即扫描（`crates/browser/src/lib.rs` L87、L355–366），原 §5.3 在等动画、截图后才扫描，D6 与 §11 自设“仅静止页面”例外，决定记录无此授权 | 已修订并复核 | D6：初始状态另开扫描页，视口/像素比/媒体/basic-auth/写拦截同截图页，按 detect 单目标时序立即 `scan_loaded_page`；删除静止页面例外。§5.3 第 9–10 步、§5.10 lib.rs 条同步；ADR-1 ① 措辞同步；V-2① 增加入场动画 http 目标；删除 §11 对应风险行。关联补充：发现的 `file` 字段为 detect 结果的一部分（`crates/foundation/src/findings.rs` 的 `finding_for`），本地目标改记 `file://` 地址而非临时服务器地址，V-2① 比较包含该字段。另开页面避免扫描的滚动揭示干扰截图与 entry 序列帧，代价是每视口多一次加载，已写入 D6 | D6、§5.3、§5.10、V-2、§11 对初始扫描语义一致，仍承接 AC-4，无豁免。实现证据归 V-2/V-7 |
| F-02 | 成立。`factory.js` L172–175 在工具清单无 Write/Edit 时生成 `readonly: true`；本次查询 [Cursor Subagents](https://cursor.com/docs/subagents)（2026-11-09）原文：readonly 时 “no file edits, no state-changing shell commands”，走查须写状态文件并运行 capture | 已修订并复核 | D11、ADR-2 ①/②/决策、§0.4 走查边界、§6.2 frontmatter 与正文第 1 条：工具改为 `Read, Write, Bash, Glob`，写入只限交接的输出目录，禁止编辑项目源码和其他文件；ADR-2 ② 改为“授予 Edit 在内的全部工具”。V-5 增加“Cursor 产物不含 `readonly: true`”一项（同一检查组，未新增检查）；公共转换器不改 | 角色权限、目录边界与 D11/ADR-2/§0.4/§6.2 一致；与 PRD“不编辑文件”的约束按“只写取证输出、不编辑项目文件”落实 |
| F-03 | 成立。§5.3 每次 capture 新启浏览器，`Browser::launch` 建新 profile、`Drop` 删除（`cdp.rs` L131–139、L314–320），登录态不跨调用；原规则“会提交或保存数据的步骤放最后一次运行”覆盖登录，阻断登录后的继续探索 | 已修订并复核 | ADR-2 负面后果与 §6.2 第 2 条：每次重放当前累积路径，登录与安全边界内可重复的测试操作可重放；不能安全重复的副作用步骤只作为最后一次运行的最后一步执行一次，此后路径列为未验证。未新增保持会话的运行时 | 规则支持“登录后看到新页面再选下一步”；WORK-N1/AC-3 测试凭据与真实数据边界保留。演练归 V-6/V-9 |
| F-04 | 成立。`skills/review-ui/references/target-and-capture.md` 命名节要求同宽不同高/DPR 加确定性后缀、同轮补采追加 `-a2`/`-a3` 且不覆盖；原 §6.4 固定模板会撞名，§5.8 按 `file` 替换会覆盖原出处 | 已修订并复核 | §6.4「采集」：`--name` 由 review-ui 按命名合同生成后传入，给出首采、条件后缀（`-h{height}-dpr{dpr}`）与 attempt 后缀写法，原图与报告原条目保留；不扩展 CLI | §6.4 同时覆盖条件冲突与补采；V-1⑤ 的 capture 模板用例不受影响。合同核对仍归 V-5/V-9 |
| F-05 | 成立。`reveal_sweep` 当前为私有函数（`crates/browser/src/lib.rs` L447）；`crates/cli/Cargo.toml` 为独立 package，经 `impeccable-browser` 依赖调用，`pub(crate)` 不可跨 crate 访问 | 已修订并复核 | §1.2 文件清单与 §5.10：`reveal_sweep` 改为 `pub fn`，CLI 经 `impeccable_browser::reveal_sweep` 复用同一实现 | 文件清单与 §5.10 可见性一致；编译由现有构建与 V-2 覆盖 |
| F-06 | 成立。原 §5.6 禁止取未来帧，V-4 却要求早于首个可用帧时取首帧，两者互斥 | 已修订并复核 | §5.6 序列帧：开始录帧后先等基准帧（最多 1000ms，计划定值）再派发动作或导航；绝不取未来帧，没有更早帧的格标“无帧”并在 `derived.frames.missing` 计数；entry 基准帧为导航前空白页。§5.3 第 2、4 步与 §5.8 报告示例同步；V-4 改为“早于首个可用帧的时刻标无帧，不用后来的帧补位” | §5.3 录帧起点、§5.6 采样、V-3（首帧空白）与 V-4 的时间语义一致 |
| F-07 | 成立。原 M5 写“V-1 后可与 M3 并行”，同时要求 M4 的走查子代理定义，而 M4 须等 V-3 | 已修订并复核 | M3 行解锁说明、M5 前置依赖、§10 并行段：只有 capture 接入（§6.3、§6.4「采集」）在 V-1、V-2 后与 M3 并行；走查接入（§6.4「走查」「差距」、§6.5 及走查复述）等 M4 定义完成。未新增里程碑或检查 | M3/M4/M5 行与并行段对同一产物的解锁条件一致；feature-demo 预检依赖的写拦截由 V-2 证明，故前置写为 V-1、V-2 |
| F-08 | 成立。`scripts/run-tests.mjs` 收集 `test/` 下全部 JS 测试，并运行 dev-plan、全仓技能元数据与 agi-mode 评测；`test/skills/test_skill_metadata.py` 遍历全部 skill。`scripts/test-phoenix-ui.mjs` L27–50 与本期直接相关 | 已修订并复核 | V-7：以 `node --test 'test/phoenix-ui/*.test.js'` 加 `npm run test:phoenix` 取代整仓 `npm test`，写明无关失败只报告；V-7 标题与 D15 改为“Phoenix UI 全量回归”；V-5 元数据改为只按四个改动 skill 的子测试判定（测试文件无按 skill 过滤参数，沿用原命令，其他 skill 失败只报告）；§9.2 第 3 条改为“依赖生成物的检查（如 render `--check`）预期失败”。§12 引自 hallmark 计划的历史教训保持原文。未改 CI workflow | 每条本地批末/收口命令均对应本期行为或直接依赖；共享浏览器层与 WORK-N3 的 UI 回归保留 |

### 决策与未完项

- 无需用户新决策：八项均按 PRD 原要求或计划内部一致性修订，未改变产品承诺、范围、权限/数据边界或外部契约。
- F-01 采用“另开扫描页”而非“截图页导航后立即扫描”：后者的滚动揭示会改变截图前页面并干扰 entry 序列帧；代价是每个视口多一次页面加载，计划未设耗时 NFR。
- 新的计划定值：F-06 的基准帧等待上限 1000ms。
- 未完项（不阻塞就绪，均为计划已登记的实施/收口证据）：A-1 本机与四平台 CDP 行为（V-2、V-8）；V-8 推送授权与 runner；V-6/V-9 宿主 CLI 登录与权限。CI 的 UI Native job 仍运行整仓测试（`.github/workflows/phoenix.yml` L116–121），本报告未授权修改；届时无关失败应与本期证据分开记录。

### 修订后结论与验证

- 已修订并复核 8 项；已解决无需修改、不采纳、待决各 0 项。复核只覆盖文档层判据，不代表实现、验收或发布通过，也不是一轮新的独立评审。
- 就绪判断：**Ready**。须改项已全部处置，范围、安全、数据与外部契约无未决项；原文 §11 的 Ready 经本次处置后成立。
- 实际检查：`bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md <仓库根>` 退出 0，48 个路径 token、24 个 MISSING，缺失项与原报告 §3 判读的类别相同（新增产物、运行期输出、skill 内相对路径、参考源路径）；`check_progress.sh` 退出 0，进度 0/6、ERROR 0、WARN 1（计划未跟踪且无实施改动，0/6 符合事实）。回读全部修改处，并检索 `Read, Bash`、`pub(crate)`、静止页面例外、“取首帧”、`npm test` 门等旧口径：计划中仅 §12 历史教训仍提到 `npm test` 失配事实，非检查门。
- 未运行：构建、Rust/Node/Python 测试、V-1–V-10、浏览器、宿主会话与 CI；本次只改文档。
- 关联文档：PRD、决定记录与 PRD 评审只读，无需同步修改。
