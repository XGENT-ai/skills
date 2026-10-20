# 用户调用与流程编排

使用面向 workflow 的用户 TDT + `userinfo.read`；跨 App 换票见 [鉴权](authentication.md)。当前角色、PID、实例所有权和审批候选人仍由 workflow 判定，不由 scope 替代。

| 需求 | REST 路由（相对服务根地址） | 额外权限／说明 |
| --- | --- | --- |
| 发现流程、版本、表单 | `GET /v1/workflows`、`GET /v1/workflows/{id}`、`GET /v1/workflows/{id}/versions`、`GET /v1/workflows/{id}/versions/{no}` | 成员不见草稿；版本详情含 `formSchema,definition` |
| 创建草稿 | `POST /v1/workflows`，`{name,description?,templateKey?}` | `workflow:page:designer` |
| 保存 YAML／JSON | `PUT /v1/workflows/{id}/draft/yaml`，`{yaml}`；或 `PUT .../draft`，`{definition}` | `workflow:page:designer` |
| 校验、预演 | `POST /v1/workflows/{id}/validate`、`/dry-runs` | `workflow:page:designer`；预演不执行真实动作 |
| 发布 | `POST /v1/workflows/{id}/publish` | `workflow:action:publish` |
| 发起、查实例 | `POST /v1/instances`，`{defId,input,idempotencyKey?}`；`GET /v1/instances`、`/{id}` | 成员按本人／参与关系可见，列表与详情范围以实际接口为准 |
| 取消实例 | `POST /v1/instances/{id}/cancel` | 发起人本人或 `workflow:action:cancel-any` |
| 动作目录 | `GET /v1/capabilities`；`GET /v1/actions/catalog?target=<key>` | 目录需 `workflow:page:designer`；先发现当前租户可用的 target／operation |
| 审批 | `/v1/todos` 及待办处理入口 | 在 workflow 中由真实候选人完成，不提供 Agent 自动审批捷径 |

## 从草稿到运行

先查动作目录／模板与目标版本表单，再创建或更新同一草稿，校验、预演、按用户意图发布，最后发起并保存 instanceId。发布是不可变新版本，存量实例继续使用原版本。保存草稿不改变正在运行的流程。

REST 外层 `ok:true` 只是接口成功，还需检查：

- YAML 保存 `data.saved` 和 `data.errors`：语法错误可能不保存；语义错误可能 `saved:true` 但仍不可发布。
- 校验 `data.ok` 和 `errors`：与外层同名，不要只读第一层。
- 发布 `data.published` 和 `errors`：`published:false` 时没有产生新版本。
- 预演 `data.ok`、报告中的假设／未验证部分：预演通过不证明业务已执行。

能力响应的 `data.actions[]` 带 `target,available,reason`；目录响应还包一层 `{available,catalog?,reason?}`。未登记／未安装可能以 `available:false` 返回，不是空目录成功；读取动作项用 `data.catalog.actions`。

画布保存 JSON 会清除草稿 YAML；两种入口维护同一草稿，不是各有一个独立版本。JSON 图的 schema 为 `xgent.flow/1`；不要上传 BPMN XML 或任意 JSON 当流程定义。复杂循环、经营事件 trigger 等用画布／JSON 定义，不能假设 Pipeline YAML 支持所有图节点。

## Pipeline YAML

YAML 使用 `name,inputs,steps`；每步唯一 `id`，且恰好有 `uses` 或 `approval` 之一。`needs` 为已有步骤 ID 数组，无依赖步骤可并行；引擎只接受可拆成合法串并联块的结构，不支持任意 DAG。动作键与输入必须来自真实目录。

以下为待替换目录键与业务字段的结构示例，不是已安装的动作：

```yaml
name: 对象审批与处理
inputs:
  - {key: objectRef, label: 对象引用, type: text, required: true}
steps:
  - id: approve
    approval:
      candidateSource: role
      candidateRole: admin
      mustHuman: true
      rejectAction: terminate
      bindsTo: [objectRef]
  - id: execute
    needs: [approve]
    uses: <target>/<operation>
    timeout: PT15M
    with:
      objectRef: "{{input.objectRef}}"
```

`with` 中 `{{input.<key>}}` 为发起变量，`{{steps.<stepId>.<output>}}` 为目录声明的动作输出；JSON 图中对应 `result.<nodeId>.<output>`。纯文本不是自动求值表达式。`timeout` 用 ISO 8601 duration；按业务设置且大于执行方允许的时限。Task Gateway 未认领任务可长期 pending，不能省略流程 timeout 并期待网关替你结束。

审批绑定实际需要保护的金额、版本、对象等字段；执行方的权威对象变化检查见 [动作执行面](action-provider.md)。`mustHuman` 用于真实审批节点，不能把动作目录的同名标签当作自动添加审批的保证。

后台服务只发起预先发布的流程，不借服务票做设计与发布。Agent 编排使用 [MCP](mcp.md) 的工具能力与当前用户权限，发布仍须在用户任务授权范围内。
