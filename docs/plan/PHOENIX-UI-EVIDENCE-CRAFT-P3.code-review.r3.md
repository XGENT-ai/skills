# Phoenix UI 预览素材与交付图片处理 P3 · R3 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R3 | [P3 开发计划](PHOENIX-UI-EVIDENCE-CRAFT-P3.md) | `d73e4cfbc2e18f4ba4deb38a2c046c6682a423be..3094bbeda67dbdbb72806c1c82f27bc2fbb5a403` 中 P3 登记修复，排除用户明确排除的无关提交；开始时工作树/index 均干净，相关输入指纹 `37ed075da720d458dacba83b4addb1472a3c7a5a2fc0685c358bae5b32cb1568`，集合见 §3.1 | [R2](PHOENIX-UI-EVIDENCE-CRAFT-P3.code-review.r2.md) |

本轮为**修复效果复审**，覆盖 R2 全部旧问题、待核实项及直接修复回归，不是整期原始实现的完整重审。R2 所审 `6033a7abcae098697166200ba08e064b0c2f3648..d73e4cfbc2e18f4ba4deb38a2c046c6682a423be` 加当时未提交内容；其报告 SHA-256 为 `97e6a324c7eaec186492d641cde00c1f16ef289f233171546eaaf7febeca60e8`。本轮登记修复为 `b7d6042f96108383541cebcf972ce931c10875d7`，实际核查版本为上述 HEAD。已有处置见 [M1 的 R2 复审处置](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M1.md#r2-复审处置2026-11-12)。保留 R1/R2 历史结论。

## 上轮问题复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R2 / F-01，沿用 R1 F-01；R2 review `aeba18a7-c5b7-456c-b190-fbfc652b4d8c` | P2；部分解决：祖先例外使 fetch 创建产品目录后才拒绝，readonly audit 接受上跳链接 | `b7d6042` 收紧例外为词法素材根尚不存在；M1 记录已验证 | **已解决** | [index.rs:40–51](../../tools/phoenix-ui/crates/cli/src/media/index.rs#L40) 在 mkdir 与 readonly 返回前拒绝已有越界路径；本轮当前 CLI 对根到自身、根到 public、子路径上跳、子路径到 public 四种场景逐一执行四个子命令，均退出 1、目录清单无变化，本机 HTTP 服务零请求；正常初始化、fetch/audit 对照通过，见 §3.2 | 无 |
| R2 / F-02，沿用 R1 F-02；同一 R2 review ID | P3；R2 已解决：纸底逐边阈值正确 | `a21c228` 修复，本轮 look 实现没有变化 | **已解决**（维持关闭） | [look.rs:80–83](../../tools/phoenix-ui/crates/cli/src/media/look.rs#L80) 仍逐边检查；本轮公开 preview 入口重验 R1 原图：左三列 180、其余 220 时 `edgeFlat=true, paper=false`，纯灰 230 为 `paper=true`；R2 渐变图和局部测试证据因实现未变复用 | 无 |

## 1. 确认问题

未发现可确认的新问题。旧问题 F-01 已关闭；F-02 保持关闭。未发现本轮修复新增或激活的独立缺陷。

新增的一项测试验证已有根链接、子目录上跳链接在创建前拒绝，并保留首次初始化对照，断言针对可观察的错误与目录副作用；属于 F-01 的实际回归保护，无须为此新增测试体系。

## 2. 待核实项与适用限制

| R2 来源项 | 本轮核对结果 | 适用边界 / 后续 |
| --- | --- | --- |
| §2 Linux arm64 H.264 与其他浏览器 | **待核实**，保留原非阻塞限制 | 回读 `local/phoenix-ui/p3-acceptance/ci-43141743915/<平台>/<平台>-media-h264.json` 四份文件：三平台 passed；Linux arm64 为 unverified、缺少受支持视频流，失败清理 verified；均对应 `54b1d67`。本轮 browser/poster 与依赖未改，不把它们说成当前 HEAD 的四平台执行。计划 A-2/V-11 允许明确保留此限制；其他浏览器仍未补验 |
| §2 宿主最终文本版本边界 | **待核实**，保留原证据边界 | 回读 [M5-A3](PHOENIX-UI-EVIDENCE-CRAFT-P3.records/M5-A3.md) 的原判据、第二/三轮与收口记录，明确未执行第四轮。当前 skill 源、宿主调用入口没有变化，沿用 R2 已核过的行为证据；本轮未重跑宿主，不声称最终文本经第四轮实际运行 |
| §2 独立 ui-loop 改动 | **本轮未复核**，按用户选择排除 | 该改动已随 `5846c384` 提交；用户明确选择“排除无关提交，仅复核 P3 修复（推荐）”。portal-dev-setup 与 ui-loop 均不在本轮结论内，不要求对其真实流程补验才能关闭 P3 F-01 |
| §2 未重跑信源、全套测试与四平台 CI | 保留覆盖限制 | 本轮只为目录修复及 F-02 当前入口补证，没有重跑 11 个真实信源、浏览器联网、图片处理、完整 Cargo/Node 套件或四平台 CI；相应代码、配置和依赖未受本次修复影响，不重复整期收口 |

这些是未变路径的历史证据限制，不是本轮目录修复主路径缺证，也不将未运行检查写成失败或通过。

## 3. 覆盖与验证

意图：在已有素材根或子路径经符号链接逃逸时，四个 media 子命令都在创建目录、联网或 readonly 接受之前拒绝，同时保留首次初始化及正常素材取用。合同来自计划 §5.10、§7 和 R2 F-01；本轮用户授权仅为复审。当前主 agent 独立评审，无子 reviewer。

### 3.1 固定版本与范围

- 仓库：`/Users/rockie/Documents/GitHub/xgent/skills`；日期：2026-11-12（+08:00）；分支 main；开始时无 tracked、staged 或 untracked 变更。
- 输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md 复审`；按最高轮次选择 R2，另存 R3，不覆盖旧报告。
- 旧目标为 `d73e4cfbc2e18f4ba4deb38a2c046c6682a423be`，当前目标为 `3094bbeda67dbdbb72806c1c82f27bc2fbb5a403`；完整读取这一区间 P3 的实现净 diff，唯一实现改动为 `tools/phoenix-ui/crates/cli/src/media/index.rs` 的祖先例外和一项回归测试。计划 §5.10 与 M1 处置记录作为合同/证据读取，不修改。
- 纳入登记修复 `b7d6042f96108383541cebcf972ce931c10875d7`；`3094bbeda67dbdbb72806c1c82f27bc2fbb5a403` 为登记回填。`2526e0166a6d07bd40d70cb8a748122dadc41ade` 仅经验、台账、评审文档，排除代码 findings。`5846c38450f0b5dd8778f3f82fc43ea1ec6ce7ce` 涉 portal-dev-setup 三文件及 ui-loop；依用户明确回复排除。R2 当时的 ui-loop 未提交内容已进入后续提交，本轮不重归因于 P3 修复。
- 计划登记的 `2d5deab`、`49be9b9`、`ea0e5a3`、`0398407`、`622cbfc`、`54b1d67`、`6033a7a`、`a21c228`、`b7d6042` 均实际解析为完整 SHA 并验证为 HEAD 祖先；前八笔由 R1/R2 覆盖，本轮没有遗漏新的 P3 登记实现。
- 计划 SHA-256：`a51b6f93e52e9ee889add80847314ecadd00e840ea25b7824c29a127694d65a1`；M1 记录：`6ea0aa3fd791216cd1fcfaa3d774983a445c90e84adde4cadd2e06d47a7143f3`。
- 输入指纹算法：以下 22 文件逐文件 SHA-256，按路径排序为 `(路径, 摘要)` 列表，再对 Python `json.dumps(entries, ensure_ascii=False)` 的 UTF-8 字节取 SHA-256。集合为 media 目录的 10 个 `.rs`；`AGENTS.md`、`mise.toml`、`docs/phoenix-ui.md`、P3 计划、M1 记录、R2 报告、`skills/phoenix-ui/src/SKILL.src.md`、工具根 `Cargo.toml` 与 `Cargo.lock`、CLI 的 `Cargo.toml`、`src/main.rs`、`src/lib.rs`。核查前后均为报告头指纹；HEAD 未变。报告与后续 SDLC 回写不作为本轮输入。
- R2 至当前的 skill 源、browser/context、prep-image、network_boundary、look.rs 与 Cargo.lock 均无 diff；未受影响检查复用 R2，范围不扩大为整期重验。

### 3.2 行为 → 最小检查 → 实际结果

环境为 macOS arm64；cargo 从 `tools/phoenix-ui` 用 `mise exec --no-deps --` 执行，使用仓库固定工具链。CLI 场景在仓外 Python `TemporaryDirectory` 内运行，标准库 HTTP 服务仅提供两张自制 48×48 PNG，子进程移除 HTTP/HTTPS/ALL_PROXY 的大小写变量；没有访问业务数据。

| 行为 | 命令 / 场景 | 实际结果 |
| --- | --- | --- |
| 当前源码构建 | `mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline` | 退出 0；本轮执行生成的 `target/debug/phoenix-ui`，SHA-256 `146d669b8159e601572ef0ec01e7f84a56bc36cd682ef7fda8de4d45cd2722c7` |
| 目录修复与必要局部回归 | `mise exec --no-deps -- cargo test -p phoenix-ui --lib media::index::tests --locked --offline -- --test-threads=1` | 4 通过、0 失败；包括根 public、根自身/子路径上跳、已有 ignore 与索引稳定性 |
| F-01 根自身残留与子路径上跳回归 | `.phoenix-ui -> .` 配 `--dir .phoenix-ui/public/media`；正常根内 `escape -> ..` 配 `--dir .phoenix-ui/escape/public/media`；分别执行 `media audit`、`fetch C1`、`preview C1`、`search map`，均加 `--json` | 8 次均退出 1 且 stderr 为边界说明；每个项目完整路径清单前后相同，未创建 public 或素材文件，本机 HTTP 服务零请求。R2 同场景的 audit 退出 0、fetch 创建产品目录的失败已消失 |
| 原 F-01 的 public 链接保护 | `.phoenix-ui -> public`；正常根内 `escape -> ../public`，各指定不存在的素材子目录；同上四个公开命令 | 8 次均退出 1；完整路径清单不变，本机 HTTP 服务零请求；源码边界判定仍位于 HTTP agent 构造和索引读取之前 |
| readonly 与首次初始化对照 | 干净项目 `media audit --json`，再 `media fetch C1 --json`（尚无索引） | audit 退出 0 且不创建 `.phoenix-ui`；fetch 正常初始化 `.phoenix-ui/media/.gitignore` 为 `*`，随后因未知编号退出 1，没有被边界分支误拒绝 |
| 正常取用及 audit | 在上一步素材区写入代表性的外部索引，图片地址指向本机服务；`media fetch C1 --json` 后 `media audit --json` | 两者退出 0；PNG 为 48×48、完整字节与 HTTP 输入相同、`integrity=decoded`；sources.json 与素材仅在素材区，public 不存在，audit 无发现 |
| F-02 当前公开入口 | `media preview C1 C2 --json`：纯灰 230；左三列灰 180、其余 220 | 退出 0、联系表文件实际生成；纯灰 `paper=true`，单暗边 `edgeFlat=true, paper=false, edges.left=#b4b4b4`；没有把生成联系表写成独立视觉/UI 验收 |
| patch 完整性 | `git diff --no-ext-diff --no-textconv --check d73e4cf HEAD -- tools/phoenix-ui/crates/cli/src/media/index.rs` | 退出 0 |

以上为当前 CLI 公开入口和局部测试证据，非浏览器、真实第三方信源或宿主会话重跑。报告不依赖旧候选二进制冒充当前实现。临时项目和 HTTP 服务已清理，线程已结束；未新增仓内测试、分支、worktree 或 Docker 容器；未改代码、原计划或里程碑记录。

## 4. 计划符合性

| 当前修复合同 | 实现 / 证据 | 结论 |
| --- | --- | --- |
| WORK-F8/AC-2、§5.10 与 §7：根/子链接越界在创建前拒绝 | index.rs:40–48；4 类链接 × 4 个子命令的当前入口对照 | 已核实，F-01 关闭 |
| WORK-F8/AC-3、§5.11：audit 同样校验边界且只读 | 同一 resolve 的 create=false 分支在拒绝之后；越界和空区 audit 实测 | 已核实，R2 的 readonly 回归关闭 |
| WORK-F7/AC-2、§5.7：纸底逐边阈值 | look.rs 未变；本轮灰底与单暗边 preview 对照 | 维持已核实，F-02 关闭 |
| 首次初始化与正常取用 | 新区初始化、当前 PNG fetch、来源清单与 audit | 当前修复主路径已补真实 CLI 证据 |
| 其余 WORK-F9、联网、宿主和分发 | 实现与依赖未受本轮修复影响 | 复用 R2 的未受影响证据与限制；不声明当前提交已重新完成四平台/整期/UI 验收 |

## 5. 总体结论与修复顺序

验收判定：**通过验收**。

修复效果：**已解决**；选定旧问题 2 项已解决（F-01 本轮关闭、F-02 维持关闭），0 项不再适用，无未解决或阻塞回归。声明范围内修复增量正确性结论：**patch is correct**。根链接回到项目和子路径上跳现在都在目录创建与 readonly 接受之前被拒绝，正常初始化与取用保持可用；未发现新增缺陷，无须继续修复。

本轮是 P3 修复效果的代码验收；历史平台与宿主证据限制按 §2 保留，不代表已发布、已合并或独立 UI 验收通过。计划来源为 PRD，可选继续需求与开发差距核对；如果需要对整期界面产物单独验收，可另行发起 review-ui。本轮不自动启动这些流程。

```text
$review-prd-dev-gaps docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.md
```
