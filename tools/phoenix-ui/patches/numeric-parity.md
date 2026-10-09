# Phoenix 数值一致性候选与精确差异

本候选只将 `foundation::js::math_pow` 的普通幂运算从平台 `f64::powf` 换成固定 Rust `libm::pow`，保留原有全部 ECMAScript guards。候选 native 已验证：8321 条历史调用中，27 条有限标量改变，其余 8294 条结果逐项完全相同；所有已记录规则结果不变。**候选 WASM 已真实重建，候选双端 8321 条历史调用、99 条边界调用及54条独立补充调用全部精确匹配 Phoenix 期望，0 双端差异，72个pure接口均有覆盖。**这证明所列数值与规则数据通过，其他 V-2 构建/平台和 M2 整体验收由主实施者汇总。

机器可读证据在 [numeric-parity.json](numeric-parity.json)，可审阅的最小源码差异在 [0003-shared-rust-pow.patch](0003-shared-rust-pow.patch)。历史 `tests/oracle/golden` 和 `tests/oracle/vectors` 未修改；内部 `impeccable-*` crate 名称保留。

## 根因与选择

固定来源 `508d7e8955de3b3caf2d8676e85206723d41a887` 的 `math_pow` 最终调用 `x.powf(y)`。`relative_luminance` 的 sRGB 非线性分支经该函数计算指数 `2.4`，`contrast_ratio` 再使用所得亮度。原 native 的 8321 条调用全部精确匹配历史期望，原 WASM 有 27 条不同，全部位于这两个函数；不是可忽略的“通过”。源码定位：`crates/foundation/src/js.rs:647`、`crates/foundation/src/color.rs:185`、`:199`。

Rust 明确说明 `powf` 的精度可能随平台和 Rust 版本改变，不能作为双端精确一致的保证。[Rust f64::powf](https://doc.rust-lang.org/std/primitive.f64.html#method.powf)。ECMAScript 对普通有限幂允许实现近似，但要求 NaN、零、无穷和负底数等特例遵循指定语义；尤其 `1 ** NaN`、`±1 ** ±Infinity` 必须是 NaN。[ECMAScript Number::exponentiate](https://tc39.es/ecma262/2024/multipage/ecmascript-data-types-and-values.html#sec-numeric-types-number-exponentiate)。

| 方案 | 范围与判断 |
| --- | --- |
| 固定 `libm = 0.2.16`、禁用默认 features | 已选。只替换末尾调用，双端使用同一 Rust 软件内核；27 条新值逐条登记。 |
| 内置一份 fdlibm pow 源码 | 需要同时维护 pow、scalbn、sqrt 等依赖与来源通知，变更大于引入现成固定 crate。 |
| 保留平台 powf | 原 27 条精确不一致仍存在，无法满足本次双端比较。 |
| 统一舍入或整体误差容忍 | 改变所有调用的验收语义，可能掩盖真实阈值分类变化，不采用。 |

这次普通有限数值内核的选择是明确的 Phoenix 行为变更，不能据 ECMAScript 允许近似便宣称与原 JavaScript 数值完全相等，也不能证明任意连续 CSS 输入的旧、新分类永远相同。

## 固定依赖与最小补丁

`crates/foundation/Cargo.toml` 的新增依赖由主实施者写入：

```toml
libm = { version = "=0.2.16", default-features = false }
```

本地已读 libm 0.2.16 缓存包的 [Cargo manifest](https://docs.rs/crate/libm/0.2.16/source/Cargo.toml)，确认版本、MIT license、Rust 1.63 要求及默认 `arch` feature。禁用默认 feature 避免按目标选择该 feature 的优化实现；[pow 源文件](https://docs.rs/crate/libm/0.2.16/source/src/math/pow.rs) 是 FreeBSD/fdlibm 来源的 Rust 软件内核。[固定版本 pow 接口](https://docs.rs/libm/0.2.16/libm/fn.pow.html)。crate 下载包 SHA-256 为 `b6d2cec3eae94f9f509c767b45932f1ada8350c4bdb85af2fcab4a3c14807981`，与 `Cargo.lock` checksum 相同。pow 源文件和完整许可文本的 SHA-256 另列于 JSON 的 `dependency` 字段。

补丁 SHA-256 为 `ef3acb2392e2ec4df1e4d4a1895fe51cb2de13e79faba701677635a66b4b8be6`。补丁前、后 `crates/foundation/src/js.rs` 的 SHA-256 分别为 `f686e0ff3d6d4448ceae5d5f9d6f47e084e80b847bd6133b999522eb2dd1a6a2`、`73be58b25457470055db9896cf28efc1481fb7bee1c63243fa7505208d3fd9b8`；前者与 `UPSTREAM.json.importedFiles` 相同。轻量测试在内存逆向重建固定来源，核对完整文件 hash，并确认函数体只变更末尾调用。

原 `y.is_nan()`、`y.is_infinite() && (x == 1 || x == -1)`、平方快捷路径、平方根快捷路径均未改动。新增 34 条 Rust 特例使用 `is_nan` 或 `to_bits` 精确判定，覆盖 `NaN ** ±0`、`1 ** NaN`、`±1 ** ±Infinity`、正负零、奇偶指数、正负无穷、负底数小数指数、最小 subnormal、上溢和下溢。新增源表位于 `crates/foundation/src/js.rs:825`；JSON 的同一张表再与 Node `Math.pow` 特例逐条精确对照。

准备事项：主实施者已同步 Cargo 精确锁；可离线使用现有 crate 缓存。发布的依赖/SBOM 和 third-party notice 仍须纳入 libm 的完整 MIT 文本，并保留 pow 源头的 Sun 许可通知。此处没有复制整包 0.1.12，也没有修改其他 crate、历史期望或生成资源。

## 独立 Phoenix 期望

| 历史调用函数 | native 相对历史期望改变 | 规则结果改变 |
| --- | ---: | ---: |
| `shared.color.relativeLuminance` | 13 | 不适用，输出标量 |
| `shared.color.contrastRatio` | 14 | 不适用，输出标量 |
| 其余全部历史调用 | 0 / 8294 | 0 |

JSON 的 `overrides` 逐条保存 `file`、1-based `line`、0-based 全局 `index`、完整 `args`、参数 SHA-256、来源文件 SHA-256、旧/新 double 和旧/新二进制位。全局 index 按实际 `pure_functions()` 的表顺序遍历，不能当作独立键。集成时必须同时验证来源 hash、行和参数，再替换该条期望；未匹配项、重复项或未使用覆写应失败。其余调用继续使用原历史结果精确比较，不使用全函数、全 case ID 或统一误差豁免。

轻量测试从实际 foundation/core dispatcher 常量构造表，读取全部历史调用，验证 27 条覆写的来源和旧值。可选真实 runtime 测试对完整 8321 条调用和 99 条新增边界逐项比较 native、新 WASM、独立 Phoenix 期望。测试默认不要求本地 Rust 或 WASM 产物，默认通过不能冒充真实 runtime 通过。

## 阈值与影响边界

生产调用点还包括 sRGB encode/decode（`crates/foundation/src/color.rs:408`、`:419`）、Lab 反变换（`:476`）、浏览器截图对比度（`crates/browser/src/screenshot_contrast.rs:120`）。对比度阈值来自 `crates/core/src/checks/rules.rs:266`、`:350` 的 `3` / `4.5`；glow 与 CSS root 背景阈值来自 `crates/core/src/checks/rules.rs:855`、`crates/core/src/checks/css_scan.rs:120` 的亮度 `0.1`。

99 条 probes 对每个中心使用 `-8,-2,-1,0,1,2,8` 个 double ULP，以及 `±1e-6`，记录旧 native、原 WASM 和候选 native 的实际完整结果：

| 输入组 | 数量 | 实际覆盖 |
| --- | ---: | --- |
| 灰阶对比度 `3` / `4.5` | 18 | 两侧标量与严格 `<` 分类 |
| `checkColors` / `checkHoverContrast` | 36 | 两种字号阈值的命中与不命中 |
| 背景亮度 `0.1` / gamma 分支 `0.03928` | 18 | 亮度阈值与线性/非线性输入两侧 |
| 非零偏移彩色 glow | 9 | 实际受 `on_dark_bg` 支配的分支；零偏移会无条件命中，未用作阈值证明 |
| CSS root RGB 取整边界 `89.5` | 9 | `parse_any_color` 先 round 通道（`crates/foundation/src/color.rs:873`），实际解析到 89 / 90 后两侧分类 |
| Oklab 转 8-bit RGB 中点 `127.5` | 9 | encode gamma 与最终 127 / 128 舍入 |

灰阶中心由 sRGB 反变换构造：对比度使用 `L = 1.05 / threshold - 0.05`，背景使用 `L = 0.1`，再计算 `255 * (1.055 * L ** (1/2.4) - 0.055)`；gamma 中心为 `255 * 0.03928`。Oklab 中性灰中心为 `(((127.5/255 + 0.055)/1.055) ** 2.4) ** (1/3)`。JSON 保存最终 double 参数，复测不依赖重新估计中心。

旧 native → 候选 native 在新增 probes 有 1 条亮度标量末位改变（`dark-luminance/+1e-6`）；所有规则完整结果和亮度/对比度分类变化为 0。原 WASM → 候选 native 的 99 条结果精确差异为 0。这些是所列数据的实际结果；尚未证明任意输入、其他 CPU/OS 或真实浏览器截图结果均相同。原 10 个无历史向量函数中的 `checkHoverContrast`、`cssTextHasDarkRootBg` 在这99条里有边界，另外8个在下述独立补充里验证。

## 十个接口的独立补充

JSON `supplemental` 保存54条 **Phoenix specification vectors**。这些是依据已读接口和具体可观察决策手写的 literal 期望，未从候选输出生成，也不是重录 JavaScript；固定源码已无可运行的原 JS engine。历史vectors/golden未增删。默认轻量测试要求此清单恰好补齐10个无历史录制接口，并与实际 dispatcher 的72个接口集合精确相等。

| 接口 | 条数 | 输入和结果的关键覆盖 |
| --- | ---: | --- |
| hwbToRgb | 6 | RGB主色、白、黑、whiteness+blackness=1的128舍入 |
| enclosingCssSelector | 5 | 空、规则内、闭合后、comment内花括号、UTF-16 emoji selector |
| isZeroOffset | 5 | null、0、0px、trim后的-0rem、0.0文本不命中 |
| collectPulseKeyframes | 5 | 空map、opacity、translation、重复名称升级、map插入顺序 |
| stripReducedMotionBlocks | 5 | 空、普通CSS、reduce移除、no-preference保留、嵌套和大小写 |
| checkHoverContrast | 6 | 缺options、两种字号低对比、充分对比、emoji、透明safe tag |
| resolveHeroHeadingSizePx | 5 | null、px、rem、clamp边界、未支持vw |
| cssTextHasDarkRootBg | 6 | 空、全文dark fastpath、root黑/白、custom property、alpha=0.5 |
| isRoundDotRadius | 6 | 空、39/40%、尺寸3.9/4px、999px例外 |
| buildHtmlPatternCorpora | 5 | 空、plain CSS、style+inline、class顺序、uppercase style |

严格初跑纠正了两个手写 hover snippet 的显示格式：固定源码 `crates/core/src/checks/rules.rs:360` 使用 `to_fixed(ratio,1)`，所以整数对比度仍写 `1.0:1`。JSON `literalCorrections` 记录了依据；finding和阈值分类没有改变。

`cssTextHasDarkRootBg` 名称不能当完整语义：固定源码 `crates/core/src/checks/css_scan.rs:106` 先对全文运行 dark/tailwind fastpath，`.card{background:#000;}` 因此三端都是true，这在补充中明确登记。另已严格确认原有 parser 限制：`:root{--canvas:#000;}body{background:var(--canvas);}` 三端都false，但插入空格后均true；collectCssCustomProps两者都正确得到--canvas=#000。`ROOT_BLOCK_RE`（同文件`:82`）消费前一closing brace，非重叠 captures 无法复用此delimiter匹配紧邻body。该逻辑缺口保存于 `supplemental.knownLimitations`，不修改生产代码、不伪装为libm回归或 detector 正确性通过；补充的custom-property正例使用实际有分隔的有效分支。

54条补充已经在候选 native/WASM 精确运行，0差异；另显式调用固定baseline native也全部匹配。这份历史证据不会成为默认测试的隐性local目录依赖，默认真实runtime只使用显式提供的三个候选产物路径。

## Italic 单项差异的严格分类

目录 case 多出的 `italic serif h1 (fraunces) at 72px "Inline Em Inside Roman"` 是 **stale-directory-golden**。固定来源的单文件 JSON golden 已包含该项，8 findings；目录 golden 针对此文件只有 7 findings，集合差异精确为这一项，没有反向差异。JSON `italic` 保存两份 golden 的固定 SHA-256 和完整 finding，轻量测试从原文件重算差异。

该 fixture 的 `tests/fixtures/antipatterns/italic-serif-display.html:146` 是 roman h1 内嵌 italic `<em>`，下一行明确要求检出。固定 adapter `crates/html/src/adapters.rs:706` 遍历实际拥有文字的子节点并使用其 computed typography；DOM 对应实现是 `crates/core/src/browser/element_checks.rs:559`。单文件与目录 case 使用同样 `--no-config --json`，仅目标路径不同（`tests/oracle/cases/detect.mjs:25`、`:36`）。

原 isolated baseline binary（SHA-256 `e99441d027be9fc00ff25da74bb1371f30cf17ece270327bab14dc9ce76c7d31`）实跑单文件得到 exit 2、8 findings、空 stderr；仅归一 `<REPO>` 前缀后完整结果精确匹配单文件 golden。原目录实际 437 对历史 436，完整 added finding 与上述集合差异一致。这些证据支持固定目录 golden 漏录，不支持关闭 detector、撤销内嵌 italic 规则或忽略整个目录 case。

`tests/oracle/DELTAS.md:76` 明确历史段落不再 accepted，但原 `tests/oracle/run.mjs:34` 的 parser 只提取所有 bullet ID，误收已失效旧记录。集成时应修复该语义，并仅登记此完整单项差异；本候选不改历史 golden 或该 runner。

## 已执行与待执行验证

已执行：固定构建工具 preflight / mise 下 `cargo test --locked --offline -p impeccable-foundation --lib`，38 pass、0 fail，含 34 特例；`cargo build --locked --offline -p impeccable-wasm --example replay_vectors --message-format=json` 成功。候选 native、原 native 和原有效 WASM 的 8321 / 99 调用结果、产物 SHA-256 见 JSON。历史模块计数为 `shared.color` 1729、`shared.inline-ignores` 118、`rules.checks` 6474；后者完整输出差异为 0。轻量测试原6项通过后新增supplemental合同，当前7项；候选 debug native与原WASM的过渡8420复验、首份候选WASM的8420复验均7 pass，作为历史结果保留，不覆盖原artifact SHA。当前完整8474复验为8 pass、0 fail、0 skip。

真实双端复验入口：先用固定工具重建候选 native `replay_vectors` 与带 pure exports 的 WASM，再运行：

```sh
PHOENIX_UI_NUMERIC_VECTORS_BIN=/absolute/path/to/replay_vectors \
PHOENIX_UI_NUMERIC_WASM_BINDINGS=/absolute/path/to/impeccable.js \
PHOENIX_UI_NUMERIC_WASM_MODULE=/absolute/path/to/impeccable_bg.wasm \
node --test test/phoenix-ui/numeric-parity.test.js
```

三个 runtime 环境变量必须同时提供；任一缺失失败。当前最终复验使用 `tools/phoenix-ui/target/debug/examples/replay_vectors` 和独立 `tools/phoenix-ui/target/wasm-verification/` 的候选 pure bindings/WASM：8 pass、0 fail、0 skip，8474次调用精确一致。WASM对应当前source version0.1.0；native复用获授权的libm候选debug产物。新证据是 JSON `artifacts.currentVerification*` / `validation.currentVerification`，旧8420产物SHA仍保留。production的wasm-bundle与pure验证目录分开，production bundle不含pure_call不可拿来向量复验。全仓 `npm run test:phoenix`、浏览器/其他 native 平台/真实 provider session 验收由主实施者继续，未执行者不写通过。这里不更改主 checker、生成资源或全仓配置。
