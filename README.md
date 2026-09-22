# XGENT Skills

XGENT 的 [Agent Skills](https://skills.sh) 集合,可安装到 Claude Code、Cursor、Codex 等支持 skill 的 Agent 中。

## 安装

安装全部 skills:

```bash
npx skills add XGENT-ai/skills
```

只安装指定 skill:

```bash
npx skills add XGENT-ai/skills --skill dev-plan
```

## Claude Code 辅助工具

本仓库同时以 [`@xgent-ai/skills`](https://www.npmjs.com/package/@xgent-ai/skills) 发布到 npm,自带 `xgent-skills` 命令,可为任意项目安装 XGENT 的 Claude Code hooks(当前包含 statusline)、在项目 `.claude/settings.json` 中启用对应配置,并装上随包 vendor 的 [impeccable](#vendor-的-impeccable):

```bash
# 在目标项目根目录执行(也可显式传目录:npx @xgent-ai/skills install <dir>)
npx @xgent-ai/skills install
```

选项:

| 选项 | 说明 |
| --- | --- |
| `--no-impeccable` | 只装 XGENT 的 hooks 与 settings,跳过 impeccable |
| `--providers=a,b` | 指定 impeccable 装进哪些 harness 目录(如 `--providers=.claude,.cursor`);默认按项目里已有的目录判断,一个都没有时只装 `.claude` |
| `--force` | 强制重装,并允许覆盖非法 JSON 的 hook 配置(先存 `.bak`) |

安装是幂等的:hook 文件与 skill 目录按内容比对,只在有变化时覆盖;`settings.json` 按顶层 key 合并,impeccable 的 hook 按标记剔旧再合并,项目自己的 hook 与其他配置都保持不动。

`install` 只装 hooks、settings 与 impeccable,不生成项目文档。出仓 App 仓库的 `CLAUDE.md` / `PRODUCT.md` / `DESIGN.md` 由 [xgent-init](skills/xgent-init/SKILL.md) skill 生成:`npx skills add XGENT-ai/skills --skill xgent-init`。

## vendor 的 impeccable

[impeccable](https://github.com/pbakaus/impeccable)(Apache-2.0)的 skill bundle 与 engine 二进制整个 vendor 在 `vendor/impeccable/` 里,随 npm 包一起发布,所以 `npx @xgent-ai/skills install` 全程不联网 —— 官方的 `npx impeccable install` 要先下 15 MB 的 bundle、首次跑 hook 时再下十几 MB 的 engine,墙内经常卡在 `Download failed`。

装出来的东西和上游 `impeccable install` 的工程内安装一致(`impeccable doctor` 报 no drift):

- `<harness>/skills/impeccable/`、`<harness>/agents/`、`<harness>/commands/`:按项目里已有的 harness 目录装,可用 `--providers` 指定;
- hook manifest:Claude Code 写 `.claude/settings.local.json`(共享的 `settings.json` 里已有 impeccable hook 时以它为准),Cursor 写 `.cursor/hooks.json`,Codex 写 `.codex/hooks.json`,Copilot / Grok 写各自的 `hooks/impeccable.json`;
- engine 二进制:只收 macOS(arm64 与 x64),按当前平台放进 `~/.impeccable/bin/<版本>/`,一台机器一份,所有项目和 `npx impeccable` 共用;非 mac 平台跳过这一步,由 launcher 首次运行时自己下载。

升级 vendor 的版本(维护者执行,需要 curl 与 unzip):

```bash
node scripts/vendor-impeccable.mjs
```

脚本会取上游最新 release,按 ed25519 签名验 bundle、按 `.sha256` 验 engine 二进制,全部通过才落盘,并把版本与校验和写进 `vendor/impeccable/VERSION.json`。要加别的平台,改脚本里的 `ENGINE_TARGETS`。当前收录:skill 4.3.1 / engine 0.1.5(darwin-arm64、darwin-x64)。上游许可与三方声明见 `vendor/impeccable/LICENSE` 与 `vendor/impeccable/NOTICE.md`。

## Skills 列表

| Skill | 说明 |
| --- | --- |
| [dev-plan](skills/dev-plan/SKILL.md) | 以资深产品经理 + 资深架构师的双重视角,基于当前代码库的真实现状撰写高质量开发计划 |
| [xgent-init](skills/xgent-init/SKILL.md) | 在出仓 App 自己的仓库里生成配套的 `CLAUDE.md` / `PRODUCT.md` / `DESIGN.md`,读清单与代码事实、一轮问清缺的,不留待填占位 |

## 目录结构

每个 skill 位于 `skills/<name>/` 目录下,包含一个带 `name` 与 `description` frontmatter 的 `SKILL.md`:

```
skills/
└── dev-plan/
    ├── SKILL.md          # skill 主体(必需)
    ├── references/       # 补充参考文档
    ├── scripts/          # 辅助脚本
    └── evals/            # 评测用例
```

## License

[MIT](LICENSE)
