# 构建与打包

发布之前的一切。这里的每一条都对应一种「构建全绿、线上坏掉」的具体死法。

依赖怎么装、包从哪来、构建器怎么配 —— 那是你这个 repo 自己的事，本文不管。
本文只管**产物要长成什么形状**，因为那是门户会校验、会拒收、会静默画错的部分。

## 1. `base` 必须是 `/apps/<listingKey>/`

生产上产物被同源挂在门户的 `/apps/<key>/` 子路径下（同源是有意的：宿主 iframe 的生产 CSP 是
`frame-src 'self'`，跨源地址上不了生产）。dev server 却服务在根路径，所以 `base` 是**双态**的：

```ts
// vite
export default defineConfig(({ command }) => ({
  base: command === 'build' ? '/apps/<listingKey>/' : '/',
  // …
}));
```

漏掉 `build` 那一态的症状很有欺骗性：iframe 里页面 200（宿主拿得到 `index.html`），
但里面每个 `/assets/…` 都打到站点根 → 404 → **白屏，控制台一片红**。
构建、类型检查、单测全绿——这条只有产物形状检查或人眼看得出来，所以预检脚本第一件事就查它。

非 vite 的构建器同理：webpack 是 `output.publicPath`，Next 静态导出是 `basePath` + `assetPrefix`。
判据统一：**`index.html` 里所有根绝对路径都要以 `/apps/<key>/` 开头。**

客户端路由也要跟着走子路径（React Router 的 `basename`），否则刷新二级页面会 404。

## 2. 产物形状：tar 根直接是 `index.html`

门户解包后在**根目录**找 `index.html`，找不到就 `VALIDATION_FAILED` 拒收。

```bash
tar czf dist.tgz -C dist .     # ✅ 根下就是 index.html、assets/…
tar czf dist.tgz dist          # ❌ 全套在 dist/ 一层下面，拒收
```

`release-cli` 传目录时已经用第一种形状打好了，只有自己 `curl` 时才需要手打。

其它约束：**≤64MB**（超了先查 source map 与未压缩素材）；空文件会被当成「构建没产出」拒掉，
而不是当成「只发版本号」——这是有意的，否则一条坏流水线会报成功。

## 3. 不需要 CORS，但可能需要 CSP

前端**从不直连自己的后端**：请求经宿主的 `postMessage` 代理由宿主同源发出。
所以别去要 CORS 放行——要了也没用。

反过来，如果前端要直连**自己的域名 / 对象存储 / 第三方**，那是浏览器直发的请求，受 per-App CSP 管。
XHR/fetch 那一类可以由 App manifest 的 `embedCsp.connectSrc` 放行；字体、样式表、图片走的是
`font-src` / `style-src` / `img-src`，不在这个字段的能力范围内。**manifest 不在自助面里**——
要放行得找平台管理员，改完下次反代 reload 生效。

所以最省事的做法是**把资源打进产物、不外链**（字体尤其：CDN 字体是这里最常见的外链，
而它恰好是 `connectSrc` 放行不了的那一类）。预检脚本会把产物里的外链列出来提醒。

## 4. 发之前在真门户里看一眼

| 通路 | 怎么用 | 硬约束 |
| --- | --- | --- |
| **本地一盒（devkit 卷挂载）** | 平台侧起一个真门户，把你的 `dist` **绝对路径**挂进去 | 同源，走生产同款 CSP；**无 HMR**，改前端要重新 build |
| **cross-origin vite dev** | listing 的 `embedUrl` 临时指向 `http://localhost:53xx`，起你仓的 dev server | 仅门户 dev 的 CSP 放行本地端口；**生产 `frame-src 'self'`，此路永远上不了生产** |

第二条最容易留后患：调试时往代码里写死的 `localhost:53xx` 如果跟着产物发上生产，
CSP 会静默拦掉，症状是「本地好好的，线上那个功能点了没反应」。预检脚本会扫这个。
