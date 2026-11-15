# ELINTP-VISUALS-P3-gaps · UI 验收目标包

- 需求来源：[PRD](../PRD-ELINTP-VISUALS.md) v1.2 · P3：WORK-F9/AC-1、AC-2，WORK-F5/AC-2、AC-5，WORK-N1/AC-1～AC-3，WORK-N2/AC-1、AC-2，WORK-N3/AC-3；[P3-gaps 计划](ELINTP-VISUALS-P3-gaps.md) §0.1、D1–D5、§5.2。
- 没有设计稿或项目 DESIGN.md，全部按规范模式比较。内容稿是代表性输入，经 elintp 公开渲染入口生成真实页面，不是为评审另造的界面。

## 单元

| 单元 | 状态 | 视口 × 主题 | 目标 | 比较口径 |
| --- | --- | --- | --- | --- |
| U1 终稿中的静态图卡片 | 默认（规则判定卡片、作者卡片含 `<b>` 与 `&`、长条目卡片、无卡片旧图）、主题切换、窄屏、无脚本、显式暗色下打印 | 1440×900、360×800；DPR 1；light/dark；无脚本与打印以桌面代表 | 共用规范 C1–C3；K1–K7 | 规范模式；卡片的具体文字是示例数据，不评文案好坏，但必须与同一 candidate 的 Reader 卡片一致；静态图内部的图形与文字由 phoenix-diagram 审核，这里只看它在页面里是否完整、是否跟随主题 |
| U2 审阅页中的静态图卡片 | 默认收起、开启标注、为带卡片的图添加标注后 | 1440×900 × light；360×800 × dark | 共用规范 C1–C3；K1–K5、K8 | 规范模式；审阅控件本身沿用既有审阅规则，只看它与卡片区的关系 |

## 规范要点

- C1：沿用固定阅读模板与系统字体；亮暗两套下正文、图与卡片文字可辨认；终稿单文件，打开时不发起网络请求，不含 iframe（PRD WORK-F5/AC-2、WORK-N2/AC-1、WORK-F3；P1 目标 C1/C3）。没有已确认的像素目标或新增 token 数值。
- C2：360 CSS px 与 1440×900 下均无页面级水平滚动；宽图在自身区域横向滚动；文字不被裁剪、不被遮挡（PRD WORK-N1/AC-1）。
- C3：状态与类型的区分不只依赖颜色；打印与禁用脚本时显示全部正文、图与卡片，交互控件不遮挡内容（PRD WORK-N1/AC-2、AC-3）。
- K1：卡片区在 `<figure>` 内，位于静态图之后、图注之前；每张卡片是一个标题加一组条目列表。卡片区没有标题元素，不出现在大纲里（计划 §5.2、D4）。
- K2：卡片的标题与条目和同一 candidate 的 Reader 图下卡片逐项相同、顺序相同（状态机的规则判定卡片在前，作者卡片在后）；没有遗漏、改写或补造。文字一律按原文显示，`<b>`、`&` 显示为字面文字，不被当作标记执行（PRD WORK-F9/AC-1 经计划 WORK-F9/AC-1 行、WORK-N2/AC-2；计划 D3、C-1）。
- K3：卡片的 `dot` 颜色不显示；卡片含义完全由标题和条目文字表达（计划 D4；PRD WORK-N1/AC-2）。
- K4：卡片使用页面已有的表面底色与分隔线颜色作边框，标题加粗（字重 650），条目沿用正文颜色；桌面宽度下多张卡片排成多列网格，每列不窄于 220 px，卡片之间与卡片区上方各留 12 px；颜色全部取页面已有变量，亮暗与打印随页面切换（计划 §5.2）。
- K5：窄屏下卡片自动排成单列，长条目在卡片内换行，卡片不超出正文宽度（计划 §5.2；PRD WORK-N1/AC-1）。
- K6：显式暗色下打印为亮色，卡片完整显示，单张卡片不被分页切开（计划 §5.2；PRD WORK-N1/AC-3）。
- K7：不带卡片数据的旧静态图照常显示，图下没有卡片区，也没有空容器或占位（计划 D5；PRD WORK-N3/AC-3）。
- K8：审阅页里的卡片与终稿显示一致；开启标注后，图块的标注控件与已添加的标注不遮挡卡片和图注（计划 §4「审阅页与终稿共用 `parseDraft`」；P2-gaps 目标 C2、C4）。

## 采集

- 计划：[ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md)。R1 前无本循环报告或目标包。
- 在仓根用开发引擎 `local/phoenix-diagram/elintp-p3-gaps/phoenix-diagram-darwin-arm64`（构建输入与当前工作树逐文件一致）对三份 candidate 运行 `static`：`minimal.statechart.json`（system，规则判定卡片）、`workflow-candidate.json`（embedded，两张作者卡片，条目含 `<b>` 与 `&`）、`orders-candidate.json`（system，两张长条目卡片）；同一 candidate 另以 `deliver` 取 Reader HTML 作卡片对照。旧图取 P3 已交付的 `local/phoenix-diagram/elintp-p3-final-out/workflow.system.svg`。输入、SVG、页面与日志放 `ELINTP-VISUALS-P3-gaps.ui-captures/r<n>/logs/`。
- 终稿：`node skills/elintp/scripts/render.mjs <logs/inputs/draft.md> <logs/pages/final.html>`，在离线浏览器上下文以 `file://` 打开。审阅页：`node skills/elintp/scripts/review.mjs <draft.md> <review.html> <result.json> --no-open`，打开输出的本机 `REVIEW URL`。无需账号，不写业务数据；本轮不提交标注。
- 条件：Playwright 驱动系统 Chrome，DPR 1；等待字体与网络空闲，截图关闭动画；无随机数或时钟内容。主题用页面自身的主题按钮切换；无脚本用 `javaScriptEnabled=false` 的新上下文；打印由浏览器在显式暗色下生成 PDF 后转图检查。

| 单元 · 状态 | 入口与前置 | 到达步骤 |
| --- | --- | --- |
| U1 · 默认 | 终稿 `file://`，离线 | 打开页面，等字体与网络空闲，按小节截图 |
| U1 · 主题切换 | 同上 | 点主题按钮切到暗色 |
| U1 · 窄屏 | 同上，360×800 | 打开页面，滚到各卡片区 |
| U1 · 无脚本 | 同上，禁用脚本 | 打开页面 |
| U1 · 打印 | 同上，先切到暗色 | 生成 PDF，转图 |
| U2 · 默认与开启标注 | 审阅页本机 URL | 打开页面；点「标注内容」开启标注 |
| U2 · 添加标注后 | 同上 | 为带卡片的图点「＋ 添加标注」，选类型并填一句话后保存 |

## 变更记录

- 2026-11-13 R1 建立。期望来自上述 PRD 与计划条目；窄屏高度 800、DPR 1、代表输入与 U2 的视口主题组合是本轮采集假设，用户调整期望后下一轮重建基线。
