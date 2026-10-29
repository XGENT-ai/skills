# PHOENIX-HALLMARK · 代码评审 R1

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1，首次完整代码评审 | [Phoenix UI 借鉴 hallmark](phoenix-ui-hallmark.md) | `886efe201c58da77e9fa223ca780f5be1101eff4..3e3885b78e08d09f9dfef2b5b78e88fa3131b115`，累计净差异；开始时工作树及 index 干净，无未跟踪文件 | 无；同名 `.review.md` 是开发计划评审，不作为代码复审前轮 |

评审 ID：`1e55e17d-4399-4289-bd1c-131954c5208e`。

计划按当前 HEAD `0be853c94ed4b0db1220147ef9a18b12558fed5e` 读取，内容 SHA-256 为 `e2a1c5d71290caaf2afdcfd92f6d93d9f3b817a950513ec0107308ea96fec5ce`。该提交相对实现提交仅修改计划与对应状态页，运行代码、测试、构建脚本及生成物与所审 target 相同。下文当前文件的代码行号均可用于该 target；新增报告和本次台账回写不在审查输入内。

## 1. 确认问题

本轮确认 8 项 P2、2 项 P3；均由本次变更引入。P3 不独立阻止验收。编号仅属于本报告，不沿用开发计划评审的同号问题。

### F-01 · [P2] 扫描焦点类名之前排除注释

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 139–143 行。
- 问题：Tailwind 分支在原始 `content` 上遍历引号，没有使用上文去注释后的文本，也没有排除 JS 行注释。只要 TSX 注释写了 `className="focus:outline-none"` 这个反例，实际组件即使完全没有移除焦点轮廓，也会产生 warning；这会使 `detect` 退出为 2，并让 Cursor 拒绝该文件的无关编辑。规则默认启用后，说明性注释本身就会阻断正常开发。
- 证据：本轮 `comment-only.tsx` 与 `block-comment.tsx` 均在注释行报 `xgent/focus-outline-removed`；对前者发起 `export const` → `export let` 的 `hook-before-edit`，实际返回 `{"permission":"deny",…}`。CSS 块分支先去注释（105–114 行），无法保护随后直接扫描原文的类名分支。与计划 [NFR-1](phoenix-ui-hallmark.md) 第 81 行的误报门槛不符。
- 修复与验证：在保留行号的前提下过滤相关语言的注释，再识别类名；验证 `//`、`/*…*/`、JSX 注释中的示例均不报，真实同名类仍报，Cursor 对注释文件的无关编辑放行。
- 置信度：高；源码因果链、CLI 与 Cursor 实际输出一致。

### F-02 · [P2] 区分 focus 变体和 focus-within 等其他条件

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 215–219 行。
- 问题：`variant.to_lowercase().contains("focus")` 将所有含该子串的变体当成焦点本身，包括 `focus-within` 和 `not-focus`。例如非可聚焦的包装 `div` 使用 `focus-within:outline-none`，不会移除其内部 input 的键盘焦点轮廓，当前规则仍报 warning 并让 Cursor 拒绝无关编辑。
- 证据：`focus-within.tsx` 只有包装 div 上的上述类，CLI exit 2，Cursor 返回 deny；隔离 Chrome 对等 CSS 中，input 获焦后实际为 `outlineStyle=auto`、`outlineWidth=1px`。CSS 分支 119 行已明确排除 `:focus-within`，类名分支却没有同等边界。
- 修复与验证：按变体语义识别 `focus`、`focus-visible` 与计划允许的任意选择器/数据变体，避免纯子串命中；加入包装控件的 `focus-within`、反向焦点条件和真正焦点变体的对照用例。
- 置信度：高；误报、编辑拒绝与浏览器计算样式均已复现。

### F-03 · [P2] 在完整的静态类名表达式中寻找焦点替代

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 139–143 行。
- 问题：每一个引号片段都被独立判断为完整类列表，导致正常的静态字符串拼接被误判。`className={"focus:outline-none " + "focus-visible:ring-2"}` 最终包含可见焦点环，与单个字符串的 DOM 结果相同；当前实现只检查第一个片段就报“没有替代”，使 CLI 失败并将这种常见的类名组合方式带入 warning 级 hook 检查。
- 证据：`concatenated-classes.tsx` 实测 exit 2、1 条 `xgent/focus-outline-removed`；等价的单串 `className="focus:outline-none focus-visible:ring-2"` 为 exit 0、无该发现。两者唯一差别是静态字符串表达式的拆分。
- 修复与验证：至少对可静态确定的拼接/类名组合统一判断；无法判断完整表达式时避免断言“没有替代”。验证单串、静态拼接及常用类名组合的等价输出，并保留真正缺替代的正例。
- 置信度：高；输入是确定的静态 JSX 表达式，且对照结果已复现。

### F-04 · [P2] 按完整的 CSS 长度值识别零轮廓

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 45–47 行。
- 问题：用 `0\b` 代表零宽度，会同时造成相反方向的错误：合法的 `outline: 0px` / `outline-width: 0px` 因 `0` 与 `p` 之间没有词边界而漏报；`outline: 0.5px solid black` 却在小数点前满足词边界，被当作轮廓已移除而误报。前者遗漏本规则要发现的缺陷，后者又会触发 warning 和编辑拦截。
- 证据：`zero-unit.css`、`zero-unit-style.css` 均 exit 0、无 xgent 发现；`fractional-outline.css` exit 2、报该规则。Chrome 对后一输入的实际计算值是 `outlineStyle=solid`、`outlineWidth=1px`，并未移除轮廓。
- 修复与验证：判断完整的零值长度与关键字，不能只匹配数值前缀；覆盖 `0`、`0px`、`0rem`、零小数、有单位非零小数及 `outline-width`，同时验证行号与 hook 行为。
- 置信度：高；正则边界、CLI 输出与浏览器结果相互印证。

### F-05 · [P2] 接受 SVG 的 title 属性所提供的可访问名称

- 定位：[dom.rs](../../tools/phoenix-ui/crates/rules/src/dom.rs)，target 侧 255–258 行。
- 问题：命名豁免只检查 ARIA 属性和子 `<title>`，遗漏 SVG 自身的 `title` 属性。`<svg title="Company logo" width="24" height="24">…</svg>` 已有 Chrome 可访问名称，仍被当前规则断言“has no accessible name”，作为 warning 使 URL 检测退出为 2。这是对实际已命名图形的误报。
- 证据：Chrome 156.0.8078.48 的 `Accessibility.getPartialAXTree` 对 `svg-html-title` 返回 `ignored=false`、role=`image`、name=`Company logo`，名称来源为 `title` 属性；同一页面的当前引擎仍报 `xgent/unlabeled-inline-svg`。Deque 的 [axe-core 4.12 官方规则说明](https://dequeuniversity.com/rules/axe/4.12/svg-img-alt) 也列出了 `title` 属性的命名方法。
- 修复与验证：将有效的 `title` 属性纳入命名来源，或使用等价的可靠名称判断；加入属性 title、子 title、ARIA 名称及确实无名称的真实 Chrome 对照，保持后者仍报。
- 置信度：高；直接比较同一 DOM 的浏览器可访问树与规则输出。

### F-06 · [P2] 在应用 emoji 数量门槛前去除同一图标的重复候选

- 定位：[dom.rs](../../tools/phoenix-ui/crates/rules/src/dom.rs)，target 侧 403–407 行。
- 问题：父元素的 `text_content` 包含子元素文本，而 `heading_after` 同时支持自身后继标题和父元素后继标题，因此 `<div class="icon"><span>🚀</span></div><h2>Feature</h2>` 会让 div 与 span 同时成为候选。实际只有一个 emoji，也会通过“至少两个”的门槛并报两条；这直接破坏了计划要求的“单个不报”，对常见的包裹结构产生误报。
- 证据：真实 Chrome 下，无包裹的单个图标为 0 条；只增加 span 包裹后为 2 条相同 `xgent/emoji-icon`；两个裸图标的对照为 2 条。现有 `emoji_icon_accepts_an_only_child_before_a_heading` 只给子元素设置字号，未模拟真实页面中父子均满足字号阈值的状态。
- 修复与验证：选定真正拥有该 emoji 文本的节点，或按同一渲染图标去重，再判断数量；验证一个包裹图标不报、两个包裹图标恰好两条、裸元素场景保持一致。
- 置信度：高；最小 DOM 变体已在真实浏览器复现。

### F-07 · [P2] 将视觉规则的可见性与 ARIA 隐藏状态分开

- 定位：[dom.rs](../../tools/phoenix-ui/crates/rules/src/dom.rs)，target 侧 309–312 行；同根因另见 389–391 行。
- 问题：窗口圆点和 emoji 等视觉检查复用了 `is_rendered_for_browser_rule`，该函数遇到自身或祖先的 `aria-hidden="true"` 就返回 false。装饰性圆点和 emoji 经常正确地从无障碍树隐藏，但仍原样呈现在页面中；给窗口栏加这个属性就能让新规则完全漏检，两个可见 emoji 也同样消失于检测结果。属性没有改变规则所针对的视觉结构。
- 证据：以仓库 `chrome.html` 为对照，只给 `.bar` 增加 `aria-hidden="true"`，结果从 1 条 `xgent/redrawn-window-chrome` 变为 0；两个图标各加同一属性，结果从 2 条 `xgent/emoji-icon` 变为 0。独立 Chrome 探针确认 `checkVisibility()=true`。根因是 [element_checks.rs](../../tools/phoenix-ui/crates/core/src/browser/element_checks.rs) 198–204 行的 ARIA 过滤；计划 §5.3 对这两条视觉规则要求的是可见性。
- 修复与验证：在规则包内为视觉检查使用实际渲染可见性；SVG 可访问性规则继续尊重 ARIA 隐藏。验证视觉相同的 ARIA 隐藏版本仍命中，`display:none` 等真实不可见版本不报；无需改内置规则或快照合同。
- 置信度：高；只改一个属性的对照与浏览器可见性结果已取得。

### F-08 · [P2] 不把 HTML 正文中的 CSS 示例当作 transition 声明

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，target 侧 94–96 行。
- 问题：对 HTML 原文直接使用声明正则，会把普通正文 `<h4>transition: all</h4>` 当作真实动画样式。一个完全没有 CSS、没有任何过渡的说明页也会报 `xgent/transition-all`。本次 oracle 登记还将仓库已有的同类正文误报接受为新期望，导致兼容回放通过掩盖了这类准确性错误。
- 证据：最小 `transition-copy.html` 没有 style/class/脚本，仍实际产生 1 条 advisory。新写的 [DELTAS.md](../../tools/phoenix-ui/tests/oracle/DELTAS.md) 第 316 行明确把 `motion.html:149` 的字面标题例子计入发现；[M3 记录](phoenix-ui-hallmark.records/M3.md) 将这批差异全部判为真命中。计划 NFR-1（第 81 行）要求误报类别处置后才能收口。
- 修复与验证：限制声明分支到对应源语言的样式位置，正文示例不报，真实 `<style>` / style 属性 / 类名继续报；按既有精确 patch 机制修正受影响 oracle case、计数与 DELTAS，不重录 golden。
- 置信度：高；最小输入与已登记的仓库实例同时证实。

### F-09 · [P3] 将控件换行指引与新标签规则保持一致

- 定位：[operate.md 源文件](../../skills/phoenix-ui/src/reference/operate.md)，target 侧第 51 行；生成的 `reference/operate.md` 有同样文字。
- 问题：新增表单守则建议 `let the label wrap`，但本次新增的 `xgent/wrapped-control-label` 正是将按钮、tab、导航标签的自动多行作为 warning。Agent 按守则让文字自然折行，会立即在同一轮 craft-floor 页面检测中被判缺陷；因此守则给出了无法通过本期自检的默认修法。计划评审已明确建议重排或移动控件，而不是靠 nowrap/截断。
- 证据：[dom.rs](../../tools/phoenix-ui/crates/rules/src/dom.rs) 111–121 行对两行标签报 warning；本轮 80px 宽、16px 字号的 `Save changes` 按钮在真实 Chrome 中报 `label wraps to 2 lines`，exit 2。[计划评审 F-23](phoenix-ui-hallmark.review.md) 的修订方向是重排或移动控件；项目/本地化的显式例外仍可按既定优先级处理。
- 修复与验证：明确让控件/控件所在行重新排布，避免将“标签文字自动折行”列为默认解决方案；经生成流程同步各 provider 文本，用窄宽度场景验证指引给出的修法能通过对应检测，并保留本地化例外。
- 置信度：高；守则文本和本次规则的实际输出直接冲突。

### F-10 · [P3] 在最后一次测试文件修改后重建开发产物的来源绑定

- 定位：[VERSION.json](../../dist/phoenix-ui/VERSION.json)，target 侧第 9 行；同值传播到 `skills/phoenix-ui/scripts/ENGINE.json` 与能力账本。
- 问题：开发清单的 `sourceInputSha256` 仍是 `4953165a…ccaa28`，按构建脚本相同输入集合、排序、序列化和 SHA-256 算法重算所审提交却为 `61974763…dd42fe`。打包后修正 `crates/rules/tests/text.rs` 的 registry 断言没有重建最终产物，导致交付的来源绑定无法在该提交复现。差异仅在测试文件，不据此声称生产二进制行为过期，但“构建绑定了当前完整输入”的证据不成立。
- 证据：[构建脚本](../../scripts/build-phoenix-ui.mjs) 42–73 行将 `crates` 下的测试也列入 `sourceInputs()`。本机 `development-release/build-record.json` 的 577 个输入逐项比较，唯一差异是 `crates/rules/tests/text.rs`：构建时 hash=`192c2fcc2b84f3999b157294bb41f12b767c978c350501190e3f690e16938a84`，target hash=`d0b4c25e5bc56bf26212dd0e93bd4b33080e63aa88648cef831b08ec7b02ec69`；[M5 记录](phoenix-ui-hallmark.records/M5.md) 的偏差段也说明在完整回归发现后修了该断言。
- 修复与验证：最终改动定稿后通过既有开发构建流程同步清单及其绑定，再重算 `sourceInputs()` 与清单值比较；不要只手改 hash。本轮所有生产规则复现均另用当前源码重建的 debug 引擎，未依赖这份过期绑定。
- 置信度：高；精确到唯一变化输入，并与实施记录吻合。

## 2. 待核实与覆盖限制

- 本轮完整 `test:phoenix` 主动结束于耗时较长的既有迁移矩阵，不能宣称整套运行通过；已执行部分和后续定向回归分别列在 §3.3。原 M5 的整套通过记录仅按其原有范围保留。
- 未在本轮重新发起付费 provider 行为会话。已读取 M4/M5 记录及本机 V-9 证据索引，原记录注明浏览器截图/渲染操作曾被权限拒绝；不能把它们记为视觉验收通过。本轮 Chrome 检测、计算样式与可访问树探针也不等于 UI 实拍或真人读屏验证。
- 浏览器证据覆盖本机 darwin-arm64 / Chrome 156.0.8078.48；其他平台、真实辅助技术和发布验收不在本轮结论内。
- 本轮未将 emoji 的所有 Unicode 序列、任意动态 JS 类名表达式或所有 SVG 可访问名称路径宣称为穷尽验证。确认问题都有具体已复现输入，不用这些未穷尽范围推导额外 findings。
- 本报告关注实现行为；未重新评审计划架构，未把后续 WASM 规则包接入、发布或跨平台验收要求当成本期遗漏。

## 3. 覆盖与验证

### 3.1 基线与范围

- 变更意图：按用户给定计划，在原生 Phoenix 引擎加入八条可独立关闭的 XGENT 规则，并同步设计/参考守则；这是计划明示的意图。
- 仓库：`/Users/rockie/Documents/gh-xgent/skills`；日期：2026-10-23（Asia/Shanghai）；分支 `main`。
- 输入：`$review-code docs/plan/phoenix-ui-hallmark.md`；采用开发计划模式。完成记录的 M1–M5 均指向 `3e3885b`，解析为表头 target，且可从当前 HEAD 到达。
- 实际比较：target 的第一父提交 `886efe201c58da77e9fa223ca780f5be1101eff4` → target；target tree=`2cbce7525295504dc739191c0c262eadac6370b6`。没有额外代码提交夹杂于已登记提交与当前 HEAD 之间。
- 排除：`0be853c94ed4b0db1220147ef9a18b12558fed5e` 仅回写计划/状态；同一实现提交内的计划、计划评审、里程碑记录与台账用于契约和证据核对，不作为代码 findings 的主对象。既有 `886efe2` 本身不在本轮差异内。
- 工作树/index：初始 clean，无 staged、unstaged、untracked；验证用当前 checkout，已确认运行代码/脚本/测试与 target 字节相同。隔离复现与日志置于 `/tmp/phoenix-hallmark-review.2d2ziA/`，没有向目标源码增加测试或修复。
- 版本复核：落盘前重算最初记录的 tracked 输入 SHA-256，全部与初始值一致；HEAD 未变。报告及登记文件在其后写入，作为本轮输出单列。

### 3.2 已覆盖行为链

| 范围 | 已核查内容与结果 |
| --- | --- |
| 原生接线 | workspace/Cargo.lock、`cli/main.rs` 的 install/Engines/hook 分发；`detect` 的 ScanOptions；hook Runtime/HookScanOptions、文件扫描和 Stop 原始内容基线。未发现包只接入当前扫描而遗漏基线的缺陷 |
| 规则实现 | `crates/rules/{src,tests}`、八条 ROWS 元数据、十三个正反例 fixture、CLI/hook 新测试；结合原生 Dom/快照/可见性实现核对真实调用，确认 F-01–F-08 |
| 过滤与兼容 | 配置 ignoreRules、value 过滤、双前缀 inline directive 与 no-advisory；现有无包测试及新增二进制/hook 测试；内置注册表与快照捕获合同无本次代码改动 |
| skill 与参考 | 五个 Phoenix 源文件、scenarios 10–14、design-reference 的 SKILL/evals 增量，生成正文与 provider bundle 的对应传播；确认 F-09，未发现名称/参数/description 接口变更 |
| oracle | 结构化比较前后 adapter：48 个 expectation entry 的元数据/内容有变化，真正改变预期输出的是 13 个 detect case；其 exit/signal/files 均不变，JSON case 去除新增 xgent 对象后的内置结果不变。F-08 表明新增期望仍需准确性复核 |
| 构建与来源 | VERSION/ENGINE/GENERATED、bundle manifest/blob、WASM bundle 输入与输出绑定、capabilities、UPSTREAM、ownership、依赖清单；对生成内容按源文件映射、hash 和现有检查核对，另发现来源绑定偏差 F-10；不将内嵌 WASM base64 的文本 diff 当作语义评审 |

本轮由当前主 agent 独立完成；未使用或声称有其他 reviewer。

### 3.3 本轮实际运行

| 验证 | 目标版本/方式 | 实际结果与限制 |
| --- | --- | --- |
| `npm test` | 当前 checkout，运行代码与 target 相同 | exit 0；Node 425 项：398 pass、27 skip、0 fail；Python 24 项及 13 项通过，skills 组 2 项因 PyYAML 缺失跳过。不是“全部检查均执行” |
| `npm run test:phoenix` | 读取既有 prepare 回执中的 WASM_BINDGEN/WASM_OPT/PHOENIX_CARGO_DENY；由原脚本构建当前源码并运行 | 未完成：主动终止于 `fixed_release_148_provider_channel_inputs_prepare_strict_migration_plans_readonly` 旧版本迁移矩阵；此前 Node installer 11/11、foundation/core/detect/html/hook 等已执行部分及 Rust installer 15/15 通过。没有记作整套通过；直接受影响范围随后单独检查，见下行 |
| 本次变更的定向回归 | `cargo test -p phoenix-ui-rules`、`cargo test -p phoenix-ui --test xgent_rules`（均 `mise exec --no-deps`、`--locked --offline`），随后 lint、WASM 与严格 oracle | exit 0：文本 10/10、DOM 18/18、CLI 9/9（含真实 Chrome fixture×宽度）；lint 无新增诊断；WASM/native 向量 8,474 个，golden/parity 失败均为 0；严格 oracle 860/860（487 exact adaptations），fail/error/missing/platform skips 均为 0。完整命令和输出在 `targeted.log`；这些结果不扩成完整套件通过 |
| 隔离文本与 Cursor 复现 | 本轮构建的 debug 引擎固定副本，SHA-256=`25a015397f8059157005778f6e5e5e8e2ea1ba57faf446279548e7f92b1973d2` | 已实际执行注释、变体、静态拼接、CSS 数值及真实正例对照；结果见 F-01–F-04/F-08。复现脚本/JSON 在上述临时目录，不加入源码 |
| 隔离 Chrome URL 对照 | 同一 debug 引擎，`detect --json --viewport 375x812 file://…` | 单/双 emoji、包裹结构、ARIA 隐藏、真实 display:none、带 title 的 SVG、两行控件标签，结果见 F-05–F-09；不会把 CLI exit 0 当作没有 advisory |
| Chrome CDP 样式/可访问树 | 独立临时 profile，本机 Chrome；仅载入本次本地测试页 | 记录 input 原生焦点、非零小数 outline、ARIA 隐藏但视觉可见，以及 SVG 从 title 属性取得名称；没有使用用户现有浏览器会话 |
| oracle 结构化对照 | 用现有 `phoenixExpected` 分别应用 base 与 target 的 adapter 数据 | 13 个输出变更 case 的 exit/signal/files 未变，JSON 内置结果未变；这证明隔离兼容性，不证明新增 advisory 都正确 |
| `mise exec --no-deps -- bun scripts/render.mjs --check` | `tools/phoenix-ui` 目录，当前源码和已交付生成物 | exit 0、`status=fresh`；19 个 provider，input=`db9070e9…f42294`、bundle=`95bd47c3…709d27`。该检查不重算原生构建的完整 sourceInputSha256，故不反证 F-10 |
| 开发来源绑定重算 | 按 `build-phoenix-ui.mjs:42–73` 相同算法重算，并与既有 build-record 的逐文件 hash 比较 | 577 个输入仅测试文件不同；产物清单绑定过期，见 F-10 |

完整 Phoenix 套件主动终止前未进入 oracle，不对其作整套通过声明；本轮严格 oracle 另行执行并输出到临时目录，既有开发验收证据保持原字节。临时日志和脚本仅本机可见，持久判断所需的输入、代码位置与实际结果已写在本报告。

## 4. 计划符合性

| 需求/约束 | 本期实现映射 | 核查结果 |
| --- | --- | --- |
| R-1 / I-1–I-9 | 规则包、五个 Phoenix 源文件、design-reference、相应文档与 evals | 采纳/改编项有实际落点；本轮未把来源清单本身当运行验收 |
| R-2、NFR-1 / V-1–V-4、V-7 | 八条规则 + CLI/URL/hooks + 正反例 | 接线已核实，现有测试覆盖既定 fixture；F-01–F-08 暴露未被这些 fixture 保护的准确性缺陷，误报收口条件未满足 |
| R-3 / V-6、V-9 | Operate 控件/表单、craft-floor 宽度/状态/外壳、live/critique 告知 | 文字及生成传播已核实；F-09 需要统一默认修法；本轮没有对 provider/视觉验收作完整通过判断 |
| R-4、R-5、R-6 | 不可信内容分级、删除前路径确认、抓取边界/失效/付费作品处理 | 增量正文、evals 与原行为记录已读取；本轮未发现可确认的新实现缺陷，受限验证见 §2 |
| R-7、C-9 | foundation 双前缀 + ownership + 页内 WASM 重生 | 前缀别名、既有词义和规则列表解析边界已核对；本轮双前缀单测、无包一致性与 hook 基线用例已实际通过；页内向量结果见 §3.3，既有向量不等于专门穷尽测试新前缀的每种页内用法 |
| NFR-2、C-1、C-2、C-7 | 独立 RulePack，内置规则/捕获合同不改，逐 case adapter/DELTAS | 结构及差异隔离成立；兼容期望通过不能关闭 F-08 的误报 |
| C-3、C-4、C-8 | render/build 元数据、原生回归入口、skill 接口保持 | 轻量回归与带引擎 render --check 通过；原生完整来源绑定有 F-10 的非阻塞偏差；名称/参数/description 的差异核对未发现越界 |

计划没有 PRD 来源。本轮不更新计划进度，不作 PRD 覆盖验收，不执行发布。

## 5. 总体结论与修复顺序

**验收判定：需要修改。**

`patch is incorrect`。确认 8 项 P2、2 项 P3，集中在默认检测的误报/漏报、守则冲突与构建证据偏差；其中 warning 误报已证明会影响 CLI 退出码，并可通过 Cursor 编辑门造成实际拦截。现有 fixture 与 oracle 的通过不能反证这些新增输入，尤其 F-08 已被错误地固化进期望。

1. 先修 F-01–F-04 的焦点规则解析边界，验证真实缺陷继续报、正常代码/注释不被 Cursor 拦截。
2. 修 F-05–F-08 的页面命名/候选去重/可见性及 HTML 声明上下文；F-09/F-10 可一并处理，最终改动定稿后经既有构建流程同步生成物及来源绑定。
3. 补充对应正反例，按精确 patch 政策修正受影响 oracle 期望，重跑受影响检查；规则定稿后重建产物与证据绑定，再对本报告逐项复审。

解除本轮阻塞的可观察条件：F-01–F-08 的负例不再产生错误发现、真实正例仍命中、受影响 CLI/Cursor/Chrome 行为与预期一致，且生成物与回归证据覆盖最终修复版本。没有把延期或未运行项记为已通过；本结论不代表合并或发布授权。

下一步：

```text
$apply-code-review docs/plan/phoenix-ui-hallmark.code-review.md
```
