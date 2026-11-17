# 复用平台能力

## 先看平台有没有，别自建一套

外部服务最常见的浪费是把**平台已经有的公共能力**又实现一遍：自己存文件、自己发通知、
自己记审计、自己排定时任务。除了白干，还会让租户在两个地方看同一件事、让平台的治理面
（审计/用量/权限）出现盲区。**动手写之前先在这两张表里找。**

### A. 门户基座能力 —— 用你自己的 TDT 直接调 `/api/v1/*`

`PLATFORM_BASE_SCOPES` 里的 scope **任何 App 都能在清单里直接声明**，不需要跨应用授权、
不需要令牌交换。

| 你想做的 | scope | 端点 |
| --- | --- | --- |
| 当前用户是谁 | `userinfo.read` | `GET /api/v1/userinfo` |
| 选人 / 读通讯录 | `directory.read`（邮箱是 PII，要**另加** `directory.email.read`） | `GET /api/v1/directory/users` |
| **站内消息 / 通知** | `notification.send`（只发给自己）· `notification.send.others`（发给别的成员） | `POST /api/v1/notifications`；服务态用 `/api/v1/notifications/service` |
| 收件箱 | `inbox.read` | — |
| **统一审计** | `audit.write` | `POST /api/v1/audit` |
| **定时任务** | `scheduler.read` / `scheduler.write` | `/api/v1/scheduler/tasks` |
| 结构化内容 / CMS | `content.read` / `content.write` | `/api/v1/content/{type}` |
| **用量计量** | `usage.report`（只收 namespace 前缀 == 你 azp 的 metricKey） | `POST /api/v1/usage/report` |
| 席位配额 | `seats.read`（SERVICE_ONLY，经 manifest `privilegedServiceScopes` 申请、审批授予，见下节） | `POST /api/v1/seats/quota` |
| 币种与汇率 | `currency.read` | `GET /api/v1/currency` |
| 判权限（PID） | — | `POST /api/v1/acl/check` |
| 用户级设置 | `settings.read` / `settings.write` | `/api/v1/settings/me` |
| Dashboard 卡片 | `widget.write`（**用户态**，服务态令牌推不了） | `PUT /api/v1/dashboard/widgets/:key/data`；清单顶层 `dashboardWidgets` 声明（不是 `dashboard.widgets`）。有用户关心的状态就该暴露至少一个，前端用 SDK 推；完整契约见 [Widget 契约](open-api-and-extensions.md#3-dashboard-widget应用动态) |

### 终端用户身份也是门户能力，但使用专门服务身份

注册、凭据、会话、条款与隐私接口在 `/api/v1/end-users*` 等基座路径，管理入口为「用户中心」。
它们不走 `/svc/user-center`，也不走成员的交换流程；`endusers.*` 和 `privacy.requests.*`
是 SERVICE_ONLY scope，按需经 `privilegedServiceScopes` 申请。终端身份不是成员通讯录，
登录、当前限制和完整分工见 [终端用户参考](end-users.md)。

### B. App 型公共服务 —— 走 `/svc/<key>` + 令牌交换

这些是**独立 App**，不是基座端点：要在清单里声明 `exchangeTargets`，用交换来的
`aud=<目标>` 令牌调（机制与排错见 [跨应用交换](app-exchange.md)）。

| 服务 | listingKey | scope 命名空间 |
| --- | --- | --- |
| **文件存储**（用户空间 + 应用空间、直传、预览、缩略图） | `files` | `files.read` / `files.write` / `files.share` |
| 多模态解析（OCR / 文档结构化） | `omni-parser` | `omni_parser.read` / `omni_parser.parse` |
| 大模型网关（OpenAI 兼容 `/v1`） | `llm-gateway` | `llm_gateway.read` / `llm_gateway.write` |

⚠️ **连字符 listingKey ↔ 下划线 scope**：`omni-parser` 的 scope 是 `omni_parser.*`，
`llm-gateway` 的是 `llm_gateway.*`。两个命名空间，写混了是 `VALIDATION_FAILED`。

### 文件与对象存储：两条接入路径

| | Files 文件服务 | `objectStorage` 基础设施 |
| --- | --- | --- |
| 接入 | 调 `/svc/files`，使用获准的用户态或服务态令牌 | 清单声明 `requiredServices: [{"kind":"objectStorage","name":"main"}]`，后端用 S3 SDK 直连；`main` 换成目标环境已登记的服务名 |
| 桶与租户 | 托管模式使用租户池 `role=tenant`：多租户共用 bucket，默认按 `t/<tenantId>/` 前缀隔离 | 使用应用池 `role=app`：按 `listingKey` 每应用一个 bucket，该应用所有租户共用；租户安装时不另建桶 |
| 隔离与治理 | Files 按租户、空间与权限收口，提供文件配额、计量、预览和清理能力 | 平台不自动加租户前缀；App 自行校验租户、组织对象 key，并负责权限、计量和生命周期；不自动纳入 Files 治理 |
| 适用 | 用户/团队文件、需要平台文件治理的应用文件 | 自带 S3 存储引擎、索引快照或缓存等由 App 自管的底层对象 |

**Files 的「应用空间」是某个租户内的逻辑空间，不是应用专属 bucket。** 租户在 Files 中自配的
bucket 与凭据优先于托管池；存储位置由 Files 解析，调用 App 不取得租户池的 S3 凭据。

应用池当前注入的是**池级共用的数据面凭据**，不发 per-App / per-tenant 子凭据；每应用独立桶
不代表云侧已做跨 App 权限隔离。声明与五项 S3 环境变量见 `xgent-app-release` skill 的
「对象存储：应用池直供 S3 五键」。五键按整组供给，不需要管理员逐键取值。

应受平台文件治理的字节走 Files；App 自管底层对象可使用平台 `objectStorage`。以下用户态、
服务态与空间选择**仅适用于 Files**，按**这份字节归谁**选择：

| 这份字节归谁 | 姿态 | 落在哪 | 用户在文件管理里 |
| --- | --- | --- | --- |
| **归 App**：系统产物、用户删不得、随 App 生命周期消失 | 服务账号 `client_credentials`（无用户上下文） | 你自己的**应用空间**（不传 spaceId，懒创建） | 只有租户管理员看得到（要让作者看见用下一行） |
| **归 App，但作者要看得见**：文件由你掌控、用户删改不得，作者本人要在文件管理里找到它 | 服务账号 + `onBehalfOf=<成员的门户 user_id>` 署名 | 你自己的**应用空间** | 署名成员在「我的应用文件」里**只读**可见，其他成员看不到；读 [代成员保存](files-member-docs.md) |
| **归用户**：用户主动另存、他看得见删得掉、算他的空间 | 令牌交换换出的 `aud=files` **用户态**令牌 | 用户选的**个人/团队空间**（带 spaceId） | 就在他自己的空间里 |

**公开固定内容 URL 是 Files 的特权服务态能力**：申请 `privilegedServiceScopes` 中的
`files.links.manage`，审批后由本 App 服务账号发布自有应用文件；用户态令牌、目录授权和
`files.read` 都不能替代。签发通常先返回 creating，只有 ready 才有 URL；持链接即可匿名读，
不会自动过期但可吊销，也会随删源/应用卸载失效。需要此能力时读
[长期链接接入](files-stable-links.md)，不要放进门户基座能力或普通用户交换流程。

⚠️ **别因为「要写」就默认必须服务态**：用户态持有 `files.write` 完全合法 —— `exchangeTargets`
背书的是整个命名空间，平台不再按读写另行裁量。凡是「用户在你的界面里点一下、另存到我的
空间」这类功能**只能**用用户态：服务态写进去的是应用空间，不署名用户根本看不见自己刚存的
东西，署名了也只能只读查看，不在他自己的空间里、删不掉。用户态写也不丢归因 —— 交换出的令牌带 `azp=<你的 key>`，files 侧照样落
`viaApp` 溯源，按来源 App 聚合展示。

### 已经立过的红线（同一条道理，都是「平台有、别自建」）

- **审计**：不自建审计表/页，推 `POST /api/v1/audit`，租户在「设置 › 审计」一处看全。
- **通讯录**：不做「同步通讯录」到自己库，按需用选人组件挑。
- **配额/套餐**：见下一节。**计费**：平台级 Credit 服务，同样不自建。
- **租户级配置**：micro/service 均可用 `adminPages.tenant` 或 `tenantConfig` 声明 schema，micro 还可嵌入自定义页；都在租户 › 应用管理 › 配置，不再按 App 手写 AppForm Section。
- **AI 功能**：终端用户面**不出现模型选择器**，服务端按场景解析模型。

## 有配额需求？用平台的套餐管理，不许自建

**红线：外部服务不自建配额/套餐/计费。** 配额的唯一事实源是平台的套餐矩阵；对方在自己库里存
一份「本租户上限」，第一次调档就会与门户对不上，而且**没有任何东西会告诉你它们分叉了**。

- 两个模型**二选一**，都只有 **backed listing** 可声明：`seatBased`（计门户成员用不用得了这个
  App，门户在 TDT 签发时收口，App 侧零代码）/ `seatRoles`（App 自管账号或资源，门户只发数、
  App 自己在建号路径上收口）。
- 上面是 App 席位/业务资源模型；平台托管终端身份另用租户共享的 `portal.end_users`，
  由平台在身份创建/删除时占用/归还，归档仍占额。不要拿成员席位代替或再扣一份身份容量，
  见 [终端用户容量](end-users.md)。
- **role 由对方在 `app.manifest.json` 里声明**（不需要门户改代码）；**各套餐的数值进不了
  manifest**，只有平台管理员在控制台能写 —— 应用侧一条写路径都没有。
  这是有意的：数值是商务面，改它等于改可售卖档位。
- 对接面只有一个：`POST /api/v1/seats/quota`（服务态令牌 + `seats.read`）。**`available === null`
  ⇒ 该租户 unlimited，放行、不要收口** —— 把 null 当 0 是这个契约最容易踩的一处。
- ⚠️ `seats.read` 是 `SERVICE_ONLY_SCOPES`：对方 manifest 里出现 `serviceScopes: ["seats.read"]`
  **会被拒收**，评审时这是**当场打回的信号**，不是配置失误。正确路径是
  `privilegedServiceScopes: [{ "scope": "seats.read", "reason": "…" }]` —— 这是**申请**不是授予：
  必进「发布审核」，审批人逐条勾选确认后 SA 才拿到（reason 原文展示给审批人）。
- **配额 ≠ 用量**，别混：配额答「还能不能再建一个」，用量答「这段时间用了多少」，后者走
  `usageReporter` + `usage.report`。计费方向另有平台级 Credit 服务，同样不自建。

真要落一版（选模型、写 manifest、配数值、上报已用、存量租户、三态处理、判定某个数该不该进
套餐表）⇒ 读 [quota-and-seats.md](quota-and-seats.md)。

