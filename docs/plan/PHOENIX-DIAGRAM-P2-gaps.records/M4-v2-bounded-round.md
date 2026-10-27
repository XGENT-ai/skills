# M4 · v2 有界验收收尾

本轮64个冻结角色均已取证：**48 passed、14 failed、2 incomplete**，M4仍未完成，计划进度保持3/4。未变更支持范围，未将错误产物或技术中断记为通过；本文件收尾当前有界执行，不代表产品验收完成。

| 范围 | passed | failed | incomplete | 合计 |
| --- | ---: | ---: | ---: | ---: |
| 62正式角色 | 47 | 13 | 2 | 62 |
| 2额外ERD缺事实反例 | 1 | 1 | 0 | 2 |
| 全部冻结角色 | 48 | 14 | 2 | 64 |

59份实际draft已完成独审，为47passed/12failed；Cl026因原index拒绝访问越界而无draft，另行独审failed。Code048/053在capture技术中断，无final/draft；两个额外ERD本就captureOnly，不伪造正式行。Codex原graph容量失败及用户授权的一次同模型恢复映射保留，逻辑角色只计一次。

完整身份账本为[bounded-round-outcome.json](../../../local/phoenix-diagram/p2-gaps-provider-v2/bounded-round-outcome.json)，135894B，SHA `6844710af9f5d8093df6e30fd81d58bc1b3072edff657d9c6f6102e9615b0767`。它逐一绑定64个原result、冻结角色及评审。最终[检查点014](../../../local/phoenix-diagram/p2-gaps-provider-v2/review-checkpoint-014.json)139947B，SHA `5ee5fa2662962b55346c44c554f45929cedfff61cb76004b4f2f097e475024da`，核实原draft到reviewed仅按原helper允许的完整变化，认证45处原答复引文及两份原始workflow材料追加。原failed、Cl028原passed、全部旧检查点和原pending均保留。

## 未通过角色与原证据

以下所有Claude未通过角色依赖冻结的glm-5.3-flash；三个Codex未通过/中断角色依赖gpt-6.1-sol、reasoningEffort=max。自然架构的deepseek/deepseek-flash角色已通过，不能把下列问题归给它。

| 角色 | 结论与精确缺口 | 原报告/记录 |
| --- | --- | --- |
| Cl006 mermaid-flowchart | failed：再次修改/修复前未实际取得最新机械progress，且一次失败门error/best记错；完整失败诊断、图、身份正向保留，无越过stall2的证据。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/mermaid-flowchart/review.md) |
| Cl007 mermaid-graph | failed：Receipt无依据归入外部系统；三次修复前未实际读取当前机械progress。手写计数正确，未触发stall2。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/mermaid-graph/review.md) |
| Cl013 repairable-diagnostic | failed：最终candidate/summary以字面省略路径交接，无法直接定位；修复、四图及自动门通过。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/repairable-diagnostic/review.md) |
| Cl014 stalled-repair | failed：最终把单独加via承诺为可解除固定labelAt重叠；源码不支持。实际完整诊断、计数与到2停止均正确。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/stalled-repair/review.md) |
| Cl017 visual-pending | failed：五个人工动作的可见默认图例称Agent逻辑，材料未给出该执行者。用户明确允许视觉pending，不以未登记或少于四张作者图像另判失败。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/visual-pending/review.md) |
| Cl018 legacy-lifecycle | failed：原finalize为output/parent-resolution，无HTML；已读到not-derivable后仍两次尝试依赖操作，均由collector在Native启动前拒绝。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/legacy-lifecycle/review.md) |
| Cl023 intent-waterfall | failed：最终穷举空隙时漏150–151ms，错误断言只有三段；六跨度及图内事实保留，不扩写成漏画span。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/intent-waterfall/review.md) |
| Cl024 mermaid-er | failed：修复前未实际读取机械progress；最终称13物理列，实际11列。图内全部列/键/基数、失败诊断与四图通过。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/mermaid-er/review.md) |
| Cl026 repo-verified | failed、无draft：真实读取排除的Native实现，原index拒绝；普通入口图另有无依据Agent图例。其余未经完整复核部分不冒写passed。 | [独立诊断](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/repo-verified/review.md) |
| Cl027 repo-dirty | failed：默认Agent图例无依据；preflight ready边放在verifyEngine前，与固定源码顺序相反。真实partial/dirty披露、诊断/机械进度及停止处理正向保留。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/repo-dirty/review.md) |
| Cl028 repo-timeline | failed、r2：最终candidate/summary字面省略路径不可解析；其余五维passed。原passed及检查点011保留，后续检查点用supersededReviews绑定更正。 | [路径复核](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/repo-timeline/recheck.md) |
| Cl031 locale-zh-tw | failed：长假获准路径仅有超过三天、人资核准，缺主管批准前提；按冻结SKILL64/66不能用另有拒绝出口补足。其余五维passed。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/locale-zh-tw/review.md) |
| Cl额外ERD missing-facts | failed、仅提问形式：两必要缺口全覆盖、未造事实/图，但封闭式确认不满足冻结的开放式要求；自动/来源/视觉N/A。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/claude/erd-missing-facts/review.md) |
| Code049 legacy-lifecycle | failed：Live仍可转Health Paused，legacy默认success图例却称terminal success。该原件与复用V-5逐字节相同，V-5语义缺口同步保留。 | [独审](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/codex/legacy-lifecycle/review.md) |
| Code048 visual-pending | incomplete：原模型容量中断，已有首次finalize，但无最终答复/登记/draft；不判业务失败或虚构通过。 | [原result](../../../local/phoenix-diagram/p2-gaps-provider-v2/execution/codex/visual-pending/attempt-1/result.json) |
| Code053 intent-timeline | incomplete：ENOSPC在最终答复/draft前中断；用户随后释放空间不追溯消除原中断，未读取未认证的全局会话。 | [独立诊断](../../../local/phoenix-diagram/p2-gaps-provider-v2/reviews/codex/intent-timeline/review.md) |

## 独立验收结果

五个必需CI ZIP共2478650385B已恢复并认证API字节/SHA、全部成员CRC及安全路径。锁定run40314945875/attempt1/head8d7f92f；原aggregateCandidate配合认证API快照实际核四平台自动与精确Node18.20.8均pass。见[原检查器诊断](../../../local/phoenix-diagram/p2-gaps-ci-v2/aggregate-diagnostic-001.json)。宿主原归档要求31行全部passed，当前不能生成；本机failedstate真实保留，因此诊断总体incomplete，未宣称正式聚合通过。

本机原完整消费仍failed：workflow/lifecycle的15秒命令超时及architecture downstream reach未过。原workflow单例在原资源门、相同时限和同一控制器下实际20/20、Reader pass，50输入及原consumer不变，但未复现原超时、未证实根因，也未覆盖另外两处原失败。见[单例实际结果](../../../local/phoenix-diagram/p2-gaps-distribution-v2-diagnostic-1/probe-2/root-outcome.json)。不把它替换为完整消费passed。

## 已准备的下一步及边界

[默认图例提案v2](../../../local/phoenix-diagram/p2-gaps-provider-v2/proposed-default-legend-fix-v2/proposal.md)包含9文件/12个源字符串：legacy success去终态限定，workflow backend五语言默认改中性处理逻辑，并同步五语言目录与生成目录/模板。私有装配/check、目录深等、补丁检查及独立静态复核均过，规范源码和当前包未改；Native行为、真实四图、全测试及新分发尚未执行。

这只针对已定位的默认图例缺口，不能解决上表的最终事实、机械progress、路径、访问边界或技术中断。完整本机消费根因也未定位。依计划§9.5保留本轮所有失败并停止自动抽签式重试；下一阶段需明确具体修复和新身份的实际失效覆盖。任何新sourceInput/bundle/tgz或题面/collector身份均不能拼回当前正式cohort，不能自动开启新64角色或更换模型。未获得需求变更前，原64角色全部可追溯通过和原聚合通过的完成条件保持。
