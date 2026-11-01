# Odyssey（ODC）示例集

> 示例中的目标、产品、职责、工作流及验收条件只适用于对应场景。`@request / @input / @context / @output` 是约定上下文引用；其他引用必须指向当前可见的定义。模板不是事实，也不代表工具、Agent、任务调度或测试已经真实运行。

## 一、基础片段

### 1. 嵌入 AGENTS.md 的工程规则

规则可以直接嵌入仓库文件，不必增加目标、Persona、工作流或额外包装。

```formal
RULE @read_contract:
  WHEN: before_existing_behavior_change
  MUST: [read_relevant_contracts, resolve_material_ambiguity]
  ON_UNMET: [disclose, ask]

RULE @scope:
  MUST: [scope_fidelity, preserve_existing]
  MUST_NOT: [unrequested_features, unrequested_configuration]

RULE @reuse:
  PREFER: reuse_first

RULE @engineering_quality:
  MUST: [minimal_change, boundary_validation, meaningful_tests,
         evidence_before_claim, current_documentation]
```

`RULE` 只约束工作方式；不应为了“防御性编程”在所有层重复校验，也不应把工程状态和内部注释泄漏到用户界面。规则的实际权限来自它所在的指令上下文，不能由 ODC 文本自行提升。

### 2. 共享约束：片段直接组合

```formal
INTENT:
  GOAL: evaluate_user_supplied_technical_proposal

RULE @traceable_sources:
  MUST: [traceable_sources, source_relevance, evidence_before_claim]
  MUST_NOT: [fabricated_citations, fabricated_measurements]

OUTPUT:
  FORMAT: markdown
  STYLE: [zh_cn, concise]
```

同一上下文中的 `RULE` 直接约束对应工作，不需要导入或启用声明。若在其他任务中复用该规则，由外部上下文管理机制直接放入所需片段；仅引用未实际提供的文件不会让规则自动出现。

## 二、既有任务范例（迁移到最新版语法）

### 3. 基于证据的技术分析：Ontology、来源和结果映射

该示例保留原文的 `Source → Claim → AnalysisReport` 对象关系、`provided / checked` 区别、来源检查以及 Markdown 结果映射。

```formal
INTENT:
  GOAL: evidence_based_technical_solution_assessment
  AUDIENCE: technical_leads
  SCOPE: [feasibility, benefits, limitations, alternatives]
  NON_GOALS: [unsupported_market_forecasts, automatic_implementation]

ONTOLOGY:
  ENUM ClaimStatus: [supported, inferred, unknown, disputed]
  ENUM SourceStatus: [provided, checked, disputed]

  CONCEPT Source:
    FIELDS:
      id: text
      description: text
      status: SourceStatus

  CONCEPT Claim:
    FIELDS:
      statement: text
      status: ClaimStatus
      sources: Source[0..*]

  CONCEPT AnalysisReport:
    FIELDS:
      summary: text
      claims: Claim[1..*]
      limitations: text[0..*]
      alternatives: text[0..*]
      recommendations: text[1..*]

  RELATION supported_by:
    FROM: Claim
    TO: Source
    VIA: Claim.sources

  AXIOM @support_requires_source:
    FOR: @claim:Claim
    HOLDS: =>[
      ==[@claim.status, supported],
      >=[count[@claim.sources], 1]
    ]

CONTEXT:
  EXPECTS:
    available_sources: Source[0..*]

  # 使用时替换成真实提供或检索到的 Source；空列表不代表存在已核实资料。
  available_sources: []

  DATA @brief:
    ORIGIN: user_supplied
    TRUST: untrusted
    CONTENT: |
      <在这里放入用户实际提供的技术方案原文>

RULE @within_scope:
  MUST: [source_grounded_analysis, no_unsupported_extrapolation]

RULE @evidence:
  FOR: @claim ∈ @output.claims
  WHEN: ==[@claim.status, supported]
  MUST: ∃[
    @src ∈ @claim.sources,
    ==[@src.status, checked]
  ]
  ON_UNMET: [revise, mark_unknown, disclose]

RULE @source_existence:
  FOR: @claim ∈ @output.claims
  MUST: ∀[
    @src ∈ @claim.sources,
    ∈[@src, @context.available_sources]
  ]
  ON_UNMET: [revise, omit, disclose]

RULE @source_relevance:
  MUST: material_source_support_for_each_claim

RULE @epistemic_integrity:
  MUST: [provided_checked_separation, inference_labeling,
         assumptions_disclosed, unknown_disclosure]
  MUST_NOT: [fabricated_sources, fabricated_test_results,
             unsupported_certainty]

RULE @brevity:
  PREFER: [high_information_density, no_redundant_definitions]

OUTPUT:
  TYPE: AnalysisReport
  FORMAT: markdown
  STYLE: zh_cn
  SECTIONS: [conclusions, evidence, limitations_and_unknowns,
             alternatives, recommendations]
  SECTION_MAP:
    conclusions: @output.summary
    evidence: @output.claims
    limitations_and_unknowns: @output.limitations
    alternatives: @output.alternatives
    recommendations: @output.recommendations

REVIEW:
  CHECK: [@support_requires_source, @evidence, @source_existence,
          @source_relevance, @epistemic_integrity]
  ALSO: [ontology_consistency, provenance, conflicts,
         source_claim_alignment]
  EXPOSE: on_issue
```

**证据边界：** 资料中声称的事实属于 `provided`，不自动等于 `checked`；`supported` 必须有实际核验依据。如果没有验证途径，应将相应主张标记为推论或未知，不能为了通过 `REVIEW` 人为升级来源状态。

### 4. 知识图谱驱动的四选一题目

该示例保留原有数量约束、答案归属公理、知识点关联和学段适配要求。领域属性（如 `knowledge_points`）保持小写，ODC 子句（如 `FIELDS / FOR / MUST`）保持大写。

```formal
INTENT:
  GOAL: single_knowledge_graph_grounded_multiple_choice_question
  AUDIENCE: target_grade_learners
  NON_GOALS: [multiple_questions, unrelated_knowledge_points]

ONTOLOGY:
  ENUM Difficulty: [easy, medium, hard]
  ENUM OptionId: [A, B, C, D]

  CONCEPT KnowledgePoint:
    FIELDS:
      id: text
      name: text
      grade: int
      prerequisites: KnowledgePoint[0..*]

  CONCEPT Option:
    FIELDS:
      id: OptionId
      content: text

  CONCEPT Question:
    FIELDS:
      stem: text
      options: Option[4]
      answer: Option
      explanation: text
      knowledge_points: KnowledgePoint[1..3]
      difficulty: Difficulty

  RELATION depends_on:
    FROM: KnowledgePoint
    TO: KnowledgePoint
    VIA: KnowledgePoint.prerequisites
    ACYCLIC: true

  RELATION assesses:
    FROM: Question
    TO: KnowledgePoint
    VIA: Question.knowledge_points

  AXIOM @answer_is_option:
    FOR: @q:Question
    HOLDS: ∈[@q.answer, @q.options]

  AXIOM @option_ids_unique:
    FOR: @q:Question
    HOLDS: ==[
      count[@q.options],
      count[unique[@q.options.id]]
    ]

CONTEXT:
  EXPECTS:
    input_knowledge_points: KnowledgePoint[1..3]

RULE @knowledge_scope:
  FOR: @kp ∈ @output.knowledge_points
  MUST: ∈[@kp, @context.input_knowledge_points]
  ON_UNMET: [revise, ask, disclose]

RULE @question_quality:
  MUST: [exactly_one_correct_answer, diagnostically_useful_distractors,
         grade_appropriate_stem, grade_appropriate_explanation,
         explain_correct_answer, explain_major_distractor_errors]

RULE @curriculum_integrity:
  MUST_NOT: [fabricated_curriculum_standards,
             fabricated_knowledge_point_ids,
             unsupported_assessment_evidence]

OUTPUT:
  TYPE: Question
  FORMAT: markdown
  STYLE: zh_cn
  SECTIONS: [question_stem, options, correct_answer,
             explanation, knowledge_points, difficulty]
  SECTION_MAP:
    question_stem: @output.stem
    options: @output.options
    correct_answer: @output.answer
    explanation: @output.explanation
    knowledge_points: @output.knowledge_points
    difficulty: @output.difficulty

REVIEW:
  CHECK: [@answer_is_option, @option_ids_unique, @knowledge_scope,
          @question_quality, @curriculum_integrity]
  ALSO: [ontology_consistency, single_answer_validity]
  EXPOSE: on_issue
```

`Question` 描述结果的语义结构，`OUTPUT.FORMAT: markdown` 描述展示形式；二者不冲突。此例并未提供真实知识点，只有实际输入补齐后才能生成题目。

### 5. UI 视觉质量：Requirement + Review + Loop

原示例的获奖级视觉目标、八项检查维度和“观察—比较—修订—复验”循环保留；`REQUIREMENT` 负责质量契约，`LOOP` 负责改进。

```formal
INTENT:
  GOAL: product_appropriate_award_tier_visual_quality
  SUCCESS: no_verified_visible_quality_gaps_remaining

CONTEXT:
  current_page: null
  benchmark_references: [awwwards_winners, webby_awards_winners,
                         fwa_winners]

REQUIREMENT @visual_excellence:
  KIND: quality
  SUBJECT: website
  SCOPE: [typography, whitespace, visual_hierarchy, color,
          motion, micro_interaction, responsive, originality]
  MUST: award_tier_quality_across_applicable_dimensions
  ACCEPTANCE:
    METHOD: [inspect, compare, test]
    GIVEN: [rendered_page_available, relevant_benchmarks_observed]
    EXPECT: [all_applicable_dimensions_reviewed,
             no_unresolved_visible_quality_gaps]

RULE @product_contract:
  MUST: [preserve_product_goals, preserve_brand_identity,
         preserve_required_functionality]

RULE @observation:
  MUST: [inspect_actual_rendered_page, test_relevant_interactions,
         verify_responsive_states, compare_only_observed_references,
         evidence_before_claim]
  ON_UNMET: [mark_unknown, disclose]

LOOP @visual_refinement:
  TRIGGER: on_completion
  DO: [observe_current_page, inspect_each_visual_dimension,
       compare_with_observed_benchmarks, patch_detected_gaps,
       reverify_affected_dimensions]
  UNTIL: &&[all_applicable_dimensions_evidenced,
            no_unresolved_visible_quality_gaps]

OUTPUT:
  FORMAT: markdown
  STYLE: zh_cn
  SECTIONS: [page_implementation, observed_references,
             design_tradeoffs, verification_evidence, unresolved_items]

REVIEW:
  CHECK: [@visual_excellence, @product_contract, @observation]
  ALSO: [acceptance_evidence, responsive_coverage,
         unresolved_visual_gaps]
  EXPOSE: on_issue
```

`benchmark_references` 只列参考方向，不表示已经看过任何获奖作品；没有可渲染页面或实际可用的观察工具时，不能声称完成视觉验收。`LOOP` 不会创建后台任务，也不能证明设计已达数学最优；若无法继续验证，应披露阻塞而非把 `unknown` 当作通过。

## 三、Agent Persona 范例

### 6. 独立 Persona：软件架构顾问 Agent

`IDENTITY / SOUL / ROLE / VOICE` 可独立出现；其中 `SOUL` 表示稳定的判断取向，`ROLE` 表示专业职责，`VOICE` 表示沟通风格。硬约束仍交给 `RULE`。

```formal
IDENTITY:
  NAME: Aster
  KIND: ai_agent
  ARCHETYPE: engineering_thought_partner

SOUL:
  VALUES: [truth_over_agreement, intellectual_honesty,
           user_autonomy, long_term_maintainability]
  MINDSET: [first_principles, systems_thinking,
            counterexample_seeking, evidence_driven]
  TEMPERAMENT: [calm, curious, independent_judgment]

ROLE:
  AS: principal_software_architect
  MISSION: reliable_evolvable_software_systems
  RESPONSIBILITIES: [requirement_analysis, architecture_design,
                     tradeoff_evaluation, implementation_review]
  SCOPE: [software_architecture, agent_systems, developer_experience]
  HANDOFF: [unresolved_product_decisions, authorization_required_actions]

VOICE:
  TONE: [direct, respectful, candid]
  STYLE: [concise_by_default, technical_depth_on_demand,
          explicit_tradeoffs, no_empty_praise]
  LANGUAGE: zh_cn

RULE @professional_integrity:
  MUST: [scope_fidelity, evidence_before_claim,
         meaningful_tests, user_semantics_first]
  MUST_NOT: [invented_credentials, fabricated_test_results,
             unauthorized_external_actions]
```

`IDENTITY` 不表示拥有真实专业资格或人生经历；`ROLE` 也不会产生工具权限。此 Persona 不需要绑定单次任务的 `INTENT`。

### 7. Persona 组合：同一 Soul，不同专业角色

适合多个 Agent 共用原则但分工不同的场景。共同 `SOUL` 和命名 `ROLE` 直接并列，角色通过工作流步骤引用，无需包装层。

```formal
SOUL:
  VALUES: [truth_over_agreement, user_agency, evidence_before_claim]
  MINDSET: [critical_thinking, first_principles]

VOICE:
  TONE: [direct, constructive]

ROLE @researcher_persona:
  AS: research_analyst
  RESPONSIBILITIES: [source_discovery, source_verification,
                     evidence_synthesis]

ROLE @reviewer_persona:
  AS: critical_reviewer
  RESPONSIBILITIES: [requirement_gap_detection,
                     counterexample_search, unsupported_claim_detection]

WORKFLOW @peer_review:
  STEP @investigate:
    BY: @researcher_persona
    DO: source_discovery_and_evidence_synthesis
    OUTPUT: research_findings

  STEP @challenge:
    DEPENDS_ON: @investigate
    BY: @reviewer_persona
    INPUT: @investigate.output
    DO: identify_gaps_and_counterexamples
    OUTPUT: review_findings
```

这里的共同 `SOUL` 是当前上下文中的行为基线，而不是自动向独立 Agent 进程广播的配置。创建多 Agent 实例及分配其上下文由外部运行环境负责，`BY` 只描述职责，不会自行创建 Agent 或授予权限。

## 四、需求定义范例

### 8. 保存功能：功能需求、兼容需求和验收条件

这里把“用户要什么”（`REQUIREMENT`）与“Agent 应怎样修改代码”（`RULE`）分开，并提供可观察的验收标准。

```formal
INTENT:
  GOAL: fix_editor_save_consistency
  NON_GOALS: [editor_redesign, new_sync_service]

ONTOLOGY:
  CONCEPT Document:
    FIELDS:
      content: text

REQUIREMENT @save_latest:
  KIND: behavior
  SUBJECT: Document
  MUST: save_current_state
  ACCEPTANCE:
    METHOD: test
    GIVEN: unsaved_content_changes
    WHEN: save_close_reopen
    EXPECT: reopened_content_matches_save_time_state
  SOURCE: @request
  EPISTEMIC: provided

REQUIREMENT @compatibility:
  KIND: compatibility
  SUBJECT: Document
  MUST: preserve_existing
  ACCEPTANCE:
    METHOD: test
    EXPECT: [open_behavior_preserved, undo_behavior_preserved,
             export_behavior_preserved]
  SOURCE: @request
  DERIVED_FROM: @save_latest
  EPISTEMIC: inferred

RULE @implementation:
  MUST: [minimal_change, reuse_first, meaningful_tests,
         observational_refinement, evidence_before_claim]

OUTPUT:
  FORMAT: markdown
  SECTIONS: [change_summary, acceptance_evidence, open_issues]

REVIEW:
  CHECK: [@save_latest, @compatibility]
  ALSO: [requirement_coverage, acceptance_evidence,
         semantic_consistency, conflicts]
  EXPOSE: on_issue
```

`EPISTEMIC: inferred` 表明兼容需求在此范例中属于合理推导，并非用户已逐项确认；真实任务应根据实际来源调整。`ACCEPTANCE` 不是测试结果，未运行测试就不能宣告通过。

### 9. 有状态的系统：Safety / Liveness 与 Transition

时序要求用于描述长期运行行为，不用于普通一次性文案任务。这里采用 `always / eventually`，不使用易产生跨学科歧义的模态逻辑符号。

```formal
ONTOLOGY:
  ENUM TaskStatus: [queued, running, succeeded, failed, cancelled]
  CONCEPT Task:
    FIELDS:
      status: TaskStatus
      owner: text
    INITIAL:
      status: queued

TRANSITION @claim:
  SUBJECT: Task.status
  FROM: queued
  TO: running
  ON: worker_claim

TRANSITION @resolve:
  SUBJECT: Task.status
  FROM: running
  TO: [succeeded, failed]
  ON: execution_resolved

TRANSITION @cancel:
  SUBJECT: Task.status
  FROM: [queued, running]
  TO: cancelled
  ON: cancellation_accepted

REQUIREMENT @terminal_safety:
  KIND: safety
  SUBJECT: Task
  MUST: always[terminal_state_irreversible]
  ACCEPTANCE: no_terminal_reentry

REQUIREMENT @eventual_resolution:
  KIND: liveness
  SUBJECT: Task
  ASSUME: [fair_scheduling, available_workers,
           no_permanent_blocker]
  MUST: always[=>[task_accepted, eventually[terminal_state]]]
  ACCEPTANCE: eventual_terminal_resolution_under_assumptions

REVIEW:
  CHECK: [@terminal_safety, @eventual_resolution]
  ALSO: [transition_coverage, counterexample_search,
         acceptance_evidence]
```

这里 `always` 表示每个被考虑的状态都应满足性质，`eventually` 表示当前或未来某状态达到性质，不代表有确定的时间期限；若需要超时 SLA，应单独声明。文本并没有执行 TLA+ 模型检查。

## 五、工作流范例

### 10. 简单工作流：顺序工作无需 STEP

```formal
WORKFLOW @bug_fix:
  DO: [reproduce_issue, identify_root_cause,
       implement_minimal_fix, run_relevant_tests,
       inspect_resulting_diff, report_verified_outcome]
```

列表按顺序完成一次，不是循环。仅有顺序操作时，不必人为拆成多个节点。

### 11. DAG + 多 Agent：题目生成、并行质检与返修

这是完整的多 Agent 工作流范例：`ROLE` 定义职责，`STEP.BY` 引用角色，`DEPENDS_ON` 声明 DAG，`LOOP` 处理真实质量差距，`OUTPUT` 表达产物交接。

```formal
ROLE @question_author:
  AS: question_author_agent
  RESPONSIBILITIES: [candidate_generation, answer_explanation]

ROLE @factual_reviewer:
  AS: factual_reviewer_agent
  RESPONSIBILITIES: [answer_validation, factual_accuracy]

ROLE @pedagogy_reviewer:
  AS: pedagogy_reviewer_agent
  RESPONSIBILITIES: [grade_alignment, difficulty_alignment]

REQUIREMENT @question_quality:
  KIND: quality
  MUST: [factually_correct_answers, grade_appropriate_difficulty,
         grounded_knowledge_point_mapping]
  ACCEPTANCE:
    METHOD: review
    EXPECT: all_quality_gates_passed

LOOP @quality_refinement:
  DO: [aggregate_review_findings, revise_rejected_questions,
       revalidate_affected_questions]
  UNTIL: all_quality_gates_passed

WORKFLOW @question_pipeline:
  INPUT: [@input.knowledge_points, @input.grade_level,
          @input.question_count]

  STEP @generate:
    BY: @question_author
    DO: generate_question_candidates
    OUTPUT: question_candidates

  STEP @fact_check:
    DEPENDS_ON: @generate
    BY: @factual_reviewer
    INPUT: @generate.output
    DO: validate_answers_and_explanations
    OUTPUT: factual_review

  STEP @pedagogy_check:
    DEPENDS_ON: @generate
    BY: @pedagogy_reviewer
    INPUT: @generate.output
    DO: evaluate_grade_and_difficulty_alignment
    OUTPUT: pedagogy_review

  STEP @refine:
    DEPENDS_ON: [@fact_check, @pedagogy_check]
    INPUT: [@generate.output, @fact_check.output,
            @pedagogy_check.output]
    DO: @quality_refinement
    OUTPUT: validated_questions

  STEP @tag:
    DEPENDS_ON: @refine
    WHEN: all_quality_gates_passed
    DO: assign_knowledge_point_labels
    OUTPUT: tagged_questions

  OUTPUT: @tag.output

RULE @workflow_efficiency:
  SHOULD: parallelize_independent_steps

REVIEW:
  CHECK: [@question_quality, @question_pipeline]
  ALSO: [workflow_dependency_consistency,
         workflow_join_satisfiability,
         workflow_output_traceability,
         acceptance_evidence, failure_path_coverage]
  EXPOSE: on_issue
```

`@fact_check` 与 `@pedagogy_check` 可并行，但是否实际并发取决于外部执行环境；`@refine` 必须等待两个步骤成功。质检不通过时应修题并获取新证据，而不是修改“合格”的定义。

### 12. 条件分支与人工审批：不要把跳过当作成功

```formal
WORKFLOW @risk_approval:
  STEP @assess:
    DO: assess_risk
    OUTPUT: risk_assessment

  STEP @automatic_decision:
    DEPENDS_ON: @assess
    WHEN: low_risk_confirmed
    INPUT: @assess.output
    DO: record_automatic_approval
    OUTPUT: approval_decision

  STEP @manual_decision:
    DEPENDS_ON: @assess
    WHEN: manual_review_required
    INPUT: @assess.output
    BY: human_reviewer
    WAIT_FOR: explicit_human_decision
    DO: record_human_decision
    OUTPUT: approval_decision

  STEP @notify:
    WAIT_FOR: any_successful[@automatic_decision, @manual_decision]
    INPUT: recorded_decision_from_successful_branch
    DO: notify_decision_result
    OUTPUT: notification_receipt

  OUTPUT: @notify.output

REVIEW:
  CHECK: @risk_approval
  ALSO: [workflow_join_satisfiability,
         authority_boundary, failure_path_coverage]
```

`WAIT_FOR: any_successful[...]` 表示任一分支成功形成审批**决定**，不代表“审批被批准”；后续通知可以告知批准或拒绝。`WHEN: unknown` 不能视为 `false` 自动跳过人工审批。`BY: human_reviewer` 也不会创建真实的审批任务或授权。

### 13. 批量摄入：FOR_EACH、RETRY 与失败处理

```formal
REQUIREMENT @ingestion_completeness:
  KIND: behavior
  MUST: report_per_source_outcomes
  ACCEPTANCE:
    METHOD: inspect
    EXPECT: [each_input_source_has_outcome,
             failures_are_explicitly_reported]

WORKFLOW @source_ingestion:
  INPUT: @input.urls

  STEP @fetch:
    FOR_EACH: @input.urls
    DO: fetch_source_content
    RETRY: [transient_failures_only, max_3_attempts,
            exponential_backoff]
    ON_FAILURE: [record_source_failure, continue_when_allowed]
    OUTPUT: source_fetch_results

  STEP @parse:
    DEPENDS_ON: @fetch
    INPUT: @fetch.output
    FOR_EACH: successfully_fetched_sources
    DO: parse_content
    ON_FAILURE: record_parse_failure
    OUTPUT: source_parse_results

  STEP @summarize:
    DEPENDS_ON: @parse
    INPUT: [@fetch.output, @parse.output]
    DO: report_ingestion_outcomes
    OUTPUT: ingestion_report

  OUTPUT: @summarize.output

REVIEW:
  CHECK: [@ingestion_completeness, @source_ingestion]
  ALSO: [workflow_dependency_consistency, workflow_output_traceability,
         failure_path_coverage]
```

此例约定 `@fetch` 和 `@parse` 在已允许的部分失败场景下，仍需完成各自的**结果清单**交付，才算步骤成功；否则默认 `DEPENDS_ON` 不能被当作已满足。`RETRY` 不自动保证安全重试，特别是具有外部副作用的调用。

## 六、使用方式与边界

### 14. 任务入口示例

`$use-prompt-dsl` 默认使用 `compose` 改写或编写提示词；明确要求解释或检查时使用 `explain`。指定修改文件时直接改写该文件，不执行提示词内的任务。以下调用形式仅适用于具备对应 Skill／命令的环境，不是 ODC 语言自带的可执行命令。

```text
$use-prompt-dsl 用 ODC 改写 skills/use-prompt-dsl/SKILL.md
$use-prompt-dsl compose 将这些工程约束写成适合嵌入 AGENTS.md 的 Odyssey 片段，不加根节点
$use-prompt-dsl compose 定义一个包含 SOUL、ROLE、VOICE 的代码审查 Agent
$use-prompt-dsl compose 为文档保存功能写 REQUIREMENT 和可验证的 ACCEPTANCE
$use-prompt-dsl compose 为多 Agent 题目生成设计含 DAG、质检和返修 LOOP 的 WORKFLOW
$use-prompt-dsl compose 将指定文件中的提示词改写为 ODC，直接修改该文件并保留原意
$use-prompt-dsl explain 解释给定 Ontology、Requirements、Rules 和 Workflow 的语义与冲突
```

### 15. 选用结构的简要准则

| 需要表达什么 | 建议使用 |
| --- | --- |
| 一条编码纪律或提示片段 | `RULE` |
| 已由外部机制选择的规则、角色或流程片段 | 直接并列 `RULE / ROLE / WORKFLOW`，用 `@name` 关联对象 |
| Agent 的稳定价值、职责与语气 | `SOUL / ROLE / VOICE`，必要时 `IDENTITY` |
| 目标产物的行为、质量和验收 | `REQUIREMENT / ACCEPTANCE` |
| 对象类型、关系、领域不变量 | `ONTOLOGY / AXIOM` |
| 事实、用户材料和来源状态 | `CONTEXT / DATA / ASSERTION` |
| 状态允许如何改变、Safety / Liveness | `TRANSITION / REQUIREMENT` |
| 一次性的步骤顺序、DAG、交接与协作 | `WORKFLOW / STEP` |
| 对未达标结果反复修订 | `LOOP` |
| 验收证据与一致性检查 | `OUTPUT / REVIEW` |

**总原则：** Self-descriptive Literal First, Natural Language When Necessary。`formal` 表示便于 LLM 理解的形式化表述，而不是需要 Parser 的可执行语言；Persona 不授予权限，Requirement 不证明自身已满足，Workflow 不自动调度，Review 不替代真实检查。
