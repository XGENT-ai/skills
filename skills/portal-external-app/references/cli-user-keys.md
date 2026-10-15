# CLI 与用户访问密钥接入

适用于用户以本人身份从 CLI、MCP 或个人自动化调用 App。人在手机、桌面客户端上登录仍走 [原生客户端授权码与 PKCE](native-clients.md)；无人值守的租户程序走服务账号。

## 清单声明

先确认目标门户已部署支持 `cliAccess` 的数据库迁移、API 和授权页。共享包中的类型存在，不表示某个门户环境已启用能力。旧门户批准一个未知字段，也不证明该字段已进入运行时目录。

在 App 自己的唯一 manifest 中声明并 bump version，通过原有发布提案审批到达；不申请修改门户的 `CLI_SERVICES`、权限枚举或授权表单。

```json
{
  "listingKey": "my-tool",
  "scopes": ["my-tool.read", "my-tool.publish"],
  "scopeLabels": {
    "my-tool.read": { "zh-CN": "读取本人有权访问的文档", "en": "Read documents available to me" },
    "my-tool.publish": { "zh-CN": "发布本人有权发布的文档", "en": "Publish authorized documents" }
  },
  "cliAccess": {
    "version": 1,
    "scopeCeiling": ["my-tool.read", "my-tool.publish"],
    "requiredScopes": ["my-tool.read"],
    "defaultScopes": ["my-tool.read"],
    "maxTtlDays": 90,
    "bindPath": "/api/v1/user-keys/bind",
    "endpoints": [
      {
        "id": "client",
        "protocol": "my-tool-v1",
        "path": "/api/v1/client",
        "requiredScopes": ["my-tool.read"]
      }
    ]
  }
}
```

- `scopeCeiling` 是用户密钥的独立上限，只含本 App 已声明的自有命名空间权限。主 `scopes` 中的 Files 等依赖权限、平台权限不能照搬进来。管理权限也不会自动进入 CLI 上限。
- `requiredScopes` 为必选项，`defaultScopes` 为初始选择；两者都须是上限的子集，默认选择须包含必选项。实际签发还受安装范围、当前主体资格和用户明确同意限制。
- `maxTtlDays` 为 1–365 的整数；App 在绑定时仍须检查自己的期限和业务限制。
- `bindPath`、端点 `path` 是 App 后端内的绝对路径，由门户拼到受信任的 `/svc/<listingKey>` 下。不能填写 URL、双斜线、路径穿越、编码分隔符、查询或片段。
- `endpoints` 可为空。`id` 在同一 App 内唯一；`protocol` 是稳定的协议标识，可以由多个 App 实现，不是 App 身份。端点必需权限须在 CLI 上限内。

新增、修改、显式 `null` 撤回均属治理变更，pending 时不改变生效目录。省略 `cliAccess` 沿用原值；对象完整替换；`null` 停止新 CLI 授权和服务发现。声明只在 listing 保留一份，不要求逐租户复制。

## 标准绑定协议 v1

门户生成 pending 密钥后，使用仅限该目标及本次所选权限的用户 TDT 调用：

```http
POST /svc/<listingKey><bindPath>
Authorization: Bearer <目标绑定 TDT>
Content-Type: application/json

{"credentialId":"<pending 密钥 UUID>"}
```

App 不接收密钥明文，也不从 body 接收用户、租户、权限或业务授权。绑定端点只接纳直接用户 TDT，拒绝服务身份、交换身份和用户访问密钥；身份取自门户自省，并执行自己的 scope、PID、租户与业务资格校验。

Bun/TypeScript 使用已配置的 `createPortalUserKeyClient`；配置中的 `userKeysUrl` 为可信门户地址下的 `/api/tokens/user-keys`，不能来自请求。

```ts
await userKeys.bindExisting(tdt,
  { userId: actor.userId, tenantId: actor.tenantId, appKey: actor.aud },
  credentialId,
  async (key, confirm) => {
    const binding = await database.transaction(async tx => {
      // 检查期限、所选权限和业务资格；幂等写入本地绑定。
      return bindLocalPolicy(tx, actor, key);
    });
    await confirm();
    return binding;
  });
```

先提交本地事务，再在事务外确认门户目标。确认失败时保留尚不可用的本地记录，以同一 `credentialId` 重试；重复绑定不得扩大原策略。所有目标绑定结束后，门户只激活成功目标，并向用户一次性展示明文。没有成功目标时不得返回可用密钥。

App 的项目、文件或资源白名单由 App 自己维护；复杂设置放在 App 自己的管理面。清单不包含可执行插件、任意请求模板或业务表单 schema。

## 使用与发现

机器入口须显式接纳 `user_access_key`。每次请求先验证门户自省的当前主体资格，再按 `credential_id` 检查本 App 已提交的绑定、资源范围与业务规则；普通 REST 不因登记 CLI 能力而自动接纳长期密钥。

客户端用密钥调用 `GET /api/access-keys/self`，读取 `endpoints`：

```ts
const candidates = self.endpoints?.filter(item => item.protocol === "my-tool-v1") ?? [];
const target = candidates.find(item => item.appKey === configuredAppKey);
// 多提供方时由用户或客户端配置明确选择。
```

每项包含安装代次相关的稳定 `id`、实际 `appKey`、`listingKey`、名称、协议、同源 `url` 与有效权限。没有匹配项或字段缺省时，客户端报告能力不可用；不能猜测服务地址。请求携带 bearer 时禁止跟随重定向。

`services.files/spms/llm/skills` 是旧客户端兼容视图；新增 App 不为它新增字段。`xgent-skills-v1` 表示既有 Skills 客户端协议，仅有一个有效提供方时可投影到 `services.skills`；多提供方使用通用目录。能力名与 listingKey 分别演进，不能把旧权限名无条件映射成新权限。

撤回声明不会替用户自动撤销已激活的密钥，也不改变 App 直接委托签发的凭证合同。要停用已有密钥，使用门户撤销或 App 移除目标的既有流程；卸载、主体失格和 consent 撤回继续按当前授权事实生效。

## 联调验收

1. 用未在门户代码登记过的 App key，经 manifest 注册和安装后，它出现在当前用户可授权的目录中；pending 提案不改变生效目录。
2. 必选和默认权限按声明显示，权限文案来自 `scopeLabels`；未同意、越界、缺必选权限、超过期限时不能签发。
3. 完成 pending → 目标绑定票据 → 本地提交与确认 → 激活 → 自省与服务发现。验证部分绑定失败和全部失败，失败 App 不可使用密钥。
4. pending 期间撤回声明、撤回 consent 或卸载 App，后续绑定和激活不得继续授予该目标；既有 App 直接签发密钥不被 CLI 声明撤回误伤。
5. 实际访问机器入口，验证跨租户、无本地绑定、资源范围越界、撤销密钥及重复绑定。类型和 SDK 单测不代替这条端到端链路。
