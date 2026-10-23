# Phoenix Diagram 候选

Phoenix Diagram 0.2.0 提供 architecture、workflow、sequence、dataflow、lifecycle、erd、class、tree、timeline、waterfall 十类的原生生成、独立 HTML 阅读与导出，可对固定提交核对仓库来源，阅读器界面支持 en、zh-CN、zh-TW、es、ko。新稿声明 `meta.input_contract: "phoenix-v1"`；原五类未标记的旧 JSON 按 `legacy-2.17` 解释，新五类只有 `phoenix-v1`，原 JSON/HTML 不被迁移。它与 archify 并存。数值图表、仪表盘及其他 infographic 不在范围内。

目前实现与验收仍在进行，四平台、真实 provider 和最终包的完整交接尚未完成。candidate 的 loopback 下载地址仅用于隔离验收；本次没有公开 npm/R2 发布。

## 候选安装

消费需要 Node.js 18+，无需 Rust、Cargo 或源码。候选交付包含同一个 npm tgz 与匹配的完整 release-dir；不要混用版本或自行改 VERSION/ENGINE。以下 `PACKAGE_DIR` 是该 tgz 的解包目录，`PROJECT` 和 `RELEASE_DIR` 换成实际绝对路径：

```sh
node "$PACKAGE_DIR/bin/xgent-skills.js" install "$PROJECT" \
  --no-phoenix-ui --no-xgent-init --providers=claude,codex \
  --diagram-release-dir="$RELEASE_DIR"
```

这里跳过 UI 是候选消费范围：包内 UI 原资源保持原样，不能把它当作 diagram 四平台发行的证据。总安装器默认启用两个 Phoenix 工具；不支持 diagram 的平台默认跳过 diagram 并说明，显式指定 `--diagram-release-dir` 时仍在写项目前失败。`--release-dir` 只控制 UI，`--diagram-release-dir` 只控制 diagram，`--no-phoenix-diagram` 跳过且保留已有 diagram 安装。所有选项、XGENT、UI、diagram 预检均先于项目写入。执行顺序为 XGENT → UI → diagram；后段失败会报告前段已完成项，不回滚另一个工具。

默认按项目已有 provider 目录选择，空项目用 `.claude`。`codex` 识别 `.agents` 或 `.codex`，diagram skill 仅进入 `$PROJECT/.agents/skills/phoenix-diagram`；本工具不新增 hooks、settings、AGENTS 或个人级 skill。新开 Claude Code/Codex 项目会话后核对实际加载的 skill 路径。安装状态位于 `$PROJECT/.phoenix-diagram/install/`，请按输出提示在本地忽略；不要删除整个目录作为恢复手段。

引擎按 VERSION 的大小、SHA-256 与 probe 校验，缓存位于 `~/.phoenix-diagram/bin/<toolVersion>/<platform>/<engineSha256>/phoenix-diagram`；`PHOENIX_DIAGRAM_HOME` 可隔离缓存。普通命令不联网补装。缺引擎时只在用户明确请求安装后执行回执的 `setupCommand`（doctor 的 `recovery.engineInstall` 使用同一指引）。candidate/development 指向当前本地包或已安装 skill 的入口，不按同名版本去 registry 取另一个包；将命令中的 `MATCHING_RELEASE_DIR` 替换为该候选配套完整 release-dir 的实际绝对路径。用户或已提供材料尚未明确给出该路径时，先询问并停止安装，等待答复；安装授权不代替资源路径，不搜索项目外目录、猜路径、省略 `--release-dir` 试装或执行占位符。该目录按当前 VERSION/ENGINE 的原字节、引擎摘要和许可校验；遗失候选包或配套目录时须重新取得同一 tgz 和资源。只有 distribution=release 才输出 `npm exec --package=@xgent-ai/skills@<固定版本>`。candidate/release 离线目录必须含四平台；development 只校验清单声明的全部平台资源，单平台开发制品不计四平台验收。目录缺少声明的资源、清单或许可证时直接失败，不回退在线下载。

## 从候选到 HTML

以下在已安装项目根目录执行；只有 Claude provider 时把 `.agents` 换为 `.claude`：

```sh
PD="$PWD/.agents/skills/phoenix-diagram/scripts/phoenix-diagram"
"$PD" --help
"$PD" doctor --json
"$PD" guide "审批工作流" --json
cp .agents/skills/phoenix-diagram/examples/minimal.workflow.json candidate.json
"$PD" finalize workflow candidate.json diagram.html --out-dir evidence-01 --json
```

示例只教结构，实际 candidate 应由授权事实写成，不能把示例当作用户的系统信息。所需日期、金额或业务条件缺失时，先询问，答复前不写 candidate 或生成图，也不附“草拟”“默认”“待确认”的猜测值；只有用户明确要求拟定这些值时才提供标为建议的方案。普通依据材料作图时，原材料已明确指出业务条件、日期或金额缺失，就直接走澄清分支；不能以主干仍可画或已有结果标签为由继续作图。其他必需事实未提供时，先提取阻止交付的缺口，改成开放式问题；不发散到材料未提及的可选分支、异常路径或设计选择。已给出的角色、关系和标签直接按原文使用，不以细化称谓、层级或内部细节为由追加问题。发送前删去候选答案、假设例子、扩展问题和流程复述，只输出剩下的简短问题列表并结束；字段格式只给未赋值占位符，不追加创作步骤、草拟流程或具体值，也不用缺失规则卡片代替询问。事实齐备后只读匹配的 schema/common/example 即可开始。新稿默认 showcase，先自动路线；Mermaid 的 flowchart/graph、sequenceDiagram、stateDiagram、erDiagram、classDiagram 由宿主保留语义后写 JSON，Rust CLI 不包含 Mermaid parser。十类之外的结构应明确提出，不靠删节点、删列、删成员、删关系、缩字或隐藏溢出来通过。

ER 图创作前分别核对关系每端的最大基数、最小参与数与引用列可空性。外键只证明引用目标，不自动证明 `NOT NULL`；“可有多个”不单独证明最小数为零。以已给 SQL 约束、联合主键、显式文字或 Mermaid 端点为依据，材料已明确就直接使用，不重复询问或要求另选表现形式；需要表示的最小数或可空性缺少依据时，先按上述开放问题规则询问并停止，不生成 candidate 或先画 `0..N`/`1` 再标“待确认”，也不用卡片兜底。用户明确要求拟定规则时才提供建议。

类图的关系种类与数量约束分别依据材料表达。成员可脱离整体存在，不等于整体可以没有成员；未给下界时保留“多个”等已有数量含义，不补 `0..*`、`1..*` 等数值标注，只省略无依据的可选数值。明确给出的数量与 Mermaid 端点原样保留，不为可选标注追加问题。

新稿或涉及语义的修复完成后，按原材料逐分支走读条件、动作、执行者与先后关系。进入处理步骤不等于已经通过或获准；条件内的动作不能前移为其他分支的共同前置动作，也不能因合并节点或回路改变顺序。跨阶段适用的动作，先按原文枚举许可覆盖的全部非终态，再逐项核对主图路径；例如“任一下单前可撤回”也覆盖提交、补充材料等适用阶段。每个适用阶段都应能行使该许可并抵达相应结果，无需先满足额外条件或完成审批、提交、补交等其他业务动作。无条件且不执行业务动作的瞬时起始跳转可与后续阶段等价，显式覆盖全部适用阶段的结构也可等价表达，不要求逐节点各画一条直接边。卡片或“各阶段可……”文案不能代替主图中这项许可的完整范围。线型、颜色与图例要对应原事实；未说明的同步/异步、批处理等执行方式，用不承诺该含义的普通关系表达。节点和标签齐全、卡片复述正确，都不能代替主图路径与原事实一致；只据已给事实修正，必需事实缺失时仍先澄清。

公开命令：

```text
validate <type> <candidate.json> [--layout-json] [--quality standard|showcase] [--input-contract NAME] [--repo-root DIR] --json
deliver <type> <candidate.json> <output.html> [compile options] [--repo-root DIR] --json
check <output.html> [--strict-provenance] --json
browser-check <output.html> [--out-dir DIR] --json
visual-check <output.html> [--out-dir DIR] --json
finalize <type> <candidate.json> <output.html> [compile options] [--repo-root DIR] [--out-dir DIR] --json
review-record <output.html> --record <review.json> --json
recover-output <private-recovery-directory> --json
install --project DIR --bundle-root DIR [--providers claude,codex] [--dry-run] --json
```

显式 quality/input-contract 与 candidate 已声明值冲突会失败。保留旧合同默认值、固定节点、方向、标签与 route pins；延期到 P3 的品牌、guided views 与旧播放偏好在兼容回执中逐项列出，不代表完整兼容已通过。普通 stdout 为一个 JSON；不要仅用退出码判断完整交付。

主要元素带 `sources`、`meta.repository` 固定 40 位提交时，用 `--repo-root` 指向该仓库顶层目录。核对只读、不联网：origin 一致、提交存在、文件与行在该提交中存在，并逐段比较工作树文件（不跟随符号链接）。凭据 URL、路径逃逸、非法行区间、provider 与主机不符或网页链接用了不支持的主机会在输入阶段失败。其他不能核对的情形（缺 `--repo-root`、git 不可用或超时、缺提交或对象、文件缺失、行越界、工作树不同、旧版 git 遇部分克隆）只把对应来源记为待核实并附原因，HTML 照常生成；回执与 summary 的 `evidence.status` 为 `verified`、`partial`、`unverified` 或 `none`，与四门结论分开报告。时间线的日期型记录用 `precision: "day"`，没有记录的耗时瀑布用 `meta.evidence: "illustrative"`。

首次失败时按实际失败门记录客观 error 数、历史最佳值和停滞计数0；同一门每次修复后，只有 error 低于该门历史最佳才更新最佳并清零，否则停滞计数加1。计数达到2立即停止并列出未解决项；错误种类变化不能重置计数，也不允许再试一次。重建交付或返回已记录门时，沿用该门历史，不因新回执重新初始化；环境失败也不允许无限重试。已通过前门而首次进入后门属于实际进展，按新门初始化记录；同一 validate 门内诊断 `stage` 首次由 `input` 变为 `layout` 也算进入后门。`supportedFixes` 写明经整图验证时，按它列出的全部路径一并修改；保留每轮全部门的真实状态，不把不同门的顶层 error 数拼成同一停滞序列。

## 看图后收口

`finalize` 顺序执行 validate、deliver、严格 check、browser-check。首个失败后其余 `not-run`。浏览器默认隔离验收并采图，不打开用户当前浏览器；工具缺失、执行失败、自动通过与视觉待审分别报告。HTML 已提交后浏览器失败会保留新 HTML，不替换回旧文件。

自动四门通过后仍为 `visualReview.status: required-pending`、`deliveryStatus: incomplete`。读取返回的 summary/browser receipt，实际查看 1440×900、2048×1320 的四张暗亮截图与必要长图，检查主图、全部标签/notes、卡片、关系遮挡、层级和控件。按包内 [交付合同的完整草稿命令](../skills/phoenix-diagram/references/delivery-contract.md) 从当前回执复制身份，逐张填写实际观察；未看图不得设置 passed。

```sh
"$PD" review-record diagram.html --record review-01.json --json
```

随后读取返回的 `evidence.summaryReceipt.path`。四门 pass、视觉 passed、`deliveryStatus: complete` 才是完整交付；原 `deliveryReceiptId`、`browserReceiptIdentity`、输入/HTML/config/engine/四图绑定必须保持。只为登记审核不得再次 finalize。封存记录已成功但 summary 写失败时，以同一记录字节重试 review-record；修改 candidate 则生成新交付、新截图与新审核。不能拿旧图或旧回执补证。

## 中断后的文件恢复

生成失败、竞争占位和提交后失败分别查看 diagnostics/commitState/publicState。若发布进程被 kill、终端输出丢失，先在**原输出父目录**只读定位持久记录：

```sh
ls -la "<原输出父目录>"
```

同一事务的记录按清理进度可能位于三处：

1. `.phoenix-diagram-recovery-<id>/publication-recovery-v1.json`
2. `.phoenix-diagram-recovery-<id>.cleanup.json`
3. `.phoenix-diagram-output-cleanup/<id>.json`

读取记录中的 target/requestedPath/commitPath，明确它绑定本次输出。固定 cleanup 目录还保留绑定的 `<原输出父目录>/.phoenix-diagram-output-cleanup/owner-v1.json`；空壳不是需要删除的垃圾。多个记录须明确选择，不能按时间自动挑选，不手改记录/备份/权限。

**无论记录在哪一处，参数始终为原 `.phoenix-diagram-recovery-<id>` 目录，即便它已不存在：**

```sh
"$PD" recover-output "<原输出父目录>/.phoenix-diagram-recovery-<id>" --json
```

该入口只处理输出 publication，不接受项目 install journal。它核验原父目录、槽位、记录、备份及物理身份并取得同一 OS 锁；仅在缺失 public 时以 no-clobber link 恢复旧文件。后来的 claimant、改过的备份/记录、替换父目录、未知 owner/额外条目或多处记录歧义均保留并停止，不覆盖用户后来修改。恢复过程中再次被 kill，可重复显式恢复同一事务。

| recoveryStatus | 含义 | 退出码 |
| --- | --- | --- |
| recovered | 原字节已恢复或核对一致，且本次清理完成 | 0 |
| absent | 对应事务与残留不存在，无文件恢复 | 0 |
| preserved | 新占位或已提交新图被保留，未恢复旧字节 | 3 |
| recovery-required | 可信事务仍待恢复/清理 | 3 |
| invalid | 参数或不支持的记录格式，读取 diagnostics | 2 |
| failed | I/O 或运行失败，读取 diagnostics | 4 |

返回的 `target` 只在可信记录核验后提供。文件恢复不生成 delivery/browser/visual 通过回执，后续交付仍需适用检查。项目安装中断则重跑原显式 install，让 Rust 在日志与 after 身份仍匹配时恢复；candidate/development 安装回执的 `recoveryCommand` 使用本次已验证的原生引擎与 bundle，不从 registry 另取同名包。引擎缓存也已遗失时先按上文恢复匹配引擎。用户后来修改不会被 before 备份盲覆盖。

## 阅读与导出

HTML 内置 Reader、字体与必要资源，可离线阅读。支持主题/风格、平移缩放、标签/id/来源路径搜索及详情；首次为 Still，用户选择 Live 后才保存偏好。十类都可做语义筛选、概览定位、展示模式和深链接；有已著关系的八类可找 Route 与上/下游 Reach，并导出标明范围、非 canonical 的范围卡片；树可折叠分支，导出与打印始终为完整树。详情按已核对、待核实（附原因）、未提供来源三态显示来源。界面语言按 `meta.locale` 选择五种之一，其他标签回退英文，finalize 回执的 `locale` 记录结果。缩减动画、页面隐藏、打印和导出临时暂停，静态图仍保留含义。页面可按声明的可读纵向画布滚动，不能用水平裁切或内部滚动掩盖内容。

支持 PNG/JPEG/WebP、主题 SVG、WebM 和 PNG 复制。能力/权限拒绝、空 Blob、编码/录制失败都应返回明确失败与可用替代动作；格式实际可用性以当前浏览器结果为准。字体许可随独立 HTML/SVG 保留。源码与第三方许可位于独立发行目录 LICENSE、NOTICE、THIRD-PARTY.json/NOTICES/LICENSES；参考归属与 Phoenix 修改单列。

维护验收的三变量 checkpoint 仅用于隔离临时项目的确定性 SIGKILL 场景，不是普通恢复步骤；默认不启用、不联网、不延迟消费命令。当前里程碑和实际验收记录见仓库开发计划，缺失的真实平台/provider/图像结果不计通过。

## 验证结果与发行边界

阶段A在本机 macOS arm64 开发引擎上，十类共15例（原五类的新/旧合同各一例、新五类各一例）通过四道自动门和确定性检查，实际查看60张暗亮截图后经原身份 `review-record` 达到完整交付。15例的Reader操作、20项导出、打印与剪贴板检查，离线生成/检查/阅读导出和8个公开中断恢复场景均已通过。十类探索（筛选、Route、Reach、树折叠、概览、展示、深链接、键盘与详情）通过，有已著关系的八类各导出一张Route与Reach范围卡片。冻结3.1.0参考包的25份JSON核心事实可被消费，原样交付均通过四道自动门并经实际看图，原JSON与15份参考HTML字节不变。本仓固定提交的来源核对覆盖已核对（含GitHub链接存在性）、工作树改动、仅本地和未提供来源四种显示，示意耗时与日精度时间线已核对。五种界面语言与fr英文回退均实际看图。这些开发结果不代替最终 npm tgz 的消费验收。

最终候选须另附 `kind: phoenix-diagram-candidate-aggregate`、`profile: "p2"` 的聚合回执。先核对其中 `cohort.tgzSha256`、source身份、runId/runAttempt与收到的候选一致，再读取以下三类实际结果：
- 四个平台与精确Node18.20.8对同一tgz的自动消费：15例四门全过，视觉保持待审。
- darwin-arm64本机对同一tgz的完整消费：15例实际看图后 `review-record`，以及探索与范围卡片、来源与界面语言范围。
- Claude Code、Codex真实会话：它们在bundle与引擎来源都相同的development包上取得。

完整Reader范围包含相机、空搜索、按标签和id搜索、无匹配反馈、详情、键盘与样式八项实际交互，双主题及Still/Live下缩减动画、页面隐藏的八项真实暂停/恢复样本，以及20项导出、剪贴板和打印暂停/恢复的证据；截图存在或单项pass不能替代这些范围。任何缺项、not-run、unavailable或身份不一致都保持incomplete；最终 `status: pass` 且 `handoff: passed` 才表示该候选的规定验收已齐备。

本说明在最终封包前固定，封包后的验收结果保存在绑定原tgz摘要的独立聚合回执中。补写结果或修复包内文件会改变tgz身份，需要重新装配并取得适用的新证据。当前尚未取得最终同包四平台及真实provider的完整通过结果，也没有公开npm/R2发行；development清单和候选loopback地址不能用于宣称公开在线安装已可用。
