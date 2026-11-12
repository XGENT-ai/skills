# PHOENIX-UI-EVIDENCE-CRAFT-P2 · R4 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R4 · 修复效果复审 | [PHOENIX-UI-EVIDENCE-CRAFT-P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.md) | `f273d6622342371545a7e0a4b70ef4537951c4f8..080c3bec7f652dc15bfd242233e18e2d54621e29`；当前 HEAD `3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d` 仅为目标后的计划回写，代码工作树与 `080c3be` 一致 | [R3](PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.r3.md) |

- 复审类型：修复效果复审；覆盖 R3 全部 F-01–F-04、V-6/V-7/V-9 待核实项及本次修复的直接回归。
- 上轮版本：R3 所审代码 `f273d6622342371545a7e0a4b70ef4537951c4f8`；R3 报告 SHA-256 `da30e21f6d5fc2b1f647e07d1661e8d01f92b5e9de64588706f2630f39a4ee95`。
- 修后版本：`080c3bec7f652dc15bfd242233e18e2d54621e29`；计划 SHA-256 `89eb8d534f4c6bbbb7e7cd7f30487dbaf1b1b000a36f2d07705202e77343d793`。
- 已有处置记录：[M1 · 评审修复](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md#评审修复)、[M2 · 验收](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M2.md#验收)、[M4 · R3 修复后的适用性](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md#r3-修复后的适用性)。处置自述不代替独立核对。

## 1. 确认问题

未发现可确认的新增问题；没有为凑数保留证据不足的候选。

### 1.1 上轮问题与待核实项复审

| 来源报告 / F-ID或稳定章节 | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R3 / F-01（承接 R2） | 隐藏与透明文字进入事实；R3 已解决 | 本轮未改 `textNodes` 的绘制过滤 | 已解决 | 当前 [`typography.rs:99–108`](../../tools/phoenix-ui/crates/browser/src/typography.rs#L99-L108) 仍按 `visibility` 与行内 `opacity` 过滤；直接 `detect` 对 `cjk/clean.html` 在 1280×800、375×812 下均为 0 条 `xgent/*` | 无 |
| R3 / F-02（承接 R2） | 缩放与 RTL 分栏仍漏报末行孤字；R3 部分解决 | `080c3be` 让栅格坐标乘渲染/布局宽度比，并按 `direction` 推进 | 已解决 | [`typography.rs:140–142`](../../tools/phoenix-ui/crates/browser/src/typography.rs#L140-L142) 的缩放和 RTL 分支与原反例一致；当前真实 `detect` 对原 `column-transform.html`、`column-rtl.html` 各报 1 条末字“末”的 `cjk-orphan`；容器自身/祖先缩放、RTL、组合缩放 RTL 及非对称 padding/border 均通过；`breaks.html` 两视口各为预期 11 条 `xgent/*` | 无 |
| R3 / F-03（承接 R2） | 保留节点参与字体发现及合并；R3 已解决 | 本轮未改保留节点、字体过滤或让位逻辑 | 已解决 | 相关调用与 `cjk.rs` 无 diff，R3 的 reserved/reserved-only 证据仍适用于该未变行为；本轮直接 `detect` 对 `webfont.html` 仍得到 1 条拆单元、2 条各自归属的字体回退及通用族歧义备注，没有把未核实情况误报为发现 | 无 |
| R3 / F-04 | 零栏距同行溢出文字被误拆并误报孤字 | `080c3be` 在零栏距、同一行时要求 Range 的实际分片证据 | 已解决 | [`typography.rs:75–97`](../../tools/phoenix-ui/crates/browser/src/typography.rs#L75-L97)、[`typography.rs:135–140`](../../tools/phoenix-ui/crates/browser/src/typography.rs#L135-L140) 对同节点用实际分片、跨节点保守不拆；原 `column-zero-gap.html` 和仓内纯文本/span 两例均无 `xgent/*` 误报，`zero-gap-real.html` 仍报真实末字“末”；RTL 与缩放组合也通过 | 无 |
| R3 §2 / V-6 | 修复版缺开发构建、安装与真实入口证据 | M4 已记录 R3 开发构建、冒烟、离线安装、render 与受影响浏览器检查通过 | 已解决（本轮缺口） | 当前 debug 二进制 SHA-256 `34d8fda5d3be67c3dc7a4ccd7cbe8ad879f15057e0aeebfd6b01054e31b3896f`，与 M2 的 R3 构建及 capture 报告一致；本轮又在该二进制上复核直接入口。未重跑完整 `test:phoenix`，不据此宣称全量回归已在新版执行 | 无本轮新增动作；阶段全量证据由 V-9 补齐 |
| R3 §2 / V-7 | 修复版缺宿主会话证据 | 当前开发构建的安装后启动器及直接 CLI 已通过；Claude 请求仍被 HTTP 403 拒绝 | 待核实 | [M4 · R3 修复后的适用性](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md#r3-修复后的适用性)记录请求在到达 Anthropic API 前因 network source not allowed 失败，未执行 skill/工具；本轮未绕过限制或把直接 CLI 冒充宿主通过 | 恢复可用 Claude 环境后，用当前开发构建补跑 V-7 的受影响宿主取证场景 |
| R3 §2 / V-9 | 上轮快照时旧 run 未终态；修复候选未跑四平台 CI | 旧 run `42991386581` 已终态成功；`080c3be` 尚未推送 | 待核实（当前候选） | 本轮 `gh run view` 核实旧 run 的 `headSha=2518626b…`、四平台 UI 与 Node 18 全部 success；对 `080c3bec…` 和 `3d328f7e…` 精确过滤均为 `[]`，远端 main 仍为 `2518626b…`。旧成功不覆盖当前修复 | 经用户另行授权推送当前候选，再按完整 SHA 核对四平台与 Node 18 终态 |

旧问题 4/4 已解决；修复引入的新问题 0。V-6 的本轮证据缺口已关闭，V-7 与当前候选 V-9 尚未关闭。

## 2. 待核实/待决策项

- **V-7 / 当前修复版宿主取证**：缺 Claude Code 实际加载已安装 Phoenix 并完成受影响取证场景的当前版本证据。解除方式见 [M4 · R3 未完项与继续动作](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md#r3-未完项与继续动作)；需先恢复网络来源允许的宿主环境，不能用本轮直接 CLI、截图或旧会话代替。
- **V-9 / 当前修复版四平台候选**：`080c3bec7f652dc15bfd242233e18e2d54621e29` 及仅改计划的 HEAD 都没有 Phoenix CI 记录。需取得新的推送授权后推送，再运行计划中的精确匹配命令并核对匹配 run 的四个 UI Native 与 Node 18 任务：

  ```sh
  candidate_sha=080c3bec7f652dc15bfd242233e18e2d54621e29
  gh run list --workflow phoenix.yml --branch main -L 100 --json databaseId,headSha,status,conclusion --jq "map(select(.headSha == \"$candidate_sha\"))"
  ```

- 嵌套分栏只取最近容器、零栏距同高跨文本节点保守不拆，是本轮计划明示的保守边界；本轮没有把它们包装成新缺陷，也没有声称这些边界已得到更一般处理能力。
- 本轮没有需求歧义或用户豁免。

## 3. 覆盖与验证

### 3.1 范围与快照

- 一句话意图：为五条中文页面规则提供真实排版行与字体事实，并让 `detect`/`capture` 以中文口径报告发现；本轮只复审 R3 修复及其直接回归。
- 仓库根 `/Users/rockie/Documents/GitHub/xgent/skills`；评审日期 2026-11-11；原输入仅为 `docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md`，按既有最高轮次 R3 进入复审。
- 已解析并确认计划登记提交均为当前 HEAD 祖先：`70fa92e3fe60a033f2b276becdead827b7a82744`、`e69c6007e5449e9fbacdfce267a377cfe4ee41f4`、`76814c1e6a94513918e173c8e1f131e1286f40a8`、`532f8d8f56ff87b739785fc5d518b5ad031ab3db`、`3d764e1b6699c37c0fcd5148b777227ef0c93c5d`、`f6cc57894db4fbca3f2633b19ca148907bef5a57`、`aa1d065f9d11983eb16c81d5e6ee19c6f90f90df`、`f273d6622342371545a7e0a4b70ef4537951c4f8`、`080c3bec7f652dc15bfd242233e18e2d54621e29`。R1–R3 已覆盖此前实现；本轮新增问题检查只针对 `f273d66..080c3be`，旧问题在当前代码上独立复核。
- 本轮代码与测试改动仅有 `typography.rs`、`xgent_rules.rs`、`cjk/breaks.html`、`cjk/clean.html`；计划与 M1/M2/M4 是合同和处置材料。`3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d` 只回写计划，未混入代码差异。区间内其余 `f899927`、`6bca491`、`f6cc578`、`7fb6e95`、`8f35a36`、`2518626` 仅作计划/进度回写，不作为独立代码审查对象。
- 当前 `tools/phoenix-ui`、`skills/phoenix-ui`、`docs/phoenix-ui.md` 相对 `080c3be` 无差异。tracked 工作树 diff SHA-256 `07c2b0cfa1503757fd38aee509763a7946153fe887c4262e6236fa08af90ae16`，仅为既有 SDLC、经验与 M4 记录；index 为空（空 diff SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`）。既有 modified/untracked 输入的聚合 SHA-256（排除本报告）为 `fca4f7aeab559f873856e0cc17052badf8ed6321add6445392a45c09a4e49438`；没有未提交代码。本 R4 报告在指纹之后新建，不作为本轮输入。
- 关键目标文件 SHA-256：`typography.rs` `52a735c7426dfd6ff337892c0f8da3bf212f642700cab6b60ef6a7fecfe7721d`，`xgent_rules.rs` `c14da4fa9ffa451b96cc30faf7043fe969206f40fe43b4dab38383a32d6b1c8c`，`breaks.html` `48eb590fc9c8d2871a8ff89337632691d0827c2c4e3c3d11f8a43359713ad1b7`，`clean.html` `ccd92fa5a31b256f0fbf787d9ea7b401010328ba20ccb031f85c8983f514027b`。

### 3.2 实际执行的检查

静态检查：

```sh
git diff --check f273d6622342371545a7e0a4b70ef4537951c4f8 080c3bec7f652dc15bfd242233e18e2d54621e29 --
git diff --name-status 080c3bec7f652dc15bfd242233e18e2d54621e29 -- tools/phoenix-ui skills/phoenix-ui docs/phoenix-ui.md
```

前者通过、无 whitespace 错误；后者为空。完整阅读了修复 patch、当前 `typography.rs` 的字符收集/分行/分栏实现、`cjk.rs` 的孤字与拆单元消费端、两个夹具及浏览器断言。

真实入口统一使用当前 `tools/phoenix-ui/target/debug/phoenix-ui`，命令形态为：

```sh
tools/phoenix-ui/target/debug/phoenix-ui detect --json --viewport <375x812或1280x800> file://<页面>
```

| 页面/检查 | 实际结果 | 复审含义 |
| --- | --- | --- |
| 原样 `column-transform.html` / `column-rtl.html` | 各 1 条 `cjk-orphan`，末行“末”，退出 0 | F-02 两个原反例均关闭 |
| 原样 `column-zero-gap.html` | `[]`，退出 0 | F-04 同行溢出误报消失 |
| 原样 `zero-gap-real.html` | 1 条末字“末”的 `cjk-orphan`，退出 0 | 零栏距真实换栏未被保守逻辑吞掉 |
| 仓内 `cjk/breaks.html`，两个视口 | 各 7 条 orphan + 4 条 split-unit，共 11 条预期 `xgent/*`；窄视口另有非本计划规则发现 | 普通/容器/自身缩放/祖先缩放/RTL 的每栏一行及跨栏单位回归通过 |
| 仓内 `cjk/clean.html`，两个视口 | `xgent/*=0`；新增刻意溢出例会产生 3 条 `text-overflow`，不计作中文规则通过证据 | F-01、F-04 反例成立，未把非 xgent 输出藏成“全空” |
| 仓内 `webfont.html` | 1 条 split-unit、2 条 font-fallback，另有 1 条通用族归属未核实 note | 字体发现与未核实语义未见回归 |
| 仓内 `latin.html` / `mixed.html` | 均无 `xgent/*`，保留既有内置发现 | 非目标文字未被本次修复新纳入 |
| 临时组合页：缩放＋RTL；另加非对称 padding/border | 各报真实末字“末” | 两项修复及既有内容区原点共同工作 |
| 临时组合页：缩放＋RTL＋零栏距连续溢出 | `[]` | 组合场景未重新引入 F-04 |
| 临时组合页：RTL＋零栏距真实换栏 | 报真实末字“末” | 方向修复没有吞掉零栏距真实分片 |

另读取并实际查看了当前修复版本的 [缩放 capture 报告](../../local/phoenix-ui/evidence-craft-p2-20261111/r3-apply/column-transform/capture-report.json)、[RTL 报告](../../local/phoenix-ui/evidence-craft-p2-20261111/r3-apply/column-rtl/capture-report.json)、[零栏距报告](../../local/phoenix-ui/evidence-craft-p2-20261111/r3-apply/column-zero-gap/capture-report.json)及对应截图：报告均 `valid=true`、无控制台/请求错误，二进制哈希与本轮直接入口一致；截图分别可见右栏孤字、左栏孤字和连续单行，与发现一致。

外部只读核验：`gh run view 42991386581 --json status,conclusion,headSha,jobs` 证实旧候选已完整成功；`gh run list --workflow phoenix.yml --branch main -L 100 ...` 对当前修复和 HEAD 精确过滤均为空；`git ls-remote origin refs/heads/main` 返回 `2518626b5614ea5a8b1784763ae5712e5b2853ee`。没有推送、取消或重跑远端任务。

本轮复用已构建且哈希匹配的当前二进制与 M4 的当前构建/安装证据，没有重复构建或运行无关全量套件；临时组合页只用于本次验证，评审结束后清理。

### 3.3 行为链与审查分工

已沿 `detect/capture → scan_loaded_page → typography::collect/MEASURE_JS → Dom 行事实 → cjk orphan/split-unit/font-fallback` 追踪；重点反查 `fragmented`、`sameRow`、`columnGrid`、`linesOf` 的分片、坐标、方向、栏距与相邻字符条件，并用真实页面结果验证可见行为。M3 skill 文案、旧 PRD 全量范围及此前已审的无关实现不在本次修复增量内，本轮未重新评审。评审、验证与裁定均由当前主 agent 独立完成，无并行 reviewer。

## 4. 计划符合性

| 需求/章节 → 本次任务 | 实现与验证状态 |
| --- | --- |
| WORK-F5、§5.5、V-2④ → 分栏行事实须按真实坐标与阅读顺序归并 | 已核实：F-02 的普通、缩放、RTL 与组合场景均得到预期结果 |
| §5.5、V-2④ → 零栏距不可断同行文字不能误拆，真实换栏仍应分开 | 已核实：F-04 的纯文本、行内 span、真实换栏及 RTL/缩放组合均符合当前合同 |
| V-2⑦ → 保留节点不产出、不合并字体发现 | 已核实：相关代码未变，F-03 保持解决；本轮字体入口结果未见回归 |
| V-6/V-7/V-9 → 修复候选的构建安装、宿主与平台证据 | V-6 本轮缺口已补；V-7 宿主和当前 V-9 仍未完成，M4 不能记完成 |
| M3 与本期其余范围 | 本轮 diff 未触及，沿用前序评审而不宣称重新全审 |

计划顶部「需求来源」为 [PRD v1.2 · P2](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)。本轮不要求未来发布、部署或终验事项提前在代码修复中完成，也不把修复通过写成 PRD 验收。

## 5. 总体结论与修复顺序

验收判定：需补证

**修复结论：已解决；`patch is correct`（限 `f273d66..080c3be` 及本轮旧问题复核）。** R3 的 F-01–F-04 均已关闭，本次修复未发现新的 P0–P3 问题或直接回归。当前真实 `detect`、`capture` 报告与截图支持本机行为结论，但没有覆盖当前候选的 Claude 宿主取证和四平台 CI，因此不能通过整期阶段验收。

先恢复宿主环境并补 V-7，再经用户明确授权推送当前候选、补齐 V-9；证据齐备后用同一计划复审。解除阻塞的可观察条件是：V-7 当前修复版会话实际执行并交付所需产物，V-9 同一候选完整 SHA 对应的四个 UI Native 与 Node 18 任务全部成功。本判定不是合并、发布或 PRD 验收授权。

```text
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md
```
