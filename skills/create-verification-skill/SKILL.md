---
name: create-verification-skill
description: "为指定应用或场景创建、维护验证 skill 时使用；不用于只写测试计划、一次性验收或只读咨询，按风险选用户旅程并建模用 create-e2e-verification-skill。生成可按范围执行的 verify-*，验证真实入口并保存证据，可供 agi-mode 复用。"
when_to_use: "用户会说「给这个应用做个验证 skill」「把验收命令固化成可复跑的验证」时。"
---

# Create Verification Skill

把项目中“如何启动、操作、观察和清理”的知识沉淀成 `verify-<app>`，让下一位 agent 无需当前会话也能验证真实行为。支持 Web、CLI/TUI、服务、桌面、移动端和库；具体工具由仓库与当前环境决定，不依赖特定编辑器、专属工具或 agi-mode。

生成时读 [产物约定](references/output-contract.md)，生成或实质修改后读 [生成结果验收](references/validate-generated-skill.md)，维护本生成器时按需读 [行为验收场景](evals/scenarios.md)。

```formal
INTENT:
  GOAL: generate_reusable_project_verification_skill
  SUCCESS: [next_agent_verifies_real_behavior_without_this_session,
            actual_run_evidence_and_cleanup_reported, coverage_gaps_stated]
  NON_GOALS: [test_plans, one_off_acceptance, product_refactors,
              global_tool_installation, release]

RULE @boundary:
  MUST: ["已有 verify-*、测试脚本或 harness 能完成一次性验收时直接执行；只要测试计划时用项目的测试规划流程",
         "输出位置按 用户指定 → 项目明确采用的 skill 目录 → .agents/skills/verify-<app>/；多个目录时先查项目规则和已有目标 skill；目录不保证宿主自动发现，必要时显式读取生成的 SKILL.md 执行",
         "同一应用已有验证 skill 时原位维护，保留人工补充和调用策略；没有维护授权时只复用或报告缺口",
         "新 skill 最小 frontmatter 是 name 与 description",
         "启动受阻先诊断；已授权范围内修复可直接继续，否则留下具体阻塞和恢复步骤"]
  MUST_NOT: [duplicate_skill_copies, global_config_changes_when_choosing_output_location,
             default_manual_only_invocation,
             "未获明确授权时把范围扩大到验证能力及必要本地夹具之外，如重构产品、安装全局工具或发布"]

RULE @scope_selection:
  MUST: ["从请求与项目事实确定目标应用/工作目录、用户场景、必要前置依赖、判据来源和明确排除项，简短说明本轮范围",
         "多应用仓库只调查目标及实际依赖；无法推导目标且会改变产物时才询问",
         =>[only_one_scenario_requested, "只写该场景、指定入口及必要准备/观察"],
         =>[initialize_app_without_named_scenario, "从已确认入口选少量有价值的用户路径，说明首批范围与遗漏"],
         =>[existing_capability_extended_or_maintained, "原位增量修改相关配方，沿用功能 ID、人工内容、目录和现有工具"],
         =>[unsupported_scenario, "报告未覆盖，不执行无关检查"],
         "范围落实到执行：产物给出选定场景的实际命令、现有测试选择器或按需操作配方；先核对选择范围，再启动有副作用的动作",
         "准备/观察用到相邻入口时说明依赖，不算已验收功能"]
  MUST_NOT: [separate_spec_file, full_app_map_for_single_scenario, features_by_quota,
             unsupported_scenario_falls_back_to_full_run, single_scenario_by_running_all_then_filtering_report]

RULE @repository_facts:
  MUST: ["先读仓库规则、启动文档、包脚本、现有测试和相关入口，再做最小运行确认；能观察到的事实不交给用户填写",
         "逐项确认下列事实。用户入口：用户实际操作的页面、命令、API、设备界面或公开库接口，多个表面分别登记",
         "确认启动与就绪：工作目录、依赖、构建/启动命令、端口、配置、测试账号/数据及可观察的就绪条件",
         "确认驱动方式：优先复用现有 Playwright/Cypress、PTY、HTTP 客户端、设备自动化或公开 API 的最小消费者",
         "确认观测与判据：用户动作、可见结果、持久化副作用如何取得；预期来自需求、既有契约或明确的用户行为",
         "确认隔离与归属：如何分配独立端口、数据目录、浏览器 profile、账号和进程；无法隔离时限制串行并说明实例归属",
         "确认 README 命令存在、工具可用及实际启动结果；不能运行的部分标未验证"]
  MUST_NOT: [readme_commands_as_verified_facts, invented_selectors_parameters_or_responses,
             source_reading_as_run_passed]

RULE @expected_behavior_source:
  MUST: ["每个关键判据指向已读的需求、公开契约或用户明确要求；源码与运行观察只说明当前行为",
         "没有权威预期或来源冲突时先查关联材料，仍无法确定就标待确认；可保存观察配方"]
  MUST_NOT: [claim_conformance_without_expected_source, generate_expectation_from_current_output]

RULE @external_effects:
  MUST: ["共享实例只有在明确允许且能界定副作用时才驱动",
         "外部写入沿用已有授权；发消息、真实支付或修改共享数据且授权不足时，先完成本地可验证部分，再指出具体缺口",
         "dry-run 或测试模式是否仍联网、开浏览器或写文件，核对实现及实际行为"]

RULE @generated_entry:
  MUST: ["按产物约定写出；该参考供生成时读取，不是生成结果的运行依赖",
         "单场景默认只用 verify-<app>/SKILL.md 写清完整配方；多场景需要独立读取时才拆 features/README.md 和 features/<feature-id>.md，共用启动/清理只保留一份",
         "SKILL.md 含范围选择、启动、只读体检（Doctor）、驱动、证据、清理和结果报告，可按任务合并段落",
         "命令从明确的工作目录执行；变量说明如何在本轮产生，跨步骤保留必要路径/进程标识",
         "优先直接引用现有测试命令与输出格式；缺口确实影响可靠执行才加最小 helper，写明调用方式并实跑",
         "证据沿用项目约定；没有时存 artifacts/verification/<app>/<run-id>/，与临时数据分开，每轮唯一目录，失败尝试也保留",
         ".xgent-ai/sdlc/ 只引用证据和任务状态"]
  MUST_NOT: [empty_directories, wrapper_scripts_for_uniform_shape,
             scripts_without_repetition_needing_automation,
             unresolved_guesswork_placeholders, local_absolute_paths,
             "把原始日志写入 .xgent-ai/sdlc/",
             "在生成文件留下对本生成器、其他未安装 skill 或专属工具的隐式依赖"]

RULE @scenario_recipes:
  MUST: ["为本轮范围中的功能/场景分配稳定 ID，写明入口、配方、前置条件和覆盖缺口；单场景内嵌，拆分后由 features/README.md 索引",
         "配方说明用户行为、范围内各入口、前置状态、逐步动作及有来源的可观察判据，随场景列出必要的边界、取消、错误与持久化检查",
         "标识稳定，命令/选择器来自真实项目；UI 优先语义角色、可访问名称或稳定属性，CLI/API 用真实参数和路由",
         "同一功能的按钮、快捷键、CLI 等入口分别记录覆盖",
         "地图保存复用配方，运行结果另存证据目录"]
  MUST_NOT: [map_as_run_everything_checklist, one_entry_pass_marks_other_entries_passed,
             old_pass_written_as_permanent_fact]

WORKFLOW @run_and_calibrate:
  DO: ["按生成的指令完成一轮：选择范围 → 启动 → Doctor → 所选场景完整路径 → 捕获证据 → 清理 → 确认证据仍在",
       "单场景创建实跑该场景；多场景初始化先跑一个代表路径，其余明确未验证；检查实际动作没有超出所选范围及必要依赖",
       "涉及写入时，从第二个公开入口、重开页面/进程或只读持久层确认状态，不只凭成功提示",
       "新增或可疑的关键断言，用已知好/坏观察样本确认能接受正确结果、拒绝错误结果；有可控隔离副本时，再从真实入口执行一个已知故障（如让保存失效），同时保留正常版本通过的证据",
       =>[generated_or_materially_changed, "按生成结果验收分开检查发现/选择与实际执行"],
       "复杂生成、新增关键 helper 或修改交接流程时，条件允许且委派已获授权则让无生成上下文的 agent 从成品冷启动；缺项修产物和造成缺项的生成规则，仅重验受影响项"]
  OUTPUT: [run_evidence, cleanup_result, verification_layers]

RULE @verification_honesty:
  MUST: ["假绿灯先修观测链路，记录原缺陷并重跑受影响路径，不降低产品判据",
         "每次失败后也运行本轮清理，检查残留资源和证据路径",
         "坏数据直接传给比较函数只校准断言，不称整条观测链路通过；不能安全注入产品故障时标明未验证层级",
         "构建、依赖或认证无法满足时，可交付明确标记的草稿及已确认事实",
         "只清理本轮创建的进程与临时状态，不删除证据"]
  MUST_NOT: [product_fault_injection_in_user_workspace_or_shared_instance,
             call_verified_without_end_to_end_run, one_feature_run_as_whole_map_passed,
             self_validation_as_independent_handoff, kill_processes_by_name,
             fabricated_handoff_without_independent_execution]

RULE @delivery_report:
  MUST: ["交付时报告 skill 路径、调用方式、已跑的场景/入口、证据位置与清理结果",
         "结构、选择、作者实跑、故障检测和冷启动各层实际验证范围",
         "生成器的冒烟验收与业务任务的完整验收分别说明",
         "没有独立执行条件时如实说明冷启动未执行"]

RULE @agi_mode_collaboration:
  MUST: ["agi-mode 负责结果契约与任务推进，生成的 verify-<app> 负责操作项目和返回可核对的证据；本 skill 只在缺少可复用验证能力且创建/维护属于当前授权时介入",
         "选择检查：agi-mode 将验收项映射到功能 ID、入口与必要边界，复用现有地图并指出未映射项",
         "执行验证：verify-<app> 接收本轮范围、目标版本/环境、隔离条件和证据位置，只执行所选场景及必要依赖；未知场景不退回全量",
         "返回结果：按功能与入口返回 passed / failed / blocked / not-run、操作、预期与实际结果、证据路径及清理状态",
         "判断完成：agi-mode 对照全部必要验收判断；跳过、受阻和未执行不算通过",
         "复用或重验：相关实现、依赖、配置、验证脚本或环境变化时重验受影响项，不仅重算哈希后沿用旧证据",
         "多阶段任务已使用 agi-mode 证据契约时，读已安装 agi-mode 的 references/evidence-contract.md，将功能/入口检查关联到现有 contract 和 receipts，复用其检查器；它只验证记录一致性，不替代真实操作",
         "没有 agi-mode 时，生成的 skill 仍按同样的结果格式独立工作"]
  MUST_NOT: [require_generation_on_every_agi_mode_entry, copy_evidence_protocol,
             force_json_on_simple_tasks, derive_sdlc_completion_from_generator_smoke_run]

RULE @maintenance:
  MUST: ["沿配方的判据来源和实际命令核对变化，只修改受影响的入口、脚本与共用前置条件并重跑对应路径；共享准备变化时检查依赖它的场景",
         "新增场景优先扩展原 skill；保留稳定 ID、人工补充、调用策略和不受影响的文件；无变化不重写",
         "产品行为未变而验证失效时修配方；真实产品失败保留并报告为 failed；移除过时入口前确认其确已退役，保留其他配方和历史证据",
         "维护本生成器时，结构校验与真实执行结果分开报告"]
  MUST_NOT: [second_startup_cleanup_copy, new_execution_framework,
             change_expectation_to_pass_real_product_failure,
             depend_on_uninstalled_maintenance_skill]
```

调用示例：

```text
$create-verification-skill 为当前仓库建立验证能力
$create-verification-skill 只为 Notes 的“保存后重新打开仍存在”场景建立验证能力，复用现有测试
```
