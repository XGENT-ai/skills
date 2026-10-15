# 终端用户托管认证协议（enduser-auth-v1）

> 顾客身份先读 [能力与启用条件](end-users.md)。本文适用于已登记的 Web BFF / iOS 终端认证，
> 不使用成员的 `/oauth/authorize`、`/oauth/token`、`publicClient` 或 `redirectUrls`。
> `<auth-origin>` 是 profile 登记的精确 HTTPS 认证源；`<portal-origin>` 是门户 API 公开源。

## 1. 登记与发现

租户在用户中心维护 profile；App 后端持 `endusers.clients` 登记客户端：

```http
POST <portal-origin>/api/v1/end-users/clients
Authorization: Bearer <服务态令牌>
Content-Type: application/json

{
  "siteKey": "customers",
  "clientId": "example-web",
  "kind": "web_bff",
  "name": "网页客户端",
  "redirectUris": ["https://app.example.com/auth/callback"],
  "protocolVersions": ["enduser-auth-v1"],
  "status": "active",
  "expectedRevision": 0
}
```

租户和 owner 从服务票确定；profile 须已存在。新增 `expectedRevision=0`，更新先回读提交当前
revision，冲突处理后再试；已有客户端不能改绑 profile 或 kind。成功 data 返回
`id,kind,name,redirectUris,protocolVersions,status,revision`，持久保存 revision。
`GET /api/v1/end-users/realm?siteKey=...` 可核对本空间及客户端配置，但当前不返回 revision；
公开能力发现也不返回它。遇到后台修改引起的 CONFLICT，需由有权操作人从用户中心核验当前修订，
不能凭空增加服务回读端点、盲猜 revision 或循环覆盖。iOS 使用 `kind:"ios"`，
可登记反向域名形式的 scheme；Web 必须 HTTPS。回调精确匹配，拒绝通配符、userinfo、fragment、
以及占用 `code/state/error` 的查询参数；不要套用成员的回环 HTTP 例外。

从认证源获取：

```text
GET <auth-origin>/api/public/end-user-auth/clients/<clientId>
```

成功 Envelope 的 data 包含 `protocolVersion`、`clientId`、`authorizationEndpoint`、
`tokenEndpoint`、`refreshEndpoint`、`logoutEndpoint`、`targetedTicketEndpoint`、`methods`、
`pkceMethods:["S256"]`、`responseModes:["query"]`、`codeTtlSeconds:60`、`reauthSupported`。
未知/停用客户端、停用 profile、host 不符回 `200 {ok:true,data:null}`。
按元数据 methods 判可用方法（password/apple）；不能从协议版本推断供应商已配通，
也不能从 reauthSupported 推断所有重新认证用途均已上线。

## 2. 普通授权码流程

1. BFF 为本次请求保存随机 state、PKCE verifier、应用浏览器会话、回调和期限。
   浏览器只持应用会话句柄，Portal access/refresh 与服务账号 secret 留在 BFF；
   原生端使用系统浏览器与系统安全存储，不把服务账号 secret 放进安装包。
2. 顶层打开元数据给出的授权地址，不能嵌入 iframe：

```text
GET <auth-origin>/api/public/end-user-auth/authorize
  ?client_id=example-web
  &response_type=code
  &redirect_uri=<已登记的精确回调，URL编码>
  &state=<随机state>
  &code_challenge=<BASE64URL(SHA256(verifier))>
  &code_challenge_method=S256
```

以上六项必须各出现一次。state 是 22–512 字符无填充 base64url，由密码学随机源生成；
verifier 为 43–128 字符 `A-Z a-z 0-9 -._~`，challenge 为 43 字符无填充 base64url。
拒绝 plain、未知/重复参数及自报 tenant/owner/site/audience/scope/profile/subject；
authorize 不接受 lang/view，托管页自己提供语言切换。

3. 平台创建最长 600 秒的浏览器绑定 flow，303 到同一认证源 interaction 页面。
   登录可复用同 profile 的托管浏览器会话，仍要确认账号与目标应用；可换账号或取消。
   注册走邮箱→验证码→密码及必选条款；找回走已验证邮箱验证码；
   首登须改密时完成前不给可用会话。Apple 按 provider sub 绑定，不按同邮箱自动合并账号。
   当前托管 password 表单使用邮箱；不要假定用户名/手机号标识也能直接在此表单登录。
4. 回调成功带 code/state；取消带 `error=access_denied` 与原 state。BFF 核验本地事务、state、
   浏览器绑定和期限并单次消费，然后在服务端换码；非法回调不跳转。

```http
POST <auth-origin>/api/public/end-user-auth/code-token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code&client_id=example-web&redirect_uri=<同一回调>&code=<code>&code_verifier=<verifier>
```

换码 URL 不带查询参数，表单只收这五项，各一次。授权码最多 60 秒、单次使用。
错误 verifier 不消费码；并发正确请求只有一个成功。丢回包后重新授权，不重放旧码领取 refresh。
成功 data 的字段名是：

```json
{
  "protocolVersion": "enduser-auth-v1",
  "endUserId": "<主体UUID>",
  "ownerAppKey": "<身份owner>",
  "siteKey": "customers",
  "sessionId": "<设备会话UUID>",
  "accessToken": "<owner访问票>",
  "refreshToken": "<刷新凭据>",
  "expiresIn": 900,
  "mustChangePassword": false
}
```

这是 `{ok:true,data:...}` 内的 data，不是成员 OAuth 的 snake_case token 响应。
授权码只能交给 code-token，不能交给下面的 token 定向票端点。
成功或取消后转到不含 code/state 的应用 URL。

## 3. 自省、资源鉴权与脱敏资料

BFF/资源后端用获准 `token.introspect` 的服务账号 Basic 调：

```http
POST <portal-origin>/api/tokens/introspect
Authorization: Basic <服务账号clientId:secret的base64>
Content-Type: application/json

{"token":"<owner访问票>"}
```

先解 Envelope，再校验 data。owner access 必须同时满足：

- `active=true`、`kind=enduser`、`credential_type=tdt`，且 `exp` 未过期；
- `end_user_id=principal_id`，与已绑定应用会话的主体相同；`session_id` 有效；
- `tenant_id/owner_app_key/site_key` 与登记值一致；
- `aud=listingKey=azp=owner_app_key`，`scopes=[]`；
- `user_id/role/groups/permissions` 为 null，`bypass/isPlatformAdmin=false`。

HTTP 自省字段为 `end_user_id/session_id/owner_app_key/site_key`；SDK 上下文对应
`endUserId/sessionId/ownerAppKey/siteKey`。成员票、服务票即使 active 也不能替代。
files 定向票不能当 owner access 登录。仅解 JWT 或相信浏览器提交的主体都不构成认证。

TS 后端使用 `@xgent/portal-server-sdk`，仅在具名终端用户端点配置
`acceptedPrincipalKinds:["enduser"]`、`acceptedCredentialTypes:["tdt"]`，再用
`requireEndUser`、校验目标 audience、登记空间与 App 本地业务权限；SDK 缺省只开放 user/service。
终端用户没有成员 PID/管理员角色，不能直接套用成员的四道闸或放开所有后台路由。
自省正缓存上限 **50 秒**，还须受真实 exp 截断；端到端撤销预算 60 秒。
敏感操作可用 `strictIntrospection:true`。合法 inactive 才是认证失效；超时、坏 JSON、
失败信封或依赖故障拒绝为服务故障，不降级匿名。

读取资料另用 `endusers.read` 服务 Bearer 调 `GET /api/v1/end-users/<id>`；
普通 DTO 只有 `identifierMasked/contacts[].addressMasked` 等脱敏字段，
不提供通用 userinfo 或明文联系方式端点。专门的本人数据导出另有授权与范围。

## 4. 刷新、退出、定向票

| 操作 | 请求 | 语义 |
| --- | --- | --- |
| 刷新 | `POST <auth-origin>/api/public/end-user-auth/refresh`，JSON `{"refreshToken":"<当前值>"}` | 返回 accessToken、**新** refreshToken、expiresIn；同会话串行轮换并原子保存，旧值不可重放 |
| 设备退出 | 同前缀 `POST /logout`，JSON `{"refreshToken":"<当前值>"}` | 撤销当前设备会话；丢弃本地凭据，退出结果不明按应用状态处理 |
| 定向票 | 同前缀 `POST /token`，Bearer 终端用户 access，JSON `{"target":"owner"}` 或 `{"target":"files"}` | 只能选平台白名单 target；files 仅 files.read，仍需逐文件业务授权 |

access 当前为 15 分钟，refresh 为 30 天滑动并轮换；始终使用实际 expiresIn。
sessionId 只在设备会话创建时给出，刷新不改变也不再返回。应用设备退出不清认证域的托管
浏览器会话，因此退出后再登录不能代替重新认证。管理员全部下线会连托管浏览器会话一起失效；
单会话撤销不影响其他设备。

## 5. 敏感操作重新认证

持**当前 owner 设备会话**的 access（不是成员/服务/files 票或托管 Cookie）在认证源申请：

```http
POST <auth-origin>/api/public/end-user-auth/reauth/requests
Authorization: Bearer <当前owner访问票>
Content-Type: application/json

{"client_id":"example-web","redirect_uri":"https://app.example.com/auth/callback","purpose":"confirm_action","operationKey":"example-web:export:<本次操作号>"}
```

operationKey 使用字母、数字、`._:-`，1–128 字符；示例的尖括号占位须替换成合法操作号。
每次尝试新号并与具体业务操作绑定。body 只收这四项，不带查询参数。
申请回 requestId、expiresIn（≤600）、authorizationEndpoint。建立新的 state/verifier，
按 §2 六参数额外加 `reauth_request=<requestId>`。平台锁定原主体，要求重新输入密码或完成 Apple
重新认证，不复用 Cookie 免密。回调仍经 code-token 兑换，但 data **仅**包含：

```json
{"reauthToken":"<不透明证明>","expiresIn":300,"purpose":"confirm_action","operationKey":"<原操作号>","endUserId":"<当前主体UUID>"}
```

不含 access/refresh/sessionId，不新建会话。证明 TTL 不超过实际认证时间后 300 秒及 flow 剩余期限。

- `confirm_action`：App 自己消费。可信证据是 BFF 持自己保存的 verifier 直接换码的成功响应，
  不是浏览器单独提交一个字符串。核对 purpose、主体、原发起 client/应用会话、操作号和原操作内容；
  本地 TTL 取实际剩余值与 300 秒较小值并扣网络耗时，原子单次消费；执行时再校验会话和业务权限。
  Portal 没有此证明的验证端点，也不替 App 消费它；不要拿它调用 token introspect。
- `delete_passport`：只供 [删除命令](end-user-lifecycle.md) 消费，不能替代 confirm_action。
- `bind_apple/add_password/unlink_apple` 当前返回 `END_USER_AUTH_UNAVAILABLE`，不要做成可用功能。

取消、换账号、退出或放弃操作时清理 App 本地待执行证明。操作号不承载个人资料或业务敏感内容。

## 6. 站点认证的另一条入口

平台 Sites 主域的 `/_xgent/auth/*` 提供 HTML/JSON 认证入口，同样属于终端身份：密码、邮箱数字验证码
`verify_contact/login/reset_password`、首登改密、退出；首次邮箱验证由所属 App 服务签发
registration-ticket，不能拿它直接换会话。站点 Cookie 只信反代注入的真实站点上下文，
不能自报头取得；设备 JSON 密码入口使用 tenantId/ownerAppKey/siteKey 与凭据，仍检查资格。
这些入口不是 profile 的额外登录方法，也不能把普通站点 Cookie 混作托管认证域的浏览器会话。

已有手机 OTP 绑定/登录/恢复接口，但当前缺生产发送适配；详见 [能力限制](end-users.md)。
已有用户名等账号的设备 JSON 密码入口为 `POST /api/public/end-user-auth/login`，
body 含 `tenantId,ownerAppKey,siteKey,identifierKind,identifier,password,deviceId?`；
成功设备会话字段见 §2，须首改时仅回 changePasswordTicket，不能提前建立应用登录状态。
新 Web BFF / iOS 集成优先按本文托管协议完成，不能直接套用成员登录或把站点表单当 PKCE 回调。

JSON 成功为 `200 {ok:true,data}`；业务失败为 `200 {ok:false,error}`，按 error.code 处理。
400/401/403/429 和服务故障另按传输/鉴权语义处理；404 只表示路径不存在。
