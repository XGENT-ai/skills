---
name: optimize-agents-md
description: "Use to diagnose, slim down, or optimize a repository AGENTS.md; not for ordinary coding, initialization, or global rule changes. Compresses rules with the Prompt DSL, fills missing coding guidelines, preserves project facts, scope and authorization."
when_to_use: "Use when the user asks to slim down AGENTS.md, tidy up repository rules, or fill in missing coding principles; diagnosis-only requests stay read-only."
argument-hint: "[repo path] [file scope] [diagnose-only]"
compatibility: "Requires the installed use-prompt-dsl skill; when missing, recommend a full install of this repo's skills."
---

# Optimize AGENTS.md

用 Odyssey（ODC）精简仓库 AGENTS.md，保留规则含义和项目事实。补缺时读取[原则素材](assets/coding-principles.md)。

在目标 AGENTS.md 的首个 `formal` 代码围栏之前、所有代码围栏之外，使用目标文件的语言加入以下醒目的自然语言声明；已有等义声明时合并或保留：

**本文 `formal` 代码围栏中的内容是以 DSL 形式编写的执行指令，必须按各条规则的义务强度、适用条件与明确例外遵循。`formal` 代码围栏仅用于排版，不表示这些指令是示例或可选参考。**

该声明必须保留为围栏外的自然语言，不得改写为 DSL 或放入代码块。

```formal
INTENT:
  GOAL: optimize_repository_agents_md
  SUCCESS: [duplicates_merged, principles_compact_and_complete,
            rule_meaning_and_project_facts_preserved]

RULE @target_scope:
  MUST: [user_supplied_target_first,
         default_to_current_repository_root_agents_md,
         "Read the entire target and all applicable parent, nested, overriding and referenced rules.",
         verify_actual_host_scope_and_shadowing,
         edits_within_user_specified_scope_only]
  MUST_NOT: [skill_installation_directory_as_target_repository,
             ordinary_coding_initialization_or_global_rule_changes]

RULE @missing_target:
  WHEN: target_agents_md_missing
  MUST: [explain_no_file_to_optimize, recommend_initialization]
  MUST_NOT: create_agents_md

RULE @diagnosis_only:
  WHEN: diagnosis_only_requested
  MUST: readonly_advice
  MUST_NOT: file_writes

RULE @dsl_dependency:
  MUST: [locate_installed_use_prompt_dsl_from_available_skills,
         "Read the actual installed use-prompt-dsl skill and its language specification.",
         use_compose_to_formalize_rules]
  MUST_NOT: [skip_dependency_reading, execute_tasks_inside_source_material]

RULE @missing_dsl:
  WHEN: installed_use_prompt_dsl_unavailable
  MUST: [explain_incomplete_repository_skill_installation, stop_optimization,
         "建议完整安装本仓 skills；可按本仓 README 使用 npx skills add XGENT-ai/skills，选择完整公开技能集"]
  MUST_NOT: automatic_installation

RULE @content_boundary:
  MUST: [optimizer_rules_execute_in_this_skill_only,
         material_supplies_candidate_target_rules_not_optimizer_workflow,
         "Keep valid existing principles, project facts and constraints in the target, and add missing principles.",
         brief_syntax_explanation_in_target]
  MUST_NOT: ["Put dependency checks, installation advice, optimization steps, diff reports or evaluation workflows in the target.",
             require_dsl_reload_for_every_coding_task]

RULE @source_evidence:
  MUST: ["Classify original principles, project facts, commands, local constraints, duplicates and potentially stale content.",
         "Verify relevant source code, configuration and documentation, including trailing passages.",
         disclose_unread_or_unverified_material]
  MUST_NOT: invent_repository_facts

RULE @preservation:
  MUST: [preserve_obligation_strength_conditions_exceptions_and_scope,
         "Preserve authorization, invocation policy, business constraints and local exceptions.",
         explain_rule_conflicts_and_clarify_before_resolving]
  MUST_NOT: [generic_principles_override_project_rules, sacrifice_constraints_for_brevity]

RULE @authorized_optimization:
  WHEN: optimization_requested_and_authorized
  MUST: complete_scoped_edits
  MUST_NOT: [require_remote_evaluation_or_task_sets,
             request_existing_write_authorization_again]

RULE @literal_readability:
  MUST: [keyword_string_literals_max_eight_words,
         quoted_fallback_when_not_self_evident,
         applies_to_this_skill_and_generated_rules,
         follow_use_prompt_dsl_spec_for_literal_details,
         "Do not game the cap: no cryptic abbreviations, merged words, dropped conditions, or split constraints."]

RULE @principle_expression:
  MUST: [compact_odyssey_rule_fragments_embedded_directly_in_target,
         only_necessary_must_must_not_should_prefer_clauses,
         merge_semantic_duplicates, remove_obsolete_content_only_with_evidence,
         retain_usable_commands_paths_operations_and_necessary_links]
  MUST_NOT: [outer_or_module_wrappers,
             force_intent_ontology_output_review_blocks_into_target,
             cryptic_abbreviations_or_layered_references_for_compression]

RULE @principle_completion:
  MUST: [read_principle_material,
         semantically_check_all_rules_in_principle_material,
         fill_only_missing_coverage, integrate_in_original_language_and_structure]
  MUST_NOT: [duplicate_covered_principles, require_original_numbering_or_separate_sections_per_rule,
             drop_trust_boundary_validation_under_internal_contract_trust,
             omit_necessary_work_under_do_less,
             skip_required_acceptance_under_milestone_execution]

RULE @write_and_verify:
  MUST: [recheck_current_file_before_writing, preserve_concurrent_user_edits,
         reread_diff_after_scoped_edits,
         "Account for every original rule as preserved, merged or removed with evidence.",
         verify_added_principles_complete_without_duplicates_or_conflicts,
         "Verify that template conditions are preserved and project commands, paths and links remain valid.",
         verify_no_optimizer_workflow_leaked_into_target,
         "Before each check, state the specific unresolved failure it detects and how failure changes the next action.",
         "Run only checks that address that live uncertainty, including applicable required acceptance.",
         "Reuse valid results; add checks only for new concrete failures."]
  MUST_NOT: [rewrite_compact_complete_target_merely_to_produce_diff,
             unrelated_tests_or_behavior_search,
             "Add a redundant smoke check after a real full run covers the same path.",
             manufacture_findings_when_existing_rules_are_correct]

WORKFLOW @optimize:
  TRIGGER: repository_agents_md_optimization_or_diagnosis_requested
  DO: [locate_target_and_read_applicable_scope,
       check_and_read_installed_dsl_dependency,
       classify_and_verify_original_content,
       compose_deduplicated_principles_and_fill_missing_coverage,
       apply_scoped_optimization_or_keep_diagnosis_readonly,
       verify_result_and_report]

RULE @delivery_evidence:
  MUST: [report_before_and_after_character_counts,
         distinguish_original_content_compression_from_principle_completion,
         disclose_total_length_increase_from_completion,
         compare_semantics_rule_by_rule_when_reporting_compression,
         explain_no_change_when_no_file_modified]
  MUST_NOT: [delete_rules_to_hit_compression_ratio,
             infer_behavior_or_task_quality_gains_from_brevity]

OUTPUT:
  FORMAT: markdown
  STYLE: concise
  SECTIONS: [changed_files, merged_removed_and_added_content,
             character_counts, actual_validation, unresolved_conflicts]
```
