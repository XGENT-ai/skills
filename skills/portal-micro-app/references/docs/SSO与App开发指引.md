# SSO 与 App 开发指引

面向**接入 XGENT.ai Portal 的应用开发者**。本文讲清三件事：

1. **SSO（单点登录）** —— Portal 如何认证用户、如何把"用户身份"安全地传给你的应用。
2. **App Manifest** —— 一个应用如何**自描述**自己的形态、权限范围、扩展点、导航、依赖与权限模型（ACL）。这是接入的"契约"。
3. **App 接入** —— 三种集成方式（嵌入式微应用 / 服务端 OAuth / 独立后端应用）、用 SDK 或令牌调用 Portal Open API、贡献小组件 / 计划任务 / 通知，以及跨应用互相调用。

> 阅读前置：[`README.md`](../README.md)（怎么把平台跑起来）、[`PRODUCT.md`](../PRODUCT.md)（产品定位）。
> 本文所有 HTTP 约定遵循 [`CLAUDE.md`](../CLAUDE.md)：**业务状态不走 HTTP 状态码**，一律 `200 + 响应体`。

---

## 目录

- [1. 核心模型：会话与 TDT](#1-核心模型会话与-tdt)
- [2. 用户登录 Portal（SSO 入口）](#2-用户登录-portalsso-入口)
- [3. 应用从哪来：应用市场 vs 自助注册](#3-应用从哪来应用市场-vs-自助注册)
- [4. App Manifest 规范](#4-app-manifest-规范) ★
- [5. 集成方式一：嵌入式微应用（iframe + SDK）](#5-集成方式一嵌入式微应用iframe--sdk)
- [6. 集成方式二：服务端 OAuth（authorization_code）](#6-集成方式二服务端-oauthauthorization_code)
- [7. 集成方式三：独立后端应用（introspection + 服务账号）](#7-集成方式三独立后端应用introspection--服务账号)
- [8. Portal Open API v1](#8-portal-open-api-v1)
- [9. TDT 详解：结构与生命周期](#9-tdt-详解结构与生命周期)
- [10. 授权与同意（Consent）](#10-授权与同意consent)
- [11. 跨应用调用（令牌交换 Token Exchange）](#11-跨应用调用令牌交换-token-exchange)
- [12. 权限管理（ACL）运行时](#12-权限管理acl运行时)
- [13. 扩展点与公共服务](#13-扩展点与公共服务)
- [14. 本地开发与调试](#14-本地开发与调试)
- [15. 外部服务类应用接入（Docker 镜像交付）](#15-外部服务类应用接入docker-镜像交付) ★
- [附录 A：Scope 目录](#附录-ascope-目录)
- [附录 B：错误码目录](#附录-b错误码目录)
- [附录 C：Manifest / 注册字段参考](#附录-cmanifest--注册字段参考)

---

## 1. 核心模型：会话与 TDT

Portal 有意把**"用户在 Portal 的登录态"**和**"应用调用 Open API 的凭证"**分成两套，永不混用：

| | Portal 会话（Session） | TDT（Tenant-Delegated Token） |
| --- | --- | --- |
| 是什么 | 用户在 Portal 这一侧的登录态 | 应用代表"某用户在某租户下"（或某服务在某租户下）调用 Open API 的短期凭证 |
| 载体 | `httpOnly` Cookie + Redis 会话（应用读不到） | JWT（HS256），放在 `Authorization: Bearer <TDT>` |
| 维度 | （user） | 用户态：user × tenant × app；服务态：service × tenant |
| 有效期 | 7 天 | 默认 3600s（应用可配 `tdtTtl`，60s–24h） |
| 谁签发 | `/auth/*` 登录链路 | `/api/tokens/mint`（宿主注入）、`/oauth/token`（服务端 OAuth）、`/api/tokens/service`（服务账号 client_credentials） |
| 给谁用 | 只给 Portal 自己的前端/后端 | 给你的应用 |

**TDT 有两种**（`kind`）：

- **用户态**（`kind:"user"`，最常见）—— 代表某个登录用户。携带 `user_id`，可解析出该用户的角色与 ACL 权限。
- **服务态**（`kind:"service"`）—— 代表一个服务账号（M2M），**无用户上下文**。由 `client_credentials` 自助签发，用于后台/批处理调用租户级 Open API。服务态令牌**整体绕过用户 ACL**（见 [§12](#12-权限管理acl运行时)）。

**为什么分两套？** 应用拿不到 Portal 的会话 Cookie，也就无法冒充用户在 Portal 上做任何事；应用能拿到的只是一个**作用域受限、短期、可随时吊销**的 TDT。即使 TDT 泄露，影响面被 scope、租户、有效期、撤销机制层层收窄。

```
 浏览器 ──Cookie(httpOnly)──► Portal 前端 ──Session──► Portal API
                                   │
                            （宿主注入 TDT）
                                   ▼
   你的应用（iframe / 服务端）──Bearer TDT──► Portal Open API (/api/v1/*)
                                   │
                            （或代表用户调用你自己的独立后端）
                                   ▼
   你的独立后端 ──introspect──► Portal 验证 TDT（§7）
```

---

## 2. 用户登录 Portal（SSO 入口）

用户怎么登进 Portal，是你的应用拿到身份的前提。你的应用**不需要自己实现登录**——用户只要登录了 Portal，进入你的应用时身份就会被宿主透传过来。

### 2.1 支持的登录方式

| 方式 | 说明 | 接口 |
| --- | --- | --- |
| OAuth 第三方 | `github` / `google` / `apple` / `wechat`(微信) / `lark`(飞书)，标准授权码 + PKCE(S256) | `GET /auth/:provider/start` |
| 邮箱密码 | argon2id 哈希校验 | `POST /auth/password/login` |
| 本地开发账号 | dev mock IdP，仅 `DEV_MOCK_OAUTH=true` 时可用，本地把整条登录链路跑通 | `GET /auth/dev/start` |
| 两步验证（2FA） | 上述任一方式成功后，若用户开了 TOTP，则进入待验证态 | `POST /auth/tfa/verify` |

所有 `redirect_uri` 都回到 **Portal 自己的源**（`/auth/:provider/callback`），以保证会话 Cookie 始终是第一方 Cookie。登录成功后写入会话并落审计。

### 2.2 多租户：身份全局、成员/角色按租户

- **身份是全局的**：一个用户在 Portal 只有一份身份。
- **成员关系与角色是按租户的**：同一个人可以是 A 租户的 `admin`、B 租户的 `user`。
- 登录后默认落到用户的某个**非平台**租户；顶栏可切换：`POST /auth/switch-tenant { tenantId }`。
- 你的应用拿到的每个 TDT 都带 `tenant_id`，**所有业务查询都按租户隔离**。同一个应用在不同租户下是相互独立的实例（独立的 `appKey` 记录、密钥、配置、数据）。

### 2.3 自定义域名与品牌（可选）

平台支持每个租户绑定自定义域名。当用户从某个**已验证**域名访问登录页时：

- `GET /auth/branding` 返回该域名所属租户的品牌（名称 / Logo / 主色），登录页与顶栏据此换肤。
- 登录默认租户会优先落到该域名对应的租户（前提：用户是其成员）。

本地无 DNS，可用 `?__host=` 模拟：`http://localhost:5300/login?__host=star.example.com`（仅 `DEV_MOCK_OAUTH=true` 时生效）。

> 对应用开发者的影响：通常**无**。身份与租户解析都在 Portal 侧完成，应用只消费 TDT 里的 `tenant_id`。

---

## 3. 应用从哪来：应用市场 vs 自助注册

一个可被租户使用的应用，最终是一条租户下的 `apps` 记录，其 `appKey` 就是你应用的 **client_id**，也是 TDT 的 `aud`。它有**两条来路**：

### 3.1 路线一：应用市场（推荐）

平台维护一份全局的**应用清单（marketplace listing）**目录。开发者把应用定义为一条 listing（它就是应用的 **Manifest**，见 [§4](#4-app-manifest-规范)），平台管理员发布（`draft → published`）后，**租户管理员**在「应用市场」里**安装**它。

- 安装 = 把 listing 的**开发者字段快照复制**进该租户的一条 `apps` 记录，并记录来源指针（`marketplaceListingId` + `marketplaceVersion`），以便后续"有更新"时一键同步。
- 安装时：默认启用 listing 声明的所有导航项；`micro` 应用自动生成一把 App Secret；ACL Manifest 随快照写入 `apps.aclManifest`，进入角色矩阵的可授权范围（其中 `defaultForMember` 项对 `member` 基线的对账见 [§4.6](#46-manifest-生命周期)）。
- **依赖**：listing 可声明依赖（`dependencies: listingKey[]`）。安装会**按拓扑序自动补装**缺失依赖；卸载被依赖的应用会被拦截（除非 `force`）。依赖图在编辑时做**环检测**。见 [§4.5](#45-依赖dependencies)。

清单状态：`draft`（平台内部草稿）/ `published`（可被租户安装）/ `delisted`（下架但已装实例继续可用）。

> **零改码 · 零重部署（APP-INTEGRATION）**：新增/上架一个 App 收敛为**写一条清单 + 起它自己的容器**——清单是单一事实源（身份 + scope + ACL + 导航 + `serviceBaseUrl` + `exchangeTargets` + `embedCsp` + `deployDescriptor`，见 [§4.2](#42-manifest-字段全集)），其余配置一律派生。平台三件套（portal-api / web / reverse-proxy）镜像**不重建、不重启**；后端走 `<key>-server:8080` 约定 + 同源 `/svc/<key>`，前端上传 `dist` 同源托管。详见 [§7.5](#75-部署与路由约定独立后端如何接入网关app-integration) 与 [`goal/APP-INTEGRATION.md`](../goal/APP-INTEGRATION.md)。

平台管理员接口（平台租户控制台，`/api/console/market/*`，需平台管理员会话）：

```
GET    /api/console/market/listings           # 列出全部清单
POST   /api/console/market/listings           # 新建清单（draft）
GET    /api/console/market/listings/:id
PATCH  /api/console/market/listings/:id        # 更新（listingKey 不可改；改 dependencies 会做环检测）
POST   /api/console/market/listings/:id/status # draft | published | delisted
DELETE /api/console/market/listings/:id
```

租户管理员接口（浏览/安装/更新，`/api/admin/market/*`，需管理员会话）：

```
GET    /api/admin/market/listings             # 浏览已发布清单（带本租户安装状态 + 依赖关系）
GET    /api/admin/market/listings/:key
POST   /api/admin/market/install              # 安装（body { listingKey }，连带补装依赖）
POST   /api/admin/market/listings/:key/update # 同步到清单最新版本（仅刷新开发者字段，运营字段不动）
```

> 卸载/停用走应用管理：`POST /api/admin/apps/:appKey/status`（`uninstalled` / `disabled`，带依赖拦截，可 `force`）。

### 3.2 路线二：自助注册（自定义/本地应用）

不走市场，由**租户管理员**在「应用管理」后台直接注册一条 `apps`。适合本租户私有、不打算上架的应用。

```
POST   /api/admin/apps                       # 注册应用
PUT    /api/admin/apps/:appKey               # 更新（appKey 不可改）
POST   /api/admin/apps/:appKey/status        # active | disabled | uninstalled（带依赖拦截，可 force）
POST   /api/admin/apps/:appKey/secrets       # 创建 / 轮换 App Secret
GET    /api/admin/apps/:appKey/secrets       # 列出密钥（只含前缀，不含明文）
POST   /api/admin/apps/:appKey/secrets/:id/revoke  # 吊销密钥
```

> ⚠️ 两条路线的关键差异：**ACL Manifest（权限模型）和 `navItems`（导航项）只能由应用自身声明**（代码里的 manifest 注册表 / 市场清单），**不能**由租户管理员在注册表单里手填——`POST /api/admin/apps` 的请求体里没有这两个字段。所以一个走自助注册、且没有进 manifest 注册表的"裸应用"是没有 ACL Manifest 的。要让应用拥有完整权限模型与可贡献导航，请走市场（或把 manifest 登记进代码注册表，见 [§4.6](#46-manifest-生命周期)）。

### 3.3 App Secret

`micro` 应用走 iframe 握手时**不需要**自己持有 App Secret（宿主代为注入 TDT）。**服务端 OAuth**（[§6](#6-集成方式二服务端-oauthauthorization_code)）才需要它在 `/oauth/token` 认证。

- 市场安装的 `micro` 应用会**自动**创建一把 Secret（管理员可在应用详情看前缀、轮换）。
- 创建/轮换：`POST /api/admin/apps/:appKey/secrets`，返回的明文**只展示一次**，形如 `sk_<appKey>_<随机>`。
- 轮换（`{ "rotate": true }`）：旧密钥立即吊销，并**同时吊销该应用已签发的所有 TDT**（bump app version，见 [§9.3](#93-撤销机制四把钥匙)）。吊销单把密钥同理。

> 安全提醒：App Secret 等价于应用的口令，**只放服务端**，不要进前端 bundle、不要进 Git。

---

## 4. App Manifest 规范

★ **Manifest 是一个应用对平台的"自描述契约"**：它声明应用的形态、要申请的权限范围、要贡献的扩展点/导航、依赖哪些别的应用，以及——最关键——它的**权限模型（ACL Manifest）**。平台据此渲染入口、走授权门、做权限矩阵、隔离数据。

### 4.1 概念与快照模型

Manifest 的**权威载体是一条市场清单（marketplace listing）**。清单的字段分两类：

- **开发者字段**（developer fields）—— 应用的"出厂定义"，由开发者/平台维护。安装时**快照复制**进 `apps` 行。
- **运营字段**（operator fields）—— 由安装它的租户管理员调（可见性、置顶、启用哪些导航项、令牌交换白名单等），**不属于** Manifest。

安装就是把开发者字段从 `marketplace_listings` 一次性拷进该租户的 `apps` 行。之后清单升级，租户用「同步更新」重新拷开发者字段（运营字段保持不动）。

> 对"自助注册"的应用而言，开发者字段是管理员通过 `POST /api/admin/apps` 直接填的（但不含 ACL Manifest / navItems，见 [§3.2](#32-路线二自助注册自定义本地应用)）。

### 4.2 Manifest 字段全集

下表是开发者字段（即安装时会拷进 `apps` 的快照）。完整 schema 见 [`apps/api/src/modules/market/service.ts`](../apps/api/src/modules/market/service.ts) 的 `ListingInput` / `developerFields`，与 [`apps/api/src/modules/apps/index.ts`](../apps/api/src/modules/apps/index.ts) 的 `appInput`。

| 字段 | 类型 / 默认 | 说明 |
| --- | --- | --- |
| `listingKey` / `appKey` | string，`^[a-z0-9-]+$`，2–40 | 应用标识。清单的 `listingKey` 在安装时成为 `apps.contentOwner`、并作为 `appKey` 基名；**就是 TDT 的 `aud`**。不可改 |
| `name` / `tagline` / `desc` | string | 名称 / 一句话简介 / 详细描述 |
| `icon` / `color` / `logoUrl` / `cat` | string | 图标 / 主色 / Logo / 分类 |
| `type` | `micro`(默认) / `link` / `native` / `service` | 形态，见 [§4.3-a](#43-a-应用形态type) |
| `embedUrl` | string? | `micro` 应用被嵌入的 iframe 源 |
| `allowedOrigins` | string[] | 允许与宿主握手的源白名单（postMessage 来源校验） |
| `landingUrl` | string? | `link` 应用的落地页 |
| `nativeRoute` | string? | `native` 应用在 Portal 内的路由 |
| `scopes` | Scope[] | 应用**声明**需要的权限范围（[附录 A](#附录-ascope-目录)） |
| `extPoints` | string[] | 声明可贡献的扩展点（仅 `settings.section`；旧 `dashboard.widget` 已移除，改用 `dashboard.widgets`） |
| `dashboard.widgets` | DashboardWidgetDef[] | 声明应用可展示的 Dashboard Widget（[§13.1](#131-应用动态--dashboard-widgetgoalbi-dashboardmd)）；安装快照进 `apps.dashboardWidgets` |
| `navItems` | `{ id, label, icon, path }[]` | 贡献到 Portal 左侧导航的入口（由应用声明） |
| `dependencies` | listingKey[] | 依赖的其他清单（[§4.5](#45-依赖dependencies)） |
| `seatBased` | bool，默认 false | **席位制声明**（USAGE-2）。`true` ⇒ 只有被分配席位的成员能使用该 App，席位配额随套餐由平台配置。**仅 backed listing 可置 true**（须有 `serviceBaseUrl` 或 `deployDescriptor`，否则写时拒 `VALIDATION_FAILED`）——保证安装 `appKey === listingKey` 不漂移。安装时快照到 `apps.seat_based`；收口在门户单点（`visibleAppRows`/`mintForApp`），独立后端零改造。详见 [`goal/USAGE-2.md`](../goal/USAGE-2.md) |
| `aclManifest` | AclManifest \| null | **权限模型**（[§4.4](#44-acl-manifest权限-schema)）。`null` = 不声明 ACL。控制台清单 API 可直接收（写时校验形状），不再只能走代码注册表 |
| `scopeLabels` | Record<scope, i18n> | App 命名空间 scope 的同意页文案（[附录 A](#附录-ascope-目录)）。平台基础 scope 用内置文案，缺失降级显示原串 |
| `serviceBaseUrl` | string? | 独立后端可达地址（[§7.5](#75-部署与路由约定独立后端如何接入网关app-integration)）。生产同源 `/svc/<key>`；`null` = 按此约定。供宿主 `callService` 运行时解析 |
| `exchangeTargets` | listingKey[] | 本应用经令牌交换读取哪些 App（[§11](#11-跨应用调用令牌交换-token-exchange)）；安装时自动建立交换白名单 |
| `embedCsp` | `{ connectSrc?: string[] }`? | 同源托管下的 per-App CSP 例外（额外 `connect-src` 来源，如直传对象存储），[§7.5](#75-部署与路由约定独立后端如何接入网关app-integration) |
| `deployDescriptor` | DeployDescriptor \| null | 数据化按需部署描述（镜像 digest / 端口 / 命令 / 资源），**仅平台管理员**可写，[§7.5](#75-部署与路由约定独立后端如何接入网关app-integration) |
| `contentTypes` | ContentType[]（只读派生） | 应用用「内容管理」公共服务存储的数据结构（[§4.4-内容类型](#内容类型content-types)），代码定义，按 `listingKey` 归属 |
| `tdtTtl` | int，60–86400，默认 3600 | 签发 TDT 的有效期（秒） |
| `allowRefresh` / `refreshTtl` | bool / int，默认 true / 30 天 | 是否签发 refresh_token 及其有效期 |
| `allowedGrants` | GrantType[]，默认全开 | `authorization_code` / `token_exchange` / `refresh_token` |
| `webhookEvents` | WebhookEvent[]，默认全订阅 | 订阅的应用事件（[§13.5](#135-webhook-事件)） |
| `publisher` / `version` / `featured` / `sort` | —— | 发布者 / 版本（驱动"有更新"）/ 推荐位 / 排序 |

运营字段（安装后由租户管理员设，**非 Manifest**）：`visibility` / `showInCenter` / `pinned` / `enabledNavItemIds` / `webhookUrl` / `allowExchange` / `exchangeWhitelist`。

#### 4.3-a 应用形态（`type`）

| `type` | 形态 | 说明 |
| --- | --- | --- |
| `micro` | 沙箱 iframe | **最常见**。你的页面被嵌进 Portal，用 SDK 经 postMessage 握手、拿 TDT。见 [§5](#5-集成方式一嵌入式微应用iframe--sdk)。 |
| `link` | 外链 | Portal 只展示入口，点击在新标签打开你的站点（`landingUrl`）。无握手、无 TDT 注入。 |
| `native` | Portal 内置页 | 一等公民页面（如「租户管理」控制台），路由到 `nativeRoute`，**仅平台租户管理员可见**。普通应用用不到。 |
| `service` | 无头服务 | **无前端**的独立后端（如多模态解析）。不露入口卡片、不可打开，仅作为被调用方（令牌交换 / 服务态直调）存在；应用市场与应用中心对租户隐藏，平台控制台仍可治理。声明 `serviceBaseUrl`，`embedUrl` 恒为空。接入见 [§15](#15-外部服务类应用接入docker-镜像交付)。 |

### 4.4 ACL Manifest（权限 Schema）

ACL Manifest 是应用的**自描述权限模型**——它把应用的「页面」「操作」「数据范围」「内置角色模板」全部声明出来，平台据此：① 在「权限管理 › 角色矩阵」里渲染可授权项；② 把 `defaultForMember` 项授予 `member` 基线；③ 在 SDK 握手与令牌自省时算出"这个用户在这个应用里能做什么"，回灌给前端（UX 门）与你的后端（安全门）。

类型定义见 [`packages/shared/src/acl.ts`](../packages/shared/src/acl.ts)：

```ts
interface AclManifest {
  version: string;          // 版本（reconcile 基准）
  landingPageKey?: string;  // 应用可见性判定看这个页面（缺省取第一个 page）
  groups?: AclGroup[];      // 页面分组（矩阵分组）
  pages: AclPage[];
  actions: AclAction[];
  roleTemplates?: AclRoleTemplate[]; // 内置角色模板；管理员在角色管理"从模板克隆"出新角色（克隆后脱钩）
}

interface AclPage {
  key: string;               // 应用内稳定 id，进 PID。如 "projects"
  path: string;              // 路由 pattern，匹配运行时 ?r=。如 "/projects/:id"
  label: I18nText;           // 字符串或 { "zh-CN": "...", "en": "..." }
  parentKey?: string;        // 页面树（矩阵分组 + 前缀通配）
  navItemId?: string;        // 关联 navItems[].id：本页是该导航项的目标
  supportedScopes?: DataScope[]; // 该页提供哪些数据范围（缺省 ["all"]）
  defaultForMember?: boolean; // 默认授予内置 member 基线（reconcile 时对账，见 §4.6）
}

interface AclAction {
  key: string;               // 操作 id。如 "question.publish"
  label: I18nText;
  pageKey?: string;          // 该操作所属页面（缺省 = 应用级操作）
  supportedScopes?: DataScope[];
  dangerous?: boolean;       // UI：破坏性（红色 / 二次确认）
  defaultForMember?: boolean;
}

interface AclRoleTemplate {
  key: string;
  name: I18nText;
  desc?: I18nText;
  grants: { pid: string; condition?: { dataScope?: DataScope } }[]; // 纯授予，无 deny
}
```

**PID（权限标识）语法** —— `<appKey>:<kind>:<key>`，`kind ∈ { page, action }`。通配（由粗到细）：

```
<appKey>:*                整个应用
<appKey>:page:*           所有页面
<appKey>:action:*         所有操作
<appKey>:page:projects.*  页面子树（按 key 的点分层级做前缀）
<appKey>:page:projects    精确
```

**数据范围（DataScope，ABAC-lite）** —— `own` < `team` < `all`：

- `own` → 行的 `ownerUserId == 当前用户`；`team` → 行属于当前用户所在的某个组；`all` → 不做行级过滤。
- 应用在 `supportedScopes` 里声明每个页面/操作支持哪些范围，矩阵据此提供选项。
- 同一 PID 命中多条授予时，**最宽的范围胜出**（纯加法 RBAC：任一命中即放行）。

**模型要点（纯加法 RBAC + ABAC-lite，见 [`goal/ACL.md`](../goal/ACL.md)）**：角色只**授予**、从不拒绝；无角色继承；用户有效权限 = 其所有角色授予的**并集**；默认拒绝；租户 `admin` 角色 **bypass**（放行一切，永不参与解析）。

示例（研发项目管理 App，节选自 [`apps/api/src/modules/acl/manifests.ts`](../apps/api/src/modules/acl/manifests.ts)）：

```ts
const SPMS_MANIFEST: AclManifest = {
  version: "1.0.0",
  landingPageKey: "issues",
  groups: [{ key: "work", label: { "zh-CN": "工作", en: "Work" }, sort: 0 }],
  pages: [
    { key: "issues", path: "/issues", label: { "zh-CN": "Issues" }, navItemId: "nav-issues",
      parentKey: "work", supportedScopes: ["own", "team", "all"], defaultForMember: true },
    { key: "projects", path: "/projects", label: { "zh-CN": "项目" },
      supportedScopes: ["team", "all"] },
  ],
  actions: [
    { key: "issue.create", label: { "zh-CN": "新建 Issue" }, pageKey: "issues",
      supportedScopes: ["own", "team", "all"], defaultForMember: true },
    { key: "project.delete", label: { "zh-CN": "删除项目" }, pageKey: "projects",
      dangerous: true, supportedScopes: ["own", "all"] },
  ],
  roleTemplates: [
    { key: "spms-viewer", name: { "zh-CN": "研发·只读" }, grants: [{ pid: "spms:page:*" }] },
    { key: "spms-admin",  name: { "zh-CN": "研发·管理" }, grants: [{ pid: "spms:*" }] },
  ],
};
```

平台自身的页面/操作由保留的 **`portal` Manifest** 描述（`appKey = "portal"`），它永不落库，由聚合器在算权限时注入。

#### 内容类型（content types）

如果应用用 Portal 的「内容管理」公共服务（headless CMS）落数据，它声明一组**内容类型**（字段集合 + 记录范围）。内容类型是**代码定义**的（架构层），按 `listingKey` 归属（`contentOwner`），清单上**只读展示**给安装评审看，**不**作为可改配置。字段类型见 [附录 A 末](#附录-ascope-目录) 引用的 `FIELD_TYPES`；记录范围 `user`（每用户私有）/ `tenant`（租户共享）。运行时通过 `sdk.content.*` 或 `/api/v1/content/*` 读写（[§13](#13-扩展点与公共服务)）。

开发者真正配置内容类型的位置是 [`apps/api/src/modules/content/schemas.ts`](../apps/api/src/modules/content/schemas.ts) 的 `CONTENT_SCHEMAS`。`owner` 必须等于市场清单的 `listingKey`（也是安装快照里的 `contentOwner`），Portal 会把这些 schema 派生为清单详情里的 `contentTypes`：

```ts
// apps/api/src/modules/content/schemas.ts
def("todo", "task", "待办任务", "user", [
  { key: "title", label: "标题", type: "text", required: true, maxLength: 200 },
  { key: "due", label: "到期时间", type: "datetime" },
  { key: "priority", label: "优先级", type: "enum", options: ["low", "med", "high"], default: "med" },
  { key: "done", label: "已完成", type: "boolean", default: false },
]);
```

> 配套要求：应用清单要声明 `content.read` / `content.write`，否则即使 schema 存在也不能通过 Open API 读写内容。

### 4.5 依赖（dependencies）

清单可声明它依赖的其他清单（`dependencies: listingKey[]`）。规则：

- **环检测**：平台管理员编辑依赖时，整张依赖图必须无环（白/灰/黑三色 DFS），且不能自依赖、不能引用不存在的清单。
- **安装即补装**：安装一个清单会按**拓扑序**（依赖在前、目标在后）自动补装所有缺失依赖；已装的跳过。
- **卸载拦截**：当还有已安装应用依赖某应用时，停用/卸载它会被 `DEPENDENCY_REQUIRED` 拦截，除非传 `force:true`。

示例：`知识库` 依赖 `files`（文件管理）—— 安装知识库会先确保 files 已装。

### 4.6 Manifest 生命周期

```
①代码注册表                ②市场清单                 ③安装快照              ④运行时注入
manifests.ts          marketplace_listings        apps 行            SDK init.acl / introspection
(MANIFEST_REGISTRY)  ─seed 盖章─► .aclManifest ─install 快照─► apps.aclManifest ─解析每用户有效权限─►
                                  .navItems / .scopes / ...        .navItems / ...    前端 UX 门 + 后端安全门
```

1. **代码注册表**：ACL Manifest 在 [`apps/api/src/modules/acl/manifests.ts`](../apps/api/src/modules/acl/manifests.ts) 里按 `listingKey` 集中定义（单一事实源），是开发者真正"写 Manifest"的地方。
2. **盖章到清单**：`db:seed` 把 `manifestForListing(listingKey)` 盖到 `marketplace_listings.aclManifest`。
3. **安装快照**：安装时 `developerFields` 把 `aclManifest`（优先取清单列，回退取代码注册表）连同 `navItems`/`scopes`/`extPoints` 等一起拷进 `apps` 行。
4. **运行时**：Portal 用 `apps.aclManifest` + 用户的角色授予，算出该用户对该应用的**有效权限集**，分别注入：
   - **SDK 握手**（`init.acl`）→ 前端做 UX 门（隐藏按钮/页面）。
   - **令牌自省**（introspection 的 `permissions`/`bypass`/`groups`）→ 你的独立后端做安全门。

> 升级 Manifest = 改 `manifests.ts` 的 `version` + 内容 → 重新 seed/发布清单 → 租户「同步更新」。
>
> `member` 基线的 `defaultForMember` 授予由 `reconcileTenantRoles(tenantId)` 对账（盘点各已装应用 manifest 的 `defaultForMember` 项 → 幂等补种到 member 角色 → bump aclEpoch）。它**目前在 `db:seed` 时对所有租户执行**；运行时安装不会自动补种 member 基线，管理员可在角色矩阵手动调整。

---

## 5. 集成方式一：嵌入式微应用（iframe + SDK）

这是接入 Portal 的**推荐姿势**。你的应用是一个普通 Web 页面，被 Portal 以沙箱 iframe 嵌入；通过 `@xgent/portal-sdk` 与宿主握手，**无需自己实现登录、无需持有 App Secret**——宿主会把当前用户在当前租户下的 TDT 注入给你。

参考实现：[`apps/sample-app/src/main.ts`](../apps/sample-app/src/main.ts)（端到端跑通 SDK 的每个能力）。

### 5.1 握手与令牌注入流程

```
 Portal 宿主页 (MicroAppHost)              你的 iframe 应用 (portal-sdk)
        │                                         │
        │ ① 渲染前先过 consent 门（见 §10）         │
        │────────────── 挂载 iframe ──────────────►│
        │                                         │ ② createPortalClient()
        │◄──────── req: ready ────────────────────│
        │ ③ event: init {appKey,apiBase,host,     │
        │     theme,locale,user,route,fullscreen, │
        │     acl} ───────────────────────────────►│ ready() 返回 InitPayload
        │                                         │
        │◄──────── req: getToken {scopes?} ───────│ ④ getToken()
        │ ⑤ POST /api/tokens/mint（会话态）         │
        │    → {access_token, expires_in,...} ────►│ 拿到 TDT（SDK 自动缓存）
        │                                         │
        │                                         │ ⑥ 用 TDT 调 Open API /api/v1/*
        │◄──── event: theme/locale/route ─────────│   或 sdk.callService 调独立后端
```

要点：
- 宿主只接受**来源在 `embedUrl` + `allowedOrigins` 白名单内**的 postMessage，否则忽略。
- `getToken` 由宿主用**用户会话**调 `POST /api/tokens/mint`，应用全程拿不到 App Secret。
- TDT 在 SDK 内**自动缓存**，到期前 30s 才重新申请。
- iframe 用 `sandbox="allow-scripts allow-same-origin allow-forms allow-popups ..."` 加固，`referrerPolicy=no-referrer`。

### 5.2 SDK 速查

```ts
import { createPortalClient, type InitPayload } from "@xgent/portal-sdk";

const sdk = createPortalClient();          // 必须运行在 Portal 的 iframe 内
const init: InitPayload = await sdk.ready(); // 握手，拿 appKey/apiBase/host/theme/locale/user/route/acl...

// —— 身份与令牌 ——
const token = await sdk.getToken();          // 原始 TDT（一般不用，下面便捷方法已内置）
const me    = await sdk.userinfo();          // GET /api/v1/userinfo

// —— 权限（ACL）UX 门：真正的安全门在后端 ——
if (sdk.acl.can("myapp:action:item.delete")) renderDeleteButton();
const scope = sdk.acl.scope("myapp:page:items"); // "own" | "team" | "all"

// —— 写入 Portal ——
await sdk.notify({ title: "标题", body: "正文", type: "示例", link: "/app/your-app" });

// —— 应用动态 / Dashboard Widget（需 Manifest dashboard.widgets[] 声明 + widget.write，见 §13.1）——
await sdk.dashboard.putWidgetData("activity", {
  title: "本应用动态",
  items: [{ title: "新增一条动态", body: "实时数据", at: new Date().toISOString(), link: "/app/your-app" }],
}, { staleAfterSec: 3600 });
await sdk.dashboard.clearWidgetData("activity");

// —— 内容服务（headless CMS，需 content.read/write + 声明内容类型）——
const types = await sdk.content.types();
const rows  = await sdk.content.list("note", { limit: 20 });
const row   = await sdk.content.create("note", { title: "hi", done: false });
await sdk.content.update("note", row.id, { done: true });
await sdk.content.remove("note", row.id);

// —— 计划任务（公共调度服务，需 scheduler.read/write，见 §13.4）——
const task = await sdk.scheduler.create({ name: "每日提醒", cron: "0 9 * * *", params: { title: "..." } });
await sdk.scheduler.update(task.id, { status: "paused" });
await sdk.scheduler.cancel(task.id);

// —— 调用你自己的独立后端（宿主代理，零跨域，见 §7.4）——
const data = await sdk.callService("myapp", "/api/things", { method: "GET" });
// sdk.files.* 就是 callService("files", …) 的薄封装（PLAN-4 文件管理）：
const f = await sdk.files.upload(file, { spaceId });
const { items } = await sdk.files.list({ spaceId });

// —— 与宿主的 UI 协作 ——
sdk.resize(document.body.scrollHeight + 8);  // iframe 高度自适应
sdk.navigate("/app/another-app");            // 让宿主跳转到 Portal 内某路由
sdk.routeSync("/detail");                    // 把内部路由同步到地址栏 ?r=/detail（可分享/刷新还原）
sdk.setDirty(true);                          // 声明有未保存更改 → 宿主拦截离开
sdk.requestFullscreen(true);                 // 请求全屏

// —— 订阅宿主事件 ——
sdk.onTheme((t) => applyTheme(t));           // "light" | "dark"
sdk.onLocale((l) => applyLocale(l));         // "zh-CN" | "en" | "zh-TW"
sdk.onRoute((p) => renderRoute(p));          // 地址栏 ?r= 变化（如浏览器后退）
sdk.onFullscreen((on) => {/* ... */});
```

`InitPayload` 字段：

```ts
interface InitPayload {
  appKey: string;
  apiBase: string;                    // Open API 绝对地址，拼 /api/v1/* 用
  host: string;                       // Portal web 源，用于拼可分享深链
  theme: "light" | "dark";
  locale: string;                     // "zh-CN" | "en" | "zh-TW"
  user: { id: string; name: string } | null;
  route: string | null;               // 深链：地址栏 ?r= 携带的内部路由，用于还原
  fullscreen: boolean;
  acl?: AclInit;                       // 本应用的有效权限：{ bypass, permissions:[{pid,scope}], groups }
}
```

### 5.3 最小可用例子

```ts
import { createPortalClient } from "@xgent/portal-sdk";

async function main() {
  if (window.parent === window) {
    document.body.textContent = "请在 XGENT Portal 内打开本应用（需要宿主握手）。";
    return;
  }
  const sdk = createPortalClient();
  const init = await sdk.ready();
  const user = await sdk.userinfo().catch(() => null);

  document.body.innerHTML = `当前用户：${user?.name ?? "—"}`;
  sdk.onTheme((t) => document.documentElement.dataset.theme = t);
  sdk.resize(document.body.scrollHeight + 8);
}
main();
```

> 部署提醒：浏览器里 iframe 应用**直接** `fetch` Open API 受 **CORS** 限制——后端 CORS 白名单是 `PORTAL_BASE_URL` + `SAMPLE_APP_URL` + 各内置 App 前端源（`FILES_APP_URL`/`SPMS_APP_URL`/…）。生产部署时需把你应用的源加入允许列表（环境变量），或更省事地用 `sdk.callService`（见 [§7.4](#74-宿主代理sdkcallservice零跨域)）让**宿主代你转发**，iframe 完全不碰跨域。

---

## 6. 集成方式二：服务端 OAuth（authorization_code）

当你的应用有**自己的后端**、希望在服务端持有 TDT（而非靠 iframe 握手），走标准 OAuth2 授权码。**授权码（code）有两种取法**，第二步换码端点 `/oauth/token` 完全相同：

- **[§6.1 会话态取码](#61-会话态取码嵌入--同源页面)** —— 用户已持 Portal 会话、你的页面与 Portal **同源**（或由宿主代发请求）时用它：前端 `POST /api/tokens/authorize` 直接拿 code，不重定向。
- **[§6.2 重定向式取码](#62-重定向式授权入口-oauthauthorize独立域名站点)** ★ —— 你的应用是**独立域名站点**（不嵌入 Portal、与 Portal 跨源、浏览器里没有 Portal 会话 Cookie）时用它：把浏览器**重定向**到 `GET /oauth/authorize`，Portal 用自己的第一方会话 Cookie 发码后再 302 回你的回调地址。这是支持「非嵌入式独立应用」的标准姿势。

### 6.1 会话态取码（嵌入 / 同源页面）

```
① 用户在 Portal 已登录（持有会话）
② 你的页面（或宿主）以会话态请求授权码：
   POST /api/tokens/authorize         （会话认证）
   body: { appKey, scopes? }
   → 200 { ok:true, data: { code } }   # 60s、一次性

③ 你的后端用 code 换 TDT：
   POST /oauth/token
   body(x-www-form-urlencoded 或 json):
     grant_type   = authorization_code
     code         = <上一步的 code>
     client_id    = <appKey>
     client_secret= <App Secret>
   → { access_token, token_type:"Bearer", expires_in, scope, refresh_token? }
```

- `refresh_token` 仅当应用 `allowRefresh=true` 时返回，有效期 `refreshTtl`（默认 30 天）。
- 续期：`POST /oauth/token { grant_type:"refresh_token", refresh_token, client_id, client_secret }`（refresh token **一次性、用后轮换**）。
- 主动吊销：`POST /oauth/revoke { token, client_id, client_secret }`（按 jti 吊销，幂等）。

`grant_type` 受应用的 `allowedGrants` 约束（`authorization_code` / `token_exchange` / `refresh_token`），未启用的会得到 `GRANT_NOT_ALLOWED`。

> `authorization_code` 与 `refresh_token` 路径在签发时**隐式记录 consent**（按签发的 scope）；只有 iframe 宿主注入（`/api/tokens/mint`）走**显式 consent 门**（见 [§10](#10-授权与同意consent)）。

### 6.2 重定向式授权入口 `/oauth/authorize`（独立域名站点）

★ 当你的应用部署在**自己的域名**、不嵌入 Portal 主界面时，浏览器里没有 Portal 的会话 Cookie，§6.1 的会话态 `POST /api/tokens/authorize` 用不了。改走**重定向式授权码**：把用户浏览器**整页重定向**到 Portal 的 `GET /oauth/authorize`——这是一次**顶层导航**到 Portal 自己的源，所以 Portal 的第一方会话 Cookie（`SameSite=Lax`）会随请求带上，Portal 据此发码并 302 回你登记的回调地址。**无需** CORS：authorize 是浏览器导航、`/oauth/token` 是你后端到 Portal 的服务器间调用，都不经过浏览器跨域 `fetch`。

```
① 浏览器重定向到 Portal 授权端点（你的前端发起整页跳转）：
   GET /oauth/authorize?
       response_type=code
       client_id=<appKey>
       redirect_uri=<你登记的回调，需精确匹配>
       scope=<空格分隔，可省略=取应用声明的全部 scope>
       state=<你的 CSRF 随机串，原样回传>
       code_challenge=<PKCE，公开客户端必带>          # 见下「客户端类型」
       code_challenge_method=S256

   · 用户未登录 Portal → Portal 302 到 /login?return=<原 authorize 链接>；
     用户用任意方式登录（含 2FA）后，自动 302 回 /oauth/authorize 继续。前端无感。
   · 用户已登录 → Portal 直接发码（隐式同意，首次也不打断）。

② Portal 302 回你的回调（code 60s、一次性）：
   <redirect_uri>?code=<code>&state=<原样回传，请校验>

③ 你的后端（或 SPA）用 code 换 TDT —— 端点同 §6.1：
   POST /oauth/token  body(x-www-form-urlencoded 或 json):
     grant_type    = authorization_code
     code          = <上一步的 code>
     client_id     = <appKey>
     redirect_uri  = <必须与第①步完全一致>
     # 二选一或都带：
     client_secret = <App Secret>        # 机密客户端
     code_verifier = <PKCE 原值>          # 公开客户端（SPA）
   → { access_token, token_type:"Bearer", expires_in, scope, refresh_token? }
```

**回调白名单（`redirect_uri`）** —— `redirect_uri` 必须**精确等于**应用登记的某个回调地址，否则 Portal 在自己页面报错、**绝不**重定向到未登记地址（防开放重定向 / 授权码截获）。登记位置：自助注册（[§3.2](#32-路线二自助注册自定义本地应用)）的应用表单「回调地址 redirectUrls」字段（`appInput.redirectUrls`，每行一个）。

> 现状：`redirectUrls` 目前**仅自助注册的应用**支持；市场清单（marketplace listing）的开发者字段暂未携带 `redirectUrls`，走市场分发的独立站点应用如需此能力请联系平台补齐（一个加列 + 快照的增量改动）。

**客户端类型（决定用什么换码）** —— 两者都支持：

| 类型 | 第①步 | 第③步换码 |
| --- | --- | --- |
| 机密客户端（有后端，持 App Secret） | `code_challenge` 可省略 | 必带 `client_secret`（可同时带 PKCE） |
| 公开客户端（纯前端 SPA，无密钥） | **必带** `code_challenge`（S256） | 必带 `code_verifier`，不发 `client_secret` |

- 只支持 **PKCE S256**（`code_challenge_method=plain` 会被拒）。公开客户端若省略 `code_challenge`，换码时又拿不出 `client_secret`，将无法完成——即无 PKCE 降级旁路。
- **租户绑定**：发码绑定到**用户当前选中的租户**（与 §6.1 一致）。同一应用需登记在该租户下（`client_id=<appKey>` 按当前租户解析）。用户属于多个租户时，让其在 Portal 顶栏切到目标租户后再发起授权。
- **同意**：登录用户首次授权**隐式同意**、直接发码，不弹同意页（与 §6.1 的 `authorization_code` 行为一致，见 [§10](#10-授权与同意consent)）；用户可事后在「个人 › 已授权应用」撤销。
- `refresh_token` / 续期 / 吊销 / `allowedGrants` 约束均同 §6.1。

**错误形态** —— authorize 端点是浏览器导航，不返业务信封：

| 情况 | 形态 |
| --- | --- |
| `client_id` 不存在 / `redirect_uri` 未登记 / 缺 `response_type=code` | **不**重定向；Portal 自身页面 `400` + 错误说明（HTML） |
| `redirect_uri` 已登记但其它可恢复错误（如该应用未启用 `authorization_code`、`code_challenge_method` 非 S256） | 302 回 `<redirect_uri>?error=<code>&state=...`（标准 OAuth 错误回传） |
| 换码 `/oauth/token` 失败 | 同 §6.1，走 Portal 统一信封 `200 { ok:false, error:{ code } }`（**非** RFC 的 `400 {error}` 形态，见 [§8.1](#81-统一响应信封)）；成功则返裸 OAuth token JSON |

---

## 7. 集成方式三：独立后端应用（introspection + 服务账号）

当你的应用是一个**独立的资源服务器**（有自己的进程、自己的数据库），不复用 Portal 的进程内鉴权，但仍要消费 Portal 签发的 TDT —— 这是**最重的、也是大型业务 App 的标准模式**（文件管理、研发项目管理、学校管理、题库都用它）。

参考实现：[`apps/spms-server/src/lib/gate.ts`](../apps/spms-server/src/lib/gate.ts)、[`apps/spms-server/src/lib/identity.ts`](../apps/spms-server/src/lib/identity.ts)（files-server 同构）。

### 7.1 你的后端如何验 TDT：令牌自省（introspection）

你的服务**不验证 JWT 签名**（HS256 密钥只在 Portal），而是把 bearer TDT 发给 Portal 的自省端点换取已解析的声明：

```
POST /api/tokens/introspect
  Authorization: Basic base64(<saClientId>:<saSecret>)   # 服务账号凭证
  （dev 通道：仅当 DEV_MOCK_OAUTH=true 时可改用 x-resource-key，且仅接受
   FILES_RESOURCE_KEY 一把 key；生产 DEV_MOCK_OAUTH=false 一律拒绝该 header，必须走 Basic）
  body: { token: "<待验证的 TDT>" }
→ 200 { ok: true, data: {
    active, kind, aud, listingKey, azp, tenant_id, user_id, scopes, role,
    bypass, groups, permissions:[{pid,scope}], aclStamp, exp
  } }
```

> ⚠️ 自省响应同样走统一信封（[§8.1](#81-统一响应信封)）：**声明在 `data` 里**。解析请写 `claims = body.data ?? body`（`gate.ts` 同款）——直接读顶层的 `active` 会把所有有效 TDT 误判为无效（外部接入真实踩过）。`azp` 标识经令牌交换/服务账号**代表行事的来源应用**（无则 `null`），用于出处归因。

要点（见 `gate.ts`）：
1. 验证一个**无效/已撤销** TDT 也是**成功**的自省，只是返回 `active:false`（不要把它当传输错误）。
2. 自省结果**缓存到 `exp` 前**（上限 60s），减少往返。
3. 拿到声明后自己做四道闸：
   - **身份**：`(listingKey ?? aud) === 你的 listingKey`（否则不是发给你的令牌）。自省补了 **`listingKey`**（= 应用的**稳定身份** / `contentOwner`）；`aud` 是**安装态 appKey**。独立后端 App 安装时**禁止 `appKey` 漂移**（同租户重复安装直接 `ALREADY_INSTALLED`，不生成 `-2`），所以二者相等——但**请按 `listingKey` 鉴权**，对"安装态 appKey 是什么"透明（APP-INTEGRATION §4.0）。
   - 所需 `scope ∈ scopes`（否则 `INSUFFICIENT_SCOPE` / 403）；
   - 结构性管理操作看 `role === "admin"`（`requireAdmin`，角色来自 Portal 成员关系，非 TDT claim）；
   - 细粒度操作看 ACL：`bypass || permissions 命中目标 PID`（`requirePerm`，[§12](#12-权限管理acl运行时)）。
4. **租户隔离**：把每条查询都按 `claims.tenant_id` 收口。
5. 限流由你自己做（spms-server 按 `(aud, tenant)` 每分钟固定窗口）。

> 安全基线：独立后端必须校验 `(listingKey ?? aud) === 你的 listingKey`。宿主代理 `sdk.callService` 只是转发便利层，跨应用隔离的权威安全闸在资源服务器自己这道身份校验。

批量权限检查（可选）：`POST /api/v1/acl/check { token, pids:[...] }`（同样的服务账号鉴权）→ `{ active, results:[{pid,allowed,scope}] }`。

### 7.2 服务账号（M2M）

自省/批量检查的调用方要先有一个**服务账号**（service account），由平台管理员在控制台创建（`/api/console/service-accounts`）：

```
GET    /api/console/service-accounts
POST   /api/console/service-accounts            # { name, clientId, capabilities[], scopes[] }
PATCH  /api/console/service-accounts/:id         # 改名/启停/能力/scope
POST   /api/console/service-accounts/:id/rotate  # 轮换密钥（明文只显示一次）
POST   /api/console/service-accounts/:id/revoke
DELETE /api/console/service-accounts/:id
```

能力（`capabilities`）：

| 能力 | 含义 |
| --- | --- |
| `token.introspect` | 可调自省端点（验证门户签发的 TDT）—— 独立后端验 TDT 必需 |
| `client_credentials` | 可自助签发**服务态 TDT**（见 §7.3） |

服务账号是平台级身份，用 `clientId:secret` 走 HTTP Basic 认证。`token.introspect` 是验令牌能力，不按租户限制。`client_credentials`（主动签发服务态 TDT）则受**租户访问策略**约束（M3 已实施）：

- `tenantAccess = "all"`：可为任意租户签发（高风险——单密钥泄露横跨所有租户）。
- `tenantAccess = "allowlist"`：仅能为 `allowedTenantIds` 名单内的租户签发，否则 `TENANT_NOT_ALLOWED`。
- 新建具备 `client_credentials` 的账号**必须显式选择**策略（控制台默认指向更安全的 `allowlist`，选 `all` 会弹高风险提示）。**PATCH 给既有账号新增 `client_credentials` 能力时同样必须显式带上 `tenantAccess`**，否则 `VALIDATION_FAILED`——避免静默沿用旧默认 `all`。既有账号迁移为 `all`，行为不变。

### 7.3 服务态 TDT（client_credentials）

当你的后端要**主动**调租户级 Open API（无用户上下文，如批处理/系统消息），用 `client_credentials` 自助签发服务态 TDT：

```
POST /api/tokens/service
  Authorization: Basic base64(<clientId>:<secret>)
  body: { grant_type: "client_credentials", tenant_id: "...", scope?: "a b c" }
→ 200 { access_token, token_type:"Bearer", expires_in, scope }
```

- 需要服务账号具备 `client_credentials` 能力。
- `tenant_id` 受服务账号的租户访问策略约束（§7.2）：`allowlist` 账号传入名单外的租户 → `TENANT_NOT_ALLOWED`。
- 签发的 scope = `请求 scope ∩ 服务账号 scopes`（不传则取服务账号全部），**绝不扩张**。
- 产物是 `kind:"service"` 的 TDT，**无 `user_id`**，整体绕过用户 ACL（仅受 scope 约束）。

### 7.4 宿主代理（`sdk.callService`，零跨域）

你的 iframe 前端要调你的独立后端，最省事的是**让宿主代转发**，而不是 iframe 直接跨域 `fetch`：

```ts
const data = await sdk.callService("myapp", "/api/things", { method: "POST", body: JSON.stringify({...}) });
```

宿主（Portal web 源）收到后：① 为该应用铸/复用一个 host-proxy TDT；② 用它带上 `Authorization: Bearer` 去 `fetch` 该服务；③ 把响应回传 iframe。于是你的独立后端**只需信任 Portal web 源一个跨域来源**，iframe 完全不碰 CORS。401/403 时宿主会自动重铸令牌重试一次。

宿主**运行时**解析服务地址（APP-INTEGRATION §4.1）：`callService(name, …)` 按 `name`（= `listingKey`）从 `GET /api/apps/:key` 取 `serviceBaseUrl`（生产默认同源 `/svc/<key>`），**不再**有烘焙进 web 包的 `SERVICE_REGISTRY` / `VITE_*_SERVER_BASE`——所以新增独立后端无需重建门户前端。`sdk.files.*` 就是 `callService("files", …)` 的薄封装。

> `callService` 不替你的后端做授权。新增独立后端时，必须在资源服务器里按 §7.1 校验 `(listingKey ?? aud) === 你的 listingKey` 和所需 scope；不要只因为请求来自 Portal web 源就信任它。

> **示例（文件管理）**：管理员在 Portal 配置 files 应用的存储桶/回调时，Portal 用 `GET /api/admin/apps/:appKey/files-token` 铸一把短期**管理员 TDT**（含 `files.*` scope，仅声明了 files 作用域的应用可用）直连 files-server——不走 consent 门（属管理配置，非用户数据访问），Portal 也永不经手 S3 凭证。其它独立后端如需类似的"管理控制面直连"，须自行参照新增端点。

### 7.5 部署与路由约定（独立后端如何接入网关，APP-INTEGRATION）

目标：新增一个独立后端 App = **写一条清单 + 起它自己的容器**，平台三件套（portal-api / web / reverse-proxy）镜像**不重建、不重启**。约定如下（详见 [`goal/APP-INTEGRATION.md`](../goal/APP-INTEGRATION.md)）：

- **统一内网端口 `8080`**：你的后端容器**监听 8080**（端口可由 env 配，容器里设成 8080）。反向代理用**一条通用规则**把 `/svc/<key>/*` 转发到 `<key>-server:8080`（命名约定 `<listingKey>-server`），所以网关对新 App 零改动、零重载。
- **网关只当哑路由**：四道闸（§7.1）在你后端自己实现；网关另加两道收口——已上架/已部署 key 的**白名单 map**（未注册 = 路由不可达）+ App 容器**网络隔离**。
- **前端同源托管**：你的前端产物（`dist`，`index.html` 在压缩包根）由平台管理员上传（`POST /api/console/market/listings/:id/frontend`，`.tar.gz`），解压到共享卷,反代以 `/apps/<key>/*` **同源**服务——CSP 维持 `frame-src 'self'`，无需动态 CSP。个别 App 需额外 `connect-src`（如直传对象存储）由清单 `embedCsp` 声明。内置 App 仍可把 `dist` 烘焙进镜像，两条路并存。
- **按需部署（可选）**：清单可带 `deployDescriptor`（镜像/端口/资源/env，**仅平台管理员**填，参数化、不拼自由 shell），deploy-controller 据此把后端容器**用时才起、闲时缩零**。配置 App 的 DB **由 App 自己在启动时迁移**（门户不替它跑 migrate/bootstrap）。生产是 K8s（按描述模板化创建 Deployment/Service，固定 securityContext/资源上限、镜像认 digest）；过渡期第三方后端可"常驻 + 手动登记"（同[知识库 devkit](知识库后台接入与本地联调指南.md)）。
- **外部镜像 App 不走按需部署**：deploy-controller 只会编排 monorepo 内建后端（`DEPLOYABLE_APP_SERVICES` 注册表 + config-added `deployDescriptor`）。服务端代码不在本 repo、以独立 Docker 镜像交付的应用（知识库、多模态解析…）生产**常驻**运行（独立 Deployment / compose profile，可多副本），注册与交付契约见 [§15](#15-外部服务类应用接入docker-镜像交付)。
- **本地 dev**：每个后端仍各用自己的端口（4100–4600，见 [§14](#14-本地开发与调试)），统一 8080 只在容器/生产。

---

## 8. Portal Open API v1

应用拿到 TDT 后，用 `Authorization: Bearer <TDT>` 调用 Open API。基址是握手时拿到的 `apiBase`（本地 `http://localhost:3000`），路径前缀 `/api/v1`。

> **可视化文档（API-DOCS）**：租户管理员在门户「管理 › API 文档」（`/admin/api-docs`，需 `portal:page:admin.api-docs`）可浏览本页所述**全部公共契约**的交互式 OpenAPI 视图（三语，只读，禁 Try-it），并下载原始 `OpenAPI JSON`。该视图与本 Markdown 是同一份白名单事实源——包含用户态 Open API、OAuth/同源授权码流程，以及需平台开通服务账号的受限服务态契约（令牌自省 / ACL 校验 / client_credentials / 用量与席位上报 / 目录建号，逐项标注 token 类型、所需 scope 与前置条件，如 `directory.provision` 的 `azp=sms` 硬门）。独立业务 App 各自的 `/svc/<key>/docs` 不聚合进本视图。

### 8.1 统一响应信封

遵循 [`CLAUDE.md`](../CLAUDE.md)：**业务状态不走 HTTP 状态码**。

```jsonc
// 成功
{ "ok": true, "data": { /* ... */ } }
// 业务失败（HTTP 仍是 200）
{ "ok": false, "error": { "code": "INSUFFICIENT_SCOPE", "message": "需要 scope: ...", "details": { } } }
```

只有**传输/认证/路由**层故障才用 HTTP 非 200：

| HTTP | 含义 |
| --- | --- |
| `400` | 参数结构不正确（`VALIDATION_FAILED`） |
| `401` | TDT 缺失 / 无效 / 过期 / 已撤销（`UNAUTHENTICATED` / `INVALID_TOKEN` / `TOKEN_EXPIRED` / `TOKEN_REVOKED`） |
| `403` | scope 不足（`INSUFFICIENT_SCOPE`）/ 权限不足（`INSUFFICIENT_PERMISSION`）/ 服务账号无能力（`FORBIDDEN`） |
| `404` | 路由不存在（注意：**数据不存在**不是 404，而是 `200 + data:null` 或业务错误码） |
| `429` | 触发限流（`RATE_LIMITED`） |
| `5xx` | 真服务故障 |

SDK 已封装：非 `ok` 时把 `error` 作为异常抛出，你直接 `try/catch` 即可。

### 8.2 端点一览

每个端点都：① 校验 TDT；② 校验所需 scope；③ 按 `(appKey, tenant)` 限流；④ 落审计。

| 方法 + 路径 | 所需 scope | 说明 |
| --- | --- | --- |
| `GET /api/v1/userinfo` | `userinfo.read` | 当前 TDT 用户的基本资料 + 本次 scopes |
| `GET /api/v1/directory/users` | `directory.read`（+ `directory.email.read` 才含邮箱） | 当前租户成员的基础资料（id/name/role）。**默认不含邮箱**（PII）；仅当 TDT 还持 `directory.email.read` 时才返回 `email` 字段（M5） |
| `POST /api/v1/notifications` | `notification.send`（跨用户需 `notification.send.others`） | 默认给**当前用户**发通知（自投）。传 `userId≠自己` 给本租户另一位活跃成员需额外持 `notification.send.others`（M2，跨租户拒绝），并受限频 + 强出处（收件人看到「由 <发送者> 经 <App>」）约束。`link` 必须是站内深链（M1，外站/带协议一律 `VALIDATION_FAILED`）。`title/body/type` 有长度上限。落库 + 实时推送 + 按偏好发邮件 |
| `PUT /api/v1/dashboard/widgets/:widgetKey/data` | `widget.write` | 推送某个 Widget 的应用动态数据（应用须在 Manifest `dashboard.widgets[]` 声明该 key，否则 `WIDGET_NOT_DECLARED`）。按用户、upsert |
| `DELETE /api/v1/dashboard/widgets/:widgetKey/data` | `widget.write` | 清除该 Widget 的数据 |
| `POST /api/v1/audit` | `audit.write` | 独立后端把关键业务操作写入平台统一审计日志（设置 › 审计），建议记录 `event/object/detail/result/diff` |
| `GET /api/v1/settings/me` | `settings.read` | 读当前用户偏好 |
| `PUT /api/v1/settings/me` | `settings.write` | 写当前用户偏好 |
| `GET /api/v1/content-types` | `content.read` | 列出本应用声明的内容类型 |
| `GET /api/v1/content/:type` | `content.read` | 列出某内容类型的记录 |
| `POST /api/v1/content/:type` | `content.write` | 新建一条记录 |
| `GET /api/v1/content/:type/:id` | `content.read` | 读一条记录 |
| `PATCH /api/v1/content/:type/:id` | `content.write` | 改一条记录 |
| `DELETE /api/v1/content/:type/:id` | `content.write` | 删一条记录 |
| `GET /api/v1/scheduler/tasks` | `scheduler.read` | 列出本应用为当前用户创建的计划任务 |
| `POST /api/v1/scheduler/tasks` | `scheduler.write` | 创建计划任务（见 [§13.4](#134-调度服务scheduler)） |
| `PATCH /api/v1/scheduler/tasks/:id` | `scheduler.write` | 改名/改 cron/暂停恢复（须本应用为该用户所建） |
| `DELETE /api/v1/scheduler/tasks/:id` | `scheduler.write` | 取消 |

服务账号专用（HTTP Basic，非 TDT；见 [§7](#7-集成方式三独立后端应用introspection--服务账号)）：

| 方法 + 路径 | 能力 | 说明 |
| --- | --- | --- |
| `POST /api/tokens/introspect` | `token.introspect` | 验 TDT，返回解析后的声明 + ACL |
| `POST /api/v1/acl/check` | `token.introspect` | 批量 PID 权限检查 |
| `POST /api/tokens/service` | `client_credentials` | 自助签发服务态 TDT |

> 用户态公共服务基本是**自作用域**的：应用代表 **TDT 里的那个用户**操作他自己的数据（用户私有内容类型、自己的计划任务、自己的设置）。这些端点都要求用户上下文（`userGate`），服务态令牌不可调用（`directory.read` 例外，它不要求用户）。唯一的跨用户能力：通知可向本租户的另一位活跃成员投递（传 `userId`），且需 `notification.send.others` scope（M2，跨租户拒绝、限频、强出处）。

请求示例：

```bash
curl https://<apiBase>/api/v1/userinfo -H "Authorization: Bearer <TDT>"
# → { "ok": true, "data": { "id":"...", "name":"...", "nickname":"...", "email":"...", "avatarUrl":"...", "tenantId":"...", "role":"admin|user", "scopes":[...] } }

curl -X POST https://<apiBase>/api/v1/notifications \
  -H "Authorization: Bearer <TDT>" -H "content-type: application/json" \
  -d '{ "title": "来自我的应用", "body": "正文", "type": "提醒", "link": "/app/my-app" }'
```

### 8.3 限流

按 `(appKey, tenant)` 每分钟限流，额度由**租户套餐**决定：旗舰版 600 / 专业版 300 / 标准版 120（默认 120）。超额返回 `RATE_LIMITED`。独立后端（[§7](#7-集成方式三独立后端应用introspection--服务账号)）自己再做一层限流。

---

## 9. TDT 详解：结构与生命周期

### 9.1 它是什么

TDT = **Tenant-Delegated Token**，一个 HS256 JWT。Claims：

```jsonc
{
  "iss": "xgent-portal",
  "aud": "<appKey>",          // 受众 = 你的应用（= listingKey）
  "kind": "user",             // "user"（默认，有 user_id）| "service"（client_credentials，无 user_id）
  "tenant_id": "...",
  "user_id": "...",           // 服务态无此字段
  "scopes": ["userinfo.read", "notification.send"],
  "av":  3,                    // 签发时的 app token 版本
  "uv":  1,                    // 签发时的 user token 版本
  "uav": 2,                    // 签发时的 (user,app) 版本
  "jti": "...",                // 唯一 id，可单独吊销
  "iat": 1730000000,
  "exp": 1730003600
}
```

### 9.2 签发路径

| 路径 | 触发者 | 场景 | 产物 kind |
| --- | --- | --- | --- |
| `POST /api/tokens/mint` | 用户会话（宿主） | iframe 微应用握手时由宿主注入（[§5](#5-集成方式一嵌入式微应用iframe--sdk)） | user |
| `POST /oauth/token`（authorization_code） | App Secret | 服务端 OAuth（[§6](#6-集成方式二服务端-oauthauthorization_code)） | user |
| `POST /oauth/token`（refresh_token） | App Secret | 续期 | user |
| `POST /oauth/token`（token-exchange） | 来源应用 App Secret | 跨应用调用（[§11](#11-跨应用调用令牌交换-token-exchange)） | user |
| `POST /api/tokens/service`（client_credentials） | 服务账号 Basic | 独立后端主动调用（[§7.3](#73-服务态-tdtclient_credentials)） | service |

**scope 永不扩张**：签发的 scope = 应用声明的 `scopes` ∩ 本次请求的 `scopes`（不传则取应用全部声明）。

### 9.3 撤销机制（四把钥匙）

TDT 短期有效，且支持**即时吊销**——网关每次校验都会比对四个维度：

| 维度 | 怎么触发 | 影响面 |
| --- | --- | --- |
| `jti` 黑名单 | `POST /oauth/revoke` | 单个 TDT |
| app version（`av`） | 应用被停用/卸载、App Secret 轮换或吊销 | 该应用**所有**已签发 TDT |
| user version（`uv`） | 管理员停用/移除该用户（同时销毁其所有会话） | 该用户**所有**应用的 TDT |
| (user,app) version（`uav`） | 用户在「已授权应用」里撤销某个应用 | 该用户对**该应用**的 TDT |

任一维度版本号被 bump，对应的旧 TDT 立即失效（无需等过期）。独立后端因自省结果有 ≤60s 缓存，吊销可能有秒级延迟。

---

## 10. 授权与同意（Consent）

应用第一次代表用户访问数据前，用户要**显式同意**它申请的 scope。

### 10.1 iframe 宿主注入前的 consent 门

`micro` 应用挂载 iframe **之前**，宿主先检查 consent；未授权则渲染一个"授权屏"，列出应用申请的每个 scope，用户点「授权」后才挂载 iframe、才会签发 TDT。

相关接口（会话态）：

```
GET  /api/me/consent/:appKey   → { appKey, appName, required[], granted[], needsConsent }
POST /api/me/consents { appKey }   # 记录对该应用"声明 scope"的授权（不扩张）
```

`POST /api/tokens/mint` 内部也有同一道门：未授权直接抛 `CONSENT_REQUIRED`（携带 `appName` + 缺失的 `scopes`）。

> ⚠️ 同意即"窄化"：`/api/tokens/mint` 会把本次签发的 scope **记为**用户的同意范围。如果你用一个**子集** scope 去 mint，会**收窄**已有同意，之后再用更宽的 scope mint 就会触发 `CONSENT_REQUIRED`。需要更宽 scope 前先重新走 consent。

### 10.2 用户侧管理

用户在 **个人中心 › 已授权应用** 可查看/撤销对每个应用的授权（撤销即 bump 该 `(user,app)` 版本，旧 TDT 立即失效）。

---

## 11. 跨应用调用（令牌交换 Token Exchange）

应用 A 想代表用户去调用应用 B 的能力（拿一个 `aud=B` 的 TDT），走 **OAuth Token Exchange**。这是一条**严格不可提权**的链路，必须同时满足四个条件：

1. **存在授权关系** A→B（`app_exchange_grants`，由管理员建立）。
2. **B 开启了交换**：`targetApp.allowExchange = true`。
3. **A 在 B 的白名单**：A 的 `appKey` ∈ `targetApp.exchangeWhitelist`。
4. **scope 取交集**：结果 scope = A 当前 TDT 的 scopes ∩ B 声明的 scopes（绝不扩张）。

再加一道**用户同意**：用户须事先授权"A 代表我访问 B"，否则抛 `EXCHANGE_CONSENT_REQUIRED`。

```
POST /oauth/token
  grant_type    = urn:ietf:params:oauth:grant-type:token-exchange
  subject_token = <A 的 TDT>
  audience      = <B 的 appKey>
  client_id     = <A 的 appKey>          # 必须等于 subject_token.aud
  client_secret = <A 的 App Secret>
→ { access_token: <aud=B 的 TDT>, scope, expires_in }
```

用户授权页（前端路由）：`/exchange-consent?source=<A>&target=<B>&return=<回跳>`。配套接口：

```
GET  /api/me/exchange-consent/:source/:target   # 谁在申请、目标、scope、是否已授权、是否结构上允许
POST /api/me/exchange-consents                  # 记录授权
GET  /api/me/exchange-consents                  # 列出
POST /api/me/exchange-consents/:source/:target/revoke
```

> 服务端到服务端的交换不带会话，所以 consent 必须**事先**在授权页记录好。撤销交换授权**不**会回滚已签发的短期 TDT（它们很快自然过期），但会拦截后续交换。

常见错误码：`EXCHANGE_NOT_ALLOWED`（缺授权关系/未开启/不在白名单）、`EXCHANGE_CONSENT_REQUIRED`（用户未授权）、`GRANT_NOT_ALLOWED`（来源应用未启用 `token_exchange`）。

---

## 12. 权限管理（ACL）运行时

[§4.4](#44-acl-manifest权限-schema) 讲的是应用**声明**权限模型；本节讲它在运行时**怎么生效**。模型是最简的**纯加法 RBAC + ABAC-lite**（见 [`goal/ACL.md`](../goal/ACL.md)）：

- **角色只授予、不拒绝**；用户有效权限 = 其所有角色授予的**并集**；默认拒绝。
- **`admin` 角色 bypass**：放行一切，永不参与解析。
- **`member` 基线**：每个活跃成员隐式持有，自动获得各应用 Manifest 里 `defaultForMember` 的页面/操作。
- 角色与授权由租户管理员在「权限管理 › 角色矩阵」（`/admin/roles`）配置：

```
GET    /api/admin/roles            # 角色列表（成员数/授权数）
POST   /api/admin/roles            # 新建角色
PUT    /api/admin/roles/:id        # 改名/启停
DELETE /api/admin/roles/:id
GET    /api/admin/roles/:id/grants # 该角色的授权（PID + 数据范围）
PUT    /api/admin/roles/:id/grants # 覆写授权
```

**应用怎么消费 ACL**（两道门，缺一不可）：

1. **前端 UX 门**（隐藏不可见的入口/按钮）：用握手注入的 `init.acl`，经 `sdk.acl.can(pid)` / `sdk.acl.scope(pid)` 判断。**仅 UX，不可信**。
2. **后端安全门**（真正拦截）：
   - 进程内应用：复用 Portal 的 `requirePerm`。
   - 独立后端：自省结果带 `bypass` / `permissions:[{pid,scope}]` / `groups`，自己实现 `can(claims, pid)`（`bypass || permissions.some(p => pidMatches(p.pid, pid))`）与 `permScope(claims, pid)`（取最宽数据范围作行级过滤）。参考 [`apps/spms-server/src/lib/gate.ts`](../apps/spms-server/src/lib/gate.ts)。

服务态 TDT（`kind:"service"`）**整体绕过用户 ACL**——它没有用户，自省返回的 `permissions` 为空、`bypass=false`，你的后端应据 scope（而非 PID）授权服务态调用。

用户自己查权限：`GET /api/me/permissions`（门户/导航推导）、`GET /api/me/permissions/:appKey`（单应用有效权限，SDK 注入即源于此）。

---

## 13. 扩展点与公共服务

应用不仅能"被调用"，还能**反向把数据/入口贡献进 Portal**。

### 13.1 应用动态 / Dashboard Widget（goal/BI-DASHBOARD.md）

Dashboard 是**可配置的 BI 工作台**：系统内置 Widget + 应用 Manifest 声明的 Widget 进同一个 Widget Catalog，用户在页面/Tab/区域里挑选展示。应用通过 **Manifest `dashboard.widgets[]`** 声明可展示的 Widget，再用 `widget.write` 往**自己声明过的 Widget** 推数据（"应用动态"）。

> 旧的 `extPoints: ["dashboard.widget"]` + `sdk.contributeWidget("dashboard.widget", …)` 机制已移除。`extPoints` 仅保留 `settings.section`。

Widget 类型与数据形状（平台通用最小契约）：

| 类型 | 用途 | `data` 形状 |
| --- | --- | --- |
| `metric` | 指标 | `{ value, label?, hint?, trend?: { direction:"up"\|"down"\|"flat", value } }` |
| `activity` | 应用动态流 | `{ title?, items: { title, body?, at?, link?, icon? }[] }` |
| `insight` | 图表 | `{ chart: { kind:"line"\|"bar"\|"area"\|"pie"\|"donut"\|"funnel", x?, series:{ name, data:number[] }[] }, caption? }` |
| `topic_grid` | 入口网格 | `{ items: { title, subtitle?, icon?, color?, link? }[] }` |
| `topic_list` | 清单列表 | `{ items: { title, subtitle?, meta?, link?, icon? }[] }` |

清单声明示例（市场清单 / seed）。`owner` = 应用的 `listingKey`：

```ts
{
  listingKey: "todo",
  scopes: ["userinfo.read", "widget.write"],
  dashboard: {
    widgets: [
      { key: "todo-stats", source: "app", owner: "todo", type: "metric", title: "待办概览",
        defaultItemsPerRow: 4, minColPx: 220, minH: 1, defaultH: 1, dataMode: "app_push" },
      { key: "activity", source: "app", owner: "todo", type: "activity", title: "待办动态",
        defaultItemsPerRow: 3, minColPx: 320, minH: 2, defaultH: 3, dataMode: "app_push" },
    ],
  },
}
```

运行时推送当前用户的某个 Widget 数据：

```ts
await sdk.dashboard.putWidgetData("activity", {
  title: "待办动态",
  items: [
    { title: "新增待办「准备周会材料」", body: "截止今天 18:00", at: new Date().toISOString(), link: "/app/todo" },
    { title: "今日完成 5 项", at: new Date().toISOString() },
  ],
}, { staleAfterSec: 3600 });
```

底层接口 `PUT /api/v1/dashboard/widgets/:widgetKey/data`（upsert，重复 PUT 覆盖）。要点：
- 必须先在 Manifest `dashboard.widgets[]` 声明该 `key`；写未声明的 Widget 得到 `WIDGET_NOT_DECLARED`。
- 数据**按用户**写入当前 TDT 用户，只对该用户可见；按 Widget 的 `type` 校验数据形状。
- 同一个 `(app, user, widgetKey)` 只有一条记录；`DELETE /api/v1/dashboard/widgets/:widgetKey/data` 或 `sdk.dashboard.clearWidgetData(key)` 清除。
- `link` 建议指向 Portal 深链，如 `/app/todo?r=/today`。
- 用户在 `activity` 区域可按 App 勾选"显示哪些应用的动态"；安装快照随 `developerFields` 把 `dashboardWidgets` 拷进 `apps` 行（与 `navItems`/`aclManifest` 一致）。

### 13.2 顶部/侧边导航贡献（navItems）

应用在 Manifest 里声明 `navItems[]`（`{ id, label, icon, path }`），管理员启用其中一个子集（`enabledNavItemIds`）；启用的项会出现在每个成员的左侧「应用导航」分组，深链到 `/app/:appKey?r=<path>`。导航项可经 `AclPage.navItemId` 关联到 ACL 页面，从而按权限/可见性收口。

清单声明示例：

```ts
{
  listingKey: "spms",
  navItems: [
    { id: "nav-issues", label: "Issues", icon: "list-checks", path: "/issues" },
    { id: "nav-board", label: "看板", icon: "columns-3", path: "/board" },
  ],
  aclManifest: {
    version: "1.0.0",
    pages: [
      { key: "issues", path: "/issues", label: { "zh-CN": "Issues" }, navItemId: "nav-issues", defaultForMember: true },
      { key: "board", path: "/board", label: { "zh-CN": "看板" }, navItemId: "nav-board" },
    ],
    actions: [],
  },
}
```

生命周期：
- 市场安装时，`navItems` 随开发者字段快照到 `apps.navItems`，并默认把所有声明项写入 `enabledNavItemIds`。
- 租户管理员之后只能在应用管理里启用/停用这些声明项，不能手填新的 `navItems`。
- Shell 通过 `GET /api/apps/nav` 聚合当前用户可见应用的已启用项；点击后打开 `/app/:appKey?r=<path>`，SDK 初始化时 `init.route` 就是这个 `path`。
- 如果 `navItems[].id` 绑定了 `AclPage.navItemId`，侧栏会结合当前用户 ACL 隐藏无权访问的入口；但应用前端仍要用 `sdk.acl` 做 UX 门，后端仍要做真正安全门。

约定：`id` 在应用内保持稳定，`path` 使用应用内部路由（以 `/` 开头），`icon` 用 Portal 已支持的图标名。改 `id` 等价于删除旧菜单再新增新菜单，租户管理员的启用状态会随快照变化而受影响。

### 13.3 消息服务

`POST /api/v1/notifications`（scope `notification.send`）默认给**当前用户**发通知（自投）。单一收口同时完成：落库 + WebSocket 实时推送（铃铛）+ 按用户偏好发邮件。

**跨用户投递（M2）**：传 `userId` 发给**本租户的另一位活跃成员**（如通知 Issue 指派人，跨租户拒绝）需要额外的 `notification.send.others` scope —— 自投类应用只声明 `notification.send`（无法跨发），只有确需指派通知的应用（如 spms）才声明 `notification.send.others`，用户授权屏会显式展示「可代你向其他成员发送通知」。跨发还受两道限频（每 actor、每 actor→收件人）约束，并强制带可信出处：收件人在收件箱/邮件看到「由 <发送者> 经 <App>」以判断真伪。

**内容约束**：`link` 必须是**站内深链**（白名单前缀 `/app/`、`/inbox`、`/settings`、`/dashboard`），带协议（`javascript:`/`http(s):`…）或协议相对（`//host`）一律 `VALIDATION_FAILED`（M1，防跨用户存储型 XSS + 伪官方钓鱼）。`title`/`body`/`type` 有最大长度。门户自身的内部通知走同一收口，但对不安全 `link`/`actions[].href` 采**净化降级**（丢弃该项并 warn）而非整条失败。

### 13.4 调度服务（Scheduler）

`/api/v1/scheduler/tasks`（scope `scheduler.read`/`scheduler.write`）让应用代表当前用户管理**用户作用域**的计划任务：

- 任务归属 `(应用, 用户)`，每个 `(应用,用户)` 上限 **10** 个（`SCHEDULER_MAX_PER_APP`）。
- `cron` + `tz`（默认 `Asia/Shanghai`）。
- **触发时**：若应用配了 `webhookUrl` → 调你的 webhook（payload 带 `userId`）；否则走内置 `scheduler-notify` → 给任务主人发一条收件箱提醒。
- 管理员在后台只读看到应用创建的任务（标「来自 <App>」），不可改。

### 13.5 Webhook 事件

应用可订阅关于**自身**的事件（`webhookEvents`，默认全订阅），Portal POST 到 `webhookUrl`：

| 事件 | 触发 |
| --- | --- |
| `app.enabled` / `app.disabled` / `app.uninstalled` | 应用状态变化 |
| `consent.granted` / `consent.revoked` | 用户授权/撤销 |

回调体：`{ event, payload, at }`。投递为 best-effort（5s 超时，失败不重试本阶段）。

> 区分：本节 webhook 是**平台→应用**的生命周期事件。你的应用向**调用方**发的业务回调（如异步任务完成回调 + HMAC 验签头）不属于此机制，属于应用自身的 API 契约，写进你自己的对接文档（例：多模态解析的 `X-Omni-Signature`）。

### 13.6 内容管理（headless CMS）

应用声明内容类型（[§4.4-内容类型](#内容类型content-types)），即可用 `sdk.content.*` 或 `/api/v1/content/*`（scope `content.read`/`content.write`）做 CRUD，**自己不用建表/不用后端**。用户私有类型的记录按 `(应用, 用户)` 隔离；租户共享类型对全租户可见。「样例待办」就是纯靠内容服务落数据的例子。

接入步骤：

1. 在 `CONTENT_SCHEMAS` 里新增 schema，`owner` 写应用的 `listingKey`，`key` 是应用内逻辑类型名，`recordScope` 选 `user` 或 `tenant`。
2. 在市场清单 / seed 的 `scopes` 里声明 `content.read`；需要写入则再声明 `content.write`。
3. 重新 seed 或更新市场清单，租户同步更新/重新安装后，清单详情会展示派生出的 `contentTypes`。
4. 应用运行时用 SDK 或 Open API 操作该类型。

SDK 示例：

```ts
const types = await sdk.content.types(); // GET /api/v1/content-types
const task = await sdk.content.create("task", {
  title: "整理需求",
  priority: "med",
  done: false,
});
const rows = await sdk.content.list("task", {
  q: "需求",
  filter: { done: false },
  sort: "-updatedAt",
  limit: 20,
});
await sdk.content.update("task", task.id, { done: true });
await sdk.content.remove("task", task.id);
```

Schema 字段支持 `text` / `textarea` / `number` / `boolean` / `date` / `datetime` / `enum` / `json`，可配置 `required`、`default`、`maxLength`、`min`、`max`、`options`。写入时 Portal 会按 schema 做校验与基础类型转换，失败返回 `VALIDATION_FAILED`。

隔离规则：
- `recordScope:"user"`：记录归属当前 TDT 用户，同一租户内其他用户不可见。
- `recordScope:"tenant"`：记录在当前租户内共享，但仍按应用 owner 隔离。
- 存储表名内部是 `${owner}__${key}`，两个应用都声明 `task` 也不会互相碰撞；调用方只能解析自己 `contentOwner` 下的 schema。

### 13.7 应用配置（App Config）——不要在应用内自建「设置」入口

应用的**租户级配置**（如存储桶、网关默认超时/日志保留、各类开关）统一收口到**平台的应用配置页**，而**不在微应用自己的导航里单独放「设置」**。同一份配置在两处可达，都复用 `AppForm` 的分页配置视图（`apps/web/src/pages/admin/AppForm.tsx`）：

- **平台左下角 `设置 › 应用配置`**（`Settings` 页 `apps` 标签，租户管理员）——主从列表选应用 → 进入该应用的配置分页。
- **`应用中心 / 应用管理` 中的应用配置**（`/admin/apps/:appId`）——同一套 `AppForm`。

**实现模式（参考 Files / 大模型网关）**：

1. 应用的独立后端暴露一个**租户配置读写端点**（如 llm-gateway-server 的 `GET/PUT /api/v1/settings`），仍按 TDT 四道闸鉴权（写操作落 ACL action）。
2. 平台为该端点新增一个**仅管理员的短时 admin-TDT 铸造接口** `GET /api/admin/apps/:appKey/<svc>-token`（无 consent 门，`assertAdmin` 授权；`signTdt` 铸 aud=appKey、按需 scope 的 600s TDT）—— 见 `files-token` / `llm-gateway-token`。
3. 前端写一个 `*-config-api.ts`（用上述接口取 admin-TDT，**浏览器直连**应用后端，门户不经手敏感配置）+ 一个 `XxxSection.tsx`，在 `AppForm` 里按 `form.scopes.includes("<scope>")` 条件挂一个配置 tab（参考 `showStorage` / `showGateway`）。

> 微应用侧因此**不再渲染自己的「设置」页/导航项**；其 ACL Manifest 里的 `settings` 页仅作为后端 `requirePerm` 用的 PID 保留（无 `navItemId`）。

### 13.8 统一审计日志——应用不自建审计，写入平台审计

应用**不要自建审计页/审计表**。需要审计的关键操作（建/改/删渠道、改密钥、改配置、建撤 Token…）经 **`POST /api/v1/audit`（scope `audit.write`）** 推到**平台统一审计**，管理员在 `设置 › 审计`（或后台 `审计`）一处查看：

```http
POST /api/v1/audit        Authorization: Bearer <TDT>
{ "event": "创建渠道", "object": "OpenAI 主渠道", "detail": "provider=openai", "result": "success", "diff": { "...": "..." } }
```

落库为一条 `auditLogs`：`actorName` = 应用展示名（按 aud 解析），`actorId` = 触发用户，`event/object/detail/diff` 来自请求体。属 best-effort，失败不应阻断被审计的操作本身。独立后端用**当次的用户 TDT**（`claims.token`）调用即可——所以应用 Manifest 需声明 `audit.write` scope，用户授权屏会展示「将该应用的关键操作写入平台审计日志」。

---

## 14. 本地开发与调试

```bash
bun install
bun run db:generate && bun run db:migrate && bun run db:seed
bun run dev:all      # api:3000 + web:5300 + 各样例/内置 App（见下）
```

`dev:all` 起的端口：

| 服务 | 端口 | 说明 |
| --- | --- | --- |
| `@xgent/api` | 3000 | Portal 后端 / Open API |
| `@xgent/web` | 5300 | Portal 前端（宿主） |
| `@xgent/sample-app` | 5301 | 样例微应用（SDK 教程） |
| `@xgent/todo-app` | 5302 | 样例待办（内容服务） |
| files-server / files-app | 4100 / 5303 | 文件管理（独立后端，需 minio） |
| spms-server / spms-app | 4200 / 5304 | 研发项目管理（独立后端，库 `xgent-spms`） |
| sms-server / sms-app | 4300 / 5305 | 学校管理（独立后端，库 `xgent-sms`） |
| qbank-server / qbank-app | 4400 / 5306 | 题库（独立后端，库 `xgent-qbank`） |
| lms-server / lms-app | 4500 / 5307 | 教学管理（独立后端，库 `xgent-lms`；教研字典单一事实源） |
| llm-gateway-server / llm-gateway-app | 4600 / 5308 | 大模型网关（独立后端，库 `xgent-llm-gateway`） |

> **本地一把起齐用 `bun run dev:all`**，不要再叠跑单个 `dev:spms` 之类——Vite 端口被占会自增抢相邻端口（如第二个 spms-app 顶掉 sms-app 的 :5305），导致某 App 的 iframe 加载到另一个 App；重起先把旧进程全停掉再 `dev:all`。本地每个后端各用自己的端口（4100–4600）；统一内网端口 `8080`（[§7.5](#75-部署与路由约定独立后端如何接入网关app-integration)）只在容器/生产。按需部署 controller 是生产组件，本地不跑；非生产（`!isProd()`）下 `getAppRuntime` 自动把可部署 App 视作 `ready`，本地不会卡「服务正在部署」门、也无需手动置 `ready`（生产仍走真实门）。

打开 http://localhost:5300 → **使用本地开发账号登录** → 选一个账号。

### 14.1 种子账号（dev 登录，均 `@xgent.ai`）

- `rockie` — 晨光教育集团 admin、星网在线学校 user（跨租户）、**平台管理员**
- `chenjing` — admin，**已开 2FA**（TOTP 密钥 `JBSWY3DPEHPK3PXP`；备用码 `XGENTBK1`–`XGENTBK4`）
- `liming` — 晨光 user、星网 admin
- 另有 `wangfang`（停用）、`zhaolei`、`sunyue`

### 14.2 调试技巧

- **样例微应用**就是活的 SDK 教程：进入 **样例待办**，逐个验证「通知 / 小组件 / 内容 / 提醒（scheduler）/ 深链 / 全屏 / 未保存拦截」。
- **每分钟 cron 提醒**最适合验证调度：在样例待办「提醒」页建一个 `* * * * *`，≤60s 后看铃铛；取消务必走应用（SDK→DELETE），直接删库行不会停掉内存里的 croner。
- **独立后端**：自省优先走服务账号 Basic。`x-resource-key` dev 通道**仅在 `DEV_MOCK_OAUTH=true` 时生效**（M4），且只由门户侧 `FILES_RESOURCE_KEY` 比对放行，并不分别识别 spms/sms/qbank 的 `*_RESOURCE_KEY`；生产（`DEV_MOCK_OAUTH=false`）一律拒绝该 header，必须走 Basic。`PORTAL_INTROSPECT_URL` 默认 `http://localhost:3000/api/tokens/introspect`。
- **回归脚本**在各 server 的 `scripts/`，覆盖 TDT/隔离/consent/交换/令牌策略/调度/ACL/依赖等（见 [`README.md`](../README.md) 的 Verification scripts）。
- 算 2FA 验证码：`cd apps/api && bun -e "import * as O from 'otpauth'; console.log(new O.TOTP({secret:O.Secret.fromBase32('JBSWY3DPEHPK3PXP')}).generate())"`。
- 后端脚本登录请用**无 2FA**用户（如 `liming`）——开了 2FA 的 dev 登录会停在 `pendingTfa`。

> 注意：`db:seed` 会重新生成所有 UUID，使已打开的浏览器会话失效（需重新登录）。独立后端各有自己的 DB 与 seed，互不影响。

---

## 15. 外部服务类应用接入（Docker 镜像交付）

★ 本章面向**服务端代码不在本 repo**、以 **Docker 镜像**交付、集成进 Portal 的应用团队。它是 [§7](#7-集成方式三独立后端应用introspection--服务账号)（独立后端应用）在"外部 repo"场景下的落地补全：§7 讲"你的后端怎么验令牌"，本章讲"你的镜像怎么被注册、布线、部署、联调，以及怎么被别人调用"。

已接入的参照实现：**知识库**（外部镜像 + 门户侧前端，被其他服务经服务账号直调，[knowledge-app-contract.md](knowledge-app-contract.md)）、**多模态解析**（外部镜像、无前端 `service` 型，被文件管理经令牌交换调用，[omni-parser-integration.md](omni-parser-integration.md)）。

### 15.1 内建 App 与外部镜像 App 的分界

| | 内建 App（files / spms / sms / qbank / lms / llm-gateway…） | 外部镜像 App（知识库、多模态解析…） |
| --- | --- | --- |
| 代码 / 构建 | 本 monorepo，全部长在**同一个** runtime 镜像里（compose `command` 选择跑哪个后端） | 你自己的 repo（任意语言/栈），**独立镜像** |
| 部署 | `DEPLOYABLE_APP_SERVICES` 注册表 + deploy-controller 按需拉起 / 缩零 | **常驻**（deploy-controller 只能编排 monorepo 脚本，不管外部镜像）：独立 Deployment / compose profile，无状态可多副本 |
| 注册 | seed / `bootstrap:prod` 内建 | `app.manifest.json` + `register-app`（dev）/ `provisioning.ts` 登记（生产），见 [§15.3](#153-appmanifestjson-与注册布线) |
| `/svc/<key>` 白名单 | 内建于 Caddyfile | `register-app` 写白名单 map（dev）/ 随生产登记 |

两类 App 的**运行时契约完全一致**（四道闸、容器统一 :8080、`/svc/<key>` 路由、健康信封）——差异只在"怎么被注册和部署"。

### 15.2 交付物清单

一次接入 = 外部团队交付四样东西（参照 [knowledge-app-contract.md](knowledge-app-contract.md) 的结构为你的应用写一份同等精度的契约文档）：

1. **后端镜像**：
   - 容器内监听 **8080**（可由 env 覆盖，容器编排统一注 8080）；服务名/网络别名必须是 **`<listingKey>-server`**（通用反代规则按此 DNS 命中）。
   - 多架构 **amd64 + arm64**（生产集群是 amd64）。
   - **不烘焙任何 `.env` / 密钥**，配置全部经环境变量注入。
   - 交付方式：`docker save` tar + sha256，或推平台私有 registry；**不发公共 dockerhub**。
2. **`app.manifest.json`**（[§15.3](#153-appmanifestjson-与注册布线)）——放在你自己的 repo，是应用对平台自描述的**单一事实源**。
3. **env 契约表**：逐个列出镜像**实际读取**的变量名。⚠️ 若镜像内部用自有前缀（如知识库镜像读 `XGENT_PG_DSN`，而门户契约别名叫 `KNOWLEDGE_DATABASE_URL`），契约表必须写清映射；端口变量同理（有的镜像**不读裸 `PORT`**）——compose/K8s 注入时按镜像认的名字对齐。
4. **运维口径**（两问都没有默认答案，必须显式声明）：
   - **DB 迁移由谁跑**：镜像启动自迁（如多模态解析），或外部命令手动跑（如知识库 `scripts/migrate.sh up`）。
   - **是否需要每租户 bootstrap**：若业务表 FK 到自己的 `tenants` 表，须给出首用前的建租户命令/SQL（`id` 必须等于门户租户 UUID，否则首写外键冲突）；若 `tenant_id` 只是普通列则无需。
   - `/health` 判活口径（[§15.4](#154-被调方资源服务器硬契约)）。

若你的应用有前端（`type:"micro"`），前端产物 `dist/` 也在交付物内——同源托管见 [§7.5](#75-部署与路由约定独立后端如何接入网关app-integration)。

### 15.3 `app.manifest.json` 与注册布线

[§3](#3-应用从哪来应用市场-vs-自助注册) 的控制台 API 是手工路径；外部镜像 App 的标准路径是 **manifest 文件 + 脚本注册**：

**manifest 字段**（清单字段全集见 [§4.2](#42-manifest-字段全集)，样例见 `deploy/app-devkit/manifests/`）：`listingKey`（= TDT `aud` = 服务账号/容器命名基名）、`name/version/cat/tagline/desc/icon/color`、`type`（`micro` 有前端 / **`service` 无前端**）、`scopes` + `scopeLabels`、`aclManifest`（纯 scope 鉴权的服务可为 `null`）、`navItems`、`embedUrl`、`embedCsp.connectSrc`、`dependencies`、`exchangeTargets`、`serviceBaseUrl`（`/svc/<listingKey>`），以及 dev 专用的 `serviceAccount{clientId,secret}`、`exchangeInitiatorSecret`（本应用作为交换发起方时的 App Secret 明文，仅本地联调）。

**scope 规则**（清单写入时校验，`apps/api/src/modules/market/service.ts` 的 `validateScopes`）——一条清单只能声明：

- 平台基础 scope（`userinfo.read` / `audit.write` / …）；
- **自己命名空间**：`<listingKey>.` 前缀及其连字符→下划线变体（`omni-parser` → `omni_parser.*`）；
- **已声明 `exchangeTargets` 的目标命名空间**（交换携带目标 scope，所以发起方必须把目标 scope 声明进自己的 `scopes`，否则交换结果 scope 交集为空、调目标全 403）。

越界一律 `VALIDATION_FAILED`。App 命名空间 scope **随清单入库**，不用改门户代码（见 [附录 A](#附录-ascope-目录)）。

**注册（dev / 一盒）**：`bun run register-app <manifest>`（`apps/api/scripts/register-app.ts`，幂等；`NODE_ENV=production` 拒跑）。它做四件事：① 按 `listingKey` upsert 并发布 listing（复用门户写时校验）；② 直写 `token.introspect` 服务账号（secret 取 manifest 明文）；③ 把 `exchangeInitiatorSecret` 写进**已安装**实例的 App Secret——所以**先安装、再重跑一次** register-app，否则发起交换 401；④ 写 Caddy `/svc/<key>` 白名单 map（反代启动即 import，免改 Caddyfile、免重启）。

**注册（生产）**：由平台侧把 manifest 内容登记进 [`apps/api/src/db/provisioning.ts`](../apps/api/src/db/provisioning.ts) 的 `LISTING_DEFS` / `SA_DEFS` / `EXCHANGE_WIRING`，随 `bootstrap:prod` 幂等落库（服务账号 secret 取 env，缺省随机生成、一次性打印）。生产**没有** manifest 明文密钥这条路。

**服务账号约定**：`clientId = <listingKey>-server`、capability `token.introspect`（需主动签服务态 TDT 才加 `client_credentials`，见 [§7.2](#72-服务账号m2m)）。⚠️ 平台侧落库的 secret 与你镜像 env 里的 secret **必须一致**——漂移的症状是自省 401 或「跨应用授权缺失」（实为 `SECRET_INVALID`），排查从比对两侧 secret 开始。

**交换布线四件套**（A 经令牌交换调 B，[§11](#11-跨应用调用令牌交换-token-exchange)）：A→B 的授权关系（`app_exchange_grants`）+ B `allowExchange=true` + A ∈ B 白名单 + 用户 exchange consent。前三样由安装期 `wireExchangeTargets` / 生产 provisioning 幂等建立；consent 在用户首次进入发起方 App 时随 consent 门共授（覆盖其声明的 `exchangeTargets`）。

### 15.4 被调方（资源服务器）硬契约

在 [§7.1](#71-你的后端如何验-tdt令牌自省introspection) 四道闸之上，外部实现最常踩的三处精确形状：

1. **自省信封解包**：`POST /api/tokens/introspect` 的声明在 **`data` 里**（[§8.1](#81-统一响应信封)），解析必须 `claims = body.data ?? body`。直接读顶层 `active` → 一切有效 TDT 被误判 401（外部接入真实事故）。
2. **`/health` 判活信封**：`GET /health` 返回 `{"service":"<listingKey>","db":"ok","redis":"ok"|"disabled","time":...}`——`"db"` 是字符串 `"ok"`，**不是** `true`；平台/devkit 的 healthcheck 按此判活。存活与就绪可再分 `/healthz`（进程活）与 `/health`（依赖就绪）。
3. **门户三变量 all-or-nothing**（推荐启动策略，知识库/多模态解析已采用）：自省地址（`PORTAL_INTROSPECT_URL`）+ 服务账号 clientId + secret 三者**全缺** → 门户鉴权停用、受门路由返 503；**缺一不全** → 启动 fail-fast 并打印缺失变量。避免半配置状态静默放行。

**三种令牌来路，三种授权姿势**——你的服务作为被调方，入站 `Bearer` 有三种来历：

| 来路 | `kind` | `user_id` | 授权依据 |
| --- | --- | --- | --- |
| 自己前端（宿主注入 / `sdk.callService`） | `user` | 有 | scope + PID/ACL（`bypass \|\| permissions`） |
| 其他 App 令牌交换（如 文件管理→多模态解析） | `user` | 有 | 同上；scope 是**交集**（发起方 TDT ∩ 你声明的 scopes）；`azp` = 发起方，用于出处归因 |
| 服务账号 `client_credentials`（其他服务直调，如 某服务→知识库） | `service` | **无** | **只按 scope**（`permissions` 空、`bypass=false`，勿走 PID 门） |

其余照 §7.1：租户隔离按 `claims.tenant_id`（永不信任请求体）、自省缓存 ≤60s、自建 `(aud, tenant)` 限流、审计经 `POST /api/v1/audit`（[§13.8](#138-统一审计日志应用不自建审计写入平台审计)）。

### 15.5 无前端服务的租户管理员配置

`type:"service"` 应用（如多模态解析）没有自己的界面，租户级配置走 [§13.7](#137-应用配置app-config不要在应用内自建设置入口) 的「应用配置」模式，职责切分：

- **外部团队实现**：`GET/PUT /v1/settings`（TDT 四道闸；写门槛按 scope 或 admin role），存租户级覆盖，运行时按 `内置默认 → 租户覆盖 → 单次请求参数` 叠加。
- **门户侧按 App 补一套**（不是零改码路径）：铸短时 admin-TDT 的端点 `GET /api/admin/apps/:appKey/<svc>-token` + 前端 `<svc>-config-api.ts` + `AppForm` 里按 scope 条件挂载的配置 Section（参照 files / llm-gateway 三件套）。浏览器**直连**你的 `/svc/<key>/v1/settings`，门户不经手敏感配置。

> 现状：多模态解析的后端 `/v1/settings` 契约已定，但门户侧三件（`omni-parser-token` / config-api / Section）**尚未实现**——接入新服务时请把这三件列入门户侧工作量。

### 15.6 自有认证面与 Portal 认证面的划界

有的服务自带认证体系（如异步任务网关：外部客户端 API-Key、工作节点 Node-Token、自有 admin 会话，另有 gRPC 端口）。划界规则：

- **凡经 `/svc/<key>` 暴露给门户侧的 HTTP 面**（管理 App、平台服务要调的面），一律走 TDT 自省四道闸——不得让门户侧调用方复用你自有的 admin 会话 / API-Key。
- **自有的节点/外部客户端面不在 Portal 信任域内**：保持原认证不变，但**不得**从 `/svc/<key>` 暴露。`/svc` 约定只转发 HTTP :8080；gRPC 等额外端口在网关契约之外，需单独规划暴露方式（独立 Service/LB）并在交付契约中声明。
- **平台/其他 App 调你的服务**一律走 Portal 机制：有用户上下文走令牌交换、无用户上下文走服务账号 `client_credentials` 签服务态 TDT（scope 落你的命名空间）——而不是给平台发你体系内的 API-Key。这样吊销、审计、租户策略都收口在门户。
- **「本 repo 管理 App + 外部 server」是标准形态**：管理前端是普通 `micro` 应用（`sdk.callService("<key>", …)` 打你的门户面），ACL Manifest 不设 `defaultForMember`（仅租户 admin 经 bypass 可见），导航照 [§13.2](#132-顶部侧边导航贡献navitems) 声明。

### 15.7 本地联调（一盒 App-Devkit）

[§14](#14-本地开发与调试) 的 `dev:all` 是 monorepo 内部姿势；外部团队**不需要**克隆本 repo，闭环是「一盒」：平台交付两个镜像（`ai-portal-one-box` runtime + `ai-portal-proxy` 反代），你提供 `app.manifest.json` + `compose.env` + 自己的镜像，compose 起齐后跑 `register-app` 注册即通。冒烟：

```bash
curl http://localhost/svc/<key>/health          # 反代 → 你的容器（判 §15.4 健康信封）
curl -X POST http://localhost/svc/<key>/v1/...  # 缺/错 token 应 401/403
```

操作步骤、compose 接线、排查速查表见 [`外部App本地联调指南-one-box.md`](外部App本地联调指南-one-box.md) 与 `deploy/app-devkit/README.md`（本章不复制细节，以免两处漂移）。

---

## 附录 A：Scope 目录

完整定义见 [`packages/shared/src/scopes.ts`](../packages/shared/src/scopes.ts)。

| Scope | 含义 |
| --- | --- |
| `userinfo.read` | 读取你的基本资料 |
| `directory.read` | 读取租户通讯录（**不含邮箱**，M5） |
| `directory.email.read` | 读取通讯录成员的邮箱（PII，须与 `directory.read` 同时持有，M5） |
| `notification.send` | 向**你自己**发送通知（自投） |
| `notification.send.others` | 向本租户**其他成员**发送通知（跨用户投递，M2） |
| `inbox.read` | 读取你的收件箱 |
| `settings.read` / `settings.write` | 读 / 写你的设置 |
| `widget.write` | 向工作台推送应用动态 Widget 数据 |
| `audit.write` | 将该应用的关键操作写入平台审计日志 |
| `scheduler.read` / `scheduler.write` | 查看 / 创建取消你的计划任务 |
| `content.read` / `content.write` | 读 / 写该应用为你存储的内容 |
| `files.read` / `files.write` / `files.share` | 文件管理 App：读 / 上传修改删除 / 创建分享 |
| `pms.read` / `pms.write` | 研发项目管理 App：读 / 写 Issue·看板·迭代·项目 |
| `sms.read` / `sms.write` | 学校管理 App：读 / 管理校区·班级·学生·教师·字典 |
| `qbank.read` / `qbank.write` / `qbank.review` | 题库 App：读 / 录入编辑 / 审核（提交·通过·打回·发布） |
| `lms.read` / `lms.write` | 教学管理 App：读 / 管理教研字典、教材、章节目录与课程 |
| `llm_gateway.read` / `llm_gateway.write` / `llm_gateway.admin` | 大模型网关 App：读模型与调用日志 / 创建 API Token 与异步任务 / 管理渠道、路由、租户日志与监控 |

申请原则：**最小够用**。用户授权屏会逐条展示，scope 越多越劝退。`files.*` / `pms.*` / `sms.*` / `qbank.*` / `lms.*` / `llm_gateway.*` 是各独立后端 App 专用，普通应用用不到。内容类型的字段类型（`FIELD_TYPES`）与记录范围见 [`packages/shared/src/constants.ts`](../packages/shared/src/constants.ts)。

**App 命名空间 scope 随清单注册**：`<listingKey>.<动作>` 形式的应用自有 scope（如 `knowledge.read`、`omni_parser.parse`）**无需**改 `packages/shared/src/scopes.ts`，随市场清单的 `scopes` + `scopeLabels` 入库即生效（同意页文案取 `scopeLabels`，缺失降级显示原串）。清单写入时校验（[§15.3](#153-appmanifestjson-与注册布线)）：只能声明平台基础 scope、自己命名空间（含连字符→下划线变体，`omni-parser` → `omni_parser.*`）、以及已声明 `exchangeTargets` 的目标命名空间；越界 `VALIDATION_FAILED`。

## 附录 B：错误码目录

业务错误统一在 `200` 信封的 `error.code` 里（前端只认 `code`，不认文案）。与应用接入最相关的：

| code | 场景 |
| --- | --- |
| `UNAUTHENTICATED` | 缺 TDT（HTTP 401） |
| `INVALID_TOKEN` / `TOKEN_EXPIRED` / `TOKEN_REVOKED` | TDT 无效/过期/已撤销，或 `aud` 不是本应用（独立后端自省校验，HTTP 401） |
| `INSUFFICIENT_SCOPE` | scope 不足（HTTP 403） |
| `INSUFFICIENT_PERMISSION` | ACL 权限不足（HTTP 403） |
| `CONSENT_REQUIRED` | 宿主注入前用户未授权该应用 |
| `EXCHANGE_CONSENT_REQUIRED` | 跨应用调用未获用户授权 |
| `EXCHANGE_NOT_ALLOWED` | 缺授权关系/目标未开启/不在白名单 |
| `GRANT_NOT_ALLOWED` | 应用未启用该 grant_type |
| `APP_NOT_FOUND` / `APP_DISABLED` / `APP_NOT_EMBEDDABLE` | 应用不存在/已停用/不可嵌入 |
| `SECRET_INVALID` | App Secret 错误 |
| `EXT_POINT_NOT_DECLARED` | 未声明该扩展点（`settings.section`） |
| `WIDGET_NOT_DECLARED` | 应用未在 Manifest `dashboard.widgets[]` 声明该 Widget |
| `DASHBOARD_CONFIG_INVALID` | Dashboard 配置校验未通过（页面/区域/范围越界） |
| `SCHEDULER_LIMIT` / `TASK_NOT_OWNED` | 计划任务超限/越权操作 |
| `RATE_LIMITED` | 触发限流（含 M2 跨用户通知的每 actor / 每 actor→收件人 两道桶） |
| `VALIDATION_FAILED` | 参数校验失败（含 M1 通知 `link` 非站内深链、`title/body/type` 超长；M3 新建 `client_credentials` 服务账号未显式选租户策略） |
| `TENANT_NOT_ALLOWED` | `client_credentials` 服务账号为不在 allowlist 的租户签发服务态 TDT（M3） |
| `INVALID_GRANT` | 授权码/refresh_token 无效或 client 不匹配 |
| `LISTING_NOT_FOUND` / `LISTING_NOT_PUBLISHED` | 清单不存在/未上架 |
| `ALREADY_INSTALLED` | 应用已安装到当前租户 |
| `DEPENDENCY_CYCLE` / `DEPENDENCY_UNAVAILABLE` / `DEPENDENCY_REQUIRED` | 依赖环 / 依赖缺失或未上架 / 被依赖（卸载受阻） |
| `SERVICE_ACCOUNT_NOT_FOUND` / `CAPABILITY_NOT_GRANTED` | 服务账号不存在 / 缺所需能力 |
| `INTROSPECT_FAILED` | 自省失败（门户不可达/返回异常） |
| `CONTENT_TYPE_NOT_FOUND` / `CONTENT_ENTRY_NOT_FOUND` | 内容类型/记录不存在 |

完整列表见 [`packages/shared/src/errors.ts`](../packages/shared/src/errors.ts)。

## 附录 C：Manifest / 注册字段参考

应用的"出厂定义"以**市场清单（marketplace listing）**为权威载体；自助注册走 `POST /api/admin/apps`（字段子集，无 `aclManifest`/`navItems`）。

- **清单字段（含 ACL Manifest / 依赖 / 内容类型）** → 见 [§4.2](#42-manifest-字段全集) 全集表，schema 在 [`apps/api/src/modules/market/service.ts`](../apps/api/src/modules/market/service.ts)（`ListingInput` / `developerFields`）。
- **ACL Manifest 类型** → [`packages/shared/src/acl.ts`](../packages/shared/src/acl.ts)；代码注册表 → [`apps/api/src/modules/acl/manifests.ts`](../apps/api/src/modules/acl/manifests.ts)。
- **自助注册字段（`appInput`）** → [`apps/api/src/modules/apps/index.ts`](../apps/api/src/modules/apps/index.ts)：`appKey`(必填,不可改) / `name`(必填) / `desc` / `icon` / `color` / `cat` / `type` / `landingUrl` / `embedUrl` / `allowedOrigins` / `redirectUrls` / `scopes` / `tdtTtl`(60–86400,默认3600) / `allowRefresh` / `refreshTtl` / `allowedGrants` / `allowExchange` / `exchangeWhitelist` / `extPoints` / `enabledNavItemIds` / `showInCenter` / `visibility` / `pinned` / `sort` / `webhookUrl` / `webhookEvents`。

---

*本文档随实现演进。若代码与文档不符，以 [`apps/api/src/`](../apps/api/src/)、[`apps/*-server/src/`](../apps/) 与 [`packages/`](../packages/) 的实现为准。*
