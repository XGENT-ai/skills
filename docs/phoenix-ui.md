# XGENT Phoenix UI：安装与使用

Phoenix UI 是 XGENT 自主维护的设计工具，版本、规则、构建和发布都由本仓控制。三方许可见包内 `dist/phoenix-ui/NOTICE.md`、`THIRD-PARTY-NOTICES.md`。

## 安装

Phoenix 只随 `@xgent-ai/skills` 的 npm 安装器分发：

```bash
# 在目标项目根目录执行
npx @xgent-ai/skills install
```

一次安装分两段，先完成全部只读预检再写项目：

1. **预检**：校验参数、`.claude/settings.json` 与 guard 配置；把包内 `dist/phoenix-ui/VERSION.json` 固定的 engine 下载或从 `--release-dir` 读取，按大小、SHA-256 和 `engine-probe` 身份核对后放进用户 cache；再以 dry-run 计算 Phoenix 的写入和合并；共享 hook 配置不是合法 JSON 对象等预检失败不写项目。
2. **写入**：按下节的选择删除待替换工具，写入 XGENT 部分，随后直接覆盖 Phoenix UI 的 skill、agents 和启动文件；符号链接的 `skills/phoenix-ui` 目录换成副本，不改链接目标。共享配置只替换 Phoenix hook，保留其他 handler 和配置；重写后为标准 JSON，原有注释不保留。安装不创建回执、日志或备份，保留配置和业务数据。中断后重跑同一命令覆盖安装。

安装输出列出写入、合并与移除及重载步骤；相同内容重复安装显示「未变」。预检失败不写项目，写入失败显示未完成并提示处理错误后重跑。

| 选项 | 作用 |
| --- | --- |
| `--no-phoenix-ui` | 跳过 Phoenix，仍安装 XGENT 部分；不删除已装版本，也不处理 impeccable |
| `--keep-impeccable` | 保留旧 impeccable，与 Phoenix UI 共存，不再询问 |
| `--release-dir=DIR` | 离线安装 engine：DIR 是发布者整理的完整 release 目录（`VERSION.json`、各平台 engine、许可与清单），必须与当前包的 manifest 完全一致，零网络 |
| `--providers=a,b` | 指定安装目标目录；`codex` 同时选择 `.agents`（skills）与 `.codex`（hooks）。默认按项目里已有的目录判断，都没有时只装 `.claude` |

**engine**：放在 `~/.phoenix-ui/bin/<版本>/<平台>/<engineSha256>/`（可用 `PHOENIX_UI_HOME` 改 engine cache 的根目录；提醒去重标记也在其中），按引擎摘要隔离，新旧项目固定的引擎可以共存，也不覆盖旧版缓存。hook、context 等普通运行不联网补下载：engine 缺失或版本不符时 hook 放行并每会话提醒一次补装命令，CLI 返回 4 并给出固定包版本的命令：

```bash
npm exec --package=@xgent-ai/skills@<包版本> -- phoenix-ui engine install --project <项目>
```

终端命令统一通过 `npm exec --package=@xgent-ai/skills -- phoenix-ui <verb>` 调用；对话里的设计工作流用 `/phoenix-ui`（Codex 为 `$phoenix-ui`），二者不要混用。

## 删除或保留 impeccable

`npx @xgent-ai/skills install` 发现 impeccable 时默认删除：交互终端以 `[Y/n]` 确认（回车即删除），无交互环境直接删除，`--keep-impeccable` 保留。删除范围是所选 provider 的项目级与个人级 skill、固定 agents/commands 入口、旧 hooks、`skills-lock.json` 与 `.agents/.skill-lock.json` 中的 impeccable 记录，并关闭相应的 impeccable 插件启用项；不创建工具备份，不留旧命令转跳入口。只移除引用旧 launcher 的 command handler，保留同组的其他 hooks、matcher 和未知配置。保留时 Phoenix 照常安装，与 impeccable 共存。

Phoenix UI 文件直接按当前包覆盖，包括 skills.sh 或手动复制修改过的文件。运行只检查 ENGINE 和引擎。独立辅助 skill 由各自安装渠道管理，现有文件保留。

这些操作不改业务代码、文档和 impeccable 已生成的数据，不迁移旧数据。清理后若安装中断，直接重跑；需要旧工具时从其安装渠道重新安装。

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
- skills.sh：五个辅助 skill（design-md、design-reference、ui-pattern-research、review-ui、apply-ui-review）仅由 `npx skills add XGENT-ai/skills` 独立安装和更新，npm 安装器不写入、不认领、不比对它们的版本；`phoenix-ui` skill 用 `metadata.internal: true` 排除在默认发现列表之外，只通过 npm 安装器分发。已由 npm 安装的 Phoenix 主技能应通过 npm 更新，不再用 skills.sh 覆盖。这些 skill 不依赖 Phoenix，没有 Phoenix CLI 时按各自的流程工作；检测到 Phoenix 时，review-ui 可借用取证命令与盲测走查，feature-demo 可在录制前用取证命令核对。

## XGENT 自有检测规则

Phoenix UI 在 61 条内置规则之外，另带一组 XGENT 自有规则（ID 以 `xgent/` 开头）。它们默认启用、与内置规则分开定义，可以按规则单独关闭。规则包只装进原生 `phoenix-ui` 二进制：`detect`（源码文件与 URL 目标）和设计 hook 会运行它；live 叠层与浏览器扩展在页面内使用 WASM 检测、不加载这个规则包，所以同一页面在叠层或扩展里看不到下面这些发现，结果可能与 `detect <url>` 不同；需要这部分发现时以 `detect` 的结果为准。

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
| `xgent/cjk-tight-leading` | 页面（`detect <url>`，浏览器渲染） | warning | `detector.ignoreRules`、`hooks ignore-rule` |
| `xgent/cjk-line-length` | 页面（`detect <url>`，浏览器渲染） | warning | `detector.ignoreRules`、`hooks ignore-rule` |
| `xgent/cjk-orphan` | 页面（`detect <url>`，浏览器渲染） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、`--no-advisory` |
| `xgent/cjk-split-unit` | 页面（`detect <url>`，浏览器渲染） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、`--no-advisory` |
| `xgent/cjk-font-fallback` | 页面（`detect <url>`，浏览器渲染） | advisory | `detector.ignoreRules`、`hooks ignore-rule`、`--no-advisory` |

「源码文件」指文本扫描，覆盖 CSS、HTML、Vue、JSX、TSX 等前端源文件；「页面」指把 http(s):// 或 file:// URL 交给 `detect`，由真实浏览器渲染后检查，可用 `--viewport <W>x<H>` 指定视口（默认 1280x800）。warning 级发现计入失败数，可能让退出码为 2；advisory 单独列出、不改变退出码，`--no-advisory` 只能隐藏 advisory 级发现。两条文本规则也随设计 hook 在编辑后运行：`xgent/focus-outline-removed` 会在 Stop 深检提醒中给出，并在 Cursor before-edit 中拒绝写入；`xgent/transition-all` 是 advisory，hook 默认不输出 advisory 发现。页面规则依赖浏览器渲染，hook 不会产生它们，需要时单独运行 `detect <url>`。

五条 `xgent/cjk-*` 规则只作用于以汉字为主的文本：去掉空白与标点后，汉字超过可见字符的一半。其中正文指 `p`、`li`、`td`、`th`、`dd`、`blockquote`、`figcaption` 里超过 10 个字符的直接文本，标题指 `h1`–`h6`；日文、韩文和夹少量中文的拉丁文本不在其内，标识符或路径密集、汉字不过半的中文技术段落也不在其内，仍按内置规则判定。

- `xgent/cjk-tight-leading`：多行中文正文行高低于字号的 1.5 倍（`line-height: normal` 不判定）。
- `xgent/cjk-line-length`：中文正文至少两行超过 40 个汉字宽（每行渲染宽度除以字号与字距之和）。
- `xgent/cjk-orphan`：正文或标题的最后一行只剩一两个汉字。
- `xgent/cjk-split-unit`：数字和紧跟的单位（如“38 秒”“3 个工作日”“99.5%”）被拆到两行。
- `xgent/cjk-font-fallback`：汉字被意外的字体渲染，只报三种情况：声明的 Web 字体加载失败、汉字因此改由后面的字体渲染；同一段汉字由两个以上字体渲染，且其中有声明的具名字体；声明的字体都没有渲染汉字，列表里也没有通用字体族。只声明通用字体族、具名字体渲染了全部汉字、拉丁字体在前而汉字落到后面的字体，都是正常结果。同一字体栈的同一种情况只报在第一处，并写出其余处数。发现里的字体名是系统报告的名称，可能是本地化名（如苹方在部分系统语言下报为“蘋方-簡”）；这时只写了具名字体、没写通用字体族的声明会被报成第三种情况，补一个通用字体族即可。手工 `<br>` 造成的孤字与数字单位分行同样会报。

在这些中文正文上，内置的 `line-length`、`tight-leading` 由对应的中文规则接替，不再出现；关闭某条中文规则不会让内置规则回来。标题与其他标签上的文本仍由内置规则判定。升级后中文页面的 `detect <url>` 可能因此新增两类 warning 并使退出码变为 2，不接受时用 `hooks ignore-rule` 逐条关闭。hook 不产生这五条规则；live 叠层与浏览器扩展不运行它们，中文正文在那两处仍只有内置规则的结果。

字体来源无法核实时（字体仍在加载、浏览器没有给出实际字体，或分不清是失败的 Web 字体还是排在它前面的通用字体族在起作用），不报 `xgent/cjk-font-fallback`，`detect` 在发现之后向 stderr 输出 `note:` 行，写明 URL、元素序号、该段文字的开头和原因。备注不是发现，不进 JSON 发现数组，不影响退出码；没有字体回退发现不等于字体已经核实。

关闭方式（按影响范围选择）：

- `detector.ignoreRules`：项目级关闭，手动 `detect` 与 hook 共用这份过滤；配置在 `.phoenix-ui/config.json`，本地覆盖在 `.phoenix-ui/config.local.json`。不要手改这两个文件，走下面的 hooks 命令。
- `hooks ignore-rule <id>`：把规则 ID 追加到 `detector.ignoreRules` 的受支持写法，影响整个项目；范围较大，先征得用户同意。只关一个文件中的一条时用 `phoenix-ui hooks ignore-value <id> "*" --file <path>`，窄例外默认写入项目共享 config。
- inline 豁免：前缀为 `phoenix-ui-disable*`，词形为 `disable`（整个文件）、`disable-line`、`disable-next-line`（单行），后接一个或多个规则 ID（逗号分隔）或 `*`，可带 `: 理由` 或 `-- 理由`。内联标记按源文件内容生效；URL 扫描读取渲染后的页面，页面规则的例外用项目级忽略。`--no-inline-ignores`（或 `--no-config`）可绕过内联豁免。
- `detect --no-advisory`：只在单次扫描中隐藏 advisory 级发现，warning 级发现与退出码不受影响。

在 Cursor 上启用新版本后，本来含无替代 `focus:outline-none` 的文件会对任何编辑触发 before-edit 拒绝：该检查扫描整份投影后的文件，不扣除编辑前已经存在的发现，所以与该焦点样式无关的编辑也会被拒。处理方式是先修掉该文件的焦点样式；确需临时放行时用 `phoenix-ui hooks ignore-value xgent/focus-outline-removed "*" --file <path>` 只关这个文件中的这一条。不要为绕过拒绝改用 inline 豁免。例外默认通过 `ignore-value` 写入项目 config；`ignore-file`、`ignore-rule` 影响面更大，应先征得用户同意。

## 取证命令（capture）

`phoenix-ui capture` 按步骤把页面操作到命名状态，在多个视口下截图，并逐张写出出处报告。它只证明“页面在这些步骤后渲染成这样、截图本身没坏”：图里是不是所称的状态、好不好、要不要修，归调用方、评审与用户；命令不打分、不出处置词、不读写项目源码。它与内部截图服务 `capture-server`、comp-led 组件评审的 `component-review capture` 是不同的命令。

```bash
DEMO_PASSWORD=<测试密码> phoenix-ui capture http://localhost:5173/ \
  --viewport desktop,mobile --type full --states states.json \
  --out .phoenix-ui/review --block-writes
phoenix-ui capture stitch before.png after.png --match 320,320 --label 修改前,修改后 --out compare.png
```

- 目标：http(s):// 与 file:// 原样打开；本地 `.html` 文件或目录由内置静态服务器经 `http://127.0.0.1:<随机端口>` 提供（目录入口 `index.html`）。视口 `desktop`（1440x900）、`mobile`（390x844）、`<宽>x<高>` 或 `<标签>=<宽>x<高>`；`--dpr`、`--color-scheme light|dark`、`--reduced-motion`。
- 状态文件（`--states`）：`initial` 描述初始状态的截图方式，`states` 逐个列出 `name` 与 `steps`；每个状态在不共享 cookie 与存储的新页面里从头执行自己的步骤。定位用 `{"role":…, "name":…}`、`{"text":…}` 或 `{"css":…}`（可加 `nth`），只查主文档，不进 iframe 与 Shadow DOM；动作有 click、dblclick、hover、fill、type、select、press、drag、scroll、wait、waitFor；原生日期、时间、颜色、范围输入用 fill 按其值格式设值（如 `2026-11-20`），type 落在这类控件上会失败。截图类型 `viewport`、`full` 或 `{"element": <定位>}`；`frames` 出序列帧，`mark` 出编号标注图。完整格式见 `phoenix-ui capture --help`。
- 输出与命名：默认 `.phoenix-ui/capture/`，文件名模板默认 `{viewport}{-state}.png`（初始状态为 `desktop.png`，其他为 `desktop-<状态>.png`），`--name` 可改；报告 `capture-report.json`（schema `phoenix-ui-capture-v1`）逐张记录目标、地址、视口、像素比、配色、截图类型、拍摄时间、浏览器与命令版本、步骤与观察、控制台错误、失败请求与图片、横向溢出、被拦截的写请求和该状态的页面规则发现（`findings`，与 `detect` 同一套规则与项目配置）；页面扫描无法核实某段文字的字体来源时，该图条目另有 `findingsNotes` 数组，文本与 `detect` 的 `note:` 行相同，终端相应输出 `NOTE <状态> <视口> <文件>: …`。`findingsNotes` 表示所写 URL 与段落的字体来源尚未核实，不是发现，没有字体回退发现也不等于已核实：等字体加载稳定后在现有轮次内重取该状态，或人工核对该段的实际字体并记下依据；仍未解除的在最终交付的未验证项里写明 URL、段落与原因，不为它另加阻塞门或检测轮次；已存在的报告按文件名合并，只重取部分状态时其余条目保留。派生图：`--mask` 遮字版、`--thumbs` 扫视缩略图（480/180 宽）、`--sheet` 状态并排图、`frames` 序列帧、`mark` 标注图、`capture stitch` 修改前后拼图，都不替代状态截图。
- 有效性：每张主截图自检步骤是否全部执行、有限动画是否结束、文字是否大面积透明、是否整张单色或有异常全黑带、画布是否绘出、尺寸是否符合截图类型。无效图写成 `<名>.invalid.png`，同名旧图（含失败状态的旧图）删除。退出码：0 全部有效；1 无法运行（参数或状态文件错误、浏览器不可用、目标打不开），不写报告；2 已运行但有失败状态、无效图、派生图未产出或扫描出错。
- 浏览器与隐私：使用本机 Chrome、Chromium、Edge 或 Brave（`PHOENIX_UI_BROWSER` 可指定），无浏览器时退出 1 并说明，其他命令照常；资料目录是临时的，结束后删除。引擎自身不发请求；浏览器通过启动参数关闭组件更新、账号、时间、搜索和 Safe Browsing 密钥等后台联网，目标页面自己的资源照常加载。在本机 Google Chrome 157.0.8103.40 上未观察到目标页面及其资源之外的请求；Edge、Brave、Chromium 的后台联网尚未逐一验证。`--block-writes` 中止页面、iframe 与 worker 发出的非 GET 请求并逐条记录，`--allow-write <路径>` 放行必要写入；凭据用步骤里的 `env` 引用环境变量，它的值与 URL 中的密码在报告、终端与错误信息中替换为 `***`（页面自己显示在截图里的内容不在此列）。
- 走查子代理：Phoenix 随包安装 `phoenix-ui-ux-walker`，以不知道任务的首次使用者身份，通过 capture 重放步骤走一条主要路径，报告卡住、走错与犹豫；Web 完工轮在有主要操作流程时与完工评审同轮派出。

## 素材命令（media）

`phoenix-ui media` 为设计稿与方案预览检索开放授权素材。素材只取到当前项目的 `.phoenix-ui/media/`，不复制进产品资产目录、不从产品代码引用，也不把许可清单或署名写进产品。分级描述条目自身的许可；“可上线”并不授权把素材放进当前产品。

```bash
phoenix-ui media search "old world map|nautical chart" --kind map --sheet
phoenix-ui media preview C1 C2
phoenix-ui media search paper --kind texture --use page
phoenix-ui media fetch C1 T1 --maps all --resolution 1k
phoenix-ui media search "rocket launch" --kind video
phoenix-ui media fetch N1 --video-size medium
phoenix-ui media audit src public
```

- `search`：词组以 `|` 分隔；`--kind all|texture|design|photo|blueprint|map|plate|science|video` 分别为全部、材质贴图、设计器物、摄影、测绘图纸、地图、版画、科学和视频；`--limit` 为 1–30，全部默认 12，其余默认 6。`--use page` 只保留“可上线”条目。`--sheet` 下载预览并生成联系表。
- `preview <编号>…`：为索引里的候选重新生成联系表。
- `fetch <编号>…`：只认当前素材区的编号。贴图的 `--maps color,roughness,normal` 或 `all` 取颜色、粗糙度与 OpenGL 法线，默认 color；`--resolution 1k|2k|4k|8k` 默认 1k。视频的 `--video-size medium|small|preview|orig` 默认 medium。`--max-mb` 限制单个文件，默认 256（1–2048）。下载超时 `--download-timeout-ms` 默认 180000。
- `audit [路径…]`：默认核对整个当前项目，只读且不联网。跳过 `.git`、`node_modules` 与当前项目的 `.phoenix-ui/`，不跟随符号链接。
- 公共选项：`--dir` 可改素材区，但必须仍在当前项目的 `.phoenix-ui/` 内，符号链接不能绕过边界；`--json` 输出结构化报告。联网子命令的 `--timeout-ms` 默认 12000，范围 1000–120000。选项支持 `--key value` 与 `--key=value`。

11 个信源为 Poly Haven、V&A、Wellcome、美国国会图书馆、NASA、Wikimedia Commons、芝加哥艺术博物馆、克利夫兰艺术博物馆、大都会博物馆、Japan Search、荷兰国立博物馆。材质取 Poly Haven；设计器物查各博物馆、Japan Search 与 Poly Haven 模型预览；摄影查 Commons、国会图书馆与 NASA；图纸和地图查国会图书馆及 Commons；版画查 Wellcome、Japan Search 与 Commons；科学查 NASA 与 Wellcome；视频查 NASA。`all` 并发查询全部入口，单个信源失败不影响其余结果。

许可分为 `page`（可上线：许可允许相应用途，仍须核对条目条件）、`check`（需核对：不明确或提及第三方权利）、`reference`（仅参考：非商业、禁止演绎或另需授权）。Poly Haven 为 CC0；美术馆的公有领域或 CC0 项按其声明分级；Japan Search 只取 CC0、公有领域、CC BY、CC BY-SA，ColBase 条目保留“出典：ColBase”署名要求。Commons 按单个文件许可，BY-SA 改色或合成后须同许可共享；V&A 为仅参考。国会图书馆不推定无版权限制，Wellcome 保留条目许可，NASA 保留第三方署名与限制。命令不判断肖像权、商标或具体商业用途。

素材区保存 `index.json`（跨次检索沿用编号）、`sources.json`（逐文件的来源、许可、署名、取用日期、真实尺寸、SHA-256 与完整性）与 `.gitignore`（首次写入 `*`，已有文件不改）。这些文件留在本机；换机器后须重新检索取用。索引不是结果缓存，离线时不会返回旧候选。

`candidates.png` 每格标编号、分级色带与原图尺寸，中文标题看终端或 JSON。`look` 记录整图明暗 `tone`、低饱和度 `mono`、四边色 `edges` 与平均边色 `edge`、边缘是否平坦 `edgeFlat`、浅色纸底 `paper`、可压字区域 `calm` 和宽高比 `ratio`。预览失败的格子显示 `NO PREVIEW`，`look` 为 null。

取用会检查体积、内容类型、文件签名与完整解码：图片必须整张解码成功，坏图不落位、不登记；贴图还核对信源给出的字节数。视频以首帧解码为成功条件，同时生成 `-poster.png`，需要本机浏览器支持该视频编码；浏览器不可用、编码不支持或超时均使该视频取用失败，视频与首帧都不落位。首帧就是第 0 帧，开头是黑场时保留黑图。

交付前对实际产品源码和静态资源路径运行 `media audit <路径…>`（如 `media audit src public`），避免把项目内安装的技能文档当作产品扫描；省略路径时扫描当前目录：`reference` 表示产品引用素材区路径，`copy` 表示存在字节完全相同的副本，`credit` 表示出现来源/下载/许可链接、许可原文或署名。清掉发现后再交付；`credit` 可能碰巧匹配产品文案，主代理判断保留时在交付报告写明依据。

退出码：0 完成且没有错误（搜索零条也为 0）；1 参数/素材区/索引错误、全部编号不存在或全部信源不可用；2 部分信源、预览或取用失败，或 audit 有发现。全部连接失败时明确输出“网络不可用”，不改索引；其他全部失败列出原因。国会图书馆在部分网络下会对非浏览器客户端返回 403，按不可用报告，不绕过限制。

## 交付图片处理（prep-image）

```bash
phoenix-ui embed-prompt src/assets/hero.png --prompt "项目图片的来源说明"
phoenix-ui prep-image src/assets/hero.png --width 600 --widths 600,1200 --out public/img
phoenix-ui prep-image src/assets/logo.png --width 300 --trim --out public/img
phoenix-ui prep-image src/assets/hero.png --width 600 --crop 16:9 --out public/img
phoenix-ui embed-prompt --scan public/img
```

`prep-image` 完全离线，接受 PNG、JPEG、WebP 文件或目录（仅当前层的这三种图片）。输入没有内嵌来源或 `<图片>.json` 旁注时拒绝该文件，先用 `embed-prompt` 写入准确来源。目录输入必须指定 `--out`，且不能输出回该输入目录。

- `--width <显示宽>` 导出显示宽的 2 倍；`--widths <像素宽,…>` 增加实际像素宽版本，至少给一个选项，两者可同给，取值 1–16384。超过原图宽时不放大，报告注明并去重。
- `--trim` 裁全透明边；`--crop <宽:高>` 居中裁切，也接受 `/`、`x` 或小数（0.01–100）。先应用 EXIF 方向，再裁透明边、按比例裁切、缩放；全透明图、APNG 和动画 WebP 被拒绝。
- 不透明像素输出 JPEG，`--quality 1–100` 默认 82；有透明像素输出无损 WebP，缩放使用预乘 alpha 避免暗边，保留原 ICC 配置。不提供有损 WebP、AVIF 或格式选项。
- `--out <目录>` 默认各输入所在目录。文件名为 `<原文件名去扩展名>-<像素宽>.jpg|webp`；本批输出与任何输入冲突或不同输入命中同一输出时，相关输入失败，不覆盖原图；其他既有同名输出可覆盖。
- 两个以上输出生成按宽度升序的 `srcset`/`sizes` 片段，路径相对当前目录并 URL 编码。`--sizes` 可改 sizes；有 `--width W` 时默认 `(max-width: Wpx) 100vw, Wpx`，否则 `100vw`。

原图字节保持不变，报告列出 SHA-256、输出像素尺寸与体积，`--json` 返回结构化报告。来源载体沿用 `embed-prompt`：PNG 的 tEXt、JPEG 的 COM（文字过长改旁注）、WebP 的 `<图片>.json` 旁注；移动 WebP 时须同时移动旁注。输出可直接 `embed-prompt --read` 或 `--scan`。

退出码：0 全部成功；1 参数错误或没有输入，不写文件；2 个别输入失败，其余照常处理。

## 联网边界

| 命令 | 联网范围 |
| --- | --- |
| `context`、`hook`、`detect <源码文件>` | 不发起外网请求，保留既有本机端口探测 |
| `detect <url>`、`capture` | 目标页面与其自身资源；浏览器自身后台请求的验证范围见上文“浏览器与隐私” |
| `media search/preview/fetch` | 显式调用时访问素材信源与条目提供的图片地址；`media audit` 不联网 |
| `prep-image` | 纯本地图片处理，不联网 |

以上命令运行时不自动更新或下载 engine。

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

用 `review-ui` 评审、`apply-ui-review` 按报告修复，再由新的独立评审确认；修复方最多登记「已修改待评审」。这两个 skill 不依赖 Phoenix；检测到 Phoenix 时 review-ui 可借用取证命令与盲测走查（差距来源「盲测走查」，apply-ui-review 复走同一路径），评审记录按 [review-ui 合同](../skills/review-ui/references/report-contract.md) 与 [修复记录合同](../skills/apply-ui-review/references/record-contract.md) 以 Markdown 维护。

## 维护者：统一候选与本地交接

最终 binary 和 bundle 从同一 HEAD/tree/构建输入生成；旧制品或相近 SHA 不替代它。维护工具按 mise 与 build-tools.lock.json 固定，Node 18 是消费底线；工具缺失/版本不符在 prepare 阶段处理，不在 build/test 中隐式补装：

```bash
cd tools/phoenix-ui
mise exec --no-deps -- node ../../scripts/prepare-phoenix-ui.mjs --prepare
# build 与 test:phoenix 自动读取 environmentFile 回执；其他命令仍按回执在当前 shell
# 导出 WASM_BINDGEN、WASM_OPT、PHOENIX_CARGO_DENY；Linux ARM64 还需 PHOENIX_UI_BROWSER。
mise exec --no-deps -- node ../../scripts/check-phoenix-deps.mjs --write
mise exec --no-deps -- npm --prefix ../.. test
mise exec --no-deps -- node ../../scripts/build-phoenix-ui.mjs --development --output-dir ../../local/phoenix-ui/development-release
mise exec --no-deps -- node ../../scripts/smoke-phoenix-ui.mjs --release-dir ../../local/phoenix-ui/development-release --target <本机平台> --stage-runtime
mise exec --no-deps -- npm --prefix ../.. run test:phoenix
```

prepare 显式联网准备锁定依赖、工具、浏览器和公告快照；随后 audit/build/runtime 使用已准备缓存与离线合同，`CARGO_NET_OFFLINE` 不单独证明全部网络已禁止。`--development` 仅是本机开发制品。`check-phoenix-deps.mjs --distribution-dir <目录>` 另核对分发许可，真实 browser/provider 会话仍分别取证。完整维护入口见 [prepare](../scripts/prepare-phoenix-ui.mjs)、[build](../scripts/build-phoenix-ui.mjs)、[audit](../scripts/check-phoenix-deps.mjs)、[runtime tests](../scripts/test-phoenix-ui.mjs)。

统一的 [Phoenix CI](../.github/workflows/phoenix.yml) 在同一次运行中检查 UI 和 Diagram，并给出共同结果。UI 与 Diagram 在 macOS/Linux 四个平台同架构冷构建/运行，不恢复编译缓存，UI 另有 Node18 consumer。每个平台上传 `phoenix-ui-<平台>` 与 `phoenix-diagram-<平台>` 制品，发布只取这八份引擎制品；整次 run 成功即视为全部 job 通过。CI 不持 R2/npm 凭据。

发布只用一条命令，在 **main 分支上干净、已推送且 Phoenix CI 成功**的 HEAD 上执行，不创建独立发布分支，也不从 detached HEAD 发布。版本号先在 `package.json` 改好。改过 `skills/phoenix-ui/src` 或 provider/hook 生成器时，在 `tools/phoenix-ui` 下运行 `mise exec --no-deps -- bun scripts/render.mjs`；改过 `skills/phoenix-diagram` 时运行 `node scripts/build-phoenix-diagram.mjs --sync-bundle`，并提交生成的 skill 与 bundle。两者都不需要 prepare 或原生构建；`VERSION.json` 与 `ENGINE.json` 不提交，开发构建只在本地写出，发布时由脚本写入：

```bash
mise exec --no-deps -- npm run release:phoenix                      # 上传 R2 与 npm 发布前各确认一次，npm 会要 OTP
mise exec --no-deps -- npm run release:phoenix -- --prepare-only     # 只做到安装检查；再按脚本提示的命令（带 --run-id）运行即发布
```

[release-phoenix-ui.mjs](../scripts/release-phoenix-ui.mjs) 依次执行，任一步失败即停：

1. 核对当前分支为 `main`、工作树干净、npm 上还没有这个版本，找到该提交最新一次成功的 Phoenix CI run（也可用 `--run-id` 指定）。
2. 从这次 run 下载 UI 与 Diagram 各四个平台的 engine 制品（`phoenix-ui-<平台>`、`phoenix-diagram-<平台>`），逐个核对 GitHub digest，并确认 CI 构建所用的两个 bundle 与当前提交一致、提交的生成文件与源码一致。
3. 用 `aws s3api put-object --if-none-match '*'` 把八个 engine 上传到 R2 的 `<前缀>/<工具>/v<版本>/ci-<run>/<平台>/<文件>`，从不覆盖：对象已存在时只接受字节相同的；随后从每个公共 URL 回读，核对大小与 SHA-256。凭据取已导出的 `AGENT_RELEASE_R2_*`，没有时读仓根 `.env.cf` 里的同名行。已上传的对象有误时，为同一提交手动再触发一次 Phoenix CI（得到新 run ID；Actions 页面的 Re-run 沿用原 ID）后重新发布，不会撞上旧对象。
4. 从 HEAD（`git archive`）只打一次联合 tarball：写入两个工具的 release 清单及其副本（UI 的 `skills/phoenix-ui/scripts/ENGINE.json`，Diagram 的 `bundle/ENGINE.json` 与 `skills/phoenix-diagram/ENGINE.json`），第三方许可取自 darwin-arm64 的 CI 制品，最终包再次通过发布校验。`xgent-skills install` 默认安装两者。
5. 用 [check-phoenix-ui-release.mjs](../scripts/check-phoenix-ui-release.mjs) 在 Node 18 和维护 Node 上联合新装（两个 engine 从公共 URL 下载），核对 Diagram 引擎摘要、可运行入口及重复安装；另检查 Phoenix 副本覆盖重装和重复安装。
6. `npm publish` 这个 tarball，再从 registry 下载回来核对 shasum、integrity 和 SHA-256。

只改脚本、skill 或文档、引擎源码未变时，可用 `--reuse-engines <已发布版本>` 跳过 CI 和 R2（第 2、3 步），直接复用该 npm 版本的 UI 与 Diagram 引擎：

```bash
mise exec --no-deps -- npm run release:phoenix -- --reuse-engines 0.7.10 [--prepare-only]
```

脚本从 registry 下载该版本并核对 integrity，然后检查：自那次发布以来，编译进 UI 引擎的源码（`tools/phoenix-ui` 中除 `scripts/render.mjs`、`scripts/lib` 外的部分，以及 Rust `include_str!`/`include_bytes!` 引用的仓库文件）在 git 中没有改动；Diagram 的源码输入摘要与旧清单的 `sourceInputSha256` 一致；提交的 `GENERATED.json` 与 Diagram bundle 和当前源码一致。任一项不满足就停止，需要改为走 CI 构建。通过后按第 4 步从 HEAD 打包，只是引擎相关字段和第三方许可来自旧版本，`npmPackageVersion`、`sourceCommit` 和 bundle 摘要仍取当前提交。之后照常执行第 5、6 步。引擎只认 `toolVersion` 与清单内容，不绑定 npm 版本；能复用的最早版本是 Diagram 去掉版本绑定后首次构建发布的版本（0.7.9 及以前不行）。

每步产物保存在 `local/phoenix-ui/releases/v<版本>-<短SHA>/ci-<run>/`（复用引擎时为 `reuse-<版本>/`），中断后重跑同一命令即可续接：已打包就直接从安装检查继续，否则重新核对 CI 制品，已上传的对象只做字节比对。预发布版本必须加 `--tag`。CI 测过的提交之后 main 又有新提交时，等待 main 当前 HEAD 的 CI 成功后再发布；不另建发布分支或检出旧提交发布。

在仓库根目录直接 `npm publish` 会被 `prepublishOnly` 拒绝：检出里没有 release 清单，`dist/phoenix-ui/VERSION.json` 不提交，开发构建写出的也只是没有 engine 下载地址的开发清单。发布过的 R2 对象和 npm 版本都不能覆盖，出错只能发新版本。

替身、开发包和四平台 CI 都不能单独证明线上 URL 或最终包可用。
