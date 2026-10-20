# Agent Instruction Evolution · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [AGENTS.md 与 Skills 自进化](agent-instruction-evolution.md) | SHA-256 `7c1127111ce01d732ba1bca7c09e95fda1daa90d1a28cf4a0e3e09e0e9e2541e` | 全文，含 R-1–R-5、C-1–C-5、NFR-1–NFR-3、M1–M5、V-1–V-11 |

评审日期：2026-10-15（Asia/Shanghai）。本轮为完整首审，不是修复效果复审。评审对象 ID：`9a6386c1-91c6-4a95-98b4-989e9b806613`。

## 1. 确认问题

### [P2] F-01 · 定义断线后识别、接续原命令的远端恢复接口

- **定位**：计划第 203、237、345、373、413、428–429 行；§2 `remote.py`、§3 Trial、§5.6、NFR-1、V-11、M2/M3。
- **问题**：控制器在 Codex 命令提交后退出、沙箱仍存活时，计划要求根据 `remote_task_id` 查询并接续任务，却未定义该 ID 是沙箱、命令还是独立任务，也未说明新控制器如何取得原命令的运行状态、退出码、补丁和导出阶段。计划选用的 Harbor 公共执行接口并不直接返回持久命令身份。仅知道沙箱存活，不能判定其中的 Codex 是否已完成。实现者仍须重新决定关键恢复控制流；始终返回 `unresolved` 虽能避免盲目重提，却不能兑现已登记任务可接续的 NFR-1。
- **证据**：亲自读取并校验了计划锁定的 [Harbor 0.24.0 发布 wheel](https://files.pythonhosted.org/packages/9e/63/0e3d788e8d1ed116125507c6a01e3fb95cf82be71f2ea7040a1458709d21/harbor-0.24.0-py3-none-any.whl)，SHA-256 为 `23b7ba616a3aae4eff561ced5e2c51c7186f2981e53a770dea9c689d3969877c`。其中 `harbor/environments/modal.py:1356–1387` 等待命令输出和退出，再返回 `ExecResult`；没有把持久命令句柄交给调用者。`harbor/trial/trial.py:492–522` 的恢复、收尾依赖控制器协程的异常处理与 `finally`。官方 [Modal Sandbox 接口](https://modal.com/docs/sdk/py/latest/Sandbox)区分沙箱重连、沙箱状态与命令执行；[ContainerProcess](https://modal.com/docs/sdk/py/latest/container_process)另有命令状态。此证据说明需要恢复桥接，不说明 Modal 无法支持恢复。仓库 [dev-plan](../../skills/dev-plan/SKILL.md)第 144、157、172 行要求关键接口、故障恢复和就绪前提在计划中明确。
- **修订**：在 §2/§3/§5.6 选定一种恢复桥接，定义提交、查询、取消、导出操作的输入输出、持久身份及状态转换。例如重连原沙箱，读取远端 runner 写出的阶段、完成收据和产物清单；明确命令启动、补丁捕获、导出、销毁各阶段由谁推进，以及提交窗口不明时的处理。把所需适配归入 M2/M3，不新增平台或常驻服务。
- **复核**：改稿后能从已保存字段唯一追到原执行，区分沙箱与命令状态，并说明断线后由谁完成产物捕获。实现后的 V-11 覆盖“原命令已启动时直接终止控制器，再以新进程恢复”：不新增模型执行，能识别原命令、取回原结果及补丁、清理原资源；状态确实不可确认时保留 `unresolved`。
- **置信度**：高；计划接口缺口与锁定依赖行为均已读到。**阻塞**：M2/M3 的恢复接口落定及 V-11；不阻塞与此无关的确定性开发。

### [P2] F-02 · 将 M2 验收所需的最小 run 通路归入 M2

- **定位**：计划第 201、405–406、427–429 行；V-3/V-4、M1–M3；关联 R-4、R-5、C-1。
- **问题**：M2 的退出检查 V-3/V-4 均调用 `xgent-evolve run`。M1 只安排 `evolution.py` 的 `validate/快照`，M2 只明确安排 `remote.py` 与烟测配置，而 `evolution.py` 的 `run/resume` 和搜索状态被安排在 M3；M3 又依赖 M2 的稳定远端接口。照现有任务归属实施，M2 没有可执行的验收入口，必须由实施者自行移动后续任务或另造临时入口。
- **证据**：V-3/V-4 的实际命令在第 405–406 行；CLI 由 `evolution.py` 持有见第 201 行；M1–M3 的交付归属见第 427–429 行。已阅读全文，未找到将最小 `run/smoke` 接线明确放入 M2 的补充条款。
- **修订**：M2 显式交付 `evolution.py` 中 smoke 模式的远端 `run` 接线，连接 M1 的冻结输入和 `remote.py`，具备 V-3/V-4 所需的结果、收据和资源清理。M3 保留完整 compare/search 调度、恢复和搜索状态；不必新增里程碑。
- **复核**：改稿后 V-3/V-4 调用链的全部产物由 M1/M2 交付，M2 不依赖 M3 的实现；随后在 M2 实际执行 V-3/V-4，才能记风险门通过。
- **置信度**：高；由计划内的命令与任务归属直接确认。**阻塞**：M2 退出；不阻塞 M1。

## 2. 待核实与实施核对项

以下不计入上述两项确认问题，也不因尚未实施而判失败。

| 项目 | 缺什么证据与影响 | 责任角色 / 最晚确认点 |
| --- | --- | --- |
| 真实远端资源与固定运行组合 | Modal 工作区、限额、模型身份与访问，以及 Harbor/Codex/Modal 组合尚未实跑。计划 §4/§11 已登记，不要求本次评审创建账号或付费运行 | 运行者提供已有授权配置，实施者执行 V-3/V-4；真实调用前及 M2 风险门 |
| 凭证启动与工具可读边界 | Harbor wheel 的 `Codex.run:1704–1723` 会写入或上传认证文件并链接到 Codex 配置目录，`:1767` 使用绕过内部沙箱的执行参数。计划第 294 行已要求工具环境移除凭证，因此不能断言未来实现必然泄漏；但 M2 不能只凭配置文件或环境变量名称收据证明隔离。需核对实际启动环境、工具子进程环境和认证文件可读边界，记录必要的启动适配及无泄密探针结果。[官方非交互认证说明](https://learn.chatgpt.com/docs/non-interactive-mode#authenticate-in-automation)也要求将凭证限制在所需调用范围，并防止不受信代码继承 | 远端适配实施者；V-3 及首次模型执行前。不能满足既定契约时依计划拒绝任务 |
| 统计区间的退化样本 | V-5 已要求少组数正确判定，现有 `minimum_groups` 与 `inconclusive` 约定不足以证明具体算法错误。实现时需明确区间算法及退化分布处理；可用三个独立组的配对差异全部为 +1 作为反例，核对是否会产生缺乏证据支撑的精确区间 | 统计实现者；M3/V-5。无足够证据时保持未定，不在评审中预判涨分 |
| 干净安装、自然请求及真实恢复 | 文档说明和 metadata 不能证明兄弟 skill 发现、真实触发、授权写回、远端超时后恢复和资源销毁。沿用 V-9–V-11 真实验收，不用本次静态检查替代 | 主集成人；M5 阶段收口 |

无远端时的客观事实修正可合理解释为新 skill 按既有编辑授权直接处理；`apply --run` 的未验证分支如何衔接可在实现说明中澄清。本轮不据此推断所有路径断开，也不要求新增命令。

## 3. 覆盖与验证

### 基线与实际分工

- 代码 HEAD：`8e3d2811ffdc4366a38e917e842ff3344c142915`，分支 `main`；事实来自当前工作树。
- 审查起点 dirty：`.xgent-ai/sdlc/index.md`、`.xgent-ai/sdlc/status/plan/index.md` 已修改；目标计划及其状态页 `.xgent-ai/sdlc/status/plan/bbae83cd-7065-4f90-a350-5e6439fdfed6.md` 为未跟踪文件。它们是评审前已有内容，本轮不将其视为实现进展。
- 计划版本见报告头；计划尚未提交，当前原文可取回范围为本工作树，其他机器需先同步原文。评审不修改原计划、代码或实施进度，不提交、推送或发布。
- 实际启动三位只读 reviewer：`facts`、`architecture`、`schedule` 并行；schedule 完成后复用该 reviewer 另审 `delivery`。四个视角均交回结果，主 agent 亲自复核确认问题的计划段落与原始依赖代码，并单写本报告。

| 维度 | 核查结果 | 覆盖限制 |
| --- | --- | --- |
| facts | 仓内 Eval/Hillclimb 的所有权、判定词、证据隔离；npm 分发与测试入口；初始化保留约定；外部固定版本引用未发现独立事实错误 | 运行前提的文档或源码核对不证明远端已验收 |
| architecture | 保留 Harbor/Modal、角色隔离、单写 JSON 状态、探索与确认最佳分离的路线；F-01 补齐真实依赖未直接提供的恢复接口 | 未运行云任务、模型或依赖安装 |
| schedule | 全部 R/C/NFR 均有设计、任务和检查映射；F-02 为验收入口归属断链；外部职责和集中收口安排合理 | 不推算工期，不把外部配置缺失一概判为设计错误 |
| delivery | 安装→授权配置→入口→诊断/候选→远端评价→确认/采用→报告→失败恢复均有文档落点，恢复实现细节由 F-01 收敛 | skill/CLI 主路径仅做文档推演，尚未执行 V-10/V-11 |

UI 不适用：交付为 skill、CLI、文件和报告，没有产品页面；未进行浏览器验收。官方 Codex [AGENTS 加载](https://learn.chatgpt.com/docs/agent-configuration/agents-md)与 [skills 来源](https://learn.chatgpt.com/docs/build-skills)已核对，计划对多级指令、override 和多来源 skills 的处理有依据；当前官方文档不替代固定 Codex 版本的远端探针。

### 实际检查

| 检查 | 结果及解释 |
| --- | --- |
| `bash skills/dev-plan/scripts/check_paths.sh docs/plan/agent-instruction-evolution.md /Users/rockie/Documents/gh-xgent/skills` | 退出 0；只识别到 2 个路径样式 token，1 个 MISSING 为 `docs/project-status/protocol.md`。这是旧台账的兼容路径，当前使用 `.xgent-ai/sdlc/protocol.md`，不构成错误复用；脚本覆盖很窄，另已人工核对确认问题所需路径 |
| `python3 skills/dev-plan/scripts/check_plan.py docs/plan/agent-instruction-evolution.md` | ERROR 0 / WARN 0；只证明结构和引用规则 |
| `bash skills/dev-plan/scripts/check_progress.sh docs/plan/agent-instruction-evolution.md /Users/rockie/Documents/gh-xgent/skills` | 0/5；ERROR 0 / WARN 1。告警为工作树有 4 个未提交改动、计划未被识别为已修改；已核实为未跟踪初稿和已有台账登记，没有已完成里程碑未回写的证据 |
| `npm test` | 退出 0；Node 263 通过、6 跳过，Python 59 通过、2 跳过。跳过项未算通过；本结果不证明新增工具或真实远端能力已验收 |
| 远端 / 模型 / 干净安装 / 恢复验收 | 本次未运行，V-3/V-4/V-9–V-11 保持未来验收项；未运行 Phoenix runtime 验收，本计划也不改该 runtime |

## 4. 映射与实施编排

R-1–R-5、C-1–C-5、NFR-1–NFR-3 已在 §0.1 保留一份设计与检查映射；V-1–V-11 均有实现或验收落点。除 F-01 的恢复接口未收敛、F-02 的 M2 入口归属外，未确认其他需求断链或范围外系统被塞入本方里程碑。既有原则、显式调用策略及授权边界应保留。

| 波次 / 成果 | 前置与解锁证据 | 可并行工作与共享责任 | 汇合验收 |
| --- | --- | --- | --- |
| M1 协议、包与快照 | 无；V-1/V-2 | 协议、共享数据、锁文件与 `evolution.py` 由单一集成人维护；独立协议工作可推进，恢复相关字段须结合 F-01 收敛 | V-1/V-2；包契约与 V-2 解锁远端输入 |
| M2 远端适配与最小 smoke/run | M1 稳定包契约；真实调用另需已授权配置 | 按 F-01 明确远端恢复桥接，按 F-02 补最小 CLI。稳定协议下独立 `compare.py` 的确定性开发可同时进行 | V-3/V-4 为早期独立风险门，必须先于真实搜索 |
| M3 搜索、比较与恢复 | M1 协议及 M2 稳定远端接口 | 统计模块可分工；搜索状态与 `evolution.py` 单写，不能因文件分离绕过接口依赖 | V-5/V-6；完整恢复行为最终由 V-11 收口 |
| M4 采用、新 skill 与接口文档 | M1 授权契约、M3 搜索及报告接口 | apply 与同文件其他改动串行集成；主执行者统合新 skill 和公共接口说明 | V-7 |
| M5 真实闭环与阶段收口 | M1–M4 局部检查；冻结确认集、预算和真实资源 | 主集成人统一执行并记录；共用远端配置和资源须隔离或按有限并发调度 | V-8–V-11；不以 fixture 或 metadata 结项 |

依赖边为：M1 包/V-2→M2；M1 协议与 M2 稳定接口→M3；M1 授权与 M3 搜索/报告→M4；M1–M4 交付及真实配置→M5。M2 风险门只卡真实搜索，不能让无关确定性开发等待全量终验。现有 M5 集中收口和未失效证据复用安排可保留，无需新增里程碑或重复映射表。

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

**建议状态：Blocked；与计划自报 Ready 不一致。** 两项均为 P2，但 F-01 涉及本期关键 NFR-1 的外部执行与恢复契约，不能仅以可以开始 M1 将整份计划判为 Ready。阻塞范围是尚未落定的远端恢复接口及依赖它的交付；独立协议、打包与统计的确定性工作仍可推进。未实跑云账号、额度与组合本身不是新增设计缺陷，继续按计划已有风险门处理。

修订顺序：

1. 先收敛 F-01：明确远端命令身份、查询与产物恢复协议，补对应的控制器进程退出验收场景。
2. 再修 F-02：把 V-3/V-4 必需的最小 smoke/run 接线归入 M2，保持 M3 的完整搜索与恢复职责。
3. 对照差异复核受影响的接口、里程碑和退出条件，重新判定 Ready/Blocked；真实验收记录继续留到各自规定时点，不预填通过。

下一步：

```text
$apply-doc-review docs/plan/agent-instruction-evolution.review.md
```

本报告只给修订建议；未自行启动改稿、实施或后续评审。

## 修订处置与复核

- 来源：本报告原始版本 SHA-256 `da4b7d61471f73bfb993be802444504b77e1fc346c7d2c9ed1829a416fc6961e`；上方首审正文、所审基线与原结论保留，F-ID 不变。
- 目标：[AGENTS.md 与 Skills 自进化](agent-instruction-evolution.md)；修前 SHA-256 `7c1127111ce01d732ba1bca7c09e95fda1daa90d1a28cf4a0e3e09e0e9e2541e`（与所审版本一致）；修后 SHA-256 `38c2c1900d038209cae4220ed38aa4e839155ab54fc53acfd7ee1b5283325bb4`。
- 复核日期与基线：2026-10-15（Asia/Shanghai）；main / `dirty@8e3d2811ffdc4366a38e917e842ff3344c142915`。起点已有计划、报告及两类状态页/索引的未提交改动，按当前内容修订；没有回退原文、修改实现源码或运行云任务。成果尚未提交、推送，仅当前工作树可复现，其他机器需先同步。
- 本次范围：F-01、F-02 全部确认问题，以及 §2 待核实/实施核对项的去向。仅作本轮改稿效果复核，没有发起新的独立评审。

| 问题/报告章节 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。重新下载并校验同一 Harbor 0.24.0 wheel，亲读 modal.py:1328–1387、trial.py:492–522、codex.py:1761–1808：exec 只返回完成结果，控制器侧 finally 还会处理会话/配置。另读并校验 [Modal 1.5.4 wheel](https://files.pythonhosted.org/packages/0a/93/ca23b7b421e2c9738b710eb1d33eb25a88cba32dff864003a692a0e21f8d/modal-1.5.4-py3-none-any.whl)，其 sandbox.py:1223–1316、1868–1912、1991–2008、2313–2319 定义按名/ID 重连、资源状态、exec 和 filesystem；沙箱重连能力不能替代命令结果收据 | 已修订并复核 | §0.2 D5、§0.6、§1–§4、§5.1/§5.6、NFR-1、失败模式、V-3/V-4/V-6/V-11、M1–M3、§11；新增 §2.2 与 RemoteExecution，锁定 Modal 1.5.4，补总时限内的 recovery_window_sec | **已解决（文档层面）**：remote_task_id 为预分配逻辑执行 UUID，sandbox_id 为资源身份；submit/query/cancel/export/cleanup 的输入输出、命令阶段、退出/产物、责任与未决语义已定义。原 runner 负责断线后的捕获，新控制器只接续原执行；V-11 明确 running 后 SIGKILL 控制器、新进程恢复、模型执行数不增加。真实能力仍待 M2/M3 实施和 M5/V-11 |
| F-02 | 成立。修前 §2.1、V-3/V-4 与 M1–M3 的归属确实断开，全文无其他最小入口；修后按 CLI→冻结输入→远端接口→产物/报告→清理逐项核对 | 已修订并复核 | §2.1 run、V-3/V-4、M1–M3、§11；M2 同时交付 evolution.py 的 smoke/run 接线、最小报告与 remote.py/runner，M3 才扩展 compare/search、完整 resume 和搜索状态 | **已解决（文档层面）**：V-3/V-4 所需产物均在 M1/M2，M2 不再依赖 M3；里程碑仍为五个，退出门未删减。是否实际通过 V-3/V-4 仍由 M2 验收记录证明 |

### 决策与未完项

本次采用既定 Harbor/Modal 路线内的按次 runner 收据桥接，没有增加平台、常驻服务、公开 CLI 命令或另一套 verdict。runner 接管原命令监督、会话保全和捕获，恢复身份与状态归 remote.py；控制器继续拥有搜索、导出、独立验收及资源清理。Modal 1.5.4 的固定源码选择和总预算内预留恢复窗口用于落实该契约，未扩大云权限或采用授权。没有需要用户重新裁定的原则、调用策略或范围变化。

以下是实施/验收义务，未计入本次已解决的两项文档问题，也未填为通过：

| 原 §2 项目 | 本次核对与保留去向 | 解除条件 |
| --- | --- | --- |
| 真实远端资源与固定运行组合 | 保留计划 §4/§11、M2 的 V-3/V-4；源码可证明 API 存在，不能证明账号、额度或完整组合可用 | 运行者提供已有授权配置，实施者记录真实远端接入、隔离及校准证据 |
| 凭证启动与工具可读边界 | 重读 Harbor codex.py:1704–1723、1761–1808；不因新增任务用户或控制目录就假定凭证隔离已成立，继续按 §5.1 和 V-3 核对启动、工具子进程环境及认证文件可读性 | 首次模型执行前完成无泄密探针；不能满足既定契约则拒绝任务 |
| 统计区间的退化样本 | 保留 M3/V-5 的少组数与区间核对；实施时纳入三个独立组差异全为 +1 的反例，不在本次静态改稿中预判算法错误或效果通过 | 固定算法及退化处理有可复核依据，证据不足保持 inconclusive |
| 干净安装、自然请求及真实恢复 | 保留 M5/V-9–V-11；V-11 增补原命令运行时直接终止控制器的必要场景 | 实际记录调用、写回、原执行结果取回、重复恢复和原资源销毁，不能用 metadata/fake 替代 |

无远端时的客观事实修正继续按计划 §2.1 apply 与 §5.7 的明确授权及 unverified 标记处理；本次没有另造命令，也没有实施或验证该分支。没有需同步修改的其他 PRD/计划或既有 skill 接口；公共接口说明仍按本计划 M4 的原定清单在实施时交付。

### 修订后结论与验证

- 处置数量：已修订并复核 2 项；已解决无需修改 0 项；不采纳 0 项；尚未解决的确认问题 0 项。上表四项真实实施核对继续有效。
- 当前就绪判定：**Ready**。关键恢复契约与 M2 入口归属已明确，原首审建议 Blocked 的文档原因已解除；云资源与实际兼容性仍按规定的最早风险门处理。本结论只表示计划可实施，不表示独立复审通过、远端能力可用或整期已完成。
- 实施状态仍为 **0/5、尚未开始**。实施者协议、恢复快照/完成记录、需求账本、顶层排除和新 skill 的调用/授权策略与修前逐字一致，没有新增完成记录。

| 本次实际检查 | 结果与适用范围 |
| --- | --- |
| Harbor/Modal 固定发布源码核对 | 两个 wheel 均校验 SHA-256；Harbor 与原审证据相同，Modal 为计划 §4 所列 `3e54e26037c445af42f9a9ef9862b66bdd2e0b1faeced5fcc7adf3e5f59e44ed`。仅下载读取源码，没有安装依赖、创建资源或调用模型 |
| `python3 skills/dev-plan/scripts/check_plan.py docs/plan/agent-instruction-evolution.md` | 退出 0，ERROR 0 / WARN 0；只证明计划结构与引用规则 |
| `bash skills/dev-plan/scripts/check_paths.sh docs/plan/agent-instruction-evolution.md /Users/rockie/Documents/gh-xgent/skills` | 退出 0；2 个路径样式 token，MISSING 1 个为旧台账兼容路径 `docs/project-status/protocol.md`。现用 `.xgent-ai/sdlc/protocol.md` 已读，非错误复用；另人工核对恢复设计、M1/M2 调用链及新增路径标记 |
| `bash skills/dev-plan/scripts/check_progress.sh docs/plan/agent-instruction-evolution.md /Users/rockie/Documents/gh-xgent/skills` | 退出 0，0/5、ERROR 0 / WARN 1；提示工作树 7 个未提交改动而计划未被识别为已修改。计划为未跟踪文件，已对修前快照确认正文变化及实施进度未变，没有实施完成未回写 |
| `npm test` | 退出 0；Node 263 通过、6 跳过，Python 59 通过、2 跳过。跳过不算通过；不证明拟新增的工具、runner 或远端能力已实现 |
| 范围、进度与依赖核对 | 修前快照对比通过；M1/M2 交付 smoke 所需冻结输入、远端适配、报告和清理，M3 负责比较/搜索及完整恢复。工具、新 skill 和 test/evolution 实施目录仍未创建 |
| 远端/模型/干净安装/故障恢复及 Phoenix runtime | 未运行；V-3/V-4/V-9–V-11 是未来实施验收。此次未修改 Phoenix runtime |

下一步（仅建议，不自动执行）：

```text
/goal 按 docs/plan/agent-instruction-evolution.md 开发
```
