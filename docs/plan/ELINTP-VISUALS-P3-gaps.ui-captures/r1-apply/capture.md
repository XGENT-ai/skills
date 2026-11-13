# R1 → apply · 修复自检材料

- 采集：2026-11-14 00:01 +08:00；来源：[R1 报告](../../ELINTP-VISUALS-P3-gaps.ui-review.md)；目标：[目标包](../../ELINTP-VISUALS-P3-gaps.ui-target.md)，指纹 `0a84be6430e5a5eb2271ee726b4288dedcba32a6126cf61941882074a8f82867`（沿用 R1 口径，重新计算相同）。
- 版本：`dirty@c7c77b8d`，仅产品改动 `skills/elintp/assets/page.css`。当前 CSS 与采集页面只有注释措辞差异，声明相同；保留生成页供核对。提交后修复版本见 [M2 修复记录](../../ELINTP-VISUALS-P3-gaps.records/M2.md#ui-验收修复)及计划完成记录。
- 共用输入：直接复用 R1 `logs/inputs/draft.md`、三份静态 SVG 与旧 SVG；没有重新生成图、改变数据或目标。当前渲染入口生成 `logs/pages/final.html`、`logs/pages/review-a3.html`。
- 工具：Playwright 1.63.0 / 系统 Chrome 157.0.8103.78，headless，DPR 1，reducedMotion=reduce，字体就绪；U1 离线 `file://`；U2 为 review CLI 本机临时 URL `http://127.0.0.1:62460/`。没有 mock、业务账号或真实业务数据，没有提交标注。
- 产品影响范围：共用 `.image-scroll`（终稿与审阅页的静态图、普通图片）；打印 `.figure-cards`。没有更改引擎、数据、卡片文字、屏幕卡片尺寸与色板。

## 复现与自检

| 问题 / 条件 | 操作与结果 | 共用材料 |
| --- | --- | --- |
| UI-01 · U1 / 360×800 / light | 首图顶端距视口 16 px；图区域内横向滚轮 200 px ×12，300 ms 后及再等 1.5 s 各拍一帧。修复前 y=400 的卡片像素左缘 19→20，修复后 20→20；图仍滚至 scrollLeft=500（820−320）。页面 scrollX=0，scrollWidth=clientWidth=360 | [修复前即时](before-U1-narrow-360-light-figure-scrolled-a3.png)、[前稳定](before-U1-narrow-360-light-figure-scrolled-settled-a3.png)；[修复后即时](U1-narrow-360-light-figure-scrolled-a3.png)、[后稳定](U1-narrow-360-light-figure-scrolled-settled-a3.png) |
| UI-03 · U2 / 360×800 / dark | 开启「标注内容」，为第 2 图添加「疑问」及原 R1 文句，工具条显示 1 条标注；该图顶端距视口 260 px，同样滚轮 ×12。y=600 卡片左缘修复前 19→20，修复后 20→20；图 scrollLeft=477（797−320），页面宽360、scrollX=0，标注仍可见 | [修复前即时](before-U2-annotated-scrolled-360-dark-a3.png)、[前稳定](before-U2-annotated-scrolled-settled-360-dark-a3.png)；[修复后即时](U2-annotated-scrolled-360-dark-a3.png)、[后稳定](U2-annotated-scrolled-settled-360-dark-a3.png) |
| UI-02 · U1 / 显式 dark → A4 打印 | 点主题按钮切暗，四边 10 mm，printBackground=true；PDF 仍 5 页。第 2–4 页三组卡片完整，右侧竖边框与圆角可见，单张卡片未跨页；打印 color-scheme=light，底色 rgb(241,243,242)，边框仍 1px var(--line)，break-inside=avoid | [第2页](U1-print-dark-a4-2.png)、[第3页](U1-print-dark-a4-3.png)、[第4页](U1-print-dark-a4-4.png)；200 dpi 右缘：[p2](U1-print-right-200dpi-2.png)、[p3](U1-print-right-200dpi-3.png)、[p4](U1-print-right-200dpi-4.png)；原 PDF `logs/final-print.pdf` |

- 主 agent 已打开全部修复后截图（4 张滚动帧、5 张打印整页、3 张 200 dpi 局部），逐一核对；同状态共用截图，不按问题重复拍。
- 复现前材料来自当前未修改的入口；旧 R1 材料仅作目标和历史对照，没有冒充新采集。
- 定位实验：页面无文档横向溢出，contain 隔离了页面回弹；打印时边框落在 PDF 的片段裁切边缘，grid 的 1 px 内边距使其内收并恢复右缘。1 px 外边距与内描边实验未恢复，不进入产品代码。实验材料保留在本目录或 logs/，不作为独立验收结论。
- 像素断言：`logs/check-pixels.cjs` 读取上述真实 PNG，经浏览器 canvas 检查卡片左缘，修复前、后 8 个读数分别为 19/20、20/20（两单元）；结果 `logs/pixel-edges.json`。不是以 DOM scrollX 代替实拍。
- 运行日志：`logs/capture.cjs`、`logs/shots.json`（逐帧 UTC 时间与地址）、`logs/ops.json`、`logs/checks.json`、`logs/service.json`；浏览器 pageerror 0，console 有 1 条未携带 URL 的 404；补查页面资源未复现（`logs/noise.json` 为空），未断言其来源，也未扩大修复范围。
- Phoenix UI context 因源码副本缺 ENGINE.json 未运行，未安装/升级；无 PRODUCT.md / DESIGN.md，以目标包与现有实现核实。检测器未运行，浏览器与 PDF 自检实际执行。
- 所有本任务临时浏览器与 review 服务已关闭，标注结果未提交；PNG、PDF、脚本和日志按原忽略约定只保留在本机。复审可直接引用本轮材料；跨机需同步忽略材料或重采。

## 范围与复用

- UI-01 对应 U1 两张修复后滚动图；UI-03 对应 U2 两张修复后滚动图；UI-02 对应同一份 PDF 第 2–4 页及对应右缘局部。
- R1 桌面亮暗、无脚本、Reader 内容一致、旧图兼容的材料保留引用价值：本次未改其布局/内容/脚本/色板；滚动变更已在窄屏两入口重验，打印变更已重验全部卡片组。不是本次新拍，也不代表重新独立评分。
- 没有缺证阻塞或待产品决定项。S1–S4 建议不在本次三条差距修复范围。

## 受影响源码 SHA-256

- `skills/elintp/assets/page.css`：`c35c6da44441b6f8fc245c2595b5d3e9aa2dddff0556e5243688dcdbdb665049`
- `skills/elintp/scripts/render.mjs`：`543e4e89b032e2a4af07457843a2ec2cfa9f9231b6462181201707bcff8f31f6`
- `skills/elintp/scripts/review.mjs`：`16673a333989e506dbd83210470992a55f8697204731609e1471f0e25b768ecf`
- `skills/elintp/scripts/draft.mjs`：`de29f49f77464aae7417f66166b914a06f2e634734ef3d95ce7eee60384684f5`
