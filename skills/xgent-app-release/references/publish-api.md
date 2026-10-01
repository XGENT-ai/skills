# 发布面：令牌、端点、字段、CI

## 0. 五项输入全部放同一份本地配置文件

| 输入 | 键名 | 说明 |
| --- | --- | --- |
| `listingKey` | `LISTING_KEY` | 一个 repo 一个 App，从头到尾不变。每条命令手打一次纯噪音，还容易打成另一个 App —— 那会直接 404 |
| 目标门户地址 | `TARGET_XGENT_PLATFORM` | 本次发版打哪一台（旧名 `XGENT_PORTAL_URL` 仍认）。`--portal` 可覆盖 |
| 目标门户的发布令牌 | `XGENT_RELEASE_TOKEN` | `xrel_…`，按 App 绑定。`--token` 可覆盖 |
| 清单目录地址 | `MANIFEST_STORE` | 可选。不配 ⇒ 不投目录（整步跳过，不报错也不提示） |
| 目录的发布令牌 | `MANIFEST_STORE_TOKEN` | 可选。不配 ⇒ 回落 `XGENT_RELEASE_TOKEN`（两个地址是同一台时那就是对的那把） |

配置文件按顺序取第一个存在的：`--config <path>` / `$XGENT_REGISTRY_CONFIG` /
`./.xgent-registry.env` / `${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env` / `~/.xgent-registry.env`。
格式是 `KEY=value`；同一份文件也放你镜像推送那侧要的键（`REGISTRY` / `PULLER_AUTH`）。
同名环境变量优先，命令行参数又优先于环境变量 —— **CI 里注入环境变量即可覆盖**。

前两个位置是**显式指定**的：指到一个不存在的文件会直接报错，不会悄悄回退到别的候选——
回退意味着你可能在读另一个 App 的身份，而命令照样成功，只是发错了地方。

### 令牌为什么放这里（这条推翻了旧规则）

旧规则「密钥别落盘、走 `--token` 或 env」是按**人手敲命令**的模型写的：env 短暂、文件长存。
今天发版几乎都是**经 skill 让 agent 驱动 CLI**：强制走 `--token`，agent 必须先把令牌**取出来**
才能传进去 —— 于是它进 agent 上下文、进对话记录、进工具调用日志，还出现在命令行里（同机
其它进程 `ps` 可见）。**那比落盘更不安全，而且泄露面不可撤销**（记录已经写出去了）。
让 CLI 自己读文件，令牌就**从不经过 agent**。这个文件本来就是凭证文件（`PULLER_AUTH` 一直在里面）。

护栏没有删，换成了真正管用的两条 —— CLI 每次运行都替你查，**只 warn 不阻断**：

| 检查 | 触发 | 修法 |
| --- | --- | --- |
| 文件权限 | mode 允许同组 / 其他人读 | `chmod 600 .xgent-registry.env` |
| 有没有被忽略 | `git check-ignore` 判定它**未被忽略**（这才是真会泄露的场景） | `echo '.xgent-registry.env' >> .gitignore` |

（做成硬失败只会逼人把令牌搬回命令行 —— 那正是要消灭的那件事。）

```bash
# .xgent-registry.env（chmod 600，且必须 .gitignore）
LISTING_KEY=my-app
TARGET_XGENT_PLATFORM=https://portal.example.com
XGENT_RELEASE_TOKEN=xrel_…
MANIFEST_STORE=https://catalog.example.com
MANIFEST_STORE_TOKEN=xrel_…
```
```bash
npx @xgent/release-cli status      # 地址与令牌都不用写在命令里
```

## 0a. 一次 publish 打两个端点

| 站 | 端点 | 失败语义 |
| --- | --- | --- |
| 第一站（真发版） | `POST <TARGET_XGENT_PLATFORM>/api/market/release/<key>` | **必成**：失败 ⇒ 整体非零退出，且**不投目录** |
| 第二站（投目录） | `PUT <MANIFEST_STORE>/api/market/catalog/<key>` | **best-effort**：连不上 / 非 2xx / `ok:false` 都只 warn，退出码不变 |

第二站仅发送 manifest，**没有 dist 文件**。跨平台同步使用主平台已生效 listing 与原始包，
不是 catalog。主平台归档留存、目标门户与主平台不同时的交付边界，以及同步验收见
[前端归档与同步验收](frontend-archive.md)。不要把投目录成功报告成「前端包已归档/可同步」。

两个地址相同时**照常两次调用** —— 它们是不同端点，不会互撞。
没带 `--manifest` 就不投（没有清单可投），CLI 会提示一句 —— 所以**标准发布命令要带上它**。

第二站送的是这次发版**生效后**的那一份清单，即原始字节**再套上 `--version` / `--image`
的覆盖**。这不是「改了内容」，恰恰是为了两站说同一句话：第一站的服务端本来就取
`body.version ?? manifest.version` 与 `body.image ?? manifest.deployDescriptor.image`，
批准时还会把这两个值嫁接回 manifest 才落 listing。原样投原始字节的结果是：目标门户
已经是 `$VER` / 新镜像，目录还停在清单里写的旧值，而 `onebox.sh add` 正是照
`deployDescriptor.image` 拉镜像 —— 别人起出来的是旧容器，且没有任何信号。

一个刻意的例外：目标门户回 `PROPOSAL_PENDING`（你上一版还在待审队列里）时，
**目录照投**。那是目标门户的**队列状态**，不是「这份清单是什么」的结论；不投的话，
目标门户与目录是同一台时，一条待审提案会把目录永久冻在提交它的那一版。
那次 `publish` 的退出码仍是非 0 —— 发版确实没成。

目录里那份是**净化过的公开副本**：`serviceAccount` 只留 `clientId`，`exchangeInitiatorSecret`
与 `deployDescriptor` 的 `env` / `envFile` / `hostPort` 一律剔除（密钥与单部署事实不跨部署共享）。
它是**只读投影**：不参与任何门户的治理判定、不回写任何 listing、不触发任何部署。
唯一的消费者是开发环境 —— 别人 `onebox.sh add <key> --from <目录地址>` 时从这里拉清单。

## 1. 发布令牌 `xrel_…` 的语义

- 平台管理员在 **控制台 › 应用清单 › 编辑清单 › 发布令牌** 签发，**明文只显示一次** → 直接进 CI secret。
- **一枚令牌只对一个 listing 有效**。拿它去动别的 key 一律 `404`——门户**故意不区分**
  「不属于你」和「那个 App 不存在」，免得令牌变成探测别人 App 是否存在的工具。
- 可随时吊销、可设过期时间。**每次调用实时查库**，所以吊销后下一次调用就 `401`，没有缓存窗口。
- 它能**无需人工批准而应用**的仍只有自动通过档：`dist` · `version` · `deployDescriptor.image` ·
  展示字段 ·「值是 App 内路由 `/…`」的 `helpEntry`（外站 `https://…` 那一形态进审核档）。
  治理变更只能**提议**（随 `manifest` 提交，进平台审核队列，批准前库里一字不动）。
  它也**读不回** `descriptor.env`——响应体里永远没有它，因为那里面是生产密钥。

## 2. 端点

```
POST   /api/market/release/:key                发布（落成一条发布提案）
GET    /api/market/release/:key                whoami（校验令牌，只返回持有者本就知道的东西）
GET    /api/market/release/:key/status?proposalId=<id>  只读：兼容状态 + 指定提案 delivery；省略 id 默认最新
GET    /api/market/release/:key/proposals/:id  只读：单条提案状态 + delivery（--wait-review 审批轮询用）
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
  -F dist=@dist.tgz \             # 使用持久留存的原始包；只换后端可省，但不代表前端归档已补齐
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
⇒ 提案 `pending` 等平台审批；只有版本/产物/镜像/展示字段的差异 ⇒ `auto_approved` 自动通过并应用清单/产物。
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

成功时 `data` 里（先看 `status`：`"applied"` = 清单/产物阶段已应用，不代表整条交付已确认；`"pending"` = 等审批，此时只有
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
| `delivery` | 新门户的本提案交付投影；旧门户可能没有该字段，不能据此确认网关 |

## 5. 应用与执行不是一个事务

坏包在解压/staging 校验阶段被拒时不会替换线上产物。之后的清单/产物应用、后端部署与网关加载
是分阶段执行：后续失败可能发生在清单已经应用之后，须查看 applyError 与 delivery.failureStage。
不能把任意“发布失败”解释为所有副作用都没发生，也不能把提案 applied 当成完整交付。

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
执行失败须回读对应阶段；旧容器一旦已被替换，不能承诺继续提供旧服务。

## 6.1 按本次提案查询交付

新增契约的版本与启用前提见 [公开面与交付确认](public-delivery.md)：这些能力尚待正式发包/上线；
目标门户须提供 delivery，扩展 CSP 还须 strict 与受控 v2 网关。不得仅按包版本标签猜能力已开。

```bash
npx @xgent/release-cli publish --version "$VER" --dist dist/ --manifest app.manifest.json --wait
npx @xgent/release-cli publish --manifest app.manifest.json --wait-review
npx @xgent/release-cli status --proposal-id <id> --wait
```

| 开关 | 等什么 | 新提交为 pending 时 |
| --- | --- | --- |
| `--wait [秒]` | 获批后，本次固定目标的后端 + 网关；默认 1800 秒 | 打印提案号，退出 0；不等待人工 |
| `--wait-review [秒]` | 人工审批，再等待本次交付；默认 1800 秒 | 等待；拒绝、撤回或超时非零退出 |
| `status --proposal-id <id> --wait` | 继续查询这一提案，不能偷换成最新任务 | 等本次提案交付或超时 |

纯前端 backend=not_applicable，仍要等 gateway。failed/superseded 使等待非零退出；unknown 保持
未确认，超时非零退出。普通 status 只展示；配 --wait 才按交付结果设置退出码。
旧门户完全没有 delivery 字段时，CLI 明示“无法核验网关”，只兼容原后端规则；即使退出 0，
也不能声称端到端确认。有字段但为 null/unknown 则缺乏证据，不能冒用旧任务成功。

旧版 status 字段保留 version/distDigest/distUpdatedAt/image/deployment/proposal；指定 proposalId
只限定新增 delivery，旧 proposal 字段仍可能是最近提案。不要混用两者。新 delivery 的形状：

| 字段 | 含义 |
| --- | --- |
| proposalId / approval | 被查询提案及其审批状态 |
| state | pending / applied / failed / unknown / superseded |
| target | null 或固定目标摘要：manifestDigest、distDigest、deploymentConfigDigest、gatewayDigest、gatewayRevision |
| backend | state= pending / ready / failed / unknown / not_applicable；jobId 与脱敏 error |
| gateway | state= pending / applied / failed / unknown / not_applicable；desiredRevision、appliedRevision 与脱敏 error |
| failureStage / error | approval / application / backend / gateway，或 null；仅公开脱敏状态 |

旧目标缺失及无法证明的运行事实为 unknown；后续目标取代本次为 superseded，哪怕摘要后来回到
相同值也不能借旧修订冒认。只有本次实际后端目标和匹配网关回执齐备才 applied。
最近一条 succeeded job 或 deployment.ready 不能替代上述判据。

维护者查询仍用绑定此 App 的 release token：跨 key 不可读，指定别的 App 的 proposalId 也无数据。
新交付状态不返回环境值、内部 upstream 或原始执行日志。可交接提案号、摘要和脱敏失败阶段。

平台管理员另有控制台 session 接口：

```
GET  /api/console/releases/:id/delivery
POST /api/console/releases/:id/retry-gateway
Body: { "expectedTargetDigest": "<当前提案的 gatewayDigest>" }
```

Envelope.data 为同一 delivery DTO；重试必须目标仍有效，旧目标冲突拒绝。重试仅协调网关，
不重放清单，不触发后端重部署。发布令牌不能调用该管理接口；非平台租户无权使用。

## 7. CI 范式

`listingKey` 从仓里的配置文件来，所以 CI 里不用重复它。先在 CI 中准备好本 skill，将 `SKILL_DIR` 设为它的 `SKILL.md` 所在目录；以下步骤在 App repo 根目录执行。

```yaml
env:
  SKILL_DIR: "<本 skill 的 SKILL.md 所在目录>"
  # 仓里那份 .xgent-registry.env 通常不进 CI —— 在这里注入即可（环境变量优先于文件）。
  TARGET_XGENT_PLATFORM: https://portal.example.com
  XGENT_RELEASE_TOKEN: ${{ secrets.XGENT_RELEASE_TOKEN }}
  MANIFEST_STORE: https://catalog.example.com          # 可选：顺带投一份到清单目录
  MANIFEST_STORE_TOKEN: ${{ secrets.MANIFEST_STORE_TOKEN }}

steps:
  # ⓪ 必须在最前：@xgent/release-cli 自己就在私有包仓上，.npmrc 没配的话 ① 取不到包。
  - run: eval "$(node "$SKILL_DIR/scripts/npm-token.mjs")"   # 换私有包只读令牌
  - run: npx @xgent/release-cli whoami                  # ① 先验令牌，别等构建完才发现过期
  - run: <你自己的依赖安装与构建>                        # ② base=/apps/<key>/
  - run: node "$SKILL_DIR/scripts/preflight.mjs" --dist dist --version $VER
  # 按 frontend-archive.md 生成 DIST_ARCHIVE、校验文件；使用 CI 的跨步骤变量机制传递包路径。
  - run: bunx @xgent/release-cli publish --version "$VER" --dist "$DIST_ARCHIVE" --image <key>:$VER --wait
          --manifest deploy/portal/app.manifest.json   # ← 不带它目录永远是空的
  # 无论发布成功、待审或失败，都将原始包、校验文件及发布记录存到项目的持久制品存储。
```

涉及跨平台同步的交付还需按 [前端归档与同步验收](frontend-archive.md) 检查主平台条目与下载摘要；
`--wait` 等部署/网关，不核验当前归档可下载。

`--wait` 是让这条流水线**诚实**的那一步：没有它，换版失败时任务照样绿。
它**不会**把流水线卡在人工审批上——改了权限面的那次发版会打印提案号后退出 0，平台管理员
已被通知；真要门禁到审批，把 `--wait` 换成 `--wait-review`。

版本号从哪来：用 git tag 或 `package.json` 的 version 都行，关键是**每次发布都不同**。
把它固定成 `latest` 之类的常量，等于放弃「线上跑的是哪一版」这个能力。

`--dry-run` 只打印将要发送的内容、不发请求，改 CI 脚本时先跑它。

## 8. 私有包只读令牌（`GET /api/market/release/:key/npm-token`）

`@xgent/{release-cli,shared,portal-sdk,portal-server-sdk,portal-ui}` 都在私有包仓上 —— **包括发版用的 CLI 本身**，
所以这一步是整条发布链的前置，不只是装依赖。**你不需要云账号**：拿同一枚 `xrel_`
向门户换一枚 ≤12 h 的只读令牌，门户持那把云凭据。

```
GET <TARGET_XGENT_PLATFORM>/api/market/release/<key>/npm-token
Authorization: Bearer xrel_…
→ 200 { ok: true, data: { registry, scopes: ["@xgent"], token, expiresAt } }
```

| 情况 | 响应 | 你该做什么 |
| --- | --- | --- |
| 没令牌 / 令牌失效 | `401` | 重新拿一枚发布令牌 |
| 令牌绑的是别的 App | `404` | 核对 `LISTING_KEY` |
| 打太频繁 | `429` | CI 里只换一次，把结果传下去 |
| 平台没配私有包仓库 | `200` + `NPM_REGISTRY_NOT_CONFIGURED` | **平台侧**的事，贴给管理员 |
| 平台的仓库凭据被拒 | `200` + `NPM_REGISTRY_UNAUTHORIZED` | 同上，你这边不用改 |
| 仓库暂时不可达 | `200` + `NPM_REGISTRY_UNAVAILABLE` | 重试一次 |

- 令牌是**域级只读**的：能装 `@xgent/*`（`release-cli` 也在内），不能发布、不能删。
- 不缓存到文件：它 12 h 就过期，CI 每次跑现换即可（门户侧自带缓存，不会每次都打云上）。
- 客户端脚本 `scripts/npm-token.mjs`（`--raw` / `--npmrc` / `--check`）把这些都封好了。
