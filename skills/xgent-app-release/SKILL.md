---
name: xgent-app-release
description: 把一个 App 的前端产物发布到 XGENT.ai Portal —— 在 App 自己的 repo 里用 xrel_ 发布令牌一条命令完成「构建 → 打包 → 上传 dist → bump 版本 →（可选）换镜像」，不登录门户控制台、不找门户运维代传。凡任务涉及发版/发布前端/上传产物/release-cli/xrel_ 令牌/POST /api/market/release/、配 vite base、在 CI 里写发布步骤、或出现「发布 401 / 404」「发布 200 但 ok:false」「/apps/<key>/ 白屏或资源 404」「发上去了但线上没变」这类症状时，务必先用本 skill 再动手——即使用户只说「发个版」。Use whenever publishing or debugging an XGENT portal app frontend release from the app's own repo: release tokens, dist packaging, version bumps, CI wiring, or a blank/404 /apps/<key>/ page after a publish.
---

# xgent-app-release · 前端产物自助发布

**这个 skill 用在 App 自己的 repo 里**（门户代码不在你手上，也不需要在）。目标是：把构建好的前端
一次推上门户的某个 listing，并让「线上跑的是哪一版」在控制台上可辨认。

自助面**封顶三件**：`dist`（前端产物）· `version`（清单版本号）· `deployDescriptor.image`（部署镜像引用）。
其余一切（scopes / ACL 清单 / 依赖 / 跨应用授权 / 服务地址 / 席位 / descriptor 的 env / 上下架）都会
改变权限面或影响其他租户的治理，**令牌一律拒**（返回 `200 + VALIDATION_FAILED`，库里一字不改）——
那些要找平台管理员走门户发版，不要试图绕。

## 先备齐三样，缺一样就发不出去

| 你需要 | 从哪来 | 放在哪 |
| --- | --- | --- |
| `listingKey` | 平台给的 App 标识，小写字母/数字/连字符。它同时是 `/svc/<key>`、`/apps/<key>/`、scope 命名空间、令牌的 aud —— **四位一体，永不改** | 仓里的本地配置文件 `LISTING_KEY=…`（`.gitignore` 掉），`--key` 可覆盖 |
| 发布令牌 `xrel_…` | 平台管理员在 控制台 › 应用清单 › 发布令牌 签发，**明文只显示一次** | `XGENT_RELEASE_TOKEN` 或 `--token`。**是密钥，不进配置文件** |
| 门户地址 | 问平台要 | `XGENT_PORTAL_URL` 或 `--portal`。随环境变，别写死进仓里 |

三者来路不同是有意的：身份不变所以落盘，环境会变、密钥不能落盘所以走参数。
详见 [references/publish-api.md](references/publish-api.md) §0。

## 发布五步，每步都有验收

```bash
export XGENT_RELEASE_TOKEN=xrel_…          # CI secret，绝不写进仓库
export XGENT_PORTAL_URL=https://portal.example.com
VER=1.4.2                                   # listingKey 在配置文件里，命令里不用重复
```

1. **先验令牌，再构建。** `npx @xgent/release-cli whoami`
   → 验收：打印 key + 令牌前缀 + 过期时间。放在构建之前是因为构建可能十分钟，
   而令牌过期/被吊销的现象只有调用时才现形。
2. **构建，`base` 必须是 `/apps/<key>/`。** 产物在生产被挂到那个子路径下，
   `base` 少了 → 资源请求打到站点根 → 页面 200 但白屏。这是本流程翻车率第一名。
   → 验收：`grep -o 'src="[^"]*"' dist/index.html`，路径都以 `/apps/<key>/` 开头。
3. **预检。** `node <skill>/scripts/preflight.mjs --dist dist --version $VER`
   → 验收：脚本零 ✗ 退出。它把「构建看着成功、线上却坏」的几种成因一次性挡下（base 前缀、
   根 `index.html`、包大小、dev 地址残留、版本号形状、令牌有效性）。
4. **发布。**
   ```bash
   npx @xgent/release-cli publish --version $VER --dist dist/
   # 有后端、这次还换了镜像时，加 --image <key>:$VER --wait
   ```
   → 验收：打印 `✓ <key> 已发布 <version>` + 产物 digest。**失败时线上那份原封不动**
   （门户先落 staging、验根 `index.html`、再 swap；被拒时 `version` 与 `digest` 都不动）。
5. **线上看一眼。** `npx @xgent/release-cli status` 确认版本与 digest 就是本次这一份；
   然后浏览器打开门户 → 应用中心 → 你的 App，走通主路径。

`@xgent/release-cli` 不在公共 npm 上；你的环境取不到它时**不要卡在这里**——端点就一条
`POST /api/market/release/:key`，`curl` 兜底见 [references/publish-api.md](references/publish-api.md) §2。

## 四条硬约定（都是「不知道就会中」的那种）

- **`version` 每次都要 bump，且是必填项**——哪怕这次只换产物没改功能。产物 digest 变了而 version
  没变，控制台上就再也分不清「线上跑的是哪一版」，而这正是发布链路存在的意义。
  出仓 App 的 version 归你（门户的清单同步器对它们两层都跳过）；若你发的版本号在门户下次部署后
  被改回旧值，那不是你的问题——去找平台管理员确认你这个 key 有没有被标成「版本归 App 所有」。
- **发布是替换，不是合并。** 上一版的文件不会留着。所以「只补传一个改了的文件」这种操作不存在，
  每次都传完整 dist。
- **tar 根必须直接是 `index.html`。** `release-cli` 传目录时已经用 `tar czf … -C dist .` 打好；
  只有自己 `curl` 时才需要自己打，`tar czf x.tgz dist` 那种套一层 `dist/` 的包会被拒收。
- **上限 64MB**，且门户只按顶层条目数报数。真超了先查有没有把 source map / 未压缩素材打进去。

## 顺带换镜像（有后端的 App）

同一次 `publish` 可以带 `--image <name>:<tag>`：镜像引用一变，门户自动排一条重部署任务，
生产两条链路（pm2 / K8s）都会滚到新版本。三个易错点：**① `<name>` 就是你的 App key，不是
`<key>-server`；② 只写相对名，仓库前缀由门户拼；③ tag 不可变——同 tag 覆盖推送门户看不出变化，
不会触发换版。** 该 App 必须已由平台管理员配了 `deployDescriptor`，否则这一项直接 `VALIDATION_FAILED`。

**换镜像时加 `--wait`。** 产物是同步的（打印成功时已在线上），换容器不是——门户只排了任务，
容器过一会儿才换、而且可能失败。不加 `--wait`，CI 会在这之前就退出码 0，把「发布成功」
和「新版本在跑」画上等号。细节见 [references/publish-api.md](references/publish-api.md)。

## 发之前想在真门户里看一眼

两条本地通路，各有硬约束，别混用：

| | 用在哪 | 约束 |
| --- | --- | --- |
| **devkit 卷挂载**（一盒） | 服务端同学本地跑一个真门户，把你的 `dist` 目录挂进去 | 同源，满足生产同款 CSP；**没有 HMR**，改前端要重新 build |
| **cross-origin vite dev** | 前端内循环改页面 | 门户 dev 的 CSP 才放行 `localhost:53xx`；**生产 CSP 是 `frame-src 'self'`，此路上不了生产** |

⭐ 无论哪条，**都不需要门户给你开 CORS**：前端从不直连自己的后端，请求都经宿主 `postMessage` 代理发出。

## 按需读

| 场景 | 读 |
| --- | --- |
| 产物形状（`base` 双态、tar 根、CSP 与外链、发前在真门户里试） | [references/build-and-package.md](references/build-and-package.md) |
| 令牌语义、端点原始形状、字段白名单、返回体、`curl` 兜底、CI 范式 | [references/publish-api.md](references/publish-api.md) |
| 症状 → 原因速查（发布报错 / 线上白屏 / 发了没换版） | [references/troubleshooting.md](references/troubleshooting.md) |

报告结果时如实说清：发了哪个 key、哪个版本、digest 是多少、有没有在浏览器里实际打开过。
「命令返回 ✓」只证明产物落库了，不证明页面能用——第 5 步没做就说没做。
