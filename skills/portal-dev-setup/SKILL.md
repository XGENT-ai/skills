---
name: portal-dev-setup
description: '在你自己 App 的 repo 里，用 Docker 起一个真实的 XGENT.ai 门户（一盒 / one-box）做本地联调——不 clone 门户、不改门户代码、不重建门户镜像。一条 onebox.sh init 检查本地 puller 凭证（.xgent-registry.env 里的 REGISTRY + PULLER_AUTH，缺了就停下来让用户先去开发团队要）→ 自动 docker login 并拉门户镜像 → 从镜像里取出 compose 资产、挑空闲端口、生成 compose.env；再按固定顺序 migrate → seed → register-app → 起栈，然后 iframe/curl 跑通全链路。凡任务涉及「首次把我的 App 接到门户上调试 / 起一盒 / 起本地门户 / 拉门户镜像 / 本地跑不通门户联调 / 一盒重置」，或出现 拉不到门户镜像、unauthorized、denied、/svc/<key> 404 或 502、localhost 打不开、port is already allocated、COMPOSE_PROJECT_NAME 撞栈把别的容器接管了、portal-api 一直 unhealthy、register-app 报 VALIDATION_FAILED、自省 401、有效 TDT 被判 INVALID_TOKEN、iframe 白屏、跨应用交换在发起方 401 这类症状时，务必先用本 skill 再敲 docker compose——即使用户只说「把门户跑起来」。Use when an external App team brings up, smoke-tests, resets, or debugs a real XGENT portal locally (the one-box Docker stack) from their own repo, without the portal monorepo.'
---

# 门户一盒 · 在自己 repo 里起一个真实门户

**这个 skill 用在 App 自己的 repo 里**——门户代码不在你手上，也不需要在。它起的是一个跑在 Docker 里的
**真实门户**（精简镜像），你的后端/前端接上去，走的就是生产同一套协议：TDT 自省、四道闸、
iframe 托管、跨应用令牌交换。

需要的一切都在这里的 `scripts/` 与 `references/`，加上一件事：**门户镜像**。

## 0. 先备齐三样

| 你需要 | 从哪来 |
| --- | --- |
| **puller key** | 找**你们自己的开发团队**要。镜像仓库不开放匿名拉取，没有它一盒起不来——见下面「先配 puller key」。**镜像 tag 不用问**：一盒和它的反代都挂了 `latest`，`init` 不给 `--image` 就用它 |
| `listingKey` | 平台给的 App 标识（小写字母/数字/连字符）。它同时是 `/svc/<key>`、iframe 路径 `/apps/<key>/`、scope 命名空间、令牌的 `aud`——四位一体，永不改 |
| `app.manifest.json` | 你自己写、放你自己 repo 的门户契约。样例在 init 之后会出现在 `portal-onebox/app-devkit/manifests/`（一个 micro、一个 service） |

后端镜像可以晚一步：**先把门户本身跑通**是对的第一步，你的 `app-backend` 没配也不影响门户起来。

### 先配 puller key

`init` 会自己 `docker login` + `docker pull`，前提是本地有一份只读拉取凭证。**没有就会停在这一步**——
这时**不要去猜仓库地址、也不要从别处翻凭证**，让用户去找他们自己的开发团队要。

```bash
cp .claude/skills/portal-dev-setup/puller.env.example ./.xgent-registry.env
chmod 600 ./.xgent-registry.env
# REGISTRY=<仓库域名>     不带 https://、不带端口、无尾斜杠
# PULLER_AUTH=<base64>    base64 的「用户名:口令」：printf '%s' '<用户名>:<口令>' | base64
# PROJECT=<项目名>        省掉 --image 时用它拼 <REGISTRY>/<PROJECT>/one-box:latest
```

`PULLER_AUTH` 和 `~/.docker/config.json` 里 `auths.<REGISTRY>.auth` 是同一个值，两边可以互抄。
查找顺序：`$XGENT_REGISTRY_CONFIG` → `./.xgent-registry.env` →
`${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env` → `~/.xgent-registry.env` → skill 目录下的 `registry.env`。
**同名环境变量优先**，CI 里注入 `REGISTRY` / `PULLER_AUTH` 即可，不必落盘。

- 这是**团队共用的只读凭证**，不是谁的个人密码：有人离职要找开发团队轮换，而轮换会同时作废全团队的现有凭证。
- ⚠️ **base64 不是加密**，`base64 -d` 一条命令就还原成明文。别提交进 git（skill 目录自带 `.gitignore` 挡了这两个文件名），别转发出团队。
- 拿到的是**离线 tar**（没有 registry 访问）就先 `docker load` 两个镜像——两个都在本地时 `init` 会跳过登录与拉取，不需要 puller key。

## 1. 首次用：一条命令铺好

```bash
.claude/skills/portal-dev-setup/scripts/onebox.sh init --key <你的listingKey>
```

不给 `--image` 就取 `<REGISTRY>/<PROJECT>/one-box:latest`（反代同仓同 tag）——**开新项目不用先去问 tag**。
`latest` 是**可变指针**：它跟着最新一版走，本地已经有同名镜像时 compose 不会回仓库看一眼，
所以开工前先 `onebox.sh pull` 一次。要钉住某一版（复现一个 bug、或团队统一版本）就显式
`--image <registry>/<项目>/one-box:v<版本>-<sha>`，那种 tag 是不可变的。

它做五件事，都在**你的 repo 根**落到 `portal-onebox/`：

0. **备齐镜像**：读上面那份配置 → `docker login` → 拉 runtime 与 proxy 两个镜像。
   proxy 的镜像名默认从 runtime 推出来（同仓同版本、仓库名 `proxy`），有的部署不叫这个，
   拉不到时用 `--proxy-image` 显式指定。**这一步在最前面，凭证不齐就停在这儿**，
   不会让人跑到一半才发现，也不会留下半个空目录。
1. **从镜像里取出 compose 资产**（`docker create` + `docker cp /app/deploy`）——所以版本天然与镜像对齐，
   不需要门户仓，也不会有「文档里的 compose 和镜像对不上」这种漂移。
2. **挑空闲端口**。一盒只发布 6 个宿主端口（80/443 · 5432 · 6379 · 9000/9001），
   开发机上这些十有八九被占——它逐个探测并退让，把结果写进 `compose.env` 并回显。
3. **生成 `compose.env`**：三段模板拼装 + 一段本机覆盖块（随机密钥、端口、项目名、镜像 tag）。
   覆盖块**放在文件最末尾**，因为 docker compose 读 env-file 是**后定义者胜**——以后要改配置也往那下面加，别回上面改。
4. 把 `portal-onebox/` 加进 `.gitignore`（里面是密钥，且随时能重新生成）。

它**不会**覆盖已存在的 `compose.env`（那是这台机器唯一的一份密钥）。要整份重来加 `--force`。

跑完编辑 `portal-onebox/compose.env` 末尾，填你的 App：`APP_IMAGE`（后端镜像 tag），
micro 型再加 `APP_FRONTEND_DIST`（前端 dist 的**绝对路径**，目录里要有 `index.html`）。

> `service` 型（无前端、纯 API）**不要**填 `APP_FRONTEND_DIST`——它不露卡、不可打开，只作被调用方。

之后一切都走同一个脚本，它会按生效的 env 拼好那串 `--env-file` / 四层 `-f` / 一组 `--profile`：

| | |
| --- | --- |
| `onebox.sh status` | 生效 env + 容器状态 + 宿主侧健康探测（**出问题先跑这个**） |
| `onebox.sh env` | 只看生效 env 与配置告警 |
| `onebox.sh smoke` | 只跑健康探测 |
| `onebox.sh dc <args>` | `docker compose <拼好的参数> <args>` |
| `onebox.sh chain` | 打印那串参数（想自己敲 compose 时抄它） |
| `onebox.sh pull` | 只（重）拉门户镜像。不带参数时从 `compose.env` 读当前版本 |

## 2. 起栈：顺序本身是契约

```bash
S=.claude/skills/portal-dev-setup/scripts/onebox.sh

# 1) 基础设施
$S dc up -d postgres redis minio

# 2) 门户库迁移 + 一盒种子   ★ 破坏性：truncate cascade，清掉租户/用户/清单/安装
$S dc run --rm portal-api bun run db:migrate
$S dc run --rm portal-api bun run db:seed:onebox

# 3) 四个基础服务各自的库（xgent-* 库由 postgres 首次初始化时自动建好）
for k in files ingest llm-gateway git; do $S dc run --rm portal-api bun run db:$k:migrate; done

# 4) 注册你的 App：读 manifest → 建 listing + 服务账号 + 写 /svc 放行 map
$S dc run --rm -v "$PWD/<放 manifest 的目录>:/devkit:ro" \
   portal-api bun run register-app /devkit/app.manifest.json

# 5) 起门户三件套 + 基础服务 + 你的 app-backend
$S dc up -d
```

三条顺序约束，颠倒了症状都很难反查：

- **`db:seed:onebox` 必须在 `register-app` 之前。** 种子第一步是 `truncate … marketplace_listings … cascade`——
  先注册后种子 = 你的 App 注册被静默清掉，市场里找不到它。
- **`register-app` 最好在 `reverse-proxy` 之前。** 它落的是一个 `/svc` 放行 map 文件，而反代**启动时**才读那个目录。
  反代已经在跑就补一句 `$S dc exec reverse-proxy caddy reload`，否则 `/svc/<key>` 一直 404。
- **要做跨应用交换的，`register-app` 得跑两次。** 发起方 App Secret 绑在**已安装实例**上，所以是
  `register-app` → 在应用市场里装上你的 App → **再跑一次 `register-app`**（幂等）。漏了的症状是交换在发起方 401。

一盒里带四个基础服务：`files`（文件管理）· `ingest`（信息获取）· `llm-gateway`（大模型网关）· `git`（Git 服务）。
你的 App 要用它们的数据，就在 manifest 里声明 `exchangeTargets` 走令牌交换。

## 3. 冒烟

```bash
.claude/skills/portal-dev-setup/scripts/onebox.sh smoke
```

探 `GET /health`（门户；注意**不是** `/api/health`，那个 404）和每个 `GET /svc/<key>/health`。
全绿再开浏览器（地址以 `init` 回显的为准，改过端口就不是 `http://localhost`）：
dev 登录 → `rockie@xgent.ai`（演示租户 admin + 平台管理员）。

种子只种两个账号，另一个是 `liming@xgent.ai`（普通成员）——**ACL 成员基线没到位的问题只在非管理员身上现形**，
验收要用它再走一遍，管理员那边永远是绿的。

- **micro**：应用市场安装 → 应用中心打开 → iframe 加载 `/apps/<key>/` → `sdk.ready()` 握手 →
  `sdk.callService()` 通 → 首次跨应用读数据弹交换授权页。
- **service**：无 UI，走 curl —— `/health` 判活 · 缺/错 token 应 401/403（四道闸）· 正确 TDT 应 200 信封。

> 你的后端要实现的运行时契约（验 TDT + 四道闸、解包自省信封 `claims = body.data ?? body`、
> 按 `claims.tenant_id` 隔离、`/health`、容器内统一监听 **8080**）不在本 skill 范围内——
> 一盒不替你实现。完整说明在 init 之后的 `portal-onebox/app-devkit/README.md`。

## 4. 出问题了

先 `onebox.sh status`，再对着 **[references/troubleshooting.md](references/troubleshooting.md)** 对号入座。
那张表按「你看到什么」编排。最容易白白浪费半天的两条先放这儿：

> **`portal-api` / `*-server` 显示 `unhealthy` 是假红，不用查。** 一盒镜像没装 `curl`（省体积），
> 而 compose 的 healthcheck 写的正是 `curl -fsS …/health`，于是**永远**失败——`docker inspect` 里
> 看到的是 `curl: not found`。判活只认宿主侧探测。`reverse-proxy` 和 pg/redis/minio 的 healthy 是真的。

> **`COMPOSE_PROJECT_NAME` 必须唯一。** 同机跑两套 compose 而项目名相同，compose 会认为它们是同一项目——
> 容器互相接管、命名卷共享，症状是「我起了一盒，结果把另一套的容器停了」。`init` 已经给你设成 `onebox-<key>`。

## 5. 后端想跑在宿主上（保留热重载）

`app-backend` 默认是**镜像**，靠网络别名 `<key>-server` 被反代解析到；宿主上 `bun dev` 起的进程在容器网里
没有这个名字，而每改一行就重建镜像等于没有热重载。绕法是让一个转发容器顶住那个别名：

```yaml
# portal-onebox/host-backend.yml —— 叠在 app-dev.yml 之后
services:
  app-backend:
    image: alpine/socat
    command: TCP-LISTEN:8080,fork,reuseaddr TCP:host.docker.internal:<你 dev server 的端口>
    extra_hosts: ["host.docker.internal:host-gateway"]
```

然后 `XGENT_ONEBOX_HOME=portal-onebox $S dc -f portal-onebox/host-backend.yml up -d app-backend`。
此时 `APP_IMAGE` 不再被用到（compose 仍要求它有值，随便填一个）；`APP_KEY` 照旧——别名还是靠它。

## 6. 重置与拆栈

```bash
S=.claude/skills/portal-dev-setup/scripts/onebox.sh
$S dc down          # 停容器，留命名卷（数据还在，下次 up 接着用）
$S dc down -v       # ★ 连命名卷一起删：pg/minio/apps/caddy 的数据全没
```

只想重置门户数据：重跑 `db:seed:onebox`（同样破坏性），然后**重跑 `register-app`**（你的 listing 被种子清掉了），
最后 `caddy reload`。种子会重新生成 UUID，浏览器会话随之失效——重登一次 dev 登录，不是坏了。

换门户镜像版本：改 `compose.env` 末尾的 `XGENT_IMAGE` / `XGENT_PROXY_IMAGE` 两行，
`$S pull` 把新版本拉下来，再 `$S dc up -d`。compose 资产要不要跟着更新，
重跑 `init --force --home <另一个空目录>` 对比着看。

## 7. 一盒不是门户，别拿它当门户

- 它只带上面四个基础服务，其余门户内置 App 的代码**不在镜像里**——
  `bun --filter @xgent/<别的>-server …` 报 `no packages matched the filter` 是刻意的。
- `bootstrap:prod` 与部署控制器**启动即拒**并打印原因；完整的 `db:seed`（十几个 App 的种子链）也必失败，
  一盒只能用 `db:seed:onebox`。
- 它是 dev 联调套件：mock OAuth 常开、服务账号密钥是已知明文。**不要用于生产，也不要暴露到公网。**
- 不装 ffmpeg：`PREVIEW_MEDIA_CONVERTER_URL` 必须留空，视频海报/网格缩略图退化成图标，
  图片/PDF/文本预览不受影响。
