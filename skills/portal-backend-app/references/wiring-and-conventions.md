# 注册布线、部署接线与平台规范（monorepo 后端 App）

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §3/§4/§7.5/附录 A、`docs/用量统计服务接入指引.md`、CLAUDE.md 及历史事故（2026-07）。冲突时以门户仓库为准。

## 1. 注册链（单一事实源在代码）

```
①代码注册表                    ②市场清单                  ③安装快照         ④运行时
acl/manifests.ts + LISTING_DEFS ─seed─► marketplace_listings ─install─► apps 行 ─► SDK init.acl / introspection
```

| 声明什么 | 写在哪 |
| --- | --- |
| ACL Manifest | `apps/api/src/modules/acl/manifests.ts`（按 listingKey 集中定义） |
| listing（scopes/navItems/exchangeTargets/serviceBaseUrl/依赖…） | `apps/api/src/db/provisioning.ts` 的 `LISTING_DEFS` |
| 服务账号 | 同文件 `SA_DEFS`；约定 `clientId = <key>-server`，capability `token.introspect` |
| 跨应用交换 | 同文件 `EXCHANGE_WIRING`（`{ 发起方: [目标...] }`） |
| 内容类型（如用内容服务） | `apps/api/src/modules/content/schemas.ts`，owner=listingKey |

升级 Manifest = 改 `manifests.ts` 的 version+内容 → 重新 seed/发布清单 → 租户同步更新。`db:seed` 是 portal-only + **破坏性**（重生成 UUID、会话失效）；App 自己的种子写成独立**非破坏性**脚本。

**scope 命名空间**（写时校验 `market/service.ts::validateScopes`）：一条清单只能声明 平台基础 scope ∪ 自己命名空间（`<listingKey>.` 前缀，连字符 key 接受下划线变体如 `llm_gateway.*`）∪ 已声明 `exchangeTargets` 的目标命名空间。App 命名空间 scope 随清单入库，**无需**改 `packages/shared/src/scopes.ts`；配 `scopeLabels` 三语文案。

平台基础 scope 速查：`userinfo.read`、`directory.read`（+`directory.email.read` 含邮箱）、`notification.send`（+`.others` 跨用户）、`inbox.read`、`settings.read/write`、`widget.write`、`audit.write`、`scheduler.read/write`、`content.read/write`。

## 2. 部署接线 checklist（新建 App 漏一项生产就出事）

| 接线 | 位置 | 漏掉的症状 |
| --- | --- | --- |
| `/svc/<key>` 白名单 | `deploy/caddy/Caddyfile` | 生产 404（网关白名单 map 收口，未注册=路由不可达） |
| compose profile | `deploy/docker-compose.yml` 加 `app-<key>`（同 runtime 镜像 + `command: bun --filter @xgent/<key>-server`） | 容器起不来 |
| 按需部署注册表 | `apps/api/src/modules/deploy/registry.ts` 的 `DEPLOYABLE_APP_SERVICES` | 生产卡「服务正在部署」门 |
| **Dockerfile COPY 新 workspace** | 根 `Dockerfile` | 本地全绿、镜像构建才炸（真实事故） |
| 前端构建 | `build:apps` 保持**串行**（并行 OOM） | CI/构建机 OOM |
| dev:all | 根 `package.json` concurrently 挂 server+app | 本地起不齐 |

容器/生产统一监听 8080（本地 dev 各用自己的 4xxx）；反代通用规则 `/svc/<key>/* → <key>-server:8080`。宿主运行时按 listing `serviceBaseUrl` 解析服务地址（无烘焙注册表），新增后端无需重建门户前端。

## 3. API 与数据规范（评审必查）

- **HTTP 状态码只反映传输/路由层**：业务成功/失败一律 `200 + {ok, data|error}`；不用 201/204；数据不存在 = `200 + data:null` 不是 404；4xx/5xx 只留参数结构错/未登录/未授权/真故障。
- **列表服务端分页**：复用 `Page<T>` 契约与现有 helpers（所有 admin/console 列表页同款）。
- **字典表**（租户级可维护枚举/分类）：排序列固定 `sort integer NOT NULL DEFAULT 0`，查询 `orderBy(asc(sort), asc(name))`，创建 `sort: body.sort ?? 0`、更新 `if (body.sort !== undefined)`。**例外**：树/兄弟次序（章节树、子母题）保留 `seq`，语义是序号不是字典排序。
- **迁移手写**：`drizzle-kit generate` 在本仓已知损坏——迁移 SQL 手写进 server 的 migrations 目录。
- **审计**：不自建审计表/页，`POST /api/v1/audit`（scope `audit.write`，用当次用户 TDT），best-effort 不阻断业务。
- **不做「同步通讯录」**：成员一律 `@xgent/portal-ui` 的 `UserPicker` curated 按需挑选。

## 4. 用量上报（如需计量/计费）

心智模型：**日桶（UTC 日切）+ 桶内绝对值幂等覆写**。上报的是整桶重算的绝对值不是增量（重试/补报天然幂等）；唯一键 `(tenantId, metricKey, day, dimsKey)`。

1. **登记指标**：`packages/shared/src/usage.ts` 的 `USAGE_METRICS`（key 前缀必须=你的 listingKey；kind 选对：counter=流量求和 / gauge=水位取峰值，选错聚合口径灾难；dims 白名单防高基数——别放 userId/requestId）。
2. **服务账号三件套**：capabilities += `client_credentials`、scopes += `usage.report`、`ownerAppKey` = listingKey（`SA_DEFS` 加 `usageReporter: { ownerAppKey }`，`ensureServiceAccount` 幂等补齐）。`metricKey` 前缀必须等于令牌 `azp`——别的 SA 报你的指标整批拒收。
3. **上报**：按租户铸服务态令牌（**检查返回 scope 含 `usage.report`**——SA 未授权时不报错只返空 scope，应视为可重试失败；`APP_NOT_INSTALLED`/`APP_DISABLED`/`TENANT_NOT_ALLOWED` 才是该租户永久跳过）→ `POST /api/v1/usage/report { records:[{tenantId, metricKey, day, dims?, value}] }`，单批 ≤500、all-or-nothing、`rejected` 非空响亮 log。
4. 参考实现：counter+水位线 `apps/llm-gateway-server/src/lib/usage-report.ts`；gauge+快照 files。

## 5. 应用配置 tab（租户级配置三件套）

不在微应用里自建「设置」页；配置收口平台应用配置页：

1. server 暴露 `GET/PUT /api/v1/settings`（四道闸，写操作落 ACL action）；
2. `apps/api/src/modules/apps/index.ts` 加 `GET /api/admin/apps/:appKey/<key>-token`（assertAdmin，signTdt 铸 aud=appKey、按需 scope 的 600s admin-TDT，无 consent 门）；
3. `apps/web/src/lib/<key>-config-api.ts` + `AppForm.tsx` 按 `form.scopes.includes("<key>.…")` 条件挂 Section（参照 `showStorage`/`showGateway`），浏览器直连你的后端。

## 6. 本地 dev 要点

- 一把起齐 `bun run dev:all`；绝不叠跑单个 dev 脚本（Vite 端口自增会抢相邻 App 端口，iframe 加载错应用）。重起先清干净旧监听。
- 非生产下部署门自动视作 ready，本地不会卡「服务正在部署」。
- 种子账号均 `@xgent.ai`：`rockie`（晨光 admin + 平台管理员）、`liming`（晨光 user/星网 admin，**无 2FA，脚本登录用它**）、`chenjing`（admin，开了 2FA）。
- 验证脚本放 `apps/<key>-server/scripts/verify-*.ts` 并挂进 `apps/api/scripts/verify-all.ts`；交付标准 = 本服务 verify 全绿 + `bun run verify:all` 不回归。
