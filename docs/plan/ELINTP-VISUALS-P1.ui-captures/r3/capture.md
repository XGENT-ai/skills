# R3 · 采集清单

- 目标包：[冻结目标](../../ELINTP-VISUALS-P1.ui-target.md)；指纹 `b4f0cbfb2b40298df116b80dca5ee8dc41a92ed597f55be1cdc11ab90943dbba`，与 R1/R2 相同。计算输入：[target-fingerprint.txt](target-fingerprint.txt)，无目标图附件。
- 被测 HEAD `c127116152cef65411c9b8f3b855f259bf62c2a1`；tracked dirty SHA-256 `0a85c86b07ba0d268d062f06e5b13be9cc15be1d4d3f63856156c1eed1415c9e`。elintp 受测文件无未提交修改。
- 输入及引用素材与 R2 完全一致；本轮用当前公开 render.mjs 重建 U1、U2、U2-todo、U2-done、U3 到 r3/logs/pages。未使用旧页面作为新采集入口。
- Playwright 1.59.1 / Chrome 157.0.8103.39；1440×900 与 360×800；DPR 1；light/dark；等待 networkidle 与 fonts.ready；截图关闭动画。
- 新图全为本轮真实浏览器原图，主 agent 已逐张打开并核对采集轨迹与画面。10 张新图覆盖三模板大纲跳转/滚动、U3 末项跳转及 U1 无脚本阅读器；相同状态共用图片。
- 实际操作：[observed-interactions.md](observed-interactions.md)。历史操作出处：[R2 实操](../r2/observed-interactions.md)。

## 新图出处

| 图片 | 状态 | 视口 / DPR | 系统主题 | 时间 UTC | 实际入口 / 工具 |
| --- | --- | --- | --- | --- | --- |
| [U1-outline-scroll-1440-light-03.png](U1-outline-scroll-1440-light-03.png) | U1-outline-scroll-1440-light-03 | 1440×900 / 1 | light（未手选） | 2026-11-11T06:50:36.450Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U1.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-outline-scroll-360-dark-03.png](U1-outline-scroll-360-dark-03.png) | U1-outline-scroll-360-dark-03 | 360×800 / 1 | dark（未手选） | 2026-11-11T06:50:48.891Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U1.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-outline-scroll-1440-light-03.png](U2-outline-scroll-1440-light-03.png) | U2-outline-scroll-1440-light-03 | 1440×900 / 1 | light（未手选） | 2026-11-11T06:50:52.855Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U2.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U2-outline-scroll-360-dark-03.png](U2-outline-scroll-360-dark-03.png) | U2-outline-scroll-360-dark-03 | 360×800 / 1 | dark（未手选） | 2026-11-11T06:51:04.460Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U2.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-second-1440-light-02.png](U3-outline-second-1440-light-02.png) | U3-outline-second-1440-light-02 | 1440×900 / 1 | light（未手选） | 2026-11-11T06:51:07.487Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-scroll-1440-light-03.png](U3-outline-scroll-1440-light-03.png) | U3-outline-scroll-1440-light-03 | 1440×900 / 1 | light（未手选） | 2026-11-11T06:51:08.146Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-last-1440-light-04.png](U3-outline-last-1440-light-04.png) | U3-outline-last-1440-light-04 | 1440×900 / 1 | light（未手选） | 2026-11-11T06:51:08.403Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U3.html#s4`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-second-360-dark-02.png](U3-outline-second-360-dark-02.png) | U3-outline-second-360-dark-02 | 360×800 / 1 | dark（未手选） | 2026-11-11T06:51:18.485Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U3-outline-scroll-360-dark-03.png](U3-outline-scroll-360-dark-03.png) | U3-outline-scroll-360-dark-03 | 360×800 / 1 | dark（未手选） | 2026-11-11T06:51:19.117Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U3.html#s2`；Playwright 1.59.1 / Chrome 157.0.8103.39 |
| [U1-nojs-reader-1440-light-01.png](U1-nojs-reader-1440-light-01.png) | U1-nojs-reader-1440-light-01 | 1440×900 / 1 | light（未手选） | 2026-11-11T06:51:23.321Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r3/logs/pages/U1.html`；Playwright 1.59.1 / Chrome 157.0.8103.39 |

## 历史复用出处与覆盖

- R2 的原始地址、时间、工具、矩阵条件及版本完整保留于 [R2 capture.md](../r2/capture.md)。下表仅指明原轮次和状态，不将旧图标记为新图。
- R1 跳到正文焦点继续复用原路径 [U1-skip-focus-360-light-01.png](../r1/U1-skip-focus-360-light-01.png)，出处沿用 R2 采集清单中的历史登记。

| 目标条件 | 单元 | 本轮证据 | 历史复用 | 覆盖 |
| --- | --- | --- | --- | --- |
| C1/C2 结论、组件、状态 | U1–U4 | 大纲相关页面的新图 | R2 四单元 default 桌面亮/窄屏暗及 U2 三进度 firstscreen | 已覆盖 |
| C3 亮暗/保留/拒绝存储 | U1–U4 | U1–U3 四条件真实键盘切换/刷新；新图 light/dark | R2 拒绝存储、U4 亮暗及嵌入独立主题 | 已覆盖 |
| C4 大纲跳转与滚动 | U1–U3 | 新图与操作 1–36；todo/done 操作 37–38 | U4 单小节无大纲，R2 原结论 | 已覆盖 |
| C5 宽内容、窄屏 | U1–U4 | U1–U3 四条件当前页面；新图 | R2 宽表、image/image-scroll、reader-scroll、record-image | 已覆盖 |
| C6 键盘/焦点/释义 | U1–U4 | U1–U3 Tab/Enter 主题、大纲 | R2 notes/theme-focus 与宽表、图片键盘；R1 skip-focus | 已覆盖 |
| C7 无脚本/打印 | U1 共用代表 | U1-nojs-reader 新图、操作 39 | R2 nojs 其余状态及三组各四页打印 | 已覆盖 |
| C8 语言/离线/缺图/转义 | U1–U4 | 当前 U1–U3 操作 | R2 U3/U4/ja、开页与实测 | 已覆盖 |
| U1-1 结构/嵌入阅读器 | U1 | 新图与大纲实操 | R2 default、reader 系列 | 已覆盖 |
| U2-1 三进度 | U2 | partial 大纲；todo/done 操作 37–38 | R2 三进度全页/首屏 | 已覆盖 |
| U3-1 繁体结构 | U3 | 当前大纲及实际图文 | R2 default 桌面亮/窄屏暗 | 已覆盖 |
| U4-1 英文/ja/单节/自绘图 | U4 | 无新增 | R2 default、ja 与实操；评分继承 R2 | 已覆盖 |

## 待评单元与材料边界

- U1、U2、U3 交独立评审；U4 继承 R2 评分，不重新打分。新图的窄屏滚动状态中大纲已滚出视口，当前项事实以实际操作记录为依据。
- 旧截图按需比较；R2 default 图中的初始大纲可作为旧版本静态外观，当前大纲行为只用本轮证据。
- 旧问题 UI-03 由 U3 新图和四条件操作覆盖；UI-05 由 U1 新图覆盖。已关闭问题用原轮次有效证据保持。
- 没有必需条件缺证；未对触屏拖动做推断。
