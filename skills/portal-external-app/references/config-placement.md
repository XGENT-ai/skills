# 配置放哪一层

一个配置项放错层，后果都不在开发时出现：业务参数进了 `requiredEnv`，每次发版审批人都要对着一个看不懂的键发愁；
本该随时改的配置做成了环境变量，改一次就得换一次版；每租户不同的值写成了全平台一份，第二个租户一来就互相覆盖。
**先按下表判，再写代码。**

## 一张表

| 这个配置是… | 放哪 | 谁改 | 改了要换版吗 |
| --- | --- | --- | --- |
| **环境依赖**：地址、连接串、证书、第三方密钥；全平台一份 | 清单 `requiredEnv`（键名，值归平台）。连接类键用约定名或 `requiredServices[].envKey` 绑定，平台自动供给 | 平台管理员 | 要 |
| **业务参数**：超时、阈值、默认文案、开关；全平台一份、部署后不变 | **先在代码里给缺省值，不进清单**。确实要部署时可覆盖，才进 `requiredEnv`，并且**必须**在 `requiredEnvNotes` 里写清填什么 | 平台管理员（照你的说明填） | 要 |
| **全平台一份、要随时改、不换版** | **平台级管理页**：你自己的页面，清单 `adminPages.platform` 声明，门户嵌进 控制台 › 应用市场 › 编辑你的应用 | 平台管理员 | 不要 |
| **每个租户不同** | micro：清单 `adminPages.tenant` 嵌入 App 页面；service：清单 `tenantConfig` 声明受限 schema，门户渲染动态表单。都进 租户 › 应用管理 › 配置 | 租户管理员 | 不要 |
| **应用内的业务设置**：模板、字典、流程规则… | 你应用里的设置页，用 `aclManifest` 声明权限（不给 `defaultForMember` ⇒ 默认只有租户管理员能进） | 被授权的角色 | 不要 |
| **每个用户的偏好** | 你自己的库，按 `userId` 存 | 用户本人 | 不要 |
| **每租户的密钥**（租户自己的第三方 API key） | 租户级管理页里录入，你的后端加密存库，**只写不回显** | 租户管理员 | 不要 |

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
  "requiredEnv": ["WISH_LIST_DATABASE_URL", "WISH_LIST_REPORT_TITLE"],
  "requiredEnvNotes": { "WISH_LIST_REPORT_TITLE": "导出报表的抬头。不确定就填「心愿单月报」" }
  ```

- 说明会原样给审批人看：**别写任何值或密钥**。

## Service 动态配置（`tenantConfig`）

无前端 service 的普通租户配置不再向平台申请手写页面。在 manifest 声明一个 Tab、同源 GET/PUT 路径、所需 scopes 和受限 schema：

```jsonc
"tenantConfig": {
  "label": { "zh-CN": "解析预算", "en": "Parsing budget" },
  "path": "/v2/policy",
  "scopes": ["omni_parser.read", "omni_parser.govern"],
  "schema": {
    "type": "object",
    "properties": {
      "foreground": {
        "type": "object",
        "title": { "zh-CN": "前程预算", "en": "Foreground budget" },
        "properties": {
          "wall_time_ms": {
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
  `scale` 表示「界面值 × scale = wire 值」，例如秒→1000 毫秒、MiB→1048576 字节；integer 缩放后必须是安全整数。
- 配置 GET 返回 `{ "policy": {…}, "effective": {…} }`。Portal 展示 effective 与继承/覆盖状态；保存前重新 GET 最新
  policy，只合并本次编辑或清除的 schema 路径，再整份 PUT，保留未展示字段。PUT 接收合并后的完整 policy 并返回同一形状。
- 字段缺省时 effective 必须已有应用默认值；旧后端没有返回 schema 所需 effective 字段时，Portal 显示版本不支持并禁止写入。
  App 必须在每次读写时重新校验租户管理员和 scopes，不能只信 Portal 签票或浏览器校验。
- `tenantConfig` 只能由 `type:"service"` 声明，scopes 必须已在 listing `scopes` 中。字段新增、修改和撤回都是治理档；
  批准后按 listing 现值对所有租户生效，不进安装快照。首版不支持动态选项、远程 `$ref`、脚本或密钥录入。

### OmniParser：六项 `foreground` 后端接入清单

OmniParser 的 manifest 已按上面的 `tenantConfig` 形状声明。后端要实现的是**现有 `/v2/policy` 的增量契约**，不要另建配置 API，也不要把 `foreground_limits`（单次请求字段）写成 policy 字段。

| `policy.foreground` / `effective.foreground` 键 | wire 类型与单位 | 无覆盖时的应用默认 | Portal 展示 |
| --- | --- | --- | --- |
| `wall_time_ms` | 整数毫秒 | `10000` | 10 秒（`scale=1000`） |
| `max_input_bytes` | 整数字节 | `16777216` | 16 MiB（`scale=1048576`） |
| `max_result_bytes` | 整数字节 | `4194304` | 4 MiB（`scale=1048576`） |
| `max_nodes` | 整数项 | `100000` | 100000 个节点 |
| `max_container_depth` | 整数层 | `8` | 8 层 |
| `max_embedded_items` | 整数项 | `10000` | 10000 个资源项 |

DTO 规则：

- `policy.foreground` 是**可选、可部分覆盖**的对象；缺整个对象或缺某一键都表示继承。已有 policy 的其他键必须原样保留。
- `effective.foreground` 必须**始终包含全部六键**，值是应用默认、租户覆盖和平台安全界共同决定的当前有效值；不能只回显 raw 覆盖值。它不包含单次请求 `foreground_limits` 的临时收紧。
- 例如，raw 只覆盖等待时间时，GET 仍返回完整 effective：

  ```json
  {
    "policy": { "foreground": { "wall_time_ms": 12000 } },
    "effective": {
      "foreground": {
        "wall_time_ms": 12000,
        "max_input_bytes": 16777216,
        "max_result_bytes": 4194304,
        "max_nodes": 100000,
        "max_container_depth": 8,
        "max_embedded_items": 10000
      }
    }
  }
  ```

后端行为与验收：

1. 沿用现有租户管理员 + `omni_parser.read` / `omni_parser.govern` 双闸和租户隔离，不新增 scope。每次读写都重新判权，不能信任浏览器或 Portal 的预检。
2. PUT 接受合并后的完整 policy，校验六键的类型和实际安全范围；非法或超界时**整份拒绝、原 policy 不变**。成功 PUT 后返回与 GET 同形的最新 `{ policy, effective }`。
3. 清除某个 raw 覆盖键后，该键在 effective 中回到继承值；编辑一个键不得重置其他覆盖键，也不得改变已有 Jobs 的冻结策略、预算或 fragment topology。
4. `budget: EnhancementBudget` 继续独立表示模型调用/费用预算，不能用 `foreground` 替代。实际 Probe / Inspect / FastParse 必须把应用默认、租户 policy、请求 `foreground_limits` 和平台硬界按既定规则取交集；**只把六键加入 DTO 而不接入实际执行路径不算完成**。
5. 旧后端没有 `effective.foreground` 时，Portal 会显示「当前服务版本不支持」且不会 PUT。不要为了制造“支持”假象而在旧版本补一个空对象或静态默认值，然后让实际执行绕开这些限制。
6. 至少验证：完整/部分 raw 的 GET、六键逐一保存与刷新读回、清除覆盖、非法值原样拒绝、未编辑旧键保留、普通成员/无 govern/跨租户拒绝，以及真实执行确实遵守六项限制。

## 管理页嵌入（`adminPages`）

门户不为你写配置界面。你在自己的前端里做好页面，在清单里声明它的路由，门户把它嵌进管理面：

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

- **形状**：`route` 是你应用内的路由（以 `/` 开头，不能 `//`、`..`、编码分隔符）；`label` 是 Tab 名，字符串或
  多语对象（`zh-CN` 必填，≤40 字）。两级都可选，`null` 清空。
- **只有可嵌入的前端应用能声明**：`type` 是 `micro`，`openModes` 含 `embed`。service 型提交即拒。
- **治理档**：新增或改动都会进「发布审核」，因为它决定什么页面以管理员身份跑在门户管理面里。
- **被嵌入时收起你自己的导航**：SDK 握手的 `init.adminPage` 为 `"platform"` / `"tenant"` 时，只渲染那一页，
  不要侧栏、不要顶栏。宿主固定把 `route` 交给你，不同步你的内部路由跳转。
- **平台级的前提**：你的应用要装在平台租户里（每次生产部署会把已上架的应用补装进平台租户，管理员在那里卸载过的除外；没装时 Tab 里会给安装按钮）。平台管理员的
  当前租户不是平台租户时，Tab 里会先让他切换过去，门户不会替他换签票身份。
- **门户读的是 listing 现值**：批准后立刻对所有租户生效，不用等安装快照同步。
- **旧版门户不认识这个字段**：不会拒收，但会把那次发布转人工审一次，字段被忽略。

## 已有的门户手写配置页

文件管理、大模型网关、知识库等早期应用的租户配置页是门户代码里按应用手写的（签票端点 + 配置 API +
配置区块）。那是过渡做法：新 micro App 用 `adminPages`，新 service 的普通配置用 `tenantConfig`；不要再向平台申请这类按应用写的门户代码。
