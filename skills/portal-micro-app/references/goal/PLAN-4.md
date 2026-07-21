# PLAN-4 · XGENT.ai Portal 四期：文件管理 App（独立后端 · 对象存储 · 安全分享 · 协作空间）

> 站在一期底座（`goal/PLAN-1.md`）+ 二期平台化层（`goal/PLAN-2.md`）+ 三期应用市场/内容服务（`goal/PLAN-3.md`）之上。
> 本期交付一个**应用市场上架的独立 App「文件管理」**，区别于三期「内容管理公共服务」：文件能力**自带独立后端**（`@xgent/files-server`），不并入门户 `apps/api`。门户只新增**最小耦合点**：一个 TDT 自省（introspection）端点，让任何独立资源服务器能验证门户签发的 TDT。

---

## 0. 本期范围与决策（已与产品确认）

四个架构分叉已逐一确认，决策如下（与默认推荐不同处在 §0.2 说明取舍）：

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | 文件服务定位 | **App 自带独立后端** | 文件能力是一套独立服务 `@xgent/files-server`（自有 Elysia + Drizzle + 自有库表 + S3 客户端），**不复用** `apps/api` 的 `gate()`。门户其他下游 App 经 **TDT + 令牌交换**调它的 API（需求 #8）。 |
| 2 | 存储配置入口 | **文件管理 App 内（管理员设置页）** | 租户管理员在文件管理 App 的「存储设置」视图里填 S3/minio（endpoint/bucket/AK/SK），直送 **files-server**（TDT 鉴权 + 管理员校验）；SK 加密落库于 files-server。**凭证不进入门户 `apps/api`**（见 §0.2 取舍 b）。 |
| 3 | 密纹（fingerprint）定义 | **仅逐文件哈希（SHA-256 内容摘要）** | 每个对象的密纹 = `SHA-256(bytes)`，上传时由 SDK 计算、finalize 提交、落库。批量密纹 API 取一组 fileKey → 返回 `[{key, digest, ...}]`，**不做组级折叠**。SDK 提供同一算法（`crypto.subtle`）本地复算。 |
| 4 | 上传方式 | **预签名直传（presigned PUT）** | files-server 用租户 S3 凭证签发 presigned PUT URL → 客户端**直传** bucket（不过 files-server）→ 回调 `/finalize` 记元数据/密纹/触发上传 Hook。下载同理走 presigned GET。 |

> 命名：App 中文名「文件管理」，listingKey/appKey `files`；后端服务 `@xgent/files-server`，前端微应用 `@xgent/files-app`。

### 0.1 与三期「内容管理公共服务」的根本区别

| 维度 | 三期 内容服务 | 四期 文件管理（本期） |
| --- | --- | --- |
| 形态 | 门户**公共服务**（`apps/api/modules/content`，Open API `/api/v1/content/*`） | **独立后端 App**（`apps/files-server`，自有 API `/api/v1/files/*`、自有 DB） |
| 鉴权 | 复用门户 `gate()`（进程内 `requireTdt`） | files-server **自实现网关**，经门户 **introspection** 验证 TDT |
| 持久化 | 门户 Postgres `content_entries` | files-server 自有库表 + 租户自带 **S3/minio bucket**（对象） |
| 下游接入 | 同进程，scope 直达 | **令牌交换** A→files（needs grant + 白名单 + scope 交集 + 用户同意）后调 files-server |

### 0.2 关键取舍（确认项的后果，显式记录而非默默处理）

- **a. 独立后端 ⇒ 必须新增门户 introspection（调用方是一个 service account）。** 独立资源服务器拿不到 `apps/api` 进程内的 `requireTdt`/Redis 撤销表。所以门户新增 `POST /api/tokens/introspect`，内部复用 `lib/tdt.ts` 的 `verifyTdtSignature` + `isTdtRevoked` + 成员角色查询，返回 `{active, aud, tenant_id, user_id, scopes, role, exp}`。**调用它的 files-server 是一个专用 service account**（`service_accounts` 表 · 租户无关 · 平台管理员配置，见 §3.1/§3.4）：单一主体、独立可轮换的密钥、`token.introspect` 能力、按 `actorType:"service"` 审计——是**最小权限**凭证(只能验真、不能签发/伪造 token、不碰任何租户数据；租户隔离另由 claims 里的 `tenant_id` 在 files-server 侧收口，见 §0.2c)。因主体租户无关，一把密钥服务所有租户本就在设计内，无多租户隐患。本地 dev 允许「共享 `TDT_SIGNING_KEY` 本地验签」作离线兜底，但默认走 introspection。
- **b. 凭证流向澄清（与所选项预览文案的偏差）。** 所选项预览写「S3 凭证经 iframe ↔ 门户传递」；但既然是**独立后端**，让凭证绕经门户没有意义且更不安全。最终方案：S3 凭证在文件管理 App 的管理员页录入 → **直送 files-server**（带 TDT，files-server 经 introspection 拿到 `role==admin` 才接受）→ AES-256-GCM 加密落 files-server 库。**门户 `apps/api` 永不接触 S3 凭证。** iframe↔门户只承载 SDK 握手与 TDT 下发。此处对预览文案做了纠正，明确记录。
- **c. 租户级 Bucket 隔离 = 租户自带 bucket。** 每个租户在存储设置里指向**自己的 S3 服务/bucket**，隔离天然成立（不同租户不同 bucket/凭证）；files-server 仍按 `spaceId/fileId/filename` 命名 object key。共享单 bucket + `tenant/<id>/` 前缀作为可选退化方案（本地 minio 默认每租户一 bucket）。

### 0.3 明确不在本期

- 文件版本历史/回收站/配额计量；在线预览渲染（Office/PDF 查看器）——仅给 presigned 下载/原生预览。
- 富后处理流水线（病毒扫描、缩略图、OCR/文本抽取入全文索引）——本期 Hook 只交付**事件投递 + 内置「名称/标签入库索引」**，重处理留接口。
- 组级密纹折叠（按确认项 #3 不做）；秒传/分片续传/断点续传。
- 第三方（portal 生态外）应用接入；introspection 的多租户密钥精细化（见 §8 坑位）。

---

## 1. 拓扑与新增工作区

```
apps/
  files-server/   ★新 · 独立后端 (Elysia + Drizzle + @aws-sdk/client-s3)  :4100
  files-app/      ★新 · 文件管理微应用 (Vite + React + Tailwind + shadcn)  :5176
  api/            门户后端 — 仅新增 introspection 端点 + files 作用域 + 市场 seed
  web/            门户壳 — 仅新增 CSP/CORS 放行 + files scope 标签
packages/
  shared/         + files.read/write/share 作用域、错误码、DTO
  portal-sdk/     + sdk.files.*（presign/finalize/list/get/remove/fingerprint/digest/spaces/shares）
基础设施：
  minio           本地对象存储  api :9000 / console :9001（docker）
```

端口：portal-api `:3000`、web `:5173`、sample `:5174`、todo `:5175`、**files-app `:5176`**、**files-server `:4100`**、minio `:9000/:9001`。

> files-server 与 files-app 是两个独立 workspace。files-app（iframe 内）用 `@xgent/portal-sdk` 完成握手取 TDT，再带 TDT 直接调 files-server（跨源，需 CORS + CSP，见 §6 坑位）。

---

## 2. 数据模型

### 2.1 门户侧（`apps/api`，迁移 `0008_service_accounts`）

新增一套**纯净 M2M 身份模型**（租户无关，平台管理员配置）：

| 表 | 关键列 | 说明 |
| --- | --- | --- |
| `service_accounts`（新，**租户无关**主体） | `id, name, clientId unique, status('active'｜'disabled'), capabilities text[]（`token.introspect` / `client_credentials`）, scopes text[]（client_credentials 服务态令牌可携带的 Open API scope）, createdAt, updatedAt` | **无 `tenantId`** ⇒ 单一全局主体，不靠平台租户撑。由平台管理员在控制台管理（§3.4/§3.5）。 |
| `service_account_secrets`（新） | `id, serviceAccountId fk, secretHash, prefix, status('active'｜'revoked'), createdAt, lastUsedAt` | 镜像 `app_secrets`，复用 `sha256`/`randomToken`；一次性展示，可轮换/吊销。 |

- `apps` / `marketplace_listings`：**无需新列**。文件管理作为标准 micro 上架，复用现有快照安装；`marketplace_listings` 仅 seed 一条 `files` 清单。
- introspection 不需要其它新表（复用 `memberships` 取角色、Redis 取撤销版本）。

### 2.2 files-server 侧（自有 Drizzle，`apps/files-server/drizzle/`，独立迁移链）

| 表 | 关键列 | 说明 |
| --- | --- | --- |
| `storage_configs` | `tenantId unique, provider('s3'), endpoint, region, bucket, accessKeyId, secretKeyEnc(AES-GCM), forcePathStyle bool, prefix, createdAt, updatedAt` | 每租户一条；SK 加密落库（`FILES_ENC_KEY`）。租户级 bucket 隔离的承载（需求 #1）。 |
| `spaces` | `id, tenantId, name, kind('personal'｜'team'), ownerUserId null, createdAt` | 个人空间按 (tenant,user) 懒创建；**团队空间归属租户**（不归个人）→ 成员离开空间与文件不灭（需求 #5）。 |
| `space_members` | `spaceId, principalType('user'｜'group'), principalId, role('owner'｜'editor'｜'viewer')`，唯一 `(spaceId, principalType, principalId)` | 成员与归属解耦；删成员不删空间/文件。 |
| `files` | `id, tenantId, spaceId, objectKey, name, folder, size, contentType, digest(SHA-256 密纹), ownerUserId, tags text[], status('pending'｜'active'), searchText, createdAt, updatedAt`；索引 `(tenantId,spaceId)`、GIN(`tags`)、`to_tsvector(searchText)` | presign 时建 `pending` 行，finalize 落 `digest`+meta 转 `active`（需求 #2/#6）。 |
| `file_shares` | `id, tenantId, fileId null, spaceId null, token unique, role('viewer'｜'downloader'｜'editor'), expiresAt null, passwordHash null, createdByUserId, revoked bool, createdAt` | 颗粒度角色 + 过期 + 口令（需求 #4）。fileId/spaceId 二选一。 |
| `file_hooks` | `id, tenantId, event('file.uploaded'), url, active bool, secret null, createdAt` | 租户管理员配置的上传后处理 Webhook（需求 #7）；内置「入库索引」始终执行。 |

> files-server 自带 `db/schema.ts` + `drizzle.config.ts` + `db/migrate.ts`，DB 由 `FILES_DATABASE_URL` 指定（默认与门户同一 Postgres 实例的独立库 `xgent-files`，彻底解耦迁移链）。

### 2.3 shared 增量（`packages/shared`）

- `scopes.ts`：`files.read` / `files.write` / `files.share` + `SCOPE_LABELS`（三语标签经 `labels` 命名空间）。
- `errors.ts`：`STORAGE_NOT_CONFIGURED`、`FILE_NOT_FOUND`、`SPACE_NOT_FOUND`、`SPACE_FORBIDDEN`、`SHARE_NOT_FOUND`、`SHARE_EXPIRED`、`SHARE_PASSWORD_REQUIRED`、`SHARE_PASSWORD_INVALID`、`INTROSPECT_FAILED`、`STORAGE_UNREACHABLE`。
- `dto.ts`：`StorageConfigDTO`（不含明文 SK，回显 `accessKeyId` + `secretKeySet:boolean`）、`SpaceDTO`、`SpaceMemberDTO`、`FileDTO`、`FileFingerprintDTO`（`{key,digest,size,name}`）、`ShareDTO`、`PresignResult`（`{fileId,objectKey,uploadUrl,headers}`）、`FileHookDTO`。
- `constants.ts`：`SHARE_ROLES`/`SPACE_ROLES`/`SPACE_KINDS` + 标签；`FILES_DIGEST_ALGO = "SHA-256"`（密纹算法单一真源，前后端共用）；`FILE_LIST_MAX_LIMIT`。
- **service account（§3.4/§3.5）**：`constants.ts` `SERVICE_ACCOUNT_CAPABILITIES = [token.introspect, client_credentials]` + 标签；`dto.ts` `ServiceAccountDTO`（capabilities/scopes，不含明文密钥，回 `secretPrefix`/`secretSet`）；`errors.ts` `SERVICE_ACCOUNT_NOT_FOUND`、`CAPABILITY_NOT_GRANTED`。

---

## 3. 门户侧机制（`apps/api`，仅 introspection）

### 3.1 TDT 自省端点（需求 #8 的基石）

`POST /api/tokens/introspect`（挂在 `modules/token/index.ts`）：
- **鉴权（service account）**：files-server 用一个**专用 service account**（`service_accounts` 行，**租户无关**）调用——HTTP Basic `clientId:secret` → `validateServiceAccount()`（复用 `sha256`/`timingSafeEqual`）；门户校密钥 + 该账号 `capabilities` 含 `token.introspect`，否则 `FORBIDDEN`。租户无关 ⇒ 一把密钥服务所有租户、可独立轮换/吊销、调用计入审计（`actorType:"service"`）。`token.introspect` 是 service-account **能力**，与用户 consent 的 `SCOPES` 词表分离。dev 兜底：`x-resource-key` == env `FILES_RESOURCE_KEY`（空 ⇒ 端点 fail-closed）。
- **入参**：`{ token }`（被验的 TDT）。
- **逻辑**：`verifyTdtSignature(token)` 失败 → `{active:false}`；`isTdtRevoked(claims)` → `{active:false}`；查 `memberships` 得 `(user_id, tenant_id)` 的 `role`；返回 `{active:true, aud, tenant_id, user_id, scopes, role, exp}`。
- 复用 `lib/tdt.ts` 现成函数，**不碰** TDT 签发逻辑。

### 3.2 下游 App → files 的访问链（需求 #8，零新机制）

完全复用二期/一期令牌交换：下游 App（如「智能批改」）若要读文件，
1. 自身 `apps.scopes` 声明 `files.read`（否则交集后丢失）；用户对其授权。
2. `POST /api/tokens/exchange`（A=mistakes → B=files）：需 `appExchangeGrants` 关系 + files `allowExchange` + 白名单含 A + 用户跨应用同意（`exchangeConsents`）。结果 scope = `intersect(A.scopes, files.scopes)`，`aud=files`。
3. 拿 `aud=files` 的 TDT 调 files-server；files-server 经 §3.1 introspection 验证 → 放行。

> 即「下游 App 通过 TDT 访问 API」= 标准资源服务器模式：files 是 resource，下游 App 交换出 `aud=files` 的 TDT 再调用。门户侧仅需 introspection + 把 files 标为 `allowExchange` 并配白名单（seed）。

### 3.3 App 互联通道（两条，按场景选，互不排斥）

| 通道 | 形态 | 适用 | CORS |
| --- | --- | --- | --- |
| **A · 令牌交换（默认，需求 #8）** | 下游 App **后端** `exchange` 出 `aud=files` 的 TDT 直调 files-server | 服务端到服务端 | server-to-server，无浏览器 CORS |
| **B · host 代理（可选）** | 门户 SDK 暴露**通用原语** `sdk.callService(name, path, init)`：iframe 把调用经 postMessage 交给门户 host，由 host(:3000) 代为 fetch 已注册服务（小注册表 `name → {url, aud}`）再回传 | 微应用**浏览器侧**便利调用 | iframe 不直连 files-server ⇒ 控制面**零 CORS**、files-server 可内网私有、TDT 由 host 挂载 |

- B 是**任何「独立后端」App 都可复用**的浏览器侧互联通道，非 files 专用；`sdk.files.*` 只是它的薄封装。字节上传仍走 direct presigned PUT（只剩 bucket CORS，已由 §4.2 自动化）。
- B 给核心 SDK 加的是**通用** `callService()`，不把某个具体 App 焊进 SDK。

### 3.4 服务账号管理（平台控制台 · 由平台管理员配置）

M2M 主体是一等公民，**平台管理员**在控制台 `/console` 管理（复用 PLAN-2 平台会话网关 `assertPlatformSession` + `actorType:"platform"` 审计）：
- **后端** `modules/platform/service-accounts.ts`，`/api/console/service-accounts/*`：列表（分页 `Page<T>`）/ 创建（返回**一次性**密钥）/ 改（name·status·capabilities·scopes）/ 轮换密钥 / 吊销密钥 / 删除。`validateServiceAccount(clientId, secret, capability)` 供 introspection 与 §3.5 复用。
- **前端** `pages/console/ServiceAccounts.tsx`：列表 + 创建/编辑弹窗（勾选 **capabilities**：`token.introspect` / `client_credentials`；开了 `client_credentials` 再多选 **scopes**）+ 一次性密钥展示 + 轮换/吊销/停用；SideNav「平台」组入口。遵守前端两步法（`impeccable` + Chrome）。
- **seed**：建 `files-server` 服务账号（capability `token.introspect`）+ 初始密钥，使「装上即用」；dev 密钥取自 `FILES_RESOURCE_KEY`，平台管理员可在控制台再轮换。

### 3.5 client_credentials —— 服务态 TDT（M2M 自助签发）

带 `client_credentials` 能力的 service account 可**自助签发服务态 TDT**调门户 Open API（无用户上下文）：
- **端点** `POST /api/tokens/service`（grant_type=`client_credentials`）：Basic `clientId:secret` + `tenant_id` + 可选 `scope`。校 SA active + 含 `client_credentials` 能力（否则 `CAPABILITY_NOT_GRANTED`）+ 密钥有效；`scope = intersect(请求, SA.scopes)`（非扩张）。SA 由平台管理员所建（已具全租户信任）⇒ 可指定任一 `tenant_id`；**按 SA 限定租户集**留作后续。
- **服务态 TDT**（`lib/tdt.ts` `signServiceTdt`）：`kind:"service"`、`sub:"service:<clientId>"`、`user_id:null`、`aud:<clientId>`、`tenant_id`、`scopes`、`actorType:"service"`、新增 per-SA 撤销计数 `sv`。
- **网关接纳**：`requireTdt`/`gate()` 识别 `kind==="service"` → 验签 + jti + `sv`（跳过用户版本校验）。**用户绑定端点**（userinfo、user 域内容、scheduler owner 等）对服务态令牌**拒绝**（`user_id` 空 ⇒ `FORBIDDEN`）；服务态仅可调**租户级、非用户绑定**的 Open API（逐端点白名单）。
- **撤销**：停用 SA / 轮换密钥 → bump `tdt:saver:<clientId>` ⇒ 该 SA 既有服务态 TDT 全失效。
- **范围诚实说明**：files-server 本期只用 `token.introspect`，**不**签发服务态令牌；`client_credentials` 是已就位的通用 M2M 能力，供需要的 SA 使用（验证见 §7）。

---

## 4. files-server 机制（核心）

### 4.1 自实现网关 `gate(headers, scope)`

镜像门户 `modules/openapi/index.ts` 的 `gate()`，但验证走 introspection：
- 取 `Authorization: Bearer <TDT>` → 调门户 `POST /api/tokens/introspect`（带 `x-resource-key`）→ `active && aud==="files"` 否则 `INVALID_TOKEN`；缓存结果至 `exp`（短 TTL）。
- scope 校验：`scopes.includes(scope)` 否则 `INSUFFICIENT_SCOPE`。
- 限流 + 审计：files-server 自有（Redis 计数 + 自有 audit 表/日志），不依赖门户。
- `requireAdmin()`：introspection 回的 `role==="admin"` 否则 `FORBIDDEN`（存储设置/Hook 配置专用）。

### 4.2 存储配置（需求 #1）

- `GET/PUT/DELETE /api/v1/files/storage`（`requireAdmin`）：写入 `storage_configs`（SK 经 AES-GCM 加密）；`PUT` 后做一次 `HeadBucket` 连通性探测，失败回 `STORAGE_UNREACHABLE`。探测通过后自动 `PutBucketCors`，把 files-app 源写入 bucket 的 CORS 策略（放行浏览器 direct presigned PUT/GET）——**免除人工配置 bucket CORS**（直传模型里唯一会咬人的那处，见 §9）。
- `GET` 回 `StorageConfigDTO`（**不回明文 SK**，回 `secretKeySet:true`）。
- S3 客户端：`@aws-sdk/client-s3` + `@aws-sdk/s3-request-presigner`，`forcePathStyle` 适配 minio。每请求按租户配置即时构建 client（按 tenantId 缓存）。

### 4.3 上传/下载（需求 #4 直传 + #2 密纹）

- `POST /api/v1/files/presign`（`files.write`）`{spaceId,name,contentType,size}` → 校验空间成员/角色 → 建 `pending` 文件行 → 返回 `presigned PUT URL`（object key=`<spaceId>/<fileId>/<name>`）。
- 客户端用 SDK 计算 `digest=SHA-256(bytes)`（§4.5）并直传 bucket。
- `POST /api/v1/files/finalize`（`files.write`）`{fileId,digest,size?}` → 转 `active`、落 `digest`/size/searchText → **触发上传 Hook**（§4.6）→ 回 `FileDTO`。
- `GET /api/v1/files/:id/download`（`files.read`）→ presigned GET URL。
- `DELETE /api/v1/files/:id`（`files.write`）→ 删对象 + 行。

### 4.4 密纹批量 API（需求 #2/#3）

- `POST /api/v1/files/fingerprint`（`files.read`）`{keys:[fileKey...]}` → `[{key, digest, size, name}]`（库内逐文件查；**无组级值**，按确认项 #3）。未找到的 key 跳过并在响应里标 `missing:[...]`。

### 4.5 密纹算法（单一真源，前后端一致）

```
file 密纹 = hex( SHA-256( 文件原始字节 ) )      // FILES_DIGEST_ALGO = "SHA-256"
```
- SDK（浏览器）：`crypto.subtle.digest("SHA-256", arrayBuffer)` → hex。
- files-server：finalize 直接采信客户端 `digest`（直传模式服务端不见字节）；可选后台校验——从对象流式读回 `SHA-256` 比对（作为一个内置后处理 Hook，默认关，大文件慎用）。
- 用途：去重、完整性校验、本地集合与服务端集合比对（同算法 ⇒ 同结果）。

### 4.6 上传 Hook / 后处理（需求 #7）

finalize 成功后：
1. **内置**：把 `name + tags` 写入 `files.searchText`（全文索引，始终执行）。
2. **可配 Webhook**：对 `file_hooks`（event=`file.uploaded`, active）逐个投递 `{event, payload:{fileId,objectKey,digest,name,size,spaceId,tenantId,userId}, at}`，best-effort + 短超时（镜像门户 `deliverWebhook`/`fireWebhook` 的范式，在 files-server 内重实现）。可选 HMAC 签名（`file_hooks.secret`）。
- Hook 配置 UI 在管理员设置页（`requireAdmin`）。重处理（缩略图/OCR）留作后续 Hook 消费者，不在本期。

### 4.7 空间与成员（需求 #5）

- `GET/POST /api/v1/files/spaces`、`PATCH/DELETE /api/v1/files/spaces/:id`、`GET/POST/DELETE /api/v1/files/spaces/:id/members`。
- 个人空间懒创建（每用户一个 `kind=personal`）；团队空间 `kind=team` 归租户，成员表独立 → **成员离职只删 membership，空间/文件留存**（连续性）。
- 角色：`owner`（管理成员 + 删空间）/`editor`（增删文件）/`viewer`（只读）。`gate` 后每个文件/空间操作校验调用者在该空间的角色。

### 4.8 搜索与组织（需求 #6）

- `GET /api/v1/files`（`files.read`）`?spaceId=&q=&tags=a,b&type=&owner=&sort=&limit=&offset=`：
  - `q` → `searchText` 全文（`to_tsvector` / 退化 ILIKE）+ `name` ILIKE。
  - `tags` → 数组重叠（`&&`）。`type`（contentType 前缀）、`owner`、日期区间过滤。
  - `sort`/`limit`/`offset` 对齐内容服务 ergonomics（封顶 `FILE_LIST_MAX_LIMIT`）。

### 4.9 分享（需求 #4）

- `POST /api/v1/files/:id/shares`（`files.share`）`{role, expiresAt?, password?}` → 建 `file_shares`（口令 argon2 哈希，`token=randomToken`）。空间级分享同理 `POST .../spaces/:id/shares`。
- **公开访问路由**（无 TDT，files-server 上）：`POST /s/:token`（口令校验）→ 校 `revoked`/`expiresAt`/`passwordHash` → 通过则回 presigned GET（`downloader`/`editor`）或预览元数据（`viewer`）。过期/口令错走 `SHARE_EXPIRED`/`SHARE_PASSWORD_INVALID`（200 + 错误体）。
- `GET/DELETE /api/v1/files/.../shares`：列出/撤销。

---

## 5. Seed / 运行编排 / 配置

- **门户 seed**（`apps/api/db/seed.ts`）：上架 `files` 清单（type micro，embedUrl=`FILES_APP_URL`，scopes `[userinfo.read, files.read, files.write, files.share, notification.send]`，cat「管理」，version `1.0.0`，featured 次于待办）。给晨光租户预装一份 + 配 `allowExchange:true` 并把示例下游（如 `mistakes`）加入交换白名单 + 预置交换授权，演示需求 #8。
- **files-server bootstrap**（`apps/files-server/scripts/bootstrap.ts`）：本地创建 minio bucket（晨光租户）+ 写一条 `storage_configs`（指向 minio）+ 懒建个人/一个团队空间，使「装上即用」。
- **env 新增**（`.env`）：
  - 门户：seed `files-server` **service account**（`service_accounts` 行 + 密钥，capability `token.introspect`，平台管理员可在 `/console` 再配/轮换）；dev 密钥 = `FILES_RESOURCE_KEY`。`FILES_APP_URL=http://localhost:5176`。
  - files-server：`FILES_DATABASE_URL`、`FILES_SERVER_PORT=4100`、`FILES_ENC_KEY`（AES-GCM 32B）、`PORTAL_INTROSPECT_URL=http://localhost:3000/api/tokens/introspect`、service account 的 `FILES_SA_CLIENT_ID`/`FILES_SA_CLIENT_SECRET`（dev 兜底 `FILES_RESOURCE_KEY`）、`REDIS_CONN_STRING`（限流，可与门户共用）、`MINIO_*`（本地默认 endpoint/AK/SK，仅 bootstrap 用）。
- **根 `package.json`**：`dev:all` 加 `@xgent/files-server` + `@xgent/files-app`；新增 `dev:files`（两者并起）。`db:files:generate|migrate`。
- **门户放行（坑位，见 §9）**：`apps/web/index.html` CSP `frame-src` 加 `http://localhost:5176`（必须）；门户 `apps/api` CORS `origin` 加 `FILES_APP_URL`（SDK 握手回源）。**若 files-app 控制面走 §3.3-B host 代理**：iframe 不直连 files-server，无需 files-app `connect-src` / files-server CORS。**若走 §3.3-A 直连**：files-app `index.html` CSP `connect-src` 含 `http://localhost:4100` + files-server CORS 放行 `FILES_APP_URL`。bucket CORS 由 §4.2 自动下发。

---

## 6. 前端（强制两步法：`impeccable` 设计 + Chrome 真浏览器验证）

文件管理微应用 `apps/files-app`（React + Tailwind + **shadcn/ui**，对齐三期待办 App 的栈与三语 i18n 做法）：

- **文件浏览/上传**：空间侧栏 + 文件网格/列表；拖拽上传（presign→直传→finalize 进度）；多选/重命名/删除；面包屑/文件夹。
- **协作空间**：空间切换、成员管理（角色）、个人 vs 团队；空状态/越权态。
- **安全分享**：分享弹窗（角色 + 过期 + 口令）；分享链接复制；公开访问页（口令输入、过期/失效态）。
- **高级搜索**：全文搜索框 + 标签筛选 + 类型/归属/时间过滤；结果高亮。
- **管理员「存储设置」**：S3/minio 表单（endpoint/bucket/AK/SK/forcePathStyle）+ 连通性测试 + Hook 配置；仅 `role==admin` 可见（前端读 `userinfo.role` 隐藏，files-server `requireAdmin` 再校验）。
- **体验底线**（`impeccable`）：跟随宿主主题（`.dark`）与语言（`init.locale`+`onLocale`）；reduced-motion；载入/错误/空态；大文件上传进度与失败重试。
- **i18n**：app 自带 zh-CN/en/zh-TW 词典，跟随宿主实时切换（同三期待办做法）。
- **与 files-server 通信**：默认走 `sdk.files.*`（底层 §3.3-B host 代理，零 CORS）；字节上传走 direct presigned PUT。直连 + CORS 作退化选项。
- 业务态一律 200 + envelope，按 `error.code` 分支（`STORAGE_NOT_CONFIGURED` 引导管理员去配置等）。

> 门户壳前端本期仅新增：files 三个 scope 的三语标签（`labels` 命名空间）。其余 UI 都在 files-app 内。

---

## 7. 验证

- **门户脚本**：`verify-introspect.ts`（service-account 鉴权 + 自省：有效/过期/撤销/缺密钥或能力被拒/角色返回）、`verify-service-accounts.ts`（平台控制台 SA CRUD + capabilities/scopes + 密钥轮换/吊销 + 非平台管理员拦截）、`verify-client-credentials.ts`（签发服务态 TDT → 调租户级 Open API 命中 / 用户绑定端点被拒 / scope 非扩张 / 无 `client_credentials` 能力被拒 / 停用或轮换后既有令牌失效）。
- **files-server 脚本**（`apps/files-server/scripts/`，对真 minio）：
  - `verify-storage.ts`（配置 CRUD + 连通性探测 + SK 不回显 + 非管理员拦截）。
  - `verify-upload.ts`（presign→直传→finalize→下载；pending/active 流转）。
  - `verify-fingerprint.ts`（逐文件密纹落库 + 批量取回 + SDK 同算法复算一致 + missing 标记）。
  - `verify-spaces.ts`（个人/团队空间、成员角色、**移除成员后文件仍在** = 连续性）。
  - `verify-share.ts`（角色/过期/口令；过期与错口令走 200 错误体；撤销失效）。
  - `verify-search.ts`（q/tags/type/owner/sort/limit + 全文）。
  - `verify-exchange-files.ts`（下游 App 交换 `aud=files` TDT → 调 files-server 命中；无授权被拒）= 需求 #8 闭环。
- **Chrome 真浏览器**：登录门户 → 安装/打开文件管理 → 管理员配 minio → 上传（直传进度）→ 建团队空间 + 加成员 → 分享（设口令+过期）开新标签验证公开页 → 搜索/标签筛选 → 切门户语言为 English 验证实时英文 → 移除成员后文件仍可见（连续性）。
- `bun run typecheck` 全 workspace 绿；门户 i18n 三语 parity（新增 3 个 files scope 标签 × 3）；files-app 自带三语词典 parity。

---

## 8. 分期（建议落地顺序）

1. **Phase 0 · 契约与脚手架 + M2M 身份**：shared（scopes/errors/dto/constants + SA capabilities）；**`service_accounts`/`service_account_secrets` 两表（迁移 `0008`）+ 平台控制台 SA 管理（§3.4）**；门户 introspection 端点 + **client_credentials 服务态令牌（`/api/tokens/service` + `lib/tdt.ts` 服务态分支 + 网关接纳，§3.5）** + seed `files-server` SA + 市场 seed + `allowExchange`；新建 `files-server`/`files-app` 两 workspace + files-server Drizzle schema + 首条迁移；minio 基础设施文档/脚本；env。→ `verify-introspect` + `verify-service-accounts` + `verify-client-credentials`。
2. **Phase 1 · 存储 + 直传 + 密纹**：storage_configs（加密）+ S3 客户端 + presign/finalize + 租户 bucket 隔离 + 密纹落库 + 批量密纹 API。→ `verify-storage`/`verify-upload`/`verify-fingerprint`。
3. **Phase 2 · 空间与成员（连续性）+ 文件列表/搜索**。→ `verify-spaces`/`verify-search`。
4. **Phase 3 · 分享（角色/过期/口令）+ 公开访问页**。→ `verify-share`。
5. **Phase 4 · 上传 Hook + 后处理**（事件投递 + 内置索引 + Hook 配置）。
6. **Phase 5 · SDK**：通用 `sdk.callService(name, path, init)` host 代理原语（§3.3-B）+ `sdk.files.*`（presign/finalize/list/get/remove/fingerprint/digest/spaces/shares，默认走 callService）+ 本地 `digest()`。
7. **Phase 6 · files-app 前端**（impeccable 设计 + Chrome 验证）：浏览/上传/空间/分享/搜索/存储设置 + 三语 i18n。
8. **Phase 7 · 下游 App 经 TDT 接入（#8）**：交换链打通 + `verify-exchange-files` + 一个最小下游调用样例。
9. **Phase 8 · 收尾**：i18n parity、README/CHECKLIST、全验证脚本回归。

---

## 9. 关键复用点 & 实现期坑位

**复用**：门户 `lib/tdt.ts`（`verifyTdtSignature`/`isTdtRevoked`）做 introspection；`modules/token/service.ts` `exchange()` + `appExchangeGrants`/`exchangeConsents` 做下游接入（零改造）；`createSecret`/市场 `installListing` 安装文件管理；`deliverWebhook`/`fireWebhook` 范式（在 files-server 内重实现）；前端 `components/ui`/i18n `lib/locales`、三期待办 App 的 shadcn + i18n 结构作为 files-app 模板；`lib/pagination` 范式。

**坑位（务必记住）**：
- **CSP/CORS 放行**（typecheck/脚本查不出，只有真浏览器/真请求能发现）：① 门户 `index.html` `frame-src += :5176`（必须）；② **bucket CORS**（浏览器 direct presigned PUT 必需）——由 §4.2 保存配置时自动 `PutBucketCors` 下发，别手漏；③ 仅 §3.3-A 直连模式才需 files-app `connect-src += :4100` + files-server CORS 放行 files-app 源——**改用 §3.3-B host 代理则这两条都不需要**。门户 introspection 是服务端到服务端，无 CORS。
- **introspection 凭证 = 平台级 service account，不是隔离边界**：单后端多租户用**一个 service 主体 + 一把密钥**自省所有租户 TDT 是安全的——它只回 token 自带声明、不能签发/伪造、不碰数据；**租户隔离由 claims 的 `tenant_id` 在 files-server 侧收口**，与这把密钥无关。(上一稿把它写成「多租户密钥风险」是说重了，已纠正。)service account 用 `client_credentials` 自助签发服务态 TDT 调其它 API 已纳入本期（§3.5）；files-server 自身只用 `token.introspect`，不签发服务态令牌。
- **直传模式服务端不见字节 ⇒ 密纹采信客户端**。完整性强校验需开「流式回读重算」后处理 Hook（默认关）。不要默认相信外部上传者；本期信任边界是「已授权用户经 SDK 计算」。
- **minio forcePathStyle 必开**，否则 presigned URL 走 virtual-host 风格连不上本地 minio；endpoint 用 `http://localhost:9000`。
- **files-server 独立迁移链**：用 `FILES_DATABASE_URL`（默认独立库 `xgent-files`），别和门户 `db:generate`/`db:migrate` 混跑，避免迁移互相覆盖。
- **门户 `apps/api` start 无 --watch**；改 introspection 后需重启。`db:seed` 重生成 UUID → 重登录 + 重新预置交换授权。
- **角色信号来自 introspection**（不在 TDT claims 内）：files-app 管理员页用 `userinfo` 暴露的 `role` 隐藏，但**真正的授权在 files-server `requireAdmin`**（前端隐藏不算鉴权）。需在 §3.1/userinfo 暴露 tenant role。

---

## 10. 需求 → 设计映射（自检）

| 需求 | 落点 |
| --- | --- |
| 1 · 租户级 Bucket 隔离 + 管理员配 S3 + 本地 minio | §2.2 `storage_configs`（每租户自带 bucket/凭证）+ §4.2 管理员设置 + §5 minio bootstrap |
| 2 · 一组 file key → 密纹 API | §4.4 `POST /files/fingerprint`（逐文件 SHA-256） |
| 3 · SDK 同密纹算法 | §4.5 `crypto.subtle SHA-256`，`FILES_DIGEST_ALGO` 单一真源；SDK `files.digest()` |
| 4 · 安全分享（角色/过期/口令） | §4.9 `file_shares` + 公开访问页 |
| 5 · 协作空间（成员离开仍连续） | §4.7 团队空间归租户 + 成员表解耦 |
| 6 · 全文搜索/标签/过滤 | §4.8 `GET /files`（tsvector + tags + 过滤） |
| 7 · 上传后 Hook 后处理 | §4.6 `file.uploaded` Webhook + 内置索引 |
| 8 · 下游 App 经 TDT 访问 | §3.1 introspection + §3.2 令牌交换 + §3.3 互联通道（A 令牌交换 / B host 代理）+ §3.4 服务账号（平台管理的纯净 M2M 身份）+ §3.5 client_credentials 服务态令牌 + §4.1 files-server gate；`verify-exchange-files` |
