# R1-apply · 采集清单

- 目标包：[ELINTP-VISUALS-P4-gaps.ui-target.md](../../ELINTP-VISUALS-P4-gaps.ui-target.md)，目标指纹 `cbb77004f7a8`（与 R1 相同，本轮未改目标包）。
- 来源报告：[R1 UI 验收报告](../../ELINTP-VISUALS-P4-gaps.ui-review.md)（判定：需要修改；无停滞预警、无待裁定）。
- 被测版本：基线 HEAD `8f3e66650c6ce97f673e8695f201e8e0b810b284` + 本轮工作树改动（`tools/phoenix-diagram` 差异 SHA-256 `a8335984e37676c7aaa47784db21120c019103719dbaebeb30a9af9b32ee7498`；内容与本轮 `UI修复` 提交相同）。引擎按计划 §9.1 从当前代码构建（dev 调试版 `tools/phoenix-diagram/target/debug/phoenix-diagram`，`mise exec --no-deps -- mbx build … --bin phoenix-diagram`），装配模板 `assets/template.html` 已按 `assemble-phoenix-diagram-reader.mjs` 重新生成（964902 字节）。
- 输入：沿用 R1 的四份 candidate（未复制，直接读 [r1/logs/inputs/](../r1/logs/inputs/)）：bar `08661f06…`、line `389db7d6…`、limits `f2b944bc…`、pie `cfc6302d…`。数据、语言、单位、note 行与 R1 完全一致。
- 引擎产物：每份 candidate 跑 `finalize chart` 与 `static chart --font-policy embedded`，8/8 exit 0、`automatedStatus: pass`，summary 在 [logs/products/](logs/products/)。
- 交互与实测：Playwright（`tools/phoenix-ui/node_modules`）驱动系统 Chrome。测量脚本 [logs/verify.cjs](logs/verify.cjs)、[logs/factcheck.cjs](logs/factcheck.cjs)、[logs/zoom.cjs](logs/zoom.cjs)，原始读数 [logs/verify.json](logs/verify.json)、[logs/factcheck.json](logs/factcheck.json)。条件同 R1：1440×900、DPR 1（对齐矩阵）或 DPR 3（放大核对）、`?theme=light|dark`、`reducedMotion: reduce`、等 `document.fonts.ready`；无账号、无业务数据写入、固定输入、无网络请求。

## 本轮新图出处

拍摄时间 2026-11-16 19:10–19:25 +08:00，`file://` 本机地址。

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 所用工具 |
| --- | --- | --- | --- |
| [U1-default-1440-light](U1-default-1440-light.png) / [-dark](U1-default-1440-dark.png) | U1 · 阅读器默认 | 1440 宽整页 / 引擎默认 / light、dark | phoenix-diagram finalize 自动截图 |
| [U2-default-1440-light](U2-default-1440-light.png) / [-dark](U2-default-1440-dark.png) | U2 · 阅读器默认 | 同上 | 同上 |
| [U3-default-1440-light](U3-default-1440-light.png) / [-dark](U3-default-1440-dark.png) | U3 · 阅读器默认 | 同上 | 同上 |
| [U4-default-1440-light](U4-default-1440-light.png) / [-dark](U4-default-1440-dark.png) | U4 · 阅读器默认 | 同上 | 同上 |
| [U1-table-1440-light](U1-table-1440-light.png) / [-dark](U1-table-1440-dark.png) | U1 · 数据表展开 | 1440×900 / 1 / light、dark | Playwright + 系统 Chrome |
| [U2-table-1440-light](U2-table-1440-light.png) / [-dark](U2-table-1440-dark.png) | U2 · 数据表展开 | 同上 | 同上 |
| [U3-table-1440-light](U3-table-1440-light.png) / [-dark](U3-table-1440-dark.png) | U3 · 数据表展开 | 同上 | 同上 |
| [U4-table-1440-light](U4-table-1440-light.png) / [-dark](U4-table-1440-dark.png) | U4 · 数据表展开 | 同上 | 同上 |
| [U1-static-1440-light](U1-static-1440-light.png) / [-dark](U1-static-1440-dark.png) | U1 · embedded 静态图 | 1440 宽整页 / 引擎默认 / light、dark | phoenix-diagram static 自动截图 |
| [U2-static-1440-light](U2-static-1440-light.png) / [-dark](U2-static-1440-dark.png) | U2 · embedded 静态图 | 同上 | 同上 |
| [U3-static-1440-light](U3-static-1440-light.png) / [-dark](U3-static-1440-dark.png) | U3 · embedded 静态图 | 同上 | 同上 |
| [U4-static-1440-light](U4-static-1440-light.png) / [-dark](U4-static-1440-dark.png) | U4 · embedded 静态图 | 同上 | 同上 |
| [U4-ring-list-light-zoom](U4-ring-list-light-zoom.png) / [-dark](U4-ring-list-dark-zoom.png) | U4 · 阅读器默认（环形图与列表放大） | 1440×900 / 2 / light、dark | Playwright 元素截图 |
| [U4-node-index-light-zoom](U4-node-index-light-zoom.png) / [-dark](U4-node-index-dark-zoom.png) | U4 · 阅读器默认（节点索引「Fixtures」条目） | 同上 | Playwright 元素截图 |
| [U2-lowload-light-zoom](U2-lowload-light-zoom.png) / [-dark](U2-lowload-dark-zoom.png) | U2 · 阅读器默认（「低负载」近零标注放大） | 同上 | Playwright 裁剪截图 |

R1 的「改前」对照图仍在 [r1/](../r1/) 原位置，本轮未覆盖、未改评审材料与 `judge.md`。

## 实测读数（供复审直接引用）

- UI-01：阅读器「Fixtures」扇区与列表色块计算样式 `rgb(101, 163, 13)`（亮）/ `rgb(163, 230, 53)`（暗）；同图系列绿 `rgb(5, 150, 105)` / `rgb(52, 211, 153)`；节点索引 chip 边框同值，`--outline-tone: var(--accent-stroke)`；节点 `data-node-kind` 仍为 `external`。静态图取值与阅读器相同（`logs/verify.json` 的 `U4`）。
- UI-03：「低负载」两标注基线 y=263、276，圆点在 y=284，零轴线 y=284：均在标记上方 8px 以上、互距 13、且在轴线上方 8px（`logs/verify.json` 的 `U2`；静态 SVG 逐值相同）。
- UI-04：「导出」行 `rect.ch-span` x=192、宽 336 → 右端 528；同行上限刻度 `line.ch-limit` x=528（亮暗一致）；528 之后是三行共有的 `rect.ch-track`（x=192、宽 420）。
- UI-02：图例文字及各级祖先 `text-decoration-line: none`；把 `.diagram-container` 的 `--canvas-dot` 置透明后同一处横线消失，见 `logs/bar-legend-*-grid-on.png` 与 `-grid-off.png`、`logs/line-legend-*`。

## 环境噪声

- 8 轮交互控制台无 error/warning、无 pageerror、无失败请求；`documentElement.scrollWidth = clientWidth = 1440`。
- 无其他噪声。
