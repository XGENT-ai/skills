# 嵌入式微应用（iframe + SDK）开发

微应用是被 Portal 以沙箱 iframe 嵌入的普通 Web 页面：**不自己实现登录、不持有 App Secret**，身份由宿主经 postMessage 握手注入。本文件是工作流与红线；具体契约按下表读取。

这里的宿主身份是门户成员。若同一个产品另有面向顾客/学生的独立网站或原生客户端，
终端身份的注册、登录、会话与隐私治理读 [终端身份](end-users.md)；
不能把成员 SDK 握手、角色或成员 OAuth 直接当作终端用户认证。


`/apps/<key>/` 是线上前端挂载 URL；下表链接相对于本参考文件。

## 按任务读参考

| 任务 | 读 |
| --- | --- |
| 握手 / SDK API / 令牌与 consent / callService | [sdk-reference.md](sdk-reference.md) |
| 声明导航、scope、ACL、Widget、依赖 | [manifest-and-acl.md](manifest-and-acl.md) |
| 调 Open API / Widget 推送 / 通知 / 内容 / 计划任务 / 审计 / 错误码 | [open-api-and-extensions.md](open-api-and-extensions.md) |

## 新建一个 micro App 前端的最小闭环

1. 在 App 自己的仓库新建 Vite + React 前端，配置固定空闲端口并开启 `strictPort`，避免自动换端口后加载错应用。
2. 安装下述公开分发的 SDK/UI 包，按本地 [SDK 参考](sdk-reference.md) 接入握手与路由。
3. 先在 App 内做完整导航（见下文设计红线），再在自己的 `app.manifest.json` 声明可选的顶级快捷方式 `navItems` 以及 `scopes` / `aclManifest` / `dashboardWidgets`（顶层数组，见 [Widget 契约](open-api-and-extensions.md#3-dashboard-widget应用动态)）；租户管理员不能代填开发者声明。
4. Vite `base` 设为 `/apps/<key>/`，构建 `dist/`，用 `xgent-app-release` skill 的 `publish --dist` / `publish --manifest` 提交；涉及治理字段等待审批，按本次 proposal 的交付状态核对结果。
5. 本地联调用 `portal-dev-setup` 的一盒环境。调自己的后端用 `sdk.callService`，前端托管在同源 `/apps/<key>/`。门户 API 的 CORS 名单只覆盖内置 App，外部 App 的源加不进去，也不必申请。
6. 在真实浏览器从宿主进入并走通主路径、路由往返与权限边界；不以构建通过代替 UI 验证。

`@xgent/portal-sdk` / `@xgent/portal-ui` / `@xgent/shared` 在你的 repo 里**照常作为 npm 依赖安装**，
来源是平台的私有包仓（公共 npm 上没有，直接 `bun add` 会 E404）。配法见 `xgent-app-release`
skill 的「第 0 步」。仓里还没有 `.xgent-registry.env` ⇒ 停下，请开发者先去门户「开发者应用」生成
配置文件。**不许 vendor**：不把 SDK 的产物或源码拷进仓，也不用 `file:` / `link:` / tar 依赖顶替。

## SDK 硬规则

- 私有页面从 `const sdk = createPortalClient(); const init = await sdk.ready();` 开始。SDK 不在 iframe 内时自动切到同源 standalone，同一套自举，App 不用自己判断是否在 iframe，也不要做「请在门户内打开」提示页；standalone 需在清单声明 `openModes`，从 `/open/<key>` 进入，未声明时 `ready()` 报 typed error。顶层必须处理 `ready()` typed error 和 `onContextChanged()`，失效时卸载私有视图与应用缓存，并提供显式 `openGate()`；不要自动跳登录。匿名页可不调用 `ready()`。生产从 `/apps/<key>/` 推导 key，开发的 appKey/host/apiBase 来自构建配置，不能读 URL 配认证主机。详见 [SDK 参考](sdk-reference.md)。
- `sdk.getToken()` 自动缓存、到期前 30s 续签——**不要**自己存 TDT、不要写 localStorage。
- 用户长期凭证由本 App 后端委托门户签发。前端只将新建/迁移响应保存在当前一次性展示组件；不要用 Query/Mutation 缓存保存含 secret/token 的响应。关闭后清除，复制失败提供选中文本和重试。门户集中管理入口是 `/me?tab=keys`；App 保留项目/能力/路由配置、旧密钥停用与远端撤销重试。
- 全局平台管理员不在 TDT JWT / `InitPayload` 里；`sdk.acl.bypass` 和 `sdk.userinfo().role` 只表示当前租户。前端即使拿到 session-only `/auth/me.isPlatformAdmin` 也只能做展示，不能转发给后端授权；跨租户后端只认 TDT 自省返回的 `isPlatformAdmin`。
- 调自己的独立后端一律 `sdk.callService("<listingKey>", path, opts)`（宿主代转发、零跨域、401/403 自动重铸重试一次）。iframe 直接跨域 `fetch` 独立后端是错误姿势。
- 权限：`sdk.acl.can(pid)` / `sdk.acl.scope(pid)` 只做**隐藏按钮/入口**的 UX 门；真正拦截靠后端。前端判过 ≠ 安全。
- 路由：内部路由变化调 `sdk.routeSync(path)` 同步到 `?r=`；同时**必须**订阅 `sdk.onRoute` 处理宿主推回的路由（浏览器后退、同 App 多导航项切换）。只写单向 routeSync 会出现"同一 App 两个菜单点了不切换"的 bug。二级详情页把完整路径（如 `/items/<id>`）routeSync 出去，刷新/分享才能还原。
- 版头面包屑：调 `sdk.setBreadcrumbs(crumbs)` 上报页面层级（宿主最左恒为「应用图标 + 应用名」= 应用首页，你报的是其后那几级；首页推 `[]`，每次是整条 trail 的**全量覆盖**）。四条规矩：① 挂在**视图驱动的 effect** 上，**不要**散在各个 `routeSync()` 调用点——宿主发起的路由变化只走 `onRoute`，挂错的症状是"侧栏切页后面包屑空着"；② `onRoute("")`（空路由）必须落到首页视图，版头最左那一级靠它工作；③ 标签要异步查询才知道时**宁可少一级**，不推 id 占位、不推"加载中…"，数据回来再整条覆盖（`FR-12` 这种展示 key 可以推，内部 UUID 不可以）；④ `label` 用**自己当前的语言**解析好（宿主不翻译），切语言在 `onLocale` 里重推。上限 6 级 / label 120 字符 / route 512 字符且必须 `/` 开头（不得以 `//` 开头），超限被静默截断；末级不带 `route`。
- **App 内不要自绘图标与名称**：版头最左那一级恒为「应用图标 + 应用名」且可点回应用首页，App 再画一份就是同屏说两次，而且租户改过 App 名之后两处会说得不一样（`apps.name` 租户可改、i18n 词条改不了）。tagline / 副标题同理（应用中心已展示过）。可以留：角色 / 工作区 / 当前对象这类**运行期事实**，以及 `main.tsx` 里脱离门户打开时的 standalone 壳。浏览器验收时检查版头与 App 内容没有重复标题。
- **帮助页入口 `helpEntry`**：清单里声明一个可空字符串，版头工具条最右侧（全屏与刷新的**左边**）就多一枚帮助按钮；不声明就不出按钮。两种形态：`"/help"` = App 内路由（宿主置 `?r=`，不离开门户，**自动档**）、`"https://docs.example.com"` = 外站文档（新标签 + `noopener`，**走治理审核**，因为门户版头等于替这个域名背书）。`/…` 与 crumb `route` 同一条规则；外站必须 https（`http://` 连回环也拒）、无 fragment。它是**安装期快照**，改了要 bump 自己的 `version`；**租户不可覆盖**（想指向自家知识库去知识库 App 建条目）。门户不托管、不渲染、不翻译帮助正文。
- **低频入口放版头、不占侧栏**：帮助中心 / 接入指引这类低频页别长期占着侧栏一个位置 —— 用 `helpEntry` 挂到版头，并把这一级显式推进面包屑。
- 主题/语言：订阅 `sdk.onTheme` / `sdk.onLocale`。App 内部用内存路由 + routeSync，不在 iframe 内 pushState。
- 未保存更改调 `sdk.setDirty(true)`。在门户内切到别的标签或页面不会卸载 iframe，草稿还在，所以不提示；只有关闭该标签、刷新或关闭浏览器页，以及离开门户框架（切换租户、退出登录、前往 `/open`、`/login` 等门户外页面或其他源，含前进后退到这些地方）时才确认，取消则保留原 iframe。
- **标签保活**：每个 App 一个标签，最多 6 个保持加载。超出时卸载最久未用、非当前且未置脏的 iframe，标签与 `?r=` 路由保留，切回时重新加载并以该路由握手；刷新浏览器也只恢复标签与路由。所以 App 必须能从 `init.route` 还原视图，草稿要么及时保存、要么 `setDirty(true)`（脏标签不会被卸载）。后台标签里 `routeSync` 只更新自己的标签，`sdk.navigate` 与 `requestFullscreen` 被忽略，`requestStepUp` 直接返回 `{ state: "rejected" }`。
- **iframe 高度二选一**：满高型（编辑器、聊天、带底部输入栏）**不调** `sdk.resize`，外壳 `height: 100dvh`（或 `100%`）+ 内部滚动——软键盘缩小可见区时宿主让 iframe 一起变矮、在 iframe 内触发 `resize`，App 在 `resize` 里把插入点 / 焦点输入框滚回视图，且 iOS 上输入控件计算字号 ≥16px（更小时 Safari 聚焦即放大，宿主不让位）；内容高度型调 `sdk.resize(scrollHeight)`，由宿主滚动区滚动，**调过一次就不再复位为满高、也不再为键盘让位**。没有键盘高度事件，也不需要。详见 [SDK 参考「iframe 高度」](sdk-reference.md#iframe-高度满高型与内容高度型软键盘)。

## Consent 硬规则

- 宿主挂 iframe 前有 consent 门；mint 会把**本次签发的 scope 记为用户的同意范围**。用子集 scope mint 会**收窄**已有同意，之后更宽的 mint 触发 `CONSENT_REQUIRED`。日常 getToken 不要传裁剪过的 scopes。
- 声明了 `exchangeTargets` 的 App，用户首次进入时 consent 门会**共授**跨应用交换同意——跨应用读数据为空时先想到这个（详见 [跨应用交换](app-exchange.md)）。

## iframe 已知坑（都真实踩过）

- `window.prompt` / `window.confirm` / `window.alert` 在跨源沙箱 iframe 里**被静默忽略**——一律用应用内 DOM 模态框。
- Tailwind 的 `/alpha` 颜色修饰符（如 `bg-primary/50`）在主题的普通 `var()` 颜色上**静默失效**——用 `opacity-NN` 或预算好的色值。
- Radix 菜单/Popover 触发器放在模态 Dialog 里"点了没反应"：触发器组件必须 `forwardRef`，且弹层 z-index 要高于 dialog overlay。
- 跨源 iframe 里剪贴板 API 受限；Chrome extension 也看不进跨源 iframe（验证时从宿主页面操作）。
- TanStack Query 在 iframe 里设 `networkMode: 'always'`（queries 与 mutations）并 `retry: 0`。请求经宿主转发，`navigator.onLine` 与实际连通无关；默认 `online` 模式下失败的查询会挂起为 `paused`（无 error、无 data），页面渲染成空列表。重试还要求页面可见，浏览器标签在后台时失败的查询同样挂起，此后「重试」按钮无效。`retry: 0` 让首次失败直接进入错误态。
- `.xg-md` markdown 渲染若用 `@xgent/file-preview` 的 `./markdown` 子路径，宿主需映射 hsl 通道 CSS 变量。

## 设计红线（平台一致性）

- **App 内自带完整导航，不依赖 `navItems`**：`navItems` 是租户管理员在「应用配置 → 顶部导航」里逐项决定是否加进门户左侧导航的快捷方式。安装后默认全部关闭，是否开启、开启哪几项由各租户决定。用户从应用中心、启动器或收藏进入时，宿主只打开 App 首页（`init.route` 为空）；App 若没有自己的侧栏或顶栏，其余页面就没有入口。因此：
  1. 所有面向用户的页面都要能从 App 首页，经 App 自己的导航或页内链接到达。
  2. `navItems` 只挑少数值得置顶的高频入口，`path` 指向 App 内已有的路由。每项还要被 `aclManifest` 某个页面的 `navItemId` 指向才会进侧栏；没有 `aclManifest` 的 App，租户启用了也不会显示。
  3. 从快捷方式或宿主推回路由（`onRoute`）进入时，App 导航高亮对应项。

  例外：以 `init.adminPage` 嵌入管理页时收起自身导航（见[配置分层](config-placement.md)）；只有一个页面的 App 不需要菜单。
- **配置界面别挂进你自己的导航**：全平台可热改的配置做平台级管理页、每租户不同的做租户级管理页，都在清单 `adminPages` 里声明，由门户嵌进应用市场编辑页 / 租户应用配置；应用内的业务设置才做应用内设置页（`aclManifest` 授权）。判断表见 [配置分层](config-placement.md)。**不自建**审计页（写 `POST /api/v1/audit`）。
- **不做**「同步通讯录」：成员选择一律用 `@xgent/portal-ui` 的 `UserPicker` 按需 curated。
- 列表页服务端分页，复用 `Page<T>` 契约与现有 helpers。
- 复用 `@xgent/portal-ui` 已有组件再造新轮子。输入相关的三个子路径无其他依赖，可单独引入：
  - `@xgent/portal-ui/keyboard`：`isEnterKey` / `onEnterKey`，避免中文输入法确认候选词的那次回车触发提交。
  - `@xgent/portal-ui/input`：`ComposingInput` / `ComposingTextarea`，组字期间不触发 `onChange`，防抖搜索或逐键规整的受控输入框用它。
  - `@xgent/portal-ui/dict-name`：`resolveDictName(row, locale, init.localeOrder)` 解析字典行的显示名，不自己写语言回退。
- **页面铺满客户区，上限 1600**：iframe 就是客户区（14" 笔记本上约 1272px，App 自带侧栏再减 ~200px）。页面框架写 `mx-auto w-full max-w-[1600px]` + 页边距，**不要**把整页收成 `max-w-3xl` / `max-w-[880px]` 居中——笔记本上会左右各空 150–200px。窄只加在内容块上且**左对齐**：表单 / 设置列 ≤ 720px、Markdown 文档列 ≤ 880px、段落正文 ≤ 72ch；多个设置分区 `lg:` 并排两栏、卡片网格 `xl:` 加一列。允许居中窄列的只有：对话流（≤ 800px）、多步向导、授权 / 无权限卡片、空状态、纸面预览、弹窗 / 抽屉。

## 完成标准

类型检查/单测只证明代码对，不证明功能对。微应用改动必须在真浏览器（Chrome extension）里从宿主进入、走通主路径与关键边界后才算完成。导航验收时，在没有开启任何 navItems 的租户里从应用中心打开 App，确认经 App 内导航能到达每个页面；环境起不来就显式说明"未在浏览器中验证"。
