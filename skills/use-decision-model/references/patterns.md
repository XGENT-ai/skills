# TypeSafe Patterns：先选决策流程，再接供应商

以下对应 [TypeSafe 官方 Patterns 目录](https://docs.typesafe.ai/patterns)的六个模式。每篇都是本地文件。它们讲的是**如何组合判断与代码分支**；Jev 的题型和示例阈值属于原例。用 OpenAI 或 Gemini 时，先确认[对应供应商](integration.md)给出的概率、置信度或评分能表达同一判断。

| 你正在解决的问题 | 读哪篇 | 代码要做的选择 |
| --- | --- | --- |
| 几个分支读同一份证据，希望少一次往返 | [推测式并行提问 · Speculative fan-out](recipes/fan-out.md) | 同批问各分支的问题，只用实际分支的答案。 |
| 大多数输入不值得查历史记录、调用慢模型或进入深审 | [先用是非题筛选 · Screen with a Noul first](recipes/noul-screen.md) | 小输入先筛；明确通过才付第二阶段成本，灰区有单独去向。 |
| 有限选项必须选一个，但现实中可能全不对 | [独立判断有无匹配 · Companion Noul](recipes/companion-noul.md) | 同批问「最像哪一个」与「是否有匹配」，后者否决前者。 |
| 严重度均值看起来很高，但原始分布分歧很大 | [按评分置信度分流 · Routing by confidence](recipes/confidence-routing.md) | 先判断评分是否足以自动分流，再按严重程度分派；必要时看业务边界一侧的概率总和。 |
| 查余额与发起确认的误判成本不同 | [按动作概率路由 · Routing by probability](recipes/intent-routing.md) | 查赢家的概率并使用该动作的门槛；高分只能打开获准的流程。 |
| 用户要按不同偏好对同一组候选重排 | [多维组合评分 · Composite scoring](recipes/composite-scoring.md) | 各维独立评分，归一化并保存；只改权重时无需重跑推理。 |

**别混淆两种“先判断”。** Companion Noul 为了**避免选错**：两个问题共享证据，可以同时发，代码先看存在性。Noul screen 为了**省掉后续工作**：第一题真的要先返回，否则客户记录已经查了、慢模型也已经跑了。想看完整的检索、抽取或审核流水线时，再查[本地 Cookbook](cookbooks.md)。
