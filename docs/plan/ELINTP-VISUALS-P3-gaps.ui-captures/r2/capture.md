# R2 · 采集清单

- 目标包：[ELINTP-VISUALS-P3-gaps.ui-target.md](../../ELINTP-VISUALS-P3-gaps.ui-target.md)；目标指纹 `0a84be6430e5a5eb2271ee726b4288dedcba32a6126cf61941882074a8f82867`（沿用 R1 口径重新计算，相同；口径见 [R1 target-fingerprint.txt](../r1/target-fingerprint.txt)，无目标图附件）。
- 被测版本：HEAD `3889a9ae16396d710f637583721b12a73d3eb897`；`git diff --binary HEAD` SHA-256 `1cea6a865927e8ebfc11d9fe34df371be67404635601612127d895aa659f8dd1`（`skills/elintp`、`tools/phoenix-diagram` 相对 HEAD 无未提交改动）。受测文件 SHA-256：`skills/elintp/assets/page.css` `c35c6da44441…`，`render.mjs` `543e4e89b032…`，`review.mjs` `16673a333989…`，`draft.mjs` `de29f49f7746…`。
- 输入：复用 R1 内容稿 `../r1/logs/inputs/draft.md` 及其引用的三份静态 SVG 与旧 SVG（四个小节各一张图，同 R1）。
- 页面：本轮以当前代码重新生成。终稿 `node skills/elintp/scripts/render.mjs <R1 draft.md> logs/pages/final.html`（退出 0）；审阅页 `node skills/elintp/scripts/review.mjs <R1 draft.md> logs/pages/review.html logs/pages/review.result.json --no-open`，本机 `http://127.0.0.1:60556/`，结束时 SIGTERM 停止，未提交标注。
- 工具与条件：Playwright 1.63.0 驱动系统 Chrome 157.0.8103.78（headless），DPR 1，`reducedMotion: reduce`，等待 networkidle 与 `document.fonts.ready`，截图关闭动画。终稿在离线上下文以 `file://` 打开，非 `file:`/`data:` 请求一律拦截并记录。打印：页面主题按钮切到暗色后 `page.pdf`（A4，打印背景，四边 10 mm，与 R1 补采 a2 条件相同），pdftoppm 26.10.0 以 80 dpi 转整页、200 dpi 转右侧局部（x=1200 起，宽 454，全页高）。
- 主 agent 已逐张打开下列本轮图片，核对画面是目标页面、视口与主题相符，不是报错页或空白页。

## 本轮新图出处

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 拍摄时间 UTC | 实际地址 |
| --- | --- | --- | --- | --- |
| [U1-narrow-360-light-figure-scrolled.png](U1-narrow-360-light-figure-scrolled.png) | U1 · 窄屏，第 1 张图区域内横向滚轮 12 次，滚轮刚停 | 360×800 / 1 / light | 2026-11-13T16:20:35.302Z | `file://…/r2/logs/pages/final.html`，离线 |
| [U1-narrow-360-light-figure-scrolled-settled.png](U1-narrow-360-light-figure-scrolled-settled.png) | U1 · 同上，滚轮停后 1.5 s | 360×800 / 1 / light | 16:20:36.894Z | 同上 |
| [U2-annotated-scrolled-360-dark.png](U2-annotated-scrolled-360-dark.png) | U2 · 添加标注后，第 2 张图区域内横向滚轮 12 次，滚轮刚停 | 360×800 / 1 / dark | 16:20:44.722Z | `http://127.0.0.1:60556/` |
| [U2-annotated-scrolled-settled-360-dark.png](U2-annotated-scrolled-settled-360-dark.png) | U2 · 同上，滚轮停后 1.5 s | 360×800 / 1 / dark | 16:20:46.251Z | 同上 |
| [U1-print-dark-a4-p1.png](U1-print-dark-a4-p1.png)～[p5](U1-print-dark-a4-p5.png) | U1 · 显式暗色下打印，四边 10 mm，PDF 第 1–5 页 | A4 / 80 dpi 转图 | PDF 生成约 16:20:47Z | 终稿 `file://`，离线；原 PDF `logs/final-print.pdf` |
| [U1-print-dark-a4-p2-right-200dpi.png](U1-print-dark-a4-p2-right-200dpi.png)、[p3](U1-print-dark-a4-p3-right-200dpi.png)、[p4](U1-print-dark-a4-p4-right-200dpi.png) | U1 · 同一 PDF 第 2–4 页右侧局部（含最右一张卡片） | A4 局部 / 200 dpi | 同上 | 同上 |

实际执行的操作与看到的结果见 [observed-interactions.md](observed-interactions.md)。

## 历史复用

以下 R1 截图为同目标、同输入、同采集条件下的历史材料，本轮未重拍，标「复用 R1」：

| 截图（R1） | 单元 · 状态 |
| --- | --- |
| [U1-default-1440-light-01](../r1/U1-default-1440-light-01.png)～[04](../r1/U1-default-1440-light-04.png) | U1 · 默认四张图 |
| [U1-theme-1440-dark-01](../r1/U1-theme-1440-dark-01.png)、[02](../r1/U1-theme-1440-dark-02.png) | U1 · 主题切换到暗色 |
| [U1-narrow-360-light-figure](../r1/U1-narrow-360-light-figure.png)、[narrow-360-light-01](../r1/U1-narrow-360-light-01.png)、[02](../r1/U1-narrow-360-light-02.png)、[narrow-360-dark-03](../r1/U1-narrow-360-dark-03.png) | U1 · 窄屏（滚动前） |
| [U1-nojs-1440-light-01](../r1/U1-nojs-1440-light-01.png) | U1 · 禁用脚本 |
| [U1-readercards-sc](../r1/U1-readercards-sc-1440-light.png)、[wf](../r1/U1-readercards-wf-1440-light.png)、[orders](../r1/U1-readercards-orders-1440-light.png) | 对照：Reader 卡片 |
| [U2-default-1440-light](../r1/U2-default-1440-light.png)、[U2-enabled-1440-light](../r1/U2-enabled-1440-light.png)、[U2-annotated-1440-light](../r1/U2-annotated-1440-light.png)、[U2-annotated-360-dark](../r1/U2-annotated-360-dark.png) | U2 · 默认收起、开启标注、添加标注后 |
| [U2-annotated-caption-360-dark-a4](../r1/U2-annotated-caption-360-dark-a4.png)、[U2-enabled-longcards-360-dark-a4](../r1/U2-enabled-longcards-360-dark-a4.png) | U2 · 窄屏图注与长条目卡片 |

R1 同状态旧图可作对照：滚动 [U1 a3 刚停](../r1/U1-narrow-360-light-figure-scrolled-a3.png) / [稳定](../r1/U1-narrow-360-light-figure-scrolled-settled-a3.png)，[U2 a3 刚停](../r1/U2-annotated-scrolled-360-dark-a3.png) / [稳定](../r1/U2-annotated-scrolled-settled-360-dark-a3.png)；打印 [p2-a2](../r1/U1-print-dark-a4-p2-a2.png)、[p3-a2](../r1/U1-print-dark-a4-p3-a2.png)、[p4-a2](../r1/U1-print-dark-a4-p4-a2.png)、[p2 局部](../r1/U1-print-dark-a4-p2-a2-right-200dpi.png)。

## 验收条件覆盖

| 目标条件 | 单元 | 证据 | 类型 |
| --- | --- | --- | --- |
| C1 模板、亮暗、单文件离线 | U1、U2 | U1 default 01–04、theme dark 01–02、narrow dark 03；U2 全部 | 复用 R1 |
| C2 窄屏与宽图（含宽图滚动时页面不动） | U1、U2 | U1、U2 本轮四张滚动帧；操作 1、2；U1 narrow 四张、U2 annotated 360 | 本轮实拍 + 实测；复用 R1 |
| C3 不只靠颜色；打印与无脚本 | U1 | 本轮 print p1–p5 与三张局部；操作 3；nojs 01 | 本轮实拍；复用 R1 |
| K1 位置与结构、不进大纲 | U1、U2 | U1 default 01–03；U2 default | 复用 R1 |
| K2 与 Reader 卡片一致、原文显示 | U1 | U1 default 01–03 对照 readercards；本轮 print p2–p4 | 复用 R1；本轮实拍 |
| K3 不显示 dot 颜色 | U1 | U1 default 01–03 对照 readercards | 复用 R1 |
| K4 外观与网格 | U1 | U1 default 01–03、theme dark 01–02；本轮 print p2–p4 | 复用 R1；本轮实拍 |
| K5 窄屏单列与换行 | U1、U2 | U1 narrow 01、02、dark 03；U2 a4 两张；本轮四张滚动帧 | 复用 R1；本轮实拍 |
| K6 显式暗色下打印 | U1 | 本轮 print p1–p5 与 p2–p4 右侧局部；操作 3 | 本轮实拍 + 实测 |
| K7 旧图无卡片区 | U1 | U1 default 04；本轮 print p5 | 复用 R1；本轮实拍 |
| K8 审阅页一致与不遮挡 | U2 | U2 default、enabled、annotated 1440/360、a4 两张；本轮 U2 两张滚动帧 | 复用 R1；本轮实拍 |

缺证：无。

## 补采 a2

评审报告 1440 屏幕各状态只有修复前的 R1 复用图，本轮被测代码上缺少新的视觉证据。现以当前代码生成的同一页面（`logs/pages/final.html`；审阅页重新启动），按 R1 首采相同步骤重拍。拍摄前把鼠标移到视口右下角，避免大纲悬停态。主 agent 已打开下列每张图核对。

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 时间 UTC | 实际地址 |
| --- | --- | --- | --- | --- |
| [U1-default-1440-light-01-a2](U1-default-1440-light-01-a2.png)～[04-a2](U1-default-1440-light-04-a2.png) | U1 · 默认，第 1–4 张图 | 1440×900 / 1 / light | 2026-11-13T16:27:11.297Z–11.708Z | 终稿 `file://…/r2/logs/pages/final.html`，离线 |
| [U1-theme-1440-dark-01-a2](U1-theme-1440-dark-01-a2.png)、[02-a2](U1-theme-1440-dark-02-a2.png) | U1 · 点主题按钮切到暗色，第 1、2 张图 | 1440×900 / 1 / dark | 16:27:11.879Z、12.010Z | 同上 |
| [U1-nojs-1440-light-01-a2](U1-nojs-1440-light-01-a2.png) | U1 · 禁用脚本，第 1 张图 | 1440×900 / 1 / light | 16:27:12.775Z | 同上，`javaScriptEnabled=false` |
| [U2-default-1440-light-a2](U2-default-1440-light-a2.png) | U2 · 默认收起，第 2 张图 | 1440×900 / 1 / light | 16:27:13.660Z | `http://127.0.0.1:49400/` |
| [U2-enabled-1440-light-a2](U2-enabled-1440-light-a2.png) | U2 · 开启标注，第 2 张图 | 1440×900 / 1 / light | 16:27:13.824Z | 同上 |
| [U2-annotated-1440-light-a2](U2-annotated-1440-light-a2.png) | U2 · 为第 2 张图添加标注后 | 1440×900 / 1 / light | 16:27:14.278Z | 同上 |

- 工具与条件同首采。操作见 observed-interactions.md 第 4–6 条。
- 补采后，上表各状态改用本轮 a2 实拍；「历史复用」节中仍复用 R1 的是：U1 窄屏滚动前四张、Reader 对照三张、U2 360 添加标注后与 a4 两张。

## 补采 a3

评审报告禁用脚本下只有第 1 张图的截图，看不到第 2、3 张图的卡片，也看不到页顶。现以同一页面 `logs/pages/final.html`、禁用脚本的新离线上下文补拍页顶及第 2–4 张图。主 agent 已打开下列每张图核对。

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 时间 UTC | 实际地址 |
| --- | --- | --- | --- | --- |
| [U1-nojs-1440-light-top-a3](U1-nojs-1440-light-top-a3.png) | U1 · 禁用脚本，页顶（未滚动） | 1440×900 / 1 / light | 2026-11-13T16:33:08.850Z | 终稿 `file://…/r2/logs/pages/final.html`，离线，`javaScriptEnabled=false` |
| [U1-nojs-1440-light-02-a3](U1-nojs-1440-light-02-a3.png)、[03-a3](U1-nojs-1440-light-03-a3.png)、[04-a3](U1-nojs-1440-light-04-a3.png) | U1 · 禁用脚本，第 2、3、4 张图 | 1440×900 / 1 / light | 16:33:08.933Z–08.997Z | 同上 |

- 工具与条件同首采。操作见 observed-interactions.md 第 7 条。
