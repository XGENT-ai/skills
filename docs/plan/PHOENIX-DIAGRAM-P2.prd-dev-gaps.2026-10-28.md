# Phoenix Diagram P2 · PRD 交付覆盖复审（gaps 修复效果）

> 审查日期：2026-10-28（Asia/Shanghai）。模式：**修复效果复审**，不是完整重审。输入：`/review-prd-dev-gaps docs/plan/PHOENIX-DIAGRAM-P2-gaps.md`。按该计划的需求来源，定位到 [PRD v1.0](../PRD-PHOENIX-DIAGRAM.md) P2、[P2 原计划](PHOENIX-DIAGRAM-P2.md)和上轮差异报告。
>
> - **上轮报告**：[PHOENIX-DIAGRAM-P2.prd-dev-gaps.md](PHOENIX-DIAGRAM-P2.prd-dev-gaps.md)，2026-10-23，sha256 `7f4ed460d61b41789e5cc8970302b563b5f967944f3bede24b183fabf1ef0284`，台账评审对象 `98308b27-f2d8-4bf9-ace7-d2abec1022e4`。上轮报告保持不变。
> - **所审对象的前后版本**：PRD 和原计划未变，分别为 `e099e478…4263b`、`56c0fb21…64d8`。[gaps 计划](PHOENIX-DIAGRAM-P2-gaps.md)从上轮交接时的 Ready、0/4（`6948cfc4…`）变为 4/4 已完成（`36764a546045b101fc2c0a28f73c23eb4213df1e4cce91c4032e5c52ade9ec3d`）。最终候选从 `c1703a9` 变为 `5585a5f`。
> - **本轮范围**：
>   - 上轮 G-01–G-06、FU-1–FU-4 及待核实清单（上轮为空）。
>   - gaps 实施直接影响的 P1/P2 AC 回归，包括 D7/D14/D25/D26 的产品修复和新的 64 角色宿主验收。
>   - 本轮新发现的差异。
>
>   P3 的 16 条 AC 只核对去向。
> - **已有处置记录**：[gaps 完成记录](PHOENIX-DIAGRAM-P2-gaps.md#完成记录)，[M1](PHOENIX-DIAGRAM-P2-gaps.records/M1.md)、[M2](PHOENIX-DIAGRAM-P2-gaps.records/M2.md)、[M3](PHOENIX-DIAGRAM-P2-gaps.records/M3.md)、[M4-v4](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md)，以及 [R1 代码评审](PHOENIX-DIAGRAM-P2-gaps.code-review.md)（通过验收，0 findings）。PRD、计划、完成记录与代码均按只读处理。

## 1. 结论与范围

**上轮四项 follow-up 均已关闭，P2 没有未处置的明确遗漏。但 Claude Code 宿主路线的终验不完整，另有一项 workflow 图例作者指引缺口已由用户决定延期。**

本轮新增 A 0、B 0、C 2。两项 C 均已决（D-03），待决 0，待核实 0。需要新增实施的 follow-up 为 0，因此**无需 gaps plan**。另有 1 项文档同步（FU-5），只需更正 gaps 计划 D28 依据栏中的会话次数，不改变任何决定，由计划维护者就地修订即可。

- **旧问题解决情况**：
  - G-01、G-02、G-06：已解决。
  - G-03：按上轮关闭判据已解决。Claude 的 ERD 反例在问句形式上没有通过计划 V-6，这部分另归 G-07。
  - G-04：作为保留的合理变更仍存在，无需动作。
  - G-05：同 bundle 复用的前提已不再使用，不再适用。
- **修复引入的新问题：0。**
  - R1 代码评审无新增 finding。
  - V-5 本机 core 与扩展范围、V-7 四平台、Node18 和本机消费均在新候选上通过。
  - 本轮复跑的六区、四组浏览器失败对照、V-1 和 V-3 均通过。
  - G-07、G-08 是新验收暴露的问题，不是 gaps 修复引入的回归。
- **覆盖**：37 条 P1/P2 AC 中，36 条已满足，1 条部分满足（WORK-F2/AC-1，关联 G-07/G-08）。上轮为 33 条已满足、4 条部分满足。
- **未覆盖或缺证的范围**：
  - Claude Code 路线全部使用第三方模型：v6 经 TokenHub 使用 `glm-5.3-flash` 与 `deepseek/deepseek-flash`（D12），D27 改用 DeepSeek 直连的 `deepseek-flash`。没有 Anthropic 模型证据。
  - 本轮没有重跑四平台 CI、宿主会话和完整浏览器收口，只复跑了上轮复现和局部检查。
  - 没有独立 UI/UX 验收，也不涉及真人指标、P3 和公开发行。

结论边界：在 §6 注明的基线内，P2 的 PRD 覆盖在 Codex 路线完整满足；Claude Code 路线带 D-03 记录的限制。不能据此宣称两个宿主均完整通过，也不能说 P2 交付没有任何限制。报告完成不等于产品验收，本轮也不写 SPMS。

## 2. 上轮问题复审

本轮结果统一使用：已解决、仍存在、部分解决、回归、待核实、不再适用、本轮未复核。证据等级沿用上轮：D=文档自述；S=本轮静态核对；H=既有真实运行或会话原件；V=本轮实际执行或实际查看。RE1–RE7 定义见 §6.2。

| 来源报告 / G-ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| [7f4ed460](PHOENIX-DIAGRAM-P2.prd-dev-gaps.md#g-01--六个有效参考时区被当作非法输入--a) / G-01 | A：六个 SystemV 夏令时区被最终引擎拒绝 | FU-1 → gaps M1；原生 SystemV 规则、固定 ICU77.1 来源，修复提交 `fd0be93` | **已解决** | V：RE1 在 v6 CI 候选引擎上，六区 validate/deliver/check `--strict-provenance` 均为 pass，冻结参考 renderer 均 exit 0，原参考 JSON 未改；RE2 的 format 6/6、timeline 7/7、oracle 与来源检查均为 verified。S：[format 测试](../../tools/phoenix-diagram/tests/format.rs#L163)直接列出六区，已无拒绝豁免。H：[V-4](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L151) 在 v6 实看 24 图并完成六次 review-record，结果为 complete | 无 |
| 同上 / G-02 | A：新建 out-dir 时，浏览器缺席或启动失败被 ENOENT 掩盖 | FU-2 → gaps M2；先准备回执目录，再保存浏览器首因 | **已解决** | V：RE1 重跑上轮四组对照。缺席时 fresh 和 precreated 均为 exit 5 / unavailable / `browser/unavailable`；启动失败时两者均为 exit 4 / failed / `browser/launch`。三个前门均 pass，HTML 保留，证据目录只有 browser-check 回执和输出清理元数据，没有截图。上轮 fresh 缺席的结果是 exit 4、仅报 ENOENT。H：V-4 的 12 组公开命令矩阵在 v6 全部通过 | 无 |
| 同上 / G-03 | A：ERD 作者把未确认的最小参与数和可空性先画成事实，原审核未拦截 | FU-3 → gaps M3：ERD 具体规则、`erd-facts.json` 反例和独立判据；M4 两宿主新会话 | **已解决**（按上轮关闭判据） | S：[SKILL 第 58 行](../../skills/phoenix-diagram/SKILL.md#L58)、[创作默认值第 28 行](../../skills/phoenix-diagram/references/authoring-defaults.md#L28)、description 前置“缺必需事实时仅提必要问题”。H：Codex 的 intent-erd、mermaid-er 和反例均 passed。Claude 的 mermaid-er 在 v6 passed；intent-erd 的 v6（GLM）把两端可选性画反而 failed，D27 换用 deepseek-flash 后 passed。所有会话都没有“先补规则、再请确认”。V：RE6 核读 Claude 反例审核原件，两问覆盖两个缺口、未重问、无 candidate 或 Native，但都是夹带取值的“是否”问句 | 问句形式未通过 V-6，归 G-07 按 D-03 照记；G-03 本身无剩余动作 |
| 同上 / G-04 | B：phoenix-v1 workflow 保语义修复（D-01） | gaps D14 在用户授权下补充可选整图修法和自动图例避让，D26 补充自动标签避让 | **仍存在**（保留的合理变更） | H：V-5 与 V-7 在 v6 通过；R1 代码评审核对了显式 pins 和旧合同保持 | 无 |
| 同上 / G-05 | B：同 bundle 下受限复用宿主记录（D-02） | gaps 修改了 skill bundle，按 C-3/D5 让 64 角色全部在新同包重新取证 | **不再适用** | H：[cohort-freeze](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L151) 固定 64 角色和新 bundle `d4a10828…`；旧宿主记录没有拼入新聚合 | 无；只表示不再用该复用方式，不代表复用曾有误 |
| 同上 / G-06 | B：Reader 高度适配合理，但旧测试未同步、也未进入维护入口 | FU-4 → gaps M3 | **已解决** | V：RE3 的 skill 与 reader-layout 测试 15/15 通过；[原生 runner 第 49 行](../../scripts/test-phoenix-diagram-native.mjs#L49)已列入该套件。H：R1 代码评审的 `npm run test:diagram` Node 143/143 通过。Reader 产品代码没有为迁就测试而回退 | 无 |
| 上轮 §5 待核实清单 | 上轮为空 | — | 不适用 | — | — |

## 3. 本轮新增差异

### A · 明确遗漏：0 项

### B · 合理变更：0 项

gaps 计划中 D7、D14、D25、D26 的产品修复均有用户的明确授权，目的是让行为更符合 F1/F5/N1，不偏离 PRD，因此不作为差异计数。D21–D23 调整的是验收方法：仓库题改为 main-v2、D22 限定成功回执的读取范围、D23 将项目内相对读取记为非阻断。这些调整不改变 PRD 的验收含义，记入 §6.1。

### C · 需 Triage：2 项，均已决

#### G-07 · Claude Code 宿主路线 5 个角色经重跑仍失败，provider 聚合 incomplete · C（已决 D-03）

- **PRD 期望**：
  - [WORK-F2](../PRD-PHOENIX-DIAGRAM.md#work-f2-从意图与已有材料完成创作) 终验期为 P2，要求宿主 Agent 从意图和材料完成候选并修复。
  - [WORK-F11](../PRD-PHOENIX-DIAGRAM.md#work-f11-按-phoenix-方式安装迁移与恢复) 的 provider 范围沿用 phoenix-ui；当前 CLI 为 `install … [--providers claude,codex]`。
  - PRD 没有规定宿主数量。P2 原计划与 gaps 计划把“两宿主各 31 行全过 + ERD 反例 + provider 聚合 pass”作为 F2/F3/F4/N1 宿主行为的终验证据，见 gaps [§9.3](PHOENIX-DIAGRAM-P2-gaps.md#93-冻结角色题面与判据) 的“最终覆盖保持64个角色”及 V-6/V-7。
- **实际交付**：
  - Codex：31/31 正式角色和 ERD 反例全部 passed，provider 归档 pass。
  - Claude：27/31。四个正式角色在原 v6、D27 首轮、D27 第二轮（中性项目根）中各失败一次，共 3 次会话；ERD 反例在 v6 因 TokenHub 403 中断 lane 而未启动，只在 D27 两轮运行，均失败，共 2 次会话。gaps 计划 D28 依据栏写作“每角色原v6一次加D27两轮共3次真实会话”，与 [v6 记录](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L172)“claude/erd-missing-facts未启动”不符，本报告以记录为准。
    - `browser-unavailable`、`visual-pending`：D27 两轮的交接本身合格，失败在于图例把人员步骤标成“外部系统/策略”等系统类别（见 G-08）；`browser-unavailable` 第二轮还补出材料没有的“客户接收处理结果”终步。
    - `stalled-repair`：停止链正确（停滞计数 2 即停、之后无编辑），但答复中推导出的修复界限错误（D15）。
    - `repo-verified`：main-v2 的语义和来源在 v6 与 D27 首轮均通过。失败原因三次各不相同：v6 在固定 collector 字面量后追加了 `; echo "exit=$?"`；D27 首轮读取了已安装的 `scripts/phoenix-bootstrap.cjs` 与 `ENGINE.json`；第二轮读取了案例仓库的 `tools/phoenix-diagram/src/cli.rs`、`lib.rs`。
    - ERD 反例：使用封闭式“是否”问句。
  - [最终聚合](../../local/phoenix-diagram/p2-gaps-ci-v6/aggregate-final-001.json) 的整体 status 与 handoff 均为 incomplete。四平台、Node18、本机和 codex 均 pass；claude-code 为 incomplete，唯一诊断是 `provider:claude-code` / `core/evidence`。
- **证据链**：
  1. gaps M4 → [v6 角色独审](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L172)、[D27 重跑](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L224)、[D27 第二轮](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L265)、[M4 完成](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L286)。
  2. 本轮读取的原件：RE5（聚合与 `providers-final.json` 的 unavailableReason）、RE6（ERD 反例和 visual-pending 第二轮审核原件，以及实拍图）。
  3. 另外三个失败角色的结论来自记录与独审，本轮没有重读其 transcript。
- **影响**：Claude Code 用户在人员审批类 workflow、停滞后的修复建议、限定资料范围的仓库解释，以及 ERD 缺事实问句这几类场景，可能得到错误图例、错误修复界限、越界读取或封闭式问题。记录中有一项 D27 首轮场景名泄露，它污染了 erd-missing-facts 的首轮结果；第二轮已改用中性根，仍然 failed。
- **分类依据**：PRD 没有指定宿主，但 provider 范围和计划终验承诺都包括 Claude Code。是否接受这条路线的残余失败，是验收承诺上的取舍，因此归 C。Root 逐角色把失败归因于宿主协议或语义，未发现产品或 skill 缺陷；其中图例一项同时被记为作者指引缺口，单列为 G-08。PRD 文本层面，只有 F2/AC-1（材料没有的类别和步骤）在该路线未满足。其余几项违反的是计划或 SKILL 的更严格规则，见 §4 覆盖账本。
- **处置**：用户 2026-10-28 选择“这5项不阻断M4（推荐）”（D-03）。失败照记、不改判，claude-code 聚合保持 incomplete。
- **Follow-up**：本期无动作。出现以下情况时重新评估：
  - skill 或 bundle 再次变更，需要新同包重取宿主证据；
  - 可以使用 Anthropic 模型路线；
  - 后续期次的终验仍要求两宿主全部通过，需另行决定。

#### G-08 · workflow 人员步骤只能选系统组件类别，默认图例把人员标成“外部系统/策略/处理逻辑” · C（已决 D-03：延期）

- **PRD 期望**：
  - [WORK-F1](../PRD-PHOENIX-DIAGRAM.md#work-f1-生成十种语义完整的图解) 的 workflow 要保留泳道、步骤和关系语义。
  - [WORK-F2/AC-1](../PRD-PHOENIX-DIAGRAM.md#work-f2-从意图与已有材料完成创作) 要求候选只使用已给或已查证的事实。
  - 现行 [SKILL 第 66 行](../../skills/phoenix-diagram/SKILL.md#L66)要求“线型、颜色与图例都必须对应材料事实”。
- **实际交付**：
  - workflow 节点 `type` 只能取 [componentType](../../skills/phoenix-diagram/schemas/common.schema.json#L119) 的七个系统组件类别；省略 legend 时为 truthful auto（[默认值第 40 行](../../skills/phoenix-diagram/references/authoring-defaults.md#L40)）。
  - [Workflow 默认值](../../skills/phoenix-diagram/references/authoring-defaults.md#L24)没有说明人员步骤应选哪种类别。
  - deepseek-flash 在 D27 两轮中 4/4 次把人员步骤标成系统类别。本轮实际查看了第二轮 visual-pending 的 1440 亮色原图（RE6）：对“申请人 / 主管 / 管理员”的审批流，图例显示“处理逻辑 1 / 策略 1 / 外部系统 2”。
  - 存在合规写法：Codex 全部使用中性 `backend` 并通过；原 GLM 会话使用了自定义图例“人员角色”。
- **证据链**：[图例类别观察](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L284) → D27 两轮审核 → RE6 审核原件与实拍图（PNG sha256 前缀 `9a84f8d1`，与审核记录一致）。
- **影响**：读者会把人员审批误读成系统或策略组件。是否出错取决于宿主模型，不是渲染错误。
- **分类依据**：补齐需要在三者之间取舍：只补作者指引、增加人员或中性类别（改 schema，gaps §0.5 已排除），或改变图例规则。这是产品取舍，因此归 C。用户在 D28 中明确：“workflow人员步骤图例类别的作者指引缺口留作后续改进，不在本期”。
- **处置**：延期（D-03）。目前没有承接的计划或登记位置，本报告和台账保留这一去向。
- **Follow-up**：本期无。下次修订 phoenix-diagram 的 skill/schema，或编写 P3 计划时，决定是否纳入并选择方案；纳入后按 C-3 用新同包重取宿主证据。

## 4. 完整需求覆盖账本

覆盖状态使用：已满足 / 部分满足 / 未满足 / 已确认变更 / 范围外 / 未核实。“本轮未复核”表示本轮只确认该行的证据仍适用于新候选，没有重新执行，结论沿用上轮或 M4 记录。“已满足”只在注明的基线和样本范围内成立。

### 4.1 P1/P2 已交付范围：37 条 AC

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F1/AC-1，§3 F1 | 旧五类事实保持；D7/D14/D25-D/D26 修改了 architecture、workflow、dataflow 的渲染 | 原 M2/M3/M6/M8；gaps M4 V-5/V-7 | M4-v4 v6 | H：V-5 core 15 例（含 legacy 五类）四门、60 图实看并登记 complete；V-7 四平台、Node18、本机均 pass；R1 核对旧合同和显式几何不变 | 已满足 | G-04 保留 |
| WORK-F1/AC-2，§3 F1 | 新五类；六个 SystemV 时区补齐 | 原 M1–M3；gaps M1/M4 | M1、M4-v4 | V：RE1、RE2；H：V-4 六区 24 图实看并完成 review-record | 已满足 | G-01 已解决 |
| WORK-F1/AC-3，§3 F1 | 非法输入明确失败 | 原 M1/M6；gaps M1 | M1 | V：RE2 含不支持时区名的拒绝回归；其余本轮未复核 | 已满足 | 无 |
| WORK-F2/AC-1，§3 F2 | 原五类意图；缺事实先问 | 原 M7/M8；gaps M4 V-6 | M4-v4 | H：两宿主的 missing-date、missing-amount、missing-condition 均 passed；Codex 的 workflow 类角色全部 passed。V/H：Claude 的 browser-unavailable、visual-pending 在 D27 两轮把人员步骤标成系统类别，第二轮还补出材料没有的终步（RE6 实拍） | **部分满足** | G-07、G-08（D-03） |
| WORK-F2/AC-2，§3 F2 | Mermaid 四类 | 原 M7/M8；gaps V-6 | M4-v4 | H：两宿主的 mermaid-flowchart、graph、sequence、state 最终均 passed；Claude 的 mermaid-state 由 D27 重跑后 Root 裁定通过 | 已满足 | 无 |
| WORK-F2/AC-3，§3 F2 | 新五类意图与 ER/class Mermaid；ERD 和类图事实边界 | gaps M3/M4 | M3、M4-v4 | S：ERD/Class 规则；H：两宿主的 intent-erd、intent-class、mermaid-er、mermaid-class、tree、timeline、waterfall 最终均 passed；V：RE6 中 Claude 反例未补造事实，但问句形式未过 V-6 | 已满足 | G-03 已解决；问句形式归 G-07 |
| WORK-F2/AC-4，§3 F2 | 停滞 2 次后停止并报告 | 原 M7/M8；gaps V-6/V-9 | M4、M4-v4 | H：两宿主的 repairable-diagnostic 均 passed；Codex 的 stalled-repair passed；Claude 停止链正确（停滞计数 2 即停、无后续编辑），但答复中的修复界限推导错误（D15，计划层判据） | 已满足 | Claude 的答复准确性归 G-07 |
| WORK-F3/AC-1，§3 F3 | 仓库事实可追溯；repo 题改为 main-v2（D21） | 原 M4/M7；gaps V-6 | M4-v4 | H：Codex 的 repo-verified（D27）、repo-dirty、repo-timeline 均 passed；Claude 的 repo-dirty、repo-timeline passed，repo-verified 的语义和来源在 v6 与 D27 首轮通过，但因宿主协议问题 failed（v6 改动了固定调用字面量，D27 两轮读取授权外实现） | 已满足 | 协议失败归 G-07 |
| WORK-F3/AC-2，§3 F3 | 脏文件、缺来源明确限定 | 原 M4；gaps V-6 | M4-v4 | H：两宿主的 repo-dirty 来源均为 partial，并如实披露 | 已满足 | 无 |
| WORK-F3/AC-3，§3 F3 | 托管链接，无凭据 | 原 M4/M6 | 原 M4/M6；M4-v4 V-5 evidence | H：V-5 的 evidence 范围在 v6 pass；本轮未复核 | 已满足 | 无 |
| WORK-F3/AC-4，§3 F3 | 时间精度、示意属性 | 原 M1/M3；gaps V-6 | M4-v4 | H：两宿主的 waterfall-illustrative、intent-waterfall（Claude 为 D27 重跑）、intent-timeline、repo-timeline 均 passed | 已满足 | 无 |
| WORK-F4/AC-1，§3 F4 | 原五类四门 | 原 M6/M8；gaps V-5/V-7 | M4-v4 | H：V-5、V-7 均 pass；本轮未复核 | 已满足 | 无 |
| WORK-F4/AC-2，§3 F4 | 新五类 showcase 零错误零警告；D26 新增 `composition/label-arrowhead` | 原 M2/M3；gaps D26 | M4-v4 | H：V-5 core 新五类 showcase 四门；D26 两个原 candidate 先红后绿 | 已满足 | 无 |
| WORK-F4/AC-3，§3 F4 | unavailable、failed、视觉未做分别呈现 | gaps M2/M4 | M2、M4-v4 | V：RE1；H：V-4 的 12 组；Codex 的 browser-unavailable、failed、visual-pending 均 passed；Claude 的 browser-failed passed，另两项在 D27 交接合格，但因图例失败 | 已满足 | G-02 已解决；图例归 G-08 |
| WORK-F4/AC-4，§3 F4 | 旧证据或失配不能作为本次通过依据 | 原 M4/M6；gaps V-5/V-7 | M4-v4 | H：V-5 登记后 reviewRecovery、reviewBindings、newDelivery 均 pass；V-7 resume 同项 pass | 已满足 | 无 |
| WORK-F5/AC-1，§3 F5 | Reader 基础操作；D25-A/C 修改了护照 | 原 M5；gaps D25 | M4-v4 | H：D25 红证与修后 Reader 49/49；V-5 Reader 范围 pass | 已满足 | 无 |
| WORK-F5/AC-2，§3 F5 | 十类 lens、Route、Reach | 原 M5/M6 | M4-v4 | H：V-5 explore 范围 pass；本轮未复核 | 已满足 | 无 |
| WORK-F5/AC-3，§3 F5 | Still、Live、打印与导出保持静态语义 | 原 M5；gaps 8067ac9 回归修复 | M4-v4 | H：e7eb4a5 暴露 WebM 悬停改变状态，经 8067ac9 修复；v6 offline/explore 首跑 pass | 已满足 | 无 |
| WORK-F5/AC-4，§3 F5 | 概览、深链接、键盘、可返回 | 原 M5/M6；gaps D25 | M4-v4 | H：V-5 explore pass；聚焦缩放会裁掉图例（D25 E），与上轮 E5 观察一致，Root 判为非阻断 | 已满足 | 记为已知限制，不计差异 |
| WORK-F6/AC-1，§3 F6 | 原五类五格式 | 原 M6；gaps V-5/V-7 | M4-v4 | H：V-5、V-7 pass；本轮未复核 | 已满足 | 无 |
| WORK-F6/AC-2，§3 F6 | 新五类与范围卡片 | 原 M5/M6 | M4-v4 | H：V-5 pass；本轮未复核 | 已满足 | 无 |
| WORK-F6/AC-3，§3 F6 | canonical 导出与探索恢复 | 原 M5；gaps 8067ac9 | M4-v4 | H：同 F5/AC-3 的修复，v6 通过 | 已满足 | 无 |
| WORK-F6/AC-4，§3 F6 | 失败状态准确 | 原 M5/M6 | M4-v4 | H：V-5 范围 pass；本轮未复核 | 已满足 | 无 |
| WORK-F11/AC-1，§3 F11 | 四平台匹配资源 | 原 M8；gaps V-7 | M4-v4 | H：RE5 中新 tgz `030c891a…` 的四平台 consumer 与 Node 18.20.8 均 pass | 已满足 | 终验 P3 |
| WORK-F11/AC-2，§3 F11 | 离线、幂等、provider | 原 M6/M8；gaps V-7 | M4-v4 | H：V-7 本机消费 pass；两宿主 64 项目、128 回执的初装审计 pass | 已满足 | 终验 P3 |
| WORK-F11/AC-5，§3 F11 | 固定身份恢复 | 原 M8 | 原 M8；R1 | H：R1 记录 npm test 及 bootstrap 公开入口测试通过；本轮未复核 | 已满足 | 终验 P3 |
| WORK-N1/AC-1，§3 N1 | 四桌面尺寸；D25-B 修复 390 宽度工具栏 | 原 M6；gaps D25 | M4-v4 | H：V-5 主图 60 张、扩展 92 张实看，Root 判为非阻断项 | 已满足 | 无 |
| WORK-N1/AC-2，§3 N1 | 对确切产物做真实视觉审核 | 原 M6/M7；gaps V-4/V-5 | M4-v4 | H：V-4 六区 24 图、V-5 15 例登记 complete；两宿主 visual-pending 的交接合格（Claude 的失败在图例） | 已满足 | 图例归 G-08 |
| WORK-N1/AC-3，§3 N1 | 十类相同门槛；Reader 测试同步 | gaps M3/M4 | M3、M4-v4 | V：RE3；H：V-5；在 390 宽度下，聚焦实体可能部分越出舞台，属既有镜头行为，护照不遮挡、底栏完整 | 已满足 | G-06 已解决 |
| WORK-N1/AC-4，§3 N1 | 五种界面语言与回退 | 原 M5；gaps V-5/V-6 | M4-v4 | H：V-5 locale pass；两宿主 locale-zh-tw passed（Claude 为 D27） | 已满足 | 无 |
| WORK-N2/AC-1，§3 N2 | 原五类由 Rust 实现 | 原 M6/M8 | M4-v4 | H：V-7 pass；本轮未复核 | 已满足 | 终验 P3 |
| WORK-N2/AC-2，§3 N2 | 新五类由 Rust 实现；SystemV 为原生实现 | gaps M1 | M1 | V：RE2 来源 verified，format/timeline 均为原生实现；运行期不依赖 Node | 已满足 | 终验 P3 |
| WORK-N3/AC-1，§3 N3 | 2.17 五类保持 | 原 M1/M6 | M4-v4 | H：V-5 legacy 五例；两宿主 legacy-lifecycle 最终 passed（D27） | 已满足 | 终验 P3 |
| WORK-N3/AC-2，§3 N3 | 参考十类有效 JSON | gaps M1/M4 | M1、M4-v4 | V：RE1 六区参考副本通过，原件未改；H：两宿主 reference-31-json passed | 已满足 | G-01 已解决；终验 P3 |
| WORK-N4/AC-1，§3 N4 | 确定性与身份 | 原 M4/M6 | M4-v4 | H：V-5、V-7 pass；本轮未复核 | 已满足 | 终验 P3 |
| WORK-N4/AC-2，§3 N4 | 浏览器失败时保留产物并分清阶段 | gaps M2 | M2、M4-v4 | V：RE1 前三门 pass、HTML 保留、状态准确 | 已满足 | 终验 P3 |
| WORK-N5/AC-1，§3 N5 | 文本安全、离线 | 原 M4/M6 | M4-v4 | H：V-5、V-7 的 offline 范围 pass；本轮未复核 | 已满足 | 终验 P3 |

### 4.2 P3 的 16 条 AC：范围外

去向与上轮 [§3.2](PHOENIX-DIAGRAM-P2.prd-dev-gaps.md#32-p3-的16条ac均为范围外不作为本期遗漏) 相同，本轮没有变化：WORK-F7/F8/F9/F10 的全部 AC，WORK-F11/AC-3–4，WORK-N2/AC-3，WORK-N3/AC-3–4，WORK-N4/AC-3，WORK-N5/AC-2。gaps 计划 §0.5 继续排除这些内容，也没有做公开发行或修改活跃安装。

### 4.3 用户旅程与终验

| 旅程/UAT | 本轮核对 | 剩余事项 |
| --- | --- | --- |
| S1 / UAT-1 | 新 tgz 的四平台无 Rust 消费、十类自动门、本机视觉 | 公开发行仍未进行，属 P3 |
| S2 / UAT-2 | 六区参考时间线可生成；ERD 缺事实时先问；参考 JSON 和旧输入均不覆盖原件 | G-07/G-08：Claude 路线的图例与问句形式 |
| S3 / UAT-3 | 仓库来源与 dirty、示意时长 | Claude 的 repo-verified 越界读取，归 G-07 |
| S4 / UAT-4 | 浏览器缺席与启动失败分别表达，旧证据不复用 | 无 |
| S5 / UAT-5 | Reader/导出回归与 D25 修复 | 聚焦缩放裁图例为已知限制 |
| S6 / UAT-6、S7 / UAT-7 | 未纳入本期 | P3 |

F1、F3、F4、F5、F6、N1 在本轮基线内没有剩余差异。F2 带 D-03 记录的限制（F2/AC-1 在 Claude 路线部分满足）。N2/N3 只登记本期交付部分；F11/N4/N5 的终验仍在 P3。

## 5. Triage 与决策账本

本轮没有新的待决问题卡。已有的明确决定直接沿用，不重复询问。D-01、D-02 是上轮登记的；D-03、D-04 是本报告对 gaps 计划内用户决定的映射编号，原文编号见“确认日期与来源”列。

| 决策 ID | 关联 G-ID | 用户选择与理由 | 确认日期与来源 | 新目标/验收边界 | 跟进/延期/豁免去向 |
| --- | --- | --- | --- | --- | --- |
| D-01 | G-04 | 本期修 workflow 布局和诊断，保留事实、质量与停滞原则 | 2026-10-21；上轮 §4 | 不变；D14/D26 在其后另获最小修复授权 | 已实现 |
| D-02 | G-05 | 同 bundle 下受限复用 | 2026-10-22；上轮 §4 | bundle 已改变，不再适用 | 不再使用 |
| D-03 | G-07、G-08 | “这5项不阻断M4（推荐）”：Claude 的 stalled-repair、browser-unavailable、visual-pending、repo-verified 与 ERD 反例照记失败，不阻断；“不接受这些产物为通过，claude-code provider聚合保持incomplete并如实记录”；“workflow人员步骤图例类别的作者指引缺口留作后续改进，不在本期” | 2026-10-28；gaps 计划 [D28](PHOENIX-DIAGRAM-P2-gaps.md#02-决策表)、§9.5 第 5 项、[M4 完成](PHOENIX-DIAGRAM-P2-gaps.records/M4-v4.md#L286) | 只适用于已经独审并经 Root 复核、归因为宿主行为的这 5 个角色；产品、schema、质量、视觉、停止门槛不变 | G-07：照记，无动作；G-08：延期为后续改进，尚无承接位置 |
| D-04 | G-07 | 对失败和未完成的角色换路线重跑：Claude 改用 DeepSeek 直连的 `deepseek-flash`，Codex 沿用原路线，每角色一次；之后对仍失败的 5 个 Claude 角色在中性根再跑一次 | 2026-10-28；gaps 计划 [D27](PHOENIX-DIAGRAM-P2-gaps.md#02-决策表) | 替代 D12 对这些角色的路线；原失败会话保留，不改判 | 已执行；结果见 G-07 |

D-03 是对 M4 验收的明确变更，用来确定 G-07/G-08 的处置。它不能证明 Claude 路线的这些行为已经实现，本报告也没有用它改写覆盖状态。

## 6. 待核实项与审查基线

**待核实清单为空。** 没有缺失原件阻塞差异分类、决策或计划判断。下列限制属于证据适用范围，不是待补证项。

### 6.1 固定基线

- **Checkout**：`codex/phoenix-diagram-p2-gaps-20261023` / `5ebc7aaef75a25529d5419dd4d79358524e557af`，工作树 dirty。dirty 部分只有 `.gitignore`、`.xgent-ai` 台账/经验，以及各轮评审报告。`git diff 5585a5f HEAD` 在 tools、scripts、skills、dist、bin、test、package.json、docs/phoenix-diagram.md 范围内为空，这些路径也没有未提交改动。
- **产品身份**：
  - 当前 171 个构建输入与 v6 清单（`local/phoenix-diagram/d26-root-1/native-4/source-inputs.json`）逐项比对大小和 SHA-256，差异 0。
  - candidate 的 `source.commit` 为 `5585a5f6…`，`inputSha256` 为 `b5506db7…`，bundle 为 `d4a10828…`，tgz 为 `030c891a…`。
  - CI darwin-arm64 候选引擎的 SHA-256 为 `cac20e36ee7cb7bfc914830138e9c7088e74d133650ac67d87c1839ec7ea3436`；`engine-probe` 输出 `phoenix-diagram-engine 0.2.0 input=1 receipt=1 viewer=phoenix-v2`。
- **输入文档**：PRD `e099e478…4263b`；原计划 `56c0fb21…64d8`；gaps 计划 `36764a54…9ec3d`；M4-v4 记录 sha256 前缀 `c0f3a2ba4a99da98`；R1 代码评审前缀 `ed0e10d3ea1971d0`。
- **环境**：darwin-arm64，Node 25.2.1，ICU 77.1，tz 2025b，Rust 经 mise/mbx。本轮没有 dispatch CI、运行宿主会话、push、公开发布或改动活跃安装。
- **验收方法调整**：gaps 计划 D21–D23 经用户确认，调整了验收方法。仓库题改为 main-v2；成功回执只要求读取必要交付字段；项目内对同一原件的相对读取记为非阻断。最终 64 角色覆盖、事实、质量、视觉和停止门槛不变，不改变 PRD 的验收含义。
- **记录时间**：恢复快照、M4-v4 与台账中的部分时间（17:30、20:40、21:00 +08:00）晚于对应提交时间（7415d44 14:59、11bdb2a 15:34、321493c/5ebc7aa 15:50），也晚于本轮核查时钟（约 16:52）。本报告以提交和内容为准，不修改原记录。

### 6.2 证据登记

| ID | 证据与入口 | 本轮结果及限制 |
| --- | --- | --- |
| RE1 | [复现脚本](../../local/phoenix-diagram/p2-gaps-prd-dev-gaps-20261028/reproduce.mjs)（由上轮脚本派生，只换引擎并增加 deliver/check）、[结果](../../local/phoenix-diagram/p2-gaps-prd-dev-gaps-20261028/reproduction-results.json)（sha256 前缀 `b044094055c1e322`） | V：用 CI 候选引擎实际执行。六区：参考 renderer exit 0；validate、deliver、check `--strict-provenance` 均 pass，HTML 948,698 字节且包含时区名；原参考 JSON 未改。四组浏览器对照结果见 §2。没有逐项比对墙钟数值，这部分由 RE2 的 oracle 和 V-4 承担 |
| RE2 | [V-1 日志](../../local/phoenix-diagram/p2-gaps-prd-dev-gaps-20261028/v1-tests.log) | V：`format/generate.mjs --check` 为 475 numbers / 37 timestamps / 87 zones / 32708 instants；`check-phoenix-diagram-source.mjs --check` 为 verified；Rust format 6/6、timeline 7/7（含 `system_v_zones_preserve_the_frozen_incident_and_time_semantics`） |
| RE3 | [V-3 日志](../../local/phoenix-diagram/p2-gaps-prd-dev-gaps-20261028/v3-tests.log) | V：`skill.test.js` 与 `reader-layout.test.mjs` 15/15 通过；静态核对原生 runner 第 49 行 |
| RE4 | SKILL 第 3、17、47、58、66 行；创作默认值第 24、28、31、40 行；`common.schema.json` 第 119 行；`tests/format.rs` 第 163 行 | S：ERD 规则、时区说明、开放问句和图例规则均已落地；workflow 人员类别缺少指引 |
| RE5 | `local/phoenix-diagram/p2-gaps-ci-v6/aggregate-final-001.json`（sha256 `6ac21816…f277`）、`providers-final.json` | H：读取原件。run 40560828981 对应 5585a5f；四平台、Node18、本机、codex 均 pass，claude-code incomplete。本轮没有重新执行聚合，也没有向远端重新认证 |
| RE6 | `local/phoenix-diagram/p2-gaps-provider-v6/reviews-d27b/claude/{erd-missing-facts,visual-pending}/review.md`（前缀分别为 `5f77875a`、`14f74ce8`）；实拍图 `~/.local/share/xgent/phoenix-diagram-acceptance/p2-gaps-v6-acceptance-v6-d27b-39489f/cases/case-001/project/deliverables/evidence-01/account-application.visual-check.1440x900.light.png` | V：实际查看该 PNG，图例为“处理逻辑 1 / 策略 1 / 外部系统 2”。读取 ERD 反例的逐字最终答复。其余失败角色只读了记录，没有重读其 transcript |
| RE7 | [R1 代码评审](PHOENIX-DIAGRAM-P2-gaps.code-review.md) 的本轮实际执行一节 | H：同一 HEAD 与工作树下，npm test 完整重跑 exit 0（502 pass / 26 skip），`test:diagram` exit 0（Rust 373 pass / 2 ignored，Node 143/143）。本报告没有重跑这两项 |

## 7. Follow-up 与 gaps plan

先前的 gaps plan 已承接 FU-1 至 FU-4，均已核实关闭，不需要重复建计划。本轮的两项新差异都已有用户决定，没有新增的确定实施项，因此**无需 gaps plan**。

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| FU-1–FU-4 / G-01、G-02、G-03、G-06 | 已完成 | 见 §2 | 无 | [gaps 计划](PHOENIX-DIAGRAM-P2-gaps.md) M1–M4，4/4 已完成，R1 代码评审通过 |
| G-07 / D-03 / F2 | 无 | — | — | 照记；满足 §3 列出的条件时重新评估 |
| FU-5 / G-07 / D-03（文档同步） | 把 gaps 计划 [D28](PHOENIX-DIAGRAM-P2-gaps.md#02-决策表) 依据栏中的“每角色原v6一次加D27两轮共3次真实会话”改为与记录一致：四个正式角色 3 次，ERD 反例 2 次（v6 未启动） | 改后与 M4-v4 记录一致；D28 的选择和适用范围不变 | 无 | 由计划维护者就地修订。这是单句事实更正，不涉及实施，因此不建 gaps plan；本报告不修改原计划 |
| G-08 / D-03 / F2/AC-1 | 本期无；后续再选方案 | 方案确定后，用人员审批类 workflow 题让两宿主在新包上通过 | 产品取舍；schema 变更需另行授权 | 延期为后续改进，目前未登记承接计划；编写 P3 计划或修订 skill 时带入 |
| P3 的 16 条 AC | 不规划 | 按 PRD 原分期 | P2 结果 | P3 |

本报告没有开始执行任何修复，也没有修改 PRD、计划、完成记录或代码。报告和台账回写尚未提交；`local/` 下的原件只在本机，跨机器重放前需要先同步或保全。

### 下一步

本期 PRD 覆盖复审已完成，没有待决项或缺证项。带 D-03 限制的部分已如实登记；跨期需求的终验期仍以 PRD 为准。FU-5 的单句更正由计划维护者处理。

可选的后续阶段是编写 P3 计划。编写时建议带入 G-08 的去向，以及 G-07 的重新评估条件：

```text
/dev-plan docs/PRD-PHOENIX-DIAGRAM.md P3
```
