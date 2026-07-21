# 知识库联调套件 (knowledge devkit)

> **本套件是「门户一盒」通用套件的样板实例,已被泛化。** 新接入(知识库以外的任何 App)
> 请用 **[`../app-devkit/`](../app-devkit/)**(App 无关,一份 override 吃任意 key,经
> `register-app` 数据化注册,不再把 App 烘焙进门户镜像)。本目录保留为知识库的**带 chroma 的
> 完整工作样板**,且依赖镜像里已烘焙的 `knowledge` 路由——若你的门户镜像已是「一盒」版,请改走
> 通用套件 + [`../app-devkit/manifests/knowledge.manifest.json`](../app-devkit/manifests/knowledge.manifest.json)。

让**知识库团队**在本地把整条链路跑通：一个已注册好 `knowledge` App 的门户 + 你们的
`knowledge-server` 后端 + 你们的临时前端。无需门户源码,只需 Docker + 我们交付的镜像。

> 配套契约/实现说明：[`../../docs/知识库后台接入与本地联调指南.md`](../../docs/知识库后台接入与本地联调指南.md)。
> 本 README 只讲**怎么把本地栈跑起来**。

## 你会拿到什么(平台团队交付)

1. **门户镜像**(两个)：`xgent-ai-portal`(runtime) + `xgent-ai-portal-proxy`(proxy)。
   以 `docker save` tar 包交付(不上 docker hub)。镜像里已烘焙好 `knowledge` 的市场清单、
   ACL、scope、`/svc/knowledge` 反代路由、`/apps/knowledge` 静态托管。
2. 本 `deploy/` 目录(`docker-compose.yml` + `knowledge-devkit/` + `postgres/` + `caddy/`)。

## 你要准备(知识库团队)

1. **后端镜像** `knowledge-server`：监听 `8080`(容器内统一端口,反代按 `/svc/<key>` → `<key>-server:8080` 路由),暴露 `GET /health`,按契约读环境变量。
2. **临时前端**：`bun run build`(或等价)产出的 `dist/` 目录。

## 步骤

```bash
# 0) 载入门户镜像(无需 docker hub)
docker load < xgent-ai-portal.tar.gz
docker load < xgent-ai-portal-proxy.tar.gz
docker load < knowledge-server.tar.gz        # 你们自己的后端镜像

# 1) 配置 env：复制基础模板,再把 knowledge 增量并进去
cp deploy/compose.env.example deploy/compose.env
cat deploy/knowledge-devkit/compose.env.knowledge.example >> deploy/compose.env
#    编辑 deploy/compose.env：
#    - XGENT_IMAGE / XGENT_PROXY_IMAGE = 你 load 进来的门户镜像 tag
#    - KNOWLEDGE_IMAGE                  = 你们的后端镜像 tag
#    - KNOWLEDGE_FRONTEND_DIST          = 你们前端 dist 的【绝对路径】
#    - (强随机) SESSION_SECRET / TDT_SIGNING_KEY 等基础密钥

# 2) 起本地基础设施(pg/redis/minio)——会自动建好 xgent-knowledge 库
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  --profile local-infra up -d postgres redis minio

# 3) 门户库迁移 + 开发种子(含 knowledge 清单,未安装)
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  run --rm portal-api bun run db:migrate
docker compose --env-file deploy/compose.env -f deploy/docker-compose.yml \
  run --rm portal-api bun run db:seed

# 4) 起门户 + 反代 + 你们的 knowledge-server(挂上前端 dist 卷)
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml \
  -f deploy/knowledge-devkit/docker-compose.knowledge-dev.yml \
  --profile local-infra --profile app-knowledge \
  up -d reverse-proxy portal-api knowledge-server

# 5) 你们自己的库迁移(命令以你们镜像为准)
docker compose --env-file deploy/compose.env \
  -f deploy/docker-compose.yml \
  -f deploy/knowledge-devkit/docker-compose.knowledge-dev.yml \
  --profile app-knowledge exec knowledge-server <你们的迁移命令>
```

## 冒烟(走通主路径)

打开 `http://localhost/` →

1. **dev 登录** → 选 `rockie`(平台管理员 + 晨光教育集团 admin)。
2. **应用市场 → 安装「知识库」** → 会连带补装「文件管理」,并自动接上
   `knowledge → files` 令牌交换(grant + 白名单 + 应用密钥)。
3. **应用中心打开知识库** → iframe 加载 `/apps/knowledge/`(你们的前端),`sdk.ready()` 握手成功。
4. **前端调后端**：`sdk.callService("knowledge", "/api/...")` 通 → 证明宿主代理 +
   你们的四道闸 + 自省全链路 OK。
5. **读/写文件**(若实现)→ 首次弹交换授权页,同意后证明 `knowledge→files` 交换生效。

## 排查

- **knowledge-server 起不来**：`docker compose ... logs knowledge-server`。多半是
  `KNOWLEDGE_IMAGE` 没设或库没迁移。
- **iframe 404 / 空白**：`KNOWLEDGE_FRONTEND_DIST` 路径不对(必须绝对路径,且目录里有
  `index.html`)。改完重起 `reverse-proxy`。
- **`/svc/knowledge` 502**：`knowledge-server` 没起(确认带了 `--profile app-knowledge`)。
- **自省被拒(401)**：`KNOWLEDGE_SA_CLIENT_SECRET` / `KNOWLEDGE_RESOURCE_KEY` 与门户
  种子里的不一致——本套件默认两边都用 `knowledge-dev-resource-key`,保持相等即可。
- **db:seed 后要重登**：seed 重新生成 UUID,会话失效,重新 dev 登录。

## 注意

- 这是 **dev** 联调套件(`DEV_MOCK_OAUTH=true`),**不要**用于生产。生产改走按需部署
  (把 knowledge 纳入 `DEPLOYABLE_APP_SERVICES`),见接入指南 §5 tier 2。
- knowledge 在本套件里是**常驻**服务(不经 deploy-controller 按需拉起),所以要显式带
  `--profile app-knowledge`。
