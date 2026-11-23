# Files 文件服务接入（主 API）

App 要在门户的文件服务里上传、列出、下载、替换文件，或把自己的内容存进应用空间时读取。
不过期的公开 URL 见 [长期持链接](files-stable-links.md)；服务账号代成员保存见 [代成员保存](files-member-docs.md)；
「这份字节归谁、用哪种身份写」先查 [平台能力 · 文件与对象存储](platform-capabilities.md#文件与对象存储两条接入路径)。

## 1. 选接入路径

| 谁在调 | 令牌 | 调用方式 |
| --- | --- | --- |
| 你的 micro 前端，代表当前成员 | 宿主自动换出的 `aud=files` 用户态票 | `sdk.files.*`（即 `callService("files", …)`，零跨域） |
| 你的后端，代表某个成员 | 用该成员的入站 TDT 做令牌交换，`audience=files` | REST，机制与排错见 [跨应用交换](app-exchange.md) |
| 你的后端，代表 App 自己（无用户） | 本 App 服务账号的服务态 TDT（`POST /api/tokens/service`） | REST，只能进本 App 的应用空间 |

REST 基址：去掉末尾 `/` 的 `PORTAL_BASE_URL` + `/svc/files/api/v1/files`，带 `Authorization: Bearer <TDT>`。
所有业务结果是 `{ ok, data | error }` 信封，业务失败也是 HTTP 200，必须检查 `ok`。

- 用户态票的 `aud` 必须是 `files`。拿你自己 App 的 TDT 直连 files 会得到 401 `INVALID_TOKEN`，这是最常见的接错。
- 服务态票不校验 aud，只看 scope，但必须带 `azp`（manifest 注册的服务账号恒绑本 App）。服务态票碰不到个人/团队空间。
- 后台异步任务不要靠用户票续命：交换不发 refresh_token。提交时用用户票核权限、取输入，之后的产物用服务账号写进应用空间。

## 2. 清单前置

```json
{
  "dependencies": ["files"],
  "exchangeTargets": ["files"],
  "scopes": ["files.read", "files.write"],
  "serviceAccount": { "clientId": "your-app-server" },
  "serviceScopes": ["files.read", "files.write"]
}
```

- `dependencies`：安装你的 App 时先装 files；files 被卸载前会被你的依赖拦住。
- 用户路径（SDK 或后端交换）：`exchangeTargets` 含 `files`，且 `scopes` 声明所需的 `files.*`。漏声明时 iframe 里 `sdk.files.*` 全部失败，不是 CORS 问题。
  `files.read` 读与列，`files.write` 上传/改名/删除/替换，`files.share` 只在要建分享链接时声明。scope 越少，授权屏越容易通过。
- 服务账号路径：`serviceScopes` 写 `files.read` / `files.write`（要求 `files` 在 `dependencies` 或 `exchangeTargets` 里），且须声明 `serviceAccount`。
  租户没装 files 时签出的服务态票不含 `files.*`；SA 未获授权时签发不报错，只返回更窄的 scope。用票前检查返回的 `scope`。
- 平台托管前端（`/apps/<key>/`）默认 `connect-src` 只有 `'self'` 与平台 CDN，不含对象存储源；图片/媒体可显示预签名 GET，
  但浏览器直传 PUT 要在 `embedCsp.connectSrc` 写目标环境存储的精确 HTTPS origin（禁止通配）。一盒把存储代理到门户同源，
  所以一盒能传、生产被 CSP 拦是这条漏声明。租户自配桶的源无法预先枚举，必须支持时改由后端走 `POST /content`。

## 3. 上传

### 3.1 presign → PUT → finalize（字节直传存储，不经 files）

```text
POST /presign   { name, contentType, size, spaceId?, folder?, onBehalfOf? }
  → { fileId, objectKey, uploadUrl, headers }          # 建 pending 行
PUT  uploadUrl  只带返回的 headers（Content-Type 参与签名），body = 字节
POST /finalize  { fileId, digest, size?, tags? }  → FileDTO（status=active）
```

- 不传 `spaceId`：用户态落本人个人空间，服务态落本 App 应用空间。传了要求对该空间有 editor。`folder` 形如 `/media/`。
- PUT 签名 900 秒，限制的是请求开始时间；pending 行保留 24 小时，超时自动清理并释放预占配额，之后 finalize 返回 `FILE_NOT_FOUND`。
- PUT 确认失败时 `DELETE /<fileId>` 提前释放；finalize 响应丢失时先重试 finalize，不要删，可能已发布。
- finalize 以存储里对象的实际大小为准，忽略客户端 `size`；`digest` 是调用方申报值，服务端不重算。需要可信摘要用 3.2。
- 发布时对象被复制到只有服务端可写的新 key，之后再向旧 `uploadUrl` PUT 不会改变已发布内容。
- 权限：用户文件只有上传者本人能 finalize；应用空间文件要求调用方对它有 editor。
- `tags` 是标签名数组，按租户标签字典查找或新建。finalize 会触发租户配置的上传 Webhook。

### 3.2 `POST /content`（后端代传，字节经过 files）

query：`name`（必填）、`spaceId?`、`folder?`、`onBehalfOf?`；请求体就是原始字节，`Content-Type` 头即文件类型。
一次请求直接落为 active，服务端计算 size 与 SHA-256，不留 pending 行。上限默认 200 MiB，超出 413 `PAYLOAD_TOO_LARGE` 并关闭连接。
可带 `Idempotency-Key`（1–128 个可见 ASCII）：同键同请求重放返回原 FileDTO，同键不同内容报 `IDEMPOTENCY_CONFLICT`。

### 3.3 新版本与原位覆盖

| 需求 | 接口 | 要点 |
| --- | --- | --- |
| 上传新版本（产生新 fileId） | `POST /<id>/replace/presign { uploadName, contentType, size, name? }` → PUT → `POST /<newId>/replace/finalize { digest, size? }` | 扩展名锁定为 `uploadName` 的扩展名，`name` 只改主名；对历史版本发起报 `REPLACE_SUPERSEDED`（`details.headId` 给链头） |
| 同一 fileId 原位覆盖 | `PUT /<id>/content`，头 `If-Match: <revision>`，体为新字节 | 仅限应用空间的所属 App（服务态）；revision 不符报 `REVISION_CONFLICT`（`details.currentRevision`）；名称与类型不变，不触发 Webhook |

旧版本保持可下载、可预览，原分享继续指向原版本。删除链头会把整条链置为墓碑。

## 4. `sdk.files.*` 速查（`@xgent/portal-sdk` 0.5.9 起）

```ts
const meta = await sdk.files.upload(file, { spaceId, name, tags });   // presign → PUT → finalize，无进度
// PUT 失败时 SDK 尽力 remove 并在错误上附 fileId；要进度条或指定 folder 就用低层三步
const pre = await sdk.files.presign({ name, contentType, size, spaceId, folder });
await sdk.files.finalize({ fileId: pre.fileId, digest: await sdk.files.digest(file), size });

await sdk.files.list({ spaceId, q, tags, tagIds, type, owner, via, folder, withThumbs, sort: "-createdAt", limit, offset }); // { items, total }，limit ≤ 200
await sdk.files.get(id);
const { url } = await sdk.files.download(id);   // 预签名 GET，900 秒；用 openDownload(url)
await sdk.files.rename(id, { name, tags, folder });  await sdk.files.remove(id);
await sdk.files.replace(fileId, file, { name });  await sdk.files.latest(fileId);  await sdk.files.versions(fileId, { limit, offset });
await sdk.files.tags.putForFile(fileId, [{ tagId, meta }]);   // 第二参是数组；整份替换，每文件 ≤ 24 个
await sdk.files.fingerprint([fileId1, fileId2]);              // { items: [{ key, digest, size, name }], missing }
await sdk.files.spaces.list();  await sdk.files.appSpaces.list();
const share = await sdk.files.shares.forFile(fileId, { role: "downloader", openMode: "preview", expiresAt, password });
```

- `list` 不带 `spaceId` 时，用户态只列本人个人空间和所属团队空间；`via=self` 排除应用经手文件，`via=app` 是「我的应用文件」，`via=<appKey>` 只看该 App 经手的。`folder` 过滤必须同时带 `spaceId` 或 `via`。
- 下载链接是跨源预签名 URL，用 SDK 的 `openDownload(url)`，原因见 [SDK 参考 §5.1](sdk-reference.md#51-下载文件只用-opendownload别自己写-a-hrefclick)。服务端按 fileId 缓存下载 URL（900 秒内复用）可省限流额度。
- 分享需要用户上下文与 `files.share`，服务态调用报 403 `FORBIDDEN`。`role` 为 `viewer`/`downloader`/`editor`。
  链接按 `openMode` 拼：`download`（缺省）→ `<门户源>/s/<token>`；`preview` 与 `preview-only` → `<门户源>/p/<token>`。
- `storage.*`、`hooks.*` 是租户管理员的存储与 Webhook 配置面，普通 App 不需要调用。

## 5. 权限：scope × 空间角色 × ACL

三层同时满足才放行：

1. **scope**：令牌带 `files.read` / `files.write` / `files.share`，缺则 403 `INSUFFICIENT_SCOPE`。
2. **空间角色**（files 自己判定，是真正的访问控制）：`owner > editor > viewer`。读/下载/`latest` 要 viewer；改名、删除、替换、打标签、finalize 应用文件要 editor；管理团队空间成员与空间分享要 owner。不足返回 HTTP 200 `SPACE_FORBIDDEN`。
3. **ACL**：files 不读你 App 的 ACL。你的 ACL 只决定自己界面上显示哪些入口。

| 空间 | 谁有角色 |
| --- | --- |
| 个人空间 | 本人 owner |
| 团队空间 | 成员表里的用户（由用户在文件管理中创建和邀请） |
| 应用空间（每租户每 App 一个，首次写入时创建） | 所属 App 的服务态 owner；**租户管理员也是 owner**；署名成员对署名文件只读 |

存储配置与 Webhook 看门户租户角色 `admin`，否则 403 `FORBIDDEN`。

## 6. 应用空间：App 存自己的内容

- 服务态不传 `spaceId` 即写本 App 应用空间；文件 `ownerUserId=null`、`viaAppKey=<你的 listingKey>`。用户态写入同样记 `viaAppKey`（取令牌 `azp`），归属仍是该用户。
- 文件管理里没有给其他用户或 App 授权应用空间的入口。要开放给别人，在你自己的 App 里按业务权限提供读取，或用 [长期持链接](files-stable-links.md) 公开发布。
- 租户管理员能看、能删应用空间文件。不能被管理员删改的业务数据不要只存在 Files 里。
- 处理器不支持应用空间文件，调用报 `PROCESSOR_APP_FILE_UNSUPPORTED`。
- 要让作者本人看见、读取成员隐私开关：见 [代成员保存](files-member-docs.md)。

## 7. 拉式检测新版本：`GET /<id>/latest`

拷贝或引用过文件的 App 用当初记下的 fileId 一次往返确认有没有新版本（`files.read` + 对该文件 viewer）：

```text
GET /<fileId>/latest  →  { head: FileDTO, isLatest: boolean, hops: number }
```

- 从 pending、历史版本、链头或墓碑出发都能解析；`head.status` 可能是 `"deleted"`（整链已删）。
- 判断「源已更新」用 `head.id !== 你记的 fileId`。`head.digest` 在 presign 路径是申报值，只能辅助判断。
- 没有推送通知：在打开、展示或同步时调用。版本链数据异常时报 `CHAIN_BROKEN`，不会返回其他租户或空间的行。
- 完整历史：`GET /<fileId>/versions?limit&offset`（链头在前，limit ≤ 200）。

## 8. 错误码与限流

| HTTP | code | 含义与处理 |
| --- | --- | --- |
| 401 | `UNAUTHENTICATED` / `INVALID_TOKEN` | 缺票、票失效、缺租户，或用户态票 aud 不是 `files` |
| 403 | `INSUFFICIENT_SCOPE` | 缺所需 `files.*`；服务态先核对签发返回的 scope |
| 403 | `FORBIDDEN` | 服务态票缺 azp；分享、团队空间管理等用户上下文接口被服务态调用；非管理员调存储/Webhook |
| 409 | `APP_EXIT_REQUIRED` | 该租户的 Files 正在协同卸载，停止受理新增写入；读取与删除不受影响 |
| 413 | `PAYLOAD_TOO_LARGE` | `/content` 或原位覆盖超过上限 |
| 429 | `RATE_LIMITED` | 退避 `details.retryAfter` 秒（1–60，到下一窗口） |
| 503 | `PORTAL_UNAVAILABLE` | files 连不上门户（自省或成员核实）；不写入，退避重试 |
| 200 | `SPACE_FORBIDDEN` / `SPACE_NOT_FOUND` / `FILE_NOT_FOUND` / `FILE_GONE` | 角色不足 / 空间不存在 / 文件不存在 / 文件已成墓碑（`GET /<id>` 仍可探测） |
| 200 | `STORAGE_QUOTA_EXCEEDED` | 超租户存储配额，`details` 带 `limit` / `used` / `requested`；finalize 超限时 pending 草稿被删除 |
| 200 | `STORAGE_NOT_CONFIGURED` | 租户没自配存储且平台未开托管存储池，请租户管理员或平台处理 |
| 200 | `STORAGE_POOL_UNREACHABLE` | 平台托管存储暂不可达，是平台故障，不要提示用户「去配置存储」 |
| 200 | `IDEMPOTENCY_CONFLICT` / `REVISION_CONFLICT` / `REPLACE_SUPERSEDED` / `REPLACE_CONFLICT` / `REPLACE_SOURCE_GONE` | 见 §3.2、§3.3；后两者表示输给并发替换或删除，刷新后重新发起 |
| 200 | `VALIDATION_FAILED` / `FOLDER_INVALID_PATH` | 缺 `name`/`fileId`、`onBehalfOf` 不合规（`details.field`）/ 目录路径非法；修请求，不要原样重试 |

限流由 files 自己执行，固定一分钟窗口，默认各 600 次/分（部署可调）：

- 用户态按「租户 × 用户」计。同一成员经任何 App 调用 files 都消耗同一份额度。
- 服务态按「服务账号 × 租户」计，与代表多少用户无关；长期持链接的管理调用也在其中。
