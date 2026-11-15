# ELINTP-VISUALS-P4 代码评审报告（R2 · 修复效果复审）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2（修复效果复审） | [ELINTP-VISUALS-P4 · 数值图表（Phoenix Diagram 新增 chart 图型 · 柱、折线、饼、限额四种形态 · 数据表随图交付）](ELINTP-VISUALS-P4.md)，当前版本 `sha256:40196c2a…` | 上轮目标 `8f382080dc6a084d34d021c2c470427d80e378ce` → 本轮目标 HEAD `2beb7e0d55055fcde82c6ec8ce35247e3ed8606e`；代码净差异只在修复提交 `3f38d05350b389c7b0b6f4b45869b5482558ccb8`（登记为 M1、M2 的「修复」），其余提交与 `3f38d053` 的计划部分只改 `docs/plan/ELINTP-VISUALS-P4*` | [R1 报告](ELINTP-VISUALS-P4.code-review.md)（`sha256:e30dd325…`，审 `3889a9ae..8f382080`） |

- **复审类型**：修复效果复审（不是完整重审）。上轮覆盖的 5 条问题逐条复核；新增问题只查修复 diff 及其直接激活的行为，不把上轮未审的范围当作已审。
- **处置记录**：[M1 记录 · 评审修复](ELINTP-VISUALS-P4.records/M1.md#评审修复)（A1–A3 验收与「未完项」）、[M2 记录 · 评审修复](ELINTP-VISUALS-P4.records/M2.md#评审修复)。
- **版本链**：上轮目标 `8f382080` → 计划回填提交 `02623508`（只改计划）→ 修复提交 `3f38d053`（父提交 `02623508`）→ 计划回填提交 `2beb7e0d`（只改计划）。`git rev-list --parents` 与 `git show --stat` 已确认。
- **工作树**：`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp`、`test`、`scripts`、`dist`、`bin`、`README.md`、`docs/phoenix-diagram.md` 无 tracked 差异、无 untracked 文件。`skills/use-llm-gateway/**`、`.xgent-ai/**` 与既有评审文档的未提交改动属于其他范围。

## 上轮问题复审

| 来源报告 / F-ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01 · P1 | 饼图顺序说明不参与画布宽度，en/es 的静态图与阅读器都被裁切 | 已修：`layout/chart.rs:614-620` 把「边距 2 倍 + 说明估计宽」并入画布宽；`layout/chart.rs:750-756` 复用同一字符串 | **已解决** | 3 行短标签饼图 en/es/ko/zh-CN/zh-TW 与 2 行 en/es/ko/zh-CN/zh-TW、8 行 en：`static` 与 `browser-check` 全部 exit 0（上轮 es 3 行 `static` 退 1、`browser-check` 退 1；现均通过）。单行 es 饼图 `static` 也通过（其阅读器失败属 F-06） | 无 |
| R1 / F-02 · P1 | 竖向轴把画布宽度当作刻度像素长度，宽图刻度 7.7–8.3 px 重叠且合法图被检查拒绝 | 已修：`layout/chart.rs:325`、`:974` 的槽宽下限 + `:329`、`:978` 改用 `PLOT_H`；`check/chart.rs:221-229`、`:336-350` 改用最远刻度定比例 | **已解决** | 6 行 ×3 系列（100000–305000 ms）：5 个刻度、间距 60 px；20 点折线（100–803）：6 个刻度、间距 48 px（上轮 32 个/7.74 与 30 个/8.28）。两份 `deliver`/`static`/`browser-check` 全部 exit 0；新增用例 `wide_and_mixed_magnitude_charts_keep_spaced_ticks_and_pass_the_artifact_check` 通过，断言间距 ≥ 40 | 无 |
| R1 / F-03 · P2 | 1 px 最短柱被内容检查当成错位，1 vs 1000 与 0.334971 被拒 | 部分修：`check/chart.rs:302` 按 `max(1 px, 精确长度)` 求柱外端；`layout/chart.rs:1068` 竖向柱以零线为锚点延长 | **部分解决** | 通过：竖向 1 vs 1000、竖向 0.334971 vs 1000、横向 1 vs 1000（上轮三例均退 1）。仍失败：横向负值小量级 `[-0.334971, 1000]`、`[-0.5, 1000]`、`[-1.0, 1000]`、`[-1.786, 1000]`、`[-1.8, 1000]`、`[-2.0, 1000]` 全部 `deliver` 退 1，报 `A bar is not placed on the shared linear scale from zero`；`[-5.0, 1000]` 通过。原因：`layout/chart.rs:1257-1258` 负值柱以 `x_at(v)` 为起点、长度 `max(1.0)`，被截短时柱从零线向**正方向**延伸（`-2.0` 实测 zero 292.53、柱 291.6..292.6），与 `check/chart.rs:302` 期望的 `零线 − 1` 相差 0.07–0.9 px，两侧都超过 0.05 容差 | 把负值横向柱锚在 `zero_x - len`（标注同步用 `zero_x - len - 4`），补横向负值最短柱用例；修后断言四个方向的最短柱都通过、且 4 px 错位仍报错 |
| R1 / F-04 · P2 | 横向柱图的 0 值标注高出所属行，静态图被节点裁切检查拒绝 | 已修：`layout/chart.rs:1243-1250` 改为 `zero_x + 4`、`center_y + 3.8`、`start`，与其它标注同基线 | **已解决** | 含 0 的横向柱图 `static`、`browser-check` 均 exit 0；SVG 中 0 与 42 的基线分别为 70.8 与 106.8，行距 36 与其它标注一致（上轮 `static` 退 1 报 `nodeTextClips`）。全负值 + 0 行的横向柱图同样通过 | 无 |
| R1 / F-05 · P3 | 测试名仍写「eleven types」 | 已修：`tests/cli.rs:128` 改名 twelve | **已解决** | `--test cli` 11/11 通过，测试名与十二类一致 | 无 |

**汇总**

- **旧问题解决情况**：F-01、F-02、F-04、F-05 按原关闭判据解决（4/5）；F-03 部分解决（竖向两例与横向正值通过，横向负值仍失败）。
- **修复引入的新问题**：未发现。修复 diff 只动 `layout/chart.rs`、`check/chart.rs`、`tests/chart.rs`、`tests/cli.rs` 与计划文档；chart 之外的图型不经过这些文件，`check_anchors` 13/13、`input` 26/26 全过。饼图加宽与竖向槽宽下限在 1/2/3/6/8/20 行、5 种语言下未观察到新失败（20 行超宽属计划 D5 已接受的可读性失败）。
- **未覆盖 / 缺证**：十一类静态 SVG 与基线的逐字节比对本轮未重做（修复只触 chart 专属文件，依据见覆盖节）；F-06 的共用阅读器根因未定位（见待核实）。

## 确认问题

### F-06 · [P2] 单行图表的阅读器交付全部失败，计划记录的 V-N2 仍未处置

- 定位：`tools/phoenix-diagram/viewer/reader-layout.js:247-262`（`belowFold`）与 `:313-327`（`settleOverflow` 声明溢出）、`tools/phoenix-diagram/src/browser/probe.js:302`（`excludedNotesHeight` 扣减）、`tools/phoenix-diagram/src/browser/observation.rs:88-101`（`verticalScrollAccepted` 判定）。这三处本计划从未修改（`git log 3889a9ae..HEAD --` 为空），因此问题的落点在共用阅读器/探针，而新图型的单行输入把它激活；计划的处置记录见 [M1 记录 · 未完项](ELINTP-VISUALS-P4.records/M1.md#未完项评审修复) 与 [恢复快照](ELINTP-VISUALS-P4.md) 「当前阻塞：V-N2 待用户决定」。
- 问题：`rows` 只有一行的 `chart`（四种形态都算）能通过 `deliver` 与 `static`，但 `finalize` 与 `browser-check` 在 1440×900、1600×1000、1920×1080、2048×1320 四个桌面视口 × 亮暗两色全部失败，返回 `browser/viewport-containment`、`deliveryStatus: incomplete`，只有 390×844 通过：用户拿不到可交付的阅读器 HTML，而计划 §5.4 L514 明确把「只有一行时没有线段」写进折线形态，schema 的 `rows` 也是 `minItems: 1`。计划把该现象记为 V-N2「待用户决定（修复或确认延期）」，既未修复也没有用户豁免，按本期缺陷报告。
- 证据：单行 candidate `{"form":"bar","rows":[{"id":"a","label":"Throughput","values":[42]}]}`、单行折线、单行饼（es）、单行限额、以及一行两系列的「Before vs after」柱图，各跑 `deliver`（0）、`static`（0）、`browser-check`（1）、`finalize`（1，`deliveryStatus: incomplete`）。失败测量的关键值：`overflowY: true`、`overflowX: false`、`verticalScrollAccepted: false`、`readerFlow.unclipped: true`、`notesBelowDiagram: true`、`excludedHeight: 55`、cards 顶部 884 高 39（视口高 900）。对照：单节点 `tree` 的 `browser-check` 通过，两行竖向柱、三行折线均通过，说明不是「画布偏短」的通用问题。受影响范围比计划记录更宽——记录只写「竖向柱/折线」，实测单行饼与单行限额同样失败。
- 修复与验证：处置方向沿用计划已经列出的三选一：(a) 统一阅读器 `belowFold` 与探针 `excludedHeight` 的图下说明区口径，并按 V-5 回归十一类；(b) schema 要求竖向柱/折线至少 2 行，同步 `chart.md`、schema 与错误信息；(c) 用户确认延期到 P5。修后对四种形态各一份单行 candidate 断言 `finalize` 退出 0，并用该批加两份多行样本覆盖四个桌面视口与 390×844。
- 置信度：高（真实命令入口 + 失败测量 + 计划记录的同一现象；根因在阅读器/探针内部的具体分支未定位，见待核实）。

## 待核实 / 待决策

1. **F-06 的根因未定位到具体分支**（执行者：apply-code-review）。缺什么：为什么单行时页面高度超过视口、阅读器却没有声明 `data-reader-overflow`。已尝试：用 headless Chrome 在 1440×900（`--window-size=1440,987`）导出 `bar1`/`bar2`/`limits1`/`pie1-es` 的 `data-reader-*`、`scrollHeight`、cards 位置。受阻原因：headless 结果与探针不一致（headless 下连通过的两行样本也报 999–1044 的文档高度、`data-reader-overflow` 为空），无法据此判定通过/失败的边界。补证动作：用同一个 CDP 视口与探针交互序列（或临时在 `settleOverflow` 打印溢出预算）对比两份 HTML。完成条件：给出「阅读器认为已适配、探针认为溢出」的具体分支与改动面。
2. **F-06 的处置方向需用户选择**（执行者：用户）。输入是 (a)/(b)/(c) 中的一项；未决定前计划保持「阻塞」，本期不能判通过。

## 覆盖与验证

- 意图：给 Phoenix Diagram 增加 `chart` 图型（柱、折线、饼、限额）与随图数据表，并让 elintp 渲染静态图数据表；本轮只核对 R1 问题的修复效果与修复 diff 的直接后果。
- 仓库根：`/Users/rockie/Documents/gh-xgent/skills`；评审日期 2026-11-15。
- 审查方式：`git diff --no-ext-diff --no-textconv 8f382080 2beb7e0d` 的净差异，逐个读取 `layout/chart.rs`、`check/chart.rs`、`tests/chart.rs`、`tests/cli.rs` 的改动与周边上下文，并对照计划 §5.4、`chart.schema.json`、记录中的处置结论。
- 工具链：`mise`（rust 1.99.1）+ `mbx 1.28.0`；开发二进制 `tools/phoenix-diagram/target/debug/phoenix-diagram` 由 `mbx build --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline` 确认与当前源码一致（`Finished dev profile` 无编译）。所有 candidate、产物与截图目录在仓库外的 `/tmp/p4r2`，未写入目标仓。
- 实际运行（目标版本 `2beb7e0d`，代码等于 `3f38d053`）：
  - `mbx test --test chart --test cli` → 15/15、11/11 通过；`--test check_anchors --test input` → 13/13、26/26 通过（全新增 chart 用例与 chart 最小示例的锚点均在修复后通过）。
  - 真实入口矩阵：34 份多行 chart candidate 中 27 份 `deliver` 通过（7 份失败：20 行柱图 1 份归计划 D5 已接受的「画布过宽触发可读性检查」，6 份横向负值小量级归 F-03 残留）；这 27 份的 `browser-check` 全部通过、`static` 26 份通过（`barh-neg-5.0` 未跑 `static`）。5 份单行 candidate 的 `deliver`/`static` 全部通过、`browser-check` 与 `finalize` 全部失败（F-06）。另有 3 份既有类型/反向对照：单节点 `tree` 通过；一份 `timeline` candidate 是我写错的非法输入（schema 拒绝，未纳入结论）。
  - `static` 与 `browser-check` 的具体失败/通过判据取引擎自身输出：`static` 的 `nodeTextClips`/`canvasTextClips`、`browser-check` 的 `viewport-containment` 等检查项与实际证据，不用主观视觉判断。
  - 负例反证：把 `bar-1-1000` 的 HTML 最短柱从 1 px 改成 20 px 后跑 `check`，仍报 `A bar is not placed on the shared linear scale from zero`（退出 1），说明 F-03 的放宽没有让真错位漏过；原始 HTML 的 `check` 退出 0。
- 未覆盖：十一类静态 SVG 与基线的逐字节比对本轮未重做。依据：修复 diff 只改 `layout/chart.rs`、`check/chart.rs` 与两份测试（chart 专属分派），`check_anchors` 与 `input` 全过；如需独立证明，可在基线快照上补跑。
- 本轮未重做：计划 V-6 的离线安装与 elintp 讲页验收（修复不涉及 elintp 与安装链路）、V-5 全量 `npm run test:diagram`、人眼看图。视觉质量结论以引擎检查为准，不作主观评价。

## 计划符合性

| 需求/约束 | 计划落点 | 本轮核实 | 状态 |
| --- | --- | --- | --- |
| M1 修复：F-02（竖向柱）、F-03、F-05 | 完成记录 M1「修复 3f38d053」；[M1 记录](ELINTP-VISUALS-P4.records/M1.md#评审修复) | F-02、F-05 已解决；F-03 只覆盖竖向与横向正值 | 部分成立：F-03 残留（横向负值最短柱） |
| M2 修复：F-01、F-02（折线）、V-N1（折线）、V-N2 | 完成记录 M2「修复 3f38d053」；[M2 记录](ELINTP-VISUALS-P4.records/M2.md#评审修复) | F-01、F-02 折线已解决；V-N1 折线行样本通过；V-N2 仍待决策且实测范围更宽 | 缺口：V-N2 未处置（F-06） |
| §5.4 L500「绘图区高 240…最小刻度间距 40」 | §5.4 | 宽柱 60 px、20 点折线 48 px 间距，窄图 48 px | 成立 |
| §5.4 L501「数值不为 0 时柱长至少 1」 | §5.4；D5 | 竖向两例与横向正值成立；横向负值仍被拒，且被截短的柱画过零线 | 存在缺陷：F-03 残留 |
| §5.4 L514「只有一行时没有线段」（单行输入合法） | §5.4；schema `rows.minItems: 1` | 单行四种形态的阅读器交付全部失败 | 存在缺陷：F-06 |
| §5.4 L523 饼图说明随图交付 | §5.4；V-2 | 5 种语言、1/2/3/8 行饼图 `static` 通过，阅读器除单行外通过 | 成立（单行问题归 F-06） |
| §5.4 L499 竖向槽宽规则 | §5.4 | 实现新增「槽宽不低于 `PLOT_H / 行数`」下限（`layout/chart.rs:325`），计划正文未同步 | 轻微缺口：计划正文与实现不一致，按计划 `@progress_writeback` 应就地修订 |
| V-5 十一类结果不变 | 完成记录 M5 | 修复只改 chart 专属文件；`check_anchors`、`input` 全过 | 成立（未重做逐字节比对） |
| 恢复快照「5/5 完成 + 当前阻塞 V-N2」 | 恢复快照 | 与 F-06 一致：里程碑交付完成，但存在未处置的用户可见缺陷 | 快照如实；「全部里程碑已完成」不构成验收通过 |

## 总体结论与修复顺序

**验收判定：需要修改。**

修复效果复审结论：**未完全解决**（F-01、F-02、F-04、F-05 已解决；F-03 部分解决）。

正确性结论：**patch is incorrect**（只覆盖修复 diff `8f382080..2beb7e0d` 的代码部分）。F-03 的修复在 `check/chart.rs:302` 补上了最短柱的期望外端，但横向负值分支（`layout/chart.rs:1257-1258`）仍以 `x_at(v)` 为起点，被截短的柱向正方向越过零线，同一个检查因此继续拒绝合法图；修复的用例清单（`tests/chart.rs:265-268`）也只有竖向 1.0、竖向 0.334971、横向正值 1.0，没有覆盖该分支。F-06 不是修复提交引入，但它是计划记录在案、未修复也未经用户豁免的用户可见缺陷，阻断本期验收。

理由与覆盖限制：39 份合法 chart candidate（34 份多行 + 5 份单行）的实际交付链路确认 F-01、F-02、F-04、F-05 已关闭，修复没有引入新问题；剩下两处都是「合法输入被拒」的功能缺陷，其中 F-06 需要用户先在计划列出的三个方向中定一个。

修复顺序与解除条件：

1. 先修 F-03 残留：负值横向柱锚在零线外侧，补横向负值最短柱用例；解除条件为四个方向的最短柱 `deliver`/`static` 通过，且 4 px 错位仍报错。
2. 再处置 F-06：用户先选 (a) 统一阅读器/探针口径、(b) schema 要求 ≥2 行、(c) 确认延期；选 (a) 要按 V-5 回归十一类，选 (b) 要同步 `chart.md` 与 schema 并使错误指向 `rows`。
3. 两项处理后按原范围复审（以本报告为输入），确认 `finalize` 在单行四种形态退出 0。

本结论只覆盖 `8f382080..2beb7e0d` 的代码与当前计划版本，不是合并、发布或需求验收授权。
