# ACP 接入参考

适用于 `spi --mode acp`。以下接口按 SPI 0.49.0 整理；运行时以 `initialize` 响应声明的能力为准。[返回调用指南](../SKILL.md)。

## 能力与会话恢复

- `result.protocolVersion` 为 `1`。`result.agentCapabilities.loadSession` 是布尔值，`sessionCapabilities.list` 和 `sessionCapabilities.close` 是能力对象（可为 `{}`），检查键是否存在，不与 `true` 比较。
- 支持文字和符合模型能力的图片。不支持客户端文件系统或终端能力、音频、嵌入内容，以及 HTTP/SSE 型客户端 MCP 服务。只从已确认可信的配置传入客户端 MCP 服务器；不用时传 `mcpServers: []`。
- 连接可有多个会话，但同一 `sessionId` 不能并发跑 prompt。SPI 0.49.0 一条连接最多容纳 8 个会话（含正在创建的），到上限时可能关闭最久未使用的空闲会话；没有空闲会话则拒绝新建。主动关闭不再使用的会话，不依赖它们常驻。

在连接已初始化时，先列出同一工作目录下的历史，再按选中的 ID 加载。示例中的项目路径用实际的绝对路径替换；第二条在收到列表结果后发送，第三条在不再使用会话时发送。

```jsonl
{"jsonrpc":"2.0","id":10,"method":"session/list","params":{"cwd":"/absolute/project"}}
{"jsonrpc":"2.0","id":11,"method":"session/load","params":{"cwd":"/absolute/project","sessionId":"<sessionId>","mcpServers":[]}}
{"jsonrpc":"2.0","id":12,"method":"session/close","params":{"sessionId":"<sessionId>"}}
```

读取 `result.sessions`；若返回 `result.nextCursor`，下一次 `session/list` 把它作为 `params.cursor`，并保留相同的 `cwd`。加载时收到的 `session/update` 是历史重放，不代表新模型或工具执行。等 `session/load` 响应后再发新的 prompt。

## 如何处理审批和工具状态

收到的消息若有 `method: "session/request_permission"` 和 `id`，它是 SPI 发给客户端的请求，不是客户端先前请求的响应。用 `params.sessionId` 找到会话，显示 `params.toolCall` 所代表的操作与 `params.options` 提供的选项。会话内普通读取和 bookkeeping 操作不一定弹审批；命令、写入、越出会话目录的读取等需按 SPI 发来的请求审批。

如果对 ID 为 `7` 的请求，用户选择了返回选项中的 `allow_once`，回第一行；如果取消或没有审批界面，回第二行。实际只发送其中一个，并保持原请求的 ID 值与类型。

```jsonl
{"jsonrpc":"2.0","id":7,"result":{"outcome":{"outcome":"selected","optionId":"allow_once"}}}
{"jsonrpc":"2.0","id":7,"result":{"outcome":{"outcome":"cancelled"}}}
```

普通工具卡由 `tool_call` 创建，`tool_call_update` 以同一个 `toolCallId` 更新。交回 `jobId` 的后台命令另有一张执行卡：`tool_call` 带 `kind: "execute"` 和 `rawInput.jobId`，之后的 `tool_call_update.rawOutput` 提供执行状态。即使 prompt 已返回，也继续处理这些更新；原工具完成、等待窗口结束、或 `stopReason: "end_turn"` 都不能代替执行卡的终态。只加载历史时，不会重新启动或重新取得控制权。ACP 没有 `command_*` 方法，不要发送 [原生 RPC 控制请求](rpc.md#后台命令控制)。

## 常见失败

| 表现 | 调用方怎么处理 |
| --- | --- |
| `-32000`（`auth_required`） | 用启动 SPI 的同一环境核对模型与凭据。`authenticate` 只重读已有 SPI 凭据，不开启登录，也不接收密钥。 |
| `-32002`（`resource_not_found`） | 核对会话 ID、工作目录与会话存储位置；不要自动新建会话冒充续接。 |
| `-32010` 且 `error.data.reason` 为 `prompt_active` 或 `session_replaying` | 等原 prompt 或 load 响应，不要并行重发。 |
| `-32011` 且 `error.data.reason` 为 `session_limit` | 释放不再使用的会话，再新建。 |
| 进程在初始化前退出、或 stdout 不是协议帧 | 读独立 stderr，核对实际执行文件、`--mode acp` 和启动参数；直接启动 SPI，不经过会打印启动信息的 shell。 |

`session/cancel` 本身无响应；用尚未完成的 `session/prompt` 响应和执行卡更新来判断已知结果。会话关闭后不应把晚到的审批决定应用到新的会话。
