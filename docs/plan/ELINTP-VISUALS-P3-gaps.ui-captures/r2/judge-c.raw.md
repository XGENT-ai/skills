# R2 · 评审 C 原文

- 派发：补采 a3 后，全新上下文 general-purpose subagent 只重评 U1，只读、不写文件，模型继承主 agent（claude-opus-5-5），2026-11-14。材料同评审 B，另含 a3 图片及 capture.md「补采 a3」节；未给评审 A、B 原文、代码、diff、修复记录、机器检查、门槛。
- 以下为评审返回原文；仅「开图轨迹」中 r2、r1 图片条目省去了各自目录的绝对路径前缀（目录见各组标题），其余未改动。

---

## 评分表（U1 终稿中的静态图卡片）

| 维度 | 分 | 理由 |
| --- | --- | --- |
| 布局 | 3 | 已是满分。1440 下卡片区紧跟图、在图注之前，三列每列 272 px、两列每列 414 px，列间距 12 px，图底到卡片区上沿约 12 px。360 下单列，卡片在 x=20–339，不超出正文宽度。宽图横滚刚停和停稳两帧位置相同，页面不动。打印时右列卡片右边框完整，右边与分隔线右端对齐。上一轮扣分的两处都已消失。 |
| 视觉 | 3 | 已是满分。亮色卡片底 (238,242,239)、边框 (198,208,205)；暗色卡片底 (32,50,54)、边框 (69,95,97)，都跟着页面切换。显式暗色下打印出来是亮色。标题加粗，条目是正文色，没有 dot 颜色。和 R1 的判断一致，没有退化。 |
| 交互与状态 | 3 | 已是满分。主题按钮能切到暗色。禁用脚本时页顶没有主题按钮，三个卡片区共 7 张卡片全部显示，旧图下没有卡片区。打印 5 页，没有一张卡片被分页切开。360 下宽图能在自己的区域里滚到最右，页面不跟着动。 |
| 细节 | 1 | 已是满分。三组卡片和 Reader 对照图逐项比过，标题、条目、顺序都一样（规则判定卡片在前）。`&lt;b&gt;`、`&amp;` 按原文显示。长条目在卡片内换行，没有截断。大纲只有 4 个小节，没有卡片标题。 |
| **U1 合计** | **10** | 上一轮 9.5 |

## 上一轮差距复核

| 编号 | 上一轮摘要 | 本轮状态 | 依据 |
| --- | --- | --- | --- |
| UI-01 | 360 下在第 1 张图里横滚越过末端，滚轮刚停时整页左移 1 px | **已解决** | 我在内存里读了截图像素（没写任何文件），取 y=420/600 两行。本轮刚停帧（`r2/U1-narrow-360-light-figure-scrolled.png`）和停稳 1.5 s 帧（`…-scrolled-settled.png`）的卡片边缘都在 x=20–339，和 R1 滚动前的 `U1-narrow-360-light-figure.png` 相同。R1 刚停帧 `…-scrolled-a3.png` 在 x=19–338，就是那 1 px 的偏移。本轮两帧间没有位移。 |
| UI-02 | 显式暗色打印时，每页最右一张卡片缺右边框 | **已解决** | `U1-print-dark-a4-p2/p3/p4-right-200dpi.png` 里最右卡片右边框和圆角都在，右边与分隔线右端对齐（200 dpi 局部图中 x≈376）。R1 的 `U1-print-dark-a4-p2-a2-right-200dpi.png` 右边是被切掉的。p2–p4 整页里卡片完整。 |

## 差距清单

没有未关闭的差距。可挽回分合计 0，和 10 − 10 = 0 对得上。

## 建议（不扣分，目标包里没有对应依据）

- **S1（沿用）** 打印时小节眉题落在上一页底部，标题和图在下一页。本轮仍可见：p1 底的「为什么成立」、p2 底的「量级」、p3 底的「风险与前提」。可以在打印样式里给眉题加 `break-after: avoid`，或把眉题和 h2 包在一起用 `break-inside: avoid`。
- **S2（沿用）** 卡片网格等高拉伸，短卡下半部是空的，例如 `U1-default-1440-light-01-a2.png` 里「概况」「成立的规则」两张。可以在卡片网格上加 `align-items: start`。
- **S3（沿用）** 卡片条目用的是正文行高和列表间距，卡片偏高。可以单独收紧卡片内 `li` 的间距。
- **S4（沿用）** 360 下宽图被视口切断，没有可横滚的提示。
- **S5（沿用，归 phoenix-diagram）** 暗色下图底 (2,6,23)、页面 (21,33,36)、卡片底 (32,50,54) 三层颜色接缝明显（`U1-theme-1440-dark-01/02-a2.png`）。
- **S6（沿用）** 卡片标题看起来和条目字号接近或略小，层级主要靠字重区分。
- **S7（沿用，无法从截图核实）** 图滚动容器的 `overscroll-behavior-x: contain`：被测提交说明里提到了，但截图看不出是否用了这个属性。现象层面 UI-01 已解决，这条可以关闭。
- **S8（沿用，归 phoenix-diagram）** 360 下图滚到最右时，可见区域大多是空的分组框（`r2/U1-narrow-360-light-figure-scrolled*.png`）。
- **新建议 S9** 打印时图区底色不印出来，SVG 内部图例下方的留白在打印里像图和卡片之间的大空当（p3、p4 约 70 px@80 dpi，约 22 mm）。屏幕上这段留白在灰底里，看不出来。卡片区上方 12 px 的规定本身是满足的。如果要处理，应由 phoenix-diagram 收紧 viewBox 底部留白，或在打印样式里保留图区底色。
- **新建议 S10** 打印列宽较窄时，条目末尾会单字换行，例如 p2「2. 网关扣款：结账 / 支付」后单独一个「中」。这符合 K5「长条目在卡内换行」，只影响观感。

## 看不出的格子

- **旧静态图（第 4 张）在暗色下的样子**：本轮和 R1 都没有暗色下第 4 张图的截图，无法判断它是否完整、是否跟随主题（C1、K7 在暗色下）。亮色（`U1-default-1440-light-04-a2.png`）、禁用脚本（`U1-nojs-1440-light-04-a3.png`）和打印（p5）都显示正常，旧图下没有卡片区。
- **1440 暗色下第 3 张图（长条目卡片）**：没有截图，只有 R1 复用的 360 暗色图 `U1-narrow-360-dark-03.png` 可看，那张显示正常。
- **360 窄屏滚动前的状态**：本轮没有重拍，单列和换行的判断沿用 R1 复用图。本轮滚动帧里卡片单列、宽度正常，和 R1 一致。
- **字重 650 和 token 是否对得上**：截图只能看出标题加粗、颜色跟随主题，看不出具体的字重数值和 CSS 变量名。

以上几格都不扣分，也不当作通过。

## 总评

U1 本轮 10 分，比上一轮 9.5 高。上一轮扣分的两处布局问题都已消失：窄屏横滚时页面不再左移 1 px（UI-01），打印时右列卡片右边框完整（UI-02）。1440 亮、暗、禁用脚本三种状态重拍后，卡片位置、网格列宽、间距、颜色跟随主题、与 Reader 逐项一致、原文显示都没有退化。不需要任何结构性改动。剩下的只有不扣分的建议，以及旧图暗色和 1440 暗色第 3 张图两处缺截图；要彻底关闭，只需补拍暗色下第 3、4 张图。

## 开图轨迹

- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-target.md
- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-captures/r2/capture.md
- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-captures/r2/observed-interactions.md
- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge.md
- r2 图片（用 Read 查看，或在内存中解码测像素，不写文件）：
  - U1-narrow-360-light-figure-scrolled.png、U1-narrow-360-light-figure-scrolled-settled.png
  - U1-print-dark-a4-p1.png ～ U1-print-dark-a4-p5.png
  - U1-print-dark-a4-p2-right-200dpi.png、U1-print-dark-a4-p3-right-200dpi.png、U1-print-dark-a4-p4-right-200dpi.png
  - U1-default-1440-light-01-a2.png ～ U1-default-1440-light-04-a2.png
  - U1-theme-1440-dark-01-a2.png、U1-theme-1440-dark-02-a2.png
  - U1-nojs-1440-light-top-a3.png、U1-nojs-1440-light-01-a2.png、U1-nojs-1440-light-02-a3.png、U1-nojs-1440-light-03-a3.png、U1-nojs-1440-light-04-a3.png
- r1 图片：
  - U1-narrow-360-light-figure.png
  - U1-narrow-360-light-figure-scrolled-a3.png、U1-narrow-360-light-figure-scrolled-settled-a3.png（只做像素测量）
  - U1-narrow-360-light-01.png、U1-narrow-360-light-02.png、U1-narrow-360-dark-03.png
  - U1-readercards-sc-1440-light.png、U1-readercards-wf-1440-light.png、U1-readercards-orders-1440-light.png
  - U1-print-dark-a4-p2-a2.png、U1-print-dark-a4-p3-a2.png、U1-print-dark-a4-p4-a2.png、U1-print-dark-a4-p2-a2-right-200dpi.png
- 另外用 `ls` 列过 r1、r2 目录下的 PNG 文件名，没有打开其他文件。
