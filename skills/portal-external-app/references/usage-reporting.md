# 用量上报（usageMetrics / usage.report）

App 要让租户管理员和平台在门户用量页看到「这段时间用了多少」时读取。
「还能不能再建一个」是配额，见 [配额与席位](quota-and-seats.md)；席位占用的 gauge 也按本文合同上报。

## 1. 心智模型

| kind | 语义 | 期内聚合 | 例子 |
| --- | --- | --- | --- |
| `counter` | 流量：桶内累计发生量 | 求和 | 调用次数、处理页数、token 数 |
| `gauge` | 水位：桶内快照值 | 先把同一天所有维度行加成当日总量，再取期末值与峰值 | 存储字节、对象数、已占席位 |

- 桶固定为一天，按 **UTC** 切日。唯一键是 `(tenantId, metricKey, day, dimsKey)`；`dimsKey` 由门户按维度键排序生成，`@xgent/shared` 导出同一实现 `usageDimsKey()`。
- 上报的是**桶内绝对值**，门户按唯一键覆写。同一桶重报任意次都安全，所以重试、补报、回放不需要去重日志；反过来，上报 `+N` 增量会在重试时重复计数。
- 方向只有推：App 后端定时上报，门户不会来读你的库。
- kind 选错，聚合口径就错（counter 被取峰值、gauge 被求和），声明前想清楚。

## 2. 在 manifest 声明指标

```json
{
  "serviceAccount": { "clientId": "your-app-server" },
  "usageReporter": true,
  "usageMetrics": [
    { "key": "your-app.jobs.completed", "kind": "counter", "unit": "count", "dims": ["type"],
      "label": { "zh-CN": "完成任务数", "en": "Jobs completed", "zh-TW": "完成任務數" } },
    { "key": "your-app.storage.bytes", "kind": "gauge", "unit": "bytes",
      "label": { "zh-CN": "存储用量" } }
  ]
}
```

- `usageReporter: true` 让本 App 服务账号获得 `usage.report`；manifest 注册的服务账号恒有 `client_credentials`、恒绑 `azp=<listingKey>`。
  声明 `usageReporter` 必须同时声明 `serviceAccount`。
- `key`：必须以 `<listingKey>.` 开头，形如 `^[a-z0-9-]+(\.[a-z0-9_-]+)+$`，≤ 120 字符，不重复，不能与平台内置指标同名；每个 App 最多 24 条。
  前缀就是上报者身份，不能报别的 App 的指标，`portal.*` 只由门户自己写入。
- `unit` 只能是 `tokens` / `bytes` / `count`。金额单位 `microAmount` 不开放声明，金额类指标只由平台内置。
- `label` 必须有 `"zh-CN"`，`en` / `zh-TW` 缺省回退 zh-CN（旧形状 `{zh,en,tw}` 仍接受）。
- `usageMetrics` 是治理档：随发布提案进「发布审核」，批准后才生效；批准前上报这些 key 会被判为「未注册」整批拒收。
- 每次发布以整份声明替换上一版。下一版漏写某个 key，之后上报它会被拒；想停用某指标就停止上报。

## 3. dims 白名单

- 声明的 `dims` 最多 8 个，名称形如 `^[a-zA-Z][a-zA-Z0-9_]{0,39}$`；`currency`、`baseCurrency` 是保留名，不能用。
- 上报时 dims 的键必须在该指标白名单内，值是 1–120 字符的字符串；省略 dims 等于空维度，是一个独立的桶。
- 不要把用户 id、请求 id、时间戳这类无界值当维度。成员级明细留在你自己的 App 页面，门户日桶只到「租户 × 少量业务维度」。
- 同一指标始终用同一组维度键上报。gauge 的当日总量是把当天所有维度行相加，混报一行空维度和几行带维度的数据会重复计数。

## 4. 上报合同

### 4.1 签服务态票（每租户一张）

```text
POST {PORTAL}/api/tokens/service
  Authorization: Basic base64(<clientId>:<secret>)
  { "grant_type": "client_credentials", "tenant_id": "<租户 id>", "scope": "usage.report" }
→ { ok: true, data: { access_token, token_type: "Bearer", expires_in, scope } }
```

| 签发结果 | 含义 | reporter 该做什么 |
| --- | --- | --- |
| `data.scope` 不含 `usage.report` | 服务账号还没获得授权（提案未批准等），签发不报错只返回更窄的 scope | 视为可重试失败，本轮不推进水位线 |
| `APP_NOT_INSTALLED` / `APP_DISABLED` | 你的 App 在该租户未安装或已停用 | 跳过该租户并记日志，继续处理其他租户 |
| `TENANT_NOT_ALLOWED` | 服务账号不允许为该租户签票 | 同上 |
| `CAPABILITY_NOT_GRANTED` | 服务账号没有 `client_credentials` | 修配置 |

只能为装了你 App 的租户签票。每条记录的 `tenantId` 必须等于签票时的 `tenant_id`，否则整批被拒：按租户签票、按租户上报。

### 4.2 `POST /api/v1/usage/report`

```text
POST {PORTAL}/api/v1/usage/report
  Authorization: Bearer <服务态 TDT>
{ "records": [
    { "tenantId": "<租户 id>", "metricKey": "your-app.jobs.completed", "day": "2026-11-23", "dims": { "type": "ocr" }, "value": 42 }
] }
→ { ok: true, data: { upserted: 1, rejected: [], defaulted: 0 } }
```

| 规则 | 不满足时 |
| --- | --- |
| 票带 `usage.report`，且是带 `azp` 的服务态票 | HTTP 403 `INSUFFICIENT_SCOPE` / 403 `FORBIDDEN` |
| `records` 为 1–500 条的数组 | HTTP 200 `VALIDATION_FAILED`；超过就拆批 |
| `metricKey` 已注册（平台内置或已批准的 `usageMetrics`），且前缀等于你的 `azp` | 记录进 `rejected` |
| `day` 是 `YYYY-MM-DD` 的 UTC 日期，不能晚于 UTC 今天 | 同上 |
| `value` 是非负安全整数 | 同上 |
| `dims` 符合 §3 | 同上 |
| `tenantId` 存在、等于令牌所属租户，且不是平台租户 | 同上 |

- **整批全有或全无**：任一条不合法，整批不写入，`upserted: 0`，`rejected` 逐条给出 `{ index, reason }`。同批合法记录也没有写入。
  这种拒收是确定性的，原样重试不会变好；修数据后整桶重推。
- 成功判定：`ok === true && data.upserted === records.length`。`rejected` 非空说明数据或声明有缺陷，完整记录 reason，不要静默吞掉。
- `defaulted` 只对平台旧指标的兼容路径有意义，外部 App 恒为 0。
- 限流：与其他 `/api/v1/*` 调用共用每分钟「令牌 aud × 租户」的桶，额度按租户套餐：免费 120 / 专业 300 / 尊享 600 / 无限 1200。
  超限返回 HTTP 200 `RATE_LIMITED`（信封 `ok:false`）。每租户每轮只发少量请求时足够。

## 5. 两种 reporter 模式

两者都依赖 §1 的覆写语义；按指标性质选，不要混用到同一指标上。

### 5.1 counter + 水位线（流量，明细在你自己的事件表）

```text
① 读持久化水位线（最后一个「整批全部成功」的 UTC 日；首次运行取昨天）
② 按 UTC 日重聚合明细 [水位线, 今天]，每个 (tenant, day, dims) 一行
③ 按租户签票，每批 ≤ 500 条上报
④ 本轮全部成功才把水位线推进到昨天；今天的桶保持开放，下一轮继续覆写
```

- 任一可重试失败（门户不可达、5xx、scope 尚未授予）都不推进水位线，恢复后第一轮自动补齐缺口内所有日桶。
- 水位线那一天也重推，用来收进日切附近迟到写入的明细。稳态下每轮就是「昨天 + 今天」两个桶。
- 补报深度受你的明细保留期限制，超出时按现存数据上报并记录缺失天数。
- 聚合按 UTC 切日（如 `created_at AT TIME ZONE 'UTC'`），`day` 在 SQL 里就输出成 `YYYY-MM-DD` 字符串（如 `to_char(…, 'YYYY-MM-DD')`）。直接取 `date` 类型时，部分驱动会把它解析成日期对象，序列化后过不了 `day` 校验。

### 5.2 gauge + 快照（水位，任意时刻可直接算出当前总量）

```text
① 按租户聚合当前总量（每个声明的维度各一行）
② 按租户签票，上报「今天」这个桶
③ 一天跑几次：快照可覆写，多跑能抵御单次失败，不需要水位线
```

- 声明过的维度要报齐，某维度为空也报 0。
- 租户的数据清零后，聚合查询不再出这一行，但仍要为它报 0；否则用量页会把上次的快照当作当前值。
  快照租户集合取「聚合结果 ∪ 所有启用过你 App 的租户」。
- 峰值与期末值由门户计算，你只报快照。

建议保留一个手动触发一次上报的脚本，用于联调、补报和回放。

## 6. 数字在哪里看

- 租户管理员：门户 `/admin/usage`（管理 › 用量统计），可按维度看日曲线、导出 CSV。
- 平台管理员：`/console/usage`（平台 › 平台用量），按指标看租户排行与单租户下钻。
- 数字的滞后由你的 reporter 周期决定，不需要做成实时。

## 7. 接入清单

1. manifest 写 `serviceAccount`、`usageReporter: true`、`usageMetrics`，随发布提案提交并等审批通过。
2. 签票后确认返回 `scope` 含 `usage.report`。
3. 按 §5 实现 reporter，从你自己的明细或当前状态整桶重算。
4. 联调：造几条数据 → 手动触发上报 → 以租户管理员打开 `/admin/usage` 看到指标 → 再触发一次，数字不变。

## 8. 常见症状

| 症状 | 多半是 |
| --- | --- |
| 签票成功但上报 403 `INSUFFICIENT_SCOPE` | 签出的 scope 为空：`usageReporter` 未声明或提案未批准 |
| 上报 403 `FORBIDDEN` | 用了用户态票，或服务账号没绑定所属 App |
| 签票 `APP_NOT_INSTALLED` | 该租户没装你的 App（本地常见于新建的测试租户） |
| `upserted: 0`，reason 写「未注册指标」 | `usageMetrics` 漏声明该 key，或声明所在提案还在待审 |
| reason 写「不属于上报者的命名空间」 | `metricKey` 前缀不是你的 listingKey |
| reason 写「day 必须是 UTC 日期」 | `day` 不是 `YYYY-MM-DD` 字符串（见 §5.1 驱动日期问题） |
| 数字差一天 | 聚合没按 UTC 切日 |
| gauge 偏大 | 同一指标混报了空维度和带维度的行 |
| 门户数字与明细求和对不上 | 先对齐口径（UTC 日切、计入哪些状态），再手动触发 reporter 覆写 |
