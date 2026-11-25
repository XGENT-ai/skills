# Gemini Decisions API

[Google 官方契约](https://aistudio.google.com/docs/decisions-api)：仅用于 Gemini Developer API `v1beta` 和模型 `gemini-decision-1`。这是封闭式判断接口，不生成解释或任意 JSON，也没有 Box 原语。

## 最小请求：区分政策咨询与退款要求

REST：`POST https://generativelanguage.googleapis.com/v1beta/decisions`，请求头为 `Content-Type: application/json` 和 `x-goog-api-key: $GEMINI_API_KEY`。也可用 Google Gen AI SDK 3.2 或更高版本：Python `google-genai` 的 `client.decisions.create`，JavaScript `@google/genai` 的 `ai.decisions.create`；客户端选择 `v1beta`。

```json
{
  "model": "gemini-decision-1",
  "input": "账单里有两笔相同的扣款，请问退款政策是什么？",
  "questions": {
    "refund_requested": {
      "type": "noul",
      "instructions": "客户是否明确要求退还款项，而不只是在咨询政策？"
    }
  }
}
```

回包用同一题目 ID 取 `answers.refund_requested`。例如，若得到 `{"type":"noul","noul":0.03,"confidence":0.97}`（示意，非实测），表示模型很有把握地判为**未明确要求退款**。此时显示政策说明；不要因 `confidence` 很高而创建退款申请。顶层另有 `usage`：按模态计输入 token，`total_output_tokens` 始终为 0，仅计费输入。

## 题型与读数

| `type` | 请求的 `criteria` | 同 ID、同 `type` 的答案 |
| --- | --- | --- |
| `noul` | 可省略；需界定正反例时用 `true_description`、`false_description`。 | `noul` 是 P(是)，范围 0.01–0.99；`confidence = max(noul, 1 − noul)`，不是另一个 P(是)。 |
| `choice` | 必填，对象将 2–255 个选项 ID 映射到说明；ID 自明可用空字符串。 | `choice` 是赢家，`probabilities` 按选项 ID 给出分布，`confidence` 是赢家概率。精确并列按 ID 的 UTF-8 字节序取首，不按提交顺序。即使全不适合也有赢家，可加 `other` 或另问是否适用。 |
| `score` | 必填，从低到高的 2–255 个非空说明字符串，通常建议 2–10 档。 | `score` 是从 0 起算的档位下标期望值。`probabilities` 和 `legend` 以字符串下标为键；要取离散档就取概率最高档，再查 `legend`。`confidence` 是各档概率中的最大值，不是对平均分的置信度。 |

## 输入、上限与失败

- `input` 是非空字符串、单个内容块或块数组；支持文本、图片、音频、视频、PDF。业务 JSON 对象要先序列化成文本，不要直接当内容块；相邻文本块之间不会自动加分隔符。媒体块用 `mime_type` 加 `data`（base64）或 `uri`（Files API URI、公开或签名 HTTPS URL）；不支持 YouTube URL 或直接传 `gs://`。
- `questions` 是映射，单次 1–1,000 题；键匹配 `[A-Za-z0-9_-]+`，最多 64 字符。各题独立，模型看不到题目 ID；完整题意写进必填的 `instructions`（仅文本）。共享输入与所有题目合计不超过 262,144 tokens。共享输入在一次请求中只计一次，跨请求重复则重新计入。
- 不支持对话历史块、工具调用或工具结果；多轮记录可序列化成文本。不支持上下文缓存、Batch API、Flex 或 Priority 推理。
- 超限或键不合法会返回 HTTP 400；输入被安全过滤时也是 400，返回 `error.code`、`error.message` **而非** `answers`。进入错误分支，不要把无答案当成 P(是) 为零。
