# 发布后的公开面、CSP 与交付确认

## 版本与启用前提

本节契约由 `@xgent/shared 0.7.0`、`@xgent/release-cli 0.7.0` 起提供，两包已在私有包仓发布。
装上新版本只代表客户端认识这些字段与交付状态，**不代表目标门户已上线对应能力**：实际使用前回读
目标门户的能力，并以平台发布公告为准。

严格 CSP 校验默认关闭。平台完成历史来源盘点、兼容检查及受控 v2 网关切换后，才可启用扩展声明。
只更新 App 的依赖、看到 whoami 成功或通过本地预检，都不代表服务器已启用。未启用时旧门户仍只接受
connectSrc 兼容契约；不能凭空添加其他指令后宣称生效。预检会按新生产语法检查并提示能力门，不能替平台开启它。

## 入口与权限边界

| 页面 | 正式入口 | 关键责任 |
| --- | --- | --- |
| 前端静态/SPA 匿名页 | `/apps/<key>/…` | 正确 base、嵌套路由刷新、匿名页不先强制登录；数据授权仍由 App 验证 |
| 后端直出 HTML | `/svc/<key>/…` | App 适配 cookie Path、form、fetch、Location、资源与预览；后端自己设置 CSP |
| 站点/独立域名 | `publicEntrypoints` + Sites | 申请 sites 服务特权，满足平台开关、DNS/TLS/路由和租户权益；独立核对站点修订 |
| MCP/OAuth 元数据 | `oauthDiscovery` → `/.well-known/…/svc/<key>` | 平台发现能力、v2 网关、获批声明和匿名元数据；按下节验收 |

Sites 需要平台启用、登记入口和 DNS，租户具有 sites.enabled/portal.public_sites，自定义域还需
portal.custom_site_domains。`sites.read/write` 经 privilegedServiceScopes 申请，不是用户权限。
publicEntrypoints 只声明站点可绑定的后端前缀，不增加门户根代理。新 App 不得占用 `/v`、`/share`。
历史根路径和 listingKey 改名均需另行迁移授权；改链接前缀不能代替 cookie、子请求和数据归属迁移。

## OAuth 标准发现发布

目标门户须已部署发现能力的迁移、API/controller 与两种运行形态对应的 v2 网关模板；不支持 v1。
不要求 `PUBLIC_SITES_ENABLED`、Sites 权益或 CSP strict 开关。`@xgent/shared` 从 0.11.1 提供声明类型，
CLI ≥0.7.0 支持本提案 delivery 查询；包版本和 whoami 成功不能证明目标平台已部署。

合入 App 自己的完整 manifest，递增 version；不要把片段当完整清单发布，也不要作为散表单字段提交：

```json
{
  "oauthDiscovery": {
    "authorizationServer": true,
    "protectedResources": ["/mcp"]
  }
}
```

非空声明要求最终清单有 deployDescriptor 或 serviceBaseUrl。只接受上面两个键，省略的值分别为
false、空数组。资源路径填 App 内部路径，如 `/mcp` 或 `/`，不含 `/svc/<key>` 前缀。
除根 `/` 外，路径由非空的字母、数字、`.`、`_`、`~`、`-` 段组成；禁止单独的 `.`/`..` 段、
空段、尾斜线、URL、百分号编码、query、fragment、空白和通配符。去重排序，不允许指定域名或 upstream。

| 声明 | 门户 origin 上的标准入口 | App 匿名 GET/HEAD 端点 |
| --- | --- | --- |
| authorizationServer=true | `/.well-known/oauth-authorization-server/svc/<key>` | `/.well-known/oauth-authorization-server` |
| protectedResources 含 `/mcp` | `/.well-known/oauth-protected-resource/svc/<key>/mcp` | `/.well-known/oauth-protected-resource/mcp` |
| protectedResources 含 `/` | `/.well-known/oauth-protected-resource/svc/<key>` | `/.well-known/oauth-protected-resource` |

`oauthDiscovery` 是元数据路由，`cliAccess` 是门户用户密钥目录；也不能用 publicClient、publicEntrypoints
替代。App 保持既有 OAuth 与 MCP 鉴权，门户不生成 JSON、不改 issuer/resource、scope、token 或客户端回调。

1. 按本 skill 的配置文件与令牌流程提交 `npx @xgent/release-cli publish --manifest app.manifest.json`。
   声明新增/变更/撤销均按实际差异进入治理审批；等价重排/去重不进审。记录 proposalId，等待平台管理员批准。
   本地 preflight 不校验 oauthDiscovery，也不能证明平台能力；提交成功和 pending 都不能报上线。
2. `npx @xgent/release-cli status --proposal-id <proposalId> --wait` 查询固定提案，要求 `delivery.state=applied`。
   publish 的 `--wait` 不等人工审批，需连审批一起等用 `--wait-review`。失败、unknown、superseded 按下文四阶段处理。
3. 从目标配置取门户 origin（无尾斜线）为 `PORTAL_ORIGIN`、App key 为 `LISTING_KEY`，匿名检查实际声明的地址：

   ```bash
   curl -i "$PORTAL_ORIGIN/.well-known/oauth-authorization-server/svc/$LISTING_KEY"
   curl -I "$PORTAL_ORIGIN/.well-known/oauth-authorization-server/svc/$LISTING_KEY"
   curl -i "$PORTAL_ORIGIN/.well-known/oauth-protected-resource/svc/$LISTING_KEY/mcp"
   curl -I "$PORTAL_ORIGIN/.well-known/oauth-protected-resource/svc/$LISTING_KEY/mcp"
   curl -i "$PORTAL_ORIGIN/svc/$LISTING_KEY/.well-known/oauth-authorization-server"
   curl -i "$PORTAL_ORIGIN/svc/$LISTING_KEY/.well-known/oauth-protected-resource/mcp"
   ```

   GET 须为 200 JSON，标准/旧地址内容一致；HEAD 同状态及 JSON Content-Type、无正文。
   issuer 为 `$PORTAL_ORIGIN/svc/$LISTING_KEY`；根 resource 与之相同，`/mcp` 资源加 `/mcp`。
   核对既有 OAuth 端点不变，匿名 MCP 仍有约定的 401 与 WWW-Authenticate。再由目标客户端验证授权页与回调，
   包括约定的 WorkBuddy / WorkBuddy AI 客户端；HTTP 验收不能冒称客户端通过。

**撤销/回退**：省略字段保留已批准值；null 或空对象清空；对象完整替换，删去资源即撤销该条。
提交新版本并批准、等待本次 delivery applied 后核对撤销地址 404；恢复旧声明也按新版本审批。
仅撤销发现不删除 App 旧元数据端点，移除后端配置须同时撤销声明。单租户卸载不撤全局声明；
停止/下架/删除 listing 随新网关代撤路由，恢复服务后可恢复获批声明。

**分诊**：已声明标准入口的非 GET/HEAD 方法返回 405（Allow: GET, HEAD），未声明/撤销/额外尾段返回 404。
404 先查获批声明、本次 delivery 与服务状态；HTML 查实际 v2 模板/加载版本；旧地址也失败或非 JSON 查 App。
请求使用门户 origin，站点域名继续走站点上下文。后端错误透传，不应靠改 issuer 或硬编码宿主端口绕过。
仅改声明不重部署后端；网关失败交管理员按本提案目标重试，不靠重发相同镜像。报告分别列明平台能力、
声明批准、delivery、HTTP 与客户端验收结果，不要求所有安装此 skill 的门户都已具备能力。

## 六类来源声明

`embedCsp` 仅扩展本 App 的 `/apps/<key>/` 托管前端策略，不改变门户壳、沙箱或后端公开 HTML 的 CSP。
完整 manifest 中可申报：

```json
{
  "embedCsp": {
    "connectSrc": ["https://api.example.com", "wss://events.example.com"],
    "scriptSrc": ["https://components.example.com"],
    "styleSrc": ["https://styles.example.com"],
    "fontSrc": ["https://fonts.example.com"],
    "imgSrc": ["https://images.example.com"],
    "mediaSrc": ["https://media.example.com"],
    "platformSources": { "scriptSrc": ["jsCdn"], "styleSrc": ["jsCdn"], "fontSrc": ["jsCdn"], "connectSrc": ["jsCdn"] }
  }
}
```

来源是精确 origin（协议、主机、可选端口），生产只接受 HTTPS；仅 connectSrc 另接受 WSS。
允许末尾 `/`，不允许路径、query、fragment、凭据、通配符、控制字符、嵌入空白或未知字段。
唯一例外：connectSrc 可原样申报 `blob:`、`data:`（前端 `fetch()` 本地 blob/data URL 时需要），
其余字段不接受，`blob:https://…`、`data:,x` 等带内容的写法也不接受。
不能申报 unsafe-inline、unsafe-eval、self、nonce、hash 等 CSP 关键字，也不能申报其他 scheme 源。
App 不能修改 frame-ancestors、base-uri、object-src；平台既有兼容基线不是可自行申请的权限。
开发环回 HTTP/WS 仅在服务端显式开发模式允许，本发布预检按生产规则拒绝。

platformSources 只允许上述四个指令，值只允许 jsCdn，映射到目标门户当前配置的 CDN origin；
不把某套门户 CDN 域名写死在 App 清单。它不能用于 imgSrc/mediaSrc，也不扩散到其他 App。

省略整个 embedCsp 沿用已批准声明；null 清空额外声明；提供对象是**完整替换**，不是逐指令合并，
未写指令会撤回，空数组表示该指令无额外来源。来源增删、清空和别名变更均按实际差异进入治理审核；
排序、重复项及等价 origin 归一化后不制造变更。若省略后没有差异，不会仅因省略而进审。

## 固定提案的四阶段

1. 审批：pending 等人工；提交成功不等于批准。
2. 清单/产物应用：提案 applied 只说明这一阶段完成，仍检查 applyError。
3. 后端：只接受本次固定配置目标的实际运行证据；最近任务 succeeded 或别的镜像 ready 不代替它。
4. 网关：只接受本次目标摘要和修订匹配的回执；纯前端后端为 not_applicable，仍要等待网关。

支持新 delivery 的门户配合新版 CLI：`publish --wait` 等获批后的后端与网关；pending 仍直接退出 0，
需要等人工决定使用 `--wait-review`。继续追踪用 `status --proposal-id <id> --wait`。
失败或 superseded 非零退出；unknown 不能当成功，继续等待到预算结束仍未确认则非零退出。
旧门户没有 delivery 时只按旧后端规则兼容，并明确提示“无法核验网关”；退出 0 也不声称完整交付确认。

网关失败由平台管理员重试网关配置，不必重发清单，也不触发后端重部署。发布令牌没有管理员重试权限。
API 字段与端点见 [发布 API](publish-api.md)。交付确认仍不是业务可用性的替代：真实验正文、CSP、
正确/错误口令、过期/撤销、表单/数据源、预览权限、跨租户拒绝和重部署后的访问。HTTP 200 可能是占位页。
交接只给提案号、版本、时间、脱敏状态和路径形状，不发 token、环境值或内部上游地址。
