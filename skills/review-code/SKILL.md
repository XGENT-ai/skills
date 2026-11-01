---
name: review-code
argument-hint: "[开发计划] [未提交/commit/A..B/基准分支] [上轮报告] [复审/完整评审] [问题ID/文件范围] [报告路径] [仅聊天]"
description: 用于评审代码 diff、提交、分支差异或开发计划登记的提交及修复复审；不用于单独评审计划、全仓普查或直接修复。核对旧问题与新增回归，落盘报告并判定是否需修改。
disable-model-invocation: true
---

# review-code · 代码变更评审

按以下 ODC 声明完成代码变更评审；列表逐项适用。引用路径相对本文件解析，按阶段读取：

- 确定模式与版本时读 [输入范围与 Git 取证](references/git-scopes.md)。
- 理解行为与选择审查深度时读 [审查视角与取舍](references/review-lenses.md)。
- 确定报告路径、复审约定、问题格式与验收判定时读 [问题与报告合同](references/report-contract.md)。

```formal
INTENT:
  GOAL: assess_actionable_defects_and_promised_behavior_of_code_changes
  SCOPE: [requested_diff, relevant_cross_file_impact, requirement_compliance,
          verification_evidence, repair_effect_recheck]
  SUCCESS: [evidence_grounded_conclusions, explicit_coverage_and_limits,
            complete_report_delivered]
  NON_GOALS: [plan_only_review, whole_repository_audit, unrequested_repairs]

RULE @review_standard:
  MUST: codex_native_review_defect_threshold
  MUST_NOT: finding_count_as_quality_metric

RULE @review_boundary:
  MUST: code_and_plan_read_only_by_default
  MUST_NOT: [automatic_repairs, automatic_plan_progress_updates, automatic_commits,
             automatic_pr_comments, automatic_pr_approval]

RULE @authorized_repairs:
  WHEN: user_also_authorized_repairs
  MUST: [preserve_original_review_baseline_before_repairs,
         distinguish_post_repair_recheck_results]

RULE @runtime_context:
  MUST: [discover_target_repository_paths_stack_rules_and_tools_at_runtime,
         follow_user_language, work_independently_of_locally_installed_codex_cli_or_community_skills]

WORKFLOW @review_code:
  DO: [read_applicable_instructions_and_summarize_intent,
       resolve_and_freeze_review_scope_and_report_path,
       establish_coverage_checklist,
       recheck_previous_issues_when_applicable,
       read_diff_and_trace_required_behavior,
       validate_and_countercheck_candidate_findings,
       adjudicate_and_recheck_snapshot_consistency,
       write_and_reread_complete_report_unless_chat_only,
       write_registered_project_status_after_primary_artifact_delivery_when_allowed,
       deliver_verdict_report_link_and_next_step_commands]
  OUTPUT: code_change_review_report
```

## 1. 确定审查对象

```formal
RULE @scope_evidence:
  MUST: [read_repository_subdirectory_and_review_specific_instructions,
         summarize_change_intent_in_one_sentence,
         distinguish_explicit_user_requirements_from_code_inferred_intent,
         resolve_input_using_git_scopes_reference,
         freeze_repository_root_mode_base_and_target_sha,
         freeze_worktree_snapshot_file_limits_and_optional_plan_version]
  MUST_NOT: missing_commit_message_automatically_stops_review

RULE @plan_default:
  WHEN: only_development_plan_given_without_another_explicit_mode
  MUST: use_plan_registered_commits_and_current_uncommitted_changes_as_scope

RULE @uncommitted_default:
  WHEN: no_mode_specified_and_uncommitted_changes_exist_and_plan_default_does_not_apply
  MUST: declare_uncommitted_mode

RULE @initial_review_without_target:
  WHEN: initial_review_with_clean_worktree_and_no_explicit_target
  MUST: ask_for_review_target
  MUST_NOT: [silently_choose_latest_commit, silently_choose_default_main_branch]

RULE @ambiguous_scope:
  WHEN: conflicting_modes_or_missing_ref_or_ambiguous_range_endpoints
  MUST: [clarify_affected_scope, allow_independent_instruction_reading]
  MUST_NOT: [silently_replace_baseline, silently_expand_to_whole_repository]

RULE @coverage_checklist:
  MUST: [resolve_report_path_using_report_contract,
         list_changed_files_and_behaviors_required_upstream_and_downstream,
         list_plan_scope_if_any_verification_methods_and_evidence_gaps,
         include_report_write_and_reread]
```

## 修复效果复审

```formal
RULE @recheck_selection:
  WHEN: explicit_recheck_or_plan_mode_with_existing_code_review_report
  MUST: [enable_repair_effect_recheck, read_report_contract_recheck_terms,
         default_to_highest_report_round_as_previous_in_plan_mode,
         honor_explicit_full_review_request_with_full_review_process,
         locate_previous_report_from_user_input_context_round_or_explicit_record_link]
  MUST_NOT: choose_ambiguous_previous_report_by_latest_file_mtime
  ON_UNMET: clarify_only_ambiguous_scope

RULE @recheck_coverage:
  WHEN: repair_effect_recheck
  MUST: [default_cover_all_previous_issues_unverified_items_and_direct_repair_regressions,
         honor_user_selected_ids_or_scope_and_mark_others_unreviewed,
         recheck_each_old_issue_in_current_code,
         inspect_repair_diff_for_newly_introduced_or_activated_defects]
  MUST_NOT: [require_old_issue_to_overlap_repair_diff,
             require_old_issue_to_be_introduced_by_current_repair,
             present_remaining_old_issue_as_new_finding]

RULE @missing_previous_report:
  WHEN: recheck_requested_without_usable_previous_report
  MUST: allow_current_target_review
  MUST_NOT: claim_previous_issues_individually_closed

RULE @recheck_versions:
  WHEN: repair_effect_recheck
  MUST: [distinguish_previous_review_baseline_repair_diff_and_current_checked_version,
         derive_and_declare_diff_from_parseable_report_version_when_user_omits_new_diff,
         recheck_evidenced_old_issues_and_limit_new_issue_check_when_diff_is_unknown,
         recheck_old_issues_even_with_clean_worktree]
  MUST_NOT: [apply_initial_no_diff_rule_to_skip_old_issue_recheck,
             claim_full_original_change_review_from_repair_increment_only]

RULE @recheck_delivery:
  WHEN: repair_effect_recheck
  MUST: [apply_new_finding_gates_only_to_new_findings,
         output_issue_by_issue_recheck_table_using_source_ids,
         preserve_previous_report_links_and_versions,
         default_save_new_report_and_preserve_historical_conclusions,
         link_independent_review_record_to_previous_round_when_ledger_enabled]
  MUST_NOT: partial_pass_overwrites_previous_unchecked_scope
```

```text
$review-code BASE_SHA..FIXED_SHA，对照 goal/LOGIN.code-review.md 复审修复效果，逐项记录结果。
```

## 2. 理解需求与行为

```formal
RULE @behavior_context:
  MUST: [read_complete_diff_before_risk_directed_context,
         inspect_relevant_functions_callers_models_permission_boundaries_tests_and_configuration,
         use_code_outside_diff_to_prove_impact,
         reread_truncated_output_in_parts_or_disclose_uncovered_scope,
         select_applicable_review_lenses_and_depth_from_actual_changes]
  MUST_NOT: [unrelated_preexisting_issues_as_new_findings,
             mandatory_full_checklist_or_finding_per_dimension]

RULE @plan_compliance:
  WHEN: development_plan_provided
  MUST: [read_requested_plan_version_and_necessary_references,
         extract_current_task_or_milestone_promises_requirement_and_acceptance_ids_and_exclusions,
         verify_actual_consequences_of_omissions_deviations_and_scope_expansion,
         treat_plan_as_contract_material_to_verify,
         require_only_current_scope_commitments,
         list_unclear_scope_as_pending_and_continue_independent_code_review]
  MUST_NOT: [author_completion_claim_as_code_evidence,
             report_future_milestone_work_as_current_defect,
             modify_plan_or_turn_code_review_into_plan_architecture_review]

RULE @without_plan:
  WHEN: no_development_plan_provided
  MUST: review_using_user_requirements_existing_contracts_and_call_chains

RULE @reviewer_assignment:
  MUST: current_agent_reviews_by_default

RULE @requested_parallel_review:
  WHEN: user_or_applicable_repository_rules_require_parallel_reviewers
  MUST: [same_baseline_and_explicit_read_only_scope_for_each_reviewer,
         complementary_behavior_chains_or_risk_dimensions,
         main_agent_rechecks_raw_evidence_deduplicates_and_alone_writes_report]
  MUST_NOT: [majority_opinion_as_evidence, fabricated_independent_review]
```

## 3. 验证候选问题

```formal
RULE @new_finding_admission:
  WHEN: candidate_enters_current_round_new_findings
  MUST: [introduced_or_actually_activated_by_reviewed_change,
         explain_baseline_to_target_behavior_difference,
         prove_causal_chain_when_change_makes_old_code_newly_reachable,
         concrete_reachable_input_or_state_and_failure_behavior,
         identify_affected_caller_or_contract_and_minimal_repair_direction,
         material_correctness_security_performance_or_maintainability_impact,
         repair_expectation_matches_actual_repository_standards,
         directly_read_target_code_and_relevant_requirements_or_rules,
         verify_explicit_behavior_changes_against_promised_contracts,
         use_minimal_reproduction_when_needed]
  MUST_NOT: [vague_possible_cross_module_impact,
             pure_style_personal_preference_unrelated_refactor_or_speculative_defense,
             rely_on_unexpressed_author_intent,
             intentional_behavior_change_alone_as_bug]
  # 旧问题按复审表独立核对。

RULE @existing_protection:
  MUST: check_existing_validation_types_upstream_filters_transactions_and_compensation
  MUST_NOT: infer_failure_only_from_missing_local_try_catch_null_check_or_cache

RULE @verification_execution:
  WHEN: validating_review_candidate
  MUST: [choose_existing_tests_type_checks_or_minimal_experiments_relevant_to_behavior,
         read_commands_and_configuration_before_execution,
         use_isolated_temporary_directory_for_written_reproductions,
         record_command_tested_version_actual_result_and_limits,
         mark_passed_only_after_actual_execution]
  MUST_NOT: [automatic_fixes_snapshot_updates_migrations_or_deployments,
             access_real_business_data,
             add_tests_or_repairs_to_target_repository,
             mandatory_unrelated_full_test_suite,
             test_pass_replaces_requirement_check]
  ON_UNMET: retain_sufficient_static_evidence_or_list_unverified_item

RULE @historical_verification:
  WHEN: reviewing_historical_commit_or_range
  MUST: [run_verification_on_corresponding_target_snapshot,
         compare_baseline_when_needed_to_distinguish_existing_and_new_failures]
  MUST_NOT: current_checkout_test_proves_another_commit

RULE @external_and_domain_evidence:
  MUST: [consult_and_cite_version_matched_official_material_when_local_evidence_cannot_confirm_external_contract,
         disclose_evidence_gap_when_material_unavailable,
         state_actual_data_scale_or_complexity_for_performance_issue,
         state_input_control_and_reachable_path_for_security_issue]
  MUST_NOT: pattern_match_alone_as_vulnerability_conclusion

RULE @candidate_countercheck:
  MUST: [seek_counterevidence_for_each_candidate,
         merge_duplicates_by_root_cause_and_repair_action,
         retain_all_independent_valid_issues,
         move_insufficiently_evidenced_candidates_to_unverified_items]
  MUST_NOT: [finding_quota, keep_only_most_severe_few, uncertain_issue_disguised_as_p3]
```

## 4. 裁定、落盘与交付

```formal
RULE @finding_presentation:
  MUST: [follow_report_contract_short_precise_location_single_paragraph_p0_to_p3_and_confidence,
         new_finding_primary_location_overlaps_reviewed_diff,
         allow_old_issue_recheck_and_supporting_evidence_outside_diff,
         cite_applicable_rule_file_and_minimal_line_range_when_material_to_conclusion]

RULE @snapshot_consistency:
  MUST: [recheck_coverage_checklist_and_version_changes,
         recheck_affected_conclusions_when_relevant_files_or_plan_change,
         retain_snapshot_version_and_gaps_when_changes_cannot_converge]
  MUST_NOT: new_report_file_becomes_current_review_input

RULE @complete_report:
  MUST: [review_object_before_findings,
         confirmed_findings_unverified_items_coverage_and_verification,
         plan_compliance_only_when_plan_provided,
         overall_conclusion_and_repair_order,
         acceptance_verdict_on_first_line_of_overall_conclusion]
  # 正确性结论默认使用原词；inconclusive 是本 skill 的证据不足扩展。
  MUST_NOT: claim_inconclusive_is_native_codex_json_value

RULE @report_persistence:
  UNLESS: user_explicitly_requests_chat_only_or_no_file
  MUST: [write_complete_markdown_report_even_with_empty_findings_or_insufficient_verification,
         reread_actual_report_for_findings_evidence_baseline_limits_and_conclusion,
         link_actual_existing_file]
  ON_UNMET: [state_report_not_saved_and_reason_and_target_path,
             deliver_complete_report_in_chat,
             do_not_claim_successful_save]

OUTPUT:
  FORMAT: markdown
  STYLE: follow_user_language
  SECTIONS: [acceptance_verdict, report_link, overall_conclusion,
             important_findings_or_limits, copyable_next_step_commands]
  correctness_values: ["patch is correct", "patch is incorrect", inconclusive]
  verdict_source: references/report-contract.md

REVIEW:
  CHECK: [@new_finding_admission, @recheck_coverage, @snapshot_consistency,
          @complete_report, @report_persistence]
  ALSO: [coverage_disclosure, evidence_traceability, actual_report_links]
  EXPOSE: on_issue
```

## 5. 验收判定与下一步

```formal
RULE @acceptance_continuation:
  MUST: [code_review_as_acceptance_gate_after_plan_implementation,
         review_repair_recheck_continuation_until_pass_or_explicit_user_waiver,
         final_reply_starts_with_contract_acceptance_verdict,
         next_step_commands_use_current_client_skill_syntax_and_actual_paths]
  MUST_NOT: automatically_start_repairs_rechecks_ui_acceptance_or_prd_gap_review

RULE @needs_changes_next:
  WHEN: "验收判定为「需要修改」"
  MUST: suggest_apply_code_review_with_current_report_then_review_code_recheck

RULE @passed_next:
  WHEN: "验收判定为「通过验收」或「用户豁免通过」"
  MUST: |
    变更涉及界面且没有覆盖当前版本的 UI 验收通过时，可选 review-ui <计划路径>；
    计划来自 PRD 时，可选 review-prd-dev-gaps <计划路径>；
    都不适用时说明本计划代码验收完成。仍有 P3 时，可选
    apply-code-review <本报告路径>，处理后照常复审。

RULE @prd_origin:
  MUST: |
    计划是否来自 PRD，以计划顶部「需求来源」为准；旧计划没有该行时，
    账本来源明确指向 PRD 的也算。未附计划时不提示差异核查。

RULE @needs_evidence_next:
  WHEN: "验收判定为「需补证」"
  MUST: [list_missing_evidence_or_verification_and_how_to_obtain,
         suggest_review_code_with_plan_or_original_scope_after_evidence_ready]

RULE @explicit_waiver:
  WHEN: user_explicitly_waives_remaining_issues_for_current_period_at_invocation_or_after_review
  MUST: [append_contract_waiver_record_to_current_report_and_reread,
         give_passed_next_steps]
  MUST_NOT: waiver_described_as_repaired
```

调用示例（自然语言参数，不要求专用解析器）：

```text
使用 $review-code goal/LOGIN.md
使用 $review-code 审查当前未提交变更。
使用 $review-code 审查未提交的 src/auth.ts 和 tests/auth.test.ts。
使用 $review-code 审查 commit abc1234，参考 docs/plan/login.md 的 M2。
使用 $review-code 审查 a1b2c3d..d4e5f6a，报告写到 docs/reviews/login.code-review.md。
使用 $review-code 审查相对 origin/main 的差异。
使用 $review-code 审查相对 main 的已提交差异，仅聊天输出。
```

## 项目状态回写

```formal
RULE @project_status:
  WHEN: repository_has_registered_ledger_or_existing_sdlc_protocol
  MUST: |
    交付原产物后读取仓内回写规则；默认协议为仓根 .xgent-ai/sdlc/protocol.md，
    兼容已有 docs/project-status/protocol.md。两套并存且未登记切换时先核对，不双写。
    按协议更新受影响对象当前状态与来源链接，必要时维护索引；v4 日常直接更新当前页，
    事件留存条件或旧版事件要求以仓内协议为准。只读／仅聊天／不保存要求优先；
    登记失败须说明，不影响如实交付已完成的原产物。
  MUST_NOT: [create_ledger_when_not_enabled, require_separately_installed_skill]

RULE @review_status_record:
  WHEN: project_status_writeback_allowed
  MUST: [independent_id_for_each_actual_review,
         exact_base_and_target_or_worktree_fingerprint_scope_conclusion_and_report_path,
         plan_association_only_from_actual_input_and_allow_no_plan,
         link_recheck_to_previous_round_and_preserve_old_conclusion]
  MUST_NOT: "将 patch is correct 登记为已验收、已合并或已发布"
```
