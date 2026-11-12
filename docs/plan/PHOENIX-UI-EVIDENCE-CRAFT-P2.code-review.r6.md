# PHOENIX-UI-EVIDENCE-CRAFT-P2 · R6 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R6 · 修复效果复审 | [PHOENIX-UI-EVIDENCE-CRAFT-P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.md) | R5 目标 `3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d` → 当前 HEAD `9b07ef066c9ebd00eea83587756f44532d1d9d75`，另核对当前未提交内容；新增代码仅为 `73a832a24f9bb0b6462913b45ad8e01ceef2d686` 的测试断言修正 | [R5](PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.r5.md) |

- 上轮版本与结论：R5 所审 HEAD 为 `3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d`，无未提交代码；旧代码问题已解决，但 V-7/V-9 缺证，整体验收为「需补证」。
- 本轮版本：当前 HEAD `9b07ef066c9ebd00eea83587756f44532d1d9d75`；V-9 实际 CI 候选为 `cd60330b222592aa56e3e653c6a8da9d0e25c972`，其后只有计划/收口记录提交。V-7 宿主引擎与 `3d328f7` 相同，当前引擎仍未改。
- 本轮范围：复核 R5 的 F-01–F-04、V-6/V-7/V-9，审查新增测试差异及其直接回归；不是对最初实现的完整重审。
- 处置依据：[M2 测试断言修正](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M2.md#v-9-补证发现的测试断言修正)与 [M4 R5 后补证](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md#r5-后补证)。本轮独立读取当前代码、宿主 transcript/capture/截图和 CI 元数据/留存日志，不把处置声明直接当作结论。

## 1. 确认问题

### 1.1 上轮问题与待核实项复审

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R4 / F-01 | 隐藏文字进入行/字体判定；R5 已解决 | `aa1d065f9d11983eb16c81d5e6ee19c6f90f90df`；当前源码未再变化 | 已解决 | `tools/phoenix-ui/crates/browser/src/typography.rs:84-109` 按文字所在节点的 `visibility`、祖先 inline/opacity 与字符 rect 排除未绘制内容；`tools/phoenix-ui/crates/rules/src/cjk.rs:243-259` 的页面规则自行过滤 reserved node 与非 passage。`3d328f7..9b07ef0` 对这些实现无差异，R5 浏览器证据继续适用 | 无 |
| R4 / F-02 | 分栏同行混入、栏尾孤字漏报；R5 已解决 | `f273d6622342371545a7e0a4b70ef4537951c4f8`、`080c3bec7f652dc15bfd242233e18e2d54621e29` | 已解决 | `typography.rs:146-214` 仍由多栏容器计算 origin/pitch/gap，按栏聚行，并在零栏距时要求同一文本 range 实际 fragmented；V-7 当前宿主 detect/capture 对 RTL、缩放各报告预期的末字孤字 | 无 |
| R4 / F-03 | `visibility:hidden` 文字触发字体回退；R5 已解决 | `aa1d065f9d11983eb16c81d5e6ee19c6f90f90df`；探针保留原过滤 | 已解决 | 当前 `typography.rs` 的可见文字过滤与 `cjk.rs` 的 `passage` 前置检查未变；V-7 三页无字体备注，V-9 四平台通用字体族断言实际通过 | 无 |
| R4 / F-04 | 宿主注入节点触发字体回退；R5 已解决 | `aa1d065f9d11983eb16c81d5e6ee19c6f90f90df` | 已解决 | `cjk.rs:255-258` 在逐元素读取 facts 后调用 `is_reserved_node`；`tools/phoenix-ui/crates/core/src/browser/driver.rs:1310-1325` 继续排除 overlay、`claude-`/`cic-` 与 live 子树。V-7 在真实 Claude 宿主中无字体回退/未核实备注 | 无 |
| R5 / V-6 | 开发构建/安装证据；R5 已解决 | M4 的 V-6 构建、安装与开发运行证据；此后仅测试断言变化 | 已解决 | 本轮未改变引擎、安装器或构建输入；复用 M4 已核实的 V-6 证据。V-7 又经公开安装入口在仓外独立目录实际安装并运行，未出现安装回归 | 无 |
| R5 / V-7 | 当前修复版缺真实 Claude 宿主取证 | M4「R5 后补证」；成功会话 `0f9be354-a094-428a-bc59-e3f6bc7bd1c8` | 已解决 | 成功 transcript 为 27 轮、退出 0、`is_error=false`，实际调用已安装 Phoenix 的 detect/capture 并读取三图。三页 detect 结果分别为 RTL 1 条 orphan、缩放 1 条 orphan、零栏距空数组，均无 `note:`；capture 三图 valid、无 failures/consoleErrors/failedRequests/findingsNotes。本轮另打开 PNG，可见预期栏归属与零栏距连续单行。首次 403 是保留的历史失败，不是成功会话的结论 | 无 |
| R5 / V-9 | 当前修复版缺四平台与 Node 18 CI | 修正测试后推送 `cd60330b222592aa56e3e653c6a8da9d0e25c972`；run [43007878500](https://github.com/XGENT-ai/skills/actions/runs/43007878500) | 已解决 | 本轮实时查询确认 run completed/success、headSha 精确匹配，四个 UI Native 与 Node 18 均 success。留存的五个 job 日志摘录显示通用字体族、中文分流、fixture 扫描均实际 ok，两个 macOS 另有字体回退测试 ok；Linux 各 786 pass、macOS 各 787 pass，均 0 fail/0 missing goldens，Node 18.20.8 离线安装与 capture smoke 通过 | 无 |

### 1.2 本轮新增问题

未发现可确认的问题。

新增代码差异只有 `tools/phoenix-ui/crates/cli/tests/xgent_rules.rs:433-443`：`a_generic_font_stack_is_verified_without_notes` 从只接受退出 0 改为接受 0 或已有 warning 的退出 2，仍拒绝其他退出码，并继续断言 stderr 不含 `note:`。`clean.html` 的分栏/溢出夹具会触发与字体核实无关的 warning；这次修改与该实际退出契约一致，没有移除字体备注断言或吞掉扫描错误。V-9 的四平台日志也证明修正后的断言真实执行并通过，而非浏览器不可用跳过。

## 2. 待核实 / 待决策项

无。R5 的 V-7/V-9 缺口均已有当前代码线的真实入口证据，不需要以未运行验证保留结论。

本轮重新下载五个 CI job 日志时，`gh run view 43007878500 --log --job <实际 job id>` 均退出 0 但返回空 stdout；因此指定测试行的证据来自 R5 后补证时已保存并在本轮读取的 excerpt，而 run/job 状态与 headSha 由本轮实时查询确认。空的重下载结果不被当作新通过证据，也不推翻已留存日志。

## 3. 覆盖与验证

### 3.1 范围与快照

- 一句话意图：在既有拉丁排版检测旁新增中文正文排版规则与实际行/字体探针，并以真实宿主和跨平台运行证明交付可用。
- 仓库：`/Users/rockie/Documents/GitHub/xgent/skills`；分支 `main`；核查时间 2026-11-11；原始输入仅为 `docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md`，按计划模式选择最高轮次 R5 复审。
- 提交核对：计划登记实现、修复与补证 SHA 均可解析且为当前 HEAD 祖先。本轮增量为 `3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d..9b07ef066c9ebd00eea83587756f44532d1d9d75`；其中 `73a832a24f9bb0b6462913b45ad8e01ceef2d686` 改测试，`cd60330b222592aa56e3e653c6a8da9d0e25c972`、`80b35124f954f04906a0094fcb351e83156f587e`、`9b07ef066c9ebd00eea83587756f44532d1d9d75` 仅改计划/记录。
- 计划 SHA-256：`84e2aa2a429d08d4c036ec89db3ca326fed51226575d08e24d1467c0dcab46db`；R5 报告 SHA-256：`e1998eec63f2fb3ed0a833978a13c6b484495648119e24e9a441efcc35e20a67`。
- 当前 index 为空，无未提交代码。tracked 修改仅为经验文件、SDLC 索引/状态页；untracked 为既有评审报告和 SDLC 状态页，没有新增实现或测试输入。写报告前，全部 tracked 修改与 untracked 文件的聚合 SHA-256 为 `bea89cf2407bceae887f4141685234364da384cf7dc2d4c547c8b333f9458099`；计划、M1–M4、相关探针/规则/driver/测试及 cjk 夹具聚合 SHA-256 为 `cf6a1b4c4b9b8d0e987140b1e41ee2e104e0b68bac404253643d665a2c21e1ba`。算法为按路径排序后对每项写入 `path + NUL + 8 字节大端长度 + 文件字节` 再取 SHA-256；R6 报告及其后的状态登记不属于输入。
- 未提交 tracked 文件为 `.xgent-ai/experience/ddfc2db0-1081-4c9e-8cee-8aecf3179c7e.md`、`.xgent-ai/sdlc/index.md`、`.xgent-ai/sdlc/status/plan/beb7ee04-62ec-4ec9-8e00-0ff2e45a5c87.md`、`.xgent-ai/sdlc/status/plan/index.md`、`.xgent-ai/sdlc/status/review/index.md`。untracked 还包括 P2 计划状态 `ddfc2db0-…`、七个既有 review 状态页、P2 的 R1–R5/计划评审报告及 P1-gaps 代码评审报告；均只作范围/历史上下文，不作代码 findings 对象。
- 文件限制：无用户指定文件过滤。本轮不重审未变化的全部原始实现；V-6 与 M1–M3 的既有结论只在输入未变且证据仍适用时复用。

### 3.2 实际核对与证据分层

- 静态/差异：读取完整 `3d328f7..9b07ef0` 文件清单和代码 patch；`cd60330..9b07ef0` 对引擎、skill、dist 与公开文档无差异。当前 `typography.rs`、`cjk.rs`、`driver.rs` 与 R5 相同，旧问题修复没有漂移。
- 局部测试：未在本机重复运行已通过的相同浏览器测试。`73a832a` 在 M2 已执行 `mise exec --no-deps -- cargo test -p xgent-cli --test xgent_rules a_generic_font_stack_is_verified_without_notes -- --nocapture --test-threads 1` 并通过；本轮由四平台当前候选日志进一步核实，不因报告需要重复消耗。
- 真实宿主入口：读取 `local/phoenix-ui/evidence-craft-p2-20261111/r5-evidence/host-session-proxy.jsonl` 的实际 tool call/result，不以最终文字自述替代工具证据；另读 `capture/capture-report.json` 并打开三张 PNG。宿主临时目录和开发二进制现已清理，本轮核对的是留存产物，不声称重新运行了 Claude。
- CI：实时 `gh run view 43007878500 --json status,conclusion,headSha,url,jobs` 为 success，headSha=`cd60330…`，目标五个 job 全 success；`git ls-remote origin refs/heads/main` 返回当前 `9b07ef0…`。测试细节读取同目录 `ci-cd60330-{linux-x64,linux-arm64,darwin-arm64,darwin-x64,node18}.excerpt.txt`。
- 评审分工：当前主 agent 独立复核、裁定和写报告，无并行 reviewer。未修改代码、计划、测试快照或远端状态；`gh` 仅只读查询。

## 4. 计划符合性

| 计划/验收项 | 本轮实现与证据 | 状态 |
| --- | --- | --- |
| R5 F-01–F-04 修复保持有效 | 当前实现仍包含可见文字、分栏和 reserved-node 修复；宿主结果与关闭判据一致 | 已核实 |
| M2 / V-9 测试断言修正 | 仅允许已知 warning 的退出 2，保留 `note:` 否定断言；四平台实际执行通过 | 已核实 |
| M4 / V-7 当前修复版宿主取证 | 仓外公开安装入口 + Claude Skill + detect/capture + capture 报告与截图均已核对 | 已核实 |
| M4 / V-9 四平台与 Node 18 | run 43007878500 对应 `cd60330`，五个目标任务 success，留存日志包含指定断言和 smoke 结果 | 已核实 |
| M1–M3、V-6 与原始完整实现 | 本轮未做全量重审；在源码/输入未变范围内复用前序已核实证据 | 复用，非本轮新增覆盖 |

本期 M4 及 R5 遗留补证已收口。本计划来自 PRD，但本次代码验收不等于 PRD 终验、npm/R2 发布或 SPMS 录入；WORK-N4 的后续终验边界不变。

## 5. 总体结论与修复顺序

**验收判定：通过验收。**

修复结论：**已解决**。`patch is correct`（限 `3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d..9b07ef066c9ebd00eea83587756f44532d1d9d75` 及声明的旧问题复审范围）。

旧 F-01–F-04 全部保持解决，V-6/V-7/V-9 均有足够证据；本轮新增测试断言没有可确认缺陷或直接回归，也没有剩余 P0–P3。当前版本的用户主路径已有真实 Claude 宿主与 capture 证据，四平台及 Node 18 的当前候选证据齐全。本判断不保证未知 bug 不存在，也不是合并、发布或 PRD 终验授权。

无需修复或再次空跑复审。本计划代码验收完成；如需继续核对 PRD 与开发交付差异，可执行：

```text
$review-prd-dev-gaps docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md
```
