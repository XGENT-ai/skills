---
name: portal-dev-setup
description: '在你自己 App 的 repo 里，用 Docker 起一个真实的 XGENT.ai 门户（一盒 / one-box）做本地联调——不 clone 门户、不改门户代码、不重建门户镜像。一条 onebox.sh init 检查本地 puller 凭证（.xgent-registry.env 里的 REGISTRY + PULLER_AUTH，缺了就停下来让用户先去开发团队要）→ 自动 docker login 并拉门户镜像 → 从镜像里取出 compose 资产、挑空闲端口、生成 compose.env；再一条 onebox.sh up 按固定顺序 migrate → seed → register-app → 起栈并自动体检；跑不通先 onebox.sh doctor（把排查表能自动判的都判一遍并给出可粘的修法），要拉别的 App 陪调用 onebox.sh add <key>。凡任务涉及「首次把我的 App 接到门户上调试 / 起一盒 / 起本地门户 / 拉门户镜像 / 本地跑不通门户联调 / 一盒重置」，或出现 前端产物目录 /srv/www/apps 不可写、EACCES、duplicate input 导致反代全站 502、拉不到门户镜像、unauthorized、denied、/svc/<key> 404 或 502、localhost 打不开、port is already allocated、COMPOSE_PROJECT_NAME 撞栈把别的容器接管了、portal-api 一直 unhealthy、register-app 报 VALIDATION_FAILED、自省 401、有效 TDT 被判 INVALID_TOKEN、iframe 白屏、跨应用交换在发起方 401 这类症状时，务必先用本 skill 再敲 docker compose——即使用户只说「把门户跑起来」。Use when an external App team brings up, smoke-tests, resets, or debugs a real XGENT portal locally (the one-box Docker stack) from their own repo, without the portal monorepo.'
---

# 门户一盒 · 在自己 repo 里起一个真实门户

**这个 skill 用在 App 自己的 repo 里**——门户代码不在你手上，也不需要在。它起的是一个跑在 Docker 里的
**真实门户**（精简镜像），你的后端/前端接上去，走的就是生产同一套协议：TDT 自省、四道闸、
iframe 托管、跨应用令牌交换。

需要的一切都在这里的 `scripts/` 与 `references/`，加上一件事：**门户镜像**。

以下命令在你的 App repo 根目录执行。先按实际加载位置设置 `SKILL_DIR`，表示本 `SKILL.md` 所在目录；`scripts/`、`references/` 和 `puller.env.example` 都相对于它，不假定安装目录名：

```bash
SKILL_DIR="<本 SKILL.md 所在目录>"
S="$SKILL_DIR/scripts/onebox.sh"
```

> **路径约定（先读这条，能省一次白找）**：本 skill 里出现的 `apps/…` `packages/…` `docs/…`
> `deploy/…` 这类路径**都在门户仓**。在 App 自己的 repo 里它们**不存在** —— 它们标注的是
> 「门户侧的实现在哪」或某段内容的出处，**不是让你去打开的文件**。找不到不是配置错误：
> 别去创建、别去全局搜、别把它当缺失依赖报出来。你需要的一切都在本 skill 的
> `scripts/` 与 `references/`（自包含）。
>
> ⚠️ 三个例外，它们**不是**门户仓路径：`portal-onebox/…`（`init` 在你 repo 里生成的目录）、
> `/apps/<key>/` 与 `/svc/<key>`（带前导斜杠 = 线上 **URL 路径**）、
> `/app/deploy/app-devkit/…`（门户**容器内**的路径，`docker compose run` 读得到）。
>
> 同理，**地址也不写死**：本 skill 里的门户地址一律是 `portal.example.com` 这类占位符或
> `<你的门户地址>`。真实地址只作为**值**写进你自己那份 `.xgent-registry.env`
> （`MANIFEST_STORE` / `TARGET_XGENT_PLATFORM`）。

## 一套就够：一台机器只起一盒

一盒是**全机共用**的联调底座，compose 项目名固定 `xgent-onebox`——它不属于某个 App，
你在本机调的所有 App 都接进这**同一套**。`docker compose ls` 已经能看到一套在跑时，
**别再 `init` 第二套**（init 检测到本机已有别的一盒会直接停下）：

- 要拉平台侧的别的 App 进来陪调：`onebox.sh add <key>`（§2.1），它进的就是这套。
- 要调你自己的**另一个** App（在另一个 repo）：`XGENT_ONEBOX_HOME=<已有 portal-onebox/ 的路径>`
  复用那套栈跑 `register-app` / `dc`；宿主上的 dev 后端则直连一盒的 pg/redis/RustFS（§5）。
- 不用担心端口打架：init 挑宿主端口时逐个探测退让，一盒的 pg/redis/RustFS 落位永远避开
  你本地 dev 栈的 5432/6379/9000-9001（见 §5 的直连表）。

## 0. 先备齐三样

| 你需要 | 从哪来 |
| --- | --- |
| **puller key** | 找**你们自己的开发团队**要。镜像仓库不开放匿名拉取，没有它一盒起不来——见下面「先配 puller key」。**镜像 tag 不用问**：一盒和它的反代都挂了 `latest`，`init` 不给 `--image` 就用它 |
| `listingKey` | 平台给的 App 标识（小写字母/数字/连字符）。它同时是 `/svc/<key>`、iframe 路径 `/apps/<key>/`、scope 命名空间、令牌的 `aud`——四位一体，永不改 |
| `app.manifest.json` | 你自己写、放你自己 repo 的门户契约。样例在 init 之后会出现在 `portal-onebox/app-devkit/manifests/`（一个 micro、一个 service） |

后端镜像可以晚一步：**先把门户本身跑通**是对的第一步，你的 `app-backend` 没配也不影响门户起来。

### 先配 puller key

`init` 会自己 `docker login` + `docker pull`，前提是本地有一份只读拉取凭证。**没有就会停在这一步**——
这时**不要去猜仓库地址、也不要从别处翻凭证**，让用户去找他们自己的开发团队要。

```bash
cp "$SKILL_DIR/puller.env.example" ./.xgent-registry.env
chmod 600 ./.xgent-registry.env
# REGISTRY=<仓库域名>     不带 https://、不带端口、无尾斜杠
# PULLER_AUTH=<base64>    base64 的「用户名:口令」：printf '%s' '<用户名>:<口令>' | base64
# ONEBOX_PROJECT=<项目名> 省掉 --image 时用它拼 <REGISTRY>/<ONEBOX_PROJECT>/one-box:latest
# PROJECT=<项目名>        推【你自己 App 镜像】的项目（xgent-image-push 用，两个 skill 共用本文件）
```

`PULLER_AUTH` 和 `~/.docker/config.json` 里 `auths.<REGISTRY>.auth` 是同一个值，两边可以互抄。
查找顺序：`$XGENT_REGISTRY_CONFIG` → `./.xgent-registry.env` →
`${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env` → `~/.xgent-registry.env` → skill 目录下的 `registry.env`。
**同名环境变量优先**，CI 里注入 `REGISTRY` / `PULLER_AUTH` 即可，不必落盘。

- 这是**团队共用的只读凭证**，不是谁的个人密码：有人离职要找开发团队轮换，而轮换会同时作废全团队的现有凭证。
- ⚠️ **base64 不是加密**，`base64 -d` 一条命令就还原成明文。别提交进 git（skill 目录自带 `.gitignore` 挡了这两个文件名），别转发出团队。
- ⚠️ **`ONEBOX_PROJECT` 与 `PROJECT` 是两个项目，别只填一个。** 一盒是门户团队发的调试镜像，你的 App 镜像在你自己的项目下；拿 `PROJECT` 去拼一盒引用会 `not found`，而报错看上去像 tag 写错（脚本在回退时会明确告警，别忽略那一行）。
- 拿到的是**离线 tar**（没有 registry 访问）就先 `docker load` 两个镜像——两个都在本地时 `init` 会跳过登录与拉取，不需要 puller key。

### 装 `@xgent/*` 私有包？不需要云账号

要装 `@xgent/shared` / `@xgent/portal-sdk` / `@xgent/portal-server-sdk` / `@xgent/portal-ui` 时，
**别去申请云账号、也别装云厂商 CLI**：拿你的**发布令牌**向门户换一枚 ≤12 h 的只读令牌就行（门户持云凭据）：

```bash
RELEASE_SKILL_DIR="<xgent-app-release 的 SKILL.md 所在目录>"
eval "$(node "$RELEASE_SKILL_DIR/scripts/npm-token.mjs")"
npm install
```

- **还没有发布令牌就停下。** 仓里没有 `.xgent-registry.env`（或里面没有 `XGENT_RELEASE_TOKEN`）⇒ 请开发者
  先去门户「开发者应用」› 你的应用 › 凭证 › **生成配置文件**；应用还没建 ⇒ 先在开发者应用里
  「申请建立应用」，平台管理员批准后才能生成。拿到之前**不许 vendor**：不把 `@xgent/*` 拷进仓、
  不用 `file:` / `link:` / tar 依赖。
- 报 `NPM_REGISTRY_NOT_CONFIGURED` 是**平台侧**还没配，贴给管理员，你这边不用改任何配置。

细节见 `xgent-app-release` skill 的「第 0 步」。

## 1. 首次用：一条命令铺好

```bash
"$SKILL_DIR/scripts/onebox.sh" init --key <你的listingKey>
```

不给 `--image` 就取 `<REGISTRY>/<ONEBOX_PROJECT>/one-box:latest`（反代同仓同 tag）——**开新项目不用先去问 tag**。
`latest` 是**可变指针**：它跟着最新一版走，本地已经有同名镜像时 compose 不会回仓库看一眼，
所以沿用 `latest` 升级先 `onebox.sh pull`，再 `upgrade` → `up`（§6），同时更新镜像与部署资产。要钉住某一版（复现一个 bug、或团队统一版本）就显式
`--image <registry>/<项目>/one-box:v<版本>-<sha>`，那种 tag 是不可变的。

它做五件事，都在**你的 repo 根**落到 `portal-onebox/`：

0. **备齐镜像**：读上面那份配置 → `docker login` → 拉 runtime 与 proxy 两个镜像。
   proxy 的镜像名默认从 runtime 推出来（同仓同版本、仓库名 `proxy`），有的部署不叫这个，
   拉不到时用 `--proxy-image` 显式指定。**这一步在最前面，凭证不齐就停在这儿**，
   不会让人跑到一半才发现，也不会留下半个空目录。
1. **从镜像里取出 compose 资产**（`docker create` + `docker cp /app/deploy`）——所以版本天然与镜像对齐，
   不需要门户仓，也不会有「文档里的 compose 和镜像对不上」这种漂移。
2. **挑空闲端口**。HTTP/HTTPS、pg、redis、RustFS 的 6 个宿主端口逐个探测并退让；
   Kafka HOST 另按 9092/19092/29092 挑选，结果写进 `KAFKA_PORT`。Kafka EXTERNAL 独立默认
   发布回环 9095，若已占用，在 `compose.env` 末尾另设 `KAFKA_EXTERNAL_PORT`；它不参与自动挑选。
   挑出来的 pg/redis/RustFS 端口同时就是你宿主进程直连一盒基建的口（见 §5 的直连表）。
3. **生成 `compose.env`**：三段模板拼装 + 一段本机覆盖块（随机密钥、端口、项目名、镜像 tag）。
   Kafka 的 `KAFKA_CLUSTER_ID` 与 `KAFKA_ADMIN_PASSWORD` 每个新盒随机生成，密码不回显；文件权限为 600。
   覆盖块**放在文件最末尾**，因为 docker compose 读 env-file 是**后定义者胜**——以后要改配置也往那下面加，别回上面改。
4. 把 `portal-onebox/` 加进 `.gitignore`（里面是密钥，且随时能重新生成）。

它**不会**覆盖已存在的 `compose.env`（那是这台机器唯一的一份密钥）。已有一盒走 §6 的 `upgrade`；只有重置才用 `init --force`。

跑完编辑 `portal-onebox/compose.env` 末尾，填你的 App：`APP_IMAGE`（后端镜像 tag），
micro 型再加 `APP_FRONTEND_DIST`（前端 dist 的**绝对路径**，目录里要有 `index.html`）。

> `service` 型（无前端、纯 API）**不要**填 `APP_FRONTEND_DIST`——它不露卡、不可打开，只作被调用方。

关于 `APP_IMAGE` 的两条，不知道会各卡一次：

- **容器内必须监听 8080。** 反代的通用规则是 `/svc/<key>/* → <key>-server:8080`，devkit 只注入 `PORT=8080`；
  manifest 里的 `deployDescriptor.port` **一盒不读**（那是生产部署控制器的字段）。镜像不认 `PORT`、认自己那套变量名的，
  把它写进 `compose.env` 末尾（例：`ZO_HTTP_PORT=8080`）。漏了的症状是 `/svc/<key>` 502。
- **生产镜像是 amd64，开发机多半是 arm64。** devkit 跟随本机架构、不会替你转译；amd64 镜像在 arm64 机上
  起不来（`exec format error`）。在 `compose.env` 末尾加一行 **`APP_PLATFORM=linux/amd64`**（Apple Silicon 走
  Rosetta，慢但能跑）。留空 = 跟随本机，行为不变。
  > 这个钩子随一盒镜像走。老镜像的 `app-dev.yml` 里没有它（`"$S" env` 看不到 `APP_PLATFORM` 就是），
  > 那就自己叠一层：`portal-onebox/app-platform.yml` 里写 `services: {app-backend: {platform: linux/amd64}}`，
  > 再 `"$S" dc -f portal-onebox/app-platform.yml up -d app-backend`。

之后一切都走同一个脚本，它会按生效的 env 拼好那串 `--env-file` / 四层 `-f` / 一组 `--profile`：

| | |
| --- | --- |
| `onebox.sh up` | **按顺序把整套铺起来**（迁移→种子→各库→注册→起栈），幂等可重跑，跑完自动体检 |
| `onebox.sh doctor` | **体检**：排查表里能自动判的都判一遍，每条给一行可直接粘的修法。**跑不通先跑它** |
| `onebox.sh add <key>` | 把一个平台侧 App 拉进来陪调：按它的 manifest 拉镜像+注册+建库（+建桶 / `migrateArgs`）+**生成 compose**+起容器+冒烟，非破坏性；目录里的清单型 App（如 observability）`up` 会自动走这一步 |
| `onebox.sh status` | 生效 env + 容器状态 + 宿主侧健康探测 |
| `onebox.sh env` | 只看生效 env 与配置告警 |
| `onebox.sh smoke` | 只跑健康探测 |
| `onebox.sh dc <args>` | `docker compose <拼好的参数> <args>` |
| `onebox.sh chain` | 打印那串参数（想自己敲 compose 时抄它） |
| `onebox.sh pull` | 只（重）拉门户镜像，不更新部署资产。不带参数时从 `compose.env` 读当前版本 |
| `onebox.sh upgrade [--image <ref>]` | 更新镜像与部署资产，保留配置和数据；完成后运行 `up`（§6） |

## 2. 起栈：顺序本身是契约

```bash
S="$SKILL_DIR/scripts/onebox.sh"
"$S" up          # ← 就这一条。按顺序铺完，跑完自动体检
```

顺序是契约（种子会 truncate、反代启动时才读 /svc map），`up` 存在的意义就是**不让人自己记它**。
它幂等，改完 manifest 或 compose.env 重跑即可。下面这几步是它替你做的事——想手动控某一步时照着敲：

```bash
# 1) 基础设施由 "$S" up 先做资产检查、存量对象存储切换，再起 postgres/redis/rustfs/kafka。
#    不手工 dc up rustfs；旧资产会在启动前拒绝并提示先 upgrade。
#    以下是 up 成功后单独重跑某一步的示例。

# 2) 门户库 + 四个基础服务各自的库（它们的 xgent-* 库由 postgres 首次初始化时建好）
"$S" dc run --rm portal-api bun run db:migrate
for k in files llm-gateway git org; do "$S" dc run --rm portal-api bun run db:$k:migrate; done

# 3) 一盒种子   ★ 破坏性：truncate cascade，清掉租户/用户/清单/安装
"$S" dc run --rm portal-api bun run db:seed:onebox

# 3b) 你自己的库：新镜像在 postgres【首次初始化】时按 APP_KEY 建好 `xgent-<key>`；
#     老镜像不建，换过 APP_KEY 的也不会补建（初始化脚本只在数据卷为空时跑一次）。
#     连不上库的症状是你的容器起不来，而不是一条像样的报错 —— 先确认它在：
"$S" dc exec postgres psql -U postgres -lqt | grep -q 'xgent-<你的key>' \
  || "$S" dc exec postgres psql -U postgres -c 'CREATE DATABASE "xgent-<你的key>"'
#     迁移怎么跑以你的镜像为准（多数是启动自迁，或一条 --role migrate 之类的 argv）。

# 4) 注册你的 App：读 manifest → 建 listing + 服务账号 + 写 /svc 放行 map
"$S" dc run --rm -v "$PWD/<放 manifest 的目录>:/devkit:ro" \
   portal-api bun run register-app /devkit/app.manifest.json

# 5) 起门户三件套 + 基础服务 + 你的 app-backend
"$S" dc up -d
```

顺序约束，颠倒了症状都很难反查：

- **各服务自己的库要在 `db:seed:onebox` 之前迁移完。** 一盒里 deploy-controller 启动即拒，
  所以「逐租户基线」由种子代跑（`org` 的预置档案字段就是这么来的）。表还没建就只是一条
  `⚠ org per-tenant baseline FAILED`，栈照样起得来 —— 症状要到你在组织架构里**建第一个员工**
  时才出现：`UNKNOWN_FIELD 未知字段：name`（零字段定义时连预置的「姓名」都是未知 key）。
  补救：`"$S" dc run --rm portal-api bun run db:org:migrate` 之后重跑一次 `db:seed:onebox`。
- **`db:seed:onebox` 必须在 `register-app` 之前。** 种子第一步是 `truncate … marketplace_listings … cascade`——
  先注册后种子 = 你的 App 注册被静默清掉，市场里找不到它。
- **`register-app` 最好在 `reverse-proxy` 之前。** 它落的是一个 `/svc` 放行 map 文件，而反代**启动时**才读那个目录。
  反代已经在跑就补一句 `"$S" dc exec reverse-proxy caddy reload`，否则 `/svc/<key>` 一直 404。
- **`register-app` 跑完先去市场里确认卡片在。** dev 模式下它会顺手把这条 listing 授予现有租户（生产是平台管理员显式勾选），
  但**旧一点的一盒镜像里没有这段代码**——症状是控制台显示已上架、租户市场里连卡片都不出现，且**没有任何报错**。
  看不到就按 §6 更新镜像缓存后 `"$S" upgrade` → `"$S" up` 同步资产，或按 troubleshooting §6 手工授予，别接着往下排查前端。
- **要做跨应用交换的，`register-app` 得跑两次。** 发起方 App Secret 绑在**已安装实例**上，所以是
  `register-app` → 在应用市场里装上你的 App → **再跑一次 `register-app`**（幂等）。漏了的症状是交换在发起方 401。

一盒里带四个基础服务：`files`（文件管理）· `llm-gateway`（大模型网关）· `git`（Git 服务）·
`org`（组织架构：部门/岗位/任职/汇报线），外加一个**平台基础服务应用** `observability`（日志与监控）。
你的 App 要用前四个的数据，就在 manifest 里声明 `exchangeTargets` 走令牌交换；
组织架构另有一条**服务态**只读面（`org.read`，不走用户态交换、也不需要 consent）：
`/svc/org/api/v1/query/superiors`（批量查直属上级，上限 200）与 `/svc/org/api/v1/query/capabilities`
（`{ superior: boolean }` —— 本租户有没有汇报数据）。⚠️ 一盒装上的是**空组织**，要数据先在
App 里走「员工 → 导入」（CSV），否则上级一律回 `null` / `reason: "no-employee"`。

日志与监控不用声明——种子把它登记成平台基础服务应用，你的 App 的服务账号
**默认就持有 `observability.ingest`**，拿服务态令牌直接写 `/svc/observability/v1/ingest/<stream>`（落 `app_<你的key>_<stream>`）。

### Kafka 的启动边界

Kafka 随基础设施常驻，`up` 单独启动并最多等健康 60 秒；失败只告警并显示日志，继续门户与现有 App 初始化。
`doctor` 检查 Kafka 的 TCP 监听；探针只证明监听已打开，不能替代客户端的认证、授权与收发验证。
`add` 识别 `requiredServices: [{"kind":"kafka","name":"main"}]`，也兼容 `requiredEnv` 中的
`<PREFIX>_KAFKA_USERNAME`。它为 App 创建 SCRAM-SHA-512 用户与前缀 ACL，并写入五键：
`<PREFIX>_KAFKA_BROKERS`、`_KAFKA_USERNAME`、`_KAFKA_PASSWORD`、`_KAFKA_SASL_MECHANISM`、
`_KAFKA_TOPIC_PREFIX`（后四项同样带 `<PREFIX>`）。`<PREFIX>` 是 listingKey 大写、连字符改成下划线。
密码第一次生成后写入 `compose.env` 并显示 `***`，重复 `add` 及失败重试沿用原值。
不要为联调把 `KAFKA_ADMIN_PASSWORD` 加进 App 的 `requiredEnv`。

例如 `order-worker` 的用户名是 `app_order_worker`，主题、消费组、事务 ID 都必须以
`app.order-worker.` 开头。机制固定 `SCRAM-SHA-512`。App 必须显式建主题（副本数 1），不能修改
保留期或增加分区；前缀外访问被 broker 拒绝。Kafka 供给失败只告警，保留已保存的密码，按提示
修复后重跑原 `add`。一盒仍向容器注入整份 `compose.env`，不承诺 App 容器之间的密钥隔离。
声明范例、客户端配置、最小 ACL 与可粘的手工修复命令见 [Kafka 接入指引](references/kafka.md)。

宿主地址为 `127.0.0.1:<KAFKA_PORT>`，容器网络为 `kafka:9092`，均使用 SASL_PLAINTEXT。
一盒 EXTERNAL 默认 `127.0.0.1:9095`，保持回环，不开放公网。生产跨 VM 的外部口配置由运维处理：
监听地址、非回环广告地址和安全组需同时匹配，且 SASL_PLAINTEXT 不加密消息。

### 2.0a 日志与监控是怎么进来的（`up` 替你做的 ④b / ⑥）

- **④b** `observability` 不是门户内置 workspace，是清单型 App（`app-devkit/manifests/observability.manifest.json`）。
  `up` 对目录里每个清单型 App 自动走一遍 `add`：拉镜像（`<REGISTRY>/<ONEBOX_PROJECT>/observability:v0.5.4-…`；
  多架构清单会自动挑本机架构，只有单 amd64 的版本才要在 compose.env 钉 `OBSERVABILITY_PLATFORM=linux/amd64`
  走仿真）→ 注册 → 建库 `xgent-observability` + 在一盒 RustFS
  建桶 `xgent-observability` → `init-db`（清单 `deployDescriptor.migrateArgs`）→ 生成 `generated/observability.yml` 起容器。
  它要的那批 `ZO_*` 值（元数据库/对象存储）已经在 `onebox/compose.env.onebox.example` 里指向一盒的 postgres/RustFS；
  `OBSERVABILITY_PORT=8080` 让它在 8080 上听（一盒反代只反代 `<key>-server:8080`）。
- **⑥** 门户自身的控制台日志：`up` 用 `portal-self` 签一把只带 `observability.ingest` 的 `xsak_` 写进 compose.env
  的 `OBS_ACCESS_KEY`，再起 `portal-logs`（fluent-bit）；portal-api 的 stdout/stderr 走 docker 的 fluentd 日志驱动
  进去，落 `app_portal_console`（`docker logs portal-api` 照常可用）。采集段已并入
  `onebox/docker-compose.onebox.yml`（原 portal-logs.yml 并入）；collector 只在目录含 observability 时由 `up` 签密钥并显式起。
- 不建议去掉：它就是联调时看日志的面。坚持要去掉就把 `XGENT_APP_CATALOG` 里的 `observability` 删掉再 `up`（种子会重跑，破坏性），并把 `onebox/docker-compose.onebox.yml` 里五段 logging 注掉——否则 daemon 会有 fluentd 重连噪音。
- 它的界面（`/apps/observability/`）**不在一盒镜像里**——那是 App 团队经发布提案上传的产物；一盒里只有服务面。
  看落了什么用它的查询面：`POST /svc/observability/api/t<租户UUID去横线>/_search?type=logs`，**只认用户票**（`xsak_` 会 401），
  票从 `/auth/dev/start` 登进去后在浏览器里拿，或 `"$S" dc run --rm portal-api bun -e '…mintForApp…'`。

### 2.0 `register-app` 还是 `xgent-app-release`？

两条都能把清单送进一盒的门户，**解决的不是同一件事**：

| | `register-app`（本 skill 默认） | release-cli（`xgent-app-release` skill） |
| --- | --- | --- |
| 到达方式 | 一条命令直写库，幂等 | `xrel_` 令牌 → `POST /api/market/release/:key` → **发布提案** |
| 首次接入 | 立即生效 | **治理档 ⇒ 落 pending**，要自己在 `/console/releases` 批 |
| SA 密钥 | manifest 里那个已知明文，改都不用改 | manifest **拒收**明文（`mode:"prod"`），随机签发且 `issuedSecret` **只回显给审批人** |
| `exchangeInitiatorSecret` | 写进已安装实例（首个租户装完要重跑一次） | manifest 拒收；批准时平台生成并保管、换版注入 `<PREFIX>_APP_SECRET`（要钉值就写进 compose.env，apply 时接管） |
| **后端镜像 tag** | 不参与（一盒的容器归 compose） | 提交 `deployDescriptor.image`、也落库，**但一盒里没人消费**（见下） |
| 适合 | 天天改 manifest 调试 —— **默认走这条** | 排练生产发版链：定级会不会被打回治理档、审批屏长什么样、`requiredEnv` 缺值会不会被拒 |

release-cli 在一盒里是**通的**（`/api/market/release/*` 就在 portal-api 里，走 `xrel_` bearer、
刻意不在 `/api/console/*` 下）：用 `rockie@xgent.ai` 登控制台签一个 `xrel_` 令牌 → 提交 → 自己批。
这是**唯一**能在本地把生产那条自助发版链完整演一遍的地方，值得在正式发版前跑一次。

⚠️ **`deployDescriptor.image` 在一盒里换不了版。** 生产上「提交新 image tag」就是换后端的完整
路径（自动档、立即生效 → 排部署任务 → deploy-controller 拉镜像换容器）；一盒里这条链**断在
第三步**：`ensureDeploymentQueued` 在 `isOneBox()` 直接返回不排 job（CR-4 #8），deploy-controller
本身也在 `onebox-extras` profile 后面、启动即 `refuseOnOneBox`。tag 会如实落进 listing、控制台也
看得到，但**没有任何进程去拉它**——一盒里跑的容器永远是 compose 起的那个。换后端版本请改
`compose.env` 的 `APP_IMAGE` 再 `up -d`。
> 也就是说 release-cli 在一盒里能验的是**治理定级**（这次改动会不会被判自动档 / 要不要审批），
> 验不了**换版效果**。老镜像还会给你一条永远停在 `deploying` 的假行，别照着它去查部署链。

> 密钥那条有干净解法，不必去审批屏抄：portal-api 也读 `compose.env`，把
> `<PREFIX>_SA_CLIENT_SECRET`（和声明了 `exchangeTargets` 时的 `<PREFIX>_APP_SECRET`）
> 写在里面，apply 就用你钉的这个值接管并保管，而不是随机签发（EXCHANGE-SECRET-HOLD：
> 接管后值落在平台侧、换版时注入容器，compose.env 里那行之后可删）。`<PREFIX>` = listingKey 大写、连字符换下划线
> （`omni-parser` → `OMNI_PARSER`）。`_APP_SECRET` 两侧不一致的症状是跨应用交换在 `/oauth/token` 401。

### 2.1 再拉一个别的 App 进来（依赖多模态解析、知识库这类）

一条命令：

```bash
"$S" add omni-parser        # 或 knowledge / task-gateway / …
```

它按 manifest 把这个 App 装起来：拉镜像 → 注册清单+服务账号（**非破坏性、幂等**，不是那个会
truncate 的 `db:seed:onebox`）→ 建 `xgent-<key>` 库 → **生成** compose 片段 → 起容器 → 冒烟。

**清单声明了 `dependencies` 的**（比如 workflow 依赖 files 和 task-gateway）：注册时门户要求每个依赖在**这台一盒里**
已经有清单，否则拒绝（`DEPENDENCY_UNAVAILABLE`），跟依赖装没装无关。`add` 会在拉镜像之前先查一遍，把缺的依赖一次列全，然后停下。
这时先逐个 `"$S" add <依赖>`，再重跑原来那条 `add`。之后在「应用管理 → 应用市场」里安装这个 App，依赖会一起装上、自动授予，不用一个个去装。
依赖的清单要是没写 `deployDescriptor.image`，`add` 它时带上 `--image <REGISTRY>/<ONEBOX_PROJECT>/<key>:<tag>`。

**compose 片段是从 manifest 生成的，不是每个 App 在 skill 里预置一份 YAML。** 那样每接一个新
App 都要改一次 skill，等于把问题换了个地方。manifest 里已经有全部所需：

| compose 里的 | 来自 manifest |
| --- | --- |
| 镜像（**版本由清单钉住，可复现**；相对名按 `<REGISTRY>/<ONEBOX_PROJECT>/` 补全） | `deployDescriptor.image` |
| 容器内端口 / 健康路径 | `deployDescriptor.port` / `.healthPath` |
| 网络别名 `<key>-server`、库名 `xgent-<key>` | `listingKey`（约定） |
| 库连接串 `<PREFIX>_DATABASE_URL`（与生产同一个约定名），另带旧名 `<PREFIX>_PG_DSN` / `DATABASE_URL`，都指向 `xgent-<key>` | `listingKey`（约定） |
| 自省身份 | `serviceAccount.clientId` |
| 还要补哪些 env（值归平台；`add` 会点名片段与 `compose.env` 里都没有的那几个） | `requiredEnv` |

**密钥不从清单来**：一盒现生成，**同时**写进本地门户 DB 与容器 env —— `add` 会在注册前把
密钥注进临时清单（用完即删）。所以清单里有没有明文密钥都无所谓，这也正是 release-cli
提交的那份的样子（prod 模式本来就拒收密钥），而平台目录里那份按红线**只有 `clientId`**。

两把，按需生成：

| 密钥 | 什么时候有 | 门户侧从哪读 | 容器侧 env |
| --- | --- | --- | --- |
| 服务账号密钥 | 总是 | 注入清单的 `serviceAccount.secret` | `<PREFIX>_SA_CLIENT_SECRET` |
| App Secret | 清单声明了 `exchangeTargets` 时 | 注入清单的 `exchangeInitiatorSecret` | `<PREFIX>_APP_SECRET` |

⚠️ 第二把只有**跨应用交换的发起方**需要。漏了它的症状很难查：交换在**发起方**上 401，
看起来像被调方的问题。另外发起方密钥的哈希只随 register 布进**当时已装**的行 —— 所以第一个
租户的顺序是「`add` → 浏览器里勾选安装 → 再跑一次 `add`」（幂等）；之后装的租户会随安装
自动补布（EXCHANGE-SECRET-HOLD 的安装路径布线）。

**清单从哪来**（优先级从高到低）：

```bash
"$S" add <key> --manifest <文件>       # ① 你手上有一份，最高优先
"$S" add <key> --from <目录门户地址>    # ② 从 App 清单目录拉
"$S" add <key>                        # ③ 配了 MANIFEST_STORE 走②，否则用一盒镜像自带的样例清单
```

**推荐把目录地址配进 `.xgent-registry.env`**，之后 `add` 任意 App 都不用带 `--from`：

```
MANIFEST_STORE=<目录门户地址>          # 地址与令牌找门户团队要
MANIFEST_STORE_READ_TOKEN=xcat_…      # 只读令牌：只能读公开清单，读不到任何密钥
```

目录是 App 发版时顺带投递的一份**只读投影**（「这个 App 最近一次说自己是什么」），所以：

- 它能拉到的 App **不限于镜像里预置的那四份样例** —— 这正是配它的理由；
- 它可能比某个门户**已生效**的版本新（那台还没批），也可能旧（那个 App 还没发过版）。
  本地联调用足够；**别拿它当权威**。
- 取不到时**四种情形各给一句人话 + 两条出路**（`--manifest` 或镜像自带样例），不是堆栈：
  `404` = 那个门户没开目录能力或地址不对 · `401` = 只读令牌失效 ·
  `data:null` = 目录里还没有这个 key（那个 App 还没发过版？）· 连不上 = 检查地址与网络。

**镜像不用你操心**：配合调试用的平台侧 App 一律以**稳定版**发到与一盒**同一个项目**下，同一把
puller key 就能拉。要换版才加 `--image <ref>`。有些镜像只发了 amd64，在 Apple Silicon 上直接 pull 会报
`no matching manifest`。这种情况 `add` 会自动改拉 amd64 那一份（仿真运行，冷启动慢），并把
`<PREFIX>_PLATFORM=linux/amd64` 写进 `compose.env`。

> **你要花时间排查的只有你自己那个 App。** 别人的 App 在这里是稳定件——起不来先 `"$S" doctor`，
> 还不行报给门户团队，别自己去调它。

生成的片段落在 `portal-onebox/generated/<key>.yml`，**别手改**（重跑 `add` 会覆盖）；要加东西就
再叠一层自己的文件（见 §2.2）。少数 App 有清单描述不了的自带 infra（知识库要一个 Chroma 边车），
那种由 skill 的 `services/<key>.extra.yml` 补——**只有例外才需要文件**。

跑完还差**两步，都不是运维活**：

1. 浏览器里勾一下：`rockie@xgent.ai` 登控制台 → 租户 → 可用应用 → 勾上它保存。
   （service 型 App 的勾选**就是**安装；不装的话跨应用交换拿不到它的 scope。）
2. 在**你自己**的 manifest 里两处一起加，再 `"$S" up`：

```jsonc
"scopes":          ["…", "omni_parser.read", "omni_parser.parse"],
"exchangeTargets": ["omni-parser"]        // 没有它，声明对方 namespace 的 scope 会被 register-app 拒
```

首次跨应用调用浏览器会多弹一次交换授权页，是正常的。

> **为什么建库/迁移在一盒要 `add` 代劳，生产不用。** 生产上这两件各有其主：**建库**归批准时的
> 平台供给（按约定名 `<PREFIX>_DATABASE_URL` 建库并注入，不在部署链里），**迁移**归部署链——`deployDescriptor.migrateArgs` 让 deploy-controller 在换
> 容器**之前**跑一个一次性容器（同镜像、同 env、只换 argv），失败就让这次部署失败而**旧容器
> 继续服务**，不会变成换完之后的崩溃循环。一盒**两样都没有**（没有 controller，`migrateArgs`
> 一次也不会跑），所以 `add` 替你建库，迁移靠镜像自己启动时迁——**这也意味着一盒里验不出
> `migrateArgs` 写得对不对**，那条只能等真部署。

### 2.2 改 compose 的规矩（叠一层，别动 init 铺出来的）

`"$S" dc` 已经替你拼好了：`--env-file compose.env` + `-f docker-compose.yml` +
`-f onebox/docker-compose.onebox.yml`（填了 App 那两行再加 app-dev / app-frontend）+ 一串
`--profile`。`"$S" chain` 把这串打出来，想自己敲 compose 时抄它。

**三条规矩：**

1. **改配置值 → `compose.env` 末尾追加**，不要回上面改。端口、项目名、镜像 tag、你自己那些
   env 全在这里；env-file 是**后定义者胜**，追加就是覆盖。
2. **改拓扑（加服务、换 image、改别名） → 写一个新文件叠在最后**：
   `XGENT_ONEBOX_HOME=portal-onebox "$S" dc -f portal-onebox/<你的>.yml up -d <服务名>`。
   `-f` 出现在子命令前就仍是全局选项，位置没问题。
3. **别改 `portal-onebox/` 里 init 铺出来的那几个 yml** —— 它们是从镜像里取出来的，
   `upgrade` 会同步这些平台资产，`init --force` 也会覆盖；不要在平台原文件里维护自己的改动。你的东西永远是**新文件**。

**合并规则（实测 `docker compose config`，三条都不直觉）：**

| 字段 | 规则 |
| --- | --- |
| `command` / `entrypoint` | **整体替换**（后者赢，不拼接） |
| `environment`、`labels` 这类映射 | **逐键合并**，同名键后者赢 |
| `expose` / `ports` / `networks.*.aliases` 这类序列 | **追加**——⚠️ 别指望用 override "改" 一个别名，你会**同时得到两个**；要换就在自己的文件里定义整个服务 |
| `volumes` | 按**容器内挂载点**合并：`v3:/data` 会顶掉上一层的 `v1:/data`，而 `v2:/other` 原样保留 |

改完先看合并结果再起（不 up 也能看）：

```bash
XGENT_ONEBOX_HOME=portal-onebox "$S" dc -f portal-onebox/<你的>.yml config | less
```

**加一个新服务，必备四件**（少哪件的症状都在排查表里）：`image` · `platform`（arm64 机器上跑
amd64 镜像要 `linux/amd64`）· 网络别名 `<key>-server`（反代靠它找上游）· `expose: ["8080"]`
（容器内统一 8080）。

## 3. 冒烟

```bash
"$SKILL_DIR/scripts/onebox.sh" smoke
```

探 `GET /health`（门户；注意**不是** `/api/health`，那个 404）和每个 `GET /svc/<key>/health`。
全绿再开浏览器（地址以 `init` 回显的为准，改过端口就不是 `http://localhost`）。

**smoke 的 HTTP 检查不能覆盖信封内的 db 判据。** 外部 App 支持平铺与 Envelope：HTTP 非 2xx 不通过；2xx 顶层含 `ok` 时，必须 `ok === true && data.db === true`；否则按 2xx 判健康。发布前按 `portal-external-app` skill 的 `references/health-contract.md` 运行其 `scripts/verify-health.ts`（直连与 `/svc` 两条路径），不要自行增加“禁止信封”或字段命名限制。

**dev 登录的入口**：登录页密码表单下方的「本地开发账号」按钮（`DEV_MOCK_OAUTH=true` 才出现），
或直接打开 `/auth/dev/start` 进 mock IdP 选账号 → 选 `rockie@xgent.ai`（演示租户 admin + 平台管理员）。
**按钮没出现**：先 `curl <地址>/auth/providers` 看 `dev` 字段在不在——在就走 `/auth/dev/start`，
那说明这版镜像的前端与 API 对不齐，按 §6 更新缓存后运行 `upgrade` → `up`。

种子只种两个账号，另一个是 `liming@xgent.ai`（普通成员）——**ACL 成员基线没到位的问题只在非管理员身上现形**，
验收要用它再走一遍，管理员那边永远是绿的。

- **micro**：应用市场安装 →（一盒**不排**部署任务：没有部署控制器，排了也没人取，所以状态如实停在
  `not_deployed`。老镜像会给你一条**永远 `deploying`** 的行 —— 两种都不挡使用，别去查部署链）→ 应用中心打开 → **首次打开先出「授权并打开」同意屏**（正常行为，自动化走查要把这一步算进去）→
  iframe 加载 `/apps/<key>/` → `sdk.ready()` 握手 → `sdk.callService()` 通 → 首次跨应用读数据再弹一次交换授权页。
- **service**：无 UI，走 curl —— `/health` 判活 · 缺/错 token 应 401/403（四道闸）· 正确 TDT 应 200 信封。

> 你的后端要实现的运行时契约（验 TDT + 四道闸、解包自省信封 `claims = body.data ?? body`、
> 按 `claims.tenant_id` 隔离、`/health`、容器内统一监听 **8080**）不在本 skill 范围内——
> 一盒不替你实现。完整说明在 init 之后的 `portal-onebox/app-devkit/README.md`。

## 4. 出问题了

```bash
"$S" doctor
```

**先跑它，别翻文档。** 它把排查表里能自动判的都判一遍（镜像版本、命名卷属主、内联 key 撞 map、
架构不匹配、缺库、目录里有 key 但容器没起、每条 `/svc` 的 404/502/000），每条不过的下面直接给
一行可粘的修法。判不了的再对着 **[references/troubleshooting.md](references/troubleshooting.md)**
按「你看到什么」对号入座。

> **「照文档做，行为却不符」先看版本。** `status` / `smoke` 里 `/health` 那行会回一个 `version`：
> `init` 把镜像的 tag + ID + 构建日期写进了 `compose.env` 的 `APP_VERSION`。报 `dev` 说明这份 compose.env
> 是老 `init` 铺的（或你自己改过）。一盒镜像比文档旧一天，就足以让「注册后自动授予租户」这类行为整个不存在，
> 而现场没有任何报错 —— 真实案例（CR-4）。按 §6 更新缓存并运行 `upgrade` → `up`，同步新镜像与资产再判。
那张表按「你看到什么」编排。最容易白白浪费半天的两条先放这儿：

> **看到 `unhealthy` 先读那条探针，再决定要不要查。**
> `docker inspect <容器> --format '{{json .Config.Healthcheck.Test}}'`，三种情形：
> · `curl -fsS …/health`（`portal-api` / `*-server`）⇒ **假红**，且说明这盒的 compose 是**旧镜像**
> 里那份：精简镜像没装 `curl`，`docker inspect .State.Health` 里是清一色 `curl: not found`。
> · `wget … http://127.0.0.1:80/`（`reverse-proxy`）⇒ 只要 `XGENT_SITE_ADDRESS` 是主机名就**必然**
> 假红：Caddy 把它 308 到 `https://127.0.0.1/`，对一个 IP 建 TLS 没有 SNI，回 `SSL alert number 80`。
> · 探针已是 `bun -e fetch(...)` / `curl -fsS -o /dev/null http://127.0.0.1:80/`（新镜像的两条）
> 却仍 `unhealthy` ⇒ **真红**，去看 `"$S" dc logs <服务>`。
> 判活以 `"$S" smoke` 的宿主侧探测为准；pg/redis/RustFS 的 healthy 检查对应服务。Kafka 看 `doctor`，其 TCP healthy 不代表 App 凭据与 ACL 已可用。

> **`COMPOSE_PROJECT_NAME` 固定是 `xgent-onebox`，别靠改名来「并存」两套。** 同机两套 compose
> 项目名相同，compose 会认为它们是同一项目——容器互相接管、命名卷共享，症状是「我起了一盒，
> 结果把另一套的容器停了」。一台机器本来就只该有一套一盒（见文首「一套就够」）：init 检测到
> 本机已有别的一盒会停下，第二个 App 要联调就接进现有那套。

## 5. 后端想跑在宿主上（保留热重载）

`app-backend` 默认是**镜像**，靠网络别名 `<key>-server` 被反代解析到；宿主上 `bun dev` 起的进程在容器网里
没有这个名字，而每改一行就重建镜像等于没有热重载。绕法是让一个转发容器顶住那个别名：

```yaml
# portal-onebox/host-backend.yml —— 叠在 app-dev.yml 之后
services:
  app-backend:
    image: alpine/socat
    command: TCP-LISTEN:8080,fork,reuseaddr TCP:host.docker.internal:<你 dev server 的端口>
    extra_hosts: ["host.docker.internal:host-gateway"]
```

然后 `XGENT_ONEBOX_HOME=portal-onebox "$S" dc -f portal-onebox/host-backend.yml up -d app-backend`。
此时 `APP_IMAGE` 不再被用到（compose 仍要求它有值，随便填一个）；`APP_KEY` 照旧——别名还是靠它。

**宿主进程的 DB / 缓存 / 对象存储也用一盒的，别再单起。** postgres / redis / RustFS 都发布了
宿主端口，而 init 挑端口时避开了本机已占用的口——所以它们和你本地 dev 栈的
5432 / 6379 / 9000-9001 不冲突（典型落位 15432 / 16379 / 19000-19001；以 `compose.env` 末尾的
`POSTGRES_PORT` / `REDIS_PORT` / `MINIO_PORT` / `MINIO_CONSOLE_PORT` 为准，`"$S" env` 也回显）。
调试你自己的 App 时——宿主上的 dev server、另一个 repo 的服务、临时排查脚本——直接指过来：

| 服务 | 宿主侧地址 | 凭据 / 说明 |
| --- | --- | --- |
| Postgres | `postgres://postgres:postgres@localhost:<POSTGRES_PORT>/xgent-<你的key>` | 库不存在先建（§2 3b 那条 CREATE DATABASE） |
| Redis | `localhost:<REDIS_PORT>` | 无密码 |
| RustFS (S3) | `http://localhost:<MINIO_PORT>` | `minioadmin` / `minioadmin`；控制台在 `http://localhost:<MINIO_CONSOLE_PORT>/rustfs/console/`，桶自己建（建议 `xgent-<你的key>`） |

这样宿主进程和容器里的 App 看到的是同一份库、同一个桶。注意上面是**宿主侧**地址——
容器网里地址不变（`postgres:5432` / `redis:6379` / `minio:9000`）；`minio` 是 RustFS 的兼容别名，`MINIO_*` 配置继续使用。

## 6. 初始化、升级与重置

三条路径各有用途：

| 目的 | 命令 | 配置与数据 |
| --- | --- | --- |
| 首次初始化 | `"$S" init --key <listingKey>` → 编辑 App 配置 → `"$S" up` | 新建资产、密钥与端口配置 |
| 已有一盒升级 | `"$S" upgrade [--image <ref>]` → `"$S" up` | 保留 compose.env、generated/、backup/、端口与数据卷 |
| 主动清空联调数据重置 | `"$S" dc down -v` → `"$S" init --force --key <listingKey>` → `"$S" up` | 删除当前 compose 的命名卷并重新生成配置；不可当升级使用 |

### 升级：换资产，保留本机配置与对象

```bash
S="$SKILL_DIR/scripts/onebox.sh"
"$S" upgrade --image <registry>/<项目>/one-box:v<版本>-<sha>
"$S" up
```

不指定 `--image` 时使用当前 `XGENT_IMAGE`；显式换 runtime 后会按同仓同 tag 推导 proxy，
需要不同反代镜像可加 `--proxy-image <ref>`。`upgrade` 要求已有 `compose.env`，
重新从镜像取出部署资产，**不重建配置、不重挑端口、不删卷**。只有镜像或版本变化时，
才在文件末尾追加 `XGENT_IMAGE` / `XGENT_PROXY_IMAGE` / `APP_VERSION`；原有密钥、自定义行和 `add` 写入的键保留。
Kafka 另按需补缺 `KAFKA_CLUSTER_ID` / `KAFKA_ADMIN_PASSWORD` / `KAFKA_PORT`：新 ID 与密码随机生成，
新端口按 9092/19092/29092 挑选，已有的三键一个不改。密码写入不回显，`compose.env` 保持 600。
`generated/` 和 `backup/` 原样保留。两个目标镜像本地都在时跳过拉取，支持本地构建或离线 `docker load`。
沿用可变 `latest` 要先更新缓存：`"$S" pull` → `"$S" upgrade` → `"$S" up`；显式指定新的不可变 tag
或已载入的本地新 tag，可直接 `upgrade --image <ref>`。单独 `pull` 不更新资产，不能代替 `upgrade`。

升级也保留 PostgreSQL 的**现有数据大版本与挂载布局**。脚本在启动数据库前只读检查卷内
`PG_VERSION`，已有 18 的一盒不会因新模板默认 16 而被降级；旧布局挂 `/var/lib/postgresql/data`，
18 起的版本目录布局挂其父目录 `/var/lib/postgresql`。未知、冲突或非空但无法识别的布局会中止，
不冒险初始化另一份空库。此时保留卷与原镜像，按提示核对；**不要删卷解决版本错误**。
真正升级 PostgreSQL 大版本需要另行备份并安排 pg_upgrade 或导出/恢复，不随门户换版自动执行。

`up` 先检查资产，再通过共用切换脚本把存量 MinIO 复制到全新的 RustFS 卷；旧卷保留，
桶列表、对象清单和正文摘要一致才提交。正式 RustFS 只在脚本成功后启动，并等待 `/health`。
`add` 与建桶入口同样经过这道切换门。**`up` 仍会重种门户库**（租户、用户、清单、安装与 UUID），
这是一盒原有契约；App 自己的数据库不被重种，对象存储数据由切换校验保留。

备份在一盒目录的 `backup/`，包含旧镜像归档、每次切换清单及 `rollback-minio.sh`。
失败时先读退出提示：提交前失败恢复旧服务；已提交但新服务不健康时保留 `rustfs-data`，
按日志排错后重跑 `up`。**不要删除权威新卷。** 回退入口会先停掉正式 RustFS，重新核对当前数据：
有切换后写入则拒绝回退并保留新卷；此时需在副本上验证 MinIO 兼容与全部对象，再由人工安排恢复，
不能直接把服务指回旧卷。运行回退入口时通过 `--env-file <一盒目录>/compose.env` 提供原凭据，勿写入命令行。

Kafka 数据也随升级保留。`KAFKA_CLUSTER_ID` 与 `kafka-data` 绑定，备份需同时保留配置与卷；
已有数据时不能重生成 ID、换空卷或改 `KAFKA_DATA_DIR` 来消除拒启。改变目录不会迁移消息。
单节点没有高可用，重启期间不可用；需要回退平台代码时保留 Kafka 卷与原配置。

### 停栈与重置

`"$S" dc down` 只停容器、保留卷；下次 `up` 仍会重种门户库。只想重置门户数据也可重跑
`db:seed:onebox`，随后重跑 `register-app`、重载 Caddy；浏览器需要重新登录。

`down -v` 是主动删除当前 compose 命名卷的操作，pg、RustFS、Kafka、apps、caddy 数据会丢失。
先备份要保留的 App 数据与对象，再按上表重置；退役 MinIO 卷和 `backup/` 不在新 compose 删除范围内，
保留供核对，不因升级自动删除。命名卷属主故障按 [排查表 §7](references/troubleshooting.md#7-命名卷属主不对eacces-一族)
处理，升级本身不要求清空所有数据。

## 7. 一盒不是门户，别拿它当门户

- 它只带上面四个基础服务，其余门户内置 App 的代码**不在镜像里**——
  `bun --filter @xgent/<别的>-server …` 报 `no packages matched the filter` 是刻意的。
- `bootstrap:prod` 与部署控制器**启动即拒**并打印原因；完整的 `db:seed`（十几个 App 的种子链）也必失败，
  一盒只能用 `db:seed:onebox`。
- 它是 dev 联调套件：mock OAuth 常开、服务账号密钥是已知明文。**不要用于生产，也不要暴露到公网。**
- `compose.env` 权限 600 只保护宿主文件；一盒仍把整份配置注入容器，是可信 App 之间的本地联调模型，
  不构成对恶意 App 的凭据隔离。不要把 Kafka 管理密码当作 App 的连接凭据。
- 不装 ffmpeg：`PREVIEW_MEDIA_CONVERTER_URL` 必须留空，视频海报/网格缩略图退化成图标。
- 文件管理界面**不渲染预览**（镜像构建时关掉了前端的格式渲染器），打开任何文件都是下载卡；
  服务端的预览描述符 API 不变，你的 App 自带 `@xgent/file-preview` 时照常渲染。
