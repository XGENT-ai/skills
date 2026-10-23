# PHOENIX-HALLMARK · 代码评审 R4（修复效果复审）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R4，修复效果复审 | [Phoenix UI 借鉴 hallmark](phoenix-ui-hallmark.md) | `dfb6643c2376aa3b3a238010deee6b9732b52754..035bbc690e695ebd06ebc6165d543d16bf5d4be2`，当前未提交记录，以及 R3 所列全部旧问题、待核实项和直接回归 | [R3 代码复审](phoenix-ui-hallmark.code-review.r3.md) |

评审 ID：`69e27b35-e431-46f8-8cf2-9c8c1a2bf87f`。日期：2026-10-23（Asia/Shanghai）。由当前主 agent 独立完成，未分派其他 reviewer。

上轮所审范围为 `150e5160b74dd72bf8f62afcb2690a81120670e4..dfb6643c2376aa3b3a238010deee6b9732b52754`，R3 报告 SHA-256 为 `53ddc033cf9d4f7af74d8fe9f51d243e678432f98cba839723ebda3d4a448d37`。本轮修复提交为 `d0a67061b126ad25600cc52ccaff5c968a45f410`；后续 `035bbc6…` 更新计划、处置记录和状态页。新增问题仅按本轮两端树净差异归因；旧问题在当前版本逐项核对。本轮不是对 R1 原始实现的完整重审。

已有处置：[M1「代码评审 R3 修复」](phoenix-ui-hallmark.records/M1.md#代码评审-r3-修复2026-10-23)、[M5「代码评审 R3 修复」](phoenix-ui-hallmark.records/M5.md#代码评审-r3-修复开发产物重绑与回归2026-10-23)。以下结论来自独立代码核查和验证，不直接采用处置记录的关闭自述。

## 1. 确认问题与上轮问题复审

### 1.1 上轮问题复审表

沿用 R3 及其 R1/R2 来源编号。R3 对象为 `7a527f25-29bf-494e-9397-0f438b5e35f0`，R2 为 `1012ce80-e25c-4d14-9f91-60e2342882b1`，R1 为 `1e55e17d-4399-4289-bd1c-131954c5208e`；报告链由上轮链接可追溯。修后版本统一为本轮 target。

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R3→R2→R1 / F-01 | 注释中的焦点类名误报；已解决 / P2 | R1 注释等长空白化；本轮未改注释处理 | **已解决** | [文本回归](../../tools/phoenix-ui/crates/rules/tests/text.rs#L219) 与 hook 回归通过，真实类名正例仍命中 | 无 |
| R3→R2→R1 / F-02 | focus-within、否定变体误判；已解决 / P2 | R1 共用变体语义；本轮未改变体判断 | **已解决** | [变体正反例](../../tools/phoenix-ui/crates/rules/tests/text.rs#L250)、否定任意选择器回归通过 | 无 |
| R3→R2→R1 / F-03 | 嵌套静态类名被拆开判断，未知操作数仍断言缺少替代；部分解决 / P2 | R3 重写完整表达式边界，`d0a6706…` | **已解决** | [class_list_span](../../tools/phoenix-ui/crates/rules/src/text.rs#L777)、[新增回归](../../tools/phoenix-ui/crates/rules/tests/text.rs#L351)；嵌套安全与未知操作数由 warning/deny 变为 0 条/allow，嵌套真缺陷仍 warning/deny；详见 §1.2 | 无 |
| R3→R2→R1 / F-04 | 完整零长度漏报、非零小数误报；已解决 / P2 | R1 完整值判断、R2 优先级剥离；本轮未改 | **已解决** | [完整零长度](../../tools/phoenix-ui/crates/rules/tests/text.rs#L471) 与颜色函数零值回归通过 | 无 |
| R3→R2→R1 / F-05 | SVG title 属性名称遗漏；已解决 / P2 | R1 接纳非空 title；DOM 生产代码本轮未改 | **已解决** | [名称分支](../../tools/phoenix-ui/crates/rules/src/dom.rs#L260)、[名称/空值对照](../../tools/phoenix-ui/crates/rules/tests/dom.rs#L254)；DOM 21 项通过，保留此前适用的 Chrome 证据 | 无 |
| R3→R2→R1 / F-06 | 包裹 emoji 重复计数；已解决 / P2 | R1 按直接文本归属选候选；本轮未改 | **已解决** | [归属判断](../../tools/phoenix-ui/crates/rules/src/dom.rs#L402)、[单个/多个包裹图标](../../tools/phoenix-ui/crates/rules/tests/dom.rs#L632) 回归通过 | 无 |
| R3→R2→R1 / F-07 | aria-hidden 让视觉规则漏检；已解决 / P2 | R1 独立视觉可见性判断；本轮未改 | **已解决** | [视觉可见性](../../tools/phoenix-ui/crates/rules/src/dom.rs#L553) 与 aria-hidden/实际隐藏正反例通过；SVG 原 ARIA 行为保持 | 无 |
| R3→R2→R1 / F-08 | 其他属性值内 style 示例及 script 内样式块字符串误报；部分解决 / P2 | R3 按标签、属性及原始文本边界读取，`d0a6706…` | **已解决** | [markup_style_spans](../../tools/phoenix-ui/crates/rules/src/text.rs#L305)、[上下文回归](../../tools/phoenix-ui/crates/rules/tests/text.rs#L557)；原报告 title/data/script-style 输入均从 1 条变为 0，真实属性/块继续命中；详见 §1.3 | 无 |
| R3→R2→R1 / F-09 | 标签折行守则冲突；已解决 / P3 | R1 改为重排控件行；本轮未改守则 | **已解决** | [源 operate.md](../../skills/phoenix-ui/src/reference/operate.md#L51) 与生成版本同句一致；带引擎 render 检查 fresh | 无 |
| R3→R2→R1 / F-10 | 开发产物来源绑定过期；已解决 / P3 | R3 定稿后重建与重绑 | **已解决** | 按原 sourceInputs 函数独立重算 577 个输入，差异 0；聚合、VERSION、ENGINE、build-record 及分发引擎 hash 一致，见 §3.3 | 无 |
| R3→R2 / F-11 | 紧凑 !important 漏报；已解决 / P2 | R2 strip_css_important；本轮未改 | **已解决** | [优先级正反例](../../tools/phoenix-ui/crates/rules/tests/text.rs#L444) 通过，保留非零小数反例 | 无 |
| R3 / F-12 | 每个字符串重复解析整个 JSX 容器，扫描二次增长；新增 / P2 | R3 一次预计算结构、区间去重，`d0a6706…` | **已解决** | [结构预计算](../../tools/phoenix-ui/crates/rules/src/text.rs#L696)、[区间去重](../../tools/phoenix-ui/crates/rules/src/text.rs#L550)、[规模回归](../../tools/phoenix-ui/crates/rules/tests/text.rs#L412)；同一 400,932 B 输入 CLI 5.7386s→0.0477s、编辑门 5.8427s→0.0530s，见 §1.4 | 无 |

旧问题共 12 项，本轮均已解决：F-03/F-08/F-12 本轮关闭，其余 9 项维持关闭；不再适用 0 项，未复核旧 finding 0 项。R3 的证据限制逐项保留于 §2。

### 1.2 F-03 关闭证据

以原报告的自含 `cn = (...parts: string[]) => parts.join(" ")` 为前提，`cn(("focus:outline-none"), cn("focus-visible:ring-2"))` 在 base 报 1 条 warning，target 为 0；实际 `hook-before-edit` 对 `Save`→`Save now` 的无关文字编辑由 deny 变为 allow。`("focus:outline-none ") + extra` 与 `cn(("focus:outline-none"), extra)` 同样由误报/deny 变为 0 条/allow。把静态嵌套的替代改为 `mt-2`，target 仍报 1 条 warning，snippet 为完整 `focus:outline-none mt-2`，实际编辑门仍 deny。原括号拼接、数组 join、平坦 cn 的安全例维持 0 条。

另外以固定随机种子生成 504 个完全静态表达式组合，覆盖字符串、分组、带空格的拼接、嵌套 cn 与数组 join，target 的逐行命中与静态类列表结果一致（119 个应命中行）；200 个含未知变量、条件、插值或 helper 数据的组合为 0 条 warning。这些是有限的组合回归，不表示支持任意 JavaScript 求值。

### 1.3 F-08 关闭证据

原 `<p title='Avoid style="transition: all .2s"'>Guide</p>`、`data-example='style="transition: all .2s"'` 和 `<script>const tpl = "<style>.x { transition: all .2s; }</style>";</script>` 在 base 各 1 条 advisory，target 各 0 条。textarea/title 中的标签示例也不再命中。真实 `style="transition: all .2s"`、真实 `<style>` 块和 Vue `:style="{ transition: 'all .2s' }"` 继续命中；五种标记扩展名、属性引号、含 `>` 的框架表达式与未闭合 style 的回归通过。

### 1.4 F-12 关闭证据

使用 R3 报告同一合成输入：`export function App(){ return (<>\n`，N 行 `<button className="px-2 py-1" title="Save item {i}">Save</button>\n`，最后 `</>); }\n`。同机顺序运行已固定 hash 的 release base/target，所有 CLI 结果均为 `[]`、exit 0。

| 行数 / 字节数 | base CLI | target CLI |
| --- | --- | --- |
| 1,000 / 65,932 B | 0.1672s | 0.0333s |
| 3,000 / 199,932 B | 1.1928s | 0.0258s |
| 6,000 / 400,932 B | 5.7386s | 0.0477s |

6,000 行文件仅修改一处 title，实际 Cursor `hook-before-edit` 从 5.8427s 降至 0.0530s，均 allow。新旧比对与源码中的结构复用相符，原报告的大容器重复解析回退已消除；没有据此承诺所有源文件的耗时或任意表达式的线性复杂度。

### 1.5 本轮新增问题

未发现由本轮修复引入、证据充分且值得作者修复的新问题。

## 2. 待核实项与覆盖限制

- R3 留存的 V-9 浏览器操作权限、provider 产品观感与 UI 实拍限制仍保留。本轮未启动真实 provider 会话或独立 UI 验收，也未重新运行 URL/Chrome 矩阵；DOM 生产代码、fixture 与对应测试未被本轮修改，复跑其 FakeDom 回归并引用此前适用证据，不把它写成本轮浏览器通过。
- 本机为 darwin-arm64，其他平台与发布不在本轮运行验证范围；未穷尽 Unicode、动态类名、任意 JavaScript/框架语法。本轮是修复效果复审，不是整期原始实现的完整重审。
- 范围外观察：同一行两个正文撇号包住表达式时，例如 `export const a = <div><p>Don't</p><button className={cn("focus:outline-none")}>Save</button><p>you're done</p></div>;`，base 与 target 都没有报告该真实类名。该漏报在本轮基线已经存在，未列入 R3 的旧 finding，不作为本轮新增或 F-03 的误报残留；若继续处理，应另行固定该范围。M1 新增的跨行撇号回归已通过，不能据此声称所有撇号上下文均已覆盖。
- 完整 `npm run test:phoenix` 本轮未重跑。已读取实施方 `local/phoenix-ui/r3-fix-evidence/test-phoenix.log`，末尾 `EXIT=0`，108 组 Rust 成功结果、8,474 向量零失败与 860/860 oracle；并确认对应构建的 577 个输入与当前字节完全一致。该全量结果归为复用的实施证据；本轮独立验证见 §3.3。上述限制不阻塞本轮已选修复的判断。

## 3. 覆盖与验证

### 3.1 范围与固定版本

- 原始输入：`$review-code docs/plan/phoenix-ui-hallmark.md`。仅给计划，按开发计划模式选择最高轮次 R3，执行修复效果复审，另存 R4。
- 意图：修复 F-03 的完整嵌套表达式边界、F-08 的属性/原始文本误报和 F-12 的扫描性能回退，并同步开发产物与能力账本；来自 M1/M5 的明示处置目标。
- 仓库：`/Users/rockie/Documents/gh-xgent/skills`，main，HEAD=`035bbc690e695ebd06ebc6165d543d16bf5d4be2`，target tree=`31ab6288691bf3ded315117a4594e1638a4c5f38`。计划 SHA-256=`ac12011998bfd5963df420466f8bc99ede00394ded340306a8b72cc9171a4e05`。
- 计划登记的 `3e3885b78e08d09f9dfef2b5b78e88fa3131b115`、`e532b6011b30da6382e39bb4a8286e1557704bbc`、`5c2f0d766374d78202ae1c62aa1d364d068ca495`、`d0a67061b126ad25600cc52ccaff5c968a45f410` 全部解析成功且在当前分支上。前三项已被此前轮次覆盖；本轮以 R3 target 到当前 HEAD 的净差异归因。区间无未登记代码提交；`035bbc6…` 是记录更新，保留作契约上下文。
- 当前 index 干净；未提交 tracked 仅 `.xgent-ai/experience/index.md`，SHA-256=`1114ac749fb114a21ce1de2e4a5c9dce55e88c0895c4f8d29d88966906a7eaf5`；untracked 仅 `.xgent-ai/experience/3e3db19e-9909-4216-aa61-15b7d0104cb7.md`，SHA-256=`3decf9e8dcd0efa1d9da79c01c81ec3ac5b6d1da3c6c399fb2aa08a053d40cc7`。两者已读，都是经验记录，不是代码 findings 对象，未修改。
- 开始时 4,776 个 tracked/untracked 普通文件按相对路径映射 SHA-256，再以排序、UTF-8、无额外空格的 JSON 序列化求聚合：`0284ddad1cfe2f6ef906310c3effcfaad04ed924feead7e0cbccb4f58ea0d79e`。写报告前逐项重算差异 0，HEAD 未变；报告与随后台账为输出，不并入评审输入。
- 范围共 15 个提交内变更文件：生产 `crates/rules/src/text.rs`；文本/hook 测试；VERSION、ENGINE、GENERATED、capabilities；计划、M1/M5、R3 报告及四份台账。完整生产/测试 diff 已读；GENERATED 的 20 处变化逐键核对，全部仅为 ENGINE hash；文档、记录和台账作为契约与处置证据检查，不把自述当运行结果。

### 3.2 已审行为链

| 范围 | 本轮覆盖 |
| --- | --- |
| 文本规则 | 全部修复 diff、注释与类名扫描入口、字面量与结构索引、完整表达式/条件边界、静态求值、HTML 属性/样式/原始文本、行号及 snippet；旧值/变体回归 |
| CLI 与 hook | 原生文件扫描入口、before-edit 的项目/大小门及同步扫描调用、规则配置/双前缀 inline/advisory 过滤；真实 release allow/deny 输入及规模对照；未改生产接线 |
| DOM 与守则旧问题 | 确认生产代码、源守则及原测试未改，复跑 DOM 回归，核对源/生成守则与生成新鲜度；本轮未重新做视觉验收 |
| 构建与分发 | 全部清单变化；按原构建函数重算输入集合与聚合，核对分发引擎和 ENGINE/VERSION；bundle hash 保持不变 |

### 3.3 本轮实际执行与复用证据

独立命令运行于 target 字节；临时脚本、输入、固定引擎副本和日志均在隔离临时目录，未向源码添加测试或修复。

| 命令 / 方法 | 版本与结果 |
| --- | --- |
| 仓根 `npm test` | exit 0；Node 425 项：398 pass、27 skip、0 fail；Python 24 与 13 项通过，skills 2 项跳过 |
| tools/phoenix-ui 下 `mise exec --no-deps -- cargo test -p phoenix-ui-rules --locked --offline` | exit 0；文本 23/23、DOM 21/21，包含新规模回归 |
| 同目录 `mise exec --no-deps -- cargo test -p impeccable-hook --test hook_tests --locked --offline` | exit 0；78/78，含嵌套安全/未知 allow、嵌套缺替代 deny 和旧回归 |
| 同目录 `mise exec --no-deps -- cargo test -p phoenix-ui --test xgent_rules --locked --offline -- --skip url_scan_matches_the_fixture_expectations` | exit 0；8/8；1 个 URL/Chrome 用例明确未运行 |
| 仓根 `node scripts/check-phoenix-lints.mjs` | exit 0；No added diagnostics；保留既有 fmt 差异和 539 条 clippy 诊断 |
| tools/phoenix-ui 下 `mise exec --no-deps -- bun scripts/render.mjs --check` | exit 0；fresh，19 providers；input `0373592b…0069a`、bundle `a35570d9…ee95` |
| base/target release `detect --json <隔离输入>` | R3 具体误报输入均关闭，真实缺陷保持；504 个静态组合与 200 个未知值组合结果符合判据；范围外既有漏报单列于 §2 |
| base/target release `hook-before-edit`，Cursor preToolUse/Edit stdin | 嵌套安全、两种未知操作数 target allow；嵌套缺替代 target deny；平坦安全例维持 allow；规模数据见 §1.4 |
| 原 sourceInputs 函数独立重算 | 577/577 一致，聚合 `ad81b77b2f61b61ad39688e57f7bfcee12a15f0af17adc290207d5a057add2b6` = build-record.inputSha256 = VERSION.sourceInputSha256；ENGINE 与 VERSION 字节相同；分发引擎 hash 匹配 VERSION |
| 复用实施方完整 `npm run test:phoenix` | 对应当前 577 个输入，日志末尾 exit 0；108 组 Rust、8,474 向量零失败、860/860 oracle。不是本轮独立全量执行，适用性与限制见 §2 |

固定引擎身份：base release SHA-256=`ed72b03cf51e97f50b19f5cd8b2a48f5ae8a3c9c50eab653a5993fac8c5ed9eb`，与 R3 所审 target 的 VERSION 一致；target release SHA-256=`84b67a03d7b4821a0d124d36644d6c96b4a93b81209fa36df5cfbe46681edd57`，与当前 VERSION 及重算构建记录一致。临时证据根目录：`/var/folders/cv/pn7p1v7s737cqlt605jyl7lm0000gn/T/phoenix-hallmark-r4.jrd1mhyk/`；其中保存输入、指纹、binding、CLI/hook 结果、组合矩阵、timings 与日志，仅本机可用。复现所需输入、版本和关键结果已写入报告。

## 4. 本轮计划符合性

| 契约 / 本轮承诺 | 核查结果 |
| --- | --- |
| R-2、NFR-1；M1 的 F-03/F-08 修复，V-1/V-2 | 原报告的误报类别已关闭，真实缺陷保留；规则、CLI、实际编辑门与组合回归相符。§2 的基线既有漏报不归因为本轮引入 |
| R3 F-12；CLI/编辑前同步扫描 | 同一大 JSX 容器不再按每个属性字符串重解析；原输入的大幅性能回退解除 |
| C-3；M5 定稿后重建与来源绑定，V-8 | 输入、聚合、清单及实际分发字节一致，F-10 维持关闭；未手改 skill 源/守则，render fresh |
| C-2、NFR-2；内置行为与 oracle 精确期望 | 本轮未改内置规则、golden、adapter 期望或 DELTAS；复用的当前输入全量日志通过；未以期望更新掩盖新输入的错误 |
| R-3–R-7、provider/UI/其他平台及发布 | 本轮未改变这些接口、授权和守则；保留原证据及明确限制，不扩成本轮重新全面验收 |

计划需求来源为用户需求，无 PRD。本轮不修改原计划或里程碑进度。

## 5. 总体结论与下一步

**验收判定：通过验收（本轮声明的修复范围）。**

修复效果结论：**已解决**。R3 所列 F-01–F-12 共 12 项均有关闭依据，9 项维持关闭，3 项本轮关闭，不再适用 0 项。本轮新增 findings 为 0，修复净差异的正确性结论为 `patch is correct`。具体输入、真实 CLI/编辑门、组合回归与构建来源共同支撑该判断；这不表示已经完整重审整期原始变更或不存在 §2 所列范围外旧问题。

本轮代码复审完成，无需启动 apply-code-review。UI 实拍、真实 provider 观感、其他平台和发布仍按原范围分别验证；本结论不是合并或发布授权。若要继续核查 §2 的基线既有漏报，可显式选择完整重审：

```text
$review-code docs/plan/phoenix-ui-hallmark.md 完整评审
```
