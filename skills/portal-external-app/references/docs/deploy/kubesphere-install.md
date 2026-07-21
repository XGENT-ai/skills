# KubeSphere 安装步骤(KubeKey v4 + ks-core OCI)

> 目标:从零装出一套 **KubeSphere**,为 XGENT.ai Portal 在 KubeSphere 上的动态部署(`goal/KubeSphere.md`)打底。
>
> **本指南已在真实环境跑通(2026-06-11)**:腾讯云 CVM(Ubuntu 24.04、4C/7.5G)单节点,**KubeKey v4.0.5 + Kubernetes v1.34.3 + KubeSphere v4.2.1(社区版)**。命令以此为准。
>
> **版本提醒**:KubeKey/KubeSphere 版本与兼容矩阵更新快,占位版本以官方为准。**KubeKey v4 与 v3 安装方式完全不同**(见 §0)。

---

## 0. 关键认知:KubeKey v4 ≠ v3

- **v3**:`kk create cluster --with-kubernetes ... --with-kubesphere ...` 一把装 K8s + KubeSphere。
- **v4(本指南)**:**没有 `--with-kubesphere`**。`kk` 只装 **Kubernetes**;**KubeSphere 4.x 走 Helm/OCI 单独装**。v4 是 inventory + config(playbook)驱动。
- 判断你手上的 kk:`./kk version`(`major:4` 即 v4)、`./kk create cluster --help`(无 `--with-kubesphere` 即 v4)。

整条流程 = **① kk 装 K8s → ② Helm 装 ks-core(KubeSphere)→ ③ 许可证激活 → ④ 装扩展**。

---

## 1. 前置条件

- Linux 节点(本指南 Ubuntu 24.04;单节点 ≥ 2C4G,演示建议 8G)。
- **`ubuntu` 等账号有 passwordless sudo**(本指南据此用 `connector: type: local`,免 SSH 密码)。
- 依赖:`apt-get install -y socat conntrack ebtables ipset ipvsadm curl`。
- 出网可达镜像源(国内设 `export KKZONE=cn` + config `zone: "cn"`)。
- **⚠️ 主机名必须全小写**(RFC-1123)。大写主机名(如 `VM-0-7-ubuntu`)会让 kk 的「Add worker label to node」失败、CNI 不装(见 §8①)。先 `sudo hostnamectl set-hostname <小写名>` 并同步 `/etc/hosts`。

---

## 2. 用 KubeKey v4 装 Kubernetes

```bash
# 下 kk(国内加速)
export KKZONE=cn
curl -sfL https://get-kk.kubesphere.io | sh -    # 或 VERSION=<ver> 指定
chmod +x kk

# 生成 inventory + config 模板(这是你这个 kk 版本的真实 schema,别手抄别的版本)
./kk create inventory -o inventory.yaml
./kk create config    -o config.yaml --with-kubernetes v1.34.3   # 默认即 v1.34.3,别用太新的
```

**编辑 `inventory.yaml`** —— 单节点只需在 `spec.hosts` 下加 `localhost`(`groups` 默认已全指向 localhost):

```yaml
spec:
  hosts:
    localhost:
      connector:
        type: local        # 本机直跑,免 SSH/密码(需 passwordless sudo)
  groups:                  # 以下为生成器默认,单节点不用改
    k8s_cluster: { groups: [kube_control_plane, kube_worker] }
    kube_control_plane: { hosts: [localhost] }
    kube_worker:        { hosts: [localhost] }   # 同节点既当控制面又当 worker
    etcd:               { hosts: [localhost] }
```
> 多节点:在 `spec.hosts` 填各节点(`connector: {type: ssh, host, user, password/privateKeyPath}` + `internal_ipv4`),并把节点名分配到 `kube_control_plane` / `kube_worker` / `etcd` 组。

**编辑 `config.yaml`** —— 改一处:

```yaml
spec:
  zone: "cn"                          # 默认 ""，设 cn 走国内源拉二进制/镜像
  kubernetes: { kube_version: v1.34.3 }   # 保持默认；别用 1.36+(超出 kk/KubeSphere 支持)
  # calico / containerd / 默认 local StorageClass / etcd 默认即可，不用动
```
> `config.yaml` 里 `storage_class.local.enabled/default: true` 默认开 —— 装完自带默认 StorageClass(OpenEBS local-path),省一步。

**起集群**(耗时长,后台跑 + 看日志):

```bash
nohup bash -c "yes yes | ./kk create cluster -i inventory.yaml -c config.yaml" > kk-install.log 2>&1 &
# 看进度(日志含 TTY 动画噪声,过滤时间戳行 + 去 ANSI):
grep -aE "[0-9]{2}:[0-9]{2}:[0-9]{2} CST " kk-install.log | sed "s/\x1b\[[0-9;]*m//g" | tail -20
# 看完成:出现 "Playbook ... finish. total:N,success:N,...,failed:0"
```

---

## 3. 集群健康收尾

```bash
# kubeconfig 给当前用户
mkdir -p ~/.kube && sudo cp /etc/kubernetes/admin.conf ~/.kube/config && sudo chown $(id -u):$(id -g) ~/.kube/config

kubectl get nodes        # 期望 Ready
kubectl get pods -A      # 期望全部 Running
kubectl get sc           # 期望 openebs-hostpath (default)
```

**若节点卡在 `NotReady` 且报 `cni plugin not initialized`**(calico-node 已 Running、`/etc/cni/net.d/10-calico.conflist` 已写,但 containerd 没加载装好后才出现的 CNI 配置)——**重启 containerd 即可**(见 §8③):

```bash
sudo systemctl restart containerd && sleep 6 && sudo systemctl restart kubelet
kubectl get nodes        # 几秒后 Ready
```

---

## 4. 装 KubeSphere(ks-core,OCI Helm chart)

KubeSphere 4.x **不在** `charts.kubesphere.io/main`(那是 3.x 仓库,且对不存在的 `.tgz` 返回 200+HTML 误导人)。真实源是 **OCI**:`oci://hub.kubesphere.com.cn/kse/ks-core`(国内可达;集群此前也从该 registry 拉的 Calico)。

**chart 版本 → KubeSphere 版本**(用 `helm show chart oci://hub.kubesphere.com.cn/kse/ks-core --version <v>` 查 `appVersion`):

| ks-core chart | KubeSphere |
| --- | --- |
| 1.2.1 | v4.2.0 |
| 1.2.3 | v4.2.0-**community** |
| **1.2.4** | **v4.2.1**(本指南装的) |

```bash
helm upgrade --install -n kubesphere-system --create-namespace ks-core \
  oci://hub.kubesphere.com.cn/kse/ks-core --version 1.2.4
# helm 会阻塞等 post-install hook 任务(即便没 --wait);完成后:
helm -n kubesphere-system list                 # ks-core ... v4.2.1 deployed
kubectl get pods -n kubesphere-system          # ks-apiserver/ks-console/ks-controller-manager 全 Running
```

---

## 5. 激活许可证(4.x 硬门槛,社区版免费)

KubeSphere 4.x 登录后会要求激活(社区版和企业版都要)。**社区版许可证免费、可续期**。

1. **Cluster ID = `kube-system` 命名空间 UID**(不是 host Cluster CR、不是 kubesphere-config uid):
   ```bash
   kubectl get ns kube-system -o jsonpath="{.metadata.uid}"; echo
   ```
   (激活页面也会显示这个 ID;以页面/上述命令一致的为准。填错会报 `cluster ID mismatch: expected <kube-system-uid>, got <你填的>`。)
2. 在 https://kubesphere.co/apply-license/ 填表(姓名/公司/**邮箱**/**Cluster ID**/时长)→ 邮箱收证。**一邮箱一证**;重申请用 `name+kse@gmail.com` 这种别名换"另一个邮箱"。
3. 登录控制台 → 在激活页或 **平台设置 → 许可证管理 → 添加许可证** 粘贴 → 激活。

> 装的若是企业版构建(1.2.4)而社区证激活不了,降到社区版:`helm upgrade ks-core ... --version 1.2.3`。本环境实测社区证可激活 1.2.4。

---

## 6. 控制台访问

ks-console 默认是 NodePort **30880**。

```bash
kubectl -n kubesphere-system get svc ks-console      # 80:30880/TCP
```

- 默认账号 **`admin` / `P@88w0rd`**(首登强制改密)。
- **ipvs 模式下 NodePort 不绑 `127.0.0.1`**:本机自测用节点私网 IP,如 `curl http://<节点IP>:30880/`(返回 302→/login 即正常),别用 127.0.0.1(会 000)。
- **⚠️ 若云安全组限制端口 ≤ 某值(如腾讯云轻量/CVM 上限 20000),而 K8s NodePort 固定 30000–32767 —— 默认 NodePort 一个都开不了。** 用 **externalIP + 低端口** 绕开(无需改 apiserver):
  ```bash
  NODE_IP=$(hostname -I | awk '{print $1}')
  kubectl -n kubesphere-system patch svc ks-console --type json -p '[
    {"op":"add","path":"/spec/externalIPs","value":["'"$NODE_IP"'"]},
    {"op":"add","path":"/spec/ports/-","value":{"name":"external","port":18080,"targetPort":8000,"protocol":"TCP"}}]'
  # 然后安全组放行 TCP:18080,浏览器开 http://<公网IP>:18080
  ```
  XGENT 门户以后直接让反代(Caddy)走 **80/443**(均 ≤20000),不碰 NodePort。

---

## 7. 启用扩展(应用商店 / 监控 / DevOps)

KubeSphere 4.x 能力是**可插拔扩展**,在控制台 **扩展中心 / 扩展市场** 按需安装:

- **应用商店(Apps)** —— 装 PG/Redis/MinIO operator 等(仅演示用集群内 infra 时需要)。
- **监控(WhizardTelemetry)** —— Prometheus/Grafana/日志/告警(补 DEPLOY 留的"无指标平台"缺口;7.5G 内存的单节点先按需开,别全开)。
- **DevOps** —— CI/CD(本期 XGENT 不需要,可后续)。

---

## 8. 踩坑速查(本次都真踩到并已解决)

1. **大写主机名** `VM-0-7-ubuntu` → K8s 节点名小写 `vm-0-7-ubuntu`,kk `kubectl label node VM-0-7-ubuntu` 报 NotFound、playbook 在装 CNI 前中止。**先 `hostnamectl set-hostname` 小写 + 改 /etc/hosts,再装**。
2. **`kk delete cluster` 太浅**(不清 `/etc/kubernetes/pki`、不杀全旧 pod)→ delete+重建会叠在半死集群上(旧 kube-proxy 残留→证书错→ipvs 不编程→ClusterIP `10.233.0.1:443` 超时→Calico operator 连不上 API→NotReady)。重装前**深度清理**:`sudo rm -rf /etc/kubernetes /var/lib/kubelet /var/lib/etcd /etc/cni/net.d /var/lib/calico /run/calico /opt/cni ~/.kube && sudo ipvsadm -C`。
3. **containerd 不重载装好后写入的 CNI 配置** → 节点 NotReady「cni plugin not initialized」(即便 calico-node Running、conflist 和 /opt/cni/bin 都在)。**`systemctl restart containerd`(再 kubelet)** → 秒级 Ready。
4. **ks-core 不在 `charts.kubesphere.io/main`**(3.x 仓库,假 .tgz 返回 HTML)→ 用 **OCI** `oci://hub.kubesphere.com.cn/kse/ks-core`。
5. **激活 Cluster ID = kube-system 命名空间 UID**(非 host Cluster CR);填错报 `expected <uid>, got <...>`。

---

## 9. 接入 XGENT(已落地)

> **XGENT-on-KubeSphere 已端到端跑通**(真集群 K8s 1.34.3)。完整部署/运维手册见 **[`kubesphere.md`](kubesphere.md)**;规划与进度见 `goal/KubeSphere.md`。本节只留集群侧要点。

集群 + KubeSphere 就绪后:

1. **企业空间/项目**:Workspace `xgent` + namespace `xgent-prod`(带 `kubesphere.io/workspace: xgent` 标签,控制台可见)。已建。
2. **镜像仓库**:Harbor 仍 TODO;本期用 **in-cluster `registry:2`**(单节点无 docker → **kaniko 节点原生构建**,见 `kubesphere.md §11.2`),并加了 **token 认证接 Portal service account**(`deploy/k8s/registry-auth/`)。
3. **装平台**:`helm upgrade --install xgent deploy/k8s/chart -n xgent-prod -f <values> --set image.tag=<tag>` → pre-install hook 跑 migrate→bootstrap → 平台三件套 Running + 6 个 `replicas=0` 后端。
4. **按需部署**:租户装 qbank → controller scale lms+qbank `0→1` → migrate → provision → health → `ready`,LMS 字典非空。
5. **暴露**:本期用 in-cluster 校验;公网走 ingress/LB 或复用控制台同款 Caddy hostNetwork 80/443(§6),TLS 终止点见 `kubesphere.md §6`。
6. **仍 TODO**:Harbor、监控/日志(WhizardTelemetry)、公网 ingress。
