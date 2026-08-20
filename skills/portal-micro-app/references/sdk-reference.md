# SDK 与令牌参考（微应用视角）

> 提炼自门户仓库 `docs/SSO与App开发指引.md` §5/§7.4/§8.1/§10（2026-07）（门户仓文件，App 自己的 repo 里没有；本文件已自包含，不必去找）。冲突时以门户仓库原文为准。

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
- TDT 是 HS256 JWT，`aud` = 你的 appKey，`tenant_id`/`user_id`/`scopes` 都在 claims 里；短期有效（默认 3600s）、可即时吊销。

## 2. InitPayload

```ts
interface InitPayload {
  appKey: string;
  apiBase: string;      // Open API 绝对地址，拼 /api/v1/* 用
  host: string;         // Portal web 源，拼可分享深链用
  theme: "light" | "dark";
  locale: string;       // "zh-CN" | "en" | "zh-TW"
  user: { id: string; name: string } | null;
  route: string | null; // 深链：地址栏 ?r= 携带的内部路由，用于还原
  fullscreen: boolean;
  acl?: AclInit;        // { bypass, permissions:[{pid,scope}], groups }
}
```

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

// Dashboard Widget（需 Manifest dashboard.widgets[] 声明 + widget.write）
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

// 下载文件（跨源预签名 URL 唯一安全姿势，见 §5.1）
import { openDownload } from "@xgent/portal-sdk";
openDownload(presignedUrl);

// 与宿主 UI 协作
sdk.resize(document.body.scrollHeight + 8);
sdk.navigate("/app/another-app");            // 让宿主跳 Portal 内路由
sdk.routeSync("/detail");                    // 内部路由 → 地址栏 ?r=（可分享/刷新还原）
sdk.setDirty(true);                          // 未保存更改 → 宿主拦截离开
sdk.requestFullscreen(true);

// 订阅宿主事件
sdk.onTheme(t => ...); sdk.onLocale(l => ...);
sdk.onRoute(p => ...);                       // ?r= 变化（后退 / 同 App 多导航切换）
sdk.onFullscreen(on => ...);
```

## 4. Consent（授权与同意）

- `micro` 应用挂载 iframe **之前**宿主先查 consent；未授权渲染"授权屏"列出申请的 scope，用户点授权后才挂载、才签发 TDT。
- `POST /api/tokens/mint` 内部同一道门：未授权抛 `CONSENT_REQUIRED`（带 appName + 缺失 scopes）。
- ⚠️ **同意即窄化**：mint 会把本次签发的 scope **记为**用户的同意范围。用子集 scope mint 会收窄已有同意，之后更宽的 mint 触发 `CONSENT_REQUIRED`。需要更宽 scope 前先重新走 consent；日常 getToken 不要传裁剪过的 scopes。
- 用户在「个人中心 › 已授权应用」可撤销（撤销即 bump (user,app) 版本，旧 TDT 立即失效）。
- 声明了 `exchangeTargets` 的 App，consent 门会**共授**跨应用交换同意。

## 5. callService vs 直连 CORS

- 浏览器里 iframe 直接 `fetch` Open API / 独立后端受 CORS 限制；后端 CORS 白名单是环境变量维护的有限源列表。
- 首选 `sdk.callService`：宿主为该应用铸/复用 host-proxy TDT、代为 `fetch`、回传响应；你的后端只需信任 Portal web 源一个跨域来源。401/403 时宿主自动重铸令牌重试一次。
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
  sdk.resize(document.body.scrollHeight + 8);
}
main();
```
