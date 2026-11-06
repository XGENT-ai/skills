# 确认表单数据与结果格式

`scripts/decision-form.mjs` 读取表单数据（下称 form.json），校验后在 `127.0.0.1` 提供确认页。用户提交后，脚本把结果写入 result.json 并退出。字段值是给用户看的原文摘要，语言跟随文档；模板不解析 Markdown，换行按原样显示。

## form.json

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `schemaVersion` | 是 | 固定为 `1` |
| `template` | 是 | `requirements`（PRD / CRD）或 `dev-plan`（开发计划） |
| `locale` | 否 | `zh-CN`（默认）或 `en`，决定界面文字 |
| `round` | 是 | 从 `1` 开始；修订稿复核与受影响条目用 `2`、`3`… |
| `title` | 是 | 文档或本次确认的名称 |
| `documents` | 是 | `[{ type, path, version }]`；`type` 写 `PRD`、`CRD` 或 `Dev-plan`，`version` 写版本号、提交或读取时间 |
| `scope` | 否 | 用户限定的范围，如「P1 全部功能」 |
| `items` | 是 | 条目列表，按文档中的阅读顺序排列，`id` 不可重复 |

所有条目都有 `id`、`title`、`source`（原文位置，如 `PRD §3.2`），可选 `impact`（会改变选择的关键限制）和 `revision`（见下）。`dependsOn`、`affects` 只能引用本表单里的 `id`；引用不在本轮的条目时去掉该引用，并把需要用户知道的关系写进 `impact`。

### requirements 条目（PRD / CRD 功能点）

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `what` | 是 | 做什么，用用户能看到的行为写 |
| `why` | 是 | 价值或动机；文档没写就填「文档未说明」 |
| `acceptance` | 否 | 可观察的验收结果，文字列表 |
| `phase` / `priority` | 否 | 原文的分期与优先级 |
| `dependsOn` | 否 | 前置功能的 `id` 列表 |
| `details` | 否 | `[{ label, text }]`，放文档特有字段，如 CRD 的「请求接收方」「提出方立场」「协作契约」 |

### dev-plan 条目

每项用 `kind` 区分。`decision` 是技术架构决策，`feature` 是功能点。

| `kind` | 字段 | 说明 |
| --- | --- | --- |
| `decision` | `choice`、`why`（必填） | 计划选定的方案与理由 |
| | `alternatives` | `[{ name, tradeoff }]`，计划里写过的备选及弃选原因；名称不可重复 |
| | `cost` | 选定方案的代价与风险 |
| | `affects` | 受这项决策影响的功能 `id` |
| `feature` | `what`、`how`（必填） | 做什么；计划中的主要落点与路径，计划没写就填「计划未说明」 |
| | `why`、`milestone`、`verification` | 价值、所属里程碑、验证方式 |
| | `dependsOn` | 前置功能的 `id` 列表 |

### revision（第二轮及以后）

只用于 `round ≥ 2`，展示这一项为什么回来以及修订前后的内容。条目正文字段写修订后的完整版本。

| 字段 | 必填 | 说明 |
| --- | --- | --- |
| `reason` | 是 | `modify`：用户上一轮要求修改；`dependency`：受其他决定影响 |
| `before` / `after` | 是 | 修订前后变化的那段内容 |
| `note` | 否 | 用户上一轮的修改说明，原样引用 |
| `cause` | 否 | `dependency` 时写明由哪个决定引起，如「AD-3 已驳回：文件不再保留 30 天」 |

## result.json

```json
{
  "schemaVersion": 1,
  "form": { "template": "dev-plan", "round": 1, "title": "…", "documents": [] },
  "submittedAt": "2026-11-06T08:30:00.000Z",
  "items": [
    { "id": "AD-1", "decision": "modify", "alternative": "请求内同步流式下载" },
    { "id": "F-2", "decision": "confirm" },
    { "id": "F-4", "decision": "reject", "note": "本期不做差异比对" },
    { "id": "F-3", "decision": null }
  ]
}
```

- 每个条目恰好一条答复。`decision` 为 `confirm`、`reject`、`modify` 或 `null`（未决）。
- `note` 是用户的原话：`modify` 时必填，除非选了 `alternative`；`confirm` / `reject` 可附。
- `alternative` 只出现在选了修改的 `decision` 条目上，值是该条目 `alternatives` 中的一个 `name`。
- 服务端拒绝不合格的提交，页面原地显示原因，结果文件只在提交通过时写入。
