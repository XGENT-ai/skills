---
name: resolve-blocked
argument-hint: "<计划路径/链接> [评审报告] [阻塞项/范围] [决策记录路径]"
description: 用于解除开发计划中 Blocked/阻塞/待拍板事项；不用于从零写计划、整体评审或按计划实施。查证解阻，需用户处理时用工具阻断式问答，外部依赖用 CRD 交接，存决策并回改计划。
when_to_use: "用户会说「把计划里的阻塞解掉」「这个依赖一直没给，怎么办」「这几项等我拍板」时。"
---

# resolve-blocked · 逐项解除开发计划阻塞

```formal
INTENT:
  GOAL: restore_development_plan_readiness
  SUCCESS: [all_in_scope_blockers_accounted_for, evidence_based_resolution,
            user_actions_explicitly_presented_through_interaction_tool,
            decisions_saved_and_plan_revised, readiness_matches_actual_prerequisites]
  NON_GOALS: [implementation, whole_plan_review, nonblocking_review_fixes,
              external_system_writes, automatic_crd_sending]

RULE @scope_and_authorization:
  MUST: [follow_user_language, resolve_references_relative_to_this_skill,
         discover_target_repo_rules_stack_and_tools_at_runtime,
         honor_chat_only_list_only_and_no_plan_edit_requests]

RULE @local_authorization:
  MUST: |
    调用即授权写入计划同目录的决策记录、修订计划中与阻塞相关的内容、创建或续写本地 CRD；
    记录路径按 references/decisions-contract.md。代码、PRD、评审报告和实施记录默认只读。
  MUST_NOT: [implement_features, run_migrations_deployments_or_side_effecting_commands,
             write_external_systems, modify_dependency_projects, auto_send_crd,
             add_sending_approval_steps, treat_saved_crd_as_resolved_dependency]

RULE @routing:
  MUST: |
    从零写或重写计划用 dev-plan，整体评审用 review-dev-plan，落实全部评审意见用 apply-doc-review；
    本技能只处理阻塞实施的事项，非阻塞意见留给 apply-doc-review。
    可用时读取 dev-plan/SKILL.md 的计划状态与红线部分及 references/plan-skeleton.md；
    复杂架构取舍按需读其 references/architecture-quality.md。缺失时沿用计划自身结构，
    不因用户模板或历史章节名不同而强行迁移。

RULE @evidence_and_collaboration:
  MUST: [treat_plan_assertions_as_claims_to_verify,
         trace_verified_and_user_confirmed_claims_to_source_code_or_user_words,
         describe_agent_actions_dependency_projects_and_deliverables,
         specify_missing_environment_access_evidence_verification_entry_and_pass_criteria]
  MUST_NOT: [invent_confirmation, invent_verification, assign_human_job_owner_fields,
             substitute_find_owner_or_handoff_to_qa_for_concrete_actions]

WORKFLOW @resolve_plan:
  DO: [anchor_inputs_and_baseline, resume_existing_decisions,
       collect_classify_and_show_blockers, @item_resolution,
       save_decision_record, revise_plan, verify_and_deliver, update_existing_project_ledger]

RULE @input_baseline:
  MUST: [locate_requested_plan, ask_if_no_unique_plan,
         read_applicable_agents_and_claude_rules_full_plan_linked_prd_and_key_decisions,
         finish_truncated_passages_before_quoting,
         record_plan_path_content_hash_code_commit_dirty_state_and_date,
         read_decisions_contract_and_existing_decision_record]
  MUST_NOT: [choose_neighboring_plan_without_basis, search_review_reports_by_default]

RULE @review_report_source:
  MUST: |
    计划是阻塞事实源。用户指定报告时读取；否则仅在来由/证据缺失、顶部 Blocked 无对应项、
    或需核对计划引用的评审结论时回读。用计划引用或同目录中「评审对象」指向本计划的报告，
    优先读处置节仍待决项；多候选不能唯一确定时询问，不按时间猜选。
    已处置意见以当前计划为准，不重新当作阻塞。

RULE @resume:
  WHEN: decision_record_exists
  MUST: [reuse_same_record_and_existing_b_ids, reuse_applicable_confirmed_decisions,
         compare_current_plan_hash_with_previous_post_revision_hash,
         inspect_changed_passages_and_preservation_of_resolved_plan_conclusions,
         reopen_deleted_changed_or_overturned_conclusions_with_evidence,
         recheck_remaining_blockers_against_latest_crd_delivery_accounts_quotas_access_spike_measurement_and_user_input,
         assign_unused_b_ids_to_new_blockers, label_new_reopened_and_carried_items]

RULE @resume_prerequisites:
  WHEN: decision_record_exists
  MUST: |
    无可交接 CRD 的外部依赖补齐请求，旧记录的人岗分工不算已有请求。
    用户已明确要等的事项不重复催问；上轮未答或留待之后决定的决策项，
    本轮重新提供上下文并通过交互工具询问。

RULE @unchanged_rerun:
  WHEN: plan_prerequisites_external_facts_and_user_response_state_unchanged_and_required_crd_and_next_actions_complete
  MUST: [preserve_plan_and_record, deliver_full_remaining_blockers_and_user_actions]
  MUST_NOT: [append_empty_round, duplicate_sdlc_event, repeat_ineffective_verification,
             repeat_explicitly_acknowledged_waiting_requests]
  # 文档无变化不豁免 @user_action_gate；本轮产生新答复或交接状态即不再属于无变化重跑。

RULE @blocker_inventory:
  MUST: |
    收集顶部 Blocked 理由、§11「是否阻塞=是」、需求账本中阻塞的 NFR-*/A-* 与影响关键决策的未知数字、
    [未核实·阻塞]、契约表「未核实阻塞」、决策表/ADR 的 Blocked、实施进度的当前阻塞及阻塞里程碑；
    已读评审报告时补充其处置节仍待决且计划未体现的阻塞。其他模板取等价内容。
    按根因去重并保留全部来源；未标注但涉及范围、安全、数据、外部契约或明确要求的 NFR 的未决项，
    先列为「疑似阻塞」，用户确认后才纳入。从 B-01 起编号，续做不占用已有 ID。

RULE @inventory_presentation:
  MUST: [classify_by_resolution_method, revise_classification_after_investigation,
         show_inventory_before_item_processing]
  # 总览：B-ID、事项、来源、阻塞的里程碑/任务、类型、计划处理方式。

RULE @resolution_method:
  MUST: |
    事实可查：代码、配置、依赖、仓库或官方文档有答案时自行查证，不问用户。
    需要决策：产品语义、范围/分期、权限与数据归属、外部契约、关键 NFR、难回退架构与成本，查证后附推荐确认。
    本方验证/访问前置：只读边界内自行验证；用户能补齐的实测/spike/环境/账号/访问前置当轮通过工具获取；
    其余写明缺口、agent 执行入口与判据，不越界执行。未实测不自动意味着依赖项目要改动。
    外部依赖：被依赖项目需改能力/配置，或补契约、环境、交付证据时，实际使用 crd 创建/续写本地请求。
    用户需提供信息、解决外部前置或协调交接时，执行 @user_action_gate。

RULE @item_context:
  WHEN: preparing_item_resolution
  MUST: [process_upstream_dependencies_first,
         identify_affected_requirements_milestones_tasks_and_latest_blocking_step,
         locate_plan_claim_and_missing_evidence_or_decision,
         cite_checked_behavior_with_path_and_line_and_expose_refuted_premises,
         gather_constraints_from_user_prd_c_ids_and_repo_rules]

RULE @item_resolution_criteria:
  WHEN: preparing_item_resolution
  MUST: |
    需决策时给 2–3 个真正可行且实质不同的方案：做法、交付/风险/工作量/可回退性后果、计划改动位置。
    推荐扎根于证据与约束；证据不足则说明缺口，不编推荐理由。
    事实充分即可据证据解除；查证受阻时按归属转为本方验证/访问前置或外部依赖。

RULE @external_crd:
  WHEN: dependency_project_gap_confirmed
  MUST: [locate_and_read_available_crd_skill, actually_create_or_continue_local_crd,
         reuse_same_topic_crd_and_stable_r_q_resp_history,
         integrate_dependency_replies_as_requesting_party,
         prepare_handoff_material_before_requesting_user_handoff]
  MUST_NOT: [merely_recommend_later_crd_invocation, switch_to_crd_respond_for_incoming_reply,
             duplicate_already_covered_requests, prescribe_dependency_internal_implementation,
             convert_dependency_implementation_into_local_tasks,
             invent_replies_schedules_or_commitments]

RULE @external_handoff:
  WHEN: dependency_project_gap_confirmed
  MUST: |
    交接包含本方/接收项目、受阻计划任务、缺口证据、可观察行为/交付物、必要契约与验收；本方验证单列。
    只有接收项目无法确定且会影响请求对象时才问，不问人岗分配。
    已有用户决定沿用；新的范围、权限、数据、契约、关键 NFR 取舍通过工具确认，未确认部分标提议。
    计划阻塞处与决策记录链接真实 CRD 和条目，写明待答/待交付、解除证据和恢复入口；
    未实际发送写「文档已保存，未发送」，需要用户交接/协调即执行 @user_action_gate。
    找不到 crd 时查其他可用安装位置；仍缺失则交付上述 CR 要素并标「CRD 生成受阻」，不伪造调用或文件、不自行安装。
    回复或接受不等于前置满足；纯契约澄清可由充分答复解除，真实准入/联调必须回收交付及本方验证证据。
    本方须基于目标、职责和双方长期维护/演进成本形成明确技术立场；未发送或尚待对方接受，
    不把本方结论一律标草案。已有有效结论与必要确认足以定案时关闭对应契约问题，转向真实交付验证。
    往返仅重复原有理由、无新证据时，提炼最小剩余分歧与本方主张；权限内直接定论，
    需用户裁定或跨项目协调时执行 @user_action_gate，不再把一轮草案当作进展。

RULE @user_action_gate:
  WHEN: user_decision_information_or_action_required
  UNLESS: applicable_answer_or_handoff_acknowledgment_exists_or_user_explicitly_deferred_this_round
  MUST: [prepare_available_evidence_options_and_handoff_artifacts_first,
         invoke_available_ask_user_tool_under_current_mode_and_tool_constraints,
         wait_for_explicit_user_response, keep_answer_dependent_steps_blocked]
  MUST_NOT: [chat_only_request, final_checklist_as_tool_substitute,
             file_only_request, treat_tool_invocation_as_user_response,
             close_turn_as_handoff_complete_while_request_unanswered]

RULE @interaction_tools:
  WHEN: user_action_gate_requires_new_interaction
  MUST: |
    必须用阻断式 Ask User Tool 或等效交互：如 AskUserQuestion、当前模式允许的 request_user_input。
    这不仅适用于拍板，也适用于请用户解决外部依赖、协调其他项目、交接 CRD、补环境/访问或回传证据。
    仅有 request_user_input_async 等异步工具时，实际调用并保持请求待答；等待期间只做独立工作，
    独立工作做完仍等待明确答复，不能发完卡片就结束或让后续消息替代该请求。
    工具不可用、调用失败或受当前模式限制且无等效工具时，报告「用户交互受阻」与具体原因，
    暂停依赖该答复的工作；屏幕消息仅说明能力缺口，不算请求已送达、交接完成或用户已确认。

RULE @question_content:
  MUST: [ask_minimum_missing_information, reuse_existing_authorization_and_answers,
         make_each_tool_question_self_contained,
         include_b_ids_blocked_work_requested_decision_or_action_and_consequences,
         put_recommended_decision_first_with_reason_and_allow_free_text]
  MUST_NOT: [require_reading_earlier_chat_to_answer, request_secret_pasting,
             replace_concrete_action_with_follow_up_or_find_owner]

RULE @question_batching_and_handoff:
  MUST: |
    外部交接题写接收项目、真实文档链接/条目、用户要做的具体动作、需回传的答复/交付证据；
    能在工具内回答的直接问；需离线协调的请用户明确答复完成情况、已接手等待或暂缓，
    不把此交互变成申请 agent 自动发送的审批。
    独立项在工具上限内合并，优先上游或可解多项的问题；其余保留待处理。
    下游选项依赖前题时顺序询问，得到答复后重判选项。

RULE @answer_semantics:
  MUST: [only_explicit_answer_counts_as_confirmation,
         record_follow_recommendation_as_user_authorized_recommendation,
         clarify_ambiguous_free_text_once, honor_nonrecommended_choice,
         explain_fact_conflict_or_new_blocker_once_and_record_final_user_decision]
  MUST_NOT: [default_selection_as_confirmation, empty_answer_as_confirmation,
             timeout_as_confirmation, coordination_acknowledgment_as_dependency_resolution]

RULE @answer_followthrough:
  MUST: |
    用户明确「已接手/等待对方」可完成本轮交接确认，但解除条件未满足仍为 Blocked。
    拒绝或暂缓时保留未决，不在本轮重复催问；落实其他已确认项并完成独立查证。
    用户要求停止即停止并交接已完成部分。未答、暂缓、未及处理项均进入最终完整清单。

LOOP @item_resolution:
  TRIGGER: blocker_inventory_ready
  DO: [investigate_next_actionable_item, prepare_context_and_crd_when_needed,
       apply_user_action_gate_when_needed, process_received_answers,
       verify_resolution_conditions, update_decisions_and_plan, recheck_related_blockers]
  UNTIL: |
    本轮范围内全部解除，或剩余项仅为用户明确拒绝/暂缓、已确认交接后的真实外部等待、
    工具不可用等无法继续的缺口或超出授权边界；用户要求停止时立即退出。
    已发送但未答的工具请求本身不满足退出条件；不因分类、提问或生成 CRD 就结束。

RULE @decision_record:
  MUST: [follow_decisions_contract, single_main_agent_writer,
         append_current_round_and_item_conclusions_without_rewriting_history,
         reread_saved_record]
  # 对象、轮次、总览、逐项上下文/选项/推荐/原话或证据/计划落点、未解除阻塞、就绪结论。

RULE @plan_revision:
  MUST: [reread_affected_passages_before_editing, preserve_concurrent_user_changes,
         write_resolved_substantive_conclusions_into_plan,
         limit_unresolved_edits_to_conditions_collaboration_sources_and_next_actions,
         synchronize_premises_decisions_contracts_design_tasks_verification_then_status]
  MUST_NOT: [edit_unrelated_sections, write_b_ids_qa_history_or_skill_names_into_plan,
             insert_this_round_process_notes_into_plan, premark_new_work_complete,
             rewrite_existing_completion_history, treat_unaccepted_external_contract_as_fact]

RULE @plan_revision_locations:
  MUST: |
    需求账本：A-* 改已解除或转 C-*/R-*；未知 NFR 填确认值及「用户 YYYY-MM-DD 确认」或证据路径。
    事实/契约：[未核实·阻塞] 改 [已核实·足够] 或 [已核实·缺口]，附证据与最小增量。
    决策表/ADR：选择、理由、未选备选与重评条件，已确认决策为 Accepted。
    同步受影响接口、失败语义、迁移、§9 验证、里程碑任务及依赖。
    §11 已解除项删除或改不阻塞；其余保留解除条件、下一步、CRD 来源与最晚确认点。
    按事实更新实施进度当前阻塞和基线；按 dev-plan 标准重判顶部状态：
    范围、安全、数据、外部契约或明确要求的 NFR 仍有未决则 Blocked，前置全部解决才 Ready。

RULE @verification:
  MUST: [reread_plan_decisions_and_current_crd,
         trace_each_resolved_item_to_plan_conclusion, search_and_remove_refuted_claims,
         check_requirements_design_milestones_acceptance_links,
         align_top_status_risks_and_progress,
         check_real_cr_entries_or_explicit_generation_blockers_for_external_gaps,
         check_local_verification_actions_and_pass_criteria,
         exclude_job_owner_waiting_and_unauthorized_sending,
         reconcile_all_remaining_b_ids_counts_actions_and_conditions_with_final_list,
         verify_required_user_actions_were_tool_presented_and_answer_state_recorded,
         record_post_revision_plan_hash_and_readiness_in_current_round]

RULE @verification_execution:
  MUST: |
    采用 dev-plan 协议且脚本可用时，运行 check_paths.sh <计划> <仓库根> 与 check_progress.sh <计划> <仓库根>；
    不适用或缺失时人工核对并说明。收口前完成当前授权与工具内所有查证、答复处理及本地交接材料；
    仅缺信息、用户暂缓、真实外部等待或超出边界时如实交接，不因已提问或发现外部依赖就停。

OUTPUT:
  FORMAT: markdown
  SECTIONS: [status_and_counts, full_remaining_blocker_table, artifacts_and_next_actions]
  STYLE: [self_contained, no_dependency_on_collapsed_messages_or_file_opening]

RULE @delivery:
  MUST: |
    开头给真实 Ready/Blocked 变化、本轮已解除/新增或重开/仍阻塞数量，以及「需你处理 N 项」
    （按需用户动作的 B-ID 计数，无则明确 0）；仍阻塞说明在等什么。
    Blocked 时紧接完整清单，一项一行：B-ID | 阻塞事项与影响 | 当前缺口/需哪一方补齐 |
    你现在要做什么 | 解除判据与恢复入口。覆盖全部重开、未答、暂缓及未及处理项，不用「等 N 项」省略。
    共用动作标全部 B-ID；无变化重跑、仅聊天、部分交付同样适用。清单是汇总，不能替代 @user_action_gate。
    外部项写接收项目、已保存未发送/待答复/待交付等实际进展、待交付物、真实 CRD 链接与条目；
    未生成写原因和已知交接内容。未发送且需用户交接时写交给哪个项目、哪份材料、需回传什么。
    本方验证写环境/访问前置、执行入口与判据；哪一方仅区分本方 agent、用户和依赖项目，不设人岗负责人。
    未知分工写待确认问题；无需用户操作写「无需你操作，等待……」。
    再给计划、decisions、本轮 CRD 链接与关键已解除项；写入失败明确哪份未完成，不声称已保存。

RULE @next_actions:
  MUST: |
    给当前客户端可复制调用及真实路径（Codex $resolve-blocked，Claude Code /resolve-blocked）：
    Ready → /goal 按 <计划路径> 开发。
    Blocked → 先推进清单中的交接/补证/决定；需修订请求或整合答复给 crd <实际 CRD 路径>；
    明确哪些条件满足后再运行 resolve-blocked <计划路径> 回收证据和重判，不能只给重跑命令。
    全部在等待且无可执行动作时，说明条件满足前重跑无效；有独立工作则一并列出。
  MUST_NOT: start_implementation_from_suggested_next_command

RULE @project_ledger:
  WHEN: repository_has_registered_ledger_or_existing_sdlc_protocol
  MUST: |
    交付原产物后读仓内台账规则；默认 .xgent-ai/sdlc/protocol.md，兼容已有 docs/project-status/protocol.md。
    两套并存未登记切换时先核对，不双写。按协议更新受影响对象当前状态、来源与必要索引；
    v4 日常直接更新当前页，事件条件或旧版事件要求依仓内协议。
    登记修订后计划版本/就绪状态、决策记录及已解除/剩余阻塞；不把解阻记成实施完成。
    只读/仅聊天/不保存优先；登记失败说明，不影响如实交付原产物。
  MUST_NOT: [create_unrequested_ledger, require_extra_skill_installation]

REVIEW:
  CHECK: [@scope_and_authorization, @local_authorization, @user_action_gate,
          @interaction_tools, @answer_semantics, @answer_followthrough,
          @unchanged_rerun, @verification, @verification_execution, @delivery]
  EXPOSE: on_issue
```

决策记录路径、模板、结果状态与续写规则见 [references/decisions-contract.md](references/decisions-contract.md)，锚定输入时读取。验收设计见 [evals/scenarios.md](evals/scenarios.md)，仅在验证本技能行为时读取。

```text
$resolve-blocked goal/LOGIN.md
$resolve-blocked goal/LOGIN.md，参考 goal/LOGIN.review.md，只处理与 M2 相关的阻塞。
$resolve-blocked goal/LOGIN.md，B-03 的供应商答复已到：<答复要点>。
```
