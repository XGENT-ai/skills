# PHOENIX-DIAGRAM-P1 · PRD 交付覆盖核查

> 审查日期：2026-10-20（Asia/Shanghai）。评审对象 ID：`88cb4f52-4171-4daf-b298-1d9e1c538848`。
> 输入：[原计划](PHOENIX-DIAGRAM-P1.md)、[PRD v1.0](../PRD-PHOENIX-DIAGRAM.md)。模式：首次完整核查，范围为 **P1 的 23 条 AC**。
> 代码基线：`6b90f881985b01ead331ef1c107bb1d3652dde23`；实际候选验收源码 `c48d00824a7333af5af5a02f7149ef1f5618486e`。两者的产品源码、包资源、随包说明和验收脚本相同。本次没有修改上述输入或产品代码。

## 1. 结论与范围

**在下述固定候选、实际验收样本和证据范围内，P1 的 23 条 AC 已覆盖。A 明确遗漏 0 项；B 合理变更 1 项；C 需 Triage 0 项；待核实 0 项；待决 0 项；新增 follow-up 0 项。无需 gaps plan。**

本结论由 PRD 向下逐项核对，结合实现、原始运行材料和本次验证得出，未将计划的 7/7 或既有 AC trace 的 passed 直接当作结论。原计划完成前提成立：七份里程碑记录存在、完成记录均登记提交，M7 的同包收口证据可取回。本次只读重放生产聚合成功，四平台、精确 Node 18.20.8 和两宿主原档均通过原验证器。

验收对象是 **P1 候选**：同一 npm tgz `6544a703affe0d0c2d3c473f62c636de1399a990d39d4e44a49474a50554984f`，7,150,669 bytes。候选在线获取使用固定 loopback HTTP，离线使用匹配 release-dir；公开 HTTPS、registry 发行及两个 Phoenix 工具的联合发行仍未验收。该边界来自 [PRD §7](../PRD-PHOENIX-DIAGRAM.md#7-分期交付建议) 与[计划 §5.6、§8](PHOENIX-DIAGRAM-P1.md#56-构建四平台候选与同包消费)，不将其改报为本期遗漏。

本期只覆盖原有五类图。其余 30 条 AC 的 P2/P3 去向列于 §3；全部 11 条跨期需求继续按 PRD §7.3 保留终验期，未登记为整条已上线。本报告完成也不表示已公开发布。

## 2. 三类差异

### A · 明确遗漏：0

未发现证据足以确认、且尚未补齐的 P1 有效需求或验收缺口。

### B · 合理变更：1

#### G-01 · 候选引擎恢复按分发身份选择入口 · B

- **PRD 期望**：[WORK-F11/AC-5](../PRD-PHOENIX-DIAGRAM.md#L278) 要求缺失、错身份或不支持平台时普通命令不隐式下载，并给固定包版本的恢复指引；AC-1/AC-2 同时要求匹配资源、身份核对和离线安装。
- **实际交付**：`release` 使用固定 registry 版本；`candidate/development` 使用当前本地 npm bin 或已装 launcher，显式传入匹配 release-dir。未提供资源绝对路径时，宿主只询问路径并停止安装；已有安装授权不代替资源来源。见 [bootstrap setupCommand](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L485)、[SKILL 环境与停止](../../skills/phoenix-diagram/SKILL.md#环境与停止)及[包内说明](../phoenix-diagram.md#L21)。
- **证据链**：M4/M5 的 R1 修复 → [M5「R1 评审修复」](PHOENIX-DIAGRAM-P1.records/M5.md#r1-评审修复)明确记录用户确认按 distribution 区分，保留包版本 0.7.4、不 bump/publish → [计划 D7、§5.1](PHOENIX-DIAGRAM-P1.md#51-cli输入闸与布局) → M7 最终同包六场恢复专项 → 本次最终原生引擎下 bootstrap **20/20**，生产聚合重放通过。恢复专项原件见 §5 的 E6。
- **原因与备选取舍**：未发行候选不能假定 registry 的同名同版本包具有本候选的字节身份。机械指向 registry 会恢复另一份包或不可获得的资源；本地匹配入口加固定清单保留原要求的身份、显式安装和离线承诺。公开发布不是本期恢复的前提。
- **影响**：候选接收者需提供配套完整资源的绝对路径；普通生成、安装授权、四平台范围及完整性门没有放宽。
- **分类依据与处置**：已有可核实约束、可用替代行为、用户确认记录及实测，归 B；沿用 D-01，不重复 Triage。当前 SKILL、随包文档、D7/§5.1 及实际命令已一致。
- **Follow-up**：无新增任务。计划 §0.6 的“固定包版本指引”是较粗的摘要，执行细节以已修订 D7/§5.1 为准；它没有要求 candidate 改走 registry，不据此重新打开已解决的 F-06 或另建文档修复计划。

### C · 需 Triage 的实现差异：0

没有需要用户新增选择的范围、体验、权限、外部契约或验收取舍。联合公开发行的后续授权门保留原去向，不属于本次 C 项。

## 3. 完整需求覆盖账本

证据等级：**S**＝本次静态核实；**H**＝已发生且本次核对身份、原始成员或原日志的运行/审核；**V**＝本次执行的测试、只读重放或图像查看。E1–E6 的原件入口及验证范围见 §5。表中的“已满足”均受 §1、§5 的基线和样本限制。

### 3.1 P1 全部 23 条 AC

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| [WORK-F1/AC-1](../PRD-PHOENIX-DIAGRAM.md#L132) | 五类事实、节点、关系、方向保留 | M1–M4/M7；V-2–V-5/V-9/V-15 | M1/M2/M3/M4、M7 当前收口 | S：[五类 Rust 分派](../../tools/phoenix-diagram/src/layout/mod.rs#L30)；[事实与固定几何断言](../../tools/phoenix-diagram/tests/architecture.rs#L186)、[workflow 事实断言](../../tools/phoenix-diagram/tests/workflow.rs#L433)。H/V：E1–E4 的五类双合同、原生 oracle、宿主候选；本次另看五类暗亮图 | 已满足 | 新五类留 P2 终验 |
| [WORK-F1/AC-3](../PRD-PHOENIX-DIAGRAM.md#L134) | 未知类型、非法引用、重复身份、矛盾数据具体失败 | M1/M4/M7；V-2/V-6/V-15 | M1/M4/M7 | S：[输入闸](../../tools/phoenix-diagram/src/input.rs#L118)、[重复身份/端点/区间反例](../../tools/phoenix-diagram/tests/input_contract.rs#L228)。H/V：E1/E2 原生 input_contract 与失败回执；首门失败、后门 not-run，旧输出不算本次通过 | 已满足 | 无 |
| [WORK-F2/AC-1](../PRD-PHOENIX-DIAGRAM.md#L147) | 五类自然意图；只用已给事实，缺事实先问 | M5/M7；V-11/V-13 | M5、M7 宿主主场景 | S：[创作与澄清规则](../../skills/phoenix-diagram/SKILL.md#L11)。H/V：E4 两宿主各五自然意图及三缺事实；本次独读审批原题/完整候选，核对双条件、重提、复核回路、合并职责与六处撤回；提示变体限制见 §5 | 已满足 | 终验 P2；不推首次可靠率 |
| [WORK-F2/AC-2](../PRD-PHOENIX-DIAGRAM.md#L148) | flowchart、graph、sequenceDiagram、stateDiagram 语义转换 | M5/M7；V-13 | M5/M7 | S：[Mermaid 路由](../../skills/phoenix-diagram/SKILL.md#L36)。H/V：E4 每宿主四场景，原题及候选保留方向、条件、回路、return/async、waiting/recovery；E1 重核原会话与产物身份。按 PRD 不要求箭头样式复刻 | 已满足 | erDiagram/classDiagram 留 P2 |
| [WORK-F2/AC-4](../PRD-PHOENIX-DIAGRAM.md#L150) | 聚焦诊断修复；连续两轮不改善即停，不删事实 | M4/M5/M7；V-6/V-13 | M4/M5/M7 | S：[历史最佳及停止规则](../../skills/phoenix-diagram/SKILL.md#L58)、[诊断字段](../../tools/phoenix-diagram/src/diagnostic.rs#L22)。H/V：E4 repairable/stalled 原调用与候选快照；两次实际修改未改善后停止，重复 baseline 不冒充修复；未选越界会话仍保原 | 已满足 | 无 |
| [WORK-F4/AC-1](../PRD-PHOENIX-DIAGRAM.md#L177) | 四自动门分别列出，全门绑定才自动通过 | M4/M7；V-6/V-7/V-9/V-15 | M4/M7 | S：[单 snapshot finalize](../../tools/phoenix-diagram/src/delivery/transaction.rs#L213)。H/V：E1/E3 共 50 组五类双合同的四门及 candidate/HTML/config/engine；审核后仍是原 delivery/browser 身份 | 已满足 | 新五类留 P2 |
| [WORK-F4/AC-3](../PRD-PHOENIX-DIAGRAM.md#L179) | 诊断可操作；浏览器缺席、失败、视觉未做/失败分开 | M4/M7；V-6/V-7/V-13 | M4/M7 | S：[具体失败分支](../../tools/phoenix-diagram/tests/delivery.rs#L243)。H/V：E4 unavailable/failed/pending 原回执及 E3 负例，保留各自阶段与 incomplete；已有视觉 failed 原尝试仍保留 | 已满足 | 无 |
| [WORK-F4/AC-4](../PRD-PHOENIX-DIAGRAM.md#L180) | 变更、旧回执和失配拒绝；摘要/bytes/scope 可核 | M4/M7；V-6/V-9/V-15 | M4/M7 | S：[严格当前性](../../tools/phoenix-diagram/src/delivery/provenance.rs#L38)、[原身份 review](../../tools/phoenix-diagram/src/delivery/review.rs#L22)。H/V：E1/E3 身份变异拒绝、新 delivery 拒旧审核、summary 失败同记录重试 | 已满足 | 无 |
| [WORK-F5/AC-1](../PRD-PHOENIX-DIAGRAM.md#L193) | 主题、风格、缩放、label/id 查找与详情，不改事实坐标 | M4/M7；V-8/V-9/V-15 | M4/M7 Reader | S：[查找入口](../../tools/phoenix-diagram/viewer/node-finder.js#L13)、[camera](../../tools/phoenix-diagram/viewer/viewer-camera.js#L8)。H/V：E3 各 Reader 的八项真实操作、前后事实/几何快照与恢复；E5 当前单测 | 已满足 | 完整探索终验 P2 |
| [WORK-F5/AC-3](../PRD-PHOENIX-DIAGRAM.md#L195) | 首次 Still、主动 Live；reduce/hidden/print/export 保静态含义 | M4/M7；V-8/V-9/V-15 | M4/M7 | S：[首次偏好初始化](../../tools/phoenix-diagram/viewer/motion-governor.js#L678)。H/V：E3 每组八个 reduce/hidden 样本、四 print PDF、导出/恢复；E5 当前 Governor 单测 | 已满足 | 无 |
| [WORK-F6/AC-1](../PRD-PHOENIX-DIAGRAM.md#L209) | 五种真实非空格式、双主题 SVG、可用时复制 PNG | M4/M7；V-8/V-9/V-15 | M4/M7 实际导出 | S：[导出入口](../../tools/phoenix-diagram/viewer/export.js#L1248)。H/V：E3 每组 20 个 theme×mode×format 的签名/解码/尺寸，50 组共 1,000 次；真实 PNG 剪贴板和独立能力/权限分支 | 已满足 | 新五类/范围卡片留 P2 |
| [WORK-F6/AC-3](../PRD-PHOENIX-DIAGRAM.md#L211) | canonical 保完整图与风格，去临时叠层，导出后继续探索 | M4/M7；V-8/V-9/V-15 | M4/M7 | S：[私有 clone 清理](../../tools/phoenix-diagram/viewer/export-cleanup.js#L8)。H/V：E3 完整 SVG、stateRestoration、20 导出及 print；E5 当前 clone/状态隔离单测。未把范围卡片当完整图 | 已满足 | 无 |
| [WORK-F6/AC-4](../PRD-PHOENIX-DIAGRAM.md#L212) | 不支持/编码/权限失败准确反馈；空文件和晚到事件不报成功 | M4/M7；V-8/V-9/V-15 | M4/M7 | S：[编码及终态反例](../../tools/phoenix-diagram/tests/viewer/unit.test.mjs#L294)。H/V：E3 真实剪贴板失败与 PNG fallback；E5 **37/37**，含空 Blob、录制 error 后到事件、timeout、资源释放。故障 fixture 只证明相应分支 | 已满足 | 无 |
| [WORK-F11/AC-1](../PRD-PHOENIX-DIAGRAM.md#L274) | 四平台 npm 匹配资源/引擎，大小/SHA/probe，无 Rust 消费 | M5–M7；V-10/V-12/V-14/V-15 | M5/M6/M7 | S：[bootstrap 预检](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L457)。H/V：E1/E2/E3，同 tgz 四原生构建与消费、在线/缓存/离线原回执，应用与 npm 子进程 rustc/cargo 不可解析 | 已满足 | 候选范围；迁移/终验 P3 |
| [WORK-F11/AC-2](../PRD-PHOENIX-DIAGRAM.md#L275) | 完整离线资源零网络；provider/default/幂等/skip 保留 | M5/M7；V-10/V-11/V-14 | M5/M7 | S：[计划/写入分离](../../tools/phoenix-diagram/src/install.rs#L149)。H/V：E3 OS 网络隔离安装、E2 原生 install 及 E5 当前 bootstrap/安装器轻量用例；两真实 host 加载另有 E4，不扩称其他 host 已实测 | 已满足 | 无 |
| [WORK-F11/AC-5](../PRD-PHOENIX-DIAGRAM.md#L278) | 不隐式获取、匹配恢复、预检零写、写失败可恢复；沿用 D-01 | M4/M5/M7；V-10/V-11/V-14 | M4/M5 R1/R3、M7 | S：[按 distribution 恢复](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L485)。H/V：E3 缺资源/坏缓存/unsupported 拒绝且项目不变；E6 六恢复角色场景；E5 最终真实引擎 bootstrap20/20；E2 install 中断与恢复 | 已确认变更 | G-01 / D-01；无新增动作 |
| [WORK-N1/AC-1](../PRD-PHOENIX-DIAGRAM.md#L291) | 四桌面无横溢、裁切/主控件遮挡；纵向只准可读例外 | M4/M7；V-7/V-9/V-15 | M4/M7 | S：[browser probe](../../tools/phoenix-diagram/src/browser/probe.js#L14)、[Reader 布局](../../tools/phoenix-diagram/viewer/reader-layout.js#L10)。H/V：E3 50×10 viewport/theme 实测，含四桌面及窄屏；已有 160 图审核，本次抽看 12 图未发现相应缺陷 | 已满足 | 十类及语言终验 P2 |
| [WORK-N1/AC-2](../PRD-PHOENIX-DIAGRAM.md#L292) | 默认确切产物真实暗亮审核，截图不等于批准 | M4/M5/M7；V-6/V-9/V-13/V-15 | M4/M7 图像与 review-record | S：[实际看图合同](../../skills/phoenix-diagram/references/delivery-contract.md#浏览器与实际看图)、[审核绑定](../../tools/phoenix-diagram/src/delivery/review.rs#L22)。H/V：E1/E3 四平台 40 原交付审核后 complete；Node18 十组仍 pending/incomplete；E4 负面状态保真，本次实际图像查看另列 E5 | 已满足 | 无 |
| [WORK-N2/AC-1](../PRD-PHOENIX-DIAGRAM.md#L307) | 五类生成、布局、校验、交付在 Rust，Reader 可留 JS | M1–M4/M6/M7；V-1/V-9/V-12/V-14 | M1–M4/M6/M7 | S：input→[layout](../../tools/phoenix-diagram/src/layout/mod.rs#L30)→[render](../../tools/phoenix-diagram/src/render.rs#L937)→[delivery](../../tools/phoenix-diagram/src/delivery.rs#L25)；[launcher 只启动 native](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs#L533)。H/V：E1/E2/E3 匹配四原生引擎；Node oracle 仅维护测试 | 已满足 | 新五类 P2；发行身份终验 P3 |
| [WORK-N3/AC-1](../PRD-PHOENIX-DIAGRAM.md#L322) | 五类旧核心/身份/方向/标签/固定几何，原 JSON/HTML 不覆盖 | M1–M4/M7；V-2–V-5/V-9/V-13/V-15 | M1–M4/M7 | S：[无标记默认 legacy](../../tools/phoenix-diagram/src/input.rs#L148)、[sequence 默认几何反例](../../tools/phoenix-diagram/tests/support/remaining_sequence.rs#L473)。H/V：E2 双合同 oracle、E3 legacy 五组、E4 十份旧输入原字节；延期字段明确列出处置，不算全量兼容 | 已满足 | 3.1 十类核心 P2；延期字段/扩展终验 P3 |
| [WORK-N4/AC-1](../PRD-PHOENIX-DIAGRAM.md#L338) | 同引擎/输入/配置确定性；身份可核；输入输出 alias 拒绝 | M4/M7；V-6/V-9/V-15 | M4/M7 | S：[确定性测试](../../tools/phoenix-diagram/tests/delivery.rs#L68)、[普通/symlink/hardlink alias](../../tools/phoenix-diagram/tests/delivery.rs#L172)。H/V：E2 原生真实 FS；E3 50 组确定性字节比较及独立 receipt | 已满足 | 无 |
| [WORK-N4/AC-2](../PRD-PHOENIX-DIAGRAM.md#L339) | 校验/提交失败保旧字节；提交后失败如实部分通过，不借旧证据 | M4/M7；V-6/V-9/V-15 | M4/M7 | S：[首失败短路](../../tools/phoenix-diagram/tests/delivery.rs#L205)、[提交后 browser 失败](../../tools/phoenix-diagram/tests/delivery.rs#L243)。H/V：E2 故障/竞争；E3 每消费环境八项公开恢复、claimant/篡改/换父目录保护与审核重试；E4 各真实失败状态 | 已满足 | 其他恢复范围终验 P3 |
| [WORK-N5/AC-1](../PRD-PHOENIX-DIAGRAM.md#L353) | 内容不执行，安装后离线生成/检查/阅读，字体本地 | M1/M4/M6/M7；V-2/V-7/V-9/V-12/V-15 | M1/M4/M6/M7 | S：[文本转义](../../tools/phoenix-diagram/src/render.rs#L34)、[嵌入资源](../../tools/phoenix-diagram/src/render.rs#L15)。H/V：E3 恶意 literal 保留、sentinel 未执行、OS 禁网生成/strict、CDP 离线 Reader/导出与零远程资源请求 | 已满足 | 外部访问终验 P3 |

### 3.2 其余 30 条 AC：明确范围外并保留终验去向

以下仅核对分期边界，不宣称已经实现或验收。每行包含原始 AC ID；范围依据是 [PRD §7.2](../PRD-PHOENIX-DIAGRAM.md#72-分期矩阵)，计划 §0.5 明确排除，没有把未经批准的删减记作延期。

| PRD 需求/验收 | 本期边界 / 后续能力 | 计划与记录 | 覆盖状态 | 去向 / 终验 |
| --- | --- | --- | --- | --- |
| WORK-F1/AC-2 | erd/class/tree/timeline/waterfall 及同等闭环 | §0.5；无本期完成声明 | 范围外 | P2 / F1 终验 P2 |
| WORK-F2/AC-3 | 新五类及 erDiagram/classDiagram | §0.5；M5 只声明五类 | 范围外 | P2 / F2 终验 P2 |
| WORK-F3/AC-1、AC-2、AC-3、AC-4 | 固定仓库证据、托管来源、时间/时长事实 | §0.5；输入 deferredFields 不计交付 | 范围外 | P2 / F3 终验 P2 |
| WORK-F4/AC-2 | 新五类统一交付、showcase | §0.5；M7 只验原五类 | 范围外 | P2 / F4 终验 P2 |
| WORK-F5/AC-2、AC-4 | 十类筛选/Route/Reach/tree 折叠、概览/展示/深链接/键盘完整验收 | §0.5、§6.2；保留代码不当终验 | 范围外 | P2 / F5 终验 P2 |
| WORK-F6/AC-2 | 新五类导出与 Route/Reach 范围卡片 | §0.5、§6.3 | 范围外 | P2 / F6 终验 P2 |
| WORK-F7/AC-1、AC-2 | 内置/官方品牌标识与固定外部资源 | §0.5、M1/M6 许可范围 | 范围外 | P3 / F7 终验 P3 |
| WORK-F8/AC-1、AC-2 | 主动本地预览、最新有效版本与停服 | §0.5；无本期完成声明 | 范围外 | P3 / F8 终验 P3 |
| WORK-F9/AC-1、AC-2 | 两架构比较及成对失败恢复 | §0.5；无本期完成声明 | 范围外 | P3 / F9 终验 P3 |
| WORK-F10/AC-1、AC-2、AC-3 | 替换 archify、调用方路由、最终十类描述 | §0.5；M5 保留旧入口与调用方 | 范围外 | P3 / F10 终验 P3 |
| WORK-F11/AC-3、AC-4 | 受管 archify 迁移、跨版本升级及用户主动回滚 | §0.5、§5.5；P1 新安装中断恢复不冒充迁移 | 范围外 | P3 / F11 终验 P3 |
| WORK-N1/AC-3、AC-4 | 十类统一可读性及 en/zh-CN/zh-TW/es/ko 语言质量 | §0.5、§6.2；P1 只声明实际范围/fallback | 范围外 | P2 / N1 终验 P2 |
| WORK-N2/AC-2、AC-3 | 新五类 Rust（P2）；最终发行身份与许可（P3） | §0.5、§1.3、§8；P1 来源/许可前提另已核 | 范围外 | 分别 P2/P3 / N2 终验 P3 |
| WORK-N3/AC-2、AC-3、AC-4 | 3.1 十类核心（P2）；品牌/来源/废弃展示字段及扩展约定（P3） | §0.5、§3.1；延期指针保持可查 | 范围外 | 分别 P2/P3 / N3 终验 P3 |
| WORK-N4/AC-3 | 比较/预览/安装回滚/导出取消等完整恢复边界 | §0.5、§8；P1 已有保护不替代此终验 | 范围外 | P3 / N4 终验 P3 |
| WORK-N5/AC-2 | 完整外部访问/品牌/公开来源安全与更新约束 | §0.5、§8；P1 本地安全及无隐式更新另已核 | 范围外 | P3 / N5 终验 P3 |

### 3.3 场景、UAT 与横向边界

| PRD 场景/UAT | 本次核实 | 后续去向 |
| --- | --- | --- |
| S1 / UAT-1 | E1–E3：四平台同包、无 Rust、在线/离线安装、五类生成/阅读/导出；公开发行未做 | 新五类 P2；联合发行另按计划 §8 |
| S2 / UAT-2 | E2/E4：五类自然意图、四 Mermaid、旧 JSON、缺事实澄清及兼容字段边界 | 新五类/参考输入 P2；品牌及完整字段处置 P3 |
| S3 / UAT-3 | 仅 WORK-N5/AC-1 的内容注入与离线边界 | 仓库/时间依据 P2，来源安全 P3 |
| S4 / UAT-4 | E2–E5：首失败短路、旧文件保留、提交后部分状态、错身份/旧证据拒绝、真实图审 | 新五类重验同门槛 |
| S5 / UAT-5 | E3/E5：五类阅读、Still/Live、暂停/恢复、真实导出及默认视觉审核 | 十类探索/语言/卡片 P2；品牌 P3 |
| S6 / UAT-6 | 仅 P1 新安装、provider/预检、恢复提示及当前输出事务保护 | 旧安装迁移/回滚、入口替换 P3 |
| S7 / UAT-7 | 本期不验预览或比较 | P3 |

PRD §2.2、§4 和 §5.3 的横向边界也已核：Rust 正常生成不调用 Node 图解 renderer；npm/浏览器 JS 的职责保留；源码以冻结内容身份而非虚构上游 Git commit 固定；核心 oracle 和上游源码不进入消费路径；实际许可证随资源保存；普通生成不下载、预览或改个人安装；旧 archify 与文档调用方留到 P3。NG1–NG6 仍为原明确排除项。没有将性能、无限输入规模、任意宿主成功率或 Windows 发行添加为新门槛。

## 4. Triage 与决策账本

待决问题卡：无。本轮没有向用户请求新授权或取舍。

| 决策 ID | 关联 G-ID | 用户选择与理由 | 确认日期与来源 | 新目标/验收边界 | 跟进/延期/豁免去向 |
| --- | --- | --- | --- | --- | --- |
| D-01（既有决定，本报告映射） | G-01 | 按 distribution 区分恢复，候选保持本地匹配身份；不 bump/publish | [M5「R1 评审修复」](PHOENIX-DIAGRAM-P1.records/M5.md#r1-评审修复)明确记录用户确认，计划 D7 已承接；源记录没有给出原确认会话的精确日期，本轮于 2026-10-20 读取，未补造日期 | release 固定 registry；candidate/development 当前入口 + 配套 release-dir；显式授权、SHA/size/probe、缺路径先问即停 | 已实现并验证，无新增任务；不变更公开发行门 |

本报告未把 R1–R4 代码评审作为本技能的前轮报告，也不沿用其 F-ID 冒充 G-ID。它们只作为修复来源与证据导航；本轮核查全部 P1 AC。

## 5. 审查基线、验证与证据限制

### 5.1 可取回基线

| 对象 | 版本 / 范围 |
| --- | --- |
| 工作树 | 分支 `codex/phoenix-diagram-reader-facts-20261017`；HEAD `6b90f881985b01ead331ef1c107bb1d3652dde23`；审查开始时 dirty，但无产品代码改动。原有改动为两个台账文件、R1–R4 代码报告及四份评审状态页，均保留 |
| PRD | v1.0，54,200 bytes；SHA-256 `e099e47858cc9efeecdda26e1786242a2393cf50d0198538fc0817b144e4263b`；P1 按 §7.2，23 AC；全文共 53 AC |
| 原计划 | 122,719 bytes；SHA-256 `29d9770abf93fb57e2f29cb95b59d384559c36360acf991b2595150430df46b1`；正文与七份记录均在 HEAD 可取回 |
| 完成记录 | [M1](PHOENIX-DIAGRAM-P1.records/M1.md)、[M2](PHOENIX-DIAGRAM-P1.records/M2.md)、[M3](PHOENIX-DIAGRAM-P1.records/M3.md)、[M4](PHOENIX-DIAGRAM-P1.records/M4.md)、[M5](PHOENIX-DIAGRAM-P1.records/M5.md)、[M6](PHOENIX-DIAGRAM-P1.records/M6.md)、[M7](PHOENIX-DIAGRAM-P1.records/M7.md#当前收口检查点)。M7 为 210,923 bytes / `cf3f57d786dce9ee3a82e64622eb36da004fb3ee6b2b6379e0ee254720013668`；旧进行中段落按各自 cohort/日期保留，不覆盖最终收口 |
| 候选源码 | commit `c48d00824a7333af5af5a02f7149ef1f5618486e` / tree `9e89e0f4a91f1874cc4f92a41bd577996693434f`；本次核对其到当前工作树的产品目录 diff 为空 |
| 当前生成输入 | 146 个 canonical tuple；SHA-256 `336ed1d3de2f036c1e2cc620771a3240d0d95b328de8dc68adc844c0752a0e3f`，本次重算并与最终 candidate/source-inputs.json 原字节相等；19 个 bundle 文件，manifest SHA `4805c3a4e35280b7ea3f79bc3b4e757f304c9e7af8fab92b7a125c30ba2865e8` |
| 实际运行环境 | 本次 macOS arm64、Node 25.2.1；bootstrap 使用最终候选 darwin-arm64 引擎。历史四原生平台及 Node18 各保留自己的环境、浏览器、安装、网络与阶段回执；本次没有重新运行这些远程场景 |

### 5.2 原始证据与本次动作

| 编号 | 实际来源 | 本次核实及结果 |
| --- | --- | --- |
| E1 | [生产聚合输入](../../local/phoenix-diagram/m7-ci/39353915898/final-production-aggregate.attempt1.input.json)、[原输出](../../local/phoenix-diagram/m7-ci/39353915898/final-production-aggregate.attempt1.output.json)、[聚合器](../../scripts/aggregate-phoenix-diagram-candidate.mjs) | **本次重新执行**原聚合器，使用只读 Actions API 核 run/job/artifact 身份，重核原 ZIP/tar/成员、原始调用及产物；exit 0，status=pass / handoff=passed / diagnostics=[]。输出在本次 `/tmp/phoenix-prd-dev-gaps.6J9zrr/aggregate-replay.json`，与原输出的机器事实一致。不重新生成、不重绑复制文件的 inode |
| E2 | [CI 39353915898](https://github.com/XGENT-ai/skills/actions/runs/39353915898)、[四平台原测试日志索引](../../local/phoenix-diagram/m7-ci/39353915898/native-test-summaries.all-four.json)、[静态引文与适用性](../../local/phoenix-diagram/m7-ci/39353915898/final-static-applicability.json) | 原 run 的十个 jobs 成功；本次对四份原日志校 SHA 并重新统计，每平台 21 组、267 pass / 0 fail / 2 ignored。两项 ignored 的真实浏览器范围由 E3 承担。本次核对 163 处实现/测试引文的整文件和指定行 SHA 均相符，并读关键输入、五类事实断言、交付/当前性、导出、安装与恢复段落 |
| E3 | [五消费环境导航](../../local/phoenix-diagram/m7-ci/39353915898/current-consumer-evidence-index.json)，其中四平台完整原 ZIP 及 Node18 原 ZIP | E1 逐成员重核；四平台各十组 complete，Node18 十组自动 pass、视觉 required-pending/incomplete。共 50 组、500 个 viewport/theme 测量、1,000 次真实导出；Reader/clip/print/motion/offline/content/recovery 与审核拒绝各保真实 scope。上述是既有实际运行，本次未启动浏览器重跑 |
| E4 | [主会话台账](../../local/phoenix-diagram/m7-ci/39353915898/provider-session-ledger.final.json)及其两份最终 provider-evidence.zip / provider-acceptance.json | E1 重核实际新会话、已装资源读取、原生调用/结果、安装、candidate/产物与逐场 review。两宿主各 22 场景有相应证据。本次另读缺条件、自然审批、Mermaid sequence、停滞等原题和判断材料，独立逐路径核对 Codex 审批候选与暗亮原图。选中结论不删除 10 个未选会话，也不推出首次成功率 |
| E5 | 本次 `npm test`；`PHOENIX_DIAGRAM_BIN=<最终 candidate/release-dir/phoenix-diagram-darwin-arm64> node --test test/phoenix-diagram/bootstrap.test.js`；`node --test tools/phoenix-diagram/tests/viewer/unit.test.mjs`；图像读取 | 全部 exit 0：Node 474 项，451 pass / 0 fail / 23 明确 skip；Python 22/24/2/13 全 OK；bootstrap **20/20、0 skip**；Viewer **37/37、0 skip**。本次实际查看 12 张原图：linux-x64 的 legacy-architecture、new-workflow、new-sequence、legacy-dataflow、new-lifecycle 各 1440×900 light/dark，加 Codex 自然审批同窗口 light/dark；未见横向裁切、标签/主控件遮挡或语义缺失。解包图逐字节核原截图身份；不声称本次重看全部 160 图。日志在同一 `/tmp/phoenix-prd-dev-gaps.6J9zrr/` |
| E6 | [最终同包六恢复场景](../../local/phoenix-diagram/m7-ci/39216914041/recovery-specialty.review.json)、[同包连接证据](../../local/phoenix-diagram/m7-ci/39353915898/same-package-cohort-linkage.json)、[最终 AC trace](../../local/phoenix-diagram/m7-ci/39353915898/ac-trace.final.json) | 恢复场景保两宿主的未授权不装、缺路径问后停、提供路径后匹配恢复三类实际结果；记录 SHA `76cd3056aebe14fb8bc40986371a5674f0a5f857416ea8907cea74b2eef990da`。旧/新 run 的宿主资源以相同 tgz 和 21 个相同 harness 成员连接，不迁入旧 CI 的失败结论。AC trace 的 23/46 为导航和原验收判断；本报告重新建立 §3 的 PRD 账本 |

本次聚合执行命令（凭据由本机既有登录态仅传给只读 API 子进程，未写入报告或证据）：

```sh
node scripts/aggregate-phoenix-diagram-candidate.mjs \
  --input local/phoenix-diagram/m7-ci/39353915898/final-production-aggregate.attempt1.input.json \
  --output /tmp/phoenix-prd-dev-gaps.6J9zrr/aggregate-replay.json
```

### 5.3 证据限制与非本期事项

- **待核实清单为空**：未发现阻止本期 AC 判断的缺失原件或未定事实。静态、历史实际运行、本次只读重放、本次单测和本次图像抽查在上表分别标注，没有把它们互相代替。
- 本次没有重新跑完整 `test:diagram` / `test:phoenix`、远程四平台生成、宿主模型会话或真实浏览器交互。采用最终四平台原生日志及当前消费原档，并核对代码/契约/输入未变。原有 Phoenix UI runtime 只保留 M7 已说明的未变范围，未声称本次全量通过。
- 自然审批场景附加的通用步骤提醒、318B JSON 读取指导及 Codex 173B 事实/路径自检提醒已亲读其原记录；未提供业务事实、拓扑、坐标或删改验收门。两宿主各 22 个选中场景证明注明条件下的路径成立，不证明首次、默认或任意模型的可靠率。原 54 个主会话、10 个未选尝试及六恢复场景保留。
- 原 CI39216914041 的图审窗口失败仍是失败；本次采用独立成功的 CI39353915898。同包连接只证明字节/成员等价，不把失败 run 改成成功。
- 真实原 ZIP、宿主原档、图片和记录位于 gitignored 的 `local/` 及本机 `~/.local/share/xgent/`。本机可访问且本次重放已验证；跨机器需要保全原件，Git 中的摘要或本报告不能替代原档。临时复核输出不作为唯一证据来源。
- R3/R4 的 core-runner 计时偶发观察不改变产品交付结论，本次完整轻量检查没有复现。联合发行阻断继续按计划 §8/§11 留待发行前处理；本报告不作新的合并或发行决定。

## 6. Follow-up 与 gaps plan

| G-ID / D-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | gaps plan 任务或排除去向 |
| --- | --- | --- | --- | --- |
| G-01 / D-01 / WORK-F11/AC-5 | 无新增工作；保留已实现的按 distribution 恢复 | 匹配入口/资源、不隐式下载、缺路径停步、原身份恢复；现有证据满足 | 无 | 已由 M4/M5 修复及 M7 六恢复场景承接，不重复建计划 |
| P2/P3 的 30 条 AC | 本次不规划 | 按 §3.2 及 PRD 原 AC | 各期原前置 | 后续期次；不是未决 C 或本期遗漏 |

已检查当前文档/台账中同名 gaps 与相关承接记录，未发现本期尚未承接的确定增量。**无需 gaps plan**；未调用 dev-plan，也未执行任何补齐工作。

报告和项目状态索引按本轮独立对象登记，原 PRD、原计划、完成记录、既有代码评审及产品代码保持原样。下一步如继续终局范围，可单独编制 P2 计划；本期覆盖核查已完成，跨期终验仍按 PRD。

```text
$dev-plan docs/PRD-PHOENIX-DIAGRAM.md P2
```
