# Databricks：在 SQL 中用 ai_decide

要逐行判断表中的文本，把文本列作为第一项 `state`；第二项 `questions` 是常量字符串，内容为按题目 ID 索引的非空 JSON 对象。可先用一条文本理解返回值：

```sql
SELECT ai_decide(
  '请退还重复扣的那笔钱。',
  '{"refund":{"type":"noul","instructions":"客户是否明确要求退款？"}}'
) AS decision;
```

`decision` 是 `VARIANT`。先看 `error_message`：非空时 `response` 为 null，该行应计为调用失败；不要用零填补。成功时从 `response.answers.refund.probability` 取 P(是)，不是从 Jev 的 `answers.refund.noul` 取。

需要在 SQL 外调用时，工作区 REST 路径为 `POST /api/2.0/ai-functions/ai-decide`，不是 OpenAI 或 Gemini 的 Decisions 路径。示例不是长期承诺；按工作区核对 [SQL 函数契约](https://docs.databricks.com/aws/en/sql/language-manual/functions/ai_decide)或 [REST 契约](https://docs.databricks.com/api/ai-functions/v1/ai-decide)。
