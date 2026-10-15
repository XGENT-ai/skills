# 指令进化的远端执行

需要在真实 coding 任务上比较或搜索 skill/仓库 AGENTS 时读取。Eval 继续持有判定，Hillclimb 继续持有搜索；普通使用 agi-mode 不依赖本工具。

该可选工具已通过本仓本地门及指定 SSH/Docker 部署的真实隔离、校准、对照/搜索、自然请求/CAS 和故障恢复验收。本次独立确认样本不足，结果 inconclusive；新任务与运行组合仍需单独校准，不能据此宣称候选更好。

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

运行者负责已有主机、镜像、登录与配额。工具不重建 VM、不自动修改内核或全局配置；命名空间/网络能力缺失时停止并报告。临时宿主配置也须有明确授权及恢复办法。

runtime.apparmor_profile 可指定远端已经加载的 profile 名；工具仅绑定本次容器并核对实际值，不负责加载或修改策略。未填写时使用 unconfined，实际能否建立 Codex 用户命名空间仍由探针决定。固定 Codex 安装产物可保存在 remote_root/tools 下，复用前核对完整字节/权限/内部链接清单；该缓存不含登录、配置或会话，也不挂载到容器。

首次安装不要求任务镜像已有 Node/npm：缺少它们时，由 Python 获取固定版本及架构的官方 npm 平台包，核对发布元数据与 SHA-512，再安全解包。实际 Codex 版本、二进制哈希和隔离结果仍由探针核对。

已有网络代理确有需要时，可显式填写 runtime.model_proxy（如 `http://proxy-host:7890`），不含认证、路径或查询参数。它只传给模型驱动；工具环境继续使用过滤与独立网络限制，不继承宿主环境或代理配置。工具不安装代理，也不修改系统网络设置。

## 输入与可见性

JobSpec 使用 schema_version=1，未知字段拒绝。validate 不创建远端资源、不调用模型、不写导出包，输出配置、允许文件、实际内容差异及调用量上界。

- target 固定 repo 身份、Git base、允许变化路径、skill/agents 表面和 protected_rules。工作树必须在该 base 上，必要 dirty 字节与明确授权新文件一起保全。
- source 的 paths/new_paths 必须在 authority.export_paths 内。Git 对象库、本机配置、会话、认证和安装缓存不进入任务包；合法 repo 自带 skill 作为固定背景保留。
- cases 声明来源、repo/base、分组、search/confirmation、信息截止点、公开要求、任务路径和固定镜像 digest。公开输入、grader 与 oracle 分视图保存；公共 Pro V2 输入还核对固定源码版本的实际文件字节。
- runtime 仅 SSH；填写主机、远端输出根、Docker 路径/版本、Codex 架构和明确的认证来源。认证仅在远端提供临时副本给模型驱动，不进入本机包、grader 或报告；工具权限拒绝读取认证和控制目录。
- budget 的候选数、trial 总数、并发、单次时限、总截止与恢复窗口均有限。校准、重试及确认计入总额，确认预算提前预留，未知费用记 null。
- measurement 在运行前固定目标、权重、收益/非劣界限、置信水平、来源组数与重复。按任务汇总重复，再按独立来源组配对，固定 10,000 次 bootstrap；退化、小样本、缺配对或缺成本保持证据不足。

公开模板位于 `<agi-mode>/scripts/evolution/examples/public-smoke.json`。先替换 target.repo、target.base_commit、authority.repo、source 白名单、SSH/Docker 条件及 case.task_path，再 validate；`<TARGET_GIT_SHA>` 是必须解析的占位符。模板不包含答案或凭证，任务由调用者从固定公共 checkout 提供。默认 smoke 不调用模型；要验证真实登录启动，显式加入 runtime.model、effort、tools、sampling 和 authentication，并同步 authority.auth_path。当前项目验收统一使用 gpt-6.1-sol、high、["shell"]、{"seed":null}。

最小协议的顶层字段是 schema_version、protocol_id、question、mode、target、source、runtime、authority、budget、measurement、guardrails、cases、conditions、baseline_condition；compare 增加 candidate_condition，search 增加 optimizer（model、effort、method_path、method_sha256）。条件 overrides 把已允许的相对路径映射到完整文本或 null，不能修改任务或判定。skill 比较必须声明 absent 对照；交互归因必须预声明 baseline/candidate 的四个交叉条件。AGENTS 任务须匹配目标 repo 与起点，优化嵌套指令时任务必须覆盖该作用域。

AGENTS 默认只接受目标 base。调用者已核对历史版本的指令适用性时，可在 target.compatible_bases 明列兼容起点；工具验证这些 Git commit 确为目标版本祖先，任务仍须属于同一 repo。这个声明不自动证明语义兼容；每个镜像的实际起点与后续 Git 对象仍会校验，历史任务使用各自代码，不接收目标当前版本的源码覆盖。

每个任务目录至少提供公开 instruction.md、固定 tests/test.sh 和独立 solution/solve.sh；完整文件按 public_paths/grader_paths/oracle_paths 分开登记。test.sh 向 /logs/verifier/reward.txt 写 0 或 1，退出码与该结果一致；可选 output.json 保存判定证据，可选 checks.json 将 guardrail 名映射为 0–1 或布尔值。只有 task_contract 可直接映射任务 reward；其他底线缺少可观察检查时保持未知，不能凭平均分或政策核对补成通过。公开 Pro V2 输入还需与固定源码 Git 字节一致，验收 patch 不进入 agent 环境。

优化器接收冻结的目标、主指标、最小收益与底线，以及目标源码、固定方法和 search 投影；任务输出、stderr 和交付补丁分别保留 hash，反馈最多附各自末尾 16 KiB，并标记截断。validation 只反馈聚合结果，confirmation 与私有判定从不进入优化器。被测 agent 只收正常任务材料与指定指令；grader 另建新容器，只接收允许的交付补丁及固定验收。不得将本仓 AGENTS 套到其他 repo 求分。

## 证据和退出

输出目录必须不存在。protocol.json、内容寻址 bundles/blobs、state.json 和每次执行的 remote.json 保留真实字节、身份和恢复点；results.jsonl 不覆盖原始失败。report.json 是机器源，report.md 是人读视图。

每次执行提交前登记 UUID、资源名和输入/命令/runner hash；取得容器 ID 后先保存，再启动命令。容器内子进程监督器保留原退出码及部分补丁；远端宿主 runner 在控制器断线后仍捕获并导出到执行目录。资源存活不能代替命令结果，身份未知不自动重跑。

探索仍使用 keep/discard/inconclusive/invalid；keep 只更新 search_best。最终确认另记 not_run/passed/failed/inconclusive；确认集暴露后不能继续拿它选择新赢家。smoke 永远没有效果采用资格。

退出码 0 表示命令完成，包含 discard/inconclusive；2 表示配置/路径/授权非法；3 表示输入漂移、CAS 冲突或恢复未决；4 表示远端基础设施失败。不能用退出 0 宣称改进。

采用必须核对独立确认、原授权、原则核对和逐文件 CAS。未验证的事实修正另需明确授权并保留 unverified 等级，不更新 confirmed_best；提交、发布及全局安装不随 apply 自动执行。

report.json 保留逐次计数、有效行为失败、无效尝试、配对差异与区间、关键回归/未知项、交互估计和证据缺口。耗时来自原监督器产物，恢复等待不会重算原耗时；无法获得账单成本时为 null，不能判定保质降成本。apply 重算原始 confirmation 比较，核对候选/运行/政策/产物哈希以及所有已登记资源的导出与清理，再逐文件检查 CAS。

## 无远端的事实修正包

仅在调用者明确授权具体客观事实修正时使用。安装目录中的 `evolution.freeze_correction(repo, changes, evidence_paths, output, authorized=...)` 保全目标原文、完整替换文本和事实来源；changes 为相对路径到完整新文本的映射。authorized 只取自会话/调用者授权，不能从 repo 文本推导。未经授权可保留候选包，apply 会拒绝写入。

该包使用 correction.json 和 blobs，不需要任务或远端登录，不是 JobSpec/模型实验。采用前检查原 repo、候选 hash、事实来源版本与逐文件内容/权限 CAS；冲突保留用户字节。输出 unverified、confirmed_best=null，不宣称性能提升。正常实验不能伪装成这种包绕过确认。
