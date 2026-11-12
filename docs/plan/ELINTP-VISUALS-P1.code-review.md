# ELINTP-VISUALS-P1 · 代码变更评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [ELINTP-VISUALS-P1](ELINTP-VISUALS-P1.md)，SHA-256 `bcc6381c7d5f652dd2eeba22f8c716787656b699f959fbddc38dcafb348eb015` | `0b35470b506d5dfd0efb34f032ab43ce70f7d26e..55fe9a265d607676d561506b81aff7580886eae8`；另核对 HEAD `3956733678ece9754614d6f5a900993d42dee9ca` 的未提交快照，指纹 `4baa24754ec1c3e8d62dcf324fae1f9921178b30821fc335958036acc0bdb980` | 无；既有 `.review.md` 是开发计划评审，不是代码评审前轮 |

## 1. 确认问题

### F-01 · [P2] 隔离各内联 SVG 的 ID 与本地引用

- 定位：`skills/elintp/scripts/draft.mjs:257-258`（target `55fe9a265d607676d561506b81aff7580886eae8`）。
- 问题：同一内容稿放入两张各自合法、但重复使用常见 `id` 的 SVG 时，脚本原样内联所有 ID，`href="#…"`、`xlink:href` 与 `url(#…)` 因此在整个 HTML 文档中发生跨图解析；后续图会使用前一张图的同名定义，改变形状、颜色、标记或裁切结果。基线没有该渲染器，本变更新增多图内联能力并引入此冲突。
- 证据：计划 §5.4 允许 `svg:` 图及上述本地引用（`docs/plan/ELINTP-VISUALS-P1.md:450-457`）。本轮用两张 SVG 分别定义同名 `id="shape"`：第一张是红色方块，第二张是绿色三角形；公开 `render.mjs` 入口 exit 0 后，Chrome 实际显示两张均为红色方块，第二张未显示自己的绿色三角形。实现侧 `draft.mjs:75` 原样输出 ID/引用，`:257-258` 直接拼接各图，没有每图命名空间。
- 修复与验证：按图为 ID 分配确定性的唯一前缀，并同步重写 `id`、`href`/`xlink:href`、所有 `url(#…)` 及 ARIA ID 引用；同一 SVG 内重复 ID 应报格式错误。增加两张 SVG 使用相同 ID、不同图形/渐变/标记的公开入口回归，并在浏览器确认第二张仍显示自己的图形，同时复跑 `node --test test/elintp/render.test.js`。
- 置信度：高；公开入口和真实浏览器均已复现，输入不依赖异常时序或外部环境。

## 2. 待核实 / 待决策项

无额外阻塞性待核实项。F-01 的修复方案只需保持现有 SVG 白名单与确定性合同，不需要改变产品范围。

## 3. 覆盖与验证

### 意图与冻结范围

一句话意图：本期让 `elintp` 通过受限内容稿和随 skill 分发的 Node 脚本，生成四种固定模板、组件完整、可离线阅读的单文件 HTML。用户显式要求是审查该计划对应实现；上述行为细节来自计划与 PRD，而非假定作者另有未表达意图。

- 仓库根：`/Users/rockie/Documents/gh-xgent/skills`；评审日期：2026-11-11。
- 原输入：`docs/plan/ELINTP-VISUALS-P1.md`，采用开发计划模式。
- 登记实现提交：`23c6748b857c410682a7534b92080f338d4e4310`；登记最终验收提交：`55fe9a265d607676d561506b81aff7580886eae8`（仅文档）。两者均在当前 `main` 上。
- 净差异比较：最早登记提交的第一父 `0b35470b506d5dfd0efb34f032ab43ce70f7d26e` → `55fe9a265d607676d561506b81aff7580886eae8`。`249f754b`、`e069505e` 只回写计划，`39567336` 只回写状态，不作为代码差异混入结论。
- 当前 `skills/elintp/` 与 `test/elintp/` 相对 target 无差异；关键文件 SHA-256 在评审前后保持一致，包括 `draft.mjs` `78ef66b0d594933f8db80d3e4954606b8fa647dc78cc5e624912ef73e941fd5a`。
- index 为空；tracked 工作树 diff SHA-256 `2d90af3e29849dff3aae1c19d443d319ef9a58a2bc618a2a1a18e308a2a38499`，空 index diff SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。完整未提交指纹见「评审对象」。

### 文件与行为覆盖

- 完整读取净差异及当前目标文件：`SKILL.md`、`scripts/draft.mjs`、`scripts/render.mjs`、`scripts/templates.mjs`、`references/draft-format.md`、三个 `assets/` 文件和 `test/elintp/render.test.js`。
- 追踪了 frontmatter/slot/组件解析 → 错误聚合与退出码 → 模板拼装/旧页保护；文字/链接/SVG 安全边界；图片 data URI、阅读器 `srcdoc` 与 SVG 内联；主题、释义、大纲、键盘焦点、窄屏和打印；公开 skill 流程、无 Node 降级及分发文件。
- 当前 dirty 内容已枚举并读取：tracked 的 `AGENTS.md`、`docs/ai-native-sdlc.md`、经验/状态索引，以及 untracked 的 PRD、决定/评审文档、旧 HTML、计划评审和状态页。它们没有未提交的可执行代码改动；计划、记录、评审与状态文档不作为代码 finding 对象，PRD 用作需求合同。
- 主 Agent 独立评审，未使用并行 reviewer，未修改实现、测试或计划。

### 实际验证

| 类型 | 命令 / 证据 | 目标版本与结果 |
| --- | --- | --- |
| 现有回归 | `node --test test/elintp/render.test.js` | 当前文件与 target 相同；Node `v25.2.1`，4 pass、0 fail。覆盖格式错误/旧页保护、非法 progress、SVG 常见注入与外部资源拒绝、组件/嵌入/确定性。通过不覆盖 F-01 的跨图 ID 冲突。 |
| 公开入口最小复现 | 在隔离目录以两张同名 ID、不同图形的 SVG 运行 `node …/skills/elintp/scripts/render.mjs duplicate-ids.md duplicate-ids.html` | exit 0，输出 `PAGE:`；输入、SVG 与产物均在 `/tmp/elintp-p1-review.lgk9vH/`，未写入仓库。 |
| 真实浏览器复现 | Chrome headless 打开上述输出并截图 | 第一张为红色方块；第二张也错误显示红色方块，应为绿色三角形。截图 SHA-256 `18eca43d479d90155a97839056abf57a7edd39f468295632527ae02d1acd8faa`。 |
| 当前版本真实使用证据 | [M5](ELINTP-VISUALS-P1.records/M5.md)、[M6](ELINTP-VISUALS-P1.records/M6.md) | 记录基于实现提交 `23c6748b`，其后无源码变化；干净副本经公开命令生成并改写页面，Chrome 完成通读、主题与大纲检查。该证据支持主路径已走通，但未覆盖同名 SVG ID，不能否定 F-01。 |

未复跑完整 V-1/V-2/V-4/V-5、`npm pack`、全仓测试或 Phoenix 运行时；这些已有同版本记录或不直接覆盖本次缺陷。F-01 的浏览器复现是对记录证据缺口的定向补查，不以局部测试代替需求核对。

## 4. 计划符合性

| 计划承诺 | 实现与验证 | 判定 |
| --- | --- | --- |
| M1–M4：受限内容稿、四模板、组件、格式错误与旧页保护 | `draft.mjs`、`render.mjs`、`templates.mjs` 与测试；现有测试通过 | 已核实，除 F-01 的多图正确性 |
| WORK-F1 / WORK-F9 / WORK-N2：离线图片、阅读器、SVG 白名单与文字转义 | data URI、`srcdoc`、SVG 元素/属性/引用白名单；安全测试与 M4 离线记录 | 已核实；F-01 是引用隔离错误，不是已证实脚本执行 |
| WORK-F3 / WORK-N1：主题、大纲、键盘、窄屏、无脚本与打印 | `page.css`、`page.js`；M4 当前版本 Chrome 与 PDF 记录 | 已核实到现有证据范围 |
| M5：公开 skill、内容稿参考、无 Node 降级与作者行为 | `SKILL.md`、`draft-format.md`；M5 六个全新上下文演练 | 已核实到记录范围 |
| M6：干净副本真实使用、邻接面不变、分发与旧页 | M6 记录；当前源码指纹与其基线一致 | 已核实到记录范围；不扩为 P2–P5 终验 |
| §4 / §5.4：每个 `svg:` 图正确内联 | 两张独立 SVG 同名 ID 时发生跨图引用 | 存在 F-01 |

本期未要求修改 Phoenix 运行时，实际也未修改；P2–P5 和 WORK-N3 后续工作不作为本期缺陷。

## 5. 总体结论与修复顺序

**验收判定：需要修改。**

`patch is incorrect`。F-01 使合法多图输入产生错误图形，直接破坏图文说明所传达的事实；其余审查范围未发现另一项达到门槛的缺陷，当前版本已有真实入口证据，但不能覆盖该冲突。

先修复 F-01 的每图 ID/引用隔离，再增加双图同名 ID 回归并运行现有 4 项测试与浏览器场景。可观察的解除条件是：第二张图显示自己的图形，所有本地引用仍指向本图定义，重复运行输出保持逐字节一致。本结论不是合并或发布授权。
