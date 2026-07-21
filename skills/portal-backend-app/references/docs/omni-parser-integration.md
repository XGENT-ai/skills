# Omni Parser × XGENT.ai Portal 集成说明（接入 Runbook）

> 把 **omni-parser-server** 作为 **headless 独立后端应用**接入 XGENT.ai Portal，
> 供**文件管理（files）与知识库（knowledge）**两个应用经**跨应用令牌交换（§11）**或
> **服务态 TDT（§7.3）**调用（knowledge 把它当上游，入库前解析）。两个消费方对本服务等价
> ——本服务只验 `aud=omni-parser` + scope，对来源透明。
> 本文是随 Docker 镜像一起交付的接入手册；术语与章节号参见
> [`docs/SSO与App开发指引.md`](./SSO与App开发指引.md)，本仓实现见
> [`docs/plan-sso-integration.md`](./plan-sso-integration.md)。
>
> 交付物：① 镜像 `omni-parser-server`（监听 8080，基于 [`Dockerfile.server`](../Dockerfile.server)）；
> ② 本文；③ 消费方写客户端用的 [结果 schema + 输入格式/限制参考](./consumer-reference.md)。
> omni-parser **无前端、不贡献导航**，只对外提供解析服务。

---

## 0. 一图看懂

```
files 应用（用户操作：上传/解析文件）
   │ files-server 持有用户 TDT (aud=files)
   │ ① 令牌交换 POST /oauth/token (grant=token-exchange, audience=omni-parser)  →  aud=omni-parser 的用户 TDT
   │    （或服务账号 client_credentials 签发服务态 TDT，无用户上下文）
   ▼ Authorization: Bearer <TDT>
reverse-proxy  /svc/omni-parser/*  ──►  omni-parser-server:8080
                                          │ ② 取 Bearer TDT
                                          │ ③ POST /api/tokens/introspect（本服务的服务账号 Basic）
                                          │ ④ 两道闸（aud + scope）+ 租户隔离
                                          ▼ 解析/入队/读结果（按 tenant_id+user_id 收口）
                                          │ ⑤ 异步任务终态 → POST job.callback_url（带 HMAC 签名）回调 files
```

---

## 1. 登记清单（Portal 侧，由平台/接入方在 Portal 仓操作）

omni-parser 作为 **`service` 类型** App 登记（无前端、不露卡）。权威工件是
[`portal/app-devkit/manifests/omni-parser.manifest.json`](../portal/app-devkit/manifests/omni-parser.manifest.json)，
`register-app` 读它完成 listing + 服务账号 + `/svc` 放行的注册。下面是它的关键约定。

### 1.1 Scope（命名空间 `omni_parser`，仅两个，纯 scope 鉴权）

> ⚠️ 命名是**下划线** `omni_parser.*`（listingKey `omni-parser` 的下划线变体）。`omniparser.*`（无分隔符）
> 在 `register-app` 的 namespace 校验下会被拒。**无 `.admin`、无 ACL Manifest**（`aclManifest: null`）。

| Scope | 含义 | 授权屏文案 |
|---|---|---|
| `omni_parser.read` | 查任务 / 取结果（读） | 「读取解析任务/结果」 |
| `omni_parser.parse` | 提交 / 取消解析·OCR·转写·理解·截图（写） | 「提交解析任务（消耗算力）」 |

外加平台基础 scope `userinfo.read`。

### 1.2 `service` listing（headless，无 embedUrl/navItems/ACL）

`omni-parser.manifest.json` 关键字段：

```jsonc
{
  "listingKey": "omni-parser",
  "type": "service",                 // 第四种 App 类型：无前端 headless 后端，对用户隐藏
  "scopes": ["userinfo.read", "omni_parser.read", "omni_parser.parse"],
  "aclManifest": null,               // 纯 scope 鉴权，无 PID / 角色门
  "dependencies": [], "exchangeTargets": [],   // omni-parser 自身不消费别的 App
  "serviceBaseUrl": "/svc/omni-parser",
  "serviceAccount": { "clientId": "omni-parser-server", "secret": "<dev 明文 / 生产随机>" }
}
```

> 消费方（files / knowledge）在**它们自己的**清单里声明 `exchangeTargets:["omni-parser"]` +
> `omni_parser.*` scope，来换 `aud=omni-parser` 的 TDT 调本服务（§3）。omni-parser 这侧 `exchangeTargets` 为空。

### 1.3 端点 → scope 映射（本服务实现，两道闸：aud + scope）

| 方法 + 路径 | 所需 scope | 行级范围 |
|---|---|---|
| `POST /v1/parse` `…/ocr` `…/screenshot` `…/understand/image` `…/transcribe` | `omni_parser.parse` | — |
| `POST /v1/jobs` `POST /v1/understand/video` | `omni_parser.parse` | 写入 `user_id` |
| `GET /v1/jobs/{id}` `…/stream` | `omni_parser.read` | 普通用户→`user_id==caller`；service/admin→本租户 |
| `DELETE /v1/jobs/{id}` | `omni_parser.parse` | 同上 |
| `GET /v1/settings` | `omni_parser.read` | 整租户（纯 scope，见 §6 注） |
| `PUT /v1/settings` | `omni_parser.parse` | 整租户（纯 scope，见 §6 注） |
| `GET /health` `GET /healthz` `GET /readyz` | 公开 | — |

> **纯 scope 模型**：只校验 `aud=omni-parser` + scope，**无 ACL PID 门、无 admin 角色门**（`aclManifest: null`）。
> service 态 TDT（无 user）按 scope 授权、看整租户；普通用户态只看自己的任务。租户隔离始终按 `tenant_id` 收口。

---

## 2. 服务账号（必需）

平台管理员在控制台 `/api/console/service-accounts` 建一个服务账号，授予能力
**`token.introspect`**（本服务验 TDT 必需）。把 `clientId` / `secret` 作为镜像环境变量注入：

```
OMNI_SA_CLIENT_ID=<clientId>
OMNI_SA_SECRET=<secret>     # 走 Secret，绝不进镜像层 / Git
OMNI_PORTAL_INTROSPECT_URL=https://<portal>/api/tokens/introspect
```

设置好两项凭证后，`OMNI_SSO_ENABLED` 自动为 `true`（也可显式置 `true`）。
缺凭证却 `OMNI_SSO_ENABLED=true` 会**启动期 fail-fast**。

自检：

```bash
curl -s -u "$OMNI_SA_CLIENT_ID:$OMNI_SA_SECRET" \
  -H 'content-type: application/json' \
  -d '{"token":"<某个有效 TDT>"}' \
  "$OMNI_PORTAL_INTROSPECT_URL"
# → {"ok":true,"data":{"active":true,"aud":"omni-parser","scopes":[...],...}}
```

> ⚠️ **自省响应是 `{ ok, data }` 信封，claims 在 `.data` 里。** 本服务的自省客户端按
> `claims = body.data ?? body` 解包（见 `src/auth/introspect.rs`，并有回归测试
> `enveloped_introspection_is_unwrapped`），所以会接受真实门户签发的 TDT。**仅验证
> `/introspect` 有返回是不够的**——务必用一个真·交换来的 `aud=omni-parser` TDT 自测能被
> 接受（200/202，而非 401）。裸读顶层 `active` 会拿到 `undefined` → 误判 → 每个有效令牌
> 都 401，是另一后端第一版翻车的同款 bug，本服务已规避。

---

## 3. 消费方接线（files / knowledge，令牌交换 §11）—— 含 scope 交集陷阱

让消费方（files **和** knowledge）能代表用户调 omni-parser——**下列每条对两个消费方各做一遍**：

1. **消费方清单必须在 `scopes` 里声明 `omni_parser.read` / `omni_parser.parse`，并在 `exchangeTargets` 含 `"omni-parser"`。**
   ⚠️ 交换结果 scope = **消费方当前 TDT 的 scopes ∩ omni-parser 声明的 scopes**（§11，绝不扩张）。
   若消费方的 TDT 不含 `omni_parser.*`，交集为空 → 换来的 TDT 无 scope → 调本服务全部 403。**这是最易踩的坑。**
2. 消费方清单声明 `exchangeTargets: ["omni-parser"]`，且 `allowedGrants` 含 `token_exchange`（否则 `GRANT_NOT_ALLOWED`）。
3. omni-parser 安装实例（运营字段）设 `allowExchange = true`，并把两个消费方的 `appKey` 列入
   `exchangeWhitelist = ["files", "knowledge"]`。
4. 平台建立 `app_exchange_grants`：files→omni-parser、knowledge→omni-parser。
5. 用户首次需在交换授权页同意「<消费方> 代表我访问 omni-parser」（否则 `EXCHANGE_CONSENT_REQUIRED`）。

> 这些都在 Portal / 消费方仓侧配置，**本服务代码无需改动**（只验 `aud=omni-parser` + scope）。

**服务态 TDT（§7.3，无用户上下文，用于批处理）替代路径**：消费方侧需具
`client_credentials` 能力的服务账号，且账号 `scopes` 含 `omni_parser.*`（服务态签发同样取「请求 ∩ 账号 scopes」交集），按租户签发。
此路径下本服务的 Job `user_id = NULL`，按 `tenant_id` 隔离仍成立，但失去按用户的 `own` 行级范围。

---

## 4. 部署（镜像 + 反代 + 持久化）

### 4.0 镜像获取

本地联调可用 `docker load` 导入交付的 tar 包（校验 sha256 后）：

```bash
shasum -a 256 -c omni-parser-server-sso.arm64.tar.gz.sha256   # 校验完整性
gunzip -c omni-parser-server-sso.arm64.tar.gz | docker load   # → 载入 xgent-ai/omni-parser-server:sso
```

> 联调镜像为 **arm64**（联调机即 arm64）。生产 amd64 / 多架构镜像经现有
> `.github/workflows/release-docker.yml`（buildx 多架构 + 推 `ghcr.io` + cosign 签名）出，
> 届时按 digest 交付。

### 4.1 容器与命名约定

- 容器**监听 8080**；**服务/容器名必须是 `omni-parser-server`**，以满足反代通用规则
  `/svc/omni-parser/* → omni-parser-server:8080`（§7.5）。
- DB 迁移由本服务**启动时自跑**（sqlx migrate），门户不替它迁移。
- **镜像已内置 `libpdfium.so`**（解析依赖，运行期 dlopen 加载，置于 `/usr/local/lib` + `PDFIUM_LIB_PATH`）；
  以及 Tesseract / ffmpeg / LibreOffice / ImageMagick。无需额外挂载。
- **无需「每租户 bootstrap」**：`tenant_id` 是 jobs 表的**普通文本列**（非 FK 到任何 tenants 表），
  首次调用即直接写入，无安装期灌租户 UUID 的前置步骤。
- 无前端产物（headless），不上传 `dist`、不涉及 `embedCsp` / `frame-src`。

### 4.2 完整环境变量清单

| 变量 | 必需 | 说明 |
|---|---|---|
| `OMNI_SSO_ENABLED` | 推荐 | `true` 进入 SSO 模式（有凭证时自动 true）|
| `OMNI_PORTAL_INTROSPECT_URL` | ✅ | Portal 自省端点 |
| `OMNI_SA_CLIENT_ID` / `OMNI_SA_SECRET` | ✅ | 服务账号凭证（Secret 注入）|
| `OMNI_LISTING_KEY` | — | 身份闸目标，默认 `omni-parser` |
| `OMNI_INTROSPECT_CACHE_SECS` | — | 自省缓存上限，默认 60 |
| `OMNI_RATE_PER_MIN` | — | 每租户每分钟额度，默认 120（0=不限）|
| `OMNI_PORTAL_AUDIT_URL` | — | 统一审计端点（best-effort）|
| `OMNI_CALLBACK_HMAC_SECRET` | 推荐 | 回调签名密钥（空=不签名）|
| `OMNI_CALLBACK_MAX_ATTEMPTS` | — | 回调重试次数，默认 3 |
| `OMNI_DB_URL` | — | `sqlite:///data/omni-jobs.db?mode=rwc`（默认）或 `postgres://…`（生产）|
| `OMNI_ALLOW_UPLOADS` | — | 默认 `false`（SSO 模式）→ **url-only**，拒绝直接 `file` 上传，输入必须传 `url` |
| `OMNI_LLM_*` | — | 可选 LLM 端点（理解/ASR/LLM-OCR），见 [llm-setup.md](llm-setup.md) |

### 4.3 docker run

```bash
docker run -d --name omni-parser-server -p 8080:8080 \
  -e OMNI_SSO_ENABLED=true \
  -e OMNI_PORTAL_INTROSPECT_URL=https://portal.example.com/api/tokens/introspect \
  -e OMNI_PORTAL_AUDIT_URL=https://portal.example.com/api/v1/audit \
  -e OMNI_SA_CLIENT_ID=$OMNI_SA_CLIENT_ID \
  -e OMNI_SA_SECRET=$OMNI_SA_SECRET \
  -e OMNI_LISTING_KEY=omni-parser \
  -e OMNI_RATE_PER_MIN=120 \
  -e OMNI_CALLBACK_HMAC_SECRET=$OMNI_CALLBACK_HMAC_SECRET \
  -v omni-data:/data \
  ghcr.io/<org>/omni-parser-server:<digest>
```

### 4.4 docker compose / K8s

- compose：见仓库 [`docker-compose.yml`](../docker-compose.yml) 的 `omni-server`，已含
  网络别名 `omni-parser-server` 与全部 `OMNI_SSO_*` 变量；机密用 compose `secrets`。
- K8s：固定 `securityContext` / 资源上限，镜像**认 digest**（不可变）；机密走 `Secret`；
  挂 `PersistentVolume` 到 `/data`。`deployDescriptor` 用的镜像 digest 随交付一并给出。

### 4.5 反代路由

确认 `/svc/omni-parser/* → omni-parser-server:8080` 已在白名单 map 中（未注册 = 路由不可达）。
网关只当哑路由，鉴权（aud + scope）全部在本服务实现。

### 4.6 生产形态（持久化与扩缩）

**结论：本服务无需任何对象存储桶，多副本无状态开箱即用** —— 因为：

- **输入永远是 files 的 URL**（用户先把文件传 files，再把 files 预签名下载 URL 经 `url` 字段交给 omni-parser）。
  omni-parser 对 `url` 输入**只存 URL 字符串、不落字节**，worker 懒取（URL 自带签名，**无需 TDT**）。
  → 输入侧天生无状态，任何副本都能处理。**SSO 模式默认 `OMNI_ALLOW_UPLOADS=false`，直接拒绝 `file` 上传**，
  强约束 url-only（不靠约定）。
- **结果是 JSON、存在 DB**（jobs 表 `result` 列），不是文件、不需要桶；用 Postgres 即跨副本共享。
- **数据库**：生产用 `OMNI_DB_URL=postgres://…`（镜像已含 `postgres` 特性，启动自跑 PG 迁移）；
  出队用 `FOR UPDATE SKIP LOCKED`，**多副本 worker 安全并发**。

所以生产形态 = **N 副本无状态 + Postgres**，无 PVC、无对象存储。唯一约束：files 预签名 URL 的有效期
（默认 900s）要盖住「排队 + 执行」时长 —— 长任务让消费方签发更长 TTL 的 URL。

> SQLite 仅单机零运维用（多副本会锁文件），生产必须 Postgres。`LocalBlobStore` 仅在
> `OMNI_ALLOW_UPLOADS=true`（独立/本地 CLI）时才用到，Portal 部署用不上。

**架构**：联调 arm64 已交付；生产 amd64 / 多架构按 §4.0 出。

### 4.7 一盒本地联调（one-box devkit）

用平台的 [`portal/app-devkit`](../portal/app-devkit/README.md) 一把起真实门户 + 本服务。omni-parser 是
`service` 类型，按 [`docs/外部App本地联调指南-one-box.md`](./外部App本地联调指南-one-box.md) 走 §B(curl)：

- 用样例清单 [`portal/app-devkit/manifests/omni-parser.manifest.json`](../portal/app-devkit/manifests/omni-parser.manifest.json) 跑 `register-app`。
- 镜像**同时认平台契约名**（`OMNI_PARSER_*` / `PORTAL_INTROSPECT_URL`）和本服务的 `OMNI_*` 名
  （契约名优先级见 `config.rs::env_any`），所以 devkit 的 `compose.env.app.example` 直接生效、**无需 remap**。
  以下任一组都可：

  ```bash
  # 平台契约名（devkit compose.env.app.example 即用这套）
  OMNI_PARSER_SA_CLIENT_ID=omni-parser-server        # == manifest serviceAccount.clientId
  OMNI_PARSER_SA_CLIENT_SECRET=omni-parser-dev-resource-key
  PORTAL_INTROSPECT_URL=http://portal-api:3000/api/tokens/introspect
  OMNI_PARSER_PG_DSN=postgres://postgres:postgres@postgres:5432/xgent-omni-parser
  OMNI_PARSER_AUDIENCE=omni-parser
  OMNI_SSO_ENABLED=true
  # —— 或本服务原生名 OMNI_SA_CLIENT_ID / OMNI_SA_SECRET / OMNI_PORTAL_INTROSPECT_URL /
  #    OMNI_DB_URL / OMNI_LISTING_KEY，二选一即可。
  ```
- 冒烟：`curl http://localhost/svc/omni-parser/health` → `{"service":"omni-parser","db":"ok",...}`；
  缺/错 token 打 `/svc/omni-parser/v1/parse` → 401/403；拿 `aud=omni-parser` 的 TDT → 200。

**已验证的端到端**（真门户一盒）：files 上传（presign→PUT→finalize）→ files 预签名下载 URL
（`http://minio:9000/...`）→ omni-parser `POST /v1/parse {url}` → `200` + 解析出文本。✅

**起一盒时实测的几个坑（devkit 环境，非本服务问题）**：

1. **建库**：one-box 的 postgres init 只建 files/spms/knowledge 等库，**不建 `xgent-omni-parser`**。先手动建：
   ```bash
   docker compose ... exec -T postgres psql -U postgres -c 'CREATE DATABASE "xgent-omni-parser"'
   ```
   （本服务启动时会自动迁移这个库。）
2. **`/svc` 放行 map**：`register-app` 在一次性 run 容器里可能**写不了** caddy 的 svc-allow 卷（EACCES）。
   反代起来后手动补一行并重启反代：
   ```bash
   docker compose ... exec -T reverse-proxy sh -c 'printf "omni-parser \"1\"\n" > /etc/caddy/svc-allow/omni-parser.map'
   docker compose ... restart reverse-proxy        # 重启即 import；否则 /svc/omni-parser 404
   ```
3. **宿主端口冲突**：若主机已占用 5432/6379/9000，把 `compose.env` 的 `POSTGRES_PORT/REDIS_PORT/MINIO_PORT`
   改到空闲端口（内部容器网络不变，不影响联调）。
4. **铸 TDT（无 UI，纯 curl/脚本）**：照 files `_helpers.ts` 的 dev 流程 ——
   `/auth/dev/start` → `/auth/dev/login?state=<state>&uid=rockie@xgent.ai` → `/auth/switch-tenant`
   → `/api/me/consents {appKey}` → `/api/tokens/mint {appKey}`（拿 `access_token`）。
   注意 dev 管理员 uid 是邮箱 **`rockie@xgent.ai`**，不是 `rockie`。

---

## 5. 回调契约（异步任务，Phase 6c）

异步任务（`POST /v1/jobs`、`/v1/understand/video`）终态后，若提交时带了 `callback_url`，
本服务 **POST** 一个 JSON 到该地址：

```jsonc
{
  "job_id":   "…uuid…",
  "status":   "succeeded" | "failed",
  "tenant_id":"…",
  "app_key":  "…",            // = aud（安装态）
  "result":   { /* 成功时的 OmniResult，失败时为 null */ },
  "error":    "…",            // 失败原因，成功时为 null
  "at":       1718000000      // unix 秒
}
```

- **签名校验**：若配置了 `OMNI_CALLBACK_HMAC_SECRET`，请求带头
  `X-Omni-Signature: sha256=<hex>`，其中 `<hex> = HMAC_SHA256(secret, 原始请求体字节)`。
  另带 `X-Omni-Event: <status>`。接收方应按**原始字节**重算并恒定时间比较。
- **SSRF 约束**：`callback_url` 仅接受 `https`（本机 `http://localhost|127.0.0.1|::1` 例外，用于 dev）；
  其他一律在**提交期**被拒（`400`）。
- **重试**：失败按指数退避（1s/2s/4s…）重试至多 `OMNI_CALLBACK_MAX_ATTEMPTS` 次，best-effort，不阻断 worker。
- **补充拉取**：消费方也可用 SSE `GET /v1/jobs/{id}/stream` 作为回调之外的进度拉取通道。

校验示例（Node）：

```js
import crypto from "node:crypto";
function verify(rawBody, header, secret) {
  const expected = "sha256=" + crypto.createHmac("sha256", secret).update(rawBody).digest("hex");
  return crypto.timingSafeEqual(Buffer.from(header), Buffer.from(expected));
}
```

---

## 6. 应用配置（`/v1/settings`，Phase 6a）

租户管理员可存一份**租户级 `OmniParserConfig` 覆盖**，运行时按
`base → 租户覆盖 → 单次请求覆盖` 叠加到该租户的每次解析（同步与异步均生效）。

- 配置 UI **复用 Portal 的「应用配置」页**：浏览器用 Portal 铸的**管理员短时 TDT**
  （含 `omni_parser.parse`）**直连本后端** `/v1/settings`，门户不经手敏感配置。
- 读：`GET /v1/settings` → 当前覆盖（未设为 `{}`）。写：`PUT /v1/settings`（JSON 对象，整体替换）。

```bash
curl -X PUT https://portal.example.com/svc/omni-parser/v1/settings \
  -H "authorization: Bearer <管理员 TDT>" -H 'content-type: application/json' \
  -d '{"ocr_mode":"ensemble","dpi":200}'
```

---

## 7. 冒烟验证（接入后）

```bash
BASE=https://portal.example.com/svc/omni-parser
TDT_T1=<租户1 用户的交换 TDT>     # 含 omni_parser.read+parse
TDT_T2=<租户2 用户的交换 TDT>

# 0) 公开探针
curl -s $BASE/health                   # → {"service":"omni-parser","db":"ok",...}

# 1) 解析（写）→ 期望 200
curl -s -F file=@report.pdf -H "authorization: Bearer $TDT_T1" $BASE/v1/parse | jq '.modality'

# 2) 提交异步任务 → 202 + job
JOB=$(curl -s -F file=@clip.mp4 -H "authorization: Bearer $TDT_T1" $BASE/v1/understand/video | jq -r .id)

# 3) 跨租户隔离 → 期望 404（租户2 看不到租户1 的任务）
curl -s -o /dev/null -w '%{http_code}\n' -H "authorization: Bearer $TDT_T2" $BASE/v1/jobs/$JOB   # → 404

# 4) 限流 → 连续超过 OMNI_RATE_PER_MIN 次后期望 429
for i in $(seq 1 130); do
  curl -s -o /dev/null -w '%{http_code} ' -F file=@a.png -H "authorization: Bearer $TDT_T1" $BASE/v1/ocr
done   # 末尾出现 429

# 5) 错误码：缺 scope → 403 INSUFFICIENT_SCOPE；有 scope 无 PID → 403 INSUFFICIENT_PERMISSION
```

期望响应：跨租户/他人任务一律 `404`（不暴露存在性）；scope/PID 不足 `403`；
无效/撤销 TDT `401`；自省端点不可达 `503`（`INTROSPECT_FAILED`）；超额 `429`。

---

## 8. 排错速查

| 现象 | 多半原因 |
|---|---|
| 启动即退出，日志 `OMNI_SSO_ENABLED is set but … missing` | 缺 `OMNI_SA_CLIENT_ID/SECRET`（fail-fast）|
| **每个有效 TDT 都 `401`（自省明明有返回）** | 自省响应是 `{ok,data}` 信封、claims 在 `.data`，客户端裸读顶层 `active` 拿到 `undefined`。本服务已按 `data ?? body` 解包（§2）；若仍翻车，确认门户未改响应形状 |
| 所有请求 `401 INVALID_TOKEN` | 身份闸不过：`OMNI_LISTING_KEY` 与 TDT 的 `listingKey/aud` 不一致 |
| 消费方调用全部 `403 INSUFFICIENT_SCOPE` | **scope 交集为空**：消费方（files/knowledge）清单未声明 `omni_parser.*`（§3 陷阱）|
| `503 INTROSPECT_FAILED` | 自省端点不可达 / `OMNI_PORTAL_INTROSPECT_URL` 配错 / 服务账号无 `token.introspect` |
| 回调收不到 / 验签失败 | `callback_url` 非 https 被提交期拒；或两侧 `OMNI_CALLBACK_HMAC_SECRET` 不一致；按原始字节验签 |
| 路由 404（反代） | 服务名不是 `omni-parser-server`，或 key 未进网关白名单 |
