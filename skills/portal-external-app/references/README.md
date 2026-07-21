# XGENT.ai Portal

XGENT.ai Portal 是一个面向多租户 SaaS 的平台基座，用来承载不断增长的业务 App、统一身份与权限、提供平台级服务，并把 App 接入、部署、治理、运行期集成收敛到一套稳定的协议和控制面。

这个仓库不是某一个 App 的代码库，而是一个持续演进的平台工程。当前已经有多种 App 接入平台，后续还会继续增加；根 README 只描述平台基座和平台级服务，具体 App 的业务说明、接入细节和联调方式放在各自文档或 App 目录中维护。

## 项目目标

- 提供统一的门户 Shell：登录、租户切换、侧边导航、仪表盘、通知中心、个人设置、管理控制台和平台控制台。
- 提供多租户身份基座：全局用户、租户成员关系、租户角色、2FA、邀请、强制改密、会话管理和安全审计。
- 提供 App 接入基座：App 注册、市场发布、租户安装、iframe 托管、SDK 通信、Open API、权限 Scope、Token 交换和用户授权。
- 提供平台级服务：通知、调度任务、仪表盘扩展点、内容注册表、文件预览能力、目录服务、服务账号、用量统计、席位/套餐、运行期设置和部署控制。
- 支持独立后端 App：业务 App 可以有自己的前端、后端和数据库，平台通过 TDT、Token introspection、服务注册和网关代理完成最小耦合。
- 保持可本地验证：用 Bun workspace、PostgreSQL、Redis 和一组验证脚本覆盖平台关键协议、租户隔离、权限边界和集成链路。

## 非目标

- 不把所有业务逻辑集中进门户后端。业务 App 应该拥有自己的领域模型和服务边界。
- 不在根 README 维护每个 App 的完整业务说明。App 会持续增加，根 README 只维护平台共性能力和入口。
- 不让 App 直接读取门户登录态。门户会话是 httpOnly cookie + Redis session，App 通过平台签发的 TDT 调用 Open API 或自己的后端。
- 不允许 Token 交换提权。跨 App 调用必须经过授权、白名单、Grant、Scope 交集和用户同意。
- 不把平台控制面变成业务 App 配置的万能后台。平台只管理租户、应用、权限、服务账号、部署、配额、运行设置等平台职责。

## 技术栈

| 层 | 技术 |
| --- | --- |
| Monorepo | Bun workspaces |
| Backend | Bun, Elysia, Drizzle ORM, PostgreSQL, Redis |
| Frontend | Vite, React, TypeScript, Tailwind, design tokens |
| Realtime | Elysia WebSocket, Redis Pub/Sub |
| Scheduling | croner, Redis locks, task runs |
| Token | TDT, JWT HS256, Redis revocation, introspection |
| Packages | `@xgent/shared`, `@xgent/portal-sdk`, `@xgent/portal-ui`, `@xgent/file-preview` |

## 目录结构

```text
apps/
  api/          平台后端：认证、租户、App、市场、Token、Open API、控制台、平台服务
  web/          门户 Shell：登录、工作台、管理后台、平台控制台、App Host
  *-app/        业务 App 前端，数量会持续增加
  *-server/     独立业务 App 后端，按 App 自治

packages/
  shared/       前后端共享契约：DTO、错误码、Scope、Envelope、常量
  portal-sdk/   iframe App 与门户宿主之间的 postMessage SDK
  portal-ui/    可复用平台 UI 组件
  file-preview/ 文件预览组件与预览协议

docs/
  平台接入、部署、外部 App 联调、服务接入和专题说明
```

平台核心端口：

- `apps/api`: `http://localhost:3000`
- `apps/web`: `http://localhost:5300`

业务 App 和独立服务会占用额外端口，数量会随平台接入的 App 增长而变化，以各 App 的 `package.json` 和相关文档为准。

## 平台基座

### 身份与租户

- 全局用户 + 租户成员关系，用户可以同时属于多个租户。
- 租户内角色、平台管理员、租户管理员、成员和更细粒度 ACL。
- OAuth / 本地开发 IdP / 密码登录 / 2FA / 邀请 / 强制改密 / 远程登出。
- 租户隔离是平台默认约束：平台 API 和业务 App 服务都应以 `tenantId` 作为访问边界。

### 门户 Shell

- 统一应用入口、侧边导航、顶部栏、主题、语言、通知、个人设置和工作台。
- App 可以声明导航项、仪表盘组件和运行期配置，门户负责呈现和权限裁剪。
- `type=micro` 的 App 通过 iframe 托管，门户通过 SDK 提供 Token、主题、语言、路由、通知、服务调用等宿主能力。
- `type=native` 的平台页面可以作为门户一等页面挂载，例如平台控制台。

### 平台控制面

- 平台控制台管理租户、租户管理员、域名、品牌、服务账号、市场应用、套餐、席位、用量和运行设置。
- 租户管理后台管理本租户用户、应用、市场安装、角色权限、审计、任务和用量。
- 平台管理 API 采用 fail-closed 策略，关键接口需要平台管理员会话或 `x-platform-key`。

## 平台级服务

### App 市场与安装

- 平台维护 App listing、版本、依赖、默认 Scope、运行时配置和服务注册。
- 租户管理员从市场安装、启用、升级或卸载 App。
- App 会不断增加；平台通过统一 listing、安装和配置协议避免为每个 App 改门户核心逻辑。

### Token 与授权

- TDT 是平台向 App 签发的短期 JWT，绑定用户、租户、App、Scope 和 audience。
- 支持 iframe SDK 获取用户态 TDT，也支持 server-side OAuth authorization code。
- 支持 service account、client credentials、Token introspection 和 Token revocation。
- 支持跨 App Token exchange，交换结果受双方配置、白名单、Grant、Scope 交集和用户同意共同约束。

### Open API 与 SDK

- Portal Open API v1 提供用户信息、目录、通知、调度、仪表盘、设置等平台服务。
- `@xgent/portal-sdk` 提供 `ready()`、`getToken()`、`userinfo()`、`notify()`、`scheduler.*`、`dashboard.*`、`callService()`、主题/语言/路由事件等能力。
- 独立后端 App 通过 `sdk.callService(name, path, init)` 走宿主代理或直接服务调用，减少控制面 CORS 和凭证暴露。

### 通知、任务与仪表盘

- 通知服务支持站内信、实时 WebSocket 推送、来源记录、链接安全收口和发送权限控制。
- 调度服务支持 App 创建用户级任务、任务运行记录、并发控制、失败处理和 webhook / inbox 触发。
- 仪表盘支持平台组件和 App 贡献组件，按用户、租户和权限呈现。

### 目录、权限与审计

- 目录服务提供租户成员查询、最小化字段返回、可选邮箱 Scope 和服务侧用户开通能力。
- ACL/RBAC 支持平台能力、租户角色、App 权限和服务账号能力。
- 审计覆盖用户、管理、服务账号、凭证、关键配置和敏感操作。

### 用量、席位与运行设置

- 用量统计按租户、App、服务和日期聚合，为计费、配额和运营报表提供基础数据。
- 席位和套餐能力用于租户级容量控制。
- 运行期设置用于开关能力、配置品牌、域名、服务接入参数和平台策略。

### 部署与外部 App

- 平台包含部署控制、服务注册、镜像/环境描述和 KubeSphere / Docker / PM2 部署文档。
- 外部 App 可以独立开发、独立部署，再通过市场 listing、服务账号、TDT introspection 和服务注册接入平台。

## 本地开发

### 前置依赖

- Bun >= 1.3
- PostgreSQL
- Redis
- 部分独立 App 或存储能力可能需要额外基础设施，以对应 App 文档为准

根目录 `.env` 保存数据库、Redis、会话、TDT、OAuth、平台管理员 Key、服务账号和各类运行设置。开发环境可以启用 `DEV_MOCK_OAUTH=true`，用本地模拟 IdP 完成登录链路。

### 安装与初始化

```bash
bun install
bun run db:generate
bun run db:migrate
bun run db:seed
```

全量迁移所有已接入的独立 App 数据库：

```bash
bun run db:migrate:all
```

### 启动

只启动平台基座：

```bash
bun run dev
```

启动平台和当前已接入的 App：

```bash
bun run dev:all
```

常用单项入口：

```bash
bun run dev:api
bun run dev:web
```

打开 `http://localhost:5300`，使用本地开发账号登录。根 README 不维护完整账号清单，账号和租户种子以 `apps/api/src/db/seed.ts` 及各 App seed 脚本为准。

## 验证

平台级健康检查：

```bash
bun run health:platform
bun run health:all
```

平台级完整验证：

```bash
bun run verify:all
bun run verify:onebox
```

核心专项验证脚本位于 `apps/api/scripts/`，覆盖 Token、租户隔离、平台控制台、市场、服务账号、Token exchange、ACL、通知、调度、内容、用量、席位、部署和安全收敛等平台能力。独立 App 的验证脚本位于各自 `*-server/scripts/` 下。

## 关键约定

- **业务状态用 Envelope 表达。** 平台业务响应使用 `{ ok, data }` 或 `{ ok:false, error }`，传输、认证和路由错误才使用 HTTP 4xx/5xx。
- **门户登录态不暴露给 App。** App 只能通过 TDT、SDK、Open API、服务账号或 Token introspection 参与平台协议。
- **Scope 是平台能力边界。** App 需要声明 Scope，用户或管理员授权后才能调用对应平台服务。
- **跨 App 调用不能提权。** Token exchange 只会得到双方允许范围内的 Scope 交集。
- **独立后端 App 与平台最小耦合。** 平台负责身份、授权、安装、配置、服务发现和治理；App 后端负责自己的业务数据和领域规则。
- **App 数量会持续增长。** 新 App 应优先复用平台现有接入协议和服务，避免为单个 App 在门户核心中增加特殊逻辑。

## 文档入口

- [SSO 与 App 开发指引](docs/SSO与App开发指引.md)
- [外部 App 本地联调指南](docs/外部App本地联调指南-one-box.md)
- [Files 文件服务接入指引](docs/Files文件服务接入指引.md)
- [任务中心 API 清单](docs/任务中心-API清单.md)
- [用量统计服务接入指引](docs/用量统计服务接入指引.md)
- [部署：Docker](docs/deploy/docker.md)
- [部署：PM2](docs/deploy/pm2.md)
- [部署：KubeSphere](docs/deploy/kubesphere.md)
