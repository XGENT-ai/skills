# FILE-STORAGE-2 · 应用存储（其他应用利用文件服务存储的内容）

> 站在 `goal/PLAN-4.md`（文件管理 App · 独立后端 `@xgent/files-server`）与 `goal/FILE-STORAGE.md`（多 Provider）之上。
> 本期目标：当前 files 的目录/空间**全部以「用户视角」组织**（个人空间 + 团队空间，文件必属于某个用户）。本期新增**第二类视角 ——「应用存储」**：其他 App 借助文件服务存储的内容，让文件管理界面既能呈现「用户的文件」，也能呈现「应用的文件」。
>
> 阅读前置：先读 [`docs/Files文件服务接入指引.md`](../docs/Files文件服务接入指引.md) §2/§3/§6/§9（空间/文件模型、对接方式、scope×空间角色×ACL）与 [`docs/SSO与App开发指引.md`](../docs/SSO与App开发指引.md) §7/§11/§13（独立后端、令牌交换、服务账号）。本文复用其全部概念。
> 所有 HTTP 约定遵循 [`CLAUDE.md`](../CLAUDE.md)：**业务状态不走 HTTP 状态码**，一律 `200 + { ok, data | error }`。

---

## 目录

- [0. 范围与核心判断](#0-范围与核心判断)
- [1. 现状盘点（as-is）](#1-现状盘点as-is)
- [2. 目标设计（to-be）](#2-目标设计to-be)
- [3. 数据模型变更](#3-数据模型变更)
- [4. 令牌机制变更（门户）](#4-令牌机制变更门户)
- [5. files-server 后端变更](#5-files-server-后端变更)
- [6. 前端变更（files-app）](#6-前端变更files-app)
- [7. 关键风险与陷阱](#7-关键风险与陷阱)
- [8. 分阶段实施 + 验证](#8-分阶段实施--验证)
- [9. Done 标准](#9-done-标准)
- [10. 改动文件清单](#10-改动文件清单)

---

## 0. 范围与核心判断

### 0.1 两个场景（需求原文拆解）

> 「目录的存储都是以用户的视角，需要增加一类应用的存储，即其他应用利用文件存储服务存储的内容。」

| # | 场景 | 谁存的 | 归属 | 谁能看 |
| --- | --- | --- | --- | --- |
| **A** | **service-account 应用存储** | 其他 App 用**服务账号**（M2M，无用户上下文）写入 | **应用所有**（不属于任何用户） | **默认只有租户管理员**；管理员可授权其他**用户**；被授权的其他**应用**也可访问 |
| **B** | **TDT 用户存储** | 其他 App 用**用户 TDT**（令牌交换，代表某个用户）写入 | **仍属于该用户** | 该用户本人；只是在文件管理界面**多一类「应用存储」分组**，让用户看到「不同应用替我创建的文件」 |

两个场景的**共同点**是都要回答「这个文件是哪个 App 经手的」——本期把它抽象成文件的**来源标记 `viaAppKey`**（provenance）。
两个场景的**根本差异**在**归属**：A 是**应用所有**（需要一个不属于用户的容器 + 一套目录级授权），B 是**用户所有**（沿用现有归属，仅加来源标记 + UI 分组）。

### 0.2 核心判断（决定整体工作量）

1. **场景 A 必须新开通道。** files-server 现网关（`apps/files-server/src/lib/gate.ts`）有两道硬约束直接挡死场景 A：
   - `gate()` 要求 `(listingKey ?? aud) === "files"`——而服务账号令牌的 `aud = 服务账号 clientId`，永远不是 `files` → **服务态令牌进不来**；
   - 所有文件操作走 `requireUser()`——**服务态令牌（无 `user_id`）一律 403**，因为现模型里文件必属于用户。

   所以场景 A 不是「加个空间类型」就行，要：**(a) 网关放行服务态令牌（按 scope 授权，不绑 aud）；(b) 新增不属于用户的容器（`kind="app"` 空间）；(c) 一套目录级授权。**

2. **场景 B 是纯增量。** 用户经令牌交换拿到 `aud=files` 的**用户态** TDT，落文件的链路与今日**完全一致**（presign→直传→finalize，归属仍是该用户）。只需：**(a) 文件加来源标记 `viaAppKey`；(b) UI 多一个「应用存储」分组按来源 App 聚合展示。** 不动归属、不动鉴权。

3. **授权用「文件管理专属的目录级 ACL」，不蹭平台粗粒度 ACL。**（本期关键决策，见 §0.3 决策表 #5）平台 ACL（`/admin/roles`，goal/ACL.md）只能按 `app:page/action` + `own/team/all` 授权，**无法表达「只授权 A 应用的存储、不授权 B 应用的」**（无资源级 grant）。本期改为：**应用存储下每个 App 是一个「目录」（`app` 空间），对每个目录单独授权**——复用并扩展现成的 `space_members`（加 `app` 主体；`group` 主体现可借 introspection 回传的 `groups` 真正解析），租户管理员按目录授予 用户/用户组/应用。这就是需求说的「通过系统的权限管理授权」，只是落到**文件管理自己的、目录粒度的**授权面。

4. **来源标记走 `azp`（acting party）令牌声明，可信。** 令牌交换 / 服务态签发时把「经手 App」记到新声明 `azp`，introspection 透出，files-server 自动落 `viaAppKey`。独立后端零额外代码即带上；另允许 `presign` 显式传 `viaApp` 作便捷兜底。

### 0.3 决策表

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | 应用文件的容器 | **新增 `kind="app"` 空间**（每租户 × 每 App 一个，懒创建） | 复用 spaces/files 全套机制（objectKey、列表、检索、分享、预览）；`ownerUserId=null`、`ownerAppKey=<App>`。 |
| 2 | 场景 A 写入身份 | **网关对服务态令牌按 scope 放行**（不绑 `aud`） | 服务账号的 scope 由平台管理员管控，scope 即授权；用户态令牌仍要求 `aud===files`。 |
| 3 | 「经手 App」标识 = **稳定 listing 身份** | **令牌 `azp` = `listingKey`/`contentOwner`（非 raw appKey/clientId）** | 交换签发写 `azp = sourceApp.contentOwner ?? sourceApp.appKey`；服务态写 `azp = sa.ownerAppKey`（必须是 listingKey）。`viaAppKey`/应用目录 `ownerAppKey`/grant 的 `app` 主体**三者同一 key 空间**（listingKey），不混 appKey。**服务态无 `azp` → 直接拒绝进入应用存储**（不回退 clientId）。 |
| 4 | 场景 B 归属 | **不变（仍属用户）** | 文件落用户个人/团队空间，仅多 `viaAppKey`；UI 用 `via` 过滤分组。 |
| 5 | 场景 A 用户授权 | **文件管理专属、目录（`app` 空间）级 ACL** | 扩展 `space_members`：主体 `user│group│app`、角色 `viewer│editor│owner`，**按目录**授予；租户管理员（`role==="admin"`）管理。**不**用平台粗粒度 ACL。 |
| 6 | 场景 A 默认可见性 | **仅租户管理员** | 「看所有应用目录」与「管理目录授权」统一用 introspection 的 **`role==="admin"`**（成员表租户管理员），**不**用 ACL `bypass`（见 #8）；非管理员 default-deny，须被授到具体目录。 |
| 7 | 应用间访问 | **目录授权的 `app` 主体** | 给 A 应用目录加一条 `principalType="app", principalId=<B 应用 listingKey>` 的 grant，B 应用的服务账号即可访问。 |
| 8 | 管理员判定来源 | **`role==="admin"`（成员表），不混 `bypass`** | introspection 同时回 `role`（成员表 admin/member）与 `bypass`（ACL，可能含自定义 bypass 角色）。应用存储治理只认 `role==="admin"`，避免某个自定义 bypass 角色**静默**获得应用存储超管。`bypass` 不作应用存储门；`groups` 仅用于解析 group 主体 grant。 |
| 9 | preview / processors / hooks | **preview、hooks 入本期；processors 派生写回**应用目录**出本期** | 见 §5.5。preview 用统一 `resolveActorRole` 让被授权用户/管理员可预览应用目录文件；upload hook payload `userId` 放宽为可空并加 `ownerAppKey/viaAppKey`；processor 把派生文件写回应用目录（`ownerUserId=null`）暂不支持（§0.5）。 |

### 0.4 关键取舍（显式记录）

- **a. 目录级 = App 级，不做子文件夹级 ACL。** 「目录」单位 = `app` 空间（每个 App 在「应用存储」下是一个顶级目录）。文件自带的 `folder` 路径**不**进入授权判定；按 `folder` 子树细分授权列为后续（无投机抽象）。
- **b. 用户授权粒度到「目录 + 主体」，但主体暂不含平台角色。** 目录 grant 主体支持 `user`/`group`/`app`——`group` 解析依赖 introspection 回传的 `groups`（现已具备）。授予「平台 ACL 自定义角色」需 introspection 额外透出用户角色 id，列为后续。
- **c. 服务态令牌严格圈在应用存储内。** 无 `user_id` 的令牌**只能**操作 `app` 空间（自己的 + 被授权的），**绝不**碰个人/团队空间；presign 不传 `spaceId` 时落它**自己的** App 目录，而非个人空间。
- **d. 服务账号绑定 App 须经租户内有效性校验。** `service_accounts` 是**全局**主体（`tenantAccess=all/allowlist`）。仅加 `owner_app_key` 不够——`/api/tokens/service` 签发时必须额外校验：`tenant_id` 下存在 `status="active"` 的 App 且其 `contentOwner`(=listingKey) `=== sa.ownerAppKey`，否则拒签。防止一个 SA 为「该租户根本没装/已停用的 App」铸出 `azp`，凭空造出应用目录。`tenantAccess=allowlist` 的租户白名单校验（现状）继续生效。
- **e. `presign.viaApp` 手填兜底视为「不可信的展示分组」。** 只在令牌缺 `azp` 时使用，且仅决定 UI 归类、绝不参与访问控制。UI 文案不得把手填来源呈现成强可信来源；可信来源以 `azp` 为准（host-proxy 透传真实经手 App 列为后续）。
- **f. 真·跨 App 端到端无法进 CI 的部分，列手动冒烟。** service-account 链路要平台管理员先建好「绑定到某 App 的服务账号」，CI 内用 dev 资源密钥/mock 模拟；真实第三方 App 接入留手动清单。

### 0.5 明确不在本期

- 子文件夹（`folder` 路径）粒度的 ACL；按 `folder` 树授权。
- 目录 grant 授予「平台 ACL 自定义角色」（需 introspection 透出角色 id）。
- **processor 把派生文件写回应用目录**（应用目录文件运行处理器后产出新文件、`ownerUserId=null`）——本期 processors 仍只服务用户空间文件（§5.5）；应用目录文件的处理器派生列为后续。
- 应用存储的配额/计费、跨 App 的去重共享、应用目录之间的文件搬运。
- 把场景 B 的用户文件「转交」成场景 A 的应用文件（或反向）。
- 服务态令牌的令牌交换（M2M→M2M exchange）；本期服务态令牌直接由 `client_credentials` 自签（已有），不引入服务态交换。

---

## 1. 现状盘点（as-is）

| 层 | 位置 | 现状 / 障碍 |
| --- | --- | --- |
| 空间模型 | `apps/files-server/src/db/schema.ts` `spaces` | `kind ∈ {personal, team}`，`ownerUserId` 必填语义（personal 懒建）。**无应用容器。** |
| 文件模型 | 同上 `files` | `ownerUserId NOT NULL`、无来源字段。**文件必属用户、不知由哪个 App 经手。** |
| 成员/角色 | 同上 `space_members` | `principalType ∈ {user, group}`、`principalId uuid`、角色 owner/editor/viewer。`resolveRole` **只解析 user 主体**（group「stored but not resolved」），**无 app 主体**。 |
| 网关 | `apps/files-server/src/lib/gate.ts` | `gate()` 硬要 `aud===files`；`requireUser()` **拒服务态令牌**；`FilesClaims` **不含** `bypass/groups/permissions`——**files-server 当前完全不消费 ACL**（仅 scope + admin + user 三道）。 |
| 文件路由 | `apps/files-server/src/modules/files.ts` | presign 默认落**个人空间**、finalize 校验 `ownerUserId===userId`、list 限「可见空间(个人+团队成员)」。全程 `requireUser`。 |
| 空间路由 | `apps/files-server/src/modules/spaces.ts` | `/spaces` 只列个人+团队；成员管理 `requireSpaceRole(owner)`。 |
| 门户令牌 | `apps/api/src/lib/tdt.ts`、`modules/token/{index,service}.ts` | `signTdt`（用户态，`aud=appKey`）/ `signServiceTdt`（服务态，`aud=clientId`、`kind=service`、`user_id=null`）；令牌交换在 `token/service.ts` `exchange()`。**令牌不记「经手/源 App」。** |
| 服务账号 | `apps/api/src/modules/platform/service-accounts.ts`、`db/schema.ts` `serviceAccounts` | 有 `clientId/scopes/capabilities/tenantAccess/allowedTenantIds`。**无「绑定到某 App」字段。** |
| introspection | `apps/api/src/modules/token/index.ts` `/api/tokens/introspect` | 已透出 `kind/aud/listingKey/tenant_id/user_id/scopes/role/bypass/groups/permissions/aclStamp/exp`；**服务态 `permissions:null`、无 `azp`。** |
| ACL 消费样例 | `apps/spms-server/src/lib/gate.ts`、`apps/llm-gateway-server/src/lib/gate.ts` | 已有 `can()/requirePerm()/permScope()` + `bypass/groups/permissions` 的成熟范式——**本期 files-server 照搬**。 |
| 前端 | `apps/files-app/src/{App.tsx, components/SpaceSidebar.tsx, components/FileView.tsx}` | 纯 state 切换；侧栏只有「个人空间 / 团队空间」两类；`init.route/onRoute/routeSync` 已接 host。 |
| 共享 | `packages/shared/src/constants.ts` `SPACE_KINDS` | `["personal","team"]`，`SPACE_KIND_LABELS` 无 app。 |

---

## 2. 目标设计（to-be）

### 2.1 概念模型：用户视角 ⊕ 应用视角

```
                       文件管理（files）
       ┌───────────────────────┬────────────────────────────┐
       │   用户视角（现状）       │      应用视角（本期新增）         │
       │   spaces.kind ∈        │   spaces.kind = "app"          │
       │   { personal, team }   │   ownerUserId=null             │
       │   ownerUserId=<user>   │   ownerAppKey=<App>            │
       └───────────────────────┴────────────────────────────┘
                  │                          │
         「我的文件 / 协作空间」        「应用存储」分类（新 nav）
                  │                          │
                  │            ┌─────────────┴──────────────┐
                  │            │  场景B：我的应用文件          │  场景A：应用目录
                  │            │  (用户空间内 viaAppKey≠null) │  (kind=app，应用所有)
                  │            │  按 viaAppKey 聚合           │  每个App一个目录
                  │            │  ——本人始终可见             │  ——默认仅管理员/被授权方
                  └────────────┴────────────────────────────┘
```

- **来源标记 `files.viaAppKey`**（两场景共用）：该文件由哪个 App 经手。
  - 场景 B：`ownerUserId=<user>`、`viaAppKey=<经手App>`（文件仍属用户）。
  - 场景 A：`ownerUserId=null`、`viaAppKey=ownerAppKey=<拥有App>`（文件属应用）。
- **应用目录 `kind="app"` 空间**（仅场景 A）：每租户 × 每 App 一个，App 首次以服务账号写入时**懒创建**（`ensureAppSpace(tenant, appKey)`）。

### 2.2 场景 A：service-account 应用存储

**写入链路**（App 的独立后端，无用户上下文）：

```
① App 服务账号自签服务态 TDT（已有能力，PLAN-4 §3.5）
   POST {PORTAL}/api/tokens/service
     grant_type    = client_credentials
     Authorization = Basic <clientId:secret>
     tenant_id     = <租户>
     scope         = files.read files.write           # SA 须被平台管理员授予 files.*
   → { access_token: <服务态TDT，kind=service，azp=<本SA绑定的App>> }   # §4 azp

② 直连 files-server（与用户态同一套 REST）
   POST {FILES_BASE}/api/v1/files/presign
     Authorization: Bearer <服务态TDT>
     { name, contentType, size }                       # 不传 spaceId → 落本App的应用目录
   → { fileId, uploadUrl, headers }                    # 网关：服务态按 scope 放行；落 ensureAppSpace(tenant, azp)
   ③ PUT 直传桶 → ④ POST /finalize
```

- 网关对**服务态**令牌：跳过 `aud===files`，只校验 `files.*` scope（§4.4 / §5.1）。
- 文件落 `kind=app` 空间（`ownerAppKey=azp`），`ownerUserId=null`、`viaAppKey=azp`。
- 服务态令牌**只能**操作应用目录：不传 `spaceId` → 自己的 App 目录；传 `spaceId` → 必须是它有 `editor` 的应用目录；**永不**落/读个人/团队空间（§0.4c）。

**可见性与授权（目录级 ACL）**：

- **默认**：应用目录无任何 grant（除拥有 App 自身）。`resolveRole` 对 `app` 空间：
  - actor 是拥有 App（`azp === space.ownerAppKey`）→ `owner`（短路，类比个人空间 owner）；
  - actor 是用户且 `role==="admin"`（租户管理员，**非 ACL bypass**，§0.3 #8）→ 视同 `owner`（看/管所有应用目录）；
  - actor 是用户/用户组 → 查 `space_members`（`user`/`group` 主体，`group` 用 introspection 回传 `groups` 解析）；
  - actor 是其他 App → 查 `space_members`（`app` 主体）；
  - 否则 → 无权（default-deny）。
- **授权**（「通过系统的权限管理授权给其他用户 / 其他被授权的应用」）：租户管理员在「应用存储」目录的授权面板增删 grant（`requireAdmin`）：
  - 给**用户/用户组**：`POST /spaces/:id/members { principalType:"user"|"group", principalId, role }`；
  - 给**应用**：`POST /spaces/:id/members { principalType:"app", principalId:<App key>, role }`。

### 2.3 场景 B：TDT 用户存储 + 来源标记

**写入链路**（App 代表用户，令牌交换；与今日唯一差异是落 `viaAppKey`）：

```
① App 持用户 TDT（aud=本App，含 files.write）
② 交换成 aud=files 的【用户态】TDT（SSO §11）→ 该交换令牌带 azp=<源App>   # §4 azp
③ presign（不传 spaceId → 落该用户个人空间），files-server 读 azp → viaAppKey=<源App>
④ 直传 → finalize（归属仍是该用户）
```

- 归属、鉴权**完全不变**（仍 `requireUser`、仍校验空间角色）。
- 仅 `viaAppKey` 被记上：可信来源 = 令牌 `azp`（= 源 App 的 listingKey）；令牌无 `azp` 时才用 `presign.viaApp` 兜底——**视为不可信的展示分组**，仅影响 UI 归类、不参与访问控制（§0.4e）。

**呈现**：文件管理「应用存储」分类下，「我的应用文件」分组 = 用户**自己拥有**（`ownerUserId === 当前用户`）且 `viaAppKey≠null` 的文件，按 `viaAppKey` 聚合（不同 App 一组）。
> ⚠️ 必须显式加 `ownerUserId === userId` 条件：`accessibleSpaceIds` 含**团队空间**，若只过滤 `viaAppKey IS NOT NULL`，普通用户会在「我的应用文件」里看到团队空间内**他人**经 App 创建的文件（越权泄露，见 §5.3 与 §9 回归）。本人始终可见，无需额外授权。

### 2.4 访问控制矩阵（应用存储）

| 主体 | 场景 B（用户所有，viaApp） | 场景 A 应用目录（应用所有） |
| --- | --- | --- |
| 归属用户本人 | 读写自己的文件；「我的应用文件」始终可见 | — |
| 经手 App（用户态 TDT，带 azp） | 代用户写（落 viaAppKey）；按空间角色 | — |
| 拥有 App 的服务账号（服务态，azp=自己） | — | `owner`（读写自己的应用目录） |
| 租户管理员（`role==="admin"`，非 ACL bypass） | 按空间角色（同今日） | 视同 `owner`（看/管所有应用目录 + 目录授权） |
| 其他用户/用户组（目录 grant） | — | 按 grant 角色（viewer/editor） |
| 其他 App 服务账号（`app` 主体 grant） | — | 按 grant 角色（viewer/editor） |
| 未授权用户/应用 | 看不到他人文件 | default-deny（看不到该目录） |

> 应用存储 nav 分类对**所有登录用户**可见（人人可能有场景 B 文件）；**场景 A 应用目录**的可见性由目录级 ACL 决定（默认仅管理员）。

---

## 3. 数据模型变更

### 3.1 files-server 库（`xgent-files`，独立迁移链 → 新迁移 `0002_app_storage.sql`）

| 表 | 变更 |
| --- | --- |
| `spaces` | ① `kind` 取值加 `"app"`（仅类型/约束层，列已是 text）。② 新增 `owner_app_key text`（nullable；仅 `kind=app` 行非空，存拥有 App 的 listingKey）。③ **部分唯一索引**：`UNIQUE (tenant_id, owner_app_key) WHERE kind='app'`（每租户每 App 一个目录）。 |
| `files` | ① `owner_user_id` **改为 nullable**（应用文件无用户 owner）。② 新增 `via_app_key text`（nullable；来源 App）。③ 新增索引 `(tenant_id, via_app_key)`（场景 B「我的应用文件」查询）。 |
| `space_members` | ① `principal_type` 取值加 `"app"`。② `principal_id` **由 `uuid` 改 `text`**（user/group 存 uuid 串，app 存 appKey；该列无 FK，安全）。唯一约束 `(space_id, principal_type, principal_id)` 不变。 |

> **迁移手写**（不靠 `drizzle-kit generate`）：部分唯一索引（`WHERE kind='app'`）与 `ALTER COLUMN … TYPE` 是 drizzle-kit 不稳定/不生成的点（同 LMS COALESCE 索引、reseed 经验）。`schema.ts` 同步改 `$type<>()`/`text()` 即可让 TS 对齐，迁移文件人工写 SQL。

### 3.2 门户库（`apps/api` → 新迁移）

| 表 | 变更 |
| --- | --- |
| `service_accounts` | 新增 `owner_app_key text`（nullable）：该服务账号「代表/经手」的 App，**存 `listingKey`/`contentOwner`（稳定身份），不是 raw appKey/clientId**。`client_credentials` 自签时写入令牌 `azp = sa.ownerAppKey`（§4）。**为空则该 SA 不能进入应用存储**（§4 校验 + 网关拒绝），**不**回退 `clientId`。 |

> 共享类型 `packages/shared/src/constants.ts`：`SPACE_KINDS = ["personal","team","app"]`、`SPACE_KIND_LABELS.app = "应用存储"`（en `App Storage` / zh-TW `應用儲存`）。`packages/shared/src/dto.ts`：`FileDTO.viaAppKey?: string|null`、`SpaceDTO.ownerAppKey?: string|null`、`SpaceMemberDTO.principalType` 加 `"app"`。

---

## 4. 令牌机制变更（门户）

> 目标：令牌可信地携带「经手/拥有 App」（`azp`），让 files-server 自动归因，无需 App 额外传参。改动小、对其他资源服务器同样有益。

| 文件 | 改动 |
| --- | --- |
| `apps/api/src/lib/tdt.ts` | ① `TdtClaims` 加可选 `azp?: string`（**稳定 listing 身份**）。② `signTdt` 入参加可选 `azp`，写进 payload（令牌交换的「源 App」）。③ `signServiceTdt` 入参加可选 `azp`（= SA 的 `ownerAppKey`），写进 payload。**不改签名/校验/撤销逻辑**（`azp` 只是附加声明）。 |
| `apps/api/src/modules/token/service.ts` `exchange()` | 铸交换令牌（`signTdt`）时传 **`azp = sourceApp.contentOwner ?? sourceApp.appKey`**——归一到 listingKey，与 introspection 现有 `listingKey=contentOwner` 同口径（`token/index.ts:220`），避免 appKey 漂移（`<key>-2`）导致三套 key 对不上。 |
| `apps/api/src/modules/token/index.ts` `/api/tokens/service` | **签发前校验 SA→App 绑定在该租户有效**（§下方）；通过后 `signServiceTdt` 传 `azp = sa.ownerAppKey`。`ownerAppKey` 为空时**不**为 app-storage 用途签发（无 `azp`，网关将拒）。 |
| `apps/api/src/modules/token/index.ts` `/api/tokens/introspect` | 响应加 `azp: claims.azp ?? null`（用户态、服务态都透出）。 |
| `apps/api/src/modules/platform/service-accounts.ts` | 创建/更新 SA 接 `ownerAppKey?`（控制台可选填，填 listingKey）；DTO 透出。 |

**服务账号绑定 App 的租户内有效性校验（`/api/tokens/service`，§0.4d）**：当 `sa.ownerAppKey` 非空时，签发前校验——

```
存在一行 apps：tenant_id = <请求租户>
            AND status = 'active'
            AND contentOwner = sa.ownerAppKey      # contentOwner 即 listingKey
否则 → 拒签（APP_NOT_INSTALLED / APP_DISABLED）
```

> 防止全局 SA 为「该租户未安装/已停用的 App」凭空铸 `azp`、造出无主应用目录。`tenantAccess=allowlist` 的租户白名单校验（现状）继续叠加生效。
>
> **不引入服务态令牌交换**：服务账号直接 `client_credentials` 自签（已有），`azp` 来自 SA 绑定，简单且够用（§0.5）。

---

## 5. files-server 后端变更

### 5.1 网关 `lib/gate.ts`（消费 ACL + 放行服务态 + 行动者抽象）

照搬 spms/llm-gateway 的 ACL 范式，并加服务态分流：

```ts
export interface FilesClaims {
  tenantId: string;
  userId: string | null;
  aud: string;
  azp: string | null;            // 新：经手/拥有 App 的 listingKey（introspection 透出）
  kind: "user" | "service";      // 新
  scopes: string[];
  role: string | null;           // 成员表角色（admin/member）— 应用存储治理只认它
  groups: string[];              // 新：用户组（解析 group 主体 grant）
  bypass: boolean;               // 新：ACL bypass —— ⚠ 不作应用存储门（见 §0.3 #8），仅备其他用途
  exp: number | null;
}

// 网关：用户态仍要求 aud===files；服务态按 scope 放行（不绑 aud）。
export async function gate(headers, scope): Promise<FilesClaims> {
  const r = await introspect(token);
  if (!r.active) throw 401 INVALID_TOKEN;
  const kind = (r.kind ?? "user") as "user" | "service";
  if (kind === "user" && (r.listingKey ?? r.aud) !== env.audience)
    throw 401 INVALID_TOKEN;     // 用户态：aud 必须是 files（不变）
  if (!r.tenant_id) throw 401;
  if (!(r.scopes ?? []).includes(scope)) throw 403 INSUFFICIENT_SCOPE;   // scope 即服务态授权
  return { …, azp: r.azp ?? null, kind, role: r.role ?? null,
           groups: r.groups ?? [], bypass: r.bypass ?? false };
}

// 行动者：用户态 → {userId}；服务态 → {appKey: azp}。
// ⚠ 服务态【必须】有 azp（= SA.ownerAppKey 的 listingKey）；缺 azp 一律拒绝进入应用存储
//   —— 不回退 aud/clientId，保证 viaAppKey/ownerAppKey/grant 三者同一 key 空间（§0.3 #3）。
export type Actor = { kind:"user"; userId:string; isAdmin:boolean; groups:string[] }
                  | { kind:"app";  appKey:string };
export function requireActor(claims: FilesClaims): Actor {
  if (claims.kind === "service") {
    if (!claims.azp) throw new HttpError(403, "FORBIDDEN", "服务态令牌缺少 azp（未绑定 App），不能访问应用存储");
    return { kind: "app", appKey: claims.azp };
  }
  if (!claims.userId) throw new HttpError(403, "FORBIDDEN", "该接口需要用户上下文");
  return { kind: "user", userId: claims.userId, isAdmin: claims.role === "admin", groups: claims.groups };
}
export function requireUser(claims): string { … }   // 保留：纯用户接口（分享等）仍要用户上下文
```

> **管理员判定统一用 `role === "admin"`**（成员表租户管理员），**不**用 ACL `bypass`（§0.3 #8）——目录授权管理（`requireAdmin`）、「看/管所有应用目录」皆然。`bypass` 字段保留但不作应用存储门，避免某个带 `bypass` 的自定义 ACL 角色静默拿到应用存储超管。`requireAdmin` 不变（`role==="admin"`，亦用于存储/Webhook）。

### 5.2 空间 `modules/spaces.ts`

- `ensureAppSpace(tenantId, appKey)`：懒建/取 `kind=app, ownerAppKey=appKey, ownerUserId=null` 的目录（类比 `ensurePersonalSpace`）。
- **统一行动者角色解析 `resolveActorRole(space, actor): SpaceRole|null`**（建议补强：抽一个 helper，让 `files`/`preview`/未来 `processors` 都走它、不各自漏改）。按 actor 类型 + 空间 kind 分流（§2.2/§2.4 矩阵）：
  - `app` 空间 + app actor 且 `appKey===ownerAppKey` → `owner`（短路，类比个人空间 owner）；
  - `app` 空间 + user actor 且 `isAdmin`（`role==="admin"`，**非 bypass**）→ `owner`；
  - 否则查 `space_members`：user actor 命中 `(user,userId)` 或 `(group, g∈groups)`；app actor 命中 `(app, appKey)`；取最宽角色。
  - personal/team 分支保留原 `resolveRole(space, userId)` 语义（user actor）。
  - 配套 `requireActorSpaceRole(tenant, actor, spaceId, min)`，取代散落的 `requireSpaceRole(...userId...)`。
- `listSpaces` 增「应用目录」：用户态——`isAdmin` 列全部 `kind=app`，普通用户列其被授权（成员/组命中）的 `kind=app`；服务态——列拥有 + 被授权的应用目录。**`/spaces` 默认仍只回 personal+team**（向后兼容），应用目录走新端点或 `?kind=app` 过滤。
- 成员管理端点（`POST/DELETE /spaces/:id/members`）：**对 `app` 空间改用 `requireAdmin`**（拥有者是 App、用户管不了 owner 角色），并接 `principalType:"app"`、`principalId` 文本化。team 空间维持 `requireSpaceRole(owner)`。

新增端点：

| 方法 + 路径 | scope | 门 | 说明 |
| --- | --- | --- | --- |
| `GET /api/v1/files/app-spaces` | `files.read` | actor 可见者 | 列应用目录（`{ id, appKey, name, fileCount, memberCount? }`）。管理员=全部；用户=被授权；app=拥有+被授权 |
| `GET /api/v1/files/app-spaces/:id/grants` | `files.read` | 成员 viewer 或 admin | 列目录 grant（`SpaceMemberDTO[]`，含 `app` 主体） |
| `POST /api/v1/files/app-spaces/:id/grants` | `files.write` | **admin** | upsert grant `{ principalType:"user"\|"group"\|"app", principalId, role }` |
| `DELETE /api/v1/files/app-spaces/:id/grants/:principalType/:principalId` | `files.write` | **admin** | 移除 grant。**路径必须带 `principalType`**：唯一键是 `(space_id, principal_type, principal_id)`，user(uuid)/group(uuid)/app(text key) 的 id 可能撞值；只带 `principalId` 会误删（现有 `DELETE /spaces/:id/members/:principalId` 的遗留缺陷，本期一并修：成员删除也带 `principalType`）。 |

> grants 端点是 §5.1 成员端点的「应用目录别名 + admin 门」语义封装；实现上可直接复用成员逻辑，前端用更贴切的「授权」语义。

### 5.3 文件 `modules/files.ts`

- `presign`：
  - 服务态 actor → 目标空间 = `body.spaceId ? 校验该 app 空间 editor : ensureAppSpace(tenant, actor.appKey)`；落 `ownerUserId=null, viaAppKey=actor.appKey`。
  - 用户态 actor → 现逻辑（默认个人空间）；落 `viaAppKey = claims.azp ?? body.viaApp ?? null`。
- `finalize`：用户文件维持 `ownerUserId===userId`；**应用文件**（`ownerUserId=null`）改判「actor app 对该空间有 editor」。`viaAppKey` 在 presign 已定，不在 finalize 改。
- `list`（`GET ""`）：新增 `via` 过滤——
  - `via=self` → `viaAppKey IS NULL`（个人/团队视图用，隐藏应用经手文件）；
  - `via=<appKey>` → `viaAppKey = appKey`；
  - `via=app` →「我的应用文件」：`viaAppKey IS NOT NULL` **且 `ownerUserId = actor.userId`**。⚠ **`ownerUserId` 条件不可省**——`accessibleSpaceIds` 含团队空间，只过滤 `viaAppKey IS NOT NULL` 会把团队空间内**他人**经 App 创建的文件泄露进当前用户的「我的应用文件」（§2.3 告警、§9 回归）。
  - **缺省（不传）= 全部**（向后兼容，其他消费方不受影响）。
  - 服务态 actor 的可见空间 = 拥有 + 被授权的应用目录（不含个人/团队）。
- `download/get/:id`、`PATCH/DELETE /:id`：把 `requireUser` 换 `requireActor`，空间角色判定走统一 `resolveActorRole`/`requireActorSpaceRole`（服务态 app 能读写自己/被授权目录的文件）。
- **分享 `modules/shares.ts` 维持 `requireUser`**：对外分享语义绑用户创建者，服务态不参与（最小面，§0.5 不扩）。

### 5.5 周边模块：preview / processors / hooks（这些都内置了「文件必属用户」的假设，必须一并表态）

现有这三处都按「用户态 + 文件属用户」写死，直接拿应用目录文件会出错或语义错位，故逐一定性：

| 模块 | 现状假设 | 本期处理 |
| --- | --- | --- |
| **preview** `modules/preview.ts`（`/:id/preview`，~323） | `requireUser` + `resolveRole(space, userId)` | **入本期**。预览是被授权用户/管理员看应用目录文件的常见路径。保持 `requireUser`（仅用户态可预览），但访问判定换成统一 `resolveActorRole(space, {kind:"user",...})`，让被授权用户/管理员能预览应用目录文件。衍生渲染件（`file_renditions`，落 `_previews/` 保留前缀、无 `files` 行）天然不进文件列表，无归属问题。 |
| **processors** `modules/processors.ts`（`saveDerivedFile(tenant, userId, src, …)` 强写 `ownerUserId=userId`，~68） | 派生文件写回 `src.spaceId` 且 `ownerUserId=userId` | **派生写回应用目录出本期**（§0.5）。本期 processors 仅服务**用户空间**文件；对 `kind=app` 源文件运行处理器**暂不支持**（路由处显式挡掉，给清晰 `error`，而非静默写出 `ownerUserId=null` 的歧义派生件）。待后续设计「应用目录派生件归属」。 |
| **hooks** `modules/hooks.ts`（`runUploadHooks(tenant, file, userId)`，payload `userId: string`，~28） | upload webhook payload 必带 `userId: string` | **入本期**。应用目录文件 finalize 也应触发 `file.uploaded`。`runUploadHooks` 第三参放宽为 `userId: string | null`；payload 加 `viaAppKey` 与（应用文件时的）`ownerAppKey`、`userId` 改为可空。接入文档（Phase 5）同步更新 payload 契约。 |

> 统一走 §5.2 的 `resolveActorRole`/`requireActorSpaceRole`（建议补强项）正是为了让 files / preview /（将来）processors **不各自漏改**访问判定。

### 5.4 引导/回归脚本

- `scripts/bootstrap.ts`：可选种一个「示例应用目录」+ 一条目录 grant，开箱演示（晨光租户）。
- 新增 `scripts/verify-app-storage.ts`：覆盖 §8 的服务态写入、目录懒建、目录 ACL（user/group/app 主体）、场景 B viaApp、`via` 过滤、服务态圈禁个人空间，并含以下**负例/越权回归**：
  - `via=app` **不**返回团队空间内他人文件（§5.3 泄露防回归）；
  - 服务态**缺 `azp`** → `requireActor` 拒（FORBIDDEN）；
  - 删除 grant **不带 `principalType`** 误删的防回归（带 type 才命中唯一键）；
  - 对 `kind=app` 源文件跑 processor → 明确 `error`（不写歧义派生件）；preview 应用目录文件被授权用户可成功。

---

## 6. 前端变更（files-app）

> CLAUDE.md 强制：前端 UI **设计阶段用 `impeccable` skill**、**验证阶段用 Chrome extension** 真浏览器跑通，缺一不可。

| 文件 | 改动 |
| --- | --- |
| `apps/api/src/db/seed.ts` + 安装/provisioning | files-app `navItems` 增 `{ id:"nav-app-storage", label:"应用存储", icon:"database", path:"/app-storage" }`；`FILES_MANIFEST.pages` 增 `{ key:"app-storage", path:"/app-storage", navItemId:"nav-app-storage", defaultForMember:true }`（仅驱动导航，不作鉴权门——鉴权在目录级 ACL）。 |
| `apps/files-app/src/App.tsx` | `spaceForRoute()` 识别 `/app-storage`；新增「应用存储」视图状态；`onRoute/routeSync` 接 `/app-storage`（同 host 路由同步范式）。 |
| `apps/files-app/src/components/SpaceSidebar.tsx` | 侧栏加第三类「应用存储」入口（`Database` 图标）；语义从「空间」泛化为「视图/分类」。 |
| `apps/files-app/src/components/AppStorageView.tsx`（新） | 复用 `FileView` 网格/列表：① 顶部「我的应用文件」(场景 B，`list({via:"app"})` 按 `viaAppKey` 分组)；② 「应用目录」(场景 A，`app-spaces` 列表 → 选中目录 `list({spaceId})`）。空态/无权用 `StateBlock`（`Database` 图标）。 |
| `apps/files-app/src/components/AppGrantsDialog.tsx`（新，仅管理员） | 某应用目录的「授权」面板：列/增/删 grant（用户/用户组/应用 × viewer/editor），调 `app-spaces/:id/grants`。`sdk.userinfo().role==="admin"` 才显示入口。 |
| `apps/files-app/src/lib/i18n.ts` | 三语 parity 新增：`view.appStorage`、`appStorage.myAppFiles`、`appStorage.appDirs`、`appStorage.grant*`、`empty.noAppStorage/forbidden` 等。 |
| `packages/portal-sdk/src/index.ts` | `sdk.files.appSpaces.{list,grants,addGrant,removeGrant}`、`sdk.files.list({ via })` 透出；类型补 `viaAppKey/ownerAppKey`。 |

> 平台「权限管理」(`/admin/roles`) **本期无需改**——场景 A 用户授权落在文件管理自己的目录级 ACL（AppGrantsDialog），不经平台 ACL 矩阵（§0.3 #5）。

---

## 7. 关键风险与陷阱

1. **服务态放行不能破坏隔离（头号）。** 网关对服务态跳过 `aud` 检查后，授权完全靠 scope + `azp`。务必：① SA 遵循最小权限（应用存储 SA 只给 `files.read/write`）；② 服务态**必须有 `azp`**（缺则 `requireActor` 拒），且 actor **严格圈在 `app` 空间**——`resolveActorRole` 对个人/团队空间遇到 app actor 一律拒；presign 服务态不传 `spaceId` 落自己 App 目录而非个人空间。回归用例必须含「服务态读个人空间被拒」「服务态缺 azp 被拒」。
2. **`principal_id` 改 text 的回归。** `ALTER COLUMN … TYPE text` 后，既有 user/group 主体的查询用串比较；确认 `space_members` 无 FK（现状无 `.references()`），现存 team 空间成员判定零回归。删除成员/grant **一律带 `principalType`**（§5.2），避免 uuid/appKey 撞值误删。
3. **`azp` = 稳定 listing 身份，且三套 key 必须同源。** `viaAppKey`、应用目录 `ownerAppKey`、grant `app` 主体三者都用 `listingKey`/`contentOwner`（交换侧 `sourceApp.contentOwner`、服务态侧 `sa.ownerAppKey`），**不混 appKey**（appKey 安装期可漂移成 `<key>-2`）。旧链路/iframe host-proxy 令牌可能无 `azp`（host 以 `appKey=files` 铸 host-proxy 令牌 → 经手 App 丢失）：**服务态无 azp 直接拒应用存储**；场景 B（用户态）才允许 `presign.viaApp` 兜底，且**绝不**用它决定访问、只决定 UI 分组（不可信展示，§0.4e）。host-proxy 透传真实经手 App 列为后续增强。
4. **introspection 缓存延迟。** 目录 grant 增删后，files-server introspection 缓存（≤60s）使授权秒级生效；与 ACL epoch/uv 一致。grant 改动后告知「最长 1 分钟生效」。
5. **`kind=app` 不可被当普通空间改名/删除/建分享。** 个人空间已有「禁改禁删」分支；应用目录同样：不可经 `PATCH/DELETE /spaces/:id`（团队语义），其生命周期随 App（卸载 App 时如何处理应用目录与文件——本期：保留不删，列后续治理）。
6. **向后兼容硬要求。** `/spaces` 默认仍只回 personal+team；`list` 不传 `via` 仍回全部；个人/团队视图与所有现有消费方（knowledge/omni-parser/exam 等经交换/host-proxy 调 files）零回归。`verify-upload/spaces/share/search/fingerprint/exchange-files` 全绿是闸。

---

## 8. 分阶段实施 + 验证

> 每阶段「→ verify:」即成功判据（CLAUDE.md §4 Goal-Driven）。

**Phase 0 · 基线（半天）**
- 跑现有 `verify-upload/spaces/share/search/fingerprint/exchange-files` 对本地 MinIO，记录全绿基线。
- → verify：现状全绿（后续每阶段回归对照）。

**Phase 1 · 共享类型 + 门户令牌 `azp` + SA 绑定校验（0.5–1 天）**
- `SPACE_KINDS+app`、DTO 放宽；`tdt.ts` 加 `azp`；`exchange()` 传 `azp=sourceApp.contentOwner`；`/api/tokens/service` **先校验 SA→App 在该租户有效**再传 `azp=sa.ownerAppKey`；introspection 透出 `azp`；SA 加 `ownerAppKey`（迁移 + 控制台）。
- → verify：workspace `tsc` 全绿；现有 token/exchange 回归脚本不破；新断言——交换令牌 introspect 出 `azp=源App 的 listingKey`、服务态 introspect 出 `azp=SA.ownerAppKey`；**负例**：`ownerAppKey` 指向「未安装 / 已 disabled」的 App、或 `ownerAppKey` 为空（app-storage 用途）、或租户不在 SA allowlist → `/api/tokens/service` 拒签。

**Phase 2 · files-server 库 + 网关（核心，1–2 天）**
- 迁移 `0002_app_storage`（手写：app 空间部分唯一索引、`files.owner_user_id` nullable + `via_app_key`、`space_members.principal_id`→text + `app` 主体）。
- 网关消费 ACL（照搬 spms 范式）+ 服务态放行 + `requireActor`；`ensureAppSpace` + `resolveRole` 分流。
- → verify：`tsc` 绿；`verify-spaces` 回归（team 成员判定不破）；新增 `verify-app-storage` 起步用例：服务态 presign 落自身 App 目录、服务态读个人空间被拒。

**Phase 3 · 文件路由 + 目录授权端点（1–1.5 天）**
- presign/finalize/list/get/download/patch/delete 走 `requireActor` + `viaAppKey` + `via` 过滤；`/app-spaces` + grants 端点；bootstrap 种示例目录。
- → verify：`verify-app-storage` 全绿——
  - 场景 A：App-SA 写入→落 app 目录(`ownerUserId=null,viaAppKey=app`)；默认仅管理员可列；授 user→该用户可列；授 group→组内可列；授 app(B)→B 的 SA 可读；
  - 场景 B：经交换的用户态(带 azp)写入→落用户个人空间 + `viaAppKey=源App`；`via=app` 聚合到位、`via=self` 隐藏；
  - **越权回归**：团队空间内 A、B 两用户各经 App 创建文件 → A 的 `via=app` **只**见自己的、**不见** B 的（§5.3 `ownerUserId` 条件）；
  - 服务态圈禁个人/团队空间、服务态缺 azp 被拒、删除 grant 必带 principalType；
  - preview/processors：被授权用户可预览应用目录文件；对 `kind=app` 源文件跑 processor 明确报错；
  - 向后兼容：`verify-upload/spaces/share/search/fingerprint/exchange-files` 仍全绿。

**Phase 4 · files-app 前端（1–1.5 天，强制 impeccable + Chrome）**
- 先 `impeccable` 出「应用存储」分类（侧栏入口、两分组信息层级、目录授权面板、空/无权态）。
- 实现 `AppStorageView` + `AppGrantsDialog` + nav/manifest + SDK + i18n×3 parity。
- → verify：**Chrome extension** 真浏览器逐项（zh-CN + en）：① 普通用户看到「应用存储/我的应用文件」按来源 App 分组；② 普通用户默认看不到应用目录；③ 管理员看到应用目录 + 在授权面板给某用户授权 → 该用户复登可见；④ 截图留档。环境起不来则显式标注「未在浏览器验证」。

**Phase 5 · 接入指引文档（实现并验证通过后，0.5 天）**
- 更新 `docs/Files文件服务接入指引.md`：新增「应用存储接入」章节——场景 A（服务账号 + `ownerAppKey`/`azp` + 落 app 目录 + 目录级授权）、场景 B（交换 azp/`viaApp` + 「应用存储」分组），并把新端点并入 §8 Open API 全集、新错误码并入 §14。
- → verify：文档与已落地实现逐条对照一致（本期决策三：**实现后再更新**，不提前写未实现 API）。

---

## 9. Done 标准

- [ ] 场景 A 全链路：App 服务账号（绑 `ownerAppKey`=listingKey）经 `client_credentials` 写入 → 落 `kind=app` 应用目录（`ownerUserId=null`、`viaAppKey=拥有App`）；默认仅租户管理员可见；管理员经目录授权面板授予 用户/用户组/其他应用后，被授权方可访问。
- [ ] 场景 B 全链路：App 经令牌交换（携 `azp`=源 App listingKey）代用户写入 → 文件仍属用户、落个人空间 + `viaAppKey=源App`；「应用存储/我的应用文件」按来源 App 聚合呈现，本人始终可见。
- [ ] `azp` 全程为**稳定 listing 身份**（`viaAppKey`/`ownerAppKey`/grant `app` 主体同源），服务态缺 `azp` 拒入应用存储；**SA→App 绑定经租户内有效性校验**（未安装/disabled/空 ownerAppKey 拒签，含负例）。
- [ ] 管理员判定统一 `role==="admin"`（非 ACL bypass）；网关对服务态令牌按 scope 放行且严格圈禁个人/团队空间（含被拒回归）；files-server 消费 introspection `groups` 解析目录授权的 group 主体。
- [ ] **越权回归**：`via=app` 不返回团队空间内他人文件；删除成员/grant 必带 `principalType`（不误删）。
- [ ] **周边模块表态落地**：preview 走统一 `resolveActorRole`、被授权用户可预览应用目录文件；upload hook payload `userId` 可空 + 带 `viaAppKey/ownerAppKey`；对 `kind=app` 源文件跑 processor 明确报错（派生写回应用目录出本期）。
- [ ] `verify-app-storage.ts` 全绿；PLAN-4 既有 `verify-*` 与令牌交换回归零破坏；workspace `tsc` 绿。
- [ ] files-app「应用存储」分类经 impeccable 设计 + Chrome 真浏览器验证（zh-CN + en）；i18n×3 parity。
- [ ] `azp` 令牌声明 + SA `ownerAppKey` 落地，introspection 透出。
- [ ] （Phase 5）`docs/Files文件服务接入指引.md` 应用存储章节回填，与实现一致。

---

## 10. 改动文件清单（速查）

| 文件 | 类型 |
| --- | --- |
| `packages/shared/src/constants.ts`（`SPACE_KINDS+app`、label） | 改 |
| `packages/shared/src/dto.ts`（`FileDTO.viaAppKey`、`SpaceDTO.ownerAppKey`、`SpaceMemberDTO.principalType+app`） | 改 |
| `apps/api/src/lib/tdt.ts`（`azp` 声明 + signTdt/signServiceTdt 入参） | 改 |
| `apps/api/src/modules/token/service.ts`（exchange 传 azp=sourceApp.contentOwner） | 改 |
| `apps/api/src/modules/token/index.ts`（service 签发前校验 SA→App 绑定、传 azp、introspect 透出 azp） | 改 |
| `apps/api/src/modules/platform/service-accounts.ts` + 门户库迁移（SA `ownerAppKey`） | 改 + 迁移 |
| `apps/files-server/src/db/schema.ts`（spaces/files/space_members） | 改 |
| `apps/files-server/src/db/migrations/0002_app_storage.sql` | 新增（手写） |
| `apps/files-server/src/lib/gate.ts`（ACL 消费 + 服务态放行 + requireActor 强制 azp） | 改 |
| `apps/files-server/src/modules/spaces.ts`（ensureAppSpace + 统一 resolveActorRole/requireActorSpaceRole + app-spaces/grants 端点 + app 主体成员 + 删除带 principalType） | 改 |
| `apps/files-server/src/modules/files.ts`（requireActor + viaAppKey + via 过滤含 ownerUserId + 服务态圈禁） | 改 |
| `apps/files-server/src/modules/preview.ts`（访问判定走 resolveActorRole，应用目录可预览） | 改 |
| `apps/files-server/src/modules/processors.ts`（对 `kind=app` 源文件明确报错；派生写回应用目录出本期） | 改 |
| `apps/files-server/src/modules/hooks.ts`（`runUploadHooks` userId 可空 + payload 加 viaAppKey/ownerAppKey） | 改 |
| `apps/files-server/scripts/{bootstrap.ts, verify-app-storage.ts}` | 改 + 新增 |
| `apps/files-app/src/{App.tsx, components/SpaceSidebar.tsx, components/AppStorageView.tsx(新), components/AppGrantsDialog.tsx(新), lib/i18n.ts}` | 改 + 新增 |
| `packages/portal-sdk/src/index.ts`（`sdk.files.appSpaces.*` + `list({via})`） | 改 |
| `apps/api/src/db/seed.ts` + provisioning（files-app navItem + FILES_MANIFEST `app-storage` 页） | 改 |
| `apps/api/src/modules/acl/manifests.ts`（`FILES_MANIFEST` 加 `app-storage` 页，仅导航） | 改 |
| `docs/Files文件服务接入指引.md`（Phase 5：应用存储章节） | 改（实现后） |

---

*本文档为 FILE-STORAGE-2 的设计与实施计划。若代码与文档不符，以 [`apps/files-server/src/`](../apps/files-server/src/) 与 [`packages/`](../packages/) 的实现为准。`docs/Files文件服务接入指引.md` 的应用存储章节按 §8 Phase 5（实现并验证通过后）回填。*
