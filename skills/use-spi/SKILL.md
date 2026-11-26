---
name: use-spi
description: '用于让管理工具、IDE、桌面应用、任务调度器或 CI 安装、启动和控制 Super Pi 的 spi 进程。覆盖 CF/COS 安装、ACP 接入、SPI 原生 RPC、多轮会话、流式输出、工具审批、取消与恢复，以及 JSON/print 单次调用。不用于 SPI 内部开发或手动 TUI 操作。'
---

# use-spi

## 选调用方式

| 调用方需要什么 | 入口 | 一轮何时结束 |
| --- | --- | --- |
| 现有 ACP 客户端、多个会话或客户端呈现工具审批 | `spi --mode acp` | 对应 `session/prompt` 的 JSON-RPC 响应，检查 `result.stopReason` 或 `error` |
| 自有管理工具需要连续对话、推送进度、插话、取消或查询 SPI 状态 | `spi --mode rpc` | 普通 prompt 先收到受理响应，再等 `agent_settled`；结算不代表成功 |
| 单次任务，调用方还要结构化进度 | `spi --mode json -p` | 消费 JSON Lines 到 EOF，再检查进程退出码和本次事件 |
| 单次任务，只需要回答文本 | `spi -p` | 读 stdout，检查进程退出码 |

不要为单次任务实现长连接，也不要解析交互式 TUI。ACP 是标准 JSON-RPC 2.0；SPI 原生 RPC 虽然同样逐行传 JSON，却不接受 `method`、`params` 或 `initialize`。两者都是调用方启动的本地子进程，不是 HTTP 服务或后台守护进程。

## 安装或升级

任选一个发布渠道，不必装两次。默认用 Cloudflare R2（CF，HTTPS）：

```bash
curl -fsSL https://download.xgent.ai/super-pi/install.sh | bash
```

腾讯云 COS（中国大陆镜像，HTTP）：

```bash
curl -fsSL http://download.supagent.cn/super-pi/install.sh | bash
```

COS 用 HTTP 传输脚本、归档和校验文件；校验和不能验证发布方身份。只在部署环境明确允许时使用，不要在 CF 失败后自动降级到 HTTP。

两条命令默认安装各自渠道的最新版本到 `~/.local/bin/spi`。固定部署版本或指定目录时，把末尾的 `| bash` 改为 `| bash -s -- --version <版本> --prefix <目录>`，安装路径是 `<目录>/bin/spi`；加 `--dry-run` 只预览。选版本时确认该渠道已发布目标系统和架构的归档。安装后在管理工具的运行环境用实际路径执行 `spi --version`（默认 `"$HOME/.local/bin/spi" --version`）。

## 启动与隔离

- 从管理工具的实际运行环境执行 `spi --version`、`spi --help`，核对目标二进制和所需模式；用绝对可执行路径与 argv 直接启动，不拼 shell 命令。RPC、JSON、print 用子进程 `cwd` 选项目；ACP 每个会话在请求中传绝对 `cwd`。
- 把 stdin、stdout、stderr 分开。长连接持续读取 stdout 和 stderr；每条请求写一个 UTF-8 JSON 对象和换行并 flush。以 ID 关联响应，另行分发通知和服务端请求，不假定下一行就是自己的响应。不要用写完 prompt 就关闭 stdin 的管道跑 ACP/RPC。
- 用 `--model <provider/id>` 固定模型时，在与子进程相同的环境用 `spi auth check` 核对凭据。凭据用 SPI 的凭据存储或提供商环境变量，不写进 argv、客户端配置或协议日志。需要隔离用户配置时设置 `SPI_CODING_AGENT_DIR`，会话存放和查找可指定 `--session-dir`。
- 仅为已信任的项目加载项目配置。`--approve` 是项目配置的信任决定，不是工具审批；`--no-approve` 跳过项目本地配置，不是沙箱。按任务收紧工具和进程权限；例如不需要 MCP 时加 `--no-mcp`，纯文本联调加 `--no-tools`。不要把原生 RPC 或 print 的工具调用当作具有 ACP 的逐次审批。

## ACP：会话、更新和审批

启动：`spi --mode acp`。以下是按顺序发送的请求，不是一次性输入；拿到上一条响应后再发下一条，用真实项目路径和返回的 `sessionId`。不要通过 CLI 传 prompt、`--no-session` 或 `--session`。

```jsonl
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":"/absolute/project","mcpServers":[]}}
{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"<sessionId>","prompt":[{"type":"text","text":"Say spi ready"}]}}
```

- 从 `initialize` 的结果读取 `protocolVersion` 和 `agentCapabilities`，按实际能力使用可选方法；不要假定一条连接可无限容纳会话。一次只给同一会话发一个 prompt。处理 `session/update` 通知：按 `params.sessionId` 分流；`params.update.sessionUpdate == "agent_message_chunk"` 时追加 `params.update.content.text`。工具卡按 `toolCallId` 更新，不把它们当作回答文本。SPI 直接访问宿主文件系统，不读取 IDE 中未保存的缓冲区。
- `session/request_permission` 是 SPI 发给客户端的请求。呈现实际操作和返回的选项；有授权时用**原 ID 及其 JSON 类型**返回 `result: {"outcome":{"outcome":"selected","optionId":"<所选 optionId>"}}`。无审批交互或用户撤回时返回 `result: {"outcome":{"outcome":"cancelled"}}`，不要自动选允许。
- `session/prompt` 返回 `end_turn`、`max_tokens` 或 `cancelled`；provider 失败走 JSON-RPC `error`。`end_turn` 只证明这轮结束，不保证工具或后台命令成功。取消发 `{"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"<sessionId>"}}`，等待原 prompt 的响应；该通知也会取消该会话树中尚在运行的工作，不影响其他会话。
- 保存工作区、会话存储位置与 `sessionId`。若声明 `sessionCapabilities.list`，用 `session/list`（`cwd`，续页传返回的 `nextCursor` 为 `cursor`）找会话；若 `loadSession` 为真，用 `session/load`（`sessionId`、`cwd`、`mcpServers`）恢复。等 load 响应后再发 prompt，把之前的历史重放与新回答分开。尚未发 prompt 的新会话不在持久历史中。若声明 `sessionCapabilities.close`，用 `session/close`（`sessionId`）释放会话，不删除历史；连接不用时关闭 stdin 并等待进程退出。

按需阅读 [ACP 接入参考](references/acp.md)：能力字段、历史回放、审批回包和执行卡。

## SPI 原生 RPC：受理、结算和控制

启动：`spi --mode rpc`；只在不保留会话时加 `--no-session`。无需握手，可用 `get_state` 探测就绪。示例请求：

```jsonl
{"id":"state-1","type":"get_state"}
{"id":"prompt-1","type":"prompt","message":"Say spi ready"}
{"id":"text-1","type":"get_last_assistant_text"}
```

- 用唯一**字符串** `id`。响应为 `{"id":"prompt-1","type":"response","command":"prompt","success":true}`；失败是 `success:false` 带 `error`。其它行可能是会话事件或扩展 UI 事件。
- `prompt` 受理后，`message_update.assistantMessageEvent` 中的 `text_delta` 是新片段，不是全文；用当前轮次的 `message_end.message` 读最终 assistant 内容与 `stopReason`；收到 `agent_settled` 再发下一轮。检查 `aborted` 和本轮的失败证据；若没有本轮回复，不用历史回复补作结果。示例第三条仅在结算后发送，它返回 `data.text`（如果有），只是会话中最后一条 assistant 文本，可能来自前一轮。
- 取消当前运行发 `{"id":"abort-1","type":"abort"}` 并等待响应；需要插话或排下一条消息，用 `steer` 或 `follow_up`（`message`）。已交回 `jobId` 的后台命令可能跨轮继续，用 `command_list`、`command_status`、`command_wait`、`command_cancel` 管理；后三个命令带原 `jobId`。判断真实执行结果看 `data.state`、`exitCode` 和 `cleanup`，不看 RPC 的 `success` 或某次等待窗口是否到期。
- `get_state` 可读 `data.sessionId` 和持久会话的 `data.sessionFile`。继续已落盘会话可在相同项目用 `--session <path|id>` 重启，或在 RPC 连接里发 `switch_session`（`sessionPath`）替换当前会话；`new_session` 创建并切换到新会话。等待替换响应，检查 `success` 和 `data.cancelled`，再发 prompt。一个进程只有一个当前会话；并发独立会话用多个进程。任务结束后关闭 stdin 并等进程退出。

按需阅读 [原生 RPC 接入参考](references/rpc.md)：常用命令字段、会话切换和后台任务的等待与取消。

## 单次任务与联调

```bash
spi -p --no-session -- 'Say spi ready'
spi --mode json -p --no-session -- 'Say spi ready'
```

自动化传任意文本时可用 stdin，写完关闭；以 `@` 起头的 argv 即使在 `--` 后也会被当作文件输入。JSON 模式同样用 `text_delta` 增量刷新、用本轮的 assistant `message_end` 取完整回答，不要把事件流当作一个 JSON 文档。需要保留会话时去掉 `--no-session`。

给管理工具自己的启动、请求和关闭阶段设期限。等待超时或连接意外断开不是成功，也不证明之前没改文件；连接仍可用时先发协议取消，单次模式由调用方终止其启动的子进程。等进程收尾并核对实际效果后再决定是否重跑，避免多个进程同时写同一会话。协议完成、进程退出和业务验收是三回事。

联调可完全不调用真实模型：在临时目录保存 `{"responses":[{"text":"spi ready"}]}`，设置 `SPI_FAUX_SCRIPT` 为其绝对路径、`SPI_CODING_AGENT_DIR` 为独立的临时配置目录，并加 `--offline --no-mcp --no-tools`。用所选模式跑一次从启动到终态的真实调用；ACP 再用受控工具调用验证审批（不要加 `--no-tools`），RPC 验证受理响应与轮次结算确实分开。生产启动不要继承 `SPI_FAUX_SCRIPT`；`--offline` 只关闭启动网络操作，不是阻止模型或工具访问网络的方式。
