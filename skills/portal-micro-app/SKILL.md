---
name: portal-micro-app
description: '开发/修改 XGENT Portal 的嵌入式微应用前端（micro 型 App，iframe + @xgent/portal-sdk，即 apps/*-app）。凡任务涉及新建一个 App 前端、给某 App 加页面/导航项/Dashboard Widget、SDK 握手/getToken/callService、consent 授权屏、或 iframe 内的 UI 异常（弹窗无效/菜单点不动/主题/路由同步/全屏/剪贴板）时，务必先用本 skill——即使用户没提"微应用"三个字。Use when building or changing any micro-app frontend embedded in the portal (apps/*-app), the portal SDK handshake, navItems, dashboard widgets, or iframe-specific UI issues.'
---

# portal-micro-app · 嵌入式微应用（iframe + SDK）开发

微应用是被 Portal 以沙箱 iframe 嵌入的普通 Web 页面：**不自己实现登录、不持有 App Secret**，身份由宿主经 postMessage 握手注入。权威契约随本 skill 附带：[`references/docs/SSO与App开发指引.md`](references/docs/SSO与App开发指引.md)（下称"指引"；`references/` 是门户仓库文档的镜像，指引内的相对链接在其中原样可解析）；本 skill 告诉你读哪节、以及文档之外反复踩过的坑。

> 本 skill 自包含，可整目录拷到任何 repo 使用。若你正在门户 monorepo 内工作，指引以 `docs/SSO与App开发指引.md` 原件为准（副本用 `.claude/skills/sync-portal-skill-refs.sh` 同步）；文中提到的 `apps/`、`packages/` 代码路径也仅在门户 monorepo 内存在，外部项目按指引文档实现即可。

## 先读什么（按任务对号入座）

| 任务 | 读 |
| --- | --- |
| 新建 App 前端 / 理解握手 | 指引 §5（握手 + SDK 速查 + 最小例子）；活教材 `apps/sample-app/src/main.ts` |
| 声明导航 / Widget / 内容类型 / scope | 指引 §4（Manifest）、§13（扩展点） |
| 调自己的独立后端 | 指引 §7.4（`sdk.callService` 宿主代理） |
| 授权屏 / CONSENT_REQUIRED | 指引 §10 |
| 前端权限门 | 指引 §12（`sdk.acl` 只是 UX 门，安全门在后端） |

参考同构实现：`apps/spms-app`（重业务）、`apps/todo-app`（纯内容服务）、`packages/portal-sdk`（SDK 源码）。

## 新建一个 micro App 前端的最小闭环

1. 新建 `apps/<key>-app`（Vite + React，复制最接近的现有 app 骨架），包名 `@xgent/<key>-app`，端口取当前空闲的 53xx（看根 `package.json` 的 `dev:all` 已占端口）。
2. 挂进根 `package.json` 的 `dev:all`（concurrently 一把起齐）。**绝不**单独叠跑 `dev:<key>`——Vite 端口被占会自增抢相邻端口，导致别的 App iframe 加载错页面。
3. 清单声明：`navItems` / `scopes` / `aclManifest` / `dashboard.widgets` 只能由应用自身声明（代码注册表 `apps/api/src/modules/acl/manifests.ts` + `apps/api/src/db/provisioning.ts` 的 `LISTING_DEFS`），租户管理员不能手填。改完重新 `db:seed` 或走清单同步更新。
4. dev 直连 Open API 需 CORS：把 app 源加进 api 的 CORS 白名单 env；生产则优先 `sdk.callService`/同源托管，前端完全不碰跨域。
5. UI 开发遵守 CLAUDE.md 强制两步：设计用 `impeccable` skill，验证用 Chrome extension 真浏览器走通主路径。

## SDK 硬规则

- 一切从 `const sdk = createPortalClient(); const init = await sdk.ready();` 开始。不在 Portal iframe 内（`window.parent === window`）时给出提示页，不要白屏。
- `sdk.getToken()` 自动缓存、到期前 30s 续签——**不要**自己存 TDT、不要往 localStorage 写。
- 调自己的独立后端一律 `sdk.callService("<listingKey>", path, opts)`（宿主代转发、零跨域、401/403 自动重铸重试一次）。iframe 直接跨域 `fetch` 独立后端是错误姿势。
- 权限：`sdk.acl.can(pid)` / `sdk.acl.scope(pid)` 只做**隐藏按钮/入口**的 UX 门；真正拦截靠后端。前端判过 ≠ 安全。
- 路由：内部路由变化调 `sdk.routeSync(path)` 同步到地址栏 `?r=`；同时**必须**订阅 `sdk.onRoute` 处理宿主推回的路由（浏览器后退、同 App 多导航项切换）。只写单向 routeSync 会出现"同一 App 两个菜单点了不切换"的 bug——宿主带 echo-guard 推路由事件，应用必须消费。二级详情页把完整路径（如 `/items/<id>`）routeSync 出去，刷新/分享才能还原。
- 主题/语言：订阅 `sdk.onTheme` / `sdk.onLocale`；高度用 `sdk.resize`；未保存更改用 `sdk.setDirty(true)` 让宿主拦离开。

## Consent 硬规则

- 宿主挂 iframe 前有 consent 门；mint 会把**本次签发的 scope 记为用户的同意范围**。用子集 scope 去 mint 会**收窄**已有同意，之后更宽的 mint 触发 `CONSENT_REQUIRED`。所以 getToken 不要传裁剪过的 scopes，除非明确要窄化。
- 声明了 `exchangeTargets` 的 App，用户首次进入时 consent 门会**共授**跨应用交换同意——跨应用读数据为空时先想到这个（见 portal-app-exchange skill）。

## iframe 已知坑（都真实踩过）

- `window.prompt` / `window.confirm` / `window.alert` 在跨源沙箱 iframe 里**被静默忽略**——一律用应用内 DOM 模态框。
- Tailwind 的 `/alpha` 颜色修饰符（如 `bg-primary/50`）在主题的普通 `var()` 颜色上**静默失效**——用 `opacity-NN` 或预先算好的色值。
- Radix 菜单/Popover 触发器放在模态 Dialog 里"点了没反应"：触发器组件必须 `forwardRef`，且弹层 z-index 要高于 dialog overlay。
- 跨源 iframe 里剪贴板 API 受限；Chrome extension 也看不进跨源 iframe（验证时从宿主页面操作）。
- `.xg-md` markdown 渲染若用 `@xgent/file-preview` 的 `./markdown` 子路径，宿主需映射 hsl 通道 CSS 变量。

## 设计红线（平台一致性）

- **不自建**「设置」页/导航（租户配置走平台应用配置页，指引 §13.7）、**不自建**审计页（写 `POST /api/v1/audit`，§13.8）。
- **不做**「同步通讯录」：成员选择一律用 `@xgent/portal-ui` 的 `UserPicker` 按需 curated。
- 列表页服务端分页，复用 `Page<T>` 契约与现有 helpers。
- 复用 `packages/portal-ui` 已有组件再造新轮子。

## 完成标准

类型检查/单测只证明代码对，不证明功能对。微应用改动必须在真浏览器（Chrome extension）里从宿主进入、走通主路径与关键边界后才算完成；环境起不来就显式说明"未在浏览器中验证"。
