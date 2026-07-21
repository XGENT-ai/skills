# ONE-BOX — 通用「门户一盒」本地联调套件

> Register: **platform / infra**。目标读者:平台团队(产出套件)+ 外部第三方 App 团队(消费套件)。
>
> **一句话目标**:把今天**只服务知识库**的 [`deploy/knowledge-devkit/`](../deploy/knowledge-devkit/) 泛化成一个
> **App 无关**的「门户一盒」——任何不在本仓库里的独立 App / 服务(知识库、多模态解析器 omni-parser、
> 以及未来批量第三方),只要在**自己的 repo** 里拿到①门户镜像(tar / 私有 registry)②一份 `app.manifest.json`,
> 就能 `docker compose up` 起一个**真实门户**、把自己的后端/前端接上、跑通全链路联调——**不 clone 门户 monorepo、
> 不改门户代码、不重建门户镜像**。
>
> **形态决策(已定)**:做**最小化的真实门户**(复用 portal-api / web / Caddy + pg/redis/minio),不做轻量 mock 桩——
> 真实门户高保真、能抓出真实集成 bug,且本计划基本零新代码(复用既有注册/路由/交换面)。
> **分发决策(已定)**:面向**外部第三方团队 · 黑盒镜像**——对方只需 Docker。
>
> 关联:[`APP-INTEGRATION.md`](./APP-INTEGRATION.md)(新增 App 零改码零重部署的底层重构,本计划是其「本地版」落地)、
> [`DEPLOY.md`](./DEPLOY.md)(生产按需部署,与本地联调正交)、
> [`docs/SSO与App开发指引.md`](../docs/SSO与App开发指引.md)(运行时契约)、
> [`docs/知识库后台接入与本地联调指南.md`](../docs/知识库后台接入与本地联调指南.md)(知识库个案,泛化前的样板)、
> [`docs/omni-parser对接要求.md`](../docs/omni-parser对接要求.md)(`service`/headless App 个案)。

---

## 1. 目标与非目标

**目标**

1. 外部 App 团队**在自己 repo 内**用 Docker 一把起齐:真实门户(三件套 + 基础设施)+ 它自己的后端(+ 前端)。
2. **接入 = 写一份 `app.manifest.json` + 跑一个通用 `register-app` 步骤**;新 App **不需要**改门户任何代码、不需要重建门户镜像。
3. 全链路可联调自测:dev 登录 → 应用市场安装 → iframe 握手 → `sdk.callService` 宿主代理 → 令牌交换(如 `knowledge→files` / `knowledge→omni-parser`)。
4. **同时覆盖两类后端 App**:`micro`(有前端、iframe 嵌入,知识库)与**新增的 `service` 类型**(无前端、仅提供 API,omni-parser)。
5. 复用既有契约与不变量:四道闸鉴权、scope 不提权、租户隔离、TDT 自省/交换——**门户行为与生产一致**(仅 dev 门 `DEV_MOCK_OAUTH` 打开)。

**非目标**

- 不做生产部署。生产走 `bootstrap:prod` + 按需部署(deploy-controller / `DEPLOYABLE_APP_SERVICES` / K8s),见 [DEPLOY.md](./DEPLOY.md)。本套件**仅 dev**。
- 不做轻量 mock 桩(已否决:保真度低、会与真实门户漂移、是需独立维护的新代码)。
- 不替外部 App 实现运行时契约(自省 + 四道闸 + `/health` + 信封解包仍由对方照 `apps/spms-server/` 实现)。
- 不要求外部团队拿到门户源码——一切以**镜像 + 声明式 manifest**交付。

---

## 2. 现状:为什么 `knowledge-devkit` 还不够通用

`knowledge-devkit` 能跑通,**只因为 `knowledge` 被提前烘焙进了门户**,三处都是**写死 knowledge 的**:

| 烘焙点 | 位置 | 对新 App(如 omni-parser)的后果 |
| --- | --- | --- |
| listing + 服务账号(已知密钥) | [`apps/api/src/db/seed.ts`](../apps/api/src/db/seed.ts) 内联插入 knowledge listing(L424-449)+ knowledge-server SA(L542-561,secret=`KNOWLEDGE_RESOURCE_KEY`) | 新 App **没有** listing/SA → 市场里看不到、自省无凭证 |
| `/svc/<key>` 路由白名单 | [`deploy/caddy/Caddyfile`](../deploy/caddy/Caddyfile) 内联 `knowledge "1"`(L80-84) | 新 key 不在白名单 → `/svc/<newkey>` 直接 404 |
| compose override 全写死 knowledge | [`deploy/knowledge-devkit/docker-compose.knowledge-dev.yml`](../deploy/knowledge-devkit/docker-compose.knowledge-dev.yml)(`knowledge-server` / `KNOWLEDGE_IMAGE` / `KNOWLEDGE_FRONTEND_DIST` / profile `app-knowledge` / chroma) | 换个 App 要重写整份 override |

→ **今天接一个新 App 必须重建门户镜像(改 seed + Caddyfile)。这正是本计划要消灭的。**

**好消息:泛化所需的底层能力 APP-INTEGRATION Phase 0–3 已基本就绪**,本计划主要是**把它们串成一个 App 无关的套件**,而非造新功能:

- ✅ **listing 写入已数据化**:[`market/service.ts`](../apps/api/src/modules/market/service.ts) 的 `createListing` 已接收**完整** App 声明(`ListingInput`:`aclManifest` / `scopeLabels` / `serviceBaseUrl` / `exchangeTargets` / `embedCsp` / `deployDescriptor` / `scopes`(含 App 命名空间)/ `navItems` / `embedUrl` / `dependencies`),并写时校验(scope 命名空间、manifest 形状、依赖无环)。平台管理员 HTTP 面:`POST /api/console/market/listings` + 发布 `POST .../status`([`market/console.ts`](../apps/api/src/modules/market/console.ts))。
- ✅ **服务发现已数据化**:`serviceBaseUrl` 缺省即 `/svc/<key>`;设了 `serviceBaseUrl`/`deployDescriptor` 即标记为「有后端」→ 安装时强制 `appKey == listingKey`(禁 `-2` 漂移,保证 `aud` 对齐,APP-INTEGRATION §4.0)。
- ✅ **令牌交换已数据化**:listing 的 `exchangeTargets` 在**安装时自动接线**(`wireExchangeTargets`:目标开 `allowExchange` + 把来源加白名单 + 建 active grant)。`knowledge→files` / `knowledge→omni-parser` 只是 manifest 一个字段 + 安装即生效。
- ✅ **`/svc` 路由可数据化放行**:Caddyfile 已有 `import /etc/caddy/svc-allow/*.map`(共享卷 `caddy-svc-allow`)——**往卷里丢一个 `<key>.map`(内容 `<key> "1"`)即放行**,无需改 Caddyfile。`/apps/*` 是通配静态(**无白名单**),把外部 dist 放进 `/srv/www/apps/<key>/` 即同源托管。per-App `connect-src` 走 `import /etc/caddy/apps-csp/*.map`。
- ✅ **自省接受 SA Basic**:外部后端用 `Authorization: Basic base64(clientId:secret)` 调 `POST /api/tokens/introspect` 即可([`token/index.ts`](../apps/api/src/modules/token/index.ts) `authResourceCaller`)。(dev 的 `x-resource-key` 捷径**只**比对 `FILES_RESOURCE_KEY`,对新 App 不通用——所以走 SA Basic。)

**唯一缺口**(本计划补的):没有一个**通用、吃外部声明式 manifest** 的注册入口——`ensureListing`/`ensureServiceAccount`([`provisioning.ts`](../apps/api/src/db/provisioning.ts))只认硬编码的 `LISTING_DEFS`/`SA_DEFS`,遇到未知 key 直接抛错。补一个通用 `register-app` 即可闭环。

> 顺带收益:`seed.ts` 与 `provisioning.ts` 对 knowledge 有**两份且已不同步**的定义(scopes/navItems/embedUrl 都不一致)。外部 App 一律走 manifest 单一来源,从根上避免这种重抄漂移。

---

## 3. 核心设计

### 3.1 单一事实源:`app.manifest.json`

外部 App 在**自己 repo** 写**一份**声明式清单,既是「交付契约」也是 `register-app` 的输入。字段直接映射到 `ListingInput` + 服务账号 + 套件接线:

```jsonc
{
  // —— 身份(listingKey == aud == 安装态 appKey,有后端故禁 -2 漂移) ——
  "listingKey": "omni-parser",
  "name": { "zh-CN": "多模态解析器", "en": "Omni Parser" },
  "version": "1.0.0",
  "type": "service",                   // "micro"(有前端,iframe 嵌入) | "service"(无前端,仅 API)

  // —— 权限声明(写时按 §3.5 校验:平台基础 scope / 本 namespace / 已声明 exchangeTargets 的目标 scope) ——
  "scopes": ["userinfo.read", "omni_parser.read", "omni_parser.parse"],
  "scopeLabels": {                     // 同意页 / 交换授权页文案
    "omni_parser.read":  { "zh-CN": "读取解析任务/结果", "en": "Read parse jobs" },
    "omni_parser.parse": { "zh-CN": "提交解析任务",       "en": "Submit parse jobs" }
  },
  "aclManifest": null,                  // 可选;service 通常省略,micro 见知识库附录 A

  // —— micro 才需要(service 省略这三项) ——
  "navItems": [{ "id": "nav-docs", "label": { "zh-CN": "文档" }, "route": "/docs" }],
  "embedUrl": "/apps/omni-parser/",    // 同源子路径(套件把 dist 挂到这里);service: null
  "embedCsp": { "connectSrc": [] },    // 个别 App 直连外部时的 connect-src 例外

  // —— 拓扑与跨应用调用 ——
  "dependencies": ["files"],           // 安装本 App 会按拓扑序先补装这些
  "exchangeTargets": ["files"],        // 本 App 作为「来源」要经令牌交换调用谁(安装即接线)

  // —— 服务发现(缺省 /svc/<key>,套件按命名约定路由到 <key>-server:8080) ——
  "serviceBaseUrl": "/svc/omni-parser",

  // —— dev 凭证(仅本地联调!register-app DB 直写已知明文,便于对方 repo 固定到 env) ——
  "serviceAccount": { "clientId": "omni-parser-server", "secret": "omni-parser-dev-resource-key" },
  "exchangeInitiatorSecret": null      // 若本 App 是交换「来源」(如 knowledge),填它的 App Secret
}
```

**职责切分(刻意)**:`app.manifest.json` = **门户契约**(被注册进门户 DB);**本机接线**(镜像 tag、dist 绝对路径、容器端口)放 `compose.env`(`APP_KEY` / `APP_IMAGE` / `APP_FRONTEND_DIST` / `APP_PORT=8080`)。前者跟着 App 走、可入对方 repo;后者跟着本机走。

### 3.2 通用注册步骤:`register-app`

新增 [`apps/api/scripts/register-app.ts`](../apps/api/scripts/register-app.ts),以**一次性容器**跑(沿用既有 `docker compose run --rm portal-api bun run db:seed` 的形态),读 manifest 完成**幂等**注册:

1. **listing(按 key upsert,不是裸 `createListing`)**:`createListing` 遇同 `listingKey` 直接抛 `SLUG_TAKEN`([`service.ts:378`](../apps/api/src/modules/market/service.ts)),所以 register-app 必须**先按 `listingKey` 查**:存在 → `updateListing(id, …)`(listingKey 不可变),不存在 → `createListing(…)`;再 `setListingStatus(id, "published")`([`service.ts:412`](../apps/api/src/modules/market/service.ts))使其可见。两条路都复用既有写时校验;建议抽一个 `upsertListingByKey`。`serviceBaseUrl` 缺省补 `/svc/<key>`(标记「有后端」)。
2. **服务账号(已知明文 + 幂等密钥策略)**:**DB 直写**一个 `token.introspect` SA,secret = manifest 里的**已知明文**(照 `seed.ts` knowledge SA 的写法)。`clientId` 唯一([`schema.ts:551`](../apps/api/src/db/schema.ts)),重跑须有**确定性策略**:无此 clientId → 建 SA + active secret;已存在 → 当前 active secret 明文 **== manifest 则不动**,**不同则轮换**(旧 secret 置 inactive、插新 active),**绝不静默留旧值**(否则幂等不可测、密钥口径与 manifest 漂移)。> 为什么不走 `POST /api/console/service-accounts`?该 API **只签发随机密钥、只回显一次**,无法让对方 repo 在 env 里固定——dev 联调要**可重复 `up`**,故已知明文,仅 dev。
3. **交换来源密钥**:若 `exchangeInitiatorSecret` 非空(本 App 是交换来源,如 knowledge),DB 直写其 App Secret(`wireExchangeTargets` 在安装时接线 grant+白名单,但**不**插这把来源密钥,需在此补)。
4. **`/svc` 放行**:往 `caddy-svc-allow` 卷写 `<key>.map`(内容 `<key> "1"`)。**注册在「起反代之前」跑**,反代启动时 `import …/*.map` 即读到——**无需热重载**;若要对**已在跑**的反代热加,文档给一条 `docker compose exec reverse-proxy caddy reload`(复用 [`deploy-controller.ts`](../apps/api/scripts/deploy-controller.ts) `openRoute` 同款机制)。
5. **类型分流**:`type:"service"` 则不写 embedUrl/navItems、不挂前端,并据类型从应用中心/市场浏览中过滤(§4.2);`type:"micro"` 则前端 dist 由套件挂进 `/srv/www/apps/<key>/`(通配静态托管,无需注册)。

> 守门:`register-app` 在 `NODE_ENV=production` 时**拒绝运行**(已知明文密钥仅限 dev),与 [`prod-guard.ts`](../apps/api/src/lib/prod-guard.ts) 对 `DEV_MOCK_OAUTH` 的态度一致。

### 3.3 通用 compose override:一份文件吃任意 key

新增 [`deploy/app-devkit/docker-compose.app-dev.yml`](../deploy/app-devkit/),用 `${APP_KEY}` 参数化、**一份复用所有 App**。关键技巧:服务名是静态的,但用**网络别名** `${APP_KEY}-server` 让 Caddy 的 `<key>-server:8080` DNS 解析命中:

```yaml
services:
  app-backend:                          # 外部后端(对方镜像,不进 monorepo)
    image: ${APP_IMAGE:?set APP_IMAGE}
    profiles: ["app-external"]
    env_file: [./compose.env]
    environment: { PORT: "8080", "${APP_KEY_PORT_ENV:-APP_SERVER_PORT}": "8080" }
    expose: ["8080"]
    networks:
      default:
        aliases: ["${APP_KEY:?set APP_KEY}-server"]   # ← Caddy 按 <key>-server:8080 命中

  reverse-proxy:                        # micro App:把外部 dist 同源挂到 /apps/<key>/
    volumes:
      - ${APP_FRONTEND_DIST:-/dev/null}:/srv/www/apps/${APP_KEY}:ro
```

- **后端内循环两选一**:(i) 对方**预构建镜像**接入(改代码=重建镜像);(ii) **挂源码 + `bun --watch`**(或对方的 watch 命令)进 `app-backend` 容器,服务名别名仍是 `<key>-server:8080`,门户那坨不用动。
- **`service` 类型**:不带 `reverse-proxy` 那段卷挂载(`APP_FRONTEND_DIST` 留空),只起 `app-backend`。
- App 自带的基础设施(omni-parser 的对象存储、knowledge 的 chroma 向量库)按需在 override 里加一个服务(同 knowledge-devkit 的 `chroma`)。

### 3.4 端到端流程(通用,knowledge-devkit 的泛化版)

```
external-repo/
└─ xgent-devkit/                       # 平台交付 + 对方填两个文件
   ├─ app.manifest.json                # ← 对方写(§3.1)
   ├─ compose.env                      # ← 对方填本机接线(APP_KEY/APP_IMAGE/APP_FRONTEND_DIST)
   ├─ docker-compose.app-dev.yml       # 平台给的通用 override(§3.3)
   └─ (门户 deploy/ 目录:base compose + caddy + postgres)

# 0) 载门户镜像 + 你的后端镜像(docker load,无需 docker hub)
# 1) compose.env:基础模板 + DEV_MOCK_OAUTH=true + APP_KEY/APP_IMAGE/APP_FRONTEND_DIST/SA 凭证
# 2) 起基础设施(pg/redis/minio)
# 3) 门户库迁移 + 门户基线 seed(只建门户世界:租户/用户 rockie/内置 App,不含你的 App)
        docker compose ... run --rm portal-api bun run db:migrate
        docker compose ... run --rm portal-api bun run db:seed
# 4) ★ 通用注册:读 manifest 注册 listing+SA+/svc 放行(写 caddy-svc-allow 卷)
        docker compose ... run --rm portal-api bun run register-app /devkit/app.manifest.json
# 5) 起 门户三件套 + 你的 app-backend(别名 <key>-server:8080)(+ micro 则挂前端)
# 6) 你自己的库迁移 / 每租户 bootstrap(命令以你镜像为准)
# 7) 冒烟(§7)
```

### 3.5 Scope 声明规则(需收紧 —— Phase 0 前置,安全口径最高优先)

> ⚠️ **现状与意图相反,必须先解决,否则验收会「说会拒、实际放行」**。[`market/service.ts:284`](../apps/api/src/modules/market/service.ts) 把**整个 `SCOPES` 常量**当「平台基础白名单」(`BASE_SCOPES = new Set(SCOPES)`),而 `SCOPES` 里已含 `files.*`/`pms.*`/`sms.*`/`qbank.*`/`lms.*`/`llm_gateway.*`/`knowledge.*` 等**各内置 App 的 scope**([`packages/shared/src/scopes.ts`](../packages/shared/src/scopes.ts))。→ **今天任意 App 都能声明 `files.read` 而不被拒**;旁边那句注释「a listing can never declare another App's scope」与实现相悖。

**选定规则(收紧,让 namespace 隔离真正成立)** —— 一个 listing 可声明的 scope =
1. **平台基础 scope `PLATFORM_BASE_SCOPES`**(真正 App 无关的那批:`userinfo`/`directory`/`directory.email`/`notification(.send/.others)`/`inbox`/`settings`/`widget`/`audit`/`scheduler`/`content`);**∪**
2. **本 namespace**(`<listingKey>` 及其下划线变体,如 `omni-parser`→`omni_parser`);**∪**
3. **已声明的 `exchangeTargets` 的 namespace**(令牌交换要求来源 App 携带目标 scope,故只有声明了「我要交换调用 files」的 App 才能声明 `files.*`)。

**前置改动(本质是 APP-INTEGRATION §4.2 的 latent 修正)**:把 `PLATFORM_BASE_SCOPES` 从 `SCOPES` **拆出**([`scopes.ts`](../packages/shared/src/scopes.ts)),`service.ts` 的 `BASE_SCOPES` 改用它;`validateScopes` 增按 `exchangeTargets` namespace 放行(注意它与 scopes 在同一 `validateDeveloperFields` 内可同时拿到)。

**效果(验收口径与代码一致)**:omni-parser 声明 `files.read` 但 `exchangeTargets` 不含 files → **被拒** ✓;knowledge 声明 `files.read` 且 `exchangeTargets:["files"]` → **放行** ✓。内置 App 经 provisioning 直写(绕过校验)不受影响。

> 回退(若本期不收紧):必须**承认现状**(所有内置 scope 皆可声明)并**删掉** Phase 0「files.read 被拒」那条验收——不能让计划与代码口径相反。**推荐做收紧**(reviewer 标记为最高风险项)。

---

## 4. 两类后端 App 的差异处理(新增 `service` 类型)

今天 `AppType` 枚举是 `["micro", "link", "native"]`([`packages/shared/src/constants.ts:56`](../packages/shared/src/constants.ts))。本计划**新增第四种 `service`**:无前端、仅提供 API 的独立后端资源服务器。它是 headless 后端的**一等公民表达**——既描述了形态,又顺手解决了可见性(应用中心/市场按 `type` 过滤,无需另加 `hidden` 列)。

### 4.1 `micro`:有前端(知识库 knowledge)

- manifest:`type:"micro"`、有 `embedUrl: /apps/knowledge/`、`navItems`、`aclManifest`、`dependencies:["files"]`、`exchangeTargets:["files"]`、`scopes` 含 `files.read/write`、`exchangeInitiatorSecret`=knowledge App Secret。
- 套件:挂前端 dist 到 `/apps/knowledge/`;起 chroma 服务。
- 冒烟:走完整 iframe 链路(§7-A)。

### 4.2 `service`:无前端、仅 API(多模态解析器 omni-parser)

- manifest:`type:"service"`、`embedUrl:null`、无 navItems、`scopes:["omni_parser.read","omni_parser.parse"]`、`exchangeInitiatorSecret:null`(它是**被调用方**,不是来源)。它的自带库 + 对象存储在 override 里加服务。
- **可见性(由类型解决,但当前三处都没做 → 列为 Phase 3 硬验收)**:`service` 应当不进应用中心、不在用户侧市场浏览露卡。**现状缺口**:① 用户侧市场 [`listPublishedForTenant`](../apps/api/src/modules/market/service.ts)(`service.ts:434`)只按 `published/cat/q` 过滤,**无 type 过滤**;② 应用中心 [`visibleAppRows`](../apps/api/src/modules/apps/service.ts)(`service.ts:73`)只隐藏 `native`;③ 控制台市场卡片 [`Marketplace.tsx:92`](../apps/web/src/pages/admin/Marketplace.tsx) 的 `onOpen` 会 `navigate('/app/<key>')`。**漏掉任一处,omni-parser 都会露卡甚至被点开**。平台清单管理仍应可见可治理。它仍是一条可被**依赖补装**的 listing,令牌交换照常(交换要求目标已安装)。
- 交换接线:让 **knowledge** 的 manifest 增 `exchangeTargets:["files","omni-parser"]` + scope `omni_parser.*` + `dependencies` 含 `omni-parser` → 安装 knowledge 时 `wireExchangeTargets` 自动把 omni-parser 开 `allowExchange` + 白名单 `[knowledge]` + 建 grant。首次调用弹**交换授权页**(用户同意一次,见 `exchangeConsents`)。
- 冒烟:无 UI,走 curl(§7-B)。

---

## 5. 分阶段计划(每阶段独立可验证)

> 原则:先打通**通用注册 + 通用套件**让「知识库经通用套件零改码跑通」成立,再补 `service` 类型与文档。
> 用 knowledge 当**回归基准**(它今天能跑),用 omni-parser 当**新 App 验证**(它今天跑不了)。

**Phase 0 — manifest schema + 校验器 + scope 规则收紧(§3.5,最高优先)**
- 定义 `app.manifest.json` 的 TS 类型(置于 `packages/shared` 或 `apps/api`),校验复用 `market/service.ts` 既有 `validateAclManifest`/`validateDependencies` 等。
- **scope 规则收紧(§3.5,前置)**:从 `SCOPES` 拆出 `PLATFORM_BASE_SCOPES`;`service.ts` `BASE_SCOPES` 改用它;`validateScopes` 改为「平台基础 ∪ 本 namespace ∪ `exchangeTargets` namespace」。
- **verify**:knowledge(`files.read` + `exchangeTargets:["files"]`)**通过**;omni-parser(`files.read` 但 `exchangeTargets` 无 files)**被拒**;内置 App 经 provisioning 直写仍正常(绕过校验);`scopes` 契约测试全绿。

**Phase 1 — 通用 `register-app` 脚本(幂等语义写严)**
- `apps/api/scripts/register-app.ts`:读 manifest → **按 key upsert listing**(find→create/update→`setListingStatus("published")`,§3.2-1)→ **DB 直写 SA + 幂等密钥策略**(§3.2-2)→ 交换来源密钥(若有)→ 写 `caddy-svc-allow/<key>.map`;整体**幂等可重跑**;`NODE_ENV=production` 拒跑。
- compose 小改(**已定 OK**):给 `portal-api` 服务**补挂 `caddy-svc-allow` 卷**(现仅挂在 reverse-proxy/deploy-controller,[`docker-compose.yml:67`](../deploy/docker-compose.yml);register-app 要往里写)。
- **verify(注册层 · 幂等)**:对**纯门户基线**(`db:seed` 后)**连跑两次** `register-app knowledge.manifest.json` —— 第二次**无报错、无副作用漂移**(同 secret 不动、listing 字段一致);产出的 listing 字段、SA 自省、`/svc/knowledge` 可达 与原烘焙版**等价**;`register-app omni-parser.manifest.json` 后 `/svc/omni-parser` 可达、安装后交换 grant 建立。
- **verify(干净机器最小冒烟 · 提前做)**:在**只装 Docker、无门户源码**的环境,仅用「门户 tar + 一份 manifest + 一个外部 `service` 镜像」跑通 `register-app` + `curl /svc/<key>/health` 200。**不验 UI**,只验注册 + 路由 + 卷挂载链路——尽早暴露路径 / compose 相对目录 / 卷挂载问题(reviewer 建议)。

**Phase 2 — 通用套件目录 `deploy/app-devkit/`**
- `docker-compose.app-dev.yml`(§3.3 参数化 override)+ `compose.env.app.example` + `manifests/`(放样例)+ `README.md`(通用起栈步骤,§3.4)。
- knowledge-devkit **保留为「样板实例」**,README 顶部指到通用套件(或直接以通用套件 + knowledge manifest 取代)。
- **verify**:**只用通用套件**(无任何 knowledge 专属代码/override),按 README 把知识库端到端起来,§7-A 冒烟全过。

**Phase 3 — 新增 `service` app 类型 + omni-parser 接入**
- **加类型(无需 DB 迁移)**:`APP_TYPES` 增 `"service"`([`constants.ts:56`](../packages/shared/src/constants.ts))。`type` 在 `marketplace_listings`/`apps` 都是 **text 列**([`schema.ts:163/231`](../apps/api/src/db/schema.ts)),**不需 enum 迁移**——只改 TS/Zod/UI/过滤。`ListingInput`/校验放行;`createListing` 对 `service` **不要求 embedUrl/navItems**,并确认按 `micro` 分支的逻辑([`service.ts:609`](../apps/api/src/modules/market/service.ts) `if (row.type === "micro") createSecret(...)`)对 `service` 走对路径。
- **可见性(硬验收 —— 三处都要改,漏一处即露卡)**:① 用户侧市场 `listPublishedForTenant`([`service.ts:434`](../apps/api/src/modules/market/service.ts))增 `type != 'service'` 过滤;② 应用中心 `visibleAppRows`([`service.ts:73`](../apps/api/src/modules/apps/service.ts))扩为对所有人隐藏 `service`;③ 控制台市场 [`Marketplace.tsx:92`](../apps/web/src/pages/admin/Marketplace.tsx) 对 `service` 隐藏 / 禁用 `onOpen`。`embeddable` 判定([`apps/index.ts:104`](../apps/api/src/modules/apps/index.ts))、打开入口 [`Apps.tsx`](../apps/web/src/pages/Apps.tsx)/[`MicroAppHost.tsx`](../apps/web/src/pages/MicroAppHost.tsx) 对 `service` 视为不可打开;平台清单管理 [`console/Market.tsx`](../apps/web/src/pages/console/Market.tsx) 仍展示。
- **UI/文案**:控制台 [`MarketListingForm.tsx`](../apps/web/src/pages/console/MarketListingForm.tsx)(`form.type === "micro"` 分支)增 `service` 选项、隐藏 embedUrl/navItems 字段;各处类型 Badge + locales 增 `typeService`。
- **omni-parser 接入**:按 §4.2 用通用套件以 `type:"service"` 注册 + 作为 knowledge 依赖被动补装。
- **verify(硬)**:typecheck 全绿;现有 micro/link/native 不回归;omni-parser 注册后**用户侧市场不露卡 + 应用中心不露卡 + 控制台市场不可点开**(三处逐一核),但平台清单管理可见、可作依赖安装;§7-B 的 curl 四道闸全过;`knowledge→omni-parser` 交换打通(首次弹同意页)。

**Phase 4 — 文档 + 交付打包**
- 把 [`docs/知识库后台接入与本地联调指南.md`](../docs/知识库后台接入与本地联调指南.md) 抽象出一份**App 无关**的《外部 App 本地联调指南(one-box)》,knowledge / omni-parser 作为两个具体 example(各附 manifest)。
- 交付脚本 `scripts/pack-onebox.sh`:`docker save` 门户 + proxy 镜像 → tar,连同 `deploy/`(base compose + caddy + postgres + app-devkit)打成一个交付包。
- **verify**:在一台**只装 Docker、无门户源码**的干净机器上,仅用交付包 + 一份 manifest,跑通 knowledge **和** omni-parser。

**Phase 5(可选 · 人体工学)— 启动期 manifest 自动加载**
- 让 portal-api 启动时(仅 dev)扫描一个挂载目录 `apps-registry/*.manifest.json` 自动 upsert,免去单独 `register-app` 一步——「丢个文件就注册」。
- 一键冒烟脚本 `verify:onebox`(类似 `verify:all`)封装 §7。

---

## 6. 安全 / 边界(不可降级)

- **仅 dev**:`DEV_MOCK_OAUTH=true` + `NODE_ENV` 不为 production。[`prod-guard.ts`](../apps/api/src/lib/prod-guard.ts) 在 production 下见 `DEV_MOCK_OAUTH=true` 即 `exit(1)`;`register-app` 自身在 production 拒跑。**本套件绝不用于生产**。
- **已知明文 SA 密钥仅 dev**:为可重复 `up` 而固定;生产 SA 走 `POST /api/console/service-accounts` 随机签发(回显一次)或 `provisioning` 从强随机 env 取。
- **门户安全不变量原样保留**:四道闸(`aud/listingKey` + scope + requireAdmin + requirePerm)、scope 命名空间不可提权(写 listing 时 `validateScopes`,**前提是 §3.5 收紧已落地**——否则现状下 namespace 隔离形同虚设)、令牌交换需 `allowExchange`+白名单+active grant+用户同意——**与生产同一套代码**,套件只换 dev 门。
- **治理边界**:`register-app` 等价于平台管理员建 listing/SA;它是**平台团队交付的 dev 工具**,不暴露给租户管理员(后者仍只能在已上架清单里安装,见 APP-INTEGRATION §7)。
- **外部镜像**:`docker save` tar 或私有 registry(不上 docker hub);生产再谈 digest 固定 + 扫描准入(APP-INTEGRATION §7)。omni-parser 注意 **amd64+arm64 multi-arch**(本地 arm64 / 生产 amd64,见其对接要求 §7)。

---

## 7. 冒烟(验收主路径)

打开 `http://localhost/` →

**A. `micro` · 有前端(knowledge)**
1. dev 登录 → 选 `rockie`(晨光教育 admin + 平台管理员)。
2. 应用市场 → 安装「知识库」→ 连带补装 `files`,状态 `ready`;`knowledge→files` 交换接线。
3. 应用中心打开知识库 → iframe 加载 `/apps/knowledge/`,`sdk.ready()` 握手成功。
4. 前端调后端:`sdk.callService("knowledge","/api/...")` 通 → 证明宿主代理 + 四道闸 + 自省全链路 OK。
5. 读文件 → 首次弹交换授权页,同意后 `knowledge→files` 交换生效。
6. 顶栏切租户 → 看不到上一租户数据(租户隔离)。

**B. `service` · 无前端(omni-parser,无 UI 走 curl)**
1. 安装 knowledge(依赖里带 omni-parser → 被动补装;或单独装)。
2. `/health` 判活:`curl http://localhost/svc/omni-parser/health` → `{"service":"omni-parser","db":"ok",...}`(经反代同源,容器内 8080)。
3. 缺/错 token:`curl -X POST .../svc/omni-parser/v1/parse` → 401/403(四道闸)。
4. 拿一个 `aud=omni-parser` 的 TDT(经 knowledge 交换,或 dev 自助走 `/api/tokens/authorize`→`/oauth/token`)→ 正确 → 200 信封。
5. **信封解包自测**:用**有效** TDT 必须被**接受**;若返 `INVALID_TOKEN` 而门户自省该 TDT `active:true`,即没解包 `.data`(knowledge 1.0.0 同款坑)。

---

## 8. 排查速查

| 症状 | 多半原因 |
| --- | --- |
| `/svc/<key>` 404 | `caddy-svc-allow/<key>.map` 没写成 / 反代在写 map **之前**就起了(先 `register-app` 再起反代,或 `caddy reload`) |
| `/svc/<key>` 502 | 你的 `app-backend` 没起 / 刚 crash(`docker compose logs app-backend`);或没听 8080;或网络别名 `<key>-server` 没配 |
| iframe 404 / 空白 | `APP_FRONTEND_DIST` 不对(须绝对路径且含 `index.html`),或 `service` 类型却配了 embedUrl |
| 自省 401 | SA Basic 不对:manifest 的 `serviceAccount.secret` 与后端实读的 `*_SA_CLIENT_SECRET` 不相等 |
| 有效 TDT 被判 `INVALID_TOKEN` | 没解包自省信封 `claims = body.data ?? body` |
| `EXCHANGE_NOT_ALLOWED` | 来源 listing 没声明 `exchangeTargets` / 目标没安装 / 没建 grant / 用户没同意 |
| `db:seed` 后浏览器要重登 | seed 重生成 UUID,会话失效,重 dev 登录 |
| `register-app` 重跑报 `SLUG_TAKEN` | 没走按 `listingKey` upsert,裸调了 `createListing`(§3.2-1) |
| 声明的 scope 没被拒(本该拒) | scope 收紧未落地:`BASE_SCOPES` 还是整套 `SCOPES`(§3.5,Phase 0 前置) |
| 安装报 `ALREADY_INSTALLED` 或 `-2` 漂移 | 有后端 App 须 `appKey==listingKey`(设 `serviceBaseUrl`/`deployDescriptor` 标记 backed) |

---

## 9. 决议(已拍板)

1. **形态 = 最小真实门户**(复用三件套 + pg/redis/minio),非 mock 桩。
2. **分发 = 外部第三方 · 黑盒镜像**(对方只需 Docker;门户以 tar / 私有 registry 交付)。
3. **接入 = 一份 `app.manifest.json` + 通用 `register-app`**;新 App 零改门户码、零重建门户镜像。
4. **注册机制 = 通用脚本 DB 直写**(已知 dev 密钥,可重复 `up`);console API 随机密钥作「无 DB 访问」回退;启动期自动加载作可选人体工学(Phase 5)。
5. **通用 override 用 `${APP_KEY}` + 网络别名 `<key>-server`** 一份吃所有 App;后端可预构建镜像或挂源码 watch。
6. **新增 `service` app 类型**(无前端、仅 API):`AppType` 加第四种,顶替原拟的 `hidden` 列——可见性由类型过滤(应用中心/市场浏览隐藏 `service`,平台控制台可见),omni-parser 即 `type:"service"`(Phase 3)。
7. **`portal-api` 补挂 `caddy-svc-allow` 卷**(已定 OK),令 `register-app` 能写 `/svc` 放行 map(Phase 1)。
8. **Scope 声明规则收紧(§3.5,最高优先)**:`PLATFORM_BASE_SCOPES` 从 `SCOPES` 拆出;可声明 = 平台基础 ∪ 本 namespace ∪ `exchangeTargets` namespace。使「omni-parser 声明 `files.read` 被拒」真正成立(Phase 0 前置);否则承认现状并删该验收,二选一,不许口径相反。
9. **`register-app` 幂等语义写严**:listing 按 `listingKey` upsert(非裸 `createListing`,后者 `SLUG_TAKEN`);SA 按 `clientId` upsert + 「同 secret 不动 / 变则轮换 / 不留旧值」密钥策略(Phase 1)。
10. **`service` 可见性 = 硬验收**:用户侧市场 + 应用中心 + 控制台市场打开入口**三处**都过滤/禁用 `service`,漏一处即不通过(Phase 3)。

---

*本计划随实现演进。落地以 `apps/api/scripts/register-app.ts`、`deploy/app-devkit/`、
`apps/api/src/modules/market/service.ts`、`deploy/caddy/Caddyfile` 的实现为准。*
