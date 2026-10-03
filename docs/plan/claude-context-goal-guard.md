# claude-context-goal-guard · Claude Code goal 上下文收尾提醒（本地只读 hook · 回合结束即原生暂停）

> **实施须知：直接依据本计划、仓库规则和当前代码实施或续做。** 先读下方「实施者定位」「实施进度」，核对工作树，再按相关正文与里程碑推进、验证并回写进度；使用 goal 执行时也遵循这些约定。
>
> **计划状态：Ready**
>
> 调查基线：2026-10-03 · dirty@`eab9a030a8089fae41d49de898d662069c78b286`（仅新增本计划与记录目录）；Claude Code 2.1.288、Node v25.2.1、macOS arm64。
>
> 本期交付：**读取当前 Claude Code 会话 transcript，在估算上下文使用率严格超过阈值（默认 70%，handler 命令 `--threshold N` 可配）且本会话 `/goal` 仍 active 时，提醒 agent 更新进度并交接；agent 结束回合时由 Stop hook 以 `continue: false` 结束回合，触发 Claude Code 原生「Goal paused」，再由用户自己新开对话继续。脚本可经 `install` 可选安装。**
> 本期独特职责：与 Codex 版（`docs/plan/codex-context-goal-guard.md`）不同，Claude Code 不给 agent 暂停 goal 的工具，暂停由 hook 结束回合触发；hook 仍不写任何文件。
> **顶层排除：hook 不写进度或交接文档，不另建任务记录或状态文件，不清除 goal，不自动创建新对话，不转移或恢复 goal，不改动 Codex 版行为。**

## 实施者定位

执行本计划的 agent 是**资深软件工程师**，依据本计划、仓库规则和当前代码实施。

- **表达与源码**：报告用路径、需求/检查/里程碑 ID 定位，不复述计划。源码不放需求编号、agent 标记或规划元数据，注释解释必要原因；追溯放记录或提交说明。
- **完成的定义**：本里程碑退出检查全部通过才能记完成；未运行、失败、环境缺失分别记缺口，不冒充通过。集中到阶段收口的终验由收口里程碑负责，通过前不宣称整期或跨期需求已验收。
- **工作顺序**：需要行为测试的改动先红、再绿、再重构。按实际前置成果及检查推进，不等无关终验，数据、安全、发布及用户分期门不得绕过；稳定契约下可用 fixture 开发，不能替代真实集成。按 §9 执行局部检查与集中验收，同环境走查可合并；相关代码、配置、依赖、输入与环境未变可引用已通过证据，否则重跑受影响检查，收口仍跑必要集成回归。
- **并行交付**：独立任务在收益超过委派/集成成本且工具可用时使用 subagents，模型默认继承。任务说明只给目标/ID、稳定契约、可写与禁改路径、运行资源和检查；共享文件单人负责，数据库、端口、浏览器等隔离或串行。子 agent 返回结果、路径、验证证据与缺口，默认不另写任务文档；主 agent 审查并集成验收。边界变化先回报，失败只阻塞依赖任务；结束对话前收集或停止仍在运行的任务。

## 实施进度（实施期持续更新）

本节是跨对话恢复的**唯一汇总入口**，协议文字不随实施改写，实施期只更新「恢复快照」和「完成记录」。续做时先读仓库规则、本节、当前任务相关正文与依赖，再核对 `git status` 和相关 diff；记录与工作树不符时先查明原因，不凭记录覆盖用户改动，也不凭代码存在推断验收已通过。只有继续未完任务、核验前置验收、排错或审查时，才按链接读取对应记录的相关部分；不要默认加载全部记录或原始日志。

**回写时机**：里程碑完成后立即回写，再报告完成或提交；下游只需部分成果时，所需前置检查通过并在记录保存证据即可推进。实质进展后受阻、暂停、交接、发现偏差或结束对话时也保存成果与缺口；普通子任务切换不单独回写。

**一次回写**：主 agent 更新对应记录，再同步快照变化字段和完成记录一行，不重写无变化内容或追加流水。快照写当前任务、进展与下一步，命令及证据只进记录。首次回写删占位行，每个已记进展的里程碑恰好一行，状态仅「进行中 / 阻塞 / 已完成」；完成时移至已完成行末，最近完成取该行。更新时间带时区，摘要 1–2 句，记录填相对计划的链接（如 `[M1 记录](FEATURE.records/M1.md)`）。范围、决策、接口、数据、风险或退出条件有偏差时，就地修订对应正文，不追加勘误历史。

**独立记录**：按需建 `<计划名>.records/M<n>.md`，头部写计划链接、更新时区、状态及验证基线（commit SHA；未提交写 dirty@起始SHA 与关键改动路径）。正文只写交付行为/路径、必要偏差、未完项与继续动作；验收按 `检查 ID/覆盖范围 | 命令或步骤 | 结果/必要证据` 记录，共同环境和基线在头部写一次，有差异再单列。同一测试组可覆盖多条退出条件，须逐条可追溯；引用原证据与复用依据，不复制断言全文、成功日志或调试流水。未运行、失败、环境缺失明确区分；只有复杂交接、排错或证据需独立保存时才另建任务记录/日志/截图并链接。

**回写后自查**：每次回写后直接核对以下条目：「当前进度 n/N」的 N 等于里程碑总数，n 只计状态为「已完成」的行；「最近完成」对应最后一条已完成记录，不按编号推算；链接指向已落盘文件。上述结构、计数与链接检查不证明验收真实通过。

### 恢复快照

- 最近更新：2026-10-03 15:15 +10:00（Australia/Sydney）
- 当前进度：4/4 个里程碑完成
- 当前状态：本期全部里程碑完成；生产脚本经安装器部署后在 Haiku 4.5 真实会话中验证提醒、收尾、原生暂停与两条接续路径；启用说明已落盘
- 最近完成：M4 · 实测启用与人工接续
- 下一步：无本期任务；可选后续见 §11（硬停止决定、Opus/Sonnet 真实触发与 `--resume` 用生产脚本复验）
- 当前阻塞：无
- 代码基线：dirty@eab9a030a8089fae41d49de898d662069c78b286；`hooks/claude/`、`test/claude/`、`bin/xgent-skills.js`、`test/xgent-skills.test.js`、`package.json`、`README.md`、`docs/claude-context-goal-guard.md`、计划与记录目录

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 |
| --- | --- | --- | --- | --- |
| M1 | 已完成 | 2026-10-03 13:50 +10:00 | 真实会话 8 次越线均原生暂停且评估器未误判；三模型窗口对照 statusline 一致；发现后台任务唤醒与提醒不具强制力，已修订 §5 | [M1 记录](claude-context-goal-guard.records/M1.md) |
| M2 | 已完成 | 2026-10-03 14:25 +10:00 | 新增只读 guard 脚本、设置示例与 11 组行为测试，V-2、V-3、V-4 脚本部分通过；无法读会话记录时 Stop 告警，已修订 §5.6 | [M2 记录](claude-context-goal-guard.records/M2.md) |
| M3 | 已完成 | 2026-10-03 14:50 +10:00 | 新增 Claude guard flag、询问与 `settings.local.json` 合并安装，与 impeccable、Codex guard 共存；V-6 通过，57 个 Node 测试与 Codex 测试全绿 | [M3 记录](claude-context-goal-guard.records/M3.md) |
| M4 | 已完成 | 2026-10-03 15:15 +10:00 | 安装器部署后真实走查：越线提醒、计划更新、与评估器及组合 Stop hook 共存、Goal paused 不续跑、`/clear` 与新对话接续、无记录交接、后台唤醒再收口、停用；V-4 组合部分与 V-5 通过，启用说明落盘 | [M4 记录](claude-context-goal-guard.records/M4.md) |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| R-1 | 功能 | 用户 2026-10-03「参考 Codex 版做 Claude Code 版」；同日追加「阈值可配置，Codex 与 Claude 分别设定」，默认 Codex 65、Claude 70 | 读取本地会话记录；估算上下文使用率**严格超过阈值**时触发，阈值由 handler 命令 `--threshold N`（1–99 整数）给出，缺省 70，无效时检测不可用；仅对本会话 active 的 `/goal` 生效 | §5.1、§5.2、§5.5；V-2、V-6 | 已确认 |
| R-2 | 功能 | 沿用 Codex 版 R-2 | 进度更新与交接说明由 agent 按任务已有约定完成；hook 不写进度或记录 | §0.4、§5.3；V-3、V-5 | 已确认 |
| R-3 | 功能 | 沿用 Codex 版 R-3；用户 2026-10-03 选择「hook 结束回合」 | 收尾后停止原会话 goal 的自动续跑：Stop hook 返回 `continue: false`，由 Claude Code 原生把 goal 置为「Goal paused」 | §5.4、ADR-1；V-1、V-5 | 已确认 |
| R-4 | 功能 | 沿用 Codex 版 R-4 | 只提醒用户自己新开对话（或 `/clear`）继续；不自动继承 goal | §6；V-5 | 已确认 |
| R-5 | 功能 | 用户 2026-10-03 选择「脚本+文档+安装器」 | `install` 可选安装本 guard，合并到目标项目 Claude Code 设置并保留其他配置 | §1.2、§5.5；V-6 | 已确认 |
| NFR-1 | 兼容性 | Hooks 文档：transcript 异步写入、可能滞后；transcript 格式非公开契约 | 格式、模型窗口或 goal 状态无法确认时不猜测用量、不结束回合、不声称已暂停 | §4、§5.2、§8；V-2、V-4 | 已知 |
| NFR-2 | 不破坏 | 仓库外科手术式改动规则；Codex 版 NFR-2 | hook 只读 transcript；安装时保留其他 hooks 与设置；Codex guard 的 flag、文件和行为不变 | §3、§5.5；V-3、V-6 | 已知 |
| NFR-3 | 性能 | Hooks 文档：command hook 默认超时 600 秒，过长；M1 实测 | 每次工具批次后同步运行；超时 5 秒。M1 整份读取 38.5 MB 真实 transcript 耗时 0.17 s | §2、§7；V-1、V-4 | 已知 |
| C-1 | 约束 | `AGENTS.md` §2、§3 | 单一脚本、配置示例、启用说明与安装器的最小增量；不顺带改 impeccable、statusLine 或既有 skills | §1.2；V-3、V-6 | 已确认 |
| C-2 | 约束 | `package.json` `engines.node >=18`；`bin/xgent-skills.js`、`.claude/hooks/xgent-statusline.js` 为零依赖 Node | 脚本用 Node 标准库、CommonJS，零第三方依赖；测试用 `node:test` | §0.2 D2；V-2 | 已确认 |
| A-1 | 运行契约 | Goal 文档；[M1 记录](claude-context-goal-guard.records/M1.md) | ①PostToolBatch `additionalContext` 送达主 agent；②Stop `continue: false` 后原生「Goal paused」，评估器不判 met/impossible，无后台任务时不自动续跑；后台任务的 check-in 与完成通知会唤醒回合，guard 再次收口；③`goal_status` 判定 active 与实际一致 | M1 / V-1 通过；后台唤醒按 §5.3、§5.4 处理 | 已解除 |
| A-2 | 运行契约 | hook 输入与 transcript 均不含窗口；[M1 记录](claude-context-goal-guard.records/M1.md) | 已验证表：`claude-opus-5-5`、`claude-sonnet-5-5` 为 1,000,000（是否 `[1m]` 均同），`claude-haiku-4-5-20251001` 为 200,000；与 statusline `context_window_size` 一致 | M1 / V-1 通过；限 M1 账号与模型，其余按 ADR-2 不支持 | 已解除 |

阈值用整数严格比较：`used * 100 > window * threshold`，恰好等于阈值不触发。估算值不承诺与 Claude Code UI 完全一致。

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 检测事件 | `PostToolBatch` 提醒 + `Stop` 收口 | 每批工具调用结束、下次模型请求前追加提醒，并行工具只触发一次；无工具调用的回合由 Stop 兜底；不能实时监测 | Hooks 文档 PostToolBatch 与 additionalContext 章节；M1 已验证送达；新版本不可用时退回 `PostToolUse` |
| D2 | 实现语言 | Node（CommonJS，零依赖） | 与 statusLine、安装器同栈，目标环境已为 npx 安装器具备 Node；不需要 SQLite，不复用 Codex 版 Python 脚本 | C-2；Codex 版选 Python 的理由是只读 SQLite，本版不存在 |
| D3 | goal 状态来源 | 只读当前 transcript 中最后一条 `goal_status` 附件 | 与 Claude Code `--resume` 恢复 goal 的规则同义；无需进程内状态 | §4；A-1③ |
| D4 | 暂停执行者 | Stop hook 返回 `continue: false` + `stopReason` | 见 ADR-1 | 用户 2026-10-03 选择；Goal 文档 |
| D5 | 窗口来源 | 内置「已验证模型 → 窗口」表；改变窗口的环境变量出现时视为不支持 | 见 ADR-2 | A-2 |
| D6 | 用量口径 | 最新主链 assistant `message.usage` 的 `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`；有 `iterations` 时取最后一项 | 与 2.1.288 statusLine `used_percentage` 计算一致，不计 output | §4；M1 与 statusline 对照 |
| D7 | 安装落点 | 脚本复制到目标 `.claude/hooks/context-goal-guard.js`；handler 合并进 `.claude/settings.local.json`；command 用 `node "$CLAUDE_PROJECT_DIR/.claude/hooks/context-goal-guard.js"` | 个人启用、不随仓库共享给团队；路径随项目根可移植，无需写机器绝对路径 | `HOOK_ARTIFACTS['.claude'].dest`（impeccable 同样落 `settings.local.json`）；Hooks 文档 `CLAUDE_PROJECT_DIR` |
| D8 | 源文件位置 | `hooks/claude/`，不放仓库 `.claude/hooks/` | 安装器会把仓库 `.claude/hooks/` 下全部文件无条件复制到目标项目，放那里会绕过用户选择 | `bin/xgent-skills.js` `HOOKS_SRC_DIR` 复制循环 |
| D9 | 安装器形态 | 新增 `--claude-context-goal-guard` / `--no-claude-context-goal-guard` 与独立询问；独立安装函数 | Codex flag 语义不变；与 `installContextGoalGuard` 的合并循环形状相似但文件、事件、匹配式不同，第二个用例先受控重复，不抽公共函数 | NFR-2；架构价值排序「三次再抽象」 |
| D11 | 阈值配置 | handler 命令参数 `--threshold N`，安装器 `--claude-context-goal-guard-threshold=N` 写入，重装缺省沿用旧值 | 按项目生效、在 `/hooks` 可见；与 Codex 版（`--context-goal-guard-threshold=N`，缺省 65）互相独立；不用环境变量 | 用户 2026-10-03 选择 |
| D10 | 去重与状态 | 无 hook 自有状态 | 收尾期间每批工具都会收到同一短提醒；agent 视为同一动作 | R-2、NFR-2 |

### 0.3 ADR-lite

#### ADR-1：由 Stop hook 结束回合来暂停 goal

- 状态：Accepted（用户 2026-10-03 选择；M1 实测通过）
- 背景与驱动：R-3 要求收尾后停止自动续跑。Claude Code `/goal` 是会话级 prompt 型 Stop hook，评估器判「未满足」就再开一轮；agent 没有暂停或清除 goal 的工具（2.1.288 内置 ProposeGoal 说明：只能提议新 goal，清除需用户 `/goal clear`）。
- 备选：
  1. **hook 结束回合（选）**：Goal 文档「Other errors retry or pause the goal」列明 hook 结束回合时 goal 暂停，发消息才继续；Hooks 文档写明 `continue: false` 优先于任何 `decision: "block"`。
  2. **只提醒，请用户手动 `/goal clear`**：零控制，但评估器会继续驱动续跑，R-3 无保障；淘汰。
  3. **让 agent 写出使评估器判定 met/impossible 的内容**：会在 transcript 留下虚假的「已达成/不可能」记录，并依赖评估器措辞；淘汰。
- 决策：PostToolBatch 只提醒；Stop 在触发条件下先请求一次收尾（`stop_hook_active` 为 false），再次 Stop 时结束回合。
- 正面后果：暂停由确定性代码触发，不依赖 agent 遵从；hook 不写任何状态。
- 负面/中性后果：hook 承担了「结束回合」这一控制动作，偏离 Codex 版「只提示」；唯一强制点在 Stop 边界，提醒本身不具强制力（M1 中 agent 曾在用户说「继续」后忽略 6 次提醒继续工作）；goal 续轮的 `stop_hook_active` 恒为 true，越线后若无工具批次，agent 可能未收到提醒就被暂停（§5.4 残余风险）；暂停时仍在运行的后台任务会经 check-in 或完成通知唤醒回合，每次多两次简短调用。
- 重新评估触发：新版本中 `continue: false` 不再暂停而是清除 goal、评估器开始在收尾回合判 met/impossible、或唤醒回合推进了原任务；用户要求越线后硬停止（§11）；或 Claude Code 提供 agent 可调用的暂停接口。

#### ADR-2：上下文窗口来源

- 状态：Accepted（M1 实测）
- 背景与驱动：hook 输入（2.1.288 schema）没有窗口字段，transcript 只记录解析后的模型 id（如 `claude-opus-5-5`），不记录 `[1m]` 选择；Claude Code 内部窗口解析还受 `CLAUDE_CODE_DISABLE_1M_CONTEXT`、`DISABLE_COMPACT` + `CLAUDE_CODE_MAX_CONTEXT_TOKENS` 与服务端模型能力影响。
- 备选：
  1. **内置已验证模型表（选）**：只收录在同环境用 statusline `context_window.context_window_size` 对照过的模型，首期为 §0.1 A-2 所列三项；可选 1M 而 transcript 无法区分且窗口不同的模型、未收录模型、以及上述环境变量出现时一律「检测不可用」。
  2. 读取 `~/.claude/cache/model-catalog/` 内部缓存（本机缓存中 `claude-opus-5-5` 记录 `max_input_tokens: 1000000`）：未公开、可能随版本改变，且不反映 `[1m]`/环境变量；淘汰。
  3. statusLine 把窗口写到状态文件供 hook 读取：需新增持久状态、依赖交互式 statusLine 已配置，耦合两个组件；淘汰。
  4. 安装时让用户填写固定窗口：切换模型后静默失准；淘汰。
- 正面后果：只对已验证组合给数，未知时明确不可用。
- 负面后果：新模型上线需补表并重验；窗口还受账号与服务端模型能力影响，表只对验证过的账号类型成立，启用说明须给出用 statusline 自查的方法。
- 重新评估触发：hook 输入或 transcript 开始提供窗口/`[1m]` 信息；用户主力模型不在表内。

### 0.4 职责与事实所有权

| 参与方 | 拥有 | 不承担 |
| --- | --- | --- |
| Claude Code | transcript、goal 状态与评估、Stop 合并语义、暂停与恢复 | 本项目 65% 策略 |
| hook | 只读判定、收尾提醒、触发条件下结束回合、检测不可用告警 | 进度/交接内容、任何文件写入、清除或恢复 goal |
| agent | 安全收尾、按既有约定更新进度与交接、最终回复 | 暂停 goal、创建接续会话 |
| 用户 | 新开对话或 `/clear`、重新设置 goal | 被要求采用新增记录格式 |

### 0.5 明确不在本期

- **非交互（`-p`）、桌面应用、Remote Control、云端会话**——首期只验证交互式 CLI；其他入口另行验证后再写入支持范围。
- **子 agent 自身上下文**——子 agent 事件直接跳过；只保护主会话。
- **Codex 版改造或两版共享代码**——格式与语言不同，不抽公共模块。
- **自动新开对话、自动设置新 goal**——明确放弃（R-4）。

## 1. 当前事实与改动面

### 1.1 已核实事实与缺口

- **已核实·足够**：`/goal` 是会话级 prompt 型 Stop hook；未满足则再开一轮，满足或判不可能时清除并在 transcript 记录；`/goal clear` 与 `/clear` 都会清除；`--resume`/`--continue` 会恢复仍 active 的 goal，但重置轮次与计时；hook 结束回合、限流等失败时 goal 暂停，发消息继续；后台任务未完成时跳过评估，交互式会话会有 idle check-in（默认 30 分钟，`CLAUDE_CODE_GOAL_CHECKIN_MINUTES` 可调，0 关闭）。来源：[Goal 文档](https://code.claude.com/docs/en/goal)。
- **已核实·足够**：`continue: false` 让 Claude 停止处理并优先于事件自身的 decision；`stopReason` 留在会话中；PostToolBatch、PostToolUse、Stop 等支持 `hookSpecificOutput.additionalContext`；同事件所有匹配 hook 并行运行，同一 handler 出现在多个设置文件只运行一次；`agent_id` 仅在子 agent 内触发时出现；`${CLAUDE_PROJECT_DIR}` 指向会话启动的项目根；transcript 异步写入，可能缺少当前回合最新消息。来源：[Hooks 文档](https://code.claude.com/docs/en/hooks)。
- **已核实·足够**（2.1.288 安装包内置 schema 与本机 transcript，非公开契约）：Stop 输入含 `stop_hook_active`、`last_assistant_message`、`background_tasks`；transcript 的 `attachment.type == "goal_status"` 有三种形态——设置时 `{met:false, sentinel:true, condition}`、评估未满足 `{met:false, condition, reason}`、满足/清除 `{met:true, ...}`，另有 `failed`；`--resume` 恢复规则为「最后一条 goal_status 既非 met 也非 failed 即 active」。assistant 记录含 `isSidechain`、`message.model` 与 `message.usage`（含可选 `iterations`）；压缩写入 `type:"system", subtype:"compact_boundary"`。statusLine 的 `used_percentage = round((input + cache_creation + cache_read) / window * 100)`。内置暂停文案含「Goal paused · a hook ended the turn · send a message to continue」。
- **已核实·缺口**：hook 输入与 transcript 均无窗口大小 → 按 ADR-2 内置已验证表 → 漏做则无法计算比例。
- **已核实·足够**（[M1 记录](claude-context-goal-guard.records/M1.md)）：A-1、A-2 的真实行为；续轮 `stop_hook_active` 恒为 true；子 agent 事件带 `agent_id` 而 `transcript_path` 仍为主会话；`/clear` 生成新 transcript；`--resume` 恢复暂停 goal 为 active 但不自动运行；连续 Stop block 上限 9 次（`CLAUDE_CODE_STOP_HOOK_BLOCK_CAP`）；block reason 在 UI 显示为「Stop hook error」。
- **已核实·足够**：安装器把仓库 `.claude/hooks/` 全部文件无条件复制到目标项目，statusLine 写入 `.claude/settings.json`，impeccable 的 Claude hooks 落 `.claude/settings.local.json`；Codex guard 由 `installContextGoalGuard` 合并 `.codex/hooks.json`，按正则剔旧再追加（`bin/xgent-skills.js`）。测试辅助 `run()` 默认追加 `--no-context-goal-guard` 以避免交互询问（`test/xgent-skills.test.js`）。

### 1.2 文件清单

★ 为新增文件。

| 文件 | 改动 | 用途 |
| --- | --- | --- |
| `hooks/claude/context-goal-guard.js` ★ | 新增 | 单一只读 hook 程序，stdin 读输入，stdout 输出 hook JSON |
| `hooks/claude/settings.example.json` ★ | 新增 | `PostToolBatch` / `Stop` 两个 handler 的设置片段，供手动合并 |
| `test/claude/context-goal-guard.test.js` ★ | 新增 | 以子进程运行真实脚本，配合临时 transcript 的行为测试 |
| `docs/claude-context-goal-guard.md` ★ | 新增 | 兼容范围、安装与手动部署、信任与生效确认、goal 条件写法、人工接续、停用与排查 |
| `bin/xgent-skills.js` | 修改 | 新 flag、询问、`installClaudeContextGoalGuard`、安装后步骤、帮助文本 |
| `test/xgent-skills.test.js` | 修改 | `run()` 默认追加 `--no-claude-context-goal-guard`；新增安装器测试 |
| `package.json` | 修改 | `files` 增加 `hooks/claude/context-goal-guard.js`、`hooks/claude/settings.example.json`、`docs/claude-context-goal-guard.md` |
| `README.md` | 修改 | install 选项表与 Claude guard 小节 |

不修改：Codex guard 的脚本、测试、文档与 flag；`MANAGED_SETTINGS`（statusLine）；impeccable 安装逻辑；仓库自身 `.claude/` 设置。

## 2. 模块、接口与依赖

只有一个生产脚本，内部分为输入校验、goal 状态读取、用量读取、窗口解析、输出生成几个函数，不新增 adapter 或后台进程。

| 入口/数据 | 契约与不变量 | 依赖与验证 |
| --- | --- | --- |
| stdin | 一份 hook JSON；只处理 `PostToolBatch`、`Stop`；有 `agent_id` 即子 agent，直接空操作 | 进程入口测试 |
| transcript | 只读读取 `transcript_path` 全文按行解析，解析失败的行（含未写完的末行）跳过；只看 `isSidechain` 非 true 的记录 | 临时 JSONL fixture；M1 已测 38.5 MB 0.17 s |
| goal 状态 | 自尾部找到的第一条 `goal_status` 决定：`met` 或 `failed` 为真→非 active；否则 active；找不到→无 goal | fixture；A-1③ |
| 用量 | 最近一条有 `message.usage` 的主链 assistant 记录，跳过 `model == "<synthetic>"` 与 API 错误记录；若其后出现 `compact_boundary`，在新统计出现前视为无数据（空操作） | fixture；M1 对照 statusline |
| 窗口 | `message.model` 查已验证表（§0.1 A-2）；`CLAUDE_CODE_DISABLE_1M_CONTEXT`、`DISABLE_COMPACT`、`CLAUDE_CODE_MAX_CONTEXT_TOKENS` 任一非空→不支持 | fixture；A-2 |
| stdout | 空操作：exit 0、无输出（M1 已确认）；提醒、Stop 请求、结束回合与告警格式见 §5 | 进程入口测试 + M1 |

脚本内任何异常都转为「检测不可用」处理，永不以 exit 2 退出（exit 2 会阻断）。超时 5 秒；整份同步读取已足够快（NFR-3），不为倒序分块读取增加复杂度。

## 3. 数据与写入边界

**hook 运行期间不写任何文件**：不写进度、交接、缓存、锁、完成标记或日志。transcript 只读。测试需核验运行前后临时目录文件树与内容不变。安装器只写 §1.2 列出的目标文件，非法 JSON 的处理与既有约定一致（默认报错，`--force` 先备份 `.bak`）。

## 4. 集成与契约

| 依赖 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| PostToolBatch `additionalContext` | 已核实·足够 | Hooks 文档；M1 送达并记为 `hook_additional_context` | 输出 `{"hookSpecificOutput":{"hookEventName":"PostToolBatch","additionalContext":…}}` | 新版本不送达则改用 `PostToolUse` 并重跑 V-1 |
| Stop `decision: "block"` / `continue: false` | 已核实·足够 | Hooks 文档；M1：8 次结束回合均「Goal paused」，压过评估器与另一 Stop hook 的 block | §5.4 | 新版本若改为清除 goal 或误判，停用并重验 |
| `stop_hook_active` 在 goal 续轮中的取值 | 已核实·足够 | M1：goal 续轮恒为 true；首轮、用户消息与唤醒回合为 false | §5.4 | — |
| 后台任务与 idle check-in | 已核实·缺口 | M1：暂停后 check-in 与后台完成通知各开启一回合；M4：唤醒回合被结束后界面显示「Goal not yet met… continuing」与「/goal active」而非「Goal paused」，但约 7 分钟无自动回合 | §5.3 要求停止后台任务；§5.4 列出并再次收口；启用说明解释界面差异 | 每次唤醒多两次简短调用，不推进原任务 |
| transcript 格式（goal_status、usage、compact_boundary、sidechain） | 已核实·本机 2.1.288；非公开契约 | §1.1 | 首期只承诺实测版本 | 未知结构→检测不可用 |
| 上下文窗口 | 已核实·足够（限已验证表） | M1 statusline 对照 | ADR-2 | 不支持→检测不可用 |
| 工作区信任与设置生效时机 | 已核实·足够（goal 与设置中的 hook 同受工作区信任约束；M4：运行中删除 handler 后 `/hooks` 数秒内即反映） | Goal 文档 Requirements；Hooks 文档 `/hooks` 章节；M4 记录 | 启用说明仍建议安装与停用后重启会话 | 未信任时 hook 与 `/goal` 都不可用 |
| `disableAllHooks` / `allowManagedHooksOnly` | 已核实·文档 | Goal 文档 Requirements | 停用说明不推荐 `disableAllHooks`，因为它同时禁用 `/goal` | — |

## 5. 核心机制

### 5.1 判定顺序

1. 解析输入；事件不是 `PostToolBatch`/`Stop`、或带 `agent_id` → 空操作。
2. 读 transcript 判定 goal；无 goal 或非 active → 空操作（无论用量）。
3. 读最新用量；压缩后尚无新统计 → 空操作。
4. 解析窗口；不支持或格式异常 → 检测不可用（§5.6）。
5. 解析阈值参数（无效 → 检测不可用）；未超过阈值 → 空操作；超过 → 按事件走 §5.3 或 §5.4。

### 5.2 用量口径与边界

```text
used    = input_tokens + cache_creation_input_tokens + cache_read_input_tokens   # 有 iterations 时取最后一项
window  = 已验证模型表[message.model]
trigger = used * 100 > window * threshold    # --threshold N，缺省 70
display = round(used / window * 100)
```

不使用 output、累计用量、goal 的 token 统计或速率限额百分比。transcript 可能滞后一条消息，65% 留出收尾余量；不保证一次大输出或自动压缩前一定捕获。

### 5.3 收尾提醒（PostToolBatch）

提醒只含比例与以下动作，不带会话摘录或 goal 原文：

1. 停止开始新的实质任务；已启动的工作处理到可交接，不为收尾发起全量测试或额外研究。
2. 按当前任务已有约定更新进度与记录：已完成、未完成、已验证、未验证、下一步。没有既有记录时在最终回复交接，不新建文件。
3. 停止仍在运行的后台命令与子 agent（用户明确要求保留的除外），并在交接中列出；否则 goal 的 check-in 与完成通知会在暂停后唤醒会话。
4. 不要声称 goal 条件已满足或不可能；写明「因上下文收尾而暂停，工作未完成」。
5. 完成后结束本回合；告知用户：guard 会在回合结束时暂停 goal，请在同一项目目录新开对话或 `/clear`，提供计划路径或交接摘要，并重新设置 `/goal`。

提醒开头说明它优先于继续推进 goal 的指令：即使收到「继续」类消息，也先完成收尾并结束回合，新工作在新对话中进行。提醒不具强制力，M1 中 agent 曾忽略；唯一强制点是 §5.4。

暂停前每批工具都会收到同一提醒，agent 视为同一收尾动作，不重复追加记录。

### 5.4 Stop 收口

- goal 非 active、未超阈值、子 agent：空操作。
- active 且超阈值、`stop_hook_active == false`（首轮、用户消息或 check-in/后台通知唤醒的回合）：返回 `decision: "block"`，reason 为 §5.3 指令；Stop 输入的 `background_tasks` 非空时逐项列出 id 与命令并要求停止；末尾补一句「若已完成收尾，只需一句确认后结束」。这只是请求补做收尾，不是暂停。
- active 且超阈值、`stop_hook_active == true`（含 goal 续轮）：返回 `continue: false`，`stopReason` 写比例、「guard 已结束回合，goal 暂停」、§6 的接续指引；`background_tasks` 非空时附上数量，并说明它们结束或 check-in 时会短暂唤醒会话、guard 会再次结束回合。
- 残余风险：goal 续轮的 `stop_hook_active` 恒为 true（M1），越线后本轮若没有工具批次，agent 会在未收到提醒时被暂停。`stopReason` 因此同时提示「如交接不完整，在原会话发一句『按上下文收尾提醒补交接』，回合结束后会再次暂停」。
- 每条 Stop 链最多 block 一次，不触及 Claude Code 的连续 block 上限。

### 5.5 安装器

`installClaudeContextGoalGuard(targetDir, force)`：

1. 复制 `hooks/claude/context-goal-guard.js` 到目标 `.claude/hooks/context-goal-guard.js`（内容相同则「未变」）。
2. 读取 `.claude/settings.local.json`：不存在视为 `{}`；非法 JSON 默认报错，`--force` 时备份 `.bak` 后覆盖；`hooks` 不是对象或某事件不是数组时报错不改。
3. 对 `PostToolBatch` 与 `Stop`：剔除 command 匹配 `.claude/hooks/context-goal-guard.js` 的旧 handler（条目只剩空 hooks 时整条移除），再追加 `{ hooks: [{ type: "command", command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/context-goal-guard.js"', timeout: 5 }] }`；其他事件、handler、顶层字段原样保留；重复安装幂等。
4. 打印安装后步骤：核对 `claude --version`；重启会话并信任工作区；用 `/hooks` 确认两个 handler 来源为 local settings；先用测试 goal 验证；接续与停用方法。

询问与 flag 与 Codex guard 平行：交互环境询问 `[y/N]`，非交互默认跳过并提示 flag；两 flag 同时出现报错；与 Codex guard 共用同一 readline 迭代器。

### 5.6 检测不可用

只在 **Stop 且 goal active**（或会话记录缺失、读失败而无法判定 goal）时输出 `{"systemMessage": "context goal guard 检测不可用：<原因>；参见 docs/claude-context-goal-guard.md 排查，或从 .claude/settings.local.json 移除本 guard"}`，避免每个工具批次刷屏；PostToolBatch 上同样情况静默；stdin 不可解析时事件未知，静默空操作。不可用时不结束回合、不显示 0%。

## 6. 用户交互

成功路径由 agent 回复与 `stopReason` 共同完成，例如：

> 上下文估算使用率 68% 已超过 65%，guard 已结束本回合，goal 暂停。进度已更新到当前任务计划。请在同一项目目录新开对话（或输入 `/clear`），提供计划路径并说明继续执行；新对话不继承 goal，如需继续使用本 guard，请重新设置 `/goal`。

- 没有既有记录时，agent 回复中给出交接摘要，接续指引改为「粘贴以上交接摘要」。
- 旧会话的 goal 保持暂停；用户在旧会话发任何消息都会恢复 goal 并再次触发 guard；`--resume` 也会恢复 goal。不想保留可在旧会话 `/goal clear`。
- 收尾请求在界面显示为「Stop hook error: …」，属正常的继续请求，启用说明需解释。
- hook 不新开对话、不清除或重建 goal。

## 7. 运行保障

- 兼容范围限定为 M1/M4 实测的 Claude Code 版本、模型与平台，写入启用说明；新版本不自动视为兼容。
- 首期只支持交互式 CLI（§0.5）。
- 需要 `node` 在 Claude Code 的 hook 运行环境可解析，与已安装的 statusLine 一致。
- 告警与提醒不包含 goal 原文、会话内容或秘密。
- 超时 5 秒（M1：38.5 MB transcript 0.17 s）。
- 已验证窗口限 M1 账号类型与三个模型；启用说明给出用 statusline 核对 `context_window_size` 的方法，不一致时不要启用。

## 8. 失败模式、启用与停用

| 失败/触发 | 爆炸半径 | 数据后果 | 用户表现 | 检测 | 恢复 | 验证 |
| --- | --- | --- | --- | --- | --- | --- |
| transcript 缺失/格式变化/模型不在表内 | 当前会话失去保护 | 无 | Stop 时显示检测不可用 | systemMessage | 核对版本与模型；补表需重验；或移除 guard | V-2、V-4 |
| PostToolBatch 提醒未送达 | 收尾缺失 | 交接可能不完整 | Stop 首次请求收尾补救 | V-1 | 改用 PostToolUse | V-1 |
| `continue: false` 未暂停或被自动续跑（版本变化） | 原 goal 继续消耗上下文 | 无 | goal 仍 active | V-1 goal 状态 | 用户 `/goal clear`；停用并重验 | V-1、V-5 |
| agent 忽略提醒继续推进（如用户说「继续」） | 越线后继续消耗上下文直到回合结束 | 无 | 提醒出现但工作继续 | M1 已复现 | Stop 仍结束回合；如需更强控制见 §11 开放项 | V-5 |
| 评估器在收尾回合判 met/impossible | goal 被清除并留下误导记录 | transcript 记录失真 | 「Goal cleared」而非「Goal paused」 | V-1 | 修订提醒措辞后重验 | V-1 |
| 后台任务或 check-in 在暂停后唤醒 | 开启简短回合 | 无，进度不前进 | 暂停后又出现回合并再次结束 | M1 已复现 | 提醒要求停止后台任务；Stop 列出并再次收口 | V-1、V-5 |
| 同步 hook 超时 | 本次检测失效 | 无 | Claude Code hook 错误提示 | V-4 | 优化读取或在实测后调整超时 | V-4 |
| 越过阈值后无工具批次即被暂停 | 交接可能缺失 | 无 | stopReason 提示补交接 | §5.4 | 用户在原会话补一句 | V-5 |
| 设置文件非法 JSON / 结构异常 | 安装中止 | 不修改 | 安装器报错 | V-6 | 手动修复或 `--force`（备份） | V-6 |
| 与其他 Stop hook 组合 | 结束回合语义可能受影响 | 无 | — | V-4 | 组合实测；冲突未解决不启用 | V-4 |

启用顺序：M1 通过 → 脚本与测试 → 安装器 → 在干净隔离项目用安装器安装 → 重启会话、信任工作区 → `/hooks` 确认来源与两个 handler → 测试 goal 真实触发与接续 → 在说明中的已验证环境启用。

停用：从 `.claude/settings.local.json` 删除 command 指向 `context-goal-guard.js` 的两个 handler，保留其他 hooks，重启会话；之后可删脚本副本。不要用 `disableAllHooks`，它也会禁用 `/goal`。已暂停的 goal 不受影响。

## 9. 验证

| 检查 ID | 覆盖与可观察结果 | 入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | A-1 / A-2 / R-3（M1 已通过，见记录）：在隔离项目用最小探针 hook 与测试 goal 实测：PostToolBatch 提醒被主 agent 看到；Stop 首次 block、再次 `continue: false` 后 UI 显示 Goal paused，transcript 与 `/goal` 状态仍为未达成，无评估器 met/impossible；把 `CLAUDE_CODE_GOAL_CHECKIN_MINUTES` 调小并带一个后台命令，观察到 check-in 时刻之后仍无自动回合；记录 goal 续轮的 `stop_hook_active` 取值、子 agent 事件的 `agent_id`；对每个计划支持的模型记录 statusline 的 `context_window_size` 与脚本口径的比例差；记录 ≥ 10 MB transcript 的读取耗时；确认空 stdout 被接受 | 真实 Claude Code 交互会话；探针与测试 goal 去敏；不改用户日常设置 | M1，必须在启用前 |
| V-2 | R-1 / NFR-1：缺省阈值 69.99%、70%、70.01%，`--threshold 65` 覆盖及无效参数；`iterations` 取最后一项；output 不计入；sidechain 与 `<synthetic>` 忽略；无 goal、met、failed、clear 后、重新设置后；压缩后无新统计；半行；未知模型与改窗环境变量；子 agent 输入 | `node --test test/claude/*.test.js`（Node 25 不接受目录参数）；临时 JSONL，子进程运行真实脚本 | M2 |
| V-3 | R-2 / NFR-2 / C-1：运行前后临时目录文件树与内容不变；提醒指向既有任务约定；`git diff` 只触及 §1.2 文件 | 同上 + diff 审查 | M2、M3 |
| V-4 | NFR-1 / NFR-3：坏输入、transcript 不存在、读权限错误、超大文件下的降级与耗时；Stop 两个分支与告警格式；与 goal 评估器及另一个 Stop hook 并存时的结果 | 单测覆盖脚本部分；组合实测在真实会话 | 脚本部分 M2，组合部分 M4 |
| V-5 | R-2 / R-3 / R-4：从干净目录经安装器部署后真实触发：有既有计划时 agent 更新计划并交接；无记录时回复中交接；Goal paused 后不自动续跑；新对话与 `/clear` 两条接续路径，未重新设置 goal 时不触发，重新设置后触发；旧会话发消息会恢复并再次暂停 | 真实 Claude Code、隔离项目与测试计划 | M4 |
| V-6 | R-5 / NFR-2：新建与合并 `settings.local.json`、保留其他事件/handler/顶层字段、幂等重装、`--force` 备份、结构异常报错、flag 冲突、非交互跳过、交互询问与 Codex 询问共存、Codex guard 测试不变 | `node --test test/*.test.js test/claude/*.test.js`；`python3 -m unittest discover -s test/codex` | M3 |

静态检查：`node --check hooks/claude/context-goal-guard.js`，不替代行为验收。fixture 只能证明比较与输出，真实 65% 触发与暂停只认 V-1、V-5。

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 核实运行契约 | 本计划 | 搭建隔离项目、测试 goal 与最小探针 hook（只输出固定提醒与按开关返回 Stop 输出，可临时记录 hook 输入到隔离目录）；串行使用同一会话；不启用日常 guard | V-1 通过；固定 Stop 输出、空操作格式、超时值与已验证模型表，就地更新 §0.1、§4、§5、ADR-1/2 与 §11，A-1/A-2 解除后计划状态改为 Ready；回写「实施进度」 |
| M2 | 只读检测与提醒脚本 | M1 固定的契约与模型表（已具备） | 新增脚本、设置示例与行为测试；同一负责人维护脚本与契约 | V-2、V-3、V-4 脚本部分通过；回写「实施进度」 |
| M3 | 安装器集成 | M2 脚本路径与 handler 形态稳定（可与 M2 后半并行，`bin/xgent-skills.js` 与其测试单人修改） | flag、询问、安装函数、步骤输出、帮助、`package.json`、README | V-6 通过，Codex guard 既有测试全绿；回写「实施进度」 |
| M4 | 实测启用与人工接续（阶段收口） | M2、M3 通过 | 从干净隔离目录用安装器部署；走查信任、`/hooks`、真实触发、与其他 Stop hook 组合、两条接续路径、停用；写完启用说明 | V-4 组合部分、V-5 通过，已验证版本/模型/限制写入 `docs/claude-context-goal-guard.md`；回写「实施进度」 |

规模小，默认串行；只有 M3 在 M2 契约稳定后可交给子 agent 并行。

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1、A-2 | — | M1 已解除；M4 已用生产脚本在 Haiku 4.5 复验；Opus/Sonnet 真实触发与 `--resume` 仍只有 M1 探针证据 | 后续按需 | 否 |
| 越线后是否硬停止 | 提醒不具强制力，agent 可能越线后继续工作到回合结束 | **待用户决定**：本期按默认维持现状（只在 Stop 收口）实现；如需增加「越线后仍有 N 个工具批次未结束回合时，在 PostToolBatch 返回 `continue: false`」，需新增 N 或第二阈值，并会打断进行中的收尾 | 下一期 | 否，已按默认实现 |
| 续轮越线后无工具批次即被暂停 | 交接可能缺失 | stopReason 兜底提示；M4 中续轮越线时 agent 收到一次提醒并完成交接 | 持续观察 | 否 |
| 暂停后后台任务/check-in 唤醒 | 额外简短回合；界面标签显示 active | 提醒要求停止后台任务；M4 复验：Stop 列出任务并再次收口，之后无自动回合；agent 用 `kill %1` 停不掉 Claude Code 后台任务 | 持续观察 | 否 |
| transcript 格式与窗口随版本、账号变化 | 漏检测或比例失准 | 只承诺实测版本与模型；未知检测不可用；说明给出自查方法 | M4 | 否 |

- 最终状态：**Ready**。
- 定级理由：M1 在真实 Claude Code 2.1.288 会话中验证了提醒送达、原生暂停、评估器与其他 Stop hook 的组合、续轮取值与三个模型的窗口，A-1 / A-2 解除；M2–M4 交付并验收生产脚本、安装器与启用说明；剩余项均有降级或兜底，硬停止为可选增强。

## 12. 已知坑

- `AGENTS.md` §5：文档、内置 schema 与真实行为分别记录，不能用字段或文案存在推断暂停成功。
- 安装器无条件复制仓库 `.claude/hooks/` 全部文件（`bin/xgent-skills.js` `HOOKS_SRC_DIR`）：guard 源文件必须放 `hooks/claude/`。
- `test/xgent-skills.test.js` 的 `run()` 默认追加 `--no-context-goal-guard` 来避开询问：新增询问后必须同样默认追加 `--no-claude-context-goal-guard`，否则既有交互测试的输入会错位。
- Stop 的 `decision: "block"` 只是请求继续，不是暂停；暂停只由 `continue: false` 触发。Stop 链里不要重复 block：Claude Code 连续 9 次 block 会强制结束（M1 记录）。
- 子 agent 事件带 `agent_id`，不得用其输入判定主会话。
- agent 无法暂停或清除 goal：提醒里不得要求 agent 执行 `/goal clear` 或宣称已暂停。
- `disableAllHooks` 会连 `/goal` 一起禁用，不能作为只停用本 guard 的办法。
- Codex 版的 `--context-goal-guard` 语义不得改变；两版共享名称 `context-goal-guard` 但目录不同（`.codex/hooks/` 与 `.claude/hooks/`）。
