# PHOENIX-HALLMARK · 代码评审 R2（修复效果复审）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2，修复效果复审 | [Phoenix UI 借鉴 hallmark](phoenix-ui-hallmark.md) | `3e3885b78e08d09f9dfef2b5b78e88fa3131b115..150e5160b74dd72bf8f62afcb2690a81120670e4`；核对 R1 全部问题及修复直接影响的回归；开始时工作树/index 干净 | [R1 代码评审](phoenix-ui-hallmark.code-review.md) |

评审 ID：`1012ce80-e25c-4d14-9f91-60e2342882b1`。日期：2026-10-23（Asia/Shanghai）。本轮由当前主 agent 独立完成，未分派其他 reviewer。

上轮审查基线为 `886efe201c58da77e9fa223ca780f5be1101eff4..3e3885b78e08d09f9dfef2b5b78e88fa3131b115`；R1 报告内容 SHA-256 为 `4e1339c1a9f5414392e6d492b421c23b1d335c2b730623c914e4ddc5391abc9d`。本轮修复提交为 `e532b6011b30da6382e39bb4a8286e1557704bbc`，当前 target 是表头的 `150e516…`，后者仅回填计划中的修复 SHA，代码与 `e532b60` 相同。本轮没有重新完整审查 R1 的原始实现范围。

处置来源：[M1](phoenix-ui-hallmark.records/M1.md#代码评审-r1-修复2026-10-23)、[M2](phoenix-ui-hallmark.records/M2.md#代码评审-r1-修复2026-10-23)、[M3](phoenix-ui-hallmark.records/M3.md#代码评审-r1-修复2026-10-23)、[M4](phoenix-ui-hallmark.records/M4.md#代码评审-r1-修复2026-10-23)、[M5](phoenix-ui-hallmark.records/M5.md#代码评审-r1-修复2026-10-23) 的「代码评审 R1 修复」节。以下结果来自本轮独立核查，不直接采用记录中的“已修改并验证”声明。

## 1. 确认问题与上轮问题复审

### 1.1 上轮问题复审表

来源报告统一为 [R1](phoenix-ui-hallmark.code-review.md)，评审对象 `1e55e17d-4399-4289-bd1c-131954c5208e`，版本及 hash 见报告头；修后版本统一为当前 target。

| 来源 ID | 上轮问题 / 优先级 | 已有处置 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| F-01 | 注释中的焦点类名误报 / P2 | M1：扫描前空白化注释 | **已解决** | `text.rs:200–263`；行注释、块注释、JSX 注释的旧/新引擎对照从 3 条 warning 变为 0，Cursor 无关编辑返回 allow；真实移除轮廓仍报；文本及 hook 回归通过 | 无 |
| F-02 | focus-within、not-focus 等误判 / P2 | M1：共用焦点变体语义判断 | **已解决** | `text.rs:494–559`；两个旧负例从 exit 2 变为 exit 0，真正 focus 仍报；任意选择器、否定条件的现有测试通过 | 无 |
| F-03 | 拆开的静态类名表达式误报 / P2 | M1：合并相邻的 `+` 字面量，跳过部分动态表达式 | **部分解决** | 裸 `+` 拼接已通过；给操作数加括号、数组 join 和同一个 cn 调用仍拆开判定，exit 2 且 Cursor deny；见 §1.2 | 完整表达式分析或在不能确定时停止断言缺少替代 |
| F-04 | 零宽单位漏报、非零小数误报 / P2 | M1：完整长度分量判断 | **已解决** | 原始 `0px` / `outline-width:0px`、`0rem`、零小数与 `0.5px` 对照及单测通过；颜色函数内的零不再当宽度。本次修复另外引入的优先级后缀漏报列为 F-11 | 按 F-11 修复新增回归 |
| F-05 | SVG title 属性名称遗漏 / P2 | M2：接纳非空 title 属性 | **已解决** | `dom.rs:260–267`；DOM 测试覆盖非空/空白 title；真实 Chrome 的 `svg-title.html` 保留唯一无名 SVG 的 1 条发现，属性 title、子 title、ARIA 名称均不误报 | 无 |
| F-06 | 包裹 emoji 重复计数 / P2 | M2：候选由直接文本归属确定 | **已解决** | `dom.rs:402–407`；真实 Chrome 两宽度下单个包裹图标 0 条、两个包裹图标恰好 2 条；FakeDom 测试通过 | 无 |
| F-07 | aria-hidden 让视觉规则漏检 / P2 | M2：独立视觉可见性 helper | **已解决** | `dom.rs:546–568`；Chrome 两宽度的 aria-hidden 圆点/emoji 继续命中，真正隐藏 fixture 为 0；SVG 继续使用原 ARIA 过滤；DOM 回归通过 | 无 |
| F-08 | HTML 正文示例当作 transition 声明 / P2 | M1/M3：样式位置掩码、五个 oracle case 更正 | **部分解决** | 原 `<h4>transition: all</h4>` 已不报，五个受影响 oracle case 实测通过；转义显示的 HTML 代码示例仍被当作真实 style 属性，见 §1.3 | 将样式属性识别限制在实际标签内 |
| F-09 | 标签折行指引与检测冲突 / P3 | M4：重排控件行或移动控件，给标签空间 | **已解决** | 源与生成 `operate.md:51` 一致；render --check 为 fresh；隔离页面在两宽度下从 80px 按钮的 1 条 wrapped-control-label/exit 2，改为控件行重排后的 0 条/exit 0 | 无 |
| F-10 | 开发产物来源绑定过期 / P3 | M5：定稿后重新构建并同步绑定 | **已解决** | 按构建脚本原函数重算 577 个输入，得到 `a68bf113bb7977f11cd88580d36979e5aa4b8de123a556e986dea228bcd7582d`；与 VERSION、ENGINE、build-record 一致，逐项输入差异为 0；分发二进制 hash 一致 | 无 |

统计：实际解决 8 项，部分解决 2 项；不再适用 0 项，无未复核的旧 finding。F-03/F-08 是原问题残留，不计为本轮新增 finding。

### 1.2 R1 / F-03 · [P2] 对带括号及类名组合表达式保留完整性判断

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，当前 target 侧 426–428 行，未知表达式判断另见 435–436 行；沿用 R1 / F-03。
- 问题：本次合并只接受两个引号之间恰好为 `+` 的情况，未知表达式判断也只看紧邻字面量的 `+`。因此 `className={("focus:outline-none ") + ("focus-visible:ring-2")}` 仍会把第一个字面量当作完整类列表，尽管其最终字符串与已通过的单串完全相同。数组 `.join(" ")` 和 `cn("focus:outline-none", "focus-visible:ring-2")` 也保留同一误报；本轮使用的 cn 在 fixture 中明确实现为参数 join，无库行为假设。Cursor 对这些文件的无关编辑仍返回 deny，R1 关于无法确定完整表达式时应避免断言缺少替代的关闭条件尚未满足。
- 证据：旧/新引擎对照中，裸拼接从 exit 2 变为 0；括号拼接、数组 join 和 cn 组合仍各报 1 条 warning、exit 2，对应三个 hook-before-edit 均 deny；等价单串为 0 条，真正缺少替代的正例仍报。修后记录 [M1](phoenix-ui-hallmark.records/M1.md) 将 F-03 列为已修改并验证；上述组合仍重现 R1 的同类错误。
- 修复与验证：以完整 className 表达式为判断边界，处理支持的静态组合；不能证明完整内容时跳过“没有替代”的结论。回归至少覆盖单串、裸/带括号拼接、数组 join、常用类名组合和真正缺少替代的正例，并检查 Cursor 的实际放行/拒绝。
- 置信度：高；确定性输入和 CLI、编辑门均已复现。

### 1.3 R1 / F-08 · [P2] 仅保留真实标签中的 style 属性

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，当前 target 侧 303–305 行；沿用 R1 / F-08，单引号分支有相同边界。
- 问题：`mask_markup_styles` 在整份原文上搜索 `style="…"`，没有验证匹配位于真实标签内。普通文档正文 `<pre><code>&lt;div style="transition: all .2s"&gt;Example&lt;/div&gt;</code></pre>` 只是转义显示的代码，页面没有任何内联样式或过渡，仍被保留为 CSS 并报 `xgent/transition-all`。原始标题负例已修，但“正文示例不是声明”的准确性问题仍存在于常见代码文档场景。
- 证据：隔离的 `body-code.html` 只含该正文、常规标题和文档外壳，旧/新引擎都报 1 条 advisory；原 `<h4>` 负例修后为 0，真实 `<style>` 和真实 style 属性对照仍各为 1。当前源码重建的 debug 引擎也复现相同正文误报。计划 NFR-1（第 81 行）要求处置误报类别；本轮五个 oracle case 的通过只覆盖原标题示例的删除。
- 修复与验证：识别真实 HTML 标签的属性上下文，排除正文和实体转义的标签示例；验证单双引号属性、真实 `<style>`、注释与转义代码块，保持真实声明命中和行号正确。
- 置信度：高；源码边界和无 CSS 的最小输入一致。

### 1.4 本轮新增 F-11 · [P2] 判断 outline 值前剥离 CSS 优先级后缀

- 定位：[text.rs](../../tools/phoenix-ui/crates/rules/src/text.rs)，当前 target 侧 333–337 行，与修复 diff 重叠；同根因影响 338–345 行的 outline-style / outline-width 分支。
- 问题：把原正则替换为完整分量判断后，紧贴值的 `!important` 被一起传入关键字/零长度匹配。合法的 `button:focus{outline:none!important}` 中，`none!important` 不等于 `none`，`outline:0!important` 的后缀也无法当作单位，因而不再产生 warning。原基线能够检测这些写法，本次修复却让真实移除键盘焦点标记的紧凑 CSS 静默通过；加一个空格才又恢复命中。
- 证据：相同文件、相同命令 `detect --json <file.css>`，基线引擎对 `outline:none!important`、`outline:0!important`、`outline-width:0!important`、`outline-style:none!important` 均 exit 2、1 条该 warning；修后分发引擎均 exit 0、0 条。`outline:none !important` 修后仍 exit 2。当前源码重建的 debug 引擎也确认首个紧凑输入漏报。基线引擎 hash 与 R1 所记 `25a01539…` 精确一致，修后分发引擎有独立来源绑定，见 §3。
- 修复与验证：先从声明值中正确分离末尾的 CSS 优先级，再判断完整关键字/长度。加入上述有无空格、零值单位及非零小数的回归，确保紧凑优先级写法命中且 `0.5px` 继续不误报；与 F-03/F-08 一起重建最终产物。
- 置信度：高；基线→目标的行为回退已直接验证，未把原有的 `0px!important` 漏报冒充新增。

## 2. 待核实项与覆盖限制

- R1 关于完整 `test:phoenix` 未跑完的缺口：本轮读取 [M5 修复记录](phoenix-ui-hallmark.records/M5.md) 及本机 `local/phoenix-ui/r1-evidence/test-phoenix-full-pass.log`，其中有整套 EXIT=0、WASM 8,474 向量零失败和 oracle 860/860 的完成记录。本轮没有重新运行整套命令；只将其作为已有实施证据，并独立运行 §3 所列定向检查。不能表述为本轮重新通过全部回归。
- R1 的 provider/视觉验收限制仍保留：本轮未发起付费 provider 会话或实拍 UI 验收；原 V-9 的浏览器操作权限缺口未因此关闭。真实 Chrome 的规则结果和隔离布局对照已执行，但不等于产品观感验收、真实辅助技术或其他平台验收。
- 原 R1 所列所有 Unicode emoji、任意动态类名和全部 SVG 名称路径的穷尽验证仍不在本轮范围；本轮确认问题均有明确输入，不据此推导额外缺陷。
- 无需用户决定即可修复的已确认问题均已在 §1 给出。没有将缺少全量重跑或后续发布范围独立列为代码 finding。

## 3. 覆盖与验证

### 3.1 范围与固定版本

- 输入：`$review-code docs/plan/phoenix-ui-hallmark.md`。按开发计划模式及已有 R1 默认启用修复效果复审；用户未要求完整重审。
- 变更意图：修复 R1 的十项检测准确性、守则一致性与构建来源问题，属于计划及修复记录明示的意图。
- 仓库：`/Users/rockie/Documents/gh-xgent/skills`，分支 `main`。登记的实现及修复提交均在当前分支可达。R1 target 后仅有 `0be853c94ed4b0db1220147ef9a18b12558fed5e`（计划/状态回写）、`e532b6011b30da6382e39bb4a8286e1557704bbc`（修复）、`150e5160b74dd72bf8f62afcb2690a81120670e4`（计划回填）。没有夹杂未登记的代码提交。
- 实际比较：报告头 base→target 的净差异。计划、R1 报告、里程碑记录和台账作为契约/证据读取；进度回写不作为代码 finding 主对象。没有额外 staged、unstaged 或 untracked 输入。
- target tree：`bc541ed2dd9a2497fb86992b0d5d653d9ccf7456`。计划按 target 版本读取，SHA-256=`d8f00848f96dec1b13a7a2f75520036f0a6713a0b28ad97aa5a9eee54d4eaa6c`。
- 初始 4,771 个 tracked 普通文件按仓根相对路径建立内容 SHA-256 映射；对其按路径排序、无多余空白且 ensure_ascii=false 的 JSON 再作 SHA-256，得到 `d5ce4a93ded60c00a5cb8441f76f27f140099840fe283a1ac4ad2055e7fc294a`。写报告前逐项重算，差异为 0，HEAD 未变。该指纹用于变化检测，代码的可取回快照是上述 Git target；新报告与台账回写是本轮输出。

### 3.2 已审行为链

| 范围 | 实际覆盖 |
| --- | --- |
| 文本解析 | 完整阅读 text.rs 修复 diff、相关原分支/测试、扩展名入口与 hook 调用；检查注释、变体、完整表达式、CSS 长度/颜色函数、样式位置和行号。确认两项旧问题残留和 F-11 |
| 页面检测 | 完整阅读 dom.rs 修复 diff、相关直接文本/可见性 helper、DOM/CLI 新测试和六个新增 fixture；SVG、emoji 数量门槛与视觉/ARIA 边界实测通过 |
| hook | 本次生产接线未改；核查 before-edit 调用和新增测试，并在隔离项目实际验证误报可触发 deny、注释旧负例已 allow |
| 守则与生成物 | 源/生成 operate.md 的完整改动，SKILL 元数据、VERSION/ENGINE/GENERATED；19 provider manifest 的 34 处变动为 16 个 SKILL hash 与 18 个 operate hash；17 个变更 blob 均校验内容 hash，SKILL 正文不变，operate 与生成正文一致 |
| oracle 与来源 | 结构化比较完整 adapter：653 个 expectation 中只有 5 个 case 内容变化，其余 648 个完全相同；分别应用 base/target 精确 patch 并核对 darwin/win32 输出，变化仅删除 motion.html:149 的 advisory 及关联计数，exit/files/内置结果未变。UPSTREAM、capabilities 的绑定与变更内容、构建输入及二进制 hash 已核查 |

### 3.3 本轮实际执行

所有命令都在当前 target 字节上运行；只写隔离临时项目和既有 ignored 构建目录，没有添加目标测试或修复。

| 命令 / 方法 | 实际结果 |
| --- | --- |
| 仓根 `npm test` | exit 0；Node 425 项：398 pass、27 skip、0 fail；Python 24 项、13 项通过，skills 2 项因 PyYAML 缺失跳过 |
| tools/phoenix-ui 下 `mise exec --no-deps -- cargo test -p phoenix-ui-rules --locked --offline` | exit 0；文本 17/17、DOM 21/21 |
| 同目录 `mise exec --no-deps -- cargo test -p impeccable-hook --test hook_tests --locked --offline` | exit 0；74/74 |
| 同目录 `mise exec --no-deps -- cargo test -p phoenix-ui --test xgent_rules --locked --offline` | exit 0；9/9，含真实 Chrome 的 13 fixture × 375x812/1280x800；用时 187.45s |
| 同目录 `mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline --message-format=json` | exit 0；固定 debug 副本 hash=`e2ce50a7dbe638f6a42c17477941b313bef053c1a429ab469f1c4eb597ed520e`，三项未解决问题均在该重建引擎复现 |
| 仓根 `node scripts/check-phoenix-lints.mjs` | exit 0，No added diagnostics；既有 fmt 差异及 539 条 clippy 诊断仍按基线保留 |
| tools/phoenix-ui 下 `mise exec --no-deps -- bun scripts/render.mjs --check` | exit 0，status=fresh、19 providers；input=`0373592b…0069a`，bundle=`a35570d9…ee95` |
| 旧/新分发行为对照及 Cursor 输入 | 22 个文本输入、5 个 hook 输入，完整输出保存在临时目录；F-01/F-02 的旧反例已关闭，F-03/F-08 残留和 F-11 的 base→target 回退见 §1 |
| 两宽度的标签布局对照 | 固定 80px 列导致标签折行：两宽度均 1 条 warning、exit 2；重排控件行并给按钮空间：两宽度均 0 条 xgent、exit 0，未用 nowrap/截断 |
| `checkOracle({bin, prefix})` 定向回放 | 对 §1.1 F-08 的五个受影响 case 分别运行，5/5 pass，无 error/missing/skip；过滤运行的 acceptanceComplete=false，未冒称全量 oracle 通过 |
| 完整 JSON/产物绑定核对 | 五个 oracle case 的 base/target 精确输出差异、darwin/win32 字段检查通过；原构建函数重算 577 个输入与 build-record 逐项完全一致；分发引擎 hash=`6bf2c7c7d54e9cd4bb6a7357d8cd11c6f7c72c4730e810e4751b00126fc8ddec`，与 VERSION 一致 |

临时证据根目录：`/var/folders/cv/pn7p1v7s737cqlt605jyl7lm0000gn/T/phoenix-hallmark-r2.m4iesl32/`。分发引擎用固定副本；基线复现复用 R1 保留的引擎，读取其字节 hash 确认等于 R1 报告中的完整值。临时目录仅本机可用；判断所需输入、版本、位置及结果已写入本报告，不依赖把临时文件提交到仓库。

## 4. 本轮计划符合性

| 契约 / 本轮任务 | 核查结果 |
| --- | --- |
| R-2、NFR-1 / 修复检测准确性，V-1–V-4 | F-01/F-02/F-04–F-07 的原问题关闭；F-03/F-08 部分解决且 F-11 为新增漏报，误报类别处置的收口条件仍未满足 |
| R-3 / 控件标签指引与检测一致，V-6 | F-09 已关闭，文字、生成物及两宽度的重排方案一致 |
| C-2 / oracle 精确 patch | 五个受影响 case 的变更可解释、定向回放通过；golden 与其他 case 没有改写，兼容通过不关闭剩余误报 |
| C-3 / 生成与开发构建绑定，V-8 | F-10 已关闭，来源及生成绑定可复现；下一轮修改规则后需按既有构建流程重绑最终版本 |
| 其余 R-4–R-7、发布与其他平台 | 本次修复没有改变相关授权/接口或引擎接线；本轮未重新完整验收这些旧范围，UI/provider/跨平台限制见 §2 |

计划来源为用户需求，无 PRD。本轮不修改计划进度，不替代 UI/PRD 覆盖验收。

## 5. 总体结论与修复顺序

**验收判定：需要修改。**

修复效果结论：**未完全解决**。R1 的 8 项问题已解决，F-03/F-08 部分解决；本轮新增 1 项 P2（F-11）。修复增量的正确性结论为 `patch is incorrect`，由 F-11 的确定回归支撑；旧问题单独影响验收，不冒充新增问题。没有 P0/P1，也没有以跳过的全量检查或后续发布要求凑入 findings。

1. 修 F-03 的表达式完整性判断，确认安全的类名组合不被 CLI/Cursor 误报，真正缺少替代仍可发现。
2. 修 F-08 的实际标签边界与 F-11 的声明优先级解析，保留真实声明/数值正反例；补齐对应回归并复跑直接受影响检查。
3. 规则定稿后经既有开发构建同步生成物和来源绑定，再逐项复审 F-03、F-08、F-11 及直接回归。

解除本轮阻塞的条件：上述明确负例不再误报，紧凑 !important 正例恢复命中，hook 行为正确，相关测试及最终产物绑定覆盖修复版本。本报告与台账保存不代表已合并或发布。

下一步：

```text
$apply-code-review docs/plan/phoenix-ui-hallmark.code-review.r2.md
```
