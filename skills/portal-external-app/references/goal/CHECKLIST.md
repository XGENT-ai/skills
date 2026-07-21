# PLAN-1 · 开发清单 (CHECKLIST)

> 一期（P0 优先 · 全模块铺开）实现状态。`[X]` 已完成并在浏览器/脚本验证，`[ ]` 未做（本期明确延后到二期+）。
> 依据 `goal/PLAN-1.md`。验证脚本见 `apps/api/scripts/`，启动见根 `README.md`。

---

## ✅ 一期已完成

### Phase 0 · 地基与脚手架
- [X] Bun workspaces monorepo（`apps/{api,web,sample-app}` + `packages/{shared,portal-sdk}`）
- [X] Drizzle 连 PostgreSQL、Redis 客户端连通；首版 migration（18 张表）
- [X] `.env` 新增键（SESSION_SECRET / TDT_SIGNING_KEY / DEV_MOCK_OAUTH / OAUTH_* …）
- [X] `lib/response`（信封 ok/fail）+ 错误码枚举（`packages/shared`）
- [X] `session-guard` / `tenant-resolver` 中间件 + `/health`
- [X] Vite+Tailwind+设计 token（来自 `prototype/`）+ 明暗主题（`data-theme`）
- [X] 外壳 TopBar + SideNav + 路由（镜像原型）
- [X] 验证：`bun dev` 同起 api+web；`/health` 返回 `{ok:true,db,redis}`；浏览器外壳渲染 + 主题切换

### Phase 1 · 身份 / 会话 / OAuth / TFA / 多租户
- [X] 表：tenants/users/identities/memberships/user_groups/tfa_secrets + seed（2–3 租户、跨租户用户 Rockie）
- [X] 通用 OAuth2 授权码框架 + 5 provider 留桩（GitHub/Google/Apple/微信/Lark，缺凭据=未配置）
- [X] `dev` mock IdP（本地伪 IdP，走完整授权码往返）
- [X] Redis 会话 + httpOnly Cookie；首登自动开户
- [X] TFA：TOTP 绑定 / 校验 / 备份恢复码（hash 存）
- [X] 登录审计（成功/失败、provider、IP、设备）
- [X] 顶栏租户切换（解析 + 切换端点 + 守卫）
- [X] 登录页 / TFA 页接真实流程
- [X] 验证（浏览器）：dev 登录 → TFA → dashboard；切租户数据随变；刷新保持；登出清会话

### Phase 2 · TDT 令牌服务与 API 网关
- [X] TDT = JWT（HS256），claims `{iss,tenant_id,user_id,aud,scopes,exp,jti,av,uv,uav}`
- [X] `POST /oauth/token` — `authorization_code` 签发
- [X] `POST /oauth/token` — `token-exchange`（授权关系 + 开关 + 白名单 + **scope 取交集**）
- [X] `POST /oauth/token` — `refresh_token`（轮换）
- [X] `POST /oauth/revoke` + Redis 撤销表（jti / per-app / per-user / per-user-app 版本号）
- [X] 校验中间件：验签 + exp + tenant + aud + scope + 撤销命中
- [X] Portal Open API v1：`userinfo` / `directory/*` / `notifications` / `settings/*`（按 scope 守卫 + 调用审计）
- [X] 最小限流（Redis，按 app+tenant）
- [X] 验证：`scripts/verify-tdt.ts` 19/19（签发/交换/交集/撤销/scope 403/401）

### Phase 3 · 应用管理 + 应用中心 + 微应用宿主 + SDK + 示例应用
- [X] 表：apps/app_secrets/app_visibility/app_exchange_grants/favorites
- [X] 管理端：应用列表 + AppForm（原型全字段）
- [X] Secret 创建 / 轮换 / 吊销（明文仅一次）
- [X] 启用 / 停用 / 卸载（联动撤销 TDT：提升 app 版本号 + Webhook 通知）
- [X] 应用中心：卡片网格 + 搜索 + 分类 + 收藏 + 详情浮层 + 可见性过滤 + 空态
- [X] 微应用宿主：受控 sandbox iframe + 加载/错误/超时/无权限/不可嵌入降级态
- [X] `packages/portal-sdk`：ready/getToken/userinfo/notify/onTheme/resize/navigate + 生命周期
- [X] `apps/sample-app`：经 SDK getToken → 调 `/api/v1/userinfo` 渲染 → 发通知
- [X] 验证（端到端）：注册 → 中心出卡片 → 宿主打开 → 握手 → 注入 TDT → 拉 userinfo → 发通知 → 铃铛

### Phase 4 · 收件箱与实时
- [X] 表：notifications/notification_prefs
- [X] 收件箱页（左列表/右详情、筛选、已读/未读/归档/删除、批量、全部已读）
- [X] WebSocket（Elysia `/ws`，会话 Cookie 鉴权）+ Redis Pub/Sub
- [X] 铃铛角标实时 + 下拉面板 + 断线重连补偿（onResync）
- [X] 验证（浏览器）：外部发通知 → **不刷新**铃铛与收件箱实时 +1 → 读/归档生效

### Phase 5 · 治理：用户 / 审计 / 任务
- [X] 用户管理：列表、邀请、角色分配、启停/移除（联动撤销会话+TDT）、TFA 重置、基础用户组
- [X] 审计日志：时间线 + 多维筛选（事件/结果/搜索）+ 详情抽屉
- [X] 计划任务：`croner` 调度（按租户时区）、列表、创建/编辑、启停、立即执行、执行历史、超时、重试
- [X] 失败 Inbox 告警 + Redis 锁防重叠
- [X] 验证（浏览器）：邀请/停用用户；审计筛选；建 webhook 任务 → 立即执行 → 历史 + 失败告警进 Inbox（实时）

### Phase 6 · 个人资料与设置
- [X] Profile：基本资料编辑
- [X] 安全：TFA 开启/关闭（二维码密钥+备份码）、已绑定 IdP、活跃会话、登录历史
- [X] 已授权应用 + 撤销（per-user-app TDT 立即失效）
- [X] Settings：通用（语言/时区/主题）、通知偏好、隐私、扩展点贡献分区（按 `ext_points` 占位）
- [X] i18n 骨架（zh-CN 完整 + en 占位）
- [X] 验证（浏览器）：编辑资料保存；TFA 开关；主题/语言切换持久化；撤销一个应用授权

### Phase 7 · 横切硬化与收尾
- [X] 租户隔离复查（每条业务查询带 tenant 作用域）
- [X] CSP（SPA：frame-src 限制内嵌、frame-ancestors）+ iframe sandbox + API 安全响应头（nosniff/X-Frame-Options/Referrer-Policy）
- [X] 密钥脱敏（hash 存、明文仅一次）
- [X] 焦点态/键盘可达基线（focus-ring）+ `prefers-reduced-motion`
- [X] 骨架/空/错态（Skeleton / StateBlock）
- [X] seed/demo 完善 + 根 `README`
- [X] 验证：`scripts/verify-isolation.ts` 9/9（跨租户隔离 + RBAC + 安全头）；CSP 不破坏应用、iframe 仍可嵌入；全主路径浏览器复跑

### 完成定义（§11 · 一期 Done）
- [X] 浏览器走通主链路：dev 登录 → TFA → Dashboard → 应用中心打开示例微应用 → 注入 TDT → 调 Portal API → 写收件箱并实时提醒
- [X] 管理端：应用注册/配置（全字段）/密钥轮换吊销/启停卸载 + TDT 撤销联动
- [X] 用户/审计/任务三块治理可用，审计可见全程关键事件
- [X] 多租户隔离 + 顶栏切换，种子含 2–3 租户与跨租户用户
- [X] 接口遵循 §3 信封约定；UI 还原原型且经真浏览器验证；安全基线（CSP/sandbox/脱敏/最小权限/短时令牌）就位

---

## ⏸ 本期不做（延后到二期+，依据 §0.2）

### M1 身份 P1/P2（部分 · 见下「二期增量」）
- [X] 邮箱密码登录
- [X] 账号绑定/解绑
- [X] 会话设备远程登出
- [ ] TFA 强制策略
- [ ] 多租户风控提醒（短信 TFA 仅留桩）

### M2/M6/M7/M8/M9 P1/P2
- [ ] 最近使用、收藏置顶的持久化高级形态
- [X] 扩展点 Widget 的真实数据贡献（见下「二期增量 · M2/M7 扩展点 Widget」）
- [X] 通知偏好邮件渠道真实投递（见下「二期增量 · M6/M9 邮件渠道」）
- [ ] 数据导出
- [ ] 无障碍增强项（达基线，不做全量 P2）

### M3 微应用宿主 P1/P2（部分 · 见下「二期增量」）
- [X] 深链接路由同步
- [X] 生命周期 `beforeUnload` 守卫（脏标记 + 离开确认）
- [~] 全屏（单应用全屏已做；分屏多应用并存延后）

### M4/M5 P1（部分 · 见下「二期增量」）
- [X] 用户 Consent 同意页（首开应用先授权 scope，再注入 TDT）
- [X] 速率限制 → 按租户套餐配额（plan 驱动）
- [X] Webhook 事件扩展（app.enabled/disabled/uninstalled + consent.granted/revoked）+ **per-app 事件订阅矩阵**（按应用过滤投递）
- [X] 跨应用交换的同意页（exchange consent gate + 授权页 + Profile 撤销）
- [X] 令牌策略高级项（allowRefresh / refreshTtl / allowedGrants 授权方式白名单）

### M10–M12 P1/P2
- [ ] CSV 导入
- [ ] 批量高级操作
- [ ] 审计留存策略
- [X] 任务并发/超时高级控制（见下「二期增量 · M12 任务并发/超时」）

### M13 平台管理员后台（部分 · 见下「二期增量」）
- [X] **平台管理员 API（仅 API · 经 Swagger 操作）**：租户 CRUD + 租户管理员 CRUD，`x-platform-key` 操作凭据守卫（fail-closed）
- [ ] 平台管理员前端 UI（本次只做 API）
- [ ] 套餐配额（plan 目前为自由文本，无配额校验）
- [ ] 租户品牌定制（brand 字段可写，无专门定制界面）
- [ ] 平台审计（平台操作已写 `audit_logs`，无专门审计视图）

### MX 横切
- [X] i18n 全量（默认 zh-CN + en + zh-TW 全量翻译；见下「二期增量 · MX i18n 全量」）
- [ ] 可观测性（本期仅日志）
- [ ] 无障碍全量 P2（本期达基线）
- [ ] GitHub Actions / CI/CD（按需求本期不做）

### 真实 OAuth Provider（凭据相关）
- [ ] GitHub/Google/Apple/微信/Lark 真实登录（代码已接好，`.env` 填入凭据即启用；本地用 dev mock IdP 验证）

---

## 二期增量（在一期完成后追加）

### M1 身份 P1（邮箱密码 / 账号绑定 / 远程登出）
- [X] 邮箱密码登录：`users.password_hash`（Bun argon2id）+ `POST /auth/password/login`（设 cookie，返回 pendingTfa）+ `POST /api/me/password`（设/改密，改密需验当前）；登录页加邮箱密码表单
- [X] 账号绑定/解绑：`GET /auth/:provider/link`（带 linkUserId 走同一授权码往返，dev mock 支持自定义 uid）→ 回调 `linkIdentity`（已绑他人 → CONFLICT）；`POST /api/me/identities/:id/unbind`（拒绝解绑唯一登录方式）；安全页加绑定/解绑
- [X] 会话设备远程登出：`/api/me/sessions` 返回 sid 的哈希句柄（不暴露 sid）+ `POST /api/me/sessions/:id/revoke`（登出当前则清 cookie）；安全页非当前会话加「登出」
- [X] 验证：`scripts/verify-m1.ts` 16/16（密码设/改/登录、绑定+冲突、解绑、远程登出）；浏览器走通密码登录 → dashboard、安全页四张卡（密码/绑定/会话）渲染与「登出」生效
- [ ] 仍延后：TFA 强制策略、短信 TFA、多租户风控提醒

### M13 平台管理员 API（仅 API · Swagger 操作）
- [X] 操作凭据守卫：`x-platform-key` 头校验 `PLATFORM_ADMIN_KEY`；缺凭据 fail-closed（403），缺/错 → 401/403
- [X] 租户：列表（含成员/管理员/应用计数）、创建（slug 唯一 + 校验）、详情（含管理员）、更新（slug 不可改、`status=suspended` 停用）、删除（非空拒绝，`?force=true` 级联）
- [X] 租户管理员：列表、授予（email 自动开户 / userId 指定）、撤销（降级为成员；拒绝移除最后一名管理员）
- [X] 平台操作写 `audit_logs`（`actorType=platform`；删除租户记为平台级 `tenantId:null` 以免被级联清除）
- [X] Swagger UI：`@elysiajs/openapi`（provider `swagger-ui`）挂 `/swagger`，含 `platformKey` apiKey 方案 + Authorize
- [X] 验证：`scripts/verify-platform.ts` 27/27（鉴权门 / 租户 CRUD / 管理员 CRUD / 删除守卫）；浏览器打开 `/swagger` 渲染出 Platform Admin 全部端点
- [ ] 仍延后：平台管理前端 UI、套餐配额校验、品牌定制界面、平台审计视图

### M4/M5 P1（用户 Consent 同意页）
- [X] 同意门：`/api/tokens/mint` 在签发 TDT 前校验 `hasConsent`，未授权 → `CONSENT_REQUIRED`（带 appName + scopes）
- [X] 同意端点：`GET /api/me/consent/:appKey`（needsConsent + required/granted）、`POST /api/me/consents`（按应用声明的 scope 记录授权，写审计「授权应用」）
- [X] 宿主同意屏：`MicroAppHost` 在挂载 iframe 前若 needsConsent 则渲染授权屏（scope 列表 + SCOPE_LABELS），授权后再握手/注入 TDT；握手超时只在 iframe 显示后计时
- [X] 验证：`scripts/verify-consent.ts` 9/9（needsConsent → mint 拦截 → 授权 → mint 通过）；浏览器走通：撤销 → 打开应用见授权屏 → 授权并打开 → 应用加载并拉到 userinfo
- [X] 配额：`planRateLimit(plan)` + `resolveTenantLimit`（Redis 缓存 60s）驱动 Open API 限流（旗舰 600/专业 300/标准 120/分）
- [X] Webhook 事件扩展：状态切换补发 `app.enabled`；授权/撤销补发 `consent.granted`/`consent.revoked`（事件目录在 `@xgent/shared` WEBHOOK_EVENTS）
- [X] 验证：`scripts/verify-m45.ts` 10/10（配额映射+限流机制；本地接收端实测 app.disabled/enabled/uninstalled 投递 + per-app 事件订阅过滤）

### M4/M5 P1 收尾（跨应用交换同意页 / 令牌策略高级项 / per-app 事件订阅矩阵）
- [X] **跨应用交换同意页**：新表 `exchange_consents`（user×source×target）；`token-exchange` 在三重结构校验+scope 交集后再加用户同意门，未授权 → 新错误码 `EXCHANGE_CONSENT_REQUIRED`（带 source/target/scopes）。新端点 `GET /api/me/exchange-consent/:source/:target`（needsConsent + exchangeAllowed）、`POST /api/me/exchange-consents`、`GET /api/me/exchange-consents`、`POST …/:source/:target/revoke`。前端 `/exchange-consent?source=&target=&return=` 授权页（源→目标 + scope + 未配置告警），Profile › 已授权应用新增「跨应用授权」卡片可撤销。种子预置李明 mistakes→classroom 同意。
- [X] **令牌策略高级项**：apps 新增 `allowRefresh` / `refreshTtl` / `allowedGrants`（授权方式白名单，新错误码 `GRANT_NOT_ALLOWED`）。`/oauth/token` 三个 grant 分支按策略守卫：关闭刷新→不返回/拒绝 refresh_token，按 `refreshTtl` 续期；`allowedGrants` 限制各应用可用授权方式（含源应用是否可发起 token-exchange）。AppForm「高级令牌策略」分区可视化编辑。
- [X] **per-app 事件订阅矩阵**：apps 新增 `webhookEvents`（默认全量保持兼容）；`deliverWebhook(app,event,payload)` 单一投递口按订阅过滤，替换所有 `fireWebhook` 直调点。AppForm「事件订阅矩阵」勾选格 + 全选/全不选。
- [X] 验证（脚本）：`scripts/verify-exchange-consent.ts` 14/14（拦截→状态→授权→交换通过→列表→撤销→再拦截→未配置拒绝）；`scripts/verify-token-policy.ts` 8/8（refresh 签发/续期/关闭拦截、allowedGrants 拒绝 authorization_code 与源侧 token_exchange）；`scripts/verify-m45.ts` +2（订阅过滤）。回归 verify-tdt 19/19 / verify-consent 9/9 / verify-isolation 9/9 / verify-m1 16/16 / verify-platform 27/27 全绿。
- [X] 验证（浏览器）：rockie@sunrise 走通 `/exchange-consent` 授权页（mistakes→classroom 授权成功；mistakes→study 未配置告警+禁用授权）；Profile「跨应用授权」卡片出现并可撤销；AppForm 高级令牌策略 + 事件订阅矩阵渲染并保存成功。

### M3 微应用宿主增强（深链接 / 生命周期 / 全屏）
- [X] SDK 扩展（`packages/portal-sdk`）：`routeSync`/`onRoute`、`setDirty`、`requestFullscreen`/`onFullscreen`；`InitPayload` 加 `route`/`fullscreen`；新增 host 事件 `route`/`fullscreen`
- [X] 深链接路由同步：宿主把内嵌应用内部路由写入 `/app/:appId?r=…`，握手时经 `init.route` 还原；应用 `routeSync` 反向更新地址栏
- [X] 生命周期守卫：应用 `setDirty(true)` → 宿主在「返回」时弹自定义确认弹层（非原生 dialog），并挂 `beforeunload` 拦截标签关闭/刷新
- [X] 全屏：宿主头部全屏开关（fixed inset-0），双向 `fullscreen` 事件；`sample-app` 加两视图 + 脏标记 + 全屏按钮作为被测对象
- [X] 验证（浏览器）：切换视图 → 地址栏 `?r=/detail` 同步；带 `?r=/detail` 刷新 → 还原详情视图；全屏铺满/退出；脏标记 → 返回弹「离开应用？」确认
- [ ] 仍延后：分屏多微应用并存、浏览器前进/后退完整路由历史语义

### M6/M9 邮件渠道（通知偏好邮件真实投递）
- [X] `lib/mail.ts`：nodemailer SMTP 传输（从 `.env` SMTP_* 读取；465=implicit TLS→secure）；启动探活日志；`mailEnabled()`/`verifyTransport()`/`sendMail()`/`renderNotificationEmail()`（品牌化 HTML 模板）
- [X] `modules/inbox/notify.ts` · `createNotification()`：落库 + Redis 实时推送 + 按偏好邮件投递的**单一收口**；`emailEnabledFor(user,app,type)` 偏好优先级解析（app+type 最具体 > 全局 `*`）；邮件投递 fire-and-forget（不阻塞通知）
- [X] 收口替换两处通知创建点：Open API `POST /api/v1/notifications`、任务失败 `alertAdmins`
- [X] 验证：`scripts/verify-email.ts` 9/9（SMTP 连通/鉴权 + 偏好优先级 + 真实发信 + createNotification 端到端）；浏览器：设置页「邮件通知」开关在三语言下渲染

### M12 任务并发/超时高级控制
- [X] 全局并发上限：`TASK_MAX_CONCURRENCY`（默认 5）+ FIFO 信号量（`modules/tasks/semaphore.ts`），超额排队等槽；`GET /api/admin/tasks/runtime` 暴露 active/capacity/waiting
- [X] 每任务并发策略 `concurrencyPolicy`：`skip`（默认，沿用 Redis 锁丢弃重叠触发）/ `queue`（轮询等锁释放，上限=单次最大运行时长）
- [X] 重试退避 `retryBackoffS` + `retryBackoffStrategy`（fixed/exponential）；锁 TTL 按最大运行时长（含退避）计算；信号量等待后刷新锁 TTL
- [X] schema 新增 3 列（migration `0003_next_preak.sql`）+ shared 枚举/标签 + AdminTasks「高级控制」分区（并发策略/重试间隔/退避方式 Segmented）+ 顶栏并发槽位指示
- [X] 验证：`scripts/verify-tasks.ts` 17/17（信号量容量/FIFO、退避数学、skip/queue 端到端经共享 Redis 锁、重试退避计时）；浏览器：并发徽标 + 新建任务高级控制分区（三语言）

### M2/M7 扩展点 Widget（真实数据贡献）
- [X] 新表 `widget_contributions`（tenant×app×user×extPoint，data jsonb；migration `0004_young_blonde_phantom.sql`）+ 新 scope `widget.write` + 新错误码 `EXT_POINT_NOT_DECLARED` + shared `EXT_POINTS`/`WidgetData`/`WidgetContributionDTO`
- [X] Open API：`PUT/DELETE /api/v1/widgets/:ext`（守卫 `widget.write` + 校验应用已声明该扩展点 + 上限 12 项 + 写审计）；`GET /api/me/widgets`（按租户/用户读取 + join 应用元数据，仅 active 应用）
- [X] portal-sdk `contributeWidget(ext,data)`/`clearWidget(ext)`；`apps/sample-app` 加「贡献工作台小组件」按钮 + 加载时自动贡献
- [X] 前端 `ContributedWidget` 通用渲染（标题/摘要/键值项/链接）；Dashboard「应用贡献」区渲染 `dashboard.widget`；Settings 扩展点卡片渲染 `settings.section`（真实数据 + 声明占位并存）；seed 预置 rockie@sunrise 两条贡献（sample-todo→dashboard、analytics→settings）
- [X] 验证：`scripts/verify-widgets.ts` 13/13（贡献/读回/upsert/未声明拒绝/未知扩展点/缺 scope 403/缺 title/删除）；浏览器端到端：授权屏含 widget.write → 打开应用 → 点「贡献」→ Dashboard 待办 3→4 实时更新

### MX i18n 全量（zh-CN / en / zh-TW）
- [X] 机制升级：`lib/i18n.tsx` + `lib/locales/`（按命名空间分目录，`types`/`common`/`time`/`shell`/`dashboard`/`settings`/`auth`/`apps`/`inbox`/`profile`/`admin`/`appform`/`labels` 聚合）；`Locale` 三语言；`translate()` 供非 React 处（relTime 本地化）；3 选项语言选择器；`getLocale()` 模块级镜像
- [X] 全量抽取：22 个页面/组件硬编码中文 → `t()`；共享 `*_LABELS`（scope/provider/category/webhookEvent/grantType/extPoint/taskPolicy/backoff）经 `labels` 命名空间本地化；`MicroAppHost` locale 从 context 实时下发给微应用
- [X] 一致性：**489 键 × 3 语言键集完全对齐（无缺译/多余/跨命名空间冲突）**；服务端错误消息 + 应用贡献的数据内容（如通知标题、微应用名）按设计保持原文（数据非 chrome）
- [X] 验证：浏览器三语言切换走通 shell + 各页面（zh-CN/en/zh-TW 即时切换、收件箱相对时间本地化、扩展点数据保持原文）；全包 `tsc` 通过

---

## 二期（PLAN-2 · 平台化）— 已完成并验证

> 依据 `goal/PLAN-2.md`。schema 增量见 migration `0005_plan2_platform.sql`；新增验证脚本 `verify-console`/`verify-branding`/`verify-scheduler`。全包 `tsc` 通过；一期回归脚本全绿。

### Phase 0 · 平台租户 + 租户管理 native App（需求 1）
- [X] schema：`tenants.isPlatform`、`apps.{navItems,enabledNavItemIds,nativeRoute}`、`tasks.{createdByAppId,ownerUserId}`、新表 `tenant_domains`；shared `APP_TYPES += native`、`PLATFORM_TENANT_SLUG`、`SCHEDULER_MAX_PER_APP`
- [X] seed：专用平台租户（slug `platform`，`isPlatform=true`，排首位）+ Rockie 为其 admin + `native` 应用「租户管理」(`nativeRoute=/console`)；租户 `createdAt` 错时以保证默认租户确定性
- [X] `modules/platform/service.ts` 抽取（租户/管理员/域名纯函数 + `assertPlatformSession`/`isPlatformAdmin`/`resolveHostTenant`）；双通道复用：`x-platform-key`（Swagger，保留）+ 会话版 `/api/console/*`（`modules/platform/console.ts`），二者 fail-closed
- [X] `loadMe` 增 `isPlatformAdmin`（全局，与活跃租户无关）；native 应用在应用中心/启动器仅对平台 admin 可见，点击走 `nativeRoute` 不走 iframe
- [X] 前端 `/console` 全套：租户列表 + 新建 + 详情（名/套餐/状态/品牌编辑）+ 管理员授/撤 + 域名管理 + 删除守卫（平台租户不可删）；SideNav「平台」组 + 路由守卫（仅平台 admin）
- [X] 验证：`verify-console.ts` 26/26（会话门 fail-closed / 租户·管理员·域名 CRUD / x-platform-key 回归 / native 可见性 / 越权）；浏览器：Rockie 见「平台」组 → /console CRUD + 域名增删标验证 → 应用中心 native 卡片「打开」路由 /console

### Phase 1 · App 贡献顶级导航（需求 2）
- [X] `GET /api/apps/nav`（按可见 active 应用汇总启用的 `navItems` → `NavItemDTO[]`，复用可见性过滤 + native 网关）；`AppForm`「顶部导航」分区（增删条目 + 每条启用开关）；`SideNav`「应用导航」组渲染，点击深链 `/app/:appKey?r=<path>`（native→nativeRoute，link→外链）
- [X] 验证：浏览器：给 sample-todo 声明并启用「样例·详情」(/detail) → 左栏「应用导航」出现 → 点击深链打开应用并还原详情视图；seed 预置该导航项

### Phase 2 · 自定义域名 + 租户品牌（需求 4）
- [X] `resolveHostTenant(host)`（仅 `verified`；dev `?__host=` 覆盖受 `DEV_MOCK_OAUTH` 控）；公共 `GET /auth/branding`（免鉴权，返 `BrandingDTO|null`）；登录默认租户 hint 串过 OAuth flow state / 密码登录，`pickActiveTenant(userId, preferred)` 仅在用户确为成员时生效
- [X] 前端 `BrandingProvider`（注入 `--brand-blue`/`--brand-blue-hover` + Logo/名）；`Login` 品牌面板 + `TopBar` Logo 按域名品牌渲染，否则回落 XGENT；seed 预置 acme/star/qz(verified)+pending(unverified) 域名
- [X] 验证：`verify-branding.ts` 9/9（verified-only 解析 / 品牌端点 / 登录 hint：成员→切换、非成员/未验证→回落）；浏览器：`?__host=star.example.com` 登录页显示「星网在线学校」+ 橙色主色覆盖；李明经该域名登录默认进星网

### Phase 3 · 消息 + 计划任务公共服务（需求 5，self-scoped）
- [X] scope `scheduler.read/write`；Open API `/api/v1/scheduler/tasks`（create/list/patch/delete）——归属 `(createdByAppId, ownerUserId)`、每(应用,用户)上限 `SCHEDULER_MAX_PER_APP`、越权 `TASK_NOT_OWNED`；触发调 App Webhook（payload 带 `userId`）或 builtin `scheduler-notify` 给 owner 发通知
- [X] 消息服务：`POST /api/v1/notifications` 文档化为「平台消息服务 v1」（复用 `createNotification` 单一收口）；`AdminTasks` 纳入 App 任务（「来自 <App>」标签 + 只读，门户 mutate → `FORBIDDEN`）
- [X] `portal-sdk` `scheduler.create/list/update/cancel`；`apps/sample-app`「提醒」视图（每日/每分钟创建 + 列表 + 取消）
- [X] 验证：`verify-scheduler.ts` 18/18（scope 403 / CRUD / 同用户跨应用 + 跨用户隔离 / 上限 / 触发→通知 / 管理员只读 FORBIDDEN）；浏览器：sample-app 设「每分钟提醒」→ AdminTasks 见来源标签+只读 → cron 触发 → 收件箱实时收到提醒 → 取消停止

### Phase 4 · 硬化与收尾
- [X] 隔离复查：非平台 admin 拒访 `/api/console/*`（NOT_PLATFORM_TENANT）；未验证域名不泄露品牌/不改默认租户；App 任务跨应用/跨用户越权拦截；双通道 fail-closed
- [X] i18n 三语 parity：**567 键 × 3 语言键集完全对齐**（新增 `console` 命名空间 + shell/apps/appform/auth/admin 增量）；新 UI 复用 DESIGN 原子（focus-ring / 空错态 StateBlock / 暗色 / reduced-motion）
- [X] seed/demo 完善（平台租户 + native 应用 + 域名 + sample-todo 导航项 + scheduler scopes）；README 补 PLAN-2 启动/验证说明；本 CHECKLIST 更新
- [X] 验证：全包 `tsc` 通过；一期回归脚本（verify-tdt/isolation/platform/consent/m1/tasks/widgets/token-policy/exchange-consent/m45）+ 本期 console/branding/scheduler 全绿（13 脚本 195 检查）；主路径浏览器复跑（明暗主题）

## 已知小项
- [ ] Phase 6 device/ip 增强之前创建的旧会话在「安全」页显示「未知设备」（新登录正常采集；审计始终正确）
- [ ] 自定义域名品牌的 `--brand-blue` 覆盖在 dev 下仅在 URL 携带 `?__host=` 时持续（登录后跳 `/dashboard` 丢参→回落默认色）；生产读真实 Host 不受影响

---

# PLAN-3 · 应用市场 + 内容管理公共服务 + 待办 App

> 依据 `goal/PLAN-3.md`。脚本 `apps/api/scripts/verify-{content,market}.ts`，启动见根 `README.md`。

### Phase 0 · 契约与数据模型
- [X] 迁移 `0006_curved_exiles`：新表 `marketplace_listings`（全局目录，平台拥有）+ `content_entries`；`apps` += `marketplaceListingId`/`marketplaceVersion`/`contentTypes`
- [X] shared：scopes `content.read/write`；常量 `FIELD_TYPES`/`LISTING_STATUS`/`CONTENT_RECORD_SCOPES`/`CONTENT_LIST_MAX_LIMIT`；DTO `FieldDef`/`ContentType`/`ContentEntryDTO`/`MarketplaceListingDTO`/`InstalledListingDTO`；错误码 `LISTING_NOT_FOUND`/`LISTING_NOT_PUBLISHED`/`ALREADY_INSTALLED`/`CONTENT_TYPE_NOT_FOUND`/`CONTENT_ENTRY_NOT_FOUND`
- [X] env `TODO_APP_URL`；seed 上架「待办」listing（task 内容类型）；sample-todo 增 `note` 类型 + content scopes（供脚本验证）
- [X] 验证：全包 `tsc` 通过；`db:migrate` + `db:seed` 通过

### Phase 1 · 内容管理公共服务（headless CMS）
- [X] `modules/content/service.ts`：按 FieldDef 校验+强转、user/tenant 域隔离、Directus 式 `filter[field]`/`q`/`sort`/`limit`/`offset`
- [X] Open API `/api/v1/content-types` + `/api/v1/content/:type`（GET list / POST / GET:id / PATCH / DELETE，gated content.read/write）
- [X] `portal-sdk` `content.types/list/get/create/update/remove`
- [X] 验证：`verify-content.ts` 20/20（类型自省 / CRUD / 必填+enum+数字校验 / filter+q+sort / 未知类型 / scope 403 / 跨用户隔离 / 删空）

### Phase 2 · 应用市场后端
- [X] `modules/market/service.ts`：平台 listing CRUD + 发布；租户浏览（已上架 + 安装态）+ install（快照复制 + 来源指针 + micro 自动签发密钥 + 卸载后复装）+ update（同步开发者字段）
- [X] 平台通道 `/api/console/market/*`（assertPlatformSession）；租户通道 `/api/admin/market/*`（assertTenant+assertAdmin）；挂载于 `src/index.ts`
- [X] 验证：`verify-market.ts` 22/22（listing CRUD / 上架门槛 / 安装快照 / ALREADY_INSTALLED / 更新 / 卸载复装 / 非平台-admin 与非租户-admin 拦截）

### Phase 3 · 应用市场前端
- [X] 租户 `pages/admin/Marketplace.tsx`（卡片网格 + 分类/搜索 + 安装/已安装/有更新 + 安装前权限评审弹窗）；入口在 `AdminApps` 顶部「应用市场」
- [X] 平台 `pages/console/Market.tsx`（listing 列表 + 草稿/上架/下架 切换 + 删除）+ `MarketListingForm.tsx`（含内容类型 FieldDef[] 结构化编辑器）；SideNav 平台组「应用市场」
- [X] `lib/market-api.ts`；路由 `/admin/market`、`/console/market`
- [X] 验证（impeccable + Chrome）：平台建/上架 listing → 租户从市场安装（权限评审）→ 应用中心可见

### Phase 4 · 待办 App
- [X] `apps/todo-app`（Vite，:5175）：纯前端跑在 `sdk.content`('task') + 调度提醒 + 工作台小组件；快速添加 / 今日·即将·逾期·已完成 分组 / 勾选完成 / 行内编辑 / 删除 / 优先级 / 到期 / 空错载入态 / 跟随宿主主题 / reduced-motion
- [X] 根 `dev:all` + `dev:todo`；API CORS 放行 `todoAppUrl`；**修复 `apps/web/index.html` CSP `frame-src` 加 `:5175`（否则 iframe 被拦，握手超时）**
- [X] 验证（Chrome 端到端）：安装→授权→打开→增/改/勾完成/删（刷新后持久，证明内容服务存储）→ 工作台「待办概览」小组件计数实时正确 → 「每日提醒」开关 → 计划任务页见「来自 待办」只读任务

### Phase 5 · 收尾
- [X] i18n 三语 parity：**668 键 × 3 语言完全对齐**（新增 `market` 命名空间 + labels 增 content/scheduler scope 标签 + shell `nav.marketConsole`）
- [X] README 补 todo-app / 端口 / dev:all / verify-content / verify-market；本 CHECKLIST 更新
- [X] 验证：全包 `tsc` 通过；回归 verify-tdt(19)/widgets(13)/scheduler(18)/platform(27) + 本期 content(20)/market(22) 全绿

### 增量 · 待办 App → React + shadcn 日期选择器 + 三语 i18n
- [X] `apps/todo-app` 由纯 TS 重写为 **React + Tailwind + shadcn**（cn/Button/Popover/Calendar 原子 + tailwindcss-animate；teal 主色的 shadcn token 主题，跟随宿主明暗）
- [X] **shadcn 日期选择器**（Popover + react-day-picker v8 Calendar + Button 触发）替换原生 `<input type=date>`；月份/星期/周首日/触发日期格式经 date-fns locale 本地化
- [X] **App 级三语 i18n**（zh-CN/en/zh-TW）跟随宿主 `init.locale` + `onLocale` 实时切换；文案 + 相对到期 + 日历全部本地化
- [X] 验证（Chrome）：装→开→shadcn 日历选日期（六月 2026 / 一二三…周一首）→ 添加带到期任务；切换门户语言为 English → App 实时变英文（To-Do / Active·All·Done / Pick a date / Jun 20 / June 2026 周日首），无 console error

### 增量 · 内容 schema 改为架构层 code 注册表（owner + 表名命名空间）
- [X] 内容 schema 不再是应用市场可编辑配置 / 不随安装快照；改为 code 注册表 `apps/api/src/modules/content/schemas.ts`（`CONTENT_SCHEMAS`，每项标记 `owner` + 全局唯一 `table=owner__key`，如 `todo__task`/`sample__note`）
- [X] 迁移 `0007_content_registry`（手写 + journal）：删 `apps.contentTypes` / `marketplace_listings.contentTypes`；加 `apps.contentOwner`（安装时=listingKey）；`content_entries` 由 `(appId,typeKey)` 改为 `tableName`（+tenantId/ownerUserId，去 appId）
- [X] 内容服务 + openapi 路由按调用方 `contentOwner` → 注册表 → 表名 解析（应用只能访问自己 owner 声明的表 = 隔离边界）；`MarketplaceListingDTO.contentTypes` 改为注册表派生的**只读**字段；移除 console 上架表单的内容类型编辑器（改只读展示）
- [X] SDK / 待办 App 不变（仍按逻辑 key "task" 调用）；verify-content 20、verify-market 22 全绿
- [X] 验证（Chrome）：console 上架表单「内容类型」分区只读；待办 App 安装→授权→新增任务持久（经 todo__task）

---

## PLAN-4 · 文件管理 App（独立后端 · 对象存储 · 安全分享 · 协作空间）

> 应用市场上架的独立 App，自带后端 `@xgent/files-server`（:4100）+ 微应用 `@xgent/files-app`（:5176）+ 租户自带 S3/minio。门户唯一耦合点 = TDT introspection。全部 8 个 workspace `tsc` 绿；PLAN-1/2/3 回归全绿（无破坏）。

### Phase 0 · 契约与脚手架 + M2M 身份
- [X] shared：scopes `files.read|write|share` + 标签；errors（STORAGE_NOT_CONFIGURED/FILE_NOT_FOUND/SPACE_*/SHARE_*/INTROSPECT_FAILED/STORAGE_UNREACHABLE/SERVICE_ACCOUNT_NOT_FOUND/CAPABILITY_NOT_GRANTED）；constants（SERVICE_ACCOUNT_CAPABILITIES/SPACE_KINDS/SPACE_ROLES/SHARE_ROLES/FILES_DIGEST_ALGO/FILE_LIST_MAX_LIMIT）；dto（ServiceAccountDTO/StorageConfigDTO/SpaceDTO/SpaceMemberDTO/FileDTO/FileFingerprintDTO/PresignResult/ShareDTO/FileHookDTO）
- [X] 门户迁移 `0008_service_accounts`（手写 + journal）：`service_accounts`（租户无关）+ `service_account_secrets`
- [X] 平台控制台 SA 管理 `modules/platform/service-accounts.ts` + `/api/console/service-accounts/*`（列表分页/创建一次性密钥/改/轮换/吊销/删）；`validateServiceAccount()` 复用
- [X] introspection `POST /api/tokens/introspect`（SA Basic + capability `token.introspect`，dev 兜底 `x-resource-key`；返回 active/aud/tenant_id/user_id/scopes/role/exp）
- [X] client_credentials `POST /api/tokens/service` + `lib/tdt.ts` `signServiceTdt`（kind=service/sv 撤销计数）+ 网关接纳服务态令牌；用户绑定端点 `assertUserToken` 拒服务态
- [X] seed：`files-server` SA（密钥=FILES_RESOURCE_KEY）+ `files` 市场清单 + 预装晨光 + allowExchange/白名单 mistakes + 李明 mistakes→files 交换授权；env 新增 FILES_* / MINIO_*
- [X] 新 workspace `apps/files-server`（Elysia+Drizzle+@aws-sdk/client-s3，独立迁移链 `xgent-files`）+ `apps/files-app`（Vite+React+shadcn）；CSP frame-src += :5176；根 `dev:files`/`dev:all`/`db:files:*`
- [X] 验证：verify-introspect 15 / verify-service-accounts 19 / verify-client-credentials 9

### Phase 1 · 存储 + 直传 + 密纹
- [X] `storage_configs`（SK AES-256-GCM 落库）+ 每租户 S3 client（forcePathStyle/缓存）+ `GET/PUT/DELETE /files/storage`（requireAdmin，PUT 做 HeadBucket 探测 + 自动 PutBucketCors，SK 不回显）
- [X] `POST /files/presign`（建 pending 行 + presigned PUT）→ 客户端直传 → `POST /files/finalize`（落 digest=SHA-256/size/searchText，pending→active）；`GET /files/:id/download`（presigned GET）；`DELETE /files/:id`
- [X] 批量密纹 `POST /files/fingerprint`（逐文件 + missing 标记）；前后端同一 SHA-256 算法
- [X] bootstrap 脚本（配 minio + 团队空间 + 成员）；验证 verify-storage 11 / verify-upload 10 / verify-fingerprint 7（对真 minio）

### Phase 2 · 空间与成员（连续性）+ 列表/搜索
- [X] 个人空间懒创建 / 团队空间归租户；`space_members`（user/group，角色 owner/editor/viewer）解耦归属；`GET/POST/PATCH/DELETE /files/spaces` + `/spaces/:id/members`；移除成员只删 membership（空间+文件留存）
- [X] `GET /files`（q 全文 tsvector+ILIKE / tags 重叠 / type 前缀 / owner / sort / limit/offset 封顶）
- [X] 验证 verify-spaces 13（含连续性）/ verify-search 11

### Phase 3 · 分享（角色/过期/口令）+ 公开访问页
- [X] `file_shares`（颗粒度角色 + 过期 + argon2 口令）；`POST /files/:id/shares`、`/spaces/:id/shares`、列表、`DELETE /shares/:id` 撤销
- [X] 公开路由（无 TDT）`GET/POST /s/:token`：口令/过期/撤销校验 → presigned GET（downloader/editor）或元数据（viewer）；过期/错口令走 200 + 错误体
- [X] 验证 verify-share 16

### Phase 4 · 上传 Hook + 后处理
- [X] 内置「名称/标签入库索引」（finalize 始终执行）+ 可配 `file_hooks` webhook（best-effort + 短超时 + 可选 HMAC-SHA256）；`GET/POST/PATCH/DELETE /files/hooks`（requireAdmin）
- [X] 验证 verify-hooks 7（本地 Bun.serve 接收 + HMAC 校验 + 禁用不投递 + 内置索引命中）

### Phase 5 · SDK
- [X] 通用 host 代理原语 `sdk.callService(name, path, init)`（§3.3-B，零控制面 CORS）+ 门户 `MicroAppHost` 服务注册表/TDT 挂载/转发；`iframe allow="clipboard-write"`
- [X] `sdk.files.*`（digest/presign/finalize/upload/list/get/download/rename/remove/fingerprint/spaces/shares/storage/hooks，默认走 callService；字节走 direct presigned PUT）+ 本地 `digest()`
- [X] 全包 tsc 绿

### Phase 6 · files-app 前端（impeccable 设计 + Chrome 验证）
- [X] React+Tailwind+shadcn 微应用（紫色身份，跟随宿主明暗 + 三语 + reduced-motion + skeleton/空/错/越权态）：空间侧栏 / 文件网格·列表 / 多选 / 拖拽上传进度+重试 / 重命名 / 删除 / 高级搜索（结果高亮）/ 分享弹窗 / 成员管理（连续性提示）/ 管理员存储设置 + Hook 配置；独立公开访问页 `/s/:token`
- [X] 平台控制台 `pages/console/ServiceAccounts.tsx`（列表/创建·编辑弹窗·capabilities/scopes/一次性密钥/轮换·吊销·停用·删）+ SideNav 平台组「服务账号」+ 门户 i18n 增 files 三个 scope 标签 × 3 + `nav.serviceAccounts` ×3 + `serviceAccounts` 命名空间
- [X] i18n 三语 parity：门户 **717 键 × 3** + files-app **146 键 × 3** 完全对齐
- [X] 验证（Chrome 真浏览器，rockie@晨光）：服务账号控制台页 ✓ / files-app 握手+渲染（callService）✓ / 搜索高亮 ✓ / 文件菜单 ✓ / 分享弹窗建带口令链接（host 代理 POST）✓ / 公开页口令解锁→下载 ✓ / 切英文（宿主+App 实时本地化）✓ / 管理员存储设置（已连接、SK 不显）✓ / 团队空间成员弹窗（角色 + 连续性提示）✓。注：拖拽/原生文件选择器无法经自动化驱动（跨源 iframe + 原生 picker），字节直传路径由 verify-upload 对真 minio 验证

### Phase 7 · 下游 App 经 TDT 接入（#8）
- [X] 复用令牌交换：mistakes（声明 files.read）→ exchange 出 aud=files TDT（scope 交集，丢弃 files.write）→ 调 files-server 命中；直用 mistakes TDT 被 files-server 拒（aud≠files）；无交换授权→EXCHANGE_CONSENT_REQUIRED；错密钥→拒
- [X] 验证 verify-exchange-files 8（即最小下游调用样例）

### Phase 8 · 收尾
- [X] README 补 文件管理(PLAN-4) 章节 + 端口 + dev:files + files-server/Phase-0 验证脚本；本 CHECKLIST 更新
- [X] 全包 `tsc` 8/8 绿；回归 verify-tdt(19)/isolation(9)/content(20)/market(22)/scheduler(18)/widgets(13)/consent(9)/exchange-consent(14)/token-policy(8) 全绿；本期 portal 43 + files-server 83 = 126 检查全绿

### 需求 → 验证 自检
- [X] #1 租户级 Bucket 隔离 + 管理员配 S3 + 本地 minio → verify-storage + 浏览器存储设置
- [X] #2 一组 file key → 密纹 API → verify-fingerprint
- [X] #3 SDK 同密纹算法（SHA-256 单一真源）→ verify-fingerprint（本地复算一致）
- [X] #4 安全分享（角色/过期/口令）→ verify-share + 浏览器公开页
- [X] #5 协作空间（成员离开仍连续）→ verify-spaces + 浏览器成员弹窗
- [X] #6 全文搜索/标签/过滤 → verify-search + 浏览器搜索高亮
- [X] #7 上传后 Hook 后处理 → verify-hooks
- [X] #8 下游 App 经 TDT 访问 → introspection + 令牌交换 + 服务账号 + client_credentials + verify-exchange-files

### 增量 · 存储设置移至门户「应用管理」配置页（由租户管理员配置，非应用内）
- [X] 移除 files-app 内的「存储设置」入口/视图（侧栏 cog + StorageSettings 组件 + switch/tabs 原子，删除死代码）；普通文件浏览界面不再出现配置入口
- [X] 门户新增 `GET /api/admin/apps/:appKey/files-token`（assertAdmin，无 consent 门）签发 aud=appKey 的管理员 files-TDT（scope=intersect(files.*, app.scopes)）
- [X] 门户 `lib/files-config-api.ts`（管理员浏览器用该令牌**直连** files-server :4100 读写 storage/hooks；apps/api 不经手 S3 凭证）+ `pages/admin/FilesStorageSection.tsx`（对象存储表单 + 上传 Hook，复用 Field/Toggle/Badge 原子）
- [X] 集成进 `AppForm`：仅在编辑 + 应用声明 `files.write` 时显示「文件存储」分区；i18n 新增 `appForm.files.*` ×3（门户 741 键 × 3 parity）+ files-app `empty.goConfig` ×3（147 键 × 3）
- [X] files-app 未配置存储的空态改为引导「前往应用管理」（sdk.navigate("/admin/apps")）
- [X] 验证（Chrome，rockie@晨光）：files-app 侧栏无存储设置 ✓；应用管理 → 文件管理 → 文件存储 分区「已连接」+ 回显配置(SK 不显) ✓；「保存并测试连接」→「存储配置已保存，连接正常」(门户网页直连 files-server，HeadBucket 探测) ✓；全包 tsc 8/8 绿

---

# PLAN-5 · 研发项目管理 App（XGENT Track · 独立后端 · 多租户 · TDT 鉴权 · AI Agent 演示+预留）

> 依据 `goal/PLAN-5.md` + 需求 `goal/PMS.md`。把已存在的单租户、无鉴权 spms-app/spms-server 改造成上架应用市场的独立后端 App。门户侧 100% 复用 PLAN-4 机制（自省端点 / 服务账号 / callService 宿主代理 / 市场安装），零新门户逻辑。全包 `tsc` 10/10 绿；PLAN-1/2/3/4 回归不受影响。

### Phase 0 · 契约与门户接入（零新机制）
- [X] shared `scopes.ts` +`pms.read`/`pms.write`（+ zh-CN SCOPE_LABELS）；`errors.ts` +`ISSUE/SPRINT/PROJECT/TEAM/MEMBER_NOT_FOUND`/`INVALID_TRANSITION`
- [X] `apps/web/src/lib/locales/labels.ts` +`scope.pms.read`/`scope.pms.write` ×3（zh-CN/en/zh-TW）
- [X] `apps/api/src/lib/env.ts` +`spmsResourceKey`/`spmsSaClientId`；`apps/api/src/index.ts` CORS origin +`spmsAppUrl`（iframe 直连 widget/notify 需放行 :5177）
- [X] 门户 seed：`spms` 市场清单（icon kanban / color #5B5BD6 / extPoints dashboard.widget / navItems Issues·看板·迭代）+ `spms-server` 服务账号（capability token.introspect, secret=SPMS_RESOURCE_KEY）+ 装入 晨光 & 星网
- [X] `apps/web` MicroAppHost `SERVICE_REGISTRY` +`spms`→:4200；`index.html` CSP `frame-src` +:5177 / `connect-src` +:4200
- [X] 端口：spms-app 5173→**5177**、spms-server 3001→**4200**；包名 → `@xgent/spms-app` / `@xgent/spms-server`（workspace 接入 + typecheck 脚本）
- [X] **三处 DB 连接改读 `SPMS_DATABASE_URL`（独立库 xgent-spms）**：`db/index.ts`/`migrate.ts`/`drizzle.config.ts`（+ `lib/env.ts`）；`.env` 增量；根 `package.json` +`dev:spms`/`db:spms:migrate`/`spms:bootstrap`，`dev:all` 加 spms

### Phase 1 · spms-server 多租户 + 鉴权网关
- [X] schema 全表 +`tenantId` + 复合索引；issues 改 uuid 代理主键 + 展示 `key` 唯一 `(tenantId,key)`；teams/labels +`key`；members +`portalUserId`/`agentKey`；通知 +`forMemberId`；单迁移 `0000_*`（全新空库）→ migrate ✓
- [X] 移植 `lib/{env,crypto,redis,response,gate}.ts`（gate 复用 files-server 范式；Redis 缺省降级进程内 Map）
- [X] 路由全部改 `/api/v1/pms/*` 前缀；每 handler `gate(headers, pms.read|pms.write)` + 所有查询按 `claims.tenantId` 收口；编号事务内按 `(tenantId,teamId)` 递增
- [X] `app.ts` onError 优先映射 `HttpError`(→status)/`AppError`(→200)；CORS 放行 PORTAL_BASE_URL + SPMS_APP_URL
- [X] → verify-gate(6) / verify-tenant-isolation(8) / verify-issues(10)

### Phase 2 · 身份映射 + 成员目录同步
- [X] `lib/identity.ts`：人类懒同步（首触达经 `directory.read` 拉通讯录 upsert，**不回写短码覆盖真名**——浏览器验证发现并修复）；4 个 Agent 按租户懒建；`ensureAiLabel`
- [X] bootstrap `me` = 当前用户 member id；`scripts/bootstrap.ts <tenantId>` 按租户重建演示数据（teams/labels/projects+PLC/cycles/sprints+燃尽 snapshots/issues）
- [X] → verify-identity(5)

### Phase 3 · Scrum / 通知
- [X] sprints/backlog/burndown/velocity 全 gate + 租户收口（`:issueKey` 按展示 key 解析）；App 内 notifications 收件箱（tenant+user 过滤）+ 关键事件转门户铃铛 `notification.send`
- [X] → verify-scrum(9) / verify-notify(1)

### Phase 4 · AI Agent 演示 + 预留接口
- [X] 指派 Agent → `aiAssigned=true` + 附 AI 生成 标签 + App 收件箱 + `dispatchAgentTask()`（单一扩展点；演示写脚本化活动流 + setTimeout 链；契约即未来真 worker 订阅点，无 LLM/队列/webhook）
- [X] → verify-agent-assign(6)

### Phase 5 · spms-app 接 SDK
- [X] `main.tsx` 握手（iframe → createPortalClient/ready → **applyTheme 用 `dataset.theme` 而非 .dark**）+ standalone 兜底 + onTheme/onLocale 实时跟随；`lib/ctx.tsx` 注入 sdk/init/locale/t/api
- [X] `lib/api.ts` → `createApi(sdk)` 走 `callService("spms", "/api/v1/pms"+path)`（App 不持令牌）；store/AppData/issues/sprints 经 `useApi()`；当前用户取 `me`（去 'linzm' 硬编码）、默认团队取首个（去 'AGT' 硬编码）
- [X] 去外观设置（删 SettingsPanel + useSettings 主题/强调色写入）；去重宿主 chrome；StateBlock/Skeleton；index.css 加 skeleton + `prefers-reduced-motion` 降级
- [X] 「研发现状」工作台小组件贡献（我的进行中 / 当前 Sprint 剩余 / 活跃项目 / AI Agent 进行中，深链回 App，去抖重推）

### Phase 6 · 三语 i18n
- [X] `lib/i18n.ts`（makeT + normLocale）zh-CN/en/zh-TW 三语词典，**163 键 × 3 parity**；各组件文案 + 状态/优先级/PLC/Sprint/Agent 角色枚举 + 相对时间 抽取进词典；roadmap 月份 locale 感知

### Phase 7 · 设计/样式 + Chrome 真浏览器验证
- [X] tokens 已对齐（index.css）；AI 工作区卡片满 border + orange-50（无左色条、无卡里套卡，遵 DESIGN.md）；App Glyph 身份色 #5B5BD6
- [X] **Chrome 全链路（rockie@晨光）**：登录→打开 App（consent）→列表/看板双视图 ✓；IssueDetail 抽屉 + AI 工作区卡片 ✓；新建 Issue（live callService POST，编号 AGT-319，Rockie 署名）✓；行内指派 Forge（live PATCH → aiAssigned + AI 生成 标签 + 工作区卡片 + 活动流 + 铃铛 2→3）✓；Scrum 燃尽/速度 ✓；切 English 实时英文 ✓；切深色 App 跟随（data-theme，无半暗）✓；工作台「研发现状」小组件（数值对齐 + 深链）✓；切 星网 验证隔离（同 key AGT-319 内容不同、看不到晨光数据 + 当前用户按租户独立）✓；directory.read 同事入选择器 + Agent 角色本地化 ✓
- [X] 已知工具限制：跨源 iframe DOM 对 chrome 扩展不透明（评论框/popover 用坐标点击；写路径由 verify 脚本服务端佐证）

### Phase 8 · 收尾
- [X] README 补 PLAN-5 章节 + 端口 + dev:spms + spms-server 验证脚本；本 CHECKLIST 更新；演示数据按租户重建（晨光+星网）
- [X] 全包 `tsc` 10/10 绿；spms-server 45 检查（gate 6 / isolation 8 / identity 5 / issues 10 / scrum 9 / agent 6 / notify 1）全绿；spms-app i18n 163×3 parity

### 需求（PMS.md）→ 验证 自检
- [X] Issues 列表/看板双视图 + 行内编辑（状态/优先级/负责人）→ 浏览器 + verify-issues
- [X] Issue 详情抽屉 + 子任务 + 活动流 + AI 工作区卡片 → 浏览器
- [X] AI 指派（aiAssigned + AI 生成 标签 + dispatchAgentTask 演示）→ verify-agent-assign + 浏览器
- [X] 项目/周期/路线图 + PLC 阶梯 → 浏览器（roadmap lane 改按 index 映射，因 project id 变 uuid）
- [X] 敏捷 Scrum（待办/Sprint/燃尽/速度/故事点）→ verify-scrum + 浏览器
- [X] 收件箱（App 内动态）+ 门户铃铛 → verify-notify + 浏览器
- [X] 工作台看研发现状（dashboard.widget）→ 浏览器
- [X] 多租户（门户硬约束）→ verify-tenant-isolation + 浏览器（双租户）
- [X] 上架应用市场 + 样式 Portal 风格 + 三语 → seed/清单 + 浏览器换肤/英文 + i18n parity

### 增量 · 去重宿主功能（用户反馈）
- [X] spms-app 移除与平台重复的外壳：左上角 **XGENT Track Brand 块**（宿主有租户/工作区切换）、**收件箱** 导航 + InboxView（宿主左导航已有收件箱）、左下角 **Profile & Settings 页脚**（宿主用户菜单已有）；Sidebar 现以搜索框起始、AI Agents 列表收尾
- [X] **消息收发统一走平台消息服务**：移除 App 自有 `notifications` 表/路由/收件箱（schema 去表 + 重生成单迁移 0000 + bootstrap 去种子 + 删 routes/notifications.ts）；指派 Agent 等关键事件经门户 Open API `POST /api/v1/notifications`（`notification.send` → 平台 createNotification 单点：落库 + 实时推送 + 按偏好邮件 → 门户铃铛）发送；`lib/notify.ts` 保留为唯一消息出口
- [X] 前端清理：App.tsx 去 inbox view/notifications query/markAllRead/inboxCount；types.ts View 去 'inbox'；api.ts 去 notifications/markAllRead + Notification 类型；CommandPalette 去「打开收件箱」命令
- [X] 验证：全包 `tsc` 10/10 绿；spms verify 45（gate 6/isolation 8/identity 5/issues 10/scrum 9/agent 6/notify 1，notify 直接佐证「消息经平台服务发送、门户铃铛收到」）全绿；Chrome 确认侧栏三处已移除 + 门户收件箱徽标随 Agent 指派递增（平台消息服务收到）
