# BI Dashboard · 可配置首页与多 Tab 分析看板 · Widget Catalog · App 动态面板

> 本计划基于当前 Portal Dashboard（`apps/web/src/pages/Dashboard.tsx`）与既有应用扩展机制（`extPoints=dashboard.widget`、`/api/me/widgets`、`widget_contributions`）演进。目标不是做一个固定首页，而是把 Dashboard 升级为**租户/用户可配置的 BI 工作台**：系统内置 Widget + 应用 Manifest 暴露的 Widget 统一进入 Widget Catalog，用户在页面/Tab/区域/布局里选择展示。
>
> 核心变化：现有“应用贡献”改名并重塑为**应用动态**。应用不再只是往一个固定 Dashboard 插槽塞卡片，而是通过 Manifest 声明可展示的 Widget，并通过 Open API 推送动态数据；Dashboard 自定义配置中可勾选哪些应用的动态面板参与展示。

---

## 0. 目标与边界

### 0.1 要交付什么

1. **可配置 BI Dashboard**
   - 至少有一个不可删除的“首页”。
   - 用户/租户可新增多个 Tab。
   - 每个页面（首页和 Tab）可配置标题、副标题、区域顺序、布局方式、区域内 Widget。
2. **区域化布局**
   - 页面从上到下由多个“区域（Section）”组成。
   - 每个区域先确定 `widgetType`，再从该类型所有可展示 Widget 中选择要放入的 Widget。
   - 区域支持不同布局方式：水平平铺、瀑布流、Bento Grid、Masonry。
3. **Widget Catalog**
   - 所有可配置面板统称 `Widget`。
   - Widget 类型：
     - `metric` 指标面板：类似当前“可用应用 / 未读通知 / 当前角色”。
     - `activity` 动态面板：原“应用贡献”，来自应用动态数据。
     - `insight` 洞察面板：图表、趋势、分布、漏斗等 BI 图表。
     - `topic_grid` 专题网格：类似当前“常用应用”。
     - `topic_list` 专题列表：类似当前“最近通知”。
   - 系统内置一批 Widget；应用通过 Manifest 暴露自己的 Widget。
4. **应用动态替换应用贡献**
   - 文案、模型、配置入口都从“contributed widget / 应用贡献”过渡为“app activity / 应用动态”。
   - Dashboard 自定义配置中可按 App 勾选哪些应用动态面板可显示。
5. **安全和治理**
   - Widget 可见性受应用安装状态、scope、ACL 和 Manifest 声明约束。
   - 应用只能写入自己声明的动态 Widget 数据。
   - 配置变更有审计、可恢复默认。

### 0.2 不在本期

- 完整自助 BI 数据建模、任意 SQL/指标编辑器、跨数据源 JOIN。
- 拖拽式自由画布。第一期用“区域 + 预设布局 + 每行列数（maxItemsPerRow）”，降低实现和响应式风险。
- 多人实时协同编辑 Dashboard 配置。
- 复杂权限：例如某个 Widget 给某个角色独立授权。第一期继承 App 可见性、ACL 与内置数据权限。

---

## 1. 产品模型

### 1.1 术语

| 术语 | 含义 |
| --- | --- |
| Dashboard | 当前租户/用户的工作台容器。 |
| Page / Tab | Dashboard 下的一个页面。首页是固定 Page；其它是用户新增 Tab。 |
| Section | 页面内从上到下排列的布局区域。每个 Section 绑定一种 Widget 类型。 |
| Widget | 可放入 Section 的面板定义。定义来自系统内置或应用 Manifest。 |
| Widget Instance | 某个 Widget 在某个 Section 中的一次放置，包含 span、高度、局部配置。 |
| Widget Catalog | 当前用户可选择的 Widget 集合。 |
| App Activity | 应用动态面板。应用声明 Activity Widget，并持续推送当前用户相关的数据。 |

### 1.2 目标配置体验

配置流程按“先结构，后内容”：

1. 选择页面：默认首页，或新增/切换 Tab。
2. 编辑页面标题、副标题。
3. 从上到下添加 Section：
   - 选择 Widget 类型：指标 / 动态 / 洞察 / 专题网格 / 专题列表。
   - 选择布局方式：水平平铺 / 瀑布流 / Bento Grid / Masonry。
   - 选择响应式预设。
4. 在当前 Section 内选择 Widget：
   - 系统按 Section 的 `widgetType` 过滤 Widget Catalog。
   - 可继续按来源过滤：系统内置 / 某个 App / 已收藏 / 最近使用。
   - 对 `activity` 类型可按应用勾选“显示哪些应用的动态面板”。
5. 调整 Widget 顺序、高度、是否折叠标题、局部展示参数；区域级调整每行列数（maxItemsPerRow）。
6. 保存；支持恢复默认。

> 关键原则：配置页面时不让用户在一个区域里混杂所有 Widget 类型。先选区域类型，再选该类型 Widget，页面结构更稳定，后续响应式也更可控。

---

## 2. 布局系统

### 2.1 布局基准：内容区真实宽度，不用列栅格

不采用固定列栅格，也不做 per-widget 自由 span。每个 Section 只声明 `maxItemsPerRow`（2–6，桌面满宽时一行最多放几个 Widget），渲染器再按**内容区真实宽度**折算到当前实际列数。

关键：折算依据是 Dashboard 内容容器的实测宽度（用 `ResizeObserver` 监听容器，而非 `window.innerWidth`）。原因是左侧导航的展开/收起会改变 Dashboard 可用宽度，却不改变视口宽度——同一块屏幕，导航收起时能多排一列，展开时该减一列。只看视口宽度会算错。

### 2.2 每行列数怎么算

每个 Widget 类型带一个 `minColPx`（单个 Widget 舒适显示所需的最小像素宽）。某 Section 在某一刻的有效列数：

```
effectiveCols = clamp(floor(contentWidth / type.minColPx), 1, section.maxItemsPerRow)
```

- 内容区越窄（导航展开，或窗口变小），`floor(contentWidth / minColPx)` 越小，自动减列，到极窄时落到 1 列（stack）。
- 永不超过 `maxItemsPerRow`，也不少于 1。
- 同一 Section 内 Widget 同类型、等宽填充，因此不需要 per-widget 宽度。要让某区域整行只放一个，把该 Section 的 `maxItemsPerRow` 设为 1。

### 2.3 响应式预设

`responsivePreset` 不再表达“几列栅格”，而是表达 Section 在窄内容区下**收敛到单列的激进程度**（作用于 §2.2 折算之上的额外下限）：

| 预设 | 行为 | 适用 |
| --- | --- | --- |
| `dense` | 尽量保持多列，仅在极窄内容区（约 <480px）才落单列 | 指标、紧凑网格 |
| `balanced` | 默认；内容区约 <640px 折单列 | 通用 |
| `editorial` | 偏少列大卡，内容区约 <820px 即落单列/双列 | 洞察、Bento |
| `single-focus` | 恒为单列纵向，无视 maxItemsPerRow | 大图表、专题报告 |

（阈值以内容区宽度为准，不是视口；具体数值实现时再校准。）

### 2.4 类型默认尺寸

每个 Widget 类型给一组默认值，替代旧的 60 列 span 三元组：`{ defaultItemsPerRow, minColPx, defaultH, minH, maxH? }`。新建某类型 Section 时，`defaultItemsPerRow` 作为该 Section `maxItemsPerRow` 的初值；`H` 以行高单位计。

| 类型 | 默认每行 | `minColPx` | 默认高度 | 说明 |
| --- | --- | --- | --- | --- |
| `metric` | 4 | ~220 | 1 | 一行常见 2–6 个；窄内容区自动减列。 |
| `activity` | 3 | ~320 | 3 | 应用动态；可一行 3 个，也可设 `maxItemsPerRow=1` 拉满。 |
| `insight` | 2 | ~420 | 4 | 图表需要较大宽度，默认半屏。 |
| `topic_grid` | 2 | ~360 | 3 | 常用应用、快捷入口等。 |
| `topic_list` | 2 | ~360 | 3 | 最近通知、待办、审批列表等。 |

约束规则：
- 类型默认值决定 Section 初始 `maxItemsPerRow` 和每个 Widget 的默认高度。
- Widget Instance 只能覆盖高度 `spanH`（须在类型 `minH`/`maxH` 内）；宽度不再 per-widget 覆盖，由 Section 的 `maxItemsPerRow` + 内容区宽度统一决定。
- `maxItemsPerRow` 取 2..6（`single-focus` 或整行拉满的区域为 1）。横向最多 6 个 Widget，超出自动换行。

### 2.5 Section 布局方式

| 布局方式 | 说明 | 适用类型 |
| --- | --- | --- |
| `tiles` 水平平铺 | 按顺序从左到右填充，满行换行。最稳定。 | 全部，默认 |
| `waterfall` 瀑布流 | 固定列数，按列高度放置。 | `activity` / `topic_list` |
| `bento` Bento Grid | 预设大中小组合，重要 Widget 可突出。 | `metric` / `insight` / `topic_grid` |
| `masonry` Masonry | 类 Pinterest，高度自由，按最短列填充。 | `activity` / `topic_list` / 轻量 `insight` |

第一期建议先实现 `tiles` + `bento`，`waterfall/masonry` 可先保留配置枚举但隐藏或标 beta，避免一次性把布局复杂度拉满。

---

## 3. Widget Catalog 设计

### 3.1 Widget Definition

Widget Definition 是可配置面板的声明。系统内置 Widget 和 App 暴露 Widget 都转换成同一结构。

```ts
export type DashboardWidgetType = "metric" | "activity" | "insight" | "topic_grid" | "topic_list";
export type DashboardWidgetSource = "system" | "app";
export type DashboardLayoutMode = "tiles" | "waterfall" | "bento" | "masonry";

export interface DashboardWidgetDef {
  key: string;                 // owner 内稳定 key，如 "unread-notifications"
  source: DashboardWidgetSource;
  owner: "portal" | string;    // system 使用 "portal"，应用使用 listingKey/contentOwner
  type: DashboardWidgetType;
  title: string;
  subtitle?: string;
  icon?: string;
  color?: string;
  description?: string;
  defaultItemsPerRow: number;  // 新建该类型 Section 时 maxItemsPerRow 的初值
  minColPx: number;            // 单个 Widget 最小舒适像素宽，按内容区宽度折算列数用
  minH: number;
  maxH?: number;
  defaultH: number;
  allowedLayouts?: DashboardLayoutMode[];
  dataMode: "computed" | "app_push" | "app_query";
  refreshPolicy?: {
    intervalSec?: number;
    staleAfterSec?: number;
  };
  configSchema?: DashboardWidgetConfigField[];
}
```

### 3.2 系统内置 Widget

先内置这些 Widget，替换当前硬编码 Dashboard：

| key | 类型 | 当前来源 |
| --- | --- | --- |
| `portal.available-apps` | `metric` | 当前“可用应用” |
| `portal.unread-notifications` | `metric` | 当前“未读通知” |
| `portal.current-role` | `metric` | 当前“当前角色” |
| `portal.favorite-apps` | `topic_grid` | 当前“常用应用” |
| `portal.recent-notifications` | `topic_list` | 当前“最近通知” |
| `portal.app-activity-feed` | `activity` | 聚合应用动态，可按 App 过滤 |

后续可加：
- `portal.recent-apps`
- `portal.pending-tasks`
- `portal.tenant-usage`
- `portal.audit-summary`（管理员可见）

### 3.3 App Widget

应用通过 Manifest 声明 Widget，而不是只声明 `extPoints`。

```ts
interface AppManifestDashboard {
  widgets?: DashboardWidgetDef[];
}
```

示例：

```ts
{
  listingKey: "spms",
  scopes: ["pms.read", "notification.send", "widget.write"],
  dashboard: {
    widgets: [
      {
        key: "assigned-issues",
        source: "app",
        owner: "spms",
        type: "metric",
        title: "指派给我的 Issue",
        defaultItemsPerRow: 4,
        minColPx: 220,
        minH: 1,
        defaultH: 1,
        dataMode: "app_query",
      },
      {
        key: "activity",
        source: "app",
        owner: "spms",
        type: "activity",
        title: "研发项目动态",
        defaultItemsPerRow: 3,
        minColPx: 320,
        minH: 2,
        defaultH: 3,
        dataMode: "app_push",
      },
      {
        key: "burndown",
        source: "app",
        owner: "spms",
        type: "insight",
        title: "迭代燃尽图",
        defaultItemsPerRow: 2,
        minColPx: 420,
        minH: 3,
        defaultH: 4,
        dataMode: "app_query",
      },
    ],
  },
}
```

### 3.4 App Activity：替换“应用贡献”

> 产品尚未对外发布、无生产数据，因此直接替换旧机制，不保留兼容层、不背 legacy 包袱。

现有机制（将被删除）：
- App 声明 `extPoints: ["dashboard.widget"]`
- App 调 `PUT /api/v1/widgets/dashboard.widget`
- Portal 在固定“应用贡献”区域渲染，数据存 `widget_contributions`

新机制：
- App Manifest 声明 `dashboard.widgets[]`，其中 `type:"activity"` 的就是可展示动态面板。
- App 调新接口写入 Widget 数据：`PUT /api/v1/dashboard/widgets/:widgetKey/data`。
- Dashboard 配置里，用户在 `activity` Section 中选择：
  - 显示全部已安装 App 动态
  - 只显示勾选的 App
  - 只显示指定 Widget

替换动作（一次性，无兼容映射）：
- 删除 `dashboard.widget` 这个 ext point 的写路径与 `widget_contributions` 表（`extPoints` 机制本身保留给 `settings.section` 等其它点）。
- 把现有用到它的示例 App（sample / spms / todo 等）直接改造到 Manifest `dashboard.widgets` + 新写入接口。
- UI 文案从“应用贡献”改为“应用动态”。
- 开发者文档直接以 `dashboard.widgets` 为准，不再提 `dashboard.widget` 贡献。

---

## 4. Dashboard 配置模型

### 4.1 配置层级

```ts
interface DashboardConfig {
  id: string;
  tenantId: string;
  ownerUserId?: string | null; // null = 租户默认；非空 = 用户个性化
  version: number;
  mode: "tenant_default" | "user_custom";
  pages: DashboardPageConfig[];
  updatedAt: string;
}

interface DashboardPageConfig {
  id: string;
  kind: "home" | "tab";
  title: string;
  subtitle?: string;
  icon?: string;
  sort: number;
  sections: DashboardSectionConfig[];
}

interface DashboardSectionConfig {
  id: string;
  title?: string;
  subtitle?: string;
  widgetType: DashboardWidgetType;
  layoutMode: DashboardLayoutMode;
  responsivePreset: "dense" | "balanced" | "editorial" | "single-focus";
  maxItemsPerRow?: 2 | 3 | 4 | 5 | 6;
  appFilter?: {
    mode: "all" | "include" | "exclude";
    appKeys: string[];
  };
  widgets: DashboardWidgetInstanceConfig[];
}

interface DashboardWidgetInstanceConfig {
  id: string;
  widgetRef: {
    owner: "portal" | string;
    key: string;
  };
  spanH?: number;            // 仅高度可覆盖；宽度由 Section maxItemsPerRow + 内容区宽度决定
  config?: Record<string, unknown>;
  sort: number;
}
```

### 4.2 默认配置

初始 Dashboard 等价于当前页面：

1. 首页标题：`你好，{name}`
2. Section 1：`metric` / `tiles`
   - `portal.available-apps`
   - `portal.unread-notifications`
   - `portal.current-role`
3. Section 2：`activity` / `tiles`
   - `portal.app-activity-feed`，默认 `appFilter=all`
4. Section 3：`topic_grid` / `tiles`
   - `portal.favorite-apps`
5. Section 4：`topic_list` / `tiles`
   - `portal.recent-notifications`

这样能无破坏替换现有 Dashboard，同时给后续自定义留下结构。

### 4.3 租户默认 vs 用户自定义

推荐两层：
- 租户默认 Dashboard：租户管理员配置，所有成员默认继承。
- 用户自定义 Dashboard：用户复制租户默认后个性化。

第一期若要收敛范围，可以先做用户级配置；但数据表应预留 `ownerUserId=null` 表示租户默认，避免后续迁移。

合并规则：
- 如果用户没有自定义配置，读取租户默认。
- 如果租户默认不存在，读取系统默认。
- 用户点击“恢复默认”时删除用户自定义配置。

---

## 5. 数据模型

### 5.1 新表

```sql
dashboard_configs
  id uuid pk
  tenant_id uuid not null
  owner_user_id uuid null       -- null = tenant default
  version int not null default 1
  config jsonb not null
  created_at timestamptz
  updated_at timestamptz

dashboard_widget_data
  id uuid pk
  tenant_id uuid not null
  app_id uuid null              -- null = system/computed cache
  owner_user_id uuid null       -- user scoped data; null = tenant scoped
  widget_owner text not null    -- portal or listingKey/contentOwner
  widget_key text not null
  type text not null
  data jsonb not null
  stale_at timestamptz null
  updated_at timestamptz
```

唯一约束注意：`owner_user_id` 可空，**不能**用裸 `unique(... owner_user_id ...)`。Postgres 在唯一约束里把 NULL 视为互不相等，会放进多条 `owner_user_id = null` 的“租户默认”行，约束拦不住。改用 `COALESCE` 表达式唯一索引（手写迁移，与 LMS 字典表同款做法）：

```sql
CREATE UNIQUE INDEX dashboard_configs_uniq
  ON dashboard_configs (tenant_id, COALESCE(owner_user_id, '00000000-0000-0000-0000-000000000000'));

CREATE UNIQUE INDEX dashboard_widget_data_uniq
  ON dashboard_widget_data (
    tenant_id,
    COALESCE(owner_user_id, '00000000-0000-0000-0000-000000000000'),
    widget_owner,
    widget_key
  );
```

> drizzle-kit 不一定能从 schema 生成这种表达式唯一索引，这两条按既有惯例手写进迁移文件。

### 5.2 Marketplace / apps 快照字段

新增开发者字段：

```ts
dashboard?: {
  widgets: DashboardWidgetDef[];
}
```

落库位置：
- `marketplace_listings.dashboard_widgets jsonb default []`
- `apps.dashboard_widgets jsonb default []`

安装/同步时随 `developerFields` 快照拷贝，和 `navItems`、`aclManifest` 一致。

### 5.3 删除旧表

`widget_contributions` 直接删除（未发布、无生产数据）：
- 移除 `/api/me/widgets` 旧接口与 `dashboard.widget` 写路径。
- Dashboard Catalog 只读新 `dashboard_widget_data`，无兼容映射层。
- 改造 sample / spms / todo 等示例 App 到新写入接口（详见 §3.4）。

---

## 6. API 设计

### 6.1 用户 Dashboard 配置

```http
GET  /api/me/dashboard-config
PUT  /api/me/dashboard-config
POST /api/me/dashboard-config/reset
```

- `GET` 返回有效配置：用户自定义 > 租户默认 > 系统默认。
- `PUT` 写用户自定义配置，做 schema 校验、Widget 可见性校验、`maxItemsPerRow`/高度范围校验。
- `reset` 删除用户自定义配置。

> 错误约定（遵循项目 API 规范）：业务校验失败统一返回 **200 + 错误结构**，不用 4xx：
> `{ "ok": false, "error": { "code": "VALIDATION_FAILED", "message": "...", "fields": [...] } }`。
> 4xx/5xx 只留给参数不正确、未登录、未授权、路由不存在等传输/服务层故障。

### 6.2 管理员租户默认配置

```http
GET  /api/admin/dashboard-config
PUT  /api/admin/dashboard-config
POST /api/admin/dashboard-config/reset
```

- 仅租户 admin。
- 写审计：`更新 Dashboard 默认配置`。

### 6.3 Widget Catalog

```http
GET /api/me/dashboard-widgets
```

返回当前用户可选择的 Widget：
- 系统内置 Widget
- 当前租户 active App 声明的 `dashboard.widgets`

过滤规则：
- App 必须 active 且用户可见。
- Widget 所属 App 的 ACL 落地页/相关权限需可访问。
- 对 `app_query` Widget，前端只拿定义；渲染时由宿主代理调用对应 App 服务。
- 对 `app_push` Widget，必须有可用数据或允许显示空态。

### 6.4 App 写入 Widget 数据

```http
PUT    /api/v1/dashboard/widgets/:widgetKey/data
DELETE /api/v1/dashboard/widgets/:widgetKey/data
```

要求：
- TDT scope：`widget.write`
- `claims.aud` 对应的 App Manifest 必须声明 `dashboard.widgets[].key == widgetKey`
- 只能写自己的 Widget
- 默认 user-scoped：写入当前 TDT 用户的数据
- 后续可支持 tenant-scoped，但需要额外 scope 或 admin token

请求示例：

```json
{
  "data": {
    "title": "研发项目动态",
    "items": [
      { "title": "Issue #128 已指派给你", "body": "登录页 2FA 文案调整", "at": "2026-06-10T08:00:00Z", "link": "/app/spms?r=/issues/128" }
    ]
  },
  "staleAfterSec": 3600
}
```

---

## 7. 前端设计

### 7.1 Dashboard 渲染器

新增组件：

```text
DashboardPage
  DashboardTabs
  DashboardRenderer
    DashboardSection
      WidgetFrame
        MetricWidgetRenderer
        ActivityWidgetRenderer
        InsightWidgetRenderer
        TopicGridWidgetRenderer
        TopicListWidgetRenderer
```

要求：
- Section 全宽，不把整个页面做成大卡片。
- Widget 用 8px 或以下圆角，和现有 UI 风格一致。
- 文字溢出要有 line clamp / tooltip / responsive fallback。
- 图表区域必须有稳定高度，避免加载后布局跳动。

### 7.2 配置器

入口：
- Dashboard 页右上角“自定义”。
- 管理员在设置或应用管理里可配置租户默认。

配置 UI 建议：
- 左侧：页面和 Section 树。
- 中间：实时预览。
- 右侧：属性面板。
- 添加 Widget 时打开 Catalog Drawer，按 Widget 类型自动过滤。

操作：
- 新增 Tab / 重命名 / 排序 / 删除 Tab。
- 新增 Section / 选择类型 / 选择布局方式 / 选择响应式预设 / 设置每行列数（maxItemsPerRow）。
- 添加 Widget / 移除 / 排序 / 调整高度。
- 对 `activity` Section 提供 App 勾选器。

### 7.3 Widget 渲染契约

不同类型接受不同数据形状，先定义平台通用最小契约：

```ts
interface MetricWidgetData {
  value: string | number;
  label?: string;
  hint?: string;
  trend?: { direction: "up" | "down" | "flat"; value: string };
}

interface ActivityWidgetData {
  items: Array<{ title: string; body?: string; at?: string; link?: string; icon?: string }>;
}

interface InsightWidgetData {
  chart: {
    kind: "line" | "bar" | "area" | "pie" | "donut" | "funnel";
    x?: string[];
    series: Array<{ name: string; data: number[] }>;
  };
}

interface TopicGridWidgetData {
  items: Array<{ title: string; subtitle?: string; icon?: string; color?: string; link?: string }>;
}

interface TopicListWidgetData {
  items: Array<{ title: string; subtitle?: string; meta?: string; link?: string; icon?: string }>;
}
```

第一期图表建议用已有轻量图表库或引入 Recharts/ECharts 二选一；若仓库已有图表库则沿用。

---

## 8. Manifest 与开发者文档改造

### 8.1 Manifest 字段

在 `SSO与App开发指引.md` 后续补：
- `dashboard.widgets` 字段说明
- `WidgetDef` 尺寸与类型约束
- App 动态写入接口
- `dashboard.widget` 贡献机制已移除、迁移到 `dashboard.widgets` 的说明

### 8.2 SDK

`packages/portal-sdk` 用新方法替换旧的 `contributeWidget`/`clearWidget`（直接删除旧方法，未发布无需 deprecate 过渡）：

```ts
sdk.dashboard.putWidgetData(widgetKey, data, opts?)
sdk.dashboard.clearWidgetData(widgetKey)
```

---

## 9. 迁移策略

### 9.1 现有 Dashboard 迁移

当前硬编码内容迁移成系统默认 Dashboard：
- `StatTile` -> `portal.*` metric Widget
- `dashWidgets` -> `portal.app-activity-feed`
- `favApps` -> `portal.favorite-apps`
- `recentNotif` -> `portal.recent-notifications`

### 9.2 文案迁移

- “应用贡献” -> “应用动态”
- `dashboard.contributions` locale 改为 `dashboard.appActivity`
- 开发者文档直接以 `dashboard.widgets` 为准（旧 `dashboard.widget` 贡献机制不再存在）

### 9.3 数据迁移

产品未发布、无生产数据，无需数据迁移脚本：直接删除 `widget_contributions` 表，以新表 `dashboard_widget_data` 为唯一数据源。改造示例 App（sample / spms / todo）后，它们的动态通过新接口重新写入即可。

---

## 10. 分期实施计划

### Phase 1 · 模型与只读渲染

目标：新 Dashboard 渲染器可用，但配置先用系统默认。

改动：
- `packages/shared`：新增 Dashboard 类型。
- `apps/api`：新增 Widget Catalog 聚合接口、默认配置生成。
- `apps/web`：重写 `DashboardPage` 为配置驱动渲染。
- 暂不动旧 `/api/me/widgets`（Phase 4 随 `widget_contributions` 一并删除）。

验收：
- 当前 Dashboard 视觉和信息不倒退。
- 系统默认配置能渲染指标、应用动态、常用应用、最近通知。

### Phase 2 · 用户自定义配置

目标：普通用户可自定义自己的 Dashboard。

改动：
- 新增 `dashboard_configs` 表。
- `GET/PUT/reset /api/me/dashboard-config`。
- Web 配置器：页面/Tab/Section/Widget 增删改排。
- Widget span 和 Section 布局校验。

验收：
- 用户可新增 Tab。
- 首页和 Tab 可改标题/副标题。
- 可添加不同 Widget 类型区域。
- 可从 Catalog 选择 Widget 放入区域。
- 刷新后配置保留；恢复默认可用。

### Phase 3 · App Manifest Widgets

目标：应用可通过 Manifest 暴露 Widget。

改动：
- `marketplace_listings` / `apps` 增 `dashboard_widgets`。
- 市场清单表单和详情展示 Widget 列表。
- 安装/同步快照支持该字段。
- `GET /api/me/dashboard-widgets` 纳入 App Widget。

验收：
- 一个测试 App 声明 `metric/activity/insight` Widget 后出现在 Catalog。
- 未安装/停用 App 的 Widget 不出现。
- 用户无权访问 App 时 Widget 不出现。

### Phase 4 · App Activity 数据写入

目标：应用动态正式替换应用贡献。

改动：
- 新增 `dashboard_widget_data` 表。
- 新增 `/api/v1/dashboard/widgets/:widgetKey/data`。
- SDK 用 `dashboard.putWidgetData` 替换旧 `contributeWidget`。
- 删除 `widget_contributions` 表、`dashboard.widget` 写路径、`/api/me/widgets`。
- 改造 sample / spms / todo 示例 App 到新接口。
- Dashboard 配置器支持 activity Section 的 App 勾选。

验收：
- App 只能写自己声明过的 Widget。
- 动态面板可按 App include/exclude。
- 示例 App 通过新接口写入的动态正确显示为“应用动态”。
- 代码中已无 `widget_contributions` / `dashboard.widget` 残留。

### Phase 5 · 洞察图表与高级布局

目标：BI 感更强，支持图表和 Bento/Masonry。

改动：
- 引入/复用图表组件。
- `insight` Widget 数据契约落地。
- Section 支持 `bento`。
- `waterfall/masonry` 按优先级实现。

验收：
- 一个 App 暴露图表 Widget 并可配置到洞察区域。
- Bento 中大卡/小卡布局稳定。
- 2/3/4/5/6 个 Widget 一行的预设都能正确响应。

### Phase 6 · 租户默认与治理

目标：管理员可配置全租户默认 Dashboard。

改动：
- `/api/admin/dashboard-config`。
- 用户继承/覆盖逻辑。
- 配置变更写审计。
- 可导入/导出 Dashboard JSON。

验收：
- 管理员更新默认后，新用户看到新默认。
- 已有用户自定义不被覆盖。
- 用户可恢复到租户默认。

---

## 11. 验收清单

- 首页不可删除，但可改标题、副标题和内容区域。
- 可新增、重命名、排序、删除 Tab。
- Section 必须先选择 Widget 类型，Widget 选择列表按类型过滤。
- 一行支持 2/3/4/5/6 个 Widget，随内容区真实宽度（含左侧导航展开/收起）自动减列。
- 每种 Widget 类型有默认每行数（maxItemsPerRow 初值）与高度范围，越界配置被拒绝。
- `activity` Section 可按 App 勾选显示范围。
- 系统内置 Widget 和 App Widget 在一个 Catalog 中选择。
- App Manifest 声明的 Widget 能随安装快照进入租户 App。
- App 不能写未声明的 Widget 数据。
- 示例 App 通过新接口写入的动态显示为“应用动态”；旧 `dashboard.widget` / `widget_contributions` 已彻底移除。
- Dashboard 配置保存、恢复默认、跨刷新稳定。
- 移动端不出现横向溢出，Widget 按预设折叠。
- 配置接口有 schema 校验，校验失败返回 200 + 错误结构（`error.code = VALIDATION_FAILED`），不用 4xx。
- 管理员默认配置变更写审计。

---

## 12. 关键风险与决策点

| 风险 | 说明 | 建议 |
| --- | --- | --- |
| 过早做自由拖拽 | 响应式和持久化复杂度高，容易导致 UI 不稳定。 | 第一期用 Section + 预设布局，后续再加拖拽。 |
| Widget 数据契约过散 | 每个 App 返回任意 JSON 会让渲染器不可控。 | 先定义 5 类 Widget 的平台通用最小契约；高级自定义后置。 |
| 图表库选择 | ECharts 功能强但体积大；Recharts 轻但复杂图弱。 | 先调研现有依赖；若无，优先 Recharts，复杂 BI 后续再升级。 |
| 租户默认和用户自定义冲突 | 管理员改默认是否覆盖用户配置容易引发困惑。 | 明确“用户自定义优先，不自动覆盖”；提供恢复默认。 |
| 替换旧 extPoints 写路径 | 删除 `dashboard.widget` 会影响 sample/spms/todo。 | 未发布、无生产数据，直接把这三个示例 App 改造到新机制，不保留兼容层。 |
| ACL 绑定 | Widget 是否需要独立权限可能膨胀。 | 第一期继承 App 可见性和页面权限；独立 Widget 权限后置。 |

---

## 13. 推荐的最终产品形态

Dashboard 不再是“Portal 首页堆几个固定卡片”，而是一个**面向租户工作流的可组合 BI 工作台**：

- 管理员给租户配置默认工作台：例如校长看到“学校经营驾驶舱”，研发负责人看到“项目健康度”，教师看到“今日教学任务”。
- 普通用户在默认基础上增加个人 Tab：例如“我的待办”“质量监控”“AI 使用情况”。
- 应用通过 Manifest 把自己可展示的信息产品化为 Widget，而不是让 Portal 猜业务数据怎么展示。
- “应用动态”成为跨应用轻量 feed；“洞察面板”承担真正 BI 图表；“专题网格/列表”承担入口和清单型工作流。

这个模型能同时承接当前 Dashboard、应用动态、未来 BI 图表和各业务 App 的看板需求，而且不会把平台拖进每个业务 App 的私有 UI 细节里。
