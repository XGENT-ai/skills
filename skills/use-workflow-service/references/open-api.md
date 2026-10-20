# Open API：发起、追踪与业务事件

根地址 `<PORTAL_BASE_URL>/svc/workflow`。以下五条路由均需 `workflow.trigger`，后台默认服务态；持有效目标用户票与该 scope 的用户也可调用。所有业务 JSON 使用 camelCase。

| 方法与路径 | 请求／结果 |
| --- | --- |
| `POST /v1/open/instances` | `{defId,input?,idempotencyKey?}` → `{instanceId,replayed}` |
| `GET /v1/open/instances/{id}` | → 实例摘要或 `null` |
| `POST /v1/open/instances/{id}/events` | `{eventName,messageId,payload?}` → `{duplicated,consumed}` |
| `POST /v1/open/events` | 经营事件 → 去重／分发结果，见下文 |
| `GET /v1/open/instances?objectRef=...` | 可加 `status,page,pageSize` → `{items,total,page,pageSize}` |

成功为 HTTP 200 + `{ok:true,data:...}`，业务失败可能是 HTTP 200 + `{ok:false,error:{code,message}}`。查询不存在／越租户返回 `data:null`；推送到不存在实例则为业务 `NOT_FOUND`。404 表示路由不存在；鉴权 401／403、并发 409、门户不可用 503 等另处理。不能只调用 `raise_for_status()` 就当业务成功。

## 显式发起与恢复

```http
POST /v1/open/instances
Authorization: Bearer <调用方服务票>
Content-Type: application/json

{
  "defId": "<已发布且启用的流程 UUID>",
  "input": {"orderId": "order-42"},
  "idempotencyKey": "<调用方>:order-42:fulfillment:1"
}
```

`input` 按已发布版本的发起表单提供，不要把示例 `orderId` 当固定字段。服务态发起人记为 `service:<azp>`，不能通过输入伪造自然人；需要发起人上级等用户上下文的流程应选用户发起。

持久化本 App run 与上述不可变请求，取到 `instanceId` 后立即保存。幂等键按 **(tenant,idempotencyKey)** 去重，跨流程、来源 App 和用户共享该空间，须包含调用方与业务意图；它不校验同键请求内容是否相同。重传只用原键原请求，不靠改 payload 让旧键执行新工作。发起前仍会检查流程／输入，停用或配置变化后的重放可能被拒绝，已有 instanceId 时直接查询。

普通发起体只支持表中字段，顶层 `objectRef` 不会建立对象关联；需要按对象回查时使用经营事件触发，并在流程 trigger 中配置对象映射。不要发送 `tenant_id`、`initiatorId` 或 `skipInputValidation` 试图覆盖服务端上下文。

查询摘要包含 `id,defId,status,initiatorId,input,error,createdAt,finishedAt`，不包含完整节点图／审批待办。状态：

| 状态 | 调用方含义 |
| --- | --- |
| `running` | 正在推进或等待人／事件／动作；继续查询 |
| `waiting_resubmit` | 等发起人重提，不能当成功／终态 |
| `completed` | 流程成功完成 |
| `rejected`、`canceled`、`failed` | 不成功的终态，分别记录 |

Open API 没有取消、审批或发布路由。需要取消时走已授权的用户入口；不能为后台凭据添加 `userinfo.read` 来绕过用户身份。实例终态不提供通用出站 webhook；默认后台轮询。若要回调本 App，可在流程里显式安排调用本 App 的动作节点，并实现 [动作执行面](action-provider.md)。

## 向指定实例推事件

```json
{
  "eventName": "payment.received",
  "messageId": "<来源>:payment-42:received:1",
  "payload": {"paymentId": "payment-42"}
}
```

`eventName` 对应已配置的等待事件，必填且不超过 120 字；`messageId` 必填且不超过 200 字。事件先于节点激活到达可缓冲，缓冲保留 7 天。`consumed:false` 可能只是节点还没等到它，不是要求换 messageId 再发；重复投递正常返回 `duplicated:true`。

两类事件入口共用 **(tenant,messageId)** 去重，跨实例也不能复用 ID。指定实例事件重复 ID 只保留首次内容，不比较后续 payload；请求超时重发原消息，不以变更内容重用旧 ID。流程停用时推送会返回业务 `WORKFLOW_DISABLED`。

`action.terminal` 是保留事件，只有对应动作执行面自己的服务票（`azp=task.target`）能落定，普通业务调用方不能用它伪造动作成功。顶层 `occurredAt` 不属于此入口的请求字段。

## 发布经营事实并按对象关联

```json
{
  "source": "<当前服务票的 azp>",
  "eventType": "order.paid",
  "messageId": "<来源>:order-42:paid:1",
  "occurredAt": "<RFC3339 时间>",
  "objectRef": "<来源>:order:42",
  "payload": {"orderId": "order-42"},
  "causationId": "<原业务因果链 ID>"
}
```

仅 `source,eventType,messageId` 必填；时间、对象、payload 和因果链按业务提供。`source,eventType` 各最多 64 字。服务态 `source` 必须等于 `azp`；用户态来源强制改写为 `user:<uid>`。流程匹配的是合成事件名 **`<source>.<eventType>`**。若要求以真实 App key 为来源，应使用该 App 自己绑定的 SA，而非绑定 workflow 的租户自助凭据。

事件会按已发布启用流程的 trigger 发起实例，也可按事件名与 objectRef 推进等待节点。触发定义示意如下，需合入 `xgent.flow/1` JSON 定义后校验发布（Pipeline YAML 不表达 trigger）：

```json
{"trigger":{"kind":"event","source":"<source>","eventType":"order.paid","objectRefFrom":"orderId"}}
```

此例把 payload.orderId 作为实例 objectRef，故回查用 `objectRef=order-42`；若要保留请求的带命名空间 objectRef，省略 `objectRefFrom`。字段路径须以实际 payload 为准。

首次接收通常回 `{duplicated:false,eventId,started,advanced}`；`started` 是发起数，`advanced` 是推进数，零不等于传输失败。重放可能回 `{duplicated:true,dispatching:true}`，表示原分发尚待补齐。只有归一化 payload 内容不同的同 ID 重放才标 `conflict:true`；source／对象等其他字段不提供同等冲突校验。调用方须冻结完整事件。保留原 causationId，不能每次重投换因果链 ID 来规避循环保护。

业务事件可靠投递可复用本 App outbox；终态消费、事件投递重试都以已持久化记录恢复，不引入第二套业务事实源。
