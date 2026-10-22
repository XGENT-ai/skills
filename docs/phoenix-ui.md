# XGENT Phoenix UI：安装、迁移与回滚

Phoenix UI 是 XGENT 自主维护的设计工具，承接 [impeccable](https://github.com/pbakaus/impeccable) skill 4.5.0 / engine 0.1.11 的固定源码（Apache-2.0）。版本、规则、构建和发布都由本仓控制：不查询上游 latest，不从上游或 impeccable.style 下载任何东西。来源、修改标识与三方许可见包内 `dist/phoenix-ui/NOTICE.md`、`THIRD-PARTY-NOTICES.md`。

## 安装

Phoenix 只随 `@xgent-ai/skills` 的 npm 安装器分发：

```bash
# 在目标项目根目录执行
npx @xgent-ai/skills install
```

一次安装分两段，先完成全部只读预检再写项目：

1. **预检**：校验参数、`.claude/settings.json` 与 guard 配置；把包内 `dist/phoenix-ui/VERSION.json` 固定的 engine 下载或从 `--release-dir` 读取，按大小、SHA-256 和 `engine-probe` 身份核对后放进用户 cache；再以 dry-run 计算 Phoenix 的写入和合并。仅由待重装工具目录引起的冲突在清理后重新计算，其余预检失败不写项目。
2. **写入**：按下节的选择删除待替换工具；再写 XGENT 部分（statusline hook、settings、可选 guard 与 AGENTS.md），最后由 Rust 安装执行器在项目锁内重读这些文件，事务化写入 Phoenix 的 skill、agents、hooks、五个辅助 skill 和安装回执 `.phoenix-ui/install/active.json`。清理的旧工具不备份、不参与回滚；清理后被打断时重跑安装。Phoenix 写入中途失败（如磁盘已满）时状态为「待恢复」，可能已部分写入；重跑同一命令会先按日志回滚或补完上次事务再安装。

安装输出逐项列出写入、合并与移除，状态为「已安装 / 未变 / 冲突,未激活 / 待恢复 / 已安装但融合未就绪」之一，并给出事务 ID、备份位置、完整回滚命令和各工具的重载步骤。只有状态为「已安装」或「未变」时才打印「完成」；冲突与融合未就绪以退出码 3 结束，engine 或文件环境失败以 4 结束，参数错误为 1（XGENT 部分）或 2（Phoenix 执行器）。

| 选项 | 作用 |
| --- | --- |
| `--no-phoenix-ui` | 跳过 Phoenix，仍安装 XGENT 部分；不删除已装版本 |
| `--no-impeccable` | 已弃用，等同 `--no-phoenix-ui`，保留一个迁移主版本 |
| `--replace-impeccable` | 删除所选 provider 的项目级及个人级旧 impeccable 工具和 hooks，不备份、不再询问 |
| `--keep-impeccable` | 保留 impeccable，与 Phoenix UI 共存，不再询问；无交互环境默认保留 |
| `--release-dir=DIR` | 离线安装 engine：DIR 是发布者整理的完整 release 目录（`VERSION.json`、各平台 engine、许可与清单），必须与当前包的 manifest 完全一致，零网络 |
| `--providers=a,b` | 指定安装目标目录；`codex` 同时选择 `.agents`（skills）与 `.codex`（hooks）。默认按项目里已有的目录判断，都没有时只装 `.claude` |

**engine**：放在 `~/.phoenix-ui/bin/<版本>/<平台>/<engineSha256>/`（可用 `PHOENIX_UI_HOME` 改 engine cache 的根目录；提醒去重标记也在其中），按引擎摘要隔离，新旧项目固定的引擎可以共存，也不覆盖旧版缓存。hook、context 等普通运行不联网补下载：engine 缺失或版本不符时 hook 放行并每会话提醒一次补装命令，CLI 返回 4 并给出固定包版本的命令：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui engine install --project <项目>
```

终端命令统一通过 `npm exec --package=@xgent-ai/skills -- phoenix-ui <verb>` 调用；对话里的设计工作流用 `/phoenix-ui`（Codex 为 `$phoenix-ui`），二者不要混用。

## 替换或保留已有工具

`npx @xgent-ai/skills install` 发现 impeccable 时询问是否替换。选择是，直接删除所选 provider 的旧 skill、固定 agents/commands 入口和旧 hooks，并关闭相应的 impeccable 插件启用项；项目级与个人级路径均在范围内，不创建工具备份，不留旧命令转跳入口。只移除引用旧 launcher 的 command handler，保留同组的其他 hooks、matcher 和未知配置。选择否则保留这些工具与配置，Phoenix 正常安装，不因旧入口、旧 live 状态或版本不同而阻止激活。

无交互环境默认共存；自动化可用 `--replace-impeccable` 或 `--keep-impeccable` 明确选择。没有 `.phoenix-ui/install/` 事务状态的 Phoenix 副本属于待重装工具，直接删除对应 skill 目录后安装，不要求旧回执，也不备份。已有事务或受管安装仍按回执升级、恢复；早期显式迁移留下的 Phoenix 受管兼容入口也由回执维护，不当作旧工具删除。

这些操作不改业务代码、文档和 `.impeccable/` 数据。个人级旧入口只在选择替换时删除。清理后若安装中断，直接重跑；需要旧工具时从 Git 或其安装渠道恢复。

## 显式迁移旧数据

下面的备份与严格来源核验仅适用于显式 `phoenix-ui migrate --from impeccable`，普通 npm 安装按上一节替换或共存。

安装器只认领本包已发布版本（`@xgent-ai/skills` 0.3.0–0.6.0，对应 skill 4.3.1/4.5.0）写入且未被改动的文件和 hook：

显式执行 `phoenix-ui migrate --from impeccable --project <项目> --bundle-root <bundle目录>` 时检查固定旧状态及源码中的孤立注入和变体标记；完整参数见 `phoenix-ui migrate --help`。

- 未改动的旧 skill、agents、commands 移入 `.phoenix-ui/migrations/<事务ID>/` 备份；旧 hook 分组或嵌套 handler 按完整结构精确匹配后移除，邻近的用户 handler、matcher 与未知字段原样保留。
- 每个旧 skill 位置留一个限期提示入口 `impeccable`，只在被显式调用时列出新命令，不挂 hook、不调用旧工具；下一个主版本移除。
- `.impeccable/` 中可迁移的配置、设计 sidecar 与已核验的数据复制到 `.phoenix-ui/`，未知键原样保留并报告「未迁移」；固定快捷 pin 重新生成为 Phoenix 版本。
- `AGENTS.md`、`CLAUDE.md`、`DESIGN.md`、`PRODUCT.md` 中的旧入口只报告行号与替换建议，不自动改写。
- 个人级目录里早期上游版本的分命令 skill（如 skills.sh 装到 `~/.agents/skills` 的 `polish`、`teach-impeccable`）没有 hook 或 engine，只报告位置与移除、重载步骤，不阻止激活。

显式迁移时，以下情况会阻止激活（退出码 3，项目不被修改），按输出逐项处理后重跑。普通安装仍保留外部辅助 skill 的融合检查：

| 情况 | 处理 |
| --- | --- |
| 旧 skill 或旧 hook 被用户改过、被 shell 包装，或结构无法唯一确定 | 输出列出文件与条目；先备份/还原或移出活跃目录，再重跑 |
| 固定旧状态路径或注入日志中记录的 live、注入或变体残留；显式迁移还检查源码中的孤立标记 | 用旧项目的 launcher 正常停止、`live-inject --remove`、接受或丢弃变体；不要按 PID 杀进程 |
| 个人级（如 `~/.claude/skills/impeccable`）旧入口、旧 hook 或插件 | 安装器只读发现、不改个人配置；按输出在对应工具中禁用或移除后重载并重跑 |
| 外部管理的辅助 skill 副本与本包不同（含 skills.sh 副本、符号链接） | Phoenix 仍激活，状态为「已安装但融合未就绪」；按输出用 `npx skills add XGENT-ai/skills --skill <名称>` 更新，或备份后移出活跃目录再重跑 |

## 回滚与恢复

安装输出中的回滚命令只撤销该事务写入的条目：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui migrate --rollback <事务ID> --project <项目>
```

- 独占文件按迁移后的摘要核对后还原；共享 JSON 只逆转本次加入或移除的条目，后来新增的 allow、guard 阈值和未知字段保留。受管条目被改过时返回 3 并列出冲突，不强制回退。
- 回滚不撤销在 Phoenix 中做的产品代码修改，也不把新数据格式塞回旧工具；`.phoenix-ui/` 中的备份、日志与新数据保留，确认稳定后可手动删除。
- 普通安装直接删除的 impeccable 工具和未受管 Phoenix 副本不在回滚范围内，不会从安装日志恢复。
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

Phoenix 的 hook 按完整分组写入；同一事件里已有其他工具（含旧版 goal guard）的分组（包括都没有 matcher 的分组）互不影响，重复安装不产生重复 handler。安装器测试覆盖各目录的文件与 hook 合并；工具真实加载以各自会话为准，同名 skill 的优先级按工具规则（Claude Code 个人级优先于项目级），安装器无法确定时会提示待人工核对。

## 渠道

- npm：`@xgent-ai/skills` 包含 Phoenix 运行资源（skill bundle、`VERSION.json`、许可），engine 二进制按 manifest 从固定地址获取或离线目录提供，不进 npm 包。
- skills.sh：五个辅助 skill（design-md、design-reference、ui-pattern-research、review-ui、apply-ui-review）继续公开，可单独安装；`phoenix-ui` skill 标为 internal，只通过 npm 安装器分发。没有 Phoenix CLI 时，辅助 skill 按各自的人工流程工作。
- 原 `vendor/impeccable/` 副本及其 vendor/R2 脚本已删除，需要时从 git 历史查看；旧安装的迁移依据是 `tools/phoenix-ui/migration-sources/` 中按 npm 包核对过的固定记录。

## 融合用法与 UI 验收

本节适用于支持结构化 context 与 managed 评审协议的匹配版本；CLI 操作成功不代表 UI 通过，候选验收与公开发布见后面的维护流程。下列 `<包版本>` 从实际回执/候选包读取；未发布候选先用维护者提供的 tarball 与匹配离线资源，不把 latest 当作该候选。

Phoenix UI 适用于独立网站、应用和嵌入式界面等 UI 项目。`phoenix-ui init` 直接依据目标项目的事实和已确认信息生成或补齐 PRODUCT.md，不依赖 `xgent-init`；产品边界、平台和设计约束均来自项目本身，不预设宿主平台或接入方式。subagent 与并行方式由 coding agent 结合自身能力、策略和用户约束选择，Phoenix 不另设授权询问或等待步骤；critique 的单上下文执行仍须先完成 A 再运行 B，并披露降级原因。

视觉参考用 `design-reference`，交互选型用 `ui-pattern-research`，维护项目规范用 `design-md`，已选方向的实现继续用 `phoenix-ui`。先读取明确目标的上下文：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui context --target "route:/orders" --json
```

核对 `projectRoot`、`target.path`、原始 `body` 与 `provenance`；`selection_required` 时确认候选，不自动取第一个。`parseCoverage` 是有限解析范围，正文和未支持的 YAML 仍按原文核实，空 `conflicts` 不证明正文无冲突。profile/defaultAdvice 是建议，不能替代项目规范、补未知 token 或回套冻结目标。

用户明确接受方向后，登记原 brief 和真实接受引用，再回读 context。仅 `surfaceBrief.status: accepted`、`usable: true` 且目标适用时沿用方向；`present` 是提案，`stale`、有接受记录的 `missing` 或 `ambiguous` 要核对原文与决定，不自动重抽方向：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui surface-brief link "route:/orders" "docs/design/orders-brief.md" --decision "<实际接受决定引用>"
```

`link` 保存原文件路径、SHA-256 与接受记录，原 brief 留在选定仓库，不复制第二份可编辑正文。URL/route 是逻辑目标，`target.exists: null` 不表示文件缺失。设计流程详见包内 `design-reference`、`ui-pattern-research`、`design-md` 的实际指令。

用 `review-ui` 评审、`apply-ui-review` 按报告修复，再由新的独立评审确认；修复方最多登记「已修改待评审」。先按固定包回执核对 `engine-probe` 身份，再查实际 `review` 帮助、输入形状和本循环状态，只有无旧历史的新循环才 begin：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui review --help
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui review schema --json
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui review status --prefix "docs/ui/orders" --json
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui review begin --prefix "docs/ui/orders" --target "docs/ui/orders.ui-target.md" --tested-version "docs/ui/orders.tested-version.json" --json
```

`review schema --json` 返回 `reviewSchema: 1`、`discoveryVersion: 1` 的完整 typed examples 与枚举变体，可用 `--kind tested-version` 等选择一种输入。它描述当前 serde 字段形状，不是完整 JSON Schema；样例的文字、数字和布尔值均为占位，不证明执行成功或当前 UI 事实。真实身份、路径、hash、版本和 revision 取实际操作与取证。帮助和 schema 不读写循环；新前缀的父目录不存在时，`status` 可直接返回 `absent`，不创建目录或循环。

`--tested-version` 的 JSON 由宿主提供真实 HEAD 加 clean/dirty 指纹，或可核对的 build ID，并与本轮应用/采集一致；省略时为 Unknown，不能判通过。context 提供目标和规范来源，不自动生产版本记录。后续 capture、judge-pack、judge add、finalize、apply record 使用返回的 round/current revision 和实际合同输入；**退出 0 不等于 UI 通过**，必须回读 verdict 与报告，`confirmation_required` 仍为草稿。参数与输入边界见 [Phoenix managed 评审协议](../skills/review-ui/references/phoenix-review.md)，目标、报告和处置沿 [review-ui 合同](../skills/review-ui/references/report-contract.md) 与 [修复记录合同](../skills/apply-ui-review/references/record-contract.md)。

已有 Markdown 历史无 managed 索引时继续 manual，只有显式 `review import --prefix <前缀> --input <映射.json> --json` 才接管；未知历史不能填零。`legacyMarkdown` 无法确定历史最佳时返回 3 并保持人工循环。managed 的 engine 缺失/不匹配、损坏或未知 schema 时只读历史，可收集未登记素材，不计分、裁定或修复产品代码；恢复匹配版本后续接原 round/revision，不删索引或换前缀绕过。生成区被手改时先另存原文与 hash，用 `projectionRebase` 显式核对副本、当前 hash 和 cycle revision 后恢复权威区块；要保留的采集/处置再以当前 revision 提交。封存报告与评审原文不覆盖。

## 维护者：统一候选与本地交接

A 阶段只内部验收，不单独发布。M6 先在专用统一候选树整合 A、M4/M5、发布工具与同步元数据，再从同一 HEAD/tree/构建输入生成最终 binary 和 bundle；隔离模块的旧制品或相近 SHA 不替代它。维护工具按 mise 与 build-tools.lock.json 固定，Node 18 是消费底线；工具缺失/版本不符在 prepare 阶段处理，不在 build/test 中隐式补装：

```bash
cd tools/phoenix-ui
mise exec --no-deps -- node ../../scripts/prepare-phoenix-ui.mjs --prepare
# 按 environmentFile 回执，在当前 shell 导出其中的 WASM_BINDGEN、
# WASM_OPT、PHOENIX_CARGO_DENY；Linux ARM64 还需 PHOENIX_UI_BROWSER。
mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --write
mise exec --no-deps -- npm --prefix ../.. test
mise exec --no-deps -- node ../../scripts/build-phoenix-ui.mjs --development --output-dir ../../local/phoenix-ui/development-release
mise exec --no-deps -- node ../../scripts/smoke-phoenix-ui.mjs --release-dir ../../local/phoenix-ui/development-release --target <本机平台> --stage-runtime
mise exec --no-deps -- npm --prefix ../.. run test:phoenix
```

prepare 显式联网准备锁定依赖、工具、浏览器、迁移包和公告快照；随后 audit/build/runtime 使用已准备缓存与离线合同，`CARGO_NET_OFFLINE` 不单独证明全部网络已禁止。`--development` 仅是本机开发制品。`check-phoenix-deps.mjs --distribution-dir <目录>` 另核对分发许可，真实 browser/provider 会话仍分别取证。完整维护入口见 [prepare](../scripts/prepare-phoenix-ui.mjs)、[build](../scripts/build-phoenix-ui.mjs)、[audit](../scripts/check-phoenix-deps.mjs)、[runtime tests](../scripts/test-phoenix-ui.mjs)。

[CI workflow](../.github/workflows/phoenix-ui.yml) 在 macOS/Linux 四个平台同架构冷构建/运行，不恢复编译缓存，另有 Node18 consumer。固定成功 run 的四份 native、四份 phase evidence、一份 Node18 evidence 必须保留原始 ZIP/digest、HEAD/tree、工具链、输入/制品摘要、许可及全部真实 step/phase 结果。CI 不持 R2/npm 凭据；缺平台、失败 phase 或摘要不符不能交接。

发布只用一条命令，在**干净、已推送且 Phoenix CI 成功**的 HEAD 上执行。版本号先在 `package.json` 改好，并用上面的开发构建同步 `VERSION.json`/`ENGINE.json`/`GENERATED.json`：

```bash
mise exec --no-deps -- npm run release:phoenix                      # 上传 R2 与 npm 发布前各确认一次，npm 会要 OTP
mise exec --no-deps -- npm run release:phoenix -- --prepare-only     # 只做到安装检查；再跑一次不带参数即发布
```

[release-phoenix-ui.mjs](../scripts/release-phoenix-ui.mjs) 依次执行，任一步失败即停：

1. 核对工作树干净、npm 上还没有这个版本，找到该提交成功的 Phoenix CI run（也可用 `--run-id` 指定）。
2. 下载 CI 的九个原始 ZIP，逐个核对 GitHub digest。
3. 用 [publish-phoenix-ui-r2.mjs](../scripts/publish-phoenix-ui-r2.mjs) 条件上传四个 engine（已存在即拒绝覆盖），公共 URL 回读通过后冻结 release manifest。凭据取已导出的 `AGENT_RELEASE_R2_*`，没有时读仓根 `.env.cf` 里的同名行。
4. 用 [pack-phoenix-ui-release.mjs](../scripts/pack-phoenix-ui-release.mjs) 只打一次 tarball。
5. 用 [check-phoenix-ui-release.mjs](../scripts/check-phoenix-ui-release.mjs) 在 Node 18 和维护 Node 上新装（engine 从公共 URL 下载），检查 0.3.0/0.6.0 工具保留共存、直接删除替换、无回执 Phoenix 副本重装与受管安装回滚。删除的旧工具不会回滚恢复。
6. `npm publish` 这个 tarball，再从 registry 下载回来核对 shasum、integrity 和 SHA-256。

每步产物保存在 `local/phoenix-ui/releases/v<版本>-<短SHA>/`，中断后重跑会复核已有产物再续接。预发布版本必须加 `--tag`。CI 测过的提交之后 main 又有新提交时，用 `git worktree add <目录> <该SHA>` 在该提交上执行，产物留在那个 worktree 的 `local/` 下。

在仓库根目录直接 `npm publish` 会被 `prepublishOnly` 拒绝：检出里的 `dist/phoenix-ui/VERSION.json` 是开发清单，没有 engine 下载地址。发布过的 R2 对象和 npm 版本都不能覆盖，出错只能发新版本。

M6 终验在日常发布检查之外，还用同一 tarball 做过 A→B 升级/回滚与真实融合验收（见计划 M6 记录）；替身、开发包和四平台 CI 都不能单独证明线上 URL 或最终包可用。
