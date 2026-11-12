# ELINTP-VISUALS-P2 · PRD 与开发交付差异核查 · 修复效果复审

- 审查日期：2026-11-12（Asia/Shanghai）；类型：**修复效果复审**。输入为 gaps plan，按其需求来源回到原计划与上轮差异报告。
- 上轮报告：[ELINTP-VISUALS-P2.prd-dev-gaps.md](ELINTP-VISUALS-P2.prd-dev-gaps.md)，当前 `sha256:129353c7b50270b96b901547f6642f19ad0b34a92e109292e784d50a1177b87f`，台账对象 `432f3b5e-45fd-41c8-a55f-8f76add076d8`；上轮代码基线 `0d097e94`。
- PRD：[ELINTP-VISUALS PRD v1.2](../PRD-ELINTP-VISUALS.md) `sha256:ff8353d0…3e0d3b`，与上轮相同；期次 P2。
- 原计划：[ELINTP-VISUALS-P2](ELINTP-VISUALS-P2.md)，本轮未变，历史完成记录不改。
- 处置计划：[ELINTP-VISUALS-P2-gaps](ELINTP-VISUALS-P2-gaps.md) `sha256:16853f3c…837730`；[M1 记录](ELINTP-VISUALS-P2-gaps.records/M1.md) `sha256:9024c9c5…8f96`；[R1 代码评审](ELINTP-VISUALS-P2-gaps.code-review.md) `sha256:4159d24c…e95d`（通过验收，所审 target `68fd7618`）；[R2 UI 验收](ELINTP-VISUALS-P2-gaps.ui-review.r2.md) `sha256:adab4766…7adc`（通过验收，含 UI修复 `a72dbb62`）。
- 所审对象前后版本：elintp `0d097e94` → `b57ea7bc`（`skills/elintp/`、`test/elintp/` 自该提交后至 HEAD 无变化）。观察 checkout：main / `a06bc268abe7b5fb66542c6479d2db588b6cf2f8`，dirty；`skills/elintp/`、`test/elintp/` 干净，dirty 只在其他文档、经验与台账文件。
- 本轮范围：上轮 G-01、G-02（D-01）、G-03；上轮待核实项（0 项）；以及这次修复直接影响的 `WORK-F4/AC-1、AC-2、AC-3、AC-5、AC-6`、`WORK-N2/AC-3`、`WORK-N3/AC-2`。其他行本轮未复核（见 §3）。

## 1. 结论与范围

结论：**上轮两项 follow-up 已解决，修复没有引入新差异；本轮无待决、无待补证。** 这个结论以上轮完整核查为准，加上本轮对受影响行的复核：P2 范围内的需求在注明的基线和证据范围内已覆盖，G-03 的合理变更保留。

| 类别 | 数量 | G-ID |
| --- | --- | --- |
| A · 明确遗漏 | 0 未关闭（上轮 1：G-01 已解决） | G-01 |
| B · 合理变更 | 1（保留） | G-03 |
| C · 需 Triage | 0 未关闭（上轮 1：G-02 已决 D-01、已按决定解决） | G-02 |
| 本轮新增差异 | 0 | — |
| 待核实 / 待决 | 0 / 0 | — |
| 需 follow-up | 0 | — |

- **旧问题解决情况**：G-01、G-02 都已解决；G-03 不需要处理，仍然成立。
- **修复引入的新问题**：无。
- **未覆盖 / 缺证范围**：除修复直接影响的行以外，其余 P2 行本轮未复核，沿用上轮结论。依据是这些行的代码路径自 `0d097e94` 起没有改动（diff 只涉及 `review.js`、`draft.mjs`、`render.mjs`、`templates.mjs`、`draft-format.md` 和 `render.test.js`）。角色验收仍是 Agent 模拟，不是真人 UAT。

报告完成不等于产品交付完成：`WORK-N1`、`WORK-N3` 的整条终验分别在 P1（已完成）和 P5 进行，本报告不宣称它们已整体终验。本轮不需要新的 gaps plan。

## 上轮问题复审

| 来源报告 / G-ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| [上轮](ELINTP-VISUALS-P2.prd-dev-gaps.md#g-01--无图注的截图不能被标注--a) / G-01 | A · 空图注图片（含缺失图片）没有标注按钮，`WORK-F4/AC-1` 部分满足 | gaps M1 §5.1：`68fd7618` | **已解决** | 关闭判据：按钮出现、`quote` 非空且被服务端接受、带图注图片输出不变。① 静态：[draft.mjs:243](../../skills/elintp/scripts/draft.mjs#L243) 只在 `caption.trim()` 为空时加 `data-ref`，有图注的分支字节不变；[review.js:21–25](../../skills/elintp/assets/review.js#L21) 第二遍只追加未登记的 `figure[data-ref]`。引言只允许段落、容器只允许段落和列表（[draft.mjs:154–155](../../skills/elintp/scripts/draft.mjs#L154)），所以图片只会出现在小节顶层，两遍登记覆盖了全部图片。② **本次真实 Chrome 157**（Playwright 无头模式，scratchpad）：同小节“空图注图 → 段落 → 两张带图注图”加一张含连续空格的缺失图。7 个块都有「＋ 添加标注」，空图注图编号追加为 `s1:b4`，原有块号不变，缺失图为 `s2:b2`；提交 HTTP 200，两项 `quote` 与原引用逐字相等，无 `pageerror`。③ 历史运行：M1 V-2（旧 P2 升级、同轮草稿、96 字缺失引用），R1 代码评审（103 字、连续双空格），UI R1/R2 U1 | 无 |
| [上轮](ELINTP-VISUALS-P2.prd-dev-gaps.md#g-02--终稿额外内嵌一份资源快照含截图或阅读器的终稿体积约翻倍--c待决) / G-02 · D-01 | C · 终稿多内嵌一份资源快照，体积约翻倍；D-01 选择终稿不带快照 | gaps M1 §5.2：`68fd7618` | **已解决**（按 D-01 新验收边界） | ① 静态：[render.mjs:44–46](../../skills/elintp/scripts/render.mjs#L44) 只在审阅模式注入 `elintp-assets`，终稿只注入 `elintp-draft`；[render.mjs:27](../../skills/elintp/scripts/render.mjs#L27) 在页面无快照时报错。② **本次 CLI**（scratchpad，4 张仓内截图 + 1 张缺失图）：在没有原资源的空 cwd 用 `--assets-from <审阅页>` 生成终稿，退出 0。终稿不含 `elintp-assets`，含 1 个 `elintp-draft`，无审阅残留；体积 1,242,195 B，同稿 P1 渲染器（`1ee47f4b`）1,241,546 B，比值约 1.0005（上轮为 2,469,998 B 对 1,241,228 B）。用新终稿作 `--assets-from` 时退出 1，stderr 提示改用审阅页或 result.json，输出文件字节不变。③ 定向测试 `node --test test/elintp/render.test.js test/elintp/review.test.js` 16/16 通过（Node v25.2.1），覆盖旧 P2 终稿、审阅页、result.json 来源与符号链接入口。④ 公开文档：[draft-format.md](../../skills/elintp/references/draft-format.md#审阅恢复与终稿) 的数据表、提取脚本、`--assets-from` 来源和终稿保留内容都已同步；`SKILL.md` 只提到用审阅页作来源，无需修改 | 无 |
| [上轮](ELINTP-VISUALS-P2.prd-dev-gaps.md#g-03--每轮重开新的本机审阅地址而不是原地刷新已打开的页面--b) / G-03 | B · 每轮重开新的本机地址，保留 | 无需处置 | **仍存在（B 保留，无需动作）** | `review.mjs` 自 `0d097e94` 起未改；ADR-1 的取舍与重新评估条件不变 | 无 |
| [上轮](ELINTP-VISUALS-P2.prd-dev-gaps.md#5-待核实项与审查基线) / 待核实项 | 0 项 | — | 不适用 | — | 无 |

## 2. 三类差异

本轮没有新增差异。上轮 G-01（A）、G-02（C，D-01）的完整依据保留在[上轮报告 §2](ELINTP-VISUALS-P2.prd-dev-gaps.md#2-三类差异)，分类不变，本轮只更新关闭状态（见上表）；G-03 仍为 B。

## 3. 需求覆盖账本（本轮范围）

证据等级：静态 = 本次读代码；本次 = 本报告实测；历史运行 = 记录或评审中的实际运行。

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F4/AC-1 任一内容块可标注 | P2 | 原 P2 M1；gaps §5.1、M1 | gaps M1 V-2/V-3 | 空图注图片与缺失图片都可标注，有图注的块不变（静态；本次 Chrome；历史运行 M1 V-2、R1、UI R2） | 已满足 | G-01 已解决 |
| WORK-F4/AC-2 提交结果含标题/小节/引文/类型/原话 | P2 | 原 P2 M1；gaps §5.1④ | gaps M1；UI R2 UI-01 | 空图注图片的 `quote` 为完整原引用，服务端接受；结果顶层带 `title`（本次 Chrome；历史运行 UI R2）；成功文案改为“已提交给当前 Agent”（`a72dbb62`，静态） | 已满足（未退化） | — |
| WORK-F4/AC-3、AC-5 续接与旧标注保留 | P2；G-03 已确认变更 | 原 P2 M2/M3；gaps C-3、§3.1⑤ | gaps M1 V-2 | 两遍编号保证原有块 ID 不变（静态；本次 Chrome 中空图注图追加为 `s1:b4`）；旧 P2 `--previous` 续接与同轮草稿重开均附着原块（历史运行 M1 V-2、R1 代码评审） | 已满足（未退化） | G-03 保留 |
| WORK-F4/AC-6 离线终稿保留内容稿，无审阅残留 | P2；D-01 | 原 P2 M3；gaps §5.2、M1 | gaps M1 V-2/V-3 | 终稿只含 `elintp-draft`，内容稿可精确取回（本次 CLI；历史运行 M1 V-3 断网 `file://`） | 已满足 | G-02 已解决 |
| WORK-N2/AC-3 终稿无待审数据 | P2 | 原 P2 §3；gaps §5.2 | gaps M1 V-2 | 终稿无 `elintp-review`、审阅控件与快照（本次 CLI） | 已满足（未退化） | — |
| WORK-N3/AC-2 调用方不改调用方式 | P1 已交付；P2 不得退化 | gaps §5.2；V-1/V-2 | gaps M1 V-1/V-2；R1 代码评审 | 两参数 `render.mjs` 保留，符号链接入口通过测试；带截图终稿体积回到 P1 量级（本次 CLI 与测试） | 已满足（未退化） | G-02 已解决 |
| WORK-F4/AC-4；WORK-N2/AC-5；WORK-N1/AC-1–4 增量；WORK-N3/AC-1 | P2 / 增量 / 不退化 | 原 P2 | 原 P2 M1–M4 | 相关代码路径未改（`review.mjs`、`page.*`、`review.css` 无 diff；`SKILL.md` 与 frontmatter 不变，见 gaps M1 V-4）。UI R1/R2 对新增空图注控件做过键盘、窄屏、打印、无脚本检查 | 本轮未复核（上轮：已满足） | — |
| WORK-N2/AC-1、AC-2、AC-4；WORK-F1–F3 | P1 已终验 | — | P1 记录 | — | 本轮未复核（上轮：范围外，未见退化） | — |
| WORK-F5–F10；WORK-N3/AC-3–5 | P3–P5 | — | — | 不属于本期 | 范围外 | P3–P5 计划 |

## 4. Triage 与决策账本

### 待决

无。

### 已决

沿用上轮 [D-01](ELINTP-VISUALS-P2.prd-dev-gaps.md#已决)（2026-11-11 用户确认：终稿不带资源快照），本轮没有新决定。实际交付符合 D-01 的新验收边界：终稿不含 `elintp-assets`，体积与 P1 同量级；审阅页与 result.json 的快照、续接方式不变；新终稿不能再作 `--assets-from` 来源，旧 P2 终稿仍可作来源；已生成的旧终稿不迁移。

## 5. 待核实项与审查基线

- 阻塞性待核实项：无。
- 本次实测（scratchpad，未改仓库）：定向测试 16/16；CLI 生成审阅页、终稿与 P1 对照，并检查来源报错；真实 Chrome 157（Playwright 1.63.0 无头模式）完成空图注图片的标注与提交。服务正常退出，没有留下自有进程。
- 本次没有重跑的项目，采用历史运行结果：断网 `file://` 阅读、旧 P2 实际升级与同轮草稿重开、超过 80 个字符的缺失引用（本次样例 73 字，这一项看 M1 V-2 的 96 字和 R1 的 103 字）、全新上下文作者模拟（M1 V-3）、UI 视觉/键盘/打印矩阵（UI R1/R2）。这些结果对应的源码与当前相同，只有 `templates.mjs` 的成功文案不同；UI R2 已核对该文案与终稿无关。
- 观察（不属于 PRD 差异，本报告不修改原文）：
  1. gaps plan 恢复快照的「下一步」仍写“独立 UI 复审”，但 [R2 UI 验收](ELINTP-VISUALS-P2-gaps.ui-review.r2.md) 已于 2026-11-12 通过，属于进度回写滞后。
  2. UI修复 `a72dbb62` 只改了 zh-CN 一条成功文案，晚于 R1 代码评审（target `68fd7618`）。本次静态核对无误，也经真实 Chrome 显示，但还没有正式的代码评审轮次。按 UI R2 的建议，可选执行 `review-code`。

## 6. Follow-up 与 gaps plan

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| G-01 / — / WORK-F4/AC-1 | 无（已解决） | — | — | 由 [gaps plan](ELINTP-VISUALS-P2-gaps.md) M1 完成；实施进度以其[恢复快照](ELINTP-VISUALS-P2-gaps.md#恢复快照)为准 |
| G-02 / D-01 / WORK-F4/AC-6、WORK-N3/AC-2 | 无（已解决） | — | — | 同上 |
| G-03 / — / WORK-F4/AC-3、AC-5 | 无 | — | — | 保留 B，无需跟进 |

**无需 gaps plan**：没有未承接的已确定增量，也没有待决或缺证事项。P2 的 PRD 覆盖核查在本基线下完成；跨期需求的终验期仍以 PRD §7 为准。
