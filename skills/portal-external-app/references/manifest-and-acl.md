# Manifest 与 ACL 声明（微应用视角）

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。

## 1. 模型：开发者字段快照

Manifest 的权威载体是一条**市场清单（marketplace listing）**。安装 = 把开发者字段快照复制进该租户的 `apps` 行。`listingKey` 就是 TDT 的 `aud`，不可改。

清单升级怎样到达**已安装**的租户（新安装总是取获批清单的当前值）：

- **发布提案批准后自动同步**：`aclManifest` 整份替换，并为受影响租户补齐内置 `member` 基线（只补新的 `defaultForMember` 授予，不收回管理员手工改过的授予）；`navItems` 只按 `id` 补新增项，已存在项的 label/icon/path 不更新；`scopes` 保持安装时的快照，不自动扩大。
- **新增 scope 与已有导航项的修改**要等租户管理员执行应用更新（`POST /api/admin/market/listings/<listingKey>/update`）：它把开发者字段（含 `scopes`、`navItems`）重拷为清单当前值，运营字段不动，并再对齐 member 基线。所以新版代码要容忍租户仍停在旧 scope 集合上，按令牌实际带的 scope 降级。
- 本地一盒 `register-app` 注册时，只有清单声明了 `logoUrl` 才自动同步已装租户；否则改完 ACL 或 navItems 后同样要执行上面的应用更新。

⚠️ **ACL Manifest 和 navItems 只能由应用自身声明**（自己的 `app.manifest.json`），租户管理员不能在注册表单手填。修改后提交完整清单发布提案，按治理要求审批，并核对已安装租户的同步结果。

## 2. 微应用相关的开发者字段

| 字段 | 说明 |
| --- | --- |
| `listingKey` | 小写字母或数字开头，只含小写字母、数字、中划线，最长 63。门户自身路径段（`api`、`apps`、`console`、`admin` 等）和平台自有 App 的标识是保留字，申请时报 `KEY_RESERVED`。= TDT `aud`、PID 前缀，不可改 |
| `name` / `tagline` / `desc` / `color` / `cat` | 展示信息 |
| `icon` | 门户图标注册表里的名字（lucide 名，如 `file-question`、`git-branch`），选与功能对应、不与已装 App 撞的图标。清单只校验是字符串：写了注册表里没有的名字不会报错，宿主静默显示通用网格图标，需要的图标缺失时请平台补登记 |
| `type` | `micro`（iframe 嵌入，本 skill 场景）/ `link`（独立站点）/ `native` / `service`（无前端）。缺省 `micro` |
| `openModes` | 打开方式子集：`embed`（门户 iframe）/ `standalone`（从 `/open/<key>` 顶层打开）。省略 = `["embed"]`，`service` 省略为 `[]`；`service` 必须为空，其他类型不得为空。整组替换，治理档 |
| `landingUrl` | `type: "link"` 必填：门户在新标签打开这个绝对地址，不嵌入、不代理、不握手、不注入 TDT；需要用户身份时站点自己走 OAuth 授权码（`redirectUrls` / `publicClient`，见 [原生客户端](native-clients.md)）。只收 https（http 仅限 `localhost`/`127.0.0.1`/`[::1]`），不得含用户名或口令。治理档 |
| `embedUrl` | iframe 源。生产同源托管为 `/apps/<key>/`。宿主自动放行它的源做 postMessage 握手；`allowedOrigins` **不是**清单字段，额外来源由管理员在应用配置里设 |
| `scopes` | 申请的权限范围（最小够用——授权屏逐条展示，越多越劝退）。只能是平台基础 scope、本 App 命名空间（`<listingKey>.` 或中划线换下划线的前缀），或 `exchangeTargets` 里目标 App 的命名空间；SERVICE_ONLY scope 一律拒收。完整目录与文案以 `@xgent/shared` 导出的 `PLATFORM_BASE_SCOPES` / `SERVICE_ONLY_SCOPES` / `SCOPE_LABELS` 为准 |
| `scopeLabels` | App 命名空间 scope 的同意页三语文案 |
| `navItems` | `{ id, label, icon, path }[]`，可由租户管理员提升到门户左侧导航的**快捷方式候选**，默认不显示。它不是 App 的导航菜单，App 内导航见 [微应用开发](micro-app.md#设计红线平台一致性) |
| `helpEntry` | 版头帮助按钮的入口，可空字符串。`"/help"` = App 内路由（宿主置 `?r=`，自动通过档）／`"https://…"` = 外站文档（新标签 + `noopener`，治理档）。不声明就不出这枚按钮 |
| `dashboardWidgets` | 可加到 Dashboard 的 Widget 声明（**顶层数组**；嵌套的 `dashboard.widgets` 门户从不读取、提交即拒）。契约见 [open-api-and-extensions.md §3](open-api-and-extensions.md#3-dashboard-widget应用动态) |
| `adminPages` | 两级管理入口：`platform` 为平台管理员、`tenant` 为当前租户管理员。各级可声明 `{ label, path, scopes, schema }`（micro/service），或 `{ route, label }` 嵌入页（仅 micro + embed）；均为治理档。嵌入时 `init.adminPage` 有值、App 只渲染该页。配置作用域与 GET/PUT 合同见 [配置分层](config-placement.md) |
| `sensitiveActions` | 需要用户再次验证身份的敏感动作：`[{ key, labelI18n: { "zh-CN", "en"?, "zh-TW"? }, pid, displayFields: ["字段名"] }]`。`key` 在本 App 内唯一；缺 `key`/`pid`/`labelI18n["zh-CN"]` 的条目和重复 key **静默丢弃**，不报错，发布后核对生效声明。增删动作、改 `pid` 或 `displayFields` 均为治理档（只调顺序不算变更），获批后随自动同步到达已装租户（运行期按安装快照判定）。同时要在 `scopes` 声明平台基础 scope `stepup.manage`；运行期未声明的动作报 `STEP_UP_ACTION_NOT_DECLARED`。流程见 [SDK 参考](sdk-reference.md#敏感动作再次验证requeststepup) |
| `dependencies` | 依赖的其他清单（安装按拓扑序补装；卸载被依赖会拦截）。错误码：`DEPENDENCY_CYCLE`（成环，含依赖自身）、`DEPENDENCY_UNAVAILABLE`（依赖清单不存在，或安装时未上架）、`DEPENDENCY_REQUIRED`（卸载/停用仍被其他已装 App 依赖的 App） |
| `exchangeTargets` | 经令牌交换读取哪些 App（安装时自动建交换白名单 + consent 共授） |
| `embedCsp` | 托管前端的精确来源声明；开发版本支持 connectSrc/scriptSrc/styleSrc/fontSrc/imgSrc/mediaSrc，须正式发布并启用 strict 后使用，旧门户只支持 connectSrc |

运营字段（租户管理员设，非 Manifest）：`visibility` / `showInCenter` / `pinned` / `enabledNavItemIds` / `webhookUrl` / `allowExchange` / `exchangeWhitelist`。

令牌策略同样归租户管理员（应用配置里设，Manifest 不接受，写了只会让发布进人工审核且不生效）：`tdtTtl`（TDT 有效期秒数，60–86400，缺省 3600）/ `allowRefresh` / `refreshTtl` / `allowedGrants`（允许的授权方式，缺省全开；关掉 `token_exchange` 后本 App 发起交换报 `GRANT_NOT_ALLOWED`）。App 不要依赖某个固定的 TDT 寿命。

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
  relations?: { key: string; label: I18nText }[]; // 本 App 的归属关系词表，如 lead / assignee
}
interface AclPage {
  key: string;               // 稳定 id，进 PID
  path: string;              // 路由 pattern，匹配运行时 ?r=
  label: I18nText;           // 字符串或 { "zh-CN", "en" }
  parentKey?: string;        // 页面树（矩阵分组 + 前缀通配）
  navItemId?: string;        // 关联 navItems[].id（侧栏显示的前提，见 §5）
  supportedScopes?: DataScope[]; // 缺省 ["all"]
  defaultForMember?: boolean;    // 授予内置 member 基线
  supportedRelations?: string[]; // 矩阵可为本项选的归属关系（⊆ relations）；缺省 = 不提供
  defaultCondition?: { dataScope?: DataScope; relations?: string[] }; // 随 defaultForMember 一起种下的条件
}
interface AclAction {
  key: string; label: I18nText; pageKey?: string;
  supportedScopes?: DataScope[]; dangerous?: boolean; defaultForMember?: boolean;
  supportedRelations?: string[]; defaultCondition?: { dataScope?: DataScope; relations?: string[] };
}
interface AclRoleTemplate {
  key: string; name: I18nText; desc?: I18nText;
  grants: { pid: string; condition?: { dataScope?: DataScope; relations?: string[] } | null }[];
  preset?: boolean; // true：安装/对齐时直接建成真角色（按 key 幂等），管理员只需发给成员
}
```

**归属关系（relations）**：授予可以带「只限本人处于某关系的行」的条件，关系名由 App 在根 `relations` 声明、由 App 后端判定（自省 `permissions[].relations`，执行要求见 [集成契约 §3](integration-contract.md#3-运行时鉴权四道闸)）。一条条件要么是 `dataScope`、要么是 `relations`，不能同时有。写入时校验，不通过整份拒收（`VALIDATION_FAILED`）：`relations[].key` 唯一且匹配 `^[a-z0-9][a-z0-9_-]*$`、必须带 `label`；`supportedRelations` 只能引用已声明的关系；`defaultCondition.relations` 必须是本项 `supportedRelations` 的子集；角色模板里带 relations 的授予不能用通配 PID，且目标项（本 App 内）必须支持这些关系。不声明 `supportedRelations` 的项，管理员在矩阵里配不出关系条件。

`preset` 模板每个 PID 只播种一次：管理员之后手工收回的授予不会被下次对齐顶回；只种本 App 前缀的 PID。

**PID 语法** `<listingKey>:<kind>:<key>`，`kind ∈ {page, action}`。前缀固定用 `listingKey`，即使某租户的安装 appKey 与它不同；通配由粗到细：

```
<listingKey>:*  >  <listingKey>:page:*  >  <listingKey>:page:projects.*  >  <listingKey>:page:projects
```

**DataScope（ABAC-lite）** `own` < `team` < `all`：`own` 行的 ownerUserId==当前用户；`team` 行属于用户所在组；`all` 无行级过滤。同一 PID 命中多条授予时最宽范围胜出。

**模型**：纯加法 RBAC——角色只授予不拒绝、有效权限=所有角色授予并集、默认拒绝、租户 `admin` bypass 一切、`member` 基线自动获得各应用的 `defaultForMember` 项。

## 4. 运行时两道门

1. **前端 UX 门**：握手注入 `init.acl` → `sdk.acl.can(pid)` / `sdk.acl.scope(pid)` 隐藏入口/按钮。**仅 UX，不可信。**
2. **后端安全门**：独立后端从自省结果拿 `bypass`/`permissions`/`groups` 真正拦截。前端判过 ≠ 安全。

## 5. navItems 生命周期

- 市场安装时 `navItems` 快照到 `apps.navItems`，`enabledNavItemIds` 默认为空，即全部不进左侧导航。租户管理员在「应用配置 → 顶部导航」逐项开启，只能启停声明项，不能手填新项。因此左侧导航里有没有这些入口因租户而异，App 的页面入口必须由 App 内导航保证。
- Shell 经 `GET /api/apps/nav` 聚合可见应用的已启用项；点击打开 `/app/:appKey?r=<path>`，SDK 握手时 `init.route` 就是这个 path。
- 一项进左侧导航须**同时**满足：租户管理员已启用它；该 App 对当前用户可见；有某个 `AclPage.navItemId` 指向它的 `id`。没有任何页面绑定的项，启用了也永不显示。侧栏**不**按逐页 ACL 再筛：能看见 App 的成员都看得见已启用的项，点进去后由 App 前端 `sdk.acl` 做 UX 门、后端做安全门。
- 约定：`id` 应用内稳定（改 id = 删旧菜单加新菜单，租户启用状态受影响）；`path` 用应用内部路由（`/` 开头）；`icon` 规则同 §2 的 `icon` 字段。

## CSP 与发布确认的版本边界

新增扩展能力尚未因本文更新而上线；strict 默认关闭，需平台盘点来源后受控启用。六类来源
使用 HTTPS origin（connectSrc 另允许 WSS 及原样 `blob:`/`data:`），禁止通配、路径、凭据和 unsafe-inline/unsafe-eval；
platformSources 仅 scriptSrc/styleSrc/fontSrc/connectSrc 可声明 ["jsCdn"]，跟随平台 CDN。
省略整个 embedCsp 沿用，null 清空，对象是完整替换；来源增删均治理，排序去重不算变更。

新版 delivery 追踪本次 proposalId 的审批、清单/产物、后端和网关四阶段。纯前端 backend 为
not_applicable 仍须等网关；unknown 不当成功，superseded 表示目标被后续取代。旧门户无 delivery
不能确认网关。CSP 只扩展本 App 的 /apps 前端，不更改门户壳、沙箱或 /svc 后端公开页策略；
公开数据授权与真实浏览器主路径仍需单独验证。
