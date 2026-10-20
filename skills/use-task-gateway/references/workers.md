# 执行服务与 VM 对齐

仅在需要新增执行能力或排查执行端时使用本参考。调用已有服务的 App 只需掌握其 payload/结果契约，不必安装 agent。

## 服务与机器的分工

App 调用 `service_name`，节点反向领取该服务队列；普通任务请求没有用于直接指定机器的 `host_id/node_id/cwd`。确需某 VM 上的本地仓库/CLI 会话时，可将专门服务只部署到该 VM，并保存业务工作区到服务名的绑定。现有服务亲和配置可影响分配，但普通 metadata 不是任意节点派发接口。

租户管理员创建 `POST /v1/services`（`task_gateway.admin` + 用户 admin role），填写展示名、描述、可选 template_id 和超时；保存服务器返回的真实 name，创建体中的 name 不用于指定物理名。服务 `provisioned=false` 时用 `POST /v1/services/{name}/provision` 修复队列，无需重复创建。

手工接入由管理员用 `POST /v1/node-tokens` 为该机的完整服务集合签发 Node Token，凭据只展示一次。

先确认服务在线节点，再验证实际任务。缺 VM、依赖服务或容器时请求用户/运维提供环境，不擅自拉镜像或安装。

## SSH 自动部署的能力确认

需要新增机器且选择 SSH 自动部署时，先用具有 `task_gateway.read` 的 TDT 调用目标 Gateway 的 `GET /v1/onboarding`。HTTP 成功且信封 `ok:true` 后，读取 `data.auto_deploy`；鉴权或请求失败不能当成能力关闭。`auto_deploy=false` 时向网关维护方提供实际环境和版本，按用户选择使用手工接入或等待开通；不要把“接异步任务”默认扩展成机器部署。

1.9.3 的打包配置默认启用 SSH 部署，根密钥由**网关自己的 manifest** 声明生成，并由支持该能力的门户在同一部署复用。调用方 App 不声明或持有网关根密钥。正常开通和升级由网关维护方通过 manifest、新镜像与发布链完成，不要求管理员登录系统手改环境，也不能通过重新生成存量密钥绕过故障。目标门户和网关仍须完成对应升级与发布；新版 skill 或本地 onebox 验证通过不代表线上能力已开启。

能力确认后仍需准备目标服务/模板，并由机器所有者安装租户 SSH 公钥、核对目标主机指纹。自动部署会按保存快照生成完整 agent 配置并重启该机 agent，可能覆盖手工 YAML；当前服务 PATCH 只改展示名/描述，不是 executor 更新接口。五步向导可见只证明入口可用，实际部署成功还须确认 agent 上线并完成真实任务。

## 最小多服务格式配置

以下单服务配置使用当前 `services[].executor.kind` 格式；所有名称/路径均为需要替换的示例。Node Token 由 agent 进程环境注入，机器只需出站连接 gRPC，不走 Portal `/svc`。

```yaml
agent_id: "report-vm-01"
node_token: "${XGENT_NODE_TOKEN}"
service_key:
  path: "service"
gateway:
  address: "gateway.example.com:50051"
  tls: true
services:
  - name: "report-worker-a1b2c3-d4e5"
    max_concurrent_tasks: 1
    executor:
      kind: cli
      command: ["/usr/bin/python3", "/opt/task-handlers/report.py"]
      input_mode: stdin
      timeout_secs: 300
      cwd: "/srv/report-work"
    response:
      success:
        body: "<stdout>"
      failed:
        body: "<stderr>"
      max_bytes: 1048576
```

agent_id 应稳定，不在每次重启时随机生成，尤其使用 lease 时。Node Token 必须允许配置中的所有服务且租户归属相符。开发明文 gRPC 可按实际部署将 tls 设为 false；生产使用平台给定 TLS 地址和信任配置，不默认跳过验证。

服务 `task_timeout_secs` 应覆盖执行器 timeout 并留出回传余量。App 的 HTTP/观察超时不替代上述执行时限。

## payload 与执行模式

`service_key.path` 是从提交 payload 读取服务名的 dot-path。本例要求 `payload.service` 等于服务真实 name；该校验发生在执行侧。stdin 只收到整个 payload，HTTP metadata 不自动并入 stdin。需要 operation/report_id/workspace_id 等参数时放入 payload，并在适配器中校验。

| executor kind | 适用能力 |
| --- | --- |
| `cli` | 一次非交互命令执行，退出码 0 成功；按 response 模板返回 stdout/stderr |
| `sync-api` | agent 调用已有同步 HTTP 服务 |
| `async-api` | agent 提交下游异步 API 并按配置轮询下游 |
| `streaming-cli` | 持续转发 CLI stdout，App 提交需 streaming=true |
| `streaming-sync-api` | 消费下游流式 HTTP 响应 |

这些是执行端模式；App 到 Gateway 的提交/回查始终可以是异步 HTTP。不要因 App 要异步，就要求下游必须用 async-api。采用 API executor 时，以实际下游提交/轮询/流格式补配置；当前 schema 维护入口是 `gateway/src/agent/config.rs`，不要把 CLI 示例直接改 kind 当作完整 API 配置。

CLI 适配器解析 JSON 后选择已配置能力，使用 argv 调用固定程序；网关不会自动执行 payload.command。用户传工作区逻辑 ID，再映射允许目录，避免直接采信任意绝对路径或 shell 文本。`cwd` 与 executor.env 是本机启动配置，不按任务 payload 动态求值；`<payload.prompt>` 当前会产生带引号的 JSON 字符串，不能当已解码 argv，复杂输入优先 stdin。

仓库、工具、模型登录/凭据、产物上传均由 VM/App 准备；systemd 运行环境不会自动继承交互终端 export。模型密钥不放任务 payload，执行输出控制在 response.max_bytes 内，大文件上传对象/文件服务后返回引用，本地路径不自动变下载链接。

需要流式 CLI 时，在 executor 内设置：

```yaml
kind: streaming-cli
# command/input_mode/timeout_secs 等 CLI 配置仍须提供
streaming:
  response_format: lines
```

真实进程需换行并 flush。当前 streaming-cli 按原始 stdout 行转发，不按 content_path 自动提取某个 CLI 的 JSON 事件；NDJSON 等要由适配器/App 解析。

`max_concurrent_tasks` 按单服务、单 agent 限制，多服务仍可并行；同仓库写入须调用方串行化、加锁或使用独立 worktree。Gateway 不创建/同步仓库、提供交互终端或管理 coding-agent 会话；task_id 不是 CLI session_id，多节点也不共享本地会话。App 等待超时、普通 cancel 拒绝、乃至任务已 failed，都不能单独证明远端所有进程和外部副作用已停止；严格生命周期由执行适配层管理并验证。
