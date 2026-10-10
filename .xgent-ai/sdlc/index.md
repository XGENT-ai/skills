# sdlc 索引

用途：本层状态资料与下级入口；原文档和证据保留原位置。

覆盖本目录全部直接文件和子目录，不自列。按相对路径稳定排序；索引不计作事实事件。

| 文件 / 子目录 | 用途 / 对象 ID / 事件范围 | 入口 |
| --- | --- | --- |
| event-template.md | 状态规则或模板 | [入口](event-template.md) |
| events | 下级导航 | [入口](events/index.md) |
| protocol.md | 状态规则或模板 | [入口](protocol.md) |
| state-model.md | 状态规则或模板 | [入口](state-model.md) |
| status | 下级导航 | [入口](status/index.md) |

## 维护约定

改变本目录内容的执行者按[目录索引维护规则](protocol.md)（目录索引维护一节）同步索引：先保存事实和状态，再维护本层及入口变化的祖先；无变化不重写。创建子目录时同步建立其 index.md；合并按 ID 与路径保留双方记录，缺失目标先标待核对，不按时间覆盖或删除历史。查询只报告问题。

## 当前重点与发现范围

- 2026-10-10：[Phoenix UI计划](status/plan/7818a838-d021-4b2e-92c7-6d57c7ae8af0.md)仍dev-plan-ready / dev-plan-in-progress、1/6。第十二次CI四个非Windows native成功，Windows路径回归失败，两后置smoke跳过；精确路径候选本机23项/oracle860及Node191/Python61通过，准备第十三次正常推送。[M2记录](../../docs/plan/phoenix-ui.records/M2.md)与[M5记录](../../docs/plan/phoenix-ui.records/M5.md)保存各自范围；M5隔离库102项、矩阵/图片证据核心通过，完整入口正补存退出码，完整协议/CLI继续。其他guard事项及首轮评审处置保留。

- 本轮核查：2026-10-07T04:11:20+11:00；本地 main / `1f9d85651c33dc1ca11ddcda7dac2b69bbe30c74`；观察开始时 clean，保存后仅台账 dirty。未 fetch/pull，未核实远端及其他机器的实时状态。
- 协议 v3 沿用；本轮确认 state-model.md / event-template.md 指纹与原版 v1 一致，按兼容规则升级至 state-model v2 和新版事件模板。旧事件及对象 ID 保留，归档页保留原观察。新增 6 条接续事件，共 16 条；仍登记 3 份计划（2 份活动、1 份归档）、2 份独立评审，未发现独立 PRD。
- 首选：[Codex guard](status/plan/b67ee46a-c9c1-4c5b-bd57-65fe9f1b6460.md)：dev-plan-ready / dev-plan-completed（3/3），证据 stale。计划评审 F-01–F-09 已 review-applied；初轮代码评审 review-not-required 仅覆盖原快照。执行 `$review-code docs/plan/codex-context-goal-guard.md`，复审后续阈值、Git 根目录路径和显式安装变更，明确受影响验收的证据覆盖。
- 可独立跟进：[Claude guard](status/plan/90d15d49-af8b-4e97-bb61-dadb6498747c.md)：dev-plan-ready（provisional，未发现文档评审）/ dev-plan-completed（4/4，stale）。执行 `$review-code docs/plan/claude-context-goal-guard.md`，核对默认 70% 与历史 65% 记录、当前安装入口和 V-6 口径；Opus/Sonnet 真实触发与生产 --resume 为原计划声明的后续范围。两项共用安装器，若后续修复触及共享文件需统一处理。
- 两份计划的完成记录均无提交列；里程碑记录存在，所引用起始基线及主分支成果提交可达。独立 Codex 验证 ref `63fc5faf56f5ac186211501f065b4d2a96479491` 可取回但非 main 祖先，只是历史验收快照。按 v2 保留已完成状态，以证据质量表达后续改动覆盖限制，未发现应重开原里程碑的实际失败证据。
- 后续提交范围：`54c565c`（可配置阈值）、`f7c4ea2`（Codex Git 根目录路径）、`4d70f33`（两种 guard 取消安装询问，默认跳过、显式 flag 安装）。Claude 原计划 §5.5 / V-6 与 M3 记录仍描述询问；源文同步及新行为验证交由后续评审核对，本轮不改原文。
- [xgent-init 历史方案](status/plan/091b457b-53b0-4d16-bb58-406a02ccf1f6.md) 按原文历史标记保留归档入口，不重新判定就绪/实施或推荐旧待办。验收/集成/发布统一枚举仍 TBD；计划 completed 与历史评审通过不等于当前产品终验或发布。
- 发现范围：本地项目文档、docs/plan 及其记录、原文关联报告、相关 hooks/测试/安装器与可用 Git 历史；技能模板/安装副本、依赖、机器缓存不列为项目工作项。evals/agi-mode/practices 是技能设计/验证记录，不是项目开发计划。未展开归档历史逐项验收，忽略目录及其他机器未盘点。[README](../../README.md) 提供当前技能说明。
- 本轮仅升级状态口径、校准跟进与建议并检查结构；未执行业务测试、apply 或新评审，未提交/推送，本次更新尚未随 Git 共享。
