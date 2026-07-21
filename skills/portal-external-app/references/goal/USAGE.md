# USAGE · XGENT.ai Portal · 用量统计(计量底座,为未来收费做基础)

> 站在 `goal/PLAN-2.md`(平台层/Open API)、`goal/LLM-GATEWAY-1.md`(调用日志与成本估算)、`goal/FILE-STORAGE-2.md`(应用存储/空间维度)、`goal/BI-DASHBOARD.md`(widget 机制)之上。
> 本期交付**平台级用量计量(metering)底座**:统一的指标模型 + 上报契约 + 聚合存储 + 租户/平台两级查询与展示。目标是让「LLM token、文件存储容量、以及未来任何可衡量用量」都以同一套契约进入门户,**使下一期的计费(billing)只需在聚合数据之上加价格与账期,而不必回头改采集链路**。

---

## 0. 本期范围与决策

### 0.1 最重要边界:本期是「计量 metering」,不是「计费 billing」

- 本期回答的问题是:**「某租户在某天用了多少 X」**——采集、聚合、展示、导出,口径可审计。
- 不回答「应收多少钱」:无价格表、无账单、无支付、无配额硬限制。llm-gateway 已有的 `estimatedCost`(micro-USD)作为**指标值透传**,是参考口径而非应收金额。
- 判断标准:计费引擎下一期落地时,**只新增**(价格/账期/账单表 + 结算逻辑),不修改本期任何采集与聚合契约。

### 0.2 核心决策表

| # | 决策点 | 选择 | 含义 |
|---|--------|------|------|
| 1 | 聚合数据放哪 | **门户 portal DB 新模块 `modules/usage`**,各源 App 明细留在自己库里 | 门户只存日聚合(小体量、长保留);对账/下钻走源 App 现有明细(`gateway_call_logs`、`files` 表) |
| 2 | 采集方向 | **源 App 主动上报**(push)到门户 Open API,不是门户去拉 | 门户不需要知道每个 App 的库结构;新 App 接入 = 实现上报,零门户改动 |
| 3 | 上报身份 | App 的 **SA 服务令牌**(client_credentials,已有),新增 scope `usage.report` | 复用 PLAN-4 的 M2M 体系;`azp`(appKey)即上报者身份 |
| 4 | 指标两类 | **counter(流量,桶内累计)** 与 **gauge(水位,桶内快照)** | counter 计费时按账期求和(token 数);gauge 按峰值/均值(存储字节);聚合语义在指标定义里声明,查询层据此汇总 |
| 5 | 上报语义 | **幂等覆写**:上报「某租户某日某指标某维度的桶内绝对值」,门户 upsert 覆写 | 重试/重跑天然安全;源 App 可反复重推整天数据自愈,无需增量游标对账 |
| 6 | 粒度 | **日桶**(UTC 日切),维度 `dims` 白名单受控 | 计费以日为最小单位足够;小时级留在源 App 明细。UTC 日切全平台统一,展示层可换算文案但数值桶不切换时区 |
| 7 | 指标注册 | **代码内置目录**(portal 常量)本期;App manifest 声明式注册预留下一期 | 本期只有 3 个源(llm-gateway/files/portal 自身),内置目录最简;manifest 化等第 4 个源出现再做 |
| 8 | 查询面 | 租户管理员 `/admin/usage` + 平台 console `/console/usage` 两级 | 与现有 admin/console 分治一致;成员个人用量不在门户做(llm-gateway 自己的 `/usage` 页已覆盖) |

### 0.3 关键取舍(显式记录而非默默处理)

- **a. 聚合入门户、明细留源 App。** 门户 `usage_daily` 每租户每指标每天最多几行,十年也只是百万级;而 `gateway_call_logs` 三十天就滚动清理。计量的长期审计基础放门户,请求级追溯走源 App——两边职责不重叠。代价:门户上的数字与源 App 实时页可能有小时级滞后,页面上明示「统计截至」时间。
- **b. 幂等覆写而非增量累加。** 上报体是「桶内绝对值」(如 `2026-07-03 tenant-A llm-gateway.tokens.total model=gpt-4o → 183_204`),门户按唯一键 upsert。增量(+N)语义在重试/重启场景必然双计,对账成本远高于让源 App 直接重算整桶。代价:源 App 在补报窗口内必须能重算——llm-gateway 的 call_logs 保留 30 天,即最大补报深度(§3.2 水位线机制)。
- **c. llm-gateway 上报从 `gateway_call_logs` 重聚合,不复用 `gateway_usage_daily`。** 现有 `gateway_usage_daily` 的日切与门户 UTC 口径未必一致,且含 userId 维度(门户不需要)。reporter 直接对 call_logs 按 UTC `date_trunc` 聚合「今天+昨天」两桶整桶覆写,口径自洽、可自愈。`gateway_usage_daily` 保持 llm-gateway 内部使用,互不干扰。
- **d. metricKey 命名空间强制 = 上报者 appKey。** `usage.report` 只接受 `metricKey` 前缀等于令牌 `azp` 的指标(`llm-gateway.*` 只能由 llm-gateway 的 SA 上报);门户自身指标前缀 `portal.` 由进程内直写、不走 HTTP。防止 App 间伪造/覆写彼此用量——这是计费数据,完整性优先。
- **e. 各源 App 自带上报 worker,不经门户 tasks/webhook 调度。** llm-gateway 已有 `worker.ts` 惯例(startWorker),files-server 照搬。门户 tasks 是租户级业务调度器,把平台计量挂上去会引入「任务被租户改动/删除」的风险面。代价:每个源 App 多一个常驻定时器,可接受。
- **f. 维度(dims)白名单 + 规范化键做唯一索引。** `dims` 是 jsonb,但每个指标声明允许的维度键(如 `llm-gateway.tokens.*` 只允许 `model`),上报时校验并拒绝越界;唯一索引落在派生列 `dimsKey`(规范化 `k=v` 串,空维度为 `''`,手写迁移,沿用 COALESCE/默认值唯一索引的既有做法)。防高基数维度打爆聚合表。
- **g. 成本即指标,不加专列。** llm-gateway 的估算成本作为独立指标 `llm-gateway.cost.estimated`(unit=`microUSD`, counter)上报,而不是给 `usage_daily` 加 `cost` 列。表结构保持「一行一个数」,未来任何指标要带成本就再报一条成本指标,无 schema 变更。
- **h. 存储口径 = 「对象仍占用后端存储」。** `files` 表 `status='active'` 的 `size` 求和(含回收站/软删,若有——对象没物理删除就占容量),`pending`(未完成上传)不计;renditions(预览缓存)**要计**——为此给 `file_renditions` 补 `sizeBytes` 列(生成时写入;存量行快照任务里按对象 stat 回填,回填不到的按 0 计并 log 行数,不静默)。

### 0.4 明确不在本期

- **计费引擎全部**:价格表、订价方案(plan × 指标单价)、账期/账单生成、支付、发票。§7 只给蓝图。
- **配额与硬限制**:不做「超量拦截」。llm-gateway 已有的 per-token RPM / dailyTokenLimit 保持原样,与本期无关。
- **egress 流量计量**(文件下载字节):presign 直连对象存储,服务端看不到下载流量,需要供应商账单 API 或日志投递才能计量——单列一期。
- **成员级(per-user)用量页**:llm-gateway 自己的用量页已按 own/all scope 覆盖;门户本期只到租户粒度(`dims` 预留了未来加 `userId` 的空间,但不上报)。
- **小时级实时曲线 / 用量告警**:日桶够计费;实时监控走 llm-gateway `/metrics`(Prometheus)既有通道。
- **App manifest 声明式指标注册**(§0.2 决策 7):内置目录先行。
- **历史数据回灌**:上线日起算;llm-gateway 可选把 call_logs 现存 30 天回放一次(一次性脚本,做不做不影响验收)。

---

## 1. 拓扑与改造面(无新工作区、无新端口)

```
apps/api                      # 门户(改造)
  src/modules/usage/          #   新模块:指标目录 + 聚合表读写 + admin/console 路由
  src/modules/openapi/        #   += POST /api/v1/usage/report(挂 usage.report scope)
  src/db/migrations/00xx_usage_daily.sql   # 手写迁移
  scripts/verify-usage.ts     #   新 verify(纳入 verify:all)

apps/llm-gateway-server       # 源 App ①(改造)
  src/worker.ts               #   += usage reporter(每小时,今天+昨天整桶覆写)

apps/files-server             # 源 App ②(改造)
  src/db (migration)          #   += file_renditions.sizeBytes
  src/worker or index         #   += 每日存储快照 worker(gauge 上报)

apps/web                      # 前端(改造)
  pages/admin/AdminUsage.tsx  #   租户用量页(/admin/usage)
  pages/console/ConsoleUsage.tsx  # 平台用量页(/console/usage)
  SideNav / router / locales  #   导航 + 三语

packages/shared               # dto:上报体 / 查询响应类型
```

不新增服务、不新增数据库;门户聚合表落在 portal DB。

---

## 2. 数据模型(portal DB · `modules/usage`)

### 2.1 指标目录(代码常量,`packages/shared` 或 `modules/usage/metrics.ts`)

```ts
type MetricDef = {
  key: string;            // 'llm-gateway.tokens.total' —— 前缀 = 来源 appKey(§0.3d)
  kind: 'counter' | 'gauge';
  unit: 'tokens' | 'bytes' | 'count' | 'microUSD';
  dims: string[];         // 允许的维度键白名单,如 ['model'];越界拒收
  label: { zh: string; ... };  // 展示名,三语
};
```

本期内置指标:

| key | kind | unit | dims | 说明 |
|-----|------|------|------|------|
| `llm-gateway.requests` | counter | count | model | 调用次数(成功+失败) |
| `llm-gateway.tokens.input` | counter | tokens | model | 输入 token |
| `llm-gateway.tokens.output` | counter | tokens | model | 输出 token |
| `llm-gateway.tokens.total` | counter | tokens | model | 合计 token(计费主口径) |
| `llm-gateway.cost.estimated` | counter | microUSD | model | 估算成本透传(§0.3g) |
| `files.storage.bytes` | gauge | bytes | kind(`files`\|`renditions`) | 存储占用水位(§0.3h) |
| `files.storage.objects` | gauge | count | kind | 对象数水位 |
| `portal.seats.total` | gauge | count | — | 租户成员数(演示「其他可衡量用量」+ 未来按席位计费基础) |

### 2.2 聚合表 `usage_daily`

```ts
export const usageDaily = pgTable('usage_daily', {
  id: text('id').primaryKey(),
  tenantId: text('tenant_id').notNull(),        // 无 FK 到 apps:App 卸载后历史用量必须留存
  appKey: text('app_key').notNull(),            // 来源:'llm-gateway' | 'files' | 'portal'
  metricKey: text('metric_key').notNull(),
  day: date('day').notNull(),                   // UTC 日切(§0.2 决策 6)
  dims: jsonb('dims').notNull().default({}),
  dimsKey: text('dims_key').notNull().default(''), // 规范化 'model=gpt-4o';唯一索引用(§0.3f)
  value: bigint('value', { mode: 'number' }).notNull(),
  kind: text('kind').notNull(),                 // 'counter' | 'gauge'(冗余自指标定义,查询免 join)
  updatedAt: timestamp('updated_at', { withTimezone: true }).notNull(),
}, t => [
  uniqueIndex('usage_daily_uniq').on(t.tenantId, t.metricKey, t.day, t.dimsKey),
  index('usage_daily_tenant_day_idx').on(t.tenantId, t.day),
  index('usage_daily_metric_day_idx').on(t.metricKey, t.day),   // console 跨租户查询
]);
```

- **保留策略:不清理**。计费审计要求长保留,体量估算(50 租户 × 20 指标行/天 ≈ 36 万行/年)完全可控。
- 迁移手写(唯一索引含默认值列,沿用既有手写惯例)。
- 无小时表、无明细表、无上报日志表——幂等覆写(§0.3b)使对账游标和去重日志都不必要;上报失败由源 App 下一轮整桶重推自愈。

---

## 3. 采集链路

### 3.1 上报契约 `POST /api/v1/usage/report`(Open API)

- 鉴权:TDT 服务令牌(client_credentials),新增 scope **`usage.report`**,加入各源 App SA 的授权范围(seed 增量);门户按 `azp` 校验 metricKey 前缀(§0.3d),越权项整批拒绝并在响应体列明。
- 请求体(批量,单批 ≤ 500 条):

```jsonc
{ "records": [ {
    "tenantId": "…",                        // 源 App 服务全租户,逐行带租户
    "metricKey": "llm-gateway.tokens.total",
    "day": "2026-07-03",                    // UTC
    "dims": { "model": "gpt-4o" },
    "value": 183204
} ] }
```

- 语义:按 `(tenantId, metricKey, day, dimsKey)` **upsert 覆写**(§0.3b)。校验:指标必须在目录内、dims 键在白名单内、tenantId 必须存在且非平台租户则正常入库(未知租户整批拒绝——防脏数据,而不是静默丢弃)。
- 响应遵循项目 API 约定:传输层 200,业务结果在响应体(`{ upserted, rejected: [{index, reason}] }`);4xx 只用于未认证/scope 不足。

### 3.2 LLM token 用量(llm-gateway reporter)

- `worker.ts` 新增 `runUsageReport()`,**每小时**执行(与 runRetention 并列):
  1. 读持久化**水位线** `usageReportWatermark`(最后一个「已确认上报成功」的整天,UTC;存 `gateway_settings`,首启初始化为昨天)。
  2. 对 `gateway_call_logs` 按 UTC 聚合 **[watermark, 今天]** 区间内的全部日桶(watermark 当天也重推,捕捉日切附近的迟到写入):`group by tenantId, date_trunc('day', createdAt at UTC), model`,产出 5 个指标(requests / tokens.input / tokens.output / tokens.total / cost.estimated,cost 为 null 的行计 0 并照常求和)。
  3. 用自己的 SA client_credentials 换 `usage.report` 服务令牌,整桶批量上报;**整批成功后才把水位线推进到昨天**(今天的桶保持开放,下一轮继续覆写)。
- 正常运行时水位线 = 昨天,每轮实际就是「今天+昨天」两桶;reporter 宕机/门户不可达多天后,水位线不动,恢复即自动补报缺口内所有日桶——计费审计基础不允许「窗口滑过即永久丢失」。补报深度受 call_logs 保留 30 天约束:若缺口超出保留期,按现存数据上报并 log 缺失天数(不静默)。
- llm-gateway 现有 `/api/v1/usage/summary`、用量页、`gateway_usage_daily` **全部不动**(§0.3c)。

### 3.3 文件存储容量(files-server 快照 worker)

- 迁移:`file_renditions` 增加 `sizeBytes bigint`;rendition 转 `ready` 时写入实际大小。
- 新增每日快照任务(每天跑 2 次防单次失败,gauge 覆写幂等):
  1. 按租户 `SUM(files.size) WHERE status='active'` → `files.storage.bytes` dims `{kind:'files'}`;`COUNT(*)` → `files.storage.objects` dims `{kind:'files'}`。
  2. 按租户 `SUM(file_renditions.sizeBytes) WHERE status='ready'` → `files.storage.bytes` dims `{kind:'renditions'}`;`COUNT(*)` → `files.storage.objects` dims `{kind:'renditions'}`(目录声明了 kind 维度,两个 kind 都必须报齐,否则对象数与导出口径不完整);`sizeBytes IS NULL` 的存量行首轮尝试对象 stat 回填,回填失败按 0 计且 log 计数(§0.3h,不静默)。
  3. 上报当日桶(gauge 语义 = 当日最后快照值)。
- 未来按 App 细分(`spaces.ownerAppKey` 维度)已在模型上留好(dims 白名单加一个键即可),本期不报。

### 3.4 门户自身指标(进程内直写)

- `portal.seats.total`:门户每日任务(index.ts 内与 startScheduler 并列的轻量定时器)统计各租户成员数,**直接写 `usage_daily`**,不绕 HTTP(§0.3d)。
- 作用:验证「非 HTTP 源」路径 + 给未来按席位计费留口径。

### 3.5 未来指标清单(接入即照 §3.1 契约,零门户改动)

| 候选指标 | 来源 | kind | 备注 |
|---------|------|------|------|
| 组卷次数 / AI 解析次数 | exam / qbank | counter | AI 类动作是天然计费点 |
| 文档解析页数 | omni-parser | counter | 外部服务 App 首个案例 |
| 消息/通知发送量 | portal(inbox/notify) | counter | 进程内直写 |
| 调度任务执行次数 | portal(tasks) | counter | 进程内直写 |
| API 请求数 | portal(rate-limit 中间件顺带累计) | counter | 与 plan 限速对照 |
| 下载流量 egress | 对象存储供应商侧 | counter | 明确下一期(§0.4) |

---

## 4. 查询 API(读侧)

### 4.1 租户管理员(`/api/admin/usage/*`,assertAdmin)

- `GET /api/admin/usage/summary?from&to` — 期内各指标汇总(counter 求和 / gauge 取期末与峰值),含指标目录元数据(label/unit/kind),供概览卡片。
- `GET /api/admin/usage/series?metricKey&from&to&dim=model` — 按日序列,可选按单一维度分组(top N + 其他归并,N=8,归并数量在响应里注明——不静默截断)。
- `GET /api/admin/usage/export?from&to` — CSV 导出(全指标 × 日 × 维度平铺)。

### 4.2 平台 console(`/api/console/usage/*`,assertPlatformSession)

- `GET /api/console/usage/overview?from&to` — 全平台各指标总量 + 按日趋势。
- `GET /api/console/usage/tenants?metricKey&from&to&page&pageSize` — 租户排行,**沿用 `Page<T>` 服务端分页契约**。
- `GET /api/console/usage/tenants/:tenantId/summary?from&to` — 下钻单租户(复用 4.1 的聚合逻辑)。
- `GET /api/console/usage/export?from&to` — 跨租户 CSV。

聚合规则统一实现于 `modules/usage/service.ts`:counter → `SUM(value)`;gauge → **先把所选范围内的维度行按日求和成「当日总量」**(如 `files.storage.bytes` 的 `kind=files` + `kind=renditions` 两行相加;console 全平台口径再跨租户求和),**再**取期末值(最后一日总量)+ 峰值(日总量的 `MAX`)双口径同时返回——直接对行取 `MAX(value)` 会拿到最大的单一维度分量而非租户总水位峰值,属错误口径,verify 里加断言防回归。展示层选用、计费期可各取所需。

---

## 5. 前端(强制两步法:`impeccable` 设计 + Chrome 真浏览器验证)

### 5.1 `/admin/usage` 租户用量页

- 结构:时间范围选择(近 7/30/90 天 + 自定义)→ 概览卡片(本期 LLM token 合计、估算成本、存储水位、席位)→ Tab 分区:**LLM 用量**(按日曲线 + 按模型分组)、**存储**(水位曲线 + files/renditions 构成)、**全部指标**(表格 + CSV 导出)。
- 图表复用 BI-DASHBOARD insight chart 既有组件(注意既有 gotcha:图表容器需确定高度,不能 flex-1)。
- 页面明示「统计截至 <最后 updatedAt>,UTC 日切」,管理成员对小时级滞后有正确预期(§0.3a)。

### 5.2 `/console/usage` 平台用量页

- 全平台概览 + 指标切换的租户排行(分页表格,点击行下钻到单租户视图)+ 导出。
- 平台侧关注横向对比,信息架构与 console 既有列表页(租户/域名)保持一致。

### 5.3 Dashboard widget(可选增强,M4 内小项)

- 系统 widget `tenantUsage`(computed 模式,仅租户管理员可见):本月 token 合计 + 存储水位两枚数字,点击跳 `/admin/usage`。挂入 widgets 目录 6+1。

### 5.4 导航与 i18n

- SideNav:admin 区加「用量统计」、console 区加「平台用量」;三语文案与现有 locales 结构一致(新增 `usage.*` 键)。

---

## 6. Seed / verify / 里程碑

### 6.1 Seed 增量

- `db/seed.ts`:各源 App SA 的 scope 集合 += `usage.report`(llm-gateway、files)。
- 不造假用量数据;本地验证靠 verify 脚本上报真实/构造桶。

### 6.2 验证(每步可独立回归)

1. **M1 门户底座** → `apps/api/scripts/verify-usage.ts`:
   - 上报 → 查询往返一致;同桶重报覆写不叠加(幂等);
   - 跨命名空间上报被拒(files SA 报 `llm-gateway.*` → rejected);dims 越界被拒;未知租户整批拒;
   - gauge 聚合口径:构造多维度行(两个 kind),断言峰值/期末取的是「日总量」而非最大单分量(§4);
   - admin/console 两级查询 scope 正确(租户管理员看不到他租户;console 排行分页契约);CSV 内容抽查。
2. **M2 LLM 接入** → 扩展 verify:经 mock 渠道打若干次调用 → 手动触发 reporter → 门户 `usage_daily` 数值与 llm-gateway `/api/v1/usage/summary` 当日口径一致(token 三项 + cost)。
3. **M3 存储接入** → 上传已知大小文件(+生成一个 rendition)→ 触发快照 → gauge 等于预期字节;重跑快照值不变(gauge 幂等)。
4. **M4 前端** → Chrome extension 真浏览器走查:两个演示租户(晨光/星网)各自 `/admin/usage` 数字与 API 一致、console 排行可下钻、三语切换、暗色主题;若环境起不来则显式说明未浏览器验证。
- `verify-usage.ts` 纳入 `verify:all` 套件(reseed 后冒烟覆盖)。

### 6.3 里程碑

```
M1 门户 metering 底座(schema + 目录 + report/查询 API) → verify-usage 全绿
M2 llm-gateway reporter 接入                            → 双边口径对账一致
M3 files-server 快照接入(含 renditions.sizeBytes)      → 已知字节数断言通过
M4 前端两页 + widget + i18n                              → 浏览器走查通过
```

M1 是唯一的依赖根;M2/M3 彼此独立可并行;M4 依赖 M1(有数即可画,不必等 M2/M3 全齐)。

---

## 7. 通往计费的路径(下一期蓝图,本期不实现)

记录映射关系,证明本期契约足以承载计费:

- **账期聚合**:月账单 = 该月 UTC 日桶聚合;counter 求和(token 单价 × 量)、gauge 取时长加权均值或峰值(存储按 GB·月)——两口径 §4 查询层已同时给出。
- **价格与方案**:新增 `plan_prices(plan, metricKey, unitPrice, includedQuota)` 与租户覆盖表;`tenants.plan` 字段已存在。计费只读 `usage_daily`,不回改采集。
- **配额与限制**:软预警(用量页 + 通知)先行,硬限制需源 App 配合(llm-gateway 网关处最自然),届时以门户下发配额、源 App 执行的方向设计。
- **对账**:门户聚合 vs 源 App 明细(call_logs / files 表)双口径抽查,即 §6.2 验证在生产的常态化版本。
