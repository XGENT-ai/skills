# Cloudflare Clef：区分 Workers binding 与 REST

在 Worker 中用 AI binding 调模型；模型标识传给 `run`，请求体还要写对应的短模型名。下面从 binding 的直接返回值读 P(是)：

```js
const decision = await env.AI.run("@cf/cloudflare/clef", {
  model: "clef",
  state: "所有用户都无法结账。",
  questions: { outage: { type: "noul", instructions: "是否报告了影响所有用户的故障？" } },
});
const pOutage = decision.answers.outage.noul;
```

改走 REST 时，用 Bearer 令牌向 `https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/run/@cf/cloudflare/clef` 发 POST。先检查 `success`，成功后读 `result.answers.outage.noul`；不要把 `result` 带回 binding 的读取路径。

图像放顶层 `images`，可传 base64 data URL；能看图不等于能返回 Box。示例不承诺长期兼容：以 [Workers AI 模型页](https://developers.cloudflare.com/workers-ai/models/clef/index.md)和[输入契约](https://developers.cloudflare.com/workers-ai/models/clef/schema-input.json)为准，不照搬 Jev、OpenAI 或 Gemini 的端点与图像字段。
