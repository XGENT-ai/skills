---
name: portal-external-app
description: '接入「外部镜像服务类应用」——服务端代码不在本 repo、以 Docker 镜像交付的 App（知识库/多模态解析/异步任务网关这类）。凡任务涉及为外部服务写对接契约或 app.manifest.json、register-app/provisioning 注册布线、/svc 路由与健康检查、外部服务的服务账号与 scope、一盒(one-box)本地联调、或评审外部团队的交付物时，务必先用本 skill——即使用户只说"把 XX 服务接进来"。Use when integrating an externally-built docker-delivered service app into the portal: manifest, register-app, provisioning, /svc wiring, one-box debugging, or reviewing an external team''s delivery.'
---

# portal-external-app · 外部镜像服务类应用接入

外部镜像 App = 服务端在别的 repo（任意语言/栈）、独立镜像交付、**常驻**（不走按需缩零）。运行时契约与内建 App 完全一致，差异只在注册与部署。本文件是判断与流程；具体契约按需读 `references/`（为本 skill 提炼的自包含参考，可整目录拷到外部团队 repo；权威源是门户仓库 `docs/`，冲突以门户仓库为准）。

## 按任务读参考

| 任务 | 读 |
| --- | --- |
| 实现/评审资源服务器（四道闸、health、env、认证面划界、配置面） | [references/integration-contract.md](references/integration-contract.md) |
| 写 manifest、注册布线、一盒联调、排查 | [references/registration-and-onebox.md](references/registration-and-onebox.md) |
| 为外部服务写/审对接契约文档 | [references/contract-doc-template.md](references/contract-doc-template.md) |

## 先判形态（决定后面走哪条线）

- 有用户前端？→ `type:"micro"`（前端 dist 同源托管 `/apps/<key>/`，前端开发见 portal-micro-app skill）；无前端 → **`type:"service"`**（无头：不露卡、不可打开，仅作被调用方，控制台可治理）。
- 谁调它？→ 自己前端（用户态 TDT）/ 其他 App 后端（令牌交换，见 portal-app-exchange skill）/ 其他服务无用户直调（服务账号 `client_credentials`）。三种来路授权姿势不同（integration-contract.md §3 的表）。
- 它自带认证体系（API-Key/节点 Token/gRPC）？→ 按 integration-contract.md §6 划界：门户面必走 TDT 四道闸；自有节点面保持原认证但**不得**进 `/svc`（`/svc` 只转发 HTTP :8080）。

## 交付物评审 checklist（收外部团队东西时逐项验）

1. 镜像：监听 8080、网络别名 `<listingKey>-server`、amd64+arm64、**不烘焙 .env**、`docker save` tar + sha256 或私有 registry；
2. `app.manifest.json`（外部团队 repo 里的单一事实源）；
3. env 契约表：**镜像实际读取的变量名**（镜像自有前缀与门户契约别名的映射必须写清；有的镜像不读裸 `PORT`）；
4. 运维口径：DB 迁移由谁跑 + 是否需每租户 bootstrap（需要则 `tenants.id` 必须 = 门户租户 UUID）+ `/health` 口径；
5. 对接契约文档：按 [references/contract-doc-template.md](references/contract-doc-template.md) 的结构与精度。**没有这份文档的接入不算完成**——新服务（如任务网关）要先补。

## 资源服务器硬契约（外部实现最常炸的三处）

1. **自省信封解包**：声明在 `data` 里，必须 `claims = body.data ?? body`——裸读顶层 `active` 会把一切有效 TDT 判 401（真实事故）；
2. **`/health` 形状**：`{"service":"<key>","db":"ok",...}`，`"db"` 是字符串 `"ok"` 不是 `true`（healthcheck 按此判活）；
3. **门户三变量 all-or-nothing**：自省地址 + SA clientId + secret 全缺→鉴权停用 503；缺一→启动 fail-fast 打印缺失项。

完整契约（四道闸、三种令牌来路、审计、划界）见 [references/integration-contract.md](references/integration-contract.md)。

## 注册布线（细节见 registration-and-onebox.md）

- **dev / 一盒**：`bun run register-app <manifest>`（幂等，生产拒跑）= upsert listing + 直写服务账号 + 写发起方 App Secret + 写 `/svc` 白名单 map。⚠️ `exchangeInitiatorSecret` 写的是**已安装**实例——先安装 App，**再重跑一次** register-app，否则发起交换 401。
- **生产**：登记进 `apps/api/src/db/provisioning.ts`（`LISTING_DEFS` / `SA_DEFS` / `EXCHANGE_WIRING`）随 `bootstrap:prod` 落库。生产没有 manifest 明文密钥这条路。
- **scope 三规则**：平台基础 scope ∪ 自己命名空间（含连字符→下划线变体）∪ 已声明 `exchangeTargets` 的目标命名空间；越界 `VALIDATION_FAILED`。
- **secret 一致红线**：平台落库的服务账号 secret 与镜像 env 里的必须一致；漂移症状是自省 401 或「跨应用授权缺失」（实为 `SECRET_INVALID`），排查先 sha256 比对两侧。

## 无前端服务的租户管理员配置

外部团队实现 `GET/PUT /v1/settings`（TDT 四道闸）；**门户侧要按 App 补三件**（不是零改码）：`<key>-token` 铸造端点 + `<key>-config-api.ts` + `AppForm` 配置 Section——参照 files / llm-gateway，步骤见 portal-backend-app skill。接入新服务时把这三件列入门户侧工作量（omni-parser 至今欠着）。

## 一盒联调与冒烟

外部团队不用克隆本 repo：平台给两个门户镜像 + `deploy/` 目录，对方出 manifest + `compose.env` + 自己的镜像。步骤、冒烟命令与排查速查表见 [references/registration-and-onebox.md](references/registration-and-onebox.md) §4–§6。
