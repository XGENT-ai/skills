# ELINTP-VISUALS-P4-gaps · UI 验收评审 · R2

## 评审对象

| 轮次 | 开发计划 | 目标包 | 被测版本 | 截图 | 上轮报告 |
| --- | --- | --- | --- | --- | --- |
| R2（复审：UI-01/UI-03 修复后全量重评） | [ELINTP-VISUALS-P4-gaps · 数值图表补缺](ELINTP-VISUALS-P4-gaps.md) | [ELINTP-VISUALS-P4-gaps.ui-target.md](ELINTP-VISUALS-P4-gaps.ui-target.md) · 目标指纹 `cbb77004f7a8`（与 R1 相同，未重建基线） | 仓根 `84f91b5e`（含 UI修复 `9e2fc134`；未提交改动仅 docs 台账与评审文档，`tools/phoenix-diagram` 无差异）· `file://` 本地产物 | [r2](ELINTP-VISUALS-P4-gaps.ui-captures/r2/) | [R1](ELINTP-VISUALS-P4-gaps.ui-review.md) |

## 验收判定

**通过验收**（停滞预警：UI-03 所在 U2，首次触发）。

本轮分数 **9**（U2），门槛 8/10，无门槛项；待裁定 1 项（计划 §5.2 文字回填，不属界面差距、不阻断）。理由：UI-01 修复在真实界面复核达标（收尾扇区与列表色块、节点索引 chip 均为绿系强调色）；UI-02、UI-04 经评审放大复核与主 agent 独立实测双重佐证为 R1 在整页缩略视图下的误读，本轮关闭为不再适用；唯一未关闭差距 UI-03 为 P2（低负载点标注与系列线碰撞），不阻止通过，列为可选打磨项。

## 评分

| 单元 | 布局 | 视觉 | 交互与状态 | 细节 | 单元总分 |
| --- | --- | --- | --- | --- | --- |
| U1 两系列柱状图（en） | 3 | 3 | 3 | 1 | **10** |
| U2 两系列折线图（zh-CN） | 2 | 3 | 3 | 1 | **9** |
| U3 限额图（zh-CN） | 3 | 3 | 3 | 1 | **10** |
| U4 六行饼图（en） | 3 | 3 | 3 | 1 | **10** |

- 本轮分数：**9**（最低单元 U2）。全量轮：U1–U4 全部本轮重评，所有条件均有有效证据。
- 历史最佳：**9**（基线自 R1 建立后未重建；R1 最佳 8.0）。
- 门槛：8/10（用户未另行指定，沿用 R1 默认）。
- 评审方式：单一评审，无确认评审、无重评，无分数拼接。

## 轮次轨迹

| 轮次 | 本轮分数（最低单元） | 历史最佳 | 未关闭 P0/P1 | 重构轮 | 备注 |
| --- | --- | --- | --- | --- | --- |
| R1 | 8.0（U2） | 8.0 | 1 | — | 首轮；4 条差距（UI-01 P1、UI-02/03/04 P2） |
| R2 | 9.0（U2） | 9.0 | 0 | — | UI-01 已解决；UI-02/UI-04 不再适用；UI-03 仍存在；停滞预警（UI-03 · U2） |

## 上轮差距复核

| 编号 | 上轮严重度与摘要 | 上轮处置（M2 记录 · UI 验收修复） | 本轮结果 | 依据 |
| --- | --- | --- | --- | --- |
| UI-01 | P1 · U4 收尾扇区与列表色块为中性灰，非 S2 绿系强调色 | 已修改待评审（`9e2fc134`：新增 `--accent-stroke` 强调色槽，饼图收尾行扇区、列表色块、节点索引 chip 同槽） | **已解决** | r2 U4 default/static 亮暗四图：收尾扇区、列表第 6 行色块、节点索引 chip 均为绿系强调色（亮=橄榄黄绿、暗=亮青柠），与 Tests 青绿可辨、色块与扇区同色（评审 2–3x 放大＋主 agent 逐张开图核对） |
| UI-02 | P2 · U1/U2 图例系列名疑似贯穿横线（删除线观感） | 不采纳（实测 `text-decoration` 全链 none，横线为画布点阵观感） | **不再适用** | 评审对 R1 原图同位置 2x 放大复核无贯穿线（等宽小字字形叠加的缩略图错觉），静态图例同查无；主 agent 独立实测：图例名 `text-decoration-line: none`，`--canvas-dot` 置透明后横线消失（[对照图](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/factcheck/bar-legend-light-canvasdot-off.png)） |
| UI-03 | P2 · U2 低负载点两条标注压叠、被标记与轴线压住 | 已修改待评审（`9e2fc134`：标注改为向远离轴线一侧堆叠） | **仍存在** | R1 所记「圆点压数字、轴线穿字」已不复现（标注在标记与轴线上方）；但 p95 线穿过「B 0.3」字形、两行标注紧贴（评审放大复核＋主 agent 4x 抽查，见差距清单核对说明） |
| UI-04 | P2 · U3 仅上限虚线框疑似画过上限刻度 | 不采纳（实测框右缘=刻度位置） | **不再适用** | 评审 6x 放大：虚线段止于上限刻度，右侧为三行共有的轨道浅色带；主 agent 实测：虚线框右缘 x=896.3 与上限刻度 x=896.3 完全重合（亮暗一致）（[factcheck-r2.json](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/interact/factcheck-r2.json)） |

已关闭且未受影响的条目：无（R1 四条差距全部在本轮复核）。

## 差距清单

### UI-03 · [P2] U2 低负载点数值标注与系列线碰撞（U2 · 布局 · 可挽回 1.0）

- 位置：[U2-default-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r2/U2-default-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r2/U2-default-1440-dark.png)、[U2-table-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r2/U2-table-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r2/U2-table-1440-dark.png)「低负载」数据点（画布中部偏左下、x 轴上方）；U2-static 两主题同位置（同源缺陷，S6 一致）。
- 现状 → 期望：「A 0.1」与「B 0.3」两行标注上下紧贴，p95 绿线从「B 0.3」文字上穿过（穿过小数点与「3」），p50 蓝线贴着两行右缘穿过行间 → 数值标注互不重叠、不被系列线穿过、与点标记和轴线留出间隙（依据：比较口径「没有溢出、遮挡、截断」；S7 图自明、S8 数值可读）。
- 修法：改折线图数值标注定位逻辑——同一 x 附近多系列点重叠时按系列序把标注分别放到点上方/下方错开（最小间距≥一行标注高）；标注放置后与折线段做相交检查，相交则向无线的另一侧偏移；标注与圆点标记间留 ≥2px 间隙。现有 SVG 文本元素内调整坐标即可，不需要换组件或重做素材。
- 来源：评审 R2（沿用 R1 编号）。
- **停滞预警（UI-03 所在 U2，本轮首次触发）**：R1 对它的最终处置为「已修改待评审」而本轮仍判存在。下一轮若继续修复，须先诊断原因——本次修复只把标注移到标记与轴线上方，未做与系列线段的避让——再选择最小有效修复、补证或提交争议；不强制重构。
- 核对说明（主 agent，不改分、不删差距）：对 r2 图 4x 放大抽查（[U2-lowload-light-zoom4x.png](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/factcheck/U2-lowload-light-zoom4x.png)）：p95 绿线确实穿过「B 0.3」的「0」与「3」字形；「A 0.1」「B 0.3」为两行可分辨文本，1x 下行间净空约 1–2px（评审「行距≈0」为观感描述，实测两行未合并）；标注块底缘与圆点标记净空约 5px（评审「间距≈0」偏严）；圆点标记位于轴线上是 0.1≈0 的必然几何。以修复方下一轮实测为准。

本轮无新增差距；无 P0、无 P1。

## 待裁定事项

1. **计划 §5.2 文字与实现的差异（文档回填与否）**——非界面差距，不影响本轮验收判定。
   - 原因：修复轮为实现目标包 S2「绿系强调色、与其他色槽同等醒目」，为饼图收尾扇区/列表色块新增了专用强调槽 `--accent-stroke`；计划 §5.2 字面仍写 `var(--external-stroke)`（中性灰）。相关差距 UI-01 已按目标包验收解决，仅计划文档文字未同步。
   - 相关差距：UI-01（已解决）。
   - 已有尝试：修复轮按目标包实现，并在 M2 记录注明差异、留待用户确认。
   - 可选项：a) 确认后回填计划 §5.2 文字为强调槽方案；b) 保持计划原文留档。

## 建议（不扣分）

- 建-1：节点索引摘要按固定宽度截断数字（U1「After 26…」、U2「p95 2…」；U3 三节点完整）。S1–S8 未覆盖节点索引；建议加宽摘要列或超宽换行/隐藏。
- 建-2：画布利用率低（U1/U2 绘图区约占中部 360×370px）。无设计稿约束，属偏好。
- 建-3：数据表数值列左对齐，0.1 与 1234567 悬殊位数不便比大小，可改右对齐。规范未约束。
- 建-4：U2 y 轴刻度显示原始数字（2500000 ms），可加千分位分隔符（不违反 S8）。规范未要求。

打磨 UI-03 时可顺带处理以上建议；不做也不影响已达成的验收。

## 机器检查

| 检查项 | 工具与版本 | 结果 | 对应差距/条件 |
| --- | --- | --- | --- |
| 引擎 finalize/static 自动检查（4 candidate × finalize＋static） | phoenix-diagram 0.2.0（HEAD `84f91b5e` dev 构建） | 8/8 exit 0，status=pass，diagnostics 0（[r2/logs/products/](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/products/)） | S6、S8 前置 |
| 静态 SVG `data-phoenix-table` 与阅读器表格逐格比对 | node 脚本（[table-parity.cjs](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/interact/table-parity.cjs)） | 4/4 same=true（表头与全部行/格一致） | S6 |
| 数据表展开＋首行复制、横向溢出、控制台与失败请求监测 | Playwright + 系统 Chrome（[interact.cjs](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/interact/interact.cjs)） | 8/8 copyOk；剪贴板为纯数字＋制表符、无单位/系列键/%；无溢出（1440/1440）；无控制台错误、无失败请求（[interact-results.json](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/interact/interact-results.json)） | S3、S7 |
| U1/U3 引擎自动截图与 R1 逐字节比对 | `cmp` | 16/16 相同（U1/U3 视觉未变，复用有效）；U2/U4 截图不同（差异与修复范围一致）；html/svg 与 R1 不同（模板装配含新增强调色槽，预期） | 证据有效性 |
| UI-04 几何实测（仅上限行虚线框 vs 上限刻度） | Playwright 读取 DOM（[factcheck-r2.cjs](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/interact/factcheck-r2.cjs)） | 虚线框右缘 x=896.3 = 该行上限刻度 x=896.3（亮暗一致）；刻度右侧为三行共有的浅色轨道带 | UI-04（关闭佐证） |
| UI-02 样式实测（图例名贯穿线） | Playwright + 3x 裁剪（[factcheck-ui02.cjs](ELINTP-VISUALS-P4-gaps.ui-captures/r2/logs/interact/factcheck-ui02.cjs)） | 图例名 `text-decoration-line: none`；`--canvas-dot` 置透明后贯穿横线消失（亮色对照 4 张） | UI-02（关闭佐证） |
| 目标指纹复算 | python（沿用 R1 保存口径） | 当前目标包「单元／规范要点」文本与 R1 保存的规范化文本逐字节一致，指纹 `cbb77004f7a8` 未变 | 基线 |

未检查：悬停/按下/焦点可见与键盘走查、axe/Lighthouse 对比度扫描——目标包采集矩阵未含（与 R1 一致），评审已列为「看不出」格子；不在单元矩阵内的工具栏控件（主题开关、Classic/经典、Still/静态、导出、全屏、缩放）未检查。

## 采集与覆盖

- 环境与被测版本：仓根 HEAD `84f91b5e1e24a9a13b9ac5e370142fbb8e6bc0ac`（含 UI修复 `9e2fc134`）；`git status` 核对 `tools/phoenix-diagram`、`dist/phoenix-diagram` 无未提交差异；引擎按计划 §9.1 从当前代码离线锁定构建（dev 调试版，0.2.0）。`file://` 打开本地产物，无网络请求。
- 采集工具与条件：finalize/static 内建浏览器自动截图（1440 宽整页，light/dark）；Playwright + 系统 Chrome（1440×900、DPR 1、`reducedMotion: reduce`、`?theme=light|dark`、networkidle、`document.fonts.ready`）；固定输入、无随机内容；无账号、无业务数据写入。
- 目标文件 hash 清单：输入 4 份与 R1 逐字节相同（bar `08661f06…`、line `389db7d6…`、limits `f2b944bc…`、pie `cfc6302d…`）；目标指纹 `cbb77004f7a8`（口径与输入见 [r1/target-fingerprint.txt](ELINTP-VISUALS-P4-gaps.ui-captures/r1/target-fingerprint.txt) 与 [r2/capture.md](ELINTP-VISUALS-P4-gaps.ui-captures/r2/capture.md)）。
- 证据类型：本轮实拍 24 张（引擎自动 16＋交互 8）；本轮实测 8 轮复制＋4 份逐格比对＋两项争议几何/样式实测；无历史复用、无 apply 图引用（U1/U3 引擎截图与 R1 逐字节相同，直接以本轮新拍呈现）；缺证：无（en/es「limit only」不在单元矩阵，目标包已注明）。
- 唯一截图数与覆盖映射：24 张，逐格映射与出处见 [r2/capture.md](ELINTP-VISUALS-P4-gaps.ui-captures/r2/capture.md)；无补采。
- 环境噪声：无。
- 评审方式：1 位全新上下文只读 subagent（agent `reviewer`，模型 `zai-coding-cn/glm-5.3-flash`），U1–U4 全量重评；无确认评审、无重评、非单上下文；评审原文 [judge-a.raw.md](ELINTP-VISUALS-P4-gaps.ui-captures/r2/judge-a.raw.md)，合并结论 [judge.md](ELINTP-VISUALS-P4-gaps.ui-captures/r2/judge.md)。
- 主 agent 对评审结论的核对说明：UI-03 附核对说明（见差距清单）；UI-02、UI-04 的关闭由主 agent 以独立实测佐证（见机器检查）；主 agent 未以自己核实新增差距。
