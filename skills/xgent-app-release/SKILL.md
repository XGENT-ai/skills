---
name: xgent-app-release
description: '把一个 App 的新版本发布到 XGENT.ai Portal —— 在 App 自己的 repo 里用 xrel_ 发布令牌一条命令提交「前端产物 dist / bump 版本 / 换镜像 / 整份 app.manifest 清单」，落成发布提案：无治理变更自动生效，改权限面的进平台「发布审核」等批准；不登录门户控制台、不找门户运维代传。凡任务涉及发版/发布前端或后端镜像/上传产物/提交或修改 app.manifest.json/首次把 App 接入门户/release-cli/xrel_ 令牌/POST /api/market/release/、配 vite base、在 CI 里写发布步骤、或出现「发布 401 / 404」「发布 200 但 ok:false」「PROPOSAL_PENDING / 一直 pending 等审」「/apps/<key>/ 白屏或资源 404」「发上去了但线上没变 / 容器没换版」这类症状时，务必先用本 skill 再动手——即使用户只说「发个版」。Use whenever publishing or debugging an XGENT portal app release from the app''s own repo — frontend dist, backend image, or manifest/governance changes via release proposals: release tokens, packaging, version bumps, CI wiring, pending approvals, or a blank/404 /apps/<key>/ page after a publish.'
---

# xgent-app-release · App 版本自助发布

**这个 skill 用在 App 自己的 repo 里**（门户代码不在你手上，也不需要在）。目标是：把构建好的前端
一次推上门户的某个 listing，并让「线上跑的是哪一版」在控制台上可辨认。

每次提交在门户侧落成一条**发布提案**，按内容自动定级：

- **自动通过档**：`dist`（前端产物）· `version` · `deployDescriptor.image` · 展示字段
  （name / tagline / desc / icon / color / cat / navItems / dashboardWidgets）——
  提交即生效，与从前一字不差，且每次留档（时间 + diff + 令牌前缀）。
- **审核档**：其余一切（scopes / ACL 清单 / 依赖 / 跨应用授权 / 服务地址 / 席位 /
  授权文案 scopeLabels / iframe 指向 embedUrl / 部署形态 / 部署前置 env 键清单
  requiredEnv / 服务账号身份 serviceAccount.clientId / 首次接入）都会改变权限面，
  ⇒ 提交成功但进入平台管理员的「发布审核」队列，**批准前库里一字不动**。
  被拒绝时 `status` 与 `--wait` 都能看到原因。

判据只有一条：**这次提交有没有改变权限面** —— 不是一张字段白名单。散字段（往表单里直接塞
`scopes=...`）仍然直接拒：治理变更只能经 manifest 整份提交。

### 那你 repo 里那份 `app.manifest.json` 呢？

**它就是你 App 清单的唯一事实源**，而且**直接参与发版**：
`publish --manifest deploy/portal/app.manifest.json` 把它随提案交上去 ——
无治理变更的自动生效，有治理变更的等平台批准。门户代码里不再保留你清单的副本，
「改了 manifest 却被门户下次部署改回去」的静默漂移已经从机制上消灭。

manifest **绝不携带密钥值**：`serviceAccount.secret`、`deployDescriptor.env` 有值、
SERVICE_ONLY scope（如 `seats.read`）写进 `serviceScopes` 都会**提交即拒**。
要用平台特权 scope，走 `privilegedServiceScopes: [{ "scope": "seats.read", "reason": "为什么需要" }]`
—— 它是**申请**不是授予：必进「发布审核」，审批人逐条确认后你的服务账号才拿到；
reason 会原文展示给审批人，写清楚用途能少一轮往返。上报自己的用量指标，先在
`usageMetrics` 里声明（key 必须以你的 listingKey. 开头；同样治理档）——没声明的
metricKey 会被上报接口拒收并计入 `rejected`。要声明「部署我需要哪些环境变量」
用 `requiredEnv`（只交键名），值永远由平台管理员在控制台填。


> **路径约定**：本 skill **不要求你有门户仓**，也不会让你去打开门户仓里的文件——需要的
> 一切都在这里的 `references/` 与 `scripts/`。
>
> ⚠️ 唯一容易误读的是 `/apps/<key>/`：它是**线上 URL 路径**（你的产物在生产被挂载到的
> 子路径，也就是 vite 的 `base`），**不是**任何仓库里的目录。看到它不要去找、不要去建。

## 先备齐三样，缺一样就发不出去

| 你需要 | 从哪来 | 放在哪 |
| --- | --- | --- |
| `listingKey` | 平台给的 App 标识，小写字母/数字/连字符。它同时是 `/svc/<key>`、`/apps/<key>/`、scope 命名空间、令牌的 aud —— **四位一体，永不改** | 仓里的本地配置文件 `LISTING_KEY=…`（`.gitignore` 掉），`--key` 可覆盖 |
| 发布令牌 `xrel_…` | 平台管理员在 控制台 › 应用市场 › 接入新应用（或 应用清单 › 发布令牌）签发，**明文只显示一次** | `XGENT_RELEASE_TOKEN` 或 `--token`。**是密钥，不进配置文件** |
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
   然后浏览器打开门户 → 应用中心 → 你的 App，走通主路径。**`status` 报 404 不等于发布失败**
   （见下），第 4 步的返回体已经给了版本与 digest，浏览器那一眼照走不误。

`@xgent/release-cli` 不在公共 npm 上；你的环境取不到它时**不要卡在这里**——端点就一条
`POST /api/market/release/:key`，`curl` 兜底见 [references/publish-api.md](references/publish-api.md) §2。

**只读面（`status` / `--wait`）不保证每个门户都有**——它比发布面晚一版上线。同一枚令牌
whoami `200` 而 `/status` `404`，就是这种情况：**令牌没问题，别停下来改令牌或改 key**。
两种 404 的响应体一字不差（门户故意不区分「不属于你」和「不存在」），只能靠 whoami 分诊；
分诊表与替代验收方式见 [references/troubleshooting.md](references/troubleshooting.md)。

## 四条硬约定（都是「不知道就会中」的那种）

- **`version` 每次都要 bump**——哪怕这次只换产物没改功能。产物 digest 变了而 version
  没变，控制台上就再也分不清「线上跑的是哪一版」，而这正是发布链路存在的意义。
  version 归你所有（清单事实源在你仓里）；`--version` 可省略，省略时取 manifest.version。
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
`--wait` 轮询的就是上面那个只读面：**门户上没有它时这一步失效**，「容器换没换」只能人工确认，
如实说明，不要因为流水线绿了就报「新版本已在跑」。

## 首次发布（你的 App 还不在市场里）

1. 平台管理员在 控制台 › 应用市场 › **接入新应用** 输入你的 `listingKey` ⇒ 建一条草稿占位 +
   签发 `xrel_` 令牌（经安全渠道交给你）。此刻你的 App 还不存在，只是有了提交的门。
2. 你第一次 `publish --manifest deploy/portal/app.manifest.json --dist dist/ --image <ref>` ⇒
   **必然进审核队列**（首次提交携带全部治理字段，无论内容）。
3. 平台批准 ⇒ listing 建成上架 + 服务账号建出（client secret 明文一次性回显给审批人，
   平台经外部渠道交给你）+ 产物与镜像同一次生效。
4. 之后的日常发版与老 App 完全相同 —— 首次与后续是同一条代码路径，没有第二套流程。

## 等审批（pending 之后会发生什么）

- `publish` 返回 pending 时**退出码 0**（提交成功不是失败），打印提案 id 与待审字段清单。
- `--wait`（默认 1800s）会轮询到 `applied` / `rejected`：批准且换了镜像 ⇒ 继续等容器换版；
  **被拒绝 ⇒ 打印平台填的原因并非零退出**（CI 该红就红）；超时 ⇒「仍在等审批」+ 非零退出。
- 同一 App 同时只允许一条待审提案，且待审期间**任何新提交都被拒**（纯 dist/version 的
  自动档也一样 —— 放行会「后交先生效」，批准旧提案时把你后发的版本滚回去）：返回
  `PROPOSAL_PENDING` + 在审提案 id，等审批或先撤回。
  撤回是你的权利（不是审批动作）：
  `curl -X DELETE -H "Authorization: Bearer $XGENT_RELEASE_TOKEN" $XGENT_PORTAL_URL/api/market/release/<key>/proposals/<id>`
- 拒绝原因不推送 —— 靠 `status` / `--wait` 轮询看（门户刻意不做通知面）。

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
