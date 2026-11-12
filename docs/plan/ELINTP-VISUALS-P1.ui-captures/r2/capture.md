# R2 · 采集清单

- 目标包：[冻结目标](../../ELINTP-VISUALS-P1.ui-target.md)；目标指纹 `b4f0cbfb2b40298df116b80dca5ee8dc41a92ed597f55be1cdc11ab90943dbba`，与 R1 相同，无重建基线。
- 算法沿用 R1：单元与规范要点 LF 文本 + `attachments: []`，输入见 [target-fingerprint.txt](target-fingerprint.txt)。无目标图附件。
- 被测 HEAD `b90edbb795dbdf0beb84f718eea4adb37abedd49`；tracked dirty SHA-256 `c2cddc827aa3bc1c8c9a39317e23aae6b516db8f35f428208b645bc85c0b3fb7`；elintp 源码清单 SHA-256 `699ba1a211f109abcec775f45a39b7c44e6a8471815132f9ba103e3c0ceb23ab`。elintp 受测文件无未提交修改。
- 七份输入原样复用 R1/inputs，本轮由当前 render.mjs 重新生成七个页面，输出在 r2/logs/pages；不使用旧构建。输入只是合法内容数据，非评审模拟页面。
- Playwright 1.59.1 / 系统 Chrome 157.0.8103.39；桌面 1440×900、窄屏 360×800；DPR 1；等待 networkidle 和字体；截图关闭动画。无时钟/随机内容。
- 本轮图片全为 R2 新实拍；R1 截图仅按需作为历史比较。交互实际执行见 [observed-interactions.md](observed-interactions.md)。本页不含机器检查、修复记录和判定。
- 主 agent 已逐张打开本轮图片，核对采集轨迹、视口和目标画面。长页图高度超过视口，PDF 页图为浏览器输出的直接转换。

## 浏览器图片出处

| 图片 | 单元 / 状态 | 视口 / DPR | 系统 → 实际主题 | 时间 UTC | 地址 / 工具 |
| --- | --- | --- | --- | --- | --- |
| [U1-default-1440-light.png](U1-default-1440-light.png) | U1 / U1-default | 1440×900 / 1 | light → system | 2026-11-11T06:06:30.619Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-theme-focus-1440-dark-01.png](U1-theme-focus-1440-dark-01.png) | U1 / U1-theme-focus-1440 | 1440×900 / 1 | light → dark | 2026-11-11T06:06:31.255Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-notes-expanded-1440-dark-02.png](U1-notes-expanded-1440-dark-02.png) | U1 / U1-notes-expanded-1440 | 1440×900 / 1 | light → dark | 2026-11-11T06:06:31.935Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-default-360-dark.png](U1-default-360-dark.png) | U1 / U1-default | 360×800 / 1 | dark → system | 2026-11-11T06:06:36.329Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-default-1440-light.png](U2-default-1440-light.png) | U2 / U2-default | 1440×900 / 1 | light → system | 2026-11-11T06:06:38.381Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-default-360-dark.png](U2-default-360-dark.png) | U2 / U2-default | 360×800 / 1 | dark → system | 2026-11-11T06:06:43.844Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-default-1440-light.png](U3-default-1440-light.png) | U3 / U3-default | 1440×900 / 1 | light → system | 2026-11-11T06:06:45.689Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U3.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-default-360-dark.png](U3-default-360-dark.png) | U3 / U3-default | 360×800 / 1 | dark → system | 2026-11-11T06:06:51.064Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U3.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U4-default-1440-light.png](U4-default-1440-light.png) | U4 / U4-default | 1440×900 / 1 | light → system | 2026-11-11T06:06:52.905Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U4.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U4-default-360-dark.png](U4-default-360-dark.png) | U4 / U4-default | 360×800 / 1 | dark → system | 2026-11-11T06:06:56.959Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U4.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-1440-light-03.png](U1-reader-1440-light-03.png) | U1 / U1-reader-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:06:58.958Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-360-light-03.png](U1-reader-360-light-03.png) | U1 / U1-reader-360 | 360×800 / 1 | light → system | 2026-11-11T06:07:00.232Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-todo-default-1440-light.png](U2-todo-default-1440-light.png) | U2 / U2-todo-default | 1440×900 / 1 | light → system | 2026-11-11T06:07:00.924Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2-todo.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-done-default-1440-light.png](U2-done-default-1440-light.png) | U2 / U2-done-default | 1440×900 / 1 | light → system | 2026-11-11T06:07:01.608Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2-done.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U4-ja-default-1440-light.png](U4-ja-default-1440-light.png) | U4 / U4-ja-default | 1440×900 / 1 | light → system | 2026-11-11T06:07:02.289Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U4-ja.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-nojs-1440-light.png](U1-nojs-1440-light.png) | U1 / U1-nojs | 1440×900 / 1 | light → system | 2026-11-11T06:07:03.456Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-interacted-1440-light-04.png](U1-reader-interacted-1440-light-04.png) | U1 / U1-reader-interacted-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:24.297Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-bottom-1440-light-05.png](U1-reader-bottom-1440-light-05.png) | U1 / U1-reader-bottom-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:24.604Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-interacted-360-light-04.png](U1-reader-interacted-360-light-04.png) | U1 / U1-reader-interacted-360 | 360×800 / 1 | light → system | 2026-11-11T06:07:26.413Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-bottom-360-light-05.png](U1-reader-bottom-360-light-05.png) | U1 / U1-reader-bottom-360 | 360×800 / 1 | light → system | 2026-11-11T06:07:26.713Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-firstscreen-1440-light-01.png](U2-firstscreen-1440-light-01.png) | U2 / U2-firstscreen-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:27.406Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-todo-firstscreen-1440-light-01.png](U2-todo-firstscreen-1440-light-01.png) | U2 / U2-todo-firstscreen-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:28.083Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2-todo.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-done-firstscreen-1440-light-01.png](U2-done-firstscreen-1440-light-01.png) | U2 / U2-done-firstscreen-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:28.760Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2-done.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U4-ja-firstscreen-1440-light-01.png](U4-ja-firstscreen-1440-light-01.png) | U4 / U4-ja-firstscreen-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:29.464Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U4-ja.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-second-1440-light-02.png](U3-outline-second-1440-light-02.png) | U3 / U3-outline-second-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:30.408Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-second-360-light-02.png](U3-outline-second-360-light-02.png) | U3 / U3-outline-second-360 | 360×800 / 1 | light → system | 2026-11-11T06:07:31.350Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-image-360-light-01.png](U1-image-360-light-01.png) | U1 / U1-image-360 | 360×800 / 1 | light → system | 2026-11-11T06:07:32.120Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-image-scroll-360-light-02.png](U1-image-scroll-360-light-02.png) | U1 / U1-image-scroll-360 | 360×800 / 1 | light → system | 2026-11-11T06:07:32.589Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-image-360-dark-01.png](U1-image-360-dark-01.png) | U1 / U1-image-360 | 360×800 / 1 | dark → system | 2026-11-11T06:07:33.340Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-image-scroll-360-dark-02.png](U1-image-scroll-360-dark-02.png) | U1 / U1-image-scroll-360 | 360×800 / 1 | dark → system | 2026-11-11T06:07:33.812Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-nojs-reader-1440-light-a2.png](U1-nojs-reader-1440-light-a2.png) | U1 / U1-nojs-reader-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:07:34.815Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-nojs-image-360-light-01.png](U1-nojs-image-360-light-01.png) | U1 / U1-nojs-image-360 | 360×800 / 1 | light → system | 2026-11-11T06:07:35.714Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-scroll-1440-light-03.png](U3-outline-scroll-1440-light-03.png) | U3 / U3-outline-scroll-1440 | 1440×900 / 1 | light → system | 2026-11-11T06:09:06.067Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-scroll-360-light-03.png](U3-outline-scroll-360-light-03.png) | U3 / U3-outline-scroll-360 | 360×800 / 1 | light → system | 2026-11-11T06:09:07.566Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-keyboard-360-light-07.png](U1-reader-keyboard-360-light-07.png) | U1 / U1-reader-keyboard-360 | 360×800 / 1 | light → system | 2026-11-11T06:09:08.636Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-reader-scroll-360-light-08.png](U1-reader-scroll-360-light-08.png) | U1 / U1-reader-scroll-360 | 360×800 / 1 | light → system | 2026-11-11T06:09:09.020Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-record-image-360-light-01.png](U2-record-image-360-light-01.png) | U2 / U2-record-image-360 | 360×800 / 1 | light → system | 2026-11-11T06:09:09.716Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U2.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |

## 打印图片出处

| 图片 | 原 PDF / 来源 | 系统 / 手选 | 生成时间（PDF 元数据） | 工具 / 条件 |
| --- | --- | --- | --- | --- |
| [U1-print-1.png](U1-print-1.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print.pdf`，第 1/4 页；U1.html 同上 file URL | dark / system | Wed Nov 11 14:07:04 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-2.png](U1-print-2.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print.pdf`，第 2/4 页；U1.html 同上 file URL | dark / system | Wed Nov 11 14:07:04 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-3.png](U1-print-3.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print.pdf`，第 3/4 页；U1.html 同上 file URL | dark / system | Wed Nov 11 14:07:04 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-4.png](U1-print-4.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print.pdf`，第 4/4 页；U1.html 同上 file URL | dark / system | Wed Nov 11 14:07:04 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-light-1.png](U1-print-light-1.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-light.pdf`，第 1/4 页；U1.html 同上 file URL | light / system | Wed Nov 11 14:07:37 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-light-2.png](U1-print-light-2.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-light.pdf`，第 2/4 页；U1.html 同上 file URL | light / system | Wed Nov 11 14:07:37 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-light-3.png](U1-print-light-3.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-light.pdf`，第 3/4 页；U1.html 同上 file URL | light / system | Wed Nov 11 14:07:37 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-light-4.png](U1-print-light-4.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-light.pdf`，第 4/4 页；U1.html 同上 file URL | light / system | Wed Nov 11 14:07:37 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-manual-dark-1.png](U1-print-manual-dark-1.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-manual-dark.pdf`，第 1/4 页；U1.html 同上 file URL | dark / dark | Wed Nov 11 14:07:39 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-manual-dark-2.png](U1-print-manual-dark-2.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-manual-dark.pdf`，第 2/4 页；U1.html 同上 file URL | dark / dark | Wed Nov 11 14:07:39 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-manual-dark-3.png](U1-print-manual-dark-3.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-manual-dark.pdf`，第 3/4 页；U1.html 同上 file URL | dark / dark | Wed Nov 11 14:07:39 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |
| [U1-print-manual-dark-4.png](U1-print-manual-dark-4.png) | `docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/U1-print-manual-dark.pdf`，第 4/4 页；U1.html 同上 file URL | dark / dark | Wed Nov 11 14:07:39 2026 CST | Chrome PDF → Poppler 26.10.0 pdftoppm -scale-to 1500；A4，1061×1500 栅格 |

## 条件到证据映射

| 条件 | 单元 | 证据 | 覆盖 |
| --- | --- | --- | --- |
| C1/C2 结论、共用组件与状态 | U1–U4 | 本轮 default、U2 三进度 firstscreen | 已采集 |
| C3 亮暗、保留、拒绝存储 | U1–U4 | default 代表图、U1 theme-focus；操作记录四条件切换/刷新，U1 storage denied | 已采集/实操 |
| C4 大纲与当前位置 | U1–U4 | U3 outline-second/scroll，U2 三进度与各单元操作记录 | 已采集/实操 |
| C5 窄屏与宽内容 | U1–U4 | 本轮 default、U1 image/image-scroll、reader-scroll、U2 record-image；各视口实测与键盘记录 | 已采集/实测/实操 |
| C6 键盘、焦点、释义 | U1–U4 | U1 theme-focus、notes-expanded、image focus、reader-keyboard；四条件实操 | 已采集/实操；跳到正文焦点复用 R1 U1-skip-focus-360-light-01.png |
| C7 无脚本和打印 | U1 共用组件代表 | nojs 全页、nojs-reader、nojs-image；三组打印各四页 | 已采集 |
| C8 三语言/ja、离线、缺图、转义 | U1–U4 | U3、U4、U4-ja，实际语言与开页记录 | 已采集/实操 |
| U1-1 模板与嵌入阅读器 | U1 | default、reader 系列、操作记录 | 已采集/实操 |
| U2-1 三进度顺序 | U2 | partial/todo/done 全页与首屏 | 已采集 |
| U3-1 繁体四段 | U3 | default 与 outline | 已采集/实操 |
| U4-1 单小节、英文/ja、自绘图 | U4 | default、ja、操作记录 | 已采集/实操 |

## 历史复用与材料边界

- R1 跳到正文焦点截图保留原路径与轮次：`../r1/U1-skip-focus-360-light-01.png`。本轮各模板实际 Tab 仍经过该链接；其标记和焦点样式未变，故仅复用这一视觉状态。上一轮评分不继承，四单元均交独立评审。
- R1 judge.md 作为上一轮结论，R1 图片仅按比较需要打开。输入文档中内嵌的历史截图属于阅读页内容，不冒充本轮采集。
- 37 张浏览器原图 + 12 张 PDF 页图，共 49 张唯一新图。无覆盖历史产物。
- 无必需证据缺失；不为无新视觉信息的操作重复拍图。触屏拖动未列入冻结目标，不据鼠标拖动推断触屏结果。
