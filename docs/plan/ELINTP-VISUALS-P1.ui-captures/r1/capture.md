# R1 · 采集清单

- 目标包：[ELINTP-VISUALS-P1.ui-target.md](../../ELINTP-VISUALS-P1.ui-target.md)。目标指纹 `b4f0cbfb2b40298df116b80dca5ee8dc41a92ed597f55be1cdc11ab90943dbba`。
- 指纹算法：目标包“单元”和“规范要点”LF 文本，加 `attachments: []`；无目标图附件。精确输入见 target-fingerprint.txt。采集输入不是目标图，不参与期望附件 hash。
- 被测 HEAD `fd55e6485f2407ba8eec464a32fca601dfc0a879`；仓库 tracked dirty 指纹 `4b87f296dea8d24547d4b28989fefe4a7561ec79670c4a78d8c7ee1bd42fd080`。skills/elintp 工作树干净。七个页面全部由此版本的公开 render.mjs 当场生成，不使用旧构建。
- elintp 完整源码清单 SHA-256 `5025d6980e746214d3f62eab0f073d4eaad5321d86106c237ec1870e45c0e5f2`（键排序 JSON hash）；输入与输出版本固定。
- 工具：Playwright 1.59.1，系统 Chrome 157.0.8103.39。桌面 1440×900、窄屏 360×800，DPR 1；完整页面或关键状态的可视区域原图。PDF 是产品浏览器打印输出，PNG 由 Poppler 26.10.0 直接转换。
- 本轮全部为新采集；无历史证据/评分复用。没有请求拦截或界面注入；只有存储拒绝状态通过浏览器 init script 模拟存储不可用，不改 UI。
- 第一轮四模板×两视口×两主题先做运行时检查；代表视觉证据用桌面亮色、窄屏暗色。新增图分别补足进度、语言回退、焦点、释义、嵌入阅读器、无脚本、打印，未按问题复制同状态图片。
- 主 agent 已逐张开图并对照本轮采集轨迹，均为对应当前产品页面。U2 长图内有作为内容输入的历史截图；历史截图仅作为页面内图片，不是本轮页面截屏来源。

## 图片出处

每张浏览器截图均由 Playwright screenshot 在实际 file URL 生成，等待 networkidle、document.fonts.ready，截图禁用动画；打印图为浏览器打印 PDF 的直接页面渲染。完整页原图的高度可大于视口，视口保持下表条件。主题中的 system 表示无手动选择，跟随系统；其余为点击后的实际选择。

| 图片 | 单元与状态 | 视口 / DPR | 系统主题与手选 | 拍摄时间 UTC 或明确时区 | 实际地址 / 来源 |
| --- | --- | --- | --- | --- | --- |
| [U1-default-1440-light.png](U1-default-1440-light.png) | U1 · default | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:10.941Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-theme-focus-1440-dark-01.png](U1-theme-focus-1440-dark-01.png) | U1 · theme | 1440×900 / 1 | light；实际主题 dark | 2026-11-11T04:55:11.586Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-notes-expanded-1440-dark-02.png](U1-notes-expanded-1440-dark-02.png) | U1 · notes | 1440×900 / 1 | light；实际主题 dark | 2026-11-11T04:55:12.255Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html#s2` |
| [U1-default-360-dark.png](U1-default-360-dark.png) | U1 · default | 360×800 / 1 | dark；实际主题 system | 2026-11-11T04:55:16.644Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U2-default-1440-light.png](U2-default-1440-light.png) | U2 · default | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:18.682Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U2.html` |
| [U2-default-360-dark.png](U2-default-360-dark.png) | U2 · default | 360×800 / 1 | dark；实际主题 system | 2026-11-11T04:55:24.162Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U2.html` |
| [U3-default-1440-light.png](U3-default-1440-light.png) | U3 · default | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:26.016Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U3.html` |
| [U3-default-360-dark.png](U3-default-360-dark.png) | U3 · default | 360×800 / 1 | dark；实际主题 system | 2026-11-11T04:55:31.619Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U3.html` |
| [U4-default-1440-light.png](U4-default-1440-light.png) | U4 · default | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:33.499Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U4.html` |
| [U4-default-360-dark.png](U4-default-360-dark.png) | U4 · default | 360×800 / 1 | dark；实际主题 system | 2026-11-11T04:55:37.568Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U4.html` |
| [U1-reader-1440-light-03.png](U1-reader-1440-light-03.png) | U1 · reader | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:39.576Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-360-light-03.png](U1-reader-360-light-03.png) | U1 · reader | 360×800 / 1 | light；实际主题 system | 2026-11-11T04:55:40.865Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U2-todo-default-1440-light.png](U2-todo-default-1440-light.png) | U2 · todo | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:41.571Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U2-todo.html` |
| [U2-done-default-1440-light.png](U2-done-default-1440-light.png) | U2 · done | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:42.251Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U2-done.html` |
| [U4-ja-default-1440-light.png](U4-ja-default-1440-light.png) | U4 · ja | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:42.924Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U4-ja.html` |
| [U1-nojs-1440-light.png](U1-nojs-1440-light.png) | U1 · nojs | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:55:44.095Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-interacted-1440-light-04.png](U1-reader-interacted-1440-light-04.png) | U1 · reader | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:58:42.912Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-bottom-1440-light-05.png](U1-reader-bottom-1440-light-05.png) | U1 · reader | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:58:43.217Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-interacted-360-light-04.png](U1-reader-interacted-360-light-04.png) | U1 · reader | 360×800 / 1 | light；实际主题 system | 2026-11-11T04:58:44.949Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-bottom-360-light-05.png](U1-reader-bottom-360-light-05.png) | U1 · reader | 360×800 / 1 | light；实际主题 system | 2026-11-11T04:58:45.223Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U2-firstscreen-1440-light-01.png](U2-firstscreen-1440-light-01.png) | U2 · firstscreen | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:58:45.420Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U2.html` |
| [U2-todo-firstscreen-1440-light-01.png](U2-todo-firstscreen-1440-light-01.png) | U2 · todo | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:58:45.616Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U2-todo.html` |
| [U2-done-firstscreen-1440-light-01.png](U2-done-firstscreen-1440-light-01.png) | U2 · done | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:58:45.789Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U2-done.html` |
| [U4-ja-firstscreen-1440-light-01.png](U4-ja-firstscreen-1440-light-01.png) | U4 · ja | 1440×900 / 1 | light；实际主题 system | 2026-11-11T04:58:45.976Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U4-ja.html` |
| [U1-skip-focus-360-light-01.png](U1-skip-focus-360-light-01.png) | U1 · skip | 360×800 / 1 | light；实际主题 system | 2026-11-11T04:59:12.899Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-scroll-360-light-06.png](U1-reader-scroll-360-light-06.png) | U1 · reader | 360×800 / 1 | light；实际主题 system | 2026-11-11T05:00:06.119Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-print-1.png](U1-print-1.png) | U1 · 打印第 1/4 页 | A4 PDF；栅格 1061×1500 | 系统 dark；print | 2026-11-11T12:55:45+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U1-print-2.png](U1-print-2.png) | U1 · 打印第 2/4 页 | A4 PDF；栅格 1061×1500 | 系统 dark；print | 2026-11-11T12:55:45+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U1-print-3.png](U1-print-3.png) | U1 · 打印第 3/4 页 | A4 PDF；栅格 1061×1500 | 系统 dark；print | 2026-11-11T12:55:45+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U1-print-4.png](U1-print-4.png) | U1 · 打印第 4/4 页 | A4 PDF；栅格 1061×1500 | 系统 dark；print | 2026-11-11T12:55:45+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U1-print-light-a2-1.png](U1-print-light-a2-1.png) | U1 · 打印第 1/4 页 | A4 PDF；栅格 1061×1500 | 系统 light；print | 2026-11-11T12:58:47+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U1-print-light-a2-2.png](U1-print-light-a2-2.png) | U1 · 打印第 2/4 页 | A4 PDF；栅格 1061×1500 | 系统 light；print | 2026-11-11T12:58:47+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U1-print-light-a2-3.png](U1-print-light-a2-3.png) | U1 · 打印第 3/4 页 | A4 PDF；栅格 1061×1500 | 系统 light；print | 2026-11-11T12:58:47+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U1-print-light-a2-4.png](U1-print-light-a2-4.png) | U1 · 打印第 4/4 页 | A4 PDF；栅格 1061×1500 | 系统 light；print | 2026-11-11T12:58:47+08:00（PDF CreationDate） | 同 U1 file URL；浏览器打印后 pdftoppm -scale-to 1500 -png |
| [U3-outline-second-1440-light-02.png](U3-outline-second-1440-light-02.png) | U3 · 大纲第二项激活后 | 1440×900 / 1 | light；实际主题 system | 2026-11-11T05:03:13.348Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U3.html#s2` |
| [U1-image-360-dark-a2.png](U1-image-360-dark-a2.png) | U1 · 评审请求补采 | 360×800 / 1 | dark；实际主题 system | 2026-11-11T05:05:38.415Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-nojs-reader-1440-light-a2.png](U1-nojs-reader-1440-light-a2.png) | U1 · 评审请求补采 | 1440×900 / 1 | light；实际主题 system | 2026-11-11T05:05:38.733Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-toolbar-scroll-360-light-a2.png](U1-reader-toolbar-scroll-360-light-a2.png) | U1 · 评审请求补采 | 360×800 / 1 | light；实际主题 system | 2026-11-11T05:05:39.373Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-toolbar-keyboard-360-light-a3.png](U1-reader-toolbar-keyboard-360-light-a3.png) | U1 · 评审请求补采 | 360×800 / 1 | light；实际主题 system | 2026-11-11T05:05:39.546Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |
| [U1-reader-toolbar-zoom-focus-360-light-a4.png](U1-reader-toolbar-zoom-focus-360-light-a4.png) | U1 · 阅读器键盘放大焦点 | 360×800 / 1 | light；实际主题 system | 2026-11-11T05:06:29.469Z | `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r1/logs/pages/U1.html` |

## 条件到证据映射

| 条件 | 覆盖单元 | 证据类型与材料 | 标记 |
| --- | --- | --- | --- |
| C1 结论、固定外观与层级 | U1–U4 | 新实拍：各 default；U2 firstscreen；U4-ja firstscreen | 已覆盖 |
| C2 组件、状态符号 | U1–U4 | 新实拍：各 default，U1 notes-expanded | 已覆盖；截图与图集中 U1，U4 覆盖自绘图 |
| C3 亮暗、保留、存储拒绝 | U1–U4 | 新实拍 default 两代表；新实操 observed-interactions.md 的四条件操作；U1 theme-focus | 已覆盖 |
| C4 大纲有无、跳转、当前项 | U1–U4 | 新实拍 default、U2 todo/done；新实操 observed-interactions.md | 已覆盖 |
| C5 桌面/窄屏、宽表/图滚动 | U1–U4 | 新实测四条件；新实拍各 default；U1 reader-scroll 与实际横滚 | 已覆盖 |
| C6 键盘与焦点 | U1–U4 | 新实操四条件，U1 连续 Tab；新实拍 skip-focus、theme-focus、notes-expanded | 已覆盖 |
| C7 无脚本/打印 | 共用组件以 U1 代表 | 新实拍 nojs；新打印两次各四页 PNG；observed-interactions.md | 已覆盖；未对其他模板重复打印 |
| C8 语言、离线、转义与缺图 | U1–U4 | 新实拍 U3、U4、U4-ja；新实操各页与注入文字、缺图 | 已覆盖 |
| U1-1 PRD 顺序/嵌图交互 | U1 | default、reader-*；observed-interactions.md | 已覆盖 |
| U2-1 三种进度 | U2 | partial/todo/done 首屏与完整页；observed-interactions.md | 已覆盖 |
| U3-1 四问题/繁体控件 | U3 | desktop-light、mobile-dark；observed-interactions.md | 已覆盖 |
| U4-1 单小节/语言回退/自绘图 | U4 | default、ja；observed-interactions.md | 已覆盖 |

## 操作记录与缺证

独立评审只读 [observed-interactions.md](observed-interactions.md)，无需打开其他 JSON、输入稿、运行检查或页面源码。没有必需格子缺证；手机指针拖动未造成平移的事实保留，另有横向滚轮实际到达右侧图的证据。

## 补采轨迹

- 首采 16 张浏览器图。为看清长页首屏、进度与语言，补四个首屏；为区分阅读器内部滚动与截断，补四张交互/底部图和一张横滚图；补一张跳转焦点图，共 26 张浏览器图，另补 U3 第二项大纲激活后的真实状态一张，合计 27 张浏览器图。
- 两次打印共 8 张页图，总计 35 个唯一图像文件。打印第二次只改变系统偏好，保留第一次全部材料。所有补采使用新文件名，没有覆盖原图。
- 补采阅读器的第一次脚本选到了隐藏 SVG，未取得可拖动区域，采集工具退出；改用页面可见图容器后完成采集。该工具错误不列产品差距；无图片被该失败尝试覆盖。

- 补采 U3-outline-second-1440-light-02.png：补足桌面 U3 点击第二项后锚点与当前标记的实际状态，操作摘要已纠正为轨迹观察事实。

- 独立评审请求补采：窄屏真实图片区域、无脚本阅读器顶部、工具条横滚和键盘状态四图；现有 31 张浏览器图 + 8 张打印页图 = 39 个唯一图像文件。此前总数是补采前计数，不覆盖原图。

- 最后补采 U1-reader-toolbar-zoom-focus-360-light-a4.png：新开上下文，从阅读器主题按钮继续真实 Tab 到放大并 Enter；比例变为 125%。最终 32 张浏览器图 + 8 张打印页图 = 40 个唯一图像文件。
