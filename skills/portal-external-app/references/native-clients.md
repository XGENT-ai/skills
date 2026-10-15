# 门户成员的原生客户端（iOS / Android / 桌面）接入

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。
> `<portal-origin>` 指门户公开源（与 `PORTAL_BASE_URL` 相同，客户端用）；`<API_BASE_URL>` 是后端拿到的
> 内部门户 API 地址（服务端调用用）；`<key>` 指你的 listingKey。

本文只讲门户成员 `kind=user`。顾客、学生等 `kind=enduser` 使用独立的
[终端用户认证协议](end-user-auth.md)，不能套用下面的成员 OAuth、席位、角色或推送设备接口。

原生客户端是**你这个 App 的另一种前端**：它拿到的是和 iframe 前端同一种用户态 TDT，打你
后端的 `/svc/<key>/…`，后端照常走四道闸（[integration-contract.md](integration-contract.md) §3）。
区别只在**令牌怎么到手**：iframe 由宿主注入，原生客户端自己走 OAuth。

## 0. 速答

| 问题 | 结论 |
| --- | --- |
| 推荐哪种登录方式？ | **授权码 + PKCE（S256），在系统浏览器里完成**，`publicClient: true`，不带任何密钥。这是本文门户成员的原生登录路径 |
| 用户长期凭证能当设备凭证吗？ | **不能**。它是绑定「租户 + 用户 + 目标 App」的 bearer 密钥，没有设备绑定，换不出 TDT / 刷新令牌，资源端默认拒收。设备上的用户用授权码登录；无人值守设备用服务账号 |
| 用户被移出 / 停用后 60 秒内失效？ | **成立，前提是后端按契约逐请求自省**（SDK 正缓存上限 55 s）。刷新令牌在下一次刷新时被拒并作废，所以客户端以后端 401 为准（§3） |
| 有不依赖用户登录的成员查询 / 事件推送吗？ | **查询有，成员事件推送没有**。服务账号可逐租户列在职成员、实时查单个成员状态；门户不推送成员加入/移出/停用事件（§4） |

## 1. 登录：授权码 + PKCE

### 1.1 manifest 声明（治理档，首次与每次改动都进「发布审核」）

```jsonc
"publicClient": true,
"redirectUrls": ["https://app.example.com/oauth/callback", "com.example.app:/oauth/callback"]
```

- `publicClient: true` ⇒ 授权端**强制** PKCE，客户端不持有、也不需要 App Secret。**别把 App Secret 打进 IPA/APK/安装包**。
- `redirectUrls` 是**逐字精确匹配**：不允许通配符、fragment、URL 内用户名口令；`http:` 只允许
  `localhost` / `127.0.0.1` / `[::1]`。可用 https（Universal Link / App Link，**优先**，域名归属可证明）
  或自定义 scheme。桌面端回环回调要登记**固定端口**——不支持「任意端口」匹配。
- TDT 有效期、是否允许刷新、刷新令牌有效期**不是 manifest 字段**，由平台/租户管理员按 App 配置
  （缺省：TDT 3600 s、允许刷新、刷新令牌 30 天）。客户端一律读响应里的 `expires_in`，别写死。

### 1.2 流程

```
① 系统浏览器打开
GET <portal-origin>/oauth/authorize
    ?response_type=code&client_id=<key>&redirect_uri=<已登记回调>
    &code_challenge=<BASE64URL(SHA256(verifier))>&code_challenge_method=S256
    &state=<随机值>[&scope=<空格分隔，只能收窄>]
→ 302 <redirect_uri>?code=…&state=…          # 未登录先走门户登录（含 2FA / SSO），再回到这里

② 换令牌（表单编码）
POST <portal-origin>/oauth/token
    grant_type=authorization_code&code=…&redirect_uri=<同上>&client_id=<key>&code_verifier=…
→ { "access_token", "token_type": "Bearer", "expires_in", "scope", "refresh_token"? }   # 管理员关了刷新则无 refresh_token

③ 刷新
POST <portal-origin>/oauth/token
    grant_type=refresh_token&refresh_token=…&client_id=<key>
→ 同上，带一枚新的 refresh_token

④ 调自己的后端
Authorization: Bearer <access_token>  →  <portal-origin>/svc/<key>/…
```

硬规则：

- **必须用系统浏览器**（iOS `ASWebAuthenticationSession`、Android Custom Tabs、桌面默认浏览器），
  不用内嵌 WebView：登录依赖门户会话 Cookie 与 2FA/SSO 页面，WebView 里既拿不到已有会话，也违背
  公开客户端的安全前提。
- **租户 = 浏览器里门户会话当前所在的租户**，授权端没有租户参数。多租户用户要先在门户切换租户再
  授权；客户端「切换租户」= 重新走一次 ①②。当前租户以后端自省得到的 `tenant_id` 为准。
- `code` 一次性、60 s 内有效；**校验回来的 `state`** 与发起时一致。授权端没有额外同意屏，同意在
  ② 兑换时记录。
- 错误分两类：`client_id` 不存在或 `redirect_uri` 未登记 ⇒ **停在门户错误页、不会跳回 App**（防止
  把错误带到未登记地址）；其余可恢复错误跳回 `redirect_uri?error=<code>&state=…`。
  `/oauth/token` 失败返回 `{ "ok": false, "error": { "code" } }`，**按响应体判，不能只看 HTTP 状态**。
  常见码：`INVALID_GRANT`（code / verifier / redirect_uri / client_id 不符、令牌已用过、用户已撤销该 App
  的授权，或成员 / 用户 / 租户已停用）、
  `SEAT_REQUIRED`（席位制 App 未分配席位）、`APP_DISABLED`、`GRANT_NOT_ALLOWED`（管理员关了刷新）。

### 1.3 刷新令牌：轮换、单飞、本地保管

- **每次刷新都轮换**：旧 `refresh_token` 立即作废，**被拒的那次同样会消耗掉它**。
- **重复使用 = 整次登录作废**：一枚令牌被用掉 30 秒后又被出示，门户按泄露处理，连同它之后轮换出的
  最新令牌一起吊销（这样被盗的令牌会在真客户端下一次刷新时失效）。30 秒内的重复出示（自己的并发
  刷新、网络重试）只拒绝这一次，不连累已经换到新令牌的那个请求。所以：
  - 同一账号的刷新必须**单飞**（全进程一把锁，并发请求等同一次刷新结果）；没做到时，输掉的那次会
    拿到 `INVALID_GRANT`，此时应重读本地存储里的新令牌，而不是直接退出。
  - 先把新 `refresh_token` **原子落盘**，再使用新 `access_token`；响应丢失（断网、进程被杀）后旧令牌
    已不可用，只能重新登录——把这当成正常分支处理。
- 刷新窗口是**滑动**的：每次刷新都重置为完整有效期，长期活跃的设备不会被强制重登。
- `refresh_token` 放 Keychain / Android Keystore 支撑的加密存储；`access_token` 放内存即可。
- **退出登录**：先吊销刷新令牌，再删本地全部令牌并注销 §1.4 登记的推送设备：

  ```
  POST <portal-origin>/oauth/revoke
       Content-Type: application/x-www-form-urlencoded
       token=<refresh_token>&client_id=<key>
  → { "revoked": true }
  ```

  只需 `client_id`，不带密钥；作废的是这次登录的整条刷新链。令牌已失效或不存在同样返回
  `revoked: true`，可放心重试；`client_id` 与令牌不符返回 `INVALID_GRANT`。`access_token` 公开客户端
  撤不掉，丢弃即可，剩余有效期内仍能用。需要清掉浏览器里的门户会话时，再引导用户在门户退出。
- **丢失设备**：用户在门户「已授权应用」里撤销该 App，当前租户里该用户这个 App 的全部刷新链、未兑换的
  授权码和 TDT 立即作废（所有设备一起下线，重新登录即可）；管理员停用成员同样生效（§3）。

### 1.4 推送设备登记

APNs / FCM 推送令牌登记在门户，**不要自建设备表**（所有原生 App 共用一张，推送凭据集中在平台）：

```
POST   <portal-origin>/api/me/devices     Bearer <用户 TDT>
       { "platform": "ios"|"android", "provider": "apns"|"fcm", "token", "appKey": "<key>", "appVersion"?, "locale"? }
GET    <portal-origin>/api/me/devices
DELETE <portal-origin>/api/me/devices/:id   # 退出登录时调
```

响应不回显推送令牌。推送能否送达取决于本环境平台是否已配置对应的 APNs / FCM 渠道，接入前与平台确认。

## 2. 设备凭证：别拿用户长期凭证顶

门户的**用户长期凭证**（`xuak_` 前缀，用户在门户个人资料的「用户凭证」页或经 App 委托签发）是为 CLI、MCP、
个人自动化这类**机器入口**设计的。接入时使用 [CLI 与用户访问密钥契约](cli-user-keys.md) 的受审 `cliAccess` 声明和标准绑定协议：

- 绑定**租户 + 用户 + 目标 App**，是纯 bearer 密钥：没有设备字段、没有持有证明，谁拿到谁能用；
- **换不出** TDT 或刷新令牌，不能顶替登录；
- 资源端 SDK **默认只收** `tdt` / `service_access_key`，只有显式开放的机器入口才接收
  `user_access_key`，且必须逐请求按 `credentialId` 核对本 App 自己的绑定记录。

所以按「谁在用这台设备」二选一：

| 场景 | 用什么 | 说明 |
| --- | --- | --- |
| **人在设备上用 App**（手机、平板、桌面客户端） | §1 授权码 + PKCE | 每次安装各有一条独立的刷新链，天然按设备区分；身份、席位、ACL 与 Web 端完全一致 |
| **无人值守设备**（采集器、kiosk、网关、IoT） | **服务账号** | 租户管理员在门户 `/admin/service-accounts` 自助创建，`xsak_` 长期密钥直接 Bearer，或 clientId + secret 换服务态 TDT。令牌 `kind:"service"`、无 `user_id`，只按 scope 授权；可选 scope 只来自你 manifest 的 `scopes`（见 [integration-contract.md §3.1](integration-contract.md)） |

**不要**让用户长期凭证冒充设备身份，也不要让服务账号冒充某个用户——前者绕开设备边界，后者绕开
成员资格、席位与 ACL。

## 3. 停用 / 移出后多快失效

成员被停用、移出，或租户被停用后，门户**自省**对这位用户的 TDT 与用户长期凭证**立即**返回
`active:false`：每次自省都实时查成员、用户、租户状态与 App 安装状态，门户侧不缓存用户结果。成员
停用 / 移出还会同时作废该用户已签出 TDT 的版本号与门户会话。

剩下的延迟全在**你的后端缓存**里，这就是 60 秒预算的构成：

| 你的后端 | 最坏延迟 |
| --- | --- |
| 用 `@xgent/portal-server-sdk` 且传了 `cache` 适配器（默认配置） | 30 s（正缓存默认值） |
| 用 SDK 但不传 `cache` | 0（不做正缓存，每请求直查） |
| 调大 `positiveCacheMs` | 上限 55 s（SDK 强制截断；加门户 5 s 配置缓存，总预算 60 s） |
| `strictIntrospection: true` | 0（每请求直查，敏感写操作用这个） |
| 其他语言自实现 | 按 [integration-contract.md](integration-contract.md) §3 「到期与缓存」实现即同上 |

**保证成立的前提**（任何一条不满足，60 s 都不成立）：

1. **每个请求都经自省**，正缓存 ≤ 55 s。本地验签、只看 `exp`、或用 TDT 换一个自建长会话 ⇒ 延迟变成
   TDT 剩余寿命（缺省 1 h，最长 24 h）。
2. 客户端以**后端返回 401（`active:false`）**为「已失去权限」的首要信号；刷新结果要等 TDT 到期才看得到。

**不在 60 秒保证里的**：

- **成员停用不会主动吊销刷新令牌**，只在下一次被使用时拦下：成员停用 / 移出、用户停用或租户停用期间，
  `/oauth/token` 的授权码兑换与刷新都返回 `INVALID_GRANT`，**所出示的那枚刷新令牌随之作废**，
  之后成员恢复也不会复活它。停用期间从没刷新过的链，在成员恢复后照常可用。
  客户端处理：刷新返回 `INVALID_GRANT`，或后端返回 401（`active:false`）⇒ 视为已退出，清本地令牌并
  回到登录页。两者都要处理：TDT 没到期之前客户端不会去刷新，最先看到的通常是后端的 401。
- 用户创建的**服务账号 / `xsak_`** 不随成员停用失效：它们属于租户，不属于个人，需要管理员单独处置。

## 4. 不依赖用户登录的服务端能力

### 4.1 成员查询 ✅

服务账号在 manifest 的 `serviceScopes` 里声明 `directory.read`（治理档，审批后授予；租户自助建的服务
账号勾不到这类平台基础 scope）：

```
POST <API_BASE_URL>/api/tokens/service              # 每个租户各换一枚，令牌钉死在该租户
     Authorization: Basic base64(<saClientId>:<saSecret>)
     grant_type=client_credentials&tenant_id=<租户 UUID>&scope=directory.read
GET  <API_BASE_URL>/api/v1/directory/users           Bearer <服务态 TDT>
→ { "users": [{ "id", "name", "role", "email"?, "seat"? }] }
```

- 只返回**在职**成员（成员与用户状态均 active），停用/移出的人直接不在列表里；`email` 需另加
  `directory.email.read`；席位制 App 附带 `seat`。加 `?appAccess=caller` 附带每人 `canUseApp`。
- 目前**没有**分页、增量（`updatedSince`）和按 id 查询。

查**单个用户此刻**的状态，用服务账号 Basic 认证（与自省同一把凭证）调：

```
POST <API_BASE_URL>/api/v1/acl/check
     Authorization: Basic base64(<saClientId>:<saSecret>)
     { "tenant_id", "user_id", "listing_key": "<key>", "pids"?: [...] }
→ { "membershipActive", "role", "bypass", "seatHeld", "appInstalled", "plan", "results" }
```

`membershipActive:false` 即不在职。**有副作用的动作之前现查、不缓存**。只能查本服务账号够得着的
租户：归属租户与租户白名单之内，且你的 App 在该租户**有安装记录**（停用、卸载过的也算，此时
`appInstalled:false`）；否则返回 `TENANT_NOT_ALLOWED`。

### 4.2 事件推送 ❌（成员事件）

门户**不推送**成员加入、移出、停用事件，也没有 `app.installed` 事件。现有的推送只有：

- 租户管理员在 App 配置里填的 webhook 地址，事件仅 `app.enabled` / `app.disabled` / `app.uninstalled` /
  `consent.granted` / `consent.revoked`：**无签名、不重试、5 s 超时、payload 不含租户 id**，只能当提示，
  不能据此做授权决定。
- App 卸载协调是**拉取式**：App 轮询待处理的卸载操作，门户不回调 App 的地址。
- 平台业务事件总线由 App 发布、App 订阅，门户自己不往里发成员事实。

### 4.3 推荐组合

| 需求 | 做法 |
| --- | --- |
| 用户请求的实时授权 | 每请求自省（§3），这就是唯一权威 |
| 后台任务要对某人做事（推送、定时任务、代发通知） | 动作前 `acl/check` 现查该用户 |
| 清理离职者在本 App 的数据 | 逐租户定期拉 `directory/users` 与本地出现过的 user id 比对；**只用来清理，不把通讯录镜像进自己库**当选人数据源（选人用门户的选人组件） |
