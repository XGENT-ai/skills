# 普通 App 的异步任务 HTTP 契约

所有路径相对于 `TASK_GATEWAY_BASE_URL`，通常为 `https://<portal-host>/svc/task-gateway`。带 `Authorization: Bearer <合法 TDT>`。业务响应为 `{ok:true,data}` 或 `{ok:false,error:{code,message,details?}}`，业务失败常用 HTTP 200；认证/授权、限流和依赖故障还可能为 401/403/429/503。代理可能返回非 JSON 错误页，客户端应保留 HTTP 状态并报告协议/传输错误，不能把解析失败当任务失败。

## 发现和提交

| 方法 | 路径 | Scope | 数据 |
| --- | --- | --- | --- |
| GET | `/v1/services` | `task_gateway.read` | `data.items`：`name,display_name,description,active_nodes,runtime_scope,owned,provisioned` |
| POST | `/v1/tasks` | `task_gateway.submit` | 需 `Idempotency-Key`；返回 `data.task_id` |
| GET | `/v1/tasks/{task_id}` | `task_gateway.read` | 详情，含 payload、状态和结果 |
| GET | `/v1/tasks?service=...&state=...&limit=...&cursor=...` | `task_gateway.read` | `data.items/next_cursor`；列表没有 payload |
| POST | `/v1/tasks/{task_id}/cancel` | `task_gateway.submit` | 无请求体；仅 pending 可取消 |

服务目录包含本租户自有服务和公开的平台服务。使用真实 `name`，不把展示名当物理名。`runtime_scope` 表示 `tenant_managed/platform_managed`；提交还要求 `provisioned=true`。`active_nodes>0` 只证明节点在线，仍需验证执行能力。

以下示例假定目标 agent 的 `service_key.path=service`；服务名、业务 ID 与 payload 字段需换成目标服务的实际契约：

```json
{
  "service_name": "report-worker-a1b2c3-d4e5",
  "payload": {
    "service": "report-worker-a1b2c3-d4e5",
    "operation": "build-report",
    "report_id": "report-001"
  },
  "metadata": {"run_id": "run-001", "attempt": "1"},
  "streaming": false
}
```

```bash
# 业务后端预先保存 request.json 和 IDEMPOTENCY_KEY；不要每次重传重新生成。
curl --silent --show-error "$TASK_GATEWAY_BASE_URL/v1/tasks" \
  -H "Authorization: Bearer $TG_TDT" \
  -H 'Content-Type: application/json' \
  -H "Idempotency-Key: $IDEMPOTENCY_KEY" \
  --data-binary @request.json
```

curl 示例仅展示传输；实现中还须检查 HTTP 状态及响应 `ok`，成功时读取 `data.task_id`，不要从顶层读 task_id。

- `payload` 在网关层可为任意 JSON；执行器 dispatch 从配置的 dot-path 读取服务名。常用 `payload.service == service_name == agent services[].name`；嵌套路径则按执行配置传。没有该字段仍可能受理，随后报 `service_key_missing/mismatch`。
- `metadata` 必须是 string → string；对象/数字先按业务约定序列化。`_` 开头的键保留给网关，调用方提供时会被剥离。普通 metadata 不构成可信身份。
- `streaming` 默认 false；true 还需要流式 executor 配置，不会自动改造 CLI。
- `callback_url` 可选，缺省可能继承租户默认回调。payload 会在详情回显，凭据应留在后端/执行器环境，勿随任务传。

## 持久化与幂等

在第一次发请求前持久化：本 App 的租户/用户/业务对象、run/step/attempt、身份归因策略、目标服务、完整请求体和幂等键。保存不可变请求快照，避免重试时加入新时间戳等字段。接收 task_id 后立即关联；响应丢失或保存 task_id 前崩溃时，用原记录恢复提交。

`Idempotency-Key` 是 1–128 个可打印 ASCII 字符。命名例 `reports:<唯一runId>:<stepId>:attempt:1`；不要把秘密放进 key。去重范围为 **(tenant_id, origin_app, key)**，不含 user_id 或 service_name。相同租户和来源下的不同用户/服务必须避免碰撞。

| 情况 | 处理 |
| --- | --- |
| 网络超时/断线，提交是否落地未知 | 原身份归因、原 key、原请求体重传，取得原 task_id |
| 同 key 改了 service/payload/metadata/streaming/callback_url | `IDEMPOTENCY_CONFLICT`；检查业务记录，不改 key 掩盖问题 |
| 已确认失败，产品策略决定重新执行 | 创建新 attempt 和新 key；旧 key 只会回放旧任务 |
| TDT 到期 | 合法刷新/重取，维持原 tenant 和 origin；换成不同 azp 的 SA 可能变成新去重空间 |

提交重传及轮询沿用 App 的有限重试预算，网络/429/503 可退避并加抖动（有 `Retry-After` 时尊重）；预算耗尽保留恢复入口。参数、权限、同意或幂等冲突先处理原因，不自动换身份或重做任务。幂等只保证提交去重，节点重投、超时与外部系统副作用仍需业务幂等或补偿。

任务与幂等记录有保留期，不是永久去重账本。当前 `gateway/src/audit.rs` 按约 30 天清理 tasks 分区，再清理对应幂等记录；过期后同 key 可能创建新任务。本 App 应及时保存终态结果；恢复很久以前的未知执行或遇到 NOT_FOUND 时，先核对业务账本和目标部署保留策略，不直接重提有副作用的工作。

## 状态与结果

普通 HTTP 面的状态是小写六态：

| 状态 | 调用方行为 |
| --- | --- |
| `pending` | 已排队，继续观察；无节点可长期停留 |
| `assigned` / `running` | 节点已领取/执行，继续观察 |
| `streaming` | 继续观察，可展示 `partial_result` |
| `completed` | 成功终态，保存/消费 result |
| `failed` | 失败终态，保存 error_message；取消也归此态 |

不要把 submit 成功当执行完成，不使用 `succeeded/cancelled` 作为普通任务终态。

```json
{
  "ok": true,
  "data": {
    "task_id": "01900000-0000-7000-8000-000000000001",
    "state": "completed",
    "service_name": "report-worker-a1b2c3-d4e5",
    "payload": {"service": "report-worker-a1b2c3-d4e5", "report_id": "report-001"},
    "result": {"report_id": "report-001", "file_id": "file-001"},
    "error_message": null,
    "partial_result": null,
    "metadata": {"run_id": "run-001", "attempt": "1"}
  }
}
```

详情还带 `created_at/completed_at` 以及 `tenant_id/user_id/origin_app`。示例省略这些字段。result 在网关中可解析为 JSON 时直接返回 JSON 值，否则返回字符串，空结果为 null；不能一律 `JSON.parse(result)`，也不能把 null 或空字符串当失败。failed 时读取 `error_message`，按执行服务契约保留 result 中的错误细节。多个 poller 或重复回调都可能观察同一终态，本 App 按 run/attempt 条件更新，并对通知、计费或后续步骤等结果消费做幂等。

可从每 1–2 秒轮询起步，再按任务时长逐步放缓。每次取一个有效令牌，进程重启从已保存 task_id 恢复；本次等待期限、单次 HTTP 超时和执行器超时分别配置。期限到达只结束观察，不把任务记成已停止；没有证据时不另起新任务。

`partial_result` 是累积文本：连续拿到 `"a"`、`"ab"` 应展示 `"ab"`。Portal HTTP 没有 `/v1/tasks/{id}/stream`、SSE 或 WebSocket 订阅；需要向浏览器推送时由本 App 后端轮询再转发。

列表默认按用户过滤；`scope=tenant` 需管理员，服务态无 user_id 时看全租户。详情/取消只做租户校验；本 App 必须用自己保存的 run/task 关联执行用户和业务权限检查。列表默认过滤不能代替该授权。

## 取消与回调

普通 `/v1/tasks/{id}/cancel` 只取消 pending，成功 `data={task_id,state:"failed"}`；领取后返回 HTTP 200 + `INVALID_STATE`。重复或竞态请求可能已进入终态，重新查权威状态。添加 `abortIfRunning` 不能让此路由终止运行进程；专用运行中止仅在 workflow 动作面。

可选回调优先级：请求 `callback_url` → 租户默认 → 无。默认拒绝 loopback、私网和 link-local，提交和投递均有 SSRF 检查。内部 `portal-api` 或 App 容器地址不能直接当公开回调。

回调 POST 只有 `{"task_id":"...","state":"..."}`；没有 result、用户 TDT、自定义认证头或当前内建签名。它不能直接通过需要 TDT 的普通业务路由。若采用回调，仅接受本 App 已关联的 task_id，把通知去重后用自己的合法 TDT 回查权威详情，再更新业务状态。投递有重试但不保证送达，保留轮询恢复路径；普通接入可直接只做轮询。

## 非鉴权故障定位

| 现象 | 先检查 |
| --- | --- |
| 404/502 或非 JSON | Gateway 安装/启用、实际基址、Portal 反代；是否误拼内部 API_BASE_URL |
| `NOT_FOUND` | 服务物理名、租户可见性、provisioned；任务是否属于当前租户或已过保留期 |
| `VALIDATION_FAILED` | 幂等头、JSON、metadata 字符串值、callback SSRF |
| 长期 pending | 服务 active_nodes、gRPC 出站连接、节点 token 服务集合、并发容量 |
| 领取后失败 | 对齐 payload dispatch 路径、执行程序/运行账号、依赖、环境凭据与 stderr |

行为依据：`gateway/src/portal/tasks.rs`、`gateway/src/queue/redis.rs`、`gateway/src/metadata_guard.rs`、`gateway/src/portal/respond.rs`。不要求调用方自行实现网关的 canonical JSON 摘要，保存并重传原请求即可。
