# 服务账号、签票与撤销

四道闸、声明形状、主体类型及缓存只维护在 [集成契约](integration-contract.md)，本篇补充签票和撤销操作。

## 4. 服务账号（M2M）

平台服务账号由 manifest 审批或控制台创建（`/api/console/service-accounts`）；租户自助凭证边界见 [集成契约 §3.1](integration-contract.md#31-服务账号从哪来平台代建-vs-租户自助)。`clientId:secret` 走 HTTP Basic。能力：

| capability | 含义 |
| --- | --- |
| `token.introspect` | 可调自省/批量检查——独立后端必需 |
| `client_credentials` | 可自助签发服务态 TDT（受租户策略约束） |

租户访问策略（针对 client_credentials）：`tenantAccess="all"`（任意租户，高风险）或 `"allowlist"`（仅名单内，否则 `TENANT_NOT_ALLOWED`）。新增该能力必须显式选策略，否则 `VALIDATION_FAILED`。

签发服务态 TDT：

```
POST /api/tokens/service
  Authorization: Basic base64(<clientId>:<secret>)
  body: { "grant_type": "client_credentials", "tenant_id": "...", "scope?": "a b c" }
→ 200 { ok: true, data: { access_token, token_type:"Bearer", expires_in, scope } }
```

签发 scope = 请求 ∩ 服务账号 scopes（绝不扩张，SA 未授权时**不报错只返空 scope**——调用方必须检查返回的 scope 再用）。

## 5. 宿主代理与管理控制面

`sdk.callService` 不替代后端四道闸。管理配置优先使用 [配置分层](config-placement.md) 中的 `adminPages` / `tenantConfig`；旧 files-token / llm-gateway-token 等按 App 签票端点只供既有配置页，不作为新 App 模板。

## 6. 用户 TDT 撤销（四把钥匙）

用户票校验比对四个版本维度，任一 bump 旧令牌立即失效：

| 维度 | 触发 | 影响面 |
| --- | --- | --- |
| jti 黑名单 | `POST /oauth/revoke` | 单个 TDT |
| app version | 应用停用/卸载、Secret 轮换/吊销 | 该应用全部 TDT |
| user version | 管理员停用/移除用户 | 该用户全部应用 TDT |
| (user,app) version | 用户撤销该应用授权 | 该用户对该应用的 TDT |

服务态按服务账号版本与租户策略处理，不能套用户版本表。自省缓存与端到端撤销预算以 [集成契约](integration-contract.md) 为准；敏感操作可用严格自省。
