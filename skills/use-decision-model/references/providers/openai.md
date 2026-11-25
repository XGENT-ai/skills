# OpenAI Decisions：从证据得到可供代码判断的答案

[官方中文指南](https://developers.openai.com/zh-Hans/api/docs/guides/decisions)列出的接口已正式可用（GA），目前唯一支持的模型是 `gpt-6-luna`。使用 `POST https://api.openai.com/v1/decisions`，请求头为 `Content-Type: application/json` 和 `Authorization: Bearer $OPENAI_API_KEY`（密钥由服务端注入）。它返回判断，不生成报告，也不执行工具。需要自定义 JSON 输出或让模型提出工具调用时，改用 Responses API 的相应能力。

## 请求和答案怎样对应

`model` 指定模型；`input` 放共享证据，可以是文本字符串，也可以是有序的消息、工具调用及其输出数组；`questions` 是 **1–400** 道题的数组。为每题设置唯一 `name` 和清楚的 `instructions`。独立问题可同批；若下一题取决于上一题的答案，则分两次请求。成功回包的 `answers` 也是数组：按 `name` 找到答案，再判 `type`，因为单题可能返回 `refusal`。

| 问题 | 如何设题 | 如何读答案 |
| --- | --- | --- |
| `predicate`：条件成立吗？ | 把条件写成能由证据判断的命题。 | `probability` 是条件为真的估计概率（0–1），没有单独的 `confidence`。接近 0 倾向否，接近 0.5 是难以区分。 |
| `choice`：选哪一类？ | `choices` 是含 `value`、可选 `description` 的数组。两者模型都能看到；优先用有意义且不同的值，可能都不合适时加 `other`。 | `choice` 是给定值之一；`probabilities` 按项返回 `{value, probability}`，另有 `confidence`。被选中不等于证据充分。 |
| `score`：在有序尺度上处于哪里？ | `levels` 由低到高列出 2–10 档，用 `label` 标识并写明各档的 `description`。 | `score` 是从 0 开始的档位下标按概率加权的均值，可落在两档之间；`probabilities` 的项含 `value`（下标）、`label`、`probability`，另有 `confidence`。中间分可能来自两端分歧。 |

不要把 `confidence` 当成最大选项概率：官方 Choice 示例中，所选项概率为 0.95，`confidence` 为 0.93。指南没有给出通用换算公式。用自己的标注样本，按误报和漏报的代价校准阈值。

## 只读示例：是否展示取消续费的帮助入口？

以下是请求体。读取用户原话判断其意图，不查询账户，也不取消订阅。

```json
{
  "model": "gpt-6-luna",
  "input": [{
    "role": "user",
    "content": [{"type": "input_json", "data": {"message": "想换套餐，是不是要先停掉现在的？"}}]
  }],
  "questions": [{
    "type": "predicate",
    "name": "wants_to_stop_renewal",
    "instructions": "message 中的客户是否明确表示想停止今后的自动续费？仅咨询换套餐不算。"
  }]
}
```

示意回包片段（非实测）：

```json
{
  "answers": [{"name": "wants_to_stop_renewal", "type": "predicate", "probability": 0.53}]
}
```

代码只选择要显示的内容，不改变订阅状态；此回包会显示澄清问题。高、低阈值仅为演示。

```javascript
function helpText(decision) {
  const a = decision.answers.find(x => x.name === "wants_to_stop_renewal");
  if (!a || a.type !== "predicate") return "暂时无法判断，请手动选择帮助主题。";
  if (a.probability >= 0.90) return "查看如何关闭自动续费";
  if (a.probability <= 0.10) return "浏览套餐帮助";
  return "你是想关闭续费，还是只想更换套餐？";
}
```

若请求超时或 HTTP 失败，调用方走服务不可用分支，不构造概率 0 传给这段代码。

## 输入、缓存与运行边界

- **结构化证据**：用户消息允许 `input_text`、`input_image` 和 `input_json`。对象或数组放在 `input_json.data`，其中名为 `role` 或 `image_url` 的键只是数据。开发者／系统消息只传输入文本（系统消息按开发者处理）；助手历史可含输出文本或拒答。函数或自定义工具的调用与输出按顺序给出，以 `call_id` 配对，作为证据而非执行指令。
- **图像**：`input_image.image_url` 接受公开 HTTP(S) URL 或内联 base64 data URL，不接受 `file_id`。共享图放在 `input`，候选图可放在各题 `instructions` 的非空文本／图像内容项数组中，不加 `role` 包装；两处共用请求的图像限额。不支持文件、音频或输入项引用；语音可通过 GPT-Live 集成，不是把音频直接传入此端点。
- **重复输入**：共享 `input` 的相同前缀达到 1,024 Token 才有资格命中提示缓存；`questions` 及其中的图像不计入该前缀。

移植案例时，用本页的 `predicate`、`choices[]` 或 `levels[]` 表达同一题意，不照抄 Jev 的 `criteria` 或 `answers[id]`。能判断图片里有没有目标，不等于提供 Box 定位。换模后按[相同标注样本重新校准](../integration.md)。
