# 配置放哪一层

一个配置项放错层，后果都不在开发时出现：业务参数进了 `requiredEnv`，每次发版审批人都要对着一个看不懂的键发愁；
本该随时改的配置做成了环境变量，改一次就得重启或重部署；每租户不同的值写成了全平台一份，第二个租户一来就互相覆盖。
**先按下表判，再写代码。**

## 一张表

| 这个配置是… | 放哪 | 谁改 | 生效方式 |
| --- | --- | --- | --- |
| **环境依赖**：地址、连接串、证书、第三方密钥；全平台一份 | 清单 `requiredEnv`（键名，值归平台）。连接类键用约定名配合 `requiredServices` 供给；非约定名由管理员绑定来源，不声明 `requiredServices[].envKey` | 平台管理员 | 修改运行环境后重启/重部署，无须为改值重建镜像 |
| **业务参数**：超时、阈值、默认文案、开关；全平台一份、部署后不变 | **先在代码里给缺省值，不进清单**。确实要部署时可覆盖，才进 `requiredEnv`，并且**必须**在 `requiredEnvNotes` 里写清填什么 | 平台管理员（照你的说明填） | 修改运行环境后重启/重部署，无须为改值重建镜像 |
| **全平台一份、要随时改** | **平台级管理页**：清单 `adminPages.platform` 声明 schema 表单或 App 嵌入页，入口在 控制台 › 应用市场 › 编辑你的应用 | 平台管理员 | 保存后由 App 应用，无须重新部署 |
| **每个租户不同** | micro/service：`adminPages.tenant` 或 `tenantConfig` 声明 schema；micro 也可用 `adminPages.tenant.route` 嵌入 App 页面。入口在 租户 › 应用管理 › 配置 | 租户管理员 | 保存后由 App 应用，无须重新部署 |
| **应用内的业务设置**：模板、字典、流程规则… | 你应用里的设置页，用 `aclManifest` 声明权限（不给 `defaultForMember` ⇒ 默认只有租户管理员能进） | 被授权的角色 | 保存后由 App 应用，无须重新部署 |
| **每个用户的偏好** | 你自己的库，按 `userId` 存 | 用户本人 | 保存后由 App 应用，无须重新部署 |
| **每租户的密钥**（租户自己的第三方 API key） | 租户级管理页里录入，你的后端加密存库，**只写不回显** | 租户管理员 | 保存后由 App 应用，无须重新部署 |

另外几类**根本不是配置**，别给它们留任何入口：

- **能从平台推出来的值**：门户地址与别的 App 的服务地址（`${PORTAL_BASE_URL}/svc/<key>`）、租户 ID（令牌的
  `tenant_id`，前端 `init.tenant`）、界面语言（`init.locale`）、主题。
- **按套餐开关的功能**：走 `entitlements`（套餐与授予），不是你应用里的一个开关。
- **通知偏好**：走平台通知（清单 `notificationEvents`），用户在平台统一管理，不自建。
- **前端构建产物里的环境值**：同一份 dist 会发到好几套门户，**不要用 `VITE_*` 在构建时写死任何环境相关的值**；
  运行时从 SDK 握手（`init`）或你自己的后端拿。

## 一条大原则：审批人只懂环境

审批你发版的平台管理员负责的是**环境**：这台机器、这个网络、这些服务和密钥。你的业务参数他不懂，
也**不该被要求懂**。所以：

- 清单里 `requiredEnv` 的每一个键，批准前都**必须有值**（缺了就是 `REQUIRED_ENV_MISSING`，提案留待审）。
  业务参数进了这张表，就等于要求审批人替你做一个业务决定。
- 真要进，就在 `requiredEnvNotes` 里给它写一句「填什么、不确定时填什么」（≤200 字，只写给需要解释的键），
  审批屏会把它贴在键名旁：

  ```jsonc
  "requiredEnv": ["MY_APP_DATABASE_URL", "MY_APP_WORKER_CONCURRENCY"],
  "requiredEnvNotes": { "MY_APP_WORKER_CONCURRENCY": "部署时固定的 worker 数量；不确定时填 4。" }
  ```

- 说明会原样给审批人看：**别写任何值或密钥**。

## schema 组件和管理页分别在哪里

平台/租户决定权限与数据作用域；schema/嵌入页决定呈现方式。

| 场景 | 清单声明 | 谁渲染配置项 | 支持范围 |
| --- | --- | --- | --- |
| 平台管理员配置 | `adminPages.platform: { label, path, scopes, schema }` | 门户通用表单 | micro/service，不要求 embed |
| 租户管理员配置 | `adminPages.tenant: { label, path, scopes, schema }` | 门户通用表单 | micro/service，不要求 embed |
| 独立租户配置 Tab | `tenantConfig: { label, path, scopes, schema }` | 同一门户通用表单 | micro/service |
| 复杂管理页面 | `adminPages.platform/tenant: { route, label }` | App 前端，门户 iframe 嵌入 | micro + embed |
| 既有 Skills revision/CAS 设置 | `tenantConfig.protocol: "skills-settings-v1"` | 门户协议适配组件 | micro/service，不带 schema |

同一层管理页选择 schema 或 route，不能混写。平台与租户可以分别选不同模式。普通数字、枚举、开关配置优先 schema；需要文本、密钥或复杂业务交互时用 App 页面。同一个租户设置不要在 `tenantConfig` 和 `adminPages.tenant` 重复声明，否则会出现两个 Tab。

实现定位（均为门户仓文件，外部项目按本文合同接入）：

- `packages/shared/src/tenant-config.ts`：`AppTenantConfig` 及字段类型；`dto.ts`：`AppAdminPageDecl = AppEmbeddedAdminPage | AppTenantConfig`。
- `apps/api/src/modules/market/{tenant-config,admin-pages}.ts`：字段归一、类型与权限准入。
- `apps/api/src/modules/apps/tenant-config-ticket.ts`：同源管理票及平台/租户授权。
- `apps/web/src/pages/admin/AppConfigSection.tsx`：两级复用的动态表单；`apps/web/src/lib/tenant-config-api.ts`：读写、单位换算与 policy 合并。
- `apps/web/src/components/apps/AppAdminPagePanel.tsx`：按 schema/route 分派；入口分别是 `MarketListingForm.tsx` / `AppForm.tsx`。

## 通用 schema 配置

micro 和 service 均可声明一个 Tab、同源 GET/PUT 路径、所需 scopes 和受限 schema，无须为 App 新增门户页面代码。以下字段结构既适用于 `tenantConfig`，也适用于 `adminPages.platform` / `adminPages.tenant`。

以下为虚构服务 `my-service` 的示例，仅演示分组、单位换算与字段类型；路径、权限、业务字段和默认值由各 App 自行定义：

```jsonc
"tenantConfig": {
  "label": { "zh-CN": "任务设置", "en": "Task settings" },
  "path": "/v1/policy",
  "scopes": ["my-service.read", "my-service.admin"],
  "schema": {
    "type": "object",
    "properties": {
      "execution": {
        "type": "object",
        "title": { "zh-CN": "执行设置", "en": "Execution settings" },
        "properties": {
          "timeout_ms": {
            "type": "integer", "title": { "zh-CN": "最多等待", "en": "Wait limit" },
            "default": 10, "unit": { "zh-CN": "秒", "en": "s" }, "scale": 1000
          },
          "mode": {
            "type": "select", "title": "模式", "default": "auto",
            "options": [{ "value": "auto", "label": "自动" }, { "value": "fast", "label": "快速" }]
          },
          "strict": { "type": "boolean", "title": "严格模式", "default": false }
        }
      }
    }
  }
}
```

- 支持 `integer` / `number`、`select` 静态 options、`boolean` 和 object 嵌套分组；标签、说明与单位支持多语。
  `scale` 表示「界面值 × scale = wire 值」，例如秒→1000 毫秒、MiB→1048576 字节；integer 缩放后必须是安全整数。`default` / `minimum` / `maximum` 使用界面单位；后端 policy/effective 使用 wire 单位。
- 配置 GET 返回 `{ "policy": {…}, "effective": {…} }`。Portal 展示 effective 与继承/覆盖状态；保存前重新 GET 最新
  policy，只合并本次编辑或清除的 schema 路径，再整份 PUT，保留未展示字段。PUT 接收合并后的完整 policy 并返回同一形状。
- 字段缺省时 effective 必须已有应用默认值；旧后端没有返回 schema 所需 effective 字段时，Portal 显示版本不支持并禁止写入。
  schema 的 `default` 不替后端生成 effective，也不自动保存默认值。App 必须在每次读写时重新校验对应层级的管理员和 scopes，不能只信 Portal 签票或浏览器校验。
- schema 声明支持 `type:"micro"` 和 `type:"service"`，scopes 必须已在 listing `scopes` 中。字段新增、修改和撤回都是治理档；
  批准后按 listing 现值对所有租户生效，不进安装快照。不支持自由文本/string、数组、动态选项、远程 `$ref`、脚本或密钥录入。字段最多 100 个；object 从根深度 0 起最多到 4，select 最多 100 个 options，scopes 为 1–20 项。

### 既有 revision/CAS 设置（`skills-settings-v1`）

已有 Skills 设置线协议的 micro/service 可复用 `tenantConfig`，无需新增门户 key 分支。新配置使用 schema 或嵌入页；此协议只适配既有的设置格式。

```jsonc
"tenantConfig": {
  "protocol": "skills-settings-v1",
  "label": { "zh-CN": "Skills 设置", "en": "Skills settings", "zh-TW": "Skills 設定" },
  "path": "/v1/settings",
  "scopes": ["my-app.read", "my-app.admin"]
}
```

- `path` 固定为 `/v1/settings`；不带 `schema` 或其他字段。必须恰含同一自有命名空间的 read/admin，命名空间为 listingKey 或其连字符转下划线形态；两项须在 manifest 自身 `scopes` 中显式声明。重复、混合命名空间、依赖或平台权限均拒绝。
- GET/PUT 沿用 Skills 的 revision/CAS 合同；不会转换为上节的 policy/effective。PUT 使用 `expectedRevision` 和最小 patch，冲突读回再处理；丢响应后先 GET 确认，不自动重放 PUT。存储、业务校验、审计及严格自省仍由 App 负责。
- 通用 `GET /api/admin/apps/:appKey/tenant-config-token` 必须由当前管理员会话携带 `expectedUserId` / `expectedTenantId`，返回 `{ token, expiresIn: 600, serviceBase }`。本安装 appKey/contentOwner/来源 listingKey 必须一致，且安装仍有两项 scope；只支持 canonical `/svc/<listingKey>`（可带单尾斜杠，返回归一地址）。App 请求只带 Bearer、不带 Portal cookie。
- 旧 `skills-token` 入口仅对本协议适配为 `skillsBase` 响应，同样现读获批声明，无旧 key 兜底。声明省略保留、null 撤回、完整对象替换，均沿既有治理；null 立即停止新票，已出票沿现有 JTI/资格撤销及最长 600 秒边界，不承诺声明撤回本身即时吊销全部旧票。
- 批准声明不补安装 scope、角色 grant 或 consent。全部外部 listing 的自动版本/品牌同步均保留安装 scopes；新增权限须租户管理员显式调用 `POST /api/admin/market/listings/:key/update`（版本已相同也可调用）。新安装取当前获批 scopes，内置定义同步仍 union。
- 旧 null 不自动恢复；平台有权者须在支持新协议的 API 上明确治理恢复。先升级 Portal API，再提交并批准 bump 后的正式清单，最后升级宿主前端；声明样例不是已批准事实。
- `@xgent/shared` 联合类型从 0.11.0 起提供（实际可安装状态以门户发布回执为准）：先用 `isSkillsSettingsConfig` 分派，再访问旧 `AppTenantConfig.schema`。所有 DTO 消费方均须加守卫。

## 两级管理页（`adminPages`）

schema 示例（字段定义遵循上一节；外层 manifest 的 `scopes` 还须包含两级声明的权限）：

```jsonc
"adminPages": {
  "platform": {
    "label": "平台默认值", "path": "/v1/platform-settings", "scopes": ["my-app.admin"],
    "schema": { "type": "object", "properties": { "enabled": { "type": "boolean", "title": "默认启用", "default": true } } }
  },
  "tenant": {
    "label": "租户设置", "path": "/v1/settings", "scopes": ["my-app.read", "my-app.admin"],
    "schema": { "type": "object", "properties": { "enabled": { "type": "boolean", "title": "启用", "default": true } } }
  }
}
```

- 两级 schema 共用 GET/PUT `{ policy, effective }` 协议，浏览器请求 `/svc/<listingKey><path>`。门户不保存业务配置值；App 的平台端点保存全局值，租户端点按 `tenant_id` 隔离。平台租户只是运行身份，不能把平台默认值误存为仅该租户可见的配置。
- 门户向 `GET /api/admin/apps/:appKey/admin-page-token?level=platform|tenant` 请求管理票，同时携带 `expectedUserId` / `expectedTenantId`。平台级额外要求平台管理员及当前平台租户；两级均要求有效安装、listing 来源一致、声明仍有效、安装 scopes 覆盖声明 scopes。返回 `{ token, expiresIn: 600, serviceBase }`，浏览器只向规范同源服务发送 Bearer，不带 Cookie。
- `tenantConfig` 的 schema 继续走 `tenant-config-token`，权限与租户级 schema 一致。声明撤回立即停止新票，已签票遵循自省资格检查与最长 600 秒有效期。
- `@xgent/shared` 从 0.12.0 提供管理页联合类型。消费方用 `"schema" in page` 分派后再访问 `page.schema` 或 `page.route`；仅 iframe 的组件使用 `AppEmbeddedAdminPage`。需先升级 Portal API 和宿主前端，再提交并批准新声明；升级类型包本身不会升级门户运行件。

需要自定义页面时仍可使用嵌入模式：

```jsonc
"adminPages": {
  "platform": { "route": "/admin/model-knowledge", "label": { "zh-CN": "模型知识", "en": "Model knowledge" } },
  "tenant":   { "route": "/settings", "label": "应用设置" }
}
```

| | 平台级 `platform` | 租户级 `tenant` |
| --- | --- | --- |
| 出现在 | 控制台 › 应用市场 › 编辑你的应用，多一个 Tab | 租户 › 应用管理 › 配置，多一个 Tab |
| 以谁的身份运行 | **平台租户**里的平台管理员（你那一份装在平台租户里的实例） | 当前租户的管理员 |
| 你的服务端怎么判权 | 自省结果 `isPlatformAdmin === true`（且 `tenant_scope === "platform"`）；数据存「不分租户」的那一份 | 令牌的 `tenant_id` + 管理员（`role` / `bypass`）；按租户存 |

- **嵌入模式形状**：`route` 是你应用内的路由（以 `/` 开头，不能 `//`、`..`、编码分隔符）；`label` 是 Tab 名，字符串或
  多语对象（`zh-CN` 必填，≤40 字）。两级都可选，`null` 清空。
- **route 模式只支持可嵌入前端**：`type` 是 `micro`，`openModes` 含 `embed`。service 可声明 schema，不能声明 route。
- **治理档**：新增或改动都会进「发布审核」，因为它决定什么页面以管理员身份跑在门户管理面里。
- **被嵌入时收起你自己的导航**：SDK 握手的 `init.adminPage` 为 `"platform"` / `"tenant"` 时，只渲染那一页，
  不要侧栏、不要顶栏。宿主固定把 `route` 交给你，不同步你的内部路由跳转。
- **平台级的前提**：你的应用要装在平台租户里（每次生产部署会把已上架的应用补装进平台租户，管理员在那里卸载过的除外；没装时 Tab 里会给安装按钮）。平台管理员的
  当前租户不是平台租户时，Tab 里会先让他切换过去，门户不会替他换签票身份。
- **门户读的是 listing 现值**：批准后立刻对所有租户生效，不用等安装快照同步。

## 已有的门户手写配置页

文件管理、大模型网关、知识库等早期应用的租户配置页是门户代码里按应用手写的（签票端点 + 配置 API +
配置区块）。那是过渡做法。当前已发布契约的接入按上表选择；新能力优先扩展通用声明与组件，不再向平台申请按应用写的门户代码。


## 配置接入验收

按本次涉及的配置层级检查：

- 环境依赖：清单键名与镜像实际读取一致，值不进入 manifest/dist；已获批运行件具备所用供给能力。
- 平台配置：租户管理员不能操作；保存全局值后在实际业务路径生效，不误存为平台租户的一份租户配置。
- 租户配置：A 租户修改不影响 B；非法值拒绝且原值不变；保存/刷新读回一致。schema 表单还要验证单位换算、清除覆盖恢复继承，以及保留未展示字段。
- UI：从门户相应管理入口进入；管理页只展示当前配置内容，权限由后端重新校验。
