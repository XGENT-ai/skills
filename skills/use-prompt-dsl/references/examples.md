# Odyssey 示例

按任务选择相关示例；示例中的目标、预算、风格与领域类型不是通用要求。待提供的输入必须由真实材料补齐，不能把模板占位当作已获得的事实。

## 片段与 AGENTS.md：省略 PROMPT

直接嵌入规则，沿用承载文档的作用域；不用补 `PROMPT`、`MODULE` 或任务骨架。

```odc
RULE @read_contract:
  when: "准备修改现有行为"
  must: "先读取相关契约；影响实现的不确定性须指出并澄清"

RULE @scope:
  must_not: "未经请求增加功能或配置"

RULE @reuse:
  prefer: "满足契约时复用现有实现"
```

编写或解释上述片段不执行规则中的动作；正常加载的仓库指令可约束当前任务，但规则本身不创建任务。

## 共享片段：只在确有复用时用 MODULE

```odc
MODULE @evidence_policy:
  RULE @traceable_sources:
    must: "引用来源须能追溯到确实提供或实际检索的材料"

INTENT:
  goal: "评估用户提供的技术方案"
USE: [@evidence_policy]
OUTPUT:
  format: markdown
```

这里同样没有 `PROMPT`。`USE` 的模块正文已经提供，不代表自动加载外部规则库。

## 基于证据的技术分析

该示例将 Ontology、数据可信度、符号约束及 Markdown 输出映射结合起来。

```odc
PROMPT @research_analysis:
  INTENT:
    goal: "评估指定技术方案的可行性、收益、局限与替代选择"
    audience: "技术负责人"
    non_goals: [无依据的市场预测, 自动实施]

  ONTOLOGY:
    ENUM ClaimStatus: [supported, inferred, unknown, disputed]
    ENUM SourceStatus: [provided, checked, disputed]

    CONCEPT Source:
      fields:
        id: text[1]
        description: text[1]
        status: SourceStatus[1]

    CONCEPT Claim:
      fields:
        statement: text[1]
        status: ClaimStatus[1]
        sources: Source[0..*]

    CONCEPT AnalysisReport:
      fields:
        summary: text[1]
        claims: Claim[1..*]
        limitations: text[0..*]
        alternatives: text[0..*]
        recommendations: text[1..*]

    RELATION supported_by:
      from: Claim
      to: Source
      via: Claim.sources

    AXIOM @support_requires_source:
      for: @claim:Claim
      holds: =>[
        ==[@claim.status, supported],
        >=[count[@claim.sources], 1]
      ]

  CONTEXT:
    expects:
      available_sources: Source[0..*]

    # 使用时填入实际存在的 Source 对象引用，不要捏造来源
    available_sources: []

    DATA @brief:
      origin: user_supplied
      trust: untrusted
      content: |
        <在这里放置待分析的技术方案正文>

  RULE @within_scope:
    must: "只分析材料和可说明依据所支持的范围"

  RULE @evidence:
    for: @claim ∈ @output.claims
    when: ==[@claim.status, supported]
    must: ∃[
      @src ∈ @claim.sources,
      ==[@src.status, checked]
    ]
    on_unmet: [mark_unknown, disclose]

  RULE @source_existence:
    for: @claim ∈ @output.claims
    must: ∀[
      @src ∈ @claim.sources,
      ∈[@src, @context.available_sources]
    ]
    on_unmet: [revise, omit, disclose]

  RULE @source_relevance:
    must: "有来源还不够，来源内容必须实质支持对应主张"

  RULE @inference:
    must: "明确区分已核实信息、合理推论、假设和未知因素"

  RULE @no_fabrication:
    must_not: "编造来源、实测数据或确定性技术结论"

  RULE @brevity:
    prefer: "信息密度高，减少重复定义"

  OUTPUT:
    type: AnalysisReport
    format: markdown
    sections: [结论, 主要依据, 局限与不确定性, 可行替代, 建议]
    section_map:
      结论: @output.summary
      主要依据: @output.claims
      局限与不确定性: @output.limitations
      可行替代: @output.alternatives
      建议: @output.recommendations

  REVIEW:
    check: [@support_requires_source, @evidence, @source_existence,
            @source_relevance, @inference, @no_fabrication]
    also: [ontology_consistency, provenance, conflicts]
    expose: on_issue
```

**使用说明**：`@context.available_sources` 应由真实材料来源组成。`provided` 不自动等于 `checked`。如果没有实际核验途径，应保留不确定性，不能仅为了让规则通过而把来源状态改成 `checked`。

## 基于知识图谱的四选一题目

```odc
PROMPT @question_designer:
  INTENT:
    goal: "围绕输入知识点设计一道四选一题目"
    audience: "给定学段的学习者"
    non_goals: [生成多道题, 引入无关考点]

  ONTOLOGY:
    ENUM Difficulty: [easy, medium, hard]
    ENUM OptionId: [A, B, C, D]

    CONCEPT KnowledgePoint:
      fields:
        id: text[1]
        name: text[1]
        grade: int[1]
        prerequisites: KnowledgePoint[0..*]

    CONCEPT Option:
      fields:
        id: OptionId[1]
        content: text[1]

    CONCEPT Question:
      fields:
        stem: text[1]
        options: Option[4]
        answer: Option[1]
        explanation: text[1]
        knowledge_points: KnowledgePoint[1..3]
        difficulty: Difficulty[1]

    RELATION depends_on:
      from: KnowledgePoint
      to: KnowledgePoint
      via: KnowledgePoint.prerequisites
      acyclic: true

    RELATION assesses:
      from: Question
      to: KnowledgePoint
      via: Question.knowledge_points

    AXIOM @answer_is_option:
      for: @q:Question
      holds: ∈[@q.answer, @q.options]

    AXIOM @option_ids_unique:
      for: @q:Question
      holds: ==[
        count[@q.options],
        count[unique[@q.options.id]]
      ]

  CONTEXT:
    expects:
      input_knowledge_points: KnowledgePoint[1..3]

  RULE @knowledge_scope:
    for: @kp ∈ @output.knowledge_points
    must: ∈[@kp, @context.input_knowledge_points]
    on_unmet: [revise, ask, disclose]

  RULE @single_answer:
    must: "四个选项恰好有一个正确答案，干扰项应有诊断价值"

  RULE @grade_alignment:
    must: "题干与解析应符合给定学段的表达和认知水平"

  RULE @explanation:
    must: "解释正确答案的依据，并说明主要干扰项为何错误"

  RULE @no_invented_standard:
    must_not: "虚构课程标准条款、知识点 ID 或评价依据"

  OUTPUT:
    type: Question
    format: markdown
    sections: [题干, 选项, 答案, 解析, 关联知识点, 难度]
    section_map:
      题干: @output.stem
      选项: @output.options
      答案: @output.answer
      解析: @output.explanation
      关联知识点: @output.knowledge_points
      难度: @output.difficulty

  REVIEW:
    check: [@answer_is_option, @option_ids_unique, @knowledge_scope,
            @single_answer, @grade_alignment, @explanation,
            @no_invented_standard]
    expose: on_issue
```

`Question` 是模型应构造的**语义结果类型**，`OUTPUT.format: markdown` 是用户最终看到的表现形式；它们不冲突。`section_map` 说明两者的对应关系。

## UI 任务：完成后循环自查

```odc
PROMPT @site_design:
  INTENT:
    goal: "将当前页面打磨到适合本产品的获奖作品级视觉水准"
    success: "按当前真实画面与交互核对全部检查维度，没有尚未处理的可见差距"

  CONTEXT:
    current_page: "待提供：当前项目与可运行页面入口"
    benchmark: "Awwwards、Webby Awards、FWA 获奖作品；只选与当前产品适配的实际参考"

  RULE @product:
    must: "保留既定产品目标、品牌与必须支持的功能"

  RULE @visual_quality:
    must: "逐项检查字体、留白、视觉层级、色彩、动效、微交互、响应式与原创性；结合实际参照解释质量标准，某项不适用须给出依据"

  RULE @observation:
    must: "读取当前真实渲染画面，实测相关交互与响应式状态；只报告实际查看过的参考，缺证据的维度保留 unknown"
    on_unmet: [disclose]

  LOOP @self_review:
    trigger: on_completion
    do:
      - "读取当前页面的真实渲染画面，实测相关交互与响应式状态"
      - "按 @visual_quality 逐项检查，与 @context.benchmark 中实际查看过的作品比较"
      - "发现差距时修订页面，并重新检查受影响项"
    until: "全部检查维度均已验证，没有剩余可见改进或 unknown 项"

  OUTPUT:
    format: markdown
    sections: [页面实现, 实际参照, 设计取舍, 验证证据, 未完成项]

  REVIEW:
    check: [@product, @visual_quality, @observation]
    expose: on_issue
```

这是给已明确请求实际开发的 `execute` 使用的任务约定；`compose` 只输出提示词。此例未指定轮数限制，不添加 `max`；给定限制时才填入相应值。`LOOP` 不创建后台任务或证明数学上的最优。

## 操作示例

```text
$use-prompt-dsl compose 把这些原则写成嵌入 AGENTS.md 的 Odyssey 片段，省略 PROMPT
$use-prompt-dsl compose 为摄影师作品集写开发提示词，黑白风格、移动端优先，完成后逐项检查
$use-prompt-dsl execute 按下面的 DSL 修订指定文件并核对结果：……
$use-prompt-dsl explain 解释下面的本体、公理、规则例外与未知条件：……
```
