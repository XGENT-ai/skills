# Codex goal 上下文收尾提醒

当本地估算上下文使用率**严格超过 65%**且当前根线程的 goal 为 `active` 时，本 hook 提醒 agent 收尾：更新已有任务记录、交接未完成项、按用户明确的暂停策略调用原生 goal 工具，再提醒用户自行新开对话。恰好 65% 不触发。hook 只读取本地数据，不写进度、修改 goal 或创建对话。

## 兼容范围与限制

首期格式限定为 Codex CLI / App Server **0.160.0**。已验证环境为 macOS arm64、Python 3.13.7；运行服务标识及完整验收见 [M1 记录](plan/codex-context-goal-guard.records/M1.md)与 [M3 记录](plan/codex-context-goal-guard.records/M3.md)。Python 至少需要 3.9。后续 Codex 版本不自动视为兼容，未知格式会报告检测不可用。

估算值来自当前会话最新完整 `token_count` 的 `last_token_usage.total_tokens / model_context_window`，可能滞后于 UI。累计 tokens、cached tokens 和 reasoning tokens 不重复加入。压缩或模型/窗口改变后，没有新统计时不沿用旧值。子 agent 跳过；fork 只读取自己的 goal。读取位置由真实 hook 输入和 `CODEX_HOME` 决定，未设置时使用 `~/.codex`。

检查发生在同步 `PreToolUse` 和 `Stop` 边界；已经调度的调用可能完成，部分工具路径没有 tool hook。提醒依赖 agent 执行，不能强制暂停。首次 Stop 可以请求补做收尾，第二次仍未暂停时只告警；此告警不能阻止 goal 自动续跑。上述事件、信任及输出语义见 [官方 Hooks 文档](https://learn.chatgpt.com/docs/hooks)。

## 获取与部署

### 使用 install

功能已接入 `install` 源码和 npm 打包清单，尚未发布到 npm。当前从本仓库根目录运行：

```sh
node bin/xgent-skills.js install /absolute/path/to/project
# 显式选择安装，不再询问此选项
node bin/xgent-skills.js install /absolute/path/to/project --context-goal-guard
```

发布包含此功能的新版本后，可用 `npx @xgent-ai/skills install`。交互安装会询问是否安装 Codex goal 上下文收尾提醒，输入 `y` 或 `yes` 确认；回车或非交互环境默认跳过。`--context-goal-guard` 显式安装，`--no-context-goal-guard` 跳过询问并保留已有 guard；两者不能同时使用。此选择独立于 `--providers`、`--no-impeccable` 和 `--xgent-init`。

安装器检查 `python3` 为 3.9+ 且包含 SQLite 标准库，然后复制脚本到目标项目 `.codex/hooks/context-goal-guard.py`，将两个同步 handler 合并到 `.codex/hooks.json`，使用项目规范绝对路径和 3 秒超时。其他事件、handler、matcher 和顶层字段保留；重复安装更新 guard，不重复添加。非法 JSON 默认报错，`--force` 可先备份为 `.bak` 再覆盖；合法 JSON 的未知结构需手动修复。

Python 缺失或版本不足时，按安装器提示先安装 Python 并运行 `python3 --version` 核对，再重试；也可用 `--no-context-goal-guard` 完成其他安装。Codex 运行环境也必须能解析 `python3`，否则按下方手动配置说明改为解释器绝对路径。

安装结束会打印下方「信任与生效确认」的操作步骤及可复制的当前 goal 暂停策略。**脚本已部署不等于已生效**：用户仍需在 Codex 内信任项目、审阅并信任启用两个 handler、明确当前 goal 的暂停策略，并用测试 goal 验证真实暂停。安装器不代替用户完成这些步骤。

### 手动部署

也可单独复制仓库文件。首期手动部署的精确本地验证 revision 记在 [M3 记录](plan/codex-context-goal-guard.records/M3.md)：在可访问该本地仓库的机器 clone 并 checkout 该 revision（该 revision 不含后续 install 集成）。远程发布后，才可从 `https://github.com/XGENT-ai/skills.git` checkout 对应已发布 revision；不要将尚未发布的 SHA 当作远程可获取版本。

```sh
git clone /absolute/path/to/skills /tmp/skills-guard-source
git -C /tmp/skills-guard-source checkout <M3记录中的本地验证revision>
```

先核对 `codex --version` 和 `python3 --version`。以下路径均替换为实际目录；从项目根目录取得规范绝对路径（例如 `pwd -P`），避免 `/var` 与 `/private/var` 等别名导致信任来源不一致。

```sh
mkdir -p "/absolute/path/to/My Project/.codex/hooks"
cp "/tmp/skills-guard-source/hooks/codex/context-goal-guard.py" "/absolute/path/to/My Project/.codex/hooks/context-goal-guard.py"
```

打开目标项目 `.codex/hooks.json`，将 [配置示例](../hooks/codex/hooks.example.json)中的两组 handler **追加**到已有 `PreToolUse` 和 `Stop` 数组；没有文件或事件时才新建对应对象。保留其他事件、handler、matcher 和顶层字段。不要用整个示例覆盖已有 manifest。

将两个 command 中的 `/path/to/My Project` 替换为项目规范绝对路径，保留 JSON 内转义的双引号：

```json
{"type": "command", "command": "python3 \"/absolute/path/to/My Project/.codex/hooks/context-goal-guard.py\"", "timeout": 3}
```

若 Codex 环境无法解析 `python3`，将其换为已核对的 Python 3 可执行文件绝对路径，也保留必要引号。普通含空格路径已列入验收；包含 shell 元字符的路径应另行正确引用并验证，不直接粘贴到 command。同步超时为 3 秒。

## 信任与生效确认

1. 从目标项目启动 Codex，审阅并信任项目 `.codex/` 层。
2. 打开 `/hooks`，找到来源为本项目 `.codex/hooks.json` 的两个 guard handler；审阅命令、路径、事件与超时，分别信任并启用。
3. 再次确认两个 handler 的来源、启用和信任状态；确认其他 hooks 仍在列表中。未信任时会被跳过，安静无告警不能证明已生效。
4. 修改 hook 定义或 command 路径后，重新在 `/hooks` 审阅信任。信任绑定定义 hash，不伪造 hash。替换脚本内容也需自行重新审阅；不要假定定义 hash 会检测脚本内容变化。
5. 在隔离项目、测试 goal 中验证提示和真实暂停后，再用于日常任务。可从项目子目录启动，command 的绝对路径仍有效。

安装、信任和阅读说明都不构成暂停授权。在**当前 goal** 的用户指令里明确，例如：

> 对当前 goal，如果本地估算上下文使用率严格超过 65%，请先安全收尾，按已有任务约定保存进度和交接说明，再使用原生 goal 工具将其设为 paused 并核对，无需二次确认。未完成工作不要标为 complete。请提醒我自己新开对话继续。

后续 `/goal resume` 撤销先前暂停请求，需重新明确策略。普通命令测试可使用 `features.code_mode.enabled=true` 和 `features.code_mode.direct_only_tool_namespaces=["functions"]`；code mode 使用默认路径。M1 验证了两者；仅关闭 code mode 不保证普通命令回退。配置字段见 [官方配置参考](https://learn.chatgpt.com/docs/config-file/config-reference)。

## 收尾与人工接续

成功回复应说明真实 `paused` 状态、已有记录位置、剩余任务和下一步。无既有记录时，agent 在回复中给出交接摘要（完成/未完成、验证/未验证、下一步、仍运行的进程），不强制创建新文件。进度写入失败时，也先在回复保留最小摘要并明确失败。

在同一项目目录自行新开对话，提供已有计划路径，或粘贴交接摘要并说明继续。新对话没有继承的 goal；需要用户手动设置 goal、重新明确 65% 暂停策略后，guard 才对它生效。旧 goal 保持 paused；可保留，或回旧对话主动 `/goal clear`。恢复旧 goal 不清空上下文，不作为腾出上下文的办法。线程生命周期命令见 [官方 Goals 文档](https://developers.openai.com/cookbook/examples/codex/using_goals_in_codex)。

如果 agent 请求确认、拒绝暂停或暂停失败，仍算尚未暂停；在原对话执行 `/goal pause` 后再接续。不要依据“已提醒”判断暂停成功。

## 排查与停用

| 表现 | 检查与处理 |
| --- | --- |
| `/hooks` 没有 guard 或显示待审/禁用 | 核对项目层信任、来源、manifest JSON、绝对 command 路径；重新审阅信任并启用 |
| “检测不可用” | 核对 Codex 版本、进程实际 `CODEX_HOME`、会话和 `goals_1.sqlite` 的存在与读权限；不手工补库或改状态。格式/schema/锁异常持续时在 `/hooks` 停用 guard |
| hook 超时/失败提示 | 本次检测失效，工具可能继续。检查 Python、文件读取与存储延迟，在隔离环境重验后再调整 3 秒上限 |
| 超过 65% 但没有提示 | 核对当前根线程是否有 active goal、新统计是否已记录及工具事件覆盖；不能用 UI 百分比直接代替本地统计 |
| Stop 后仍 active 或持续续跑 | 在原对话 `/goal pause`；检查暂停授权和其他 Stop hooks。其他 hooks 的继续/停止要求须组合实测，不能宣称所有组合兼容 |

临时停用：在 `/hooks` 中仅禁用本 guard 的两个 handler。永久移除：从两个事件数组中只删除 command 指向 `context-goal-guard.py` 的 handler，保留其他 hooks；确认不再引用后可删除该脚本副本。任务记录保留，paused goal 不自动恢复，不需要数据库迁移或回填。

## 本地验证

```sh
python3 -m unittest discover -s test/codex -p 'test_*.py'
python3 -m py_compile hooks/codex/context-goal-guard.py
```

行为测试通过真实脚本入口、临时 JSONL 和只读 SQLite 验证阈值、身份、无写入与异常降级；真实 Codex 部署和接续证据位于上述里程碑记录。fixture 不能替代真实阈值与调度验收。
