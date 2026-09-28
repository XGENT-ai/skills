# 公开页面、站点与应用身份迁移

本文说明当前接入方式；能力是否已部署到目标门户，要用实际接口和浏览器确认。发布审核通过与后端健康都不能代替公开页面验收。

## 1. 先选入口

| 需求 | 入口 | 前提 |
| --- | --- | --- |
| 匿名 SPA、分享页面 | `/apps/<listingKey>/…` | 匿名页不先强制 SDK 登录；读取的数据仍需业务授权 |
| 后端直接返回 HTML、公开回调 | `/svc/<listingKey>/…` | 平台去掉服务前缀；App 自己区分匿名与受保护路由，公开服务应常驻 |
| 租户站点、独立域名 | `publicEntrypoints` + Sites 服务 | 入口声明、服务特权、平台站点配置、租户权益及路由就绪 |

前两条已经是正式接入方式，不需要为了每个新 App 请求门户增加根级路径。
`publicEntrypoints` 是**站点可以绑定的后端入口目录**，不是“在门户根路径新增一条代理”。
仅声明它，不会让门户根级 `/v`、`/share` 等路径指向你的服务。
历史根路径属于兼容契约，需要与平台约定数据迁移和切换，不可自行复用或覆盖。

## 2. 后端直出页必须认识外部路径

例如外部 `/svc/demo/v/abc` 转到后端 `/v/abc`。后端路由可以继续用 `/v`，
但发给浏览器的 URL 必须包含外部 base：公开链接、表单 action、fetch、资源引用、
重定向 Location、口令 cookie 的 Path，以及受保护预览的票据交接。

将这些地址集中从可信的 public base 配置生成；不要只修改“复制链接”字段。
不要信任任意请求自报的 Host 或转发前缀生成外部地址。不同入口的 cookie 名/Path 应避免冲突。
仅换前缀会破坏既有 App 时，应先发布完整路径适配版本，再切换正式链接。

后端 HTML 自己设置 CSP；`embedCsp` 是前端 `/apps/<key>/` 托管策略，不会替代后端公开页的策略。
匿名可达不等于数据公开：token、口令、过期、撤销、租户边界、数据源与提交配额仍由 App 验证。

## 3. 复用已有 Sites 能力

清单片段（合并到自己完整 manifest；特权 reason 写实际用途）：

```json
{
  "publicEntrypoints": [{ "key": "pages", "pathPrefix": "/public" }],
  "privilegedServiceScopes": [
    { "scope": "sites.read", "reason": "查询本应用在本租户的站点和域名就绪状态" },
    { "scope": "sites.write", "reason": "为本应用在本租户创建公开站点并管理域名" }
  ]
}
```

同时在既有 `deployDescriptor` 中设置 `alwaysOn: true`。入口最多 8 条；key 小写字母开头，
只含小写字母、数字、连字符，最长 40；pathPrefix 是后端站内绝对路径，不能是 URL，
不能有 query、fragment、点段、空段或控制字符。末尾斜杠会归一化。
省略 publicEntrypoints 保留既有登记；`[]` 弃用所有入口，会影响引用它们的站点。

申报走发布提案治理审核。`sites.read/write` 是服务专用权限，写进
`privilegedServiceScopes` 申请并由管理员批准，不能放入用户 `scopes` 或普通 `serviceScopes`。

平台须打开 `PUBLIC_SITES_ENABLED`，配置站点 baseDomain/入口 DNS，租户具有
`sites.enabled`、`portal.public_sites`，绑定自定义域时还需 `portal.custom_site_domains`。
没有这些前提，应向平台明确反馈缺哪一项，不能仅因声明批准就发链接。

后端以自己的服务账号，逐租户换取 `client_credentials` 服务票，调用：

1. `POST /api/v1/sites { siteKey, entrypointKey, name? }`，同租户、App、siteKey 幂等。
   响应 Envelope 的 `data.site` 含站点事实；平台地址由返回的 code 与平台配置派生，不硬编码域名。
2. 需要自定义域名时 `POST /api/v1/sites/:id/domains { domain, operationKey }`，
   按返回挑战配置 TXT 和入口 DNS，再 `POST /api/v1/sites/:id/domains/:domainId/check`。
3. 四项 ownership/dns/tls/routing 全过才把该域名视为 ready。
   `GET /api/v1/sites/:id` 回读站点修订，路由 appliedRevision 到达目标 revision 才算送达。
4. 暂停用 `POST /api/v1/sites/:id/suspend`；域名释放和归档按响应的修订做并发检查，
   不靠删 manifest 字段假装已经清理完。

租户与应用从服务票推导，不能用请求体指定其他租户。
站点请求按 pathPrefix 重写并保留 query；App 必须适配站点外部 URL。
边缘路由头用于选站而非登录凭证；站点路径会剥除门户 `xg_sess`，
受保护预览需自己的票据交接，不能依赖门户会话 cookie 跨到独立域名。

## 4. 改 listingKey 是身份迁移

改显示名不需要换 key。确需换 key 时，它是新的 aud、服务身份、安装身份和资源所有者，
不能只重发同一镜像或复制业务数据库。迁移必须覆盖旧公开链接、页面版本、口令/撤销/过期、
文件应用空间与授权、内容 schema、加密字段及新旧写入归属。

在双方确认迁移和回滚前保留旧数据库、服务和路由。不能用 HTTP 404、HTTP 200 或 HTML
标题猜 token 属于哪个服务；“不存在”可能返回 200 占位页。切回路由也不会自动搬回新产生的数据。

## 5. 验收与交接

至少检验：正式匿名页真实正文；有效/口令/过期/撤销；GET 与 POST 子请求；cookie/Location；
有权与错误/过期预览；跨租户拒绝；重部署后地址仍可用。HTTP 200 和 `/health` 通过都不够。

反馈平台时交提案号、镜像/版本、失败路径形状、时间、脱敏错误与浏览器 CSP 指令；
不贴链接 token、口令、服务密钥。区分缺少接入说明、平台能力不足与 App 路径适配问题。

## 6. 扩展 CSP 与交付能力门

开发版本已实现 embedCsp 六类精确来源（connectSrc/scriptSrc/styleSrc/fontSrc/imgSrc/mediaSrc），
以及 platformSources 在 script/style/font/connect 四类中的 jsCdn 别名。它只作用于 /apps 托管前端，
不能替代本后端 HTML 的 CSP。生产来源须 HTTPS（connect 另可 WSS），不可通配、带路径/凭据或
unsafe-inline/unsafe-eval 等关键字；增删均需治理审核。省略沿用、null 清空，对象完整替换。

这些能力尚待正式发布与目标门户受控启用，strict 缺省关闭，不能从已有包版本或 whoami 推断已开启。
支持 delivery 的门户按 proposalId 分别确认审批、清单/产物、实际后端及网关；纯前端也等网关。
unknown 不能报成功，superseded 查询后续提案；旧门户无 delivery 时 CLI 明示无法核验网关。
网关失败由平台管理员单独重试，不必重发镜像。Sites 权限与站点修订仍按本文独立核验。
