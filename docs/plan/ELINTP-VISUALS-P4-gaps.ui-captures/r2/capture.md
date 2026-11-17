# R2 · 采集清单

- 目标包：[ELINTP-VISUALS-P4-gaps.ui-target.md](../../ELINTP-VISUALS-P4-gaps.ui-target.md)；目标指纹 `cbb77004f7a8f00836ad9783257cc271053e942afb75a0c425adb2f16a123fba`，与 R1 相同——按 R1 保存的口径（[r1/target-fingerprint.txt](../r1/target-fingerprint.txt)）复算，当前目标包「单元／规范要点」文本与 R1 已保存的规范化文本逐字节一致，期望未变，不重建基线。
- 被测版本：仓根 HEAD `84f91b5e1e24a9a13b9ac5e370142fbb8e6bc0ac`；`git diff --binary HEAD` SHA-256 前 16 位 `4400ef59aee2895c`（未提交改动仅为 docs 台账与评审文档，`tools/phoenix-diagram`、`dist/phoenix-diagram` 相对 HEAD 无差异，`git status` 核对）。引擎从当前 HEAD 代码构建：`mise exec --no-deps -- mbx build --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --bin phoenix-diagram`（dev 调试版；构建缓存命中即完成，Cargo 新鲜度检查通过）。
- 输入：与 R1 相同的四份 candidate（[bar](../r1/logs/inputs/bar.json) / [line](../r1/logs/inputs/line.json) / [limits](../r1/logs/inputs/limits.json) / [pie](../r1/logs/inputs/pie.json)），SHA-256 复核与 R1 一致：bar `08661f06…`、line `389db7d6…`、limits `f2b944bc…`、pie `cfc6302d…`。
- 引擎产物：对每份 candidate 重跑 `finalize chart <in> <out>.html --out-dir … --json` 与 `static chart <in> <out>.embedded.svg --font-policy embedded --out-dir … --json`，8/8 exit 0，summary 在 [logs/products/](logs/products/)（`<n>.finalize-summary.json`、`<n>.embedded.static-summary.json`，均 status=pass、diagnostics 空）。产物哈希随 summary；本轮四份 html/svg 与 R1 产物不同（模板随目录装配更新，见报告机器检查）。
- 工具与条件（与 R1 相同）：
  - 引擎自动截图：finalize/static 内建浏览器检查，1440 宽整页（高 1044），light/dark 各一张；phoenix-diagram 0.2.0。
  - 交互与数据表截图：Playwright（tools/phoenix-ui/node_modules）驱动系统 Chrome，headless，1440×900、DPR 1、`reducedMotion: reduce`，`?theme=light|dark`，等待 networkidle 与 `document.fonts.ready`；展开 `details.chart-data` 后截视口；选中表首行复制后在浏览器内读剪贴板。脚本与结果：[logs/interact/interact.cjs](logs/interact/interact.cjs)、[logs/interact/interact-results.json](logs/interact/interact-results.json)。
  - 无账号、无业务数据写入；固定输入，无随机内容。
- 主 agent 已逐张打开下列本轮全部 24 张图片，核对画面为目标页面、视口与主题相符，不是报错页、登录页或空白页。

## 本轮新图出处

拍摄时间均为 2026-11-16（本轮采集），`file://` 本机地址，无网络请求（控制台与失败请求记录为空）。

| 截图 | 单元 · 状态 | 视口 / DPR / 主题 | 所用工具 |
| --- | --- | --- | --- |
| [U1-default-1440-light.png](U1-default-1440-light.png) / [-dark](U1-default-1440-dark.png) | U1 · 阅读器默认 | 1440 宽整页 / 引擎默认 / light、dark | phoenix-diagram finalize 自动截图 |
| [U2-default-1440-light.png](U2-default-1440-light.png) / [-dark](U2-default-1440-dark.png) | U2 · 阅读器默认 | 同上 | 同上 |
| [U3-default-1440-light.png](U3-default-1440-light.png) / [-dark](U3-default-1440-dark.png) | U3 · 阅读器默认 | 同上 | 同上 |
| [U4-default-1440-light.png](U4-default-1440-light.png) / [-dark](U4-default-1440-dark.png) | U4 · 阅读器默认 | 同上 | 同上 |
| [U1-table-1440-light.png](U1-table-1440-light.png) / [-dark](U1-table-1440-dark.png) | U1 · 数据表展开 | 1440×900 / 1 / light、dark | Playwright + 系统 Chrome |
| [U2-table-1440-light.png](U2-table-1440-light.png) / [-dark](U2-table-1440-dark.png) | U2 · 数据表展开 | 同上 | 同上 |
| [U3-table-1440-light.png](U3-table-1440-light.png) / [-dark](U3-table-1440-dark.png) | U3 · 数据表展开 | 同上 | 同上 |
| [U4-table-1440-light.png](U4-table-1440-light.png) / [-dark](U4-table-1440-dark.png) | U4 · 数据表展开 | 同上 | 同上 |
| [U1-static-1440-light.png](U1-static-1440-light.png) / [-dark](U1-static-1440-dark.png) | U1 · embedded 静态图 | 1440 宽整页 / 引擎默认 / light、dark | phoenix-diagram static 自动截图 |
| [U2-static-1440-light.png](U2-static-1440-light.png) / [-dark](U2-static-1440-dark.png) | U2 · embedded 静态图 | 同上 | 同上 |
| [U3-static-1440-light.png](U3-static-1440-light.png) / [-dark](U3-static-1440-dark.png) | U3 · embedded 静态图 | 同上 | 同上 |
| [U4-static-1440-light.png](U4-static-1440-light.png) / [-dark](U4-static-1440-dark.png) | U4 · embedded 静态图 | 同上 | 同上 |

## 验收条件覆盖

| 目标条件 | 单元 | 证据 | 类型 |
| --- | --- | --- | --- |
| S1 图例键 | U1、U2 | U1/U2 default 亮暗（图例色块内键字母、右侧「键 · 名称」）；标注首词同为键 | 本轮实拍 |
| S2 强调色槽 | U3、U4 | U3 default 亮暗（超限填充与「超限」图例醒目、正常填充弱化）；U4 default 亮暗（第 6 扇区与列表色块同色、可辨） | 本轮实拍 |
| S3 数据表纯数字 | U1、U2、U4 | U1/U2/U4 table 亮暗（表头带单位、括号随语言；单元格纯数字）；复制文本（interact-results.json） | 本轮实拍 + 实测 |
| S4 仅上限行 | U3 | U3 default 与 table 亮暗（虚线框、图例「仅上限」） | 本轮实拍 |
| S5 note 行 | U3 | U3 default 与 table 亮暗（「导入」「导出」两行带 note） | 本轮实拍 |
| S6 静态同源 | U1–U4 | U1–U4 static 亮暗（图面与阅读器一致）；`data-phoenix-table` 与阅读器表格逐格比对由主 agent 机械执行（结果只进报告） | 本轮实拍 + 实测 |
| S7 图自明、可展开 | U1–U4 | default 与 table 全部截图 | 本轮实拍 + 实测 |
| S8 图文一致 | U1–U4 | default 亮暗与 table 亮暗（数值、单位、标签与输入一致） | 本轮实拍 + 实测 |

缺证：无。en/es 的「limit only」图例文案不在本轮单元矩阵（目标包采集节已注明，由计划 V-2 目录测试覆盖）。

## 环境噪声

- 无：控制台、失败请求、页面级横向溢出在 8 轮交互中均为空/无；无补采、无废弃试跑。
