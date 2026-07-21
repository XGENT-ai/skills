# KubeSphere · 动态部署开发计划

> 本期目标：把现有「按需部署」从 Docker Compose 迁到 **KubeSphere / Kubernetes**，让租户首次安装某市场应用时，`deploy-controller` 在 K8s 集群里**动态拉起**对应后端服务。生产部署改为 K8s-only，Docker Compose 降为本地开发 / 单机演示。
>
> 前置阅读：`goal/DEPLOY.md`（当前 Docker 方案）、`docs/deploy/docker.md`。本计划**复用** DEPLOY 已落地的全部按需部署机制（状态机、DB 表、安装链路、两层模型、MicroAppHost 状态页），只替换 controller 的**编排后端**。

---

## 进度与现状（更新于 2026-06-11）

> 本节记录已完成的部分。**结论：本计划已端到端跑通并在真集群（K8s 1.34.3 @ 119.91.53.146）验收——含真 XGENT 镜像的按需安装（Phase 6 M3）。** 代码/chart/脚本/文档（Phase 2/3/4/7/9）+ Phase 1/1.5 + §2.1 全部落地；最初受阻的「真镜像 e2e」已用 in-cluster `registry:2` + kaniko 节点原生构建（绕开本机 arm64→amd64 QEMU 慢 + 节点无 docker）打通。**M3 头条：装 qbank → controller scale lms+qbank 0→1 → migrate → provision:global → health → `ready`；LMS 字典非空（subjects=9/stages=3/grades=14）+ qbank→lms exchange grant active；第二租户复用不重 deploy。**

**✅ KubeSphere 底座已就绪**（安装全过程见 `docs/deploy/kubesphere-install.md`，已是已验证流程）：
- 腾讯云单节点 CVM（Ubuntu 24.04，4C/7.5G，公网 `119.91.53.146`），**Kubernetes v1.34.3 + KubeSphere v4.2.1 社区版（已激活）**；KubeKey v4.0.5 装 K8s + `ks-core` OCI（`oci://hub.kubesphere.com.cn/kse/ks-core` chart 1.2.4）装 KubeSphere。
- 控制台 **https://kube.supagent.cn**：Caddy（`edge` 命名空间，hostNetwork 直绑 80/443）+ **Let's Encrypt** 自动证书，HTTP 自动 301 跳 HTTPS，证书自动续。

**Phase 0 环境事实（已确认 / 对计划假设的修正）**：

| 项 | 计划原设想（§2/§9 Phase0） | 实际 / 决定 |
| --- | --- | --- |
| StorageClass | 需自办 | ✅ 已有默认 `openebs-hostpath`（KubeKey config `storage_class.local` 自带），监控/PVC 可直接用 |
| 镜像 registry | Harbor | ⏳ **Harbor 未装（仍 TODO）**；过渡期 docker.io 镜像走 **DaoCloud 镜像**（`docker.m.daocloud.io/...`，docker.io 在国内不可达） |
| Ingress / LB / 暴露 | ingress-nginx 终止 TLS 或 LB 直通 Caddy（§4） | ⚠️ 集群无 ingress controller、无云 LB；**且腾讯云安全组 TCP 端口上限 20000，而 K8s NodePort 固定 30000–32767 → 任何 NodePort 都开不了**。改用 **Caddy hostNetwork 直绑 80/443**（§4「保留 Caddy」路线，已验证）。**门户上线复用同一套**，不碰 NodePort。 |
| TLS | 待定（§4 二选一） | ✅ **Caddy + Let's Encrypt（TLS-ALPN-01）已验证**；Deployment 必须 `strategy: Recreate`（hostPort 80/443 冲突会让 RollingUpdate 死锁） |
| 外部 DB/Redis/S3 | 外部托管（§2.3） | ⏳ 未接（XGENT 落地时办） |
| 企业空间/项目 `xgent`/`xgent-prod` | §2.1 | ✅ 已建（Workspace `xgent` + namespace `xgent-prod`，带 `kubesphere.io/workspace` 标签，控制台可见） |

**✅ 本期完成（2026-06-11，代码/chart/脚本/文档 + 真集群验证）**：
- **Phase 2** `DeploymentDriver` 抽象（`drivers/{types,docker-compose,kubernetes,index}.ts`）+ `KubernetesDriver`（`@kubernetes/client-node@1.4.0`，scale 0→1）+ 注册表 `k8s` 坐标 + controller 接线 `makeDriver()`；`DEPLOY_BACKEND=docker` 不回退（`bun run k8s:verify:driver` 31 green，typecheck 干净）。
- **Phase 3 + 7** Helm chart `deploy/k8s/chart/`：9 Deployment（平台 3 + 6 个 `replicas=0` 后端）+ 8 Service + Ingress + ConfigMap + 按 workload 拆 Secret（reverse-proxy 无业务 secret）+ 最小权限 RBAC（get/patch by resourceName，无 list/watch）+ pre-install/upgrade hook Job（迁移→bootstrap，weight 排序）+ NetworkPolicy；`values.prod/demo`；`helm lint` ×3 green。
- **Phase 4** Caddyfile upstream env 模板化（compose 默认 + K8s FQDN override，一份文件两后端）；`caddy validate` 通过。
- **Phase 1.5 / 真集群验证**：RBAC 9/9（白名单可 patch、非白名单/list/watch/跨 ns/create/pods 全拒）、SA strategic-merge patch scale 0→1 被接受 Pod 起来、Service FQDN `<svc>.<ns>.svc.cluster.local:<port>` 解析且 HTTP 可达、**整 chart prod+demo `helm install --dry-run=server` 通过 K8s 1.34.3 schema 校验**。
- **脚本/文档**：`db:migrate:deployed`（升级迁已部署 app 库）、`deploy/k8s/build-push.sh`、`docs/deploy/kubesphere.md`（构建→装 chart→初始化→按需→更新→回滚→FAQ），`docker.md` 标注为本地开发/单机演示。

**e2e 验证拓扑（本次真集群）**：in-cluster `registry:2`（hostPort 127.0.0.1:5000 + containerd `localhost:5000` http mirror）+ kaniko 节点原生构建 runtime/proxy + 集群内 PG/Redis/MinIO（演示 infra）+ 按 §5 拆的 8 个 Secret（强密钥，SA 密钥两边同值）+ `helm install`（hook migrate→bootstrap）→ 按需装 qbank 验收。

**仍 TODO（非阻塞）**：Harbor 替代 registry:2（Phase 1/8，正式镜像仓库）；监控/日志接入（Phase 8 WhizardTelemetry）；ingress/LB（本次 e2e 用 in-cluster 校验，未走公网域名）。

**新增踩坑沉淀**（详见 `docs/deploy/kubesphere.md` §11 与项目记忆 `kubesphere-1-implementation`）：
- **chart 渲染期**：① `--dry-run=server` 拒收「只有注释的空文档」（条件模板 `{{- if }}` 放首行；行内 `{{- $x -}}` 左 trim 会把 `apiVersion` 粘到注释末尾 → hoist + `{{- /* */ -}}`）；② `lookup` 保留 controller scale 后的副本数避免 `helm upgrade` 重置回 0；③ `@kubernetes/client-node` 1.x 对象参数 API（read 无 `.body`，patch 用 `setHeaderOptions`+`PatchStrategy`）。
- **真集群运行时（只有跑起来才暴露）**：④ bun 镜像 `USER bun` 非数字 → `runAsNonRoot` 下 kubelet 拒启 → 必须 `runAsUser:1000`；⑤ bun 的 fetch 不应用 client-node 加载的集群 CA → controller 加 `NODE_EXTRA_CA_CERTS=…/serviceaccount/ca.crt`；⑥ K8s 给同名 Service 注入 `LMS_SERVER_PORT=tcp://…` 冲掉 server 监听端口 → `NaN` → 全 Pod `enableServiceLinks:false`；⑦ caddy 反代要 root 绑 :80 且 `drop ALL` 去掉了 `NET_BIND_SERVICE` → 反代单独 securityContext（不 runAsNonRoot + add NET_BIND_SERVICE）。
- **构建**：⑧ 本机 arm64→amd64 QEMU 交叉构建奇慢（bun install 卡 9min）→ 改 kaniko 节点原生（bun install 2.9s）；⑨ kaniko 构建 target 前的所有 stage，故 proxy build 也要 `--build-arg INSTALL_DOCKER_CLI=false`；⑩ runtime 镜像 docker-ce-cli 改 `ARG INSTALL_DOCKER_CLI`（K8s 传 false，去脆弱外部 apt 依赖 + controller 无需 docker）。安装期踩坑见 `kubesphere-install.md` §8。

---

## 0. 范围收口

### 本期做

- 把 `deploy-controller` 的编排层从「调 `docker compose up`」抽象成 `DeploymentDriver` 接口，新增 **KubernetesDriver**（`@kubernetes/client-node`，集群内 in-cluster 配置）。
- 动态部署机制：每个微应用后端在集群里是一个 **Deployment（初始 `replicas=0`）+ Service**；首次安装时 controller 把 `replicas` 从 `0→1`（scale-to-zero 冷启动）。
- 沿用**全局共享服务**租户模型：一个 listing 一个 Deployment，服务所有租户，隔离仍在「单库 + TDT introspection」内。
- 用 **Helm chart**（KubeSphere App Store 也是 Helm 体系，未来可发成应用模板）声明全部静态拓扑：平台服务、6 个 app 后端（`replicas=0`）、Service、Ingress、共享 ConfigMap + **按 workload 拆的 Secret**（最小密钥面）、controller 的最小权限 RBAC。
- 反代沿用现有 **Caddy 代理镜像**（已烘焙前端 + 单源 CSP），改为集群内 Deployment，upstream 指向 K8s Service DNS；集群 Ingress/LB 在前面终止 TLS（此模式 Caddy 只监听 `:80`）。
- 镜像推送到**容器镜像仓库**（Harbor / 任意 registry）+ `imagePullSecrets`（替代本地 `:local` 镜像）。
- 平台初始化（`db:migrate:platform`、`bootstrap:prod`）+ 升级迁移以 **Helm pre-install/pre-upgrade hook Job**（复用 SA + pull secret + `controller-secret`）跑，**迁移先于平台 Deployment 启动**。
- controller 去掉 `docker.sock` 这种宿主机 root 等价权限，改为 **ServiceAccount + 命名空间内最小权限 Role**（只能 scale 白名单内的 6 个 Deployment）。
- KubeSphere 平台集成：企业空间/项目（namespace）、监控/日志/告警可观测、Workloads/Harbor。
- 文档：`docs/deploy/kubesphere.md`；并把 `docs/deploy/docker.md` 标注为「本地开发 / 单机演示」。

### 本期不做

- 不做**每租户独立 namespace / 独立库**（沿用全局共享服务；强隔离留作后续）。
- 不做完整 CI/CD pipeline（KubeSphere DevOps / Jenkins）。镜像构建+推送先用脚本/手工，pipeline 留作后续。
- 不做 Knative / KEDA 的「闲置自动缩到 0」（本期只做「安装时 `0→1`」，缩容手动）。
- 不做多集群 / 多区域 / 联邦。
- 不做 HPA 自动扩缩（副本数本期固定 1）。
- 不做蓝绿 / 金丝雀；更新走 `helm upgrade` 滚动发布。
- 不做云厂商 Secret Manager / External Secrets Operator（先用 K8s Secret；ESO 留作后续）。
- 不做把微应用发成 KubeSphere App Template「一键安装」（Helm chart 已为此铺路，发模板留作后续）。

---

## 1. 当前状态判断

### 1.1 可直接复用（不改）

DEPLOY 已落地的按需部署机制与编排后端无关，本期**整段复用**：

- **DB 表**：`app_service_deployments`（全局/按 listing）+ `app_deployment_jobs`（任务队列）— 不变。
- **状态机 / 幂等规则**：`apps/api/src/modules/deploy/service.ts`（`ensureDeploymentQueued` / `queueDeploymentsForInstall` / `getAppRuntime` / `retryDeployment`）— 不变。
- **两层模型**：服务部署（全局，一次）+ 租户开通（每租户，幂等）— 不变。
- **provision 逻辑**：`apps/api/src/db/provisioning.ts`（`provisionGlobal` / `provisionTenantPortalWiring`）纯 DB 操作，与编排器无关 — 不变。
- **安装链路改造**、**Runtime API**（`GET /api/apps/:appKey/runtime`、`POST /api/admin/apps/:appKey/deployment/retry`）、**MicroAppHost 部署状态页** — 不变。
- **DEPLOYABLE_APP_SERVICES 注册表**（`apps/api/src/modules/deploy/registry.ts`）的白名单语义 — 保留，仅**增加 K8s 坐标字段**。
- **`<key>:provision:global` / 各后端 `bootstrap.ts` / `db:<key>:migrate`** — 不变；controller 仍以子进程方式跑（见 §3.4）。

### 1.2 必须替换 / 新增

- `apps/api/scripts/deploy-controller.ts` 里 `composeUp()` 是**唯一**碰 Docker 的地方 → 抽象成 `DeploymentDriver.ensureServiceUp()`，新增 K8s 实现。
- 注册表增加每服务的 K8s 坐标（deployment / service / port；namespace optional，缺省用 env，§3.5）。
- `waitHealthy()` 的 healthUrl 在集群内是 Service FQDN（`http://<svc>.<ns>.svc.cluster.local:<port>/health`）；逻辑不变，URL 来源改为按 driver 派生。
- 新增 Helm chart（`deploy/k8s/`）：全部 K8s 资源清单。
- 新增镜像仓库推送流程 + `imagePullSecrets`。
- 新增 controller 的 ServiceAccount / Role / RoleBinding。
- 反代从 compose service 改为集群内 Deployment + Service + Ingress；Caddyfile upstream 改 K8s DNS。
- compose.env 单文件 → 共享 ConfigMap（非密）+ **按 workload 拆的多个 Secret**（最小密钥面，§5）。

### 1.3 关键约束（继承 DEPLOY，仍成立）

- 单域名、同站点反代：Portal Web/API/WS/OAuth 同源；微应用同域子路径 `/apps/<key>/`、`/svc/<key>/`。**单源 CSP** 由反代权威下发。
- 独立后端只通过 TDT + introspection 接入；7 个业务库物理隔离。
- 生产禁用 `DEV_MOCK_OAUTH=true` 与 `*_RESOURCE_KEY` dev fallback；`prod-guard` 不变，密钥来源改为 K8s Secret。
- 仅进程健康 ≠ 可用：`provision:global` + per-tenant `bootstrap` 必须真跑过（qbank 的 LMS 字典下拉非空才算 ready）。

---

## 2. 目标 K8s / KubeSphere 拓扑

### 2.1 命名空间与归属

- KubeSphere **企业空间（Workspace）**：`xgent`。
- KubeSphere **项目（Project = namespace）**：`xgent-prod`（平台 + 全部 app 后端同 namespace；如需更强隔离可拆 `xgent-system` / `xgent-apps`，本期单 namespace）。
- controller 的 RBAC 严格限定在该 namespace，且按 Deployment 名进一步收口（§7）。

### 2.2 工作负载

| 资源 | 类型 | 初始副本 | 说明 |
| --- | --- | --- | --- |
| `portal-api` | Deployment + Service | 1 | `@xgent/api`，容器 `:3000` |
| `deploy-controller` | Deployment + ServiceAccount | 1 | 唯一驱动 K8s API 的组件；最小权限 RBAC |
| `reverse-proxy` | Deployment + Service | 1 | Caddy 代理镜像（烘焙前端 dist + 单源 CSP）；前置集群 Ingress/LB |
| `files-server` | Deployment + Service | **0** | `:4100`，按需 `0→1` |
| `spms-server` | Deployment + Service | **0** | `:4200`，按需 `0→1` |
| `sms-server` | Deployment + Service | **0** | `:4300`，按需 `0→1` |
| `qbank-server` | Deployment + Service | **0** | `:4400`，按需 `0→1` |
| `lms-server` | Deployment + Service | **0** | `:4500`，按需 `0→1` |
| `llm-gateway-server` | Deployment + Service | **0** | `:4600`，按需 `0→1` |
| `xgent-config` | ConfigMap | — | 非密配置（base URL / 端口 / `XGENT_SITE_ADDRESS` / 特性开关） |
| `xgent-secret` | Secret | — | 全部密钥 + DB URL + SA secret + OAuth + SMTP |
| Ingress | Ingress | — | 单域名路由（见 2.4） |

**关键设计**：app 后端的 Deployment、Service、Ingress 规则**全部由 chart 预先创建**（所以 `/svc/<key>/` 路由始终可解析，对应 Caddy「懒解析 upstream」的等价物），只是 Deployment 起始 `replicas=0`。controller 的动态动作 = **patch `replicas` 0→1**。静态拓扑走 Helm（GitOps 友好），动态扩容走 SDK。

> 备选（不采用）：完全「create-on-demand」（安装时才 `apply` Deployment+Service+Ingress）。缺点：Service/Ingress 解析竞态、create/delete 抖动更多。scale-to-zero 更稳、路由更稳。

### 2.3 基础设施（外部托管为默认）

- 生产：`xgent-secret` 里的 `*_DATABASE_URL` / `REDIS_CONN_STRING` / `MINIO_*` 指向**外部托管** PG / Redis / S3。
- 演示 / 自包含：用 KubeSphere App Store 安装 PG / Redis / MinIO operator（StatefulSet + PVC + StorageClass），仅供单集群演示。
- 7 个业务库（`xgent-portal` / `xgent-files` / `xgent-spms` / `xgent-sms` / `xgent-qbank` / `xgent-lms` / `xgent-llm-gateway`）的创建：外部库由 DBA 预建或一次性 init Job；集群内 operator 则用其初始化机制。

### 2.4 公网路由（单域名，不变）

集群 Ingress/LB → `reverse-proxy`（Caddy） → 各 upstream（K8s Service DNS）。路由表与 DEPLOY §2.2 完全一致：

| 路径 | 目标 |
| --- | --- |
| `/` | `apps/web/dist`（Caddy 内静态） |
| `/api`·`/auth`·`/oauth`·`/health`·`/ws` | `portal-api.xgent-prod.svc:3000` |
| `/apps/<key>/*` | 微应用静态包（Caddy 内静态） |
| `/svc/<key>/*` | `<key>-server.xgent-prod.svc:<port>` |

TLS 两种方式（择一，§4）：①集群 Ingress（ingress-nginx）终止 TLS，HTTP 透传给 Caddy；②LB 直通 :443 给 Caddy，Caddy 继续做 ACME。

---

## 3. DeploymentDriver 抽象与按需部署改造

### 3.1 为什么仍要抽象（即便生产是 K8s-only）

「KubeSphere 取代 Docker」指**生产**。但：① 本地开发 / 单机演示仍用 Compose；② 把唯一碰编排器的代码收口到一个小模块，K8s 改造才是 surgical 且可测的。所以保留 `DeploymentDriver`，提供 `docker`（dev-only）与 `kubernetes`（生产）两个实现，`DEPLOY_BACKEND` 选择。

### 3.2 接口（新增 `apps/api/src/modules/deploy/drivers/`）

`composeUp` 是 controller 唯一的编排副作用，所以接口面极小：

```ts
// apps/api/src/modules/deploy/drivers/types.ts
import type { DeployableService } from "../registry";

export interface DeploymentDriver {
  /** 幂等地把该 app 后端拉到可服务状态。
   *  docker: compose up profile;  k8s: patch Deployment replicas 0→1。 */
  ensureServiceUp(svc: DeployableService): Promise<void>;
  /** controller 据此 URL 轮询 /health（集群内 Service FQDN / compose 网络名）。 */
  healthUrlFor(svc: DeployableService): string;
  /** （可选，后续缩容用）scale 到 0 / compose stop。 */
  scaleDown?(svc: DeployableService): Promise<void>;
}

export function makeDriver(): DeploymentDriver {
  return (process.env.DEPLOY_BACKEND ?? "kubernetes") === "docker"
    ? new DockerComposeDriver()   // 现有 composeUp 逻辑搬进来（dev-only）
    : new KubernetesDriver();
}
```

### 3.3 KubernetesDriver（核心新增）

```ts
// apps/api/src/modules/deploy/drivers/kubernetes.ts
import * as k8s from "@kubernetes/client-node";

export class KubernetesDriver implements DeploymentDriver {
  private api: k8s.AppsV1Api;
  private ns = process.env.XGENT_K8S_NAMESPACE ?? "xgent-prod";

  constructor() {
    const kc = new k8s.KubeConfig();
    kc.loadFromCluster();                 // in-cluster: ServiceAccount token
    this.api = kc.makeApiClient(k8s.AppsV1Api);
  }

  async ensureServiceUp(svc: DeployableService) {
    const ns = svc.k8s.namespace ?? this.ns;   // 本期单 namespace，字段缺省即用 env
    const name = svc.k8s.deployment;
    const dep = await this.api.readNamespacedDeployment(name, ns);
    if ((dep.body.spec?.replicas ?? 0) >= 1) return;             // 已起，幂等
    await this.api.patchNamespacedDeployment(
      name, ns, { spec: { replicas: 1 } },
      undefined, undefined, undefined, undefined, undefined,
      { headers: { "content-type": "application/strategic-merge-patch+json" } },
    );
    // 仅 patch 副本数；健康仍由 controller 轮询 healthUrlFor()（见 §3.4）。
  }

  healthUrlFor(svc: DeployableService) {
    const ns = svc.k8s.namespace ?? this.ns;
    return `http://${svc.k8s.service}.${ns}.svc.cluster.local:${svc.k8s.port}/health`;
  }
}
```

> controller 只 `patch replicas`，不改镜像 tag —— 镜像 tag 由 Helm values 在 chart-apply 时写进 Deployment。更新走 `helm upgrade`（§8）。

### 3.4 迁移 / provision / bootstrap：保持 in-controller 子进程（不改）

DEPLOY 里 controller 用 `Bun.spawn(["bun","run", svc.migrateScript])` 等子进程跑迁移/bootstrap，从自身 env 读各库 URL。K8s 下 controller Pod 是**同一 runtime 镜像**（含完整 workspace + drizzle SQL + 各 bootstrap 脚本），`envFrom` 注入它专属的 **`controller-secret`（含全部 7 个库 URL + 全部 SA/app secret，§5）**+ 共享 ConfigMap → 子进程继承 → **行为与今天完全一致**。注意：「持全量 secret」**只有 controller 这一个 workload**，其余 Pod 按 §5 拆分最小密钥面。所以 `handleDeploy` / `handleProvisionTenant` 的迁移/provision/bootstrap 段**不动**，只把 `composeUp` 换成 `driver.ensureServiceUp`、`waitHealthy` 的 URL 换成 `driver.healthUrlFor`。

```ts
// handleDeploy 改动点（其余不变）
- const up = await composeUp(svc.profile, svc.serviceName);
- if (up.code !== 0) throw new Error(...);
+ await driver.ensureServiceUp(svc);           // docker: compose up；k8s: scale 0→1
  const mig = await run(["bun", "run", svc.migrateScript]);   // 不变
  if (svc.globalProvisionScript) await provisionGlobal(job.listingKey); // 不变
- if (!(await waitHealthy(svc.healthUrl))) throw ...;
+ if (!(await waitHealthy(driver.healthUrlFor(svc)))) throw ...;
```

> 备选（不采用，留作后续硬化）：把 migrate/bootstrap 改成 **K8s Job**（run-to-completion Pod），controller watch 到完成再继续。更隔离、可设独立资源上限、不阻塞 controller 领任务循环；但多了模板+watch+日志收集，本期不需要。

### 3.5 注册表增加 K8s 坐标（`registry.ts`）

```ts
export interface DeployableService {
  // 现有字段保留：serviceName / profile / migrateScript /
  //   globalProvisionScript / bootstrapScript / healthUrl / publicBaseUrl
  // namespace 本期单 namespace 故缺省 → driver 用 XGENT_K8S_NAMESPACE；保留 optional
  // 只为将来真要多 namespace 时不破坏签名，本期注册表里不填，避免「填了但 driver 不读」的误导。
  k8s: { namespace?: string; deployment: string; service: string; port: number };
}

// 例（本期不填 namespace，driver 按 env 默认 xgent-prod）：
qbank: {
  serviceName: "qbank-server", profile: "app-qbank",        // docker driver 用
  migrateScript: "db:qbank:migrate",
  globalProvisionScript: "qbank:provision:global",
  bootstrapScript: "apps/qbank-server/scripts/bootstrap.ts",
  healthUrl: "http://qbank-server:4400/health",             // docker 网络名（dev）
  publicBaseUrl: "/svc/qbank",
  k8s: { deployment: "qbank-server", service: "qbank-server", port: 4400 },
},
```

白名单语义不变：只有注册表里的 listing 才触发部署；K8s driver 只 patch `k8s.deployment` 命名的 Deployment（与 RBAC 的 `resourceNames` 互为防线，§7）。

---

## 4. 反向代理与路由

**沿用现有 Caddy 代理镜像**（已烘焙 7 个前端 dist + 单源 CSP + SPA fallback + 压缩 + body 上限 + WS upgrade），改为集群内 Deployment + Service。**唯一改动**：Caddyfile 的 upstream 从 compose service 名改为 K8s Service FQDN，且做成可模板化（环境变量 / Helm 渲染），例如：

```
reverse_proxy /api/*  portal-api.xgent-prod.svc.cluster.local:3000
reverse_proxy /svc/files/*  files-server.xgent-prod.svc.cluster.local:4100
...
```

**关键：分清「公网站点地址」与「Caddy 监听地址」。** 现有 Caddyfile 是 `{$XGENT_SITE_ADDRESS::80} { ... }` —— 这个值是 **Caddy 的站点/监听地址**，一旦设成域名，Caddy 会对它自动 ACME 申请 HTTPS。所以 `XGENT_SITE_ADDRESS` **只决定 Caddy 监听什么**，不能拿它当「公网域名」用；公网域名另用 `PORTAL_BASE_URL` / `API_BASE_URL`（应用层生成回调/链接）表达。

TLS（择一）：
- **ingress-nginx 终止 TLS（推荐）**：Ingress `host: portal.example.com`（cert-manager / KubeSphere 证书）→ HTTP 转发到 `reverse-proxy` Service:80。此模式下 **Caddy 必须只监听 `:80`，即 `XGENT_SITE_ADDRESS=:80`**，否则 Caddy 也会去 ACME，和 ingress 抢、且集群内拿不到 :443 证书会失败。公网域名只写在 Ingress `host` + ConfigMap 的 `PORTAL_BASE_URL`。CSP/SPA/body/WS 仍由 Caddy 负责（不要在 ingress 注解里重写，避免 `meta ∩ header` 类最严交集误挡同源 `/svc`、iframe）。
- **LB 直通（Caddy 自己 ACME）**：Service `type=LoadBalancer` 暴露 :443 直给 Caddy，`XGENT_SITE_ADDRESS=portal.example.com`，Caddy 继续 ACME（需公网可达 + DNS 指向 LB IP + 出网）。`PORTAL_BASE_URL` 与之一致。

> 不采用「用 ingress-nginx Ingress 直接路由 + 另起静态文件服务」：会丢掉 Caddy 的烘焙 SPA、单源权威 CSP，得重做 SPA fallback / 头 / body 限制 / 压缩，风险大、收益低。Caddy 是单一 CSP 权威源这点必须守住。

---

## 5. 配置与密钥

compose.env 单文件**不**整块塞进一个共享 Secret —— 那会让 reverse-proxy 拿到 DB URL、让 files-server 拿到 llm-gateway 的库 URL，密钥面过大。按**最小权限**拆：非密配置共享一个 ConfigMap，密钥按 workload 拆成多个 Secret，各 Pod 只 `envFrom` 自己需要的那份。

**共享 ConfigMap `xgent-config`**（非密，可广播给所有 Pod）：`API_BASE_URL`、`PORTAL_BASE_URL`/`PUBLIC_BASE_URL`、各 `*_APP_URL`（同源下多为相对）、端口、`DEPLOY_BACKEND=kubernetes`、`XGENT_K8S_NAMESPACE`、`NODE_ENV=production`、特性开关。（`XGENT_SITE_ADDRESS`/`ACME_EMAIL`/`FILES_STORAGE_ORIGIN`/`XGENT_MAX_BODY` 是反代专属，见 §4，放反代自己的 env 或一个 `proxy-config`。）

**按 workload 拆的 Secret**：

| Secret | 注入给 | 内容 |
| --- | --- | --- |
| `portal-secret` | portal-api | `SESSION_SECRET`、`TDT_SIGNING_KEY`、portal `*_DATABASE_URL`、`REDIS_CONN_STRING`、OAuth 凭据、SMTP、`SMS_APP_SECRET`/`QBANK_APP_SECRET`（portal-api 安装应用时跑 provisionTenantPortalWiring 写源 app 密钥 hash，缺则回落 dev 默认值→与后端不匹配→SMS/题库读 LMS 报「跨应用授权缺失或未开启」；须与 `controller-secret` 及对应后端一致） |
| `controller-secret` | deploy-controller | **全量**：7 个库 URL + 全部 `*_SA_CLIENT_SECRET` + app secret（它要为每个 app 跑 migrate/bootstrap/provision，唯一需要全量的 workload） |
| `files-secret` | files-server | `FILES_DATABASE_URL`、`FILES_ENC_KEY`、`FILES_SA_CLIENT_SECRET`、S3/MinIO 凭据、`REDIS_CONN_STRING` |
| `llm-gateway-secret` | llm-gateway-server | `LLM_GATEWAY_DATABASE_URL`、`LLM_GATEWAY_SECRET_ENC_KEY`、其 SA secret、（如有）供应商渠道密钥、`REDIS_CONN_STRING` |
| `spms`/`sms`/`qbank`/`lms`-secret | 对应后端 | 仅该 app 自己的 `*_DATABASE_URL` + 其 SA secret + （qbank/sms 的）app secret + `REDIS_CONN_STRING` |

各 Pod 的 `envFrom`：
- portal-api → `[xgent-config, portal-secret]`
- deploy-controller → `[xgent-config, controller-secret]`（全量）
- `<key>-server` → `[xgent-config, <key>-secret]`（只看到自己的库/密钥，看不到别的 app 的）
- **reverse-proxy → 不注入任何业务 Secret**，只给它的几个反代专属 env（无 DB/SA/OAuth secret）。

后端读 `process.env` 不变（沿用容器 env 注入模型，`start:prod` 不带 `--env-file`）。`prod-guard` 不变：弱/空密钥仍拒绝启动；密钥来源换成 K8s Secret。**`*_ENC_KEY` 务必备份**（丢失则既有密文不可解）。

> 后续：External Secrets Operator / KubeSphere 证书与密钥管理对接（本期不做）。开 etcd 静态加密保护 K8s Secret（默认仅 base64）。

---

## 6. 镜像与制品

「替代 Docker」意味着不能再用本地 `:local` 镜像 —— 节点要从 registry 拉。

- 仍是仓库根多阶段 `Dockerfile` 的 `runtime` + `proxy` 两目标（前后端同版本），构建后 **push 到 registry**（KubeSphere 自带 **Harbor**，或任意私有/公有 registry）。
- chart 用 `image: <registry>/xgent-ai-portal:<tag>` + `imagePullSecrets`。
- 本期镜像构建+推送先用脚本（`docker build && docker push`）或 `make` 目标；KubeSphere DevOps / Jenkins pipeline 留作后续。
- `proxy` 镜像的前端 base / `VITE_*_SERVER_BASE=/svc/<key>` 烘焙逻辑不变（同源相对路径）。

---

## 7. 安全收口与 RBAC（相对 Docker 的净提升）

Docker 方案里 controller 挂 `docker.sock` ≈ 宿主机 root。K8s 下换成**命名空间内最小权限**：

- controller 用专属 **ServiceAccount**；`KubeConfig.loadFromCluster()` 读 SA token。
- **Role**（仅 `xgent-prod` namespace）：**只给 driver 真正用到的 `apps/deployments` 的 `get` + `patch`**，且按 `resourceNames` 限定到 6 个 app 后端 Deployment。
  - 当前 driver 用 HTTP 轮询 `/health` 判就绪，**不 watch rollout**，所以**不授予 `list`/`watch`，也不给 `pods`**（用不到就不给）。
  - **不**给 create/delete Deployment、不给跨 namespace、绝不 cluster-admin。
- **双重防线**：即便 RBAC 放宽，controller 代码也只会 patch `DEPLOYABLE_APP_SERVICES[*].k8s.deployment` 白名单里的名字；租户输入永不进 K8s 调用。

```yaml
# 草图：与 §3.3 driver（仅 readNamespacedDeployment + patch）逐一对应
kind: Role
metadata: { name: deploy-controller, namespace: xgent-prod }
rules:
  - apiGroups: ["apps"]
    resources: ["deployments"]
    resourceNames: ["files-server","spms-server","sms-server","qbank-server","lms-server","llm-gateway-server"]
    verbs: ["get","patch"]
```

> 若后续把就绪判定从「HTTP 轮询」改成「watch Deployment rollout 状态」，再补 `list`/`watch`（注意 K8s 限制：`resourceNames` 只能约束 `get/patch/update/delete`，`list`/`watch` 只能 namespace 级），并在计划里写清 watch 的用途与 field selector 限制。本期不需要。

其余收口（不变 / 复用）：生产拒 `DEV_MOCK_OAUTH=true`、禁 dev resource-key fallback、SA secret 不被 seed 覆盖；Pod 非 root、`readOnlyRootFilesystem`（可行处）、`drop ALL` capabilities。

**NetworkPolicy（可选，但要算全访问来源）**：若给 app 后端加 ingress 白名单，**允许来源不止 reverse-proxy / portal-api** —— `deploy-controller` 要轮询 `/health`、migrate/bootstrap/health 的 **Job Pod** 也要连到该后端及其库。所以允许来源至少包含这些 label：`reverse-proxy`、`portal-api`、`deploy-controller`、以及 Job 用的统一标签（如 `xgent.io/role: job`）。漏掉 controller / Job 标签会让部署卡在健康检查。出口（egress）还要放行到外部托管 DB/Redis/S3 与 OAuth/SMTP/供应商域名。

---

## 8. 初始化、按需部署与升级在 K8s 上的流程

### 8.1 首次部署 —— 顺序很关键

**不能**「先 `helm install` 把 portal-api / controller 拉起来，再跑迁移」：空库时 portal-api 一查表就报错（可能 CrashLoop），controller 轮询 `app_deployment_jobs` 也会持续报错刷屏。正确顺序是 **先建 namespace/config/secret/RBAC + 跑迁移/bootstrap，再启动平台 Deployment**。用 **Helm hook** 把这条顺序固化进一次 `helm upgrade --install`：

- `pre-install,pre-upgrade` hook，`hook-weight` 升序：先 namespace/ConfigMap/Secret/RBAC（hook 之外的普通资源 chart 会先于带 weight 的 hook 创建命名空间级依赖；实践中把 Secret/ConfigMap/SA 也标成低 weight 的 hook 或确保它们随 chart 先建）→ `db:migrate:platform` Job（weight 0）→ `bootstrap:prod` Job（weight 1）。
- 这两个 Job **必须复用与 controller 同一套挂载**：`serviceAccountName`、`imagePullSecrets`、`envFrom: [xgent-config, controller-secret]`（要全部库 URL/密钥）。**不要**用裸 `kubectl create job --image=... -- bun run ...`（那样没有 env / pull secret / SA，必跑不起来）。
- 平台 Deployment（portal-api / controller / reverse-proxy）与 6 个 `replicas=0` 的 app 后端是普通资源，hook 成功后才进入正常 install/upgrade 阶段被创建/更新 → 启动时表已就绪。

```yaml
# deploy/k8s/chart/templates/job-migrate-bootstrap.yaml（草图）
apiVersion: batch/v1
kind: Job
metadata:
  name: xgent-init-{{ .Release.Revision }}
  annotations:
    "helm.sh/hook": pre-install,pre-upgrade
    "helm.sh/hook-weight": "0"
    "helm.sh/hook-delete-policy": before-hook-creation,hook-succeeded
spec:
  backoffLimit: 1
  template:
    metadata:
      labels: { xgent.io/role: job }        # NetworkPolicy 允许来源（§7）
    spec:
      restartPolicy: Never
      serviceAccountName: deploy-controller   # 复用 SA
      imagePullSecrets: [{ name: harbor-pull }]
      containers:
        - name: init
          image: "{{ .Values.image.registry }}/xgent-ai-portal:{{ .Values.image.tag }}"
          envFrom:
            - configMapRef: { name: xgent-config }
            - secretRef:    { name: controller-secret }   # 全量库/密钥
          command: ["sh","-c"]
          # 幂等：迁移在前，bootstrap 在后（bootstrap:prod 自身幂等）
          args: ["bun run db:migrate:platform && bun run bootstrap:prod"]
```

```bash
# 0) 构建+推镜像（节点要能从 registry 拉）
docker build -t <registry>/xgent-ai-portal:<tag>       --target runtime . && docker push <registry>/xgent-ai-portal:<tag>
docker build -t <registry>/xgent-ai-portal-proxy:<tag> --target proxy   . && docker push <registry>/xgent-ai-portal-proxy:<tag>

# 1) 预先建好 Secret（首次用占位/真实值；bootstrap 产出的一次性 SA secret 回填后再 helm upgrade）
#    kubectl -n xgent-prod create secret generic controller-secret --from-env-file=...

# 2) 一次 helm：pre-install hook 跑 迁移→bootstrap，成功后才拉起平台 Deployment
helm upgrade --install xgent deploy/k8s/chart -n xgent-prod --create-namespace \
  -f deploy/k8s/values.prod.yaml --set image.tag=<tag>

# 3) bootstrap Job 日志里一次性打印的 SA secret → 回填各 *-secret，再 helm upgrade 一次
kubectl -n xgent-prod logs job/xgent-init-1

# 4) 平台健康：用 chart 内的 Job 模板（带 envFrom+SA+pull secret），或 `helm test`；
#    勿用裸 `kubectl create job --image=... -- bun run health:platform`（无 env/SA/pull secret，跑不起来）。
helm test xgent -n xgent-prod   # 若把 health:platform 包成 helm test hook
```

hook 跑完后 `portal-api` / `deploy-controller` / `reverse-proxy` `Running`，6 个 app 后端 `replicas=0`（未起）。打开域名 → OAuth 登录 → 门户与应用市场。

### 8.2 按需安装（自动 scale 0→1）

租户安装 `qbank`（依赖 `lms`）：

1. Portal 写租户 `apps` 行 + 为每个 listing 入队 deploy / provision-tenant（链路不变）。
2. controller 领 `lms` deploy 任务：`driver.ensureServiceUp(lms)` → **patch `lms-server` replicas 0→1** → `db:lms:migrate` → `lms:provision:global` → 轮询 `lms-server.xgent-prod.svc:4500/health` → `ready`。
3. 对 `qbank` 同样：scale → migrate → `qbank:provision:global`（建 qbank↔lms consent co-grant 等）→ health → `ready`。
4. 对当前租户跑 per-tenant `bootstrap`（lms→qbank 顺序）+ `provisionTenantPortalWiring`。
5. App Center 打开：未就绪显示「服务部署中」状态页（轮询 Runtime API），失败显示原因 + 「重试部署」。

第二个租户安装同应用：复用已 `replicas=1` 的服务（不重复 scale、不重跑全局 migrate/provision），仍跑该租户幂等 per-tenant bootstrap。

### 8.3 更新与回滚 —— 迁移在前，代码在后

`helm upgrade` 是滚动发布。若「先升级代码再迁移」，新代码会打在旧 schema 上，启动失败或运行期报错。所以两条硬约束：

**① 向前兼容迁移（expand / contract，硬约束）**：单次发布的迁移必须是 **additive / 向后兼容** 的——新列可空或带默认、不删列不改类型不加破坏性约束。破坏性清理（删列/改类型）**拆到下一次发布**，且只在「所有代码都已是新版」之后做。这样新旧代码能短暂共存（滚动期间 + 回滚窗口）。

```
发布 N：  additive 迁移（兼容旧代码） → 升级代码到 N
发布 N+1：（确认 N 全量生效后）cleanup 迁移（删旧列等） → 升级代码到 N+1
```

**② 迁移先于代码滚动（默认路径）**：复用 §8.1 的 `pre-upgrade` hook Job，在 Deployment 滚动**之前**跑完迁移：
- 平台库：`db:migrate:platform`（hook 已含）。
- 已 `ready` 的 app 库：hook 里再跑一遍这些 app 的 `db:<key>:migrate`（读 `app_service_deployments.status='ready'` 得清单；建议加个 `db:migrate:deployed` 聚合脚本）。`replicas=0` 未部署的 app 不迁移（首次安装时由 controller 迁）。
- hook 全绿后，`helm upgrade` 才进入正常阶段滚动 portal-api / app 后端到新镜像 → 新代码落在已迁移 schema 上。

```bash
# 构建+推新 tag → 一次 helm（pre-upgrade hook 先迁移，再滚动）
helm upgrade --install xgent deploy/k8s/chart -n xgent-prod -f deploy/k8s/values.prod.yaml --set image.tag=<new>
kubectl -n xgent-prod create job health-all-<new> ...   # health:platform + health:deployed（用 chart 内 Job 模板）
```

- **回滚**：`helm rollback xgent <REV>`（前端随 proxy 镜像 tag 一起回滚）。因为遵守 ①，旧代码仍兼容当前（已迁移的）schema；本期不做自动 down migration。
- **缩容**（手动）：`kubectl -n xgent-prod scale deploy/<key>-server --replicas=0`。

---

## 9. 开发任务

### Phase 0 · 前置与基线确认
- [ ] 确认 KubeSphere / K8s 版本、ingress controller（ingress-nginx）、StorageClass、LB（裸金属需 MetalLB）、registry（Harbor）可用。
- [ ] 确认「K8s-only 生产、Compose 仅本地」范围；外部托管 DB/Redis/S3 连接信息就位。

### Phase 1 · 镜像与仓库
- [ ] `runtime` / `proxy` 镜像 push 到 registry；chart 用 `imagePullSecrets`。
- [ ] 镜像构建+推送脚本 / `make` 目标。
- 验收：节点能从 registry 拉到镜像；无本地 `:local` 依赖。

### Phase 1.5 · K8s 冒烟 spike（强烈建议，先于 Phase 2 全面铺开）
用**最小一条竖切**提前暴露集群差异，避免到 Phase 6 才发现 SDK/DNS/RBAC/网络策略不对。只做 `files-server` 一个：
- [ ] 一个 `files-server` Deployment（`replicas=0`）+ Service + controller 的 SA/Role（仅 `files-server` get/patch）。
- [ ] 一段最小 driver 代码：`readNamespacedDeployment` + `patch replicas=1`，验证 **`@kubernetes/client-node` 版本签名**（各版本方法签名/返回结构有差异）。
- [ ] 从 controller Pod 内 `curl` Service FQDN `/health`，验证 **Service DNS** 与就绪判定口径。
- [ ] 验证 **RBAC** 真能 patch（且 patch 非白名单 Deployment 被拒）、**NetworkPolicy** 不挡 controller→后端。
- 验收：一条 `0→1` 竖切在真集群跑通；记录 client-node 版本、DNS 后缀、RBAC/NP 细节，回灌 Phase 2/3/7。

### Phase 2 · DeploymentDriver 抽象 + KubernetesDriver
- [ ] 新增 `drivers/{types,docker-compose,kubernetes}.ts`；`composeUp` 逻辑搬进 DockerComposeDriver。
- [ ] `KubernetesDriver.ensureServiceUp`（scale 0→1）+ `healthUrlFor`（Service FQDN）。
- [ ] `registry.ts` 增加 `k8s` 坐标；`deploy-controller.ts` 改用 `makeDriver()`，替换 `composeUp` / `waitHealthy` URL 来源。
- [ ] 加依赖 `@kubernetes/client-node`。
- 验收：`DEPLOY_BACKEND=docker` 行为与今天一致；`=kubernetes` 时 controller 能 patch 副本数（可先用 mock/集群外 kubeconfig 单测 driver）。

### Phase 3 · Helm chart（静态拓扑）
- [ ] `deploy/k8s/chart/`：namespace、共享 ConfigMap、**按 workload 拆的 Secret**（`portal-secret` / `controller-secret`(全量) / 各 `<key>-secret`，§5）、portal-api / deploy-controller / reverse-proxy Deployment+Service、6 个 app 后端 Deployment(`replicas=0`)+Service、Ingress、SA/Role/RoleBinding。
- [ ] **pre-install/pre-upgrade hook Job**（`job-migrate-bootstrap.yaml`，§8.1）：`serviceAccountName` + `imagePullSecrets` + `envFrom controller-secret`，先于平台 Deployment 跑迁移→bootstrap。
- [ ] 每个 Deployment：`envFrom` 共享 ConfigMap + **自己那份 Secret**（reverse-proxy 不挂业务 Secret）、readiness/liveness probe（`/health` 含 `db:true`）、非 root、资源 request/limit、Job/工作负载统一 label（供 NetworkPolicy）。
- [ ] （可选）NetworkPolicy：app 后端 ingress 允许 reverse-proxy / portal-api / **deploy-controller / Job**（§7）。
- [ ] `values.prod.yaml` / `values.demo.yaml`（demo 含集群内 infra）。
- 验收：一次 `helm upgrade --install` 先跑完 hook（迁移→bootstrap）再拉起平台三件套 + 6 个 `replicas=0` 后端；`/svc/*` 路由可解析（502/503-before-scale）；reverse-proxy Pod 内无 DB/SA secret。

### Phase 4 · 反代与路由
- [ ] Caddyfile upstream 改 K8s Service FQDN（模板化）；reverse-proxy 进集群。
- [ ] Ingress / LB 终止 TLS；单源 CSP / SPA fallback / body 上限 / WS upgrade 全部经 Caddy。
- 验收：登录、WS 铃铛、iframe、`sdk.callService()`、Files presign/finalize 全走通；`curl -I` 见安全头；app 后端未起时反代仍正常启动。

### Phase 5 · 基础设施接入
- [ ] 生产：外部 PG/Redis/S3 连接串入 Secret；7 库就位（DBA 预建或 init Job）。
- [ ] 演示：KubeSphere App Store 装 PG/Redis/MinIO operator（PVC/StorageClass）。
- 验收：`portal-api` / 各后端 `/health` 的 `db:true`、`redis` 往返通过。

### Phase 6 · 初始化与按需部署跑通
- [ ] `db:migrate:platform` / `bootstrap:prod` 以 **pre-install hook Job**（envFrom `controller-secret` + SA + pull secret）跑通：**先迁移再启动平台 Deployment**，避免空库 portal-api CrashLoop / controller 报错刷屏（幂等、SA secret 一次性打印回填）。`health:platform` 同样走带 env 的 chart 内 Job，**不用裸 `kubectl create job --image`**。
- [ ] （更新场景）`pre-upgrade` hook 先迁移已 `ready` 的 app 库（`db:migrate:deployed`）再滚动代码（§8.3）。
- [ ] 首次安装 `qbank` → controller scale `lms`+`qbank` 0→1 → migrate → provision:global → health → ready；per-tenant bootstrap 跑过。
- [ ] 第二租户复用、不重复 scale；失败有状态页 + 重试。
- 验收（继承 DEPLOY M4 头条，改在 K8s 上）：**安装 qbank 后其 LMS 字典下拉非空**（provision:global + per-tenant bootstrap 真生效，非仅进程健康）；`health:deployed` 只查 `ready`。

### Phase 7 · 安全收口与 RBAC
- [ ] controller SA + 最小权限 Role/RoleBinding（按 `resourceNames` 限定 6 个 Deployment）。
- [ ] 去 `docker.sock`；Pod 非 root / drop caps / 只读根 fs（可行处）。
- [ ] 按 workload 拆 Secret（reverse-proxy 无业务 secret；各 app 看不到别人的库，§5）。
- [ ] 生产拒 `DEV_MOCK_OAUTH=true` / dev resource-key；SA secret 不被覆盖；可选 NetworkPolicy（允许来源含 controller / Job，否则卡健康检查，§7）。
- 验收：controller 无法越权（不能改非白名单 Deployment / 跨 namespace）；无 docker.sock；reverse-proxy 拿不到 DB/SA secret。

### Phase 8 · KubeSphere 平台集成与可观测
- [ ] 工作负载归入企业空间 `xgent` / 项目 `xgent-prod`；KubeSphere UI 可见可运维。
- [ ] 监控（Prometheus/Grafana）/ 日志 / 告警接入（补上 DEPLOY 留的「无指标平台」缺口）。
- [ ] Harbor 作镜像仓库。
- 验收：KubeSphere 控制台能看到各 Deployment、Pod 日志、`/health` 指标、controller 部署日志。

### Phase 9 · 文档与验收
- [ ] `docs/deploy/kubesphere.md`：构建/推镜像 → 装 chart → 初始化 → 按需部署 → 更新 → 回滚 → 常见问题。
- [ ] `docs/deploy/docker.md` 标注为「本地开发 / 单机演示」。
- [ ] smoke test 清单（K8s 版）跑通。
- 验收：空集群按文档可达「可登录 + 可打开微应用 + 可调独立后端 + qbank 字典非空」。

---

## 10. 里程碑

- ✅ **M1 · driver 抽象就位**：`DEPLOY_BACKEND` 切换可用；docker 行为不回退；K8s driver 可 patch 副本数。（`k8s:verify:driver` 31 green）
- ✅ **M2 · chart 平台环境可用**：一次 `helm install` 先经 pre-install hook 跑完迁移→bootstrap，再拉起平台三件套 + 6 个 `replicas=0` 后端；反代单域名暴露；Secret 按 workload 拆分。（真集群 helm rev4 全 1/1）
- ✅ **M3 · 按需部署可用**：首次安装应用自动 scale 0→1 + migrate + provision + health + 落库 `ready`；二次复用不重 deploy；**qbank LMS 字典非空（9/3/14）+ qbank→lms exchange grant active**。
- ✅ **M4 · 安全收口**：去 docker.sock，controller 最小权限 RBAC（真集群 9/9 验证）；生产 dev 开关全禁；Pod 非 root + drop ALL caps。
- 🟡 **M5 · KubeSphere 集成与文档**：✅ 企业空间/项目归属 + `docs/deploy/kubesphere.md` 完整 + smoke 跑通；⏳ **监控/日志（WhizardTelemetry）+ Harbor 仍待做**（本期用 in-cluster `registry:2` 验证，已加 token 认证接 Portal SA，见 `deploy/k8s/registry-auth/`）。

> 全部里程碑（除 M5 的监控/Harbor 收尾）已在真集群 K8s 1.34.3 端到端验证。详见 §进度与现状。

---

## 11. 风险清单

- **scale-to-zero 冷启动延迟**：首次打开应用要等 Pod 调度+拉镜像+启动+健康，比 compose 更久（镜像拉取）。需 UI 明确展示（已有状态页），并预热/常驻镜像、设合理 `HEALTH_TIMEOUT_MS`。
- **健康判定**：`waitHealthy` 仍轮询 `/health`（`db:true`）；要确保 readiness probe 与 controller 轮询口径一致，避免「Pod Ready 但 db 未通」误判 ready。
- **migrate/bootstrap 在 controller 子进程**：长迁移会阻塞 controller 领任务循环（与今天同），且需要 `controller-secret` 持有全部 DB URL（唯一全量 secret 的 workload）；如演进成独立 Job 要重做日志/watch。
- **初始化/升级顺序**：必须迁移先于代码（pre-install/pre-upgrade hook），否则空库 CrashLoop / 新代码打旧 schema；Secret 须在 hook 跑前就存在（先 `kubectl create secret` 或随 chart 低 weight 先建）。
- **RBAC 边界**：本期只授 `deployments` 的 `get/patch`（按 `resourceNames`），不授 `list/watch`；靠代码白名单兜底，勿放宽 create/delete。若改 watch rollout 再补，并注意 `list/watch` 不能按名收窄。
- **CSP / 单源**：TLS 终止点（ingress vs Caddy）若各自加 CSP 头会触发最严交集误挡同源 `/svc`、iframe；必须只保留 Caddy 一个权威源。
- **镜像仓库**：节点拉不到镜像（registry 鉴权 / `imagePullSecrets` 缺失）= 全盘起不来；属新引入故障面。
- **有状态数据**：外部托管时 DB 与对象存储须一起备份/恢复（files 元数据↔对象一致性）；`*_ENC_KEY` 丢失不可逆。
- **Secret 管理**：K8s Secret 默认仅 base64；生产应开 etcd 加密 / 后续接 ESO。
- **网络策略**：不加 NetworkPolicy 时 app 后端在 namespace 内可被任意 Pod 访问；按需收口。
- **演示 operator 基础设施**：集群内 PG/Redis/MinIO 的可用性/备份压力大，仅适合演示，勿用于生产。

---

## 12. 交付物清单

- [ ] `apps/api/src/modules/deploy/drivers/{types,docker-compose,kubernetes}.ts`
- [ ] `registry.ts` 增 `k8s` 坐标；`deploy-controller.ts` 改用 `makeDriver()`
- [ ] 依赖 `@kubernetes/client-node`
- [ ] `deploy/k8s/chart/`（Helm chart：工作负载 / Service / Ingress / 共享 ConfigMap / **按 workload 拆的 Secret** / RBAC）
- [ ] `job-migrate-bootstrap.yaml`（pre-install/pre-upgrade hook Job，复用 SA + pull secret + `controller-secret`）
- [ ] （可选）NetworkPolicy（允许来源含 reverse-proxy / portal-api / controller / Job）
- [ ] `deploy/k8s/values.prod.yaml` / `values.demo.yaml`
- [ ] Caddyfile（K8s upstream 模板化；ingress-TLS 模式下 `XGENT_SITE_ADDRESS=:80`）
- [ ] 镜像构建+推送脚本
- [ ] `db:migrate:deployed` 聚合脚本（升级时迁移已 `ready` 的 app 库）
- [ ] controller ServiceAccount / Role(get+patch by name) / RoleBinding
- [ ] `docs/deploy/kubesphere.md`
- [ ] `docs/deploy/docker.md` 标注为本地开发用途

---

## 13. 推荐实施顺序

1. 抽象 `DeploymentDriver`，搬现有 compose 逻辑进 DockerComposeDriver（保证 docker 不回退）。
2. 写 KubernetesDriver（scale 0→1 + healthUrlFor）+ 注册表 `k8s` 坐标 + controller 接线。
3. 写 Helm chart（静态拓扑，app 后端 `replicas=0`）+ values。
4. 适配 Caddy upstream 到 K8s DNS + Ingress/LB 终止 TLS。
5. 接基础设施（外部托管优先）+ Secret/ConfigMap 拆分。
6. Job 跑 migrate/bootstrap，平台起来，跑通首次安装自动 scale + qbank 字典非空。
7. controller 最小权限 RBAC + 去 docker.sock + 生产开关收口。
8. KubeSphere 企业空间/项目归属 + 监控/日志 + Harbor。
9. 按 `docs/deploy/kubesphere.md` 从空集群整体验收。
