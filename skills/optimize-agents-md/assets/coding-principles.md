## Coding Principles

Odyssey 片段：`RULE` 声明本文作用域内的常驻规则；`MUST` 必须，`MUST_NOT` 禁止，`SHOULD` 通常应满足，`PREFER` 在不冲突时优先；`=>[a, b]` 表示若 a 则 b，`&&[a, b]` 表示同时满足。直接嵌入 AGENTS.md，不加外层包装。

```formal
RULE @think_before_coding:
  MUST: [state_assumptions_explicitly_before_implementation,
         when_uncertain_or_confused_stop_identify_and_ask,
         when_multiple_interpretations_exist_present_all_and_explain_tradeoffs,
         surface_simpler_approaches, push_back_when_warranted]
  MUST_NOT: silently_choose_or_hide_alternative_interpretations

RULE @simplicity:
  MUST: [simplest_correct_solution,
         validate_only_at_system_boundaries_including_user_input_external_apis_and_files,
         trust_internal_code_established_contracts_invariants_and_framework_guarantees,
         handle_only_real_actionable_failures]
  MUST_NOT: [speculative_abstractions, unnecessary_fallbacks,
             excessive_defenses, impossible_scenario_guards,
             repeated_validation_along_internal_call_chains,
             layered_catches_or_silent_recovery_masking_programming_errors]

RULE @scope:
  MUST: [implement_requested_scope_only, preserve_existing_patterns,
         reuse_or_simplify_before_abstracting,
         allow_complete_deletion_of_confirmed_unused_code_within_agreed_scope]
  PREFER: small_changes
  MUST_NOT: [unrequested_features_configuration_compatibility_layers_ui_or_architecture_changes,
             feature_flags_or_backward_compatibility_shims_when_direct_code_changes_suffice,
             compatibility_hacks_using_unused_variable_underscore_renames_or_type_reexports]

RULE @requirement_confirmation:
  WHEN: non_brainstorming
  MUST: [propose_unrequested_requirements_only_when_deemed_necessary,
         obtain_user_confirmation_before_adopting_suggested_requirements]
  MUST_NOT: unilaterally_add_unrequested_requirements

RULE @legacy_compatibility:
  UNLESS: user_explicitly_requests_legacy_compatibility
  MUST_NOT: [add_legacy_compatibility_requirements,
             add_legacy_mode_branches_or_compatibility_layers]

RULE @user_experience:
  MUST: [design_around_user_expectations, user_facing_copy_serves_users]
  MUST_NOT: [unnecessary_engineering_internal_state_or_obvious_interaction_explanations_in_ui,
             developer_notes_implementation_comments_or_internal_instructions_in_ui,
             prompts_or_warnings_unless_helping_meaningful_decisions_or_preventing_major_errors]

RULE @current_documentation:
  MUST: [comments_explain_why,
         add_comments_only_for_nonobvious_logic_or_non_self_descriptive_code,
         update_or_remove_stale_comments_and_docs_when_behavior_changes,
         current_concise_searchable_sources_of_truth]
  MUST_NOT: [add_docstrings_comments_or_type_annotations_to_otherwise_unchanged_code,
             comments_paraphrasing_code_or_recording_development_history,
             accumulated_historical_annotations_or_corrections_appended_to_stale_text,
             duplicate_documentation]

RULE @evidence:
  MUST: [read_relevant_source_specs_and_doc_passages_fully_before_conclusions,
         cover_all_relevant_sections_and_tail_in_long_documents,
         disclose_unverified_items,
         investigate_existing_behavior_and_contracts_before_changes]
  MUST_NOT: [infer_unread_content, invent_requirements_or_constraints]

RULE @meaningful_tests:
  MUST: [protect_meaningful_behavior_key_boundaries_and_real_regressions,
         exercise_interfaces_from_external_caller_perspective,
         assert_actual_caller_observable_outcomes,
         delete_tests_that_still_pass_when_all_tested_function_bodies_are_empty_and_return_no_data,
         remove_obsolete_or_redundant_tests_within_agreed_scope,
         reuse_valid_verification_when_relevant_code_configuration_and_conditions_unchanged,
         retest_only_after_meaningful_change_new_failure_or_discovered_concrete_acceptance_evidence_gap,
         run_smallest_relevant_check]
  MUST_NOT: [redundant_tests, trivial_assertions, implementation_coupled_tests,
             per_function_coverage_chasing, exhaustive_imaginary_failures,
             add_unnecessary_regression_burden,
             whole_suite_repetition_for_reassurance_or_report_challenge_alone]

RULE @automated_test_scope:
  MUST: automated_tests_assert_outcomes_produced_by_deterministic_code
  MUST_NOT: [write_automated_tests_for_llm_output,
             write_automated_tests_for_static_source_text]

RULE @complete_milestones:
  MUST: [prioritize_quality_and_completeness_over_token_savings,
         full_effort_in_required_reasoning_investigation_implementation_and_verification,
         concise_communication, respect_agreed_scope_and_stopping_rules,
         autonomously_finish_actual_remaining_work_within_agreed_scope,
         fix_related_defects_found_during_verification_within_agreed_scope,
         understand_full_scope_and_acceptance_before_long_tasks,
         deliver_coherent_complete_milestones,
         targeted_verification_coverage_during_development,
         broad_regression_coverage_at_meaningful_milestones_and_before_delivery,
         measure_progress_by_complete_usable_deliverables]
  MUST_NOT: [repeated_stops_after_fragmentary_changes,
             full_suite_after_every_micro_edit, test_frequency_as_progress,
             stop_at_findings_list_while_actionable_in_scope_work_remains,
             open_ended_hunts_for_hypothetical_or_unrelated_problems]

RULE @temporary_resource_cleanup:
  WHEN: code_and_test_reviews_completed
  MUST: [inspect_task_related_temporary_directories_git_worktrees_branches_and_docker_containers,
         identify_resources_safe_to_remove,
         verify_no_background_task_dependencies_before_cleanup,
         promptly_remove_safe_temporary_resources_to_reclaim_disk_space]

RULE @complexity_budget:
  MUST: [each_added_check_abstraction_dependency_error_branch_test_or_ui_element_justifies_maintenance_cost,
         remove_existing_redundant_complexity_before_adding_abstractions_or_dependencies,
         rethink_overcomplicated_design, complete_necessary_work_correctly,
         =>[adding_complexity_for_speculative_or_vanishingly_unlikely_race_or_security_edge_cases,
            &&[explain_concrete_failure_mode_likelihood_and_complexity_cost_to_user,
               obtain_user_buy_in_after_explanation_and_before_adding_complexity]]]
  MUST_NOT: [add_layers_to_an_overcomplicated_design,
             silently_expand_scope_for_theoretical_robustness]
  PREFER: fewer_additions
```
