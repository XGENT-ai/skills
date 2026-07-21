---
name: portal-backend-app
description: 在本 monorepo 内新增或修改「独立后端 App」（apps/<key>-server + apps/<key>-app 双包模式，如 spms/qbank/lms/llm-gateway/exam/library）。凡任务涉及新建一个后端服务、TDT 自省/四道闸/服务账号、App 的 scope 与 ACL 声明、LISTING_DEFS/seed 布线、给某 *-server 加 API 端点、DB 迁移或字典表、应用配置 tab、部署接线（Caddy/compose/DEPLOYABLE_APP_SERVICES）时，务必先用本 skill——即使用户只说"给 XX 应用加个接口"。Use when creating or extending a standalone-backend app inside this monorepo: gate/introspection, service accounts, listing/seed wiring, migrations, admin config tabs, deploy wiring.
---

# portal-backend-app · monorepo 独立后端 App 开发

独立后端 = 自己的进程 + 自己的 DB，消费 Portal 签发的 TDT。权威契约随本 skill 附带：[`references/docs/SSO与App开发指引.md`](references/docs/SSO与App开发指引.md)（下称"指引"；`references/` 是门户仓库文档的镜像，文内相对链接原样可解析）§7/§12/§13；参考实现 `apps/spms-server/src/lib/gate.ts` + `identity.ts`（各 server 同构）。本 skill 是"新建/扩展一个 server"的操作序 + 硬规则。

> 本 skill 自包含，可整目录拷到任何 repo 使用；但本 skill 描述的是**门户 monorepo 内**的双包模式，文中 `apps/`、`deploy/` 路径在外部项目不存在——服务端在别的 repo 时请改用 portal-external-app skill。门户仓内工作时指引以 `docs/` 原件为准（`.claude/skills/sync-portal-skill-refs.sh` 同步副本）。

## 新建一个后端 App 的全链 checklist

按序做完，每步都有既有样板可抄（选最接近的现有 server，如轻业务抄 `todo-server`、重业务抄 `lms-server`）：

1. **双包**：`apps/<key>-server`（Bun + Elysia，dev 端口取空闲 4xxx）+ `apps/<key>-app`（前端见 portal-micro-app skill）。DB 命名 `xgent-<key>`。
2. **鉴权层**：整抄 `spms-server/src/lib/gate.ts` + `identity.ts`，改 listingKey/scope 常量。不要自己手写自省逻辑。
3. **清单与布线**（单一事实源，管理员不能手填）：
   - ACL Manifest → `apps/api/src/modules/acl/manifests.ts`；
   - listing（scopes/navItems/exchangeTargets/serviceBaseUrl 等）→ `apps/api/src/db/provisioning.ts` 的 `LISTING_DEFS`；
   - 服务账号 → `SA_DEFS`（约定 `clientId = <key>-server`，capability `token.introspect`）；
   - 跨应用交换 → `EXCHANGE_WIRING`（详见 portal-app-exchange skill）；
   - scope 只能落自己命名空间（`<key>.*`，连字符 key 用下划线变体）——校验在 `market/service.ts::validateScopes`。
4. **种子**：App 自己的 seed/bootstrap 写成**非破坏性**独立脚本（`db:seed` 是 portal-only + 破坏性；链路关系见 CLAUDE.md）。
5. **dev:all**：server+app 挂进根 `package.json` 的 concurrently；绝不叠跑单个 dev 脚本。
6. **部署接线**（新建时一次做全，漏一个生产就 404/起不来）：
   - `deploy/caddy/Caddyfile` 的 `/svc/<key>` 白名单；
   - compose 的 `app-<key>` profile（同一 runtime 镜像 + `command: bun --filter @xgent/<key>-server`）；
   - `apps/api/src/modules/deploy/registry.ts` 的 `DEPLOYABLE_APP_SERVICES`（按需部署门）；
   - **Dockerfile 把新 workspace 目录加进 COPY**——漏了本地全绿、镜像构建才炸（真实事故）；
   - `build:apps` 保持串行（并行 OOM）。
7. **验证脚本**：`apps/<key>-server/scripts/verify-*.ts` 覆盖 TDT/隔离/ACL/核心业务，并挂进 `apps/api/scripts/verify-all.ts`。没有 verify 脚本的功能不算交付。

## 四道闸（每个受保护端点必过）

自省 `POST /api/tokens/introspect`（服务账号 Basic）后：

1. **身份**：`(claims.listingKey ?? claims.aud) === 本服务 listingKey`——按 listingKey 鉴权，对安装态 appKey 透明；
2. **scope**：所需 scope ∈ `claims.scopes`（403 `INSUFFICIENT_SCOPE`）；
3. **结构性管理操作**：`claims.role === "admin"`；
4. **细粒度**：`bypass || permissions 命中 PID`，数据范围取最宽（`own`<`team`<`all`）。

硬形状（写错就是全量 401 的事故）：

- 自省响应走统一信封，**声明在 `data` 里**：`claims = body.data ?? body`；
- `active:false` 是**成功**的自省（令牌无效），不是传输错误；结果缓存 ≤60s；
- 服务态 TDT（`kind:"service"`，无 `user_id`）**只按 scope 授权**，`permissions` 恒空，勿走 PID 门；
- 租户隔离一律 `claims.tenant_id`，永不信任请求体；限流自建 `(aud, tenant)` 每分钟窗口。

## API 与数据规范（CLAUDE.md 强制，评审必查）

- **业务状态不走 HTTP 状态码**：一律 `200 + {ok, data|error}`；4xx/5xx 只留传输/认证/路由层。数据不存在 = `200 + data:null`，不是 404。
- 列表端点服务端分页，复用 `Page<T>` 契约。
- **字典表**统一 `sort integer NOT NULL DEFAULT 0` + `orderBy(asc(sort), asc(name))`；树/兄弟次序表保留 `seq`，别混。
- 迁移：`drizzle-kit generate` 在本仓已知损坏——**手写迁移 SQL** 放进 server 的 migrations 目录。
- 平台公共服务红线：审计写 `POST /api/v1/audit`（不自建审计表/页）；租户配置走应用配置 tab（下节）；不做「同步通讯录」（UserPicker curated）；用量按 [`references/docs/用量统计服务接入指引.md`](references/docs/用量统计服务接入指引.md) 上报。

## 应用配置 tab（租户级配置，指引 §13.7）

三件套，参照 files / llm-gateway 逐个对号：

1. server 暴露 `GET/PUT /api/v1/settings`（四道闸，写操作落 ACL action）；
2. `apps/api/src/modules/apps/index.ts` 加 `GET /api/admin/apps/:appKey/<key>-token`（assertAdmin + signTdt，600s admin-TDT）；
3. `apps/web/src/lib/<key>-config-api.ts` + `AppForm.tsx` 按 `form.scopes.includes("<key>.…")` 条件挂 Section（浏览器直连 server，门户不经手敏感配置）。

微应用侧**不**再渲染自己的「设置」导航。

## 验证

改动完成 = 该 server 的 `verify-*.ts` 全绿 + `bun run verify:all` 不回归 + （有 UI 面时）浏览器走通。dev 环境要点：自省优先服务账号 Basic；`x-resource-key` 仅 `DEV_MOCK_OAUTH=true` 且只认 `FILES_RESOURCE_KEY` 一把；后端脚本登录用无 2FA 用户（如 `liming`）。
