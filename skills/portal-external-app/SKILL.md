---
name: portal-external-app
description: 接入「外部镜像服务类应用」——服务端代码不在本 repo、以 Docker 镜像交付的 App（知识库/多模态解析/异步任务网关这类）。凡任务涉及为外部服务写对接契约或 app.manifest.json、register-app/provisioning 注册布线、/svc 路由与健康检查、外部服务的服务账号与 scope、一盒(one-box)本地联调、或评审外部团队的交付物时，务必先用本 skill——即使用户只说"把 XX 服务接进来"。Use when integrating an externally-built docker-delivered service app into the portal: manifest, register-app, provisioning, /svc wiring, one-box debugging, or reviewing an external team's delivery.
---

# portal-external-app · 外部镜像服务类应用接入

外部镜像 App = 服务端在别的 repo（任意语言/栈）、独立镜像交付、**常驻**（不走 deploy-controller 按需缩零）。运行时契约与内建 App 完全一致，差异只在注册与部署。权威文档随本 skill 附带（`references/` 是门户仓库文档的镜像，文内相对链接原样可解析）：[`SSO与App开发指引.md`](references/docs/SSO与App开发指引.md) **§15**（先通读，下称"指引"）；操作手册 [`外部App本地联调指南-one-box.md`](references/docs/外部App本地联调指南-one-box.md) + [`app-devkit README`](references/deploy/app-devkit/README.md)；契约文档范本 [`knowledge-app-contract.md`](references/docs/knowledge-app-contract.md)；已接入案例 [`omni-parser-integration.md`](references/docs/omni-parser-integration.md)、[`知识库后台接入与本地联调指南.md`](references/docs/知识库后台接入与本地联调指南.md)。

> 本 skill 自包含，可整目录拷到外部团队 repo 使用。文中 `apps/api/...` 等门户 monorepo 路径只在门户仓内存在——外部项目只需按 references 文档实现，门户侧步骤由平台方执行。门户仓内工作时文档以 `docs/` 原件为准（`.claude/skills/sync-portal-skill-refs.sh` 同步副本）。

## 先判形态（决定后面走哪条线）

- 有用户前端？→ `type:"micro"`（前端 dist 同源托管 `/apps/<key>/`，前端开发见 portal-micro-app skill）；无前端 → **`type:"service"`**（无头：不露卡、不可打开，仅作被调用方，控制台可治理）。
- 谁调它？→ 自己前端（用户态 TDT）/ 其他 App 后端（令牌交换，见 portal-app-exchange skill）/ 其他服务无用户直调（服务账号 `client_credentials`）。三种来路授权姿势不同（指引 §15.4 的表）。
- 它自带认证体系（API-Key/节点 Token/gRPC）？→ 按 §15.6 划界：门户面必走 TDT 四道闸；自有节点面保持原认证但**不得**进 `/svc`（`/svc` 只转发 HTTP :8080）。

## 交付物评审 checklist（收外部团队东西时逐项验）

1. 镜像：监听 8080、网络别名 `<listingKey>-server`、amd64+arm64、**不烘焙 .env**、`docker save` tar + sha256 或私有 registry；
2. `app.manifest.json`（外部团队 repo 里的单一事实源）；
3. env 契约表：**镜像实际读取的变量名**（知识库读 `XGENT_PG_DSN` 而契约别名叫 `KNOWLEDGE_DATABASE_URL`；有的镜像不读裸 `PORT`——必须写清映射）；
4. 运维口径：DB 迁移由谁跑（启动自迁 or 外部命令）+ 是否需每租户 bootstrap（业务表 FK 自己的 tenants 表就需要，`id` 必须 = 门户租户 UUID）+ `/health` 口径。
5. 对接契约文档：对标 `knowledge-app-contract.md` 的精度（路由→scope→PID 表、env、运维命令、信任边界）。没有这份文档的接入不算完成——新服务（如任务网关）要先补。

## 资源服务器硬契约（外部实现最常炸的三处）

1. **自省信封解包**：`POST /api/tokens/introspect` 的声明在 `data` 里，必须 `claims = body.data ?? body`——裸读顶层 `active` 会把一切有效 TDT 判 401（真实事故）；
2. **`/health` 形状**：`{"service":"<key>","db":"ok",...}`，`"db"` 是字符串 `"ok"` 不是 `true`（devkit healthcheck 按此判活）；
3. **门户三变量 all-or-nothing**：自省地址 + SA clientId + secret 全缺→鉴权停用 503；缺一→启动 fail-fast 打印缺失项。

## 注册布线

**dev / 一盒**：`bun run register-app <manifest>`（`apps/api/scripts/register-app.ts`，幂等，生产拒跑）= upsert listing + 直写 `token.introspect` 服务账号 + 写发起方 App Secret + 写 Caddy `/svc` 白名单 map。⚠️ `exchangeInitiatorSecret` 写的是**已安装**实例——先在门户安装 App，**再重跑一次** register-app，否则发起交换 401。

**生产**：登记进 `apps/api/src/db/provisioning.ts`（`LISTING_DEFS` / `SA_DEFS` / `EXCHANGE_WIRING`）随 `bootstrap:prod` 落库；secret 取 env、缺省随机生成一次性打印。生产没有 manifest 明文密钥这条路。

**scope 三规则**（`market/service.ts::validateScopes`）：平台基础 scope ∪ 自己命名空间（含连字符→下划线变体，`omni-parser`→`omni_parser.*`）∪ 已声明 `exchangeTargets` 的目标命名空间；越界 `VALIDATION_FAILED`。App 命名空间 scope 随清单入库，**不用改门户代码**。

**secret 一致红线**：平台落库的服务账号 secret 与镜像 env 里的必须一致。漂移症状是自省 401 或「跨应用授权缺失」（实为 `SECRET_INVALID`）——排查先 sha256 比对两侧。

## 无前端服务的租户管理员配置

外部团队实现 `GET/PUT /v1/settings`（TDT 四道闸）；**门户侧要按 App 补三件**（不是零改码）：`<key>-token` 铸造端点 + `<key>-config-api.ts` + `AppForm` 配置 Section——参照 files / llm-gateway，具体步骤见 portal-backend-app skill 的「应用配置 tab」节。接入新服务时把这三件列入门户侧工作量（omni-parser 至今欠着）。

## 一盒联调与冒烟

外部团队不用克隆本 repo：平台给 `ai-portal-one-box` + `ai-portal-proxy` 两镜像，对方出 manifest + `compose.env` + 自己的镜像，compose 起齐后 register-app。冒烟：

```bash
curl http://localhost/svc/<key>/health          # 判上面的健康信封
curl -X POST http://localhost/svc/<key>/v1/...  # 缺/错 token 应 401/403
```

排查速查：`/svc/<key>` 404 = 白名单 map 没写成或反代先于 register-app 起；502 = 后端没起/没听 8080/别名没命中；交换 401 = 发起方 App Secret 未写（先装再重跑）。更多见 one-box 指南末尾速查表。
