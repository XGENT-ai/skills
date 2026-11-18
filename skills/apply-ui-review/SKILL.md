---
name: apply-ui-review
argument-hint: "<UI验收报告> [开发计划] [问题ID/单元] [修复方向/用户决定]"
description: 用于执行 UI 验收报告中的修复、补证与解阻；不用于初次 UI 评审、独立评分、代码评审处置或无报告的界面开发。核实差距、修复界面、补齐环境与实测材料，同状态共用证据，记录后交回复审。
disable-model-invocation: true
---

# apply-ui-review · 落实 UI 验收评审

把独立评审给出的差距清单变成真实的界面改进；按以下 ODC 声明执行，列表逐项适用。记录位置与每轮内容遵循 [references/record-contract.md](references/record-contract.md)。启动方式、组件库、设计规范和检查命令从目标仓发现，语言跟随用户。本技能是独立的 SDLC 执行流程：依据报告、目标包和修复记录续接。

```formal
INTENT:
  GOAL: real_interface_improvements_closing_acceptance_gaps
  SCOPE: [report_gaps_and_missing_evidence_list, necessary_interface_code_styles_assets_and_tests,
          repair_records_and_apply_captures]
  SUCCESS: [gaps_verified_on_running_interface_before_action,
            blocking_items_and_shared_root_causes_prioritized,
            records_captures_and_evidence_ready_for_independent_review]
  NON_GOALS: [self_scoring_or_acceptance_verdicts, initial_review_or_re_review,
              editing_reports_captures_or_judge_md, new_visual_direction]

RULE @runtime_context:
  MUST: [discover_target_startup_component_library_design_rules_and_check_commands_at_runtime,
         follow_user_language,
         resolve_references_relative_to_this_file]

RULE @verify_on_running_interface:
  MUST: |
    评审报告和评审截图是线索，不是免核实的结论：每条差距先对照截图、目标包的
    规范要点、比较口径和 DESIGN.md，在运行中的真实界面复现核实；来源为「盲测
    走查」的差距不是看图得出的，先按走查报告的状态文件或路径复走确认。
    判断评审看错、差距已不存在或修复生效，依据必须是当前有效的实拍截图、页面
    实测值和实际操作结果；不能读了代码就下结论。同一页面、状态、视口、主题、
    语言、数据与版本的多个问题共用一图并注明位置，不按问题编号分别截图；
    已有证据经版本、条件、目标和影响范围核验仍有效时直接引用，历史证据不冒充
    新采集。现状确认不了就如实标注，不想当然脑补。
  MUST_NOT: [judge_from_code_reading_alone,
             act_on_report_claims_without_current_verification]

RULE @invocation_authority:
  MUST: ["调用即在报告与项目范围内授权：核实、修复、补证、解除可自行处理的阻塞、运行必要的检查、按计划约定提交并回写记录",
         "评审看过目标和截图，可修复的差距默认全部处理",
         "没有代码可改不等于没有工作：报告中的缺证清单也是执行输入"]

RULE @no_self_scoring:
  MUST: [record_only_actually_changed_gaps,
         disposition_at_most_modified_pending_review]
  MUST_NOT: [self_declared_scores_or_acceptance_claims,
             mark_fixed_and_verified,
             count_untried_gaps_as_attempts,
             fake_gap_ids_for_evidence_only_runs]
  # 修复是否到位由下一轮 review-ui 的独立评审判定；只有真正改过的才记「已修改待评审」，
  # 下一轮的顽固差距判定以此为准；重复实际修改仍无效时由复审转为停滞，不无限微调。

RULE @protected_artifacts:
  MUST: [write_user_decisions_into_target_package_only_after_explicit_confirmation]
  MUST_NOT: [edit_review_reports_captures_or_judge_md,
             push_deploy_or_other_actions_beyond_existing_authorization]
  # 经确认写入目标包时注明日期和用户原话，并追加变更记录。

RULE @tool_independence:
  MUST: [continue_from_report_target_package_and_repair_records]
  MUST_NOT: [bind_workflow_to_tool_versions_rule_versions_or_skill_document_hashes,
             block_verification_repair_or_supplementation_due_to_absent_or_incompatible_tools]

RULE @no_change_no_noise:
  WHEN: blocked_condition_unchanged_and_no_new_changes_or_evidence
  MUST: update_original_record_with_unblock_conditions
  MUST_NOT: [create_new_rounds_or_duplicate_captures,
             recommend_rerunning_review_or_apply_under_same_conditions]

WORKFLOW @apply_ui_review:
  DO: [locate_report_target_package_and_repair_history,
       route_by_verdict_and_anchor_head_and_dirty_state,
       supplement_evidence_and_unblock_within_scope,
       verify_gaps_grouped_by_page_state_on_running_interface,
       order_and_implement_fixes_at_right_layer,
       self_check_on_running_interface_and_run_relevant_checks,
       update_this_round_record_section_per_contract,
       commit_per_convention_and_register_ui_fix_sha_when_authorized,
       reread_records_check_final_diff_and_hand_back_to_review_ui]
  OUTPUT: repair_records_and_apply_captures
```

## 1. 锚定输入

```formal
RULE @input_location:
  MUST: ["定位用户给的 UI 验收报告，从报告头的「评审对象」找到计划、目标包、截图目录和上轮报告，并核实它们确实对应",
         "用户没给报告时，从上下文或计划同目录取轮次最大的 .ui-review 报告",
         "有多份无法判断时询问"]
  MUST_NOT: choose_report_by_file_mtime_or_guess_round

RULE @verdict_routing:
  MUST: ["需要修改：处理可修复的差距；带「停滞预警」按下文诊断原因，不强制重构；带「待裁定」的先不改、记「待决定」，除非用户本次已就它给出方向",
         "需补证：本技能执行报告中的补证工作，并处理已评单元的可修复差距；不因缺证或没有确认缺陷而直接退回 review-ui，也不默认让用户准备环境或采集证据",
         "需用户裁定：用户本次没给方向时，转述报告里的选项请用户决定，不动停滞和待裁定的部分；给了方向就照做，并把用户原话记下来",
         "通过验收 / 用户豁免通过：只有用户要继续打磨时，才处理剩余差距"]
  MUST_NOT: hand_executable_supplementation_back_to_review_ui_or_user

RULE @user_accepted_deviations:
  MUST: ["用户接受某处偏差或修改期望时，经明确确认后写进目标包对应单元的比较口径或规范要点，注明日期和用户原话，并追加变更记录；下一轮评审据此重建基线",
         "用户接受整体界面时，提示由 review-ui 记为豁免通过，本技能不改报告"]

RULE @reading_scope:
  MUST: ["读目标包、报告中的差距清单、待裁定事项和上轮复核表，以及以往各轮的修复记录（见记录合同）",
         "记下当前 HEAD 和未提交改动，分清用户已有的修改与本次修复",
         "用户限定了差距编号或单元时，只处理这些"]
```

## 补证与解阻

```formal
RULE @evidence_supplementation:
  MUST: |
    读取报告、采集清单及缺失状态的原始记录，逐项确定缺什么、执行动作和完成条件；
    能在当前权限和项目范围内完成的立即做，不把清单原样交给用户。依项目既有流程
    启动或恢复测试环境，准备测试账号与种子数据，复用已有 mock/拦截方法到达目标
    状态，执行缺失交互并实测，必要视觉转折才截图；明确模拟范围，不把替身结果
    冒充真实集成；只用测试数据，不修改真实业务数据、不绕过权限或扩大产品范围。
    缺目标时先找已有规范及已确认设计；找不到或互相冲突才请求决定，不自造期望。
    补证时发现真实产品缺陷，核实后在授权范围内修复并自检；仅缺截图或执行记录时
    完成采集即可，不为体现 apply 而改代码；待裁定项可先补事实，但不替用户决定
    期望或修复方向。

RULE @capture_rules:
  MUST: ["补采文件放 <前缀>.ui-captures/r<n>-apply/，记录被测版本、目标指纹、单元/状态/视口/主题、操作、实际结果和文件路径",
         "保留每次必要采集，不按问题复制同状态图片；临时服务用完清理，截图与日志遵循项目忽略约定"]
  MUST_NOT: overwrite_review_captures_reports_or_judge_md

RULE @supplementation_record:
  MUST: ["在原修复记录位置的本轮补证小节记录补证动作、结果与剩余阻塞",
         "补证材料是交接输入，不是独立验收结论：记录被测版本、条件与出处，供 review-ui 核对后直接引用有效材料，仅失效或看不清时由它补采并完成独立评分"]
  MUST_NOT: [count_evidence_only_runs_as_fix_attempts_or_refactor_rounds,
             present_apply_captures_as_next_round_new_collection]

RULE @blocked_reporting:
  WHEN: missing_permissions_materials_or_product_decisions_beyond_own_reach
  MUST: ["向用户提出具体请求：写清已尝试动作、所需输入与解除条件",
         "继续不受阻的事项；保留原报告、材料和修复历史，不重置记录"]
```

## 2. 核实与排序

```formal
RULE @grouped_verification:
  MUST: ["按页面状态分组打开报告截图和目标图，一次复现核实该状态的全部相关问题；同一原图只需打开一次，看不清先放大，不逐条重复截图",
         "再对照目标包的规范要点、比较口径和 DESIGN.md",
         "核实后分组处置：成立的进入修复"]

RULE @rejection_evidence:
  MUST: ["评审看错（运行中的界面与评审描述不符，附截图或实测值）、与目标包或规范矛盾、属于比较口径排除的差异：记「不采纳」并写明依据，不改代码",
         "报告附了核对说明的差距，同样自己核实，确实不成立就记不采纳"]

RULE @pending_gaps:
  MUST: ["需要素材、文案、数据、接口或产品决定才能修的：记「待决定」或「阻塞」，然后继续处理其他条目",
         "待核实的先在运行中的界面上确认现状；已经没问题就附截图如实记录，不为它改代码"]

RULE @blind_walk_recheck:
  MUST: ["先在真实页面按走查报告的状态文件或路径复走一遍确认；复现不了就如实记录，不为它改代码",
         "修复后复走同一路径并记录结果；是否到位由下一轮 review-ui 判定"]

RULE @suggestions:
  MUST: "建议不扣分；与规范一致且代价小时可以顺手做，但不能因此降低对目标的还原度"

RULE @fix_ordering:
  MUST: ["P0/P1 先修；同一严重度内按可挽回分从高到低",
         "能一次消掉多条差距的共同根因优先：如字体没加载、token 用错、布局原语选错、容器约束"]
```

## 3. 修复

```formal
RULE @fix_layering:
  MUST: ["在正确的层级修根因，层级从高到低：主题与 token、共享组件、页面布局、单处样式",
         "用 DESIGN.md 的 token 和现有组件",
         "目标确实要求偏离规范时，记下依据"]
  MUST_NOT: [magic_numbers_to_match_screenshots,
             duplicate_shared_components_for_single_page]

RULE @repair_scope:
  MUST: ["只改界面范围内必要的代码、样式、素材和测试",
         "保留用户已有的修改，不 reset、stash 或回退"]
  MUST_NOT: expand_into_api_data_contract_or_plan_scope_without_decision
  # 修复需要改接口、数据契约或计划范围时，记「待决定」，不擅自扩大。

RULE @regression_repair:
  MUST: "报告判为回归的差距：先用上一轮以来的 diff 找出是哪个改动引入的，修正它"
  MUST_NOT: trade_one_regression_for_another

RULE @stagnation_diagnosis:
  WHEN: report_has_stagnation_warning
  MUST: |
    只诊断预警范围（全局，或某条差距所在单元）：写清以往的尝试为什么没奏效；
    先区分实现缺陷、证据不足和标准争议；有实现依据时再检查根因——布局模型或
    组件选错、素材质量不够、token 或主题不符、字体或图片没正确加载、容器与
    断点约束、数据形态、采集环境与目标不一致。选证据最强的一条，采取最小有效
    修复或补证；只有结构性根因才重构，不为提分强制换组件或布局。
    把范围、诊断、实际改动与证据写进记录；实际重构每个范围每基线最多一次，
    没有重构也照实记录。
  MUST_NOT: [forced_refactor_for_score, unlimited_micro_tuning]
  # 标准争议记待决定；重复实际修改仍无效时由复审转为停滞。
```

## 4. 自检

```formal
RULE @self_check:
  MUST: ["启动当前代码，先复用现有自动检查验证受影响条件，再按页面状态统一采集必要代表帧，与目标对照",
         "共享组件、主题、字体、素材、依赖或布局变化时扩大复查范围；未受影响的有效结果复用，不机械重跑全部页面",
         "一张修复后的状态图可核实多条差距；可供复审的截图直接存到 <前缀>.ui-captures/r<n>-apply/，不覆盖评审截图",
         "「盲测走查」差距另按走查报告的状态文件或路径复走同一路径，记下实际步骤与结果"]

RULE @check_execution:
  MUST: ["运行计划和仓库要求的相关检查：组件测试、单元测试、类型检查、lint、构建",
         "项目有视觉回归测试时，只在改动符合目标的情况下更新快照，并说明更新了哪些",
         "检查失败时分清新引入的回归、原本就有的失败或环境问题；新引入的回归继续修"]

RULE @startup_blockage:
  WHEN: app_cannot_start_or_captures_cannot_be_taken
  MUST: ["先按补证与解阻流程处理；仍不能完成时，自检记「未运行」并写明已尝试动作及剩余原因",
         "处置仍是已修改待评审，由下一轮评审在真实界面上判定"]
  MUST_NOT: conclude_fixed_from_code_reading
```

## 5. 提交、记录与交付

```formal
RULE @record_updates:
  MUST: ["按记录合同保存本轮处置；更新这一轮已有的小节，不重复追加",
         "保留用户文字、其他轮次和历史评审"]

RULE @commit_rules:
  MUST: |
    计划有提交约定或用户授权提交时：只暂存本次修复和记录，遵守仓库的提交规范，
    不跳过提交钩子；提交后在完成记录对应里程碑行的「提交」列追加「UI修复」加
    短 SHA，再单独提交这次登记。两者都没有时不提交，修复留在工作树，
    下一轮评审以工作树为被测版本。
  MUST_NOT: fabricate_sha_when_commit_fails

RULE @final_diff_check:
  MUST: ["回读记录和计划的改动；查看最终 diff，确认没有无关修改",
         "评审报告、截图和 judge.md 都没被改动；目标包只多了经用户确认的决定；用户的修改没被覆盖"]

RULE @delivery_content:
  MUST: ["回复各类处置的数量和关键改动、重构假设（如有）、补证与解阻的动作/结果/材料路径/剩余缺口、自检和检查结果、提交 SHA 或未提交的原因、记录链接、待用户决定的事项",
         "交接附影响范围、共用截图到问题的映射、可复用材料及仍缺的条件",
         "没有新改动、新证据或阻塞解除时，续接原记录并说明解除条件"]

RULE @review_handback:
  MUST: |
    有新的可评证据、可执行的修复与补证完成后，结尾给下一步：用
    `review-ui <计划路径或原范围>` 复审，按当前客户端写法给出（如 Codex
    `$review-ui`、Claude Code `/review-ui`）。只有剩余工作确实是独立评审或
    确认评审、可执行项已完成时才交回复审；仍有外部阻塞时说明解除条件，
    不建议原条件下空跑。
  MUST_NOT: start_review_by_itself
```

调用示例：

```text
使用 $apply-ui-review goal/LOGIN.ui-review.r2.md
使用 $apply-ui-review goal/LOGIN.ui-review.r3.md，只处理 UI-04 和 UI-07。
使用 $apply-ui-review goal/LOGIN.ui-review.r5.md，U3 按设计稿改成两栏卡片。
```

## 项目状态回写

若仓库已登记状态台账，或存在仓根 `.xgent-ai/sdlc/protocol.md`（兼容已有 `docs/project-status/protocol.md`；两套并存且未登记切换时先核对，不能双写），交付原产物后读取台账的回写规则（默认目录读上述协议），按仓内协议更新受影响对象的当前状态与来源链接，必要时维护索引；v4 日常直接更新当前页，事件留存条件或旧版事件要求以仓内协议为准；未启用时不自动建表，不依赖另装技能。只读/仅聊天/不保存约束优先，登记失败须说明，不影响如实交付已完成的原产物。

接着原评审对象登记处置：差距编号、处置、修复版本、自检结果和未完项，并链接实现记录。「已修改待评审」不登记为已解决，修复也不能登记成独立复审通过。
