# 配额与席位（seatBased / seatRoles）

> 有「每租户最多能建几个 X」这类诉求时读本文。提炼自门户仓 `apps/api/src/modules/seats/`、
> `packages/shared/src/constants.ts`、`apps/api/src/db/provisioning.ts`（门户仓文件，App 自己的
> repo 里没有；本文件已自包含）。冲突时以门户仓库为准。

## 0. 红线

**外部服务不自建配额/套餐/计费。** 配额的唯一事实源是平台的套餐矩阵。对方在自己库里存一份
「本租户上限」，第一次调档就会与门户对不上，而且**没有任何东西会告诉你它们分叉了**。

**配额 ≠ 用量**：配额答「还能不能再建一个」，用量答「这段时间用了多少」。后者走 `usageReporter`
+ `usage.report`（指标注册表 = 内置 `USAGE_METRICS` ∪ manifest `usageMetrics` 声明，见 §5）。
计费方向另有平台级 Credit 服务，同样不自建。

## 1. 先选模型：二选一

两者都是 listing 上的标记，且**仅 backed listing** 可声明 —— end-state 得有 `serviceBaseUrl` 或
`deployDescriptor`，否则 `createListing`/`updateListing` 直接 `VALIDATION_FAILED`：「只有带独立后端
(serviceBaseUrl / 部署描述)的应用可声明席位制」（`market/service.ts` 的 `assertSeatBasedEligible`）。

| | 用在哪 | 谁收口 |
| --- | --- | --- |
| `seatBased: true` | 计「门户成员」用不用得了这个 App | **门户**。TDT 签发时就拦：无席位 ⇒ `SEAT_REQUIRED`，App 侧零代码 |
| `seatRoles: ["teacher","student"]` | App **自管账号/资源**（sms 的师生号那类） | **App 自己**。门户只发数，不拦对方的建号请求 |

## 2. 四要素分属四个事实源

「配额」不是一件事。想清楚你要动的是哪一件，再决定找谁：

| 要素 | 谁写 / 写在哪 | 要门户发版吗 |
| --- | --- | --- |
| role 存不存在（`seatRoles: ["service"]`） | **对方自己的 `app.manifest.json`**（清单仍在门户仓的 App 才是 `LISTING_DEFS`） | 不用 |
| 各套餐给几个（生效值） | 平台管理员 → `PUT /api/console/seat-plans` upsert `app_seat_quotas(listing_key, plan, role)` | 不用 |
| 单租户加购 | 平台管理员 → `PUT /api/console/tenants/:id/seat-addons`（仅 pro/ultra 可加购） | 不用 |
| role 的三语标签 `SEAT_ROLE_LABELS` | 门户 `packages/shared/src/constants.ts` | **要**（不加则控制台显示裸 role key，`?? role` 兜底不报错） |
| SA 的 `seats.read` | **对方 manifest `privilegedServiceScopes: [{scope, reason}]` 申请** → 「发布审核」逐条确认批准即授予（`SA_DEFS.serviceScopes` 降为种子/运维兜底，union top-up 不回滚审批授予） | 不用 |

⚠️ **各套餐的数值进不了 manifest，也进不了 `LISTING_DEFS`** —— `AppManifest` 与 `ListingDef`
都只有 `seatBased` / `seatRoles` 两个字段，没有任何 per-plan 数值字段。应用侧**没有任何写路径**
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
- 出仓四件（knowledge / task-gateway / omni-parser / pagebuilder）已**不在** `LISTING_DEFS`
  里 —— 清单唯一事实源就是对方仓的 manifest，不存在双事实源问题；**别把定义加回门户**
  （verify-split「清单零残留」棘轮看守）。
- 仓内 App 在 `LISTING_DEFS` 加 `seatRoles` **不用 bump version**：它走「代码权威
  字段」通道，每次部署都对齐，跟版本治理无关。

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
sms 先例每个 role 报两条（`allocated` + `available`），两个 role 共 4 条。

⚠️ **硬门**：`ingestUsage` 对**未注册**的 metricKey **拒收该条记录**，且指标的
`appKey` 必须等于上报者的 azp。漏注册的症状是上报接口返 200 但记录进 `rejected`，
控制台「已用」恒 0、`overQuota` 恒 false、加购决策没有依据（拒收理由现在会显式指出
「未注册 —— 请在 usageMetrics 里声明」，不再是无提示的静默失真）。
指标注册表 = 内置 `USAGE_METRICS` ∪ 各 listing 的 manifest `usageMetrics` 声明：
**外部 App 在自己 manifest 的 `usageMetrics` 里声明 gauge 条目**
（`[{ key, kind: "gauge", unit: "count", label: {zh,…} }]`，key 前缀必须=listingKey、
金额单位 microAmount 不开放；治理档，批准后生效）——不再需要门户发版；
仓内 App 仍在 `packages/shared/src/usage.ts` 登记。

## 6. 两层快照：谁读哪一层

安装时 `seatBased` / `seatRoles` 会快照进该租户的 `apps` 行。三个面读的表**不同**：

| 面 | 读哪张 |
| --- | --- |
| 平台配额矩阵 `getSeatPlans` | `marketplace_listings.seat_roles` |
| 租户加购 `getTenantAddons` | **`apps.seat_roles`（已装租户快照）** |
| `POST /api/v1/seats/quota` | **两者都不读** —— roles 来自请求体，只查 plan + 配额表 + 加购表 |

**存量租户不需要人工回刷**，两条路都自带传播：

- `LISTING_DEFS` 路：`syncFromSource` 对 listing 层与已装 `apps` 行**各打一次**
  `codeAuthoritativePatch`（union 语义），随 `bootstrap:prod` 自动到达。
- manifest 路：`updateListing` 直接批量 update 所有已装 `apps` 行，`registerFromManifest` 之后
  还会再跑一次 `syncInstalledAppsFromListing`。

推论：**漏声明不会 fail-closed**。`/api/v1/seats/quota` 不读任何 seatRoles 声明，照样按请求体里的
role 返回兜底值。真实症状是**控制台配额矩阵与加购面不出这一行** ⇒ 平台配不了、租户加购不了
⇒ 所有租户静默跑兜底。真正 fail-closed 的是 SA 缺 `seats.read` → 403。

## 7. 三个坑

- **manifest 的 `seatRoles` 是整份覆盖，不是 union**（`updateListing`，改动还会批量传播到所有已装
  租户的 `apps` 行）。下一版 manifest 少写一个 role ⇒ 那个 role 从所有租户消失。清单字段
  「只增不减」那条规矩管的是 scopes/dependencies/exchangeTargets，**不覆盖 `seatRoles`**。
  （走 `LISTING_DEFS` 那条是 union，不会静默删。）
- **role 名没有格式校验，改名 = 换一个新 role**：旧 `(listing_key, plan, role)` 配额行成孤儿 ——
  全仓没有任何地方删它们，`getSeatPlans` 只按当前 `seatRoles` 出行，于是孤儿行在控制台不可见
  却还躺在库里（改回同名会「复活」旧数值），而新 role 悄悄回落兜底。**role 名定下来就别动。**
- **`SEAT_ROLE_DEFAULTS` 按 role 名索引，不带 App**（`Record<role, Record<plan, number>>`）——
  全平台共用一个 role 名命名空间，两个 App 都用 `node` 就共享同一份代码兜底。DB 里的权威值
  倒是按 `(listing_key, plan, role)` 分得开。要放代码兜底就挑个不会撞的名字。

## 8. 判定：这个数该不该进套餐表

**先问一句「你说的是配额还是用量」** —— 两者经常被混为一谈。三个面的命名空间也各不相同：

| 面 | 命名 | 例 |
| --- | --- | --- |
| scope | **下划线** | `task_gateway.submit` |
| 用量指标 `USAGE_METRICS.key` | **连字符** | `task-gateway.tasks.submitted` |
| 配额 | **不是字符串键** | `(task-gateway, pro, service) → 10` |

配额的键空间是 `app_seat_quotas` 的唯一索引 `(listing_key, plan, role) → seats:int`。namespace 已经
由 `listingKey` 占掉，你要提供的只是一个**短 role 名**。（所以「entitlements 的 key 怎么命名」是个
不成立的问题 —— 平台没有 entitlements 这个东西。）

**再问「是不是真要按套餐卖」。** 只是防滥用的安全上限就别进套餐表 —— 进了就等于承诺它是可售卖
档位，以后想改口径要动所有租户的既有配额。

**最后看它像不像一个「池子」。** `app_seat_quotas` 存的不是参数，是池子：它自带加购
（`available = quota + addon`）、已占用（gauge）、超额（`overQuota = allocated > available`）三件配套。
配套不成立的东西塞进去不会报错，只会让控制台显示一行永远「已用 0 / 未超额」、还挂着语义错的
加购入口的假数据。

⚠️ 别拿「唯一键里没有『每个父对象』这一维」当判据 —— 一个**对所有实例一律生效**的上限就是
一个整数，`(app, plan, role) → 5` 存得下。只有「A 服务 3 个、B 服务 10 个」那种**真嵌套**配额才卡在
键的形状上。平台**不**为此新建通用 entitlements 表：真功能要排期，代价是平台从此有两套配额模型。

## 9. 案例：task-gateway 的 `max_services` / `max_nodes_per_service`（2026-08-19）

对方提议拿它们当「entitlements 的 key」问门户怎么映射到套餐表。逐条判定：

- **`max_services`**（每租户最多几个 service）→ **能落**，就是一个 role。按 §2 的事实源表分工：
  role 由对方 manifest 声明 · 数值平台在控制台配 · gauge 由对方 manifest `usageMetrics` 声明 ·
  `seats.read` 由对方 manifest `privilegedServiceScopes` 申请（审批授予）· 门户侧只剩
  `SEAT_ROLE_LABELS` 三语标签一件（可选，不加显示裸 role key）。
- **`max_nodes_per_service`**（每个 service 最多几个 node，对所有 service 一律生效）→
  **能落，但配套三件空转**（§8）。按「随不随套餐变」三分：
  **(a)** 不随套餐变（纯防滥用）→ App 侧产品常量，别进表；
  **(b)** 随套餐变、卖的是**总容量** → 租户级 `node` 池（`seatRoles: ["service","node"]`），三件配套
  全部成立，门户看得见真实水位、加购这条商业路径也保住 —— **默认推荐这条**；
  **(c)** 随套餐变、且确实要卖「节点密度」→ 把这个一律生效的上限当一个 role 落，标签必须说清是
  「每个服务的上限」而不是总量（否则平台管理员会当成总量配错），接受那行的配套空转。

## 10. 落地清单

**外部 App（manifest 路，除标签外无需门户发版）**

1. manifest 加 `seatRoles: [...]`（治理档）
2. manifest 加 `usageMetrics` gauge 条目（每 role 两条：`.allocated` / `.available`；治理档）
3. manifest 加 `privilegedServiceScopes: [{ "scope": "seats.read", "reason": "…" }]`（申请，审批逐条确认后授予）
4. （可选）门户 `SEAT_ROLE_LABELS` 加三语标签 —— 唯一仍要门户发版的一件；不加则控制台显示裸 role key
5. 批准部署后，平台管理员在控制台按 `App × role × 套餐` 配一轮真实数值

**仓内 App（LISTING_DEFS 路，随门户发版）**

1. `LISTING_DEFS[<key>]` 加 `seatRoles: [...]`
2. `SEAT_ROLE_LABELS` 加三语标签；`SEAT_ROLE_DEFAULTS` 加各档默认值（可选；不加则一律兜底 1）
3. `USAGE_METRICS` 加 gauge 条目（每 role 两条）
4. `SA_DEFS[<key>].serviceScopes` 加 `seats.read`
5. 部署后控制台配数值

**App 侧**

1. 建号/建资源路径上调 `POST /api/v1/seats/quota`，按 §4 三态分支
2. 每日上报 gauge（`allocated` = 当前实际数，`available` = 配额 + 加购；unlimited 不报）
3. 形状类上限（每个实例最多几个子对象）留作 App 侧常量
