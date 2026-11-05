# Changelog

记录版本变更与历史开发事项；安装和使用方法见 [README.md](README.md)。以下为从 README 迁出的历史记录，不是完整发布历史。

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

### 2026-10-22：Agent 指令演化阶段交付

- v1 在指定 SSH/Docker 组合完成真实隔离、任务对照/搜索、自然请求/CAS 与故障恢复验收；独立确认样本不足，未证明候选优于基线。
- v2 按已有结果阶段交付：工程和已测子范围可用，完整 skill 三条件及 TypeSafe 反馈进入真实 optimizer 尚未验收，没有已证实的指令收益。后续运行需另行核对资源与验收条件。

详细范围、证据及后续安排见 [M4 阶段记录](docs/plan/agent-instruction-evolution-v2.records/M4.md#2026-10-22-阶段交付)。
