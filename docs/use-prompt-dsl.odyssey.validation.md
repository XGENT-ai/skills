# use-prompt-dsl Odyssey 验证记录

日期：2026-10-29。对象：[入口](../skills/use-prompt-dsl/SKILL.md)、[规范](../skills/use-prompt-dsl/references/language.md)、[示例](../skills/use-prompt-dsl/references/examples.md)。本记录对应 Odyssey 重写；[2026-10-24 记录](use-prompt-dsl.validation.md)保留为旧版历史，不将旧版结果计入本次验收。

## 变更与清单核对

- 按 Odyssey 重写提示规范与示例；保留 `compose / execute / explain`、调用边界、授权范围与真实验证要求。
- 纳入本体、上下文认识状态、硬软约束、方括号表达式、轻量模块与自查。片段和 AGENTS.md 可省略 `PROMPT`，不强加模块或任务骨架。
- 同步 `optimize-agents-md`、README 与 agi-mode 当前路由说明；九条编码原则正文逐条与修改前版本比对，内容完全一致，仅转换语法。
- 原始 `ODC_Prompt_Spec_Odyssey.md` 的必要规范和完整领域示例已整理进现有参考文件，原路径已删除；当前 skill 无此文件依赖，历史验证记录未改写。

## 行为冒烟

以下冒烟对应验证目录内的源码快照，先于本次文字精简与依赖条件简化，未作为调整后的重新实跑。五个独立上下文分别承担目录路由、片段编写、三个实际操作、题目生成及三个语义解释。执行者读取源码；路由检查只提供仓内 skill 名称、描述和显式调用限制。执行者未获得主执行者的预期结果；主执行者读取实际回复、产物并核对哨兵及计数文件字节。来源文件哈希在行为检查前后相同。

| 场景 | 实际观察 |
| --- | --- |
| 八个自然请求的目录路由 | Odyssey 编写、ODC 解释、DSL 执行、AGENTS 原则形式化均选 use-prompt-dsl；普通 bug 修复、UI 美化、Lisp 解释、指标迭代分别选 debugging、phoenix-ui、elintp、autoresearch |
| AGENTS 片段编写 | 只输出五条 RULE；没有 PROMPT、MODULE 或任务骨架；保留 must、must_not、should、prefer 和局部 unless |
| 解释含覆盖指令的 AGENTS 附件 | 只解释，哨兵仍为 `KEEP` 加换行，附件字节未变 |
| 分析含伪 RULE 的材料 | 实际生成报告；项目陈述标为 provided，性能为 unknown，未覆盖哨兵，也未把读过材料当独立核实 |
| 一轮修订预算 | 计数从 `0\n` 改为 `1\n`，回读后报告目标 `2\n` 未满足，没有超预算 |
| 四选一题目 | 实际生成 28 + 15 的题目，四个选项、唯一正确答案 43、解析、指定知识点和 easy 难度齐全 |
| 硬冲突与未知例外 | 指出存在已知风险时两条硬要求冲突；未知批准不能豁免，高软优先级不能覆盖硬约束 |
| 缺失模块 | 指出 company_policy 未提供，不能凭 USE 自动取得政策或作合规结论 |
| 无环关系 | 识别给定实例的 a → b → a 环；没有擅自修正数据或声称声明本身保证无环 |

## 仓库检查与证据

系统 Python 缺少 PyYAML，使用临时依赖环境执行 `uv run --no-project --with pyyaml -- npm test`：Node 402 通过、27 跳过、0 失败；Python 三组 24、2、13 项全部通过，含 YAML 元数据校验。未修改 Phoenix runtime，无需该运行时的验收。

本次文字精简与依赖条件简化后重跑同一命令，结果相同；日志见验证目录的 `npm-test-text-cleanup.log`。

description 全文及前 100 字符均已回读；全文 92 字符，触发与排除条件完整。skill 引用链接可达，原文件确已删除，`git diff --check` 通过。

本地证据保存在 [验证目录](../local/use-prompt-dsl-validation/2026-10-29-odyssey/)：任务输入、实际回复、初始值与最终夹具、源码快照、哈希、产物字节核对结果及完整 npm 日志；不随 skill 分发。回复中的临时绝对路径是原执行位置，归档保留对应文件。

这是同模型、共享文件系统下的冒烟，不是隔离盲测或基线质量比较。未导出完整宿主工具轨迹；未验收实际安装后的自动发现、真实 UI 开发、多模型效果或提示词质量收益。上述结果仅支持已观察的路由和行为。
