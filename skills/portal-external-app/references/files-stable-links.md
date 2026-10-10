# Files 长期持链接 URL 接入

需要把本 App 的普通文件发布为固定内容、不过期、可吊销的图片/视频/PDF URL 时读取。
这是服务态公开分发能力；需逐领取者核业务权限的文件继续走 BFF 与受控产物，不改变其 24 小时寿命。

## 1. 申请与文件归属

在 App 自己的 manifest 申请，由门户发布审核逐项批准：

```json
"privilegedServiceScopes": [
  { "scope": "files.links.manage", "reason": "把本应用生成的海报公开给第三方平台抓取" }
]
```

`reason` 写明公开用途。不能放进普通 `serviceScopes`；用户态令牌、租户自助服务账号、
默认服务授予都不能取得此 scope。平台服务账号必须归属当前 App，App 与 Files 在目标租户正常安装。

先用本 App 获准的 `files.write` 服务态令牌上传并完成文件，省略 `spaceId` 写入自有应用目录。
个人/团队文件和其他 App 授权给你的目录，即使可读写也不能发布。成员隐私开启时，
带 `onBehalfOfUserId` 的代成员文件不可发布。单份快照上限 200,000,000 字节。

后端通过门户 `POST /api/tokens/service`、`client_credentials`、Basic 服务账号凭据、
目标 `tenant_id` 与所需 `scope` 取得服务态 TDT（签发请求包含 `files.links.manage`）；
不交换用户态令牌。服务账号密钥留在后端，不给前端。平台需先启用该环境的长期链接开关并验证存储位置。

## 2. 签发与恢复

基址为去掉末尾 `/` 的 `PORTAL_BASE_URL` 加 `/svc/files/api/v1/files`。
管理调用带 `Authorization: Bearer <服务态TDT>`；JSON 请求带 `Content-Type: application/json`。

| 操作 | 相对路径 | 成功 data |
| --- | --- | --- |
| 签发 | `POST /<fileId>/links`，body `{ "requestKey": "<业务幂等键>", "expectedRevision": <FileDTO.revision> }` | 链接 DTO，通常 creating、url 为 null |
| 丢回包恢复 | `GET /links/by-request-key?requestKey=<编码后的业务键>` | 原 DTO 或 null |
| 轮询 | `GET /links/<linkId>` | DTO；只有 ready 带 url |
| 列表 | `GET /<fileId>/links?limit=50&cursor=<id>` | items/nextCursor，列表不带 URL |
| 吊销 | `DELETE /links/<linkId>` | linkId/state/cleanupState 或 null；不可逆，幂等 |

检查 HTTP 状态和 `{ ok, data | error }` 信封，不能只看 HTTP 200。
同 App/安装下同 `requestKey` 同参数返回原任务/URL/终态；参数改变报 `IDEMPOTENCY_CONFLICT`。
文件 revision 不符报 `FILE_REVISION_CONFLICT`。要发布新内容，用新业务键；失败或吊销的旧键不会重建。

creating 从 2 秒起退避至 10 秒，直到 ready 或 failed/revoked/invalidated；到 ready 才保存/使用 URL。
DTO 带 protocolVersion=1、id/fileId/sourceRevision/state/name/contentType/size/sha256/url、
expiresAt=null、createdAt/readyAt/cleanupState/lastError（稳定码）。
签发/查询与其他服务态 Files 调用共用每「服务账号 × 租户」600 次/分额度，多任务统一节流。

`LINK_ISSUE_DISABLED` 表示未启用签发或公网 base 未配置；`LINK_SOURCE_NOT_ELIGIBLE` 表示来源不适用；
`NOT_FOUND` 不区分不存在与不属于本 App。401/403 是凭据/当前资格失败；Files 主安装或租户/App
停用时，查询和吊销也拒绝，恢复后才能操作。503 表示依赖暂不可用，可有界退避。

共享 DTO/schema/scope 与错误码已由 `@xgent/shared@0.8.4` 提供；无需增加浏览器 SDK 方法，REST 即可调用。

## 3. 使用与验收

只使用返回的 `<门户源>/svc/files/l/<token>`，不拼内部 Files 端口或存储预签名地址。
URL 是不记名凭证：持有即可匿名 GET/HEAD，不需要 Cookie/TDT；避免写日志、埋点或公开无关页面。
`expiresAt=null`；签发令牌过期或服务账号正常轮换密钥不会改变已发 URL。

完整 GET/HEAD 的 Content-Length 是快照大小，单区间 Range 返回 206、区间长度与 Content-Range；
强 ETag 是快照 SHA-256，条件请求可返回 304。网关保留 no-referrer 且不压缩。
至少用 256 KiB 文件验证完整 GET 和较大 Range 的字节 SHA-256、长度头，以及 HEAD、304、416；
公开响应不应泄漏内部 `X-Xgent-File-Link-Length`。平台部署必须同时更新 Files 和网关的长度桥接，
内部服务端口的 chunked 帧不是公开契约验收目标。

CORS 允许任意 Origin、不带凭证；页面 CSP 需允许门户源的 img-src/media-src/connect-src。
HTML/SVG/XML/JS 等危险类型以附件、application/octet-stream 与 sandbox 返回，不当成可执行内联内容。

| 公开状态 | 客户端处理 |
| --- | --- |
| 200/206 | 正常字节；校验预期大小/类型 |
| 304 | 复用已存缓存 |
| 404 LINK_UNAVAILABLE | 不存在、吊销、删源或资格不符，不区分；不要入库为成功文件 |
| 410 GOVERNANCE_HOLD | 平台临时下架，解除后可能恢复 |
| 400/416 | 改正 Range；只支持单区间 |
| 429/503 | 按 Retry-After 有界退避；503 也可能是紧急停分发 |

失败是 JSON 与 no-store，HEAD 同状态无体；`fetch().ok` 为 false 或媒体元素触发 onerror。
成功缓存必须每次重验，不能用永久缓存绕过吊销。

## 4. 生命周期

源覆盖、替换版本或改名不改变已发快照；删源文件/版本链会失效。主动吊销不可逆，快照异步物理清理。
签发账号停用/撤 scope、App/租户或 Files 停用期间返回 404，恢复资格后可恢复。
App 卸载/重装改变安装代次、租户或服务账号删除会失效并清理；重装不复活旧 URL。
平台 hold 返回 410，解除后仍要检查资格与来源。已下载或已开始传输的字节无法追回。
