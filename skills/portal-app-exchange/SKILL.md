---
name: portal-app-exchange
description: '跨应用调用（OAuth Token Exchange）的布线与排错。凡任务涉及让 App A 读 App B 的数据（如 组卷读题库、文件调多模态解析、任务中心读 LMS）、声明 exchangeTargets/交换白名单/consent 共授，或出现 EXCHANGE_NOT_ALLOWED / EXCHANGE_CONSENT_REQUIRED / CONSENT_REQUIRED / SECRET_INVALID / GRANT_NOT_ALLOWED / 跨应用下拉列表莫名为空 等症状时，务必先用本 skill 再动手改代码。Use for wiring or debugging cross-app token exchange: grants, whitelists, consent co-grant, scope intersection, and the classic failure modes (secret drift, missing user consent rows, consent narrowing).'
---

# portal-app-exchange · 跨应用调用（令牌交换）布线与排错

App A 代表用户调 App B = 拿一个 `aud=B` 的 TDT，走 OAuth Token Exchange，**严格不可提权**。完整机制、请求格式、consent 端点与排错决策树在 [references/exchange-reference.md](references/exchange-reference.md)（为本 skill 提炼的自包含参考，可整目录拷到任何 repo；权威源是门户仓库 `docs/SSO与App开发指引.md` §11，冲突以门户仓库为准）。


> **路径约定（先读这条，能省一次白找）**：本 skill 里出现的 `apps/…` `packages/…` `docs/…`
> `deploy/…` 这类路径**都在门户仓**。在 App 自己的 repo 里它们**不存在** —— 它们标注的是
> 「门户侧要做什么」或某段内容的出处，**不是让你去打开的文件**。找不到不是配置错误：
> 别去创建、别去全局搜、别把它当缺失依赖报出来。你需要的一切都在本 skill 的
> `references/`（自包含）。若你正在门户仓里工作，那这些路径就是可以直接打开的真实文件。
>
> ⚠️ 一个例外：`/apps/<key>/`（带前导斜杠）是**线上 URL 路径**——微应用产物的挂载点，
> 与仓内的 `apps/<key>-app/` 目录无关，别混。

## 模型一页纸（改代码前先对齐）

放行 = 同时满足五件事：

1. A→B 授权关系存在（`app_exchange_grants`）；
2. B `allowExchange = true`；
3. A ∈ B 的 `exchangeWhitelist`；
4. 用户已同意"A 代表我访问 B"（`exchange_consents`，服务端交换不带会话，consent 必须**事先**记录）；
5. 结果 scope = A 当前 TDT scopes ∩ B 声明 scopes——**发起方 A 的清单必须把 B 命名空间 scope 声明进自己的 `scopes`**（且 B ∈ A 的 `exchangeTargets`）。交集为空 = 换到的令牌调 B 全 403。

被调方 B 看到的令牌：`kind:"user"`、有 `user_id`、`azp` = A（出处归因）；B 照常四道闸鉴权、无需改代码。

## 布线一条新链路（A 读 B）

1. A 的清单：`exchangeTargets` += `B`，`scopes` += 所需 `B 命名空间.*`；
2. 门户 monorepo：`apps/api/src/db/provisioning.ts` 的 `EXCHANGE_WIRING` 加 `A: [..., "B"]`；dev 安装期 `wireExchangeTargets` / 外部 App 的 register-app 同效；
3. consent：用户首次进入 A 时 consent 门对声明的 `exchangeTargets` **共授**。布线晚于用户首次 consent 的存量用户会缺 `exchange_consents` 行——见排错；
4. 发起方代码照参考实现（门户 monorepo 的 `apps/files-server/src/lib/exchange.ts`；多目标看 `apps/library-server`）：换到的令牌按 **(目标 appKey, user)** 缓存复用——缓存 key 必须含目标 appKey（曾有"换票缓存串 App"的真实 bug）；
5. 交换失败**优雅降级**成可区分状态（`not_installed` / `unreachable` / `needs_consent`），别把「B 没装」渲染成传输错误、别把 consent 缺失静默吞成空列表。

## 排错速查（详表见 references/exchange-reference.md §6）

| 症状 | 第一嫌疑 |
| --- | --- |
| `EXCHANGE_NOT_ALLOWED` | 缺 grant / B 未开 allowExchange / A 不在白名单 |
| `EXCHANGE_CONSENT_REQUIRED` / 跨应用列表静默为空 | 该用户缺 `exchange_consents` 行（布线晚于 consent 的存量用户；查表按 (user, source, target)，缺则参照正常用户行补插） |
| `SECRET_INVALID` / 「跨应用授权缺失」但布线在 | **App Secret 漂移**：A 侧 env secret ≠ 门户 `app_secrets` 哈希（sha256 比对；A 运行环境漏配 `*_APP_SECRET` 常见） |
| `GRANT_NOT_ALLOWED` | A 的 `allowedGrants` 未含 `token_exchange` |
| 换到令牌但调 B 全 403 | scope 交集为空（A 未声明 B 命名空间 scope，或入站 TDT 没带） |
| 更宽 scope mint 报 `CONSENT_REQUIRED` | 同意被子集 mint **收窄**过——先重新走 consent 再宽 mint |

## 红线

- 撤销交换 consent 不回滚已签发的短期 TDT（自然过期），但拦后续交换——"撤销后还能用几分钟"不是 bug。
- 交换永不扩权：要更多 scope，改**声明**（A 的 scopes/exchangeTargets + 用户再同意），不是在交换请求里多要。
- 服务态令牌（`client_credentials`）不走本链路——判别标准：请求是否代表某个用户？是 → 交换；否 → 服务态直调（见 portal-external-app / portal-backend-app skill）。
