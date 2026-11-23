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

签发 scope = 请求 ∩ 上限（省略 `scope` 即取整个上限；绝不扩张，请求项不在上限内时**不报错只剔除**，可能返回空 scope——调用方必须检查返回的 scope 再用）。上限按签发时刻现算，不缓存：

- 服务账号自身 scopes，绑定 App 的平台服务账号另加平台当前配置的默认授予；
- 去掉目标 App 未安装在该租户的外域 scope（平台必带应用的 scope 不受此限）；
- 租户自助建的服务账号，自域 scope 还要在该租户安装快照的 `scopes` 里，租户收窄后下一次签发即生效。

绑定 App 的服务账号要求该 App 在该租户已安装且启用，否则 `APP_NOT_INSTALLED` / `APP_DISABLED`。

## 5. 宿主代理与管理控制面

`sdk.callService` 不替代后端四道闸。管理配置优先使用 [配置分层](config-placement.md) 中的 `adminPages` / `tenantConfig`；旧 files-token / llm-gateway-token 等按 App 签票端点只供既有配置页，不作为新 App 模板。

## 6. 用户 TDT 撤销（四把钥匙）

用户票校验比对四个版本维度，任一 bump 旧令牌立即失效。应用与授权两个计数器按租户分开，一个租户的操作不影响其他租户：

| 维度 | 触发 | 影响面 |
| --- | --- | --- |
| jti 黑名单 | `POST /oauth/revoke` | 单个 TDT |
| (租户, 应用) version | 该租户停用/卸载应用、轮换/吊销 Secret | 该租户内该应用的全部 TDT |
| user version | 管理员停用/移除用户、角色或授予变更、席位回收、密码或其他凭据变更 | 该用户全部应用 TDT |
| (租户, 用户, 应用) version | 用户撤销该应用授权 | 该用户在该租户对该应用的 TDT |

所以角色/授予变更后旧用户票会被资源端判 401：经 `sdk.callService` 的请求由宿主换新票重试一次；自己持票调后端的 App 收到 401 应重新取票，不要当成登录失效。

服务态按服务账号版本与租户策略处理，不能套用户版本表。自省缓存与端到端撤销预算以 [集成契约](integration-contract.md) 为准；敏感操作可用严格自省。
