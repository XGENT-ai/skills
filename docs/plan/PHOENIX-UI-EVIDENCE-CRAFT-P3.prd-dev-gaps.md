# PHOENIX-UI-EVIDENCE-CRAFT-P3 · PRD 与开发交付差异核查

- 审查对象：[Phoenix UI 取证、中文排版与素材细节增强 PRD](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md) v1.3（`sha256:2591ffd2d61da4d8915ce504f8edc01d0d7518efbe2739706fbc9b23965dcdd9`，已提交；v1.2→v1.3 只改了 WORK-N1 的两处澄清与版本号，由本期 M5 按计划 §6.8 写入）· **P3（第 3 期）** × [开发计划 PHOENIX-UI-EVIDENCE-CRAFT-P3](PHOENIX-UI-EVIDENCE-CRAFT-P3.md)（`sha256:a51b6f93e52e9ee889add80847314ecadd00e840ea25b7824c29a127694d65a1`，已提交，5/5 里程碑完成，状态 Ready）
- 审查类型：首次完整 P3 核查（非复审；P1、P2 核查见 [P1 报告](PHOENIX-UI-EVIDENCE-CRAFT-P1.prd-dev-gaps.md)、[P2 报告](PHOENIX-UI-EVIDENCE-CRAFT-P2.prd-dev-gaps.md)，本报告独立编号）
- 审查日期：2026-11-12（+08:00）· 核查人：Agent（review-prd-dev-gaps）
- 代码基线：main `3094bbeda67dbdbb72806c1c82f27bc2fbb5a403`；工作树 dirty 项只有 SDLC 台账、R3 代码评审报告与本报告，引擎与 skill 源码无未提交改动。四平台 CI 候选为 `54b1d67`（run [43141743915](https://github.com/XGENT-ai/skills/actions/runs/43141743915)）；`54b1d67..HEAD` 的实现差异只有两次评审修复（`media/index.rs`、`media/look.rs`）与 `docs/phoenix-ui.md` 4 行。

## 1. 结论与范围

- 本期范围（PRD §7.1、§7.2，计划顶部「需求来源」）：WORK-F7/AC-1–AC-4、WORK-F8/AC-1–AC-4、WORK-F9/AC-1–AC-5、WORK-N1/AC-4（**WORK-N1 在本期终验**，AC-1–AC-3 属 P1、持续满足）、WORK-N2/AC-3（终验期 P4）、WORK-N4/AC-2（每期；终验期 P4）；UAT-9、UAT-10、UAT-11；另承接 P2 差距报告 D-01（typeset 断行指引一句）。
- 整体结论：**存在明确遗漏、存在合理变更、待补证并存；两项需决策的差异已由用户决定**。本期 13 条功能验收中 12 条已满足，1 条部分满足（G-01）；WORK-N1 的终验结论要等 G-06 的关闭尝试有了 CI 结果再下。
- 差异计数（按独立 G-ID）：**A 明确遗漏 1；B 合理变更 4；C 需 Triage 2（已决 2 / 待决 0）**；待核实 4；需跟进 3 项（G-01 补齐、G-06 关闭尝试、G-07 与 G-05 的 PRD 同步），均由 gaps plan 承接。
- 独立验证：本轮除核对完成记录外，用 HEAD 构建的 debug 引擎（`sha256:51d3d0f96a6324451be628129effe3caff54185b436c23ce312ce00f00ad9d2e`）在仓库外的临时项目里实际运行了 17 项离线检查（图片处理的 UAT-11 场景、裁切与裁边、素材区越界、断网检索、联系表与观感数据、取用与来源清单、交付前核对），全部通过，并打开了联系表与输出图片查看；运行了 media 与 prep-image 的 31 个单元测试（含本机 Chrome 上的 WebM 与 H.264 首帧），全部通过；静态核实了终端输出、信源许可过滤、浏览器启动参数、联网边界测试的残余清单与全部文档落点。真实信源检索、宿主会话与四平台 CI 沿用完成记录，本轮未重跑（见 §5）。
- 报告完成不等于产品交付完成：gaps plan 实施并验收之前，不宣称 P3 已完整交付。

## 2. 三类差异

### A · 明确遗漏

#### G-01 · 检索与预览的终端结果行不带许可文字 · A

- PRD 期望：[WORK-F7/AC-1 · PRD:324](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)「每条带编号、标题、信源、原图尺寸、许可与使用分级」；行为段（PRD:316）同。
- 实际交付：`--json` 的条目有 `license`、`licenseUrl`、`attribution`；不加 `--json` 时每条只有 `<编号>  <分级>  <宽>x<高>  <信源>  <标题>`。本轮实测一条 CC BY-SA 4.0 的条目输出为 `C1  可上线  4000x3000  commons  Carte marine 1650`，看不到许可名称，也看不出它要求署名与同许可共享。
- 证据链：计划 §5.12 把终端行定为这五个字段 → M1 交付 → [media/mod.rs:427-437](../../tools/phoenix-ui/crates/cli/src/media/mod.rs#L427)（本次静态核实）→ 本次实际运行 `media preview C1`（上例）。完成记录的 V-1、A1 都用 `--json` 验证，没有覆盖终端输出。
- 影响：直接在终端使用、或 agent 没加 `--json` 时，候选只有三档分级，CC0 与 CC BY-SA 无从区分；skill 只要求「展示候选时带分级」，所以给用户的候选表也可能不带许可。素材不进产品，后果限于设计阶段的信息不全，不阻塞主路径。
- 分类依据：验收条件明确，恢复不需要产品取舍（多一列文字）；计划的终端格式不是经确认的需求变更。
- 处置：补齐。
- Follow-up：F-01——终端的检索与预览结果行加上许可文字；验收为同一条目在终端行与 `--json` 的 `license` 一致。gaps plan 任务见 §6。

### B · 合理变更

#### G-02 · 浏览器自身的后台请求按清单登记，PRD 的 WORK-N1 已相应改写 · B（已确认变更）

- PRD 期望：v1.2 的 WORK-N1/AC-4「页面检测与取证命令只请求调用方指定的目标页面及该页面自身加载的资源」。
- 实际交付：引擎进程自身不发请求；浏览器启动参数新增 8 个开关与 15 个关闭特性（[cdp.rs `default_chrome_args`](../../tools/phoenix-ui/crates/browser/src/cdp.rs)）。macOS 上的 Google Chrome 157 已无残余请求；仍存在的浏览器自身请求列在 [docs/phoenix-ui.md「联网边界」](../phoenix-ui.md#联网边界)。PRD v1.3 的 AC-4 相应写为「浏览器自身仍发出的请求以 docs/phoenix-ui.md 列明的清单为限」。
- 证据链：计划 D15（用户 2026-11-11 确认）→ M2 的 V-3 与残余排查 → M5 的 V-9 A4、V-11 → `crates/cli/tests/network_boundary.rs` 的残余清单（macOS 为空，本次静态核实）→ PRD diff（本次核对：只改约束段一句、AC-4 一处限定与版本号）。
- 影响：联网边界从「零例外」变为「引擎零请求 + 浏览器残余如实列出」。
- 分类依据：约束已证实（Chrome 的后台请求不能全部由参数关闭）；口径变更有用户确认，PRD 已同步。
- 处置：保留。Linux 上后来发现的两项残余超出了确认时的事实，拆为 G-06 单独决定。
- Follow-up：无（G-06 另计）。

#### G-03 · 没有来源的图片不能处理 · B

- PRD 期望：[WORK-F9/AC-1、AC-3 · PRD:365、367](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)：给定显示宽度后得到 2 倍宽的输出；输出带有与原图相同的内嵌来源，`embed-prompt --scan` 不报缺失。PRD 没有说原图本身没有来源时怎么办。
- 实际交付：输入没有内嵌来源也没有旁注时，该文件失败并提示 `先嵌入来源再处理：phoenix-ui embed-prompt <图片> --prompt <来源>`，不产生输出，其余文件照常（本次实际验证）。
- 证据链：[计划评审 F-08](PHOENIX-UI-EVIDENCE-CRAFT-P3.review.md)（原设计会产出没有来源的图，`--scan` 随即退出 3）→ 计划 §5.13、§5.14 第 9 步 → M3 的 V-5 → `--help` 与 docs「交付图片处理」均写明。
- 影响：项目自有、还没写入来源的图片要先跑一次 `embed-prompt`；多一步，换来输出目录的 `--scan` 一定干净。
- 分类依据：两条验收在「原图没有来源」时互相冲突，拒绝是唯一让 AC-3 始终成立的做法；它执行的是 Phoenix 既有规则（上线图片都带来源），失败信息给出了可照做的命令；行为已实现并有测试与实测证据。
- 处置：保留。
- Follow-up：无。

#### G-04 · WebP 输出的来源写在旁注文件里，不在图片内部 · B

- PRD 期望：WORK-F9/AC-3「输出图带有与原图相同的内嵌来源」。
- 实际交付：不透明图输出 JPEG，来源写进 COM 段；带透明通道的图输出无损 WebP，来源写进 `<图片>.json` 旁注。`embed-prompt --read` 与 `--scan` 对两者都能读到（本次实际验证：6 个输出 `0 missing`，文字与原图逐字相同）。
- 证据链：计划 D12（输出格式，用户 2026-11-11 确认）、D13 → M3 → [embed_prompt.rs](../../tools/phoenix-ui/crates/context/src/embed_prompt.rs) 的 `write_provenance` → docs 写明「移动 WebP 时须同时移动旁注」。
- 影响：只移动 WebP 而不带旁注时来源丢失，`--scan` 会报出。
- 分类依据：旁注是 `embed-prompt` 对 WebP 的既有载体，本期没有另造；验收用的 `--scan` 判据成立；输出格式的取舍有用户确认。
- 处置：保留。
- Follow-up：无。

#### G-05 · 荷兰国立博物馆的条目没有限定为公有领域或 CC0 · B

- PRD 期望：[WORK-F7 行为段 · PRD:318](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)「许可分级沿用参考规则：……美术馆只取公有领域或 CC0」。
- 实际交付：大都会、芝加哥、克利夫兰三家在请求与映射里都限定了公有领域或 CC0；荷兰国立博物馆按记录里的许可链接分级——CC0 与公有领域标记为可上线，其他 Creative Commons 链接按其许可分级，记录没有许可信息时为需核对，条目不丢弃（[sources.rs:736-750](../../tools/phoenix-ui/crates/cli/src/media/sources.rs#L736)，本次静态核实）。录下的真实响应样本里只出现了公有领域标记与 CC0。
- 证据链：计划 §5.2 rijks 行 → M1 → 参考实现 `local/oil-ui-main/scripts/curate-media.mjs:398` 的同一写法（本次读取）。
- 影响：`--use all` 的结果里可能出现来自该馆的需核对条目；分级没有被抬高，`--use page` 仍只留可上线条目。
- 分类依据：PRD 同一句要求「沿用参考规则」，而参考规则对这一家正是如此，PRD 的概括与它所指的规则在这一点上不一致；实现保持了「条目的许可如实给出」的目标，素材也不进产品。
- 处置：保留；PRD 的这句概括可在下次修订时写准。
- Follow-up：文档同步一句（PRD:318 注明荷兰国立博物馆按条目许可分级），并入 F-03 的 PRD 同步。

### C · 需 Triage 的实现差异

#### G-06 · Linux 上的 Chrome 在页面检测与取证时仍发出组件更新检查等后台请求 · C（已决 → D-01）

- PRD 期望：[WORK-N1/AC-4 · PRD:557](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)（**本期终验**）：只请求目标页面及其资源，浏览器残余以 docs 清单为限；「上述命令运行中都不自动更新或下载」。计划 D15 向用户确认时写的效果是「页面检测与取证期间不再触发 Chrome 的组件更新下载、账号与搜索请求」。
- 实际交付：macOS 的 Chrome 157（arm64、x64）无残余。Linux x64 的 Google Chrome 156.0.8078.79 在 `detect <url>` 与 `capture` 的全部 4 个测试场景里都向 `update.googleapis.com` 发出 `POST /service/update2/json`（组件更新检查）；Linux arm64 的 Chromium 147.0.7727.0 在 `capture` 的 2 个场景里向 `android.clients.google.com` 发出 `POST /checkin`。两者已写进 docs 与测试的 Linux 残余清单，测试在清单内通过。
- 证据链：M5「V-11 联网边界排查」与 CI netlog（run 43141743915）→ [network_boundary.rs:19-22](../../tools/phoenix-ui/crates/cli/tests/network_boundary.rs#L19)（本次静态核实）→ [docs/phoenix-ui.md:200-207](../phoenix-ui.md#联网边界)（本次读取）。
- 影响与原因：受影响的是在 Linux 上运行页面检测、取证以及其他启动浏览器的命令。请求来自浏览器自身，不带项目信息。测试用的代理一律回 502，所以更新检查之后会不会真的下载组件**没有证据**，docs 也这样写。原因：Chrome 156 的组件更新器不理会 `--disable-component-update` 对这一路径的关闭；M5 的排查结论是「未找到可直接采用的新关闭开关」。
- 为什么需要决定：残余登记这条路线用户已确认（D15），PRD 字面条件也满足；但确认时的事实是 macOS 上只剩一个取密钥的请求，Linux 上出现的是**更新检查**，正好落在「不自动更新或下载」这条约束上，且 D15 写明的效果在 Linux 上没有达到。接受还是再关一次，是安全口径的取舍。
- 本轮补充的事实：两个版本的 Chromium 源码里各有一个把该请求改发到指定地址的开关——Chrome 156 的 `--component-updater=url-source=<地址>`（[component_updater_command_line_config_policy.cc](https://raw.githubusercontent.com/chromium/chromium/156.0.8078.79/components/component_updater/component_updater_command_line_config_policy.cc)），Chromium 147 的 `--gcm-checkin-url=<地址>`（[gservices_switches.cc](https://raw.githubusercontent.com/chromium/chromium/147.0.7727.0/google_apis/gcm/engine/gservices_switches.cc)）。引擎已经用同样的办法把账号请求改发到本机丢弃端口（`--gaia-url=http://127.0.0.1:9`）。这两个开关在 Linux 上是否真能消除残余**尚未验证**，只能在四平台 CI 上用现有的 netlog 诊断确认。
- 可行选项：见 §4 问题卡 Q1。
- 处置：**已决（D-01，2026-11-12 用户答复）**——再做一轮关闭尝试。
- Follow-up：F-02，见 §6。

#### G-07 · 视频取用依赖本机浏览器能解码该视频；linux-arm64 的 Chromium 取不了 · C（已决 → D-02）

- PRD 期望：[WORK-F8/AC-4 · PRD:348](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)「取用视频时同时得到首帧静态图」；[WORK-N2/AC-3 · PRD:574](../PRD-PHOENIX-UI-EVIDENCE-CRAFT.md)「素材检索、取用与图片处理命令在四个平台可用」。PRD §4 前置依赖表（PRD:631）没有把本机浏览器列为 WORK-F8 的依赖。
- 实际交付：首帧由本机无头浏览器解码取得；浏览器不可用、不支持该编码或超时时，这条视频取用失败，视频与首帧都不落位、不登记，退出码 2，原因写明。图片与贴图的取用不需要浏览器。
- 证据链：计划 D9、ADR-2、[计划评审 F-06](PHOENIX-UI-EVIDENCE-CRAFT-P3.review.md) → M2（真实 NASA 视频在本机取用成功并看过首帧）→ M5「H.264 平台证据」：darwin-arm64、darwin-x64、linux-x64 通过；linux-arm64 的 Chromium 147 报 `DEMUXER_ERROR_NO_SUPPORTED_STREAMS`，记为未验证、失败清理已验 → 本轮单元测试在本机 Chrome 上 WebM 与 H.264 首帧均通过 → docs:161 与 `--help` 写明依赖。
- 影响与原因：视频是 9 个类别中的 1 个，信源只有 NASA，文件都是 H.264 的 MP4。在不带 H.264 解码器的 Chromium 上（CI 的 linux-arm64 即是），或没有浏览器的机器上，视频类素材取不下来。原因：PRD D9 不引入 ffmpeg，本期也不新增第三方包，没有浏览器之外的解码手段。
- 为什么需要决定：这是 PRD 没有写的前置条件，并让「四个平台可用」在 linux-arm64 的视频取用上不成立；ADR-2 没有用户确认记录。
- 可行选项：见 §4 问题卡 Q2。
- 处置：**已决（D-02，2026-11-12 用户答复）**——接受现状，PRD 写明这一前置；不改代码。
- Follow-up：F-03，见 §6。

## 3. 完整需求覆盖账本

证据等级：**实测**＝本次实际验证；**静态**＝本次静态核实；**记录**＝完成记录里的运行或测试结果（未重跑）；**CI**＝run 43141743915。

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F7/AC-1 检索结果每条带编号、标题、信源、原图尺寸、许可与分级；编号跨次不变（PRD:324） | 本期 | §5.1–§5.6；M1 | [M2 验证](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M2.md#验证) V-1、V-2；[A1](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5-A1.md) | 真实信源 6 条地图带原图尺寸、重复检索编号相同（记录）；编号稳定与并发不重号（实测：单元测试）；`--json` 字段齐全、终端行缺许可（实测 + 静态） | 部分满足 | G-01 |
| WORK-F7/AC-2 联系表带编号、分级、原图尺寸，并给出四边主色、纸底、可压字区域（PRD:325） | 本期 | §5.7；M1 | M2 V-1；A1 | 本轮生成并打开联系表：每格缩略图、编号、分级色带（绿/橙）、原图尺寸；`look` 含 `edges`、`paper`、`calm`，浅灰图为纸底、带深色边框的图不是（实测） | 已满足 | — |
| WORK-F7/AC-3 只要可上线时不出现其他分级（PRD:326） | 本期 | §5.1、§5.4；M1 | M2 V-1③ | 真实检索加 `--use page` 全为可上线（记录）；单元测试覆盖（实测） | 已满足 | — |
| WORK-F7/AC-4 单个信源不可用时注明、其余照常；全部不可达时报网络不可用，不返回缓存（PRD:327） | 本期 | §5.12；M1 | M2 V-1⑦；A1；A4 | 代理指向丢弃端口：退出 1、「网络不可用：11 个信源均无法连接」、不建索引；已有索引时也不回放旧条目（实测）；LoC 403 时其余照常、退出 2（记录） | 已满足 | 待核实 V-1（LoC） |
| WORK-F7 行为：关键词与多组词、8 个类别加全部、11 个信源（PRD:314-315） | 本期；D6 测绘图纸加查 Commons（用户 2026-11-11 确认） | §5.2、§5.3；M1 | M1；M2 V-1⑧ | `--help` 列出 9 个类别值（实测）；11 个信源的映射各有样本测试（实测：单元测试；LoC 样本为自写） | 已满足 | 待核实 V-1 |
| WORK-F7 行为：各信源许可口径（PRD:318） | 本期 | §5.2、§5.5；M1 | M1 | 分级规则与 Japan Search 过滤、ColBase 署名、V&A 仅参考、NASA 第三方提示（静态 + 单元测试）；三家美术馆限定公有领域/CC0，荷兰国立博物馆按条目许可（静态） | 已确认变更（见 G-05 依据） | G-05 |
| WORK-F7 行为：调用在对话中可见（PRD:313） | 本期 | §6.1、§6.5；M4 | [M4](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M4.md) V-7；[A3](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5-A3.md) | SKILL 的 Media requests 入口与 new-work 条款在源文件与生成物中（静态）；三轮真实宿主会话的检索与取用均为可见调用并有事前说明（记录） | 已满足 | 待核实 V-2 |
| WORK-F8/AC-1 按编号取到素材区，核对真实宽高与解码完整性；来源清单六项（PRD:345） | 本期 | §5.8；M2 | M2 V-1④；A1 | 本轮取两张图：落在 `.phoenix-ui/media/`，报告宽高与系统看图工具一致，`integrity=decoded`，`sources.json` 有文件名、标题、来源页、许可、署名、取用日期（实测）；真实信源取用（记录） | 已满足 | — |
| WORK-F8/AC-2 目标在 `.phoenix-ui/` 以外时拒绝并说明（PRD:346） | 本期 | §5.10；M1 | M1 评审修复、[R3 复审](PHOENIX-UI-EVIDENCE-CRAFT-P3.code-review.r3.md) | `--dir public/img` 退出 1、说明原因、目录无变化（实测）；符号链接越界的 3 个单元测试通过（实测）；R3 的 4 类链接 × 4 个子命令（记录） | 已满足 | — |
| WORK-F8/AC-3 交付前核对：产品不引用素材区、没有许可清单或署名（PRD:347） | 本期 | §5.11、§6.1；M2、M4 | M2 V-1⑨；M4 V-7；A3 | `media audit` 报出副本、引用、署名三类并退出 2，清掉后退出 0（实测）；skill 要求交付前对产品路径运行并报告（静态）；宿主会话实际运行且 0 发现（记录） | 已满足 | 待核实 V-2 |
| WORK-F8/AC-4 贴图取颜色、粗糙度、法线并核对完整；视频同时得到首帧（PRD:348） | 本期 | §5.8、§5.9；M2 | M2 V-1④⑥；M5 H.264 | 贴图三张逐张核对字节数的单元测试、WebM 与 H.264 首帧成对登记（实测：本机 Chrome）；真实 Poly Haven 三张贴图与 NASA 视频（记录）；linux-arm64 的 Chromium 取不了 H.264（CI） | 已满足（视频以浏览器能解码为前提） | G-07 |
| WORK-F8 行为：素材只用于设计稿预览，不进项目资产与产品代码；来源清单不进产品（PRD:338-339） | 本期 | D3、§5.10、§6.1；M1、M4 | A3 | 素材区自带内容为 `*` 的 `.gitignore`，`git status` 不出现素材区文件（实测）；宿主会话 `src/` 前后哈希相同（记录）；skill 明确后续建议也不得把取用文件转为产品素材（静态） | 已满足 | — |
| WORK-F9/AC-1 输出为显示宽的 2 倍；不足不放大并注明；不透明出 WebP 或 JPEG，透明保留透明通道（PRD:365） | 本期；D12 输出 JPEG 与无损 WebP（用户确认） | §5.13、§5.14；M3 | [M3](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M3.md) V-4、V-5；[A2](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5-A2.md) | 3000 宽不透明图按显示宽 600 得到 1200×800 的 JPEG；800 宽的图最高 800 并注明「未放大」；透明图输出 WebP 且保留透明通道（实测，已看图） | 已满足 | G-03 |
| WORK-F9/AC-2 多宽度文件与 `srcset`/`sizes` 片段，路径可解析（PRD:366） | 本期 | §5.14；M3 | M3 V-4②；A2 | 片段含 600w、1200w 与 `sizes`，路径相对执行目录都存在（实测）；片段贴进页面后实拍无失败图片（记录） | 已满足 | — |
| WORK-F9/AC-3 原图字节不变；输出带相同来源，`--scan` 不报缺失（PRD:367） | 本期 | §5.14、§5.15；M3 | M3 V-4③；A2 | 5 个输入处理前后 SHA-256 相同；每个输出读出的来源与原图逐字相同；`SCAN: 6 rasters, 0 missing`（实测） | 已满足 | G-03、G-04 |
| WORK-F9/AC-4 按比例裁切、裁透明边；报告列出像素尺寸与字节数（PRD:368） | 本期 | §5.14；M3 | M3 V-4④；A2 | `--crop 16:9` 得 1200×675；`--trim` 裁出 839×839 的内容区；报告含尺寸与字节数（实测） | 已满足 | — |
| WORK-F9/AC-5 不可解码的文件单独失败并写明原因（PRD:369） | 本期 | §5.14；M3 | M3 V-4①；A2 | 坏文件报「图片解码失败」及原因，其余 3 张照常，退出 2（实测） | 已满足 | — |
| WORK-N1/AC-4 静态运行不外发；页面检测与取证只访问目标页面及资源（浏览器残余以 docs 清单为限）；不自动更新或下载；素材只在显式调用时联网、调用可见（PRD:557）**· 本期终验** | 本期；PRD v1.3 已按 D15 改写 | §5.16；M2、M5 | M2 V-3；[A4](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5-A4.md)；M5 V-11 | 静态三命令在只放行本机的 sandbox 内外输出相同、代理无请求（记录）；macOS 上页面检测与取证只见目标主机（记录、CI）；Linux 上有两项已登记的浏览器残余（CI）；`media` 之外无代码路径访问信源（静态） | 已确认变更；Linux 残余按 D-01 做关闭尝试，结论待 gaps plan 的 CI 结果 | G-02、G-06 |
| WORK-N1/AC-1–AC-3（P1 交付，持续满足，随本期终验核对）（PRD:554-556） | 跨期持续 | §7；M5 | M5 V-8、V-10 | capture 既有浏览器测试在收口全绿（记录）；走查与完工评审子代理本期无 diff，`skills/phoenix-ui/src` 只改了 SKILL 与 4 个 reference（静态） | 已满足 | — |
| WORK-N2/AC-3 素材与图片处理命令在四个平台可用，无需 Rust，缺 engine 给补装命令（PRD:574）· 终验期 P4 | 本期部分 | D1、§7；M5 | M5 V-8、V-11 | 四平台与 Node 18 的构建、暂存安装、冒烟全部成功，media 与 prep-image 冒烟结果一致，空 engine 目录退出 4（CI；`distribution=development`） | 已满足（开发候选范围）；发布包未核实 | G-07；待核实 V-3 |
| WORK-N4/AC-2 动 skill 前整合清单到本期范围，diff 逐项核对（PRD:608）· 终验期 P4 | 每期 | §0.6；M4、M5 | M4 V-6；M5 V-10 | 计划 §0.6 的 I-1–I-5 先于 skill 改动；本期 skill 源只改 5 个计划内文件，独立 skill 无 diff（静态） | 已满足 | — |
| WORK-N4/AC-1 第三方声明保留 oil-ui 的 MIT 全文与版权行（P1 交付，持续） | 跨期持续 | §6.6；M5 | M5 V-8、V-10 | NOTICE 与 THIRD-PARTY-NOTICES 的 oil-ui 条目补上 media、prep-image 的改编范围，版权行仍在（静态） | 已满足 | — |
| PRD §2.4 素材联系表：分级标记一眼可辨（PRD:148） | 本期 | D7、§5.7；M1 | M2 V-1 | 联系表每格底部整条色带加 PAGE/CHECK/REF 字样（实测，已看图）；格内文字只有 ASCII，标题看终端或 JSON | 已满足 | — |
| UAT-9 检索、预览、取用素材；取到资产目录被拒；产品无素材引用与署名（PRD:731） | 本期 | V-1、V-7、V-9 A1/A3 | M2、M4、A1、A3 | 同 WORK-F7/AC-1–3、WORK-F8/AC-1–3、WORK-N1/AC-4（实测 + 记录） | 已满足（终端许可文字除外） | G-01 |
| UAT-10 离线或某信源超时（PRD:732） | 本期 | V-1⑦、V-9 A1 | M2、A1 | 同 WORK-F7/AC-4（实测 + 记录） | 已满足 | — |
| UAT-11 3000px 不透明 PNG、显示宽 600、多宽度（PRD:733） | 本期 | V-4、V-9 A2 | M3、A2 | 同 WORK-F9/AC-1–3（实测） | 已满足 | — |
| P2 差距报告 D-01：typeset 断行指引补一句 | 承接（用户 2026-11-11 决定） | §6.4；M4 | M4 V-6 | `typeset.md` 源文件与生成物第 74 行都有「优先用 `<br>`；整段包进 inline-block 会退出 `xgent/cjk-*` 检查」一句（静态） | 已满足 | — |
| WORK-F10–F14、WORK-N2/AC-4、WORK-N4/AC-3 | P4 | — | — | 不在本期；由 P4 计划承接 | 范围外 | P4 |

无计划落点的需求：无。本期每条验收都在计划 §0.1 账本与里程碑中有落点；G-01 的缺口出在计划 §5.12 对终端格式的设计，不是漏了需求。未明确要求的 NFR（检索耗时、下载速度、输出体积）按实际运行记录看无明显低效，未做系统性验证。

## 4. Triage 与决策账本

### 待决问题

（无。Q1、Q2 已于 2026-11-12 答复，问题卡保留如下，决定见「已决决策」。）

#### Q1（G-06）· Linux 上浏览器的更新检查请求：再关一次，还是按现状登记？

- 期望与实际：WORK-N1/AC-4（本期终验）要求页面检测与取证只访问目标页面，且运行中不自动更新或下载。macOS 的 Chrome 157 已做到零残余；Linux x64 的 Chrome 156 每次都向 `update.googleapis.com` 发组件更新检查，Linux arm64 的 Chromium 147 在取证时向 `android.clients.google.com` 发 checkin。两项都已写进 docs 与测试清单。
- 影响与原因：只影响 Linux 上启动浏览器的命令；请求来自浏览器自身，不带项目信息；更新检查之后是否下载组件没有证据。现有参数关不掉这两条。
- 已知与未知：两个版本的 Chromium 源码里各有一个可把该请求改发到本机丢弃端口的开关（本轮已查到），引擎对账号请求已用同样办法；是否有效要上四平台 CI 才能确认。
- 选项：
  1. **再做一轮关闭尝试（推荐）**：加上这两个开关，在四平台 CI 上用现有 netlog 确认。清零则把 Linux 残余清单与 docs 改为空；没清零则保留登记并写明试过什么。代价：改 `cdp.rs` 几行与一轮 CI，需要推送授权。理由：WORK-N1 是本期终验的安全项，现在有具体、成本低的关闭办法。
  2. **接受现状**：沿用 D15 的登记口径，WORK-N1/AC-4 按 PRD v1.3 的字面在本期终验通过；不改代码。后果：Linux 上的更新检查长期存在，是否下载仍未知。
  3. **稍后决定**：保持待决，WORK-N1 的终验结论暂不下。

#### Q2（G-07）· 视频取用依赖本机浏览器的解码器：接受并写进 PRD，还是加退路？

- 期望与实际：WORK-F8/AC-4 要求取用视频时同时得到首帧；WORK-N2/AC-3 要求取用命令四个平台可用；PRD 的前置依赖表没有把浏览器列为素材取用的依赖。实现用本机无头浏览器解码首帧，解不了就整条视频取用失败并说明。
- 影响与原因：视频是 9 个类别中的 1 个，只有 NASA 一个信源，全是 H.264。三个平台实测通过；linux-arm64 的 Chromium 没有 H.264 解码器，视频取不了；没有浏览器的机器同样。图片与贴图不受影响。原因是 PRD 自己定的「不引入 ffmpeg」与本期「不新增第三方包」。
- 选项：
  1. **接受现状并同步 PRD（推荐）**：在 PRD §4 前置依赖表写明「取用视频需要本机浏览器且支持该视频编码，否则该视频取用失败」，WORK-N2/AC-3 注明视频取用以此为前提。不改代码。理由：与 PRD 的不引入 ffmpeg 一致，也保住了「取到的文件都经过解码核对」。
  2. **解码不了时退回信源缩略图**：视频照常落位，首帧图改用 NASA 提供的缩略图并标明「非首帧、视频未经解码核对」。后果：AC-4 的「首帧」与 AC-1 的完整性核对都要放宽，需要改代码与 PRD。
  3. **稍后决定**：保持待决，linux-arm64 的视频取用继续记为未验证。

### 已决决策

沿用的既有决定：计划 D3、D6、D12、D15 均为用户 2026-11-11 确认，出处见计划 §0.2；P2 报告的 D-01 已由本期 M4 落实。下表的 D-01、D-02 是本报告的编号，与 P2 报告的 D-01、计划的 D1–D18 无关。

| 决策 ID | 关联 G-ID | 用户选择与理由 | 确认日期与来源 | 新目标/验收边界 | 跟进/延期/豁免去向 |
| --- | --- | --- | --- | --- | --- |
| D-01 | G-06 | 「再关一次」：加上把组件更新检查与 checkin 改发到本机丢弃端口的两个启动开关，在四平台 CI 上用现有 netlog 确认。理由：WORK-N1 是本期终验的安全项，有具体、成本低的关闭办法 | 2026-11-12 · 本轮交互答复（问题卡 Q1） | 清零：Linux 残余清单与 docs 改为空，WORK-N1/AC-4 在四平台上按「目标页面之外无请求」验收。没清零：保留登记，docs 写明试过的开关与结果，按 PRD v1.3 的清单口径验收。两种结果都不屏蔽主机 | F-02 → gaps plan；CI 需用户授权推送 |
| D-02 | G-07（并带 G-05） | 「接受并同步 PRD」：视频取用以本机浏览器能解码该视频为前提，解不了即该视频取用失败；不改代码。理由：与 PRD 的不引入 ffmpeg 一致，保住取到的文件都经过解码核对 | 2026-11-12 · 本轮交互答复（问题卡 Q2） | PRD §4 前置依赖表写明该前置与未就绪时的表现；WORK-N2/AC-3 注明视频取用以此为前提；linux-arm64 上不带 H.264 的 Chromium 取不了视频属已知限制，不再记为待验证缺口 | F-03 → gaps plan（PRD 文档同步，同时写准 PRD:318 的荷兰国立博物馆口径） |

## 5. 待核实项与审查基线

### 待核实项

| # | 缺失的事实 | 影响的需求与结论 | 补证方式 | 是否阻塞 |
| --- | --- | --- | --- | --- |
| V-1 | 美国国会图书馆在本机网络下对非浏览器客户端一律 403；它的响应映射只有按参考夹具自写的样本，没有真实响应的证据 | WORK-F7 的 11 个信源之一；不可用时的表现已按 AC-4 验证 | 在能访问 LoC 的网络下运行 `media search "<词>" --kind map`，用真实响应补一份样本 | 否 |
| V-2 | 最终的 skill 文本没有再跑一轮真实宿主会话（第三轮之后只删掉了「必须另发独立消息」的额外要求） | WORK-N1/AC-4 的「对话中可见调用记录」、WORK-F8/AC-3 的流程侧；三轮真实会话按原判据通过 | 下次动 phoenix-ui skill 或发布前跑一次 V-9 A3 场景 | 否 |
| V-3 | 四平台证据来自开发候选（`distribution=development`）；最终 npm 包的安装与旧版升级没有在本期执行 | WORK-N2/AC-3（终验期 P4） | 发布时由既有发布检查覆盖 | 否 |
| V-4 | Edge、Brave 与其他 Chrome 版本的后台请求没有逐一验证 | WORK-N1/AC-4 的适用范围；docs 已写明 | 需要时用 `network_boundary` 的同一做法在对应浏览器上跑一次 | 否 |

### 审查基线

- 输入版本：PRD v1.3 与计划的哈希见页头；完成记录 M1–M5、M5-A1–A4；计划评审及 14 项处置；R1–R3 代码评审（R3「通过验收」，未提交）。
- 代码：HEAD `3094bbed`；本轮实测用 HEAD 构建的 debug 引擎（哈希见 §1），不是 CI 候选制品；CI 候选 `54b1d67` 之后的实现改动只有 `media/index.rs`、`media/look.rs` 两处评审修复，已被本轮单元测试与 R3 覆盖。
- 本次实际验证：仓库外临时项目里的 17 项离线检查与 31 个单元测试（环境 macOS arm64、Google Chrome，本机回环 HTTP 服务，代理变量已移除）；一次终端输出实测（G-01）。
- 本次静态核实：终端输出格式、信源许可过滤、启动参数、残余清单、skill 源与生成物、docs、README、声明、PRD diff、本期改动面。
- 未重跑并沿用记录的：11 个真实信源的检索与取用（V-1、A1，2026-11-12；media 的信源代码此后未变）、宿主会话（V-7、A3）、联网边界的浏览器段与 sandbox 段（V-3、A4）、四平台 CI（V-11）、`test:phoenix`。本轮没有向第三方信源发出请求。
- Chromium 开关的事实来自对应版本标签下的源码文件（2026-11-12 读取），未在浏览器上运行验证。

## 6. Follow-up 与 gaps plan

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| G-01 / WORK-F7/AC-1 | F-01：终端的检索与预览结果行加上许可文字 | 同一条目终端行的许可与 `--json` 的 `license` 一致；对外说明同步 | 无 | [PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps](PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md) M1（§5.1，V-1、V-2） |
| G-06 / D-01 / WORK-N1/AC-4 | F-02：浏览器启动参数加 `--component-updater=url-source=<本机丢弃端口>` 与 `--gcm-checkin-url=<本机丢弃端口>`；按 CI 结果更新联网边界测试的 Linux 残余清单与 docs | 四平台 CI 的 netlog：两个主机不再出现则清单与 docs 为空；仍出现则保留登记并写明试过的开关。macOS 维持零残余；既有浏览器测试不回归 | 四平台 CI 需用户授权推送 | [PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps](PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md) M2（§5.2，V-2、V-4） |
| G-07、G-05 / D-02 / WORK-F8/AC-4、WORK-N2/AC-3、WORK-F7 | F-03：PRD 同步——§4 前置依赖表加视频取用的浏览器前置；WORK-N2/AC-3 注明该前提；PRD:318 写准荷兰国立博物馆按条目许可分级；版本号加一 | PRD 与实现、docs 一致；只改计划 §5.3 所列四处（含 WORK-F8 的依赖一句）与版本号 | 无 | [PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps](PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md) M1（§5.3，V-3） |
| G-02、G-03、G-04 | 无 | — | — | 保留，无动作 |

gaps plan：[PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps](PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md)，生成时状态 **Ready**，两个里程碑。它只规划上表三项增量，需求来源写同一 PRD 与 P3，注明补齐原计划；G-02、G-03、G-04 与 §5 的四项待核实不在其中。实施进度以该计划的「恢复快照」为准，本报告不抄写。四平台 CI 需要用户授权推送，这只影响它的收口检查，不影响开工。

---

- 报告状态：已落盘并回读核对；Triage 已答复并登记；gaps plan 已生成并回读。
- 下一步（复制即用）：`/review-dev-plan docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md`；评审处置后 `/goal 按 docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md 开发`；代码评审通过后可再次 `/review-prd-dev-gaps docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md` 复核本期覆盖。
