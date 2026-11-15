# R2 · 评审 B 原文

- 派发：补采 a2 后，全新上下文 general-purpose subagent 重评 U1、U2，只读，模型继承主 agent（claude-opus-5-5），2026-11-14。材料同评审 A，另含 a2 图片及 capture.md「补采 a2」节；未给评审 A 原文、代码、diff、修复记录、机器检查、门槛。
- 评审在 scratchpad 写了一个临时像素解码脚本 `png.py`（不在仓库内），未改动项目文件。
- 以下为评审返回原文，未改动。

---

# R2 UI 验收评审：ELINTP-VISUALS-P3-gaps · U1、U2

两个单元本轮都是 **10 分**。上一轮的三条差距（UI-01、UI-02、UI-03）在新图上都已关闭，没有发现回归，也没有新的有依据差距。唯一的证据缺口是禁用脚本下第 2、3 张图的卡片本轮没有截图，见第 5 节。

## 1. 评分表

| 单元 | 布局 | 视觉 | 交互与状态 | 细节 | 单元分 |
| --- | --- | --- | --- | --- | --- |
| U1 终稿中的静态图卡片 | 3 | 3 | 3 | 1 | **10** |
| U2 审阅页中的静态图卡片 | 3 | 3 | 3 | 1 | **10** |

**U1**
- **布局 3**：已是最高档，没有可扣的差距。
  - 1440 下三列卡片每列 272 px，两列卡片 414 px，卡片之间 12 px，卡片区与图之间 12 px（02-a2 中图底 y=311，卡片顶 y=323）。
  - 360 下单列，卡片在 x=20–340 内。
  - 窄屏在图内横向滚动时，刚停帧和稳定帧里卡片边框都在 x=20/340，和滚动前一致，页面不再移动。
  - 显式暗色打印的 p2–p4 中，最右一张卡片的右边框和圆角都完整。
  - 旧图下没有卡片区。
- **视觉 3**：已是最高档。
  - 卡片用页面表面色和分隔线色，标题加粗，条目用正文色。
  - 亮色（default a2）、暗色（theme a2）、打印（转为亮色）都随页面切换。
  - 不显示 dot 颜色。
- **交互与状态 3**：已是最高档。
  - 主题按钮能切到暗色；禁用脚本时第 1 张图的三张卡片完整显示。
  - 显式暗色打印输出为亮色，5 页里没有卡片被分页切开。
  - 宽图能在自身区域滚到最右（露出「已支付」）。
- **细节 1**：三组卡片和 Reader 对照图逐项相同、顺序相同；`&lt;b&gt;`、`&amp;` 按字面显示；长条目在卡片内换行，没有截断。

**U2**
- **布局 3**：已是最高档。
  - 1440 下卡片在图后、图注前，两列，间距 12 px。
  - 「✎ 编辑标注 · 疑问」、标注原话和「× 删除标注」都在图上方，不遮挡卡片和图注。
  - 360 暗色下在图内横向滚动，刚停帧和稳定帧的卡片边框都在 x=20/340，页面不再左移。
- **视觉 3**：已是最高档。卡片外观和终稿一致，亮暗随主题切换，不显示 dot 颜色。
- **交互与状态 3**：已是最高档。
  - 默认收起时没有标注控件；开启后出现「＋ 添加标注」。
  - 添加后按钮变为「✎ 编辑标注 · 疑问」，工具条显示「1 条标注」。
  - 宽图能滚到最右。
- **细节 1**：卡片标题、条目、原文显示和图注逐处比对，与终稿没有差异。

## 2. 上一轮差距复核表

| 编号 | 单元 | 上一轮问题 | 本轮状态 | 证据 |
| --- | --- | --- | --- | --- |
| UI-01 | U1 | 360 下在图内横向滚轮后，整页左移 1 px | **已解决** | 新采集 `U1-narrow-360-light-figure-scrolled.png` 和 `…-settled.png`：卡片左右边框在 x=20/340（取 y=420、600 两行），和 R1 滚动前的图一致。R1 的 a3 刚停帧是 19/339。本轮稳定帧和 R1 稳定帧逐像素相同（0 差异）。 |
| UI-02 | U1 | 显式暗色打印时，每页最右一张卡片缺右边框 | **已解决** | 新采集 `U1-print-dark-a4-p2/p3/p4.png` 和三张 200 dpi 右侧局部：最右一张卡片的右边框、右上和右下圆角都完整。R1 的 `p2-a2` 局部图在页面右缘被截断。 |
| UI-03 | U2 | 360 审阅页在图内横向滚轮后，刚停时整页左移 1 px | **已解决** | 新采集 `U2-annotated-scrolled-360-dark.png` 刚停帧：图容器左缘、标注按钮、卡片左右边框都在 x=20/340（取 y=150、400、600 三行），和稳定帧一致。R1 的 a3 刚停帧是 19/339。刚停帧和稳定帧的像素差异只在图区域内（x 21–339，y 260–539），卡片、标注控件和标题都没有变化。 |

回归检查：
- 本轮 1440 a2 的 10 张图和 R1 同状态图做了逐像素比较，差异只是图区域和文字里零散的抗锯齿像素（每张约 3–5 千个点）。
- 卡片边框的 x 位置在两轮里逐列相同（x=440/711、724/995 等），没有布局回归。
- 打印 p2–p4 和 R1 a2 相比，只有右边框补齐了，其余排版相同。

## 3. 差距清单

无。没有仍未关闭的有依据差距，可挽回分合计 0，和 10 − 10 = 0 吻合。

## 4. 建议（不扣分）

- **S1（沿用）**：打印时小节眉题落在上一页底部，标题和图在下一页。例如 p1 底部的「为什么成立」、p2 底部的「量级」、p3 底部的「风险与前提」。可以给小节眉题加 `break-after: avoid`，或者把眉题和 h2 包成一组，加 `break-inside: avoid`。
- **S2（沿用）**：网格里的卡片被拉成等高，短卡片下半部空着。例如 01-a2 的「概况」「成立的规则」、打印 p2 第三张卡片。可以给卡片区加 `align-items: start`。
- **S3、S6（沿用）**：卡片条目沿用正文行高和列表间距，卡片偏高；卡片标题看起来略小于条目。规范只要求标题字重 650，所以不扣分。
- **S4（沿用）**：窄屏下宽图被视口切断，没有提示还能横向滚动。
- **S5（沿用，归 phoenix-diagram）**：暗色下静态图底色比页面和卡片底色更深，接缝明显（theme-dark-01/02-a2）。
- **S8（沿用，归 phoenix-diagram）**：360 下把图滚到最右时，可见区域只剩一个空分组框（U2 两张滚动帧）。
- **S9（新观察）**：U1、U2 的「刚停」帧里，图内容比稳定帧多向左偏 1 px，图容器最右 1 px 那一列短暂露出页面底色，1.5 s 后恢复。页面本身不动，文字也没有被裁剪，所以不违反 C2，也不扣分。如果要消除，属于图滚动容器的末端回弹问题，不涉及页面布局。

## 5. 看不出的格子

- **禁用脚本下第 2、3 张图的卡片**：本轮只有 `U1-nojs-1440-light-01-a2.png`，只拍到第 1 张图（3 张卡片完整）。R1 也只列了 `U1-nojs-1440-light-01`。第 2、3 张图在禁用脚本时卡片是否完整，在列出的截图里都看不到，本轮不推断通过或失败。
- **禁用脚本下主题按钮是否隐藏**：nojs-01-a2 没有拍到页顶区域。这一项不属于 K 规范，C3 只要求控件不遮挡内容，而截图里没有看到任何控件遮挡卡片。
- **窄屏滚动前的四张图仍是复用 R1，本轮没有重拍**：本轮滚动稳定帧和 R1 稳定帧逐像素相同，可以间接说明当前代码在窄屏卡片区的渲染和 R1 一样，但这不算新采集的滚动前图。

## 6. 总评

U1 和 U2 并列最低，都是 10 分。三条旧差距（窄屏整页移动 1 px、打印时右列卡片缺右边框）在新采集图上都已消除，没有新的有依据差距，也没有回归。不需要任何结构性改动。剩下的只是不扣分的建议，以及禁用脚本下第 2、3 张图没有截图这一处证据缺口。

## 开图轨迹

- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-target.md
- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-captures/r2/capture.md
- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-captures/r2/observed-interactions.md
- /Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P3-gaps.ui-captures/r1/judge.md
- r2 中的 PNG（其中 4 张滚动帧另经像素脚本测量）：
  - U1-narrow-360-light-figure-scrolled.png、U1-narrow-360-light-figure-scrolled-settled.png
  - U2-annotated-scrolled-360-dark.png、U2-annotated-scrolled-settled-360-dark.png
  - U1-print-dark-a4-p1.png ～ p5.png
  - U1-print-dark-a4-p2-right-200dpi.png、p3-right-200dpi.png、p4-right-200dpi.png
  - U1-default-1440-light-01-a2.png ～ 04-a2.png
  - U1-theme-1440-dark-01-a2.png、02-a2.png
  - U1-nojs-1440-light-01-a2.png
  - U2-default-1440-light-a2.png、U2-enabled-1440-light-a2.png、U2-annotated-1440-light-a2.png
- r1 中 capture.md 列出的 PNG：
  - U1-narrow-360-light-figure.png
  - U1-narrow-360-light-figure-scrolled-a3.png
  - U1-narrow-360-light-figure-scrolled-settled-a3.png（只经像素脚本读取）
  - U2-annotated-scrolled-360-dark-a3.png、U2-annotated-scrolled-settled-360-dark-a3.png
  - U1-print-dark-a4-p2-a2.png、p3-a2.png、p4-a2.png、U1-print-dark-a4-p2-a2-right-200dpi.png
  - U1-readercards-sc-1440-light.png、U1-readercards-wf-1440-light.png、U1-readercards-orders-1440-light.png
  - U1-narrow-360-light-01.png、U1-narrow-360-light-02.png、U1-narrow-360-dark-03.png
  - U2-annotated-360-dark.png、U2-annotated-caption-360-dark-a4.png、U2-enabled-longcards-360-dark-a4.png
- r1 中以下 PNG 只经像素脚本和 r2 a2 图逐像素比较，没有目视打开：
  - U1-default-1440-light-01～04.png
  - U1-theme-1440-dark-01、02.png
  - U1-nojs-1440-light-01.png
  - U2-default-1440-light.png、U2-enabled-1440-light.png、U2-annotated-1440-light.png
- 像素测量用的临时解码脚本：/private/tmp/claude-501/-Users-rockie-Documents-gh-xgent-skills/9786f846-8dac-480c-b7ff-2e5081bb984a/scratchpad/png.py

另外，对 r2 目录执行过一次 `ls`，因此看到了 `judge-a.raw.md` 和 `logs/` 的名字，但没有打开它们。
