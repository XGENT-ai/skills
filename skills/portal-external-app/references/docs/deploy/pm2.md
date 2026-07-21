# 裸机 / pm2 单机生产部署（不用 Docker 应用镜像、不用 k8s）

> **定位**：把门户部署到单台 Ubuntu VM，后端用 **pm2** 常驻跑 bun 进程、前端静态由原生 **Caddy** 出、前面挂 **CDN（同域加速）**。基础设施（Postgres/Redis/MinIO）用 **docker compose `local-infra`** 起在同一台机。
> 对照：Docker 单盒见 [`docker.md`](docker.md)，K8s 生产见 [`kubesphere.md`](kubesphere.md)。本路径与它们复用同一套按需部署机制（状态机、`app_service_deployments` 表、`provisionGlobal`、per-tenant bootstrap、App Center 状态页），只是编排后端是 `DEPLOY_BACKEND=process`（pm2 常驻，`ensureServiceUp` 空转）。

目标机（已探测）：`ubuntu@portal.xgent.ai` → `43.134.174.238`，Ubuntu 24.04 / x86_64 / 4 vCPU / 7.5 GiB / 50 GB。DNS 当前直指该 VM。

---

## 1. 架构与端口

```
浏览器 ─HTTPS─▶ [CDN 同域加速(可后置)] ─回源─▶ Caddy(VM:443/80, ACME)
                                                   ├ /                → /srv/www/portal
                                                   ├ /apps/<key>/     → /srv/www/apps/<key>
                                                   ├ /api /auth /oauth /ws /health → 127.0.0.1:3000
                                                   └ /svc/<key>/      → 127.0.0.1:<port>
pm2 (NODE_ENV=production, 读 /etc/xgent/portal.env):
  portal-api :3000 · deploy-controller(DEPLOY_BACKEND=process)
  files 4100 · spms 4200 · sms 4300 · qbank 4400 · lms 4500 · llm-gateway 4600 · task 4900 · exam 4910 · library 4920 · git 4930 · todo 4940
docker compose --profile local-infra: postgres 5432 · redis 6379 · minio 9000/9001 (均绑 127.0.0.1)
```

端口/库/SA 是 `deploy/pm2/ecosystem.config.cjs`、`deploy/caddy/Caddyfile.pm2`、`apps/api/src/modules/deploy/drivers/process.ts` 三处的共同约定，改端口要同步三处。

**本期生产 App**：portal + `files spms sms qbank lms llm-gateway task exam library git todo`（`LISTING_DEFS` 全量都会进市场，pm2 侧必须全部常驻，否则租户安装会卡在「部署中」）。`knowledge`/`omni-parser` 第三方常驻按需另接（[`external-apps.md`](external-apps.md)）。

---

## 2. 代码改动（已落仓库）

- `apps/api/src/modules/deploy/drivers/process.ts` — pm2 driver：`ensureServiceUp` no-op；`healthUrlFor` → `http://127.0.0.1:<port>/health`。
- `apps/api/src/modules/deploy/drivers/index.ts` — `makeDriver()` 增加 `DEPLOY_BACKEND=process` 分支。
- `deploy/pm2/ecosystem.config.cjs`、`deploy/caddy/Caddyfile.pm2` — 配置产物。

这样市场安装应用时，deploy-controller 仍走 `handleDeploy`（noop ensureServiceUp → `db:<key>:migrate` → `provisionGlobal` → 探 `127.0.0.1:<port>/health` → 翻 `ready`）+ `handleProvisionTenant`（每租户 bootstrap + 兑换接线），自助多租户安装照常。

---

## 3. Phase 0 — VM 装齐工具链

```bash
ssh ubuntu@portal.xgent.ai

# Node LTS —— pm2 守护进程本身是 node 的（被它管理的子进程才是 bun）
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt-get install -y nodejs
sudo npm i -g pm2

# bun —— 与 bun.lock 一致，pin 1.3.14
curl -fsSL https://bun.sh/install | bash -s "bun-v1.3.14"
echo 'export PATH="$HOME/.bun/bin:$PATH"' >> ~/.bashrc && export PATH="$HOME/.bun/bin:$PATH"
sudo ln -sf "$HOME/.bun/bin/bun" /usr/local/bin/bun   # 让 pm2(以 root 起的 startup) 也能找到 bun

# Docker Engine + compose 插件（仅给 local-infra）
sudo apt-get update && sudo apt-get install -y ca-certificates curl gnupg
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo $VERSION_CODENAME) stable" | sudo tee /etc/apt/sources.list.d/docker.list
sudo apt-get update && sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
sudo usermod -aG docker ubuntu   # 重新登录后免 sudo 用 docker

# Caddy（apt 官方源，带 systemd 单元 + 自动 ACME）
sudo apt-get install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt-get update && sudo apt-get install -y caddy
```

防火墙/安全组：放行 **80/443**；**不要**对公网开 5432/6379/9000（下面 local-infra 端口都绑 127.0.0.1）。

---

## 4. Phase 1 — 基础设施（docker compose local-infra）

```bash
sudo mkdir -p /opt/xgent && sudo chown ubuntu:ubuntu /opt/xgent
git clone <repo-url> /opt/xgent/app && cd /opt/xgent/app && git checkout brand-xgent

cp deploy/compose.env.example deploy/compose.env
# 编辑 deploy/compose.env：填 POSTGRES_PASSWORD / MINIO_ROOT_USER / MINIO_ROOT_PASSWORD 强随机
```

把 local-infra 端口绑回环（编辑 `deploy/docker-compose.yml` 的 postgres/redis/minio `ports` 为 `127.0.0.1:5432:5432` 等，或用 compose override）。然后：

```bash
docker compose --env-file deploy/compose.env --profile local-infra up -d postgres redis minio
# postgres 首启自动跑 deploy/postgres/init/01-create-databases.sql → 建好 11 个库
#（数据卷已初始化过的旧机器不会重跑：手动补 CREATE DATABASE "xgent-exam" / "xgent-library" / "xgent-git"）
docker compose --env-file deploy/compose.env exec postgres psql -U postgres -l   # 确认 xgent-* 库都在
```

MinIO 建 bucket `xgent-files`（控制台 `http://127.0.0.1:9001` 经 ssh 隧道，或 `mc`），记下 access/secret。

---

## 5. Phase 2 — 构建前端 + 铺静态

```bash
cd /opt/xgent/app
export PATH="$HOME/.bun/bin:$PATH"
# 本机 7890 代理在 VM 上不存在；如装慢可设 NO_PROXY，见 memory bun-install-proxy
bun install --frozen-lockfile

# 同源相对路径；build:apps 是顺序 vite 构建（峰值 ~2-3GB，本机 7.5G 够）
VITE_API_BASE="" BRAND=xgent bun run build:apps

# 铺静态
sudo mkdir -p /srv/www/portal /srv/www/apps && sudo chown -R ubuntu:ubuntu /srv/www
rsync -a --delete apps/web/dist/ /srv/www/portal/
for k in files spms sms qbank lms llm-gateway task exam library git todo; do
  rsync -a --delete "apps/${k}-app/dist/" "/srv/www/apps/${k}/"
done

# 中和被 git 跟踪的 dev .env（防止 bun 自动加载 dev 默认值如 DEV_MOCK_OAUTH=true 泄漏）
: > /opt/xgent/app/.env
```

> 若 VM 偏小，可在本机 `bun run build:apps` 后只 `rsync dist/` 上来；后端仍在 VM 上 bun 跑源码，无跨架构问题。

---

## 6. Phase 3 — 写 `/etc/xgent/portal.env`

```bash
sudo mkdir -p /etc/xgent && sudo touch /etc/xgent/portal.env
sudo chmod 600 /etc/xgent/portal.env && sudo chown ubuntu:ubuntu /etc/xgent/portal.env
# 生成强随机：openssl rand -hex 32
```

内容（**唯一真值源**，root:600，不进 git）：

```dotenv
NODE_ENV=production

# --- 生产门强制（apps/api/src/lib/prod-guard.ts）---
DEV_MOCK_OAUTH=false
SESSION_SECRET=<rand32>
TDT_SIGNING_KEY=<rand32>
PLATFORM_ADMIN_KEY=<rand32>
FILES_ENC_KEY=<rand32>                  # 丢失=既有密文不可解，必须离线备份
LLM_GATEWAY_SECRET_ENC_KEY=<rand32>     # 同上

# --- 公网/同源（单域，所有 *_APP_URL = 门户域名，无 CORS）---
PORTAL_BASE_URL=https://portal.xgent.ai
SAMPLE_APP_URL=https://portal.xgent.ai
TODO_APP_URL=https://portal.xgent.ai
FILES_APP_URL=https://portal.xgent.ai
SPMS_APP_URL=https://portal.xgent.ai
SMS_APP_URL=https://portal.xgent.ai
QBANK_APP_URL=https://portal.xgent.ai
LMS_APP_URL=https://portal.xgent.ai
LLM_GATEWAY_APP_URL=https://portal.xgent.ai
TASK_APP_URL=https://portal.xgent.ai
EXAM_APP_URL=https://portal.xgent.ai
LIBRARY_APP_URL=https://portal.xgent.ai
TODO_APP_URL=https://portal.xgent.ai
KNOWLEDGE_APP_URL=https://portal.xgent.ai

# --- 库 / Redis（local-infra，走回环）---
POSTGRES_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-portal
FILES_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-files
SPMS_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-spms
SMS_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-sms
QBANK_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-qbank
LMS_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-lms
LLM_GATEWAY_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-llm-gateway
TASK_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-task
EXAM_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-exam
LIBRARY_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-library
GIT_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-git
TODO_DATABASE_URL=postgres://postgres:<pgpass>@127.0.0.1:5432/xgent-todo
REDIS_CONN_STRING=redis://127.0.0.1:6379

# --- 进程间 ---
API_BASE_URL=http://127.0.0.1:3000
PORTAL_INTROSPECT_URL=http://127.0.0.1:3000/api/tokens/introspect
LMS_SERVER_URL=http://127.0.0.1:4500
# TODO-APP §3.2: spms-server → todo-server 待办镜像 ingest 推送
TODO_SERVER_URL=http://127.0.0.1:4940
DEPLOY_BACKEND=process

# --- 服务账号 secret（bootstrap:prod 首跑打印后回填，再重跑幂等）---
FILES_SA_CLIENT_SECRET=<after-bootstrap>
SPMS_SA_CLIENT_SECRET=<after-bootstrap>
SMS_SA_CLIENT_SECRET=<after-bootstrap>
QBANK_SA_CLIENT_SECRET=<after-bootstrap>
LMS_SA_CLIENT_SECRET=<after-bootstrap>
LLM_GATEWAY_SA_CLIENT_SECRET=<after-bootstrap>
TASK_SA_CLIENT_SECRET=<after-bootstrap>
EXAM_SA_CLIENT_SECRET=<after-bootstrap>
LIBRARY_SA_CLIENT_SECRET=<after-bootstrap>
GIT_SA_CLIENT_SECRET=<after-bootstrap>
TODO_SA_CLIENT_SECRET=<after-bootstrap>
# 跨应用兑换发起方（sms/qbank→lms，files→omni-parser，exam→qbank/lms，library→files/exam/lms，
# git→files 双向桥，spms→git 代码 tab）
# 强随机自拟即可：provisionGlobal 会把 env 值的 hash 种进 app_secrets（缺省会种 dev 弱密钥）
SMS_APP_SECRET=<rand32>
QBANK_APP_SECRET=<rand32>
FILES_APP_SECRET=<rand32>
EXAM_APP_SECRET=<rand32>
LIBRARY_APP_SECRET=<rand32>
GIT_APP_SECRET=<rand32>
SPMS_APP_SECRET=<rand32>

# --- OAuth（三个 provider；回调 https://portal.xgent.ai/auth/<provider>/callback）---
OAUTH_GITHUB_CLIENT_ID=
OAUTH_GITHUB_CLIENT_SECRET=
OAUTH_GOOGLE_CLIENT_ID=
OAUTH_GOOGLE_CLIENT_SECRET=
OAUTH_LARK_APP_ID=
OAUTH_LARK_APP_SECRET=

# --- 对象存储（local-infra minio）---
MINIO_ENDPOINT=http://127.0.0.1:9000
MINIO_REGION=us-east-1
MINIO_ACCESS_KEY=<minio-access>
MINIO_SECRET_KEY=<minio-secret>
MINIO_BUCKET=xgent-files
FILES_STORAGE_ORIGIN=

# --- 首个平台管理员（OAuth 按邮箱关联）---
BOOTSTRAP_ADMIN_EMAIL=<你的邮箱>
BOOTSTRAP_ADMIN_NAME=Platform Admin

# --- SMTP（留空禁用邮件）---
SMTP_HOST=
SMTP_PORT=465
SMTP_USERNAME=
SMTP_PASSWORD=
SMTP_FROM=
SMTP_TLS=implicit
```

---

## 7. Phase 4 — 平台迁移 + 生产初始化

```bash
cd /opt/xgent/app
set -a; . /etc/xgent/portal.env; set +a     # 进 shell 注入，供一次性命令用

bun run db:migrate:platform     # 门户库 schema
bun run db:migrate:all          # 各 app 库 schema（首次一次性）
bun run bootstrap:prod          # 平台租户 + 首管理员 + 市场清单 + SA + 部署占位行(not_deployed)
#   ↑ 打印各 *_SA_CLIENT_SECRET → 复制回 /etc/xgent/portal.env，然后可重跑确认幂等
```

`bootstrap:prod` 在 `isProd()` 下先跑 `deploymentViolations()`：DEV_MOCK_OAUTH / SESSION_SECRET / TDT_SIGNING_KEY / FILES_ENC_KEY / LLM_GATEWAY_SECRET_ENC_KEY 任一弱或缺即拒启。

---

## 8. Phase 5 — pm2 拉起后端 + controller

```bash
cd /opt/xgent/app && mkdir -p logs
pm2 start deploy/pm2/ecosystem.config.cjs        # portal-api + 9 后端 + deploy-controller
pm2 save
pm2 startup systemd -u ubuntu --hp /home/ubuntu  # 按提示 sudo 执行打印出的那行，开机自启

pm2 status
bun run health:platform     # 门户 + DB OK
curl -fsS http://127.0.0.1:4500/health           # 抽查某后端(lms)直连
```

---

## 9. Phase 6 — Caddy 反代 + TLS（DNS 直连阶段）

```bash
sudo cp deploy/caddy/Caddyfile.pm2 /etc/caddy/Caddyfile
sudo tee /etc/caddy/caddy.env >/dev/null <<'EOF'
XGENT_SITE_ADDRESS=portal.xgent.ai
ACME_EMAIL=you@example.com
XGENT_MAX_BODY=512MB
FILES_STORAGE_ORIGIN=
EOF
# 让 systemd 的 caddy 读这个 env：
sudo systemctl edit caddy   # 加 [Service]\nEnvironmentFile=/etc/caddy/caddy.env
sudo caddy validate --adapter caddyfile --config /etc/caddy/Caddyfile
sudo systemctl reload caddy   # 首次可 restart；ACME 自动签 portal.xgent.ai 证书

curl -I https://portal.xgent.ai/    # 看 200 + CSP / X-Content-Type-Options / Referrer-Policy / Permissions-Policy
```

> DNS 此刻仍直指 VM（43.134.174.238），HTTP-01/TLS-ALPN 挑战可达，证书自动签发。

> **租户自定义域名（零运维）**：Caddyfile.pm2 自带 `https://` catch-all + `on_demand_tls`——租户在 租户设置 › 域名 向导里过 TXT 验证、把域名指到本 VM 后，首次 HTTPS 访问自动签证书（签发前回调 portal-api `/api/domains/tls-allowed`，只放行已验证域名）。无需再改 Caddy。若前面挂了 CDN（Phase 8 方案 A），租户域名需直指本 VM 或在 CDN 加同源站点。

---

## 10. Phase 7 — 装核心 App（controller 自动翻 ready）

浏览器开 `https://portal.xgent.ai/` → OAuth 登录（邮箱匹配 `BOOTSTRAP_ADMIN_EMAIL`）→ 应用市场，依次安装：`lms` → `qbank` → `sms` → `files` → `llm-gateway` → `task` → `exam` → `library` → `git`（依赖会自动先装：qbank→lms，exam→qbank/lms，library→files/exam/lms，git→files）。

每次安装 controller 自动：`db:<key>:migrate` → `provisionGlobal` → 探 `127.0.0.1:<port>/health` → `ready` + 该租户 per-tenant bootstrap。

```bash
bun run health:deployed     # 查已 ready 服务
# 卡住排查：pm2 logs deploy-controller ；SELECT * FROM app_deployment_jobs ORDER BY created_at DESC;
```

---

## 11. Phase 8 — 前面挂 CDN（方案 A，同域，可后置）

1. 加回源主机名 `origin.xgent.ai → 43.134.174.238`；把它也加进 `XGENT_SITE_ADDRESS`（或单独 site block）让 Caddy 给它签证书。
2. CDN（腾讯云 EdgeOne / Cloudflare 等）：站点 `portal.xgent.ai`，**回源 `origin.xgent.ai`（HTTPS）**。
3. 缓存：缓存 `/assets/*`、`/apps/*/assets/*`（hash 文件名长 TTL）；**绕过**：`/api*` `/auth*` `/oauth*` `/ws*` `/svc/*` 与 `index.html`（不缓存，保证发版即时）。
4. 把 `portal.xgent.ai` 的 DNS 切到 CDN。浏览器始终只见一个 origin → cookie / CORS / CSP / iframe **零改动**。

---

## 12. 运维：备份 / 更新 / 回滚

**备份（cron）**
```bash
# 每库 dump（或 pg_dumpall）
for db in xgent-portal xgent-files xgent-spms xgent-sms xgent-qbank xgent-lms xgent-llm-gateway xgent-task xgent-exam xgent-library xgent-git; do
  docker compose --env-file deploy/compose.env exec -T postgres pg_dump -U postgres "$db" | gzip > "/backup/$db-$(date +%F).sql.gz"
done
mc mirror local/xgent-files /backup/minio/     # 对象存储
# 离线备份 FILES_ENC_KEY + LLM_GATEWAY_SECRET_ENC_KEY（丢失=密文永久不可解）
```

**更新**（零停机滚动）
```bash
cd /opt/xgent/app && git pull
export PATH="$HOME/.bun/bin:$PATH"
bun install --frozen-lockfile
VITE_API_BASE="" BRAND=xgent bun run build:apps
rsync -a --delete apps/web/dist/ /srv/www/portal/
for k in files spms sms qbank lms llm-gateway task exam library git todo; do rsync -a --delete "apps/${k}-app/dist/" "/srv/www/apps/${k}/"; done
set -a; . /etc/xgent/portal.env; set +a
bun run db:migrate:all
pm2 reload all
# 若本次更新引入了新 App 后端：先在 Postgres 建库、补 /etc/xgent/portal.env 的对应键，
# 再 pm2 start deploy/pm2/ecosystem.config.cjs（reload 只滚动已有进程，不会加新进程）
bun run health:all
# CDN 对 index.html 不缓存即时生效；必要时刷 CDN 静态缓存
```

**回滚**：`git checkout <上个 tag>` → 重复更新流程。DB 无自动 down migration——破坏性迁移须拆成向前兼容步骤，先备份。

---

## 13. 排错

- **App 一直「部署中」**：`pm2 logs deploy-controller`；查 `app_deployment_jobs` 的 `error`。常见：对应后端 pm2 没起（`pm2 status`）/ 端口不符 / `127.0.0.1:<port>/health` 不通。修后 App Center 点「重试部署」。
- **`/svc/*` 502**：对应 `<key>-server` pm2 没起或刚 crash（`pm2 restart <key>-server`）。Caddy 懒解析 upstream，后端没起也能正常启动。
- **登录后立刻退出 / cookie 丢**：确认 `PORTAL_BASE_URL=https://portal.xgent.ai`、走 HTTPS（Caddy 自动 Secure cookie）、CDN 方案 A 不改 origin。
- **字典下拉空但服务 ready**：per-tenant bootstrap 或 `lms:provision:global` 没跑——重试该租户安装会重新入队 `provision-tenant`。
- **pm2 重启后找不到 bun**：`pm2 startup` 生成的 systemd 单元 PATH 不含 `~/.bun/bin`——已用 `ln -sf ... /usr/local/bin/bun` 兜底；或在单元里加 `Environment=PATH=...`。
- **`bootstrap:prod` 拒启**：按打印的 `✗` 修 `/etc/xgent/portal.env`（弱/缺密钥、`DEV_MOCK_OAUTH=true`）。
```
