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

本仓库同时以 [`@xgent-ai/skills`](https://www.npmjs.com/package/@xgent-ai/skills) 发布到 npm,自带 `xgent-skills` 命令,可为任意项目安装 XGENT 的 Claude Code hooks(当前包含 statusline)并在项目 `.claude/settings.json` 中启用对应配置:

```bash
# 在目标项目根目录执行(也可显式传目录:npx @xgent-ai/skills install <dir>)
npx @xgent-ai/skills install
```

安装是幂等的:hook 文件按内容比对,只在有变化时覆盖;`settings.json` 按顶层 key 合并,已有的其他配置保持不动。

## Skills 列表

| Skill | 说明 |
| --- | --- |
| [dev-plan](skills/dev-plan/SKILL.md) | 以资深产品经理 + 资深架构师的双重视角,基于当前代码库的真实现状撰写高质量开发计划 |

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
