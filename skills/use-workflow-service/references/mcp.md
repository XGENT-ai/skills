# Agent 的 MCP 接入

地址 `https://<portal-host>/svc/workflow/mcp`，只支持带 Bearer `xuak_` 的 POST JSON-RPC；无会话、不开 SSE，GET／DELETE 返回 405。Agent 密钥由用户本人在 workflow 版头帮助进入的“Agent 接入”页签发，明文仅回显一次；不要自动代签、写入仓库或日志。普通 TDT／服务密钥不能访问 MCP，`xuak_` 也不能访问 REST。

密钥能力为 `read,run,author,publish`，只是工具能力上限；每次调用仍复核属主当下角色与 PID、本地绑定、到期和吊销。先 initialize，再 tools/list，以真实返回的工具与 inputSchema 为准：

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"<调用方>","version":"1"}}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

按服务返回的协议版本工作，当前也接受 `2025-03-26`。不要求客户端保持长期连接。每把密钥每分钟最多 60 次调用，批量按条计；429 尊重 Retry-After。

| 能力 | 当前工具 |
| --- | --- |
| read | `workflow_list,workflow_get,instance_list,instance_get,ci_project_list,ci_build_get` |
| run | `instance_start,instance_cancel,ci_build_start` |
| author | `template_list,action_catalog,workflow_create,workflow_update_draft,workflow_validate` |
| publish | `workflow_publish` |

工具表没有审批、代签密钥、运行任意 shell 或绕过用户权限的入口。

## 普通流程

先 workflow_get 核对流程版本／发起表单；需要设计时先 action_catalog 查真实 target 和输入输出。workflow_create 的 templateKey 与 yaml 二选一；已返回 id 但 `saved:false` 时修改同一 id，不反复创建。workflow_update_draft 的 yaml 与 definition 二选一。语义错误可能保存成功仍有 errors，发布前读取 [编排参考](authoring.md) 的内层结果规则。

发起示意，ID／输入必须替换成发现的值：

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"instance_start","arguments":{"defId":"<流程 UUID>","input":{},"idempotencyKey":"<调用方>:<业务意图>:1"}}}
```

保存 instanceId，之后用 `instance_get` 的 `{id:"<instanceId>"}` 查询。重传保持同键原输入；同键重放不是重做。实例可能等待人工或外部事件，不能反复 instance_start 催进度。取消实例不保证 runner 停止。

tools/call 需同时检查 HTTP、JSON-RPC error、result.isError；对象数据在 `structuredContent`，其他形状从 content 文本解析。工具结果已解掉 REST 外层信封，不能再统一取 `.data`。`isError:false` 仍可能有 `saved:false`、`ok:false` 或 `published:false`，须检查操作内层字段。500 等真服务故障可能是 JSON-RPC 错误，503 则可作为 `isError:true` 的工具错误返回。

## 受控项目构建

1. `ci_project_list {}` 发现本人获授权的 projectKey 和交付类型，不自行登记／猜测项目。
2. `ci_build_start {projectKey,commit,version,idempotencyKey}`：commit 为完整 40 位小写 SHA；version 为该提交 `ci-build.sh --version` 的三段数值版本。固定请求并保存返回 instanceId／instancePath。
3. **至少间隔 20 秒**调用 `ci_build_get {instanceId}`。实例状态、双架构分支与最终产物身份都需核验；只有 `buildComplete:true` 才表示登记要求的构建／最终发布全部成功，单分支或单次上传成功不够。
4. 待确认时从 instancePath 回宿主回查原任务。同键重放只回原实例；显式新 attempt 或新运行必须符合用户意图。取消／撤回失败不证明没有产物，不能盲目重推。

受控项目只能经 ci_build_start 发起；普通 instance_start、Open API、event、schedule、subflow 都受 `CI_GATE_REQUIRED` 保护，不能拿其中一个作为旁路。普通无人值守自动化仍用服务态 Open API；不要假定受控项目构建也接受相同凭据。
