# ELINTP-VISUALS-P2 · 代码变更评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [ELINTP-VISUALS-P2](ELINTP-VISUALS-P2.md) | `1ee47f4bbb3458afc6f9c6934e482cdadb763c3e..924f8793f37733d9faf7b0aed87aca7ef07c0fba`；另核对当前未提交变更 | 无 |

- 评审日期：2026-11-11（Asia/Shanghai）。当前主 agent 独立审查，未使用并行 reviewer。
- 计划版本：SHA-256 `114e04a9b0906f26d2a8d2d93de248a085edacf8b25e55da39a98ff6dd92da2b`；需求来源 PRD 版本：SHA-256 `ff8353d0d1026e2a9e2dd1d884e12ab2708f33fc4f2897cddb097e15373e0d3b`。

## 确认问题

### F-01 · [P1] 让 CLI 主模块判断兼容符号链接路径

- 定位：target `924f8793f37733d9faf7b0aed87aca7ef07c0fba`，[skills/elintp/scripts/render.mjs:69](../../skills/elintp/scripts/render.mjs#L69)。
- 问题：新增的入口判断用 `resolve(process.argv[1])` 对比 `fileURLToPath(import.meta.url)`，前者不解析符号链接，而 Node 的模块 URL 使用真实路径；从符号链接安装目录或 macOS 的 `/tmp` 链接路径启动时，条件为假，`main()` 完全不执行。原有直接渲染和新增 `--assets-from` 终稿都会以 0 退出、无错误且没有输出文件，既破坏既有调用兼容性，也会让 Agent 把缺失终稿误判为成功；基线在相同路径下会直接执行并正常生成页面，因此是本次重构引入的回归。
- 证据：本轮在隔离目标副本执行两参数命令 `node /tmp/elintp-p2-code-review.Kv1FZt/skills/elintp/scripts/render.mjs draft.md target-two-arg-via-link.html`，实际为退出 0、stdout 为空、输出不存在；`--assets-from` 同样如此。改用该脚本的 `/private/tmp/...` 真实路径后立即生成页面；基线 `1ee47f4b` 经同一 `/tmp` 链接路径生成 13,678 字节页面。计划 [M4 / C2](ELINTP-VISUALS-P2.md) 要求保留旧的两参数终稿 CLI，[AGENTS.md:185](../../AGENTS.md#L185) 明确存在 source symlink 本地安装方式。
- 修复与验证：入口比较前将启动路径规范化为真实路径，同时保持被 `review.mjs` import 时不执行 CLI；新增通过符号链接路径 spawn 的回归测试，分别断言旧两参数及 `--assets-from` 退出 0、实际生成预期 HTML，再复跑 `test/elintp/render.test.js`、`test/elintp/review.test.js` 和真实终稿入口。
- 置信度：高；静态原因、目标复现、真实路径对照和基线对照一致。

## 待核实 / 待决策项

无。发现的真实入口故障已在评审边界内完成最小复现与基线对照，不需要用户补证。

## 覆盖与验证

**一句话意图**：本期在 elintp 固定模板中加入本机人工标注、受保护的连续局部重讲、可恢复内容稿/资源快照及无审阅数据的离线终稿，并保留原阅读与终稿生成能力。

### 范围与快照

- 仓库根：`/Users/rockie/Documents/gh-xgent/skills`；原始输入：`docs/plan/ELINTP-VISUALS-P2.md`，按计划模式确定为初审。
- 基线：`1ee47f4bbb3458afc6f9c6934e482cdadb763c3e`；目标：`924f8793f37733d9faf7b0aed87aca7ef07c0fba`，已验证是当前分支祖先。比较方式为两端树的累计净差异，不叠加中间状态。
- `924f8793..HEAD` 仅有 `e658c5cad30b9b52e433e67d3ec123e55d2bb4c3`，且只修改本计划的完成记录，作为进度回写排除，不纳入代码 findings。
- 纳入完整差异：`skills/elintp/` 的 9 个运行时代码、模板资源及公开文档文件，以及 `test/elintp/render.test.js`、`test/elintp/review.test.js`；同时读取 PRD WORK-F4/WORK-N2、P1 既有 `draft.mjs`、`page.css`、`page.js` 和调用方。提交中的计划、里程碑记录及 `.xgent-ai` 状态/经验文件按过程材料核对，不作为可执行代码审查。
- 冻结时目标与 HEAD 的 `skills/elintp/`、`test/elintp/` tree 相同；`git diff 924f8793 -- skills/elintp test/elintp` 为空，相关 untracked 清单为空。当前仓内其他 PRD、评审、状态和经验 dirty 文件不属于本实现代码；附带 PRD 仅作需求上下文。
- 运行验证使用独立临时 worktree `/tmp/elintp-p2-code-review.Kv1FZt`，实际 HEAD 为 `924f8793f37733d9faf7b0aed87aca7ef07c0fba`；没有用当前 checkout 的测试替代历史目标验证。

### 已审行为链

1. **M1 / 首轮审阅**：draft 解析与区域登记 → 共用 review/final 渲染 → 浏览器块目录、三类标注、编辑与 localStorage → loopback POST → 服务端校验、权威结果与退出。
2. **M2 / 连续局部重讲**：上一结果及快照校验 → 当前资源覆盖/缺失回退 → 页头/结构与逐区域正文、引用资源指纹 → 清除已改区域旧标注并保留未改区域标注 → 同轮同路径/端口恢复。
3. **M3 / 离线终稿**：从当前页或结果 JSON 提取快照 → 校验并规范化资源 → 共用正文构造 → 去除审阅控件、标注与待审结果，只嵌入内容稿和实际资源。
4. **M4 / 兼容与交付**：原 CLI、公开说明、frontmatter、分发文件、旧阅读逻辑、测试和真实入口证据；发现 F-01 所覆盖的兼容缺口。

### 本轮实际验证

| 类型 | 命令或入口 | 实际结果与限制 |
| --- | --- | --- |
| 目标版本定向测试 | Node `v25.2.1`；`node --test test/elintp/render.test.js test/elintp/review.test.js` | 13/13 通过，覆盖安全嵌入、资源往返、真实 HTTP 校验/重试、局部区域保护、跨目录恢复及非法上一结果。测试路径为真实路径，因而未覆盖 F-01 的符号链接入口。 |
| 静态与分发 | `git diff --check 924f8793^ 924f8793 -- skills/elintp test/elintp`；`npm pack --dry-run --json` 后只解析清单 | 均通过；elintp 11/11 文件在包内，新增 `review.mjs`、`review.js`、`review.css` 为 3/3，未发布或生成 tgz。frontmatter 三个字段无差异，未触及 spec-human-decision、Phoenix、summarize 或 README。 |
| 真实 Chrome 主路径 | 在目标隔离副本从公开 Node 入口启动审阅；真实 Chrome + CDP，360px 首轮及 1440px 终稿 | 首轮实际完成 s1 标注、刷新恢复和主动提交，结果含正确 section/quote，服务退出 0；修改已标注 s1 后进入 round 2，未改的 s2 可标注，同输出、同结果路径及同端口重开后恢复未提交标注；页面在 360px 无横向溢出。 |
| 离线终稿 | 空 cwd 从当前审阅页 `--assets-from` 生成终稿，Chrome `file://`、`offline=true` | 绕过 F-01、改用真实脚本路径后，终稿含修改后的精确稿本及原图，保留未改区域，主题切换可用，无 review JSON/工具栏，HTTP(S) 请求为 0；1440×900 截图已实际读取，正文、表格、大纲与主题可读。 |
| 候选问题对照 | 目标符号链接路径 vs 真实路径 vs 基线符号链接路径 | 目标两种 CLI 均静默无输出，真实路径成功，基线成功；确认 F-01，不以推测或测试缺失代替缺陷证据。 |
| 既有真实使用记录 | 读取 M1/M3/M4 的 V-1–V-7 记录并与当前提交对照 | 作为当前实现的已有证据保留，包括存储禁用、打印/no-script、资源缺失及全新作者交接；本轮未重复三位全新作者的角色模拟，也不把这些记录升级为真人 UAT 或满意度结论。该限制不影响 F-01 的成立。 |

## 计划符合性

| 计划 / 需求 | 本轮映射 | 结果 |
| --- | --- | --- |
| M1；WORK-F4 AC-1/AC-2；WORK-N2 AC-2/AC-4 | 默认审阅页、块标注、持久化、loopback 提交、结构化结果 | 代码与本轮 Chrome 主路径已核实；写失败保留服务由目标测试覆盖。 |
| M2；WORK-F4 AC-3；WORK-N2 AC-3/AC-4 | 区域/资源保护、旧标注去留、缺源恢复、连续轮次与同轮重开 | 已核实；普通/嵌套块、共享资源和非法快照由代码与测试覆盖，浏览器核实同轮恢复。 |
| M3 / C1、C3；WORK-F4 AC-2/AC-5 | 公开恢复流程、自包含离线终稿、无审阅数据 | 真实路径下已核实；内容稿与资源恢复及离线行为符合本期契约。 |
| M4 / C2；WORK-F4 AC-2 | 保留旧两参数终稿 CLI 并新增资源覆盖 | **存在缺陷 F-01**：符号链接路径下旧、新 CLI 都不执行且错误地返回成功。 |
| M4 / V-6、V-7；PRD P2 与边界 | 定向回归、分发、旧页/存储/打印及真实使用；不扩 C7、不改 frontmatter | 本轮完成直接依赖检查并读取持久记录；未发现越界实施。V-7 角色模拟未重复，真人 UAT 不属于本期替代结论。 |

## 总体结论与修复顺序

**需要修改**

正确性结论：**patch is incorrect**。F-01 使受支持的符号链接调用静默成功但不交付终稿，直接违反本期保留 CLI 兼容性的承诺。其余已覆盖的审阅、局部重讲、资源恢复和离线阅读路径未发现可确认问题；本结论只覆盖上述目标提交与声明的工作树范围，不是合并或发布授权。

1. 先修 F-01 的真实路径入口判断，并补两种 CLI 经符号链接启动的行为回归。
2. 在修复提交上复跑两个 elintp 测试文件，再经符号链接实际生成审阅页和离线终稿，确认 import 不触发 CLI、输出文件真实存在且内容一致。
