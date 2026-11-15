# ELINTP-VISUALS-P3-gaps · UI 验收评审 R1

## 评审对象

| 轮次 | 开发计划 | 目标包 | 被测版本 | 截图 | 上轮报告 |
| --- | --- | --- | --- | --- | --- |
| R1（首轮，新建目标包） | [ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md) | [ELINTP-VISUALS-P3-gaps.ui-target.md](ELINTP-VISUALS-P3-gaps.ui-target.md) · `0a84be6430e5` | `c7c77b8d`，dirty `1b6e8e02d42b`（未提交改动不涉及 `skills/elintp`、`tools/phoenix-diagram`）；终稿 `file://…/r1/logs/pages/final.html`，审阅页 `http://127.0.0.1:55563/` 等本机临时端口 | [r1](ELINTP-VISUALS-P3-gaps.ui-captures/r1/capture.md) | 无 |

## 验收判定

**通过验收**

本轮分数 9.5（最低单元 U1），门槛 8（默认）。没有 P0/P1 门槛项，没有停滞预警或待裁定事项。

两个单元的卡片位置、结构、内容与 Reader 一致，原文显示、亮暗跟随、窄屏单列、禁用脚本、旧图兼容和审阅页一致性都符合目标。剩下 UI-01–UI-03 三条 P2/P3，都是 1 px 级别的偏差，作为可选打磨项。

本判定只表示界面在所测版本和条件下的验收结论，不代表合并、发布或产品终验。

## 评分

| 单元 | 布局 | 视觉 | 交互与状态 | 细节 | 单元分 | 各评审单元分 | 达标 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| U1 终稿中的静态图卡片 | 2.5 | 3 | 3 | 1 | **9.5** | A 10 → B 9.5（补采 a2 后重评，取较低整组） | 是 |
| U2 审阅页中的静态图卡片 | 2.9 | 3 | 3 | 1 | **9.9** | A 10 → C 9.9（补采 a3）→ D 9.9（补采 a4；取 D 整组） | 是 |

- 本轮分数：9.5（U1）。全量轮：两个单元都有本轮独立评分，所有条件都有有效证据。
- 历史最佳：9.5。
- 门槛：8/10（用户未指定，取默认值，后续轮次沿用）。
- 四维理由见 [judge.md](ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge.md)。

## 轮次轨迹

| 轮次 | 本轮分数（最低单元） | 历史最佳 | 未关闭 P0/P1 | 重构轮 | 备注 |
| --- | --- | --- | --- | --- | --- |
| R1 | 9.5（U1） | 9.5 | 0 | — | 首轮；新建目标包；补采 3 次，重评 3 次 |

## 差距清单

### UI-01 · [P2] 窄屏滚过宽图末端后整页瞬时左移 1 px（U1 · 布局 · 可挽回 0.25）

- 位置：[U1-narrow-360-light-figure.png](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-narrow-360-light-figure.png) 对照 [U1-narrow-360-light-figure-scrolled.png](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-narrow-360-light-figure-scrolled.png)：第 1 张图下方卡片区与图注。
- 现状 → 期望：在图区域内横向滚轮越过末端，滚轮刚停时，图以外的整页内容左移 1 px，卡片左边框从 x=20 变成 x=19。期望是宽图只在自身区域滚动，页面不动（目标包 C2）。
- 修法：按评审 B 原文，先在 360 视口找出超出视口宽度的元素，修它的盒模型或网格列宽，不要用 `overflow-x: hidden` 遮掉。评审 D 对同一现象的修法是给图的横向滚动容器加 `overscroll-behavior-x: contain`。
- 来源：评审 R1（评审 B 新-1）。
- 核对说明（主 agent，不改分、不改差距）：
  - 同一上下文里重复操作 a3，滚轮刚停那一帧确实左移 1 px，1.5 s 后复位，见 [U1-…-scrolled-a3](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-narrow-360-light-figure-scrolled-a3.png) 与 [settled-a3](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-narrow-360-light-figure-scrolled-settled-a3.png)。
  - 一次滚到末端、不越过末端时，不出现位移。
  - 整个过程中页面 `scrollX`、`visualViewport.offsetLeft` 都是 0，`scrollWidth` 等于 `clientWidth`（360）。在 360 下，`.image-scroll` 之外没有元素的右缘超出视口。
  - 因此，原文「页面文档实际可以横向滚 1 px」与实测不符。现象更像滚动越过末端后传到页面引起的回弹。修复方请先核实成因再选修法。

### UI-02 · [P2] 打印时最右一张卡片缺右边框（U1 · 布局 · 可挽回 0.25）

- 位置：[U1-print-dark-a4-p2-a2.png](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-print-dark-a4-p2-a2.png) 的「成立的规则」、[p3-a2](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-print-dark-a4-p3-a2.png) 的「Escalation path」、[p4-a2](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-print-dark-a4-p4-a2.png) 的「失败处理」，局部见 [200 dpi 局部](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U1-print-dark-a4-p2-a2-right-200dpi.png)。
- 现状 → 期望：显式暗色下打印时，每页最右一张卡片的右侧竖边框缺失，底色到右缘后直接变白；屏幕上的右边框完整。期望是打印时卡片完整显示（K6、C3）。
- 修法：按评审 B 原文，检查 `@media print` 下卡片网格与 figure 的宽度规则：卡片用 `box-sizing: border-box`，列用 `minmax(0, 1fr)`，修完用 10 mm 页边距的 PDF 在 200 dpi 下复看右缘。
- 来源：评审 R1（评审 B 新-2）。主 agent 补采时也独立看到同一现象，没有另开编号。
- 核对说明：以打印媒体、宽 718 px 和 794 px 测量，卡片区和最右卡片的右缘都正好等于正文右缘（718.0/718，794.0/794）。卡片是 `border-box`，布局层面没有超宽。裁切发生在打印输出的可打印区域边缘，页边距 0 和 10 mm 两次打印都出现。供修复方定位，不改结论。

### UI-03 · [P3] 审阅页窄屏滚过宽图末端后整页瞬时左移 1 px（U2 · 布局 · 可挽回 0.1）

- 位置：[U2-annotated-scrolled-360-dark-a3.png](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U2-annotated-scrolled-360-dark-a3.png) 对照 [U2-annotated-scrolled-settled-360-dark-a3.png](ELINTP-VISUALS-P3-gaps.ui-captures/r1/U2-annotated-scrolled-settled-360-dark-a3.png)：顶部分隔线、「编辑标注」按钮、卡片左缘。
- 现状 → 期望：在第 2 张图区域内横向滚轮后，滚轮刚停时整页左移 1 px，1.5 s 后复位。期望是页面不动（C2）。
- 修法：给终稿和审阅页共用的图横向滚动容器加 `overscroll-behavior-x: contain`；同时核对 360 下 `scrollWidth` 不大于 `clientWidth`（评审 D 原文）。与 UI-01 很可能同源，修一次两处生效，但需要分别复核。
- 来源：评审 R1（评审 C 新-1 = 评审 D 新-1）。
- 核对说明：同 UI-01，审阅页在该时刻页面 `scrollX` 为 0，`scrollWidth` 为 360。

## 待裁定事项

无。

## 建议（不扣分）

见 [judge.md · 建议](ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge.md#建议不扣分)。其中观感最明显的是：

- **S1 打印分页**：小节眉题落在上一页底部，标题和图到了下一页。这属于阅读模板层的打印规则，目标包没有对应要点；是否本次引入未核实。
- **S2、S3 卡片偏高**：卡片网格等高拉伸，条目间距沿用正文，卡片显得偏高。Reader 卡片同样等高，现状与 Reader 一致。
- **S4 窄屏宽图缺可滚动提示**：图在 360 下被视口切断，没有滚动条或渐隐提示。

## 机器检查

| 检查项 | 工具与版本 | 结果 | 差距 |
| --- | --- | --- | --- |
| 控制台错误、页面异常、失败请求 | Playwright 1.63.0 / Chrome 157.0.8103.78 | 首采全部上下文 0 条（logs/events.json） | — |
| 终稿离线请求 | 同上，离线上下文拦截非 `file:`/`data:` 请求 | 0 | — |
| 页面级水平溢出 | 同上，`scrollWidth` 对 `innerWidth` | 终稿 1440/360 × light/dark、审阅页 1440 light 与 360 dark 均相等；360 下 `.image-scroll` 之外没有超出视口的元素 | UI-01、UI-03 为瞬时回弹，见核对说明 |
| 卡片内容与 Reader 一致 | 同上，读页面 `.figure-card` 与 Reader `.cards` 文本 | 三组卡片的标题、条目、顺序逐项相同 | — |
| 结构（K1） | 同上，读 DOM | 每个带卡图的子元素顺序为 `.image-scroll` → `.figure-cards` → `figcaption`；卡片区内标题元素 0；大纲 4 项，都是小节 | — |
| 样式与 token（K4–K6） | 同上，计算样式 | 底色 = `--surface`，边框 = `--line` 1px，标题字重 650，`gap` 12/12，`margin-top` 12；1440 列宽 272/414 px，360 单列 320 px；暗色与打印换用对应变量；打印时 `break-inside: avoid` | — |
| 卡片文字对比度 | 同上，按 WCAG 公式计算 | 亮 12.27、暗 11.31、打印 14.73（标题与条目同色） | — |
| 旧图兼容（K7） | 同上 | 第 4 张图没有 `.figure-cards` | — |
| 无脚本 | 同上，`javaScriptEnabled=false` | 3 个卡片区、7 张卡片，主题按钮隐藏，无溢出 | — |
| 打印几何 | 同上，打印媒体下 718/794 px | 最右卡片右缘等于正文右缘，无布局超宽 | UI-02 核对说明 |
| iframe 与字体资源 | 同上 | iframe 0；字体规则 6 条，全部来自 embedded 策略的第 2 张图 | — |
| 可访问性扫描（axe 等） | — | 未检查：本轮没有运行扫描工具，对比度按计算样式单独测量 | — |
| 键盘走查 | — | 未检查：卡片区没有新增交互控件；审阅控件沿用既有规则，不在比较范围内 | — |

## 采集与覆盖

- **环境与版本**：
  - HEAD `c7c77b8d3512a0030b1ea7b4e8125b83498a1a69`，`git diff --binary HEAD` SHA-256 `1b6e8e02…ea13d`。`skills/elintp`、`tools/phoenix-diagram` 相对 HEAD 无改动，受测文件哈希见 logs/source-sha256.txt。
  - 静态图由开发引擎 `local/phoenix-diagram/elintp-p3-gaps/phoenix-diagram-darwin-arm64` 生成，构建清单的 191 个源输入与工作树逐文件一致，所以没有重新构建。
  - 页面由当前 `render.mjs`、`review.mjs` 生成。
  - 目标包 SHA-256 `e74d0ef5…d89133`，无目标图附件；指纹的计算口径见 [target-fingerprint.txt](ELINTP-VISUALS-P3-gaps.ui-captures/r1/target-fingerprint.txt)。
- **采集**：
  - 首采 22 张：U1 15 张（含打印 4 页），Reader 对照 3 张，U2 4 张。
  - 补采 a2：窄屏宽图滚动 1 张，10 mm 页边距打印 5 页，200 dpi 局部 1 张。
  - 补采 a3：U1、U2 窄屏滚轮刚停与 1.5 s 后各 2 张。
  - 补采 a4：U2 窄屏图注与长条目卡片 2 张。
  - 合计 35 张，全部为本轮实拍，没有历史复用，无缺证。逐张出处与覆盖映射见 [capture.md](ELINTP-VISUALS-P3-gaps.ui-captures/r1/capture.md)，操作见 [observed-interactions.md](ELINTP-VISUALS-P3-gaps.ui-captures/r1/observed-interactions.md)。
  - 主 agent 逐张打开核对过。
- **补采的原因**：
  - a2：评审 A 报告看不出窄屏宽图自身滚动，以及打印页边距来源。首采 PDF 未设页边距，内容贴纸边，属于采集参数问题，补采改用 10 mm。
  - a3：核对评审 B 指出的第 10 条记录措辞。原写「页面本身未横向移动」，依据只是 `scrollX` 读数，已按截图事实更正；同时补 U2 窄屏宽图滚动的证据。
  - a4：评审 C 报告看不出 U2 窄屏下的长条目换行和图注。首采覆盖表把它们算作已覆盖，实际证据不足，这是主 agent 的覆盖规划疏漏。
- **环境噪声**：U2-enabled-1440-light 左侧大纲「旧静态图」一项处于悬停态。原因是点工具条按钮后鼠标停在那里，已实测 `:hover=true`，不属于被测状态。首次 zsh 循环误写了两个空名日志文件，在 logs/ 内，已被忽略。
- **评审方式**：
  - 共 4 位全新上下文、只读的 subagent，各自单独评审，模型都继承主 agent。
  - A 首评 U1、U2；补采后，B 重评 U1，C、D 依次重评 U2。
  - 每个单元取总分较低那位的整组分数，差距取并集。
  - 没有确认评审，也不是单上下文评审。各评审原文：[A](ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge-a.raw.md)、[B](ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge-b.raw.md)、[C](ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge-c.raw.md)、[D](ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge-d.raw.md)。
  - 评审材料只有目标包、截图、capture.md 和操作记录，没有给代码、计划、机器检查结果或门槛。
- **盲测走查**：检测到走查子代理，但两个单元都只是阅读和标注展示，没有填表提交、多步流程这类主要操作流程，所以没有派。
- **主 agent 核对说明**：见 UI-01–UI-03 下方，都只附事实，不改分、不删差距。
- **范围与前提**：
  - 门槛取默认值 8。
  - 首轮目标包的期望来自 PRD 与计划，未经用户确认。窄屏高度 800、DPR 1、代表输入（三份 candidate 与 P3 旧图），以及 U2 只覆盖桌面亮色和窄屏暗色，都是本轮采集的假设。
  - PNG、PDF 和运行日志按仓库忽略规则不入库，只有原工作树能完整复现。
