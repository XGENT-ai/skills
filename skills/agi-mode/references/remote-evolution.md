# 指令进化的远端执行

需要在可重放的代码或文档任务上比较、搜索 skill/仓库 AGENTS 时读取。每类任务都须有匹配的公开契约和质量量表。Eval 继续持有判定，Hillclimb 继续持有搜索；普通使用 agi-mode 不依赖本工具。

v1 工具已通过本仓本地门及指定 SSH/Docker 部署的真实隔离、校准、对照/搜索、自然请求/CAS 和故障恢复验收；当时独立确认样本不足，结果 inconclusive。v2 的质量评价和 TypeSafe 辅助反馈另行验收，不能沿用 v1 的验收结论；新任务与运行组合仍需单独校准。

## 安装与入口

定位实际安装的 agi-mode；以下 `<agi-mode>` 替换为该目录。工具随 skill 携带，使用自己的 Python 依赖锁，不要求修改 Node 安装器或用户全局配置。

```bash
uv sync --locked --project <agi-mode>/scripts/evolution
uv run --locked --project <agi-mode>/scripts/evolution xgent-evolve validate --spec /path/to/job.json
uv run --locked --project <agi-mode>/scripts/evolution xgent-evolve run --spec /path/to/job.json --output /path/to/new-run
uv run --locked --project <agi-mode>/scripts/evolution xgent-evolve resume --run /path/to/existing-run
uv run --locked --project <agi-mode>/scripts/evolution xgent-evolve apply --run /path/to/existing-run --repo /path/to/target-repo
```

resume 从原运行目录保全的协议、字节和 runner 恢复，不重新读取原始任务目录或复用模型会话。它先查询全部已登记执行，再接续未提交工作；身份、启动窗口或原退出不明时保存恢复缺口并停止，不自动重新支付。已捕获的基础设施失败按事前 max_retries 另记尝试，原失败仍在。

容器内收据暂不可读时，对同一执行最多查询三次；持续不可读仍为 unresolved。这个读取重试不创建新执行，也不重启原命令。

需要 Python 3.12+、uv、SSH，以及运行者已经提供的远端 Docker 和固定 Codex 运行条件。本机只做确定性工作，模型和判定都在 SSH 主机的独立容器中运行；不提供本机模型 backend。

当前运行组合固定 Harbor 0.24.0、Codex CLI 0.162.1，模型/effort 按原调用授权显式配置。远端执行器的 CPU 架构与任务镜像的平台分别记录：arm64 Docker 引擎可运行公开 amd64 任务镜像，Codex 及其沙箱辅助程序使用引擎原生架构，避免在 QEMU 中建立用户命名空间。

`case.platform`缺省为`linux/amd64`。仅v2且`case.source.kind=repo`的自有任务可显式选`linux/arm64`，须提供与平台匹配的固定镜像并重新建立运行/质量基线；v1和Pro V2任务仍只接受amd64。执行器使用声明的平台，不自动改架构或替换官方任务环境；不同平台的历史分数和资格不互相迁移。

运行者负责已有主机、镜像、登录与配额。工具不重建 VM、不自动修改内核或全局配置；命名空间/网络能力缺失时停止并报告。临时宿主配置也须有明确授权及恢复办法。

runtime.apparmor_profile 可指定远端已经加载的 profile 名；工具仅绑定本次容器并核对实际值，不负责加载或修改策略。未填写时使用 unconfined，实际能否建立 Codex 用户命名空间仍由探针决定。固定 Codex 安装产物可保存在 remote_root/tools 下，复用前核对完整字节/权限/内部链接清单；该缓存不含登录、配置或会话，也不挂载到容器。

v2私有状态的根层glob显式定位Codex目录，避免rg把文件名模式递归匹配到已被拒绝的会话目录内，再次遮罩子文件而阻止工具启动。目录及私有文件拒绝、公有代理CA读取和网络限制保持。新环境除了无工具的登录回应，还应验证真实shell读取公开输入、写入获准输出，以及已有会话状态仍不可读。

首次安装不要求任务镜像已有 Node/npm：缺少它们时，由 Python 获取固定版本及架构的官方 npm 平台包，核对发布元数据与 SHA-512，再安全解包。实际 Codex 版本、二进制哈希和隔离结果仍由探针核对。

已有网络代理确有需要时，可显式填写 runtime.model_proxy（如 `http://proxy-host:7890`），不含认证、路径或查询参数。它只传给模型驱动；工具环境继续使用过滤与独立网络限制，不继承宿主环境或代理配置。工具不安装代理，也不修改系统网络设置。

## 输入与可见性

JobSpec 支持 schema_version=1/2，未知版本或字段拒绝。v1 保留原二元指标；v2 使用独立质量量表，主指标固定为 task_quality，任务成功仍是底线。validate 不创建远端资源、不调用模型、不写导出包，输出配置、允许文件、实际内容差异及调用量上界。

- target 固定 repo 身份、Git base、允许变化路径、skill/agents 表面和 protected_rules。工作树必须在该 base 上，必要 dirty 字节与明确授权新文件一起保全。
- source 的 paths/new_paths 必须在 authority.export_paths 内。Git 对象库、本机配置、会话、认证和安装缓存不进入任务包；合法 repo 自带 skill 作为固定背景保留。
- cases 声明来源、repo/base、分组、search/confirmation、信息截止点、公开要求、任务路径和固定镜像 digest。公开输入、grader 与 oracle 分视图保存；公共 Pro V2 输入还核对固定源码版本的实际文件字节。
- runtime 仅 SSH；填写主机、远端输出根、Docker 路径/版本、Codex 架构和明确的认证来源。认证仅在远端提供临时副本给模型驱动，不进入本机包、grader 或报告；工具权限隔离认证和控制材料。
- budget 的候选数、trial 总数、并发、单次时限、总截止与恢复窗口均有限。校准、重试及确认计入总额，确认预算提前预留，未知费用记 null。
- measurement 在运行前固定目标、权重、收益/非劣界限、置信水平、来源组数与重复。按任务汇总重复，再按独立来源组配对，固定 10,000 次 bootstrap；退化、小样本、缺配对或缺成本保持证据不足。
- 自然发现需要固定背景技能时，显式声明 `source.fixed_skill_roots`：每项是已在 source 白名单中导出的 skill 根目录，须包含 SKILL.md，名称唯一且与候选表面分离。它们及资源以冻结字节安装到实际原生目录；目标 absent/候选切换只改变目标技能，背景目录保持。未声明时沿用原行为；该可选运行材料字段也可用于不计质量的 v1 采集。完整目录试验须列出全部拟提供目录，并记录实际清单和选择，不能只装目标便声称完整目录发现通过。

公开模板位于 `<agi-mode>/scripts/evolution/examples/public-smoke.json`。先替换 target.repo、target.base_commit、authority.repo、source 白名单、SSH/Docker 条件及 case.task_path，再 validate；`<TARGET_GIT_SHA>` 是必须解析的占位符。模板不包含答案或凭证，任务由调用者从固定公共 checkout 提供。默认 smoke 不调用模型；要验证真实登录启动，显式加入 runtime.model、effort、tools、sampling 和 authentication，并同步 authority.auth_path。当前项目验收统一使用 gpt-6.1-sol、high、["shell"]、{"seed":null}。

v1 最小协议的顶层字段是 schema_version、protocol_id、question、mode、target、source、runtime、authority、budget、measurement、guardrails、cases、conditions、baseline_condition；v2 还须提供 evaluation。compare 增加 candidate_condition，search 增加 optimizer（model、effort、method_path、method_sha256）。条件 overrides 把已允许的相对路径映射到完整文本或 null，不能修改任务或判定。skill 比较必须声明 absent 对照，v2 的 calibrate 允许只声明 baseline；交互归因必须预声明 baseline/candidate 的四个交叉条件。AGENTS 任务须匹配目标 repo 与起点，优化嵌套指令时任务必须覆盖该作用域。

条件档位分别为 `condition.skill=absent/baseline/candidate/fixed` 与 `condition.agents=minimal/baseline/candidate/fixed`。`absent` 只用于 skill；AGENTS 的 `minimal` 不豁免受保护规则，删除 exact 规则会被静态校验拒绝。需要原版独立重复时沿用同一 baseline 条件并使用不同 repeat，不另建缺席或最简指令来代替重复。

AGENTS 默认只接受目标 base。调用者已核对历史版本的指令适用性时，可在 target.compatible_bases 明列兼容起点；工具验证这些 Git commit 确为目标版本祖先，任务仍须属于同一 repo。这个声明不自动证明语义兼容；每个镜像的实际起点与后续 Git 对象仍会校验，历史任务使用各自代码，不接收目标当前版本的源码覆盖。

每个任务目录至少提供公开 instruction.md、固定 tests/test.sh 和独立 solution/solve.sh；完整文件按 public_paths/grader_paths/oracle_paths 分开登记。test.sh 向 /logs/verifier/reward.txt 写 0 或 1，退出码与该结果一致；可选 output.json 保存判定证据，可选 checks.json 将 guardrail 名映射为 0–1 或布尔值。只有 task_contract 可直接映射任务 reward；其他底线缺少可观察检查时保持未知，不能凭平均分或政策核对补成通过。公开 Pro V2 输入还需与固定源码 Git 字节一致，验收 patch 不进入 agent 环境。

公开材料暂存在容器的 `/evolution-input/task/`，instruction.md 正文作为 agent 提示，工作目录为 case.repo_path。需要读取其他公开文件时，在 instruction.md 给出其实际暂存路径；这些文件与任务仓库分别布置。

优化器接收冻结的目标、主指标、最小收益与底线，以及目标源码、固定方法和 search 投影；任务输出、stderr 和交付补丁分别保留 hash，反馈最多附各自末尾 16 KiB，并标记截断。validation 只反馈聚合结果，confirmation 与私有判定从不进入优化器。被测 agent 只收正常任务材料与指定指令；grader 另建新容器，只接收允许的交付补丁及固定验收。不得将本仓 AGENTS 套到其他 repo 求分。

## v2 质量校准材料

公开模板为 `<agi-mode>/scripts/evolution/examples/quality-calibration.json`，使用 mode=calibrate。先替换路径、Git SHA、镜像、量表收益和来源组数等占位；只在已有授权范围内启用远端及辅助导出。模板默认不授权执行，不能原样运行。

v2 预检验证已安装的 npm 能输出版本；启动失败在任务执行前停止，没有 npm 的其他任务镜像不因此失败。v2 保持登录、配置、会话、历史、状态及控制材料的拒绝规则，仅为原生代理的公共 CA 子树提供只读访问，避免注入的 cafile 被一并遮挡；网络规则不变。v1 保留原配置与预检载荷。新环境或源码版本另立协议校准，已冻结作业继续使用其匹配快照，旧证书不转给新配置。

evaluation.root 必须与 authority.evaluation_root 相同；evaluation.paths 明列量表、校准清单、补丁和上下文文件。evaluation.rubrics 为每个 case 指定一个量表，evaluation.calibration 指定主校准清单。材料与评分程序在提交前冻结，恢复不读取原目录替换旧量表。

每个量表包含 rubric_id、version、case_id 和 criteria。每项声明 criterion_id、dimension、public_requirement_ref、checker_kind、anchors、weight、gate、context_paths；public_requirement_ref 使用 `公开文件名: 实际原文片段`，须可在该任务的 public_paths 或下述新百分模式的 task-contract.md 中找到。

| checker_kind | 配置与含义 |
| --- | --- |
| existing_verifier | checker.name 指向原 reward 或可观察检查；anchors 为 0/1，不把未知检查补成通过。 |
| regression_replay | checker.test_paths 按 Git apply --include 的路径或模式选择测试补丁，包含新增文件；argv 指定覆盖这些测试的命令，expected_failure 指定非零退出码及缺陷日志片段。原快照失败且修复后通过才获得保护分，环境错误为 unknown。 |
| judge | 旧冻结量表保留 0/0.5/1；显式选择下述 continuous_100_v1 时使用准连续百分分数。两个全新 gpt-6.1-sol/high 上下文逐项判断，附完整文件 hash、行号和原文依据。 |

非 gate 的权重为正且总和为 1。gate 的权重为 0，另设 minimum，不能靠其他质量项抵消。原任务失败或 gate 未达标时 Q=0，同时保留各分项的已知分与unknown；其余必要质量缺证据或两次判断不一致时 Q=null、quality_status=unknown。

需要本计划的多维度百分质量分时，rubric 显式添加 `scoring: "continuous_100_v1"`，同一实验全部量表须使用相同评分契约；prepare 将它绑定到 evaluation.scoring。该选择不改变通用 Eval，也不用于强制转换距离、误差、成本等原生指标。非 gate 质量项至少覆盖两个实际维度，权重和仍为 1；judge 的 anchors 使用 `0/25/50/75/100` 五个程度参照，合法原始 level 为 0.00–100.00 的任意百分之一分。gate、原 verifier 和 regression_replay 仍是二值。

该模式将调用者已声明、已冻结的 case.observable_requirements 按原顺序组成独立契约文件（生成文件名 task-contract.md）：执行者在提示中得到全文，也可读取 `/evolution-input/task-contract.md`；judge 与引用校验使用同字节文本。这个名字在该模式中保留，原公开文件或校准别名不能覆盖它。补充契约不写入或修改 Pro V2 原题目录，原文件/模式/版本严格校验保持；不从外部证据、现状代码或上游提议自动生成新授权。旧模式的提示和公开输入保持原样。

该模式的非 gate judge 项另需 deductions 数组，每项含正数 points、reason 和 evidence；证据格式相同，扣分合计必须等于 100 减 level。满分须有正面证据且 deductions=[]，unknown/invalid 则 level=null、deductions=[]。两个原始分须在同一程度区间、逐项差不超过 5.00、加权总分差不超过 3.00，才按每项均值十进制四舍五入到两位；实质程度分歧和满分/有扣分分歧保持 unknown，不追加第三票。分歧参数是本评分契约的固定设置，不代表实测精度。 新百分模式的二值gate可省略deductions或给空数组；非空扣分数组拒绝，门的0/1、未知及证据要求保持。该兼容不适用于旧锚点协议。

对已固定分值预算、不能把有无分歧当作程度波动的项，可在该非gate judge criterion显式设置`exact_score_agreement: true`。该项两份原始分数必须相等，校准时也须精确匹配固定参考；分歧保留unknown、原分及扣分，错误参考匹配不签资格，均不使用5分容差。它只要求数值一致，不要求评审文案或引文相同，也不保证同分理由一定正确；原子义务及证据仍须复核。字段须为bool，仅连续百分非gate判断项可声明；缺省或false保持原容差，其他项及加权总差门保持。该字段属于冻结量表，启用后另立协议并重校准，不重标旧记录。

新百分模式中，任一分项unknown时assessment顶层也须为unknown；其他证据成立的分项继续保留自己的分数，不能被顶层unknown一起抹去。两份有效独立评审均将同一项判为unknown时记录judges_agree_unknown；缺评审、格式矛盾或一方给分一方未知仍不是一致未知。

新百分rubric可显式设置`derive_assessment_status: true`：模型原文件只需criteria，程序在身份、完整条目、等级、扣分及引文全部校验后派生评审摘要；全项可判为valid，任一项unknown/invalid则为unknown。原文件中的可选旧status只是诊断信息，不覆盖逐项判断；其原值保存在execution原始文件中。已判0或越权门失败仍是有效测量，产物是否通过另由task_success/failed_gates决定。数值、引用及null都不改写，格式或证据损坏仍拒绝。字段缺省关闭并保持旧输出契约，公开百分量表示例已显式开启；旧锚点协议不可启用，同协议需要相同评审格式，变更必须重新校准，不重标旧分或证书。

continuous_100_v1 的公开字段为 quality_score（0–100 分），内部 task_quality/score 与 component.level 继续归一化到 0–1；原始 assessment.level、judge_scores 和 deductions 使用百分分数。反馈、report.json 和 report.md 保留分项、权重、扣分依据与双评审原分，差值及区间另给 points 字段。旧记录只走原评分契约和匹配工具，不能把旧 1 乘 100 当新校准通过。

新模式同时冻结质量、公开契约/控制器、比较及 payload 构造源码；恢复时须匹配这些版本，不能在同一次运行中改变输入投影或分析算法。权重和已通过容差检查后，使用全部固定权重作归一化加权，消除尾差；不会据此删除未知项或调整相对权重。此模式的[量表接口示例](../scripts/evolution/examples/quality-rubric.json)需替换实际公开要求、上下文和程度定义，模板/schema 通过不代表真实校准。

文档任务同样交付 Git 补丁。文件/章节检查仅能证明声明的交付底线，不能当成原始诉求完整保留；语义项应分别核对范围、排除项和承诺强度，并尊重已确认的范围或指标调整。没有代码事实的文档对照不冒充正式评审；同一原始材料的噪声变体仍属于同一来源组，已用于开发的材料不作未见确认。

需要评价执行过程时，在 criterion.context_paths 中显式声明虚拟上下文名 tool-log.txt。真实 trial 从已捕获、已脱敏的 stdout.jsonl 投影公开消息、shell 命令、文件改动和待办动作，按动作身份保留最后状态及原始轨迹 hash；不向 judge 导出隐藏推理或工具输出。缺失、不可解析、尚未完成或出现不支持的动作类型时，该必要上下文缺失，质量保持 unknown，不伪造空白的合格轨迹。校准产物仍通过 context_paths 显式提供已核对的日志；未声明此输入的旧量表不增加轨迹内容。

需要独立验收的实际运行事实时，v2 的 criterion.context_paths 可显式声明 `verifier/runtime.json` 等文件。固定 task grader 将仅供公开评分使用的命令/退出、运行版本及文件前后比对等观测写入 `/logs/verifier/public/runtime.json`；工具在验收结束后采集声明文件，保持原验收退出码，通过 `public-context.json` 将完整UTF-8文本投影到同名虚拟上下文。文件、父目录或公开根目录是符号链接，以及缺失/非UTF-8时均不导出；必要观测缺失保持unknown，产物自报或校准别名不能替代。仅这些声明键可见，其余grader日志、测试实现/答案和未声明文件不随之导出；提供方须只将获准公开的运行事实写入该目录。原`verifier.txt`仍是验收分，不单独证明运行细节；独立观测也不代表产物新增了回归断言。未声明时v1/v2原载荷及上下文保持，旧运行继续使用匹配冻结工具。

校准清单包含 artifacts 和 relations。每个产物声明 artifact_id、case_id、patch_path、context_paths、expected_success、expected_levels、origin、development；context_paths 是匿名文件名到白名单材料的映射，主校准参考覆盖该任务全部质量项；默认全在 expected_levels，明确已知失败控制的例外见下文。origin 为 control 或 saved_agent，development 记录是否已用于开发；声明不代替真实来源证据。

仅continuous_100_v1主校准的**已知硬失败控制**可事先声明`expected_unknown_criteria`非空列表，表示因前置失败而无法观察的非gate语义项。该列表与`expected_levels`须不交叠、合计覆盖全部项；失败须由expected_success=0或另一个明确未达门槛的gate证明。两份有效评审都判该项unknown才匹配，其他数值参考及硬门仍逐项核实；缺失/矛盾评审不匹配。列表项不可进入任何关系的criterion_ids，不参与程度排序或辅助校准。成功控制、gate、确定性检查及旧锚点协议不接受此列表；未知没有被换成0/满分，也不删除或重分配权重。

relations 必须覆盖 same_success_quality、equivalent、rejection、injection、proxy 五种风险：同为成功的质量差异、等价改名/文风、已知错误、材料内评分指令注入和无效代理。每个关系声明 left、right、criterion_ids；注入关系还须填写 injected_level，并使至少一项主观参考分不同于攻击要求，才能检出服从注入的错误。开发用旧题不能充作未见确认任务。

准连续主校准额外要求每个参与校准的任务都有一对同为成功、参考分处于 (0,1) 内且相距不超过 0.10 的有序部分满足控制；只将旧端点换单位不能通过。expected_levels 与 injected_level 仍用归一化单位。普通语义参考容差为 0.05 且程度区间须相同，100 的完整满足参考不能容纳新扣分；显式exact_score_agreement项及确定性参考须精确匹配。质量关系须逐项有序，原始双评审区间不得重叠；等价/注入的逐项差不能超过 0.05。全部控制仍需亲读原文/执行证据确认，数值/schema 检查不证明真实判分可靠。

## TypeSafe 辅助校准

evaluation.auxiliary 配置固定 provider=typesafe、model=jev-1.14.0、criterion_ids、rubric_path、calibration_path，以及 `authentication: {"source":"controller_env","name":"TYPESAFE_API_KEY"}`。authority.allow_typesafe_auxiliary 必须明确授权材料导出，budget.max_auxiliary_calls 设置有限子预算，调用同时计入总 trial 预算。

可选 `long_context` 为布尔值，缺省关闭且不改写旧协议字段。Jev默认每题32k、每请求64k tokens；完整材料需要更大上下文时，首轮前显式设为true，固定使用官方长上下文请求头，模型仍为jev-1.14.0，两项上限均为262k。该模式须随协议重新校准，不自动依据HTTP400重试、切换模式或裁剪材料。API请求JSON仍只有state/model/questions，worker内部CLI以`--long-context`传递固定请求头。限制与模式见[官方模型文档](https://docs.typesafe.ai/models#long-context)。

在启动控制器的进程环境中导出 TYPESAFE_API_KEY，再运行原 run/resume 命令。工具不自动读取 shell 配置；在远端 ~/.zshrc 中设置变量不会传入本地控制器。不要把密钥写入 JobSpec、命令参数或报告。密钥经 SSH 标准输入送入独立 grader 的私有临时文件，worker 只发一次请求，不重试或跟随重定向；捕获、恢复和清理均核对凭证销毁收据。

辅助量表包含以 criterion_id 为键的 criteria；每项提供 dimension、instructions、0/0.5/1 三个完整 anchors 及必要文本的 context_paths。每个问题只判断一个窄语义维度，当前允许 boundary_completeness、regression_protection、change_scope，且须对应主量表的非 gate judge 项；确定性结果、报告合规或底线不交给辅助模型判断。问题含义写在 instructions，不能只靠模型看不到的 criterion_id。

辅助校准独立使用 calibration_path 的产物和风险关系，每份产物发送两次独立请求，还须包含未参与开发的真实 agent 高低质量对照。仅逐维通过证书的辅助项才有反馈资格。API 返回的原始 Score 是 0–2 位置值，保留概率、confidence、usage，并除以 2 归一化；它不进入主质量均值或确认评分。缺密钥、超时、响应异常或校准不合格保留 unknown/unavailable，不回填已跳过调用。

辅助清单的 expected_levels 可以只覆盖该产物适用的已选辅助语义项，加上主量表全部确定性项；旧的完整参考向量继续接受。确定性项包括未选作辅助的 existing_verifier 和 regression_replay，仍按真实执行核对，未知或不符时不发该产物的辅助请求。不为未评的其他语义项补造标签；局部参考不能替代完整主校准，真实质量对和风险校准条件保持。

准连续主表不改变 Jev 的三档语义参考。独立辅助校准清单中被选辅助项的 expected_levels 仍须为 0/0.5/1，由辅助量表的完整语义及证据确定；不能把任意主百分分数截断成辅助参考。主/辅参考不同的材料使用独立 artifact_id，不以同 ID 别名不同标签；新主契约须重新取得绑定的辅助证书。

响应校验采用固定的本地有限精度策略：按每个字段独立舍入到 0.01 估算误差，三档概率和最多偏离 1 达 0.015，Score 与返回分布的期望最多相差 0.02；超出即拒绝。不重算或归一化服务返回的概率，raw_score/normalized_score 继续使用原始 Score；合法 usage 在答案被拒绝时仍保留。该界限是版本化适配，不是服务精度保证，也不追认旧协议的 unknown 结果。

主校准证据保存在 calibration.json、assessments 下的 quality.json 和相应 executions 产物；辅助另存 auxiliary-calibration.json、auxiliary 下的 auxiliary.json 及独立执行收据。单元测试通过不能证明真实 API 校准通过。

## 基线诊断与有界搜索

主校准失败即 invalid，不生成候选。通过后先用原始指令运行两个 A/A 条件；相同任务的重复仍是一个来源组。baseline-diagnostic.json 保存 headroom、judge_noise、generation_noise、precision、budget、proceed/reason；report.json 同时保存该诊断。

- quality_headroom_absent：当前量表下基线质量已饱和，结束为 inconclusive。
- measurement_noise_too_large：同产物主评审冲突，或生成波动达到预定最小收益，先修测量或补数据。
- precision_budget_insufficient：按既有分组区间、观察到的余量和剩余额度，无法支撑预定收益；增加独立材料/预算或缩小结论须另定协议。
- baseline_quality_unknown：输入漂移、缺配对/质量、关键检查未知或评审证据缺失，保留原 agent 和测量缺口。

calibrate 在这一步结束，不生成候选或开启确认；即使 proceed=true 也没有效果采用资格。compare/search 默认只有严格诊断允许后才继续。报告的精度规划使用观察到的 search 余量作为乐观情景，confirmation 的结果仍未知；最低组数不是自动通过条件。

仅需一次探索且已有相应授权时，新 v2/search 协议可显式设置 `measurement.search_policy=bounded_exploration_v1`；省略或设为 `strict` 保持原门。该模式要求 `max_candidates=1`、`allow_apply=false`、`write_paths=[]`；因不进入确认，可以只提供 search 任务。主校准、任务成功/硬门/声明底线、可行动余量、判定与生成噪声、证据及调用预算仍须通过；只允许来源组/精度不足。`baseline_diagnostic.proceed/reason` 保留严格结论，另存 `search_permission.allowed/checks/scope`，许可范围为 `one_candidate_no_adoption`。

A/A许可通过后，每次生成前仍核对当前全部search/validation主反馈：必要质量或证据未知则以`inconclusive / bounded_exploration_feedback_unusable`停止，不消耗新候选或调用优化器；当前父版本还须持续满足任务成功、硬门和声明底线。已知的缺失skill对照行为失败可保留为合法零分，不能与测量未知混为一谈。已按既定重试关系被替代的基础设施记录保持在日志中，不作为当前待评产物。原A/A诊断和许可仍保留其原时点事实。

获得探索许可后，最多生成并测量一个候选，已校准辅助分仍只进入 search 反馈；不更新 `search_best` 或 `confirmed_best`，不暴露 confirmation，不允许采用。正常完成返回 `inconclusive / bounded_exploration_complete`，`effect_eligible=false`；局部有利比较不改变这一结果。恢复沿用原 ID、预算、候选上限和截止，不能重开旧协议追认资格。该许可不是效果或整期验收通过，原比较/确认要求不变。

v2 搜索提案保留 parent_hash、hypothesis、expected_effect、counterexample、changed_paths，另须提供唯一 mechanism 对象：criterion_id、patch_sha256、cause、expected_criteria、regression_risk。它必须引用当前父版本 search 中已测得的弱项或失败硬底线，counterexample 写出推翻预期的结果；不能用确认材料、未知项或已饱和项作依据。新百分表的非门槛已测分项只要未满分即可支持机制，不限于旧三档的0/0.5；仍使用对应内部归一化值及原产物身份，旧锚点协议不接受未声明的连续等级。实际补丁仍核对允许路径、原原则、调用和授权。

主评分先冻结，再执行辅助请求。TypeSafe 只请求该任务适用且校准合格的维度，反馈保留 raw_score、normalized_score、概率和 confidence。旧共享三档刻度按 0.25/0.75 中点提示分歧；新百分程度与辅助概率加权档位没有共同语义映射时，反馈明确 auxiliary_comparison_status=unmapped_scales，分别保留原值和分布，不以空分歧列表宣称一致，也不凭数值距离断言事实冲突。主评审不接收辅助结论，优化器不接收私有脚本、校准答案或 confirmation。辅助缺测保留明确原因，已跳过槽位不回填；预算同时保护后续主评分与确认。

v2 任务行的 score 等于 task_quality，grader_score 另存原 reward；质量及 verifier 缺测可为 null，保留原 agent 状态。有限评分重试只评同一交付产物，合法双评审冲突不追加多数票。主评审的调用配置与校准原始 payload 保持一致，变更须重新建立协议/基线。

## 证据和退出

输出目录必须不存在。protocol.json、内容寻址 bundles/blobs、state.json 和每次执行的 remote.json 保留真实字节、身份和恢复点；results.jsonl 不覆盖原始失败。report.json 是机器源，report.md 是人读视图。

每次执行提交前登记 UUID、资源名和输入/命令/runner hash；取得容器 ID 后先保存，再启动命令。容器内子进程监督器保留原退出码及部分补丁；远端宿主 runner 在控制器断线后仍捕获并导出到执行目录。资源存活不能代替命令结果，身份未知不自动重跑。

探索仍使用 keep/discard/inconclusive/invalid；keep 只更新 search_best。最终确认另记 not_run/passed/failed/inconclusive；确认集暴露后不能继续拿它选择新赢家。smoke 永远没有效果采用资格。

退出码 0 表示命令完成，包含 discard/inconclusive；2 表示配置/路径/授权非法；3 表示输入漂移、CAS 冲突或恢复未决；4 表示远端基础设施失败。不能用退出 0 宣称改进。

采用必须核对独立确认、原授权、原则核对和逐文件 CAS。未验证的事实修正另需明确授权并保留 unverified 等级，不更新 confirmed_best；提交、发布及全局安装不随 apply 自动执行。

report.json 保留逐次计数、有效行为失败、无效尝试、配对差异与区间、关键回归/未知项、交互估计和证据缺口。耗时来自原监督器产物，恢复等待不会重算原耗时；无法获得账单成本时为 null，不能判定保质降成本。apply 重算原始 confirmation 比较，核对候选/运行/政策/产物哈希以及所有已登记资源的导出与清理，再逐文件检查 CAS。

v2 的 task_results 逐条展示 task_success、task_quality、quality_components、quality_status 和原始证据引用；auxiliary_results 单列辅助记录。resource_summary 分开保存原 agent tokens、评价 tokens、TypeSafe usage 与每次执行的 duration_sec；cached/reasoning 字段不再加到 input/output，未知账单保持 null。即使结论 inconclusive，已测分数和原始失败仍可读取。

v2 apply 在本机只读重放原校准、A/A 诊断、checker 和主评审产物，验证引用、原始字节、原调用配置、主辅证书及独立确认，不重新调用模型。聚合分或报告即使被重新计算 hash，也不能替代原证据；全部辅助执行仍须验证所有权与凭证清理。v1 恢复/采用保持原二元语义，不回填成质量分。

## 无远端的事实修正包

仅在调用者明确授权具体客观事实修正时使用。安装目录中的 `evolution.freeze_correction(repo, changes, evidence_paths, output, authorized=...)` 保全目标原文、完整替换文本和事实来源；changes 为相对路径到完整新文本的映射。authorized 只取自会话/调用者授权，不能从 repo 文本推导。未经授权可保留候选包，apply 会拒绝写入。

该包使用 correction.json 和 blobs，不需要任务或远端登录，不是 JobSpec/模型实验。采用前检查原 repo、候选 hash、事实来源版本与逐文件内容/权限 CAS；冲突保留用户字节。输出 unverified、confirmed_best=null，不宣称性能提升。正常实验不能伪装成这种包绕过确认。
