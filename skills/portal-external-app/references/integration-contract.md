# 外部镜像服务类应用 · 集成契约

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §7/§15 与已接入案例（知识库 / omni-parser，2026-08）（门户仓文件，App 自己的 repo 里没有；本文件已自包含，不必去找）。冲突时以门户仓库为准。

## 1. 形态与分界

外部镜像 App = 服务端在自己的 repo（任意语言/栈）、独立 Docker 镜像交付。与门户内建 App 的**运行时契约完全一致**（四道闸、统一 :8080、`/svc/<key>` 路由、健康信封），差异只在注册与部署：

| | 内建 App | 外部镜像 App |
| --- | --- | --- |
| 镜像 | 全部长在同一 runtime 镜像 | 独立镜像（`docker save` tar / 私有 registry，不发公共 dockerhub） |
| 部署 | deploy-controller 按需拉起/缩零 | **常驻**（controller 不编排外部镜像）：独立 Deployment / compose profile，无状态可多副本 |
| 注册 | seed / bootstrap 内建 | `app.manifest.json` + register-app（dev）/ provisioning 登记（生产） |

**App 类型**：有用户前端 → `micro`（前端 dist 由平台同源托管到 `/apps/<key>/`）；无前端 → **`service`**（headless：应用市场/应用中心不露卡、不可打开，仅作被调用方；平台控制台清单管理仍可治理，可作依赖被补装）。

## 2. 交付物（外部团队 → 平台）

1. **后端镜像**：容器内监听 **8080**（可由 env 覆盖，编排统一注 8080）；服务名/网络别名 **`<listingKey>-server`**（通用反代规则按此 DNS 命中）；多架构 **amd64 + arm64**（生产集群 amd64）；**不烘焙 `.env`/密钥**。⚠️ 只需要认这一个**容器口**；发布到宿主机的哪个口（`deployDescriptor.hostPort`）归平台按环境分配，别在镜像或 manifest 里把它当常量依赖（详见 registration-and-onebox.md）。
2. **`app.manifest.json`**（见 registration-and-onebox.md）。
3. **env 契约表**：逐个列镜像**实际读取**的变量名。⚠️ 镜像内部若用自有前缀（知识库读 `XGENT_PG_DSN`，门户契约别名 `KNOWLEDGE_DATABASE_URL`；且**不读裸 `PORT`**），必须写清映射，编排按镜像认的名字注入。
4. **运维口径**（没有默认答案，必须显式声明）：
   - **迁移 argv**：迁移文件**必须在镜像里**，二进制上给一个迁移入口（如 `--role migrate`），
     由 `deployDescriptor.migrateArgs` 在**换容器前**以一次性容器执行。
     要求：幂等 · `pg_advisory_lock` 包住 · 失败非零退出 · expand-contract 前向兼容。
     ~~外部命令（`scripts/migrate.sh up` 式）~~ 已作废 —— 脚本留在对方 repo 里门户拿不到，
     对方 CI 也连不到门户的库；「启动自迁」同样不再推荐（失败变崩溃循环 + 挤健康窗口）；
   - 是否需**每租户 bootstrap**：业务表 FK 自己的 `tenants` 表就需要——首次写入前 `INSERT INTO tenants (id, slug) VALUES ('<门户租户UUID>','<slug>') ON CONFLICT DO NOTHING`，`id` **必须等于门户租户 UUID**（自铸 uuid 会在首写触发外键冲突）；`tenant_id` 只是普通列则无需；
   - `/health` 判活口径（§4）。
5. **对接契约文档**：按 contract-doc-template.md 的结构写（路由→scope→PID 表、env、运维命令、信任边界）。

## 3. 运行时鉴权（四道闸）

每个受保护请求：取 `Authorization: Bearer <TDT>` → 调门户自省 → 四道闸：

```
POST {PORTAL_INTROSPECT_URL}            # 如 http://portal-api:3000/api/tokens/introspect
  Authorization: Basic base64(<saClientId>:<saSecret>)
  body: { "token": "<TDT>" }
→ 200 { "ok": true, "data": {
    "active", "kind": "user"|"service", "aud", "listingKey", "azp",
    "tenant_id", "user_id", "scopes": [], "role", "isPlatformAdmin", "bypass", "groups",
    "permissions": [{ "pid", "scope" }], "aclStamp", "exp" } }
```

1. **身份**：`(claims.listingKey ?? claims.aud) === 你的 listingKey`，否则 401 `INVALID_TOKEN`；
2. **scope** ∈ `claims.scopes`，否则 403 `INSUFFICIENT_SCOPE`；
3. 结构性管理操作看 `claims.role === "admin"`；
4. 细粒度看 `bypass || permissions 命中 PID`（纯 scope 鉴权的服务可 `aclManifest:null`，跳过此闸）。

如端点是**平台级跨租户**读/写，在上述四道闸外另加 `claims.isPlatformAdmin === true`。该值由 Portal 在每次自省时按 `user_id` 检查其是否为平台租户 active admin，与当前 `tenant_id` / `role` / `bypass` 独立；它是自省派生信息，**不在 TDT JWT claims 里**。不能用当前租户 admin、平台租户 ID 比较或前端头推导全局身份。

硬形状（外部实现最常炸的四处）：

- **信封解包**：声明在 `data` 里，`claims = body.data ?? body`。裸读顶层 `active` → 一切有效 TDT 被判 401（知识库 1.0.0 真实事故）。`active:false` 是成功的自省不是传输错误。自省结果缓存 `min(exp, now+60s)`，key 建议 `sha256(token)`。
- **全局身份字段**：`active:true` 始终带布尔 `isPlatformAdmin`，服务态恒为 `false`。虽然 Portal 在自省时实时计算，下游缓存会让身份变更最多延后 60s 生效；更敏感的操作应缩短缓存或直查。
- **门户三变量 all-or-nothing**：自省地址 + SA clientId + SA secret **全缺** → 门户鉴权停用、受门路由 503；**缺一不全** → 启动 fail-fast 打印缺失变量。避免半配置静默放行。
- **租户隔离**按 `claims.tenant_id`（永不信任请求体）；限流自建 `(aud, tenant)` 每分钟窗口（参考默认 600/min）。

**三种令牌来路 × 授权姿势**：

| 来路 | kind | user_id | 授权依据 |
| --- | --- | --- | --- |
| 自己前端（宿主注入 / callService） | user | 有 | scope + PID/ACL |
| 其他 App 令牌交换（如 files→omni-parser） | user | 有 | 同上；scope=发起方 TDT ∩ 你声明的 scopes；`azp`=发起方（出处归因） |
| 服务账号 client_credentials（服务直调） | service | **无** | **只按 scope**（permissions 空、bypass=false、isPlatformAdmin=false，勿走 PID 门；用户拥有的写操作应拒绝） |

### 3.1 服务账号从哪来：平台代建 vs 租户自助

有**两条**路会产出打给你的 `client_credentials` 凭证，两条铸出来的令牌形状一样（`kind:"service"`、
`user_id:null`、`azp` = 某个 listingKey、`tenant_id` 钉死），你的闸**一个字都不用改**。区别只在谁建、
建给谁：

| | 平台代建（`SA_DEFS` / manifest `serviceScopes`） | 租户自助（租户管理员，控制台路径 `/admin/service-accounts`） |
| --- | --- | --- |
| 建的人 | 平台管理员 / 你的 manifest 声明 | **租户自己的管理员**，不经平台 |
| 典型用途 | 你的服务端调门户（files/notification/audit/seats…） | 该租户的采集器、脚本、CI **调你** |
| 租户范围 | 由平台决定（可 `all`） | **恒 allowlist=[他自己那一个租户]**，请求体里出现别的租户 id 直接 `VALIDATION_FAILED` |
| `azp` | manifest 里的 `serviceScopes.ownerAppKey` | = 租户选的那个已装 App 的 listingKey（**用户填不了、也不用填**） |
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

## 4. 健康检查

- `GET /health`（就绪）：`{"service":"<listingKey>","db":"ok","redis":"ok"|"disabled","time":<unix>}` —— `"db"` 是字符串 `"ok"` **不是** `true`；200=就绪、503=依赖挂。平台/devkit healthcheck 按此判活。
- 可另设 `GET /healthz`（存活，`{"status":"ok"}`）。

## 5. 响应信封与审计

- 业务响应统一 `{ ok, data | error }`，业务失败 HTTP 仍 200；4xx/5xx 只留传输/认证/路由层（401 缺/坏令牌、403 scope/权限、429 限流）。
- 审计：不自建审计页，关键操作用当次用户 TDT `POST {API_BASE_URL}/api/v1/audit`（需声明 `audit.write` scope），best-effort。

## 6. 自有认证面与 Portal 认证面的划界

服务自带认证体系（API-Key、节点 Token、自有 admin 会话、gRPC 端口等）时：

- 凡经 `/svc/<key>` 暴露给门户侧的 HTTP 面，一律 TDT 四道闸——不得让门户侧调用方复用你的自有认证。
- 自有节点/外部客户端面保持原认证，但**不得**从 `/svc` 暴露；`/svc` 只转发 HTTP :8080，gRPC 等额外端口在网关契约之外，需单独规划暴露并写进交付契约。
- 平台/其他 App 调你：有用户上下文走令牌交换、无用户上下文走服务账号 client_credentials（scope 落你的命名空间）——不是给平台发你体系内的 API-Key。吊销、审计、租户策略都收口在门户。
- 若有 MCP/stdio 之类**免鉴权信任面**：只限本地可信进程，发货镜像不得暴露（知识库的 `--role mcp` 规则）。

## 7. 无前端服务的租户管理员配置

- 外部团队实现 `GET/PUT /v1/settings`（四道闸；读/写门槛按 scope），存租户级覆盖，运行时 `内置默认 → 租户覆盖 → 单次请求参数` 叠加。
- 门户侧需按 App 补三件（平台方工作量，不是零改码）：`<key>-token` admin-TDT 铸造端点 + 前端 config-api + 应用配置页 Section。浏览器直连你的 `/svc/<key>/v1/settings`，门户不经手敏感配置。
