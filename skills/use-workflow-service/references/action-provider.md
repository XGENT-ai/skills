# 提供动作执行面

仅在要让 workflow 节点执行本 App 业务动作时读取。普通发起／发事件无需实现这里的协议。若已有 Task Gateway 的注册服务和执行器，优先复用其 workflow 动作目录，不另造执行面；普通 App 不能直接调用网关的引擎专用 `/workflow/v1/*`。

## 布线与身份

执行方的 Portal App 暴露 `<自身服务根地址>/workflow/v1/*`。平台需为目标租户安装该 App，并在 workflow 运行配置 `WORKFLOW_ACTION_TARGETS` 登记 target／baseUrl；baseUrl 缺省时由 `PORTAL_BASE_URL` 推导 `/svc/<target>`。Workflow 的服务账号还须实际获授 `<target 下划线化>.workflow_actions`，相应清单权限／命名空间由平台批准生效。只写一个 URL 不会自动授予调用权限。

入站验证用执行方自己的 SA 自省：`POST <PORTAL_INTROSPECT_URL>`，Basic 自己的 clientId:secret，JSON `{token}`。解包 `claims=body.data ?? body`；要求 `active=true`、非空 tenant、`kind=service`、`azp=workflow`、目标 scope。自省失败返回 503 并拒绝访问；无效票 401，身份／scope 不符 403。**服务票不按 aud 等于执行方 key 校验**，其 audience 是来源 SA clientId。所有业务查询仍按自省租户收口。

服务票仅授予引擎调用资格；`actor` 和 `approvals` 是归因与业务校验输入。执行方必须核对本 App 业务 ACL 和权威对象，不能因 `azp=workflow` 就把所有业务操作视为已获用户授权。当前 `credential` 委托槽未提供用户授权凭据，不应依赖它存在。

## 路由与形状

统一返回 `{ok:true,data:...}` 或 `{ok:false,error:{code,message}}`；HTTP 200 也可能拒绝。实际执行失败应作为 `data.state=failed`，并给 `code,reason,retryable`；不要把已有执行结论变成无法回查的接入错误。

| 方法与路径 | 契约 |
| --- | --- |
| `GET /workflow/v1/actions` | 当前租户可用动作目录 |
| `POST /workflow/v1/actions/{key}/submit` | 接收引擎提交；`Idempotency-Key` 头等于 body.submissionKey；15 秒内回答，长任务回 accepted |
| `GET /workflow/v1/actions/{key}/status?businessKey=...&submissionKey=...` | 必须回显 submissionKey；查询零副作用 |
| `POST /workflow/v1/actions/{key}/cancel` | body `{businessKey,submissionKey,externalRef?}`；幂等、如实报告撤回结论 |
| `POST /workflow/v1/actions/{key}/preview` | body `{inputs,actor}`，可兼容可选 approvals；零副作用 |
| `POST /workflow/v1/leases/{leaseKey}/release` | v1.1 可选租约面，见下文 |

目录 data 的顶层为 `{version:"1",actions:[...]}`；每项至少包含 `key,version,supportedVersions,name,inputs,outputs,capabilities,cost,retryable,mustHuman`。`name` 用 `{zh,en,tw}`；输入形状 `{key,type,required,label}`，输出形状 `{key,type}`；capabilities 为 `{preview,cancel,status,subjectCheck}` 布尔值；cost.kind 为 `none|credits|money`。只列可用动作，未安装回业务 `APP_NOT_INSTALLED`。能力版本用整数，旧版本语义必须保留；不支持 operationVersion 时回 `OPERATION_VERSION_UNSUPPORTED`，不能静默按新语义执行。

submit body 由引擎提供：

```json
{
  "submissionKey": "<不透明提交键>",
  "businessKey": "<不透明业务键>",
  "operation": "<与路径 key 相同>",
  "operationVersion": 1,
  "inputs": {"objectRef": "<业务对象引用>"},
  "actor": {"userId": "<发起人或 service:来源>", "kind": "user"},
  "approvals": [],
  "callback": {"instanceId": "<UUID>", "eventName": "action.terminal"},
  "definition": {"defId": "<UUID>", "versionNo": 1}
}
```

`actor.kind` 为 user／service，userId 以 `service:` 开头表示非自然人，不能当用户 ID 查权限。敏感明细留在业务 App，inputs 只传必要引用和展示摘要。submit 的 state 为 `accepted|completed|failed|suppressed`，可回 `externalRef,result,artifacts,reason,retryable,code`。`suppressed` 是业务主动抑制，不能当技术失败自动重试。

status 的 state 为 `pending|running|completed|failed|suppressed|unknown`。**unknown 仅表示没有该提交记录**；后端抖动须报不可用，不能返回 unknown。任何已产生效果的提交必须可回查。即使目录声明 `status:false` 或 `cancel:false`，当前引擎仍会调用这两个面，不能留成 404。preview 只在支持时调用，返回 `{verified,plan?,estimatedCost?,reason?}`；未实查回 `verified:false` 及原因，不把未知费用写成零。

## 两级去重与撤回

- **submissionKey**：同 attempt 重传保持不变，按租户持久化提交记录；回放原结果，不重复执行。
- **businessKey**：跨 attempt 不变，用于防第二次业务效果；新 submissionKey 也不能重复退款／发布。两个键都按不透明字符串存取，不解析内部结构做业务判断。
- 网络超时不证明未执行；新 attempt 前应能查回旧结果。status／终态回调必须带正确 submissionKey，迟到旧 attempt 不能落定新 attempt。
- cancel 回 `{state:"canceled"}` 才表示撤回成功且未产生效果；已经有结论时回真实 `completed|failed|suppressed`。成功撤回后同 businessKey 的新 submissionKey 应允许重新执行，已有效果则复用原结论。无法终止的任务如实报告能力／失败，不伪造 canceled。

## 终态回调与审批对象变化

可以只实现 status，让引擎轮询；实现回调时用**执行方自己绑定的服务票**，`workflow.trigger`，且 `azp` 必须等于该动作 target。借用绑定 workflow 的租户自助凭据不能代替执行方来源。向 `<PORTAL_BASE_URL>/svc/workflow/v1/open/instances/<callback.instanceId>/events` 投递：

```json
{
  "eventName": "action.terminal",
  "messageId": "<submissionKey>:terminal",
  "payload": {
    "businessKey": "<原 businessKey>",
    "submissionKey": "<原 submissionKey>",
    "state": "completed",
    "result": {}
  }
}
```

只回 `completed|failed|suppressed`；允许早于 submit 响应到达。`duplicated:true` 或 `consumed:false` 本身不要求重投，后者可能是旧 attempt／已落定。HTTP／信封失败与上述数据分开处理，保留可恢复记录和有限重试。业务 `WORKFLOW_DISABLED` 不持续重投。

approvals 每项含 `nodeId,taskId,cycle,assigneeId,actedAt,subject,subjectHash`。subject 是审批时绑定的对象快照；subjectHash 为键排序、无空白 JSON 的 SHA-256。声明 `subjectCheck=true` 时，对照权威对象逐字段复核；变化则**不执行**，回 `state:failed,code:APPROVAL_STALE,retryable:false,staleApprovals:[{nodeId,taskId}],current:<同形权威快照>`，让引擎重新请求人工审批。仅有 hash 并不证明当前对象未变。

产物用执行方服务票写入 Files 应用空间，回 `artifacts:[{fileId,name,size,contentType}]` 引用；不要写入会让成员绕过实例权限的公共／团队空间。用户下载走 workflow 实例产物门，工作流不复制大结果与日志。

## v1.1 可选租约

submit 可带 `placement:{leaseKey,affinityKey?,mode?,leaseTtlS?}`。引擎 leaseKey 为实例 ID；优先沿用健康 lease，再按 affinity 选择，最后共享派发。`mode` 为 prefer／require，TTL 范围 300–86400 秒。租约复用回 `{placement:{nodeId,reused}}`，只说明路由沿用，不证明磁盘现场仍在。非终态可给 `retryAfterS`，引擎钳制到 2–600 秒。

支持租约时实现 release：重复释放幂等，未知 key 回 `{released:false}`，不回 404。实例终态后引擎释放，执行方 TTL 兜底；不支持租约不必为普通动作新增现场系统。

验收应覆盖：同提交重传、同业务跨 attempt 防重复、执行后可回查、旧 attempt 回调、预演零副作用、能力版本拒绝、审批失效、真实撤回与新 attempt、跨租户隔离。不要用仅检查 HTTP 200 的测试证明动作契约已实现。
