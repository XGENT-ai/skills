# TypeSafe / Jev：System One 原厂接口

本页的地址、鉴权与 Box 用法针对 TypeSafe 托管服务。若使用第三方端点，以 [System One 兼容实现对照](../system-one.md)核对差异，无需把推理改回 Jev。

文本示例使用 `POST https://api.typesafe.ai/v1/systemone`，服务端发送 `Authorization: Bearer $TYPESAFE_API_KEY` 与 `Content-Type: application/json`。`jev-latest` 是示例别名；校准时记录实际模型版本。下面是一道只读的是非题：

```json
{
  "model": "jev-latest",
  "state": {"message": "账单里有两笔相同的扣款，请问退款政策是什么？"},
  "questions": {
    "wants_refund": {
      "type": "noul",
      "instructions": "客户在 `message` 里明确要求退回款项吗？",
      "criteria": {"true": "要求实际退还款项", "false": "只问政策或只描述扣款"}
    }
  }
}
```

`state` 是共享证据；`questions` 和返回的 `answers` 都是按自定题目 ID 索引的对象。响应还包含 `model`、`usage`。Jev 的原语和字段如下：

| 题型 | 如何定义与读取 |
| --- | --- |
| `noul` | 可选 `criteria.true/false` 描述正反情形；取 `answers.wants_refund.noul`，是 P(是)，没有单独的 `confidence`。例中若 P(是) 很低，提供政策说明；落在灰区就澄清，不把请求失败当成低概率。 |
| `choice` | `criteria` 是候选键到说明的对象。读 `choice` 和 `probabilities[choice]`；`confidence` 是另一个量。相对最像不等于确有匹配。 |
| `score` | `criteria` 是从低到高的有序档位；读 `score`、`legend`、`probabilities` 和 `confidence`。均值可为小数；若关心是否越过某业务边界，可加总边界另一侧的档位概率。 |
| `box` | 图像或 Box 请求需 `TypeSafe-Danger-Zone-Images: true`；让 `state` 引用 `assets` 里的图像，不能直接发本地路径。`box.left/right/top/bottom` 相对宽高归一化，另有定位置信度。先判目标存在，再裁图。 |

阅读本地 [Patterns](../patterns.md) 和 [Cookbooks](../cookbooks.md) 不用密钥。遇到模型支持范围或 SDK 类型变化时，核对 [TypeSafe HTTP 契约](https://docs.typesafe.ai/api/system-one.md)及 [Box 文档](https://docs.typesafe.ai/primitives/box.md)，不要把 Gemini 同名 `noul` 的响应结构移植过来。
