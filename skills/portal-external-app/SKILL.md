---
name: portal-external-app
description: '接入「外部镜像服务类应用」——服务端代码不在本 repo、以 Docker 镜像交付的 App（知识库/多模态解析/异步任务网关这类）。凡任务涉及为外部服务写对接契约或 app.manifest.json、register-app/provisioning 注册布线、/svc 路由与健康检查、外部服务的服务账号与 scope、一盒(one-box)本地联调、或评审外部团队的交付物时，务必先用本 skill——即使用户只说"把 XX 服务接进来"。Use when integrating an externally-built docker-delivered service app into the portal: manifest, register-app, provisioning, /svc wiring, one-box debugging, or reviewing an external team''s delivery.'
---

# portal-external-app · 外部镜像服务类应用接入

外部镜像 App = 服务端在别的 repo（任意语言/栈）、独立镜像交付、**常驻**（不走按需缩零）。运行时契约与内建 App 完全一致，差异只在注册与部署。本文件是判断与流程；具体契约按需读 `references/`（为本 skill 提炼的自包含参考，可整目录拷到外部团队 repo；权威源是门户仓库 `docs/`，冲突以门户仓库为准）。


> **路径约定（先读这条，能省一次白找）**：本 skill 里出现的 `apps/…` `packages/…` `docs/…`
> `deploy/…` 这类路径**都在门户仓**。在 App 自己的 repo 里它们**不存在** —— 它们标注的是
> 「门户侧要做什么」或某段内容的出处，**不是让你去打开的文件**。找不到不是配置错误：
> 别去创建、别去全局搜、别把它当缺失依赖报出来。你需要的一切都在本 skill 的
> `references/`（自包含）。若你正在门户仓里工作，那这些路径就是可以直接打开的真实文件。
>
> ⚠️ 一个例外：`/apps/<key>/`（带前导斜杠）是**线上 URL 路径**——微应用产物的挂载点，
> 与仓内的 `apps/<key>-app/` 目录无关，别混。

## 按任务读参考

| 任务 | 读 |
| --- | --- |
| 实现/评审资源服务器（四道闸、health、env、认证面划界、配置面） | [references/integration-contract.md](references/integration-contract.md) |
| 写 manifest、注册布线、一盒联调、排查 | [references/registration-and-onebox.md](references/registration-and-onebox.md) |
| 为外部服务写/审对接契约文档 | [references/contract-doc-template.md](references/contract-doc-template.md) |
| 有「每租户最多几个 X」的配额诉求（选模型、数值谁配、已用怎么来） | [references/quota-and-seats.md](references/quota-and-seats.md) |

## 先判形态（决定后面走哪条线）

- 有用户前端？→ `type:"micro"`（前端 dist 同源托管 `/apps/<key>/`，前端开发见 portal-micro-app skill）；无前端 → **`type:"service"`**（无头：不露卡、不可打开，仅作被调用方，控制台可治理）。
- 谁调它？→ 自己前端（用户态 TDT）/ 其他 App 后端（令牌交换，见 portal-app-exchange skill）/ 其他服务无用户直调（服务账号 `client_credentials`）。三种来路授权姿势不同（integration-contract.md §3 的表）。
- 它自带认证体系（API-Key/节点 Token/gRPC）？→ 按 integration-contract.md §6 划界：门户面必走 TDT 四道闸；自有节点面保持原认证但**不得**进 `/svc`（`/svc` 只转发 HTTP :8080）。

## 交付物评审 checklist（收外部团队东西时逐项验）

1. 镜像：监听 8080、网络别名 `<listingKey>-server`、amd64+arm64、**不烘焙 .env**、`docker save` tar + sha256 或私有 registry；
2. `app.manifest.json`（外部团队 repo 里的单一事实源）；
3. env 契约表：**镜像实际读取的变量名**（镜像自有前缀与门户契约别名的映射必须写清；有的镜像不读裸 `PORT`）；
4. **迁移入口**：迁移文件在镜像里、二进制上有一个迁移 argv（见下节）。「迁移脚本留在自己 repo 里」= 不合格；
5. 运维口径：是否需每租户 bootstrap（需要则 `tenants.id` 必须 = 门户租户 UUID）+ `/health` 口径；
6. **有没有重复造平台已有的能力**：自己存文件 / 自己发通知 / 自己记审计 / 自己排定时任务 / 自己算配额 —— 逐项对照下面两张表，命中就要求改走平台面；
7. **配额诉求**：有没有「每租户能建多少」这类上限？有就走平台套餐（见 [references/quota-and-seats.md](references/quota-and-seats.md)），**不接受对方自建**；
   角色本身该由对方在自己的 `app.manifest.json` 里声明 `seatRoles`；`serviceScopes` 里出现 `seats.read` 仍是当场打回的信号，但对方可以在 `privilegedServiceScopes` 里**申请**它（带 reason，进「发布审核」逐条确认批准后授予）；
8. 对接契约文档：按 [references/contract-doc-template.md](references/contract-doc-template.md) 的结构与精度。**没有这份文档的接入不算完成**——新服务（如任务网关）要先补。

## 资源服务器硬契约（外部实现最常炸的四处）

1. **自省信封解包**：声明在 `data` 里，必须 `claims = body.data ?? body`——裸读顶层 `active` 会把一切有效 TDT 判 401（真实事故）；
2. **`/health` 形状**：`{"service":"<key>","db":"ok",...}`，`"db"` 是字符串 `"ok"` 不是 `true`（healthcheck 按此判活）；
3. **门户三变量 all-or-nothing**：自省地址 + SA clientId + secret 全缺→鉴权停用 503；缺一→启动 fail-fast 打印缺失项。
4. **平台级跨租户闸**：如有跨租户路由，只认服务端自省返回的 `claims.isPlatformAdmin === true`。它由 Portal 按用户实时计算、不在 JWT 里；`role` / `bypass` 只属于当前租户，服务态恒为 `false`，不接受前端自报或 Cookie 转发。

完整契约（四道闸、三种令牌来路、审计、划界）见 [references/integration-contract.md](references/integration-contract.md)。

## 注册布线（细节见 registration-and-onebox.md）

- **dev / 一盒**：`bun run register-app <manifest>`（幂等，生产拒跑）= upsert listing + 直写服务账号 + 写发起方 App Secret + 写 `/svc` 白名单 map。⚠️ `exchangeInitiatorSecret` 写的是**已安装**实例——先安装 App，**再重跑一次** register-app，否则发起交换 401。日志里 `wired into N …` 的 N 是「**改了**几个实例」，0 也可能是「本来就对」——判据是比哈希，见 registration-and-onebox.md §2.2。
- ⚠️ **发布 ≠ 租户能看见**：租户市场对 `tenant_listing_grants` 是 fail-closed。**dev 的 `register-app` 会自动授予当时已存在的租户**（非 service 型，输出里有 `租户可用应用:` 一行）；**生产不自动**（提案批准 / 控制台导入都不授予——语义是平台管理员显式勾选）。症状是「控制台显示已上架、租户市场里却根本看不到」——去 平台 › 租户管理 › 租户 › 「可用应用」勾上保存（registration-and-onebox.md §2.1）。
- **生产（标准路径）**：清单事实源在**对方仓的 `app.manifest.json`**，经
  **发布提案**到达：平台管理员在 控制台 › 应用市场 › 接入新应用 按 key 签发 `xrel_` 令牌
  （无行先建 draft 占位），对方 `publish --manifest … --dist … --image …` ⇒ 首次必进
  「发布审核」队列 ⇒ 批准 = `registerFromManifest` 一次建全（listing 上架 + SA + /svc +
  已装租户对齐），SA secret 明文一次性回显给审批人。**不要**再把外部 App 登记进
  `LISTING_DEFS` —— 那会成为把对方清单静默改回去的第二事实源（verify-split「清单零残留」
  棘轮看守）。门户保留的平台侧事实只有：部署行、Caddyfile 内联行、`EXCHANGE_WIRING`，
  以及作为**种子**的 `SA_DEFS` 与内置 scope 常量 / `USAGE_METRICS` ——「审批而非发版」后，
  特权服务态 scope 经对方 manifest 的 `privilegedServiceScopes` 申请、审批即授予（SA_DEFS
  不再是唯一授予点，union top-up 也不会回滚审批授予）；用量指标经 manifest `usageMetrics`
  声明（治理档）；控制台应用配置的 scope 校验运行时按 listing 声明放行（不卡编译期枚举）。
- **后续版本**：对方 CI 自助（`xgent-app-release` skill）——无治理变更自动通过，
  治理变更进「发布审核」等平台批准。
- **scope 三规则**：平台基础 scope ∪ 自己命名空间（含连字符→下划线变体）∪ 已声明 `exchangeTargets` 的目标命名空间；越界 `VALIDATION_FAILED`。
- **secret 一致红线**：平台落库的服务账号 secret 与镜像 env 里的必须一致；漂移症状是自省 401 或「跨应用授权缺失」（实为 `SECRET_INVALID`），排查先 sha256 比对两侧。

## 数据库迁移：平台在部署时替你跑

划分决定一切细则：**schema 是平台级的**（随一次部署发生一次），**数据行是租户级的**（随安装发生）。
所以 DDL 属于部署步骤 —— 不属于容器启动，也不属于逐租户安装。

外部团队声明 `deployDescriptor.migrateArgs`（平台管理员填），控制器在**换容器之前**用
**同镜像 + 同 env** 跑一次性容器（`--rm`；pm2 / compose / K8s 三个 flavor 都实现了）：

```
job(deploy|redeploy)
 ├─ ① migrateArgs 非空 → <entrypoint> <migrateArgs>；失败 ⇒ 任务红 + 旧容器继续服务
 ├─ ② ensureServiceUp     ← 到这一步才换容器
 ├─ ③ waitHealthy(180s)   └─ ④ openRoute
```

**为什么不是「启动时自迁移」**：失败会变成崩溃循环（要 `docker logs` 才知道为什么），
还要挤 180 秒健康窗口。放在换容器前，失败时旧版仍在服务，且是面板上一个有名字、
带对方 stderr 的步骤。

**为什么必须在镜像里**：迁移要在**库所在的地方**跑，而跨越边界的产物只有镜像；对方 CI
连不到（也不该连到）门户的库。且镜像 = 那一版代码，它要求的 schema 与它同源同版本 ——
分开走就会漂。**门户永远不执行由 App 投递进来的脚本**（宿主机侧的 `migrateScript` 只对
仓内 built-in 开放，`descriptorToService()` 对 config App 钉死为 `null`）。

要求外部团队做到四条：迁移打进镜像 + 一个迁移 argv（如 `--role migrate`）· `pg_advisory_lock`
包住 · **失败非零退出**别自愈 · **expand-contract**（迁移跑在换容器前，那一刻旧版本还在服务）。

⚠️ **`migrateArgs` 只有门户部署了对应版本才生效**。反过来的顺序（先填字段、门户没更新）
会被 jsonb 原样存下、**无人读、不报错** —— 与被删掉的 `migrate`/`provision`/`bootstrap`
三个死布尔同病。次序固定：门户先部署 → 对方出镜像 → 才填字段。

⚠️ **每租户 bootstrap 目前没有出口**：`bootstrapScript` 对 descriptor App 恒为 `null`，
门户不会替外部服务建租户行。若对方的表 FK 自己的 `tenants`，接入时必须显式定方案
（对方开 provisioning 端点 / 首次使用自建 / 门户加钩子），否则症状可能是**静默写 0 行**
（`INSERT … WHERE EXISTS (SELECT 1 FROM tenants …)` 这种写法不会报错）。

## 改动启动必需 env 的键名，必须发版前知会门户

`descriptor.env` / `envFile` 是**平台管理员治理字段**，manifest 永远带不进值（提交即拒），
`requiredEnv` 只交键名。**键名集合的变化属治理档**：写进 manifest 的 `requiredEnv` 一变
（新增/改名/删除），提案就转入「发布审核」，管理员批准前先看到新键名 —— 这是把
「知会门户」变成流程内动作的那道门。**CR-3 之后它还是一道真闸**：批准前门户拿这张清单比对
`descriptor.env ∪ envFile` 的键集合（只看键在不在，不读值），缺了就拒绝生效
（`REQUIRED_ENV_MISSING`，提案留 pending）；审批屏逐键标状态，非密钥可当场填。改名要写成
`{ key, renamedFrom }` 一条，门户会把旧键在 `descriptor.env` 里的值搬过去（**envFile 里的搬不了**，
门户不写那个文件，那一条会判「缺」并提示运维改名）。仍有一件它管不到：对方
**不写进 `requiredEnv`** 的静默改名依旧无门可拦 —— 症状是服务
换版后 `refusing to start`，而发布方那侧只看到成功。2026-08-19 知识库把 `XGENT_OPENAI_*`
整族改成 `XGENT_EMBED_*` 就是这么崩了几小时的；只改其中一个 key 更坏 —— 服务起得来，
但 base URL/model/**dims** 静默回落默认值。所以收交付物时仍要求：改必需 env 必须**同步改
manifest 的 `requiredEnv`**（改名走 `renamedFrom`），并把「本版新增/改名的必需 env」列进 checklist。发布面本身
（令牌、tag 不可变、`--wait`/`status`）见 `xgent-app-release` skill。

## 无前端服务的租户管理员配置

外部团队实现 `GET/PUT /v1/settings`（TDT 四道闸）；**门户侧要按 App 补三件**（不是零改码）：`<key>-token` 铸造端点 + `<key>-config-api.ts` + `AppForm` 配置 Section——参照 files / llm-gateway，步骤见 portal-backend-app skill。接入新服务时把这三件列入门户侧工作量（omni-parser 至今欠着）。

## 先看平台有没有，别自建一套

外部服务最常见的浪费是把**平台已经有的公共能力**又实现一遍：自己存文件、自己发通知、
自己记审计、自己排定时任务。除了白干，还会让租户在两个地方看同一件事、让平台的治理面
（审计/用量/权限）出现盲区。**动手写之前先在这两张表里找。**

### A. 门户基座能力 —— 用你自己的 TDT 直接调 `/api/v1/*`

`PLATFORM_BASE_SCOPES` 里的 scope **任何 App 都能在清单里直接声明**，不需要跨应用授权、
不需要令牌交换。

| 你想做的 | scope | 端点 |
| --- | --- | --- |
| 当前用户是谁 | `userinfo.read` | `GET /api/v1/userinfo` |
| 选人 / 读通讯录 | `directory.read`（邮箱是 PII，要**另加** `directory.email.read`） | `GET /api/v1/directory/users` |
| **站内消息 / 通知** | `notification.send`（只发给自己）· `notification.send.others`（发给别的成员） | `POST /api/v1/notifications`；服务态用 `/api/v1/notifications/service` |
| 收件箱 | `inbox.read` | — |
| **统一审计** | `audit.write` | `POST /api/v1/audit` |
| **定时任务** | `scheduler.read` / `scheduler.write` | `/api/v1/scheduler/tasks` |
| 结构化内容 / CMS | `content.read` / `content.write` | `/api/v1/content/{type}` |
| **用量计量** | `usage.report`（只收 namespace 前缀 == 你 azp 的 metricKey） | `POST /api/v1/usage/report` |
| 席位配额 | `seats.read`（SERVICE_ONLY，经 manifest `privilegedServiceScopes` 申请、审批授予，见下节） | `POST /api/v1/seats/quota` |
| 币种与汇率 | `currency.read` | `GET /api/v1/currency` |
| 判权限（PID） | — | `POST /api/v1/acl/check` |
| 用户级设置 | `settings.read` / `settings.write` | `/api/v1/settings/me` |
| Dashboard 卡片 | `widget.write` | 见 portal-micro-app skill |

### B. App 型公共服务 —— 走 `/svc/<key>` + 令牌交换

这些是**独立 App**，不是基座端点：要在清单里声明 `exchangeTargets`，用交换来的
`aud=<目标>` 令牌调（机制与排错见 portal-app-exchange skill）。

| 服务 | listingKey | scope 命名空间 |
| --- | --- | --- |
| **文件存储**（用户空间 + 应用空间、直传、预览、缩略图） | `files` | `files.read` / `files.write` / `files.share` |
| 多模态解析（OCR / 文档结构化） | `omni-parser` | `omni_parser.read` / `omni_parser.parse` |
| 大模型网关（OpenAI 兼容 `/v1`） | `llm-gateway` | `llm_gateway.read` / `llm_gateway.write` |

⚠️ **连字符 listingKey ↔ 下划线 scope**：`omni-parser` 的 scope 是 `omni_parser.*`，
`llm-gateway` 的是 `llm_gateway.*`。两个命名空间，写混了是 `VALIDATION_FAILED`。

**文件这条尤其别自己来**：外部服务要落盘产物时用 files 的**应用空间**（服务态令牌 + 目录
ACL + `viaApp` 溯源），而不是自己挂卷或自己接 S3 —— 否则租户的配额、审计、预览、清理
全都绕过了平台。

### 已经立过的红线（同一条道理，都是「平台有、别自建」）

- **审计**：不自建审计表/页，推 `POST /api/v1/audit`，租户在「设置 › 审计」一处看全。
- **通讯录**：不做「同步通讯录」到自己库，按需用选人组件挑。
- **配额/套餐**：见下一节。**计费**：平台级 Credit 服务，同样不自建。
- **租户级配置**：放门户的 App 配置页（AppForm Section），不是 App 内的设置页。
- **AI 功能**：终端用户面**不出现模型选择器**，服务端按场景解析模型。

## 有配额需求？用平台的套餐管理，不许自建

**红线：外部服务不自建配额/套餐/计费。** 配额的唯一事实源是平台的套餐矩阵；对方在自己库里存
一份「本租户上限」，第一次调档就会与门户对不上，而且**没有任何东西会告诉你它们分叉了**。

- 两个模型**二选一**，都只有 **backed listing** 可声明：`seatBased`（计门户成员用不用得了这个
  App，门户在 TDT 签发时收口，App 侧零代码）/ `seatRoles`（App 自管账号或资源，门户只发数、
  App 自己在建号路径上收口）。
- **role 由对方在 `app.manifest.json` 里声明**（不需要门户改代码）；**各套餐的数值进不了
  manifest，也进不了 `LISTING_DEFS`**，只有平台管理员在控制台能写 —— 应用侧一条写路径都没有。
  这是有意的：数值是商务面，改它等于改可售卖档位。
- 对接面只有一个：`POST /api/v1/seats/quota`（服务态令牌 + `seats.read`）。**`available === null`
  ⇒ 该租户 unlimited，放行、不要收口** —— 把 null 当 0 是这个契约最容易踩的一处。
- ⚠️ `seats.read` 是 `SERVICE_ONLY_SCOPES`：对方 manifest 里出现 `serviceScopes: ["seats.read"]`
  **会被拒收**，评审时这是**当场打回的信号**，不是配置失误。正确路径是
  `privilegedServiceScopes: [{ "scope": "seats.read", "reason": "…" }]` —— 这是**申请**不是授予：
  必进「发布审核」，审批人逐条勾选确认后 SA 才拿到（reason 原文展示给审批人）。
- **配额 ≠ 用量**，别混：配额答「还能不能再建一个」，用量答「这段时间用了多少」，后者走
  `usageReporter` + `usage.report`。计费方向另有平台级 Credit 服务，同样不自建。

真要落一版（选模型、写 manifest、配数值、上报已用、存量租户、三态处理、判定某个数该不该进
套餐表）⇒ 读 [references/quota-and-seats.md](references/quota-and-seats.md)。

## 一盒联调与冒烟

外部团队不用克隆本 repo：平台给两个门户镜像 + `deploy/` 目录，对方出 manifest + `compose.env` + 自己的镜像。步骤、冒烟命令与排查速查表见 [references/registration-and-onebox.md](references/registration-and-onebox.md) §4–§6。
