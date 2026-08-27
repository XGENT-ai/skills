---
name: xgent-image-push
description: 把 App 镜像构建并推送到自建的私有 Harbor 镜像仓库（地址由本地配置提供，不写在这个 skill 里）。用户提到发版、上线新版本、发布镜像、推镜像、docker push、打 tag 推仓库、配 CI 的镜像构建推送步骤、私有镜像仓库、Harbor、robot 账号推不上去、push 卡住推不动、ImagePullBackOff 之前的推送环节，都用这个 skill —— 哪怕他们只说"发个版"或"把这个服务的镜像推上去"没提仓库名字。它在推之前自动预检四条会让流水线静默失败的硬约束（链路、镜像架构、tag 不可变、保留策略），绕过它直接 docker push 很容易踩中其中一条。
---

# 推镜像到私有仓库

## 仓库地址从哪来（**不在这个 skill 里**）

仓库域名属内部信息，没有硬编码在这里。脚本按顺序找第一个存在的配置文件：

```
1) --config <path> 或 $XGENT_REGISTRY_CONFIG
2) ./.xgent-registry.env                        ← 项目内，要加进 .gitignore
3) ${XDG_CONFIG_HOME:-~/.config}/xgent/registry.env
4) ~/.xgent-registry.env
5) <skill 目录>/registry.env                    ← 同目录 registry.env.example 是模板
```

文件是 `KEY=value`，必填 `REGISTRY`（域名，不带协议）和 `PROJECT`（Harbor 项目名）。
**同名环境变量优先**，所以 CI 里直接注入 `REGISTRY` / `PROJECT` 即可，不必落盘。

找不到配置时脚本会停下来并告诉用户怎么建 —— 这时**不要猜域名，也不要从别处翻**，
让用户去问运维要，或指给他们 `registry.env.example`。

| | |
| --- | --- |
| 完整引用 | `<REGISTRY>/<PROJECT>/<app>:<tag>` |
| repository 名 | 就是 App 名，问运维要，别自创 |
| 推送账号 | `robot$<PROJECT>+<app>`，密钥在 CI secret 里 |

**推的人只推一个地方**：配置里的那个 `<REGISTRY>`。写入口只有这一个，
别自己另外指一个地址，也不要为了"保险"往第二个地方推一遍。

⚠️ **拉取方看到的 host 不一定和你推的这个一样**，但**路径部分完全相同**
（`<PROJECT>/<app>:<tag>`）。分发是运维侧的事，对推送方没有任何影响：
一次 push、一个 tag 就够，你不需要知道对方从哪拉。

⚠️ **推完到部署侧能拉到之间可能有一段延迟**（通常几十秒，大镜像更久）。
这段时间里对方拉到的是 `manifest unknown` —— 是"还没到"，不是"慢"。
⇒ 别 push 完立刻催部署；更**不要因为立刻拉不到就重推、或者换个 tag 再推一次**，
那只会在仓库里留下一堆无人使用的 tag，还会挤占保留策略的名额（见 ④）。

## 工作流

### 1. 先把三件事问清楚（不要猜）

- **哪个 App** → 决定 repository 名，写错会报 `denied`
- **什么 tag** → 规范是 `v<MAJOR>.<MINOR>.<PATCH>-<7位 git sha>`（例 `v1.4.0-1a2b3c4`，详见 ③）。
  sha 那半截是机械的，`git rev-parse --short=7 HEAD` 就行；
  **版本号必须问用户** —— 不要从 git tag、`package.json`、`VERSION` 里猜，也不要自作主张
  给 patch 位 +1。版本号进的是他们的发布历史，由他们决定；你猜错了没人会当场发现。
  用户说不清就直接问"这次发 v 几"。tag 不可变，推错了只能换一个新的
- **凭证从哪来** → `ROBOT_USER` / `ROBOT_SECRET` 环境变量，或 CI secret。**不要**让用户把密钥贴进对话；已经贴了就提醒他们事后轮换

### 2. 用脚本推（首选）

脚本就在这个 skill 目录下的 `scripts/push-image.sh`（用绝对路径调用，别假设当前工作目录）：

```bash
ROBOT_USER='robot$<project>+<app>' ROBOT_SECRET="$SECRET" \
  <skill 目录>/scripts/push-image.sh <app> <tag>
```

地址与项目名由上面的配置提供，命令行里不用写。

脚本按顺序做：链路预检 → tag 冲突检查 → 保留策略预警 → login → build（锁 amd64）→ 架构核对 → push → 验证 → logout。
任何一项不过就停在那里，不会把时间浪费在注定失败的 push 上。

常用参数：`--config <path>` · `--context <dir>` · `--dockerfile <path>` · `--no-build`（镜像已在本地）· `--dry-run` · `--skip-link-check`
（先跑 `--dry-run`：只做只读检查，不 login/build/push。）

这个 skill 是自包含的：`SKILL.md` + `scripts/push-image.sh` + `registry.env.example`，
除了 `docker` 和 `curl` 不依赖任何外部脚本或仓库。整目录复制到别处，再补一份本地配置即可使用。
⚠️ 填好的 `registry.env` / `.xgent-registry.env` **不要提交**（skill 目录自带 `.gitignore` 挡了这两个名字）。

### 3. 没有脚本时的等价手工步骤

先从配置里取到 `REGISTRY` / `PROJECT`（别把域名写死进 CI 脚本）：

```bash
IMAGE="${REGISTRY}/${PROJECT}/${APP}:${TAG}"
# 注意单引号 —— robot 名里的 $ 会被 shell 展开成空
echo "$ROBOT_SECRET" | docker login "$REGISTRY" -u 'robot$<project>+<app>' --password-stdin
docker build --platform linux/amd64 -t "$IMAGE" .
docker image inspect "$IMAGE" --format '{{.Os}}/{{.Architecture}}'   # 必须 linux/amd64
docker push "$IMAGE"
docker logout "$REGISTRY"
```

用 `--password-stdin`，不要用 `-p` —— 后者会把密钥留在 shell history 和 CI 日志里。

### 4. 报告结果

推完告诉用户：完整镜像引用、digest、当前 repo 里有几个 tag（接近 10 个要提醒）。
**不要**替用户去改部署侧的「镜像仓库地址」或碰编排层 —— 那是部署/运维侧的事，见下方边界。

## 四条硬约束

这四条是实测踩出来的，前两条的失败表现都不像"推失败"，所以值得在动手前就挡住。

### ① CI 必须与仓库同地域，不能跨境推

跨境链路上的**推送**实测 **49 KB/s**、RTT 440ms、丢包 35% —— 一个 2.93GB 的镜像要 17 小时。
（2026-08-11 在生产机上实测。⚠️ 这是**那一天、那条路**的数字：仓库搬迁或链路调整后要重测，
否则它会从论据变成过时的断言。）
表现是 `docker push` 卡在 `Retrying in N seconds` 直到超时，日志里看不出原因，重试也没用。
**这条没有绕过办法**，只能换 runner 位置。

脚本量 **TLS 握手耗时**做判断（默认 >300ms 视为跨境）。用 TLS 而不是 TCP 握手是因为
挂代理时 TCP 只连到本地代理、量不出真实距离，而 TLS 握手是与仓库本体完成的，穿过隧道也照样反映距离。
它是代理指标不是真吞吐，但把 440ms 和同地域的几十毫秒分开绰绰有余。
用户坚持要试可以 `--skip-link-check`，但要先把上面的数字讲清楚。

拉取方向是好的（同一条跨境链路上 7.2 MB/s），所以"我能 pull 说明网络没问题"不成立 —— 推拉不对称。

### ② 镜像必须是 `linux/amd64`

生产机是 x86_64。Apple Silicon 上直接 `docker build` 出来是 arm64，
**推得上去、拉得下来，容器起不来**（`exec format error`）—— 看着像仓库坏了，其实不是。
所以构建要显式 `--platform linux/amd64`，推之前再 inspect 核对一次实际架构。

### ③ tag 不可变，且要符合规范

部署侧按镜像引用的变化触发重建，**覆盖同一个 tag 不会触发重建** —— 线上还跑着旧的那份，
而用户以为发上去了；出问题时也没有可回退的目标。所以脚本发现 tag 已存在会直接拒绝。
真要覆盖得 `--force-tag`，并且要让用户明白重建不会自动发生。

**规范：`v<MAJOR>.<MINOR>.<PATCH>-<7位 git sha>`**，例 `v1.4.0-1a2b3c4`。
正式发版可以省掉 sha 那半截（`v1.4.0`）。

一个 tag 要同时扛三件事，这个格式是能同时满足的最简形式：

| 要求 | 谁来满足 | 不满足会怎样 |
| --- | --- | --- |
| **唯一** —— 每次构建都是新 tag | sha | 覆盖旧 tag ⇒ 不触发重建（见上） |
| **可追溯** —— 能倒查回 commit | sha | 线上出事，查不到这版是哪份代码 |
| **可读可排序** —— 一眼看出谁新谁旧 | 版本号 | 保留策略删最老的、回滚要指名上一个，光看 sha 判断不了顺序 |

**版本号怎么排是推送方自己的事** —— 这个 skill 只要求 tag 的**形状**（唯一、可追溯、可排序），不规定什么时候进 minor、什么时候进 patch。别替他们决定。

不带 tag 参数跑脚本，它会把当前 HEAD 的 sha 打出来让你拼——**版本号那半截它不猜**。

**禁止的 tag**（脚本硬拒）：`latest` `prod` `stable` `dev` `test` `main` `master` `rc` 等。
这些是**可变指针**语义 —— 一旦有人推了 `latest`，「引用没变但内容变了」就成立，
整条不可变的前提当场失效。

#### 例外：不经部署侧的项目可以开 `latest`（2026-08-27）

上面那条推理链的每一环都挂在「部署侧按镜像引用触发重建」上。**给人手工 `docker pull`
的镜像**（本地调试环境那一类，通常放在单独的项目里）没有部署侧、没有重建、没有回滚目标 ——
链断了，禁令的前提也就不在了。反过来还缺了点东西：新人第一次拉的时候，
**他还没有渠道知道有哪些 tag**，`latest` 补的正是这个洞。

配置里逐个列名打开（不是总开关，打错的 `prod` / `main` 仍然被拦）：

```
MUTABLE_TAGS=latest
```

⚠️ **开了也不许 build 完直接推成 `latest`**，脚本会拦。可变 tag 只能"移"：

```bash
push-image.sh <app> v1.2.3-1a2b3c4                     # ① 先推不可变的那一份（真镜像）
push-image.sh <app> latest --point-at v1.2.3-1a2b3c4   # ② 再把指针移过去
```

第 ② 步是 OCI 分发规范里的**重打 tag**：把 `v1.2.3-1a2b3c4` 那一份的 manifest 原始字节取下来，
一个字节不改地 `PUT` 到 `latest` 上。digest 由字节算出来 ⇒ **必然与源相同** ⇒ 落在
**同一个 artifact** 上（不新增 artifact、不占保留策略名额、不动任何 blob，跨境也是一瞬）。
纯 registry API，**不需要 docker，也不 login**。移完会再拉一次服务端的 digest 核对。

⚠️ 别拿这两条顶替：
- `docker pull + docker tag + docker push` —— 要把镜像字节拉下来再传上去（跨境 49KB/s），
  而且 `pull` 只取本机架构那一份，多架构 manifest list 会被**压扁**成单架构。
- `docker buildx imagetools create` —— 源是**单架构 manifest** 时它会包一层 index，
  digest 变了、Harbor 里多出一个 artifact。本脚本推的正是单架构镜像，会踩上。

为什么不许直接推：仓库的 GC 开着 `delete_untagged`。`latest` 一移走，上一个 digest
如果没有别的 tag 就成了 untagged，下一次 GC 连底层 blob 一起回收 ⇒ 这个 repository
**永远只剩一份镜像**，既没有历史也没有可回退的目标。这个后果不可逆，所以那条 die
没有留 `--force` 之类的绕过口子。

⚠️ 还要提醒拉取方：`latest` 变了他们**不会知道**。本地已有同名 `latest` 时
`docker run` / compose 默认不回仓库查 —— 让他们开工前先 `docker pull`，
要钉住某一版就用带版本号的 tag。

**脚本还会核对 git 状态**，两条都是踩过的坑：

- **工作区不干净直接拒绝**（`--allow-dirty` 放行）。带着未提交的改动构建，tag 指向的 commit
  **复现不出这个镜像** —— 出事时你会对着一个查不到的版本。
- **tag 里的 sha 与 HEAD 不符时告警**。推错 commit 比推错 tag 更难发现：tag 看着挺对。

团队统一格式之后，把 `TAG_PATTERN`（正则）写进配置就从"提醒"升级为"强制"。
不配的时候只提醒不拦，是因为各团队可能还在用历史格式，不该由这个脚本单方面掐断发布。

### ④ 每个 repository 只保留**最近推送的 10 个** tag

保留策略每天 02:00 跑，GC 每周日 03:00 回收存储。判据是"最近 push 的 10 个"，
**不是"最近在用的 10 个"** —— 连着推 11 个测试 tag，会把**生产正在跑的那个挤掉**，
当时毫无征兆，等到 Pod 重调度、扩容或新节点加入那天才 `ImagePullBackOff`。

⇒ 调试镜像不要往生产 repo 推。tag 数接近 10 时提醒用户确认生产在跑的 tag 不在最老的那几个里。
需要更大的保留数要找运维改 `RETAIN_LATEST_K`，不要自己想办法绕。这个 registry 不是归档仓库。

## 报错对照

| 症状 | 是什么 | 怎么办 |
| --- | --- | --- |
| push 卡住 / 一直 `Retrying in N seconds` / 几十 KB/s | runner 与仓库跨境（①） | 换同地域 runner。不是仓库故障，重试无效 |
| `unauthorized: unauthorized to access repository` | 没 login / 密钥错 / robot 被吊销或轮换过 | 重新 login；仍失败找运维查 robot 状态 |
| `denied: requested access to the resource is denied` | 路径写错 —— 项目名必须是配置里的 `PROJECT`，repository 必须是 App 名 | 核对 `<REGISTRY>/<PROJECT>/<app>:<tag>` |
| `x509: certificate signed by unknown authority` | 本机 CA 库或系统时间 | 服务端是正式证书（含中间证书），**正常机器不需要 `--insecure-registry`**；配了是掩盖问题 |
| 拉得到但 `exec format error` | 架构不对（②） | `--platform linux/amd64` 重建重推 |
| 之前推过的老 tag 不见了 | 被保留策略挤掉（④） | 重推。长期留存另想办法 |
| 推到一半断了 | 重推即可，已推上去的层会跳过 | ⚠️ 别反复中断大镜像：每次中断在对象存储留未完成分片（运维侧配了 7 天清理） |

## 密钥怎么处理

- 只从环境变量/CI secret 取，**不写进文件、不打印、不进 git**
- 用完 `docker logout` —— 别把凭证留在共享 runner 的 `~/.docker/config.json` 里
- ⚠️ Harbor 的 robot 权限粒度是**项目级**（已知偏差，做不到只到一个 repository）：
  密钥泄露 = 项目下**所有 App 的 repository** 都能被推/拉。所以不要放公共 runner、不要写进共享脚本。
  能推别人的 repository 不等于可以推 —— 只推自己那一个
- 轮换/吊销找运维；轮换会让旧密钥**立刻失效**，CI 换上新密钥前会一直红，要约时间

## 边界：这些不归推镜像的人管

- 部署侧的「镜像仓库地址」设置、切换时机与停机窗口 —— **部署侧**
- 部署侧的**拉取**凭证 —— **运维侧**。你手里那份是**推**的，两回事
- registry 的可用性、证书、容量、GC —— **运维侧**
- 运维侧原有的镜像分发方式（如果之前有）：切到 registry 之后**不是降级路径** ——
  仓库地址一非空，镜像名就成了完全限定名，旧的那条分支被永久跳过。别拿它当兜底

**已经在生产跑着的 App**（现在从别处拉镜像，只是要把仓库地址切到这里），第一次接入有一步容易漏：
除了推新版本，还要**把当前在跑的那个 tag 原样推一份进来**，
否则部署侧把镜像源切过来的那一刻拉不到同名 tag，切换即故障。

**还没上过生产的新 App 没有这一步** —— 不存在"在跑的 tag"要补，推你自己的版本即可。
