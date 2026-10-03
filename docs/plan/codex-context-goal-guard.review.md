# codex-context-goal-guard 开发计划评审报告

- 评审对象：[docs/plan/codex-context-goal-guard.md](codex-context-goal-guard.md)（untracked 新文件，274 行，自报状态 **Blocked**）
- 评审基线：2026-10-03 · commit `ccbfd7396cbae349d4027d001c0978aa66b671f3` · 工作树 dirty（11 个 untracked 文件，含本计划；均与计划改动面无关）· 本机 codex-cli 0.160.0
- 评审方式：主 agent + 4 个独立只读 reviewer（facts / architecture / schedule / delivery）并行初审，主 agent 交叉复核并亲自核验关键证据（`update_goal` 契约二进制原文、官方 Hooks 文档信任机制）；官方三处引用文档均实际抓取核实
- 建议状态：**Blocked**（与计划自报一致；新增 1 条 P1 须在 M1 开工前修订）

## 1. 确认问题

1. [P1] F-01 · §4 对 `update_goal` 工具契约的「已核实」遗漏了禁止 agent 主动暂停的限制条款，且 V-1 缺少对「agent 转而请求用户确认」中间态的判定口径
   - 定位：`docs/plan/codex-context-goal-guard.md:148`（§4「agent 的原生 goal 工具 | 已核实·当前工具定义」）；关联 `:75`（§0.2 暂停执行者依据「当前 `update_goal` 工具契约」）、`:184`（§5.3 第 3 步指示 agent 设 `paused`）、`:236`（V-1 判据）；关联 R-3、A-1、M1
   - 问题：计划把「当前工具定义」作为「hook 触发后由 agent 暂停 goal」的已核实依据，但 0.160.0 的 `update_goal` 定义明确限制：paused 仅在用户显式请求时设置、绝不主动暂停、不清楚就询问用户。hook 注入的提醒不显然构成「用户显式请求」，计划全文未记录该限制；§7 启用告知（`:211`）能否构成 standing instruction 未作分析；V-1 只有「暂停成功/续跑停止」二元判据，没有为契约最可能引导出的「agent 转而向用户确认」中间态规定归类。实施者可能把契约性拒绝误读为环境故障，或把「agent 询问了用户」记成 V-1 通过——这直接决定 R-3 是否可达与 A-1 是否解除。
   - 证据：本机 codex-cli 0.160.0 原生二进制（`@openai/codex-darwin-arm64` vendor `aarch64-apple-darwin/bin/codex`）strings（2026-10-03 实测）：「Set status to `paused` only at the user's explicit request to pause this goal, never on your own initiative. Ask if unclear; a later resume revokes that request. Report the returned status and stop goal work. Budget limits take precedence over pausing.」以及「update_goal can only mark the existing goal complete, blocked, or paused at the user's explicit request; resume, budget-limited, and usage-limited status changes are controlled by the user or system」；会话记录 `.codex/sessions/2026/10/03/rollout-2026-10-03T00-00-00-01a0fcea-2b9a-7f80-83e9-4d93dd1818f9.jsonl:17` 工具清单含同一描述（记录在该句末尾截断）；[Using Goals in Codex](https://developers.openai.com/cookbook/examples/codex/using_goals_in_codex)（2026-10-03 抓取）：「Pausing, resuming, clearing … remain controlled by the user or the system」
   - 修订：§4 该行如实补引限制条款（注明本地记录截断、以 M1 抓取的完整定义为准）；§0.2「暂停执行者」依据栏同步更正；V-1 增加判定分类——① agent 在启用告知前提下直接暂停、② agent 转而请求用户确认（建议计入暂停失败分支，走 §8 用户 `/goal pause`，不算 A-1 解除）、③ 拒绝；「提示引用用户预设 65% 策略是否构成显式请求」由 M1 实测判定，不在计划中预设结论
   - 复核：改稿后检查 §4 / §0.2 / V-1 三处引用该限制且 V-1 枚举中间态归类；实现后以 M1 实测记录中 agent 的实际行为分类复核
   - 置信度：高（二进制 + 会话记录 + 官方文档三源一致）；阻塞：M1（V-1 判据须在执行前修订；不改变整体 Blocked 结论，A-1 本就阻塞）

2. [P2] F-02 · 用户获得与脚本部署位置未定义，`hooks.example.json` 的「安装后脚本的路径」没有所指
   - 定位：`docs/plan/codex-context-goal-guard.md:116`、`:120`、`:226`；关联 C-1、M3
   - 问题：用户启用 guard 须先取得 `context-goal-guard.py` 并让 hooks 配置 command 指向它。计划排除安装器（`:78`、`:120`），却写示例配置「使用安装后脚本的路径」——「安装」动作本期不存在；脚本落点（skills 仓库 clone 内 vs 目标项目 `.codex/` 下）、command 路径形态（绝对/相对）全文无决策。官方文档对 repo-local hook 推荐 `$(git rev-parse --show-toplevel)` 形式，但该脚本不在目标仓库内，此模式不能直接套用。目标用户在旅程第一步「获得/安装」即无法照做，M3 启用走查只能由实现者临场发挥。
   - 证据：计划 `:116`/`:120`/`:226` 三处陈述无法同时落地的文本对照；[Hooks 文档](https://learn.chatgpt.com/docs/hooks)（2026-10-03 抓取）：「For repo-local hooks, prefer resolving from the git root instead of using a relative path」
   - 修订：在 §1.2 或 §8 增补部署约定——获取方式（clone 本仓库或复制单文件）、推荐脚本落点（如目标项目 `.codex/hooks/` 或固定用户目录）、`hooks.example.json` 中 command 的路径形态与空格引用示例（呼应 `:134`）
   - 复核：改稿后检查示例路径无需读者自行发明即可解析；实现后 M3 在干净目录按文档完整走一遍启用
   - 置信度：高（计划文本可直接核对）；阻塞：M3（启用与说明文档）

3. [P2] F-03 · 启用说明缺「双层信任 + 改后重信 + 确认生效」三步，存在 guard 静默未生效风险
   - 定位：`docs/plan/codex-context-goal-guard.md:211`（§7 信任）、`:118`（文档主题清单仅「hook 信任」）、`:226`（启用顺序）、`:216-224`（§8 失败表）
   - 问题：官方机制有两道门：项目 `.codex/` 层须受信任，项目内 hook 才会加载；每个非托管 hook 按定义 hash 单独审阅信任，**任何对 hooks 定义的编辑（如修正路径）都使信任失效、需重新审阅**（`/hooks` 操作）。计划只写「按原生信任机制显式启用」，文档主题清单无对应展开，也无「确认 guard 已 armed」的验收步骤；§8 失败表没有「配置合并后 hook 未加载/未信任」行。用户合并配置后若项目层未信任或编辑后未重信，guard 不加载，保护缺失——启动时对「待审 hook」有警告，但项目层未信任时的提示形态文档未明确，存在静默可能。
   - 证据：[Hooks 文档](https://learn.chatgpt.com/docs/hooks)（2026-10-03 抓取）：「Project-local hooks load only when the project .codex/ layer is trusted」「Codex records trust against the hook's current hash, so new or changed hooks are marked for review and skipped until trusted」「Use /hooks in the CLI to inspect hook sources, review new or changed hooks, trust hooks」「If hooks need review at startup, Codex prints a warning that tells you to open /hooks」
   - 修订：§7/§8 启用顺序与 `:118` 文档主题补：项目层信任前置、`/hooks` 审阅操作、编辑配置后须重新信任的提示、「确认 hook 已加载生效」步骤（如 `/hooks` 查看状态）；§8 失败表加「配置合并后 hook 未加载/未信任」行
   - 复核：改稿后核对启用说明能否让读者在无内部知识下确认 guard 生效；M3 实测包含「未信任 → 信任 → 生效」走查
   - 置信度：中（两道门文档明确；项目层未信任是否完全静默待实测）；阻塞：否，M3 前修订

4. [P3] F-04 · M1 退出条件的就地更新清单遗漏 §0.1 账本 A-1 行
   - 定位：`docs/plan/codex-context-goal-guard.md:250`（M1 退出条件「A-1 解除后就地更新 §4、§11 和计划状态」）对照 `:64`（§0.1 A-1 行状态「阻塞」）
   - 问题：A-1 解除后若严格按清单执行，§0.1 账本将残留「阻塞」，与 §11/计划状态形成两处冲突状态（通用协议「就地修订对应正文」隐含覆盖，但显式清单漏项）。
   - 证据：计划内两处定位直接对照；`AGENTS.md` §5「Never leave two conflicting versions」
   - 修订：M1 退出条件更新清单加入「§0.1」（或改写为「A-1 相关状态处」）
   - 复核：改稿后检查 M1 退出条件枚举覆盖 §0.1；或实施后核对 M1 记录中账本状态已同步
   - 置信度：高；阻塞：否

5. [P3] F-05 · M1 前置「隔离 Codex 测试环境」无准备责任与时点
   - 定位：`docs/plan/codex-context-goal-guard.md:250`（M1 前置依赖「本计划与隔离 Codex 测试环境」）对照 `:236`（V-1 需「隔离 Codex 测试线程、测试任务与已有测试进度文件」）及 `:40`（快照下一步已预设该环境）
   - 问题：环境被声明为 M1 前置依赖，但全文无句子说明由谁、何时搭建（隔离线程、测试 goal、已有进度文件均非 §1.2 交付物），构成隐藏的资源前置。本地 `thread_goals` 表当前 0 行、`.codex` 无任何 hooks 配置，M1 是从零起步，无既有数据可借用。
   - 证据：上述三处定位；只读查询 `.codex/goals_1.sqlite`（`mode=ro`）：`thread_goals` 0 行；`.codex/` 无 `hooks.json`、`config.toml` 无 `[hooks]`
   - 修订：在 M1「内容与并行边界」或前置列补一句：隔离测试线程、测试任务与进度文件由实现者在 M1 起步搭建，作为 V-1 运行环境
   - 复核：改稿后检查 M1 行包含环境搭建责任；或实施后以 M1 记录中的环境搭建步骤为证
   - 置信度：高；阻塞：否

6. [P3] F-06 · 新增 `tests/codex/` 与仓库既有 `test/`（单数）目录约定不一致
   - 定位：`docs/plan/codex-context-goal-guard.md:117`、`:237`（V-2 命令 `python3 -m unittest discover -s tests/codex …`）
   - 问题：仓库现有唯一测试目录为 `test/`（`test/xgent-skills.test.js`），计划新创顶层 `tests/`。Python 与 Node 测试分目录本身合理，但两个测试根目录并存未见选择理由。
   - 证据：`test/xgent-skills.test.js` 存在；`tests/` 不存在；`package.json` 无 test script
   - 修订：§1.2 注明新增 `tests/` 是有意与 Node 的 `test/` 区分，或改用 `test/codex/`；二选一
   - 复核：改稿后检查 §1.2 与 §9 V-2 路径一致
   - 置信度：高；阻塞：否

7. [P3] F-07 · 无既有文档分支未说明用户需把交接摘要带入新对话
   - 定位：`docs/plan/codex-context-goal-guard.md:203-205`（§6 示例与无文档替换规则）；关联 R-4、V-5
   - 问题：无既有记录时交接只存在于旧对话最终回复。示例文案让用户「提供计划路径并说明继续执行」，但没有计划路径可提供的场景下，用户在新对话只说「继续执行」，新 agent 无任何上下文。`:205` 只规定了 agent 回复措辞替换，未把给用户的接续指令变为「新开对话并粘贴以上交接摘要」。
   - 证据：计划 §6 全文（文档推演，尚无实现可跑）
   - 修订：§6 补一句无文档分支的用户指引示例
   - 复核：改稿后检查两条分支（有计划/无计划）各有明确用户动作；V-5 故障走查覆盖无文档场景
   - 置信度：高；阻塞：否

8. [P3] F-08 · 未告知用户：新对话无 active goal，guard 在新对话中静默失效，旧 goal 保持 paused 无处置指引
   - 定位：`docs/plan/codex-context-goal-guard.md:59`（R-4）、`:103`（`thread_goals` 以 `thread_id` 为主键）、`:131`（无记录视为无 goal）、`:203`（§6 接续文案）、`:118`（「人工接续」主题）
   - 问题：goal 归属线程，新对话即新线程，hook 查不到 goal 记录 → 直接空操作。用户按 §6 指引新开后 65% 保护不再存在且无人告知；旧线程的 paused goal 如何处置（原对话 resume 还是弃置、新对话是否重建 goal）也无指引。R-4 排除自动继承（用户已确认），但不排斥在文案/文档中告知后果与手动选项。
   - 证据：由计划自身事实（`:103`/`:131`）推导；goal 归属线程见计划 `:107` 所引 Goals 文档
   - 修订：§6 最终回复模板与「人工接续」文档主题中补一句后果告知与手动选项
   - 复核：改稿后核对文案明确「新对话不受 guard 保护，除非重建 goal」
   - 置信度：高（由计划自身事实推导）；阻塞：否

9. [P3] F-09 · 「检测不可用」告警未规定给出用户下一步
   - 定位：`docs/plan/codex-context-goal-guard.md:152`（「以简短 `systemMessage` 告知检测不可用」）、`:218-219`（§8 失败表前两行）
   - 问题：格式不兼容或 SQLite 异常时用户每次工具调用都可能看到告警，但告警规格只要求「告知检测不可用」，不含指向说明文档或停用方式的下一步；失败表「恢复与验证」列写的是 V-2/V-4（实现者入口），不是用户动作。目标操作者看到反复告警不知如何处置。
   - 证据：计划 `:152`/`:218-219`；[Hooks 文档](https://learn.chatgpt.com/docs/hooks)（2026-10-03 抓取）确认 `systemMessage` 会作为警告 surfaced
   - 修订：§4 或 §7 规定告警文案须含一个下一步（如「见 docs/codex-context-goal-guard.md 故障说明或按说明停用」）；失败表恢复列补用户侧动作
   - 复核：改稿后检查告警规格含指向；实现后 V-4 走查确认告警可见且可行动
   - 置信度：中（告警具体措辞待实现，「故障说明」主题可能覆盖，但计划未建立两者关联）；阻塞：否

## 2. 待核实 / 待决策

| 待核实项 | 缺什么证据 | 影响 | 谁可提供 | 最晚确认点 |
| --- | --- | --- | --- | --- |
| `update_goal` 完整定义与参数 schema | 本地会话记录在限制条款末尾截断（「…revokes that r」），二进制 strings 补全了描述句但无 parameters | F-01 的最终措辞（不影响其成立） | 实现者 M1 抓取完整工具定义，或对照 openai/codex 0.160.0 源码 | M1 |
| Stop 空输出接受度 | 文档张力：「Stop expects JSON on stdout when it exits 0」vs 通用「Exit 0 with no output is treated as success」；§2「允许的空输出」依赖后者 | §5.4 空操作分支实现细节 | 实现者 M1 实测 | M1 |
| 项目层未信任时的提示形态 | 官方文档只明确「待审 hook 启动时有警告」，项目层未信任是否静默未说明 | F-03 的静默程度定级 | 实现者 M1/M3 实测 | M3 |
| `CODEX_HOME` 进程继承 | 评审 shell 非 Codex 进程变量为空；`.codex/` 为活跃 home（WAL/sessions 今日写入）仅间接证据 | §2 环境解析设计前提 | 实现者 M1 在真实 hook 进程内验证 | M1 |
| A-1 全链路（提醒送达、线程定位、暂停、停止续跑） | 仓库内无任何已验证证据；本机 `.codex` 无 `hooks.json`、`config.toml` 无 `[hooks]`，从未启用过任何 hook，M1 从零起步 | 计划自报 Blocked 的核心 | 实现者 V-1 | M1（M2 前） |

## 3. 覆盖摘要

- **基线**：计划 `docs/plan/codex-context-goal-guard.md`（评审期间 md5 未变，无并发修改）；commit `ccbfd73`；dirty。计划 L7 称调查时「clean」：其余 untracked 文件 mtime 均晚于计划文件创建时间，该声明倾向成立；计划文件自身 untracked 属写作产物，不计缺陷。
- **agent 分工**：facts / architecture / schedule / delivery 各 1 名独立只读 reviewer 并行初审，互不持有对方结论；主 agent 完成需求账本核对、跨视角追问（`thread_goals` 空表 → M1 环境搭建缺口；无 hooks 配置史 → F-03 佐证；`/goal pause` 存在性 → 已由 cookbook 证实）与全部 findings 裁定。
- **四维覆盖**：facts（安装器符号、JSONL 字段、SQLite schema、版本、外部 URL、内部一致性全核）；architecture（10 项关键选择逐项核实，9 项成立）；schedule（双向映射无断链、依赖图无循环、骨架协议逐字比对一致）；delivery（用户旅程 9 步逐步走查，UI 不适用——本期无界面交付）。
- **证据等级**：本地实证（JSONL/SQLite/二进制 strings）+ 官方文档抓取（2026-10-03）+ 文档推演。交付物尚未实现，无浏览器/终端实测；交互文案（§5.3/§6）结论属文档推演，待 M3 实测验收。
- **脚本**：`check_paths.sh` exit 0——4 个 MISSING 全部为 §1.2 ★ 计划新增路径，无虚构「复用/已核实」路径；`check_progress.sh` exit 0（0 ERROR / 1 WARN）——WARN 为工作树存在与计划无关的 untracked 文件，计划的进度记账本身一致（0/3、无预填完成）。

## 4. 映射与实施编排

**映射完整**：schedule 视角双向追踪 R-1..R-4 / NFR-1..2 / C-1..2 / A-1 → §0.2 决策与正文落点 → §1.2 交付物 → M1-M3 → V-1..V-5，无断链、无孤儿验收；§0.1 账本即完整映射入口（新版 dev-plan 允许，省 §13 不构成缺陷）。

**依赖边**（无循环、无倒置；风险门多防守点无绕过路径）：

| 波次/任务 | 前置及解锁证据 | 可同时做的工作 | 写入边界/共享负责人 | 运行资源约束 | 汇合验收 |
| --- | --- | --- | --- | --- | --- |
| M1 核实契约 | 本计划；隔离测试环境由实现者 M1 起步自建（F-05 待写明） | 无 | 最小探针 + 计划回写，单实现者 | 隔离 Codex 线程独占、串行 | V-1（须先按 F-01 修订判据） |
| M2 检测与提示实现 | M1 定位/暂停契约 + V-1 证据 | 无（身份规则依赖 M1 结果，提前开发必返工） | `hooks/codex/`、`tests/codex/` 单负责人 | 本地 unittest | V-2、V-3、V-4 脚本部分 |
| M3 启用与接续验收 | M2 交付物 + 脚本检查通过 | 无（说明文档内容依赖实测结果） | `docs/codex-context-goal-guard.md` + 隔离项目配置 | 真实 Codex 资源串行 | V-4 混合配置 + V-5 |
| 启用（非里程碑） | A-1 解除 + M3 通过 | — | 用户按说明操作 | — | §8 启用顺序 |

原计划「串行实现，不为并行额外拆 agent」（L254）结论成立，无安全并行空间；确认沿用原编排，仅 F-04/F-05 两处 M1 行文字需补。

## 5. 总体结论与修订顺序

- **建议状态：Blocked**——与计划自报一致。技术路线的主要主张（hook 事件与输出语义、同步/超时机制、信任机制、SQLite 直连只读、Python 3 stdlib 选择、用量口径、A-1 风险门设计）经官方文档与本地 0.160.0 实证核实**成立**；评审未发现颠覆方案的问题。
- 新增的 P1（F-01）强化而非改变 Blocked 结论：它揭示「agent 按 hook 提醒暂停」与 `update_goal` 契约的显式请求条款之间存在计划未披露的冲突，A-1 仍是解除启用的唯一门，且 V-1 的判定口径必须先修订，否则 M1 的结论不可信。
- **修订顺序**：① F-01（契约前提与 V-1 判据，M1 开工前）；② F-04、F-05（M1 退出条件与前置责任，M1 开工前顺手）；③ F-02、F-03（部署约定与启用信任说明，M2 期间至 M3 前）；④ F-06～F-09（P3，随相关章节顺手修订）。
- **解除阻塞的可观察条件**：M1 按修订后 V-1 判据实测通过（根线程定位、普通工具与 code mode 提示送达、agent 原生暂停——含「转而请求确认」的明确分类、goal 续跑停止、用量历史不重置）→ A-1 解除，就地更新 §0.1/§4/§11 与计划状态 → Ready。若实测落入「agent 依契约拒绝主动暂停」，按 §5.4 失败分支重新设计控制路径，不得降级为「提醒过了就算暂停」。
- 本评审通过仅表示计划文本层面可继续推进 M1；计划评审通过不代表功能已实现或可用。

## 6. 修订处置与复核

- 来源：本报告修前 SHA-256 `07a84ae77a3c2c82866fd2491a4df3237e66a82a8a2fb9a51d26e1b7263f0479`；上述原问题、证据、结论及所审基线原样保留。
- 目标：[codex-context-goal-guard.md](codex-context-goal-guard.md)；修前 SHA-256 `67db62bee640af86fe81fabea541a555e7d0d65796c54fdf6aae02882dbc9559`，修后 SHA-256 `9dc48c5b6553a440d826c4aa76426504981140f98ba0218f2d88edd94b54517e`。
- 复核日期与代码基线：2026-10-03（Australia/Sydney）；commit `09f7f923f87b13890f11b700d4a043969cbfc5ad`；开始时工作树 clean，交付时仅目标计划及本报告 dirty。报告原审 commit 与当前 commit 不同，但两份输入的修前内容均与当前 HEAD 一致，无已有未提交修改或处置节。
- 本次范围：F-01～F-09、§2 全部 5 项待核实项及受影响的需求/设计/任务/验收映射。只修订文档并只读核实事实，未执行 M1～M3，未写代码、启用 hook 或修改 goal。以下“已修订并复核”仅指文档缺口已消除，不代表运行验收通过，也不是重跑完整独立评审。

### 逐项处置

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。亲读原 §0.2/§4/§5.3/V-1；本机 0.160.0 二进制 strings 与当前会话工具定义均限制仅按用户显式请求暂停、存疑询问、resume 撤销原请求；原会话记录第 17 行确在撤销句末截断。[Goals 文档](https://developers.openai.com/cookbook/examples/codex/using_goals_in_codex)的生命周期权限说明亦一致 | 已修订并复核 | §0.2、§4、§5.3/§5.4、§7、§8、V-1、M1、§11/§12 | 三处要求位置均写明限制；V-1 区分直接暂停、转而确认、拒绝/调用失败。后两类及用户手动兜底均不解除 A-1；目标运行完整定义/schema 与预设策略的有效性仍由 M1 实测 |
| F-02 | 成立。原 §1.2 只有未定义的“安装后”路径；[package.json](../../package.json)的 repository 字段提供获取来源，files 列表未包含拟新增 hooks；独立手动部署符合 C-1 | 已修订并复核 | §1.2、§8、V-5、M3 | 固定 clone/已验证版本 → 单文件复制到目标 `.codex/hooks/` → 双引号包围绝对脚本路径 → 合并配置；JSON 解码及命令分词确认空格路径是单个参数。真实部署及子目录启动留给 M3 |
| F-03 | 成立。[Hooks 文档](https://learn.chatgpt.com/docs/hooks)的加载与信任章节明确项目层和 hook 定义 hash 两道信任门；未信任项目是否完全静默仍未知 | 已修订并复核 | §1.2 文档主题、§7、§8 启用/失败表、V-4、M3 | 已补项目层信任、`/hooks` 审阅、改后重信、来源/启用/信任状态确认；M3 覆盖未信任 → 信任生效 → 定义修改后重信，未把静默程度写成已证事实 |
| F-04 | 成立。原 M1 更新清单未列 §0.1，但该处 A-1 明确为阻塞 | 已修订并复核 | M1 退出条件；§11 就绪条件 | 更新清单包含 §0.1、§4、§11 和计划状态；仍保留 A-1 阻塞，不提前填完成 |
| F-05 | 成立。原 M1 与 V-1 引用未准备环境；本次 SQLite 只读查询 `thread_goals` 仍 0 行，本地无 hooks.json，config.toml 无 `[hooks]` 节 | 已修订并复核 | 恢复快照下一步、V-1、M1 前置/内容 | 实现者负责在 M1 起步搭建隔离测试线程、测试 goal 与进度文件，再运行探针；本次未搭建或创建 goal |
| F-06 | 成立的是未解释的目录选择，不是 Python/Node 必须共用测试根的硬限制。既有 [test/xgent-skills.test.js](../../test/xgent-skills.test.js) 位于单数 `test/`，无既有 `tests/` | 已修订并复核 | §1.2、V-2 | 选择最小改法：使用 `test/codex/`；文件清单与 unittest 命令一致，计划无旧 `tests/codex` 引用；本报告历史引用保留原审路径 |
| F-07 | 成立。原 §6 无文档分支只替换摘要措辞，未要求将摘要带入新对话 | 已修订并复核 | §6、V-5、M3 | 有文档分支提供实际计划路径，无文档分支粘贴交接摘要并说明继续；M3 需分别走查两条分支，含进度写入失败 |
| F-08 | 成立。只读 schema 确认 goal 以 thread_id 为主键；[Goals 文档](https://developers.openai.com/cookbook/examples/codex/using_goals_in_codex)明确线程归属；结合 §5.1 无 active goal 不触发规则，可推出新对话的保护边界 | 已修订并复核 | §1.2 文档主题、§6、V-5、M3 | 说明新对话不继承 goal，须手动设置 goal 并明确暂停策略才可使用 guard；旧 goal 可保持 paused 或由用户在旧对话 clear，resume 须重新明确策略。自动继承仍排除，真实接续验收留给 M3 |
| F-09 | 成立。原 §4 告警规格仅说明检测不可用，§8 前两行只有实现者验证入口；[Hooks 文档](https://learn.chatgpt.com/docs/hooks)确认 systemMessage 会呈现为告警 | 已修订并复核 | §4、§8 前两行、V-4 | 告警要求给出 `/hooks` 停用与随附说明排查入口；失败表分清用户动作和实现者复测。真实告警可见性/可行动性留给 V-4 |

### 待核实项与决策

| 原 §2 项目 | 本次证据与状态 | 计划落点与后续 |
| --- | --- | --- |
| `update_goal` 完整定义与参数 schema | 当前会话定义及二进制描述足以核实 F-01 限制；仍待核实目标测试环境的完整定义/schema，不以本会话代替目标环境 | §4、V-1、M1：保存完整定义/schema 和去敏用户指令，验证预设 65% 策略是否可直接暂停；不预设其满足显式请求条款 |
| Stop 空输出接受度 | 仍待核实。亲读 Hooks 通用输出与 Stop 小节，空输出成功和 Stop 要求 JSON 的文字张力仍在 | §4、V-4、M1/M2：M1 实测并固定空操作格式，列为 M1 退出条件及 M2 前置，不把“空操作”直接等同空 stdout |
| 项目层未信任时的提示形态 | 仍待核实。官方明确加载与信任规则，未明确该情形是否完全静默 | §8、V-4、M3：记录实际提示；在生效确认之前不以无告警推断已启用 |
| `CODEX_HOME` 进程继承 | 本次命令进程实测继承项目 `.codex`，补足该进程的环境证据；真实 hook 进程仍待核实 | §1.1、V-1、M1：在真实 hook 进程记录继承值与正确数据库定位；不将 shell 证据冒称 hook 证据 |
| A-1 全链路 | 仍待核实/阻塞。无 active 测试 goal，无 hook 配置及完整链路证据；本次没有运行 V-1 | §0.1、§4、V-1、M1、§11：按三类行为记录结果，仅直接原生暂停并停止续跑等全部判据满足后解除 |

部署方式（复制单文件、绝对路径）和测试目录（`test/codex/`）属于 C-1 范围内的可逆实施细节，沿用现有架构与仓库惯例，无新产品范围或权限决策。9 条缺陷意见均采纳；没有待用户拍板项。预设策略与原生显式请求契约的适配仍是技术验证门，实测失败后另行修订控制路径，不能通过本次文案修订宣称已解决运行问题。

### 修订后结论与验证

- 本次文档处置：**9 项已修订并复核，0 项不采纳，0 项待修订**；原 §2 的 **5 项目标运行待核实事项全部保留**并明确时点/责任，A-1 仍阻塞。状态 **Blocked**，可以推进 M1 契约验证；进度仍 **0/3**，无新增完成记录。
- `bash skills/dev-plan/scripts/check_paths.sh docs/plan/codex-context-goal-guard.md .`：exit 0，检查 9 个路径样式 token，6 个 MISSING。逐条核对为 §1.2 四个 ★ 新增交付物与目标项目部署目录/脚本 `.codex/hooks/`、`.codex/hooks/context-goal-guard.py`；无缺失的现有复用引用。
- `bash skills/dev-plan/scripts/check_progress.sh docs/plan/codex-context-goal-guard.md .`：exit 0，**ERROR 0 / WARN 0**，0/3、3 个里程碑、0 条完成记录一致。
- `git diff --check`：通过。实际解析计划 JSON command 并以 shlex 分词，包含空格的脚本绝对路径保持一个参数；未执行示例命令或未来脚本。
- 回读受影响章节并核对 R-1～R-4、NFR-1～NFR-2、C-1～C-2、A-1 的设计/任务/验收映射；搜索旧测试路径与“安装后”口径，目标计划已消除，原报告中的旧行号/路径明确保留为历史基线。报告追加前已验证原文与修前 hash 一致，交付复核确认追加前缀未改。
- 未运行未来 unittest、py_compile、V-1～V-5 或终端启用走查：四个交付物尚未实现，此次只证明文档修订。拟新增 `docs/codex-context-goal-guard.md` 必须在 M3 落实部署、双层信任、接续与故障说明；无需修改未指定的上游文档。源报告原结论保留，此处仅为本次修订复核结论。
