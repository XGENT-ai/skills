# PHOENIX-UI-EVIDENCE-CRAFT-P2 · R3 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3 · 修复效果复审 | [PHOENIX-UI-EVIDENCE-CRAFT-P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.md) | `aa1d065f9d11983eb16c81d5e6ee19c6f90f90df..f273d6622342371545a7e0a4b70ef4537951c4f8`；当前 HEAD `2518626b5614ea5a8b1784763ae5712e5b2853ee` 仅为目标后的计划回写，代码工作树与 `f273d66` 一致 | [R2](PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.r2.md) |

- 复审类型：修复效果复审；覆盖 R2 全部 F-01–F-03、V-6/V-7/V-9 待核实项及本次修复的直接回归。
- 上轮版本：R2 所审代码 `aa1d065f9d11983eb16c81d5e6ee19c6f90f90df`；R2 报告 SHA-256 `c0bd35423f0ca7d33adfedc196c252570fcda0e9f163641ad32be31062dd11a5`。
- 修后版本：`f273d6622342371545a7e0a4b70ef4537951c4f8`；计划 SHA-256 `8e71b8681fa2a31ad91b735f9eafe0b59a8a41cfb1110899763390d94b5c7d38`。
- 已有处置记录：[M1 · 评审修复](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md#评审修复)、[M2 · 验收](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M2.md#验收)、[M4 · R1 评审修复后的适用性](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md#r1-评审修复后的适用性)。处置自述不代替独立核对。

## 1. 确认问题

### F-02 · [P2] 让分栏栅格覆盖缩放坐标与 RTL 阅读顺序

- 定位：[tools/phoenix-ui/crates/browser/src/typography.rs:186–188](../../tools/phoenix-ui/crates/browser/src/typography.rs#L186-L188)，target `f273d6622342371545a7e0a4b70ef4537951c4f8`。
- 问题：普通 LTR 的每栏一行反例已经修复，但同一旧问题仍未完全解决。给同一两栏段落加 `transform: scale(.5)` 后，字符矩形处于缩放后的坐标系，`columnGrid` 却仍使用未缩放的 CSS 宽度与栏距；在 `dir="rtl"` 下，阅读顺序向较左栏推进，而当前只在 `at(c) > at(prev)` 时换栏。两种页面都把“末”实际单独放在最后一栏，`detect` 却都返回零发现，继续漏报末行孤字。
- 证据：计划要求行事实按实际渲染行逐栏给出（[计划 §5.5:461](PHOENIX-UI-EVIDENCE-CRAFT-P2.md#L461)），M1 也把缩放与 RTL 列为未处理边界（[M1:37](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md#L37)），没有用户豁免记录。当前二进制 SHA-256 `7db07266e8f0d21be8e553708542313361ac633d5adf3f416df10b167578b059`，Chrome 157 下对 `/tmp/phoenix-p2-r3-review/column-transform.html` 与 `column-rtl.html` 分别运行 `phoenix-ui detect --json --viewport 1280x800 file://<page>`，实际均为 `[]`；`capture --viewport desktop` 也均为 `findings=0`。已打开 `capture-column-transform/desktop.png`、`capture-column-rtl/desktop.png` 核对，前者右栏、后者左栏均只有一个“末”，两份报告均 `valid=true`。
- 修复与验证：让栅格的 `left`/`pitch` 与字符矩形处于同一坐标系，并按容器的书写方向识别栏位推进；不要用仅适用于未缩放 LTR 的索引比较替代实际换栏。至少把上述两个真实页面加入浏览器回归，同时保留本轮已通过的普通每栏一行、容器内分栏与跨栏数字单位场景。
- 置信度：高。当前真实 `detect`、`capture`、截图和代码坐标/方向条件相互印证。

### F-04 · [P2] 零栏距时不要把同行溢出文字判为换栏

- 定位：[tools/phoenix-ui/crates/browser/src/typography.rs:186–188](../../tools/phoenix-ui/crates/browser/src/typography.rs#L186-L188)，target `f273d6622342371545a7e0a4b70ef4537951c4f8`。
- 问题：在 `columns: 2; column-gap: 0; white-space: nowrap` 的段落中，不可断文字越过按栏宽计算出的栅格位置时，`at(c) > at(prev)` 会为真，而间距条件退化为 `c.left - prev.right > -1`；相邻字形通常满足该条件，于是一条实际连续行被拆成多“栏”。本次 13 字页面视觉上只有一行且以“末”结尾，当前却误报 `xgent/cjk-orphan: last line holds only "末"`。基线 `aa1d065` 没有这条栅格换栏分支，同行字符不会被它拆开，因此这是本轮修复新增的错误，不是原 F-02 的既有漏报。
- 证据：运行 `phoenix-ui detect --json --viewport 1280x800 file:///tmp/phoenix-p2-r3-review/column-zero-gap.html`，实际得到 1 条 `cjk-orphan`；`capture` 同样报告 1 条，但已打开 `capture-column-zero-gap/desktop.png` 确认全部 13 字在同一行。计划 §5.5 要求“间距条件让溢出本栏的不可断文字留在原行”（[计划:461](PHOENIX-UI-EVIDENCE-CRAFT-P2.md#L461)）；M1:37 已记录零栏距风险但未实测、未修复，本轮结果证实该风险可触发。
- 修复与验证：零栏距时不能把栏位索引前进单独当成换栏证据；使用能区分真实分栏与同行溢出的浏览器事实，无法区分时保守不判。加入上述不可断单行回归，断言没有 `cjk-orphan`，并复测正常多行两栏及每栏一行，避免简单放宽条件后重新引入 F-02。
- 置信度：高。真实入口误报、实际截图与新旧分支差异均已核对。

### 1.1 上轮问题与待核实项复审

| 来源报告 / F-ID或稳定章节 | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R2 / F-01 | 隐藏与透明文字进入事实；R2 已解决 | 本轮未改该分支 | 已解决 | `textNodes` 相关代码未变；`cjk` 16 项通过；当前真实 `detect` 对 `cjk/clean.html` 仍为 `[]` | 无 |
| R2 / F-02 | 每栏一行时两栏被拼成一行；R2 部分解决 | `f273d66` 改用分栏容器栅格 | 部分解决 | 原 LTR 反例、容器内版本均报孤字“末”，跨栏 `38`/`秒`也报拆单元；缩放与 RTL 仍漏报，见上方 F-02 | 修坐标系与方向后复跑所列真实页面 |
| R2 / F-03 | 保留节点参与字体发现及合并；R2 已解决 | 本轮未改过滤与让位分支 | 已解决 | `cjk`、`rule_pack` 测试通过；普通段加四个保留节点只报普通段且没有 `+N`，仅保留节点页面为零发现 | 无 |
| R2 §2 / V-6 | 完整开发包安装链未覆盖修复版 | 本轮未重跑本机 V-6 全链 | 待核实 | 当前 CI 在三个平台完成 native/WASM 构建、Node 冒烟及完整 runtime 测试；darwin-x64 仍在运行。这不等于计划 V-6 的本机 development-release、pack/install、render 与一盒全链 | 修复后在当前候选复跑适用的 V-6 阶段 |
| R2 §2 / V-7 | 三类宿主会话未覆盖修复版 | 本轮未启动 Claude/Codex/Pi | 待核实 | 本轮有当前版本的直接 `detect`/`capture` 与真实截图，但没有宿主 skill 会话证据 | 修复后按 M4 复跑受影响宿主场景 |
| R2 §2 / V-9 | 修复提交未跑四平台 CI | 当前 HEAD 已推送，run `42991386581` | 待核实 | 2026-11-11T17:32+08:00：darwin-arm64、linux-arm64、linux-x64 UI 任务成功；darwin-x64 仍 `in_progress` | 等同一 run 到终态并核对 darwin-x64 |

## 2. 待核实/待决策项

- **V-6、V-7**：本轮没有把当前 CI 或直接 CLI 证据冒充为完整打包安装链和宿主会话验收。由于 F-02、F-04 已足以判定需要修改，本轮不额外运行这些昂贵验收；修复后再按 M4 的受影响范围补证。
- **V-9 / darwin-x64**：[Phoenix run 42991386581](https://github.com/XGENT-ai/skills/actions/runs/42991386581) 在本报告快照时仍未终态。三个 UI 平台已成功，但不能据此写“四平台通过”。可用 `gh run view 42991386581 --json status,conclusion,jobs` 复核。
- 上述证据缺口不改变本轮已复现的两个正确性结论；本轮没有需求歧义或用户豁免。

## 3. 覆盖与验证

### 3.1 范围与快照

- 一句话意图：为五条中文页面规则提供真实排版行与字体事实，并让 `detect`/`capture` 以中文口径报告发现；本轮只复审 R2 修复及其直接回归。
- 仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；评审日期 2026-11-11；原输入仅为 `docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md`，按既有最高轮次 R2 进入复审。
- 已解析并确认计划登记提交均为当前 HEAD 祖先：`70fa92e3fe60a033f2b276becdead827b7a82744`、`e69c6007e5449e9fbacdfce267a377cfe4ee41f4`、`76814c1e6a94513918e173c8e1f131e1286f40a8`、`532f8d8f56ff87b739785fc5d518b5ad031ab3db`、`3d764e1b6699c37c0fcd5148b777227ef0c93c5d`、`f6cc57894db4fbca3f2633b19ca148907bef5a57`、`aa1d065f9d11983eb16c81d5e6ee19c6f90f90df`、`f273d6622342371545a7e0a4b70ef4537951c4f8`。R1/R2 已覆盖此前的完整实现；本轮新增问题检查只针对 `aa1d065..f273d66`，旧问题则在当前代码上独立复核。
- 本轮修复代码文件仅有 `typography.rs`、`xgent_rules.rs`、`cjk/breaks.html`；计划与 M1/M2/M4 变更是合同和处置材料，不作为代码 finding。`2518626b5614ea5a8b1784763ae5712e5b2853ee` 只回写计划，未混入代码差异。
- 当前 `tools/phoenix-ui`、`skills/phoenix-ui`、`docs/phoenix-ui.md` 相对 `f273d66` 无差异。tracked 工作树 diff SHA-256 `7d288663a238ec6dce176d478ddcc90272ab8bac21fc1f10787f5e31f57f61b4`，内容仅为既有 SDLC/经验资料；index 为空（空 diff SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`）。初始 9 个 untracked 均为既有 P1/P2 报告或 SDLC 资料，聚合 SHA-256 `f79dff49d562656503969914be37324ad5861b3a9db274836ff9a2c669268514`，没有未提交代码。本 R3 报告在指纹之后新建，不作为本轮输入。
- 关键目标文件指纹：`typography.rs` `5cf63ffc729fcb1b9a2f0a37bc58424c12c539e8e4d02020a522ff83dc7b60d4`，`xgent_rules.rs` `03f2a2ebad5e3eac310c1600ee0b8b4477343f57685eaa0dbc2504eb5f09274d`，`breaks.html` `bef8988b66edd1df19d2d74c189315e67d4decbb6fdd1f3acc3cc763cce1ece4`。结束核对时上述文件与计划均未变化。

### 3.2 实际执行的检查

在 `tools/phoenix-ui` 下、目标代码 `f273d66` 上运行：

```sh
mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline
mise exec --no-deps -- cargo test -p phoenix-ui-rules --test cjk --locked --offline
mise exec --no-deps -- cargo test -p impeccable-browser --lib typography --locked --offline
mise exec --no-deps -- cargo test -p impeccable-core --test rule_pack --locked --offline
mise exec --no-deps -- cargo test -p phoenix-ui --test xgent_rules url_scan_matches_the_fixture_expectations --locked --offline -- --test-threads=1
```

结果：构建成功；`cjk` 16/16、`typography` 7/7、`rule_pack` 4/4 通过；浏览器夹具表 1 项通过、11 项过滤，耗时 198.06s，覆盖全部 URL 的两个视口及新增每栏一行断言。本轮没有在本地重跑无关全量套件。

真实入口使用 `tools/phoenix-ui/target/debug/phoenix-ui`，SHA-256 与 capture 报告一致：

| 页面 | `detect` 实际结果 | 复审含义 |
| --- | --- | --- |
| 临时 `columns-one-row.html` | 1 条 `cjk-orphan`，末行“末” | R2 原反例已修 |
| 临时 `columns-container.html` | 1 条同样的孤字 | 段落位于分栏容器内也已修 |
| 临时 `columns-split.html` | 含 `38`/`秒` 的 `cjk-split-unit` | 跨栏数字单位已修 |
| 临时 `single-column.html` | 同文单栏报孤字“末” | 对照成立 |
| 仓内 `cjk/breaks.html` | 8 条预期发现，新增末例为“行” | 与 `xgent_rules` 一致 |
| 仓内 `cjk/clean.html` | `[]` | F-01 无回归 |
| 临时 `reserved.html` / `reserved-only.html` | 普通段仅 1 条且无 `+N` / `[]` | F-03 无回归 |
| 临时 `column-transform.html` / `column-rtl.html` | 均为 `[]` | F-02 仍存在，截图证明实际孤字 |
| 临时 `column-zero-gap.html` | 误报 1 条孤字“末” | F-04，截图证明实际只有一行 |

`capture file://<page> --viewport desktop --out /tmp/phoenix-p2-r3-review/capture-<name>` 已覆盖普通每栏一行、仓内 `breaks.html`、缩放、RTL、零栏距和对照页；报告均有效，主要反例截图已实际打开查看。对照页里的 `38`/`秒` 发现经截图确认确实发生视觉换行，没有误列为缺陷。所有临时页面、JSON、stderr 和 capture 证据保存在 `/tmp/phoenix-p2-r3-review/`。

外部验证只读取 GitHub Actions：run `42991386581` 的 HEAD 为 `2518626`，其代码与 `f273d66` 相同。快照时三个 UI 平台成功，darwin-x64 未完成；本地只停止 `gh run watch`，没有取消远端任务。

### 3.3 行为链与审查分工

已沿 `detect/capture → scan_loaded_page → typography::collect/MEASURE_JS → Dom 行事实 → cjk orphan/split-unit/font-fallback` 追踪；重点反查 `columnGrid`、`linesOf` 的坐标、栏距、方向和相邻字形条件，并用页面外证据验证实际可见结果。保留节点路径同时复核了 `cjk` 与规则包让位测试。M3 skill 文案、发布内容和旧 PRD 全量范围不在本次修复增量内，本轮未重新评审。评审与裁定均由当前主 agent 独立完成，无并行 reviewer。

## 4. 计划符合性

| 需求/章节 → 本次任务 | 实现与验证状态 |
| --- | --- |
| §5.5、V-2③ → 行事实必须来自实际渲染行，不可断同行文字不能误拆 | 存在缺陷：F-04 已由真实页面证实 |
| §5.5、V-2④ → 多栏文字须分开；R2 修复每栏一行 | 部分完成：普通 LTR 已修，缩放与 RTL 仍有 F-02 漏报 |
| V-2⑦ → 保留节点不产出、不合并字体发现 | 已核实，F-03 保持解决 |
| V-4 → 夹具发现与页面可见行为一致，两个视口等价 | 本次相关夹具表通过；新发现的边界页面尚未成为仓内回归 |
| V-6/V-7/V-9 → 修复候选的交付、宿主与平台证据 | 尚未完整补齐，见 §1.1、§2；不把旧候选证据当作当前完整证据 |

计划顶部「需求来源」为 [PRD v1.2 · P2](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)。本轮不要求未来发布、部署或终验事项提前在本次代码修复中完成。

## 5. 总体结论与修复顺序

验收判定：需要修改

**修复结论：未完全解决；`patch is incorrect`（限 `aa1d065..f273d66`）。** F-01、F-03 保持已解决；F-02 的普通每栏一行场景已修，但缩放与 RTL 仍部分存在。本轮新增 1 项 P2（F-04），没有与其他重复或未经证实的候选混入。

先修 F-04 的零栏距误报，再补齐 F-02 的坐标系与方向处理；两处都在同一换栏判定中，修复时应共同回归，避免消除误报后重新合并真实分栏。关闭条件是：本报告的普通、缩放、RTL、零栏距页面经真实 `detect` 得到预期，`capture` 与截图一致，相关浏览器夹具和探针测试通过；随后按 M4 补齐受影响的 V-6/V-7，并确认 V-9 四平台终态。该判定不是合并、发布或 PRD 验收授权。

```text
$apply-code-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.r3.md
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md
```
