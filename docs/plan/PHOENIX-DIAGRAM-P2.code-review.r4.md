# Phoenix Diagram P2 · R4 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R4，开发计划模式，修复效果复审 | [PHOENIX-DIAGRAM-P2](PHOENIX-DIAGRAM-P2.md) | 修复增量 `62b8a2affd29e78a9ba9400b7dec6daeca26721a..5d6a511ce61e82cf547f4f5372781da47d01622b`；核对 R3 的全部问题及待核实项 | [R3 修复效果复审](PHOENIX-DIAGRAM-P2.code-review.r3.md) |

- R3 所审增量为 `c434577dfba4c0e8953baa315bf5bfb2fa517626..62b8a2affd29e78a9ba9400b7dec6daeca26721a`；报告 SHA-256 为 `762b9aafd337ce3332b56dfa82c2df9180c515e9854ef43c02b3fe8f03e88a99`，评审对象 ID 为 `1e32f818-3bfe-4e1b-9bc3-18cc065b47cc`。
- 本轮实现修复为 `768e739ab520be7cdd79596b41929887dc22da74`；`5d6a511ce61e82cf547f4f5372781da47d01622b` 只回填计划 SHA，也是当前核查 HEAD。已有处置见 [M2 / 评审修复](PHOENIX-DIAGRAM-P2.records/M2.md#评审修复)，收口范围见 [M8 / 评审修复后的重做范围](PHOENIX-DIAGRAM-P2.records/M8.md#评审修复后的重做范围)。
- 计划采用当前版本，SHA-256 为 `bfb464eaec24d7a28bb29200de012939e3190b7d4c771e013febf72c7c6dd860`。本轮评审对象 ID 为 `50e026b1-8ff1-4094-a1d1-cd4f47cb63c1`。
- 本轮审查修复增量及直接回归，没有重新完整评审原 217 个文件。开始时没有未提交代码；既有报告和台账改动是评审上下文，不作为代码 findings 对象。

## 确认问题

未发现本轮新增的可确认问题。F-04 已解决，F-01–F-03 保持已解决；当前没有确认的 P0–P3 代码问题。

## 上轮问题复审

来源为上述 R3 报告及其固定版本；F-01–F-03 沿用 R3 对 R1/R2 的稳定 ID 映射。代码问题与待核实项分别记录，未把缺少终验材料记为代码已证实有错。

| 来源 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R3 / R1 F-01 | 非法仓库 URL 诊断泄露凭据；已解决 | M4，`1b9f910` | **已解决** | [evidence.rs](../../tools/phoenix-diagram/src/evidence.rs):257–270 未变；当前引擎对前导空格、虚构凭据 URL 执行 `validate tree --json`，exit 1、`repository/url-invalid`，stdout/stderr 不含虚构 token；evidence/CLI 回归通过 | 无本问题源码剩余动作 |
| R3 / R1 F-02 | 跨年时间线缺少年份；已解决 | M3，`1b9f910` | **已解决** | [timeline.rs](../../tools/phoenix-diagram/src/layout/timeline.rs):93–117、897–904 未变；2020/2026 同月日样本交付通过，两个节点的 context/aria 分别含完整年份；timeline 与产物检查回归通过 | 最终候选绑定仍见待核实项 |
| R3 / R2 F-03 | 多外键目标的可见详情不完整；已解决 | M2，`1b9f910`、`7842b95` | **已解决** | [erd.rs](../../tools/phoenix-diagram/src/render/erd.rs):342–374 与模板 context 换行规则保留；原三外键、末列改指、单外键 PK/UK 和参考 ERD 在两视口均完整显示目标，clipped=[]，Reach 可操作；末列改指后两视口截图均能区分，已实看窄屏变体 | 无本问题源码剩余动作 |
| R3 / F-04 | 窄屏长详情使导航条盖住 Reach；P2 | M2，`768e739` | **已解决** | [template.source.html](../../tools/phoenix-diagram/viewer/template.source.html):211–213 限高并启用面板滚动。相同输入修前亮/暗真实触摸均命中 `btn-node-finder`；修后面板高 198.53px，真实手势滚动 56px 后按钮命中自身，可信触摸启动 orders 到目标实体的 downstream Reach，页面未滚动。已实看亮/暗打开与滚动后截图；桌面亮色截图与修前逐字节相同；12 个长外键及相关参考例也通过 | 无本问题源码剩余动作；完整 corpus 仍归 M6 |
| R3 / 待核实第 1 项 | 新候选、四平台与宿主收口未重做 | M6/M8 仍进行中 | **待核实** | 当前源摘要为 `e7fda251…`，包中仍为 `7fcabd88…`；没有本轮新候选或同包四平台/宿主证据 | 按 M8 记录补当前引擎与候选证据 |
| R3 / 待核实第 2 项 | V-14 Phoenix UI 全量未完成 | M8 / V-14 | **待核实** | 本轮 npm、diagram 全量及 fmt/clippy 已通过；未运行 `npm run test:phoenix`，本次没有改 Phoenix UI 工作区 | 补齐规定的 Phoenix UI runtime 全量并记录；保留本轮未失效的已通过证据 |
| R3 / 待核实第 3 项 | 局部浏览器结果不覆盖完整十类、五语言、导出与候选验收 | M6/M8 范围未变 | **待核实** | 本轮验证了相关 ERD 的详情、真实触摸/点击及截图，未执行完整 V-7/V-9/V-12/V-13 或独立 UI 验收 | 在最终修复版本补受影响收口，不能由局部通过推导整期通过 |

## 待核实与验证缺口

1. **当前源码尚无对应的最终候选。** 按 `build.rs` INPUTS 的 170 个文件重算，当前 `sourceInputSha256` 为 `e7fda2511ec9934cbd86239c9c871ac22cf658eadf53b377ea0c49ee10c0314e`，与本轮真实交付回执一致；`skills/phoenix-diagram/ENGINE.json`、`dist/phoenix-diagram/bundle/ENGINE.json`、`dist/phoenix-diagram/VERSION.json` 三份文件仍逐字节相同，记录 `7fcabd8897c0b3b563d5b5db74b0eaeaed2b61b3c715ff75305830c7077cb20e`。计划如实保持 6/8、M6/M8 进行中，因此这里是已声明的收口缺口，不是本轮新增发行 finding。按 M8 记录重做新引擎上的 V-7 第 1–5 步、受影响 corpus/来源/语言检查、新候选四平台与本机消费、受影响宿主行和 P2 聚合。
2. **Phoenix UI 全量证据仍缺。** R3 保留的 V-14 缺口未由本轮关闭；`npm test` 和 diagram 维护测试通过不能替代 `npm run test:phoenix`。没有把未运行记为断言失败。后续相关源码、契约、配置与依赖不变时，本轮已通过的 npm/diagram/fmt/clippy 证据可按计划复用。
3. **尚未完成当前版本的完整浏览器与候选验收。** 本轮没有运行整套 `--corpus`、十类探索、全部导出格式、五语言、四平台消费或真实 Claude Code/Codex 新会话，也没有作独立 UI 验收或 `review-record complete`。参考 ERD 的 8 次详情检查全部通过，但本轮只实际查看其中 membership、order_item 的窄屏截图；完整实际看图记录仍归 M6。M2 未完项另有 waterfall 窄屏详情截断观察，本轮未复核，不能据此声称该项已解决，也不把它混入修复增量的新 findings。

## 覆盖与验证

变更意图是让窄屏长详情在导航条上方滚动并保留 Reach 操作，同时让 corpus 检查真实操作 Reach。评审于 2026-10-22（Asia/Shanghai），仓库为 `/Users/rockie/Documents/GitHub/xgent/skills`，由当前单个 reviewer 完成。

### 范围固定

- 原输入：`$review-code docs/plan/PHOENIX-DIAGRAM-P2.md`。已有 R1–R3，按最大轮次选择 R3 进行修复效果复审。
- 已逐一解析计划登记提交并确认均在 HEAD 可达：原实施为 `711a5b7b44ba7b1c76dfe1c3ca2e2a3418b43ca2`、`e09bc820e0c7c412ad34124cbae0ce17d9bf3805`、`a8f3ec40d39f2d90217d08151b037baec7f0b328`、`8539d18c18ef2055f29aca3e866015b1bedc0b74`；前轮修复为 `1b9f9105dcaab4984fd70e4ea0d97b0b9986d9f1`、`7842b957191470b7d46cab0f0b6796b23e33aacc`；本轮加入 `768e739ab520be7cdd79596b41929887dc22da74`。之前已审部分没有扩大为完整重审。
- 从最早登记提交到 HEAD，其他提交 `0a8c6a88b62485d83a5ec45f876e4170f1f5cbc6`、`5c0528d6f7473d958560866a65c69731a89dc424`、`0b5b37d9179682842e6c35b6cfc77f27d62c7d6d`、`c434577dfba4c0e8953baa315bf5bfb2fa517626`、`62b8a2affd29e78a9ba9400b7dec6daeca26721a`、`5d6a511ce61e82cf547f4f5372781da47d01622b` 均只修改计划文件，作为进度回写排除；没有未登记代码提交。
- 实际比较为 `git diff --no-ext-diff --no-textconv 62b8a2a… 5d6a511… --`，完整读取 9 个变更文件的 diff：6 个代码/测试文件为 `scripts/test-phoenix-diagram-scopes.mjs`、`test/phoenix-diagram/core-runner.test.js`、`tools/phoenix-diagram/viewer/template.source.html`、`assets/template.html`、`tests/export-acceptance.mjs`、`tests/export-acceptance.test.mjs`（后三者相对 `tools/phoenix-diagram/`）；另有计划与 M2/M8 记录。追踪详情放置、面板滚动、Reach 状态、导航遮挡、真实输入控制器、corpus 接线及截图/回执绑定；F-01–F-03 检查必要的未变实现和回归。
- 开始时无 staged 差异；tracked 未提交仅计划状态页和 review 索引，untracked 为 R1–R3 报告及对应状态页，没有未提交代码。以路径 → 文件字节 SHA-256 固定全部 4,837 个可见 tracked/untracked 文件，另存 HEAD、branch 和完整 index 条目；快照 SHA-256 为 `ea34cf4ca22f6212822e5b852ed3543efec3db557e630575ee8b20d262c3a7a1`。全部验证后、写报告前再次核对，文件、HEAD、index 均未变。本轮报告及台账回写不混入被审范围。

### 实际验证

当前版本验证使用上述 HEAD 的源码；没有修改源码、测试或计划，没有 checkout/reset/stash 用户工作树。输入、脚本、二进制副本、日志和截图仅写系统临时目录。

| 命令 / 检查 | 结果与限制 |
| --- | --- |
| Node 25.2.1 下 `npm test` | exit 0；Node 490 tests，467 pass / 0 fail / 23 skipped；Python 四组 22、24、2、13 tests 均 OK。 |
| `MISE_ENV_CACHE=0 mise exec --no-deps -- node scripts/test-phoenix-diagram-native.mjs`（`test:diagram` 同一入口） | exit 0；assemble `--check`、fmt、clippy 通过；Rust 344 passed / 0 failed / 2 ignored；Node 214 tests，209 pass / 5 skipped；Reader/上游 125/125 pass。工具报告 Rust 1.99.0、mbx 1.24.0、Node 25.2.1。此入口不覆盖真实浏览器或 provider 会话。 |
| 同一三长外键 ERD，修前/修后引擎 `deliver erd` | 均 exit 0、status=pass。使用 R3 描述的原始输入，两个实体宽 340、同排，orders 的三个长外键均指向 `customer_account_external_identity.external_account_identifier`，关系为 orders → customer_account_external_identity。 |
| 当前 `runPassportAcceptance` 检查修前产物 | 390×844 亮、暗各一次均按预期失败：`Reach downstream of orders cannot be operated; its center hits btn-node-finder`；1440×900 亮色通过。说明新增控制器能捕获原 F-04。 |
| 同一控制器检查修后产物 | 三长外键在 390×844 亮/暗均经真实滚动与触摸启动正确 Reach；1440×900 亮/暗均由点击启动，4 次 pass。桌面亮色焦点截图与修前 SHA-256 相同。 |
| 独立 `touch-check.mjs`，相同输入与 390×844 触摸模拟 | 修前两主题面板高 254.86px，真实点击 isTrusted=true、target=btn-node-finder，focus/Reach 清空。修后两主题面板高 198.53px、底边不压导航条，真实手势滚动 56px 后按钮中心命中其 span，最后一行 context 可见、页面 scrollY 不变；isTrusted=true 的触摸启动 originId=orders、direction=downstream、nodeIds=[orders, customer_account_external_identity]。已实际查看打开与滚动后的亮/暗截图。 |
| 12 个长外键 ERD，当前引擎与真实 Reader | showcase 交付通过；390×844 亮/暗经滚动、触摸启动 Reach，1440×900 亮色点击启动，3 次 pass、clipped=[]。 |
| R2 原三外键、末列改指、单外键 PK/UK 四份输入 | 交付均通过；各在 1440×900 与 390×844 检查详情和真实 Reach，共 8 次 pass。全部目标在 context 中，无横向截断；末列改指的两视口截图均与原样本不同。已实际查看窄屏原样本和变体。 |
| 冻结参考 orders.erd、subscription-billing.erd | 当前引擎交付通过；order_item、membership、subscription、payment 在桌面/窄屏共 8 次详情与 Reach 检查全部通过，目标完整、clipped=[]；全部运行离线、零远程请求。冻结参考文件的大小/摘要前后完全相同。实际查看了两张窄屏参考截图，未冒称完成整套 corpus 看图。 |
| 当前引擎 `validate tree bad-url.json --json` | exit 1、`repository/url-invalid`；虚构 token 未进入 stdout/stderr。 |
| 当前引擎 `deliver timeline`，2020/2026 同月日输入 | exit 0、status=pass；产物节点的 context/aria 分别含 `2020-01-01 00:00:00 UTC+00:00` 和 `2026-01-01 00:00:00 UTC+00:00`。本轮未重新做该样本的浏览器看图，相关 Rust/产物回归通过。 |
| `git diff --no-ext-diff --no-textconv --check 62b8a2a 5d6a511 --` | exit 0。 |

当前实测二进制 SHA-256 为 `2231e80ead0afaf82ed495a092080e572c887a33b964bca9cb5e11830c59e123`，交付回执的源码摘要与当前 170 个构建输入一致。修前使用 R3 留存二进制，SHA-256 为 `a03d585aec11238a1155371da1207b075aa101e965b716f7b8a9c998d88127e4`；从 base `62b8a2a` 的 Git 树重算相同输入集合得到 `c29de5e38ff29329b65f6f1479abacd9552e7300d244ce1366ee485a58d3d210`，与该二进制的实际交付回执及 R3 记录一致。没有用当前 checkout 的执行结果冒充历史版本验证。

真实浏览器为 Chrome 156.0.8078.48，使用独立临时 profile、debugging pipe 和触摸模拟，未使用用户登录会话。临时材料位于 `/var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-p2-r4-review-_rocbjxc/`，含 `input-snapshot.json`、`engine-identity.json`、原始输入、交付回执、`check.mjs`/`results.json`、`touch-check.mjs`/`touch-results.json`、截图与维护测试日志。该目录可被系统清理；源码版本、输入构成、步骤及可观察结果已在报告中保留。独立触摸驱动初次在修前焦点清空后读取空 Range 时失败，修正驱动后重新完成全部对照；没有把该驱动错误归为产品缺陷。

## 计划符合性

| 本轮相关承诺 | 实现 / 验证状态 |
| --- | --- |
| M2 / WORK-F1、§5.2，完整外键目标可读 | F-03 保持解决，桌面/窄屏、多列/单列与末列改指验证通过。 |
| §6.1 / WORK-F5、§7 / WORK-N1，窄屏探索可用 | 原 F-04 输入的亮/暗真实触摸、面板滚动及可见命中均已核实，F-04 关闭；未宣称全部 UI 状态终验。 |
| V-8，合成三长外键与触摸 Reach 接线 | 本轮源码接线、维护单测及局部真实浏览器通过；完整 `--corpus`、全部截图实际看图仍归 M6。 |
| M6/M8 / V-7、V-9、V-12–V-14、§9.3 | 仍为 6/8、两收口里程碑进行中。新候选与受影响证据、Phoenix UI 全量尚待补齐；本轮不能据此判整期验收通过。 |

## 总体结论与下一步

验收判定：**需补证**。

本轮修复增量的正确性结论为 `patch is correct`：没有发现新增缺陷，F-04 的关闭条件有修前/修后真实触摸、尺寸、命中、状态与截图证据，F-01–F-03 保持已解决。本轮新关闭 1 项、保持解决 3 项、不再适用 0 项。

包含 R3 全部待核实项的**复审总体修复效果结论为无法判定**，整体验收为 `inconclusive`：3 项待核实范围仍未完成，没有当前源码对应的最终候选及完整收口证据。该限制不推翻上述代码问题的关闭，也不能由修复增量正确推导整期完成。

先按 [M8 重做范围](PHOENIX-DIAGRAM-P2.records/M8.md#评审修复后的重做范围) 完成新引擎上的 M6、同包候选的四平台/本机消费、受影响宿主行、P2 聚合和仍缺的 V-14，保存可追溯证据后再次复审。后续相关内容未变的维护测试结果可复用。本轮不修改代码或计划，不启动修复、推送、CI、宿主会话或后续验收技能。

补证后下一步：`$review-code docs/plan/PHOENIX-DIAGRAM-P2.md`。
