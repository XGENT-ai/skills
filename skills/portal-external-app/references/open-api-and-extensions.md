# Open API 与扩展点（微应用视角）

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。
> 完整端点、字段与示例见门户在线 API 文档 `/admin/api-docs`：租户管理员默认可见，也可经角色授权「API 文档」页给其他成员；只读、不能在页内发请求，可下载 OpenAPI JSON。

## 1. 统一响应信封

业务状态**不走 HTTP 状态码**：

```jsonc
{ "ok": true, "data": { ... } }                                  // 成功
{ "ok": false, "error": { "code": "INSUFFICIENT_SCOPE", "message": "...", "details": {} } }  // 业务失败，HTTP 仍 200
```

HTTP 非 200 只留传输/认证/路由层：400 参数结构错、401 TDT 缺失/无效/过期/撤销、403 scope 或权限不足、404 路由不存在（**数据不存在不是 404**，是 `200 + data:null`）、5xx 真故障。`/api/v1` 的限流也是 200 信封（见 §2）；429 只出现在凭证类端点（如访问密钥自描述，带 `details.retryAfter`）和匿名公开端点。SDK 已封装：非 ok 把 error 作为异常抛出，`try/catch` 即可。

## 2. 端点一览（基址 = init.apiBase + `/api/v1`）

**用户态**（`Authorization: Bearer <TDT>`，前端经 SDK 调）：

| 端点 | scope | 说明 |
| --- | --- | --- |
| `GET /userinfo` | `userinfo.read` | 当前用户资料 + 本次 scopes |
| `GET /directory/users` | `directory.read`（+`directory.email.read` 才含邮箱） | 租户成员基础资料（默认不含邮箱，PII） |
| `POST /notification-events` | `notification.send`（发给他人需 `notification.send.others`） | 声明式成员通知（`sdk.notifyEvent`），见 §4 |
| `POST /notifications` | `notification.send`（发给他人需 `notification.send.others`） | 自由文本通知，见 §4 |
| `PUT /dashboard/widgets/:key/data` | `widget.write` | 推送 Widget 数据（须先声明，见 §3） |
| `DELETE /dashboard/widgets/:key/data` | `widget.write` | 清除 Widget 数据 |
| `POST /audit` | `audit.write` | 关键操作写平台统一审计（不自建审计），服务态也可调，见 §7 |
| `GET/PUT /settings/me` | `settings.read/write` | 当前用户偏好 |
| `GET /content-types`、`/content/:type` CRUD | `content.read/write` | 内容服务，见 §6 |
| `/scheduler/tasks` CRUD | `scheduler.read/write` | 用户的计划任务，见 §5 |

**服务态**（`client_credentials` 换来的服务态令牌，服务账号须绑定所属 App，即令牌带 `azp`；用户态令牌调这些端点回 403）：

| 端点 | scope | 说明 |
| --- | --- | --- |
| `POST /notification-events/service`、`GET /notification-events/service/:eventId` | `notification.send` | 声明式成员通知及其投递状态，见 §4 |
| `POST /notifications/service` | `notification.send` | 自由文本通知给指定成员或按角色扇出，见 §4 |
| `/scheduler/service/tasks` CRUD | `scheduler.read/write` | App 后端自己的计划任务，见 §5 |
| `/content-admin/schemas`、`/content-admin/entries/:type`、`DELETE /content-admin/tables/:table` | `content.write`（列记录 `content.read`） | 注册内容类型、无用户写入，见 §6 |
| `POST /usage/report` | `usage.report` | 用量上报，见 [用量上报](usage-reporting.md) |
| `POST /seats/quota`、`GET /entitlements`、`/quota/*`、`/grant-entitlements`、`/grant-quota/*` | `seats.read`、`entitlements.read/consume`（服务专用，须申请） | 见 [配额与席位](quota-and-seats.md) |

限流：每个请求按 (令牌 `aud`, 租户) 计入每分钟固定窗口。用户态 `aud` 是本租户安装的 appKey，服务态 `aud` 是服务账号 clientId，所以同一 App 的前端与后端各算各的桶。额度随租户套餐：免费版 120 / 专业版 300 / 尊享版 600 / 无限版 1200（次/分钟；无限版也限流）。超额回 **HTTP 200** + `RATE_LIMITED`（`details: { limit, windowSeconds }`），按错误码退避重试，别只看状态码。通知等端点另有自己的桶（见 §4）。

## 3. Dashboard Widget（应用动态）

有用户关心的状态（待办数、最近动态、待处理项）就该暴露至少一个 Widget——用户在工作台「自定义」里才挑得到你。三步：**声明 → 申请 scope → 前端推数据**。

**① 声明**：`app.manifest.json` 的**顶层** `dashboardWidgets` 数组（自动通过档，发版即生效）。

```jsonc
"dashboardWidgets": [
  { "key": "my-stats", "source": "app", "owner": "<你的 listingKey>", "type": "metric",
    "title": "我的待办", "dataMode": "app_push" }
]
```

- `owner` **必须等于 listingKey**；`type` 只能是 `metric` / `activity` / `insight` / `topic_grid` / `topic_list`（**下划线**，`topic-list` 提交即拒）。
- `title` / `subtitle` / `description` 是**纯字符串**（单语；给多语对象不生效，`xgent-app-release` 的 preflight 会报错）。
- ⚠️ 旧文档写过嵌套的 `dashboard: { widgets: [...] }`——门户从来只读顶层 `dashboardWidgets`，那种写法现在提交即拒。

**② scope**：`scopes` 里加 `widget.write`（平台基础 scope）。

**③ 推数据**：用**用户态令牌**——前端 `sdk.dashboard.putWidgetData(key, data, { staleAfterSec? })` / `clearWidgetData(key)`，即 `PUT|DELETE /api/v1/dashboard/widgets/:key/data`，body `{ data, staleAfterSec? }`。

- 数据**按用户**存：同一 (租户, 用户, App, widgetKey) 一行，重复 PUT 覆盖。某个用户没被推过，他的卡片就是空态——通常在 App 打开 / 数据变化时推一次。
- `staleAfterSec` 到期后不再展示（上限 7 天）；不传 = 永不过期。
- 写未声明的 key ⇒ `WIDGET_NOT_DECLARED`。

| 类型 | 用途 | `data` 形状（服务端按类型裁剪） |
| --- | --- | --- |
| `metric` | 指标 | `{ value, label?, hint?, trend?: { direction:"up"\|"down"\|"flat", value } }`；value ≤40 字、label ≤60、hint ≤120；**`link` 会被丢弃**（App 推的指标卡不可点） |
| `activity` | 动态流 | `{ title?, items: { title, body?, at?, link?, icon? }[] }`；最多 20 条 |
| `insight` | 图表 | `{ chart: { kind:"line"\|"bar"\|"area"\|"pie"\|"donut"\|"funnel", x?, series:[{name,data}] }, caption? }`；≤8 条序列、每条 ≤60 点 |
| `topic_grid` / `topic_list` | 入口网格 / 清单 | `{ items: { title, subtitle?, icon?/meta?, link? }[] }`；grid ≤24、list ≤30 条 |

`link` 必须是站内深链（`/app/`、`/inbox`、`/settings`、`/dashboard`、`/console/`、`/admin/` 开头），如 `/app/<key>?r=/today`；不合格的链接被静默去掉。

**用户怎么加上**：工作台 →「自定义」→ 新增一个**与 Widget 类型相同**的分区 → 在选择器里挑你的 Widget → 保存。只有该用户看得见（已启用、有可见性、有席位）的 App 的 Widget 才会出现在选择器里。

**限制（别踩）**：

- **服务态令牌推不了**（端点只收用户态）：纯后端 / 无前端的 App 目前没法给 Widget 供数。
- `dataMode: "app_query"` 只对平台自带的 pagebuilder 生效，别的 App 声明它卡片永远为空——用 `app_push`。
- `titleI18n`、`refreshPolicy`、每个 Widget 的尺寸字段（`defaultItemsPerRow` / `minColPx` / `minH` / `defaultH` …）目前**不生效**，尺寸按类型缺省值走。
- 从 manifest 里把 Widget **全部删掉**不会清掉已安装租户里的声明（空数组被当作「没声明」）。

## 4. 通知（消息服务）

两条入口，**成员通知优先用声明式事件**；自由文本入口不受事件声明、模板与租户策略治理。

**声明式事件（推荐）**：先在清单 `notificationEvents` 登记事件（`key`、`schemaVersion`、`payloadSchema`、`templateVariables`、渠道、`templates` 等；字段形状同 [终端用户生命周期](end-user-lifecycle.md) §8 的示例，成员事件省略 `allowedAudienceKinds` 即为 `member`）。发送时只带事实（`payload`），文案由平台按事件模板和收件人语言渲染，渠道由平台、租户策略和用户偏好决定。

- 前端：`sdk.notifyEvent({ producerEventId, eventType, payload, schemaVersion?, occurredAt?, audience?, link? })`，即用户态 `POST /notification-events`。缺省发给当前用户；`audience: { kind: "users", userIds }` 指向他人需 `notification.send.others`，且都须是本租户活跃成员。
- 后端：服务态 `POST /notification-events/service`，`audience` 必填：`{ kind: "users", userIds }` 或 `{ kind: "role", role: "admin" | "member" }`；角色人数超过平台扇出上限会被拒。与 `/notifications/service` 共用每 (App, 租户) 60 次/分钟的桶。
- `producerEventId`（1–128 位，字母/数字/`._:-`）是幂等键，重试沿用同一个：同键同内容回原 `eventId` 且 `duplicate: true`，同键异内容 `IDEMPOTENCY_CONFLICT`。事件未声明或已弃用 `NOTIFICATION_EVENT_UNKNOWN`；`payload` 不符声明或序列化后超过 64KB `NOTIFICATION_SCHEMA_INVALID`。
- 返回 `{ eventId, status: "accepted", duplicate }`。**accepted 只表示已受理**，不是已送达，更不是已读。
- 查投递状态：服务态 `GET /notification-events/service/:eventId`。只返回本租户、本 App 当前安装发出的成员事件，否则 `data: null`；字段 `eventStatus`、`routingComplete`、`routedRecipients`、`deliveryCounts: [{ channel, status, count }]`、`readEvidence: "not-provided"`、`checkedAt`，不含收件人明细，也不提供已读证据。轮询有独立的 60 次/分钟桶，不占发送额度；`eventId` 不是 UUID 回 400。

**自由文本（`/notifications`）**：落库 + 铃铛实时推送 + 按偏好发邮件。

- 用户态 `POST /notifications`，body `{ title, body?, type?, link?, userId?, titleI18n?, bodyI18n? }`；`sdk.notify` 的类型只含 `title/body/type/link`，发给自己。默认发给当前用户；`userId` 指向他人需 `notification.send.others`，须是本租户活跃成员（跨租户拒绝），受两道限频（每发送者 20 次/分钟、每「发送者→收件人」5 次/分钟），收件人看到「由 <发送者> 经 <App>」出处。
- 服务态 `POST /notifications/service`：`userId`（本租户活跃成员）与 `role`（`admin` / `member`）**必须且只能给一个**；无需 `.others`；role 扇出最多发 50 人，超出部分不发也不报错（成员多时用声明式事件）；返回 `{ delivered }`。
- 长度：`title` ≤200、`body` ≤4000、`type` ≤40，每个语言变体分别校验。
- 多语：`titleI18n` / `bodyI18n` 必须是对象，键只能是 `zh-CN` / `en` / `zh-TW`，值为字符串，**必须含 `zh-CN`**；按收件人的界面语言选取（未设置按 `zh-CN`），回退顺序 收件人语言 → `zh-CN` → `title` / `body`（`title` 仍必填）。role 扇出逐人渲染。

两条入口的 `link` 规则相同：只允许站内路径，`?`/`#` 之前的部分以 `/app/`、`/console/`、`/admin/` 开头，或是 `/inbox`、`/settings`、`/dashboard` 本身及其子路径；带协议、以 `//` 开头、含反斜杠或控制字符一律 `VALIDATION_FAILED`。

## 5. 计划任务（Scheduler）

**用户态 `/scheduler/tasks`**（`sdk.scheduler.*`）：

- 任务归属 `(应用, 用户)`，每对上限 10 个；`cron` + `tz`（默认 `Asia/Shanghai`）。
- 触发时：应用配了 `webhookUrl` → 调 webhook（payload 带 userId）；否则给任务主人发收件箱提醒。
- 取消务必走应用（SDK→DELETE）；直接删库行不会停掉内存里的调度器。

**服务态 `/scheduler/service/tasks`**：App 后端自己的任务，归属 `(应用, 租户)`，没有用户，到点回调 App 自己的服务后端。

- 前提：清单声明的服务账号持 `scheduler.read` / `scheduler.write`，且持 `scheduler.callback`（服务专用 scope，经 `privilegedServiceScopes` 申请）；缺回调身份建任务即 `SCHEDULER_NO_CALLBACK_IDENTITY`，App 没有可回调的服务后端即 `SCHEDULER_NO_CALLBACK_BASE`。
- 创建 body `{ name, cron | runAt, tz?, callbackPath, params?, timeoutS? }`：`cron` 与 `runAt`（RFC3339，一次性）二选一，`runAt` 已过去就立即触发一次；`callbackPath` 以 `/` 开头、不含 `..` 与协议，主机由平台按 App 服务地址解析；`timeoutS` 为 1–600，默认 30。
- 上限：每 (应用, 租户) 默认 1000 个 active/paused 任务（平台可调），超限 `SCHEDULER_LIMIT` 带 `used / limit`；终态任务不占额度。
- PATCH 可改 `name` / `tz` / `params` / `status`（`active` / `paused`），周期任务改 `cron`、一次性任务改 `runAt`，两者不能互转；终态任务、已入队的一次性任务改时间回 `TASK_STATE_INVALID`，请新建。
- 回调：`POST <服务地址><callbackPath>`，`Authorization: Bearer` 一张只含 `scheduler.callback`、钉死租户、短寿命的服务票，请求头 `x-xgent-delivery-id` / `x-xgent-task-id` / `x-xgent-attempt`，body `{ task, trigger: "schedule", at, params, taskId, deliveryId, attempt, scheduledFor }`。接收方照常自省，并**断言 `kind=service`、`azp` 等于自己的 listingKey、含 `scheduler.callback`**：每个接回调的 App 都持这个 scope，只验 scope 会放行别的 App 的回调。
- 至少一次投递：只有 2xx 算成功（3xx 不跟随，按失败算）；失败指数退避，最多 30 次、总跨度一天以上，用 `x-xgent-delivery-id` 去重。列表与详情带 `lastFiredAt`、`lastDeliveryStatus`、`attempts`。App 在该租户停用或卸载后不再回调。

## 6. 内容服务（headless CMS）

存结构化记录**不用自建表/后端**。内容类型归属 App 的 listingKey，存储表名 `<listingKey>__<key>`，应用间不碰撞，App 只能碰自己命名空间里的类型。

- **注册类型（服务态，按租户）**：App 后端用服务态令牌（`content.write`）`POST /content-admin/schemas`，body `{ key, name, recordScope, fields }`；归属取令牌 `azp`，不读 body。类型按租户注册，新租户要各注册一次；同一 (租户, key) 重复调用覆盖名称与字段定义，可在开通或发版时重放。`DELETE /content-admin/schemas/:key` 只注销类型、**不删记录**，清数据另调 `DELETE /content-admin/tables/<listingKey>__<key>`。
- 形状：`key` 小写字母开头、小写字母/数字/`-`，≤63；`name` ≤120；`fields` 1–100 个，字段 `key` 字母开头、字母/数字/`_`，≤64，`label` ≤120。
- 字段类型 `text/textarea/number/boolean/date/datetime/enum/multienum/json`；`enum` / `multienum` 必须带 1–100 个非空字符串 `options`（`multienum` 的值是字符串数组）；支持 `required/default/maxLength/min/max`，写入按 schema 校验。
- `recordScope: "user"` 记录按 (应用, 用户) 隔离；`"tenant"` 租户内共享。
- **读写记录**：前端用户态 `GET /content-types`、`/content/:type` CRUD（`content.read/write`）。后端无用户写入（如匿名表单提交）用服务态 `POST /content-admin/entries/:type`，列表 `GET /content-admin/entries/:type` 返回 `{ items, total, page, pageSize }`；服务态写入的记录不属于任何用户，`user` 型类型下用户态读不到。
- 类型未注册 `CONTENT_TYPE_NOT_FOUND`；注销不存在的类型 `CONTENT_SCHEMA_NOT_FOUND`。

## 7. 审计

关键操作（建/改/删配置、密钥等）POST `/api/v1/audit`，用户态或服务态令牌均可（`audit.write`）：`{ event, object?, detail?, result?, diff?, idempotencyKey? }`。

- `event` 必填；超长截断：`event` 120、`object` 200、`detail` 1000、`idempotencyKey` 200 字符。`result` 只认 `"failure"`，其余记为成功。
- 带 `idempotencyKey` 时按 (租户, key) 去重：重投不产生第二条，仍回 `{ recorded: true }`。App 用本地 outbox 重投审计时带上它。
- best-effort，失败不阻断业务本身。

## 8. 常见错误码（前端只认 code 不认文案）

| code | 场景 |
| --- | --- |
| `CONSENT_REQUIRED` | 宿主注入前用户未授权（或同意被子集 mint 收窄过） |
| `EXCHANGE_CONSENT_REQUIRED` / `EXCHANGE_NOT_ALLOWED` | 跨应用调用未获用户授权 / 缺布线 |
| `INSUFFICIENT_SCOPE` / `INSUFFICIENT_PERMISSION` | scope / ACL 不足（HTTP 403） |
| `WIDGET_NOT_DECLARED` | 未声明该 Widget |
| `NOTIFICATION_EVENT_UNKNOWN` / `NOTIFICATION_SCHEMA_INVALID` / `IDEMPOTENCY_CONFLICT` | 通知事件未声明 / 载荷不符声明 / 同一 `producerEventId` 内容不同 |
| `SCHEDULER_LIMIT` / `TASK_NOT_OWNED` / `TASK_STATE_INVALID` | 计划任务超限 / 越权 / 终态或已入队不可改 |
| `SCHEDULER_NO_CALLBACK_IDENTITY` / `SCHEDULER_NO_CALLBACK_BASE` | 服务态任务缺回调服务账号 / 缺可回调的服务后端 |
| `VALIDATION_FAILED` | 参数校验失败（含通知 link 非站内深链、超长） |
| `RATE_LIMITED` | 限流（`/api/v1` 为 HTTP 200 信封，见 §2） |
| `APP_NOT_FOUND` / `APP_DISABLED` | 应用不存在 / 停用 |
| `CONTENT_TYPE_NOT_FOUND` / `CONTENT_ENTRY_NOT_FOUND` | 内容类型 / 记录不存在 |

完整错误码表见 `@xgent/shared` 导出的 `ERROR_CODES` / `ERROR_MESSAGES`。

用户在「已授权应用」里撤销某个 App：该用户对它的同意和它发出的跨应用交换同意一起删除，它签给该用户的 TDT 立即失效（已换出的目标令牌按短有效期自然过期）；之后再打开会重新走 `CONSENT_REQUIRED`，调交换目标会 `EXCHANGE_CONSENT_REQUIRED`。配了 webhook 的 App 收到 `consent.revoked`，只能当提示。
