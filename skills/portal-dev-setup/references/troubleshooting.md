# 一盒排查表

**先跑 `onebox.sh doctor`。** 下面这些能自动判的它都判了，并直接给你可粘的修法；这张表是给它判不了的那些。
按「你看到什么」编排。
下文在 App repo 根目录执行命令；`SKILL_DIR` 表示本 skill 的 `SKILL.md` 所在目录（本文件的上一级），`S="$SKILL_DIR/scripts/onebox.sh"`；`portal-onebox/` = `init` 铺出来的那个目录。

## 目录

- [§1 假红与假绿（先读这节）](#1-假红与假绿先读这节)
- [§2 首次用：init 就没过去](#2-首次用init-就没过去)
- [§3 起栈就失败](#3-起栈就失败)
- [§4 打不开 / 路由不通](#4-打不开--路由不通)
- [§5 认证、自省与跨应用交换](#5-认证自省与跨应用交换)
- [§6 注册与种子](#6-注册与种子)
- [§7 命名卷属主不对（EACCES 一族）](#7-命名卷属主不对eacces-一族)

---

## 1. 假红与假绿（先读这节）

共同点是：**你看到的状态位在说谎**，照着它查会一路查歪。

| 你看到 | 真相 |
| --- | --- |
| `portal-api` / `files-server` / `git-server` … 长期 `(unhealthy)`，但功能一切正常 | **先读探针再判**：`docker inspect <容器> --format '{{json .Config.Healthcheck.Test}}'`。是 `curl -fsS …/health` ⇒ **假红**，且说明这盒的 compose 是旧镜像那份（精简镜像没装 `curl`，`.State.Health` 里清一色 `curl: not found`）；已经是 `bun -e fetch(...)` ⇒ **真红**，去看 `"$S" dc logs <服务>`。判活一律以 `"$S" smoke` 为准；pg/redis/RustFS 的 healthy 一直是真的 |
| `reverse-proxy` 长期 `(unhealthy)`，而 `https://localhost/` 一切正常 | **旧镜像上必然的假红**（与站点地址有关，不是 Caddy 坏了）：那条探针是 `wget -q -O /dev/null http://127.0.0.1:80/`，`XGENT_SITE_ADDRESS` 一旦是主机名，Caddy 就把它 308 到 `https://127.0.0.1/` —— 对一个 **IP** 建 TLS 没有 SNI，`on_demand_tls` 选不出证书，回 `SSL alert number 80`（`docker inspect <容器> --format '{{range .State.Health.Log}}{{.Output}}{{end}}'` 能看到）。新镜像换成了不跟随重定向的 `curl -fsS -o /dev/null http://127.0.0.1:80/`，308 本身即判活 |
| `up` 提示「资产是旧版」/ 没有 rustfs 服务或切换脚本 | 指定新版 tag 跑 `"$S" upgrade --image <ref>` 后 `up`；沿用 latest 先 pull，再 upgrade → up。init --force 是重置，不用于升级 |
| RustFS 控制台端口根路径返回 403 | 打开 `http://localhost:<MINIO_CONSOLE_PORT>/rustfs/console/`；存活探测走 API 口 `/health` |
| PostgreSQL 报版本不兼容，或升级前检查提示未知/冲突数据布局 | 保留现有卷和原镜像，勿 down -v。脚本按卷内 PG_VERSION 保留已有大版本与挂载点；不能识别时先核对布局，不初始化空库。真正的大版本迁移另行备份并用 pg_upgrade/导出恢复 |
| 对象存储切换未提交、退出 10/20 | 按脚本提示处理并保留备份；原服务会在安全条件满足时恢复。已检测到正式 RustFS 时不要删除新卷 |
| 已提交但 RustFS 不健康 / 回退提示数据有变化 | `rustfs-data` 是权威，勿删。回退先隔离正式服务，变化时拒绝回旧卷，需复制新卷做人工核验；细节见 SKILL.md §6 |
| `curl http://localhost/api/health` → 404 `路由不存在` | **探错路径了。** 门户健康端点是 `/health`，不带 `/api`；各服务是 `/svc/<key>/health` |
| 改了 `compose.env` 却「没生效」 | 这份文件是**拼装**出来的（基础模板 + 一盒增量 + devkit 增量 + 本机覆盖块），同一个键出现三四次很常见。**docker compose 后定义者胜**，你多半改在了中间那处。`"$S" env` 会把「同键多个不同值」标出来，并显示实际生效的那个。改配置一律往**文件最末尾**加 |
| 一盒里 `bun --filter @xgent/<某个>-server …` 报 `no packages matched the filter` | **刻意的。** 精简镜像只保留 `files` / `llm-gateway` / `git` / `org` 四个基础服务的代码，其余在构建时就删掉了（`ingest` 已移出基础集） |
| `bootstrap:prod` / 部署控制器一启动就退出并打印拒绝原因 | **刻意的。** 一盒是调试底座，不是门户，这两样启动即拒 |
| 完整的 `db:seed` 失败 | 它会拉起十几个 App 的种子链，而那些代码不在镜像里。一盒只能用 `db:seed:onebox` |
| 视频没有海报、网格缩略图变成图标 | 一盒不装 ffmpeg。`PREVIEW_MEDIA_CONVERTER_URL` 必须**留空**；填 `auto` 会让每次转换去 exec 一个不存在的二进制 |
| 文件管理里打开任何文件都是下载卡，看不到预览 | **刻意的。** 一盒构建时关掉了前端的格式渲染器（省 303 MB 幻灯片字体与十来个 chunk）。服务端的预览描述符 API 不变——你的 App 自带 `@xgent/file-preview` 时照常渲染 |

---

## 2. 首次用：init 就没过去

| 症状 | 成因与修法 |
| --- | --- |
| `init` 停在「没有 puller 凭证」 | **这是设计好的硬停。** 镜像仓库不开放匿名拉取，让用户去找他们**自己的开发团队**要一份 puller key（团队共用的只读凭证），按 SKILL.md §0「先配 puller key」落到 `.xgent-registry.env`。⚠️ **不要猜仓库域名、不要从别的项目里翻凭证**——猜出来的地址只会换一个更难懂的报错 |
| `PULLER_AUTH 不是 base64 的「用户名:口令」` | 值填成了明文口令，或 base64 里混进了换行。重算一遍：`printf '%s' '<用户名>:<口令>' \| base64`（用 `printf` 不用 `echo`——`echo` 会多一个换行，编出来的串是错的） |
| 配好了却还是报「没有 puller 凭证」 | 脚本没读到你那份文件。它按顺序找 `$XGENT_REGISTRY_CONFIG` → `./.xgent-registry.env` → `${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env` → `~/.xgent-registry.env` → skill 目录下的 `registry.env`，而 `./` 指的是**你运行命令时的目录**。显式指路：`XGENT_REGISTRY_CONFIG=/abs/path/to/.xgent-registry.env` |
| `docker login … 失败` / `unauthorized: unauthorized to access repository` | 口令已被轮换（团队共用凭证，别人离职就会换），或 `~/.docker/config.json` 里留着**陈旧** auth。⚠️ 陈旧凭证不会提示你去登录，只会一直 401 —— 先 `docker logout <registry>`，再让开发团队补一份新的 `PULLER_AUTH` |
| `denied: requested access to the resource is denied` | 你在 **push**。puller 是只读账号；你自己 App 的镜像推你自己的仓库 |
| 拉 proxy 镜像时 `not found`，runtime 却拉下来了 | 代理镜像名是从 runtime **推出来**的（同仓同版本、仓库名 `proxy`），有的部署不叫这个。用 `--proxy-image <完整tag>` 显式指定 |
| 「镜像域名与配置里的 REGISTRY 对不上」 | `--image` 的域名段和 `REGISTRY` 不是同一个 —— 登录 A 却去 B 拉，必然 401。对齐这两个再来 |
| 想确认账号到底能不能用 | **判据只有 `docker pull` 本身。** 别拿 `curl` 去探仓库的管理 API（`/v2/_catalog` 之类）——只读账号在那里的 401/403 跟能不能拉是两回事 |
| `manifest unknown` / `not found` | 按顺序查两条。① **项目名不对**——一盒镜像与**你自己 App 的镜像不在同一个项目**下，而 `.xgent-registry.env` 里的 `PROJECT` 是后者（`xgent-image-push` 用的那个）。填 `ONEBOX_PROJECT=<一盒的项目名>`（找开发团队要），或 `--image` 给全。脚本回退用 `PROJECT` 时会打一行黄色告警，**那行就是答案**。② **tag 写错**——一盒只有 `:latest`（可变指针）和 `:v<版本>-<7位sha>`（不可变）两种，别自己编版本号 |
| 拉过了却还是旧的一版（修好的 bug 又出现） | 用的是 `:latest` 而本地已经有一份同名的 —— `docker run` / compose **默认不回仓库查**。先 `onebox.sh pull` 更新 latest 缓存，再 `upgrade` → `up` 同步资产；要钉住某一版用 `upgrade --image <ref>` |
| `no matching manifest for linux/amd64` | 一盒只发 arm64（它是给开发机用的调试底座）。`--platform linux/arm64` 硬拉下来容器也起不来（`exec format error`）。要 amd64 找开发团队 |
| **你自己的** `app-backend` 起来就 `exec format error`（或 `no matching manifest`） | 架构反了：生产镜像按规矩是 **amd64**，而开发机多半是 arm64，devkit 跟随本机架构、不会替你转译。叠一层 override 指定平台（SKILL.md §1 有现成的 `app-platform.yml`），Apple Silicon 上走 Rosetta，慢但能跑。⚠️ 别为了本地方便去发一个 arm64 的生产 tag |
| 卡在 `Retrying in N seconds` 直到超时 | 链路或代理。把 `HTTP_PROXY`/`HTTPS_PROXY` 与 Docker Desktop 的代理设置对齐，或临时关掉代理重试 |
| `x509: certificate signed by unknown authority` | 多半是仓库地址写错（不带端口、不带 `https://`）。**不要**去 `daemon.json` 加 `insecure-registries` 绕过 |
| `init` 说「镜像里没有 /app/deploy」 | 你给的不是门户一盒镜像（比如给成了自己 App 的镜像，或代理镜像）。`--image` 要的是 **runtime** 那个 |
| `init` 说 compose.env 已存在 | 保护你的密钥不被覆盖。只想补配置就直接往那份文件**末尾**加；确实要整份重来才加 `--force` |
| 后续命令报「没找到一盒目录」 | 你不在 repo 里跑，或 `init` 落在了别处。用 `XGENT_ONEBOX_HOME=/abs/path/to/portal-onebox` 显式指定 |

---

## 3. 起栈就失败

| 症状 | 成因与修法 |
| --- | --- |
| `port is already allocated` | 一盒只发布 6 个宿主端口：80/443（`HTTP_PORT`/`HTTPS_PORT`）· 5432（`POSTGRES_PORT`）· 6379（`REDIS_PORT`）· 9000/9001（`MINIO_PORT`/`MINIO_CONSOLE_PORT`）。`init` 会自动避开当时被占的，但你**之后**又起了别的东西就会撞——改 `compose.env` 末尾那几行。⚠️ 改了 `HTTP_PORT` 要同步 `PORTAL_BASE_URL` 与 `FILES_APP_URL`，否则浏览器侧的绝对链接指错端口（`"$S" env` 会告警） |
| 起了一盒，**别的** compose 栈的容器被停/被接管 | 两套栈同名。`COMPOSE_PROJECT_NAME` 相同 ⇒ compose 认为是同一项目，容器名冲突、命名卷共享。一盒项目名固定 `xgent-onebox`，同机只该有一套——已有在跑的一盒时别再 init 第二套，把你的 App 接进现有那套（SKILL.md 文首「一套就够」） |
| 报缺 `APP_IMAGE` / `APP_KEY` | `docker-compose.app-dev.yml` 里的 `${APP_IMAGE:?}` 是**解析期**求值的，跟 profile 无关。`$S` 会按 `compose.env` 里这两行有没有值自动决定带不带那层——手敲 compose 时才会撞上 |
| 报缺 `APP_FRONTEND_DIST` | 你给 `service` 型 App 叠了前端 override。service 无前端，不要那层 |
| 有服务起来就 crash-loop 刷屏，日志淹没真问题 | 少叠了一盒那层 override（`onebox/docker-compose.onebox.yml`）。它的作用就是关掉两个在精简镜像里跑不起来、却在基础 compose 里默认启动的服务。用 `"$S" dc` 不会漏 |
| portal-api 启动即退，日志提到 `DEV_MOCK_OAUTH` | `NODE_ENV` 没被覆盖成 `development`。镜像烘的是 `production`，而 `DEV_MOCK_OAUTH=true` 在 production 下被拒绝 |
| 一盒把你本机某个数据库写花了 | 你关掉了 `local-infra` 让一盒连本机的 PG。库名会撞（`xgent-portal` / `xgent-files` …），而 `db:seed:onebox` 是**会往里写**的。别这么做——一盒内部走的是 compose 网络里的 `postgres:5432`，与宿主端口无关，错开发布端口就够了 |
| 你的 App 自己的库不存在 | 一盒的 postgres 首次初始化只建了它认识的那批 `xgent-*` 库（新镜像另按 `APP_KEY` 建**你自己**那一个）。别的 App 的库要自己建：`"$S" dc exec postgres psql -U postgres -c 'CREATE DATABASE "xgent-<key>"'` |
| **反代整个起不来、全站 502**，`"$S" dc logs reverse-proxy` 里有 `duplicate input` / `adapting config` | 你给一个**已内联**的 key 写了 `/svc` 放行 map。`knowledge` / `omni-parser` / `task-gateway` / `pagebuilder` 直接写在容器版 Caddyfile 的 map 块里，本来就放行；同名键再来一遍 Caddy 直接拒绝加载**整份配置**——挂的不是那一条路由，是反代。删掉再起：<br>`"$S" dc exec -u root reverse-proxy sh -c 'rm -f /etc/caddy/svc-allow/{knowledge,omni-parser,task-gateway,pagebuilder}.map'` 然后 `"$S" dc restart reverse-proxy`。<br>注册链本身不会这么写（`registerFromManifest` 对内联 key 跳过并回 warning）——**是照着老文档手工补那一行**造成的 |

---

## 4. 打不开 / 路由不通

| 症状 | 成因与修法 |
| --- | --- |
| `/svc/<key>/...` → **404** | `/svc` 放行 map 没写成，或**反代在写 map 之前就起了**（Caddy 启动时才读那个目录，不会热重载）。跑 `register-app`，然后 `"$S" dc exec reverse-proxy caddy reload`（`add` / `up` 注册完会自动重读；只有自己手工跑 `register-app` 时才要这一步） |
| `register-app` 输出里有 `/svc 放行未写成 … EACCES: permission denied`，随后 `/svc/<key>` 404 | 命名卷属主不对，见 **[§7](#7-命名卷属主不对eacces-一族)**（那一节的修法一次修好全部三个卷）。注册本身是成功的，缺的只是那一行放行 |
| `/svc/<key>/...` → **502** | 后端不在。① 容器没起/崩了：`"$S" dc logs app-backend`；② 没监听容器内 **8080**（反代的通用规则是 `/svc/<key>/* → <key>-server:8080`，多数镜像认 `PORT`，你的若要别的变量名就在 `compose.env` 里补）；③ `APP_KEY` 与 manifest 的 `listingKey` 不一致，网络别名 `<key>-server` 没命中 |
| 探测全是 **000** | 反代根本没起，或你探的端口不是 `HTTP_PORT`。`"$S" dc ps` / `"$S" dc logs reverse-proxy` |
| 后端跑在**宿主**上，`/svc/<key>` 502 | 反代在容器网里解析不到宿主进程。用 SKILL.md §5 的 socat 转发容器顶住那个别名 |
| iframe 404 / 空白 | `APP_FRONTEND_DIST` 必须是**绝对路径**且目录里有 `index.html`（相对路径会按 compose 文件所在目录解析，很迷惑）。另一半原因是前端构建时的 base 不是 `/apps/<key>/`——资源路径会全部指错 |
| 浏览器整站打不开 | 先 `"$S" smoke` 看 `/health`。通了就是地址问题（改过端口就不是 `http://localhost`）；不通看 `"$S" dc logs reverse-proxy portal-api` |
| `service` 型 App 在应用市场/应用中心**看不到** | **设计如此**：service 型对用户隐藏（无前端、不可打开），平台控制台的清单管理里仍可见可治理 |

---

## 5. 认证、自省与跨应用交换

| 症状 | 成因与修法 |
| --- | --- |
| 自省 **401** | 服务账号 Basic 不对：manifest 的 `serviceAccount.secret` 与你后端实际读的 `<PREFIX>_SA_CLIENT_SECRET` / `<PREFIX>_RESOURCE_KEY` 不相等。两边读的是同一份 `compose.env`，对齐了就一定一致 |
| 门户自省为 `active:true` 的 TDT，被你的后端判 `INVALID_TOKEN` | 后端没解包自省信封：`claims = body.data ?? body`。这是外部 App 最常见的第一个坑 |
| `EXCHANGE_NOT_ALLOWED` | 来源 listing 没声明 `exchangeTargets` / 目标 App 没安装 / 没建授权 / 用户没同意，四选一 |
| 交换在**发起方** 401 | 发起方 App Secret 还没写进去——它绑的是**已安装实例**。顺序：`register-app` → 在市场里装上你的 App → **再跑一次 `register-app`**（幂等） |
| 跨应用取数据的下拉列表莫名为空 | 多半是用户级的同意记录缺行（布线晚于同意）。在门户里对该 App 重新走一次授权屏 |
| 切了租户还能看到上一租户的数据 | 你的后端没按 `claims.tenant_id` 隔离。这是运行时契约的硬要求，一盒不替你实现 |

## 6. 注册与种子

| 症状 | 成因与修法 |
| --- | --- |
| `register-app` 成功、控制台「清单管理」也看得到，但**演示租户的应用市场里没有卡片** | 授予行没写成。市场对租户是 **fail-closed** 的：没有 `tenant_listing_grants` 行就连卡片都不出现，且**不报错**。dev 模式的 `register-app` 本该顺手授予现有租户——**旧一点的一盒镜像里没有这段代码**，所以按 SKILL.md §6 更新缓存后 `upgrade` → `up` 换新镜像与资产重跑一遍；换了还没有，就用平台管理员账号在控制台「租户 → 可用应用」里把它勾上（或 `PUT /api/console/tenants/<id>/apps`），再回市场安装 |
| `register-app` 报 `VALIDATION_FAILED`（scope） | manifest 声明了**别的 App 的 scope**，却没把那个 App 列进 `exchangeTargets`。规则：一个 listing 能声明的 scope = 平台基础 scope ∪ 本 namespace（`<listingKey>` 及其下划线变体，如 `omni-parser` → `omni_parser`）∪ 已声明 `exchangeTargets` 的 namespace |
| `register-app` 报 `DEPENDENCY_UNAVAILABLE`（依赖的应用清单不存在：`<dep>`） | 清单 `dependencies` 里的 App 在这台一盒里还没有**注册**（跟装没装无关）。先 `"$S" add <dep>`（`add` 会在拉镜像之前预检，把缺的一次列全）。如果你自己的 App 是靠 `up` 注册的，补完依赖后**单独**重跑注册那一条：`"$S" dc run --rm -v "$PWD:/devkit:ro" portal-api bun run register-app /devkit/app.manifest.json`。**别重跑 `up`**：它的种子会 truncate 清单表，刚加的依赖又没了。之后在「应用管理 → 应用市场」里安装你的 App，依赖会一起装上 |
| `register-app` 拒跑，提到 production | dev 模式的 `register-app` 拒绝 `NODE_ENV=production`。`compose.env` 末尾必须有 `NODE_ENV=development` |
| 跑完 `db:seed:onebox` 后市场里找不到你的 App | **顺序反了。** 种子第一步是 `truncate … marketplace_listings … cascade`，先注册后种子 = 注册被清掉。必须 seed 在前、register-app 在后 |
| `seed:onebox` 报 `unknown listing key` | 那个 key 既不是内置基础服务，`app-devkit/manifests/<key>.manifest.json` 也不存在。要么放一份 manifest 进去，要么把它从 `XGENT_APP_CATALOG` 里去掉 |
| seed 之后浏览器要求重登 | 种子重新生成 UUID，会话随之失效。重走一次 dev 登录，不是坏了 |
| 发布/上传前端产物报 `前端产物目录不可写：/srv/www/apps（EACCES）` | 同一条根因，见 **[§7](#7-命名卷属主不对eacces-一族)**。⚠️ **别照错误提示去改 `XGENT_APPS_DIR`**：那条建议只对 pm2/单机部署成立。容器栈里 `/srv/www/apps` 是**与反代共享**的卷、反代就从它服务 `/apps/<key>/`，指向别处会让发布「成功」而页面白屏/404——一个更难查的症状 |
| dev 登录后看不到某个 App | 用 `liming@xgent.ai`（普通成员）复验 —— **ACL 成员基线没到位的问题只在非管理员身上现形**，管理员那边永远是绿的 |

---

## 7. 命名卷属主不对（EACCES 一族）

一个根因，两个常见落点：`register-app` 写 `/svc` 放行 map（`/etc/caddy/svc-allow`）、发布前端产物
（`/srv/www/apps`）。第三个 `/etc/caddy/apps-csp` 同理，只是更少被走到。

**成因**：空的命名卷，属主由**第一个挂它的容器**决定。反代是 root，portal-api 是 `bun`(uid 1000)——
旧镜像里这三个目录压根不存在，于是谁先起谁定调，卷落到 root 手里，portal-api 就写不进去了。
`one-box` / `proxy` 的 **v1.2.0 起**已在 runtime 层预建这三个目录并 chown 给 `bun`（`latest` 已指向它）。

镜像升级不会自动改变已有卷的属主。先按 SKILL.md §6 更新镜像缓存，再用 `"$S" upgrade` 同步部署资产，再按下面的定点修法纠正
这三个目录的属主；不需要删除 PostgreSQL 或对象存储卷。随后 `"$S" up` 会按一盒既有规则重种门户库。

若主动选择丢弃所有当前联调数据，可按 SKILL.md §6 的重置路径执行 `dc down -v` → `init --force` → `up`。
重置会丢失 App 数据与对象；它不是升级的必经步骤。

### 定点修复属主

反代是 root 且挂着同样这三个卷，所以补属主不必进 portal-api：

```bash
"$S" dc exec -u root reverse-proxy \
  chown -R 1000:1000 /srv/www/apps /etc/caddy/svc-allow /etc/caddy/apps-csp
```

三个一起补——只补当前报错的那个，剩下两个会在下一步再拦你一次。chown 完直接重试原操作，
**不用重启 portal-api**。`/svc` 放行那条还要补一行 map 并 reload：

```bash
"$S" dc exec reverse-proxy sh -c \
  'printf "<key> \"1\"\n" > /etc/caddy/svc-allow/<key>.map && caddy reload --config /etc/caddy/Caddyfile'
```

换到 v1.2.0+ 且是全新的卷之后还这样，把它反馈给门户团队。

