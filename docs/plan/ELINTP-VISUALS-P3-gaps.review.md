# ELINTP-VISUALS-P3-gaps · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md) | SHA-256 `209ed0e19bd540cb29c7c046666c3bdf94c3566ecc1947156f9695537eb39715` | 全文（§0–§12） |

- 评审日期：2026-11-13；首次完整评审（非复审）。
- 代码基线：`main` / `29d692703cfca0e81aa9f7cd5d171ec517f10926`，dirty（未提交的只有台账状态页、P3 的 R1/R2 评审报告、P3 差异核查报告和本计划；`tools/`、`skills/`、`test/`、`dist/` 无改动）。
- 背景资料：[P3 计划](ELINTP-VISUALS-P3.md)、[P3 差异核查](ELINTP-VISUALS-P3.prd-dev-gaps.md)（G-01 / D-01）、[PRD](../PRD-ELINTP-VISUALS.md) v1.2、[仓库 AGENTS.md](../../AGENTS.md)。

## 确认问题

1. [P2] F-01 · §9 开发构建的 `--sync-dist` 会覆盖受控 dist 许可文件，超出 §1.2 声明范围且 V-3 看不到
   - 定位：计划 §9 环境 L373（`--sync-dist`）；§1.2 L298–L299（只列 `dist/phoenix-diagram/bundle/**`）；C-4 L237（只 `--sync-bundle`）；V-3 L385（只检查 bundle）。
   - 问题：按 §9 执行开发构建时，`--sync-dist` 先 `rm -rf dist/phoenix-diagram`，再把本地开发构建的 `LICENSE`、`NOTICE.md`、`THIRD-PARTY*` 等文件复制进受控 dist。这些文件在 git 跟踪中；本次本地开发构建的 `THIRD-PARTY-NOTICES.md` 与当前 dist 的内容不同（hash 不同），所以实施后会出现计划未声明的受控文件改动。V-3 只检查 `dist/phoenix-diagram/bundle`，看不到这些差异；实施者可能把它们一并提交，或留下未说明的 dirty 工作树。该命令还会写 `skills/phoenix-diagram/ENGINE.json`，但计划只声明 bundle。
   - 证据：[`scripts/build-phoenix-diagram.mjs` L466–L484](../../scripts/build-phoenix-diagram.mjs#L466) 的 `syncDist` 明确 `rmSync(target)` 后复制除 source-inputs、build-receipt 和引擎资产之外的全部文件；`git ls-files dist/phoenix-diagram` 列出 `LICENSE`、`NOTICE.md`、`THIRD-PARTY-NOTICES.md` 与 `THIRD-PARTY-LICENSES/*`；`sha256sum` 比较本地开发构建与 dist 的 `THIRD-PARTY-NOTICES.md` 分别为 `59cea28b…` 与 `0322230d…`；[`docs/phoenix-ui.md` L253](../phoenix-ui.md#L253) 只要求 `--sync-bundle` 并提交 bundle。
   - 修订：从 §9 开发构建命令中删除 `--sync-dist`，`$E` 用本地输出目录的二进制即可，bundle 仍按 C-4 用 `--sync-bundle` 生成。若确实需要更新 dist，把 `dist/phoenix-diagram` 的非 bundle 文件列入 §1.2 范围，写明来源与是否提交，并把 V-3 第 4 步改为检查 `git status --short dist/phoenix-diagram`（不只 bundle）。
   - 复核：重跑不带 `--sync-dist` 的构建；`git status --short dist/phoenix-diagram` 只出现预期的 bundle 变化；`git diff --stat` 不出现 `THIRD-PARTY*`、`LICENSE`、`NOTICE.md`。无需新增验收行，并入 V-3 第 4 步即可。
   - 置信度：高（脚本、跟踪文件和实际内容差异均已核对）；阻塞：否（删掉该参数即可，但按原文执行会越界）。

2. [P2] F-02 · §8 称“不引入新的运行期失败模式”，但新 SVG + 旧 elintp 是新的硬失败，回滚与恢复说明也不完整
   - 定位：计划 §8 L363–L364；§11 L405；§5.3 L355；§0.1 C-2；§2 L311。
   - 问题：新引擎在静态 SVG 根下写 `<metadata data-phoenix-cards="">`；旧 elintp 的静态元素白名单不含 `metadata`，会把整个 SVG 判为格式错误（退出 2，不覆盖旧页）。这是“新引擎 + 旧 elintp”环境的新增运行期失败模式，不是 §8 所说的“没有新失败模式”。§8 的“回滚即回退提交”也不完整：已经生成的带卡片 SVG 不会随提交回退消失，旧 elintp 仍会拒绝；“重新取图”若用新引擎重取，仍会写 metadata，只有改用旧引擎或先更新 elintp 才行。§11 用“两者随同一仓库和 npm 包发布”来缓解，但 elintp 是独立安装的 skill（C-2 也确认它不以 phoenix-diagram 安装为前提），`static-figure.md` 又是引擎侧文档，旧 elintp 用户不会看到；同仓/同包不等于同一次更新。
   - 证据：当前 [`skills/elintp/scripts/draft.mjs` L9](../../skills/elintp/scripts/draft.mjs#L9) 的 `STATIC_SVG_ELEMENTS` 不含 `metadata`，L344 对白名单外元素直接 `bad`。主 agent 用当前 elintp 对一份在 `</desc>` 后插入 `<metadata data-phoenix-cards="">…</metadata>` 的真实静态 SVG 运行 `render.mjs`：退出 2，诊断 `SVG L1 metadata：元素不在白名单`，未写出页面。P3 旧静态 SVG（`local/phoenix-diagram/elintp-p3-final-out/workflow.system.svg`）确实没有 metadata。README 说明 phoenix-diagram 走 npm 安装器、并从 `npx skills add` 默认发现列表排除；elintp 仍按普通 skill 独立安装/更新（[README.md L44](../../README.md#L44)）。
   - 修订：在 §8 明确写出这个新增失败模式与可执行恢复：旧 elintp 下需更新 elintp，或用旧引擎重新取图；只回退提交不能恢复已生成的新 SVG。§11 的风险行把缓解改成“发布/安装时同版更新 elintp 与 phoenix-diagram，发布说明写明依赖顺序；旧 elintp 报格式错误时不把它当 SVG 缺陷”，并把“最晚确认点”落到具体发布/更新负责人，而不是泛写“发布时”。若用户要求旧 elintp 也能读新 SVG，需要另选载体（例如根上的 base64 `data-*` 属性），这会改变 D2，属于设计变更，不应在本计划里默认实现。
   - 复核：用当前（旧）elintp 对一份带 metadata 的新 SVG 运行 `render.mjs`，确认退出 2 且旧页不变；用新 elintp 对 P3 旧 SVG 运行，确认通过。计划文本核对 §8/§11 与 C-2 的表述一致。实现期验证仍由 V-1 的旧 SVG 用例和新 SVG 用例覆盖，不新增验收行。
   - 置信度：高（旧 elintp 拒绝已实测；安装渠道与 C-2 已核对）；阻塞：否（§11 已把混用版本列为非阻塞风险；但原文的“无新失败模式”和恢复说明不成立）。

3. [P3] F-03 · `docs/phoenix-diagram.md` L101 的“只含”不在 §5.3 的替换范围，改后与 metadata 矛盾
   - 定位：计划 §5.3 L357；[`docs/phoenix-diagram.md` L101](../phoenix-diagram.md#L101)。
   - 问题：§5.3 只把“不含 Reader 页面标题、摘要卡片、节点索引或交互控件”替换成“不含…；卡片以 `<metadata …>` JSON 随 SVG 交付”。L101 原句仍是“SVG 无脚本、无外部资源，只含主图、关系、标签与图内图例”；改后文件实际还含卡片 JSON，“只含”变成不准确表述。C-3 要求现行复述全部更新，V-3 第 3 步只搜“摘要卡片”，不会发现这个残留。
   - 证据：[`docs/phoenix-diagram.md` L101](../phoenix-diagram.md#L101) 原文；计划 §5.3 的替换文本；`skills/phoenix-diagram/references/static-figure.md` L26 用的是“含”而非“只含”，不受影响。
   - 修订：改写 L101 时把“只含”改为“图面只含”，或写成“含主图…；另带 `<metadata data-phoenix-cards="">` 卡片数据”。同步核对 `delivery-contract.md` L86 用的是“包含/不包含”，无需同样处理。
   - 复核：读改写后的 L101，确认“图面”与“文件数据”分开；`git grep '摘要卡片'` 与人工读一遍同时通过。
   - 置信度：高；阻塞：否。

4. [P3] F-04 · 三处“已核实”行号范围与实际不符，`figure` 插入点落在引用范围之外
   - 定位：计划 §1.1 L290（`render` L15–L48、`inlineSVG` L306–L420、`figure` 分支 L593–L607）；§10 L396 的同构引用。
   - 问题：按当前工作树，`static_style::render` 是 L15–L47；`inlineSVG` 是 L306–L414；`figure` 分支是 L593–L609，计划要求插入卡片区的 `result.push(...)` 在 L609，正好落在所引用的 L593–L607 之外。计划把这些称为“已核实·缺口”，行号漂移会让实施者按范围找不到插入点，或误以为 `figure` 分支到 L607 已结束。
   - 证据：[`static_style.rs` L15–L47](../../tools/phoenix-diagram/src/render/static_style.rs#L15)、[`draft.mjs` L306–L414](../../skills/elintp/scripts/draft.mjs#L306)、[`draft.mjs` L593–L609](../../skills/elintp/scripts/draft.mjs#L593)；§10 的 `write_common` 与 `style`/`metrics` 行号（L335–L337、L586–L592）核对无误。
   - 修订：把三处改为 L15–L47、L306–L414、L593–L609；更稳的写法是同时保留 `static_style::render`、`inlineSVG`、`figure` 分支等符号名，行号只作辅助。
   - 复核：改后按行号打开对应文件，确认范围覆盖 `result.push` 的插入点。
   - 置信度：高；阻塞：否。

5. [P3] F-05 · ADR-1 的重新评估触发漏了“只能渲染图片/SVG、不能承载 HTML 卡片”的宿主
   - 定位：计划 §0.3 ADR-1 “重新评估触发” L264；§0.5 把 SPMS 门户留待后续。
   - 问题：当前选择把卡片做成宿主页面的 HTML，前提是宿主能渲染 DOM。触发条件只写“需要卡片保留 Reader 原样外观，或不能解析 SVG 内数据的宿主”；对“能读 SVG、但只能输出图片或纯 SVG”的宿主（门户阅读面、导出图片、非 HTML 阅读器）没有覆盖。这类宿主读得到 metadata，却没有承载 `.figure-cards` 的页面结构，只能退化或另做渲染；下期遇到时会被迫重新决定，而不是在 ADR 触发里提前暴露。
   - 证据：计划 §0.3 L264 原文；§0.5 的“SPMS 门户阅读面实测”仍开放；`skills/phoenix-diagram/references/static-figure.md` 的静态图是“可内联 SVG”，并不保证宿主有 HTML 容器；`render.rs` 的卡片本来就是 SVG 外 HTML。
   - 修订：在“重新评估触发”中补一条“宿主只能渲染图片/纯 SVG，或要求卡片在图内一起显示”；这不改变本期 D-01 的实现，只是把下一次决策的触发条件写全。
   - 复核：读 ADR-1 触发句，确认三类宿主（Reader 外观、不能解析 SVG、不能承载 HTML）都能被识别。
   - 置信度：高；阻塞：否。

6. [P3] F-06 · V-1 的“四张新截图”与四个 static 产物/每份两张的口径不一致，静态审核登记也没有说明
   - 定位：计划 §9 V-1 L383（步骤 1 生成 statechart 与 workflow 各两种策略，共 4 份 static 产物；步骤 3“打开四张新截图”）；§0.3 ADR-1 的审核身份说明（L262）；[`static-figure.md` L34–L40](../../skills/phoenix-diagram/references/static-figure.md#L34)。
   - 问题：每份 `static` 产物按静态图合同有两张截图（亮/暗），4 份产物是 8 张；V-1 只写“打开四张”，没有说明是只抽查 statechart 的四张，还是四个产物各一张。若按“每份两张”执行，workflow 的两份产物各有两张没有进入看图；若只抽查 statechart，应写明 workflow 图面与 P3 相同的依据。另一方面，ADR 已说明卡片数据进入 SVG 的 SHA-256 身份，改卡片会使原审核登记失效；V-1 只打开截图，没有执行 `review-record`，而静态图合同要求两张截图 + `review-record` 才算完成。若 V-1 的产物被当作交付而非测试输入，`visualReview` 会停在 `required-pending`。
   - 证据：V-1 步骤 1 的两种 candidate × 两种字体策略；[`static-figure.md` L34–L40](../../skills/phoenix-diagram/references/static-figure.md#L34) 的“看图与登记”要求两张截图和非空 observations，再运行 `review-record`；计划 §0.3 的“改卡片会使原审核登记失效”；V-1 步骤 3 原文“打开四张新截图看图”。
   - 修订：把步骤 3 写成“打开 8 张（4 份产物各亮/暗两张）”；若只抽查 statechart，写明“workflow 图面与 P3 相同，只做自动门与页面占位检查”，并说明 4 份产物是否作为交付、是否需要 `review-record`。若作为交付，按现有静态图流程为每份登记；若只是测试输入，明确标注，避免把测试产物当成已审核交付。
   - 复核：按修改后的步骤清点截图数（4×2）；若登记，检查 summary 的 `visualReview.status` 与 `deliveryStatus`；若只抽查，读计划文本确认抽查范围与依据。
   - 置信度：高（截图数量按合同可直接计数；审核登记按 ADR 与静态图合同可核实）；阻塞：否。

## 待核实 / 待决策

1. **混用版本的发布协同**：谁在何时把 elintp 与 phoenix-diagram 同版更新，发布说明写在哪里。计划 §11 把最晚确认点放在“发布时”，但本计划不发布 npm，也没有指定发布负责人。可由用户或发布负责人提供；最晚在首次对外更新前确认。不影响 M1 开工；若用户要求旧 elintp 兼容，需要先决定是否改载体（根上的 base64 `data-*` 等）。
2. **`local/` 旧产物的可复现性**：V-1 的 P3 旧 SVG、V-3 的 `baseline-sha256.txt` 和旧引擎都在 `local/`，计划 §9 明确 `local/` 不入库。当前工作树有这些文件；换机器或清空 `local/` 时，需要从 P3 的记录/提交重建旧引擎与基线，计划没有写重建步骤。实施者若在不同工作树执行，需先确认或补这一步；不影响本机执行。
3. **`architecture-quality.md` 缺失**：`review-dev-plan` 的 SKILL.md 要求按需读 `references/architecture-quality.md`，但源目录和已安装目录都没有该文件。本次架构重点按 `review-lenses.md` B 节、代码事实和 ADR 完成；若该参考本应存在，属于 review skill 的独立工具缺口，不影响本计划结论。
4. **实现期才能完成的行为**：`embedded` 字体策略的 static 浏览器检查、360 px/打印/断网的页面实测、`review-record` 的实际登记、V-3 的 22 份内联页面和 V-4 角色演练，均未在本次评审中执行；按计划留给 V-1/V-3/V-4。本次只核实代码与计划的可执行性，不把未运行当成通过。

## 覆盖摘要

- **代码基线**：`main` / `29d692703cfca0e81aa9f7cd5d171ec517f10926`，dirty；`tools/`、`skills/`、`test/`、`dist/` 在评审时无改动。计划版本 SHA-256 见“评审对象”。
- **实际分工**：两个只读 `reviewer` subagent 并行执行，一个 facts（计划主张与代码事实）、一个 architecture（架构与技术路线）；主 agent 同期做 C 节硬检查、全局矛盾整理、旧 elintp 负向实测和报告综合。两个 reviewer 均完成，没有运行有副作用的迁移/部署/发布。
- **覆盖面**：计划全文、P3 差异核查 G-01/D-01、P3 计划的相关决策、PRD v1.2、仓库 AGENTS.md、`static_style.rs`、`render.rs`、`layout/statechart.rs`、`common.schema.json`、`draft.mjs`、`page.css`、`static-probe.js`、`static-figure.md`、`delivery-contract.md`、`docs/phoenix-diagram.md`、`docs/phoenix-ui.md`、构建脚本、README 与安装说明、`local/` 旧引擎/旧输出/基线。
- **脚本与命令**：`check_paths.sh docs/plan/ELINTP-VISUALS-P3-gaps.md .` 通过，22 个路径样式 token，MISSING 0；`check_plan.py` 为 ERROR 0 / WARN 3（`WORK-F9/AC-1`、`WORK-F5/AC-2`、`WORK-N3/AC-3` 自定义 ID 需人工核对）；自定义 ID 已对照 PRD/P3 账本人工核对。脚本通过不证明行为真实；本次的代码行为由 facts reviewer 的只读命令和主 agent 的负向实测补充。
- **已实测（只读）**：当前开发引擎对 `minimal.statechart.json` 的 system 字体 static 通过，产物含规则卡片且无 metadata；`deliver` 的 statechart/workflow Reader HTML 与 `elintp-p3-base-out` 基线 SHA-256 一致；P3 旧 workflow 静态 SVG 经当前 elintp 内联通过；带 metadata 的新 SVG 经当前（旧）elintp 退出 2，诊断“元素不在白名单”，旧页未写。`embedded` 字体策略、浏览器视觉、页面 360 px/打印/断网未在本次执行。
- **C 节硬检查**：范围、发布与评审、需求落点、交付断点逐项核对；未发现生产发布/代码评审被写成里程碑，也未发现范围外系统被纳入。确认的 F-01 是“开发构建命令会写计划未声明的受控 dist 文件”，属于范围/验证指令问题；F-02 是外部兼容/恢复说明问题。没有把未实现的卡片 UI 当成缺陷。
- **评审工具限制**：`review-dev-plan` 引用的 `architecture-quality.md` 不存在（见待核实 3）；`check_plan.py` 的自定义 ID 警告不替代人工映射。

## 关键决策评估

| 选择 | 驱动需求 | 证据/版本 | 收益/代价 | 先进性与持续性 | 结论 |
| --- | --- | --- | --- | --- | --- |
| D1 卡片由宿主渲染 HTML，引擎只给数据 | D-01、WORK-F9/AC-1 | 用户 2026-11-13；`render.rs` L1440 的卡片本来就是 SVG 外 HTML | 图面/截图不变；宿主可复用页面样式；代价是宿主必须实现卡片区 | 与现有 Reader 卡片的形态一致；其他宿主可按数据实现 | 保留 |
| D2 卡片数据写在根下 `<metadata data-phoenix-cards="">` | D-01、单文件随图 | `static_style.rs` 的 `</desc>` 后插入点；`common.schema.json` 三字段；旧 elintp L9 白名单 | 单文件、进入 SVG 身份、无额外 sidecar；代价是旧 elintp 拒绝 | 语义位置正确；旧版本兼容需发布协同（F-02）；根 `data-*` + base64 是可能替代，但可读性差、当前无此要求 | 保留；补失败/恢复说明 |
| D3 只含 `dot`、`title`、`items`，顺序同 Reader | 与 Reader 同源 | `common.schema.json` L288–L322（additionalProperties false、dot 枚举、title minLength 1、items 字符串）；`statechart.rs` L972–L987 | 契约最小、与现有 schema 完全一致；`dot` 只校验不显示 | 未来新增字段会再次改契约；当前 schema 稳定 | 保留 |
| D4 卡片沿用页面变量，不显示七色点 | WORK-N1/AC-2（颜色不是唯一手段） | `page.css` L1–L40 的 `--surface`/`--line`/`--fg`；`dot` 只校验 | 亮暗/打印自动跟随；无需新增色板；代价是失去 Reader 的色点提示 | 与页面设计系统一致；不增加维护面 | 保留 |
| D5 metadata 可选，无卡片不写 | WORK-N3/AC-3 兼容 | `static_style.rs` 的 `scene.cards` 可为空；P3 旧 SVG 无 metadata | 旧 SVG 继续可用；新 elintp 无卡片时不产生空区 | 兼容路径清晰 | 保留 |
| ADR-1 A（metadata）而非 B（summary）/C（sidecar） | 单文件、身份、审核 | `static-summary.json` 含本机路径/审核状态；`scene.cards` 唯一来源 | A 自包含；B 多文件/命名；C 不在身份内 | A 与“随图交付”一致 | 保留；ADR 触发补图片宿主（F-05） |
| 不新增命令/参数/summary 字段（C-6） | 最小改动 | `static`/`finalize` 现有 CLI；`review-record` 现有分流 | 不扩接口；卡片数据只进 SVG | 变化局部 | 保留 |

## 总体结论与修订顺序

**是否需要修改：需要**

- 建议状态：**Ready（修订 F-01、F-02 及 P3 项后；无新增开工阻塞）**。计划自报也是 `Ready`；两者的差异不是状态级别，而是计划文本与验证指令必须先纠正。F-02 牵涉旧 elintp 的兼容策略；计划已把混用版本列为非阻塞风险，但原文“不引入新的运行期失败模式”和“重新取图”的恢复说明不成立。若用户要求旧 elintp 也能读新 SVG，需要另定载体，届时应重新评估 D2，而不是在本计划里默认实现。
- 理由：D1/D2/D3/D4/D5 与仓库事实、schema、Reader 卡片的唯一来源一致；卡片数据、插入点、解析规则和页面结构可执行。确认的问题集中在验证指令（F-01/F-06）、失败/恢复表述（F-02）和三处文字/行号精度（F-03/F-04/F-05），均可在对应批次就地修正，不改变范围或核心设计。
- 修订顺序：先 F-02（把新增失败模式、回滚和发布协同写清，必要时由用户决定是否换载体），再 F-01（删 `--sync-dist` 或扩大范围与检查），再 F-06（截图数与静态审核登记口径），最后 F-03/F-04/F-05（文档、行号、ADR 触发）。改完重跑 `check_paths.sh` 与 `check_plan.py`（若 ID/路径不变，预期仍是 0 missing / 0 error），不需要重跑本次未执行的行为检查；实现期的 V-1/V-3/V-4 按计划执行。
- 解除阻塞的可观察条件：无新增阻塞。若用户选择“必须兼容旧 elintp”，则先完成载体决策并更新 D2/ADR 与 F-02，再开工；若接受“旧 elintp 明确拒绝并提示更新”，按 F-02 的文字修订即可。

下一步：`apply-doc-review docs/plan/ELINTP-VISUALS-P3-gaps.review.md`

## 修订处置与复核

- 来源：本报告首版 `sha256:8f8e7c252528f8fd6176c0179886b0fff70f62a72dfafc15c95795f0137c9b17`；问题 ID F-01–F-06、待核实 1–4 保持不变。
- 目标：[ELINTP-VISUALS-P3-gaps](ELINTP-VISUALS-P3-gaps.md)；修前 `sha256:209ed0e19bd540cb29c7c046666c3bdf94c3566ecc1947156f9695537eb39715`（即所审版本），修后 `sha256:8d4a3a7568b6cabad562cf588faacca3502b4289914be3401b6d1123b389773c`。
- 复核日期与代码基线：2026-11-13；main / `29d692703cfca0e81aa9f7cd5d171ec517f10926`，dirty（同评审时，`tools/`、`skills/`、`test/`、`dist/` 无改动）。
- 本次范围：全部 F-01–F-06 与影响修订的待核实项；这是修订复核，不是重跑的独立评审。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。`scripts/build-phoenix-diagram.mjs` `syncDist` 先 `rmSync(dist/phoenix-diagram)` 再复制构建目录里除引擎、receipt、source-inputs 外的全部文件；`git ls-files` 列出 `LICENSE`、`NOTICE.md`、`THIRD-PARTY*`。本计划不做安装验收（P3 用 `--sync-dist` 是为 V-6 离线安装，本计划无此项）。`skills/phoenix-diagram/ENGINE.json` 与 `dist/phoenix-diagram/VERSION.json` 被 `.gitignore` 忽略，不进 git status | 已修订并复核 | §9 环境：删 `--sync-dist` 并写明原因；V-3 第 4 步改为 `git status --short dist/phoenix-diagram`（整个目录）并点名非 bundle 文件。§1.2 不需扩大。ENGINE.json 已被忽略，没按建议加进 git status 检查 | 回读 §9 环境与 V-3 第 4 步；`$E` 仍是开发构建输出目录里的二进制（`local/phoenix-diagram/elintp-p3-final` 的布局里有 `phoenix-diagram-darwin-arm64`）。实际构建留给 M1 |
| F-02 | 成立。`draft.mjs` L9 `STATIC_SVG_ELEMENTS` 不含 `metadata`；本次独立复现：在 P3 旧 `workflow.system.svg` 的 `</desc>` 后插入 `<metadata data-phoenix-cards="">[]</metadata>`，当前 elintp `render.mjs` 退出 2，诊断「SVG L1 metadata：元素不在白名单」，未写页面；原 SVG 退出 0。README 说明 phoenix-diagram skill 由 npm 安装器随引擎更新，elintp 走普通 skill 安装；`package.json` `files` 包含 `skills`，但安装器只装 Phoenix 两个工具，所以「同包」不等于「同一次更新」 | 已修订并复核 | §8 改为三条：新 elintp 内的失败语义；新增的「新引擎 + 旧 elintp」失败模式与恢复（更新 elintp，或用旧引擎重取）；回滚不改变已生成的 SVG。§11 风险行：影响写明不是 SVG 缺陷；缓解分为本计划内（M1 同一提交，elintp 推送后即可更新，早于引擎发布）和范围外（下一次 `npm run release:phoenix` 的发布者在 CHANGELOG 写明需同时更新 elintp）；最晚确认点写成具体事件。D2 载体不变 | 回读 §8、§11 与 C-2、§0.5（CHANGELOG 属发布流程、范围外）一致。新 elintp 对新/旧 SVG 的行为由 V-1、V-2 在实施时验证 |
| F-03 | 成立。`docs/phoenix-diagram.md` L101 原文是「只含主图、关系、标签与图内图例」；`static-figure.md` L26 用「含」，`delivery-contract.md` L86 用「包含」，不受影响 | 已修订并复核 | §5.3：L101 额外把「只含」改为「图面只含」 | 回读 §5.3；实际改写与 `git grep` 由 V-3 第 3 步执行 |
| F-04 | 成立。当前 `static_style.rs` `render` 是 L15–L47；`inlineSVG` 是 L306–L414；`figure` 分支是 L593–L609，`result.push` 在 L609。§10 引用的 L335–L337、L586–L592 正确 | 已修订并复核 | §1.1 两条「已核实·缺口」改成正确行号并保留符号名；D4 的依据列也有同样的 L593–L607 错误，一并改成 `figure` 分支 L593–L609 | 按新行号对照源码：范围包含插入点 |
| F-05 | 成立。原触发条件没覆盖「能读 SVG、但没有 HTML 卡片容器」的宿主 | 已修订并复核 | ADR-1「重新评估触发」补上「只能渲染图片/纯 SVG（没有可承载 HTML 卡片的页面结构、要求卡片随图一起显示）的宿主」；§0.5 第一条照旧指向 ADR-1 | 回读 ADR-1：三类宿主都能识别 |
| F-06 | 成立。V-1 第 1 步产出 4 份 static 产物，按 `static-figure.md`「看图与登记」，每份有亮、暗两张截图，并需要 `review-record` 才算完成 | 已修订并复核 | V-1 第 3 步：明确 4 份产物是场景作者的交付，逐份打开亮、暗两张（共 8 张），写 observations 后运行 `review-record`；结果列补「每份两张截图」以及 `visualReview.status: "passed"`、`deliveryStatus: "complete"`。选「作为交付登记」而不选「只抽查」：V-1 模拟真实作者流程，而且 ADR-1 说明卡片会改变 SVG 身份，登记能证明现有审核流程能接受带卡片的 SVG | 按新步骤数是 4×2=8 张；登记结果由实施时的 V-1 给出 |
| 待核实 1 | 并入 F-02：最晚确认点和负责人写成「下一次 `npm run release:phoenix` 的发布者写 CHANGELOG 时」，范围外，不阻塞 | 已修订并复核 | §11 | 用户没要求旧 elintp 也能读新 SVG，载体不改 |
| 待核实 2 | `local/` 产物只在本机存在。计划 `@implementation_route` 已规定「计划前提被推翻时停下、就地修订正文」，换机器时缺基线就走这条规则；本机已有这些产物 | 不采纳（不改正文） | — | 换工作树实施时，先确认 `local/phoenix-diagram/elintp-p3-*` 存在 |
| 待核实 3 | `review-dev-plan` 引用的 `architecture-quality.md` 缺失，是评审 skill 自己的缺口，跟本计划无关 | 不适用（范围外，另需处理） | — | 如要补，另开评审 skill 修订 |
| 待核实 4 | 都是实施期验证，计划已分配给 V-1/V-3/V-4 | 已解决无需修改 | — | 实施时执行 |

### 决策与未完项

- 没有新的用户决策。D1–D5、ADR-1 选 A 不变；「旧 elintp 兼容新 SVG（改用根上 `data-*` 等载体）」没有需求依据，没有纳入。如果以后要求兼容，需重新评估 D2。
- 范围外待办：下一次 phoenix-diagram npm 发布时，CHANGELOG 要写明需同时更新 elintp（§11）。

### 修订后结论与验证

- 6 项都已修订并复核；待核实 1 并入 F-02，2 不采纳，3 不适用，4 无需修改。
- 实际运行：`check_paths.sh docs/plan/ELINTP-VISUALS-P3-gaps.md .`：24 个路径 token，MISSING 0；`check_plan.py`：ERROR 0 / WARN 3（同评审时，`WORK-F9/AC-1`、`WORK-F5/AC-2`、`WORK-N3/AC-3` 是自定义 ID，已人工核对）；`check_progress.sh`：0/2，ERROR 0，WARN 1（计划文件未跟踪、工作树里的其他改动是台账和评审文档，不是里程碑交付）。F-02 的旧 elintp 负向实测见上表。
- 未运行：开发构建、`static`、浏览器、`review-record` 和 V-1–V-4（都是实施期检查）。
- 就绪判断：计划保持 **Ready**。修订没有引入新的范围、外部契约或阻塞；关联 PRD 与 P3 计划不需要同步。
