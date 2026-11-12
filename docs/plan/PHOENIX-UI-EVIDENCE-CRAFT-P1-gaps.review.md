# PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps · 开发计划完整评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps · capture 步骤失败提示补齐](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md) | `sha256:af3ccb2fd85df5d23f7587751d0b0b3232965b334723d49640baad714a566c6b` | 全文：G-01/G-02、D1–D4、§5.1–§5.4、V-1/V-2、M1、实施进度与提交合同 |

## 1. 确认问题

1. [P2] F-01 · 新增候选不保证能写回状态文件并重放
   - 定位：[计划 D2、§5.1–§5.3，243、289–301 行](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md#L243-L301)；关联 G-01 / WORK-F1/AC-3。
   - 问题：`suggest` 统一把 `textOf(e)` 输出为 `{"text":…}`，但 `textOf` 会回退到 `aria-label` 或 `value`，文字定位却只查询 `innerText`。因此隐藏、离屏或禁用目标可能得到无法重新定位该候选的写法。补足阶段还未继承空文字和 60 字上限过滤，空文字会生成状态文件直接拒绝的 `{"text":""}`，使本期“可照做的修正写法”在常见图标按钮、带 aria 名称的控件等场景失效。
   - 证据：[`page_js.rs:26、35–39`](../../tools/phoenix-ui/crates/cli/src/capture/page_js.rs#L26-L39) 分别定义候选取值与真实文字匹配；[`plan.rs:446–449、480–482、502–507`](../../tools/phoenix-ui/crates/cli/src/capture/plan.rs#L446-L507) 会过滤空白字符串并报“应为非空字符串”；计划相关文字阶段已有空值/长度过滤，补足阶段没有。
   - 修订：在 §5.1 固定新路径的序列化契约：`like` 非空时，只有非空、≤60 字的实际 `innerText` 才输出 `text`；否则复用 D3 的转义规则输出 `css`，不得把 `aria-label`、`value` 或空字符串伪装成 `text`。`like=null` 的既有 missing 分支保持原输出，满足 C-1；同步修改 D2 的“候选均为 text”表述。
   - 复核：改稿后沿“候选取值 → JSON 类型 → 状态文件解析/定位”核对，确认隐藏、离屏、禁用及修订后的遮挡新路径均只会给出可重放的 `text` 或 `css`，missing 旧路径未变。不另增验收门。
   - 置信度：高；由计划算法、定位实现和解析器直接证明。阻塞：否，M1 内可就地修正。

2. [P2] F-02 · “可用的可见元素”仍会包含其他禁用候选
   - 定位：[计划 D2、§5.1、§5.3，243、289–290、301 行](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md#L243-L301)；关联 G-01。
   - 问题：相关文字阶段只排除隐藏元素，不排除禁用元素；补足阶段只排除 `:disabled` 和 `aria-disabled="true"`，遗漏 `pointer-events:none`。当页面另有一个相关文字相同但不可操作的控件时，隐藏/禁用目标会把它列为“可用的可见元素”，调用方复制后仍进入禁用警告，修正没有闭环。
   - 证据：[`page_js.rs:104–109`](../../tools/phoenix-ui/crates/cli/src/capture/page_js.rs#L104-L109) 已把 `disabled`、`aria-disabled="true"`、`pointer-events:none` 三种状态统一视为禁用；计划 §5.1 的两阶段过滤与该定义不一致。
   - 修订：在 §5.1 明确：`like` 非空的隐藏/离屏/禁用及遮挡替代候选，在相关文字与同类补足两阶段都排除上述三种禁用状态；`like=null` 的 missing 旧路径继续沿用原过滤。§5.3 的“可用”与 D2 同步采用这一口径。
   - 复核：核对两个候选阶段使用同一可用性条件，且 missing 的 C-1 不变承诺仍成立；不另建测试组或里程碑门。
   - 置信度：高；计划内部语义与现有禁用定义直接冲突。阻塞：否。

3. [P2] F-03 · 遮挡分支只给遮挡物，漏掉来源要求的同类/相关文字候选
   - 定位：[计划本期范围、D3、§5.3，20、244、298–301 行](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md#L20-L301)；来源为 [G-01 Follow-up，40 行](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md#L40)。
   - 问题：G-01 对隐藏、遮挡、禁用三类目标要求状态文件格式的可见元素写法，并在存在同角色或相关文字的可见可交互元素时列出至多 3 个候选；遮挡时还须含遮挡物定位。当前遮挡设计只追加 `{"css":"<遮挡物>"}`，没有调用候选生成，计划顶部所称“同一套提示延伸到遮挡”因此未完整实现。
   - 证据：来源同一句分别规定“遮挡时含遮挡物的定位”和“有同角色或相关文字……时列出至多 3 个候选”；计划 §5.3 遮挡分支只有遮挡物 CSS，只有禁用分支调用 `suggest`。
   - 修订：在 D3/§5.3 明确遮挡警告先保留原句，再附遮挡物 CSS，并在有替代项时附 `suggest` 的至多 3 个候选；目标自身应排除，候选遵守 F-01/F-02 的新路径契约。在既有 V-2 `covered` 场景提供一个可见替代按钮并同时核对遮挡物与候选，不新增检查 ID。
   - 复核：对照 G-01 逐项确认遮挡物与替代候选都有设计、文件和既有 V-2 落点，且不改变“遮挡仍警告并执行”的行为。
   - 置信度：中；来源文字明确，但候选分句与遮挡物分句的适用范围存在轻微语法解释空间。阻塞：否。

4. [P2] F-04 · G-02 的开放 Shadow DOM 验收没有落到夹具与断言
   - 定位：[计划 G-02 账本、D4、§5.4、V-1/V-2，231、245、276、303–316 行](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md#L231-L316)；来源为 [G-02，42–50 行](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md#L42-L50)。
   - 问题：设计会让 iframe 与开放 Shadow DOM 共用 `scope_note`，但 V-1 和 V-2 的 `wait-framed` 只覆盖 iframe。若实现只恢复 iframe 计数或开放 Shadow DOM 路径拼接有误，现有验收仍会通过，不能证明来源明确要求的开放 Shadow DOM 超时提示。
   - 证据：G-02 Follow-up 同时点名 iframe 与开放 shadow root；[`failures.html:10–15`](../../tools/phoenix-ui/tests/fixtures/capture/pages/failures.html#L10-L15) 只有 `srcdoc` iframe，没有开放 Shadow Root；[`failures.json`](../../tools/phoenix-ui/tests/fixtures/capture/pages/failures.json) 也只有 `framed` 状态。计划 §1.2 未列 `failures.html` 的修改。
   - 修订：复用既有 `step_failures_and_warnings`：在 `failures.html` 增加一个开放 Shadow Root 及内部目标，在 `failures.json` 增加对应的 `waitFor` 出现等待超时状态，并在 V-2 断言原因含开放 shadow root 限制说明；把 `failures.html` 补入顶部开发范围与 §1.2。保留现有 `wait-hidden` 原断言即可，不新增独立检查或收口。
   - 复核：G-02 → §5.4 → 两个夹具 → M1 → V-2 能分别追到 iframe 与开放 Shadow DOM；`until:hidden` 仍按 D4 只在原路径工作，不追加额外反向断言门。
   - 置信度：高；来源覆盖与现有夹具/命令可直接核对。阻塞：否。

5. [P2] F-05 · V-2 把定向缺陷回归扩大为整组 capture 与全工作区检查
   - 定位：[计划 V-2，316 行](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md#L316)。
   - 问题：V-2 以“E2E 暴露的缺陷防复发、定向回归”为准入，却运行全部 capture 集成测试、全部名字含 capture 的 lib 测试，以及全工作区 rustfmt/clippy。这样会把动画、写拦截、脱敏、derived 等未改行为纳入本批退出，与本计划的小增量及定向验证节奏不符，也会让无关回归阻塞交付。
   - 证据：[`capture.rs`](../../tools/phoenix-ui/crates/cli/tests/capture.rs) 含多组独立浏览器场景，本计划实际修改集中在 `step_failures_and_warnings`；[`check-phoenix-lints.mjs:18–20`](../../scripts/check-phoenix-lints.mjs#L18-L20) 实际执行 `fmt --all --check` 和 `clippy --workspace --all-targets`；`src/capture/` 下的 lib 测试还覆盖 derived、plan、report、serve、validity 等未改模块。
   - 修订：V-2 首条命令增加 `step_failures_and_warnings` 过滤器并删除“其余 capture 二进制用例全过”；删除无本期行为映射的 `cargo test --lib capture`。将静态检查收窄为 `phoenix-ui` package 的 fmt/clippy，而不是调用全工作区脚本；保留 V-1 的真实 CLI/Chrome 主路径，不另设全量收口。
   - 复核：V-2 的文字和命令均只覆盖本期缺陷及直接改动目标，不再出现无过滤器的整组测试或全工作区入口。
   - 置信度：高；实际命令范围与计划声明的定向准入直接矛盾。阻塞：否。

6. [P2] F-06 · M1“提交一次”与 SHA 回填后的计划单独提交冲突
   - 定位：[计划 `@commit_protocol`、`@progress_consistency` 与 §10，180–200、324 行](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md#L180-L324)。
   - 问题：§10 要求“M1 完成后提交一次（含代码、测试、夹具与进度回写）”，但提交协议要求先提交成果、回填短 SHA，再单独提交只改计划文件的更新。按字面只提交一次就无法回填 SHA，执行第二次计划提交又违反 §10，实施者无法同时满足两条完成合同。
   - 证据：计划 186 行明确“回填……再单独提交只改计划文件的更新”，193–200 行要求最终每行有 SHA；324 行则限定“一次”。
   - 修订：将 §10 改为“提交点：M1 完成后按 `@commit_protocol` 提交并回填 SHA”，删除“一次”及“含……进度回写”的限制；保留推送需另行授权的约束。
   - 复核：按“成果提交 → SHA 回填 → 计划文件单独提交”的顺序可同时满足 §10、`@commit_protocol` 与 `@completion_handoff`；初始 0/1 进度不变。
   - 置信度：高；同一计划内的直接文本冲突。阻塞：否。

## 2. 待核实/待决策

无。上述六项均有本地计划、来源文档和代码证据，不需要用户或外部方重新决定范围、接口或产品语义。修改后的实际浏览器输出仍由实施后的 V-1/V-2 验证，不属于开工前未知。

## 3. 覆盖摘要

### 基线与证据

- 计划：未跟踪工作树文件；所审 SHA-256 为 `af3ccb2fd85df5d23f7587751d0b0b3232965b334723d49640baad714a566c6b`，评审结束时复核未变。
- 需求来源：[PRD v1.2](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)（WORK-F1/AC-3、AC-7 与边界）、[PRD 与开发交付差异核查](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md) G-01/G-02，以及[已确认决定](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.decisions.md)。
- 代码：2026-11-10，`main` @ `0509ba6c9012f5eb5b6a51507a28d10ad7a66601`；工作树 dirty，当前工作树参与调查；`tools/phoenix-ui` 与 HEAD 无差异。
- 证据等级：文档推演 + 行为级静态代码核查。未运行构建、测试、浏览器、发布或真实宿主会话；未把计划中尚未实施的验收当作已通过事实。
- 图形 UI：不适用。本对象是 CLI、状态文件和 JSON 报告；交付可用性已按调用方修正与重跑旅程审查。

### 四视角分工与裁定

| 视角 | 实际覆盖 | 主要结论 |
| --- | --- | --- |
| facts | 计划主张、定位/解析/报告代码、内部矛盾 | 确认 F-01/F-02，提供 F-03 的来源矛盾证据；现有 hints/warnings 通路可复用 |
| architecture | D1–D4、候选算法、内部字段与 CSS 转义 | 技术路线可行，无新模块/依赖；确认补足候选准入问题，并入 F-01 |
| schedule | 需求→设计→文件→M1→V-1/V-2、依赖、进度与提交 | 确认 F-04/F-05/F-06；单批线性编排成立 |
| delivery | 失败→理解→复制定位→修正重跑、终端与报告通路 | 确认禁用候选修正不闭环，并入 F-02；隐藏主路径设计完整 |

四名独立 reviewer 实际运行；facts 与 delivery 首轮超时后仅基于已有只读调查恢复并交回，主 agent 逐条回读来源和代码裁定。未采纳两项过度验收建议：不为 `until:hidden` 另增反向断言门，也不把 `srcdoc` 限制提示扩展成另建 `frame.html` 并重跑的恢复验收。

### 脚本检查

- `check_paths.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md .`：10 个路径样式 token，MISSING 0。
- `check_progress.sh … .`：0/1、里程碑 1、完成记录 0、ERROR 0；WARN 1 是“工作树有其他未提交改动而本计划未改”的通用提示，当前初稿 0/1 与 M1 一致，不构成进度缺陷。
- `check_plan.py …`：ERROR 0；WARN 2 为 `WORK-F1/AC-3`、`WORK-F1/边界` 自定义 ID，已人工追到 PRD 与差异报告，映射有效。
- 脚本通过只证明结构/路径，不证明行为已实现或验收已通过。

## 4. 映射与实施编排

### 需求与验收映射

| 范围 | 设计与交付 | 验收 | 核对结论 |
| --- | --- | --- | --- |
| G-01 / WORK-F1/AC-3 隐藏、遮挡、禁用提示 | D1–D3、§5.1–§5.3 → `page_js.rs`、`steps.rs`、测试与夹具 → M1 | V-1 隐藏修正重跑；V-2 三类既有场景 | 主链存在；候选有效性、禁用过滤及遮挡替代项须修 F-01–F-03 |
| G-02 / WORK-F1 边界、AC-7 | D4、§5.4 → `LocateError.scope_note` 与超时拼接 → M1 | V-1 iframe；V-2 `wait-framed` | iframe 链路完整；开放 Shadow DOM 缺验收，见 F-04 |
| C-1/C-2 旧判定与报告结构不变 | 复用 `failures[].hints`、`shots[].steps[].warnings` 和既有终端行 | V-2 保留原原因、结果和 missing 断言 | 成立；F-01/F-02 明确要求 missing 旧路径不变 |
| C-3–C-5 文件、mise/Cargo、安全处理 | §1.2、§5、§9 | V-1/V-2 与常规静态检查 | 有落点；验证范围按 F-05 收窄 |
| 进度与提交 | 恢复快照、M1 记录、提交协议 | 完成后每行有 SHA | 进度初始一致；提交次数按 F-06 修正 |

### 依赖边与汇合

| 波次/任务 | 前置及解锁证据 | 可同时做的工作 | 写入边界/共享负责人 | 运行资源约束 | 汇合验收 |
| --- | --- | --- | --- | --- | --- |
| M1 实现 | G-01/G-02 契约及 F-01–F-04 的计划修订 | 不建议并行；单批小改动，委派与集成成本高于收益 | `page_js.rs`、`steps.rs`、既有测试与夹具均由主实施者统一修改 | 批内仅做回答具体问题的最小反馈 | §1.2 全部交付物齐备 |
| M1 批末验证 | M1 交付齐备 | V-1/V-2 默认串行，不硬拆代理 | 主实施者集成，输出各自临时目录 | mise/Rust、离线 Cargo、Chrome；浏览器不在 sandbox 内，测试单线程 | V-1 与收窄后的 V-2 |
| 回写与提交 | 汇合验收通过 | 无 | 主实施者更新 M1 记录、快照和完成记录 | 不推送，除非用户另行授权 | 按 F-06 的提交链完成并提示代码评审 |

无循环依赖、跨系统开发前置或安全并行空间；共享页内脚本和错误结构不适合拆分写入。现有 M1 单批结构保留，不新增里程碑。

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

建议状态：**Ready（落实本报告修订后）**。原计划自报同为 **Ready**，因此差异不是阻塞状态，而是六项已确认、可在 M1 内解决的 P2 文本/设计/验收缺陷；当前没有须用户或外部回答的阻塞项。Ready 只表示修订后的实施前置完整，不表示功能已实现、验收已通过或已发布。

修订顺序：

1. 先修 F-01/F-02 的候选可重放与可用性契约，再修 F-03 的遮挡替代候选，避免三处算法分别漂移。
2. 补 F-04 的开放 Shadow DOM 夹具与既有 V-2 映射，同时保留 `until:hidden` 原语义。
3. 按 F-05 收窄 V-2，确保新增夹具仍由一个定向测试和 V-1 真实场景覆盖。
4. 按 F-06 统一提交合同；最后复核计划内链接、0/1 进度与需求映射。

下一步：`$apply-doc-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.review.md`，核实并修订计划；处置完成后再判定是否按 Ready 开工。本评审未修改原计划或代码，也未自动启动修订/实施。

## 修订处置与复核

- 来源：本报告原始 SHA-256 `4da4557282074a32ca958a3b5b7e387be8256d8d1a841fb6145f507e822c132c`；F-01–F-06 首次处置，原问题、所审基线与原结论保留。
- 目标：[当前计划](PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md)；修前 SHA-256 `af3ccb2fd85df5d23f7587751d0b0b3232965b334723d49640baad714a566c6b`，与所审版本一致；修后 SHA-256 `7387e4f279ea1d7e4befdd8e207395c2718240588fb8fcc1ca32d2bc1eb625aa`。
- 复核日期与代码基线：2026-11-10（Asia/Shanghai）；main @ `0509ba6c9012f5eb5b6a51507a28d10ad7a66601`，dirty；既有改动见报告基线，另有 gaps 计划/评审及状态页未跟踪。亲自读取相关源码，`git diff --stat -- tools/phoenix-ui` 无输出；本次只修文档和处置台账，不改代码、夹具或实施记录。
- 本次范围：全部六项及需求/任务/验证关联；复核为文档和行为静态推演，不是新一轮独立评审。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果 / 剩余动作 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。`page_js.rs` 的 `textOf` 含 aria-label/value 回退，文字定位仅匹配 innerText；`plan.rs` 的 target/required 拒绝空串。原 §5.1 补足统一输出 text 会产生错误写法。 | 已修订并复核 | D2、§5.1：新增路径以非空且 ≤60 字的实际 innerText 输出 text，否则按 D3 转义输出 css；V-2 改为按多候选合同核对。 | 已解决（文档层面）。沿候选取值→JSON→解析/定位逐项推演：空/长文字与 aria/value 回退走 CSS，正常 innerText 走 text；CSS 不保证唯一，保留 D3 指认边界。missing 的过滤、顺序、text 输出均保持原样。实际输出待 M1 的 V-1/V-2。 |
| F-02 | 成立。`page_js.rs` 的目标禁用判定同时含 `:disabled`、aria-disabled（小写 true）、pointer-events:none，原候选两阶段过滤不一致。 | 已修订并复核 | D2、§5.1 两阶段统一三种禁用过滤，§5.3 引用同一“可用”口径。 | 已解决（文档层面）。隐藏/离屏/遮挡/禁用的新路径均用非空 like；missing 的 like=null 不增加过滤，C-1 不变；运行验收留给 M1。 |
| F-03 | 成立。G-01 Follow-up 同时要求遮挡物与存在的同类/相关候选；原遮挡分支只有 CSS。 | 已修订并复核 | D3、§1.2、§5.3、V-2：原警告后附遮挡物 CSS 与 suggest 替代项，排除自身；failures.html 规划追加可见替代按钮。 | 已解决（文档层面）。G-01→设计→文件→M1→既有 V-2 covered 均有落点；遮挡警告并执行不变，无新增检查 ID。 |
| F-04 | 成立。G-02 同时点名 iframe 与开放 shadow root；现有 failures.html/json 及测试只有 iframe，计划未安排 shadow 断言。 | 已修订并复核 | 顶部范围、§1.2、§5.4、V-2：规划开放 Shadow Root 内“影内按钮”，新增 wait-shadow 出现超时状态，与 wait-framed 分别验证限制说明。 | 已解决（文档层面）。两份夹具→M1→V-2 闭环；既有 wait-hidden 断言不变，不另加反向断言门。实际夹具与断言仍待实施。 |
| F-05 | 成立。capture.rs 有多个独立用例；check-phoenix-lints.mjs 执行全 workspace fmt/clippy，原 V-2 超出定向缺陷范围。 | 已修订并复核 | V-2 使用 step_failures_and_warnings 过滤器，去掉 lib capture 与整组退出要求；fmt/clippy 限定 phoenix-ui 包，clippy 不检查依赖诊断。 | 已解决（文档层面）。V-1 真实修正旅程保留，V-2 命令无 workspace/全组测试入口；存量诊断仍按既有 lint-baseline 区分，无新增全量收口。未运行这些实施命令。 |
| F-06 | 成立。§10 的“一次”与 @commit_protocol 明定的 SHA 回填后单独计划提交冲突。 | 已修订并复核 | §10 提交点改为按 @commit_protocol 提交并回填 SHA。 | 已解决（文档层面）。成果提交→回填→计划文件单独提交可同时满足完成合同；0/1、尚未开始及推送授权约束不变。 |

### 决策与未完项

六项均在 G-01/G-02 既定补齐范围内，无新产品/架构取舍，无不采纳或待决项。采纳 F-03 对来源的解释，因为同一句 Follow-up 同时约定遮挡物与替代候选，现有算法即可承接；无需扩展需求。F-01/F-02 的严格合同仅用于 like 非空的新页内候选路径，既有 missing 与 Rust role_hints 不在调整范围。

未完工作是计划 M1 的真实实现、夹具、测试及 V-1/V-2 验收；本次未执行、未登记实施进展，也未更改上游 PRD、原计划或历史交付记录。无需另行同步上游文档。

### 修订后结论与验证

- 已修订并复核 6 项；无需修改 0 项；不采纳 0 项；待核实/待决/待修订 0 项。仅关闭本报告的文档缺陷，未声称实现通过验收或独立复审通过。
- **当前计划判定：Ready**。需求、候选契约、两种作用域提示、交付文件、M1 与验证/提交合同已一致，无开工前阻塞；实施进度仍为 0/1。
- 本次验证对应：路径与文件范围→`bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md .`：11 个 token，MISSING 0；进度/提交合同→`bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md .`：0/1，ERROR 0，WARN 1（未跟踪初稿无法被脚本识别为已修改；人工确认无实施进展）；需求/设计/验收表→`python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md`：ERROR 0，WARN 2（WORK-F1/AC-3、WORK-F1/边界已追到 PRD 与 G-01/G-02）。
- 逐项回读计划 D2–D4、§1.2、§5、V-1/V-2、M1 与提交协议，核对 diff 和本地链接；原报告正文保留。未运行构建、Rust 测试、浏览器或全仓检查，本次静态文档复核足以判断修订是否消除上述设计/映射问题，未来行为验收不能由其替代。
- 下一步（仅建议）：`/goal 按 docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1-gaps.md 开发`。
