# ELINTP-VISUALS-P3-gaps · UI 验收评审 R2

## 评审对象

| 轮次 | 开发计划 | 目标包 | 被测版本 | 截图 | 上轮报告 |
| --- | --- | --- | --- | --- | --- |
| R2（复审，全量轮；目标未变，不重建基线） | [ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md) | [ELINTP-VISUALS-P3-gaps.ui-target.md](ELINTP-VISUALS-P3-gaps.ui-target.md) · `0a84be6430e5` | `3889a9ae`（含 UI 修复 `b0e182ff`），dirty `1cea6a865927`（未提交改动不涉及 `skills/elintp`、`tools/phoenix-diagram`）；终稿 `file://…/r2/logs/pages/final.html`，审阅页 `http://127.0.0.1:60556/`、`:49400/` 等本机临时端口 | [r2](ELINTP-VISUALS-P3-gaps.ui-captures/r2/capture.md) | [R1](ELINTP-VISUALS-P3-gaps.ui-review.md) |

## 验收判定

**通过验收**

本轮分数 10（U1、U2 并列），门槛 8（沿用 R1 默认值）。没有 P0/P1 门槛项，没有未关闭差距，也没有停滞预警或待裁定事项。

R1 的三条打磨项都由独立评审判为已解决：
- 窄屏横向滚宽图时，终稿（UI-01）和审阅页（UI-03）的页面都不再瞬时左移 1 px。
- 显式暗色打印时，最右一张卡片的右边框完整（UI-02）。

1440 下亮色、暗色、禁用脚本和审阅页各状态都在当前代码上重拍过，没有发现回归。

本判定只表示界面在所测版本和条件下的验收结论，不代表合并、发布或产品终验。

## 评分

| 单元 | 布局 | 视觉 | 交互与状态 | 细节 | 单元分 | 各评审单元分 | 达标 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| U1 终稿中的静态图卡片 | 3 | 3 | 3 | 1 | **10** | A 10 → B 10（补采 a2）→ C 10（补采 a3；取 C 整组） | 是 |
| U2 审阅页中的静态图卡片 | 3 | 3 | 3 | 1 | **10** | A 10 → B 10（补采 a2；取 B 整组） | 是 |

- 本轮分数：10。全量轮：两个单元都有本轮独立评分，所有条件都有有效证据。
- 历史最佳：10（R1 为 9.5）。
- 门槛：8/10，用户未指定，沿用 R1。
- 四维理由见 [judge.md](ELINTP-VISUALS-P3-gaps.ui-captures/r2/judge.md)。

## 轮次轨迹

| 轮次 | 本轮分数（最低单元） | 历史最佳 | 未关闭 P0/P1 | 重构轮 | 备注 |
| --- | --- | --- | --- | --- | --- |
| R1 | 9.5（U1） | 9.5 | 0 | — | 首轮；新建目标包；补采 3 次，重评 3 次 |
| R2 | 10（U1、U2） | 10 | 0 | — | 复审 UI 修复 `b0e182ff`；补采 2 次，重评 2 次；UI-01–UI-03 全部关闭 |

## 上轮差距复核

| 编号 | 上轮严重度与摘要 | 上轮处置（取自修复记录） | 本轮结果 | 依据 |
| --- | --- | --- | --- | --- |
| UI-01 | P2 · 窄屏滚过宽图末端后整页瞬时左移 1 px（U1） | 已修改待评审：`.image-scroll` 加 `overscroll-behavior-x: contain`（[M2 UI 验收修复](ELINTP-VISUALS-P3-gaps.records/M2.md#ui-验收修复)） | 已解决 | 评审 A、B、C 都判已解决。[刚停](ELINTP-VISUALS-P3-gaps.ui-captures/r2/U1-narrow-360-light-figure-scrolled.png)、[1.5 s 后](ELINTP-VISUALS-P3-gaps.ui-captures/r2/U1-narrow-360-light-figure-scrolled-settled.png)两帧中，卡片左缘都在 x=20 |
| UI-02 | P2 · 打印时最右一张卡片缺右边框（U1） | 已修改待评审：打印时 `.figure-cards` 加 1 px 左右内边距 | 已解决 | 评审 A、B、C 都判已解决。[p2 局部](ELINTP-VISUALS-P3-gaps.ui-captures/r2/U1-print-dark-a4-p2-right-200dpi.png)、[p3](ELINTP-VISUALS-P3-gaps.ui-captures/r2/U1-print-dark-a4-p3-right-200dpi.png)、[p4](ELINTP-VISUALS-P3-gaps.ui-captures/r2/U1-print-dark-a4-p4-right-200dpi.png) 中，右边框和圆角都完整 |
| UI-03 | P3 · 审阅页窄屏滚过宽图末端后整页瞬时左移 1 px（U2） | 已修改待评审：与 UI-01 共用同一条规则 | 已解决 | 评审 A、B 都判已解决。[刚停](ELINTP-VISUALS-P3-gaps.ui-captures/r2/U2-annotated-scrolled-360-dark.png)、[1.5 s 后](ELINTP-VISUALS-P3-gaps.ui-captures/r2/U2-annotated-scrolled-settled-360-dark.png)两帧中，卡片与标注控件的左缘都在 x=20 |

## 差距清单

无未关闭差距。

## 待裁定事项

无。

## 建议（不扣分）

全文见 [judge.md · 建议](ELINTP-VISUALS-P3-gaps.ui-captures/r2/judge.md#建议不扣分)。R1 的 S1–S6、S8 仍可见；本轮新增三条：

- **S9 滚轮刚停那一帧**：图自身的滚动区域里，内容多偏 1 px，最右 1 px 露出页面底色，1.5 s 后复位。页面和卡片都不动，符合 C2。
- **S10 打印时图区底色不印**：图例下方的留白在纸上像图和卡片之间的大空当。属于静态图的 viewBox 或打印底色问题。
- **S11 打印列宽较窄**：条目末尾会单字换行。

观感最明显的仍是 S1（打印时小节眉题与标题分页）和 S2（卡片等高拉伸）。

## 机器检查

| 检查项 | 工具与版本 | 结果 | 差距 |
| --- | --- | --- | --- |
| 被测代码生效 | 读生成的 `final.html` | 含 `.image-scroll { overscroll-behavior-x: contain; }`，打印规则含 `.figure-cards { padding-inline: 1px; }`；`page.css` SHA-256 `c35c6da44441…` 与修复记录一致 | — |
| 控制台错误、页面异常、失败请求 | Playwright 1.63.0 / Chrome 157.0.8103.78 | 终稿全部上下文 0 条；离线拦截的非 `file:`/`data:` 请求 0 条。审阅页每次启动有 1 条 `favicon.ico` 404，来自 review CLI 本机服务，不影响可见行为。修复记录里那条未定位 URL 的 404 很可能就是它（未在修复方环境复现核实） | — |
| 窄屏宽图滚动时页面不动（UI-01、UI-03） | 同上；对真实截图逐行读像素（`logs/check-pixels.cjs`、`logs/checks.json`） | 终稿 360 light、审阅页 360 dark（添加标注后）各做 3 次：滚轮 200 px ×12，读滚动前、300 ms、1.5 s 三个时刻。卡片左缘全部为 x=20（R1 刚停时为 19）。图滚动距离分别为 500 和 477；页面 `scrollX` 为 0，`scrollWidth` 等于 360 | 已解决 |
| 打印右边框（UI-02） | pdftoppm 26.10.0，200 dpi 局部读像素 | p2–p4 最右卡片在 x=375–376 有 `--line` 颜色 `rgb(184,195,193)` 的边框，外侧为白色 | 已解决 |
| 打印时卡片的样式与分页（K6） | 同上，打印媒体下读计算样式 | 显式暗色下打印为 `color-scheme: light`；卡片底色 `rgb(241,243,242)`，边框 1px，`break-inside: avoid`；PDF 共 5 页，与 R1 a2 相同 | — |
| 打印规则不影响屏幕 | 同上，屏幕媒体下读计算样式 | 1440 与 360 下 `.figure-cards` 的 `padding-inline` 为 0，`gap` 和 `margin-top` 为 12；列宽 1440 下为 272/414，360 下为 320，与 R1 相同；页面无水平溢出 | — |
| 1440 屏幕回归 | ImageMagick 7.1.2-25，比较 a2 与 R1 同状态图（`logs/diff-a2-vs-r1.txt`） | 10 张图严格比较各有 0.25–0.42% 的像素不同；容差 10% 下其中 9 张为 0。`U2-enabled` 剩 132 px，位于 51×12 的区域，就是 R1 记录的大纲悬停噪声 | — |
| 可访问性扫描、键盘走查 | — | 未检查。本次修复没有新增交互控件；对比度沿用 R1 计算结果，卡片颜色变量未变 | — |

## 采集与覆盖

- **环境与版本**：
  - HEAD `3889a9ae16396d710f637583721b12a73d3eb897`；`git diff --binary HEAD` SHA-256 `1cea6a86…659f8dd1`。
  - `skills/elintp`、`tools/phoenix-diagram` 相对 HEAD 无改动。受测源码 SHA-256 与修复记录一致，见 capture.md。
  - 输入复用 R1 的内容稿和静态 SVG。终稿与审阅页由当前 `render.mjs`、`review.mjs` 重新生成。
  - 目标指纹沿用 R1 口径重新计算，结果相同，不重建基线。
- **采集**：
  - 首采 12 张：U1 窄屏滚动 2 张、显式暗色打印 5 页、200 dpi 右侧局部 3 张；U2 窄屏滚动 2 张。
  - 补采 a2，10 张：U1 1440 默认 4 张、暗色 2 张、禁用脚本 1 张；U2 1440 默认收起、开启标注、添加标注后各 1 张。
  - 补采 a3，4 张：U1 禁用脚本页顶及第 2–4 张图。
  - 合计 26 张本轮实拍，主 agent 逐张打开核对过。
  - 修复方留在 `r1-apply/` 的自检图没有直接交给评审；本轮在提交后的版本上重新拍摄，条件相同。
- **历史复用**：沿用 R1 的有以下几类，详见 capture.md「历史复用」节。
  - U1：窄屏滚动前 4 张、Reader 对照 3 张；
  - U2：360 添加标注后 1 张，以及 a4 的 2 张。
  - 本次两条规则只影响滚动容器的越界回弹和打印媒体。屏幕计算样式与 R1 相同，这些静态状态不受影响，可以沿用。
- **补采原因**：
  - a2：评审 A 指出 1440 各状态只有修复前的复用图。
  - a3：评审 B 指出禁用脚本下只拍了第 1 张图，看不到第 2、3 张图的卡片和页顶。
- **仍未单独拍摄的格子**：评审 C 提到 1440 暗色下第 3、4 张图，以及 360 滚动前状态没有本轮重拍。
  - 它们对应的条件已有有效证据：C1 暗色由 theme dark 01–02 a2 和 R1 360 dark 03 覆盖；K7 由 default 04 a2、nojs 04 a3 和打印 p5 覆盖。
  - 本次改动不影响这些屏幕状态，因此不算缺证，也没有再补采。
  - 评审 C 还提到截图看不出字重 650 和变量名。这一项由计算样式检查覆盖，R1 已测过，本轮的样式未变。
- **环境噪声**：
  - 审阅页 `favicon.ico` 404，见上表。
  - a2 拍摄前把鼠标移到视口右下角，避开了 R1 记录的大纲悬停态。
- **评审方式**：
  - 共 3 位全新上下文、只读的 subagent，模型都继承主 agent。A 评 U1、U2；补采 a2 后 B 重评 U1、U2；补采 a3 后 C 重评 U1。
  - 每个单元取总分较低那位的整组分数；各位同分时，取材料最全的一位。差距取并集。
  - 没有确认评审，也不是单上下文评审。
  - 评审材料只有目标包、截图、capture.md、操作记录和 R1 judge.md。没有给代码、diff、修复记录、机器检查结果或门槛。
  - 原文：[A](ELINTP-VISUALS-P3-gaps.ui-captures/r2/judge-a.raw.md)、[B](ELINTP-VISUALS-P3-gaps.ui-captures/r2/judge-b.raw.md)、[C](ELINTP-VISUALS-P3-gaps.ui-captures/r2/judge-c.raw.md)。
  - 评审 B 在会话 scratchpad 写过一个临时解码脚本，不在仓库内。
- **盲测走查**：检测到走查子代理，但两个单元都只是阅读和标注展示，没有主要操作流程，所以没有派，与 R1 相同。
- **主 agent 核对说明**：
  - 抽查了三位评审描述的现象，与截图一致。
  - 评审 C 的建议 S7 写了「被测提交说明里提到了」。评审材料里没有提交说明，这句话的来源应是 R1 judge.md 中原 S7 的修法建议，不影响分数和差距。

## 下一步

- 判定为通过验收，没有剩余差距。是否继续打磨 S1、S2 等建议由你决定；要打磨，执行 `$apply-ui-review docs/plan/ELINTP-VISUALS-P3-gaps.ui-review.r2.md`，然后用 `$review-ui docs/plan/ELINTP-VISUALS-P3-gaps.md` 复审。
- 本循环的 UI 修复已提交为代码 `b0e182ff`。不打磨时，先用 `$review-code docs/plan/ELINTP-VISUALS-P3-gaps.md` 复审这次改动。
- 之后可选：`$review-prd-dev-gaps docs/plan/ELINTP-VISUALS-P3-gaps.md`。
