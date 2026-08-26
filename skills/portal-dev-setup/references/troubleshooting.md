# 一盒排查表

按「你看到什么」编排。先跑 `onebox.sh status`（它一次给出生效 env + 容器状态 + 宿主侧探测），再来这里对号入座。
下文 `$S` = `.claude/skills/portal-dev-setup/scripts/onebox.sh`。

## 目录

- [§1 假红与假绿（先读这节）](#1-假红与假绿先读这节)
- [§2 起栈就失败](#2-起栈就失败)
- [§3 打不开 / 路由不通](#3-打不开--路由不通)
- [§4 认证、自省与跨应用交换](#4-认证自省与跨应用交换)
- [§5 注册与种子](#5-注册与种子)
- [§6 镜像与拉取](#6-镜像与拉取)

---

## 1. 假红与假绿（先读这节）

这几条的共同点是：**你看到的状态位在说谎**，照着它查会一路查歪。

| 你看到 | 真相 |
| --- | --- |
| `portal-api` / `files-server` / `git-server` … 长期 `(unhealthy)`，但功能一切正常 | **假红。** 一盒镜像**没装 `curl`**（连 `wget` 也没有），而 compose 的 healthcheck 写的正是 `curl -fsS …/health` —— 它永远失败。`docker inspect <容器> --format '{{json .State.Health}}'` 会看到清一色 `/bin/sh: 1: curl: not found`。判活只认宿主侧 `$S smoke`。`reverse-proxy`（caddy 镜像自带 wget）与 pg/redis/minio 的 healthy 是真的 |
| `curl http://localhost/api/health` → 404 `路由不存在` | **探错路径了。** 门户健康端点是 `/health`，不带 `/api`。`/svc/<key>/health` 才是各后端的 |
| 改了 `deploy/compose.env` 却「没生效」 | 那份文件是**三段拼装**出来的（base 模板 + onebox 增量 + devkit 增量 + 你的本机覆盖），同一个键出现三四次很常见。**docker compose 后定义者胜**，你多半改在了中间那处。`$S env` 会把「同键多个不同值」直接标出来，并显示实际生效的那个 |
| 一盒里 `bun --filter @xgent/spms-server …` 报 `no packages matched the filter` | **不是坏了，是刻意的。** 精简镜像里只保留了 `files` / `ingest` / `llm-gateway` / `git` 四个基础 App 的 workspace，其余在 build 阶段就被删掉 |
| `bootstrap:prod` / `deploy-controller` 一启动就退出并打印拒绝原因 | **刻意的。** 镜像烘死 `XGENT_ONEBOX=1`，prod-guard 的 `refuseOnOneBox` 直接拒。一盒是调试底座，不是门户 |
| 完整的 `bun run db:seed` 在一盒里失败 | 它会拉起十几个 App 的种子链，而那些 workspace 不在镜像里。一盒只能用 `db:seed:onebox` |
| 视频没有海报、网格缩略图变成图标 | 一盒不装 ffmpeg（省 419MB）。`PREVIEW_MEDIA_CONVERTER_URL` 必须**留空**；填 `auto` 会让每次转换去 exec 一个不存在的二进制。图片/PDF/文本预览不受影响 |

---

## 2. 起栈就失败

| 症状 | 成因与修法 |
| --- | --- |
| `port is already allocated` | 一盒只发布 6 个宿主端口：80/443（`HTTP_PORT`/`HTTPS_PORT`）· 5432（`POSTGRES_PORT`）· 6379（`REDIS_PORT`）· 9000/9001（`MINIO_PORT`/`MINIO_CONSOLE_PORT`）。撞了就在 compose.env **末尾**改这几个。⚠️ 改 `HTTP_PORT` 后门户地址随之变，所有 `http://localhost/...` 都要跟着改（`$S smoke` 会自动跟着走） |
| 起了一盒，**别的** compose 栈的容器被停/被接管 | 两套栈同名。`COMPOSE_PROJECT_NAME` 还是模板默认的 `xgent` ⇒ compose 认为它们是同一项目，容器名冲突、`pg-data`/`apps-data` 等命名卷被共享。每套栈给唯一名：`COMPOSE_PROJECT_NAME=onebox-<你的 App key>` |
| 报缺 `APP_IMAGE` / `APP_KEY`，即使你没打算起外部 App | `docker-compose.app-dev.yml` 里的 `${APP_IMAGE:?}` 是**解析期**求值的，跟 profile 无关 —— 只要那层 `-f` 带上了就必须有值。不接外部 App 时**整层不要带**（`$S` 会按 compose.env 里有没有 `APP_KEY`+`APP_IMAGE` 自动决定） |
| 报缺 `APP_FRONTEND_DIST` | 你给 `service` 型 App 叠了 `docker-compose.app-frontend.yml`。service 无前端，不要那层 |
| `deploy-controller` / `pagebuilder-server` 起来就 crash-loop 刷屏 | 少叠了 `-f deploy/onebox/docker-compose.onebox.yml`。那层的作用就是把这两个「在精简镜像里跑不起来但在 base compose 里默认启动」的服务挪到 `onebox-extras` profile 后面 |
| portal-api 启动即退，日志提到 prod-guard / `DEV_MOCK_OAUTH` | `NODE_ENV` 没被覆盖成 `development`。镜像烘的是 `production`，而 `DEV_MOCK_OAUTH=true` 在 production 下被 prod-guard 拒绝 |
| 一盒的库把本地开发库污染了 | 你关掉了 `--profile local-infra` 让一盒去连本机那套 PG。库名会撞（`xgent-portal` / `xgent-files` …），而 `db:seed:onebox` 是**会往里写**的。别这么做——按 §2 第一行错开发布端口即可，一盒内部走的是 compose 网络里的 `postgres:5432`，跟宿主端口无关 |

同机还跑着门户 monorepo 的 `dev:all`？**应用层完全不冲突**（那边是 api:3000 · web:5300 · 后端 4100–4995 · micro-app 5301–5318）。会撞的只有基础设施三件。

---

## 3. 打不开 / 路由不通

| 症状 | 成因与修法 |
| --- | --- |
| `/svc/<key>/...` → **404** | `/svc` 放行 map 没写成，或**反代在写 map 之前就起了**（Caddy 启动时 `import` 那个目录，不会热重载）。跑 `$S dc run --rm -v "<manifests 目录>:/devkit:ro" portal-api bun run register-app /devkit/<key>.manifest.json`，然后 `$S dc exec reverse-proxy caddy reload` |
| `/svc/<key>/...` → **502** | 后端不在。① 容器没起/崩了：`$S dc logs <key>-server`（外部 App 是 `app-backend`）；② 没监听容器内 **8080**（反代的通用规则是 `/svc/<key>/* → <key>-server:8080`）；③ `APP_KEY` 与 manifest 的 `listingKey` 不一致，网络别名 `<key>-server` 没命中 |
| 后端跑在**宿主**上（保留热重载），`/svc/<key>` 502 | 反代在容器网里解析不到宿主进程。用一个转发容器顶住那个别名：把 `app-backend` 换成 `alpine/socat`，`command: TCP-LISTEN:8080,fork,reuseaddr TCP:host.docker.internal:<你的 dev 端口>`，`extra_hosts: ["host.docker.internal:host-gateway"]`，叠在 `app-dev.yml` **之后**。此时 `APP_IMAGE` 不再被用到（compose 仍要求它有值，随便填） |
| iframe 404 / 空白 | `APP_FRONTEND_DIST` 不对：必须是**绝对路径**且目录里有 `index.html`（相对路径会按 `deploy/` 解析，很迷惑）。或者你的 App 是 `service` 型却配了 embedUrl/前端 override |
| 浏览器整站打不开 | 先 `$S smoke` 看 `/health`。通了说明是端口/地址问题（改过 `HTTP_PORT` 就不是 `http://localhost` 了）；不通看 `$S dc logs reverse-proxy portal-api` |
| `service` 型 App 在应用市场/应用中心**看不到** | 这是**设计如此**：service 型对用户隐藏（无前端、不可打开），平台控制台的「清单管理」里仍可见可治理 |

---

## 4. 认证、自省与跨应用交换

| 症状 | 成因与修法 |
| --- | --- |
| 自省 **401** | 服务账号 Basic 不对：manifest 的 `serviceAccount.secret` 与后端实际读的 `<PREFIX>_SA_CLIENT_SECRET` / `<PREFIX>_RESOURCE_KEY` 不相等。两边读的是同一份 compose.env，所以对齐了就一定一致 |
| 门户自省为 `active:true` 的 TDT，被你的后端判 `INVALID_TOKEN` | 后端没解包自省信封。`claims = body.data ?? body` —— 这是外部 App 最常见的第一个坑 |
| `EXCHANGE_NOT_ALLOWED` | 来源 listing 没声明 `exchangeTargets` / 目标 App 没安装 / 没建 grant / 用户没同意，四选一 |
| 交换在**发起方** 401（如 `knowledge→files`） | 发起方 App Secret 还没写进去。`app_secrets` 绑定的是**已安装实例**，所以顺序是：`register-app` → 在市场里装上这个 App → **再跑一次 `register-app`**（幂等）。详见 SKILL.md §3 |
| 跨应用下拉列表莫名为空 | 多半是用户级 `exchange_consents` 缺行（布线晚于同意）。这属于跨应用授权面，用 `portal-app-exchange` skill 排 |
| 切了租户还能看到上一租户的数据 | 你的后端没按 `claims.tenant_id` 隔离。这是运行时契约第 3 条，一盒不替你实现 |

## 5. 注册与种子

| 症状 | 成因与修法 |
| --- | --- |
| `register-app` 报 `VALIDATION_FAILED`（scope） | manifest 声明了**别的 App 的 scope**，却没把那个 App 列进 `exchangeTargets`。规则：一个 listing 可声明的 scope = 平台基础 scope ∪ 本 namespace（`<listingKey>` 及其下划线变体）∪ 已声明 `exchangeTargets` 的 namespace |
| `register-app` 拒跑，提到 production | dev 模式的 `register-app` 拒绝 `NODE_ENV=production`。一盒里必须把 `NODE_ENV=development` 写进 compose.env 末尾 |
| 跑完 `db:seed:onebox` 后，市场里找不到你的 App | **顺序反了。** 种子第一步是 `truncate … marketplace_listings … cascade`，先注册后种子 = 注册被清掉。必须 seed 在前、register-app 在后 |
| `seed:onebox` 报 `unknown listing key: <key>` | 这个 key 既不在 `LISTING_DEFS` 里，`deploy/app-devkit/manifests/<key>.manifest.json` 也不存在。要么放一份 manifest 进去，要么把它从 `XGENT_APP_CATALOG` 里去掉 |
| seed 之后浏览器要求重登 | 种子重新生成 UUID，会话随之失效。重走一次 dev 登录，不是坏了 |
| 某个 key 有 listing 却起不来 / 起得来却没 listing | `XGENT_APP_CATALOG` 与镜像构建时的 `--build-arg XGENT_BASE_APPS` 漂了。两者必须一致；`git` 依赖 `files`，要么一起在、要么一起不在 |
| dev 登录后看不到某个 App | 用 `liming@xgent.ai`（普通成员）复验 —— **ACL member 基线没到位的问题只在非管理员身上现形**。管理员那边永远是绿的 |

## 6. 镜像与拉取

| 报错 | 成因与修法 |
| --- | --- |
| `unauthorized: unauthorized to access repository` | 没登录，或口令被轮换过。⚠️ `~/.docker/config.json` 里留着**陈旧** auth 时不会提示你去登录，只会一直 401 —— 先 `docker logout <registry>` 再重登 |
| `denied: requested access to the resource is denied` | 你在 **push**。puller 是只读账号；你自己 App 的镜像推你自己的 registry |
| `manifest unknown` / `not found` | tag 写错了。这个仓库 **tag 不可变、不接受 `latest`**，以开发团队给的那两行为准，别猜、别补 `:latest` |
| `no matching manifest for linux/amd64` | 一盒**只发 arm64**。`--platform linux/arm64` 硬拉下来容器也起不来（`exec format error`）。要 amd64 找开发团队 |
| 卡在 `Retrying in N seconds` 直到超时 | 链路或代理。把 `HTTP_PROXY`/`HTTPS_PROXY` 与 Docker Desktop 的代理设置对齐，或临时关掉代理重试 |
| `x509: certificate signed by unknown authority` | 多半是域名/端口写错（就是 `<registry>`，不带端口、不带 `https://`）。**不要**去 `daemon.json` 加 `insecure-registries` 绕过 |

⚠️ 别拿 `curl` 去探 Harbor 的 API（`/v2/_catalog` 之类）判断账号好不好 —— 只读账号在管理接口上的行为跟拉取不是一回事。**判据只有 `docker pull` 本身。**
