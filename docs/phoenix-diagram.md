# Phoenix Diagram

Phoenix Diagram 0.2.0 提供 architecture、workflow、sequence、dataflow、lifecycle、erd、class、tree、timeline、waterfall、statechart 十一类的原生生成、独立 HTML 阅读与导出，可对固定提交核对仓库来源，阅读器界面支持 en、zh-CN、zh-TW、es、ko。新稿声明 `meta.input_contract: "phoenix-v1"`；全部类型只有 `phoenix-v1`：原五类未标记的 JSON 报 `input/contract-required`，可用 `--input-contract phoenix-v1` 声明；`legacy-2.17` 不再受支持，原 JSON/HTML 不被迁移。statechart 从有限模型穷举状态、判定规则并画出最短出错路径。仓库以 phoenix-diagram 为唯一活跃图解入口；其他项目及个人旧安装须分别处理。数值图表、仪表盘及其他 infographic 不在范围内。

UI 与 Diagram 共用 [Phoenix CI](../.github/workflows/phoenix.yml)：同一次运行完成两个工具的检查，发布仍使用同一个 `@xgent-ai/skills` npm 包。

## npm 发布安装

npm 发布包与仓库的 `xgent-skills install` 默认同时安装 Phoenix UI 和 Phoenix Diagram，可用 `--no-phoenix-diagram` 单独跳过 Diagram。首次 npm 安装会重装所选 provider 下没有 Diagram 安装记录的旧 skill 副本或软链接；软链接只替换入口，不删除其指向的源目录。已有安装记录交由引擎校验，跳过 Diagram 时保留旧入口。Diagram skill 随 npm 安装器与固定引擎一起安装、更新；`metadata.internal: true` 将它从 `npx skills add XGENT-ai/skills` 的默认列表排除，避免不同渠道覆盖配套版本。维护发布统一从 `main` 运行 `npm run release:phoenix`，不创建发布分支或使用 detached HEAD。该命令从同一次 Phoenix CI run 取 Diagram 与 UI 的四平台引擎（制品 `phoenix-diagram-<平台>`），用同一上传器传到 R2 并回读，再写入 Diagram 的 release 清单与 `ENGINE.json` 副本，和 UI 一起打包；开发清单不能发布。引擎不绑定 npm 版本：源码输入未变时，`--reuse-engines <已发布版本>` 直接复用该版本的 Diagram 与 UI 引擎，见 [Phoenix UI 发布](phoenix-ui.md)。最终 tarball 在 Node 18 与维护 Node 上验证联合安装、Diagram 入口和重复安装后才进入 npm 发布。

## 离线安装

消费需要 Node.js 18+，无需 Rust、Cargo 或源码。离线安装使用同一个 npm 包与匹配的完整 release-dir；不要混用版本或自行改 VERSION/ENGINE。以下 `PACKAGE_DIR` 是包目录，`PROJECT` 和 `RELEASE_DIR` 换成实际绝对路径：

```sh
node "$PACKAGE_DIR/bin/xgent-skills.js" install "$PROJECT" \
  --no-phoenix-ui --providers=claude,codex \
  --diagram-release-dir="$RELEASE_DIR"
```

总安装器默认启用两个 Phoenix 工具；不支持 diagram 的平台默认跳过 diagram 并说明，显式指定 `--diagram-release-dir` 时仍在写项目前失败。`--release-dir` 只控制 UI，`--diagram-release-dir` 只控制 diagram，`--no-phoenix-diagram` 跳过且保留已有 diagram 安装。所有选项、XGENT、UI、diagram 预检均先于项目写入。执行顺序为 XGENT → UI → diagram；后段失败会报告前段已完成项，不回滚另一个工具。

默认按项目已有 provider 目录选择，空项目用 `.claude`。`codex` 识别 `.agents` 或 `.codex`，diagram skill 仅进入 `$PROJECT/.agents/skills/phoenix-diagram`；本工具不新增 hooks、settings、AGENTS 或个人级 skill。新开 Claude Code/Codex 项目会话后核对实际加载的 skill 路径。安装直接覆盖工具文件，不保留安装历史或备份；中断后重跑即可。成功后清理旧版本留下的 `$PROJECT/.phoenix-diagram/install/`，保留图表输出等项目数据。

引擎按 VERSION 的大小、SHA-256 与 probe 校验，缓存位于 `~/.phoenix-diagram/bin/<toolVersion>/<platform>/<engineSha256>/phoenix-diagram`；`PHOENIX_DIAGRAM_HOME` 可隔离缓存。普通命令不联网补装。缺引擎时只在用户明确请求安装后执行回执的 `setupCommand`（doctor 的 `recovery.engineInstall` 使用同一指引）。candidate/development 指向当前本地包或已安装 skill 的入口，不按同名版本去 registry 取另一个包；将命令中的 `MATCHING_RELEASE_DIR` 替换为该候选配套完整 release-dir 的实际绝对路径。用户或已提供材料尚未明确给出该路径时，先询问并停止安装，等待答复；安装授权不代替资源路径，不搜索项目外目录、猜路径、省略 `--release-dir` 试装或执行占位符。该目录的 VERSION.json 须与包内清单逐字节相同，引擎按摘要校验；目录缺少本平台引擎或清单时直接失败，不回退在线下载。只有 distribution=release 才输出 `npm exec --package=@xgent-ai/skills@<固定版本>`。

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

示例只教结构，实际 candidate 应由授权事实写成，不能把示例当作用户的系统信息。所需日期、金额或业务条件缺失时，先询问，答复前不写 candidate 或生成图，也不附“草拟”“默认”“待确认”的猜测值；只有用户明确要求拟定这些值时才提供标为建议的方案。普通依据材料作图时，原材料已明确指出业务条件、日期或金额缺失，就直接走澄清分支；不能以主干仍可画或已有结果标签为由继续作图。其他必需事实未提供时，先提取阻止交付的缺口，改成开放式问题；不发散到材料未提及的可选分支、异常路径或设计选择。已给出的角色、关系和标签直接按原文使用，不以细化称谓、层级或内部细节为由追加问题。例如，“超预算还需批准”只说明追加条件；材料未要求具体执行先后时，不补造先后，也不为细化先后追加问题。发送前删去候选答案、假设例子、扩展问题、流程复述，以及列表前后汇报已核事实或解释为何要问的段落，只输出剩下的简短问题列表并结束，从第一条问题开始，列表前后不加介绍或解释；字段格式只给未赋值占位符，不追加创作步骤、草拟流程或具体值，也不用缺失规则卡片代替询问。事实齐备后读取匹配的 schema/common/example 与创作默认值；宿主读取返回被截断时，分段补齐必需内容后再写 candidate，不以子进程 stdout 完整代替宿主实际可见读取。新稿默认 showcase，先自动路线；Mermaid 的 flowchart/graph、sequenceDiagram、stateDiagram、erDiagram、classDiagram 由宿主保留语义后写 JSON，Rust CLI 不包含 Mermaid parser。十类之外的结构应明确提出，不靠删节点、删列、删成员、删关系、缩字或隐藏溢出来通过。

ER 图创作前分别核对关系每端的最大基数、最小参与数与引用列可空性。外键只证明引用目标，不自动证明 `NOT NULL`；“可有多个”不单独证明最小数为零。`fromOptional`/`toOptional` 省略或写 `false` 仍表示最小值1，写 `true` 表示0；两种取值都要有材料依据，省略字段不能表示未知。以已给 SQL 约束、联合主键、显式文字或 Mermaid 端点为依据，材料已明确就直接使用，不重复询问或要求另选表现形式；需要表示的最小数或可空性缺少依据时，先按上述开放问题规则询问并停止，不生成 candidate 或先画 `0..N`/`1` 再标“待确认”，也不用卡片兜底。用户明确要求拟定规则时才提供建议。

类图的关系种类与数量约束分别依据材料表达。成员可脱离整体存在，不等于整体可以没有成员；未给下界时保留“多个”等已有数量含义，不补 `0..*`、`1..*` 等数值标注，只省略无依据的可选数值。“一个/单个/多个”等已给数量须由关系标签或相应的单值/集合成员明确表达，不能只保留组合或创建销毁文字而漏掉数量。明确数值与 Mermaid 端点照用，不为可选标注追加问题。

耗时瀑布分别核对跨度结束、trace 采集截止与显示窗口。请求或父跨度的 `end` 不自动给出 trace 的精确采集截止时刻；仅由自动扩展或人为固定得到的窗口边界不能作为观测事实。未给精确截止时，不在名称、`detail`、卡片、提示或最终答复中补精确截止时刻或截至该时的精确已观测时长。未结束的跨度保持 `incomplete`，有记录支持的区间可表达“至少/≥”时长下界：同一 trace 已记录的最晚时刻可给出采集截止下界，采集截止时仍开放的跨度可据其起点到该时刻的差给出时长下界，无须知道精确截止；不能由此补精确截止或最终耗时，缺少可省略的截止说明不追加问题。推导时长比较、极值或排序结论时，以材料已给事实、已知数值或上下界为依据，不能默认未结束跨度短于已完成跨度。材料直接给定的比较事实照用，不为证明它额外索要数值。上下界足以证明时，未知总时长也可参与比较；否则写清仅比较已知完成跨度或明确窗口内片段，不把该结论扩为全部跨度的最终耗时比较。判断无子跨度覆盖的区间时，核对相关未结束子跨度；缺少其结束依据就不作零覆盖断言。

审批有多个继续分支时，逐条保留各分支所需的全部批准前提；某一分支写有批准，不代表其他仅按资格分流的分支也已获准。逐个从获准结果沿实际可达分支回溯，核对每条路径的条件是否足以推出该结果所需的全部批准；不能只因有拒绝出口，就把其余资格分流边视为已批准。源码或图中的等价条件表达同样有效，不要求固定批准词或独立批准节点、边。

新稿或涉及语义的修复完成后，按原材料逐分支走读条件、动作、执行者与先后关系。进入处理步骤不等于已经通过或获准；资格条件与批准结果分别核对。原文要求某人批准后才能执行的动作，须由该执行者的批准节点或出边中的批准条件控制，不能仅以进入审批和满足资格条件代替。条件内的动作不能前移为其他分支的共同前置动作，也不能因合并节点或回路改变顺序。跨阶段适用的动作，先按原文枚举许可覆盖的全部非终态，再逐项核对主图路径；例如“任一下单前可撤回”也覆盖提交、补充材料等适用阶段。每个适用阶段都应能行使该许可并抵达相应结果，无需先满足额外条件或完成审批、提交、补交等其他业务动作。无条件且不执行业务动作的瞬时起始跳转可与后续阶段等价，显式覆盖全部适用阶段的结构也可等价表达，不要求逐节点各画一条直接边。有条件出口或执行检查的阶段不属于这种瞬时起始跳转。例如，材料给“合并前可撤回”时，图中单独表示的批准阶段或获批后尚未合并的阶段也要保留撤回路径，不能在审批通过时提前结束这项许可。卡片或“各阶段可……”文案不能代替主图中这项许可的完整范围。节点文字、`tag`/`sublabel`、卡片标题与条目、线型、颜色与图例都要对应原事实。未说明的同步/异步、批处理等执行方式，使用中性职责、动作与普通关系；不能仅凭 Queue、Worker 等名称推定节点任务的执行方式，也不为可省略的标注追加问题。组件的用途同样须有材料依据，不能按常见用法补充；例如，材料只给“读写 Redis”时，不标为“缓存”。节点和标签齐全、卡片复述正确，都不能代替主图路径与原事实一致；只据已给事实修正，必需事实缺失时仍先澄清。

仓库图缩写文字、改写卡片或调整布局后，也要保留实际命令名、源码条件和替代路径，所用函数或校验名称仍与各入口、命令的条件对应；只在特定分支执行的校验，不能写成所有调用必经的前置条件或安全保证。合并不同入口、命令或条件分支时，共享前置只能放这些实际可走路径共同必经的动作；例如 A 入口执行 X、B 入口执行 Y，不能合并为两者都执行 X。分别沿源码核对有条件的校验、默认补足和失败返回，保留各自的条件与结果。沿调用链把实际传入的参数和禁用开关带入条件判断；可被入口参数跳过的校验，不能写成必经步骤或无条件保证。对短路、跳过、禁止或必经的断言，逐条核对实际分支的完整条件；源码存在一个提前返回分支，不代表相关入口或不同参数组合都走该分支。来源核验为 `verified` 也不能代替这项语义核对。

Workflow 发生共用通道或箭头拥挤时，按实测节点与整组相关路线，先给共用起点或终点周围留出独立通道；必要时调整未固定节点的列、行或间距，包括回路起点、多入边的汇合与撤回目标。端点可在图的边缘或内部，以所有相关路线的实测结果为准，不由主路径顺序预定位置；不要为了保留拥挤布局而先叠加端口或通道 pin。自己添加的端口或路线约束冲突时，撤掉这些约束，让自动路线重新联排相关边；剩余实测问题再按诊断修复，保持原业务事实、质量和停滞停止规则。

公开命令：

用户或宿主对 CLI 执行器、命令形式或原始结果留存有明确约束时，所有 CLI 调用（含 `guide` 和帮助命令）都按这些约束执行。下方示例只说明参数，不能据此省略任务已明确要求的调用层或结果记录。命令形式已固定时，不自行添加环境变量赋值前缀或更换调用层。资料范围已限定时，搜索也须限于获准资料；不能用整个 skill 根的搜索读到已排除的实现，排障也不枚举授权外的个人缓存或应用目录。

```text
validate <type> <candidate.json> [--layout-json] [--quality standard|showcase] [--input-contract NAME] [--repo-root DIR] --json
deliver <type> <candidate.json> <output.html> [compile options] [--repo-root DIR] --json
check <output.html> --json
browser-check <output.html> [--out-dir DIR] --json
visual-check <output.html> [--out-dir DIR] --json
finalize <type> <candidate.json> <output.html> [compile options] [--repo-root DIR] [--out-dir DIR] --json
review-record <output.html> --record <review.json> --json
install --project DIR --bundle-root DIR [--providers claude,codex] [--dry-run] --json
```

显式 quality/input-contract 与 candidate 已声明值冲突会失败。保留旧合同默认值、固定节点、方向、标签与 route pins。品牌由固定目录或摘要固定的本地资源支持；旧 guided views 为 `unsupported-display`，旧播放偏好为 `ignored-display`，首次仍是 Still。这些终局处理列在 `compatibility.dispositions`，不再称为延期，核心兼容通过不能代替完整展示兼容。见[兼容约定](../skills/phoenix-diagram/references/compatibility.md)。普通 stdout 为一个 JSON，显式 preview 为 NDJSON 事件流；不要仅用退出码判断完整交付。

主要元素带 `sources`、`meta.repository` 固定 40 位提交时，用 `--repo-root` 指向该仓库顶层目录。核对只读、不联网：origin 一致、提交存在、文件与行在该提交中存在，并逐段比较工作树文件（不跟随符号链接）。凭据 URL、路径逃逸、非法行区间、provider 与主机不符或网页链接用了不支持的主机会在输入阶段失败。其他不能核对的情形（缺 `--repo-root`、git 不可用或超时、缺提交或对象、文件缺失、行越界、工作树不同、旧版 git 遇部分克隆）只把对应来源记为待核实并附原因，HTML 照常生成；回执与 summary 的 `evidence.status` 为 `verified`、`partial`、`unverified` 或 `none`，与自动门结论分开报告。时间线的日期型记录用 `precision: "day"`，没有记录的耗时瀑布用 `meta.evidence: "illustrative"`。

每轮失败后，在可回读的进度或本次项目记录中写明实际失败门、诊断阶段、客观 error 数、历史最佳值和停滞计数；首次失败的停滞计数为0。同一门每次修复后，只有 error 低于该门历史最佳才更新最佳并清零，否则停滞计数加1。修改 candidate 或修复前先核对该门计数：达到2立即停止并列出未解决项；即使不再运行命令，也不能继续修改 candidate 或将它回退到历史较好版本。错误种类变化不能重置计数，也不允许再试一次。修复前实际读到失败 diagnostic 的 `subject`、`evidence`、`supportedFixes`；宿主返回截断或提取遗漏时先补齐，不能把原有实测或字符串修法筛掉、转成空对象后视为已读。只修相关邻域。重建 candidate 或交付、调整布局或卡片、返回已记录门时，沿用该门历史，不因新回执重新初始化；对最小示例的只读对照或验证不计修复，也不重置历史。环境失败也不允许无限重试。已通过前门而首次进入后门属于实际进展，按新门初始化记录；同一 validate 门内诊断 `stage` 首次由 `input` 变为 `layout` 也算进入后门。`supportedFixes` 写明经整图验证时，选定的一项修法所列全部路径是同一整组方案，须一次完整应用，不挑单个路径先试；保留每轮全部门的真实状态，不把不同门的顶层 error 数拼成同一停滞序列。

## 看图后收口

`finalize` 顺序执行 validate、deliver、browser-check 三道自动门，首个失败后其余 `not-run`。它写出 HTML、截图（默认在 HTML 旁，`--out-dir` 可指定）和 HTML 旁唯一的 `<名称>.finalize-summary.json`；重跑会覆盖它们。浏览器默认隔离验收并采图，不打开用户当前浏览器；工具缺失、执行失败、自动通过与视觉待审分别报告。HTML 写出后浏览器失败会保留新 HTML。

自动三门通过后仍为 `visualReview.status: required-pending`、`deliveryStatus: incomplete`。实际查看 summary `screenshots` 列出的 1440×900、2048×1320 四张暗亮截图，检查主图、全部标签/notes、卡片、关系遮挡、层级和控件，按[交付合同](../skills/phoenix-diagram/references/delivery-contract.md)写审核记录；未看图不得设置 passed。

```sh
"$PD" review-record diagram.html --record review-01.json --json
```

review-record 只接受自动三门已通过、且 HTML 仍与 summary 记录一致的交付，并把审核写回同一 summary。三门 pass、`visualReview.status: passed`、`deliveryStatus: complete` 才是完整交付。答复中的 HTML、candidate 和 summary 路径优先原样复制回执给出的真实值。若转为相对路径，先以宿主实际报告的本次工作目录逐条核对它们仍指向相同文件，再输出这些已核对的字符串；不能拿上级目录或消息中出现的其他目录作相对基准。简短回复也要保留定位所需的目录，不能用 `...` 代替路径或裁去前缀冒充绝对路径。只为登记审核不得再次 finalize；修改 candidate 则重新 finalize 并重新看图审核。另读取 summary 中名为 `deliver` 的阶段回执 `receipt.compatibility.coreCompatibility` 与 `receipt.compatibility.dispositions`，逐项交接非 `supported` 字段及实际原因。`deliveryStatus: complete` 仅表示交付流程完成，不能代替这些兼容限制的交接。

## 按请求启用的作者工具

- [品牌资源](../skills/phoenix-diagram/references/brand-marks.md)：明确要求品牌标识时查询固定目录，只有明确采集请求才访问用户提供的公共 URL。普通编译只读本地固定资源，不因节点名称猜品牌或联网。
- [本地预览](../skills/phoenix-diagram/references/preview.md)：明确请求才启动，`--open` 才打开系统浏览器；保留 last-good 并报告新尝试错误。服务停止和页面可用不等于正式交付通过。
- [架构比较](../skills/phoenix-diagram/references/architecture-compare.md)：明确请求比较两份 architecture 时执行，提供 Before/Delta/After，走 visual-check、12 张截图实际审核，结果记在比较 summary，不推断原因、风险或可合并性。

普通作图继续使用单图 finalize 与四图审核，不自动附带上述操作。维护者增加类型时遵守[扩展约定](../skills/phoenix-diagram/references/extending-types.md)。

## 阅读与导出

HTML 内置 Reader、字体与必要资源，可离线阅读。支持主题/风格、平移缩放、标签/id/来源路径搜索及详情；首次为 Still，用户选择 Live 后才保存偏好。十一类都可做语义筛选、概览定位、展示模式和深链接；有关系的九类（statechart 的箭头由模型推导）可找 Route 与上/下游 Reach，并导出标明范围、非 canonical 的范围卡片；树可折叠分支，导出与打印始终为完整树。详情按已核对、待核实（附原因）、未提供来源三态显示来源。界面语言按 `meta.locale` 选择五种之一，其他标签回退英文，finalize 回执的 `locale` 记录结果。缩减动画、页面隐藏、打印和导出临时暂停，静态图仍保留含义。页面可按声明的可读纵向画布滚动，不能用水平裁切或内部滚动掩盖内容。

支持 PNG/JPEG/WebP、主题 SVG、WebM 和 PNG 复制。能力/权限拒绝、空 Blob、编码/录制失败都应返回明确失败与可用替代动作；格式实际可用性以当前浏览器结果为准。字体许可随独立 HTML/SVG 保留。源码与第三方许可位于独立发行目录 LICENSE、NOTICE、THIRD-PARTY.json/NOTICES/LICENSES；参考归属与 Phoenix 修改单列。
