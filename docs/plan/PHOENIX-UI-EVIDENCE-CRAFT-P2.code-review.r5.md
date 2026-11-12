# PHOENIX-UI-EVIDENCE-CRAFT-P2 · R5 代码评审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R5 · 修复效果复审 | [PHOENIX-UI-EVIDENCE-CRAFT-P2](PHOENIX-UI-EVIDENCE-CRAFT-P2.md) | R4 目标 `080c3bec7f652dc15bfd242233e18e2d54621e29` → 当前 HEAD `3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d` 加当前未提交内容；代码净差异为空，工作树指纹见 §3 | [R4](PHOENIX-UI-EVIDENCE-CRAFT-P2.code-review.r4.md) |

上轮实际 HEAD 也为 `3d328f7e151ff229ce1378cd2b0bf75c1a81bb6d`，只有代码目标之后的计划 SHA 回填。R4 报告 SHA-256 为 `62c35fbd9ad17ed5e39c09b3adc02bcc31e9871d14c3cbf0b908a5445d13f930`。本轮覆盖其 F-01–F-04、V-6 已关闭项及 V-7/V-9 待核实项；不是对原始实现的完整重审。已有处置记录为 [M1](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M1.md#评审修复)、[M2](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M2.md#验收) 与 [M4](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md#r3-修复后的适用性)。

## 1. 确认问题

未发现可确认的新增问题。本轮没有新增代码或测试 diff，不重新将已关闭问题登记为发现。

### 上轮问题复审

| 来源报告 / 原编号 | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R4 / F-01（沿用原编号） | 隐藏/透明行内文字进入事实；已解决 | `aa1d065` 修复，当前 `080c3be` 保留 | 已解决 | 独立读取 `typography.rs:87–110`：行内 opacity 与文本所在地 visibility 共同过滤，行事实与字体 run 共用；文件哈希与 R4 相同，复用 R4 两视口 clean 真实入口结果 | 无 |
| R4 / F-02（沿用原编号） | 缩放与 RTL 分栏漏报孤字；已解决 | `080c3be` | 已解决 | 独立读取 `typography.rs:171–180,197–212`：原点、步长和容差按 scale 换算，按 direction 推进；复核留存的 transform/RTL capture，各报末字“末”，所用二进制哈希匹配当前版本 | 无 |
| R4 / F-03（沿用原编号） | 保留节点参与字体发现及合并；已解决 | `aa1d065` 修复，当前保持 | 已解决 | 独立读取 `cjk.rs:253–260` 与 `driver.rs:1310–1325`：在字体命中与去重前排除保留节点；相对 R4 无 diff，复用 R4/R3 的相关入口与规则验证 | 无 |
| R4 / F-04（源自 R3） | 零栏距同行溢出误报孤字；已解决 | `080c3be` | 已解决 | 独立读取 `typography.rs:198–212`：同节点用 Range 分片证明换栏，跨节点保守不拆；零栏距 capture 的 findings 为空、valid 为 true；源码/夹具/二进制哈希匹配 R4，真实换栏正例沿用 R4 结果 | 无 |
| R4 §1.1 / V-6 | 当前修复的构建安装证据；已解决（本轮缺口） | M4 的 R3 开发构建、冒烟、安装；R4 独立入口核验 | 已解决 | 代码与构建输入无变更，debug 二进制仍为 `34d8fda5…`；M4 构建和安装证据仍适用。没有把该增量验证称为新版全量测试 | 无新增动作 |
| R4 §2 / V-7 | 当前版本 Claude 宿主取证缺证 | M4 留存失败会话，无新成功记录 | 待核实 | 实读 `host-session.jsonl` 的结果：`is_error=true`、`api_error_status=403`、`api_error=network_edge_blocked`；会话未执行取证。直接 CLI/capture 不替代宿主场景 | 恢复可用宿主环境后，按 M4 既有场景补证 |
| R4 §2 / V-9 | 当前候选四平台与 Node 18 CI 缺证 | 旧候选 `2518626` 成功，当前候选尚未推送 | 待核实 | 本轮实时查询：`080c3be`、`3d328f7e` 均无匹配 Phoenix run；远端 main 仍为完整 SHA `2518626b5614ea5a8b1784763ae5712e5b2853ee` | 经授权推送候选后，按完整 SHA 核对四个 UI Native 与 Node 18 终态 |

旧代码问题 4/4 维持已解决；新增问题 0；V-6 证据继续适用；两项待核实证据未关闭。本轮表内行号按当前文件重新定位，历史报告及其结论未改写。

## 2. 待核实项

- **V-7：当前修复版 Claude 宿主取证。** 留存会话 `33299391-19c7-4cd7-a4b9-7b4ad3438887` 在 API 前置网络边界失败；本轮没有环境已恢复的证据，未重试或绕过该限制。应按 [M4 的受影响取证场景](PHOENIX-UI-EVIDENCE-CRAFT-P2.records/M4.md#r3-修复后的适用性)，在干净项目安装本次开发构建，由 Claude 实际调用已安装 Phoenix，对缩放、RTL、零栏距页面取证，并逐页披露未解决发现与字体未核实备注。沿用计划指定的 Claude 范围，不扩大宿主矩阵。
- **V-9：当前候选的四平台验证。** 旧 run `42991386581` 对应 `2518626b…`，不含 `080c3be` 修复；当前目标及仅回填计划的 HEAD 均无匹配 run。本次是代码评审，没有推送授权，未执行推送或远端重跑。补证需四个 UI Native 和 Node 18 任务全部成功，并按计划核对 Linux 字体取数断言实际执行。

这两项是证据不足，不是已证明的新代码错误。没有用户豁免。嵌套分栏和零栏距同高跨文本节点的保守边界仍按现有计划处理，不另立需求。

## 3. 覆盖与验证

### 范围、版本与清单

- 意图：复核中文排版探针修复是否保持有效，及当前修复候选是否已有宿主与平台验收证据。
- 日期：2026-11-11；仓库 `/Users/rockie/Documents/GitHub/xgent/skills`，分支 `main`。输入仅为本计划路径，按最高轮次 R4 进入修复效果复审。
- 计划 SHA-256：`89eb8d534f4c6bbbb7e7cd7f30487dbaf1b1b000a36f2d07705202e77343d793`，与 R4 相同。完整读计划，重点核对 §5.5、V-2、V-6/V-7/V-9、M4 和复验规则。
- 登记提交均已解析并确认是当前 HEAD 祖先：`70fa92e3fe60a033f2b276becdead827b7a82744`、`e69c6007e5449e9fbacdfce267a377cfe4ee41f4`、`76814c1e6a94513918e173c8e1f131e1286f40a8`、`532f8d8f56ff87b739785fc5d518b5ad031ab3db`、`3d764e1b6699c37c0fcd5148b777227ef0c93c5d`、`f6cc57894db4fbca3f2633b19ca148907bef5a57`、`aa1d065f9d11983eb16c81d5e6ee19c6f90f90df`、`f273d6622342371545a7e0a4b70ef4537951c4f8`、`080c3bec7f652dc15bfd242233e18e2d54621e29`。这些实现均已进入前序评审；本轮没有上轮遗漏的新增登记实现。
- 完整读取 `080c3be..HEAD` diff，仅计划代码基线和完成记录 SHA 回填。相对 `080c3be` 的 `tools/phoenix-ui`、`skills/phoenix-ui`、`dist/phoenix-ui`、`docs/phoenix-ui.md` 净差异为空。旧区间内计划/记录回写及已由前轮处理的经验记录不纳入新增代码 findings。
- 当前 tracked 未提交文件为 M4 记录、既有经验和 SDLC 状态/索引；untracked 为既有评审报告及 SDLC 计划/评审页。逐项盘点，没有未提交实现或测试。M4 的未提交增量仅补记旧候选 CI 成功，不能关闭新候选 V-9；报告与台账本身不作为代码 findings 对象。
- tracked diff SHA-256：`0e46dff60480d4d62d304d00c4f20ff7354c3dcd27a454f4d08b0d7544a5addc`；index 为空，其 diff SHA-256 为 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`。输入快照共 2845 个文件（上述 tracked 代码/产物目录、公开文档、计划/M1/M2 和所有现有 modified/untracked 文件）；按路径排序的 `{路径:SHA-256}` 紧凑 JSON 聚合哈希为 `41bb19530939d3fb316c022a3eea6737d0efe7c539de64a5e732621633e94530`。本轮新报告与后续登记不属于输入。

### 已执行检查与证据复用

本轮行为没有变化。验证映射为：旧修复适用性 → 比较源码/夹具/构建输入及二进制哈希；宿主缺口 → 读取当前证据日志；CI 缺口 → 按候选 SHA 只读查询。未启动编译、全量测试、新宿主会话或 UI 验收。

| 检查 | 实际命令/方式 | 结果与限制 |
| --- | --- | --- |
| 版本与增量 | `git rev-parse HEAD`；`git log --format='%H %s' --name-only 70fa92e^..HEAD`；`git diff --no-ext-diff --no-textconv 080c3be HEAD --`；`git diff --cached --no-ext-diff --no-textconv` | 已读提交清单和本轮完整增量；只有计划 SHA 回填，index 为空 |
| 代码未变 | `git diff --no-ext-diff --no-textconv --name-status 080c3be -- tools/phoenix-ui skills/phoenix-ui dist/phoenix-ui docs/phoenix-ui.md` | 空输出；没有新增可归因回归的代码差异 |
| 哈希 | `shasum -a 256` 对计划、R4、探针、浏览器测试、breaks/clean、debug 二进制；Python 对输入文件聚合 | 探针 `52a735c7…`、测试 `c14da4fa…`、breaks `48eb590f…`、clean `ccd92fa5…`、二进制 `34d8fda5…` 均与 R4 全哈希一致 |
| 现存真实入口证据 | 解析 [transform](../../local/phoenix-ui/evidence-craft-p2-20261111/r3-apply/column-transform/capture-report.json)、[RTL](../../local/phoenix-ui/evidence-craft-p2-20261111/r3-apply/column-rtl/capture-report.json)、[zero-gap](../../local/phoenix-ui/evidence-craft-p2-20261111/r3-apply/column-zero-gap/capture-report.json) | 三份报告均 valid，无 failures；发现分别为孤字“末”、孤字“末”、空数组。三份报告的 binarySha256 与当前 debug 二进制一致。这是复核已有证据，不是本轮重新执行或开图 |
| 宿主 | 解析 [host-session.jsonl](../../local/phoenix-ui/evidence-craft-p2-20261111/r3-apply/host-session.jsonl) 的 result | `is_error=true`、HTTP 403、`network_edge_blocked`；未把 `subtype=success` 当作验收成功 |
| 远端 CI | `gh run list --workflow phoenix.yml --branch main -L 100 --json databaseId,headSha,status,conclusion` 精确筛选三个候选；`git ls-remote origin refs/heads/main` | 仅旧候选 `2518626b…` 命中 completed/success 的 `42991386581`；当前两个 SHA 无记录，远端 main 仍为旧候选。查询为本轮实时执行；旧 run 各 job 结果沿用 R4 已核实的终态，不重复请求 |

沿旧问题涉及的 `textNodes → linesOf → orphan/split_unit`、`font_fallback → is_reserved_node → 命中合并` 检查当前实现；静态推演与复用的真实入口结果分开记录。R4 的本机场景与 M4 的构建安装结果在代码、输入和二进制未变时继续适用，符合仓库的最小验证及证据复用规则。本轮无并行 reviewer，未重新审查原始实现、M3 全部文案或执行全仓普查。

报告落盘回读后、台账登记前再次比较上述 2845 个输入文件与 HEAD，均未改变。临时指纹文件仅用于本次一致性核对，结束后删除；未创建测试目录、worktree、分支、容器或后台进程。报告和未提交输入仅在当前工作树可取回，未提交共享。

## 4. 计划符合性

| 当前承诺 | 结论 |
| --- | --- |
| WORK-F5 / §5.5 / V-2：隐藏文字、分栏、缩放、RTL、零栏距与保留节点 | 旧修复维持有效；当前实现与 R4 验证版本一致 |
| V-6：受影响构建、安装与浏览器路径 | 既有证据仍适用，没有因评审轮次变化失效 |
| V-7：当前修复版宿主取证 | 待核实；403 失败记录不能满足验收 |
| V-9：同一当前候选的四平台与 Node 18 成功 | 待核实；旧候选成功不能转移给新代码 |
| M4 阶段收口 | 仍缺 V-7/V-9；计划维持 3/4 与阻塞，本轮未修改计划进度 |

计划来自 PRD v1.2 P2。本轮不要求范围外发布、部署、SPMS 录入或其他宿主矩阵。

## 5. 总体结论与后续

验收判定：需补证

**旧代码问题修复结论：已解决；包含待核实项的整体复审结论：无法判定。** 四项旧代码问题保持关闭，没有新增 findings。此次无新增代码 diff，省略新的 patch 正确性结论，保留 R4 对其修复增量的 `patch is correct` 判断；该判断不能替代整期验收。

剩余顺序：恢复 Claude 环境并补当前修复版宿主取证；经授权推送候选，以完整 SHA 核对四个平台和 Node 18 成功及指定断言。证据齐备后再复审，无需在代码与条件未变时重复本机已通过的检查。本报告不是合并、推送、发布或 PRD 验收授权。

```text
$review-code docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P2.md
```
