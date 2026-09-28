---
name: portal-app-exchange
description: '跨应用调用（OAuth Token Exchange）的布线与排错。凡任务涉及让 App A 读 App B 的数据（如 组卷读题库、文件调多模态解析、任务中心读 LMS）、声明 exchangeTargets/交换白名单/consent 共授，或出现 EXCHANGE_NOT_ALLOWED / EXCHANGE_CONSENT_REQUIRED / CONSENT_REQUIRED / SECRET_INVALID / GRANT_NOT_ALLOWED / 跨应用下拉列表莫名为空 等症状时，务必先用本 skill 再动手改代码。Use for wiring or debugging cross-app token exchange: grants, whitelists, consent co-grant, scope intersection, and the classic failure modes (secret drift, missing user consent rows, consent narrowing).'
---

# portal-app-exchange · 跨应用调用（令牌交换）布线与排错

App A 代表用户调 App B = 拿一个 `aud=B` 的 TDT，走 OAuth Token Exchange，**严格不可提权**。
本文可直接在 App 自己的仓库使用；声明写入 `app.manifest.json`，平台授权由管理员处理。`/apps/<key>/` 是线上 URL 路径。

> **本文件读法**：先读「模型一页纸」对齐五件事，再读末尾「红线」避免踩坑；中间章节是按需的——布线时读「布线新链路」，写发起方时读「交换请求 / 发起方实现」，调不通时直接看「排错决策树」。每节都不长，且彼此独立。

## 模型一页纸（动手前先对齐这五件事）

放行 = **同时**满足：

1. A→B 授权关系存在（`app_exchange_grants`）；
2. B `allowExchange = true`；
3. A ∈ B 的 `exchangeWhitelist`；
4. 用户已同意"A 代表我访问 B"（`exchange_consents`，服务端交换不带会话，consent 必须**事先**记录）；
5. 结果 scope = A 当前 TDT scopes ∩ B 声明 scopes——**发起方 A 的清单必须把 B 命名空间 scope 声明进自己的 `scopes`**（且 B ∈ A 的 `exchangeTargets`），交集为空 = 换到的令牌调 B 全 403。

被调方 B 看到的：`kind:"user"`、有 `user_id`、`azp` = A（出处归因）；B 照常四道闸鉴权、**无需改代码**。

## 交换请求

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

## Consent 面

- 用户授权页（门户前端路由）：`/exchange-consent?source=<A>&target=<B>&return=<回跳>`。
- 配套接口：
  - `GET /api/me/exchange-consent/:source/:target`（谁在申请 / scope / 是否已授权 / 结构上是否允许）
  - `POST /api/me/exchange-consents`（记录）
  - `GET /api/me/exchange-consents`（列出）
  - `POST /api/me/exchange-consents/:source/:target/revoke`（撤销）
- **共授**：A 声明了 `exchangeTargets` 时，用户首次进入 A 的 consent 门会一并记录对这些目标的交换同意——正常新用户不需要单独走授权页。
- **撤销语义**：撤销交换 consent 不回滚已签发的短期 TDT（自然过期），但拦截后续交换——"撤销后还能用几分钟"不是 bug。

## 布线新链路（A 读 B）

完整 4 步：

1. A 的清单：`exchangeTargets` += `B`，`scopes` += 所需 `B 命名空间.*`；
2. 随发布提案提交 A 的清单，批准后由平台完成交换授权布线；请平台管理员核对 A→B 授权关系、B 的 `allowExchange`、白名单与 A 的 `allowedGrants`。本地一盒通过 `register-app` 注册清单；不要在 App 仓库创建门户配置文件；
3. 用户 consent 靠共授覆盖；**布线晚于用户首次 consent** 的存量用户会缺 `exchange_consents` 行——见排错决策树；
4. B 侧无需改代码——照常四道闸（scope 交集 + azp 归因）。

## 发起方实现要点

发起方后端按上面的标准 token-exchange 请求换票，再用返回的 Bearer token 调目标 App。不要把 App Secret 放进前端。

- 换到的令牌按 **(目标 appKey, user)** 缓存、到期前复用；缓存 key 必须含目标 appKey（曾有"换票缓存串 App"的真实 bug，现象是「有时 403、有时又通」）。
- 交换失败**优雅降级**为可区分状态（`not_installed` / `unreachable` / `needs_consent`）：别把「目标没装」渲染成传输错误、别把 consent 缺失静默吞成空列表。**交换结果本身就是可用性探测信号**。
- 想要更多 scope，改**声明**（A 的 scopes/exchangeTargets + 用户再同意），不是在交换请求里多要。

## 排错决策树

| 症状 | 病根 | 处置 |
| --- | --- | --- |
| `EXCHANGE_NOT_ALLOWED` | 缺 grant / B 未开 `allowExchange` / A 不在白名单 | 查布线步 1–2；本地一盒重跑 `register-app`，生产交平台管理员核对授权 |
| `EXCHANGE_CONSENT_REQUIRED` | 该用户没有 A→B consent 行 | 通过上述 consent 查询接口核对，再引导用户走 `/exchange-consent` 完成授权；不要代替用户补写同意 |
| 跨应用下拉/列表静默为空 | 同上（发起方吞了 consent 错误） | 先查 consent 接口，再查 scope 交集 |
| `SECRET_INVALID` / 「跨应用授权缺失或未开启」但布线在 | **App Secret 漂移**：A 侧运行密钥与平台保管的当前密钥不一致 | 请平台管理员核对/轮换密钥并重新注入；核对 A 的运行环境是否漏配 `*_APP_SECRET`，不要输出密钥 |
| `GRANT_NOT_ALLOWED` | A 的 `allowedGrants` 未含 `token_exchange` | 改 A 清单/注册 |
| 换到令牌但调 B 全 403 `INSUFFICIENT_SCOPE` | 交集为空：A 未声明 B 命名空间 scope，或 A 入站 TDT 本身没带 | 查 A 清单 scopes + 入站令牌 scopes |
| 更宽 scope mint 触发 `CONSENT_REQUIRED` | 同意被子集 mint **收窄**过（mint 把本次 scope 记为同意范围） | 先重新走 consent 再宽 mint；日常 mint 别传裁剪 scopes |

## 与服务态直调的边界

`client_credentials`（服务账号签服务态 TDT，无用户上下文）不走本链路——那是"服务调服务"的姿势，consent / 白名单 / 交集都不适用，靠服务账号的 scope 与租户策略收口。判别：请求是否代表**某个用户**？是 → 令牌交换；否 → 服务态直调（见 portal-external-app skill 的服务账号契约）。

**写入时这条判别的实用形式是「这份数据归谁」**（两种姿态在目标 App 里落进不同的容器、有不同的可见性，选错了功能会当场消失）：

| 归属 | 姿态 | 后果 |
| --- | --- | --- |
| **归用户**：用户主动存的、他在目标 App 里看得见删得掉、算他的配额 | 令牌交换（用户态） | 落用户自己的容器；令牌带 `azp=A`，目标侧照样有来源归因 |
| **归 App**：系统产物、用户删不得、随 A 的生命周期消失 | 服务账号（服务态） | 落 A 的应用级容器，默认只有租户管理员可见 |

典型误判：把「用户点一下另存到我的空间」做成服务态——写进去的是 A 的应用容器，用户在目标 App 里根本看不见自己刚存的东西，功能等于没有。

## 红线

- 撤销交换 consent 不回滚已签发的短期 TDT（自然过期），但拦后续交换——"撤销后还能用几分钟"不是 bug。
- 交换永不扩权：要更多 scope，改**声明**（A 的 scopes/exchangeTargets + 用户再同意），不是在交换请求里多要。
- 服务态令牌（`client_credentials`）不走本链路——判别标准见上节。**写入时务必先回答「这份数据归谁」**，否则功能会"看上去上线了"但用户根本找不到。
- **别拿另一个 App 的清单当平台规则**：它只声明了读，往往是它自己的写面不代表用户，而不是平台禁止。规则只有模型一页纸那五条。