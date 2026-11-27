# 外部镜像服务类应用 · 集成契约

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。

## 1. 形态与分界

外部镜像 App = 服务端在自己的 repo（任意语言/栈）、独立 Docker 镜像交付。鉴权、统一 :8080 与 `/svc/<key>` 路由遵循门户契约；**`/health` 支持平铺 JSON 与 Envelope，使用平台同一判据**。注册与部署方式如下：

| | 内建 App | 外部镜像 App |
| --- | --- | --- |
| 镜像 | 全部长在同一 runtime 镜像 | 独立镜像（`docker save` tar / 私有 registry，不发公共 dockerhub） |
| 部署 | deploy-controller 按需拉起/缩零 | 由平台按部署描述交付；支持的部署驱动和副本能力须与管理员确认 |
| 注册 | seed / bootstrap 内建 | `app.manifest.json` + register-app（本地一盒）/ 发布提案审批（生产） |

**App 类型**：有用户前端 → `micro`（前端 dist 由平台同源托管到 `/apps/<key>/`）；无前端 → **`service`**（headless：应用市场/应用中心不露卡、不可打开，仅作被调用方；平台控制台清单管理仍可治理，可作依赖被补装）。

## 2. 交付物（外部团队 → 平台）

1. **后端镜像**：容器内监听 **8080**（可由 env 覆盖，编排统一注 8080）；服务名/网络别名 **`<listingKey>-server`**（容器形态的网关声明地址 `<listingKey>-server:<deployDescriptor.port>` 按此 DNS 命中）；多架构 **amd64 + arm64**（生产集群 amd64）；**不烘焙 `.env`/密钥**。⚠️ 只需要认这一个**容器口**；发布到宿主机的哪个口（`deployDescriptor.hostPort`）归平台按环境分配，别在镜像或 manifest 里把它当常量依赖（详见 registration-and-onebox.md）。
2. **`app.manifest.json`**（见 registration-and-onebox.md）。
3. **env 契约表**：逐个列镜像**实际读取**的变量名。⚠️ 镜像内部若用自有前缀（知识库读 `XGENT_PG_DSN`，门户契约别名 `KNOWLEDGE_DATABASE_URL`；且**不读裸 `PORT`**），必须写清映射，编排按镜像认的名字注入。
4. **运维口径**（没有默认答案，必须显式声明）：
   - **迁移 argv**：迁移文件**必须在镜像里**，二进制上给一个迁移入口（如 `--role migrate`），
     由 `deployDescriptor.migrateArgs` 在**换容器前**以一次性容器执行。
     要求：幂等 · `pg_advisory_lock` 包住 · 失败非零退出 · expand-contract 前向兼容。
     不用镜像外的迁移脚本（门户拿不到对方仓的脚本，对方 CI 也连不到门户的库），也不在启动时自迁
     （迁移失败会变成崩溃循环，并挤占健康检查窗口）；
   - 是否需**每租户 bootstrap**：业务表 FK 自己的 `tenants` 表就需要——首次写入前 `INSERT INTO tenants (id, slug) VALUES ('<门户租户UUID>','<slug>') ON CONFLICT DO NOTHING`，`id` **必须等于门户租户 UUID**（自铸 uuid 会在首写触发外键冲突）；`tenant_id` 只是普通列则无需；
   - `/health` 判活口径（§4）。
5. **对接契约文档**：按 contract-doc-template.md 的结构写（路由→scope→PID 表、env、运维命令、信任边界）。

## 3. 运行时鉴权（四道闸）

每个受保护请求：取 `Authorization: Bearer <TDT 或端点已接纳的长期凭证>` → 由 SDK 从缓存取有效声明，未命中或须实时复核时才调门户自省 → 执行四道闸。Bun/TypeScript 使用 `@xgent/portal-server-sdk` 的 `createPortalAuthenticator`（从平台私有包仓安装，不 vendor），缓存接线见 [§3.3](#33-app-后端鉴权缓存)；其他语言实现同一鉴权契约。

```
POST {PORTAL_INTROSPECT_URL}            # 如 http://portal-api:3000/api/tokens/introspect
  Authorization: Basic base64(<saClientId>:<saSecret>)
  body: { "token": "<Bearer 凭证>" }
→ 200 { "ok": true, "data": {
    "active", "kind": "user"|"service", "principal_id", "credential_type", "credential_id", "aud", "listingKey", "azp",
    "tenant_id", "user_id", "scopes": [], "role", "isPlatformAdmin", "bypass", "groups",
    "permissions": [{ "pid", "scope", "relations"? }]|null, "aclStamp", "exp"?,
    "name", "avatar_url", "plan", "tenant_scope" } }
```

1. **身份**：端点显式允许 user/service；用户态检查 `(claims.listingKey ?? claims.aud) === 你的 listingKey`，否则 401 `INVALID_TOKEN`。服务态 aud 是来源 SA clientId、azp 是来源 App，按端点用途校验，不能把来源当目标；
2. **scope** ∈ `claims.scopes`，否则 403 `INSUFFICIENT_SCOPE`；
3. 用户结构性管理操作看 `claims.role === "admin"`；服务身份不能携带用户角色进入此闸；
4. 用户细粒度操作校验 PID，并执行命中授予的 `scope`（own/team/all）与 `relations`。字段必须保留、校验并在业务查询中收口；不支持受限范围时拒绝该授予，不能升级成 all。纯服务端点按 scope、来源及业务绑定授权。

如端点是**平台级跨租户**读/写，在上述四道闸外另加 `claims.isPlatformAdmin === true`。该值由 Portal 在每次自省时按 `user_id` 检查其是否为平台租户 active admin，与当前 `tenant_id` / `role` / `bypass` 独立；它是自省派生信息，**不在 TDT JWT claims 里**。不能用当前租户 admin、平台租户 ID 比较或前端头推导全局身份。

展示与租户字段：`name` / `avatar_url` 仅当令牌 scopes 含 `userinfo.read` 时有值（昵称优先于姓名），否则及服务态恒为 `null`；`plan` 是租户套餐（`free` / `pro` / `ultra` / `unlimited`）；`tenant_scope` 在令牌所属租户是平台租户时为 `"platform"`，否则 `"tenant"`，平台级身份仍以 `isPlatformAdmin` 为准。

硬形状（外部实现最常炸的四处）：

- **信封与故障**：门户响应先检查 `ok === true` 且 `data` 为对象，再校验声明；`ok:false`、缺/null data、坏 JSON、非法声明、自省 HTTP 401/403/5xx 或超时均返回资源端 503。不能从失败信封回退到顶层；合法 `active:false` 或真实到期才是 401。若实现兼容裸声明，只能在根本不存在 ok 字段时走独立完整校验分支。
- **主体与凭证类型**：service 允许 tdt/service_access_key，user 允许 tdt/user_access_key；未知类型、互串类型返回 503。缺省 kind 仅兼容完整合法用户声明，不能推断服务身份。principal_id 是主体 UUID，user 时须与 user_id 一致；service 无 user_id/role/groups/permissions，bypass/isPlatformAdmin 为 false。用户长期 key 只在明确迁移的机器入口启用，并逐请求核验 credentialId 对应的本地业务绑定；普通 REST 默认只接纳 tdt/service_access_key。
- **到期**：TDT 的 exp 必须存在且非 null；无过期长期 key 可省略（兼容旧 null）。exp 是非负安全整数 Unix 秒；0 为已到期，负数/非数值为 503。命中缓存仍判真实到期，不滑动续期或伪造到期，见 [§3.3](#33-app-后端鉴权缓存)。
- **全局身份字段**：`active:true` 始终带布尔 `isPlatformAdmin`，服务态恒为 `false`。Portal 在自省时实时计算；短缓存会延迟身份更新。平台级跨租户及其他要求实时复核的操作用严格自省。
- **门户三变量 all-or-nothing**：自省地址 + SA clientId + SA secret **全缺** → 门户鉴权停用、受门路由 503；**缺一不全** → 启动 fail-fast 打印缺失变量。避免半配置静默放行。
- **租户隔离**按 `claims.tenant_id`（永不信任请求体）；限流自建 `(aud, tenant)` 每分钟窗口（参考默认 600/min）。
- **一致性夹具**：非 TS 实现用 `@xgent/shared` 发布物里的 `fixtures/auth-contract.json` 做解析测试：`bases` 是三类主体的合法声明，`cases` 逐项改字段并给出期望 `active` / `inactive` / `protocol_error`（后者即资源端 503），`cacheCases` 覆盖缓存键隔离。

**批量判 PID**：自省已带 `permissions`，一般在本地判即可。需要门户按当前授予现判时，用同一把 SA 凭证调：

```
POST {API_BASE_URL}/api/v1/acl/check
  Authorization: Basic base64(<saClientId>:<saSecret>)
  body: { "token": "<Bearer 凭证>", "pids": ["<listingKey>:action:x", ...] }
→ 200 { "ok": true, "data": { "active", "results": [{ "pid", "allowed", "scope", "relations"? }] } }
```

令牌无效时 `active:false`、`results` 为空；非成员主体（服务态等）每项一律 `allowed:false`。长期凭证没有 TDT 时改传 `{ tenant_id, user_id, listing_key, pids?, explicit_pids? }` 查该用户此刻的在职、角色、席位与授予（见 [原生客户端 §4.1](native-clients.md)）；其中 `explicit_pids` 返回 `explicitlyHeldPids`，只算显式授予、不认管理员 bypass，最多 1000 项。

**错误码**（完整目录以 `@xgent/shared` 导出的 `ERROR_CODES` 为准）：

- 调自省 / acl/check 时：缺 Basic 凭证 → HTTP 403 `FORBIDDEN`；SA 不存在、密钥错、SA 停用或缺 `token.introspect` 能力 → HTTP 200 + `SERVICE_ACCOUNT_NOT_FOUND` / `SECRET_INVALID` / `CAPABILITY_NOT_GRANTED`；门户鉴权依赖超时 → 503 `PORTAL_UNAVAILABLE`。这些都按上面的信封规则在资源端转成 503，不是 401。
- 资源端（`@xgent/portal-server-sdk` 的约定）：坏令牌或 listingKey 不符 → 401 `INVALID_TOKEN`；缺 scope → 403 `INSUFFICIENT_SCOPE`；主体类型不允许或无 PID 授予 → 403 `FORBIDDEN`；门户不可用或协议错误 → 503 `PORTAL_UNAVAILABLE`。
- 门户签用户票（`POST /api/tokens/mint`）时常见：`CONSENT_REQUIRED`（带 `appName`、`scopes`，引导用户授权）、`SEAT_REQUIRED`（席位制 App 未分配席位）、`APP_DISABLED`；令牌交换另见 [跨应用交换](app-exchange.md#排错决策树)，服务态签发见 [服务令牌](service-tokens.md)。

**三种令牌来路 × 授权姿势**：

以下表格描述成员/服务端点。若 App 还面向终端用户，按 §3.2 单独开放具名端点，不放宽这些后台闸。

| 来路 | kind | user_id | 授权依据 |
| --- | --- | --- | --- |
| 自己前端（宿主注入 / callService） | user | 有 | scope + PID/ACL |
| 其他 App 令牌交换（如 files→omni-parser） | user | 有 | 同上；scope 交集及 consent 见 [跨应用交换](app-exchange.md)；`azp`=发起方（出处归因） |
| 服务账号 client_credentials（服务直调） | service | **无** | **只按 scope**（permissions 空、bypass=false、isPlatformAdmin=false，勿走 PID 门；用户拥有的写操作应拒绝） |

### 3.1 服务账号从哪来：平台代建 vs 租户自助

有**两条**路会产出打给你的 `client_credentials` 凭证，两条铸出来的令牌形状一样（`kind:"service"`、
`user_id:null`、`azp` = 某个 listingKey、`tenant_id` 钉死），你的闸**一个字都不用改**。区别只在谁建、
建给谁：

| | 平台代建（平台预置 / manifest `serviceScopes`） | 租户自助（租户管理员，控制台路径 `/admin/service-accounts`） |
| --- | --- | --- |
| 建的人 | 平台管理员 / 你的 manifest 声明 | **租户自己的管理员**，不经平台 |
| 典型用途 | 你的服务端调门户（files/notification/audit/seats…） | 该租户的采集器、脚本、CI **调你** |
| 租户范围 | 由平台决定（可 `all`） | **恒 allowlist=[他自己那一个租户]**，请求体里出现别的租户 id 直接 `VALIDATION_FAILED` |
| `azp` | 平台服务账号的 `ownerAppKey`；manifest 注册默认绑定自己的 `listingKey`（`serviceScopes` 是 scope 字符串数组） | = 租户选的那个已装 App 的 listingKey（**用户填不了、也不用填**） |
| 可选 scope | `serviceScopes` / `privilegedServiceScopes`（可含 SERVICE_ONLY，走审批） | **只有 `scopes`（用户态那份清单）里、属于你自己命名空间的那些**；平台基础 scope 与 SERVICE_ONLY 一律不可选 |

**⚠️ 对你的唯一要求，就在最后一行**：租户自助的 scope 池取的是**该租户安装你时那份 `apps.scopes`
快照**（= manifest 的 `scopes`），不是 `serviceScopes`。所以——

> 一条只写在 `serviceScopes` / `privilegedServiceScopes` 里的 scope，**租户自助永远勾不到**。
> 想让租户自己开凭证来调你的某条路（典型：日志/指标摄取），那条 scope 必须出现在 manifest 的
> **`scopes`** 里。

这不矛盾：`scopes` 是「这个 App 在这个租户能做的事」的清单，用户态与租户自助的服务态都从它派生；
`serviceScopes` 是「**你的服务端**代表自己去调**别人**」的那一份，不属于租户可自助的范围。

其余几条租户自助的行为，都是你已经在依赖的机制，列出来是为了让你知道不必另做：

- **卸载即失效**：`ownerAppKey` 的装机门跑在**签发时刻**，租户把你卸了之后新令牌立刻
  `APP_NOT_INSTALLED`；同时门户会连坐停用该租户自助建的、绑在你身上的那些服务账号，把**已签出**
  的令牌一并断掉（服务态令牌只认 SA 版本号，不停用会一直活到 TTL 到期）。**停用**是可逆动作，
  不连坐 —— 签发门以 `APP_DISABLED` 挡住新令牌，敞口只有已签出那批的剩余 TTL。
- **scope 收窄即时生效**：租户在应用配置里把你的某条 scope 去掉之后，那些自助钥匙**下一次
  签发**就不再带它（不需要租户去改凭证、也不需要你做任何事）。所以你侧永远以令牌里实际带的
  scope 为准，别缓存「这把 clientId 有哪些 scope」。
- **密钥只显示一次**，可轮换/吊销；轮换即刻作废在途令牌。
- **审计**落在该租户自己的审计日志里（创建/轮换/吊销/删除/每一次签发）。
- 每租户有个数上限（默认 10）。

### 3.2 终端用户端点：显式开放，使用本地业务授权

平台另有 `kind=enduser` 身份，完整能力与接入见 [终端用户参考](end-users.md)。
`@xgent/portal-server-sdk` 缺省只收 user/service；仅在顾客端点配置
`acceptedPrincipalKinds:["enduser"]`、`acceptedCredentialTypes:["tdt"]`，校验 tenant、
ownerAppKey、siteKey、目标 audience、endUserId 与本地业务权限。该主体没有成员
user_id/role/groups/permissions，bypass/isPlatformAdmin 恒 false；不要套用成员管理员/PID 闸，
也不要把整个后端都改成接收终端用户。

HTTP 自省须保留 `end_user_id/session_id/owner_app_key/site_key/euv`；
SDK 的 EndUserAuthContext 使用相应驼峰字段。终端用户正缓存上限 **50 秒**，
仍受 exp 截断，端到端撤销预算 60 秒；要求实时复核的操作用 `strictIntrospection`，见 §3.3。
终端用户登录使用 `/api/public/end-user-auth/*`，不是成员 `/oauth/*`；
业务资料留在 App，平台身份管理走专门的服务态权限。

### 3.3 App 后端鉴权缓存

在**模块级**按入口策略创建并复用 authenticator，别在处理每个请求时新建。SDK 负责自省协议、缓存键、TTL、并发合并、失效和故障处理；App 只提供存储，并在每次业务请求执行适用的身份、scope、ACL 数据范围及本地业务绑定检查。不要为入站鉴权另缓存原始 bearer、claims 或放行结果。

| 使用场景 | SDK 配置 | 请求表现 |
| --- | --- | --- |
| 普通业务 API（独立 App 的默认做法） | 传 App 自己的 `cache` 适配器 | 正缓存默认 30 秒；命中不调门户，相同凭证的并发冷请求合并。不传适配器则没有正缓存，但仍合并并发自省。 |
| **仅限与门户共用同一可信 Redis 数据库**的服务 | 在适配器上加 `getVersions`；需 SDK 0.6.13+ 和支持版本校验的门户 | 只针对成员 TDT，缓存命中批量核对撤销计数和租户 ACL 版本。凭证吊销后不再沿用旧结果；ACL 变更用原票读新权限。其他凭证类型不启用版本校验。独立 App 不连接门户 Redis。 |
| 平台级跨租户操作、受控机器凭证、敏感写入等必须实时复核的入口 | 独立实例设 `strictIntrospection: true` | 每次独立向门户确认，不读写缓存、不共享在途自省。仅设置 `positiveCacheMs: 0` 仍可能合并在途请求，不能当作严格模式。 |

Bun/TypeScript 接线示例：`portalConnection` 是启动时已校验的服务端配置（`introspectUrl`、`clientId`、`clientSecret`），`redis` 是 App 已有的 Redis 客户端。

```ts
import { createPortalAuthenticator, type AuthCache } from "@xgent/portal-server-sdk";

const cache: AuthCache = {
  get: key => redis.get(key),
  set: (key, value, ttlMs) => redis.set(key, value, "PX", ttlMs),
};
const ordinaryAuth = createPortalAuthenticator({ ...portalConnection, cache });
const strictAuth = createPortalAuthenticator({ ...portalConnection, strictIntrospection: true });
// 在各自路由里用 ordinaryAuth.authenticate(bearer) 或 strictAuth.authenticate(bearer)，再做四道闸。
```

若平台确认 App 和门户使用**同一可信 Redis 实例及逻辑库**，在上例 `cache` 对象中增加 `getVersions: keys => redis.mget(...keys)`。独立 App 使用自有 Redis 时保持上例不变；不能指望只升级 SDK 就获得跨服务即时失效。

SDK 从自省开始计时，正缓存受真实 `exp` 截断，上限 55 秒（终端用户 50 秒），端到端撤权预算 60 秒。缓存键隔离完整自省地址、调用者配置和 bearer 哈希，不在值中保存原始凭证；只缓存验证通过的有效声明。版本变化、版本证明缺失或缓存依赖故障时回门户自省，不继续用旧授权；门户也不可用时返回 503，不降级放行。已开始的请求不追溯中止。

**验收**：同票并发冷请求只发一次自省，预热后的普通重复请求不再自省；严格路由逐请求自省。共用 Redis 的服务用两个独立 SDK 实例预热后，验证吊销旧票被拒、改 ACL 后同票读新权限、另一租户不受影响。模拟缓存或版本读取失败，确认回门户且不沿用旧授权。独立 App 的短缓存模式按真实到期和撤权预算验证，不承诺即时失效。

## 4. 健康检查

- **实现、出仓或发布镜像前必读 [health-contract.md](health-contract.md)**：有完整正确实现、错误示例与可执行验收命令。
- `GET /health`（匿名就绪接口）支持平铺 `{"service":"my-app","db":"ok","redis":"disabled"}` 与信封 `{"ok":true,"data":{"db":true}}`。HTTP 非 2xx 不通过；2xx 顶层含 `ok` 时必须 `ok === true && data.db === true`；否则按 HTTP 状态判健康，不校验平铺字段。必要依赖故障建议返回 503，尤其平铺响应不能仅把 db 改为 `"down"` 仍返回 200。
- 可另设 `GET /healthz`（存活，`{"status":"ok"}`），不能用它代替就绪检查。发布前用 skill 的同源脚本验证；Envelope 的 `data.db` 必须是布尔 true，不能混用平铺示例的字符串。

## 5. 响应信封与审计

- **业务接口**响应统一 `{ ok, data | error }`，业务失败 HTTP 仍 200；4xx/5xx 只留传输/认证/路由层（401 缺/坏令牌、403 scope/权限、429 限流）。**`/health` 的形状与故障状态按 §4 实现；允许信封，也允许保留平铺响应，不应被中间件意外改成 `data.db:"ok"`。**
- 审计：不自建审计页，关键操作用当次用户 TDT `POST {API_BASE_URL}/api/v1/audit`（需声明 `audit.write` scope），best-effort。

## 6. 自有认证面与 Portal 认证面的划界

服务自带认证体系（API-Key、节点 Token、自有 admin 会话、gRPC 端口等）时：

- 凡经 `/svc/<key>` 暴露给门户侧的 HTTP 面，一律 TDT 四道闸——不得让门户侧调用方复用你的自有认证。
- 自有节点/外部客户端面保持原认证，但**不得**从 `/svc` 暴露；`/svc` 只转发 HTTP 到 `deployDescriptor.port`（约定 8080），gRPC 等额外端口在网关契约之外，需单独规划暴露并写进交付契约。
- 平台/其他 App 调你：有用户上下文走令牌交换、无用户上下文走服务账号 client_credentials（scope 落你的命名空间）——不是给平台发你体系内的 API-Key。吊销、审计、租户策略都收口在门户。
- 若有 MCP/stdio 之类**免鉴权信任面**：只限本地可信进程，发货镜像不得暴露（知识库的 `--role mcp` 规则）。

## 7. 配置入口

配置归属和唯一接入合同见 [配置分层](config-placement.md)。micro/service 的平台与租户页均可在 `adminPages` 声明 schema；独立租户设置也可用 `tenantConfig.schema`，使用已有 GET/PUT policy API 和门户通用签票/表单。新 App 不复制按 key 定制的 token 端点、config-api 或 AppForm Section。
