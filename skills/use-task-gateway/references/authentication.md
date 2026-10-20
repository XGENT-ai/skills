# Portal 鉴权与调用身份

普通 App 任务调用需要 `task_gateway.read` 与 `task_gateway.submit`。建服务、签节点凭据等管理操作另需 `task_gateway.admin` + 用户 `role=admin`，不应作为日常提交权限。App Secret、SA Secret、Node Token 和网关 API Key 是不同凭据。

## 代表用户调用

在调用方自己的 manifest 合并，保留原有项：

```json
{
  "exchangeTargets": ["task-gateway"],
  "scopes": ["task_gateway.read", "task_gateway.submit"]
}
```

前端把当前 App 的用户 TDT 交给自己的后端。后端先验证本 App 身份及业务权限，再换取目标 TDT。不能把 `aud=<调用方 App>` 的用户票直接交给 Gateway。

后端请求（以下为协议示意，变量均从调用方配置/请求上下文取得）：

```http
POST <PORTAL_BASE_URL>/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=urn:ietf:params:oauth:grant-type:token-exchange
&subject_token=<当前 App 用户 TDT，逐字段 URL 编码>
&audience=task-gateway
&client_id=<调用方 App key>
&client_secret=<调用方 App Secret>
&scope=task_gateway.read%20task_gateway.submit
```

使用 HTTP 库的 form encoder，不手工拼接含 token 的 query。`scope` 可省略以取获准交集，提供时只能收窄。门户根据来源 App 的用户授权、目标声明和请求上界决定结果；以实际获授 scope 为准，不能靠多请求 scope 提权。

除了提交 manifest，门户还须有生效的交换布线：来源允许 `token_exchange`、目标允许交换且白名单含来源、来源到目标授权关系存在、用户同意此交换。新用户可由首次进入 App 的共授覆盖；接入晚于原 consent 的存量用户可能缺同意。缺同意时引导至门户的 `/exchange-consent?source=<app>&target=task-gateway&return=<编码的返回地址>`，不要代填用户同意记录。

读取 `access_token`、`expires_in` 和 `scope`；若目标门户使用 `{ok,data}` 信封，先检查并解包。App Secret 只放后端。

换票缓存至少区分租户、来源 App、目标 App、用户、本次请求 scope/授权上下文；按有效期提前失效，不能按目标 App 单独缓存造成用户串票。已有门户 SDK/代理能正确换票时复用，不另建平行密钥流。

## 后台服务态调用

无当前用户的队列恢复、调度等使用门户 SA `client_credentials`。给用户 manifest 增加 `scopes` 不会给来源 SA 自动增加权限；平台代建的调用方 SA 需实际授予 `task_gateway.read/submit`（通过其服务权限声明/平台配置），并限定租户范围。

单租户也可由租户管理员在 `/admin/service-accounts` 为已安装 Task Gateway 创建专用调用凭据，选择这两个 scope。此凭据绑定的 `azp` 可能是 `task-gateway`，不要假设必为调用方 App；持久化任务恢复需要维持原来源归因，不能随意更换凭据类别。

```http
POST <API_BASE_URL>/api/tokens/service
Authorization: Basic <调用方 SA client_id:client_secret 的 Basic 编码>
Content-Type: application/json

{
  "grant_type": "client_credentials",
  "tenant_id": "<从本 App 已授权业务记录确定的租户>",
  "scope": "task_gateway.read task_gateway.submit"
}
```

响应先检查 HTTP 与信封，当前门户服务签票常为 `data.access_token`，兼容 `data.token` 时依实际签发契约处理；检查有效期与实际权限。不要使用 Gateway 自身用于 introspection 的 SA 密钥作为调用方凭据。

**用户票和服务票的 audience 不同：** 当前 `gateway/src/portal/gate.rs::identity_from_claims` 仅要求用户票 `(listingKey ?? aud)=task-gateway`。服务态票的 `aud/listingKey` 可以是来源 SA clientId，例如 `reports-server`，来源 App 由 `azp` 归因；不要因其 audience 不是 `task-gateway` 就拒绝合法后台调用。两类都须是有效票、具有非空真实 `tenant_id` 和相应 scopes；服务票无 `user_id`，不因此拥有 admin role。

长任务持久化业务状态，令牌按有效期刷新/重取，不持久化首次用户 token 当长期凭据。以服务身份继续用户发起的任务，需要本 App 已明确的后台执行授权；无该授权时要求用户重新授权并保留可恢复状态。

## 鉴权排错

| 现象 | 核查 |
| --- | --- |
| 401 `INVALID_TOKEN` | 用户票 audience 错误、过期/吊销、无有效租户；不要把服务票的来源 audience 当成故障 |
| 403 `INSUFFICIENT_SCOPE` | 用户授权交集或 SA 实际权限缺 read/submit；检查当前签发/自省结果 |
| `EXCHANGE_NOT_ALLOWED` / `GRANT_NOT_ALLOWED` | 交换授权、目标白名单、来源 grant 是否生效 |
| `EXCHANGE_CONSENT_REQUIRED` | 引导该用户补跨 App 同意，不降级成空列表或高权限 SA |
| `SECRET_INVALID` | 来源 App Secret 与门户保存值是否漂移；不是 SA Secret |
| 503 `INTROSPECT_FAILED` | Gateway 到 Portal 的自省连接或 Gateway 自身 SA；交网关运维处理 |

用户 token 401 可在合法刷新/重新授权后有限重试。403、交换配置或 consent 失败不应无界刷新。不能用 payload/metadata/自报头覆盖租户、user_id 或 origin_app。
