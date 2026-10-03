# codex-context-goal-guard · 未提交变更代码评审

## 1. 确认问题

未发现可确认的问题。Findings：0。

本次未发现由变更引入、具有具体可达触发条件且值得作者修复的正确性、安全、兼容性或性能缺陷；也未发现本期 M1–M3 的实质需求遗漏。

## 2. 待核实项与证据限制

- 本轮未重新启动真实 Codex 模型回合、daemon 或 TUI，也未重新进行项目信任操作。真实提醒送达、原生暂停、结束调度及信任流程的判断来自对已保存 M1/M3 结构化证据与实现的交叉核查；不能将这些历史运行记为本轮重新执行通过。
- 其他 Codex 构建、操作系统、Python 版本及其他 Stop hook 组合未在本轮验证。当前说明明确限定已实测环境，见[启用说明第 7、11 行](/Users/rockie/Documents/GitHub/xgent/skills/docs/codex-context-goal-guard.md:7)及 [M3 第 20、31–33 行](/Users/rockie/Documents/GitHub/xgent/skills/docs/plan/codex-context-goal-guard.records/M3.md:20)。这些是已声明边界，不作为新增缺陷。
- 新版本或不同配置需要补证时，应在隔离环境重跑计划 V-1、V-4、V-5，核对实际 hook 输入、暂停工具结果、goal 状态及结束事件。现有证据仅支持所捕获结束边界，没有证明无限时间内绝不续跑。

## 3. 覆盖与验证

### 范围与版本

变更意图：新增只读上下文 guard，在当前根线程 active goal 的估算使用率严格超过 65% 时提示 agent 收尾，并交付部署说明与验收证据。此意图由用户提供的计划和本次代码共同确认。

- 仓库：`/Users/rockie/Documents/GitHub/xgent/skills`。
- 评审日期：2026-10-03，Australia/Sydney。
- 原始请求：`$review-code 未提交的代码 docs/plan/codex-context-goal-guard.md`。
- 模式：uncommitted，全部 staged、unstaged、untracked；计划路径作为验收依据，不是文件范围限制。
- 基线：HEAD `c9be5a81f739a9c7588810e36c0efd220731e4fc`；目标为开始评审时固定的工作树快照。index 相对 HEAD 无差异；初始快照有 1 个 tracked 修改文件、10 个 untracked 新文件，无冲突。
- tracked 取证：`git diff --no-ext-diff --no-textconv HEAD --`，另核对 cached/unstaged 层；untracked 按新增文件读取。
- 适用规则：根目录 [AGENTS.md](/Users/rockie/Documents/GitHub/xgent/skills/AGENTS.md)。未发现变更目录下额外 AGENTS.md。
- 单 agent 评审；未委派独立 reviewer。代码、测试、配置和原计划未修改；本报告是唯一新增评审产物，不纳入本轮输入。

以下为评审输入及支持文件的 SHA-256。写报告前复核，所有指纹均未改变。

| 文件 | SHA-256 |
| --- | --- |
| `docs/plan/codex-context-goal-guard.md` | `8a4dd717a02939aec7cf3370010adc05ab2523e16ac9462937d595afff41a57a` |
| `docs/codex-context-goal-guard.md` | `6853885f2ea939d41d915e45d50a9c2cab1721d6e8a9557b7e09f978c1076f92` |
| `hooks/codex/context-goal-guard.py` | `5643e59543bbdf8e25d09098042a57828ddab8f8d97028d97d1391d18e1f1bcf` |
| `hooks/codex/hooks.example.json` | `812be6d4267661c978e549f521d4e472208bc424a0ec78469cf35d375b71e0a3` |
| `test/codex/test_context_goal_guard.py` | `c0f62efb914bb5aacdadefc5ac46d9b26b2e37702f0ad67fd0c65e740f6644ca` |
| `docs/plan/codex-context-goal-guard.records/M1.md` | `d82edcea9be10c56a85b5db41726592fe4b75842eddf383fc526f367c0f7acfc` |
| `docs/plan/codex-context-goal-guard.records/M1.evidence.json` | `23d5d1f6a9c156c0502d1406ce808ab263be86828fb00dc9a8f380a9a942971d` |
| `docs/plan/codex-context-goal-guard.records/M1.probe.py` | `8fb32c8d1ac8bc0bd25c560e3a7431243c1a52f6fd3af798d6e101bb17f4a308` |
| `docs/plan/codex-context-goal-guard.records/M2.md` | `25d220474e6f4f3cb981c62a8e6051f0ae65514932544d5e88d7120c812f9f78` |
| `docs/plan/codex-context-goal-guard.records/M3.md` | `d1012aee3a6112cdcdc9d491d1786cbad03ff2490fa9983f9f2f3b62e96ac930` |
| `docs/plan/codex-context-goal-guard.records/M3.evidence.json` | `d7e26031dbddb814c64c5b67c9a7545d18924f8d8612e5828a68942db24db9e0` |
| `AGENTS.md`（支持规则） | `c14c246dfd13197f0baebbefaabf555abd30603d5a47c166e9c6a02afd1831fb` |
| `package.json`（交付范围支持） | `9210c2b84931d7c79fb696641c87cc9710565121f7e252d221175935d20e031f` |

index 指纹为 `git ls-files --stage -z` 输出的 SHA-256：`d671547da9c620c5363ee4659053ca02dcc555aa503b8c492b496f4cb07b2b21`。本地验证 ref `refs/guard-validation/context-goal-guard` 指向 `63fc5faf56f5ac186211501f065b4d2a96479491`；它是部署证据，未取代本次 HEAD → 工作树的评审基线。

### 已审行为链

| 行为 | 实现与反证核查 |
| --- | --- |
| 输入与线程定位 | 核查脚本第 18–44 行：仅采用指定 transcript，限制在当前 home 的 sessions 内，核对 UUID、meta.id/session_id、格式版本；子 agent 先跳过，fork 使用自己的身份，不采用继承的线程环境变量。M1 的 identity/hook_inputs 支持实测映射。 |
| goal 状态及只读边界 | 核查第 47–56、153–160 行：SQLite `mode=ro`，按当前 thread 参数化读取状态；无记录或非 active 在用量解析前退出。测试核查文件字节、文件树与数据库逻辑值，缺库时不会建库。 |
| 最新统计与失效边界 | 核查第 59–129 行：倒序块读取，丢弃未完成末行；只采用 last_token_usage，拒绝非法统计，不回退越过较新压缩或模型切换标记。读取到有效新模型统计后恢复；同模型下一回合不一律误判为统计失效。 |
| 阈值、提醒和 Stop | 核查第 132–168 行：整数严格比较；PreToolUse 仅追加上下文，未 deny 工具；Stop 首次请求补收尾，续推后仍 active 则告警；提醒保留用户明确授权、无记录、写失败和手动接续分支。 |
| 配置与部署 | 两个同步 handler、3 秒超时、含空格绝对路径；说明要求追加合并、项目与 handler 信任、定义修改后重信，停用保留其他 hooks。实际本地 clone/checkout 成功，四项交付文件逐字节一致。 |
| 验收及进度记录 | 解析两个 evidence JSON，核查身份、用户指令、工具调用/输出、统计、goal 更新、结束状态、信任列表、故障及停用证据；将 M1 探针与 M3 生产脚本分开。M1.probe.py 的证据写入是明确排除于生产部署的测试行为。 |

未进行全仓质量普查、无关安装器/skills 的深度评审或外部发布验证。JSON 证据重点核查上述行为链，没有重新执行原始验收 harness。

结束复核时另出现并发新增的 `docs/diagrams/codex-guard-sequence.json` 与 `docs/diagrams/codex-guard-setup-workflow.json`。已读取以核对是否影响原审查输入；生产脚本、配置、测试、说明和计划指纹均未改变。这两份图示不在初始快照，未纳入本报告的正确性裁定或图示质量验收，后续继续生成的图示产物也不由本报告覆盖。

### 本轮实际执行

环境实测：Python 3.13.7；`codex --version` 返回 `codex-cli 0.160.0`。

| 命令或实验 | 目标版本 | 结果与限制 |
| --- | --- | --- |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s test/codex -p 'test_*.py' -v` | 当前工作树，以上指纹 | 19 项通过，测试运行 3.042 秒；含严格阈值、身份隔离、只读、最新完整行、压缩/模型/窗口失效、数据库故障及 Stop 分支。不是实时调度验收。 |
| 隔离 fixture 经真实脚本 stdin/stdout 追加 4 个边界断言 | 当前脚本 | 全部通过：模型切换后新统计恢复；压缩后新统计再进入同模型下一回合；model_changed/model_context_changed 后无新统计均报告不可用。复用测试夹具，无测试/修复写入仓库。 |
| 临时目录 `git clone --no-hardlinks --no-checkout`，随后 `git checkout --detach 63fc5faf56f5ac186211501f065b4d2a96479491` | 本地验证快照 | checkout clean；脚本、配置、入口测试、启用说明与当前文件逐字节一致。只证明本地获取，不证明远程发布。 |
| `py_compile.compile(..., cfile=<隔离临时目录>/guard.pyc, doraise=True)` | 当前脚本 | 通过；编译产物保存在临时目录，未写入仓库。 |
| `git diff --no-ext-diff --no-textconv --check`、status、index 与输入指纹核查 | 当前工作树 | 通过；index 未改变，评审输入未变化，改动未触及安装器或 package.json。diff --check 本身不检查 untracked 文件。 |

### 已有运行证据与外部契约

[M1.evidence.json](/Users/rockie/Documents/GitHub/xgent/skills/docs/plan/codex-context-goal-guard.records/M1.evidence.json) 的 `threshold_*` 与 `ordinary_direct` 包含两条工具路径的明确用户请求、记录更新、paused 工具结果、Stop 空操作、idle/completed 及结束后 API/只读查询。它用于核查运行契约，不能代替生产脚本验收。

[M3.evidence.json](/Users/rockie/Documents/GitHub/xgent/skills/docs/plan/codex-context-goal-guard.records/M3.evidence.json) 的 `production_sha256`、`clean_checkout.delivery_sha256` 与当前文件一致；`threads.existing/new/write_failure`、`after_end_api`、`readonly_goals` 支持生产链路、无 goal 不触发、手动重建后触发及写失败诚实交接。相关结束事件中 existing/write_failure 各有 1 个 turn/started，new 有用户发起的 2 个，最终均 completed/idle。`hook_lists`、`fault_events`、`disabled_turn` 和 manifest 对照支持已记录的信任、重信、超时、warning、具体 Stop 组合与停用行为。

本轮查阅官方资料作为当前公开语义的辅助证据，具体构建的行为仍以本地证据为准：

- hook 输入含当前 model，子 agent 使用父 session_id；systemMessage 是 UI/事件流告警；Stop block 请求继续，stop_hook_active 表示已被 Stop 续推。这与实现使用方式相符。[官方 Hooks 文档](https://learn.chatgpt.com/docs/hooks)
- direct_only_tool_namespaces 是 code mode 通过直接工具调用使用的 namespace 配置，与测试入口说明相符。[官方配置参考](https://learn.chatgpt.com/docs/config-file/config-reference)
- 更新当前非终止 goal 的状态可保留用量历史；goal 生命周期提供 pause/resume/clear，与本期交接约定相符。[App Server goal 接口](https://learn.chatgpt.com/docs/app-server#manage-a-thread-goal)、[Goals 文档](https://developers.openai.com/cookbook/examples/codex/using_goals_in_codex)

## 4. 计划符合性

计划为[当前工作树版本](/Users/rockie/Documents/GitHub/xgent/skills/docs/plan/codex-context-goal-guard.md)，指纹见上表；本次范围为 M1–M3，需求位于第 58–66 行，验收位于第 253–259 行。

| 需求/里程碑 | 本次实现与验收映射 | 评审状态 |
| --- | --- | --- |
| R-1、NFR-1 / M2 / V-2 | 只对当前根线程 active goal 计算严格 >65%；最新统计、身份和异常边界由入口测试及补充断言验证。 | 已核实 |
| R-2、NFR-2 / M2–M3 / V-3、V-5 | 生产 hook 只读，提示沿用已有记录；M3 中 agent 更新记录，无记录和写失败在回复交接。 | 实现与已有运行证据相符 |
| R-3、A-1 / M1、M3 / V-1、V-5 | 明确用户请求 → agent 保存/交接 → 原生 paused → 状态核对 → completed/idle；M3 的生产脚本 hash 与当前一致。 | 已有运行证据支持；本轮未重跑模型链路 |
| R-4 / M3 / V-5 | 提示及说明要求用户手动新线程/goal/策略；M3 new 第一轮无 goal，第二轮在明确用户请求后手动创建并暂停。 | 实现与已有运行证据相符 |
| NFR-1、NFR-2 / M2–M3 / V-4 | 入口故障、Stop 两阶段由测试验证；实际超时/warning、信任/重信、保留其他 Stop handler 与停用由 M3 证据支持。 | 局部验证通过；集成证据已核查 |
| C-1、C-2 / M2–M3 | 单一生产脚本、标准库、配置片段及说明；未改 npm 安装器或已有 skills。 | 已核实 |
| 本地获取方式调整 / M3 | 从本地验证 ref 提供未发布成果，未宣称远程已有该 SHA；隔离 clone/checkout 本轮复验成功。 | 合理且可验证的变更 |

未发现需要回退完成状态的实质缺口；上述状态不扩大支持版本，也不将 fixture 通过等同真实阈值和调度通过。

## 5. 总体结论与修复顺序

**patch is correct**。

在明确的未提交变更范围及已声明实测环境内，未发现可确认的新增缺陷或本期需求遗漏。入口测试、额外边界实验及本地部署获取验证均通过；已保存的真实运行证据与当前交付文件相符。

没有确认问题需要安排修复。若交付文件、运行构建或 Stop 组合发生变化，应重跑受影响检查及隔离集成验收；成功的可观察条件仍为交接保存/诚实说明、原生 goal paused、状态核对及完成结束判定。本评审结论不构成发布、部署或修改用户日常配置的授权。
