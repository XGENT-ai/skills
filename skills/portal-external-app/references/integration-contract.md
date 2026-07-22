# 外部镜像服务类应用 · 集成契约

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §7/§15 与已接入案例（知识库 / omni-parser，2026-07）。冲突时以门户仓库为准。

## 1. 形态与分界

外部镜像 App = 服务端在自己的 repo（任意语言/栈）、独立 Docker 镜像交付。与门户内建 App 的**运行时契约完全一致**（四道闸、统一 :8080、`/svc/<key>` 路由、健康信封），差异只在注册与部署：

| | 内建 App | 外部镜像 App |
| --- | --- | --- |
| 镜像 | 全部长在同一 runtime 镜像 | 独立镜像（`docker save` tar / 私有 registry，不发公共 dockerhub） |
| 部署 | deploy-controller 按需拉起/缩零 | **常驻**（controller 不编排外部镜像）：独立 Deployment / compose profile，无状态可多副本 |
| 注册 | seed / bootstrap 内建 | `app.manifest.json` + register-app（dev）/ provisioning 登记（生产） |

**App 类型**：有用户前端 → `micro`（前端 dist 由平台同源托管到 `/apps/<key>/`）；无前端 → **`service`**（headless：应用市场/应用中心不露卡、不可打开，仅作被调用方；平台控制台清单管理仍可治理，可作依赖被补装）。

## 2. 交付物（外部团队 → 平台）

1. **后端镜像**：容器内监听 **8080**（可由 env 覆盖，编排统一注 8080）；服务名/网络别名 **`<listingKey>-server`**（通用反代规则按此 DNS 命中）；多架构 **amd64 + arm64**（生产集群 amd64）；**不烘焙 `.env`/密钥**。
2. **`app.manifest.json`**（见 registration-and-onebox.md）。
3. **env 契约表**：逐个列镜像**实际读取**的变量名。⚠️ 镜像内部若用自有前缀（知识库读 `XGENT_PG_DSN`，门户契约别名 `KNOWLEDGE_DATABASE_URL`；且**不读裸 `PORT`**），必须写清映射，编排按镜像认的名字注入。
4. **运维口径**（没有默认答案，必须显式声明）：
   - DB 迁移由谁跑：镜像启动自迁（omni-parser 式）或外部命令（知识库 `scripts/migrate.sh up` 式）；
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
    "tenant_id", "user_id", "scopes": [], "role", "bypass", "groups",
    "permissions": [{ "pid", "scope" }], "aclStamp", "exp" } }
```

1. **身份**：`(claims.listingKey ?? claims.aud) === 你的 listingKey`，否则 401 `INVALID_TOKEN`；
2. **scope** ∈ `claims.scopes`，否则 403 `INSUFFICIENT_SCOPE`；
3. 结构性管理操作看 `claims.role === "admin"`；
4. 细粒度看 `bypass || permissions 命中 PID`（纯 scope 鉴权的服务可 `aclManifest:null`，跳过此闸）。

硬形状（外部实现最常炸的三处）：

- **信封解包**：声明在 `data` 里，`claims = body.data ?? body`。裸读顶层 `active` → 一切有效 TDT 被判 401（知识库 1.0.0 真实事故）。`active:false` 是成功的自省不是传输错误。自省结果缓存 `min(exp, now+60s)`，key 建议 `sha256(token)`。
- **门户三变量 all-or-nothing**：自省地址 + SA clientId + SA secret **全缺** → 门户鉴权停用、受门路由 503；**缺一不全** → 启动 fail-fast 打印缺失变量。避免半配置静默放行。
- **租户隔离**按 `claims.tenant_id`（永不信任请求体）；限流自建 `(aud, tenant)` 每分钟窗口（参考默认 600/min）。

**三种令牌来路 × 授权姿势**：

| 来路 | kind | user_id | 授权依据 |
| --- | --- | --- | --- |
| 自己前端（宿主注入 / callService） | user | 有 | scope + PID/ACL |
| 其他 App 令牌交换（如 files→omni-parser） | user | 有 | 同上；scope=发起方 TDT ∩ 你声明的 scopes；`azp`=发起方（出处归因） |
| 服务账号 client_credentials（服务直调） | service | **无** | **只按 scope**（permissions 空、bypass=false，勿走 PID 门；用户拥有的写操作应拒绝） |

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
