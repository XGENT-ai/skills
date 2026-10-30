# PHOENIX-DIAGRAM-P3 · 阻塞决策记录

## 对象

| 类型 | 文档 |
| --- | --- |
| 开发计划 | [P3 计划](PHOENIX-DIAGRAM-P3.md) |
| 需求基线（只读） | [PHOENIX-DIAGRAM PRD v1.0](../PRD-PHOENIX-DIAGRAM.md) |
| 评审报告（只读；回读未决项及修订处置） | [P3 计划评审](PHOENIX-DIAGRAM-P3.review.md#修订处置与复核) |

## 处理轮次

| 轮次 | 日期 | 计划版本（处理前 → 后，SHA256） | 代码基线 | 本轮解除 | 新增/重开 | 仍阻塞 | 计划状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 第 1 轮 | 2026-10-28（Asia/Shanghai） | `5e7aa60f8838f0681f54dddb9fe730215bf0b25c2339df167af478fd0b9f3414` → `a40cc8fb78b45b7e9439c5756a2bc9e2da98d729dd3f00a39d3752b73a6563aa` | main / `dce8662270f993cae3521b59ceff8ac6f22e1b63`；dirty，仅文档 | B-01（用户确认） | —；B-01 首次编号，属计划已有问题 | — | Blocked → Ready |

处理前 dirty 路径：`.xgent-ai/experience/index.md`、`.xgent-ai/experience/f1ab0791-df3f-4265-ae5d-b37c89cd87e4.md`、`.xgent-ai/sdlc/status/plan/index.md`、`.xgent-ai/sdlc/status/plan/b41a85fb-9c61-4d64-a2ad-58483ebc7aa9.md`、`.xgent-ai/sdlc/status/prd/06d03c42-b8de-4f89-bddc-e20f595b5fd2.md`、`.xgent-ai/sdlc/status/review/index.md`、`.xgent-ai/sdlc/status/review/152a12f9-044f-4063-91bc-0dbb63c482ad.md`、本计划及评审报告。原有改动保留，产品代码未改。本轮只修订计划、新建本记录并同步既有计划状态页；未提交或推送，成果尚限本工作树。

## 阻塞总览

| B-ID | 阻塞事项 | 来源定位 | 阻塞范围 | 类型 | 结果（最新） | 计划落点 |
| --- | --- | --- | --- | --- | --- | --- |
| B-01 | 无历史所有权回执的 Archify 项目副本应如何移除 | 处理前计划 §0.1 A-1、§0.3 ADR-2、§1.1、§3.2、§11；评审「决策与未完项」第 1 项 | WORK-F11/AC-3、AC-4；M5，继而 M6–M8；最晚 M5 开工前 | 需要决策 | 已解除（用户确认，第 1 轮）；采用确认后直接移除、不备份 | §0.1 C-6/A-1、D6/ADR-2、§1.1、§2、§3.2、§5.4、§7–§11、恢复快照、§12 |

## 逐项记录

### B-01 · 确认后直接移除旧 skill，不备份

- 来源：处理前计划 `A-1` 为阻塞、`ADR-2` 为 Blocked，§3.2 留有 A/B 两条路线；§11 认定数据归属与迁移覆盖未决。评审处置已关闭 F-01–F-03，但明确保留 A-1，没有替用户选政策。
- 阻塞范围：M5 的旧目录处理、安装状态与回滚语义；M6 唯一入口依赖 V-5，M8 的迁移角色和 WORK-F11 终验也受影响。M1–M3 的独立作者工具不依赖该决定。
- 上下文：字节符合已知来源不自动构成所有权；既有历史副本未发现可识别的 Archify 所有权回执。原计划既不能静默认领，也不能在未选择的 A/B 路线下实施迁移。
- 已核实事实：
  - `tools/phoenix-diagram/src/install/plan.rs:182`–227 从有效 active 取得归属，物理身份不同的后来替换即使字节相同也保持冲突；无归属的现存目标不被采用。
  - `tools/phoenix-diagram/src/install/state.rs:23`–32 的 managed 边界仅为所选 provider 下的 `skills/phoenix-diagram`；`:83`–116 验有效 active 的项目身份、checksum 和文件归属。
  - `bin/xgent-skills.js:62`–164 识别/清理的旧工具是 impeccable；`:410`–412 有询问后删除且不备份的既有 UI 形态，但其个人目录/hooks 范围不能直接带入本产品。
  - `tools/phoenix-diagram/upstream/SOURCE.json` 的 legacy.fileCount 为 70，只是 oracle 闭包。只读 `git ls-tree -r 4a65a1525f9ed03c775a0e18935901e1edbb773f -- skills/archify` 与 `git ls-files skills/archify` 均得到 219 个文件；该固定版本与当前目录仅 SKILL.md 增加一行 when_to_use。完整来源仍用于识别原版，不能以 70 文件表替代全目录检查。
- 约束：PRD `WORK-F11/AC-3`（第 276 行）原要求旧入口有可定位备份，`AC-4` 要求真实事务及后来修改保护；计划 C-5 仅允许临时项目验收，不实际处理活跃/个人安装，不接管外部管理入口。用户的新决定明确覆盖旧 skill 备份政策，其余原有保护未要求放宽。
- 选项：

  | 选项 | 做法 | 后果与代价 |
  | --- | --- | --- |
  | A（原推荐，未选） | 显式迁移时按完整来源和物理快照严格认领，备份后替换 | 可迁移无回执的原版副本，但增加所有权转换与旧目录备份/恢复机制 |
  | B（未选） | 仅接受已有有效所有权回执，无回执报 legacy-unowned | 不增加认领政策；现有无回执副本无法自动迁移，只能手动处理 |
  | C（用户提出并选定） | 识别原版、展示精确范围，用户确认后直接移除旧 skill，不备份 | 不转换旧目录所有权；删除后不自动恢复旧 skill，需重新下载安装；新入口受管事务继续可恢复/回滚 |

- 原推荐与理由：A 能覆盖 PRD 所要求的旧项目接替，且沿用原备份与冲突保护；查证无法代替用户决定数据归属政策。用户明确选择了不同恢复取舍后，以 C 为最终路线，不再要求重复批准 A 或备份机制。
- 用户确认原话（2026-10-28，Codex 问答）：**“用户确认后，直接移除 Archify ，不用备份，只是skill而已，随时都可以下载回来”**。
- 后果说明：已告知删除后的旧 skill 不由安装失败恢复或回滚重建，恢复须重新下载安装；新入口事务、后来修改保护及个人/外部管理范围边界保持。没有把该答复解释为立即删除当前安装，也没有扩大到自定义或未知来源目录。
- 后续动作：M5 按计划 §3.2/§5.4 实现。`--dry-run` 展示精确目标、不备份后果及重装指引；实际移除要求 `--confirm-remove-archify`，联合入口可将用户肯定答复透传；未确认、拒绝、EOF 或无交互且缺标志不迁移。journal 仅记录旧目录删除事实/进度，不保存其内容；新入口恢复和回滚单独核对。此处是产品合同，本轮没有运行这些待实现命令。
- 结论：**已解除（用户确认）**；采用 C，A-1 转入已确认 C-6，ADR-2 改为 Accepted。PRD 本轮只读，计划显式登记用户决定替代原 WORK-F11/AC-3 的备份要求，不以新口径声称原备份条款通过。
- 计划修订：同步顶部范围摘要/需求来源、需求账本、D6/ADR-2、事实缺口、请求/回执、删除与新入口恢复边界、NFR/失败模式、V-5、迁移角色题面、故障注入、M5/M8、§11 及恢复快照。固定历史来源只用于识别/重装依据；既有源码/许可保留要求不变。
- 复核：已核对需求 → 决策 → 接口/数据 → M5/M8 → V-5/V-8/V-9 的追踪；没有遗留待选 A/B 或旧 skill 备份/回滚承诺。新入口及比较产物自己的备份不在本次取消范围内。

## 其他前置核对

- §4、§9.2 和 M3/M4 的真实资源是明确排在实施期的交付及执行门：M3 准备提交、授权发布、匿名 HTTPS 的 URL/提交/125 bytes/SHA 核对先于 V-4。该 PNG 和 P3 功能尚未交付，不能把缺少未来产物视为已运行验收失败，亦不提前填通过。只读 `gh repo view --json nameWithOwner,isPrivate,viewerPermission,defaultBranchRef` 返回 `XGENT-ai/skills`、isPrivate=false、viewerPermission=ADMIN、main，仅说明现有入口，不证明已获本任务发布授权或资源可取回。
- 评审「修订处置与复核」已将 F-02 的资源编排改为 M1–M3 准备、M4 使用；F-01 比较审核合同和 F-03 新旧样本布局也已处置。本轮没有重开已处理意见。P2 G-07/G-08 的既有限制保留。
- 未发现需要依赖项目修改能力或补齐契约的外部缺口，故没有创建 CRD。尚未执行的资源准备、真实四平台/provider、浏览器及产品验收均在原计划执行门处理，不等于已解除这些将来的验收任务。

## 未解除阻塞

无。已解除 1 项；新增/重开 0 项；仍阻塞 0 项；需用户当前处理 0 项。

## 复核与验证

- `mise exec --no-deps -- npm test`：exit 0。Node 479 项：449 pass、30 skipped、0 fail；Python 三组 24 + 2 + 13 = 39 项通过。
- `bash skills/dev-plan/scripts/check_paths.sh docs/plan/PHOENIX-DIAGRAM-P3.md .`：95 个路径 token，MISSING 38 个，与处理前同一组；均为已标注的新交付或 M4 输出，无新增缺失复用锚点。
- `bash skills/dev-plan/scripts/check_progress.sh docs/plan/PHOENIX-DIAGRAM-P3.md .`：0/8、ERROR 0、WARN 1。计划本来未被 Git 跟踪，且已有其他文档改动，触发陈旧提示；核对没有实施完成行，不虚增进度。
- `python3 skills/dev-plan/scripts/check_plan.py docs/plan/PHOENIX-DIAGRAM-P3.md`：ERROR 0、WARN 9，均为原有 WORK-F*/WORK-N* 自定义 ID，已按 PRD 及账本复核。
- `git diff --check` 通过；未跟踪计划另对处理前副本逐段核对差异。仅政策相关章节及真实基线/状态发生变化，固定实施协议、其他能力路线、完成记录保持；PRD、评审报告、产品代码与实施记录未写入。
- 未运行 `npm run test:diagram`、`npm run test:phoenix` 或真实浏览器/平台/provider 验收：没有修改 runtime。以上仅为文档一致性和轻量仓库检查，不是 P3 产品验收或独立全量复审。

## 就绪结论（最新一轮）

- 计划状态：**Blocked → Ready**。
- 理由：原唯一政策阻塞已有用户明确决定，计划已按直接移除/不备份统一接口、失败、恢复与替代验收含义，无未决的关键开工范围、数据归属、外部契约或 NFR。
- 再次运行前需要：无。实施仍从 M1 开始，进度保持 0/8；M4 前按 §9.2 补资源授权和真实可取回证据，M8 按既定门核实际四平台/provider 资源与结果。
- 后续入口：`/goal 按 docs/plan/PHOENIX-DIAGRAM-P3.md 开发`。本轮不自行启动实施。
