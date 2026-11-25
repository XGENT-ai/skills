# TypeSafe Cookbooks：按任务查本地配方

从「代码最后要做什么」找一篇读，无需顺序通读或下载原文。以下 27 篇来自 [TypeSafe Cookbook 目录](https://docs.typesafe.ai/cookbooks.md)（2026-11-25 核对）。要选流程而不是找完整案例，请看[六个 TypeSafe Patterns](patterns.md)。案例里的题型、字段和测量来自 Jev；用其他供应商时只迁移做法，按[对应契约](integration.md)取值，重新校准阈值。

## LLM 与 Agent

| 要解决的问题 | 读这篇 |
| --- | --- |
| 将一句话转成已知函数和合法参数，保留未声明的默认值 | [函数调用](recipes/function_calling.md) |
| 在很长的技能目录里为当前轮次选至多一个，而不是硬选 | [两阶段技能推荐](recipes/skill_suggestion.md) |
| 独立筛查输入和输出；更换处置政策不重跑模型 | [输入输出护栏](recipes/llm_guardrails.md) |
| 验证一条引文的上下文是否支持主张 | [引文核验](recipes/citation_check.md) |
| 没有标准答案时，筛出不受给定段落支持的回答 | [根据来源核查回答](recipes/verify_llm_outputs.md) |
| 回答之前，剔除无关检索片段，把冲突证据另列 | [RAG 段落分流](recipes/classifying_rag_passages.md) |
| 判断一次交互是否值得记住，并在后续请求中取用 | [简单记忆系统](recipes/basic_memory.md) |

## 分类

| 要解决的问题 | 读这篇 |
| --- | --- |
| 在深层分类树上保留多个候选路径 | [层级分类与束搜索](recipes/hierarchical_classification.md) |
| 细分类把握不足时，仍返回有用的父类 | [按置信度回退到粗分类](recipes/classification_using_confidence.md) |
| 对消息流区分隐藏、模糊和高亮，同一消息可能有多个特征 | [Twitch 聊天过滤](recipes/twitch_chat_filter.md) |

## 值与结构提取

| 要解决的问题 | 读这篇 |
| --- | --- |
| 用便宜模型提取记录；只升级字段检查不通过的样本 | [结构化提取级联](recipes/sde_cascade.md) |
| 从文字里找绝对或相对日期；由代码做日历运算 | [日期提取](recipes/date_extraction_cookbook.md) |
| 在原文的若干金额、电话或邮箱中选出所问的那一个 | [预解析候选值提取](recipes/pre_parsed_value_extraction_cookbook.md) |
| 把丢失排版的文字恢复成 Markdown，同时保留原文 | [段落拼接与结构恢复](recipes/autoformat.md) |
| 一句话点多件商品，逐件识别，再填写各自的菜单选项 | [从自然语言构建购物车](recipes/cart_from_primitives.md) |

## 检索与匹配

| 要解决的问题 | 读这篇 |
| --- | --- |
| 已有检索器，想改善短名单的排序 | [检索结果重排](recipes/rerank_typesafe.md) |
| 候选太多，一轮 Choice 放不下 | [从大候选集逐轮缩小范围](recipes/infinite_cardinality_choice.md) |
| 在一份文档中找到回答问题的原文行，允许无答案 | [按行语义查找](recipes/semantic_find.md) |
| 判断来自两个来源的记录是否指同一实体 | [实体对齐与复核](recipes/entity_alignment.md) |
| 根据实时转录判断哪些提纲要点已讲过 | [实时提词清单](recipes/intelliprompter.md) |

## 校准、批处理与特征

| 要解决的问题 | 读这篇 |
| --- | --- |
| 没有答案标签时，比较重复分类是否改变了处置 | [Choice 自一致性](recipes/consistency_choice_cookbook.md) |
| 检查重复的是非判断在阈值附近会不会翻转 | [Noul 自一致性](recipes/consistency_noul_cookbook.md) |
| 同一份材料要回答多道独立问题，衡量合批的代价 | [并行提问](recipes/parallel_questions.md) |
| 把模型判断变成有标签预测任务可复用的特征列 | [自动发现语义特征](recipes/autoresearch_feature_discovery.md) |

## 图像与扫描件

| 要解决的问题 | 读这篇 |
| --- | --- |
| 从扫描表单定位记录、读日期、明确标出看不清的项 | [读取扫描接种记录](recipes/reading_scanned_documents.md) |
| 验证 OCR 给出的值，定位原图并裁出局部核对 | [OCR 置信度与边界框](recipes/ocr_confidence_bounding_boxes.md) |
| 从多帧观测整理每次到访，而不是逐帧重复计数 | [喂鸟器到访日志](recipes/feeder_watch.md) |
