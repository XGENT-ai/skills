# AGENTS.md

Combine with project-specific rules. Favor caution over speed; use judgment for trivial tasks.

Odyssey: `RULE` names a rule; `WHEN` limits scope; `MUST` requires, `MUST_NOT` forbids, `PREFER` favors. Lists apply item by item; this file's authority/scope are unchanged.

## 1. Think Before Coding

```formal
RULE @think_before_coding:
  WHEN: before_implementation
  MUST: [state_assumptions_explicitly, present_alternative_interpretations_and_tradeoffs,
         surface_simpler_approaches, push_back_when_warranted,
         stop_identify_and_ask_when_uncertain_or_confused]
```

## 2. Simplicity First

```formal
RULE @simplicity:
  MUST: [simplest_correct_solution, simplify_if_overcomplicated,
         each_added_check_abstraction_dependency_error_branch_test_or_ui_worth_its_upkeep,
         complete_necessary_work_correctly,
         remove_existing_redundant_complexity_before_adding_abstractions_or_dependencies]
  MUST_NOT: [unrequested_features_flexibility_config_compatibility_ui_or_architecture,
             speculative_design, single_use_abstractions, impossible_scenario_defenses,
             add_layers_to_overcomplicated_design, silently_expand_scope_for_theoretical_robustness]
  PREFER: fewer_moving_parts

RULE @speculative_complexity:
  WHEN: adding_complexity_for_speculative_or_vanishingly_unlikely_race_or_security_edge_cases
  MUST: [explain_concrete_failure_mode_likelihood_and_complexity_cost_to_user,
         obtain_user_buy_in_after_explanation_and_before_adding_complexity]

RULE @requirement_confirmation:
  WHEN: non_brainstorming
  MUST: [propose_unrequested_requirements_only_when_necessary,
         obtain_user_confirmation_before_adopting_suggested_requirements]

RULE @failure_handling:
  MUST: [validate_only_at_system_boundaries_including_user_input_external_apis_and_files,
         trust_established_internal_contracts_invariants_and_framework_guarantees,
         handle_only_real_actionable_failures]
  MUST_NOT: [redundant_internal_checks, unnecessary_fallbacks_or_excess_defenses,
             catches_or_silent_recovery_masking_programming_errors]

RULE @user_experience:
  MUST: user_expectations_first
  MUST_NOT: [developer_notes_implementation_comments_or_internal_instructions_in_ui,
             unnecessary_engineering_or_obvious_interaction_explanations,
             prompts_or_warnings_unless_helping_meaningful_decisions_or_preventing_major_errors]
```

## 3. Surgical Changes

```formal
RULE @surgical_changes:
  MUST: [minimal_change, every_changed_line_traces_to_request,
         match_existing_style_and_patterns, reuse_or_simplify_before_abstracting,
         remove_own_newly_unused_imports_variables_and_functions,
         report_unrelated_dead_code]
  MUST_NOT: [unrelated_code_comment_or_formatting_improvements, refactoring_unbroken_code,
             delete_preexisting_dead_code_unless_requested,
             feature_flags_or_compatibility_shims_when_direct_changes_suffice,
             compatibility_hacks_using_unused_variable_underscore_renames_or_type_reexports]

RULE @documentation:
  MUST: [comments_explain_why, concise_current_searchable_docs,
         add_comments_only_for_nonobvious_logic_or_non_self_descriptive_code,
         update_or_remove_stale_comments_and_docs_on_behavior_change]
  MUST_NOT: [code_paraphrases_in_comments, development_history_annotations,
             stale_text_with_appended_corrections, duplicate_docs,
             add_docstrings_comments_or_type_annotations_to_otherwise_unchanged_code]
```

## 4. Goal-Driven Execution

```formal
RULE @goal_driven_execution:
  MUST: [verifiable_success_criteria, brief_multistep_plan_with_checks,
         test_invalid_inputs_then_implement_validation,
         reproduce_bug_in_test_before_fix, tests_pass_before_and_after_refactor,
         iterate_until_verified, prioritize_quality_and_completeness_over_token_savings,
         full_effort_in_required_reasoning_investigation_implementation_and_verification,
         concise_communication, respect_agreed_scope_and_stopping_rules,
         autonomously_finish_remaining_in_scope_work,
         fix_related_in_scope_defects_found_during_verification]
  MUST_NOT: [stop_at_findings_while_actionable_in_scope_work_remains,
             open_ended_hunts_for_hypothetical_or_unrelated_problems]

RULE @meaningful_tests:
  MUST: [test_meaningful_behavior_key_boundaries_and_real_regressions,
         exercise_interfaces_as_external_callers_and_assert_caller_observable_outcomes,
         remove_requested_obsolete_or_redundant_tests,
         delete_requested_tests_that_pass_when_all_tested_functions_are_empty_and_return_no_data,
         run_smallest_relevant_check,
         reuse_valid_verification_when_relevant_code_configuration_and_conditions_unchanged,
         retest_only_after_meaningful_change_new_failure_or_concrete_acceptance_evidence_gap]
  MUST_NOT: [redundant_or_trivial_tests, implementation_coupled_tests,
             per_function_coverage_chasing, exhaustive_imaginary_failures,
             unnecessary_regression_burden,
             whole_suite_repetition_for_reassurance_or_report_challenge_alone]

RULE @automated_test_scope:
  MUST: automated_tests_assert_outcomes_produced_by_deterministic_code
  MUST_NOT: [write_automated_tests_for_llm_output, write_automated_tests_for_static_source_text]

RULE @complete_milestones:
  WHEN: long_task
  MUST: [understand_full_scope_and_acceptance_first, complete_coherent_usable_milestones,
         targeted_development_checks, broad_regression_at_milestones_and_before_delivery]
  MUST_NOT: [repeated_stops_after_fragments, full_suite_after_each_micro_edit,
             test_frequency_as_progress]

RULE @temporary_resource_cleanup:
  WHEN: code_and_test_reviews_completed
  MUST: [inspect_task_related_temporary_directories_git_worktrees_branches_and_docker_containers,
         identify_resources_safe_to_remove, verify_no_background_task_dependencies_before_cleanup,
         promptly_remove_safe_temporary_resources_to_reclaim_disk_space]
```

## 5. Evidence & Quotation

```formal
RULE @evidence:
  MUST: [read_relevant_source_specs_and_docs_before_conclusions,
         cover_all_relevant_sections_and_tail, inspect_behavior_and_contracts_before_changes,
         disclose_unverified_items, durable_claims_traceable_to_source_passage_and_content]
  MUST_NOT: [invented_requirements_or_constraints, unread_or_truncated_content_as_known,
             source_details_reconstructed_from_memory]
```

Read the exact passage before quoting wording, numbers, identifiers, statuses, commands or dates; re-read truncated passages before use. When a quote contradicts its source, search the whole repo, fix every copy of that error and notify the owner; leave no conflicting versions.

## 6. Skill Descriptions

- `SKILL.md` descriptions: triggers and essential exclusions first, then outcome. Put misrouting exclusions beside triggers, never only at the end or in the body.
- Target ≤250 characters (Chinese ≤200), preferably ~100; the first 100 should convey applicability independently. These are repo writing targets, not universal agent truncation limits.
- Put procedures, exhaustive error lists, tool details and duplicate bilingual text in the body/references; descriptions select skills, not list workflows.
- After editing, read the full description and its first 100 characters. Preserve intent, boundaries and unrelated frontmatter/instructions; no mechanical cuts. Validate YAML with `npm test`; generic `quick_validate.py` rejects this repo's intentional `argument-hint`, `when_to_use` and `disable-model-invocation` fields.

## 7. Reference Skill Integration Gate

Only for borrowing another skill's rules/workflows into an existing skill, not ordinary edits.

- Before editing, read current target rules/reference passages and show a concrete checklist in chat or the requested plan for each change: target file/rule, current behavior, reference source/passage, adopt/adapt/reject, impact on principles/stance, verification. Assess affected applicability, invocation, authorization, interfaces and acceptance requirements explicitly.
- Existing principles/stance are the baseline; adapt or reject conflicts. Integration does not authorize changes to principles, stance or invocation/authorization policy. Unresolved changes need explicit user direction; honor existing explicit authorization without asking again.
- Check the diff against the checklist; update it and resolve new conflicts before further behavior changes, never only after merging.

## Experience Capture

During development, use [experience-capture](skills/experience-capture/SKILL.md) at a natural pause for a useful non-obvious failure, user correction or repeated automatable workflow. Save a few current-context lines, then resume; no investigation, evidence fetching, full-record reload or task interruption for collection. Skip without a pause, when read-only or disabled. Deeper work waits for the user's `$agi-mode 复盘` / `$agi-mode reflect`.

## Working in This Repository

- Stay on the current branch unless the user explicitly requests a new one; historical examples and skill/workflow conventions are not authorization.
- Run Node/Python/metadata checks with `npm test`. Phoenix runtime changes also require `npm run test:phoenix`: mise, Rust 1.99.0 and pinned build tools are mandatory; missing tools fail, not skip. Run and record browser, native-platform and real provider-session acceptance separately; neither command proves those passed.
- `skills/<name>/` is the source of truth. Gitignored `.claude/skills/`, `.agents/skills/` and `.codex/skills/` are local installs. skills.sh copies need push/reinstall for local edits; source symlinks reflect them immediately. When work depends on recent changes and a loaded skill is an installed copy, first run `diff -rq .claude/skills/<name> skills/<name>` and use source when different. Point behavior checks at `skills/<name>/...`.
- When skill names, arguments, next-step commands, verdicts or status words change, update all interface restatements in the same change: README tables/loop; under `skills/agi-mode/`, `playbooks/sdlc.md`, `playbooks/what-next.md`, `evals/scenarios.md`, plus `assets/sdlc/state-model.md` and `assets/sdlc/protocol.md` for verdict/report-type changes; the skill's `agents/openai.yaml`; and copied rules like `skills/agi-mode/references/interface-design.md`. Find with `grep -rn '<skill-name>' skills README.md docs`. Each must fully describe the new behavior, not just avoid conflicts. Preserve historical records/snapshots; do not add absent `agents/openai.yaml`.
- For review-ui contract/`rules.v1.json` changes, sync the four contract hashes, rule version, Rust review policy, shared counterexamples and both UI skills. JSON projects existing contracts, without separate verdicts/policy.
- To create/revise skills, follow [Skill 编写与修订](skills/agi-mode/playbooks/authoring-a-skill.md); agi-mode is not auto-invoked (`disable-model-invocation`), so this playbook is not otherwise loaded.
