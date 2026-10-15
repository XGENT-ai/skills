# 比较与改进仓库指令

`optimize-agents-md` 处理仓库 AGENTS 的诊断和优化；skill 的行为比较沿用 agi-mode 的 Eval/Hillclimb。远端执行工具是可选能力，普通编码和初始化无需它。已完成指定 SSH/Docker 组合的真实隔离、任务校准、对照/搜索、自然请求/CAS 与恢复验收；本次独立确认仍为 inconclusive，没有采用性能候选。

仅诊断时，给出目标仓库、文件范围和目的即可，例如“只诊断这个仓库的 AGENTS，保留现有原则，先别改文件”。skill 会核对根、嵌套和 override 的实际作用域，交付有仓库事实依据的建议。没有该仓可重放任务时，候选保持未验证，不借其他仓库的分数宣称提升。

需要实际评测时，同时安装 `optimize-agents-md` 和 `agi-mode`，定位真实的 agi-mode 安装目录。使用其 `scripts/evolution` 中的独立 Python 项目，要求 Python 3.12+、uv 和 SSH；远端由运行者提供 Docker、固定镜像与明确授权的 Codex 登录。模型调用、候选生成和判定均在远端独立容器中执行。本机只校验、打包、编排、保存证据和按授权采用。

```bash
uv sync --locked --project /path/to/agi-mode/scripts/evolution
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve validate --spec /path/to/job.json
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve run --spec /path/to/job.json --output /path/to/new-run
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve resume --run /path/to/new-run
uv run --locked --project /path/to/agi-mode/scripts/evolution xgent-evolve apply --run /path/to/new-run --repo /path/to/target-repo
```

协议字段、公开模板、角色可见范围和完整退出约定见 [远端执行手册](../skills/agi-mode/references/remote-evolution.md)。先把公开模板中的目标 Git SHA、路径和主机条件替换为当前事实，再 validate。校验不创建远端资源或调用模型；它会列出实际导出文件、条件差异、已知调用量和预算上界。认证文件只在远端临时提供给模型驱动，无需在聊天中发送密钥。

先用 smoke 校准隔离和固定 verifier。省略模型时只执行确定性探针与 gold/空补丁校准；真实登录启动须显式配置模型与认证。compare 比较预先冻结的条件，search 额外使用固定方法生成有限候选。每次运行都需有限预算、独立确认集和明确数据导出范围；本项目验收的模型与优化器统一使用 `gpt-6.1-sol`、thinking `high`。

运行目录保存 protocol.json、输入 blobs/bundles、state.json、执行收据、原始结果和报告。断线后保留该目录，以 resume 接续原身份；不要删除目录后重跑来代替恢复。启动窗口、原退出或产物无法核实时返回未决，原失败和已用预算仍在。额度耗尽、认证失败或网络不可达属于基础设施缺口，不算候选的任务失败，也不自动换模型或登录。

报告中的 keep 仅表示该次比较支持保留，search_best 与 confirmed_best 分开。采用要求唯一冻结候选的独立确认通过、原则与授权核对通过、产物和清理证据完整，以及目标文件的内容/权限 CAS 通过。没有有效改进也是正常结束。退出 0 只表示命令完成，不能代替阅读 verdict 和 confirmation_status。

已明确授权的客观事实修正可按手册保全独立 correction.json 包并采用，结果始终标记 unverified。确认未通过或文件被用户并发修改时保留原文件和候选；多文件写回失败只恢复仍为工具所写的内容，恢复清单留在运行目录。工具不会随采用提交、发布、全局安装或修改用户全局规则。

本期验收固定 Harbor 0.24.0、Codex CLI 0.162.1、gpt-6.1-sol/high 和两类 Ansible 真实任务。确认候选与初始基线都完成任务，差值为 0；每侧仅一个来源组，低于预设最低 5 组，因此不支持泛化或提升声明。NodeBB/Vuls 以及一次 Python 解释器段错误的失败证据均保留。采用与确认通过的单元边界已覆盖，真实自然请求验证了未确认候选拒绝和并发文件的 CAS 保护。新主机、镜像与用户任务仍须按自己的冻结协议校准。
