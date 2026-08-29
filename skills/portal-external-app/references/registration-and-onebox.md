# 注册布线与一盒（one-box）本地联调

> 提炼自门户仓库 `docs/外部App本地联调指南-one-box.md`、`deploy/app-devkit/README.md`、`docs/SSO与App开发指引.md` §15.3（§4 一盒部分对齐 2026-08 的精简镜像与自建 Harbor）（门户仓文件，App 自己的 repo 里没有；本文件已自包含，不必去找）。冲突时以门户仓库为准。

## 1. `app.manifest.json`（单一事实源，放你自己的 repo）

既是交付契约也是注册脚本的输入。字段：

- 身份/展示：`listingKey`（= TDT aud = 服务账号/容器命名基名）、`name`、`version`、`cat`、`tagline`、`desc`、`icon`、`color`
- 形态：`type`（`micro` 有前端 / `service` 无前端）、`embedUrl`（micro：`/apps/<key>/`）、`embedCsp.connectSrc`、`helpEntry`（micro：版头帮助按钮入口，`"/help"` App 内路由 = 自动档 ／ `"https://…"` 外站文档 = 治理档；不声明就不出按钮）
- 权限：`scopes` + `scopeLabels`（同意屏逐 scope 的说明文案，支持多语——形状见下）、`aclManifest`（纯 scope 鉴权可 `null`）、`navItems`（micro）
- 关系：`dependencies`、`exchangeTargets`、`serviceBaseUrl`（`/svc/<listingKey>`）
- 服务态与计量：`usageReporter`（true ⇒ SA 得 `client_credentials` + `usage.report`）、`serviceScopes`（服务态 scope；SERVICE_ONLY **永拒**）、`privilegedServiceScopes`（SERVICE_ONLY 的**申请**：`[{scope, reason}]`，reason 必填，最高治理档、审批逐条确认后授予）、`usageMetrics`（用量指标声明：key 前缀必须=listingKey、金额单位不开放；治理档，批准后 ingest 才认这些 key）
- 部署/运维：`requiredEnv`（部署所需 env 的**键名**清单，值永远由平台配；键集合一变提案转入「发布审核」）、`deployDescriptor`（`image` 归自动档、`hostPort` 归平台不算变更，**其余一切属治理档**）

⚠️ **`deployDescriptor.hostPort`（宿主机发布口）不归 App 定**：它是部署环境相关的事实——同一份 manifest 会发到一盒与好几套生产门户，各自端口地貌不同，而 App 团队看不见目标机器上谁占了什么。规则三句话：**首次注册**当建议值采纳（撞了自动退让到平台端口池 20000–20999，**不会因此拒掉注册**）；**listing 已存在则一律忽略**（那个口已经写进 Caddy 的 `/svc` map、也是在跑的容器发布出来的口）；发布响应的 `warnings` 会说明实际是哪个口。App 只管 `port`（容器内监听口，约定 8080）。
**例外**：`extraPorts[].host` **不参与退让**，撞了硬拒——那些是对外契约口（worker 节点连的就是那个数字），静默挪走等于把它们全断掉，且门户侧一条都测不到。
- dev 专用明文：`serviceAccount{clientId,secret}`（`clientId` 约定 `<key>-server`，且**必须是自己的** —— 服务账号身份归平台，声明已归属另一 App 的 clientId 会进人工审、批准也会在生效时被写入闸拒绝）、`exchangeInitiatorSecret`（本应用作为交换发起方时的 App Secret）

**文案字段的形状**（写错的那一组**不会报错**，所以单列一张表）：

| 字段 | 形状 | 写错的后果 |
| --- | --- | --- |
| `tagline` / `desc` / `icon` / `color` / `cat` / `navItems[].label` / `dashboardWidgets[].title` | **纯字符串**（单语） | 给多语对象 ⇒ 门户把它拼成字面量 `"[object Object]"` 存进 text 列，显示在每个租户的应用卡片与详情上 |
| `name` | 纯字符串 | 给对象不报错，但会被**拍平成 zh-CN**，另外两种语言就此丢掉 |
| `scopeLabels[<scope>]` | 字符串 **或** `{ "zh-CN": …, "zh-TW": …, "en": … }` | 存 jsonb，同意屏按门户当前语言解析；回退链 当前语言 → `zh-CN` → `en` → 键名，所以 **`zh-CN` 是回退终点，必写** |
| `aclManifest` 内的 `label` / `name` / `desc` | 同上（字符串或多语对象） | 角色矩阵按请求 locale 解析 |
| `usageMetrics[].label` | **`{ "zh": …, "en"?: …, "tw"?: … }`** —— 键名与上一行**不是同一套** | 缺 `zh` ⇒ 提交即拒 |

`scopeLabels` 还有两条静默失效：**键不在 `scopes` 里的在写入时被直接丢弃**（同意屏上那条回落成键名，改 scope 名忘了改文案键就中）；
**平台基础 scope（`userinfo.read` / `audit.write` / `notification.*` …）的文案不归 App 写** —— 那是平台统一维护的措辞，
各 App 各写一份，同一条权限在不同应用的同意屏上就会说得不一样，比缺文案更糟；发现平台漏了哪条，反馈给平台补。

**scope 三规则**（写时强校验）：一条清单只能声明——
1. 平台基础 scope（`userinfo.*`/`audit.write`/`notification.*`/`settings.*`/`scheduler.*`/`content.*` 等）；
2. **自己命名空间**：`<listingKey>.` 前缀及连字符→下划线变体（`omni-parser` → `omni_parser.*`）；
3. **已声明 `exchangeTargets` 的目标命名空间**（例：omni-parser 声明 `files.read` 但 exchangeTargets 不含 files → 拒；knowledge 声明 `files.read` 且 `exchangeTargets:["files"]` → 放行）。

越界 → `VALIDATION_FAILED`。App 命名空间 scope 随清单入库，**不用改门户代码**。

## 2. 注册（dev / 一盒）：`register-app`

```bash
bun run register-app <你的>.manifest.json    # 幂等；NODE_ENV=production 拒跑
```

四件事：① 按 listingKey upsert + 发布 listing（复用门户写时校验：scope 命名空间 / ACL 形状 / 依赖无环）；② DB 直写 `token.introspect` 服务账号（secret = manifest 明文；同 secret 不动、不同则轮换）；③ 把 `exchangeInitiatorSecret` 写进**已安装**实例的 App Secret——⚠️ 所以**先在门户安装 App、再重跑一次 register-app**，否则发起交换 401（判定方法见 §2.2，别看日志里那个数字）；④ 往 `caddy-svc-allow` 卷写 `<key>.map` 放行 `/svc/<key>`（免改 Caddyfile、免重启反代）。

### 2.1 第五件事：授予租户（dev 自动 / 生产不自动）

**发布一条 listing 并不会让它出现在任何租户的市场里。** 那道闸 fail-closed 在
`tenant_listing_grants` 上：列表按授予集过滤、安装前 `assertTenantMayUseListing` 无授予行直接
`LISTING_NOT_GRANTED`。授予行本来由 `installListing` 在**安装那一刻**写（「安装即授予」，依赖一并
授予）——这对被依赖方成立，但对**目标 App 自己**是先有鸡还是先有蛋：路由闸先于安装拒绝。

| 路径 | 行为 |
| --- | --- |
| **dev / 一盒**：`register-app <manifest>` | **自动授予当时已存在的所有租户**（非 service 型），输出里有一行 `租户可用应用: 新授予 N 个租户` |
| **生产**：发布提案批准 / 控制台导入 manifest（`mode:"prod"`） | **不自动授予** —— 语义是平台管理员显式勾选，自动授予会绕过治理面 |

⚠️ **判断你手上的一盒镜像有没有这个自动授予**：跑完 `register-app` 看输出里**有没有
`租户可用应用:` 这一行**。没有 ⇒ 镜像早于这次改动，按下面手动补。

⚠️ **自动授予只覆盖「注册那一刻已存在」的租户**：之后新建的租户不会被补授，要么再跑一次
`register-app`，要么手动勾。

**手动补授权（生产必走 / 老镜像上必走）**：**平台 › 租户管理 › 选中租户 › 「可用应用」tab**，
勾上你的 App 再保存（计数会从 `4/5` 变 `5/5`），之后租户市场才出现「安装」。

⚠️ 没授予时的症状极具迷惑性：**平台控制台「清单管理」显示已上架**（还能点「转为草稿 / 下架」，
看着一切正常），但**租户市场里根本不出现这个 App**——没有报错、没有灰掉的卡片，就是不在列表里。
别去查发布状态，那不是病根。

⚠️ `service` 型**永不写授予行**（自动与手动都不写）：它在那张表里的勾选语义是**安装 / 卸载**
（勾上即 `installListing`、取消即卸载），事实源是 `apps.status` 而不是授予表——一个勾选只许一个
事实源。

### 2.2 `wired into N installed instance(s)` 的 N 是「改了几个」，不是「对了几个」

`wireInitiatorSecret` 对 `secretHash === hash` 的实例直接 `continue` —— **不计数**。所以 **N=0 有两义**：
① 还没有任何安装实例；② 所有实例本来就已经是对的。register-app 在 N=0 时打印的
`(none installed yet — install then re-run…)` **只覆盖第 ① 种**，看到它别当成「没写成」。

**判据是比哈希，不是看数字**（三处必须相等）：

```
sha256(<manifest 的 exchangeInitiatorSecret>)
  == 门户 app_secrets 里 prefix='sk_<listingKey>' 且 status='active' 那行的 secret_hash
  == sha256(<你后端 env 的 <PREFIX>_APP_SECRET>)
```

「先装再重跑」那一步**仍然要跑一次**：`installListing` / `wireExchangeTargets` 只写交换 grant + 白名单，
**不写 App Secret**；只有 `register-app`（与 seed 的 provisioning 路径）写它。所以装完之后的那次重跑是
必要的，**之后每次重跑都显示 0，那是正常的**。

## 3. 注册（生产）

生产的清单事实源就是对方仓的 `app.manifest.json`：平台在控制台
「应用市场 › 接入新应用」按 key 签发 `xrel_` 令牌（无行先建 draft 占位），对方
`publish --manifest` 提交，平台在「发布审核」批准 = `registerFromManifest` 建全
（listing 上架 + SA + /svc + 已装租户对齐）。**不要**把外部 App 登记进 `LISTING_DEFS` ——
门户只保留平台侧事实（`EXCHANGE_WIRING` / Caddy 内联行 / 部署行，及作为**种子**的
`SA_DEFS` 与 scope 常量 —— 特权 scope 已可经 `privilegedServiceScopes` 申请、审批授予），
`bootstrap:prod` 对外部 key 只自愈 SA 与部署行。**生产没有 manifest 明文密钥这条路**
（secret 取 env、缺省随机生成、明文只回显一次给审批人）。

⚠️ **secret 一致红线**：平台落库的服务账号 secret 与你镜像 env 里的必须一致。漂移症状 = 自省 401，或「跨应用授权缺失/未开启」（实为 `SECRET_INVALID`）。排查：对你侧 secret 求 sha256 与门户 DB 存的哈希比对。

## 4. 一盒联调流程

分工：平台交**两个门户镜像** + `deploy/` 目录（含 `app-devkit/` 与 `onebox/`）。你交
①manifest ②`compose.env`（本机接线：`APP_KEY`=listingKey、`APP_IMAGE`、micro 才需
`APP_FRONTEND_DIST` **绝对路径**）③你的后端镜像（+ micro 的前端 dist）。
**不 clone 门户、不改门户代码、不重建门户镜像。**

```
<registry>/<项目>/one-box:latest   # portal runtime → XGENT_IMAGE       (~850MB)
<registry>/<项目>/proxy:latest     # caddy proxy    → XGENT_PROXY_IMAGE (~68MB)
```

⚠️ 三条硬前提，缺一条第 0 步就过不去：① **仓库地址与拉取账号都要问平台团队** —— 仓库不开放
匿名拉取，得有一个 **puller 账号**（只读凭证，按团队发放），`docker login <registry>` 用的就是它；
凭证别转发给团队外的人、别写进会提交的文件（`portal-dev-setup` skill 把这两样收在一份
`.xgent-registry.env` 里，照它配即可）。② **这两个调试镜像有 `latest`**（跟着最新一版走，开新项目
不用先问 tag）；代价是本地已有同名 `latest` 时 compose **不回仓库查**，换版本前先 `docker pull`。
要钉住某一版就写 `:v<版本>-<7位sha>`（当前 `v1.1.0-5c1660e`），那种 tag 不可变。③ **一盒只发 `arm64`**（它是给开发机用的调试底座，开发机全是
Apple Silicon），生产门户走另一条链。无 registry 访问时则要离线 `docker save` tar，`docker load` 即可
（那条路上镜像的本地 tag 可能叫 `ai-portal-one-box:local` / `xgent-ai-portal-proxy:local`）。

### 4.1 一盒里有什么，以及刻意没有什么

一盒是**调试底座，不是门户**：它按 `deploy/onebox/Dockerfile` 单独构建，只带四个**基础服务 App**——
`files`（文件管理）· `ingest`（信息获取）· `llm-gateway`（大模型网关）· `git`（Git 服务）。
`git` **依赖 `files`**，两者要么一起在 `XGENT_BASE_APPS` 里、要么一起不在。

下面三件「缺失」都是刻意的，**不是环境坏了**，收到这些报错别当配置问题查：

- 其余内置 App（spms/sms/qbank/lms/…）的 workspace **已从镜像里删掉** ⇒
  `bun --filter @xgent/spms-server …` 直接报 `no packages matched the filter`；
- 镜像烘死 `XGENT_ONEBOX=1` ⇒ `bootstrap:prod` 与 deploy-controller **启动即拒**并打印原因；
- 没有 `ffmpeg` 与 `docker CLI`（合计省 552MB）⇒ `PREVIEW_MEDIA_CONVERTER_URL` 必须**留空**
  （填 `auto` 会让每次转换去 exec 一个不存在的二进制），视频海报/网格缩略图退化成图标，
  图片/PDF/文本预览不受影响。

### 4.2 起栈之前：躲开本机已占的端口与项目名

一盒**只发布 6 个宿主端口**（reverse-proxy 80/443 → `HTTP_PORT`/`HTTPS_PORT`、postgres 5432 →
`POSTGRES_PORT`、redis 6379 → `REDIS_PORT`、minio 9000/9001 → `MINIO_PORT`/`MINIO_CONSOLE_PORT`）；
其余（portal-api 的 3000、各 `<key>-server` 的 8080、你的 app-backend 的 8080）只 `expose`，不占宿主。
撞了就在 `compose.env` 里改这几个；改的只是发布口，一盒内部一律走 compose 网络的
`postgres:5432` / `redis:6379` / `minio:9000`。改了 `HTTP_PORT` 之后门户地址随之变化，下文冒烟里的
`http://localhost/…` 都要跟着改。

⚠️ 两个必改/必不做：

- **`COMPOSE_PROJECT_NAME=onebox-<你的 App key>`** —— 模板里写死的是 `xgent`，同机起两套栈时
  compose 会认为它们是同一个项目：容器名冲突、命名卷被共享，症状是「我起了一盒，结果把另一套的
  容器停了」。
- **不要**关掉 `local-infra` 让一盒去连你本地那套 PG —— 库名会撞（`xgent-portal` / `xgent-files` …），
  而 `db:seed:onebox` 是会往里写的，等于拿一盒的种子污染你的本地开发库。

### 4.3 端到端命令（从 repo 根跑；相对路径解析自第一个 `-f` 文件所在目录）

```bash
# 0) docker login <registry> && docker pull 上面两个 tag（或 docker load tar）+ 你的镜像
# 1) env 三层：基础模板 + 一盒增量 + 本套件增量
#      cp  deploy/compose.env.example                deploy/compose.env
#      cat deploy/onebox/compose.env.onebox.example  >> deploy/compose.env   # ★ 一盒必需
#      cat deploy/app-devkit/compose.env.app.example >> deploy/compose.env
#      填 XGENT_IMAGE / XGENT_PROXY_IMAGE / APP_KEY / APP_IMAGE /(micro)APP_FRONTEND_DIST
#      + 你后端读的 SA 密钥与 DSN（与 manifest 的 serviceAccount.secret 等值）+ 强随机门户密钥
# 2) 起基础设施:  --profile local-infra up -d postgres redis minio
# 3) 门户迁移 + 基线 seed:
#      run --rm portal-api bun run db:migrate
#      run --rm portal-api bun run db:seed:onebox     # ★ 不是 db:seed
# 4) 注册:        run --rm -v "$(pwd)/deploy/app-devkit/manifests:/devkit:ro" \
#                   portal-api bun run register-app /devkit/<你的>.manifest.json
# 5) 起门户三件套 + 你的后端（网络别名 <key>-server:8080）:
#      -f docker-compose.yml \
#      -f onebox/docker-compose.onebox.yml            # ★ 一盒必须叠这一层
#      -f app-devkit/docker-compose.app-dev.yml [micro 加 app-frontend.yml]
#      --profile local-infra --profile app-external up -d reverse-proxy portal-api app-backend
# 5b) 按需起基础 App 后端（同一个镜像，compose command 选跑哪个）+ 各自库迁移:
#      --profile app-files --profile app-ingest --profile app-llm-gateway --profile app-git up -d
#      run --rm portal-api bun run db:files:migrate   # 同理 db:ingest / db:llm-gateway / db:git
# 6) 你自己的库迁移：跑你镜像的迁移 argv（生产由门户经 migrateArgs 自动跑，这里手动跑同一条）
#      run --rm --env-file <同一份> <你的镜像> --role migrate
#    每租户 bootstrap（若你的表 FK 自己的 tenants）：门户目前没有钩子，本地手动建行
```

⚠️ **第 3 步必须是 `db:seed:onebox`**：`db:seed` 会串起十几个 App 的种子链（qbank/lms/exam/task/…），
而它们的 workspace 不在一盒镜像里 —— **必失败**。`db:seed:onebox` 只种 `XGENT_APP_CATALOG` 里的四个基础 App。

⚠️ **第 4 步要跑在起反代之前**：注册会往 `caddy-svc-allow` 卷写 `<key>.map`，反代启动时 `import` 才读得到。

⚠️ **依赖 `files` 的 App 别忘了 5b**：`files` 已在镜像里、`/svc/files` 已放行、市场安装会连带补装它，
但**后端进程不起来**读文件就是 502/401。`FILES_SA_CLIENT_SECRET` 必须与 `db:seed:onebox` 种的 SA secret
等值，否则自省 401。

细节（含 App 自带基础设施如向量库 sidecar、把后端跑在宿主上保留热重载的 socat 绕法）见交付包内
`deploy/app-devkit/README.md`。

## 5. 冒烟

打开 `http://localhost/` → dev 登录选 `rockie`（租户 admin + 平台管理员）。

**第 0 步：确认租户已被授予**（否则下面第 1 步无从点起）。新版 `register-app` 在 dev 下会自动授予并
打印 `租户可用应用: …`；没看到那一行（老镜像）就去 平台 › 租户管理 › 选中租户 › 「可用应用」勾上保存（§2.1）。

⚠️ `db:seed:onebox` 只种两个 dev 账号：`rockie@xgent.ai`（admin）与 `liming@xgent.ai`（普通成员）——
**用普通成员再走一遍**。ACL member 基线没到位的问题（`defaultForMember` 没种进 member 角色）
只在非管理员身上才现形，管理员的 `bypass` 会把它整个盖住。

- **micro**：应用市场安装（连带补装依赖）→ 应用中心打开 → iframe 加载 `/apps/<key>/`、`sdk.ready()` 握手 → `sdk.callService("<key>", ...)` 通 → 跨应用读数据首次弹交换授权页 → 切租户看不到上一租户数据。
- **service**（无 UI 走 curl）：

```bash
curl http://localhost/svc/<key>/health           # {"service":"<key>","db":"ok",...}
curl -X POST http://localhost/svc/<key>/v1/...   # 缺/错 token → 401/403（四道闸生效）
# 拿一个 aud=<key> 的 TDT（经交换，或 dev 自助 /api/tokens/authorize → /oauth/token）→ 200 信封
```

## 6. 排查速查

| 症状 | 病根 |
| --- | --- |
| **租户市场里看不到你的 App**（控制台却显示已上架） | **该租户没被授予**（§2.1）。三种成因：① 一盒镜像早于自动授予那次改动（输出里没有 `租户可用应用:` 行）；② 该租户是注册之后才建的；③ 走的是生产路径（本就不自动）。补法：平台 › 租户管理 › 租户 › 「可用应用」勾上保存。**不是发布状态问题** |
| 安装报 `LISTING_NOT_GRANTED` | 同上（这是同一道闸的显式报错形态） |
| `wired into 0 installed instance(s)` | **未必是没写成**：0 = 本次没有改动，含「本来就是对的」。按 §2.2 比哈希判定 |
| `/svc/<key>` 404 | 白名单 `.map` 没写成，或反代先于 register-app 起（重启反代/重跑注册） |
| `/svc/<key>` 502 | 后端没起 / 没听 8080 / 网络别名 `<key>-server` 没命中 |
| 有效 TDT 被判 `INVALID_TOKEN` | 没解包自省信封（`claims = body.data ?? body`） |
| register-app 报 scope `VALIDATION_FAILED` | 声明了别人的 scope 但没列进 `exchangeTargets` |
| 发起交换 401 | `exchangeInitiatorSecret` 未写进已安装实例（先安装再重跑 register-app）或 secret 漂移 |
| 受门路由 503 | 门户三变量（自省地址/SA id/secret）配置不全 |
| `db:seed`/`bun --filter @xgent/<别的 App>` 报 `no packages matched the filter` | **刻意的**：一盒只带四个基础 App，其余 workspace 已删。seed 用 `db:seed:onebox`（§4.3） |
| 读写文件 502 / 401 | `files` 后端没起（`--profile app-files` + `db:files:migrate`）；401 多半是 `FILES_SA_CLIENT_SECRET` 与种子里的 SA secret 不等值 |
| `git` 连不上本机 Gitea/GitHub | 一盒是 `NODE_ENV=production`，SSRF 守卫默认拒环回与私网 —— 把目标 hostname 显式列进 `GIT_ALLOWED_HOSTS` |
| 视频海报/网格缩略图不出 | 一盒没装 ffmpeg，`PREVIEW_MEDIA_CONVERTER_URL` 应留空（§4.1）；其余预览不受影响 |
| 起了一盒把另一套栈的容器停了 | `COMPOSE_PROJECT_NAME` 没改（§4.2） |
