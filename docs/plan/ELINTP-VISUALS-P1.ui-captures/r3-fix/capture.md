# R3 直接修复 · 取证记录

- 修前基线 `c127116152cef65411c9b8f3b855f259bf62c2a1`；修后版本见 [M4 修复记录](../../ELINTP-VISUALS-P1.records/M4.md#r3--直接修复)。目标指纹仍为 `b4f0cbfb2b40298df116b80dca5ee8dc41a92ed597f55be1cdc11ab90943dbba`，未改目标。
- Playwright 1.59.1 / Chrome 157.0.8103.39；DPR 1；1440×900/light 与 360×800/dark。原样复用 R1 输入，经当前公开 render.mjs 重建 U1、U2、U2-done、U3 至本目录 logs/pages；真实 file 页面，无请求拦截或临时 UI 修改。
- 修前与修后实操：[before.json](logs/before.json)、[after.json](logs/after.json)；执行入口 [regression.cjs](logs/regression.cjs)。直接锚点：[supplement.json](logs/supplement.json)。本轮是修复方复验，未重新评分或覆盖历史报告。
- 主 agent 已逐张打开下列代表图并核对真实采集轨迹。拍摄时间采用浏览器同步写入 PNG 的时间，并结合操作轨迹核验，不以文件时间单独证明真实性。

| 图 | 状态 / 视口 / DPR / 系统主题 | 拍摄时间 UTC | 实际入口 |
| --- | --- | --- | --- |
| [U3-outline-last-1440-light-fixed.png](U3-outline-last-1440-light-fixed.png) | U3 末项跳转；1440×900 / 1 / light | 2026-11-11T07:18:18.267435Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3-fix/logs/pages/U3.html#s4` |
| [U3-outline-scroll-1440-light-fixed.png](U3-outline-scroll-1440-light-fixed.png) | U3 滚回同位置；1440×900 / 1 / light | 2026-11-11T07:18:18.699638Z | 同上 |
| [U3-outline-scroll-360-dark-fixed.png](U3-outline-scroll-360-dark-fixed.png) | U3 窄屏滚回；360×800 / 1 / dark | 2026-11-11T07:18:19.885496Z | 同上 |
| [U1-reader-nojs-1440-light-fixed.png](U1-reader-nojs-1440-light-fixed.png) | U1 禁用脚本；1440×900 / 1 / light | 2026-11-11T07:18:20.692632Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3-fix/logs/pages/U1.html` |
| [U1-reader-enabled-1440-light-fixed-a4.png](U1-reader-enabled-1440-light-fixed-a4.png) | U1 启用脚本并切换阅读器至暗色；1440×900 / 1 / light | 2026-11-11T07:23:10.365665Z | 同上 |

- UI-03：修前 U3、U2-done 在两视口的相同位置因路径而标记不同；修后均一致。U1/U2 长页邻近路径未回归；四页×两视口直接打开 #s2，上滚返回后当前项一致，页面无横向溢出。
- UI-05：无脚本 hostThemeVisible=false、readerThemeVisible=false；启用脚本两者可见，阅读器主题实际 light→dark。源图不修改。
- 正常切主题的首张 enabled 图处于过渡动画；已保留。a4 为等待过渡完成、animations=disabled 的有效补充帧；a2/a3 为本轮诊断留图，不作为新增状态。
- 打印：实际 Chrome PDF 修前/修后各 4 页，逐页 pdftoppm 栅格完全相同，见 [print-comparison.json](logs/print-comparison.json)。最初固定像素高度/媒体切换后的框高断言不适合真实打印流程，改为实际打印输出前后对照；产品打印代码未修改。
- 未覆盖或改写旧图；既有 .gitignore 覆盖本目录 PNG 和 logs。没有业务账号或数据写入，自有 Chrome 实例均关闭。
