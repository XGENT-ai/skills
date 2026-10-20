# Portal 鉴权与身份

## 后台服务调用

普通后台接入使用 `workflow.trigger`，访问 [Open API](open-api.md)。两种凭据来源：

- **调用 App 自己的 SA**：在调用方 manifest 合并 `serviceScopes += workflow.trigger`，若尚未声明目标命名空间，同时合并 `exchangeTargets += workflow`；由平台批准并授予实际服务权限。服务调用仍走 `client_credentials`，不是用户 token-exchange。只加用户 `scopes` 不会给 SA 加权限。
- **租户专用脚本／CI**：租户管理员在 `/admin/service-accounts` 为已安装的 workflow 创建凭据，勾选 `workflow.trigger`。可选项来自已安装 workflow 的 `scopes` 快照；找不到时核对安装快照／更新。不要借用 workflow 后端用于自省的 SA 密钥。

用 HTTP 客户端的 Basic 支持和 JSON 编码器签票：

```http
POST <API_BASE_URL>/api/tokens/service
Authorization: Basic <调用方 SA clientId:secret 的 Basic 编码>
Content-Type: application/json

{
  "grant_type": "client_credentials",
  "tenant_id": "<已授权业务记录的租户>",
  "scope": "workflow.trigger"
}
```

检查 HTTP 和信封，读取 `data.access_token`（若部署契约为 `data.token`，按实际字段适配）、有效期与实授 scope，再以 `Authorization: Bearer <token>` 调 workflow。服务票缓存按租户与权限隔离，提前刷新，不记录明文 token。普通 REST 也接受门户有效的服务长期密钥 `xsak_`，其租户及 scope 限制仍生效。

**服务票 audience 是调用 SA 的 clientId，不要求等于 `workflow`。** Workflow 对服务态要求有效身份、非空 `tenant_id` 与 `azp`，再由路由判 `workflow.trigger`。租户自助凭据的 `azp` 通常为它绑定的 `workflow`；调用 App 自己 SA 的 `azp` 才是调用方 key。发业务事件时 `source` 必须等于实际 `azp`，不能自行编造来源。

## 代表用户调用

普通用户面用 `userinfo.read`；业务 API 和 ACL 见 [用户调用与编排](authoring.md)。调用方 A 不能直接把 `aud=A` 的用户票传给 workflow。跨 App 时，在 A 清单合并（保留既有项）：

```json
{
  "exchangeTargets": ["workflow"],
  "scopes": ["userinfo.read"]
}
```

若明确要以用户身份使用 Open API，另声明并获授 `workflow.trigger`；普通用户面不需要这个 scope。

前端把 A 的当前用户票交给自己的后端；后端先验本 App 的身份和业务权限，再用 A 的 **App Secret** 换目标票：

```http
POST <PORTAL_BASE_URL>/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=urn:ietf:params:oauth:grant-type:token-exchange
&subject_token=<A 的当前用户 TDT>
&audience=workflow
&client_id=<A 的 appKey>
&client_secret=<A 的 App Secret>
&scope=userinfo.read
```

这是字段示意，使用表单编码器逐字段 URL 编码。`scope` 可省略取获准交集，提供时只收窄；显式空值／未知 scope 不合法。按部署的 OAuth 响应解包 `access_token`、`expires_in`、`scope`，不能忽略失败信封。用户票必须以 `workflow` 为目标，并有有效租户、用户及所需权限。

交换还要求 A 允许 `token_exchange`、目标允许交换且白名单含 A、A→workflow 授权关系和用户交换同意都生效。清单提交不等于生产已生效。新用户可能由首次进入 A 的共授覆盖；存量用户缺同意时，引导至 `/exchange-consent?source=<A>&target=workflow&return=<编码的返回地址>`，不代写同意记录。

换票缓存须区分租户、来源／目标 App、用户和请求 scope；App Secret 仅在后端。复用已有 Portal SDK／代理和换票封装。日常获取本 App 用户票不要传裁剪 scopes，子集 mint 会收窄原同意；不要把 TDT 放 localStorage，或把短期用户票当长期后台凭据。

`xuak_` 仅用于 [MCP](mcp.md)，不能打 REST；TDT／`xsak_` 也不能替代 MCP Agent 密钥。不同凭据之间不自动降级。

## 常见故障

| 现象 | 核查与处置 |
| --- | --- |
| 401 `INVALID_TOKEN` | 用户票目标、有效期、租户、凭据类型；服务票不要误按目标 audience 判断 |
| 403 `INSUFFICIENT_SCOPE` | 实际签发 scope、SA 授权或用户交集；声明尚未批准／安装快照过旧也会缺权限 |
| 403 `INSUFFICIENT_PERMISSION`／`FORBIDDEN` | 用户 PID、候选人／所有权或服务态误入用户操作；scope 不等于角色权限 |
| `EXCHANGE_NOT_ALLOWED`／`GRANT_NOT_ALLOWED` | A→workflow 交换布线、目标白名单、来源 grant |
| `EXCHANGE_CONSENT_REQUIRED` | 引导该用户补交换同意，不切高权限 SA |
| `SECRET_INVALID` | A 的 App Secret 与平台保存值是否一致，别混成 SA Secret |
| 503 `PORTAL_UNAVAILABLE` | workflow 到门户自省不可用，交运维核对；必须保持拒绝访问 |
| `/svc/workflow` 404／502 | 前者查注册和反代路由，后者查后端可达性；不是数据不存在 |

只有合法刷新／重新授权后的 401 适合有限重试；权限、consent 和布线失败不能靠无界刷新解决。令牌身份来自门户自省，不接受请求体／自报头覆盖 tenant、user 或 azp。
