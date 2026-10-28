# PHOENIX-DIAGRAM-P2-gaps · 当前版本完整重审

> 评审日期：2026-10-26（Asia/Shanghai）。模式：完整重审，兼核上轮全部问题。计划、产品代码、历史报告和验收原件均未修改。

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| 开发计划 | [PHOENIX-DIAGRAM-P2-gaps](PHOENIX-DIAGRAM-P2-gaps.md) | SHA-256 `1c7ea4b5d2561a628ee86092b9907e41e51900d0ddeb975f1f640133e7f9527c` | 全文，M1–M4、V-1–V-9；重点含 D21 与 §9.3–9.5 的验收设计 |

- 上轮：[2026-10-23 评审及修订处置](PHOENIX-DIAGRAM-P2-gaps.review.md)，评审对象 ID `df03c1ac-f891-4ae9-9451-b673beae7382`。当前报告文件 SHA-256 `cd99e1fe58b348be35bd676666807ca1a7232f8bd7bfd85047e190022f906b95`；其原始报告版本及处置前后版本保留在原报告。
- 计划版本链：上轮所审 `6948cfc4…a560ce` → 原处置后 `b6021484…945d4` → 本轮上述 `1c7ea4b5…f9527c`。本轮不是只检查原修订差异，也不覆盖原报告。
- 背景资料（非修订目标）：[PRD v1.0](../PRD-PHOENIX-DIAGRAM.md)、[原 P2 计划](PHOENIX-DIAGRAM-P2.md)、[PRD 交付差异报告](PHOENIX-DIAGRAM-P2.prd-dev-gaps.md)、[M4 实施记录](PHOENIX-DIAGRAM-P2-gaps.records/M4.md)。

## 确认问题

未发现需要阻塞实施的 P0–P2。以下为本轮新增的两项 P3，编号不与上轮 F-01–F-07 混用。

1. **[P3] F-08 · 补明旧调度器、安装审计及 fresh 目录检查的适配落点**
   - 定位：计划 §1.2 第170行、§9.4 第334/344行、V-9 第294行；关联 C-5、M4/V-6。
   - 问题：D21/§8/§9.5 已明确取消额外16项全过的启动前置，§9.4 又要求 factory 预建 `stdio/inspection-chain/`。但具体接线只点出 factory/index 和派生 FileRefs，V-9 也未点名现有 controller/初装 auditor 的旧门。若实施者只按这份具名清单改动，正式安装、完整初装审计和作者启动仍会要求旧 all16-ready；合法的预建空链目录也会被误判为非 fresh。计划的目标和“派生受影响接线”已经允许替换这些旧门，因此本项是落实锚点与低成本验证的补充，不是把待实现功能判为缺陷，不涉及新的产品或授权决策。
   - 证据：当前 Source21 [control.py](../../local/phoenix-diagram/p2-gaps-provider-prep-21/control.py):99–105 的 `semantic_ready()` 要求16项全部通过，117/128/145行分别用于 formal install/full audit/author；155–156行要求 `stdio` 完全为空。[verify-installation-v2.mjs](../../local/phoenix-diagram/p2-gaps-provider-prep-21/verify-installation-v2.mjs):29–43 同样检查 all16，100行对未启动项目要求空 `stdio`。主 agent 已逐段回读；这些是当前复用接口，不是推测的执行风险。
   - 修订：可在 §1.2/§9.4 补明新版控制配置、初装审计和作者前检的窄适配：按冻结角色清单、实际安装身份及本题 fresh 证据解锁，移除 all16 语义依赖；fresh 只允许唯一、owned/ordinary/0700 且为空的 `inspection-chain`，仍拒绝旧 capture/latest/lock/分页及其他输出。旧控制文件、历史门和原件保持原归属。
   - 复核：V-9 可补不启动模型的接线正反例，证明没有 all16 报告也可通过新版安装/作者前检；预建空链可通过，有旧 capture/latest 或非普通目录仍拒绝。不得以直接删除 fresh 检查实现。
   - 置信度：高（旧门和目录断言均实读）；阻塞：否。D21 已定替换方向，现有“派生受影响接线”足以授权必要适配。

2. **[P3] F-09 · M4 记录末尾的“下一步”仍指向退役轮次**
   - 定位：计划恢复快照第54–55行、§8 第276行、§9.5 第348行；所引用的 [M4 记录](PHOENIX-DIAGRAM-P2-gaps.records/M4.md) 第493–495行。
   - 问题：当前计划及 M4 记录头部要求先实现 V-9，再沿 Source21/D12 冻结64角色，Kimi 建议不再是推进前置。记录末尾却仍用独立的“下一步”要求继续 Source19 Code31、CI9，并等待两仓库的 Deepseek 选择。续做者按链接深入记录时，可能接续已经退役的队列或再次等待已有结论。顶部唯一恢复入口和当前 v2 章节能消除这项误导，故不阻塞实施。
   - 证据：计划第54/276/348行与记录第5、14–16行一致；记录第495行仍写“继续Source19 Code31双队列及独审”及“Claude两仓库同包Deepseek选择待答”。两处当前行动不一致，不能把旧尾段当成现行调度依据。
   - 修订：将末尾“下一步”明确标为 Source19 历史安排，或同步为 V-9→单次冻结→64角色与独立终验；不改写原失败、会话或历史通过结论。
   - 复核：对照计划快照、§9.5 与 M4 记录头尾，确认现行续做指令只有一套，历史指令有明确归属。
   - 置信度：高（两处原文直接冲突）；阻塞：否。

## 上轮问题复审

来源统一为上述上轮报告/对象 ID；本表“已解决”指原计划修订问题已关闭，不表示 V-6/V-7 或 M4 已验收。

| 来源 ID | 上轮问题 / 已有处置 | 本轮结果 | 当前证据与复核范围 | 剩余动作 |
| --- | --- | --- | --- | --- |
| F-01 | 时区创作说明未安排同步；原处置补入 M3/§5.1 | 已解决 | 计划第214/288行及 [authoring-defaults.md](../../skills/phoenix-diagram/references/authoring-defaults.md):31 明确六区、既有接受范围和原输入重试；schema description 保持不变是显式选择 | 受影响真实行为按 M4 收口，不能据文档推定全部通过 |
| F-02 | ERD 反例判据未绑定实际缺口；原处置补充 V-6 | 已解决 | 第291行要求核对商品最小参与数、orders.customer_id 可空性，不重复已给事实；[erd-facts.json](../../tools/phoenix-diagram/tests/provider/erd-facts.json) 分离作者题面与独审字段 | 新 collector/cohort 的两宿主结果仍须真实取得 |
| F-03 | 六区看图数量歧义；原处置明确24张 | 已解决 | 第289/301行均为每交付四张、六交付共24张；[evidence.rs](../../tools/phoenix-diagram/src/browser/evidence.rs):204–225 的 targets 为两视口×两主题 | 本轮没有重看全部24张；旧登记按实际身份和失效范围复用 |
| F-04 | 原 delivery 指向歧义；原处置明确新运行原位登记 | 已解决 | 第289/301行与 §0.5 历史只读一致，V-5 禁止为 complete 重做 delivery 的约束仍在 | 无文档剩余动作 |
| F-05 | 冻结/最终提交 SHA 回填不明；原处置补充 §10 | 已解决 | 第367行区分提前冻结回填 M1–M3、最终提交回填 M4、后续修复追加及无提前冻结的情况；当前完成记录保存冻结 SHA | M4 退出后仍须实际完成最终提交及回填 |
| F-06 | 原 seal_browser 引用区间不完整；原处置修正定位 | 已解决 | 计划原调查引用已修；当前 [transaction.rs](../../tools/phoenix-diagram/src/delivery/transaction.rs):331–340 已先保留原诊断再 seal。不能将旧调查中的调用顺序当作当前运行事实 | 无原问题剩余动作 |
| F-07 | 旧豁免类与 oracle 实际命中面混写；原处置修正措辞 | 已解决 | 计划第140行保留准确的原调查区分；当前 [format.rs](../../tools/phoenix-diagram/tests/format.rs):101–128 已直接逐区对照、无 SystemV 拒绝豁免，M1 记录包含扩展后的 oracle | 不把原 oracle 的命中面继续写成当前现状 |

上轮非阻塞待核实项也已追踪：执行期配额/runner 仍由 §11 登记；固定 ICU 原文与本机 Node 版本本轮再核；原 M2/M7 调查材料保留历史归属，未重跑其旧环境。未发现由原七项修订引入的新增 P0–P2；F-08/F-09 属当前验收设计与恢复记录的新增建议。

## 待核实 / 待决策

没有尚需用户决定的关键范围、权限、数据或外部契约前提。以下为计划已如实列明的实施/验收未完成项，不据此把设计判成 Blocked：

| 待证事项 | 影响 / 所需证据 | 责任与最晚确认点 |
| --- | --- | --- |
| V-9 及两个 main-v2 fixture 尚待实现 | 无损重组、8KiB边界、机械计数、链和 inspect 恢复必须由真实局部测试证明；本评审只核设计及现有接缝 | 主实施者；冻结 collector/harness 及新会话前 |
| 新版 V-6 的62正式角色及2反例 | 逐角色取得原始会话、加载/安装、语义、真实视觉及协议证据，不能用旧 Source21 两个 Workflow 的 passed 填新正式记录 | 主实施者及独审；M4 退出前 |
| V-7 最终同 cohort 桥接、消费及聚合 | 新候选/harness/collector/开发包/会话身份须由原入口实核；旧 CI10、失败或已清理原档不能冒充新收口 | 主实施者；M4 退出前 |
| 当次 CPU、磁盘、Chrome、账号及 CI 配额 | 启动时实核原资源门；失败如实保持对应项未完成，不因本轮评审擅换模型或重打包 | 执行者；各任务启动及 M4 退出前 |

## 覆盖摘要

### 基线与实际分工

- 调查基线：`codex/phoenix-diagram-p2-gaps-20261023` / `67d446e81da39e5cc7e2e515aeb1df6309f0b092`，dirty。事实源是当前工作树；已有 Source21 三份 MD、生成的 dist/ENGINE、计划/记录及状态资料等未提交改动均保留。
- 本轮实际启动四名独立只读 reviewer：facts、architecture、schedule、delivery；主 agent 负责需求账本、候选问题裁定、原始代码交叉回读、浏览器抽查和单写报告。评审结束四名均已返回，没有遗留运行任务。
- 当前 VERSION 实读并由 `sourceInputs()`/bundle manifest 重算核对：171个 Native 输入，sourceInput `a45958a28c32f0a1968294baf9c3e6bb94e2bb59b38fcf57f35b0565056c11cf`；bundle `d4a1082892fb609b50c5dad95eecddcfd39f9b68fac71d7f292552dfe87d25e9`；声明的 darwin-arm64 engine SHA `59d45937363c8ed044e5e791bc1a97f9fc2640ec790bc71575a925bd1e86ce98`。本次没有运行该引擎来代替缺失的终验。
- 计划、上轮报告、产品文件与引用的旧 helper 在收尾复核中未变；本轮写入只有本报告及受影响的状态登记。未提交/推送，dirty 版本与 gitignored 私有材料目前仅本工作树可取回。

### 四个维度

| 视角 | 已核范围与结论 | 限制 |
| --- | --- | --- |
| facts | 当前产品身份、§1.1 关键主张、M1–M3交付落点、当前 Source21 原结果归属、collector/default DTO、archive/aggregate 身份；V-9 明确待实现，没有完成声明漂移 | 原调查缺口按历史背景理解；没有重跑全部旧原生或宿主验收 |
| architecture | D1/D2 的局部方案沿现有边界成立；D21分页、按完整值去重、链头前移、inspect不spawn、分门计数与失败恢复足以编码。原 archive 严格 cohort 与 harness collector 认证确实存在；§9.5没有允许混身份拼接 | 未实现的分页/恢复正确性由 V-9证明；旧调度接线的具名补充见 F-08 |
| schedule | 8条 WORK、5条 C、FU-1–FU-4 均可双向追至设计、交付物、V与M；V-9为M4内前置，其他题/本机/CI不再依赖额外16语义小门；原提交及失效范围规则可操作 | M4记录旧尾段有 F-09；没有编造工期或要求每里程碑重跑全量 |
| delivery | 安装/普通命令、六区输入→阅读/导出、fresh目录失败→新证据目录恢复、ERD缺事实→只问必要问题、角色→独审→聚合的主路径均有设计和验收落点；自动complete与事实通过继续分开 | 浏览器仅做下述单例抽查，不能代替全部六区、导出编码或64角色终验 |

候选“旧 all16 门仍存在，因此设计必须 Blocked”未采纳：D21、第276/348行已经明确要求取消该前置，第170行也允许派生受影响接线。旧实现尚未改动本身不是计划缺陷。F-08保留可直接操作的落点和 fresh 回归建议，不将其扩大为未授权或未决的关键设计。

### 原始依据与真实环境抽查

- 时间数据：architecture reviewer 于本日在线抓取 [ICU release-77-1 zoneinfo64.txt](https://raw.githubusercontent.com/unicode-org/icu/release-77-1/icu4c/source/data/misc/zoneinfo64.txt)，360,734B/SHA `c0c34daadb59d2552252405b9be08894f83d2cd8145d0f6eb35e1557721dd55f`，与固定 fixture 逐字相同；六区各150转换、最终年1977。主 agent 也读取官方原文与本地来源；文件头 `2025a` 与本机 Node 的 tz `2025b` 分别保留，未将六区等价扩大到全库。原29,544样本探针仅作为已保存的路线证据，不声称本轮重跑。
- 当前 Node 实测为 `v25.2.1 / ICU77.1 / tz2025b`。Native输入根 [build.rs](../../tools/phoenix-diagram/build.rs):9–20 与 `package.json.files` 均排除维护 collector，CI harness 单独认证它；支持“改测试辅助不必重打产品包”的边界。
- facts reviewer 实读 Source21 原 `call-002.stdout`：1,014,315B、13条 diagnostic 的 measuredScene 完整值相同，唯一紧凑UTF-8为37,652B。主 agent 回读相应记录及 collector/aggregate 行为，不把截断预览当成全部诊断。
- delivery reviewer 读取 V-4 十二份原始 stdout 并核 SHA/字节：unavailable均 exit5，launch-failed均 exit4；四次 finalize 的前三门均pass。六区原登记为6个同ID complete/visual passed、共24原图。这是旧证据核对，不是本轮重新执行12组或逐张重看24图。
- 主 agent 在隔离 Chrome156 上实际打开 [AST4ADT 独立HTML](../../local/phoenix-diagram/p2-gaps-real-10/ast4adt/diagram.html)，最终使用1440×900、DPR1桌面模拟：查看暗色完整图；切亮色并点击 Payment failure alert；详情读到 `Monitoring › 09-30 23:05:00 UTC-03:00`；关闭详情、复位视图、切主题并展开导出菜单，PNG/JPEG/WebP/SVG/WebM入口可见。实际查看截图，默认完整图五事件与三泳道可读，无页面水平溢出，控制台无error/warn；焦点相机的局部放大不作为完整图截断。未触发文件下载或重新登记 review-record，截图作为本次工具观察，未写进旧证据目录。
- 上述浏览器抽查前后 HTML、candidate、summary 的SHA分别保持 `000016666a192c70a273294faa84177937e8011a14b7beec31de664acf28e689`、`8a05a81bb2144223336714713994364045263db569f6c381a4ec96c4d51703cd`、`2878210ba5785f130cf55f0cb614db27dafbceed77086971e0b83bd88ebb5cf2`。隔离页已关闭。没有将本次局部走查写成 V-4/V-5 全量通过。

### 本轮检查

| 检查 | 实际结果及解释 |
| --- | --- |
| `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P2-gaps.md .` | exit0；4个路径token，唯一MISSING为顶部协议的兼容路径 `docs/project-status/protocol.md`。实际 `.xgent-ai/sdlc/protocol.md` 存在，不是交付物缺失；另人工核对文件清单，新增fixture均标★ |
| `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P2-gaps.md .` | exit0；3/4，4条记录、3条已完成，0 ERROR / 0 WARN；记录及提交引用不据此自动认定终验通过 |
| `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P2-gaps.md` | exit0；0 ERROR / 13 WARN。8个WORK是原PRD工作号；5个参数占位是执行期引擎/运行目录/映射/input/result，逐项核对合理 |
| `MISE_ENV_CACHE=0 mise exec --no-deps -- npm test` | exit0；Node496项，473pass / 23skip / 0fail；Python四组22/24/2/13项均OK。不能代替未运行的runtime、真实平台或宿主验收 |

本轮未运行 `npm run test:diagram`、`npm run test:phoenix`、Native构建/CLI收口、宿主新会话或 CI dispatch；没有产品运行时代码改动。UI证据等级为文档/静态依据核对加上述单例真实浏览器交互抽查。所有四维均有实际 reviewer 覆盖，没有把缺失的终验写成通过。

## 映射与实施编排

需求映射没有断链：WORK-F1→§5.1/V-1/V-4；WORK-N3→§3/5.1/6.2/V-1/V-3/V-4/V-5；WORK-F4→§5.2/V-2/V-4/V-6；WORK-F2→§6.1/V-3/V-6；WORK-N1→§6.2/V-3/V-5；WORK-N2→§2/3/5.1/V-1/V-7/V-8；WORK-N4→§5.2/8/V-2/V-4/V-5；WORK-N5→§3/5.2/8/V-1/V-2/V-5。C-1–C-4由既定原生、范围、身份与skill检查承接；C-5由§9.3–9.5/V-6/V-9承接。FU-1→M1/M4，FU-2→M2/M4，FU-3/FU-4→M3/M4；G-04/G-05保留既有批准边界，P3产品能力和公开发行仍明确排除。

两个 main-v2 只收窄这两题的解释路径，来源/语义要求没有删除：[bootstrap](../../skills/phoenix-diagram/scripts/phoenix-bootstrap.cjs):457–482/506–556 与两实际 launcher 支持计划列出的正常路径及配置差异。帮助/安装/项目选择的边界按第325行由公开入口fixture承接，缺覆盖只补精确反例；repair/stall仍有独立正式角色，不用主流程题穷举全部分支。

当前拓扑顺序成立，不需新增里程碑：

| 波次 / 任务 | 前置与解锁证据 | 可并行工作 | 共享负责人 / 运行资源 | 汇合验收 |
| --- | --- | --- | --- | --- |
| 已完成M1/M2/M3 | 对应V-1/V-2/V-3局部记录 | 未失效证据直接引用 | 主实施者核对产品及记录基线 | M4，不扩大为整期通过 |
| V-9支撑 | 当前冻结产品合同及既有collector接缝 | collector视图与题面fixture可独立准备，共享接线单人集成 | 主实施者统一factory/controller/audit/index及身份；不启动模型/浏览器 | V-9局部通过 |
| 单次正式冻结 / 初装 | V-9通过；Source21/D12/main-v2/collector/harness/64角色身份固定 | 可准备CI候选及各题安装 | 主实施者统一pins、真实安装审计及本题fresh；F-08补充可并入此处 | 分别解锁各自终验 |
| V-6各独立题 | 只依赖本题安装、资料、模型路线、fresh证据 | 最多两作者；不等其他题语义或额外16项 | 独立项目/profile；遵守Chrome、负载及磁盘门 | 62正式+2个capture-only反例逐项通过 |
| V-4/V-5/V-7/V-8 | 各自实际依赖和身份已就绪；仅重跑失效项 | 与宿主题独立推进 | 本机完整消费一个浏览器任务；CI独立；主实施者统一候选和五个必需ZIP预算 | 同cohort消费、原聚合与全部适用收口门 |
| M4结束 | V-4–V-9、64角色、原聚合全部通过 | 无额外全量轮次 | 主实施者完成最终提交及SHA回填 | 方可记M4完成并提示review-code |

Codex默认抽样16行不是覆盖上限：[aggregate](../../scripts/aggregate-phoenix-diagram-candidate.mjs):175–184 支持传完整31行。额外两ERD反例保持独立绑定，不塞入scenario表。原 [archive](../../local/phoenix-diagram/p2-gaps-provider-prep-19/archive.mjs):53–57 和 aggregate:296–309/466–477 的严格身份合同不放宽；冻结后改变认证身份的定向复验只记修复诊断，不拼入旧正式记录。限次后仍失败，完成独立检查并准确保留未通过角色，M4继续未完成，不能自动换模或开启新整包循环。

## 总体结论与修订顺序

**是否需要修改：无需。** 本轮只有两项P3，当前建议与计划自报状态一致；没有需要先修改才能实施的确认问题。

- 建议状态：**Ready**。8条WORK及5条约束映射完整，关键外部/身份前提有依据，D21的题面、无损视图、执行次数和失败收尾均有明确边界。上轮F-01–F-07在计划层面已解决，无新增P0–P2。
- 本结论是计划就绪判断。当前实施仍为3/4，V-9尚待实现，新版V-6/V-7与最终聚合尚未完成；旧Source21的2passed/4failed保留，不能据评审Ready关闭M4或原PRD缺口。
- 可选采纳顺序：先F-08，明确接线锚点及fresh局部验证；再F-09，消除旧记录尾段的当前行动歧义。不需要重做产品方案、增加语义预检或新建模型路线。

可直接继续原计划实施：

```text
/goal 按 docs/plan/PHOENIX-DIAGRAM-P2-gaps.md 开发
```

如需先采纳两项非阻塞建议，可选：

```text
$apply-doc-review docs/plan/PHOENIX-DIAGRAM-P2-gaps.review.2026-10-26.md
```

本轮不自动启动上述后续技能或实施任务。
