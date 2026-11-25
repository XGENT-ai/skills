# System One：TypeSafe 原厂与兼容实现

TypeSafe 最先提出 System One 决策模型接口，Jev 是它的参考实现。**System One Compatible 指具体端点实现了这套接口，不表示模型由 TypeSafe 提供，也不表示该厂商的所有端点都兼容。** 既可以通过网关调用 Jev，也可以用同一提问方式调用另一家的模型。

## 先认出共同的接口

以[原厂请求示例](providers/typesafe.md)为基准：在服务的基础地址后 `POST /v1/systemone`，提交 `model`、共享证据 `state` 和按题目 ID 索引的 `questions`。基础文本题型是 `noul`（是非）、`choice`（单选）、`score`（有序评分）；成功回包按同一 ID 从 `answers` 取值。例如题名为 `refund` 的 Noul，从 `answers.refund.noul` 读 P(是)。这部分才是复用对象；凭证、模型 ID、Box、批量上限和置信度定义都要按实现核对。

| 实现 / 服务 | 接入点 | 能复用什么，哪里不同 |
| --- | --- | --- |
| [TypeSafe / Jev](providers/typesafe.md)（原厂） | `https://api.typesafe.ai` | 参考请求与回包；Jev 的图像资产、Box 及启用请求头见原厂页。 |
| [Ollama](providers/ollama.md)（本机运行时） | `http://localhost:11434` | 使用同一文本端点和题目结构，可让支持 System One 的本地权重（如 `nimble`）回答，不必使用 Jev。候选上限和置信度含义看所选模型与运行时。 |
| [OpenRouter](https://openrouter.ai/docs/guides/community/typesafe-sdk.md)（网关） | `https://openrouter.ai/api` | 可复用 TypeSafe SDK 的推理调用，改用 OpenRouter 的地址、密钥和所选模型 ID。SDK 会补 `/v1/systemone`，不要在基础地址重复加 `/v1`。推理兼容不涵盖模型列表：`client.models.list()` 不能按 TypeSafe 的形状解析该网关目录。 |
| [Vercel AI Gateway](https://vercel.com/docs/ai-gateway/sdks-and-apis/typesafe.md)（网关） | `https://ai-gateway.vercel.sh/typesafe` | 在这个兼容入口使用 TypeSafe 客户端和网关凭证，例如模型 `typesafe-ai/jev`；不要换成 Vercel 原生 Decision API 的字段。 |
| [Decision Models](https://decisionmodels.io/jev-compatible-api)（独立托管服务） | `https://api.decisionmodels.io` | 接受 `POST /v1/systemone`，使用该服务自己的 Bearer key、模型 ID 和限额。例如 `s1-fast` 每次只接一道题，不能因为请求体相同就照搬 Jev 的多题合批。 |

## 遇到没列出的厂商

1. **按具体端点识别。** 找其公开文档或已安装 SDK 的请求与响应示例，核对 URL、鉴权、`state`、题目映射和 `answers[id]`。只有题型同名，或模型名里带 *Decider*，不算接口兼容。
2. **说明在哪一层兼容。** 比如 [Perplexity 原生接口](providers/perplexity.md)可以复用文本题目形状，但走 `/v1/decisions`；[Cloudflare Clef REST](providers/cloudflare.md)需要处理 `result` 外层。这两种都不能直接照搬 TypeSafe 客户端的默认调用。如果供应商另有明确的兼容端点，按那个端点而不是按品牌归类。
3. **协议能通，再比较行为。** 先核对一题及其回包，再核对任务实际依赖的多题、图像或模型列表；未提供的能力记为未知。只有得到授权才发送试请求。即使 JSON 完全相同，仍须用标注样本[重新校准阈值](integration.md)。
