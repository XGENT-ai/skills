# Open API 与扩展点（微应用视角）

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §8/§13/附录 B（2026-07）（门户仓文件，App 自己的 repo 里没有；本文件已自包含，不必去找）。冲突时以门户仓库原文为准。

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

Manifest `dashboard.widgets[]` 声明 → 用 `widget.write` 往**自己声明过的** Widget 推数据；写未声明的 key 得 `WIDGET_NOT_DECLARED`。

| 类型 | 用途 | `data` 形状 |
| --- | --- | --- |
| `metric` | 指标 | `{ value, label?, hint?, trend?: { direction:"up"\|"down"\|"flat", value } }` |
| `activity` | 动态流 | `{ title?, items: { title, body?, at?, link?, icon? }[] }` |
| `insight` | 图表 | `{ chart: { kind:"line"\|"bar"\|"area"\|"pie"\|"donut"\|"funnel", x?, series:[{name,data}] }, caption? }` |
| `topic_grid` / `topic_list` | 入口网格 / 清单 | `{ items: { title, subtitle?, icon?/meta?, link? }[] }` |

规则：数据按用户 upsert（同一 (app,user,widgetKey) 一条，重复 PUT 覆盖）；按类型校验形状；`link` 用 Portal 深链如 `/app/todo?r=/today`。

声明示例（listing 内）：

```ts
dashboard: { widgets: [
  { key: "todo-stats", source: "app", owner: "todo", type: "metric", title: "待办概览",
    defaultItemsPerRow: 4, minColPx: 220, minH: 1, defaultH: 1, dataMode: "app_push" },
]}
```

## 4. 通知（消息服务）

- 默认**自投**（发给当前用户）：落库 + 铃铛实时推送 + 按偏好发邮件，一个收口。
- 跨用户投递：传 `userId`（本租户活跃成员）需额外 `notification.send.others` scope；跨租户拒绝；受两道限频（每 actor、每 actor→收件人）；收件人看到「由 <发送者> 经 <App>」强出处。
- `link` 必须站内深链（白名单前缀 `/app/`、`/inbox`、`/settings`、`/dashboard`）；带协议或 `//host` 一律 `VALIDATION_FAILED`。`title/body/type` 有长度上限。

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
