---
name: portal-app-exchange
description: '跨应用调用（OAuth Token Exchange）的布线与排错。凡任务涉及让 App A 读 App B 的数据（如 组卷读题库、文件调多模态解析、任务中心读 LMS）、声明 exchangeTargets/交换白名单/consent 共授，或出现 EXCHANGE_NOT_ALLOWED / EXCHANGE_CONSENT_REQUIRED / CONSENT_REQUIRED / SECRET_INVALID / GRANT_NOT_ALLOWED / 跨应用下拉列表莫名为空 等症状时，务必先用本 skill 再动手改代码。Use for wiring or debugging cross-app token exchange: grants, whitelists, consent co-grant, scope intersection, and the classic failure modes (secret drift, missing user consent rows, consent narrowing).'
---

# portal-app-exchange · 跨应用调用（令牌交换）布线与排错

App A 代表用户调 App B = 拿一个 `aud=B` 的 TDT，走 OAuth Token Exchange，**严格不可提权**。权威章节随本 skill 附带：[`references/docs/SSO与App开发指引.md`](references/docs/SSO与App开发指引.md)（下称"指引"；`references/` 是门户仓库文档的镜像，文内相对链接原样可解析）§11（模型）、§15.3/§15.4（布线与被调方）。参考实现（门户 monorepo 内路径，外部项目按指引 §11 实现同等逻辑）：发起方 `apps/files-server/src/lib/exchange.ts`（files→omni-parser）；多目标 `apps/library-server`（→files/exam/lms）。

> 本 skill 自包含，可整目录拷到任何 repo 使用；门户仓内工作时指引以 `docs/` 原件为准（`.claude/skills/sync-portal-skill-refs.sh` 同步副本）。

## 模型一页纸（改代码前先对齐）

放行 = 同时满足五件事：

1. A→B 授权关系存在（`app_exchange_grants`）；
2. B `allowExchange = true`；
3. A ∈ B 的 `exchangeWhitelist`；
4. 用户已同意"A 代表我访问 B"（`exchange_consents`，服务端交换不带会话，consent 必须**事先**记录）；
5. 结果 scope = A 当前 TDT scopes ∩ B 声明 scopes——**发起方 A 的 listing 必须把 B 的命名空间 scope 声明进自己的 `scopes`**（且 B ∈ A 的 `exchangeTargets`，否则清单写入就被 `validateScopes` 拒）。交集为空 = 换到的令牌调 B 全 403。

被调方 B 看到的令牌：`kind:"user"`、有 `user_id`、`azp` = A（出处归因）；B 照常四道闸鉴权。

## 布线一条新链路（A 读 B）

1. A 的 listing（`apps/api/src/db/provisioning.ts` `LISTING_DEFS`，或外部 App 的 `app.manifest.json`）：`exchangeTargets` 加 `B`，`scopes` 加所需的 `B 命名空间.*`；
2. `EXCHANGE_WIRING` 加 `A: [..., "B"]`（生产 provisioning 幂等建 grant + B 开 allowExchange + 白名单）；dev 安装期 `wireExchangeTargets` 同效；
3. consent：用户首次进入 A 时，consent 门对声明的 `exchangeTargets` **共授**（平台级行为）。存量用户/布线晚于 consent 的，`exchange_consents` 可能缺行——见排错；
4. 发起方代码照 `files-server/src/lib/exchange.ts`：`POST /oauth/token`，`grant_type=urn:ietf:params:oauth:grant-type:token-exchange`，`subject_token`=A 的当前入站 TDT，`audience=B`，`client_id=A` + `client_secret=A 的 App Secret`。换到的令牌**按 (appKey, user) 缓存、到期前复用**；缓存 key 必须含 appKey（曾有"换票缓存串 App"的 bug）；
5. 交换失败要**优雅降级**成能区分的状态（`not_installed` / `unreachable` / `needs_consent`），别把「B 没装」渲染成传输错误——files 的 processors 可用性探测就是范本。

## 排错决策树（按错误码对号）

| 症状 | 病根 | 处置 |
| --- | --- | --- |
| `EXCHANGE_NOT_ALLOWED` | 缺 grant / B 未开 allowExchange / A 不在白名单 | 查布线三件（上节 1–2 步）；dev 重跑 register-app / 重装 |
| `EXCHANGE_CONSENT_REQUIRED` | 用户没有 A→B 的 consent 行 | 引导走 `/exchange-consent?source=A&target=B`；**存量用户**多半是布线晚于 consent 导致 `exchange_consents` 缺行——直接查该表按 (user, source, target) 有无行，缺则参照同租户正常用户的行补插 |
| 跨应用下拉/列表**静默为空** | 同上（发起方把 consent 错误吞成空列表） | 先查 consent 行，再查交集 scope |
| `SECRET_INVALID` / 提示「跨应用授权缺失或未开启」但布线明明在 | **App Secret 漂移**：A 后端 env 里的 secret ≠ 门户 `app_secrets` 存的哈希 | 对 A 后端 env 的 secret 求 sha256，与门户 DB `app_secrets` 该 App 的哈希比对；portal-api 侧漏配 `*_APP_SECRET` env 是常见形态 |
| `GRANT_NOT_ALLOWED` | A 的 `allowedGrants` 未含 `token_exchange` | 改 A 的清单/注册 |
| 换到令牌但调 B 全 403 `INSUFFICIENT_SCOPE` | 交集为空：A 没把 B 命名空间 scope 声明进自己 scopes，或 A 入站 TDT 本身没带 | 查 A listing 的 scopes + 入站令牌 scopes |
| 更宽 scope mint 触发 `CONSENT_REQUIRED` | mint 记录的同意被**子集 mint 收窄**过 | 先重新走 consent，再宽 mint；避免用裁剪 scopes 去 mint |

## 红线

- 撤销交换 consent 不回滚已签发的短期 TDT（自然过期），但拦后续交换——别把"撤销后还能用几分钟"当 bug 修。
- 交换永不扩权：想要更多 scope，改的是**声明**（A 的 scopes/exchangeTargets + 用户再同意），不是在交换请求里多要。
- 服务态令牌（`client_credentials`）不走 exchange 这条链——无用户上下文的服务间直调见 portal-external-app / portal-backend-app skill。
