# Phoenix Diagram P2 · PRD 交付覆盖核查

> 审查日期：2026-10-23（Asia/Shanghai）。输入：[P2 原计划](PHOENIX-DIAGRAM-P2.md)、[PRD v1.0](../PRD-PHOENIX-DIAGRAM.md)。
> 模式：首次完整 P2 核查；不是 R5 代码复审。原 PRD、计划、完成记录与产品代码保持只读。

## 1. 结论与范围

**存在明确遗漏，尚不能宣称 P2 的 PRD 覆盖全部完成。A 明确遗漏 3 项，B 合理变更 3 项，C 需 Triage 0 项；待决 0 项，阻塞分类的待核实 0 项；确定 follow-up 4 项。**

本轮从 PRD 向下核对 P2 的 14 条 AC，同时复核本期终验的 WORK-F1/F2/F3/F4/F5/F6/N1 的 P1 条件，并核对其他已交付能力的持续保障。覆盖账本包含 P1/P2 共 37 条 AC，P3 的 16 条另列去向。按 AC 计，33 条在注明的证据范围内已满足，4 条部分满足；它们关联 3 个独立缺口，不按失败样本数重复计数。

- G-01：参考接受的六个 SystemV 夏令时时区在最终 Rust 引擎中仍被拒绝，影响 WORK-F1/AC-2、WORK-N3/AC-2。
- G-02：新的 --out-dir 遇到浏览器缺席或启动失败时，浏览器回执写入失败掩盖真实原因；影响 WORK-F4/AC-3。
- G-03：真实 ERD 创作会话先补造未确认的关系约束，再声称完整交付；原验收将这类会话判 passed，影响 WORK-F2/AC-3。

原计划的 8/8 完成、R5 的通过验收与本轮结论分别对应不同检查范围。四平台、Node 18、本机收口和两宿主证据确实存在，**本轮生产聚合重放仍为 pass / passed**；但聚合证明证据身份与既定判据成立，不能证明其未覆盖的合法输入均兼容，也不能修正原语义审核的误判。G-01 在 M1/M3 被记录为已知缺口，G-02 在 M7 被记录为既有产品观察，G-03 的原始候选和答复也已保留；它们不是本轮新引入的回归。

验收对象是候选源码 c1703a95db1fa08ffe0c7145d91806a12fda2378、同一 npm tgz 02029422b771c59ffcb49262f4de9f9997e009dd135dbd555cf4102b485a7b0f。公开发行、活跃安装替换、品牌、预览、比较和旧安装迁移继续属于原定后续范围。N2/N3/F11/N4/N5 的终验期仍为 P3。

已生成独立增量计划 [PHOENIX-DIAGRAM-P2-gaps](PHOENIX-DIAGRAM-P2-gaps.md)，状态 **Ready**、0/4 尚未开始；承接三项补齐及一项验证同步。本轮不执行修复。

## 2. 三类差异

### A · 明确遗漏：3 项

#### G-01 · 六个有效参考时区被当作非法输入 · A

- **PRD 期望**：[WORK-N3/AC-2](../PRD-PHOENIX-DIAGRAM.md#work-n3-保留旧输入并允许兼容扩展) 承诺固定参考十类有效 JSON 的核心数据可被消费；[WORK-F1/AC-2](../PRD-PHOENIX-DIAGRAM.md#work-f1-生成十种语义完整的图解) 要求 timeline 保留时间语义。并未豁免参考接受的 SystemV 时区。
- **实际交付**：将冻结 payment-incident.timeline.json 的 meta.timezone 分别改为 SystemV/AST4ADT、SystemV/CST6CDT、SystemV/EST5EDT、SystemV/MST7MDT、SystemV/PST8PDT、SystemV/YST9YDT。冻结 Node renderer 六次均 exit 0 并各生成 811,607 bytes HTML；最终候选引擎六次均 exit 1，报 timeline/invalid-timezone，定位 /meta/timezone。显式选择 phoenix-v1，未靠合同缺省制造差异。
- **证据链**：原计划 M1/M3、§5.5 → [M1 偏差](PHOENIX-DIAGRAM-P2.records/M1.md#偏差已就地修订计划正文)、[M3 偏差](PHOENIX-DIAGRAM-P2.records/M3.md#偏差) 已承认六个缺口 → [zone](../../tools/phoenix-diagram/src/format/time.rs#L17) 只接受现有 IANA/别名/固定偏移 → [format 测试](../../tools/phoenix-diagram/tests/format.rs#L101) 特判参考接受而 Rust 拒绝的 SystemV/*DT 为预期结果 → 本次最终引擎与冻结 renderer 对照。
- **实际验证**：[复现脚本](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/reproduce.mjs)、[结果](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/reproduction-results.json)。工具版本为 Node 25.2.1 / ICU 77.1 / tz 2025b，最终引擎二进制 SHA-256 为 ae6224cf3e7a7c77dfdad918e56e72240891979dc88771be510d2303e801fe31。
- **影响**：有效的旧时间线无法进入布局、阅读和导出；换成相似城市时区可能改变历史/未来夏令时，不能算保语义迁移。
- **分类依据**：依赖中没有这些名称不构成兼容删减的批准。恢复既有有效输入不需要新的产品取舍；本次还已验证不新增依赖的原生实现路线，见 gaps 计划。没有查到豁免或延期决定，归 A。
- **处置 / Follow-up**：补六个时区的准确历史转换及后续规则，保留作者名称与原 JSON；去掉“拒绝即通过”的测试例外，增加端到端与季节切换反例。由 FU-1 承接。

#### G-02 · 新证据目录使浏览器缺席被误报为执行失败 · A

- **PRD 期望**：[WORK-F4/AC-3](../PRD-PHOENIX-DIAGRAM.md#work-f4-完成绑定确切产物的校验与交付) 要求浏览器未安装、执行失败、视觉未做及失败分别呈现，并提供真实阶段、证据和可采取的修复。
- **实际交付**：使用合法 minimal.tree.json，finalize tree … --out-dir <尚不存在的单层目录> --json。OS 沙箱隐藏 Chrome 时，前三门 pass，内嵌 browser receipt 为 unavailable，但 browser-check 阶段、status、automatedStatus 均被改成 failed，exit 4，顶层仅有 output/parent-resolution / ENOENT。仅预建同一个证据目录，结果即变为 unavailable / exit 5，诊断 browser/unavailable。以 /usr/bin/false 模拟真实启动失败也出现同根因：新目录只报 ENOENT，预建后能保留 browser/launch。
- **证据链**：原计划 M4/M7/M8，终验复核 F4 全部 AC → [M7 继续动作第 4、7 项](PHOENIX-DIAGRAM-P2.records/M7.md#继续动作) 记录该问题但未承接修复 → [browser/mod.rs](../../tools/phoenix-diagram/src/browser/mod.rs#L275) 在 discovery 前尚未准备证据目录，launch 失败时又清理新建目录 → [finalize](../../tools/phoenix-diagram/src/delivery/transaction.rs#L340) 先 seal_browser、后收集 browser diagnostics；seal 失败覆盖阶段并跳过原诊断收集。
- **实际验证**：[四组对照结果](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/reproduction-results.json) 中 browserCases，包含 unavailable/failed × 新建/预建目录；未改用户浏览器或安装环境。使用已读的 [OS 隐藏浏览器规则](../../local/phoenix-diagram/m7-claude-prep/browser-unavailable-os-preflight/browser-unavailable.sb)。公开说明的[快速路径](../phoenix-diagram.md#L33)正是新的 evidence-01，不要求调用者先 mkdir。
- **影响**：宿主会把浏览器问题误判成目录问题；M7 原会话确实记录了按 ENOENT 建目录再试。真实缺席信息仍能在嵌套 receipt 找到，不能说完全消失，但汇总阶段和主要诊断与它冲突。
- **分类依据**：是有效调用的失败表达缺口。P1 起已存在、宿主能解释或最终没有误报成功，均不能豁免 F4 本期终验条件。没有恢复原要求以外的取舍，归 A。
- **处置 / Follow-up**：让证据目录与回执生命周期一致，保留实际浏览器状态及诊断；错误目标、路径冲突、执行失败继续准确失败，不伪造截图或视觉通过。由 FU-2 承接。

#### G-03 · ERD 作者把待确认约束先画成事实，语义验收未拦截 · A

- **PRD 期望**：[WORK-F2 行为边界及 AC-3](../PRD-PHOENIX-DIAGRAM.md#work-f2-从意图与已有材料完成创作) 要求候选来自给定材料并保留键、基数和关系含义；其事实原则不只适用于旧五类。[现行 skill](../../skills/phoenix-diagram/SKILL.md#L11) 明确要求缺少影响含义的规则先问，答复前不写 candidate；“默认/待确认”不能替代事实。
- **原材料**：v11h 的 intent-erd 两宿主原题给出四张表、列、FK、联合 PK，以及客户可无订单、每订单至少一条明细、每条明细对应一个商品。没有说明 orders.customer_id 是否可空，也没有说明商品是否允许一条明细都没有。
- **实际交付**：两份完整候选都把订单所属客户这一端画为恰一、商品对应明细端画为 0..N，卡片也复述后者。Claude 的原生最终答复明确承认“有两处需要你确认，是我按常规理解定的”，列出商品 0..N 与 customer_id 不能为空，并称四门和视觉通过、交付完整。Codex 同样输出两项确定约束并称 complete，没有提出缺口。这不是仅有机器默认值，而是可见基数与业务说明已经形成了断言。
- **证据链**：原计划 M7/V-11 的只用给定事实与 ERD 基数 → [M8 的 v11h 重跑](PHOENIX-DIAGRAM-P2.records/M8.md#评审修复后的重做) → 当前记录每宿主 intent-erd 被原 reviewer 判 passed → 本轮独立读取原 prompt、完整 candidate 和原生最终答复，逐个核对未给规则。取回原件的 ZIP 身份、会话 ID、prompt/candidate 摘要和答复切片均已核对，保存在 [原件核查快照](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/host-erd-source-review.json)。Claude 会话 4f49b9fc-f2a1-4e83-8c5a-34f38592085e；Codex 会话 01a1643e-8234-7941-8eb0-02d7912f8553。
- **影响**：读者会把未经确认的可空性和最小参与数当成数据库事实；图片、键和自动门均通过，也不能证明该语义正确。原 reviewer 的 passed 与本轮原件核查结论不一致，二者均保留，不改写原档。
- **分类依据**：原请求只是作图，没有让作者拟定规则。要求未来作者先向其实际用户澄清，直接恢复已有原则；不需要让本次审查用户代替测试材料的作者决定这些业务事实，归 A。
- **处置 / Follow-up**：将“最大基数、最小参与数、FK 可空性分别取证”落实为 ERD 的具体决策提示；只有材料有缺口时才问，不能对已明确的 Mermaid 端点或 SQL PK 重复询问。验收增加原题先澄清、无 candidate 的反例；另用明确补齐两项事实的独立正例验证完整交付。修正验收不能继续把“交付后请确认假设”计为事实通过。由 FU-3 承接。

### B · 合理变更：3 项

#### G-04 · 参考布局失败改为保语义修复与可操作诊断 · B

- **PRD 期望**：F1/F2/F4 要求事实完整、能按具体诊断修复；没有要求复制参考布局算法或其失败集合。
- **实际交付**：按原计划 D12，仅 phoenix-v1 schema 2 的 workflow 基础布局失败后尝试端口分散、外通道、交叉光晕等修复；已通过的几何及 legacy-2.17 保持。整图撤 pin 与文字容量修法先整图重编验证，超长内容保留到卡片，不删业务语义。
- **证据链**：[D12](PHOENIX-DIAGRAM-P2.md#02-决策表)、[M7 继续动作第 7 项](PHOENIX-DIAGRAM-P2.records/M7.md#继续动作)、[workflow 行为测试](../../tools/phoenix-diagram/tests/workflow.rs)、E1/E2 的最终引擎和宿主记录。
- **依据与影响**：冻结参考对同样首稿也会失败；改动保留事实、质量阈值与停滞原则，并有 2026-10-21 的既有用户裁定，映射 D-01。属于内部实现适配，保留。
- **Follow-up**：无新增动作；不能用本项吞掉 G-03 的未确认业务规则。

#### G-05 · 四平台自动消费、本机完整审核与受约束宿主复用 · B

- **PRD 期望**：交付结论真实绑定确切文件；默认视觉审核不被截图生成代替。未要求每个 CI 消费产物都标完整。
- **实际交付**：ADR-2 保留 CI 产物视觉 required-pending；同包 darwin-arm64 做 review-record 完整闭环，另外三平台留截图观察。D13 只在 skill bundle 相同的条件下，重跑受影响宿主行，其他行保留原生记录，并用最终引擎逐字节重放主图 SVG/失败码；Reader 详情由本机浏览器覆盖。
- **证据链**：[ADR-2、D13、§9.3](PHOENIX-DIAGRAM-P2.md#93-收口环境证据复用与留存) → M6/M8 → [聚合实现](../../scripts/aggregate-phoenix-diagram-candidate.mjs#L369) → 本轮用 GitHub 连接器读取当前 run/jobs/artifacts，再实际调用 aggregateCandidate，E1 为 pass/passed、diagnostics=[]；E2 的文件摘要全部一致。
- **依据与影响**：有明确成本与恢复条件、原始证据和本机完整审核；D13 既有用户确认见 D-02。保留该证据组合，不把它扩成所有平台均做了完整视觉闭环。本项只确认取证和复用方式，**不等于原语义 reviewer 的判断必然正确**，G-03 已单列。
- **Follow-up**：无现存独立动作。gaps 若修改 skill bundle，就不满足同 bundle 复用前提，必须按新计划重新验收，不能借本项沿用旧宿主结论。

#### G-06 · 固定画布高度适配合理，但旧测试未同步且未进入维护入口 · B

- **PRD 期望**：N1/AC-3 要求十类可读，N3/AC-2 要求参考JSON可交付；不要求Reader适配决策逐字节复制冻结版。
- **实际交付与依据**：[M7继续动作第2项](PHOENIX-DIAGRAM-P2.records/M7.md#继续动作)为解决五份有效参考JSON的浏览器门失败，将固定画布的高度适配扩展到phoenix-v1的contain/legacy/空reader-fit，保留作者几何、可读下限与排除条件。实现见[reader-layout.js](../../tools/phoenix-diagram/viewer/reader-layout.js#L36)；E1/E2的全部参考JSON浏览器证据支持其产品目标，属于合理内部适配。
- **遗留验证差异**：本轮直接执行[reader-layout.test.mjs](../../tools/phoenix-diagram/tests/viewer/reader-layout.test.mjs#L164)时，wide legacy and other contracts retain their exact frozen Reader decisions在contract=null的固定画布上失败：当前正确进入高度适配，冻结版没有进入。该断言也把phoenix-v1固定画布列在应完全相同的范围内，已与M7规则过期。[日常原生测试入口](../../scripts/test-phoenix-diagram-native.mjs#L48)列出unit/explore/catalogs/evidence和export，但未列本套件，因此既有test:diagram通过并不与本次失败矛盾。[M2的R2修复局部回归](PHOENIX-DIAGRAM-P2.records/M2.md#L54)也已记录同一失败，并因其不属于V/npm/CI入口而未处理；本轮确认仍存在。
- **分类与影响**：运行期适配有需求依据和实测，不应为旧断言回退产品。保留适配，并补齐与之匹配的行为测试和维护接线；本轮未把失败测试改成通过或归因于环境。
- **Follow-up**：FU-4：拆分真正保持冻结行为的排除/宽图场景与本期有意改变的固定画布场景；对新路径验证源SVG不变、可读下限、溢出声明及返回能力，将完整套件加入原生维护入口。只同步既定行为，不新增Reader产品能力。

### C · 需 Triage：0 项

没有尚需新产品选择的差异。未向用户询问是否修复已经明确的要求，也没有用默认选项或超时取得批准。

## 3. 完整需求覆盖账本

证据等级：D=文档自述；S=本轮静态代码核对；H=既有真实运行/会话原件；V=本轮实际执行或实际查看。E1–E7 的版本、方法与限制在 §5 定义。“已满足”限定于这些基线和实际样本，不代表任意输入、线上发行或真人成功率。

### 3.1 P1/P2 已交付范围：37 条 AC

| PRD 需求/验收及定位 | 本期范围/已批准变更 | 计划与里程碑 | 完成记录 | 实际行为与证据等级 | 覆盖状态 | G-ID/去向 |
| --- | --- | --- | --- | --- | --- | --- |
| WORK-F1/AC-1，§3 F1 | 旧五类事实与方向持续保持 | M2/M3/M6/M8；V-3/4/7/13 | M2/M3/M6/M8 | S：Rust 十类分派；H/V：E1/E2 十五例与原五类 oracle/语料 | 已满足 | G-04 的适配保留 |
| WORK-F1/AC-2，§3 F1 | 新五类完整事实、同等阅读导出 | M1–M3/M6/M8；V-1/3/4/7/8 | M1/M2/M3/M6/M8 | S/H：列/键/外键目标、类成员/关系、树、时间/跨度检查；V：E2/E5；六个参考时区不能生成，E4 | 部分满足 | G-01 / FU-1 |
| WORK-F1/AC-3，§3 F1 | 未知类型、非法引用/重复身份/矛盾具体失败 | M1/M6/M8；V-1/7 | M1/M6/M8 | S/H：input_contract 与语义门；E1/E2 首门失败、后门未运行；没有用旧文件冒充成功 | 已满足 | 无 |
| WORK-F2/AC-1，§3 F2 | 原五类意图、缺事实先问 | M7/M8；V-10/11 | M7/M8 | S：SKILL 澄清与语义规则；H/V：E1/E3 五意图、missing-date/amount/condition 原记录 | 已满足 | 不外推为新 ERD 也通过 |
| WORK-F2/AC-2，§3 F2 | 四类原 Mermaid 语义 | M7/M8；V-11 | M7/M8 | H/V：E1/E3 flowchart/graph/sequence/state，拓扑/方向/消息审核记录 | 已满足 | 无 |
| WORK-F2/AC-3，§3 F2 | 新五类意图、ER/class Mermaid | M7/M8；V-11 | M7/M8 | H：各场景原件；V：E3 两份自然 ERD 补造最小基数/可空性，原 passed 不足以关闭事实要求 | 部分满足 | G-03 / FU-3 |
| WORK-F2/AC-4，§3 F2 | 定向修复、两轮停滞后停 | M7/M8；V-11 | M7/M8 | S：现行历史最佳计数；H/V：E1/E3 repair/stall，授权 x=171/172 两轮未改善后停，未删事实 | 已满足 | G-04；无增量 |
| WORK-F3/AC-1，§3 F3 | 十类固定仓库/提交/有效行；断言由作者负责 | M4/M6/M7/M8；V-5/9/11 | M4/M6/M7/M8 | S：evidence.rs 与 repository-authoring；H/V：E1–E3 十类 git 用例、web 来源和宿主逐行语义审核 | 已满足 | 原生工具只证明来源可核对性 |
| WORK-F3/AC-2，§3 F3 | 缺来源/脏文件明确限定 | M4/M6/M8；V-5/9/11 | M4/M6/M8 | S：worktree_matches 和三态；H/V：E1–E3 dirty 为 worktree-differs、unsourced/无 repo-root 不称 verified；E5 护照可读 | 已满足 | 无 |
| WORK-F3/AC-3，§3 F3 | 四类托管永久链接、本地标识、无凭据 | M4/M6/M8；V-5/9 | M4/M6/M8 | S：location/URL 输入门；H：真实 git 与 provider 链接测试；V：E2 摘要/链接记录，E5 local-only；R1 凭据修复已进入候选 | 已满足 | 不把链接格式测试说成四站线上测试 |
| WORK-F3/AC-4，§3 F3 | 时间/时长依据与日精度 | M1/M3/M6/M7/M8；V-4/9/11 | M1/M3/M6/M7/M8 | S：precision 和 evidence 徽标；H/V：E1–E3、E5 date-only / illustrative / incomplete ≥；无虚构因果箭头 | 已满足 | G-01 是时区接收缺口，另列 |
| WORK-F4/AC-1，§3 F4 | 原五类四门、确切身份 | M6/M8；V-7/13 | M6/M8 | H/V：E1/E2 原五类两合同，同 candidate/HTML/config/engine；原 delivery 审核后 complete | 已满足 | 无 |
| WORK-F4/AC-2，§3 F4 | 新五类同四门，showcase 零错误/警告 | M2/M3/M6/M8；V-3/4/7/13 | M2/M3/M6/M8 | H/V：E1/E2 新五类 showcase 四门与具名检查完整；未将基础检查等同 showcase | 已满足 | 有效参考时区的前置接收另见 G-01 |
| WORK-F4/AC-3，§3 F4 | 可操作诊断及 unavailable/failed/视觉分列 | M4/M6/M7/M8；V-5/7/11 | M4/M6/M7/M8 | H：原失败会话；V：E4 证明 fresh out-dir 的汇总与嵌套状态冲突、原启动诊断丢失 | 部分满足 | G-02 / FU-2 |
| WORK-F4/AC-4，§3 F4 | 旧证据、变更、失配不通过 | M4/M6/M8；V-5/7/13 | M4/M6/M8 | S：provenance/metadata/evidence bound；H/V：E1/E2 reviewBindings/newDelivery/reviewRecovery、篡改反例 | 已满足 | G-05 只按已批准边界复用 |
| WORK-F5/AC-1，§3 F5 | 旧五类主题/风格、查找、详情、坐标不改 | M5/M6/M8；V-6/7/13 | M5/M6/M8 | H/V：E1/E2 Reader 真实八项操作、导出前后状态，E6 单测 | 已满足 | 无 |
| WORK-F5/AC-2，§3 F5 | 十类 lens/Route/Reach、空/冲突、树折叠 | M5/M6/M8；V-6/7/13 | M5/M6/M8 | S/H/V：E1/E2/E6 十五例探索；八类有向关系卡片，timeline/waterfall 无出边不造因果，树导出完整 | 已满足 | 无 |
| WORK-F5/AC-3，§3 F5 | Still、主动 Live、reduce/hidden/print/export 静态语义 | M5/M6/M8；V-6/7/13 | M5/M6/M8 | H/V：E1/E2 每例 motion/print 样本；E5 首开 Still，E6 控制器/树打印 | 已满足 | 无 |
| WORK-F5/AC-4，§3 F5 | 概览/展示/深链接可返回，键盘与关闭，不改源 | M5/M6/M8；V-6/7/13 | M5/M6/M8 | H/V：E1/E2 explore 的 overview/presentation/deep-link/keyboard/details 与源摘要；E6 状态机 | 已满足 | 不把放大时局部视口当默认完整图 |
| WORK-F6/AC-1，§3 F6 | 原五类五格式、双主题 SVG、PNG 剪贴板 | M6/M8；V-7/13 | M6/M8 | H/V：E1/E2 每例20项真实格式/签名/浏览器解码与 clipboard | 已满足 | 无 |
| WORK-F6/AC-2，§3 F6 | 新五类同格式；Route/Reach 带上下文、非 canonical | M5/M6/M8；V-6/7/13 | M5/M6/M8 | H/V：E1/E2 新五类导出；八类 route/reach 卡片 variant 与 canonical=false，签名/解码；E6 控制器 | 已满足 | 无关系的两类不伪造卡片 |
| WORK-F6/AC-3，§3 F6 | canonical 完整图/主题/风格，无临时叠层，恢复探索 | M5/M6/M8；V-6/7/13 | M5/M6/M8 | S/H/V：export-cleanup、E1/E2 的 canonical/状态恢复/完整树，E6 | 已满足 | 无 |
| WORK-F6/AC-4，§3 F6 | 格式/权限/编码失败准确，不报空文件/后到成功 | M5/M6/M8；V-6/7 | M5/M6/M8 | H/V：E1/E2 真实能力失败与 fallback；E6 空 Blob、录制 error/timeout 终态 | 已满足 | 只确认相应浏览器能力范围 |
| WORK-F11/AC-1，§3 F11 | 保持四平台匹配资源，无 Rust 消费 | M8；V-12/13 | M8 | H/V：E1 当前 API 同一 tgz 四原生消费与 size/SHA/probe，Rust-free PATH | 已满足 | 候选；终验 P3 |
| WORK-F11/AC-2，§3 F11 | 保持离线/幂等/provider/skip | M6/M8；V-7/12/13 | M6/M8 | H/V：E1/E2 安装与 OS 拒网/Reader 离线、重复与 skip 保留 | 已满足 | 迁移回滚留 P3 |
| WORK-F11/AC-5，§3 F11 | 保持固定身份恢复与预检 | M8；V-12/13 | M8 | S/H/V：bootstrap、E1 消费拒绝反例与项目无写入；P1 G-01/D-01 的 candidate 恢复决定继续有效 | 已满足 | P1 差异不重新编号为本轮 G-ID |
| WORK-N1/AC-1，§3 N1 | 保持四桌面、可读纵向例外 | M6/M8；V-7/13 | M6/M8 | H/V：E1/E2 四桌面+390、双主题真实测量；E5 本次主图抽看 | 已满足 | 不声称本次重看所有尺寸 |
| WORK-N1/AC-2，§3 N1 | 默认确切产物真实看图，独立状态 | M6/M7/M8；V-7/11/13 | M6/M7/M8 | S/H/V：review-record 原身份；E1/E2 十五例完整，CI 保留 pending；E5 实看17张原截图 | 已满足 | G-05；语义事实另受 G-03 限定 |
| WORK-N1/AC-3，§3 N1 | 十类同窗口/操作/视觉门槛、详情按需 | M2/M5/M6/M8；V-6/7/8/13 | M2/M5/M6/M8 | H/V：E1/E2 含长外键、waterfall 长详情/标题、窄屏真实 Reach；E5 主图与护照 | 已满足 | R1–R4 修复进入最终候选 |
| WORK-N1/AC-4，§3 N1 | 作者内容按语言；五界面语言与明确回退 | M1/M5/M6/M7/M8；V-2/6/9/11 | M1/M5/M6/M7/M8 | S/H/V：唯一目录、resolved locale；E1/E2 zh-TW/es/ko/en/zh-CN/fr；E3 语言行，E5 四语言/回退截图；作者内容未冒称机器翻译 | 已满足 | 英文宿主答复观察不等同图内语言错误 |
| WORK-N2/AC-1，§3 N2 | 原五类 Rust 核心保持 | M6/M8；V-7/12/13 | M6/M8 | S：layout/render/check/delivery 原生；H/V：E1/E2 无旧 Node 正常执行 | 已满足 | 终验 P3 |
| WORK-N2/AC-2，§3 N2 | 新五类 Rust 核心 | M1–M3/M8；V-3/4/12/13 | M1/M2/M3/M8 | S：五类 Rust 模块与 dispatch；H/V：E1/E2 原生四平台；Node renderer 仅用于维护 oracle/本次对照 | 已满足 | 终验 P3 |
| WORK-N3/AC-1，§3 N3 | 保持2.17五类身份/方向/显式几何/原文件 | M1/M6/M8；V-1/7/8 | M1/M6/M8 | H/V：E1/E2 legacy 五例、v1 lifecycle 与旧输入 oracle；不覆盖源 | 已满足 | 后续字段处置留 P3 |
| WORK-N3/AC-2，§3 N3 | 3.1十类核心与 lifecycle v1/v3、原件不覆盖 | M1/M3/M6/M8；V-1/4/8 | M1/M3/M6/M8 | H/V：E2 的25 JSON/15 HTML不变；V：E4 六个有效参考时区被拒绝，不能以示例全过代表有效输入全集 | 部分满足 | G-01 / FU-1，终验仍 P3 |
| WORK-N4/AC-1，§3 N4 | 同输入/引擎/配置确定性；alias 拒绝 | M4/M6/M8；V-5/7/13 | M4/M6/M8 | S：evidenceSha256 入配置；H/V：E1/E2 determinism、真实 FS alias/摘要 | 已满足 | 终验 P3 |
| WORK-N4/AC-2，§3 N4 | 确定性失败保旧；浏览器失败不复用旧通过 | M6/M8；V-7/13 | M6/M8 | H/V：E1/E2 八恢复/新交付；E4 浏览器缺席时新HTML仍保留且 incomplete，前三门真实分列 | 已满足 | 原因误报另见 G-02 |
| WORK-N5/AC-1，§3 N5 | 文本安全、安装后离线生成/检查/阅读 | M4/M6/M8；V-5/7/12/13 | M4/M6/M8 | S：新文本转义、git 禁传输；H/V：E1/E2 XSS literal、OS 拒网与 CDP 零远程；E6 来源显示 | 已满足 | 外部访问终验 P3 |

### 3.2 P3 的16条AC：均为范围外，不作为本期遗漏

| PRD 需求/验收 | 本期边界与去向 | 计划/记录 | 覆盖状态 |
| --- | --- | --- | --- |
| WORK-F7/AC-1、AC-2 | 品牌查询/采集、固定资源异常处置 → P3 | 原计划 §0.5；brand 明示延期 | 范围外 |
| WORK-F8/AC-1、AC-2 | 主动预览、有效版本推进与停止 → P3 | 原计划 §0.5 | 范围外 |
| WORK-F9/AC-1、AC-2 | 两版架构比较与成对恢复 → P3 | 原计划 §0.5 | 范围外 |
| WORK-F10/AC-1、AC-2、AC-3 | 唯一入口替换、调用方/fallback、描述与许可终局 → P3 | 原计划 §0.5/0.6；未修改 archify/elintp/documentation-writer | 范围外 |
| WORK-F11/AC-3、AC-4 | 旧安装迁移、中断/取消/回滚 → P3 | 原计划 §0.5；不以当前候选安装替代终验 | 范围外 |
| WORK-N2/AC-3 | 最终发行身份与来源许可全集 → P3 | 原计划 §0.5；本期仍保留现有许可与固定候选 | 范围外 |
| WORK-N3/AC-3、AC-4 | brand/views/animation 最终处置、扩展约定 → P3 | 原计划 §0.5；无隐式迁移原 JSON/HTML | 范围外 |
| WORK-N4/AC-3 | 比较/预览/安装/导出取消与并发恢复终局 → P3 | 原计划 §0.5 | 范围外 |
| WORK-N5/AC-2 | 安装/品牌外部访问与公开来源安全终局 → P3 | 原计划 §0.5；本期来源凭据门已复验 | 范围外 |

### 3.3 用户旅程与终验

| 旅程/UAT | 本轮核对 | 尚需动作/后续 |
| --- | --- | --- |
| S1 / UAT-1 | 同包四平台，无 Rust 安装；十类自动门、本机视觉、阅读导出 E1/E2 | G-01 的有效参考时间线补齐；公开发行仍未进行 |
| S2 / UAT-2 | 原/新自然意图、六 Mermaid、旧/参考 JSON 与源不覆盖 E1–E3 | G-03 先问清缺失 ERD 事实；G-01 兼容 |
| S3 / UAT-3 | 固定来源、dirty/local-only、日精度、illustrative E1–E5 | N5 终局留 P3 |
| S4 / UAT-4 | 旧成果保护、变更拒绝、视觉独立；本次四组失败对照 | G-02 真实失败原因/状态 |
| S5 / UAT-5 | 暗亮、探索可撤回、图像/视频/canonical/卡片、五语言 E1/E2/E5/E6 | 不由这些自动/视觉证据替代业务事实核对 |
| S6 / UAT-6、S7 / UAT-7 | 本期只保持候选安装和旧内容；迁移/预览/比较未纳入 | P3，不作本期通过 |

F1/F2/F4 仍有上表部分满足条件，因此七条本期终验需求不能整体宣称已全部收口；F3/F5/F6/N1 的当前证据仍有效。N2/N3 只登记本期交付部分；F11/N4/N5 不提前终验。本轮不写 SPMS shipped/approved 状态。

## 4. Triage 与决策账本

没有新 C 问题卡，没有新用户裁定。以下是有来源的既有决定，编号仅为本报告映射，不冒充原文已有 D-ID。

| 决策 ID | 关联 G-ID | 选择与理由 | 确认日期与来源 | 验收边界 | 跟进去向 |
| --- | --- | --- | --- | --- | --- |
| D-01 | G-04 | 本期修 workflow 布局/诊断，保留事实、quality 与停滞原则 | 2026-10-21，M7「继续动作」第4项的19:45裁定及第7项实现记录；是记录转述的既有用户决定 | phoenix-v1 schema 2，失败后最小修复；不降质量、不改 legacy | 已实现；无新任务 |
| D-02 | G-05 | 受影响8行重跑，其他行用最终引擎重放；解决同包收口成本 | 2026-10-22，M8「评审修复后的重做」首项与原计划 D13；是记录转述的既有用户决定 | 相同 bundle；主图 SVG/失败码重放；不放宽产品默认视觉要求 | 当前候选满足；bundle 改变后不适用 |

G-01 的已知限制与 G-02 的“非本期回归”没有被登记为批准删减；G-03 的原 reviewer passed 也不是用户批准补造事实。A 的补齐不需要新选择；gaps 计划不含接受限制或延后的假设。

## 5. 审查基线、验证与限制

### 5.1 固定基线

- 当前 checkout：main / 93c4842ca2b2191fe13acc6b09472c8b5244c88f，dirty。开始时仅已有评审报告、SDLC/经验状态页及索引变动，没有产品代码变动；本轮产物另列，不混入所审代码。
- PRD v1.0 SHA-256：e099e47858cc9efeecdda26e1786242a2393cf50d0198538fc0817b144e4263b。
- P2 原计划 SHA-256：56c0fb210c4f61ead2f162cea9fe36be5afee23e5e11f22b99af6044d97064d8。M1–M8 的八份记录均已读取，按原里程碑时间理解历史的“未完/已取代”，不把早期旧候选当最终结果。
- 最终候选 c1703a9；引擎 sourceInputSha256 07d3da1edcfa9b68c9abd3fe53eb34244f886faba3b0abadcc305ed83dfcc2e6。当前170个构建输入逐文件匹配其清单，差异0。
- 环境：darwin-arm64，Node 25.2.1，ICU 77.1，tz 2025b；最终候选引擎的独立单链接副本。没有更新依赖、安装到活跃项目、运行新宿主会话、dispatch CI、推送或公开发布。
- [调查基线快照](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/baseline.json) 保存原始 dirty 状态及 PRD/计划/八份记录摘要。该忽略目录只在本机，不代替可同步的报告。

### 5.2 证据登记

| ID | 证据与入口 | 本轮核对结果及限制 |
| --- | --- | --- |
| E1 | [实际聚合重放结果](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/aggregate-result.json)、[重放入口](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/aggregate-recheck.mjs)；原 aggregate-39816459249.input.json | 本轮 GitHub 连接器 GET 原始 run/jobs/artifacts，run39816459249 attempt1、c1703a9、10 jobs success。当前 aggregateCandidate 实际重读候选/harness/四平台/Node18/本机/两宿主归档，exit0、pass/passed、diagnostics=[]。这是只读取证重放，没有重新执行四平台或宿主。 |
| E2 | [M6最终索引](../../local/phoenix-diagram/p2-core-13/index-c01701a05fc1b26a.json)、[字节审计结果](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/evidence-audit.json)；M8本机消费状态 | M6索引SHA cf3f6dad09012ebce3d0eb02133e75c75edff13505f070c243cb691ae5da7ef2；15例complete/passed、11个范围pass。1,093处引用对应1,029个独立path+digest，全部大小/摘要一致；170构建输入一致。corpus25JSON/15HTML不变；这些正例不代表所有合法输入已覆盖。 |
| E3 | v11h每宿主4行 + v11d沿用27行，见原聚合输入 providers；[ERD原件复核](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/host-erd-source-review.json) | 聚合分别核对每宿主31行原生会话、读取、回复、产物和最终引擎重放。本轮另读ERD原题/候选/原生答复及repo/mermaid-class/stall/browser-unavailable审核原文。G-03纠正原语义结论，G-02说明阶段receipt仍可读但summary误报；没有声称31行均重新由本轮人工逐语义审核。 |
| E4 | [最终引擎对照复现](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/reproduction-results.json) | 六个时区冻结renderer通过/Rust拒绝；四组浏览器环境×目录对照。全部为本轮实际执行，未修改产品代码。 |
| E5 | p2-core-13原始PNG，路径由索引提供 | 本轮实际查看17张：新五类1440×900暗亮10张；verified、worktree-differs、local-only护照3张；zh-TW/es/ko/fr回退主图4张。默认主图事实和控件可读；changed护照的AUTO133%图例局部裁切与M6既有观察一致，不据此声称全面UI验收。其余看图范围承接经身份核对的历史观察。 |
| E6 | [npm test日志](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/npm-test.log)、[Reader定向测试日志](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/reader-tests.log) | 本轮 npm test exit0，Node494项/471pass/0fail/23skip，Python四组22/24/2/13 OK。Reader五套与导出控制器为114项/113pass/1fail/0skip，exit1；失败为G-06的过期冻结行为断言。未将单测等同真实浏览器/原生平台/模型会话。 |
| E7 | [SystemV路线探针](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/system-v-probe.json)、[探针代码](../../local/phoenix-diagram/p2-prd-dev-gaps-20261023/probe-system-v.mjs) | 为FU-1核实路线：从已读的[ICU77.1官方数据](https://github.com/unicode-org/icu/blob/release-77-1/icu4c/source/data/misc/zoneinfo64.txt)提取六区历史转换与最终规则，用固定Node25.2.1/ICU77.1/tz2025b比较每区4,924个边界/季节样本，零差异。上游文本头写tz2025a，未冒充其为Node的tz2025b；结论限于六区经实测等价，不把该数据替换整个tzdb。 |

当前未重跑 npm run test:diagram / test:phoenix：产品未改，读取 M8/R5 对同源代码已有维护结果，不将其报为本次执行。真实旧版Git仍是V-5“环境可用时追加”的未运行观察；当前Git与包装防传输测试有既有证据，不以模拟冒充旧版实测，也不据此新增未经约定的阻塞门。

**待核实清单为空**：没有阻止上述差异分类/范围确定的缺失原件。未重新执行的历史验证和非目标环境属于证据适用限制，不被写成这次通过。性能SLO、真人语言质量和任意规模输入可靠率均未建立，本报告不添加此类承诺。

## 6. Follow-up 与 gaps plan

未发现既有 gaps plan 承接以下四项。原 P2 计划只有 G-01 的已知限制、G-02 的产品观察、G-03 的旧 passed 记录与G-06适配的实施记录，没有尚在推进的补齐/验证同步任务。

| G-ID / PRD ID | 确定的增量工作 | 验收条件 | 依赖/阻塞 | 去向 |
| --- | --- | --- | --- | --- |
| FU-1 / G-01 / F1 AC-2、N3 AC-2 | 六区原生时间规则、固定来源与反例 | 六种原参考JSON接收；墙钟/偏移/边界与固定Node一致；实际图/详情/导出可读；原JSON不变 | 已验证技术路线，无产品待决 | 独立gaps计划M1及M4 |
| FU-2 / G-02 / F4 AC-3 | 证据目录准备/保留与诊断传播 | fresh/precreated × missing/launch-failed正确分列；没有伪造截图/完整交付，原HTML/输入保护；真实路径失败仍准确失败 | 无产品待决 | 独立gaps计划M2及M4 |
| FU-3 / G-03 / F2 AC-3 | ERD必需事实澄清提示及语义验收纠偏 | 原题先问缺口且停止、无candidate/图；完整材料与显式Mermaid照常交付；不得先补规则后请确认 | skill bundle改变使旧宿主证据不能按D13直接复用 | 独立gaps计划M3及M4 |
| FU-4 / G-06 / N1 AC-3、N3 AC-2（验证同步） | Reader高度适配测试与维护入口同步 | 现行既定行为有真实断言、既定排除/源几何不变有反例；维护入口实际执行该套件 | 无产品待决 | 独立gaps计划M3及M4 |
| G-04 / D-01 | 保留布局适配 | 不降低语义、质量或停滞门槛 | 无 | 无新增工作 |
| G-05 / D-02 | 保留受约束证据组合 | 适用条件仍须逐项验证；不掩盖G-03 | 无 | 无新增工作 |
| P3 的16条AC | 本轮不规划 | 按PRD原分期 | 原前置 | P3；非本期遗漏 |

gaps计划：[PHOENIX-DIAGRAM-P2-gaps.md](PHOENIX-DIAGRAM-P2-gaps.md)，**Ready，0/4 尚未开始**。FU-1→M1，FU-2→M2，FU-3/FU-4→M3，M4统一新候选/真实宿主与回归终验。计划结构/引用与进度检查均0个ERROR；脚本对原PRD工作号、未来运行路径、未启用的兼容台账路径和既有dirty工作树的提示已人工核对。顶部实施合同与原骨架逐字一致，原PRD/计划/八份记录/170构建输入未变。只规划FU-1–FU-4及其直接回归；不执行、不替原计划修改完成事实，也不把计划Ready说成缺口已解决。

下一步：

~~~text
$review-dev-plan docs/plan/PHOENIX-DIAGRAM-P2-gaps.md
~~~

计划评审和修订后再按普通开发回路实施、代码验收，最后重新核查原P2覆盖。报告、计划及状态登记未提交/推送；本机忽略目录中的原件需保全或同步后才能跨机器重放。

