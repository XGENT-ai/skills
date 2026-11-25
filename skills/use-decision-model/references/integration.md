# 选择接口与供应商、校准与换模

先确认项目在用的**接口协议**，再确认谁提供端点和模型。System One 以 TypeSafe 为源头，原厂和兼容实现共用一套基础文本契约；OpenAI Decisions 和 Gemini Decisions 的原生接口另列。不要只看供应商或模型名来选 SDK。

| 接口契约 | 同一道是非题如何请求、如何取值 | 容易混淆的地方 |
| --- | --- | --- |
| [System One：TypeSafe 与兼容实现](system-one.md) | `state` + 按 ID 索引的问题对象；从 `answers[id].noul` 读 P(是)。 | [TypeSafe / Jev](providers/typesafe.md) 是参考实现；[Ollama](providers/ollama.md)、独立托管服务和网关也能提供兼容端点。不要默认支持 Jev 的 Box、批量上限或模型列表。 |
| [OpenAI · Decisions](providers/openai.md) | `input` + 带 `name` 的问题数组；在 `answers[]` 按 `name` 找到 `predicate`，读 `probability`。 | 原生结构化证据可以用 `input_json`；不要把它当 Gemini 的文本或 Jev 的 `state`。 |
| [Google · Gemini Decisions](providers/gemini.md) | `input` + 按键索引的问题对象；从 `answers[id].noul` 读 P(是)。 | 另有 `confidence = max(p, 1-p)`。例如 `p=0.02` 时，`confidence=0.98` 是很确定的**否**，不是“是”。 |

**有独立路径或回包的端点**：Workers AI 见 [Cloudflare Clef](providers/cloudflare.md)；数仓行内判断见 [Databricks `ai_decide`](providers/databricks.md)；直接调 `pplx-decider` 见 [Perplexity](providers/perplexity.md)。部分题目形状可复用，不等于这些原生端点兼容 TypeSafe SDK。

尚未选型时，先问哪个条件不能妥协：数据能否离开本机、要判断的媒体类型、是否真需要定位而不仅是看懂图、候选规模、成本与尾延迟。先用只读样例将请求与回包配对：题名、是非方向、候选赢家及分布、评分档位顺序、错误分支。真实请求须在已有授权和预算内；离线走查无需密钥。

## 在同一错误预算下比较

1. 从获准使用的数据中准备标注样本，覆盖没命中、候选缺失、阈值两侧及少数但代价高的错例。保留现有规则或检索作为基线，把调阈值与最终评估的数据分开。
2. 固定证据、题意、候选和各自的模型版本；按每家协议编码同一任务，不要求请求体字节相同。逐题看正确性和校准：例如对二值标签 `y`，用 P(是) 与标签之间的均方误差（Brier 分）衡量概率预测；误差越小越好。服务错误另外计数，不能从总请求分母里删除。
3. 每个模型各自调阈值，再在保留样本上比较**误动作率、自动处理比例、复核量、延迟和成本**。即使两个模型都返回 `confidence`，其定义和数值也可能不同。没有满足错误预算的阈值时，保留人工路径。
4. 获准灰度时让新策略先只记录「本来会怎么做」，不重复执行动作。将问题版本、模型版本、策略阈值一同切换，能一起回滚。只改权重或阈值可重放缓存；若证据、选项、问题含义或模型改变，旧分布不能当作新模型结果。
