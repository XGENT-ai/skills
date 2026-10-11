# XGENT Phoenix UI：安装、迁移与回滚

Phoenix UI 是 XGENT 自主维护的设计工具，承接 [impeccable](https://github.com/pbakaus/impeccable) skill 4.5.0 / engine 0.1.11 的固定源码（Apache-2.0）。版本、规则、构建和发布都由本仓控制：不查询上游 latest，不从上游或 impeccable.style 下载任何东西。来源、修改标识与三方许可见包内 `vendor/phoenix-ui/NOTICE.md`、`THIRD-PARTY-NOTICES.md`。

## 安装

Phoenix 只随 `@xgent-ai/skills` 的 npm 安装器分发：

```bash
# 在目标项目根目录执行
npx @xgent-ai/skills install
```

一次安装分两段，先完成全部只读预检再写项目：

1. **预检**：校验参数、`.claude/settings.json` 与 guard 配置；把包内 `vendor/phoenix-ui/VERSION.json` 固定的 engine 下载或从 `--release-dir` 读取，按大小、SHA-256 和 `engine-probe` 身份核对后放进用户 cache；再以 dry-run 计算 Phoenix 的写入、合并与迁移。任何一项失败都不写项目。
2. **写入**：先写 XGENT 部分（statusline hook、settings、可选 guard 与 AGENTS.md），再由 Rust 安装执行器在项目锁内重读这些文件，事务化写入 Phoenix 的 skill、agents、hooks、五个辅助 skill 和安装回执 `.phoenix-ui/install/active.json`。两段之间被打断时，已完成的 XGENT 部分保留、Phoenix 不激活，直接重跑同一命令即可。Phoenix 写入中途失败（如磁盘已满）时状态为「待恢复」，可能已部分写入；重跑同一命令会先按日志回滚或补完上次事务再安装。

安装输出逐项列出写入、合并与移除，状态为「已安装 / 未变 / 冲突,未激活 / 待恢复 / 已安装但融合未就绪」之一，并给出事务 ID、备份位置、完整回滚命令和各工具的重载步骤。只有状态为「已安装」或「未变」时才打印「完成」；冲突与融合未就绪以退出码 3 结束，engine 或文件环境失败以 4 结束，参数错误为 1（XGENT 部分）或 2（Phoenix 执行器）。

| 选项 | 作用 |
| --- | --- |
| `--no-phoenix-ui` | 跳过 Phoenix，仍安装 XGENT 部分；不删除已装版本 |
| `--no-impeccable` | 已弃用，等同 `--no-phoenix-ui`，保留一个迁移主版本 |
| `--release-dir=DIR` | 离线安装 engine：DIR 是发布者整理的完整 release 目录（`VERSION.json`、各平台 engine、许可与清单），必须与当前包的 manifest 完全一致，零网络 |
| `--providers=a,b` | 指定安装目标目录；`codex` 同时选择 `.agents`（skills）与 `.codex`（hooks）。默认按项目里已有的目录判断，都没有时只装 `.claude` |
| `--force` | 只影响 guard 的非法 JSON 备份覆盖；不放宽 Phoenix 的冲突保护 |

**engine**：放在 `~/.phoenix-ui/bin/<版本>/<平台>/`（可用 `PHOENIX_UI_HOME` 改 engine cache 的根目录；提醒去重标记也在其中），一台机器每个版本一份。hook、context 等普通运行不联网补下载：engine 缺失或版本不符时 hook 放行并每会话提醒一次补装命令，CLI 返回 4 并给出固定包版本的命令：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui engine install --project <项目>
```

终端命令统一通过 `npm exec --package=@xgent-ai/skills -- phoenix-ui <verb>` 调用；对话里的设计工作流用 `/phoenix-ui`（Codex 为 `$phoenix-ui`），二者不要混用。

## 从 impeccable 迁移

安装器只认领本包已发布版本（`@xgent-ai/skills` 0.3.0–0.6.0，对应 skill 4.3.1/4.5.0）写入且未被改动的文件和 hook：

- 未改动的旧 skill、agents、commands 移入 `.phoenix-ui/migrations/<事务ID>/` 备份；旧 hook 分组或嵌套 handler 按完整结构精确匹配后移除，邻近的用户 handler、matcher 与未知字段原样保留。
- 每个旧 skill 位置留一个限期提示入口 `impeccable`，只在被显式调用时列出新命令，不挂 hook、不调用旧工具；下一个主版本移除。
- `.impeccable/` 中可迁移的配置、设计 sidecar 与已核验的数据复制到 `.phoenix-ui/`，未知键原样保留并报告「未迁移」；固定快捷 pin 重新生成为 Phoenix 版本。
- `AGENTS.md`、`CLAUDE.md`、`DESIGN.md`、`PRODUCT.md` 中的旧入口只报告行号与替换建议，不自动改写。
- 个人级目录里早期上游版本的分命令 skill（如 skills.sh 装到 `~/.agents/skills` 的 `polish`、`teach-impeccable`）没有 hook 或 engine，只报告位置与移除、重载步骤，不阻止激活。

以下情况会阻止激活（退出码 3，项目不被修改），按输出逐项处理后重跑：

| 情况 | 处理 |
| --- | --- |
| 旧 skill 或旧 hook 被用户改过、被 shell 包装，或结构无法唯一确定 | 输出列出文件与条目；先备份/还原或移出活跃目录，再重跑 |
| 旧 live 会话、注入或变体残留 | 用旧项目的 launcher 正常停止、`live-inject --remove`、接受或丢弃变体；不要按 PID 杀进程 |
| 个人级（如 `~/.claude/skills/impeccable`）旧入口、旧 hook 或插件 | 安装器只读发现、不改个人配置；按输出在对应工具中禁用或移除后重载并重跑 |
| 外部管理的辅助 skill 副本与本包不同（含 skills.sh 副本、符号链接） | Phoenix 仍激活，状态为「已安装但融合未就绪」；按输出用 `npx skills add XGENT-ai/skills --skill <名称>` 更新，或备份后移出活跃目录再重跑 |

## 回滚与恢复

安装输出中的回滚命令只撤销该事务写入的条目：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui migrate --rollback <事务ID> --project <项目>
```

- 独占文件按迁移后的摘要核对后还原；共享 JSON 只逆转本次加入或移除的条目，后来新增的 allow、guard 阈值和未知字段保留。受管条目被改过时返回 3 并列出冲突，不强制回退。
- 回滚不撤销在 Phoenix 中做的产品代码修改，也不把新数据格式塞回旧工具；`.phoenix-ui/` 中的备份、日志与新数据保留，确认稳定后可手动删除。
- 进程中断后，下一次安装会先读日志恢复；也可显式执行 `phoenix-ui migrate --recover --project <项目>`。遇到无法确认的新改动时停下并列出冲突，不覆盖。

一致性保证覆盖进程崩溃或强制退出；不承诺突然掉电后零丢失，掉电损坏时从备份人工恢复。

## 支持矩阵

| 平台 | engine 资产 |
| --- | --- |
| macOS Apple Silicon / Intel | `darwin-arm64` / `darwin-x64` |
| Linux x64 / ARM64 | `linux-x64` / `linux-arm64` |

本期支持 macOS 和 Linux。Windows 的发布资产、hook 会话及 ARM64 兼容验收延期。

其他平台在预检阶段以退出码 4 停止，不写项目；可加 `--no-phoenix-ui` 安装其余部分。用户不需要 Rust 编译器。

| 目录 | 工具 | 写入 | 安装后 |
| --- | --- | --- | --- |
| `.claude` | Claude Code | skills、agents、`.claude/settings.json` hooks | 重启会话，在 `/hooks` 核对 Phoenix 设计 hook |
| `.agents` + `.codex` | Codex | `.agents/skills`、`.codex/hooks.json` | 在 Codex 中运行 `/hooks`，重新审阅并信任本项目的 Phoenix handler |
| `.cursor` | Cursor | skills、`.cursor/hooks.json` | 重新打开会话 |
| `.github` | GitHub Copilot | skills、`.github/hooks/phoenix-ui.json` | 重新打开会话 |
| `.grok` | Grok | skills、`.grok/hooks/phoenix-ui.json` | 重新打开会话 |
| `.gemini` | Gemini CLI | skills、`.gemini/settings.json` hooks | 重新打开会话 |
| `.agent`、`.dsh`、`.hermes`、`.kiro`、`.opencode`、`.pi`、`.qoder`、`.rovodev`、`.trae`、`.trae-cn`、`.veto`、`.vibe` | 对应工具 | skills（无 hooks） | 重新打开会话 |

Phoenix 的 hook 按完整分组写入；同一事件里已有其他工具或 goal guard 的分组（包括都没有 matcher 的分组）互不影响，重复安装不产生重复 handler。安装器测试覆盖各目录的文件与 hook 合并；工具真实加载以各自会话为准，同名 skill 的优先级按工具规则（Claude Code 个人级优先于项目级），安装器无法确定时会提示待人工核对。

## 渠道

- npm：`@xgent-ai/skills` 包含 Phoenix 运行资源（skill bundle、`VERSION.json`、许可），engine 二进制按 manifest 从固定地址获取或离线目录提供，不进 npm 包。
- skills.sh：五个辅助 skill（design-md、design-reference、ui-pattern-research、review-ui、apply-ui-review）继续公开，可单独安装；`phoenix-ui` skill 标为 internal，只通过 npm 安装器分发。没有 Phoenix CLI 时，辅助 skill 按各自的人工流程工作。
- 原 `vendor/impeccable/` 与 `scripts/vendor-impeccable.mjs`、`scripts/publish-vendor-r2.mjs` 只作来源历史保留，不是安装或更新入口。
