# sdlc 索引

用途：本层状态资料与下级入口；原文档和证据保留原位置。

覆盖本目录全部直接文件和子目录，不自列。按相对路径稳定排序；索引不计作事实事件。

| 文件 / 子目录 | 用途 / 对象 ID / 事件范围 | 入口 |
| --- | --- | --- |
| event-template.md | 按需补充事件模板，普通进展不使用 | [入口](event-template.md) |
| events | 历史及必要补充事件，按需追溯 | [入口](events/index.md) |
| protocol.md | 状态规则或模板 | [入口](protocol.md) |
| state-model.md | 状态规则或模板 | [入口](state-model.md) |
| status | 下级导航 | [入口](status/index.md) |

## 维护约定

改变本目录内容的执行者按[目录索引维护规则](protocol.md)（目录索引维护一节）同步索引：先保存事实和状态，再维护本层及入口变化的祖先；无变化不重写。创建子目录时同步建立其 index.md；合并按 ID 与路径保留双方记录，缺失目标先标待核对，不按时间覆盖或删除历史。查询只报告问题。

## 工作项入口

当前状态、证据与下一步见各对象页；完整活动及归档对象见[状态导航](status/index.md)。本节仅维护工作项入口，不随进展刷新。

| 工作项 / 类型 | 原文 | 状态页 |
| --- | --- | --- |
| Phoenix UI（开发计划） | [docs/plan/phoenix-ui.md](../../docs/plan/phoenix-ui.md) | [状态](status/plan/7818a838-d021-4b2e-92c7-6d57c7ae8af0.md) |
| Codex guard（开发计划） | [docs/plan/codex-context-goal-guard.md](../../docs/plan/codex-context-goal-guard.md) | [状态](status/plan/b67ee46a-c9c1-4c5b-bd57-65fe9f1b6460.md) |
| Claude guard（开发计划） | [docs/plan/claude-context-goal-guard.md](../../docs/plan/claude-context-goal-guard.md) | [状态](status/plan/90d15d49-af8b-4e97-bb61-dadb6498747c.md) |
| AGENTS.md 与 Skills 自进化（开发计划） | [docs/plan/agent-instruction-evolution.md](../../docs/plan/agent-instruction-evolution.md) | [状态](status/plan/bbae83cd-7065-4f90-a350-5e6439fdfed6.md) |

[xgent-init 历史方案](../../docs/plan/xgent-init.md)保持[归档入口](status/plan/091b457b-53b0-4d16-bb58-406a02ccf1f6.md)。
