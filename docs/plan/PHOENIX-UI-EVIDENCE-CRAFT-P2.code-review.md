# PHOENIX-UI-EVIDENCE-CRAFT-P2 · R1 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [PHOENIX-UI-EVIDENCE-CRAFT-P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.md)，核查版本 `7fb6e951dbf6bae1aeaa5093e2ce5b951b3bd12e` / SHA-256 `99b177ace57845cd199a9da211f5f18ab6d5b1eac826c96fe21123c01741561d` | 计划模式初审，`45e384462842d19ce59cf7e363cf9a8f139b3f1c..f6cc57894db4fbca3f2633b19ca148907bef5a57` 净差异；另核对当前未提交内容，无未提交代码 | 无 |

## 1. 确认问题

### F-01 · [P2] 行内隐藏文本不得进入行文字与字体事实

- 定位：`tools/phoenix-ui/crates/browser/src/typography.rs:100-104`（target `f6cc57894db4fbca3f2633b19ca148907bef5a57`）。
- 问题：候选元素本身可见时，`textNodes` 只排除不参与行内排布的块级后代，没有检查文本节点自身及行内祖先的 `visibility` / `opacity`；这两类隐藏文字仍保留正尺寸 `Range` 矩形，于是会进入 `linesOf` 和字体 run。不可见数字、单位或末尾汉字因此可以制造/掩盖 `cjk-split-unit`、`cjk-orphan`，隐藏文字还可能触发字体回退提示，用户会为页面上看不见的内容修复排版。
- 证据：计划 `PHOENIX-UI-EVIDENCE-CRAFT-P2.md:462-464` 明确要求“可见文字/可见汉字”；`typography.rs:122-125` 仅凭正尺寸矩形接收字符，而 `visibility:hidden`、`opacity:0` 均有布局矩形。真实入口复现：以 `tests/fixtures/xgent/cjk/clean.html` 的隐藏片段段落为模板，把不可见的“38”分别放进 `visibility:hidden` 与 `opacity:0` 的 span，可见文字只有“耗时 秒”，但 `detect --json --viewport 1280x800` 对两段各报一条 `number "38" and its unit "秒" sit on different lines`（退出 0，无备注）。现有 `clean.html:16-20` 只覆盖 `display:none`，所以测试未捕获该问题。
- 修复与验证：在文本节点进入行文字和字体 run 前过滤视觉隐藏内容；按文本节点计算 `visibility`，允许可见子级覆盖隐藏祖先，并核对至候选元素的透明度链，不把 `aria-hidden` 误当作视觉隐藏。扩展真实浏览器夹具，分别覆盖 `visibility:hidden`、`opacity:0` 的数字/单位、末尾汉字和独立字体 run，断言它们不影响任何发现。
- 置信度：高；代码路径、计划条款和真实 `detect` 结果一致。

### F-02 · [P2] 行重建须分开横向断开的多栏文字

- 定位：`tools/phoenix-ui/crates/browser/src/typography.rs:141-146`（target `f6cc57894db4fbca3f2633b19ca148907bef5a57`）。
- 问题：`linesOf` 只按垂直重叠归行，没有保留字符的左右坐标或检查横向间距；同一段落中不同栏的同一高度文字会被拼接成一条假行。`cjk-orphan` 因而会漏掉某栏真实的短末行，相邻假行也会使 `cjk-split-unit` 的首尾数字/单位判断失真。
- 证据：既有 `merge_text_rects_into_lines` 在 `crates/foundation/src/browser/dom.rs:271-281,317-327` 明确以横向 gap 防止两栏被并成页面宽行，新探针只复用了垂直条件。真实入口复现使用 109 个汉字、`500px × 160px` 的两栏段落（栏间距 `100px`，`16px/32px`）：截图中右栏末行只有“末”，左栏末行是完整文本；`detect --json` 返回 `[]`，`capture` 的有效截图条目也是 `findings: []`。相同文字改为 `200px` 单栏后，真实末行同为“末”，`detect` 正确报出 `xgent/cjk-orphan`。评审实际打开了截图核对，而非只读退出码。
- 修复与验证：保留字符/行的横向几何，并采用既有行矩形合并的 gap 守卫，同时保留原垂直行带。加入两栏真实浏览器回归：一例必须检出右栏孤字，另一例确保跨栏边界不会制造或漏掉数字单位拆行。
- 置信度：高；单栏对照、双栏真实页面、截图与发现集合构成完整因果链。

### F-03 · [P2] 页面级字体规则须遵守保留节点排除

- 定位：`tools/phoenix-ui/crates/rules/src/cjk.rs:251-257`（target `f6cc57894db4fbca3f2633b19ca148907bef5a57`）。
- 问题：`font_fallback` 通过页面钩子遍历全部元素，只检查字体事实和正文/标题候选；驱动对 `.impeccable-*` 叠层、`claude-*` / `cic-*` ID 与 live 子树的排除仅发生在元素规则循环，页面钩子绕过了它们。保留覆盖层因此会产生 `xgent/cjk-font-fallback`，还会被计入正常段落的 `(+N more elements…)` 合并数量。
- 证据：计划 `PHOENIX-UI-EVIDENCE-CRAFT-P2.md:404-407` 明确把驱动排除作为前提；实际排除位于 `crates/core/src/browser/driver.rs:1345-1360`，页面钩子在 `driver.rs:1480-1483` 另行直接执行。真实入口复现：仅含 `<p id="claude-debug">` 和失败 Web 字体的页面仍报 `web font "Fixture Han" failed to load`；一个正常段落加同栈 `claude-debug` 段落时，正常发现错误地带上了 `(+1 more element with this font stack)`。
- 修复与验证：在页面规则遍历进入候选及去重前应用与驱动相同的保留节点判断，优先共享谓词，避免两套选择器漂移。增加公开页面钩子测试：排除节点自身不能产出发现，也不能增加正常同栈段落的合并计数。
- 置信度：高；两个真实页面分别复现了直接误报和计数污染。

## 2. 待核实 / 待决策项

- 计划 §11 的“整段按意群包进行内 span 后不再成为候选”与 D3“汉字须过半”仍是留给用户的显式决策，本轮不作为新缺陷，也不代替用户选择。
- 临时探针对复杂 MutationObserver / 页面样式的影响只有既有四次会话和夹具证据，不能据此证明所有动态页面安全；本轮没有观察到新增故障，不列为猜测性 finding。
- 没有影响本轮“需要修改”判定的证据缺口；三项问题均已在当前目标代码对应的真实浏览器入口复现。

## 3. 覆盖与验证

### 3.1 范围与快照

- 一句话意图：为 Phoenix UI 增加中文正文/标题的五条页面规则、排版事实探针与备注通道，并把中文排版和伪汉字处置口径接入 skill。
- 仓库根：`/Users/rockie/Documents/GitHub/xgent/skills`；评审日期：2026-11-11。
- 原输入：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md`，未指定其他 Git 模式；按“完成记录”确定计划模式初审。
- 基线 / 目标：`45e384462842d19ce59cf7e363cf9a8f139b3f1c..f6cc57894db4fbca3f2633b19ca148907bef5a57`，比较两端树的累计净效果，不叠加中间版本问题。
- 登记实现/修复提交：`70fa92e3fe60a033f2b276becdead827b7a82744`、`e69c6007e5449e9fbacdfce267a377cfe4ee41f4`、`76814c1e6a94513918e173c8e1f131e1286f40a8`、`532f8d8f56ff87b739785fc5d518b5ad031ab3db`、`3d764e1b6699c37c0fcd5148b777227ef0c93c5d`、`f6cc57894db4fbca3f2633b19ca148907bef5a57`，均为当前 `main` 祖先。
- 进度回写 `f8999276f91ccf8ce8980fdfff18e10e5a12c352`、`6bca4917c46a8fd6ec7aeac066b9307dd5ae2f8c` 不单独作为代码变更；目标后的 `7fb6e951dbf6bae1aeaa5093e2ce5b951b3bd12e` 只用于最终计划/验收记录，不改变代码目标。
- 开评 HEAD 为 `7fb6e951dbf6bae1aeaa5093e2ce5b951b3bd12e`；`tools/phoenix-ui`、`skills/phoenix-ui` 与 `docs/phoenix-ui.md` 在 HEAD、工作树及目标间的代码内容一致。index 为空，SHA-256 为标准空摘要 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。
- 开评未提交内容只有台账、计划评审/报告文档，没有代码。tracked 净 diff 的 SHA-256 为 `52e87dfe9d7211b14892e90ecc9952161bdfa2ef29d96f5aaf2fefb1f57b360b`；按路径排序的 untracked「文件 SHA-256 + 路径」清单总 SHA-256 为 `9045c7f81f2b9b9def324a084f4104aaaa6ab408470fb5408a74c10542c933bd`。本报告与随后的台账回写不属于本轮代码输入。
- 纳入：手写 Rust / 页内 JavaScript、公开规则钩子、detect/capture 接线、测试与夹具、skill 源与文档。生成的 `dist/phoenix-ui/bundle`、渲染产物和 live 浏览器包按生成一致性核验，不把生成文本当作独立手写逻辑重复评审。
- 两名只读 reviewer 分别初查探针和规则/让位链路；主 agent 独立回读代码、裁定全部候选，并亲自完成三项真实入口复现。未以投票或 reviewer 数量作为证据。

### 3.2 已审行为链

- DOM 默认事实接口、快照/测试 DOM 实现、逐字 `Range` 行文字、字体 run、Web 字体状态/失败面归属、强制排版与清理、CDP 字体读取。
- 五条规则的候选、分类、门槛、级别、去重与同栈合并；内置 `line-length` / `tight-leading` 让位；ignore / `--no-advisory` 边界；无规则包、拉丁文本、hook/live 的兼容路径。
- `ScanOptions.notes` 到 detect stderr 与 capture `findingsNotes` 的传递、去重和报告序列化。
- skill 中文排版、伪汉字替换、备注披露口径，以及渲染产物和公开文档复述。

### 3.3 实际执行

目标代码先在 `tools/phoenix-ui` 用以下命令构建；工作树代码与目标一致，所得二进制 SHA-256 为 `3dc489cebf48830bb84971294f89ded03e6933e7d0327c3cdcddda2aba7672e9`，真实页面使用 Chrome `157.0.8103.40`：

```bash
mise exec --no-deps -- cargo build -p phoenix-ui --locked --offline
```

以下最小相关测试全部通过，但它们没有覆盖 F-01–F-03 的输入：

```bash
mise exec --no-deps -- cargo test -p phoenix-ui-rules --test cjk --locked --offline
# 15 passed

mise exec --no-deps -- cargo test -p impeccable-browser --lib typography --locked --offline
# 7 passed

mise exec --no-deps -- cargo test -p impeccable-core --test rule_pack --locked --offline
# 4 passed

mise exec --no-deps -- cargo test -p phoenix-ui --test capture initial_findings_match_detect --locked --offline
# 1 passed，73.57s

mise exec --no-deps -- bun scripts/render.mjs --check
# status: fresh；19 个 provider 的生成结果一致
```

真实入口反例均通过当前构建的 `tools/phoenix-ui/target/debug/phoenix-ui detect --json --viewport 1280x800 file:///tmp/phoenix-p2-review/<页面>.html` 执行；F-02 另运行 `capture` 并打开 `desktop.png`。实际结果见各 finding。临时页面位于仓库外，未修改目标仓库的测试或实现。

既有当前版本证据也已核对来源，而非重新冒称本轮执行：

- M2 的 V-1/V-2 覆盖桌面与移动视口、五条规则、关闭规则、拉丁对照、慢字体备注及 capture。
- M3 的 V-5 与 M4 的 V-7/A1/A3 记录真实编码 agent 会话；V-7 的中文夹具覆盖 `532f8d8` 修复后的桌面/移动页面。
- M4 的 V-9 第二次候选记录精确绑定目标 `f6cc578`，GitHub Actions run `42970903091` 的四平台与 Node 18 冒烟成功。本轮未重新运行远程 CI、整组宿主会话、npm/R2 发布或全量 `test:phoenix`；这些缺口不妨碍三项已复现缺陷成立。

## 4. 计划符合性

| 计划范围 | 映射与结论 |
| --- | --- |
| M1 / §5.5–5.7 | 探针、事实接口和备注链路已实现，但 F-01、F-02 表明“可见行文字/字体事实”尚未满足完整合同。 |
| M2 / WORK-F5、WORK-N3 | 五条规则、级别、让位、开关与拉丁对照有实现及测试证据；F-01/F-02 违反实际孤字与拆单位判定，F-03 违反保留节点排除前提。本期不能记为代码验收通过。 |
| M3 / WORK-F6、WORK-N4 | 中文口径、伪汉字替换与备注披露已落到源文件、生成物及公开文档；`render --check` 为 fresh，既有真实 agent 会话证据存在。本轮未发现该部分新增缺陷。 |
| M4 / V-6–V-9 | 证据记录与候选 SHA 对应，CI 和真实入口记录可用；它们未覆盖本轮三个反例，不能以既有通过记录覆盖新问题。 |
| 后续/开放范围 | npm/R2 发布、SPMS 录入、真人公测和 §11 两项选择不因本轮评审自动扩大或改判。 |

## 5. 总体结论与修复顺序

**需要修改**

**正确性结论：`patch is incorrect`。** F-01–F-03 都是本次新增路径上可到达、已用真实 `detect` 验证的行为缺陷：分别造成隐藏内容误报、多栏孤字漏检、保留覆盖层误报及合并计数污染。现有测试和 CI 通过说明主路径已有回归基础，但不能覆盖这些反例；本结论不构成合并或发布授权。

先修 F-01 与 F-02 的页内事实采集并用真实页面回归，再修 F-03 的页面级排除与去重；修复后按原关闭条件复审三项，并复跑本报告 §3.3 的相关测试。无需为通过本轮评审重做无关全量验证。

```text
$apply-code-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.md
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md
```
