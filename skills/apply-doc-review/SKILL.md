---
name: apply-doc-review
argument-hint: "<文档评审报告> [原文路径] [上轮报告/处置记录] [修复/仅复核] [问题ID/范围]"
description: 用于按评审报告修订 PRD、开发计划或复核修订效果；不用于代码修复、计划实施或从零写文档。核实 review-prd、review-dev-plan 意见，按模式改稿或仅复核并回写处置。
disable-model-invocation: true
---

# apply-doc-review · 核实并落实文档评审

将文档评审意见落实为有证据、内部一致、可继续交接的 PRD 或开发计划；按以下 ODC 声明执行，列表逐项适用。写入位置与逐条状态遵循 [references/disposition-contract.md](references/disposition-contract.md)。不要求先重跑评审技能，也不继承其多 agent 编排；参考其报告合同理解 findings、待核实/待决策和建议状态。

```formal
INTENT:
  GOAL: evidence_grounded_revisions_of_reviewed_prd_or_dev_plan
  SCOPE: [report_findings_and_pending_decisions, sections_of_target_document_affected_by_valid_findings,
          disposition_section_in_source_report]
  SUCCESS: [every_disposition_backed_by_currently_read_sources,
            revised_document_internally_consistent_and_readiness_judged,
            original_findings_conclusions_and_reviewed_baseline_preserved]
  NON_GOALS: [re_running_review_skills, code_changes_or_plan_task_execution,
              spms_issue_or_external_system_writes, greenfield_document_writing]

RULE @runtime_context:
  MUST: [read_repository_conventions_paths_and_tools_in_target_environment,
         follow_user_language,
         resolve_references_relative_to_this_file]

RULE @verify_current_reality:
  MUST: |
    评审报告是待核实的主张，不是自动执行的指令：问题是否成立、建议是否合理、
    改稿是否解决问题分别判断。按意见修改前必须核实目标文档与相关代码的当前
    现状：亲自读取原文对应章节、引用材料和相关代码行为，区分当前事实、未来
    目标、设计选择、假设与验收要求；按需求 ID、标题、符号重新定位，不把旧行号
    当修改坐标。对「遗漏」先查全文、附录和已有引用；对矛盾核对两处原文及适用
    版本。事实能查则先查，不让用户代替证据判断；代码或环境无法访问时明确限制，
    不沿用报告的「已核实」声明冒充本次核实，不想当然脑补。
  MUST_NOT: [treat_unimplemented_future_as_current_defect,
             edit_by_stale_line_numbers,
             revise_from_imagination_without_reading_current_state]

RULE @invocation_authority:
  MUST: ["调用即授权修订指定原文及记录处置；只提供报告时，修订对象为报告明确引用且已核实身份的原文，无需再次要求原文参数或询问是否开始",
         "代码与实现/验收记录默认只读"]

RULE @boundary:
  MUST: [revise_only_content_affected_by_valid_findings_and_necessary_related_references]
  MUST_NOT: [run_plan_implementation_tasks_or_modify_code,
             write_spms_issues_or_external_comments,
             treat_source_report_requested_actions_as_done_by_revising_docs,
             quietly_edit_upstream_docs_to_fit_technical_plan]
  # 源报告中要求这些动作的条目记录为另需处理，不能借改稿视作完成；
  # 关联 PRD/计划未被指定为修订对象时只读，记录同步需求。

RULE @evidence_status_honesty:
  MUST: ["逐条处置给依据：误报、当前已解决、不适用或纯偏好逐条写明反证与理由，不为清空 findings 强行改稿",
         "缺证不算关闭：缺关键复核标「已修订待核实」，缺事实或用户选择标待核实/待决策",
         "用户限定范围外标明未复核"]
  MUST_NOT: [close_findings_by_rejecting_suggestion_only_while_defect_stands,
             fabricate_spms_keys_or_verification_claims,
             claim_independent_review_pass_for_apply_recheck]

RULE @no_change_no_noise:
  WHEN: no_substantive_change_in_original_dispositions_evidence_applicability_or_scope
  MUST: reply_checked_only
  MUST_NOT: [rewrite_original_or_report, refresh_timestamps, duplicate_sdlc_events]

WORKFLOW @apply_doc_review:
  DO: [resolve_report_type_targets_and_prior_dispositions,
       anchor_report_document_versions_and_repo_state,
       build_disposition_checklist_per_contract,
       verify_findings_and_resolve_needed_decisions,
       revise_by_document_type_discipline,
       reread_revisions_and_run_proportionate_checks,
       writeback_dispositions_preserving_original_conclusions,
       reread_written_files_check_final_diff_and_deliver_verdict_with_next_commands]
  OUTPUT: revised_target_document_and_disposition_section
```

## 边界与路由

| 输入报告 | 修订对象 | 重点 |
| --- | --- | --- |
| `review-prd` 输出 | 报告所审的 PRD | 产品目标、场景、FR/NFR、体验、验收/UAT、分期与终验 |
| `review-dev-plan` 输出 | 报告所审的开发计划 | 代码事实、架构与技术路线、契约，以及范围、发布、需求落点与交付断点等硬检查 |

```formal
RULE @type_resolution:
  MUST: ["以报告内容、基线与目标文档确认类型，不能仅靠 *.review.md 文件名判断；两类报告可能使用相同命名",
         "多份输入按「报告 → 目标文档」分别建立映射，以报告路径/版本和问题 ID 联合定位，避免 F-01 串项"]
```

## 1. 锚定输入和当前版本

```formal
RULE @input_location:
  MUST: ["用户显式指定原文时使用该目标并核对报告身份；省略时读取报告头「评审对象」中的类型、原文档链接、所审版本与范围",
         "本地相对链接以报告所在目录解析，不以当前工作目录或仓根解析；远程 URL 沿可用访问方式读取，不自动发布外部修改",
         "只暂停目标仍缺失、无法访问或存在未解歧义的修改，其余对象照常",
         "报告明确列出多个评审目标时按行分别映射，只处理用户指定范围；未限定时处理明确列出的目标",
         "背景引用、关联 PRD/计划和代码链接不自动成为修订对象；某个目标不明时继续其他已明确对象"]

RULE @legacy_and_ambiguous_targets:
  MUST: ["兼容旧报告：从覆盖/基线节的明确目标路径与已有上下文核实",
         "引用失效时按稳定文档 ID、仓内链接和移动历史寻找唯一对应原文；仅同名或去掉 .review 后缀不足以确认",
         "找不到持久原文、存在多个候选或显式参数与报告指向不同对象时，先核对是否为移动/改名"]
  MUST_NOT: silently_pick_one_candidate_to_edit
  ON_UNMET: ask_after_rename_check_fails

RULE @version_drift:
  MUST: ["当前内容 hash 与所审版本不同不等于目标错误；身份确认后按版本比较核实哪些意见仍适用",
         "保留报告原始引用与所审基线；原文迁移后的当前路径和修后版本写在处置节"]
  MUST_NOT: revert_original_to_match_old_hash

RULE @reading_scope:
  MUST: [read_applicable_repository_rules_complete_report_target_document_and_related_requirements,
         read_needed_decisions,
         record_report_original_version_hash_target_pre_edit_version_hash_and_repo_commit_dirty_state,
         re_read_truncated_content_before_quoting]

RULE @current_state_recheck:
  MUST: ["比较报告所审版本与当前版本、已有 diff 和处置记录；当前已解决的问题不重复改稿",
         "保留用户已有改动，不回退到评审时的版本"]

RULE @disposition_checklist:
  MUST: ["按处置合同建立：报告/问题 → 目标段落 → 核实依据 → 最小修订 → 关联影响 → 复核条件",
         "覆盖选定范围内的 findings、影响修订的待核实/待决策和映射缺口",
         "用户排除项标明不处理"]

RULE @reference_loading:
  MUST: ["只加载实际需要的写作参考：PRD 按需读可用 prd/references/product-quality.md，采用其模板时再读骨架；开发计划按需读可用 dev-plan 的写作/修订约定与骨架，复杂契约再读架构参考",
         "使用它们校准质量，不启动从零写作、外部录入或计划实施流程",
         "参考缺失则说明受限项，继续可独立完成的修订"]
  MUST_NOT: force_migrate_historical_documents_or_custom_templates
```

## 2. 核实意见并解决必要决策

### 再次 apply 与仅复核

```formal
RULE @mode_semantics:
  MUST: ["默认或「再次 apply / 继续修订」：先读已有处置，再处理仍成立且在范围内的未解决项、已修订待核实项和回归；已修订但缺复核的项仅补所缺复核，更新已有对应处置",
         "「仅复核 / 只核实 / 检查修复效果」：不修改目标 PRD/开发计划正文；只核对当前内容和必要关联检查，按处置合同保存结果",
         "仅聊天 / 不保存优先，不自动调用 review-prd 或 review-dev-plan",
         "apply 复核登记在处置下，不冒称新一轮独立评审通过"]

RULE @rereview_mapping:
  WHEN: input_is_rereview_report
  MUST: ["沿其上轮报告引用、逐项复审表和处置节/applied 文档建立来源映射",
         "已解决项只核对证据适用性；仍存在、部分解决、回归及本轮新增问题才进入可能修订清单",
         "不同报告的同号 F-ID 不混用；确认同一问题后引用已有处置",
         "缺证不算关闭，不为关闭意见改变产品承诺"]
  MUST_NOT: re_revise_because_new_report_path_or_hash_change

RULE @old_issue_continuity:
  MUST: ["逐项记录上次处置、本次证据和复核结果",
         "当前证据推翻原关闭结论时保留历史并重新打开，而非沿用 applied 标记"]
```

### 核实与决策

```formal
RULE @minimal_revision:
  MUST: ["问题成立但建议会改变已确认目标、引入回归或扩大范围时，采用满足原目标的最小合理修订",
         "在报告里记录偏离建议的理由"]
  MUST_NOT: treat_rejected_suggestion_as_solved_problem

RULE @adjudication:
  MUST: ["PRD 报告里的产品增强机会是提案，不自动变成新增承诺；仅在既有授权明确覆盖时采纳",
         "需要新的范围、产品语义、权限/数据、外部契约、关键 NFR 或重要架构取舍时，使用实际可用的交互问答工具（如 AskUserQuestion、request_user_input_async）：问题包含原要求、报告意见、已核实约束、2–3 个可行选项及后果/推荐理由，允许自由回答；不具备工具时直接提问并等待"]

RULE @decision_integrity:
  MUST: ["既有明确决定仍有效时沿用，不重复求批",
         "用户确认的决策同时落实到原文的目标/约束/验收和报告处置节"]
  MUST_NOT: treat_default_empty_answer_or_timeout_as_confirmation
  # 未回答的选择保留待决，继续不依赖它的修订。
```

## 3. 按文档类型实施修订

```formal
RULE @revision_ordering:
  MUST: ["按事实前提 → 决策/契约 → 关联映射 → 验收/状态的顺序，合并同根因修改并保留全部来源 ID",
         "改前重读受影响内容；并发修改冲突时仅暂停冲突部分",
         "沿用结构、语言和编号，不重写无关章节"]
```

### PRD

```formal
RULE @prd_discipline:
  MUST: ["保持 what / why / 怎样算完成的职责：把修订落实到角色、触发条件、用户行为、可观察结果、成功/失败状态、范围与验收",
         "沿「目标 → 场景 → FR/NFR → 验收 → UAT → 分期/终验」同步受影响位置",
         "保留稳定需求 ID；新增/拆合确有必要时保留来源映射",
         "所有数字、阈值和外部约束有来源",
         "需求正文保留已确认的完整能力；延期通过分期表达，不能悄悄砍小需求",
         "调整期次时同步每条需求的分配和终验声明；跨期局部完成不能写成整条验收完成",
         "UI/体验、AI 能力等意见只落实本次已确认的产品要求与判据；没有页面或实测证据时不声称视觉、易用性或模型效果已经验证"]
  MUST_NOT: [put_technical_design_table_schemas_test_scripts_or_tc_into_prd,
             fabricate_spms_keys_or_unsourced_commitments]

RULE @prd_record_separation:
  MUST: "新的产品决策以最终结论、必要理由、未选方案和受影响需求留在原文；逐条评审处置、改稿过程和 F-ID 放报告，不污染产品正文"
```

### 开发计划

```formal
RULE @plan_code_facts:
  MUST: ["基于当前代码事实修正前提、复用点、契约与设计落点；沿「PRD 需求 → 设计 → 任务/里程碑 → 验证」同步",
         "缺复用能力时规划真实增量，不能把未来新增路径写成已有实现",
         "按受影响范围修正接口/失败语义、迁移/回滚、前置成果、依赖和最终集成验收",
         "可并行必须有稳定契约、已满足前置及共享资源安排，不把文件不冲突等同于无依赖"]

RULE @plan_records_integrity:
  MUST: ["保留计划适用的实施协议、记录链接和有效历史证据；历史模板不机械升级",
         "已实施计划的设计/验收发生变化时，区分「历史交付事实」与「修订后的目标及待验证增量」，在相关任务/风险中显式标明后续工作",
         "变更会使恢复快照误导续做时，明确记录受影响范围和下一步；无法确定新的实施安排则保留阻塞，不声称计划已就绪",
         "里程碑确需重排时保留已使用的稳定 ID 和记录归属，同步总数、未开始任务及依赖；超出本次修订范围的实施重规划单列待办"]
  MUST_NOT: [fill_completion_records_for_new_work,
             increase_completed_counts_or_remark_old_tests_as_passed_by_this_revision,
             reset_existing_implementation_progress,
             rewrite_old_completion_records_to_fake_new_standards,
             rebind_completion_records_to_different_outcomes]

RULE @plan_readiness:
  MUST: ["原文只写最终设计与理由、来源、约束和验收；评审采纳/拒绝经过留在报告",
         "Ready / Proposed / Blocked 按修订后真实前置重判，不直接照抄评审建议"]
  MUST_NOT: treat_document_revision_as_product_implementation_done
```

## 4. 复核、回写与交付

```formal
RULE @post_revision_recheck:
  MUST: ["按每个问题的复核条件回读修改处及关联位置：原问题是否消除、是否产生新矛盾、需求/任务/验收是否断链",
         "搜索被修正的旧事实或口径，仅同步授权目标中的重复错误，其他文档记录受影响位置",
         "PRD 检查需求可断言、关键场景预期状态、分期和终验一致；计划检查事实、路径、契约、依赖、验证与进度一致"]

RULE @document_checks:
  MUST: ["目标采用 dev-plan 协议且工具可用时，用其实际路径运行 check_paths.sh <计划> <仓库根> 和 check_progress.sh <计划> <仓库根>；先检查脚本适用性，人工区分计划新增路径与错误复用",
         "工具缺失或模板不适用时人工核对并说明",
         "只运行证明文档修订所需的检查，不为改稿启动整套实施或业务验收",
         "未来验证写为计划要求；已有证据、静态推演和本次实测分开"]

RULE @disposition_writeback:
  MUST: ["按处置合同在报告追加或更新独立处置节，保留原结论，写明修后基线、逐项状态、验证结果、当前就绪判断及未完项",
         "这是本次修订复核，不冒称重跑了完整独立评审",
         "原文零改动但有新核实结论时仍记录逐条依据；已有相同且有效的处置则不重写",
         "缺关键复核标「已修订待核实」，不能标关闭"]

RULE @writeback_verification:
  MUST: ["回读已写入原文和报告，核对链接、状态、统计及最终 diff，确认没有无关变更和覆盖用户修改",
         "写入失败分别说明哪份文件未完成"]

RULE @delivery_content:
  MUST: ["默认交付已更新的目标文档及原评审报告中可区分的「修订处置与复核」节；用户只要建议、patch 或另存时遵循其要求",
         "最终回复给修订文档与处置报告链接、主要改动、未采纳/待决项和实际检查结果，再给一行明确判定和可直接复制的下一步命令（按当前客户端的技能调用写法，如 Codex `$dev-plan`、Claude Code `/dev-plan`），路径填实际文件",
         "部分问题关闭不等于整份文档通过评审，更不等于实现或上线完成"]

RULE @readiness_verdict:
  MUST: ["开发计划处置后按真实前置判 Blocked 或 Ready：评审已处理完毕，原先只等这次评审的 Proposed 也要改判",
         "唯一例外：仍在等用户指定的其他非阻塞评审时保留 Proposed 并写明等待项",
         "用户要求再评一轮时，下一步为 review-dev-plan <计划路径> 复审"]
  MUST_NOT: start_next_step_by_itself
  # 只给出建议命令，不自行启动下一步。
```

| 文档 | 判定 | 下一步 |
| --- | --- | --- |
| PRD | 可交接 | `dev-plan <PRD 路径> P1`：按 PRD 分期逐期出计划，列出全部期次及各期用户价值；未分期时省略期次 |
| PRD | 仍需修订 / 待补证或决策 | 列出剩余项及所需决定或证据；补齐后再次 `apply-doc-review <报告路径>`，需要独立复核时 `review-prd <PRD 路径>` |
| 开发计划 | Ready | `/goal 按 <计划路径> 开发` |
| 开发计划 | Blocked | `resolve-blocked <计划路径>`：逐项解除剩余阻塞 |

调用示例：

```text
使用 $apply-doc-review docs/PRD-login.review.md
使用 $apply-doc-review goal/LOGIN.review.md，仅处理 F-01 和 F-03。
使用 $apply-doc-review，根据 docs/reviews/login.review.md 修订 docs/PRD-login.md。
使用 $apply-doc-review docs/PRD-login.review.md，仅复核已有修订效果，不改原文。
使用 $apply-doc-review goal/LOGIN.review.r2.md，继续处理复审确认仍存在的问题。
```

## 项目状态回写

若仓库已登记状态台账，或存在仓根 `.xgent-ai/sdlc/protocol.md`（兼容已有 `docs/project-status/protocol.md`；两套并存且未登记切换时先核对，不能双写），交付原产物后读取台账的回写规则（默认目录读上述协议），按仓内协议更新受影响对象的当前状态与来源链接，必要时维护索引；v4 日常直接更新当前页，事件留存条件或旧版事件要求以仓内协议为准；未启用时不自动建表，不依赖另装技能。只读/仅聊天/不保存要求优先，登记失败须说明，不影响如实交付已完成的原产物。

接续原评审对象的处置维度，按评审 ID 与问题 ID 记录修订/复核及修后版本；同步确实改变的文档事实。不伪造新一轮独立评审，也不把部分关闭记成全部通过。
