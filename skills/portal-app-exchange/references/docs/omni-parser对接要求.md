# `omni-parser` 对接要求(平台 → omni-parser 团队)

> **读者**:omni-parser 后端团队 + XGENT 平台团队。
> **omni-parser 是知识库(knowledge)的上游**:解析多模态文档、音视频等,产出结构化内容,供 knowledge 入库(embed → Chroma/PG)。数据流:`raw 多模态文件 → omni-parser 解析 → 结构化产物 → knowledge 入库`。
> **集成方式与 knowledge 同构(集成方式三:独立后端资源服务器)**,但本次已定三点不同:
> 1. **headless** —— 无前端、无 `embedUrl`/`navItems`,用户不直接访问;只被 knowledge(及未来其他应用)在**入库时**调用。
> 2. **被调用方** —— 经 OAuth 令牌交换 `knowledge → omni-parser` 调用(与 `knowledge → files` 同构)。
> 3. **同步 + 异步双模 + 有状态** —— 小文档同步直返;音视频/大文档走异步任务(submit → poll/webhook);自带 Postgres + 对象存储,存任务与解析产物,按租户隔离。
>
> **前置必读**:[`SSO与App开发指引.md`](./SSO与App开发指引.md) §7(独立后端)/§11(令牌交换)/§13.8(审计);[`knowledge-app-contract.md`](./knowledge-app-contract.md)(同构参考);`apps/spms-server/`(参考实现,四道闸 + 令牌交换调 files 的同构案例)。

---

> **接入机制已更新(门户一盒)** —— 本文写于手工注册年代。omni-parser 现在是平台新增的 **`service` 应用类型**
> (headless、无前端的一等公民),经 **`app.manifest.json` + 通用 `register-app` + [`deploy/app-devkit/`](../deploy/app-devkit/)**
> 数据化接入:平台团队**不再逐项手工注册**(scope / 服务账号 / headless app / `/svc` 白名单 / compose profile 全由 manifest +
> register-app 落库)。样例清单已就绪:[`deploy/app-devkit/manifests/omni-parser.manifest.json`](../deploy/app-devkit/manifests/omni-parser.manifest.json);
> 总流程见 [`外部App本地联调指南-one-box.md`](./外部App本地联调指南-one-box.md) 与 [`deploy/app-devkit/README.md`](../deploy/app-devkit/README.md)。
>
> 下文「平台侧」口径(§0 右列 / §1「隐藏 app」/ §6 / §8 的 `:4800`)按此**已被取代**,保留作背景;**运行时契约(§2–§5、§7、§9)是 omni-parser 团队照实现的,不变。**

---

## 0. 一页速览:谁做什么

| 工作 | 由谁做 |
| --- | --- |
| 资源服务器验 TDT(自省 + 四道闸 + **信封解包**) | **omni-parser 团队**(抄 `apps/spms-server/src/lib/gate.ts`) |
| 租户隔离、限流、`/health`、异步任务/队列 | **omni-parser 团队** |
| 同步 `/v1/parse` + 异步 `/v1/jobs` API + **generated API reference** | **omni-parser 团队** |
| 后端 docker 镜像(8080、读 env、amd64+arm64)+ 契约清单交付 | **omni-parser 团队** |
| 注册 scope `omni_parser.read`/`omni_parser.parse` | **平台团队**(随清单 `scopes`+`scopeLabels` 入库) |
| 注册 headless app(`aud=omni-parser`、`allowExchange`、白名单 `[knowledge]`,无 embedUrl) | **平台团队** |
| token-exchange grant `knowledge → omni-parser`;knowledge listing 加 `omni_parser.*` scope + 依赖 | **平台团队** |
| 服务账号(`token.introspect`)→ 发 clientId/secret | **平台团队** |
| 服务发现 `/svc/omni-parser`(Caddy 通用 `/svc/*` + 白名单 map + dev vite 端口) | **平台团队** |
| 基础设施(Postgres / 对象存储 / 队列)provision:本地 compose、生产 K8s | **平台团队**(omni-parser 只在 env 点名连接串) |
| compose profile `app-omni-parser` / K8s Deployment(指向**对方外部镜像**) | **平台团队** |

---

## 1. 集成模型

- **listingKey / aud = `omni-parser`**(连字符);**scope 命名空间 `omni_parser`**(下划线)—— 与 `llm-gateway`/`llm_gateway` 的双命名空间约定一致(scope 不含连字符)。
- 身份模型:omni-parser **不持门户 HS256 密钥、不自验 JWT 签名、不自实现登录**。只把收到的 `Authorization: Bearer <TDT>` 发给门户自省端点换声明,据此鉴权。
- **headless = `service` 类型**:不贡献 UI;以 `type:"service"` 注册(拿 `aud` + 配 `allowExchange`),由**类型过滤对用户隐藏**——应用市场不露卡、应用中心不露卡、不可启动;平台控制台「清单管理」仍可见可治理。仍可作依赖被补装,令牌交换照常。
- **被 knowledge 经令牌交换调用**;其他应用(files/qbank…)要用解析能力,加一条 grant 即可复用(故做成资源服务器而非内部私有服务)。

---

## 2. 运行时契约(资源服务器,照 `apps/spms-server/`)

### 2.1 验 TDT:自省 + 四道闸 + ⚠️ 信封解包

1. 取 `Authorization: Bearer <TDT>`,缺失 → **401 `UNAUTHENTICATED`**。
2. `POST {PORTAL_INTROSPECT_URL}`,带服务账号 Basic(`Authorization: Basic base64(saClientId:saSecret)`),body `{ "token": "<TDT>" }`。
   - `active:false` 是**成功**返回(令牌无效/过期),不是传输错误 → 映射 401。
   - 结果缓存到 `min(exp, now+60s)`,按 `sha256(token)`。
3. **⚠️ 自省响应是门户统一信封 `{ "ok": true, "data": { "active": true, ... } }`,声明在 `.data` 里。**
   - **必须解包:`claims = body.data ?? body`**(照 `apps/spms-server/src/lib/gate.ts:72-78`)。
   - 这是 **knowledge 1.0.0 踩过的坑**:裸读顶层 `active`(信封里是 `undefined`)→ 误判每个有效令牌为 `INVALID_TOKEN`。**别重蹈。** 同样适用于你消费的**所有**门户响应(`/api/v1/acl/check`、审计、令牌交换)——统一在 HTTP client 层解包,不要只补自省一处。
4. **四道闸**(在 handler 前集中做):
   - **aud**:`claims.aud === "omni-parser"`,否则 **401 `INVALID_TOKEN`**。
   - **scope**:所需 scope ∈ `claims.scopes`,否则 **403 `INSUFFICIENT_SCOPE`**。
   - headless 服务一般**无结构性管理/无 PID 细粒度**,scope 即主授权门。如需更细,再按 `claims.permissions` 命中 PID。
5. **service-kind TDT**:knowledge 经交换通常带**用户态** TDT(交换自用户的 knowledge TDT,含 `user_id`);若也支持服务态(`kind:"service"`)调用,按 **scope-only** 授权,不走 PID。

### 2.2 租户隔离
每条任务/产物查询按 `claims.tenant_id` 收口。**永不信任**请求体里的 `tenantId`,只认 TDT 里的。

### 2.3 scope
- `omni_parser.parse` —— 提交解析任务 / 同步解析(写)。
- `omni_parser.read` —— 查任务状态 / 取结果(读)。

### 2.4 限流 / 健康检查
- 限流:自己一层,per `(aud, tenant)` 固定窗口(spms 默认 600/min)→ **429 `RATE_LIMITED`**。
- 健康:`GET /health` → 统一信封 **`{"service":"omni-parser","db":"ok|down","redis":"disabled|ok","time":<unix>}`**(平台判活 grep `"db":"ok"` —— ⚠️ 别用 `"db":true`,knowledge devkit 探针踩过)。`GET /v1/healthz` → `{"status":"ok"}` 仅存活。

### 2.5 自省响应形状(你要消费的字段,**包在 `{ok,data}` 里**)
`{ active, kind, aud, listingKey, tenant_id, user_id, scopes[], role, bypass, groups[], permissions:[{pid,scope}], exp }`(`aclStamp` 接受并忽略)。

---

## 3. 令牌交换:knowledge → omni-parser([指引 §11](./SSO与App开发指引.md))

knowledge **不**拿用户的 `aud=knowledge` TDT 直调你(aud 不符会被你的闸拒)。走 OAuth Token Exchange 换一个 `aud=omni-parser` 的 TDT:

```
POST {PORTAL}/oauth/token
  grant_type    = urn:ietf:params:oauth:grant-type:token-exchange
  subject_token = <knowledge 当前 TDT>
  audience      = omni-parser
  client_id     = knowledge            # 必须 = subject_token.aud
  client_secret = <knowledge App Secret>
→ { access_token: <aud=omni-parser 的 TDT>, scope, expires_in }
```

- 交换后的 scope = `subject_token.scopes ∩ omni-parser 声明的 scopes`。
- **平台侧配**:建立 `knowledge → omni-parser` 授权关系;omni-parser 开 `allowExchange` 并把 `knowledge` 加入交换白名单;knowledge listing 增 `omni_parser.read`/`omni_parser.parse` scope + 依赖 `omni-parser`;用户事先在交换授权页同意。
- 其他消费方(files/qbank)同法加 grant 即可复用。

---

## 4. API 契约(同步 + 异步)

> 本节定**最小要求与形状**;**完整端点 / 请求响应 schema / 支持的模态与格式 / 大小时长上限,由 omni-parser 团队随镜像交付一份 generated API reference**(knowledge 没给这个,导致前端没法照着写——别重复)。所有响应用统一信封 `{ ok, data | error }`。

### 4.1 输入:字节怎么进来(择一或都支持)
- **(a) 直传字节** —— multipart / base64 body。最稳,无外部依赖。
- **(b) 传引用** —— URL(presigned)或门户 files `file_id`,omni-parser 拉取。
  - 注:`knowledge→files` 交换现已接通(knowledge 清单 `exchangeTargets:["files"]`),(b) 经 files `file_id` 拉取可用;但 **(a) 直传字节 / presigned URL 仍是最稳首选**(无外部依赖)。

### 4.2 同步:`POST /v1/parse`(小文档 / 快解析)
- scope `omni_parser.parse`。一次请求直返结构化结果。
- 设**大小 / 时长上限**;超限返业务错并提示改用异步(§4.3),不要硬撑长连接。

### 4.3 异步:任务式(音视频 / 大文档,分钟级)
- `POST /v1/jobs`(submit)scope `omni_parser.parse` → `{ job_id, status:"pending" }`。
- `GET /v1/jobs/{id}`(poll)scope `omni_parser.read` → `{ status: pending|running|succeeded|failed, result? | error? }`。
- **可选 webhook**:submit 时带 `callback_url`;回调**必须可验真**(HMAC 签名/共享密钥),消费方据此校验。
- 任务**生命周期**明确(pending/running/succeeded/failed + 超时/重试),**幂等**(同一输入可去重,避免重复解析)。

### 4.4 输出:结构化产物(最小约定)
- 文本**分块** + 每块元数据(来源页码 / AV 时间轴 / 模态)。
- 整体元信息:`mime`、页数 / 时长、语言、检测到的模态。
- AV 必须带**时间轴**(供 knowledge 做带时间戳的检索)。

### 4.5 解析用到的模型(OCR / ASR / Vision / LLM)
- **若调用 LLM,优先走门户 `llm-gateway`**(OpenAI 兼容 `/v1/*`,统一 token 用量统计 / 计费)——与 knowledge 的「LLM 走 gateway」决策一致,别各自直连 OpenAI。
- OCR/ASR 引擎自带或外部,在交付清单里说明(算力 / GPU 需求一并写明)。

---

## 5. Env 契约(镜像**不含** `.env`,运行时注入;容器内统一听 **8080**)

| 变量 | 必填 | 默认 / 示例 | 说明 |
| --- | --- | --- | --- |
| `OMNI_PARSER_PG_DSN` | ✅ | `postgres://…/xgent_omni_parser` | **自己的库**,存任务 + 产物元数据 |
| 对象存储(`*_S3_ENDPOINT`/`_BUCKET`/`_ACCESS_KEY`/`_SECRET`) | ✅ | MinIO/S3 | 存上传字节 + 解析产物 |
| `PORTAL_INTROSPECT_URL` | ✅ | `http://localhost:3000/api/tokens/introspect` | 门户自省端点 |
| `OMNI_PARSER_SA_CLIENT_ID` | ✅\* | `omni-parser-server` | 服务账号 Basic(平台发) |
| `OMNI_PARSER_SA_CLIENT_SECRET` | ✅\* | —(明文只发一次) | 同上 |
| `OMNI_PARSER_AUDIENCE` | | `omni-parser` | 接受的 TDT `aud` |
| `OMNI_PARSER_SERVER_PORT` | | `8080`(容器/生产统一) | 反代 `/svc/omni-parser/* → omni-parser-server:8080`;本地裸跑可另取(如 4800) |
| `API_BASE_URL` | | `http://localhost:3000` | 门户 Open API(审计 / 回拉 files) |
| `REDIS_CONN_STRING` | | 空 | 可选;限流 / 自省缓存 / 任务队列 |
| 异步 worker / 队列配置 | | | 并发、超时、重试 |
| 模型 / 算力配置 | | | GPU、ASR/OCR endpoint;走 gateway 的 `base_url` + token |

\* 三个门户自省变量(introspect URL + SA id + secret)是 **all-or-nothing**:全缺 → 门户鉴权停用(被闸路由 503);缺一不全 → **拒绝启动并打印缺哪个**(照 knowledge 契约 §5)。

> **⚠️ env 命名**:要么用平台契约前缀 `OMNI_PARSER_*`,要么把你们镜像**实读的变量名完整列清**。knowledge 用了 `XGENT_PG_DSN`(与契约 `KNOWLEDGE_DATABASE_URL` 不一致)差点踩坑——以你们实读的为准,别让平台猜。

---

## 6. 平台侧 checklist(外部阻塞项)

> **已数据化(门户一盒)** —— 下列手工步骤现由 **`omni-parser.manifest.json` + `register-app` + `deploy/app-devkit/`** 自动完成,逐项对应:
> 1/2/3 → manifest 的 `scopes`+`scopeLabels` / `type:"service"`(headless,无 embedUrl/navItems)/ `serviceAccount{clientId,secret}` 字段,`register-app` 一把落库(SA 用已知 dev 明文,可重复 `up`);
> 4 → 在 **knowledge 的 manifest** 加 `exchangeTargets:["files","omni-parser"]` + `dependencies` 含 `omni-parser` + scope `omni_parser.*`,**安装 knowledge 时自动接线**(`wireExchangeTargets`:omni-parser 开 `allowExchange` + 白名单 `[knowledge]` + 建 grant);
> 5 → `register-app` 写 `/svc/omni-parser` 白名单 map(无需手改 Caddyfile);`apps/web/vite.config.ts` 的 `:4800` 仅 **dev:all 非 docker** 跑法,一盒里走 `/svc/omni-parser`;
> 7 → 通用 `deploy/app-devkit/docker-compose.app-dev.yml`(`APP_KEY=omni-parser` + 网络别名 `omni-parser-server:8080` + profile `app-external`),**一份 override 吃任意 key,不再有 per-app `app-omni-parser` profile**。
> 6(基础设施)仍由平台按需在 app 专属 override 里加服务(同 chroma)。下面保留逐项口径作背景。

1. 注册 scope `omni_parser.read` / `omni_parser.parse`(随清单 `scopes`+`scopeLabels` 入库)。
2. 注册 **headless app**:`aud=omni-parser`、`allowExchange=true`、`exchangeWhitelist=["knowledge"]`;**无 `embedUrl`/`navItems`**(不可被用户启动)。
3. 建**服务账号**(capability `token.introspect`)→ 把 `clientId`/`secret` 发 omni-parser 团队。
4. **token-exchange grant** `knowledge → omni-parser`;knowledge listing 增 `omni_parser.*` scope + 依赖 `omni-parser`。
5. **服务发现**:Caddy 通用 `/svc/*` 白名单 map 加 `omni-parser "1"`;`apps/web/vite.config.ts` dev proxy 加 `"omni-parser": 4800`。
6. **基础设施 provision**:Postgres 库 + 对象存储 bucket(+ 队列)——本地 compose、生产 K8s;omni-parser 只在 env 点名连接串(同 chroma 的处理)。
7. **compose profile `app-omni-parser`** / K8s Deployment,指向对方**外部镜像**(不进 monorepo 镜像)。

---

## 7. Ops

- **DB 迁移**:带外执行的命令(镜像不自动迁移),给出在容器内的调用方式。
- **⚠️ 每租户 bootstrap**:若业务表 FK `tenants(id)` 且用门户租户 UUID **verbatim、不自动建**(同 knowledge §7),则**首次某租户调用前**要插一行 `tenants(id = 门户租户 UUID, slug)`,否则首次写入 FK 报错。**明确你们是否需要这一步**,有则给命令。
- **异步任务**:清理 / 超时 / 重试策略;worker 起法。
- **镜像交付**:`docker save` + **sha256**(私有 registry 或 tar 包,不上 docker hub)。
- **⚠️ 架构**:本地联调机是 **arm64**,**生产 KubeSphere 是 amd64**。直接出 **amd64 + arm64 multi-arch**(knowledge 第一版只出了 arm64,生产还要补——别重复)。

---

## 8. 自测(无 UI,curl)

> 端口口径:**dev:all 非 docker** 裸跑时是 `:4800`(下例);**门户一盒**里后端在容器内听 8080、经反代同源,改用 `http://localhost/svc/omni-parser/...`(如 `curl http://localhost/svc/omni-parser/health`)。

```bash
# 1) 判活
curl http://localhost:4800/health          # 期望 {"service":"omni-parser","db":"ok",...}

# 2) 缺 token → 401 UNAUTHENTICATED
curl -X POST http://localhost:4800/v1/parse -d '{}'

# 3) 拿一个 aud=omni-parser 的 TDT(经 knowledge 交换;或 dev 自助注册一个 omni-parser app 走 §6 拿)
#    再 curl,aud/scope 不符 → 401/403;正确 → 200 信封
curl -X POST http://localhost:4800/v1/parse -H "Authorization: Bearer <aud=omni-parser TDT>" -F file=@sample.pdf
```

> **⚠️ 信封解包自测**:用一个**有效** TDT 调用,必须被**接受**。若返 `401 INVALID_TOKEN` 而门户自省该 TDT 是 `active:true`,就是没解包 `.data`(knowledge 1.0.0 的同款 bug)——回 §2.1 第 3 条。

---

## 9. 交付物

1. **后端 docker 镜像** —— 听 8080、暴露 `/health`、按 §5 读 env、**amd64 + arm64**。
2. **API reference** —— 同步 `/v1/parse` + 异步 `/v1/jobs` 的端点 / 请求 / 响应 schema、支持的输入模态与格式、大小 / 时长上限、产物结构、webhook 回调与验真方式。
3. **契约清单** —— `listingKey/aud: omni-parser`、`version`、scope(`omni_parser.read`/`omni_parser.parse`)、env 契约最终取值、令牌交换需求(`audience=omni-parser`)。
4. **运维命令** —— DB 迁移 + 每租户 bootstrap(若需)+ `/health` 口径 + 异步 worker 起法。

---

*本文随实现演进。契约以 `docs/SSO与App开发指引.md` + `apps/spms-server/`(参考实现)+ `docs/knowledge-app-contract.md`(同构案例)为准。*
