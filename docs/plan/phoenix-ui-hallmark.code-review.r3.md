# PHOENIX-HALLMARK · 代码评审 R3（修复效果复审）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3，修复效果复审 | [Phoenix UI 借鉴 hallmark](phoenix-ui-hallmark.md) | `150e5160b74dd72bf8f62afcb2690a81120670e4..dfb6643c2376aa3b3a238010deee6b9732b52754`，以及 R2 所列全部旧问题和待核实项 | [R2 代码复审](phoenix-ui-hallmark.code-review.r2.md) |

评审 ID：`7a527f25-29bf-494e-9397-0f438b5e35f0`。日期：2026-10-23（Asia/Shanghai）。由当前主 agent 独立完成，未分派其他 reviewer。

上轮审查基线为 `3e3885b78e08d09f9dfef2b5b78e88fa3131b115..150e5160b74dd72bf8f62afcb2690a81120670e4`；R2 报告 SHA-256=`6ad18e7dcde998ddcc173c9fd8228830b09d09ca343c57955e876127e7253be1`。本轮修复提交为 `5c2f0d766374d78202ae1c62aa1d364d068ca495`，target `dfb6643…` 只回填修复 SHA。两端树净差异是本轮新增问题的归因范围；旧问题按当前 target 核对，不要求落在该 diff。本轮没有重新完整审查 R1 的原始实现。

已有处置：[M1「代码评审 R2 修复」](phoenix-ui-hallmark.records/M1.md#代码评审-r2-修复2026-10-23)、[M5「代码评审 R2 修复」](phoenix-ui-hallmark.records/M5.md#代码评审-r2-修复开发产物重绑与回归2026-10-23)。以下结论以实际代码和独立验证为准。

## 1. 确认问题与上轮问题复审

### 1.1 上轮问题复审表

以下沿用 R2 及其 R1 来源编号；R2 对象为 `1012ce80-e25c-4d14-9f91-60e2342882b1`，R1 对象为 `1e55e17d-4399-4289-bd1c-131954c5208e`。版本及报告链接见上文；修后版本统一为本轮 target。

| 来源报告 / ID | 上轮问题与结论 | 已有处置 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R2→R1 / F-01 | 注释中的焦点类名误报，已解决 / P2 | R1 注释空白化；本次未改该逻辑 | **已解决** | `text.rs:207–263`；文本注释用例及 hook 的注释-only 放行回归；原代码保持不变 | 无 |
| R2→R1 / F-02 | focus-within、not-focus 等误判，已解决 / P2 | R1 共用变体语义判断；本次未改该逻辑 | **已解决** | `text.rs:1052–1134`；变体和否定条件的文本回归通过 | 无 |
| R2→R1 / F-03 | 静态类名表达式拆开判定，部分解决 / P2 | R2 新增表达式窗口与解析器 | **部分解决** | 原括号拼接、数组 join、平坦 cn 已通过；嵌套静态 cn 仍 warning / Cursor deny，见 §1.2 | 正确确定完整表达式边界，未知时停止缺少替代的断言 |
| R2→R1 / F-04 | 零宽单位漏报、非零小数误报，已解决 / P2 | R1 完整值判断；本次增加优先级剥离 | **已解决** | `text.rs:394–474`；零长度、颜色函数、非零小数回归通过，紧凑优先级另按 F-11 关闭 | 无 |
| R2→R1 / F-05 | SVG title 属性名称遗漏，已解决 / P2 | R1 接纳非空 title；本次未改 DOM 实现 | **已解决** | `dom.rs:260–267`、`tests/dom.rs:254–297`；FakeDom 正反例通过；R2 真实 Chrome 证据对应代码未变 | 无 |
| R2→R1 / F-06 | 包裹 emoji 重复计数，已解决 / P2 | R1 按直接文本归属选候选；本次未改 DOM 实现 | **已解决** | `dom.rs:402–407`、`tests/dom.rs:632` 起的回归通过；R2 两宽度证据对应代码未变 | 无 |
| R2→R1 / F-07 | aria-hidden 让视觉规则漏检，已解决 / P2 | R1 独立视觉可见性 helper；本次未改 DOM 实现 | **已解决** | `dom.rs:546–568`；视觉规则的 aria-hidden / 真隐藏回归通过；原 SVG ARIA 处理未改 | 无 |
| R2→R1 / F-08 | 正文代码示例当 transition 声明，部分解决 / P2 | R2 仅在 tag_spans 返回的片段里找 style 属性 | **部分解决** | 原转义代码块、script 内 div 字符串已通过；其他属性值内的 style 示例、script 内 style 字符串仍误报，见 §1.3 | 识别真实属性及样式块上下文 |
| R2→R1 / F-09 | 标签折行指引冲突，已解决 / P3 | R1 改为重排控件行；本次未改守则 | **已解决** | 源/生成 `reference/operate.md:51` 一致；生成物新鲜度检查通过；R2 布局实测所对应内容未变 | 无 |
| R2→R1 / F-10 | 开发产物来源绑定过期，已解决 / P3 | R2 定稿后重建并重绑 | **已解决** | 原构建函数重算 577 个输入，聚合 `0a7b3808…e180` 与 VERSION、ENGINE、build-record 一致，逐项差异 0；分发引擎 hash 一致 | 后续改规则须再次重建 |
| R2 / F-11 | 紧凑 !important 漏报 / P2 | R2 增加 strip_css_important | **已解决** | `text.rs:400–428`；四种紧凑写法由 0 条恢复为各 1 条 warning，带空格/单位正例及 0.5px 反例通过 | 无 |

旧问题共 11 项：9 项已解决（其中 8 项维持上轮关闭、F-11 本轮关闭），F-03/F-08 部分解决；不再适用 0 项，无未复核的旧 finding。以下两条沿用旧编号，不计为本轮新增。

### 1.2 R2→R1 / F-03 · [P2] 将完整嵌套类名表达式作为判断边界

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 693–698 行；提前返回见 655–658 行。
- 问题：`group_window` 只选最小包围括号，且 `expression_text` 只要能解析该子组就立即返回，没有确认它属于更大的类名表达式。例如 fixture 明确定义 `cn = (...parts: string[]) => parts.join(" ")` 时，`cn(("focus:outline-none"), cn("focus-visible:ring-2"))` 是完整、静态且已写替代的类列表，当前实现却单独判断第一组字符串并报 warning。括号外为动态操作数的 `("focus:outline-none ") + extra`、`cn(("focus:outline-none"), extra)` 也被当成完整类列表。原先“不能证明完整内容时不应断言没有替代”的关闭条件尚未满足，Cursor 对这些安全文件的无关文字修改仍会 deny。
- 证据：原 R2 的括号拼接、数组 join、平坦 cn 在 target 均 0 条 / exit 0；上述自含 cn 定义的嵌套静态输入在 base、target 都报第 2 行 1 条 warning / exit 2，target `hook-before-edit` 把 `Save` 改成 `Save now` 返回 deny。相同输入把替代改为 `mt-2` 仍正常 warning / deny，说明修复不能仅将所有组合一律放行。当前源码重建的 debug 引擎也复现安全输入误报。`tests/text.rs:312` 的既有嵌套 join 用例未覆盖这个窗口边界。
- 修复与验证：先确定完整表达式及外层调用/运算边界，再执行支持的静态求值；不能确定时跳过缺少替代的结论。补充完整静态嵌套、带括号的未知操作数、真实缺替代三类输入，并逐项检查 CLI 与 Cursor 行为。
- 置信度：高；代码边界、两个版本的 CLI、当前源码引擎和实际编辑门结果一致。

### 1.3 R2→R1 / F-08 · [P2] 区分真实 style 属性与其他属性里的示例

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 304–309 行；样式块分支另见 299–301 行。
- 问题：本次将正则范围缩到一个真实标签，但仍对标签的全部文本运行 style 属性正则，没有跳过其他属性的引号值。因此 `<p title='Avoid style="transition: all .2s"'>Guide</p>` 与 `data-example='style="transition: all .2s"'` 都没有真实 style 属性，仍产生 `xgent/transition-all` advisory。另一个同类残留是样式块正则仍扫描全文，`<script>const tpl = "<style>.x { transition: all .2s; }</style>";</script>` 中的字符串也被当作样式块。原“显示出来的代码示例不是声明”的问题只关闭了部分输入，文档页面仍会收到误报。
- 证据：上述 title、data-example、script-style 三个隔离 HTML 输入在 base、target 都各报 1 条该 advisory；原转义代码块与 script 内 div 示例由 1 条变为 0。真实 style 属性对照仍 1 条，snippet 和行号正确。当前源码 debug 引擎独立复现 title 误报。计划 [NFR-1](phoenix-ui-hallmark.md#01-需求与约束账本)（第 81 行）要求处置误报类别；本轮没有把这些旧输入误报列成新增 finding。
- 修复与验证：在实际标签内按属性边界读取 style，其他属性值不再次当作标签/属性解析；样式块也沿实际标记上下文识别，排除脚本字符串等原始文本。回归单双引号、其他属性中相反引号的 style 示例、script 字符串、实体转义正文与真实声明。
- 置信度：高；确定性输入不含真实 CSS，base/target 和当前源码均可复现。

### 1.4 本轮新增 F-12 · [P2] 避免为每个字符串重解析整个 JSX 容器

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 655–658 行，与本轮 diff 重叠；调用循环见 486–489 行。
- 问题：在 `return (<>…</>)` 的普通 JSX 中，各个属性字符串都可能选中同一个外层 return 括号；每次都先把整段 JSX 重新 token 化，解析失败后才回退到单个属性。因而含 N 个字符串、L 字节容器的干净文件也反复处理约 N×L 的内容，本次把原先接近线性的文本扫描变成明显的二次增长。该逻辑同步用于 CLI 和编辑前扫描：低于现有 1 MiB 上限的 400,932 字节输入已让一次无关编辑等待约 6 秒，即使文件没有任何移除焦点的类。
- 证据：使用已核 hash 的 base/target release 二进制，固定同一组 JSX 输入、同机顺序运行：1,000 行 / 65,932 B，0.016s→0.177s；3,000 行 / 199,932 B，0.030s→1.512s；6,000 行 / 400,932 B，0.046s→6.334s，结果均为 `[]` / exit 0。反转运行顺序复测，target 为 6.262s / 5.993s，base 为 0.082s / 0.043s。实际 `hook-before-edit` 对最后一份文件只改标题文字：base 0.045s、target 6.146s，均 allow；因此时间来自扫描而非发现数量。输入由 `export function App(){ return (<>\n`、N 个 `<button className="px-2 py-1" title="Save item {i}">Save</button>\n`（i=0…N−1）及 `</>); }\n` 组成，属于明确的合成规模实验，不代表所有项目的耗时。[before_edit.rs](../../tools/phoenix-ui/crates/hook/src/before_edit.rs) 第 26、777–778 行允许扫描这份文件。
- 修复与验证：避免按字面量反复解析同一大容器，可缓存窗口分析/失败结果，或直接识别属性的完整表达式边界；同时避免为明确无关的字面量重复做整个容器的解析。用相同规模输入对比 release CLI 与实际编辑门的增长，并保留 F-03 的静态/未知表达式正确性回归。
- 置信度：高；base→target 性能回退、规模增长和真实 hook 路径均已直接测量，未以源码猜测代替测量。

## 2. 待核实项与覆盖限制

- R2 留存的 V-9 浏览器操作权限、产品观感及 UI 实拍限制仍保留；本轮未发起 provider 会话或独立 UI 验收。原有 provider 文字/工具行为记录不能升级为当前视觉验收通过。
- 本机为 darwin-arm64；其他平台和发布不在本轮运行验证范围。未穷尽所有 Unicode、任意动态类名和 SVG 名称路径，也没有重新完整审查原始实现。本轮两个旧问题残留及 F-12 均有独立、明确输入，不依赖这些穷尽性缺口。
- 本轮额外启动的完整 `npm run test:phoenix` 通过安装集成、部分 Rust 套件后，因未改安装/迁移用例耗时较长，由评审者主动以 SIGTERM 停止。最终退出 1；这不是全量通过，也不是已证实的新增测试失败。改为独立执行本轮相关的规则、hook、CLI、lint、render 检查。已读取 M5 对应记录和 `local/phoenix-ui/r2-fix-evidence/test-phoenix-final.log`，日志末尾有 8,474 向量零失败、860/860 oracle，且当前 577 个输入与该构建记录一致；该整套通过属于实施方既有证据，不写成本轮独立全量通过。

## 3. 覆盖与验证

### 3.1 范围与固定版本

- 原始输入：`$review-code docs/plan/phoenix-ui-hallmark.md 150e516..HEAD docs/plan/phoenix-ui-hallmark.code-review.r2.md 复审`。r2 路径作为用户指定的上轮报告，本轮另存 r3，保留历史。
- 意图：按 M1/M5 处置记录修复 F-03/F-08/F-11，并同步定稿后的生成物与构建来源；这是记录明示的意图。
- 仓库：`/Users/rockie/Documents/gh-xgent/skills`，分支 main，开始时工作树/index 干净。实际比较是报告头两个完整 SHA 的树净差异，不包含起点提交自身的修改，无额外文件限制或未提交输入。
- target tree=`7c2c952f24cbfe3d578275dde42d322f0b26cedc`；计划取 target 版本，SHA-256=`a621334d50d9a0f9e9854fd7fafc1f0b4fd024d8637b0f38e4df90635f1c845d`。上轮报告、计划与代码等评审输入可从该 Git 快照取回。
- 初始 4,773 个 tracked 普通文件按仓根相对路径映射内容 SHA-256，再将映射按路径排序，以 UTF-8、ensure_ascii=false、无额外空格的 JSON 编码后作 SHA-256：`1485fdff2bfc76f2e38cf63d8baa69f7f38c922c598149f9008bc58b7728c787`。写报告前逐项重算差异为 0、HEAD 未变；本轮报告与台账为输出，不加入被审 diff。
- 范围共 15 个变更文件：生产 `crates/rules/src/text.rs`；文本/hook 测试；VERSION、ENGINE、GENERATED、capabilities 四份清单；计划、M1/M5、R2 报告及四份台账文件。完整 diff 已读。文档与台账用于契约/处置核对；新 findings 不以状态自述代替运行代码证据。

### 3.2 已审行为链

| 范围 | 覆盖 |
| --- | --- |
| 文本规则 | 全部修复 diff 与 text.rs 当前全文；注释、焦点变体、静态/未知表达式、CSS 值/优先级、HTML 标签/属性/原始文本、行号和 snippet |
| Hook / CLI | 文本入口、before-edit 的内容大小门与同步调用、完整 allow/deny 输入、现有和新增 hook 测试；未改 hook 生产接线 |
| 旧 DOM / 守则问题 | 确认对应生产代码和源守则未改，复跑 DOM/文本回归，复核源/生成文本与清单；复用适用的 R2 证据，不冒称完整原始重审 |
| 构建来源与生成物 | 全部清单 diff；GENERATED 仅改 ENGINE hash 的 20 处副本引用；bundleSha256 保持 `a35570d9…ee95`。按原 sourceInputs 函数重算全部输入与 build-record，验证分发字节来源 |
| 性能 | 两个 release 固定副本顺序比较三个规模、反向顺序复测及真实 Cursor 输入；F-12 仅引用实测规模，不推断未经测量的容量 |

### 3.3 本轮实际执行

命令在 target 字节上执行；临时输入、引擎固定副本和日志放在隔离临时目录，未向目标源码添加测试或修复。

| 命令 / 方法 | 结果 |
| --- | --- |
| 仓根 `npm test` | exit 0；Node 425 项：398 pass、27 skip、0 fail；Python 24 项、13 项通过，skills 2 项因 PyYAML 缺失跳过 |
| tools/phoenix-ui 下 `mise exec --no-deps -- cargo test -p phoenix-ui-rules --locked --offline` | exit 0；文本 19/19、DOM 21/21 |
| 仓根 `npm run test:phoenix` | 主动中止，最终 exit 1（SIGTERM 使 cargo 退出 101）；此前安装集成 11/11、Rust 46 组完成且无失败。停在未改 `phoenix-ui-install --test installer`，未将中止视为代码缺陷；环境变量来自既有 prepare 回执 |
| tools/phoenix-ui 下 `mise exec --no-deps -- cargo test -p impeccable-hook --test hook_tests --locked --offline` | exit 0；76/76，包含新增 allow / deny 及旧注释-only 放行用例 |
| 同目录 `mise exec --no-deps -- cargo test -p phoenix-ui --test xgent_rules --locked --offline` | exit 0；9/9，含真实 Chrome 的 13 fixture × 375x812 / 1280x800；用时 172.96s |
| 仓根 `node scripts/check-phoenix-lints.mjs` | exit 0；No added diagnostics，既有 fmt 差异及 539 条 clippy 诊断按基线保留 |
| tools/phoenix-ui 下 `mise exec --no-deps -- bun scripts/render.mjs --check` | exit 0；fresh、19 providers，input `0373592b…0069a`、bundle `a35570d9…ee95` |
| base / target `detect --json <隔离文件>` | 原 R2 三个静态组合与转义正文已修；F-11 正反例通过；嵌套静态 cn 和 HTML 属性示例残留由 release 与当前源码 debug 引擎共同确认 |
| target `hook-before-edit`，stdin 为 Cursor preToolUse / Edit | 静态括号拼接 allow，真实缺替代 deny；安全嵌套 cn 和括号外未知表达式仍 deny；400,932 B 文件的无关编辑 allow，但耗时 6.146s |
| 构建输入与分发来源独立重算 | 577/577 输入一致；聚合 `0a7b3808079bc20e7c3230a7645344648fab85fbf9c3b74e917c7f500455e180`；VERSION=ENGINE=release VERSION，分发引擎 hash 匹配 VERSION |

引擎身份：base release SHA-256=`6bf2c7c7d54e9cd4bb6a7357d8cd11c6f7c72c4730e810e4751b00126fc8ddec`，与 R2 独立核查的分发引擎精确一致；target release SHA-256=`ed72b03cf51e97f50b19f5cd8b2a48f5ae8a3c9c50eab653a5993fac8c5ed9eb`，与当前 VERSION 和 build-record 绑定一致；当前源码 debug SHA-256=`9c482faf343c349bcb04eb69eb949e6f26c887051159fbf73a45c7452ceb60a9`，由本轮 test:phoenix 重建后复制固定。

临时证据根目录：`/var/folders/cv/pn7p1v7s737cqlt605jyl7lm0000gn/T/phoenix-hallmark-r3.rgk_ytsg/`，包含输入、CLI/hook 对照、timings、binding 与本轮日志。该目录仅本机可用；判断需要的版本、最小输入、代码位置及结果已写入本报告。

## 4. 本轮计划符合性

| 契约 / 本轮任务 | 核查结果 |
| --- | --- |
| R-2、NFR-1；F-03/F-08/F-11 准确性修复，V-1/V-2 | 原 R2 示例通过、F-11 关闭；F-03/F-08 的同类误报仍有明确输入，不能按 M1 自述认定全部关闭 |
| 原生文件检测与 hooks 行为 | 接线保持；F-03 残留仍直接影响 Cursor 编辑放行；F-12 新增同步扫描性能回退 |
| C-3 / 定稿后生成与来源绑定，V-8 | 577 个输入和分发字节均匹配；F-10 维持关闭；再次改规则后须按既有流程重建并重绑 |
| C-2 / oracle 精确期望，NFR-2 | 本次没有改 golden、adapter 期望或内置规则；实际回归结果见 §3.3，不能用 oracle 通过关闭未覆盖的误报输入 |
| 其余 R-3–R-7 / 界面、provider 与发布 | 修复未改变这些接口/授权/守则，沿用原范围和限制；本轮未重新全面验收 |

计划来源为用户需求，无 PRD。本轮不修改计划进度，不替代 UI/PRD 验收。

## 5. 总体结论与修复顺序

**验收判定：需要修改。**

修复效果结论：**未完全解决**。R2 所列 11 项旧问题中 9 项已解决，F-03/F-08 部分解决；新增 1 项 P2（F-12 性能回退）。本轮修复 diff 的正确性结论为 `patch is incorrect`，由 F-12 的确定回退支撑；两项旧问题按复审结果独立影响验收。不将剩余误报重新编号，也不将全量检查或视觉限制本身列成代码缺陷。

1. 先修 F-03 完整表达式边界，并在同一解析链解除 F-12 的重复工作；保留静态安全组合、未知操作数与真实缺替代的正确行为。
2. 修 F-08 的真实属性/样式块上下文，确保代码示例不报、真实声明继续命中。
3. 复跑直接受影响回归及 release CLI/编辑门规模对照；定稿后重新构建并重绑产物，再逐项复审 F-03/F-08/F-12。

解除阻塞条件：安全嵌套表达式不再误报/拒绝编辑，未知表达式不作“缺少替代”断言，HTML 代码示例为 0 条，真实正例保持；同一大容器不再随每个字符串重复解析，release CLI 与实际编辑门的规模回退得到验证。报告和台账保存不代表合并、发布或产品验收授权。

下一步：

```text
$apply-code-review docs/plan/phoenix-ui-hallmark.code-review.r3.md
```
