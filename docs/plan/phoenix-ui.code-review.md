# Phoenix UI · R1 代码变更评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1（初审） | [PHOENIX-UI](phoenix-ui.md) | 累计净差异：b2289fa05f1e4f0251bc3a86e3c04f182aba156e..a5ed3cc5219f575b480e726b56be3d87be34a79d；无未提交代码 | 无 |

计划版本为观察 HEAD 28789e485046c47aae2bbef9e49d3b5461767da8，SHA-256 为 8c2207419e6f2150b7cdcb1d9020cea8fdc9182313983768e0233e421c38b17a。用户确认纳入未逐一登记的七个 Phoenix 提交，排除 a391a034d85c7fc45d40df036ea164fa771eee1b。现有 .review.md 是计划文档评审，不是前轮代码评审。

下列主定位均为 target 侧 a5ed3cc5219f575b480e726b56be3d87be34a79d；已逐文件比较 Git 目标内容与当前文件，确认一致。七项先用当前源码构建的 debug engine 复现，再用固定候选 release engine 与匹配 bundle 复现。

## 确认问题

### F-01 · [P1] 将 skills install/update 的项目分支接入事务执行器

- 定位：[main.rs:54](../../tools/phoenix-ui/crates/cli/src/main.rs#L54)；相关分流 60–61 行。
- 问题：兼容入口 phoenix-ui skills install 直接进入继承安装器，只有顶层 install/update 进入事务执行器。因此在新项目执行 skills install --project --providers=claude --yes --dry-run --bundle-root DIR --json，仍实际写入 skill、agent 和 hook，输出安装完成，且没有 active receipt、备份和事务日志。用户只预览安装就会激活文件变更；同一路径上的更新也绕开项目冲突与恢复保护。根因是本次新增的事务分流未覆盖兼容命名空间。
- 证据：[commands.rs:877](../../tools/phoenix-ui/crates/skills/src/commands.rs#L877) 的继承流程不处理 dry-run，实际复制入口在 [1143–1166 行](../../tools/phoenix-ui/crates/skills/src/commands.rs#L1143)；新增帮助在 26 行承诺项目安装事务化和 dry-run。固定候选对照：顶层 install --dry-run 退出 0 / planned / 写入 0 文件；skills install --dry-run 退出 0 / 写入 **69 文件** / 无 .phoenix-ui/install/active.json。计划 243、339–342 行要求 dry-run 只读及项目事务安装。
- 修复与验证：统一两个命名空间的项目 install/update 分流，或明确拒绝兼容入口的项目写入；保留用户级入口。断言两个命名空间 dry-run 的完整目录树不变，并覆盖用户改动冲突、实际安装回执和中断恢复。
- 置信度：高；同一候选 engine/bundle、两个隔离新目录的直接对照。

### F-02 · [P2] 在迁移时保留本机配置的 Git 忽略保护

- 定位：[installer.rs:631–633](../../tools/phoenix-ui/crates/install/src/installer.rs#L631)。
- 问题：旧项目将 .impeccable/config.local.json 保持在 Git 之外时，迁移会复制出 .phoenix-ui/config.local.json，但安装完成只追加 migrations 和 install.lock 的忽略项。旧路径的 ignore 不匹配新目录，导致浏览器本机路径、个人偏好及保留的未知配置进入可暂存集合，随后 git add . 会把本应本地保留的内容带入共享仓库。问题由本次状态目录迁移引入，不能依赖以后运行 live 才获得保护。
- 证据：配置路径映射见 [data.rs:357–358](../../tools/phoenix-ui/crates/install/src/data.rs#L357)，权限保留见 380 行，复制计划见 396–400 行。固定候选迁移退出 0 / installed；旧路径 git check-ignore -v 退出 0，新路径退出 **1**；git status --short --untracked-files=all .phoenix-ui/config.local.json 返回 **?? .phoenix-ui/config.local.json**。已有 [detect/config.rs:1058–1062](../../tools/phoenix-ui/crates/detect/src/config.rs#L1058) 与 [live/gitignore.rs:17](../../tools/phoenix-ui/crates/live/src/gitignore.rs#L17) 的保护分别在配置写入和 live-inject 时调用，安装器没有调用。计划 318、510 行要求 local 保持本地、默认忽略。
- 修复与验证：安装迁移时建立本机配置忽略项，保留已有 exclude 内容，覆盖子项目/linked worktree，并让重装能补齐保护。迁移完成、尚未调用 live 时，断言新 local 配置被 Git 忽略。
- 置信度：高；真实 Git 仓库 fixture 的迁移与 ignore/status 对照已运行。

### F-03 · [P2] 按有效启用状态判断旧插件冲突

- 定位：[installer.rs:444–446](../../tools/phoenix-ui/crates/install/src/installer.rs#L444)；状态比较见 451 行。
- 问题：个人发现把 installed_plugins.json.plugins 的安装记录当作 enabledPlugins 使用，而安装记录的对象/数组不等于 false。即使用户已在 settings 将旧插件禁用，注册表仍保留条目，安装器也会返回冲突，要求再次禁用、重载、重试；照提示操作不能解除阻塞。应阻止实际活跃旧入口，不能把已安装但已禁用的插件判为启用。
- 证据：候选 fixture 设置 enabledPlugins["impeccable@fixture-marketplace"]=false，项目没有启用覆盖，注册表保留 user-scope 条目；dry-run 退出 **3 / conflict**，冲突指向 installed_plugins.json，提示插件已启用。官方文档区分安装与启用，支持保留安装而禁用，并定义本地/项目/用户设置优先级：[安装范围](https://code.claude.com/docs/en/discover-plugins#choose-an-install-scope)、[管理已安装插件](https://code.claude.com/docs/en/discover-plugins#manage-installed-plugins)。计划 330、421 行限制的是活跃旧入口/设计 hook。
- 修复与验证：结合适用范围、启用设置和覆盖关系判断；安装注册表用于发现来源。验证禁用的 user 插件不阻止安装、真正启用仍阻止、项目或本地重新启用仍形成冲突。
- 置信度：高；公开 CLI 复现与官方禁用/范围语义一致。

### F-04 · [P2] 对迁移接受的配置格式统一运行时读取

- 定位：[context_json.rs:157–158](../../tools/phoenix-ui/crates/context/src/context_json.rs#L157)。
- 问题：迁移器接受带注释的配置并保留原字节，新增结构化 context 却严格解析整个 config，即使只有 buildPath、没有 profile 也会失败。注释配置迁移成功后，context --json 立即返回 unavailable，不能取得产品规范、方向映射和 profile 事实；继承的严格 JSON reader 也会忽略这些迁移设置。安装成功与运行时消费契约不一致。
- 证据：[data.rs:546–552](../../tools/phoenix-ui/crates/install/src/data.rs#L546) 使用接受 JSONC 的 json::parse_value；[现有 data 测试:169–188](../../tools/phoenix-ui/crates/install/tests/data.rs#L169) 要求注释配置无冲突且原字节保留。固定候选实际迁移含注释的 buildPath=code 配置退出 **0 / installed**，字节保持；随后 context --json 退出 **4 / unavailable**，错误为 key must be a string at line 2 column 2。继承 reader 的严格解析见 [context/util.rs:128–130](../../tools/phoenix-ui/crates/context/src/util.rs#L128)。
- 修复与验证：统一读取安装器接受并保留的格式；若决定拒绝某格式，应在预检明确拒绝并同步输入契约。用同一注释配置验证迁移→结构化 context→相关配置实际生效。
- 置信度：高；跨安装/运行入口重现，且仓内有明确格式测试。

### F-05 · [P2] 在只读 status 中校验草稿的已登记证据

- 定位：[status.rs:74–76](../../tools/phoenix-ui/crates/review/src/protocol/status.rs#L74)。
- 问题：草稿分支只统计 captures/packets/judges 和登记状态，没有校验证据 hash、冻结目标或评审材料。begin/capture 后修改目标期望或截图，status 仍退出 0，返回 managed / ok:true / 无缺项，并建议 judge-pack；实际写命令随即因同一漂移退出 3。恢复入口不能识别失效证据，与 managed 异常须保持只读的探测合同不一致。
- 证据：固定候选分别修改目标 24px→32px、改变已登记截图字节，status 均为 **0 / managed / ok:true**，missingCells、projectionConflicts 均空；对应 judge-pack 均退出 **3**，分别报目标期望改变、截图 hash 不匹配。写路径校验见 [protocol.rs:627–689](../../tools/phoenix-ui/crates/review/src/protocol.rs#L627)；[Phoenix 评审协议](../../skills/review-ui/references/phoenix-review.md#固定-verbs-与调用时机) 要求证据漂移时 status 返回 unknown / ok:false / 退出 4。封存结果的已有校验不能覆盖草稿。
- 修复与验证：只读校验草稿冻结快照、当前依赖、captures 和使用中的 judge/packet，保持不写入、不恢复；漂移按合同返回 unknown。覆盖修改、删除目标/截图/评审原文及未变草稿的反例。
- 置信度：高；同一失效输入在查询和写命令上的直接对照。后续写入已有保护，本项不声称失效截图被封存为通过。

### F-06 · [P2] 按限定单元计算采集完整性

- 定位：[finalize.rs:967–971](../../tools/phoenix-ui/crates/review/src/protocol/finalize.rs#L967)。
- 问题：FrozenMatrix 保留全部单元格子，finalize 却对整个 cells 集合检查图片完整性，未过滤 round.scope。目标含 U1/U2、本轮只评 U1 时，即使 U1 完整采集并独立评分 7 分、有可修复 P2，仍因范围外 U2 未采而判需补证；正确判定应是范围内需要修改。限定轮本来只禁止宣称全局通过，这里把范围外采集变成当前缺证，覆盖与下一步随之扩大。
- 证据：候选公开 CLI 序列 begin --scope U1→U1 capture→U1 judge→finalize：**full=false，U1 score="7"，evidenceComplete=false，verdict="需补证"**；missingCells 列出了 U2。矩阵保留全量格子见 [matrix.rs:99–192](../../tools/phoenix-ui/crates/review/src/matrix.rs#L99)，selected_units 才应用 scope（252–262 行）。[限定范围合同](../../skills/review-ui/references/report-contract.md#验收判定) 仅在范围内达标时要求需补证并进入全量轮，当前反例未达标。[M6 记录](phoenix-ui.records/M6.md#原r6补评与限定轮封存v5限定单元) 已观察范围外格子被报缺采，但其 9.4 分样本未暴露低分轮误判。
- 修复与验证：采集覆盖、missingCells、finalize 完整性和报告补证动作统一按选定单元。验证 U1=7→需要修改、U1 达标→需补证并明确新全量轮、U1 自身缺采→需补证；均不把 U2 写成当前范围内缺采。
- 置信度：高；完整 CLI 序列重现，影响验收判定与执行范围。

### F-07 · [P2] 按单元归属隔离目标附件缺项

- 定位：[packet.rs:359–360](../../tools/phoenix-ui/crates/review/src/protocol/packet.rs#L359)。
- 问题：judge-pack 对每单元都要求整个目标包 missing 为空，任一单元的目标附件缺失会使所有单元不可评审。U1 有已接受 DESIGN.md 和完整当前截图、U2 独有目标文件还未提供时，即使 --units U1，也返回无完整单元，无法保存 U1 的独立评分与可先处理差距，违背缺证单元标缺证、其他单元继续评审的承诺。
- 证据：候选 fixture 来源列为 U1→存在的 ../DESIGN.md、U2→缺失 missing.md，U1 规范和截图完整；begin 只标 Unreadable target attachment: docs/missing.md，judge-pack --units U1 却退出 **2 / No selected unit has complete evidence**。附件缺项全局汇总见 [target.rs:154–158](../../tools/phoenix-ui/crates/review/src/target.rs#L154)；计划 387、473 行要求分单元找来源、缺证时保留已评部分；[review-ui 步骤 1.4](../../skills/review-ui/SKILL.md#1-锚定对象与目标包) 要求其他单元照常评审。
- 修复与验证：记录缺项的单元/共享规范依赖，只阻止依赖它的单元。验证 U2 独有附件缺失时能导出并评分 U1，U2 明确未评，整轮不能通过；共享必需来源缺失仍正确阻断。
- 置信度：高；来源归属明确、完整截图已登记，CLI 拒绝可稳定重现。

## 待核实项与验证限制

- 完整 npm run test:phoenix 本轮未跑完：完成原生构建、Node 安装 5 项与 29 个 Rust 组后，主动终止剩余 workspace 执行，转为问题相关检查。最终退出 1 来自本次终止，不作为产品测试失败；不能声明全量回归通过。修复后按仓库约定完整重跑。
- 没有重跑四平台 CI、Chrome/Safari 界面走查、真实 Claude/Codex 会话或公开 R2/npm 发布。M3–M6 原证据保持原版本、原范围，作者已完成的记录不替代本轮判断。
- 人工深入检查集中于关键改动和行为链。导入原字节、生成 blob、字体/图片及历史 oracle 以来源/摘要和现有校验取证；不声明每份导入源码逐行审完、每份图像逐像素验收。未证实推测未进入 findings。

## 覆盖与验证

变更意图：承接固定 Impeccable 能力，形成自主 Phoenix 运行时、事务安装迁移、设计上下文与可恢复 UI 验收协议，交付可验证的发布候选。

- 日期：2026-10-13，Asia/Shanghai；仓根 /Users/rockie/Documents/GitHub/xgent/skills。
- 原输入：$review-code docs/plan/phoenix-ui.md；开发计划模式、初审，M1–M6。
- base：b2289fa05f1e4f0251bc3a86e3c04f182aba156e；target：a5ed3cc5219f575b480e726b56be3d87be34a79d / tree 3b86dd19d46c11c09a21c63d3b289a9cee831cd2。
- 观察/当前测试 HEAD：28789e485046c47aae2bbef9e49d3b5461767da8 / tree d69a97eaa8de8ff5b44945107f2056f894983645；分支 phoenix-ui/m2-verification-20261009。开始时 worktree/index clean，无 untracked 输入。关键运行时路径与 target 字节一致；报告和本轮台账新文件不纳入输入。
- 累计范围含下表 34 个实现/候选提交；区间中 27 个只改计划/记录/台账的提交不作为代码 findings 对象。用户确认纳入 0a5d229、0644b7a、bae9587、d3e0401、54f6724、ed00667、6353351。排除 a391a034d85c7fc45d40df036ea164fa771eee1b 的独立 agi-mode 文档改动；完成/状态回写 7901ce186eedac4407425c1a26ae6a12c8ed35dc、28789e485046c47aae2bbef9e49d3b5461767da8 作为证据，不增加实现 diff。
- 已盘点累计文件清单。深入检查入口分流、bootstrap/receipt/bundle、安装 ownership/事务/迁移格式/个人发现、context/profile/accepted brief、目标指纹/矩阵、采集/packet/计分/跨轮历史/封存/投影/处置、构建/CI/发布交接、辅助 skills 调用边界。README/agi-mode 仅核对 Phoenix 引入的差异，排除用户指定提交。
- UPSTREAM.json 的 2091 项在固定上游 checkout 上全部 hash 匹配；目标原路径中 1889 项字节不变、145 项改变、57 项迁移布局。哈希只证明来源与变化边界，不代替业务验证。
- 本次单 reviewer，由主 agent 完成；未派独立 reviewer，未自动修复。

### 实际验证

| 版本 / 命令 | 实际结果 | 覆盖 |
| --- | --- | --- |
| 观察 HEAD；npm test | 退出 0；16 个 Node 测试文件通过；Python 22 / 24 / 2 / 13 项通过 | 轻量回归与元数据 |
| 观察 HEAD；npm run test:phoenix | 原生构建、Node 安装 5 项通过；已完成 29 个 Rust 组、627 项通过、0 失败；其余主动终止，最终退出 1 | 部分运行时，未完成组不计通过 |
| 观察 HEAD；mise exec --no-deps -- mbx test -p phoenix-ui-review --locked --offline -- --test-threads=1 | 退出 0；226 项通过 | 现有 review 测试，未捕获 F-05–F-07 反例 |
| 观察 HEAD；mise exec --no-deps -- mbx test -p impeccable-context --test context_json --locked --offline -- --test-threads=1 | 退出 0；27 项通过 | 现有结构化 context 测试 |
| 观察 HEAD；mise exec --no-deps -- mbx test -p phoenix-ui-install --test data config_and_local_copy_preserve_bytes_unknown_fields_and_use_retain_rollback --locked --offline -- --test-threads=1 | 退出 0；1 项通过 | 迁移格式与原字节契约 |
| 固定 target release engine + 匹配 bundle；七项隔离 fixture | F-01–F-07 全部重现 | 目标制品行为；PNG/评分是协议测试输入，不是产品 UI 验收 |

mbx 检查经仓库 preflight/buildEnvironment，固定 Rust 1.99.0、mbx 1.21.1，采用 macOS loopback 测试网络策略。当前 checkout 测试按实际 HEAD 记录，不冒充历史提交的全量结果；目标行为另由未重编的 release 制品确认。

候选 release 位于 local/phoenix-ui/wt-a5ed3cc/local/phoenix-ui/b-final8/publication-rehearsal/release/。已核对 VERSION/交接记录的 sourceCommit/sourceTree、实际 binary size/SHA 与匹配 bundle：darwin-arm64 **18250976 字节**，binary SHA-256 071306081e366f2fc8f132a605ad6f5a5248cc58930fc4748c15d84698506db1；manifest ebd5ed4adb2fd357920fced366c467dc9df37d2f9c649dee76d965bd70fe72e3；bundle 95bca755477c6864e11eab1104d2147d0d4c80dca24ec15a058c6af4d96e7c30。未重打包、重编或发布该制品。

本机临时证据目录为 /var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-review-mc1hlbyl/。目标复现命令：

    python3 /var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-review-mc1hlbyl/target-verification/verify.py

脚本使用一次性 fixture，重跑须用新临时目录；输入/输出保存在 target-verification/results.json，来源在 provenance.json。日志为 /tmp/phoenix-code-review-{npm,runtime,focused}-test.log。这些临时证据未随 Git 共享；另一机器可按各 finding 条件重建 fixture。

## 计划符合性

| 承诺 | 实现与检查证据 | 本轮判断 |
| --- | --- | --- |
| R-1/R-2/C-1；M1–M3 自主运行时、工具链与承接 | 来源摘要、bootstrap、构建/CI/交接代码、原生构建与部分回归 | 关键链已核对；全量及实会话限制如上 |
| R-6/NFR-1/NFR-4；M3/M6 预检、事务迁移、local 数据保护 | 安装器、Node 安装 5 项、候选反例 | 存在 F-01–F-04 |
| R-3/R-5；M4 规范优先、context 与已接受方向 | context/profile/brief、辅助 skills、27 项 | 主链已核对；迁移消费存在 F-04 |
| R-4/C-4；M5/M6 缺证、范围隔离、独立材料与恢复 | review、四份合同、规则表、226 项、目标 CLI 反例 | 存在 F-05–F-07 |
| R-7/C-3/C-5；M6 命名、渠道、许可、发布交接 | package/launcher、构建/发布器、现有 publication 轻量测试、候选 manifest/bundle | 未发现确认的新缺陷；公开 URL/发布未验证 |
| 真实浏览器、宿主和平台 | M3–M6 原记录 | 本轮仅核对原范围，未重做真实会话/视觉验收；Windows 为明确后续范围 |

## 总体结论与修复顺序

验收判定：**需要修改**。

正确性结论：**patch is incorrect**。确认 **1 项 P1、6 项 P2**。事务安装的兼容入口可在 dry-run 下写项目；local 保护、迁移消费、插件冲突以及 managed 证据/范围判断均有直接反例。现有测试通过不能关闭这些问题。

1. 先处理 F-01，统一项目写入边界；随后 F-02，保护本机数据。
2. 修复 F-03/F-04 的安装/运行契约，再处理 F-05–F-07 的状态读取与单元覆盖。
3. 各反例先红后绿，完成 npm test、完整 npm run test:phoenix，按受影响范围复跑平台/真实会话，再发起代码复审。原完成记录和历史验收不改写为本轮通过。

下一步：

    $apply-code-review docs/plan/phoenix-ui.code-review.md

修复后：

    $review-code docs/plan/phoenix-ui.md

以本报告为 R1 逐项复审；本次没有启动修复或 UI 验收。

## 纳入提交（完整 SHA）

| SHA | 提交说明 |
| --- | --- |
| 624147968ecc8098e2c72b1ff69ba5db11053a91 | feat(phoenix-ui): prepare M1/M2 runtime verification candidate |
| 19630c25aa4eaa25c88ed59d349f733bbe4a9daf | fix(phoenix-ui): prepare pinned dependencies on clean CI runners |
| de3a85739ddc04c346c123e9b36ff86b177ec0b7 | fix(phoenix-ui): audit off-host packages and validate Windows cargo shims |
| da0f780478a762a9435f22e173f5dcf7ef8636c5 | fix(phoenix-ui): address platform CI failures |
| 3eec43f153c1e49b28deab0ed18f457e4f760d19 | fix(phoenix-ui): bind ARM browser and repair CI portability |
| 544f76e4cea60aa953f268d0f2f707025d890131 | fix(phoenix-ui): isolate platform test environments and reconnect lease |
| 5090fb5a104097592236474e17f10c7b753cb867 | fix(phoenix-ui): select Git Bash and allow the pinned Chromium sandbox in CI |
| a07cb752f29d9db46e12460fb95e3bb5ad99a23b | fix(phoenix-ui): resolve Git Bash before spawning Python plan checks |
| 1cef4e883bfb56ecede957feaaf80ea23b94685c | test(phoenix-ui): retain raw Bash output for Windows diagnosis |
| d5b6753bc3d4f4baa6fa6578e32f8ad67838085f | fix(dev-plan): match complete snapshot colons in byte locales |
| b75f1513b19e2c25ca672e4a49b0673b7c81c012 | test(phoenix-ui): invoke Windows hook contract with raw cmd syntax |
| 889fccbf084d712aca1b11e4805b731bb99e04a1 | test(phoenix-ui): stabilize collision and file URL fixtures |
| f189b65f80cb771bfbdc9a1d9a258ceb9567cc94 | fix(phoenix-ui): retain exact Windows oracle path semantics |
| 22d9e507d021d9dceba0b059c339c5d3a0fbd668 | fix(phoenix-ui): normalize Windows lint diagnostic paths |
| 16543bfbf9c02b4d08fd57e6dec424e987fb3da8 | test(phoenix-ui): retain complete native oracle evidence |
| 67e2817acf760de19015149a21fd291e4648cff5 | fix(phoenix-ui): retry owned staged executable file locks |
| d107adf24b567c308647cab660d69a8fb9b6dada | fix(phoenix-ui): replay the oracle faithfully on native Windows |
| 36885072fc6be76d1d0880aea66635d2dd9ba318 | test(phoenix-ui): pin oracle metadata tests to their stated platform |
| 142aba6a56b8c88776b7b2de6093c3af72f03f09 | fix(phoenix-ui): close the remaining Windows oracle differences |
| 3e236198e72a038514cdaf3c8321a186fbad221e | feat(phoenix-ui): prepare transactional A-stage candidate |
| 9d97862c21dc892a98a5d80200b4092f36a40029 | test(phoenix-ui): allow bounded ARM migration runs |
| 17d188aed2dc2188c1ad823b70b5b5dbbe4d3a97 | ci(phoenix-ui): allow full cold native verification to finish |
| a44658dfdfdb1068fba0a123ef6751ca36fb2886 | fix: refresh Phoenix A license inventory and acceptance evidence |
| 1ef9e37f4fd86f17a59b649a0eec67f546d6d115 | feat(phoenix-ui): accept phase A runtime and migration |
| 0a5d229a0c740cf9ab759e4cc675387c53b8c577 | feat(phoenix-ui): integrate B context and review for verification |
| 0644b7ae6c9a997533f2d515a167840ad2812817 | fix(phoenix-ui): add review discovery and absent-prefix status |
| bae95870c60d3a958f163539e2128ce0540b88d1 | fix(phoenix-ui): project managed review reports per the report contract |
| d3e0401df4978ee00d0c06778917b685c67f0868 | fix(phoenix-ui): take previous review material from the last judged round |
| 54f672456e46eeb76322a476a053dbe2b61ae846 | fix(phoenix-ui): let an unused stale packet not block new judge packets |
| ed0066738989436dc95c2ac6f1465066cdd95595 | ci(phoenix-ui): give native jobs 180 minutes |
| 6353351aa40655c594173f282977ec4203081813 | perf(phoenix-ui): validate each sealed round and packet once per command |
| 526c692556857d69ceb1934b7415f83145f63723 | fix(phoenix-ui): preserve hook results after input pipe closes |
| 17d5716b6427026cb5df75e63889e06e12fa8c8f | fix(phoenix-ui): bound frozen-source dependencies across review history |
| a5ed3cc5219f575b480e726b56be3d87be34a79d | fix(phoenix-ui): publish answers before acknowledging requests |

