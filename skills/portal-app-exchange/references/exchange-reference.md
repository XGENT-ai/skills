# 令牌交换（Token Exchange）机制参考

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §11/§15.3/§15.4（2026-07）。冲突时以门户仓库为准。

## 1. 放行条件（五件缺一不可）

App A 代表用户调 App B = 用 A 的 TDT 换一个 `aud=B` 的 TDT。**严格不可提权**：

1. **授权关系** A→B 存在（`app_exchange_grants`）；
2. **B 开启交换**：`targetApp.allowExchange = true`；
3. **A 在 B 白名单**：A ∈ `targetApp.exchangeWhitelist`；
4. **用户同意**：用户事先记录过"A 代表我访问 B"（`exchange_consents`）——服务端到服务端交换不带会话，consent 必须**事先**存在，否则 `EXCHANGE_CONSENT_REQUIRED`；
5. **scope 交集**：结果 scope = A 当前 TDT scopes ∩ B 声明 scopes（绝不扩张）。⚠️ 这要求 **A 的清单把 B 命名空间 scope 声明进自己的 `scopes`**（且 B ∈ A 的 `exchangeTargets`，否则清单写入即被 scope 校验拒）；交集为空 = 换到的令牌调 B 全 403。

## 2. 交换请求

```
POST {PORTAL}/oauth/token
  grant_type    = urn:ietf:params:oauth:grant-type:token-exchange
  subject_token = <A 的当前入站 TDT>
  audience      = <B 的 appKey>
  client_id     = <A 的 appKey>        # 必须等于 subject_token.aud
  client_secret = <A 的 App Secret>
→ { access_token: <aud=B 的 TDT>, scope, expires_in }
```

产物是**用户态** TDT（有 `user_id`），且带 `azp = A`（发起方，B 侧自省可见，用于出处归因）。A 的 `allowedGrants` 须含 `token_exchange`，否则 `GRANT_NOT_ALLOWED`。

## 3. Consent 面

- 用户授权页（门户前端路由）：`/exchange-consent?source=<A>&target=<B>&return=<回跳>`。
- 配套接口：`GET /api/me/exchange-consent/:source/:target`（谁在申请/scope/是否已授权/结构上是否允许）、`POST /api/me/exchange-consents`（记录）、`GET /api/me/exchange-consents`（列出）、`POST /api/me/exchange-consents/:source/:target/revoke`。
- **共授**：A 声明了 `exchangeTargets` 时，用户首次进入 A 的 consent 门会一并记录对这些目标的交换同意——正常新用户不需要单独走授权页。
- **撤销语义**：撤销交换 consent 不回滚已签发的短期 TDT（自然过期），但拦截后续交换——"撤销后还能用几分钟"不是 bug。

## 4. 布线一条新链路（A 读 B）

1. A 的清单：`exchangeTargets` += `B`，`scopes` += 所需 `B 命名空间.*`；
2. 门户 monorepo：`apps/api/src/db/provisioning.ts` 的 `EXCHANGE_WIRING` 加 `A: [..., "B"]`（生产幂等建 grant + B 开 allowExchange + 白名单）；dev 安装期 `wireExchangeTargets` / 外部 App 的 register-app 同效；
3. 用户 consent 靠共授覆盖；**布线晚于用户首次 consent** 的存量用户会缺 `exchange_consents` 行（见 §6）；
4. B 侧无需改代码——照常四道闸（scope 交集 + azp 归因）。

## 5. 发起方实现要点

参考实现（门户 monorepo）：`apps/files-server/src/lib/exchange.ts`（files→omni-parser）、`apps/library-server`（多目标 →files/exam/lms）。

- 换到的令牌按 **(目标 appKey, user)** 缓存、到期前复用；缓存 key 必须含目标 appKey（曾有"换票缓存串 App"的真实 bug）。
- 交换失败**优雅降级**为可区分状态（`not_installed` / `unreachable` / `needs_consent`），别把「目标没装」渲染成传输错误、别把 consent 缺失静默吞成空列表。交换结果本身就是可用性探测信号。
- 想要更多 scope，改**声明**（A 的 scopes/exchangeTargets + 用户再同意），不是在交换请求里多要。

## 6. 排错决策树

| 症状 | 病根 | 处置 |
| --- | --- | --- |
| `EXCHANGE_NOT_ALLOWED` | 缺 grant / B 未开 allowExchange / A 不在白名单 | 查布线（§4 步 1–2）；dev 重跑 register-app / 重装 |
| `EXCHANGE_CONSENT_REQUIRED` | 该用户没有 A→B consent 行 | 引导走 `/exchange-consent`；存量用户多半是布线晚于 consent——查 `exchange_consents` 按 (user, source, target) 缺行则参照同租户正常用户的行补插 |
| 跨应用下拉/列表静默为空 | 同上（发起方吞了 consent 错误） | 先查 consent 行，再查 scope 交集 |
| `SECRET_INVALID` / 「跨应用授权缺失或未开启」但布线在 | **App Secret 漂移**：A 侧 env 的 secret ≠ 门户 `app_secrets` 哈希 | A 侧 secret 求 sha256 与门户 DB 哈希比对；A 的运行环境漏配 `*_APP_SECRET` 是常见形态 |
| `GRANT_NOT_ALLOWED` | A 的 allowedGrants 未含 token_exchange | 改 A 清单/注册 |
| 换到令牌但调 B 全 403 `INSUFFICIENT_SCOPE` | 交集为空：A 未声明 B 命名空间 scope，或 A 入站 TDT 本身没带 | 查 A 清单 scopes + 入站令牌 scopes |
| 更宽 scope mint 触发 `CONSENT_REQUIRED` | 同意被子集 mint **收窄**过（mint 把本次 scope 记为同意范围） | 先重新走 consent 再宽 mint；日常 mint 别传裁剪 scopes |

## 7. 与服务态直调的边界

`client_credentials`（服务账号签服务态 TDT，无用户上下文）不走本链路——那是"服务调服务"的姿势，consent/白名单/交集都不适用，靠服务账号的 scope 与租户策略收口。判别：请求是否代表**某个用户**？是 → 令牌交换；否 → 服务态直调。
