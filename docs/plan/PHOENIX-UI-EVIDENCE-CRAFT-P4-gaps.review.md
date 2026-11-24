# PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps 开发计划评审报告

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps · 对比页无回答的宿主回退文本与原计划收口里程碑定义复原](PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md) | sha256:88fa46e9c1a494210ef8a60134461c1e26278086195dd37d83ee225d183b5c2e（未提交工作树文件） | 全文 |

背景资料（不属修订目标）：[PRD WORK-F10/AC-4](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)、[P4 PRD 交付覆盖核查差异报告](PHOENIX-UI-EVIDENCE-CRAFT-P4.prd-dev-gaps.md)（G-01、G-02 的来源与跟进表）、[原 P4 计划](PHOENIX-UI-EVIDENCE-CRAFT-P4.md)（R-2 的复原对象）。

## 确认问题

1. [P2] F-01 · 顶层排除与 C-2 的「已完成里程碑定义行只读」未写明 R-2 复原例外
   - 定位：计划 `:21`（顶层排除句）与 `:233`（C-2）；被字面禁止的恰是 `:214`（M1）、§0.1 R-2、§0.2 D2、§1.2 表、V-3（`:279`）与 §11 风险表第 2 行共同要求复原的那一行
   - 问题：M5 既是「已完成里程碑」又是「§10 的收口里程碑」，两条从句在被改的那一行重叠——一处授权改、两处声明只读。V-3 的成功判据「`git diff` 只触及该行」按排除句字面即构成违规。实施者按恢复协议先读范围与排除，最坏情况会拒绝或质疑复原动作；通读全计划虽可消解，但计划自身的约束不应互斥。
   - 证据：计划 `:21`、`:233` 与 §0.1 R-2、§9.1 V-3 并排即可证；原计划 §10 `:736` 现为六列进度行（M5 定义行位置），同一进度信息已完整保留在原计划「完成记录」表 `:225`，复原不丢任何事实（facts reviewer 独立复核 `0eda22cd^:736` 五列定义行在案）。
   - 修订：顶层排除句与 C-2 各加同一括注例外，如「（§10 收口里程碑 M5 的定义行按 R-2 复原，是唯一例外；M1–M4 定义行、完成记录表、恢复快照与历史 SHA 只读）」。§12 末条「复原后不要再把进度写进去」保留不动。其余排除项（引擎、evals、其他 reference/skill、README、npm、P5）不变。
   - 复核：改后授权从句与只读从句覆盖不相交的行集合；全文 `rg -n "定义行" docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md` 核对口径一致。改稿后的行为验证由既有检查 V-3 承担，不另设断言。
   - 置信度：高（两处原文并排即可证，两位 reviewer 分别独立发现于不同位置）；阻塞：否。

2. [P3] F-02 · V-4 的两个命令片段按字面不可执行
   - 定位：计划 `:280`（V-4 命令列）
   - 问题：`bin/xgent-skills.js install --release-dir` 缺 `=DIR`——CLI 只接受 `--release-dir=` 前缀形态，裸 `--release-dir` 落入未知选项，打印用法并 exit 1；`PHOENIX_UI_QUESTION_DISABLED=1 phoenix-ui compare --start …` 的裸 `phoenix-ui` 不是安装后的可用入口，真实入口是 `<skill-base-dir>/scripts/phoenix-ui`（或 `npm exec` 形式）。实施者一次试错即可自纠，模式本身已被原 P4 计划 V-7 证实可跑通，不阻塞交付；但 §9 的命令列按合同应可直接运行。
   - 证据：`bin/xgent-skills.js:337-345`（选项解析：仅 `--no-phoenix-ui`、`--keep-impeccable` 与 `--providers=`、`--release-dir=` 前缀，未知选项 usage + exit 1，主 agent 独立复核）；`docs/phoenix-ui.md:25`（`--release-dir=DIR`）；生成参考 `skills/phoenix-ui/reference/compare.md:33-35`（入口写法）；原计划 §9.1 V-7 行的成功命令形态。
   - 修订：V-4 命令列改为可照抄形态：安装写 `bin/xgent-skills.js install <夹具目录> --release-dir=<仓库>/local/phoenix-ui/development-release`；入口写「安装后的 `skills/phoenix-ui/scripts/phoenix-ui` compare --start …」。
   - 复核：改稿后对照 `bin/xgent-skills.js:337-345` 的解析与生成参考 `:33-35` 的入口逐字核对 V-4 命令列；行为验证由 V-4 本身承担。
   - 置信度：高（CLI 解析与入口均有源码级证据）；阻塞：否。

## 待核实/待决策

1. CI run `43449295726`（headSha `968b6539`，四平台 + Node 18 全 success）只有差异报告的记录证据，本地无 CI 日志。影响 §9.2「不跑 `test:phoenix`」的依据强度；本地已独立核实候选以来 compare 源、CLI 与 skills src 零改动，是独立佐证，V-1 的 render fresh 也会在批末兜住生成面。责任人：有网络权限者；最晚确认点：M1 批末前如需升级依据强度再查，当前不阻塞（本地零改动 + fresh 门已足以支撑本期不重跑的判断）。

## 覆盖摘要

- 基线：2026-11-24（+08:00）；仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；HEAD `2f64baa6`（main），与计划自报基线一致；工作树脏文件仅为该计划（untracked）、差异报告（untracked）与 SDLC 状态页登记，不影响事实核查。
- 分工：两位只读 reviewer 并行——facts（计划主张逐条对照代码与历史证据）、architecture（D1/D2/V-4 设计与持续性、跨文档闭环）；主 agent 同期做硬检查并逐条复核候选问题的原始证据（CLI 解析、`0eda22cd^:736`、`docs/phoenix-ui.md:229`、`vague-feedback.md:43`、`evals/scenarios.md:30`、PRD `:388-393`）后合并裁定。两位 reviewer 对事实底座结论一致：A-1 可取回、R3 终验证据在 M4 记录 `:22` 在案、`--start` 退出 2 与引擎 `compare/mod.rs:311-315` 及 docs `:230` 三方一致、生成物 fresh、evals 场景 26 失败信号已覆盖「把无回答当成选了推荐」。
- 主 agent 硬检查（C 节）：范围越界——无（改动面全部落在 docs、生成物与原计划 §10 一行，可追溯到 R-1/R-2/G-01/G-02）；发布与评审——提交点不推送（推送需用户授权），代码评审正确地未列为里程碑；需求落点——R-1→V-1/V-2/V-4、R-2→V-3、C-1→V-1、C-2→V-3、C-3 结构性满足，无孤儿需求；交付断点——无（纯文档交付，V-1 fresh 门保分发一致）。
- 脚本检查：`check_paths.sh` 通过，唯一 MISSING 为 `PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.records/`，计划 §1.2 已标「★ 新增，实施后建」，非虚构复用路径。
- 跳过项说明：`architecture-quality.md` 不在 review-dev-plan 的 references 下，architecture reviewer 按 review-lenses B 节执行；未运行 render、compare 或安装（只读约束），V-4 端到端属实施期收口检查。

## 关键决策评估

| 选择 | 驱动需求 | 证据 | 收益/代价 | 先进性与持续性 | 判定 |
| --- | --- | --- | --- | --- | --- |
| D1：回退写法落 `compare.md` 参考，docs 只留一句并指回 | R-1（G-01 的文本缺口） | `docs/phoenix-ui.md:229` 绑定句；`vague-feedback.md:43`「where the host has one」同构先例；render 管线 + C-1 fresh | 行为契约单一归属（src），docs 是摘要+指针；代价是每次双处同步 + V-2 的 `rg` 轻量漂移网 | 可持续：语义再变仍只改 src 一处+渲染+docs 一句；「docs 纳入渲染」对一句文本不成比例，正确拒绝 | 保留 |
| D2：从 `0eda22cd^` 逐字复原，不重写 | R-2（G-02 记录缺陷） | `0eda22cd^:736` 五列定义行（①–⑦ 与 V-6–V-9 一一对应）；完成记录表 `:225` 已有同进度行 | 定义（要求了什么）+完成记录（交付了什么 SHA）恢复计划自证；零信息丢失 | ①–⑦ 是当时实际执行的步骤；重写“更干净”反而与实情不符并构成改史 | 保留（附 F-01 的例外措辞修订） |
| V-4 有界真实使用验收 + §11 非阻塞退路 | R-1 验收行；差异报告 G-01 follow-up 明示 | 文本存在性（V-1/V-2）证不了「新会话真的照做」，V-4 就是该行为的场景 E2E；安装入口复用 P4 V-7 | 一次会话覆盖改动行为的可遵循性；失败信号由 evals 场景 26 承接，不扩判据 | 双向恰当：未为一句文本加额外验收门，也未拿旧证据冒充新文本已验；退出 4+无工具组合靠文本同一性（V-2）覆盖，比例正确 | 保留 |
| §0.5 排除与跨文档闭环 | 差异报告待核实①②、G-01 判据已覆盖 | scenarios.md:30；遵循率统计与发布期安装验证均无本期授权 | 每条排除有理由与去向；PRD 只读正确，G-01/G-02 关闭者是后续复审，不由本计划排评审 | 闭环无断点：G-01 验收↔V-1/V-2/V-4、G-02 验收↔V-3 映射成立 | 保留 |

## 总体结论与修订顺序

是否需要修改：**需要**。

理由：确认问题 1 条 P2（F-01 措辞互斥）+ 1 条 P3（F-02 命令形态）。建议状态：**Ready（修订后）**——计划自报 Ready，差异只在上述两处文本须先改；无影响范围、安全、数据或外部契约的未决项，不构成 Blocked。F-01 两位 reviewer 分别在两处独立发现，证据已由主 agent 独立复核；事实底座（A-1、R3 证据、退出码三方一致、渲染 fresh、§9.2 前提）全部核实成立。

修订顺序：先 F-01（`:21` 顶层排除与 `:233` C-2 各加一句括注例外），再 F-02（`:280` V-4 命令列改为可照抄形态）；两处互不依赖，可一次改稿完成。改稿后无需重审全文：V-3 既有检查覆盖 F-01 的改稿正确性，V-4 本身覆盖 F-02 的命令可执行性。

下一步：

```text
/apply-doc-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.review.md
```

---

## 修订处置与复核

- 来源：本报告原文（sha256 `51d6292a07b30dc32a98202d93151dc8d32753ad09ef339ce7c33a697c3ceb3d`；原问题 ID F-01、F-02 与「待核实/待决策 1」身份不变，以上各节的结论、证据与基线保持原样）。
- 目标：[PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md](PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md)；修前 `88fa46e9c1a494210ef8a60134461c1e26278086195dd37d83ee225d183b5c2e`（与报告所审版本逐字节相同，无版本漂移），修后 `2a373d961532895c8880123fddb475f3436254dc03e524d818e9292c58b672e5`。
- 复核日期与代码基线：2026-11-24（+08:00）；仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；HEAD `2f64baa6`（main）。工作树脏文件仅为本计划与差异报告（均 untracked）及 SDLC 状态登记。本轮无代码改动，未运行渲染、compare 或安装。
- 本次范围：报告全部 finding（F-01、F-02）与「待核实/待决策 1」；用户未限定条目，与报告的评审范围（全文）一致。

| 问题/报告章节 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果/后续 |
| --- | --- | --- | --- | --- |
| F-01（P2）顶层排除与 C-2 未写 R-2 例外 | 成立。读原文 `:21`、`:233`，与 §0.1 R-2（`:231`）、§0.2 D2（`:242`）、§9.1 V-3（`:279`）、§11 第 2 行、§1.2（`:266`「只读，§10 例外」）并排比对：被字面禁止的正是原计划 §10 收口里程碑的定义行，而它同时是「已完成里程碑」。原计划现状读自 `sed -n '/^## 10/,/^## 11/p'`——M5 行为六列进度行、表头五列；`git show 0eda22cd^:docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md` 第 736 行取回五列定义行（阶段收口，①–⑦），与 A-1 一致。两条只读从句覆盖了唯一被授权修改的行，计划自身约束互斥。 | 已修订并复核 | `:21` 顶层排除与 `:233` C-2 各加同一括注例外（§10 收口里程碑的定义行按 R-2 复原为唯一例外；其余里程碑定义行、完成记录表、恢复快照与历史 SHA 仍只读）。§12 末条与其余排除项未动。偏离建议处：不写 `M5`、`M1–M4`，改写「§10 收口里程碑／其余里程碑」——本计划只定义 M1，写入原计划里程碑编号会被 `check_plan.py` 判为未定义引用（实测 ERROR 4），且与本文件既有写法（`:214`、`:231` 均称「收口里程碑」）一致。 | 授权集合与只读集合已不相交：授权＝§10 收口里程碑定义行；只读＝其余里程碑定义行、完成记录表、恢复快照、历史 SHA。V-3 的「`git diff` 只触及该行」不再与排除句冲突。行为验证仍由 V-3 承担，本处不另设断言。 |
| F-02（P3）V-4 命令片段不可执行 | 成立。`bin/xgent-skills.js:335-346`：flags 白名单只有 `--no-phoenix-ui`、`--keep-impeccable`、`--providers=`、`--release-dir=` 前缀；裸 `--release-dir` 落入 unknown，走 `usage()` 加 `exit 1`，且 `releaseDir` 只从 `--release-dir=` 切片。入口：生成参考 `skills/phoenix-ui/reference/compare.md:33-35`（src 为 `{{scripts_path}}/phoenix-ui`）用 `<skill-base-dir>/scripts/phoenix-ui`；安装后 skill 落在 `<target>/.claude/skills/phoenix-ui`（`skills/phoenix-ui/scripts/phoenix-bootstrap.cjs:70` 以 `.claude/skills/phoenix-ui/scripts/ENGINE.json` 判身份；本机 `.claude/skills/phoenix-ui/scripts/phoenix-ui` 存在）。`PHOENIX_UI_QUESTION_DISABLED` 在引擎内有效。 | 已修订并复核 | `:280` V-4 命令列改为可照抄形态：安装 `bin/xgent-skills.js install <夹具目录> --release-dir=<仓库>/local/phoenix-ui/development-release`；入口 `<夹具目录>/.claude/skills/phoenix-ui/scripts/phoenix-ui compare --start --payload two.json`。附带一处报告未列的同源缺陷：该安装前置的开发 release 目录当前不存在，且 `skills/phoenix-ui/scripts/ENGINE.json` 记 `npmPackageVersion 0.7.12` 而 `package.json` 为 `0.7.13`，不先构建会在预检阶段因清单不符失败——故命令列写明先按 `docs/phoenix-ui.md` `:291` 在 `tools/phoenix-ui` 下跑一次开发构建（`scripts/build-phoenix-ui.mjs:99-110` 会同步重写 `dist/phoenix-ui/VERSION.json` 与 `ENGINE.json`）。这是对建议列的最小扩写，理由记此。 | 改稿后逐字核对：安装形态与 CLI 解析一致；入口与生成参考 `:33-35` 及 P4 V-7 的成功形态一致。端到端可执行性仍由 V-4 在收口时承担，本次不代跑。 |
| 待核实/待决策 1（CI run `43449295726` 缺日志） | 本轮网络可用（`gh` 已认证，读前加载 `.env.local`），已取回 run 级与逐 job 结果：workflow `Phoenix`、`push`/`main`、headSha `968b6539`；UI 四平台（darwin-x64、darwin-arm64、linux-x64、linux-arm64）与 UI Node 18 portable consumer smoke 全 `success`；Diagram 三平台 `success`、Diagram Native darwin-arm64 `cancelled`，故 run 级 conclusion 为 `failure`。即报告所写「四平台 + Node 18 全 success」只在 UI 侧成立。原计划 V-9（`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md:716`）已明确「Diagram 任务的失败与本期无关时只报告」，compare 的候选验收依据因此成立。 | 已解决无需修改（证据补齐） | 计划 §9.2 不改：它只主张「`compare` 的实现与验收已在候选 `968b6539` 上通过」与「本期不触引擎源码」，两条均被 UI 侧成功与 `git diff` 支持。原计划 §10 历史行与其完成记录只读，本轮未动。 | 剩余：无阻塞项。Diagram darwin-arm64 的取消属 CI/P4 侧观察，不在本计划范围，未新开缺陷。附带一处措辞更正：候选以来 `crates/cli/src/media/mod.rs` 有一处改动（`e86f1c79`，代理注入重构，非 compare 路径），故「CLI 零改动」仅对 compare 相关路径成立，不影响结论。 |

### 决策与未完项

- 无新的产品/设计决策待用户拍板：F-01 是事实一致性修补，F-02 是命令形态纠正，均落在报告已确认的范围内。
- 唯一自主扩写：V-4 增列「先跑一次开发构建」这一前置，理由与证据见上表 F-02 行；若要求严格回退到报告原文的最小范围，可删该分句，但安装仍会在预检阶段因清单不符失败。
- 未采纳项：无。用户排除项：无。

### 修订后结论与验证

- 计数：已修订并复核 2（F-01、F-02）；已解决无需修改 1（待核实 1）；不采纳 0；待核实/待决策 0；待修订/阻塞 0。本报告范围内须改项全部处置完毕，无剩余阻塞。
- 就绪判断：**Ready**。报告建议「Ready（修订后）」的两处文本前置已完成；改后范围、验收、排除与风险表自洽，未引入新的范围、安全、数据或外部契约未决项。这是本次修订复核，不是新一轮独立评审。
- 实际运行的文档检查：
  - `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md` → `ERROR 0 · WARN 0`。改稿过程中曾因写入原计划里程碑编号报 ERROR 4，已按本计划的 ID 约定改写消除。
  - `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md .` → 8 个路径 token，唯一 MISSING 为 `docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.records/`，计划 §1.2 已标「★ 新增」，与评审前一致。改稿一度把开发 release 目录写成仓内相对路径而被判 MISSING，已改回 `<仓库>/` 前缀。
  - `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md .` → `ERROR 0 · WARN 1`（1 个里程碑未完成、工作树有未提交改动而计划文件未被 git 跟踪）。该 WARN 源于计划尚未提交，与本次改稿无关，计划本身仍为 0/1 未开始。
  - 口径一致性：全文「定义行」七处逐条读过——`:20`/`:21` 为本期职责与例外、`:148`/`:203` 是原计划 `@progress_*` 协议文字（约束「进度回写」，复原不是进度回写）、`:214`/`:231`/`:235`/`:242`/`:312` 均指「收口里程碑定义行」同一对象，无第二套口径。
- 未运行且本轮不属必要验证：V-1–V-4（实施期检查，含 render `--check`、compare 端到端与安装）、`test:phoenix`、四平台 CI。本轮为只读文档修订，未产生需要行为验收的运行时改动。
- 范围外观察（只报告，未修改）：原计划 §10 的 M5 行现仍为六列进度行，即 R-2 的复原对象，属实施期动作；其历史完成记录与 SHA 保持原样。

下一步：

```text
/goal 按 docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4-gaps.md 开发
```
