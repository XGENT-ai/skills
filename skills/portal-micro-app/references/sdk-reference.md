# SDK 与令牌参考（微应用视角）

> 本文是可在 App 仓库独立使用的契约参考；能力以目标平台已发布版本和管理员配置为准。

## 1. 握手与令牌注入

你的页面被 Portal 以沙箱 iframe 嵌入（`sandbox="allow-scripts allow-same-origin allow-forms allow-popups ..."`，`referrerPolicy=no-referrer`）。流程：

```
Portal 宿主 (MicroAppHost)                你的 iframe (portal-sdk)
   │ ① 先过 consent 门（§4）                  │
   │───────────── 挂载 iframe ──────────────►│ ② createPortalClient()
   │◄──────── req: ready ───────────────────│
   │ ③ event: init {appKey,apiBase,host,     │
   │    theme,locale,user,route,fullscreen,  │
   │    acl} ────────────────────────────────►│ ready() 返回 InitPayload
   │◄──────── req: getToken {scopes?} ───────│ ④ getToken()
   │ ⑤ 宿主用用户会话 POST /api/tokens/mint    │
   │    → {access_token,...} ────────────────►│ 拿到 TDT（SDK 自动缓存）
   │                                          │ ⑥ 用 TDT 调 /api/v1/* 或 callService
```

要点：
- 宿主只接受来源在 `embedUrl` + `allowedOrigins` 白名单内的 postMessage。
- 应用**全程拿不到 App Secret**；TDT 由 SDK 自动缓存、到期前 30s 才重新申请——不要自己持久化令牌。
- TDT 是 HS256 JWT，`aud` = 你的 appKey，`tenant_id`/`user_id`/`scopes` 都在 claims 里；短期有效（默认 3600s）、可即时吊销。**`isPlatformAdmin` 不在 JWT 里**，它是 Portal 资源服务器自省时实时派生的字段。

## 2. InitPayload

```ts
interface InitPayload {
  appKey: string;
  apiBase: string;      // Open API 绝对地址，拼 /api/v1/* 用
  host: string;         // Portal web 源，拼可分享深链用
  theme: "light" | "dark";
  locale: string;       // "zh-CN" | "en" | "zh-TW"
  localeOrder?: string[]; // 租户语言次序（回退链用）；旧宿主缺省
  user: { id: string; name: string } | null;
  tenant?: { id: string; name: string; slug: string | null } | null; // 当前租户；旧宿主缺省，判空
  route: string | null; // 深链：地址栏 ?r= 携带的内部路由，用于还原
  fullscreen: boolean;
  acl?: AclInit;        // { bypass, permissions:[{pid,scope}], groups }
  serviceBinary?: boolean; // 宿主能代理 callServiceBinary；旧宿主缺省，standalone 由 SDK 置 true
  executableDownloadBase?: string; // 平台配置的可执行文件下载基址（无产品前缀）
  adminPage?: "platform" | "tenant"; // 被门户嵌成管理页（清单 adminPages）时有值：只渲染那一页，收起自己的侧栏/顶栏
}
```

### 2.1 平台管理员的前后端边界

- `InitPayload` 不含 `isPlatformAdmin`；`acl.bypass` 代表**当前租户 admin**，`sdk.userinfo().role` 也只是 TDT 当前租户的角色，都不是全局平台管理员判据。
- 当 micro App 以 `/apps/<key>/` **同源部署**时，前端可调 session-only `GET /auth/me` 取 `data.isPlatformAdmin` 做菜单/文案展示；该端点是 Portal shell 会话面，不是 TDT Open API/SDK 稳定契约，独立后端无 Cookie 也不应调它。非同源/dev 环境不要假设它可用。
- 前端值**绝不参与授权**：不发 `X-Is-Platform-Admin`，不把 body 里的布尔传给后端当判据，不转发 Cookie 让后端代调 `/auth/me`。如资源服务器有平台级跨租户路由，它必须以 SA Basic 自省 TDT，并校验自省返回的 `claims.isPlatformAdmin === true`（服务态恒为 `false`）。

## 3. SDK API 速查

```ts
import { createPortalClient } from "@xgent/portal-sdk";
const sdk = createPortalClient();            // 必须运行在 Portal 的 iframe 内
const init = await sdk.ready();

// 身份与令牌
const token = await sdk.getToken();          // 原始 TDT（一般不用，便捷方法已内置）
const me    = await sdk.userinfo();          // GET /api/v1/userinfo

// 权限 UX 门（真正的安全门在后端）
sdk.acl.can("myapp:action:item.delete");     // boolean
sdk.acl.scope("myapp:page:items");           // "own" | "team" | "all"

// 写入 Portal
await sdk.notify({ title, body, type, link: "/app/your-app" });

// Dashboard Widget（需 Manifest 顶层 dashboardWidgets[] 声明 + scopes 含 widget.write；数据按当前用户存）
await sdk.dashboard.putWidgetData("activity", { title, items: [...] }, { staleAfterSec: 3600 });
await sdk.dashboard.clearWidgetData("activity");

// 内容服务（headless CMS，需 content.read/write + 声明内容类型）
await sdk.content.types(); await sdk.content.list("note", { limit: 20 });
await sdk.content.create("note", {...}); await sdk.content.update("note", id, {...});
await sdk.content.remove("note", id);

// 计划任务（需 scheduler.read/write）
const task = await sdk.scheduler.create({ name, cron: "0 9 * * *", params });
await sdk.scheduler.update(task.id, { status: "paused" }); await sdk.scheduler.cancel(task.id);

// 调你自己的独立后端（宿主代理，零跨域）
const data = await sdk.callService("myapp", "/api/things", { method: "GET" });
// sdk.files.* 即 callService("files", …) 的薄封装

// 取自家后端的完整二进制响应（≤ 32 MiB，见 §5.2；需 portal-sdk ≥ 0.5.4）
const { data, contentType, size } = await sdk.callServiceBinary("myapp", `/exports/${id}/content`, { method: "POST" });

// 下载文件（跨源预签名 URL 唯一安全姿势，见 §5.1）
import { openDownload } from "@xgent/portal-sdk";
openDownload(presignedUrl);

// 与宿主 UI 协作
sdk.resize(document.body.scrollHeight + 8);   // 仅内容高度型；满高型不调，见下文「iframe 高度」
sdk.navigate("/app/another-app");            // 让宿主跳 Portal 内路由
sdk.routeSync("/detail");                    // 内部路由 → 地址栏 ?r=（可分享/刷新还原）
sdk.setBreadcrumbs([                         // 页面层级 → 宿主版头面包屑
  { label: "需求池", route: "/requirements" },
  { label: "FR-12 登录页改造" },             // 末级不带 route（它就是当前页）
]);                                          // 首页推 []；整条 trail 全量覆盖
sdk.setDirty(true);                          // 拦 Shell 离开、刷新/关闭、跨 pathname 的前进后退
                                             // 同 App ?r= 不拦；取消保留原 iframe
                                             // 需要含门户提交 1590f2e3 的运行件；仅升级 SDK 不启用旧宿主
sdk.requestFullscreen(true);

// 订阅宿主事件
sdk.onTheme(t => ...); sdk.onLocale(l => ...);
sdk.onRoute(p => ...);                       // ?r= 变化（后退 / 同 App 多导航切换）
                                             // p === "" ⇒ 宿主点了「回应用首页」
sdk.onFullscreen(on => ...);
```

### `setBreadcrumbs` 的四条规矩

⚠️ 它需要 `@xgent/portal-sdk` **≥ 0.2.1**。`0.x` 上 `^` 只放行 patch —— 依赖范围写
`^0.1.0` 的仓**不会**自动吃到它（症状：`sdk.setBreadcrumbs is not a function`）。
把 `package.json` 里的范围显式提上去再装。

宿主版头最左恒为「应用图标 + 应用名」（= 应用首页），你上报的是**其后**那几级。

1. **不含应用本身那一级**；首页推 `[]`。每次调用是**整条 trail 的全量覆盖**，不是追加。
2. **挂在视图驱动的 effect 上**（与决定路由的是同一份 state），**别**散在各个
   `routeSync()` / `navigate()` 调用点 —— 宿主发起的路由变化只走 `onRoute`（且不许在
   `onRoute` 里 echo `routeSync`），挂错的症状是「侧栏切页后面包屑空着」。
3. **`onRoute("")` 必须落到首页视图** —— 用户点最左那一级时宿主就是把 `?r=` 清空再推给你。
4. **标签要异步查询才知道时宁可少一级**，不推 id 占位、不推「加载中…」；数据回来再整条
   覆盖一次（两段式，谁后到谁生效）。`FR-12` 这种**展示 key** 是已知事实可以推，
   `a3f9…` 这种内部 UUID 不可以。

`label` 由你用**自己当前的语言**解析好（宿主不翻译），切语言要在 `onLocale` 里重推。
上限 6 级 / label 120 字符 / `route` 512 字符且必须 `/` 开头（不得以 `//` 开头），
超限被宿主静默截断。fire-and-forget：旧宿主收不到也不会报错，版头退化成「图标 + 名字」。

### App 内部不要自绘图标与名称

**App 内部不得渲染自己的图标与名称。** 版头最左那一级恒为「应用图标 + 应用名」，且它是
可点的应用首页；App 再画一份 = 同屏两处说同一句话，而且租户改过 App 名之后两处会说得
不一样（`apps.icon` / `apps.name` 租户可改，App 内的 i18n 词条改不了）。**tagline / 副标题**
同理（应用中心已展示过）。**可以留**：角色、工作区、当前对象这类运行期事实；以及
`main.tsx` 里 App 脱离门户打开时的 standalone 提示页（那一屏没有版头）。

### `helpEntry`：在清单里声明帮助页入口

清单里加一个可空字符串 `helpEntry`，版头工具条最右侧（全屏与刷新的**左边**）就多一枚
帮助按钮；不声明就不出按钮（不渲染，不是禁用）。

```jsonc
"helpEntry": "/help"                      // App 内路由：宿主置 ?r=，用户不离开门户
"helpEntry": "https://docs.example.com"   // 外站文档：新标签打开（noopener）
```

- `/…` 与 crumb 的 `route` **同一条规则**（`/` 开头、不得 `//` 开头、无控制字符、≤512）；
  `https://…` 必须可解析、必须 https（`http://` 连回环也拒）、无 fragment、无通配符。
- **内页路由自动档，外链走治理审核**（门户版头替一个域名背书）。
- 它是**安装期快照**：改了内容要 bump 自己的 `version`。**租户不可覆盖** —— 想指向自家
  知识库的诉求去知识库 App 实现。
- 门户**不托管、不渲染、不翻译**帮助正文，这个字段只是一个入口。

### 低频入口放版头，不占侧栏

帮助中心、接入指引这类低频页面不该长期占着 App 侧栏一个位置：声明 `helpEntry` 让它挂在
版头那枚帮助按钮上，并把这一级显式推进面包屑，进去之后位置依然清楚。范例是研发项目管理
的帮助中心（指引 + FAQ）。

### iframe 高度：满高型与内容高度型（软键盘）

宿主给 iframe 的高度只有两种模式，按 App 形态二选一：

| | 满高型 | 内容高度型 |
| --- | --- | --- |
| 适合 | 编辑器、聊天、带底部输入栏的 App | 文档式页面、短表单 |
| 写法 | **不调** `sdk.resize()`；外壳 `height: 100dvh`（或 `100%`），内部滚动 | `sdk.resize(document.body.scrollHeight + 8)`，iframe 变成固定像素高，由宿主滚动区滚动 |
| 软键盘弹出 | iframe 的布局视口随宿主可见区变矮，iframe 内触发 `resize`；收起后恢复 | Android：宿主滚动区随布局视口变矮，浏览器把焦点输入框滚入可视区；iOS：宿主不调整，保持浏览器原生平移 |

- **调过一次 `resize` 就是内容高度型**：宿主不会再把 iframe 恢复为满高，也不再为它让位。满高型 App 不要为了「追可视区」去调 `resize`。
- **满高型 App 自己负责两件事**：
  1. 在 `window` 的 `resize` 里让当前插入点或焦点输入框保持可见，例如对焦点输入框 `scrollIntoView({ block: "nearest" })`，或按选区矩形调整内部滚动容器的 `scrollTop`。宿主只改 iframe 尺寸，不动 App 的内部滚动。
  2. iOS 上输入控件（`input` / `textarea` / `contenteditable`）的**计算字号 ≥16px**。字号更小时 Safari 聚焦即放大页面，宿主检测到缩放就不让位，键盘照旧盖住底部。
- App 拿不到也不需要键盘高度：没有对应的 SDK 事件，iframe 里的 `100dvh` / `innerHeight` 已经是可见区。
- **不承诺**：键盘高度数值；与键盘动画逐帧同步（按帧合并，可能滞后一两帧）；捏合缩放与聚焦放大状态下的让位；横屏加键盘时的最小可用高度（可能很矮，甚至为 0）；iPad 浮动 / 分离键盘（不缩小可视区，不触发）；外接 / 硬件键盘（iPad 快捷栏可能触发，不承诺）；键盘打开时拖动期间的短暂闪跳；全屏状态下内容高度型 App 的滚动；门户自身页面在 iOS 上的让位。
- **standalone**（App 自己就是顶层页面）：门户不介入，App 自己设置 viewport meta（如 `interactive-widget=resizes-content`）并监听自己的 `visualViewport`。

## 4. Consent（授权与同意）

- `micro` 应用挂载 iframe **之前**宿主先查 consent；未授权渲染"授权屏"列出申请的 scope，用户点授权后才挂载、才签发 TDT。
- `POST /api/tokens/mint` 内部同一道门：未授权抛 `CONSENT_REQUIRED`（带 appName + 缺失 scopes）。
- ⚠️ **同意即窄化**：mint 会把本次签发的 scope **记为**用户的同意范围。用子集 scope mint 会收窄已有同意，之后更宽的 mint 触发 `CONSENT_REQUIRED`。需要更宽 scope 前先重新走 consent；日常 getToken 不要传裁剪过的 scopes。
- 用户在「个人中心 › 已授权应用」可撤销（撤销即 bump (user,app) 版本，旧 TDT 立即失效）。
- 声明了 `exchangeTargets` 的 App，consent 门会**共授**跨应用交换同意。

## 5. callService vs 直连 CORS

- 浏览器里 iframe 直接 `fetch` Open API / 独立后端受 CORS 限制；后端 CORS 白名单是环境变量维护的有限源列表。
- 首选 `sdk.callService`：宿主为该应用铸/复用 host-proxy TDT、代为 `fetch`、回传响应；你的后端只需信任 Portal web 源一个跨域来源。401/403 时宿主自动重铸令牌重试一次。
- ⚠️ **目标不是你自己时走的是令牌交换，不是 mint**：`callService("<别的 App>", …)`（`sdk.files.*` 就是它的薄封装）由宿主换一张 `aud=<目标>` 的票，所以**清单里必须有 `exchangeTargets` + 目标命名空间 scope**，用户也要同意过。漏声明的症状是 iframe 里这类调用全挂——不是 CORS，查 portal-app-exchange skill。
- `callService` 不替后端做授权——后端仍要自省校验 aud/scope（这不是前端的事，但别以为走了代理就"安全了"）。

### 5.1 下载文件：只用 `openDownload`，别自己写 `<a href>.click()`（BUG-48）

预签名下载 URL 指向**对象存储源**，跟 App 前端不同源。`<a href=url>.click()` 于是不是"下载"，而是让 App 的 iframe **自己导航**过去——门户 CSP 的 `frame-src` 只放行 App 前端源，Chrome 会把**整个 App** 换成拦截页：「该内容被屏蔽了。请联系网站所有者以解决此问题。」控制台是 `Framing '…' violates … "frame-src …"`。

```ts
import { openDownload } from "@xgent/portal-sdk";
const { url } = await sdk.callService("myapp", `/attachments/${id}/download`);
openDownload(url);            // ✅ 顶层弹窗不受 frame-src 管，一闪即落盘
```

三个反直觉点：

- 加 `a.download = name` **救不回来**——该属性对跨源 URL 会被浏览器忽略，照样导航、照样被拦。
- 宿主 sandbox 的 `allow-downloads` 是**另一半**（已有）：缺它，弹窗会开但文件被静默丢弃。它治不了上面那个导航拦截，两件事别混。
- 服务端要保证 URL 带 `Content-Disposition: attachment`（files-server 的 `/:id/download` 默认如此），否则新标签页变成预览而不是落盘。
- 例外：**blob:/同源** URL（前端自己生成的 CSV 等）用 `<a download>.click()` 没问题，属性生效、不产生导航。

### 5.2 受控字节：`callServiceBinary`

字节不能暴露成签名 URL 时（一次性领取证明、App 自己判领取资格），由你的后端直接返回字节，前端用 `callServiceBinary` 取。它和 `callService` 同一个地址解析与取票，区别在于把**完整**响应读完后以 `ArrayBuffer` 交给你：

```ts
import { type ServiceBinaryError } from "@xgent/portal-sdk";
const init = await sdk.ready();
if (init.serviceBinary !== true) return showHostTooOld();   // iframe 内的旧宿主：HOST_UNSUPPORTED
try {
  const { data, size } = await sdk.callServiceBinary("myapp", `/exports/${id}/content`, {
    method: "POST", headers: { "x-claim-proof": proof }, timeoutMs: 600_000, signal,
  });
  if (size !== expected.size || await sdk.files.digest(data) !== expected.sha256) throw new Error("摘要不符");
  const url = URL.createObjectURL(new Blob([data], { type: "application/zip" }));
  Object.assign(document.createElement("a"), { href: url, download: expected.fileName }).click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
} catch (error) {
  const { code, status, details } = error as ServiceBinaryError;   // 两种传输形状相同
}
```

- **上限 33,554,432 字节**，整段读完才交付，不交出截断的字节；没有流式、断点续传，也没有进度事件。实测领取 32 MiB 时页面峰值内存增量约为上限的 2 倍。
- **它不判业务**：你的 `{ok:false}` 信封原样 reject（code / message / details，带 HTTP status）；`{ok:true,data:null}` 是 `EMPTY_RESPONSE`；其他 JSON 是 `UNEXPECTED_RESPONSE` —— **JSON 文件不能经它领取**，二进制接口要返回非 JSON 的 content-type。
- **其余失败码**：超限 `TOO_LARGE`；跳转、断流、网络失败 `TRANSPORT_FAILED`；长度不符 `TRUNCATED`；非 2xx 且不是信封 `HTTP_ERROR`；路径含 `..` 等为 `INVALID_PATH`；SDK 本地码 `NOT_READY` / `HOST_UNSUPPORTED` / `TIMEOUT` / `ABORTED`。取票失败（如 `EXCHANGE_NOT_ALLOWED`）原样到达，后端收不到请求。
- ⚠️ **只在 401 时换票重试一次**（`callService` 是 401/403 都重试）：403 可能出现在你已消耗一次性证明之后，重发等于重放。所以后端的 401 只能出现在消耗证明之前。另外，复用的 keep-alive 连接若在你写出任何响应字节前被关闭，Chrome 会自己把同一请求重发一次（任何方法），宿主拦不住 —— **一次性证明的消费方必须拒绝第二次出示**。
- `timeoutMs` 是从发起到读完的**总期限**，缺省 120 s，慢网下不够下完 32 MiB，按大小放宽。`signal` 可取消；iframe 重载、换代或离开 App 都会让调用得到 `ABORTED`，可用 `setDirty(true)` 让宿主在离开前确认。
- 响应头只给 `contentType`，不回传 `Content-Disposition`：文件名从你自己的 JSON 里取。存盘用同源 `blob:` 加 `<a download>`（sandbox 已带 `allow-downloads`）。

## 6. 最小可用例子

```ts
import { createPortalClient } from "@xgent/portal-sdk";
async function main() {
  if (window.parent === window) {
    document.body.textContent = "请在 XGENT Portal 内打开本应用（需要宿主握手）。";
    return;
  }
  const sdk = createPortalClient();
  const init = await sdk.ready();
  const user = await sdk.userinfo().catch(() => null);
  document.body.innerHTML = `当前用户：${user?.name ?? "—"}`;
  sdk.onTheme(t => document.documentElement.dataset.theme = t);
  // 这是内容高度型页面：按内容报高度。满高型（编辑器、聊天）不要调，见 §3「iframe 高度」。
  sdk.resize(document.body.scrollHeight + 8);
}
main();
```
