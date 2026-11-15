# ELINTP-VISUALS-P4 代码评审报告

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1（初审） | [ELINTP-VISUALS-P4 · 数值图表（Phoenix Diagram 新增 chart 图型）](ELINTP-VISUALS-P4.md)，工作树版本（`sha256:efba608de980856c346085c46e2a9897cd6ecbb02a72ff307c401ef7d7d0fc50`，与 HEAD `02623508` 一致） | 基线 `3889a9ae16396d710f637583721b12a73d3eb897`（`1d9e3f57^`）→ 目标 `8f382080dc6a084d34d021c2c470427d80e378ce`；登记提交 `1d9e3f57`、`1310cb63`、`5f40e694`、`75d5823e`、`cf55f536`、`14d598ba`、`8f382080` | 无 |

- 区间 `1d9e3f57^..8f382080` 内其余提交都只改 `docs/plan/ELINTP-VISUALS-P4*` 或 `.xgent-ai/`（进度回写），未纳入代码审查；HEAD `02623508` 只回填计划里的一行 SHA，也在区间之外。
- 工作树：`tools/phoenix-diagram`、`skills/phoenix-diagram`、`skills/elintp`、`test`、`scripts`、`dist`、`bin`、`README.md`、`docs/phoenix-diagram.md` 无 tracked 差异、无 untracked 文件（`git diff HEAD` 与 `git ls-files --others` 均为空）。`skills/use-llm-gateway/**`、`.xgent-ai/**` 与既有评审报告的未提交改动不属于本计划范围。
- 本计划没有更早的代码评审报告；P3、P3-gaps 的 `.code-review.md` 审的是前一期计划的提交，不作为本计划上轮。计划中的「修复」提交 `14d598ba` 已包含在上述目标内。

## 确认问题

### F-01 · [P1] 饼图顺序说明不参与画布宽度，默认语言下阅读器与静态图都被裁切

- 定位：`tools/phoenix-diagram/src/layout/chart.rs:611-615`（另见 `:741-751`，target 侧 `8f382080`）
- 问题：饼图 `width` 只取「列表右缘」与「环形图右缘」的较大值，`chart.pie.order` 说明固定从 `x=28` 起画，既不参与宽度计算也没有换行或截断。英文文案在 10.5 px 下实测 441 px、西语 523 px，而两三个短标签份额的画布只有 396–428 px，说明右端直接落到 viewBox 外：`static` 的浏览器阶段判 `outside-svg`，阅读器 `browser-check` 判 `browser/text-containment`，默认语言（en）下常见饼图因此无法完成交付；阅读器里同一句被容器裁掉，用户看到半句。
- 证据：
  - 输入 `{"schema_version":1,"diagram_type":"chart","meta":{"title":"Pie","evidence":"sourced"},"form":"pie","rows":[{"id":"r0","label":"A","values":[3]},{"id":"r1","label":"B","values":[4]},{"id":"r2","label":"C","values":[5]}]}`。
  - `static chart <candidate> out.svg --json` 退出 1：`browser/rendered-dom`，viewBox `428×312`，`canvasTextClips: [{"text":"Sectors run clockwise from 12 o'clock, in the same order as this list.","failures":[{"reason":"outside-svg"}]}]`。
  - 同一 candidate 的 `deliver` HTML 再跑 `browser-check` 退出 1：`browser/text-containment`，svgRect `642×468`、viewBox `428×312`（说明文字随 viewBox 放大到约 661 px）。
  - 同一输入 zh-CN、ko 通过（说明 25/41 字符），es 同样失败（523 px）。计划 [§5.4 L523](ELINTP-VISUALS-P4.md) 只要求「环形图下方一行说明」，未把说明宽度纳入画布；引擎自身的浏览器契约要求文字在画布内（`tools/phoenix-diagram/src/browser/probe.js:213-222`）。
  - 计划 M2/V-2 的「四形态双策略静态图 complete」用的是 zh 四项份额样例，未覆盖 en/es 与窄列表。
- 修复与验证：把说明文字宽度计入 `width`（拉丁文案的 `text_units` 估计明显偏小），必要时按语言换行或缩短；修后对「2–3 个短标签份额」的饼图在 en、es、zh 各取一份 `static`，并对其 `deliver` HTML 跑 `browser-check`，断言自动阶段 pass。
- 置信度：高（两条命令的实际失败证据 + 同输入跨语言对照）。

### F-02 · [P1] 竖向轴的刻度步长按画布宽度计算，宽图刻度挤在一起且合法图过不了检查

- 定位：`tools/phoenix-diagram/src/layout/chart.rs:973`（竖向柱状图）、`tools/phoenix-diagram/src/layout/chart.rs:327`（折线图，同一根因）
- 问题：两处纵向数值轴都把 `plot_w`（画布宽度，宽图可达 1300 以上）当作刻度像素长度传给 `axis_scale`，应传绘图区高 `PLOT_H`（240）。宽图上刻度数可达 30 以上：刻度线间距 7.7–8.3 px，而刻度文字 10.5 px，纵向重叠；同时内容检查从「第一个非零刻度」反推比例，0.01 的坐标舍入被刻度比值（约 31）放大到约 0.15 px，超过 0.05 容差，`deliver` 与 `static` 报 `A tick is not on the shared linear scale` / `A bar is not placed on the shared linear scale from zero` 并拒绝合法图。
- 证据：
  - 6 行 × 3 系列、数值 100000–305000（单位 ms）：`validate --layout-json` 显示 32 个 `chart-tick`，y 间距 7.74；`deliver` 与 `static` 均退出 1，诊断含刻度 290000/300000/310000 与柱 300000（`outer: 51.8`）。
  - 20 点折线（数值 100–803）：30 个刻度、间距 8.28，`deliver` 退出 1（`value: 433, expected: 173.71`）。
  - 计划 [§5.4 L500](ELINTP-VISUALS-P4.md) 要求「绘图区高 240…最小刻度间距 40」；随仓库交付的最小示例已是 10 个刻度、间距 26.67，还没到硬失败，但已低于该承诺。
  - 横向柱状图 `HORIZONTAL_AXIS_W`/80 的写法是正确的，说明这两处是轴向取错。
- 修复与验证：两处改用 `PLOT_H`；补宽图用例（6×3 柱、20 点折线）在 `deliver` 与 `static` 的自动阶段通过，并断言刻度间距 ≥ 40。
- 置信度：高（布局 JSON 与两条命令的实际失败都可复现）。

### F-03 · [P2] 计划要求的 1 px 最短柱被内容检查当成错误定位，合法柱图交付失败

- 定位：`tools/phoenix-diagram/src/check/chart.rs:302`；对照 `tools/phoenix-diagram/src/layout/chart.rs:1063`、`:1249-1252`
- 问题：计划 [L501](ELINTP-VISUALS-P4.md) 要求「数值不为 0 时柱长至少 1」，布局按此用 `max(1.0)`；内容检查却要求柱外端与精确比例位置相差 ≤ 0.05 px。于是当 `|v| < 轴跨度 × 0.95 / 240`（横向为 `/560`）时必然失败：1 与 1000 的竖向柱图 `deliver` 与 `static` 都退出 1，报 `A bar is not placed on the shared linear scale from zero`，用户无法从错误信息知道是量级差造成的，也没有可照着做的修法。混合量级的随机样本里这是最常见的失败（9 次失败中 5 次）。
- 证据：
  - 输入 `{"form":"bar","rows":[{"id":"a","label":"Almost none","values":[1]},{"id":"b","label":"All of it","values":[1000]}]}`。
  - `deliver` 退出 1，诊断 `artifact/semantic-relation`，证据 `{"value":1.0,"zero":284.0,"outer":283.76}`；`static` 同样退出 1。
  - 0.334971 与大跨度、19.14 与 −51 等样本报同一诊断。
- 修复与验证：让检查接受布局的最短柱规则（例如按 `max(1.0, 精确长度)` 比较柱端，或把外端容差改为随最短柱放宽）；用 1 vs 1000 与 0.334971 两例断言 `deliver`+`static` 通过，并保留真错位仍能报出的负例。
- 置信度：高。

### F-04 · [P2] 横向柱图的 0 值标注高出所属行，静态图被节点裁切检查拒绝

- 定位：`tools/phoenix-diagram/src/layout/chart.rs:1242`
- 问题：横向柱图的非零值标注基线是 `center_y + 3.8`，0 值却用 `center_y - 6.0`（同一行差 9.8 px）。1 个系列时行带高 26，0 的标注顶端高出节点框约 4 px，`static` 的 `nodeTextClips` 判失败；即使侥幸通过，0 的标注也明显高于同行其他标注。计划只要求 0 不画柱、标注在柱外端，没有要求改变基线。
- 证据：
  - 输入 `{"form":"bar","orientation":"horizontal","rows":[{"id":"a","label":"Idle","values":[0]},{"id":"b","label":"Busy","values":[42]}]}`。
  - `static` 退出 1，`nodeTextClips: [{"id":"a","text":"0","box":{"top":78,...},"textBounds":{"top":74,...}}]`（亮暗两次）。
- 修复与验证：0 值沿用与其他标注相同的基线（水平位置仍在零线）；用含 0 的横向柱图取 `static` 断言 pass，并断言 0 与非 0 标注基线差 ≤ 1。
- 置信度：高。

### F-05 · [P3] 测试名仍写「eleven types」

- 定位：`tools/phoenix-diagram/tests/cli.rs:128`
- 问题：图型已增至十二类，测试名 `cli_guide_routes_the_eleven_types_and_lists_alternatives_when_ambiguous` 仍写 eleven。不影响行为，但这是对当前接口事实的错误复述，会误导后续维护者；计划 §V-5 第 4 步要求「十一类」的现行复述在同一改动内更新。
- 修复与验证：改名为 twelve，保持断言不变。
- 置信度：高。

## 待核实 / 待决策

无。评审边界内能取得的证据已在本轮取得：自动检查、真实入口 `deliver`/`static`/`browser-check`、elintp 实际渲染、错误输入路径都已运行并据此裁定。

## 覆盖与验证

- 意图：为 Phoenix Diagram 增加 `chart` 图型（柱、折线、饼、限额）与随图数据表，并让 elintp 渲染静态图的数据表，同时保持十一类输入与产物不变。
- 仓库根：`/Users/rockie/Documents/gh-xgent/skills`；评审日期 2026-11-15。
- 审查方式：`1d9e3f57^..8f382080` 的净差异（`git diff --no-ext-diff --no-textconv`），逐个读取新增的 `layout/chart.rs`、`render/chart.rs`、`check/chart.rs`、schema、semantics、`static_style`、elintp `draft.mjs`/`page.js`/`page.css` 与全部改动文件，并对照计划 §5、§9 与完成记录。未审：计划文件自身与记录的内容质量（只作背景）、`dist/phoenix-diagram/bundle` 与源的一致性（由 `build.test.js` 覆盖）、十一类静态 SVG 对基线的逐字节比对（见下）。
- 实际运行（目标版本 `8f382080`，开发二进制 `tools/phoenix-diagram/target/debug/phoenix-diagram`，`mise exec --no-deps --`）：
  - `mbx test --test chart --test cli --test check_anchors --test input` → 14/11/13/26 全过；`--test evidence --test render_remaining` → 10/6 全过（`--test delivery` 在整 crate 运行中 6/6 通过）。
  - 其余相关套件：`architecture` 15、`class` 9、`erd` 9、`tree` 6、`timeline` 7、`waterfall` 4、`workflow` 24、`statechart*` 27、`render_check` 15、`render_source` 4、`preview` 5、`i18n` 4、`format` 6、`svg_path` 6、`text_segmentation` 7 全过 —— 十一类与登记点无回归，且未改动任何 fixture/oracle。
  - `mbx fmt --check` 通过；`node --test test/phoenix-diagram/skill.test.js test/phoenix-diagram/build.test.js tools/phoenix-diagram/tests/viewer/catalogs.test.mjs` 26 全过；`node scripts/assemble-phoenix-diagram-reader.mjs --check` 通过；`node --test test/elintp/render.test.js` 16 全过。
  - 真实入口：最小示例 `deliver`/`static` pass；`static` 产出的 `<metadata data-phoenix-table>` 与阅读器 `details.chart-data` 逐格一致（bar 2 系列、limits 3 行含超限/只有上限/行级 unit 三种写法）；elintp `renderPage` 把静态图的数据表渲染成可展开 `.figure-data`，与卡片 metadata 并存时也正确。
  - 错误输入：饼图负值 `/rows/0/values/0`、全 0 `/rows`、两个系列 `/series`，柱图数值个数不符 `/rows/0/values`，`form: "column"` `/form`，均 `finalize` 退出 1 且不产出 HTML（WORK-F6/AC-3 成立）。
  - 40 份随机 candidate 的探针：9 次失败，全部归入 F-01（1 次）、F-03（5 次）、F-04（2 次）与已由计划 D5 接受的「画布过宽触发可读性检查」（1 次），没有发现新的根因。
- 真实入口与静态推演分开记录：F-01/F-04 的结论来自引擎自己的浏览器测量（`static`、`browser-check`）与产出 HTML，不是主观视觉判断；本轮没有重做人眼看图，也没有重跑 V-6 的临时项目离线安装验收。
- 未覆盖：十一类静态 SVG 与基线的逐字节比对。依据是计划 M5 记录称 33/33 MATCH，本轮核对到「静态改动对 chart 生效、`.chart-data` 不出现在非 chart 静态 SVG、全部旧套件与未改 fixture 通过」；该比对不影响本轮结论，若维护者需要独立证明，可在基线快照构建后补做。
- 无关失败：`tests/evidence.rs::a_git_timeout_ends_the_whole_process_group` 在整 crate 运行时失败（`tests/evidence.rs:611` 读不到 pid 文件），同一测试在本轮首次运行与计划 V-5 记录中通过，且该测试未被本改动修改。判断为与本改动无关的负载敏感波动，仅报告，不扩大修复。

## 计划符合性

| 需求/约束 | 计划落点 | 本轮核实 | 状态 |
| --- | --- | --- | --- |
| WORK-F6/AC-1 标签、系列、数值、单位一致 | §5.3、§5.4；V-1、V-2 | 柱/限额的阅读器表与静态表逐格一致；数值原样写出，单位在刻度/表头 | 成立；饼图静态交付受 F-01 阻断 |
| WORK-F6/AC-2 轴含 0、负值分侧、多系列图例与非颜色区分 | §5.3、§5.4；V-1 | 轴含 0，负值在零线另一侧，系列键与图例齐全 | 存在缺陷：F-02 使宽图刻度重叠并交付失败；F-03 拒绝小量级柱 |
| WORK-F6/AC-3 错误输入指向具体行且不产出图 | §5.2；V-1、V-2、V-4 | 5 个错误输入均退出 1、路径精确、无输出 | 成立 |
| WORK-F6/AC-4 每图带可展开/可复制的数据表，限额标出超限 | §5.4、§5.7、§5.8、§6；V-1–V-3 | 阅读器 `.chart-data`、静态 `<metadata data-phoenix-table>`、elintp `.figure-data` 三处一致 | 成立；饼图静态路径受 F-01 阻断 |
| WORK-F6/AC-5 没有真实数值不画，示意时标注 | §5.1、§5.3、§5.10 | `meta.evidence` 必填，`illustrative` 画徽标，`sourced` 不画 | 成立 |
| WORK-F6/AC-6 相同交付入口、主题与导出，未知形态拒绝 | §5.1、§5.9；V-1 | `deliver`/`static`/`validate`/`preview` 都接受 `chart`；`form` 未知被 schema 拒绝 | 部分成立：F-01 使常见饼图无法 `finalize`/取静态图 |
| WORK-F5/AC-3 elintp 选用数值图表并显示数据表 | §6.3；V-3 | SKILL 对照表含 `chart`；静态图数据表渲染为 `<details class="figure-data">` | 成立 |
| WORK-F9/AC-5 数值图表可取得静态图，完整性/亮暗/体积同十一类 | §5.8；V-2、V-5 | 柱图静态图 pass；饼图在 en/es 失败 | 存在缺陷：F-01 |
| WORK-N3/AC-4 十一类结果不变，文字同步 | §5.9、§5.10、§6.3；V-5 | 十一类套件与未改 oracle 全过；`skill.test.js` 十二类描述与前 100 字通过；`skills`、`README.md`、`docs/phoenix-diagram.md`、`bin` 的现行复述无「十一类」残留（PRD 的「既有十一类」指本期新增前的图型，保留原文；测试名见 F-05） | 成立（字节比对未独立重做） |
| M1–M5 完成记录与验证门槛 | 完成记录 | 登记提交与记录齐全；V-2 的「四形态静态图」只在 zh 样例成立，V-6 的横向柱状图不含 0 值行 | 计划符合性缺口：F-01、F-04 未被这两处验收覆盖 |

## 总体结论与修复顺序

**验收判定：需要修改。**

`patch is incorrect`：本轮在声明的 diff 内确认了 2 个 P1（F-01 饼图说明裁切使默认语言下的饼图无法完成阅读器与静态图交付；F-02 竖向轴刻度步长用错长度，宽图刻度重叠且合法图被检查拒绝）与 2 个 P2（F-03 拒绝计划要求的最短柱；F-04 横向柱图 0 值标注越出行带），另有 1 个 P3。其余承诺（schema/语义、数据表三处一致、十一类回归、错误输入诊断、elintp 集成）已核实成立。

修复顺序：先修 F-01 与 F-02（两处都阻断整类图的交付，并决定后续验收能否复现），再修 F-03、F-04（同一批可一起验证），最后是 F-05。修完至少复验：四种形态各一份 en/es/zh 的 `deliver` + `static` + `browser-check`；1 vs 1000 的柱图；含 0 的横向柱图；6×3 柱与 20 点折线；随后按原范围复审。

本结论只覆盖 `3889a9ae..8f382080` 的代码与当前计划版本，不是合并、发布或需求验收授权。
