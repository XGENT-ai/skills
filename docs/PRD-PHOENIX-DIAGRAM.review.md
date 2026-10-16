# PHOENIX-DIAGRAM PRD 评审报告

评审 ID：68af25e5-856b-4d48-9b73-b3f892abb419  
调查日期：2026-10-16（Asia/Shanghai）  
模式：全文初次评审；四个只读 reviewer 独立调查，主 agent 交叉复核并单写报告。

## 评审对象

| 类型 | 原文档 | 所审版本 | 评审范围 |
| --- | --- | --- | --- |
| PRD | [PHOENIX-DIAGRAM · Rust 图解工具](PRD-PHOENIX-DIAGRAM.md) | v1.0；SHA-256：e099e47858cc9efeecdda26e1786242a2393cf50d0198538fc0817b144e4263b | 全文 §0–§8，含 11 条功能需求、5 条非功能需求、全部 AC、UAT 与 P1/P2/P3 分期 |

## 1. 确认问题

**未发现可确认的问题。** 当前没有需要在进入开发/测试计划前写回 PRD 的阻塞缺陷。

交叉复核后，以下内容不判为缺陷：

- 参考 lifecycle v3 拒绝现有 v1，是已核实的迁移增量；[WORK-N3](PRD-PHOENIX-DIAGRAM.md#L313) 已承接核心语义、固定几何、迁移副本及废弃字段的处置，没有假称参考版天然兼容。
- 参考版可选视觉审核与现有默认审核不同；[D5 与 C3/C4](PRD-PHOENIX-DIAGRAM.md#L377) 明确保留默认审核、采用受约束纵向页面滚动，属于已确认选择。
- Rust 核心、diagram 分发和十类能力是未来目标。在已查入口中尚未提供的目标能力，不构成这份需求文档的错误；现有 Phoenix UI 分发也没有被写成 diagram 已经可用。
- 部分保护条目的终验在 P3，不代表早期允许丢失事实、泄露内容或伪造成功。[§7 首段](PRD-PHOENIX-DIAGRAM.md#L438) 要求所有已交付部分持续满足质量与授权门槛，[P1 前置](PRD-PHOENIX-DIAGRAM.md#L444) 已包含固定源码、许可与匹配资源。

## 2. 待核实 / 待决策

没有新增的阻塞产品决定。下表是 PRD 已明确保留的下游证据节点，不是本轮发现的虚假现状，也不要求在 PRD 评审时提前实现产品。具体负责人尚未指定。

| 编号 | 尚需取得的证据 | 责任角色 / 应满足节点 | 对产品及交接的影响 |
| --- | --- | --- | --- |
| E-01 | 可取回的固定参考源码身份、Phoenix 修改与完整第三方许可 | 本仓维护者；实现前固定来源，候选发行时核对分发材料 | 本地参考目录没有 .git。遵守 [WORK-N2 与 §4](PRD-PHOENIX-DIAGRAM.md#L303)，不能将目录版本号冒称上游提交，也不能以未经核对的 latest 替换基线。无需改写既有要求。 |
| E-02 | darwin-arm64、darwin-x64、linux-x64、linux-arm64 的匹配原生资源、在线/离线安装及最终发行包证据 | 开发与发行承接者；各期候选验收前，P1 先满足原五类图闭环 | [WORK-F11](PRD-PHOENIX-DIAGRAM.md#L274) 明确四平台、摘要和引擎身份要求；本轮未运行安装或原生平台验收。 |
| E-03 | 目标 provider 的真实加载、入口发现与重载结果；迁移后的回执消费与文档嵌入检查 | 接入项目维护者及下游验收承接者；首次安装与 P3 迁移验收时 | [WORK-F10/F11](PRD-PHOENIX-DIAGRAM.md#L255) 已有产品边界。文件复制或安装器测试不能证明宿主加载成功；更新调用方时须连同截图、回执和最终嵌入验收约定一起适配。 |
| E-04 | 确切 HTML 的真实浏览器结果、图像审核、导出能力、语言显示与窄屏可读性 | 作者 / 具备图像能力的审核者及下游验收承接者；每期相应类型交付时 | [WORK-F4/F5/F6/N1](PRD-PHOENIX-DIAGRAM.md#L168) 已定义失败、缺席与格式降级。本轮没有浏览器或视觉实测，不能据此声称产品效果通过。 |

## 3. 代码现状核实矩阵

“已有满足”仅表示现状主张有依据，不表示 Phoenix 目标能力已完成。源码证据与实际执行分列；参考包是本地快照，未核对上游线上最新版本。

| PRD 主张 / 定位 | 类型 | 实际行为与证据等级 | 与目标的关系 | 产品影响 |
| --- | --- | --- | --- | --- |
| [§1.1：现有包为 2.17.0-dev.1、五类图、Node 执行](PRD-PHOENIX-DIAGRAM.md#L16) | 现状 | [包版本](../skills/archify/package.json#L3)、[五类注册](../skills/archify/bin/archify.mjs#L13)、[renderer 选择与 Node 启动](../skills/archify/bin/archify.mjs#L54)；静态源码核实 | 已有满足现状描述；Rust 为真实增量 | 不把改名等同于引擎迁移。 |
| [§1.1：本地参考为 3.1.0，记录日期为 2026-10-14，CLI 十类](PRD-PHOENIX-DIAGRAM.md#L23) | 本地参考事实 | [包](../local/archify-main/archify/package.json#L3)、[本地发布记录](../local/archify-main/CHANGELOG.md#L7)、[十类注册](../local/archify-main/archify/bin/archify.mjs#L12)；目录检查确认没有 .git | 已有满足本地描述；上游提交身份未核实 | PRD 已区分本地参考与可取回固定源码，没有声称已核对上游最新提交。 |
| [§1.1：参考 lifecycle v3 拒绝现有 v1](PRD-PHOENIX-DIAGRAM.md#L25) | 兼容现状 | 主 agent 与 facts reviewer 分别直接调用内置 validateSchema；[旧示例](../skills/archify/examples/deployment-release.lifecycle.json#L2) 在旧校验器中通过，在参考校验器中失败；[参考 schema](../local/archify-main/archify/schemas/lifecycle.schema.json#L7) 要求 mainPath 与版本 3，并拒绝 lanes、viewBox 等字段 | 已核实；兼容是必要的真实增量 | WORK-N3 已要求保留有效意图并报告未解决缺口，不能以直接拒绝输入作为迁移完成。 |
| [§1.1：没有保存偏好时从 Still 开始](PRD-PHOENIX-DIAGRAM.md#L26) | 阅读器现状 | [readStored](../local/archify-main/viewer/motion-governor.js#L515) 读取偏好；[初始化](../local/archify-main/viewer/motion-governor.js#L671) 仅保存的 live 会取消暂停；[运行说明](../local/archify-main/archify/references/viewer-runtime.md#L34) 一致；静态源码 | 已有满足；未做浏览器验证 | PRD 正确处理了发布记录中 Live by default 的表述歧义，首次静态与主动 Live 的产品语义明确。 |
| [§1.1、D5：默认视觉审核与滚动政策有差异](PRD-PHOENIX-DIAGRAM.md#L27) | 既有规范 / 已确认目标 | [旧默认审核与桌面首屏要求](../skills/archify/references/delivery-contract.md#L103)；[参考纵向例外](../local/archify-main/archify/references/delivery-contract.md#L437)、[参考可选审核](../local/archify-main/archify/references/delivery-contract.md#L581)；文本核实 | 差异已明确裁定，不是未处理冲突 | 保留默认真实审核，允许可读纵向页面空间，禁止裁剪、内部图滚动或缩字伪通过。 |
| [WORK-F4：连续自动门、确切版本证据、视觉单独裁定](PRD-PHOENIX-DIAGRAM.md#L173) | 目标 / 可参考契约 | [参考四阶段定义](../local/archify-main/archify/bin/finalize.mjs#L22)、[连续门说明](../local/archify-main/archify/references/delivery-contract.md#L263)；[现有三类结论](../skills/archify/references/delivery-contract.md#L27) 已区分确定性、浏览器和视觉结果；仅静态核实 | 参考结构可借鉴；Phoenix 实现为真实增量 | PRD 没有将基础检查、自动通过与完整通过混同；浏览器缺席和执行失败有不同结果。 |
| [§1.1、WORK-F11：参照 Phoenix 分发](PRD-PHOENIX-DIAGRAM.md#L28) | 可复用契约 / 目标 | [npm 入口](../package.json#L15) → [只读预检与 dry-run](../bin/xgent-skills.js#L73) → [大小/摘要/引擎身份](../skills/phoenix-ui/scripts/phoenix-bootstrap.cjs#L88) 与 [离线清单核对](../skills/phoenix-ui/scripts/phoenix-bootstrap.cjs#L239) → [正式安装调用及失败反馈](../bin/xgent-skills.js#L494)；静态调用链 | UI 机制可参考，diagram 为真实增量 | 当前路径、cache、manifest 和工具身份仍专属 UI；PRD 已要求 diagram 状态可辨认且不得影响 UI。未执行安装。 |
| [WORK-F11：provider、恢复、回滚与后来修改保护](PRD-PHOENIX-DIAGRAM.md#L275) | 参照规则 / 目标 | [provider 选择与 Codex 展开](../tools/phoenix-ui/crates/install/src/cli.rs#L119)、[锁内恢复与回滚](../tools/phoenix-ui/crates/install/src/transaction.rs#L400)、[回滚前校验事务、备份及恢复项](../tools/phoenix-ui/crates/install/src/transaction.rs#L958)；[普通命令仅核对现有引擎](../skills/phoenix-ui/scripts/phoenix-bootstrap.cjs#L302)；静态源码 | 有参考依据；diagram 的受管迁移与隔离仍待实现 | 不把 UI 的旧 impeccable 来源记录直接当成 archify 的可认领依据；PRD 没有作此承诺。 |
| [WORK-F11：四平台支持范围](PRD-PHOENIX-DIAGRAM.md#L270) | 已确认产品约束 | [Phoenix 支持矩阵](phoenix-ui.md#L72) 为 macOS/Linux 四平台；bootstrap 内部另识别可选 windows-x64，见 [TARGETS](../skills/phoenix-ui/scripts/phoenix-bootstrap.cjs#L12)；文本/源码核实 | 与用户确认范围相容 | 内部能识别 Windows 不等于本 PRD 必须承诺 Windows；原生平台验收仍未执行。 |
| [§1.1、WORK-F10：当前调用方与 fallback](PRD-PHOENIX-DIAGRAM.md#L29) | 现状 / 接口迁移 | [elintp 探测、fallback 与离线嵌入](../skills/elintp/SKILL.md#L122)、[documentation-writer 探测](../skills/documentation-writer/SKILL.md#L124)、[截图/回执消费约定](../skills/documentation-writer/SKILL.md#L146)、[架构参考](../skills/documentation-writer/references/architecture.md#L111)、[用户手册参考](../skills/documentation-writer/references/user-manual.md#L83)、[README](../README.md#L228)；文本核实 | 已有调用关系已覆盖；替换为真实增量 | F10 与 C7 不只要求改名，也要求适配能力与回执、保留各调用方授权和最终文档验收；当前旧引用不是 PRD 缺陷。 |
| [WORK-N1：五种阅读器语言](PRD-PHOENIX-DIAGRAM.md#L294) | 参考清单 / 目标 | [manifest](../local/archify-main/archify/locales/manifest.json#L2) 枚举 en、zh-CN、zh-TW、es、ko；仅核实清单，未运行阅读器或检查全部译文 | 部分核实；Phoenix 显示质量为未来验收 | PRD 已区分界面翻译、作者内容和回退，未假称已实测。 |
| [§1.1：SPMS 未重复立项](PRD-PHOENIX-DIAGRAM.md#L31) | 当前外部登记事实 | 2026-10-16 实际只读查询：指定项目 requirement_list，limit=100、offset=0，items=[]、hasMore=false；phoenix-diagram / phoenix / infographic 搜索均为空且未截断；archify 仅返回 Issue BUG-95，未截断 | 当次查询与 PRD 一致 | 未创建或修改需求、Issue、TC、项目基本信息；未将登记状态当作实现状态。 |

## 4. 覆盖与基线

### 4.1 版本与工作树

- 仓库：XGENT-ai/skills；本机路径 /Users/rockie/Documents/GitHub/xgent/skills。
- HEAD：4a65a1525f9ed03c775a0e18935901e1edbb773f；当前工作树参与调查。
- 初始 dirty：.xgent-ai/sdlc/index.md、.xgent-ai/sdlc/status/index.md 已修改；.xgent-ai/sdlc/status/prd/、docs/PRD-PHOENIX-DIAGRAM.md 未跟踪。它们是本轮开始前的状态。
- 评审规则取本仓 skills/review-prd 的源文件及 report-contract / review-lenses，并以 skills/prd/references/product-quality.md 校准产品维度。
- PRD 与源代码只读；本轮写入仅为独立报告及已有 SDLC 协议要求的评审索引/状态。结束前复核 PRD 身份与相关调查文件，变化只影响对应结论时才重审。

### 4.2 四个审查视角

| reviewer | 实际覆盖 | 专属结论 / 主 agent 复核 |
| --- | --- | --- |
| facts | 本地旧/新版本、输入兼容、默认播放与审核、分发调用链、当前调用方 | 没有事实类 findings；主 agent 复读关键来源，独立复现 lifecycle 差异并核对 SPMS。 |
| product | 范围与取舍、十类图价值、事实完整性、宿主 Agent 闭环、分发及迁移授权 | 没有必要产品修订；主 agent 核对已确认边界，只保留一个可选增强机会。 |
| experience | 首次安装、作者修复、离线阅读、探索/导出、维护迁移；主题、窗口与异常状态 | 没有可确认体验阻塞；结论限于文档与静态材料，不冒充真实 UI 验收。 |
| acceptance | SMAR、目标/场景/AC/UAT、逐条期次、跨期终验及持续保护 | 53 条 AC 分配一致；主 agent 用独立只读脚本核对数量、期次、遗漏及重复。 |

四个 reviewer 均为独立只读 subagent，没有派生更多 agent，也没有写最终报告。结论由主 agent 根据原始证据裁定。

### 4.3 六个产品设计维度

| 维度 | 实质结论 |
| --- | --- |
| 创新性 | 连续完成意图整理、候选、定向修复、当前版本检查与交付，减少作者自行串联命令和判断证据的工作；不只是换目录名。 |
| 先进性 | 十类图、来源与时间依据、完整事实及可复用导出形成明确价值；Rust 是用户约束，PRD 没有以技术名称替代收益或承诺未知生态兼容。 |
| 可用性 | 作者、读者、接入维护者及本仓维护者均有完成路径；缺事实、无路径、依赖缺席、检查失败、冲突及恢复均有结果与下一步。 |
| 便捷体验 | 自然语言 / Mermaid / JSON 输入、连续交付、独立诊断与主动预览覆盖首次及高频任务；没有要求用户为常规使用掌握 Rust。 |
| 美观 UI | 主图与标题优先、详情按需展开、暗亮一致、四桌面窗口及窄屏完整性、遮挡与滚动边界均可核对；真实视觉审核资源缺失时不能完成该门。实际布局美观尚未验证。 |
| AI 赋能 | 宿主 Agent 有明确输入权限、图型选择、事实整理、来源核对、候选与修复输出；未知事实不补造、停滞要停止、原材料不隐式改写。无 Agent 的已有 JSON 路径仍成立，不需要为本工具额外设计模型服务或订阅。 |

### 4.4 实际证据与验证限制

已完成：文档与源码调查、旧/参考校验器的直接执行对照、AC 分期映射检查、PRD 中 24 处本地链接存在性检查，以及上述 SPMS 只读查询。没有把参考目录中已有截图或历史测试当成本轮 UI / 性能证据。

仓库轻量回归执行了 npm test，**退出码 1，未全量通过**：

- Node：272 项，264 通过、8 跳过、0 失败。
- test/codex：22 项通过。
- test/dev-plan：24 项中 3 项因 check_progress.sh 子进程超过 10 秒而报错：test_final_state_rejects_pending_rows、test_final_state_with_implementation_and_fix_shas_passes、test_legacy_table_without_commit_column_only_warns。调用及超时口径见 [测试定义](../test/dev-plan/test_check_progress.py#L73) 与 [子进程封装](../test/dev-plan/bash_check.py#L6)。
- runner 在该组失败后停止，后面的 test/skills 与 agi-mode/evals 未执行。相关脚本未被本轮改动；没有将超时推断为 PRD 缺陷，也没有在本次评审中扩展修复。复现日志暂存于 /tmp/phoenix-diagram-prd-review-npm-test.log。

本轮未运行 Phoenix runtime、浏览器、原生四平台、真实 provider 会话、生成后的 UI/导出验收或收费模型评测；没有安装、迁移、启动预览服务、公开发布或写 SPMS。以上限制不会被本次 PRD 通过结论覆盖。

## 5. 需求追踪与分期终验

### 5.1 SMAR 与任务闭环

| 检查项 | 核对结果 |
| --- | --- |
| Specific | [角色与授权](PRD-PHOENIX-DIAGRAM.md#L74)、十类语义、旧/新 JSON 核心与展示字段、自动/浏览器/视觉结论及源版本身份均有边界。 |
| Measurable | 四桌面窗口、错误/警告门、摘要与字节一致性、事实不丢失、成功/失败/未做和回滚保留条件可判断；没有无依据的工期或真人样本门槛。 |
| Actionable | S1–S7 均能指向业务动作、结果状态与 UAT；需求不是内部模块或工具名的清单。 |
| Reasonable | Rust、四平台及默认审核遵从已确认约束；模型、浏览器、图像能力和固定源码被明确列为前提，缺席时不伪装成功。 |

[§6.1–§6.3](PRD-PHOENIX-DIAGRAM.md#L399) 中 R1–R6、G1–G4、S1–S7 均有承接。逐项核对安装与缺引擎、自然语言/Mermaid/JSON 创作、脏工作树与缺时间记录、当前证据失效、离线探索与导出降级、迁移冲突及回滚、主动预览与比较，未发现关键目标或场景悬空。UAT 描述预期状态，没有把 TC、造数或测试脚本误设为 PRD 必备章节。

### 5.2 AC 分配与每期价值

独立脚本从正文逐需求提取 AC，再与 [§7.2 分期矩阵](PRD-PHOENIX-DIAGRAM.md#L450) 双向比对：16 条需求、53 条 AC，期次无错配、遗漏或重复。

| 期次 | 分配 AC 数 | 独立用户价值与出口 | 规模判断 |
| --- | --- | --- | --- |
| P1 | 23 | 原五类图形成安装 → Rust 生成 → 离线阅读/导出 → 真实审核闭环；固定源码/许可、匹配原生资源、浏览器与图像能力均为前置 | 建议 5–8 个 Milestone，具体拆解由 dev-plan 核实。 |
| P2 | 14 | 全十类图、仓库与时间依据、完整探索能力；沿用 P1 质量门 | 建议 5–8 个 Milestone，超预算反馈拆期，不能删终局要求。 |
| P3 | 16 | 旧入口迁移与调用方接手、品牌、主动预览、架构比较及维护恢复 | 建议 5–8 个 Milestone；连续性例外 9–10 个须说明理由，最多 10 个。 |

[§7.3 终验表](PRD-PHOENIX-DIAGRAM.md#L473) 与正文、分期矩阵及 [§7.4 交接](PRD-PHOENIX-DIAGRAM.md#L491) 一致：

- WORK-F1、WORK-F2、WORK-F4、WORK-F5、WORK-F6、WORK-N1 共 6 条在 P2 终验。
- WORK-F11、WORK-N2、WORK-N3、WORK-N4、WORK-N5 共 5 条在 P3 终验。
- 11 条跨期需求均要求阶段交付后停在交付段，不能提前把整条转为已上线。持续保护见 §7 首段，不因终验较晚而延期生效。
- 未录入 SPMS 的 WORK 工作号、未来真实 key 回填、稳定 AC ID、期次与终验 release 的关系已有说明。没有真实 key 或尚未创建 TC 不构成断链。

## 6. 产品增强机会

**可选：嵌入离线文档后，复制阅读状态链接仍能回到宿主文档中的对应图。**

[elintp](../skills/elintp/SKILL.md#L136) 要求使用 iframe srcdoc 保持单文件离线交付；[参考版嵌入地址契约](../local/archify-main/archify/references/viewer-runtime.md#L17) 有明确宿主地址接缝。[readerLink](../local/archify-main/viewer/template.source.html#L701) 缺少配置时使用当前 frame 地址，[Focus 复制动作](../local/archify-main/viewer/focus.js#L1395) 消费该结果。由此可提出减少无效 iframe 地址分享的机会；这里只做静态推演，未复现浏览器行为。

若采纳，需明确 WORK-F5/F10 在嵌入场景中的地址恢复与无有效地址时的失败结果；代价是调用方需要额外衔接。不得因此引入托管服务或网络依赖。当前 PRD 承诺独立阅读器深链接和嵌入完整性，没有承诺嵌入后的地址共享，故本项不进入 findings，也不阻塞交接。

## 7. 总体结论与修订顺序

**是否需要修改：无需。**

下一步：

    $dev-plan docs/PRD-PHOENIX-DIAGRAM.md P1

**建议结论：通过。** 在已核实范围内，这份 PRD 可以作为开发与测试计划的输入；没有确认的交接阻塞，也没有新增必须写回的产品决定。原 PRD 保持原字节。

无需安排正文修订。下游按既有 P1 → P2 → P3 承接：先固定可取回源码与许可、明确分发适配增量；再实现本期完整用户路径与当前产物证据；随后逐期满足 UAT、真实平台/provider 证据和终验。现有调用方的截图、回执消费与最终文档检查须按 F10/C7 一起适配。若选择采纳上述增强机会，再形成单独的产品决定与相应 AC。

本次通过仅覆盖产品需求质量与已核实的现状描述，不表示功能已实现、性能已实测、UI 已验证、仓库全量回归通过或可以公开上线。
