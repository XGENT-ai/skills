# PHOENIX-UI-EVIDENCE-CRAFT-P2 · PRD 与开发交付差异核查

- 审查对象：[Phoenix UI 取证、中文排版与素材细节增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.2（`sha256:09b19f65376834f46480a3dec8394e8ab1a18ba2707f0399da0e9be0caad1e25`，已提交）· **P2（第 2 期）** × [开发计划 PHOENIX-UI-EVIDENCE-CRAFT-P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.md)（`sha256:84e2aa2a429d08d4c036ec89db3ca326fed51226575d08e24d1467c0dcab46db`，已提交，4/4 里程碑完成，状态 Ready）
- 审查类型：首次完整 P2 核查（非上轮复审；P1 核查见 [P1 差异报告](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md)）
- 审查日期：2026-11-11T21:51+08:00 · 核查人：Agent（review-prd-dev-gaps）
- 代码基线：main `9b07ef066c9ebd00eea83587756f44532d1d9d75`；CI 候选 `cd60330`（run [43007878500](https://github.com/XGENT-ai/skills/actions/runs/43007878500) success，四平台 + Node 18）；`cd60330..HEAD` 的 `tools/phoenix-ui/crates/` 无差异，引擎代码与候选一致。工作树 dirty 项均为台账、评审报告与本报告（不改引擎与 skill 源）。

## 1. 结论与范围

- 本期范围（PRD §7.1/§7.2）：WORK-F5/AC-1–AC-7、WORK-F6/AC-1–AC-4、WORK-N3/AC-3（含跨期 AC-1/AC-2 的持续满足与 **P2 终验**）、WORK-N4/AC-2（每期）；UAT-7、UAT-8、UAT-17（UAT-14 随 N3 终验核对）；WORK-N1/N2 的持续约束对本期新代码继续适用。
- 整体结论：**在上述基线与证据范围内，本期全部需求与验收条件已覆盖**。差异计数：**A 明确遗漏 0；B 合理变更 3；C 需 Triage 1（已决 1 / 待决 0）；待核实 0**；follow-up 4（1 项已定增量、3 项发布后观察）。
- 独立验证：本轮除核对完成记录外，用与候选引擎代码一致的开发构建（`local/phoenix-ui/r3-development-release`，引擎 `sha256:530ddf6661f4aff5…`，构建自 dirty@`2518626`，即 R3 修复后源码）实际运行了 10 个夹具页的 `detect`（含 `--no-advisory` 与 `detector.ignoreRules` 各一次），并静态核实了规则、让位钩子、备注通道与全部文档落点。
- WORK-N3 本期终验条件成立：AC-1/AC-2 在 P2 变更后保持（V-6 oracle golden 逐字节不变、V-8 处置段无 diff、本轮 latin/mixed 实测内置行为不变），AC-3 已交付。按 PRD §7.3，该需求的人工「已上线」流转待 SPMS 录入（外部事项，见 §5 基线）。
- 无需生成 gaps plan：无 A；唯一 C（G-01）已由用户拍板为一句指引补充并选择并入下期承接（D-01）；B 类均为发布后观察项，不构成本期代码增量。

## 2. 三类差异

### A · 明确遗漏

无。

### B · 合理变更

#### G-02 · 字体名按声明名匹配，本地化字体名可能把正常结果报成第三种情况 · B

- PRD 期望：[WORK-F5/AC-7 · PRD:272](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)；§2.1「汉字字体意外回退」正常结果条款「不因实际字体名与声明的关键字不同而算回退」（PRD:90）。
- 实际交付：`cjk.rs` 的 `listed()` 按声明族名与系统报告名做等值匹配（`custom` 字体豁免）。声明英文名（如 `PingFang SC`）而系统按本地化名报告（V-9 的 macOS runner 实测把 PingFang SC 报为「蘋方-簡」）、或声明本地化名，且不写通用族时，会被判为第三种情况（误报）；补一个通用字体族即可消除。
- 证据链：M2 交付 → [计划 §11「字体名匹配的局限」](PHOENIX-UI-EVIDENCE-CRAFT-P2.md#11-风险开放问题与就绪状态) → `crates/rules/src/cjk.rs:305-322`（本轮静态核实）→ [docs/phoenix-ui.md](../../docs/phoenix-ui.md) 规则说明段已写明该局限与消除办法。
- 影响：advisory 级、不改变退出码；受影响的是「只写本地化/英文本地字体名且不写通用族」的声明写法；不阻塞主路径。
- 分类依据：实现受浏览器字体名报告差异约束（V-9 实测跨 runner 不一致）；缓解（docs 披露 + 补通用族建议 + 反馈调整）已实现并有文档证据；与 PRD Q2 对误报率「先上线、据反馈调整并对外说明」的既定处置一致。
- 处置：保留；发布后按误报反馈调整匹配策略。
- Follow-up：观察项 F-O1（无本期代码动作）。

#### G-03 · `cjk-split-unit` 单位识别采用封闭词表，少见多字单位拆行不报 · B

- PRD 期望：WORK-F5/AC-3（PRD:268）「数字与紧随单位被拆到两行时报 `xgent/cjk-split-unit`」。
- 实际交付：`UNIT_WORDS`（14 个多字单位）+ `UNIT_CHARS`（单字量词与符号全集）识别；词表外的多字单位（如「光年」「千瓦时」）拆行漏报。
- 证据链：M2 交付 → `crates/rules/src/cjk.rs:30-56`（本轮静态核实）→ 计划 §11「单位词表覆盖不全」→ [M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M2.md) 验收表 split-unit 全部用例（含 `38 秒`、`1,238 位`、`99.5%`）通过；本轮 `article.html`/`breaks.html` 实测均报出。
- 影响：advisory 漏报，不影响退出码与主路径；PRD 的判定以示例给出，未要求穷举量词。
- 分类依据：封闭词表是实现约束下避免把普通汉字误判为单位的选择；漏报类已在计划 §11 披露并由维护者按反馈增补，属可持续的小步演进，不构成对验收条件的实质偏离。
- 处置：保留；按反馈增补 `crates/rules/src/cjk.rs` 常量。
- Follow-up：观察项 F-O2（无本期代码动作）。

#### G-04 · 依赖系统字体的两类字体判定只有 macOS 自动化证据 · B

- PRD 期望：WORK-F5/AC-7 对「同段混用」「无通用族」两种情况正确判定。
- 实际交付：判定逻辑（`unexpected_fallback` 的 case 2/3）是平台无关的 Rust 代码；自动化夹具（`fonts-macos.html`）只在 macOS 断言（D14：CI 不装中文字体）；Linux 侧以 V-9 通用族夹具证明探针取数正常、无虚假备注，两类判定的 Linux 行为没有独立自动化或本轮实测证据。
- 证据链：M2 交付 → [M4 记录 A-1 收口](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md) → V-9 四平台日志（Linux 各 786 pass，通用族/中文分流断言 ok）→ [R6 复审](PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.r6.md) F-03 行确认。
- 影响：Linux 上这两类若有误报/漏报，要到发布后真实反馈才会发现；不阻塞本期验收（macOS 行为已证，逻辑平台无关）。
- 分类依据：验证策略受 CI 环境无中文字体约束（已核实）；替代证据（平台无关逻辑 + Linux 探针取数 + macOS 行为）与收口决定见 M4；PRD 未要求按平台自动化。
- 处置：保留 M4 的收口；发布周期内维护者可在 Linux 人工复跑字体夹具或为 CI 补中文字体。
- Follow-up：观察项 F-O3（无本期代码动作）。

### C · 需 Triage 的实现差异

#### G-01 · 按意群断句的 span 使文字退出中文规则与字体备注的候选范围 · C（已决 → D-01）

- PRD 期望：WORK-F5 行为段（PRD:258）「检查的元素范围与现有 `line-length` 相同」（有直接文本的元素）——实现忠实；但 PRD 的目标是检出中文排版缺陷，未预见到 WORK-F6 typeset 指引「短段落按语义手动断句」与该范围的相互作用。
- 实际交付：候选要求元素有直接文本（`passage()`，`cjk.rs:66-90`；正文另要求直接文本 >10 字符，与内置 `line-length` 的现行条件一致，计划 §1.1 已核实）。V-5 与 V-7 的会话都按「按意群断句」把标题或短段落整段放进 `inline-block` 的 span，元素从此没有直接文本，五条 `xgent/cjk-*` 规则与字体备注都不再覆盖该段；用 `<br>` 断句则不受影响。
- 证据链：M3/M4 记录的 V-5、V-7 会话观察 → [计划 §11 对应行](PHOENIX-UI-EVIDENCE-CRAFT-P2.md#11-风险开放问题与就绪状态)（开发方已把「是否放宽候选」明确留给用户）→ 本轮静态核实 `cjk.rs` 候选条件与 [docs/phoenix-ui.md](../../docs/phoenix-ui.md)「正文指……超过 10 个字符的直接文本」的对外说明一致。
- 影响：被 span 包裹的中文正文不再被 detect 覆盖（孤字/行长/行高/字体回退与未核实备注）；这是 PRD 明文范围与产品目标之间的张力，不是实现偏离。
- 分类依据：实际行为与 PRD 目标存在差异、处置需要产品取舍（维持范围 / 指引缓解 / 修订 PRD 放宽），且无既有决定可沿用——开发计划仅承诺「不在本期擅自扩大」，决策权在用户。
- 处置：**已决（D-01，2026-11-11 用户答复）**——维持 PRD 候选范围不变；在 typeset 的中文断行指引补一句「按语义断句优先用 `<br>`；整段包 inline-block span 会使该段退出中文规则检测」；该增量并入下期计划（如 P3）或下次动 skill 时顺手带上。
- Follow-up：F-01（已定增量，见 §6）。

## 3. 完整需求覆盖账本

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F5/AC-1 行高 1.4 报 warning 写明倍数与门槛；1.6 不报（PRD:268） | 本期 | §5.3；M2 | M2 验收表 R1-2 | 本轮实测 `tight.html`：`xgent/cjk-tight-leading` warning，`line-height 1.40x … (need >=1.5)`，退出 2；V-7 A2 实测 1.6 不报。本次实际验证 | 已满足 | — |
| WORK-F5/AC-2 ≥2 行超 40 字报 warning 给最长行字数；仅一行超长不报（PRD:268） | 本期 | §5.3；M2 | M2 验收表 | 本轮实测 `long.html`：warning `~52 Han-width chars on 6 of 7 rendered lines`；单行长行不报由 `crates/rules/tests/cjk.rs` 门槛两侧用例覆盖。本次实际验证 + 已有测试 | 已满足 | — |
| WORK-F5/AC-3 孤字/拆单元/意外回退三条 advisory 不改退出码（PRD:268） | 本期 | §5.3；M2 | M2 验收表 | 本轮实测 `article.html`（三条全报）、`breaks.html`（孤字+拆单元，退出 0）、`webfont.html`/`slow-font.html`（回退）。本次实际验证 | 已满足 | G-03（单位词表边界） |
| WORK-F5/AC-4 汉字为主不重复报内置两条；拉丁行为不变、不触发 cjk（PRD:269） | 本期 | §5.3/§5.4 让位；M1/M2 | M1、M2 验收 | 本轮实测：`article.html` 无内置发现；`latin.html`/`mixed.html` 只有内置 `line-length`/`tight-leading`、无 cjk；`replaces_builtin` 只对正文候选返回两条 ID（静态核实 + `crates/core/tests/rule_pack.rs`）。本次实际验证 | 已满足 | — |
| WORK-F5/AC-5 `ignoreRules`/`hooks ignore-rule` 逐条关；`--no-advisory` 只藏三条 advisory（PRD:269） | 本期 | §5.3；M2 | M2 验收表（V-3 夹具 9–11） | 本轮实测：`detector.ignoreRules` 关 `cjk-line-length` 后其余照报；`--no-advisory` 后仅剩两条 warning、退出 2。hook 命令写同一配置列表（docs「关闭方式」）。本次实际验证 | 已满足 | — |
| WORK-F5/AC-6 假名为主/英文夹中文不触发；汉字为主按汉字判定（PRD:269） | 本期 | §5.2 `han_dominant`；M2 | M2 验收表（V-1 ①⑤） | 本轮实测 `mixed.html`（假名段）无 cjk；`\p{Han}` 不含假名/谚文（静态核实）；混排样本见 V-1 ⑤。本次实际验证 + 已有测试 | 已满足 | — |
| WORK-F5/AC-7 通用族/正常加载不报；失败、混用、无通用族报出；无法核时不报且 `note:`（stderr）+ 报告 `findingsNotes`（PRD:270） | 本期 | §5.5 探针 + §5.7 备注；M1 | M1、M2 验收 | 本轮实测：`clean.html`（仅 `sans-serif`）无发现无 note；`webfont.html` 报失败回退且 `note:` 仅在 stderr、不进 JSON；`slow-font.html` 失败照报（不因加载中被豁免）；`findingsNotes`/`NOTE` 行在 `crates/cli/src/capture/{mod,report}.rs` 静态核实。字段说明在 docs/phoenix-ui.md（`--help` 保持 golden 逐字节不变，非 PRD 要求项）。本次实际验证 | 已满足 | G-02、G-04 |
| WORK-F6/AC-1 zh 界面行高 1.6–1.75、标题不碰撞、数字单位不拆（PRD:297） | 本期 | §6.1 typeset + §6.2 craft-floor；M3 | M3 V-4/V-5；M4 V-7 A1 | typeset.md「Chinese copy」节与 craft-floor Type 行逐条对应 PRD 口径（本轮读取核实）；V-7 A1 changelog 会话实测正文 1.7、按视口核对。已有运行记录 + 本轮静态核实 | 已满足 | G-01（span 断句交互） |
| WORK-F6/AC-2 按 25–40 字评估中文正文，不用 45–75 字符（PRD:297） | 本期 | §6.1；M3 | M3 V-5；M4 V-7 A1 | V-5 会话全程未用 45–75 字符评估、改 36em 正文栏；typeset/craft-floor/evals#19 三处口径一致（本轮核实）。已有运行记录 | 已满足 | — |
| WORK-F6/AC-3 comp-led 伪汉字替换并逐条列出，可辨文案逐字沿用（PRD:297） | 本期 | §6.3 new-work + finish-reviewer；M3 | M3 V-4；M4 V-7 A3 | new-work.md hero 段与评审输入合同含替换记录（本轮读取核实）；V-7 A3 真实 comp-led 会话替换 4 处伪汉字并逐条列出、其余逐字沿用。已有运行记录 + 本轮静态核实 | 已满足 | — |
| WORK-F6/AC-4 中英混排基线/字重接近；窄幅展示字限位（PRD:297） | 本期 | §6.1；M3 | M4 V-7 A1 | V-7 A1 实测核对基线对齐与视觉字重并记录；typeset「中英混排」条目在册。已有运行记录 | 已满足 | — |
| WORK-N3/AC-3 默认启用、可单独关、hook 不产生、live/扩展不运行、docs 如实写明（PRD:588） | 本期（终验） | §5.3/§5.4/§6.4；M2/M3 | M2 V-3、V-8 | 本轮实测默认启用与逐条关；规则注册在页面包、hook 只跑文本规则（`lib.rs` 结构 + M2 V-2 夹具 9–11）；「hook 不产生这五条规则；live 叠层与浏览器扩展不运行它们」见 docs/phoenix-ui.md 规则说明段（本轮读取）。本次实际验证 + 已有测试 | 已满足 | — |
| WORK-N3/AC-1 既有流程行为与升级前一致（P1 交付，持续满足）（PRD:586） | 跨期持续 | §4；M4 V-8 | M4 V-8/V-6 | oracle golden 逐字节不变（V-6 全量回归 0 missing）；本轮 latin/mixed 内置行为与升级前口径一致。已有测试 + 本次实际验证 | 已满足 | — |
| WORK-N3/AC-2 四个处置词与含义不变（P1 交付，持续满足）（PRD:586） | 跨期持续 | §6.3；M3 | M3 V-4 处置段 | finish-reviewer 的 Disposition/Output Contract/Verdict Pass 三段无 diff；R6 复审维持。已有检查 | 已满足 | — |
| WORK-N4/AC-2 每期动 skill 前整合清单更新并逐项核对（PRD:594） | 每期 | §0.6；M3 | M3 V-4；M4 V-8 | 计划 §0.6（C-1–C-8 采用/调整/拒绝）先于 skill 修改；V-8 按 `git diff 45e3844` 逐项核对，独立 skill 与 browser-bundle/wasm/comp-verbs 无 diff。已有检查 | 已满足 | — |
| WORK-N1（持续约束：无新增网络行为） | 约束继续适用 | §5.5；M1 | M4 | 探针经 CDP 读目标页自身字体，不发起网络请求；`slow-font` 等待重取在页内完成。静态核实 | 已满足 | — |
| WORK-N2/AC-1、AC-2（持续约束：Node ≥18、无 Python） | 约束继续适用 | §9.2 D14 | M4 V-9 | Node 18.20.8 离线安装与 capture smoke 通过（CI run 43007878500）。已有测试 | 已满足 | — |
| UAT-7 中文页 detect 发现集合精确一致 | 本期 | §9 V-7 A2 | M4 | V-7 A2 目的页实测：行高/行长 warning + 孤字 advisory、无内置、无 font-fallback，修复后消失，退出码 2。已有运行记录 | 已满足 | — |
| UAT-8 comp-led 伪汉字替换清单一一对应 | 本期 | §9 V-7 A3 | M4 | 同 WORK-F6/AC-3。已有运行记录 | 已满足 | — |
| UAT-17 拉丁页发现集合与升级前一致 | 本期 | §9 V-7/V-9 | M2/M4 | latin.html 四平台夹具断言 + 本轮实测。已有测试 + 本次实际验证 | 已满足 | — |
| UAT-14 comp-led 门禁行为不变（随 N3 终验） | 跨期 | §9 V-8 | M4 | oracle golden 不变。已有测试 | 已满足 | — |

无计划落点的需求：无（本期每条 AC 均有 §0.1 账本行与里程碑落点）。

## 4. Triage 与决策账本

### 待决问题

（无。G-01 已于本轮答复。）

### 已决决策

| 决策 ID | 关联 G-ID | 用户选择与理由 | 确认日期与来源 | 新目标/验收边界 | 跟进/延期/豁免去向 |
| --- | --- | --- | --- | --- | --- |
| D-01 | G-01 | 「补指引提示」：候选范围维持 PRD 原文不变；在 typeset 中文断行指引补一句——按语义断句优先用 `<br>`，整段包 inline-block span 会使该段退出 `xgent/cjk-*` 检测与字体备注。理由：最小文档增量即可弥合 typeset 指引与检测范围的交互，不必改 PRD 与引擎 | 2026-11-11 · 本轮交互答复（review-prd-dev-gaps 问题卡，timed 模式实答） | typeset.md「Chinese copy」断行条目含手法提示；引擎与 PRD 范围不变；验收＝渲染 `--check` fresh 且新句在生成物中 | F-01：并入下期计划（如 P3）或下次动 skill 时带上；不为本句单独开计划 |

## 5. 待核实项与审查基线

### 待核实项

无阻塞项。披露两项证据边界（不改变覆盖状态）：

1. G-04：Linux 上「混用/无通用族」两类判定的行为证据仅有平台无关逻辑推断 + Linux 探针取数证据；补证方式＝发布周期内在装中文字体的 Linux 环境复跑 `fonts-macos.html` 同类夹具，或给 CI 装中文字体。M4 已按 A-1 收口为发布后观察。
2. PRD §8 SPMS 录入仍因令牌 401 未执行（全 PRD 外部依赖，不属 P2 验收条件）；WORK-N3 终验后的「已上线」流转依赖该录入。P1 报告已登记同一事项。

### 审查基线

- 输入版本：PRD v1.2（哈希见页头，已提交）；计划 `sha256:84e2aa2a…`（已提交）；完成记录 M1–M4（含 R1–R3 修复与 R5 补证修订）。
- 代码：HEAD `9b07ef0`；CI 候选 `cd60330` 与 HEAD 的 `tools/phoenix-ui/crates/` 无差异；本轮实测用 `local/phoenix-ui/r3-development-release` 引擎（构建自 dirty@`2518626`，即 R3 修复源码，`sha256:530ddf6661f4aff5…`，与 V-7 宿主取证所用一致）。本轮未重跑 `test:phoenix`、CI 或宿主会话——R6 已实时核验 CI run 43007878500，输入未变，按验证范围规则复用。
- 验证方式分级：完成记录自述 / 已有测试与 CI / 本次静态核实 / 本次实际运行（10 夹具 × `detect`，`--no-advisory`、`detector.ignoreRules` 各 1 次）分别在账本标注。
- 评审链：计划评审 →（十项处置）→ R1→R6 代码评审（R6「通过验收」）；本报告不改任何评审原判定。

## 6. Follow-up 与 gaps plan

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| G-01 / D-01 / WORK-F5×F6 | typeset.md「Chinese copy」断行条目补一句手法提示（`<br>` 优先；span 退出检测），随后按维护者流程渲染并提交生成物 | `render --check` fresh；新句出现在 `skills/phoenix-ui/reference/typeset.md` 与生成物；无其他 diff | 无 | 并入下期计划（如 P3）或下次动 skill 时带上（用户 D-01 选择；不单独开计划） |
| G-02 / WORK-F5/AC-7 | 无本期动作；发布后按误报反馈调整字体名匹配 | 误报实例复现并消除 | 用户反馈 | 排除：发布后观察（计划 §11） |
| G-03 / WORK-F5/AC-3 | 无本期动作；按反馈增补单位词表常量 | 新单位拆行用例报出 | 用户反馈 | 排除：发布后观察（计划 §11） |
| G-04 / WORK-F5/AC-7 | 无本期动作；发布周期在 Linux 复跑字体夹具或 CI 装中文字体 | 两类判定在 Linux 有自动化或人工证据 | 发布周期 | 排除：M4 A-1 已收口为发布后观察 |

gaps plan：**无需生成**（无 A 类；唯一已定增量 F-01 为一句指引，用户已选择并入下期计划承接；其余为发布后观察项）。若下期立项 P3，建议把 F-01 列入其 §0.6/§6 的顺手项清单。

---

- 报告状态：已落盘并回读核对。历史报告：P1 核查见 [PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md)（本报告独立编号，不合并其 G-ID）。
- 下一步（复制即用）：`$review-dev-plan docs/plan/<下期计划>` 时把 F-01 纳入；P3 立项：`$dev-plan docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md P3`。
