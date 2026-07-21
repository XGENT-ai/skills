# Docker 部署指南（本地开发 / 单机演示）

> **定位**：Docker / Compose 是 **本地开发与单机演示** 路径。**生产部署用 Kubernetes / KubeSphere**（见 [`kubesphere.md`](kubesphere.md)）。两条路径复用同一套按需部署机制（状态机、`app_service_deployments` / `app_deployment_jobs` 表、两层模型、MicroAppHost 状态页），只是编排后端不同：Compose 用 `DEPLOY_BACKEND=docker`（`docker compose up` 拉起 profile），K8s 用 `DEPLOY_BACKEND=kubernetes`（scale Deployment 0→1）。详见 `DeploymentDriver` 抽象（`apps/api/src/modules/deploy/drivers/`）。

XGENT.ai Portal 的容器化部署。单域名、同站点反代；平台服务常驻，微应用后端**按需启动**——第一个租户安装某市场应用时，`deploy-controller` 才拉起对应服务。

> 对应规划：`goal/DEPLOY.md`。本文档只覆盖 Docker / Compose 路径。K8s 生产部署见 `goal/KubeSphere.md` + `docs/deploy/kubesphere.md`。

---

## 1. 架构速览

| 组件 | 说明 |
| --- | --- |
| `reverse-proxy` (Caddy) | TLS、静态前端、`/api`·`/auth`·`/oauth`·`/ws` → portal-api、`/svc/*` → 各 App 后端 |
| `portal-api` | 门户后端（@xgent/api），容器内 `:3000` |
| `deploy-controller` | 唯一持有 Docker socket，按 `app_deployment_jobs` 队列拉起 App 服务 |
| `files/spms/sms/qbank/lms/llm-gateway-server` | 各独立后端，profile `app-<key>`，**默认不启动** |
| `postgres`/`redis`/`minio` | profile `local-infra`，演示用；生产可换外部托管服务 |

公网路由（单域名）：`/` → 门户壳；`/apps/<key>/` → 微应用静态包；`/svc/<key>/` → 对应后端；`/api`·`/auth`·`/oauth`·`/health`·`/ws` → portal-api。

**两层模型**（关键，见 `goal/DEPLOY.md` §3.1.1）：
- **服务部署**（全局，按 listing 一次）：起容器 + 迁移该库 + `<key>:provision:global`（跨应用接线）+ 等健康 → `ready`。
- **租户开通**（每租户一次，幂等）：该租户在此 App 的默认数据 / 字典 / 令牌交换接线。即使服务已 `ready`，第二个租户安装时仍跑一次。

只有进程健康不等于可用——qbank 的 LMS 字典下拉要非空，依赖 `lms:provision:global` + 每租户 `lms`/`qbank` bootstrap 都跑过。

---

## 2. 前置条件

- 一台装好 **Docker Engine + Docker Compose 插件** 的服务器。
- 一个域名 + 解析（生产 TLS）。本地演示可用 `:80`。
- 决定数据库 / Redis / 对象存储：用内置 `local-infra`（演示/单机），或外部托管服务（生产推荐）。

镜像里**不含任何 `.env`**；所有配置由 `deploy/compose.env` 注入。

---

## 3. 构建镜像

仓库根目录的多阶段 `Dockerfile` 产出两个目标，同一次构建上下文（前后端永远同版本，便于回滚）：

```bash
docker build -t xgent-ai-portal:<version>       --target runtime .
docker build -t xgent-ai-portal-proxy:<version> --target proxy   .
```

- `runtime`：Bun 运行镜像，承载 portal-api / 各 App 后端 / deploy-controller（同一镜像，靠 Compose `command` 选择启动哪个）。含 Docker CLI + compose 插件（供 controller）、完整 workspace、drizzle 迁移与各 bootstrap/provision 脚本。
- `proxy`：Caddy + 烘焙好的前端 `dist`。

> 前端服务基址在构建期烘焙为同源相对路径（`VITE_*_SERVER_BASE=/svc/<key>`、`VITE_API_BASE=`）。如需改变，用 `--build-arg` 覆盖。白标用 `--build-arg BRAND=<name>`。

镜像里类型检查**不**跑（内存重、在 CI/host 上跑）。提交前本地：`bun run typecheck && bun run build`。

---

## 4. 配置 `deploy/compose.env`

```bash
cp deploy/compose.env.example deploy/compose.env
# 编辑，填入：镜像 tag、域名、强随机密钥、数据库/Redis/对象存储连接、SMTP、OAuth 等
```

必须设置（生产，否则 `bootstrap:prod` 拒绝启动）：

- `SESSION_SECRET`、`TDT_SIGNING_KEY`（`openssl rand -hex 32`）
- `DEV_MOCK_OAUTH=false`
- `FILES_ENC_KEY`、`LLM_GATEWAY_SECRET_ENC_KEY`（**强随机，且务必备份**——丢失则既有密文不可解）
- 各 `*_DATABASE_URL`、`REDIS_CONN_STRING`（外部服务则指向外部主机）
- 至少一个 OAuth provider 凭据（生产登录入口）
- `XGENT_SITE_ADDRESS`=你的域名（自动 HTTPS）或 `:80`（本地）
- `BOOTSTRAP_ADMIN_EMAIL`=首个平台管理员邮箱（OAuth 登录按邮箱关联）

> `deploy/compose.env` 已被 gitignore。该文件被读两次：`--env-file`（compose 插值）+ 各服务 `env_file`（容器环境）。

---

## 5. 首次部署

```bash
# (可选) 本地基础设施
docker compose --env-file deploy/compose.env --profile local-infra up -d postgres redis minio

# 1) 平台库迁移
docker compose --env-file deploy/compose.env run --rm portal-api bun run db:migrate:platform

# 2) 生产初始化（幂等：平台租户 + 首个管理员 + 市场清单 + 服务账号 + 部署占位行）
docker compose --env-file deploy/compose.env run --rm portal-api bun run bootstrap:prod
#    ↑ 若有新发的 service-account secret，会一次性打印——复制回 compose.env 的 *_SA_CLIENT_SECRET

# 3) 启动平台服务（此时不启动任何 App 后端）
docker compose --env-file deploy/compose.env up -d reverse-proxy portal-api deploy-controller

# 4) 平台健康检查
docker compose --env-file deploy/compose.env run --rm portal-api bun run health:platform
```

打开 `https://<域名>/`（或 `http://localhost`）→ 用 OAuth 登录（邮箱匹配首个管理员）→ 进入门户与应用市场。

---

## 6. 按需安装应用（自动拉起后端）

租户管理员在 **应用管理 → 应用市场** 点击安装某个独立后端应用（如 `qbank`）：

1. Portal 写入租户 `apps` 行（含依赖，如 `qbank` 依赖 `lms`，先装 `lms`）。
2. 为每个 listing 入队**服务部署任务** + **租户开通任务**。
3. `deploy-controller` 领取任务：`docker compose --profile app-lms up -d lms-server` → 迁移 → `lms:provision:global` → 等 `/health` → `ready`；再对 `qbank` 同样处理。
4. 对当前租户跑 `lms`/`qbank` 的 per-tenant bootstrap（字典/示例）+ 令牌交换接线。
5. App Center 打开该应用：服务未就绪时显示「服务正在部署」状态页（自动轮询），失败显示原因 + 「重试部署」。

第二个租户安装同一应用：复用已 `ready` 的服务（不重起容器、不重跑全局迁移/provision），但仍对该新租户跑一次幂等开通。

部署后健康检查（只查 `ready` 的服务）：

```bash
docker compose --env-file deploy/compose.env run --rm portal-api bun run health:deployed
```

---

## 7. 更新部署

```bash
docker build -t xgent-ai-portal:<new> --target runtime . && docker build -t xgent-ai-portal-proxy:<new> --target proxy .
# 备份数据库（含 files 的对象存储一并备份）
docker compose --env-file deploy/compose.env run --rm portal-api bun run db:migrate:platform
# 改 compose.env 的 XGENT_IMAGE / XGENT_PROXY_IMAGE 为 <new>
docker compose --env-file deploy/compose.env up -d reverse-proxy portal-api deploy-controller
# 已部署的 App 服务：逐个 up -d 替换 + 跑其单应用迁移
docker compose --env-file deploy/compose.env --profile app-<key> up -d <key>-server
docker compose --env-file deploy/compose.env run --rm portal-api bun run db:<key>:migrate
docker compose --env-file deploy/compose.env run --rm portal-api bun run health:platform
docker compose --env-file deploy/compose.env run --rm portal-api bun run health:deployed
```

`bun run db:migrate:all` 是维护用的「平台 + 全部应用」迁移聚合脚本（**不**用于首次部署）。

---

## 8. 回滚

- **代码 / 前端**：把 `XGENT_IMAGE` / `XGENT_PROXY_IMAGE` 改回上一个 tag，`docker compose up -d`。前端随镜像 tag 一起回滚。
- **数据库**：本期不做自动 down migration；破坏性迁移须拆成向前兼容步骤。先备份。
- **Secret**：新旧并存一段时间，确认新值生效后再吊销旧值。

---

## 9. 冒烟测试清单

- [ ] 登录页 + OAuth 登录
- [ ] 门户首页 / Dashboard
- [ ] App Center 打开一个微应用（iframe）
- [ ] `sdk.callService()` 调一个独立后端（如打开 files 上传/列目录）
- [ ] WebSocket 铃铛（通知）
- [ ] Files presign/finalize 上传下载
- [ ] 安装 `qbank` 后其 **LMS 字典下拉非空**（验证 provision:global + per-tenant bootstrap 真生效）
- [ ] `curl -I https://<域名>/` 能看到安全响应头（CSP / X-Content-Type-Options / Referrer-Policy / Permissions-Policy）

---

## 9.5 租户自定义域名（零运维）

租户管理员在 **租户设置 › 域名** 的配置向导里自助完成，全程无需运维：

1. 添加域名 → 向导给出 TXT 记录（`_xgent-verify.<domain>` = `xgent-verify=<token>`），租户在自己的 DNS 面板添加后点「检查」——portal-api 做真实 DNS 探测，通过即标记已验证。
2. 把域名 CNAME 到平台主域（或 A 记录指到本机 IP），向导可实时检查解析。
3. HTTPS 证书**自动签发**：Caddy `on_demand_tls` 在该域名首次 HTTPS 访问时申请证书，签发前回调 `GET /api/domains/tls-allowed?domain=…`（200=已验证租户域名才放行）。无需改 Caddyfile、无需 reload。

前提：Caddy 直接终结 TLS（`XGENT_SITE_ADDRESS`=域名）。若 TLS 在 ingress/LB 终结（K8s，`XGENT_SITE_ADDRESS=:80`），自定义域名的证书要配在 ingress（如 cert-manager），并把该 host 路由到反代——向导的 TXT/DNS 检查仍然可用，HTTPS 状态在 ingress 配好后变绿。

## 10. 常见问题

- **App 一直「部署中」**：看 `docker compose logs deploy-controller`；`select * from app_deployment_jobs order by created_at desc` 看任务 `error`。修因后 App Center 点「重试部署」或 `POST /api/admin/apps/<appKey>/deployment/retry`。
- **字典下拉为空但服务 ready**：per-tenant bootstrap 或 `lms:provision:global` 没跑。重试该租户安装会重新入队 `provision-tenant`。
- **`/svc/*` 502**：对应 App 服务还没起（profile 未启动）或刚 crash；controller 起来后会自动拉起。Caddy 懒解析 upstream，App 没起也能正常启动。
- **iframe / `/svc` 被 CSP 拦**：生产前端不再带 `<meta>` CSP，权威 CSP 由反代下发（同源 `'self'`）。files-app 直传对象存储需把 `FILES_STORAGE_ORIGIN` 设为对象存储公网 origin。
- **上传大文件超时**：字节直传走预签名 → 对象存储（不经反代）；反代 `XGENT_MAX_BODY` 仅限控制面 body。
- **外部 DB/Redis/S3**：不带 `--profile local-infra`，把 `*_DATABASE_URL` / `REDIS_CONN_STRING` / `MINIO_*` 指向外部即可。

---

## 11. 进入生产镜像的 App

进入生产镜像（前端构建 + 反代路由 + 市场清单）：`web`（门户）、`files`、`spms`、`sms`、`qbank`、`lms`、`llm-gateway`、`task`、`exam`、`library`、`git`、`todo`（TODO-APP 服务化后 todo 已是 `todo-server` 承载的生产 App）。

仅开发用、**不**进入生产镜像：`sample-app`（dev 演示；`bootstrap:prod` 不创建其清单）。

外部镜像 App（后端由对方团队交付、**不**进 `runtime` 镜像、不参与 `build:apps`、不在 `DEPLOYABLE_APP_SERVICES` → **常驻、非按需**）：

- `knowledge`（知识库，`micro` 型）—— 其**市场清单 + 服务账号 + `/svc/knowledge` 反代路由 + `knowledge→files` 令牌交换**随镜像由 `bootstrap:prod` 一并生成；只差「跑起对方 `knowledge-server` 镜像 + 前端 dist + Chroma + 库 + 注密钥」。
- `omni-parser`（全模态解析，`service` 型 headless）—— 同样**随 `bootstrap:prod` 一并生成**市场清单（`type:"service"` 隐藏）+ 服务账号 + `/svc/omni-parser` 反代路由（已内联）+ `files→omni-parser` 令牌交换；只差「跑起对方 `omni-parser-server` 镜像 + 库 + 注密钥 + 安装实例」。

**这两个外部镜像怎么随门户一并部署（Compose + K8s 完整步骤、env 契约、令牌交换、排错），见 [`external-apps.md`](external-apps.md)。** 本地联调走 `deploy/app-devkit/`（通用一盒）/ `deploy/knowledge-devkit/`（知识库样板）。
