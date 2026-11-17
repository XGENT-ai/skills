# Phoenix UI P3 gaps · R1 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R1 | [PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps](PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md) | 下列四个登记提交及当前未提交内容；最终核对版本 `f4d34303058dd966579b508b544fb941790f57e2`，文件限制与指纹见 §3 | 无；原 P3 的 R3 属另一计划，不作为本计划复审基线 |

日期：2026-11-12（UTC+8）。评审对象 ID：`872b4025-3968-4d21-92cc-bfb6506978d8`。按用户调用的 `review-code` 执行，由当前 agent 独立评审，未委派、修复、改计划、提交或推送。

## 1. 确认问题

未发现可确认的问题。P0 / P1 / P2 / P3 均为 0。

- 许可从每个条目的 `license` 原文输出；空字符串不产生括号。`search` 与 `preview` 经同一 `print_result`，JSON 分支与 fetch 的文件行未改。
- 浏览器启动参数经过公共启动入口进入 detect/capture；未加入主机屏蔽。候选先保留 Linux 残余清单，取得完整四场景证据后才删除清单，符合计划规定的收口顺序。
- 新增断言扩展既有用例，覆盖计划明确要求的终端输出与启动参数；没有新造 fixture 或扩大测试框架。
- PRD 的四处口径与版本号变更符合本期约定；荷兰国立博物馆许可解析、视频解码及文件落位行为与所补文字一致。

## 2. 待核实项与范围限制

本期无剩余阻塞性待核实或待决策项。

本轮实际补齐 [M2 记录](PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.records/M2.md#v-1-真实使用验收) 明示的图像未目检缺口：查看了原联系表与截图，并重新完成当前版本的 V-1，亲自查看新联系表和截图。没有把原记录的“全部通过”直接当作证据。

保留以下已约定边界，不扩展为缺陷或新门槛：

- LOC 对本机请求返回 403，按 docs 的单信源不可用契约显示 `UNAVAILABLE`；Commons 成功，未绕过服务限制。本轮没有验证全部 11 个外部信源。
- 四平台证据仅支持实际记录的浏览器版本和四个场景，不保证任意版本、任意时长均无后台请求；Edge、Brave、发布安装、宿主 skill 与 P4 不在本计划范围。
- CI 制品明确是 development，`browserAcceptance` / `providerSessionAcceptance` 仍为 `not-run`；本报告没有把 CI 全绿当作宿主验收或发布证据。当前 CLI 的真实使用验收另见 §3.3。

## 3. 覆盖与验证

### 3.1 固定范围与版本

意图：在素材终端结果上展示条目许可，实测关闭 Linux 浏览器两类残余请求，并同步相应文档。

仓库：`/Users/rockie/Documents/GitHub/xgent/skills`；分支 `main`。原输入：`$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md`。模式为计划初审，非整仓普查。

| 纳入提交（完整 SHA） | 内容 |
| --- | --- |
| `c2e6e24109220e1d072685fdcdd232ffe748efd0` | M1 许可结果行、既有检索用例、docs 与 PRD |
| `ed35605cb4cf64322b8620967cb9ee953130dab9` | M2 两个改发参数及包含断言 |
| `0f78b221d9d9334534fa2e0d59f51e5444774f22` | M2 记录登记的断言格式修正 |
| `f8a9993fa7865c99d1d23c1270ad426ad80f7a02` | 按 CI 判定删除残余清单、收口 docs |

四个 SHA 均已解析并确认是当前 HEAD 的祖先。逐个核对登记提交的实现 patch，再核对最终净效果；共同起始版本为 `402714ab7e8271bf5d0bb13b314ed173fa3bdedf`，最终版本为 `f4d34303058dd966579b508b544fb941790f57e2`。这不是该整个 Git 区间的全范围评审：用户本轮明确选择“按计划范围评审，排除无关提交”，排除合并 `86f8eb44eda3ee52a9a183c30ec5b95b3902ea43` 带入的其他工作。`577f34d7`、`b6f4a850`、`26f3617c`、`f4d34303` 的进度回写不作为代码 findings 对象。

代码 findings 限定以下五个文件；已阅读全部实现 diff，并追踪必要上下游：

| 文件（相对仓根） | 核对行为与直接上下游 |
| --- | --- |
| `tools/phoenix-ui/crates/cli/src/media/mod.rs` | 参数入口、execute → JSON/print_result、search/preview/fetch、既有检索用例；rights 的分级名与 sources 的许可字段 |
| `tools/phoenix-ui/crates/browser/src/cdp.rs` | default_chrome_args → Browser::launch；BrowserEngine、detect/capture 与视频首帧调用 |
| `tools/phoenix-ui/crates/cli/tests/network_boundary.rs` | 两种页面 × detect/capture，代理观测、netlog 留存、空 allowed 集合及外部资源例外 |
| `docs/phoenix-ui.md` | 终端字段、隐私与联网说明、公开素材使用步骤 |
| `docs/PRD-PHOENIX-UI-EVIDENCE-CRAFT.md` | v1.4、许可口径、WORK-F8 依赖、WORK-N2/AC-3 与视频前置表 |

核对 `0f78b221..86f8eb44` 的 `tools/` 及上述两份产品文档无差异；`f8a9993f..HEAD` 的同范围也无差异。因此候选的运行参数、处置轮最终引擎与当前受审实现分别可比对，合并未引入本计划代码差异。

计划版本：HEAD 下文件，SHA-256 `cac0275a55f09c7eb9f90daf71784c20f1265dafeae30609621c9d5c03a174dd`。当前代码与计划均无未提交差异；index 无 staged 差异、无冲突。初始 dirty 内容全部属于台账和评审记录，单列纳入盘点但不产生代码 findings：

```text
M  .xgent-ai/sdlc/status/plan/3fafbda8-0f05-482b-8320-18ca474ee753.md
M  .xgent-ai/sdlc/status/review/aeba18a7-c5b7-456c-b190-fbfc652b4d8c.md
M  .xgent-ai/sdlc/status/review/index.md
?? .xgent-ai/sdlc/status/review/3c7347d8-9253-4dc1-a51f-0f10e1244af1.md
?? .xgent-ai/sdlc/status/review/afed3e01-bb34-4c56-bb84-c63004ae4044.md
?? .xgent-ai/sdlc/status/review/d4d30e83-37c3-4412-bd39-6359b3b14c94.md
?? docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.review.md
?? docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.code-review.r3.md
?? docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3.prd-dev-gaps.md
```

工作树内容指纹 `a45a89eee080bccf6a95496be2ab8d8e5fa5720df2691806225a93eb1f9e98bf`：对初始 `git ls-files -z` 与 `git ls-files --others --exclude-standard -z` 中实际文件的字节逐个 SHA-256，共 4727 个路径，组成 path→hash 字典，以 Python `json.dumps(..., sort_keys=True)` 的 UTF-8 再作 SHA-256。index 指纹为 `SHA256(git ls-files --stage -z)`：`68fcf0bf4f03ebdac71a8b1443f2fc739a713dedf8b40cd13af3f4c105a582b2`。生成报告前逐文件复比无变化，HEAD 未变；本轮报告与状态回写不回流为审查输入。代码可从 Git 目标树恢复，排除的未提交记录只描述原工作树可见状态。

### 3.2 静态与局部验证

| 检查 | 实际执行与结果 | 适用限制 |
| --- | --- | --- |
| 构建当前 CLI | `tools/phoenix-ui` 下 `mise exec --no-deps -- cargo build -p phoenix-ui --bin phoenix-ui --locked --offline`，成功 | 为当前版本真实入口取证，不使用旧二进制冒充 |
| 许可输出与原检索行为 | 同目录 `env -u HTTPS_PROXY -u HTTP_PROXY -u ALL_PROXY -u https_proxy -u http_proxy -u all_proxy mise exec --no-deps -- cargo test -p phoenix-ui --lib media::tests --locked --offline -- --test-threads=1`，2 passed / 0 failed / 69 filtered | 局部测试，不代替真实信源与图像检查 |
| V-3 与差异整洁度 | 五文件完整净差异；`git diff --no-ext-diff --no-textconv --check 402714ab HEAD -- <上述五文件>` 成功 | PRD 恰四处与版本号；无本计划引入的 skill、help、oracle、依赖、安装器改动 |
| V-2 其余及阶段回归 | 本轮从 GitHub 核对两轮 CI 的任务结果、源码版本、候选制品阶段回执；复用未变化实现对应的已有检查 | 未在本机重跑 default_args_shape、network_boundary、lint 或全套回归；原命令与本机结果见 M2，四平台运行见下节 |

本轮 CLI 二进制 SHA-256：`1f090bb979b7151d63c0615d0ca79156cf228751aeb34bfcef928ff3934dace4`。构建/测试环境为仓库 mise 固定工具链，应用内部实现采用 workspace Rust 2021；未扩大为库 API 风格审查。

### 3.3 当前版本真实入口（V-1）

在仓库外新建临时 Git 项目，通过当前 `target/debug/phoenix-ui` 的公开命令执行。先读 `media --help` 与 docs 素材命令节；角色步骤不依赖内部接口。代理只用于本轮验证子进程，没有修改系统代理或产品配置。

| 步骤 | 实际结果 |
| --- | --- |
| `media search "nautical chart\|old world map" --kind map --sheet` | 6 条 Commons 候选，各行含 `可上线（Public domain）`，联系表显示六张地图及 C1–C6；LOC 403，exit 2，符合部分信源失败契约 |
| 同检索加 `--json` | 6 条对应编号与终端许可逐条匹配，均为 `Public domain` |
| `media preview C1 C2` | exit 0，两行均含许可，联系表生成 |
| `media fetch C1` | exit 0，`C1-5736843.jpg`，1920×1403、992224 字节；取用行保持原格式 |
| 在 `.phoenix-ui/preview.html` 引用 `media/C1-5736843.jpg`，执行 `capture .phoenix-ui/preview.html --viewport desktop --out .phoenix-ui/check` | Chrome 157.0.8103.51；exit 0，1 shot、0 invalid、0 failed、0 findings；`failures`、`failedImages`、`failedRequests` 均空 |
| 亲自查看联系表与 `desktop.png` | 候选图和编号可见；截图中 C1 地图完整显示，无空白图或加载错误。补齐原记录的目检缺口 |
| `media audit` | exit 0，核对发现 0 项 |

取证过程中的两项环境/操作修正均已闭环：首次 shell 无代理变量，Commons TLS 直连失败；确认系统已启用本机 7890 代理后，终止无效的长超时重试，显式设置 HTTP(S)_PROXY 并保留回环 NO_PROXY，完整流程成功。首次 audit 把评审者放在临时项目根的命令输出日志识别为素材引用/许可；把这些自建日志移到项目外后为 0 项，没有修改产品规则、忽略项或实现。

新截图 SHA-256：`e72f4a4bacb5621644dc60404d8043a629c38b227e119d40a75c87379cd98316`；capture-report SHA-256：`52b945346b418839c3cccb463e6f5f8de2fe6cdefce551cc341c6da39bbe95ae`。临时验证产物仅用于本轮取证，结束时清理，不作为需要维护的仓内测试或素材。

### 3.4 四平台原始证据（V-4）

本轮通过 GitHub connector 查询任务与制品，并下载候选四平台 evidence ZIP。逐一核对 ZIP SHA-256 与 GitHub artifact digest 相同、回执 `workflowRun` 与 `sourceCommit` 匹配，再读取全部 16 份 netlog；不是仅引用实施者摘要。

- [候选轮 43194037946](https://github.com/XGENT-ai/skills/actions/runs/43194037946)：head SHA `86f8eb44eda3ee52a9a183c30ec5b95b3902ea43`，四个 UI Native 任务及 UI Node 18 portable consumer smoke 均 success。每个平台四份 `network-{detect,capture}-network-{contained,external}.html.netlog.json` 齐全。
- [处置轮 43215790210](https://github.com/XGENT-ai/skills/actions/runs/43215790210)：制品所列 head SHA `f8a9993fa7865c99d1d23c1270ad426ad80f7a02`，四个 UI Native 的构建、冒烟、Full Phoenix runtime tests offline 均 success，Node 18 冒烟 success；没有重触发 CI。

| 候选 evidence artifact ID | 平台 / netlog 浏览器版本 | 场景数 | 两个残余主机相关事件 |
| --- | --- | --- | --- |
| 13933372985 | darwin-arm64 / 157.0.8103.52 | 4 | 均 0 |
| 13936438133 | darwin-x64 / 157.0.8103.51 | 4 | 均 0 |
| 13933142676 | linux-x64 / 156.0.8078.79 | 4 | 均 0 |
| 13932309439 | linux-arm64 / 147.0.7727.0 | 4 | 均 0 |

检查所有事件 params 中的 `update.googleapis.com`、`android.clients.google.com` 均无命中；抽取 URL/host/hostname/host_and_port 后，contained 场景只有 `127.0.0.1`，external 场景另有页面所需 `boundary-probe.invalid`。这里的“无外发”允许计划明示的回环丢弃端口请求，不声称浏览器不尝试任何本机连接。实际版本与 docs 一致，符合 §5.2 第一条处置路线。制品可用性受 GitHub 的 14 天保留期限制，报告保留运行和 artifact 身份。

## 4. 计划符合性

| 需求 / 本期任务 | 实现与证据 | 判定 |
| --- | --- | --- |
| WORK-F7/AC-1，M1、§5.1 | mod.rs:430–445 许可原文与空串分支；当前 CLI search/JSON/preview 对照、2 项局部测试、实际联系表 | 已核实 |
| WORK-N1/AC-4，M2、§5.2 | cdp.rs:78–79 参数；network_boundary.rs:420 起空清单；候选 16 份 netlog、最终四平台及 Node 18 CI、当前 capture | 已核实本期规定的关闭尝试与收口 |
| WORK-F7 许可文字、WORK-F8/AC-4、WORK-N2/AC-3，§5.3 | PRD 四处与版本号；sources.rs:741 起条目许可解析，poster.rs 及 fetch.rs:387–435 的视频解码/落位契约 | 已核实本期文字同步，未将 WORK-N2 跨期终验提前 |
| C-2 / C-3 / C-4 与明确排除 | 无主机屏蔽、无额外参数、无 oracle/help/skill/安装器/发布变更；新增测试限于计划约定 | 已核实 |

原 P3 差异报告中的 G-02/G-03/G-04 及其余待核实项沿本计划 §0.5 排除，不算本轮遗漏。未更新计划进度或旧报告结论。

## 5. 总体结论与下一步

验收判定：**通过验收**。

正确性结论：**patch is correct**，仅覆盖 §3.1 声明的登记提交、五文件最终行为及本轮工作树范围。未发现 P0–P3 缺陷；当前真实入口、图像目检、许可局部测试与四平台原始证据支持本计划的交付约定，无需执行修复。结论不等于合并、发布或整份 PRD 验收授权。

本计划代码验收完成。计划来源为 PRD，可选继续差异核查：

```text
$review-prd-dev-gaps docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P3-gaps.md
```
