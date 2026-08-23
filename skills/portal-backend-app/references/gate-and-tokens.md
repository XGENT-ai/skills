# 自省、四道闸与服务账号（资源服务器契约）

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §7/§9/§12（2026-08），响应形状对照 `apps/api/src/modules/token/index.ts` 实现核实。冲突时以门户仓库为准。

## 1. 令牌自省（introspection）

你的服务**不验 JWT 签名**（HS256 密钥只在 Portal），把 bearer TDT 发给自省端点换已解析声明：

```
POST {PORTAL}/api/tokens/introspect
  Authorization: Basic base64(<saClientId>:<saSecret>)   # 服务账号凭证
  body: { "token": "<待验证的 TDT>" }
→ 200 { "ok": true, "data": {
    "active": true, "kind": "user"|"service",
    "aud": "<安装态 appKey>", "listingKey": "<稳定身份>", "azp": "<代表行事的来源App>"|null,
    "tenant_id": "...", "user_id": "..."|null, "scopes": [...], "role": "admin"|"user"|null,
    "isPlatformAdmin": true,
    "bypass": false, "groups": [...]|null, "permissions": [{ "pid", "scope" }]|null,
    "aclStamp": "..."|null, "exp": 1730003600
  } }
```

硬规则：
- **信封解包**：声明在 `data` 里，必须 `claims = body.data ?? body`。裸读顶层 `active` 会把一切有效 TDT 误判 401（真实事故）。
- 无效/已撤销 TDT 也是**成功**的自省，只是 `active:false`——不要当传输错误。
- `active:true` 始终返回布尔 `isPlatformAdmin`：Portal 每次自省都按 `user_id` 实时检查其是否为**平台租户的 active admin**，与当前 `tenant_id` / `role` / `bypass` 独立；服务态恒为 `false`。该字段是**自省派生信息，不在 TDT JWT claims 里**。
- 结果缓存到 `exp` 前（上限 60s）；因此吊销和平台管理员身份变更可能有秒级延迟。
- dev 通道 `x-resource-key` 仅 `DEV_MOCK_OAUTH=true` 时生效且只认门户侧 `FILES_RESOURCE_KEY` 一把；生产一律拒绝，必须走 Basic。

## 2. 四道闸（每个受保护端点）

1. **身份**：`(claims.listingKey ?? claims.aud) === 你的 listingKey`。按 **listingKey**（稳定身份）鉴权而非裸 aud——aud 是安装态 appKey。不匹配 → 401 `INVALID_TOKEN`。
2. **scope**：所需 scope ∈ `claims.scopes` → 否则 403 `INSUFFICIENT_SCOPE`。
3. **结构性管理操作**：`claims.role === "admin"`（角色来自 Portal 成员关系，非 TDT claim）→ 否则 403。
4. **细粒度 ACL**：`claims.bypass || permissions 命中目标 PID`；数据范围取命中授予里**最宽**的（`own`<`team`<`all`）做行级过滤 → 否则 403 `INSUFFICIENT_PERMISSION`。

外加两条恒规则：**租户隔离**一律按 `claims.tenant_id` 收口（永不信任请求体）；**限流**自建 `(aud, tenant)` 每分钟固定窗口。

**平台级跨租户端点另加一道全局闸**：只认服务端自省的 `claims.isPlatformAdmin === true`。不能用 `role === "admin"` / `bypass` 代替（它们只表示 TDT 当前租户），也不能靠 `tenant_id === PLATFORM_TENANT_ID` 推导（平台管理员切到普通租户后仍有全局身份）。拒绝浏览器转发的 Cookie、`X-Is-Platform-Admin` 或 body 字段。

`can(claims, pid)` 实现：`bypass || permissions.some(p => pidMatches(p.pid, pid))`；PID 通配 `app:*` ⊃ `app:page:*` ⊃ `app:page:x.*` ⊃ 精确。批量检查可用 `POST /api/v1/acl/check { token, pids[] }`（同服务账号鉴权）。

## 3. 服务态令牌的授权姿势

`kind:"service"`（client_credentials 签发）**无 `user_id`、整体绕过用户 ACL**：自省返回 `permissions` 空、`bypass=false`、`isPlatformAdmin=false`。对服务态调用**只按 scope 授权**，勿走 PID 门；涉及"用户拥有"的写操作应拒绝服务态。

三种令牌来路对照：

| 来路 | kind | user_id | 授权依据 |
| --- | --- | --- | --- |
| 自己前端（宿主注入 / callService） | user | 有 | scope + PID/ACL |
| 其他 App 令牌交换 | user | 有 | 同上；scope 是交集；`azp`=发起方（出处归因） |
| 服务账号 client_credentials | service | 无 | 只按 scope |

## 4. 服务账号（M2M）

平台管理员在控制台创建（`/api/console/service-accounts`），`clientId:secret` 走 HTTP Basic。能力：

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
→ 200 { access_token, token_type:"Bearer", expires_in, scope }
```

签发 scope = 请求 ∩ 服务账号 scopes（绝不扩张，SA 未授权时**不报错只返空 scope**——调用方必须检查返回的 scope 再用）。

## 5. 宿主代理与管理直连

- `sdk.callService` 只是转发便利层：你的后端仍必须完整四道闸，不要只因请求来自 Portal web 源就信任。
- **管理控制面直连**（应用配置 tab 用）：门户为管理员铸短时 admin-TDT（`GET /api/admin/apps/:appKey/<svc>-token`，assertAdmin + signTdt，aud=appKey、按需 scope、600s），浏览器直连你的后端读写配置，Portal 不经手敏感配置。参照 files-token / llm-gateway-token。

## 6. TDT 撤销（四把钥匙）

网关每次校验比对四个版本维度，任一 bump 旧令牌立即失效：

| 维度 | 触发 | 影响面 |
| --- | --- | --- |
| jti 黑名单 | `POST /oauth/revoke` | 单个 TDT |
| app version | 应用停用/卸载、Secret 轮换/吊销 | 该应用全部 TDT |
| user version | 管理员停用/移除用户 | 该用户全部应用 TDT |
| (user,app) version | 用户撤销该应用授权 | 该用户对该应用的 TDT |

你的自省缓存 ≤60s，所以吊销传导有秒级延迟——安全敏感操作可缩短缓存或直查。
