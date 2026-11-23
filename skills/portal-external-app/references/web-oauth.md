# 门户成员的 Web OAuth（有后端的独立站点）

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。
> `<portal-origin>` 指门户公开源；`<key>` 指你的 listingKey。

适用于**不嵌进门户 iframe**、有自己后端（BFF）的 Web 站点，以门户成员身份登录。后端持有
App Secret，是机密客户端。嵌入式页面用 SDK 握手（[SDK 参考](sdk-reference.md)），原生客户端与
纯前端 SPA 是公开客户端（[原生客户端](native-clients.md)），顾客/学生等终端用户走
[终端用户认证协议](end-user-auth.md)。三者都不套用本文。

拿到的 `access_token` 与 iframe 前端是同一种用户态 TDT（`aud=<key>`），后端照常逐请求自省
（[integration-contract.md](integration-contract.md) §3）。

## 1. 公开客户端与机密客户端

| | 公开客户端（`publicClient: true`） | 机密客户端（缺省） |
| --- | --- | --- |
| 授权请求的 PKCE | **必须**带 S256，缺了跳回 `error=invalid_request` | 可选；带了只认 S256 |
| 换码 | `code_verifier`，不带密钥 | 没带 PKCE ⇒ **必须** `client_secret`；带了 PKCE ⇒ verifier 必须对，secret 可选 |
| 刷新 | 链从 PKCE 开始 ⇒ 不要求密钥 | 链不是从 PKCE 开始 ⇒ **必须** `client_secret` |
| 撤销 refresh_token | 只需 `client_id` | 同左 |
| 撤销 access_token | 做不到（没有密钥） | `client_id` + `client_secret` |

规则是「这条链是怎么开始的」：带了 PKCE 的授权码，后续刷新可以不出示密钥；任何时候只要出示了
`client_secret`，它就必须正确，否则 `SECRET_INVALID`。机密客户端也建议带 PKCE，授权码被截获时
没有 verifier 换不出令牌。

`publicClient` 与 `redirectUrls` 都是清单的治理字段，回调地址的登记规则（逐字精确匹配、`http:`
只限回环）见 [原生客户端 §1.1](native-clients.md)。

## 2. 流程

```
① 浏览器顶层跳转（不是 iframe、不是 fetch）
GET <portal-origin>/oauth/authorize
    ?response_type=code&client_id=<key>&redirect_uri=<已登记回调>&state=<随机值>
    [&code_challenge=<BASE64URL(SHA256(verifier))>&code_challenge_method=S256]
    [&scope=<空格分隔，只能收窄>]
→ 302 <redirect_uri>?code=…&state=…        # 未登录先走门户登录（含 2FA / SSO），再回到这里

② BFF 换令牌（表单编码，服务端发起）
POST <portal-origin>/oauth/token
    grant_type=authorization_code&code=…&redirect_uri=<同上>&client_id=<key>
    &client_secret=<App Secret>[&code_verifier=…]
→ { "access_token", "token_type": "Bearer", "expires_in", "scope", "refresh_token"? }

③ 刷新
POST <portal-origin>/oauth/token
    grant_type=refresh_token&refresh_token=…&client_id=<key>&client_secret=<App Secret>
→ 同上，带一枚新的 refresh_token
```

- **租户 = 门户会话当前所在的租户**，授权端没有租户参数；切换租户 = 用户在门户切换后重走 ①②。
  会话没有任何租户时停在门户错误页。
- `code` 一次性、60 s 内有效；回调里**校验 `state`**。没有额外同意屏，同意在 ② 兑换时记录。
- `scope` 与 App 声明的 scopes 取交集，省略即全部声明的 scope。
- 管理员没给该 App 开 `authorization_code` 授权方式 ⇒ 跳回 `error=unauthorized_client`；
  关了刷新 ⇒ ② 不返回 `refresh_token`，③ 报 `GRANT_NOT_ALLOWED`。
- 令牌只留在 BFF 的服务端会话里，不下发到浏览器；浏览器拿着 BFF 自己的会话 Cookie。
- 刷新令牌的轮换、重复出示即整链吊销、单飞与先落盘再使用，规则与原生客户端相同，见
  [原生客户端 §1.3](native-clients.md)。多实例 BFF 要把单飞锁放在共享存储里。

## 3. App Secret 从哪来

`client_secret` 按**租户安装实例**校验。来源二选一：

- 平台托管的 `<PREFIX>_APP_SECRET`：App 声明了 `exchangeTargets` 时由平台签发，布到每个已装
  租户，平台部署时注入运行环境（见 [交付与环境供给](external-delivery.md)）。一盒联调用清单里的
  `exchangeInitiatorSecret`。
- 租户管理员在门户「应用配置 → 密钥」为本租户创建：明文只显示一次；轮换会让本租户该 App
  已签发的 TDT 全部失效。

`SECRET_INVALID` 先查运行环境里的密钥是否与当前租户的有效密钥一致，不要在日志里输出密钥。

## 4. 撤销与退出

```
POST <portal-origin>/oauth/revoke        Content-Type: application/x-www-form-urlencoded
     token=<refresh_token>&client_id=<key>                       # 作废这次登录的整条刷新链
     token=<access_token>&client_id=<key>&client_secret=<App Secret>   # 立即吊销这一枚 TDT
→ { "revoked": true }
```

- 刷新令牌已失效或不存在同样返回 `revoked: true`，可放心重试；`client_id` 与令牌不符返回 `INVALID_GRANT`。
- 用户退出 BFF：先撤刷新链、再撤 TDT，最后清 BFF 会话。门户会话不随之退出，需要时引导用户在门户退出。

## 5. 错误

- `client_id` 不存在、App 未启用或 `redirect_uri` 未登记 ⇒ **停在门户错误页，不跳回**；其余授权
  错误跳回 `redirect_uri?error=<code>&state=…`。
- `/oauth/token` 失败返回 `{ "ok": false, "error": { "code" } }`，按响应体判，不只看 HTTP 状态。常见码：
  `INVALID_GRANT`（code / verifier / redirect_uri / client_id 不符、已用过或过期，用户撤销了该 App
  的授权，或成员 / 用户 / 租户已停用）、`SECRET_INVALID`、`SEAT_REQUIRED`、`APP_DISABLED`、
  `GRANT_NOT_ALLOWED`。
- 停用、移出后的失效时效与客户端处理同原生客户端，见 [原生客户端 §3](native-clients.md)。

## 6. 同源页面的会话取码

部署在门户同源下的页面已带门户会话 Cookie，可由前端调 `POST <portal-origin>/api/tokens/authorize`
`{ "appKey": "<key>", "scopes"? }` 直接取 `{ code }`（响应信封）。这种 code 不绑 `redirect_uri`、
不带 PKCE，BFF 换码时**必须**带 `client_secret`。独立域名的站点用不了它，走 §2。
