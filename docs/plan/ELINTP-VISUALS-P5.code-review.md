# ELINTP-VISUALS P5 代码评审（R1）

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [ELINTP-VISUALS-P5 · XGENT-ai Skills · 变更视图与逐词标注](ELINTP-VISUALS-P5.md) | `84f91b5e1e24a9a13b9ac5e370142fbb8e6bc0ac..31bb3ff4043fff77338a0fc5a01746aae1c17252` 净差异（登记提交 `b7fab31a` M1、`55135ea3` M2、`9217a694` M3、`6330c17b`/`679e2d08` 修复、`31bb3ff4` M4；`d0f06aea` 为 M5 验证、docs-only）＋当前工作树（无产品代码改动，见覆盖与验证）；HEAD `e33e18d6` | 无 |

## 确认问题

### F-01 · [P2] tree 悬空检查漏掉「子节点标为 changed、父节点标为新增/删除」的输入，视图留下断头线

- 定位：tools/phoenix-diagram/src/changes.rs:276（`else if child_mark.is_none()` 分支；target `31bb3ff4`）
- 问题：计划 §5.2 的悬空规则要求「父子连线的某端在它出现的视图里不存在」时报 `change/dangling-relation`，并举例「父节点被标为删除，而子节点既没有标为删除也没有换父」。实现只对子节点**无标记**时检查父节点的新增/删除；子节点标为 `changed` 时三个分支都不触发，schema 合法的输入（子 `changed` ＋ 父 `removed`/`added`，无 `parent_before`）通过校验并交付。由于连线无标记、内容检查两侧一致也不会报，变更后（或前）视图会显示一条指向被隐藏节点的连线，违反 AC-2 与 §5.2 端点规则；对照实现读法，这正是「按上面的有效标记，连线某端不存在」应拒绝的情形。
- 证据：用 M5 最终开发构建 `local/phoenix-diagram/elintp-p5-m5`（sourceInputSha256 `e5e8e5a3…`，与 M5 记录一致）实测：
  - `phoenix-diagram validate tree /tmp/f1-tree.json --json`（节点 `r→p→c`，`p` `change:"removed"`，`c` `change:"changed"`）→ `status:"pass"`、0 条 diagnostics、exit 0；
  - `deliver` 同输入 → exit 0；SVG 中 `r→p` 边带 `data-change="removed"`（变更后隐藏），`p→c` 边无 `data-change`（变更后仍显示）——变更后视图留下指向被隐藏节点的断头线；
  - 对照组：同输入但 `c` 不带标记 → 正确报 `change/dangling-relation`（`/nodes/2/parent`）。
- 修复与验证：把父节点新增/删除的悬空判定条件从 `child_mark.is_none()` 放宽为「子节点未标为新增或删除」，使 `changed` 与无标记同样触发（消息分别说明原因）；tests/change.rs 增加该场景的拒绝断言；复跑上述探针确认 `changed` 子节点被拒、正常的 `changed` 输入（父节点两视图都在）不受影响。
- 置信度：高——实现分支与计划规则的偏差可直接读出，并已实测复现与对照。

### F-02 · [P2] 说明带计数行换行后 `data-change-count` 错配，es 等语言窄画布的合法输入无法交付

- 定位：tools/phoenix-diagram/src/render/changes.rs:269（每行调用 `count_runs`）与 280–303（按行内下标从 `marks[0]` 重新配对全量计数；target `31bb3ff4`）
- 问题：计数行超过栏宽时 `pack_items` 把三项拆到多行；`count_runs` 对每一行都从计数列表头部（added）开始按下标配对，第二行起的计数 run 被标成错误的 `data-change-count`。artifact 检查按标记求和核对 counts，错配后核对必然失败，合法输入被 `artifact/change-mark` 拒绝、无法交付。触发条件真实存在：说明带栏宽为画布宽减 48，tree 最小画布实测 320（栏宽 272），es 计数行「+ 1 añadidos · − 1 eliminados · ~ 1 cambiados」约 46 字符 × 11.5 × 0.6 ≈ 317px，必然换行；en ≈248px、zh ≈159px 暂不触发。
- 证据：同一构建实测：`/tmp/f2-tree.json`（tree，`meta.locale:"es"`，4 节点、三类计数各 1）→ `deliver` exit 1，两条 `artifact/change-mark`：「The description band reports 2 for added instead of 1」「The description band reports 0 for changed instead of 1」（evidence expected 1 / actual 2、1 / 0）。
- 修复与验证：让计数 run 与 marks 的配对跨行延续（传入全局起始下标，或构建行时按所属计数项分组）；tests/change.rs 增加「计数行换行」回归（窄画布或长 locale 触发拆行）；复跑 es 探针确认三类计数核对通过。
- 置信度：高——触发条件、错配方向与核对失败均已实测复现，与实现逻辑一一对应。

## 待核实/待决策项

无。本轮结论依赖的证据（源码 diff、两个探针的实测、M4/M5 记录的真实入口证据）均已在评审边界内取得；没有需要用户或后续里程碑补充的事项。

## 覆盖与验证

意图一句话：单一输入的变更标记在四类结构图（workflow、lifecycle、tree、erd）的校验、场景附着、渲染与静态收口，加上新增 `annotation` 图型与阅读器三视图，全部按计划 §5、§6 落地。

- 仓库根与日期：`/Users/rockie/Documents/gh-xgent/skills`，2026-11-17。
- 原输入与解析：开发计划模式。登记提交 `b7fab31a12bc49d8a8fd6b5bb416a7f33ad65d82`（父提交即计划基线 `84f91b5e…`）、`55135ea31044377f0e74ba4d5f7ad98b641a32b5`、`9217a694545177fa5b638bfb40507f65eeb90f95`、`6330c17be1f1b899c00d2b3da340d14af9a2cc32`、`679e2d08efd569f479820c22096b230b316e7e0a`、`31bb3ff4043fff77338a0fc5a01746aae1c17252` 均在当前分支（`merge-base --is-ancestor` 逐个确认）；登记提交连续，中间只有 6 个进度回写提交（`d9f0d748`、`f269b5f8`、`81b5a8ed`、`3134af40`、`d0f06aea`、`e33e18d6`，`--name-status` 逐一核实只改 `docs/plan/ELINTP-VISUALS-P5*`），故审查 `<最早登记提交>^..31bb3ff4` 净差异：97 文件，+6024/−114（`--name-status -M` 盘点）。
- 工作树指纹：HEAD `e33e18d60d7946e545233b09336b1c583d260f89`；未提交改动仅 `.xgent-ai/sdlc/status/**`（4 改 6 增）与 `docs/plan/ELINTP-VISUALS-P4-gaps*` 评审文档/截图目录，逐项核对不含 `tools/`、`skills/`、`test/` 产品文件，按约定不作 findings 对象、不影响本轮输入快照。
- 文件限制：`dist/phoenix-diagram/bundle/**`、`tools/phoenix-diagram/assets/template.html`、`assets/catalogs.json` 为生成物，未逐行评审——抽查 3 个新文件与维护源 `diff` 一致、`assets/template.html` 与目标树一致，一致性另由 build.test.js 断言与 M5 记录 V-6 第 5 步（`--sync-bundle` 后无差异、bundleSha256 `ffc25760…`）承担。
- 已审行为链：schema 四类＋common＋annotation（§5.1）；`changes::resolve`（端点规则、tree 两视图约束、换父、erd 字段继承、计数，§5.2）；`changes::attach`（关系映射、erd 干线、lifecycle 合流点、说明带，§5.3）；`render/changes.rs`＋`render.rs`/`render/tree.rs`/`render/erd.rs` 接入（§5.4）；`static_style.rs` 三处（§5.5）；annotation 的 schema/semantics/layout/render/check（§5.6–5.9）；十三个登记点（§5.10）；阅读器 `change-view.js`/`focus.js`/模板/CSS/装配与五语言键（§6.1）；技能与文档文字（§6.2/6.3）；新测试与既有测试改动。
- 未覆盖部分：阅读器 JS 的浏览器内行为（视图切换、键盘、导出为当前视图）本轮未另开浏览器实测，依据 M4 V-4 与 M5 V-7 记录（同一目标构建的真实页面走查与截图）；`tests/evidence.rs`、`tests/render_check.rs` 与 11 个其他图型 `layout/*.rs` 的改动经 diff 行统计核实仅为图型登记/`changes: None` 补齐（11 处全为 `+ changes: None,`），未逐行读。
- 实际命令与结果：`git rev-parse/merge-base/log/diff`（范围取证）；`validate tree /tmp/f1-tree.json` → pass（F-01 证据）；`deliver tree /tmp/f2-tree.json` → exit 1 两条 artifact/change-mark（F-02 证据）；`deliver tree /tmp/f1-tree.json` → pass，边标记分布证实断头线。两个探针均为隔离 `/tmp` 临时文件、未写入仓库；所测二进制 `elintp-p5-m5` 与目标码同源（M5 记录 sourceInputSha256 `e5e8e5a3…`）。
- 证据类型区分：源码级静态读码（全部 diff）；引擎级局部实测（validate/deliver，M5 最终构建）；真实入口证据引用 M4/M5 记录（离线安装、真实页面、截图、零网络 elintp 页，见计划符合性）。
- reviewer 分工：单 agent（本评审），无并行 reviewer。

## 计划符合性

- 计划：[ELINTP-VISUALS-P5.md](ELINTP-VISUALS-P5.md)（工作树版本，SHA-256 `d1ff8642…`），里程碑 M1–M5 全部完成，记录齐备（[M1](ELINTP-VISUALS-P5.records/M1.md)、[M2–M3](ELINTP-VISUALS-P5.records/M2M3.md)、[M3](ELINTP-VISUALS-P5.records/M3.md)、[M4](ELINTP-VISUALS-P5.records/M4.md)、[M5](ELINTP-VISUALS-P5.records/M5.md)）。
- 映射：§5.1–§5.10、§6.1–§6.3 与实现逐节核对一致；缺口即 F-01（违反 §5.2 悬空规则与 AC-2 的校验完备性）、F-02（§5.4 说明带计数契约在换行分支失效）。二者均为本期引入的缺陷，非计划遗漏或范围扩张。
- 计划 §1.2 文件清单之外出现的改动（`tests/evidence.rs`、`tests/render_check.rs`、11 个 `layout/*.rs`）均为 `DiagramType` 穷举与 `CompiledDiagram` 新字段的编译驱动补齐，属 §5.10 登记点范畴；计划预期修改的 `test/phoenix-diagram/reader-assets.test.js` 实际无需改动——其断言按 fragments 清单推导，无硬编码片段数（已核对）。
- 需求/验收证据：M5 §9.2 表将 WORK-F5、WORK-F9、WORK-N3 各 AC 映射到 V-2/V-3/V-4/V-6/V-7 的实际检查；V-7 为当前版本（`d0f06aea` 起 代码与 `31bb3ff4` 相同）的真实入口证据：离线安装后仅凭 skill 完成 tree 变更视图与 annotation 交付、截图审核 `deliveryStatus: complete`、变更前 PNG 与双策略静态图、elintp 零网络页断网/主题/360 检查。本轮两缺陷是该验证未覆盖的输入组合，不影响上述记录的有效性。

## 总体结论与修复顺序

**需要修改**（patch is incorrect）。

两处本期引入的 P2：F-01 让一类 schema 合法的 tree 变更输入带着断头线视图通过校验（校验完备性缺口），F-02 让 es 等长文案语言在窄画布下的合法输入因计数错配被交付检查拒绝。其余覆盖充分：十三个登记点、静态收口、阅读器接线与文档文字与计划一致，十二类无标记输入字节一致由 M5 V-6 承担，两能力的真实入口证据由 M5 V-7 承担。评审判断不构成 merge/deploy 授权。

修复顺序：先 F-01（校验契约根因，`changes.rs` 一处分支 ＋ 拒绝断言），再 F-02（`render/changes.rs` 计数配对跨行延续 ＋ 换行回归），最后复跑 tests/change.rs、tests/annotation.rs 与两个探针场景；完成后按计划模式复审。

## 下一步

```bash
$apply-code-review docs/plan/ELINTP-VISUALS-P5.code-review.md
# 处置完成后复审：
$review-code docs/plan/ELINTP-VISUALS-P5.md
```
