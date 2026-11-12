# Phoenix UI 预览素材与交付图片处理 P3 · R1 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [P3 开发计划](PHOENIX-UI-EVIDENCE-CRAFT-P3.md) | `b60ee18e396d90d3c380cf8e97f45643fec2331b..6033a7abcae098697166200ba08e064b0c2f3648` 净差异，加当前未提交变更；工作树内容指纹 `59e7cff1260e848d3aab9a4eb1803e3ce17d3f728dfd84028bfd49fe201e78d0`，定义见 §3 | 无；同名 `.review.md` 是计划文档评审，不是上一轮代码评审 |

## 1. 确认问题

### F-01 · [P2] 拒绝素材根目录指向产品目录的符号链接

- 定位：[media/index.rs:36–39](../../tools/phoenix-ui/crates/cli/src/media/index.rs#L36)，target 侧；本期新增。
- 问题：当项目的 `.phoenix-ui` 本身是指向 `public` 的符号链接时，`real_base` 变成 `<项目>/public`；当前检查只要求它位于项目内，再用它判断素材区边界，因此默认 `media fetch` 会成功把图片、`.gitignore` 和 `sources.json` 写进真实的 `public/media/`。这不是并发或竞态假设：已有目录布局即可触发，直接违反本期素材不进入产品、符号链接不能绕过边界的合同；新增的子目录越界检查没有覆盖根目录被重定向的情形。
- 证据：[计划 §5.10](PHOENIX-UI-EVIDENCE-CRAFT-P3.md#510-素材区边界)、[§7 写入范围](PHOENIX-UI-EVIDENCE-CRAFT-P3.md#7-nfr安全与运行保障)、[公开文档第 151 行](../phoenix-ui.md#L151)。隔离项目中创建 `public/media/`、`.phoenix-ui -> public`，在索引中登记本机 HTTP 服务的正常 PNG，运行 `media fetch C1 --json`：退出 **0**，报告 `area=.phoenix-ui/media`、`integrity=decoded`，实际新增 `public/media/C1-map.png`（64×48、442 字节）和 `public/media/sources.json`。对照 `.phoenix-ui/escape -> ../public` 的子目录链接，命令退出 1，且没有创建 `public/new`，说明现有保护只覆盖了后一种情况。
- 修复与验证：将允许的物理根锚定在规范化后的项目目录下的 `.phoenix-ui`，创建目录和联网前拒绝根目录链接到该边界以外；在现有素材区边界场景中补入 `.phoenix-ui -> public`，确认命令退出 1，且产品目录无新增或覆盖。保留普通素材区与已有子目录越界场景。
- 置信度：高；同版本候选二进制经公开 CLI 实际复现，源码和合同对应明确。

### F-02 · [P3] 按每条边分别判断浅色纸底

- 定位：[media/look.rs:108](../../tools/phoenix-ui/crates/cli/src/media/look.rs#L108)，target 侧；本期新增。
- 问题：`paper` 用四条边亮度、饱和度的总平均值判断，允许一条不满足条件的边被其余三条抵消；计划 §5.7 要求四条边带分别满足亮度 > 0.75、饱和度 < 0.2。于是带较暗边框的候选也会被标成浅色纸底，使消费 `look.paper` 的选图流程得到错误的观感信息；`edgeFlat` 仅检查各边内部是否平坦，不能保证每边足够亮。
- 证据：[计划第 538 行](PHOENIX-UI-EVIDENCE-CRAFT-P3.md#L538)。本机 HTTP 服务返回 48×48 灰度 PNG：左侧三列为 RGB(180,180,180)，其余为 RGB(220,220,220)。以该图为索引条目的 `previewUrl`，运行 `media preview C1 --json`，实际退出 0，返回 `edgeFlat=true`、`edges.left=#b4b4b4`、`paper=true`。左边亮度为 `180/255≈0.706`，按计划应为 `paper=false`。
- 修复与验证：对四条 `strip` 使用逐边条件；在已有观感数据用例中核对“一边低于阈值、其余边满足”的结果，同时保留纯浅灰纸底场景。
- 置信度：高；输入、阈值、实际输出均已核对。该问题优先级低，单独不阻止验收。

## 2. 待核实项与适用限制

- **Linux arm64 H.264 没有成功证据**：回读的 `linux-arm64-media-h264.json` 是 `unverified`，原因为缺少受支持的视频流，失败清理为 `verified`。计划 A-2、V-11 已明确允许按平台如实登记未验证，因此不作为新增缺陷，也不写成四平台 H.264 均可用。Edge、Brave 和其他浏览器版本的后台请求同样未在本轮补验。
- **宿主最终文本证据有版本边界**：本轮回读了 A3 第三轮原始 transcript、产品前后 hash 和截图；该轮入口仍含后来删除的“独立 assistant 消息”形式要求。最终入口允许可见工具说明承载事前说明，没有第四轮完整宿主运行。沿用原计划的说明、调用可见、隔离及 audit 判据，可复用已有行为证据；不声称实际运行过最终文本的第四轮会话。
- **当前另有独立 skill 未提交改动**：`skills/ui-loop/SKILL.md` 删除执行边界段，DSL 主体未改。本轮纳入其静态差异核对，但它不属于 P3 登记实现，未运行该 skill 的真实页面流程，也不把本次结论作为该独立 skill 的行为验收。
- 本轮没有重新访问 11 个真实素材信源、运行整个 Cargo/Node 套件或启动四平台 CI；使用留存的当前候选证据与下述定向复现。生成的浏览器 JS 中大段 WASM base64 未逐字节人工审查；检查了输入/输出摘要关系，未将二进制差异当作新增手写逻辑。

## 3. 覆盖与验证

意图：新增原生素材检索、预览、隔离取用与审计，以及保留来源的交付图片处理，同时限制自动路径的联网。这来自计划明示承诺；本轮用户要求是代码评审，未授权修复、提交或发布。

### 3.1 固定版本与范围

- 仓库：`/Users/rockie/Documents/GitHub/xgent/skills`；日期：2026-11-12（+08:00）；当前主 agent 独立评审，无并行 reviewer。
- 原输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md`，按计划默认范围执行初审。
- checkout：main，dirty@`5cf76ec8fd8b66f124684f22a7cd093e9048eefc`。
- 计划版本：当前文件 SHA-256 `3548beef618928f8d52b49359eaa0e9340e60a4045b2c545e9169c8eda1a0ce9`；PRD v1.3 SHA-256 `2591ffd2d61da4d8915ce504f8edc01d0d7518efbe2739706fbc9b23965dcdd9`。
- 所有登记提交均已解析并确认是 HEAD 的祖先：

| 登记用途 | 完整 SHA |
| --- | --- |
| M1 | `2d5deabaa7b49b0f23aef19b6d5b7cb81e82f3d5` |
| M2 | `49be9b99e19f5b4b2c7da1ea332f7b791cbdf022` |
| M3 | `ea0e5a329575a73c524d6cd8d3e87db17727ad0b` |
| M4 | `0398407ade72053ab5b695f6a76912b92ac0bb35` |
| M5 首轮候选 | `622cbfcf5356582c5b09b5ae665f18557bd8ed21` |
| M5 最终代码候选 | `54b1d67907a255bb838a7b8c24c0caa8539e48ed` |
| M5 收口及公开文档补充 | `6033a7abcae098697166200ba08e064b0c2f3648` |

中间 `eab1443`、`bd4e7eb`、`9d2fde2`、`e719aea` 及后续 `5cf76ec` 为本计划进度/SHA 回填。没有发现需另行决定纳入的未登记实现提交。`6033a7a` 还补充 `docs/phoenix-ui.md`，已纳入，不能仅按进度提交排除。代码 findings 基于基线至目标的净变化，不报告中间版本已经修复的问题；实际运行代码与 `54b1d67` 相同。

纳入的手写实现/直接调用链：

- `crates/cli/src/media/{mod,sources,rights,relevance,index,look,sheet,fetch,poster,audit}.rs`：参数、11 源映射、尺寸补全、分级、混排、稳定编号、联系表、完整解码、首帧、清单与 audit。
- `crates/cli/src/prep_image.rs`、`crates/context/src/{embed_prompt,lib}.rs`：输入收集、EXIF/ICC、透明裁边/裁切/缩放、格式、碰撞检查、来源与 HTML 片段；追踪原 `embed-prompt` 的读取优先级和超长 JPEG 路径。
- CLI 的 `main.rs`、`lib.rs`、`Cargo.toml`，`crates/detect/src/lib.rs`，`capture/mod.rs`，`crates/browser/src/cdp.rs`；`network_boundary.rs`、media 夹具与 oracle 用例、`scripts/smoke-phoenix-ui.mjs`。
- `skills/phoenix-ui/src/SKILL.src.md`、四份 reference、evals、生成的 skill/provider/bundle；`README.md`、`docs/phoenix-ui.md`、PRD 两处澄清、ENGINE/第三方声明及依赖边。

工作树和排除项：

- index 无差异，空 patch SHA-256 为 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。
- 初始 `git diff --no-ext-diff --no-textconv HEAD` 的字节 SHA-256 为 `8d72618029c16d0e482e81eeff0fade4ab8f617051f0b12ebba51df8588d07e8`；除 ui-loop 外，tracked 未提交内容为经验/状态记录。ui-loop 内容 SHA-256 为 `bff797cab9f91b23d33fbded964d4e091b91a77930d16cf666c87a02cc71a7dc`。
- untracked 全部为 `.xgent-ai/sdlc/status/` 下台账或 `docs/plan/` 下 P1/P2/P3 评审/差距报告；没有未跟踪实现文件。它们及计划/里程碑记录不作为代码 findings 对象；P3 计划与记录用于合同和验收证据。
- 报告头工作树指纹：枚举 `git diff --name-only -z b60ee18e HEAD` 的现存文件，加 `skills/ui-loop/SKILL.md`，得到 202 个路径；逐文件 SHA-256，按路径排序为 `(路径, 摘要)` 列表，再对 Python `json.dumps(entries, ensure_ascii=False)` 的 UTF-8 字节取 SHA-256。结束前重核无漂移。此指纹用于检测变化，未提交内容仅原工作树可复现；本轮新增报告、状态登记不作为本轮输入。

### 3.2 行为 → 检查 → 实际结果

| 改变的行为 | 最小检查 / 实际命令 | 结果与边界 |
| --- | --- | --- |
| 素材区符号链接隔离 | 临时项目+标准库本机 HTTP 服务，公开 `media fetch C1 --json`；子目录链接拒绝对照 | F-01 复现：根链接退出 0 并实际写入 public；子链接退出 1，无越界目录创建 |
| 纸底观感数据 | 48×48、左三列灰度 180/其余 220 的 PNG；公开 `media preview C1 --json` | F-02 复现：左边亮度不达标仍 `paper=true` |
| 图片处理与来源 | 临时 RGBA/不透明 PNG 先 `embed-prompt`，再 `prep-image opaque.png alpha.png bad.png --width 40 --widths 32,64 --out out --json` | 退出 2：坏图独立失败，另两图各输出 32/64 宽；JPEG/WebP 正确、不放大、两份原图 SHA 不变；四份输出来源逐字一致 |
| 来源扫描、输入保护 | `embed-prompt --scan out`；再以 `opaque.png` 和真实存在的 `opaque-32.jpg` 同批处理 | `SCAN: 4 rasters, 0 missing`、退出 0；冲突批退出 2、零成功文件、两项错误、原输入字节不变 |
| skill 及多 provider 生成一致性 | `cd tools/phoenix-ui && mise exec --no-deps -- bun scripts/render.mjs --check` | 退出 0、`status=fresh`，19 providers；bundle SHA `e602673c5f39ed02dee29accfb5df482bf8f0164561d6fa4f27d1968b45cf3f5`。首次调用未保留完成回执，补取一次完整结果；未运行聚合测试 |
| 真实素材/图片/宿主主路径 | 回读 [M1](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M1.md)、[M2](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M2.md)、[M3](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M3.md)、[M5](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5.md) 及 A1–A3；另读 A3 原始 transcript、verification 与 A2 原始 report | 记录覆盖真实检索、贴图、视频、EXIF/ICC、安装和实际页面。A3 原始调用含 search、fetch、两次独立 audit；产品 hash 前后一致、无 public。亲看其 mobile.png，真实海图可见。本轮没有重跑宿主会话或宣称独立 UI 验收 |
| 四平台候选 | 回读本机 `local/phoenix-ui/p3-acceptance/ci-43141743915/` 各平台 `*-ci-results.json`、`*-media-h264.json` 及现存 smoke 回执 | 四平台回执的 head/sourceCommit 均为 `54b1d67` 完整 SHA、各 phase success；H.264 为三平台 passed / Linux arm64 unverified。darwin-x64 缺独立 smoke JSON，三字段沿用 M5 所记 job 日志证据，不冒称本轮回读了不存在的文件；未重新向 GitHub 查询 |

上述定向复现使用本机开发候选 `local/phoenix-ui/development-release/phoenix-ui-darwin-arm64`，SHA-256 实算为 `ab7ce96746219869ded5c1d3107eb72bba58111f9664cc6a9167a13c7dce092a`。回读其 `build-record.json` 并逐项比较 484 个构建输入，当前仅 `network_boundary.rs` 与记录不同，运行实现输入一致。因此这些结果证明当前运行实现；不声称该旧候选的测试文件是最终版，也不以本机结果代替其他平台执行。

所有新建复现项目、本机服务、线程均已退出并清理；未在目标仓新增测试、修复代码或改变计划。复用了既有开发候选与验收资料，没有创建分支、worktree 或 Docker 容器。

## 4. 计划符合性

| 本期合同 | 实现 / 证据 | 核对结果 |
| --- | --- | --- |
| WORK-F7/AC-1–AC-4 | sources、rights、relevance、index、sheet、look；M2、A1 | 主链具备，F-02 为纸底判定偏差；源尺寸跟随、单源失败、离线不返缓存均有实现和记录 |
| WORK-F8/AC-1–AC-4 | fetch、poster、audit、index；M2、A1/A3 | 解码、来源清单、三贴图、首帧与审计具备；F-01 使素材隔离合同未完全成立 |
| WORK-F9/AC-1–AC-5 | prep_image 与 provenance API；M3、A2、本轮 CLI 检查 | 本轮未发现可确认缺陷；来源、输入保护和失败隔离定向通过，EXIF/ICC/视觉沿用已匹配实现的记录 |
| WORK-N1/AC-4 | 浏览器参数、network_boundary、明确调用入口；PRD v1.3、M5、A3/A4 | 参数与自动路径分离有实现；已登记的 Linux 浏览器残余和代理局限明确，不扩大为任意联网许可；本轮未复跑网络测试 |
| WORK-N2/AC-3 | 原生路由、构建/安装回执、四平台 CI | 开发候选证据可追溯；Linux arm64 H.264 限制保留，不宣称 npm/R2 已发布或跨期终验 |
| WORK-N4/AC-2、P2-D01 | 计划 §0.6、第三方声明、typeset、生成检查 | 对应接口与来源说明已落位；依赖仅新增既有包的边。live 两份文件为锁文件直接生成物，不计额外手写开发；当前 ui-loop dirty 改动单列，不归因于 P3 提交 |

## 5. 总体结论与修复顺序

验收判定：**需要修改**。

正确性结论：**patch is incorrect**。F-01 已证实会把素材写入产品目录，阻断本期隔离合同；F-02 是低优先级的观感数据错误。其余证据限制见 §2，不把缺失或未执行项写成通过。

先修复 F-01，确认根目录链接与子目录链接均在写入/联网前被拒绝，同时正常素材区仍能取用；随后修复 F-02 并核对逐边阈值。按受影响行为复验后再做修复效果复审，保留本轮结论。

```text
$apply-code-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.code-review.md
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md 复审
```
