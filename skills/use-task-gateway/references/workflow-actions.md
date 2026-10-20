# workflow 引擎专用动作面

仅当实际接入的是平台 workflow 引擎时采用。身份要求同时满足 `kind=service`、门户归因 `azp=workflow`、`task_gateway.workflow_actions`。其他 App 即使持有此 scope 也不能调用；不能更改请求体/metadata 伪造身份。普通 App 使用 [task-api.md](task-api.md)，不要为获取 lease 或 abort 而冒用引擎。

使用已有 workflow SA 请求门户 `/api/tokens/service`，scope 为 `task_gateway.workflow_actions`，tenant_id 从已授权流程记录取得。SA audience 可以是 `workflow-server`，无需改成 task-gateway；该面另外检查 azp。服务态不带 admin role，建服务/发节点令牌由管理员准备。

## 路由与字段

基址同 Portal Gateway HTTP；字段采用 camelCase。

| 方法与路径 | 要点 |
| --- | --- |
| GET `/workflow/v1/actions` | `data.actions` 为当前动作目录；从中选择真实 key、版本和输入/输出字段 |
| POST `/workflow/v1/actions/{key}/submit` | 必须有 Idempotency-Key 头，body submissionKey 不能代替 |
| GET `/workflow/v1/actions/{key}/status?submissionKey=...&businessKey=...` | 用提交头的值回查；业务键复用时也带 businessKey |
| POST `/workflow/v1/actions/{key}/cancel` | body submissionKey，可带 businessKey、abortIfRunning |
| POST `/workflow/v1/actions/{key}/preview` | 预览/部分输入校验，不能证明实际执行一定成功 |
| POST `/workflow/v1/leases/{leaseKey}/release` | 整个流程终结后释放本实例 lease |

根据目录实际输入构造，例如某动作要求 inputs.service 时：

```json
{
  "submissionKey": "wf:instance-001:step-build:attempt:1",
  "businessKey": "instance-001:step-build",
  "inputs": {"service": "ci-runner", "repository": "<已授权仓库>"},
  "metadata": {"run_id": "instance-001"},
  "placement": {"leaseKey": "instance-001", "mode": "prefer", "leaseTtlS": 7200},
  "operationVersion": 1
}
```

这只是形状示例，不宣称目录存在名为 ci-runner 的服务或通用 repository 字段。真实 action 定义输入；runner dispatch path 常为 `inputs.service`，与普通面的顶层 service 不同。

相同 submissionKey 按租户和动作回放原 attempt，原任务已失败也不变。新 submissionKey 配同 businessKey 时，当前查重只复用非 failed 任务：同摘要回放，异摘要 BUSINESS_KEY_CONFLICT；原任务 failed 后允许同 businessKey、新 submissionKey 创建新 attempt。执行重试仍须遵守 retryable/业务策略，不自动重跑已取消或中止的工作。

businessKey 的提交查重范围实际为 `(tenant, target service)`，摘要含 action key，因此共享目标服务的不同动作不要使用相同 businessKey，建议包含实例和步骤标识。status/cancel 的业务键回退查询另限定当前动作，精确 submissionKey 优先。新 submissionKey 复用了旧任务时，同时带 businessKey 回查/取消，避免仅按新键查不到任务。

当前部分目录项设置 `validate_inputs=true`，submit 和 preview 都会校验必填、声明类型以及 service 一致性；其他目录输入仍主要由执行器校验。旧文档中“preview 永远只回显”已经过时，也不能反过来假设它对所有 action 完整校验。operationVersion 给了就检查目录 supportedVersions，省略则跳过该版本闸。

动作目录的目标服务可为私有平台服务；普通 `/v1/tasks` 和 gRPC 不作为绕过目录授权的入口。目录项退役后停止新提交/preview，既有任务 status/cancel 继续可用。

## 状态、失败和中止

提交响应取 `data.taskId`，state 可为 accepted/completed/failed/suppressed；回查 state 为 pending/running/completed/failed/suppressed/unknown，不能套用普通 HTTP 六态。pending/执行中按 `data.retryAfterS` 等待（当前分别约 10/5 秒，流式约 3 秒）；终态不带该提示。unknown 不等于执行失败，检查 submissionKey/businessKey 与业务记录。

业务调用失败仍为 HTTP 200 + ok:false。任务 failed 的 `reason/code/retryable` 在 data 顶层，是否重做遵循 retryable 与本 App 策略；`nodeError` 是执行器自报错误，独立于网关 code，不能拿它冒充网关撤回/中止信号。终态 result.artifacts 若为数组，会原样提升到 data.artifacts；网关不存储或验证产物，文件由 runner 自行上传。

| cancel 请求 | 结果 |
| --- | --- |
| 只有 submissionKey，任务 pending | state=cancelled；后续 status 为 failed + code=INVALID_STATE + retryable=false |
| 只有 submissionKey，任务已领取 | ok:false + INVALID_STATE |
| 带 abortIfRunning=true，任务已领取 | 立即记终态并回 state=aborting；status 为 failed + code=ABORTED + retryable=false |
| 已终态 | 通常 INVALID_STATE；已中止请求的重放可回 aborting |

中止信号通过下一次节点心跳送达，新 agent 终止对应进程树或不启动尚未执行的任务；旧 agent 可能继续执行，迟到结果被拒绝。aborting 响应不证明物理进程已停止；取消/中止不当成可重试执行失败。流程终态通知依赖回查；body.callback 上下文不是已实现的可靠终态推送。

## placement / lease

解析顺序：活跃 lease 的健康节点 → affinityKey 一致性哈希 → 共享队列。prefer 默认允许回退；require 缺 leaseKey/affinityKey 时 VALIDATION_FAILED，有 leaseKey 但 lease/哈希均无法派发时返回 PLACEMENT_UNAVAILABLE + error.details.retryable=true。仅有 affinityKey 的哈希路径即使 require 也可回退共享队列，不能当强制粘连。哈希随节点池变动，不代表固定物理机器。

leaseTtlS 默认 7200，钳制到 300–86400；带同 lease 的提交及回放会尽力单调延长 TTL。长审批窗口要覆盖等待期；TTL 过后不可承诺现场仍在。placement.nodeId 为实际/定向节点，reused=true 仅表示复用了活跃租约路由，不是现场存在证明。

runner 需要稳定 agent_id 和 workspace_root 才使用 lease 工作目录。并发同 lease 共享目录，lease 不提供步骤互斥；引擎保证写操作顺序。流程整体结束后 release；释放范围为同租户 leaseKey 的所有 active 服务行，未知 key 返回 released=false 且成功，不是 404；不要每个步骤结束就释放后续还要使用的现场。

当前实现维护入口：`gateway/src/portal/workflow_actions/{handlers,dto,catalog,placement,idempotency}.rs`。目标部署的目录与能力以实际接口为准，不复制历史一盒的动作列表、镜像版本或固定 CI 服务名。
