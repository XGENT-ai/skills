# ELINTP-VISUALS-P1 · UI/UX 验收 R2

| 轮次 | 开发计划 | 目标包 | 被测版本 | 截图 | 上轮报告 |
| --- | --- | --- | --- | --- | --- |
| R2 · 全量复审 · 规范模式 | [ELINTP-VISUALS-P1](ELINTP-VISUALS-P1.md) | [冻结目标](ELINTP-VISUALS-P1.ui-target.md) · b4f0cbfb2b40 | b90edbb7；tracked dirty c2cddc827aa3；elintp 源码干净；当前公开入口生成的本地 file 页面 | [r2](ELINTP-VISUALS-P1.ui-captures/r2/) | [R1](ELINTP-VISUALS-P1.ui-review.md) |

## 验收判定

**通过验收**

本轮 **9/10**，沿用门槛 **8/10**，历史最佳 **9/10**。全部四单元及必要条件已有有效证据；未关闭 P0/P1 为 **0**。UI-01、UI-02、UI-04 已解决；UI-03 部分解决、UI-05 仍存在，均为 P2，可选打磨，不阻止通过。

UI-05 保持「待裁定」，不因通过而视为解决或用户豁免。无停滞预警。此结论仅适用于下列版本及阅读条件，不代表代码复审、合并、发布或整份产品终验。本轮只评审，未修改产品代码或计划进度。

## 评分

| 单元 | 布局 /3 | 视觉 /3 | 交互与状态 /3 | 细节 /1 | 总分 |
| --- | --- | --- | --- | --- | --- |
| U1 PRD 阅读页 | 3 | 3 | 2.5 | 1 | **9.5** |
| U2 开发计划阅读页 | 3 | 3 | 3 | 1 | **10** |
| U3 CRD 阅读页 | 3 | 3 | 2 | 1 | **9** |
| U4 通用技术阅读页 | 3 | 3 | 3 | 1 | **10** |

最低单元 U3：点击第二项已正确，但滚轮返回相同位置又标记第四项。U1 剩余无脚本阅读器主题按钮。每维原理由见 [judge.md](ELINTP-VISUALS-P1.ui-captures/r2/judge.md)，完整原文见 [judge-a.raw.md](ELINTP-VISUALS-P1.ui-captures/r2/judge-a.raw.md)。四单元全部由本轮独立评审重新评分；U1 可挽回 0.5、U3 可挽回 1，与失分一致。评分完成并保存后才读取历史门槛，未向评审透露门槛。

## 轮次轨迹

| 轮次 | 本轮分数（最低单元） | 历史最佳 | 未关闭 P0/P1 | 重构轮 | 备注 |
| --- | --- | --- | --- | --- | --- |
| R1 | 6（U1） | 6 | 3 | — | 首轮；补证复评，产品未修改 |
| R2 | 9（U3） | 9 | 0 | — | UI 修复 873ecb3b 后独立复审；同一目标基线 |

目标指纹未变。读取了 [M4 完整修复记录](ELINTP-VISUALS-P1.records/M4.md#ui-验收修复)：R1 后 UI-01/02/03/04 各有一次「已修改待评审」，UI-05 为「待决定」，无重构。UI-03 本轮首次判部分解决，尚未达到连续两次修改后部分解决等顽固差距条件；只有两个有效评分轮，也不触发全局平台期。UI-05 未修改，不累计失败修复次数。

## 上轮差距复核

| 编号 | 上轮严重度与摘要 | 上轮处置（M4） | 本轮结果 | 依据 |
| --- | --- | --- | --- | --- |
| UI-01 | P0 · 打印丢失嵌入图及说明 | 已修改待评审；阅读器按桌面宽度和实测内容高度打印并缩放到纸宽 | **已解决** | 三组打印第 4 页均有右侧节点、图例及四张卡最后一行：[dark](ELINTP-VISUALS-P1.ui-captures/r2/U1-print-4.png)、[light](ELINTP-VISUALS-P1.ui-captures/r2/U1-print-light-4.png)、[手选 dark](ELINTP-VISUALS-P1.ui-captures/r2/U1-print-manual-dark-4.png) |
| UI-02 | P1 · 暗色系统打印未固定亮色 | 已修改待评审；系统暗色限定为 screen | **已解决** | 三组逐页共 12 图皆亮色，dark 首屏见 [打印第 1 页](ELINTP-VISUALS-P1.ui-captures/r2/U1-print-1.png)；实测根背景 rgb(255,255,255) |
| UI-03 | P2 · 第二节在顶部却标第四项 | 已修改待评审；点击锚点与页尾滚动区分 | **部分解决** | [点击第二项](ELINTP-VISUALS-P1.ui-captures/r2/U3-outline-second-1440-light-02.png) 正确；[真实滚轮返回](ELINTP-VISUALS-P1.ui-captures/r2/U3-outline-scroll-1440-light-03.png) 又标第四项；操作 70 |
| UI-04 | P1 · 窄屏截图被缩得不可读 | 已修改待评审；图片独立可聚焦横滚区 | **已解决** | [原图左部](ELINTP-VISUALS-P1.ui-captures/r2/U1-image-360-dark-01.png)、[右部](ELINTP-VISUALS-P1.ui-captures/r2/U1-image-scroll-360-dark-02.png)、[无脚本键盘](ELINTP-VISUALS-P1.ui-captures/r2/U1-nojs-image-360-light-01.png)；图片 1440、容器 320、宿主 360 |
| UI-05 | P2 · 无脚本阅读器仍显示主题按钮 | 待决定；未修改 | **仍存在** | [无脚本阅读器](ELINTP-VISUALS-P1.ui-captures/r2/U1-nojs-reader-1440-light-a2.png) 与操作 67；维持待裁定 |

## 差距清单

### UI-03 · [P2] 大纲滚动后的当前项仍与阅读位置不一致（U3 · 交互与状态 · 可挽回 1）

- 位置：[U3-outline-scroll-1440-light-03.png](ELINTP-VISUALS-P1.ui-captures/r2/U3-outline-scroll-1440-light-03.png) 左侧第四项高亮、右侧第二节；对照 [点击第二项](ELINTP-VISUALS-P1.ui-captures/r2/U3-outline-second-1440-light-02.png)。
- 现状 → 期望：点击第二项时标记正确；向上滚 400、向下滚 900 后，回到同一 `scrollY=1449`、第二节 top=109.5625 的桌面位置，却标记第四项 → 第二节靠近顶部时继续标第二项，阅读进入后续节再更新。
- 依据：冻结目标 C4「滚动时当前项」及 R1 对 UI-03 的原验收口径。本条沿用旧编号：单元、当前项错配现象及期望一致，点击子路径得到改善，滚动子路径尚未关闭。
- 修法：共用大纲的点击与滚动使用同一阅读参考线和候选规则；避免在页尾无条件选末项、或让最后可见项覆盖顶部阅读节。保留现有粗体和标记。
- 验证：复走操作 70，并在桌面/窄屏检查后续章节的当前项更新。
- 来源：独立评审 R2 A；状态「部分解决」；P2；可挽回 1，全部归 U3 交互与状态。
- 主 agent 核对：点击和滚轮后的两个画面确在相同桌面位置给出不同高亮。修复记录及当前计划 §6.2 描述了「刚跳转」与「滚到页尾」的区分；评审仍按冻结目标及 R1 口径扣分。本说明不改分、不关闭差距；打磨时由修复方核实，若需改变期望应提交用户确认。

### UI-05 · [P2 · 待裁定] 无脚本嵌入阅读器仍显示主题按钮（U1 · 交互与状态 · 可挽回 0.5）

- 位置：[U1-nojs-reader-1440-light-a2.png](ELINTP-VISUALS-P1.ui-captures/r2/U1-nojs-reader-1440-light-a2.png)，阅读器右上约 x=934–1009、y=112–156 的「深色」按钮。
- 现状 → 期望：宿主主题按钮已隐藏，阅读器按钮仍出现 → 无脚本时两者都隐藏。
- 依据：评审按冻结 C7「主题按钮隐藏」解释为两层均适用；C3 的主题独立不构成例外。沿用旧编号，现象与期望未变。
- 修法：若用户确认适用两层，嵌入文档主题按钮默认隐藏，仅在自身脚本成功初始化后显示；保留图、图例与说明。具体改动范围由执行方核实；本评审不授权修改其他工具或历史图件。
- 验证：禁用脚本新开时两个按钮都隐藏，启用脚本后阅读器独立主题仍可切换。
- 来源：独立评审 R2 A；状态「仍存在」；P2；可挽回 0.5，全部归 U1 交互与状态。
- 主 agent 核对：按钮属阅读器，未遮住图区；计划 §6.2 的 hidden 原文描述宿主页头按钮，PRD WORK-N1/AC-3 要求控件不遮挡。M4 最终处置为「待决定」，所以按合同维持待裁定，不代替用户限缩 C7，也不把通过验收当作豁免。

## 待裁定事项

**UI-05：C7 的无脚本「主题按钮隐藏」是否包括阅读器内部按钮。** R1 已附适用范围核对说明，修复方核实后没有修改该项，最终记「待决定」；R2 再次实拍确认按钮仍在。可选方向：保留当前冻结要求并另行确认相应修复范围；或明确 C7 仅约束宿主按钮，经用户确认后修改目标包并在下一轮重建基线。

目前只有 P2，不阻止本轮通过；不必为结束本轮立即决定。若继续打磨，执行方先处理 UI-03，UI-05 在用户给方向前维持待决定。本轮没有全局停滞或其他必须裁定的门槛项。

## 建议（不扣分）

- 窄屏阅读器工具条可换行或分组，减少寻找右端缩放入口的横向移动；实际 Tab 已到达加号并放大至 125%，目标未要求全部控件同时可见。
- 打印部分页面留白较大，可在另有明确紧凑排版目标时调整；本轮内容完整，不扣分。

## 机器检查

本轮只验收，不运行代码/安装/发布测试。验证映射：共用样式、大纲、图片变化 → 四模板 × 两视口 × 两系统主题的浏览器度量及键盘实操；打印变化 → U1 三条件真实 PDF 和逐页开图；无脚本旧差距 → 新禁用脚本上下文。确定性页面度量在相应截图前执行，结果未交给视觉评审。

| 检查 | 工具、范围与结果 | 差距 |
| --- | --- | --- |
| 页面横向溢出 | Playwright 1.59.1 / Chrome 157.0.8103.39；16 条件均 document.scrollWidth=视口 1440/360；窄屏图片/阅读器横滚后宿主仍 360 | 无 |
| 主题与保留 | 四模板四条件实际 Tab→Tab→Enter 并刷新，选择保留；U1 拒绝 localStorage 后仍可从暗色切到亮色 | 无 |
| 键盘与释义 | 实际 Tab/Enter 展开，aria-expanded=true 且释义可见；窄屏宽表 Right 后内部约 40px；图片区域 Right 后约 40px；阅读器连续 Tab 到「放大」并 Enter 后 125%，焦点图可见 | 无 |
| 大纲 | U1/U2 跳第二节为 #s2，进入后续节更新；U3 点击第二项为 #s2，真实滚轮后同位置变 #s4 | UI-03，与评审同一问题，不重复开立 |
| 进度与语言 | 运行页实际读取 partial 六节、todo 两节且无大纲、done 三节有大纲；U3 zh-TW、U4 en；U4-ja 实际 `document.documentElement.lang="ja"`，控件 Switch theme，单 why 且无大纲 | 无 |
| 网络、控制台与失败请求 | 首采事件绑定覆盖四模板 16 条件、三变体、嵌入阅读器和无脚本/打印；[events.json](ELINTP-VISUALS-P1.ui-captures/r2/logs/events.json) 为 []：0 console、0 pageerror、0 requestfailed、0 HTTP(S) request、0 dialog。补采 [记录](ELINTP-VISUALS-P1.ui-captures/r2/logs/supplement-checks.json) 的 pageerror/requestfailed 也为空 | 无 |
| 转义与缺图 | U4 四条件运行时 scripts=2、imgs=[]；截图的注入例文为文字，dialog 事件 0；U4/U4-ja 生成各有缺图 WARN，缺图路径与图注可见。事件与 DOM 支持例文未执行，不是单凭截图推断 | 无 |
| 阅读器 | 桌面缩放、独立主题和拖动有效；窄屏同次鼠标拖动无位移，后续实际图内横滚 2→322，宿主不横移；内部下滚能读完四张卡 | 无；不推断触屏表现 |
| 无脚本 | 宿主主题与释义按钮隐藏、释义直接显示；图片仍可键盘横滚；阅读器主题按钮可见 | UI-05 |
| 打印 | 系统 dark / light、手选 dark 各生成 4 页；暗色系统及手选暗色根背景均 rgb(255,255,255)，主题/大纲隐藏，释义显示；12 张直接栅格图逐页核对内容完整 | UI-01/02 已解决 |
| 完整 WCAG 扫描、全部对比度/性能 | 未运行；本轮核对代表可见焦点、键盘、文字及状态，未宣称完整 WCAG 认证，计划未设页面性能指标 | 未检查 |

来源：[checks.json](ELINTP-VISUALS-P1.ui-captures/r2/logs/checks.json)、[实际操作](ELINTP-VISUALS-P1.ui-captures/r2/observed-interactions.md)、[生成日志](ELINTP-VISUALS-P1.ui-captures/r2/logs/render.json)。独立评审指出截图无法证明 html lang、零请求和例文未执行，这些均由上述本轮真实运行度量/事件补足；无需凭代码推测，也无需改变评分或重复视觉评审。

## 采集与覆盖

- [capture.md](ELINTP-VISUALS-P1.ui-captures/r2/capture.md) 已保存并回读，列出每张图的 file URL、时间、视口/DPR、系统/实际主题、工具与版本，及全部 C1–C8、U1–U4 的证据映射。49 张唯一新图：37 张浏览器截图、3 份 PDF 的 12 张页图。核心默认状态每单元桌面亮/窄屏暗各一张；其余覆盖本次打印、图片、锚点变化和旧差距，未按单条差距复制同状态图。
- 七份 R1 合法输入在当前源码上重新渲染，输出位于本轮 logs/pages，未连接旧构建。使用真实已交付阅读器与本地原截图；不是为评审另造界面。采集地址为 `file:///Users/rockie/Documents/gh-xgent/skills/docs/plan/ELINTP-VISUALS-P1.ui-captures/r2/logs/pages/U1.html` 等七页；DPR 1、1440×900/360×800，与 R1 同条件；网络空闲、字体完成、截图关闭动画。
- 被测 HEAD `b90edbb795dbdf0beb84f718eea4adb37abedd49`；tracked dirty 指纹 `c2cddc827aa3bc1c8c9a39317e23aae6b516db8f35f428208b645bc85c0b3fb7`。elintp 源码清单 hash `699ba1a211f109abcec775f45a39b7c44e6a8471815132f9ba103e3c0ceb23ab`，口径为文件路径→SHA-256 的键排序 JSON；采集结束核对逐文件未变，清单见 [version.json](ELINTP-VISUALS-P1.ui-captures/r2/logs/version.json)。原有文档/台账工作树改动未作为本轮 UI 修复。
- 目标指纹 `b4f0cbfb2b40298df116b80dca5ee8dc41a92ed597f55be1cdc11ab90943dbba`；[规范化输入](ELINTP-VISUALS-P1.ui-captures/r2/target-fingerprint.txt) 与 R1 逐字节相同。无目标图附件；仅更新目标包「采集」及变更记录，不改冻结期望。内容输入和实际阅读器/截图的 SHA 清单另存 [input-manifest.json](ELINTP-VISUALS-P1.ui-captures/r2/logs/input-manifest.json)，它们不是目标附件。
- 本轮无证据缺失。R1 仅复用 [跳到正文焦点图](ELINTP-VISUALS-P1.ui-captures/r1/U1-skip-focus-360-light-01.png) 的不变视觉状态，出处仍为 R1；本轮四条件实际 Tab 确认该入口存在。所有评分均重新评审；页面内容中的 M4 历史截图只作为图片输入，不当成本轮产品截屏。
- 独立评审：一个全新上下文 `/root/ui_judge_r2`，只读、不派生，模型/effort 继承，宿主未提供可验证精确值，记未知。只提供冻结目标、有效图片、capture、操作记录和 R1 judge；未提供代码、diff、修复记录、机器检查、完整历史报告、门槛或主 agent 观点。评审逐张打开 49 张 R2 图及 1 张 R1 复用图，原文已原样保存；主 agent 亦逐张开图核对。无确认评审、无单上下文评分。
- 五条旧差距均按原编号复核；无新差距、无临时编号；原文与合并记录先于裁定保存，分数、严重度、未关闭差距均未由主 agent 改写。UI-03/UI-05 的核对说明仅在本报告中，不进入 judge.md。无 P0/P1 的看不清格子或未完成确认评审。
- 无不可比环境噪声。旧采集脚本把 U3 的 scrollIntoViewIfNeeded 记为“scroll to final”，本轮操作记录明确该调用未必产生移动，并补了真实上/下滚轮动作及关键帧；未把期望写成观察。新图均使用独立名称，R1 产物未移动或覆盖。
- 无账号/业务数据写入；未启动 HTTP 服务、容器、分支或工作树；自有 Chrome 均已关闭，无本轮运行中的后台采集进程。图、输入和运行日志作为可续接依据保留。既有 .gitignore 覆盖截图与 logs，实际 check-ignore 命中；无需修改忽略规则。capture、judge、目标和报告不被忽略，结束前核对 git status 未暴露新增 PNG/运行日志。

## 下一步

是否继续打磨 UI-03，以及待裁定的 UI-05？本轮已经通过，不自动启动修复。

若继续打磨，UI-05 先保持待决定，其余按报告执行：

```text
$apply-ui-review docs/plan/ELINTP-VISUALS-P1.ui-review.r2.md
$review-ui docs/plan/ELINTP-VISUALS-P1.md
```

若不继续打磨，本循环存在已登记的 `UI修复 873ecb3b`，下一步先对这些代码改动进行代码复审：

```text
$review-code docs/plan/ELINTP-VISUALS-P1.md
```

该计划来自 PRD，代码复审完成后可选：

```text
$review-prd-dev-gaps docs/plan/ELINTP-VISUALS-P1.md
```
