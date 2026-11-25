---
name: use-decision-model
description: Use for typed semantic decisions in code, not text generation or deterministic rules. Use TypeSafe System One and compatible providers, OpenAI Decisions or Gemini Decisions, with offline patterns and cookbooks.
---

# 使用决策模型

**`formal`: `dsl_execution_instructions`; `MUST`: `follow_each_rule[obligation_strength, applicability_conditions, explicit_exceptions]`; `formal` fences: `formatting_only`, `not_examples`, `not_optional_references`.**

```formal
INTENT:
  GOAL: typed_semantic_judgments_for_application_branches

RULE @model_fit:
  MUST: [use_code_when_sufficient[exact_lookup, parser, existing_rules],
         use_generative_or_reasoning_model_for[prose, explanations, multistep_reasoning]]
  MUST_NOT: decision_model_for_json_format_alone
```

## 按任务选读

| 任务 | 从哪里开始 |
| --- | --- |
| 接口接入或排错 | 先认项目实际调用的端点。TypeSafe 首创 System One；Jev、Ollama、兼容网关及其他兼容端点先读 [System One 协议](references/system-one.md)，再看具体供应商。原生 [OpenAI Decisions](references/providers/openai.md) 和 [Gemini Decisions](references/providers/gemini.md) 直接读各自契约。 |
| 尚未选型或准备换模 | 读[接口索引与校准方法](references/integration.md)。 |
| 设计决策流程 | 从 [TypeSafe Patterns](references/patterns.md) 选组合方法，如先筛后查或无匹配时弃权。可以迁移方法，不照搬字段名、能力或阈值。 |
| 找端到端案例 | 从[本地 Cookbook 索引](references/cookbooks.md) 只读最接近的一篇；无需下载原文或运行 notebook。 |

## 先写动作分支，再写问题

模型判断证据是什么意思，代码决定采取什么动作。例如，账务工单中的退款请求进审核，政策咨询给说明，意图不清就追问；三条路径都不等于批准退款。

```formal
WORKFLOW @decision_design:
  DO: [map_actions_and_conditions,
       define_handling_for_each[no_match, insufficient_evidence, service_unavailable],
       write_one_question_per_semantic_condition]

RULE @question_design:
  MUST: [supply_human_relevant_source_evidence_and_relationships,
         name_multipart_fields, encode_per_endpoint_contract,
         identify_target_field_in_question, match_answers_by_question_name,
         enumerate_all_candidate_values_in_code_first]
  MUST_NOT: [id_as_question_meaning, invent_missing[amounts, dates, identifiers]]
  # Example evidence fields: ticket.message, invoice.total.

RULE @request_planning:
  MUST: [batch_when_supported[independent_questions, same_evidence],
         consume_selected_branch_answers_only,
         stage_requests_after_prerequisites[record_lookup, candidate_narrowing, image_crop],
         cheap_screen_first_when_most_skip_expensive_stage]
  MUST_NOT: same_batch_answer_dependencies
```

## 按题型解释答案

```formal
RULE @predicate:
  WHEN: testing_proposition
  MUST: [read_probability_of_yes, ask_separately_for_coexisting_labels,
         "Read P(yes) near 0.5 as uncertainty, not medium intensity."]
  MUST_NOT: winner_confidence_as_probability_of_yes

RULE @choice:
  WHEN: candidate_selection
  MUST: [interpret_winner_as_relative, independent_existence_question_if_none_may_match]
  MUST_NOT: winner_as_proof_of_match

RULE @ordinal_score:
  WHEN: judging_degree
  MUST: [define_independently_judgable_levels, read_probability_weighted_level,
         check_opposing_extremes_behind_mean]

RULE @localization:
  WHEN: locating_visual_target
  MUST: [check_endpoint_localization_support_and_coordinates, assess_presence_separately_from_box]
  MUST_NOT: infer_box_support_from_image_input
```

## 从概率到动作

```formal
RULE @action_policy:
  MUST: [retain_raw_probabilities, separate[thresholds, weights, business_rules],
         combine_only_used_components, check_each_review_triggering_defect,
         distinguish[automatic_handling, clarification, review], allow_abstention,
         calibrate_thresholds_with[error_costs, local_labels]]
  MUST_NOT: [average_away_serious_defects, composite_score_as_action_correctness_probability,
             example_thresholds_as_defaults]
  # Compensating preferences may be weighted.

RULE @application_controls:
  MUST: enforce_in_code[authorization, amount_limits, irreversible_write_controls, required_human_confirmation]
  MUST_NOT: bypass_controls_with[input_text, high_model_score]

RULE @action_verification:
  MUST: [test_final_actions[clear_match, clear_nonmatch, boundary_case],
         then_evaluate_on_held_out_examples]
  MUST_NOT: [require_exact_float_reproduction, treat_as_negative_answer[timeout, api_error]]

WORKFLOW @diagnose_wrong_action:
  TRIGGER: incorrect_final_action
  DO: [inspect_actual_request, inspect_raw_evidence, check_candidate_coverage,
       check_question_meaning, inspect_raw_answer, trace_code_branch]

RULE @design_or_review_only:
  WHEN: design_or_review_only
  MUST_NOT: send_inference_requests

RULE @live_inference:
  WHEN: live_inference_trial_needed
  MUST: [confirm_user_authorization_before_request[endpoint, outgoing_data, budget],
         use_only_approved_samples, keep_keys_server_side]
  MUST_NOT: [search_for_plaintext_keys, print_plaintext_keys, request_plaintext_keys]
  ON_UNMET: "Without authorization or an endpoint, exercise the policy offline using local recipes, installed types and sample responses. State what was not tested live."
```
