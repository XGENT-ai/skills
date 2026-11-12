# R2 · 采集清单

- 目标包：[冻结目标](../../ELINTP-VISUALS-P2-gaps.ui-target.md)；重新计算的目标指纹为 `06c637df09c2a8979c811753f0537f22004713dfbfaee0e5423c57957cbba160`，与 R1 相同，未重建基线。目标文件 SHA-256 `ab9400900ff46a4470e381cddd3a173cb7321348dd5e5888b2e21bcae6e3cc11`；规范化输入与计算口径复用 [target-fingerprint.txt](../r1/target-fingerprint.txt)，无目标图附件。
- 当前被测 HEAD `b57ea7bcee33573cc10f4dd5799a470d1f18220c`；`git diff --binary HEAD` 的 SHA-256 为 `c61475e896d444b8bbe1446bef4b05e17d8f8f79682af910d5b0bfc738795bff`（含既有无关 dirty）；`skills/elintp`、`test/elintp` 工作树干净。版本与文件哈希见 [version.json](logs/version.json)。
- R2 主评审没有新拍截图，也没有把历史操作写成重新执行。当前 U1 成功状态复用 apply R1 的两张真实提交图及结果；其余 U1 状态复用 R1；U2 继承 R1 的独立已通过结论与原材料。
- 当前九个受测产品文件 SHA-256 与 apply `sourceAfter` 全部一致，当前 HEAD 已包含对应源码提交 `a72dbb62`。原图、原始结果和出处均保留，不搬运或改名。
- apply 工具：Playwright 1.63.0 / 系统 Chrome 157.0.8103.78；DPR 1；等待 networkidle、fonts.ready，截图关闭动画；真实公开 review CLI、loopback HTTP 与 `/submit`，无接口替身或数据拦截。
- R1 工具与条件：Playwright 1.63.0 / Chrome 157.0.8103.78；DPR 1；桌面 1440×900、窄屏 360×800；终稿为离线 file URL；打印为 Chrome PDF / pdftoppm 26.10.0。逐图原始时间、入口与工具见 [R1 capture.md](../r1/capture.md)。
- 实际操作观察：[observed-interactions.md](observed-interactions.md)。R2 主 agent 已打开两张 apply 图及交付给评审的 R1 U1 对照图，核对页面、视口、主题、文字和原始出处。

## 当前有效 apply 图片出处

| 原图（不搬入 R2） | 单元 · 状态 | 视口 / DPR / 主题 | 时间 UTC | 实际入口 / 工具 | 有效性 |
| --- | --- | --- | --- | --- | --- |
| [U1-submitted-after-1440-light.png](../r1-apply/U1-submitted-after-1440-light.png) | U1 · 主动提交成功 | 1440×900 / 1 / light | 2026-11-12T03:11:23.264Z | `http://127.0.0.1:55336/`；Playwright 1.63.0 / Chrome 157.0.8103.78 | apply 复用；当前产品文件哈希匹配 |
| [U1-submitted-after-360-dark.png](../r1-apply/U1-submitted-after-360-dark.png) | U1 · 主动提交成功 | 360×800 / 1 / dark | 2026-11-12T03:11:24.398Z | `http://127.0.0.1:55356/`；统一工具 | apply 复用；当前产品文件哈希匹配 |

对应完整结果：[桌面 result](../r1-apply/logs/after-1440-light.result.json)、[窄屏 result](../r1-apply/logs/after-360-dark.result.json)。同状态图片共同覆盖页面标题、成功反馈与完整提交结果，不按问题复制图片。

## 历史复用图片与结论

以下材料均保留 R1 原始出处，不冒充 R2 新采集；逐图地址、时间和工具以 [R1 capture.md](../r1/capture.md) 为准。

| 单元 · 状态 | 原图 / 原记录 | R2 用途 | 有效性 |
| --- | --- | --- | --- |
| U1 · 上轮提交对照 | [U1-submitted-1440-light.png](../r1/U1-submitted-1440-light.png) | 与当前成功状态对照 | 历史复用 R1 |
| U1 · 默认、续接、标注与恢复 | [default](../r1/U1-default-1440-light.png)、[continued](../r1/U1-continued-1440-light.png)、[annotated](../r1/U1-annotated-1440-light.png) | 未受影响状态 | 历史复用 R1 |
| U1 · 长引用编辑与焦点 | [U1-missing-edit-360-dark.png](../r1/U1-missing-edit-360-dark.png) | 未受影响状态 | 历史复用 R1 |
| U1 · 失败保留 | [U1-submit-failure-1440-light.png](../r1/U1-submit-failure-1440-light.png) | 未受影响状态 | 历史复用 R1 |
| U1 · 无脚本与打印 | [nojs](../r1/U1-nojs-1440-light.png)、[print-1](../r1/U1-print-1.png) | 未受影响状态；完整 PDF 原记录仍在 R1 | 历史复用 R1 |
| U2 · 全部已通过条件 | [R1 U2 图片组](../r1/capture.md#新图出处)与[操作 13–20](../r1/observed-interactions.md) | 继承 R1 独立评分 10.0，不在 R2 重评 | 历史复用 R1；当前源码指纹核对记录见 logs/version.json |

## 验收条件覆盖

“apply 复用”与“历史复用 R1”均指真实页面证据；没有证据类型的格子不推断通过。

| 目标条件 | 单元 | 当前 / 历史证据 | 类型 | 覆盖 |
| --- | --- | --- | --- | --- |
| C1 固定模板、亮暗与资源 | U1、U2 | apply 两张成功图；R1 default、missing-edit、U2 图片组 | apply + R1 复用 | 已覆盖 |
| C2 窄屏、长引用与宽图 | U1、U2 | apply 360 dark；R1 missing-edit、U2 missing 图片 | apply + R1 复用 | 已覆盖 |
| C3 键盘、焦点、文字与 alert | U1、U2 | R1 操作 4–8、19、21；missing-edit、submit-failure、outline 图片 | R1 复用 | 已覆盖 |
| C4 默认、已有标注、成功与失败 | U1 | apply 两张成功图与操作 1–2；R1 default、annotated、submit-failure | apply + R1 复用 | 已覆盖 |
| C5 无脚本、打印与离线 | U1、U2 | R1 nojs、print、U2 全组及操作 9–10、13–20 | R1 复用 | 已覆盖 |
| U1-1 两图标注、类型、编辑删除 | U1 | R1 annotated、missing-edit、submit-failure 与操作 4–11、21 | R1 复用 | 已覆盖 |
| U1-2 完整原引用 | U1 | R1 missing-edit；apply 操作 1–2 的完整结果 | apply + R1 复用 | 已覆盖 |
| U1-3 旧结果与同轮草稿续接 | U1 | R1 continued、annotated 与操作 1–3、11 | R1 复用 | 已覆盖 |
| U1-4 刷新、主动提交与失败保留 | U1 | apply 两张成功图、操作 1–2 的完整结果；R1 submit-failure 与操作 21 | apply + R1 复用 | 已覆盖 |
| U2-1～U2-4 | U2 | R1 U2 全组图片、操作 13–20 与已登记 CLI 实测 | R1 复用并继承已通过结论 | 已覆盖 |

## 待评单元与材料边界

- 本轮独立评审只重新评分 U1，并复核 UI-01、UI-02；可读取目标包、本清单、observed-interactions.md、上述 U1 当前与对照图片，以及 [R1 judge.md](../r1/judge.md) 的 U1 部分。
- U2 未受影响且 R1 已独立通过，继承原整组评分与结论，不要求评审重新打开全部 U2 图片或重新打分。
- 版本日志、CLI/页面机器检查、走查材料、计划、修复记录、代码与 diff 均不属于独立评审材料。
- 全量覆盖缺证 0；R2 新实拍 0；apply 复用 2 张；R1 历史材料按上表复用。未对触屏、真人速度或真实 Agent 后续改写作新推断。
