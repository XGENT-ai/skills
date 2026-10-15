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

## 当前重点

以下沿用各状态页原核查结果，本次精简未重判业务状态或重跑产品验收；继续前核对当前来源。完整对象见[状态导航](status/index.md)，历史按需读取。

| 工作项 / 原文 | 原登记进展与证据 | 下一步及完成条件 |
| --- | --- | --- |
| Phoenix UI（开发计划）· [docs/plan/phoenix-ui.md](../../docs/plan/phoenix-ui.md) · [状态](status/plan/7818a838-d021-4b2e-92c7-6d57c7ae8af0.md) | 实施 6/6（dev-plan-completed / confirmed），R3 修复范围通过；已合并；0.7.1 据[发布登记](events/plan/7818a838-d021-4b2e-92c7-6d57c7ae8af0/7b9d490a-c71d-4304-a483-2cba21a7d9a8.md)发布为 latest 并回读一致；0.7.0 误发不可安装，未 deprecate；rc 未发 npm | 后续版本用 `npm run release:phoenix`；发布工具如需评审，以计划路径 review-code。原结论与各轮限制见状态页。 |
| Codex guard（开发计划）· [docs/plan/codex-context-goal-guard.md](../../docs/plan/codex-context-goal-guard.md) · [状态](status/plan/b67ee46a-c9c1-4c5b-bd57-65fe9f1b6460.md) | 实施 3/3（dev-plan-completed / stale），初轮评审只覆盖原快照 | `$review-code docs/plan/codex-context-goal-guard.md`：覆盖后续阈值、Git 根目录路径和显式安装变更，明确受影响验收的证据。 |
| Claude guard（开发计划）· [docs/plan/claude-context-goal-guard.md](../../docs/plan/claude-context-goal-guard.md) · [状态](status/plan/90d15d49-af8b-4e97-bb61-dadb6498747c.md) | 实施 4/4（dev-plan-completed / stale）；未发现文档评审，就绪证据 provisional | `$review-code docs/plan/claude-context-goal-guard.md`：核对阈值、安装入口和 V-6 口径。与 Codex guard 共用安装器，涉及共享改动时统一核查。 |

[xgent-init 历史方案](../../docs/plan/xgent-init.md)保持[归档入口](status/plan/091b457b-53b0-4d16-bb58-406a02ccf1f6.md)，不恢复旧待办。开发完成不自动代表产品终验或发布。

## 台账维护

2026-10-15 按用户精简要求切换至协议 v4 / state-model v3。日常只更新当前页，必要时才新增补充事件；原 110 条事件及 ID 保留。活动页的历史清单已核对前序覆盖后压缩为末端和历史入口，归档页保留原观察。原产物、评审判定、核查时间及项目授权未改变；旧评审建议已改为指向计划当前状态，历史限制按原核查时点解释；唯一待决、未解问题和证据限制继续保留在对应状态页。无变化不重写，当前页后续直接回写的事实不能被旧事件覆盖。此次维护未提交、未推送。
