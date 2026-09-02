# CLAUDE.md 模板 · 外部 App 仓库

> **用途**：出仓 App（代码不在门户 monorepo，以「镜像 + dist + `app.manifest.json`」交付）自己仓库根目录的 `CLAUDE.md`。
> **用法**：整份复制到你仓库根目录另存为 `CLAUDE.md` → 把 `<APP_KEY>` / `<APP_NAME>` / `<PREFIX>` / `[方括号]` 换成你的事实 → 删掉不适用的小节（`service` 型删掉前端那几节；没有字典表就删字典表那节）→ 删掉本说明块（`✂️` 那一行连同以上全删）。
> **不要把它当参考文档留在原地**：这份文件的价值在于被 agent 每次读到。抄进去、删干净、只留你真的遵守的条款——留着做不到的条款比没有更糟。
> **姊妹模板**：`PRODUCT.md 模板` · `DESIGN.md 模板` · `CLAUDE.md 模板`（同一批下发，三份配套用）。
> **平台口径事实源**（下面每一条的出处，与本模板冲突时以它们为准）：你仓库里已装的 skill ——
> `portal-external-app`（接入契约 · `app.manifest.json` · `/svc` 与健康检查）· `portal-micro-app`（SDK 握手 / 版头 / consent / iframe 坑）·
> `xgent-app-release`（发布提案 · 产物打包 · 排错）· `xgent-image-push`（镜像构建与推送）· `portal-dev-setup`（一盒本地联调）·
> `portal-app-exchange`（跨应用调用）。**门户仓的文档你访问不到，别在本文件里引它的路径。**

<!-- ✂️ ————————— 以上是模板说明，复制后整块删掉 ————————— -->

# CLAUDE.md

行为准则，用来减少常见的 LLM 编码错误。与项目自身的规范合并使用。

**取舍**：这些准则偏向审慎而非速度。琐碎任务自行判断。

## 1. Think Before Coding

**不要假设。不要藏起困惑。把取舍摆出来。**

动手之前：
- 明确说出你的假设。不确定就问。
- 有多种解释时全部列出来——不要默默选一个。
- 有更简单的做法就说出来。该反驳时反驳。
- 有不清楚的地方就停下。指出困惑在哪。问。

## 2. Simplicity First

**用最少的代码解决问题。不做预留。**

- 不做没被要求的功能。
- 不为一次性代码造抽象。
- 不加没被要求的「灵活性」「可配置性」。
- 不为不可能发生的场景写错误处理。
- 写了 200 行而它可以是 50 行，就重写。

自问：「资深工程师会不会觉得这写复杂了？」会，就简化。

## 3. Surgical Changes

**只碰必须碰的。只清理自己造成的混乱。**

改既有代码时：
- 不「顺手改进」相邻代码、注释、格式。
- 不重构没坏的东西。
- 匹配既有风格，哪怕你会用别的写法。
- 发现无关的死代码就说出来——不要删。

自己的改动造成孤儿时：
- 删掉**因你的改动**而不再被使用的 import / 变量 / 函数。
- 不删既有的死代码，除非被要求。

检验标准：每一行改动都能直接追溯到用户的诉求。

## 4. Goal-Driven Execution

**定义成功标准。循环到可验证为止。**

把任务转成可验证的目标：
- 「加校验」→「为非法输入写测试，然后让它们通过」
- 「修 bug」→「写一个能复现它的测试，然后让它通过」
- 「重构 X」→「保证重构前后测试都过」

多步任务先说一句计划：
```
1. [步骤] → 验证：[检查点]
2. [步骤] → 验证：[检查点]
```

---

## Project Conventions

本仓交付的是 XGENT.ai Portal 的一个 App：`<APP_KEY>`（<APP_NAME>，`type: [micro|service]`）。代码在本仓，**清单事实源也在本仓**（`app.manifest.json`）；门户仓里没有它的副本。

### 本地开发

[填：起本仓的命令、端口、依赖的基础设施容器、测试与 verify 的入口。写清「一把起齐」的那条命令，以及最容易踩的启动坑。]

### `key` 是四位一体的，永不改

```
listingKey == TDT 的 aud == 安装态 appKey == /svc/<APP_KEY> == scope 命名空间
```

改 key **等于换一个 App**：已安装租户、已授出的 scope、已签发的令牌、反代路由全部对不上。带连字符的 key，scope 命名空间用下划线（`<app_key>.read`），两种写法门户都认。

### 清单事实源是本仓的 `app.manifest.json`

> 字段全集、校验规则与注册路径：`portal-external-app` skill（`micro` 型的 navItems / dashboardWidgets / ACL 另见 `portal-micro-app`）。

改了清单内容就 **bump `version`**——产物与清单版本漂了没有任何机制会发现。

- **字段是安装期快照**：改清单对**已安装租户零影响**，必须显式同步（提案批准 / `register-app --prod` 都会做），而这件事**本地永远测不出来**（本地是全新安装）。
- **只增不减**：`scopes` / `dependencies` / `exchangeTargets` 同步到已安装租户时只做**并集**。清单里删掉一个 scope，已安装租户会永远留着它——收回已授出的权限走显式下架/迁移。
- **写了 `aclManifest` 必须逐租户 reconcile**（平台侧代做）：那是全平台唯一把 `defaultForMember` 种进 member 角色的地方。漏了的症状是新 PID 出现在角色矩阵里却从不进 member 基线（「配了不生效」），而且**只在普通成员身上现形**，管理员看不出来。
- **`hostPort` / `embedUrl` / `descriptor.env` 的值不归你**（见下）。
- 新增 manifest 字段前先确认门户认得它：未知字段一律落治理档等人审。
- **展示字段只收纯字符串**（`tagline` / `desc` / `icon` / `color` / `cat` / `navItems[].label` / `dashboardWidgets[].title`）：给多语对象会被拼成字面量 `"[object Object]"` 存进 text 列，然后显示在**每个租户**的应用卡片上。多语只有 `name` 收对象。
- 声明 `exchangeTargets`（要读别的 App 的数据）之前先过一遍 `portal-app-exchange` skill：`EXCHANGE_NOT_ALLOWED` / `EXCHANGE_CONSENT_REQUIRED` / 跨应用下拉莫名为空，三个症状都出在布线顺序而不是代码。

### 发布 = 提案，先分清自动档与治理档

> 打包、令牌、CI 写法与发布排错：`xgent-app-release` skill。别自己拼 `curl`。

`npx @xgent/release-cli publish`（或 `POST /api/market/release/<APP_KEY>` + `xrel_` 令牌）每次落成一条**发布提案**：

- **自动通过档**（当场生效）：`dist` 产物 · `version` · `deployDescriptor.image` · 展示字段（name / tagline / desc / icon / color / cat / navItems / dashboardWidgets）· `helpEntry` 当它是 App 内路由时。
- **治理档**（进平台「发布审核」，批准前库里一字不动）：**其余一切**，包括看着像文案实则是权限面的两个——`scopeLabels`（同意屏正文）与 `embedUrl`（iframe 指向），以及 scopes / aclManifest / dependencies / exchangeTargets / serviceBaseUrl / seat* / `requiredEnv` 键集合 / 部署形态。
- **同一 App 同时只允许一条待审提案**：待审期间任何新提交都被拒（`PROPOSAL_PENDING`，连纯 dist/version 也一样）。等审批，或先撤回。
- **换镜像不是同步的**：产物是同步的（publish 打印成功即在线上），镜像只排了一条重部署任务、过一会儿才换、**且可能失败**。CI 里换镜像**必须带 `--wait`**，否则会在容器换上去之前就退出码 0，把「发布成功」当成「新版本在跑」。
- manifest **永远不许携带密钥值**（SA secret / `descriptor.env` 有值，提交即拒）。

### 后端运行时契约（六条，逐条都有症状）

1. **验令牌**：`POST {PORTAL_INTROSPECT_URL}`，HTTP Basic = 服务账号 `clientId:secret`，body `{ token }`。
2. **⚠️ 解信封**：自省响应是统一信封，声明在 `data` 里 —— **`claims = body.data ?? body`**。直接读顶层的 `active` 会把**每一个有效 TDT** 都判成无效。这是外部接入最常踩的一条。
3. **四道闸**（自己实现，网关只是哑路由；顺序即错误面）：
   - 身份：`active === true` 且 `(claims.listingKey ?? claims.aud) === "<APP_KEY>"` 且 `tenant_id` 非空 → 否则 **401 `INVALID_TOKEN`**
   - scope：所需 scope ∈ `claims.scopes` → 否则 **403 `INSUFFICIENT_SCOPE`**
   - 角色：结构性管理操作看 `claims.role === "admin"` → 否则 **403 `FORBIDDEN`**
   - ACL：细粒度操作看 `claims.bypass || claims.permissions` 命中目标 PID
   - 跨租户的服务端判据只认自省返回的 `claims.isPlatformAdmin === true`，**不接受前端自报**。
4. **租户隔离**：每一条查询按 `claims.tenant_id` 收口，不信请求体里的租户字段。这是硬约束，出仓不降级。
5. **自省失败必须 fail-closed**：传输失败 / 非 2xx → 503，**绝不放行**。`active:false` 是一次成功的自省 → 401。
6. **`/health` 返 2xx 即视为健康**（健康体长什么样都行）；**限流自己做**，门户不替你做。

### API：HTTP status code 只反映传输/路由层，不反映业务状态

- 404 = endpoint 不存在（路由层）；「数据不存在」用 200 + 业务字段（如 `data: null`）。
- 业务成功统一 200，含新建/更新/删除/覆写，不用 201/204。
- 业务失败（校验不过 / 状态非法 / 资源冲突）也走 200 + 响应体错误结构。4xx/5xx 只留给参数不正确、未登录、未授权、真传输/服务层故障。
- 越租户与不存在**统一返回同一种「不存在」**，不泄露存在性。

### 端口与环境变量：键名归你，值和宿主口归平台

| 字段 | 谁定 | 说明 |
| --- | --- | --- |
| `deployDescriptor.port` | 你 | 容器内监听口，**就听 8080**。别要求门户注入 `PORT` 才肯听；一盒**根本不读**这个字段（反代按 `<APP_KEY>-server:8080` 找你），听别的口在一盒里直接 502 |
| `hostPort` | 平台 | 宿主机发布口。**只在首次注册当建议值**，此后清单里写什么都不生效（会回一条说明）。撞口是自动退让不是拒绝 |
| `extraPorts[].host` | 你声明需求、平台定值 | **撞了直接拒**（对外契约口，静默挪走等于把 worker 全断掉）。有不经门户请求路径的连接面必须显式 `"bindAddress": "0.0.0.0"`——绑成 loopback 与漏填后果完全相同：接管当天那个面直接掉，而门户侧任何检查都测不到 |
| `descriptor.env` 的**值** / `envFile` | 平台 | manifest 里出现任何**带值**的 `env` → **解析器直接拒收**（防生产密钥进 App 仓） |
| `requiredEnv` | 你 | 只交**键名**。它是一道闸：批准前门户拿它比对 `descriptor.env ∪ envFile` 的键集合，**缺任何一个就拒绝生效** |

- **改键名写成一条 `{ key, renamedFrom }`**，别写成一删一加——后者在门户眼里是两件不相干的事，值搬不过去。
- **新增/改名不写进 `requiredEnv`**，提案会自动通过而运维**看不到新键名**，症状是发布方以为发失败了、实际服务已换版并崩着。
- `PORT` 与 `<APP_KEY>_SERVER_PORT` 由控制器强制注入且覆盖 envFile，别指望在 envFile 里改这两个。
- **服务账号密钥永不静默轮换**：`--prod` 注册 / 提案批准时，门户没从 `<PREFIX>_SA_CLIENT_SECRET` 拿到值就**随机签发并一次性回显**（抄进平台那份 envFile 即可）；已存在却给了个不一致的值 ⇒ 打印原因并**非零退出**，不会悄悄跳过——悄悄跳过的后果是自省 401 而现场看不出原因。

### 数据库迁移：打进镜像，平台在换容器前替你跑

声明 `deployDescriptor.migrateArgs`（追加在镜像 ENTRYPOINT 后面），控制器在**换容器之前**用同镜像同 env 跑一次性容器；失败则任务转红、**旧容器原封不动继续服务**。

1. 迁移文件**打进镜像**（放在仓里的脚本 = 门户拿不到，只能回到手工运维）。
2. **advisory lock 包住**——仍可能与旧副本或另一次并发部署撞上。
3. **失败必须非零退出**，别自愈重试。吞掉错误的代价是门户以为成功、把新容器换上去，然后在运行期以各种方式碎掉。
4. **expand-contract 前向兼容**：迁移跑在换容器之前，那一刻旧版本仍在服务（K8s 滚动期间新旧还并存）。先加列/加表，删旧列留到下一个版本。

### 镜像交付（五条硬约定）

> 仓库地址、robot 账号与推送前的预检：`xgent-image-push` skill（地址不写在本文件里）。

1. **`deployDescriptor.image` 写相对名**：`<APP_KEY>:<tag>`，仓库前缀由门户在部署时拼上。写成 `<APP_KEY>-server:…` 拼出来的 repo **根本不存在**，而这件事只有在切换窗口里第一次 `docker pull` 才会发现。
2. **tag 不可变**：新版本 = 新 tag。同 tag 覆盖推送在门户侧看不出变化（引用没变就不排任务）。
3. **单 arch 必须是 amd64**；要多架构自己 `docker buildx --platform linux/amd64,linux/arm64 --push`。
4. **不带 `-amd64` 后缀**。
5. 常驻型（有匿名公开面 / 被别的服务随时调）必须 **`alwaysOn: true`**，否则会被缩容或挡在冷启动闸后面。

### 依赖 `@xgent/*` 走平台私有 registry

`@xgent/shared` · `@xgent/portal-sdk` · `@xgent/portal-ui` 从平台的私有 registry 装（地址与取 token 的方式在本仓的 `.npmrc` 与 CI 配置里，不写进本文件）。**授权 token 是短时效的**——每次构建现取，`bun install` 报 401/404 先想到它过期了，别去改版本号。

- **三个包各自独立编号，不是一套齐版**；registry 上**缺号是常态**（仓里 bump 过但没推）。以 `npm view @xgent/<pkg> versions` 为准，**写范围不要精确 pin**。
- **`^0.x` 只放行 patch**：`^0.1.0` 不会解析到 `0.2.x`。要用新 API 就显式把范围提上去，症状是 `sdk.<新方法> is not a function`。

### 前端：版头归门户（`micro` 型才有；`service` 型删掉本节及以下三节）

> SDK 全量速查（握手 / getToken / callService / routeSync / dashboard widget / iframe 已知坑）：`portal-micro-app` skill。

- **不在 App 内自绘应用图标 / 应用名 / tagline**：版头最左那一级恒为它且可点回应用首页。再画一份就是同屏说两次，而且租户改过 App 名之后两处会说得不一样（`apps.name` 租户可改，你的 i18n 词条改不了）。可以留：角色 / 工作区 / 当前对象这类**运行期事实**，以及脱离门户直接打开时的 standalone 壳。
- **页面层级用 `sdk.setBreadcrumbs(crumbs)` 上报**：挂在**视图驱动的 effect** 上（不要散在各个 `routeSync()` 调用点——宿主发起的路由变化只走 `onRoute`，挂错的症状是「侧栏切页后面包屑空着」）；首页推 `[]`；每次全量覆盖；label 用**当前语言**解析好（宿主不翻译）。
- **路由双向**：内部路由变化 `sdk.routeSync(path)`，同时**必须**订阅 `sdk.onRoute` 处理宿主推回的路由。只写单向会出现「同一 App 两个菜单点了不切换」。
- **调自己的后端一律 `sdk.callService("<APP_KEY>", path, opts)`**（宿主代转发、零跨域、401/403 自动重铸重试一次）。iframe 里直接跨域 `fetch` 自己的后端是错误姿势。
- **不要自己存 TDT**、不写 localStorage：`sdk.getToken()` 自动缓存并在到期前续签。
- **日常 `getToken` 不要传裁剪过的 scopes**：mint 会把本次签发的 scope 记为用户的**同意范围**，用子集 mint 会**收窄**已有同意，之后更宽的 mint 触发 `CONSENT_REQUIRED`。
- **`sdk.acl.can(pid)` 只是隐藏按钮的 UX 门**，真正拦截靠后端。前端判过 ≠ 安全。
- **低频入口（帮助 / 接入指引）用 `helpEntry` 挂版头**，不占侧栏。指向 App 内路由是自动档；指向外站是治理档（门户版头等于替那个域名背书）。
- **`window.alert` / `confirm` / `prompt` 在跨源沙箱 iframe 里被静默忽略**——一律用 App 内的 DOM 模态框。

### 下拉框：`<select>` 必须 `appearance-none` + 自绘 chevron

把文本输入的类直接套在 `<select>` 上，留下的是**浏览器原生箭头**——位置/大小/颜色全归浏览器，`pr-*` 推不动它（padding 只影响文字），也不跟主题 token 走。这错误反复出现，因为类型检查 / lint / 单测**全绿**，只有人眼看得出来。

```
appearance-none                    ← 唯一硬要求：关掉原生箭头
pr-8                               ← 给自绘箭头留位
<ChevronDown size={14} className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-fg-3" />
```

收在自己的 `Select` 原语里，视图里不写裸 `<select>`。宽度写在哪取决于 `className` 转发给谁：转发给 `<select>`（内部已有 `w-full`）⇒ 定宽要在外面套一层 `div`；转发给外层 wrapper ⇒ 直接写在组件上。

### 字典表统一带 `sort` + `name_i18n`（本 App 有租户级可维护的枚举/分类表时）

- 列名固定 `sort`，`integer NOT NULL DEFAULT 0`，**不要**叫 `seq`。列表统一 `ORDER BY sort, name`。前端字段也叫 `sort`。
- 必带 `name_i18n jsonb`（可空）：`name` 是**缺省 locale(zh-CN) 槽位 + 唯一键 + 回退终点**，`name_i18n` 只存非缺省 locale 覆盖，键白名单只有 `en` / `zh-TW`。校验归一用 `@xgent/shared` 的 `normalizeNameI18n()`，前端解析用 `@xgent/portal-ui` 的 `resolveDictName(row, locale)`——别再写一份。
- ⚠️ body schema 里 `nameI18n` 用 `t.Optional(t.Unknown())`：`t.Record(t.String(), t.String())` 会静默丢弃 `__proto__` 键，把「拒绝」变成「悄悄清空」。
- **例外**：树 / 兄弟次序表不算字典表，保留 `seq`——语义是序号不是字典排序。

### App 图标：必须意义契合，且必须在门户注册表里存在

不要用「机器人 / AI」那类通用图标，挑一个和它干什么直接对应的 lucide 图标；**不要和已有 App 撞图标**。图标名必须是**门户宿主支持的 lucide 名**——宿主查不到时**静默回退**，写错的名字会一路走到生产（拿不准就问平台管理员要当前可用清单与占用情况）。改 `icon` 只对**新安装**生效（租户可自行改），存量对齐要找平台。

### 本地联调：一盒（one-box）

不 clone 门户、不改门户代码：拉门户镜像起一个真门户，注册你的 manifest，再把 `dist` 绝对路径挂进去（`APP_FRONTEND_DIST`）。**步骤、凭证与端口一律走 `portal-dev-setup` skill**，不要照着别处的命令手敲。

- **一盒是 dev 底座，不是门户**：`DEV_MOCK_OAUTH=true` + 已知明文 SA 密钥。永远不要拿它当生产。dev 态 manifest 可以带 `serviceAccount.secret` 明文；`--prod` 下**给了就拒收**。
- **先对齐一盒镜像的版本**：一盒镜像滞后于门户主干时会出现「文档说有、实际没有」的现象（真实案例：注册后自动授予租户的那段代码比某次发布的镜像新一天，症状是市场里没有卡片而没有任何报错）。**遇到「照文档做但行为不符」，先确认镜像版本，再怀疑自己**。
- **一盒镜像与你自己的 App 镜像不在同一个 registry 项目下**：`init` 默认按你的项目名拼一盒镜像会 404，显式传镜像引用（值见 `portal-dev-setup` skill）。
- **你自己的库不会被自动建**：一盒的 postgres 初始化脚本只建平台与内置 App 的库，先手工 `CREATE DATABASE "xgent-<APP_KEY>"`。
- **生产镜像是 amd64、开发机多半是 arm64**：起一盒时要给你的 backend 容器显式 `platform: linux/amd64`（devkit 默认跟随本机架构，自己叠一层 override）。
- **注册必须先于反代启动**，否则 `/svc/<APP_KEY>` 404；首次 404 的另一个常见原因是放行 map 写失败（权限），进反代容器补一行再 reload。
- **注册完先看演示租户的市场里有没有卡片**：没有卡片就是授予行没写成，先解决它，别接着往下排查前端。
- **首次打开有「授权并打开」的同意屏**（正常行为，自动化走查要把它算进步骤）；`install` 排出的部署任务在一盒里会一直停在 `deploying`——一盒没有部署控制器，不挡使用。

### 完成标准

- 类型检查 / 单测只证明代码对，不证明功能对。**UI 改动必须从宿主进入、在真浏览器里走通主路径与关键边界**才算完成；环境起不来就**显式说明「未在浏览器中验证」**。
- 后端改动至少自测到：无 token → 401；缺 scope → 403；换一个租户看隔离；`curl /svc/<APP_KEY>/health` 200。
- 发布前：`release-cli whoami` 验令牌（别等构建完才发现过期），换镜像带 `--wait`。
- 症状 → 原因先查 skill 里的排查表，别猜：`/svc` 404·502 / 自省 401 / `INSUFFICIENT_SCOPE` 看 `portal-external-app`；发布 401·404·`PROPOSAL_PENDING` / 发了没换版看 `xgent-app-release`；一盒起不来看 `portal-dev-setup`；`EXCHANGE_*` 看 `portal-app-exchange`。

## Design Context

本仓有专门的设计文档，任何前端 UI 工作之前都要读（`impeccable` skill 会自动加载）：

- **[PRODUCT.md](./PRODUCT.md)** —— 战略上下文：register、用户、产品目的、品牌人格、反面参照、设计原则、可访问性。
- **[DESIGN.md](./DESIGN.md)** —— 视觉系统：色彩、字体、投影、组件，以及从门户底座继承来的那部分 token。

<!-- 填写指引：两份文档与 CLAUDE.md 同在仓根时链接就是上面这样；放到别处记得改路径。 -->

- **设计阶段**用 `impeccable` skill（视觉层级、信息架构、间距对齐、配色、动效、可访问性、空/错状态），不要「凭感觉写 Tailwind」。
- **验证阶段**在真实浏览器里从宿主进入走通主路径再报告完成。
