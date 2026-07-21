# PLAN-2 · XGENT.ai Portal 二期开发计划

> 在一期（`goal/PLAN-1.md` 全 8 Phase + 大量二期增量，见 `goal/CHECKLIST.md`）已完成并浏览器/脚本验证的底座之上，做**平台化**：平台租户 + 租户管理 App、App 贡献顶级导航、自定义域名 + 租户品牌、消息/计划任务对下游 App 开放为公共服务。
> 依据：本期五项需求 + `PRODUCT.md`（底座定位：通用 SaaS 微应用平台）+ `DESIGN.md` + `CLAUDE.md`（API 约定 / 前端两步法 / 简洁优先 / 外科手术式改动）。

---

## 0. 本期范围与决策（已拍板）

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | 租户管理 App 形态 | **门户内一等公民原生页面** | 在 `apps/web` 做原生路由（`/console/*`），并在应用中心/启动器注册成一张 App 卡片；复用现有 `/api/platform/*` 业务逻辑，鉴权从 `x-platform-key` 扩展为「**会话 + 平台租户管理员**」。不另起独立微应用。 |
| 2 | App 顶级导航粒度 | **App 可声明多条菜单项（深链）** | App 声明 `navItems[]`（`{id,label,icon,path}`，path 为应用内深链）；租户管理员**按条启用**；启用项在左侧顶级导航对全体成员可见。 |
| 3 | 自定义域名归属校验 | **平台后台手动标记 + 本地 Host 模拟** | 新表 `tenant_domains`，平台管理员在租户管理 App 里手动置 `verified`；本地用 `Host` 头/`?__host=` 查询参数模拟自定义域名访问（类比 dev mock IdP）。不做真实 DNS TXT/CNAME 探测。 |
| 4 | 公共服务鉴权 | **沿用用户态 TDT（self-scoped）** | 不引入 client-credentials/应用服务态令牌。消息发给「当前 TDT 用户自己」；计划任务**归属于 (租户, App, 用户)**，由用户在应用内经 TDT 创建/管理，触发后调用 App Webhook 或给该用户发通知。Cron 执行在服务端，不需要在线 TDT。 |

### 0.1 关键假设（需明示）

1. **平台租户 = 一个被标记的专用租户（已定）**：新增 `tenants.isPlatform`（单例语义）。一期 seed 的三个教育租户均保持「客户租户」；本期 seed **新增一个专用平台租户**——slug `platform`、名「XGENT 平台」、`isPlatform=true`，并让跨租户用户 Rockie 成为其管理员。平台租户排在 seed 首位，落实「初始租户即平台管理租户」。
2. **平台管理双通道，二者皆 fail-closed**：(a) 既有 `x-platform-key` 头（保留为 break-glass / Swagger 调试）；(b) 新增「会话 + 平台租户 admin」通道，给租户管理 App 前端用。两条通道复用同一套 service 函数。
3. **原生 App 不走 iframe**：新增 App 类型 `native` + `nativeRoute` 字段；应用中心/启动器/宿主对 `native` 应用路由到门户内部路由，而非沙箱 iframe。租户管理 App 即 `native` 类型，仅对平台租户管理员可见。
4. **本地可验证自定义域名**：因本地无法真跑 DNS，沿用一期「dev 开关」思路——`resolveHostTenant()` 在 `DEV_MOCK_OAUTH`（或新 `DEV_HOST_OVERRIDE`）开启时接受 `?__host=acme.example.com` 模拟；生产读真实 `Host`/`X-Forwarded-Host`。
5. **延续一期全部约定**：业务状态不走 HTTP code（§ envelope）；前端 UI 强制两步法（`impeccable` 设计 + Chrome extension 真浏览器验证）；i18n 三语全量对齐（zh-CN/en/zh-TW，新增键须三语同步，不得破坏现有 489 键 parity）；外科手术式改动（只动需求相关行）。

### 0.2 明确不在本期（延后到三期+）

- **client-credentials / 应用服务态令牌**（无人值守的 App 后端调公共服务）——本期 self-scoped 已满足需求，留作三期。
- **真实 DNS 校验**（TXT/CNAME 探测、自动签发 TLS 证书 / ACME）——本期手动标记。
- **跨租户的「全局应用」目录**（一份 App 定义分发到多租户）——本期 App 仍租户私有；平台租户的原生 App 是特例（按 `isPlatform` 网关），非通用全局分发机制。
- **App 自助上架 / 开发者门户 / 应用市场**（App 自描述 manifest 拉取、审核流）——`navItems` 本期由管理员在 AppForm 录入，不做远程 manifest。
- **套餐配额对租户数/域名数/任务数的强约束**（一期已有按 plan 的 Open API 限流；本期仅给 App 创建的任务加每用户上限兜底）。
- **平台级计费 / 审计独立视图**（平台操作仍写 `audit_logs`，不做独立平台审计页）。
- 平台租户品牌定制之外的**主题深度定制**（自定义字体/圆角/整套 token 覆盖）——本期品牌仅覆盖主色 + Logo + 名称。

---

## 1. 数据模型增量（Drizzle · 在一期 schema 上追加）

> 全部新列/新表延续一期租户隔离原则；新增迁移 `0005_*`（视拆分可多份）。`packages/shared` 同步枚举/DTO/错误码，保持前后端契约同源。

| 表 | 变更 | 对应需求 |
| --- | --- | --- |
| `tenants` | + `isPlatform boolean default false`（单例：平台租户） | 1 |
| `apps` | + `navItems jsonb default []`（App 声明的菜单：`{id,label,icon,path}[]`）<br>+ `enabledNavItemIds text[] default []`（租户管理员启用的项 id）<br>+ `nativeRoute text`（`native` 类型 App 的门户内路由） | 2, 1 |
| `tenant_domains`（新） | `id, tenantId(fk), domain text unique, verified boolean default false, verifyToken text, createdAt` | 4 |
| `tasks` | + `createdByAppId uuid fk→apps null`（App 经 Open API 创建则非空；租户管理员建则 null）<br>+ `ownerUserId uuid fk→users null`（任务归属用户；App 创建的任务 = 当前 TDT 用户） | 5 |

**shared 增量（`packages/shared`）**
- `constants.ts`：`APP_TYPES` 增 `"native"`；新增导出 `PLATFORM_TENANT_SLUG`（参考）。
- `scopes.ts`：新增 `scheduler.read` / `scheduler.write` 两个 scope（+ `SCOPE_LABELS` 三语标签经 `labels` 命名空间本地化）。消息服务沿用既有 `notification.send`。
- `dto.ts`：`TenantDTO` 增 `isPlatform?`、`logoUrl`/`color` 已在 `brand` 内（前端品牌消费用）；新增 `NavItemDTO`、`TenantDomainDTO`、`ScheduledTaskDTO`（Open API 任务形态，区别于 admin `taskDTO`）、`BrandingDTO`（`{tenantId,name,logoUrl,color}|null`）。
- `errors.ts`：新增 `DOMAIN_TAKEN`、`DOMAIN_NOT_VERIFIED`、`NOT_PLATFORM_TENANT`、`SCHEDULER_LIMIT`（每用户任务上限）、`TASK_NOT_OWNED`。

---

## 2. 后端机制设计

### 2.1 平台管理双通道鉴权（需求 1）
- 把 `modules/platform/index.ts` 的业务逻辑抽到 `modules/platform/service.ts`（纯函数：列表/建/详情/改/停/删租户、授/撤管理员、域名 CRUD）。
- 两个网关，均 fail-closed：
  - `assertPlatformKey(headers)`（现状，保留）——Swagger / 运维。
  - `assertPlatformSession(session)`（新增）——校验会话用户在**平台租户**（`isPlatform=true`）里是 active admin；否则 `NOT_PLATFORM_TENANT`/`FORBIDDEN`。
- 新增一组**会话版**路由（如 `/api/console/tenants…`），复用 `service.ts`，供前端原生页面调用（带 cookie，CORS 已放行 portal 源）。Swagger 版 `/api/platform/*` 不动。
- 平台操作继续写 `audit_logs`（`actorType:"platform"`；删除租户记 `tenantId:null` 以免被级联清除——沿用一期做法）。

### 2.2 租户管理 App = native 应用（需求 1+2 的「App 卡片」）
- 应用类型新增 `native`：catalog 里一张 App 行，`nativeRoute="/console"`，仅在**平台租户 + admin** 下出现在应用中心/启动器/收藏；点击路由到门户内部页面而非 iframe。
- `MicroAppHost` / 应用中心 / `AppLauncher` 在遇到 `type==="native"` 时 `navigate(nativeRoute)`，跳过握手/TDT 注入逻辑（原生页面用会话直接调 `/api/console/*`）。
- seed：平台租户下注入一张 `native` App「租户管理」。

### 2.3 App 贡献顶级导航（需求 2）
- App 在 AppForm 里声明 `navItems[]`（`{id,label,icon,path}`；path 为应用内深链，复用一期深链 `?r=` 机制；`link` 型 App 的 path 可为外链）。
- 租户管理员在 AppForm（或导航管理分区）勾选 `enabledNavItemIds`。
- 新端点 `GET /api/apps/nav`：按活跃租户 + 当前用户**可见的 active App**，汇总其 `enabledNavItemIds` 对应的 `navItems`，返回 `NavItemDTO[]`（携带 `appId` + 深链目标）。可见性复用一期 `app_visibility` 过滤。
- 点击导航项 → `/app/:appId?r=<path>`（`native` App 则 `nativeRoute`）。

### 2.4 自定义域名 + 租户品牌（需求 4）
- `resolveHostTenant(host)`：去端口 → 查 `tenant_domains.verified=true` 命中 → tenantId；dev 下接受 `?__host=` 覆盖。
- 公共品牌端点 `GET /auth/branding`（**免鉴权**，登录前可调）：据 Host 返回 `BrandingDTO`（租户名/Logo/主色）或 `null`（默认 XGENT 品牌）。
- **登录后默认租户切到该域名租户**：把 host→tenant 作为 `tenantHint` 串过登录流程——
  - OAuth：`beginAuthorize` 时把 hint 存入 flow state，callback `establishSession` 读取；
  - 密码/dev 登录：请求即带 Host，直接传入 `establishSession`；
  - `pickActiveTenant(userId, hint?)`：若 hint 租户存在且用户是其 active 成员，则优先选它；否则沿用原逻辑。
- 平台管理员在租户管理 App 里维护域名（增/删/标记 verified），写审计。
- 校验门：登录/品牌只认 `verified` 域名；未验证域名 → 视为默认 XGENT（fail-closed）。

### 2.5 消息 + 计划任务公共服务（需求 5，self-scoped）
- **消息服务**：既有 `POST /api/v1/notifications`（scope `notification.send`，发给 TDT 用户自己）即为公共消息服务——本期补充文档化 + 在 Swagger/README 标注为「平台消息服务 v1」，并复用一期 `createNotification()` 单一收口（落库 + 实时推送 + 邮件按偏好）。不扩展到「发给他人」（那需服务态令牌，已延后）。
- **计划任务公共服务**（新 Open API，scope `scheduler.*`）：
  - `POST /api/v1/scheduler/tasks`（`scheduler.write`）：当前 TDT 用户经 App 创建一个归属 `(tenant, app=aud, user)` 的任务；`createdByAppId`/`ownerUserId` 自动注入；webhook 目标默认该 App 的 `webhookUrl`；校验 cron + 每用户每应用任务上限（`SCHEDULER_LIMIT`，如 ≤10）。
  - `GET /api/v1/scheduler/tasks`（`scheduler.read`）：仅列出本 App 为本用户创建的任务（`createdByAppId=aud AND ownerUserId=user`）。
  - `PATCH`/`DELETE /api/v1/scheduler/tasks/:id`：同上归属校验，越权 → `TASK_NOT_OWNED`。
  - 执行：复用现有 `runTask`/`scheduler.ts`；App/用户态任务触发后调 App Webhook（payload 带 `userId` 上下文）或给 `ownerUserId` 发通知。Cron 在服务端跑，无需在线 TDT。
  - 每次调用写审计（`actorType:"service"`，actorName=appKey），与一期 Open API 一致。
- **租户管理员可见性**：`AdminTasks` 列表纳入 App 创建的任务（带「来自 <App>」标签，对管理员只读，不可在门户编辑——归属 App 管理），保证治理透明。

---

## 3. 前端增量（`apps/web`）

> 所有新 UI 走两步法：先 `impeccable` 定设计（沿用 `DESIGN.md` token / 原子），再 Chrome extension 真浏览器验证。新增文案全部 `t()` 化、三语同步。

- **租户管理 App（`/console/*`，需求 1）**：租户列表（含成员/管理员/应用计数）、新建、详情、编辑（名/套餐/状态/品牌）、停用/删除（守卫）、管理员授予/撤销、**域名管理**（增删 + 标记 verified）。复刻一期 `/admin/*` 治理页的视觉语言。仅平台租户 admin 可达（路由守卫 + SideNav 项）。
- **顶级导航渲染（需求 2）**：`SideNav` 增「应用导航」区——查 `GET /api/apps/nav`，把启用的 `navItems` 渲染为顶级项（图标 + 名称，collapsed 态显图标）；点击走深链。`AppForm` 增「顶部导航」分区：`navItems` 编辑器（增删条目）+ 每条启用开关。
- **租户品牌（需求 4）**：新增 `BrandingProvider`——读 `GET /auth/branding`，把租户主色注入 `--brand-blue`（及相关 tint）CSS 变量、Logo 注入登录页品牌区与 `TopBar`。`Login` 品牌面板从写死 XGENT 改为「有租户品牌则用之，否则回落 XGENT」。
- **Settings/Profile**：无需大改；如平台允许租户 admin 自助维护域名，可在 `Settings` 加只读「自定义域名」分区（MVP 仅平台后台可写，此项可选）。

---

## 4. SDK + 示例应用增量

- **`packages/portal-sdk`**：新增 `scheduler.create(cron, {name, params})` / `scheduler.list()` / `scheduler.cancel(id)`（封装 `/api/v1/scheduler/*`）；`notify()` 沿用一期。`InitPayload` 无需变更。
- **`apps/sample-app`**：加一个「设定每日提醒」按钮 → 经 SDK `scheduler.create` 建一个 user-scoped 任务（webhook 回自身或发通知）；加一个「列出我的提醒/取消」视图，作为公共服务的被测对象。
- **i18n**：sample-app 内文案非门户 chrome，按一期边界保持原文即可。

---

## 5. 分阶段里程碑（每阶段含验证标准）

> 遵循 `CLAUDE.md §4`：每阶段先定可验证目标，循环到通过再进下一阶段。**UI 验证 = Chrome extension 真浏览器跑通**（tsc/脚本只证代码正确，不算功能验证）。

### Phase 0 · 平台租户 + 租户管理 App（需求 1）
- schema：`tenants.isPlatform`、`apps` 三新列、`native` 类型；seed 平台租户 + Rockie admin + 「租户管理」native App。
- `platform/service.ts` 抽取；`assertPlatformSession` + `/api/console/*` 会话路由；保留 `x-platform-key` + Swagger。
- 前端 `/console/*` 全套页面（租户/管理员 CRUD）；应用中心出现「租户管理」卡片、SideNav 项（仅平台租户 admin）。
- **验证（浏览器）**：以平台租户 admin 登录 → 应用中心见「租户管理」→ 打开 → 建/改/停/删租户、授/撤管理员均生效并写审计；以非平台租户 admin 登录**看不到**该 App；`x-platform-key` Swagger 通道回归不破。

### Phase 1 · App 贡献顶级导航（需求 2）
- `GET /api/apps/nav` + `AppForm` 导航编辑/启用 + `SideNav` 渲染。
- **验证（浏览器）**：管理员给某 App 声明多条 `navItems` 并启用其中两条 → 普通用户左栏顶级出现这两项 → 点击深链打开 App 到对应内部视图（复用一期 `?r=`）；未启用项不出现；停用 App 后其导航项消失。

### Phase 2 · 自定义域名 + 租户品牌（需求 4）
- `tenant_domains` + `resolveHostTenant` + `/auth/branding` + 登录默认租户 hint + 平台后台域名管理。
- 前端 `BrandingProvider` + 品牌化登录/Topbar。
- **验证（浏览器）**：平台后台给某租户加域名并标 verified → 用 `?__host=<域名>` 模拟访问 → 登录页显示该租户 Logo+主色 → 登录后活跃租户即该租户（用户是其成员时）；未验证域名/无映射时回落默认 XGENT 品牌与原默认租户。

### Phase 3 · 消息 + 计划任务公共服务（需求 5）
- scope `scheduler.read/write`；`tasks` 两新列；Open API `/api/v1/scheduler/*`（归属 + 隔离 + 上限）；消息服务文档化；`AdminTasks` 纳入 App 任务（只读标签）。
- portal-sdk `scheduler.*`；sample-app 提醒视图。
- **验证（脚本 + 浏览器）**：`scripts/verify-scheduler.ts`（建/列/改/删、归属隔离、越权 `TASK_NOT_OWNED`、缺 scope 403、每用户上限）；浏览器：sample-app「设定每日提醒」→ 任务出现在「我的提醒」与 `AdminTasks`（带来源标签）→ 立即触发 → 回调/通知到达；A 应用的 TDT 不能动 B 应用的任务。

### Phase 4 · 硬化与收尾
- 隔离复查：平台租户边界（非平台 admin 不能碰 `/api/console/*`）、域名仿冒（未验证域名不泄露品牌、不改默认租户）、App 任务隔离。
- i18n 三语 parity 复核（新增键三语对齐，跑 `lib/locales/index.ts` 键集 diff）；新 UI 焦点态/空错态/暗色/`prefers-reduced-motion`。
- seed/demo 完善；`CHECKLIST.md` 更新为 PLAN-2 勾选；`README` 补本期启动/验证说明。
- **验证**：全包 `tsc`；一期回归脚本全绿（verify-tdt/isolation/platform/consent/m1/...）+ 本期新脚本；主路径浏览器复跑。

---

## 6. 依赖与排序

```
Phase 0(平台租户/租户管理App) ─┬─ Phase 2(自定义域名/品牌：域名管理 UI 依赖 0 的 console)
                              ├─ Phase 1(App 顶级导航：独立，可与 2 并行)
                              └─ Phase 3(公共服务：独立，可与 1/2 并行)
                                                                  └─ Phase 4(硬化收尾)
```
- Phase 2 的域名管理界面寄生在 Phase 0 的租户管理 App 内 → 2 依赖 0。
- Phase 1 / Phase 3 与 0 弱耦合（仅共用 schema 迁移），可在 0 之后并行。
- Phase 4 收尾，依赖全部。

---

## 7. 验证策略（贯穿全程）

| 类型 | 手段 | 证明什么 |
| --- | --- | --- |
| 类型/契约 | `tsc` + 共享 Zod/枚举 | 代码正确、前后端契约一致、i18n 键 parity |
| 后端逻辑 | `bun test` + `scripts/verify-*.ts`（platform 会话门 / scheduler 归属隔离 / 域名解析 / nav 汇总） | 业务规则正确 |
| **UI 功能** | **Chrome extension 真浏览器走通主路径与关键边界** | 功能正确（强制） |
| 设计质量 | `impeccable` 介入设计阶段 | 视觉/层级/间距/态齐全（含品牌覆盖效果） |
| 隔离/安全 | 平台租户边界、域名仿冒、App 任务越权抽查；双通道 fail-closed | 不串权、不串租、不泄露品牌 |

> 若环境跑不起来（dev server / 浏览器连不上），显式声明「未在浏览器中验证」，不默认声称完成。

---

## 8. 风险与对策

| 风险 | 对策 |
| --- | --- |
| 平台管理双通道引入越权面 | service 层单点鉴权；`assertPlatformSession` 强校验 `isPlatform` admin；两通道均 fail-closed；隔离脚本覆盖非平台 admin 访问 `/api/console/*` 被拒 |
| 本地无法真跑 DNS，自定义域名难验证 | `?__host=` dev 模拟（受开关控制），生产读真实 Host；校验门只认 `verified` |
| 域名仿冒/未验证域名泄露租户品牌或劫持默认租户 | 仅 `verified` 域名参与品牌/默认租户；未命中回落 XGENT；hint 只在「用户确为成员」时生效 |
| self-scoped 任务的 cron 执行无在线 TDT 上下文 | TDT 仅创建/管理期需要；执行在服务端按 task 行跑，归属信息落库（`ownerUserId`/`createdByAppId`），触发时回填用户上下文 |
| App 滥建计划任务 | 每 (App,用户) 任务上限 `SCHEDULER_LIMIT` 兜底 + 既有进程级并发信号量 + per-(app,tenant) 限流 |
| 原生 App 混入 catalog 破坏 iframe 宿主逻辑 | `type==="native"` 在宿主/中心/启动器分支早返回走原生路由，不触发握手/TDT 注入；可见性按 `isPlatform` 网关 |
| i18n 破坏现有 parity | 新键三语同步；收尾跑键集 diff（沿用一期 489 键校验法） |

---

## 9. 完成定义（二期 Done）

1. 平台租户管理员可在门户内「租户管理」App（应用卡片 + 原生页面）完成租户/管理员/域名的全生命周期管理；非平台租户管理员完全不可见、不可达。
2. App 可声明多条顶级菜单项，租户管理员按条启用；普通用户左栏顶级导航看到启用项，点击经深链打开对应应用视图。
3. 租户绑定并验证自定义域名后，经该域名访问展示租户品牌（名/Logo/主色），登录后默认切到该租户；未验证/无映射回落默认 XGENT。
4. 下游 App 经用户态 TDT 使用平台消息服务（发通知给当前用户）与计划任务公共服务（创建/列出/取消归属自己的 user-scoped 定时任务），归属隔离与 scope 守卫严密；租户管理员对 App 任务有只读可见性。
5. 全部接口遵循一期 envelope/状态码约定；新 UI 还原 `DESIGN.md` 且经真浏览器验证；i18n 三语 parity；安全基线（双通道 fail-closed / 域名校验 / 任务隔离 / 上限兜底）就位；一期回归脚本与本期新脚本全绿。
```
