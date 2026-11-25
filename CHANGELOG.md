# Changelog

记录版本变更与历史开发事项；安装和使用方法见 [README.md](README.md)。以下为从 README 迁出的历史记录，不是完整发布历史。

## 0.7.14

- Phoenix UI 0.1.2 与 Phoenix Diagram 0.2.1 不再提供 macOS Intel 引擎。支持的平台为 macOS Apple Silicon、Linux x64 和 Linux ARM64；此前发布的版本不受影响。
- Phoenix UI 安装时不再自动注册 Phoenix UI hooks；重装会清理所选项目工具中旧版自动注册的 hooks，保留用 `phoenix-ui hooks on` 显式启用的配置。传入 `--no-phoenix-ui` 时不清理旧 hooks。
- 新增 `agent-browser` 和 `use-decision-model` 两个 skill，分别提供浏览器 CLI 操作指引和代码中类型化语义判断的模型选型与接入指引。

## 0.7.13

- Phoenix Diagram 图类型增至十三类：新增 chart（柱状、折线、饼图与限额对比四种形态，每图附可展开数据表）与 annotation（逐词点评）类型；变更标记扩展到 workflow、lifecycle、tree、erd；新增静态图命令与字体策略，及多项渲染修复（饼图图例、负值条、limits 行、批注卡、树连线等）。
- Phoenix UI 新增 compare 对比命令与对比页、capture 页面证据命令（多状态截图、拼接与隐藏/遮挡/禁用目标的定位器建议）；新增参考媒体搜索与交付图片流程，预览媒体与浏览器网络边界收紧；新增 xgent/cjk-* 中文排版页面规则与中文排版指引。
- elintp 改为离线模板渲染图文解释，可内嵌 phoenix-diagram 静态图、页面卡片与图表数据表，支持本地审阅轮次与可恢复的离线终稿。
- 新增 software-copyright-docs、feature-demo skill；review-code 评审中收集证据并指名缺口执行人；portal-dev-setup 升级保留一盒已有数据；xgent-app-release 接受审批通过的 HTTPS connect scheme。
- 更新 rustls TLS 依赖栈并复钉 RustSec 数据库。

## 0.7.12

- 安装器默认删除旧工具：发现 impeccable 或 archify 时，删除所选 provider 下项目级与个人级的旧入口（impeccable 含 hooks）及 `skills-lock.json`、`.agents/.skill-lock.json` 中的记录，不备份；交互终端以 `[Y/n]` 确认，`--keep-impeccable`、`--keep-archify` 保留。`--no-impeccable` 与 `--replace-impeccable` 已移除，传入按未知参数退出。
- Phoenix UI 移除旧版迁移与兼容：`doctor --fix`、旧安装目录清理、`impeccable-disable*` 内联豁免前缀及 `PHOENIX_UI_HOOK_DISABLED`/`_QUIET`/`_LOG` 环境变量不再支持，改用 `.phoenix-ui/config.json` 的 `hook` 设置与 `phoenix-ui-disable*` 前缀。
- Phoenix Diagram 移除 `meta.input_contract`、`--input-contract` 与兼容回执，输入只按各类型 schema 校验；statechart 布局失败时仍在校验结果中给出每条规则的判定与最短路径。
- 新增 create-e2e-verification-skill；create-verification-skill 改写为 Odyssey DSL。dev-plan、review-code、apply-code-review 改为端到端优先：以真实入口场景验证用户可见变更，本地测试需说明理由。

## 0.7.11

- Phoenix Diagram 新增 statechart 类型，图类型增至十一类：用 JSON 声明有限模型（变量、动作、规则、终态），引擎穷举可达状态，判定每条规则与死锁，并画出到每个违例的最短路径；可按平铺、父状态或并行区域展示，回执附 `model` 结果。
- Phoenix Diagram 不再支持 `legacy-2.17` 输入：architecture、workflow、sequence、dataflow、lifecycle 未标记合同时报 `input/contract-required`，已是 phoenix-v1 的 JSON 用 `--input-contract phoenix-v1` 读取。
- 安装器先选定 provider 再写入：选 `.claude` 写 hooks、settings 与 `.claude/agents`，选 `.agents`（及 codex、`.codex`）写 `.agents/agents`；未选 `.claude` 的项目不再写入其 hooks 与 settings，未知 `--providers` 值以退出码 1 结束。
- agi-mode 新增 Subagents 规则；dev-plan、review-dev-plan、review-code 改为功能交付优先，防御与测试按真实风险投入，未明确要求的 NFR 不设专项验证或阻塞。
- Phoenix UI 和 Phoenix Diagram 均从同一提交的一次 CI 构建四平台引擎。Node 18/25 各 7 组安装检查通过。

## 0.7.8

- 修复受管 Phoenix UI 被源码安装覆盖后无法重新安装的问题；恢复后重复安装保持不变。
- 包含近期 CRD、UI 评审、阻塞处理及门户接入与发布 skill 更新。
- Phoenix UI 和 Phoenix Diagram 均从同一提交构建四平台引擎。Node 18/25 联合安装、源码覆盖恢复和重复安装核验通过；本次按要求跳过全回归。

## 0.7.3

### 修复

- 补齐 npm 包的 `skills` 默认入口，使 `npx @xgent-ai/skills install` 能选中安装器。
- 0.7.2 使用上述命令会报 `could not determine executable to run`。使用该旧版本时，可显式指定安装器：

  ```bash
  npx --package=@xgent-ai/skills@0.7.2 xgent-skills install
  ```

## 0.7.2

### 变更

- Phoenix UI 包内容目录由 vendor/phoenix-ui 改为 dist/phoenix-ui。
- 删除原 vendor/impeccable 副本及其 vendor/R2 脚本；旧源码可从 Git 历史查看。保留旧 impeccable 安装的迁移能力与 `--no-impeccable` 参数。

迁移与验证记录见 [Phoenix UI M6](docs/plan/phoenix-ui.records/M6.md)。

## 历史开发记录

以下记录按开发阶段归档，不对应 npm 发布版本。

### 2026-11-06：终止并移除 Agent 指令演化工具

- 原方法论不可行，未产生已证实的指令收益。移除 agi-mode 的远端执行工具（`scripts/evolution/`、`references/remote-evolution.md`）及其使用说明、开发计划与实验报告；Eval、Hillclimb、Arena、Reflection 流程保留。下方记录仅作历史，相关内容见 Git 历史。

### 2026-10-22：Agent 指令演化阶段交付

- v1 在指定 SSH/Docker 组合完成真实隔离、任务对照/搜索、自然请求/CAS 与故障恢复验收；独立确认样本不足，未证明候选优于基线。
- v2 按已有结果阶段交付：工程和已测子范围可用，完整 skill 三条件及 TypeSafe 反馈进入真实 optimizer 尚未验收，没有已证实的指令收益。后续运行需另行核对资源与验收条件。

