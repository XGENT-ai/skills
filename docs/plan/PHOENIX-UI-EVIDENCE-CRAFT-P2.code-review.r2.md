# PHOENIX-UI-EVIDENCE-CRAFT-P2 · R2 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2 · 修复效果复审 | [PHOENIX-UI-EVIDENCE-CRAFT-P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.md) | `f6cc57894db4fbca3f2633b19ca148907bef5a57..aa1d065f9d11983eb16c81d5e6ee19c6f90f90df`；当前代码工作树与 `aa1d065` 一致，HEAD 为其后的进度提交 `8f35a362a16aa2ff1ea135da941f36b549d0f0da` | [R1](PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.md) |

上轮报告 SHA-256：`ceb5bf9d530471dedf382065809e7fb2e92a1f8ffc00e01857bead6582ff3228`；上轮范围 `45e3844..f6cc578`。本轮覆盖 R1 的 F-01–F-03、其未完成的 V-6/V-7/V-9 证据项，以及修复差异的直接回归。已有处置记录：[M1](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md)、[M2](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M2.md)、[M4](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md)。处置记录只作为线索，以下结论均由当前代码和本轮实际验证独立核对。

## 1. 确认问题

### F-02 · [P2] 每栏只有一行时仍合并两栏并漏报孤字

- 定位：[tools/phoenix-ui/crates/browser/src/typography.rs:152-153](../../tools/phoenix-ui/crates/browser/src/typography.rs#L152-L153)（target `aa1d065`，与当前工作树一致）；R1 旧问题，本轮结果为**部分解决**。
- 问题：换栏仍要求 `!sameRow(last, c)`；当每栏只有一行时，下一栏首字与上一栏首行处于同一水平带，因此不会开启新栏，两栏会被拼成一行。真实页面使用 `p { width:500px; height:32px; columns:2; column-gap:100px; column-fill:auto; line-height:32px }`，正文为「页面扫描逐行核对中文排版末」；Chrome 157 截图清楚显示右栏仅有「末」，但当前 `detect --json` 返回 `[]`，`capture` 为 `findings=0`，仍未满足原问题“分开横向断开的多栏文字”的关闭条件。
- 证据：[同文件:146-158](../../tools/phoenix-ui/crates/browser/src/typography.rs#L146-L158) 的栏判定；本轮隔离复现 `/tmp/phoenix-p2-r2-review/columns-one-row.html`、真实截图与 `capture-report.json`。新增多行两栏夹具 `breaks.html` 的 7 条预期发现均正确，说明常规两栏案例已修复，但不能覆盖这个同行高边界。
- 修复与验证：让换栏识别覆盖“下一栏首行与上一栏首行同高”的情况，同时保留普通行内换行与行内块的行为；增加每栏仅一行、右栏末行为 1 字的真实 `xgent_rules` 夹具，并复跑本节所列 `detect`/`capture` 主路径。
- 置信度：高；静态条件与真实浏览器可观察结果一致。

本轮没有确认其他新增问题。F-02 沿用 R1 编号，不重复计为新发现。

### 上轮问题复审

| 来源报告 / F-ID或稳定章节 | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01 | [P2] 隐藏文字仍进入行与字体事实 | `aa1d065`：过滤 `visibility` 与透明行内祖先 | **已解决** | [typography.rs:116-126](../../tools/phoenix-ui/crates/browser/src/typography.rs#L116-L126)；真实 `detect clean.html` 返回 `[]`，`breaks.html` 保持 7 条预期发现；`cjk` 16 项通过 | 无 |
| R1 / F-02 | [P2] 多栏文字被拼成同行 | `aa1d065`：按文档顺序与坐标识别换栏 | **部分解决** | 多行两栏真实夹具通过；每栏一行时右栏孤字仍漏检，见上方 F-02 | 修正同行高换栏并补真实夹具 |
| R1 / F-03 | [P2] 页面规则扫描保留节点并污染去重 | `aa1d065`：共享 `is_reserved_node` 并在 `font_fallback` 排除 | **已解决** | [driver.rs:1306-1324](../../tools/phoenix-ui/crates/core/src/browser/driver.rs#L1306-L1324)、[cjk.rs:256-260](../../tools/phoenix-ui/crates/rules/src/cjk.rs#L256-L260)；真实普通+保留节点页面只报普通段 1 条且无 `+N`，仅保留节点页面为 `[]`；新增回归测试通过 | 无 |

## 2. 待核实/待决策项

这些项目不缺证到无法裁定本轮结果；F-02 已有确定反例，因此验收不受它们影响。

| 上轮证据项 | 本轮结果 | 当前缺口与补证方式 |
| --- | --- | --- |
| R1 §3.3 / V-6 场景 E2E | 待核实 | 已有证据停留在 `f6cc578`；本轮未重跑完整场景 E2E。F-02 修复后按 M4 的 V-6 步骤复核当前版本，重点确认排版变化不污染项目详情列表与报告分组。 |
| R1 §3.3 / V-7 宿主 UI | 待核实 | 已有证据停留在 `532f8d8`；本轮真实 `detect`/`capture` 不能代替 CLI、React 与卡片入口。F-02 修复后按 M4 的 V-7 A1–A3 复核当前版本。 |
| R1 §3.3 / V-9 四平台 CI | 待核实 | 本轮仅在 darwin-arm64 / Chrome 157 验证；未推送、未获得四平台结果。修复后运行 `xgent_rules` 四平台矩阵。 |

无需求取舍需要用户在本轮先拍板；单行两栏属于原计划已承诺的多栏文字行为，不是新增需求。

## 3. 覆盖与验证

### 3.1 范围与快照

- 一句话意图：为 Phoenix UI 的中文排版探针补齐真实行/字体事实，并让五条 CJK 规则只判断页面自身的可见内容。
- 仓库根：`/Users/rockie/Documents/GitHub/xgent/skills`；日期：2026-11-11；当前分支 `main`。
- 原输入：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md`。计划登记提交均解析并确认位于当前分支；R2 的修复差异为 `f6cc578..aa1d065`，`aa1d065..8f35a36` 只有计划进度回写。
- 排除的区间内进度提交：`f899927`、`6bca491`、`f6cc578`、`7fb6e95`、`8f35a36`，原因均为只改本计划、记录或进度文档。R1 已覆盖其前实现与验收修复，本轮不重复全审。
- 修复差异包括 `typography.rs`、`driver.rs`、`cjk.rs`、相应测试/夹具、ENGINE 文档、计划记录及再生成 bundle。手写代码、测试与契约均已读；生成物没有逐字审 WASM 字节，使用输入/输出哈希与 `cargo xtask bundle --check` 验证一致性。
- 未提交 tracked diff SHA-256：`fa6f6f42183f60d4002af0c5f1f34242f5e5ae8de4005991eff44892f0146025`；index 为空（空 diff SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`）；报告写入前 untracked 清单内容指纹 `6ee7aa1e5245f4fbc8213d8b3beb873844701e6a469a59decef0693ab7302815`。这些内容仅为计划、既有报告与台账，没有代码改动；本报告写于取证之后，不作为本轮输入。
- 计划当前 SHA-256：`f757add658cb6052f8db9cb2b6d5b900a1d2f9d8323fa66beb44a2daf85646c1`。`typography.rs`、`cjk.rs`、`driver.rs` 当前内容分别与 `aa1d065` 逐字节一致；没有发现审后代码漂移。
- 由当前主 agent 独立审查与裁定，无并行 reviewer。未修改实现、测试、计划进度或既有 R1 报告。

### 3.2 实际验证

所有命令均在当前修复版工作树执行；构建/测试使用 `mise exec --no-deps --`，`--locked --offline`。

| 检查 | 实际结果 |
| --- | --- |
| `cargo build -p phoenix-ui --bin phoenix-ui` | 通过；真实入口使用当前构建，capture 报告内二进制 SHA-256 `e9d4739bdfea79028ba450c2f032b70da8bae12e4964e64a8c39053aa0833256` |
| `cargo test -p phoenix-ui-rules --test cjk` | 16 passed，含保留节点与不可渲染文字回归 |
| `cargo test -p impeccable-browser --lib typography` | 7 passed |
| `cargo test -p impeccable-core --test rule_pack` | 4 passed |
| `cargo xtask bundle --check` | 通过：`Browser bundle inputs and artifact hash are current` |
| `git diff --check f6cc578..aa1d065` | 通过 |
| 真实 `detect --json`：现有 `clean.html` | `[]`；隐藏行与隐藏失败字体未再误报 |
| 真实 `detect --json`：现有 `breaks.html` | 7 条发现，与夹具预期清单一致；含多行两栏孤字/拆数字单位，未出现相邻栏伪报 |
| 真实 `detect --json`：自建普通+三类保留节点页面 | 仅普通段 1 条字体发现，无 `+N`；仅保留节点页面为 `[]` |
| 真实 `detect` + `capture`：每栏一行反例 | 页面可见右栏孤字「末」，但两者均零发现；确认 F-02 部分解决 |
| 真实 `capture`：`breaks.html` 与单行两栏反例 | 已读取截图并开图核对文字实际分栏；报告 `valid=true`、`failures=[]`，发现数分别为 7 与 0 |

本轮以针对性真实入口替代 231 秒的完整 `xgent_rules` 重跑；未把局部测试当作完整宿主 UI 或跨平台验收。一个自建长文复现因固定高度溢出成三栏而弃用，未作为通过证据。隔离验证文件位于 `/tmp/phoenix-p2-r2-review`，不在目标仓库。

### 3.3 已审行为链与限制

已沿 `scan_loaded_page → MEASURE_JS.textNodes/linesOf → SnapshotDom 行事实 → cjk_detail/split_unit/orphan` 核对可见文本与分栏；沿 `platform_fonts/han_font_facts → font_fallback → is_reserved_node` 核对失败字体、保留节点与共享发现去重；并核对元素循环提取共享谓词前后的排除条件一致。未覆盖修复版完整场景 E2E、宿主 UI 与非当前平台，分别列在 §2。

## 4. 计划符合性

计划版本为上述 SHA-256；本轮只核对 R1 修复对应的 M1/M2 行为及 M4 验证，不把未来或已明确延期的证据项写成新缺陷。

| 计划/需求 | 当前实现与证据 | 状态 |
| --- | --- | --- |
| M1 §5.5：只读取真实渲染行与可见字体事实 | F-01 的隐藏文字/字体反例已通过 | 已核实 |
| M1 §5.5、M2 孤字/拆单位：按真实多栏阅读顺序判行 | 多行两栏已修；每栏一行仍合并并漏报孤字 | **存在缺陷（F-02）** |
| M2：失败字体只归页面自身内容，并按字体栈去重 | F-03 的单测与真实保留节点页面均通过 | 已核实 |
| M4 V-6/V-7/V-9 | 本轮未重跑完整证据；没有伪称通过 | 待核实，见 §2 |

## 5. 总体结论与修复顺序

**验收判定：需要修改**

**修复效果复审：未完全解决。`patch is incorrect`（F-02）。**

F-01 与 F-03 已解决，本轮没有确认其他新增问题；F-02 的多行两栏主案例已恢复，但同行高的单行两栏仍把两栏拼成一行，已在真实 Chrome 的 `detect` 与 `capture` 主路径复现漏报。先修正该换栏条件并加入每栏一行的真实夹具，再复跑 `cjk`、`typography`、相关 `detect`/`capture` 与 bundle 一致性检查；完整 V-6/V-7/V-9 证据仍按 §2 补齐。本结论不构成合并或发布授权。

```text
$apply-code-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.r2.md
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md
```
