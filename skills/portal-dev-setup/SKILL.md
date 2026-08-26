---
name: portal-dev-setup
description: '起/修本地「门户一盒」(one-box) Docker 联调栈——deploy/onebox + deploy/app-devkit 那套：compose.env 三段拼装、local-infra(pg/redis/minio)、db:migrate + db:seed:onebox、register-app 写 /svc 放行、起 reverse-proxy + portal-api + 四个基础后端(files/ingest/llm-gateway/git) + 你自己的 app-backend。凡任务涉及「起一盒 / 起本地门户容器栈 / 让某个外部 App 接一个真实门户联调 / 一盒起不来 / 一盒重置」，或出现 /svc/<key> 404 或 502、http://localhost 打不开、port is already allocated、COMPOSE_PROJECT_NAME 撞栈把别人容器接管了、portal-api 一直 unhealthy、register-app 报 VALIDATION_FAILED、自省 401、iframe 白屏这类症状时，务必先用本 skill 再敲 docker compose——即使用户只说「把一盒起起来」。注意：门户 monorepo 自己的本地开发（bun run dev:all、api:3000 / web:5300、db:seed、verify:all）不走这里。Use when bringing up, smoke-testing, resetting, or debugging the local one-box portal Docker stack for external-app integration.'
---

# 门户一盒 · 本地联调栈

一盒 = 一个**跑在 Docker 里的真实门户**（精简镜像），给「不在门户 monorepo 里的 App」当联调底座。
它**不是**门户开发环境——改门户/内置 App 的代码走 `bun run dev:all`（见 CLAUDE.md），那条链和本 skill 零交集。

判断走哪条：**要改门户或内置 App 的代码 → dev:all。要一个真实门户去接一个外部 App（或复现外部团队的问题）→ 一盒。**

一盒里只有四个基础后端：`files` · `ingest` · `llm-gateway` · `git`。其余内置 App（spms/sms/qbank/lms/exam/agent…）的
workspace **在镜像里已被删掉**，`bun --filter @xgent/spms-server …` 报 `no packages matched the filter` 是**刻意的**。
镜像烘死 `XGENT_ONEBOX=1`：`bootstrap:prod` 与 `deploy-controller` 启动即拒；完整的 `db:seed` 也必失败（它会拉起十几个 App 的种子链）。

## 0. 动手前：先看现状

一盒常常**已经在跑**（`restart: unless-stopped`，重启机器也会自己回来）。盲目 `up` 会把别人的栈接管或半途覆盖。

```bash
.claude/skills/portal-dev-setup/scripts/onebox.sh status
```

它一次给出：当前 `deploy/compose.env` 的**生效值**（compose 是「后定义者胜」，这份文件被三段拼装过，肉眼读第一处必错）、
栈里各容器状态、宿主侧健康探测。同一个脚本还有：

| | |
| --- | --- |
| `onebox.sh env` | 只看生效 env + 配置告警（同键多值、项目名撞、NODE_ENV、latest tag、ffmpeg 那条…） |
| `onebox.sh smoke` | 只跑宿主侧健康探测（`/health` + 每个 key 的 `/svc/<key>/health`） |
| `onebox.sh dc <args>` | `docker compose` + 拼好的 `--env-file` / 四层 `-f` / 一组 `--profile`，其余照直转发 |
| `onebox.sh chain` | 打印它将要用的那串参数（想手敲 compose 时抄这个） |

> **`portal-api` / `*-server` 显示 `unhealthy` 是假红，不用查。** 一盒镜像没装 `curl`（省体积），而
> compose 的 healthcheck 写的是 `curl -fsS …/health`，于是**永远**失败：`docker inspect` 里看到的是
> `/bin/sh: 1: curl: not found`。判活只认宿主侧探测（`onebox.sh smoke`），别信这一列。
> `reverse-proxy`（caddy 镜像自带 wget）与 pg/redis/minio 的 healthy 是真的。

## 1. 三个必须先定的决定

| 决定 | 怎么定 | 定错的症状 |
| --- | --- | --- |
| `COMPOSE_PROJECT_NAME` | 每套栈一个唯一名，如 `onebox-<你的 App key>` | 用模板默认的 `xgent` ⇒ 和同机另一套 compose 被认成同一项目：容器被接管/停掉，命名卷共享 |
| 六个宿主发布端口 | 80/443 · 5432 · 6379 · 9000/9001，撞了就改 `HTTP_PORT` / `POSTGRES_PORT` / … | `port is already allocated`；改了 `HTTP_PORT` 后门户地址随之变，下文所有 `http://localhost/...` 都要跟着改 |
| 你的 App 是 `micro` 还是 `service` | 有前端(iframe) = micro，多一层 `docker-compose.app-frontend.yml` + `APP_FRONTEND_DIST`；纯 API = service，**不要**加那层 | service 配了前端 override ⇒ 起栈报缺 `APP_FRONTEND_DIST`；micro 漏了 ⇒ iframe 404 |

同机还在跑门户的 `dev:all`？**应用层完全不冲突**（那边是 api:3000 · web:5300 · 后端 4100–4995 · micro-app 5301–5318，与上表零交集），
真正会撞的只有基础设施三件 pg/redis/minio——按上表错开。**不要**反过来关掉 `--profile local-infra` 让一盒连你本地那套 PG：
库名会撞（`xgent-portal` / `xgent-files` …），而 `db:seed:onebox` 是**会往里写**的，等于拿一盒的种子污染本地开发库。

## 2. compose.env：三段拼装，本机覆盖块必须在最末尾

⚠️ `deploy/compose.env` **不进 git**，里面是这台机器唯一的一份密钥。已经有一份能用的就**别重来**——
跑 `onebox.sh env` 看生效值，缺什么往末尾补什么。下面这段只用于**从零**配一台新机器。

```bash
cp   deploy/compose.env.example                deploy/compose.env
cat  deploy/onebox/compose.env.onebox.example  >> deploy/compose.env   # 精简一盒增量
cat  deploy/app-devkit/compose.env.app.example >> deploy/compose.env   # 你的 App 接线
# 然后把本机覆盖（项目名/端口/密钥/APP_*）追加到【文件最末尾】
```

顺序即语义：**docker compose 读 env-file 是后定义者胜**，所以覆盖块写在中间等于没写。改完用
`onebox.sh env` 复核生效值——它会把「同一个键定义了多次且值不一致」直接标出来。

必须自己填的：

- `XGENT_IMAGE` / `XGENT_PROXY_IMAGE` —— 具体版本 tag。**这个仓库 tag 不可变、不接受 `latest`**，换镜像要显式改这两行。
- `NODE_ENV=development` —— 镜像烘的是 `production`，不覆盖则 prod-guard 拒绝在 `DEV_MOCK_OAUTH=true` 下启动 portal-api，且 `register-app`（dev 模式）拒跑。
- `DEV_MOCK_OAUTH=true` —— 本地 dev 登录门。
- 随机密钥：`SESSION_SECRET` / `TDT_SIGNING_KEY` / `PLATFORM_ADMIN_KEY` / `FILES_ENC_KEY` / `LLM_GATEWAY_SECRET_ENC_KEY`（`openssl rand -hex 32`）。
- `PREVIEW_MEDIA_CONVERTER_URL=` **留空** —— 一盒不装 ffmpeg，填 `auto` 会让每次转换去 exec 一个不存在的二进制。
- `APP_KEY`（== manifest 的 `listingKey`，网络别名靠它）· `APP_IMAGE` · micro 才要的 `APP_FRONTEND_DIST`（**绝对路径**，目录里得有 `index.html`）。
- 你 App 的 SA 密钥：`<PREFIX>_SA_CLIENT_SECRET` 必须**等于** manifest 里 `serviceAccount.secret`，否则自省 401。

## 3. 起栈：顺序本身是契约

用 `onebox.sh dc <compose 子命令>` 跑——它按生效的 `APP_KEY` / `XGENT_APP_CATALOG` 拼好那串 `-f` 与
`--profile`（漏一个 `-f` 就是另一套语义），你只写子命令。

```bash
S=.claude/skills/portal-dev-setup/scripts/onebox.sh   # profile 由它按生效 env 补齐，你不用写

# 1) 基础设施
$S dc up -d postgres redis minio

# 2) 门户库迁移 + 一盒种子  ★ 破坏性：truncate cascade，会清掉租户/用户/清单/安装
$S dc run --rm portal-api bun run db:migrate
$S dc run --rm portal-api bun run db:seed:onebox

# 3) 四个基础后端各自的库（xgent-* 库由 local-infra 的 postgres init 自动建好）
for k in files ingest llm-gateway git; do $S dc run --rm portal-api bun run db:$k:migrate; done

# 4) 外部 App 才需：读 manifest → 注册 listing + 服务账号 + 写 /svc 放行 map
#    manifest 在你自己 repo 里就挂你自己那份的目录；仓内样例在 deploy/app-devkit/manifests/
$S dc run --rm -v "$PWD/deploy/app-devkit/manifests:/devkit:ro" \
   portal-api bun run register-app /devkit/<你的>.manifest.json

# 5) 起门户三件套 + 基础后端 + 你的 app-backend
$S dc up -d
```

两条**顺序**约束，颠倒了症状都很难反查：

- **`db:seed:onebox` 必须在 `register-app` 之前。** 种子第一步是 `truncate … marketplace_listings … cascade`——
  先注册后种子 = 你的 App 注册被静默清掉，市场里找不到它。
- **`register-app` 最好在 `reverse-proxy` 之前。** 它落的是 `caddy-svc-allow/<key>.map`，反代**启动时** import 那个目录。
  反代已经在跑就补一句 `$S dc exec reverse-proxy caddy reload`，否则 `/svc/<key>` 一直 404。

外部 App 的**交换发起方密钥**还有第三个顺序点：`app_secrets` 绑在**已安装实例**上。所以是
`register-app` → 在市场里装上你的 App → **再跑一次 `register-app`**（幂等）把 `exchangeInitiatorSecret` 写进去。
漏了的症状是 `knowledge→files` 这类交换在发起方 401。

## 4. 冒烟

```bash
.claude/skills/portal-dev-setup/scripts/onebox.sh smoke
```

探 `GET /health`（门户，注意**不是** `/api/health`，那个 404）与每个 key 的 `GET /svc/<key>/health`。
全绿之后再开浏览器：`http://localhost/` → dev 登录 → `rockie@xgent.ai`（一盒演示租户 admin + 平台管理员）。
`db:seed:onebox` 只种两个账号，另一个是 `liming@xgent.ai`（普通成员）——**ACL 基线没到位的问题只在非管理员身上现形**，
验收要用它再走一遍。

micro App 的完整链路：应用市场安装 → 应用中心打开 → iframe 加载 `/apps/<key>/` → `sdk.ready()` 握手 →
`sdk.callService()` 通 → 首次跨应用读文件弹交换授权页。service App 无 UI，只走 curl（`/health` 判活 + 缺 token 应 401/403 + 正确 TDT 应 200 信封）。

## 5. 出问题了

先跑 `onebox.sh status`，再对着 **[references/troubleshooting.md](references/troubleshooting.md)** 的症状表对号入座——
那张表按「你看到什么」编排，覆盖 404/502、自省 401、iframe 白屏、交换报错、端口与项目名、镜像拉取、
以及几个**门自己不会报**的假绿/假红。

## 6. 重置与拆栈

```bash
S=.claude/skills/portal-dev-setup/scripts/onebox.sh
$S dc down                 # 停容器，留命名卷（数据还在，下次 up 接着用）
$S dc down -v              # ★ 连命名卷一起删：pg-data / minio-data / apps-data / caddy-* 全没
```

只想重置门户数据、不想动卷：重跑第 2 步的 `db:seed:onebox`（同样是破坏性 truncate），
然后**重跑 `register-app`**（外部 App 的 listing 被种子清掉了），最后 `caddy reload`。
⚠️ 种子会重新生成 UUID，浏览器里的会话随之失效——重登一次 dev 登录，不是坏了。

## 7. 镜像从哪来

平常**拉**，不自己构：

```bash
docker login <registry>        # puller 账号（只读），找开发团队要；用 --password-stdin
docker pull <registry>/xgent-dev/one-box:<version>
docker pull <registry>/xgent-dev/proxy:<version>
```

一盒**只发 arm64**（它是给开发机用的调试底座）。真要自己构（改了 `deploy/onebox/Dockerfile` 或基础 App 集时）：

```bash
BUILD=1 scripts/pack-onebox.sh --base-apps files,ingest,llm-gateway,git
```

它走 `deploy/onebox/Dockerfile`（**不是**仓库根那份——根那份是生产门户镜像，全量 App），
并顺手 `docker save` 出可离线交付的包。注意 `--base-apps` 改了要同步 `deploy/onebox/` 下的
`docker-compose.onebox.yml`（`XGENT_APP_CATALOG`）与 `compose.env.onebox.example`：
装了后端却没有 listing、或有 listing 却没后端，都是「装得上跑不起来」。`git` 依赖 `files`，两者要么一起在、要么一起不在。

## 相关文档

- 操作手册（最全的一份）：`deploy/app-devkit/README.md`
- App 无关的联调指南：`docs/外部App本地联调指南-one-box.md`
- 运行时契约（你的后端要实现什么）：`docs/SSO与App开发指引.md`
- manifest 样例：`deploy/app-devkit/manifests/{knowledge,omni-parser}.manifest.json`
