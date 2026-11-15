# R1 · 采集清单

- 目标包：[ELINTP-VISUALS-P3-gaps.ui-target.md](../../ELINTP-VISUALS-P3-gaps.ui-target.md)；目标指纹 `0a84be6430e5a5eb2271ee726b4288dedcba32a6126cf61941882074a8f82867`（口径与输入见 [target-fingerprint.txt](target-fingerprint.txt)，无目标图附件）。首轮，无历史复用。
- 被测版本：HEAD `c7c77b8d3512a0030b1ea7b4e8125b83498a1a69`；`git diff --binary HEAD` SHA-256 `1b6e8e02d42b2615f4cba573128240e1c6a6377e80262b3465fd36f2ab5ea13d`（未提交改动只在台账、评审报告与 `skills/use-llm-gateway`，`skills/elintp`、`tools/phoenix-diagram` 相对 HEAD 无改动）。受测文件 SHA-256 见 logs/source-sha256.txt。
- 静态图：开发引擎 `local/phoenix-diagram/elintp-p3-gaps/phoenix-diagram-darwin-arm64`（二进制 SHA-256 `9781718b9d9d…`；构建清单的 191 个源输入与当前工作树逐文件一致）对三份 candidate 运行 `static`，三份均退出 0；旧图为 P3 已交付的 `workflow.system.svg` 副本。同三份 candidate 用 `deliver` 取 Reader HTML。
- 页面：`node skills/elintp/scripts/render.mjs logs/inputs/draft.md logs/pages/final.html`（退出 0）；审阅页 `node skills/elintp/scripts/review.mjs logs/inputs/draft.md logs/pages/review.html logs/pages/review.result.json --no-open`，本机 `http://127.0.0.1:55563/`，结束时 SIGTERM 停止。未提交标注。
- 工具与条件：Playwright 1.63.0 驱动系统 Chrome 157.0.8103.78（headless），DPR 1，`reducedMotion: reduce`，等待 networkidle 与 `document.fonts.ready`，截图关闭动画。终稿在离线上下文以 `file://` 打开，非 `file:`/`data:` 请求一律拦截并记录。打印：页面主题按钮切到暗色后 `page.pdf`（A4，打印背景），用 pdftoppm 26.10.0 以 80 dpi 转图。
- 内容稿四个小节各一张图：① 状态机，system 字体，3 张规则判定卡片；② workflow，embedded 字体，2 张作者卡片，条目含 `<b>` 与 `&`；③ workflow，system 字体，2 张长条目卡片；④ P3 旧静态图，不带卡片数据。
- 主 agent 已逐张打开下列图片，核对画面是目标页面、视口与主题相符、非报错页或空白页。

## 新图出处

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 拍摄时间 UTC | 实际地址 |
| --- | --- | --- | --- | --- |
| [U1-default-1440-light-01.png](U1-default-1440-light-01.png) | U1 · 默认，第 1 张图（规则判定卡片） | 1440×900 / 1 / light | 2026-11-13T15:27:00.070Z | `file://…/r1/logs/pages/final.html`，离线 |
| [U1-default-1440-light-02.png](U1-default-1440-light-02.png) | U1 · 默认，第 2 张图（作者卡片含 `<b>`、`&`） | 1440×900 / 1 / light | 15:27:00.242Z | 同上 |
| [U1-default-1440-light-03.png](U1-default-1440-light-03.png) | U1 · 默认，第 3 张图（长条目卡片） | 1440×900 / 1 / light | 15:27:00.421Z | 同上 |
| [U1-default-1440-light-04.png](U1-default-1440-light-04.png) | U1 · 默认，第 4 张图（旧静态图，已滚到页底） | 1440×900 / 1 / light | 15:27:00.605Z | 同上 |
| [U1-theme-1440-dark-01.png](U1-theme-1440-dark-01.png) | U1 · 点主题按钮切到暗色后，第 1 张图 | 1440×900 / 1 / dark | 15:27:01.055Z | 同上 |
| [U1-theme-1440-dark-02.png](U1-theme-1440-dark-02.png) | U1 · 暗色，第 2 张图 | 1440×900 / 1 / dark | 15:27:01.227Z | 同上 |
| [U1-narrow-360-light-figure.png](U1-narrow-360-light-figure.png) | U1 · 窄屏，第 1 张图上部 | 360×800 / 1 / light | 15:27:02.413Z | 同上 |
| [U1-narrow-360-light-01.png](U1-narrow-360-light-01.png) | U1 · 窄屏，第 1 张图卡片区 | 360×800 / 1 / light | 15:27:02.064Z | 同上 |
| [U1-narrow-360-light-02.png](U1-narrow-360-light-02.png) | U1 · 窄屏，第 2 张图卡片区 | 360×800 / 1 / light | 15:27:02.244Z | 同上 |
| [U1-narrow-360-dark-03.png](U1-narrow-360-dark-03.png) | U1 · 窄屏，系统偏好暗色，第 3 张图卡片区 | 360×800 / 1 / dark | 15:27:03.248Z | 同上 |
| [U1-nojs-1440-light-01.png](U1-nojs-1440-light-01.png) | U1 · 禁用脚本，第 1 张图 | 1440×900 / 1 / light | 15:27:04.114Z | 同上，`javaScriptEnabled=false` |
| [U1-print-dark-a4-p1.png](U1-print-dark-a4-p1.png)～[p4](U1-print-dark-a4-p4.png) | U1 · 显式暗色下打印，PDF 第 1–4 页（第 5 页为旧图小节，未复制） | A4 / 80 dpi 转图 | PDF 生成 15:27:05Z | 同上；原 PDF `logs/final-print.pdf` |
| [U1-readercards-sc-1440-light.png](U1-readercards-sc-1440-light.png) | 对照：状态机 candidate 的 Reader 图下卡片（非被测单元，`.cards` 元素截图） | 1440×900 / 1 / light | 15:27:07.054Z | `file://…/r1/logs/svg/sc.reader.html` |
| [U1-readercards-wf-1440-light.png](U1-readercards-wf-1440-light.png) | 对照：作者卡片 candidate 的 Reader 卡片 | 1440×900 / 1 / light | 15:27:08.131Z | `file://…/r1/logs/svg/wf.reader.html` |
| [U1-readercards-orders-1440-light.png](U1-readercards-orders-1440-light.png) | 对照：长条目 candidate 的 Reader 卡片 | 1440×900 / 1 / light | 15:27:09.004Z | `file://…/r1/logs/svg/orders.reader.html` |
| [U2-default-1440-light.png](U2-default-1440-light.png) | U2 · 默认收起，第 2 张图 | 1440×900 / 1 / light | 15:27:09.997Z | `http://127.0.0.1:55563/` |
| [U2-enabled-1440-light.png](U2-enabled-1440-light.png) | U2 · 开启标注，第 2 张图 | 1440×900 / 1 / light | 15:27:10.221Z | 同上 |
| [U2-annotated-1440-light.png](U2-annotated-1440-light.png) | U2 · 为第 2 张图添加标注后 | 1440×900 / 1 / light | 15:27:10.721Z | 同上 |
| [U2-annotated-360-dark.png](U2-annotated-360-dark.png) | U2 · 为第 2 张图添加标注后（系统偏好暗色） | 360×800 / 1 / dark | 15:27:12.005Z | 同上 |

实际执行的操作与看到的结果见 [observed-interactions.md](observed-interactions.md)。

## 验收条件覆盖

| 目标条件 | 单元 | 证据 | 类型 |
| --- | --- | --- | --- |
| C1 模板、亮暗、单文件离线 | U1、U2 | U1 default 01–04、theme dark 01–02、narrow dark 03；U2 全部；操作 1、2 | 本轮实拍 + 实测 |
| C2 窄屏与宽图 | U1、U2 | U1 narrow 四张；U2 annotated 360；操作 3、4 | 本轮实拍 + 实测 |
| C3 不只靠颜色；打印与无脚本 | U1 | U1 default 01–03、nojs 01、print p1–p4；操作 5、6 | 本轮实拍 + 实测 |
| K1 位置与结构、不进大纲 | U1、U2 | U1 default 01–03（左侧大纲可见）；U2 default | 本轮实拍 |
| K2 与 Reader 卡片一致、原文显示 | U1 | U1 default 01–03 对照三张 readercards；default 02、theme dark 02 | 本轮实拍 |
| K3 不显示 dot 颜色 | U1 | U1 default 01–03 对照 readercards | 本轮实拍 |
| K4 外观与网格 | U1 | U1 default 01–03、theme dark 01–02 | 本轮实拍 |
| K5 窄屏单列与换行 | U1、U2 | U1 narrow 01、02、dark 03；U2 annotated 360 | 本轮实拍 |
| K6 显式暗色下打印 | U1 | U1 print p1–p4；操作 6 | 本轮实拍 + 实测 |
| K7 旧图无卡片区 | U1 | U1 default 04（另见 03 底部） | 本轮实拍 |
| K8 审阅页一致与不遮挡 | U2 | U2 default、enabled、annotated 1440、annotated 360；操作 7–9 | 本轮实拍 + 实测 |

缺证：无。U2 窄屏只拍添加标注后一帧，默认与开启标注以桌面代表；U1 无脚本以第 1 张图代表。

## 环境噪声

- U2-enabled-1440-light 左侧大纲「旧静态图」一项呈强调色，是点击工具条按钮后鼠标停留位置造成的悬停态，不属于被测状态。

## 补采 a2

评审报告看不出两项：窄屏宽图的自身横向滚动没有操作证据；打印内容贴纸边，无法判断是否来自采集参数。主 agent 已打开下列每张图核对。

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 时间 UTC | 实际地址 / 工具 |
| --- | --- | --- | --- | --- |
| [U1-narrow-360-light-figure-scrolled.png](U1-narrow-360-light-figure-scrolled.png) | U1 · 窄屏，第 1 张图区域内横向滚到最右 | 360×800 / 1 / light | 2026-11-13T15:33:43Z | 终稿 `file://`，离线；同上工具 |
| [U1-print-dark-a4-p1-a2.png](U1-print-dark-a4-p1-a2.png)～[p5-a2](U1-print-dark-a4-p5-a2.png) | U1 · 显式暗色下打印，四边 10 mm 页边距，PDF 第 1–5 页 | A4 / 80 dpi 转图 | PDF 生成 15:33:44Z | 同上；原 PDF `logs/final-print-a2.pdf` |
| [U1-print-dark-a4-p2-a2-right-200dpi.png](U1-print-dark-a4-p2-a2-right-200dpi.png) | U1 · 同一 PDF 第 2 页右侧卡片区，200 dpi 局部转图 | A4 局部 | 同上 | 同上 |

- 原 p1–p4（无页边距）保留，不覆盖。操作见 observed-interactions.md 第 10、11 条。

## 补采 a3

核对 a2 第 10 条记录，并补 U2 窄屏宽图自身滚动的操作证据（首轮评审报告 U2-annotated-360 看不出该项）。主 agent 已打开下列每张图核对。

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 时间 UTC | 实际地址 |
| --- | --- | --- | --- | --- |
| [U1-narrow-360-light-figure-scrolled-a3.png](U1-narrow-360-light-figure-scrolled-a3.png) | U1 · 窄屏，第 1 张图区域内横向滚轮 12 次，滚轮刚停 | 360×800 / 1 / light | 2026-11-13T15:41:13.606Z | 终稿 `file://`，离线 |
| [U1-narrow-360-light-figure-scrolled-settled-a3.png](U1-narrow-360-light-figure-scrolled-settled-a3.png) | U1 · 同上，滚轮停后 1.5 s | 360×800 / 1 / light | 15:41:15.128Z | 同上 |
| [U2-annotated-scrolled-360-dark-a3.png](U2-annotated-scrolled-360-dark-a3.png) | U2 · 添加标注后，第 2 张图区域内横向滚轮 12 次，滚轮刚停 | 360×800 / 1 / dark | 15:41:16.971Z | `http://127.0.0.1:50039/` |
| [U2-annotated-scrolled-settled-360-dark-a3.png](U2-annotated-scrolled-settled-360-dark-a3.png) | U2 · 同上，滚轮停后 1.5 s | 360×800 / 1 / dark | 15:41:18.506Z | 同上 |

- 工具与条件同首采。操作见 observed-interactions.md 第 12、13 条；第 10 条措辞已按截图事实更正。

## 补采 a4

补 U2 窄屏下 K5（长条目换行）与 K8（图注不被遮挡）的视觉证据；首采覆盖表中这两项原只以 U2-annotated-360 代表，不足。主 agent 已打开两张图核对。

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 时间 UTC | 实际地址 |
| --- | --- | --- | --- | --- |
| [U2-annotated-caption-360-dark-a4.png](U2-annotated-caption-360-dark-a4.png) | U2 · 添加标注后，第 2 张图卡片区与图注 | 360×800 / 1 / dark | 2026-11-13T15:44:23.782Z | `http://127.0.0.1:52845/` |
| [U2-enabled-longcards-360-dark-a4.png](U2-enabled-longcards-360-dark-a4.png) | U2 · 开启标注，第 3 张图长条目卡片区与图注 | 360×800 / 1 / dark | 15:44:23.949Z | 同上 |

- 操作见 observed-interactions.md 第 14 条。
