# 排查速查

先分清事故落在哪一层——**发不出去** / **发出去了但页面坏** / **发出去了但线上没变**。
三层的成因不重叠，混着猜会很久。

## A. 发不出去（命令报错）

| 症状 | 多半原因 | 怎么确认 |
| --- | --- | --- |
| `401` | 令牌无效 / 已吊销 / 已过期；或压根没带 `Authorization: Bearer xrel_…` | `whoami`。门户每次调用实时查库，所以「昨天还能发」不构成反证 |
| `404` | 令牌不是为这个 `--key` 签发的（也可能该 listing 不存在——门户**故意**不区分） | 核对 `--key` 与签发时的 key 逐字相同；换一个你确定存在的 key 试同一枚令牌 |
| `GET …/<key>/status` 返回 `404`，但同一枚令牌 whoami `200` | **这个门户上还没有只读面**（它比发布面晚一版上线），不是令牌的问题 | 见下面「`/status` 报 404 怎么分诊」。别改令牌、别重发 |
| `200` 但 `ok:false`，说缺根 `index.html` | 打包套了一层 | 用 `tar czf x.tgz -C dist .`，不是 `tar czf x.tgz dist`；或 `--dist` 指错了一层 |
| `200` 但 `ok:false`，说超 64MB | source map / 未压缩素材打进产物了 | `du -sh dist`，关掉 sourcemap 再看 |
| `200` 但 `ok:false`，说版本号非法 | 只能字母/数字/`. _ + -`，≤64 字符 | 别把 `v1.2.3 (build 42)` 这种带空格括号的塞进去 |
| `200` 但 `ok:false`，说属治理字段不接受散字段 | 往表单里直接塞 `scopes=...` / `aclManifest=...` 之类 | 散字段只认 `dist`/`version`/`image`/`manifest`；治理变更写进 `app.manifest.json` 用 `--manifest` 整份提交，进「发布审核」 |
| `200` 但 `ok:false`，`PROPOSAL_PENDING` | 该 App 已有一条待审提案挡路——待审期间**任何**新提交都拒，纯 dist/version 也一样（放行会「后交先生效」） | 等平台审批，或先撤回：`DELETE …/proposals/<id>`（错误 details 里带在审提案 id） |
| `200` 但 `ok:false`，说没有部署描述 | 带了 `--image`，但该 App 还没被平台管理员配 `deployDescriptor` | 纯前端 App 本来就不该带 `--image`，去掉即可；真是新后端就在 manifest 里声明 `deployDescriptor` 随提案提交审核 |
| `npx @xgent/release-cli` 取不到（`E404 … registry.npmjs.org/@xgent%2frelease-cli`） | **CLI 自己就在私有包仓上**，而这个 repo 的 `.npmrc` 没配 `@xgent:registry` —— 公共 npm 上当然没有 | `npm config get @xgent:registry`，是 `undefined` 就 `node "$SKILL_DIR/scripts/npm-token.mjs" --npmrc >> .npmrc` 再重试（SKILL.md 第 0 步）。**只有**换令牌这步本身报 `NPM_REGISTRY_NOT_CONFIGURED`（平台侧没配）才用 `curl` 兜底，能力完全等价（见 `publish-api.md` §2） |
| 缺少应用标识 | 没传 `--key`，配置文件里也没有 `LISTING_KEY` | 二选一，见 `publish-api.md` §0 |
| 缺少发布令牌 / 令牌形状不对 | 没设 `XGENT_RELEASE_TOKEN`、也没传 `--token`；或把别的令牌拿来了 | 令牌以 `xrel_` 开头。**别写进配置文件**——CLI 会告警并忽略 |
| 卡几分钟后 `fetch failed` / 上传超时，产物只有一两 MB | **本机配了代理，而 Node 内置 `fetch` 不读它**——直连跨境上行实测约 10 KB/s | 环境里有 `HTTPS_PROXY` / `HTTP_PROXY` 就是它。升到 `@xgent/release-cli` ≥0.6.0（检测到代理会带 `NODE_USE_ENV_PROXY=1` 重启自身）；Node 既不是 ≥22.21 也不是 ≥24 时按 `publish-api.md` §2 用 curl 兜底 |
| `门户网关返回 502`（或任何 ≥500） | **传输层故障，不是内容被拒**——反代/网关把连接切了，门户那边没留下任何痕迹（无提案行、无审计、产物未动） | 看 CLI 打出的已传字节与实测速率：速率低得离谱就是上面那条代理问题；速率正常则是门户侧的事，带上时间点找平台管理员 |
| `tar 不可用` | 构建镜像太精简 | 装 tar，或自己打好包用 `--dist dist.tgz` 传现成的 |

### `/status` 报 404 怎么分诊

两种 404 的响应体**一字不差**（都是 `{"ok":false,"error":{"code":"NOT_FOUND","message":"路由不存在"}}`）——
门户是**故意**让「令牌不属于你」和「这条路由不存在」长得一样的，免得令牌变成探测别人 App
是否存在的工具。所以**别对着 404 的文案猜**，用同一枚令牌先打一次 whoami：

```bash
H="authorization: Bearer $XGENT_RELEASE_TOKEN"
curl -s -o /dev/null -w 'whoami %{http_code}\n' -H "$H" "$XGENT_PORTAL_URL/api/market/release/<key>"
curl -s -o /dev/null -w 'status %{http_code}\n' -H "$H" "$XGENT_PORTAL_URL/api/market/release/<key>/status"
```

| whoami | `/status` | 结论 |
| --- | --- | --- |
| `200` | `404` | 这个门户**没有**只读面（版本旧）。令牌没问题，发布照发 |
| `404` | `404` | 令牌不是这个 key 签发的，或该 listing 不存在 |
| `401` | `401` | 令牌本身无效 / 已吊销 / 已过期 |

**没有只读面时怎么验收**：`publish` 的返回体本身就带 `version` / `distDigest` / `distFiles` /
`imageChanged` / `redeployQueued`——「产物落的是哪一份」这半边一点不缺，照常报告。缺的只有
「容器到底换没换」，那半边只能找平台管理员看「服务部署」面板，或直接压你自己 App 的健康检查。
`--wait` / `--wait-review` 同理会失效（它们就是在轮询这个面），**别把它当成保险还照绿放行**。

## B. 发出去了，但页面坏

| 症状 | 多半原因 |
| --- | --- |
| **iframe 白屏，控制台一片 404** | `base` 没设成 `/apps/<key>/` —— **最常见的一种**。资源请求打到了站点根 |
| 首页能开，**刷新二级页面就 404** | 客户端路由的 basename 没跟着走子路径 |
| 某个功能点了没反应，本地却正常 | 产物里残留 `localhost:53xx`，或直连了未在 `embedCsp.connectSrc` 里声明的外部源 —— 生产 CSP 静默拦掉。看浏览器控制台的 CSP 报错 |
| 页面在，但调后端全失败 | 不是 CORS（前端从不直连后端，请求走宿主 postMessage 代理）。查 SDK 握手、`callService` 的 key、以及该 App 在这个租户里装没装 |
| `/apps/<key>/` 打不开但发布明明成功 | 产物存储侧的事，自己这边查不了 —— 带上本次的 `distDigest` 找平台管理员 |
| 发布成功但看到的是旧页面 | 浏览器/CDN 缓存。硬刷新；仍旧则对比控制台上的 `distDigest` 与本次返回值 |

**B 类问题里最值钱的一句话**：`✓ 已发布` 只证明产物落库了，不证明页面能用。
所以流程的最后一步是在浏览器里真打开一次——这一步没做就别报「发布完成」。

## C. 发出去了，但线上没变

**先跑 `npx @xgent/release-cli status --key <key>`**——C 类的几乎每一条它都能直接回答：
线上是哪一版、实际在跑哪个镜像、最近一条部署任务成没成、失败原因是什么。

这个面较新，**门户上没有它时会 `404`**（先按 A 类那条分诊，别当成发布失败）。那种情况下
下面整张表都只能拿去问平台管理员——你手上只剩 `publish` 的返回体和浏览器。

| 症状 | 多半原因 |
| --- | --- |
| 换了 `--image`，容器还是旧的 | `status` 看最近一条任务：`failed` 就是拉不到镜像（没 push / robot 没权限 / tag 打错）。**先拉后换**，所以旧容器还在跑、服务没断 |
| `status` 显示任务还在 `queued`/`running` | 就是还没轮到/还没跑完。发布时加 `--wait` 就不用手动盯 |
| 提案一直 `pending`，`--wait` 却早早退出 0 | **这是 0.5.0 的既定行为**：`--wait` 只等换版。要卡在审批上用 `--wait-review`；平台管理员在提案落成时已收到通知，不必催 |
| 审批屏说某个库 / Redis 的键「缺」 | 这个键不叫约定名（`<PREFIX>_DATABASE_URL` / `REDIS_CONN_STRING`），平台自动供给不到 ⇒ 只能人工点一下「从平台服务取值」。改成约定名后下次就零手填（预检会提前 warn 这一条） |
| 同一个 tag 重推了，门户没反应 | tag 不可变是硬约定：引用没变**根本不排任务**（`status` 里看不到新任务就是这个）。新版本必须新 tag；真要按原样重建只能找平台管理员点面板的「重新部署」 |
| 版本号发上去了，过一阵又变回旧值 | 该 key 是**门户仓内 App**（清单定义在门户代码里），version 随门户部署被等值对齐回定义值——仓内 App 不走自助发版面。外部 App 的清单事实源在你仓，不存在这个症状；真遇到就是有人在门户控制台手改过，找平台管理员对账 |
| 发布成功、版本也对，但应用市场/租户侧看不到这个 App | listing 被平台**下架**（delisted）。发布刻意不改上下架状态——上下架归平台，找平台管理员重新上架 |
| 带 `--manifest` 提交，返回 `ok:true`，但线上一字没变 | 这次提交里有**治理变更** ⇒ 落成待审提案，批准前库里一字不动（`status` 的最近一条提案是 `pending`）。最容易被忽略的一个触发源是 `helpEntry` 写成了外站文档 `https://…`——外链是审核档，改成 App 内路由 `"/help"` 才是自动档 |
| 控制台上 `version` 与产物对不上 | 正常情况下不可能——两者同一次落库。若真发生，说明有人从别的路径改过 version，带 `distDigest` 去对账 |

## D. 发上去也跑起来了，但跨应用调用 401

症状：你的 App 去 `POST /oauth/token` 换票，门户回 `SECRET_INVALID`；或者界面上只写着
**「跨应用授权缺失或未开启」**——那句话说的是授权，而最常见的真因不是授权。

| 症状 | 多半原因 | 怎么确认 |
| --- | --- | --- |
| `SECRET_INVALID` | `<PREFIX>_APP_SECRET` 两边漂了：容器里跑的是**手填**的那一份，门户布出去的是**平台保管**的那一份的哈希 | 问平台管理员：该 App 的 `deployDescriptor.env` 或 `envFile` 里是不是手填了这个键。有就删掉它——删掉之后指纹变化会自动排一次换版，平台保管的那份就注进去了 |
| `EXCHANGE_NOT_ALLOWED` | 清单里没声明要调的那个目标 App | 在 `app.manifest.json` 的 `exchangeTargets` 里补上目标 key，随 `--manifest` 提交（这是治理档，要等人工审批） |
| 闸说缺 `<PREFIX>_APP_SECRET`，提案一直批不了 | 你在 `requiredEnv` 里写了这个键，却没声明 `exchangeTargets` | 平台不会为一个不做跨应用调用的 App 生成这枚密钥。补 `exchangeTargets`，或把这个键从 `requiredEnv` 去掉。**别让平台管理员手填一个值来让闸放行**——那正好制造上面第一行那个故障 |

这枚密钥的完整口径见 SKILL.md「`<PREFIX>_APP_SECRET`：跨应用交换的发起方密钥」。一句话：
**键名归你，值归平台，明文只在批准那一刻给审批人看一次。**

## 一条通用的分诊起手式

在 App repo 根目录执行；`SKILL_DIR` 表示本 skill 的 `SKILL.md` 所在目录（本文件的上一级）。

```bash
node "$SKILL_DIR/scripts/preflight.mjs" --key <key> --dist dist --version <v>   # 发之前
npx @xgent/release-cli status --key <key>                                  # 发之后
```

前者一次性覆盖 A 类的绝大多数与 B 类的头两条，后者覆盖 C 类（`status` 在这个门户上不存在
就跳过它，不要为了让它绿而改令牌或改 key）。**先跑它再猜**——
这些成因的共同点是 `tsc` / lint / 单测全都发现不了，靠读代码找它们非常慢。
