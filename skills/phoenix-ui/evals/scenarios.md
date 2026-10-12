# Phoenix 设计路由与上下文恢复

在新的实际 provider 对话中执行，记录加载的 skill/source 路径、工具调用、原文读写 diff 与结论。以下是待执行用例，不是已通过记录；静态 metadata 检查不能替代 V-9。

1. **普通 Markdown 规范与缺 CLI**：给已有 PRODUCT.md、无 YAML 的 DESIGN.md，要求局部优化；使 Phoenix CLI 不可用。直接读原文并延续现有规范，明确 profile/映射未加载，不自动安装、不注入默认颜色或字体。
2. **跨会话续做已接受方向**：给原 brief、接受决定和匹配的 target/path/hash 索引，要求继续实现。读取 context JSON 和原 brief，复用已定事实，不重新采访或 roll；没有第二份可编辑 brief。
3. **接受原文漂移与多个候选**：分别修改原 brief、移走原文件、配置两个适用原文。展示 stale/missing/ambiguous 与真实来源，确认必要的变更决定，不按 mtime 或新 hash 自动接受。
4. **显式更换方向**：已有已接受方向，但用户明确要求改选并说明范围。沿用产品事实，记录改选范围，通过 design-reference 形成新选择；既有冻结目标按原审批规则处理。
5. **Portal 初始化缺内部 skill**：要求初始化 Portal 子应用，已有 PRODUCT.md/DESIGN.md，未安装 xgent-init。提供显式内部安装命令并暂停 Portal 初始化；可以只读探索，不创建通用独立页面，不覆盖已有原文。
6. **已选交互方案与可选 profile**：给 ui-pattern-research 已接受方案和显式 generic/Portal profile，要求 shape/实现。消费既有状态与交互方案；profile 仅作建议，实际能力仍查代码/接口，冲突不覆盖用户决定。
7. **独立 UI 验收与有报告后的修复**：先要求完整 UI 验收，再提供 UI 报告要求修复。分别路由 review-ui/apply-ui-review；缺对应 skill 时报告前置条件。Phoenix 自检的两轮上限不截断独立循环或已授权工作，finish-reviewer/component-review 不替代 UI 验收完成证据。
