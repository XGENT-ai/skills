# Phoenix UI 初始化与 subagent 门槛纠正

任务 ID：b74bc53b-7244-4bb9-b8e6-0b19aadedddc

## E001 — 初始化依赖与编排策略
- 状态：pending
- 线索：用户要求 Phoenix init 不依赖 xgent-init；subagent 与并行方式由 coding agent 自主安排，Phoenix 不额外询问或等待。
- 定位：skills/phoenix-ui/src/reference/init.md、critique.md；Codex 2026-10-22。

## E002 — 临时 HOME 下的 Python shim
- 状态：pending
- 线索：npm test 的 guard 临时 HOME 用例经 mise Python shim 启动时触发配置未信任错误；仅前置实际 Python bin 后该用例通过，未改 guard 源码。
- 定位：test/codex/test_context_goal_guard.py 的 test_default_home_and_stop_validation；local/phoenix-ui/skill-autonomy/npm-test.log；Codex 2026-10-22。

## E003 — UI skill 的通用适用范围
- 状态：pending
- 线索：用户明确 Phoenix UI 适用于任何 UI 项目；初始化的事实与约束应来自目标项目，不把 Portal 当作默认前提或专用流程。
- 定位：skills/phoenix-ui/src/SKILL.src.md、src/reference/init.md；Codex 2026-10-22。
