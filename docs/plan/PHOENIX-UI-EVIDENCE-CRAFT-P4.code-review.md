# PHOENIX-UI-EVIDENCE-CRAFT-P4 · 代码变更评审 R1

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [PHOENIX-UI-EVIDENCE-CRAFT-P4](PHOENIX-UI-EVIDENCE-CRAFT-P4.md) | `8a49b86f216900bfa05797c07f57feedd1c5c60d..55fdfdfdb94a93621b2808980afefdf52fcad038`；另核当前未提交文档，未提交代码为空 | 无 |

## 1. 确认问题

### F-01 · [P2] 同键启动需要原子占位，避免双服务与回答互相覆盖

- 定位：`tools/phoenix-ui/crates/context/src/compare/mod.rs:324-333`（target / `55fdfdfdb94a93621b2808980afefdf52fcad038`）。
- 问题：两个携带同一 `--key` 的 `compare --start` 并发执行时，都能在对方写出状态前通过存活检查，随后各自删除状态、启动 detached 服务并读取任一服务写出的状态。实测两个公开命令均退出 0，却返回两个仍可访问的不同端口；它们共用 `<key>.state.json` 与 `<key>.answer.json`，因此后写状态会遮蔽先启动的服务，选择可能写入另一轮，未被跟踪的服务只能等超时，违背“同键已有服务应拒绝启动”的命令契约。
- 证据：当前代码在 `mod.rs:324-333` 检查后，于 `:377` 才 `spawn`，没有锁或原子占位；在仓库外临时夹具中用 Python 3 同时 `Popen` 两份 `phoenix-ui compare --start --key shared --payload round.json --timeout 90`，第 3 次有界尝试得到 `64529`、`64528` 两个端口，两个进程退出码均为 0，`GET /status?key=shared` 均返回 200。基线没有 `compare`，该竞态由本期新命令引入；顺序启动守卫和现有单服务测试不能覆盖此交错。复现服务已全部清理。
- 修复与验证：以 key 为单位原子预留启动权（含所有者标识及陈旧占用恢复），子服务确认自己仍持有该预留后才发布状态，父进程只回报自己启动的服务；增加真实公开 CLI 并发回归，断言同键只有一个成功启动、一个服务可写状态/回答，另一个得到明确的 key-in-use 失败，并检查无孤儿监听进程。
- 置信度：高；静态交错与真实公开入口复现一致。

## 2. 待核实 / 待决策项

无。未重复全量测试、四平台构建或另一次模型调用是覆盖限制，不是把本轮可取得的证据留给后续；下述 A2 缺口已有当前候选的真实失败证据，按本期未达成处理，而非待核实。

## 3. 覆盖与验证

**意图**：核对 P4 的方案对比页、动效/模糊反馈/细节/下一步指引及分发终验是否兑现当前计划承诺，并找出本期引入的可行动缺陷。

- 仓库根：`/Users/rockie/Documents/GitHub/xgent/skills`；核查时间：2026-11-13 14:09 +08:00；主 Agent 独立评审，无并行 reviewer。
- 原输入：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md`，未另指定 diff 或文件限制，按计划登记提交与当前未提交变更进入计划模式；本计划无既有代码评审报告，故为首轮完整评审。
- 固定比较：`8a49b86f216900bfa05797c07f57feedd1c5c60d..55fdfdfdb94a93621b2808980afefdf52fcad038`，累计净差异；排除路径仅为 `skills/use-llm-gateway/**`，diff SHA-256 `059e9d3df48ae69e8700e07fa54d2efbef0390ac3f20cd8e25a7111f4410e524`。
- 登记实现提交：`0a49480dfe8e8ae94d03e892c5e58ceddc9e844b`（M1/M2）、`3adeb8a62e643cfe055279d0f25af824cf9c07fc`（M3）、`d2fe94b15b9c34d368b234611a4b7dc8f97e919f`（M4）、`28a51d5e9304aa0bd189b69da910fd38f6294f1b`（宿主修订）、`4844a9033e5a6f2fbf5e3671a511e0246f869473`（M5 候选）、`c93dec5af7cc8dd8aa5d2476d75e3aac6ffa14ee` 与 `0279aecccfebb057ba3685bd1aa04e94609638c0`（模糊反馈修订）、`a18e025bc2faaae0f8450581789e160cf27fc903`（示例标注）、`18852ff1cb80441f7fc6c8fcb6ccf215b5f2374a`（动效交付边界）。均已解析并确认为 HEAD 祖先。
- 登记但仅进度/计划文档的提交：`90045192f3f56f8a40ee6e83fe5dd7cd732566bb`、`55fdfdfdb94a93621b2808980afefdf52fcad038`；另排除纯进度提交 `d8bb7f80413a06324fd14a06c20853984c73864c`、`f245216d8bc3ba46febbe539b9c0b4ec745fdbae`、`3256e46d8307c6a5e407a8205ce3aa09bdf3e959`。
- 未登记范围：用户明确选择排除 `f98e3aaea2e6b28c2d0dd8142327bbb986ab52d4`（`skills/use-llm-gateway/SKILL.md`、`references/api.md`）及合并提交 `26d654155e4b5ce7b5f2ebb023b6f7b9e862974f` 带出的这两个文件；没有把它们混入 findings。
- 当前工作树：无 staged 差异，index 指纹为空 diff 的 SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`；tracked diff SHA-256 `ede436c74b4c9aa40d50490f7cac53ed7a18ca6a9103e9f5fd775dc268bcf47a`，仅计划、M5 记录、经验与计划状态页；报告创建前 untracked 为空，无未提交代码。
- 附带版本：计划工作树 SHA-256 `2d858da555d0ab8026ca2b540d85953e2f2139fd9451ef519acf8fed2ff193af`；M5 记录 SHA-256 `3ce471383f5087bb4ea43a531b74b2320d930973e5b3ebe3236a12961932fc2d`。评审报告自身不作为输入。
- 逐读范围：完整盘点变更文件；逐读 `compare` 的 Rust 服务/载荷/页面实现、CLI 路由与辅助函数、浏览器和原始套接字测试、compare 冒烟、P4 skill 源指引及相关 PRD/计划/记录。19 个提供方生成物通过 fresh 检查，不重复逐行抄写核对；既有许可清单和未触及模块未做独立全量审计。

| 命令 / 证据 | 目标版本与实际结果 | 证据性质 |
| --- | --- | --- |
| `mise exec --no-deps -- cargo test -p impeccable-context --lib compare --locked --offline`（`tools/phoenix-ui`） | `55fdfdfd` 源码；7 passed，0 failed | 载荷与校验局部测试 |
| `mise exec --no-deps -- cargo test -p phoenix-ui --test compare_security --locked --offline -- --test-threads=1` | 1 passed；覆盖 key、Origin/Host、目录越界、点文件、CSP 与原子回答 | 真实服务原始套接字 |
| `mise exec --no-deps -- cargo test -p phoenix-ui --test compare --locked --offline -- --test-threads=1` | 1 passed，非 skip；真实浏览器验证静态方案不执行脚本、交互方案可执行、外联探针零请求，页面选择经公开 `--wait` 返回 | 当前版本真实浏览器主路径（操作模拟，不声称用户选择） |
| `mise exec --no-deps -- bun scripts/render.mjs --check` | 19 个提供方全部 fresh；input `ac29392a…`，bundle `3638cf29…` | 渲染与分发一致性 |
| `node ../../scripts/check-phoenix-lints.mjs`（`tools/phoenix-ui`） | `No added diagnostics`；既有 rustfmt 差异与 507 条 clippy 记录未扩大处理 | 变更 lint |
| `git diff --check <base> <target> -- . ':(exclude)skills/use-llm-gateway/**'` | 通过，无空白错误 | 静态交付检查 |
| 临时 Python 3 双 `--start` 复现 | 两个同键命令均退出 0 且两个服务均存活，详见 F-01 | 真实公开 CLI 并发实验 |
| [M5「完整 A2 复验」](PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M5.md#完整-a2-复验当前候选)及 `/tmp/phoenix-p4-a2-failure-evidence/rapid-{200,800}.png` | 当前候选 `55fdfdfd` 的公开安装、真实宿主与显式 Skill 执行证据；本轮直接开图，两个时刻均为计数 3 而列表空白；两图 SHA-256 均为 `a15d4fd19b3abb9364aaf9347583364bf59755b2045cf880445a31578af53bed` | 当前版本真实入口既有证据 + 本轮图像核查 |

本轮没有再次调用模型，没有把一次显式 Skill 调用推断为自然触发稳定，也没有把局部测试当作完整 A2。未重复 `test:phoenix` 全量、四平台构建、Node 18 冒烟或 CI；M5/V9 已有记录仅作历史/current-candidate 佐证，不列为本轮执行通过。A2 的快速筛选失败已有确定证据，不需要用另一次随机模型运行来推翻或确认。

## 4. 计划符合性

计划版本为上述工作树哈希；本次覆盖 M1–M5 已登记实现及当前 M5 验收门槛，只要求本期承诺。

| 需求 / 里程碑 | 实现与验证映射 | 状态 |
| --- | --- | --- |
| WORK-F10 / M1、M2 | 三类方案、隔离、并排/单个/缩放/键盘、选择回传均有实现与本轮测试；同键并发启动存在 F-01 | 存在缺陷 |
| WORK-F11、F13 / M3、M4、M5 | 动效与细节规则已进入源 skill 和 19 个渲染目标；完整 A2 仍有 G-01，不能凭局部定向通过关闭 | 本期缺口 |
| WORK-F12 / M3、M4 | 已核先回复变量并等待用户回复、无人值守不直接修改的定向记录；完整 A2 未收口 | 定向证据存在，整体验收未达 |
| WORK-F14 / M3、M4 | 已核下一步编号、代价、例外与宿主遵循记录；本轮未发现新增文本契约缺陷 | 沿用已有证据，完整 A2 未达 |
| WORK-N2、N4 / M5 | 当前渲染 fresh；V5 宿主与 V9 四平台记录已读，未在本轮重复构建 | 有登记证据，覆盖限制如上 |

### G-01 · [P2] M5 完整 A2 尚未达到“快速连续操作不空屏”的退出门槛

- 定位：`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.md:754-758`；`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P4.records/M5.md:54-60`。这是计划符合性缺口，不把尚未证明的根因硬归到某一行 skill 文本。
- 问题：在当前候选、公开安装、真实 Claude Code 宿主且实际调用 Skill 的完整 A2 中，快速连续点击筛选后，`rapid-200.png` 与 `rapid-800.png` 都显示“3 件”但任务列表为空；计划明确要求快速连续操作不空屏，记录也明确 M5 仍进行中。该失败直接产生用户可见的错误状态，其他 V6/V9 或局部定向通过不能替代这一退出条件。
- 修复与验证：执行者为计划后续实施；先定位并修复本次实际筛选/动效状态转换问题，再以同一完整 A2 请求和当前公开入口复验，实际加载 Skill，覆盖快速连击、反向、减少动效与过滤正确性，确认无空白、状态与列表一致并保存当前版本截图/报告。不得仅凭增加局部测试或另一次简化场景关闭本项。

## 5. 总体结论与修复顺序

**验收判定：需要修改。**

`patch is incorrect`：F-01 是本期新增且已复现的服务生命周期缺陷；G-01 是当前计划明确承诺但尚未达成的用户主路径门槛。对比页隔离、常规选择回传、渲染一致性等检查通过不能覆盖这两项。

1. 先修复 F-01 的同键启动原子性，并用公开 CLI 并发回归证明只存在一个有效服务。
2. 由计划后续实施完成 G-01 的真实筛选修复与完整 A2 复验，再复审相关代码及当前计划；本结论不是合并或发布授权。
