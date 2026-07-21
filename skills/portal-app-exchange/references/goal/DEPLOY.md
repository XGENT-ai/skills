# DEPLOY · Docker 容器化部署开发计划

> 本期目标：只实现 Docker 容器化部署。交付物以 `Dockerfile`、`docker compose`、反向代理配置、生产环境变量模板、迁移/初始化脚本和验收脚本为主。暂不考虑 Kubernetes、PaaS、自动扩缩容、蓝绿/金丝雀发布和完整 CI/CD。

---

## 0. 范围收口

### 本期做

- 构建可运行生产镜像。
- 用 Docker Compose 编排 Portal API、独立 App 后端、反向代理、可选本地基础设施。
- 用反向代理统一暴露门户、微应用静态资源、API、WebSocket 和 `/svc/*` 独立后端代理。
- 支持按需启动应用服务：初始部署只启动平台服务；第一个租户安装某个市场应用时，才启动该应用对应的 Docker 服务。
- 补齐生产 `.env.example`、`deploy/compose.env.example` 和部署说明。
- 补齐根脚本：构建、平台迁移、单应用迁移、健康检查、生产初始化。
- 明确 dev seed 与生产 bootstrap 的边界。
- 生产环境禁用 dev-only 鉴权入口。

### 本期不做

- 不做 Kubernetes。
- 不做 Helm、Ingress、External Secrets、HPA 等 K8s 资源。
- 不做多区域部署。
- 不做自动扩缩容。
- 不做蓝绿/金丝雀发布。
- 不做完整 CI/CD pipeline。
- 不做云厂商 Secret Manager 集成。
- 不做完整指标平台和告警系统，只保留健康检查和 stdout 日志。

---

## 1. 当前状态判断

项目现状：

- Monorepo：`apps/*` + `packages/*`，根脚本以 `bun --filter` 管理。
- 后端：`apps/api` 是门户后端；`files/spms/sms/qbank/lms/llm-gateway` 各自有独立后端、独立数据库、独立端口。
- 前端：`web` 是门户壳；`sample/todo/files/spms/sms/qbank/lms/llm-gateway` 是 Vite 微应用。
- 基础设施：PostgreSQL、Redis、对象存储/S3 或 MinIO；LLM Gateway 还需要供应商渠道密钥。
- 运行脚本：后端已有 `start`，前端已有 `build`/`preview`。
- 部署缺口：没有 Dockerfile、没有 Compose、没有反代配置、没有生产环境变量模板、没有统一迁移编排、没有生产初始化脚本。
- 当前应用市场安装逻辑只把 `marketplace_listings` snapshot 成租户 `apps` 行；还没有部署层状态和服务启动动作。

关键约束：

- Portal Web 与 Portal API 首期必须同站点反代，降低 session cookie、WebSocket、OAuth 回调复杂度。
- 微应用首期按同域子路径部署，避免多域名带来的 CSP/CORS/cookie 复杂度。
- 独立后端只通过门户签发的 TDT 与 introspection 接入。
- 生产上必须禁用 `DEV_MOCK_OAUTH=true`，并禁用 `*_RESOURCE_KEY` 这类 dev fallback 鉴权入口。
- 所有业务库物理隔离为独立 database：`xgent-portal`、`xgent-files`、`xgent-spms`、`xgent-sms`、`xgent-qbank`、`xgent-lms`、`xgent-llm-gateway`。

---

## 2. 目标 Docker 拓扑

### 2.1 Compose 服务

| 服务 | 类型 | 说明 |
| --- | --- | --- |
| `reverse-proxy` | Caddy 或 Nginx | TLS 终止、静态资源、API 代理、WebSocket、`/svc/*` 代理 |
| `portal-api` | Bun runtime | `@xgent/api`，端口 `3000` |
| `deploy-controller` | Bun runtime | 只执行白名单 Docker Compose 操作；Portal 不直接访问 Docker socket |
| `files-server` | Bun runtime / app profile | `@xgent/files-server`，端口 `4100`，按需启动 |
| `spms-server` | Bun runtime / app profile | `@xgent/spms-server`，端口 `4200`，按需启动 |
| `sms-server` | Bun runtime / app profile | `@xgent/sms-server`，端口 `4300`，按需启动 |
| `qbank-server` | Bun runtime / app profile | `@xgent/qbank-server`，端口 `4400`，按需启动 |
| `lms-server` | Bun runtime / app profile | `@xgent/lms-server`，端口 `4500`，按需启动 |
| `llm-gateway-server` | Bun runtime / app profile | `@xgent/llm-gateway-server`，端口 `4600`，按需启动 |
| `postgres` | compose profile: `local-infra` | 本地/演示可选；生产可改外部数据库 |
| `redis` | compose profile: `local-infra` | 本地/演示可选；生产可改外部 Redis |
| `minio` | compose profile: `local-infra` | 本地/演示可选；生产可改 S3 兼容对象存储 |

首期建议使用一个应用镜像，多服务通过不同 command 启动。这样镜像构建和版本回滚更简单。

初始部署只启动：

- `reverse-proxy`
- `portal-api`
- `deploy-controller`
- `postgres` / `redis` / `minio`，如果使用 `local-infra`

应用后端不随初始部署启动。它们在 Compose 文件中声明为 profile，例如 `app-files`、`app-spms`、`app-sms`、`app-qbank`、`app-lms`、`app-llm-gateway`，由部署控制器按需拉起。

### 2.2 公网路由

首期使用单域名：

| 路径 | 目标 |
| --- | --- |
| `/` | `apps/web/dist` |
| `/api/*` | `portal-api` |
| `/auth/*` | `portal-api` |
| `/oauth/*` | `portal-api` |
| `/health` | `portal-api` |
| `/ws` | `portal-api` WebSocket |
| `/apps/files/*` | `apps/files-app/dist` |
| `/apps/spms/*` | `apps/spms-app/dist` |
| `/apps/sms/*` | `apps/sms-app/dist` |
| `/apps/qbank/*` | `apps/qbank-app/dist` |
| `/apps/lms/*` | `apps/lms-app/dist` |
| `/apps/llm-gateway/*` | `apps/llm-gateway-app/dist` |
| `/svc/files/*` | `files-server` |
| `/svc/spms/*` | `spms-server` |
| `/svc/sms/*` | `sms-server` |
| `/svc/qbank/*` | `qbank-server` |
| `/svc/lms/*` | `lms-server` |
| `/svc/llm-gateway/*` | `llm-gateway-server` |

生产前端变量使用相对路径：

- `VITE_FILES_SERVER_BASE=/svc/files`
- `VITE_SPMS_SERVER_BASE=/svc/spms`
- `VITE_SMS_SERVER_BASE=/svc/sms`
- `VITE_QBANK_SERVER_BASE=/svc/qbank`
- `VITE_LMS_SERVER_BASE=/svc/lms`
- `VITE_LLM_GATEWAY_SERVER_BASE=/svc/llm-gateway`

---

## 3. 按需应用服务部署设计

### 3.1 目标行为

初始部署后，平台只提供：

- 登录、租户、平台管理。
- 应用市场浏览和安装。
- Portal Open API、TDT、introspection、通知、任务等平台能力。

当第一个租户安装某个 marketplace listing 时：

1. `installListing()` 仍先完成租户内应用安装记录。
2. Portal 为该 listing 生成一个平台级部署任务。
3. `deploy-controller` 领取任务。
4. 控制器启动该 listing 对应的 Compose profile。
5. 控制器运行该服务的数据库迁移。
6. 控制器运行该 listing 的全局初始化 / 跨应用接线（见 3.1.1），保证服务起来后真正可用，而不仅仅是进程健康。
7. 控制器等待服务健康检查成功（注意 `llm-gateway` 是 `/healthz`，见 3.4）。
8. Portal 将该 listing 的部署状态标记为 `ready`。
9. 之后任何租户安装同一个应用，只复用已部署服务，不再重复启动容器；但每个新租户仍需一次幂等的租户级开通（见 3.1.1）。

如果部署失败：

- 租户安装记录可以保留为 `active`，但应用进入“服务部署中/部署失败”状态，App Center 打开时展示明确错误。
- 或安装事务回滚。首期建议不回滚租户安装记录，便于管理员重试部署，且不会丢失安装意图。

### 3.1.1 两层模型：服务部署（全局）与租户开通（每租户）

按需部署里其实有两件不同的事，必须分开，否则第二个租户会“装了打不开但又不报错”。

- **服务部署（service deployment）**：全局、按 listing、只做一次。包含启动容器、跑该库 schema 迁移、跑该 listing 的全局初始化与跨应用接线。状态记在 `app_service_deployments`。
- **租户开通（tenant provisioning）**：每个租户一次、幂等。包含该租户在这个 App 里的默认数据、SA 授权、consent co-grant、字典可见性等租户级初始化。

当前代码这两层是混在一起的，本期必须拆清楚：

- 跨应用 App 真正可用所依赖的 wiring，今天分散在 **dev-only 的 `db:seed:lms` / `db:seed:qbank` / `db:seed:llm-gateway`** 和 **per-tenant 的 `lms:bootstrap` / `sms:bootstrap` / `qbank:bootstrap` / `llm-gateway:bootstrap`** 里，**不在 schema 迁移内**。
- 例：qbank 读 LMS 教研字典走 token-exchange 的 consent co-grant + lms-dict proxy，这套是 seed/bootstrap 建的，不是 migrate 建的。所以“启动容器 + migrate + 健康检查”得到的服务进程是健康的，但字典下拉、跨应用读全是空的——状态 `ready` 却不可用。

本期落地要求：

- 服务部署步骤里，migrate 之后必须再跑该 listing 的**全局初始化脚本**（把现有 dev-only `db:seed:*` 中跨应用接线那部分抽成幂等的、生产可用的脚本，建议命名 `<key>:provision:global`），再判 `ready`。
- 租户开通不属于全局部署任务，由安装链路在**每个租户首次安装**时单独触发幂等的 per-tenant bootstrap；即使服务已 `ready`、是第二个租户也要跑（只是不再跑全局 migrate / `*:provision:global`）。

### 3.2 为什么不让 Portal 直接操作 Docker

`portal-api` 是对外业务服务，不应该挂载 Docker socket。否则一个 Portal RCE 就等价于宿主机 root。

本期采用本机 `deploy-controller`：

- 仅部署在受控服务器内网。
- 可以挂载 Docker socket 或调用宿主机 Docker CLI。
- 只接受 Portal DB 中的白名单任务，不暴露公网 HTTP API。
- 只允许执行预定义 listingKey 到 Compose profile 的映射。

最小实现可以是一个 Bun worker：

```text
deploy-controller
  loop:
    SELECT * FROM app_deployment_jobs WHERE status='queued' ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED
    mark running
    docker compose --profile app-<key> up -d <service>
    bun run db:<key>:migrate
    bun run <key>:provision:global          # 幂等的全局初始化 / 跨应用接线（非 per-tenant）
    wait <healthUrl>                          # 注意 llm-gateway 是 /healthz
    mark deployment ready
```

### 3.3 需要新增的数据模型

在 Portal DB 增加两张表。

`app_service_deployments`：全局 listing 级别，不按租户重复部署。

| 字段 | 说明 |
| --- | --- |
| `id` | uuid |
| `listingKey` | marketplace listing key，唯一 |
| `serviceName` | Compose service，例如 `files-server` |
| `profile` | Compose profile，例如 `app-files` |
| `status` | `not_deployed` / `deploying` / `ready` / `failed` |
| `version` | 当前部署的应用版本或镜像 tag |
| `baseUrl` | 反代路径，例如 `/svc/files` |
| `healthUrl` | 容器网络健康检查 URL |
| `lastError` | 最近失败原因 |
| `deployedAt` | 最近成功时间 |
| `updatedAt` | 更新时间 |

`app_deployment_jobs`：部署任务队列。

| 字段 | 说明 |
| --- | --- |
| `id` | uuid |
| `listingKey` | 要部署的应用 |
| `tenantId` | 触发安装的租户，审计用 |
| `appId` | 触发安装的租户应用，审计用 |
| `action` | `deploy` / `redeploy` / `migrate` |
| `status` | `queued` / `running` / `succeeded` / `failed` |
| `attempts` | 重试次数 |
| `log` | 控制器输出摘要 |
| `error` | 错误摘要 |
| `createdAt` | 创建时间 |
| `startedAt` | 开始时间 |
| `finishedAt` | 结束时间 |

幂等规则：

- `app_service_deployments.listingKey` 唯一。
- `ready` 状态下再次安装不创建部署任务。
- `deploying` 状态下再次安装只复用正在执行的任务。
- `failed` 状态下允许平台管理员或租户管理员触发重试。

### 3.4 Listing 到服务的静态注册表

不要从租户输入拼 Docker 命令。新增代码级白名单，例如：

```ts
export const DEPLOYABLE_APP_SERVICES = {
  files: {
    serviceName: "files-server",
    profile: "app-files",
    migrateScript: "db:files:migrate",
    globalProvisionScript: null,            // files 无跨应用接线
    healthUrl: "http://files-server:4100/health",
    publicBaseUrl: "/svc/files",
  },
  qbank: {
    serviceName: "qbank-server",
    profile: "app-qbank",
    migrateScript: "db:qbank:migrate",
    globalProvisionScript: "qbank:provision:global", // 依赖 LMS 字典的 consent co-grant 等
    healthUrl: "http://qbank-server:4400/health",
    publicBaseUrl: "/svc/qbank",
  },
  "llm-gateway": {
    serviceName: "llm-gateway-server",
    profile: "app-llm-gateway",
    migrateScript: "db:llm-gateway:migrate",
    globalProvisionScript: "llm-gateway:provision:global",
    healthUrl: "http://llm-gateway-server:4600/healthz", // 注意是 /healthz，不是 /health
    publicBaseUrl: "/svc/llm-gateway",
  },
  spms: { ... },
  sms: { ... },
  lms: { ... },
} as const;
```

只有在这个注册表里的 listing 才触发服务部署。`todo-app`、`sample-app` 这类无独立后端的微应用可以标记为 `staticOnly`，安装时不创建部署任务。

### 3.5 Marketplace 安装链路改造

当前入口：

- `POST /api/admin/market/install`
- `apps/api/src/modules/market/tenant.ts`
- `apps/api/src/modules/market/service.ts#installListing`

改造点：

- `installListing()` 返回目标 app 和自动安装的依赖。
- 对安装顺序里的每个 listing，调用 `ensureDeploymentQueued(listingKey, tenantId, appId)`。注意 `installListing()` / `installOne()` 现在只返回 `appKey`（`apps/api/src/modules/market/service.ts:465`），`app_deployment_jobs.appId` 这个审计字段需从 `installOne` 把 appId 带出来。
- 依赖应用先排队部署；目标应用后排队部署。
- 返回值增加部署摘要：

```ts
{
  appKey,
  reactivated,
  installedDeps,
  deployments: [
    { listingKey: "lms", status: "ready" },
    { listingKey: "qbank", status: "queued" }
  ]
}
```

前端安装成功后：

- 如果全部 `ready`，沿用当前“已安装”提示。
- 如果存在 `queued/deploying`，显示“应用已安装，服务正在部署”。
- App Center 打开该应用时，如果服务未 ready，显示部署状态页。

### 3.6 MicroAppHost 与服务未就绪处理

当前 `MicroAppHost` 的 `SERVICE_REGISTRY` 是编译期静态变量。按需部署后仍可保留静态 baseUrl，因为反代路径固定；但需要新增服务状态检查。

建议新增 Portal API：

- `GET /api/apps/:appKey/runtime`

返回：

```json
{
  "appKey": "qbank",
  "listingKey": "qbank",
  "deployment": {
    "required": true,
    "status": "deploying",
    "lastError": null
  }
}
```

Host 行为：

- `required=false`：静态/原生/平台内应用，照常打开。
- `ready`：照常 iframe。
- `queued/deploying`：显示“服务部署中”，提供刷新。
- `failed`：显示错误和“重试部署”按钮。

重试 API：

- `POST /api/admin/apps/:appKey/deployment/retry`

只允许 tenant admin 或 platform admin 调用。

### 3.7 Compose profile 设计

`deploy/docker-compose.yml` 中应用服务不默认启动：

```yaml
services:
  portal-api:
    image: ${XGENT_IMAGE}
    command: bun --filter @xgent/api start:prod   # 不带 --env-file；env 由容器环境注入

  deploy-controller:
    image: ${XGENT_IMAGE}
    command: bun run deploy:controller
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - ./docker-compose.yml:/deploy/docker-compose.yml:ro
    environment:
      COMPOSE_PROJECT_NAME: xgent
      DEPLOY_COMPOSE_FILE: /deploy/docker-compose.yml

  files-server:
    image: ${XGENT_IMAGE}
    profiles: ["app-files"]
    command: bun --filter @xgent/files-server start:prod

  spms-server:
    image: ${XGENT_IMAGE}
    profiles: ["app-spms"]
    command: bun --filter @xgent/spms-server start:prod
```

控制器执行时使用固定参数：

```bash
docker compose --env-file /deploy/compose.env --profile app-files up -d files-server
```

注意：控制器容器内要么安装 Docker CLI，要么使用 Docker Engine API。首期实现建议直接使用 Docker CLI，减少开发量。

两个落地约束：

- 单镜像多 command 意味着 Docker CLI 会进到每个 app-server 容器（files-server 等也带着），平白多出攻击面。可接受，但若想收紧，可给 controller 单独一个 slim 镜像。
- controller 跑 `bun --filter <pkg> db:<key>:migrate` / `<key>:provision:global` 依赖完整的 monorepo workspace 布局 + drizzle 迁移 SQL，runtime 镜像（Phase 3）必须保留 workspace 结构和这些脚本，不能只放编译产物。

### 3.8 初始部署与安装后的流程

初始部署：

```bash
docker compose --env-file deploy/compose.env up -d reverse-proxy portal-api deploy-controller
```

如果使用本地基础设施：

```bash
docker compose --env-file deploy/compose.env --profile local-infra up -d postgres redis minio
docker compose --env-file deploy/compose.env up -d reverse-proxy portal-api deploy-controller
```

首次安装 `qbank`：

1. 租户管理员点击安装。
2. Portal 写入 `apps` 行。
3. Portal 发现 `qbank` 依赖 `lms`，先确保 `lms` 部署任务，再确保 `qbank` 部署任务。
4. `deploy-controller` 启动 `lms-server`，跑 `db:lms:migrate`，跑 `lms:provision:global`（全局字典/接线），等待健康。
5. `deploy-controller` 启动 `qbank-server`，跑 `db:qbank:migrate`，跑 `qbank:provision:global`（建立 qbank↔lms 的 consent co-grant 等），等待健康。
6. 安装链路对**当前租户**跑一次幂等的 per-tenant bootstrap（该租户在 lms/qbank 的默认数据/授权）。
7. Portal 显示可打开。

第二个租户安装 `qbank`：

- 只新增该租户 `apps` 行。
- 复用已运行的 `lms-server` 和 `qbank-server`，不重复启动容器，不重复跑全局 migrate / `*:provision:global`。
- 但仍对**该新租户**跑一次幂等的 per-tenant bootstrap（租户级默认数据/授权），否则第二个租户会“装了打不开”。

### 3.9 卸载时是否停止服务

本期不自动停止应用服务。

原因：

- 服务是全局按 listing 部署，不属于单个租户。
- 一个租户卸载不代表其他租户未使用。
- 自动停服务需要引用计数和并发保护，容易误停。

本期只做：

- 当所有租户都卸载某应用时，在平台控制台展示“可停用服务”提示。
- 平台管理员可手动执行 `docker compose stop <service>`。

后续可以补：

- `app_service_deployments.activeTenantCount`
- 空闲 N 天自动停止
- 冷启动提示

---

## 4. 开发任务

### Phase 1 · 生产脚本与健康检查

- [ ] 根 `package.json` 增加 `build`：类型检查全部 workspace，并构建所有生产启用的 Vite app。
- [ ] 增加 `build:apps`：只构建前端。
- [ ] 增加 `db:migrate:platform`：只执行 Portal DB 迁移，用于初始部署。
- [ ] 增加 `db:migrate:app:<key>` 或单应用迁移映射：供 `deploy-controller` 在应用首次部署时调用。
- [ ] 新增 `db:migrate:all`（当前不存在，只有逐个 `db:*:migrate`）作为维护脚本，但不用于初始部署。
- [ ] 增加单应用迁移脚本映射，供 `deploy-controller` 调用。
- [ ] 增加 `health:platform`：检查 `portal-api`、`deploy-controller` 依赖项和反代。
- [ ] 增加 `health:deployed`：只检查 `app_service_deployments.status='ready'` 的应用服务。
- [ ] `health:all` 可以组合 `health:platform` + `health:deployed`，但不能要求未部署应用健康。
- [ ] 确认所有后端有 `/health`；没有的补齐。注意 `llm-gateway-server` 现在是 `/healthz`（`apps/llm-gateway-server/src/app.ts:71`），需统一成 `/health`，或在 `DEPLOYABLE_APP_SERVICES.healthUrl` 里按服务配置。
- [ ] 统一 `/health` 返回体：当前 `spms/sms/qbank/lms` 是 `ok({ status:'up', ts })`（经 `ok()` 包了一层 `{ data: … }`），`api`/`files-server` 各自定制，且都没真查 db/redis。统一最小字段 `service`、`db`、`redis`、`time`，并注意与 `ok()` 信封对齐。

平台初始迁移：

1. `bun run db:migrate:platform`（当前 `db:migrate` = `bun --filter @xgent/api db:migrate`，本就只迁 portal；`db:migrate:platform` 是它的别名/明确化）

应用首次部署时由 `deploy-controller` 按 listing 调用：

- `files` → `bun run db:files:migrate`
- `spms` → `bun run db:spms:migrate`
- `sms` → `bun run db:sms:migrate`
- `qbank` → `bun run db:qbank:migrate`
- `lms` → `bun run db:lms:migrate`
- `llm-gateway` → `bun run db:llm-gateway:migrate`

验收：

- `bun run typecheck` 通过。
- `bun run build` 通过。
- `bun run db:migrate:platform` 可在空 Portal 库初始化平台表。
- `bun run health:platform` 能发现平台服务不可用。
- `bun run health:deployed` 不检查未部署应用。

### Phase 2 · 前端子路径构建

- [ ] 为生产构建配置 Vite `base`：
  - `apps/web`：`/`
  - `apps/files-app`：`/apps/files/`
  - `apps/spms-app`：`/apps/spms/`
  - `apps/sms-app`：`/apps/sms/`
  - `apps/qbank-app`：`/apps/qbank/`
  - `apps/lms-app`：`/apps/lms/`
  - `apps/llm-gateway-app`：`/apps/llm-gateway/`
- [ ] 决定 `sample-app`、`todo-app` 是否进入生产镜像；若不进入，生产 seed 不安装它们。
- [ ] 将所有前端构建产物收集到镜像内统一目录，例如 `/srv/www`。
- [ ] 检查 iframe 深链接、公开分享页和刷新 fallback。
- [ ] 重做前端 CSP：当前 `apps/web/index.html` 与 `apps/files-app/index.html` 的 `<meta http-equiv="Content-Security-Policy">` 把 `connect-src` / `frame-src` 写死成 localhost dev 源（`localhost:3000/4100…4600`、`localhost:5174…5181`）。单域名子路径上线后 `/svc/*`、`/apps/*` 都变同源，meta 只放了 localhost，浏览器对 `meta ∩ 反代 header` 取最严交集，会直接挡掉同源的 `/svc` 与 iframe。生产必须把 meta CSP 改成环境感知（同源用 `'self'`），或移除 meta、统一由反代下发（见 Phase 5）。可复用已有的 `vite-plugin-brand` build-time 模板机制。

验收：

- 不启动 Vite dev server，也能打开 Portal。
- Portal App Center 能打开每个生产启用的微应用。
- 刷新 `/apps/*` 深链接不 404。
- 生产构建的 CSP 不含任何 `localhost` 源；`/svc/*` 与 `/apps/*` iframe 在同源下不被 CSP 拦截。

### Phase 3 · Dockerfile

- [ ] 新增根 `Dockerfile`。
- [ ] 使用多阶段构建：
  - `deps`：`bun install --frozen-lockfile`。
  - `build`：`bun run typecheck` + `bun run build`。
  - `runtime`：只复制运行所需文件、`node_modules`/Bun cache、后端源码、迁移文件和前端 dist。
- [ ] 镜像默认不内置 `.env`。
- [ ] 处理 `start` 脚本里写死的 `--env-file=../../.env`：镜像不内置 `.env`，env 由 Compose `environment:`/`env_file:` 注入容器环境。需新增不带 `--env-file` 的 `start:prod`（直接读容器 env），或确认 bun 对缺失的 env-file 是回退到 `process.env`；Compose `command` 用生产变体（见 3.7）。
- [ ] 通过 Compose command 指定启动哪个服务。
- [ ] 容器以非 root 用户运行。
- [ ] 日志只输出到 stdout/stderr。

建议命令：

```bash
docker build -t xgent-ai-portal:local .
```

验收：

- 镜像可在无源码挂载的情况下运行。
- 镜像不包含真实 `.env`。
- 前端静态资源已在镜像中。

### Phase 4 · Docker Compose 与按需 profile

- [ ] 新增 `deploy/docker-compose.yml`。
- [ ] 新增 `deploy/compose.env.example`。
- [ ] 平台服务默认启动：
  - `reverse-proxy`
  - `portal-api`
  - `deploy-controller`
- [ ] 应用服务使用同一个镜像，并配置为非默认 profile：
  - `files-server`
  - `spms-server`
  - `sms-server`
  - `qbank-server`
  - `lms-server`
  - `llm-gateway-server`
- [ ] 增加 `reverse-proxy`。
- [ ] 增加 `deploy-controller`，只允许启动白名单 profile。
- [ ] 增加 `local-infra` profile：
  - `postgres`
  - `redis`
  - `minio`
- [ ] 为每个后端服务配置 `healthcheck`。
- [ ] 配置 `depends_on` 的健康条件，避免应用早于数据库启动。
- [ ] 为 Postgres、Redis、MinIO 配置 volume。

启动命令：

```bash
docker compose --env-file deploy/compose.env up -d
docker compose --env-file deploy/compose.env --profile local-infra up -d
```

验收：

- 不指定 app profile 时，只启动平台服务。
- 第一次安装市场应用后，控制器能启动对应 app profile。
- 使用 `local-infra` profile 能在单机启动完整演示环境。
- 使用外部 DB/Redis/S3 时，可不启动 `local-infra`。
- 停止任一独立后端时，Portal 本身不崩溃。

### Phase 5 · 反向代理

- [ ] 选择 Caddy 或 Nginx；首期推荐 Caddy，TLS 和静态资源配置更少。
- [ ] 新增 `deploy/caddy/Caddyfile` 或 `deploy/nginx/xgent.conf`。
- [ ] 反代规则覆盖：
  - `/api/*`
  - `/auth/*`
  - `/oauth/*`
  - `/health`
  - `/ws`
  - `/svc/*`
- [ ] 静态资源规则覆盖：
  - Portal SPA fallback。
  - 微应用 SPA fallback。
  - gzip/zstd 或 gzip/br 压缩。
  - 长缓存 hash assets，短缓存 HTML。
- [ ] 上传相关代理参数：
  - body size 上限。
  - read/write timeout。
  - WebSocket upgrade。
- [ ] 安全响应头：
  - `X-Content-Type-Options`
  - `Referrer-Policy`
  - `Content-Security-Policy`（与 Phase 2 协调：CSP 只保留一个权威来源——要么反代下发、要么前端 meta 环境感知，避免 `meta ∩ header` 最严交集误挡同源 `/svc`、iframe）
  - `Permissions-Policy`

验收：

- 登录、WebSocket、iframe、`sdk.callService()` 全部通过反代。
- 文件上传 presign/finalize 主链路可用。
- `curl -I` 能看到安全头。
- 反代在应用服务未启动时也能正常启动；若用 Nginx，需使用 Docker resolver/变量代理，避免静态 upstream 因服务未解析而启动失败。

### Phase 6 · 生产环境变量模板

- [ ] 根目录新增 `.env.example`，按服务分组。
- [ ] 新增 `deploy/compose.env.example`，只放 Compose 需要读取的变量。
- [ ] 新增 `docs/deploy/docker.md`，记录从构建镜像到启动服务的完整步骤。
- [ ] 所有真实密钥只在部署机 `.env` 或服务器环境中配置，不进入 git。

必须明确的生产变量：

- Portal：`POSTGRES_DATABASE_URL`、`REDIS_CONN_STRING`、`API_BASE_URL`、`PORTAL_BASE_URL`、`SESSION_SECRET`、`TDT_SIGNING_KEY`、`DEV_MOCK_OAUTH=false`。
- OAuth：各 `OAUTH_*` 凭据。
- Service Account：各 `*_SA_CLIENT_ID`、`*_SA_CLIENT_SECRET`。
- 独立数据库：`FILES_DATABASE_URL`、`SPMS_DATABASE_URL`、`SMS_DATABASE_URL`、`QBANK_DATABASE_URL`、`LMS_DATABASE_URL`、`LLM_GATEWAY_DATABASE_URL`。
- 文件服务：`FILES_ENC_KEY`、S3/MinIO endpoint、region、access key、secret key、bucket。
- LLM Gateway：`LLM_GATEWAY_SECRET_ENC_KEY`。
- SMTP：`SMTP_HOST`、`SMTP_PORT`、`SMTP_USERNAME`、`SMTP_PASSWORD`、`SMTP_FROM`。

验收：

- `.env.example` 不含真实值。
- 用 `deploy/compose.env.example` 复制出的 `deploy/compose.env` 能启动本地演示。

### Phase 7 · 生产初始化

- [ ] 拆分 dev seed 与生产 bootstrap：
  - `db:seed` 保留开发演示用途。
  - 新增 `bootstrap:prod`，只创建最小生产必需数据。
- [ ] `bootstrap:prod` 创建：
  - platform tenant。
  - 首个平台管理员。
  - marketplace listings。
  - service accounts。
  - `app_service_deployments` 初始记录，状态为 `not_deployed`。
  - 必要默认配置。
- [ ] `bootstrap:prod` 必须幂等。
- [ ] Service Account 初始 secret 只输出一次。
- [ ] 生产模式下拒绝：
  - `DEV_MOCK_OAUTH=true`
  - 空 `SESSION_SECRET`
  - 空 `TDT_SIGNING_KEY`
  - 空或默认的 `*_ENC_KEY` / `*_SECRET_ENC_KEY`：尤其 `LLM_GATEWAY_SECRET_ENC_KEY` 现在是 `optional` 且会回退到硬编码的 `'llm-gateway-dev-enc-key'`（`apps/llm-gateway-server/src/lib/env.ts:44`），生产必须改成 required 或在此拒绝（`FILES_ENC_KEY` 已是 required）。
  - dev fallback resource key 鉴权

验收：

- 空库迁移后执行 `bootstrap:prod`，能得到最小可登录环境。
- 初始环境不启动任何独立 App 后端。
- 重复执行不会覆盖已轮换 secret。
- 生产配置错误时启动失败，而不是降级到开发默认值。

### Phase 8 · 按需部署控制器

- [ ] 新增 `app_service_deployments` 和 `app_deployment_jobs` 迁移。
- [ ] 新增 `DEPLOYABLE_APP_SERVICES` 白名单注册表。
- [ ] 新增 `ensureDeploymentQueued()`。
- [ ] 改造 `installListing()`，安装后为目标和依赖应用创建部署任务。
- [ ] 新增 `deploy-controller` worker。
- [ ] worker 支持：
  - DB 任务领取。
  - Compose profile 启动。
  - 单应用迁移。
  - 全局初始化 / 跨应用接线（`<key>:provision:global`，幂等）。
  - 健康检查等待（按 `healthUrl`，llm-gateway 为 `/healthz`）。
  - 状态落库。
  - 失败记录与重试。
- [ ] 新增 Runtime API：
  - `GET /api/apps/:appKey/runtime`
  - `POST /api/admin/apps/:appKey/deployment/retry`
- [ ] `MicroAppHost` 增加服务未就绪状态页。

验收：

- 初始部署只运行平台服务。
- 租户首次安装 `files` 后，`files-server` 自动启动并健康。
- 第二个租户安装 `files` 不重复创建部署任务，但该租户的 per-tenant 开通仍会执行。
- 依赖安装场景下，依赖服务先启动；安装 `qbank` 后其 LMS 字典下拉非空（验证 `provision:global` + per-tenant bootstrap 真正生效，而非仅进程健康）。
- 服务部署失败时，App Center 不白屏，能看到失败原因和重试入口。

---

## 5. 生产部署流程

### 5.1 首次部署

1. 准备服务器：Docker Engine + Docker Compose Plugin。
2. 准备域名和 TLS。
3. 准备 `deploy/compose.env`。
4. 构建镜像：

```bash
docker build -t xgent-ai-portal:<version> .
```

5. 启动本地基础设施或配置外部 DB/Redis/S3。
6. 运行平台迁移：

```bash
docker compose --env-file deploy/compose.env run --rm portal-api bun run db:migrate:platform
```

7. 运行生产初始化：

```bash
docker compose --env-file deploy/compose.env run --rm portal-api bun run bootstrap:prod
```

8. 启动平台服务：

```bash
docker compose --env-file deploy/compose.env up -d reverse-proxy portal-api deploy-controller
```

9. 运行平台健康检查：

```bash
docker compose --env-file deploy/compose.env run --rm portal-api bun run health:platform
```

此时不启动任何应用后端。租户安装市场应用后，由 `deploy-controller` 按需启动。

### 5.2 更新部署

1. 构建新镜像。
2. 备份数据库。
3. 运行平台迁移。
4. 更新 Compose 镜像 tag。
5. `docker compose up -d reverse-proxy portal-api deploy-controller` 替换平台容器。
6. 对已部署的应用服务执行 `docker compose up -d <service>`，保持其 profile 运行。
7. 对已部署应用逐个运行对应单应用迁移。
8. 运行 `health:platform` 和 `health:deployed`。
9. 浏览器 smoke test：
   - 登录页。
   - Portal 首页。
   - App Center 打开一个微应用。
   - `sdk.callService()` 调一个独立后端。
   - WebSocket 铃铛。
   - Files presign/finalize。

### 5.3 回滚

- 代码回滚：把 Compose 镜像 tag 改回上一版本，执行 `docker compose up -d`。
- 前端回滚：由于前端在同一镜像内，跟随镜像 tag 回滚。
- 数据库回滚：本期不做自动 down migration；破坏性迁移必须拆成向前兼容步骤。
- Secret 回滚：新旧 secret 并存一段时间，确认新 secret 生效后再吊销旧值。

---

## 6. 交付物清单

- [ ] `Dockerfile`
- [ ] `.dockerignore`
- [ ] `deploy/docker-compose.yml`
- [ ] `deploy/compose.env.example`
- [ ] `deploy/caddy/Caddyfile` 或 `deploy/nginx/xgent.conf`
- [ ] `.env.example`
- [ ] `docs/deploy/docker.md`
- [ ] 根脚本：`build`
- [ ] 根脚本：`build:apps`
- [ ] 根脚本：`db:migrate:platform`
- [ ] 根脚本：`db:migrate:app:<key>` 或等价映射
- [ ] 根脚本：`health:platform`
- [ ] 根脚本：`health:deployed`
- [ ] 根脚本：`bootstrap:prod`
- [ ] 根脚本：`deploy:controller`
- [ ] 各后端 `start:prod`（不带 `--env-file`）
- [ ] 各跨应用 listing 的 `<key>:provision:global`（幂等全局接线）
- [ ] 所有后端 `/health`
- [ ] 前端生产子路径 base 配置
- [ ] `app_service_deployments` / `app_deployment_jobs` 迁移
- [ ] `DEPLOYABLE_APP_SERVICES` 注册表
- [ ] 按需部署 Runtime API
- [ ] `MicroAppHost` 服务部署状态页

---

## 7. 里程碑

### M1 · 本地生产构建可用

- [ ] `bun run typecheck` 通过。
- [ ] `bun run build` 通过。
- [ ] 前端 dist 按生产子路径可用。
- [ ] 所有后端 `/health` 可用。

完成标准：不依赖 Vite dev server，构建产物可被静态服务器托管。

### M2 · 镜像可运行

- [ ] `Dockerfile` 可构建。
- [ ] 镜像内包含后端运行代码、迁移文件和前端 dist。
- [ ] 镜像不包含真实 `.env`。
- [ ] 同一镜像可启动不同后端服务。

完成标准：`docker run` 或 Compose command 能启动任一后端。

### M3 · Compose 平台环境可用

- [ ] `docker compose --profile local-infra up -d reverse-proxy portal-api deploy-controller postgres redis minio` 可启动平台环境。
- [ ] 初始状态不启动独立 App 后端。
- [ ] 反代统一暴露 Portal 和平台 API。
- [ ] `db:migrate:platform` 和 `bootstrap:prod` 能在容器内运行。
- [ ] `health:platform` 通过。

完成标准：新机器只装 Docker，配置 env 后能跑起平台和应用市场。

### M4 · 按需应用部署可用

- [ ] 租户安装首个需要后端的应用时，自动创建部署任务。
- [ ] `deploy-controller` 自动启动对应 Compose profile。
- [ ] 控制器跑迁移、等健康、落库状态。
- [ ] 第二个租户安装同应用时复用已部署服务。
- [ ] 服务部署中/失败时，前端有明确状态页。

完成标准：初始只跑平台服务，首次安装应用后对应服务才启动。

### M5 · 生产安全收口

- [ ] `DEV_MOCK_OAUTH=true` 在生产模式下拒绝启动。
- [ ] dev resource key fallback 在生产模式下禁用。
- [ ] CSP/CORS 与单域名子路径部署匹配（前端 meta CSP 不再写死 localhost dev 源；`/svc/*`、`/apps/*` 同源放行）。
- [ ] Service Account secret 不被 seed 覆盖。

完成标准：生产部署没有明显 dev-only 鉴权入口。

### M6 · 文档与验收

- [ ] `docs/deploy/docker.md` 覆盖首次部署、更新、回滚、常见问题。
- [ ] smoke test 清单跑通。
- [ ] 明确哪些 App 进入生产镜像，哪些只保留开发用途。
- [ ] 文档解释首次安装应用会触发服务部署，以及如何查看/重试部署任务。

完成标准：按文档能从空服务器部署到可登录、可打开微应用、可调用独立后端。

---

## 8. 风险清单

- Vite 子路径部署可能破坏 iframe 深链接和公开分享页，需要逐个 App 验证。
- CSP 一旦收紧，最容易影响 iframe、WebSocket、对象存储直传和 LLM Gateway 供应商请求。
- Portal API 与 Web 如果不同站点，session cookie、OAuth callback、WebSocket 会明显复杂化；本期坚持同站点反代。
- `DEV_MOCK_OAUTH`、`*_RESOURCE_KEY`、默认 service secret 若带入生产，会降低鉴权边界。
- 多数据库迁移必须独立编排，不能让独立 App 表进入 portal 数据库。
- 仅跑 schema 迁移不足以让跨应用 App 可用：qbank↔lms 等的 consent co-grant / 字典接线今天在 dev-only seed 与 per-tenant bootstrap 里，必须抽成幂等的 `provision:global` + per-tenant 开通纳入部署流，否则服务 `ready` 却打不开（见 3.1.1）。
- 文件服务的 DB 元数据与对象存储内容需要一起备份和恢复，否则会出现孤儿对象或缺失对象。
- `FILES_ENC_KEY` 和 `LLM_GATEWAY_SECRET_ENC_KEY` 丢失会导致既有密文不可恢复，部署文档必须强调备份。
- Compose `depends_on` 只能处理启动顺序，不能替代应用级重试；后端连接 DB/Redis 时仍需要合理失败和重试策略。
- `deploy-controller` 挂载 Docker socket，权限很高；必须只运行在内网，且只执行白名单 profile。
- 安装链路不能同步等待长时间 Docker 部署，否则租户请求容易超时；应异步返回部署状态。
- 按需启动会带来首次打开应用的冷启动等待，需要在 UI 中明确展示。
- 应用服务是全局共享，不属于单个租户；卸载时自动停止服务可能误伤其他租户，本期不做自动停止。

---

## 9. 推荐实施顺序

1. 补根脚本和 `/health`。
2. 改前端生产 `base` 与静态产物收集。
3. 写 Dockerfile 和 `.dockerignore`。
4. 写 Compose 与本地 infra profile。
5. 写 Caddy/Nginx 反代配置。
6. 补 `.env.example` 和 `deploy/compose.env.example`。
7. 拆 `bootstrap:prod`，收紧生产 dev-only 开关。
8. 把现有 dev-only `db:seed:*` 的跨应用接线抽成幂等的 `<key>:provision:global`，并把 per-tenant bootstrap 整理成可在每个租户首次安装时幂等触发（见 3.1.1）。
9. 增加部署状态表和 `DEPLOYABLE_APP_SERVICES` 注册表。
10. 改造市场安装链路，安装后异步创建部署任务，并触发 per-tenant 开通。
11. 实现 `deploy-controller`（migrate + `provision:global` + 健康等待）。
12. 增加 Runtime API 和服务未就绪 UI。
13. 按 `docs/deploy/docker.md` 从空环境验收一遍：初始只跑平台，首次安装应用后自动拉起对应服务，且 qbank 等跨应用 App 的字典下拉非空。
