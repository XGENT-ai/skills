# 外部 App 本地联调指南（门户一盒 · one-box）

> **App 无关**的本地联调指南。把 [`docs/知识库后台接入与本地联调指南.md`](./知识库后台接入与本地联调指南.md)
> 的知识库个案抽象为通用流程:任何**不在门户 monorepo 里**的独立 App / 服务,只要拿到①门户镜像
> ②一份 `app.manifest.json`,就能 `docker compose up` 起一个**真实门户**、把自己的后端/前端接上、跑通
> 全链路联调——**不 clone 门户、不改门户代码、不重建门户镜像**。
>
> 总设计:[`goal/ONE-BOX.md`](../goal/ONE-BOX.md)。运行时契约:[`docs/SSO与App开发指引.md`](./SSO与App开发指引.md)。
> 操作手册(怎么把栈跑起来):[`deploy/app-devkit/README.md`](../deploy/app-devkit/README.md)。
> 两个具体 example 的 manifest:[`deploy/app-devkit/manifests/`](../deploy/app-devkit/manifests/)。

## 0. 一页速览:谁做什么

| 谁 | 交付 |
| --- | --- |
| **平台团队** | 门户镜像两个(portal runtime + caddy proxy,私有 registry 拉取或 `docker save` tar)+ `deploy/` 目录(含 `app-devkit/`)。镜像里**不**烘焙任何具体 App。 |
| **你(外部 App 团队)** | ①一份 `app.manifest.json`(门户契约)②一份 `compose.env`(本机接线)③你的后端镜像(+ micro 才需的前端 dist)。 |

**门户镜像拉取地址(私有 registry,当前 `arm64`;需先 `docker login`):**

```
crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-one-box:1.0.0   # portal runtime → XGENT_IMAGE
crpi-j73qdd1ocro8j5ps.cn-guangzhou.personal.cr.aliyuncs.com/xgent/ai-portal-proxy:1.0.0      # caddy proxy   → XGENT_PROXY_IMAGE
```

接入 = 写 manifest + 填 compose.env + 跑一个通用 `register-app`。**新 App 零改门户码、零重建门户镜像。**

## 1. 两类后端 App

| | `micro`(有前端,iframe 嵌入) | `service`(无前端,仅 API) |
| --- | --- | --- |
| 例 | 知识库 knowledge | 多模态解析器 omni-parser |
| 前端 | dist 同源挂到 `/apps/<key>/` | 无 |
| 用户可见性 | 应用市场/应用中心露卡、可打开 | **不**露卡、**不**可打开(平台控制台清单管理仍可治理) |
| 被调用 | 自身后端 + 可作令牌交换来源 | 仅作令牌交换/直调的**被调用方** |

`service` 是本套件**新增**的第四种 App 类型(`micro` / `link` / `native` / `service`),用类型本身表达
「无前端 headless 后端」,顺手解决可见性(应用中心/市场按类型过滤),无需额外 `hidden` 列。

## 2. 运行时契约(你的后端要实现,与生产同一套)

照 `apps/spms-server/` 实现,本套件**不**替你实现:

1. **验 TDT:自省 + 四道闸**。每个请求用 `Authorization: Bearer <TDT>` 调门户
   `POST /api/tokens/introspect`(你的后端以 `Authorization: Basic base64(clientId:secret)` 的**服务账号**身份调用),
   校验四道闸:`aud == <你的 listingKey>` · scope 命中 · `requireAdmin`(写操作看自省 role) · `requirePerm`(ACL)。
2. **解包自省信封**:`claims = body.data ?? body`。有效 TDT 必须被**接受**;若你对一个门户自省为 `active:true`
   的 TDT 返 `INVALID_TOKEN`,就是没解包 `.data`(knowledge 1.0.0 同款坑)。
3. **租户隔离**:所有数据按 `claims.tenant_id` 隔离。顶栏切租户后不得看到上一租户数据。
4. **令牌交换(可选)**:若你要代表用户调另一个 App(如 `knowledge→files`),用你的 TDT + 你的 App Secret 走
   `/oauth/token`(grant_type=token-exchange)换目标 aud 的 TDT。前提是 manifest 声明了 `exchangeTargets`(见 §3)。
5. **`/health`**:`GET /health → {"service":"<key>","db":"ok",...}`,容器内监听**统一端口 8080**(反代按
   `/svc/<key>/* → <key>-server:8080` 路由)。
6. **micro 才需的临时前端**:`bun run build` 等产出 `dist/`(含 `index.html`),套件挂到 `/apps/<key>/`。

## 3. 单一事实源:`app.manifest.json`

在**你自己 repo** 写一份声明式清单,既是交付契约,也是 `register-app` 的输入。字段直接映射到门户的市场
listing + 服务账号 + 套件接线。完整字段见 [`goal/ONE-BOX.md §3.1`](../goal/ONE-BOX.md) 与两个样例:

- micro:[`deploy/app-devkit/manifests/knowledge.manifest.json`](../deploy/app-devkit/manifests/knowledge.manifest.json)
- service:[`deploy/app-devkit/manifests/omni-parser.manifest.json`](../deploy/app-devkit/manifests/omni-parser.manifest.json)

**Scope 声明规则(写时强校验,ONE-BOX §3.5)**——一个 listing 可声明的 scope =
**平台基础 scope**(`userinfo`/`directory`/`notification`/`settings`/`audit`/`scheduler`/`content` 等 App 无关那批)
**∪ 本 namespace**(`<listingKey>` 及其下划线变体,如 `omni-parser`→`omni_parser`)
**∪ 已声明的 `exchangeTargets` 的 namespace**。

> 即:omni-parser 声明 `files.read` 但 `exchangeTargets` 不含 files → **被拒**;
> knowledge 声明 `files.read` 且 `exchangeTargets:["files"]` → **放行**。namespace 隔离真正成立。

**职责切分**:`app.manifest.json` = 门户契约(跟着 App 走,入你的 repo);**本机接线**(镜像 tag、dist 绝对路径)
放 `compose.env`(`APP_KEY` / `APP_IMAGE` / `APP_FRONTEND_DIST`)。

## 4. 通用注册:`register-app`

读 manifest,**幂等**完成注册(详见 [`apps/api/scripts/register-app.ts`](../apps/api/scripts/register-app.ts)):

1. **listing**:按 `listingKey` upsert(存在则更新,不存在则新建)→ 发布。复用门户写时校验(scope namespace / ACL / 依赖无环)。
2. **服务账号**:DB 直写一个 `token.introspect` 服务账号,secret = manifest 的**已知 dev 明文**(便于你 repo 在 env 里固定,
   可重复 `up`)。幂等密钥策略:同 secret 不动、不同则轮换、绝不静默留旧值。
3. **交换来源密钥**:若你是交换来源(如 knowledge),把 `exchangeInitiatorSecret` 写进**已安装**实例的 App Secret。
4. **`/svc` 放行**:往 `caddy-svc-allow` 卷写 `<key>.map`,反代启动即 `import`——**无需改 Caddyfile、无需重建镜像**。

> 仅 dev:已知明文密钥 + `register-app` 在 `NODE_ENV=production` 下**拒跑**。生产走 `bootstrap:prod` + 控制台随机签发。

## 5. 本地联调步骤

完整命令(含 micro / service 差异、App 自带基础设施如 chroma)见
**[`deploy/app-devkit/README.md`](../deploy/app-devkit/README.md)**。摘要:

```bash
# 0) 取门户镜像:docker login + docker pull 上面两个 registry 地址(或离线 docker load tar)+ 你的后端镜像
# 1) cp compose.env.example → compose.env;append app-devkit/compose.env.app.example;填 APP_KEY/APP_IMAGE/(micro)APP_FRONTEND_DIST
# 2) 起基础设施:--profile local-infra up -d postgres redis minio
# 3) 门户迁移 + 基线 seed:run --rm portal-api bun run db:migrate / db:seed
# 4) ★ 通用注册(把 manifests/ 临时挂进 /devkit):
#    run --rm -v "$(pwd)/deploy/app-devkit/manifests:/devkit:ro" portal-api bun run register-app /devkit/<你的>.manifest.json
# 5) 起门户三件套 + 你的 app-backend(别名 <key>-server:8080):
#    -f docker-compose.yml -f app-devkit/docker-compose.app-dev.yml [micro 加 -f .../app-frontend.yml]
#    --profile local-infra --profile app-external up -d
# 6) 你自己的库迁移 / 每租户 bootstrap(命令以你镜像为准)
# 7) 冒烟(§6)
```

## 6. 冒烟(验收主路径)

打开 `http://localhost/` → dev 登录 → 选 `rockie`(晨光教育 admin + 平台管理员)。

**A. micro(knowledge)**:应用市场安装「知识库」(连带补装 files)→ 应用中心打开 → iframe 加载 `/apps/knowledge/`,
`sdk.ready()` 握手 → `sdk.callService("knowledge","/api/...")` 通 → 读文件首次弹交换授权页 → 切租户看不到上一租户数据。

**B. service(omni-parser,无 UI 走 curl)**:
```bash
curl http://localhost/svc/omni-parser/health           # {"service":"omni-parser","db":"ok",...}
curl -X POST http://localhost/svc/omni-parser/v1/parse  # 缺/错 token → 401/403(四道闸)
# 拿一个 aud=omni-parser 的 TDT(经 knowledge 交换,或 dev 自助 /api/tokens/authorize→/oauth/token)→ 正确 → 200 信封
```
service 类型**不**在应用市场/应用中心露卡,但平台控制台「清单管理」可见可治理,且能作依赖被动补装。

## 7. 镜像交付(不发 docker hub)

平台团队用 [`scripts/pack-onebox.sh`](../scripts/pack-onebox.sh) 把门户镜像 + `deploy/` 打成一个交付包:

```bash
bun run build:apps                       # 先构建前端(可选,镜像已含)
scripts/pack-onebox.sh ./dist-onebox     # docker save 门户+proxy → tar + 复制 deploy/(含 app-devkit/)
```

对方在一台**只装 Docker、无门户源码**的机器上 `docker load` 两个 tar,按本指南 + 自己的 manifest 跑通即可。

## 排查速查

见 [`deploy/app-devkit/README.md` 的排查表](../deploy/app-devkit/README.md#排查速查) 与 [`goal/ONE-BOX.md §8`](../goal/ONE-BOX.md)。
最常见:`/svc/<key>` 404 = `.map` 没写成 / 反代先于 register-app 起;有效 TDT 被判 `INVALID_TOKEN` = 没解包自省信封;
`register-app` 报 scope `VALIDATION_FAILED` = 声明了别人的 scope 但没列进 `exchangeTargets`。
