# XGENT Skills

XGENT 的 [Agent Skills](https://skills.sh) 集合,可安装到 Claude Code、Cursor、Codex 等支持 skill 的 Agent 中。

## 安装

列出并选择安装公开的 skills (`xgent-init` 是内部初始化 skill,不在默认列表中):

```bash
npx skills add XGENT-ai/skills
```

只安装指定 skill:

```bash
npx skills add XGENT-ai/skills --skill dev-plan
```

## 项目辅助工具（Claude Code / Codex）

本仓库同时以 [`@xgent-ai/skills`](https://www.npmjs.com/package/@xgent-ai/skills) 发布到 npm,自带 `xgent-skills` 命令,可为任意项目安装 XGENT 的 Claude Code hooks(当前包含 statusline)、在项目 `.claude/settings.json` 中启用对应配置,装上随包 vendor 的 [impeccable](#vendor-的-impeccable),并可选创建通用的 `AGENTS.md`、安装 Codex 或 Claude Code 的 goal 上下文收尾提醒:

```bash
# 在目标项目根目录执行(也可显式传目录:npx @xgent-ai/skills install <dir>)
npx @xgent-ai/skills install
```

选项:

| 选项 | 说明 |
| --- | --- |
| `--no-impeccable` | 跳过 impeccable,仍安装 XGENT 的 hooks 与 settings |
| `--xgent-init` | 直接创建仓根 `AGENTS.md` (已有则跳过),不再询问 |
| `--no-xgent-init` | 跳过创建 `AGENTS.md`,不再询问 |
| `--context-goal-guard` | 直接安装 Codex goal 上下文收尾提醒,不再询问;需要 Python 3.9+ |
| `--no-context-goal-guard` | 跳过 Codex goal guard,不再询问;已有 guard 保留 |
| `--context-goal-guard-threshold=N` | Codex guard 的触发阈值百分比(1–99 整数,默认 65),同时表示安装;重装不给则沿用已装的值 |
| `--claude-context-goal-guard` | 直接安装 Claude Code goal 上下文收尾提醒,不再询问 |
| `--no-claude-context-goal-guard` | 跳过 Claude Code goal guard,不再询问;已有 guard 保留 |
| `--claude-context-goal-guard-threshold=N` | Claude Code guard 的触发阈值百分比(1–99 整数,默认 70),同时表示安装;重装不给则沿用已装的值 |
| `--providers=a,b` | 指定 impeccable 装进哪些 harness 目录(如 `--providers=.claude,.cursor`);默认按项目里已有的目录判断,一个都没有时只装 `.claude` |
| `--force` | 强制重装,并允许覆盖非法 JSON 的 hook 配置(先存 `.bak`);不覆盖已有 `AGENTS.md` |

安装是幂等的:hook 文件与 skill 目录按内容比对,只在有变化时覆盖;`settings.json` 按顶层 key 合并,impeccable 的 hook 按标记剔旧再合并,项目自己的 hook 与其他配置都保持不动。

`install` 会询问是否同时创建仓根 `AGENTS.md`,输入 `y` 或 `yes` 确认,回车默认跳过。确认后直接复制 [通用模板](skills/xgent-init/references/external-app-AGENTS.template.md),只创建 `AGENTS.md`,不安装 `xgent-init` skill,不生成 `CLAUDE.md` / `PRODUCT.md` / `DESIGN.md`。已有 `AGENTS.md` 时直接跳过并保留原文,包括使用 `--force` 时。

非交互环境默认跳过,可用 `--xgent-init` 显式创建,或用 `--no-xgent-init` 跳过询问。文件位于仓根,不受 `--providers` 影响。模板没有待填占位,通用编码准则支持不同 coding agent;Portal App 的 onebox 联调与有前端改动时的真实浏览器验收分别按适用条件执行。

`install` 还会询问是否安装 Codex goal 上下文收尾提醒,输入 `y` 或 `yes` 安装,回车和非交互环境默认跳过。此选项不受 `--providers` 或 `--no-impeccable` 影响。请安装到 Git 仓库根目录;Codex 运行环境需要 git 和 Python 3.9+。确认后先检查含 SQLite 标准库的 Python 3.9+,复制脚本到 `.codex/hooks/context-goal-guard.py`,合并 `.codex/hooks.json` 的同步 `PreToolUse` / `Stop` handler（3 秒超时）,运行时从 Git 根目录定位脚本,配置不包含本机项目绝对路径,保留其他 hooks 与配置;重复安装只更新 guard,不重复添加。

安装后会显示用户必须完成的步骤:从目标项目启动 Codex、信任项目 `.codex/` 层,在 `/hooks` 中审阅、信任并启用两个 handler,对**当前 goal** 明确超过阈值(默认 65%)时先保存进度和交接、再暂停的策略。安装输出提供可复制的指令;安装和信任本身不构成暂停授权。恢复 goal 或新开对话后需重新明确策略。当前仅验证 Codex 0.160.0、macOS arm64,完整限制与停用方法见 [启用说明](docs/codex-context-goal-guard.md)。

Codex guard 已接入源码与 npm 打包清单,尚未发布到 npm;发布后可用 `npx @xgent-ai/skills install --context-goal-guard`。当前可从本仓库源码为目标项目安装:

```bash
node bin/xgent-skills.js install /absolute/path/to/project --context-goal-guard
```

`install` 随后询问是否安装 Claude Code goal 上下文收尾提醒,同样输入 `y` 或 `yes` 安装,回车和非交互环境默认跳过,不受 `--providers` 或 `--no-impeccable` 影响。确认后复制脚本到 `.claude/hooks/context-goal-guard.js`,在个人的 `.claude/settings.local.json` 合并 `PostToolBatch` / `Stop` 两个 handler(5 秒超时),保留其他 hooks 与设置;重复安装只更新 guard。guard 只读当前会话记录:本会话 `/goal` 仍 active 且估算上下文严格超过阈值(默认 70%)时提醒 agent 按既有约定更新进度并交接,回合结束时由 Stop hook 结束回合,Claude Code 原生把 goal 置为 Goal paused,再由你在同一项目目录新开对话或 `/clear` 后重新设置 goal。hook 不写任何文件,也不清除或转移 goal。

安装后需重启会话、信任工作区,并在 `/hooks` 中确认两个 handler 来源为 local settings。当前仅验证 Claude Code 2.1.288 交互式 CLI、macOS arm64 与 Opus 5.5 / Sonnet 5.5 / Haiku 4.5,完整限制、接续与停用方法见 [启用说明](docs/claude-context-goal-guard.md)。从源码安装:`node bin/xgent-skills.js install /absolute/path/to/project --claude-context-goal-guard`。

Portal 的产品和设计规范仍保留在 [PRODUCT 模板](skills/xgent-init/references/external-app-PRODUCT.template.md) 与 [DESIGN 模板](skills/xgent-init/references/external-app-DESIGN.template.md) 中。需要生成项目上下文时,显式安装并在 Agent 对话中使用 [xgent-init](skills/xgent-init/SKILL.md),它读取清单与代码事实,补齐 PRODUCT.md / DESIGN.md (无前端的 service 型不新生成 DESIGN.md),已有文件保留:

```bash
npx skills add XGENT-ai/skills --skill xgent-init
```

## vendor 的 impeccable

[impeccable](https://github.com/pbakaus/impeccable)(Apache-2.0)整个 vendor 在 `vendor/impeccable/` 里:skill bundle 以去重形态随 npm 包发布,十几 MB 的 engine 二进制不进包、改放自家 R2,由 `install` 按 `VERSION.json` 里的地址与 sha256 取。官方的 `npx impeccable install` 要先从 GitHub 下 bundle、首次跑 hook 时再下 engine,墙内经常卡在 `Download failed`;这里 bundle 一步不联网,engine 只走一次 R2(装过一次就落在 `~/.impeccable/` 里,之后都不用了)。

`vendor/impeccable/bundle/` 不是 `universal.zip` 解开的样子,而是它的去重形态:文件内容存在 `blobs/<sha256>`,`manifest.json` 里每个 harness 一张 `路径 → sha` 的清单。上游给 19 个 harness 目录各放了一整套 skill,其中 83% 的字节是同一批文件(光 `scripts/data/font-index.json` 就是 1.1 MB × 19 份、内容完全相同),各 harness 真正不同的只有 43 个路径 —— 有的差在路径 token(`.claude/skills/...` vs `.cursor/skills/...`),有的差在按 harness 改写过的措辞(有 `AskUserQuestion` 工具的 harness 写"调用该工具",没有的写"直接问用户")。`install` 按清单把文件写回去,装出来的结果与直接展开 zip 逐字节一致(上游 `impeccable doctor` 报 no drift)。当前 bundle 展开约 39.4 MiB,去重后约 6.7 MiB。

装出来的东西和上游 `impeccable install` 的工程内安装一致(`impeccable doctor` 报 no drift):

- `<harness>/skills/impeccable/`、`<harness>/agents/`、`<harness>/commands/`:按项目里已有的 harness 目录装,可用 `--providers` 指定;
- hook manifest:Claude Code 写 `.claude/settings.local.json`(共享的 `settings.json` 里已有 impeccable hook 时以它为准),Cursor 写 `.cursor/hooks.json`,Codex 写 `.codex/hooks.json`,Copilot / Grok 写各自的 `hooks/impeccable.json`;
- engine 二进制:只收 Apple Silicon macOS(darwin-arm64),放进 `~/.impeccable/bin/<版本>/`,一台机器一份,所有项目和 `npx impeccable` 共用。缓存里已是对的那份就不再下;从本仓源码跑时直接用 `vendor/impeccable/engine/` 里的,不联网。Intel Mac、其它平台以及 R2 拉不动时都只是跳过这一步,安装照常完成,由 launcher 首次运行时自己下载。

升级 vendor 的版本(维护者执行,需要 curl、unzip 与 aws CLI):

```bash
node scripts/vendor-impeccable.mjs    # 抓上游 bundle + engine 到 vendor/
node scripts/publish-vendor-r2.mjs    # engine 传 R2,下载地址回写 VERSION.json
```

第一步取上游最新 release,按 ed25519 签名验 bundle、按 `.sha256` 验 engine 二进制,全部通过才落盘,并把版本与校验和写进 `vendor/impeccable/VERSION.json`;bundle 在落盘前按内容去重成 `blobs/` + `manifest.json`。要加别的平台,改脚本里的 `ENGINE_TARGETS`。

第二步把 engine 传到 R2 的 `vendor/impeccable/engine/v<版本>/<平台>/impeccable`,传完回读公共地址核一遍 sha256,再把 url 写进 `VERSION.json` 的 `engines`。凭据读仓库根的 `.env.cf`(`AGENT_RELEASE_R2_*`,已 gitignore,不在库里)。两步的顺序不能反 —— 第一步会重写 `VERSION.json`,先传就把 url 冲掉了;漏了第二步,发出去的包里 engine 没有下载地址,用户那边只会看到"跳过"。

当前收录:skill 4.5.0 / engine 0.1.11(darwin-arm64)。上游许可与三方声明见 `vendor/impeccable/LICENSE` 与 `vendor/impeccable/NOTICE.md`。

## Skills 列表

以下按用途分组列出 `skills/` 中的 skill。点击名称查看使用条件与完整流程。

### 需求、计划与评审

| Skill | 说明 |
| --- | --- |
| [prd](skills/prd/SKILL.md) | 撰写或优化 PRD，明确产品目标、FR/NFR、UAT、分期与终验，可按授权录入 SPMS |
| [dev-plan](skills/dev-plan/SKILL.md) | 基于需求、仓库规则与代码事实撰写或评审可执行、可跨对话续做的开发计划 |
| [test-plan](skills/test-plan/SKILL.md) | 从需求与验收标准生成测试用例，覆盖正常、边界、权限和并发场景，可按授权写入 SPMS |
| [story-points](skills/story-points/SKILL.md) | 按客观因子估算故事点，回写 SPMS 规划点数并复核 Sprint 容量 |
| [review-prd](skills/review-prd/SKILL.md) | 核实代码现状，评审产品价值、体验、AI、SMAR、UAT 与分期终验，默认将完整报告落盘并回读核验 |
| [review-dev-plan](skills/review-dev-plan/SKILL.md) | 评审开发计划的代码事实、技术方案、需求映射、依赖编排与交付可用性，输出评审报告 |
| [review-code](skills/review-code/SKILL.md) | 评审未提交变更、commit 或分支差异，可对照开发计划，输出有证据的代码评审报告 |
| [review-prd-dev-gaps](skills/review-prd-dev-gaps/SKILL.md) | 核对 PRD 与开发交付，将遗漏、合理变更和待拍板差异分流，交互 Triage 后按需调用 dev-plan 生成 gaps plan |
| [apply-code-review](skills/apply-code-review/SKILL.md) | 核实代码评审意见，实施合理修复、验证结果并回写实现记录与必要进度 |
| [apply-doc-review](skills/apply-doc-review/SKILL.md) | 根据 PRD 或开发计划评审报告修订原文，复核关联内容并记录逐条处置结果 |

把上述 skill 串成完整交付流水线（想法 → PRD → 分期 → 每期开发与并行测试 → 交付）的方法见 [AI Native SDLC](docs/ai-native-sdlc.md)，含各环节产物路径约定与测试线待补环节的现状核对。

### 开发、调试与质量

| Skill | 说明 |
| --- | --- |
| [architect](skills/architect/SKILL.md) | 实现前先设计类型、签名与模块结构，并在开发过程中持续校准 |
| [debugging](skills/debugging/SKILL.md) | 按四阶段流程调查根因、定位问题并验证修复 |
| [react](skills/react/SKILL.md) | 编写和评审 React 组件，处理 Hooks、Effects、渲染性能、数据请求与 React 19 迁移 |
| [rust](skills/rust/SKILL.md) | 编写和评审 Rust 代码，处理所有权、错误、性能、测试及 Tokio 异步模式 |
| [sql-optimization](skills/sql-optimization/SKILL.md) | 分析 SQL 执行计划，优化查询、索引、分页与批量操作 |
| [security-audit](skills/security-audit/SKILL.md) | 沿数据流与组件交互审查注入、认证授权、密钥泄露、依赖及业务逻辑漏洞 |
| [gdpr-compliant](skills/gdpr-compliant/SKILL.md) | 将 GDPR 隐私工程要求应用到个人数据处理、权限、日志、保留与删除流程 |
| [working-with-mbx](skills/working-with-mbx/SKILL.md) | 安装和排查 Cargo 共享编译缓存 mbx，核对命中、绕过与磁盘使用情况 |
| [working-with-mise](skills/working-with-mise/SKILL.md) | 配置和排查 mise 管理的工具、配置文件、PATH 与激活问题 |

### Agent 协作与实验

| Skill | 说明 |
| --- | --- |
| [agi-mode](skills/agi-mode/SKILL.md) | 以结果契约、主动实验、反证验证和可恢复执行，自主推进复杂任务到可验证交付 |
| [arena](skills/arena/SKILL.md) | agi-mode 的多候选比较兼容入口，选择基础版本并重新验证合成结果 |
| [swarm](skills/swarm/SKILL.md) | 为任务选择多 agent 协作机制，设计角色、分工、执行与收尾流程 |
| [autoresearch](skills/autoresearch/SKILL.md) | agi-mode 的 Hillclimb 兼容入口，在固定协议下搜索并确认改进 |
| [reflect](skills/reflect/SKILL.md) | agi-mode 的 Reflection 兼容入口，从会话证据提炼并验证改进假设 |

自进化流程集中在 `agi-mode` 内：[Eval](skills/agi-mode/playbooks/eval.md) 建立可信判定，[Hillclimb](skills/agi-mode/playbooks/hillclimb.md) 管理搜索，[Arena](skills/agi-mode/playbooks/arena.md) 提供不同候选，[Reflection](skills/agi-mode/playbooks/reflection.md) 提炼教训。协作与记录约定见 [技能演化](skills/agi-mode/references/skill-evolution.md)。

只需安装 `agi-mode` 即可使用上述流程；保留的 `autoresearch`、`arena`、`reflect` 名称是兼容入口，需要同时安装 `agi-mode`。它们不再独立维护执行规则。

整体方法见 [agi-mode 自进化方法论](docs/agi-mode-evolution-methodology.md)，说明样本沉淀、重放、进化实践和能力评估，以及通用方法与业务实践案例的边界。

### 文档、表达与图示

| Skill | 说明 |
| --- | --- |
| [documentation-writer](skills/documentation-writer/SKILL.md) | 基于代码事实编写和更新 README、API、架构、代码库说明与用户手册 |
| [archify](skills/archify/SKILL.md) | 将架构、流程、时序、数据流与状态关系制作成交互式 HTML 图，支持 Mermaid 输入和多格式导出 |
| [elintp](skills/elintp/SKILL.md) | 将技术主题或文档改写成面向非研发读者、带图示的通俗 HTML 说明 |
| [summarize](skills/summarize/SKILL.md) | 将需求、计划、Issue、文档或主题整理为 HTML 报告，并提交到指定 SPMS 项目的报告区 |
| [humanize-writing](skills/humanize-writing/SKILL.md) | 改写机械、夸大或模板化的文字，使表达自然、直接 |
| [meeting-minutes](skills/meeting-minutes/SKILL.md) | 整理会议议题、决定、负责人、截止日期与后续行动，生成会议纪要 |
| [pardon](skills/pardon/SKILL.md) | 补充必要上下文，用当前会话语言把上一条回答重新讲清楚 |

### 设计与转化

| Skill | 说明 |
| --- | --- |
| [kpi-dashboard-design](skills/kpi-dashboard-design/SKILL.md) | 设计 KPI 仪表盘的指标选择、信息层级与数据可视化 |
| [landing-page-conversion-audit](skills/landing-page-conversion-audit/SKILL.md) | 审查落地页、销售页或结账流程的转化障碍，按预期收入影响排序修复建议 |

### XGENT Portal 接入与发布

| Skill | 说明 |
| --- | --- |
| [xgent-init](skills/xgent-init/SKILL.md) | 内部初始化 skill,补齐 `AGENTS.md` / `PRODUCT.md` / `DESIGN.md`,保留 Portal 产品边界与设计规范;`install` 仅直接创建 AGENTS.md |
| [portal-dev-setup](skills/portal-dev-setup/SKILL.md) | 在 App 仓库中启动、体检和排查 Docker 一盒门户，完成本地联调 |
| [portal-external-app](skills/portal-external-app/SKILL.md) | 接入以 Docker 镜像交付的外部服务，核对 manifest、注册、路由、权限与交付契约 |
| [portal-micro-app](skills/portal-micro-app/SKILL.md) | 开发嵌入门户的微应用前端，处理 SDK 握手、导航、授权与 iframe 交互 |
| [portal-app-exchange](skills/portal-app-exchange/SKILL.md) | 配置和排查跨应用 OAuth Token Exchange、交换白名单、授权与 scope |
| [portal-logging](skills/portal-logging/SKILL.md) | 将应用日志与遥测接入门户，配置采集器、写入身份、日志流和字段口径 |
| [xgent-image-push](skills/xgent-image-push/SKILL.md) | 构建并推送 App 镜像到私有 Harbor，预检链路、镜像架构、tag 与保留策略 |
| [xgent-app-release](skills/xgent-app-release/SKILL.md) | 使用发布令牌提交前端产物、镜像和 manifest 变更，处理发布提案与发布故障 |

## 目录结构

每个 skill 位于 `skills/<name>/` 目录下,包含一个带 `name` 与 `description` frontmatter 的 `SKILL.md`:

`description` 用于加载前的技能选择,按「触发条件与关键排除条件 → 核心产出」组织。整条建议不超过 200 字符,中文尽量在 100 字符左右;前 100 字符应能独立表达适用边界。这是本仓库的编辑目标,不是所有 coding agent 的统一截断上限。

容易混淆的限制要紧跟触发条件,例如「仅写/评审计划,不用于按计划实施」,不能放在长段落末尾或只留在正文。操作步骤、完整错误码清单、工具细节和重复的双语说明放入正文或 references;精简后检查前缀是否仍表达原意,不要直接机械截字。

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
