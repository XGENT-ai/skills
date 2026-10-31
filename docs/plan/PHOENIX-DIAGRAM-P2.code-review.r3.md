# Phoenix Diagram P2 · R3 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3，开发计划模式，修复效果复审 | [PHOENIX-DIAGRAM-P2](PHOENIX-DIAGRAM-P2.md) | 修复增量 `c434577dfba4c0e8953baa315bf5bfb2fa517626..62b8a2affd29e78a9ba9400b7dec6daeca26721a`；逐项核对 R2 的问题及待核实项 | [R2 修复效果复审](PHOENIX-DIAGRAM-P2.code-review.r2.md) |

- R2 所审范围为 `8539d18c18ef2055f29aca3e866015b1bedc0b74..c434577dfba4c0e8953baa315bf5bfb2fa517626`；报告 SHA-256 为 `95f2b64af870928eb764fdb0d36924dec499e9f758a6f2403144f008007b03cd`，评审对象 ID 为 `a8a28d1b-313e-4778-8c24-e71ae7a63579`。
- 本轮修复提交为 `7842b957191470b7d46cab0f0b6796b23e33aacc`；`62b8a2affd29e78a9ba9400b7dec6daeca26721a` 只回填计划 SHA，也是当前核查 HEAD。已有处置见 [M2 / 评审修复](PHOENIX-DIAGRAM-P2.records/M2.md#评审修复)，收口重做范围见 [M8](PHOENIX-DIAGRAM-P2.records/M8.md#评审修复后的重做范围)。
- 计划采用当前版本，SHA-256 为 `9899acb71bfea684b090e4c0cc714b66384beb9cdef5f38980a56cc804b58e2b`。本轮评审对象 ID 为 `1e32f818-3bfe-4e1b-9bc3-18cc065b47cc`。
- 这是修复增量及直接回归的复审，未重新完整审查原 217 个文件。没有未提交代码；开始时已有的报告和台账改动不作为代码 findings 对象。

## 确认问题

R1 的 F-01、F-02 保持已解决，R2 的 F-03 续项已解决；本轮新增 **1 项 P2**。

### F-04 · [P2] 为换行后的详情保留窄屏 Reach 操作空间

- 定位：[viewer/template.source.html](../../tools/phoenix-diagram/viewer/template.source.html):206，target `62b8a2a`；生成副本为 [assets/template.html](../../tools/phoenix-diagram/assets/template.html):5094。本行在本轮 diff 内。
- 问题：在 390×844 的普通阅读模式中，合法 ERD 的三个较长外键会使新增换行规则将详情面板从约 195px 增至 255px，而当前画布只有约 281px 高。面板没有限制高度或可滚动的正文，既有避让函数找不到可用位置后仍保留重叠位置，底部导航条因此完整盖住 Upstream/Downstream 按钮。真实触摸 Downstream 按钮中心会命中导航条的节点查找按钮，清除焦点并打开查找，无法执行原本有一个目标的 Reach；亮、暗主题均复现。同一输入在修前可以正常触摸启动 Reach，因此冻结的放置逻辑不能排除本次换行改动激活的回归。它破坏了计划 §6.1 的探索能力与 §7 的窄屏完整要求。
- 证据：两个实体均设 `width:340`、`row:0`，分别在 col 0/1：`customer_account_external_identity`（label 为 customer，`id bigint pk`、`external_account_identifier bigint uk`）与 `orders`（`id bigint pk`，另有 `customer_billing_identifier_1`、`customer_billing_identifier_2`、`customer_billing_identifier_3` 三个 bigint fk，均引用 `customer_account_external_identity.external_account_identifier`）；关系为 orders → customer_account_external_identity 的 many/one，质量档为 showcase。修前与修后真实 `deliver erd` 均 exit 0、status=pass、零诊断。Chrome 156.0.8078.48 中 `#focus=orders` 打开详情后，修后面板高 254.86px，导航条覆盖 y=357.53–411.53，两个 Reach 按钮均在 y=359.47–391.47。触摸 Downstream 中心 `(221.30,375.47)` 产生 `isTrusted=true`、target=`btn-node-finder` 的 click，`focus` 由 orders 变为 null，Reach 仍为 null；修前面板高 194.86px，触摸其 Downstream 中心后得到 `direction=downstream`，nodeIds 为 orders 与目标实体。已实际查看修前亮色、修后亮/暗截图。[focus.js](../../tools/phoenix-diagram/viewer/focus.js):1204–1214、1239–1262 在无避让候选时仍使用原 top；[viewer.css](../../tools/phoenix-diagram/viewer/viewer.css):1561–1577 的面板无高度约束，导航条样式在 :765–780。[计划](PHOENIX-DIAGRAM-P2.md):443–447、500 是受影响契约。改后的 `runPassportAcceptance` 对同一失败样本仍返回 pass、clipped=[]，因为 [截断检查](../../tools/phoenix-diagram/tests/export-acceptance.mjs):542–547 只检查文字的横向尺寸，不检查控件遮挡。
- 修复与验证：让变高的详情在可用画布内滚动、展开或合理停靠，保持全部外键可读，并让 Reach 与导航条均可操作。补上述三个长外键的 390×844 亮/暗真实 Reader 回归，检查按钮实际命中元素，再用真实触摸断言 Downstream 激活正确的 Reach；保留现有桌面、短外键和全部目标可读的回归。单独验证 context 的 scrollWidth/clientWidth 不足以关闭本问题。
- 置信度：高；相同输入、修前/修后源身份、实际触摸事件、状态变化与截图一致。

## 上轮问题复审

来源为上述 R2 报告及其 SHA-256；F-01/F-02 沿用 R2 对 R1 稳定 ID 的映射。本轮没有将旧问题重新编号为新增问题。

| 来源 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R2 / R1 F-01 | 非法仓库 URL 诊断泄露凭据；R2 已解决 | M4，`1b9f910` | **已解决** | URL 错误路径与 R2 完全相同；当前引擎重新验证前导空格、虚构凭据的 URL，exit 1、`repository/url-invalid`，stdout/stderr 均无虚构 token；evidence/CLI 回归通过 | 无本问题源码剩余动作 |
| R2 / R1 F-02 | 跨年时间线没有可读年份；R2 已解决 | M3，`1b9f910` | **已解决** | Clock 与相关渲染代码未变；当前引擎重新交付 2020/2026 同月日样本通过，timeline/产物回归通过 | 最终候选证据仍见下方缺口 |
| R2 / F-03（续） | 多外键目标虽在属性中，详情单行省略；部分解决 | M2，`7842b95` | **已解决** | 原三外键样本与只改 shipping_ref 目标的变体均交付通过；桌面、390 窄屏全部目标完整，两个视口的变体截图均可区分；真实 Enter 与触摸点开后 context 为完整列表，253/253、241/241；两个参考 ERD 的 8 次详情检查全部通过。Reach 回归另列本轮 F-04 | 原可读性问题关闭；处理新增 F-04 |
| R2 / 待核实第 1 项，含 R1 的最终包缺口 | 新候选、四平台、宿主与收口未重做 | M6/M8 仍进行中 | **待核实** | 当前源码摘要为 `c29de5e3…`，包中仍为 `7fcabd88…`；本轮没有 dispatch CI、生成最终候选或开启真实宿主会话 | F-04 修复后按 M8 重做受影响候选证据 |
| R2 / 待核实第 2 项，含 R1 的 Phoenix UI 全量缺口 | R2 安装 7/7 通过，但 UI 全量被停止，不能记完整通过 | M8 / V-14 | **待核实** | 本轮未运行 `npm run test:phoenix`；本次 diff 未改 Phoenix UI 工作区，不能把 npm/diagram 通过替代此项 | 在 M8 完成 V-14 规定的 Phoenix runtime 全量并记录结果 |
| R2 / 待核实第 3 项 | 局部浏览器检查不覆盖完整十类、五语言、导出和 UI 验收 | M6/M8 收口范围未变 | **待核实** | 本轮补了相关 ERD 的真实 Reader、参考详情与触摸回归，未执行完整 V-7/V-9/V-12/V-13 或独立 UI 验收 | 保留原范围，补受影响收口；不将局部检查登记为整期通过 |

## 待核实与验证缺口

1. **新引擎与最终包仍不一致。** 本轮源码摘要 `c29de5e38ff29329b65f6f1479abacd9552e7300d244ce1366ee485a58d3d210`；`skills/phoenix-diagram/ENGINE.json`、`dist/phoenix-diagram/bundle/ENGINE.json`、`dist/phoenix-diagram/VERSION.json` 三份文件仍逐字节相同，记录 `7fcabd8897c0b3b563d5b5db74b0eaeaed2b61b3c715ff75305830c7077cb20e`。计划已经声明重开 M6/M8，本轮保留该缺口，不将其作为新增发行缺陷，也不沿用旧候选通过来验收当前源码。
2. **V-14 的 Phoenix UI 全量仍未补齐。** 本轮 npm 与 diagram 维护测试已完成，但未运行 `npm run test:phoenix`。R2 的未完成记录不因此关闭；没有将未运行描述为断言失败。
3. **局部实测不代替完整候选验收。** 两份参考 ERD 的 8 次详情检查是本轮直接调用真实控制器的局部验证，没有运行完整 `--corpus`、V-7 第 1–5 步、四平台消费、真实 Claude Code/Codex 会话、五语言与全部导出格式。上述证据按 M8 记录在最终修复版本重取。

## 覆盖与验证

变更意图是让 ERD 多列外键目标在 Reader 中完整可读，并把相关检查接入参考 corpus。评审于 2026-10-22（Asia/Shanghai），仓库为 `/Users/rockie/Documents/GitHub/xgent/skills`，由当前单个 reviewer 完成。

### 范围固定

- 原输入：`$review-code docs/plan/PHOENIX-DIAGRAM-P2.md`。已有 R1/R2，按最大轮次选择 R2 进行修复效果复审。
- 计划登记的原实施提交为 `711a5b7b44ba7b1c76dfe1c3ca2e2a3418b43ca2`、`e09bc820e0c7c412ad34124cbae0ce17d9bf3805`、`a8f3ec40d39f2d90217d08151b037baec7f0b328`、`8539d18c18ef2055f29aca3e866015b1bedc0b74`，R1 修复为 `1b9f9105dcaab4984fd70e4ea0d97b0b9986d9f1`，本轮加入 R2 修复 `7842b957191470b7d46cab0f0b6796b23e33aacc`；全部已解析并确认在当前 HEAD 可达。此前登记实现已由 R1/R2 覆盖，不在本轮完整重审。
- 从最早登记提交到 HEAD 的其他提交 `0a8c6a88b62485d83a5ec45f876e4170f1f5cbc6`、`5c0528d6f7473d958560866a65c69731a89dc424`、`0b5b37d9179682842e6c35b6cfc77f27d62c7d6d`、`c434577dfba4c0e8953baa315bf5bfb2fa517626`、`62b8a2affd29e78a9ba9400b7dec6daeca26721a` 都只修改本计划文件，作为进度回写排除。没有未登记的代码提交。
- 实际比较为 `git diff --no-ext-diff --no-textconv c434577… 62b8a2a… --`。完整读取 8 个变更文件的 diff：`scripts/test-phoenix-diagram-scopes.mjs`、`test/phoenix-diagram/core-runner.test.js`、`tools/phoenix-diagram/viewer/template.source.html`、`assets/template.html`、`tests/export-acceptance.mjs`，以及计划和 M2/M8 记录。其中 `assets/template.html` 与 `tests/export-acceptance.mjs` 相对 `tools/phoenix-diagram/`。
- 追踪了 ERD context 的生成、焦点元数据、面板定位、Reach、导航条、corpus 截图及回执绑定、真实浏览器控制器。对 F-01/F-02 的代码和测试作适用性核对；未扩为全仓普查。
- 开始时无 staged 差异。tracked 未提交改动只有计划状态页与 review 索引；untracked 为 R1/R2 报告及各自状态页，无未提交代码。以路径 → 文件字节 SHA-256 记录全部 4,835 个可见 tracked/untracked 文件，另存 HEAD、branch 和完整 index 条目；落报告前复核全部内容与 HEAD 一致。快照 SHA-256 为 `c001c16a9820a0017588506da60e5cce1e22fdc84143d728bf34c87d10a1a9d8`。本轮报告、台账和临时验证产物不混入审查对象。

### 实际验证

所有当前版本验证均在上述 HEAD 的源码上执行，未修改源码、测试或计划，未 checkout/reset/stash 工作树。临时输入、旧版本二进制与复现脚本只写系统临时目录。

| 命令 / 检查 | 结果与限制 |
| --- | --- |
| PATH 首位固定 Node 25.2.1 后 `npm test` | exit 0；Node 490 tests，467 pass / 0 fail / 23 skipped；Python 四组 22、24、2、13 tests 均 OK。 |
| `MISE_ENV_CACHE=0 mise exec --no-deps -- node scripts/test-phoenix-diagram-native.mjs`（`test:diagram` 同一入口） | exit 0；assemble `--check`、fmt、clippy 通过；Rust 344 passed / 0 failed / 2 ignored；Node 214 tests，209 pass / 5 skipped；Reader/上游 124/124 pass。工具报告 Rust 1.99.0、mbx 1.24.0、Node 25.2.1。该入口不包含真实浏览器或 provider 会话。 |
| 当前引擎 `validate tree bad-url.json --json` | exit 1；`repository/url-invalid`；虚构 token 未进入 stdout/stderr。 |
| 当前引擎 `deliver timeline`，2020/2026 同月日样本 | exit 0、status=pass；结合未变的 Clock 和本轮 timeline 回归，保留 F-02 已解决结论。 |
| 当前引擎交付单外键 PK/UK、原三外键及末列改指变体 | 均 exit 0、status=pass；三外键在 1440×900、390×844 的全部非空文本字符均处于实际可见命中区域，末列改指后两个视口的截图均改变；实际查看原样本截图。 |
| 原三外键样本，真实 Enter 与触摸点开 | Enter 事件 isTrusted=true；窄屏先真实触摸左滑，再点 orders。两种入口均 focus=orders、详情可见、包含全部目标；context 的 scrollWidth/clientWidth 分别为 253/253、241/241，实际查看窄屏截图。 |
| 两份冻结参考 ERD，直接调用 `runPassportAcceptance` | orders/order_item 与 subscription-billing 的 membership、subscription、payment，在 1440×900、390×844 共 8 次 pass；每个目标均在 context，clipped=[]，离线执行；运行前后参考文件的大小/摘要与冻结 manifest 一致。只记局部控制器实测，不冒称完成整套 corpus 或其全量看图。 |
| 3、6、9、12 个长外键样本的当前引擎交付和真实 Reader | 全部 showcase 交付通过；1440×900 下未见按钮遮挡，390×844 均观测到 Reach 按钮中心被覆盖。F-04 用最小的三个长外键样本作修前/修后及真实触摸验证。 |
| 三长外键的 390×844 真实触摸：修前亮色、修后亮/暗 | 修前 Downstream 正常激活下游两节点；修后亮/暗均命中 `btn-node-finder`、focus=null、Reach=null；实际查看修前亮色与修后亮/暗截图。 |
| 对同一遮挡样本调用改后的 `runPassportAcceptance` | status=pass、errors=[]、clipped=[]；说明现有文字尺寸断言未覆盖 F-04 的实际操作回归。 |

当前实测二进制 SHA-256 为 `a03d585aec11238a1155371da1207b075aa101e965b716f7b8a9c998d88127e4`；按 `build.rs` INPUTS 枚举并规范化排序的 170 个文件重算得源摘要 `c29de5e3…`，与每份实际交付回执一致。修前使用 R2 留存二进制，SHA-256 为 `1ee39b5fe8eafe0facfade878e6ed3beca1edc01d09bd3c79b1565d994991ea0`；重算 base `c434577` 的同一输入集合为 `ab4e277d36be7950c3ce93b4c4f7c80be00dcc7014e6847946a4c7c6f49dc19c`，与修前交付回执和 R2 记录一致。历史验证没有误用当前 checkout 代替修前版本。真实浏览器为本机 Chrome 156.0.8078.48，使用独立临时 profile 和 debugging pipe，未使用用户登录会话。

临时证据位于本机 `/var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-p2-r3-review-7pj3rx1a/`：`input-snapshot.json`、`engine-identity.json`、`erd-long-3.json`、交付回执、`reader-check.mjs`/`reader-results.json`、`reach-check.mjs`/`reach-results.json`、`passport-check.mjs`、`entrance-check.mjs`、截图及两份维护测试日志。该目录可被系统清理；F-04 的输入构成、命令、版本与可观察结果已在报告中完整描述。临时浏览器脚本的初次运行曾因同文档导航等待 load event、以及未先平移到可见节点而失败；修正测试驱动后完成实测，没有将这些驱动失败归为产品缺陷。

## 计划符合性

| 本轮承诺 | 实现 / 验证状态 |
| --- | --- |
| M2 / WORK-F1、§5.2，全部外键目标可读 | F-03 已解决：换行与完整 context 已由真实 Reader 核实；单列 PK/UK 回归保留。 |
| §6.1 / WORK-F5、§7 / WORK-N1，探索能力与窄屏完整 | **存在缺陷 F-04**：长外键详情令 Reach 被导航条盖住，真实触摸触发错误操作。 |
| V-8，新加多外键详情检查 | 接线与局部真实控制器验证通过，完整 corpus 收口仍待 M6；现有断言不覆盖按钮遮挡。 |
| M6/M8 / V-7、V-9、V-12–V-14、§9.3 证据绑定 | 计划保持 6/8、M6/M8 进行中，符合当前缺口；本轮 npm/diagram 通过不能代替最终包、四平台、宿主、Phoenix UI 全量或独立 UI 验收。 |

## 总体结论与修复顺序

验收判定：**需要修改**。

修复效果结论：**未完全解决**。旧 F-01–F-03 均已解决或保持解决；本次修复新增 F-04（P2），真实窄屏触摸操作受到阻塞。修复增量正确性结论为 `patch is incorrect`，依据为 F-04，不依据未运行的外部验收猜测代码有错。

先处理详情高度与导航/Reach 的空间冲突，保留完整外键文本，复跑同一输入的亮/暗窄屏真实触摸及相关 Reader 回归，再复审。之后按 M8 记录补新引擎/同包候选的 M6/M8 证据；未完成的候选、UI 全量及宿主验证继续保留，不由本轮局部通过关闭。本轮不授权或启动修复、CI、推送或宿主验收。

下一步：`$apply-code-review docs/plan/PHOENIX-DIAGRAM-P2.code-review.r3.md`；修复后 `$review-code docs/plan/PHOENIX-DIAGRAM-P2.md`。
