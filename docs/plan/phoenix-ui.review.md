# PHOENIX-UI 开发计划评审报告

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-UI · XGENT Phoenix UI](phoenix-ui.md) | `sha256:23306e15b8f006f45eca841e89302c23f7cfddadf87cff094cb7d908485ee83d`（未跟踪新文件，564 行；计划自述调查基线 `b2289fa`） | 全文 |

- **评审基线**：2026-10-08，HEAD `b2289fa05f1e4f0251bc3a86e3c04f182aba156e`（main）。工作树 dirty，未提交的只有本计划和 `.xgent-ai/sdlc/` 台账，以当前工作树为事实源。评审结束时复核，计划 hash 与 HEAD 均未变化。
- **上游对照**：沿用计划的约定，`U:` 指 [skill-v4.5.0 @508d7e8](https://github.com/pbakaus/impeccable/tree/508d7e8955de3b3caf2d8676e85206723d41a887) 内的相对路径，`E12:` 指 [engine-v0.1.12 @83dc4b6](https://github.com/pbakaus/impeccable/tree/83dc4b60ca4c3a13ea3cde748804858d37ef47c9)。两者都是按固定 commit 拉取的只读副本。
- **评审方式**：主 agent 加 4 个独立只读 reviewer，分别负责 facts、architecture、schedule、delivery，并行初审。主 agent 逐条复核了被采纳问题的关键证据并做了实测；4 个视角共提出 42 条候选，按根因合并为下列 30 条。
- **结论**：是否需要修改：**需要**。建议状态：**Blocked**，与计划自报的 Proposed 不一致，理由见 §5。

## 1. 确认问题

1. [P2] F-01 · §2.2 的工具链契约与 mise、rustup、cargo 实际生效的规则不符
   - 定位：`phoenix-ui.md:198`（§1.3 mise.toml、rust-toolchain.toml）、`:241-247`（mise.toml 只写 version 和 mr_boxington，components/targets 放在 rust-toolchain.toml）、`:252-257`（从仓根以 `--manifest-path` 调用）、`:263`（xtask 由 mise/mbx 执行）、`:190`（`.cargo` 的 xtask 配置导入 tools/phoenix-ui）。关联 C-1、NFR-3、V-1、M1。
   - 问题：
     - mise 激活 Rust 时会设置 `RUSTUP_TOOLCHAIN`。rustup 里环境变量的优先级高于 toolchain 文件，所以计划写进 `rust-toolchain.toml` 的 wasm32 target 和 rustfmt/clippy 组件，不会因为这个文件而被安装。
     - 本机 1.99.0 工具链现在没有 wasm32。M1 跑 `xtask bundle` 时，要么直接失败，要么靠 wasm-pack 自己联网补装，后者与"构建不隐式下载"冲突。
     - Cargo 从当前工作目录向上查找 `.cargo/config.toml`。从仓根执行时，`tools/phoenix-ui/.cargo` 里的 `xtask` alias 不生效。
     - 实施者到 M1 时只能现场重新决定工具链布局。
   - 证据：
     - 主 agent 实测：`mise exec rust@1.99.0 -- env` 输出含 `RUSTUP_TOOLCHAIN=1.99.0`；`rustup show active-toolchain` 输出 "1.99.0-aarch64-apple-darwin (overridden by environment variable RUSTUP_TOOLCHAIN)"。
     - `~/.rustup/toolchains/1.99.0-aarch64-apple-darwin/lib/rustlib/` 下只有 darwin 和 linux-musl 两类 target，没有 wasm32。
     - 官方文档（architecture reviewer 于 2026-10-08 查阅）：[mise Rust](https://mise.jdx.dev/lang/rust.html)；[rustup overrides](https://rust-lang.github.io/rustup/overrides.html)，环境变量优先于 toolchain 文件；[Cargo config](https://doc.rust-lang.org/cargo/reference/config.html)，从当前目录向上查找。
     - `U:.cargo/config.toml` 里 xtask 是一个 alias。
   - 修订：
     - 把 profile、components、targets 写进 mise.toml 的 rust 工具选项，发布用的 target 按 CI job 追加。rust-toolchain.toml 只作为不用 mise 时的兜底，并与 mise.toml 保持一致。
     - 规定所有 Rust 和 xtask 命令以 `tools/phoenix-ui` 为 cwd，或者把 `.cargo` 与 workspace 集中到一处。
     - 构建入口加断言：rustc/cargo 为 1.99.0；wasm32 已安装；target 由 mbx 管理。
   - 复核：
     - 改稿后：§2.2 和 V-1 包含上述断言。
     - 实现后：M1 在干净环境 `mise install` 之后，运行 xtask 和 §2.2 的全部命令，过程中没有隐式的 rustup 安装。
   - 置信度：高，有本机实测和官方文档。"toolchain 文件里的 targets 被完全忽略"是按优先级推断的，见待核实 V-2。阻塞：M1。

2. [P2] F-02 · M1 把 fmt --check 和 clippy -D warnings 加在承接源码上，上游从未设这道门
   - 定位：`phoenix-ui.md:249-257`（维护入口含 `mbx fmt --check` 和 `mbx clippy -- -D warnings`）、`:471`（V-1 以 §2.2 的命令为入口）、`:108`（ADR-1"不做无收益的全仓字符串替换"）、`:182`（能力账本）。关联 V-1、M1。
   - 问题：
     - 承接代码在 1.99.0 自带的 rustfmt 下大面积不合格式，上游 CI 也不跑 fmt 和 clippy，按计划 V-1 过不了。
     - 如果为了过门做全仓格式化或告警清理，会打乱与上游的 diff 可比性，影响能力账本，也影响以后逐个移植上游补丁。
     - 这个取舍会落到实施者身上，在 M1 现场决定。
   - 证据：
     - architecture reviewer 在固定源码副本上用 1.99.0 的 rustfmt 检查：259 个 src 文件中 128 个有差异，共 2,094 处。主 agent 复核了差异文件数。
     - `U:.github/workflows/ci.yml:182-185, 200-201` 只有 `cargo build/test`，没有 fmt 或 clippy 步骤。
     - clippy 没有实际运行。
   - 修订：M1 先记录 fmt 和 clippy 的基线，再从以下两种做法中选一种写进计划：
     - 做一次纯格式化提交，登记到 UPSTREAM.json 和 `.git-blame-ignore-revs`，并用测试或 oracle 证明行为没有变化。
     - 承接的 crate 暂不阻断；新 crate（review、安装器）从一开始就用 `-D warnings`；承接的 crate 用 `[lints]` 允许清单逐步收紧。
   - 复核：
     - 改稿后：V-1 写明 fmt/clippy 的适用范围和基线。
     - 实现后：M1 记录里有基线数据。
   - 置信度：fmt 高，clippy 中（未编译）。阻塞：M1。

3. [P2] F-03 · 用 `xtask bundle --check` 的逐字节比较做 native/WASM 新鲜度门，换机器或换工具链必然误报
   - 定位：`phoenix-ui.md:358`、`:434`（NFR-2 的机制）、`:450`（§8"构建阶段直接失败"）、`:472`（V-2 含 `xtask bundle --check`）。关联 NFR-2、V-2、M2。
   - 问题：
     - `--check` 对 `crates/live/assets/` 下两个跟踪文件做逐字节比较，其中页内脚本嵌着 wasm 二进制。
     - wasm 的字节会随 wasm-pack、wasm-opt、rustc 的版本以及构建机路径变化，上游 CI 因此刻意不检查这个文件。
     - 按计划的做法，换一台机器，或者第一次用 1.99.0 重新生成，都会被判 stale。
     - 维护者本机的路径还会随页内脚本和 binary 一起分发出去。
   - 证据：
     - `U:crates/xtask/src/main.rs:61-103`：`check` 分支对 `detect-antipatterns-browser.js` 和 `antipatterns.json` 逐字节比较。
     - `U:.github/workflows/ci.yml:140-146` 的注释原文是 "The in-page bundle beside it is deliberately not checked … diffing it would fail on toolchain drift rather than on a real change"，CI 只 diff `antipatterns.json`。以上两条主 agent 已复核。
     - architecture reviewer 解码了上游已提交的页内 wasm，其中有 105 处 `/Users/paulbakaus/.cargo/registry/...` 绝对路径。主 agent 没有复算。
   - 修订：
     - 新鲜度门改为比较与路径无关的输入摘要：规则注册表 JSON、browser-bundle 与 wasm 源码、工具版本。native 与 WASM 的一致性用同一组向量测试证明。
     - wasm 构建加 `--remap-path-prefix`。锁定 wasm-pack、wasm-bindgen、binaryen 的版本，并禁止它们自行安装。
     - M1 首次重新生成时登记为工具链差异，用向量测试或 oracle 证明行为不变。
   - 复核：
     - 改稿后：V-2 和 §8 不再以页内脚本字节相等为门。
     - 实现后：两个不同路径的 checkout 得到相同的门结果，产物里不含本机路径。
   - 置信度：高。阻塞：M2（V-2）。

4. [P2] F-04 · 上游 skill 源是带占位符的 `SKILL.src.md` 模板，计划没有定下 `skills/phoenix-ui` 的落盘形态、渲染器和子代理放在哪
   - 定位：`phoenix-ui.md:173`（锚点写成 `U:skill/SKILL.md`）、`:192`（skills/phoenix-ui 是唯一设计 skill 源）、`:352`（frontmatter 处理）、`:360`、`:364`。关联 R-3、C-2、V-10、M2、M4。
   - 问题：
     - `U:skill/SKILL.md` 不存在。上游源是 `SKILL.src.md`，正文和 reference 里有大量 `{{scripts_path}}`、`{{command_prefix}}` 等占位符，由上游 Bun 构建按 19 个 provider 分别渲染。上游刻意不把源文件命名为 SKILL.md，就是为了防止 `npx skills` 装到未渲染的源。
     - 照计划直接放成 `skills/phoenix-ui/SKILL.md`，skills.sh 用户会装到含占位符的 skill。保留 `SKILL.src.md`，skills.sh 和本仓的 YAML 测试又都扫不到它，§5.1 说的"独立通过 skills.sh 安装的 skill 缺机器工具时继续允许有说明的人工流程"就不成立。
     - `U:skill/agents/*.md` 子代理定义在安装时落到各 harness 的 `agents/`，与本仓 `skills/*/agents/` 只放 openai.yaml 的约定冲突，计划没有安排。
     - M4 的 V-9 子集要求"安装后的源码版本"，因此隐含依赖 M2 的渲染器和 M3 的安装器。
   - 证据：
     - `U:skill/` 下只有 `agents/ reference/ scripts/ SKILL.src.md`。
     - `U:skill/SKILL.src.md` 的 frontmatter 含 `user-invocable: true` 和 `allowed-tools: Bash(npx impeccable *)`；正文含 `{{scripts_path}}` 5 处、`{{command_prefix}}` 3 处、`{{command_hint}}` 1 处（主 agent 复核）。facts reviewer 统计，全目录共 7 类占位符，分布在 34 个文件里。
     - 渲染产物 `U:.claude/skills/impeccable/SKILL.md` 里才有 `version: 4.5.0`。
     - facts reviewer 引用的上游注释 `U:scripts/lib/utils.js:199-204` 说明了命名原因。
     - 本仓 [test_skill_metadata.py:15-22](../../test/skills/test_skill_metadata.py) 允许的键不含 `user-invocable` 和 `version`。
   - 修订：
     - §1.3 和 §5.1 写明以下几点：
       - 模板源放在哪里，例如 `tools/phoenix-ui/skill-src/`。
       - 渲染器用什么：承接上游的 transformer，还是用 Rust 实现。
       - 提交到 `skills/phoenix-ui/SKILL.md` 的是不是一份不含占位符的通用渲染，以及它面向 skills.sh 是否可用（见 D-2）。
       - 子代理源怎样映射到各 provider。
     - §1.2 的锚点改为 `U:skill/SKILL.src.md`。§4.3 改为"源里有 user-invocable 和 allowed-tools，version 由构建注入"。
     - M4 前置补上"M2 渲染器和开发安装方式"。
   - 复核：
     - 改稿后：§1.3 能找到模板、渲染器、子代理三项的归属。
     - 实现后：`rg '\{\{[a-z_]+\}\}' skills/phoenix-ui` 为空；`npm test` 通过；渲染产物与 vendor 基线逐项对比一致。
   - 置信度：高。阻塞：M2（bundle 生成）、M4。

5. [P2] F-05 · 计划说本地方向库"复用既有 catalog/roll_selection"，但上游的实际行为与此不符
   - 定位：`phoenix-ui.md:179`（§1.2 方向候选一行）、`:232`（§2.1"复用 catalog / roll_selection"、"无远端目录适配器"）、`:388`（§5.2 的候选不足返回不足、缺图用文字候选、四种 mode 都能走通）。关联 R-8、D8、V-2、V-6、M2、M4。
   - 问题：计划要求的几项行为，与上游的实际行为相反：

     | 情形 | 计划要求 | 上游实际行为 |
     | --- | --- | --- |
     | 本地目录有校验错误 | 只用本地目录 | `load_local` 只要有一处错误就返回 None，然后回落到远端 roll API |
     | 某个 well tier（graphic/interaction/atmosphere）没有已批准条目 | 返回"不足" | 返回 Err，concept-seed 以 1 退出 |
     | 条目缺卡图 | 用文字候选 | 输出上游站点的卡图 URL |
     | 条目形态 | 自有条目可用 | 校验器用 BLAND_FORM_RE 拒绝特定的"平淡"形态，会卡住 Portal 和 operate 类的首批自有条目 |

     如果按"复用"直接承接，非法目录会静默走远端或降级，tier 不全时直接报错，无图卡片仍然指向上游。
   - 证据（主 agent 已复核）：
     - `U:crates/context/src/concept_seed.rs`：
       - `:263-270`：校验失败返回 None。
       - `:366-383`：None 时走 `fetch_roll`；`select_approved_challengers(...)?` 向上传播错误。
       - `:585-592`：Err 时退出码为 1。
       - `:84-86`、`:204-216`：缺 cardBoard/cardHero 时拼出 `card_base` URL。
     - `U:crates/context/src/roll_selection.rs:177-178`：tier 缺失时返回 Err。
     - `U:crates/context/src/catalog.rs`：`:12` 定义 WELL_TIERS，`:24` 和 `:166` 用 BLAND_FORM_RE 拒绝。
     - 上游仓库里唯一的目录是测试夹具。
   - 修订：
     - §1.2 和能力账本登记必须修改的行为：校验失败和 tier 不足时返回什么、禁止回落远端、无图时走文字候选。
     - 写明 XGENT 条目是否沿用三个 tier、system 规则前缀和禁词规则。
     - V-2 和 V-6 增加用例：非法目录、只有一个 tier 的目录、无图条目、operate 和 Portal 条目。
   - 复核：
     - 改稿后：上述差异在 §1.2 和 §5.2 有登记。
     - 实现后：断网并用非法目录运行，没有远端请求，并明确提示候选不足；同一 seed 的结果稳定。
   - 置信度：行为证据高，影响判断中。阻塞：M2 退出、M4。

6. [P2] F-06 · 网络边界只按"二进制直接发 HTTP"建模，漏了间接外发；deny-upstream 进程测试证明不了 R-1 和 NFR-4
   - 定位：`phoenix-ui.md:366-376`（允许的网络接触面和检查范围）、`:76`（NFR-4"不上传代码、截图、规范或偏好"）、`:436`、`:472`（V-2"默认运行零 upstream 请求"）、`:558`。关联 R-1、NFR-2、NFR-4、M2。
   - 问题：在"拒绝原域名"的进程级测试下，以下几条路径照样能通过：
     1. concept-seed 和 serve-question 把卡图的 http(s) URL 输出给页面 `<img>` 和 agent（见 F-05）。页面加载由浏览器发出，不经过被测进程。
     2. 渲染后的 skill 和 reference 仍然指示 agent 运行 `npx impeccable …`，`allowed-tools` 也放行了它，会拉取上游 npm 包。
     3. live copy-edit 默认是 `auto` 模式，优先拉起已登录的 `codex`，其次是 `claude` CLI，在仓库源码上改文案。源码因此经由非宿主厂商的模型外发，与 NFR-4 冲突，计划没有登记。
     4. 环境变量映射漏了 `IMPECCABLE_CARD_BASE`、`IMPECCABLE_LIVE_COPY_AGENT*` 等。

     facts reviewer 另外报告了三处，主 agent 未逐一复核：`help` 会请求 `/api/commands`；字体渲染会加载 Google Fonts；generate-image 读取 `OPENAI_API_KEY`。
   - 证据（主 agent 已复核）：
     - `U:crates/context/src/concept_seed.rs:84-86, 204-216`。
     - `U:skill/SKILL.src.md` frontmatter 中的 `allowed-tools: Bash(npx impeccable *)`。
     - `U:crates/live/src/copy_edit_agent.rs:1270-1316`：`IMPECCABLE_LIVE_COPY_AGENT` 缺省为 `auto`，`command_authed("codex")` 优先。
   - 修订：
     - §5.1 增加"间接外发"一行，覆盖两类：输出给页面或 agent 的 URL 和命令；运行时拉起的 agent CLI。每类写明处置方式，例如 copy-edit 默认只在用户显式选择时调用，并提示会外发给谁（见 D-5）。
     - 补齐环境变量映射。
     - V-2 增加三项检查：扫描渲染后的 skill 和各 verb 的输出中是否有上游域名和 `npx impeccable`；页面级网络记录（HAR）；copy-edit agent 选择逻辑的测试。
   - 复核：
     - 改稿后：§5.1 有这一行及其处置。
     - 实现后：扫描零命中，HAR 里没有上游域名。
   - 置信度：中高。阻塞：M2。

7. [P2] F-07 · 迁移只认领项目级的 4.5.0，漏掉本包发布过的 4.3.1、安装器改写后的 hook 形态和个人级/插件安装；M1 也没有产出迁移 fixture
   - 定位：`phoenix-ui.md:292`（"允许的迁移来源版本"没有列举）、`:295`（各 provider"使用各自旧结构精确匹配"）、`:296`（只发现项目内的旧入口）、`:523`（M3 前置"M1迁移fixture"，全文仅此一处）、`:521`/`:471`（M1 和 V-1 不含该产物）、`:459`（切换门保留"必要的旧迁移元数据"）。关联 R-6、NFR-1、V-4、M1、M3。
   - 问题：
     - **旧版本**：npm `@xgent-ai/skills@0.3.0`（2026-09-23 发布）随包带的是 skill 4.3.1 / engine 0.1.5，0.4.0 起才是 4.5.0。计划规定"与固定旧 bundle 摘要一致才移到备份，有修改就停在预检"，于是 0.3.0 用户哪怕没改任何文件也会被判冲突。R-6 对 XGENT 自己的早期用户不成立。
     - **改写形态**：安装器写入的 hook 已经改写过，带 `[ ! -f … ] ||` 守卫、`.agents` 路径和 `commandWindows`，与上游 bundle 原形不同。"按旧结构精确匹配"必须包含这些改写形态。
     - **其他安装范围**：上游 `--scope=global`、插件安装，以及用户级的旧分命令 skill，会和项目内的 Phoenix 同时被路由。
     - **没有生产者**：这些认领元数据和 fixture 是 M3 的前置，却没有里程碑负责产出。切换门移除旧 vendor 之后，再想补就没有来源了。
   - 证据：
     - `git show f1e2554:vendor/impeccable/VERSION.json` 为 4.3.1/0.1.5。
     - `npm view @xgent-ai/skills@0.3.0 gitHead` 为 `ce5bf26`，在 f1e2554 之后、739e717 之前。
     - `739e717` 起为 4.5.0。以上三条主 agent 已复核。
     - [bin/xgent-skills.js:65-83, 126-151](../../bin/xgent-skills.js)：改写形态和两代 marker。
     - architecture reviewer 发现本机 `~/.agents/skills` 下有 audit、polish 等旧分命令 skill。
   - 修订：
     - §3.1 增加一张"迁移来源 × 安装范围"表：
       - 本包发布过的每个 bundle（至少 4.3.1 和 4.5.0），以及安装器改写后的形态，都可以自动迁移。
       - 其他上游版本、个人级安装、插件安装，分别规定"自动迁移 / 只报告 / 冲突"以及对应的解决命令。
       - 个人级目录只检测，不修改。
     - M1 的内容和退出条件增加"旧安装认领元数据与迁移 fixture"，覆盖两套 bundle × 各 provider × 两代 hook 写法 × 用户改动变体，并归入一个检查 ID。
   - 复核：
     - 改稿后：M3 的这项前置能在 M1 退出条件里找到对应产物。
     - 实现后：用 0.3.0 tarball 装出的 fixture 能无冲突迁移；个人级 fixture 的 dry-run 会列出遮蔽和重复入口。
   - 置信度：高。阻塞：M1（fixture）、M3。

8. [P2] F-08 · 迁移没有处理旧工具在项目里生成的派生物：pin 快捷 skill 和中断的 live 会话留下的东西
   - 定位：`phoenix-ui.md:282`（迁移对象只有原 `.impeccable` 配置等）、`:284`（只阻止"正在运行"的旧进程）、`:296`；`:177`、`:95`（品牌和状态隔离）。关联 R-6、NFR-1、V-4、M3。
   - 问题：
     1. **pin 快捷 skill**：上游 `pin` 在 harness 的 skills 目录里生成以命令命名的快捷 skill（例如 `polish`），正文是"Invoke /impeccable polish"。它的名字不含 impeccable，也不在 bundle 清单里。迁移把旧 skill 移进备份后，这些快捷入口指向的 skill 已经不存在。
     2. **中断的 live 会话**：会在用户的 HTML 入口留下指向 localhost 的注入标签和 CSP 补丁，或在源码里留下 `impeccable-variants-start/end` 包裹的变体。旧的注入记录在 `.impeccable/live/`。迁移后 Phoenix 用独立状态目录和新标记运行，既识别不了，也清理不了。开发期脚本和放宽过的 CSP 可能被提交甚至上线。
   - 证据（主 agent 已复核）：
     - `U:crates/context/src/pin.rs:21`（`<!-- impeccable-pinned-skill -->`）、`:81`（快捷 skill 的正文模板）。
     - `U:crates/live/src/live_inject.rs:1-3, 22-29`：插入和移除 script 标签、打 CSP 补丁，读取 `.impeccable/live/config.json`。
     - `U:crates/live/src/live_wrap.rs:374-390`：源码中的变体标记。
   - 修订：§3.1 增加"旧工具派生物"一类对象：
     - 按 pin 标记识别快捷 skill，转成 Phoenix 的 pin，或列为待处理。
     - 预检检查三项：未结束的会话、未清理的注入记录、受跟踪源码里的旧标记。发现任何一项就阻止切换，并给出用旧 launcher 完成或丢弃会话的命令。另一种做法是在能力账本里规定 Phoenix 识别旧标记，但只用于清理。
   - 复核：
     - 改稿后：§3.1 有这类对象及其处置。
     - 实现后：V-4 增加含 pin 和中断 live 会话的 fixture；dry-run 能列出它们，按提示处理后源码里没有旧标记。
   - 置信度：pin 高；live 残留中（发生概率低，但后果落在用户代码上）。阻塞：M3。

9. [P2] F-09 · 安装和迁移输出缺少用户完成过渡所需的信息
   - 定位：`phoenix-ui.md:425`（§6 只要求状态和"迁移摘要"）、`:298`（回滚需要事务 ID）、`:328`（"原用户文档不重写"）。关联 R-6、M3。
   - 问题：迁移完成后，用户还要做几件事才能真正切换过来，但输出里都没有提示：
     - **重新信任或重载 hook**：迁移改写 hook 之后，Codex 需要在 `/hooks` 里重新审阅信任，其他 provider 需要重载。不提示的话，新 hook 会静默不生效。
     - **项目文档里的旧指引**：用户项目中由模板生成的 AGENTS.md 写着"设计阶段用 impeccable…没有该 skill 时说明缺失并提示安装"。迁移后 agent 每次做设计任务都会建议用户重装 impeccable，照做就会重新引入上游 skill 和 hook。
     - **旧命令**：用户输入 `/impeccable <cmd>`（Codex 中是 `$impeccable`）时没有任何指引。
     - **回滚信息**：回滚要用的事务 ID、完整命令和备份位置，计划没要求输出。
   - 证据：
     - [external-app-AGENTS.template.md:109](../../skills/xgent-init/references/external-app-AGENTS.template.md)。
     - [bin/xgent-skills.js:678](../../bin/xgent-skills.js)：用 `COPYFILE_EXCL` 复制，已有的 AGENTS.md 永远不会被更新。
     - 同文件 `:589-592`：现有 Codex guard 已经提示"配置或脚本更新后重新审阅信任"。
     - 同文件 `:825`：当前提示的是 `/impeccable init`。
   - 修订：
     - §6 列出安装和迁移输出必须包含的内容：
       - 按 provider 前缀给出新旧命令对照。
       - 各 provider 的信任或重载步骤。
       - 事务 ID 和可直接复制的回滚命令。
       - 备份位置。
     - §3.1 增加"项目文档中的旧入口指引"：扫描 AGENTS.md、CLAUDE.md、DESIGN.md、PRODUCT.md 里的 impeccable 指引，只报告并给出替换建议，不改原文。
     - 是否提供一个版本周期的 `impeccable` 过渡入口，交用户决定（D-3）。
   - 复核：
     - 改稿后：§6 和 §3.1 有上述条目。
     - 实现后：带模板 AGENTS.md 的迁移 fixture 在 dry-run 中能列出这些项；V-5 迁移后的会话里，agent 不再建议安装 impeccable。
   - 置信度：缺口高；各 provider 的具体后果中。阻塞：M3。

10. [P2] F-10 · engine 不可用时的行为没有合同：队友、新机器、离线和不支持的平台
    - 定位：`phoenix-ui.md:269`（"普通 hook/context 执行不联网补下载"）、`:291`（预检要求可用 binary）、`:362`（不支持的平台在预检失败）、`:446`（"明确未安装与离线包入口"）、`:94`（D4 离线包可预置引擎）、`:473`（V-3"cache空/命中/损坏与离线行为正确"）。关联 R-1、R-6、C-3、V-3、V-5、M3。
    - 问题：
      - **队友和新机器**：现有安装器支持团队共享 hook（共享的 `.claude/settings.json`，以及用 git 根定位的 `.github/hooks`）。以下几种情况本机都没有 engine：队友克隆后没跑 install、换了新机器、清了缓存。上游 launcher 会在首次运行时下载，Phoenix 不再下载。
        - 沿用上游行为，缺 binary 时以 127 退出，各 provider 每次编辑都报 hook 错误。Cursor 的编辑前 hook 失败时是否会阻止编辑，待核实。
        - 如果 skill 目录没有提交，守卫命令又会静默跳过，设计检测悄悄失效。
      - **补装入口不合适**：唯一的补装入口 `xgent-skills install` 会改写项目共享文件，不适合"只补本机 engine"。
      - **离线包没定义**：来源、格式和参数都没有。
      - **不支持的平台会退化**：预检失败会连 statusline 和 AGENTS.md 一起装不上，而现在的行为是只跳过 engine、其余照装。
      - **"行为正确"不可判定**：没有给出期望输出。
    - 证据：
      - `U:skill/scripts/impeccable:199-206`：下载失败或不可用时 `exit 127`（主 agent 复核）。
      - [bin/xgent-skills.js:54-66](../../bin/xgent-skills.js)：共享和可移植的 hook 设计；同文件 `:462-470`：不支持的平台只跳过 engine。
      - [README.md:80](../../README.md)：当前由 launcher 首次运行时下载。
    - 修订：在 §2.3 和 §8 写一份"engine 可用性合同"：
      - hook 在 binary 缺失或版本不符时放行（退出 0），每个会话最多提示一次，并给出确切命令。
      - CLI 和 context 返回 4，提示同一条命令。
      - 增加一个只写用户级 cache 的 `engine install`：按项目 receipt 的版本安装，用包内 manifest 校验。
      - 定义离线 release-dir 的来源、格式和参数。
      - 决定不支持的平台上是"跳过 Phoenix、其余照装"，还是"整体失败并提示 `--no-phoenix-ui`"。
      - V-3 和 V-5 写明每种情况的期望输出，并增加"clone 后未装 engine"的场景。
    - 复核：
      - 改稿后：§2.3 和 §8 有这份合同，V-3 和 V-5 引用它。
      - 实现后：空 cache 的 fixture 下，各 provider 不阻止编辑，且只提示一次；离线安装在零网络下成功。
    - 置信度：缺口高；各 provider 的具体后果中。阻塞：M3。

11. [P2] F-11 · 安装冲突的结果、退出码和解决路径都没有定义；skills.sh 副本和个人级同名 skill 会遮蔽或架空融合版
    - 定位：`phoenix-ui.md:364`（同名自有 skill"内容不同时报告冲突，不覆盖 skills.sh 安装副本"）、`:229`（"全选 provider 成功才切 active"）、`:291`、`:293`。关联 R-3、R-4、R-6、V-4、M3。
    - 问题：
      - **冲突必然发生**：README 推荐的首选安装方式就是 `npx skills add XGENT-ai/skills`。照做过的用户再跑 install，必然遇到"同名、内容不同"的辅助 skill。
      - **冲突后怎么办没写**：计划没说这会阻止整个安装（按 L291 和 L229 推断，连 statusline 也装不上），还是只跳过这一个 skill。也没有给出认领、切换或保留外部版本的命令，没有规定冲突时的退出码，脚本化安装会把冲突当成功。
      - **只跳过也有问题**：项目里实际生效的仍是不带 CLI 契约的外部副本，融合静默失效。
      - **个人级遮蔽**：Claude Code 中同名 skill 个人级优先于项目级，`~/.claude/skills` 里的副本会让项目内的融合版永远不被调用。而冲突检查只看项目路径。
      - **是否公开未定**：phoenix-ui 本身是否在 skills.sh 公开，计划没有决定。本仓已有 `metadata.internal` 的先例。
    - 证据：
      - [README.md:10](../../README.md)。
      - [skills/xgent-init/SKILL.md:3-4](../../skills/xgent-init/SKILL.md)：`metadata.internal: true`。
      - [Claude Code skills 文档](https://code.claude.com/docs/en/skills)（主 agent 于 2026-10-08 查阅）原文："Enterprise over personal, and personal over project. With `deploy` in both `~/.claude/skills/` and the project's `.claude/skills/`, `/deploy` runs the personal one"。
    - 修订：
      - §5.1 规定冲突分级：
        - 用户改过的旧工具文件：阻止 Phoenix 部分，同时提供继续安装其余部分的路径。
        - 外部管理的辅助 skill：不阻止安装，但输出写明当前生效的是哪一份，以及切换命令。
      - 有冲突时退出码非 0。
      - 检测个人级同名副本，并提示它会遮蔽项目内的版本。
      - phoenix-ui 是否在 skills.sh 公开，交用户决定（D-2）；README 写清两条安装渠道各自能得到什么。
    - 复核：
      - 改稿后：§5.1 和 README 有冲突分级和对应命令。
      - 实现后：V-4 增加 skills.sh 副本（复制和符号链接两种）以及个人级副本的 fixture。
    - 置信度：中。"阻止整个安装"是由 L229 和 L291 推出来的，计划没有直接写。阻塞：M3。

12. [P2] F-12 · 共享配置文件有多个写入者，按整文件摘要认领会让回滚在常见状态下被拒；Node 与 Rust 的写入顺序也没定义
    - 定位：`phoenix-ui.md:196`、`:273`（迁移事务只在 Rust 实现，Node 只做 bootstrap）、`:291`（统一预检）、`:294`、`:297`（文件"仍等于上次写入摘要"才继续）、`:298`（回滚"只恢复未再改动的文件"）。关联 R-6、NFR-1、V-4、M3。
    - 问题：
      - **多写入者**：
        - `.claude/settings.local.json` 同时承载 Claude goal guard 和 Phoenix 的 Claude hook。
        - `.codex/hooks.json` 同时承载 Codex guard 和 Phoenix 的 Codex hook。
        - `.claude/settings.json` 同时承载 statusLine 和共享 hook。
        - Claude Code 自己也会在用户选择"Yes, and don't ask again"时，把 allow 规则写进 `settings.local.json`。
      - **回滚在常见状态下不可用**：按整文件摘要判断，安装后只要正常用一段时间，中断恢复和回滚就会被拒绝。
      - **写入顺序未定义**：Node 写 guard/statusline 和 Rust 写 Phoenix 的先后没有规定。如果 Rust 先写，下次安装会把自己写过的内容当成用户改动。预检通过后 Rust apply 如果失败，Node 已写的部分不在事务里。
    - 证据：
      - [bin/xgent-skills.js:57-63](../../bin/xgent-skills.js)：HOOK_ARTIFACTS 的落点。
      - 同文件 `:533, 580`：Codex guard 写 `.codex/hooks.json`；`:607, 652`：Claude guard 写 `settings.local.json`。
      - 同文件 `:786-787` 与 `:823`：guard 先于 impeccable 写入；`:804-821`：statusLine。
      - [Claude Code settings 文档](https://code.claude.com/docs/en/settings)（主 agent 于 2026-10-08 查阅）原文："When Claude asks permission to run a Bash command and you choose "Yes, and don't ask again", Claude Code saves that permission approval here as an `allow` rule."
    - 修订：
      - receipt 和 journal 按条目记录所有权（事件、matcher、handler 身份）以及被移除的旧条目；回滚做条目级逆补丁，容忍无关修改。
      - 写清写入顺序，二选一：
        - Node 的 guard/statusline 写入在 Rust apply 之前完成，Rust 在锁内重新读取现状。
        - 把这些写入并入 Rust 的同一个计划和 journal。
      - V-4 增加三个场景："新增 allow 规则后回滚"、"改 guard 阈值后回滚"、"在 Node 与 Rust 阶段之间中断"。
    - 复核：
      - 改稿后：§3.1 写明条目级所有权和写入顺序。
      - 实现后：上述三个场景都能回滚成功，且不碰无关条目。
    - 置信度：高。阻塞：M3。

13. [P2] F-13 · 五平台 CI 验证需要已推送的提交，与"验收通过后才提交"互为前置；推送授权和原生 runner 都没有登记
    - 定位：`phoenix-ui.md:473`（V-3"新CI matrix"，执行时点 M3/M6）、`:523`（M3 退出含 V-3）、`:528`（M3 验收完成后才提交）、`:37`（提交前须通过该提交点要求的检查）、`:541`（§11 责任人"实现者安排CI"）、`:522`（M2"接通…五平台build"，但退出只验本地制品）。关联 R-1、C-1、V-3、M2、M3、M6。
    - 问题：
      - **互为前置**：GitHub Actions 只能对已推送的 ref 运行。V-3 需要一个已提交并推送的候选树，而计划只允许 M3 验收通过后才提交，两者形成环。
      - **推送授权没有登记**：推送需要用户授权，但它不在任何里程碑的前置里。§11 把责任人写成了无权推送的实现者。
      - **后果**：goal 执行到 M3 时必然停下，重新决定在哪个分支提交、谁授权推送、CI 证据的 SHA 怎样对应最终提交。平台构建失败也要到 M3 才第一次暴露。
      - **runner 不足**：上游的 linux-arm64 用 cross 构建且不跑 smoke，darwin-x64 在 arm64 主机上构建。V-3 要求的"原生/对应环境 smoke"超出了上游已有的运行环境。
    - 证据：
      - 仓库目前没有 `.github/`。
      - `U:.github/workflows/release-engine.yml:24-30`（构建矩阵）、`:47-55`（linux-arm64 为 `cross: true`，smoke 只在 `!matrix.cross` 时跑），主 agent 已复核。
      - schedule reviewer 用 `gh repo view` 查得：仓库公开，Actions 已启用。
    - 修订：
      - §10 写明 CI 验证流程：主负责人把待验收树提交到临时验证分支，经用户授权推送，以该 SHA 的 CI 结果作为 V-3 证据。阶段提交树必须与它一致，否则重跑。M6 同理。
      - §11 把"推送与验证分支的授权、五平台 runner（含 linux-arm64 原生运行、darwin-x64 的运行方式）"的责任人改为用户或仓库管理员，最晚确认点设在 M2 退出。
      - M2 退出时加跑一次只构建、不测试的矩阵。
    - 复核：
      - 改稿后：M3/M6 的前置或 §11 能找到推送授权，并有"CI SHA 等于提交树"的规则。
      - 实现后：M3 记录里 V-3 引用的 run SHA，与 M3 提交树一致。
    - 置信度：高。阻塞：M3、M6；不阻塞 M1/M2 开发。

14. [P2] F-14 · 发布链路没有设计：CI 制品如何交到 R2 和 npm、凭据放在哪、如何保证不覆盖上传、发布前如何排练
    - 定位：`phoenix-ui.md:197`（`publish-phoenix-ui-r2.mjs`）、`:199`（CI workflow）、`:267`（发布对象"不可覆盖"；包内 manifest 是信任锚）、`:457`（发布次序）、`:473`（"公开R2回读仅在实际发布时"）、`:480`（V-10 的"发布演练"实际是 npm pack 安装）、`:542`。关联 D4、NFR-2、V-3、V-10、M3、M6。
    - 问题：
      - **制品交接没写**：五平台制品只能在 CI 上构建，R2 凭据却只在维护者本机的 `.env.cf` 里。CI 产物如何到达生成 manifest 和执行上传的那台机器、如何证明上传的字节就是通过 smoke 的那一份，计划都没有写。
      - **"不可覆盖"未经测试**：现有上传用 `aws s3 cp`，会直接覆盖同名对象。"不可覆盖"是新增行为，却没有任何发布前测试，第一次执行就在生产环境。
      - **演练的不是实际发布的包**：§8 的演练发生在"更新随包 manifest"之前，实际 publish 的 tarball 不是演练过的那一个。
      - **信任根没有要求**：包内 manifest 是 binary 的信任锚，因此 npm 发布身份就是信任根，计划没有对它提出要求（2FA 或可信发布）。
      - **没有负责里程碑**：发布脚本没有归属。
    - 证据：
      - [scripts/publish-vendor-r2.mjs:11](../../scripts/publish-vendor-r2.mjs)：凭据读自 `.env.cf`；同文件 `:96-101`：用 `aws s3 cp` 上传。
      - [README.md:91](../../README.md)。
      - R2 的 S3 兼容 PutObject 支持条件写，见 [Cloudflare R2 S3 API](https://developers.cloudflare.com/r2/api/s3/api/)（architecture reviewer 于 2026-10-08 查阅）。
    - 修订：
      - §8 写定发布 workflow：冷构建（发布 job 不恢复编译缓存），五平台 smoke，输出摘要清单。
      - 制品交接：按 run ID 下载，核对摘要和 smoke 记录后才生成 manifest。凭据位置二选一并写定。
      - 上传用条件写（`If-None-Match: *`），或先 HEAD 检查、存在即拒绝。
      - 更新 manifest 后重新 pack，用真实 URL 做隔离安装，再执行 publish。
      - 写明 npm 发布身份的要求。
      - 发布脚本归 M6；若 A 阶段要发布，则归 M3。
      - V-3/V-10 增加用 aws/curl 替身的排练，断言"key 已存在""缺平台""回读不符"三种情况都会被拒绝。
    - 复核：
      - 改稿后：§8 与 V-3/V-10 引用同一份摘要清单和三类拒绝用例。
      - 实现后：发布演练记录里有 run ID、摘要比对和拒绝用例的结果。
    - 置信度：中高。阻塞：M3 发布演练、M6。

15. [P2] F-15 · "阶段可并行开发、提交只纳入该阶段已验收内容"在同一工作树里做不到
    - 定位：`phoenix-ui.md:528`（提交点和并行约束）、`:524`（M4"开发不等待M3无关平台测试"）、`:525`（M5 可先用 fixture 开发）、`:530`（只给 bin/xgent-skills.js 和 package.json 指定了独占负责人）、`:364`（bundle 含 5 个 XGENT 辅助 skill）、`:267`（`bundleSha256`）。关联 M3、M4、M5。
    - 问题：M3 等待 CI 和真实会话的这段时间，M4/M5 会在同一工作树里改共享路径：
      - **bundle 由整树生成**：`vendor/phoenix-ui` bundle 和 `bundleSha256` 从整棵 skills 源生成，其中包括 M4/M5 要改的 design-md、review-ui 等。
      - **新 crate 自动进入 workspace**：上游 workspace 用 `members = ["crates/*"]`。M5 新建的 `crates/review` 会自动进入 M3 的 `mbx test --workspace --locked` 和 Cargo.lock。
      - **共享文件没有负责人**：`skills/phoenix-ui`、`crates/context`、cli 路由、Cargo.toml/lock、catalog、README、xgent-init 被 A、B 两个阶段同时修改，没有单一负责人。

      结果是，M3 的证据和提交要么混入 B 阶段内容，要么被 B 阶段的失败拖累；部分暂存也得不到"A 阶段 bundle"。
    - 证据：`U:Cargo.toml:6`（`members = ["crates/*"]`，主 agent 已复核）；计划 `:193`（`crates/review` 的位置）。
    - 修订：
      - §10 选定一种隔离方式并写明，二选一：
        - M3 提交前，B 阶段只在独立 worktree 或分支上开发。计划和记录只在主工作树由主 agent 回写，M3 提交后再 rebase。
        - M3 提交前，B 阶段只动不进入 workspace 和 bundle 的新路径。
      - 补齐单一负责人：Cargo.toml/lock、cli 路由注册、`skills/phoenix-ui`、bundle 与 VERSION.json 的生成。
      - 注明 M3 的 bundle、binary 和 CI 都从 A 阶段树生成。
    - 复核：
      - 改稿后：§10 能回答"M3 提交时，M4/M5 的改动在哪里"。
      - 实现后：M3 提交的 diff 不含 `crates/review` 和 B 阶段的 skill 改动；用提交树重新生成的 `bundleSha256` 与记录一致。
    - 置信度：高。阻塞：M3 提交，以及 M4/M5 与 M3 的并行安排。

16. [P2] F-16 · §1.3 的接口同步落点没有里程碑归属；M3 切换默认安装时，安装器复制的 AGENTS 模板仍把用户引回 impeccable
    - 定位：`phoenix-ui.md:201-204`（同步落点清单）、`:328`（xgent-init 的更新放在 B 阶段）、`:480`（V-10 在 M3 检查一致性）、`:524`（M4"接通…Portal初始化"）、`:526`（M6 前置"全部接口更新齐备"）、`:206`（写作 `report_type`）。关联 C-2、V-10、M3–M6。
    - 问题：
      - **规则要求**：AGENTS.md 的「Working in This Repository」要求，接口变化要在同一次变更里同步所有复述它的位置。
      - **AGENTS 模板会误导**：M3 把默认安装切到 phoenix-ui 时，`xgent-skills install` 仍会原样复制 AGENTS 模板，其中写着"设计阶段用 impeccable…提示安装"。新项目的 agent 会被引去安装上游工具，与 R-1/R-6 相反。
      - **同类引用**：DESIGN 模板、fill-guide、check-docs.mjs、`skills/prd/SKILL.md:238`、`skills/review-dev-plan/SKILL.md:22`、README、package.json 的 description、安装提示。
      - **归属问题**：
        - xgent-init 的改动归在 M4，而 V-10 要求在 M3 时就一致，形成双重归属。
        - agi-mode 的 playbooks/references/evals、两份 openai.yaml、`docs/phoenix-ui.md` 都没有负责产出的里程碑。
      - **漏列的文档**：两份随包发布的 goal-guard 文档引用了 `--no-impeccable`，没有列入 §1.3。
      - **字段名**：L206 写的 `report_type`，在台账协议里实际叫 `review_type`。
    - 证据：
      - [external-app-AGENTS.template.md:109](../../skills/xgent-init/references/external-app-AGENTS.template.md)、[external-app-DESIGN.template.md:137](../../skills/xgent-init/references/external-app-DESIGN.template.md)。
      - [bin/xgent-skills.js:21, 675-678](../../bin/xgent-skills.js)：复制模板。
      - [docs/claude-context-goal-guard.md:41](../claude-context-goal-guard.md)、[docs/codex-context-goal-guard.md:24](../codex-context-goal-guard.md)；[package.json:26, 29](../../package.json) 显示它们随包发布。
      - [AGENTS.md:94](../../AGENTS.md)。
      - `.xgent-ai/sdlc/protocol.md` 字段表中的 `review_type`。以上均经主 agent grep 复核。
    - 修订：给 §1.3 的每个同步落点标明负责里程碑，至少如下：

      | 里程碑 | 负责的同步落点 |
      | --- | --- |
      | M3 | AGENTS 模板中的设计入口行、xgent-init 的安装描述、prd/review-dev-plan 中的示例名、README 的安装与 vendor 节、两份 goal-guard 文档、package.json description、安装提示、旧 vendor/publish 脚本入口 |
      | M4 | Portal/DESIGN 模板、fill-guide、check-docs、design-prototype、ui-delivery、interface-design |
      | M5 | sdlc、what-next、scenarios、两份 openai.yaml、state-model/protocol 的适用性核对 |
      | M6 | `docs/phoenix-ui.md`、README 的 B 阶段部分 |

      另外把 L206 改为 `review_type`。
    - 复核：
      - 改稿后：§1.3 每一行都能找到负责里程碑。
      - 实现后：在 M3 执行 `rg -n impeccable skills README.md docs bin package.json`（排除历史计划和记录），只剩兼容别名和来源说明。
    - 置信度：AGENTS 模板一项高，其余中。阻塞：M3、M6。

17. [P2] F-17 · A 阶段（M3）是否对外发布没有决定，M3 的退出条件缺少与"切换默认安装"相称的文档和路由验收
    - 定位：`phoenix-ui.md:523`（"才切换默认安装；A阶段可独立使用"）、`:524`（"M3未过前不发布融合版"）、`:526`（M6"完成文档"）、`:204`（`docs/phoenix-ui.md`）、`:479`（V-9 路由验证在 M4/M6）。关联 R-6、M3、M6。
    - 问题：
      - **默认切换会被带进发布**：M3 提交后，main 上的安装器默认就会迁移到 Phoenix。这个包近期发布很频繁：0.4.0、0.5.0、0.6.0 分别发布于 2026-10-02、10-03、10-05。M4–M6 期间从 main 发任何一版，都会把 A 阶段带给所有用户。
      - **配套内容不到位**：
        - 安装、迁移、回滚和支持矩阵的文档要到 M6 才完成。
        - A 阶段的安装集合没有写明：Phoenix 和 5 个辅助 skill 中哪些进 M3 的 bundle。
        - A 阶段安装集合下的自然语言路由也没有验证。
      - **后果**：用户会拿到一次找不到回滚文档、路由也没验证过的迁移。
      - **决定没登记**：这属于用户的分期决定，计划没有登记。
    - 证据：主 agent 用 `npm view @xgent-ai/skills time` 复核了以上发布时间；计划中的相关行如定位所列。
    - 修订：
      - §11 登记"A 阶段是否单独发布"（D-1），最晚在 M3 开工前确认。
      - 如果发布：把安装、迁移、回滚与支持矩阵文档，以及 A 阶段安装集合下的路由评测，并入 M3 的退出条件，并写明 M3 的安装集合。
      - 如果不发布：§8 写明 M3 到 M6 之间怎样防止默认切换随版本发出，例如把默认切换的提交移到 M6，或 M3 只在分支上保留。
    - 复核：
      - 改稿后：M3 一行与 §8、§11 的说法一致。
      - 实现后（如果决定发布）：M3 记录里有文档和路由评测的证据。
    - 置信度：中高。阻塞：M3。

18. [P2] F-18 · M6 的收口清单漏掉了最终包上的迁移复验（V-4），以及 V-7 自己声明要在 M6 做的真实取证
    - 定位：`phoenix-ui.md:526`（M6 退出条件为"V-9、V-10及受影响V-2/V-3/V-5"）、`:477`（V-7 执行时点"M5局部；M6真实取证复核"）、`:474`（V-4 只在 M3 执行）、`:267`、`:364`。关联 NFR-1、V-4、V-7、M6。
    - 问题：
      - **V-4 漏了**：B 阶段会改变安装载荷，包括辅助 skill 的内容、`reviewSchema` 和 `bundleSha256`。最终发布的是 M6 的包，但以下路径只用 A 阶段 bundle 验过：从 impeccable 迁移到最终包、A→B 升级（如果 A 阶段发布过）、外部同名 skill 冲突、回滚。M6 的退出条件是封闭清单，实施者会照着清单跳过 V-4。
      - **V-7 漏了**：V-7 声明在 M6 执行，M6 的退出条件却不负责它。
      - 迁移是独立的风险门，不能靠复用 A 阶段的证据绕过。
    - 证据：定位中所列计划行；另见 `phoenix-ui.md:22`（实施协议要求"收口仍跑必要集成回归"）。
    - 修订：M6 退出条件改为"V-9、V-10、V-4（最终包，适用时含 A→B 升级）、V-7 真实取证，以及受影响的 V-2/V-3/V-5/V-6/V-8"。
    - 复核：
      - 改稿后：M6 一行包含 V-4 和 V-7。
      - 实现后：M6 记录中有用最终 tarball 运行的 V-4 用例。
    - 置信度：中高。阻塞：M6。

19. [P2] F-19 · 迁移和回滚只在文件层验收，没有覆盖迁移后真实会话里的结果
    - 定位：`phoenix-ui.md:474`（V-4 比较备份、hash 和 receipt）、`:475`（V-5 的真实会话针对新装项目）、`:480`、`:457`（"迁移/回滚演练"没有通过判据）、`:295`（"旧、新设计 hook 不同时生效"）。关联 R-6、NFR-1、M3。
    - 问题：
      - **用户层的结果没有检查**：用户判断迁移是否成功，看的是会话里的结果，但下面这些都没有检查覆盖：
        - 会话里只剩一个设计 skill。
        - 每次编辑只触发一次设计 hook。
        - goal guard 仍然生效。
        - 回滚后旧工具在会话里能用。
      - **文件层通过不代表会话正常**：文件层全部通过时，以下情况仍会让会话里的结果不一样：信任没刷新（F-09）、个人级副本遮蔽（F-11）、旧入口残留（F-08）。"旧、新设计 hook 不同时生效"这一条本身，也需要按各 provider 实际的加载规则来判断。
      - **V-9 有歧义**：V-9 写的是"安装后的源码版本"，没说明是不是在 tarball 安装的项目上运行。
    - 证据：定位中所列计划行；Claude Code 个人级优先于项目级（见 F-11 的证据）。
    - 修订：
      - V-5（M3）加入这条链路：
        1. 分别用 0.6.0 和 0.3.0 装出真实项目，再用候选包迁移。
        2. 在 Claude 和 Codex 会话中统计每次编辑的 hook 执行次数，列出可用的 skill，确认 goal guard 仍工作。
        3. 回滚，确认旧工具在会话中可用。
      - V-9 写明在 tarball 安装的项目上运行。
    - 复核：
      - 改稿后：V-5 的方法列包含上述链路。
      - 实现后：M3 记录里有会话层面的证据。
    - 置信度：中高。阻塞：M3。

20. [P2] F-20 · 人工 Markdown 循环和 Rust 裁定是两份并存的规则实现，缺少单一规则源、同步机制和模式切换语义
    - 定位：`phoenix-ui.md:120-127`（ADR-3）、`:163`、`:313`（投影写入 applied 记录的位置）、`:315`（纯 Markdown 循环仍可人工运行）、`:346`、`:419`（投影 hash 冲突即停）、`:511`（行为评测"CLI不可用"没有期望结果）。关联 R-4、C-4、V-7、V-9、M5。
    - 问题：
      - **两份实现会漂移**：同一套计分、停滞和判定规则，一份由 agent 读 Markdown 执行，一份由 Rust 实现。AGENTS.md 的接口同步清单不含 Rust 实现，修改合同时不会有任何测试失败，两者迟早不一致。
      - **跨环境续做没有出路**：一个已经由 CLI 管理的循环，如果某一轮在没有 engine 的环境里人工推进（另一台机器、只装了 skills.sh、或 F-10 的缺失情况），就会出现以下连锁问题：
        - round.json 与报告和修复记录的 hash 冲突。
        - import 只为纯旧循环设计，遇到歧义就拒绝。
        - 循环要么卡住，要么被迫重建基线，清零历史最佳和停滞计数。
        - R-4 要求的"可机械核对、跨对话恢复"对这个循环失效，而用户并不知情。
      - **投影冲突误报**：投影会写进 dev-plan 记录 `M<n>.md` 的 UI 修复节，而实施 agent 也在编辑这个文件。按整文件 hash 判冲突会频繁误报。
      - **漏评一个方案**：ADR-3 没有评估中间方案："以 Markdown（含结构化块）为唯一事实，加无状态的计算和校验命令"。
    - 证据：
      - [review-ui report-contract.md:85-141](../../skills/review-ui/references/report-contract.md)：规则全文。
      - [apply-ui-review record-contract.md:5-14](../../skills/apply-ui-review/references/record-contract.md)。
      - [AGENTS.md:94](../../AGENTS.md)：同步清单不含实现代码。
    - 修订：
      - ADR-3 补上中间方案，并做取舍。
      - 如果保留 JSON 方案：
        - 循环带模式标记。由工具管理的循环在 CLI 缺失时只读，并给出安装命令；或者允许人工推进一轮并标为 manual，同时定义重新接管的命令。
        - 合同里的数值参数和判定顺序做成带版本的机读规则表，用它驱动 Rust 黄金用例；测试钉住合同文件的 hash；AGENTS.md 的同步清单加入 review crate。
        - 投影冲突按"生成的那一节"判定，不按整文件。
      - 给行为评测"CLI 不可用"写出期望结果。
    - 复核：
      - 改稿后：ADR-3、§3.2、§5.3 有模式规则和同步机制。
      - 实现后：V-7/V-9 加入"机器 → 无 CLI → 机器"序列，结果符合规则，没有静默覆盖。
    - 置信度：中高。阻塞：M5。

21. [P2] F-21 · 分数的数值语义没有定义，二进制浮点会翻转门槛和平台期的边界判定
    - 定位：`phoenix-ui.md:338-341`、`:409`（门槛为"0–10的有限数值"）、`:488-495`（§9.2 的边界用例）。关联 C-4、V-8、M5。
    - 问题：
      - 评审可以给小数分，而 IEEE-754 double 会出现以下误差：
        - 2.4+2.3+2.6+0.7 算得 7.999999999999999。一个四维总和本应正好等于门槛 8 的单元，会被判为"需要修改"。
        - 历史最佳从 3.1 提高到 4.1，算得的提高量是 0.9999999999999996。这会误发平台期预警，进而误判重构或停滞。
      - §9.2 的边界用例（8.0、8.5、6.0→7.0）都是二进制能精确表示的数，测不出这个问题。
    - 证据：[judge.md:29](../../skills/review-ui/references/judge.md) 写明"逐个单元打分，可以给小数"。主 agent 用 `node -e` 实测，输出 `7.999999999999999` 和 `0.9999999999999996`。JS 与 Rust 的 f64 都是 IEEE-754 double。
    - 修订：
      - §5.3 规定定点十进制表示，例如以 0.01 为单位存成整数、限定输入步长、按十进制解析；所有比较都在整数上做。
      - §9.2 增加二进制不能精确表示的边界用例，例如 3.1→4.1、四维之和恰好等于门槛。
    - 复核：
      - 改稿后：§5.3 有数值表示规则。
      - 实现后：上述用例的判定结果与合同一致。
    - 置信度：数值事实高；"实现会用 f64"是推断。阻塞：M5。

22. [P2] F-22 · 机器协议在改写 review-ui 合同时，丢失或遗漏了几处细节
    - 定位：`phoenix-ui.md:341-342`（§4.2 平台期、顽固差距）、`:493`、`:496`（§9.2 对应的反例）、`:397` 与 `:407`（capture add 的拒绝条件与"补采追加"）、`:411`（judge 输入字段）、`:417`（评审材料白名单）、`:405`（指纹的附件范围）。关联 C-4、V-7、V-8、M5。
    - 问题：
      - **(a) 全局平台期**：合同的首要条件是"本轮分数低于门槛"。计划写成"历史最佳……提高不足 1.0 且未达门槛"，可以读成用历史最佳去比门槛。
        - 反例：门槛 8；R1 = 8.2（有未关闭 P1，未通过）、R2 = 7.5、R3 = 7.6。按合同，R3 应发预警；按计划的文字不发，因为历史最佳 8.2 已达门槛。
      - **(b) 顽固差距**：合同只针对 P0–P2，计划没有这个限定，P3 差距也会触发重构轮。
      - **(c) 同格补采**：两条规则在同一操作上互斥。
        - judge.md 规定，"看不出的格子"要在同轮补采并重评，取两次中较低的分。计划 L407 也写了"补采追加而非覆写原图"。
        - 但 L397 对"重复 key 不同内容"一律拒绝。cellKey 不含尝试序号，现有截图命名也是一格只能放一张。
      - **(d) 评审材料与 judge 输入**：
        - 白名单漏了 judge.md 要求的"目标包自那一轮以来有变更时，附上变更记录"，以及可选的差分图。
        - judge 输入 schema 漏了"建议"，它是合同中 judge.md 三个组成部分之一。
      - **另（不阻塞）**：L405 的"所有本地目标附件"沿用了合同"其中链接的每个目标文件"的措辞。如果规范要点里链接了 DESIGN.md，它算不算附件没有写明。建议在 v1 规格里写死附件集合。
    - 证据（主 agent 逐条复核）：
      - [report-contract.md:107-108](../../skills/review-ui/references/report-contract.md)：平台期第一条为"本轮分数低于门槛"；顽固差距为"某条 P0–P2 差距"。同文件 `:29-33`：judge.md 含"建议"。
      - [judge.md:11](../../skills/review-ui/references/judge.md)：附变更记录；`:75`：补采与取较低分。
      - [target-and-capture.md:63](../../skills/review-ui/references/target-and-capture.md)：指纹定义；`:84`：截图命名。
    - 修订：
      - §4.2 的两条按合同原文重述，并把上面两个反例加入 §9.2。
      - capture 的身份加入尝试序号（cellKey + attempt），定义补采文件的后缀，以及 judge-pack 取哪一次；拒绝条件改为"同一 cellKey + attempt 下内容不同"。
      - 白名单加入"该单元上一轮以来的变更记录"和可选差分图。
      - judge.json 增加 suggestions 字段，或者规定建议原文直通。
    - 复核：
      - 改稿后：逐字对照合同。
      - 实现后：V-7/V-8 覆盖以下用例：平台期反例、P3 顽固差距反例、同格补采、missing 后补采、重复提交的幂等性，以及"必需材料齐全"的断言。
    - 置信度：中高。阻塞：M5。

23. [P2] F-23 · 0.1.11 基线的已知缺陷没有预先登记，live 走查也没有通过标准和浏览器范围
    - 定位：`phoenix-ui.md:427`（§6：IME 等只列为走查主题，"阻断主路径"才吸收补丁，留到 M1 判断）、`:475`（V-5 只写"Chrome页面与中文IME/键盘走查"）、`:540`、`:362`（windows-x64 是支持平台）。关联 R-2、V-3、V-5、M1、M3。
    - 问题：
      - **IME 回车误提交**：承接版本的 live 页，至少在批注输入和配置输入两处按 Enter 就直接提交，不判断输入法是否正在组合。中文用户用回车选词时，会把半截指令提交出去。
      - **只测 Chrome 不够**：0.1.12 的修复特别注明，Safari 在 compositionend 之后才触发 keydown。只判 `isComposing` 的补丁在 Chrome 能过，在 Safari 照样误提交，而 live 是用系统默认浏览器打开的。
      - **Windows hook 命令**：0.1.12 还改写了 Codex 的 `commandWindows`。0.1.11 的写法是 `if exist "…" (… & exit /b)`，0.1.12 改为 `cmd /c if exist "…\…" …` 的反斜杠写法。本仓安装器写的仍是旧写法，而 windows-x64 支持需要这项修复。
      - **不应留到 M1**：这些证据现在已经拿得到，留到 M1 再判断"算不算阻断"，等于让实施者去做本该在计划期做的取舍。
      - **其他走查项没有期望结果**："服务退出""workspace 迁移""variant 接受/取消"都只列了主题。
    - 证据：
      - `U:skill/scripts/live-browser.js:866-869`、`:2459`：Enter 无条件提交。该文件与 engine-0.1.11 逐字节相同。
      - `E12:skill/scripts/live-browser.js:873-881`：`isImeKeydown` 及其 Safari 注释；同一修复还加在 `:2474`、`:2560`、`:11452`。
      - `U:crates/live/src/browser_open.rs:5-10`：用系统默认浏览器打开（delivery reviewer）。
      - `U:.codex/hooks.json` 与 `E12:.codex/hooks.json` 第 10、23 行的 `commandWindows` 对比（主 agent 复核）。
      - [bin/xgent-skills.js:131-134](../../bin/xgent-skills.js)。
    - 修订：
      - §6 和 §11 预先登记"必须吸收"的已知缺陷：IME（含 Safari 的 keyCode 229）和 Codex 的 Windows hook 命令。
      - 把 live 走查写成一张小表，列出场景、步骤、可观察的期望、浏览器（Chrome 加 Safari 或系统默认浏览器）、证据。至少覆盖以下场景：
        - IME 选词时不提交，确认后只提交一次。
        - 服务退出时，页面显示已断开及恢复办法，注入标签被移除。
        - 接受或取消变体后，源码里没有残留。
        - 生成失败时的部分结果。
        - 选择页缺图时显示文字卡片。
    - 复核：
      - 改稿后：§6 和 V-5 有这张表和已知缺陷清单。
      - 实现后：在 Chrome 和 Safari 上按表实测，留存截图或录屏；在 Windows 上确认 Codex hook 实际触发。
    - 置信度：高（来自源码对比，尚未实测）。阻塞：M1（已知缺陷登记）、M3（走查）。

24. [P2] F-24 · 设计类 skill 之间的路由与交接没有闭合
    - 定位：`phoenix-ui.md:423`（§6 主入口没有列 design-reference 和 ui-pattern-research）、`:134`（§0.4 中设计方向与实现由三方共有）、`:325-329`（§4.1）、`:388`（"用户已指定方向不重新抽取"）、`:511`（行为评测清单）。关联 R-3、V-9、M4。
    - 问题：
      - **路由重叠**：
        - "给订单页找几个视觉方向"会同时命中 design-reference 和 Phoenix new-work。后者按上游规则，在开工前必须运行 concept-seed，且"No substitute, no skip"。
        - "这个筛选交互怎么设计"会同时命中 Phoenix 的 `shape` 和 ui-pattern-research。
      - **已接受的方向会丢失**：design-reference 已接受的 brief 默认只留在对话里，保存时放在 `docs/design/<主题>-brief.md`；而 Phoenix 的 context 读的是自己的 surface brief 目录。换一次对话，用户已经接受的方向就可能被新的随机候选取代，与 L388 矛盾。
      - **Portal 初始化依赖没装的 skill**：B 阶段的 Portal 初始化沿用 xgent-init，但 xgent-init 是内部 skill，不在安装集合里。Portal 项目运行 Phoenix init 时如果没装 xgent-init 会怎样，没有定义。
      - **评测覆盖不到**：§9.3 的行为评测里没有这几类用例，误路由不会被发现。
    - 证据：
      - `U:skill/reference/new-work.md:48`："No substitute, no skip"。
      - `U:skill/reference/shape.md:1-3`；`U:crates/context/src/surface_briefs.rs:11-13`（delivery reviewer）。
      - [design-reference SKILL.md:13, 16](../../skills/design-reference/SKILL.md)：交互选型交给 ui-pattern-research；brief 的保存位置。
      - [xgent-init SKILL.md:3-4](../../skills/xgent-init/SKILL.md)：`metadata.internal: true`。
    - 修订：
      - §6 补全路由：找参考和视觉方向交给 design-reference，交互选型交给 ui-pattern-research，Phoenix 方向库只在没有既定方向时使用。
      - 规定已接受 brief 的存放位置，或由 context 去读取，使它被识别为"已指定方向"。
      - 规定 Portal init 在没装 xgent-init 时怎么做。
      - §9.3 加入上述三类用例。
    - 复核：
      - 改稿后：§6、§4.1、§9.3 有对应条目。
      - 实现后：V-9 在全新上下文重放这几类请求时路由正确，跨对话不会重新抽取方向。
    - 置信度：中。阻塞：M4。

25. [P3] F-25 · 承接 platform fallback 时漏了 Windows ARM64 回落到 x64
    - 定位：`phoenix-ui.md:362`。
    - 问题：上游 launcher 在 Windows ARM64 上找不到 arm64 资产时，会回落到 windows-x64。按计划"不支持的平台在预检失败"，现在能用的 Windows on ARM 将被拒装。
    - 证据：`U:skill/scripts/impeccable:142-145`（主 agent 复核）。
    - 修订：写明 win-arm64 映射到 win-x64（Windows on ARM 可以运行 x64 程序），或者明确放弃并登记为允许的差异。
    - 置信度：高。阻塞：否。

26. [P3] F-26 · 继承界面上的上游品牌没有列入改名清单
    - 定位：`phoenix-ui.md:179`（方向候选与选择页一行没有列品牌差异）、`:427`。关联 C-5。
    - 问题：
      - 选择页页头标志的读屏名称是"Impeccable"。delivery reviewer 还指出，serve-question 的 schema 说明要求推荐卡带上游品牌字样。
      - 用户会在 Phoenix 里看到别人的品牌，而 Apache-2.0 第 6 条不授予商标使用权。
    - 证据：`U:crates/context/src/question_page.rs:8`（`aria-label="Impeccable"`，主 agent 复核）；`vendor/impeccable/LICENSE` 第 6 条。
    - 修订：能力账本列出"用户可见的品牌面"（页面标题、标志、aria-label、卡片文案、帮助与安装输出），标为允许的差异，并用检索验证；来源归属继续保留在 NOTICE。
    - 置信度：高。阻塞：否。

27. [P3] F-27 · 自主维护之后，缺少漏洞发现、三方许可清单和补丁送达的机制
    - 定位：`phoenix-ui.md:109`（ADR-1 的代价）、`:545`（"按实际漏洞评估补丁"）、`:180`、`:267`（不自动升级、不读最新 manifest）、`:81`（C-5）。
    - 问题：
      - **漏洞来源**：fork 之后锁定的依赖约 209 条（architecture reviewer 统计），但没有漏洞通告的来源。上游 dependabot 只覆盖 bun 和 actions。
      - **三方许可**：再分发 binary 需要附带三方许可清单，NOTICE 没有覆盖 Rust 依赖。
      - **补丁送达**：修复了本机 live 或 question 服务的漏洞之后，已安装的项目会继续运行旧 binary，而且没有任何提示。
    - 修订：
      - CI 加 `cargo deny check advisories licenses`（或等价工具），生成三方许可清单并随包分发。
      - 写明补丁的通知渠道，例如发布说明、对有问题的版本执行 `npm deprecate`。
      - 可选：doctor 在本地比较项目 receipt 版本与当前包版本，不联网。
    - 置信度：中。阻塞：否。

28. [P3] F-28 · `npm test` 接入 Rust 后，会变成整个仓库的重门槛
    - 定位：`phoenix-ui.md:507`。关联 [AGENTS.md:80, 92](../../AGENTS.md)：修改 skill 描述后要求跑 `npm test`。
    - 问题：
      - 之后所有只改 skill 文档的工作，都需要 mise 和 Rust 1.99，并编译约 14 万行的 workspace。
      - 缺工具链时，如果只标"未运行"并以 0 退出，这道门对 Rust 部分就失效了。
      - package.json 的 test 脚本用了 POSIX 的 `for` 循环，在计划新增的 Windows CI 上会失败。
    - 修订：
      - 拆出单独的 `test:phoenix`，由 CI 和里程碑门强制执行；或者在 `CI=1` 时缺环境即失败，本地缺环境时明确 skip。
      - 同步更新 AGENTS.md 的测试说明。
    - 置信度：中。阻塞：否。

29. [P3] F-29 · 依赖图缺两处：M5 的 CLI 接线依赖 M2；V-5/V-9 用的固定 Web fixture 没有生产者
    - 定位：`phoenix-ui.md:525`（M5 前置）、`:477`（V-7 是公开 CLI 测试）、`:475`、`:479`（"本仓隔离真实项目""同一固定Web fixture"）。
    - 问题：
      - **M5 实际依赖 M2**：按字面，M5 可以和 M2 并行。但 `review` 子命令的注册，要与 M2 的 bin 更名、状态根修改一起改 cli 路由和 Cargo 元数据；V-7 也需要更名后的 CLI 才能跑。
      - **fixture 没人建**："固定 Web fixture""隔离真实项目"没有里程碑负责创建，M3 和 M6 难以保证用同一个 fixture 做比较。
    - 修订：
      - M5 前置写明：纯状态库只需要 M1 的 workspace；CLI 接线和 V-7 需要 M2 的 CLI 身份、状态根和 JSON/stderr 约定。
      - 由 M1（或 M3）创建 fixture 并固定路径，V-5 和 V-9 引用同一路径。
    - 置信度：中高。阻塞：否。

30. [P3] F-30 · 需求账本和 §11 混入了评审流程信息，评审结束后就会过期
    - 定位：`phoenix-ui.md:71`（R-7 写有"之后交 Opus review""计划提供独立评审所需的范围与证据"，映射到 V-10）、`:546`、`:552`（"供独立评审核查的重点"）。
    - 问题：
      - V-10 无法验证"交 Opus review"，M6"全需求映射有证据"对 R-7 的这一半无法满足。
      - L552 是写给计划评审者的，评审结束后即过期，实施者可能误以为这些问题仍待回答。
      - 这也与计划"不写写作过程"的约定不符。
    - 修订：R-7 只保留命名决定；等待评审的状态只留在 §11 对应的一行，并随本轮评审更新；删除 L552 这一段。
    - 置信度：高。阻塞：否。

## 2. 待核实与待决策

### 2.1 待用户决策

以下事项会改变分发、发布或安全边界，需要用户拍板，不能由计划作者或实施者代为决定。apply-doc-review 应把它们登记到 §11：有了决定就写入决定；没有就登记为待决阻塞，并写明最晚确认点。

| ID | 决策 | 影响的任务 / 问题 | 评审建议（供参考） | 最晚确认点 |
| --- | --- | --- | --- | --- |
| D-1 | A 阶段（M3）是否单独对外发布 | M3 退出范围、文档是否前移、默认切换提交的时机（F-17、F-18） | 不单独发布：默认切换与 M6 一起发布，M3 只在分支或本地验收。如需提前发布，就把文档和路由评测并入 M3 | M3 开工前 |
| D-2 | phoenix-ui 是否在 skills.sh 公开；npm 安装器下发哪些辅助 skill | 分发契约、F-04 的落盘形态、F-11 的冲突处理 | phoenix-ui 标为 internal，只经 npm 安装器获得，避免用户装到没有 engine 的 skill。辅助 skill 仍以 skills.sh 为主，安装器只认领内容相同的副本，并提示当前生效的是哪一份 | M2 bundle 定型前 |
| D-3 | 是否提供一个版本周期的 `impeccable` 过渡入口 | 迁移体验（F-09） | 提供一个只做跳转提示的 skill，一个主版本后移除 | M3 前 |
| D-4 | 推送验证分支与 CI、R2 凭据放在哪里、npm 发布身份：各由谁授权、谁负责 | V-3 能否执行、发布链路（F-13、F-14） | 授权推送专用的验证分支；CI 不持有 R2 凭据，发布者在本地按 run ID 交接；npm 启用 2FA 或可信发布 | CI 部分在 M2 退出前；发布凭据在 M3 演练前 |
| D-5 | live copy-edit 是否允许自动拉起非宿主的 agent CLI | NFR-4 的外发边界（F-06） | 关闭 `auto`，只在用户显式选择并确认外发对象后才调用 | M2 前 |

以下是设计层面的默认值，计划作者可以直接按评审建议写入，不需要用户拍板：迁移来源集合（F-07）、数值表示（F-21）、模式切换语义（F-20）、指纹附件集合（F-22）。持久化等级见下表第 7 项。

### 2.2 待核实

| # | 缺少的证据 | 影响 | 谁可提供 | 最晚确认点 |
| --- | --- | --- | --- | --- |
| 1 | 承接的 workspace 在 Rust 1.99.0 下 build、test、oracle 的通过情况，以及 clippy 基线 | ADR-1 的可行性、M2 范围、F-02 选哪种方案 | 实施者（M1 实测） | M1 退出 |
| 2 | mise 设置 `RUSTUP_TOOLCHAIN` 时，rustup 是否完全不处理 toolchain 文件里的 targets 和 components；wasm-pack 是否会自行执行 `rustup target add`（即隐式联网） | F-01 的修订细节 | 实施者（M1 在隔离环境实测） | M1 |
| 3 | Cursor 的编辑前 hook 失败时是放行还是阻止编辑；Codex 中个人级与项目级同名 skill 谁优先；各 provider 在 Windows 上用什么 shell 执行 hook，Codex hooks 在 Windows 上是否可用 | F-10/F-11 的严重度、支持矩阵怎么写 | 实施者（查官方文档或用 fixture 实测） | M3 设计冻结前 |
| 4 | GitHub 托管的 runner 能否为 darwin-x64 和 linux-arm64 提供 V-3 要求的"原生/对应环境" smoke | F-13、V-3 能否执行 | 实施者与仓库管理员 | M2 退出 |
| 5 | 还停留在 0.3.0（4.3.1）的用户有多少；用上游 global 或插件方式自行安装的比例 | 只影响 F-07 的优先级 | 维护者 | M3 前 |
| 6 | skills.sh 拉取的是整个仓库还是单个 skill；导入约 27MB 上游源码后体积影响有多大 | ADR-2 的代价 | 维护者 | M1 导入前 |
| 7 | 持久化等级：只保证进程崩溃后一致，还是保证掉电后也一致（后者需要对文件和目录 fsync，macOS 上要用 F_FULLFSYNC） | NFR-1 的实现机制 | 计划作者决定 | M3/M5 设计冻结前 |

## 3. 覆盖摘要

- **基线**：计划 `sha256:23306e15…e83d`（未跟踪）；代码 `b2289fa`，工作树 dirty，只有本计划和台账未提交；日期 2026-10-08。评审结束时复核，计划 hash 与 HEAD 均未变化。
- **实际分工**：
  - 4 个 general-purpose subagent 并行、各自独立初审，全部只读，模型继承主 agent：
    - facts：行为级核对计划主张与上游/本仓源码。
    - architecture：三个 ADR、构建链、安装事务、发布、协议可行性。
    - schedule：映射、依赖、提交点；逐字比对实施协议与骨架。
    - delivery：8 条用户旅程、UI 证据等级。
  - 主 agent：
    - 按固定 commit 拉取上游三个 commit 的只读副本。
    - 跑 dev-plan 校验脚本，整理需求账本映射。
    - 逐条复核被采纳问题的关键证据，合并 42 条候选。
    - 将 1 条降级：目标指纹的"非 v1 后果"已由 L315/L346 部分覆盖，只保留附件范围歧义，作为 F-22 的不阻塞补充。
    - 没有因误报撤回的条目。
- **四个视角的独立结论一致**：需要修改，没有 P0/P1。
- **计划中已核实成立的关键主张**：
  - VERSION.json 为 skill 4.5.0 / engine 0.1.11，R2 只登记了 darwin-arm64。
  - 三个上游 commit 与 tag 对应。
  - `crates/` 和 `browser-bundle/` 在 skill 4.5.0 与 engine 0.1.11 之间逐字一致（主 agent 用 `diff -rq` 复核）。
  - §1.1 关于安装器和两份脚本的行为描述。
  - CLI-CONTRACT 写的是旧口径，而 main.rs 已经路由 live。
  - 19 份 provider 产物；五平台与上游 release 矩阵一致。
  - 本机有 Rust 1.99.0 和 mbx 1.21.1。主 agent 运行 `mise exec rust@1.99.0 -- mbx doctor`，结果为"0 failures, 0 warnings"，managed targets 已启用。
  - 计划中 25 个非 ★ 的已有路径全部存在。
  - §4.2 的数字和判定顺序正确，只有 F-22 所列的两处限定词例外。
  - 「实施须知」「实施者定位」「实施进度」三段协议与骨架逐字一致。
  - 恢复快照 0/6、完成记录占位、提交点、每行退出条件结尾的「回写『实施进度』」都正确。
- **校验脚本**（`skills/dev-plan/scripts/`）：
  - `check_plan.py`：0 错误、0 警告。
  - `check_progress.sh`：0 错误。唯一的警告是工作树里有未提交的计划和台账，不属于里程碑进展。
  - `check_paths.sh`：报告 5 个 MISSING，逐项人工核对后都不是问题：
    - `./target/release` 是反例写法。
    - `docs/project-status/protocol.md` 是协议里的兼容写法。
    - `scripts/build-phoenix-ui.mjs`、`tools/phoenix-ui/compatibility.md`、`vendor/phoenix-ui/VERSION.json` 都是 ★ 拟新增文件。
  - 脚本通过不证明行为真实。
- **实测与外部查证**：
  - 主 agent：
    - mbx doctor。
    - 检查 mise 设置的环境变量和 rustlib 下已安装的 target。
    - 用 `diff` 和 `cmp` 对比上游各版本。
    - 用 node 验证浮点行为。
    - `npm view` 查各版本发布时间与 gitHead。
    - 查阅 Claude Code 的 skills 与 settings 文档（2026-10-08）。
  - reviewer：
    - architecture：在上游副本上跑 rustfmt；解码页内 wasm；查阅 mise、rustup、Cargo、R2、GitHub 的官方文档。
    - delivery：解包 0.3.0 tarball，只写入 scratchpad。
    - facts / schedule：用 `git ls-remote` 核对上游 tag。
  - 全程没有运行构建、安装、迁移、发布或任何会改动仓库的命令。
- **UI 证据等级**：只有文档推演、上游源码阅读和版本对比。目前没有可运行的 Phoenix 页面，也没有启动上游的 live 或选择页，**没有在浏览器里验证**。IME 的结论来自源码，不是实测结果。

## 4. 映射与实施编排

### 4.1 映射核对

**核对范围**：§0.1 的全部 17 行（R-1–R-8、NFR-1–4、C-1–5）→ §1–§8 的设计落点 → §9.1 的 V-1–V-10 → §10 的 M1–M6。在 ID 层面，每条需求都有设计落点和检查 ID，没有悬空引用。

账本里有几处单向映射：NFR-2 → V-1/V-3、NFR-1 → V-8、C-3 → V-4，但这些 V 行的需求列没有回写对应 ID。仍可追溯，不单列为问题。

**归属与生产缺口**（都已在 §1 中处理）：

| 缺口 | 现状 | 处理 |
| --- | --- | --- |
| 旧安装认领元数据和迁移 fixture；4.3.1 与改写后的 hook 形态 | 是 M3 的前置，但没有生产者 | F-07 |
| §1.3 的接口同步落点；两份 goal-guard 文档 | 没有负责里程碑 / 漏列 | F-16 |
| 发布脚本与发布排练 | 没有负责里程碑，也没有检查 | F-14 |
| V-4 在最终包上复验；V-7 在 M6 的真实取证 | M6 的退出条件没有列 | F-18 |
| 渲染器和开发安装方式 → M4 的 V-9 子集 | 没有列为前置 | F-04 |
| 固定 Web fixture | 没有生产者 | F-29 |
| CI 验证分支与推送授权 | 不在前置里，§11 也没有 | F-13 |
| R-7 中"交 Opus review"这一半 | V-10 无法验证 | F-30 |

### 4.2 依赖边调整

只列需要调整的边，其余沿用 §10。

| 前置 → 后置 | 解锁产物 / 证据 | 调整 | 关联 |
| --- | --- | --- | --- |
| 修订后的 §2.2 → M1 开工 | §2.2/V-1 中的工具链断言 | M1 开工前修订计划 | F-01–F-03 |
| M1 旧安装认领元数据与 fixture（新增）→ M3 开发 | fixture 的检查 ID | 加入 M1 退出条件 | F-07 |
| M2 渲染器与开发安装方式 → M4 的 V-9 子集 | V-3 生成器组 / V-4 子组 | 写入 M4 前置 | F-04 |
| M2 的 CLI 身份与状态根（V-2 CLI 组）→ M5 的 CLI 接线与 V-7 | V-2 对应组 | 写入 M5 前置 | F-29 |
| 验证分支提交加用户授权推送 → V-3（M2 只构建矩阵、M3、M6） | CI run SHA 等于提交树 | 新增流程，在 §11 登记 | F-13 |
| M3 提交（或 B 阶段在独立 worktree 进行）→ M4/M5 修改共享路径 | M3 的 SHA | 写入 §10 | F-15 |
| 用户决定 D-1 → M3 退出范围 | §11 的记录 | 新增 | F-17 |
| 发布排练（用替身）→ 实际发布 | 三类拒绝用例 | 新增 | F-14 |
| 数值表示、指纹规格、模式规则冻结 → M5 实现 | §5.3 修订 | 写入 M5 前置 | F-20–F-22 |

原有的环"M3 收口 ↔ M3 提交"由验证分支流程解开。主要瓶颈链是 M1 → M2 → M3（受外部 CI 与真实会话制约）→ M6；M4、M5 不在这条链上。计划没有工期估算，本评审也不计算关键工期。

### 4.3 并行波次与汇合验收

| 波次 / 任务 | 前置及解锁证据 | 可同时做的工作 | 写入边界 / 共享负责人 | 运行资源约束 | 汇合验收 |
| --- | --- | --- | --- | --- | --- |
| W1 = M1 | 固定源码；按 F-01–F-03 修订后的 §2.2；F-23 已知缺陷已登记 | 子任务：旧安装认领元数据与迁移 fixture（F-07）；固定 Web fixture（F-29） | `tools/phoenix-ui/**`、UPSTREAM.json、compatibility.md、mise.toml、rust-toolchain.toml、package.json 的 test 脚本归主负责人；各 fixture 目录归各自子任务 | 隔离 HOME 和旧 cache；原版 binary 不指向用户项目 | V-1、迁移 fixture 检查、fmt/clippy 基线 |
| W2 = M2 | V-1 与能力账本；D-2、D-5 已决定 | provider fixture；M5 的纯状态库（只放在独立 worktree，或不进 workspace members 的目录） | cli 路由、Cargo.toml/lock、渲染器与生成器入口、`skills/phoenix-ui`、catalog 归主负责人 | 网络观测 harness 独占端口；CI 需要验证分支和推送授权（D-4） | V-2 局部、V-3 生成器、CI 只构建矩阵 |
| W3 = M3 | 开发：冻结的 manifest/schema 与 M1 迁移 fixture；收口：V-2 全量；D-1、D-3 已决定 | B 阶段只在独立 worktree 或非共享路径进行 | `bin/*`、package.json、README A 段、F-16 的 M3 落点、A 阶段树生成的 bundle 归主负责人 | CI 用验证分支；真实会话和浏览器串行；同一项目的迁移串行 | V-2/3/4/5/10 通过且 CI SHA 等于提交树，然后 M3 提交 |
| W4a = M4 | M2 的 context/状态契约；共享路径改动在 M3 提交（或 rebase）之后；V-9 子集需要渲染器和安装方式 | W4b 中不涉及 context 的部分 | context、profiles、catalog 适配、3 个 design skill、xgent-init 其余部分归 M4；`skills/phoenix-ui` 由主负责人串行合并 | 用全新会话重放路由 | V-6、V-9 子集 |
| W4b = M5 | 纯状态部分需要 M1 workspace；CLI 接线需要 M2；context 集成需要 V-6；F-20–F-22 已冻结 | W4a | `crates/review`、两个 UI skill、两份 openai.yaml、sdlc/what-next/scenarios 归 M5；Cargo.lock 和路由注册归主负责人 | 同一循环的写入串行 | V-7 局部、V-8 |
| W5 = M6 | M3 的 SHA；V-6、V-7、V-8；M4/M5 的同步落点完成 | `docs/phoenix-ui.md`、发布排练、候选包安装 | 最终 bundle 与 VERSION.json、README B 段归主负责人 | 候选包、浏览器和评审会话串行 | V-9、V-10、V-4（最终包）、V-7 真实取证、受影响的 V-2/3/5/6/8、发布排练，然后最终提交 |

## 5. 总体结论与修订顺序

**是否需要修改：需要**

- **建议状态：Blocked**。计划自报 Proposed，两者不一致。
- **理由**：
  1. **有未决的用户决定**：D-1 A 阶段是否发布、D-2 phoenix-ui 是否公开到 skills.sh、D-4 推送与发布授权、D-5 copy-edit 是否外发。它们会改变外部分发契约、发布契约和安全边界，计划却在 L549–L550 写着"无未决的重大产品/数据决策""无必须先决定才能开始的产品问题"。
  2. **关键落地契约有已证实的缺口**：
     - M1 自身的构建契约照写执行会失败（F-01–F-03）。
     - 回滚在常见状态下不可用（F-12），迁移的认领范围也不完整（F-07、F-08）。
     - 数据外发边界有漏项（F-06）。
     - 机器协议与合同不一致（F-20–F-22）。

     这些缺口牵涉用户配置数据的安全、NFR-4 的安全与隐私，以及分发和发布这两项外部契约。按 dev-plan 的定级规则，计划不能停在 Proposed。
  3. **技术路线成立**：没有 P0/P1。从固定源码分叉、同仓 Rust workspace、mise/mbx、本地方向库这几项选择本身是成立的，问题集中在落地契约和收口门上，修订后不需要重新立项。
- **阻塞范围**：
  - M1 可以在修订 F-01–F-03、登记 F-23 的已知缺陷、加入 F-07 的 fixture 产出之后开工，不依赖 D-1–D-5。
  - M2 前要修订 F-04–F-06，并确定 D-2、D-5；M2 退出前还要确定 D-4 中的 CI 部分。
  - M3 前要修订 F-07–F-19（其中 F-07 的认领元数据和 fixture 已在 M1 产出），并确定 D-1、D-3 以及 D-4 中的发布凭据部分。
  - M4 前要修订 F-24。
  - M5 前要修订 F-20–F-22。
- **解除阻塞的可观察条件**：经 apply-doc-review 处置后，计划中能看到以下内容，满足后可以判 Ready：
  - §2.2、V-1、V-2 写明修订后的工具链和检查门。
  - §3.1 有"来源 × 范围 × 对象类型"的迁移表、条目级所有权和写入顺序、engine 可用性合同、冲突分级。
  - §5.1 有间接外发一行。
  - §10 有验证分支流程、阶段隔离方式、同步落点归属，以及修订后的 M3、M6 退出条件。
  - §5.3 有数值表示、模式规则，并与合同对齐。
  - §11 登记了 D-1–D-5 的用户决定；没有决定的，登记为待决阻塞并写明最晚确认点。
- **修订顺序**：
  1. 先纠正前提和契约：F-01、F-02、F-03（M1 构建契约），F-04（skill 源形态），F-05、F-06（方向库行为与外发边界），F-23（已知缺陷登记）。同时请用户回答 D-1–D-5。
  2. 再修安装与迁移设计：F-07、F-08、F-12、F-10、F-11、F-09，以及验收方法 F-19。
  3. 再修发布与 CI：F-13、F-14。
  4. 再修映射与编排：F-15、F-16、F-17、F-18、F-29。
  5. 再修 review 协议 F-20、F-21、F-22，以及设计路由 F-24。
  6. P3 中的 F-25–F-28、F-30，可以在对应里程碑之前顺手处理。
- **说明**：评审结论不代表功能已实现，也不代表可以发布。本评审没有执行任何构建、安装、浏览器或真实会话验证。

**下一步**（核实并修订计划，处置后判定 Blocked 或 Ready）：

```text
/apply-doc-review docs/plan/phoenix-ui.review.md
```

## 修订处置与复核

- 日期：2026-10-08；模式：修订并复核，覆盖 F-01–F-30、§2 的决策/待核实项、§4 的映射与依赖缺口。此前没有处置记录。
- 来源：本报告追加前 `sha256:cbd85a98e7e5f24e02753af4c33c74fb17e4ca12e129842efd774003ce08d723`。上方原问题、证据、结论及所审基线原样保留；本节不是新一轮独立评审。
- 目标：[Phoenix UI 开发计划](phoenix-ui.md)。修前 `sha256:23306e15b8f006f45eca841e89302c23f7cfddadf87cff094cb7d908485ee83d`，与所审版本一致；修后 `sha256:adbb709ebec69be725e810cd3a8f55af9e10344e7e3890a80617591ec6ca8151`。
- 代码基线：本地 main / `b2289fa05f1e4f0251bc3a86e3c04f182aba156e`，dirty；开始时计划、评审和既有 SDLC 登记均未提交。仅修改目标计划、追加本节和接续状态登记，未修改实现、提交或推送；当前内容仅本工作树可复现，未核实其他机器。
- 核实方式：亲读本仓安装器、发布脚本、技能合同、相关模板及固定 U/E12 源码（缩写和 commit 同本报告头），查询表中所引官方文档，再对照修后契约、依赖和退出门。未重跑原评审的 Rust/lint/WASM 实验；原报告中的实测数字不当成本次实测。

### 逐项处置

以下“已解决”均指文档缺陷及本次改稿复核；V/M 列是后续实现验收要求，不是已经运行的结果。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果与剩余实施验证 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。mise 的环境覆盖、rustup 优先级与 Cargo 从 cwd 查配置的规则，均与原命令前提不符；U:.cargo/config.toml 定义 xtask alias。 | 已修订并复核 | §2.2 明列 components/targets、固定 cwd、准备/构建分离、执行断言；先 bundle 再编 native；§9/§11。 | 已解决。配置与命令顺序一致；实际 mbx/离线子进程证明属于 M1/V-1。 |
| F-02 | 成立。原计划对全部承接源直接加严格 lint 门；U:.github/workflows/ci.yml 未提供相同基线。 | 已修订并复核 | §2.2/§9.1/§10：M1 登记 inherited fmt/clippy 基线，新增 crate 严格、修改不新增诊断。 | 已解决。未采用全仓格式清理；本次未重测原报告的告警数量，M1 需落盘可执行对比。 |
| F-03 | 成立。U:crates/xtask/src/main.rs 的字节比较与 U CI 不强求 WASM 字节一致的做法不同，不能用原门证明跨环境一致。 | 已修订并复核 | §2.2/§5.1/§8/V-2：输入摘要、生成文件完整性、两目录路径检查、native/WASM 同向量。 | 已解决。明确排除绝对路径与工具链差异误报，重建顺序闭合；实际向量/路径测试在 M2。 |
| F-04 | 成立。U:skill/SKILL.src.md 与 scripts/lib/utils.js 是模板/渲染入口，不能直接当安装成品；本仓 skills/ 必须是唯一源。 | 已修订并复核 | §1.3/§4.3/§5.1：源置 skills/phoenix-ui/src，根为通用渲染，tools 承接 Bun transformer，subagents 映射 provider，metadata/internal 及占位符门。 | 已解决。未复制第二份可编辑源；模板扫描与成品扫描区分，M2/V-3 验证 19 份产物及 tarball。 |
| F-05 | 成立。U:concept_seed.rs、catalog.rs、roll_selection.rs 的远端回落、tier 强制、卡图与 BLAND_FORM_RE 行为不满足本地库目标。 | 已修订并复核 | §1.2/§5.2：保留描述字段，改校验/筛选/失败语义；非法返回 2、不足返回 0+insufficient、缺图用文字；V-2/V-6。 | 已解决。四 mode、单 tier、无图、operate/Portal 与非法目录均有可观察验收；M2/M4 实测。 |
| F-06 | 成立。U:copy_edit_agent.rs 会选择登录的第二个 CLI；concept-seed 卡图、help、字体、模板命令均是间接外发面。 | 已修订并复核 | §0.2/§5.1/§11：沿用 NFR-4，禁止二次 CLI 外发，文案请求交当前宿主；本地 help/默认字体；环境变量清单与 HAR/子进程门。 | 已解决。未扩大数据权限；D-5 见下表。仅进程 deny 测试不再被当成全部证明，M2/V-2 实测。 |
| F-07 | 成立。Git 中 f1e2554 的 VERSION.json 为 4.3.1/0.1.5，当前本仓安装器还改写 guardedHookCommand、commandWindows 和 Codex 路径。 | 已修订并复核 | §3.1 来源×范围×对象表；M1 生产 migration-sources/fixture，M3 消费；个人/插件只读发现并阻止活跃冲突。 | 已解决。旧来源不再只认当前原 bundle；发布版本矩阵与真实安装样本由 M1/V-1、M3/V-4/V-5 验证。 |
| F-08 | 成立。U:crates/context/src/pin.rs、live_inject.rs/live_wrap.rs 有 pin、注入恢复与 variants 派生物。 | 已修订并复核 | §3.1/§6/V-4：按 marker+模板认领 pin；live 残留先停止/移除/处理变体，再 dry-run。 | 已解决。不得以删源码标记冒充恢复；M3 验中断与用户改写。 |
| F-09 | 成立。迁移成功还需实际生效入口、provider 重载/信任及可复制回滚指引；createAgents 保留已有文档。 | 已修订并复核 | §3.1/§6/§8：事务/备份/rollback、命令对照、加载核查、旧文档仅提示；D-3 过渡 skill。 | 已解决。输出与支持矩阵有归属；真实迁移/重载/回滚链在 M3/V-5。 |
| F-10 | 成立。U launcher 的 exit 127 与安装时跳过 engine 不能定义新 hook/CLI 的缺失语义。 | 已修订并复核 | §2.3/§8：hook 放行 0、CLI unavailable/4、Node bootstrap cache-only 补装、固定包/receipt、离线 release-dir、平台预检。 | 已解决。缺 engine 不依赖 Rust 自救；缺 launcher 也须有守卫提示，M2/V-3、M3/V-5 验证。 |
| F-11 | 成立。本仓删除/复制旧流程不足以处理外部管理的副本；Claude 官方同名规则确认个人级优先。 | 已修订并复核 | §5.1 冲突表、§3.1/V-4/V-5/V-9：相同内容记录认领边界；不同副本保留、返回 3 并标融合未就绪，给来源和切换路径。 | 已解决。symlink 不取得目标写权；其他 provider 优先级明确待实会话，未冒称已验证。 |
| F-12 | 成立。bin/xgent-skills.js 先装 statusline/guard，共享 JSON 中还有其他所有者；整文件 hash 会拒绝无关后续编辑。 | 已修订并复核 | §3.1：全量只读预检→Node 独立操作→Rust 重读加锁；共享条目级逆补丁/局部文本编辑；V-4。 | 已解决。中断时 Node 已完成部分独立保留，不能报整体成功；allow/guard 修改后 rollback 有明确期望。 |
| F-13 | 成立。CI 必须先获得候选提交，原“验收后提交”不足；官方 runner 矩阵提供所需标签。 | 已修订并复核 | §10/§11：候选提交与完成提交区分、候选可审阅后取得推送授权、run/head/tree 记录、五平台与 WinARM smoke。 | 已解决。不是预先授权推送；M2 退出前核验本仓 runner 权限及实跑，失败阻塞对应退出门。 |
| F-14 | 成立。scripts/publish-vendor-r2.mjs 的本地凭据与普通 s3 cp 缺少完整交接和原子拒绝覆盖契约。 | 已修订并复核 | §8/V-3/V-10/M6：CI 无凭据、固定 run 下载、摘要清单、R2 条件写、替身拒绝组、冻结 manifest 后同一 tarball 发布。 | 已解决。未采纳 HEAD+普通写的竞态方案；R2 条件支持有官方依据。演练/真实上传分别留门，不声称已发布。 |
| F-15 | 成立。U Cargo.toml 的 crates/* 与全量 bundle 生成会把同树 B 内容带进 A。 | 已修订并复核 | §8/§10：B 在独立 worktree/分支，A 单独 build/bundle/CI，主负责人独占共享集成点。 | 已解决。M3 收口后再集成 B 并复验；并行不再仅凭文件不冲突。 |
| F-16 | 成立。仓内 AGENTS 的接口同步规则、xgent-init 模板、goal guard 文档均超出原落点表。 | 已修订并复核 | §1.3 逐行分配 M1–M6；补齐模板、playbooks、evals、已有 agent 元数据、停用脚本；使用 review_type。 | 已解决。M3 先同步名称，M4/M5 补行为；历史记录保留。实际关联文件本次只读，V-10 逐项验收。 |
| F-17 | 成立。原计划“可独立使用”没有决定发布时机，默认切换存在提前发布风险。 | 已修订并复核 | §0.2/§8/§10/§11 落实 D-1：M3 仅分支内部验收，M6 统一发布；A 仍具文档/路由证据。 | 已解决。未保留单发 A 的条件分支；M3 不进入 main 日常 npm 发布。 |
| F-18 | 成立。原 V-7 声明 M6 真实取证，而 M6 退出门漏了它及最终迁移 V-4。 | 已修订并复核 | V-4/V-7/M6：最终 tarball 的旧版→最终包、A→B 升级/回滚、真实取证；列受影响 V-2/3/5/6/8。 | 已解决。A 的历史证据不能替代最终包必验项；仍全部未执行。 |
| F-19 | 成立。原 V-5/V-9 未把迁移后的实际加载、hook 次数和回滚旧工具纳入真实会话。 | 已修订并复核 | V-5/V-9、§9.3：两代旧包项目→迁移→Claude/Codex 重载/信任→设计→rollback；tarball 安装后闭环。 | 已解决。验证加载路径/hash、设计 hook 恰一次及 guard 保留，M3/M6 按变化复验。 |
| F-20 | 成立。report-contract 与 record-contract 定义人工合同，原计划未给两种模式及局部投影同步边界。 | 已修订并复核 | ADR-3/§3.2/§4.2/§5.3：评估 Markdown 结构化块方案；保留 JSON、manual/managed、缺 CLI 只读、显式 import；合同 hash 绑定规则表；生成节冲突。 | 已解决。managed 不静默转人工/清历史；AGENTS 同步归 M5，V-7/V-8 覆盖恢复与无关记录编辑。 |
| F-21 | 数值风险成立；尚无实现，不能断言已使用 f64。judge.md 允许小数，没有 0.01 步长约束。 | 已修订并复核 | §5.3/§9.2：十进制字符串、整数系数+scale、原 token 解析、精确边界与反例。 | 已解决。采用精确十进制，未采用额外量化限制；V-8 验总分 8 和 3.1→4.1 边界。 |
| F-22 | 成立。逐条对照四份 UI 合同确认：当前分数低于门槛、P0–P2、补采、变更记录/差分/suggestions 均需补齐。 | 已修订并复核 | §4.2/§5.3/V-7/V-8：attempt 身份、missing 后补采/幂等、冻结 packet、直接附件集合及整文件 hash。 | 已解决。反例与材料白名单对应；不递归追附件链接，历史封存不改写。 |
| F-23 | 成立。亲读 U/E12 的 live-browser.js、.codex/hooks.json 与本仓 hook 命令，确认 IME/Windows 修复差异；默认浏览器不限 Chrome。 | 已修订并复核 | §1.2/§6/§11/M1/M2：必须吸收最小补丁；Chrome/Safari/Windows、服务失败/退出/变体/移动 workspace 的期望表。 | 已解决。补丁列为必修而非待判断；M3/V-3/V-5 需真实证据，当前未运行浏览器。 |
| F-24 | 成立于路由与跨对话持久化缺口。U new-work 已尊重用户钉住的方向，不能误写成上游总会重抽。 | 已修订并复核 | §4.1/§5.2/§6/§9.3：三类设计入口分工、已接受 brief 原文映射与 hash、缺 xgent-init 的显式路径。 | 已解决。未授权保存的纯咨询不伪称持久化；M4/V-6/V-9 验新会话及缺 skill。 |
| F-25 | 成立。U launcher 有 Windows ARM64→x64 分支。 | 已修订并复核 | §2.3/§5.1/V-3/§10：映射 windows-x64，WinARM 对应环境 smoke。 | 已解决。保留五资产而非虚构第六资产；M2/M3 实测兼容运行。 |
| F-26 | 成立。U question_page.rs 的 aria 标签与 serve_question.rs 的卡面文案仍显示旧品牌。 | 已修订并复核 | §1.2/V-2/V-10：页面 title/aria/卡面/help 等用户可见文字替换，LICENSE/NOTICE 与历史 oracle 保留来源。 | 已解决。改名边界覆盖运行资源，不以全仓替换消除来源。 |
| F-27 | 成立。U CI/dependabot 的现状不能替代自主 fork 的漏洞、许可与送达机制。 | 已修订并复核 | §8/M2/M6：cargo-deny 公告/许可、数据库版本/时间、许可随包、修复版本与显式升级说明。 | 已解决。检查与处置有负责人/发布门；deprecate 等实际外部动作另授权。 |
| F-28 | 成立。package.json 当前总入口是 Node/Python 且含 POSIX 调度，直接加 Rust 会扩大普通文档维护门槛。 | 已修订并复核 | §9.3/§1.3/M1/V-10：轻量 npm test、跨平台 Node 调度、显式且强制的 test:phoenix；AGENTS 同步。 | 已解决。缺 Rust 不默默 skip Phoenix 门，本次未更改 package.json 或测试实现。 |
| F-29 | 成立。原 M5 CLI 需要 M2 的身份/状态契约，原 Web fixture 没有生产者。 | 已修订并复核 | §1.3/V-1/V-5/V-9/§10：M1 产 Web fixture，M2 产开发包；M5 分纯状态库与 CLI/context 接线依赖。 | 已解决。消费入口、前置和验收时点对应，不把 fixture 开发当真实集成通过。 |
| F-30 | 成立。原 R-7/§11 的评审安排会在本次处置后过期。 | 已修订并复核 | R-7 只留产品命名；§11 留最终契约、实施待验证与 Ready；评审历史保留在本报告。 | 已解决。移除等待指定评审的旧口径；实施协议和 0/6 完成记录不变。 |

### 决策与未完项

D-1–D-4 来自本次用户明确答复，均为 2026-10-08；没有把推荐选项或等待时间当成确认。D-5 是依原有 NFR-4 收敛实现，并非新增用户授权。

| 项 | 本次结论与依据 | 原文及验收落点 |
| --- | --- | --- |
| D-1 | 用户选择 M6 融合完成后统一发布，M3 只内部验收。 | §0.2/§8/§10/§11；M3 分支隔离，M6 终验。 |
| D-2 | 用户选择 Phoenix 仅随 npm 安装器分发，辅助 skill 公开；两渠道不同副本保留并提示切换。 | §5.1 的固定安装集合、internal 标记及冲突结果；V-3/V-4/V-9。 |
| D-3 | 用户选择保留一个迁移主版本的 impeccable 提示 skill，不调用旧工具。 | §3.1/§6；M3/V-4/V-5，首个迁移主版本后移除。 |
| D-4 | 用户选择 CI 无发布凭据、维护者本地交接发布，npm 启用 2FA。答复确认职责安排，未授权现在推送或发布。 | §8/§10/§11；具体候选 diff/SHA/ref/CI 或 run ID/摘要/演练可审阅后，再取得外部动作授权。 |
| D-5 | 原 NFR-4 已写“不上传代码、截图、规范或偏好”；不引入新的外发许可。移除 auto/codex/claude 二次 CLI 路径，文案请求交当前宿主。未采纳“显式确认后允许第二个 CLI”的扩展。 | §0.2/§5.1/§11；M2/V-2；将来若要开放，另行决定数据边界。 |

原 §2.2 的七项逐一处理如下。剩余实测均有负责人和最晚点，未被算成本次已验证；未发现必须先改变范围或架构才能开始 M1 的阻塞。

| 原序号 | 本次可确认部分与剩余动作 | 归属/最晚点 |
| --- | --- | --- |
| 1 | 尚未构建承接 workspace；已决定用 lint 基线+增量门，build/test/oracle 的兼容结果必须实测。 | 实施者 M1/V-1，失败阻止 M2 依赖项。 |
| 2 | [mise Rust](https://mise.jdx.dev/lang/rust.html)、[rustup 优先级](https://rust-lang.github.io/rustup/overrides.html)、[Cargo 配置](https://doc.rust-lang.org/cargo/reference/config.html) 已核对；显式组件/target 与 cwd 消除配置依赖。wasm-pack/mbx 的实际进程与联网行为仍待隔离实测。 | 实施者 M1/V-1。 |
| 3 | [Claude 同名 skill 优先级](https://code.claude.com/docs/en/skills#resolve-skills-that-share-a-name) 已核对；其余 provider 的失败放行、Windows shell、加载优先级不猜测。 | 实施者 M3 支持矩阵冻结及 V-5；缺环境阻塞相应退出门。 |
| 4 | [GitHub runner 矩阵](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) 有所列平台标签；本仓实际权限和对应架构 smoke 尚未运行。 | 主负责人/管理员 M2 退出前。 |
| 5 | 用户比例未知；迁移范围已包括两代本包来源及外部旧入口发现，不以占比缩减覆盖。 | M1 造 fixture；维护者 M3 前补用户反馈，非开工阻塞。 |
| 6 | skills.sh 实际拉取体积未测；计划保留源码同仓选择，导入前测量，不预填下载数字。 | 维护者 M1 导入前。 |
| 7 | 明确选择进程崩溃可恢复一致性，不承诺掉电零丢失；journal/同目录替换/备份和人工恢复边界已写明。 | §3.1/§7/§11；M3/V-4、M5/V-7 故障注入。 |

原 §4 映射和编排缺口分别由 F-01–F-06（构建/来源/行为）、F-07–F-14（迁移/发布）、F-15–F-19/F-29（隔离/接口/最终验收）、F-20–F-24（协议/用户旅程）覆盖；R-1–R-8、NFR-1–NFR-4、C-1–C-5、V-1–V-10、M1–M6 的稳定 ID 保留，没有增加实施完成数。

### 修订后结论与验证

- 处置统计：**30 项已修订并复核，复核结果均为文档问题已解决；0 项待修订、待核实或待决策的文档缺陷**。没有整项不采纳；F-21 不引入 0.01 量化、D-5 不开放额外 CLI 外发，理由见上表。这不改变首轮“需要修改”的历史结论，也不冒称完成独立复审。
- 当前计划判定：**Ready**。设计选择、分发和职责已明确，可从 M1 开始；实施仍为 **尚未开始，0/6**。候选推送、真实环境可用性及公开发布授权只在对应动作/退出门处约束，不预先伪造许可。
- 实际运行：`python3 skills/dev-plan/scripts/check_plan.py docs/plan/phoenix-ui.md`：0 error / 0 warning。`check_paths.sh` 扫到 9 个路径 token、4 个 MISSING：`docs/project-status/protocol.md` 是协议兼容旧路径，其余 build-phoenix-ui.mjs、compatibility.md、VERSION.json 都在正文标为计划新增，逐项人工核对后没有错误复用路径。该脚本不是全量路径证明。
- 实际运行：`check_progress.sh`：0/6、6 个里程碑、0 条完成记录、0 error / 1 warning。warning 源于计划仍为 untracked、工作树有未提交文档/台账，非遗漏完成记录；没有实施改动可计入。实施协议文字保留，恢复快照仅解除“待评审”的旧阻塞描述。
- 实际运行：`npm test` 退出 0；Node 74 项通过，Python 四组分别 21、23、2、13 项通过。本次只验证现有仓库回归；没有运行 Phoenix Rust 构建、oracle、跨平台 CI、安装迁移、浏览器/宿主真实会话或发布，不能从这些测试推导 V-1–V-10 通过。
- 关联同步：README、AGENTS、安装器、skills、构建/发布脚本和实际支持矩阵的修改均是 §1.3 已分配到里程碑的未来实施工作，本次不提前实施。接续已有 SDLC 的计划 readiness 与原评审 disposition，保留原评审 review 事件和未开始的 implementation 事实。

下一步（仅建议，本次未启动）：

```text
/goal 按 docs/plan/phoenix-ui.md 开发
```
