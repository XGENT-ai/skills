---
name: create-e2e-verification-skill
description: "用于为高风险用户旅程创建、维护 e2e-* 端到端验证 skill，可用状态机模型推出场景；不用于普通场景配方（用 create-verification-skill）、一次性验收或只写测试计划。从真实入口运行并证明能发现故障。"
when_to_use: "用户会说「做个端到端验证 skill」「这条用户路径最容易出事，固化成 E2E」「测试全过用户还是用不了，补真实验证」时。"
---

# Create E2E Verification Skill

局部测试检查写的人想到的局部情况，E2E 检查用户到底能不能做成事。本 skill 找出出事代价最高、现有测试又碰不到的用户旅程，从真实入口验证它，并留下一份 `e2e-<area>` 项目 skill，让下一位 agent 能用它检查自己的改动。

生成结果按 [模板](references/e2e-skill-template.md) 用 Odyssey（ODC）写，读者是中途接手、没见过本次工作的 agent；用户不懂模型语言也要能信任并使用结果。

```formal
INTENT:
  GOAL: generate_project_local_e2e_skill_for_costliest_user_journey
  SUCCESS: [generated_skill_catches_a_real_break_from_real_entry,
            findings_reported_in_user_terms, scope_and_gaps_stated]
  NON_GOALS: [full_test_matrix, product_fixes_unless_requested, new_test_framework,
              global_tool_installation, release]

RULE @boundary:
  MUST: ["输出位置按 用户指定 → 项目明确采用的 skill 目录 → .agents/skills/e2e-<area>/，名称与目录一致且以 e2e- 开头",
         "同一旅程已有 e2e-* 时原位维护，保留人工内容、调用策略与稳定 ID；已有 verify-* 或测试 harness 能驱动时复用，不复制第二套启动与清理",
         "一次一条旅程；多条候选只验证最强的一条，其余列在交付"]
  MUST_NOT: ["只因请求含‘验证’就落盘：解释、测试规划与一次性验收走各自流程",
             "在生成文件留下对本生成器或未安装 skill 的隐式依赖"]

RULE @target_selection:
  MUST: ["从需求、文档、事故、近期修复、经验记录和现有测试取事实；用户点名区域或缺陷时从那里开始；只问观察不到的",
         "好目标是产品对用户的一项承诺：失败时伤害用户，现有测试又碰不到让它失败的情形",
         "优先看测试到不了的层：打包安装后的入口、跨进程、持久化与重启、权限、失败提示、按公开文档首次上手",
         "选代价最高的旅程，说明为什么选它"]
  MUST_NOT: choose_the_easiest_journey_to_verify

RULE @expected_behavior_source:
  MUST: "每条规则和判据指向已读的需求、公开契约或用户明确要求；没有权威预期时标待确认，只保存观察配方"
  MUST_NOT: ["以当前输出生成预期，让缺陷自证正确", "把源码阅读写成运行通过"]

RULE @journey_model:
  WHEN: journey_has_waiting_retry_cancel_recovery_multiple_entries_or_order_sensitive_steps
  MUST: ["读取已安装 phoenix-diagram skill 的 statechart 说明并按其流程检查模型；从中取得当前命令与字段",
         "变量写用户能观察或在意的状态，动作写用户操作与系统事件，规则写成用户认同的一句话承诺",
         "同一模型同时写成生成 skill 里的 ONTOLOGY / TRANSITION / REQUIREMENT，同名 ID 一一对应（对应表见模板）",
         "每个状态和动作指向可打开的代码或文档；代码里一步完成的事，模型也算一步",
         "先用最粗的 view 取得规则判定：一个用户状态一个标签，变量值不写进标签，规则 id 取短；回执没有规则判定时继续收粗 view，已有完整规则判定时布局诊断不影响结论",
         "用户要看图、或判定需要人裁决时，再细化 view 并按 phoenix-diagram 的完整交付流程出图"]
  MUST_NOT: ["为让检查通过放宽规则或删掉变量",
             "把 phoenix-diagram 的命令、选项或路径复制进生成 skill 或脚本；它们随版本变化，只按名称引用",
             "把 ODC 声明本身当成已验证的结果"]
  # 直线旅程没有这些情形，跳过模型，直接写场景。

RULE @statechart_unavailable:
  WHEN: "phoenix-diagram 未安装、其图型表没有 statechart，或按其停滞规则已停止修改模型"
  MUST: ["继续生成：手写 ONTOLOGY / TRANSITION / REQUIREMENT，用 counterexample_search 人工推演，并标明模型穷举 not-run",
         "交付时说明缺口与补上后能多得到什么"]
  MUST_NOT: [block_e2e_generation, install_tools_without_authorization,
             report_hand_reasoning_as_model_checked]

RULE @model_results:
  MUST: [=>["规则 breaks", "最短路径只是假设：按路径逐步在真实入口复现；复现前先假定模型有误。复现成立记为发现，不成立修模型"],
         =>["规则 holds", "只说明模型成立，代码是否如此仍由场景证明"],
         =>["not-checked 或 complete: false", "列出未判定的规则与原因，不算通过"],
         "statechart 只判定 Safety 规则与死锁；Liveness 写 REQUIREMENT 与 ASSUME，由真实场景观察，不标为模型已证",
         "交付说明模型省略了什么、隐藏了哪些步骤"]

RULE @scenarios:
  MUST: ["从用户真实要完成的任务写场景，不从验收行逐条翻译：谁、要做成什么、用什么真实或有代表性的输入",
         "一个场景是一段连续任务，不是一张断言清单；有模型时用尽量少的场景走完图上每条箭头，每条规则至少有一个场景逼近它的边界，走不到的列为未覆盖",
         "从用户实际入口经真实运行栈与本方依赖执行；只有范围外依赖未就绪时才在外部边界用替身，并标明未覆盖真实集成",
         "看用户实际得到的结果，涉及写入时再从第二个入口、重开进程或只读持久层确认；后续步骤可能掩盖错误状态时，在关键步骤后立即观察",
         "用项目已有工具或短脚本，每个场景一条命令，可单独选择"]
  MUST_NOT: ["另建验收框架、认证流水线或证据体系", "为本方代码造 fixture 或内存替身",
             "未知场景退回全量执行", "不够授权时驱动共享实例、真实支付、发消息或外部写入",
             "只因名字是 dry-run 或测试模式就认定没有副作用"]

RULE @findings:
  MUST: ["当前代码违反承诺时，用用户的话和复现步骤报告，状态记 failed",
         "产品缺陷、验证漂移、环境问题分开判断"]
  MUST_NOT: [weaken_rule_or_scenario_until_it_passes, change_product_code_without_request]

RULE @generated_skill:
  MUST: ["按模板写实：真实命令、路径和名词，不留占位；未跑通的部分明确写阻塞",
         "下一位 agent 只读它就能判断改动是否涉及本旅程、运行所选场景、区分缺陷与漂移、在有意变更时更新它",
         "记录每个场景覆盖的文件与函数，以及最后确认过的提交；只有确认验证仍描述该提交的代码后才前移",
         "每轮证据写入项目约定目录，没有时写 artifacts/verification/e2e-<area>/<run-id>/，失败也保留",
         "SYNC_COMMIT 上已复现的产品缺陷写进 @known_findings，让下一位 agent 分清旧缺陷与新回归；其他运行结果和生成过程的验证层级只放证据目录与交付",
         "状态只用 passed / failed / blocked / not-run；只清理本轮创建的资源"]

WORKFLOW @prove_before_handoff:
  DO: ["按生成的 skill 完整跑一遍：选择场景 → 启动 → 只读体检 → 场景 → 取证 → 清理",
       "在隔离副本对真实代码做一次临时破坏（去掉检查、调换步骤、成功但不落盘），用同一命令重跑，必须因目标行为失败",
       "丢弃副本，确认检查回到原状：通过，或只因将要报告的发现而失败",
       "复杂生成且委派已获授权时，让无生成上下文的 agent 只凭成品和干净副本冷启动"]
  ON_FAILURE: "漏掉破坏或某步跑不通，修生成的 skill 或模型，不改破坏；语法错误、缺依赖导致的失败不算抓到"
  OUTPUT: [normal_run_evidence, broken_run_evidence, cleanup_result]

RULE @verification_status:
  MUST: "没跑过的 skill 只算草稿，分层说明结构、作者实跑、破坏检测与冷启动各做到哪里"
  MUST_NOT: [self_run_as_independent_verification, one_journey_as_whole_product_verified]

RULE @maintenance:
  WHEN: existing_e2e_skill_and_covered_code_changed
  MUST: ["从最后确认的提交起查看改动，只更新受影响的模型与场景并重跑",
         "每个场景给一个结果：in_sync（未受影响且通过）、updated（已改到描述新代码且通过）、bug（验证描述现状而代码违反承诺）、blocked（说明原因）"]
  MUST_NOT: [edit_product_code_during_maintenance, weaken_rule_to_clear_failure]

RULE @handoff:
  MUST: [journey_chosen_and_why, rules_in_one_user_sentence_each,
         "模型结果：逐条规则的判定与最短路径；或模型穷举 not-run 的原因",
         real_entry_findings_with_repro_steps, how_to_run, skill_path_and_evidence_path,
         break_detection_result, what_is_not_covered]
  SHOULD: suggest_second_target_only_when_a_close_one_was_found
```
