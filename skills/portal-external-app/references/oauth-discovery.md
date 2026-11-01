# MCP / OAuth 标准发现接入

适用于 App 自己提供 OAuth 元数据，客户端从门户 origin 查找标准发现地址的场景。
`oauthDiscovery` 只配置路由；门户透明转发，不生成 JSON，不实现 App 的授权、token、scope 或客户端回调。
门户用户访问密钥与多服务目录使用 `cliAccess`，见 [CLI 用户密钥](cli-user-keys.md)，两者不能互相替代。

## 前置与声明

目标门户须已部署 OAuth 发现迁移、API/controller 和统一 v2 网关模板（container/process 均支持）。
不支持 v1；不依赖 `PUBLIC_SITES_ENABLED`、Sites 权益或 CSP strict 开关。安装新版 SDK、
whoami 成功、后端 healthy 都不能证明这些平台前置已满足；能力未部署时交平台升级，不逐 App 手改 Caddy。
需要 TypeScript 契约时，`@xgent/shared` 从 0.11.1 提供 `OAuthDiscovery`。

将以下片段合入 App 自己的完整 `app.manifest.json`，递增 `version`：

```json
{
  "oauthDiscovery": {
    "authorizationServer": true,
    "protectedResources": ["/mcp"]
  }
}
```

- 非空声明要求最终清单有 `deployDescriptor` 或 `serviceBaseUrl`，服务须可用。需要常驻时在既有 descriptor 设置 `alwaysOn: true`。
- 只接受 `authorizationServer`（布尔值，省略为 false）与 `protectedResources`（路径数组，省略为空）。仅资源服务器可以只声明后者。
- 路径从 App 服务根起算，填 `/mcp`，不填 `/svc/<key>/mcp`。`/` 表示根资源；其他路径由非空的字母、数字、`.`、`_`、`~`、`-` 段组成。单独的 `.`/`..` 段、空段、尾斜线、URL、百分号编码、query、fragment、空白、通配符均拒绝。
- 路径去重并排序；声明中不能指定 issuer、resource、域名或 upstream。这些元数据值由 App 的响应提供。

## 路径与元数据

下表标准入口位于门户 origin；后端路径是不带 `/svc/<key>` 的 App 内部路径。

| 声明 | 标准入口 | 后端匿名 GET/HEAD 路径 |
| --- | --- | --- |
| `authorizationServer: true` | `/.well-known/oauth-authorization-server/svc/<key>` | `/.well-known/oauth-authorization-server` |
| 资源 `/mcp` | `/.well-known/oauth-protected-resource/svc/<key>/mcp` | `/.well-known/oauth-protected-resource/mcp` |
| 资源 `/` | `/.well-known/oauth-protected-resource/svc/<key>` | `/.well-known/oauth-protected-resource` |

设门户 origin 为 `$PORTAL_ORIGIN`（无尾斜线），App key 为 `$LISTING_KEY`：issuer 应为
`$PORTAL_ORIGIN/svc/$LISTING_KEY`；根资源的 resource 与之相同，`/mcp` 资源在其后加 `/mcp`。
OAuth authorize/token/register 等端点继续使用 App 既有外部地址。原 `/svc/<key>/.well-known/…`
嵌套地址继续可用；网关保留方法、query、Host、响应内容与状态，App 错误不会被修补为有效元数据。

## 提交、审批与生效

1. 使用 App 的发布令牌及目标配置；已有 `.xgent-registry.env` 时让 CLI 自行读取，不打印令牌。缺少配置时从目标门户「开发者应用 → 本 App → 凭证 → 生成配置文件」取得，确认含 LISTING_KEY、TARGET_XGENT_PLATFORM、XGENT_RELEASE_TOKEN，chmod 600 并加入 gitignore。按 [注册与一盒](registration-and-onebox.md) 准备完整清单，再提交：

   ```bash
   npx @xgent/release-cli publish --manifest app.manifest.json
   ```

   HTTP 发布也只能通过 `manifest` 字段携带完整清单，不能提交独立 `oauthDiscovery` 表单字段。
2. 保存 `proposalId`。新增、修改和撤销均按实际差异进治理审核；等价重排/去重不制造差异。待审不改变已批准路由，由平台管理员核对并批准；发布令牌不能代替管理员审批。
3. 用 release-cli ≥0.7.0 查询这一次提案：

   ```bash
   npx @xgent/release-cli status --proposal-id <proposalId> --wait
   ```

   `delivery.state=applied` 才表示本次后端与网关目标已确认。`pending` 等待，`failed` 按失败阶段交平台处理，`superseded` 追踪取代它的新提案；`unknown` 或缺少 delivery 均不能报生效。`publish --wait` 不等待人工审批；需要连审批一起等时用 `--wait-review`。
4. 再做真实 HTTP 与客户端验收。发布成功、清单 applied、后端 ready 均不能代替此步骤。

仅改发现声明不触发后端重部署；网关失败由平台管理员重试网关并核对本提案 revision，不用重发相同镜像。完整发布命令及凭据准备可使用 `xgent-app-release` skill；本参考的契约不要求访问门户源码。

## 验收与排错

从可信的目标配置取得 `PORTAL_ORIGIN`、`LISTING_KEY`，以下请求不携带登录 cookie 或 Bearer token。
按实际声明替换资源路径；`/mcp` 示例：

```bash
curl -i "$PORTAL_ORIGIN/.well-known/oauth-authorization-server/svc/$LISTING_KEY"
curl -I "$PORTAL_ORIGIN/.well-known/oauth-authorization-server/svc/$LISTING_KEY"
curl -i "$PORTAL_ORIGIN/.well-known/oauth-protected-resource/svc/$LISTING_KEY/mcp"
curl -I "$PORTAL_ORIGIN/.well-known/oauth-protected-resource/svc/$LISTING_KEY/mcp"
curl -i "$PORTAL_ORIGIN/svc/$LISTING_KEY/.well-known/oauth-authorization-server"
curl -i "$PORTAL_ORIGIN/svc/$LISTING_KEY/.well-known/oauth-protected-resource/mcp"
```

- 标准与原嵌套地址 GET 均为 200 JSON，字段相同；HEAD 同状态、JSON Content-Type、无正文。核对 issuer/resource 和既有 OAuth 端点，不只检查 HTTP 200。
- 已声明标准入口的 POST/PUT/OPTIONS 为 405，含 `Allow: GET, HEAD`；未声明、撤销、额外尾段为 404。使用门户 origin，站点域名继续走自己的站点上下文。
- 标准地址 404：先查实际获批声明、本次 delivery、服务是否已停止/下架；文件已保存或平台已升级不等于声明已批准。返回 Portal HTML：核对目标 v2 模板及实际加载版本。旧嵌套地址也失败、或返回非 JSON：检查 App 元数据端点。
- MCP 原匿名请求应继续返回约定的 401 与 `WWW-Authenticate`。元数据 HTTP 通过后，还须用目标客户端走通授权页与回调；WorkBuddy / WorkBuddy AI 等不同回调由 App 负责，不能用网关或夹具检查冒称客户端验收。

交付记录包含目标环境、版本、proposalId、delivery、标准/旧路径 HTTP 和客户端结果；不提供凭据或内部 upstream。

## 替换、撤销与生命周期

省略整个 `oauthDiscovery` 保留已批准值；`null` 或 `{}` 明确清空；对象整体替换，未列出的资源被撤销。
递增版本、提交完整清单并批准，等待本次 delivery applied，再确认被撤销的标准 URL 为 404。
撤销发现不删除 App 原元数据端点；移除后端配置时须同时撤销声明。恢复旧声明也须新版本和审批。

声明属于全局 listing，单租户卸载不撤销它。停止服务、下架或删除 listing 会随新网关代撤销路由；
恢复服务后可恢复获批声明。后端换址时发现与 `/svc` 使用同一运行上游，无需在 App 清单硬编码主机或端口。
