---
name: portal-external-app
description: '用于开发、接入或排查 XGENT Portal App：后端鉴权、iframe 微应用、平台与租户配置、manifest、跨 App 交换和日志接入。支持独立仓库 App 与门户仓内后端；发布操作用 xgent-app-release，一盒环境维护用 portal-dev-setup。'
---

# portal-external-app · Portal App 开发与接入

覆盖外部镜像服务、iframe 微应用及门户仓内独立后端。先确定代码归属、配置层级和调用身份，再只读当前任务相关参考；不因接入任务扩展为发版或重建门户。

**路径约定**：本 skill 可整目录拷到任何 repo。`references/`、`scripts/` 相对本 skill 根目录；`app.manifest.json` 属于目标 App 仓库。参考中的 `apps/`、`packages/`、`deploy/` 是门户仓内实现位置，外部项目没有这些路径，不需要克隆门户或创建同名文件。`/apps/<key>/` 与 `/svc/<key>/` 是运行时 URL。

## 配置先分层（新增或调整配置时必读）

| 配置归属 | 当前版本接入方式 |
| --- | --- |
| 数据库、网络地址、证书、部署密钥等启动环境依赖 | `requiredEnv` 声明键与可选生成规格，值由平台供给/注入或管理员维护；修改运行环境需要重启/重部署，不要求为改值重建镜像 |
| 全平台共享、需要随时调整的运行配置 | 平台管理员配置页 `adminPages.platform`；App 存全局值、后端校验 `isPlatformAdmin` |
| 每租户不同的运行配置 | micro/service 用 `adminPages.tenant` 或 `tenantConfig` 声明 schema，也可由 micro 嵌入自定义页，App 按可信 `tenant_id` 存取 |
| 业务模板、字典、流程等应用业务设置 | App 内设置页，由 ACL 授权 |
| 用户偏好 | 按用户保存，不写全平台环境变量 |

业务参数先给代码默认值；确需部署时覆盖才列 `requiredEnv`，并写 `requiredEnvNotes`。可从 SDK/门户基址推导的身份、语言、主题与服务地址不另设人工配置。配额/套餐走平台治理。

**配置归属与呈现方式分开选择**：`adminPages.platform` / `.tenant` 均支持 `{ label, path, scopes, schema }`，micro/service 使用同一个门户表单；`tenantConfig.schema` 也支持两种类型。复杂页面可继续声明 `{ route, label }`，此模式才要求 micro + embed。平台值存全局并校验平台身份，租户值按租户隔离。完整合同和运行件升级要求见下文参考。

配置项判断表、完整声明示例、GET/PUT 合同、组件与校验器位置见 [配置分层与 schema 配置](references/config-placement.md)。涉及配置时先读它，不照抄其他 App 的参数或默认值。

## 先判归属与身份

- **外部 App**：清单事实源在 App 仓库，生产通过发布提案注册；不得给它新增门户 `LISTING_DEFS` / ACL 注册表副本。后端任意语言、独立镜像交付；有前端用 `micro`，无前端用 `service`。
- **门户仓内后端**：仅当前任务明确维护仓内 `apps/*-server` / `*-app` 时读 [仓内后端](references/backend-app.md)。仓内注册、dev:all、部署接线不能套用到已出仓 App。
- **代表门户成员**：自己前端用 SDK；代表用户调另一 App 用令牌交换。**后台代表 App** 用服务账号。写入前先判断数据归用户还是归 App，见 [跨应用交换](references/app-exchange.md)。
- **顾客/学生等终端用户**：走独立的 `kind=enduser` 契约，见 [终端身份](references/end-users.md)，不套成员 OAuth、角色或席位。

## 按任务读取

| 任务 | 参考 |
| --- | --- |
| iframe 前端、导航/主题/路由、SDK 握手和真实浏览器验收 | [微应用开发](references/micro-app.md)，API 细节按其中链接读取 |
| 跨 App 调用、Token Exchange、consent / scope 排错 | [跨应用交换](references/app-exchange.md) |
| 门户 monorepo 内的独立后端开发、数据库与部署接线 | [仓内后端](references/backend-app.md) |
| 日志与遥测写入、采集器、流名和查询排错 | [日志接入](references/logging.md)；采集配置与端点契约按其中链接读取 |
| 外部镜像交付、迁移、requiredEnv / requiredServices、品牌图 | [交付与环境供给](references/external-delivery.md) |
| 判断已有平台能力：文件、通知、审计、用量、Widget、配额 | [平台能力](references/platform-capabilities.md) |
| 服务账号签票、租户策略、管理控制面与撤销 | [服务令牌](references/service-tokens.md) |
| 实现/评审资源服务器（四道闸、health、env、认证面划界、配置面） | [references/integration-contract.md](references/integration-contract.md) |
| 实现 `/health`、出仓迁移、发布镜像或健康检查失败（必读） | [references/health-contract.md](references/health-contract.md)：两种正确实现、实际判据、发布前检查命令 |
| 写 manifest、注册布线、一盒联调、排查 | [references/registration-and-onebox.md](references/registration-and-onebox.md) |
| 匿名页、后端直出 HTML、公开站点/独立域名、旧链接或 listingKey 迁移 | [references/public-entrypoints.md](references/public-entrypoints.md)：现有 /apps、/svc、Sites 三种通路及各自责任 |
| 将本 App 的图片/视频/PDF 发布为不过期、可吊销的固定内容 URL | [references/files-stable-links.md](references/files-stable-links.md)：特权 scope 申请、服务态签发、轮询、公开读取与生命周期 |
| 服务账号写进应用空间的文件，要让作者本人在文件管理里看得到 | [references/files-member-docs.md](references/files-member-docs.md)：`onBehalfOf` 署名、成员只读可见、成员内容隐私开关 |
| 门户成员的原生客户端（iOS/Android/桌面）登录与刷新、设备凭证、停用后失效时效、服务端成员查询与事件 | [references/native-clients.md](references/native-clients.md) |
| 顾客/学生等终端用户的完整能力、租户用户中心、托管注册登录、账号与隐私治理 | [references/end-users.md](references/end-users.md)：先区分成员与终端身份，再按任务读认证协议或生命周期接口 |
| CLI、MCP、个人自动化使用门户用户访问密钥，或接入多服务用户密钥目录 | [references/cli-user-keys.md](references/cli-user-keys.md)：受审 `cliAccess` 声明、标准绑定与通用目录 |
| MCP/OAuth 客户端发现元数据，或标准 `/.well-known/…/svc/<key>` 返回 HTML/404 | [references/oauth-discovery.md](references/oauth-discovery.md)：`oauthDiscovery` 声明、匿名元数据、审批、固定提案交付与撤销 |
| 为外部服务写/审对接契约文档 | [references/contract-doc-template.md](references/contract-doc-template.md) |
| 有「每租户最多几个 X」的配额诉求（选模型、数值谁配、已用怎么来） | [references/quota-and-seats.md](references/quota-and-seats.md) |

## 共用约束与验收

- 资源后端按 [集成契约](references/integration-contract.md) 自省及四道闸授权；Bun/TypeScript 使用 `@xgent/portal-server-sdk`。前端 ACL 只控制展示，平台管理员身份只认服务端自省。
- `@xgent/*` 从平台私有 npm 安装，不 vendor、不以 file/link/tar 顶替；安装步骤读 `xgent-app-release` 的 `references/private-npm.md`。缺凭证时获取开发者配置，不能改写 SDK 来绕过安装。
- `/health` 按 [健康契约](references/health-contract.md) 验证平铺/信封与依赖故障。业务接口使用门户响应信封，数据不存在不返回路由 404。
- 独立 App 本地联调复用 `portal-dev-setup` 的共享 onebox；仓内开发遵守 `bun run dev:all`。不要混连两套环境。
- 验收跟随本次改动：后端验证真实授权、租户隔离与业务结果；UI 从门户宿主进入真实浏览器验证主路径和关键边界；配置保存后读回并核对实际执行，不能只证明表单能显示；日志核对写入响应和查询落点。
- 发布操作使用 `xgent-app-release`，按本次 proposal 的审批、部署及网关证据判断交付；源码具备能力不等于目标门户已发布或启用。说明未验证项，不把未运行的检查写成通过。
