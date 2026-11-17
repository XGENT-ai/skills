# use-prompt-dsl 文白方言实测对比（未采用）

日期：2026-11-17。结论先行：**文白方言未达到「token 更少」的目标，不采用**；v1（[language.md](../skills/use-prompt-dsl/references/language.md)）保持为当前规范。实验稿 [ODC_hybrid_wenyan.md](../skills/use-prompt-dsl/references/ODC_hybrid_wenyan.md)（v2.0-hybrid-experimental）保留为未跟踪文件，不随 skill 分发。完整协议与证据在 `local/use-prompt-dsl-validation/2026-11-17-wenyan/`（gitignored），protocol.md 冻结于运行前。

本次曾按「采用」完成的改动（SKILL.md 入口、examples.md 13 例、scenarios.md 接口重述、optimize-agents-md 方言固定、README 行）已全部回滚到 v1 状态；本记录仅保留实验证据，防止将来重复同一实验。

## ① Token 成本（决定性证据）

4 组等义 prompt 对；wenyan 臂由 v1 逐字段翻译保证语义等价；gpt-tokenizer 双口径（`tokens/counts.json`）：

| 对 | o200k_base | cl100k_base | 字符数 |
| --- | --- | --- | --- |
| P1 规则片段 | 106 → 134（+26.4%） | 107 → 178（+66.4%） | 453 → 179 |
| P2 需求验收 | 284 → 308（+8.5%） | 280 → 366（+30.7%） | 1125 → 561 |
| P3 状态迁移 | 276 → 299（+8.3%） | 274 → 331（+20.8%） | 1051 → 697 |
| P4 多 Agent 工作流 | 493 → 519（+5.3%） | 490 → 636（+29.8%） | 2002 → 975 |

**文白版字符数降 40–60%，token 在两种 BPE 口径下均上升。** 机制：中文语义 Literal 约 1 token/字，英文 snake_case 复合词共享子词片、更省；结构关键字（`须` vs `MUST`、`律` vs `RULE`）节省有限，语义 Literal 才是 token 大头。在不牺牲自描述性（规范自身「戒省义」约束）的前提下，文白无法在主流 tokenizer 上反超 v1。

## ② 行为对比（次要证据：理解力等价）

3 任务 × 两方言臂，独立执行者 + 匿名评委（模型 zai-coding-cn/glm-5.3-flash，每臂 n=1，方向性）：

| 任务 | v1 | 文白 |
| --- | --- | --- |
| T1 认识状态纪律（provided/checked/unknown） | 6/6 | 6/6 |
| T2 规则冲突处置（UNLESS 例外边界） | 6/6 | 5/6（未显式并置规则张力 −0.5；docstring 超最小边界 −0.5） |
| T3 工作流分支汇合（unknown ≠ false、any_successful） | 6/6 | 6/6 |

行为面文白无理解缺陷；失败点在 token 成本，不在语义。

## ③ 翻译保真（对后续的可用发现）

本仓 AGENTS.md `@failure_handling` 片段 v1→wenyan 判定基本等价。两条可复用建议：括注宜加「含…等」避免穷尽式误读；「actionable」宜译「可采取行动的」。一条体系层发现：改写时**保留既有 `@对象名` 标识符**，否则交叉引用断链——此结论对任何方言/改写流程都适用。

## 局限

单模型、每臂单次、OpenAI 双 BPE 口径（Claude 官方分词器不公开）；中文原生模型的 tokenizer（GLM/Qwen 等）未测，若将来重开实验，应先在该口径上验证 token 方向再谈行为。评委与执行者同模型、部分盲化，均已在协议中披露。

[2026-10-29 Odyssey 记录](use-prompt-dsl.odyssey.validation.md) 与 [2026-10-24 记录](use-prompt-dsl.validation.md) 仍为现行 v1 规范的有效验证。
