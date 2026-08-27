# 发布面：令牌、端点、字段、CI

## 0. 三样输入分别从哪来

它们的性质不同，所以来路也不同——混着放会出事。

| 输入 | 来路 | 为什么 |
| --- | --- | --- |
| `listingKey` | **本地配置文件**（`LISTING_KEY=…`），`--key` 可覆盖 | 一个 repo 一个 App，这个值从头到尾不变。每条命令手打一次纯噪音，还容易打成另一个 App —— 那会直接 404 |
| 门户地址 | `--portal` 或 `XGENT_PORTAL_URL` | 随环境变（staging / prod）。写死进仓里迟早会让两个环境共用一个值 |
| 发布令牌 | `--token` 或 `XGENT_RELEASE_TOKEN` | 它是密钥。**不进配置文件**——CLI 发现文件里有令牌会告警并忽略。CI 里优先用环境变量，命令行参数会进 `ps` 和构建日志 |

配置文件按顺序取第一个存在的：`--config <path>` / `$XGENT_REGISTRY_CONFIG` /
`./.xgent-registry.env` / `${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env` / `~/.xgent-registry.env`。
格式是 `KEY=value`；本 CLI 只读 `LISTING_KEY`，同一份文件也可以放你镜像推送那侧要的键。
**记得加进 `.gitignore`。**

前两个位置是**显式指定**的：指到一个不存在的文件会直接报错，不会悄悄回退到别的候选——
回退意味着你可能在读另一个 App 的身份，而命令照样成功，只是发错了地方。

```bash
# .xgent-registry.env（不提交）
LISTING_KEY=my-app
```
```bash
export XGENT_RELEASE_TOKEN=xrel_…   # CI secret
npx @xgent/release-cli status --portal https://portal.example.com
```

## 1. 发布令牌 `xrel_…` 的语义

- 平台管理员在 **控制台 › 应用清单 › 编辑清单 › 发布令牌** 签发，**明文只显示一次** → 直接进 CI secret。
- **一枚令牌只对一个 listing 有效**。拿它去动别的 key 一律 `404`——门户**故意不区分**
  「不属于你」和「那个 App 不存在」，免得令牌变成探测别人 App 是否存在的工具。
- 可随时吊销、可设过期时间。**每次调用实时查库**，所以吊销后下一次调用就 `401`，没有缓存窗口。
- 它能**单方面生效**的仍只有自动通过档：`dist` · `version` · `deployDescriptor.image` ·
  展示字段 ·「值是 App 内路由 `/…`」的 `helpEntry`（外站 `https://…` 那一形态进审核档）。
  治理变更只能**提议**（随 `manifest` 提交，进平台审核队列，批准前库里一字不动）。
  它也**读不回** `descriptor.env`——响应体里永远没有它，因为那里面是生产密钥。

## 2. 端点

```
POST   /api/market/release/:key                发布（落成一条发布提案）
GET    /api/market/release/:key                whoami（校验令牌，只返回持有者本就知道的东西）
GET    /api/market/release/:key/status         只读：版本 / 产物 digest / 部署状态 / 最近一条提案
GET    /api/market/release/:key/proposals/:id  只读：单条提案状态（--wait 轮询用）
DELETE /api/market/release/:key/proposals/:id  撤回自己【待审】的提案（提交方的权利）
```

认证只有一件事：`Authorization: Bearer xrel_…`。没有 cookie、没有 session、没有 CSRF 面。

`curl` 兜底（`@xgent/release-cli` 取不到时用这个，能力完全等价）：

```bash
tar czf dist.tgz -C dist .        # 根下就是 index.html
curl -X POST "$XGENT_PORTAL_URL/api/market/release/<key>" \
  -H "authorization: Bearer $XGENT_RELEASE_TOKEN" \
  -F version=1.4.2 \
  -F image=<key>:1.4.2 \          # 可选；没有后端的纯前端 App 省掉
  -F dist=@dist.tgz \             # 只 bump 版本/换镜像时可省掉
  -F manifest=@deploy/portal/app.manifest.json   # 可选；清单变更/首次接入时带上
```

> `deploy/portal/app.manifest.json` 是【你自己 App 仓】的惯例路径，不是门户仓文件 ——
> 你的 manifest 只存在于你自己的 repo，按你实际存放的位置传即可。

`whoami` 就是同一路径的 `GET`，带同一个 header。

**前两条是发布面的底线，第三条不是**：只读面比发布面晚一版上线，老门户上打它得到
`404 NOT_FOUND / 路由不存在`——与「令牌不是这个 key 的」**响应体完全相同**（门户故意不区分，
免得令牌能用来探测别人的 App 是否存在）。分诊只有一条路：同一枚令牌先打 whoami，
`200` 就说明是门户没有这个面。见 `troubleshooting.md`。

## 3. 字段：四个 multipart 字段 + 定级

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `version` | （或 manifest.version） | 字母/数字/`. _ + -`，≤64 字符。**每次都要 bump** |
| `dist` | | multipart 文件，`.tar.gz`，根下 `index.html`，≤64MB。省掉 = 这次不换产物 |
| `image` | | 镜像引用。省掉 = 不换镜像 |
| `manifest` | | `app.manifest.json` 全文（文件或 JSON 字符串）。省掉 = 只发三件（全自动档） |

**这四个之外的散字段仍然直接拒**（`200 + VALIDATION_FAILED`，拒在写库之前）——
治理变更只能经 manifest 整份提交，没有「往表单里塞一个 scopes」这条路。

**定级唯一判据：这次提交有没有改变权限面。** manifest 里与当前 listing 相比有治理差异
（scopes / aclManifest / dependencies / exchangeTargets / serviceScopes /
privilegedServiceScopes / usageMetrics / serviceBaseUrl /
seat* / scopeLabels / embedUrl / embedCsp / type / 部署形态…，以及**任何门户不认识的字段**）
⇒ 提案 `pending` 等平台审批；只有版本/产物/镜像/展示字段的差异 ⇒ `auto_approved` 当场生效。
其中 `privilegedServiceScopes`（平台特权 scope 的申请，如 `seats.read`）是最高一档：
审批人要**逐条勾选确认**才能批准 —— 每条都带上说清用途的 `reason`，等待会短很多；
已持有的特权 scope 幂等重放不再进审。
**提交即拒**（连提案都不落）的只有四类：manifest 携带密钥值（SA secret / descriptor.env）、
SERVICE_ONLY scope 写进 `serviceScopes`（申请要走 `privilegedServiceScopes`）、形状非法
（含 `usageMetrics` 的 key 不在你的 listingKey 命名空间、或声明金额单位），
以及**该 App 已有一条待审提案**（`PROPOSAL_PENDING`——
待审期间连纯 dist/version 的自动档也拒，否则「后交先生效」，批准旧提案时会把后发的版本滚回去）。
另注意 `serviceAccount.clientId` 归属也算权限面：声明一个已归属别的 App 的 clientId ⇒
`pending`，且批准也会在生效时被拒 —— clientId 用自己的。

## 4. 响应怎么读

**业务失败是 `200 + { ok:false, error }`**（门户全局约定：HTTP 状态码只表达传输/路由层）。
所以只看 HTTP 状态码会把「产物形状不对」读成成功。判据固定是 `body.ok`。

只有三种情况不是 200：`401`（令牌无效/吊销/过期）、`404`（令牌不是为这个 key 签发的），以及真故障。

成功时 `data` 里（先看 `status`：`"applied"` = 已生效；`"pending"` = 等审批，此时只有
`proposalId` / `kind` / `governance`（待审字段清单）有意义）：

| 字段 | 含义 |
| --- | --- |
| `proposalId` / `status` / `kind` | 本次提案：id · applied/pending · register(首次)/update |
| `governance` | pending 时的待审字段清单 |
| `version` | 落库后的版本号 |
| `distDigest` | 本次产物的 sha256（`null` = 这次没换产物）。**控制台上「线上跑的是哪一版」靠它** |
| `distFiles` | 产物顶层条目数 |
| `distStores` | 落了哪些存储 |
| `image` / `imageChanged` / `redeployQueued` | 镜像引用、是否变了、是否排了重部署任务 |

## 5. 原子性：坏包碰不到线上那份

一次调用内：解压 → 验根 `index.html` → 落 staging → rename 换上 → **同一次落库** `version` + `distDigest`。
被拒时 `version` 与 `digest` 都不动，线上产物原封不动。

所以「发布失败了，线上是不是已经半坏了」这个担心不成立——失败就是没发生。
反过来也意味着：**版本号与产物永远同进同退**，不会出现「产物换了版本号没换」的漂移。

## 6. `--image` / `image`

给有后端的 App 用。三条硬约定：

1. **`<name>` 就是你的 App key**，不是 `<key>-server`。写错了拼出来的仓库根本不存在，
   而这件事只有在真正 `docker pull` 的那一刻才会暴露（往往是切换窗口里）。
2. **只写相对名**（`<key>:<tag>`），仓库前缀由门户在部署时拼。把前缀写死进来，换仓库就得改你的仓。
   判据与门户一致：含 `/` 且首段含 `.` 或 `:` 或等于 `localhost` 才算完整引用——
   所以 `my-app:1.4.2` 是相对名，`hub.example.com/x/y:1` 不是。
3. **tag 不可变，新版本 = 新 tag。** 同 tag 覆盖推送在门户侧看不出变化（引用没变），不会触发换版。

前提：该 App 已由平台管理员配了 `deployDescriptor`。没有的话这一项直接被拒——
「给一个没有部署描述的 App 加镜像」等于决定它从此是个被部署的后端，那是治理动作不是发布动作。

镜像本身先 `docker push` 到平台 registry。**推送那一步不在本 skill 的范围内**，它有自己的流程与预检
（跨境链路、架构、tag 不可变、保留策略），照那边的规矩来。这里只提三条会直接影响发布结果的：

- **仓库域名不写死。** 它是内部信息，由各仓的本地配置提供（形如 gitignore 掉的 `.xgent-registry.env`，
  CI 里用同名环境变量注入）。所以你的 `--image` 只写相对名，域名从头到尾不该出现在你的发布脚本里。
- **robot 密钥的权限是项目级**，不是「只能推你那一个 repository」——密钥泄露等于同项目下所有 App
  的 repository 都能被推拉。别放公共 runner。能推别人的不等于可以推。
- **只发单 arch 的话必须是 amd64**（两条生产链路都是 amd64）；Apple Silicon 上直接 build 出来是 arm64，
  **推得上去、拉得下来、容器起不来**（`exec format error`），看着像仓库坏了其实不是。
  要多 arch 用 `docker buildx --platform`。

引用一变，门户排一条重部署任务：pm2 侧**先拉后换**（拉不到则旧容器原封不动、任务转 failed），
K8s 侧滚动更新（旧 Pod 在新 Pod ready 前不下线）。**服务不会因为一次发布断掉。**

## 6.1 `--wait` / `status`：确认换版真的落地了

`publish` 返回 `redeployQueued: true` 只说明**任务排上了**。前端产物是同步的（那一刻已在线上），
换容器不是——它可能几十秒后才完成，也可能失败（镜像没 push、robot 没权限、tag 打错）。
不确认这一步，CI 会绿着退出而线上还跑着旧容器。

```bash
# 发布并等到换版结束（默认 1800s —— 含等人工审批的情况，可写 --wait 600）
npx @xgent/release-cli publish --key $KEY --version $VER --dist dist/ --image $KEY:$VER --wait

# 或任何时候单查
npx @xgent/release-cli status --key $KEY
```

`--wait` 成功 → 打印实际在跑的镜像引用、零退出；失败或超时 → 打印原因、**非零退出**。
没换镜像时它直接跳过，不空等。镜像来源不限 `--image`：**manifest 的
`deployDescriptor.image` 换了镜像同样会等**（提案批准后接着等容器换版）。判据看的是
最近一条 `deploy|redeploy` 任务，不掺 `provision-tenant`（那是某个租户的 bootstrap，
与镜像滚动无关）。

门户上没有这个面时，`status` 与 `--wait` 都用不了：产物那半边照旧由 `publish` 的返回体
（`version` / `distDigest` / `redeployQueued`）交代清楚，容器那半边只能找平台管理员看
「服务部署」面板。**别用「命令退出码 0」替代这半边的结论。**

`status` 返回：

| 字段 | 含义 |
| --- | --- |
| `version` / `distDigest` / `distUpdatedAt` | 线上现在是哪一版、哪一份产物 |
| `image` | 你**声明**的相对引用（不是解析后的完整引用） |
| `deployment` | `null` = 纯前端 App，没有后端可等 |
| `deployment.status` | `not_deployed` / `deploying` / `ready` / `failed` |
| `deployment.deployedImageRef` | **实际**在跑的完整引用（含仓库前缀）——「为什么还是旧版」靠它自查。CLI 默认只打印末段（域名是内部信息，没必要每次构建都刷进 CI 日志），**只在与声明值对不上时**才展开完整引用——那正是需要看清来源的场合，同名 tag 也可能来自另一个仓库 |
| `deployment.lastJob` | 最近一条部署任务的 `action` / `status` / `error` / `finishedAt` |

这个面是**只读**的，且仍在同一条边界内：同一枚令牌、同样绑死一个 listing、跨 key 一样 404。
`descriptor.env` 读不回来——**连藏在部署错误文本里的那份也不行**，出口会对它做值级抹除，
所以错误消息仍然可读（`manifest unknown` 之类留着），只有密钥被换成 `«redacted»`。

## 7. CI 范式

`listingKey` 从仓里的配置文件来，所以 CI 里不用重复它。

```yaml
env:
  XGENT_PORTAL_URL: https://portal.example.com
  XGENT_RELEASE_TOKEN: ${{ secrets.XGENT_RELEASE_TOKEN }}

steps:
  - run: npx @xgent/release-cli whoami                  # ① 先验令牌，别等构建完才发现过期
  - run: <你自己的依赖安装与构建>                        # ② base=/apps/<key>/
  - run: node .claude/skills/xgent-app-release/scripts/preflight.mjs --dist dist --version $VER
  - run: npx @xgent/release-cli publish --version $VER --dist dist/ --image <key>:$VER --wait
```

`--wait` 是让这条流水线**诚实**的那一步：没有它，换版失败时任务照样绿。

版本号从哪来：用 git tag 或 `package.json` 的 version 都行，关键是**每次发布都不同**。
把它固定成 `latest` 之类的常量，等于放弃「线上跑的是哪一版」这个能力。

`--dry-run` 只打印将要发送的内容、不发请求，改 CI 脚本时先跑它。
