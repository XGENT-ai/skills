# xgent-init · @xgent-ai/skills · 新增 skill：为出仓 App 仓库生成 CLAUDE.md / PRODUCT.md / DESIGN.md（新模块 · SKILL.md + 模板 references + 结构检查脚本 · `install` 零提问）

> **计划状态：Ready**
>
> 调查基线：2026-09-02 · commit `a1fc6d9` · clean。
> 已读取规则：`CLAUDE.md`（仓根：Think Before Coding / Simplicity First / Surgical Changes / Goal-Driven Execution）。
> 演进：本计划取代 `install-project-docs`（CLI 交互式生成）。用户 2026-09-02 决定「文档生成整体交给 skill，install 不再做任何提问」；前一计划的事实调查（占位清单、结构硬约束、派生规则、YAML 与替换陷阱）全部平移到本计划。
>
> 本期交付：**`skills/xgent-init/`：一个可经 `npx skills add XGENT-ai/skills --skill xgent-init` 安装到 Claude Code / Cursor / Codex 的 skill。它在出仓 App 自己的仓库里读取 `app.manifest.json` 与代码事实、一次性批量追问缺失信息，按模板生成 `CLAUDE.md` / `PRODUCT.md`（`micro` 型再加 `DESIGN.md`），段落级内容由模型基于事实填写，生成后用自带脚本校验硬结构，不留任何待填占位；已存在的文件不覆盖。** 根目录 `templates/` 迁入 skill 的 `references/` 后删除；`install` 命令不提问、不生成文档，仅可选加一行提示。
> 本期独特职责：与 `portal-external-app`（接入契约）、`portal-micro-app`（SDK）等既有 skill 的差异是：本 skill 只产出三份仓库级文档，不解释平台契约；平台口径全部引用那些 skill，不复制。
> **顶层排除：不生成 `app.manifest.json`、不改 `install` 的 hooks/settings 逻辑、不为门户 monorepo 内的 App 服务、不做 evals。**

## 0. 需求、范围与决策

### 0.1 需求与约束账本

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| R-1 | 功能 | 用户 2026-09-02：「文档生成整体交给 skill，install 不再做任何提问」 | 新 skill 生成三份文档；`install` 零交互 | D1、D2 · V2/V3/V9 | 已确认 |
| R-2 | 功能 | 用户：「利用大模型的能力来填充更多信息」 | 模板里 13 处段落级占位由 skill 基于 manifest、仓库事实与一次批量追问填写；不得编造事实 | D5、D6 · §5 · V2/V3/V4 | 已确认 |
| R-3 | 业务规则 | 用户：「如果对方目录已存在就不覆盖」 | 默认不覆盖已存在文件；只在用户明确要求时做小节级更新并展示改动 | D7 · V5/V6 | 已确认 |
| R-4 | 功能 | 用户：「可以随意修改模板…如果不再需要这些模板，可以删除」；「files 要等你开发完…再决定」 | 模板迁入 `skills/xgent-init/references/`，根目录 `templates/` 删除；`package.json` `files` 已含 `skills`，无需改 | D3 · V8 | 已确认（前计划 D8 自然解除） |
| R-5 | 功能 | 用户：「尽量不要留有 install 结束后，用户还要手动编辑的」 | 生成文件零残留占位与填写指引；由检查脚本断言 | D9 · V1/V2/V3 | 已确认 |
| R-6 | 功能 | 用户：「可以在生成的文档中注明，这些是占位，可以根据需要替换」 | 身份色以 manifest `color` 为准；hover/dark 由 skill 推导并在文件内注明「推导值，可按需调整」 | D8 · V2 | 已确认（语义随 skill 方案调整） |
| NFR-1 | 可移植性 | `README.md`「可安装到 Claude Code、Cursor、Codex 等」 | SKILL.md 不依赖单一 Agent 的专有工具；检查脚本用 Node ≥18 零依赖（与 `bin/xgent-skills.js:3`、`skills/xgent-app-release/scripts/preflight.mjs` 同口径） | D9 · V1 | 已知 |
| NFR-2 | 幂等/不破坏 | `README.md` 幂等承诺的延伸 | 三份齐（或 service 两份齐）时再次运行不写任何文件 | D7 · V5 | 已知 |
| NFR-3 | 输出确定性 | 模型输出有随机性 | 模板固定全部平台文案，模型只填 13 个槽位；硬结构由脚本断言，不靠模型自觉 | D4、D9 · V1 | 已知 |
| C-1 | 约束 | `README.md`「目录结构」；现有 12 个 skill 均为 `skills/<name>/SKILL.md` + `references/` + 可选 `scripts/` | 目录与 frontmatter（`name` / `description`）遵循仓库惯例 | D1 · §1.2 | 已确认 |
| C-2 | 约束 | `package.json` `files: ["bin", ".claude/hooks", "skills"]` | skill 随 npm 包与 GitHub 两条渠道分发，无需改 `files` | V8 | 已确认 |
| C-3 | 约束 | `templates/external-app-DESIGN.template.md:6`、`templates/external-app-PRODUCT.template.md:6` | DESIGN：frontmatter 第 1 行 + 恰好六段固定标题顺序；PRODUCT：七个二级标题固定名字与顺序，`Register` 只能是裸词 `product` / `brand` | D9 检查项 | 已确认 |
| C-4 | 约束 | `templates/external-app-DESIGN.template.md:3`、`templates/external-app-PRODUCT.template.md:23` | `service` 型不生成 DESIGN.md，仍生成 PRODUCT.md | D6 · V3 | 已确认 |
| C-5 | 约束 | `skills/xgent-app-release/scripts/preflight.mjs:26`；`skills/portal-dev-setup/SKILL.md:199` | appKey 合法集合 `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`；`PREFIX` = key 大写、`-`→`_` | D5、D9 · V4 | 已确认 |
| C-6 | 约束 | `templates/external-app-CLAUDE.template.md:10`「门户仓的文档你访问不到，别在本文件里引它的路径」 | 生成文件与 SKILL.md 不引用门户仓路径 | D4 · D9（警告级） | 已确认 |
| C-7 | 约束 | `templates/external-app-DESIGN.template.md:168` | `app-identity` 必须与 manifest `color` 同值 | D8 · D9 | 已确认 |
| C-8 | 约束 | 仓根 `CLAUDE.md` §2/§3 | 最小改动；`install` 不改逻辑 | D2 | 已确认 |
| A-1 | 假设 | PRODUCT/DESIGN 的结构约束来自外部 skill `impeccable`（模板 :6 自述），仓内无其解析器 | 模板结构 = impeccable 期望的结构；检查脚本按模板结构断言 | 实现者在装有 impeccable 的环境用生成文件跑一次 impeccable；不可用时注明未验证 | 开放（非阻塞：模板本就按 impeccable 编写） |
| A-2 | 假设 | manifest `name` 形状两处口径不一致：`templates/external-app-CLAUDE.template.md:102`「多语只有 name 收对象」vs `skills/portal-external-app/references/registration-and-onebox.md:29`「纯字符串」 | skill 两种都接受：对象取 `zh-CN`，字符串直接用 | D5 | 已解除（兼容处理，不依赖哪方正确） |

### 0.2 决策表

| # | 决策点 | 选择 | 含义/影响 | 依据 |
| --- | --- | --- | --- | --- |
| D1 | 形态 | 新增 `skills/xgent-init/`：`SKILL.md` + `references/`（三份模板 + `fill-guide.md` 槽位填写指南）+ `scripts/check-docs.mjs` | 新模块，与既有 skill 零耦合；分发走 skills.sh 与 npm 两条既有渠道 | R-1、C-1、C-2 |
| D2 | `install` 改动 | **零逻辑改动**；可选：在 `完成:` 前打印一行 `提示  项目文档由 xgent-init skill 生成（npx skills add XGENT-ai/skills --skill xgent-init）` | 非交互、幂等特性原样保留；用户不要这一行则整项删除 | R-1、C-8；`bin/xgent-skills.js:74` |
| D3 | 模板去留 | `git mv templates/external-app-*.template.md skills/xgent-init/references/`，文件名不变；删除根目录 `templates/` | `files` 无需改；不再有「根目录模板被谁用」的悬案 | R-4、C-2 |
| D4 | 模板改造 | 删除三份模板顶部到 `✂️` 行的「手工复制说明块」（CLAUDE :1-13、DESIGN :1-15、PRODUCT :1-14，含其后空行），其内容改写进 `SKILL.md` 与 `fill-guide.md`；**保留**全部 `[方括号]` 槽位、`<!-- 填写指引 -->` 注释、`<APP_KEY>`/`<APP_NAME>`/`<PREFIX>` 令牌——它们就是给模型的填写指令 | DESIGN 模板自身第 1 行即为 `---`；槽位清单固定在 §1.3，模板与指南一一对应 | R-2、C-3 |
| D5 | 事实来源顺序 | ① `app.manifest.json`（`listingKey` / `name` / `type` / `color` / `tagline` / `desc` / `navItems` / `helpEntry` / `scopes` / `aclManifest`，字段见 `skills/portal-external-app/references/registration-and-onebox.md:7-11`）② 仓库信号（`package.json`、README、目录与路由/页面/组件命名）③ **一次**批量追问仍缺的槽位。**不得编造**：无法从 ①② 得出的事实只能问，不能猜 | 用户最多被打断一次；每个槽位都能说出来源 | R-2 |
| D6 | 形态判定 | `type` 取 manifest；无 manifest 时追问；`service` → 不生成 DESIGN.md，CLAUDE.md 删去模板 :179-214 四节与 Design Context 中 DESIGN.md / impeccable / 浏览器验证三行 | 与前计划 D10 同口径 | C-4 |
| D7 | 已存在处理 | 生成前逐份检查：存在即跳过并报告；三份（或 service 两份）都在则不写任何文件；只在用户**明确要求更新**时做小节级修改，先展示改动再写，不整份重写 | 比 CLI 的「只能跳过」多出可控更新，仍以不覆盖为默认 | R-3、NFR-2 |
| D8 | 颜色 | `app-identity` = manifest `color`（无 manifest 则追问）；`app-identity-hover`（更深）与 `app-identity-dark`（更亮）由 skill 推导，并在 frontmatter 上方用 YAML 注释注明「由 app-identity 推导，可按需调整」 | 满足 C-7 硬约束；hover/dark 可调 | R-6、C-7 |
| D9 | 结构检查脚本 | `scripts/check-docs.mjs <目标仓根>`，Node ≥18 零依赖，退出非零即失败。断言：(a) 三份文件均不含 `<APP_KEY>` / `<APP_NAME>` / `<PREFIX>`、`填写指引`、`✂️`、中文方括号占位（CLAUDE.md 允许 `[步骤]`/`[检查点]` 两处）；(b) PRODUCT.md 恰好七个 H2、名字与顺序固定，`## Register` 正文只有 `product` 或 `brand`；(c) DESIGN.md 存在时：第 1 行 `---`、`name:` 与 `description:` 值为合法 JSON 字符串、`app-identity:` 与 manifest `color`（存在时）同值、恰好六个 H2 名字与顺序固定、仍含 `Page<T>` 与 `{colors.app-identity}`（防过度替换）；(d) CLAUDE.md：`type: micro|service` 行存在；key 通过 `KEY_RE`；`<大写前缀>_SA_CLIENT_SECRET` 与 key 派生一致；service 时四节标题不存在、micro 时存在；DESIGN.md 存在与否与 type 一致；(e) 警告级：文件中出现 `apps/` `packages/` `docs/` 开头的仓路径样式（C-6） | 结构不靠模型自觉；skill 流程规定「脚本不绿不算完成」 | NFR-3、R-5、C-3、C-5、C-6、C-7 |
| D10 | 触发描述 | frontmatter `description` 中英双写（同 `skills/xgent-app-release/SKILL.md` 风格）：初始化/接入新的出仓 App 仓库、生成或补齐 CLAUDE.md / PRODUCT.md / DESIGN.md 时用；门户 monorepo 内的 App、非 XGENT 项目不用 | 触发面明确 | C-1 |
| D11 | README | 「Skills 列表」表加 `xgent-init` 一行；「Claude Code 辅助工具」节末加一句指向 xgent-init。表格里其余 11 个 skill 缺行是既有问题，本期不补 | 外科手术式改动 | C-8 |
| D12 | 槽位填写指南位置 | `references/fill-guide.md`：每个槽位一行——来源优先级、追问问法、字数/形状约束、示例；SKILL.md 只放流程与红线 | SKILL.md 保持短；指南可单独维护 | R-2、NFR-3 |

### 0.3 ADR-lite

#### ADR-1：文档生成由 skill 而非 CLI 承担

- 状态：Accepted（用户 2026-09-02）
- 背景与驱动：模板 13 处段落级槽位与 6 处填写指引需要读仓库、会追问的执行者；CLI 无模型只能删槽位（前计划 R-5 的代价），且要背上 readline / TTY 门控 / 回滚等交互机制。
- 备选：① CLI 提问生成（前计划）；② CLI 出骨架 + skill 增补（两个执行者共有同一批文件）；③ skill 全权（**选择**）。
- 决策：③。`install` 保持纯非交互；skill 在目标仓内运行，可读 manifest 与代码，满足 C-7 这类只有读到 manifest 才能满足的硬约束。
- 正面后果：零 CLI 交互代码；跨 Agent 可用；已存在文件可做受控更新。
- 负面/中性后果：输出有随机性，靠模板固定文案 + 检查脚本收敛；用户必须先装 skill 再在 Agent 中运行。
- 重新评估触发：出现必须在无模型环境（纯 CI）生成文档的需求。

### 0.4 职责与事实所有权

|  | `references/*.template.md` + `fill-guide.md` | `SKILL.md`（流程） | `scripts/check-docs.mjs` | 目标仓用户 |
| --- | --- | --- | --- | --- |
| 拥有 | 平台文案、槽位定义与填写规则 | 事实采集顺序、追问、不覆盖、裁剪、完成标准 | 硬结构与残留断言 | 领域事实、最终内容 |
| 不拥有 | 领域事实 | **不复述平台契约**（引用 portal-* skill） | 内容质量 | 平台口径 |

一句话：模板定形、脚本定结构、模型定内容、用户定事实。

### 0.5 明确不在本期

- **`install` 自动安装 skill**——放弃；skills.sh 已是既有分发渠道，重复实现无益。
- **evals**——不做；README 提到的 `evals/` 目前没有任何 skill 使用。
- **生成或修改 `app.manifest.json`**——不在职责内。
- **门户 monorepo 内 App 的文档**——不服务（模板自述面向出仓 App）。
- **补齐 README 表格里其余 11 个 skill**——既有问题，另行处理。
- **前计划的 CLI 交互机制**（readline、`--docs`、回滚）——全部作废。

## 1. 当前事实与改动面

### 1.1 现状与缺口

- **已核实·足够**：skill 目录惯例为 `skills/<name>/SKILL.md` + `references/` + 可选 `scripts/`（12 个既有 skill；`README.md` 目录结构节）；frontmatter 只有 `name` 与 `description`（`skills/dev-plan/SKILL.md:1-4`）。
- **已核实·足够**：既有脚本先例：bash（`skills/dev-plan/scripts/check_paths.sh`）与 Node ESM（`skills/xgent-app-release/scripts/preflight.mjs`）；本期检查脚本需解析 JSON（manifest、frontmatter 值），选 Node。
- **已核实·足够**：`package.json` `files` 含 `skills`，新增 skill 自动随 npm 包发布；GitHub 渠道由 skills.sh 直接读仓库。
- **已核实·足够**：`install()` 末行 `完成:`（`bin/xgent-skills.js:74`），可选提示行插在其前；命令本身不读 stdin。
- **已核实·足够**：manifest 身份/展示字段（`listingKey` / `name` / `version` / `cat` / `tagline` / `desc` / `icon` / `color`）、形态字段 `type`（`micro` / `service`）、`helpEntry`、`navItems`、`scopes`、`aclManifest`（`skills/portal-external-app/references/registration-and-onebox.md:7-11`）。
- **已核实·缺口**：三份模板顶部是「手工复制」说明块，且根目录 `templates/` 没有任何使用者 → 迁入 skill 并改造（D3、D4）→ 漏做后果：skill 无模板可用。
- **已核实·缺口**：仓内没有对 PRODUCT/DESIGN 结构的机器检查 → 新增 `check-docs.mjs`（D9）→ 漏做后果：模型输出结构漂移，impeccable 解析失败。
- **已核实·缺口**：README 表格与 install 说明未提及文档生成 → D11。
- **未核实·非阻塞**：impeccable 的实际解析行为（A-1）。

### 1.2 拓扑与文件清单

| 文件 | 改动 | 说明 |
| --- | --- | --- |
| `skills/xgent-init/SKILL.md` ★ | 新增 | frontmatter + 流程（§5）+ 红线 + 完成标准 |
| `skills/xgent-init/references/external-app-CLAUDE.template.md` ★ | 由 `templates/external-app-CLAUDE.template.md` 移入 | 删 :1-13 说明块；其余原样（含 `<APP_KEY>` 等令牌、:81 槽位、:179-214 micro 四节、:236-246 Design Context 与 :243 注释） |
| `skills/xgent-init/references/external-app-PRODUCT.template.md` ★ | 移入 | 删 :1-14；其余原样（9 处槽位、3 处填写指引） |
| `skills/xgent-init/references/external-app-DESIGN.template.md` ★ | 移入 | 删 :1-15 使 `---` 为第 1 行；其余原样（:17-18 name/description、:20-22 三色、:136 标题、:140/:145/:303-308 槽位、:142-143/:305-306 指引） |
| `skills/xgent-init/references/fill-guide.md` ★ | 新增 | §1.3 槽位表的展开版：来源、问法、约束、示例 |
| `skills/xgent-init/scripts/check-docs.mjs` ★ | 新增 | D9 断言 |
| `templates/` | 删除 | 三个文件已移走，目录为空 |
| `bin/xgent-skills.js` | 可选一行 | D2 提示行 |
| `README.md` | 修改 | D11 |
| `docs/plan/xgent-init.md` ★ | 新增 | 本计划（取代 `install-project-docs`） |

无需修改：`.claude/hooks/*`、`.claude/settings.json`、`package.json`、其余 `skills/**`。

### 1.3 槽位清单（模板当前行号；`fill-guide.md` 逐条展开）

| # | 模板:行 | 槽位 | 来源优先级 | 追问问法（仅当①②得不到） |
| --- | --- | --- | --- | --- |
| S1 | CLAUDE 全文 13 处 | `<APP_KEY>` | manifest `listingKey` → 追问；必须过 `KEY_RE` | 「App 的 listingKey？」 |
| S2 | CLAUDE :77 ×2 | `<APP_NAME>`、`type` | manifest `name`（对象取 zh-CN）/ `type` → 追问 | 「App 中文名？micro 还是 service？」 |
| S3 | CLAUDE :151 | `<PREFIX>` | 由 S1 派生，不问 | — |
| S4 | CLAUDE :81 | 本地开发：启动命令、端口、依赖容器、测试入口、启动坑 | `package.json` scripts、compose 文件、README、Dockerfile → 追问 | 「一把起齐本仓的命令是什么？依赖哪些容器？」 |
| S5 | PRODUCT :30 | 普通成员在做什么、首要任务、使用频次 | manifest `tagline`/`desc`、`navItems`、路由/页面命名 → 追问 | 「普通成员进来主要做什么？」 |
| S6 | PRODUCT :31 | 管理员独有操作 | `aclManifest` PID、admin 路由 → 追问 | 「哪些操作只有租户管理员能做？」 |
| S7 | PRODUCT :41 | 产品目的一段话 | `desc` + README → 追问 | 「这个 App 解决什么问题、什么算成功？」 |
| S8 | PRODUCT :56 | 本 App 真正拥有的 3–5 条 | 从 S5–S7 与 `scopes` 命名空间归纳；不足则追问 | 「只有本 App 能做的事有哪些？」 |
| S9 | PRODUCT :62 | 品牌人格三个词 + 语气两句 | 由 S7 与领域推断并标注为建议 | 可不问，写入时说明为建议 |
| S10 | PRODUCT :80 | 具体反面参照 | 追问；无则删该段 | 「有没有明确不想像的产品或旧版本？」 |
| S11 | PRODUCT :89-90 | 两条领域设计原则 | 由 S5–S8 推导；不足则删至保留平台两条 | 可不问 |
| S12 | PRODUCT :101 | 本 App 特有可访问性约束 | 由 UI 形态推断（长表、拖拽、图表）；无则删该行 | 可不问 |
| S13 | DESIGN :17-18、:136 | name、description | S2 + `tagline`/S7 一句话 | — |
| S14 | DESIGN :20-22 | 三色 | manifest `color` + 推导（D8） | 无 manifest：「身份色 #RRGGBB？」 |
| S15 | DESIGN :140 | Creative North Star 一句话 | 由 S7 提炼 | 可不问 |
| S16 | DESIGN :145 | 工作区性格第一段 | 由 S5、UI 形态推断 | 可不问 |
| S17 | DESIGN :303-308 | Signature Component | 从组件目录识别撑起身份的自定义组件；无则整节删（模板自述） | 「有没有一个撑起产品身份的自定义组件？」 |
| — | 6 处 `<!-- 填写指引 -->` | 填完即删 | — | — |
| — | CLAUDE :243 注释 | 路径固定，删 | — | — |

写入后的报告必须列出每个槽位的来源（manifest / 仓库信号 / 用户回答 / 模型建议），用户据此纠正。

## 2. 模块、接口与依赖

| 模块 | 调用者 | 接口与不变量 | 隐藏复杂度 | 依赖分类 | 测试面 |
| --- | --- | --- | --- | --- | --- |
| `SKILL.md` | Agent（Claude Code / Cursor / Codex） | 触发条件；流程 §5；红线：不编造、不覆盖、不引门户仓路径、脚本不绿不算完成 | 事实采集与追问策略 | 进程内（Agent 读文件） | V2–V7 |
| `references/*.template.md` | SKILL.md 流程 | 输出骨架；槽位与指引标记即填写指令 | 平台文案 | 本地文件 | V1 结构基线 |
| `references/fill-guide.md` | SKILL.md 流程 | 每槽位来源/问法/约束 | 填写规则 | 本地文件 | 评审 |
| `scripts/check-docs.mjs <dir>` | SKILL.md 流程、实现者 | 输入目标仓根；读三份文件与可选 `app.manifest.json`；stdout 列出 ERROR/WARN；有 ERROR 退出 1 | D9 断言 | 进程内（Node ≥18） | V1 自测 |

## 3. 数据模型与迁移

不涉及。`templates/` → `references/` 用 `git mv` 保留历史。

## 4. 集成与契约

| 依赖/调用方 | 当前状态 | 行为级证据 | 本期处理 | 失败语义 |
| --- | --- | --- | --- | --- |
| skills.sh 安装 | 已核实足够 | `README.md` 安装节；`skills-lock.json` 记录 `skillPath: skills/dev-plan/SKILL.md` | 目录合规即可被安装 | 目录不合规 → 装不上 |
| npm `files` | 已核实足够 | 含 `skills` | 零改动 | — |
| `app.manifest.json` | 已核实足够 | 字段见 D5 | 只读；缺失则追问 | 字段缺失 → 追问，不猜 |
| `impeccable` skill | 未核实·非阻塞 | 模板 :6 自述结构来源 | 脚本按模板结构断言 | A-1 |
| `portal-external-app` / `portal-micro-app` / `xgent-app-release` / `xgent-image-push` / `portal-dev-setup` / `portal-app-exchange` | 已核实足够 | 模板正文已引用这些 skill 名 | SKILL.md 只引用不复述 | — |

## 5. 核心机制（SKILL.md 流程）

1. **前置判定**：确认目标仓是出仓 App（有 `app.manifest.json`，或用户明确说明）；门户 monorepo 内则停止并指向 `portal-micro-app`。
2. **存在性检查**：逐份检查 `CLAUDE.md` / `PRODUCT.md` / `DESIGN.md`。全部存在（service 两份）→ 报告并停止，除非用户明确要求更新（D7）。
3. **事实采集**：按 D5 顺序读 manifest 与仓库信号，按 §1.3 填一张「槽位 → 候选值 → 来源」表；`name` 兼容字符串/对象（A-2）；`type` 决定是否生成 DESIGN.md 与 CLAUDE 裁剪（D6）。
4. **一次批量追问**：把仍为空且不可推断的槽位合成一轮问题（通常 3–6 个）；用户跳过的槽位按 §1.3「无则删」处理，不留空位、不编造。
5. **渲染**：以 `references/` 模板为骨架逐槽位填写；替换 `<APP_KEY>`/`<APP_NAME>`/`<PREFIX>`；删除全部填写指引注释与 CLAUDE :243 注释；service 型执行 D6 裁剪；DESIGN frontmatter 的 `name`/`description` 用双引号并转义 `"` 与 `\`；不得改动 `Page<T>`、`{colors.x}`、六段/七段标题。
6. **写入**：只写本次缺失的文件（D7）。
7. **校验**：运行 `node <skill 目录>/scripts/check-docs.mjs <目标仓根>`；有 ERROR 则修复后重跑，直到退出 0；Node 不可用时逐条人工过 D9 清单并在报告里注明。
8. **报告**：列出生成的文件、每个槽位的来源、被删除的可选段落、hover/dark 推导值，并提示 A-1（建议在 impeccable 下读一次）。

红线：不编造用户、指标、竞品；不整份覆盖已存在文件；不复述平台契约（引用 portal-* skill）；不引用门户仓路径；脚本不绿不算完成。

## 6. 交互

- 追问只有一轮、分条编号、给默认建议，用户可整体回复「都按建议」。
- 更新已存在文件时先展示将改动的小节全文，确认后再写。
- 报告用一张「槽位 / 值 / 来源」表，不在文件里留任何元标记。

## 7. NFR 与运行保障

| ID | 基线/来源 | 目标 | 机制 | 验证 |
| --- | --- | --- | --- | --- |
| NFR-1 | README 跨 Agent 承诺 | SKILL.md 无 Agent 专有依赖；脚本 Node ≥18 零依赖 | D9 | V1、V10 |
| NFR-2 | 幂等/不破坏 | 终态再次运行零写入 | D7 | V5 |
| NFR-3 | 输出确定性 | 结构 100% 由脚本断言；文案 95% 来自模板 | D4、D9 | V1、V2、V3 |

安全：skill 只读目标仓、只写三份文件；不把 manifest 中任何密钥值写进文档（manifest 本就不许带值，`templates/external-app-CLAUDE.template.md:115`）。

## 8. 失败模式

| 失败/触发 | 爆炸半径 | 用户表现 | 检测 | 恢复 | 验证 |
| --- | --- | --- | --- | --- | --- |
| 模型编造槽位事实 | 单份文档内容 | 报告里来源标为「模型建议」，用户可纠正 | 报告来源表 | 用户改口后小节级更新 | V4 追问断言 |
| 结构漂移（多/少一段、标题改名、frontmatter 不在第 1 行） | impeccable 解析失败 | 脚本 ERROR | `check-docs.mjs` | 修复重跑 | V1 变体自测 |
| 过度替换（`Page<T>`、`{colors.x}` 被改） | DESIGN.md 语义 | 脚本 ERROR | 同上 | 同上 | V1 |
| `app-identity` ≠ manifest `color` | 壳与房颜色不一致 | 脚本 ERROR | 同上 | 同上 | V1、V2 |
| 覆盖已存在文件 | 用户内容丢失 | — | 流程第 2 步 + V5 | git 恢复 | V5 |
| 无 manifest、无 README 的空仓 | 追问变多 | 一轮 6 问 | — | — | V4 |
| Agent 环境无 Node | 脚本不可用 | 报告注明人工核对 | 流程第 7 步 | — | 走查 |

## 9. 验证

### 9.1 脚本自测（V1）

在 scratch 目录构造：① 一套按模板手工填好的合格文件（micro，带 manifest `color: "#FF6600"`）→ 脚本退出 0；② 逐项变体各一份 → 每项恰好报对应 ERROR：残留 `[中文]`、残留 `填写指引`、残留 `<APP_KEY>`、PRODUCT 少一段/顺序颠倒/`Register` 带句号、DESIGN 第 1 行非 `---`、`app-identity` 与 manifest 不同、`Page<T>` 被替换、service 型却存在 DESIGN.md、CLAUDE `[步骤]`/`[检查点]` 存在时**不**报错；③ `node --check scripts/check-docs.mjs`。

### 9.2 真实运行（Claude Code；其他 Agent 见 V10）

- **V2（micro，有 manifest 与 README）**：scratch 仓含 `app.manifest.json`（`listingKey: demo-app`、`type: micro`、`color: #FF6600`、`name` 为对象）与 README；把 `skills/xgent-init` 复制到 `<仓>/.claude/skills/xgent-init/`；在 Claude Code 中要求「初始化本仓的 CLAUDE.md / PRODUCT.md / DESIGN.md」→ 只出现一轮追问；三份文件生成；脚本退出 0；DESIGN `app-identity: "#FF6600"`；报告含来源表。
- **V3（service，无 README）**：manifest `type: service` → 只生成 CLAUDE.md、PRODUCT.md；脚本退出 0；CLAUDE.md 无四节。
- **V4（无 manifest）**：空仓 → 第一轮追问包含 key/name/type；喂 `foo--bar` 通过、`-foo` 被拒。
- **V5（终态幂等）**：对 V2 仓再次运行 → 报告全部存在、零写入（sha256 不变）；再要求「更新 PRODUCT.md 的 Users 段」→ 先展示改动，确认后仅该段变化。
- **V6（部分缺失）**：删 PRODUCT.md 再运行 → 只重建 PRODUCT.md。
- **V7（引号）**：manifest `name` 含 `"` → DESIGN frontmatter 合法，脚本退出 0。

### 9.3 静态与打包

- **V8** `git ls-files templates/` 为空；`npm pack --dry-run` 列出 `skills/xgent-init/SKILL.md`、三份模板、`fill-guide.md`、`check-docs.mjs`。
- **V9**（若保留 D2）`node bin/xgent-skills.js install "$T" </dev/null` → 打印提示行、无提问、退出 0。
- `bash .claude/skills/dev-plan/scripts/check_paths.sh docs/plan/xgent-init.md .`。

### 9.4 跨 Agent（V10）

Codex 或 Cursor 至少一个跑通 V2；无法执行时在交付说明里注明未验证。

## 10. 里程碑

| # | 里程碑 | 内容 | 验证/退出条件 |
| --- | --- | --- | --- |
| M1 | 模板迁移与改造 | `git mv` 三份模板；删说明块；写 `fill-guide.md`；删 `templates/` | `git ls-files templates/` 为空；DESIGN 模板第 1 行 `---`；`grep -c '✂️' skills/xgent-init/references/*.md` 全 0；§1.3 槽位在模板中逐条可定位 |
| M2 | 检查脚本 | `check-docs.mjs` 与 V1 自测 | V1 全绿 |
| M3 | SKILL.md | 触发描述、§5 流程、红线、完成标准 | V2–V7 |
| M4 | 文档与收尾 | README（D11）；可选 install 提示行（D2）；A-1 在 impeccable 下试读 | V8、V9、V10；check_paths |

## 11. 风险、开放问题与就绪状态

| 项目 | 影响 | 责任人/解除办法 | 最晚确认点 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| A-1：impeccable 解析行为未在仓内核实 | DESIGN/PRODUCT 可能不被读取 | M4 在装有 impeccable 的环境试读；脚本已按模板结构断言 | M4 | 否 |
| 模型在不同 Agent 上填写质量不一 | 内容质量 | 模板固定 95% 文案；来源表让用户可纠正 | V10 | 否 |
| D2 提示行是否保留 | 体验 | 用户一句话决定；默认保留 | M4 | 否 |

- 最终状态：**Ready**。
- 定级理由：产品形态已由用户拍板；模板去留（原 D8）随 skill 方案自然解除；所有事实已核实；A-1 只影响验证深度，不影响实现。

## 12. 已知坑与历史教训

1. **不要在 `references/` 里把模板命名为 `CLAUDE.md`**：Claude Code 会把子目录 `CLAUDE.md` 当规则加载；保留 `.template.md` 后缀。
2. **DESIGN :294 `Page<T>`、:83-132 `{colors.x}` 不是槽位**；脚本以其存在性防过度替换。
3. **DESIGN frontmatter 必须在第 1 行**（模板 :14 自述）；M1 删说明块时连同其后空行。
4. **frontmatter 自由文本要转义 `"` 与 `\`**（前计划 D14 教训）。
5. **`readline` 等 CLI 交互机制已作废**，不要把前计划的 M2 带进来。
6. **manifest `name` 形状两处口径不一致**（A-2），两种都接受。
7. **`[步骤]`/`[检查点]` 是 CLAUDE 正文**，脚本白名单。
8. **平台契约不进 SKILL.md**：模板已引用六个 portal-* / xgent-* skill；复述会造成第二事实源（`templates/external-app-CLAUDE.template.md:7-10`）。
9. **一轮追问**：多轮打断是前计划 CLI 方案最大的体验问题，skill 方案不能重蹈。

## 13. 需求 → 设计 → 验证映射

| ID | 需求/约束/假设 | 设计落点 | 验证/解除办法 | 结果 |
| --- | --- | --- | --- | --- |
| R-1 | skill 生成，install 零提问 | D1、D2 | V2/V3、V9 | 覆盖 |
| R-2 | 模型填充更多信息，不编造 | D5、§1.3、§5 步骤 3–4 | V2、V4、报告来源表 | 覆盖 |
| R-3 | 不覆盖，受控更新 | D7 | V5、V6 | 覆盖 |
| R-4 | 模板迁移/删除，files 不改 | D3 | V8 | 覆盖 |
| R-5 | 零残留占位 | D9(a) | V1、V2、V3 | 覆盖 |
| R-6 | 颜色注明可调 | D8 | V2 | 覆盖 |
| NFR-1 | 跨 Agent | D9 Node 零依赖 | V1、V10 | 覆盖 |
| NFR-2 | 终态零写入 | D7 | V5 | 覆盖 |
| NFR-3 | 结构确定性 | D4、D9 | V1 | 覆盖 |
| C-1 | skill 目录惯例 | D1 | V8 | 覆盖 |
| C-2 | files 含 skills | — | V8 | 覆盖 |
| C-3 | 六段/七段/Register 裸词 | D9(b)(c) | V1 | 覆盖 |
| C-4 | service 无 DESIGN | D6、D9(d) | V3 | 覆盖 |
| C-5 | KEY_RE 与 PREFIX | D9(d) | V4 | 覆盖 |
| C-6 | 不引门户仓路径 | D9(e)、红线 | V1 警告 | 覆盖 |
| C-7 | app-identity = manifest color | D8、D9(c) | V1、V2 | 覆盖 |
| C-8 | 最小改动 | D2、D11 | 评审 | 覆盖 |
| A-1 | impeccable 结构 | D9 按模板断言 | M4 试读 | 开放·非阻塞 |
| A-2 | name 形状 | D5 兼容 | V2、V7 | 已解除 |
| — | **不在本期：install 装 skill、evals、manifest 生成、monorepo App、补 README 其余行、CLI 交互机制** | §0.5 | 去向已写 | 排除 |
