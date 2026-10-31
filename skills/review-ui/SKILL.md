---
name: review-ui
argument-hint: "[开发计划] [页面URL/路由] [设计稿/目标图] [上轮报告] [复审/重建基线] [单元/范围] [门槛分] [报告路径]"
description: 用于界面实现后的 UI/UX 验收或修复复审；不用于代码 diff 评审、直接修复或从零设计。只认运行中界面的实拍截图与实测，不凭代码推断效果；由独立评审打分列差距，判定是否通过。
disable-model-invocation: true
---

# review-ui · UI/UX 验收评审

按以下 ODC 声明完成 UI/UX 验收；列表逐项适用。引用路径相对本文件解析，按阶段读取：

- 定位循环、报告路径、计分、停滞和判定时读 [UI 验收报告合同](references/report-contract.md)。
- 定位或建立目标包、计算指纹及采集真实界面时读 [目标包与采集](references/target-and-capture.md)。
- 派发、复核、合并及确认评审时读 [独立评审](references/judge.md)。
- 探测或续接 managed 循环时读 [Phoenix 评审协议](references/phoenix-review.md)。

```formal
INTENT:
  GOAL: assess_implemented_ui_against_targets
  SCOPE: one_acceptance_round_per_invocation
  SUCCESS: [actual_effects_grounded_verdict,
            complete_actionable_gap_list_enabling_clear_score_improvement_after_repairs,
            round_artifacts_saved_and_reread]
  NON_GOALS: [code_defect_review, ui_repairs, new_visual_direction,
              interaction_pattern_selection, design_md_maintenance]

RULE @round_continuity:
  MUST: [continue_same_rules_for_manual_invocations_and_goal_driven_rounds,
         resume_from_persisted_target_package_captures_raw_judgments_reports_and_repair_records,
         review_ui_and_apply_ui_review_continue_until_pass_stagnation_for_user_decision_or_explicit_waiver]
  MUST_NOT: main_agent_scores_own_ui_repairs

RULE @actual_effects_only:
  MUST: |
    分数、差距的开立与关闭、验收判定，只依据本轮真实运行产品的实拍截图、
    实际执行过的操作和页面实测结果。代码、diff、计划、设计意图及代码推断
    只用于找入口或定位问题。拿不到真实效果的单元不打分，判「需补证」并说明怎么补。
  MUST_NOT: imagined_effects_as_evidence

RULE @review_boundary:
  MUST: [product_code_plan_and_historical_reports_read_only,
         discover_target_startup_accounts_design_rules_and_capture_tools_at_runtime,
         follow_user_language]
  MUST_NOT: [ui_repairs, commits, plan_progress_updates]

RULE @applicability:
  MUST: |
    用于可运行界面的验收：计划实施并通过代码评审后可选的 UI 验收、修复复审，
    或用户给出页面与目标图／规范的单次验收。
    代码缺陷用 review-code；修复差距用 apply-ui-review；视觉方向用 design-reference；
    交互方案选型用 ui-pattern-research；维护 DESIGN.md 用 design-md。

WORKFLOW @review_ui:
  DO: [read_applicable_rules_and_goal_sources,
       resolve_prefix_round_and_existing_loop_history,
       prepare_artifact_ignore_rules_before_services_version_record_or_review_begin,
       probe_capabilities_and_select_permitted_loop_mode,
       locate_or_create_target_package_and_compute_fingerprint,
       begin_or_resume_managed_round_when_matching_or_use_manual_round,
       capture_current_real_product_matrix,
       conduct_independent_judgment_using_allowed_materials,
       save_raw_judgments_and_merged_judge_record,
       run_page_checks_then_read_full_history_repairs_and_threshold,
       adjudicate_with_required_confirmation_and_contract_rules,
       save_or_finalize_and_reread_round_state_report_and_links,
       stop_owned_temporary_services_and_check_artifact_ignores,
       write_registered_project_status_after_primary_artifact_delivery_when_allowed,
       deliver_verdict_scores_gaps_captures_and_next_step_commands]
  OUTPUT: ui_acceptance_round_report
```

## 本地产物与 Git 忽略

```formal
RULE @artifact_locations:
  MUST: |
    人工和 managed 循环都在确定前缀后、启动临时服务／记录被测版本／review begin 前，
    检查目标仓忽略规则。截图放 <前缀>.ui-captures/r<n>/，控制台、失败请求、
    临时服务及 review 命令日志集中放本轮 logs/ 子目录。
  MUST_NOT: [move_delete_or_overwrite_historical_artifacts]

RULE @artifact_ignore_rules:
  UNLESS: user_requires_read_only_no_persistence_or_no_configuration_changes_or_target_is_not_git_repository
  MUST: [read_existing_target_gitignore_or_create_when_absent,
         minimally_append_only_uncovered_cycle_capture_and_log_paths,
         reuse_existing_covering_rules,
         add_other_current_round_log_paths_only_when_actually_used,
         default_ignore_captures_and_runtime_logs_unless_repository_requires_evidence_commits]
  MUST_NOT: |
    全仓 *.png、*.log 或忽略整个 .ui-captures/；
    忽略目标图、目标包、报告、修复记录、capture.md、judge.md 或 managed 状态文件。
  # 前缀 docs/reviews/orders 的规则示例：
  # /docs/reviews/orders.ui-captures/**/*.png
  # /docs/reviews/orders.ui-captures/**/*.log
  # /docs/reviews/orders.ui-captures/**/logs/

RULE @artifact_ignore_verification:
  WHEN: target_is_git_repository
  MUST: |
    用 git check-ignore -v -- <实际产物路径> 验证命中；
    结束前用 git status --short --untracked-files=all 核对新增截图和日志。
    已跟踪的产物只说明路径，不自行移出索引。

RULE @artifact_ignore_skip:
  WHEN: user_requires_read_only_no_persistence_or_no_configuration_changes_or_target_is_not_git_repository
  MUST: disclose_why_gitignore_was_not_written
```

## 循环模式与能力探测

```formal
RULE @loop_discovery:
  MUST: |
    先确定前缀，检查 <前缀>.ui-review.json、已有报告和安装回执。
    已有纯 Markdown 历史但无 managed 索引时继续人工流程，只有显式 review import 才接管。
    全新无索引、无旧历史且用户已要求 UI 验收时，先探测真实 engine 与 review verbs；
    匹配后可 review begin，缺 CLI 或能力不匹配时走人工流程。
  MUST_NOT: [missing_cli_blocks_manual_source_reading_real_capture_or_persistence,
             claim_managed_takeover_without_machine_index,
             unknown_historical_attempt_or_refactor_count_as_zero]

RULE @existing_managed_identity:
  WHEN: managed_index_exists
  MUST: verify_pinned_engine_identity_and_actual_review_protocol_using_phoenix_reference

RULE @managed_capability_failure:
  WHEN: existing_managed_engine_missing_or_mismatched_or_future_schema_or_corrupt_index
  MUST: [read_only_history, allow_unregistered_material_collection,
         explain_how_to_restore_matching_engine_then_resume_original_round_and_revision]
  MUST_NOT: [score, adjudicate, write_dispositions_counts_or_generated_report_sections,
             create_new_cycle_to_bypass_existing_state]

RULE @managed_operations:
  WHEN: matching_managed_engine_and_protocol_available
  MUST: |
    review begin 分配或续接轮次，冻结目标与矩阵；之后依次登记采集、
    读取白名单 packet、登记评审原文、裁定与回读。
    JSON 依据实际 review --help 与 review schema --json 的 typed examples 构造；
    身份、hash、revision 来自真实操作和材料。
  MUST_NOT: [guess_json_fields, typed_examples_as_evidence, implicit_import_of_legacy_markdown]
```

## 1. 锚定对象与目标包

```formal
RULE @goal_sources:
  MUST: [read_applicable_repository_rules,
         read_project_design_md_or_theme_rules_and_linked_designs]

RULE @plan_goal_sources:
  WHEN: development_plan_provided
  MUST: [read_frontend_interaction_real_environment_ui_verification_and_milestone_acceptance_sections,
         follow_requirement_source_to_prd_ui_requirements_and_uat,
         read_designs_linked_by_plan_or_prd]

RULE @single_acceptance:
  WHEN: only_url_or_route_and_target_image_provided
  MUST: single_acceptance_review

RULE @round_identity:
  MUST: |
    按报告合同命名。人工循环依据已有最大轮次确定本轮；managed 循环读索引，
    未结束轮次续接原 round/revision，不自行加一。已有本循环报告即为复审。
    每个单元的上一轮，是最近一次覆盖它且有评审结论的轮次；
    覆盖了但没有评审结论的轮次（如全部缺证封存）不算。

RULE @target_baseline:
  MUST: [locate_or_create_target_package_using_target_and_capture_reference,
         compute_target_fingerprint,
         compare_with_previous_capture_md_fingerprint,
         restart_historical_best_stagnation_and_refactor_tracking_on_changed_fingerprint]
  # 期望或目标图改变即重建基线；只改「采集」节不影响指纹。

RULE @target_freeze:
  MUST: [after_target_established_change_only_capture_section_and_record_change,
         target_image_and_spec_conflicts_as_pending_user_decisions,
         unit_without_target_image_or_spec_as_needs_evidence,
         continue_review_of_independent_units]
  MUST_NOT: rewrite_established_expectations_during_review
```

## 2. 采集本轮截图

```formal
RULE @tested_environment:
  MUST: [start_or_connect_environment_using_target_package,
         record_head_and_dirty_fingerprint_or_build_id_and_access_address,
         confirm_current_code_is_running_instead_of_stale_build,
         run_owned_temporary_services_in_background_and_record_stop_method,
         stop_owned_temporary_services_before_finish,
         use_only_test_accounts_and_seed_data]
  MUST_NOT: [writes_to_real_business_data, destructive_operations]

RULE @capture_coverage:
  MUST: [capture_all_target_unit_state_viewport_theme_cells_each_round_unless_user_limits_scope,
         recapture_under_same_conditions,
         honor_user_limited_scope_and_mark_other_cells_unreviewed]
  MUST_NOT: cherry_pick_flattering_angles_or_data

RULE @capture_authenticity:
  MUST: [current_round_real_product_capture_for_each_image,
         follow_target_and_capture_authenticity_contract,
         treat_nonconforming_images_as_missing]
  MUST_NOT: [design_image_as_capture, old_image_as_current_capture,
             review_only_mock_page_as_product_capture]

RULE @interaction_capture:
  MUST: [actually_execute_interaction_steps,
         capture_keyframes_after_each_action_including_keyboard_focus,
         record_only_executed_actions_and_observed_results,
         save_console_logs_and_failed_requests_during_capture_for_post_judgment_analysis]
  MUST_NOT: expected_or_code_inferred_results_as_operation_records

RULE @unreachable_state:
  WHEN: required_state_unreachable
  MUST: [first_seek_existing_seed_mock_or_route_interception_methods,
         record_reason_for_still_missing_cell]
  MUST_NOT: similar_state_substituted_for_required_state

RULE @capture_record:
  MUST: [save_tested_version_target_fingerprint_conditions_cell_mapping_provenance_and_missing_cells,
         manual_round_writes_capture_md,
         managed_round_uses_review_capture_add_and_rereads_generated_manifest,
         supplementary_capture_appends_attempt]
  MUST_NOT: overwrite_original_images_or_old_records
```

## 3. 独立评审

```formal
RULE @independent_judge:
  MUST: [fresh_context_read_only_nondelegating_subagent_each_round_unless_fallback_required,
         dispatch_and_recheck_using_judge_reference,
         give_only_target_package_current_captures_capture_md_and_interaction_records,
         give_each_unit_previous_captures_and_judge_md_containing_only_judge_scores_and_gaps,
         judge_opens_only_listed_materials]
  MUST_NOT: |
    向评审提供代码、diff、提交说明、修复记录、修复者自述、完整报告、
    机器检查结果或门槛分。

RULE @managed_judge_packet:
  WHEN: managed_round
  MUST: [dispatch_review_judge_pack_allowlisted_materials,
         bind_and_verify_same_material_batch_with_packet_hash]
  MUST_NOT: [follow_material_links_to_excluded_files,
             claim_allowlist_is_host_filesystem_permission_sandbox]

RULE @judge_grouping_and_supplement:
  MUST: [group_many_cells_by_unit_for_parallel_dispatch,
         use_temporary_ids_for_new_gaps_then_main_agent_assigns_unified_ids,
         recapture_cells_reported_unreadable_then_fresh_judge_reassesses_affected_units,
         retain_lower_total_score_for_reassessed_unit]

RULE @single_context_fallback:
  WHEN: subagent_unavailable_or_judge_cannot_open_images
  MUST: |
    当前对话未参与此界面实现或修复时，主 agent 可以评审：评审前只读目标包、
    本轮和上一轮截图、capture.md、judge.md，先保存 judge.md 再做裁定阶段工作，
    报告注明「单上下文评审」。
    当前对话参与过实现或修复时，不在本对话打分，给出新对话运行本技能的命令。

RULE @judge_integrity:
  MUST: [append_only_fact_check_note_under_disputed_gap,
         leave_dispute_for_repairer_verification_and_next_round_judge_recheck]
  MUST_NOT: [main_agent_changes_scores_deletes_gaps_or_changes_gate_items,
             replace_judge_or_rejudge_due_to_disagreement]

RULE @confirmation_judge:
  WHEN: would_pass_and_lowest_unit_score_strictly_below_threshold_plus_0_5
  MUST: [dispatch_another_fresh_judge_with_same_material_batch,
         keep_lower_total_judge_entire_four_dimension_score_group_and_reasons_per_unit,
         union_gaps_from_all_judges]
  MUST_NOT: [require_confirmation_at_exactly_threshold_plus_0_5,
             splice_dimension_scores_from_different_judges]
```

## 4. 裁定与落盘

```formal
RULE @judgment_persistence:
  MUST: [save_each_judge_raw_output_and_merged_judge_md_before_adjudication_stage,
         managed_round_registers_raw_output_and_faithful_structured_results_via_review_judge_add_and_rereads]

RULE @post_judgment_checks:
  MUST: [run_machine_checks_only_after_judgments_saved,
         analyze_saved_logs_measure_overflow_in_page_inject_accessibility_scan_and_use_real_keyboard,
         read_previous_full_report_and_each_round_repair_records_after_judgment]
  MUST_NOT: code_or_stylesheet_inference_as_machine_check_result
  # 推断只提示去哪里补拍或实测。
  repair_record_location: "计划记录目录中含「UI 验收修复」节的记录；无计划时 <前缀>.ui-review.applied.md"

RULE @threshold:
  MUST: [current_user_threshold_first_then_previous_report_threshold_then_default_8_of_10,
         determine_after_judgment_without_disclosing_to_judge]

RULE @adjudication:
  MUST: [main_agent_rechecks_previous_machine_and_main_agent_verified_gaps,
         compute_round_score_gate_items_pending_decisions_and_stagnation_using_report_contract,
         derive_acceptance_verdict_using_report_contract]

RULE @round_report:
  MUST: [manual_round_writes_report_using_contract,
         managed_round_uses_review_finalize_with_current_revision,
         reread_state_report_and_links_for_verdict_scores_and_id_mappings]
  MUST_NOT: [hand_edit_managed_generated_sections,
             operation_exit_zero_as_acceptance_pass]
  ON_UNMET: [state_report_not_saved, deliver_verified_content_in_chat]

RULE @confirmation_required:
  WHEN: review_finalize_returns_confirmation_required
  MUST: [keep_round_draft, complete_confirmation_judgment_before_finalizing_again]

OUTPUT:
  FORMAT: markdown
  STYLE: follow_user_language
  SECTIONS: [acceptance_verdict, score_trajectory, lowest_scoring_unit_and_key_gaps,
             latest_capture_locations, copyable_next_step_commands]
  verdict_source: references/report-contract.md

REVIEW:
  CHECK: [@actual_effects_only, @managed_capability_failure, @target_freeze,
          @capture_coverage, @independent_judge, @judge_integrity,
          @confirmation_judge, @round_report]
  ALSO: [round_continuity, evidence_traceability, id_mapping_consistency,
         artifact_ignore_coverage]
  EXPOSE: on_issue
```

## 5. 验收判定与下一步

```formal
RULE @next_step_commands:
  MUST: [final_reply_starts_with_contract_acceptance_verdict,
         commands_use_current_client_skill_syntax_and_actual_paths]
  MUST_NOT: automatically_start_repairs_code_recheck_or_prd_gap_review

RULE @first_target_confirmation:
  WHEN: first_round_just_created_target_package
  MUST: [list_expectation_sources_and_assumptions_for_user_confirmation,
         review_next_round_as_new_baseline_after_user_changes_expectations]

RULE @needs_changes_next:
  WHEN: "验收判定为「需要修改」"
  MUST: |
    给出 apply-ui-review <本报告路径>，修复后用本技能复审。
    有「停滞预警」时写明全局重构或需重构的单元；
    有「待裁定」时列出事项请用户决定，修复方先处理其余差距。

RULE @passed_next:
  WHEN: "验收判定为「通过验收」或「用户豁免通过」"
  MUST: |
    给最新截图并询问是否继续打磨；要打磨则 apply-ui-review <本报告路径> 后复审。
    不打磨时，如本轮循环 UI 修复改了代码（登记为「UI修复」的提交或未提交改动），
    先 review-code <计划路径> 复审这些改动；之后计划来自 PRD 时可选
    review-prd-dev-gaps <计划路径>。

RULE @needs_decision_next:
  WHEN: "验收判定为「需用户裁定」"
  MUST: |
    列出停滞或待裁定事项、已尝试的做法、目标与当前截图对照，请用户选择：
    1. 接受整体现状，记为豁免通过；
    2. 接受某处偏差或修改期望，用户确认后改目标包，下一轮重建基线；
    3. 提供素材或方向，然后执行 apply-ui-review <本报告路径>。

RULE @needs_evidence_next:
  WHEN: "验收判定为「需补证」"
  MUST: |
    列出缺少的环境、账号、数据、工具或目标及获取方式；
    补齐后 review-ui <计划路径或原范围>。
    已评单元的差距可以先 apply-ui-review <本报告路径>。

RULE @explicit_waiver:
  WHEN: user_explicitly_accepts_current_overall_ui
  MUST: append_contract_waiver_record_and_reread
  MUST_NOT: waiver_described_as_resolved
```

调用示例：

```text
使用 $review-ui goal/LOGIN.md
使用 $review-ui goal/LOGIN.md，设计稿在 design/login/，门槛 8.5 分。
使用 $review-ui 验收 http://localhost:5173/orders，目标图 docs/design/orders.png。
使用 $review-ui goal/LOGIN.md，只复查 U2。
```

## 项目状态回写

```formal
RULE @project_status:
  WHEN: repository_has_registered_ledger_or_existing_sdlc_protocol
  MUST: |
    交付原产物后读取仓内回写规则；默认协议为仓根 .xgent-ai/sdlc/protocol.md，
    兼容已有 docs/project-status/protocol.md。两套并存且未登记切换时先核对，不双写。
    按协议更新受影响对象当前状态与来源链接，必要时维护索引；v4 日常直接更新当前页，
    事件留存条件或旧版事件要求以仓内协议为准。
    只读／仅聊天／不保存及 managed 能力异常的只读边界优先；
    登记失败须说明，不影响如实交付已完成的原产物。
  MUST_NOT: [create_ledger_when_not_enabled, require_separately_installed_skill]

RULE @review_status_record:
  WHEN: project_status_writeback_allowed
  MUST: [independent_id_for_each_actual_review,
         record_tested_version_target_fingerprint_scope_verdict_score_and_report_path,
         link_recheck_to_previous_round,
         "协议 review_type 没有 UI 类型时用 other，摘要注明「UI 验收评审」"]
  MUST_NOT: register_ui_acceptance_pass_as_merged_released_or_product_final_acceptance
```
