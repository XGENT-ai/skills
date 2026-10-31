## Coding Principles

Odyssey 片段：`RULE` 声明本文作用域内的常驻规则；`MUST` 必须，`MUST_NOT` 禁止，`SHOULD` 通常应满足，`PREFER` 在不冲突时优先。直接嵌入 AGENTS.md，不加外层包装。

```formal
RULE @think_before_coding:
  MUST: [state_assumptions_explicitly_before_implementation,
         when_uncertain_or_confused_stop_identify_and_ask,
         when_multiple_interpretations_exist_present_all_and_explain_tradeoffs,
         surface_simpler_approaches, push_back_when_warranted]
  MUST_NOT: silently_choose_or_hide_alternative_interpretations

RULE @simplicity:
  MUST: [simplest_correct_solution,
         validate_user_input_external_api_and_file_trust_boundaries,
         trust_established_internal_contracts_and_invariants,
         handle_only_real_actionable_failures]
  MUST_NOT: [speculative_abstractions, unnecessary_fallbacks,
             excessive_defenses, impossible_scenario_guards,
             repeated_validation_along_internal_call_chains,
             layered_catches_or_silent_recovery_masking_programming_errors]

RULE @scope:
  MUST: [implement_requested_scope_only, preserve_existing_patterns,
         reuse_or_simplify_before_abstracting]
  PREFER: small_changes
  MUST_NOT: unrequested_features_configuration_compatibility_layers_ui_or_architecture_changes

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
         update_or_remove_stale_comments_and_docs_when_behavior_changes,
         current_concise_searchable_sources_of_truth]
  MUST_NOT: [comments_paraphrasing_code_or_recording_development_history,
             accumulated_historical_annotations_or_corrections_appended_to_stale_text,
             duplicate_documentation]

RULE @evidence:
  MUST: [read_relevant_source_specs_and_doc_passages_fully_before_conclusions,
         cover_all_relevant_sections_and_tail_in_long_documents,
         disclose_unverified_items,
         investigate_existing_behavior_and_contracts_before_changes]
  MUST_NOT: [infer_unread_content, invent_requirements_or_constraints]

RULE @meaningful_tests:
  MUST: protect_meaningful_behavior_key_boundaries_and_real_regressions
  MUST_NOT: [redundant_tests, trivial_assertions, implementation_coupled_tests,
             per_function_coverage_chasing, exhaustive_imaginary_failures]

RULE @complete_milestones:
  MUST: [understand_full_scope_and_acceptance_before_long_tasks,
         deliver_coherent_complete_milestones,
         targeted_checks_during_development,
         broad_regression_at_meaningful_milestones_and_before_delivery,
         measure_progress_by_complete_usable_deliverables]
  MUST_NOT: [repeated_stops_after_fragmentary_changes,
             full_suite_after_every_micro_edit, test_frequency_as_progress]

RULE @complexity_budget:
  MUST: [each_added_check_abstraction_dependency_error_branch_test_or_ui_element_justifies_maintenance_cost,
         rethink_overcomplicated_design, complete_necessary_work_correctly]
  MUST_NOT: add_layers_to_an_overcomplicated_design
  PREFER: fewer_additions
```
