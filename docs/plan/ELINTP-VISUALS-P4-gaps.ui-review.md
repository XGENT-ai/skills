# ELINTP-VISUALS-P4-gaps · UI 验收评审 · R1

## 评审对象

| 轮次 | 开发计划 | 目标包 | 被测版本 | 截图 | 上轮报告 |
| --- | --- | --- | --- | --- | --- |
| R1（首轮；本轮建立目标包） | [ELINTP-VISUALS-P4-gaps · 数值图表补缺](ELINTP-VISUALS-P4-gaps.md) | [ELINTP-VISUALS-P4-gaps.ui-target.md](ELINTP-VISUALS-P4-gaps.ui-target.md) · 目标指纹 `cbb77004f7a8` | HEAD `8f3e6665`（工作树另有 docs 台账与评审文档未提交改动，diff SHA-256 `f7ce0226…`；`tools/phoenix-diagram`、`dist/phoenix-diagram` 相对 HEAD 无差异）· `file://` 本地产物 | [r1](ELINTP-VISUALS-P4-gaps.ui-captures/r1/) | 无 |

## 验收判定

**需要修改**

本轮分数 8.0（U2）达到门槛 8/10，但 U4 存在未关闭的 P1 门槛项 UI-01（饼图收尾扇区与列表色块是中性灰，不是 S2 要求的绿系强调色），按合同不能通过；另有 3 条 P2（其中 UI-02、UI-04 附主 agent 核对说明，事实有争议，交修复方核实）。无停滞预警、无待裁定事项。

## 评分

| 单元 | 布局 | 视觉 | 交互与状态 | 细节 | 单元总分 |
| --- | --- | --- | --- | --- | --- |
| U1 两系列柱状图（en，ms） | 3 | 2 | 3 | 1 | 9 |
| U2 两系列折线图（zh-CN，ms） | 2 | 2 | 3 | 1 | 8 |
| U3 限额图（zh-CN，req/s） | 3 | 2 | 3 | 1 | 9 |
| U4 六行饼图（en，files） | 3 | 2 | 3 | 1 | 9 |

- 本轮分数：**8.0**（最低单元 U2）；历史最佳：8.0；门槛：**8/10**（用户未指定、无历史报告，取默认）。
- 单元达标：U1、U2、U3 达标（分数 ≥ 8 且无门槛项）；U4 不达标（门槛项 UI-01）。
- 评审为单评审，无确认评审或重评，不涉及多评审取低分。

## 轮次轨迹

| 轮次 | 本轮分数（最低单元） | 历史最佳 | 未关闭 P0/P1 | 重构轮 | 备注 |
| --- | --- | --- | --- | --- | --- |
| R1 | 8.0（U2） | 8.0 | 1 | — | 首轮；建立目标包与采集基线 |

## 上轮差距复核

首轮，无。

## 差距清单

### UI-01 · [P1] 饼图收尾扇区与列表色块是中性灰，不是强调色槽（U4 · 视觉 · 可挽回 1.0）

- 位置：[U4-default-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U4-default-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U4-default-1440-dark.png) 12 点方向左侧收尾扇区（Fixtures）及右侧列表第 6 行色块；[U4-static-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U4-static-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U4-static-1440-dark.png) 同位置；U4-table 截图列表与节点索引「Fixtures」chip 同色
- 现状 → 期望：收尾扇区与列表色块为中灰/石板灰（亮暗两主题相同）→ 目标包 S2：「饼图收尾扇区（第 6 块）与列表色块用绿系强调色，列表色块与扇区同色」（G-02 强调色槽；计划 §5.2）
- 修法：改引擎饼图配色槽——把收尾扇区专用强调槽从灰中性 token 换为绿系强调 token（亮暗各配一档，与系列普通绿区分开），列表色块与节点索引 chip 引用同一 token 保持「色块与扇区同色」。纯配色 token 替换，不动布局或素材
- 来源：评审 R1
- 主 agent 实测（不改分）：运行页面取色，第 6 扇区与列表色块 computed fill 亮 `rgb(100,116,139)`、暗 `rgb(148,163,184)`（`--external-stroke` 的当前值，石板灰），第 1 扇区亮 `rgb(8,145,178)`——「现状是灰、非绿系强调」属实；两者同色（S2 的同色要求已满足）。

### UI-02 · [P2] 图例系列名疑有贯穿横线（U1＋U2 · 视觉 · 可挽回 U1 1.0 ＋ U2 1.0）

- 位置：[U1-default-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U1-default-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U1-default-1440-dark.png)、[U2-default-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U2-default-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U2-default-1440-dark.png) 左下图例区「键 · 名称」的名称文字（"Before""After""p50""p95"），亮暗一致出现
- 现状 → 期望：评审看到系列名称文字上有贯穿横线、形似删除线，且 U3 图例名称无此线 → 图例名称应为可读常规文本（S1「键 · 名称」；S7 文字图例帮助自明，删除线惯常读作「已删除/已停用」）
- 修法（评审给）：排查阅读器图例系列名样式中的 `text-decoration: line-through` 或等高伪元素/边框装饰并去掉，与 U3 图例名称样式统一；亮暗各复核一次
- 来源：评审 R1
- 核对说明（主 agent，2026-11-16）：DOM 实测图例全部 `text` 元素 computed `text-decoration-line: none`，SVG 标记中无装饰线元素，`elementsFromPoint` 命中栈只有文本与容器；对引擎截图与同页面 Playwright 重拍做 1×/3× 放大裁剪（[logs/factcheck/U1-legend-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/logs/factcheck/U1-legend-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/logs/factcheck/U1-legend-dark.png)），「横线」与画布空白区（[参照图](ELINTP-VISUALS-P4-gaps.ui-captures/r1/logs/factcheck)）的点阵纹理逐点对应——是画布背景点阵网格（点距约 40px，恰在文字 x 高度），不是删除线；U3 图例区同样有此网格。请修复方核实；若确认是网格点，UI-02 不成立，下一轮评审按修正后的事实重评该两单元的视觉维度。

### UI-03 · [P2] 折线图近零点数值标注相互压叠、压标记与轴线（U2 · 布局 · 可挽回 1.0）

- 位置：[U2-default-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U2-default-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U2-default-1440-dark.png)「低负载」数据点处；[U2-static-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U2-static-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U2-static-1440-dark.png) 同（缺陷同源，S6 一致）
- 现状 → 期望：A（0.1）与 B（0.3）两点几乎重合，标注「A 0.1」「B 0.3」上下行距≈0，标记圆圈压住「0.1」「0.3」数字，x 轴线穿过「B 0.3」→ 数值标注应互不重叠、不压数据点标记与轴线（目标包比较口径「没有溢出、遮挡、截断」；S7 图自明、S8 数值可读）
- 修法：折线图数值标注定位：同一 x 附近多系列点重叠时按系列序把标注放到点上方/下方错开（最小间距≈标注行高），y 近 0 的点统一抬到轴线上方，标注与圆点标记间留 ≥2px；现有 SVG 文本元素内调整，不换组件
- 来源：评审 R1
- 主 agent 放大核对（[logs/factcheck/U2-lowload-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/logs/factcheck/U2-lowload-light.png)）：压叠属实。另注：计划 M1 记录曾把「0.1/0.3 标注间距较紧」登记为既有观察未处理，与本条同源。

### UI-04 · [P2] 仅上限行虚线框疑越过上限刻度（U3 · 视觉 · 可挽回 1.0）

- 位置：[U3-default-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U3-default-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U3-default-1440-dark.png)「导出」行；[U3-static-1440-light.png](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U3-static-1440-light.png) / [-dark](ELINTP-VISUALS-P4-gaps.ui-captures/r1/U3-static-1440-dark.png) 同
- 现状 → 期望：评审看到虚线框画满整个标尺、虚线越过上限刻度继续延伸到标尺末端 → S4/D-02「长度到上限」，虚线框右缘应止于上限刻度
- 修法（评审给）：虚线框宽度由「画到标尺末端」改为「止于上限值位置」（右缘与该行上限刻度线重合收口）；行标尺与刻度位置不动；虚线样式与图例「仅上限」符号一致
- 来源：评审 R1
- 核对说明（主 agent，2026-11-16）：SVG 坐标实测（limits.embedded.svg）：仅上限行 span `x=192 width=336`，右缘 x=528；该行上限刻度线 `x1=x2=528`——右缘与上限刻度精确重合，未越过。评审看到「刻度线右侧的虚线段」是轨道背景 `ch-track`（width=420，右缘 x=612），三行轨道等长（另两行填充右侧的空白段同为此背景）；评审样张缩放下 span 虚线与轨道右段视觉混叠（[放大裁剪](ELINTP-VISUALS-P4-gaps.ui-captures/r1/logs/factcheck/U3-only-row-light.png)）。S4「长度到上限」按几何已精确满足；请修复方核实，若确认无误，UI-04 不成立。

## 待裁定事项

无。

## 建议（不扣分）

- 建-1：节点索引摘要按固定宽度截断，数字被截成「After 26…」「p95 2…」（U1/U2 可见；U3 三节点无截断，是宽度分配问题）。S1–S8 未覆盖节点索引；建议加宽摘要列或超宽时换行/隐藏。
- 建-2：画布利用率低——绘图区只占画布中部一小块（U1/U2 约 360×370px，四周大片点阵空白；U4 环图＋列表集中左侧）。无设计稿约束，属偏好。
- 建-3：数据表数值列左对齐；位数悬殊的数字（0.1 与 1234567）不便比大小，可改右对齐。规范未约束。
- 建-4：U2 y 轴刻度显示原始数字（2500000 ms）；可加千分位分隔符（不改变数值，不违反 S8「不舍入」）。规范未要求。

## 机器检查

| 检查项 | 工具与版本 | 结果 | 对应差距 |
| --- | --- | --- | --- |
| 引擎自动检查（validate / deliver、static / browser-check，含文本包含与几何检查） | phoenix-diagram 0.2.0（本仓 HEAD 构建，dev 二进制） | 8/8 exit 0，全部 stage pass、无诊断（`logs/products/<n>.finalize-summary.json`、`<n>.embedded.static-summary.json`） | 无新差距 |
| 静态图 `data-phoenix-table` 与阅读器表格逐格比对 | node 脚本 [logs/interact/table-parity.cjs](ELINTP-VISUALS-P4-gaps.ui-captures/r1/logs/interact/table-parity.cjs)（Node 25.2.1） | 4/4 `same=true`（表头与全部行逐格相同） | 无 |
| 数据表复制主路径（展开→选首行→复制→读剪贴板） | Playwright（tools/phoenix-ui/node_modules）+ 系统 Chrome headless；浏览器内 `navigator.clipboard.readText` | 8/8 剪贴板文本正确（见 [observed-interactions.md](ELINTP-VISUALS-P4-gaps.ui-captures/r1/observed-interactions.md)） | 无 |
| 页面级横向溢出、控制台 error/warning、失败请求 | 同上（8 轮交互逐轮记录） | scrollWidth=clientWidth=1440，无溢出；控制台与失败请求均为空 | 无 |
| 争议差距的运行时实测与放大取证 | Playwright computed style/坐标读取；`sips` 裁剪 | UI-01 属实（灰非绿）；UI-02「删除线」不成立（点阵网格）；UI-04「越过刻度」不成立（右缘=刻度 x=528）；UI-03 属实 | UI-01–UI-04（见各条核对说明/实测） |
| 可访问性扫描（axe/Lighthouse）、键盘焦点走查 | 未执行 | 本轮目标包采集矩阵未含这些状态，未纳入本轮采集；如实记「未检查」，不当作通过 | — |

机器检查未新开差距；争议事实以「核对说明」附在对应差距下，不改分、不删差距、不改门槛项。

## 采集与覆盖

- 环境与被测版本：仓根 HEAD `8f3e6665`，`tools/phoenix-diagram` 相对 HEAD 无未提交改动；引擎按计划 §9.1 以 `mise exec --no-deps -- mbx build --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --bin phoenix-diagram` 从当前代码构建（dev 二进制）。无临时服务（全部 `file://` 本地产物，无端口、无后台进程需要停止）。
- 采集工具与条件：phoenix-diagram 0.2.0 finalize/static 内建截图（1440 宽整页，light/dark）；Playwright + 系统 Chrome headless，1440×900、DPR 1、`reducedMotion: reduce`、`?theme=` 控制主题、等 networkidle 与 `document.fonts.ready`。输入与产物 hash：bar.json `08661f06…`、line.json `389db7d6…`、limits.json `f2b944bc…`（本轮新写，较计划 V-1 输入给两行加 note）、pie.json `cfc6302d…`；bar.html `5d70f5d6…`、limits.embedded.svg `4e9d843f…`（其余产物 hash 在 `logs/products/*.json` summary 内）。
- 本轮实拍 24 张（每单元：阅读器默认亮暗、数据表展开亮暗、embedded 静态图亮暗）；实测：复制 8 轮、表格逐格比对 4 份、溢出/控制台监测 8 轮；历史复用 0；缺证 0。唯一截图数 24；条件→证据映射见 [capture.md](ELINTP-VISUALS-P4-gaps.ui-captures/r1/capture.md)「验收条件覆盖」。
- 补采增加的信息：无补采；`logs/factcheck/` 为裁剪与 DOM/坐标核对材料（主 agent 核实用，不进评审材料）。
- 环境噪声：第一次交互试跑的复制读数缺陷已废弃——headless Chrome `execCommand('copy')` 不写 macOS 系统剪贴板，`pbpaste` 读到的是系统剪贴板既有无关内容；该次截图沿用，复制文本以浏览器内 `navigator.clipboard.readText()` 的第二轮核验为准，系统剪贴板未被写入或清除。无其他噪声。
- 评审方式：1 位全新上下文、只读 subagent（agent `reviewer`，模型 `coding-plan/glm-5.3-flash`，继承主 agent），整批材料一次评四个单元；无确认评审、无重评、非单上下文评审。原文：[judge-a.raw.md](ELINTP-VISUALS-P4-gaps.ui-captures/r1/judge-a.raw.md)，合并结论：[judge.md](ELINTP-VISUALS-P4-gaps.ui-captures/r1/judge.md)。
- 主 agent 对评审结论的核对说明：对 4 条差距逐条做了事实抽查（DOM/坐标实测与放大裁剪）；认为 UI-02、UI-04 的现象不成立，核对说明附在上述差距条目下，未改分、未删差距；UI-01、UI-03 属实。主 agent 未以自己核实新增差距。

## 临时编号 → 正式编号映射

新-1→UI-01、新-3→UI-02、新-2→UI-03、新-4→UI-04（P1 优先，同严重度按可挽回分从高到低）。
