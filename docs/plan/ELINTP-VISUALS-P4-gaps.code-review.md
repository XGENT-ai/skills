# ELINTP-VISUALS-P4-gaps · 代码评审（R1）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [ELINTP-VISUALS-P4-gaps](ELINTP-VISUALS-P4-gaps.md)；`sha256:eef769b169c627297ffc7d1d277f09a16f7e8339f85c7863f6c8180e5524a2b8`，HEAD `2200a033` | `b4c7b568539e59f62fc6756945b30686a70c1480..5d1bc30d948120e747278f2f5796dbcbc64d6216` 净差异（M1 `75f3c597`、M2 `5d1bc30d`）；`tools/phoenix-diagram`、计划与记录目录无未提交改动 | 无 |

意图一句话：给数值图表补四处修正——多系列图例色块写系列键、超限与饼图收尾扇区用醒目色槽、数据表单元格只放数字且单位进列头、仅上限的限额行改用虚线框并在图例写「仅上限」。

## 1. 确认问题

### F-01 · [P2] 饼图列表色块被 `.ch-fill` 全局降到 55%，与扇区不再同色

- 定位：`tools/phoenix-diagram/src/render/chart.rs:53`（target `5d1bc30d`；同处新增的 54–55 行是 `.ch-fill.c-security` 与 `.ch-span` 规则）；被波及的调用点在 `tools/phoenix-diagram/src/layout/chart.rs:645`（饼图列表色块）与 `render/chart.rs:234`（填充 mark 统一写 `ch-fill`）。
- 问题：M2 为让暗色主题的超限更醒目，把全局 `.ch-fill` 的 `fill-opacity` 从 `.9` 降到 `.55`，只给 `.ch-fill.c-security` 保留 `.9`；但 `.ch-fill` 也是饼图右侧列表每行 14×10 色块的类，而扇区用 `.ch-slice`（透明度 1）。于是任何饼图的列表色块都变成扇区颜色的淡色版，亮暗主题、阅读器与静态图一致发生；色块是「序号列表↔扇区」之间唯一的颜色连接，淡一档后读者拿键色去对扇区的映射被削弱，G-02 专门修好的 external 色块（第 6 项）反而最淡。
- 证据：基线 `b4c7b568` 的 `render/chart.rs` 写 `.ch-fill { stroke: none; fill-opacity: .9; }`，与扇区接近同色；本次 `render/chart.rs:53` 改为 `.55`。计划 D3（`ELINTP-VISUALS-P4-gaps.md:253`）把 `.55` 限定为「限额图未超限填充与『当前值』图例色块」，§5.2（`:314`）还写明「饼图收尾扇区与其列表色块用 `var(--external-stroke)`」，都没有要降低列表色块透明度。用本引擎在 HEAD 渲染 6 行 `files` 饼图（`/tmp/p4g-review/pie.json`，`finalize`/`static` 均 exit 0）后取像素：列表色块 `#76c3d5`=(118,195,213)、扇区 `#0891b2`=(8,145,178)；后端色块 (118,197,173) 对扇区 (5,150,105)；按基线的 `.9` 合成应为 (33,156,186)，即改动前两者基本同色。V-1（`:358`）对饼图只要求「清晰可辨」，M1 登记的 8/8 看图又发生在 `.55` 之前，M2 只复看了限额与柱图，所以这处副作用没有证据覆盖。
- 修复与验证：把 `.55` 限定在限额填充——例如给限额行与「当前值/超限」图例符号加 `ch-fill ch-limit-fill` 修饰类，规则写成 `svg[data-chart-ui] .ch-fill.ch-limit-fill { fill-opacity: .55; }`（或让饼图列表色块复用 `.ch-bar` 的 `.9`），`.ch-fill.c-security` 保持 `.9`，饼图色块回到与扇区同色。验证：改后对同一饼图 candidate 重跑 `finalize`/`static`，取样列表色块与扇区颜色一致；在 `tests/chart.rs` 补一条断言，饼图色块不带限额专用透明度修饰类（或扇区与色块共享同一条 `fill` 规则），再跑 `mbx test $MF --test chart`。
- 置信度：高（0.85）。依据：`.ch-fill` 只有限额填充、限额图例与饼图列表色块三处使用（`grep ch-fill src/`），像素实测与 CSS 合成一致，计划与记录均无降低饼图色块透明度的决定。

## 2. 待核实/待决策项

无。本轮能自行取得的证据都已在本轮取得，见第 3 节；无需要用户或外部输入的缺口。

## 3. 覆盖与验证

- 仓库根：`/Users/rockie/Documents/gh-xgent/skills`；评审日期 2026-11-15；代码与计划来自 `5d1bc30d`（HEAD `2200a033` 只回填计划 SHA）。
- 范围与排除：登记提交 M1 `75f3c597`、M2 `5d1bc30d`；区间内的 `8e61d77c`、`4667b732`、`2200a033` 只改计划、记录与回执，属进度回写不纳入代码 findings。工作树对 `tools/phoenix-diagram`、计划与 `records/` 无改动；`skills/use-llm-gateway/**`、`.xgent-ai/**` 与其他计划的评审文档不在本计划改动面，未审。
- 已读行为链：`layout/chart.rs` 的系列键与表格三形态、限额 mark/图例条目/标签基线 → `render/chart.rs` 的 `defs`、`fill`/`span`/`legend_glyph` → `check/chart.rs` 的系列图例、限额填充/span/图例与表格 → 五语言目录 → 装配脚本生成的 `assets/template.html`、`catalogs.json`。
- 实际命令与结果（均在本轮执行，基线 = 当前 checkout `2200a033`，与所审代码同源）：

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| 构建 | `mise exec --no-deps -- mbx build --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --bin phoenix-diagram` | up to date（0.30s），dev 二进制即当前树 |
| 图表测试 | `mbx test $MF --locked --offline --test chart` | 16 passed |
| 目录/i18n | `mbx test $MF --test i18n`；`node --test tools/phoenix-diagram/tests/viewer/catalogs.test.mjs` | 4 passed；9/9 passed |
| 锚点/浏览器契约 | `mbx test $MF --test check_anchors --test browser_contract` | 13 passed；15 passed、2 ignored（按设计只在 core 阶段跑真实 Chrome） |
| 阅读器资产 | `node --test test/phoenix-diagram/reader-assets.test.js` | 8/8 passed |
| 装配一致性 | `node scripts/assemble-phoenix-diagram-reader.mjs --check` | exit 0，写出与仓内相同的 template/catalogs sha256 |
| fmt/clippy | `mbx fmt $MF -- --check`；`mbx clippy $MF --all-targets -- -D warnings` | 均 exit 0 |
| 真实产物 | `finalize chart`（bar en/ms 两系列、limits zh-CN 含带 note 的仅上限行、pie en/files 6 行）；`static chart`（同名 embedded） | 6/6 exit 0，`automatedStatus: pass`；`deliveryStatus: incomplete` 仅因本次未登记看图记录 |
| 产物内容 | 解析上述 HTML/SVG | 系列图例色块文字 `A`/`B`，标签 `A · Before`；限额图例顺序「当前值/上限/仅上限/超限」；仅上限行只有 `rect.ch-span`（宽度比 0.8=200/250）、无 `rect.ch-fill`，`data-chart-value=""`、`data-chart-limit="200"`；表头 `Item / Before (ms) / After (ms)`、`Item / Value (files) / Share (%)`、限额表仍为「单位」列；单元格 `4200`、`41.2` 等纯数字；静态 `data-phoenix-table` 与阅读器表格逐格相同 |
| 观感/颜色 | 1440×900 亮暗 PNG 取样与裁剪查看 | 限额图超限比未超限更醒目：亮色 (228,51,90) 对白 4.3:1，未超限 (116,194,212) 2.0:1；暗色 (227,103,123) 相对亮度 .275，未超限 (23,123,146) .165；饼图列表色块与扇区颜色不一致（F-01） |

- 未覆盖与依赖的既有证据：十一类最小示例的 `deliver`/双策略 `static` 指纹 33/33、体积比 3.9%–14.6%，依据 M2 记录与 [`receipts/m2/v3-compat.log`](ELINTP-VISUALS-P4-gaps.records/receipts/m2/v3-compat.log)、`v3-run-final4.log`（同一份代码、36/36 exit 0），本轮未重跑；V-4 的离线安装与 CDP 剪贴板验收依据 [`M2 记录 · 验收`](ELINTP-VISUALS-P4-gaps.records/M2.md) 与 `receipts/m2/v4-reader.log`，本轮以 HTML/静态表内容替代核对，未重跑真实浏览器复制；截图只亲自看了限额暗色、饼图亮色与柱图暗色，其余依赖登记记录。静态推演、局部测试与真实入口证据分别记录，未以测试通过代替观感结论。

## 4. 计划符合性

| 计划要求 | 实现 | 证据 | 状态 |
| --- | --- | --- | --- |
| §5.1 图例色块写系列键，检查要求首个 `text` 等于键 | `render/chart.rs` 取 `facts["key"]`；`check/chart.rs` 增 `glyph_key` | 产物 `A`/`B`；测试改回 id 后报 `artifact/semantic-text` | 已核实 |
| §5.2 色槽循环加 `security`、`external`；限额填充 `.55`、超限 `.9` | `render/chart.rs:54-55,65-69` | 产物含 `.ch-slice/.ch-bar/.ch-fill.c-external`、`.ch-fill.c-security`；亮暗取样超限更醒目 | 已核实（副作用见 F-01） |
| §5.3 表格单元格纯数字、单位进列头、括号随语言 | `layout/chart.rs` 表格三分支、`chart.table.withUnit` | 表头/单元格实测；`catalogs.json` 五语一致 | 已核实 |
| §5.4 仅上限行 span、图例条目、标签 +14/备注 +27、检查二者其一 | layout/render/check 三处 | 产物与 `check` 均符合；本轮 limits candidate 中带 note 的两类行（有值、仅上限）`finalize` pass | 已核实 |
| §5.5 十一类不变 | 仅改 chart 代码 | 33/33 指纹回执 | 已核实（引用回执） |
| V-1/V-2/V-3/V-4 | — | V-2 本轮 16 passed；V-1/V-4 见上表与记录；V-3 fmt/clippy/装配本轮 exit 0 | 已核实（V-1 饼图观感缺改后证据，见 F-01） |

- 偏差记录：M2 把 V-4 发现的三项修复就地补进 D3、§5.2、§5.4，并在 [`M2 记录`](ELINTP-VISUALS-P4-gaps.records/M2.md)「交付行为与路径」留痕，属计划允许的就地修订；但同一轮修订没有把 `.55` 的作用域写清到「只作用于限额填充」，实现按类全局生效，形成 F-01。
- 延期项：限额图按行缩放（577 与 420 因各行上限不同而等长）经用户 2026-11-15 确认延期，见 [`M2 记录 · 延期项`](ELINTP-VISUALS-P4-gaps.records/M2.md)；按本期缺陷口径不计入，也不因此否定 V-4 的修复复验。

## 5. 总体结论与修复顺序

需要修改

- 结论：`patch is incorrect`。四处补缺本身都按计划落地，`finalize`/`static`/测试/装配在 HEAD 上均通过，但 M2 的暗色醒目度修复把 `.ch-fill` 的透明度限制扩大到饼图列表色块（F-01），所有饼图在亮暗主题与阅读器/静态图中都出现「键色淡于扇区」，而计划只把 `.55` 用于限额填充。
- 修复顺序：先按类作用域收窄 `.55`（F-01），重跑 `mbx test $MF --test chart` 并对饼图 candidate 重跑 `finalize`/`static`、看亮暗两张确认色块与扇区同色，再用 `review-code docs/plan/ELINTP-VISUALS-P4-gaps.md` 复审。其余无 P0–P3 问题，旧延期项不影响本次修复顺序。
- 限制：本轮只审 `b4c7b568..5d1bc30d` 与当前工作树，不含十一类的逐类重跑与真实浏览器复制（依据登记回执），也不构成合并、发布或需求验收授权。
