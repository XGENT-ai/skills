# 外部 App 交付、环境供给与注册

先按 [配置分层](config-placement.md) 确定每个参数的归属。本参考用于镜像交付、迁移、必需环境变量与部署接线；运行时授权见 [集成契约](integration-contract.md)。

品牌图：在开发者应用的 App 概览上传 SVG / PNG / WebP（≤2 MiB），取得门户平台存储的 `/api/assets/app-logo/<listingKey>/<sha256>.<ext>` URL，回填源清单 `logoUrl` 并递增版本。只能引用本 App 这一路径；外链、data URI、`/apps/<key>/` 图、查询串与其他 App key 均拒。`icon` 仍填命名字形，导航不显示品牌图；品牌图原比例原色展示，坏图回退字形。需要已配置的平台自用存储池与显式 `developer:action:branding.manage` + 维护者资格，租户 App 编辑者不能改此字段。

## 交付物评审 checklist（收外部团队东西时逐项验）

1. 镜像：监听 8080、网络别名 `<listingKey>-server`、amd64+arm64、**不烘焙 .env**、`docker save` tar + sha256 或私有 registry；
2. `app.manifest.json`（外部团队 repo 里的单一事实源）；
3. env 契约表：**镜像实际读取的变量名**（镜像自有前缀与门户契约别名的映射必须写清；有的镜像不读裸 `PORT`）；
4. **迁移入口**：迁移文件在镜像里、二进制上有一个迁移 argv（见下节）。「迁移脚本留在自己 repo 里」= 不合格；
5. 运维口径：是否需每租户 bootstrap（需要则 `tenants.id` 必须 = 门户租户 UUID）；**`/health` 必须通过 [健康契约验收](health-contract.md)**，按平台同一判据验证正常与依赖故障；信封响应不能只检查 HTTP 200；
6. **是否复用平台能力**：文件按 [文件与对象存储](platform-capabilities.md#文件与对象存储两条接入路径) 选择 Files 或平台 `objectStorage`，后者须明确 App 自管的租户隔离、计量与清理；通知、审计、定时任务、配额仍按 [平台能力](platform-capabilities.md) 复用平台面；
7. **租户自助凭证**：这个服务有没有「租户拿自己的程序直接调你」的场景（日志/指标摄取、批量导入、CI 回写）？有就检查那条 scope 在不在 manifest 的 **`scopes`** 里 —— 不在的话租户面开不出凭证，只能退回「找平台管理员代建」（见 [integration-contract.md §3.1](integration-contract.md)）；
8. **配额诉求**：有没有「每租户能建多少」这类上限？有就走平台套餐（见 [quota-and-seats.md](quota-and-seats.md)），**不接受对方自建**；
   角色本身该由对方在自己的 `app.manifest.json` 里声明 `seatRoles`；`serviceScopes` 里出现 `seats.read` 仍是当场打回的信号，但对方可以在 `privilegedServiceScopes` 里**申请**它（带 reason，进「发布审核」逐条确认批准后授予）；
9. 对接契约文档：按 [contract-doc-template.md](contract-doc-template.md) 的结构与精度。**没有这份文档的接入不算完成**——新服务（如任务网关）要先补。
10. **终端用户**：如有注册、登录、找回、密码管理、注销或个人数据请求，按 [终端用户参考](end-users.md) 核对身份 owner、realm、profile/client、服务权限、容量与业务数据分工；区分管理 UI、App API 和本人流程，不把成员 OAuth 当顾客认证。

## 一盒初始化与升级

宿主 skill 与镜像分别更新：先 `npx skills add XGENT-ai/skills` 更新 `portal-dev-setup`，
从实际安装目录核对 `onebox.sh` 的 SHA-256，再按发布说明的匹配镜像操作。
`requiredEnv[].generate` 的最低门户提交 / `APP_VERSION`、一盒 tag、公开 skill 提交和脚本摘要
目前均**未发布**；完整证据状态见 `portal-dev-setup` §6，不能默认当前 `latest` 已支持。
首次联调用 `portal-dev-setup` 的 `onebox.sh init` → `up`；已有一盒换版本用
`upgrade [--image <ref>]` → `up`；沿用 latest 先 `pull` 更新缓存，已加载的本地新镜像可离线升级。
不使用 `init --force` 代替升级。升级保留配置、generated/、backup/、
端口与卷；`up` 仍重种门户库，不重种 App 自己的库。自带对象存储为 RustFS，保留 `MINIO_*`
配置与 `minio:9000` 网络别名；控制台路径为 `/rustfs/console/`。存量对象由切换脚本复制、核对后才切换，
切换后有新写入时不能直接回旧卷，细节见 [registration-and-onebox.md](registration-and-onebox.md)。

## 注册布线（细节见 registration-and-onebox.md）

- **dev / 一盒**：`bun run register-app <manifest>`（幂等，生产拒跑）= upsert listing + 直写服务账号 + 写发起方 App Secret + 写 `/svc` 白名单 map。⚠️ `exchangeInitiatorSecret` 的哈希在**首个租户**安装时还没有任何实例可布——先安装 App，**再重跑一次** register-app，否则发起交换 401（之后装的租户会随安装自动补布，见 registration-and-onebox.md §2.2）。日志里 `wired into N …` 的 N 是「**改了**几个实例」，0 也可能是「本来就对」——判据是比哈希，见 registration-and-onebox.md §2.2。
- ⚠️ **发布 ≠ 租户能看见**：租户市场对 `tenant_listing_grants` 是 fail-closed。**dev 的 `register-app` 会自动授予当时已存在的租户**（非 service 型，输出里有 `租户可用应用:` 一行）；**生产不自动**（提案批准 / 控制台导入都不授予——语义是平台管理员显式勾选）。症状是「控制台显示已上架、租户市场里却根本看不到」——去 平台 › 租户管理 › 租户 › 「可用应用」勾上保存（registration-and-onebox.md §2.1）。
- **生产（标准路径）**：清单事实源在**对方仓的 `app.manifest.json`**，经
  **发布提案**到达：平台管理员在 控制台 › 应用市场 › 接入新应用 按 key 签发 `xrel_` 令牌
  （无行先建 draft 占位），对方 `publish --manifest … --dist … --image …` ⇒ 首次必进
  「发布审核」队列 ⇒ 批准后更新清单、服务账号与已装租户，后端和网关分别交付。
  SA / 交换密钥一次性回显给审批人；`requiredEnv[].generate` 声明的独立密钥永不回显。
  平台运行配置由管理员维护，App 团队无需修改门户代码。
  特权服务态 scope 经 manifest 的 `privilegedServiceScopes` 申请、逐项审批后授予；
  用量指标经 manifest `usageMetrics` 声明（治理档）。批准不等于部署与网关已经生效，
  交付判据见 [公开入口](public-entrypoints.md)。
- **后续版本**：对方 CI 自助（`xgent-app-release` skill）——无治理变更自动通过，
  治理变更进「发布审核」等平台批准。
- **安装 scope 同步**：外部 listing 自动发布、重试、版本或品牌图同步保留现存安装 scopes，不自动增加新 scope；租户管理员须显式调用 `POST /api/admin/market/listings/:key/update` 取得当前获批 scopes（安装版本已相同也可调用）。新安装取得 listing scopes；门户内置定义同步仍 union。
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
`{ key, renamedFrom }` 一条，门户会把旧键在 `descriptor.env` 里的值搬过去。普通条目在
`envFile` 中的旧名仍判缺；带 `generate` 的条目可沿用可读文件中的旧名，App 必须兼容消费，
不改文件、不复制值、不生成新主键。旧键已由平台保管时拒绝改名。仍有一件它管不到：对方
**不写进 `requiredEnv`** 的静默改名依旧无门可拦 —— 症状是服务
换版后 `refusing to start`，而发布方那侧只看到成功。2026-08-19 知识库把 `XGENT_OPENAI_*`
整族改成 `XGENT_EMBED_*` 就是这么崩了几小时的；只改其中一个 key 更坏 —— 服务起得来，
但 base URL/model/**dims** 静默回落默认值。所以收交付物时仍要求：改必需 env 必须**同步改
manifest 的 `requiredEnv`**（改名走 `renamedFrom`），并把「本版新增/改名的必需 env」列进 checklist。
**业务参数别塞进 `requiredEnv`**：审批人只懂环境，不懂你的业务。能在代码里给缺省值就不进清单；
确实要让平台部署时覆盖，就在顶层 `requiredEnvNotes`（`{ 键名: 一句说明 }`，≤200 字，只给需要解释的键写）
里写清填什么、不确定时填什么，审批屏会贴在键名旁。放哪一层的完整判断见 [配置分层](config-placement.md)。发布面本身
（令牌、tag 不可变、`--wait` / `--wait-review` / `status`）见 `xgent-app-release` skill。

### 值从哪来：键名约定与独立密钥生成

| 档 | 键 | 值从哪来 |
| --- | --- | --- |
| **平台注入** | `PORT` · `<PREFIX>_SERVER_PORT` · `PORTAL_INTROSPECT_URL` · `API_BASE_URL` · `PORTAL_BASE_URL` · `<PREFIX>_SA_CLIENT_ID` · `<PREFIX>_SA_CLIENT_SECRET` · `<PREFIX>_APP_SECRET`（声明了 `exchangeTargets` 时） | 平台换版时自己算并注入，没有人手填 |
| **自动供给** | 库连接串 · Redis 地址 · 对象存储五键（每应用一桶）· Kafka 五键 · 向量库三键 | 批准时平台按登记的服务代建资源 / 取地址并注入 |
| **随机生成** | App 独立密钥，声明 `{ key, generate: { bytes: 32, encoding: "base64" } }` | 批准时生成、加密保管；换版复用，值永不回显 |
| **自定** | 其余**环境依赖**（第三方地址、第三方已有密钥…） | 平台管理员手填：非密钥进 `descriptor.env`，密钥进主机 `envFile` |

`generate` 仅收 `bytes`（16–64 整数）与 `encoding`（标准带填充 `base64` / 小写 `hex`）。
不可用于平台派生键或服务供给目标。套件共享后端里同名共享键只由一个成员 `generate`，其余成员以
字符串 `requiredEnv` 消费（先批生成者；两名生成者拒绝）。
已有主键或匹配保管值保留；规格冲突、来源冲突、不可读 `envFile` 均硬拦，不能填新值绕过。
`generate` 的增加、撤回、规格变化都需审批；已生成值不覆盖、不提供读取或轮换入口，丢失应恢复备份。
一盒新版 `add` 在共享的 `<一盒目录>/compose.env` 中首供并稳定复用，同名键共享同一值；主 App 与其他 App
均可能读到，适用于可信团队调试。生产则按 App、按 brand 隔离，不以一盒结果证明生产隔离。

业务参数不在这张表里：先给代码缺省值；要进清单就配 `requiredEnvNotes`（见上一节末尾与 [配置分层](config-placement.md)）。

`<PREFIX>` = `listingKey` 全大写、连字符换下划线。`PORTAL_BASE_URL` 是门户**公开源**（也是调
别的 App `/svc/<key>` 的基址），`API_BASE_URL` 是内部 portal-api，**不代理 `/svc`**。

**门户内 App 的服务地址由代码派生，不逐个要求平台手填。** 以 Page Builder 为例：

| 原来的地址变量 | 应用代码使用的默认基址 |
| --- | --- |
| `LLM_GATEWAY_SERVER_URL` | `${PORTAL_BASE_URL}/svc/llm-gateway` |
| `FILES_SERVER_URL` | `${PORTAL_BASE_URL}/svc/files` |

接入或审核时核对这几项：

- **清单**：`requiredEnv` 声明 `PORTAL_BASE_URL`，保留其他确实必需的键；上面两个可推导的
  地址不再列为必填。平台注入的是门户基址，**不会自动注入这两个旧变量**，也不要把派生地址值写进 manifest。
- **实现与迁移**：代码去掉门户基址末尾的 `/` 后拼接 `/svc/<key>`，再追加具体 API 路径。
  旧变量可保留为本地调试或特殊部署的可选覆盖项。代码默认值、启动校验、清单与新镜像同步交付，
  不能只删 `requiredEnv`；平台侧核对已有覆盖值是否保留，并在未设置旧变量时验证实际调用。
- **权限**：统一入口不替代鉴权。代表用户调用仍需 `exchangeTargets`、目标 scopes 与令牌交换；
  后台调用仍用获准的服务账号权限，具体姿态见 [平台能力](platform-capabilities.md)。

这条约定针对门户内的 App 服务；第三方 API 等无法从门户基址推导的地址仍按需声明。
发布操作见 `xgent-app-release` skill「调用门户内的其他 App：从 `PORTAL_BASE_URL` 派生地址」。

**服务怎么绑到键**：键名按约定命名（`<PREFIX>_DATABASE_URL` / `REDIS_CONN_STRING`），再声明要哪条服务 ——

```jsonc
"requiredServices": [{ "kind": "postgres", "name": "main" }]
```

⚠️ `requiredServices` 的每一条**只收 `name` / `kind` / `note`**，多一个字段整份清单直接拒收。
只写 `requiredEnv` 的约定名也认（用该类型的缺省服务）。**键名不按约定来（`ZO_META_POSTGRES_DSN`
这种上游定死的名字）也能发**，只是平台管理员要在审批屏上逐个点一下「从平台服务取值」，
每次发版多等一轮。收交付物时把这一条当作可读性问题看：不是错，是白等。

**Kafka**：声明 `requiredServices: [{"kind":"kafka","name":"main"}]`，并在 `requiredEnv` 列出
`<PREFIX>_KAFKA_BROKERS` / `_KAFKA_USERNAME` / `_KAFKA_PASSWORD` / `_KAFKA_SASL_MECHANISM` /
`_KAFKA_TOPIC_PREFIX`（每项均带 `<PREFIX>`）。机制固定 `SCRAM-SHA-512`，用户名 `app_<key>`
把连字符改为下划线；主题、消费组、事务 ID 的前缀 `app.<listingKey>.` **保留连字符**。
App 显式建主题、副本数 1，不能更改保留期或增加分区；不申请管理员密码。平台是单节点
at-least-once 消息服务，不是业务记录系统，App 负责重复消费、重连与租户隔离。
一盒新版 `add` 自动供给相同权限并沿用已有密码，但整份 `compose.env` 仍注入所有容器，
不提供容器间密钥隔离。完整联调步骤见 [注册布线与一盒](registration-and-onebox.md#44-kafka-接入)。

**向量库**：声明 `requiredServices: [{"kind":"vectorStore","name":"main"}]`（每个 App 至多一条，不写
`envKey`），平台批准时建库 `app_<key>`（连字符改下划线）与只授该库权限的用户，注入
`<PREFIX>_VECTOR_DB_URI` / `_VECTOR_DB_TOKEN` / `_VECTOR_DB_NAME`；只写这三个 `requiredEnv` 不触发供给。
App 每个请求带 `dbName`，不持有 root；向量数据须能从 App 自己的权威数据重建，租户隔离由 App 负责。
这三个键不要写进 `descriptor.env` / envFile（会覆盖平台注入）。只服务与门户同机的 App 容器。
联调见 [注册布线与一盒](registration-and-onebox.md#45-向量库接入)。

⚠️ 服务账号密钥**不再要求进 envFile**：门户保管并在换版时注入容器，明文只在批准那次一次性
回显（留给对方本地联调）。收交付物时别再问对方「密钥写进 envFile 了吗」。
另外，平台级能力（如日志写入）会直接出现在对方的服务令牌里，**不需要在 manifest 里申请**。

## 收交付物时一并核对「跑在哪、要什么服务」两节

manifest 还有两节**同形的审批前置闸**（与 `requiredEnv` 同一挂载点、同一失败语义）：

- `deployRequirements`（`{ region?, size?, gpu?, network? }`）—— 这个 App 的后端需要什么样的
  机器。批准那一刻按本环境的资源池逐维匹配，不满足就拒绝生效；通过时把落点固定到命中的
  那台。**需要 `deployDescriptor`**，且只收**名字与档位** —— 网段 / IP / URL 提交即拒。
- `requiredServices`（`[{ name, kind, note? }]`）—— 运行需要哪些外部服务。批准时比对
  本环境**已登记**的「已具备服务」（控制台 › 平台设置 › App 清单目录），未登记或缺条目就
  拒绝生效。登记表描述已有服务；支持自动供给的类型还会按所选档案为 App 建资源与注入约定键。

收交付物时按这两节问三个问题：**它要跑在哪个网络里**（对方写的名字与本环境登记的名字对不
对得上）、**它连的那些基础服务本环境有没有并且登记了没有**、以及**远端主机上没有门户的
compose 网络** —— 一份在一盒里能跑的 descriptor，`env` 里写着 `postgres:5432` 时搬到远端
落点上是连不通的，那正是 `requiredServices` 要把「本环境有什么」摆到审批人眼前的原因。

被拒时对方只会收到一句泛化的「当前环境不满足…，提案已转入待审核队列」（池、网段、机器名
一个字都不出门户控制台）—— 所以**差在哪要由门户侧的人告诉他们**，别让对方对着一句话猜。

## 一盒联调与冒烟

外部团队不用克隆本 repo：平台给两个门户镜像 + `deploy/` 目录，对方出 manifest + `compose.env` + 自己的镜像。步骤、冒烟命令与排查速查表见 [registration-and-onebox.md](registration-and-onebox.md) §4–§6。

本地调试的 PostgreSQL、Redis、RustFS（S3）、Kafka 必须复用共享 `xgent-onebox`，不另起独立实例，也不连接 portal 仓自己的独立开发环境。宿主热重载与容器的接线边界见该参考 §4、§4.2 与 §4.4。
