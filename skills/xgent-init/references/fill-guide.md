# 槽位填写指南

三份模板在 `references/` 下：`external-app-CLAUDE.template.md` · `external-app-PRODUCT.template.md` · `external-app-DESIGN.template.md`。
它们 95% 的文字是平台口径，**逐字照抄**；只有下面列出的槽位需要你填。行号是模板当前行号，改模板后以内容定位为准。

## 0. 三条硬约束（改坏了 impeccable 就解析不出来）

- **PRODUCT.md**：七个二级标题，名字与顺序固定 —— `Register` / `Users` / `Product Purpose` / `Brand Personality` / `Anti-references` / `Design Principles` / `Accessibility & Inclusion`。别改名、别加段、别调序。`## Register` 的正文只能是裸词 `product` 或 `brand`，不加句号、不加解释。
- **DESIGN.md**：YAML frontmatter 必须从**文件第 1 行**开始（前面多一个空行就整块解析不出来），正文**恰好六段**，名字与顺序照抄 —— `1. Overview` / `2. Colors` / `3. Typography` / `4. Elevation` / `5. Components` / `6. Do's and Don'ts`。不要加 Layout / Motion / Responsive 这类第七段，把内容折进六段里。
- **平台口径的事实源是目标仓里已装的 skill**，不是这三份文件：`portal-external-app`（接入契约 · `app.manifest.json` · `/svc` 与健康检查）· `portal-micro-app`（SDK 握手 / 版头 / consent / iframe 坑）· `xgent-app-release`（发布提案 · 产物打包 · 排错）· `xgent-image-push`（镜像构建与推送）· `portal-dev-setup`（一盒本地联调）· `portal-app-exchange`（跨应用调用）。**门户仓的文档目标仓访问不到，别在生成的文件里引它的路径。**

## 1. 令牌替换（全文）

| 令牌 | 值 | 来源 |
| --- | --- | --- |
| `<APP_KEY>` | listingKey，如 `omni-parser` | manifest `listingKey` → 追问。必须过 `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$` |
| `<APP_NAME>` | App 中文名 | manifest `name`（对象则取 `zh-CN`，字符串直接用）→ 追问 |
| `<PREFIX>` | key 大写、`-` 换 `_`，如 `OMNI_PARSER` | 由 `<APP_KEY>` 派生，**不问** |
| `type: [micro\|service]` | `type: micro` 或 `type: service` | manifest `type` → 追问 |

`<APP_KEY>` 在 CLAUDE 模板里出现十余处（含 `xgent-<APP_KEY>`、`<APP_KEY>_SERVER_PORT`、`/svc/<APP_KEY>`），逐个替换，别漏。

## 2. 段落级槽位

| # | 位置 | 要写什么 | 来源优先级 | 追问问法 | 约束 |
| --- | --- | --- | --- | --- | --- |
| S4 | CLAUDE `### 本地开发` | 起本仓的命令、端口、依赖的基础设施容器、测试与 verify 入口、最容易踩的启动坑 | `package.json` scripts / `compose*.y*ml` / `Dockerfile` / README → 追问 | 「一把起齐本仓的命令是什么？依赖哪些容器？测试怎么跑？」 | 写「一把起齐」的那条命令；坑要具体，不写「注意配置环境变量」 |
| S5 | PRODUCT `## Users` 第 1 条 | 普通成员在做什么工作、一屏之内的首要任务、使用频次 | manifest `tagline` / `desc` / `navItems` + 路由/页面命名 → 追问 | 「普通成员进来主要做什么？多久来一次？」 | 一句话，具体动词；不写「提升效率」 |
| S6 | PRODUCT `## Users` 第 2 条 | 只有租户管理员能做的结构性操作 | `aclManifest` 的 PID、admin 路由 → 追问 | 「哪些操作只有租户管理员能做？」 | 列具体动作（建/删业务对象、改租户级设置） |
| S7 | PRODUCT `## Product Purpose` 首段 | 解决什么问题、为什么值得单独存在、什么算成功 | manifest `desc` + README → 追问 | 「这个 App 解决什么问题？什么算成功？」 | 一段话；具体名词动词，**禁用**「赋能 / 一站式 / 全流程」 |
| S8 | PRODUCT `## Product Purpose` 边界表下方 | 本 App 真正拥有的 3–5 条 | 从 S5–S7 与 `scopes` 命名空间归纳 → 追问 | 「只有本 App 能做的事有哪些？」 | 3–5 条；不能与边界表里「已归门户」的行重叠 |
| S9 | PRODUCT `## Brand Personality` | 三个词 + 两句语气说明 | 由 S7 与领域推断 | 可不问 | 标注为**建议**；语气两句要回答：陈述句还是引导句、错误信息说人话还是错误码、空状态给指令还是给解释 |
| S10 | PRODUCT `## Anti-references` 末段 | 具体的反面参照（某竞品、某旧版本）及**具体哪一点**不要 | 追问 | 「有没有明确不想像的产品或旧版本？」 | 用户说没有 → **整段删掉**，保留上面继承的六条 |
| S11 | PRODUCT `## Design Principles` 第 3、4 条 | 两条领域设计原则 | 由 S5–S8 推导 | 可不问 | 是战略判断（「先给答案再给表格」），不是视觉规则（「用 8px 圆角」——那属于 DESIGN.md）。推不出就删到只剩平台那两条 |
| S12 | PRODUCT `## Accessibility & Inclusion` 末条 | 本 App 特有的可访问性约束 | 由 UI 形态推断（长表格 / 拖拽 / 图表） | 可不问 | 无特有约束 → **删掉该行** |
| S13 | DESIGN frontmatter `name` / `description`，与 `# Design System: …` 标题 | App 中文名；一句话说清这块工作区是什么、给谁用 | S2 + `tagline` / S7 | — | 三处名字一致；frontmatter 是 YAML，值里的 `"` 与 `\` 必须转义 |
| S14 | DESIGN frontmatter `colors.app-identity` / `-hover` / `-dark` | 三个色值 | `app-identity` = manifest `color`（**硬约束**，见 §3）；hover / dark 推导 | 无 manifest：「身份色是多少（#RRGGBB）？」 | 见 §3 |
| S15 | DESIGN `## 1. Overview` 的 Creative North Star | 一句话北极星 | 由 S7 提炼 | 可不问 | 一句话，带画面感（例：「像一张随时可核对的账」） |
| S16 | DESIGN `## 1. Overview` 第一段 | 这块工作区的性格：密集数据面还是引导式流程、用户一屏之内在找什么 | 由 S5 与 UI 形态推断 | 可不问 | 其后两段（「你画的是内容区」「主题不是你的选择」）**原样保留** |
| S17 | DESIGN `### [Signature Component]` | 撑起产品身份的自定义组件：结构、状态、由来 | 组件目录里识别 | 「有没有一个撑起产品身份的自定义组件（编辑器 / 画布 / 时间线）？」 | 没有 → **连标题整段删掉**；有 → 标题换成组件真名 |

## 3. 身份色

- `app-identity` **必须**与 manifest 的 `color` 逐字相同 —— 一个在壳里（应用市场 / 侧栏的 App Glyph），一个在房里，色不一样就是同一个 App 说两种话。无 manifest 时追问，别用模板的占位值 `#2563EB`。
- `app-identity-hover` 由 `app-identity` 压暗一档（约 −12% 亮度），`app-identity-dark` 提亮一档（约 +18% 亮度，深色底上饱和蓝紫普遍掉到 AA 以下）。两个都是**推导值**：在 frontmatter 上方留一行 YAML 注释注明「由 app-identity 推导，可按需调整」。
- 不要撞 XGENT 蓝 `#0063D3`，不要撞已有 App 的色。

## 4. 必须做的一次取舍：主按钮归谁

DESIGN `## 2. Colors` 的 A / B 表格是**给用户的选择题**，不是背景介绍。渲染时按判断挑一个，在表格下方写明选了哪个、为什么，并把没选的那行留在表里（它解释了另一种先例）：

- **A. 继承门户蓝 `brand-blue` #0063D3** —— App 的交互与门户高度同构（表单、审批、配置类）。
- **B. 用身份色 `app-identity`** —— App 有强烈领域感（内容创作、数据分析），进来之后是「另一间房」。

选 B 时 `brand-blue` 仍保留在调色板里，只用于**指回门户**的东西。

## 5. 可选小节的删留

| 位置 | 留的条件 | 删法 |
| --- | --- | --- |
| CLAUDE `### 前端：版头归门户` 及其后三节（`下拉框` / `字典表` / `App 图标`） | `type: micro` | `service` 型：四节整删。保留时把标题里的括号条件「（`micro` 型才有；`service` 型删掉本节及以下三节）」去掉 |
| CLAUDE `### 字典表统一带 sort + name_i18n` | 本 App 有租户级可维护的枚举 / 分类表 | 没有就整节删；保留时去掉标题里的括号条件 |
| CLAUDE `## Design Context` 整节 | `type: micro` | `service` 型：删掉 DESIGN.md 那行、「设计阶段用 impeccable」那行、「验证阶段在真实浏览器里」那行，并把开头那句改写成不提 impeccable、不提前端 UI 的说法（例：「本仓有一份产品上下文文档：」），只留 PRODUCT.md 那行 |
| PRODUCT `## Users` 第 4 条业务角色 | 本 App 有自己的业务角色（老师/学生、审核人/提交人…） | 有就补一条，并写明它是「门户角色 + ACL PID 的组合」还是「你自己表里的一列」——后者要说明如何与租户隔离共存 |
| PRODUCT `## Anti-references` 末段 · `## Design Principles` 3–4 条 · `## Accessibility` 末条 · DESIGN `### [Signature Component]` | 见 §2 的 S10 / S11 / S12 / S17 | 无内容就整段删，**不要留空槽位** |

## 6. 六处填写指引注释与两处元注释

模板里 `<!-- 填写指引：… -->` 共六处（PRODUCT 三处、DESIGN 两处、CLAUDE `## Design Context` 一处），**填完即删**。CLAUDE `## Design Context` 里那条讲链接路径的注释也删 —— 三份文件同在仓根，链接就是模板里写的样子。

## 7. 不要动的东西（防过度替换）

- DESIGN `## 5. Components` 里的 `Page<T>` 是平台分页契约的类型名，不是槽位。
- DESIGN frontmatter `components.*` 里的 `{colors.app-identity}` / `{rounded.md}` 等花括号引用是 token 引用语法，保持原样。
- CLAUDE 里的 `[object Object]`（讲多语对象被拼成字面量的那处）、`[]`（`setBreadcrumbs` 首页推空数组）、`[步骤]` / `[检查点]`（Goal-Driven Execution 的示例）都是正文，不是槽位。
- DESIGN frontmatter 的全部色值、字体、圆角、间距、组件尺寸是门户 token 的权威副本，**一个字不改也是对的**；除身份色三条外不要调。
