# KubeSphere / Kubernetes 部署指南（生产）

> XGENT.ai Portal 的 **生产** 部署：平台服务常驻，6 个微应用后端在集群里是 `replicas=0` 的 Deployment，租户首次安装某应用时 `deploy-controller` 把它 **scale 0→1**（scale-to-zero 冷启动）。Docker / Compose 降为本地开发 / 单机演示（见 [`docker.md`](docker.md)）。
>
> 规划：`goal/KubeSphere.md`。集群从零搭建：[`kubesphere-install.md`](kubesphere-install.md)。本指南**复用** DEPLOY 已落地的全部按需部署机制（状态机、`app_service_deployments`/`app_deployment_jobs`、两层模型、MicroAppHost 状态页），只把编排后端从 `docker compose` 换成 K8s API。

---

## 1. 架构

```
集群 Ingress / LB  ──TLS──►  reverse-proxy (Caddy, 烘焙前端 + 单源 CSP)
                                  │  upstream = K8s Service FQDN
        ┌─────────────────────────┼──────────────────────────────┐
        ▼                         ▼                              ▼
   portal-api:3000        /svc/<key>/ → <key>-server         /apps/<key>/ (Caddy 内静态)
   (replicas=1)           (replicas 0→1 按需)
        ▲
   deploy-controller (唯一驱动 K8s API：scale 0→1；最小权限 RBAC，无 docker.sock)
```

| 工作负载 | 类型 | 初始副本 | envFrom |
| --- | --- | --- | --- |
| `portal-api` | Deployment + Service | 1 | `xgent-config` + `portal-secret` |
| `deploy-controller` | Deployment + ServiceAccount | 1 | `xgent-config` + `controller-secret`（**唯一全量**） |
| `reverse-proxy` | Deployment + Service | 1 | 仅反代专属 env（**无业务 Secret**） |
| `files/spms/sms/qbank/lms/llm-gateway-server` | Deployment + Service | **0** | `xgent-config` + `<key>-secret` |
| `xgent-config` | ConfigMap | — | 非密配置（pre-install hook，weight -10） |
| `<key>-secret` | Secret | — | 按 workload 拆，最小密钥面（§5） |
| `xgent-init-<rev>` | Job（hook） | — | 迁移→bootstrap，先于平台 Deployment（weight 0） |

**编排抽象**：`DeploymentDriver`（`apps/api/src/modules/deploy/drivers/`）。`DEPLOY_BACKEND` 选择：`kubernetes`（默认，生产）/ `docker`（Compose）。K8s driver 用 `@kubernetes/client-node`，in-cluster SA token，动作只有一个——`patch Deployment replicas 0→1`。镜像 tag 由 Helm values 写死，driver 不碰 tag（更新走 `helm upgrade`）。

> **client-node 版本注意**：本仓库锁定 `@kubernetes/client-node@1.4.0`。1.x 是 **对象参数** API：`readNamespacedDeployment({name,namespace})` 直接返回 `V1Deployment`（**无 `.body` 包装**，与 0.x 不同），`patch` 用 `setHeaderOptions("Content-Type", PatchStrategy.StrategicMergePatch)` 设内容类型。升级该依赖时务必复核 `drivers/kubernetes.ts` 的方法签名。

---

## 2. 前置条件

- Kubernetes ≥ 1.27（已验证 1.34.3）+ KubeSphere（已验证 4.2.1）。集群搭建见 `kubesphere-install.md`。
- 默认 StorageClass（KubeKey 自带 `openebs-hostpath`）。
- 镜像仓库（Harbor / 任意 registry），节点能拉取 + `imagePullSecret`。
- TLS 终止点二选一（§6）：① 集群 Ingress（ingress-nginx + cert-manager / KubeSphere 证书）；② LoadBalancer 直通 :443 给 Caddy 自己 ACME。
- 外部托管 PG / Redis / S3（生产推荐）或集群内 operator（仅演示）。7 个业务库：`xgent-portal` / `-files` / `-spms` / `-sms` / `-qbank` / `-lms` / `-llm-gateway`。
- 企业空间 / 项目（§2.1 已建）：Workspace `xgent`、Namespace `xgent-prod`（带 `kubesphere.io/workspace: xgent` 标签，KubeSphere 控制台可见）。

---

## 3. 构建并推送镜像

仓库根多阶段 `Dockerfile` 的 `runtime`（bun，跑 portal-api / controller / app 后端）+ `proxy`（Caddy + 烘焙前端）两目标，一个构建上下文保证前后端同版本。

```bash
# 脚本封装（自动用 git short sha 作 tag）
REGISTRY=registry.example.com/xgent TAG=v1.0.0 deploy/k8s/build-push.sh

# 等价手工：
docker build -t $REGISTRY/xgent-ai-portal:v1.0.0       --target runtime . && docker push $REGISTRY/xgent-ai-portal:v1.0.0
docker build -t $REGISTRY/xgent-ai-portal-proxy:v1.0.0 --target proxy   . && docker push $REGISTRY/xgent-ai-portal-proxy:v1.0.0
```

> `proxy` 镜像烘焙前端 base / `VITE_*_SERVER_BASE=/svc/<key>`（同源相对路径），不变。`build:apps` 在镜像内**串行**构建（并行会 OOM）。

`imagePullSecret`：
```bash
kubectl -n xgent-prod create secret docker-registry harbor-pull \
  --docker-server=registry.example.com --docker-username=<u> --docker-password=<p>
```

> 不想上 Harbor 但又要给自建 `registry:2` 加认证?见 [`registry-auth/`](../../deploy/k8s/registry-auth/README.md):registry 切 token 模式 + 一个接 **Portal service account** 的小 broker(`imagePullSecret`/kaniko 凭证即一个 SA),已在真集群跑通。

---

## 4. Secret 与配置（最小密钥面，§5）

compose.env 单文件**不**整块塞一个 Secret。非密 → 共享 ConfigMap `xgent-config`；密钥 → 按 workload 拆，各 Pod 只 `envFrom` 自己那份：

| Secret | 注入给 | 内容 |
| --- | --- | --- |
| `portal-secret` | portal-api | `SESSION_SECRET`、`TDT_SIGNING_KEY`、portal `POSTGRES_DATABASE_URL`、`REDIS_CONN_STRING`、`PLATFORM_ADMIN_KEY`、OAuth、SMTP、`SMS_APP_SECRET`/`QBANK_APP_SECRET`（portal-api 安装应用时 provision 写源 app 密钥 hash，缺则回落 dev 默认→与后端不匹配→SMS/题库读 LMS 报「跨应用授权缺失或未开启」；须与 `controller-secret`/后端一致） |
| `controller-secret` | deploy-controller + init Job | **全量**：7 库 URL + 全部 `*_SA_CLIENT_SECRET` + app secret（它要为每个 app 跑 migrate/bootstrap） |
| `files-secret` | files-server | `FILES_DATABASE_URL`、`FILES_ENC_KEY`、`FILES_SA_CLIENT_SECRET`、`MINIO_*`、`REDIS_CONN_STRING`、`FILES_APP_SECRET`（files→omni-parser 交换发起方密钥，须与 `controller-secret` 一致） |
| `llm-gateway-secret` | llm-gateway-server | `LLM_GATEWAY_DATABASE_URL`、`LLM_GATEWAY_SECRET_ENC_KEY`、其 SA secret、`REDIS_CONN_STRING` |
| `spms/sms/qbank/lms-secret` | 对应后端 | 仅该 app 的 `*_DATABASE_URL` + SA secret +（qbank/sms 的）app secret + `REDIS_CONN_STRING` |

- **reverse-proxy 不注入任何业务 Secret**，只给反代 env（`XGENT_SITE_ADDRESS`、`ACME_EMAIL`、`FILES_STORAGE_ORIGIN`、`XGENT_MAX_BODY`、各 `*_UPSTREAM` Service FQDN）。
- 生产 `secrets.create=false`：Secret 由运维 `kubectl create secret generic ... --from-env-file` 预建（也是为了回填 bootstrap 一次性打印的 SA secret）。演示 `secrets.create=true`：chart 从 `values.demo.yaml` 渲染（结构见该文件，是 §5 拆分的具体参照）。
- 非密、与命名空间相关的内部 URL（`PORTAL_INTROSPECT_URL`、`LMS_SERVER_URL`）由 chart 从 `.Release.Namespace` **自动算**，运维不用硬编码命名空间。
- `*_ENC_KEY` 丢失则既有密文不可解，**务必备份**。

```bash
# 预建 controller-secret（全量）+ 各 app secret 示例：
kubectl -n xgent-prod create secret generic controller-secret --from-env-file=controller.env
kubectl -n xgent-prod create secret generic portal-secret     --from-env-file=portal.env
kubectl -n xgent-prod create secret generic files-secret      --from-env-file=files.env
# ... spms/sms/qbank/lms/llm-gateway-secret
```

---

## 5. 首次部署 —— 顺序很关键

**不能**先把 portal-api / controller 拉起来再迁移：空库时 portal-api 一查表就 CrashLoop，controller 轮询任务表也报错刷屏。正确顺序由 Helm hook 固化进一次 `helm upgrade --install`：

```
pre-install hook:  ConfigMap/Secret(weight -10)  →  init Job(weight 0): db:migrate:platform && db:migrate:deployed && bootstrap:prod
正常阶段:          portal-api / controller / reverse-proxy（replicas=1） + 6 个 app 后端（replicas=0）
```

```bash
helm upgrade --install xgent deploy/k8s/chart -n xgent-prod --create-namespace \
  -f deploy/k8s/values.prod.yaml \
  --set image.registry=registry.example.com/xgent --set image.tag=v1.0.0

# init Job 一次性打印的 SA secret → 回填各 *-secret，再 helm upgrade 一次
kubectl -n xgent-prod logs job/xgent-init-1
```

- init Job **复用 controller 同一镜像 + `envFrom controller-secret`（全量库/密钥）**，但 `automountServiceAccountToken: false`（它只跑迁移/bootstrap，不调 K8s API，不需要 RBAC）。
- 校验：`db:migrate:platform`（平台库）→ `db:migrate:deployed`（首次为空，升级时迁移已 `ready` 的 app 库）→ `bootstrap:prod`（幂等，写 listing/SA、设 embedUrl 为相对 `/apps/<key>/`）。
- hook 全绿后 portal-api / controller / reverse-proxy `Running`，6 个 app 后端 `replicas=0`。打开域名 → OAuth 登录 → 门户与应用市场。
- **勿用裸 `kubectl create job --image=... -- bun run ...`**（无 env / pull secret，必跑不起来）；健康检查用 chart 内带 env 的 Job（或 `helm test`）。

---

## 6. 反代与路由（单源 CSP）

`reverse-proxy` = 现有 Caddy 镜像进集群。**唯一改动**：Caddyfile 的 upstream 从 compose 名改成 K8s Service FQDN，且 **env 模板化**——`reverse_proxy {$PORTAL_API_UPSTREAM:portal-api:3000}` 默认走 compose 名，K8s 下由 chart 注入 `PORTAL_API_UPSTREAM=portal-api.xgent-prod.svc.cluster.local:3000` 等。一份 Caddyfile 两种后端，Caddy 仍是**唯一权威 CSP 源**。

TLS 二选一：
- **ingress-nginx 终止 TLS（推荐）**：Ingress `host` → HTTP 转 `reverse-proxy:80`。此模式 **`proxy.siteAddress=:80`**（默认），否则 Caddy 也去 ACME 抢端口。**不要**在 ingress 注解里重写 CSP / header / body —— 第二个 CSP 源会触发最严交集误挡同源 `/svc`、iframe。
- **LB 直通**：`proxy.service.type=LoadBalancer` 暴露 :443 直给 Caddy，`proxy.siteAddress=<域名>`，Caddy 继续 ACME（需公网可达 + DNS 指向 LB IP + 出网）。

> 当前线上集群无 ingress controller、无云 LB、且安全组 TCP 端口上限 20000 而 NodePort 固定 30000–32767。门户上线复用控制台同一套 **Caddy hostNetwork 直绑 80/443**（见 `kubesphere-install.md` §6）。

---

## 6.5 白标实例 + 公网上线（本期实战：portal.supagent.cn）

XGENT Portal 是 white-label 底座,**一个实例 = 一个 brand + 一套 values + 一个域名**。布局与 `brandings/` 对称:

```
brandings/<brand>/brand.config.json   构建期(logo / 名称 / 配色,烘焙进 proxy 镜像)
deploy/k8s/<brand>/values.e2e.yaml    部署期(域名 / 命名空间 / 镜像)        ←一一对应
deploy/k8s/values.{prod,demo}.yaml    通用模板(起步参考,非实例)
```

- **branding 是构建期的**:proxy 镜像烘焙 per-brand 前端(`BRAND=<brand>` 构建,见 `brandings/<brand>/`);runtime 是 brand 无关后端。chart 的 `image.proxyTag`(默认 = `image.tag`)让实例单独钉自己 brand 的 proxy、共享 runtime。单 brand 留空即可。
- **实例 values 入库、密钥不入库**:`deploy/k8s/<brand>/values.e2e.yaml` 含域名/命名空间/镜像(`secrets.create=false`);密钥(DB/SA/enc-key/OAuth)用 `kubectl` 预建。

### 公网暴露(本集群:复用 edge Caddy,不用 ingress/LB)

`reverse-proxy` 是 ClusterIP,前面由控制台那套 **edge 命名空间的 Caddy**(hostNetwork 80/443,已给 `kube.supagent.cn` 自动签 Let's Encrypt)**加一个 vhost** 反代过去 —— 终止 TLS、自动签证书,不抢 80/443、不碰 NodePort:

```bash
# 1) DNS:portal.<域名> A 记录 → 节点公网 IP(必须先生效,Caddy 才能签证书)
# 2) edge Caddy 的 Caddyfile(configmap caddy-config)追加一段后,rollout restart caddy:
https://portal.supagent.cn {
    reverse_proxy reverse-proxy.xgent-prod.svc.cluster.local:80
}
```

- 实例 values 里 **`proxy.siteAddress=:80`**(edge Caddy 终止 TLS,reverse-proxy 自身只监听 :80,不自己 ACME)、**`ingress.enabled=false`**。
- **`PORTAL_BASE_URL`/`API_BASE_URL`/`PUBLIC_BASE_URL` 必须是真域名**(OAuth 回调 + 链接都从这里拼)。⚠️ 写进**实例 values 文件**,别只手 patch configmap —— `xgent-config` 是 chart 的 hook,下次 `helm upgrade` 会用 values 重渲染、把手改的覆盖回去。

### 登录:必须配 OAuth(生产无密码/mock 登录)

门户纯 OAuth 登录。新装实例 `GET /auth/providers` 所有 provider 都 `configured:false` → **登录页空白**(没按钮)。配一个即可(GitHub 最快):

1. 在 provider 建 OAuth app,**回调地址** = `https://<域名>/auth/<provider>/callback`(如 `https://portal.supagent.cn/auth/github/callback`);Device Flow 不用勾(走授权码流程)。
2. 把 client id/secret 写进 **`portal-secret`**(`OAUTH_GITHUB_CLIENT_ID`/`OAUTH_GITHUB_CLIENT_SECRET`),重启 portal-api:
   ```bash
   kubectl -n xgent-prod patch secret portal-secret --type merge \
     -p '{"stringData":{"OAUTH_GITHUB_CLIENT_ID":"...","OAUTH_GITHUB_CLIENT_SECRET":"..."}}'
   kubectl -n xgent-prod rollout restart deploy/portal-api
   ```
3. **谁是管理员**:`bootstrap:prod` 用 `BOOTSTRAP_ADMIN_EMAIL` 建平台管理员;OAuth **按邮箱关联**,用主邮箱是该地址的账号登录即平台管理员。
4. 验证发给 GitHub 的参数对不对:`curl -s http://<reverse-proxy>/auth/github/start -D-` 看 302 `Location` 的 `client_id` + `redirect_uri` 与 app 一致。

### 多 brand 上新实例(配方,本期未做 dtedu)

1. `BRAND=dtedu` 构建 proxy → 推成 `xgent-ai-portal-proxy:dtedu-v1`(runtime 共享);
2. `deploy/k8s/dtedu/values.yaml`:换域名/命名空间 + `image.proxyTag: dtedu-v1`;
3. `helm upgrade --install dtedu deploy/k8s/chart -n dtedu-prod -f deploy/k8s/dtedu/values.yaml`(密钥同 §4 预建);
4. edge Caddy 加该域名 vhost + 配它的 OAuth。

---

## 7. 按需安装（自动 scale 0→1）

租户安装 `qbank`（依赖 `lms`）：

1. Portal 写租户 `apps` 行 + 为每个 listing 入队 deploy / provision-tenant（链路不变）。
2. controller 领 `lms` deploy：`driver.ensureServiceUp(lms)` → **patch `lms-server` replicas 0→1** → `db:lms:migrate` → `lms:provision:global` → 轮询 `lms-server.xgent-prod.svc.cluster.local:4500/health` → `ready`。
3. `qbank` 同样：scale → migrate → `qbank:provision:global`（建 qbank↔lms consent co-grant）→ health → `ready`。
4. 对当前租户跑 per-tenant `bootstrap`（lms→qbank 顺序）+ `provisionTenantPortalWiring`。
5. App Center 打开：未就绪显示「服务部署中」状态页（轮询 Runtime API），失败显示原因 + 「重试部署」。

第二个租户装同应用：复用已 `replicas=1` 的服务（不重复 scale、不重跑全局 migrate/provision），仍跑该租户幂等 per-tenant bootstrap。

**验收头条（继承 DEPLOY M4）**：安装 qbank 后其 **LMS 字典下拉非空**（provision:global + per-tenant bootstrap 真生效，非仅进程健康）。

---

## 7.5 外部镜像 App（知识库 / 全模态解析）—— 不走 chart 的 appBackends

本 chart 的 `appBackends`（6 个内建后端）都用**同一 `runtime` 镜像 + `bun --filter`**、由 controller `scale 0→1` 按需拉起。**知识库（`knowledge`）**与**全模态解析（`omni-parser`）**是**对方团队交付的外部镜像**，既不进 `runtime` 镜像、也不能 `bun --filter` 启动，故**不在** `appBackends`、**不在** `DEPLOYABLE_APP_SERVICES`（controller 管不了它们）——它们是**常驻**服务，部署方式是**独立 Deployment+Service** 部进同 namespace：

- Service 名 `<key>-server`、端口 `8080` → 反代通用 `/svc/<key>` 经集群内 DNS 解析到它（`knowledge`、`omni-parser` 均已在 Caddyfile 白名单内联，无需挂 `/svc` 白名单卷或重建 proxy）。
- 所有 Pod 设 `enableServiceLinks: false`（同 §11 的端口 NaN 坑）；`securityContext` 用**对方镜像的非 root 用户 uid**（非 bun 镜像，未必是 1000）。
- **knowledge** 与 **omni-parser** 的门户侧登记（listing/SA/`/svc` 路由 + `knowledge→files` / `files→omni-parser` 交换）都已随 `bootstrap:prod`，部署对方镜像即接上（omni-parser 还需在租户里安装其隐藏 service 实例）。

完整清单（Secret + Deployment + Service + Chroma + 前端 dist 同源托管 + 令牌交换接线 + 排错）见 **[`external-apps.md`](external-apps.md)**。

---

## 8. 更新与回滚 —— 迁移在前，代码在后

`helm upgrade` 是滚动发布。两条硬约束：

**① 向前兼容迁移（expand / contract）**：单次发布的迁移必须 additive（新列可空 / 带默认、不删列不改类型），新旧代码可短暂共存（滚动期 + 回滚窗口）。破坏性清理拆到下一次发布、且在「所有代码都已是新版」之后。

**② 迁移先于代码滚动**：`pre-upgrade` hook（同 §5 的 init Job）在 Deployment 滚动**前**跑完 `db:migrate:platform` + `db:migrate:deployed`（按 `app_service_deployments.status='ready'` 迁移已部署的 app 库；`replicas=0` 未部署的不迁）。

```bash
deploy/k8s/build-push.sh registry.example.com/xgent v1.1.0   # 构建+推新 tag
helm upgrade --install xgent deploy/k8s/chart -n xgent-prod -f deploy/k8s/values.prod.yaml --set image.tag=v1.1.0
helm -n xgent-prod get hooks xgent | grep xgent-init           # 确认 hook 跑过
```

- **app 后端副本数在升级中保留**：chart 用 `lookup` 读 Deployment 当前 `replicas`，避免 `helm upgrade` 把 controller 已 scale 到 1 的服务重置回 0。首次安装（lookup 返回空）才默认 0。
- **回滚**：`helm rollback xgent <REV>`（前端随 proxy 镜像 tag 一起回滚）。遵守 ① 则旧代码兼容当前 schema；本期不做自动 down migration。
- **手动缩容**：`kubectl -n xgent-prod scale deploy/<key>-server --replicas=0`。

---

## 9. 安全收口与 RBAC（相对 Docker 的净提升）

Docker 方案 controller 挂 `docker.sock` ≈ 宿主机 root。K8s 下换成**命名空间内最小权限**（已在真集群验证）：

- controller 专属 **ServiceAccount**，`loadFromCluster()` 读 SA token。
- **Role**（仅 `xgent-prod`）：只给 `apps/deployments` 的 `get` + `patch`，且按 `resourceNames` 限定到 6 个 app 后端。**不给 `list`/`watch`**（controller HTTP 轮询 /health，不 watch rollout）、**不给 `pods`**、**不给 create/delete**、不跨 namespace。
- **双重防线**：代码也只 patch `DEPLOYABLE_APP_SERVICES[*].k8s.deployment` 白名单里的名字；租户输入永不进 K8s 调用。
- 全 Pod 非 root + `drop ALL` caps + `allowPrivilegeEscalation:false` + seccomp RuntimeDefault；init Job 无 SA token。
- 生产拒 `DEV_MOCK_OAUTH=true` / dev resource-key fallback；SA secret 不被 seed 覆盖。
- 可选 **NetworkPolicy**（`networkPolicy.enabled=true`）：app 后端 ingress 允许来源 = reverse-proxy / portal-api / **deploy-controller**（轮询 /health）/ **Job**（`xgent.io/role: job`）。漏掉后两者会卡健康检查。egress 放开（外部 DB/Redis/S3/OAuth/SMTP）。

**真集群验证结果**（K8s 1.34.3，impersonate SA）：patch 白名单 Deployment ✓、patch 非白名单（portal-api）✗、list/watch ✗、跨 namespace ✗、create/get pods ✗；scale 0→1 strategic-merge patch 被接受、Pod 起来；Service FQDN `<svc>.<ns>.svc.cluster.local:<port>` 解析且 HTTP 可达。

---

## 10. KubeSphere 平台集成与可观测

- 工作负载归入企业空间 `xgent` / 项目 `xgent-prod`（namespace 带 `kubesphere.io/workspace: xgent` 标签 → 控制台可见可运维）。
- 监控（WhizardTelemetry / Prometheus-Grafana）/ 日志 / 告警在扩展中心按需开（7.5G 单节点别全开）。补 DEPLOY 留的「无指标平台」缺口。
- Harbor 作镜像仓库（扩展中心 / 自建）。

---

## 11. 常见问题

- **`helm install --dry-run=server` 报 `apiVersion not set`**：某模板渲染出**只有注释的空文档**（apiserver 拒收，但 `helm template` 会静默剔除）。两类成因都已在本 chart 规避：① 条件模板（secrets/ingress/networkpolicy）把 `{{- if }}` 放在**文件第一行**，关闭时零字节；② 行内 `{{- $x := ... -}}` 的左 trim 会把下一行 `apiVersion` 粘到上一行 `#` 注释末尾——把赋值 hoist 到文件顶、注释改用 `{{- /* */ -}}` 模板注释。改 chart 后务必 `helm install --dry-run=server` 复验。
- **app 后端打开很慢**：scale-to-zero 冷启动要等调度 + 拉镜像 + 启动 + 健康，比 compose 久。已有状态页展示；可预热镜像 / 设合理 `DEPLOY_HEALTH_TIMEOUT_MS`。
- **Pod Ready 但 db 未通**：readiness probe 只看 `/health` 200；controller 的 `waitHealthy` 另查 `db:true` 才落 `ready`，口径更严。
- **节点拉不到镜像**：检查 `imagePullSecret` 与 registry 鉴权——拉不到 = 全盘起不来（新引入故障面）。
- **升级后 app 被缩回 0**：确认 chart 的 `lookup` 副本保留逻辑生效（`--dry-run=server` 会 lookup 真集群；纯 `helm template` 因无集群恒为 0，属预期）。

### 11.1 真集群运行时坑（dry-run / typecheck 都抓不到，已修进 chart）

下面 4 个只在真 Pod 跑起来才暴露，已在本 chart 修好——换镜像底座 / bun 版本时复查：

- **`CreateContainerConfigError`: image has non-numeric user (bun)**：oven/bun 镜像声明 `USER bun`（名字非 uid），kubelet 在 `runAsNonRoot: true` 下无法确认非 root → 拒启。**必须显式 `runAsUser: 1000`**（bun 用户的 uid；chart `values.runAsUser`）。
- **`unable to verify the first certificate`（controller 调 K8s API 失败）**：`@kubernetes/client-node` 1.x 用原生 `fetch`，**bun 的 fetch 不应用该库加载的集群 CA** → apiserver 证书验不过。给 deploy-controller 设 **`NODE_EXTRA_CA_CERTS=/var/run/secrets/kubernetes.io/serviceaccount/ca.crt`**（chart 已内置）让 bun 信任 SA CA。
- **app server 监听端口变 `NaN` → 健康检查超时**：K8s 给每个 Service 注入**同名 legacy 环境变量**（`lms-server` Service → `LMS_SERVER_PORT=tcp://ip:4500`），正好冲掉 server 读取监听端口的同名 env → `Number("tcp://…")=NaN`。所有 Pod 设 **`enableServiceLinks: false`**（chart 已内置）。
- **reverse-proxy 绑不上 :80**：caddy 镜像以 root 跑才能绑特权端口，不能用 `runAsNonRoot`；且 `drop: ALL` 会把 root 的 `NET_BIND_SERVICE` 也去掉。chart 给反代单独的 securityContext（不设 runAsNonRoot + `add: NET_BIND_SERVICE`）。彻底无 root 方案：Caddy 跑高位端口 + Service 映射（留作后续）。

### 11.2 单节点构建（无 docker / 无 Harbor 时）

本仓库镜像偏重（runtime 含完整 workspace），单节点 / 国内网络下：

- **别在本机 arm64 交叉构建 amd64**：QEMU 模拟下 `bun install` + 7 个 vite 构建极慢（实测 bun install 卡 9 分钟）。**改用 kaniko 在节点原生构建**（`bun install` 2.9s、每个 vite ~3s）：kaniko Job `hostNetwork: true`（其 `localhost:5000` 即 registry hostPort）+ `--registry-mirror=docker.m.daocloud.io`（docker.io 国内不可达）+ `--cache`（runtime/proxy 共享 build 阶段）+ `--context=dir://`（节点 hostPath 解出的 context tar）。kaniko 会构建到 target 为止的**所有**前置 stage，故 proxy build 也要传 `--build-arg INSTALL_DOCKER_CLI=false`。
- **K8s 镜像不装 docker CLI**：Dockerfile runtime stage 的 `docker-ce-cli` 安装用 `ARG INSTALL_DOCKER_CLI`（默认 true 给 Compose）。K8s 构建传 `--build-arg INSTALL_DOCKER_CLI=false`（controller 走 K8s API 不需要 docker，且避开 deb.debian.org / download.docker.com 这类脆弱外部 apt 依赖）。`build-push.sh` 已默认传 false。
- **单节点免 registry 备选**：本期按 registry:2 验证；单节点其实可 `ctr -n k8s.io images import` 直接喂镜像 + `imagePullPolicy: Never`，更省。

---

## 12. 验收清单

> 下列 ✅ 已在真集群 K8s 1.34.3 验证(2026-06-12);监控/Harbor 留作 M5 收尾。

- [x] 构建+推 `runtime`/`proxy` 到 registry;节点能拉,无 `:local` 依赖。(kaniko 节点原生 → in-cluster registry:2)
- [x] `helm upgrade --install` 先经 pre-install hook 跑完迁移→bootstrap,再拉起平台三件套 + 6 个 `replicas=0` 后端;`/svc/*` 路由可解析。
- [x] reverse-proxy Pod 内无 DB/SA secret;各 app 看不到别人的库。(server-dry-run + 渲染核对)
- [x] 首次安装 qbank → controller scale lms+qbank 0→1 → migrate → provision → health → `ready`;**qbank LMS 字典下拉非空(9/3/14)+ qbank→lms exchange grant active**。
- [x] 第二租户复用、不重复 scale(无新 deploy job)。
- [x] controller 无法越权(不能改非白名单 Deployment / 跨 namespace);无 docker.sock。(RBAC 9/9)
- [ ] KubeSphere 控制台监控/日志(WhizardTelemetry)接入 — **M5 待做**。

---

## 13. 交付物索引

| 交付物 | 路径 |
| --- | --- |
| DeploymentDriver 抽象 + 两实现 | `apps/api/src/modules/deploy/drivers/{types,docker-compose,kubernetes,index}.ts` |
| 注册表 K8s 坐标 | `apps/api/src/modules/deploy/registry.ts`（`k8s` 字段） |
| controller 接线 | `apps/api/scripts/deploy-controller.ts`（`makeDriver()`） |
| Helm chart | `deploy/k8s/chart/`（workloads / Service / Ingress / ConfigMap / 按 workload 拆 Secret / RBAC / hook Job / NetworkPolicy） |
| values（通用模板） | `deploy/k8s/values.prod.yaml` / `deploy/k8s/values.demo.yaml` |
| 实例 values（白标,↔ `brandings/<brand>/`） | `deploy/k8s/<brand>/values.e2e.yaml`（如 `deploy/k8s/xgent/`,§6.5） |
| registry token 认证（接 Portal SA） | `deploy/k8s/registry-auth/`（`registry-token-broker.ts` + recipe，§3 注） |
| Caddyfile（模板化 upstream） | `deploy/caddy/Caddyfile` |
| 镜像构建+推送 | `deploy/k8s/build-push.sh` |
| `db:migrate:deployed` | `apps/api/scripts/migrate-deployed.ts`（root script `db:migrate:deployed`） |
| driver 冒烟 | `apps/api/scripts/verify-k8s-driver.ts`（root script `k8s:verify:driver`） |
