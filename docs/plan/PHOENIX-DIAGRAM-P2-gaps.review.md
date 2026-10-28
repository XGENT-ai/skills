# PHOENIX-DIAGRAM-P2-gaps · 开发计划评审报告

> 评审日期：2026-10-23（Asia/Shanghai）。模式：首次完整评审。目标代码与原计划保持只读。

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-DIAGRAM-P2-gaps](PHOENIX-DIAGRAM-P2-gaps.md) | SHA-256 `6948cfc4af616dcd667ab9f61fcb9bdf2b819ea4983521392a377e83c8a560ce` | 全文（M1–M4、V-1–V-8） |

背景资料（非修订目标）：[PRD v1.0](../PRD-PHOENIX-DIAGRAM.md)、[P2 原计划](PHOENIX-DIAGRAM-P2.md)、[差异报告](PHOENIX-DIAGRAM-P2.prd-dev-gaps.md)。

## 确认问题

无 P0–P2。以下 7 条均为 P3，按相对重要性排序，均不阻塞开工。

1. [P3] F-01 · 时区接受范围变化后，创作侧文档未安排同步（来源：delivery）
   - 定位：PHOENIX-DIAGRAM-P2-gaps.md §1.2 文件清单（`references/authoring-defaults.md` 行只锚定「ERD事实检查及澄清规则同步」）、§5.1、§0.5；关联 M1、WORK-F1/WORK-N3
   - 问题：M1 完成后六个 `SystemV/*DT` 名称从拒绝变为接受，但作者实际阅读的两处规则文本仍写「IANA 名或固定偏移」：`skills/phoenix-diagram/references/authoring-defaults.md:31`（Timeline 行「`meta.timezone` 是显示时钟（IANA 名或固定偏移，缺省 UTC）」）与 `skills/phoenix-diagram/schemas/timeline.schema.json:69`（description「IANA time zone used for axis and event labels; defaults to UTC.」）。新作者会据此认为 SystemV 名称非法而改写为近似城市时区（恰好制造本期要消除的语义偏移）；曾遭拒绝的旧 JSON 作者在 skill 文档中看不到行为已变化的说明。该文实差距现状已存在（`tools/phoenix-diagram/src/format/time.rs:46–51` 已接受 33 个 ICU 别名与 6 个非 DST SystemV 名），本期进一步拉大。计划已安排 NOTICE/发行说明含 SystemV 适配（§1.2 scripts 行），但作者日常读取的创作默认值与 schema 说明不在其列。
   - 证据：`skills/phoenix-diagram/references/authoring-defaults.md:31`；`skills/phoenix-diagram/schemas/timeline.schema.json:66–70`；`tools/phoenix-diagram/src/format/time.rs:17–51`；docs/phoenix-diagram.md 无 timezone 相关表述（grep 无命中，无需同步）。
   - 修订：在 §1.2 或 §5.1 增加一条：同步 `authoring-defaults.md` Timeline 行，注明另接受六个 SystemV 夏令时名称；schema description 文字是否改由计划明确取舍（§0.5「不改输入 schema」优先时可只改创作默认值，schema 校验契约本就不含名称枚举）。
   - 复核：改稿后核对 §1.2/§5.1 已含该条；实施后 grep 文档表述与实际接受行为一致。
   - 置信度：高（两处文本逐字核实）；阻塞：否

2. [P3] F-02 · V-6 反例通过判据未绑定「所问即所缺、不重复已给事实」（来源：delivery）
   - 定位：PHOENIX-DIAGRAM-P2-gaps.md §9 V-6 行（「先问且无candidate/HTML/补造选项才过」）、§6.1（fixture 含「预期缺口的独立审核字段」）；关联 M3/M4、WORK-F2
   - 问题：M4 在每宿主全新会话投放 erd-facts 原题时，按 V-6 字面判据，一个「问了但问错/问泛」（如「请确认所有基数与可空性」，把材料已给的「每订单至少一条明细」也重新问一遍）的会话也会被放行。这与 §6.1 自己的「材料已明确时直接用，不重复询问」规则不一致地宽松，且会让 ERD 作者被不可操作的问题阻塞。fixture 的独立审核字段正是为此设计，但判据未引用它。
   - 证据：§9 V-6 行原文；§6.1 fixture 定义；差异报告 G-03 原题未给的两项事实（orders.customer_id 可空性、商品最小参与数）。
   - 修订：V-6 判据补一句：「所问覆盖 erd-facts.json 预期缺口（商品最小参与数、orders.customer_id 可空性）且不涉及材料已给事实，按独立审核字段逐条核对」。
   - 复核：改稿后核对 V-6 文字；M4 执行时按 fixture 字段留逐条核对记录。
   - 置信度：中（判据字面确实未绑定，实施者也可能自行按 fixture 字段执行）；阻塞：否

3. [P3] F-03 · V-4 截图实际查看范围表述歧义（四张 vs 六区各四张）（来源：delivery）
   - 定位：PHOENIX-DIAGRAM-P2-gaps.md §9 V-4 行（「逐张实际查看新时区四张暗亮截图」）与 §9.1（六区「逐个…finalize…保留输入与四张截图身份」）；关联 M4、WORK-F1/N1
   - 问题：V-4 行可读作六个时区共看四张（只看一个区），而 §9.1 的语义是每个交付各产四张（2 视口 × 2 主题，与 `tools/phoenix-diagram/src/browser/evidence.rs` reserve 的 targets 循环一致），六个交付共 24 张。按歧义解读执行会让其余五区的可见时间标签、轴说明未经实际查看即登记 review-record，削弱 FU-1 的「实际图/详情/导出可读」验收条件。
   - 证据：§9 V-4 行与 §9.1 原文对照；`evidence.rs` Evidence::reserve 的视口×主题 targets（每交付 4 张）。
   - 修订：V-4 或 §9.1 写明「六个交付各自的四张暗亮截图（共 24 张）均逐张实际查看并分别 review-record」。
   - 复核：改稿后核对文字；M4 记录按六个交付分别登记看图与 review-record。
   - 置信度：中；阻塞：否

4. [P3] F-04 · V-4「按现行review-record在原delivery登记」中「原delivery」指向歧义（来源：schedule）
   - 定位：PHOENIX-DIAGRAM-P2-gaps.md §9 V-4 行；对照 §0.5 末条「旧验收运行目录保持只读」与 V-5「不得为取得complete重做原delivery」；关联 M4
   - 问题：「原delivery」可被误读为原计划 P2 的历史 delivery/运行目录（§0.5 明确只读）；按 V-5 的同款用法与 §9.1「实际看图和review-record由实施者按产品合同完成」，合理解读是「本运行新产生、已 finalize 的交付上原位登记」。歧义足以造成实施停顿求证，误读执行则会向只读历史证据区写入。
   - 证据：§9 V-4/V-5 行、§0.5、§9.1 原文；原计划 V-7 第 4–5 步（docs/plan/PHOENIX-DIAGRAM-P2.md:548）的「写入运行目录 actual-reviews/、原位 review-record」流程。
   - 修订：改为「在本运行新产生的 delivery 上按现行 review-record 原位登记」或同等措辞。
   - 复核：改稿后核对 V-4、V-5、§0.5 三处不再互斥；M4 记录中 review map 指向新运行目录。
   - 置信度：中（已有 §0.5/V-5 两道约束可挡误操作）；阻塞：否

5. [P3] F-05 · 冻结候选提交与 M4 最终提交的 SHA 回填映射未写明（来源：facts，schedule 关联复核）
   - 定位：PHOENIX-DIAGRAM-P2-gaps.md §10 提交点段；关联 M1–M4 完成记录「提交」列
   - 问题：「提交点：M4（全部里程碑完成后提交一次…回填各完成记录SHA）」与「为取得同SHA四平台制品，可按原计划候选收口先提交冻结候选源码」不构成互斥（后者有「终验未过前不记M4完成」守卫，原计划 M8 一行多 SHA 也是既有实践），但计划未说明冻结提交是否回填 M1–M3 行、M4 最终提交覆盖什么内容。实施者届时需自行决定回填映射。
   - 证据：§10 提交点原文；顶部「实施进度」协议的提交规则；原计划 PHOENIX-DIAGRAM-P2.md 完成记录 M8 行（a8f3ec4、8539d18、c1703a9、78adadd 四 SHA）。
   - 修订：补一句「冻结候选源码提交回填 M1–M3 行的提交列；M4 最终提交回填 M4 行」或实际采用的映射。
   - 复核：改稿后核对提交点段；实施时完成记录每行的提交列与所述映射一致。
   - 置信度：中（清晰度问题，非矛盾）；阻塞：否

6. [P3] F-06 · §1.1 第 4 条引用行号区间未覆盖 seal_browser 调用行（来源：facts）
   - 定位：PHOENIX-DIAGRAM-P2-gaps.md §1.1 第 4 条引用 `delivery/transaction.rs:340–363`
   - 问题：支撑「finalize 在 seal 之后才 extend 浏览器诊断」的 `seal_browser` 调用在 `tools/phoenix-diagram/src/delivery/transaction.rs:333`（引用区间之前），`summary.diagnostics.extend(...)` 在 345 行（区间内）。引用区间只覆盖了 seal 之后的处理块。仅定位精度问题，结论本身经主 agent 与两位 reviewer 独立核实成立。
   - 证据：`tools/phoenix-diagram/src/delivery/transaction.rs:333–345`（seal_browser 调用与 extend 的实际顺序）、seal_browser 定义于 494 行起。
   - 修订：引用区间改为 `transaction.rs:333–345`（需要函数体时另引 494–546）。
   - 复核：`grep -n 'seal_browser\|diagnostics.extend' tools/phoenix-diagram/src/delivery/transaction.rs`。
   - 置信度：高；阻塞：否

7. [P3] F-07 · 「format 测试主动豁免这六区」超出当前实际触发面（来源：facts）
   - 定位：PHOENIX-DIAGRAM-P2-gaps.md §1.1 第 1 条（「format测试主动豁免这六区」）
   - 问题：豁免规则本身是类级（`tests/format.rs:101` 附近 `starts_with("SystemV/") && ends_with("DT")`），但现行 `tests/fixtures/format/oracle.json` 的 68 个 zone 中只含 `SystemV/EST5` 与 `SystemV/EST5EDT`，即当前实际只触发一个 *DT 区的豁免，其余五区不在 oracle 中。措辞把「规则覆盖六区」写成了「测试主动豁免六区」。不影响计划路线（§1.2 已含 oracle 再生成与六区补齐）。
   - 证据：`tools/phoenix-diagram/tests/format.rs:99–103`；`tests/fixtures/format/oracle.json` zones 过滤结果（主 agent 复核一致）。
   - 修订：改为「豁免 SystemV/*DT 类（现行 oracle 实际命中 EST5EDT）」或同等准确表述。
   - 复核：改稿后核对措辞；M1 实施后六区均进 oracle 且豁免删除由 V-1 验证。
   - 置信度：高（事实）/低（影响）；阻塞：否

## 待核实 / 待决策

无阻塞项。以下为非阻塞的证据限制与执行期事实，计划已如实登记或由本评审核实关闭：

1. M4 执行期的宿主账号配额、CI runner 可用性属执行期事实，本评审（只读）不 dispatch CI、不启动宿主会话；计划 §11 已登记「CPU争用/Chrome/CI/配额」为终验执行风险并给出「记未完成、不阻塞开发」语义。责任人：实施者；最晚确认点：M4。
2. 官方 ICU zoneinfo64.txt 线上内容：architecture reviewer 本次实际联网抓取 `https://raw.githubusercontent.com/unicode-org/icu/release-77-1/icu4c/source/data/misc/zoneinfo64.txt`，SHA-256 与计划钉值 `c0c34daa…dd55f` 逐字一致（2026-10-23），并解码 Rules 段确认 SystemV 为 legacy 规则（四月/十月最后周日 02:00），此项已闭环。
3. Node 25.2.1 的 ICU 77.1/tz 2025b：本机 `process.versions` 实测一致，nodejs.org 发行索引佐证；已闭环。
4. 原计划 M2/M7 记录中的 R2 局部回归失败与「继续动作」第 4/7 项：主 agent 逐行核对原文（M2.md:54 记录 reader-layout 既有失败未处理；M7.md:100/133 记录浏览器 ENOENT 产品观察与宿主误判），与差异报告转述一致；已闭环。

## 覆盖摘要

- **基线**：仓根 HEAD `93c4842ca2b2191fe13acc6b09472c8b5244c88f`，dirty（未提交内容为状态/评审记录与本计划等文档，无产品代码改动，与计划恢复快照自述一致）；调查日期 2026-10-23；所审计划版本 SHA-256 见「评审对象」。
- **实际 agent 分工**：四视角各一名独立只读 reviewer（facts / architecture / schedule / delivery 先后两波并行），主 agent 保留综合裁定并独立复核了：骨架合同逐字比对（仅占位符替换）、PRD 需求分期与 AC 原文、原计划 D10/D12/D13 与 §9.3、M2/M7 记录原文、G-02 根因链代码（browser/mod.rs、evidence.rs、transaction.rs）、F-01 文档文本、F-07 oracle 内容。多 agent 一致意见均未直接当事实，逐条经原文/代码回读确认。
- **四维覆盖**：
  - facts：§1.1 全部「已核实」主张、§1.2 文件清单（★ 新增标记正确）、§2/§5.2 接口可行性、§9 命令入口、内部一致性与数字，全部核实成立；无互相冲突到无法同时成立的陈述。
  - architecture：D1（SystemV 专用变体 + 固定历史表 vs 完整 ICU vs 城市映射）、D2（目录生命周期前置）、D3（prompt-only 最小提示）、D5/C-3（冻结后集中收口、bundle 改变不复用）均判定适配且代价合理；关键技术疑点（legacy SystemV 规则形态、墙钟 epoch 归一化、tz2025a/2025b 不对称）经官方文件与探针双重闭环；可编码性达标，无「实施时确定」的开放设计。
  - schedule：映射无断链（8 条 WORK + 4 条 C 双向追踪到 V-1–V-8 与 M1–M4；V-1–V-8 均有唯一执行时点）；M1/M2/M3 写入集互不相交、并行成立；检查时点符合「局部在里程碑、收口在 M4」；提交编排与证据失效规则可操作。
  - delivery：三条用户主路径（SystemV 旧 JSON、fresh out-dir 失败、ERD 澄清）文档推演闭环；失败表达在 receipt/summary/退出码三处一致；无「静态检查冒充最终验收」风险（§12 末条明确 V-8 不能代替 V-4/V-6/V-7）。
- **脚本执行**：`check_paths.sh` 0 个有效缺失（唯一 MISSING 为「兼容台账路径」`docs/project-status/protocol.md` 的协议文字引用，非交付物，差异报告已人工核对）；`check_progress.sh` 0 ERROR 1 WARN（WARN 为「计划未开始而工作树 dirty」的固有提示，dirty 内容与本计划进度无关）；`check_plan.py` 0 ERROR 13 WARN（WORK-* 上游 ID 与 §9 命令的 `<新运行目录>` 等运行期参数占位，均已人工核对为合理）。脚本通过不证明行为真实，事实结论以人工核对为准。
- **浏览器/真实环境**：本评审未执行任何真实浏览器、宿主会话或 CI（计划未实施，无新 UI 可走查）；所有实测级交付结论排在 M4 V-4–V-8，计划已如实登记该等待。UI 证据等级：文档推演 + 对既有代码/文本的静态评审。
- **覆盖缺口**：无视角被跳过；facts reviewer 未联网（由 architecture reviewer 的在线核对补位）、未逐行重读原计划 31 行宿主场景表（已确认该节存在且含每行角色约束）。

## 映射与实施编排

需求 → 设计 → 验证映射完整，无断链 ID；G-04/G-05 作为无新增工作的保留项经 C-3/§3/§12 正确处理，排除项均有去向。现有编排成立，引用原计划位置：计划 §10 里程碑表、§8 发布顺序、§9.2 取证安排。依赖边与波次（schedule reviewer 核对，主 agent 复核成立）：

| 波次/任务 | 前置及解锁证据 | 可同时做的工作 | 写入边界/共享负责人 | 运行资源约束 | 汇合验收 |
| --- | --- | --- | --- | --- | --- |
| 波次1 · M1 六区时间 | 无里程碑前置；§1.1 已核路线 | M2、M3 | 时间模块/fixtures/SOURCE.json/NOTICE/build 脚本；gaps-acceptance 六区片段交主实施者 | 本机 mise/Node25.2.1/mbx --offline；无浏览器/宿主 | V-1（M1 局部） |
| 波次1 · M2 浏览器失败回执 | Evidence/OutputSlot 契约 | M1、M3 | browser/evidence/transaction 及三个测试文件；12 组片段交主实施者 | 本机 Rust 测试 + 真实 Temp 目录 | V-2（M2 局部） |
| 波次1 · M3 ERD 澄清 + Reader 测试同步 | 现有 SKILL 原则；G-03/G-06 固定材料 | M1、M2 | SKILL.md/authoring-defaults/docs/reader 测试/native runner/erd-facts；不改 Reader 产品代码 | 本机 node --test | V-3（M3 局部） |
| 波次2 · M4 唯一阶段收口 | V-1/V-2/V-3 通过 + 源码/skill 冻结 | V-6 宿主会话可与 V-4/V-5 本机段重叠（受配额/CPU 限制）；V-7 CI 按 §8 发布顺序排在本机段后 | 主实施者统一浏览器/候选/证据身份；dist/ENGINE.json 由 build --sync-dist 生成 | darwin-arm64 Chrome + sandbox-exec（串行浏览器）；两宿主账号 + 临时项目；CI workflow_dispatch | V-4–V-8 全过 + FU-1–FU-4 真实结果 |

无安全并行空间之外的建议调整；F-01 的文档同步建议并入 M1（行为变化）或 M3（同文件修订），不产生新依赖。

## 总体结论与修订顺序

**是否需要修改：无需。** 只有 P3 确认问题、无阻塞性待核实项，且建议状态与计划自报状态一致。

- 建议状态：**Ready**（与计划自报 `Ready` 一致）。理由：四项 follow-up 的技术路线均有实测依据（六区路线经 29,544 样本探针与官方文件在线复核、目录/诊断根因经代码与本机证据确认、ERD 只具体化既有原则、Reader 只补验证）；映射无断链；编排与提交/证据规则可操作；没有产品、权限、数据归属或关键 NFR 待决。
- 覆盖限制：本评审为只读文档评审加代码/证据核对，未运行任何检查命令、未做真实浏览器/宿主/CI 验证；Ready 不表示缺口已关闭或产品已上线，实施与实测级验收仍待 M1–M4。
- P3 采纳顺序（可选）：先改 F-01（文档同步安排，影响作者可见行为）与 F-02（V-6 判据绑定，影响验收严谨性），再顺手修订 F-03/F-04（V-4 措辞）、F-05（提交回填映射）、F-06/F-07（引用与措辞精度）。
- 非阻塞建议（不进 findings，供参考）：architecture 视角提出 (a) V-6 缺事实反例可每宿主连投 2–3 次独立会话以提高统计信心——计划 §11 已登记该干预类型的残余风险与修订回路，采纳与否均成立；(b) M2 记录可补一句目录 check-then-create 并发竞争的实际处置，不必进计划正文。

## 下一步

```text
$apply-doc-review docs/plan/PHOENIX-DIAGRAM-P2-gaps.review.md   # 可选：采纳 7 条 P3 修订
$goal 按 docs/plan/PHOENIX-DIAGRAM-P2-gaps.md 开发              # 或直接开工
```

## 修订处置与复核

- 来源：本报告原始 SHA-256 `4ee17d9a8d7f12d39602392c3b098e8af56a3c21d9895bcbf4f05b9d85184298`；保留以上原问题、所审基线和总体结论。
- 目标：[P2 gaps 计划](PHOENIX-DIAGRAM-P2-gaps.md)；修前 SHA-256 `6948cfc4af616dcd667ab9f61fcb9bdf2b819ea4983521392a377e83c8a560ce`（与所审版本相同）；修后 SHA-256 `b6021484bba077f154dff6224c47aef27c2d53206ef0d3157f70a82d0ca945d4`。
- 复核：2026-10-23（Asia/Shanghai）；main / `93c4842ca2b2191fe13acc6b09472c8b5244c88f`，dirty。修改前已有状态/评审记录及未跟踪计划等文档；产品代码未改。本次覆盖 F-01–F-07，仅修订计划与处置记录，不执行开发。

| 问题 | 核实结论与依据 | 最终处置 | 修改位置与关联同步 | 复核结果 / 后续 |
| --- | --- | --- | --- | --- |
| F-01 | 成立。亲读 authoring-defaults.md Timeline 行、timeline.schema.json timezone description 与 time.rs 的 ICU_ALIASES；创作说明未体现 ICU 兼容接受范围。 | 已修订并复核 | 计划开发范围、§1.2、§5.1、V-3、M3；同文件由 M3 统一维护，新增六区说明及旧拒绝输入可重试说明。遵守 §0.5，schema 连同 description 保持不变；不将其文案当名称白名单。 | 已解决（文档层面）。六个名称对照差异报告 G-01；M3 静态核对、M4 对照原生行为，未把未来同步写成完成。 |
| F-02 | 成立。§6.1 已有不重复询问原则，而 V-6 未绑定独立审核字段；差异报告 G-03 明确两项缺口。 | 已修订并复核 | V-6 要求所问覆盖商品最小参与数、orders.customer_id 可空性，不重复材料已给事实，逐项留核对记录。 | 已解决（文档层面）。与 §6.1 原原则一致；真实两宿主会话留待 M4。 |
| F-03 | 成立。Evidence::reserve 的 targets 循环为两个视口×两个主题，每交付四张。 | 已修订并复核 | V-4 与 §9.1 同步写明六交付共24张，全部逐张实际查看、分别登记。 | 已解决（文档层面）。6×2×2=24；未声称已看图。 |
| F-04 | 成立。原 V-4 的原delivery指向不清，与 §0.5 历史只读约束存在误读空间。 | 已修订并复核 | V-4 明确本运行新产生且已finalize的各delivery；§9.1 同步本运行新交付登记。 | 已解决（文档层面）。对照 §0.5、V-5：旧运行只读，新运行原位登记，不为complete重做交付。 |
| F-05 | 成立。§10 原有冻结提交许可，但未定义各行 SHA 归属。 | 已修订并复核 | §10：提前冻结的SHA回填M1–M3；最终收口提交回填M4，影响旧里程碑的修复追加SHA；未提前冻结则最终SHA覆盖M1–M4。均按顶部协议单独提交回填，不自指。 | 已解决（文档层面）。与顶部提交协议及 §9.2 证据失效规则交叉核对；未生成提交或改完成记录。 |
| F-06 | 成立。亲读 transaction.rs:333–345，seal_browser 调用先于 diagnostics.extend。 | 已修订并复核 | §1.1 第4条引用改为333–345。 | 已解决（文档层面）。引用完整覆盖先后顺序，根因结论不变。 |
| F-07 | 成立。亲读 format.rs 类级例外，解析 oracle.json 得到 SystemV/EST5、SystemV/EST5EDT 两项；仅后者触发例外。 | 已修订并复核 | §1.1 第1条区分规则覆盖类与现有oracle实际命中项，并注明其余五个夏令时区待纳入。 | 已解决（文档层面）。保留 M1/V-1 删除例外、补齐六区的实施要求。 |

### 决策与未完项

- 7条均采纳；F-01采用报告允许的仅改创作默认值方案，并保留既有schema排除范围。无新增产品、授权或架构待决。
- 非 findings 的额外会话重复采样建议未纳入，不增加本期验收次数；M2目录并发竞争的执行记录建议留实施者按实际情况记录。
- 报告「待核实 / 待决策」第1项仍为M4执行资源风险，未当成已验证；第2–4项保留原评审证据归属，本次不重跑线上取源、版本探针或历史验收。
- 关联 PRD、原P2计划、差异报告均只读，无需同步其历史结论。authoring-defaults、schema及运行代码本次未修改，正文中的修改与验收安排留待实施。

### 修订后结论与验证

- 本次7条已修订并复核，0条待决、0条未完成文档修订。计划保持 **Ready**，进度保持 **0/4、尚未开始**。本结论仅为 apply 修订复核，不是新一轮独立评审或实现验收。
- `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P2-gaps.md .`：检查2个路径token，唯一MISSING为兼容旧台账的 `docs/project-status/protocol.md`；当前实际协议在 `.xgent-ai/sdlc/protocol.md`，非缺交付物。
- `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P2-gaps.md .`：0 ERROR、1 WARN；0/4与空完成记录一致。告警为未开始计划遇到既有dirty工作树，本次无产品实施。
- `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P2-gaps.md`：0 ERROR、13 WARN；8个WORK自定义ID均在账本并映射验证，5个参数占位均为执行期新路径/证据输入，人工核对合理。
- `npm test`：退出0；Node 494项，471通过、23跳过、0失败；Python四组分别22/24/2/13项均通过。未运行 Phoenix runtime、真实浏览器、宿主或CI；本次无运行时代码修改，V-1–V-8的实施验收未开始。
- 回读修订段落并核对关联范围、schema排除、并行写入边界、进度与SHA规则；本报告原始部分保持不变，未提交/推送，仅当前工作树可取回。

下一步：`/goal 按 docs/plan/PHOENIX-DIAGRAM-P2-gaps.md 开发`。
