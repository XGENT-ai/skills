# ACL · XGENT.ai Portal · 平台级基于角色的访问控制（RBAC ＋ ABAC-lite · 多租户 · App 权限清单 · 纯加法授予·多角色并集 · 统一鉴权层）

> 站在一期底座（`goal/PLAN-1.md`）+ 二期平台化层（`goal/PLAN-2.md`）+ 三期应用市场/内容服务（`goal/PLAN-3.md`）+ 四/五期独立后端 App（`PLAN-4/5`）+ SMS/QUIZ-BANK 等业务 App 之上。
>
> 本期交付的**不是一个业务 App，而是一项底座能力（平台机制）**：一套**平台级、基于角色的访问控制体系与服务**，供**每个租户自助使用**。核心三件事：
> 1. **每个 App 提交一份「权限清单（ACL manifest）」**——声明它**全部页面的形式路径（路由模式）**与**操作按钮**，连同 `navItems`（可被提升到顶级导航的菜单）一起，构成 App 自描述的能力 Schema。
> 2. **租户管理员可自定义角色**（增删改、可从模板克隆），为每个角色逐页面/逐操作配置访问权限（**纯加法授予** ＋ 数据范围）；一个用户可被指派多个角色，有效权限**取并集**。
> 3. **统一鉴权层**：角色→权限的解析结果，分发给三类消费者——门户壳、微应用前端、独立后端 App——成为「谁能看到什么 / 谁能做什么」的**唯一真源**；现有「应用可见性 + 导航项开关」派生于它。
>
> **模型刻意取最简的纯加法 RBAC**（Kubernetes RBAC / 经典 RBAC0 心智）：角色只「授予」、不「拒绝」，无角色继承；用户的有效权限 = 其全部角色授予之并集，默认拒绝。**加角色只会让权限变多、永不变少**——单调、可预测、零冲突解析（§4）。
>
> 与现有 OAuth `scope`（应用委托边界）**正交且并行**：scope 答「这个 **App** 能否代表用户调某 API」，ACL 答「这个 **用户** 能否做某操作」。两道闸都过才放行（§5.4）。

---

## 0. 范围与决策（已与产品确认）

### 0.1 这是什么 / 不是什么

| 维度 | 本期 ACL |
| --- | --- |
| 形态 | **平台机制**（底座能力），主体落在 `apps/api`（门户后端）+ `apps/web`（门户壳）+ `packages/shared`（契约）+ `packages/portal-sdk`（下发给微应用）。**不**新建独立后端/独立库。 |
| 边界 | **租户内（intra-tenant）授权**。一个租户的管理员管自己租户的角色与授权；**跨租户/平台级治理**（平台 console、上架清单、服务账号）仍属 **platform-admin**，不在租户 ACL 内（§0.4）。 |
| 与 scope 关系 | **正交**。scope=应用委托/同意边界（粗、App 级、走 consent）；ACL=用户授权（细、角色级）。二者**都过**才放行（§5.4）。 |
| 与现有 admin/user | **非破坏式叠加**。`memberships.role='admin'` 退化为**内置超级角色（恒全通/bypass）**，是引导安全底线；`='user'` 退化为**内置基线角色 `member`**。自定义角色在其上叠加（§3.4）。 |
| 鉴权下发方式 | **introspection 扩展**为权威源（§5.3）：现有 `/api/tokens/introspect` 已回 `role`，本期加 `permissions / groups / aclStamp / bypass`；另加 `POST /api/v1/acl/check` 批量判定。**TDT JWT 不内嵌权限**（避免膨胀与陈旧，§4.3）。 |

### 0.2 三个关键决策（产品已选定，定全设计）

| # | 决策点 | 选择 | 含义 |
| --- | --- | --- | --- |
| 1 | 用户↔角色 | **一人可多角色 · 取并集** | `user_roles` 多对多；有效权限 = 各角色授予之并集（数据范围取最宽，§4.1）。分配 UI 为多选。 |
| 2 | 权限粒度 | **页面+操作 RBAC ＋ 数据范围条件（纯加法授予并集）** | 完整模型（§4）：授予作用于「页面/操作」；可带数据范围 `own/team/all`（ABAC-lite，下发给 App 做行级过滤）；**纯加法——无 deny、无角色继承**；默认拒绝；租户 admin bypass。组合靠多角色指派 + 模板克隆。 |
| 3 | 与现有可见性 | **统一到 ACL** | `appVisibility` 表 + `visibility` 枚举**退役**（迁移成 ACL 授予，§8）；应用是否可见、导航项是否出现，均**派生**自「该用户的有效权限是否含该 App 落地页/该导航项目标页的访问权」。`enabledNavItemIds` 仅保留为**租户级粗开关**（非按角色），与 ACL 取交（§5.1）。 |

### 0.3 关键取舍（显式记录，而非默默处理）

- **a. PID（权限标识）用稳定 `key`，不用 `path`。** 页面在清单里同时有 `key`（稳定标识，进 PID 与授予）与 `path`（路由模式 / 形式路径，仅用于把运行期 `?r=` 路由解析回 page key）。**改 `path` 不破坏已配授权；改 `key` = 新权限项（旧授予置为失效）**。`key` 视同对外 API 契约（§2.4 / §9）。
- **b. 新增权限项默认「未授予」（即默认拒绝）。** App 升级清单引入新页面/操作时，对**所有自定义角色默认不授予**（仅 admin bypass 仍全通），并在「角色管理」提示「应用更新了权限项，请配置」。**绝不**默默授予（安全第一，§2.7）。
- **c. 纯加法、无 deny、无继承——收窄靠拆角色/移指派，不靠「挖洞」。** 选最简模型，是因为「加角色=加权限」的单调心智最符合管理员直觉，且**多角色并集**已能覆盖绝大多数组合需求（如「教师 + 审计员」= 指派两个角色）。要让某人少点权限：给他更窄的角色，或移除某个指派——**不存在**「在宽角色里禁用某项」。若未来真需要「无视任何角色都不能做 X」的全局硬禁止，再单设一个独立、显眼、罕用的「护栏（deny policy）」层（§0.4 后置）。
- **d. 数据范围 `team` 需门户下发用户组。** `own` App 自判（行 owner==user）；`team` 需知道用户的「组/部门」——门户在 introspection 里下发用户 `groups`（复用 `userGroups`/`userGroupMembers`），App 据此过滤；范围词汇由清单 `supportedScopes` 声明，矩阵只给合法选项（§2.5 / §5.3）。
- **e. admin 恒全通（bypass）。** 内置 `admin`（bypass=true）**不参与解析**、不可删、其授予不可编辑——保证「无论权限怎么配，管理员永远进得去」的引导底线。纯加法下也天然不可能把 admin 锁死（无 deny），bypass 只是省去给 admin 枚举授予（§3.4 / §9）。
- **f. 权限只 UX-gate 前端、安全-gate 后端。** 微应用前端拿到有效集只用于「隐藏/禁用」按钮与导航（体验）；**真正的拦截在后端**（门户 Open API 与各独立后端 App 的 `gate()`，§5.2/§5.3）。前端不可信。
- **g. 服务态令牌（M2M）旁路 ACL。** `kind='service'` 无用户上下文，仅受 scope + 服务账号 capability 约束；introspection 对其回 `permissions:null / bypass:false`（§5.5）。

### 0.4 明确不在本期

- **deny / 全局硬禁止护栏**（无视任何角色都不能做 X，类 AWS SCP / GCP deny policy）——本期纯加法不含 deny；如未来确有需求，作为**独立、显式、罕用**的护栏层后置，不混进日常角色矩阵。
- **角色继承（role extends role）**——本期扁平角色；组合用多角色指派 + 模板克隆替代。
- **跨租户 / 平台级角色**（platform-admin 治理平台 console、市场上架、服务账号）——仍走 `assertPlatformSession`（platform tenant 的 admin），**不**纳入租户 ACL。
- **字段级 / 列级权限**（同一页面内某字段只读）——天花板止于「页面+操作+数据范围」；字段级留后续。
- **时限角色 / JIT 提权 / 审批流**——留后续；`user_roles` 预留扩展位但本期不做 UI。
- **外部 IdP 角色映射**（SCIM/SAML group→role 自动同步）——`user_roles` 是同步落点，但本期只做手动指派。

---

## 1. 概念模型（术语 · 关系 · 与现有概念对齐）

### 1.1 六个核心概念

```
App ──声明──▶ 权限清单(manifest)
                 ├─ 页面(page)      = 形式路径(path 路由模式) + 稳定 key（+ 可选 navItemId 关联顶级导航）
                 ├─ 操作(action)    = 按钮/操作 key（+ 可选归属 pageKey）
                 ├─ 组(group)       = 矩阵 UI 分组（纯展示）
                 └─ 角色模板(template)= App 建议的起步角色（Viewer/Editor/Admin…）

每个 page/action ──映射──▶ PID（权限标识）= <appKey>:<page|action>:<key>，支持通配 *

租户管理员 ──创建──▶ 角色(role)  ──持有多条──▶ 授予(grant) = { pid, condition?: {dataScope} }   // 纯 allow，无 deny

用户(user) ──user_roles 多对多──▶ 角色集合（含内置 member 基线）
                                      │
                                      ▼
                          鉴权引擎解析（§4）= bypass? → 收集角色 → 匹配 grant → 命中即允许 → 数据范围取最宽
                                      │
                                      ▼
                          有效权限(effective) = { 命中的 PID 集合 + 各自数据范围 }   // 角色越多权限只增不减（单调）
                                      │
        ┌─────────────────────────────┼─────────────────────────────┐
        ▼                             ▼                             ▼
   门户壳(web)                  微应用前端(iframe)            独立后端 App(gate)
   /api/me/permissions          SDK init 注入 per-app 集       introspection 扩展 + /acl/check
   → 导航/应用可见性派生          → sdk.can() 隐藏/禁用按钮       → 校验 PID（安全真闸）
```

### 1.2 ACL vs scope（正交 · 双闸）

| | scope（已存在） | ACL permission（本期） |
| --- | --- | --- |
| 回答 | 这个 **App** 能否代表用户调此 API | 这个 **用户** 能否做此操作 |
| 粒度 | 粗（App 级，读/写域） | 细（页面 / 操作 / 数据范围） |
| 主体 | App（经 consent 同意） | 用户（经角色指派） |
| 载体 | TDT 的 `scopes` 声明 | introspection 的 `permissions`（每次新解析） |
| 配置者 | 用户（consent 页）+ 管理员（App 声明 scope） | 租户管理员（角色授予） |
| 判定点 | `requireTdt(scope)` / `gate(scope)` | `gate` 内 `requirePerm(pid)` + 数据范围过滤 |

> **放行 = scope 命中 ∧ ACL 命中 ∧ 数据范围过滤**。例：`qbank.write`（scope）让 App 能代表用户写题库；`qbank:action:question.publish`（ACL）决定**这个**用户能否发布；`dataScope='own'` 让该用户只发布自己录入的题（§5.4 实例）。

### 1.3 统一到 ACL：现有可见性如何派生

| 现有机制 | 现状 | 统一后 |
| --- | --- | --- |
| `apps.visibility`(all/group/user) + `appVisibility` 表 | 决定「谁能在应用中心看到/启动该 App」 | **退役**。App 可见 ⇔ 用户有效权限含该 App **任一页面**（默认看落地页 `landingPageKey`）的授予。迁移把旧规则转成 ACL 授予（§8）。 |
| `apps.enabledNavItemIds` | 管理员勾选「哪些声明的导航项可上顶级导航」 | **保留为租户级粗开关**（全员维度）。最终某导航项是否对**某用户**出现 = `enabledNavItemIds` ∩ 「用户可访问该导航项目标页（`page.navItemId` 关联）」。 |
| `navItems`（App 声明） | 可提升到顶级导航的菜单清单 | 不变，**且与 manifest 页面交叉引用**：`page.navItemId` 把一个页面标记为某导航项的目标页，使导航天然随角色显隐。 |

---

## 2. App 权限清单（ACL manifest）契约

### 2.1 形状（`packages/shared/src/acl.ts` 新增）

```ts
export type DataScope = "own" | "team" | "all";          // 数据范围（ABAC-lite）

export interface AclCondition {
  dataScope?: DataScope;                                  // 行级可见范围；缺省=all
  // 预留：通用属性条件（本期不出 UI）：attributes?: Record<string, string[]>
}

export interface AclPage {
  key: string;                 // 租户/版本内稳定标识，进 PID。例 "projects" / "project-detail"
  path: string;                // 形式路径=路由模式，匹配运行期 ?r=。例 "/projects/:id/issues"
  label: I18nText;             // { zh, en, ... } 或 i18n key
  parentKey?: string;          // 页面树（矩阵分组 + 前缀通配）
  navItemId?: string;          // 关联 apps.navItems[].id：本页是该导航项的目标页（§1.3）
  supportedScopes?: DataScope[]; // 本页支持的数据范围（矩阵只给这些选项；缺省=["all"]）
  defaultForMember?: boolean;  // 安装/升级时，内置 member 基线默认授予（其余默认不授予，§2.7）
}

export interface AclAction {
  key: string;                 // 操作标识。例 "project.create" / "question.publish"
  label: I18nText;
  pageKey?: string;            // 按钮所在页面（null=App 全局操作）
  supportedScopes?: DataScope[];
  dangerous?: boolean;         // UI 提示：破坏性操作（红色/二次确认）
  defaultForMember?: boolean;
}

export interface AclGroup { key: string; label: I18nText; sort?: number; } // 矩阵 UI 分组

export interface AclRoleTemplate {                          // App 建议的起步角色
  key: string; name: I18nText; desc?: I18nText;
  grants: Array<{ pid: string; condition?: AclCondition }>; // 纯 allow，无 effect 字段
}

export interface AclManifest {
  version: string;             // 清单版本（reconcile 依据，§2.7）
  landingPageKey?: string;     // 落地页：App 可见性默认看这页（缺省=第一个 page）
  groups?: AclGroup[];
  pages: AclPage[];
  actions: AclAction[];
  roleTemplates?: AclRoleTemplate[];
}
```

> `I18nText` 复用门户既有的多语言约定（参见 `apps/web/src/lib/locales/labels.ts`：consent/scope 标签三语来源）。清单内 `label` 可直接给 `{zh,en,...}`，矩阵/导航按宿主语言取词。

### 2.2 页面（形式路径）与路由匹配

- **形式路径 = 路由模式**（带参数占位），如 `/projects/:id`、`/projects/:id/issues`。它是「页面」这一权限主体的稳定形态，**不是**某个具体 URL。
- 运行期门户壳/微应用导航到 `/app/:appKey?r=/projects/123/issues`；ACL 把 `?r=` 的具体路径**匹配回**某个 `page.path` 模式 → 得 page key → 判 `appKey:page:<key>`。匹配规则：最长前缀 + 参数段通配（`:id` 匹配单段）；命中多个取**最具体**（更长、参数更少者）。
- 页面树：`parentKey` 既用于矩阵分组，也支持**前缀通配**——授予 `spms:page:projects.*` 覆盖 `projects` 下所有子页（要求子页 key 以 `projects.` 前缀或经 `parentKey` 形成树，§2.4）。
- **未在清单声明的路径** = 无对应 page = **默认拒绝**导航（壳/SDK 不渲染入口；后端不暴露则天然不可达）。鼓励 App 把「所有页面」纳入清单（用户原话：提供**所有页面**的形式路径）。

### 2.3 操作（按钮）

- 一个 `action` = 一个可被授权的操作意图，通常对应一个按钮/菜单项（新建、删除、发布、导出、批量改…）。
- `pageKey` 把操作挂到页面（按钮所在页），便于矩阵在该页下罗列其按钮；`pageKey=null` 为 App 全局操作。
- 操作的后端判定点：各 App 的 `gate()` 在写/敏感端点上 `requirePerm("<appKey>:action:<key>")`（§5.3）；前端 `sdk.can()` 决定按钮显隐/禁用（§5.2）。
- `dangerous` 仅 UI 语义（破坏性 → 红色 + 二次确认，呼应 DESIGN 的危险态与 PMS-2 的 `ConfirmDestructive`）。

### 2.4 PID 编址与通配

```
PID 文法： <appKey> ":" <kind> ":" <key>
  kind ∈ { page, action }
通配（从粗到细）：
  *                       任意（仅内置 admin 语义等价于 bypass，正常不出现在自定义授予）
  <appKey>:*              整个 App
  <appKey>:page:*         该 App 全部页面
  <appKey>:action:*       该 App 全部操作
  <appKey>:page:projects.*  该页面子树（前缀，依 key 的点号层级）
  <appKey>:page:projects    精确
```
- 匹配函数 `pidMatches(grantPid, targetPid)`：精确相等，或 `grantPid` 含 `*` 且按段/前缀覆盖 `targetPid`。
- 纯加法下**任一**命中（精确或通配）即授予；多条命中只用于数据范围取并（§4.2）。无具体度比较、无冲突。

### 2.5 数据范围条件（ABAC-lite）

- 授予可带 `condition.dataScope ∈ {own, team, all}`。门户**不懂 App 业务数据**，只把解析出的 `dataScope` 连同用户 `groups` 下发；**由 App 解释**：
  - `own` → 仅 `行.ownerUserId == claims.user_id`；
  - `team` → 行归属 ∈ 用户 `groups`（门户在 introspection 下发的 `userGroups` id 集，§5.3）；
  - `all` → 不加行级过滤。
- 词汇由清单 `supportedScopes` 声明 → 矩阵只对支持的页面/操作给数据范围选择器；不声明则只有「授予（all）/不授予」二态。
- 多角色并集时，同一 PID 的有效数据范围 = **取最宽**（`own < team < all`），见 §4.2。

### 2.6 预置角色模板

- App 在清单里给 `roleTemplates`（如 Viewer=全部 page 授予 + 无 action；Editor=+ 常规 action；Admin=`<appKey>:*`）。
- 租户管理员「新建角色」时可「从模板克隆」一键铺好授予，再微调——**降低空白配置成本**，配合多角色指派，扁平模型也能灵活组合。模板只是**起步快照**，克隆后与模板脱钩（App 升级模板不回改已建角色）。

### 2.7 清单存储与市场快照 · reconcile

- 存储：`apps.aclManifest jsonb`（与 `navItems` 同列族风格）；上架清单 `marketplaceListings.aclManifest jsonb`。安装时随其它开发者字段**快照拷贝**进 `apps`（沿用 `developerFields()` 快照范式，PLAN-3）。
- 声明来源：与 `navItems` 完全一致——**由 App 自身声明**（seed / 市场清单），**租户管理员不可编辑清单本身**，只对角色配授予。
- **reconcile（清单升级/重装时）**：
  - 新增 page/action：对所有自定义角色**默认不授予**；`defaultForMember:true` 的项给内置 `member` 基线授予；「角色管理」红点提示「N 个新权限项待配置」。
  - 删除 page/action：相关 `role_grants` 行**保留但标记失效（stale）**（矩阵置灰「已失效」，下次编辑时清理），保审计可追溯——不静默删授权记录。
  - `key` 不变即视为同一权限项（path/label 变化不影响已配授予，§0.3a）。

### 2.8 内置清单：门户壳自身也是一份清单

把门户原生界面建模为两份保留 appKey 的内置清单，使「整个体系」真正统一：

| appKey | 覆盖 | 例（pages/actions） |
| --- | --- | --- |
| `portal` | 租户内门户原生面 | page：`home`/`apps`(应用中心)/`inbox`/`settings`/`profile`；admin：`admin.users`/`admin.apps`/`admin.roles`/`admin.audit`/`admin.tasks`；action：`audit.export`/`user.invite`/`task.run`… |
| `console` | 平台 console 面 | **不进租户 ACL**——仍由 platform-admin 守（§0.4）；列此仅为完整性，租户矩阵不可见。 |

- 价值：管理员可建「只读审计员」角色（仅授予 `portal:page:admin.audit` + `portal:action:audit.export`）而**不必给全 admin**——这正是「全面+灵活」的体现：在纯加法下，靠**精确授予 + 多角色指派**就能组合出最小权限。
- 引导底线：`portal:page:admin.roles` / `portal:action:acl.manage`（管理 ACL 本身）默认仅 admin（bypass）持有；可授予他人，admin 始终 bypass（§0.3e）。

---

## 3. 数据模型（门户 `apps/api` · Drizzle · 全表多租户）

### 3.1 三张新表 + 一列

```ts
// ---------- ACL：平台级纯加法 RBAC（+ ABAC-lite 数据范围） ----------

export const roles = pgTable(
  "roles",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    tenantId: uuid("tenant_id").notNull().references(() => tenants.id, { onDelete: "cascade" }),
    key: text("key").notNull(),                          // 租户内稳定标识（builtin: "admin" | "member"）
    name: text("name").notNull(),
    desc: text("desc"),
    builtin: boolean("builtin").notNull().default(false),// admin/member 不可删
    bypass: boolean("bypass").notNull().default(false),  // admin=true → 恒全通、免解析（§4.1）
    sort: integer("sort").notNull().default(0),
    status: text("status").notNull().default("active"),  // active | disabled
    createdBy: uuid("created_by"),
    createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
    updatedAt: timestamp("updated_at", { withTimezone: true }).defaultNow().notNull(),
  },
  (t) => [unique("roles_tenant_key").on(t.tenantId, t.key), index("roles_tenant_idx").on(t.tenantId)],
);

export const roleGrants = pgTable(
  "role_grants",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    tenantId: uuid("tenant_id").notNull().references(() => tenants.id, { onDelete: "cascade" }),
    roleId: uuid("role_id").notNull().references(() => roles.id, { onDelete: "cascade" }),
    pid: text("pid").notNull(),                          // <appKey>:<page|action>:<key>，支持 *；纯授予(allow)
    condition: jsonb("condition").$type<AclCondition | null>(), // 数据范围等；null=无条件(all)
    stale: boolean("stale").notNull().default(false),    // 清单删项后置真（§2.7）
    createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
  },
  (t) => [
    unique("role_grants_role_pid").on(t.roleId, t.pid),  // 一个角色对一个 PID 至多一条授予
    index("role_grants_role_idx").on(t.roleId),
  ],
);

export const userRoles = pgTable(
  "user_roles",
  {
    id: uuid("id").defaultRandom().primaryKey(),
    tenantId: uuid("tenant_id").notNull().references(() => tenants.id, { onDelete: "cascade" }),
    userId: uuid("user_id").notNull().references(() => users.id, { onDelete: "cascade" }),
    roleId: uuid("role_id").notNull().references(() => roles.id, { onDelete: "cascade" }),
    assignedBy: uuid("assigned_by"),
    createdAt: timestamp("created_at", { withTimezone: true }).defaultNow().notNull(),
  },
  (t) => [
    unique("user_roles_user_role").on(t.userId, t.roleId),
    index("user_roles_tenant_user_idx").on(t.tenantId, t.userId),
  ],
);

// apps / marketplaceListings 各加一列：
//   aclManifest: jsonb("acl_manifest").$type<AclManifest | null>(),
```

> 无 `extendsRoleId`（无继承）、无 `effect`（无 deny）——表结构刻意最小。组合表达力来自 `user_roles` 多对多并集。

### 3.2 缓存失效戳（ACL epoch）

为让独立后端 App 的 introspection 缓存可快速失效（§4.3），在轻量计数器上加两个版本：

- **租户级 `aclEpoch`**：任一角色/授予/清单变更时 +1（存 `tenants` 一列或 Redis `acl:epoch:<tenantId>`）。
- **用户级 `uv`（已存在的 user token version）**：用户的角色指派变更时 `bumpUserVersion(userId)`（复用 PLAN-2 撤销机制）——使该用户已签发的 TDT 失效、强制重新 mint+introspect。
- introspection 回 `aclStamp = "t<aclEpoch>.u<uv>"`，App 以此为缓存键的一部分（§5.3）。

### 3.3 错误码（`packages/shared/src/errors.ts` 增量）

| code | 语义 | HTTP | 备注 |
| --- | --- | --- | --- |
| `ROLE_NOT_FOUND` | 角色不存在 | 200+业务 | 管理类失败走 200（CLAUDE.md 约定） |
| `ROLE_KEY_TAKEN` | 角色 key 冲突 | 200+业务 | |
| `ROLE_BUILTIN_LOCKED` | 内置角色不可删/admin 授予不可改 | 200+业务 | member 可改授予、不可删 |
| `MANIFEST_INVALID` | 清单结构非法（key 重复/path 非法…） | 200+业务 | seed/上架时校验 |
| `INSUFFICIENT_PERMISSION` | 运行期 ACL 拒绝（缺所需 PID） | **403** | 与既有 `INSUFFICIENT_SCOPE` 同档（鉴权失败属 4xx，CLAUDE.md 允许） |

### 3.4 与 `memberships.role` 的关系（非破坏 · 引导安全）

- **保留** `memberships.role`（admin/user）作为「是否租户管理员」的**引导事实**：`role='admin'` ⇔ 内置 `admin` 角色（bypass）。`assertAdmin`、introspection 既有 `role` 字段**完全不变**——ACL 是叠加，不改既有调用。
- 内置角色 seed（每租户 bootstrap 时建，迁移时回填）：
  - `admin`：`builtin=true, bypass=true`，不可删、其授予不可编辑（§0.3e）。`memberships.role='admin'` 的用户**隐式持有**（不必进 `user_roles`）。
  - `member`：`builtin=true, bypass=false`，**全体活跃成员隐式持有**（基线）。授予可由管理员调（组织级默认权限）。初始基线 = `portal:page:home/apps/inbox/settings/profile` + 各 App 清单 `defaultForMember` 项。
- 任一非 admin 用户的角色集合 = `{member}` ∪ `user_roles` 指派的自定义角色 → 天然多角色并集（决策 1）。

---

## 4. 鉴权引擎（有效权限解析 · 纯加法）

### 4.1 解析算法

`resolve(userId, tenantId, targetPid, ctx?) → { allowed: boolean, scope: DataScope }`

```
1. 引导旁路：若 membership.role==='admin'（内置 admin/bypass）→ return { allowed: true, scope: "all" }。
2. 角色集合：roleSet = {member 基线} ∪ assignedRoles(userId)   // 仅取 status='active' 的角色
3. 匹配授予：grants = roleSet.flatMap(grantsOf)
                       .filter(g => !g.stale && pidMatches(g.pid, targetPid))
4. 判定：if grants.isEmpty → return { allowed: false }          // 默认拒绝，无任何 deny 概念
        else allowed = true
5. 数据范围：scope = broadest( grants.map(g => g.condition?.dataScope ?? "all") )  // own<team<all 取最宽
6. return { allowed, scope }
```

- **无冲突**：没有 deny、没有继承链、没有具体度比较——「命中任一授予即允许」。**加角色/加授予只会让 `allowed` 从 false→true、`scope` 从窄→宽，永不反向**（单调性），这是选纯加法换来的最大可预测性。
- **批量**：`resolveSet(userId, tenantId, appKey)` 一次算出某 App 全部 page/action 的有效集（供 §5 三处下发）；O(角色×授予)，结果可按 `aclStamp` 缓存。

### 4.2 通配匹配与数据范围取并

- `pidMatches`：精确 / `appKey:*` / `appKey:kind:*` / `appKey:page:<prefix>.*`（页面树前缀）/ 精确 key。
- 多条授予命中同一 PID（来自不同角色或通配+精确并存）时，仅做一件事：**数据范围取最宽**（`own<team<all`）。不存在「谁覆盖谁」的优先级。

### 4.3 缓存与失效（为什么权限不进 TDT）

- **不**把权限写进 TDT JWT：(1) 角色/授予一改，已签发 token 会陈旧；(2) 大租户权限集会让 token 膨胀。
- 权威源 = **introspection 现算**（App 已经在 introspect 取 `role`，本期顺带回 `permissions`）。App 用其既有 introspection 缓存（按 token jti，TTL 30~60s，复用 PLAN-4/5 范式），缓存键并入 `aclStamp`。
- 失效路径：管理员改角色/授予 → `aclEpoch++`（变 `aclStamp` 的 `t` 段）→ App 下次 introspect 自然取到新值；改用户角色指派 → 额外 `bumpUserVersion` 使该用户 TDT 立即失效（强制重新 mint+introspect），实现**近实时收回**。

---

## 5. 有效权限如何抵达三类消费者

### 5.1 门户壳（`apps/web`）——可见性/导航**派生**

新增 `GET /api/me/permissions`（会话鉴权）：

```jsonc
{ "ok": true, "data": {
  "bypass": false,                                  // 是否租户 admin
  "roles": [{ "key": "member", "name": "成员" }, { "key": "teacher", "name": "教师" }],
  "portal": [ { "pid": "portal:page:home", "scope": "all" }, ... ],  // portal 内置清单有效集
  "apps": [                                          // 派生的可见性（§1.3）
    { "appKey": "spms", "visible": true,  "navItemIds": ["issues", "sprints"] },
    { "appKey": "qbank","visible": false, "navItemIds": [] }
  ]
}}
```

- 壳据此：渲染应用中心（`visible` 过滤，退役 `appVisibility`）、渲染顶级导航（`enabledNavItemIds ∩ navItemIds`）、原生管理菜单（`portal:page:admin.*` 是否命中，而非硬编码 `role==='admin'`）。
- `SideNav` 现有「`role==='admin'` 显示用户/应用/审计/任务」改为按 `portal:page:admin.users/...` 显隐——admin bypass 下行为完全一致，但现在可把单项授予非 admin（统一+灵活）。

### 5.2 微应用前端（iframe）——握手时注入，仅 UX-gate

- 新增 `GET /api/me/permissions/:appKey` → 返回该 App 的 `{ bypass, permissions:[{pid,scope}], groups:[...] }`。
- 门户 host 在 **SDK 握手 `init`** 阶段（与 theme/locale/route 同批，复用 PLAN-4/5 范式）把该结果塞进 init payload；`@xgent/portal-sdk` 暴露：
  - `sdk.acl.can(pid): boolean`、`sdk.acl.scope(pid): DataScope`、`sdk.acl.bypass: boolean`。
- App 用 `sdk.acl.can("spms:action:project.delete")` 决定删除按钮显隐/禁用；用 `sdk.acl.can("spms:page:reports")` 决定内部路由入口。
- **强调**：前端 gate 仅体验；真正拦截在后端（§5.3）。角色变更后壳收到 `aclStamp` 变化可提示「权限已更新，刷新生效」或经路由事件重拉。

### 5.3 独立后端 App（files/spms/sms/qbank-server）——安全真闸

**(a) introspection 扩展**（`POST /api/tokens/introspect`，向后兼容加字段）：

```jsonc
{ "ok": true, "data": {
  "active": true, "kind": "user", "aud": "spms",
  "tenant_id": "...", "user_id": "...",
  "scopes": ["pms.read", "pms.write"],
  "role": "user",                                   // 既有，保留
  "bypass": false,                                  // 新：租户 admin → true（App 可全放）
  "groups": ["<groupId>", ...],                     // 新：用户组（team 数据范围用）
  "permissions": [                                  // 新：该 aud(App) 下已解析有效集
    { "pid": "spms:page:/projects", "scope": "all" },
    { "pid": "spms:action:project.create", "scope": "team" }
  ],
  "aclStamp": "t42.u7",                             // 新：缓存失效戳
  "exp": 1717920000
}}
```
（service-state token：`bypass:false, groups:null, permissions:null`——旁路 ACL，§5.5。）

**(b) App 的 `gate()` 增 `requirePerm`**（复用 `lib/gate.ts` 范式）：

```ts
const claims = await gate(headers, "pms.write");      // 既有：scope 闸
requirePerm(claims, "spms:action:project.create");    // 新：ACL 闸 → 未命中抛 403 INSUFFICIENT_PERMISSION
const scope = permScope(claims, "spms:action:project.create"); // own|team|all
const rows = applyDataScope(query, scope, claims);    // own→ownerUserId / team→groups / all→不限
```

**(c) `POST /api/v1/acl/check`**（批量、可带上下文，给偏好「拉取式判定」的 App）：

```
body: { token, pids: string[], context?: {...} }
→ ok({ results: [ { pid, allowed: boolean, scope } ] })
```

> 推荐主路径用 (a) introspection 内嵌（App 本就在 introspect），(c) 仅用于细粒度/条件判定或不想全量内嵌的场景。

### 5.4 双闸实例（scope ∧ ACL ∧ 数据范围）

| 端点 | scope 闸 | ACL 闸 | 数据范围 |
| --- | --- | --- | --- |
| `POST /spms/projects`（建项目） | `pms.write` | `spms:action:project.create` | — |
| `DELETE /spms/projects/:id` | `pms.write` | `spms:action:project.delete` | `own`→仅删本人建的 |
| `GET /spms/issues` | `pms.read` | `spms:page:/issues` | `team`→仅本组 Issue |
| `POST /qbank/.../publish` | `qbank.review` | `qbank:action:question.publish` | `all` |

### 5.5 服务态令牌（M2M）

- `kind='service'` 无 `user_id` → 不解析 ACL；introspection 回 `permissions:null, bypass:false, groups:null`。
- 仅受 scope + 服务账号 capability 约束（PLAN-4 §3.4/3.5）。App 端：服务态调用走「系统/集成」路径，不做用户 ACL 过滤（或按集成约定固定范围）。

---

## 6. API 契约（门户）

> 统一遵守 CLAUDE.md：业务成功/业务失败 → **200 + 业务结构**；仅未登录(401)/未授权(403)/参数错(400)/传输层故障用 4xx/5xx。列表端点全部**服务端分页**（`Page<T>`，复用 `list-pagination` 范式）。

**租户管理员（会话 + `assertAdmin`，且本身受 `portal:action:acl.manage` 约束、admin bypass）**
```
GET    /api/admin/roles                      列出角色（分页）
POST   /api/admin/roles                      新建角色（可 fromTemplate 克隆）
GET    /api/admin/roles/:id                  角色详情 + 授予
PUT    /api/admin/roles/:id                  改名/描述/sort/status（内置 admin 锁）
DELETE /api/admin/roles/:id                  删角色（builtin 拒；级联清 user_roles/role_grants）
GET    /api/admin/roles/:id/grants           该角色授予
PUT    /api/admin/roles/:id/grants           整集替换授予（diff 写审计）
GET    /api/admin/acl/manifest               聚合「所有已装 App + portal 内置」的清单树（矩阵数据源）
GET    /api/admin/users/:userId/roles        用户已指派角色
PUT    /api/admin/users/:userId/roles        指派角色集合（多选；bump 该用户 uv）
GET    /api/admin/users/:userId/effective    有效权限预览（"以该用户身份预览"）
```

**当前用户（会话）**
```
GET    /api/me/permissions                   见 §5.1
GET    /api/me/permissions/:appKey           见 §5.2（SDK init 用）
```

**Open API / introspection 消费者**
```
POST   /api/tokens/introspect                扩展 permissions/groups/aclStamp/bypass（§5.3a）
POST   /api/v1/acl/check                      批量判定（§5.3c）
```

**审计**：角色增删改、授予变更、指派变更，均 `writeAudit`（复用既有 infra）——`event ∈ {新建角色, 修改角色, 删除角色, 修改角色授权, 指派角色, 移除角色}`，`diff` 记前后。

---

## 7. 租户管理员 UI（强制两步法：`impeccable` 设计 ＋ Chrome 真浏览器验证）

> 这是治理面（PRODUCT.md 原则 4「让人一眼信任的治理」）：精确、诚实于状态、可解释。设计阶段过 `impeccable`，落地后用 `mcp__claude-in-chrome__*` 真浏览器走通主路径与关键边界再报完成（CLAUDE.md 前端两步法）。

### 7.1 角色管理 `/admin/roles`

- 列表（分页）：角色名、key、成员数、内置标、状态；行操作：编辑/克隆/删除（内置 admin 删除禁用并解释）。
- 新建：名称 + key + 描述 +「从模板克隆」（聚合各 App `roleTemplates`，一键铺起步授予）。
- 顶部红点：「N 个新权限项待配置」（reconcile 提示，§2.7）。

### 7.2 权限矩阵编辑器（核心交互 · 纯加法 → 极简）

- 左：App 选择（含 `portal` 内置）；右：该 App 的 **组 → 页面 → 操作** 树（清单 `groups/pages/actions`）。
- 每节点一个**「授予」复选框**：勾=该角色授予此 PID，不勾=未授予（默认拒绝）。**没有「拒绝」态**——避免对称三态诱导的困惑。
- 支持数据范围的已授予节点：旁出 `own/team/all` 选择器（仅清单 `supportedScopes` 给的项）。
- 通配快捷：「全选本页操作」「整个 App 全通（= `appKey:*`）」。
- 危险操作（`dangerous`）红标，提示「授予后该角色成员可执行破坏性操作」。
- 顶部「以某用户预览」：选用户 → 矩阵叠加显示其**有效**判定（其全部角色并集），所见即用户所得；「为何可访问」可展开「来自角色 X 的授予」（纯加法下溯源极简，无 deny 反查）。

### 7.3 成员-角色指派

- 在用户管理（`/admin/users`）每用户加「角色」多选（chips）；保存即 `bump uv`（近实时生效）。
- 旁注「有效权限预览」入口（复用 7.2 的预览）。

### 7.4 设计要点（DESIGN tokens）

- 用既有 token（brand-blue 主操作、危险 danger-500、cool 中性、Space Grotesk / JetBrains Mono 给 PID/key 等技术串）；矩阵密度高但克制（密而静）。
- 空状态：无自定义角色 → 引导「从模板创建第一个角色」；无授予 → 解释默认拒绝。
- 三语 i18n（zh-CN 主，沿用 `labels.ts`）；状态色配图标/文字不靠纯色相（PRODUCT 可访问性）；全键盘可达 + 焦点环；`prefers-reduced-motion`。

---

## 8. 迁移与分期（建议落地顺序 · 非破坏）

> 总原则：**ACL 是叠加的、非破坏的**。任一 App 未接入 PID 校验前，照旧用 introspection `role`；接入后 admin bypass 保证旧「`role==='admin'` 才能写」行为等价。

| 阶段 | 内容 | 验证 |
| --- | --- | --- |
| **A 数据模型** | `roles/role_grants/user_roles` 表 + `apps.aclManifest` 列；每租户 seed 内置 `admin/member`；从 `memberships.role` 回填（admin→admin、user→member 基线）。 | 迁移脚本回填校验：每活跃成员有 member；每 admin 隐式 bypass。 |
| **B 清单契约** | `packages/shared/src/acl.ts` 类型；`portal` 内置清单；为现有 App（spms/qbank/sms/files/todo…）补 `aclManifest` seed + 上架清单列。`GET /api/admin/acl/manifest` 聚合。 | 清单 schema 校验（key 唯一、path 合法、PID 可解析）。 |
| **C 解析引擎** | §4 `resolve/resolveSet`（纯加法并集）+ 数据范围；`aclEpoch`/`aclStamp`；`/api/me/permissions(+/:appKey)`；introspection 扩展；`/api/v1/acl/check`。 | 回归脚本：构造多角色 + 通配 + 数据范围，断言解析结果矩阵（重点验单调性：加角色只增不减）。 |
| **D 壳/SDK 统一可见性** | 壳消费 `/api/me/permissions`，应用中心 + 顶级导航 + 原生管理菜单改为派生；`appVisibility`/`visibility` 迁移成 ACL 授予后退役；SDK init 注入 per-app 集 + `sdk.acl.*`。 | Chrome：不同角色登录看到不同导航/应用/按钮。 |
| **E 管理员 UI** | §7 角色管理 + 矩阵 + 指派 + 预览（impeccable + Chrome）。 | Chrome 真浏览器走通建角色→配授予→指派→以用户预览→该用户实际受限。 |
| **F 各 App gate 接 PID** | 先选 1 个参考 App（建议 spms 或 qbank）在 `gate()` 接 `requirePerm` + 数据范围；验证后推广其余。期间 admin bypass 保旧行为。 | 各 App 既有回归脚本 + 新增「非 admin 受限/admin 全通/数据范围」断言。 |

**回填细则**：现有 `appVisibility`（all/group/user）→ 生成对应 ACL 授予（如 group 可见 → 给一个绑定该 group 的角色 `appKey:page:<landing>` 授予，或并入 member 基线），保证迁移前后「谁能看到该 App」行为不变。

---

## 9. 关键坑位 & 复用点

**坑位（实现期照看）**
- **PID 用 `key` 不用 `path`**：改路由不破授权；改 key=新权限（旧授予置 stale，提示重配）。视 key 为对外契约。
- **新增权限默认不授予**：reconcile 绝不静默授予；只有 `defaultForMember` 项进 member 基线。
- **纯加法、无 deny**：要让某人少权限，只能「给更窄的角色 / 移除某指派」，**不存在**「在宽角色里禁用某项」。文档/UI 要讲清，避免管理员找「拒绝」按钮。
- **admin bypass**：admin 不参与解析、其授予不可编辑、不可删（引导底线）；`acl.manage` 可授他人但 admin 始终全通。
- **缓存失效**：改授予 bump `aclEpoch`、改指派 bump 用户 `uv`；App introspection 缓存键并入 `aclStamp`，否则收权不及时。
- **team 数据范围**：门户必须在 introspection 下发用户 `groups`，否则 App 无法解释 `team`；`own/team/all` 词汇以清单 `supportedScopes` 为准，矩阵不给越界选项。
- **service token 旁路**：`kind='service'` 不解析 ACL（`permissions:null`），App 端走集成路径，别误用用户级过滤。
- **统一可见性别破坏应用中心**：D 阶段务必先把旧 `appVisibility` 完整迁成 ACL 授予再退役，避免「迁移当天大家看不到 App」。
- **跨进程注入对齐**：跨域 iframe 的 init 注入与 CSP/`callService` 代理沿用 PLAN-4/5 范式（cross-origin iframe 对 Chrome 扩展不透明，验证时从父窗口断言）。

**复用点（几乎零新机制）**
- 清单存储/市场快照：照搬 `navItems` + `developerFields()` 快照范式。
- 鉴权下发：复用既有 `introspect`（已回 `role`）+ `gate()` + SDK 握手 `init` 注入。
- 撤销/失效：复用 `bumpUserVersion`/token version 计数器。
- 审计/分页：复用 `writeAudit` + `Page<T>`/`pageParams`。
- 用户组：复用 `userGroups`/`userGroupMembers` 作 `team` 来源。

---

## 10. 需求 → 设计映射（自检）

| 用户原话 | 落点 |
| --- | --- |
| 平台级、基于角色的权限管理体系和服务 | §0.1（平台机制）、§3（数据模型）、§4（引擎）、§5/§6（服务） |
| 供各租户使用（多租户） | 全表 `tenantId`；租户内授权（§0.1 边界）；内置角色每租户 seed（§3.4） |
| 每个 App 提供「所有页面的形式路径」 | `AclManifest.pages[].path`＝路由模式（§2.1/§2.2） |
| 每个 App 提供「操作按钮」 | `AclManifest.actions[]`（§2.3） |
| 「类似 Schema」 | manifest 即 App 自描述权限 Schema（§2） |
| 「类似可被提升到顶级导航的菜单一样，作为 manifest」 | 与 `navItems` 同范式声明 + `page.navItemId` 交叉引用、统一可见性（§1.3/§2.7/§5.1） |
| 租户管理员可添加新角色 | §3（roles）、§6（roles CRUD）、§7.1 |
| 为角色配置各页面/操作的访问权限 | §3（role_grants）、§4（解析）、§7.2 矩阵 |
| 「考虑全面一点」 | 内置 portal 清单 + reconcile + 缓存失效 + 审计 + 迁移 + service token 旁路 + 双闸（§2.8/§2.7/§4.3/§6/§8/§5.5/§5.4） |
| 「灵活一点」 | 多角色并集 + 数据范围 ABAC-lite + 角色模板克隆 + 通配（§4/§2.5/§2.6/§2.4）——扁平纯加法靠「精确授予 + 多角色」组合最小权限 |
```
