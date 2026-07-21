# APP-INTEGRATION — 新增 App「零改码 · 零重部署」重构计划

> Register: **platform / infra**。目标读者:平台团队。
>
> 现状:新增一个独立后端 App 要改约 10 处代码(`scopes.ts` / `manifests.ts` / `env.ts` /
> `provisioning.ts` / `seed.ts` / CORS / `MicroAppHost` / `Dockerfile` / `Caddyfile` /
> postgres init),且大多**烘焙进镜像**,所以上线后新增 App 必须**重建并重部署门户**
> (portal-api + web + proxy)——这会中断在线服务。
>
> 战略背景:基座稳定后将**批量接入大量 App**(含第三方)。届时"每个 App 改码 + 重部署"
> 不可持续。本计划把"新增 App"从**改代码 + 重部署**收敛为**写配置(DB)+ App 自己的容器
> 按需起**,平台三件套(portal-api / web / reverse-proxy)镜像**保持不动、不重启**,并让
> 平台管理员**自助上架**。
>
> 关联:[`SSO与App开发指引.md`](../docs/SSO与App开发指引.md)(契约)、[`DEPLOY.md`](./DEPLOY.md)
> (按需部署)、[`ACL.md`](./ACL.md)(权限模型)、[`知识库后台接入与本地联调指南`](../docs/知识库后台接入与本地联调指南.md)(第一个第三方 App 的接入,本计划的触发案例)。
>
> 本版已采纳一轮 review:新增 §4.0 服务身份规范(Phase 0a)、§4.3 动态路由安全边界(map 白名单 + 网络隔离)、§4.4 资产托管落到共享卷、Phase 0b scope 全链路数据化、Phase 1 覆盖两个配置页。

---

## 0. 实现状态(2026-06)

Phase 0a–4 已实现并验证(`bun run verify:all` 7/7;typecheck 18/18)。控制面/数据面逻辑就地验证;编排面(容器/集群实拉起)按本计划分期(§9.3)+ 验证条件留待 live stack。

- **P0a 服务身份**:自省补 `listingKey`(= contentOwner);6 个资源服务器 gate 改按 `listingKey ?? aud` 鉴权;独立后端 App 安装禁 `-2` 漂移(占用即 `ALREADY_INSTALLED`,卸载残留可回收)。
- **P0b 清单数据化**:`ListingInput`/`create|updateListing` 收 `aclManifest`/`scopeLabels`/`serviceBaseUrl`/`exchangeTargets`/`embedCsp`/`deployDescriptor` + 写时校验(scope 命名空间、manifest 形状、env 键白名单);`dto` scopes→`string[]`;`MarketListingForm` 平台 scope 勾选 + App scope 自由录入 + 高级 JSON;`service-accounts.sanitizeScopes` 放行已注册 App scope;同意页 scopeLabels(`resolveI18n`);install 按 `exchangeTargets` 连交换白名单。
- **P1 服务发现**:`GET /api/apps/:key` 暴露 `serviceBaseUrl`(默认 `/svc/<key>`);`MicroAppHost.callService` + files/llm-gateway 配置页运行时解析;删 web 烘焙的 `SERVICE_REGISTRY` + `Dockerfile` 的 `VITE_*_SERVER_BASE`;dev 加 vite `/svc/*` 同源代理。
- **P2 动态 `/svc`**:Caddyfile 收敛为一条 `path_regexp` + 占位上游 `<key>-server:8080` + allowlist `map`(内置 inline,配置 App 走热加载 `import`);7 个后端统一内网端口 8080(compose/k8s values+template/registry/configmap/devkit);网络隔离用既有 K8s NetworkPolicy。Caddyfile `caddy validate` + 路由行为容器实测。
- **P3 前端托管**:Caddyfile 收敛为一条 `/apps/*` 通配 + per-key CSP `map`;portal-api `POST …/frontend` 解包 dist→共享卷 + 写 per-App CSP 片段;proxy entrypoint 把内置 dist seed 进共享卷(配置 App 上传保留);compose 共享卷 `apps-data`/`caddy-apps-csp`/`caddy-svc-allow`;控制台编辑页加上传控件。
- **P4 数据化部署**:`resolveDeployable`(代码注册表 ∪ 清单 `deployDescriptor`);deploy-controller 读 DB + 配置 App 自迁移(跳过门户脚本)+ ready 后写 allowlist + 热重载;Compose 驱动按描述生成 override 服务;K8s 驱动按 §7 模板创建 Deployment/Service(固定 securityContext/limits、digest 镜像、无任意 podSpec)+ RBAC `allowDescriptorDeploys`(单命名空间)。
- **留待 live stack 验证**(本机无多容器/集群):容器实拉起 config App(Compose override / K8s 模板创建)、Caddy allowlist 跨容器热重载、K8s 配置 App 前端的 RWX-PVC 共享。
- **P5 清理**:就地清掉本次产生的孤儿(`csp_microapp` 片段、失效 import);`seed.ts` 复用 provisioning、`env.ts` per-App 瘦身、K8s 失效 `*_UPSTREAM` env 为可选后续。

---

## 1. 目标与非目标

**目标**
1. 新增/上架一个 App **不改任何代码**——平台管理员只创建一条 DB 配置(市场清单 + 部署描述)。
2. 系统**已部署**时新增 App **不重建、不重启**平台三件套,在线可用性不受影响。
3. **批量可扩展**:后端容器**按需起、闲时缩零**,平台管理员**自助上架**,无需逐个手工运维。
4. 保持现有安全不变量(scope 不可提权、deploy 不吃自由文本拼 shell、平台管理员治理、最小 RBAC)。
5. 现有 7 个内置 App **行为不变**,平滑迁移(代码注册表作为过渡期回退)。

**非目标**
- 不让**租户管理员**自助接入任意后端镜像/路由(那是平台管理员的治理权,见 §7)。
- 不改第三方 App 自身的交付形态(仍是「后端镜像 + 前端产物 + 契约」,见接入指南)。
- 不追求把**内置** App 的代码定义一次性全删——它们可继续走代码注册表,新 App 走配置(两条路并存)。

---

## 2. 现状盘点:每处改动「烘焙在哪 / 是否已是数据」

| 触点 | 烘焙位置 | 现状 | 目标 |
| --- | --- | --- | --- |
| `manifests.ts`(ACL) | api 镜像 | **代码**:`ListingInput` 不收 `aclManifest`,只能取代码注册表 | **数据**:清单 API 收 `aclManifest`(校验后入库) |
| listing 内容(名称/scopes/navItems/embedUrl/deps) | DB | **已是数据**:`ListingInput` 已支持,控制台 API 可建 | 保持;补字段(见下) |
| scope 词表 `SCOPES` | api + web 包 | **半数据**:mint 用 `app.scopes`(DB),`SCOPES` 仅做 TS 类型 + 同意页文案 | scope **标签**随清单入库;app 命名空间 scope 按约定校验 |
| `env.ts` 每 App 4 字段 | api 镜像 | **代码**:`<KEY>_APP_URL/_RESOURCE_KEY/_SA_CLIENT_ID/_APP_SECRET` | **数据**:SA 走控制台 API 建;服务地址进清单字段 |
| `provisioning.ts` `LISTING_DEFS`/`SA_DEFS` | api 镜像 | **代码**:内置 App 的出厂目录 | 仅内置 App 用;新 App 走 API,不进此表 |
| `seed.ts` | api 镜像 | **代码**(且与 provisioning 重复) | dev 复用 provisioning;与本计划正交 |
| CORS 白名单(`index.ts`) | api 镜像 | **代码**:env 列表 | **数据**:从已装 App 的 origin 动态加载;或强制走宿主代理(免 CORS) |
| `SERVICE_REGISTRY`(`MicroAppHost`) | **web 包** | **代码**:`appKey → 后端 base`,烘焙进前端 | **运行时**:host 从 API 取服务地址 |
| `Dockerfile` `VITE_*_SERVER_BASE` | **proxy 镜像** | **代码**:每 App 一个 build-arg | **删除**:前端按约定 `/svc/<key>` 或运行时取 |
| `Caddyfile` `/svc/<key>` + `/apps/<key>` | **proxy 镜像** | **代码**:每 App 两段静态路由 | **动态**:`/svc/*`→Caddy 动态上游(命名约定 + 已注册 key 的热加载 `map` + 网络隔离,§4.3);`/apps/*`→一条通配 + 运行时投放产物 |
| postgres init | proxy/infra | **代码**:每库一行 | App 自管其库(独立后端的部署职责) |
| `DEPLOYABLE_APP_SERVICES`(deploy 注册表) | api 镜像 | **代码**:按需部署的代码级 allowlist(故意不吃租户输入) | **数据**:平台管理员治理的部署描述表(保持不拼 shell) |

**关键洞察**:真正阻碍"零重部署"的是**烘焙进 web/proxy 镜像的三样**——`SERVICE_REGISTRY`、
`Dockerfile` 的 VITE 变量、`Caddyfile` 路由。把这三样**运行时化/动态化**,再加上 ACL 清单
收口为 API,即可实现新 App 全配置化。scope/CORS/SA/DB 要么已是数据,要么改动很小。

---

## 3. 核心原则

1. **平台三件套镜像不可变**:新增 App 绝不触发 portal-api / web / reverse-proxy 的重建或重启。
2. **App = 一条配置 + 一个独立容器**:身份/权限/路由是 DB 数据;后端是 App 自己的容器(独立生命周期,挂了不影响平台)。
3. **单一事实源**:一条**市场清单(listing)**承载 App 的全部声明(身份 + scope + ACL + 导航 + 服务地址 + 部署描述 + 依赖 + 交换目标),其余配置一律**派生**,不再各处重抄。
4. **安全不降级**:platform-admin 治理;scope 命名空间隔离不可提权;网关沿用四道闸;deploy 永不把租户/外部输入拼进 shell/K8s 命令。
5. **渐进、可回退**:每阶段独立可上线;代码注册表在过渡期作为 listing 缺字段时的回退,内置 App 不受影响。

---

## 4. 目标架构

### 4.0 服务身份规范(Phase 0 前置 —— 必须先定,否则后续在 aud / 路由 / 下架语义上返工)

今天三个"名字"会不一致,动手前必须钉死:

- **`listingKey`**:清单/全局标识(`/svc/<key>`、`serviceBaseUrl`、`exchangeTargets`、`callService(name)` 都用它)。
- **`appKey`**:**安装态**标识。`freeAppKey()` 在租户内冲突时会变成 `<listingKey>-2`([`market/service.ts`](../apps/api/src/modules/market/service.ts) `freeAppKey`)。
- **TDT `aud`**:mint 用的是 **`app.appKey`**([`token/service.ts`](../apps/api/src/modules/token/service.ts) `mintForApp`),而资源服务器固定校验 `aud === env.audience`(= listingKey,[`files-server/src/lib/gate.ts`](../apps/files-server/src/lib/gate.ts))。

→ 一旦 `appKey` 偏离 `listingKey`(`files-2`),`aud=files-2` 会被 `aud!==files` 直接拒。**这是既有潜在 bug,不是本计划引入的**,但本计划全程以 `listingKey` 为键,必须正面解决。**规范(建议)**:

1. **独立后端 App 禁止 appKey 漂移**:有 `serviceBaseUrl`/`deployDescriptor` 的清单,安装时 `appKey` 必须 == `listingKey`;同租户重复安装直接 `ALREADY_INSTALLED`,不生成 `-2`(`-2` 仅对无后端的纯前端/内容类 App 保留)。
2. **后端按"规范身份"鉴权,而非裸 `aud`**:introspection 响应补 `listingKey`(= `contentOwner`),资源服务器校验 `listingKey === 本服务`,把"安装态 appKey 是什么"对后端透明。两条任选其一即可解决,**1 最省、2 最稳**(可同时做)。
3. **`callService(name)` 与 `/svc/<key>` 一律用 `listingKey`**;host 解析服务地址时按 `listingKey` 查,不用安装态 appKey。

> 验证关卡:在 Phase 1 前,跑一个"同租户二次安装 + 改名"用例,确认 aud / `/svc` / `callService` 三者对齐、且下架/停用后 TDT 失效(av bump)+ 路由按 §4.3 的 map 摘除。

### 4.1 清单即单一事实源(扩字段)

`marketplace_listings` 增补开发者字段(均经控制台 API 读写、平台管理员治理):

- `aclManifest`(已有列,**补进 `ListingInput` + 写时校验**)——彻底去掉对 `manifests.ts` 的依赖。
- `scopeLabels: Record<scope, i18nText>`——App 命名空间 scope 的同意页文案随清单走,去掉对 `SCOPE_LABELS` 的依赖。
- `serviceBaseUrl`(新)——独立后端的可达地址(prod 同源 `/svc/<key>`;或外部 origin)。供动态网关 + host `callService` 解析。
- `deployDescriptor`(新,可选)——按需部署该 App 后端容器所需的**数据化**描述:`image` / `port` / `healthPath` / `migrate`/`provision`/`bootstrap` 命令 / K8s 坐标。**仅平台管理员可写**,deploy-controller 据此执行(见 §7 安全)。
- `exchangeTargets: listingKey[]`(新)——它经令牌交换读取哪些 App(取代 `EXCHANGE_WIRING` 代码)。

> 内置 App 可继续由 `provisioning.ts` 盖章这些字段;新 App 由控制台 API 写入。两者最终都只是 listing 行。

### 4.2 Scope 运行时化

- mint 已用 `app.scopes`(DB),**无需改**。
- **校验放在配置写入时(运行期),不靠编译期类型**(已定):App scope 由 `string` 承载;写清单的 `createListing/updateListing` 校验每个 `scope` 要么 ∈ **平台基础 scope**(`userinfo/notification/directory/...`,仍由 `SCOPES` const 把守 + 契约测试覆盖),要么前缀 == 本 `listingKey`(防一个 App 声明别家/平台保留 scope)。
- `SCOPES` const 降级为**平台基础 scope** 词表;App 命名空间 scope 不再进 const。
- 同意页文案:已知平台 scope 用内置文案;App scope 用清单 `scopeLabels`;都缺则降级显示原始串(不阻断)。

### 4.3 动态 `/svc/*` 路由 —— Caddy 动态上游(已定:不加跳,用 Caddy 灵活性)

**决定**:不在 portal-api 加反代跳,直接用 Caddy 的**路径捕获 + 占位符上游**,一条通用规则吃下所有独立后端:

```caddyfile
# 一条规则覆盖所有 /svc/<key>/*。新增 App = 网络里起一个名为 <key>-server 的容器
# (统一内网端口) + 一条 DB listing。Caddy 永不改、不重载。
@svc path_regexp svc ^/svc/([a-z0-9-]+)(/.*)?$
handle @svc {
    rewrite * {re.svc.2}
    reverse_proxy {re.svc.1}-server:8080   # 占位符上游,Caddy 惰性解析
}
```

两个约定即"配置":
1. **命名约定 `<key>-server`** —— 现有 `files-server`/`spms-server`/… 已是此格式,Docker/K8s DNS 直接解析。
2. **统一内网端口**(如 `8080`)—— 让规则完全通用的关键。一次性把现有 7 个 App 的监听端口统一(改 env / compose `expose` / healthcheck / `registry.ts` 的 port,**非业务代码**);之后任何 `<key>-server:8080` 容器自动可路由。

> Caddy 占位符上游[需带端口、且会**关闭主动健康检查/部分负载均衡**](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)(动态地址使然)。可接受:就绪由 deploy-controller 的 `/health` 探测 + §4.3 的 `map`(下架即摘)兜;Phase 2 需实测被动健康检查/重试策略。

**安全边界(采纳 review 意见,修正纯命名约定暴露面过大)**:纯 DNS 约定会让**网络里任何叫 `<x>-server` 的服务**都可经 `/svc/<x>` 到达,比今天的静态白名单([`Caddyfile`](../deploy/caddy/Caddyfile))暴露面大。两道收口(都做):

1. **已注册 key 的热加载 `map`**:Caddy 用一个 `map`(或 `@matcher` named list)只放行**已上架/已部署**的 `listingKey`;deploy-controller 在上架/下架/停用时改这张表并调 Caddy **admin API 热重载**(零停机、非重部署)。→ 恢复"**未注册 / 已下架 = 路由不可达**"(回应 review 的"禁用不等于路由可达")。这比纯约定多一个"生成+reload"环节,但换来与今天等价的白名单语义,**值得**。占位符上游仍保留(免每 App 改 Caddy);`map` 只是 0/1 放行门。
2. **App 后端网络隔离**:App 容器放在专用 network,只有 reverse-proxy 可达;DB / Redis / 控制面**不命名为 `*-server`、也不在该 network**。即便 `map` 误放,可达面也被限定在"本就 aud 鉴权的 App 后端"。
- 兜底仍是:Caddy 只当哑路由,`aud`/scope 四道闸在各 App 后端(无容器→502;TDT 不对/已下架→av bump 后自省 `active:false` 拒)。三层叠加。

`sdk.callService(appKey, path)`:host 不再读烘焙的 `SERVICE_REGISTRY`,改为**运行时**从 `GET /api/apps/:appKey`(或一个 `services` 端点)取 `serviceBaseUrl`(prod 即 `/svc/<key>`)。→ **去掉 web 烘焙的 `SERVICE_REGISTRY` 与 `Dockerfile` 的 VITE 变量**。

### 4.4 前端托管运行时化 —— 同源上传产物(已定:B1)

**决定**:新 App 前端一律**同源**托管,**不**走外部自托管(B2 弃用)。好处:iframe 同源,CSP 维持 `frame-src 'self'`,**零 CSP 动态化**。

> review 正确指出:Caddy **不能原生读对象存储**,"从 store 提供"必须落一个具体机制。下面定方案。

- `Caddyfile` 把每 App 的 `/apps/<key>/*` 收敛为**一条通配** `/apps/*`(`root * /srv/www/apps`,per-`<key>` SPA 回退)。
- **产物投放(选定:解压到共享卷)**:平台管理员控制台上传 `dist` zip → portal-api 校验后**解压到 `/srv/www/apps/<key>/`**(reverse-proxy 与 portal-api 共享该卷)。Caddy 照旧只服务本地静态文件(它的强项),无需插件、无需对象存储桥。上传/替换是"写卷 + DB 记元数据",**不动镜像**。
  - **多节点警告**:共享卷在 K8s 上需 **RWX PVC**(并非所有 storageClass 支持)。当前线上是**单节点 KubeSphere**,RWX/hostPath 可行;若将来多节点,改用**备选:portal-api 资产代理**(`/apps/*` 走 portal-api,从对象存储流式回源 + 缓存头)——多一跳但对存储无 RWX 要求,且能顺带做下条的 per-App CSP。两者择一,先上共享卷。
- **per-App CSP**:同源托管下 CSP 基本恒为 `'self'`;唯一例外是个别 App 的 `connect-src`(如 files 直传对象存储)。这类例外集合很小且可枚举——由"承载 index 的那一跳"按清单 `embedCsp` 下发(共享卷方案:Caddy 用 `map` 按 `<key>` 选 CSP 片段;资产代理方案:portal-api 直接设 header,更简单)。
- 内置 App 仍可把 `dist` 烘焙进镜像(`/srv/www/apps/<key>`);通配规则对"镜像内置目录"与"上传到卷"二者透明(两条路并存)。

### 4.5 CORS / CSP(因 §4.3+§4.4 决议,基本归零)

- **CSP**:前端同源托管(B1)→ `frame-src 'self'` **维持不变,无需动态 CSP**。仅个别 App 的 `connect-src` 例外(如 files 直传对象存储)由清单 `embedCsp` 声明、运行时合成。
- **CORS**:iframe 调后端一律走**宿主代理 `callService`**(经 Caddy `/svc/<key>` 同源到达),App 后端只需信任门户一个源 → **per-App CORS 白名单消失**,`index.ts` 那条 env 列表不再随 App 增长。

### 4.6 后端容器的数据化按需部署(P4 —— 已定:要做,面向批量 App)

**目标**:批量上 App 后,后端容器**用时才起、闲时缩零**,平台管理员**自助**填一条描述即可上架,无人工起容器。

- `DEPLOYABLE_APP_SERVICES`(代码 allowlist)迁为 **DB 表 `app_deploy_descriptors`**,字段来自清单 `deployDescriptor`,**仅平台管理员写**。结构(参数化、字段白名单,**不放任意 spec/shell**):
  ```
  { listingKey, image (digest 固定), port (统一 8080), healthPath,
    migrate?, provision?, bootstrap?,    // 取自受控字段,不拼自由 shell
    resources { cpu, memMi }, env? (键白名单), replicasIdle (0=缩零) }
  ```
- deploy-controller 仍**只**执行该表里、平台管理员审定过的描述,沿用现有状态机 + DEPLOY 两层模型(服务部署 + 每租户开通)+ MicroAppHost 的"部署中"状态页。
- **与 Caddy 动态路由天然契合**(§4.3):App 缩零时容器不存在 → `/svc/<key>` 502 → 状态页提示"部署中";deploy-controller 拉起 `<key>-server:8080` 后,Caddy 惰性解析即通。**两者解耦:Caddy 管"流量怎么找"、controller 管"容器何时起"**。
- **关键设计抉择 —— K8s 驱动如何"物化"一个数据定义的 App**(§9 列为待决,但 P4 必须解决):
  - 今天 `KubernetesDriver` **只 patch 预先声明好的 Deployment 的副本数**(0→1),RBAC 按 resourceName 收死(KubeSphere §7)。
  - 数据化新 App 意味着要**按描述创建 Deployment/Service**(不只是 scale 已存在的)→ controller 需要在**单一命名空间内** `create/patch/delete Deployment` 的 RBAC(比今天宽)。
  - 收口:从描述**模板化**生成 Deployment(固定 securityContext / resource limits / 命名空间 / 不接受任意 podSpec),镜像只认 digest;Compose 驱动同理(profile 由描述生成)。详见 §7。
- Compose(dev/单机)驱动:由描述生成 `<key>-server` 服务 + profile,`pull` 外部镜像后 `up`。

---

## 5. 分阶段计划(每阶段独立可上线、可验证)

> 原则:先做**收益最大、风险最低**的"路由 + ACL 收口",让**新 App 零重部署**尽快成立;
> 内置 App 全程不动。

**Phase 0a — 服务身份规范(§4.0,先于一切动手)**
- 钉死 listingKey/appKey/aud 关系:独立后端 App `appKey == listingKey`(禁 `-2` 漂移)+ introspection 补 `listingKey`,后端按它鉴权。
- 验证:同租户二次安装/改名用例下 aud、`/svc`、`callService` 对齐;下架后 TDT 失效。

**Phase 0b — 清单收口 ACL + scope 全链路数据化(api + web 各发一次)**
- `ListingInput` + `createListing/updateListing` 接受并校验 `aclManifest`、`scopeLabels`;写时做 scope 命名空间校验(§4.2)。
- **scope 数据化是全链路,不止 API 写校验**(采纳 review):
  - 共享类型:`AppDetailDTO.scopes: Scope[]` → 放宽到 `string[]`(或 `Scope | \`${string}.${string}\`` 模板类型),见 [`packages/shared/src/dto.ts`](../packages/shared/src/dto.ts)。
  - 控制台:[`MarketListingForm`](../apps/web/src/pages/console/MarketListingForm.tsx) 从"只能勾 `SCOPES`"改为"平台基础 scope 勾选 + App 命名空间 scope 自由录入(带 `<listingKey>.` 前缀校验)"。
  - 服务账号:[`service-accounts.ts`](../apps/api/src/modules/platform/service-accounts.ts) `sanitizeScopes` 不再按 `SCOPES` 一刀切丢弃 App scope(放行已注册 App 的命名空间 scope)。
  - 文案来源:同意页 / 交换授权页标签 = 内置 `SCOPE_LABELS` ∪ 清单 `scopeLabels`,缺失降级显示原串。
- 验证:纯 API 建一个带完整 `aclManifest` + 自定义 scope 的 listing → 安装 → 角色矩阵 / `init.acl` / 自省 `permissions` / 同意页文案 与代码注册表 App 等价;内置 App 回退 `manifestForListing`/`SCOPE_LABELS` 行为不变。

**Phase 1 — 运行时服务发现(去掉 web 烘焙)**
- 清单加 `serviceBaseUrl`;`GET /api/apps/:appKey`(或 `services` 端点)暴露它。
- 改为运行时取 `serviceBaseUrl` 的**所有**消费点(采纳 review,不止 MicroAppHost):
  - [`MicroAppHost`](../apps/web/src/pages/MicroAppHost.tsx) 的 `callService`(删 `SERVICE_REGISTRY`)。
  - 后台配置页直连后端的两处:[`files-config-api.ts`](../apps/web/src/lib/files-config-api.ts)、[`llm-gateway-config-api.ts`](../apps/web/src/lib/llm-gateway-config-api.ts)(它们也读 `VITE_*_SERVER_BASE` 浏览器直连)。
- 三处都迁完才能删 `Dockerfile` 的 `VITE_*_SERVER_BASE`。
- 验证:现有 App `callService` + files/网关配置页照常;新 listing 设好 `serviceBaseUrl` 后 `callService` 通,且**没重建 web/proxy**。

**Phase 2 — Caddy 动态 `/svc/*` 路由(去掉 Caddy 每 App 后端路由)**
- 一次性把现有 7 个 App **统一内网端口**(env / compose `expose` / healthcheck / `registry.ts` port)。
- `Caddyfile` 用 §4.3 的路径捕获 + 占位符上游收敛成**一条** `/svc/*` 规则。
- 验证:现有 7 个 App 的 `/svc` 流量逐个冒烟无回归;新增一个 `<key>-server:8080` 容器 + DB 行后 `/svc/<key>` 即可达,**Caddy/镜像零改动、零重载**。

**Phase 3 — 前端托管运行时化(去掉 Caddy 静态块)**
- `Caddyfile` 收敛 `/apps/*` 通配 + 运行时资产源(B1 同源上传);控制台支持上传/替换 App `dist` 到对象存储。
- 验证:现有 App 静态资源照常;新 App 上传产物后 iframe 同源加载,CSP 维持 `'self'` 不破。

**Phase 4 — 数据化按需部署(已纳入;在基座稳定、开始批量加 App 时落地)**
- `app_deploy_descriptors` 表(§4.6 结构)+ deploy-controller 读 DB(取代代码 `DEPLOYABLE_APP_SERVICES`);平台管理员控制台维护描述(填镜像 digest/资源/命令)。
- **K8s 驱动**:从描述**模板化创建** Deployment/Service(固定 securityContext + resource limits + 单命名空间),不只是 scale;controller RBAC 受控扩为该命名空间内的 Deployment create/patch/delete(§7)。Compose 驱动:由描述生成服务 + profile + `pull`。
- 验证:平台管理员填一个**外部镜像**描述 → 租户安装 → 后端容器被拉起 + 迁移 + provision + ready(缩零→1)→ 第二个租户复用;全程**不动平台镜像**;Caddy 动态路由自动接上。

**Phase 5 — 清理与一致化(可选)**
- `seed.ts` 复用 `provisioning.ts`;`env.ts` per-App 字段瘦身;文档与接入指南更新为"配置化接入"。

> 路线:**Phase 0a→0b→1→2** 达成"新增**前端走宿主代理**的 App 零重部署";**Phase 3** 覆盖独立前端;**Phase 4** 在基座稳定后落地,把"上架 + 部署"也变成平台管理员自助的纯配置操作(批量 App 的关键)。

---

## 6. 可用性 / 零停机分析

- **平台三件套不重启**:Phase 1–3 后,新 App 不再触发 portal-api/web/proxy 重建。新 App 的"上线"= 写 DB + 部署它自己的容器(P4 后这步也由 deploy-controller 自动完成,平台管理员只填描述)。
- **配置生效**:listing 入库即时生效(查询走 DB)。Caddy 用占位符上游 + 统一端口的**通用规则,新增 App 不需要改 Caddy 也不需要 reload**(惰性解析 `<key>-server:8080`)。
- **故障隔离**:新 App 后端独立容器,崩溃只影响该 App(`/svc/<key>` 502),平台与其它 App 不受影响(Caddy/网关惰性解析上游)。
- **回滚**:App 出问题 → 控制台把它 `disabled`/下架(DB 操作,即时),无需碰平台。

---

## 7. 安全考量(不可降级)

- **治理边界**:`serviceBaseUrl` / `deployDescriptor` / `aclManifest` / App 命名空间 scope **仅平台管理员**可写(控制台 console API + `assertPlatformAdmin`)。租户管理员只能在**已上架**清单里安装,不能定义后端/路由(维持今天 `POST /api/admin/apps` 不收 `aclManifest`/`navItems` 的边界)。
- **scope 不可提权**:写清单时校验 scope 命名空间(§4.2);mint/exchange 仍取交集,永不扩张。
- **网关四道闸**:动态 `/svc` 网关沿用 `aud === appKey` / scope / requireAdmin / requirePerm;不因"来自门户"就信任(接入指南 §7.4 同款告诫)。
- **deploy 不吃自由文本**:`app_deploy_descriptors` 是平台管理员审定的数据;deploy-controller 以**参数化、字段白名单**方式执行(镜像引用、命令取自固定枚举/受控字段),**绝不**把任意字符串拼进 `docker`/`kubectl` 命令——保持今天 registry.ts 的不变量。
- **P4 的 RBAC 扩张(批量 App 的主要新风险)**:K8s 驱动从"只 scale 预声明 Deployment"变为"按描述创建 Deployment"。收口:① RBAC 仅授予**单一应用命名空间**内 `Deployment/Service` 的 create/patch/delete,不给集群级;② Deployment 由平台**模板**生成——固定 `securityContext`(非 root、只读根 fs、drop caps)、强制 `resources` limits、`enableServiceLinks:false`、不接受描述里的任意 podSpec;③ 描述仍仅平台管理员可写。
- **外部镜像供应链(批量后更重要)**:第三方镜像走**私有 registry + digest 固定**(不认 tag);建议接入镜像扫描 + 准入(只允许签名/扫描过的 digest);CSP/iframe sandbox 维持现有加固。

---

## 8. 迁移与兼容

- 内置 7 App 全程**不动**:`aclManifest` 取清单列、缺则回退 `manifestForListing`;`serviceBaseUrl` 缺则回退约定 `/svc/<key>`;`/svc` 网关对它们透明。
- 过渡期"代码注册表 + DB 配置"并存;新 App 一律走 DB;内置 App 可后续按需迁为纯数据(非必须)。
- 每阶段都有"现有 App 无回归"的验证关卡(§5),失败即回滚该阶段(平台镜像 tag 回退)。

---

## 9. 决议(已拍板)

1. **`/svc` 路由 → Caddy 动态上游**(不加 portal-api 跳)。路径捕获 + 占位符上游 + `<key>-server:统一端口` 约定,一条规则通用;**外加已注册 key 的热加载 `map` 白名单 + App 网络隔离**收口暴露面(§4.3,采纳 review)。
2. **新 App 前端 → 同源上传产物(B1)**;外部自托管(B2)弃用,CSP 维持 `'self'`(§4.4)。
3. **按需部署(P4)→ 要做**,排在 P0–P3 之后、**基座稳定 + 开始批量加 App 时**落地(批量场景下"用时才起/缩零 + 平台管理员自助上架"价值放大)。过渡期(P4 落地前)第三方后端先走"常驻 + 手动登记"(同知识库 devkit),不阻塞主线。最大成本/风险是 K8s 驱动从"只 scale 预声明 Deployment"扩为"按描述模板化创建 Deployment"——按 §7 收口。
4. **scope 校验 → 配置写入时(运行期)做**,不靠编译期类型;平台基础 scope 仍由 `SCOPES` const + 契约测试把守(§4.2)。
5. **内置 App 不强迁**:代码注册表与 DB 配置**两条路并存**;内置 App 维持代码定义,新 App 走配置(§8)。

> 动手顺序:**Phase 0a(服务身份规范)→ 0b(ACL+scope 数据化)**先行——0a 是 review 标的前置,不先定会在 aud/路由/下架语义返工。

---

*本计划随实现演进。落地以 `apps/api/src/modules/market`、`apps/api/src/modules/deploy`、
`apps/web/src/pages/MicroAppHost.tsx`、`deploy/caddy/Caddyfile` 的实现为准。*
