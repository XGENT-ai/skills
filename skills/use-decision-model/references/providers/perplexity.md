# Perplexity Decisions：按题目 ID 读答案

向 `https://api.perplexity.ai/v1/decisions` 发 POST，请求头用 `Authorization: Bearer $PERPLEXITY_API_KEY` 和 `Content-Type: application/json`。请求体示例：

```json
{
  "model": "pplx-decider-v1.1-27b",
  "state": {"review": "耳机用了两周就充不进电了。"},
  "questions": {"defect": {"type": "noul", "instructions": "评论是否报告了产品故障？"}}
}
```

`questions` 和 `answers` 都是映射；成功时按 ID 取 `answers.defect.noul`，即 P(报告了故障)。同叫 `/v1/decisions` 不代表能复用 OpenAI 的题目数组；Jev SDK 默认的 `/v1/systemone` 路径和 Gemini 的 `input` 字段也不适用。

带图时把 `image_url` 内容块放进 `state` 数组，`image_url.url` 用 base64 data URL，不传远程图片地址或顶层 `images`。示例模型名和接口不作长期承诺；核对 [Perplexity 快速入门](https://docs.perplexity.ai/docs/decisions/quickstart.md)和[请求契约](https://docs.perplexity.ai/api-reference/decisions-post.md)。
