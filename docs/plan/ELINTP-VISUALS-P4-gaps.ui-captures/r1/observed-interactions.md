# R1 · 实际执行的操作与观察结果

被测产物：本轮 `logs/products/`（引擎 finalize/static 从当前 HEAD 代码生成）；浏览器：Playwright 驱动系统 Chrome（headless）、1440×900、DPR 1、`reducedMotion: reduce`。脚本：[logs/interact/capture.cjs](logs/interact/capture.cjs)、[logs/interact/copy-check.cjs](logs/interact/copy-check.cjs)；原始输出 [logs/interact/interact-results.json](logs/interact/interact-results.json)、[logs/interact/copy-results.json](logs/interact/copy-results.json)。

第一次试跑（capture.cjs）的复制核验有缺陷：headless Chrome 的 `execCommand('copy')` 不写 macOS 系统剪贴板，`pbpaste` 读到的是系统剪贴板里既有的无关内容；该次截图（`U<n>-table-1440-<主题>.png`）有效并沿用，复制文本以第二次专用核验（copy-check.cjs，`navigator.clipboard.readText()` 在浏览器内读取）为准。系统剪贴板未被写入或清除。

每个单元 × 亮暗主题各执行一轮，8 轮动作完全一致：

1. 打开 `file://…/logs/products/<name>.html?theme=<主题>`，等待 networkidle 与 `document.fonts.ready`。
2. 点 `details.chart-data` 的 summary 展开「Data table / 数据表」→ 表格出现，滚动到表格顶部，截图（`U<n>-table-1440-<主题>.png`）。
3. 点击表首行 → 用 Range 选中该行内容 → `document.execCommand('copy')` → 返回 true → `navigator.clipboard.readText()` 读回剪贴板。

## 观察结果（8 轮一致，剪贴板文本如下）

| 单元 · 主题 | 展开后表头（thea​d） | 首行单元格 | 复制返回 | 剪贴板文本（⇥ = Tab） |
| --- | --- | --- | --- | --- |
| U1 bar · light/dark | Item / Before (ms) / After (ms) | p50, 4200, 2600 | true | `p50⇥4200⇥2600` |
| U2 line · light/dark | 项目 / p50（ms） / p95（ms） | 低负载, 0.1, 0.3 | true | `低负载⇥0.1⇥0.3` |
| U3 limits · light/dark | 项目 / 当前值 / 上限 / 单位 / 状态 / 备注 | 检索, 13, 20, req/s, 未超限,（空） | true | `检索⇥13⇥20⇥req/s⇥未超限⇥` |
| U4 pie · light/dark | Item / Value (files) / Share (%) | Source, 120, 33.3 | true | `Source⇥120⇥33.3` |

- 8 轮 `document.documentElement` 无页面级横向溢出（scrollWidth=clientWidth=1440）。
- 8 轮控制台无 error/warning、无 pageerror、无失败请求（interact-results.json `consoleMsgs`、`failedReq` 均为空）。
- 截图时点在行选中之前，画面为「数据表展开、未选中」状态。
