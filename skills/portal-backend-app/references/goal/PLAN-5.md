# PLAN-5 · XGENT.ai Portal 五期：研发项目管理 App（独立后端 · 多租户 · TDT 鉴权 · AI Agent 演示+预留）

> 站在一期底座（`goal/PLAN-1.md`）+ 二期平台化层（`goal/PLAN-2.md`）+ 三期应用市场/内容服务（`goal/PLAN-3.md`）+ 四期文件管理独立后端（`goal/PLAN-4.md`）之上。
> 本期把一套**已存在的单租户、无鉴权**的研发项目管理系统（"XGENT Track"，类 Linear，需求见 `goal/PMS.md`，前端已落地 `apps/spms-app`、后端已落地 `apps/spms-server`），**改造成一个上架应用市场的独立 App「研发项目管理」**。形态与四期文件管理一致：**App 自带独立后端**（`@xgent/spms-server`，自有 Elysia + Drizzle + 自有库），经门户 **TDT 自省（introspection）** 验证身份，经 **host 代理 `sdk.callService`** 与前端通信（iframe 侧零 CORS；spms-server 仍需 CORS 放行门户 host 源，见 §4.3）。门户侧**几乎零改造**：复用四期已建的自省端点 + 服务账号模型 + `callService` 宿主代理，只新增 `spms` 的 scope / 市场清单 / 服务注册表一条目 / CSP 放行。

---

## 0. 本期范围与决策（已与产品确认）

现有 `apps/spms-app` / `apps/spms-server` 是一套**完整但孤立**的应用：单租户、无登录、前端直连自己的 `/api`、自带主题/强调色/密度切换、纯中文。本期的本质是**"门户化（portalize）+ 换肤"**，不是重写业务。四个关键分叉确认如下：

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | App 形态 | **独立后端 App**（files 同构） | `spms-server` 是独立资源服务器（自有 DB `xgent-spms` + 自实现 `gate()`），**不并入** `apps/api`。门户只新增最小耦合：复用 §3 的自省端点 + 服务注册表。 |
| 2 | 前后端通信 | **host 代理 `sdk.callService("spms", …)`** | iframe **不直连** spms-server；调用经门户 host 注入 `aud=spms` 的 TDT 再转发。**iframe 侧零 CORS**（spms-server 仍需放行门户 host 源，见 §4.3）、后端可内网私有，与 files-app 完全一致。 |
| 3 | AI Agent 执行 | **演示 + 预留接口** | Atlas/Forge/Sentry/Scribe 仍是**按租户种子的成员**；指派后置 `aiAssigned` + 打 `AI 生成` 标签 + 写**脚本化活动流**（"工作区步骤"演示）。把"Agent 执行"抽象成一个**进程内事件分发 `dispatchAgentTask()`**（§4.6），当前仅演示实现，**预留**给后续真实 LLM worker 订阅。本期**不**接真实 LLM。 |
| 4 | 多租户 | **强制租户隔离** | 现有 schema 是单租户。每张顶层表加 `tenantId`，所有查询按 `claims.tenantId` 收口；Issue 展示编号（`AGT-318`）改为**租户内唯一**。这是门户 App 的硬约束（每个 TDT 都带 `tenant_id`，见 SSO 指引 §2.2），非可选。 |

> 命名：App 中文名「研发项目管理」，listingKey/appKey/服务注册名/TDT `aud` 统一为 **`spms`**；产品 wordmark 仍叫 **XGENT Track**；后端服务 `@xgent/spms-server`，前端微应用 `@xgent/spms-app`（已存在，本期改造而非新建）。

### 0.1 与四期「文件管理」的异同（复用什么）

| 维度 | 四期 文件管理 | 五期 研发项目管理（本期） |
| --- | --- | --- |
| 形态 | 独立后端 App | **同构** · 独立后端 App |
| 鉴权 | files-server 自实现 `gate()` 经门户自省验 TDT | **同构** · spms-server `gate()` 复用 `lib/gate.ts` 范式 |
| 通信 | `sdk.callService("files", …)` host 代理 | **同构** · `sdk.callService("spms", …)` |
| 门户耦合 | 自省端点 + 服务账号 + 市场 seed + SERVICE_REGISTRY | **零新机制** · 仅复用上述 + 加 `spms` 一条目 |
| 业务核心 | 对象存储 / 直传 / 分享（全新写） | **已存在** · Issue/看板/Scrum/路线图/PLC（只做门户化改造，不重写） |
| 外部依赖 | minio / S3 / `@aws-sdk` | **无**（纯 Postgres，更轻；本期不引入新基础设施） |

**结论：本期门户侧 100% 复用四期已就位的机制，工作量集中在 (a) spms-server 加租户 + 鉴权网关 + 身份映射，(b) spms-app 接 SDK + 换肤 + 三语 i18n。**

### 0.2 关键取舍（显式记录）

- **a. 复用四期服务账号机制，但 seed 独立 SA。** 自省端点 `POST /api/tokens/introspect`（PLAN-4 §3.1）是**通用**的——它只回 token 自带声明，不关心 `aud`。所以 spms-server **不需要**门户新增任何自省逻辑。为可独立轮换/审计，seed 一个**专用** `spms-server` 服务账号（capability `token.introspect`），**不**复用 files-server 的密钥。租户隔离由 TDT claims 里的 `tenant_id` 在 spms-server 侧收口（与 files 同理），与这把自省密钥无关。
- **b. Issue 展示编号租户内唯一 ⇒ 引入代理主键。** 现 `issues.id` 直接是展示键 `"AGT-318"` 且作 PK。多租户下两个租户都会产生 `AGT-1`，冲突。改为：`id uuid`（内部/外键）+ `key text`（展示编号，**`unique(tenantId, key)`**）。子表 FK 指向 `id`；前端展示/路由用 `key`（租户内唯一，经 TDT 收口）。这是一处**有迁移成本**的结构变更，显式记录。
- **c. 后端拿不到用户姓名 ⇒ 经 Open API 同步成员目录。** 自省只回 `user_id` / `role`，不回姓名。assignee 选择器要展示真人需要姓名/头像。方案：spms-server 持当前 TDT 调门户 `GET /api/v1/directory/users`（需 `directory.read`）拉本租户通讯录，**懒同步**为 `members` 行（按 `(tenantId, portalUserId)` upsert）。故清单 scope 含 `userinfo.read` + `directory.read`。
- **d. App 不再自带"外观"。** 现 spms-app 有自己的浅/深色、蓝/橙强调、紧凑/常规切换并持久化 localStorage。门户化后**主题/语言跟随宿主**（`onTheme`/`onLocale`），移除应用内的主题与强调色切换（密度可作为应用本地偏好保留，可选）。否则会与宿主"打架"。

### 0.3 明确不在本期

- 真实 LLM/Agent 执行（按确认项 #3 仅演示 + 预留 `dispatchAgentTask()` 接口）。
- 跨 App 读取 PMS 数据（如"工作台小组件"以外的下游 App 经令牌交换调 spms-server）——机制与 files §3.2 同（零新机制），但本期不打通样例，留作后续。
- 富 Scrum 之外的新功能（容量规划自动化、依赖图、自定义工作流引擎）——只搬现有功能并门户化，不新增需求。
- 历史单租户 seed 数据迁移——本期改为**按租户重建 seed**（晨光租户一份演示数据），不迁移旧库。

---

## 1. 拓扑与改造的工作区

```
apps/
  spms-server/   ◆改造 · 独立后端 (Elysia + Drizzle + postgres)  :4200
                 + 加租户列 + 自实现 gate() + 身份同步 + Agent 事件分发 + /api/v1/pms/* 前缀
  spms-app/      ◆改造 · 研发项目管理微应用 (Vite + React + shadcn)  :5177
                 + 接 @xgent/portal-sdk(握手/callService) + 换肤(.dark 调和/字体) + 三语 i18n + 去外观设置
  api/           门户后端 — 仅新增：spms 市场 seed + spms-server 服务账号 seed（复用自省端点，无新逻辑）
  web/           门户壳 — 仅新增：MicroAppHost SERVICE_REGISTRY 加 spms 一条 + CSP frame-src 加 :5177
packages/
  shared/        + pms.read / pms.write 两个 scope（shared SCOPE_LABELS 仅 zh-CN）+ 相关错误码
                 ※ 三语 consent 标签不在 shared，在 apps/web/src/lib/locales/labels.ts（见 §2.3）
  portal-sdk/    （可选）+ 类型化 sdk.pms.* 薄封装（底层仍 callService("spms", …)）；不做也可，App 直接用 callService
基础设施：无新增（纯 Postgres，spms-server 用独立库 xgent-spms；Redis 复用门户 REDIS_CONN_STRING 作自省缓存/限流，可降级内存）
```

端口：portal-api `:3000`、web `:5173`、sample `:5174`、todo `:5175`、files-app `:5176`、files-server `:4100`、**spms-app `:5177`**、**spms-server `:4200`**。

> ⚠️ **现 `apps/spms-app/vite.config.ts` 端口是 `5173`，与门户 web 冲突；现 `spms-server` 端口是 `3001`。两者本期必须改为 `5177` / `4200`。**（见 §9 坑位）

---

## 2. 数据模型

### 2.1 spms-server 侧（改造现有 `apps/spms-server/src/db/schema.ts`，新增迁移 `0002_multitenant`）

现有表：`members`（人类+Agent 同表）、`teams`、`labels`、`cycles`、`sprints`、`sprint_snapshots`、`projects`、`issues`、`issue_labels`、`sub_issues`、`activities`、`notifications`。改造要点：

| 表 | 变更 | 说明 |
| --- | --- | --- |
| 所有顶层表（members/teams/labels/cycles/sprints/projects/issues/notifications） | **+ `tenantId text not null`** + 复合索引 `(tenantId, …)` | 租户隔离的承载。所有列表/详情查询都加 `where tenantId = claims.tenantId`。 |
| `members` | + `portalUserId text`（人类，来自 TDT `user_id`）、+ `agentKey text`（Agent，如 `atlas`）；唯一 `(tenantId, portalUserId)`、`(tenantId, agentKey)` | 人类成员 = 门户用户在该租户的投影（§4.5 懒同步）；Agent = 按租户种子。`assigneeId` 仍 FK → `members.id`（保持代理主键 uuid）。 |
| `issues` | `id` 改 **uuid 代理主键**；+ `key text`（展示编号 `AGT-318`），唯一 `(tenantId, key)`；编号按 `(tenantId, teamId)` 递增（前缀即 teamId，§4.4） | 解决跨租户编号冲突（§0.2b）。子表 FK（`issue_labels`/`sub_issues`/`activities`/`notifications.issueId`）改指向 uuid `id` + 改 `lib/serialize.ts`；**API 对前端仍以展示 `key` 作 Issue 标识**（最小化前端改动）。 |
| 子表（`issue_labels`/`sub_issues`/`activities`） | FK 指向 `issues.id`（uuid）；**可选** 冗余 `tenantId` 作纵深防御 | 隔离主要靠父表 FK；冗余 `tenantId` 便于直接按租户清理/查询（按需）。 |
| `notifications` | + `tenantId`；保留 App 内"动态收件箱"语义（与门户共享收件箱并存，见 §4.7） | App 自有的 Issue 活动通知，**不**等于门户铃铛；门户铃铛经 §4.7 的 `notification.send` 另发。 |

> spms-server 保持自有 `db/schema.ts` + `drizzle.config.ts` + `db/migrate.ts`。⚠️ **现状核对**：这三处 + `db/index.ts` **当前都读 `process.env.DATABASE_URL`，缺省值是门户库 `xgent`**（`.env.example` 也是 `DATABASE_URL=…/xgent`、`PORT=3001`）。本期必须把它们统一改读 **`SPMS_DATABASE_URL`**（默认独立库 `xgent-spms`），否则 PMS 表会建进门户库、迁移链互撞（见 §9 坑位）。现有迁移 `0000/0001` 是旧单租户 schema，新增 `0002_multitenant` 作 alter（含 `issues` 主键 text→uuid 的重建——在全新 `xgent-spms` 空库上应用 `0000→0001→0002` 即可，无存量数据）。

### 2.2 门户侧（`apps/api`）

- **无新表、无新迁移。** 完全复用四期 `service_accounts` / `service_account_secrets`（PLAN-4 §2.1）与 `marketplace_listings` / `apps`。
- seed 增量（§5）：① `spms` 市场清单一条；② `spms-server` 服务账号一条（capability `token.introspect`）+ 初始密钥；③ 晨光租户预装 `spms`。

### 2.3 shared 增量（`packages/shared`）

- `scopes.ts`：`SCOPES` 追加 `pms.read` / `pms.write`；`SCOPE_LABELS`（**实为 zh-CN 单语 source**，非三语）追加两条中文标签。
- **`apps/web/src/lib/locales/labels.ts`**（`Catalog`，consent 授权屏经 `t("scope.<scope>")` 真正读这里）：追加 `scope.pms.read` / `scope.pms.write` 的 **zh-CN / en / zh-TW** 三语；清单 `cat="管理"` 若 `category.管理` 尚未存在也补三语。**这才是 scope 三语标签的真实落点，不在 shared。**（修正：原稿误以为 shared `SCOPE_LABELS` 是三语。）
- `errors.ts`：追加 `PMS_*` 业务错误码：`ISSUE_NOT_FOUND`、`SPRINT_NOT_FOUND`、`PROJECT_NOT_FOUND`、`TEAM_NOT_FOUND`、`MEMBER_NOT_FOUND`、`INVALID_TRANSITION`（状态非法流转，可选）、`INTROSPECT_FAILED`（若 shared 未有则加，files 已有则复用）。
- `dto.ts`（可选，若要做类型化 `sdk.pms.*`）：`IssueDTO`/`IssueDetailDTO`/`SprintDTO`/`BurndownDTO`/`VelocityDTO`/`BootstrapDTO`/`NotificationDTO`/`MemberDTO`——**直接对齐现 `apps/spms-app/src/lib/types.ts`**，避免重定义两份。若本期省事，可不进 shared，App 内复用既有 `types.ts`。

> 范围诚实说明：admin 专属操作（建/删 team、删 project 等"租户结构"变更）走 **`requireAdmin`（role 来自自省）**，**不**单列 `pms.admin` scope——保持授权屏最小（同 files：requireAdmin 而非额外 scope）。Issue/评论/子任务/Sprint 规划等日常操作归 `pms.write`。

---

## 3. 门户侧机制（`apps/api` + `apps/web`，复用为主）

### 3.1 TDT 自省（**零改造**，复用 PLAN-4 §3.1）

`POST /api/tokens/introspect` 已存在且通用：以 service account（Basic）或 dev `x-resource-key` 鉴权调用方 → 验签 + 撤销检查 + 查成员角色 → 回 `{active, kind, aud, tenant_id, user_id, scopes, role, exp}`。spms-server 用专用 `spms-server` SA 调它，**门户无需任何新逻辑**。

### 3.2 服务注册表（`apps/web` · 加一条目）

`apps/web/src/pages/MicroAppHost.tsx` 的 `SERVICE_REGISTRY` 当前只有 `files`。新增：

```ts
const SPMS_SERVER_BASE = import.meta.env.VITE_SPMS_SERVER_BASE ?? "http://localhost:4200";
const SERVICE_REGISTRY: Record<string, { baseUrl: string }> = {
  files: { baseUrl: FILES_SERVER_BASE },
  spms: { baseUrl: SPMS_SERVER_BASE },   // ★新增
};
```

宿主收到 iframe 的 `callService("spms", path, init)` → 按当前所托管 App 的 appKey（=`spms`）`mintToken` 得 `aud=spms` 的 TDT → 带 `Authorization: Bearer` 转发到 `SPMS_SERVER_BASE + path`。**这套逻辑 PLAN-4 已实现，本期只加注册表一行。**

### 3.3 服务账号 seed（`apps/api/db/seed.ts` · 镜像 files SA）

> 前置：`apps/api/src/lib/env.ts` 仿现有 `filesResourceKey`/`filesSaClientId`/`filesAppUrl`，新增 `spmsResourceKey = optional("SPMS_RESOURCE_KEY")`、`spmsSaClientId = optional("SPMS_SA_CLIENT_ID","spms-server")`、`spmsAppUrl = optional("SPMS_APP_URL","http://localhost:5177")`（下方 seed 代码引用）。

```ts
const spmsSaSecret = env.spmsResourceKey || "spms-dev-resource-key";
const [spmsSa] = await db.insert(serviceAccounts).values({
  name: "研发项目管理服务 (spms-server)",
  clientId: env.spmsSaClientId,          // "spms-server"
  status: "active",
  capabilities: ["token.introspect"],
  scopes: [],
}).returning();
await db.insert(serviceAccountSecrets).values({
  serviceAccountId: spmsSa.id,
  secretHash: await sha256(spmsSaSecret),
  prefix: spmsSaSecret.slice(0, 12),
  status: "active",
});
```

平台管理员可在 `/console` 服务账号页（PLAN-4 §3.4 已建）再轮换。

### 3.4 市场清单 seed（`apps/api/db/seed.ts` · 镜像 files 清单）

```ts
{
  listingKey: "spms",
  name: "研发项目管理",
  tagline: "类 Linear · Issue/看板/Scrum/路线图 · AI Agent 协作",
  desc: "面向研发团队的项目管理：Issue 列表/看板双视图与行内编辑、敏捷 Scrum（产品待办/Sprint 规划/燃尽/速度）、项目与周期路线图、产品生命周期(PLC)，内置 Atlas/Forge/Sentry/Scribe 四个可被指派的 AI Agent（演示）。由独立后端 spms-server 承载。",
  icon: "kanban",            // lucide 图标名（impeccable 终选）
  color: "#5B5BD6",          // App 身份色（紫靛，区别于 files 的 #7A5AF8 与品牌蓝）— App Glyph 用
  type: "micro",
  cat: "管理",
  embedUrl: env.spmsAppUrl,  // http://localhost:5177
  allowedOrigins: [env.spmsAppUrl],
  scopes: ["userinfo.read", "directory.read", "pms.read", "pms.write", "notification.send", "widget.write"],
  extPoints: ["dashboard.widget"],   // 贡献"研发现状"概览到门户工作台（§4.7）；widget.write 为既有 scope，无需新增
  navItems: [
    { id: "nav-issues", label: "Issues", icon: "list-checks", path: "/issues" },
    { id: "nav-board", label: "看板", icon: "kanban", path: "/board" },
    { id: "nav-sprints", label: "迭代", icon: "timer", path: "/sprints" },
  ],
  publisher: "XGENT",
  version: "1.0.0",
  status: "published",
  featured: true,            // 研发场景旗舰，可置顶（次于/并列待办，sort 自定）
  sort: 0,
}
```

晨光租户预装一份（同 files 的 `installListing` 范式）。

### 3.5 CSP 放行（`apps/web/index.html` · 必须）

`frame-src` 现为 `'self' http://localhost:5174 http://localhost:5175 http://localhost:5176` → **追加 `http://localhost:5177`**。typecheck/脚本查不出，只有真浏览器嵌入时报错（见 §9 坑位）。

---

## 4. spms-server 机制（核心改造）

### 4.1 自实现网关 `gate(headers, scope)`（复用 files-server `lib/gate.ts` 范式）

新增 `apps/spms-server/src/lib/{gate,env,crypto,redis,response}.ts`，直接以 `apps/files-server/src/lib/*` 为模板：

- 取 `Authorization: Bearer <TDT>` → 调门户 `PORTAL_INTROSPECT_URL`（带 SA Basic + dev `x-resource-key`）→ `active && aud==="spms"` 否则 `INVALID_TOKEN`；按 token hash 缓存至 `exp`（短 TTL）。缓存/限流后端：files-server **强依赖** Redis；本期为减依赖**有意允许降级为进程内 Map**（无 `REDIS_CONN_STRING` 时），是与 files 的刻意差异。
- **错误类型**：移植 files-server `lib/response.ts` 的 `AppError`（→ 200 + 错误体）/ `HttpError`（→ 对应 HTTP status）。gate 抛 `HttpError(401/403/429)`，须在 §4.2 的 `onError` 里**优先映射**它们。
- `scopes.includes(scope)` 否则 `INSUFFICIENT_SCOPE`。
- 限流：按 `(aud, tenant)` 每分钟固定窗（Redis incr）。
- 返回 `Claims { tenantId, userId, aud, scopes, role, exp }`。
- `requireAdmin(claims)`：`role==="admin"` 否则 `FORBIDDEN`（建/删 team、删 project 等结构变更）。
- `requireUser(claims)`：`userId` 非空否则 `FORBIDDEN`（PMS 所有写操作都需用户上下文；服务态令牌无 `user_id` ⇒ 拒）。

### 4.2 路由前缀与挂载（现有 4 个 route 模块改造）

- 现路由挂在 `/bootstrap`、`/issues`、`/sprints`、`/notifications`（前缀 `/api`，见 `apps/spms-app/src/lib/api.ts` 的 `BASE='/api'`）。**统一改前缀为 `/api/v1/pms/*`**（对齐 Open-API 风格与 files 的 `/api/v1/files/*`）：
  - `GET /api/v1/pms/bootstrap`、`GET|POST /api/v1/pms/issues`、`GET|PATCH|DELETE /api/v1/pms/issues/:key`、`POST /api/v1/pms/issues/:key/comments`、`PATCH /api/v1/pms/issues/:key/sub/:subId`、`GET|POST /api/v1/pms/notifications`、`POST /api/v1/pms/notifications/read-all`、`GET /api/v1/pms/sprints`、`GET /api/v1/pms/sprints/backlog`、`GET /api/v1/pms/sprints/:id`、`GET /api/v1/pms/sprints/:id/burndown`、`GET /api/v1/pms/sprints/velocity`、`PATCH /api/v1/pms/sprints/:id/issues/:issueKey`。
- 每个 handler 第一行 `const claims = await gate(headers, scope)`（读用 `pms.read`、写用 `pms.write`），随后**所有查询/写入按 `claims.tenantId` 收口**。`createApp()` 仍保持可 `app.handle(Request)` 进程内驱动（保留现有测试方式）。
- 响应信封不变：业务一律 `200 + {ok,data} | {ok:false,error}`（现 `lib/response.ts` 的 `ok`/`fail` 已符合 CLAUDE.md §3，保留）。现 `onError` 仅按 `code` 处理 VALIDATION/NOT_FOUND/INTERNAL（保留），**但它不识别 gate 抛出的 `HttpError`/`AppError`** —— 必须按 §4.1 在 `onError` 里**先映射** `HttpError`(→status) / `AppError`(→200 错误体)，否则 401/403/限流会落成 500。

### 4.3 CORS / 鉴权姿势

- iframe（:5177）**不直连** spms-server，所以 iframe 不增加 CORS 面。**但 host 代理那次 `fetch` 跑在门户 web 页面里（浏览器，:5173）⇒ 仍是跨源请求**：`createApp()` 的 `cors()` **必须**放行 **门户 host 源 `PORTAL_BASE_URL`**（必需，否则代理 fetch 被 CORS 拦），外加 spms-app 源 `SPMS_APP_URL`（仅 standalone 调试/健康检查）。与 files-server 实测一致（`cors({ origin: [filesAppUrl, portalBaseUrl], credentials, allowedHeaders:["Content-Type","Authorization"] })`）。"零 CORS" 仅指 iframe，不指 host。
- swagger `/docs` 可保留（开发便利），但它只是文档；真实数据访问仍走 gate。

### 4.4 Issue 编号（租户内唯一）

- 现状：`nextIssueId(teamId)` 生成 `${teamId}-${maxN+1}`（前缀就是 teamId，如 `AGT`），且**全局**扫描该 team 的所有 issue 取 max。改为：`n` 取 `(tenantId, teamId)` 下当前最大编号 + 1（事务内 `select max … for update` 或独立计数表），`key=${teamId}-${n}`，`unique(tenantId, key)` 兜底。
- 现"指派 Agent 自动置 `aiAssigned`"行为保留，并接 §4.6 事件分发。

### 4.5 身份映射 / 成员目录同步（§0.2c）

- **人类成员懒同步**：任一 gated 请求拿到 `claims.userId` 后，确保存在 `members` 行 `(tenantId, portalUserId=userId, type=human)`。姓名/缩写/头像色：首次同步时 spms-server 持**当前 TDT** 调门户 `GET /api/v1/directory/users`（`directory.read`）拉本租户通讯录，批量 upsert（含当前用户与其同事，供 assignee 选择器使用）。无 `directory.read` 时退化为仅用 `userinfo` 同步当前用户（名字缺省用 `user_id` 短码）。
- **AI Agent 种子**：四个 Agent（Atlas/Forge/Sentry/Scribe，`type=agent`，`agentKey` 固定）**按租户**懒建（首次该租户触达时插入，或在租户 bootstrap 脚本里建）。它们是 `assigneeId` 的合法目标。
- `assigneeId` / `leadId` / `aiLeadId` / 活动 `whoId` 等 FK 全部指向 `members.id`（租户内）。

### 4.6 AI Agent 事件分发（演示 + 预留接口，确认项 #3）

- 当 `PATCH /issues/:key` 或 `POST /issues` 把 `assigneeId` 设为某 Agent 成员时：置 `aiAssigned=true` → 确保打上租户的 `AI 生成` 标签 → 调用 `dispatchAgentTask({ tenantId, issueId, agentKey, kind: "assigned" })`。
- `lib/agents.ts` 暴露**单一扩展点** `dispatchAgentTask(evt)`：
  - **本期实现（演示）**：按 `agentKey` 写入一串**脚本化活动流**（`activities.kind='ai'`），还原 PMS.md 的"AI Agent 工作区卡片 · 实时步骤"观感（如 Forge：分析需求 → 生成实现草稿 → 提交草稿 PR）。可带轻微延时模拟"实时"（可选 setTimeout 链，进程内）。
  - **预留**：函数签名/事件载荷即为后续真实 worker 的契约（`{tenantId, issueId, agentKey, kind, payload?}`）。注释明确"真实 LLM 执行不在本期；订阅者后续接入"。**不**在本期引入 LLM SDK、队列或 webhook 表。
- 这样"演示"与"将来真接"之间只隔一个实现替换，不改调用方。

### 4.7 通知（App 内动态 + 门户铃铛）

- App 自有 `notifications` 表的"Issue 动态收件箱"语义**保留**（应用域，进 spms-server，按租户/用户过滤）。
- **额外**：在关键事件（被指派 Issue、被 @、Agent 完成演示）时，spms-server 持当前 TDT 调门户 `POST /api/v1/notifications`（`notification.send`）给**当前用户**发一条门户铃铛通知（落库+实时推送+按偏好邮件）。这是与门户生态的集成点，最小够用。
### 4.7bis 工作台小组件 —— "研发现状"概览（`dashboard.widget`）

让用户**不进 App 就能在门户工作台看到软件开发项目的现状**。spms-app 经 `sdk.contributeWidget("dashboard.widget", …)` 把一份**按当前用户/租户**计算的概览推给门户（PUT `/api/v1/widgets/dashboard.widget`，scope `widget.write`，既有 scope）。

- **数据来源**：spms-app 已有的 `bootstrap` / `issues` / 当前 Sprint 数据（经 `callService` 从 spms-server 取，已按租户收口），在前端聚合成计数，**不**新增后端聚合端点（保持最小；后续量大再下沉到 `/api/v1/pms/overview`）。
- **内容**（`{title, summary, items[], link}`，3–4 个 item，每个 `{label, value, hint, link}`，`link` 深链回 `/app/spms?r=…`）：
  - **我的进行中 Issue**：value=数量，link → `/issues?assignee=me&status=in_progress`。
  - **当前 Sprint**：value=剩余故事点（如 `42 pts`），hint=今日燃尽（如 `今日 -8`），link → `/sprints/:id`。
  - **活跃项目**：value=进行中项目数，hint=平均进度（如 `平均 63%`），link → `/projects`。
  - **AI Agent 进行中**：value=指派给 Agent 且进行中的 Issue 数，hint=如 `Forge·Sentry`，link → `/issues?assignee=agent`。
- **时机**：App 加载完成后贡献一次；在影响概览的写操作（建/改/指派 Issue、移入 Sprint）后**去抖**重推。卸载或登出由门户按 `(user,app)` 版本自然失效；如需可 `sdk.clearWidget("dashboard.widget")`。
- **i18n / 主题**：item 文案走 §5.3 词典；value 用数字、`pts` 等 mono 友好值；颜色跟随宿主（小组件由门户渲染，App 只供数据，不渲染样式）。
- **可见性**：贡献是**按用户**的，只对当前 TDT 用户可见（门户工作台机制，SSO 指引 §10.1）。

### 4.8 Seed / 租户 bootstrap

- ⚠️ **租户 UUID 是动态的**：门户 `db:seed` 每次重生成所有 UUID（既有 gotcha），且 spms-server 是**独立库、读不到门户租户表** ⇒ **不能硬编码晨光 id**。两条路并用：
  - ① **懒 bootstrap（日常 dev 默认）**：任一租户首次 gated 触达时，确保该租户的四个 Agent 成员 + 默认团队存在（轻量、随到随建，§4.5 已含 Agent 懒建）。
  - ② **演示数据脚本** `apps/spms-server/scripts/bootstrap.ts <tenantId>`：把目标租户 id **作为参数**传入（id 从登录后的 TDT/`userinfo` 或 `/console` 读到），写较重的演示集——teams（`AGT`/`WEB`…）、labels、projects（含 PLC `phase`）、cycles、一个 active sprint + 历史 snapshots（燃尽演示）、一批 issues（含指派给 Agent、带故事点）。
- **不要**把演示数据写进 spms-server 的"无参 seed 默认跑晨光"，因为它拿不到稳定的晨光 id。

---

## 5. 前端 spms-app 改造（强制两步法：`impeccable` 设计 + Chrome 真浏览器验证）

### 5.1 接入 portal-sdk（握手 / callService）

- `package.json` 依赖加 `@xgent/portal-sdk`、`@xgent/shared`（workspace），端口改 `5177`。
- `src/main.tsx` 重写为 files-app `main.tsx` 范式：
  - 在 iframe 内（`window.parent !== window`）→ `createPortalClient()` → `await sdk.ready()` → `applyTheme(init.theme)` → 渲染，并 `sdk.onTheme`/`sdk.onLocale` 实时跟随。
  - standalone（直接打开）→ 渲染一个"请在 XGENT Portal 内打开"提示页（同 files-app `Standalone`）。
  - 把 `sdk` + `init` + `locale` 通过 props/Context 注入下游（替换 `AppDataProvider` 内部的取数依赖）。
- `src/lib/api.ts` 改造：`http()` 不再 `fetch('/api'+path)`，改为 `sdk.callService<T>("spms", "/api/v1/pms"+path, init)`，保留现有 envelope 解包与 `ApiError`。具体做法：导出 `createApi(sdk)` 工厂；`main.tsx` 建好 sdk 后注入，`store/AppData.tsx` 从 Context 取 `api`。**TDT 全程由宿主管理，App 不持令牌、不碰 App Secret。**
- 移除 `vite.config.ts` 的 `/api` 代理（iframe 模式不需要）；如需 standalone 调试可保留一个指向 `:4200` 的可选代理。

### 5.2 换肤 / 设计对齐（"样式改成 Portal 设计风格"）

现 `apps/spms-app/src/index.css` **已移植 XGENT 设计 tokens**（颜色/阴影/圆角与 DESIGN.md 一致），所以换肤工作集中在**机制对齐**而非重画：

- **暗色机制调和（关键 · 已实测纠偏）**：`tailwind.config.js` 已是 `darkMode: ['class', '[data-theme="dark"]']`（`.dark` 与 `[data-theme=dark]` 都触发），**但 `index.css` 的 CSS 变量（真正的颜色）只在 `:root[data-theme='dark']` 下翻转**。所以正确做法是 `applyTheme(theme)` 设 **`document.documentElement.dataset.theme = theme`**（一举翻转 CSS 变量 + Tailwind `dark:`，且**不需要改任何 CSS 选择器**）。⚠️ **切勿照抄 files-app 的 `classList.toggle("dark")`**——那只翻 Tailwind 工具类、不翻 CSS 变量，会"半暗不暗"。（修正：原稿把 `.dark` 列为推荐，与本仓库实际相反。）强调色：移除 `:root[data-accent='orange']` 的应用内开关（缺省即蓝，跟随宿主）。
- **移除强调色切换**：删 `:root[data-accent='orange']` 的应用内开关（跟随宿主品牌蓝；如宿主未来推 accent 再说）。
- **字体加载**：确认 `index.html` 真正加载 Space Grotesk + JetBrains Mono（body 已声明 font-family，但需 `@fontsource` 或 link 引入，否则回退系统字体）。对齐 files-app/todo-app 的字体加载方式。
- **去重宿主 chrome**：App 在 iframe 内**不应**重复门户外壳（无租户切换、全局搜索、用户菜单、门户 wordmark）。保留 App 专属左侧视图导航（Issues/看板/迭代/项目/周期/路线图/收件箱）与顶部视图切换；移除 `SettingsPanel` 的主题/强调色控件（密度可留为本地偏好或一并移除）。`public/logo-wordmark*` 若是 XGENT Track 自身标识可保留为小 App 标识，但不得冒充门户壳。
- **状态/空态/动效**：补 `StateBlock`（空/错误/越权）+ `Skeleton`（加载）+ `prefers-reduced-motion` 降级（现 `aiPulse` 等动画需提供 reduced-motion 替代）。沿用 files-app `StateBlock` 范式。
- **App Glyph 身份色**：`#5B5BD6`（§3.4），用于市场卡片/启动器/导航的圆角方块 glyph（白色线性图标）。
- 全程用 `impeccable` skill 把关视觉层级/间距/对齐/可访问性（CLAUDE.md 前端两步法第一步）。

### 5.3 三语 i18n

- 现 App 为纯中文硬编码。新增 `src/lib/i18n.ts`（`makeT(locale)` + `normLocale`，对齐 files-app/todo-app），自带 **zh-CN / en / zh-TW** 词典，跟随 `init.locale` + `sdk.onLocale` 实时切换。
- 把各组件（IssuesView/ScrumViews/OtherViews/IssueDetail/CommandPalette/NewIssueModal/Sidebar/menus）的文案抽取进词典；三语 **parity**（key 一一对应）。状态/优先级/PLC 阶段/Agent 角色等枚举标签也进词典。

### 5.4 身份与现有交互

- "当前用户"取 `init.user`；assignee/lead 选择器数据来自 `bootstrap`（spms-server 已按 §4.5 同步成员目录）。
- 命令面板 `⌘K`、新建 `c`、`/` 等快捷键与**宿主**可能冲突（宿主也有 `⌘K`）。App 内快捷键保留，但需意识到宿主可能先截获（见 §9 坑位）；必要时改用非冲突键或在聚焦 App 时绑定。

### 5.5 工作台小组件贡献

按 §4.7bis：App 加载后用已取到的 `bootstrap`/issues/Sprint 数据聚合出"研发现状"四项，`sdk.contributeWidget("dashboard.widget", payload)` 推给门户；在影响概览的写操作后去抖重推。文案走 i18n 词典。**这是纯前端 SDK 调用（aud=spms 的 TDT 调门户 Open API），无需 spms-server 改动。**

---

## 6. 设计稿与样式（impeccable 把关点）

按 CLAUDE.md「前端 UI 开发」强制两步：

- **设计阶段（impeccable）**：自动加载 `PRODUCT.md` + `DESIGN.md`。重点核对——Issue 列表/看板的密度与"density with calm"、行内气泡菜单的 popover（Shadow 3 / `xpop`）、看板列与故事点徽章（badge tone）、燃尽/速度图的配色（品牌蓝主线 + 状态色，避免橙色泛滥）、IssueDetail 滑出抽屉（drawer，Shadow 4，`rounded.xl`）、AI 工作区卡片（避免"卡里套卡"、避免左侧色条 border-left——DESIGN.md 明令禁止）、Agent glyph/avatar、空/错/越权态。
- **验证阶段（Chrome extension）**：见 §7。

---

## 7. 验证

### 7.1 spms-server 回归脚本（`apps/spms-server/scripts/`，对真门户自省）

- `verify-gate.ts`：TDT 自省鉴权——有效命中 / `aud != spms` 拒 / scope 不足 `INSUFFICIENT_SCOPE` / 过期或撤销 `INVALID_TOKEN` / 服务态令牌（无 user_id）写操作被 `requireUser` 拒。
- `verify-tenant-isolation.ts`（**核心**）：两个租户各建 Issue/Project/Sprint，互相**查不到/改不到**对方数据；Issue `key` 在两租户可同名不冲突。
- `verify-identity.ts`：首次 gated 请求懒建当前用户为 member；`directory.read` 拉通讯录 upsert 同事；assignee 选择器有真人。
- `verify-issues.ts`：CRUD + 评论 + 子任务切换 + 租户内编号递增 + 缺失 Issue 返 `200 + data:null`（非 404）。
- `verify-scrum.ts`：backlog 排序 / Issue 移入移出 Sprint（`_backlog`）/ 燃尽（理想线+实际）/ 速度（承诺/完成+均速）。
- `verify-agent-assign.ts`：指派 Issue 给 Agent → `aiAssigned=true` + `AI 生成` 标签 + 脚本化活动流落库 + `dispatchAgentTask` 被调用（演示实现产出步骤）。
- `verify-notify.ts`：被指派触发门户 `notification.send`（铃铛收到）。
- 复用 files `scripts/_helpers.ts`（登录拿会话 → mint `aud=spms` TDT → 调 spms-server）范式。

### 7.2 门户脚本

- 复用 PLAN-4 `verify-introspect.ts`（确认 `spms-server` SA 也能自省）。无需新增门户逻辑。

### 7.3 Chrome 真浏览器（强制）

登录门户（dev `rockie`/`liming`）→ 应用市场找到「研发项目管理」→ 安装/打开 → consent 授权屏列出 `pms.read/pms.write/...` → 新建 Issue（按 `c`）→ 列表/看板切换 + 看板拖拽改状态 → 行内改状态/优先级/负责人 → 指派给 **Forge**（看到 `AI 生成` 标签 + 工作区卡片步骤）→ Sprint 规划（拖 backlog 进迭代）+ 看燃尽/速度 → IssueDetail 抽屉评论 → 切门户**语言为 English** 验证实时英文 → 切门户**暗色**验证 App 跟随变暗 → **回门户工作台看「研发现状」小组件**（我的进行中 Issue / 当前 Sprint 剩余 / 活跃项目 / AI Agent 进行中，点 item 深链回 App 对应视图）→ 用**另一租户**用户登录验证**看不到**晨光数据（隔离）+ 小组件数据也按租户/用户独立。

### 7.4 静态校验

- `bun run typecheck` 全 workspace 绿（含 spms-server / spms-app）。
- i18n 三语 parity（spms-app 词典 key 对齐；shared 新增 2 个 pms scope 标签 × 3 语）。

---

## 8. 分期（建议落地顺序）

1. **Phase 0 · 契约与门户接入（零新机制）**：shared 加 `pms.read/write`（+ zh-CN `SCOPE_LABELS`）+ 错误码；**`apps/web/src/lib/locales/labels.ts` 加三语 `scope.pms.*`**；`apps/api/src/lib/env.ts` 加 `spms*` 字段；门户 seed `spms` 市场清单 + `spms-server` 服务账号 + 晨光预装；`apps/web` SERVICE_REGISTRY 加 `spms` + CSP `frame-src += :5177`；spms-server/spms-app 端口改 4200/5177；**三处 DB 连接改读 `SPMS_DATABASE_URL`（独立库 `xgent-spms`）**；env 增量。→ `verify-introspect`（确认 SA 可自省）。
2. **Phase 1 · spms-server 多租户 + 鉴权网关**：schema 加 `tenantId` + Issue 代理主键/`key` 唯一（迁移 `0002`）；移植 `lib/{gate,env,crypto,redis,response}.ts`；路由改 `/api/v1/pms/*` 前缀并逐个 `gate()`；所有查询按 tenant 收口；编号租户内唯一。→ `verify-gate` / `verify-tenant-isolation` / `verify-issues`。
3. **Phase 2 · 身份映射 + 成员目录同步**：人类懒同步、`directory.read` 拉通讯录、Agent 按租户种子；seed/bootstrap 按租户重建演示数据。→ `verify-identity`。
4. **Phase 3 · Scrum / 通知**：sprints/backlog/burndown/velocity 全 gate + 租户收口；App 内动态收件箱 + 门户 `notification.send`。→ `verify-scrum` / `verify-notify`。
5. **Phase 4 · AI Agent 演示 + 预留接口**：指派→`aiAssigned`+标签+脚本活动流；`dispatchAgentTask()` 演示实现 + 契约注释。→ `verify-agent-assign`。
6. **Phase 5 · spms-app 接 SDK**：`main.tsx` 握手 + 主题/语言跟随 + standalone 兜底；`createApi(sdk)` 走 `callService`；去外观设置；去重宿主 chrome；**"研发现状"工作台小组件贡献（§4.7bis/§5.5）**。
7. **Phase 6 · 三语 i18n**：词典 + 文案抽取 + parity。
8. **Phase 7 · 设计/样式打磨（impeccable + Chrome）**：`.dark` 机制调和、字体加载、App Glyph 身份色、StateBlock/Skeleton/reduced-motion、密度。Chrome 真浏览器全链路验证（§7.3）。
9. **Phase 8 · 收尾**：README/CHECKLIST 增补；全验证脚本回归；typecheck + i18n parity。

---

## 9. 关键复用点 & 实现期坑位

**复用**：files-server `lib/{gate,env,crypto,redis,response}.ts` 作 spms-server 网关模板；门户 `POST /api/tokens/introspect`（**零改造**）；`service_accounts` seed 范式；`marketplace_listings` 安装范式；`MicroAppHost` 的 `callService` 宿主代理（只加注册表一行）；files-app `main.tsx`/`lib/i18n.ts`/`StateBlock`/`Standalone` 作 spms-app 改造模板；spms-app **现有业务组件与 `types.ts` 整体保留**（只改取数与文案）。

**坑位（务必记住）**：
- **端口冲突（最先踩）**：现 `spms-app` vite 端口 `5173` == 门户 web；现 `spms-server` `3001`。本期必须改 `5177` / `4200`，否则起不来或抢门户端口。
- **CSP `frame-src` 必加 `:5177`**（`apps/web/index.html`）——typecheck/脚本查不出，只有真浏览器嵌入时白屏/被拦。
- **暗色：用 `data-theme` 而非 `.dark`**：tailwind 两种触发都认，但 CSS 变量只认 `[data-theme='dark']`。`applyTheme` 设 `dataset.theme`，别照抄 files-app 的 `.dark` toggle，否则颜色不翻（§5.2，已纠偏）。
- **Issue 编号跨租户冲突**：必须代理主键 uuid + `unique(tenantId, key)`（§0.2b），否则两租户 `AGT-1` 撞 PK。
- **后端拿不到姓名**：自省只回 id/role；assignee 真人名要经 `directory.read` 同步（§4.5），别指望自省返回。
- **`aud` 必须是 `spms`**：`callService` 由宿主对**当前所托管 App** mint TDT；只有 SPMS 自身的 iframe 调 `callService("spms",…)` 时 `aud=spms` 才匹配。别的 App 想读 PMS 数据要走令牌交换（不在本期）。
- **spms-server 独立迁移链**：用 `SPMS_DATABASE_URL`（独立库 `xgent-spms`），别和门户/files 的 `db:generate`/`db:migrate` 混跑。
- **DATABASE_URL 缺省撞门户库（务必先改）**：现 `db/index.ts`/`migrate.ts`/`drizzle.config.ts` 三处都读 `process.env.DATABASE_URL`，缺省 `…/xgent`（=门户库）。不改就 migrate/seed，会把 PMS 表建进门户库。三处统一改读 `SPMS_DATABASE_URL`，并更新 `.env.example`（现仍是 `DATABASE_URL=…/xgent`、`PORT=3001`）。
- **gate 的错误要在 onError 里映射**：现 `onError` 不认 `HttpError`/`AppError`，gate 抛的 401/403/429 会落成 500。移植 files-server `lib/response.ts` 的两个错误类并在 onError 优先映射（§4.1/§4.2）。
- **租户 demo 数据别硬编码 id**：门户 `db:seed` 重生成 UUID，spms-server 又读不到门户租户表；演示数据脚本要把 `tenantId` 当参数传（§4.8），Agent 走懒建。
- **apps/api 需补 env 字段**：`apps/api/src/lib/env.ts` 仿 `filesResourceKey`/`filesSaClientId`/`filesAppUrl`，加 `spmsResourceKey`/`spmsSaClientId`/`spmsAppUrl`（§3.3/§3.4 的 seed 代码引用了它们）。
- **service account 是平台级、非隔离边界**：一把自省密钥服务所有租户是安全的（只回声明、不签发/伪造、不碰数据）；租户隔离由 claims `tenant_id` 在 spms-server 收口（同 files 已澄清）。seed **专用** `spms-server` SA，不复用 files 的密钥。
- **快捷键冲突**：宿主有全局 `⌘K`；App 内 `⌘K`/`c`/`/` 可能被宿主先截获——在 iframe 聚焦时再绑定，或换非冲突键。
- **门户 `apps/api` start 无 `--watch`**：改 seed/清单后需重启；`db:seed` 重生成 UUID ⇒ 重新登录（既有 gotcha）。
- **去外观设置要清干净**：移除 App 主题/强调色切换时，一并清掉可能写死 localStorage 强制主题的逻辑，避免与宿主主题打架。
- **AI Agent 是演示**：`dispatchAgentTask` 的"实时步骤"是脚本化的，不要在文案/UI 上暗示真实 LLM 正在运行；契约预留即可（确认项 #3）。

---

## 10. 需求（PMS.md）→ 设计映射（自检）

| PMS.md 能力 | 本期落点 |
| --- | --- |
| Issues 列表/看板双视图 + 拖拽改状态 | 现 `IssuesView` 保留；取数改 `callService` + 租户收口（§4.2/§5.1） |
| 行内编辑（状态/优先级/负责人） | 现气泡菜单保留；`PATCH /issues/:key`（`pms.write`，gate） |
| Issue 详情抽屉 + 子任务 + 活动流 + **AI 工作区卡片** | 现 `IssueDetail` 保留；活动流含 §4.6 脚本化 Agent 步骤（演示） |
| AI 指派（`aiAssigned` + `AI 生成` 标签） | §4.4/§4.6 指派 Agent 自动置位 + 标签 + `dispatchAgentTask`（演示+预留） |
| 项目/周期/路线图 + PLC 阶梯 | 现 `OtherViews` 保留；`projects.phase` 等租户收口 |
| 敏捷 Scrum（待办/Sprint/燃尽/速度/故事点） | 现 `ScrumViews` 保留；`/api/v1/pms/sprints/*` 全 gate + 租户收口（§4.2） |
| 收件箱（Agent/团队动态） | App 内 `notifications` 保留（应用域）+ 关键事件转门户铃铛 `notification.send`（§4.7） |
| 工作台看研发现状（本次新增诉求） | §4.7bis/§5.5 `dashboard.widget` 贡献："我的进行中 Issue / 当前 Sprint 剩余 / 活跃项目 / AI Agent 进行中"，深链回 App；`extPoints` + `widget.write`（§3.4） |
| 命令面板 ⌘K / 新建 c / 外观设置 | ⌘K/`c` 保留（注意宿主冲突，§9）；**外观设置移除**，主题/语言跟随宿主（§0.2d/§5.2） |
| 多租户（门户硬约束） | §2.1 全表 `tenantId` + §4.1 gate 收口 + `verify-tenant-isolation` |
| 上架应用市场 | §3.4 `spms` 清单 + §3.2 SERVICE_REGISTRY + §3.3 SA + §3.5 CSP |
| 样式改成 Portal 设计风格 | §5.2 换肤（tokens 已对齐，重在 `.dark` 调和/字体/去宿主 chrome/状态态）+ §6 impeccable 把关 |
| 三语支持（项目惯例） | §5.3 zh-CN/en/zh-TW parity，跟随宿主 locale |
