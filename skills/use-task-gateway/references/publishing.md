# 服务提供方发布执行能力

由服务提供方在自己的仓库维护契约和发布材料，并对执行能力负责。Task Gateway 提供队列、节点认证、有限准入、执行状态与目录机制；Worker 的业务授权、解析/计算、外部副作用和产物持久化由提供方维护。发布服务不需要修改网关产品代码。

## 选择目录与发布身份

| 对象 | 当前发布入口 | 权限与可见性 |
| --- | --- | --- |
| 租户自管服务 | `POST /v1/services`；随后 GET 读回 | `task_gateway.admin` + 用户 admin role；物理 name 由服务器生成，仅该租户拥有和管理 |
| 平台执行服务 | `POST /v1/ops/services`，已有服务用 `PATCH /v1/ops/services/{name}` | `task_gateway.ops` + 运营租户白名单 + 对应服务写 PID；提供真实 name，由平台拥有 |
| Workflow 动作 | `POST /v1/ops/workflow-actions`，已有 key 用 `PUT /v1/ops/workflow-actions/{key}` | 同一 ops 门 + 对应动作写 PID；targetService 必须是已存在、未删除的平台服务 |

提供方有任务授权且具备相应发布身份时执行登记；否则准备完整请求文件、目标环境与所缺权限，交给具备权限的发布者执行。普通 App 的 read/submit SA、Node Token 和 Workflow 引擎票均不授予目录写权限；本 skill 不授权提升权限或修改门户授权。

`GET /v1/services` 是按调用者身份过滤的执行服务列表，不承载完整业务 schema。Workflow 动作目录承载 key、版本、输入/输出字段和能力声明；只有需要引擎调用时才登记动作，不能把租户服务直接当作动作目标。完整业务 schema 和操作白名单由提供方随服务交付并由 Worker 校验。

平台服务的可见性由实际用途决定：`is_public=true` 对有权调用 TG 的租户开放；限定来源 App 时用 `is_public=false` 和 `allowed_origin_apps`，两者互斥。名单比对门户可信 origin_app，不比对 payload 中自报来源；名单不是租户白名单或业务源权限。不要为让服务出现在目录里而自动公开它。

## 提供方的发布交付物

- 固定执行入口和 payload dispatch 路径、输入/结果 schema、操作及版本、错误/重试语义。服务名、Worker 配置与动作 target 必须一致；不从任务拼接任意命令。
- 不可变镜像/可执行文件、目标架构、匹配 agent、必要 helper/运行描述符及安装方式。注册目录、推镜像和安装 agent 都不替代 Worker 与依赖部署。
- 有限节点数、每节点并发、非终态任务量、执行器与任务时限，以及内存/临时空间/结果大小边界。按本服务实测设定，执行器超时给回报留余量，不复制其他服务的容量值。
- Node Token 与业务执行身份分别供给；源读取、产物写入和撤权由业务授权链控制。配置只给受保护文件位置，密钥、票据、签名 URL 不放进目录、任务或共享回执。

## 登记与目标环境读回

先查询现有登记，确认物理服务/动作归属和目标，按需创建或更新；发现同名异目标时报告冲突，不覆盖其他提供方的对象。创建操作没有通用幂等保证：回包未知时先 GET 对账，再决定下一步，不盲目重复 POST。

平台服务登记体包含 `name`、`description`、`is_public` 或 `allowed_origin_apps`、`max_nodes`、`max_nonterminal_tasks`、`task_timeout_secs`。用 ops 服务详情读回实际值、`runtime_scope`、`provisioned` 与节点情况。租户服务的创建/队列修复见 [workers.md](workers.md)，不能用普通创建接口指定平台物理名。

Workflow 动作使用 camelCase，以下仅示意请求形状，不是已登记的服务或通用业务 schema：

```json
{
  "key": "reports.generate",
  "description": "Generate one authorized report",
  "version": 1,
  "supportedVersions": [1],
  "targetService": "<实际平台服务名>",
  "validateInputs": true,
  "capabilities": {"submit": true, "status": true, "cancel": true, "preview": false},
  "inputs": [
    {"key": "service", "type": "string", "required": true},
    {"key": "report_ref", "type": "string", "required": true}
  ],
  "outputs": []
}
```

此例按 `inputs.service` dispatch；普通任务用 `payload.inputs`，Workflow 用 submit 的 `inputs`，须由同一执行器契约实际支持。`validateInputs` 仅覆盖声明字段的必填、类型与 service 一致性，不代替完整 schema、枚举或业务授权。

先通过 ops 单项 GET 读回服务与动作；再用实际调用身份检查 `GET /v1/services`，需要 Workflow 时由真实引擎身份检查 `GET /workflow/v1/actions`。引擎身份约束和请求见 [workflow-actions.md](workflow-actions.md)。写入成功后目录会热加载，不需要为登记动作重新发布网关。

## 执行验收与更新

在本次授权的环境、数据和费用范围内，验证目标节点实际领取、Worker 执行及调用方消费产物；按契约核对关键拒绝、恢复与取消边界。服务 provisioned、节点 healthy、动作可见只证明各自状态，不能替代执行验收。无调用身份或环境时如实交付已读回/未验证项，不伪造闭环完成。

交付回执保留环境、服务名、动作 key/current/supportedVersions、镜像/agent/依赖版本、实际容量与可见性、非秘密身份摘要、真实任务及产物消费结果。调用方据此绑定目录能力，无需接手提供方的 Worker 安装与维护。

提供方维护后续版本和停用。动作 key 的 targetService 不可变，改目标须用新 key；更新保留旧任务所需的 status/cancel 能力，DELETE 仅退役并停止新提交，既有任务仍可查询/取消。镜像发布、目录登记、业务默认切换分别按当前任务授权执行，不因完成其中一项自动推进其余项。
