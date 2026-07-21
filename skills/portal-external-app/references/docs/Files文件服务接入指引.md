# Files 文件服务接入指引

面向**需要用到「文件管理」能力的 App 开发者**——无论你是嵌入 Portal 的微应用，还是有自己进程/数据库的独立后端。本文讲清：你的 App 如何把「上传 / 下载 / 检索 / 协作空间 / 安全分享 / 内容去重」这套能力**复用** files 服务，而不必自己接对象存储、自己签名、自己做分享门。

> **files 是一个独立后端 App**（`@xgent/files-server`），不是 Portal 内置能力。它自带库（`FILES_DATABASE_URL = xgent-files`）、对接**每租户自己的** S3/MinIO/COS 桶，通过门户自省端点验 Portal 签发的 TDT。它就是 [`SSO 与 App 开发指引`](SSO与App开发指引.md) §7「独立后端应用」模式的**参考实现**。
>
> 阅读前置：先读 [`SSO与App开发指引.md`](SSO与App开发指引.md)——本文复用它的全部概念（TDT、scope、consent、令牌交换、host-proxy）。本文只补「files 这一面」。
> 所有 HTTP 约定遵循 [`CLAUDE.md`](../CLAUDE.md)：**业务状态不走 HTTP 状态码**，一律 `200 + 响应体 { ok, data | error }`。

---

## 目录

- [1. files 能给你什么](#1-files-能给你什么)
- [2. 数据模型：空间 / 文件 / 分享 / 存储](#2-数据模型空间--文件--分享--存储)
- [3. 选择你的对接方式](#3-选择你的对接方式)
- [4. 前置：在清单里声明依赖与 scope](#4-前置在清单里声明依赖与-scope)
- [5. 方式一：iframe 微应用用 `sdk.files.*`（零跨域，推荐）](#5-方式一iframe-微应用用-sdkfiles零跨域推荐)
- [6. 方式二：独立后端经令牌交换调 files-server](#6-方式二独立后端经令牌交换调-files-server)
- [7. 上传三步曲：presign → 直传 → finalize](#7-上传三步曲presign--直传--finalize)
- [8. Files Open API 全集](#8-files-open-api-全集)
- [9. 权限模型：scope × 空间角色 × ACL](#9-权限模型scope--空间角色--acl)
- [10. 内容去重（密纹 / fingerprint）](#10-内容去重密纹--fingerprint)
- [11. 安全分享与公开访问](#11-安全分享与公开访问)
- [12. 上传后处理：Webhook（file.uploaded）](#12-上传后处理webhookfileuploaded)
- [13. 租户存储桶配置（管理控制面）](#13-租户存储桶配置管理控制面)
- [14. 错误码目录（files 专属）](#14-错误码目录files-专属)
- [15. 本地开发与联调](#15-本地开发与联调)
- [16. 应用存储（其他应用利用文件服务存储）](#16-应用存储其他应用利用文件服务存储)
- [附录 A：类型参考（DTO）](#附录-a类型参考dto)
- [附录 B：关键文件索引](#附录-b关键文件索引)

---

## 1. files 能给你什么

你的 App 不必碰对象存储、不必管签名、不必自建分享逻辑，就能拥有：

| 能力 | 说明 |
| --- | --- |
| **预签名直传** | 浏览器/客户端拿到一个短期 PUT URL，**字节直传租户自己的桶**，不经过门户、也不经过 files-server（省带宽、零中转）。 |
| **内容密纹去重** | 每个文件落一个 SHA-256 **密纹（digest）**；按 `fileId` 批量查密纹，下游可据此判重/秒传。 |
| **个人 / 团队协作空间** | 个人空间按 `(租户,用户)` 懒创建；团队空间归租户所有，成员有 `owner/editor/viewer` 角色；**成员离开不删空间和文件**。 |
| **全文检索** | 按文件名 + 标签建 `searchText`，支持 `q / tags / type / owner / sort / 分页`。 |
| **安全分享** | 对单文件或整个空间生成分享链接，带**角色（仅看 / 可下载 / 可编辑）+ 可选有效期 + 可选口令**；公开访问页**无需 TDT**。 |
| **上传 Webhook** | 租户管理员可配 `file.uploaded` 回调（带可选 HMAC 签名），把上传事件喂给你自己的流水线（如解析、转码、入库）。 |
| **多云存储** | 同一套驱动支持 `s3`（通用 S3 兼容 / MinIO）、`aws`（AWS S3）、`cos`（腾讯云 COS）。**每租户指向自己的桶**（租户级隔离），密钥 AES-256-GCM 落库、门户永不经手。 |

**files-server 与门户的唯一耦合点**就是一个自省端点（`PORTAL_INTROSPECT_URL`）。它不共享门户进程内鉴权，自己实现 [SSO §7.1 的四道闸](SSO与App开发指引.md#71-你的后端如何验-tdt令牌自省introspection)：身份（`aud/listingKey === "files"`）→ scope（`files.read/write/share`）→ 角色（结构性操作要 `admin`）→ 用户上下文（文件归属用户）。

---

## 2. 数据模型：空间 / 文件 / 分享 / 存储

四张核心表（`apps/files-server/src/db/schema.ts`），都按 `tenantId` 隔离：

```
storage_configs ── 每租户一行：指向该租户自己的桶（SK 加密落库）
       │
spaces ──┬── personal：按 (tenant,user) 懒创建，名「我的文件」，删不掉
         ├── team：归租户，有成员表；成员退出不删空间/文件（continuity）
         └── app：应用存储（FILE-STORAGE-2）；每租户×每 App 一个，懒创建；
                  ownerUserId=null、ownerAppKey=<App listingKey>，文件属应用不属用户
         │
   space_members ── (user|group|app) × role(owner/editor/viewer)   # app 主体可把目录授权给别的 App
         │
files ── 元数据 + objectKey + digest(密纹) + status(pending|active) + searchText
         + ownerUserId(可空，应用文件为 null) + viaAppKey(来源 App·两场景共用)
         │
file_shares ── 覆盖「单文件 OR 整个空间」：role + 可选 expiresAt + 可选 passwordHash
```

要点：
- **对象存储里只有字节，元数据全在 files 库**。`objectKey = <prefix>/<spaceId>/<fileId>/<sanitized-name>`。
- **文件有两态**：`presign` 建一条 `pending` 行；`finalize` 落 `digest/size/searchText` 并翻成 `active`。列表/检索只返回 `active`。
- **空间角色**（`SPACE_ROLES = owner > editor > viewer`）决定「在这个空间能做什么」，与 Portal 的租户角色、ACL 是**两层**（见 [§9](#9-权限模型scope--空间角色--acl)）。
- **个人空间不可重命名 / 不可删除**；团队空间的 owner 才能改名 / 删除 / 管成员 / 建空间级分享；**应用目录（`kind=app`）随 App 生命周期，亦不可经空间接口改名/删除**。
- **应用存储是第二类视角**（用户视角之外）：其他 App 借文件服务存内容 —— 见 [§16](#16-应用存储其他应用利用文件服务存储)。`viaAppKey` 标记「这个文件由哪个 App 经手」，两场景共用。

---

## 3. 选择你的对接方式

| 你的形态 | 怎么调 files | 适用 |
| --- | --- | --- |
| **A. iframe 微应用**（`type:"micro"`） | `sdk.files.*`（宿主 host-proxy 代转发，**零跨域**） | 绝大多数场景：你要在自己的页面里上传/列文件/做分享。**首选**。见 [§5](#5-方式一iframe-微应用用-sdkfiles零跨域推荐)。 |
| **B. 独立后端**（自有进程） | **令牌交换** 拿一个 `aud=files` 的 TDT → 直连 files-server `/api/v1/files/*` | 你的后端要代表用户读写文件（如知识库后端摄入用户上传的文档）。见 [§6](#6-方式二独立后端经令牌交换调-files-server)。 |
| **C. 管理控制面** | 平台铸一把 600s **admin-TDT**（`GET /api/admin/apps/:appKey/files-token`）直连 files-server `/api/v1/storage` | 仅当你的 App 就是「文件后端」、需要在 Portal 应用配置页改存储桶。一般 App 用不到。见 [§13](#13-租户存储桶配置管理控制面)。 |

> 三种方式**底层都是同一套 files-server REST**（[§8](#8-files-open-api-全集)），区别只在「这把 TDT 从哪来、怎么送到 files-server」。文件操作**都需要用户上下文**（`requireUser`）——服务态 TDT（`kind:"service"`，无 `user_id`）会被拒，因为文件归属用户/空间。

---

## 4. 前置：在清单里声明依赖与 scope

无论哪种方式，你的 App 清单（marketplace listing）要做两件事：

### 4.1 声明对 files 的依赖

```ts
{
  listingKey: "your-app",
  dependencies: ["files"],          // 安装你的 App 时按拓扑序自动补装 files（SSO §4.5）
  // ...
}
```

安装会确保 files 先装好；卸载 files 时若你的 App 还在，会被 `DEPENDENCY_REQUIRED` 拦截。

### 4.2 声明 files scope

```ts
scopes: [
  "userinfo.read",
  "files.read",     // 列/读文件、空间、分享、查密纹
  "files.write",    // presign/finalize、改名/删除、建/管空间、（管理员）存储与 Webhook
  "files.share",    // 创建/吊销分享链接
],
```

最小够用即可——只读消费方只声明 `files.read`。用户授权屏会逐条展示，scope 越多越劝退。完整 scope 定义见 `packages/shared/src/scopes.ts`：

| Scope | 含义 |
| --- | --- |
| `files.read` | 读文件与空间、查密纹、列分享 |
| `files.write` | 上传 / 改名 / 删除 / 建管空间 / （管理员）存储桶与 Webhook |
| `files.share` | 创建 / 吊销分享链接 |

### 4.3 独立后端额外：声明令牌交换目标

方式 B 还要让你的 App 能把自己的 TDT 换成 `aud=files` 的 TDT：

```ts
{
  listingKey: "your-app",
  exchangeTargets: ["files"],       // 安装时自动建立 你的App → files 的交换白名单（SSO §11）
  allowedGrants: ["authorization_code", "token_exchange", "refresh_token"],
}
```

> `exchangeTargets` 解决了「结构上允许交换」（grant + 白名单）；运行时还要**用户事先授权**「你的 App 代表我访问 files」，否则交换抛 `EXCHANGE_CONSENT_REQUIRED`（见 [§6](#6-方式二独立后端经令牌交换调-files-server)）。

---

## 5. 方式一：iframe 微应用用 `sdk.files.*`（零跨域，推荐）

你的页面被 Portal 以沙箱 iframe 嵌入，用 `@xgent/portal-sdk` 握手拿到当前用户在当前租户下的 TDT。`sdk.files.*` 是 `sdk.callService("files", …)` 的薄封装——**宿主代你铸 TDT、代你 `fetch` files-server、把响应回传**，你的 iframe 完全不碰 CORS。

```
 iframe 应用                宿主 (MicroAppHost)              files-server (:4100 / /svc/files)
    │                            │                                   │
    │ sdk.files.list()           │                                   │
    │ = callService("files",     │                                   │
    │     "/api/v1/files", …)     │                                   │
    │───── postMessage ─────────►│ ① resolveServiceBase("files")     │
    │                            │   ← GET /api/apps/files .serviceBaseUrl
    │                            │ ② mint/复用 host-proxy TDT         │
    │                            │   ← POST /api/tokens/mint {appKey} │
    │                            │ ③ fetch + Bearer ─────────────────►│ gate: aud/scope/role/user
    │                            │                                   │ ← { ok, data }
    │◄───── 回传 data ───────────│ （401/403 自动重铸一次重试）       │
```

### 5.1 `sdk.files.*` 速查（`packages/portal-sdk/src/index.ts`）

```ts
import { createPortalClient } from "@xgent/portal-sdk";
const sdk = createPortalClient();
await sdk.ready();                              // 握手；files 操作要 user-kind TDT（iframe 注入即是）

// —— 上传 ——
const digest = await sdk.files.digest(file);    // 浏览器侧算 SHA-256（hex 串）
const f = await sdk.files.upload(file, {        // 一把梭：presign → 直传(带进度) → finalize
  spaceId,                                       // 省略 = 落个人空间
  tags: ["合同", "2026"],
});
// 想自己控进度/分片，就拆开调：
const pre  = await sdk.files.presign({ spaceId, name: file.name, contentType: file.type, size: file.size });
// ……浏览器对 pre.uploadUrl 直接 PUT（只带 pre.headers）……
const meta = await sdk.files.finalize({ fileId: pre.fileId, digest, size: file.size, tags });

// —— 文件 CRUD / 检索 ——
const { items, total } = await sdk.files.list({ spaceId, q: "合同", type: "image/", tags: ["x"], sort: "-createdAt", limit: 50, offset: 0 });
const one  = await sdk.files.get(id);
const dl   = await sdk.files.download(id);       // { url, name, contentType, size }；url 是 900s 预签名 GET
window.open(dl.url, "_blank", "noopener");
await sdk.files.rename(id, { name: "新名.pdf", tags: ["归档"] });
await sdk.files.remove(id);
const { items: fps, missing } = await sdk.files.fingerprint([fileId1, fileId2]); // 批量查密纹

// —— 协作空间 ——
const spaces = await sdk.files.spaces.list();    // 个人空间在前，团队按 createdAt
const team   = await sdk.files.spaces.create("市场部协作");
await sdk.files.spaces.rename(team.id, "市场部");
await sdk.files.spaces.remove(team.id);
const members = await sdk.files.spaces.members(team.id);
await sdk.files.spaces.addMember(team.id, userId, "editor");      // role ∈ owner|editor|viewer
await sdk.files.spaces.removeMember(team.id, userId);

// —— 安全分享 ——
const share = await sdk.files.shares.forFile(fileId, { role: "downloader", expiresAt: "2026-12-31T00:00:00Z", password: "1234" });
const sShare = await sdk.files.shares.forSpace(spaceId, { role: "viewer" });
const list  = await sdk.files.shares.listForFile(fileId);
await sdk.files.shares.revoke(share.id);
// 分享链接 = `${filesAppOrigin}/s/${share.token}`（公开访问页，见 §11）

// —— 仅管理员：存储桶 & Webhook（见 §13 / §12）——
const cfg = await sdk.files.storage.get();
await sdk.files.storage.put({ provider: "s3", endpoint, region, bucket, accessKeyId, secretKey, forcePathStyle: true, prefix: "" });
await sdk.files.hooks.create({ url: "https://you/webhook", secret: "…", active: true });
```

### 5.2 直连退路（不走 host-proxy）

如果你坚持让 iframe **直接** `fetch` files-server（而非 `sdk.callService`），files-server 的 CORS 白名单只放行 `FILES_APP_URL` + `PORTAL_BASE_URL` 两个源（`apps/files-server/src/index.ts`）。生产要加你的源就得改它的环境变量——**强烈建议直接用 `sdk.files.*`**，零配置零跨域。

---

## 6. 方式二：独立后端经令牌交换调 files-server

你的后端（自有进程/库）要**代表用户**读写文件时，走 [OAuth 令牌交换](SSO与App开发指引.md#11-跨应用调用令牌交换-token-exchange)：把你手上 `aud=你的App` 的用户 TDT 换成一个 `aud=files` 的 TDT，再拿它直连 files-server。

参考实现与回归用例：[`apps/files-server/scripts/verify-exchange-files.ts`](../apps/files-server/scripts/verify-exchange-files.ts)（错题本 `mistakes` → files）。

```
① 你的后端已持有用户 TDT（aud=your-app，含 files.read 等 scope）
② 换成 files 的 TDT：
   POST {PORTAL}/oauth/token
     grant_type    = urn:ietf:params:oauth:grant-type:token-exchange
     client_id     = your-app                 # 必须等于 subject_token.aud
     client_secret = <你的 App Secret>
     subject_token = <你的用户 TDT>
     audience      = files
   → { access_token: <aud=files 的 TDT>, scope, expires_in }
③ 用它直连 files-server：
   GET {FILES_BASE}/api/v1/files/spaces
     Authorization: Bearer <aud=files 的 TDT>
```

交换**严格不可提权**，必须同时满足（见 SSO §11）：

1. 存在 `your-app → files` 的交换授权关系（清单声明 `exchangeTargets:["files"]` 安装时自动建，[§4.3](#43-独立后端额外声明令牌交换目标)）。
2. files 开启了交换（`allowExchange=true`，运营字段）。
3. your-app 在 files 的 `exchangeWhitelist`（同样由 `exchangeTargets` 自动写入）。
4. **scope 取交集**：换出的 scope = 你的 TDT 的 scopes ∩ files 声明的 scopes（`files.read/write/share`）。**绝不扩张**——你的 TDT 只有 `files.read`，换出来就只有 `files.read`，去 presign 会吃 `INSUFFICIENT_SCOPE`。
5. **用户同意**：用户须事先在授权页授权「your-app 代表我访问 files」，否则 `EXCHANGE_CONSENT_REQUIRED`。授权页路由 `/exchange-consent?source=your-app&target=files&return=…`。

`verify-exchange-files.ts` 验证的关键事实（直接抄成你的对接 checklist）：

- ✅ `aud=your-app` 的 TDT **不能**直连 files-server → `401 INVALID_TOKEN`（aud 必须是 files）。
- ✅ 交换出的 `aud=files` TDT → files-server 放行。
- ✅ 换出 scope = 交集（只 `files.read` 就别想 `files.write`）。
- ✅ 用户无交换 consent → `EXCHANGE_CONSENT_REQUIRED`；client_secret 错 → `SECRET_INVALID`。

> 你的后端**不需要**自己的服务账号去自省——自省是 files-server 的事。你只要能做交换（持 App Secret）+ 拿到换出的 TDT 即可。文件操作要用户上下文，所以 `subject_token` 必须是 user-kind（带 `user_id`）。

---

## 7. 上传三步曲：presign → 直传 → finalize

这是 files 最核心的流程，三种对接方式都一样（方式一用 `sdk.files.*`，方式二/三用 REST）。**字节直传桶，不经过 files-server**。

```
你的客户端                    files-server                    租户的桶 (S3/MinIO/COS)
   │                              │                                 │
   │ ① 算 digest（SHA-256）        │                                 │
   │                              │                                 │
   │ ② POST /api/v1/files/presign │                                 │
   │   { spaceId?, name, contentType, size, folder? }               │
   │─────────────────────────────►│ 建 pending 行 + 预签名 PUT(900s) │
   │   ◄── { fileId, objectKey, uploadUrl, headers } ───────────────│
   │                              │                                 │
   │ ③ PUT uploadUrl（只带 headers，字节为 body）────────────────────►│ 直接落桶
   │   ◄────────────────────────────────────── 200 ────────────────│
   │                              │                                 │
   │ ④ POST /api/v1/files/finalize│                                 │
   │   { fileId, digest, size?, tags? }                             │
   │─────────────────────────────►│ 落 digest/size/searchText →     │
   │                              │ status=active + 触发上传 Hook    │
   │   ◄── FileDTO ───────────────│                                 │
```

**两个必须遵守的细节（否则直传 403 SignatureDoesNotMatch）**：

1. **PUT 时只发 presign 返回的 `headers`**（即 `{ "Content-Type": <contentType> }`），一字不差。`Content-Type` 是签名的一部分。别让 SDK/浏览器自动加 `x-amz-checksum-*` 之类——files-server 已用 `requestChecksumCalculation: "WHEN_REQUIRED"` 关掉新版 aws-sdk 的自动校验头来兼容 MinIO/COS/旧版 S3（`apps/files-server/src/lib/s3.ts` 的 `CHECKSUM_COMPAT`）。
2. **桶的 CORS 必须允许浏览器直传**。files-server 在保存存储配置时**自动发布** CORS（`AllowedMethods: GET/PUT/HEAD`，`AllowedOrigins: [filesAppUrl, portalBaseUrl, "*"]`）。自托管/自管桶要保证这条在。

`finalize` 只能 finalize**你自己上传**的那条 pending 行（`ownerUserId === 你`），否则 `SPACE_FORBIDDEN`。`finalize` 内置「`name + tags → searchText`」全文索引，并触发租户配置的 `file.uploaded` Webhook（[§12](#12-上传后处理webhookfileuploaded)）。

下载同理是预签名 GET（900s）：`GET /api/v1/files/:id/download` → `{ url, … }`，`url` 可直接 `window.open` 或 `<a download>`，files-server 会带上 `Content-Disposition: attachment; filename=…`。

---

## 8. Files Open API 全集

基址：方式一由宿主解析（生产同源 `/svc/files`，本地 `http://localhost:4100`）；方式二/三是 `FILES_BASE`。所有路径前缀 `/api/v1/files`（公开分享除外）。统一信封 `{ ok, data | error }`。

每个端点：① 自省校验 TDT（`aud/listingKey === "files"`）；② 校验 scope；③ 按 `(aud, tenant)` 限流 **600/min**；④ 多数还要校验**空间角色**或**租户管理员**。

### 8.1 文件

| 方法 + 路径 | scope | 额外门 | 说明 / Body |
| --- | --- | --- | --- |
| `POST /presign` | `files.write` | 空间内 `editor` | `{ spaceId?, name, contentType?, size?, folder? }` → `PresignResult`。无 `spaceId` 落个人空间 |
| `POST /finalize` | `files.write` | 必须是该 pending 行的 owner | `{ fileId, digest?, size?, tags? }` → `FileDTO`（翻 active + 触发 Hook） |
| `GET ` (列表/检索) | `files.read` | 可见空间内 | query：`spaceId? / q? / tags?(逗号分隔名称,any-of) / tagIds?(逗号分隔词典标签 id,any-of,FILE-ORGANIZE) / folder?(精确路径,须同传 spaceId) / withThumbs?(=1 附缩略图,见 §8.8) / type?(contentType 前缀) / owner? / via? / sort?(默认 `-createdAt`，可 name/size/updatedAt) / limit?(≤200) / offset?` → `{ items: FileDTO[], total }` |
| `POST /fingerprint` | `files.read` | 可见空间内 | `{ keys: fileId[] }` → `{ items: FileFingerprintDTO[], missing: string[] }` |
| `GET /:id` | `files.read` | 空间成员 | → `FileDTO` |
| `GET /:id/download` | `files.read` | 空间成员 | → `{ url, name, contentType, size }`（预签名 GET 900s） |
| `PATCH /:id` | `files.write` | 空间内 `editor` | `{ name?, tags?, folder? }` → `FileDTO`（重算 searchText；`folder` 为同空间移动,严格规范路径） |
| `DELETE /:id` | `files.write` | 空间内 `editor` | 删对象 + 行 → `{ deleted: true }` |

> 列表 / 检索默认只在「**用户可见的空间**」（个人空间 + 其为成员的团队空间）内查；传 `spaceId` 则限定该空间（须有角色）。只返回 `active` 文件。
>
> FILE-ORGANIZE 契约注：`FileDTO.tags` 恒为**当前 actor 可见的标签名称数组**（租户共享 ∪ 自己的个人标签,来自词典组装）；`?tags=名称` 保持名称语义（跨分类同名取并集,未知名 → 空结果,**GET 绝不落库**）；finalize/PATCH 的 `tags: string[]` 契约不变（最多 24 个名称,内部在系统默认分类下 get-or-create 共享标签）。`withThumbs=1` 是可选副作用：只入队、有界,converter 未配置时安静降级（`thumbUrl:null`,不 500）。

### 8.2 协作空间与成员

| 方法 + 路径 | scope | 额外门 | 说明 |
| --- | --- | --- | --- |
| `GET /spaces` | `files.read` | — | 列我可见的空间（含 `memberRole` + `fileCount`＝**根目录直属**文件数,与目录树逐节点口径一致；`/app-spaces` 仍为全量）,个人空间在前 |
| `POST /spaces` | `files.write` | — | `{ name }` 建团队空间（建者即 owner） |
| `PATCH /spaces/:id` | `files.write` | `owner`；个人空间禁改 | `{ name }` |
| `DELETE /spaces/:id` | `files.write` | `owner`；个人空间禁删 | 级联删成员 + 文件行 |
| `GET /spaces/:id/members` | `files.read` | `viewer` | → `SpaceMemberDTO[]` |
| `POST /spaces/:id/members` | `files.write` | `owner` | `{ principalId, principalType?(user\|group), role?(owner\|editor\|viewer) }`（upsert） |
| `DELETE /spaces/:id/members/:principalType/:principalId` | `files.write` | `owner` | 移除成员；**路径必须带 `principalType`**（唯一键含它，user/group/app 的 id 可能撞值）；**不能移除最后一名 owner**；移除只断成员关系，空间与文件保留 |

### 8.3 安全分享（鉴权侧）

| 方法 + 路径 | scope | 额外门 | 说明 |
| --- | --- | --- | --- |
| `POST /:id/shares` | `files.share` | 文件所在空间的成员 | `{ role?, expiresAt?, password? }` → `ShareDTO`（单文件分享） |
| `POST /spaces/:id/shares` | `files.share` | 空间 `owner` | 同上 → 空间级分享 |
| `GET /:id/shares` | `files.read` | 空间成员 | 列某文件的分享 |
| `GET /spaces/:id/shares` | `files.read` | `viewer` | 列某空间的分享 |
| `DELETE /shares/:shareId` | `files.share` | 创建者**或**该分享所属空间的 `owner` | 吊销（软删 `revoked=true`） |

分享 `role`（`SHARE_ROLES`）：`viewer`（仅元数据）/ `downloader`（可拿预签名下载）/ `editor`（同 downloader 口径下可下载）。

### 8.4 公开访问（**无 TDT**，给分享接收方）

| 方法 + 路径 | 说明 |
| --- | --- |
| `GET /s/:token` | 探测：`{ kind:"file"\|"space", role, requiresPassword, expiresAt }`，让公开页决定要不要先问口令 |
| `POST /s/:token` | `{ password? }` 校验有效期/口令后返回内容：文件分享 → `{ file, downloadUrl }`；空间分享 → `{ space, files[] }`。`downloadUrl` 仅当 role 允许下载时非 null |

> 公开页失效/口令错是**业务失败**（`200 + error`：`SHARE_NOT_FOUND` / `SHARE_EXPIRED` / `SHARE_PASSWORD_REQUIRED` / `SHARE_PASSWORD_INVALID`），不是传输错。

### 8.5 存储配置 & Webhook（仅租户管理员）

| 方法 + 路径 | scope | 说明 |
| --- | --- | --- |
| `GET /storage` | `files.read`(admin) | → `StorageConfigDTO\|null`（SK 永不回显，只回 `secretKeySet`） |
| `PUT /storage` | `files.write`(admin) | 写存储配置，**保存即连通探测 + 自动发 CORS**（见 [§13](#13-租户存储桶配置管理控制面)） |
| `DELETE /storage` | `files.write`(admin) | 清除配置 |
| `GET /hooks` | `files.read`(admin) | 列上传 Webhook |
| `POST /hooks` | `files.write`(admin) | `{ url, secret?, active? }` |
| `PATCH /hooks/:id` | `files.write`(admin) | `{ url?, secret?, active? }` |
| `DELETE /hooks/:id` | `files.write`(admin) | 删 |

### 8.6 健康

| 方法 + 路径 | 说明 |
| --- | --- |
| `GET /health` | `{ service:"files-server", db, redis, time }` |

### 8.7 应用存储（FILE-STORAGE-2，见 [§16](#16-应用存储其他应用利用文件服务存储)）

应用目录（`kind=app`）的列举 + 目录级授权。actor 既可是用户态 TDT，也可是带 `azp` 的服务态 TDT。

| 方法 + 路径 | scope | 额外门 | 说明 |
| --- | --- | --- | --- |
| `GET /app-spaces` | `files.read` | actor 可见者 | 列应用目录（`SpaceDTO[]`，含 `ownerAppKey/fileCount/memberCount`）。管理员=全部；普通用户=被授权的；服务态=拥有 + 被授权的 |
| `GET /app-spaces/:id/grants` | `files.read` | 目录 `viewer`+ | 列目录授权（`SpaceMemberDTO[]`，含 `app` 主体） |
| `POST /app-spaces/:id/grants` | `files.write` | **租户管理员** | `{ principalType(user\|group\|app), principalId, role?(viewer\|editor) }`（upsert） |
| `DELETE /app-spaces/:id/grants/:principalType/:principalId` | `files.write` | **租户管理员** | 移除授权（路径必须带 `principalType`） |

> 应用目录的文件读写走 §8.1 同一套（`presign/finalize/list/get/download/patch/delete`）：传 `spaceId=<应用目录>` 即可；服务态 TDT 不传 `spaceId` 时**落自己 App 的应用目录**（而非个人空间）。

### 8.8 文件整理（FILE-ORGANIZE：目录树 · 两级标签 · 缩略图）

`goal/FILE-ORGANIZE.md` 交付的组织维度。**目录路径严格规范**：`/段/…/段/`（写入接受省略首尾 `/`,由服务端补齐）,段非空、禁 `\` `.` `..` 与控制字符、单段 ≤100、全长 ≤255,非法一律 `FOLDER_INVALID_PATH`;根 `/` 不可改名/删除。目录树仅覆盖 personal/team 空间（app 空间 → `SPACE_KIND_FORBIDDEN`）,且为**用户态**端点（服务态经 presign 写 folder 字符串的现状不变）。

| 方法 + 路径 | scope | 额外门 | 说明 / Body |
| --- | --- | --- | --- |
| `GET /folders?spaceId=` | `files.read` | 空间任意角色 | 一次拉全树：显式目录 ∪ 全部 `files.folder` 的祖先闭包 → `{ items: FolderNodeDTO[], truncated }`（隐式节点 `explicit:false`;逐节点带**直接**文件数;软上限 `FOLDER_TREE_MAX_NODES` 默认 5000,超限 `truncated:true`） |
| `POST /folders` | `files.write` | 空间 `editor` | `{ spaceId, path, recursive? }`（**recursive 默认 true**：一次补齐全部缺失祖先）→ `{ path, existed }`（幂等） |
| `PATCH /folders` | `files.write` | 空间 `editor` | `{ spaceId, path, name }` 同级重命名;子树（显式行 + `files.folder` 前缀）事务内跟随;目标已存在 → `FOLDER_EXISTS` |
| `DELETE /folders` | `files.write` | 空间 `editor` | `{ spaceId, path }`;**仅空目录**（无直接文件且无显式/隐式子路径）,否则 `FOLDER_NOT_EMPTY`（`details` 带 `directFiles/childFolders` 计数） |
| `GET /tags/categories` | `files.read` | 用户态 | 分类（sort 序）嵌套**可见**标签（共享 ∪ 自己的个人）;入口幂等确保系统默认分类（`isSystem:true`,初名「未分类」,可改名不可删） |
| `POST /tags/categories` | `files.write` | **租户管理员** | `{ name, sort? }`;重名 → `TAG_NAME_CONFLICT` |
| `PATCH /tags/categories/:id` | `files.write` | **租户管理员** | `{ name?, sort? }`（system 分类展示名可改） |
| `DELETE /tags/categories/:id` | `files.write` | **租户管理员** | system → `TAG_CATEGORY_SYSTEM_IMMUTABLE`;非空 → `TAG_CATEGORY_NOT_EMPTY` |
| `POST /tags` | `files.write` | `scope=tenant` → **管理员**;`personal` → 本人 | `{ categoryId, name, scope?(tenant), sort? }`;成员建共享 → `FORBIDDEN`;个人标签仅 owner 可见/可管 |
| `PATCH /tags/:id` · `DELETE /tags/:id` | `files.write` | 共享=管理员;个人=owner | 改名/排序/删除;删除级联 `file_tags` 并重建受影响文件的旧列/searchText;他人个人标签一律 `TAG_NOT_FOUND` |
| `GET /tags/cloud?spaceId?` | `files.read` | 用户态 | 可见标签 × 可见 active 文件的计数桶 → `TagCloudItemDTO[]`（未用标签 `count:0` 也返回;缺省=全部可见空间） |
| `GET /:id/tags` | `files.read` | 空间成员 | 该文件**可见**的 tagging（含固定 schema `meta` 原样） → `FileTagDTO[]` |
| `PUT /:id/tags` | `files.write` | 空间 `editor` | `{ items:[{ tagId, meta? }] }` 全量替换（≤24;tagId 须在可见集;`meta` 走固定 schema 校验,未知 key → `TAG_META_INVALID`） |

**tagging meta 固定 schema（`schemaVersion=1`,API-only,升级只增可选 key）**：`schemaVersion?`(≥1 整数) / `confidence?` `interest?` `match?`(有限 number ∈[0,1]：置信度/兴趣度/匹配度) / `source?`(`user|ai|import|system`) / `model?`(≤64) / `reason?`(≤200,无控制字符) / `scoredAt?`(ISO-8601)。序列化 ≤2KB;`{}` 合法（人工打标默认）。

**缩略图**：`kind:"thumbnail"` rendition（jpeg,宽 ≤320,内容寻址 key `_previews/<fileId>/thumbnail-<digest前12>.jpg`,存量无后缀 key 读行值原样兼容）。list `withThumbs=1` 对本页 eligible 项（`image/*` 精确清单 + `video/*`,size ≤200MB）**只入队**（进程内 FIFO + semaphore,`THUMB_CONVERT_CONCURRENCY` 默认 2,`THUMB_QUEUE_MAX` 默认 500,队列满纯背压不写 pending）,fresh-ready 项附 `thumbUrl`;`GET /:id/preview?kind=thumbnail` 为显式分支（native 图片不再短路原图）。converter（`PREVIEW_MEDIA_CONVERTER_URL`）未配置 ⇒ 不入队不报错,`thumbUrl` 恒 null。

**缩略图 URL 稳定性与缓存（FILE-ORGANIZE-2）**：缩略图 presign TTL 由 `THUMB_URL_TTL_S` 控制（默认 **172800**=48h,clamp [3600, 604800]——SigV4 上限 7 天）,且签名时刻**量化到 TTL/2 边界**（默认 24h 窗口）：同一窗口内所有 list/descriptor 返回**逐字节相同**的 URL（浏览器缓存键稳定）,任一 URL 剩余有效期恒 ∈ [TTL/2, TTL]。缩略图对象 PUT 时落 `Cache-Control: private, max-age=31536000, immutable`——内容变更（重传新 digest）→ 新 key → 新 URL,缓存天然失效。**仅缩略图走此策略**：download / preview 原文件 presign 维持 900s 不变。文件删除时其全部 rendition 对象同步从桶中删除（已签发的缩略图 URL 立即 404）。⚠️ 若租户桶策略含 `s3:signatureAge` 类条件（拒绝签名时刻过旧的请求）,置 `THUMB_URL_QUANTIZE=0` 降级为 fresh 签名（失去窗口内 URL 字节稳定,48h TTL 与内容寻址收益保留）。

SDK 速查：`sdk.files.folders.tree/create/rename/remove`、`sdk.files.tags.categories/createCategory/updateCategory/removeCategory/create/update/remove/cloud/forFile/putForFile`、`sdk.files.list({ folder, tagIds, withThumbs })`、`sdk.files.rename(id, { folder })`。

---

## 9. 权限模型：scope × 空间角色 × ACL

files 上「你能不能做某事」由**三层与门**共同决定，缺一不可：

1. **scope（你的 TDT 带了吗）** —— `files.read/write/share`，由清单声明 + consent 授予。粗粒度。
2. **空间角色（你在这个空间是什么角色）** —— `owner > editor > viewer`，files-server 自管（`space_members` 表）。**这是文件级真正的访问控制**：读要是成员、改/删要 `editor`、管空间/建空间分享要 `owner`。
3. **Portal ACL（可选 UX 门）** —— files 在清单里声明了 ACL Manifest（`apps/api/src/modules/acl/manifests.ts` 的 `FILES_MANIFEST`），页面 `files`/`spaces`、操作 `file.upload`/`file.delete`/`file.share`，用于在**前端**隐藏入口（`sdk.acl.can("files:action:file.delete")`）。**仅 UX，不是安全门**——真正拦截在 files-server 的空间角色判定。

```ts
// FILES_MANIFEST（节选，apps/api/src/modules/acl/manifests.ts）
{
  version: "1.0.0", landingPageKey: "files",
  pages: [
    { key: "files",  path: "/",       label: { "zh-CN": "文件" },     supportedScopes: ["own","all"], defaultForMember: true },
    { key: "spaces", path: "/spaces", label: { "zh-CN": "协作空间" }, navItemId: "nav-spaces",        defaultForMember: true },
  ],
  actions: [
    { key: "file.upload", pageKey: "files", defaultForMember: true },
    { key: "file.delete", pageKey: "files", dangerous: true, supportedScopes: ["own","all"] },
    { key: "file.share",  pageKey: "files" },
  ],
}
```

> 结构性管理操作（存储配置、Webhook）看的是 **Portal 租户角色 `admin`**（来自自省结果的 `role`，非空间角色，`requireAdmin`）。普通成员调 `/storage`、`/hooks` 会吃 `403 FORBIDDEN`。

---

## 10. 内容去重（密纹 / fingerprint）

每个文件 `finalize` 时落一个 **SHA-256 digest（密纹）**（浏览器侧用 `sdk.files.digest(bytes)` 算，或你自己算后传给 `finalize`）。下游 App 据此判重 / 秒传：

```ts
// 你手上有一批 fileId，想知道它们的内容密纹（用于去重 / 比对）
const { items, missing } = await sdk.files.fingerprint([id1, id2, id3]);
// items: [{ key: fileId, digest, size, name }]  missing: 查无/无密纹/不可见的 id
```

REST 等价 `POST /api/v1/files/fingerprint { keys: fileId[] }`（scope `files.read`，只在你可见的空间内解析）。密纹也在 `FileDTO.digest` 直接带出。典型用法：上传前先算 digest，库里查有没有同 digest 的文件 → 命中就跳过上传直接引用（秒传）。

---

## 11. 安全分享与公开访问

**创建分享**（[§8.3](#83-安全分享鉴权侧)）后，把 `${filesAppOrigin}/s/${share.token}` 给接收方。接收方打开公开页：

```
访问 /s/:token ──► GET /s/:token  → { requiresPassword, expiresAt, role, kind }
                       │
              requiresPassword? ── 是 ──► 让用户输口令
                       │
              POST /s/:token { password } ──► 校验有效期 + 口令
                       │
        文件分享 → { file, downloadUrl }    空间分享 → { space, files[] }
```

- 分享可带**角色**（仅看 / 可下载 / 可编辑）、**有效期**（`expiresAt`）、**口令**（`password`，落库为 hash）。
- 公开访问**完全不需要 TDT**——分享给租户外的人也能用。
- 吊销是软删（`revoked=true`），立即对公开页失效。
- files-app 自带的公开页实现可参考 `apps/files-app/src/PublicAccess.tsx`（你的 App 通常不必自己做公开页，直接复用 files 的链接即可）。

---

## 12. 上传后处理：Webhook（file.uploaded）

你的 App 想在用户上传文件后**做点什么**（解析、转码、入你自己的库、建索引），有两条路：

- **配 Webhook**（推荐给独立后端）：租户管理员（或你的管理控制面）`POST /api/v1/files/hooks { url, secret?, active? }`。每次 `finalize` 成功，files-server **best-effort** POST 到你的 `url`：

  ```jsonc
  // body（4s 超时，失败只记日志不重试、不阻断上传）
  {
    "event": "file.uploaded",
    "payload": { "fileId", "objectKey", "digest", "name", "size", "spaceId", "tenantId", "userId" },
    "at": "2026-06-16T…Z"
  }
  ```
  配了 `secret` 则带 `x-files-signature: <HMAC-SHA256(secret, body)>`，你的端点务必验签。
- **前端串行**（iframe 场景）：`sdk.files.upload(...)` resolve 后你已拿到 `FileDTO`，直接接着做后续调用即可。

> 内置的 `name+tags → searchText` 全文索引在 `finalize` 里**总会跑**；Webhook 是**额外**的消费者，不替代它。

---

## 13. 租户存储桶配置（管理控制面）

> 仅当你的 App **就是**「文件后端」、要在 Portal 应用配置页让管理员填存储桶时才需要。普通消费 files 的 App 用不到——存储桶是 files 自己的配置项。这里记录其机制，便于你照搬到自己的独立后端配置面（呼应 [SSO §13.7](SSO与App开发指引.md#137-应用配置app-config不要在应用内自建设置入口)）。

模式（files 就是参考实现）：

1. files-server 暴露租户配置读写端点 `GET/PUT/DELETE /api/v1/files/storage`，仍按 TDT 四道闸 + `requireAdmin` 鉴权。
2. 平台为它开了一个**仅管理员的短时 admin-TDT 铸造接口**：

   ```
   GET /api/admin/apps/:appKey/files-token   （会话态，assertAdmin）
   → { token, expiresIn: 600, filesBase }
   ```
   它铸一把 `aud=appKey`、scope = `["files.read","files.write","files.share"] ∩ 该应用声明`、TTL 600s 的 TDT；**仅声明了 files 作用域的应用可用**（否则 `FORBIDDEN`）。不走 consent 门（属管理配置）。
3. 前端用这把 admin-TDT **浏览器直连** files-server 改存储——**门户永不经手 S3 凭证**，SK 直达 files-server、AES-256-GCM 加密落库。

**存储 provider 画像**（`apps/files-server/src/lib/providers.ts`，纯函数）：

| provider | endpoint | forcePathStyle | 校验 |
| --- | --- | --- | --- |
| `s3` | 显式填（通用 S3 兼容 / MinIO） | 默认 `true`（路径风格） | 无额外（dev 桶不存在会自动建） |
| `aws` | 缺省按 region 推导（vhost 风格） | 强制 `false` | `region` 必填；桶须云上预建 |
| `cos` | 缺省按 region 推导 | 强制 `false` | `region` 必填；bucket 须带 APPID 后缀（如 `media-1250000000`） |

`PUT /storage` 保存时会**实跑连通探测**（`HeadBucket`，按错误给精准提示：301 区域不符 / 403 密钥错 / 404 桶不存在）并**自动发布 CORS**。切换 provider 后必须重填 SK（旧 SK 属上一个 provider）。

---

## 14. 错误码目录（files 专属）

业务失败统一 `200 + error.code`；只有传输/认证/路由/限流走 HTTP 非 200。除 [SSO 附录 B](SSO与App开发指引.md#附录-b错误码目录) 的通用码外，files 还会返回：

| code | HTTP | 场景 |
| --- | --- | --- |
| `UNAUTHENTICATED` | 401 | 缺 Bearer TDT |
| `INVALID_TOKEN` | 401 | 自省 `active:false`，或**用户态** TDT `aud/listingKey != files`（**最常见**：拿了别的 App 的 TDT 直连 files），或缺 tenant。**服务态 TDT 跳过 aud 校验**，只认 scope（[§16](#16-应用存储其他应用利用文件服务存储)） |
| `INSUFFICIENT_SCOPE` | 403 | TDT 没有所需的 `files.read/write/share` |
| `FORBIDDEN` | 403 | 非租户管理员调存储/Webhook/应用目录授权；或**服务态 TDT 缺 `azp`**（未绑定 App）想进应用存储；或纯用户上下文接口（分享）被服务态调用 |
| `RATE_LIMITED` | 429 | 触发 files-server 自己的 `(aud,tenant)` 600/min 限流 |
| `INTROSPECT_FAILED` | 200 | files-server 连不上门户自省端点 / 返回异常 |
| `VALIDATION_FAILED` | 200 | 缺 `name`/`fileId`/`keys`/`principalId`/`name`；无效 `expiresAt`；存储缺 `endpoint/bucket/accessKeyId`；切 provider 缺 SK；COS bucket 缺 APPID 后缀 |
| `FILE_NOT_FOUND` | 200 | 文件不存在（或不在本租户） |
| `SPACE_NOT_FOUND` | 200 | 空间不存在 |
| `SPACE_FORBIDDEN` | 200 | 空间角色不足（读非成员 / 改删非 editor / 管理非 owner / finalize 非本人 / 移除最后一名 owner / 个人空间改删 / 服务态碰个人·团队空间 / 未授权访问应用目录） |
| `PROCESSOR_APP_FILE_UNSUPPORTED` | 200 | 对 `kind=app` 应用目录文件运行处理器（本期处理器仅服务用户空间文件，[§16](#16-应用存储其他应用利用文件服务存储)） |
| `SHARE_NOT_FOUND` | 200 | 分享不存在或已吊销 |
| `SHARE_EXPIRED` | 200 | 分享过期 |
| `SHARE_PASSWORD_REQUIRED` / `SHARE_PASSWORD_INVALID` | 200 | 公开页缺口令 / 口令错 |
| `STORAGE_NOT_CONFIGURED` | 200 | 该租户还没配存储桶（presign/download/finalize 前置） |
| `STORAGE_UNREACHABLE` | 200 | 保存配置时连通探测失败（区域不符 / 密钥错 / 桶不存在 / 建桶失败） |
| `FOLDER_INVALID_PATH` | 200 | 目录路径不合规（空段/`\`/`.`/`..`/控制字符/超长；presign folder、PATCH folder、目录端点共用一套严格校验） |
| `FOLDER_ROOT_IMMUTABLE` | 200 | 试图改名/删除根目录 `/` |
| `FOLDER_EXISTS` | 200 | 重命名目标目录已存在（含隐式子树） |
| `FOLDER_NOT_EMPTY` | 200 | 删除非空目录（`details.directFiles/childFolders` 带计数） |
| `FOLDER_PARENT_MISSING` | 200 | `recursive:false` 建目录且父路径不在树中 |
| `SPACE_KIND_FORBIDDEN` | 200 | 对 `kind=app` 应用目录调目录树端点（目录树仅 personal/team） |
| `TAG_CATEGORY_NOT_FOUND` / `TAG_NOT_FOUND` | 200 | 分类/标签不存在（含他租户与**他人个人标签**——不泄露存在性） |
| `TAG_CATEGORY_SYSTEM_IMMUTABLE` | 200 | 删除系统默认分类（isSystem） |
| `TAG_CATEGORY_NOT_EMPTY` | 200 | 删除仍有标签的分类 |
| `TAG_NAME_CONFLICT` | 200 | 分类/标签重名（同租户分类唯一；同分类+owner 下标签唯一） |
| `TAG_META_INVALID` | 200 | tagging meta 不符合固定 schema（未知 key / 分数越界 / 非 object / >2KB） |

> 自省结果在 files-server 缓存 ≤60s，**吊销/改 scope 可能有秒级延迟**。

---

## 15. 本地开发与联调

### 15.1 起服务

files 由 `bun run dev:all` 一把起齐（**不要单独叠跑 `dev:files`**，会抢端口导致 iframe 串台，详见 [`CLAUDE.md`](../CLAUDE.md) 本地开发约定）：

| 服务 | 端口 | 库 / 依赖 |
| --- | --- | --- |
| `files-server` | `4100` | `FILES_DATABASE_URL = xgent-files`；需 MinIO（:9000） |
| `files-app` | `5303` | iframe 前端（也是 `sdk.files.*` 的活样例） |

依赖 files 的微应用各占自己的端口（如 `knowledge-app` :5309）。

### 15.2 初始化

```bash
bun install
bun run db:files:generate && bun run db:files:migrate    # files 自己的迁移链
bun run files:bootstrap                                   # 把晨光租户存储指向本地 MinIO + 建演示团队空间
bun run dev:all
```

`files:bootstrap`（`apps/files-server/scripts/bootstrap.ts`）会用 rockie 的 admin-TDT 把「晨光」租户存储配到本地 MinIO（建桶 + 发 CORS），并建一个「市场部协作」团队空间、把李明加为 editor——开箱即有协作演示数据。**没配存储**的租户，任何上传/下载都会 `STORAGE_NOT_CONFIGURED`。

### 15.3 相关环境变量（`.env`）

```bash
FILES_APP_URL=http://localhost:5303              # CORS 白名单 + 探测源
FILES_DATABASE_URL=postgres://…/xgent-files      # files 独立库
FILES_SERVER_PORT=4100
FILES_AUDIENCE=files                             # files-server 接受的 TDT aud
FILES_ENC_KEY=…                                  # AES-256-GCM 密钥（加密租户 SK）
PORTAL_INTROSPECT_URL=http://localhost:3000/api/tokens/introspect
FILES_SA_CLIENT_ID=files-server                  # files-server 调自省的服务账号
FILES_SA_CLIENT_SECRET=…                         # 优先；缺省回退 FILES_RESOURCE_KEY
FILES_RESOURCE_KEY=files-dev-resource-key        # dev x-resource-key 通道（仅 DEV_MOCK_OAUTH=true）
# 本地 MinIO（仅 bootstrap 用来种一份 dev 存储配置）
MINIO_ENDPOINT=http://localhost:9000  MINIO_ACCESS_KEY=minioadmin  MINIO_SECRET_KEY=minioadmin  MINIO_BUCKET=xgent-files
```

> 生产 `DEV_MOCK_OAUTH=false`：files-server 调自省**必须**走服务账号 Basic（`FILES_SA_CLIENT_ID:FILES_SA_CLIENT_SECRET`），`x-resource-key` 一律被门户拒绝。该服务账号要有 `token.introspect` 能力（平台控制台 `/api/console/service-accounts` 创建）。

### 15.4 回归脚本（`apps/files-server/scripts/`，`bun --env-file=../../.env run scripts/<x>.ts`）

| 脚本 | 覆盖 |
| --- | --- |
| `verify-upload.ts` | presign → 直传 → finalize 全链 |
| `verify-spaces.ts` | 空间 / 成员 / 角色 / continuity |
| `verify-share.ts` | 分享角色 / 有效期 / 口令 / 公开访问 |
| `verify-search.ts` | `q/tags/type/owner/sort/分页` 检索 |
| `verify-fingerprint.ts` | 批量密纹去重 |
| `verify-hooks.ts` | 上传 Webhook + HMAC |
| `verify-storage.ts` | 存储配置 / provider 画像 / 探测 |
| `verify-exchange-files.ts` | **独立后端令牌交换调 files**（方式二的权威用例） |
| `verify-app-storage.ts` | **应用存储**（[§16](#16-应用存储其他应用利用文件服务存储)）：服务态写应用目录 + 目录级 ACL（user/group/app）+ `viaApp` 来源 + `via` 过滤 + 越权回归 |
| `verify-folders.ts` | **目录树**（[§8.8](#88-文件整理file-organize目录树--两级标签--缩略图)）：严格路径矩阵 / 递归 mkdir / 祖先闭包 / 子树重命名 / 空目录闸 / folder 过滤 / 权限与租户矩阵 |
| `verify-tags.ts` | **标签词典**：system 默认分类 / 共享标签 admin 闸 / 个人标签隐私（DTO·cloud·searchText·旧列）/ 字符串 tags 兼容 / meta schema 矩阵 / `?tags=` 只读回归 |
| `verify-thumbnails.ts` | **缩略图**：入队-锁序 / 有界并发 / 队列背压 / 超限不入队 / `?kind=thumbnail` 显式分支 / converter 未配置降级 / **FILE-ORGANIZE-2 稳定 URL**（`X-Amz-Expires`=TTL·量化边界·同窗口逐字节相同·`Cache-Control` immutable·内容寻址 key·换代删旧对象·删除断链·download/preview 900s 回归）（真转换需 `FILES_BASE` 指向 `PREVIEW_MEDIA_CONVERTER_URL=auto` 实例） |
| `verify-organize.ts` | **整理串联入口**（root `files:verify:organize`；`verify:all` 挂此行）：via 组合守护 + 上述三套件 |

对接方式二时，**先把 `verify-exchange-files.ts` 跑通**，再把里面的 `mistakes` 换成你的 `listingKey`。

---

## 16. 应用存储（其他应用利用文件服务存储）

> 文件服务的目录原本**全是用户视角**（个人 / 团队空间，文件必属某用户）。本章是**第二类视角**：其他 App 借文件服务存内容。两条链路，区别在**归属**：
>
> | 场景 | 谁存的 | 归属 | 默认谁能看 |
> | --- | --- | --- | --- |
> | **A · 应用存储** | App 用**服务账号**（M2M，无用户上下文）写入 | **应用所有**（`kind=app` 目录，`ownerUserId=null`） | **仅租户管理员**；可按目录授权给用户/用户组/其他应用 |
> | **B · 用户存储 + 来源标记** | App 用**用户 TDT**（令牌交换，代表某用户）写入 | **仍属该用户**（落个人/团队空间） | 该用户本人；文件管理「应用存储 / 我的应用文件」按来源 App 聚合呈现 |
>
> 两场景共用文件的**来源标记 `viaAppKey`**（这文件由哪个 App 经手）。可信来源 = 令牌的 `azp` 声明（= 源/拥有 App 的 **listingKey**，与 introspection 的 `listingKey` 同口径）。

### 16.1 `azp`：经手/拥有 App 的稳定身份

- **令牌交换签发**（场景 B）：交换出 `aud=files` 的用户态 TDT 时，门户自动写 `azp = sourceApp.contentOwner ?? sourceApp.appKey`。独立后端零额外代码即带上。
- **服务态签发**（场景 A）：`client_credentials` 自签时写 `azp = 服务账号的 ownerAppKey`。
- introspection 透出 `azp`；files-server 据此落 `viaAppKey` / 判定应用目录归属。
- `viaAppKey`、应用目录 `ownerAppKey`、目录授权的 `app` 主体**三者同一 key 空间（listingKey）**，不混 raw appKey。

### 16.2 场景 A：服务账号写应用存储

**前置（平台管理员，一次性）**：在控制台「服务账号」给该 App 建一个服务账号 —— 能力勾 `client_credentials`、scope 给 `files.read files.write`、**「归属应用」填该 App 的 listingKey**（= `ownerAppKey`）。签发服务态令牌时门户会校验：该 listingKey 的 App 在**请求租户**里已安装且 `active`，否则拒签（`APP_NOT_INSTALLED` / `APP_DISABLED`）—— 防止为「该租户没装的 App」凭空造应用目录。

**写入链路**（App 的独立后端，无用户上下文）：

```text
① 自签服务态 TDT（PLAN-4 §3.5）
   POST {PORTAL}/api/tokens/service
     grant_type=client_credentials  Authorization: Basic <clientId:secret>
     tenant_id=<租户>  scope=files.read files.write
   → { access_token }   # data.access_token；该令牌带 azp=<本SA绑定的App>
② 直连 files-server（与用户态同一套 REST）
   POST {FILES_BASE}/api/v1/files/presign   Authorization: Bearer <服务态TDT>
     { name, contentType, size }            # 不传 spaceId → 落本 App 的应用目录（懒建）
   → PresignResult
③ PUT 直传桶 → ④ POST /finalize
```

- 网关对**服务态**令牌跳过 `aud===files`，只校验 `files.*` scope。服务态**必须带 `azp`**，否则一律 `FORBIDDEN`（不回退 clientId）。
- 文件落 `kind=app` 目录：`ownerUserId=null`、`viaAppKey=ownerAppKey=azp`。
- 服务态令牌**严格圈在应用存储内**：不传 `spaceId` 落自己 App 目录；传 `spaceId` 必须是它有 `editor` 的应用目录；**绝不**碰个人/团队空间（碰则 `SPACE_FORBIDDEN`）。

**目录级授权**（[§8.7](#87-应用存储file-storage-2见-16)）：默认应用目录只有拥有 App + 租户管理员可见。管理员可在文件管理「应用存储 → 某目录 → 授权」把目录授予：
- **用户 / 用户组**：`POST /app-spaces/:id/grants { principalType:"user"|"group", principalId, role }`（`group` 主体靠 introspection 回传的 `groups` 解析）；
- **其他应用**：`{ principalType:"app", principalId:<App listingKey>, role }` —— 该 App 的服务账号即可访问。

管理员判定统一用成员表 **`role==="admin"`**（非 ACL `bypass`）。

### 16.3 场景 B：用户存储 + 来源标记

与今日上传链路**完全一致**（presign→直传→finalize，归属仍是该用户、仍校验空间角色），唯一差异是落 `viaAppKey`：

- 可信来源 = 交换令牌的 `azp`（= 源 App listingKey）。
- 令牌缺 `azp` 时才用 `presign` 显式传 `{ viaApp }` 兜底 —— **视为不可信的展示分组**，仅决定 UI 归类、**绝不参与访问控制**。

**列表 `via` 过滤**（`GET /api/v1/files?via=…`，缺省=全部，向后兼容）：

| `via` | 含义 |
| --- | --- |
| `self` | `viaAppKey IS NULL`（个人/团队视图，隐藏应用经手文件） |
| `app` | 「我的应用文件」：`viaAppKey IS NOT NULL` **且 `ownerUserId = 当前用户`** |
| `<appKey>` | 该来源 App 的文件（用户态同样附 `ownerUserId` 约束） |

> ⚠ `via=app/<appKey>` 对用户态**必须**叠加 `ownerUserId = 我`：可见空间含团队空间，否则会把团队空间内**他人**经 App 创建的文件泄露进「我的应用文件」（越权）。`verify-app-storage.ts` 有该回归。

### 16.4 周边与边界

- **preview**：被授权用户/管理员可预览应用目录文件（访问判定走统一 `resolveActorRole`）。
- **processors**：对 `kind=app` 应用目录文件运行处理器明确报 `PROCESSOR_APP_FILE_UNSUPPORTED`（派生写回应用目录出本期）。
- **upload Webhook**：应用目录 finalize 同样触发 `file.uploaded`；payload `userId` 可空，并带 `viaAppKey` 与（应用文件时的）`ownerAppKey`。
- **不在本期**：子文件夹粒度 ACL；授权给「平台 ACL 自定义角色」；processor 派生写回应用目录；应用存储配额/计费；用户文件↔应用文件互转；服务态令牌交换（M2M→M2M）。

---

## 附录 A：类型参考（DTO）

权威定义见 `packages/shared/src/dto.ts` 与 `constants.ts`；SDK 侧同名导出在 `packages/portal-sdk/src/index.ts`。

```ts
type SpaceKind = "personal" | "team" | "app";          // app = 应用存储（FILE-STORAGE-2）
type SpaceRole = "owner" | "editor" | "viewer";        // SPACE_ROLES（owner>editor>viewer）
type ShareRole = "viewer" | "downloader" | "editor";   // SHARE_ROLES
type StorageProvider = "s3" | "aws" | "cos";
const FILE_LIST_MAX_LIMIT = 200;
const FILES_DIGEST_ALGO = "SHA-256";

interface FileDTO {
  id: string; spaceId: string; name: string; folder: string;
  size: number; contentType: string; digest: string | null;   // SHA-256 密纹；pending 时 null
  ownerUserId: string | null;                                  // 应用存储文件为 null
  viaAppKey: string | null;                                    // 经手 App（listingKey）；两场景共用
  tags: string[]; status: "pending" | "active";                // tags = 当前 actor 可见的标签名（FILE-ORGANIZE 词典组装）
  thumbUrl?: string | null;                                    // 仅 withThumbs=1 列表响应携带（长效稳定预签名缩略图,§8.8）
  createdAt: string; updatedAt: string;                        // ISO
}
// —— 文件整理（FILE-ORGANIZE §8.8）——
interface FolderNodeDTO { path: string; name: string; explicit: boolean; fileCount: number; }   // fileCount = 直接文件数
interface FolderTreeDTO { items: FolderNodeDTO[]; truncated: boolean; }
interface FileTagMeta { schemaVersion?: number; confidence?: number; interest?: number; match?: number; source?: "user"|"ai"|"import"|"system"; model?: string; reason?: string; scoredAt?: string; }  // 固定 schema,未知 key 拒绝
interface TagCategoryDTO { id: string; name: string; sort: number; isSystem: boolean; tags?: TagDTO[]; createdAt: string; }
interface TagDTO { id: string; categoryId: string; name: string; scope: "tenant"|"personal"; ownerUserId: string | null; sort: number; createdAt: string; }
interface FileTagDTO { tagId: string; name: string; categoryId: string; categoryName: string; scope: "tenant"|"personal"; meta: FileTagMeta; }
interface TagCloudItemDTO { tagId: string; name: string; categoryId: string; categoryName: string; scope: "tenant"|"personal"; count: number; }
interface PresignResult { fileId: string; objectKey: string; uploadUrl: string; headers: Record<string,string>; }
interface FileFingerprintDTO { key: string; digest: string; size: number; name: string; }   // key = fileId
interface SpaceDTO { id: string; name: string; kind: SpaceKind; ownerUserId: string | null; ownerAppKey?: string | null; memberRole: SpaceRole | null; fileCount?: number; memberCount?: number; createdAt: string; }
interface SpaceMemberDTO { principalType: "user" | "group" | "app"; principalId: string; name: string | null; role: SpaceRole; }
interface ShareDTO { id: string; token: string; fileId: string | null; spaceId: string | null; role: ShareRole; expiresAt: string | null; hasPassword: boolean; revoked: boolean; createdAt: string; }
interface FileHookDTO { id: string; event: "file.uploaded"; url: string; active: boolean; hasSecret: boolean; createdAt: string; }
interface StorageConfigDTO { provider: StorageProvider; endpoint: string; region: string; bucket: string; accessKeyId: string; secretKeySet: boolean; forcePathStyle: boolean; prefix: string; updatedAt: string | null; }
```

---

## 附录 B：关键文件索引

| 关注点 | 文件 |
| --- | --- |
| 路由挂载 / CORS / health | `apps/files-server/src/index.ts` |
| 网关（自省 + scope + 服务态放行 + `requireActor`/`userActor`） | `apps/files-server/src/lib/gate.ts` |
| 文件 CRUD / 检索 / 密纹 / `viaAppKey` / `via` 过滤 | `apps/files-server/src/modules/files.ts` |
| 空间 / 成员 / 角色 / 应用目录 + 授权（`resolveActorRole`） | `apps/files-server/src/modules/spaces.ts` |
| 分享（鉴权侧） | `apps/files-server/src/modules/shares.ts` |
| 公开访问（无 TDT） | `apps/files-server/src/modules/public.ts` |
| 存储配置 | `apps/files-server/src/modules/storage.ts` |
| 上传 Webhook | `apps/files-server/src/modules/hooks.ts` |
| S3 预签名 / CORS / provider 画像 | `apps/files-server/src/lib/s3.ts`、`lib/providers.ts` |
| 库 schema | `apps/files-server/src/db/schema.ts` |
| 令牌交换调 files（权威用例） | `apps/files-server/scripts/verify-exchange-files.ts` |
| 应用存储回归（场景 A/B + 目录 ACL + 越权） | `apps/files-server/scripts/verify-app-storage.ts` |
| 文件整理：路径校验 / 词典 / tagging 写路径（`replaceFileTags` 等唯一函数群） | `apps/files-server/src/lib/organize.ts` |
| 文件整理：目录树 / 词典端点 | `apps/files-server/src/modules/{folders,tags}.ts` |
| 缩略图队列（ensure 只入队 · worker 内占锁） | `apps/files-server/src/lib/thumbnails.ts` |
| 文件整理前端（目录树 / 标签页 / 打标签） | `apps/files-app/src/components/{FolderTree,TagsView,TagPickerDialog}.tsx` |
| `azp` 令牌声明 / 服务态 SA→App 绑定校验 | `apps/api/src/lib/tdt.ts`、`modules/token/{service,index}.ts` |
| 应用存储前端（分类 / 目录 / 授权面板） | `apps/files-app/src/components/{AppStorageView,AppGrantsDialog}.tsx` |
| `sdk.files.*`（含 `appSpaces.*` + `list({via})`） 实现 | `packages/portal-sdk/src/index.ts` |
| files 市场清单 / 安装快照 | `apps/api/src/db/seed.ts`（`listingKey: "files"`） |
| files ACL Manifest | `apps/api/src/modules/acl/manifests.ts`（`FILES_MANIFEST`） |
| admin-TDT 铸造（存储配置直连） | `apps/api/src/modules/apps/index.ts`（`GET /api/admin/apps/:appKey/files-token`） |
| host-proxy（`callService` 转发） | `apps/web/src/pages/MicroAppHost.tsx` |
| iframe 前端活样例 | `apps/files-app/src/App.tsx`、`main.tsx`、`PublicAccess.tsx` |

---

*本文档随实现演进。若代码与文档不符，以 [`apps/files-server/src/`](../apps/files-server/src/) 与 [`packages/`](../packages/) 的实现为准。*
