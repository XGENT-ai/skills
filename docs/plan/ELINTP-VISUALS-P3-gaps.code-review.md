# ELINTP-VISUALS-P3-gaps · 代码评审 R1

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 · 初审 | [ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md) | `29d692703cfca0e81aa9f7cd5d171ec517f10926..3bf1ff09072727649b2e0e7e75baec6cac5802e3` 的计划净差异，加下述当前未提交文档；工作树范围指纹 `8be313d3cc60d18fb1153ed2671f8d2ecd320fdf6ed47198504017c000eaf474` | 无 |

- 日期：2026-11-13，Asia/Shanghai；仓根：`/Users/rockie/Documents/gh-xgent/skills`。
- 原输入：`$review-code docs/plan/ELINTP-VISUALS-P3-gaps.md`；按计划登记提交及当前未提交变更审查。当前代理独立审查，无并行 reviewer。
- 固定 HEAD：`c7c77b8d3512a0030b1ea7b4e8125b83498a1a69`；index tree：`537550fc4c30fabe66104a42e4f9522175337682`，无 staged 差异、无冲突。
- 计划 SHA-256：`85bd80b3bcb13a7f1b8644100fefef865c9cbec0cd6aaac0cc90af66bf726ab9`。
- 实现提交：`13435a65c0fa09d17a785df9239ef65192ed289c`；M2 登记提交：`3bf1ff09072727649b2e0e7e75baec6cac5802e3`（仅计划及 M2 记录）。两者均在当前分支。
- `4e17357e992745a78d3c3728d6862ee0867147fe`、上述 M2 提交、`c7c77b8d3512a0030b1ea7b4e8125b83498a1a69` 的进度记录不作为代码 findings 对象；区间内没有其他未登记实现提交。净实现等同 `29d69270..13435a65`。
- 本计划无旧代码评审报告；同前缀的 `.review.md` 是计划评审，原 P3 的代码报告属于另一个计划，均不作为本轮修复复审来源。

## 1. 确认问题

未发现可确认的问题，P0–P3 均为 0。没有自动修复实现、修改计划或提交变更。

## 2. 待核实 / 待决策项

本期卡片交付范围没有阻塞验收的待核实或待决策项。

覆盖边界：本轮没有进行 npm 发布、四平台构建、SPMS 门户阅读面验收、真人公测或独立 `review-ui`；这些不属于本期卡片补齐的退出条件。原 P3 范围外的比较关系标签问题保持范围外，不作本轮新增问题。

## 3. 覆盖与验证

### 3.1 意图与范围

计划明确要求：静态 SVG 携带 Reader 同源卡片数据，elintp 安全解析后在图与图注之间显示 HTML 卡片，兼容无卡片旧 SVG，保留 Reader 与 SVG 图面。本轮以此为契约，不扩展图型、协议、发布或安装范围。

完整读取本次实现 diff，覆盖全部 12 个实现提交文件：

- 引擎：`tools/phoenix-diagram/src/render/static_style.rs`；追踪 `write_common` 调用位置、`scene.cards`、Reader `render_with_layers`、输入卡片 schema 与 statechart 编译卡片顺序。
- 页面：`skills/elintp/scripts/draft.mjs`、`assets/page.css`；追踪 `static:` → `inlineSVG` → XML 实体解码 → metadata 结构校验 → HTML 转义 → figure 卡片区，以及审阅页共用 `renderPage/parseDraft` 的路径。
- 文档：`skills/elintp/SKILL.md`、`references/draft-format.md`，`skills/phoenix-diagram/references/static-figure.md`、`delivery-contract.md`，`docs/phoenix-diagram.md`。
- 交付副本：`dist/phoenix-diagram/bundle/manifest.json` 与上述两份 phoenix-diagram reference 副本。
- 测试：`test/elintp/render.test.js` 的攻击表和合法/无卡片用例；新增用例通过 CLI 检查退出码、可见 HTML 与旧文件不变，属于计划 V-2 准入的不可信文件边界验证。

当前未提交内容单列：四份 `skills/use-llm-gateway/{SKILL.md,references/api.md,references/recovery.md,references/self-hosted.md}` 全部 diff 已读取，核对平台资格、模型公开身份、价格/额度与恢复文字的内部一致性；对照相邻门户仓已登记版本 `62fa0a98b72ec349696f80fcfdb4755de59ffd89` 的 `platform-models.ts`、`affinity.ts` 及现有调用契约作为补充来源。没有发现可确认的文档回归；未运行网关、门户、真实供应商或计费调用，不能据此宣称目标部署已具备这些能力。该文档增量与本计划实现独立。

当前未提交的经验、SDLC 状态、原 P3 报告、差异报告和本计划的计划评审报告均已枚举，作为记录排除；没有未跟踪实现文件。新生成的本报告及状态回写不进入本轮输入。

### 3.2 实际执行与证据

| 行为 / 检查 | 命令或方法 | 目标版本与实际结果 |
| --- | --- | --- |
| 不可信 metadata 拒绝、转义、旧页保护与共享审阅入口 | `mise exec --no-deps -- node --test test/elintp/render.test.js test/elintp/review.test.js` | 当前实现与 `13435a65` 相同；19/19 通过，0 skipped。包括 9 类卡片攻击、合法文字转义、无卡片兼容。没有新增测试文件 |
| 已构建引擎是否真是被审源码 | 对 `local/phoenix-diagram/elintp-p3-gaps/source-inputs.json` 的 191 项逐项核对当前文件 SHA-256，同时核对 build receipt 的输入清单与 binary hash | 全部一致；输入清单 SHA-256 `bf23bbcca83910f45c1b0c3493b82591182204cfb62b89182ce989947ff5868f`；binary SHA-256 `9781718b9d9d5800fa7ab5fe549df00ad84595611b090ce510c439d7c8fea377`。复用匹配源码的开发引擎，无需另建历史工作树 |
| 当前真实 CLI 取图 | `local/phoenix-diagram/elintp-p3-gaps/phoenix-diagram-darwin-arm64 static workflow local/phoenix-diagram/elintp-p3-gaps-out/v4/orders-candidate.json <临时目录>/orders.system.svg --font-policy system --json` | 本轮实际执行，退出 0；新 SVG 与 V-4 原产物逐字节一致。没有给临时 SVG 冒记视觉审核 |
| 当前真实渲染入口 | 复制 V-4 的内容稿到隔离目录，仅将图路径改为新 SVG；`mise exec --no-deps -- node skills/elintp/scripts/render.mjs <临时目录>/draft.md <临时目录>/page.html` | 退出 0；页面出现两张完整规则卡片，位置为 `image-scroll → figure-cards → FIGCAPTION`，iframe=0，fontResources=0 |
| 浏览器离线、主题与窄屏 | 独立 Chrome headless 用户目录，通过 CDP 设置 Network offline 后加载上述新页面；切 light/dark，再设 360×800 | HTTP(S) 请求为 0；卡片底色 light `rgb(238,242,239)`、dark `rgb(32,50,54)`；360 px 时 `scrollWidth=innerWidth=360`、卡片宽 320 px、单列、文字无横向溢出。实际查看本轮亮色整页和暗色窄屏截图，文字完整 |
| 暗色下打印 | 在显式 dark 上设置 print media，调用 `Page.printToPDF`；`pdftoppm -scale-to 1400 -png <临时目录>/print.pdf <临时目录>/print`，`pdftotext <临时目录>/print.pdf -` | colorScheme=light；卡片底色 `rgb(241,243,242)`；实际查看第一页图像，卡片与所有条目完整，PDF 文本含两卡全部规则。桌面、窄屏与打印均未发现卡片裁切 |
| 十一类 Reader 不变 | 对 `elintp-p3-gaps-out/v3/*.html` 逐项核对 `elintp-p3-base-out/baseline-sha256.txt` | 11/11 SHA 相同；这属于本轮核验既有产物，不冒称本轮重新运行 11 次 deliver |
| 十一类两策略静态产物 | Python XML 解析 22 份 SVG，逐份读取对应 static summary 的三个阶段状态，比较 SVG/Reader 字节数 | 22/22 XML 有效，三门回执均 pass，体积均 ≤ Reader 50%；statechart 两策略各有 3 张规则卡片，其他最小示例无 metadata；卡片 JSON 与 Reader metadata 内容、顺序相同 |
| bundle 与接口文字 | 比较 manifest 中 38 项的源文件、副本、size 与 SHA-256；读取所有修改的接口复述 | 38/38 一致；无错误的卡片排除复述。没有运行会改写 dist 的同步命令 |
| 已有阶段证据复用 | [M1](ELINTP-VISUALS-P3-gaps.records/M1.md)、[M2](ELINTP-VISUALS-P3-gaps.records/M2.md) 与对应本机产物 | M1 记录 V-1 两类型×两字体策略及 8 张截图审核，M2 记录 fmt/clippy、22 份内联、skill 检查与 V-4 作者角色模拟。本轮源码未变且引擎输入已逐项匹配，复用未失效结果；不把记录中的命令写成本轮重新执行 |

临时复现目录在系统临时目录内；所有写入与独立 Chrome profile 均与仓库隔离。浏览器检查脚本首次因脚本语法、再次因旧 DevToolsActivePort 文件而未启动成功，修正临时脚本后完整走查通过；这两次是评审脚本问题，未归因产品。最终截图、PDF 与结果已亲自查看；收尾清理临时目录及进程，不建立额外长期验收框架。

### 3.3 取证指纹与一致性

以下 20 个文件组成审查范围与直接上下游的固定集合；逐文件取 SHA-256，再按 Python `json.dumps(mapping, sort_keys=True)` 的 UTF-8 字节取 SHA-256，得到 `8be313d3cc60d18fb1153ed2671f8d2ecd320fdf6ed47198504017c000eaf474`。内容结论在落盘前逐项重核，未变化。原始工作树另有包含记录的 34 文件指纹 `9145a10ba381fbc5903e00c674532d019e9030356c8870b636b775da4b04f1c8`；记录文件不影响实现判定。

未提交 gateway 文档仅在原工作树可复现，尚无可同步提交；本报告没有复制其全文作为快照。已提交卡片实现可从上述完整 SHA 取回。

<details>
<summary>固定文件集合与 SHA-256</summary>

| 文件 | SHA-256 |
| --- | --- |
| `dist/phoenix-diagram/bundle/manifest.json` | `f83ed30e877cced18fba58ee4d67e4074f7abe80a7065c4b8f7442b5dbd856b6` |
| `dist/phoenix-diagram/bundle/references/delivery-contract.md` | `1ad889a9be6f71469f04a065ddbdb587062a1930e6d96771d2c37a5c2102cc00` |
| `dist/phoenix-diagram/bundle/references/static-figure.md` | `4b6dbc3eb96ea8eb34a613f3437927217dd53628d33a06ef9f663a29b33ffa6a` |
| `docs/phoenix-diagram.md` | `3b6da7f509edd00a20e7e52abf2925839fadc067e0e03c42059ae9a978cb4e22` |
| `docs/plan/ELINTP-VISUALS-P3-gaps.md` | `85bd80b3bcb13a7f1b8644100fefef865c9cbec0cd6aaac0cc90af66bf726ab9` |
| `skills/elintp/SKILL.md` | `6811bf071b8d10f356a1d5108c220c3de9ed43fc670b6466c5810ab07d4e57bd` |
| `skills/elintp/assets/page.css` | `f75499305ad2aef60fcd61f79395a6e45e3ea9cac2d6b090c900e30493551d09` |
| `skills/elintp/references/draft-format.md` | `3bfda8442ce433f154075b03993cfcc4cf068adef31cdfba4ed222f8a938059b` |
| `skills/elintp/scripts/draft.mjs` | `de29f49f77464aae7417f66166b914a06f2e634734ef3d95ce7eee60384684f5` |
| `skills/elintp/scripts/render.mjs` | `543e4e89b032e2a4af07457843a2ec2cfa9f9231b6462181201707bcff8f31f6` |
| `skills/phoenix-diagram/references/delivery-contract.md` | `1ad889a9be6f71469f04a065ddbdb587062a1930e6d96771d2c37a5c2102cc00` |
| `skills/phoenix-diagram/references/static-figure.md` | `4b6dbc3eb96ea8eb34a613f3437927217dd53628d33a06ef9f663a29b33ffa6a` |
| `skills/use-llm-gateway/SKILL.md` | `6228ab041474bd773f4e361413f487ae0723c62ee9a42e575576b1ab21e3d123` |
| `skills/use-llm-gateway/references/api.md` | `4f0307ca69a496bab3f47a478a0cd7d23eb60cf7136d9e782e8abb7213174547` |
| `skills/use-llm-gateway/references/recovery.md` | `7be20ac687f0d10c00d07b6f75f3c4cd714de2b0db8332b40405a73774debc41` |
| `skills/use-llm-gateway/references/self-hosted.md` | `dc94e696ec29a1cb0a95f6d333ce5a70310321b3fc57b087b2169da33cf1e40e` |
| `test/elintp/render.test.js` | `d4a455df438b8f29e5a884f97bd5b7a9db351e53d32f219d11382ac279ab4d06` |
| `tools/phoenix-diagram/src/layout/statechart.rs` | `c0d737aecc1ff9103d0eaaefd365911b1a3736bfdb143c3da8c60dc29805f719` |
| `tools/phoenix-diagram/src/render.rs` | `d73aceb873383f051d20db7f4b9486ff7b2ac369594f57e5b056da5db21e0e92` |
| `tools/phoenix-diagram/src/render/static_style.rs` | `2b7b3b6b19740a8236c444ece77f5621499f97f6eb38137ccf2d1f5b02ec4994` |

</details>

## 4. 计划符合性

| 计划承诺 | 实现与验收依据 | 结论 |
| --- | --- | --- |
| M1 / WORK-F9 AC-1：Reader 全部卡片、原顺序随 SVG 交付 | `write_common` 取 `scene.cards` 的 dot/title/items；共用输入 schema；Reader 使用同一字段；两策略 statechart 数据比对、Workflow 当前 CLI 与页面 | 已核实 |
| M1 / WORK-F5 AC-2、C-1：安全内联，卡片在图与图注之间，单文件零网络 | `inlineSVG` 校验根直属、唯一 metadata、唯一空属性、JSON 精确键与字段类型；输出 `escapeHTML`；19 项测试及当前离线浏览器实测 | 已核实 |
| M1 / C-1：360 px、亮暗、打印可读 | 卡片 grid 与页面变量；当前截图、DOM 度量与打印 PDF | 已核实 |
| M1 / C-2、WORK-N3 AC-3：没有卡片数据时仍可用 | metadata 可选；CLI 用例包含无卡片静态 SVG；十一类既有产物与 M1 旧图兼容记录 | 已核实 |
| M2 / C-3、C-4：文档及 bundle 一致；Reader 字节不变 | 全部相关复述 diff；38 项 bundle 内容核对；11/11 Reader SHA | 已核实 |
| M2 / V-3、V-4：收口与真实入口 | 复用未失效 M2 记录；本轮再次从真实 static/render 入口生成 Workflow 页面并查看浏览器与打印产物 | 本计划代码验收证据足够；不将本轮评审冒称新的作者角色模拟 |
| 跨期终验与排除项 | 计划顶部需求来源与 §0.5 | WORK-F5、WORK-F9、WORK-N3 整体终验仍归 P5；不以本轮代码验收登记为已上线 |

## 5. 总体结论与修复顺序

验收判定：**通过验收**。

正确性结论：`patch is correct`，覆盖本报告声明的计划 diff 与未提交文档审查范围。未发现 P0–P3 问题；卡片的同源内容、安全解析、旧图兼容与当前真实页面结果均有证据支持。本轮无需整改；该结论不代表合并、发布或跨期 PRD 整体验收授权。

本计划代码验收完成。可选后续独立验收：

```text
$review-ui docs/plan/ELINTP-VISUALS-P3-gaps.md
$review-prd-dev-gaps docs/plan/ELINTP-VISUALS-P3-gaps.md
```

第一条用于正式 UI 验收（本轮浏览器检查不替代其独立报告）；第二条适用于本计划顶部明确登记的 PRD 来源，均不自动启动。
