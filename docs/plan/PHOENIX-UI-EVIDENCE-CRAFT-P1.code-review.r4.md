# PHOENIX-UI-EVIDENCE-CRAFT-P1 · R4 代码修复效果复审

## 评审对象

| 轮次 | 开发计划 | 审查范围 | 上轮报告 |
| --- | --- | --- | --- |
| R4 · 修复效果复审 | [Phoenix UI 多状态取证与盲测走查 P1](PHOENIX-UI-EVIDENCE-CRAFT-P1.md) | `92ec5fc8fb35c2e182ea9a79fc5f920a66a45e46..0509ba6c9012f5eb5b6a51507a28d10ad7a66601`；本计划实现净差异为空，复核 R3 全部旧问题及 V-6/V-8/V-9 缺证项；补证文档为未提交工作树，见 §4 | [R3 代码复审](PHOENIX-UI-EVIDENCE-CRAFT-P1.code-review.r3.md) |

本轮意图：核对恢复的真实宿主材料、当前候选 CI 和五项既有修复，判断 R3 的补证缺口是否关闭。用户限定“只审本计划”，随后授权“继续 V-8 CI 证据及之后的独立复审”。不重做整期初审。

R3 报告 SHA-256：`ec2f4da39bbec00047b362c22cf1eda8c6582423dda5f64ee4910c612b03f788`，对象 ID `67b2c314-0576-4308-8f94-f9dd1ee3130a`。处置来源为 [M2 修复](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M2.md#评审修复)、[M5 宿主补证](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M5.md#宿主补证)、[M6 宿主补证及当前 CI](PHOENIX-UI-EVIDENCE-CRAFT-P1.records/M6.md#宿主补证)。本轮由当前主 agent 独立核读源码、原始会话和 CI 制品；未派子代理，不把补证记录或场景评分直接当作验收结论。

## 1. 确认问题

未发现可确认的新增问题。F-01–F-05 的修复保持成立，未发现回归。

V-6/V-9 原材料与 V-8 当前候选缺口均已完成核查，没有剩余 P0–P2 问题。

## 2. 上轮问题复审

以下 F-ID 沿用 R1，经 R2/R3 处置；来源链不重新编号。

| 来源报告 / ID | 上轮问题与结论 | 已有处置 / 修后版本 | 本轮结果 | 当前证据 / 验证 | 剩余动作 |
| --- | --- | --- | --- | --- | --- |
| R3 §2 / R1 F-01 | Service Worker 写入绕过拦截；R3 已解决 | `e374f79`，当前实现未变 | **已解决** | 重读 [cdp.rs](../../tools/phoenix-ui/crates/browser/src/cdp.rs#L1090)：iframe/worker/sharedworker/serviceworker 子会话恢复前启用 Fetch；复用 R2 真浏览器反例验证，当前 CI 的 block_writes 测试补跨平台覆盖 | 无代码修复动作；V-8 单列 |
| R3 §2 / R1 F-02 | 静态根外符号链接可读取；R3 已解决 | `e374f79`，当前实现未变 | **已解决** | [serve.rs](../../tools/phoenix-ui/crates/cli/src/capture/serve.rs#L149) 仍先 canonicalize 并检查根边界，再读取；R2 T-4/E-2 的代码与条件未变 | 无 |
| R3 §2 / R1 F-03 | 派生图覆盖同名主图；R3 已解决 | `e374f79`，当前实现未变 | **已解决** | [mod.rs](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L698) 将主图、无效图、请求派生名一并预检；当前 CI 名称冲突测试与 R2 T-2 | 无 |
| R3 §2 / R1 F-04 | 窄元素标注 panic；R3 已解决 | `e374f79`，当前实现未变 | **已解决** | [derived.rs](../../tools/phoenix-ui/crates/cli/src/capture/derived.rs#L295) 与 [错误收集](../../tools/phoenix-ui/crates/cli/src/capture/mod.rs#L1050) 保持派生问题语义；当前 CI 小于 4px 元素测试与 R2 T-3 | 无 |
| R3 §2 / R1 F-05 | 旧格式部分重取丢失原工具身份；R3 已解决 | `1a58e6a`，当前实现未变 | **已解决** | [report.rs](../../tools/phoenix-ui/crates/cli/src/capture/report.rs#L37) 为沿用且缺逐图身份的条目补原顶层 tool，保留已有身份；追踪 mod.rs 旧报告读取、merge、脱敏及写出；复用 R3 T-1/E-1，当前 CI 的 5 项 capture::report 测试通过 | 无；不声称可恢复此前已经覆盖丢失的历史数据 |
| R3 §2–3 / V-6 | 原交接、走查与截图材料缺失；待核实 | 恢复原主/子会话、8 状态报告及图像 | **已解决** | M5 原始定位、主会话 `4c83…` 第 27 行交接和子会话 `aa66…` 第 165 行报告：无指定任务，实际自行选择操作；观察、推断、未知分开，未读源码或评审结论 | 保留历史版本边界 |
| R3 §2–3 / V-9 | 宿主处置、复走及交付缺原始材料；待核实 | 原 48 文件归档；当前 A5 iframe 分支补跑 | **已解决** | §4.2 原始日志核查、M6 逐场景原始定位、当前 A5 工具调用与实际图像；适用版本及包装器限制见 §3 | 不要求重跑未受影响场景 |
| R3 §2–3 / V-8 | 当前修复候选缺四平台及 Node18 证据；部分解决 | `0509ba6` / run `42808136237` | **已解决** | 四个 UI Native 与 Node18 全部成功；四平台实际截图及 capture 浏览器测试、制品身份已核实，见 §4.3 | 无 |
| R3 §2–3 / 生成物与范围限制 | 未逐字节重审全部 provider/live 生成物；范围外改动排除 | 本轮没有本计划实现增量 | **不再适用（作为本轮新增实现检查）** | Git 净差异仅四份 portal 文件；保留既有覆盖限制，不改称全部生成物已重审 | 无；不扩大范围 |

五项代码问题：5 项已解决，0 项仍存在，0 项回归。V-6/V-9 原材料缺口和 V-8 当前候选缺口均已解决；范围限制一项作为新增实现检查不再适用，原覆盖边界仍保留。

## 3. 待核实与覆盖限制

- 已选旧项没有剩余待核实或待决策动作；以下为证据适用范围，不扩写为全期无条件保证。
- 历史宿主场景使用原开发候选，相关 Phoenix 完工、走查、feature-demo 与 apply-ui-review 合同从 `55aa061` 到当前未变；review-ui 的增量为逐图出处抄录。旧单引擎材料支持未变的宿主行为，R3 双引擎实验和当前 A5 支持修后的身份/消费路径；没有声称当前引擎完整重跑 A1–A6。
- 原 V-9 临时项目及全部视频文件未恢复。会话中的实际工具结果、视频时长/播放检查、Write 报告和图像可以独立核对；完整历史项目的复跑能力仍不在恢复范围。A2 植入的无反馈先被主代理发现，走查发现→修复→复走的证据来自随后真实发现的滚动/标记问题，不能改写为 walker 发现该植入问题。
- 当前 A5 包装器继承 heredoc stdin，Python 启动脚本文本附加进宿主输入。原会话第 4 行及宿主回应可查，宿主未执行该脚本。此次只证明实际发生的 iframe 操作、工具出处与派发分支，不作为纯净提示词评测；后续若重跑须用 `stdin=subprocess.DEVNULL`。夹具的空白标题和窄屏提醒截断是演练页面问题，原宿主评级保留，不是本次 Phoenix 实现缺陷，也不用其“通过验收”评分推导本报告结论。
- 材料隔离由实际读取轨迹证明，不宣称文件权限隔离；A6 部分协作参数加密，采用可读事件、子会话和命令结果，未声称解密。
- 未重审 P1 全部原始实现、全部 provider/live 生成物、P2–P4 或用户排除的其他 skill。未进行发布、PRD 全量交付核查或业务产品 UI 验收。原始宿主包和本轮 CI 下载包仅本机保留，报告及台账尚未提交共享。

## 4. 覆盖与验证

### 4.1 版本、范围与一致性

- 仓根 `/Users/rockie/Documents/GitHub/xgent/skills`，分支 main；日期 2026-11-10（Asia/Shanghai）。原输入为本计划的 `$review-code`，本轮续接补证后修复效果复审。
- R3 target 为 `92ec5fc8fb35c2e182ea9a79fc5f920a66a45e46`；当前 target 为 `0509ba6c9012f5eb5b6a51507a28d10ad7a66601`。为正常推送无冲突合并了远端 `0b35470b506d5dfd0efb34f032ab43ce70f7d26e`。净差异仅 portal-dev-setup 的 SKILL.md / onebox.sh，以及 portal-external-app 的 SKILL.md / registration-and-onebox.md，均不纳入本计划评审。
- 延续 R3 排除项：software-copyright-docs/README、review-prd-dev-gaps/经验记录及无关 hook/安装变更。计划登记的 12 个不同实施/修复提交均为当前 HEAD 祖先，未因合并更换修复内容。
- index 为空，无本计划实现 dirty；已有计划、记录、状态索引和未跟踪文档仅作上下文。补证阶段更新 M6 与进度记录，评审阶段不修改实现、测试、计划契约或历史报告。当前主 agent 完成核查，没有另起独立 reviewer 进程。
- [源码指纹](../../local/phoenix-ui/evidence-craft-p1-20261110/r4-review-inputs/source-sha256.json) 是 tools/phoenix-ui、相关四项 skill、scripts、test/phoenix-ui 与 workflow 的 2221 个 tracked 文件按路径排序的 SHA-256 JSON；整体摘要 `7b8596ddf42fd703ac8b3cb9c31728cc1de2504a6ec3c45a7020d5c3a789fca7`。该集合用于一致性检测，不表示本轮逐文件重审。
- 未提交的计划、M2/M3/M5/M6 与 R3 输入已保存到 [本机文档快照](../../local/phoenix-ui/evidence-craft-p1-20261110/r4-review-inputs/documents-sha256.json)，不仅保留 hash；快照计划摘要 `adbaa3a8ccaa956fd9d672d4329c097db1eff025cbc74dee50ab4ba21e0270a3`，文档清单摘要 `8dfe27baadcf07bd6655c00f73e730a025fb24d00502b38f52af4eed0caa69b1`。快照保留 CI 收齐前的进度，最终 CI 另按 §4.3 直接核对；报告不反向纳入输入。
- 收尾核对源码指纹无变化，R3 摘要与上述原版本一致。补证后的计划快照/完成记录只回填 CI 事实与后续入口，契约未改；M6 增加本次实际结果，旧轮次判定保持可追溯。

### 4.2 宿主原始材料

- [原始清单](../../local/phoenix-ui/evidence-craft-p1-20261110/manifest.json) 的 48 个 gzip 副本逐一核对了解压后大小与 SHA-256；清单摘要 `9c5d0da6ce14dd20e6bce004355a11f89e3a29484f534df9229b4be8bcf9173d`。读投影只作定位，结论回到原 JSONL。
- A2：`7c047…` 第 195/197 行实际同轮派评审与 walker，第 326 行只重取 affected states，第 368 行提交含走查修复的批次；`5f154…` 第 451 行实际交付有原词 ship、走查范围与隔离目录。不是仅据 M6 的叙述。
- A3：`e21ee…` 第 275 行实际交付明确“未取证”“未完成 UX 走查”；原工具调用证明浏览器不可用。A4/A4b：核读实际 Write 报告与播放检查，前者 meta.environment 记录 capture，后者沿原流程且无安装提示，分别有 23.36s / 17.44s 视频结果。
- A5/apply：`5263…` 第 213/219 行使用原 walk.json 实际复走 desktop/mobile 及写拦截变体；`58ce…` 第 184/239 行修后交接没有任务提示、评审材料排除机器日志与走查。A5b 修后 `1b2e4f6a…` 第 347 行最终交付无 Phoenix/走查结论。
- A6：Codex `01a1c102…` 第 114/175 行 capture 实际成功，第 176/181 行开图/图像材料，第 251/252 行无主要操作流程的信息页交付没有走查结论。相关子活动仅完工评审与文档代理。
- 当前 A5：Claude 会话 `d1d83b38-25f8-47db-91ee-dc94987c6230`，显式代理后 exit 0，未换宿主。[原始核对及完整 fixture](../../local/phoenix-ui/evidence-craft-p1-20261110/a5-proxy-retry/verification.md) 对应 6 份 JSONL 与 161 个归档文件。U1 capture 6 张有效图逐图 binarySha256 为 `9155d97141ceb0019d6291e154f01e36745826130dd13239147d3cd6fd564e2e`；U2 由 Playwright 1.59.1 / Chrome 157.0.8103.40 实际进入 iframe，在 1440×900、390×844 完成填写和发布，得到清空输入、新条目及“提醒已发布”。采集与出处登记、judge 实际读取、两个 walker 交接和主代理复走均有原始轨迹；补证时已亲自查看两视口发布前后图，本轮条件未变复用图像核查。
- 本机 A5 构建的 469 个输入到当前候选逐项未变。历史代码检查与 R3 双引擎真实入口实验仅在其代码、调用契约和配置未变的范围复用；未为静态文档添加测试，也未重复运行本机聚合套件。

### 4.3 当前候选 CI

[Phoenix run 42808136237](https://github.com/XGENT-ai/skills/actions/runs/42808136237) 的 event=push，head_sha 为报告 target；2026-11-10 19:31:18–20:24:53+08:00，completed/success。详细原始日志、制品 ID/摘要、二进制及 VERSION.json 校验见 [V-8 核对](../../local/phoenix-ui/evidence-craft-p1-20261110/v8-ci-42808136237/verification.md)。

| 检查 | Job ID | 实际结果 |
| --- | --- | --- |
| UI Native linux-x64 | 129802470356 | success；capture=screenshot；capture.rs 8 passed / 0 failed / 0 ignored，158.82s |
| UI Native linux-arm64 | 129802470397 | success；capture=screenshot；capture.rs 8/0/0，274.80s |
| UI Native darwin-arm64 | 129802470446 | success；capture=screenshot；capture.rs 8/0/0，194.44s |
| UI Native darwin-x64 | 129802469957 | success；capture=screenshot；capture.rs 8/0/0，242.62s |
| UI Node 18 portable consumer smoke | 129827414743 | success；host.node=v18.20.8；capture=screenshot |

9 个 UI 制品的 ZIP SHA-256 均匹配 GitHub digest；四个平台 build-record 与 phases 均指向当前 target，全部六个阶段成功。直接计算 ZIP 内二进制及 VERSION.json 摘要，与各自冒烟记录一致；Node18 冒烟中的两项摘要匹配本次 linux-x64 制品。逐平台核读 capture.rs 的实际测试名、成功结果与耗时，并核对 5 项 capture::report 回归测试均成功。macOS 日志的 stdout/stderr 有交错，按实际测试名和结果关联，没有把下一测试文件标记前的文本切片误当成完整结果。四个 Diagram job 也成功，未扩大为本轮 Diagram 代码审查。

检查对应计划 V-8：四平台原生构建、完整运行时与实际 capture 截图，再由 Node18 消费同一 linux-x64 制品。完整 CI 是用户授权的该门槛，并非为文档修改扩大回归；本机没有重复运行同一套件。CI 原回执的 browserAcceptance/providerSessionAcceptance 字段不改写，宿主验收另由 §4.2 支撑。

## 5. 计划符合性

| 要求 / 原缺口 | 本轮判断 | 证据 |
| --- | --- | --- |
| WORK-F1 AC-9、WORK-F3 AC-3，逐图出处与部分重取 | 原反例已解决，保持成立 | F-05 当前源码、R3 T-1/E-1、当前 CI report 测试、A5 当前出处 |
| F-01–F-04 的写入、路径、输出名与派生错误边界 | 原反例保持关闭 | 当前源码、未变的 R2 实测、当前 CI 对应 capture 测试 |
| V-6、WORK-F4 交接与隔离 | 原材料缺口已解决 | 恢复的主/子会话、状态报告及图像 |
| V-9，WORK-F3/F4/F15–F17 宿主行为 | 原材料与 A5 分支缺口已解决，保留 §3 限制 | 原始 A1–A6 链路和当前 A5 补跑 |
| V-8、WORK-N2 P1 部分与 A-1 | 当前候选门槛已通过 | 同一提交四平台和 Node18 成功，实际截图及 capture 浏览器测试齐全 |

本轮不重新定义计划验收标准；未受影响的既有 V-1–V-5、V-7、V-10 检查沿用原评审与记录。跨期 NFR 终验和发布不计为本期新增缺陷。

## 6. 总体结论与下一步

验收判定：**通过验收**。

修复效果复审结论：**已解决**。五项代码问题保持关闭，V-6/V-9 的原始材料及当前 A5 分支、V-8 修复版四平台与 Node18 证据均已核实；未发现新增 P0–P2 或阻塞回归。没有本计划新增实现 diff，因此不另给空 diff 的 patch 正确性结论。验收限于本计划所选修复及旧待核实项，覆盖限制见 §3。

本计划本轮代码验收完成，无需空跑修复。历史 R1–R3 原文和判定保持不变；本判定不是发布或跨期需求终验授权。计划来自 PRD，可选下一步：

```text
$review-prd-dev-gaps docs/plan/PHOENIX-UI-EVIDENCE-CRAFT-P1.md
```
