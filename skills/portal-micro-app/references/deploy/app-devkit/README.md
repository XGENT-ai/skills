# 门户一盒 · 通用 App 本地联调套件 (one-box app devkit)

让**任何外部 App 团队**(知识库、omni-parser、未来第三方)在**自己的 repo 内**用 Docker 一把起齐:
一个**真实门户**(三件套 + pg/redis/minio)+ 你自己的后端(+ 前端),跑通全链路联调 ——
**不 clone 门户 monorepo、不改门户代码、不重建门户镜像**。

> 总设计见 [`../../goal/ONE-BOX.md`](../../goal/ONE-BOX.md)。本 README 只讲**怎么把栈跑起来**。
> 知识库的个案样板见 [`../knowledge-devkit/`](../knowledge-devkit/)(本套件的泛化前身,已保留)。

## 接入只需两件事

1. **写一份 `app.manifest.json`**(门户契约,进对方 repo) —— 见 [`manifests/`](./manifests/) 里的
   `knowledge.manifest.json`(micro)与 `omni-parser.manifest.json`(service)两个样例。字段说明见 ONE-BOX §3.1。
2. **填一份 `compose.env`**(本机接线:`APP_KEY` / `APP_IMAGE` /(micro 才需的)`APP_FRONTEND_DIST`)
   —— 见 [`compose.env.app.example`](./compose.env.app.example)。

接入 = 上面两份 + 跑一个通用 `register-app` 步骤。新 App **不改门户任何代码、不重建门户镜像**。

## 两类后端 App

| | `micro`(有前端,iframe 嵌入) | `service`(无前端,仅 API) |
| --- | --- | --- |
| 样例 | 知识库 knowledge | 多模态解析器 omni-parser |
| manifest `type` | `"micro"` | `"service"` |
| 前端 | dist 挂到 `/apps/<key>/`(加 `-f docker-compose.app-frontend.yml`) | 无 |
| 可见性 | 应用市场/应用中心露卡、可打开 | **不**露卡、**不**可打开(平台控制台仍可治理) |
| 冒烟 | 走完整 iframe 链路(§A) | 走 curl(§B) |

## 你会拿到什么(平台团队交付)

1. **门户镜像**两个:`ai-portal-one-box`(portal runtime)+ `ai-portal-proxy`(caddy proxy)。两种取法二选一:
   - **私有 registry 拉取**(`docker login` 后 `docker pull`,见步骤 0)——**当前为 `arm64`**:
     ```
     crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-one-box:1.0.0   # runtime
     crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-proxy:1.0.0      # proxy
     ```
   - **离线 tar**(无 registry 访问时):平台团队用 [`scripts/pack-onebox.sh`](../../scripts/pack-onebox.sh) 产出的 `docker save` tar。
2. 本 `deploy/` 目录:`docker-compose.yml` + `app-devkit/`(本套件)+ `caddy/` + `postgres/`。

镜像里**不再**烘焙任何具体 App —— App 一律经 `register-app` 数据化注册。

---

## 端到端步骤

> 所有命令从 **repo 根目录**跑(相对路径解析自 `deploy/`,即第一个 `-f` 文件所在目录)。
> 下面以 **omni-parser(service)** 为主线;micro(knowledge)的差异在每步标注。

```bash
# 0) 载入门户镜像 —— 二选一
#    (A) 私有 registry 拉取(需先 docker login;当前镜像为 arm64)
docker login crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com
docker pull  crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-one-box:1.0.0
docker pull  crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-proxy:1.0.0
#    (B) 或离线 tar(无 registry 访问时)
# docker load < xgent-ai-portal.tar.gz
# docker load < xgent-ai-portal-proxy.tar.gz
docker load < omni-parser-server.tar.gz          # 你自己的后端镜像(同样可改用你的 registry)

# 1) 配置 env:基础模板 + 本套件增量
cp deploy/compose.env.example deploy/compose.env
cat deploy/app-devkit/compose.env.app.example >> deploy/compose.env
#    编辑 deploy/compose.env:
#    - XGENT_IMAGE       = crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-one-box:1.0.0
#    - XGENT_PROXY_IMAGE = crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-proxy:1.0.0
#      (或你 docker load 的本地 tag)
#    - APP_KEY                         = 你的 manifest listingKey(如 omni-parser)
#    - APP_IMAGE                       = 你的后端镜像 tag
#    - (micro 才需) APP_FRONTEND_DIST   = 你前端 dist 的【绝对路径】
#    - 你后端读的 SA 密钥 / DB DSN 等(与 manifest 的 serviceAccount.secret 保持相等)
#    - (强随机) SESSION_SECRET / TDT_SIGNING_KEY 等门户基础密钥

# 2) 起本地基础设施(pg/redis/minio)
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  --profile local-infra up -d postgres redis minio

# 3) 门户库迁移 + 门户基线 seed(只建门户世界:租户/用户/内置 App,不含你的 App)
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  run --rm portal-api bun run db:migrate
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  run --rm portal-api bun run db:seed

# 4) ★ 通用注册:读 manifest → 注册 listing + 服务账号 + 写 /svc 放行 map。
#    用一次性容器跑,把 manifests/ 临时挂进 /devkit。注册在「起反代之前」跑,
#    反代启动时 import .../*.map 即读到——无需热重载。
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  run --rm -v "$(pwd)/deploy/app-devkit/manifests:/devkit:ro" \
  portal-api bun run register-app /devkit/omni-parser.manifest.json

# 5) 起门户三件套 + 你的 app-backend(网络别名 <key>-server:8080)
#    service:base + app-dev,profile local-infra + app-external
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml \
  -f deploy/app-devkit/docker-compose.app-dev.yml \
  --profile local-infra --profile app-external \
  up -d reverse-proxy portal-api app-backend
#    micro:额外加 -f deploy/app-devkit/docker-compose.app-frontend.yml(挂前端 dist)

# 6) 你自己的库迁移 / 每租户 bootstrap(命令以你镜像为准)
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml -f deploy/app-devkit/docker-compose.app-dev.yml \
  --profile app-external exec app-backend <你的迁移命令>

# 7) 冒烟(见下)
```

### micro(knowledge)与 service 的命令差异

micro 多一个前端 override，并随安装走完整 iframe 链路:

```bash
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml \
  -f deploy/app-devkit/docker-compose.app-dev.yml \
  -f deploy/app-devkit/docker-compose.app-frontend.yml \
  --profile local-infra --profile app-external \
  up -d reverse-proxy portal-api app-backend
```

### App 自带的基础设施(按需)

App 若依赖额外服务(知识库的 chroma 向量库、omni-parser 的对象存储),自带一个**极小的**
app 专属 override 起它,例如:

```yaml
# deploy/app-devkit/examples/knowledge-infra.yml
services:
  chroma:
    image: chromadb/chroma:latest
    restart: unless-stopped
    expose: ["8000"]
    volumes: [chroma-data:/data]
volumes:
  chroma-data:
```

再在第 5 步追加 `-f deploy/app-devkit/examples/knowledge-infra.yml`。门户那坨**不动**。
(知识库的完整带 chroma 样板见 [`../knowledge-devkit/`](../knowledge-devkit/)。)

### 依赖**内置后端 App**(如 `files` 文件管理)

`files` 是门户**内置 App**,**已打包进 one-box runtime 镜像**(与 portal-api 同一个镜像,靠 compose
`command` 选择跑哪个后端),**不需要 manifest / register-app**:它的市场清单 + 服务账号由 `db:seed` 直接种,
`/svc/files` 路由已在 Caddyfile 白名单内。knowledge 经市场安装时会**连带补装 files** 并接上 `knowledge→files`
令牌交换。但要让 files **后端真正可用**(否则读文件 502/401),需把它**起起来**——它走 base compose 的
`app-files` profile,用**同一个** `XGENT_IMAGE`:

```bash
# 5b) 起 files 后端(同镜像,compose command 选择 files-server;/svc/files 已放行)
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml -f deploy/app-devkit/docker-compose.app-dev.yml \
  --profile local-infra --profile app-files up -d files-server

# files 库迁移(xgent-files 库由 local-infra 的 postgres init 自动建)
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  run --rm portal-api bun run db:files:migrate
```

在 `compose.env` 里补/对齐 files 段(`compose.env.example` 多数已有默认):

```
FILES_DATABASE_URL=postgres://postgres:postgres@postgres:5432/xgent-files
PORTAL_INTROSPECT_URL=http://portal-api:3000/api/tokens/introspect
FILES_SA_CLIENT_ID=files-server
FILES_RESOURCE_KEY=files-dev-resource-key       # 必须 == db:seed 种的 files SA secret
FILES_SA_CLIENT_SECRET=files-dev-resource-key   # 与上等值,否则自省 401
FILES_ENC_KEY=<openssl rand -hex 32>            # 任意 ≥16 位强随机
FILES_AUDIENCE=files
MINIO_ENDPOINT=http://minio:9000  MINIO_REGION=us-east-1
MINIO_ACCESS_KEY=minioadmin  MINIO_SECRET_KEY=minioadmin  MINIO_BUCKET=xgent-files
```

> 同理,其它内置后端(`spms`/`sms`/`qbank`/`lms`/`llm-gateway`)都在**同一镜像**里,各有 `app-<key>` profile +
> `xgent-<key>` 库(postgres init 已建)+ 一段 `*_SA_*` 凭证。**只有外部 App(omni-parser 等)才走 manifest +
> register-app**;内置 App 起对应 profile 即可。

---

## 冒烟(验收主路径)

打开 `http://localhost/` → dev 登录 → 选 `rockie`(晨光教育 admin + 平台管理员)。

### A. `micro` · 有前端(knowledge)

1. 应用市场 → 安装「知识库」→ 连带补装 `files`;`knowledge→files` 交换接线。
2. 应用中心打开知识库 → iframe 加载 `/apps/knowledge/`,`sdk.ready()` 握手成功。
3. 前端调后端:`sdk.callService("knowledge","/api/...")` 通 → 宿主代理 + 四道闸 + 自省全链路 OK。
4. 读文件 → 首次弹交换授权页,同意后 `knowledge→files` 交换生效
   (需 knowledge 的 initiator App Secret 已就位,见排查表)。

### B. `service` · 无前端(omni-parser,无 UI 走 curl)

1. **不**在应用市场/应用中心露卡(service 类型对用户隐藏);平台控制台「清单管理」可见可治理。
2. `/health` 判活:`curl http://localhost/svc/omni-parser/health` → `{"service":"omni-parser","db":"ok",...}`。
3. 缺/错 token:`curl -X POST .../svc/omni-parser/v1/parse` → 401/403(四道闸)。
4. 拿一个 `aud=omni-parser` 的 TDT(经 knowledge 交换,或 dev 自助 `/api/tokens/authorize`→`/oauth/token`)→ 正确 → 200 信封。

---

## 排查速查

| 症状 | 多半原因 |
| --- | --- |
| `/svc/<key>` 404 | `caddy-svc-allow/<key>.map` 没写成 / 反代在写 map **之前**就起了(先跑第 4 步 register-app 再起反代;或 `docker compose ... exec reverse-proxy caddy reload`) |
| `/svc/<key>` 502 | `app-backend` 没起 / crash(`docker compose ... logs app-backend`);或没听 8080;或 `APP_KEY` 与 manifest `listingKey` 不一致,网络别名 `<key>-server` 没命中 |
| iframe 404 / 空白 | `APP_FRONTEND_DIST` 不对(须绝对路径且含 `index.html`),或 `service` 类型却配了 embedUrl/前端 override |
| 自省 401 | SA Basic 不对:manifest `serviceAccount.secret` 与后端实读的 `*_SA_CLIENT_SECRET` 不相等 |
| 有效 TDT 被判 `INVALID_TOKEN` | 后端没解包自省信封 `claims = body.data ?? body` |
| `EXCHANGE_NOT_ALLOWED` | 来源 listing 没声明 `exchangeTargets` / 目标没安装 / 没建 grant / 用户没同意 |
| `knowledge→files` 交换 401(initiator) | 来源 App Secret 尚未写入:appSecrets 绑定到**已安装**实例。**先安装** knowledge,**再跑一次** `register-app knowledge.manifest.json`(幂等)即可把 `exchangeInitiatorSecret` 写进该实例 |
| `register-app` 报 `VALIDATION_FAILED`(scope) | manifest 声明了别的 App 的 scope 但没把它列进 `exchangeTargets`(ONE-BOX §3.5 收紧) |
| `db:seed` 后浏览器要重登 | seed 重生成 UUID,会话失效,重 dev 登录 |

## 注意

- 这是 **dev** 联调套件(`DEV_MOCK_OAUTH=true` + 已知明文 SA 密钥),**不要**用于生产。
  生产走 `bootstrap:prod` + 按需部署(deploy-controller),见 [`../../goal/DEPLOY.md`](../../goal/DEPLOY.md)。
- 本套件里你的 App backend 是**常驻**服务(`--profile app-external`),不经 deploy-controller 按需拉起。
