# SPI 原生 RPC 接入参考

适用于 `spi --mode rpc`，不是 ACP。请求与响应的基础帧见 [调用指南](../SKILL.md)。以下请求按 SPI 0.49.0 整理；在部署的实际版本上确认所用命令，不要把未知命令当成成功。

## 常用请求

每条请求写成独立的 JSON 行，示例 ID 仅用于展示；应为每个未完成请求使用不同的字符串 ID。返回 `type: "response"` 的帧才属于请求响应；会话事件不带对应的 prompt ID。

| `type` | 额外请求字段 | 如何使用 |
| --- | --- | --- |
| `get_state` | 无 | `data` 含 `sessionId`、`isStreaming` 等；有持久文件时还可取得 `sessionFile`。 |
| `get_messages` | 无 | `data.messages` 是当前会话消息，不是仅本轮或仅新增的消息。 |
| `get_session_stats` | 无 | 读取当前会话的累计统计。 |
| `get_available_models` | 无 | 从 `data.models` 取得可用模型。 |
| `set_model` | `provider`、`modelId` | 使用可用模型的提供商和原始模型 ID，不把 `provider/id` 整串塞进 `modelId`。 |
| `prompt` | `message` | `success: true` 只确认受理；读本轮的事件和 `agent_settled`。 |
| `steer` / `follow_up` | `message` | 向当前运行插入消息或排入后续处理；不是同时启动第二个独立会话。 |
| `abort` | 无 | 请求中止当前运行；已移交的非 goal 后台命令不一定随之停止。 |
| `new_session` | 无 | 创建并切换；先读响应中的 `success` 和 `data.cancelled`，再刷新 `get_state`。 |
| `switch_session` | `sessionPath` | 打开指定持久会话文件；同样等替换响应再发 prompt。会话在记录的工作目录运行，不要用未知来源的会话路径。 |

例如，选择模型和切换会话的请求形状如下。用已查到的 ID 和文件路径替换尖括号内的值；切换前先收尾当前运行，不把这几行无条件一起发送。

```jsonl
{"id":"models-1","type":"get_available_models"}
{"id":"model-1","type":"set_model","provider":"<provider>","modelId":"<model-id>"}
{"id":"switch-1","type":"switch_session","sessionPath":"/absolute/session.jsonl"}
```

一次普通发送流程：`prompt` → 受理响应 → 本轮 `message_update`、`message_end`、工具等事件 → `agent_settled`。受理失败时不用等结算；受理后若只有结算而无本轮 assistant 消息，不拿 `get_last_assistant_text` 返回的历史内容冒充结果。

## 后台命令控制

SPI 的原生 `bash` 工具在支持后台任务、且启用了 `command` 工具时，可以用 `yieldTimeMs` 提前交回 `jobId`。不要在 prompt 刚受理时就假定任务存在：从本轮工具结果取得 `jobId`，或等 `agent_settled` 再查任务列表。`bash.timeout` 是执行期限（秒），`yieldTimeMs` 和 `command_wait.waitMs` 是观察窗口（毫秒），不会延长实际执行。只用原始 `jobId` 控制当前会话拥有的任务，例如：

```jsonl
{"id":"list-1","type":"command_list"}
{"id":"status-1","type":"command_status","jobId":"<jobId>","limit":4000}
{"id":"wait-1","type":"command_wait","jobId":"<jobId>","waitMs":10000}
{"id":"cancel-1","type":"command_cancel","jobId":"<jobId>"}
```

原生 RPC 用 `type` 选操作，不传工具调用里的 `action`。`command_list` 返回 `data.tasks`，其余三个返回任务快照作为 `data`。`command_status` 和 `command_wait` 默认从上次交付的输出位置继续；需要回读才传同一任务返回的原样 `cursor` 值。若 `data.output.gap` 为真或 `outputUnavailable` 为真，不能把缺失的输出当成空输出。

- `success: true` 表示控制请求执行成功，不是命令成功。查看 `data.state`（如 `running`、`cancelling`、`completed`、`failed`、`timed_out`、`cancelled`、`interrupted`）、`exitCode` 和 `cleanup`。`cleanup: "incomplete"` 表示清理尚未确认，不视为取消成功。
- `windowElapsed: true` 可与 `state: "running"` 同时出现。若仍需等待，使用新的请求 ID 与同一个 `jobId`，受调用方总等待上限约束；不要重启任务。可在 `command_wait` 未返回时发 `command_cancel`，再读取最终状态。
- 一轮结算不会自动回收后台命令。换会话或关闭连接会收尾旧会话拥有的任务；会话替换期间控制请求可能失败，替换后不能拿旧 `jobId` 控制新会话。
