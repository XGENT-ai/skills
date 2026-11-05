# PHOENIX-DIAGRAM-STATECHART · 代码变更评审报告

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [PHOENIX-DIAGRAM-STATECHART](PHOENIX-DIAGRAM-STATECHART.md) | 净差异 `b0d540f^1..9cdaacf`（登记提交 `b0d540f1f731fb44558f37bcbeff9a0afaff07f1`、`6bb21b6b582fdeb05f514c058d9363c54939fc`、`9cdaacf31c522d67bab7fa03a3c97882911abaec`，均为 main 祖先且中间只夹计划回写提交）；工作树干净，无未提交变更；排除 `f753731`、`c59436a`、`f254e704`（仅改计划/记录文件） | 无 |

评审基线：计划文件为 HEAD `f254e704` 版本（与 `9cdaacf` 相比仅差计划回写）。变更规模 376 文件，+42,639 / −157,805 行（绝大部分删除为 upstream/legacy-2.17 下线）。

## 1. 确认问题

### F-01 · [P2] 超出 i64 的整数字面量被静默丢弃：规则消失或零诊断失败

- 定位：`tools/phoenix-diagram/src/statechart/expr.rs:206-218`（主因 `literal()`）；静默消费点 `tools/phoenix-diagram/src/statechart/model.rs:127`、`model.rs:131`、`model.rs:165` 及表达式编译路径
- 问题：`statechart.schema.json` 的表达式字面量与 `values`/`pick`/`range` 只约束 JSON `integer`（任意精度），而 `literal()` 对无法用 i64 表示且 ≥9e15 的整数返回 `None`。三条消费路径都不报诊断：`values`/`pick` 用 `filter_map` 静默丢值（域缩小）；表达式编译里字面量返回 `None` 会向上传播，使整条 `holds` 编译为 `None`，规则**不进模型且无任何诊断**；`range` 端点 `as_i64()?` 静默丢变量，触发 `Model::parse` 的 `Err(空诊断)`。实测（本机、当前树、隔离临时文件）：`values: [0, 10000000000000000000]` 且规则 `holds` 引用同一大数时，`validate` 返回 `status: "pass"`、`diagnostics: []`，回执 `model.rules` 只剩 deadlock——作者声明要判定的规则没有任何结论，且域只剩 1 个状态（`states: 1`）；`values: [10000000000000000000]` 单独出现时 `status: "failed"` 且诊断列表为空，失败原因不可解释。这违背 R-1「得出每条规则的 holds / breaks / not-checked」的核心承诺：规则在 schema 合法输入下静默消失。
- 证据：复现命令（临时目录，已清理）：
  `mise exec --no-deps -- mbx run --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --bin phoenix-diagram -- validate statechart <bignum.json> --json`；两例实际输出见「覆盖与验证」第 6 条。schema 依据：`skills/phoenix-diagram/schemas/statechart.schema.json` 的 `$defs/expr` 字面量分支与 `values` 分支均未限制整数边界。
- 修复与验证：最小修复二选一——(a) `literal()` 对「是 JSON number 但表示不了」的值产出诊断（新码如 `statechart/integer-out-of-range`，含 pointer 与原值），`values`/`range`/`pick`/表达式各消费点不再静默跳过；(b) 在 schema 的整数字面量、`values`、`range`、`pick` 上加 `minimum`/`maximum`（i64 边界），让非法输入停在 schema 阶段。回归断言：三处各一例超界整数，断言出现诊断、规则不消失、`init` 大数路径行为不变（该路径已有 `value-not-in-domain`）。
- 置信度：高。两条失败路径均已用命令复现；根因单点（`literal()` 返回 `None` 的语义被当作「跳过」而非「报错」）。

## 2. 待核实/待决策项

1. **`sourceCommit` 指向回写提交**：`skills/phoenix-diagram/ENGINE.json` 与 `dist/phoenix-diagram/VERSION.json` 记 `sourceCommit: c59436a…`，而二进制实际包含其后 `9cdaacf` 的 viewer CSS 修复（输入身份由 `sourceInputSha256: de37abdf…` 承担）。这是仓库既有的「回写后构建、记录 HEAD」惯例（上一版同样指向 `f753731`），未按缺陷处理；若要求 sourceCommit 严格对应代码提交需另行决策。
2. **浏览器与看图验收未重放**：V-8.3（五份 finalize 三门+看图）、V-8.4（十类回归）、V-8.5（角色模拟六轮）的产物在 `local/phoenix-diagram/…`（gitignored），本轮按 M6 记录采信，未独立复核截图与 review-record。
3. **`bundleSha256`（`9a1179c4…`）未重算**：bundle 内容已用 `diff -rq skills/phoenix-diagram dist/phoenix-diagram/bundle` 逐文件确认一致（仅 dist 侧多 `manifest.json`），哈希本身未验证。

## 3. 覆盖与验证

一句话意图：对 statechart 新图型（模型检查→投影→布局→渲染→Reader/skill/文档/dist）与 legacy-2.17、upstream 两项解耦做证据化评审。仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`，日期 2026-11-04。

- **范围取证**：`git rev-parse --verify '<sha>^{commit}'` 解析三个登记提交并确认均为 HEAD 祖先；`git rev-list --reverse b0d540f^..HEAD` 确认区间内非登记提交只改计划文件；`git status --short` 为空（工作树、index、untracked 均无变化）。
- **已审行为链**（静态，全文或 diff 通读）：核心 `src/statechart/{mod,model,expr,explore,chart,regions,report}.rs`（约 3,100 行全文；标记/提升/合并/失真回退等算法对照计划 §5.4 逐条推导，含「脏组合全体可行才标记」不会产生空标记的证明）；`layout/statechart.rs`、`layout/statechart/place.rs`、`render/statechart.rs`、`check/statechart.rs`；接线 diff（`input.rs`、`input/semantics.rs`、`schema.rs`、`cli.rs`、`check.rs`、`delivery/{prepare,finalize,receipts}.rs`、`evidence.rs`、`i18n.rs`、`compare/canonical.rs`、`lib.rs`、`scene.rs`）；schema 全文（含 `$defs/expr` 与 model.rs 静默路径的可达性对照，除 F-01 外均被 schema 先行拦截）；`serde_json preserve_order` 已启用（`Cargo.toml:11`，`pick`/`set`/`init` 保持书写顺序，符合 §5.3.3）；五语目录键集一致（542 键 ×5）且代码引用的 11 个 `statechart.*` 键全部存在；viewer 18 文件 diff（含 `9cdaacf` 节点索引 CSS 修复、`readerFit "legacy"` 字符串按 §12 坑位保留）；scripts diff（assembler 简化、`verifySources`、maintenance 测试迁移、原生测试清单）；skill/docs（SKILL.md 图型表与 description、`references/statechart.md`、`schemas/README.md`、`docs/phoenix-diagram.md`、`bin/xgent-skills.js`、Node 测试枚举）。
- **移除完整性**（R-5/R-6/R-7）：`tools/phoenix-diagram/upstream/` 已不存在；`git grep` 全仓仅剩许可署名、Oracle 基线 `source` 字段、「来自 Archify 3.1 的迁移指引」与 `PhoenixDiagram` 命名空间声明中的 Archify 字样；渲染器输出为 `phoenix-diagram-*` id；11 份 schema `$id`/`title` 统一 Phoenix 基址；`pstack-statechart-MIT.txt` + NOTICE 署名就位。
- **实际命令与结果**（均在当前树执行）：
  1. `mise exec --no-deps -- mbx test --manifest-path tools/phoenix-diagram/Cargo.toml --locked --offline --test statechart --test statechart_model --test statechart_chart --test statechart_report` → 全部通过（8+6+chart+5 项，0 失败）。
  2. 同上 `--test input_contract --test cli` → 10 + 28 通过（覆盖原五类未标记报 `input/contract-required`、新类型枚举、guide、HELP）。
  3. `node scripts/assemble-phoenix-diagram-reader.mjs --check` → PASS（template.html/catalogs.json 与 viewer 源一致，含 `9cdaacf` 的 CSS 修复）。
  4. `validate statechart` 探针（空字符串变量值 + `{v}` 标签）→ 正确报 `statechart/empty-label`（`/view/state/0/label`）。
  5. `diff -rq skills/phoenix-diagram dist/phoenix-diagram/bundle` → 仅 `manifest.json` 为 dist 侧独有（dist 同步成立）。
  6. `validate statechart` 探针两大数样例 → 复现 F-01 的两种后果（pass+规则消失 / failed+零诊断）。
- **未覆盖**：upstream/legacy 删除的约 15.7 万行未逐行核对（以引用 grep + 全量测试替代）；oracle JSON 基线未重放（计划 §12 明确为固定基线）；V-8 全量聚合（`npm run test:diagram`）未重跑（M6 记录在最终代码上 exit 0，本轮以相关子集独立复验）；浏览器/看图证据见待核实第 2 条。

## 4. 计划符合性

计划：`docs/plan/PHOENIX-DIAGRAM-STATECHART.md`（HEAD `f254e704`），M1–M6 全部里程碑、V-1–V-8 验收均登记完成。

| 需求 | 实现证据 | 状态 |
| --- | --- | --- |
| R-1 statechart 模型检查（穷举、holds/breaks/not-checked、最短出错路径） | `explore.rs`（BFS、D3 上限、规则/死锁判定、路径稳定排序）、`expr.rs`、`tests/statechart_model.rs` 8 项通过；死锁/上限/确定性均有断言 | 已核实；边界缺陷见 F-01 |
| R-2 view 投影（合并/去重/自环/隐藏/区域条件/失真回退/提升/标记/上色） | `chart.rs`/`regions.rs` 对照计划 §5.4 逐条一致；`tests/statechart_chart.rs` 7 项通过（login-regions 条件、two-phase 回退、部分提升不合并等） | 已核实 |
| R-3 交付（finalize 三门、可选中父状态、图例、报告卡片、回执 `model`） | `check/statechart.rs` facts↔SVG 一致性、`report.rs` 卡片与 `model` 回执、`tests/statechart.rs`/`statechart_report.rs` 通过 | 已核实 |
| R-4 skill/文档/schema/示例/参考与 lifecycle 边界 | SKILL.md 图型表+description+边界段、`references/statechart.md`、`schemas/statechart.schema.json`、最小示例、Node 测试枚举、dist 同步 | 已核实 |
| R-5 删除 upstream 及技术耦合（保留 ICU/tz/许可/品牌来源检查） | `upstream/` 目录消失、assembler/check 脚改为自有来源、maintenance 测试迁移、`--check` 通过 | 已核实 |
| R-6 删除 legacy-2.17 合同/分支/示例，原五类未标记报 `input/contract-required` | `input.rs`/`cli.rs`/HELP/guide 更新；`input_contract` 测试 10 项通过；全仓 grep 仅剩「不再支持」说明 | 已核实 |
| R-7 Phoenix 标识 + 保留许可署名 | schema `$id`/`title`、viewer 命名空间、源码注释去 frozen 字样；NOTICE/MIT/OFL/pstack 署名保留 | 已核实 |
| C-1 生成/布局/校验全在 Rust | 新增代码全部 Rust；Reader 仅浏览器 JS | 已核实 |

M6 记录中「已知项」三条（回执 summary 相对路径、finalize stdout 600–730KB、窄屏靠高宽比门）为实施方已披露并声明本期不改的限制，不计为新问题。

## 5. 总体结论与修复顺序

**需要修改**（`patch is incorrect`）

在所审净差异范围内，statechart 功能、投影语义、布局/渲染/检查、skill/文档/dist 同步与两项解耦均与计划和仓库规则相符，测试与抽查大量通过；但 F-01（P2）在 schema 合法输入上使规则静默消失或零诊断失败，直接违背 R-1 的判定承诺，已实证。覆盖限制见第 2、3 节（浏览器验收与全量聚合按记录采信）。

修复顺序：先修 F-01（契约/数据正确性根因，单点修复+三处回归断言），随后按第 4 节无需其他动作；P3 及以下无问题。修完用 `$review-code docs/plan/PHOENIX-DIAGRAM-STATECHART.md` 复审本报告。

*本判定只覆盖所审提交与工作树，不是合并、发布或需求验收授权。*
