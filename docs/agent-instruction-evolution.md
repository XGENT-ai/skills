# 比较与改进仓库指令

`optimize-agents-md` 处理仓库 AGENTS 的诊断和优化；skill 的行为比较沿用 agi-mode 的 Eval/Hillclimb。远端执行工具是可选能力，普通编码和初始化无需它。v1 保留二元 task_success；v2 独立测 task_quality，以任务成功作底线，并先校准辨别力、诊断基线余量/噪声/精度预算，再决定是否搜索。TypeSafe 独立校准后只辅助 search，主质量及采用仍由原链负责。[v2 实施进度](plan/agent-instruction-evolution-v2.md)区分本地接线与真实闭环，不沿用首期验收结论宣称新版已验收。

2026-10-22按现有结果阶段交付，计划仍为3/4，后续测评暂停。工程测试及已测真实子范围保留；完整skill三条件、已校准TypeSafe反馈进入真实optimizer和指令质量收益尚未完成验收。资源到位后换更稳定真实任务，以3.00分为最小实际收益重新资格和A/A；当前原probe回收缺口及恢复步骤见[M4阶段交付](plan/agent-instruction-evolution-v2.records/M4.md#2026-10-22-阶段交付)。

仅诊断时，给出目标仓库、文件范围和目的即可，例如“只诊断这个仓库的 AGENTS，保留现有原则，先别改文件”。skill 会核对根、嵌套和 override 的实际作用域，交付有仓库事实依据的建议。没有该仓可重放任务时，候选保持未验证，不借其他仓库的分数宣称提升。

需要实际评测时，同时安装 `optimize-agents-md` 和 `agi-mode`，定位真实的 agi-mode 安装目录。使用该安装目录中的独立 Python 项目（仓内源码：[agi-mode/scripts/evolution](../skills/agi-mode/scripts/evolution/)），要求 Python 3.12+、uv 和 SSH；远端由运行者提供 Docker、固定镜像与明确授权的 Codex 登录。模型调用、候选生成和判定均在远端独立容器中执行。本机只校验、打包、编排、保存证据和按授权采用。

```bash
uv sync --locked --project /path/to/agi-mode/scripts/evolution
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve validate --spec /path/to/job.json
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve run --spec /path/to/job.json --output /path/to/new-run
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve resume --run /path/to/new-run
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve apply --run /path/to/new-run --repo /path/to/target-repo
```

协议字段、公开模板、角色可见范围和完整退出约定见 [远端执行手册](../skills/agi-mode/references/remote-evolution.md)。先把公开模板中的目标 Git SHA、路径和主机条件替换为当前事实，再 validate。校验不创建远端资源或调用模型；它会列出实际导出文件、条件差异、已知调用量和预算上界。认证文件只在远端临时提供给模型驱动，无需在聊天中发送密钥。

v1 先用 smoke 校准隔离和固定 verifier；省略模型时只执行确定性探针与 gold/空补丁校准。新质量实验使用 [v2/calibrate 模板](../skills/agi-mode/scripts/evolution/examples/quality-calibration.json)，从公开请求建立独立 rubric、冻结校准材料，再运行质量校准和 A/A。calibrate 不使用确认集或生成候选；compare/search 另需独立确认集，且先满足相同校准/诊断门。每次运行都需有限预算与明确导出范围；主执行、优化器、policy 和主评审统一 `gpt-6.1-sol/high`，辅助固定 `jev-1.14.0`。

启用 TypeSafe 时显式授权最小文本材料导出、冻结窄维度量表与辅助校准、设置 max_auxiliary_calls，并在启动控制器的环境中导出 TYPESAFE_API_KEY；无需在聊天中提供密钥。工具不自动读取 .zshrc，远端 shell 中设置变量不会传入本地控制器。缺测、限流、未校准维度和子预算耗尽均有记录，不能改变主分或自动采用；确认阶段零辅助调用。

完整文档超过Jev默认单题上下文时，可在新协议中显式设置`evaluation.auxiliary.long_context=true`并重新校准；缺省仍关闭。该开关只选择同一模型的官方长上下文模式，不自动重试、改问题或裁剪原始文档；容量及配置见[远端手册](../skills/agi-mode/references/remote-evolution.md#typesafe-辅助校准)。

百分程度与辅助语义档位的概率期望不是自动共轴的数值。未定义共同映射时，辅助反馈标明 unmapped_scales，保留双方原值与分布供诊断；不把数值分段差冒充事实分歧，也不由空分歧列表推断一致。

运行目录保存 protocol.json、输入 blobs/bundles、state.json、执行收据、原始结果和报告。断线后保留该目录，以 resume 接续原身份；不要删除目录后重跑来代替恢复。启动窗口、原退出或产物无法核实时返回未决，原失败和已用预算仍在。额度耗尽、认证失败或网络不可达属于基础设施缺口，不算候选的任务失败，也不自动换模型或登录。

报告中的 keep 仅表示该次比较支持保留，search_best 与 confirmed_best 分开。采用要求唯一冻结候选的独立确认通过、原则与授权核对通过、产物和清理证据完整，以及目标文件的内容/权限 CAS 通过。没有有效改进也是正常结束。退出 0 只表示命令完成，不能代替阅读 verdict 和 confirmation_status。

v2 先读 baseline-diagnostic.json 的 proceed/reason；默认饱和、评审/生成噪声和精度预算不足均停止候选。仅来源精度不足时，显式获准的 `measurement.search_policy=bounded_exploration_v1` 可按另报的 search_permission 探索最多一候选；要求 max_candidates=1、allow_apply=false、write_paths=[]，其余质量门保持。不晋级最佳、不确认/采用，整体 inconclusive，不能把探索许可当效果资格。生成前还核对新主反馈及当前父版本底线；初始条件出现质量未知等缺口时，以 bounded_exploration_feedback_unusable 停止，原A/A许可不覆盖该缺口。再读 report.json 的 task_results、逐项 quality_components/quality_evidence_ref、auxiliary_results 与 resource_summary；同样成功可以有不同 Q，缺测为 null，agent 原耗时、主评价开销和 TypeSafe usage 分列。独立来源组数与可辨别范围都要看，不能用固定最低组数保证结论。apply 只读重放原证据并重算主质量，不重新付费调用模型；辅助资源及凭证清理仍参与核验。

本开发计划的新版质量协议使用[固定多维度评分表](plan/agent-instruction-evolution-v2.records/quality-table.md)：显式选择 continuous_100_v1，公开 quality_score 为 0.00–100.00，保留分项、权重、双评审原分及有证据的扣分；task_quality/score 仍是内部归一化数。部分满足及相近质量必须先有校准对照，旧三档结果不转换成新通过。其他自进化任务仍按目标选指标，例如直接最小化距离或误差；本工具的百分质量模式不成为通用要求。新版真实校准与基线状态见实施进度。

固定分值预算的非gate判断项可显式选择`exact_score_agreement=true`，该项只接受双原分相同，校准时还须精确匹配固定参考；分歧保持unknown和证据，不用一般程度容差平均。缺省规则及其他项不变，同分仍须核对依据；字段、范围及重新校准要求见远端手册。

若质量项需要实际运行证据，v2量表可在context_paths声明`verifier/runtime.json`等公开观测，由固定grader写入其public目录后按白名单采集。验收分1与命令/版本/文件比对分别保留，缺观测时unknown；其余grader日志/私有实现不自动导出，独立观测不计作产物新增回归测试。具体目录及安全边界见远端手册。

v2 运行时还检查已安装的 npm 能否启动；公共代理 CA 可只读，登录与会话等私有材料继续隔离。v1 保留原配置与载荷，旧冻结运行按匹配快照恢复，源码/环境变更后的质量资格需新协议建立。

新环境的provider验收同时核对真实工具读写入口：模型须能读取公开输入并写入获准输出，已存在的会话及私有状态仍不可读。仅返回登录就绪文本不足以证明工具可执行；配置修正不把缺失的原评审产物补成分数。

ARM主机上的v2自有仓任务（source.kind=repo）可显式声明`case.platform=linux/arm64`及匹配的固定镜像，重新校准原任务和质量表。缺省、v1及Pro V2仍为amd64；平台不会自动切换，旧平台分数不作为新平台资格。远端SSH/Docker、模型与原生Codex的约定保持。

已明确授权的客观事实修正可按手册保全独立 correction.json 包并采用，结果始终标记 unverified。确认未通过或文件被用户并发修改时保留原文件和候选；多文件写回失败只恢复仍为工具所写的内容，恢复清单留在运行目录。工具不会随采用提交、发布、全局安装或修改用户全局规则。

首期验收固定 Harbor 0.24.0、Codex CLI 0.162.1、gpt-6.1-sol/high 和两类 Ansible 真实任务。确认候选与初始基线都完成任务，二元成功分差值为 0；每侧仅一个来源组，低于预设最低 5 组。样本不足与评分维度饱和是两个问题：现有结果不支持泛化或提升声明，也没有测量解法质量的差异，不能仅补够组数便宣称质量评价有效。NodeBB/Vuls 以及一次 Python 解释器段错误的失败证据均保留。采用与确认通过的单元边界已覆盖，真实自然请求验证了未确认候选拒绝和并发文件的 CAS 保护。新主机、镜像与用户任务仍须按自己的冻结协议校准。

新百分模式按分项保留已知程度和未知；assessment中一项unknown要求顶层unknown，不能抹掉其他已知分。已知硬失败控制可事先用expected_unknown_criteria声明不可观察的非gate语义项，要求两份有效评审一致未知，其余数值/硬门照验；成功控制、关系排序和辅助校准均不接受该例外。原硬失败Q=0仍保留分项未知，不重分配权重；旧标尺不变。

若采用新百分rubric的derive_assessment_status=true，LLM仅供逐项分数/证据及未知，程序据完整校验派生评审状态。产物已知失败或越权可以有有效评审；null、坏证据和格式缺口仍保留或拒绝。默认旧契约不变，新格式须同协议一致并重新校准；详见远端手册。
