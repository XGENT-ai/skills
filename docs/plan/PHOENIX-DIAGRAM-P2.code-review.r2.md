# Phoenix Diagram P2 · R2 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2，开发计划模式，修复效果复审 | [PHOENIX-DIAGRAM-P2](PHOENIX-DIAGRAM-P2.md) | 修复增量 `8539d18c18ef2055f29aca3e866015b1bedc0b74..c434577dfba4c0e8953baa315bf5bfb2fa517626`；逐项复核 R1 的全部问题及待核实项 | [R1 代码评审](PHOENIX-DIAGRAM-P2.code-review.md) |

- 上轮所审范围为 `e29d4cafcada8809335e0a3dfaa939be95bb9643..8539d18c18ef2055f29aca3e866015b1bedc0b74`，观察版本为 `0b5b37d9179682842e6c35b6cfc77f27d62c7d6d`。R1 报告 SHA-256 为 `75fca2bd14c1ad61ffe96147818d47ec67a6e1fe911132946372c2b2234c84c9`，评审对象 ID 为 `a9c97086-4689-4153-9e84-0b77207f5043`。
- 本轮实现修复为 `1b9f9105dcaab4984fd70e4ea0d97b0b9986d9f1`，当前核查版本为 `c434577dfba4c0e8953baa315bf5bfb2fa517626`。已有处置见 [M4/F-01](PHOENIX-DIAGRAM-P2.records/M4.md#评审修复)、[M3/F-02](PHOENIX-DIAGRAM-P2.records/M3.md#评审修复)、[M2/F-03](PHOENIX-DIAGRAM-P2.records/M2.md#评审修复)，收口重做范围见 [M8](PHOENIX-DIAGRAM-P2.records/M8.md#评审修复后的重做范围)。
- 计划采用当前版本，SHA-256 为 `628f4688b65b131ed6242ae1a516f41017327e0cd7f5b79c44296aaf87713820`。仅审修复增量及其直接上下游，未重新完整评审原 217 个文件。

## 确认问题

本轮没有新增独立问题；上轮 F-03 **部分解决，仍有 P2 剩余影响**。F-01、F-02 已解决，详见复审表。

### F-03（续）· [P2] 让多个外键目标在阅读详情中完整可读

- 定位：[src/render/erd.rs](../../tools/phoenix-diagram/src/render/erd.rs):358–361，target `c434577`；本次修复新增的多目标 context 构造。
- 问题：同一实体存在多列外键时，修复把所有“列 → 目标”拼进 `data-node-context`，Reader 随后把它放进会单行省略的 `#focus-context` 标签。合法的三列外键样本在桌面与窄屏下都只显示到第二列名称附近，后续引用目标仍无法从焦点详情读出；表行仍只画键徽标、名称、类型。完整值现在确实保留在 DOM、aria-label 和 SVG `<title>` 中，单列 PK/UK 对照也已修好，但详情没有换行、展开或完整文本提示；SVG 原生悬停提示不能补齐键盘与触屏的可见详情。因此这仍是原 F-03 的可读语义缺口，不能以属性包含全部字符串判定关闭。
- 证据：[focus.js](../../tools/phoenix-diagram/viewer/focus.js):339–342、398–402 直接将 context 写入面板；[viewer.css](../../tools/phoenix-diagram/viewer/viewer.css):1681–1694 对该标签施加 `overflow:hidden`、`text-overflow:ellipsis`、`white-space:nowrap`。样本为 `customer` 表有 `id bigint pk`、`external_id bigint uk`；`orders` 表除 `id bigint pk` 外有 `customer_ref → customer.external_id`、`billing_ref → customer.id`、`shipping_ref → customer.external_id` 三个 `bigint fk`，实体分别放在 `(row=0,col=0/1)`，关系为 `orders → customer` 的 many/one。真实 `deliver erd` 退出 0、`status=pass`。Chrome 中打开 orders 焦点详情后，1440×900 的 context `scrollWidth=479`、`clientWidth=253`；390×844 为 `479/241`，且没有 `title` 属性或展开控件。另以真实 Enter 键打开 orders 详情，确认 `focus.active()=orders`、面板显示且同样为 `479/253`。已实际查看两张截图确认省略号。只把 `shipping_ref.references` 改成 `customer.id` 后，两次交付仍通过，而两个视口的焦点截图分别逐字节相同；桌面 SHA-256 均为 `7b8de756bc59798774d9ea102d43b214389b232413c6f487f4c65ea7ba5ac239`，窄屏均为 `12da6a7572f3931cb655ace6afe7a7e012523850f83b938c5762239289ac0507`。[PRD WORK-F1](../PRD-PHOENIX-DIAGRAM.md):124、133、136 要求外键目标从可见图与阅读详情获得；[计划 §7](PHOENIX-DIAGRAM-P2.md):500 要求窄屏完整、界面无截断。新增 [ERD 回归](../../tools/phoenix-diagram/tests/erd.rs):579–626 只验证属性与 title，未验证详情的显示容量。
- 修复与验证：给外键列表提供可以完整阅读的详情区域，允许换行、滚动或明确展开，并保持节点目录按 context 首段分组。补多列外键及长目标名的真实 Reader 检查，覆盖桌面、390 窄屏与键盘/触屏可用的入口；断言全部目标可被读到，并验证只改最后一列的目标时详情能区分。保留单列 PK/UK 与 SVG 导出 title 的既有回归。
- 置信度：高；源码、实际渲染尺寸、人工看图和只改隐藏目标的截图对照一致。此处不再声称 SVG 或元数据完全缺失目标，而是明确其可见详情仍不完整。

## 上轮问题复审

来源统一为上表 R1 报告及其 SHA-256，稳定 ID 不变。

| 来源 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01 | 非法仓库 URL 的诊断泄露凭据；P2 | M4，`1b9f910`，改为不回显原始 URL 的固定消息，删除无调用方的 redact | **已解决** | `src/evidence.rs:257–270`；正常凭据 URL 和六种非法变体的 evidence 回归、真实 CLI validate/deliver 回归通过；独立用 R1 的前导空格样本执行 `validate tree --json`，exit 1、`repository/url-invalid`、`reason=syntax`，stdout/stderr 不含虚构 token | 无源码剩余动作；新候选验证另见下文 |
| R1 / F-02 | 跨年时间线没有可读年份；P2 | M3，`1b9f910`，按墙钟年份决定完整日期，刻度以完整日期判定变化 | **已解决** | `src/layout/timeline.rs:93–117,897–904`，渲染和产物门均走同一 Clock；R1 的 2020/2026 同月日样本实际交付通过。Chrome 中卡片、刻度、焦点与目录均能区分年份，已看 1440×900 截图；详情为 `2020-01-01 00:00:00 UTC+00:00`，另一个事件含 2026 | 无源码剩余动作；最终包证据尚待重做 |
| R1 / F-03 | ERD 缺少列级外键目标；P2 | M2，`1b9f910`，把每列目标加入 context、aria-label、SVG title | **部分解决** | 单列 PK/UK 两次交付和浏览器焦点详情均能区分，节点目录仍一组；多列场景被详情标签省略，见本节前的 F-03 续项 | 完整呈现多列/长外键列表并做真实 Reader 回归 |
| R1 / 待核实第 1 项 | Phoenix UI 安装用例引擎探测超时；后续全量未执行 | M8 将其归入 V-14 重做；本轮未修改该工作区 | **部分解决** | 本轮 7/7 安装测试通过，原冲突用例在 3.57 秒完成，探测超时未重现；后续未改动的 UI 安装迁移全量由 reviewer 主动停止，整套未完成，见验证表 | 在 M8 补完整 V-14；不能用本轮安装测试通过关闭全部 UI 回归证据缺口 |
| R1 / 待核实第 2 项 | 四平台、最终包和真实宿主验收未由 reviewer 独立重做 | M6/M8 已重开，原候选只证明旧引擎 | **待核实** | 当前源摘要与包内 ENGINE 摘要不同，见下文；本轮没有 dispatch、访问实时 Actions API 或开启真实宿主会话 | 在含完整修复的新引擎/同包候选上补 M6、M8 所列受影响证据 |

## 待核实与验证缺口

1. **新候选与收口尚未完成。** 本轮引擎源摘要为 `ab4e277d36be7950c3ce93b4c4f7c80be00dcc7014e6847946a4c7c6f49dc19c`；`skills/phoenix-diagram/ENGINE.json`、`dist/phoenix-diagram/bundle/ENGINE.json` 与 `dist/phoenix-diagram/VERSION.json` 仍写 `7fcabd8897c0b3b563d5b5db74b0eaeaed2b61b3c715ff75305830c7077cb20e`。三份文件字节相同，但不对应本轮源码。计划已明确重开 M6/M8，本报告不把它当成新引入的未声明发行缺陷，也不把旧候选通过当作当前整期通过。待 F-03 完整修复后，按 M8 记录重做 V-7 第 1–5 步、V-12/V-13、受影响宿主行、P2 聚合及 V-14。
2. **Phoenix UI 全量未完成。** 配置固定 wasm-opt 后，critique 与安装 7/7 通过，原超时未重现；后续未改动的 UI 安装迁移回归由 reviewer 主动停止，脚本 exit 1（SIGTERM，日志末行为 `sandbox-exec exited null`），不代表测试断言失败。已结束的 Rust 测试组合计 866 passed / 0 failed / 1 ignored，但正在执行的 `tests/installer.rs` 15 用例组未完成，后续 Rust、真实浏览器、WASM 与 oracle 阶段也未全部执行。本轮不能记 UI 全量通过，完整 V-14 仍须在 M8 补齐。
3. 本轮局部浏览器验证只覆盖上述时间线与 ERD 的亮色场景，没有重做完整十类、五语言、全部导出格式、所有桌面尺寸、四平台或 Claude Code/Codex 验收。不把局部截图检查登记为 UI 验收通过或 `review-record complete`。

## 覆盖与验证

变更意图是修复 R1 的凭据输出及两种图型的可读语义。评审于 2026-10-22（Asia/Shanghai）在仓库 `/Users/rockie/Documents/GitHub/xgent/skills` 执行，由当前单个 reviewer 完成。

### 范围固定

- 原输入：`$review-code docs/plan/PHOENIX-DIAGRAM-P2.md`。已有 R1，故按技能规则进行修复效果复审。
- 计划登记的提交 `711a5b7b44ba7b1c76dfe1c3ca2e2a3418b43ca2`、`e09bc820e0c7c412ad34124cbae0ce17d9bf3805`、`a8f3ec40d39f2d90217d08151b037baec7f0b328`、`8539d18c18ef2055f29aca3e866015b1bedc0b74` 已由 R1 覆盖；本轮加入登记修复 `1b9f9105dcaab4984fd70e4ea0d97b0b9986d9f1`。全部提交均在当前 HEAD 可达。
- 比较方式为上轮代码 target `8539d18…` 到当前 HEAD `c434577…` 的累计净差异；其中 `0b5b37d…` 和 `c434577…` 仅回填计划 SHA，不是额外实现。没有未登记代码提交。
- 修复范围共 14 个文件：4 个源文件、4 个 Rust 测试，以及计划和 M2/M3/M4/M6/M8 记录。完整读取本次 diff，并追踪 URL 错误输出、Clock 的布局/渲染/产物检查、ERD 文本转义和 Reader 焦点/目录/查找的直接消费者。没有扩为原五类布局或全仓普查。
- 开始时 index 无 staged 差异；tracked 的未提交变更只有计划状态页与评审索引，untracked 为 R1 报告及其状态页，无未提交代码。它们是上轮评审/处置材料，不作为代码 findings 对象。
- 开始时对全部 4,831 个 tracked 文件以及两个 untracked 文件记录逐文件 SHA-256；首次落报告前重新核对全部上述文件及 HEAD，内容均未变；最终交付前再次核对，仅本轮已说明的台账回写发生变化，源码、测试、计划及 R1 报告未变。报告、台账回写及临时复现产物不混入被审范围。

### 实际验证

所有运行验证使用当前源码；未 checkout、reset 或 stash 用户工作树，也未修改源码、原计划或测试。临时复现只写入系统临时目录。

| 命令 / 检查 | 实际结果及限制 |
| --- | --- |
| `npm test`（以 mise 的 Node 25.2.1 执行 npm-cli.js，PATH 首位固定为同一 Node 目录） | exit 0；Node 490 tests，467 pass / 0 fail / 23 skipped；Python 四组 22、24、2、13 tests 均 OK。与 R1 的 npm 子进程落入 Node 19 的限制不同，本轮日志首行及执行路径确认 Node 25.2.1。 |
| `MISE_ENV_CACHE=0 mise exec --no-deps -- node scripts/test-phoenix-diagram-native.mjs`（`test:diagram` 同一入口） | exit 0；装配检查、Rust fmt/clippy 通过；Rust 344 passed / 0 failed / 2 ignored；Node 214 tests，209 pass / 5 skipped；Reader/上游 124/124 pass。该入口明确不覆盖真实浏览器与 provider 会话，本轮局部浏览器检查另列。 |
| 配置固定 `WASM_OPT` 后，`MISE_ENV_CACHE=0 mise exec --no-deps -- node scripts/test-phoenix-ui.mjs`（`test:phoenix` 同一入口） | 未完成，reviewer 主动停止后 exit 1；Node critique 1/1、安装 7/7 通过，原超时未重现；已结束的 Rust 测试组 866 passed / 0 failed / 1 ignored，未结束的组及后续阶段不计通过。停止与未运行范围见验证缺口第 2 项。 |
| 当前 debug 引擎 `validate tree bad-url.json --json` | exit 1，凭据没有进入 stdout/stderr；对应 R1 F-01 的失败输入仍被拒绝。 |
| 当前 debug 引擎 `deliver timeline timeline.json timeline.html --json` | exit 0；2020/2026 标签及详情含年份；真实 Chrome 读取 DOM 并实际查看亮色截图。 |
| 当前 debug 引擎两次单外键 `deliver erd` | 均 exit 0；PK/UK 目标在 SVG title、aria、context 和焦点详情中能区分；两种结果的节点目录各保持一组。 |
| 当前 debug 引擎两次三外键 `deliver erd`；Chrome 1440×900、390×844 焦点详情对照 | 均 exit 0；全部目标存在于属性，但 context 被省略；只改末列引用目标后，两个视口各自的截图字节相同，确认 F-03 剩余影响。 |

复现用 darwin-arm64 debug binary SHA-256 为 `1ee39b5fe8eafe0facfade878e6ed3beca1edc01d09bd3c79b1565d994991ea0`。按 `build.rs` 的 INPUTS 与 canonical JSON 排序重算 170 个源码输入得到上述 `ab4e277d…`，与实际交付回执的 `engine.sourceInputSha256` 一致；它不是旧的最终候选。真实浏览器为 Chrome 155.0.8059.85，运行记录见临时 `reader-results.json`。

临时材料位于本机 `/var/folders/_v/szrkbcq52n363ks1n4jpdwb00000gn/T/phoenix-p2-r2-review-zpb7ikz7/`，包括输入、`reproduce.py`、交付回执、`reader-check.mjs`、`reader-alt-check.mjs`、原始尺寸/样式数据、8 张截图，以及 `npm-test.log`、`diagram-test.log`、`phoenix-test.log`。系统可清理该目录；缺陷的输入条件、命令、代码锚点和可观察结果已经写在本报告中，不依赖临时路径长期可取回。

## 计划符合性

| 承诺 | 本轮结果 |
| --- | --- |
| M4 / V-5，§7 凭据不得进入诊断原文 | F-01 已解决；保留错误类型与拒绝行为，固定文案覆盖原脱敏器无法识别的畸形输入。 |
| M3 / V-4，WORK-F1/AC-2 时间语义 | F-02 已解决；跨年可读日期、详情与共享 Clock 一致，原同年行为未发现新增回归。 |
| M2 / V-3，WORK-F1/AC-2 全部外键目标可读 | F-03 部分解决；单列可读，多列在焦点详情被截断，未达到全部目标可读的关闭条件。 |
| M6 / V-7–V-9 与 M8 / V-12–V-14、§9.3 证据绑定 | 计划如实保持 6/8、M6/M8 进行中；本轮没有更新这些里程碑或把旧包当作新包。完整候选和相关收口仍待后续完成。 |

## 总体结论与修复顺序

验收判定：**需要修改**。

修复效果结论：**未完全解决**。R1 的 3 项 P2 中，F-01、F-02 已解决，F-03 部分解决；本轮无新增独立问题。修复增量的正确性结论为 `patch is incorrect`，依据是 F-03 的已证实剩余可读性缺口，不依据未完成的外部验收猜测代码有错。

先让 ERD 多列/长外键目标在详情中完整可读，补真实 Reader 回归，再逐项复审 F-03 及受影响界面。随后按计划重做 M6/M8 的新引擎与同包候选证据，补齐待核实项；本报告不授权或启动修复、推送、CI 或真实宿主验收。

下一步：`$apply-code-review docs/plan/PHOENIX-DIAGRAM-P2.code-review.r2.md`；修复后 `$review-code docs/plan/PHOENIX-DIAGRAM-P2.md`。
