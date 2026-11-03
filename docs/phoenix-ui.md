# XGENT Phoenix UI：安装与使用

Phoenix UI 是 XGENT 自主维护的设计工具，承接 [impeccable](https://github.com/pbakaus/impeccable) skill 4.5.0 / engine 0.1.11 的固定源码（Apache-2.0）。版本、规则、构建和发布都由本仓控制：不查询上游 latest，不从上游或 impeccable.style 下载任何东西。来源、修改标识与三方许可见包内 `dist/phoenix-ui/NOTICE.md`、`THIRD-PARTY-NOTICES.md`。

## 安装

Phoenix 只随 `@xgent-ai/skills` 的 npm 安装器分发：

```bash
# 在目标项目根目录执行
npx @xgent-ai/skills install
```

一次安装分两段，先完成全部只读预检再写项目：

1. **预检**：校验参数、`.claude/settings.json` 与 guard 配置；把包内 `dist/phoenix-ui/VERSION.json` 固定的 engine 下载或从 `--release-dir` 读取，按大小、SHA-256 和 `engine-probe` 身份核对后放进用户 cache；再以 dry-run 计算 Phoenix 的写入和合并；共享 hook 配置不是合法 JSON 对象等预检失败不写项目。
2. **写入**：按下节的选择删除待替换工具，写入 XGENT 部分，随后直接覆盖 Phoenix UI 的 skill、agents 和启动文件；符号链接的 `skills/phoenix-ui` 目录换成副本，不改链接目标。共享配置只替换 Phoenix hook，保留其他 handler 和配置；重写后为标准 JSON，原有注释不保留。安装不创建回执、日志或备份；完成后删除早期版本留下的 `.phoenix-ui/install/` 和 `.phoenix-ui/migrations/`，保留配置和业务数据。中断后重跑同一命令覆盖安装。

安装输出列出写入、合并与移除及重载步骤；相同内容重复安装显示「未变」。预检失败不写项目，写入失败显示未完成并提示处理错误后重跑。

| 选项 | 作用 |
| --- | --- |
| `--no-phoenix-ui` | 跳过 Phoenix，仍安装 XGENT 部分；不删除已装版本 |
| `--no-impeccable` | 已弃用，等同 `--no-phoenix-ui` |
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

无交互环境默认共存；自动化可用 `--replace-impeccable` 或 `--keep-impeccable` 明确选择。Phoenix UI 文件直接按当前包覆盖，包括 skills.sh 或手动复制修改过的文件。运行只检查 ENGINE 和引擎。独立辅助 skill 由各自安装渠道管理，现有文件保留。

这些操作不改业务代码、文档和 `.impeccable/` 数据，也不迁移旧数据。个人级旧入口只在选择替换时删除。清理后若安装中断，直接重跑；需要旧工具时从 Git 或其安装渠道恢复。

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
- skills.sh：五个辅助 skill（design-md、design-reference、ui-pattern-research、review-ui、apply-ui-review）仅由 `npx skills add XGENT-ai/skills` 独立安装和更新，npm 安装器不写入、不认领、不比对它们的版本；`phoenix-ui` skill 用 `metadata.internal: true` 排除在默认发现列表之外，只通过 npm 安装器分发。已由 npm 安装的 Phoenix 主技能应通过 npm 更新，不再用 skills.sh 覆盖。没有 Phoenix CLI 时，辅助 skill 按各自的人工流程工作。
- 原 `vendor/impeccable/` 副本及其 vendor/R2 脚本已删除，需要时从 git 历史查看；发布检查重建旧版项目所用的 0.3.0/0.6.0 包摘要记录在 `tools/phoenix-ui/migration-sources/`。

## XGENT 自有检测规则

Phoenix UI 在承接上游的 61 条内置规则之外，另带一组 XGENT 自有规则（ID 以 `xgent/` 开头）。它们默认启用、与内置规则分开定义，可以按规则单独关闭。规则包只装进原生 `phoenix-ui` 二进制：`detect`（源码文件与 URL 目标）和设计 hook 会运行它；live 叠层与浏览器扩展在页面内使用 WASM 检测、不加载这个规则包，所以同一页面在叠层或扩展里看不到下面这些发现，结果可能与 `detect <url>` 不同；需要这部分发现时以 `detect` 的结果为准。

| 规则 ID | 检测方式 | 级别 | 关闭方式 |
| --- | --- | --- | --- |
| `xgent/transition-all` | 源码文件（文本扫描） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、inline 豁免、`--no-advisory` |
| `xgent/focus-outline-removed` | 源码文件（文本扫描） | warning | `detector.ignoreRules`、`hooks ignore-rule`、inline 豁免 |
| `xgent/wrapped-control-label` | 页面（`detect <url>`，浏览器渲染） | warning | `detector.ignoreRules`、`hooks ignore-rule` |
| `xgent/control-height-mismatch` | 页面（`detect <url>`，浏览器渲染） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、`--no-advisory` |
| `xgent/unlabeled-inline-svg` | 页面（`detect <url>`，浏览器渲染） | warning | `detector.ignoreRules`、`hooks ignore-rule` |
| `xgent/redrawn-window-chrome` | 页面（`detect <url>`，浏览器渲染） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、`--no-advisory` |
| `xgent/emoji-icon` | 页面（`detect <url>`，浏览器渲染） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、`--no-advisory` |
| `xgent/font-family-count` | 页面（`detect <url>`，浏览器渲染） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、`--no-advisory` |

「源码文件」指文本扫描，覆盖 CSS、HTML、Vue、JSX、TSX 等前端源文件；「页面」指把 http(s):// 或 file:// URL 交给 `detect`，由真实浏览器渲染后检查，可用 `--viewport <W>x<H>` 指定视口（默认 1280x800）。warning 级发现计入失败数，可能让退出码为 2；advisory 单独列出、不改变退出码，`--no-advisory` 只能隐藏 advisory 级发现。两条文本规则也随设计 hook 在编辑后运行：`xgent/focus-outline-removed` 会在 Stop 深检提醒中给出，并在 Cursor before-edit 中拒绝写入；`xgent/transition-all` 是 advisory，hook 默认不输出 advisory 发现。页面规则依赖浏览器渲染，hook 不会产生它们，需要时单独运行 `detect <url>`。

关闭方式（按影响范围选择）：

- `detector.ignoreRules`：项目级关闭，手动 `detect` 与 hook 共用这份过滤；配置在 `.phoenix-ui/config.json`，本地覆盖在 `.phoenix-ui/config.local.json`。不要手改这两个文件，走下面的 hooks 命令。
- `hooks ignore-rule <id>`：把规则 ID 追加到 `detector.ignoreRules` 的受支持写法，影响整个项目；范围较大，先征得用户同意。只关一个文件中的一条时用 `phoenix-ui hooks ignore-value <id> "*" --file <path>`，窄例外默认写入项目共享 config。
- inline 豁免：`impeccable-disable*`（引擎原有）与 `phoenix-ui-disable*`（skill 文案）两种前缀都生效；词形为 `disable`（整个文件）、`disable-line`、`disable-next-line`（单行），后接一个或多个规则 ID（逗号分隔）或 `*`，可带 `: 理由` 或 `-- 理由`。内联标记按源文件内容生效；URL 扫描读取渲染后的页面，页面规则的例外用项目级忽略。`--no-inline-ignores`（或 `--no-config`）可绕过内联豁免。
- `detect --no-advisory`：只在单次扫描中隐藏 advisory 级发现，warning 级发现与退出码不受影响。

在 Cursor 上启用新版本后，本来含无替代 `focus:outline-none` 的文件会对任何编辑触发 before-edit 拒绝：该检查扫描整份投影后的文件，不扣除编辑前已经存在的发现，所以与该焦点样式无关的编辑也会被拒。处理方式是先修掉该文件的焦点样式；确需临时放行时用 `phoenix-ui hooks ignore-value xgent/focus-outline-removed "*" --file <path>` 只关这个文件中的这一条。不要为绕过拒绝改用 inline 豁免。例外默认通过 `ignore-value` 写入项目 config；`ignore-file`、`ignore-rule` 影响面更大，应先征得用户同意。

## 融合用法与 UI 验收

本节适用于支持结构化 context 的匹配版本；CLI 操作成功不代表 UI 通过，候选验收与公开发布见后面的维护流程。下列 `<包版本>` 取实际安装的包版本（`ENGINE.json` 的 `npmPackageVersion`）或候选包；未发布候选先用维护者提供的 tarball 与匹配离线资源，不把 latest 当作该候选。

Phoenix UI 适用于独立网站、应用和嵌入式界面等 UI 项目。`phoenix-ui init` 直接依据目标项目的事实和已确认信息生成或补齐 PRODUCT.md，不依赖其他初始化 skill；产品边界、平台和设计约束均来自项目本身，不预设宿主平台或接入方式。subagent 与并行方式由 coding agent 结合自身能力、策略和用户约束选择，Phoenix 不另设授权询问或等待步骤；critique 的单上下文执行仍须先完成 A 再运行 B，并披露降级原因。

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

用 `review-ui` 评审、`apply-ui-review` 按报告修复，再由新的独立评审确认；修复方最多登记「已修改待评审」。这两个 skill 不依赖 Phoenix，评审记录按 [review-ui 合同](../skills/review-ui/references/report-contract.md) 与 [修复记录合同](../skills/apply-ui-review/references/record-contract.md) 以 Markdown 维护。

## 维护者：统一候选与本地交接

最终 binary 和 bundle 从同一 HEAD/tree/构建输入生成；旧制品或相近 SHA 不替代它。维护工具按 mise 与 build-tools.lock.json 固定，Node 18 是消费底线；工具缺失/版本不符在 prepare 阶段处理，不在 build/test 中隐式补装：

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

prepare 显式联网准备锁定依赖、工具、浏览器和公告快照；随后 audit/build/runtime 使用已准备缓存与离线合同，`CARGO_NET_OFFLINE` 不单独证明全部网络已禁止。`--development` 仅是本机开发制品。`check-phoenix-deps.mjs --distribution-dir <目录>` 另核对分发许可，真实 browser/provider 会话仍分别取证。完整维护入口见 [prepare](../scripts/prepare-phoenix-ui.mjs)、[build](../scripts/build-phoenix-ui.mjs)、[audit](../scripts/check-phoenix-deps.mjs)、[runtime tests](../scripts/test-phoenix-ui.mjs)。

统一的 [Phoenix CI](../.github/workflows/phoenix.yml) 在同一次运行中检查 UI 和 Diagram，并给出共同结果。UI 与 Diagram 在 macOS/Linux 四个平台同架构冷构建/运行，不恢复编译缓存，UI 另有 Node18 consumer。每个平台上传 `phoenix-ui-<平台>` 与 `phoenix-diagram-<平台>` 制品，发布只取这八份引擎制品；整次 run 成功即视为全部 job 通过。CI 不持 R2/npm 凭据。

发布只用一条命令，在 **main 分支上干净、已推送且 Phoenix CI 成功**的 HEAD 上执行，不创建独立发布分支，也不从 detached HEAD 发布。版本号先在 `package.json` 改好，并用上面的开发构建同步 `VERSION.json`/`ENGINE.json`/`GENERATED.json`：

```bash
mise exec --no-deps -- npm run release:phoenix                      # 上传 R2 与 npm 发布前各确认一次，npm 会要 OTP
mise exec --no-deps -- npm run release:phoenix -- --prepare-only     # 只做到安装检查；再按脚本提示的命令（带 --run-id）运行即发布
```

[release-phoenix-ui.mjs](../scripts/release-phoenix-ui.mjs) 依次执行，任一步失败即停：

1. 核对当前分支为 `main`、工作树干净、npm 上还没有这个版本，找到该提交最新一次成功的 Phoenix CI run（也可用 `--run-id` 指定）。
2. 从这次 run 下载 UI 与 Diagram 各四个平台的 engine 制品（`phoenix-ui-<平台>`、`phoenix-diagram-<平台>`），逐个核对 GitHub digest，并确认 CI 构建所用的两个 bundle 与当前提交一致、提交的生成文件与源码一致。
3. 用 `aws s3api put-object --if-none-match '*'` 把八个 engine 上传到 R2 的 `<前缀>/<工具>/v<版本>/ci-<run>/<平台>/<文件>`，从不覆盖：对象已存在时只接受字节相同的；随后从每个公共 URL 回读，核对大小与 SHA-256。凭据取已导出的 `AGENT_RELEASE_R2_*`，没有时读仓根 `.env.cf` 里的同名行。已上传的对象有误时，为同一提交手动再触发一次 Phoenix CI（得到新 run ID；Actions 页面的 Re-run 沿用原 ID）后重新发布，不会撞上旧对象。
4. 从 HEAD（`git archive`）只打一次联合 tarball：写入两个工具的 release 清单及其副本（UI 的 `skills/phoenix-ui/scripts/ENGINE.json` 与 `GENERATED.json` 中的摘要，Diagram 的 `bundle/ENGINE.json` 与 `skills/phoenix-diagram/ENGINE.json`），第三方许可取自 darwin-arm64 的 CI 制品，最终包再次通过发布校验。`xgent-skills install` 默认安装两者。
5. 用 [check-phoenix-ui-release.mjs](../scripts/check-phoenix-ui-release.mjs) 在 Node 18 和维护 Node 上联合新装（两个 engine 从公共 URL 下载），核对 Diagram 引擎摘要、可运行入口及重复安装；另检查 0.3.0/0.6.0 工具保留共存、直接删除替换、Phoenix 副本覆盖重装、旧安装目录清理和重复安装。
6. `npm publish` 这个 tarball，再从 registry 下载回来核对 shasum、integrity 和 SHA-256。

只改脚本、skill 或文档、引擎源码未变时，可用 `--reuse-engines <已发布版本>` 跳过 CI 和 R2（第 2、3 步），直接复用该 npm 版本的 UI 与 Diagram 引擎：

```bash
mise exec --no-deps -- npm run release:phoenix -- --reuse-engines 0.7.10 [--prepare-only]
```

脚本从 registry 下载该版本并核对 integrity，然后检查：自那次发布以来，编译进 UI 引擎的源码（`tools/phoenix-ui` 中除 `scripts/render.mjs`、`scripts/lib` 外的部分，以及 Rust `include_str!`/`include_bytes!` 引用的仓库文件）在 git 中没有改动；Diagram 的源码输入摘要与旧清单的 `sourceInputSha256` 一致；提交的 `GENERATED.json` 与 Diagram bundle 和当前源码一致。任一项不满足就停止，需要改为走 CI 构建。通过后按第 4 步从 HEAD 打包，只是引擎相关字段和第三方许可来自旧版本，`npmPackageVersion`、`sourceCommit` 和 bundle 摘要仍取当前提交。之后照常执行第 5、6 步。引擎只认 `toolVersion` 与清单内容，不绑定 npm 版本；能复用的最早版本是 Diagram 去掉版本绑定后首次构建发布的版本（0.7.9 及以前不行）。

每步产物保存在 `local/phoenix-ui/releases/v<版本>-<短SHA>/ci-<run>/`（复用引擎时为 `reuse-<版本>/`），中断后重跑同一命令即可续接：已打包就直接从安装检查继续，否则重新核对 CI 制品，已上传的对象只做字节比对。预发布版本必须加 `--tag`。CI 测过的提交之后 main 又有新提交时，等待 main 当前 HEAD 的 CI 成功后再发布；不另建发布分支或检出旧提交发布。

在仓库根目录直接 `npm publish` 会被 `prepublishOnly` 拒绝：检出里的 `dist/phoenix-ui/VERSION.json` 是开发清单，没有 engine 下载地址。发布过的 R2 对象和 npm 版本都不能覆盖，出错只能发新版本。

替身、开发包和四平台 CI 都不能单独证明线上 URL 或最终包可用。
