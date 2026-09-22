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

[impeccable](https://github.com/pbakaus/impeccable)(Apache-2.0)整个 vendor 在 `vendor/impeccable/` 里:15 MB 的 skill bundle 随 npm 包发布,十几 MB 的 engine 二进制不进包、改放自家 R2,由 `install` 按 `VERSION.json` 里的地址与 sha256 取。官方的 `npx impeccable install` 要先从 GitHub 下 bundle、首次跑 hook 时再下 engine,墙内经常卡在 `Download failed`;这里 bundle 一步不联网,engine 只走一次 R2(装过一次就落在 `~/.impeccable/` 里,之后都不用了)。

装出来的东西和上游 `impeccable install` 的工程内安装一致(`impeccable doctor` 报 no drift):

- `<harness>/skills/impeccable/`、`<harness>/agents/`、`<harness>/commands/`:按项目里已有的 harness 目录装,可用 `--providers` 指定;
- hook manifest:Claude Code 写 `.claude/settings.local.json`(共享的 `settings.json` 里已有 impeccable hook 时以它为准),Cursor 写 `.cursor/hooks.json`,Codex 写 `.codex/hooks.json`,Copilot / Grok 写各自的 `hooks/impeccable.json`;
- engine 二进制:只收 macOS(arm64 与 x64),按当前平台放进 `~/.impeccable/bin/<版本>/`,一台机器一份,所有项目和 `npx impeccable` 共用。缓存里已是对的那份就不再下;从本仓源码跑时直接用 `vendor/impeccable/engine/` 里的,不联网。非 mac 平台、以及 R2 拉不动时都只是跳过这一步,安装照常完成,由 launcher 首次运行时自己下载。

升级 vendor 的版本(维护者执行,需要 curl、unzip 与 aws CLI):

```bash
node scripts/vendor-impeccable.mjs    # 抓上游 bundle + engine 到 vendor/
node scripts/publish-vendor-r2.mjs    # engine 传 R2,下载地址回写 VERSION.json
```

第一步取上游最新 release,按 ed25519 签名验 bundle、按 `.sha256` 验 engine 二进制,全部通过才落盘,并把版本与校验和写进 `vendor/impeccable/VERSION.json`。要加别的平台,改脚本里的 `ENGINE_TARGETS`。

第二步把 engine 传到 R2 的 `vendor/impeccable/engine/v<版本>/<平台>/impeccable`,传完回读公共地址核一遍 sha256,再把 url 写进 `VERSION.json` 的 `engines`。凭据读仓库根的 `.env.cf`(`AGENT_RELEASE_R2_*`,已 gitignore,不在库里)。两步的顺序不能反 —— 第一步会重写 `VERSION.json`,先传就把 url 冲掉了;漏了第二步,发出去的包里 engine 没有下载地址,用户那边只会看到"跳过"。

当前收录:skill 4.3.1 / engine 0.1.5(darwin-arm64、darwin-x64)。上游许可与三方声明见 `vendor/impeccable/LICENSE` 与 `vendor/impeccable/NOTICE.md`。

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
