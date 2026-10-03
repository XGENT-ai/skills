# Claude Code goal 上下文收尾提醒

当前会话的 `/goal` 仍 active，且本地估算上下文使用率**严格超过阈值**（默认 70%，可配置）时，本 hook 提醒 agent 收尾：停止开始新的实质任务，按任务已有约定更新进度并交接。回合结束时由 Stop hook 结束回合，Claude Code 原生把 goal 置为「Goal paused」，再由你自己新开对话（或 `/clear`）接续。恰好等于阈值不触发。hook 只读当前会话记录，不写任何文件，也不清除、转移或恢复 goal。

## 兼容范围与限制

- 已验证：Claude Code **2.1.288** 交互式 CLI、macOS arm64、Node v25.2.1（安装器要求 Node ≥ 18）。完整验收见 [M1 记录](plan/claude-context-goal-guard.records/M1.md)、[M2 记录](plan/claude-context-goal-guard.records/M2.md)、[M3 记录](plan/claude-context-goal-guard.records/M3.md)、[M4 记录](plan/claude-context-goal-guard.records/M4.md)。新版本不自动视为兼容。
- 未验证：`claude -p` 非交互模式、桌面应用、Remote Control、云端会话。
- 只保护主会话；子 agent 内的事件直接跳过。
- 上下文窗口：hook 输入和会话记录都不含窗口大小，脚本只认下表中验证过的模型。其他模型，或设置了 `CLAUDE_CODE_DISABLE_1M_CONTEXT`、`DISABLE_COMPACT`、`CLAUDE_CODE_MAX_CONTEXT_TOKENS` 任一变量时，报告「检测不可用」，不触发。

| 模型 id（会话记录中的 `message.model`） | 窗口 |
| --- | --- |
| `claude-opus-5-5` | 1,000,000（是否选 `[1m]` 相同） |
| `claude-sonnet-5-5` | 1,000,000（是否选 `[1m]` 相同） |
| `claude-haiku-4-5-20251001` | 200,000 |

窗口还会因账号与服务端能力而不同，上表只在验证时用的 Claude Max 账号上核对过。启用前用 statusLine 自查：statusLine 命令的 stdin JSON 里 `model.id` 和 `context_window.context_window_size` 应与上表一致，不一致时不要启用。

### 用量口径

```text
used    = input_tokens + cache_creation_input_tokens + cache_read_input_tokens   # 取最新主链 assistant 记录；有 iterations 时取最后一项
trigger = used * 100 > window * threshold     # threshold 默认 70，见「阈值配置」
```

与 statusLine 的 `used_percentage` 同口径，不计 output。会话记录异步写入，可能落后一条消息。自动压缩后，出现新统计前不判定。

Claude Code 界面右下角的「NN% context used」按扣除输出预留后的有效窗口计算，读数比本 guard 高。例如 Haiku 4.5 实测 155,100 token 时，界面显示 86%（≈ 155,100 / 180,000），guard 计为 78%。

## 安装

### 使用 install

```sh
# 从本仓库源码安装（发布到 npm 后可用 npx @xgent-ai/skills install）
node bin/xgent-skills.js install /absolute/path/to/project
# 显式安装，不再询问
node bin/xgent-skills.js install /absolute/path/to/project --claude-context-goal-guard
```

交互安装会在 Codex guard 之后询问是否安装 Claude Code goal 上下文收尾提醒，输入 `y` 或 `yes` 确认；回车和非交互环境默认跳过。`--claude-context-goal-guard` 显式安装，`--no-claude-context-goal-guard` 跳过询问并保留已有 guard，两者不能同时使用。这个选项与 `--providers`、`--no-impeccable`、`--xgent-init` 及 Codex guard 的选项互不影响。

安装器会：

1. 复制脚本到目标项目 `.claude/hooks/context-goal-guard.js`；
2. 把 `PostToolBatch` 与 `Stop` 两个 handler 合并进个人的 `.claude/settings.local.json`（不进共享的 `settings.json`），command 为 `node "$CLAUDE_PROJECT_DIR/.claude/hooks/context-goal-guard.js"`，超时 5 秒；
3. 保留其他事件、handler、matcher 和顶层字段，重复安装只更新 guard。非法 JSON 默认报错，`--force` 先备份为 `.bak` 再覆盖；合法 JSON 但结构不认识时报错，需要手动修复。

### 阈值配置

阈值写在 handler 命令末尾：`--threshold N`，N 为 1–99 的整数百分比；不带参数时为 70。与 Codex 版（默认 65）各自独立。

```sh
# 安装时指定（同时表示安装，不再询问）
node bin/xgent-skills.js install /absolute/path/to/project --claude-context-goal-guard-threshold=75
```

安装器总是把阈值明确写进两个 handler 的 command，在 `/hooks` 里看得到。之后重装时不给这个 flag，就沿用已装的值。也可以直接改 `.claude/settings.local.json` 里两条 command 末尾的数字，两条要一致。参数无效时（如 `0`、`100`、`6.5`）guard 不猜测数值，按「检测不可用」处理。

阈值不宜太高：收尾本身还要消耗上下文，而 Claude Code 在接近有效窗口上限时会自动压缩（见上面的界面读数说明）。建议不超过 75%。

### 手动部署

```sh
mkdir -p "/absolute/path/to/project/.claude/hooks"
cp hooks/claude/context-goal-guard.js "/absolute/path/to/project/.claude/hooks/context-goal-guard.js"
```

把 [设置示例](../hooks/claude/settings.example.json) 里的两组 handler（默认 `--threshold 70`，按需修改）**追加**到目标项目 `.claude/settings.local.json` 已有的 `PostToolBatch`、`Stop` 数组，不要用示例覆盖整个文件。Claude Code 的 hook 运行环境里必须能找到 `node`。

## 信任与生效确认

1. 运行 `claude --version`，确认是上面验证过的版本。
2. 从目标项目启动 Claude Code，信任该工作区。工作区未被信任时，设置里的 hook 和 `/goal` 都不可用。
3. 输入 `/hooks`，确认 `PostToolBatch` 和 `Stop` 下各有一条 `[Local] node "$CLAUDE_PROJECT_DIR/.claude/hooks/context-g…`，其他 hooks 仍在。
4. 先用测试 goal 走一遍：越过阈值后应看到提醒，回合结束后出现「Goal paused · a hook ended the turn」。

## goal 条件怎么写

guard 不管进度写到哪里，提醒只要求 agent 按任务**已有约定**更新进度。最好让 goal 条件指向一份带进度区的计划，例如：

```text
/goal 按 docs/plan/xxx.md 开发：全部里程碑完成并回写进度
```

没有计划文件时，agent 会在最终回复里给出交接摘要，不会新建文件。

## 触发后会看到什么

1. 越线后的每个工具批次，agent 都会收到同一条收尾提醒：停止新工作，把手上的工作处理到可交接，更新进度，停止后台任务，写明「因上下文收尾而暂停，工作未完成」。
2. agent 第一次结束回合时，guard 可能先请求补做收尾。界面上显示为「Stop hook error: 上下文估算使用率 NN% …」，这是正常的继续请求，不是故障。
3. 下一次结束回合时，guard 结束回合，界面显示 guard 的说明和「Goal paused · a hook ended the turn · send a message to continue」。goal 评估器会照常判定「未达成」，但不会再自动续跑。

提醒没有强制力。在 goal 的自动续轮里越线时，agent 可能只收到一次提醒、甚至一次都没收到就被暂停。交接不完整时，在原会话发一句「按上下文收尾提醒补交接」，回合结束后会再次暂停。

## 接续

在同一项目目录新开对话，或在原会话输入 `/clear`。告诉 agent 计划路径（或粘贴交接摘要），再重新设置 `/goal`。新对话和 `/clear` 后都**不继承** goal：不重新设置，guard 就不会触发。

原会话的 goal 只是暂停：在原会话里发任何消息都会恢复 goal，guard 随即再次收口；`claude --resume` 也会恢复 goal（不会自动运行）。不再需要时，在原会话 `/goal clear`。

### 后台任务

暂停时如果还有后台命令或子 agent 在跑，它们完成时的通知（以及 goal 的 idle check-in）会唤醒会话。guard 会先请求一次收尾，在请求里列出仍在运行的任务，再结束回合；进度不会前进，但每次唤醒多花两次简短的模型调用。这种被唤醒后再结束的回合，界面可能显示「Goal not yet met… continuing」，状态栏仍是「/goal active」，没有「Goal paused」提示；实测此后约 7 分钟内没有自动回合。

## 停用

从 `.claude/settings.local.json` 删除 command 含 `context-goal-guard.js` 的两个 handler，保留其他 hooks，再删除 `.claude/hooks/context-goal-guard.js`。实测会话运行中修改后，`/hooks` 数秒内就不再列出它们；保险起见仍建议重启会话。已暂停的 goal 不受影响。

不要用 `disableAllHooks`：它会连 `/goal` 一起禁用。

## 排查

guard 只在 **Stop** 时显示告警，不会在每个工具批次刷屏。告警格式为「context goal guard 检测不可用：<原因>」：

| 原因 | 处理 |
| --- | --- |
| 模型 `X` 的上下文窗口未经验证 | 换用上表中的模型，或停用 guard；补表需要先在同环境核对 statusLine 的窗口 |
| 设置了改变上下文窗口的环境变量 `X` | 取消该变量，或停用 guard |
| 无法读取会话记录 / hook 输入缺少会话记录路径 | 核对 Claude Code 版本；持续出现时停用 guard |
| 用量统计格式未知 | Claude Code 的会话记录格式变了，停用 guard 并等待重新验证 |
| 阈值参数无效 | 把两条 command 末尾改为 `--threshold` 加 1–99 的整数，或重新运行安装器 |

检测不可用时，guard 不提醒、不结束回合，也不显示 0%。
