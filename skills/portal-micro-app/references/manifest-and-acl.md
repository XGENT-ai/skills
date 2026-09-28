# Manifest 与 ACL 声明（微应用视角）

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。

## 1. 模型：开发者字段快照

Manifest 的权威载体是一条**市场清单（marketplace listing）**。安装 = 把开发者字段快照复制进该租户的 `apps` 行；清单升级后租户「同步更新」重拷开发者字段（运营字段不动）。`listingKey` 就是 TDT 的 `aud`，不可改。

⚠️ **ACL Manifest 和 navItems 只能由应用自身声明**（自己的 `app.manifest.json`），租户管理员不能在注册表单手填。修改后提交完整清单发布提案，按治理要求审批，并核对已安装租户的同步结果。

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
| `embedCsp` | 托管前端的精确来源声明；开发版本支持 connectSrc/scriptSrc/styleSrc/fontSrc/imgSrc/mediaSrc，须正式发布并启用 strict/v2 后使用，旧门户只支持 connectSrc |
| `tdtTtl` | TDT 有效期秒数，60–86400，默认 3600 |

运营字段（租户管理员设，非 Manifest）：`visibility` / `showInCenter` / `pinned` / `enabledNavItemIds` / `webhookUrl` / `allowExchange` / `exchangeWhitelist`。

## 3. ACL Manifest schema

匿名页面可以位于 `/apps/<key>/…`，不应在该路由先强制 SDK 登录；数据授权仍归 App。
后端直出 HTML 可以正式使用 `/svc/<key>/…`，但其 cookie Path、表单、资源、fetch、
Location 和受保护预览必须适配外部 base。独立域名复用 `publicEntrypoints` + Sites，
声明本身不创建门户根级路径。详见 portal-external-app 的公开入口参考；发布与 CSP
实际验收见 xgent-app-release。改 listingKey 是身份/资源迁移，不是显示名修改。

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

## CSP 与发布确认的版本边界

新增扩展能力尚未因本文更新而上线；strict 默认关闭，需平台盘点来源并受控启用 v2。六类来源
使用 HTTPS origin（connectSrc 另允许 WSS），禁止通配、路径、凭据和 unsafe-inline/unsafe-eval；
platformSources 仅 scriptSrc/styleSrc/fontSrc/connectSrc 可声明 ["jsCdn"]，跟随平台 CDN。
省略整个 embedCsp 沿用，null 清空，对象是完整替换；来源增删均治理，排序去重不算变更。

新版 delivery 追踪本次 proposalId 的审批、清单/产物、后端和网关四阶段。纯前端 backend 为
not_applicable 仍须等网关；unknown 不当成功，superseded 表示目标被后续取代。旧门户无 delivery
不能确认网关。CSP 只扩展本 App 的 /apps 前端，不更改门户壳、沙箱或 /svc 后端公开页策略；
公开数据授权与真实浏览器主路径仍需单独验证。
