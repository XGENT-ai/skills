# Open API 与扩展点（微应用视角）

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。

## 1. 统一响应信封

业务状态**不走 HTTP 状态码**：

```jsonc
{ "ok": true, "data": { ... } }                                  // 成功
{ "ok": false, "error": { "code": "INSUFFICIENT_SCOPE", "message": "...", "details": {} } }  // 业务失败，HTTP 仍 200
```

HTTP 非 200 只留传输/认证/路由层：400 参数结构错、401 TDT 缺失/无效/过期/撤销、403 scope 或权限不足、404 路由不存在（**数据不存在不是 404**，是 `200 + data:null`）、429 限流、5xx 真故障。SDK 已封装：非 ok 把 error 作为异常抛出，`try/catch` 即可。

## 2. 用户态端点一览（`Authorization: Bearer <TDT>`，基址 = init.apiBase + `/api/v1`）

| 端点 | scope | 说明 |
| --- | --- | --- |
| `GET /userinfo` | `userinfo.read` | 当前用户资料 + 本次 scopes |
| `GET /directory/users` | `directory.read`（+`directory.email.read` 才含邮箱） | 租户成员基础资料（默认不含邮箱，PII） |
| `POST /notifications` | `notification.send`（跨用户需 `notification.send.others`） | 见 §4 |
| `PUT /dashboard/widgets/:key/data` | `widget.write` | 推送 Widget 数据（须先声明，见 §3） |
| `DELETE /dashboard/widgets/:key/data` | `widget.write` | 清除 Widget 数据 |
| `POST /audit` | `audit.write` | 关键操作写平台统一审计（不自建审计） |
| `GET/PUT /settings/me` | `settings.read/write` | 当前用户偏好 |
| `GET /content-types`、`/content/:type` CRUD | `content.read/write` | 内容服务，见 §6 |
| `/scheduler/tasks` CRUD | `scheduler.read/write` | 计划任务，见 §5 |

限流：按 `(appKey, tenant)` 每分钟，额度随租户套餐（旗舰 600 / 专业 300 / 标准 120）。超额 `RATE_LIMITED`。

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

- 默认**自投**（发给当前用户）：落库 + 铃铛实时推送 + 按偏好发邮件，一个收口。
- 跨用户投递：传 `userId`（本租户活跃成员）需额外 `notification.send.others` scope；跨租户拒绝；受两道限频（每 actor、每 actor→收件人）；收件人看到「由 <发送者> 经 <App>」强出处。
- `link` 必须站内深链（白名单前缀 `/app/`、`/inbox`、`/settings`、`/dashboard`、`/console/`、`/admin/`）；带协议或 `//host` 一律 `VALIDATION_FAILED`。`title/body/type` 有长度上限。

## 5. 计划任务（Scheduler）

- 任务归属 `(应用, 用户)`，每对上限 10 个；`cron` + `tz`（默认 `Asia/Shanghai`）。
- 触发时：应用配了 `webhookUrl` → 调 webhook（payload 带 userId）；否则给任务主人发收件箱提醒。
- 取消务必走应用（SDK→DELETE）；直接删库行不会停掉内存里的调度器。

## 6. 内容服务（headless CMS）

应用声明内容类型（代码定义，按 listingKey 归属）+ `content.read/write` scope，即可 CRUD，**不用自建表/后端**。`recordScope:"user"` 记录按 (应用,用户) 隔离；`"tenant"` 租户内共享。字段类型 `text/textarea/number/boolean/date/datetime/enum/json`，支持 required/default/maxLength/min/max/options，写入按 schema 校验。内部表名 `${owner}__${key}`，应用间不碰撞。

## 7. 审计

关键操作（建/改/删配置、密钥等）POST `/api/v1/audit`：`{ event, object, detail, result, diff }`。best-effort，失败不阻断业务本身。需声明 `audit.write` scope。

## 8. 常见错误码（前端只认 code 不认文案）

| code | 场景 |
| --- | --- |
| `CONSENT_REQUIRED` | 宿主注入前用户未授权（或同意被子集 mint 收窄过） |
| `EXCHANGE_CONSENT_REQUIRED` / `EXCHANGE_NOT_ALLOWED` | 跨应用调用未获用户授权 / 缺布线 |
| `INSUFFICIENT_SCOPE` / `INSUFFICIENT_PERMISSION` | scope / ACL 不足（HTTP 403） |
| `WIDGET_NOT_DECLARED` / `EXT_POINT_NOT_DECLARED` | 未声明该 Widget / 扩展点 |
| `SCHEDULER_LIMIT` / `TASK_NOT_OWNED` | 计划任务超限 / 越权 |
| `VALIDATION_FAILED` | 参数校验失败（含通知 link 非站内深链、超长） |
| `RATE_LIMITED` | 限流（HTTP 429） |
| `APP_NOT_FOUND` / `APP_DISABLED` / `APP_NOT_EMBEDDABLE` | 应用不存在/停用/不可嵌入 |
| `CONTENT_TYPE_NOT_FOUND` / `CONTENT_ENTRY_NOT_FOUND` | 内容类型/记录不存在 |
