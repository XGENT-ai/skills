# Manifest 与 ACL 声明（微应用视角）

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §4/§13.2（2026-07）（门户仓文件，App 自己的 repo 里没有；本文件已自包含，不必去找）。冲突时以门户仓库原文为准。

## 1. 模型：开发者字段快照

Manifest 的权威载体是一条**市场清单（marketplace listing）**。安装 = 把开发者字段快照复制进该租户的 `apps` 行；清单升级后租户「同步更新」重拷开发者字段（运营字段不动）。`listingKey` 就是 TDT 的 `aud`，不可改。

⚠️ **ACL Manifest 和 navItems 只能由应用自身声明**（代码注册表 / 市场清单），租户管理员不能在注册表单手填。门户 monorepo 内的声明位置：ACL → `apps/api/src/modules/acl/manifests.ts`；listing → `apps/api/src/db/provisioning.ts` 的 `LISTING_DEFS`；改完重新 seed / 发布清单 / 租户同步更新。

## 2. 微应用相关的开发者字段

| 字段 | 说明 |
| --- | --- |
| `listingKey` | `^[a-z0-9-]+$`，2–40。= TDT `aud`，不可改 |
| `name` / `tagline` / `desc` / `icon` / `color` / `cat` | 展示信息 |
| `type` | `micro`（iframe 嵌入，本 skill 场景）/ `link` / `native` / `service` |
| `embedUrl` | iframe 源。生产同源托管为 `/apps/<key>/` |
| `allowedOrigins` | postMessage 握手来源白名单 |
| `scopes` | 申请的权限范围（最小够用——授权屏逐条展示，越多越劝退） |
| `scopeLabels` | App 命名空间 scope 的同意页三语文案 |
| `navItems` | `{ id, label, icon, path }[]` 贡献到左侧导航 |
| `helpEntry` | 版头帮助按钮的入口，可空字符串。`"/help"` = App 内路由（宿主置 `?r=`，自动通过档）／`"https://…"` = 外站文档（新标签 + `noopener`，治理档）。不声明就不出这枚按钮 |
| `dashboard.widgets` | 可展示的 Dashboard Widget 声明 |
| `extPoints` | 仅 `settings.section` |
| `dependencies` | 依赖的其他清单（安装按拓扑序补装；卸载被依赖会拦截） |
| `exchangeTargets` | 经令牌交换读取哪些 App（安装时自动建交换白名单 + consent 共授） |
| `embedCsp.connectSrc` | 同源托管下的额外 connect-src（如直传对象存储） |
| `tdtTtl` | TDT 有效期秒数，60–86400，默认 3600 |

运营字段（租户管理员设，非 Manifest）：`visibility` / `showInCenter` / `pinned` / `enabledNavItemIds` / `webhookUrl` / `allowExchange` / `exchangeWhitelist`。

## 3. ACL Manifest schema

```ts
interface AclManifest {
  version: string;          // reconcile 基准
  landingPageKey?: string;  // 应用可见性判定页（缺省第一个 page）
  groups?: AclGroup[];
  pages: AclPage[];
  actions: AclAction[];
  roleTemplates?: AclRoleTemplate[]; // 管理员可"从模板克隆"角色（克隆后脱钩）
}
interface AclPage {
  key: string;               // 稳定 id，进 PID
  path: string;              // 路由 pattern，匹配运行时 ?r=
  label: I18nText;           // 字符串或 { "zh-CN", "en" }
  parentKey?: string;        // 页面树（矩阵分组 + 前缀通配）
  navItemId?: string;        // 关联 navItems[].id
  supportedScopes?: DataScope[]; // 缺省 ["all"]
  defaultForMember?: boolean;    // 授予内置 member 基线
}
interface AclAction {
  key: string; label: I18nText; pageKey?: string;
  supportedScopes?: DataScope[]; dangerous?: boolean; defaultForMember?: boolean;
}
```

**PID 语法** `<appKey>:<kind>:<key>`，`kind ∈ {page, action}`；通配由粗到细：

```
<appKey>:*  >  <appKey>:page:*  >  <appKey>:page:projects.*  >  <appKey>:page:projects
```

**DataScope（ABAC-lite）** `own` < `team` < `all`：`own` 行的 ownerUserId==当前用户；`team` 行属于用户所在组；`all` 无行级过滤。同一 PID 命中多条授予时最宽范围胜出。

**模型**：纯加法 RBAC——角色只授予不拒绝、有效权限=所有角色授予并集、默认拒绝、租户 `admin` bypass 一切、`member` 基线自动获得各应用的 `defaultForMember` 项。

## 4. 运行时两道门

1. **前端 UX 门**：握手注入 `init.acl` → `sdk.acl.can(pid)` / `sdk.acl.scope(pid)` 隐藏入口/按钮。**仅 UX，不可信。**
2. **后端安全门**：独立后端从自省结果拿 `bypass`/`permissions`/`groups` 真正拦截。前端判过 ≠ 安全。

## 5. navItems 生命周期

- 市场安装时 `navItems` 快照到 `apps.navItems`，默认全部启用（`enabledNavItemIds`）；管理员之后只能启停声明项，不能手填新项。
- Shell 经 `GET /api/apps/nav` 聚合可见应用的已启用项；点击打开 `/app/:appKey?r=<path>`，SDK 握手时 `init.route` 就是这个 path。
- `navItems[].id` 绑定 `AclPage.navItemId` 后，侧栏按用户 ACL 隐藏无权入口——但应用前端仍要 `sdk.acl` 做 UX 门、后端仍做安全门。
- 约定：`id` 应用内稳定（改 id = 删旧菜单加新菜单，租户启用状态受影响）；`path` 用应用内部路由（`/` 开头）；`icon` 用 Portal 支持的图标名。
