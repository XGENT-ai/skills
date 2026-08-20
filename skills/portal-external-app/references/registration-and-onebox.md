# 注册布线与一盒（one-box）本地联调

> 提炼自门户仓库 `docs/外部App本地联调指南-one-box.md`、`docs/SSO与App开发指引.md` §15.3（2026-07）（门户仓文件，App 自己的 repo 里没有；本文件已自包含，不必去找）。冲突时以门户仓库为准。

## 1. `app.manifest.json`（单一事实源，放你自己的 repo）

既是交付契约也是注册脚本的输入。字段：

- 身份/展示：`listingKey`（= TDT aud = 服务账号/容器命名基名）、`name`、`version`、`cat`、`tagline`、`desc`、`icon`、`color`
- 形态：`type`（`micro` 有前端 / `service` 无前端）、`embedUrl`（micro：`/apps/<key>/`）、`embedCsp.connectSrc`
- 权限：`scopes` + `scopeLabels`（三语同意页文案）、`aclManifest`（纯 scope 鉴权可 `null`）、`navItems`（micro）
- 关系：`dependencies`、`exchangeTargets`、`serviceBaseUrl`（`/svc/<listingKey>`）
- 部署/运维：`requiredEnv`（部署所需 env 的**键名**清单，值永远由平台配；键集合一变提案转入「发布审核」）、`deployDescriptor`（`image` 之外的一切属治理档）
- dev 专用明文：`serviceAccount{clientId,secret}`（`clientId` 约定 `<key>-server`，且**必须是自己的** —— 服务账号身份归平台，声明已归属另一 App 的 clientId 会进人工审、批准也会在生效时被写入闸拒绝）、`exchangeInitiatorSecret`（本应用作为交换发起方时的 App Secret）

**scope 三规则**（写时强校验）：一条清单只能声明——
1. 平台基础 scope（`userinfo.*`/`audit.write`/`notification.*`/`settings.*`/`scheduler.*`/`content.*` 等）；
2. **自己命名空间**：`<listingKey>.` 前缀及连字符→下划线变体（`omni-parser` → `omni_parser.*`）；
3. **已声明 `exchangeTargets` 的目标命名空间**（例：omni-parser 声明 `files.read` 但 exchangeTargets 不含 files → 拒；knowledge 声明 `files.read` 且 `exchangeTargets:["files"]` → 放行）。

越界 → `VALIDATION_FAILED`。App 命名空间 scope 随清单入库，**不用改门户代码**。

## 2. 注册（dev / 一盒）：`register-app`

```bash
bun run register-app <你的>.manifest.json    # 幂等；NODE_ENV=production 拒跑
```

四件事：① 按 listingKey upsert + 发布 listing（复用门户写时校验：scope 命名空间 / ACL 形状 / 依赖无环）；② DB 直写 `token.introspect` 服务账号（secret = manifest 明文；同 secret 不动、不同则轮换）；③ 把 `exchangeInitiatorSecret` 写进**已安装**实例的 App Secret——⚠️ 所以**先在门户安装 App、再重跑一次 register-app**，否则发起交换 401；④ 往 `caddy-svc-allow` 卷写 `<key>.map` 放行 `/svc/<key>`（免改 Caddyfile、免重启反代）。

## 3. 注册（生产）

生产的清单事实源就是对方仓的 `app.manifest.json`：平台在控制台
「应用市场 › 接入新应用」按 key 签发 `xrel_` 令牌（无行先建 draft 占位），对方
`publish --manifest` 提交，平台在「发布审核」批准 = `registerFromManifest` 建全
（listing 上架 + SA + /svc + 已装租户对齐）。**不要**把外部 App 登记进 `LISTING_DEFS` ——
门户只保留平台侧事实（`SA_DEFS` / `EXCHANGE_WIRING` / scope 常量 / Caddy 内联行），
`bootstrap:prod` 对外部 key 只自愈 SA 与部署行。**生产没有 manifest 明文密钥这条路**
（secret 取 env、缺省随机生成、明文只回显一次给审批人）。

⚠️ **secret 一致红线**：平台落库的服务账号 secret 与你镜像 env 里的必须一致。漂移症状 = 自省 401，或「跨应用授权缺失/未开启」（实为 `SECRET_INVALID`）。排查：对你侧 secret 求 sha256 与门户 DB 存的哈希比对。

## 4. 一盒联调流程

分工：平台交两个门户镜像（`ai-portal-one-box` runtime → `XGENT_IMAGE`；`ai-portal-proxy` caddy → `XGENT_PROXY_IMAGE`；私有 registry 或 `docker save` tar）+ `deploy/` 目录（含 app-devkit）。你交 ①manifest ②`compose.env`（本机接线：`APP_KEY`=listingKey、`APP_IMAGE`、micro 才需 `APP_FRONTEND_DIST` 绝对路径）③你的后端镜像（+ micro 的前端 dist）。**不 clone 门户、不改门户代码、不重建门户镜像。**

```bash
# 0) docker login + pull（或 docker load tar）门户两镜像 + 你的镜像
# 1) cp compose.env.example → compose.env，append app-devkit/compose.env.app.example，填 APP_*
# 2) 起基础设施:  --profile local-infra up -d postgres redis minio
# 3) 门户迁移+seed: run --rm portal-api bun run db:migrate / db:seed
# 4) 注册:        run --rm -v "$(pwd)/deploy/app-devkit/manifests:/devkit:ro" \
#                   portal-api bun run register-app /devkit/<你的>.manifest.json
# 5) 起门户三件套 + 你的后端（网络别名 <key>-server:8080）:
#      -f docker-compose.yml -f app-devkit/docker-compose.app-dev.yml [micro 加 app-frontend.yml]
#      --profile local-infra --profile app-external up -d
# 6) 你自己的库迁移：跑你镜像的迁移 argv（生产由门户经 migrateArgs 自动跑，这里手动跑同一条）
#      run --rm --env-file <同一份> <你的镜像> --role migrate
#    每租户 bootstrap（若你的表 FK 自己的 tenants）：门户目前没有钩子，本地手动建行
```

细节（含 App 自带基础设施如向量库 sidecar）见交付包内 `deploy/app-devkit/README.md`。

## 5. 冒烟

打开 `http://localhost/` → dev 登录选 `rockie`（租户 admin + 平台管理员）。

- **micro**：应用市场安装（连带补装依赖）→ 应用中心打开 → iframe 加载 `/apps/<key>/`、`sdk.ready()` 握手 → `sdk.callService("<key>", ...)` 通 → 跨应用读数据首次弹交换授权页 → 切租户看不到上一租户数据。
- **service**（无 UI 走 curl）：

```bash
curl http://localhost/svc/<key>/health           # {"service":"<key>","db":"ok",...}
curl -X POST http://localhost/svc/<key>/v1/...   # 缺/错 token → 401/403（四道闸生效）
# 拿一个 aud=<key> 的 TDT（经交换，或 dev 自助 /api/tokens/authorize → /oauth/token）→ 200 信封
```

## 6. 排查速查

| 症状 | 病根 |
| --- | --- |
| `/svc/<key>` 404 | 白名单 `.map` 没写成，或反代先于 register-app 起（重启反代/重跑注册） |
| `/svc/<key>` 502 | 后端没起 / 没听 8080 / 网络别名 `<key>-server` 没命中 |
| 有效 TDT 被判 `INVALID_TOKEN` | 没解包自省信封（`claims = body.data ?? body`） |
| register-app 报 scope `VALIDATION_FAILED` | 声明了别人的 scope 但没列进 `exchangeTargets` |
| 发起交换 401 | `exchangeInitiatorSecret` 未写进已安装实例（先安装再重跑 register-app）或 secret 漂移 |
| 受门路由 503 | 门户三变量（自省地址/SA id/secret）配置不全 |
