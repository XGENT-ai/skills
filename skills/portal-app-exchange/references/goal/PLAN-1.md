# PLAN-1 · XGENT.ai Portal 一期开发计划

> 多租户 SaaS 应用底座（Portal Shell）—— 第一期。
> 依据：`需求(功能列表 v1.0)` + `prototype/` + `PRODUCT.md` + `DESIGN.md` + `CLAUDE.md`。

---

## 0. 本期范围与决策（已拍板）

| 决策点 | 选择 | 含义 |
| --- | --- | --- |
| 范围策略 | **P0 优先 · 全模块铺开** | 把 M1–M12 的 **P0** 功能按原型做出真实后端；P1/P2 与平台级 **M13** 延后。 |
| 登录/SSO | **通用 OAuth 框架 · 全 provider 留桩** | 实现一套通用 OAuth2 授权码框架，GitHub/Google/Apple/微信/Lark 五个 provider 代码全接好，但因 `.env` 无真实凭据，本地不真跑；附配置说明。 |
| 微应用宿主 | **附示例微应用 · 端到端打通** | 额外做 `apps/sample-app` + `packages/portal-sdk`，真实演示 iframe 内嵌 + postMessage 桥 + TDT 注入 + 调 Portal API。 |
| 多租户 | **多租户 + 顶栏切换 · 种多个租户** | schema 全程带 `tenant_id` 隔离，种 2–3 个租户与跨租户用户，顶栏租户切换可用；不做 M13 平台后台。 |

### 0.1 关键假设（需明示）

1. **本地可验证登录**：因 OAuth provider 全部留桩、本地无法真跑第三方授权，为让"登录 → 会话 → TFA → 进入工作台"这条 P0 链路可在浏览器里被验证，本期**额外实现一个 `dev` mock IdP**（仅本地、由 `DEV_MOCK_OAUTH` 开关控制），它走完整的授权码往返，但 IdP 端是本地伪造页。生产/真实 provider 不受影响。
2. **Portal 会话 与 TDT 是两套凭证**：底座用户登录态用 **httpOnly Cookie + Redis 会话**（不可被下游应用读取）；下游应用访问 Portal API 用 **TDT（JWT）**。二者不混用。
3. **业务状态不走 HTTP 状态码**（遵循 `CLAUDE.md` 项目约定，见 §3）。
4. **前端 UI 强制两步法**（遵循 `CLAUDE.md`）：设计阶段用 `impeccable`，验证阶段用 Chrome extension 在真实浏览器跑通，再报完成。
5. 本期**不做** GitHub Actions / CI/CD（按需求）。

### 0.2 明确不在本期（延后到二期+）

- M1 P1/P2：邮箱密码登录、TFA 强制策略、账号绑定/解绑、会话设备远程登出、多租户风控提醒（短信 TFA 仅留桩）。
- M2/M6/M7/M8/M9 的 P1/P2：最近使用、收藏置顶持久化的高级形态、扩展点 Widget 的真实数据贡献（本期只做扩展点声明 + 占位渲染）、通知偏好邮件渠道真实投递、数据导出、无障碍增强项。
- M3 P1/P2：深链接路由同步、全屏/分屏多微应用并存、生命周期 `beforeUnload` 完整语义。
- M4/M5 P1：跨应用调用同意页、令牌策略高级项、Webhook 全事件矩阵、速率限制/配额（仅留最小实现，见 §6.2）、用户同意 Consent 页。
- M10–M12 P1/P2：CSV 导入、批量高级操作、审计留存策略、任务并发/超时高级控制（保留基础超时）。
- **M13 全部**（平台管理员后台、套餐配额、租户品牌定制、平台审计）。
- MX：i18n 仅搭骨架（zh-CN 完整 + en 占位），可观测性仅日志，无障碍达基线不做全量 P2。

---

## 1. 技术栈与总体架构

| 层 | 技术 |
| --- | --- |
| 后端 | **Bun** + **Elysia** + **Drizzle ORM** + **PostgreSQL** + **Redis** |
| 前端 | **Vite** + **React + TS** + **shadcn/ui** + **Tailwind CSS** |
| 实时 | Elysia WebSocket + Redis Pub/Sub |
| 调度 | `croner`（cron 解析/触发）+ `task_runs` 表 + Redis 分布式锁 |
| 令牌 | JWT（TDT，HS256/本地密钥）+ Redis 撤销表 |
| 校验 | 共享 Zod schema（`packages/shared`），前后端同源契约 |
| 包管理 | Bun workspaces（monorepo） |

### 1.1 Monorepo 结构

```
xgent-ai-portal/
├── package.json                 # bun workspaces 根
├── drizzle.config.ts
├── .env                         # 已存在；本期新增若干键（见 §2.1）
├── goal/PLAN-1.md
├── prototype/                   # 设计参考（只读，不改）
├── apps/
│   ├── api/                     # Elysia 后端
│   │   └── src/
│   │       ├── index.ts                 # 入口：装配模块/中间件/ws
│   │       ├── db/                       # schema.ts + client.ts + seed.ts + migrations/
│   │       ├── lib/                      # redis, jwt, crypto, response(envelope), errors
│   │       ├── middleware/               # session-guard, tenant-resolver, scope-guard, rate-limit
│   │       ├── modules/                  # 按功能模块分（auth/token/apps/inbox/...）
│   │       └── ws/                       # 实时通道
│   ├── web/                     # 前端门户外壳（the shell）
│   │   └── src/
│   │       ├── main.tsx, app/(router, providers)
│   │       ├── styles/tokens.css         # 来自 prototype/assets/colors_and_type.css
│   │       ├── components/                # shell(TopBar/SideNav) + 基于 shadcn 的原子
│   │       ├── pages/                     # 每条路由一个页面
│   │       └── lib/                       # api-client, auth, theme, ws-client, i18n
│   └── sample-app/              # 示例下游微应用（验证 SDK + TDT 端到端）
├── packages/
│   ├── shared/                  # 共享类型 + Zod + 响应信封契约 + scope 常量
│   └── portal-sdk/             # Portal SDK（postMessage 桥 + getToken/notify/theme...）
```

> **端口约定**：api `:3000`，web `:5173`，sample-app `:5174`。CORS 仅放行 web 源、带 credentials；宿主 iframe 加载 sample-app 源。

---

## 2. 环境与配置

### 2.1 `.env` 新增键（在现有两项基础上追加）

```ini
# 现有
POSTGRES_DATABASE_URL=postgres://postgres:postgres@localhost:5432/xgent-portal
REDIS_CONN_STRING=redis://localhost:6379/7

# 新增（本地开发）
API_BASE_URL=http://localhost:3000
PORTAL_BASE_URL=http://localhost:5173
SESSION_SECRET=<本地随机串>          # 会话 Cookie 签名/加密
TDT_SIGNING_KEY=<本地随机串>         # TDT(JWT) HS256 密钥
TDT_DEFAULT_TTL=3600                  # 平台默认 TDT 有效期(秒)
DEV_MOCK_OAUTH=true                   # 启用本地 mock IdP（仅本地验证用）

# OAuth provider（留空=留桩；填上即可启用对应 provider）
OAUTH_GITHUB_CLIENT_ID=
OAUTH_GITHUB_CLIENT_SECRET=
OAUTH_GOOGLE_CLIENT_ID=
OAUTH_GOOGLE_CLIENT_SECRET=
OAUTH_APPLE_CLIENT_ID=
OAUTH_APPLE_CLIENT_SECRET=
OAUTH_WECHAT_APP_ID=
OAUTH_WECHAT_APP_SECRET=
OAUTH_LARK_APP_ID=
OAUTH_LARK_APP_SECRET=
```

---

## 3. API 约定（项目级·强制）

遵循 `CLAUDE.md`：**HTTP 状态码只反映传输/路由层，不表达业务状态。**

- **业务成功**（含新建/更新/删除/覆写）统一 `HTTP 200` + 信封：
  ```json
  { "ok": true, "data": <T> }
  ```
- **业务失败**（校验不过 / 状态非法 / 资源冲突 / 数据不存在）也 `HTTP 200`：
  ```json
  { "ok": false, "error": { "code": "APP_DISABLED", "message": "应用已停用", "details": {} } }
  ```
- 数据不存在 → `200 + { ok:true, data:null }`（不是 404）。
- **仅以下走 4xx/5xx**：`400` 参数结构不正确（Zod 解析失败）、`401` 未登录、`403` 未授权/scope 不足、`404` 路由不存在、`5xx` 真服务/传输故障。
- 统一在 `lib/response.ts` 提供 `ok(data)` / `fail(code, msg, details)`；错误码集中在 `packages/shared` 枚举，前端据 `code` 做分支与文案。

---

## 4. 数据模型（Drizzle · 全表带租户隔离）

> 身份是**全局**的，租户成员关系是**每租户**的（支持"同一人在 A 租户是管理员、B 租户是用户"）。除全局身份表外，所有业务表带 `tenant_id` 且所有查询经 `tenant-resolver` 中间件强制注入租户作用域。

| 表 | 关键字段 | 对应模块 |
| --- | --- | --- |
| `tenants` | id, slug, name, plan, status, brand(json) | 多租户 |
| `users` | id, name, nickname, email, phone, avatar_url, status（全局身份） | M1/M8 |
| `identities` | user_id, provider, provider_uid, raw(json)（OAuth 绑定，支持多绑定） | M1 |
| `memberships` | user_id, tenant_id, role(admin/user), status, last_active_at | M1/M10 |
| `user_groups` | tenant_id, name | M10 |
| `user_group_members` | group_id, user_id | M10 |
| `tfa_secrets` | user_id, secret, enabled, backup_codes(hashed[]) | M1 |
| `apps` | tenant_id, app_id(公开), name, desc, logo_url, type(micro/link), cat, sort, landing_url, embed_url, redirect_urls[], allowed_origins[], scopes[], tdt_ttl, allow_exchange, exchange_whitelist[], show_in_center, visibility(all/group/user), pinned, webhook_url, ext_points[], status | M4 |
| `app_secrets` | app_id, secret_hash, prefix, status(active/revoked), created_at | M4/M5 |
| `app_visibility` | app_id, group_id?/user_id? | M2/M4 |
| `app_exchange_grants` | source_app_id, target_app_id, status | M4-F12/M5 |
| `favorites` | tenant_id, user_id, app_id | M2 |
| `notifications` | tenant_id, user_id, app_id, type, title, body, link, actions(json), unread, archived, created_at | M6 |
| `notification_prefs` | user_id, app_id, type, in_app, email | M6/M9 |
| `audit_logs` | tenant_id, time, actor_id, actor_type, event, object, result, ip, device, detail, diff(json) | M11 |
| `tasks` | tenant_id, name, cron, tz, action_type(webhook/builtin), target_app_id?, params(json), status, timeout_s, retries, alert_on_fail, next_run_at | M12 |
| `task_runs` | task_id, started_at, finished_at, duration_ms, result, log | M12 |
| `consents` | user_id, app_id, scopes[]（M5-F10 留表，UI 延后） | M5/M8 |

**种子数据（seed.ts）**：复用 `prototype/assets/data.jsx` 的 APPS / NOTIFICATIONS / USERS / AUDIT / TASKS，扩展为 2–3 个租户（晨光教育集团 / 星网在线学校 / 青舟培训中心）+ 一个跨租户用户（Rockie，在 A 租户 admin、B 租户 user），保证顶栏切换有真实数据。

---

## 5. 前端外壳与设计落地

### 5.1 设计 token 接入
- 将 `prototype/assets/colors_and_type.css` 作为**唯一真源**搬入 `web/src/styles/tokens.css`（CSS 变量：brand-blue、ink、slate-\*、status、字体、半径、间距、阴影、动效）。
- Tailwind `theme.extend` 映射这些变量；shadcn 的 CSS 变量（`--primary`=brand-blue、`--background`=paper、`--foreground`=ink、`--radius`=8px…）指向同一套 token。
- 暗色：`darkMode: ['selector', '[data-theme="dark"]']`，与原型 `data-theme` 引擎一致（明/暗/跟随系统）。

### 5.2 原子映射（shadcn 提供可访问基元 + token 提供皮肤）

| 原型原子 | 实现 |
| --- | --- |
| Button / IconBtn / Badge / Field(Input) / Checkbox / Radio / Toggle / Avatar / Skeleton / Table / Dialog(Modal) / Sheet(Drawer) / DropdownMenu·Popover | 基于 **shadcn** 改皮 |
| AppGlyph / StateBlock / Segmented / PageHeader / WidgetShell / TopBar / SideNav / TenantSwitcher / AppLauncher / NotificationBell / UserMenu | **自定义**（以 `prototype/assets/atoms.jsx` + `shell.jsx` 为还原基准） |

### 5.3 路由（镜像原型）
`/login` · `/login/tfa` · `/dashboard` · `/apps` · `/app/:appId` · `/inbox` · `/me` · `/settings` · `/admin/users` · `/admin/apps` · `/admin/apps/new` · `/admin/apps/:appId` · `/admin/audit` · `/admin/tasks`。
- 数据层 **TanStack Query** + 统一 api-client（按 §3 信封解包）。
- 全局 context：auth、theme、tenant、实时通知 store（WS 驱动铃铛角标）。

---

## 6. 核心机制设计

### 6.1 TDT 令牌服务与 API 网关（M5）
- **TDT = JWT**，claims：`{ iss, tenant_id, user_id, aud(app_id), scopes[], exp, jti }`，HS256（`TDT_SIGNING_KEY`）。
- 端点：
  - `POST /oauth/token` (grant=`authorization_code`) → 签发 aud=本应用的 TDT。
  - `POST /oauth/token` (grant=`urn:ietf:params:oauth:grant-type:token-exchange`) → 用 TDT_A 换 aud=B 的 TDT_B。
  - `POST /oauth/revoke` → 撤销。
- **校验中间件**：验签 + exp + tenant + aud + scopes；命中 Redis 撤销表（jti / per-app 版本号）即失效。
- **交换校验**：`app_exchange_grants(A→B)` 存在 + B `allow_exchange=true` + A 在 B 的 `exchange_whitelist` + **scope 取交集**（不可扩权）。
- **撤销联动**：应用停用/卸载、用户停用/移除 → 提升 per-app/per-user 令牌版本号，相关 TDT 立即失效（附录 A.3）。
- **Portal Open API v1**（按 scope 守卫）：`GET /api/v1/userinfo`、`GET /api/v1/directory/*`、`POST /api/v1/notifications`、`GET|PUT /api/v1/settings/*`。每次调用写审计（M5-F09）。

### 6.2 速率限制（最小实现）
Redis 令牌桶，按 (app, tenant) 维度限流，仅防滥用兜底；配额/计费延后。

### 6.3 实时收件箱（M6）
- Elysia WebSocket `/ws`，用会话 Cookie 鉴权；订阅 Redis 频道 `inbox:{tenant}:{user}`。
- 写通知（系统或下游应用经 `notification.send`）→ 落库 → `PUBLISH` → 在线 socket 即时推送 → 铃铛角标实时 +1。
- 断线重连：客户端带 `since` 拉取增量补偿。

### 6.4 调度（M12）
`croner` 在 API 进程内解析 cron（按租户时区 M12-F07），到点执行 `webhook`（调下游 Webhook URL，带超时/重试）或 `builtin`（如会话清理）；每次执行写 `task_runs`；连续失败经 Inbox 告警（M12-F05）；Redis 锁防重叠（M12-F06）。支持启停 + 立即执行一次。

### 6.5 OAuth 框架（M1）
- Provider 注册表：每个 provider = `{ authorizeUrl, tokenUrl, userinfoUrl, scopes, clientId, clientSecret, mapProfile }`，凭据从 env 读，缺失即"留桩"（按钮在，点击返回"未配置"提示）。
- `GET /auth/:provider/start` → state+PKCE → 重定向 IdP；`GET /auth/:provider/callback` → 换 code → 取 userinfo → **首登自动开户**（M1-F06，按租户域名/邀请归属）→ 建会话 → 若启用 TFA 转 `/login/tfa`。
- `dev` mock provider：本地伪 IdP 页，走同一套 callback 逻辑，使全链路可在浏览器验证。
- TFA（M1-F03）：`otpauth` TOTP，扫码绑定 + 校验 + 备份恢复码（hash 存）；短信 P1 留桩。
- 登录审计（M1-F10）：成功/失败、provider、IP、设备写 `audit_logs`。

---

## 7. 分阶段里程碑（每阶段含验证标准）

> 遵循 `CLAUDE.md §4`：每阶段先定可验证目标，循环到通过再进下一阶段。
> **UI 验证 = Chrome extension 真浏览器跑通**（类型检查/单测只证代码正确，不算功能验证）。

### Phase 0 · 地基与脚手架
- Bun workspaces；`apps/api`、`apps/web`、`packages/shared`、`packages/portal-sdk`、`apps/sample-app` 初始化。
- Drizzle 连 PostgreSQL、Redis 客户端连通；首版 migration；`.env` 新增键（§2.1）。
- `lib/response`(信封) + 错误码 + `session-guard`/`tenant-resolver` 中间件骨架 + `/health`。
- Vite+Tailwind+shadcn 初始化；接入设计 token；明暗主题；外壳（TopBar+SideNav）以 mock 数据渲染；路由搭好。
- **验证**：`bun dev` 同起 api+web；migration 通过；`/health` 返回 `{ok:true}`；浏览器打开 `/dashboard` 见外壳；主题切换生效。

### Phase 1 · 身份 / 会话 / OAuth 框架 / TFA / 多租户
- 表：tenants/users/identities/memberships/user_groups/tfa_secrets + seed（多租户、跨租户用户）。
- 通用 OAuth 框架 + 5 provider 留桩 + `dev` mock IdP；Redis 会话 + httpOnly Cookie；首登自动开户。
- TFA 绑定/校验/备份码；登录审计。
- 顶栏租户切换（上下文解析 + 切换端点 + 守卫）。
- 登录页 / TFA 页接真实流程（本地经 dev provider 验证）。
- **验证**（浏览器）：dev 登录 → TFA → 进入 dashboard；切换租户数据随之变；刷新会话保持；登出清会话。

### Phase 2 · TDT 令牌服务与 API 网关
- JWT 签发/校验；`authorization_code` 签发、`token-exchange` 交换（授权关系+开关+白名单+scope 交集）、`revoke`、刷新；撤销表。
- Portal Open API v1（userinfo / directory / notifications / settings，按 scope 守卫）；API 调用审计；最小限流。
- **验证**（后端，curl/`bun test`）：签发 TDT → 带它调 userinfo 通过；A→B 交换遵守开关/白名单/交集；revoke 后立即失效；scope 不足被 403。

### Phase 3 · 应用管理 + 应用中心 + 微应用宿主 + SDK + 示例应用（主链路）
- 表：apps/app_secrets/app_visibility/app_exchange_grants/favorites。
- 管理端：应用列表 + AppForm（**原型全字段**）；Secret 创建/轮换/吊销（仅创建/轮换时明文一次）；启用/停用/卸载（联动撤销 TDT + Webhook）。
- 应用中心：卡片网格 + 搜索 + 分类 + 收藏 + 详情浮层 + 可见性过滤 + 空态。
- 微应用宿主：受控 sandbox iframe + 加载/错误/超时/无权限/不可嵌入降级态。
- `packages/portal-sdk`：`ready/resize/navigate/getToken/notify/theme/locale` + 生命周期。
- `apps/sample-app`：最小微应用，经 SDK `getToken` → 调 `/api/v1/userinfo` 渲染 → 发一条通知。
- **验证**（浏览器·端到端主路径）：管理端注册 sample-app → 应用中心出现卡片 → 宿主打开 → SDK 握手 → 注入 TDT → 拉到 userinfo → 发通知 → 铃铛出现。

### Phase 4 · 收件箱与实时
- 表：notifications/notification_prefs。
- 收件箱页（左列表/右详情、筛选、已读/未读/归档/删除、批量、全部已读）。
- WebSocket 实时 + Redis Pub/Sub + 铃铛角标 + 下拉面板 + 断线补偿。
- **验证**（浏览器）：sample-app 发通知 → **不刷新**铃铛角标实时增长 → 进收件箱 → 读/归档/删除生效。

### Phase 5 · 治理：用户 / 审计 / 任务
- 用户管理：列表、邀请、角色分配、启停/移除（联动撤销会话+TDT）、TFA 重置；基础用户组（供可见范围用）。
- 审计日志：时间线、多维筛选、详情抽屉（消费各阶段写入的事件）。
- 计划任务：croner 调度、列表、创建/编辑、启停、立即执行、执行历史、重试、超时、失败 Inbox 告警、租户时区。
- **验证**（浏览器）：邀请/停用用户；审计多维筛选；建一个调 sample-app Webhook 的任务 → 立即执行 → 见历史 + 一条失败告警进 Inbox。

### Phase 6 · 个人资料与设置
- Profile：基本资料编辑；安全（TFA / 已绑定 IdP / 活跃会话 / 登录历史）；已授权应用 + 撤销。
- Settings：通用（语言/时区/主题）；通知偏好；隐私。扩展点贡献分区按应用 `ext_points` 声明渲染（基础占位）。
- i18n 骨架（zh-CN 完整 + en 占位）。
- **验证**（浏览器）：编辑资料保存；TFA 开关；主题/语言切换持久化；撤销一个应用授权。

### Phase 7 · 横切硬化与收尾
- 租户隔离复查（每条查询带 tenant 作用域）；CSP + iframe sandbox 响应头；密钥脱敏；焦点态/键盘可达基线；响应式抽查；骨架/空/错态审计；`prefers-reduced-motion`。
- seed/demo 脚本完善；根 `README` 启动说明。
- **验证**：关键页键盘走查；CSP 存在；隔离抽查（A 租户登录看不到 B 租户数据）；完整主路径浏览器走查一遍。

---

## 8. 依赖与排序

```
Phase 0 ─┬─ Phase 1(身份/会话) ─┬─ Phase 2(TDT) ─── Phase 3(应用/宿主/SDK/示例) ─┬─ Phase 4(收件箱实时)
         │                       │                                              │
         └───────────────────────┴────────────── Phase 5(治理) ────────────────┘
                                                  Phase 6(Profile/Settings) → Phase 7(硬化)
```
- Phase 2 依赖 Phase 1（会话→换 TDT）。
- Phase 3 是主链路，依赖 1+2；Phase 4 依赖 3（示例应用发通知）。
- Phase 5/6 可在 3 之后并行推进；Phase 7 收尾。

---

## 9. 验证策略（贯穿全程）

| 类型 | 手段 | 证明什么 |
| --- | --- | --- |
| 类型/契约 | `tsc` + 共享 Zod | 代码正确、前后端契约一致 |
| 后端逻辑 | `bun test` + curl（令牌/交换/撤销/网关/调度） | 业务规则正确 |
| **UI 功能** | **Chrome extension 真浏览器走通主路径与关键边界** | 功能正确（强制，UI 改动只看 diff 不算完） |
| 设计质量 | `impeccable` skill 介入设计阶段 | 视觉/层级/间距/态齐全 |
| 隔离/安全 | 跨租户抽查、CSP/sandbox 检查、密钥脱敏检查 | 租户强隔离、安全基线 |

> 若环境跑不起来（dev server / 浏览器连不上），显式声明"未在浏览器中验证"，不默认声称完成。

---

## 10. 风险与对策

| 风险 | 对策 |
| --- | --- |
| OAuth 无真实凭据，登录链路难验证 | `dev` mock IdP 走完整往返，本地端到端可验；真 provider 仅需填 env 即启用 |
| 微应用宿主无下游应用可测 | 自带 `apps/sample-app` 作为可控被测对象，端到端打通 |
| 跨应用令牌交换易出"扩权"漏洞 | 强制 scope 交集 + 三重校验（授权关系/开关/白名单）+ 撤销版本号；后端单测覆盖 |
| iframe 安全（X-Frame / postMessage 来源伪造） | sandbox + CSP + `allowed_origins` 白名单校验消息来源；不可嵌入自动降级新标签 |
| 多租户数据串租 | tenant-resolver 中间件统一注入作用域 + Phase 7 隔离抽查 |
| 调度在单进程内的可靠性 | 本期 croner + Redis 锁满足；若后续多实例，预留切换 BullMQ 的接口边界 |

---

## 11. 完成定义（一期 Done）

1. 浏览器内可走通 **附录 A.1 主链路**：dev 登录 → TFA → Dashboard → 应用中心打开示例微应用 → SDK 注入 TDT → 调 Portal API → 写收件箱并实时提醒。
2. 管理端可完成应用的注册/配置（原型全字段）/密钥轮换吊销/启停卸载，并触发 TDT 撤销联动。
3. 用户/审计/任务三块管理可用，审计能看到全程关键事件。
4. 多租户隔离与顶栏切换可用，种子含 2–3 租户与跨租户用户。
5. 全部接口遵循 §3 API 约定；UI 还原原型且经真浏览器验证；安全基线（CSP/sandbox/脱敏/最小权限/短时令牌）就位。
```
