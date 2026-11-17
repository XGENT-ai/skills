# Phoenix UI 预览素材与交付图片处理 P3 · R2 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R2 | [P3 开发计划](PHOENIX-UI-EVIDENCE-CRAFT-P3.md) | `6033a7abcae098697166200ba08e064b0c2f3648..d73e4cfbc2e18f4ba4deb38a2c046c6682a423be`，加当前未提交变更；相关输入内容指纹 `93702932373fb11b816ad60aa011f9f0b040a44d725c98ca95f6d2401d46a43b`，集合见 §3.1 | [R1](PHOENIX-UI-EVIDENCE-CRAFT-P3.code-review.md) |

本轮为**修复效果复审**，覆盖 R1 全部问题、待核实项及修复直接回归，没有完整重审整期原始实现。R1 原始基线为 `b60ee18e396d90d3c380cf8e97f45643fec2331b`，目标为 `6033a7abcae098697166200ba08e064b0c2f3648` 加当时未提交内容；R1 报告 SHA-256 为 `cde7dc38e9d9d44c70da9f83c04d1fe59026e428abe1932b398f21e6f95c42ff`。本轮修复提交为 `a21c228fe20d166be9e434a8d190e0d9ecd5abf2`，当前核查版本为上述目标 HEAD。已有处置见 [M1 评审修复](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M1.md#评审修复)，历史报告与处置结论保持不变。

## 上轮问题复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R1 / F-01；review `dd33f9d0-4a31-492e-92ea-bc9edb6c4fa8` | P2；素材根指向产品目录时可写出图片与清单 | `a21c228` 将物理根锚定为项目的 `.phoenix-ui` | **部分解决** | `.phoenix-ui -> public` 已退出 1、零请求、无新增文件；正常 fetch 仍成功。但祖先例外仍放行 `.phoenix-ui -> .`，先创建产品目录再拒绝；并使原先会拒绝的子目录上跳链接出现相同副作用，audit 还会返回成功。见 F-01 与 §3.2 | 收紧祖先例外；在任何目录创建及 readonly 返回前拒绝已有链接逃逸 |
| R1 / F-02；同一 review ID | P3；四边总平均使单边不达标仍报纸底 | `a21c228` 改为逐边同时检查两个阈值 | **已解决** | 当前 `look.rs:80–83` 使用 `all`；R1 原图、亮度渐变图、饱和度渐变图均 `edgeFlat=true, paper=false`；纯浅灰图仍 `paper=true`；3 项观感测试通过 | 无 |

## 1. 确认问题

### F-01 · [P2] 仅在素材根尚未创建时允许项目祖先例外（旧问题部分解决）

- 定位：[media/index.rs:40–41](../../tools/phoenix-ui/crates/cli/src/media/index.rs#L40)，target 侧 `d73e4cf`，位于修复 diff。
- 问题：新增的祖先分支只检查真实父目录是否为项目内的素材根祖先，没有区分“素材根尚不存在”和“已有符号链接回到项目根”。当 `.phoenix-ui -> .`、目标为 `.phoenix-ui/public/media` 时，`real_parent` 等于项目根，检查放行，`create_dir_all` 实际创建产品的 `public/media/`，随后才被最终路径检查拒绝。同一原因还激活了修复回归：正常素材根下的 `escape -> ..` 原先会提前拒绝，现在 `fetch --dir .phoenix-ui/escape/public/media` 也先创建产品目录才退出 1，而相同目标的 readonly `audit` 直接退出 0、报告无取用记录。素材文件和清单已不会像 R1 那样写进产品，但创建前拒绝越界、四个子命令执行同一边界检查的合同仍未满足，F-01 不能完全关闭。
- 证据：[计划 §5.10 第 2 步及末段](PHOENIX-UI-EVIDENCE-CRAFT-P3.md#510-素材区边界)明确要求根/子目录链接在创建目录前拒绝，[§7 写入范围](PHOENIX-UI-EVIDENCE-CRAFT-P3.md#7-nfr安全与运行保障)限定 media 写入边界。当前 CLI 实测根链接 fetch 退出 1，但新建 `public/`、`public/media/`；子链接对照中，R1 候选 audit/fetch 均退出 1、目录不变，当前 audit 退出 0、fetch 退出 1 且创建上述目录。没有索引、下载或竞态前提；失败发生在读取编号和联网之前。
- 修复与验证：祖先例外只用于词法素材根真正尚未创建的场景；素材根或其子路径已经存在时，要求其真实路径保持在锚定根内。沿用现有边界场景，加入根链接到 `.`、子链接到 `..`，同时验证 audit 退出 1、fetch 在建目录前退出 1，产品目录前后相同；保留首次正常初始化与正常 fetch 对照。无需增加锁、沙箱或其他机制。
- 置信度：高；当前源码、当前 CLI 及对应旧实现候选的前后对照一致。

本轮未另列新增 F-ID：上述直接回归与 F-01 残留同根因、同修复动作，合并记录；F-02 没有相关回归。

## 2. 待核实项与适用限制

| R1 来源项 | 本轮核对结果 | 适用边界 / 后续 |
| --- | --- | --- |
| §2 Linux arm64 H.264 与其他浏览器 | **待核实**，保留非阻塞限制 | 回读 `local/phoenix-ui/p3-acceptance/ci-43141743915/` 四份 H.264 JSON：三平台 passed；Linux arm64 为 unverified、缺少受支持视频流，failureCleanup=verified，候选均为 `54b1d67`。本次未改变解码/浏览器实现，不将历史 CI 宣称为当前提交已执行，也不将其他浏览器视为已验证；按计划 A-2/V-11 允许如实保留 |
| §2 宿主最终文本版本边界 | **待核实**，保留既有证据边界 | 回读 [M5-A3](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5-A3.md) 的第二/三轮说明、调用、隔离、audit 与产品 hash 记录；最终文本没有第四轮完整宿主运行。`skills/phoenix-ui/src` 在修复增量内无变化，相关历史行为证据仍可复用；本轮只为改变的目录/观感主路径补 CLI 证据，未重跑宿主 |
| §2 独立 ui-loop 未提交变更 | **本轮未复核**其真实页面流程，静态差异保持原限制 | 删除执行边界段的差异与 R1 相同，文件 hash 未变；不属于 P3 登记实现。本轮记录并核对该差异没有新的变化，不把 P3 结论作为 ui-loop 行为验收 |
| §2 未重跑信源、全套测试与四平台 CI | 保留覆盖限制 | 本轮运行受修复影响的本机 CLI、目录/观感测试；未重跑 11 个外部信源、浏览器联网、图片处理、生成、完整 Cargo/Node 套件或四平台 CI。未改其实现/依赖，不为复审重复整期收口 |

以上不是新增缺陷；本轮已证实的阻塞是 F-01，而非把未运行的检查当作失败。

## 3. 覆盖与验证

意图：修复素材目录的物理根隔离和浅色纸底逐边判定；来自计划与 M1 处置记录的明确合同。本轮用户授权为复审，未授权修复、提交或发布。由当前主 agent 独立评审。

### 3.1 固定版本与范围

- 仓库：`/Users/rockie/Documents/GitHub/xgent/skills`；日期：2026-11-12（+08:00）；分支 main，dirty。
- 原输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md 复审`。仅存在 R1 代码评审，按轮次选取；同名 `.review.md` 不参与代码轮次选择。新报告保存为 R2，不覆盖历史报告。
- 比较方式：R1 的可解析目标 `6033a7abcae098697166200ba08e064b0c2f3648` → 当前 `d73e4cfbc2e18f4ba4deb38a2c046c6682a423be` 净差异，并单列当前未提交内容；R1 当时的 ui-loop 未提交内容尚未入提交且本轮未变。
- 完整读取修复净 diff：`media/index.rs`、`media/look.rs` 两个实现文件及各自新增测试；`docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md`、其 `records/M1.md` 仅作合同与处置证据。`5cf76ec8fd8b66f124684f22a7cd093e9048eefc`、`d73e4cfbc2e18f4ba4deb38a2c046c6682a423be` 仅回填计划，代码变化均在 `a21c228fe20d166be9e434a8d190e0d9ecd5abf2`。区间内没有未登记实现提交。
- 计划登记的 `2d5deab`、`49be9b9`、`ea0e5a3`、`0398407`、`622cbfc`、`54b1d67`、`6033a7a`、`a21c228` 均实际解析并确认是 HEAD 祖先；前七笔已在 R1 覆盖，完整 SHA 沿用 R1 §3.1，新增修复完整 SHA 如上。
- 计划当前 SHA-256：`745bbba39d6402d3e4e9312cd7c0fee06a0c4077cc5b2cd8d25dc58e91d1e89a`；M1 处置记录：`8bc417b649f43b8f92cd5a96711dfd31d15f19db9dcdd03e4cf193310092a36b`。
- index 无差异，空 patch SHA-256 为 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。未提交实现仅 `skills/ui-loop/SKILL.md`，内容 SHA-256 `bff797cab9f91b23d33fbded964d4e091b91a77930d16cf666c87a02cc71a7dc`，HEAD 净 patch SHA-256 `022680e608693b3eefb0cf34731e151560d69e02d875ac1802e04aa72206e8fb`。其余 tracked/untracked 为经验、SDLC 状态与评审/差距文档，没有未跟踪实现；不作为代码 findings 对象。
- 报告头指纹算法：对以下 21 个当前文件逐项取 SHA-256，按路径排序为 `(路径, 摘要)`，再对 Python `json.dumps(entries, ensure_ascii=False)` 的 UTF-8 字节取 SHA-256：`media/` 下 10 个 `.rs`；上述 P3 计划、M1 记录、R1 报告；`AGENTS.md`、`mise.toml`、`docs/phoenix-ui.md`、`skills/phoenix-ui/src/SKILL.src.md`、`skills/ui-loop/SKILL.md`、CLI 的 `Cargo.toml`、`src/lib.rs`、`src/main.rs`。结束前逐文件重核，无漂移；新报告与状态回写排除在输入之外。

### 3.2 行为 → 最小检查 → 实际结果

运行环境：macOS arm64；在 `tools/phoenix-ui` 通过 `mise exec --no-deps --` 使用仓库固定工具链。CLI 项目与 Python 标准库本机 HTTP 服务位于仓库外临时目录；只提供自制 PNG，不访问业务数据；CLI 子进程移除 HTTP/HTTPS/ALL_PROXY 的大小写变量。

| 行为 | 实际命令 / 场景 | 结果 |
| --- | --- | --- |
| 当前实现可执行 | `mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` | 退出 0；运行当前生成的 `target/debug/phoenix-ui`，SHA-256 `d90564968c6fdafd45e2ae0457a90328703c0c14e0e208d045c2273c49a55426` |
| 目录隔离既有/新增用例 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib media::index::tests --locked --offline -- --test-threads=1` | 3 通过；覆盖普通/子链接拒绝、ignore 保留、根到 public 拒绝、索引并发与沿用。没有把它们通过当作目录隔离完整成立 |
| 纸底既有/新增用例 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib media::look::tests --locked --offline -- --test-threads=1` | 3 通过，含纯灰、噪声、单边亮度/饱和度失败 |
| 原 F-01 公开入口与正常路径 | 索引指向本机 PNG；`media fetch C1 --json`，根到 public、子链接到 public、新建正常区及 readonly 空区对照 | 根/子 public 链接退出 1、零 HTTP 请求；正常 fetch 退出 0，得到 48×48 PNG、`integrity=decoded`、sources.json，下载 hash 与 HTTP 输入相同；空区 audit 退出 0 且不创建目录 |
| F-01 根到项目自身残留 | `.phoenix-ui -> .`；`media fetch C1 --dir .phoenix-ui/public/media --json` | 退出 1、零 HTTP 请求，但实际创建产品的 `public/media/`；相同目标 audit 退出 0 |
| F-01 修复直接回归 | 正常 `.phoenix-ui` 内 `escape -> ..`；依次 `media audit --dir .phoenix-ui/escape/public/media --json` 与 `media fetch C1 --dir .phoenix-ui/escape/public/media --json` | 旧候选：两命令均退出 1、没有产品目录。当前：audit 退出 0，fetch 退出 1 但创建产品目录。没有预置索引，不会触发下载；F-01 因此部分解决 |
| F-02 当前公开 preview 入口 | 48×48 的 R1 左三列灰度 180/其余 220 图，以及逐行亮度 235→185、逐行饱和度递增图；分别 `media preview C1 --json` | 全部退出 0、生成联系表；三图 `edgeFlat=true, paper=false`，R1 左边仍为 `#b4b4b4`；纯灰 230 对照为 `paper=true` |
| diff 完整性 | `git diff --no-ext-diff --no-textconv 6033a7a HEAD --check` | 退出 0 |

旧候选为 `local/phoenix-ui/development-release/phoenix-ui-darwin-arm64`，实算 SHA-256 `ab7ce96746219869ded5c1d3107eb72bba58111f9664cc6a9167a13c7dce092a`，与其 build-record 一致。将 build-record 的 484 个输入逐项与 `6033a7a` 版本核对，只有测试文件 `crates/cli/tests/network_boundary.rs` 不同；运行实现匹配 R1，故用于上述边界行为的版本对照，不把它当作当前修复版。

最短复现（将 `bin` 换成当前 CLI 的绝对路径；在仓库外执行）：

```sh
tmp=$(mktemp -d)
mkdir "$tmp/.phoenix-ui"
ln -s .. "$tmp/.phoenix-ui/escape"
cd "$tmp"
"$bin" media audit --dir .phoenix-ui/escape/public/media --json
"$bin" media fetch C1 --dir .phoenix-ui/escape/public/media --json
test -d public/media
```

当前依次退出 0、1、0；正确边界应为 audit/fetch 均提前退出 1，且最后目录检查不成立。无需联外网或登记素材。

上述为真实 CLI 主路径与局部测试证据；未做浏览器或视觉/UI 验收。最终输入指纹重核未变，服务与 CLI 子进程均已结束，复现脚本、项目及临时 Git 目录已清理；未向仓库新增测试，既有候选与 CI 资料保留。未创建分支、worktree、Docker 容器，未改代码或计划。

## 4. 计划符合性

| 本轮合同 | 当前实现 / 证据 | 结论 |
| --- | --- | --- |
| WORK-F7/AC-2、§5.7 的纸底观感 | `look.rs:80–83`、3 项测试、当前 CLI 的四图 preview 对照 | 已核实；F-02 关闭 |
| WORK-F8/AC-2、§5.10 与 §7 的物理目录隔离 | `index.rs:40–48`、正常/根 public 对照及项目祖先逃逸实测 | 部分实现；F-01 残留且存在直接回归 |
| WORK-F8/AC-3、§5.11 的 audit 边界 | `execute` 先调用 `resolve_readonly`，当前对子链接上跳目标返回无取用记录 | readonly 不写入仍成立；先拒绝越界目标的合同不成立，同根因计入 F-01 |
| WORK-F9、浏览器联网、宿主说明/生成/分发 | 与 R1 目标相比，图片处理、context/browser、network_boundary 与 skill 源无变化 | 复用 R1 的未受影响证据及原有限制；不宣称当前修复提交已重跑全期或四平台检查 |

## 5. 总体结论与修复顺序

验收判定：**需要修改**。

修复效果：**未完全解决**；旧问题 1 项已解决（F-02）、1 项部分解决（F-01），无不再适用项。修复增量正确性结论：**patch is incorrect**。原来的素材文件外写已被拦截，但祖先例外仍允许创建产品目录，并使原来会拒绝的子目录上跳目标被 readonly audit 接受；不是仅因缺少全套验证而拒绝验收。

先收紧 F-01 的祖先例外，确认根/子路径回到项目祖先时 audit/fetch 都在创建前拒绝、产品目录前后相同，且正常首次初始化与取用仍可用，再针对该范围复审。F-02 及未受影响的已通过检查可复用。修复引入的同根因回归已并入 F-01，没有额外独立新问题；宿主最终文本与平台限制见 §2。

```text
$apply-code-review docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.code-review.r2.md
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md 复审
```
