# Changelog

记录版本变更与历史开发事项；安装和使用方法见 [README.md](README.md)。以下为从 README 迁出的历史记录，不是完整发布历史。

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
