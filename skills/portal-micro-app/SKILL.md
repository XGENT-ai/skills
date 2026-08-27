---
name: portal-micro-app
description: '开发/修改 XGENT Portal 的嵌入式微应用前端（micro 型 App，iframe + @xgent/portal-sdk，即 apps/*-app）。凡任务涉及新建一个 App 前端、给某 App 加页面/导航项/Dashboard Widget、SDK 握手/getToken/callService、consent 授权屏、或 iframe 内的 UI 异常（弹窗无效/菜单点不动/主题/路由同步/全屏/剪贴板）时，务必先用本 skill——即使用户没提"微应用"三个字。Use when building or changing any micro-app frontend embedded in the portal (apps/*-app), the portal SDK handshake, navItems, dashboard widgets, or iframe-specific UI issues.'
---

# portal-micro-app · 嵌入式微应用（iframe + SDK）开发

微应用是被 Portal 以沙箱 iframe 嵌入的普通 Web 页面：**不自己实现登录、不持有 App Secret**，身份由宿主经 postMessage 握手注入。本文件是工作流与红线；具体契约按需读 `references/`（为本 skill 提炼的自包含参考，可整目录拷到任何 repo 使用；权威源是门户仓库 `docs/SSO与App开发指引.md`，冲突以门户仓库为准）。


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
| 握手 / SDK API / 令牌与 consent / callService | [references/sdk-reference.md](references/sdk-reference.md) |
| 声明导航、scope、ACL、Widget、依赖 | [references/manifest-and-acl.md](references/manifest-and-acl.md) |
| 调 Open API / Widget 推送 / 通知 / 内容 / 计划任务 / 审计 / 错误码 | [references/open-api-and-extensions.md](references/open-api-and-extensions.md) |

门户 monorepo 内工作时的活教材：`apps/sample-app/src/main.ts`（SDK 每个能力的端到端演示）、`apps/spms-app`（重业务）、`packages/portal-sdk`（SDK 源码）。外部项目没有这些路径，以 references 为准。

## 新建一个 micro App 前端的最小闭环（门户 monorepo 内）

1. 新建 `apps/<key>-app`（Vite + React，复制最接近的现有 app 骨架），包名 `@xgent/<key>-app`，端口取当前空闲的 53xx（看根 `package.json` 的 `dev:all` 已占端口）。
2. 挂进根 `package.json` 的 `dev:all`（concurrently 一把起齐）。**绝不**单独叠跑 `dev:<key>`——Vite 端口被占会自增抢相邻端口，导致别的 App iframe 加载错页面。
3. 清单声明：`navItems` / `scopes` / `aclManifest` / `dashboard.widgets` 只能由应用自身声明（`apps/api/src/modules/acl/manifests.ts` + `apps/api/src/db/provisioning.ts` 的 `LISTING_DEFS`），租户管理员不能手填。改完重新 `db:seed` 或走清单同步更新。
4. dev 直连 Open API 需 CORS：把 app 源加进 api 的 CORS 白名单 env；生产优先 `sdk.callService`/同源托管，前端完全不碰跨域。
5. UI 开发遵守 CLAUDE.md 强制两步：设计用 `impeccable` skill，验证用 Chrome extension 真浏览器走通主路径。

**你的 App 不在门户 monorepo 里（代码在自己的 repo）** ⇒ 上面 1–3 步一条都不适用：没有
`apps/<key>-app` 目录、没有 `dev:all`、清单也不在 `LISTING_DEFS` 里。你的闭环是另一条：
vite `base` 设成 `/apps/<key>/` → build 出 `dist/` → 用 `xgent-app-release` skill 的
`publish --dist` 交上去（清单同理走 `publish --manifest`，你 repo 里那份 `app.manifest.json`
就是唯一事实源）；想在真门户里先看一眼，把 `dist` 绝对路径挂进一盒（`portal-external-app`
skill 的 `references/registration-and-onebox.md` §4）。**本文件其余部分（SDK 硬规则、consent、
iframe 坑、设计红线、三份 references）与你在哪个 repo 无关，照用。**

## SDK 硬规则

- 一切从 `const sdk = createPortalClient(); const init = await sdk.ready();` 开始。不在 Portal iframe 内（`window.parent === window`）时给提示页，不要白屏。
- `sdk.getToken()` 自动缓存、到期前 30s 续签——**不要**自己存 TDT、不要写 localStorage。
- 全局平台管理员不在 TDT JWT / `InitPayload` 里；`sdk.acl.bypass` 和 `sdk.userinfo().role` 只表示当前租户。前端即使拿到 session-only `/auth/me.isPlatformAdmin` 也只能做展示，不能转发给后端授权；跨租户后端只认 TDT 自省返回的 `isPlatformAdmin`。
- 调自己的独立后端一律 `sdk.callService("<listingKey>", path, opts)`（宿主代转发、零跨域、401/403 自动重铸重试一次）。iframe 直接跨域 `fetch` 独立后端是错误姿势。
- 权限：`sdk.acl.can(pid)` / `sdk.acl.scope(pid)` 只做**隐藏按钮/入口**的 UX 门；真正拦截靠后端。前端判过 ≠ 安全。
- 路由：内部路由变化调 `sdk.routeSync(path)` 同步到 `?r=`；同时**必须**订阅 `sdk.onRoute` 处理宿主推回的路由（浏览器后退、同 App 多导航项切换）。只写单向 routeSync 会出现"同一 App 两个菜单点了不切换"的 bug。二级详情页把完整路径（如 `/items/<id>`）routeSync 出去，刷新/分享才能还原。
- 版头面包屑：调 `sdk.setBreadcrumbs(crumbs)` 上报页面层级（宿主最左恒为「应用图标 + 应用名」= 应用首页，你报的是其后那几级；首页推 `[]`，每次是整条 trail 的**全量覆盖**）。四条规矩：① 挂在**视图驱动的 effect** 上，**不要**散在各个 `routeSync()` 调用点——宿主发起的路由变化只走 `onRoute`，挂错的症状是"侧栏切页后面包屑空着"；② `onRoute("")`（空路由）必须落到首页视图，版头最左那一级靠它工作；③ 标签要异步查询才知道时**宁可少一级**，不推 id 占位、不推"加载中…"，数据回来再整条覆盖（`FR-12` 这种展示 key 可以推，内部 UUID 不可以）；④ `label` 用**自己当前的语言**解析好（宿主不翻译），切语言在 `onLocale` 里重推。上限 6 级 / label 120 字符 / route 512 字符且必须 `/` 开头（不得以 `//` 开头），超限被静默截断；末级不带 `route`。
- **App 内不要自绘图标与名称**：版头最左那一级恒为「应用图标 + 应用名」且可点回应用首页，App 再画一份就是同屏说两次，而且租户改过 App 名之后两处会说得不一样（`apps.name` 租户可改、i18n 词条改不了）。tagline / 副标题同理（应用中心已展示过）。可以留：角色 / 工作区 / 当前对象这类**运行期事实**，以及 `main.tsx` 里脱离门户打开时的 standalone 壳。回归门 `apps/api/scripts/verify-app-chrome.ts`。
- **帮助页入口 `helpEntry`**：清单里声明一个可空字符串，版头工具条最右侧（全屏与刷新的**左边**）就多一枚帮助按钮；不声明就不出按钮。两种形态：`"/help"` = App 内路由（宿主置 `?r=`，不离开门户，**自动档**）、`"https://docs.example.com"` = 外站文档（新标签 + `noopener`，**走治理审核**，因为门户版头等于替这个域名背书）。`/…` 与 crumb `route` 同一条规则；外站必须 https（`http://` 连回环也拒）、无 fragment。它是**安装期快照**，改了要 bump 自己的 `version`；**租户不可覆盖**（想指向自家知识库去知识库 App 建条目）。门户不托管、不渲染、不翻译帮助正文。
- **低频入口放版头、不占侧栏**：帮助中心 / 接入指引这类低频页别长期占着侧栏一个位置 —— 用 `helpEntry` 挂到版头，并把这一级显式推进面包屑。
- 主题/语言：订阅 `sdk.onTheme` / `sdk.onLocale`；高度用 `sdk.resize`；未保存更改 `sdk.setDirty(true)` 让宿主拦离开。

## Consent 硬规则

- 宿主挂 iframe 前有 consent 门；mint 会把**本次签发的 scope 记为用户的同意范围**。用子集 scope mint 会**收窄**已有同意，之后更宽的 mint 触发 `CONSENT_REQUIRED`。日常 getToken 不要传裁剪过的 scopes。
- 声明了 `exchangeTargets` 的 App，用户首次进入时 consent 门会**共授**跨应用交换同意——跨应用读数据为空时先想到这个（详见 portal-app-exchange skill）。

## iframe 已知坑（都真实踩过）

- `window.prompt` / `window.confirm` / `window.alert` 在跨源沙箱 iframe 里**被静默忽略**——一律用应用内 DOM 模态框。
- Tailwind 的 `/alpha` 颜色修饰符（如 `bg-primary/50`）在主题的普通 `var()` 颜色上**静默失效**——用 `opacity-NN` 或预算好的色值。
- Radix 菜单/Popover 触发器放在模态 Dialog 里"点了没反应"：触发器组件必须 `forwardRef`，且弹层 z-index 要高于 dialog overlay。
- 跨源 iframe 里剪贴板 API 受限；Chrome extension 也看不进跨源 iframe（验证时从宿主页面操作）。
- `.xg-md` markdown 渲染若用 `@xgent/file-preview` 的 `./markdown` 子路径，宿主需映射 hsl 通道 CSS 变量。

## 设计红线（平台一致性）

- **不自建**「设置」页/导航（租户配置走平台应用配置页）、**不自建**审计页（写 `POST /api/v1/audit`）。
- **不做**「同步通讯录」：成员选择一律用 `@xgent/portal-ui` 的 `UserPicker` 按需 curated。
- 列表页服务端分页，复用 `Page<T>` 契约与现有 helpers。
- 复用 `packages/portal-ui` 已有组件再造新轮子。

## 完成标准

类型检查/单测只证明代码对，不证明功能对。微应用改动必须在真浏览器（Chrome extension）里从宿主进入、走通主路径与关键边界后才算完成；环境起不来就显式说明"未在浏览器中验证"。
