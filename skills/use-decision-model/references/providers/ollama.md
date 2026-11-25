# Ollama：本机 System One

本机示例端点是 `POST http://localhost:11434/v1/systemone`，不需要 API 密钥。先确认安装的 Ollama 和已下载的权重都支持 System One；普通聊天模型不能用这个请求体打分。

```json
{
  "model": "nimble",
  "state": "请退还重复扣的那笔钱。",
  "questions": {"refund": {"type": "noul", "instructions": "客户是否明确要求退款？"}}
}
```

按题目 ID 读 `answers.refund.noul`，即 P(是)。返回的 `model` 可能只是请求中的名字，做回归时另记本机权重。Choice、Score 的 `confidence` 按概率分布的熵计算，别沿用 Jev 的阈值。

上线前分别检查所选权重的候选上限、长输入是否截断，以及首次加载与热请求的延迟。示例不是长期兼容承诺；以 [Ollama System One 契约](https://docs.ollama.com/api/systemone)和[所用权重的模型页（以 Nimble 为例）](https://ollama.com/library/nimble:9b)为准，不套用 Jev、OpenAI 或 Gemini 的配置。
