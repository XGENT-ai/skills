# PHOENIX-UI-EVIDENCE-CRAFT-P4 · PRD 与开发交付差异核查

- 评审对象：[PHOENIX-UI-EVIDENCE-CRAFT-P4](PHOENIX-UI-EVIDENCE-CRAFT-P4.md)（方案对比页与设计指引，5/5 里程碑已完成）
- 需求来源：[PRD-PHOENIX-UI-EVIDENCE-CRAFT.md](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.5 · P4（第 4 期）
- 核查类型：首次完整 P4 核查（本计划此前没有 `prd-dev-gaps` 报告，非复审）
- 相关既有评审：[P4 开发计划完整评审](PHOENIX-UI-EVIDENCE-CRAFT-P4.review.md)（对象 `339c267d-7815-49f2-8218-e1da08b57ac1`）、[R1 代码评审](PHOENIX-UI-EVIDENCE-CRAFT-P4.code-review.md)、[R2 修复效果复审](PHOENIX-UI-EVIDENCE-CRAFT-P4.code-review.r2.md)
- 前序核查（不属本轮范围，仅用于跨期需求承接）：[P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md)、[P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.prd-dev-gaps.md)、[P3](PHOENIX-UI-EVIDENCE-CRAFT-P3.prd-dev-gaps.md)
- 审查日期：2026-11-24；本报告全部结论基于下列基线，未审对象的前序报告结论保持不变。

## 1. 结论与范围

**整体结论：存在合理变更；无明确遗漏、无待用户决策。** 另有 1 项已确认的计划文档缺陷（不计入 A/B/C）与 2 项待核实。

- 本期范围内逐条建账：**24 条 PRD 验收（WORK-F10/AC-1–AC-6、WORK-F11/AC-1–AC-5、WORK-F12/AC-1–AC-4、WORK-F13/AC-1–AC-3、WORK-F14/AC-1–AC-3、WORK-N2/AC-4，以及 WORK-N2/AC-1–AC-3 的终验持续满足、WORK-N4/AC-3、WORK-N4/AC-2）全部有覆盖行**，另核对 UAT-12/13/15/16/18/19 的判据。
- 三类差异：**A = 0；B = 1（G-01）；C = 0（待决 0）**。
- 待核实：2 项（WORK-N2/AC-1 的发布后安装验证、跨会话遵循稳定性），均不阻塞交付。
- 需跟进：2 项（G-01 的文本同步、G-02 的计划台账修复），尚未被任何既有计划承接，已由新 gaps 计划承接。
- 交付形态与 PRD 一致：原生 `compare` 命令 + 对比页 + 四组设计指引进 `phoenix-ui` skill；未发现以「计划已完成」代替「PRD 已实现」的情形。

### 审查基线

| 项目 | 值 |
| --- | --- |
| 代码仓 / 分支 | `/Users/rockie/Documents/GitHub/xgent/skills` · main |
| HEAD / 工作树 | `2f64baa6` · clean |
| P4 收口提交 | 候选 `968b6539`、修复 `5e2f7554`、最终收口 `0eda22cd`（均在当前分支可达） |
| PRD | v1.5 · `docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md` · `sha256:7760ed9dcfdd8017b7b77d08357d03d14f7f894c6d87ecfcfce4c426c6a9579e` |
| 计划 | `sha256:3c1485e8896d98e31d6f7c5cfe51e1e9315952edf6d1344d0b67356f637154b1` · 5/5 里程碑已完成 |
| 完成记录 | [M1](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M1.md)、[M2](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M2.md)、[M3](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M3.md)、[M4](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M4.md)、[M5](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M5.md)（`sha256:24e6618449543f5249590e3d1bd891154fceba35327809a0bcc6153a3a4448f6`） |
| 跨平台 | Phoenix CI run `43449295726`（headSha 与候选 `968b6539` 完全一致，四平台 UI + Node 18 五个 job 全部 success） |
| 收口后改动范围 | `0eda22cd..HEAD` 只改了 `README.md`（不涉及 Phoenix UI 节）与 `tools/phoenix-ui/crates/cli/src/media/mod.rs`（P3 素材面）；`compare/` 源码、`crates/cli/src/main.rs`、`serve_question.rs`、`skills/phoenix-ui/**` 均无改动 |

### 证据等级

**实测**＝本轮亲自运行；**记录**＝完成记录中的运行证据（本轮复核其命令、结果与适用基线）；**CI**＝上述 CI run；**宿主**＝V-5/V-7 的原始 provider 会话（记录中保存的产物与回执）；**静态**＝本轮读当前源码、生成物或文档文本。

## 2. 三类差异

### A · 明确遗漏

无。本期 24 条验收逐条核实均有实现与证据；发现的两处未覆盖事实与一条文档缺陷分别记在 G-01（B）、第 5 节待核实与 G-02（记录缺陷），不构成产品遗漏。

### B · 合理变更

#### G-01 · 无对比页回答时，在没有结构化提问工具的宿主上改为在答复里再问一次 · B

- PRD 期望：WORK-F10/AC-4「页面关闭而无选择时，主代理通过结构化提问再问一次；仍无回答时…」（[PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) `:391`）。
- 实际交付：引擎如实区分三种无回答（`--wait` 4 = 页面关闭、`--start` 2 = 本会话无浏览器、`--wait` 2 = 服务故障不是用户决定），并提示「改用结构化提问」（[mod.rs](../../tools/phoenix-ui/crates/context/src/compare/mod.rs) 退出码表）。在工具清单里没有结构化提问工具的宿主上，会话把同一个问题写进答复正文——V-5 R3 用 `PHOENIX_UI_QUESTION_DISABLED=1` 实测：`--start` 退出 2、没起对比页、`src/` 无改动、交付写明「待选」并请用户回复 A/B 或保持现状，同时给了推荐但没有实现。
- 证据链：M3/M4（skill 与文档）→ [M4 §验证 V-5 R3](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M4.md#验证) → `skills/phoenix-ui/src/reference/compare.md`「No answer」段、`docs/phoenix-ui.md` `:229`。
- 影响：无产品语义损失——「再问一次」与「无回答不当作选择」都成立，只是机制从专用工具退化为答复正文；用户在两种宿主上都能得到一次追问。
- 分类依据：宿主能力约束（工具不存在）造成机制差异，替代行为已实测通过，PRD 承诺（问一次、不替用户选、有基线保持现状）完整保留。**但已交付的文本没有写这个前提**：`compare.md` 的「No answer」与 docs `:229` 都直接假定结构化提问工具存在——同仓 `vague-feedback.md`「When the meaning is unclear」写的是「use the structured question tool where the host has one」，两处口径不一致。
- 处置：保留行为；补齐文本（不做产品取舍）。
- Follow-up：在 `src/reference/compare.md`「No answer」段与 `docs/phoenix-ui.md` `:229` 各加一句「宿主没有结构化提问工具时，写在答复里再问一次并在交付里标出待选/未经用户选择」；验收＝渲染 `fresh`、两处文本回读、按新文本在无工具的会话里走一次无回答分支。→ gaps 计划 M1 / R-1。

### C · 需 Triage 的实现差异

无。本期没有需要用户拍板的范围、体验、权限或契约取舍；WORK-F12 的口径变更（「回复清单后等用户回复再动手」）已由 PRD v1.5 承载（用户 2026-11-13 决定），交付与 PRD 一致，不构成差异。

### 记录缺陷（非 PRD 差异，不计入 A/B/C）

#### G-02 · 原计划 §10 的 M5 里程碑定义被进度行覆盖 · 记录缺陷

- 期望：里程碑表是定义行，进度只写「实施进度」的完成记录（[计划](PHOENIX-UI-EVIDENCE-CRAFT-P4.md) §10 自述「里程碑表、检查表、ADR 等定义行不随进度改动」）。
- 实际：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md:736` 的 M5 行现在是六列进度行（「M5 | 已完成 | 2026-11-14 00:04 | V-6–V-9 完成… | 记录链接 | 候选/收口 SHA」），挤在五列表头下；M5 的定义（构建同步、全量回归、真实使用验收与四平台候选的 ①–⑦ 步骤）在计划正文中已不可读。
- 证据链：`git show 0eda22cd -- docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` 显示 `-| M5 | 阶段收口：构建同步、全量回归、真实使用验收与四平台候选 | …` → `+| M5 | 已完成 | …`；`git show 0eda22cd^:docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` 仍可取回原定义。M1–M4 定义行完好，完成记录表也完好（本次收口回写重复写了进度）。
- 影响：不影响已交付行为，但计划不再能自证 M5 的退出条件，后续核查只能借 git 历史复原；也违反计划自身的进度回写约束。
- 处置：修文档，不改历史完成事实。
- Follow-up：从 `0eda22cd^` 取回 M5 定义行复原 §10（进度只留「完成记录」表，表内最终 SHA 不动）。→ gaps 计划 M1 / R-2。

## 3. 完整需求覆盖账本

本期范围按 PRD §7.2 分期矩阵：WORK-F10（AC-1–6）、WORK-F11（AC-1–5）、WORK-F12（AC-1–4）、WORK-F13（AC-1–3）、WORK-F14（AC-1–3）、WORK-N2/AC-4（N2 终验期 P4）、WORK-N4/AC-3（N4 终验期 P4）、WORK-N4/AC-2（每期须满足）。

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F10/AC-1 三者并排、基线标记、名称+概念/字体/色板/特征、未填不显示（`:388`） | AC-1–6 全在本期 | §6.1、M1/M2 | M2 V-1②；M5 V-7 A1；本轮实测 | 静态：`payload.rs` 基线排序（`:229`）+ `page.js` 只渲染已填字段；宿主：V-7 A1 实拍四栏（现状+两改版+截图），基线在前并标注；实测：载荷字段校验按条列出问题 | 已满足 | — |
| WORK-F10/AC-2 视口切换、100% 实际尺寸、键盘切换、截图按原图（`:389`） | 同上 | §6.1、M2/M5 | M2 V-1③；M5 V-7 A1 | 静态：`page.js` `VIEWPORTS {desktop:[1440,900],mobile:[390,844]}`（`:44`）、单方案 + `zoom==='actual'` 才 1:1（`:234`）、`keydown` 左右切换（`:363-370`）、图片按 `pixelRatio` 显示且忽略视口；宿主：V-7 A1 实测切手机视口后三个实页变 390、截图不变；焦点点进方案 iframe 后方向键不切、点页面空白处恢复，页面底部有提示 | 已满足（含已披露的 iframe 焦点限制） | —（限制见第 5 节） |
| WORK-F10/AC-3 选定+说明直接回传、无需复制粘贴、主代理不替选（`:390`） | 同上 | §5.2、M1/M3/M5 | M1 V-1⑤；M4 V-5 R2；M5 V-7 A1 | 静态：`compare.md`「Never submit a choice yourself」+ `ANSWER` 字段说明；宿主：V-7 A1 `--wait` 退出 0，`proposalId=dense`、`liked` 为操作者原文；V-5 R2 同 | 已满足 | — |
| WORK-F10/AC-4 关页无选择→再问一次→有基线保持现状并标待选/无基线按推荐并标未经选择（`:391`） | 同上 | §5.1–5.3、M1/M3 | M4 V-5 R3 | 静态：`compare.md`「No answer」三分支 + `BASELINE KEPT`；宿主：V-5 R3（无浏览器 + 无结构化提问工具）退出 2、未起页、`src/` 无改动、交付标「待选」；实测：`--wait` 无键 → 退出 2 并说明「服务已不在，不是用户的决定」 | 已满足（机制差异见 G-01） | G-01（B）+ 文本补齐 |
| WORK-F10/AC-5 非交互 HTML 不执行脚本不发网络请求；本机地址失效显示不可用与启动说明，其他方案照常（`:392`） | 同上 | §5.4、§5.6、M1 | M2 V-3；M5 V-7 A1 | 静态：`server.rs` `PROPOSAL_CSP`（`script-src 'none'`、`connect-src 'none'`、`sandbox`）与 `INTERACTIVE_CSP`（仅放行脚本，网络仍 `'none'`）；`page.js` 不可用态渲染「不可用：<原因>」+ `serve.command`/`cwd` 的启动说明 + 重试按钮；记录：V-3 用只记录首行的本机 TCP 监听证明无外部请求；实测：非本机 url 载荷被拒并给出改截图的指引 | 已满足 | — |
| WORK-F10/AC-6 决策页/component-review/live 行为不变；单方案不弹页（`:393`） | 同上 | §5.7、M1/M3 | M5 V-8 | 静态：`serve_question.rs` 只改可见性，三个页面文件无 diff（`git diff 8a49b86f`）；`payload.rs:112`「proposals 至少需要两个方案」；**实测**：单方案 `--start` 退出 1 并逐条列出问题，未创建任何状态文件 | 已满足 | — |
| WORK-F11/AC-1 交付写起点/方向/跟随顺序；序列帧证明晚于主角且同向（`:412`） | AC-1–5 | §6.4、M3/M5 | M4 V-5 R4；M5 第十二轮 | 静态：`animate.md`「Choreograph the change」四条 + 正反向时间表 + `Verify` 的 `"frames": 8`；宿主：R4 首轮暴露「被布局推动的内容无法另设延迟」的文本歧义 → 修订后通过；第十二轮交付写明起点、跟随顺序、手感与实际缓动，序列证据在位 | 已满足 | 待核实② |
| WORK-F11/AC-2 形状类过渡无重影；选中指示一块滑块；首次显示/窗口缩放直接就位（`:413`） | 同上 | §6.4、M3 | M4 V-5 R4；M5 第十二轮 | 静态：「Transitions that keep the same content」五条（含「On first display, on window resize, and after fonts finish loading, place it directly」）；宿主：第十二轮反复切换后 `ghosts=0`、减少动效 `anims=0`；R2 复审核对四次快速切换 count=3、三条正确任务、`transform=none` | 已满足 | — |
| WORK-F11/AC-3 「动作+实物」手感命名与弹簧参数；大面积元素过冲 ≤2%（`:414`） | 同上 | §6.4、M3 | M4 V-5 R4；M5 第十二轮 | 静态：`Timing and easing` 的六行手感表 + 「overshoots by no more than 1–2%」（比 PRD 的 ≤2% 更严）；宿主：第十二轮答复含「动作与实物手感、实际缓动」 | 已满足 | — |
| WORK-F11/AC-4 无可见结果的操作说出结果本身（`:415`） | 同上 | §6.4、M3 | M4 V-5 R4 | 静态：「Say the invisible result」正反例；宿主：R4「✓ 已复制」+ 被复制订单号闪底色 + 读屏文本含订单号 | 已满足 | — |
| WORK-F11/AC-5 减少动态效果各状态直接到位；未实际操作的手感标注（`:416`） | 同上 | §6.4、M3 | M4 V-5 R4；M5 第十二轮 | 静态：「Accessibility and control」+ `Verify` 的「normal-speed feel not checked」；宿主：`--reduced-motion` 一轮图；第十二轮如实标「正常速度手感未核对」 | 已满足 | — |
| WORK-F12/AC-1 模糊评价先回清单并停下，回复后才改，交付逐项对账（`:437`） | **PRD v1.5**：改为「回复清单后等用户回复再动手」（用户 2026-11-13 决定） | §6.3、M3 | M4 V-5 R1 复验 | 静态：`vague-feedback.md`「Translate first」——该回复结束本回合、用户答复前不改界面；SKILL.md `:46`；宿主：第一步只回八行清单并结束、`git status` 为空；第二步只改两文件并按同一清单八项逐项说明 | 已满足 | — |
| WORK-F12/AC-2 拿不准时同一次回复给两个方向，或给两个可比变体（live/对比页），不随机换风格（`:438`） | 同上 | §6.3、M3 | M4 V-5 R2；M5 V-7 A3 | 静态：`vague-feedback.md`「When the meaning is unclear」两种处置 + 只问一次；宿主：A3「再精致一点」只调公差、未换风格 | 已满足 | — |
| WORK-F12/AC-3 方向轮中「更大胆/更稳妥」不进入翻译（`:439`） | 同上 | §6.3、M3 | 静态核对 | 静态：`vague-feedback.md`「Boundaries」明确仍属方向轮 | 已满足 | — |
| WORK-F12/AC-4 翻译不出变量的形容词不作为改动理由（`:440`） | 同上 | §6.3、M3 | M4 V-5 R1 复验 | 静态：「Words that do not translate」；宿主：交付中「更高级」未作为改动理由 | 已满足 | — |
| WORK-F13/AC-1 演示数据反例清单、共用场景、可被当真的内容标为示例、不虚构商业声明（`:457`） | AC-1–3 | §6.5、M3 | M4 V-5 R5 + 定向复验；M5 第十二轮 | 静态：`craft-floor.md`「Demo data」整条（含「Add that label as part of the work, not as a decision deferred to the user」）；宿主：R5 初稿漏标 → 定向复验后客户页与订单页均有「演示数据 · 客户均为虚构」；第十二轮页面保留六任务虚构场景与标注 | 已满足 | — |
| WORK-F13/AC-2 同一套图标库、描边与字重相称、尺寸 1–1.25 倍、纯图标按钮有可访问名（`:458`） | 同上 | §6.5、M3 | M4 V-5 R5 | 静态：`craft-floor.md`「Icons」；宿主：R5 两个图标同一画法、纯图标按钮有可访问名称 | 已满足 | — |
| WORK-F13/AC-3 含弹窗输入/多屏导航/内部滚动时分别给核对结果，不含则不查（`:459`） | 同上 | §6.5、M3 | M4 V-5 R5；M5 第十二轮 | 静态：「Structure checks」三条 + 「a structure the surface lacks is neither checked nor mentioned」+「not as extra rounds」；宿主：R5 给出弹窗内输入（注明仿真矮视口）与弹窗内滚动结果、无两屏往返项；第十二轮无此类结构也未出现空检查 | 已满足 | — |
| WORK-F14/AC-1 交付后最多 3 条编号下一步、首条「推荐」、逐条写收益与代价（`:473`） | AC-1–3 | §6.6、M3/M4 | M4 V-5 下一步形式；M5 V-7 A3、第十二轮 | 静态：`routing.md`「Next steps after a delivery」+ SKILL.md `:106`；宿主：A3 两条（`polish` 标推荐、`critique`）各带代价；第十二轮修正后为 3 条编号、首条推荐、每条收益与成本；初版（占位命令、重复对比建议）不合格，同会话只重读 routing 规则后修正 | 已满足 | 待核实② |
| WORK-F14/AC-2 小改动/停在确认点/用户已说明/无人值守不列（`:474`） | 同上 | §6.6、M3 | M4 V-5 R6、A3 | 静态：四类排除 +「确实没有人读答复的运行」口径；宿主：文字小改不列、停在待决事项不列 | 已满足 | — |
| WORK-F14/AC-3 只列当前可执行且本次未做过的 Phoenix 命令/流程；回复编号即进入（`:475`） | 同上 | §6.6、M3 | M4 V-5 下一步形式；M5 第十二轮 | 静态：可列集合与「engineering work … is not a next step here」、「A reply with a number enters that flow」；宿主：把「只列 Phoenix 命令或流程」写进触发句后 A3 与第十二轮均符合 | 已满足 | — |
| WORK-N2/AC-4 对比页命令在四平台可用、行为同 AC-1；Windows 仍在安装预检停止（`:575`） | N2 终验期 P4 | M5 V-9 | M5「V-9 最终候选」 | CI：run `43449295726`（headSha = `968b6539`）四平台 `runtime-smoke-node25.json` 均 passed、`compare=answered`、`capture=screenshot`，四平台日志含 compare 浏览器测试 1 passed、compare_security 2 passed，无 skip；Node 18.20.8 用同一 linux-x64 制品 passed；安装器与平台预检无改动（V-8 受保护路径 diff） | 已满足 | — |
| WORK-N2/AC-1–AC-3 终验持续满足（四平台无需 Rust、缺 engine 给补装命令、无浏览器明确报错、Node ≥18 无 Python） | P1 交 AC-1/AC-2、P3 交 AC-3，**终验 P4** | 承接 [P1 G-03](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md)（B，保留） | P1 M6；P4 M5 | AC-1 仍按 P1 G-03：同一安装器 + 四平台 CI 候选 + 缺 engine 退出 4 给补装命令；`npx` 发布路径待发布时复核（0.7.13 只准备了 CHANGELOG 与包版本，无发布与发布后安装记录）。AC-2/AC-3 由 P1/P3 证据承接，P4 未退化（安装器、浏览器探测代码无 diff） | AC-1：已确认变更；AC-2/AC-3：已满足 | 待核实①（沿用 P1 G-03 的重新评估条件） |
| WORK-N4/AC-3 新命令/规则在 SKILL 命令表与 Setup/Routing、reference、docs、README 均有完整描述（`:609`） | N4 终验期 P4 | §6.8 复述矩阵、M4 | M4 V-4 落位 | 静态：SKILL.md 引擎命令表 `:97`（`compare --start/--wait`）+ Routing `:104`；`compare.md`/`vague-feedback.md`/`animate.md`/`craft-floor.md`/`routing.md` 在位；docs「方案对比页」「设计指引增补」「联网边界」；README Phoenix UI 节三条；`ENGINE.md:17`；总帮助 `detect/src/lib.rs:60` 列出 compare；evals 第 25–30 条；**实测** `render --check` 为 `status: fresh`（19 个 provider、48 份 reference，bundle `sha256:6ac482bf…` 与 A2 验收所用一致） | 已满足 | — |
| WORK-N4/AC-2 动 skill 前 §5.3 清单已更新、diff 逐项核对（`:609` 上表 AC-2） | P1/P4 每期 | §0.6、§5.3 C8–C12/C14 | 提交时序 | 静态：PRD v1.4（`c2e6e241`，2026-11-12 17:08）已含 P4 的 C8–C12 行，先于 M1/M3 的 skill 与引擎提交（`0a49480`、`3adeb8a`，同日 23:25）；M3/M4 记录的 diff 与清单逐项对齐 | 已满足 | — |
| UAT-12/13/15/16/18/19（`:735-742`） | P4 | §9 V-1/V-5/V-7 | M1–M5 记录 | 由上述行覆盖：UAT-12←AC-1–5（V-7 A1）；UAT-13←F12（V-5 R1 复验）；UAT-15←F2/F11（V-5 R4 + 第十二轮）；UAT-16←F14（A3 + 第十二轮）；UAT-18←F13（R5 + 定向复验）；UAT-19←N4（V-4 落位、NOTICE 与发布包 `THIRD-PARTY-NOTICES.md` 的 oil-ui MIT 全文与 `Copyright (c) 2026 oil-oil`） | 已满足 | — |

## 4. Triage 与决策账本

### 待决问题

无。本期没有 C 项，因此未发起交互提问；不存在「稍微后决定」的事项。

### 已决决策

| 决策 ID | 关联 G-ID / 需求 | 用户选择与理由 | 确认日期与来源 | 新目标/验收边界 | 跟进/延期/豁免去向 |
| --- | --- | --- | --- | --- | --- |
| D-（沿用 P1 决策） | WORK-N2/AC-1 | 保留「同期验证替代 npm 安装」的变更 | 2026-11-10 前后，[P1 报告 G-03](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md) 记录 | 发布时走 CI 构建、在已发布包上复核 `npx` 安装 | 不进本期 gaps 计划；重新评估条件见第 5 节待核实① |
| D-（沿用 PRD v1.5） | WORK-F12/AC-1、AC-2 | 模糊评价改为「回复清单后等用户回复再动手」 | 用户 2026-11-13 决定，PRD v1.5 头部与 M4「文本修订」记录 | 清单发出即结束回合；无人值守只交付清单 | 已实现，无动作 |

## 5. 待核实项与审查基线

### 待核实项

| # | 缺失事实 | 影响的需求与结论 | 补证方式 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| ① | 已发布 npm 包上的 `npx @xgent-ai/skills install` 安装与缺 engine 补装路径没有记录（`0.7.13` 只提交了 CHANGELOG 与包版本，无发布与发布后安装验证） | WORK-N2/AC-1 的终验口径（沿用 P1 G-03 的已确认变更）；不影响 AC-4 的四平台结论 | 下次发布时按 CI 构建引擎并在已发布的包上复核安装与补装命令，把结果记入当次的完成记录 | 否（发布需另行授权，属发布期动作） |
| ② | 单次完整 A2 只能证明「同一会话内修正后符合」，不能证明不同模型下自然触发与遵循的稳定性；M5 也如实保留了首次时间表、逐图读取、detector 次数等提示遵循偏差 | WORK-F11/AC-1、WORK-F14/AC-1 的证据强度（不影响「规则已交付、已按规则通过一次」的结论） | 需要时用同样夹具另起若干独立会话复跑同一请求，统计交付是否首版即符合；本轮未新增会话，不把它写成已验证 | 否（补证口径需用户决定投入，属可选加强） |

已披露且本轮确认无动作的限制：焦点进入方案 iframe 后方向键不切、需点页面空白处恢复（页面底部有提示，浏览器行为，V-7 A1 已记录）；「正常速度手感未核对」的披露本身即 PRD 要求的输出。

### 本轮实际执行的验证

| 检查 | 命令/入口 | 结果 |
| --- | --- | --- |
| 交付文本与源一致 | `tools/phoenix-ui` 下 `mise exec --no-deps -- bun scripts/render.mjs --check` | `status: fresh`、`providers: 19`、48 份 reference，bundle `sha256:6ac482bf…` 与 A2 验收所用 bundle 一致 |
| 单方案不弹页（AC-6） | 候选开发构建 `local/phoenix-ui/p4-a2-batched-final-release/phoenix-ui-darwin-arm64 compare --start --payload one.json` | 退出 1，列出「proposals 至少需要两个方案」，未创建状态文件 |
| 只接受本机地址（AC-5） | 同上，`proposals[1].url = https://example.com/` | 退出 1，逐条指出主机只能是 localhost/127.0.0.1/[::1] 并建议改截图 |
| 无回答不是选择（AC-3/AC-4） | 同上，`compare --wait --key nokey` | 退出 2，说明「服务已不在，也没有选择，这是服务故障不是用户的决定」 |
| 无浏览器会话（AC-4/AC-5） | 同上，`PHOENIX_UI_QUESTION_DISABLED=1 compare --start` | 退出 2，说明「没有启动对比页；改用结构化提问」 |
| 收口后是否改动交付面 | `git diff --stat 0eda22cd..HEAD`、`... 8371f77a..HEAD -- tools/phoenix-ui/crates/context/src/compare` | skill 与 `compare/` 源码均无改动（只有 `media/mod.rs` 与 README 的无关行） |

未执行：未重跑 `test:phoenix`、未重跑四平台 CI、未新起 provider 会话（按本次改动为零的判断复用已完成证据，见 §9 复用规则）。

## 6. Follow-up 与 gaps plan

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| G-01 / WORK-F10/AC-4 | `skills/phoenix-ui/src/reference/compare.md`「No answer」段与 `docs/phoenix-ui.md` `:229` 补「宿主没有结构化提问工具时写在答复里再问一次」的一句，并重新渲染 | 两处文本可回读到该前提；`render --check` fresh；按新文本在一次无工具的会话里走完无回答分支（退出 2 → 答复追问 → 交付标待选/未经选择） | 无 | [P4 gaps 计划](PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md) M1 |
| G-02 / 计划 §10 | 从 `0eda22cd^` 取回 M5 里程碑定义行复原 §10，进度只保留在「完成记录」表 | §10 五个里程碑行与五列表头对齐；M5 定义可读；完成记录表的最终 SHA 与快照不变；`git diff` 只触及该行 | 无 | [P4 gaps 计划](PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md) M1 |
| 待核实①/② | 不进入本期实施：分别是发布期动作与可选加强 | — | 发布授权 / 用户决定投入 | 报告保留，不建任务 |
| 已满足、B 保留项（WORK-N2/AC-1、AC-2、AC-3 承接） | 无动作 | — | — | 沿用 P1 G-03 的重新评估条件 |

计划：[PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md](PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md)（`sha256:88fa46e9c1a494210ef8a60134461c1e26278086195dd37d83ee225d183b5c2e`，生成时状态 `Ready`，1 个里程碑 M1，0/1 完成）。理由是两处增量均为纯文本、无范围/安全/数据/契约的未决项，依赖与退路见计划 §11。

## 7. 交付前核对

- 本期 24 条验收与 6 个 UAT 均有覆盖行；计划漏项（无）与本报告新增项（G-02）均已列出。
- A/B/C 各有依据；未把未知写成遗漏，也未把未经确认的降级写成合理变更。
- C 为 0，因此没有待决问题卡；两项沿用决定的来源与范围已登记。
- 两个跟进项都指向真实计划任务；待核实项不伪装成已解决。
- 未宣称整期 PRD 已完整验收：WORK-N2/AC-1 的终验仍依赖发布期补证。
