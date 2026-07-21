# 外部镜像应用随门户部署（知识库 knowledge · 全模态解析 omni-parser）

> **定位**：门户的大多数 App 后端是 **monorepo 内**的同一 `runtime` 镜像、靠 `bun --filter @xgent/<key>-server` 选择启动哪个（见 [`docker.md`](docker.md) / [`kubesphere.md`](kubesphere.md)）。
> **知识库（`knowledge`）**与**全模态解析（`omni-parser`，多模态文档解析器）**不一样：它们由**其它团队**以**外部 Docker 镜像**交付，**不在本仓库**、**不参与 `build:apps`**、**不进 `runtime` 镜像**，因此 `appBackends`/`bun --filter` 那条路对它们不适用。本文专门讲**怎么把这两个外部镜像随门户一并部署**（Compose 单机 + K8s 生产)。
>
> 配套文档分工：
> - 交给**对方团队**的运行时契约：[`knowledge-app-contract.md`](../knowledge-app-contract.md)、[`omni-parser对接要求.md`](../omni-parser对接要求.md)、[`omni-parser-integration.md`](../omni-parser-integration.md)。
> - **本地联调**：[`../../deploy/app-devkit/`](../../deploy/app-devkit/)（通用一盒，任意 App）、[`../../deploy/knowledge-devkit/`](../../deploy/knowledge-devkit/)（知识库样板）。
> - **本文** = 面向**运维/部署方**的「生产把外部镜像接上门户」指南。

---

## 0. 这两个 App 与内建 App 的本质差别

| | 内建 App（files/spms/sms/qbank/lms/llm-gateway/task） | 外部 App（knowledge / omni-parser） |
| --- | --- | --- |
| 镜像 | 门户 `runtime` 镜像，`bun --filter @xgent/<key>-server start:prod` | **对方交付的独立镜像**（knowledge=Zig、omni-parser=Rust 等），自带运行时 |
| 进 `build:apps` / `runtime` 镜像 | 是 | **否** |
| Helm chart `appBackends` 模板 | 适用（共用 `runtime` 镜像 + `bun` 命令） | **不适用**（必须用对方镜像 + 对方启动命令）→ 用**独立 Deployment/Service**接 |
| 按需部署（deploy-controller scale 0→1 / profile up） | 适用（在 `DEPLOYABLE_APP_SERVICES` 里） | **不适用**——controller 跑的是 monorepo 脚本，管不了外部镜像 → 这两个是**常驻**服务 |
| DB 迁移 | 由 controller/init Job 跑门户脚本 | **镜像自迁移**（启动自跑，或对方给的带外命令）——门户不替它迁 |
| 门户侧登记（listing / 服务账号 / `/svc` 路由 / 令牌交换） | `bootstrap:prod` 一把生成 | **同样随 `bootstrap:prod` 一把生成**——knowledge 与 omni-parser 都已进生产目录（§1 对照表） |

**它们要插进门户的「插槽契约」（两个外部镜像都必须满足）**：

1. **容器/服务名 = `<key>-server`、容器内监听 `8080`**。反代只有一条通用规则 `/svc/<key>/* → <key>-server:8080`（[`deploy/caddy/Caddyfile`](../../deploy/caddy/Caddyfile)），名字/端口不对就路由不到。
2. **`GET /health`** 返回门户判活口径的信封：`{"service":"<key>","db":"ok|down","redis":"disabled|ok","time":<unix>}`（200=就绪）。⚠️ 是 `"db":"ok"`，**不是** `"db":true`。
3. **镜像不含 `.env`**，全部配置运行时注入（§3.2 / §4.3 的 env 契约）。
4. **验门户令牌（TDT）走自省**：`POST {PORTAL_INTROSPECT_URL}` + 服务账号 `Basic`，解包 `{ok,data}` 信封后做 aud/scope（/PID）四道闸 + 按 `tenant_id` 租户隔离。这一切由**对方镜像实现**，门户只发自省；运维只需把**自省 URL + 服务账号凭据**注进去。
5. **同网络/同命名空间**：后端与反代必须互通（Compose 默认网络 / K8s 同 namespace），自省要能回连 `portal-api`。
6. **镜像交付**：`docker save` tar（校 sha256）或私有 registry；**生产必须 amd64**（KubeSphere 节点是 amd64；只出 arm64 联调镜像上不了生产）。

---

## 1. 门户侧登记状态对照（决定运维要不要额外动门户）

| 登记项 | knowledge | omni-parser |
| --- | --- | --- |
| 市场 listing | ✅ `bootstrap:prod` 自动建（`provisioning.ts` `LISTING_DEFS.knowledge`） | ✅ `bootstrap:prod` 自动建（`LISTING_DEFS["omni-parser"]`，`type:"service"` headless） |
| 服务账号（`token.introspect`） | ✅ `SA_DEFS.knowledge`（clientId=`knowledge-server`） | ✅ `SA_DEFS["omni-parser"]`（clientId=`omni-parser-server`） |
| `/svc/<key>` 反代路由 | ✅ Caddyfile **已内联** `knowledge "1"` | ✅ Caddyfile **已内联** `omni-parser "1"` |
| 令牌交换接线 | ✅ `knowledge→files`（`EXCHANGE_WIRING`） | ✅ 作为被调方：`files→omni-parser`（`EXCHANGE_WIRING.files`，§4.6） |
| 前端静态托管 | `/apps/knowledge/`（对方交付 dist，§3.3/§3.4） | 无前端（headless） |
| 生产可直接部署？ | **可以**——只差「跑起对方镜像 + 注密钥 + Chroma + 库」 | **可以**——只差「跑起对方镜像 + 注密钥 + 建库 + 安装实例」 |

> **一句话**：两个 App 的门户侧登记**生产都已就绪**（随 `bootstrap:prod` 烘焙：listing + 服务账号 + `/svc` 路由 + 令牌交换接线）。运维只需「跑起对方镜像 + 注密钥 + 建库（+ knowledge 还要 Chroma/前端）」，照 §3 / §4 即可，**无需再改门户代码或重建镜像**。

---

## 2. 镜像获取（两者通用）

对方以下面任一方式交付，**都不上 docker hub**：

```bash
# (A) 离线 tar：校验完整性后载入
shasum -a 256 -c knowledge-server-<arch>.tar.gz.sha256
gunzip -c knowledge-server-<arch>.tar.gz | docker load     # → 载入对方 tag

# (B) 私有 registry：docker login 后 pull（K8s 节点同样需要 imagePullSecret）
docker pull <registry>/knowledge-server:<tag>
docker pull <registry>/omni-parser-server:<tag>
```

K8s 拉私有镜像先建 pull secret（同 [`kubesphere.md`](kubesphere.md) §3）：

```bash
kubectl -n xgent-prod create secret docker-registry ext-pull \
  --docker-server=<registry> --docker-username=<u> --docker-password=<p>
```

---

## 3. 知识库（knowledge）部署

知识库是 **`micro` 型外部 App**（有前端、应用市场露卡、iframe 嵌入）。门户侧登记已随 `bootstrap:prod` 完成，运维要做的是把对方的 `knowledge-server` + 前端跑起来并接上。

### 3.1 它依赖什么

| 依赖 | 说明 | 谁 provision |
| --- | --- | --- |
| 自己的 Postgres 库 `xgent-knowledge` | 业务表 | 平台（local-infra init 已建；外部 PG 手动建库） |
| 向量库 Chroma | `chromadb/chroma` | 平台（Compose 一并起 / K8s 一个 Deployment+Service+PVC） |
| 嵌入 key | `XGENT_OPENAI_KEY`（直连 OpenAI，未来切 llm-gateway） | 运维注密钥 |
| 文件管理（files） | 经令牌交换只读/写文件；**安装 knowledge 会自动连带安装 files** | 门户（`bootstrap:prod` 已接 `knowledge→files`） |
| 前端 dist | 对方 `bun run build` 产物，同源托管在 `/apps/knowledge/` | 运维挂载（Compose 卷 / K8s 经 apps-data） |
| Redis | 可选，自省缓存/限流 | 复用平台 Redis 或省略 |

### 3.2 env 契约（镜像实读 `XGENT_*`；对照 [`knowledge-app-contract.md`](../knowledge-app-contract.md) §5）

| 变量 | 必填 | 示例 / 默认 | 说明 |
| --- | --- | --- | --- |
| `XGENT_PG_DSN` | ✅ | `postgres://…@<pg>:5432/xgent-knowledge` | 自己的库（契约别名 `KNOWLEDGE_DATABASE_URL`） |
| `XGENT_CHROMA_URL` | ✅ | `http://chroma:8000` | 向量库 |
| `XGENT_OPENAI_KEY` | ✅ | `sk-…` | 嵌入 key |
| `PORTAL_INTROSPECT_URL` | ✅\* | `http://portal-api:3000/api/tokens/introspect`（容器内）/ `https://<域名>/api/tokens/introspect` | 自省端点 |
| `KNOWLEDGE_SA_CLIENT_ID` | ✅\* | `knowledge-server` | 服务账号 Basic id（= `SA_DEFS.knowledge`） |
| `KNOWLEDGE_SA_CLIENT_SECRET` | ✅\* | —（**与门户那侧一致**） | 服务账号 Basic secret |
| `KNOWLEDGE_AUDIENCE` | | `knowledge` | 接受的 TDT `aud` |
| `KNOWLEDGE_SERVER_PORT` | | `8080` | 容器/生产监听端口（反代固定打 `knowledge-server:8080`） |
| `API_BASE_URL` | | `http://portal-api:3000` | 门户 Open API（审计） |
| `PORTAL_BASE_URL` | | `https://<域名>` | 门户 web origin（CORS） |
| `REDIS_CONN_STRING` | | — | 可选 |

\* 三个自省变量是 **all-or-nothing**：全缺 → 鉴权停用（被闸路由 503）；缺一不全 → **拒绝启动并打印缺哪个**。

**两个密钥必须两侧一致，否则跨应用静默失效**（app-secret 漂移 = `SECRET_INVALID`，常被误报成「跨应用授权缺失或未开启」）：

- `KNOWLEDGE_SA_CLIENT_SECRET`（knowledge-server 侧）必须 = 门户 `bootstrap:prod` 为该服务账号设的 secret。`bootstrap:prod` 取 `KNOWLEDGE_SA_CLIENT_SECRET` → `KNOWLEDGE_RESOURCE_KEY`，都没配则**随机生成并一次性打印**——把它回填进两侧。
- knowledge 做 `knowledge→files` 令牌交换用的 **App Secret**：门户侧 `KNOWLEDGE_APP_SECRET`（注进 `portal-api` + `deploy-controller`/init）与 knowledge-server 侧实际持有的 App Secret 必须相等，否则换 files token 报 `SECRET_INVALID`（前端表现为「跨应用授权缺失或未开启」）。

### 3.3 Compose 部署（单机 / 演示）

知识库要在 [`deploy/docker-compose.yml`](../../deploy/docker-compose.yml) 之外**追加两段**：`chroma` + `knowledge-server`，并给反代挂前端 dist。这正是 [`deploy/knowledge-devkit/docker-compose.knowledge-dev.yml`](../../deploy/knowledge-devkit/docker-compose.knowledge-dev.yml) 干的事——**生产沿用同一 override，只是 env 换成 `bootstrap:prod` 路径（不开 `DEV_MOCK_OAUTH`、用强随机密钥）**。

override 片段（生产形态，按需精简自 devkit 文件）：

```yaml
# deploy/external/knowledge.compose.yml —— 叠加在 deploy/docker-compose.yml 之上
services:
  chroma:
    image: chromadb/chroma:latest
    restart: unless-stopped
    profiles: ["app-knowledge"]
    expose: ["8000"]
    volumes: [chroma-data:/data]

  knowledge-server:
    image: ${KNOWLEDGE_IMAGE:?set KNOWLEDGE_IMAGE}     # 你 load/pull 进来的对方镜像
    restart: unless-stopped
    profiles: ["app-knowledge"]
    depends_on: [chroma]
    env_file: [./compose.env]
    environment:
      KNOWLEDGE_SERVER_PORT: "8080"                    # 反代固定打 knowledge-server:8080
    expose: ["8080"]
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://localhost:8080/health | grep -q '\"db\":\"ok\"' || exit 1"]
      interval: 15s
      timeout: 5s
      retries: 5
      start_period: 20s

  reverse-proxy:
    volumes:
      - ${KNOWLEDGE_FRONTEND_DIST:?absolute path to built dist}:/srv/www/apps/knowledge:ro

volumes:
  chroma-data:
```

部署步骤（接 [`docker.md`](docker.md) §5 之后；知识库是常驻，要**显式带** `--profile app-knowledge`）：

```bash
# 0) 外部 PG 时手动建库（local-infra 的 init 已自动建 xgent-knowledge，可跳过）
#    psql "$PG_ADMIN_URL" -c 'CREATE DATABASE "xgent-knowledge"'

# 1) compose.env 追加 knowledge 段（KNOWLEDGE_IMAGE / KNOWLEDGE_FRONTEND_DIST /
#    KNOWLEDGE_SA_CLIENT_SECRET / KNOWLEDGE_APP_SECRET / XGENT_PG_DSN / XGENT_CHROMA_URL /
#    XGENT_OPENAI_KEY / PORTAL_INTROSPECT_URL=http://portal-api:3000/...）

# 2) 起 chroma + knowledge-server（+ 反代已挂前端 dist）
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml -f deploy/external/knowledge.compose.yml \
  --profile app-knowledge up -d chroma knowledge-server reverse-proxy

# 3) 跑对方的库迁移（命令以对方镜像为准；knowledge 不自动迁移）
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml -f deploy/external/knowledge.compose.yml \
  exec knowledge-server <对方迁移命令>     # 如 scripts/migrate.sh up
```

### 3.4 K8s 部署（生产）

Helm chart 的 `appBackends` 模板写死了 `runtime` 镜像 + `bun --filter`，**接不了外部镜像**。所以把 `knowledge-server` 和 `chroma` 作为**独立清单**部进同一 namespace（`xgent-prod`）：Service 名 `knowledge-server`、端口 `8080`，反代的通用 `/svc/knowledge`（已内联白名单）即经集群内 DNS 解析到它。

```yaml
# knowledge.k8s.yaml —— kubectl apply -n xgent-prod -f
apiVersion: v1
kind: Secret
metadata: { name: knowledge-secret }
type: Opaque
stringData:
  XGENT_PG_DSN: "postgres://…/xgent-knowledge"
  XGENT_CHROMA_URL: "http://chroma:8000"
  XGENT_OPENAI_KEY: "sk-…"
  PORTAL_INTROSPECT_URL: "http://portal-api.xgent-prod.svc.cluster.local:3000/api/tokens/introspect"
  KNOWLEDGE_SA_CLIENT_ID: "knowledge-server"
  KNOWLEDGE_SA_CLIENT_SECRET: "<= 门户 bootstrap:prod 设的同值>"
  API_BASE_URL: "http://portal-api.xgent-prod.svc.cluster.local:3000"
  PORTAL_BASE_URL: "https://<域名>"
---
apiVersion: apps/v1
kind: Deployment
metadata: { name: knowledge-server, labels: { xgent.io/app-backend: "true" } }
spec:
  replicas: 1                       # 外部 App 常驻（不参与 controller 的 scale-to-zero）
  selector: { matchLabels: { app: knowledge-server } }
  template:
    metadata: { labels: { app: knowledge-server, xgent.io/app-backend: "true" } }
    spec:
      enableServiceLinks: false     # 同内建后端：避免 KNOWLEDGE_SERVER_PORT 被注成 tcp://… (KubeSphere §11)
      imagePullSecrets: [{ name: ext-pull }]
      containers:
        - name: knowledge-server
          image: <registry>/knowledge-server:<tag>
          env:
            - { name: KNOWLEDGE_SERVER_PORT, value: "8080" }
          envFrom: [{ secretRef: { name: knowledge-secret } }]
          ports: [{ name: http, containerPort: 8080 }]
          readinessProbe: { httpGet: { path: /health, port: http }, initialDelaySeconds: 5, periodSeconds: 10 }
          livenessProbe:  { httpGet: { path: /health, port: http }, initialDelaySeconds: 20, periodSeconds: 20 }
          # securityContext：用「对方镜像里的非 root 用户」的数字 uid（非 bun 镜像，未必是 1000）；
          # 不确定就先不设 runAsNonRoot，待对方给出镜像用户后收紧。
---
apiVersion: v1
kind: Service
metadata: { name: knowledge-server }
spec:
  selector: { app: knowledge-server }
  ports: [{ name: http, port: 8080, targetPort: http }]
```

`chroma` 同样一份 Deployment+Service（`chromadb/chroma`，Service 名 `chroma:8000`，挂 PVC 到 `/data`）。

**前端 dist 同源托管**：proxy 必须在 `/apps/knowledge/` 提供对方前端（shell CSP `frame-src 'self'` 要求同源）。两条路：① **不重建镜像**——把 dist 放进 proxy 与 portal-api 共享的 `apps-data` 卷的 `/srv/www/apps/knowledge/`（Compose 即 devkit 的直挂卷；K8s 需给该卷配 RWX PVC 或 initContainer 注入 dist）；② proxy 镜像**构建期烘焙**对方 dist。单机用 ①（devkit 已是此形态）；K8s 按集群存储能力二选一。

### 3.5 安装与每租户开通

1. 租户管理员**应用市场 → 安装「知识库」** → 门户连带装 files，并自动接 `knowledge→files` 令牌交换（grant + files 白名单 + knowledge 的 App Secret）。
2. **每租户首次写前**：knowledge-server 用 TDT 的 `tenant_id`（门户租户 UUID）**verbatim、不自动建**。要先插一行 `tenants(id=门户租户UUID, slug)`，否则首次写 FK 报错（[`knowledge-app-contract.md`](../knowledge-app-contract.md) §7）：
   ```sql
   INSERT INTO tenants (id, slug) VALUES ('<portal-tenant-uuid>','<slug>') ON CONFLICT (id) DO NOTHING;
   ```
3. 打开应用中心 → iframe 加载 `/apps/knowledge/` → `sdk.callService("knowledge", …)` 经宿主代理 → 自省 → 四道闸全链路。

### 3.6 冒烟

```bash
curl -s http://localhost/svc/knowledge/health            # {"service":"knowledge","db":"ok",...}
curl -s -X POST http://localhost/svc/knowledge/v1/kb/query \
  -H "authorization: Bearer <aud=knowledge TDT>" -H 'content-type: application/json' \
  -d '{"query":"hello","top_k":5}'                        # 200；缺/错 token → 401/403
```

---

## 4. 全模态解析（omni-parser）部署

omni-parser 是 **`service` 型外部 App**：**无前端**、不露卡、用户不直接访问；被**文件管理（files 的 文件处理器/预览渲染，及未来知识库）**在解析/取文本时经令牌交换调用。它**有状态但无桶**——交付的 SSO 镜像是 **url-only**（输入是 files 预签名 URL，不落字节），结果是 JSON 存 Postgres，故**多副本无状态 + Postgres** 即可（[`omni-parser-integration.md`](../omni-parser-integration.md) §4.6）。

### 4.1 它依赖什么

| 依赖 | 说明 |
| --- | --- |
| 自己的 Postgres 库 `xgent-omni-parser` | 任务 + 产物 JSON；`FOR UPDATE SKIP LOCKED` 出队，多副本 worker 安全（local-infra init 已自动建库） |
| 自省 + 服务账号 | 同 §0 契约 |
| files（输入侧 + 调用方） | files 是它的令牌交换消费方；输入是 files 预签名下载 URL（url 自带签名，无需 TDT）；约束：URL TTL 要盖住「排队+执行」时长 |
| 对象存储桶 | **不需要**（url-only）。仅 `OMNI_ALLOW_UPLOADS=true`（独立/CLI）才用，门户部署用不上 |

### 4.2 门户侧登记（已随 `bootstrap:prod` 烘焙——无需再改门户代码）

omni-parser 现在和 knowledge 一样是**生产一等公民**——`bootstrap:prod` 自动完成全部门户侧登记（见 `apps/api/src/db/provisioning.ts`）：

- **scope**：`omni_parser.read` / `omni_parser.parse`（下划线命名空间，`packages/shared/src/scopes.ts`）。
- **listing**：`LISTING_DEFS["omni-parser"]` —— `type:"service"`、`aclManifest:null`、无 `embedUrl`/`navItems`，对用户隐藏；`bootstrap:prod` 自动建（已验证：`type=service` / `embedUrl=null` / `serviceBaseUrl=/svc/omni-parser`）。
- **服务账号**：`SA_DEFS["omni-parser"]`（clientId `omni-parser-server`，capability `token.introspect`）；secret 取 `OMNI_PARSER_SA_CLIENT_SECRET` → `OMNI_PARSER_RESOURCE_KEY`，都没配则随机生成、`bootstrap:prod` 一次性打印。
- **反代路由**：[`deploy/caddy/Caddyfile`](../../deploy/caddy/Caddyfile) 已内联 `omni-parser "1"`（§4.5）。
- **令牌交换接线**：`files → omni-parser`（`EXCHANGE_WIRING.files`）；files listing 已带 `omni_parser.*` scope + `exchangeTargets:["omni-parser"]`（§4.6）。

> 所以运维侧**只剩跑镜像 + 注密钥 + 建库 + 在租户里安装实例**（§4.3–§4.7），不再需要 `register-app`（那是 dev-only、生产拒跑）或任何门户代码改动。
>
> **安装 omni-parser 实例**：它是隐藏的 `service` App，不在应用市场露卡。两条路把它装进租户：① 平台管理员在控制台「清单管理 / 应用」给目标租户安装（service App 可作治理对象安装）；② 作为依赖被携带安装。装上后 `files → omni-parser` 的 grant 在 files/omni-parser 都就位时自动接通（安装期 `wireExchangeTargets` + 部署期 `provisionTenantPortalWiring` 两条幂等路径）。**未安装 / 后端未起时 files 预览的「文本提取」会优雅降级**（`unreachable` 回退），不影响其它功能。

### 4.3 env 契约（镜像同时认平台契约名 `OMNI_PARSER_*` 与原生 `OMNI_*`）

| 变量（平台契约名） | 原生别名 | 必填 | 示例 | 说明 |
| --- | --- | --- | --- | --- |
| `OMNI_PARSER_PG_DSN` | `OMNI_DB_URL` | ✅ | `postgres://…/xgent-omni-parser` | 自己的库（生产必须 Postgres，SQLite 仅单机） |
| `PORTAL_INTROSPECT_URL` | `OMNI_PORTAL_INTROSPECT_URL` | ✅ | `http://portal-api:3000/api/tokens/introspect` | 自省端点 |
| `OMNI_PARSER_SA_CLIENT_ID` | `OMNI_SA_CLIENT_ID` | ✅\* | `omni-parser-server` | 服务账号 Basic id |
| `OMNI_PARSER_SA_CLIENT_SECRET` | `OMNI_SA_SECRET` | ✅\* | — | 服务账号 Basic secret（Secret 注入，绝不进镜像层） |
| `OMNI_PARSER_AUDIENCE` | `OMNI_LISTING_KEY` | | `omni-parser` | 接受的 TDT `aud` |
| `OMNI_PARSER_SERVER_PORT` | — | | `8080` | 容器监听端口（反代固定打 `omni-parser-server:8080`） |
| — | `OMNI_SSO_ENABLED` | | `true` | 有凭据时自动 true；缺凭据却 true → **启动期 fail-fast** |
| — | `OMNI_ALLOW_UPLOADS` | | `false` | SSO 模式默认 false → url-only，拒直传字节 |
| `API_BASE_URL` | `OMNI_PORTAL_AUDIT_URL` | | `http://portal-api:3000` | 审计 / 回拉 |
| — | `OMNI_CALLBACK_HMAC_SECRET` | 推荐 | — | 异步回调签名密钥 |
| `REDIS_CONN_STRING` | — | | — | 可选，限流/缓存/队列 |

\* 三个自省变量 all-or-nothing（同 §3.2）。env 命名细节见 [`omni-parser-integration.md`](../omni-parser-integration.md) §4.2 / [`omni-parser对接要求.md`](../omni-parser对接要求.md) §5。

> **两处密钥要对齐**（否则跨应用静默失效，同 §3.2）：① omni-parser 侧的 `OMNI_PARSER_SA_CLIENT_SECRET` 必须 = 门户 `bootstrap:prod` 为该 SA 设的 secret（缺省随机生成、一次性打印，回填两侧）；② files 作为 `files→omni-parser` 的交换发起方，其 `FILES_APP_SECRET`（files-server 侧）必须 = 门户 `provisioning.ts` 植入的 files App Secret（`FILES_APP_SECRET`，注进 `portal-api`/`deploy-controller`/init）。

### 4.4 Compose 部署

用通用 [`deploy/app-devkit/`](../../deploy/app-devkit/) override 的形态（`APP_KEY=omni-parser` + 网络别名 `omni-parser-server:8080`），或直接追加一段服务：

```yaml
# deploy/external/omni-parser.compose.yml —— 叠加在 deploy/docker-compose.yml 之上
services:
  omni-parser-server:
    image: ${OMNI_PARSER_IMAGE:?set OMNI_PARSER_IMAGE}
    restart: unless-stopped
    profiles: ["app-omni-parser"]
    env_file: [./compose.env]
    environment:
      OMNI_PARSER_SERVER_PORT: "8080"
      OMNI_SSO_ENABLED: "true"
    expose: ["8080"]
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://localhost:8080/health | grep -q '\"db\":\"ok\"' || exit 1"]
      interval: 15s
      timeout: 5s
      retries: 5
      start_period: 20s
```

```bash
#   外部 PG 时手动建库：psql "$PG_ADMIN_URL" -c 'CREATE DATABASE "xgent-omni-parser"'
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml -f deploy/external/omni-parser.compose.yml \
  --profile app-omni-parser up -d omni-parser-server
# DB 迁移由镜像启动自跑（含 postgres feature）；如对方给带外命令则照跑。
```

### 4.5 K8s 部署

同 §3.4，独立 Deployment+Service（Service 名 `omni-parser-server:8080`，`replicas` 可 >1，无状态多副本）。

**`/svc/omni-parser` 路由**：Caddyfile **已内联** `omni-parser "1"`（随 proxy 镜像），所以 K8s 下只要 namespace 里有 Service `omni-parser-server:8080`，`/svc/omni-parser` 即经集群内 DNS 解析直通——**无需** `caddy-svc-allow` 共享卷、无需重建 proxy。后端 Pod 没起时按惯例 502（懒解析），起来即通。

### 4.6 消费方接线（`files → omni-parser` 令牌交换）⚠️ scope 交集陷阱

files 用 `aud=files` 的 TDT 换一个 `aud=omni-parser` 的 TDT 来调本服务（同 `knowledge→files` 同构；[`omni-parser对接要求.md`](../omni-parser对接要求.md) §3）。门户侧全部已随 `bootstrap:prod` 接好，但有一条**铁律**值得复述：

- **交换后 scope = 消费方当前 TDT 的 scopes ∩ omni-parser 声明的 scopes**（绝不扩张）。所以 **files 的 listing 必须声明 `omni_parser.read`/`omni_parser.parse`**（已在 `LISTING_DEFS.files` + `provisionGlobal("files")` 补齐），否则交集为空 → 调 omni-parser 全 403。
- omni-parser 实例开 `allowExchange=true` + 交换白名单含 `files`；门户建 `files→omni-parser` grant；用户首次在交换授权页同意。以上由 `EXCHANGE_WIRING.files=["omni-parser"]` 在租户安装 omni-parser 时落地（`provisionTenantPortalWiring`/`wireExchangeTargets`）。
- 未来 knowledge 走 `knowledge→omni-parser` 同法接一条 grant 即可复用（本期 knowledge 的 files/omni 运行时调用 D5 暂缓，故未默认接线）。

### 4.7 冒烟

```bash
curl -s http://localhost/svc/omni-parser/health          # {"service":"omni-parser","db":"ok",...}
# 拿 files 经交换的 aud=omni-parser TDT，再调解析：
curl -s -X POST http://localhost/svc/omni-parser/v1/parse \
  -H "authorization: Bearer <aud=omni-parser TDT>" -d '{"url":"<files 预签名URL>"}'   # 200
# 缺 token → 401；scope 交集为空 → 403 INSUFFICIENT_SCOPE
```

---

## 5. 排错速查

| 现象 | 多半原因 |
| --- | --- |
| `/svc/<key>` **404** | 服务名不是 `<key>-server` / 端口非 8080；或 key 未在 Caddy 白名单（files/knowledge/omni-parser 等均已内联，§0/§4.5） |
| `/svc/<key>` **502** | 后端容器/Pod 没起或刚 crash；反代懒解析 upstream，起来即通 |
| 每个有效 TDT 都 **401 `INVALID_TOKEN`** | 镜像没解包自省 `{ok,data}` 信封（裸读顶层 `active`=undefined）——对方镜像 bug，[`omni-parser对接要求.md`](../omni-parser对接要求.md) §2.1 |
| 所有请求 **401** | aud 不符：`*_AUDIENCE` 与 TDT 的 `aud` 不一致 |
| 消费方调用全 **403 `INSUFFICIENT_SCOPE`** | scope 交集为空：消费方 listing 没声明目标 scope（files 缺 `omni_parser.*` / knowledge 缺 `files.*`，§4.6） |
| **503 `INTROSPECT_FAILED` / `AUTH_UNAVAILABLE`** | 自省 URL 配错 / 服务账号无 `token.introspect` / 三个自省变量缺一不全 |
| 「跨应用授权缺失或未开启」 | 交换发起方 App Secret 两侧不一致（`KNOWLEDGE_APP_SECRET` / `FILES_APP_SECRET` 漂移，§3.2/§4.3） |
| 知识库首次写 **FK 报错** | 没插 `tenants(id=门户租户UUID)`（§3.5 第 2 步） |
| K8s Pod 健康检查超时 / 端口变 NaN | 没设 `enableServiceLinks:false`（Service 同名 env 冲掉监听端口，KubeSphere §11） |
| `files→omni-parser` 没接通 | omni-parser 未在该租户安装（隐藏 service App，§4.2 安装）；或 omni-parser 后端未起 → files 预览文本提取优雅降级 |

---

## 6. 与其它文档的关系

- 部署主线：[`docker.md`](docker.md)（单机/Compose）、[`kubesphere.md`](kubesphere.md)（K8s 生产）——本文是其「外部镜像 App」补充。
- 运行时契约（交给对方团队）：[`knowledge-app-contract.md`](../knowledge-app-contract.md)、[`omni-parser对接要求.md`](../omni-parser对接要求.md)、[`omni-parser-integration.md`](../omni-parser-integration.md)。
- 本地联调：[`../../deploy/app-devkit/README.md`](../../deploy/app-devkit/README.md)（通用一盒）、[`../../deploy/knowledge-devkit/README.md`](../../deploy/knowledge-devkit/README.md)。
- SSO / 令牌交换机制：[`SSO与App开发指引.md`](../SSO与App开发指引.md)。
