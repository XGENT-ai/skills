# Phoenix UI · R3 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3（修复效果复审） | [PHOENIX-UI](phoenix-ui.md) | `183bc42cd0b24f7ebf97bd20b2c25e21528101aa..8f6752774204f06a8e8220c621e4c8bc9369d44b`；实现提交 `36ec3f48f63dcdbc40e9fd9f80d68a54dbece6a3`；无未提交代码 | [R2](phoenix-ui.code-review.r2.md) |

本轮核对 R2 的全部七项旧问题及待核实项，并检查 F-03 补修直接影响的回归；没有重新完整审查 M1–M6 的原累计变更。R2 报告 SHA-256 为 `bd73e864d518ced917368b2484a61f0809889c974b548bc0bc6cdcd80f6b44f6`，评审对象 ID 为 `3833fbb3-87bd-4e75-8a81-e2e3d7622a90`。本轮独立评审对象 ID 为 `207ec261-6cff-436e-b8eb-ba88c49df34c`。

上轮审查起点是 R1 target `a5ed3cc5219f575b480e726b56be3d87be34a79d`，上轮实际实现增量为 `28789e485046c47aae2bbef9e49d3b5461767da8..43a27a3c0a7f87d8b30e7a27186691a1c58af582`，核查目标为 `183bc42cd0b24f7ebf97bd20b2c25e21528101aa`。本轮从该核查目标比较到当前 HEAD。补修提交 tree 为 `b02db9dde4794b3205b9d8598c9456b0be59262d`；其后 `8f6752774204f06a8e8220c621e4c8bc9369d44b` 只登记计划中的修复 SHA，运行时与补修提交字节一致。

已有处置：[M3 的 R2/F-03 补修](phoenix-ui.records/M3.md#r1代码评审修复f-01f-04)、[M6 重开边界](phoenix-ui.records/M6.md)。作者记录仅作定位，本轮重新运行相关反例并核对代码。

## 上轮问题复审

来源均为上述 R2 报告及版本，沿用 R1 的 F-ID；修后核查版本均为当前 HEAD。

| 来源 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R2 / F-01 | P1；兼容项目 install/update 绕过事务和 dry-run；R2 已解决 | R1 修复 `43a27a3c0a7f`；本轮未再改路由 | **已解决** | [main.rs:55](../../tools/phoenix-ui/crates/cli/src/main.rs#L55)仍接同一事务入口；公开 CLI 的根 install、skills install/update 三个 dry-run 均 planned、树不变；[兼容入口回归:86](../../tools/phoenix-ui/crates/cli/tests/skills_install.rs#L86)通过 | 维持关闭 |
| R2 / F-02 | P2；local 迁移丢失 Git 忽略；R2 已解决 | `43a27a3c0a7f`；本轮抽出共用 Git 定位函数 | **已解决** | [exclude_local_state:675](../../tools/phoenix-ui/crates/install/src/installer.rs#L675)与旧解析等价；[真实 Git 回归:840](../../tools/phoenix-ui/crates/install/tests/installer.rs#L840)通过，子项目/linked worktree 的 local 配置被忽略，Unchanged 重装恢复旧块 | 维持关闭 |
| R2 / F-03 | P2；实际 Git 根/主 worktree local 来源遗漏；R2 部分解决 | 补修 `36ec3f48f63d` | **已解决** | [settings 合并入口:443](../../tools/phoenix-ui/crates/install/src/installer.rs#L443)、[local 来源:479](../../tools/phoenix-ui/crates/install/src/installer.rs#L479)；31 个布局/设置组合及另加 4 个裸库反例均与实际 Claude 2.1.299 一致；原漏挡实装现退出 3/conflict、无 active receipt；[新增 Rust 回归:985](../../tools/phoenix-ui/crates/install/tests/installer.rs#L985)及既有启用矩阵通过 | 原剩余问题关闭 |
| R2 / F-04 | P2；成功迁入不可消费的 JSONC；R2 已解决 | `43a27a3c0a7f`；本轮未改迁移 reader | **已解决** | [strict_json:548](../../tools/phoenix-ui/crates/install/src/data.rs#L548)、[注释配置回归:235](../../tools/phoenix-ui/crates/install/tests/data.rs#L235)通过；data 32 项通过。公开 CLI 注释 local 配置退出 3、原字节保留且不写 Phoenix 状态 | 维持预检拒绝方案；未宣称新增 JSONC 支持 |
| R2 / F-05 | P2；草稿 status 不识别证据漂移；R2 已解决 | `43a27a3c0a7f`；本轮未改 review | **已解决** | [status.rs:76](../../tools/phoenix-ui/crates/review/src/protocol/status.rs#L76)、[回归:2526](../../tools/phoenix-ui/crates/review/tests/cli.rs#L2526)通过；目标及登记图片改变均 unknown/退出 4、status 只读，写命令也拒绝漂移 | 维持关闭 |
| R2 / F-06 | P2；限定轮把范围外格子算缺采；R2 已解决 | `43a27a3c0a7f`；本轮未改 review | **已解决** | [finalize.rs:967](../../tools/phoenix-ui/crates/review/src/protocol/finalize.rs#L967)、[回归:2585](../../tools/phoenix-ui/crates/review/tests/cli.rs#L2585)通过；公开 CLI 的 U1=7 限定轮为需要修改、evidenceComplete=true，U1 自身缺采仍需补证 | 维持关闭；限定达标仍不能宣称全局通过 |
| R2 / F-07 | P2；一单元独有附件缺项阻断所有单元；R2 已解决 | `43a27a3c0a7f`；本轮未改 review | **已解决** | [packet.rs:359](../../tools/phoenix-ui/crates/review/src/protocol/packet.rs#L359)、[回归:2635](../../tools/phoenix-ui/crates/review/tests/cli.rs#L2635)通过；公开 CLI 可导出目标完整的 U1，U2 明确未评；共享必需目标缺项仍阻断 | 维持关闭；缺证轮不能通过 |

七项均核对：**F-03 在本轮关闭，其余六项维持已解决；无不再适用或本轮未复核项。**

## 确认问题

未发现本轮修复引入或激活的独立新增缺陷。F-03 的原剩余误阻断和漏挡均已解决；没有把它重新编号为新增 finding。

### F-03 的关闭证据

现在按 user、起始目录 shared project、起始目录旧 local、实际根 local 的顺序合并，后读的同键覆盖前值。正常 Git 子目录采用仓根，linked worktree 采用主检出根；非 Git、home 根、不能从 commondir 推导检出根的布局保留起始目录。该顺序与当前官方[本地设置位置及旧文件规则](https://code.claude.com/docs/en/settings#where-claude-code-keeps-the-local-file-in-a-git-repository)一致，并通过真实 CLI 对照核查。

下表从本轮实际生成的 results.json 摘取，注册表始终保留有效的旧插件 user 安装；root 指普通仓根或主检出根的 local 文件。

| 场景 | user / root local / cwd local | Claude enabled | 修后 Phoenix |
| --- | --- | --- | --- |
| 普通仓根禁用 | true / false / 同根 | false | 0 / planned |
| Git 子目录，根禁用 | true / false / 无 | false | 0 / planned |
| Git 子目录，根启用 | false / true / 无 | true | 3 / conflict，指向根 local |
| Git 子目录，旧 cwd false | true / true / false | true | 3 / conflict，指向根 local |
| Git 子目录，根 false 覆盖旧 cwd true | false / false / true | false | 0 / planned |
| linked worktree，主检出禁用 | true / false / 无 | false | 0 / planned |
| linked worktree，主检出覆盖旧 cwd false | true / true / false | true | 3 / conflict，指向主检出 local |

完整 31 组包含无 Git、根/子目录、linked worktree 根/子目录、bare 和 separate-git-dir worktree、separate-git-dir 子目录、home 即仓根。修前保留的 R2 engine 在相同输入和 Claude 2.1.299 上有 10 组不一致；本轮构建 engine 为 31/31 一致，Claude 查询均无加载错误。另加 bare repository 恰名为 `.git` 的 4 组，全部一致，未把推测的根定位差异报成 finding。

原漏挡反例去掉 dry-run 实装后：**退出 3、status=conflict、active receipt 不存在，Claude 仍 enabled=true。** 这是激活预检的核查；未启动真实模型会话，不声称观察了旧/新 hook 同时执行。

## 待核实项与验证限制

- `mise exec --no-deps -- npm run test:phoenix` 实际退出 1，止于系统 Node 19.3.0 的 WASM stringref CompileError，尚未进入 oracle。此前原生构建、Node 安装、Rust 103 组 1270 passed / 0 failed / 9 ignored、lint 无新增诊断已完成。外层 mise 和 preflight 检查的固定 Node 25.2.1 并未改变 npm 生命周期中脚本的实际 Node 入口。
- 随后以实际 Node 25.2.1、相同 preflight/buildEnvironment 补跑剩余阶段：WASM 8474 向量、golden/native/parity 0 失败；完整 oracle 859/860 通过、475 个精确适配、1 fail、0 error/missing/skip。唯一失败为 detect-fixture-text-framework-vite 的 8080 占用提示。`lsof -nP -iTCP:8080 -sTCP:LISTEN` 确认 Docker Desktop PID 62891 监听该端口；修前 R2 engine 与当前 engine 的该单例 actual 完全相同，detect 源码不在本轮 diff。没有把环境差异归为补修引入的缺陷，也没有停止用户的 Docker 服务。
- 仍缺完整入口单次退出 0。M6 应在固定 Node 25.2.1、8080 无冲突的隔离环境或新候选 CI 补齐；可直接用 `mise exec --no-deps -- node scripts/test-phoenix-ui.mjs` 确保脚本的实际 Node 入口。补跑和初次失败均保留，不合并冒充一次全绿。

- 当前补修尚无新四 native/Node18 CI 或同一最终 tarball 的 V-4、受影响 V-7/V-10 证据。[计划恢复快照](phoenix-ui.md#恢复快照)已将它们列为 M6 后续收口，本轮没有把后续工作报成额外代码缺陷，也没有将修复增量通过扩大为整期完成。
- 没有重跑 Chrome/Safari 产品界面走查或 Claude/Codex 真实模型会话；plugin list 仅为隔离配置状态查询。其他 UID 所有权分支依据当前官方规则和代码静态核查，未用另一账户实测。Windows 为本期明确延期范围。

## 覆盖与验证

变更意图：补齐 Claude 实际 Git local 设置来源和覆盖顺序，解除 R2/F-03 的误阻断及漏挡，同时复用 Git 定位而保持 local 数据忽略保护。用户明确请求本计划代码评审，修复意图由计划登记与 diff 确认。

- 日期：2026-10-14，Asia/Shanghai；仓根 `/Users/rockie/Documents/GitHub/xgent/skills`；分支 `phoenix-ui/m2-verification-20261009`。
- 原输入：`$review-code docs/plan/phoenix-ui.md`；开发计划模式，已有最大轮次 R2，采用修复效果复审。完成记录明确写出的 27 个 SHA 全部解析成功且为当前 HEAD 祖先。
- 精确比较：`git diff --no-ext-diff --no-textconv 183bc42cd0b24f7ebf97bd20b2c25e21528101aa 8f6752774204f06a8e8220c621e4c8bc9369d44b -- tools/phoenix-ui`；仅 installer.rs（69 行增加、19 行删除）和 tests/installer.rs（225 行增加），完整读取 patch 并追踪调用方。它与 `183bc42c..36ec3f48` 在运行时路径的 diff 相同。
- 当前 HEAD/tree：`8f6752774204f06a8e8220c621e4c8bc9369d44b` / `15b003f59d615218ad28d44aface984555987850`。起始 worktree/index clean，无 untracked 输入，index tree 等于 HEAD tree。区间只有登记的补修及计划 SHA 登记，没有需另行澄清的未登记实现提交。
- 排除：补修中的 R2 报告、计划/里程碑记录、台账与经验回写不作为新增代码 finding 对象；`8f6752774204f06a8e8220c621e4c8bc9369d44b` 是纯计划登记。此前排除的独立 agi-mode 提交 `a391a034d85c7fc45d40df036ea164fa771eee1b` 在本轮 base 之前，仍未纳入。
- 计划 SHA-256：`07a9301387b68c6bde7a54b713e5f9db7d6a6868bfabe84569faf7bcae2ebb64`。起始全部存在 tracked 文件的内容指纹为 `df4535056bd6d2178f54d45726c638d45cb5f3d076b2f938c39e3883e00dd9c8`：对 git ls-files 枚举的文件逐个 SHA-256，再对按路径排序、紧凑 JSON 的 `{path:hash}` 作 SHA-256。完整集合保存于临时 snapshot.json。写报告前已逐文件核验原集合无变化；本轮报告和台账不纳入审查输入，交付前再排除这些自有产物核验原集合。
- 深入覆盖：Project 根 canonicalize → install personal discovery → 三层设置及旧/根 local 覆盖 → .git 文件/目录和 commondir → home/UID 边界 → 激活冲突；共用函数的 exclude 与重装。另核对上轮六个已关闭问题的当前实现与公开 CLI 回归。
- 本轮由主 agent 单 reviewer 完成，没有派独立子 agent；重新读取、执行并断言作者提供的 31 组比较脚本，不以处置记录的“已修复”替代观察。没有完整重审原承接运行时、其他计划或发布资产。

### 实际验证

所有当前测试在上述 HEAD、相同源码指纹上执行。Rust 构建使用仓库 preflight/buildEnvironment，固定 Rust 1.99.0、mbx 1.21.1 和其选定构建工具，依赖 locked/offline；macOS 原生测试按入口的 loopback 网络策略运行。npm 完整入口实际使用系统 Node 19.3.0，剩余阶段直接以 Node 25.2.1 执行，版本与结果分别记录。

| 命令 / 版本 | 实际结果 | 覆盖 |
| --- | --- | --- |
| `mise exec --no-deps -- npm test`；当前 HEAD | 退出 0；16 个 Node 测试文件通过；Python 22/24/2/13 项 OK | 轻量与元数据回归 |
| `mise exec --no-deps -- npm run test:phoenix`；当前 HEAD | 退出 1；原生构建、Node 安装、Rust 103 组 1270 passed / 0 failed / 9 ignored、lint 无新增诊断完成；WASM 在实际 Node 19.3.0 入口失败，未进 oracle | 原生构建、Node→Rust 安装、Rust workspace/browser/CLI、lint、WASM 和 oracle；各阶段结果按实测记录 |
| `mise exec --no-deps -- node <临时目录>/verify-remaining.mjs`；当前 HEAD、实际 Node 25.2.1 | WASM 退出 0，8474 向量、golden/native/parity 0 失败；oracle 退出 1，859/860 通过、475 精确适配、唯一 8080 提示差异，0 error/missing/skip | 在相同 buildEnvironment 下补完首轮未完成阶段；没有重跑已通过的 Rust 与 Node 安装组 |
| `python3 -I <临时目录>/plugin-oracle.py --output <隔离目录> --engine before=<R2 engine> --engine after=<当前 engine> --bundle-root vendor/phoenix-ui/bundle --activate after`；随后读取 results.json 独立断言结果 | 退出 0；修前 10 组不一致，修后 31/31 一致；启用时均 3/conflict，禁用时均 0/planned；原漏挡实装被拒绝、无 active receipt | F-03 的原输入、扩展 Git 布局和激活门；Claude 2.1.299，独立 HOME、禁网，无真实账户会话 |
| `python3 -I <临时目录>/verify-bare-layout.py ...`；结果断言 | 退出 0；额外 4/4 一致，无 Claude 加载错误 | bare common directory 名为 `.git` 的直接相关反证 |
| `python3 -I <临时目录>/verify-critical.py`；当前构建 engine | 退出 0；三种 dry-run 树不变；注释 local 拒绝且原字节保留；目标/图片漂移 unknown 且只读；限定 U1=7 为需要修改且证据完整；缺 U2 目标仍导出 U1 | F-01/F-04/F-05/F-06/F-07 的原公开 CLI 反例；结构 fixture 不作 UI 实拍 |
| `mise exec --no-deps -- node scripts/check-phoenix-oracle.mjs --bin <engine> --prefix detect-fixture-text-framework-vite --include-actual --output <临时JSON>`；分别用 R2 engine 与当前 engine | 两次均退出 1；1/860 的该例失败，actual 完全相同，均为 8080 占用提示；0 error/missing/skip | 与上轮未关闭的 oracle 环境差异作修前/修后对照；不冒充完整 oracle 通过 |

当前 engine 从本轮 test:phoenix 的 Cargo compiler-artifact 构建产物副本保存，SHA-256 `d7f0d0efa3657aa00eb923ef1a2ae202d19ab3b9251c88d327c4bee3993ab04c`。修前 engine 使用 R2 保留的实际构建副本，SHA-256 `70c8480d25a8dd30cc42db1414fa3c5a295ce41be644f63660e88127833160a5` 与 R2 报告一致，没有拿当前 checkout 的测试冒充历史版本验证。

原始日志：`/tmp/phoenix-r3-npm-test.log`、`/tmp/phoenix-r3-runtime-test.log`、`/tmp/phoenix-r3-remaining.log`、`/tmp/phoenix-r3-plugins.log`、`/tmp/phoenix-r3-bare.log`、`/tmp/phoenix-r3-critical.log`、`/tmp/phoenix-r3-oracle-baseline.log`、`/tmp/phoenix-r3-oracle-current-vite.log`。临时证据目录为 `/var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-r3-review-ljiblra9/`，含 snapshot/scope、构建 engine、脚本、各组 results.json、replay-summary、Node 25 剩余阶段脚本/结果、全量 oracle-current.json 和两版单例 oracle JSON。临时材料不随 Git 共享；本报告保留条件和结果，其他机器需重建隔离 fixture。

## 计划符合性

| 本次承诺 | 实现与证据 | 结论 |
| --- | --- | --- |
| M3 / R-6 / §3.1 / V-4；有效旧入口发现与拒绝双运行时 | 根与起始目录的 local 来源及覆盖符合当前宿主；35 组真实状态对照与原漏挡实装拒绝 | F-03 剩余问题关闭 |
| M3 / NFR-4 / V-4；local 数据 Git 忽略及重装恢复 | enclosing_git 与旧 exclude 定位等价；真实 Git 子目录、linked worktree 和旧块恢复回归通过 | F-02 未回归 |
| M5 / R-4 / V-7/V-8；草稿漂移、限定轮和缺证单元隔离 | 实现相对 R2 未改，原反例和相关回归再次通过 | F-05–F-07 维持关闭 |
| M6；修后新候选、CI 与最终包收口 | 计划仍为 5/6，M6 进行中；尚需包含全部修复的新版本证据 | 后续范围，本轮未代办或回写实施进度 |

## 总体结论与下一步

验收判定：**通过验收（仅本轮修复范围）**。

修复效果结论：**已解决**。上轮七项均核对，F-03 本轮关闭，其余六项维持已解决；没有以不再适用关闭的项目。正确性结论：**patch is correct**，仅覆盖声明的两文件补修 diff；未确认新增阻塞回归。完整入口的 Node 启动版本问题、补跑 oracle 的 8080 环境差异及 M6 后续证据保持显式缺口，不将补修通过登记为整期完成、已合并或已发布。

本轮修复代码验收完成。下一步按计划继续 M6：生成包含全部修复的新候选，取得四 native/Node18 CI 及同一最终 tarball 的 V-4、受影响 V-7/V-10；完整测试入口需在 8080 无冲突的隔离环境或新候选 CI 中取得退出 0。收口完成后用新候选发起代码评审：

```text
$review-code docs/plan/phoenix-ui.md
```

本轮保存独立报告与状态记录，保留 R1/R2 的原结论；没有修改代码、原计划或实施记录。
