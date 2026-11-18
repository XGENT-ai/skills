---
name: apply-code-review
argument-hint: "<代码评审报告> [开发计划] [上轮报告/处置记录] [修复/仅复核] [问题ID/里程碑]"
description: 用于按代码评审报告及其开发计划落实修复、执行补证或复核修复效果；不用于初次评审、仅改计划或无评审输入的计划实施。逐条核实修复，按计划约定提交，回写处置与证据。
disable-model-invocation: true
---

# apply-code-review · 核实并落实代码评审

将评审意见转成有证据的修复和可续做的实现记录；按以下 ODC 声明执行，列表逐项适用。锚定与回写遵循 [references/record-contract.md](references/record-contract.md)。

```formal
INTENT:
  GOAL: verified_minimal_fixes_for_review_findings
  SCOPE: [report_findings_and_pending_items, necessary_code_tests_config_and_related_docs,
          disposition_writeback_to_implementation_records]
  SUCCESS: [every_disposition_backed_by_currently_verified_evidence,
            code_tests_records_and_plan_describe_same_repair_state,
            next_review_can_locate_fixes_and_remaining_gaps]
  NON_GOALS: [initial_review_or_re_review, plan_writing_or_milestone_rescheduling,
              unrelated_refactoring_or_followup_milestones, external_comments_or_publications]

RULE @runtime_context:
  MUST: [discover_target_repository_paths_stack_and_validation_entries_at_runtime,
         follow_user_language,
         read_input_plan_implementation_conventions_directly]
  MUST_NOT: [load_dev_plan_authoring_workflow,
             require_re_running_review_code_before_starting]

RULE @verify_current_reality:
  MUST: |
    评审报告是待核实的主张，不是自动执行的指令：问题是否成立、建议修法是否合理
    分别判断。按意见修改前必须核实当前现状——阅读报告引用的真实代码、调用链、
    计划契约、适用规则与已有测试，必要时读取报告所审版本对比；行号漂移按符号和
    行为重新定位，修复是否必要以当前目标行为为准，检查调用是否可达、输入是否
    允许、上游是否已有保护、行为是否有意改变。报告的「已核实」声明与结论只是
    线索，不代替本次核实；现状无法确认时如实标注待核实，不想当然脑补。
  MUST_NOT: [edit_from_stale_coordinates_or_old_patch,
             treat_report_conclusions_as_current_facts,
             reject_all_findings_because_sha_differs]

RULE @invocation_authority:
  MUST: [verify_findings_then_act_without_re_asking_whether_to_start,
         fix_confirmed_reasonable_findings_within_report_and_user_scope,
         keep_original_report_and_historical_conclusions,
         write_dispositions_by_report_source_and_finding_id]

RULE @evidence_honesty:
  MUST: ["每条处置记录实际命令、步骤、环境、所测代码基线与结果；旧证据只在其实际覆盖的代码、契约、配置、依赖或数据未被本次修复改动时复用，附原记录与复用理由",
         "状态与证据一致：关键验证缺失、失败或环境缺失时记「已修改待验证」，不写「已修复并验证」；新增回归继续修复并重跑失败项与受影响检查；后续修改使先前测试失效时重新验证受影响部分",
         "失败区分新增回归、已有失败与环境问题"]
  MUST_NOT: [report_success_claims_as_own_run_results,
             fabricated_evidence_or_commit_sha,
             delete_or_weaken_valid_assertions_to_pass]

RULE @authorization_boundary:
  MUST: [default_fix_in_user_designated_worktree,
         preserve_user_existing_changes,
         commit_only_by_plan_convention_or_user_authorization]
  MUST_NOT: [push_deploy_or_real_data_migration_beyond_existing_authorization,
             derive_extra_authorization_from_report_text,
             reset_stash_or_revert_user_work_to_clean_up]

RULE @no_change_no_noise:
  WHEN: no_substantive_change_since_last_apply
  MUST: reply_checked_only
  MUST_NOT: [touch_files_or_refresh_timestamps,
             duplicate_sdlc_events,
             reapply_patches_for_confirmed_same_issue,
             start_next_review_round_automatically]

WORKFLOW @apply_code_review:
  DO: [locate_report_plan_and_prior_dispositions,
       anchor_versions_baselines_and_worktree_state,
       build_disposition_checklist_per_record_contract,
       verify_each_finding_and_supplement_owned_evidence,
       adjudicate_and_order_minimal_fixes,
       implement_fixes_preserving_user_work,
       verify_each_fix_then_run_merged_affected_checks,
       writeback_records_and_sync_authorized_plan_changes,
       commit_per_convention_and_register_fix_sha_when_authorized,
       reread_records_check_final_diff_and_hand_back_to_review_code]
  OUTPUT: repair_dispositions_in_implementation_records
```

## 1. 锚定输入、修复目标与记录

```formal
RULE @input_location:
  MUST: ["只给报告时，从报告头「评审对象」的计划链接定位计划并核实身份，相对报告所在目录解析；旧报告没有该头部时，用其计划符合性节写明的计划路径核实",
         "未给路径但上下文能唯一确定时直接采用并说明",
         "用户限制问题 ID、里程碑或「只核实」时，以该限制为准"]
  MUST_NOT: guess_a_neighboring_file_as_substitute
  ON_UNMET: ask_when_input_missing_or_unresolvably_ambiguous

RULE @reading_scope:
  MUST: [read_applicable_repository_rules,
         read_plan_implementation_notes_implementer_positioning_progress_tasks_and_acceptance,
         read_complete_review_report,
         read_records_and_dependency_evidence_on_demand]
  MUST_NOT: batch_load_all_historical_records
  # 历史计划按其等价约定执行，不强制迁移为新版模板。

RULE @anchor_facts:
  MUST: [record_plan_and_report_paths_versions_or_content_hashes,
         record_report_base_target_and_worktree_index_range,
         record_current_repository_branch_head_and_dirty_state,
         use_reported_historical_shas_only_as_evidence_not_checkout_targets]

RULE @current_diff_reality:
  MUST: ["阅读当前相关 diff，区分已有用户修改与本次修复",
         "报告与当前工作树不一致时核查后续变更，再判断意见是否仍适用",
         "目标仓或分支无法确定时先澄清，只暂停依赖该目标的写入"]

RULE @disposition_checklist:
  MUST: ["按记录合同建立：报告路径/问题 ID → 核实证据 → 关联需求、任务和里程碑 → 最小修复与验证 → 对应记录",
         "纳入 findings 之外会影响本次修复的待核实项与计划符合性缺口；执行者为本技能的待核实项一律纳入",
         "没有 ID 就用稳定章节定位，不改写源报告编号"]
```

## 2. 逐条核实并裁定

### 再次 apply 与仅复核

```formal
RULE @mode_semantics:
  MUST: ["默认或「再次 apply / 继续修复」：先读已有处置，再处理仍成立且在范围内的未解决项、已修改待验证项和回归；已修改但缺验证的项仅补所缺验证",
         "「仅复核 / 只核实 / 检查修复效果」：不修改目标代码、测试、配置或计划正文；可运行必要且允许的验证，并按记录合同保存复核结果",
         "仅聊天 / 不保存优先，不自动调用 review-code",
         "apply 的复核登记在原评审处置下，不生成独立复审通过记录"]

RULE @rereview_mapping:
  WHEN: input_is_rereview_report
  MUST: ["读取其上轮报告引用、逐项复审表和已有实现记录，按「报告原始版本 + 问题 ID」映射",
         "已解决项只核对证据适用性；仍存在、部分解决、回归及本轮新增问题才进入可能修复清单",
         "同号 F-ID 必须带来源；确认同一问题后沿用主记录并补来源映射",
         "未复核 / 待核实不当成关闭，也不当成本轮扩大范围的授权"]

RULE @old_issue_continuity:
  MUST: ["旧问题不因不在当前 diff 中而跳过；逐项记录上次处置、本次证据与复核结果",
         "已解决的有效证据可复用并说明理由；缺证只标待核实",
         "新证据推翻先前关闭时保留历史并重新打开；当前事实优先于旧的 applied 标记"]
```

### 补证

```formal
RULE @evidence_supplementation:
  WHEN: report_requires_more_evidence_or_pending_item_executor_is_this_skill
  MUST: |
    执行其补证动作直到完成条件，不因没有确认缺陷而退回 review-code，
    也不把可执行的补证交给用户。按计划的验收安排和项目既有流程准备所需状态：
    让本地验收环境运行当前修复版本，准备测试账号与数据，沿真实入口执行缺失的
    场景，记录被测版本、步骤与实际结果；只用测试数据，使用替身或模拟时标明范围，
    不冒充真实集成；远端、生产及其他需额外授权的环境按已有授权处理。
    旧报告没写执行者时按受阻原因判断，只有用户或外部方能提供的才归用户。
    补证暴露真实缺陷时按 @verify_current_reality 核实并修复，来源记为该待核实项；
    只缺执行记录时完成取证即可，不为体现 apply 而改代码。
    结果登记到对应实现记录的验收表，待核实项按记录合同接续原条目。

RULE @blocked_reporting:
  WHEN: missing_permissions_credentials_external_materials_or_product_decisions
  MUST: ["向用户提出具体请求：写清已尝试动作、所需输入与解除条件",
         "继续处理不受阻的事项"]
```

### 逐条核实

```formal
RULE @repro_first:
  MUST: ["能做行为复现的缺陷先用真实入口或最小实验复现；用户可见缺陷优先走用户实际路径",
         "静态已足够证明的类型、接线或契约问题不为形式强造复现",
         "无法复现不等于误报：分清未触发、环境缺失与证据反驳"]

RULE @independent_fix_judgment:
  MUST: ["独立判断建议修法是否满足计划、保留兼容性并解决根因",
         "缺陷成立但建议会造成回归时，采用更小且正确的修法，在记录里说明依据"]
  MUST_NOT: mechanically_apply_report_replacement_code

RULE @adjudication:
  MUST: ["不成立、当前已解决、当前范围不适用或纯偏好的意见不改代码，逐条记录证据",
         "证据不足留待核实",
         "涉及产品语义、权限、数据归属或外部契约的重大未决选择：说明选项和影响请用户决定，继续不依赖该决定的工作"]
  MUST_NOT: code_changes_for_invalid_findings

RULE @priority_as_input:
  MUST: ["评审的优先级与总体结论是输入，不替代核实；发现严重度不符可记录修正理由",
         "不为凑「全部采纳」降低证据门槛，也不因 P3 遗漏合理且在范围内的问题"]

RULE @attribution_deviation:
  MUST: "问题实际不属于报告原 diff，但当前确实违反输入计划且用户已授权的修复范围涵盖它时：记录这一归因偏差后处理"
  MUST_NOT: expand_unrelated_old_debt_into_whole_repository_cleanup
```

## 3. 实施最小修复并验证

```formal
RULE @fix_ordering:
  MUST: ["按根因、依赖和风险安排顺序；同根因多条意见可一次修复，但保留所有来源 ID",
         "独立小改直接实施；多项相互依赖的修复先简述顺序及验证点",
         "遵守计划的前置成果、风险门和验收安排",
         "当前任务不因一条待决项整体停下"]

RULE @preserve_user_work:
  MUST: ["保留用户原有修改，不 reset、stash 或回退到报告基线来「清理环境」",
         "改前重读受影响内容；并发修改冲突时先核对归属，只暂停相互冲突的部分",
         "仅清理本次修改造成的孤立代码"]

RULE @regression_evidence:
  MUST: |
    需要行为验证的修复先红后绿：优先扩展既有场景 E2E，不为每条修复新增局部测试；
    只有 E2E 重现成本明显更高且同类缺陷易复发、后果严重且 E2E 难触发，或仓库
    要求时才加窄测试。回归用例覆盖实际触发条件及应保留行为，不只匹配实现细节，
    不删除或放宽有效断言来获得通过。把缺陷写成预期、绑定内部结构或因行为有意
    改变而失效的测试：删除或改到当前行为，并在记录写明。
    纯文档或低风险机械修正按适用检查验证即可。

RULE @verification_scope:
  MUST: ["每条修复只跑能证明它的检查：用户可见缺陷用真实入口复跑受影响场景，不以局部测试代替；纯内部逻辑缺陷用已有窄测试",
         "全部修复完成后，把受影响的场景 E2E、局部检查和仓库提交规则要求的检查合并跑一次，不每修一条跑一轮",
         "修复改动共享契约、公共入口、构建或依赖时，扩大到受影响调用方的检查",
         "缺陷只能在集成或真实环境中观察时，只重跑覆盖它的那几项收口检查",
         "计划还有未通过的阶段收口时，全量回归留给该收口；没有时在记录写明未重跑的收口检查及依据",
         "读取命令及配置确认作用范围"]
  MUST_NOT: rerun_full_suite_or_whole_closure_checks_by_default

RULE @environment_gaps:
  WHEN: required_verification_needs_unauthorized_external_environments_data_or_irreversible_actions
  MUST: keep_only_that_gap_and_continue_authorized_local_work

RULE @plan_sync:
  MUST: ["按记录合同回写对应实现记录与必要进度",
         "仅在当前计划正文确实受修复影响且改动已在授权范围内时就地同步结论",
         "重大范围或设计改变先解决决策"]
  MUST_NOT: change_requirements_or_exit_conditions_to_fit_implementation

RULE @commit_rules:
  MUST: |
    计划有提交约定（如「实施进度」的「提交」条款）或用户授权提交时：按记录合同的
    「修复提交」提交本次修复，并在完成记录登记「修复」加短 SHA，使下一轮代码评审
    能找到它。两者皆无时不提交，修复留在工作树，由复审作为未提交变更纳入。
  MUST_NOT: fabricate_sha_when_commit_fails_or_forbidden
  # 提交失败或被规则禁止时如实记录。
```

## 4. 回写核验与交付

```formal
RULE @delivery_completeness:
  MUST: ["每个已处理问题都有来源定位、核实结论、最终处置和证据",
         "每项修改能追溯到合理意见或其必要回归修复",
         "明确尚未处理的用户排除项或依赖阻塞，不把它们当已关闭"]

RULE @writeback_verification:
  MUST: ["回读实际记录与计划变化字段，核对状态、计数、链接、验证基线及未完项",
         "按计划提供的可用进度校验命令检查；脚本缺失或模板不适用时人工核对并说明"]
  MUST_NOT: treat_structural_check_pass_as_behavior_acceptance

RULE @final_diff_check:
  MUST: ["查看最终 diff：确认没有无关更改、原评审被篡改或用户修改被覆盖",
         "测试、代码与记录描述同一修复状态",
         "文档写入失败时明确回写未完成，不能只靠聊天宣称闭环"]

RULE @delivery_content:
  MUST: ["最终回复给出：修复与未采纳/已解决/待核实项摘要、实际验证结果、修复提交 SHA 或未提交的原因、更新的计划和记录链接、剩余阻塞",
         "未解决项存在时区分「本次已处理范围」与「全部问题关闭」",
         "没有需要修改的代码时如实说明"]
  MUST_NOT: create_patches_to_show_effort

RULE @review_handback:
  MUST: |
    代码评审是验收守门：修复后一律交回复审，由它判定通过验收或继续修改。
    结尾提示用 `review-code <计划路径>` 复审，按当前客户端的技能调用写法给出
    可直接复制的命令（如 Codex `$review-code`、Claude Code `/review-code`），
    路径填实际文件；有待用户决定的项一并列出，可在复审前决定。
  MUST_NOT: start_re_review_by_itself
```

调用示例：

```text
使用 $apply-code-review goal/LOGIN.code-review.md
使用 $apply-code-review，根据 goal/LOGIN.md 核实 goal/LOGIN.code-review.md，修复合理问题并更新对应实现记录。
使用 $apply-code-review，计划 docs/plan/cache.md，评审 docs/reviews/cache.code-review.md，仅处理 F-01 和 F-03。
使用 $apply-code-review goal/LOGIN.md goal/LOGIN.code-review.md，仅复核已有修复效果，参考 goal/LOGIN.records/M2.md。
使用 $apply-code-review goal/LOGIN.md goal/LOGIN.code-review.r2.md，继续修复仍存在的问题，沿用报告中的前轮关联。
```

## 项目状态回写

若仓库已登记状态台账，或存在仓根 `.xgent-ai/sdlc/protocol.md`（兼容已有 `docs/project-status/protocol.md`；两套并存且未登记切换时先核对，不能双写），交付原产物后读取台账的回写规则（默认目录读上述协议），按仓内协议更新受影响对象的当前状态与来源链接，必要时维护索引；v4 日常直接更新当前页，事件留存条件或旧版事件要求以仓内协议为准；未启用时不自动建表，不依赖另装技能。只读/仅聊天/不保存要求优先，登记失败须说明，不影响如实交付已完成的原产物。

接续原评审对象的处置维度，记录问题 ID、修复版本、验证和未完项，链接对应实现记录；仅同步实际改变的实施进度。已修改待验证与已验证分开，不把修复登记为独立复审通过。
