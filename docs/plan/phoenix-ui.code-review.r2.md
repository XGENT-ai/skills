# Phoenix UI · R2 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2（修复效果复审） | [PHOENIX-UI](phoenix-ui.md) | R1 target `a5ed3cc5219f575b480e726b56be3d87be34a79d` → 当前 `183bc42cd0b24f7ebf97bd20b2c25e21528101aa`，排除已在 R1 排除的 agi-mode 提交；实现增量为 `28789e485046c47aae2bbef9e49d3b5461767da8..43a27a3c0a7f87d8b30e7a27186691a1c58af582`；无未提交代码 | [R1](phoenix-ui.code-review.md) |

本轮复核 R1 的全部七项问题、待核实项及修复直接影响的行为，不是对 M1–M6 原累计变更的完整重审。R1 报告 SHA-256 为 `87d4a5161f95391baddae73b78611db1833740de640ad126ee4944d75c1615f7`，其评审对象 ID 为 `eb880cb9-3d5a-4d35-997b-f17d5fc2d5d2`。本轮独立对象 ID：`3833fbb3-87bd-4e75-8a81-e2e3d7622a90`。

修复提交为 `43a27a3c0a7f87d8b30e7a27186691a1c58af582`，tree `1c0a2d777e077c0a0c3784a989cc31860a74c166`。当前 HEAD 只在其后追加计划 SHA 登记，所审运行时、测试及元数据与修复提交字节一致。处置来源：[M3 / F-01–F-04](phoenix-ui.records/M3.md#r1代码评审修复f-01f-04)、[M5 / F-05–F-07](phoenix-ui.records/M5.md#r1代码评审修复f-05f-07)。处置记录的“已修复并验证”作为作者自检，以下结果为本轮独立核查。

## 上轮问题复审

下表来源均为上述 R1 报告及版本；修后核查版本均为当前 HEAD（运行时等于 `43a27a3c0a7f`）。

| 来源 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01 | P1；`skills install/update` 项目分支绕过事务，dry-run 写入 | M3 R-1；兼容项目入口接入同一事务安装器 | **已解决** | [路由 main.rs:55](../../tools/phoenix-ui/crates/cli/src/main.rs#L55)、[兼容入口回归:86](../../tools/phoenix-ui/crates/cli/tests/skills_install.rs#L86)：install/update dry-run 树不变、实装 receipt/journal、重复 update、用户修改冲突；运行通过 | 原问题无需继续修复 |
| R1 / F-02 | P2；迁入 local 配置丢失 Git 忽略 | M3 R-2；新装和 Unchanged 重装补齐 exclude 块 | **已解决** | [installer.rs:633](../../tools/phoenix-ui/crates/install/src/installer.rs#L633)、[重装:1060](../../tools/phoenix-ui/crates/install/src/installer.rs#L1060)、[真实 Git 回归:840](../../tools/phoenix-ui/crates/install/tests/installer.rs#L840)：子项目、linked worktree 和旧块补齐通过 | 原问题无需继续修复 |
| R1 / F-03 | P2；把安装登记当有效启用，禁用插件仍阻断 | M3 R-3；改查 user/project/local 设置并覆盖合并 | **部分解决** | [installer.rs:438–441](../../tools/phoenix-ui/crates/install/src/installer.rs#L438)、[现有矩阵:909](../../tools/phoenix-ui/crates/install/tests/installer.rs#L909)通过；但真实 Git 子目录/linked worktree 的 local 来源遗漏，直接对照 Claude 2.1.296 仍误阻断，反方向也允许启用旧插件时实装 Phoenix，详见下节 | 读取实际 Git 根/主 worktree 的 local 设置并补反例 |
| R1 / F-04 | P2；迁移接受 JSONC，运行时严格 JSON 不能消费 | M3 R-4；采用 R1 允许的预检拒绝方案 | **已解决** | [strict_json:548](../../tools/phoenix-ui/crates/install/src/data.rs#L548)、[拒绝回归:235](../../tools/phoenix-ui/crates/install/tests/data.rs#L235)：config、local、design 注释输入均为 InvalidJson、无迁入写入、原文件保留；data 32 项通过；与 [context reader:157](../../tools/phoenix-ui/crates/context/src/context_json.rs#L157)一致 | 关闭的是“成功迁入不可消费文件”；没有宣称运行时新增 JSONC 支持 |
| R1 / F-05 | P2；草稿 status 不识别已登记证据漂移 | M5 S-1；草稿按写路径校验 sources 与已使用 judge/packet | **已解决** | [status.rs:76–78](../../tools/phoenix-ui/crates/review/src/protocol/status.rs#L76)、[CLI 回归:2526](../../tools/phoenix-ui/crates/review/tests/cli.rs#L2526)：目标改变及五类登记文件修改/删除均 unknown/退出 4、目录不变；恢复后 managed；运行通过 | 原问题无需继续修复 |
| R1 / F-06 | P2；限定轮把范围外格子算缺采 | M5 S-2；begin、missingCells、finalize 和报告用 selected_cells | **已解决** | [finalize.rs:967](../../tools/phoenix-ui/crates/review/src/protocol/finalize.rs#L967)、[CLI 回归:2585](../../tools/phoenix-ui/crates/review/tests/cli.rs#L2585)：U1=7 → 需要修改、完整采集且报告 1/1；U1 自身缺采 → 需补证；既有限定达标反例保持需补证；运行通过 | 不把限定达标扩大为全局通过 |
| R1 / F-07 | P2；一单元独有目标附件缺项阻断全部单元 | M5 S-3；按单元表行记录缺项归属，共享缺项仍共享 | **已解决** | [packet.rs:359–360](../../tools/phoenix-ui/crates/review/src/protocol/packet.rs#L359)、[CLI 回归:2635](../../tools/phoenix-ui/crates/review/tests/cli.rs#L2635)：U2 独有缺项时可导出/评分 U1，U2 未评、整轮需补证；共享目标缺图仍阻断；运行通过 | 原问题无需继续修复 |

本轮 **6 项已解决、1 项部分解决**；没有以不再适用关闭的项目，也没有跳过任何选定旧问题。

## 确认问题

### F-03 · [P2] 读取 Git 根及主 worktree 的实际 local 插件设置（上轮部分解决）

- 来源：R1 / F-03，沿用原编号；以下误阻断与新激活的漏挡属于同一有效设置来源根因，不重复计数。
- 定位：[installer.rs:440–441](../../tools/phoenix-ui/crates/install/src/installer.rs#L440)，target 侧 `183bc42cd0b24f7ebf97bd20b2c25e21528101aa`；这两行在修复 diff 内。
- 问题：安装器只读取安装目标目录中的 `.claude/settings.local.json`，但当前 Claude Code 在 Git 子目录使用仓库根的 local 设置，在 linked worktree 使用主检出根的 local 设置，而且旧 cwd 文件与根文件共存时根值优先。因此用户在实际生效文件禁用旧插件后，子目录/worktree 的安装仍返回冲突，按提示禁用和重载也不能解除；反方向，本次去掉注册表阻断、增加 cwd local 覆盖后，根文件仍启用旧插件而 cwd 文件为 false 的状态会通过预检并激活 Phoenix。隔离实装已经返回 installed、生成 active receipt，而同目录的 `claude plugin list --json` 仍返回旧插件 enabled=true 且无加载错误，破坏计划要求的有效禁用和防双入口边界。
- 证据：[现有回归:909](../../tools/phoenix-ui/crates/install/tests/installer.rs#L909)只在没有 Git 布局的 fixture 中比较三份设置，未覆盖实际 local 来源；[计划 §3.1:330](phoenix-ui.md#31-安装与旧项目迁移)要求阻止活跃旧主入口。Claude [local 文件位置规则](https://code.claude.com/docs/en/settings#where-claude-code-keeps-the-local-file-in-a-git-repository)和[插件设置覆盖规则](https://code.claude.com/docs/en/plugins/loading#find-where-a-plugin-is-enabled)与下列本机 2.1.296 对照一致；版本由实际 `claude --version` 读取。
- 修复与验证：按受支持宿主的实际设置来源解析 user、shared project 与 local，保留根文件对旧 cwd local 文件的优先级，并处理 linked worktree 的主检出根；安装注册表继续只作发现依据。用真实 git init / worktree fixture 验证根禁用允许安装、根启用阻止安装、cwd false 不能覆盖根 true，且已禁用注册表保留和普通根目录安装的现有反例继续通过。
- 置信度：高；六个隔离布局/设置对照、修前 release 与修后 debug 的比较、实际实装和当前宿主只读查询均一致。

### F-03 的直接对照

三份设置只列本 fixture 的 `enabledPlugins["impeccable@fixture-marketplace"]`；注册表始终保留有效 user 安装，插件目录和本地 marketplace 均存在。没有访问真实账户或启动模型会话。

| 安装/Claude cwd | user | 实际根 local | cwd local | Claude enabled | 修后 Phoenix dry-run | 修前 a5ed3cc dry-run |
| --- | --- | --- | --- | --- | --- | --- |
| 普通仓根（对照） | true | false | 同根 | false | 0 / planned | 3 / conflict |
| 仓库子目录 | true | false | 无 | false | **3 / conflict** | 3 / conflict |
| 仓库子目录 | false | true | 无 | true | **0 / planned** | 3 / conflict |
| 仓库子目录 | true | true | false | true | **0 / planned** | 3 / conflict |
| linked worktree 根 | true | 主检出 false | 无 | false | **3 / conflict** | 3 / conflict |
| linked worktree 根 | true | 主检出 true | false | true | **0 / planned** | 3 / conflict |

子目录第四行再去掉 dry-run 实装：退出 0 / installed / active receipt 存在；实装后 Claude 查询仍 enabled=true / errors=[]。这是安装激活门的直接反例；没有声称观察过旧/新 hook 在真实模型会话中同时执行。

## 本轮新增问题检查

未确认其他独立根因的新增 finding。F-03 中“实际启用却通过安装”由本次解除注册表阻断及错误 local 覆盖激活，已按同根因并入旧项；修复增量不能判正确。F-01/F-02/F-04–F-07 的直接相关回归未发现其他确认缺陷。

## 待核实项与验证限制

- `npm run test:phoenix` 在 WASM 阶段退出 1：入口实际为 Node 19.3.0，无法加载验证 WASM 的 stringref；该次入口尚未进入 oracle。根 mise.toml 与 build-tools.lock.json 固定 Node 25.2.1。其前已完成原生构建、Node 安装 5 项、Rust 103 组 1269 项通过 / 0 失败 / 9 ignored，以及 lint 无新增诊断。
- 随后在固定 Node 25.2.1 和相同 buildEnvironment 下补跑剩余阶段：WASM 8474 向量，golden/native/parity 均 0 失败；oracle 全部 860 例执行，859 通过 / 1 失败 / 0 error / 0 missing / 0 skip。唯一失败是 detect-fixture-text-framework-vite 的端口占用提示：本机 Docker Desktop PID 62891 监听 *:8080；固定 a5ed3cc release 单例也同样失败，且 detect 源码不在本轮 diff。它是可复现的环境差异，不归为修复引入的代码失败；仍未取得完整入口单次退出 0。干净环境或新候选 CI 应补齐该证据。

- 当前版本没有重跑 macOS/Linux 四 native CI、Node18 最终包消费、Chrome/Safari UI 走查或真实 Claude/Codex 模型会话。R1 的这些缺口继续保留；本轮的 Claude `plugin list` 只是隔离配置/插件状态查询，不能代替宿主会话或 UI 验收。
- 计划 M6 已因运行时修复重开，原 a5ed3cc/CI33/tarball 的证据不能代表 `43a27a3c0a7f`。新候选 CI、同一最终 tarball 的 V-4、受影响 V-7 与 V-10 属于计划后续收口；本轮不把后续工作缺失报成额外代码缺陷。
- Windows 是用户明确延期范围，未计入本期验收。没有公开发布、推送、合并或修改代码/计划。

## 覆盖与验证

变更意图：修复 R1 发现的事务入口、迁移配置保护/格式、有效插件状态，以及 UI review 的草稿证据和单元范围契约。用户明确请求本计划代码评审；该意图由登记修复和 diff 确认。

- 日期：2026-10-13，Asia/Shanghai；仓根 `/Users/rockie/Documents/GitHub/xgent/skills`；分支 `phoenix-ui/m2-verification-20261009`。
- 原输入：`$review-code docs/plan/phoenix-ui.md`。开发计划模式，按已有最大轮次 R1 启用修复效果复审；计划完成记录的 26 个 SHA 均解析成功且为 HEAD 祖先。
- 上轮 base/target：`b2289fa05f1e4f0251bc3a86e3c04f182aba156e..a5ed3cc5219f575b480e726b56be3d87be34a79d`。本轮登记修复：`43a27a3c0a7f87d8b30e7a27186691a1c58af582`；当前核查 HEAD：`183bc42cd0b24f7ebf97bd20b2c25e21528101aa` / tree `e945c6da568a8a8941b3711f74db817c843dcc60`。
- 实际 diff：`git diff --no-ext-diff --no-textconv 28789e485046c47aae2bbef9e49d3b5461767da8 43a27a3c0a7f87d8b30e7a27186691a1c58af582 -- tools/phoenix-ui`，18 个运行时/测试/能力账本文件。该范围内没有未纳入实现提交；它与 R1 target→当前在这些路径的净差异相同。
- 排除：`a391a034d85c7fc45d40df036ea164fa771eee1b` 沿 R1 的明确排除继续排除；`7901ce186eedac4407425c1a26ae6a12c8ed35dc`、`28789e485046c47aae2bbef9e49d3b5461767da8`、`183bc42cd0b24f7ebf97bd20b2c25e21528101aa` 是进度/记录提交，不增加运行时 diff。修复提交中的报告、计划/记录、台账和经验条目作为范围与处置证据，不作为新代码 finding 对象。
- 起始 worktree/index clean、无 untracked 输入；index tree 等于 HEAD tree。起始全部存在 tracked 文件的内容指纹为 `d8418f651f92c4c71b65fb4c80ba17c1651f9beb181ae0109fcb82edba7453fc`：按 git ls-files 枚举，逐文件 SHA-256，再对按路径排序、紧凑 JSON 的 `{path:hash}` 作 SHA-256。完整集合与 SHA 在本机临时 snapshot.json 中。写入报告/台账前核对原集合无变化；最终排除本轮明确保存的报告/台账，再核验其余输入。
- 附带计划当前 SHA-256：`15ba26597a85751d2259a2839cd5018920024264db0a83523c717fd81803b883`。代码与契约支持证据均读取当前版本；修前比较使用 R1 的固定 release，没有拿当前 checkout 测试冒充历史验证。
- 18 文件覆盖：能力与兼容账本 2；CLI 路由/回归 2；install data/installer 及回归 4；review cli/matrix/protocol/target、finalize/packet/report/status 及 CLI 回归 9；oracle adapter 来源摘要 1。另追踪安装参数、ownership/恢复路径、Git commondir、严格 JSON readers、目标冻结、捕获验证、judge/packet、封存结果与三个相关 UI 合同入口。
- 主 agent 单 reviewer；没有派独立子 agent，未以作者自检替代本轮复核。未完整重审原承接运行时或其他计划。

### 实际验证

| 命令 / 版本 | 实际结果 | 范围 |
| --- | --- | --- |
| `npm test`；当前 HEAD | 退出 0；16 个 Node 文件通过；Python 22 / 24 / 2 / 13 项 OK | 轻量回归、元数据与规则合同 |
| `npm run test:phoenix`；当前 HEAD | 退出 1，止于 Node 19.3.0 的 WASM CompileError；Node 安装 5/5、Rust 103 组 1269 passed / 0 failed / 9 ignored、lint 无新增诊断已完成 | 原生构建、Node→Rust 安装、Rust workspace/browser/CLI、lint 与 WASM 首次检查；oracle 由下行补跑 |
| `mise exec --no-deps -- node --input-type=module -`，以 preflight/buildEnvironment 提供环境并调用 WASM/oracle checker；当前 HEAD、Node 25.2.1 | WASM 退出 0，8474 向量、golden/native/parity 0 失败；oracle 退出 1，859/860 通过（475 adaptations），唯一端口环境差异 | 对原入口尚未完成阶段补跑；保留初次失败，不冒充单次全量绿色 |
| 临时 `verify-critical.py`；当前 debug engine | 退出 0；三个 dry-run 树不变、JSONC 退出 3 且无状态写入、目标/图片漂移 status=unknown 且只读、限定 U1=7 封存为需要修改/证据完整、缺 U2 附件仍导出 U1 | 从 R1 原始反例重建隔离目录，验证 F-01/F-04/F-05/F-06/F-07；结构 fixture 不是 UI 验收 |
| 临时 `verify-plugin-sources.py`；当前 debug、固定 a5ed3cc release、Claude 2.1.296 | 退出 0；六种布局/设置对照及实装反例；Claude 查询均退出 0 且无插件加载错误 | F-03 的剩余有效设置问题及修复激活的漏挡 |
| `mise exec --no-deps -- node scripts/check-phoenix-oracle.mjs --bin <a5ed3cc固定engine> --prefix detect-fixture-text-framework-vite --include-actual --output <临时JSON>` | 退出 1；1/860 的该例失败，stderr 与当前版本同为 Port 8080 is in use by another service | 与当前唯一 oracle 失败作修前对照；基线 binary SHA 与 R1 manifest 一致 |

本轮原生检查使用仓库 preflight/buildEnvironment、固定 Rust 1.99.0 / mbx 1.21.1 和 macOS loopback 测试网络策略。浏览器测试按仓库入口自己的范围执行；这里的结构 fixture PNG 和分数不当作产品 UI 实测。

当前 debug engine 由本轮 test:phoenix 构建后保留，SHA-256 `70c8480d25a8dd30cc42db1414fa3c5a295ce41be644f63660e88127833160a5`。修前 release 的 VERSION 记录 sourceCommit `a5ed3cc5219f575b480e726b56be3d87be34a79d`、sourceTree `3b86dd19d46c11c09a21c63d3b289a9cee831cd2`，实际 binary SHA-256 `071306081e366f2fc8f132a605ad6f5a5248cc58930fc4748c15d84698506db1` 与 manifest 一致；没有重编或发布它。

原始日志：`/tmp/phoenix-r2-npm-test.log`、`/tmp/phoenix-r2-runtime-test.log`、`/tmp/phoenix-r2-plugins.log`、`/tmp/phoenix-r2-critical.log`、`/tmp/phoenix-r2-remaining.log`、`/tmp/phoenix-r2-oracle-baseline.log`。临时证据目录：`/var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-r2-review-bdb677bg/`，含 `snapshot.json`、`verify-plugin-sources.py`、`verify-critical.py`、保留 engine，以及 `plugin-replay/results.json`、`critical-replay/results.json`、`oracle-current.json`、`oracle-baseline-vite.json`、`remaining-results.json` 和各次 stdout。临时材料不随 Git 共享；本报告保存了条件和观察结果，其他机器须重建隔离 fixture，不能把临时 hash 当作已同步证据。

复现脚本已实际运行；再次运行时为 `--output` 指定新的未占用目录：

```text
python3 <临时证据目录>/verify-plugin-sources.py --engine <当前engine> --baseline-engine <a5ed3cc固定engine> --bundle-root vendor/phoenix-ui/bundle --output <新的隔离目录>
```

## 计划符合性

| 承诺 / 本期范围 | 修后实现与本轮证据 | 结论 |
| --- | --- | --- |
| M3 / R-6 / V-4；事务项目入口、local 数据保护、迁移消费 | F-01、F-02、F-04 的路由/真实 Git/拒绝格式回归，Node 安装 5 项 | 已核实原三项问题解决 |
| M3 / R-6；个人及插件活跃入口发现 | 普通三份设置矩阵通过，但实际 Git local 来源与宿主不一致 | F-03 部分解决；还不能满足有效禁用及防双入口 |
| M5 / R-4 / V-7/V-8；只读状态、限定范围和缺证单元隔离 | F-05–F-07 的公开 CLI 反例与 review 回归 | 原问题已解决；限定达标仍需全量轮，共享目标缺证仍阻断 |
| M6；修后新候选与最终包收口 | [恢复快照](phoenix-ui.md#恢复快照)明确重新验证，原 CI33/最终 tarball 不覆盖修复 | 后续范围；本轮未代办或改进度 |

F-04 关闭依据是明确拒绝不可消费格式，并保持旧文件和失败只读；F-07 关闭依据是可评单元能够继续评审，没有把目标缺项轮判成通过。后者与 [review-ui 单元隔离](../../skills/review-ui/SKILL.md#1-锚定对象与目标包)及[报告合同缺目标/限定判定](../../skills/review-ui/references/report-contract.md#验收判定)一致。

## 总体结论与修复顺序

验收判定：**需要修改**。

修复效果结论：**未完全解决**。F-01/F-02/F-04–F-07 共六项已解决，F-03 部分解决，仍为 P2。正确性结论：**patch is incorrect**，只覆盖声明的修复 diff；错误 local 来源已经使修复后的安装预检漏挡并实际激活 Phoenix。没有新增独立问题不代表旧问题全部关闭。

1. 补齐 F-03 的实际 Git 根/主 worktree local 来源和覆盖次序，加入上表正常根、子目录及 linked worktree 反例；保留已解决的注册表 false、项目/local 重启用等既有测试。
2. 完成相关回归和仓库要求的轻量/运行时检查，再以本计划发起 R3 修复效果复审。在固定 Node 25.2.1、8080 未被其他服务占用的隔离环境或新候选 CI 中取得完整 test:phoenix 退出 0；本轮未停止用户已有 Docker 服务。
3. 代码复审通过后按计划 M6 生成包含全部修复的新候选，取得本期平台 CI 与同一最终 tarball 的受影响验收证据；历史证据不改写为修后通过。

下一步：

```text
$apply-code-review docs/plan/phoenix-ui.code-review.r2.md
```

修复后：

```text
$review-code docs/plan/phoenix-ui.md
```

本轮没有启动修复、UI 验收、合并或发布。报告与状态登记另存，保留 R1 原报告和历史事件。
