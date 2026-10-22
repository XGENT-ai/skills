# PHOENIX-DIAGRAM-P1 · 开发计划评审

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md) | SHA-256 `904977ac3331368dfbf1793e97e37cd342d378d4851d7212052d85ebd721b0d2` | 全文；P1 的 11 条工作号、23 条 AC、M1–M7、V-1–V-16；初次完整评审 |

- 评审 ID：`599a4664-e089-4c42-901f-0d870d39e9f5`。
- 调查日期：2026-10-16（+08:00；开始基线为 2026-10-15T17:06:30Z）。
- 关联需求：[PHOENIX-DIAGRAM PRD](../PRD-PHOENIX-DIAGRAM.md)，SHA-256 `e099e47858cc9efeecdda26e1786242a2393cf50d0198538fc0817b144e4263b`。PRD 是背景与约束，不是本报告的修订目标。
- 本轮只读评审原计划与代码；未实现、改稿、改进度、安装、发布或写入 SPMS。报告及已启用的状态台账由主 agent 单写。

## 1. 确认问题

### [P2] F-01 · 补齐实际视觉审核登记之后、不重新交付的完整状态汇总路径

- **定位：**计划 [§3.2:259–263](PHOENIX-DIAGRAM-P1.md#L259)、[§5.1:303–308](PHOENIX-DIAGRAM-P1.md#L303)、[§5.2:327 / §5.3:331](PHOENIX-DIAGRAM-P1.md#L327)。关联 WORK-F4、WORK-N1；M4/V-6、V-9，最终包 M7/V-15。
- **问题：**实际图像审核发生在 `finalize` 采集截图并返回之后。`review-record` 当前只规定验证、登记审核结果，没有规定刷新当前汇总或返回完整交付状态；计划却要求最终 `finalize` 回读已登记审核并给出 `deliveryStatus: complete`。但每次 `finalize` 都重新 deliver、产生新 `receiptId`，浏览器和视觉记录又绑定原 delivery/browser 身份。按现合同再次运行会使刚登记的审核不再属于当前交付，不能得到承诺的完整状态。

  | 顺序 | 已有证据 | 当前合同的结果 |
  | --- | --- | --- |
  | 首次 `finalize` | Delivery R1、Browser B1、截图 | 自动通过，但视觉仍未完成 |
  | 实际查看图像并 `review-record` | Visual V1 绑定 R1/B1 | 记录被接收；未定义重新汇总后的完整状态 |
  | 再次 `finalize` | deliver 生成 R2 | V1 不属于 R2；默认封存目录还可能先报 conflict |

  换 `--out-dir` 只能解决证据目录冲突，不能把 R1 的审核变成 R2 的审核；HTML 摘要相同也不能豁免 receipt 绑定。
- **证据：**计划 `:260–261` 定义 BrowserReceipt/VisualReviewRecord 的精确绑定；`:263` 要最终 finalize 回读审核；`:303–304` 区分四门 finalize 与登记命令；`:327` 封存浏览器/图像/审核证据；`:331` 明确每次 deliver 产生新 receiptId。参考实现 [finalize.mjs:307–323](../../local/archify-main/archify/bin/finalize.mjs#L307) 同时比较 artifact 身份与 `deliveryReceiptId`，不支持用相同 HTML 偷换运行身份。问题是未来接口合同的矛盾，不是声称尚未实现的 Rust CLI 已发生运行故障。
- **最小修订：**明确一个**不重新 deliver、不更换 delivery/browser receipt 身份**的收口动作。优先可让 `review-record` 在输出锁内复核输入、HTML、四门与审核绑定后，条件更新当前 summary，并返回自动/视觉/完整状态；也可另设明确的只读汇总入口。二选一写清即可，不必同时增加多个命令。保留审核真实性、封存证据和当前性检查，不能把自动截图改成视觉 passed。
- **复核：**改稿时核对 §3.2、§5.1–§5.3、skill 交付说明及 V-6/V-9/V-15 使用同一闭环。实现时通过真实公开入口执行 `finalize → 实际图像审核 → review-record → 当前汇总`，确认 delivery/browser 身份未变化且完整状态可达；候选、HTML、截图或审核身份变化后必须失效。再次 finalize 应仍被视为新交付，不得自动继承旧审核。
- **置信度：高。**相互冲突的生命周期规则均有明确文本；主 agent 已逐段交叉核实。**阻塞：M4 的对外回执/审核后收口合同及 M7 对应验收；不阻塞独立的来源冻结与布局工作。**

### [P2] F-02 · 为输出发布中断后的条件恢复定义消费者可调用入口

- **定位：**应承载入口的计划 [§5.1:293–308](PHOENIX-DIAGRAM-P1.md#L293)；内部恢复与失败承诺在 [§2.1:209](PHOENIX-DIAGRAM-P1.md#L209)、[§5.2:323–327](PHOENIX-DIAGRAM-P1.md#L323)、[§7.1:418](PHOENIX-DIAGRAM-P1.md#L418)；kill 后恢复验收在 [§9.4:474](PHOENIX-DIAGRAM-P1.md#L474)。关联 WORK-N4；M4/V-6、V-9，最终包 M7/V-15。
- **问题：**计划允许退役旧 public 文件后、新文件可见前存在短窗口，并要求 kill 后恢复。此时进程可能来不及执行异常处理，旧字节虽然留在私有备份，public HTML 却可能缺失。计划安排了内部 `recover_publication`，但 CLI 全表没有输出恢复命令，也没有规定重跑 deliver/finalize 如何先定位、验证并恢复该次事务。消费最终 tgz、没有 Rust/源码的用户，只有备份位置，没有可执行的恢复路径。`doctor` 明确不修复；§5.5 的“下次 install 先恢复”只针对安装事务，不能替代输出恢复。
- **证据：**计划 `:209` 只定义模块函数，`:323–324` 明确提交窗口与 recovery-required，`:474` 要求 kill 后恢复。参考 [recover-output.mjs:1–40](../../local/archify-main/archify/bin/recover-output.mjs#L1) 实际提供单独命令，接收私有恢复目录并区分 recovered/absent/preserved；[atomic-output.mjs:1793–1801](../../local/archify-main/archify/renderers/shared/atomic-output.mjs#L1793) 明确恢复函数用于进程崩溃、必须指名目录、不是自动扫描器。已有内部 helper 不等于打包后的用户入口。
- **最小修订：**选择一种包内可执行方案：例如增加 `phoenix-diagram recover-output <private-recovery-directory> --json`；或明确下一次针对同一输出的显式 deliver/finalize 在重新生成前执行条件恢复。须定义 kill 后仅凭包内说明定位持久记录的方法、所有权/原父目录/备份身份验证、幂等结果及退出码。未知记录、篡改、后来 claimant 和用户修改一律保留并停止；恢复文件不能凭空重建通过回执。同步 CLI、文档、模块调用与检查。仅限**本次输出发布事务恢复**，不扩展到 P3 的安装迁移、跨版本 rollback。
- **复核：**改稿时确认公开入口明确调用内部恢复机制，V-9 覆盖真实文件系统机制，V-15 覆盖最终包可调用。实现后在无 Rust 的 tgz 消费环境，旧 public 退役后/新 public 发布前 kill，再按包内说明经公开入口恢复原字节；重复恢复幂等。插入新 claimant、修改备份/记录、替换父目录时须拒绝覆盖并保留现有内容。仅从 Rust 测试调用私有函数不算用户恢复通过。
- **置信度：高。**已全文检索恢复相关段落，并核对 CLI、失败模式与验收要求。**阻塞：M4 的输出恢复接口及 M7 的消费恢复验收；不阻塞其他已稳定合同下的独立开发。**

两项均定 P2：它们是具体的状态汇总/操作接口缺口，尚无证据表明五类原生核心路线不可行或必然造成数据丢失；但涉及对外证据与恢复合同，必须在对应实施前修订。没有确认的 P0/P1，也没有为凑数量列入的格式或偏好问题。

## 2. 待核实项、后续证据与非阻塞澄清

### 2.1 已正确安排在实施期取得的证据

以下不是新增计划缺陷；不得提前当成通过，也不因当前没有未来产物而追加开工阻塞。

| 所需证据 | 影响范围 | 提供角色 / 最晚确认点 |
| --- | --- | --- |
| 冻结归档可恢复；29 份 schema 样本、Unicode 与未知 `$ref` 的锁版本实验 | 输入/兼容和可跨机器移植 | M1 实施者；V-1/V-2、移植前。本轮重算了源集合摘要，未独立重跑全部 schema 实验 |
| no-clobber、竞争、I/O 与 kill 恢复的真实 FS 结果 | WORK-N4 / 发布安全 | M4 output/delivery 实施者；V-6/V-9。F-02 的公开入口验收还须在 V-15 补齐 |
| Reader 实际 DOM、暗亮截图、图像审核、导出文件签名/可解码性和离线零请求 | WORK-F4/F5/F6、WORK-N1/N5 | M4/M7 集成人；V-9/V-15。本轮没有 Phoenix 页面浏览器实测 |
| 四平台 runner/权限、同一 tgz 的原生消费、Node18、Claude/Codex 全新项目真实会话 | 同包分发与 host 主路径 | M6/M7 集成人；V-12–V-16。环境缺失时按计划保留受影响验收阻塞 |

### 2.2 不采纳为缺陷的候选

`ENGINE.json` 为 VERSION 逐字副本、VERSION 又带 `bundleSha256`，容易在实现时误把派生文件放入 bundle 哈希。可选地在 §3.3/§5.6 补一句排除范围与派生次序，便于实现者防错；**本轮不据此判自引用、四平台 bundle 必然不同或新增阻塞**：计划 `files[]` 只定义允许范围，没有要求覆盖 ENGINE；§5.6 已规定装配时在 staging tree 生成最终 ENGINE/VERSION。

现有同构 UI 在 [render.mjs:82–108](../../tools/phoenix-ui/scripts/render.mjs#L82) 明确先 hash bundle、再注入 ENGINE；[install/plan.rs:163–170](../../tools/phoenix-ui/crates/install/src/plan.rs#L163) 拒绝普通 ENGINE 条目，[installer.rs:936–945](../../tools/phoenix-ui/crates/install/src/installer.rs#L936) 从已验证 VERSION 派生安装。已有可直接承接的路线，不需要为此新增架构决策门或照抄 UI manifest 格式。

## 3. 覆盖摘要与证据边界

### 3.1 基线与实际分工

- 仓库：`XGENT-ai/skills`；branch `main`；代码 HEAD `4a65a1525f9ed03c775a0e18935901e1edbb773f`；**当前 dirty 工作树为调查事实源**。未提交项为本主题 PRD/其评审/计划、状态资料与 experience；本轮没有把它们视为已共享的提交成果。
- 开始与写报告前再次核对：HEAD、计划和 PRD 摘要未变；相关运行代码没有并发改动。计划自报 `Ready`，实施进度 `0/7`、未开始；本报告不改这些原文。
- 主 agent 实际启动四个独立只读 reviewer（继承主模型）：facts `47d1809c`、architecture `689adf93`、schedule `74099a57`、delivery `b14ecb87`。前三个并行独立调查，随后启动 delivery；初轮未提供预设缺陷，返回后只交换相关证据。主 agent 亲读所有确认问题的计划段落及原始代码、去重并裁定严重度/建议状态；四个 reviewer 均已结束且无遗留进程。

| 视角 | 实际覆盖与结论 | 限制 |
| --- | --- | --- |
| facts | 旧/新输入与布局参考、来源身份、Reader/证据语义、npm/provider/安装分发主张；确认 F-01、提出并交叉核实 F-02 | 未把计划自述实验当成本轮重跑结果 |
| architecture | 独立 Rust 核心、双输入合同、JSON Schema 离线边界、output/receipt 生命周期、Reader 导出与独立四平台分发；主要路线可行，F-01 须修 | ENGINE 自引用候选经反证撤回，不作阻塞理由 |
| schedule | 11 条需求/23 条 AC 双向映射，局部检查/阶段收口、来源/同包/host 证据链、提交点与并行边界；无独立编排 finding | 映射存在不代表其中接口语义已正确；F-01/F-02 是被映射机制自身的缺口 |
| delivery | 安装→入口→创作/旧 JSON→四门→实际审核→离线阅读/完整图导出→失败恢复；确认两个消费者闭环缺口 | UI 证据等级为**文档推演 + 参考代码核对**，不是原型走查或实际 Phoenix 浏览器验收 |

原计划将窗口阅读、键盘焦点、暗亮/Still、能力禁用、导出反馈与资源清理纳入设计和 V-8/V-9/V-15；未因个人审美要求重设计。没有 Phoenix 可运行页面的事实本身不构成 Blocked。

### 3.2 本轮实际检查

| 检查 | 结果与解释 |
| --- | --- |
| `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P1.md .` | 54 个路径 token，22 个 MISSING。逐类核对为明确新增产物/安装后输出（tools/skill/bin/dist/scripts/test/workflow/docs 等）、新模板/probe 的相对路径、明确不新建的 `agents/openai.yaml`，及协议中条件兼容的 `docs/project-status/protocol.md`；实际已有 `.xgent-ai/sdlc/protocol.md`。未确认虚构的已存在复用路径；这些提示不作为 findings |
| `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P1.md .` | `ERROR 0 / WARN 1`，0/7、7 行里程碑、0 行完成记录。警告是 dirty 树但计划未计入 tracked diff；计划本身为 untracked，不能据此推定已有里程碑完成或漏回写 |
| `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P1.md` | `ERROR 0 / WARN 11`；11 个 WORK 自定义 ID 已人工对照 PRD §7.2，不是悬空需求 |
| 按 §1.3 重算固定来源集合 | 498 个普通文件、无非普通条目；compact UTF-8 JSON 清单 SHA-256 为 `4f892b0c68fef5d21788264ba6a43fe9e1d377d3c2d5b558f6aa346784238395`，与计划一致。未生成/解包归档，不冒充 V-1 全部通过 |
| `git status`、HEAD、计划/PRD SHA-256 | 写报告前与开始基线一致；报告落盘后再回读并核对链接及版本 |

未运行 npm 全仓测试、Phoenix runtime、prepare/install、故障注入、真实浏览器/provider/四平台 CI 或公开发布；没有代码变更需要用运行结果证明。确认问题的裁定未依赖外部服务或网络资料，证据来自本地计划合同和可读取的同构实现。schema 库的全面运行兼容性仍由计划 V-2 验证。一次过宽的仓库规则目录搜索达到时限后已清理，随后改用 tracked 文件与限定目标目录检查；未遗留后台命令，不影响上述证据。

## 4. 映射与实施编排

### 4.1 需求到交付

核对范围为 PRD [§7.2:450–469](../PRD-PHOENIX-DIAGRAM.md#L450) 与计划 §0.1、§9、§10。**11 条工作号 / 23 条 AC 均有设计、任务和检查映射，无独立断链 ID。** 不重复原计划的完整映射表；需修的是以下两条已映射链的行为合同：

| 链路 | 当前实施/验收落点 | 本轮所需调整 |
| --- | --- | --- |
| WORK-F4 + WORK-N1 → 四门/真实审核 → 完整状态 | M4，V-6/V-7/V-9；M7，V-15 | F-01：增加明确的审核后非重交付收口动作及真实接口检查 |
| WORK-N4 → 安全发布/条件恢复 → 消费者恢复 | M4，V-6/V-9；M7，V-15 | F-02：明确内部恢复机制的公开调用、持久记录定位及最终包验证 |

其余映射覆盖五类与 legacy（V-2–V-5）、host 路由/缺事实/停滞（V-13）、阅读/完整图导出（V-8/V-9/V-15）、安装/同包（V-10–V-14）、离线与安全（V-2/V-7/V-9/V-12/V-15）。本期不替跨期需求终验，也不改变 P2/P3 排除项。

### 4.2 依赖、波次与资源

现有拓扑基本成立，无需新增里程碑或默认拆子任务文档：

`来源冻结/V-1、输入 scene/V-2 → 各类型布局/V-3–V-5 → 阶段 A/V-9 → 包与安装接缝 → 同 SHA 四平台原生制品 → 唯一 tgz/release-dir → 最终包/host/V-13–V-16`

这是依赖拓扑，不是有依据的工期或节省比例。细分解锁和并行可保留计划 §10：

| 波次 | 前置及可并行工作 | 共享负责人 / 资源 | 汇合验收 |
| --- | --- | --- | --- |
| 1 · M1 | 先固化来源，再稳定 input/scene/诊断合同 | 主集成人单写共享合同/Cargo 接入 | V-1/V-2 |
| 2 · M2/M3，M4/M5 接缝准备 | 合同稳定后按图型文件并行；依赖共享几何的任务只等所需 helper；M4 可做 Reader/output/browser 接缝，M5 可做 skill/bootstrap | geometry/render/check、公共入口/测试注册单写；**F-01/F-02 先解除，再冻结受影响的 M4 对外接口** | V-3–V-5 及各局部检查；不提前认定端到端通过 |
| 3 · 阶段 A 与安装集成 | 五图齐备后 M4 收口；M5 正向安装用真实 binary，不用 fake 成功替代 | Chrome/profile/临时项目/cache 隔离或串行；总入口/package/锁文件由主集成人处理 | V-6–V-11，其中 V-9 是阶段 A 集中收口 |
| 4 · M6 原生构建与装配 | 固定提交和包输入；四 host 可并行；四份来源与 bundle 一致后仅装配一次 | 平台独立 host；manifest、最终候选与 loopback 端口由单一装配负责人冻结 | V-12；唯一 tgz/release-dir |
| 5 · M7 | 各平台消费可并行；同机 host/图像审核若共用会话、端口或项目则串行 | 主集成人维护候选身份与证据索引，单写计划/完成记录 | V-13–V-16；F-01/F-02 最终包断言纳入 V-15 |

计划已区分 M4 开发制品与 M7 最终包，两次阶段收口不是对同一身份无差别重跑。四平台 diagram 候选可显式跳过 UI，真实两工具联合安装接缝另验，不能把它夸大成 UI 四平台发行已验收。同 SHA CI 允许提前提交候选代码、验收修复后重取受影响证据，不存在“全部验收后才能首次提交”的循环。公开 npm/R2 写入、个人/活跃安装与 P3 迁移仍不在本期授权范围。

## 5. 总体结论与修订顺序

**是否需要修改：需要。**

**建议状态：Blocked。** 计划自报 `Ready`；其需求范围、主要技术路线与实施编排基本完备，但 F-01/F-02 尚未闭合 P1 的对外证据/恢复合同，不能把整份计划认定为 Ready。这里的 Blocked 限定为这两个合同以及依赖它们的 M4/M7 工作，不否定来源冻结、输入和独立布局等前期工作的可执行性。评审通过也不会证明功能已经实现、浏览器/四平台验收通过或允许公开上线。

修订顺序：

1. **先修接口与状态机：**选择审核后非重交付的收口动作（F-01）；选择消费者输出恢复入口及持久记录定位规则（F-02）。无需修改已确认的产品原则、授权和 P1/P2/P3 边界。
2. **再同步调用链：**把所选合同写入 receipt/CLI/模块调用、skill/消费文档，以及 V-6/V-9/V-15；复用原 M4/M7，不为此增加新的阶段或真人门。
3. **最后定向复核：**确认图像审核后的完整状态不依赖新 receipt，且 kill 后用户能仅凭包内说明调用条件恢复；确认新动作不覆盖用户文件、不复用过期审核、不伪造通过回执。对应缺口解除后，再依据剩余证据判断 Ready，无需为已成立的 schedule 结论重写整份映射。

可直接复制的下一步：

```text
/apply-doc-review docs/plan/PHOENIX-DIAGRAM-P1.review.md
```

本轮没有自动启动修订或实施。台账将分别保留计划原自报状态、此次建议状态、所审 hash/范围与独立评审 ID；不把本轮建议写回原计划。

## 6. 修订处置与复核

- 处置日期：2026-10-16（+08:00）；模式：按报告修订并定向复核，首次 apply。
- 来源：本报告评审 ID `599a4664-e089-4c42-901f-0d870d39e9f5`；追加本节前的报告 SHA-256 为 `4ff86ee868f217694f4e3a62becc3df79eb73714c3d30d99c65a4465f1801120`。上文原问题、证据、所审基线及原 Blocked 建议完整保留。
- 唯一修订目标：[PHOENIX-DIAGRAM-P1](PHOENIX-DIAGRAM-P1.md)。修前 SHA-256 `904977ac3331368dfbf1793e97e37cd342d378d4851d7212052d85ebd721b0d2`，与所审版本一致；修后 SHA-256 `e896a9ef7e477871f2c303fbe3353a8874bedcc20f5e8e0dba75a9466c874d7b`。
- 代码基线：HEAD `4a65a1525f9ed03c775a0e18935901e1edbb773f`，dirty 工作树；开始前已有本主题未跟踪文档及 experience/SDLC 资料，均作为现有内容保留。相关运行代码无本次修改；未回退、实施或提交。
- 范围：F-01、F-02，及 §2 的证据边界、§4 的关联映射。没有既有处置需续接。PRD 为只读约束，SHA-256 仍为 `e099e47858cc9efeecdda26e1786242a2393cf50d0198538fc0817b144e4263b`。

### 6.1 逐项处置

下表的问题 ID 均归属上述评审 ID，不是另一次独立评审。

| 问题 | 上次处置 / 版本 | 核实结论与依据 | 最终处置 / 修改位置 | 本次复核结果与剩余动作 |
| --- | --- | --- | --- | --- |
| F-01 | 无；原所审版本 | 成立。亲读修前 §3.2、§5.1–§5.3：审核绑定 delivery/browser 身份，而再次 finalize 必须重新 deliver。参考 [finalize 的 receipt 校对](../../local/archify-main/archify/bin/finalize.mjs#L307) 同时要求 artifact 与 deliveryReceiptId 匹配，不能以同 HTML 摘要消除运行身份差异。 | **已修订并复核**。D5、[§3.2](PHOENIX-DIAGRAM-P1.md#32-回执与失效合同)、[§5.1–§5.3](PHOENIX-DIAGRAM-P1.md#51-cli输入闸与布局) 选择由 review-record 在输出锁内核实、封存并条件刷新原 summary，返回自动/视觉/完整状态；保持原四门、delivery/browser 身份。同步 §0.6、§1.2、§2.1、§5.6、§6.1、§8、V-6/V-9/V-15、§9.3、M4/M7 及 §11。 | **已解决（文档合同）**。正向路径可在同一交付下达到 complete；身份/范围变化须拒绝，重复 finalize 不继承审核，封存后 summary 写失败可重试同记录。已逐处回读并搜索清除原“最终 finalize 回读审核”的口径。实现及真实图像/公开入口验证仍由 M4、M7 执行，未宣称已通过。 |
| F-02 | 无；原所审版本 | 成立。亲读修前 CLI 全表、output 模块与 kill 验收，只有内部恢复函数，没有消费者入口；[参考恢复 CLI](../../local/archify-main/archify/bin/recover-output.mjs) 显式接收私有目录，[恢复函数](../../local/archify-main/archify/renderers/shared/atomic-output.mjs#L1793) 校对记录/目录/备份与原槽位，再 no-clobber 恢复。install 的自动恢复不能替代此行为。 | **已修订并复核**。D8、[§5.2](PHOENIX-DIAGRAM-P1.md#52-确定性提交与恢复) 和 CLI 明确 recover-output → output::recover_publication；定义发布前同步的私有持久记录、kill 后只读定位、当前用户/目录/父槽位/备份绑定、幂等状态与退出码。同步 §0.5 的期次边界、§1.2 包内 docs 文件、§2.1/§3.2、§5.6/§6.1、§7/§8、V-6/V-9/V-15、§9.3、M4/M7 及 §11。 | **已解决（文档合同）**。仅凭包内说明/--help 可定位并调用；未知记录、后来 claimant、备份或记录绑定变化及替换父目录均保留并停止；恢复不补成功回执，只接受 output-publication 记录，不接收 install journal。公开 CLI、真实 FS kill 和无 Rust 最终 tgz 消费检查均已排入 M4/M7，尚待实施取证。 |

### 6.2 决策、未采纳项与证据边界

- 两项均采用报告建议中的最小路线。F-01 复用原 review-record，不另增汇总命令；F-02 采用显式单事务恢复入口，避免重跑生成时隐式选择事务。这些是落实既有 WORK-F4/WORK-N1/WORK-N4 的本地接口选择，不改变产品目标、实际审核要求、安装授权、公开发布门或 P1/P2/P3 边界；无新增待用户拍板项。
- §2.1 的来源归档/schema 全量回归、真实 FS、Reader/导出/离线、四平台/Node18/host 会话证据继续在原执行节点取得。新增断言纳入原 V-6/V-9/V-15 与 M4/M7，不新增里程碑，不提前声称来源冻结或任何实施验收完成。
- §2.2 的 ENGINE 哈希次序属于已明确不作 finding 的可选防错说明，本次未加入 ENGINE 排除范围与派生次序的额外说明。已亲读 UI [先 hash bundle 再注入 ENGINE](../../tools/phoenix-ui/scripts/render.mjs#L82)、[普通 manifest 排除 ENGINE](../../tools/phoenix-ui/crates/install/src/plan.rs#L163) 及 [从 VERSION 派生安装](../../tools/phoenix-ui/crates/install/src/installer.rs#L936)，未据此新增自引用或四平台阻塞判断。
- 关联 PRD 无需改动；尚未存在的新 skill、消费说明、CLI/测试均由计划的原里程碑交付，本次没有提前修改或创建这些实施产物。实施进度及完成记录逐字保留：0/7，尚未开始。

### 6.3 修订后结论与实际检查

确认问题 **2 项已修订并复核**；无需修改 0、不采纳 0、待核实/待决策/待修订 0。文档层面的两个合同缺口均已解除，修后计划判定 **Ready**；原计划的 Ready 字段保留，§11 定级理由已按修后合同更新。该判定只说明可以按计划实施，不是新一轮完整独立评审通过，也不是实现、产品验收或上线完成。

| 本次检查 | 实际结果与解释 |
| --- | --- |
| `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P1.md .` | 55 个路径 token，MISSING 22。逐项核对：计划明确新增产物、安装后的 .agents 目录、tools 下相对模板/probe、明确不新建的 agents/openai.yaml，以及条件兼容的 docs/project-status/protocol.md；实际协议在 .xgent-ai/sdlc/protocol.md。未发现虚构的既有复用路径，未为消除提示提前建文件。 |
| `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P1.md .` | ERROR 0 / WARN 1；0/7、7 行里程碑、0 行完成记录。警告仍来自计划为 untracked、脚本没有取得 tracked plan diff；已比对修前副本，实施进度全文未变，不能把此次改稿计作里程碑进展。 |
| `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P1.md` | ERROR 0 / WARN 11；自定义 WORK ID 已对照 PRD §7.2：11 条、23 条 P1 AC 与计划账本一致。M1–M7、V-1–V-16 全部保留原 ID。 |
| `npm test` | 退出 0；Node 272 项，264 通过、8 跳过、0 失败；Python 四组分别 22/24/2/13 项，均 OK。跳过项不算通过证据；轻量测试不证明 Phoenix runtime 或计划验收。 |
| 定向回读、链接与 diff | 核对正向审核闭环、失效/写失败重试、持久定位/公开恢复/幂等/保护链；清除目标内旧收口口径。真实本地链接有效，实施协议内 FEATURE.records/M1.md 是原有代码示例；PRD 和实施进度未改，原报告正文保留，无运行代码变更。 |

未运行 npm run test:phoenix、原生构建、浏览器/图像/导出、故障注入、四平台或 provider 会话；本次只修订并复核文档，未执行 M1–M7，也未取得 V-6/V-9/V-15 的未来运行证据。

下一步（仅建议，不自动启动）：

```text
/goal 按 docs/plan/PHOENIX-DIAGRAM-P1.md 开发
```
