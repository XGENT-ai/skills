# USAGE-2 · XGENT.ai Portal · 席位制 + 套餐规范化(计量二期)

> 站在 `goal/USAGE.md`(计量底座,已全量落地并验证)、`goal/ACL.md`(角色/失效机制)、`goal/PLAN-3.md`(应用市场/listing)之上。
> 本期交付三件事:**① 套餐(plan)从自由文本规范化为四档枚举**;**② 应用席位制**——App 在 manifest 声明席位制,平台管理员按套餐配额,租户管理员从本租户成员中分配席位,只有拿到席位的成员才能使用该 App;**③ 席位进入用量统计**——新增「可用席位 / 已分配席位」两个 gauge 指标,自动出现在 `/admin/usage`、`/console/usage` 与 CSV。
> 收尾时同步更新 [`docs/用量统计服务接入指引.md`](../docs/用量统计服务接入指引.md)(本文档只做计划,交付在 M5)。

---

## 0. 本期范围与决策

### 0.1 最重要边界:席位是「访问开关 + 计量对象」,不是「计费」

- 本期回答的问题是:**「谁能用某个席位制 App」+「某租户某天某 App 有多少可用/已分配席位」**。分配、收口、配额、计量、展示,口径可审计。
- 不回答「加购席位收多少钱」:无价格、无订单、无支付。pro/ultra 的「付费增加席位」本期以**平台管理员手工加购**(console 直接改额度)作为付费流程的占位——数据模型(`tenant_seat_addons`)就是未来支付成功后写入的同一张表,支付引擎下一期**只新增**下单/回调,不回头改席位模型。
- 与 USAGE 一期同判据:计费落地时只加价格与账期,不修改本期任何契约。

### 0.2 核心决策表

| # | 决策点 | 选择 | 含义 |
|---|--------|------|------|
| 1 | 套餐取值 | **四档枚举 `free / pro / ultra / unlimited`**,存于 `packages/shared`(key + 三语 label),`tenants.plan` 列类型不变(text)但 API 层强校验 | 免费版/专业版/尊享版/无限版;console 创建/编辑从自由文本改为下拉;存量值迁移映射(§3.2) |
| 2 | 席位制声明 | **listing 级声明**:市场 listing 常量 + `marketplace_listings.seat_based` 列,安装时快照到 `apps.seat_based`;**seatBased 仅允许 backed listing**(有 serviceBaseUrl/deployDescriptor,安装 appKey 恒等于 listingKey、不漂移) | 席位制是 App 产品属性(同 `dependencies` 先例);纯前端/内容 listing 的 appKey 可能漂移 `-2`(`freeAppKey`),声明校验直接禁止其席位化,消除 appKey/listingKey 漂移隐患;租户自建 App(`/admin/apps/new`)恒为非席位制 |
| 3 | 配额模型 | **`(listingKey, plan) → seats` 平台级配置表** + 代码默认值兜底;`unlimited` 不入表、引擎直接放行 | 平台管理员在 console 配矩阵;没配过的组合用代码默认(free=3 / pro=20 / ultra=100,可改);unlimited 无上限无需存数 |
| 4 | 加购(付费占位) | **`tenant_seat_addons(tenantId, listingKey, extraSeats)`**,本期仅平台管理员在 console 租户详情页手工维护 | 可用席位 = 套餐配额 + 加购;free 套餐 API 层拒绝加购(§0.3d);未来支付成功即写这张表 |
| 5 | 分配模型 | **`app_seat_assignments(tenantId, listingKey, userId)`**,租户管理员在 `/admin/apps/:appId` 新「席位」Tab 分配/回收 | 维度统一用 listingKey(与配额/加购/指标同一稳定键,决策 2 保证它 = 已装 appKey);唯一键 (tenant, listingKey, user);分配/回收写审计日志 |
| 6 | 收口点 | **门户单点收口**:`visibleAppRows()`(应用中心/导航/深链) + **用户态 TDT 共同签发点 `mintForApp()`**(host mint、authorization_code、refresh_token 三条直用签发路径汇于此;token_exchange **显式不加席位门**,§0.3b) + 席位回收 `bumpUserVersion`(令牌/自省失效) | 独立后端(spms-server 等)**零改造**——没席位的人签不出直用 TDT,已发令牌靠 uv bump 失效;只在 `/api/tokens/mint` 路由层加门会漏掉授权码/刷新两条路径,必须做在共同签发点 |
| 7 | 管理员豁免 | **不豁免**:租户管理员「管理席位」不占席位,「使用 App」同样要给自己分配席位 | 计量诚实优先——席位数将来直接对账账单,任何隐性豁免都是对账噪音 |
| 8 | 席位指标 | 新增 `portal.app-seats.available` / `portal.app-seats.allocated`(gauge, dims=`{app}`),门户进程内直写(扩展现有 seats worker) | 席位数据在 portal DB,沿用 §3.4/直写通道;`portal.seats.total`(成员数)保持不变 |

### 0.3 关键取舍(显式记录而非默默处理)

- **a. 席位挂在门户,不挂在各 App。** 分配关系、配额、加购全部落 portal DB:席位的本质是「租户 × App × 成员」的平台级关系,和安装(`apps`)、可见性(`app_visibility`)、授权(`consents`)同层。放各 App 自己库会导致每个席位制 App 重复实现配额与回收,且门户无法在铸币前拦截。代价:App 内看不到「谁有席位」——不需要,能进来的都有席位。
- **b. 收口选「签发门 + 可见性过滤」双保险,不改自省协议;exchange 显式不加席位门。** 用户态 TDT 有四条签发路径:host mint(`POST /api/tokens/mint`)、`authorization_code`、`refresh_token`——这三条全部汇到共同签发点 `mintForApp()`(`modules/token/service.ts`);以及 `token_exchange` 走 `exchange()`。席位门**只做在 `mintForApp()`**(席位制 App 且用户无分配行 → 业务错误 `SEAT_REQUIRED`,顺序在安装/状态检查之后、consent/scope 收窄之前),只改 `/api/tokens/mint` 路由层会漏掉授权码/刷新两条路径。`exchange()` **不加席位门,是决策不是遗漏**:token_exchange 只能由源 App 后端持自己的 client_secret 发起(用户无法自助调用),语义是「App A 以用户身份消费 B 的数据」——那是 A 的功能面,不是用户直用 B;它已有独立治理(exchange grant 白名单 + consent 共授 + scope 收窄)。若给 exchange 加席位门,任何被依赖 App(lms/qbank/files)一旦席位化,会立刻打断所有下游 App 的既有功能(sms 字典下拉、exam 选题),违背「独立后端零改造」。残余风险(无席位用户经 A 的功能间接触达 B 数据)有界且受 A 自身席位/权限约束,显式接受;将来某席位制 App 确需堵死间接面时,再给 listing 预留 `seatGateOnExchange` 声明(本期不实现)。`GET /api/apps` 的 `visibleAppRows()` 对席位制 App 叠加席位过滤(应用中心、App 导航、深链同源,一处改全生效——实现时确认导航数据源确实同源,若有旁路一并收口)。已发令牌:回收席位时 `bumpUserVersion(userId)`,下游自省缓存(aclStamp 含 uv)与令牌校验(`isTdtRevoked` 比对 uv)自然失效——复用 ACL 既有失效机制,不新增协议。**注意既有 gotcha:bumpUserVersion 首次 bump 是 no-op 语义陷阱(SMS-2 踩过),verify 必须覆盖「回收后旧令牌立即失效」,且四条签发路径逐条断言(前三条拒、exchange 通作对照)。**
- **c. 席位与 appVisibility 是 AND 关系,不合并。** `app_visibility` 回答「谁能看见这个 App 存在」(租户管理员维护的展示策略),席位回答「谁被开通使用」(配额受套餐约束、进计量)。席位制 App 的有效访问 = 可见 ∧ 有席位。合并两者会把「计量对象」和「展示策略」搅在一起,将来计费口径说不清。
- **d. 超配额状态 = 只堵增量,不自动回收。** 两个来源会造成 已分配 > 可用:套餐降级、存量升级回填(§0.3e)。此时**不自动踢人**(破坏性动作必须人做),而是:新分配一律拒绝(`SEAT_QUOTA_EXCEEDED`),admin/console 界面显著标出超配额,由租户管理员自行回收到配额内。free 用满同理——只堵新增。加购方向的约束:free 套餐的加购在 API 层直接拒绝(`PLAN_NOT_ELIGIBLE`),不是靠前端藏按钮。
- **e. 存量升级全员回填(grandfather),不一刀切断。** spms(研发项目管理)转为席位制的迁移瞬间,已装该 App 的租户里所有 active 成员**自动获得席位**——否则生产升级当场把正在用的人锁在门外。回填可能超配额,落入 §0.3d 的超配额状态,由租户管理员事后收敛。回填只发生在迁移脚本里,新装 App 不自动分配(显式分配是常态,seed 的演示租户除外)。
- **f. 成员失效即释放席位,抽单一助手函数接入每个失效路径。** `modules/seats` 提供 `releaseSeatsForMember(tenantId, userId)`(删该用户本租户全部席位行 + uv bump + 审计),接入点显式列举而非「实现时找」:成员停用与成员移除(`modules/admin/users.ts` 的两个状态翻转处),实现期再审计一遍其他会终结成员身份的路径(目录 API、全局用户禁用等),有则一并接入并记录在验收清单。席位指向的是「租户的活人」,留着幽灵席位会虚占配额、污染计量。转校/重新加入需要重新分配。
- **g. unlimited 不上报 `available` 指标,展示层不得把 null 当 0。** gauge 表达不了 ∞:报 0 是错的、报 allocated 是误导。unlimited 租户只报 `allocated`,`available` 缺行即「无上限」。注意 usage summary 对目录内指标**总会返回一项**(空 gauge 是 `last:null`,`modules/usage/service.ts` 现状),所以「缺行」到了读侧长成 null——前端必须结合租户 plan(或 seats API)把 null 渲染成「∞ / 无上限」,而不是 0 或空态。capped 套餐两个指标都报齐;席位制 App 装了但一个人没分配,`allocated` 显式报 0(沿用「清空也要报零」纪律)。
- **h. 限流档位跟着新套餐走,但 unlimited ≠ 不限流。** `PLAN_RATE_LIMITS` 重定键:free=120 / pro=300 / ultra=600 / unlimited=1200(req/min)。限流是传输层保护不是业务配额,无限版指「席位无上限」,不是「可以打挂网关」。旧键(标准版/专业版/旗舰版/standard/pro/enterprise)迁移后不再出现,直接删除;`DEFAULT_RATE_LIMIT=120` 兜底保留。
- **i. seatBased 变更的传播。** listing 的 `seat_based` 在安装时快照到 `apps.seat_based`;若日后 listing 翻转该标记,发布流程同步 UPDATE 所有已装租户的 apps 行(一次性批量,同 aclManifest 快照的维护思路)。非市场自建 App 无 listing,恒 false。
- **j. 管理面直签令牌豁免席位门(显式声明,不是漏网)。** `GET /api/admin/apps/:appKey/files-token` 与 `.../llm-gateway-token`(`modules/apps/index.ts`)是租户管理员配置 App 后端专用的短时(600s)直签 TDT,不走 mint/consent——它们属于「管理」而非「使用」,按决策 7 的同一口径**豁免席位门**(管理员配置存储/网关不应要求先给自己发席位)。豁免以直签点白名单形式存在,verify 断言:无席位的管理员仍能取管理令牌,且这两个端点保持 assertAdmin 收口;未来新增管理面直签点必须显式加入白名单并写审计,防止豁免面静默扩大。

### 0.4 明确不在本期

- **支付/订单/账单**:加购席位的付费流程(下单、支付回调、发票)。本期 console 手工加购即数据终态,支付引擎将来只是它的另一个写入方。
- **席位申请/审批流**:成员自助申请席位、管理员审批。本期只有管理员主动分配。
- **席位有效期/临时席位/席位转让**:分配即长期有效,回收是唯一出口。
- **按席位计价**(seat × 单价进账单):属于计费期,本期只保证 `usage_daily` 里的席位口径可直接计费。
- **appVisibility 机制改造**:保持现状,与席位 AND 组合(§0.3c)。
- **成员维度的席位使用率**(有席位但从不用):数据已够(llm-gateway 等有 per-user 明细),分析页面另说。
- **非 spms App 的席位化**:本期只把 spms(研发项目管理)声明为席位制作为首个案例;其他 App 将来改一行 listing 声明即接入,零平台改动。

---

## 1. 拓扑与改造面(无新工作区、无新端口、无新服务)

```
packages/shared               # 改造
  src/constants.ts            #   PLANS 四档枚举 + 三语 label + PLAN_RATE_LIMITS 重定键 + 席位默认配额
  src/usage.ts                #   USAGE_METRICS += portal.app-seats.available / .allocated(dims: app)
  src/dto.ts                  #   席位相关 DTO(配额/分配/加购) + MarketplaceListingDTO/ListingInput += seatBased

apps/api                      # 门户(改造)
  src/db/schema.ts            #   marketplace_listings.seat_based、apps.seat_based、
                              #   app_seat_quotas、tenant_seat_addons、app_seat_assignments
  src/db/migrations/00xx_*.sql#   手写迁移 ×2(套餐规范化数据迁移 / 席位表 + 回填)
  src/modules/seats/          #   新模块:配额解析 + 分配/回收 + 加购 + releaseSeatsForMember(单一实现,admin/console 路由共用)
  src/modules/apps/service.ts #   visibleAppRows() 叠加席位过滤
  src/modules/token/service.ts#   mintForApp() 席位门(SEAT_REQUIRED,收口 host mint/授权码/refresh 三路径;exchange 不加门,§0.3b)
  src/modules/market/         #   manifest.ts 解析 + service.ts listingDTO()/developerFields()/安装快照 += seatBased
  src/modules/platform/       #   plan 强校验(service 层 createTenant/updateTenant,console 与 x-platform-key 两入口共用)
                              #   + 席位配额矩阵 + 租户加购路由
  src/modules/admin/users.ts  #   成员停用/移除两处接入 releaseSeatsForMember
  src/modules/usage/worker.ts #   seats 快照扩展:按席位制 App 上报 available/allocated
  src/db/seed.ts              #   演示租户新套餐值 + spms seatBased + 演示席位分配
  scripts/verify-seats.ts     #   新 verify(纳入 verify:all)
  scripts/verify-usage.ts     #   扩展:席位指标断言

apps/web                      # 前端(改造)
  pages/console/Console.tsx / TenantDetail.tsx   # plan 下拉 + label 展示 + 加购编辑
  pages/console/MarketListingForm.tsx            # listing 表单 += seatBased 开关(仅 backed listing 可勾)
  pages/console/ConsolePlans.tsx(新)             # /console/plans 套餐管理(配额矩阵)
  pages/admin/AdminApps.tsx(+App 详情)           # 席位 Tab(分配/回收/配额状态)
  pages/admin/AdminUsage.tsx / usage/shared.tsx  # 席位指标展示
  MicroAppHost / 应用中心                         # 无席位的兜底态(正常情况下 App 已被过滤,深链兜底)
  locales/*                                      # 三语(plan label 复用同一份)

docs/用量统计服务接入指引.md   # M5 增补(席位指标 + 目录变更)
docs/SSO与App开发指引.md 或应用接入文档 # M5 增补 seatBased 声明说明
```

独立后端(spms-server / lms-server / …)**零改造**(§0.2 决策 6)。

---

## 2. 数据模型(portal DB)

### 2.1 套餐常量(`packages/shared/src/constants.ts`)

```ts
export const PLANS = ["free", "pro", "ultra", "unlimited"] as const;
export type PlanKey = (typeof PLANS)[number];
export const PLAN_LABELS: Record<PlanKey, { zh: string; en: string; tw: string }> = {
  free:      { zh: "免费版", en: "Free",      tw: "免費版" },
  pro:       { zh: "专业版", en: "Pro",       tw: "專業版" },
  ultra:     { zh: "尊享版", en: "Ultra",     tw: "尊享版" },
  unlimited: { zh: "无限版", en: "Unlimited", tw: "無限版" },
};
export const PLAN_RATE_LIMITS: Record<PlanKey, number> = { free: 120, pro: 300, ultra: 600, unlimited: 1200 };
// 席位制 App 的代码默认配额(平台未配置时兜底);unlimited 无上限、不需要数
export const DEFAULT_PLAN_SEATS: Record<Exclude<PlanKey, "unlimited">, number> = { free: 3, pro: 20, ultra: 100 };
```

- `tenants.plan` 列保持 text(不上 pg enum——将来加档不必迁移),但**所有写入口**(console create/update、seed)校验 `PLANS` 集合,schema 默认值 `standard` → `free`。
- 所有展示点(console 列表/详情、`/admin/settings`、MeDTO 消费处)从裸值改为 `PLAN_LABELS` 三语渲染。

### 2.2 席位三张表 + 两个快照列

```ts
// listing 声明(市场侧)
seatBased: boolean("seat_based").notNull().default(false),   // += marketplace_listings
// 安装快照(租户侧,收口读这里;自建 App 恒 false)
seatBased: boolean("seat_based").notNull().default(false),   // += apps

// ① 平台级配额:套餐 × App(unlimited 不入表)
export const appSeatQuotas = pgTable("app_seat_quotas", {
  id: text("id").primaryKey(),
  listingKey: text("listing_key").notNull(),
  plan: text("plan").notNull(),                 // 'free' | 'pro' | 'ultra'
  seats: integer("seats").notNull(),            // ≥ 0
  updatedAt: timestamp("updated_at", { withTimezone: true }).notNull(),
}, t => [uniqueIndex("app_seat_quotas_uniq").on(t.listingKey, t.plan)]);

// ② 租户加购(付费占位,本期 console 手工;free 拒绝)
export const tenantSeatAddons = pgTable("tenant_seat_addons", {
  id: text("id").primaryKey(),
  tenantId: text("tenant_id").notNull(),
  listingKey: text("listing_key").notNull(),
  extraSeats: integer("extra_seats").notNull(), // ≥ 0
  note: text("note"),                           // 备注(将来放订单号)
  updatedAt: timestamp("updated_at", { withTimezone: true }).notNull(),
}, t => [uniqueIndex("tenant_seat_addons_uniq").on(t.tenantId, t.listingKey)]);

// ③ 席位分配 —— 维度用 listingKey,与配额/加购/指标同一稳定键
//    (§0.2 决策 2:seatBased 仅 backed listing,安装 appKey === listingKey 恒成立,
//     收口处 apps.app_key 可直接与 listing_key 对接,无漂移)
export const appSeatAssignments = pgTable("app_seat_assignments", {
  id: text("id").primaryKey(),
  tenantId: text("tenant_id").notNull(),
  listingKey: text("listing_key").notNull(),
  userId: text("user_id").notNull(),
  assignedBy: text("assigned_by").notNull(),
  createdAt: timestamp("created_at", { withTimezone: true }).notNull(),
}, t => [
  uniqueIndex("app_seat_assignments_uniq").on(t.tenantId, t.listingKey, t.userId),
  index("app_seat_assignments_user_idx").on(t.tenantId, t.userId),  // 可见性过滤/成员失效钩子用
]);
```

- **可用席位解析**(`modules/seats/service.ts` 单一实现,读侧全部走它):
  `available(tenant, listingKey) = plan === 'unlimited' ? Infinity : (quota(listingKey, plan) ?? DEFAULT_PLAN_SEATS[plan]) + (addon ?? 0)`
- 卸载 App 保留分配行(重装即恢复);席位收口只对 `status='active'` 生效,卸载/停用的 App 本来就进不去。不挂 apps.id 外键——分配关系跨卸载/硬删/重装存续,靠 listingKey 稳定对接。

### 2.3 用量指标目录增量(`packages/shared/src/usage.ts`)

| key | kind | unit | dims | 说明 |
|-----|------|------|------|------|
| `portal.app-seats.available` | gauge | count | app(=listingKey) | 可用席位(配额+加购);unlimited 不上报(§0.3g) |
| `portal.app-seats.allocated` | gauge | count | app | 已分配席位;装了但没分配要显式报 0 |

- dims 白名单新增 `app` 键,取值 = listingKey,基数 = 席位制 App 数(个位数),安全。
- `portal.seats.total`(租户成员数)语义不变,继续并存——它是「人头水位」,席位指标是「开通水位」。

---

## 3. 套餐规范化(M1)

### 3.1 写入口收紧(所有入口,不止 console)

- **校验收在 service 层**:`modules/platform/service.ts` 的 `createTenant()` / `updateTenant()` 校验 `PLANS` 集合(`.trim() || "standard"` 兜底改为 `"free"`,非法值抛 `AppError`)。这一层是唯一实现,天然覆盖**两个写入口**:
  - `POST/PATCH /api/console/tenants*`(`modules/platform/console.ts`,平台会话);
  - `POST/PATCH /api/platform/tenants*`(`modules/platform/index.ts`,`x-platform-key` 管理入口)——漏掉它自由文本就会从旁路回流。
- 两入口的 body schema 同步收紧为枚举(Elysia `t.Union` literal)。注意:**框架层 schema 校验失败走 HTTP 400 `VALIDATION_FAILED`**(全局 onError 现状),项目约定允许「参数不正确」用 4xx——接受 400,不强行包装成 200 业务错误。
- console 前端(`Console.tsx` 创建 Modal、`TenantDetail.tsx` 编辑):自由文本 Field → Select 下拉(四档,label 用 `PLAN_LABELS` 三语),创建默认 `free`。
- **既有 verify/种子里的旧套餐值一并清扫**(如 verify 脚本建临时租户时写的「标准版」等):plan 收紧后这些脚本会 400,属本期回归面,迁移同批修改。

### 3.2 存量数据迁移(手写 SQL)

| 旧值 | 新值 | 依据 |
|------|------|------|
| `标准版` / `standard` | `free` | 最低档对齐 |
| `专业版` / `pro` | `pro` | 同名 |
| `旗舰版` / `enterprise` | `ultra` | 最高付费档对齐 |
| `平台`(平台租户) | `unlimited` | 平台租户不参与计量/限流语义,给最高档 |
| 其他任意文本 | `free` | 未知一律最低档,并在迁移里 `RAISE NOTICE` 行数(不静默) |

- 同一迁移里改列默认值 `standard` → `free`。
- `seed.ts` 演示租户改用新值:平台 → `unlimited`、东田教育 → `ultra`、星网在线学校 → `pro`、青舟培训中心 → `free`。
- 审计日志文案(`新建租户「x」(套餐=…)`)、`MeDTO.tenant.plan`、`/api/admin/settings` 返回值自动跟随,前端展示点统一换 label 渲染。

### 3.3 限流重定键

- `PLAN_RATE_LIMITS` 换新枚举键(§2.1),删除全部旧键;`planRateLimit()` 消费方不变。迁移完成后库里不存在旧值,`DEFAULT_RATE_LIMIT` 只兜未来脏数据的底。

---

## 4. 席位制(M2):声明 → 配额 → 分配 → 收口

### 4.1 声明与快照(整条传播链,一处不落)

`seatBased` 要从声明走到收口,途经的每一环都要加字段,漏任何一环都会「声明了但进不了 DB / 前端看不见」:

1. **manifest / listing 常量**:spms 的 listing 定义(seed / `provision:global`)+= `seatBased: true`;`modules/market/manifest.ts` 的 AppManifest 解析/映射 += 字段(config 驱动接入的 App 也能声明)。
2. **共享 DTO**:`packages/shared/src/dto.ts` 的 `MarketplaceListingDTO` 与 `ListingInput` += `seatBased`。
3. **market service**:`listingDTO()`(读)、`developerFields()`(写字段白名单)、listing create/update payload 映射 += 字段;**声明校验:seatBased=true 时必须是 backed listing**(§0.2 决策 2),否则拒绝保存(`VALIDATION_FAILED`)。
4. **console 市场表单**:`MarketListingForm.tsx` += 席位制开关(非 backed listing 灰禁 + 说明)。
5. **安装快照**:installOne 把 listing 的 `seat_based` 写进 `apps.seat_based`(同 aclManifest 快照点);重装/重激活同步。
6. **变更传播**:listing 发布/更新流程发现 `seat_based` 翻转时,批量 UPDATE 已装租户的 `apps` 行(§0.3i)。

### 4.2 席位管理 API(租户管理员,requireAdmin)

挂 `modules/seats`,路由前缀 `/api/admin/apps/:appKey/seats`(动作路由用子路径,不用冒号——项目既有惯例;seatBased App 的 appKey === listingKey,路由参数即分配表的 listingKey):

- `GET  /api/admin/apps/:appKey/seats?page&pageSize&q` — 分配列表(`Page<T>` 契约,join 成员姓名/邮箱/状态)+ 头部汇总 `{ plan, quota, addon, available, allocated, overQuota }`。
- `POST /api/admin/apps/:appKey/seats` — 批量分配 `{ userIds: string[] }`:逐个校验成员 active、未重复;**配额检查整批原子**(allocated + 新增 ≤ available,unlimited 放行),超了整批拒(`SEAT_QUOTA_EXCEEDED`,响应带当前余量)。
- `POST /api/admin/apps/:appKey/seats/remove` — 批量回收 `{ userIds }`:删行 + 逐人 `bumpUserVersion` + 审计。
- 分配/回收均写审计日志(操作人、目标成员、App)。
- 非席位制 App 调这些路由 → `NOT_SEAT_BASED` 业务错误。

### 4.3 平台配置 API(平台管理员,assertPlatformSession)

- `GET  /api/console/seat-plans` — 席位制 App 清单 × 四档套餐的配额矩阵(含代码默认值标注「未配置=默认」)+ 各 App 使用概况。
- `PUT  /api/console/seat-plans` — 批量 upsert `{ items: [{ listingKey, plan, seats }] }`(plan 限 free/pro/ultra;unlimited 拒绝入表)。
- `GET/PUT /api/console/tenants/:id/seat-addons` — 租户加购读写(`{ listingKey, extraSeats, note }`);**目标租户 plan 为 free → `PLAN_NOT_ELIGIBLE` 拒绝**;unlimited 无意义同样拒绝。降低加购导致 已分配 > 可用 时允许保存但响应/界面警示(进 §0.3d 超配额状态)。
- 配额/加购变更写平台审计日志。

### 4.4 访问收口(平台侧全部改动点)

| 收口点 | 位置 | 行为 |
|--------|------|------|
| 应用中心 / App 导航 / 深链 | `modules/apps/service.ts` `visibleAppRows()` | `apps.seat_based = true` 时叠加 `EXISTS(app_seat_assignments …)` 过滤(`apps.app_key = listing_key` 对接,决策 2 保证不漂移);与 appVisibility AND(§0.3c)。一次 join,列表接口无 N+1 |
| 用户态 TDT 签发(直用三路径) | `modules/token/service.ts` **`mintForApp()`** | host mint(`POST /api/tokens/mint`)、`authorization_code`、`refresh_token` 全部汇于此,一处加席位检查,无席位 → `SEAT_REQUIRED`(200 业务错误)。顺序:安装/状态之后、consent/scope 收窄之前 |
| token_exchange | `modules/token/service.ts` `exchange()` | **显式不加席位门**(§0.3b):App 后端持密发起的跨应用数据消费,受 exchange grant + consent 共授治理;verify 以「exchange 仍通」作对照断言 |
| 管理面直签令牌 | `modules/apps/index.ts` `files-token` / `llm-gateway-token` | **显式豁免**(§0.3j):管理员配置 App 后端不占席位;保持 assertAdmin + 600s 短时;豁免点白名单化,verify 断言无席位管理员仍可取 |
| 已发令牌 / 自省缓存 | 席位回收、成员失效路径 | `bumpUserVersion(userId)` → `isTdtRevoked` 比对 uv 失效令牌、aclStamp 变化失效下游自省缓存(既有机制,注意首次 bump gotcha,§0.3b) |
| 成员停用/移除 | `modules/admin/users.ts` 两个状态翻转处 → `releaseSeatsForMember()` | 删该用户本租户全部席位行 + uv bump + 审计(§0.3f);实现期审计目录 API/全局禁用等其他成员终结路径,有则一并接入 |
| Dashboard 应用 widget | widget catalog 构建处 | 核对:席位制 App 的 manifest widgets 是否已随 `visibleAppRows` 口径过滤;若有独立数据源,同样叠加席位过滤(实现时确认,不放过旁路) |
| 深链兜底 | `MicroAppHost` | 正常情况下无席位者根本看不到入口;直接敲 URL 时 mint 返回 `SEAT_REQUIRED`,host 展示友好门(「未获得席位,请联系管理员」三语),不是白屏 |

### 4.5 存量回填迁移(与席位表同一迁移批次)

- 对每个 `seat_based=true` 且 `status='active'` 的已装 App:为该租户全部 active 成员 INSERT 分配行(`assignedBy` 记系统标识)——§0.3e 的 grandfather。
- 回填行数 `RAISE NOTICE`;超配额租户落入超配额状态,不做处理(§0.3d)。
- `bootstrap:prod` / `provision:global` 路径:listing 常量的 `seatBased` 与配额默认随发布生效;迁移脚本负责存量租户,两条路径都要过一遍演练。

---

## 5. 席位进入用量统计(M3)

- 扩展 `modules/usage/worker.ts` 的 `runSeatsSnapshot()`(同一 6h 周期、同一进程内直写通道):
  1. 现有 `portal.seats.total` 不动。
  2. 新增:遍历「已装席位制 App × 非平台租户」,每组产出:
     - `portal.app-seats.allocated` dims `{app: listingKey}` = 分配行数(join active membership,幽灵不计;无分配显式报 0);
     - `portal.app-seats.available` dims `{app}` = 配额 + 加购(**unlimited 租户跳过此指标**,§0.3g)。
  3. 租户集合 = 「装过席位制 App 的租户」全集(含刚卸载当天,报 0 收尾)——沿用 files 快照「清空也要报零」纪律。
- 配额/分配变更即时性:席位数字是日桶 gauge,6h 快照足够;管理页面上的实时余量走 §4.2 的 seats API,不依赖 usage。
- 展示(读侧零后端改动,自动进「全部指标」/CSV/console 排行与下钻;专属展示见 §6.3)。

---

## 6. 前端(强制两步法:`impeccable` 设计 + Chrome 真浏览器验证)

### 6.1 console(平台管理员)

- **`/console/plans` 套餐管理页(新)**:上半区四档套餐总览卡(label、限流档、席位默认值);下半区席位配额矩阵——行=席位制 App,列=free/pro/ultra(可编辑,空=代码默认,占位灰显示默认值)+ unlimited 列固定「∞」。SideNav console 区加「套餐管理」。
- **`/console/tenants` 列表 + `/console/tenants/:id` 详情**:plan 改下拉 + label 展示;详情页新增「席位加购」区(席位制 App 逐行:配额 / 加购 ± / 已分配 / 超配额红标;free 租户该区禁用并说明)。
- **`/console/usage`**:租户下钻自动含席位两指标;排行指标切换里可选席位指标(读侧自动)。

### 6.2 admin(租户管理员)

- **`/admin/apps/:appId` 详情新「席位」Tab**(仅 `seatBased` App 显示):顶部汇总条(可用 n / 已分配 m / 剩余,unlimited 显示 ∞;超配额红色警示条 + 文案「新分配已暂停,请回收席位」);成员分配表(`Page<T>` 分页 + 搜索,含未分配成员的添加入口,批量勾选分配/回收);配额用满时分配按钮禁用 + 引导文案(free:「免费版席位已满」;pro/ultra:「请联系平台方增加席位」)。
- **`/admin/apps` 列表**:席位制 App 行加「席位 m/n」徽标,点击直达 Tab。
- 应用中心成员视角:无席位的席位制 App 不出现(收口在后端,前端无感);深链兜底门见 §4.4。

### 6.3 用量页

- **`/admin/usage`**:概览卡加一枚「席位」(席位制 App 的 allocated 合计/available 合计);新增「席位」Tab——按 App 分组的 available vs allocated 对比条 + 按日趋势(复用 `pages/usage/shared.tsx` 组件,图表容器定高的既有 gotcha 照旧)。**unlimited 的渲染要点(§0.3g)**:summary 对目录内指标总会返回一项,`available` 无数据长成 `last:null`——前端结合租户 plan(MeDTO 已带)把 null 渲染为「∞ / 无上限」,严禁显示成 0 或空态;capped 套餐 null 才是真「暂无数据」。
- **`/console/usage`**:下钻页同款席位区块。

### 6.4 i18n 与导航

- 三语新增:`plan.*`(四档 label,console/admin/usage 共用一份)、`seats.*`(Tab/汇总/错误码文案 `SEAT_REQUIRED`/`SEAT_QUOTA_EXCEEDED`/`PLAN_NOT_ELIGIBLE`/`NOT_SEAT_BASED`)、`console.plans.*`。
- 新 console 页面按既有 console 导航惯例注册;admin 侧无新页面(Tab 挂在既有 `/admin/apps` 详情,无需新 PID;若实现中发现详情页有独立 PID 粒度,按 portal ACL manifest 惯例补注册)。

---

## 7. Seed / 迁移 / verify / 里程碑

### 7.1 Seed 增量(destructive 主种子)

- 演示租户 plan 用新枚举(§3.2);spms listing `seatBased: true`;
- 演示席位分配:**给 verify/演示流程会用到的成员种上 spms 席位**(两演示租户的管理员 + 常用演示成员)——否则 verify:all 里既有 spms 流程(铸币/自省/页面走查)会被席位门挡住而全面变红;这是本期最大的回归破坏面,seed 先行。
- 配额矩阵不强制 seed(代码默认兜底即可用);可选 seed 一行示例配置便于演示 console 编辑态。

### 7.2 验证(每步可独立回归)

1. **M1 套餐** → 扩展现有 console/平台 verify 或并入 verify-seats:非法 plan 在 **console 与 `x-platform-key` 两个入口**都被拒(400 `VALIDATION_FAILED`);四档创建/编辑往返;迁移映射抽查(标准版→free、旗舰版→ultra、未知→free);`planRateLimit` 新键生效;全仓 grep 旧套餐字面量(标准版/专业版/旗舰版/standard/enterprise)确认 verify/seed 无残留。
2. **M2 席位** → 新 `apps/api/scripts/verify-seats.ts`(纳入 `verify:all`),沿用 usage-verify 的**一次性专用租户夹具**(建租户→装 spms→测→force 删),避免与真实 worker/演示数据竞态:
   - 分配/回收往返;重复分配幂等拒;非成员/停用成员分配拒;
   - free 配额用满 → 整批 `SEAT_QUOTA_EXCEEDED`;pro + 加购后余量增加;unlimited 大批量分配放行;free 加购 → `PLAN_NOT_ELIGIBLE`;
   - 无席位成员:`GET /api/apps` 不含该 App;**四条签发路径逐条断言**——host mint / authorization_code / refresh_token 均 `SEAT_REQUIRED`,token_exchange 仍通(§0.3b 对照);分配后立即可铸;
   - **回收后旧令牌失效**(uv bump 生效,自省拒绝)——§0.3b 的 gotcha 断言;
   - 管理面直签豁免:无席位的租户管理员仍能取 `files-token` / `llm-gateway-token`(且端点保持 assertAdmin);
   - 成员停用与成员移除**两条路径**都触发 `releaseSeatsForMember`(席位行清空 + 旧令牌失效);套餐降级 → 超配额:存量令牌仍可用、新分配被拒;
   - 非席位 App / 自建 App 完全不受影响(对照组);listing 声明校验:非 backed listing 置 seatBased → 拒绝保存。
3. **M3 指标** → 扩展 `verify-usage.ts`:构造分配 → 触发 seats worker → `portal.app-seats.*` 数值与分配行数/配额一致;unlimited 租户无 available 行;回收到 0 后快照报 0;重跑幂等。
4. **M4 前端** → Chrome extension 真浏览器走查:console 套餐下拉与配额矩阵编辑、租户加购;admin 席位 Tab 分配/回收/满额禁用/超配额警示;成员视角 App 消失与深链兜底门;`/admin/usage` 席位卡与 Tab;三语 + 暗色。环境起不来则显式说明未浏览器验证。
5. 全量 `bun run verify:all` + `db:reseed` 后复跑(reseed 链路会重放 spms bootstrap,确认席位 seed 与各 bootstrap 顺序无冲突)。

### 7.3 里程碑

```
M1 套餐规范化(枚举 + console 下拉 + 迁移 + 限流重定键)                → 套餐断言全绿
M2 席位底座(声明/快照 + 三表 + admin/console API + 收口 + 回填迁移)   → verify-seats 全绿
M3 席位指标(目录 + worker 扩展)                                      → verify-usage 扩展断言全绿
M4 前端(console 套餐管理/加购 + admin 席位 Tab + 用量展示 + 兜底门)   → 浏览器走查通过
M5 文档(用量统计服务接入指引:席位指标与目录增量;App 接入文档:seatBased 声明) → 文档与实现一致
```

M1 是依赖根(配额按 plan 解析);M2 依赖 M1;M3 依赖 M2;M4 依赖 M2(M3 完成前席位 Tab 可先行,用量展示随 M3);M5 收尾。

---

## 8. 通往席位计费的路径(下一期蓝图,本期不实现)

- **定价**:`plan_prices` 增加席位单价维度(listingKey × plan × 每席位月价);账单 = `portal.app-seats.available`(购买口径)或 `allocated`(使用口径)的月度 gauge 聚合 × 单价——两口径 usage 查询层已同时提供(期末 + 峰值),计费期按商务规则选用。
- **自助加购**:租户管理员发起加购 → 支付 → 回调写 `tenant_seat_addons`(本期 console 手工写的同一张表);free 不可加购的规则在 API 层已立好,支付层不必重复。
- **配额硬闸联动**:席位天然是硬闸(分配即拦),与 USAGE 一期「不做用量硬限制」不冲突——席位限的是「人数」而非「用量」,用量类硬闸(token 限额等)仍留给计费期。
- **对账**:`app_seat_assignments` 行数 vs `usage_daily` 席位桶,即 verify 断言的生产常态化。
