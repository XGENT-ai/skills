---
name: portal-backend-app
description: '在本 monorepo 内新增或修改「独立后端 App」（apps/*-server + apps/*-app 双包模式，如 spms/qbank/lms/llm-gateway/exam/library）。凡任务涉及新建一个后端服务、TDT 自省/四道闸/服务账号、App 的 scope 与 ACL 声明、LISTING_DEFS/seed 布线、给某 *-server 加 API 端点、DB 迁移或字典表、应用配置 tab、部署接线（Caddy/compose/DEPLOYABLE_APP_SERVICES）时，务必先用本 skill——即使用户只说"给 XX 应用加个接口"。Use when creating or extending a standalone-backend app inside this monorepo: gate/introspection, service accounts, listing/seed wiring, migrations, admin config tabs, deploy wiring.'
---

# portal-backend-app · monorepo 独立后端 App 开发

独立后端 = 自己的进程 + 自己的 DB，消费 Portal 签发的 TDT。本文件是操作序与红线；具体契约按需读 `references/`（为本 skill 提炼的自包含参考；权威源是门户仓库 `docs/`，冲突以门户仓库为准）。本 skill 描述**门户 monorepo 内**的双包模式——服务端在别的 repo 时改用 portal-external-app skill。

## 按任务读参考

| 任务 | 读 |
| --- | --- |
| 自省契约 / 四道闸 / 服务账号 / client_credentials / 撤销 | [references/gate-and-tokens.md](references/gate-and-tokens.md) |
| 注册布线 / scope 目录 / 部署接线 / API 规范 / 用量上报 / 配置 tab | [references/wiring-and-conventions.md](references/wiring-and-conventions.md) |

参考实现：`apps/spms-server/src/lib/gate.ts` + `identity.ts`（各 server 同构的鉴权层）。

## 新建一个后端 App 的全链 checklist

按序做完，每步有既有样板可抄（轻业务抄 `todo-server`、重业务抄 `lms-server`）：

1. **双包**：`apps/<key>-server`（Bun + Elysia，dev 端口取空闲 4xxx）+ `apps/<key>-app`（前端见 portal-micro-app skill）。DB 命名 `xgent-<key>`。
2. **鉴权层**：整抄 `spms-server/src/lib/gate.ts` + `identity.ts`，改 listingKey/scope 常量。不要手写自省逻辑。
3. **清单与布线**（位置与规则见 wiring-and-conventions.md §1）：ACL Manifest → `manifests.ts`；listing → `LISTING_DEFS`；服务账号 → `SA_DEFS`；交换 → `EXCHANGE_WIRING`；scope 只能落自己命名空间。
4. **种子**：App 自己的 seed/bootstrap 写成**非破坏性**独立脚本（`db:seed` 是 portal-only + 破坏性）。
5. **dev:all**：server+app 挂进根 `package.json` 的 concurrently；绝不叠跑单个 dev 脚本。
6. **部署接线**一次做全（Caddy 白名单 / compose profile / `DEPLOYABLE_APP_SERVICES` / **Dockerfile COPY 新 workspace** / `build:apps` 串行）——逐项症状见 wiring-and-conventions.md §2，漏一个生产就 404/起不来。
7. **验证脚本**：`apps/<key>-server/scripts/verify-*.ts` 覆盖 TDT/隔离/ACL/核心业务，挂进 `apps/api/scripts/verify-all.ts`。没有 verify 脚本的功能不算交付。

## 四道闸（每个受保护端点必过）

自省 `POST /api/tokens/introspect`（服务账号 Basic）后：① `(claims.listingKey ?? claims.aud) === 本服务 listingKey`；② 所需 scope ∈ `claims.scopes`；③ 结构性管理操作看 `claims.role === "admin"`；④ 细粒度 `bypass || permissions 命中 PID`（数据范围取最宽）。

如端点是**平台级跨租户**操作，在四道闸之外另加 `claims.isPlatformAdmin === true`：该值是 Portal 按 `user_id` 在每次自省时实时计算的全局身份，不在 JWT 里。`role` / `bypass` 只代表 TDT 当前租户，不能代替这道闸；也不接受前端 header/字段自报。

写错就是全量 401 的硬形状：

- 自省响应走统一信封，**声明在 `data` 里**：`claims = body.data ?? body`；
- `active:false` 是**成功**的自省（令牌无效），不是传输错误；结果缓存 ≤60s；
- 服务态 TDT（`kind:"service"`，无 `user_id`）**只按 scope 授权**，勿走 PID 门；
- `active:true` 的自省结果始终带布尔 `isPlatformAdmin`；服务态恒为 `false`，身份变更传导受下游 ≤60s 缓存上限约束；
- 租户隔离一律 `claims.tenant_id`，永不信任请求体；限流自建 `(aud, tenant)` 每分钟窗口。

完整字段与服务账号细节见 [references/gate-and-tokens.md](references/gate-and-tokens.md)。

## API 与数据规范（评审必查，细则见 wiring-and-conventions.md §3）

- 业务状态不走 HTTP 状态码：一律 `200 + {ok, data|error}`；数据不存在 = `200 + data:null` 不是 404。
- 列表端点服务端分页（`Page<T>`）；字典表统一 `sort`（树/兄弟次序保留 `seq`）。
- 迁移**手写 SQL**（drizzle-kit generate 本仓已知损坏）。
- 审计写 `POST /api/v1/audit`，不自建审计；租户配置走应用配置 tab 三件套（wiring-and-conventions.md §5）；不做「同步通讯录」；计量走用量上报（wiring-and-conventions.md §4）。

## 验证

改动完成 = 该 server 的 `verify-*.ts` 全绿 + `bun run verify:all` 不回归 + （有 UI 面时）浏览器走通。dev 要点：自省优先服务账号 Basic（`x-resource-key` 仅 `DEV_MOCK_OAUTH=true` 且只认 `FILES_RESOURCE_KEY` 一把）；后端脚本登录用无 2FA 用户（如 `liming`）。
