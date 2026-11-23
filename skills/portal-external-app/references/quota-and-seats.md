# 配额与席位（seatBased / seatRoles）

> 有「每租户最多能建几个 X」这类诉求时读本文。App 声明角色和用量指标，平台管理员维护套餐数值；不需要访问门户源码。

若管理的是**平台托管终端身份**，先读 [终端用户容量与边界](end-users.md)：使用租户共享的
`portal.end_users`，创建占用、归档继续占用、删除归还，不按下面两种模型重新造一份身份计数。
本文 seatBased 面向门户成员，seatRoles 面向 App 自管业务资源；两者与平台身份容量分开判断。

## 0. 红线

**外部服务不自建配额/套餐/计费。** 配额的唯一事实源是平台的套餐矩阵。对方在自己库里存一份
「本租户上限」，第一次调档就会与门户对不上，而且**没有任何东西会告诉你它们分叉了**。

**配额 ≠ 用量**：配额答「还能不能再建一个」，用量答「这段时间用了多少」。后者走 `usageReporter`
+ `usage.report`（指标注册表 = 内置 `USAGE_METRICS` ∪ manifest `usageMetrics` 声明，见 §5 与
[用量上报](usage-reporting.md)）。
计费方向另有平台级 Credit 服务，同样不自建。

## 1. 先选模型：二选一

两者都是 listing 上的标记，且**仅 backed listing** 可声明 —— end-state 得有 `serviceBaseUrl` 或
`deployDescriptor`，否则 `createListing`/`updateListing` 直接 `VALIDATION_FAILED`：「只有带独立后端
(serviceBaseUrl / 部署描述)的应用可声明席位制」。

| | 用在哪 | 谁收口 |
| --- | --- | --- |
| `seatBased: true` | 计「门户成员」用不用得了这个 App | **门户**。TDT 签发时就拦：无席位 ⇒ `SEAT_REQUIRED`，App 侧零代码 |
| `seatRoles: ["teacher","student"]` | App **自管账号/资源**（学校管理 `lms-school` 的师生号那类） | **App 自己**。门户只发数，不拦对方的建号请求 |

要平台在执行前原子预留、扣减并记账（并发不超卖、已用由平台维护），或要按套餐开关功能、按日计次，
用另一套机制：**权益（`entitlements`）**，见 §9。

## 2. 四要素分属四个事实源

「配额」不是一件事。想清楚你要动的是哪一件，再决定找谁：

| 要素 | 谁写 / 写在哪 | 要门户发版吗 |
| --- | --- | --- |
| role 存不存在（`seatRoles: ["service"]`） | **对方自己的 `app.manifest.json`** | 不用 |
| 各套餐给几个（生效值） | 平台管理员 → `PUT /api/console/seat-plans` upsert `app_seat_quotas(listing_key, plan, role)` | 不用 |
| 单租户加购 | 平台管理员 → `PUT /api/console/tenants/:id/seat-addons`（仅 pro/ultra 可加购） | 不用 |
| role 的三语标签 `SEAT_ROLE_LABELS` | 平台维护的本地化标签（联系管理员） | **要**（不加则控制台显示裸 role key，`?? role` 兜底不报错） |
| SA 的 `seats.read` | **对方 manifest `privilegedServiceScopes: [{scope, reason}]` 申请** → 「发布审核」逐条确认批准即授予 | 不用 |

⚠️ **各套餐的数值进不了 manifest** —— App 清单只有 `seatBased` / `seatRoles` 两个字段，没有任何 per-plan 数值字段。应用侧**没有任何写路径**
（manifest 没这字段 · 发布提案携带的未知字段一律进人工审而非生效 · `seats.read` 只读）。这是有意的：
数值是商务面，改它等于改可售卖档位、影响所有租户。应用侧能做的是在契约里**提议**一组数。

⚠️ **`seats.read` 是 `SERVICE_ONLY_SCOPES`**（与 `directory.provision` / `lms.taxonomy.manage` 同列）。
外部团队在 manifest 里自己写 `serviceScopes: ["seats.read"]` **会被拒收** —— 评审交付物
时看到它是**当场打回的信号**，不是配置失误。要拿它走
`privilegedServiceScopes: [{ "scope": "seats.read", "reason": "…" }]`：这是**申请**不是授予，
必进「发布审核」（最高治理档，审批人逐条勾选确认），批准后经 `registerFromManifest`
落到 SA 的服务态 scope。

## 3. 声明写在 manifest 里（不需要门户改代码）

`seatRoles` / `seatBased` 是 `AppManifest` 的一等字段，`register-app --prod` 与控制台「导入 manifest」
都经 `manifestToListingInput` → `updateListing` 落到 listing 上：

```jsonc
{
  "listingKey": "task-gateway",
  "deployDescriptor": { "image": "…" },  // 没有它(或 serviceBaseUrl) ⇒ 席位声明会被拒
  "seatRoles": ["service", "node"]
}
```

两条操作面的事实：

- 写进 manifest 后随 `publish --manifest` 提交 —— `seatRoles` 是治理档，提案进「发布审核」
  等平台批准后生效（`register-app --prod` / 控制台「导入 manifest」是运维兜底路径，同一段
  `registerFromManifest`）。
- App 自己仓库的 manifest 是开发者声明的事实源；修改后提交新提案，并核对已安装租户的同步结果。

## 4. 对接面与三态

```
POST /api/v1/seats/quota    服务态令牌(client_credentials, azp=自己的 listingKey), scope seats.read
body { roles: ["service","node"] }
→ { tenantId, plan, listingKey, roles: [{ role, quota, quotaIsDefault, addon, available }] }
```

在**建号/建资源路径**上查它并拒绝超额，就是 App 侧的全部工作量。按三态分支，**别只看数字**：

| 响应 | 含义 | 建议动作 |
| --- | --- | --- |
| `available === null` | 该租户 unlimited | 放行，不收口。**把 null 当 0 是这个契约最容易踩的一处** |
| `quotaIsDefault: true` | 平台还没配，你拿到的是兜底 | 按兜底值放行 **+ 打告警**，别静默按一个没人定过的数收口 |
| `quotaIsDefault: false` | 权威值 | 照值收口。**值可能是 0** —— 那是平台明确决定「这档不给」 |

**解析顺序**：`app_seat_quotas` 行 → `SEAT_ROLE_DEFAULTS[role]` → `UNREGISTERED_ROLE_DEFAULTS`
（各档一律 **1**）。1 的意思是「能跑通、但显然还要配」。

- 兜底**不能取 0**：0 是合法的权威配置值（`putSeatPlans` 接受 0），拿它表示「没配」就再也分不开
  「没配」与「明确不给」——而 `quotaIsDefault` 存在的全部意义就是分开这两者。
- `seatBased` 的成员席位走**另一条**兜底 `DEFAULT_PLAN_SEATS` = free 3 / pro 20 / ultra 100，别混。
- 新 role 落地后**一定要去控制台配一轮真实数值**，别让生产挂着兜底跑。

## 5. 想让控制台显示「已用 / 超额」：得上报 gauge

平台不掌握 `seatRoles` 的占用数（收口在 App 侧），所以「已用」来自 App 自己上报的日快照：
metricKey 约定 `<listingKey>.<role>-seats.allocated`（`latestRoleAllocated` 读最近一天）。
学校管理（`lms-school`）先例每个 role 报两条（`allocated` + `available`），两个 role 共 4 条。

⚠️ **硬门：一条坏记录拒收整批。** metricKey **未注册**、指标不属于上报者 azp 的命名空间、
日期/数值/维度非法……任一条不合格，上报接口就回 `ok: true` 但 `upserted: 0`，`rejected` 逐条列出
`{ index, reason }`，同批合法的记录**也不写入**。上报方必须检查 `rejected`：漏注册的症状是
控制台「已用」恒 0、`overQuota` 恒 false、加购决策没有依据，拒收理由会写明「未注册」。
指标注册表 = 内置 `USAGE_METRICS` ∪ 各 listing 的 manifest `usageMetrics` 声明：
**外部 App 在自己 manifest 的 `usageMetrics` 里声明 gauge 条目**
（`[{ key, kind: "gauge", unit: "count", label: {"zh-CN",…} }]`，key 前缀必须=listingKey、
金额单位 microAmount 不开放；治理档，批准后生效），无需门户发版。上报合同见 [用量上报](usage-reporting.md)。

## 6. 两层快照：谁读哪一层

安装时 `seatBased` / `seatRoles` 会快照进该租户的 `apps` 行。三个面读的表**不同**：

| 面 | 读哪张 |
| --- | --- |
| 平台配额矩阵 `getSeatPlans` | `marketplace_listings.seat_roles` |
| 租户加购 `getTenantAddons` | **`apps.seat_roles`（已装租户快照）** |
| `POST /api/v1/seats/quota` | **两者都不读** —— roles 来自请求体，只查 plan + 配额表 + 加购表 |

**已批准的 manifest 变更会同步到已安装租户。** 发布后核对控制台配额矩阵与租户加购面；异常交平台管理员处理，不直接修改平台数据库。

推论：**漏声明不会 fail-closed**。`/api/v1/seats/quota` 不读任何 seatRoles 声明，照样按请求体里的
role 返回兜底值。真实症状是**控制台配额矩阵与加购面不出这一行** ⇒ 平台配不了、租户加购不了
⇒ 所有租户静默跑兜底。真正 fail-closed 的是 SA 缺 `seats.read` → 403。

## 7. 三个坑

- **manifest 的 `seatRoles` 是整份覆盖，不是 union**（`updateListing`，改动还会批量传播到所有已装
  租户的 `apps` 行）。下一版 manifest 少写一个 role ⇒ 那个 role 从所有租户消失。清单字段
  「只增不减」那条规矩管的是 scopes/dependencies/exchangeTargets，**不覆盖 `seatRoles`**。
- **role 名没有格式校验，改名 = 换一个新 role**：旧 `(listing_key, plan, role)` 配额行成孤儿 ——
  全仓没有任何地方删它们，`getSeatPlans` 只按当前 `seatRoles` 出行，于是孤儿行在控制台不可见
  却还躺在库里（改回同名会「复活」旧数值），而新 role 悄悄回落兜底。**role 名定下来就别动。**
- **`SEAT_ROLE_DEFAULTS` 按 role 名索引，不带 App**（`Record<role, Record<plan, number>>`）——
  全平台共用一个 role 名命名空间，两个 App 都用 `node` 就共享同一份代码兜底。DB 里的权威值
  倒是按 `(listing_key, plan, role)` 分得开。要放代码兜底就挑个不会撞的名字。

## 8. 判定：这个数该不该进套餐表

**先问一句「你说的是配额还是用量」** —— 两者经常被混为一谈。几个面的命名空间也各不相同：

| 面 | 命名 | 例 |
| --- | --- | --- |
| scope | **下划线** | `task_gateway.submit` |
| 用量指标 `USAGE_METRICS.key` | **连字符**（= listingKey 前缀） | `task-gateway.tasks.submitted` |
| 权益 key | `<listingKey>.` 前缀 + 小写字母/数字/`._-` | `my-app.exports` |
| 席位配额 | **不是字符串键** | `(task-gateway, pro, service) → 10` |

席位配额的键空间是 `(listingKey, plan, role) → seats:int`，namespace 已由 listingKey 占掉，你要提供的只是一个**短 role 名**。

**再问「是不是真要按套餐卖」。** 只是防滥用的安全上限就别进套餐表 —— 进了就等于承诺它是可售卖
档位，以后想改口径要动所有租户的既有配额。

**然后选机制：**

| 你要的 | 用 |
| --- | --- |
| 门户成员能不能用这个 App | `seatBased` |
| 每租户能建几个账号/资源，App 自己在建号路径上判，要加购与已用水位 | `seatRoles` + gauge 上报（§4、§5） |
| 平台在执行前原子预留与扣减（并发不超卖、已用由平台记账）；按日计次；按套餐开关功能 | 权益 `entitlements`（§9） |
| 不随套餐变的上限 | App 侧产品常量 |

`seatRoles` 存的不是参数，是池子：它自带加购（`available = quota + addon`）、已占用（gauge）、超额
（`overQuota = allocated > available`）三件配套。配套不成立的东西塞进去不会报错，只会让控制台显示一行
永远「已用 0 / 未超额」、还挂着语义错的加购入口的假数据。

⚠️ 两套机制都按租户计，**没有「每个父对象」这一维**。「每个 service 最多 N 个 node」这类对所有实例一律
生效的上限：不随套餐变就做 App 常量；要随套餐卖，优先改卖租户级总量（如 `node` 池，配套全部成立）；
确实要卖密度，就把这个上限当一个 role 落，标签写清是「每个 X 的上限」而不是总量，并接受那行配套空转。
「A 服务 3 个、B 服务 10 个」那种真嵌套配额平台不支持。

## 9. 权益（entitlements）与执行前配额

和 `seatRoles` 的区别：`seatRoles` 只发数，收口与占用都在 App；权益由平台记账 —— App 执行前向平台
**预留**额度，平台在事务里判定并占用，结束后核销或释放，`used` / `reserved` 由平台维护，不需要另报 gauge。

**声明**：manifest `entitlements`（治理档，进「发布审核」；批准后这些 key 出现在套餐与租户额度的配置面）：

```jsonc
"entitlements": [
  { "key": "my-app.projects", "kind": "resource",   "unit": "count", "window": "none",    "name": "项目数", "nameI18n": { "en": "Projects" } },
  { "key": "my-app.exports",  "kind": "consumable", "unit": "count", "window": "utc_day", "name": "每日导出次数" },
  { "key": "my-app.advanced", "kind": "feature",    "unit": "count", "window": "none",    "name": "高级功能" }
]
```

- 最多 32 条；`key` 必须以 `<listingKey>.` 开头，其余为小写字母/数字/`._-`（≤64）；`name` ≤200，`nameI18n` 只放 `en` / `zh-TW`，`sort` 为整数。
- `kind`：`feature` 开关（无额度，不能预留）；`resource` 活跃占用数（无窗口，资源删掉时归还）；`consumable` 周期消耗。
  `feature` / `resource` 的 `window` 只能是 `none`；`consumable` 用 `utc_day`（按 UTC 日重置）或 `grant`（按租户的购买来源计量，见下文）。
- `unit`：`count`；或 `microAmount`（金额，1e-6 主单位），只能是 `consumable` + `utc_day`，且必须带 ISO 4217 `currency`。
- **只增不减**：已注册 key 的 `kind / unit / window / currency` 不能改（改语义要换新 key，否则发布被拒）；删掉声明也不回收既有 key 与账本。`name / nameI18n / sort` 可改。

**数值不在清单里**：平台管理员按套餐配置开关与上限（`0` = 明确禁止，`null` = 无限），并可对单租户覆盖或加购
（仅专业版 / 尊享版可加购）；租户管理员可自设收紧。没配置的 key 是 `unconfigured`，**一律不放行**。

**读**：服务态 `GET /api/v1/entitlements?keys=a,b`（`entitlements.read`，≤50 个，缺省返回全部）→
`{ tenantId, plan, rows }`。每行含 `enabled`、`limit`（null = 无限）、`used`、`reserved`、`available`、
`enforcementMode`（`unconfigured` / `disabled` / `enforced` / `unlimited` / `over_limit`）、`source`、`periodStart`。
功能开关以 `enabled` 为准；`over_limit` 表示上限已调到占用之下，只禁新增、已有资源照常。
**读到余额不等于能扣**：扣额只认下面的预留。

**预留与结算**（服务态 `entitlements.consume`，只能操作本 App 自己的 key，别人的 key 回 403）：

| 端点 | body | 作用 |
| --- | --- | --- |
| `POST /api/v1/quota/reserve` | `{ key, operationKey, units, expiresInSeconds, resourceRef?, begin? }` | 预留，回 `{ operation, duplicate }`；`begin: true` 预留后直接进入 executing |
| `POST /api/v1/quota/operations/:id/begin` | — | `reserved → executing`，即将执行外部动作 |
| `POST /api/v1/quota/operations/:id/commit` | `{ actualUnits? }` | 核销；`actualUnits` 为 0..预留量，缺省等于预留量，差额归还 |
| `POST /api/v1/quota/operations/:id/release` | `{ reason? }` | 释放，占用全部归还 |
| `POST /api/v1/quota/operations/:id/uncertain` | `{ reason }`（1–200 字） | `executing → uncertain`：外部动作可能已发生但结果未知，占用保留 |
| `GET /api/v1/quota/operations?key=&operationKey=` | — | 按幂等键回读，无则 `data: null` |
| `GET /api/v1/quota/operations/open?state=&limit=&cursor=&staleOnly=` | — | 未终结操作分页（`limit` ≤500），回 `{ items, nextCursor }` |

- `operationKey`（1–128 位，字母/数字/`._:-`）是幂等键：同 key 同请求回原操作且 `duplicate: true`，内容不同 `QUOTA_OPERATION_CONFLICT`。幂等判据由平台按归一后的请求计算。
- `expiresInSeconds` 必填，不得超过平台上限；未开始的预留到期自动 `expired` 并归还，过期后 begin / commit 回 `QUOTA_RESERVATION_EXPIRED`。**executing 不会自动过期**，超时只进入平台对账视图。
- 额度不足 `ENTITLEMENT_QUOTA_EXCEEDED`（`details` 带 `limit / used / reserved / requested`）；未配置 `ENTITLEMENT_UNCONFIGURED`；已关闭 `ENTITLEMENT_DISABLED`。平台没开放远程配额时 reserve 也回 `ENTITLEMENT_UNCONFIGURED`。
- `resource`：同一 `resourceRef` 同时只能有一个活跃占用（重复回 `QUOTA_OPERATION_CONFLICT`）；已核销的占用在资源删除后用 release 归还。`consumable` 核销后不能 release 冲回。
- 计数权益可以不调 begin，直接从 reserved commit。不属于你或不存在的操作 ID 一律 `NOT_FOUND`。
- 重启后先按 `operationKey` 回读，再用 `open` 列表对账，把卡在 executing / uncertain 的操作凭证据 commit 或 release。
- 金额（`microAmount`）更严：reserve 必须带与定义一致的 `currency`（可带 `costBasisRef`），**必须先 begin 才能 commit**；commit 带 `currency`、`evidenceRef` 与实际费用 `actualUnits`，实际可超过预留但要给 `overrunReason`（该 key 的新执行随即冻结，等平台审查）；已开始的金额操作 release 须带 `reason` + `evidenceRef`，已结算的不能 release。

**`window: "grant"` 的 key 走另一套接口**：`GET /api/v1/grant-entitlements` 与 `/api/v1/grant-quota/reserve`、
`/grant-quota/operations/:id/{begin,commit,release}`、`/grant-quota/operations`、`/grant-quota/operations/open`（`limit` ≤200），形状同上，区别：

- 只记条数，不收 `currency`；没有 `/uncertain`，改用 release 带 `uncertain: true`；按幂等键回读返回 `{ operation }`。
- 额度来自租户的各个购买来源：读接口逐来源给出 `quantity / used / reserved / remaining / windowStart / windowEnd / nextResetAt`，`totalRemaining` 只供展示。界面要**逐来源展示**，否则用户分不清「永久有效的包」和「下个周期才恢复的额度」。
- 没有可用来源 `ENTITLEMENT_UNCONFIGURED`；租户计费停用（`suspended: true`）时新预留回 `ENTITLEMENT_SUSPENDED`，读与在途结算照常。
- 两套接口互不相认：非 grant key 调 `/grant-*`、或在 `GET /entitlements` 里点名 grant key，回 `ENTITLEMENT_PROTOCOL_UNSUPPORTED`；grant key 调 `/quota/reserve` 按未注册处理，回 `ENTITLEMENT_UNCONFIGURED`。

`entitlements.read` 与 `entitlements.consume` 都是服务专用 scope，经 manifest `privilegedServiceScopes` 申请（同 `seats.read`）。

## 10. 落地清单

**外部 App（manifest 路，除标签外无需门户发版）**

1. manifest 加 `seatRoles: [...]`（治理档）
2. manifest 加 `usageMetrics` gauge 条目（每 role 两条：`.allocated` / `.available`；治理档）
3. manifest 加 `privilegedServiceScopes: [{ "scope": "seats.read", "reason": "…" }]`（申请，审批逐条确认后授予）
4. （可选）门户 `SEAT_ROLE_LABELS` 加三语标签 —— 唯一仍要门户发版的一件；不加则控制台显示裸 role key
5. 批准部署后，平台管理员在控制台按 `App × role × 套餐` 配一轮真实数值

**外部 App（权益路）**

1. manifest 加 `entitlements: [...]`（治理档）
2. manifest `privilegedServiceScopes` 申请 `entitlements.read` / `entitlements.consume`
3. 批准后，平台管理员按套餐配置开关与上限；没配之前一律 `unconfigured`、不放行
4. 执行路径：reserve →（begin）→ 执行 → commit / release；失败重试沿用同一个 `operationKey`

**平台管理员**：维护套餐/加购数值与可选本地化标签；App 团队不能用发布令牌修改这些配置。

**App 侧**

1. 建号/建资源路径上调 `POST /api/v1/seats/quota`，按 §4 三态分支
2. 每日上报 gauge（`allocated` = 当前实际数，`available` = 配额 + 加购；unlimited 不报）
3. 形状类上限（每个实例最多几个子对象）留作 App 侧常量
