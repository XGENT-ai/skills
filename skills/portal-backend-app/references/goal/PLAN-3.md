# PLAN-3 · XGENT.ai Portal 三期：应用市场 + 内容管理公共服务 + 待办 App

> **本文为「按实现对齐」版**：记录的是最终落地、且经脚本 + 真浏览器验证的方案（含实现期的两处修正：① 内容 schema 改为架构层 code 注册表；② 待办 App 改 React + shadcn + 三语 i18n）。原始提案与本版的差异在 §9「与原始提案的差异」列出。
> 站在一期底座（`goal/PLAN-1.md`）+ 二期平台化层（`goal/PLAN-2.md`）之上。迁移 `0006_curved_exiles.sql` + `0007_content_registry.sql`。

---

## 0. 本期范围与决策（最终落地）

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | 待办 App 持久化 | **门户「内容管理」公共服务（headless-CMS 式）** | App 经 Open API + SDK 对内容条目做 CRUD；待办 App 纯前端，数据存门户。**内容 schema 是架构层 code 定义**（见 #1b），不是市场可编辑配置。 |
| 1b | 内容 schema 来源 | **架构层 code 注册表（owner 标记 + 表名命名空间）** | schema 固定、写死在 `apps/api/src/modules/content/schemas.ts`；每个内容类型标记 `owner` 应用 + 全局唯一 `table = owner__key`。**不在市场 UI 编辑、不随安装快照、租户/管理员不手改**。 |
| 2 | 市场安装模型 | **快照复制（开发者字段）+ 来源指针** | 安装把 listing 的开发者字段复制成租户自己的 `apps` 行 + 记 `marketplaceListingId`/版本号；内容 schema **不复制**，改为 `apps.contentOwner` 指针（= listingKey）指向 code 注册表。 |
| 3 | 应用市场管理深度 | **精简上架** | 清单 CRUD + 状态（草稿/已上架/已下架）+ 精选 + 版本号 + 分类。内容类型在表单里**只读展示**，不可编辑。 |
| 4 | 存量 App 处理 | **存量保持租户私有，市场只种待办** | 一期 8 个演示 App + 二期 sample-app 不动；市场首发只上架「待办」。 |
| 5 | 待办 App 技术栈 | **React + Tailwind + shadcn/ui** | 待办 App 用 React + shadcn（含 shadcn 日期选择器：Popover + react-day-picker），并做**三语 i18n 跟随宿主语言**。 |

> 命名：需求写「代办任务」，正确中文是**待办**（to-do）；统一用「待办」。

### 0.1 关键取舍（headless CMS 调研后）

- **schema-as-code（Sanity 式）**：内容类型随应用代码走、PR 评审，而非在 UI 里建模。正好契合本期：schema 固定在 code 注册表，应用是其 owner，安装只是把应用标记成该 owner。
- **字段 schema 结构对齐 Strapi `attributes`**：`FieldDef = { key, label, type, required?, unique?, default?, options?, min?, max?, maxLength? }`。字段类型本期：`text | textarea | number | boolean | date | datetime | enum | json`。
- **查询 API 对齐 Directus `/items`**：`GET/POST/GET:id/PATCH:id/DELETE:id`；列表 `filter[field]=value` + `q`（文本字段 ILIKE）+ `sort`（CSV，`-` 降序）+ `limit`/`offset`（封顶 `CONTENT_LIST_MAX_LIMIT`）。
- **表名即冲突边界**：每个内容类型有全局唯一 `table = ${owner}__${key}`（如 `todo__task`、`sample__note`）。两个应用都声明 `task` 也不撞；**应用只能解析/读写自己 owner 声明的表** = 隔离边界。
- **鉴权沿用二期**：内容写入走用户态 TDT（self-scoped），复用 Open API 网关 `gate()`（TDT + 限流 + 审计）。不引入应用服务态令牌。

### 0.2 明确不在本期

- **租户/管理员在 UI 里编辑内容 schema**（schema 是 code 层，永不手改）。
- 内容服务：GraphQL/GROQ、跨类型关系、组件/动态区、媒体库、richtext、内容草稿/发布/版本/多语言；富算子查询（本期仅相等 + `q` + sort/limit）。
- 市场：版本化发布记录/回滚、截图详情页、评分/安装量、开发者自助提交 + 审核流。
- semver 差异对比（本期版本号字符串不等即「有更新」）。
- client-credentials / 应用服务态令牌。

---

## 1. 数据模型（Drizzle · 迁移 `0006` + `0007`）

| 表 / 模块 | 形态（最终） | 说明 |
| --- | --- | --- |
| `marketplace_listings`（新，全局目录，平台拥有） | `id, listingKey unique, name, tagline, desc, icon, color, logoUrl, type(AppType), cat, embedUrl, allowedOrigins[], landingUrl, nativeRoute, scopes[], extPoints jsonb, navItems jsonb, webhookEvents[], tdtTtl, allowRefresh, refreshTtl, allowedGrants[], publisher, version, status(draft｜published｜delisted), featured, sort, createdAt, updatedAt` | **无 `contentTypes` 列**——schema 在 code 注册表。 |
| `apps` | + `marketplaceListingId fk→marketplace_listings null`（来源指针）<br>+ `marketplaceVersion text null`（判断「有更新」）<br>+ `contentOwner text null`（**code 注册表的 owner key**；安装时 = listingKey；null = 无内容 schema） | 取代原 `contentTypes(jsonb)`。 |
| `content_entries`（新，内容条目） | `id, tenantId fk, tableName text, ownerUserId fk→users null, data jsonb, createdAt, updatedAt`<br>索引 `(tenantId, tableName, ownerUserId)`、`(tenantId, tableName)` | **按 `tableName` 命名空间存储**（不再有 appId/typeKey）；user 域记 ownerUserId、tenant 域为 null；卸载重装数据仍在。 |
| `modules/content/schemas.ts`（**code，非 DB**） | `CONTENT_SCHEMAS: SchemaDef[]`，`SchemaDef extends ContentType { owner; table }`；`resolveSchema(owner,key)` / `schemasForOwner(owner)` / `toContentType()` | **内容 schema 的唯一真源**，架构层定义。当前含 `todo→task`、`sample→note`。 |

**shared 增量（`packages/shared`）**
- `scopes.ts`：`content.read` / `content.write`（+ `labels` 命名空间三语标签）。
- `constants.ts`：`FIELD_TYPES` + `FIELD_TYPE_LABELS`；`CONTENT_RECORD_SCOPES`(user/tenant)；`LISTING_STATUS`(draft/published/delisted) + 标签；`CONTENT_LIST_MAX_LIMIT`。
- `dto.ts`：`FieldDef`、`ContentType`（`{key,name,recordScope,fields}`）、`ContentEntryDTO`、`MarketplaceListingDTO`（含 embedUrl/allowedOrigins/landingUrl/nativeRoute；`contentTypes` 字段为**注册表派生的只读投影**）、`InstalledListingDTO`（+ installed/installedVersion/updateAvailable）；`AppDTO.marketplaceListingId`。
- `errors.ts`：`LISTING_NOT_FOUND`、`LISTING_NOT_PUBLISHED`、`ALREADY_INSTALLED`、`CONTENT_TYPE_NOT_FOUND`、`CONTENT_ENTRY_NOT_FOUND`。

> `0007` 为**手写迁移**（+ journal 条目）：删 `apps.contentTypes`/`marketplace_listings.contentTypes`，加 `apps.contentOwner`，`content_entries` 重构为 `tableName` 键。原因见 §8 坑位。

---

## 2. 后端机制

### 2.1 内容管理公共服务（需求 1）
- 入口复用 Open API 网关 `gate(headers, scope)`（`modules/openapi/index.ts`）。
- 端点：`GET /api/v1/content-types`（content.read）+ `GET/POST /api/v1/content/:type`、`GET/PATCH/DELETE /api/v1/content/:type/:id`（read/write）。
- **解析链**：TDT → app → `app.contentOwner` → `resolveSchema(contentOwner, :type)` → `SchemaDef`（含 `table`）。解析不到 → `CONTENT_TYPE_NOT_FOUND`。这把「应用只能访问自己 owner 的表」作为隔离边界。
- 业务在 `modules/content/service.ts`：`validateEntry`（按 FieldDef 校验+强转：required/类型/enum/min·max/maxLength）；CRUD 按 `(tenantId, schema.table, ownerUserId|null)` 隔离；列表 filter/q/sort/limit。
- `:type` 是逻辑 key（如 `task`），对外稳定；内部映射到 `schema.table`（如 `todo__task`）。
- SDK `sdk.content.{types,list,get,create,update,remove}`（按逻辑 key 调用，无感知表名）。

### 2.2 应用市场后端（需求 2/3）
- 业务集中 `modules/market/service.ts`；两通道复用、皆 fail-closed：
  - **平台上架** `modules/market/console.ts`，`/api/console/market/*`，`assertPlatformSession`：listing CRUD + `/status`（草稿↔上架↔下架）。审计 `actorType:"platform"`。
  - **租户安装** `modules/market/tenant.ts`，`/api/admin/market/*`，`assertTenant+assertAdmin`：列已上架（+本租户安装态）/详情/install/update。
- `listingDTO.contentTypes` = `schemasForOwner(listingKey).map(toContentType)`（**只读派生**，供安装评审 + console 只读展示）。

### 2.3 安装 = 快照 + 来源指针 + owner 指针（需求 2）
`installListing(tenantId, listingKey, actor)`：
1. listing 必须 `published`，否则 `LISTING_NOT_PUBLISHED`/`LISTING_NOT_FOUND`。
2. 本租户已有该 listing 的 App：active → `ALREADY_INSTALLED`；uninstalled → 复装（重置 active + 重刷开发者字段）；无 → 新建。
3. **开发者字段（从 listing 复制）**：name/desc/icon/color/logoUrl/type/cat/embedUrl/allowedOrigins/landingUrl/nativeRoute/scopes/extPoints/navItems/webhookEvents/token 策略；**`contentOwner = listingKey`**（不复制 schema）；`marketplaceListingId` + `marketplaceVersion`。appKey 默认 = listingKey（`freeAppKey` 冲突加后缀）。
4. **运营字段（安装默认值，租户可改）**：visibility=all、showInCenter=true、status=active、enabledNavItemIds=声明项、sort 末位。
5. `type==="micro"` → `createSecret` 自动签发一把 App Secret。
6. 写审计「安装应用」。
- update：用 listing 当前开发者字段重刷已装 App（运营字段不动），版本号跟进。

---

## 3. 前端（强制两步法：`impeccable` 设计 + Chrome 真浏览器验证）

- **租户「从市场添加」**：`pages/admin/Marketplace.tsx`（已上架卡片网格 + 分类/搜索 + 安装/已安装/有更新 + **安装前权限评审弹窗**，弹窗内含只读内容类型披露）；入口在 `AdminApps` 顶部「应用市场」。`lib/market-api.ts`。
- **平台上架管理**：`pages/console/Market.tsx`（清单列表 + 草稿/上架/下架/精选/删除）+ `pages/console/MarketListingForm.tsx`（基本信息/技术配置/权限范围 + **内容类型只读展示**：「内容结构由应用在架构层定义，不在此处编辑」+ 每个类型显示 名称/key/归属/字段数）。SideNav 平台组「应用市场」。
- 复用 `components/ui` 原语；业务态 200+envelope，按 `error.code` 分支。

---

## 4. 待办 App（`apps/todo-app`，市场首发，需求 5）

React + Tailwind + **shadcn/ui** 的微应用（Vite，端口 5175，env `TODO_APP_URL`，`@xgent/portal-sdk`）。

- **内容类型** `task`（owner `todo`，recordScope user）定义在 code 注册表；待办 App 按逻辑 key `task` 经 `sdk.content.*` 读写（解析到 `todo__task` 表）。
- **shadcn 日期选择器**：`components/ui/{button,popover,calendar}.tsx` + `components/DatePicker.tsx`（Popover + react-day-picker v8 + date-fns）。
- **三语 i18n**：`src/lib/i18n.ts` 自带 zh-CN/en/zh-TW 词典 + date-fns locale 映射；跟随宿主 `init.locale` + `onLocale` 实时切换——文案、相对到期、**日历本身**（月份/星期/周首日/触发日期格式）全部本地化。
- **能用好的体验**（`impeccable`）：快速添加；今日/即将/逾期/已完成 分组；勾选完成；行内编辑/删除；优先级 + 到期；空/错/载入态；跟随宿主主题（`.dark`）；reduced-motion。
- **到期提醒**：`sdk.scheduler.create`（每日提醒 → 触发回收件箱通知）。**工作台小组件**：`sdk.contributeWidget("dashboard.widget", …)`（待办/今日/已完成 计数）。

---

## 5. Seed / 运行编排

- `db/seed.ts`：上架「待办」listing（`listingKey "todo"`，type micro，embedUrl=`TODO_APP_URL`，scopes `[userinfo.read, content.read, content.write, notification.send, scheduler.read, scheduler.write, widget.write]`，extPoints `[dashboard.widget]`，version `1.0.0`，featured；**无 contentTypes**）。sample-todo 设 `contentOwner: "sample"` + content scopes。内容 schema 全在 code 注册表。存量 App 不动。
- 根 `package.json`：`dev:all` 含 `@xgent/todo-app`，加 `dev:todo`。
- `lib/env.ts` `todoAppUrl`；API CORS `origin` 加 `todoAppUrl`；**`apps/web/index.html` CSP `frame-src` 加 `http://localhost:5175`**（否则 iframe 被拦）。

---

## 6. 验证

- **后端脚本**：`verify-content.ts`（20：类型自省 / CRUD / required+enum+数字校验 / filter+q+sort / 未知类型 / scope 403 / 跨用户隔离）、`verify-market.ts`（22：listing CRUD / 上架门槛 / 安装含 contentOwner / ALREADY_INSTALLED / 更新 / 卸载复装 / 非平台-admin 与非租户-admin 拦截；幂等可重跑）。
- **Chrome 真浏览器**：平台建并上架 listing → 租户从市场安装（权限评审 + 只读内容类型披露）→ 打开待办 App：增/改/勾完成/删（刷新持久，经 `todo__task`）、shadcn 日历选到期、切换门户语言为 English 待办 App 实时英文、工作台小组件计数、计划任务页见「来自 待办」只读提醒；console 上架表单内容类型分区**只读**。
- `bun run typecheck` 全 6 包绿；门户 i18n 三语 parity（668 键 × 3，新增 `market` 命名空间）；待办 App 自带三语词典。

---

## 7. 分期（实际落地顺序）

1. **Phase 0 · 契约与数据模型**：shared + schema + 迁移 `0006` + seed 脚手架。
2. **Phase 1 · 内容服务 + SDK**：Open API 内容路由 + 校验 + 隔离 + `sdk.content.*`。✅ `verify-content`。
3. **Phase 2 · 市场后端**：`market/{service,console,tenant}` + 安装/更新。✅ `verify-market`。
4. **Phase 3 · 市场前端**：console 上架管理 & 租户市场浏览/安装。✅ impeccable + Chrome。
5. **Phase 4 · 待办 App**：`apps/todo-app` 全功能 + seed 首发。✅ Chrome 端到端。
6. **Phase 5 · 收尾**：i18n parity + 验证脚本回归 + README/CHECKLIST。
7. **增量 A · 待办 App → React + shadcn + i18n**：见 §4。
8. **增量 B（架构修正）· 内容 schema → code 注册表（owner + 表名）**：迁移 `0007`；见 §0/§1/§2.1。

---

## 8. 关键复用点 & 实现期坑位

**复用**：`gate()`/`requireTdt`/`enforceRateLimit`；`assertPlatformSession`/`assertTenant`/`assertAdmin`；`createSecret`/`bumpAppVersion`/`getAppByKey`；scheduler/notify/widgets Open API + SDK；前端 `components/ui`/`AppForm`/`pages/console/*`/i18n `lib/locales`。

**坑位（务必记住）**：
- **CSP `frame-src` 是白名单**（`apps/web/index.html`）：新增微应用源必须加入，否则 iframe 被静默拦截（宿主一直「握手超时」、iframe 显示破图）。已加 `:5175`。同步：API CORS `origin` + listing `allowedOrigins`。typecheck/脚本查不出，只有真浏览器能发现。
- **迁移 `0007` 手写**：drizzle-kit generate 对「列删除/重命名」会弹交互式确认、无法无人值守跑。`0007` 的 snapshot 未更新——下次 `db:generate` 需在交互终端跑一次重新对齐。`db:migrate`/`db:seed` 不受影响。
- **date-fns zhCN「PP」是数字式 `yyyy-MM-dd`**（日历本身仍中文）：若要「年月日」用「PPP」。
- API `start` 无 `--watch`，改后端后需重启；`db:seed` 重生成 UUID → 需重登录。

---

## 9. 与原始提案的差异（为什么变）

| 原始提案 | 最终实现 | 原因 |
| --- | --- | --- |
| 内容 schema 存在 `marketplace_listings.contentTypes`/`apps.contentTypes`，平台管理员在上架表单里**编辑** FieldDef[]，安装时快照复制 | schema 是**架构层 code 注册表**（owner 标记 + 表名命名空间）；表单只读展示；`apps.contentOwner` 指针取代快照 | 每个应用需要什么 schema 是**确定的**，属架构层、不该当作市场可编辑配置；owner 标记 + 表名解决应用间表名冲突 |
| `content_entries` 按 `(appId, typeKey)` 隔离 | 按 `tableName`（owner 命名空间）隔离 | 表名即冲突边界；卸载重装数据仍在 |
| 待办 App 纯 TS（同 sample-app），原生 `<input type=date>` | React + Tailwind + **shadcn**，shadcn 日期选择器 + 三语 i18n | 用户指定 shadcn 组件 + i18n |
