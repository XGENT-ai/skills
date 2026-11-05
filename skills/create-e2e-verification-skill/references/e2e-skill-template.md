# e2e-* 生成模板

生成时读取，不是生成结果的运行依赖。把每个 `<...>` 换成已核实的事实；未跑通的写阻塞，不留占位。写法遵循 Odyssey（ODC）：自描述字面量优先，命令、路径和影响判断的歧义才用字符串。已安装 `use-prompt-dsl` 时可读其语言规范的状态迁移一节，没有时按本模板即可。

## 模型与 ODC 对应

有模型时，`model/<area>.statechart.json` 与 SKILL.md 的声明描述同一件事，规则 ID 相同：

| ODC | statechart | 谁来证明 |
| --- | --- | --- |
| `CONCEPT.FIELDS` / `INITIAL` | `model.variables` / `model.init` | — |
| `TRANSITION`：`ON`、`WHEN`、`TO` | `model.actions`：动作 id、`when`、`set` | 走过该箭头的场景 |
| `REQUIREMENT` `KIND: safety`，`always[P]` | `model.rules`：`holds` | 模型穷举加真实场景 |
| 允许停下的终态 | `model.deadlock.ends` | 模型穷举加真实场景 |
| `REQUIREMENT` `KIND: liveness`，`ASSUME` | 无；statechart 不判定 | 只有真实场景 |
| `counterexample_search` | `breaks` 规则的最短 `path` | 真实入口复现 |

ODC 声明是约定，不是检查结果。模型与声明不一致时，以代码和需求为准修正两者。

## SKILL.md

````markdown
---
name: e2e-<area>
description: "<改动涉及哪条旅程或哪些路径时使用；不用于哪些相邻场景>。从真实入口检查<用户承诺>。"
---

# E2E · <旅程名>

<一句话：用户要做成什么；出事时用户会遇到什么。>

```formal
INTENT:
  GOAL: check_change_against_<journey>_from_real_entry
  SUCCESS: [selected_scenarios_run, evidence_saved, resources_cleaned]
  NON_GOALS: [product_fixes_unless_requested, unselected_scenarios]

ONTOLOGY:
  ENUM <State>: [<用户看到的状态>]
  CONCEPT <Journey>:
    FIELDS:
      <field>: <State>
    INITIAL:
      <field>: <初值>

TRANSITION @<action>:
  SUBJECT: <Journey>.<field>
  FROM: <状态>
  TO: <状态>
  ON: <用户操作或系统事件>
  WHEN: <守卫，无则省略>
  UNCHANGED: [<必须不变的字段>]
  # 代码：<路径:符号>

REQUIREMENT @<rule_id>:
  KIND: safety
  SOURCE: "<需求、文档或用户要求的位置>"
  HARM: "<违反时用户遇到什么>"
  MUST: always[<用户认同的承诺>]
  ACCEPTANCE: [scenario_<id>]

REQUIREMENT @<progress_id>:
  KIND: liveness
  SOURCE: "<位置>"
  ASSUME: [<进展依赖的前提>]
  MUST: always[=>[<条件>, eventually[<结果>]]]
  ACCEPTANCE: [scenario_<id>]

RULE @scope:
  MUST: ["改动碰到下表路径才运行对应场景；只运行所选场景及必要依赖",
         "未知场景报告未覆盖，不退回全量"]
  COVERAGE:
    scenario_<id>: ["<路径:符号>"]
  SYNC_COMMIT: <最后确认验证仍描述代码的 SHA>
  NOT_COVERED: ["<明确没覆盖的入口、平台或路径>"]

WORKFLOW @run:
  DO: ["工作目录 <dir>，运行 <构建或准备命令>",
       "启动 <命令>，就绪判据 <可观察条件>，等待上限 <秒>",
       "只读体检：<确认版本、配置和实例归属的命令>；不符就停，不改连其他服务",
       "运行所选 scenario_*",
       "证据写入 <证据目录>/<时间戳>/：命令、输入、输出、退出码、副作用",
       "清理本轮资源：<命令>；成功失败都执行，保留证据"]
  OUTPUT: [scenario_status, actual_observation, evidence_path, cleanup_result]

WORKFLOW @scenario_<id>:
  # <谁，要做成什么，用什么输入>
  DO: ["<用户动作> → <可执行命令或页面操作> → 预期 <可观察结果>",
       "<下一步，连续同一任务>",
       "从第二入口确认：<重开、查询或只读持久层>"]
  COVERS: [<rule_id>, <transition_id>]

RULE @model:
  WHEN: "改动影响上面的状态、动作或规则，且已安装 phoenix-diagram 且支持 statechart"
  MUST: ["读 phoenix-diagram skill，用其 statechart 校验 model/<area>.statechart.json，结果放证据目录；只取规则判定，不做 HTML 交付与看图审核",
         "规则结果完整时布局诊断不影响结论；改动已改变模型描述的行为时，先让模型描述当前代码再判定。只评审未提交改动且未获授权更新 skill 时，在临时副本改模型，结果标为参考",
         "规则 breaks 时按最短路径逐步在真实入口复现；复现前先假定模型有误",
         "规则 holds 不代表代码通过，仍运行受影响的场景"]
  MUST_NOT: "复制 phoenix-diagram 的命令或路径到本 skill"
  # 未安装时跳过模型步骤，记 not-run，场景照常运行。

RULE @known_findings:
  # SYNC_COMMIT 上已在真实入口复现的产品缺陷；没有就省略本块。修好后删除对应条目。
  MUST: ["<用户的话描述缺陷>：<复现步骤与实际结果>。<受影响的 scenario>"]

RULE @triage:
  MUST: ["场景失败先复现并定位，分为 product_bug、verification_drift、environment；实际值与 @known_findings 完全一致才算旧缺陷，同一步骤的值变了也是新问题；拿不准时在隔离副本用 HEAD 跑同一场景对比",
         "product_bug 用用户的话和步骤报告，不改预期让它通过",
         "有意的行为变更：同步更新 TRANSITION、REQUIREMENT、模型与场景，并说明依据",
         "确认验证仍描述当前代码后才前移 SYNC_COMMIT"]
  MUST_NOT: [weaken_rule_to_pass, old_pass_as_current_result]

RULE @report:
  MUST: "按场景列 passed / failed / blocked / not-run、实际观察、证据路径与清理结果；未跑的必要检查明确列出"
```
````

单条直线旅程没有分支和等待时，省略 `TRANSITION`、`@model` 和模型文件，保留 `REQUIREMENT` 与场景。旅程很短时把多个场景写在同一 `WORKFLOW` 也可以，不为统一外形拆块。只有重复操作确实需要时才加 `scripts/`，并在 `@run` 或场景里写出实际调用。
