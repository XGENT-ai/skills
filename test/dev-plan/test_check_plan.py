from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from bash_check import run_progress


ROOT = Path(__file__).resolve().parents[2]
SKILL = ROOT / "skills/dev-plan"
PLAN = """# 编辑标题

> **计划状态：Ready**
> 调查基线：2026-10-06 · abc123 · clean。
> 需求来源：用户需求，无 PRD。

## 实施者定位

按计划、仓库规则与当前代码实施。

## 实施进度

### 恢复快照

- 最近更新：尚未开始
- 当前进度：0/1 个里程碑完成
- 当前状态：尚未开始
- 最近完成：无
- 下一步：M1 · 实现并通过 V-1
- 当前阻塞：无
- 代码基线：abc123

### 完成记录

| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |
| --- | --- | --- | --- | --- | --- |
| — | — | — | 尚未开始任何里程碑 | — | — |

## 0. 需求、范围与决策

### 0.1 需求与约束账本

| ID | 类型 | 来源 | 内容 | 设计/验收落点 | 状态 |
| --- | --- | --- | --- | --- | --- |
| R-1 | 功能 | 用户需求 | 保存新标题 | §5、V-1 | 已确认 |
| NFR-1 | 安全 | 仓库规则 | 仅本人可修改 | §7、V-1 | 已确认 |

## 5. 实现

R-1 在 `src/title.ts` 中实现，保存后返回标题。

## 7. NFR、安全与运行保障

| ID | 基线/来源 | 目标或未知项 | 超限/失败行为 | 机制 | 验证 |
| --- | --- | --- | --- | --- | --- |
| NFR-1 | 既有鉴权 | 仅本人可修改 | 拒绝 | 校验身份 | V-1 |

## 9. 验证

| 检查 ID | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | R-1、NFR-1：保存后可读，越权被拒绝 | npm test，本地 | M1 实现后 |

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 编辑标题 | 无 | 修改标题模块 | V-1；回写「实施进度」 |

## 13. 需求 → 设计 → 验证映射

| ID | 需求/约束/假设 | 设计落点 | 验证/解除办法 | 结果 |
| --- | --- | --- | --- | --- |
| R-1 | 编辑标题 | §5 | V-1 | 覆盖 |
"""
TASKS = """
### 10.1 内部任务

| 任务 ID | 所属里程碑 | 交付物/影响路径 | 前置成果 | 检查 ID |
| --- | --- | --- | --- | --- |
| T-2 | M1 | 保存 API，见 §5 | T-1 的稳定契约 | V-1 |
| T-1 | M1 | 标题契约，见 §5 | 无 | V-1 |
"""


class PlanCheckTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="Plan Check ")
        self.addCleanup(self.temp.cleanup)
        self.plan = Path(self.temp.name) / "计划.md"

    def run_check(self, text, code=0, diagnostic=None):
        self.plan.write_text(text, encoding="utf-8")
        result = subprocess.run(
            [sys.executable, str(SKILL / "scripts/check_plan.py"), str(self.plan)],
            text=True, capture_output=True, timeout=5,
        )
        self.assertEqual(result.returncode, code, result.stdout + result.stderr)
        self.assertEqual(result.stderr, "")
        if diagnostic:
            self.assertIn(diagnostic, result.stdout)
        return result.stdout

    def test_trimmed_initial_plan_and_references_pass_all_structural_checks(self):
        self.assertIn("WARN 0", self.run_check(PLAN))
        result = run_progress(SKILL / "scripts/check_progress.sh", self.plan, self.temp.name, 5)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_real_execution_protocol_is_not_template_residue(self):
        skeleton = (SKILL / "references/plan-skeleton.md").read_text(encoding="utf-8")
        protocol = skeleton.split("## 实施者定位\n", 1)[1].split("### 恢复快照\n", 1)[0]
        start = PLAN.index("## 实施者定位\n")
        end = PLAN.index("### 恢复快照\n")
        self.run_check(PLAN[:start] + "## 实施者定位\n" + protocol + PLAN[end:])

    def test_optional_tasks_allow_forward_references(self):
        self.run_check(PLAN.replace("## 13.", TASKS + "\n## 13."))

    def test_duplicate_definitions_fail_but_expansion_and_mapping_do_not(self):
        text = PLAN + TASKS
        for marker in ("| R-1 | 功能", "| V-1 | R-1", "| M1 | 编辑", "| T-1 | M1"):
            row = next(line for line in text.splitlines() if line.startswith(marker))
            with self.subTest(marker=marker):
                self.run_check(text.replace(row, row + "\n" + row), 1, "ID 重复定义")

    def test_dangling_references_in_prose_tables_and_inline_ids_fail(self):
        for extra in ("引用 R-9", "检查 `V-9`", "依赖 T-9", "先做 M9", "检查 R-1/V-9", "\n| NFR-9 | 缺失 |"):
            with self.subTest(extra=extra):
                self.run_check(PLAN + "\n" + extra, 1, "ID 引用未定义")

    def test_upstream_ids_keep_their_namespace_and_require_a_source(self):
        text = PLAN.replace("R-1", "REQ-AUTH-007")
        self.run_check(text)
        self.run_check(text + "\n引用 REQ-AUTH-008", 1, "ID 引用未定义")
        self.run_check(text.replace("| 用户需求 |", "| |"), 1, "字段未填：来源")

    def test_paths_do_not_hide_adjacent_references(self):
        self.run_check(PLAN + "\n见src/R-99.md，检查V-9。", 1, "ID 引用未定义")
        self.run_check(PLAN + r"\n路径 C:\docs\V-99 和 src/R-99/V-99.md。")

    def test_unknowns_blocked_state_and_empty_completion_record_are_valid(self):
        text = PLAN.replace("Ready", "Blocked").replace("| 已确认 |", "| 未知 |")
        self.run_check(text + "\n未知：容量须在发布前确认，责任人为维护者；影响发布决策。")

    def test_missing_columns_values_and_definition_tables_fail(self):
        for before, after, message in (
            ("| 执行时点 |", "| 其他 |", "缺少字段"),
            ("| 内容与并行边界 |", "| 其他 |", "缺少字段"),
            ("| 用户需求 |", "| |", "字段未填"),
            ("| 类型 | 来源 |", "| 分类 | 出处 |", "缺少账本定义表"),
        ):
            with self.subTest(before=before):
                self.run_check(PLAN.replace(before, after), 1, message)

    def test_optional_task_table_requires_owner_dependencies_and_checks(self):
        for before, after, message in (
            ("| 所属里程碑 |", "| 其他 |", "缺少字段"),
            ("| T-1 | M1 |", "| T-1 | M9 |", "ID 引用未定义"),
            ("| T-1 | M1 |", "| T-1 | 无 |", "所属里程碑 ID"),
            ("| 无 | V-1 |", "| | V-1 |", "字段未填：前置成果"),
            ("| 无 | V-1 |", "| 无 | |", "字段未填：检查 ID"),
            ("| 无 | V-1 |", "| 无 | 随便测一下 |", "须引用验证表中的检查 ID"),
        ):
            with self.subTest(before=before, after=after):
                self.run_check(PLAN + TASKS.replace(before, after), 1, message)

    def test_known_placeholders_are_errors_including_inline_fields(self):
        for extra in ("\n<代号>", "\n正文：<可观察的业务能力>", "\n目标文件：`<名称/路径>`"):
            with self.subTest(extra=extra):
                self.run_check(PLAN + extra, 1, "残留骨架占位")
        self.run_check(PLAN.replace("2026-10-06", "<YYYY-MM-DD>"), 1, "残留骨架占位")

    def test_suspected_custom_placeholders_warn_without_rejecting_unknowns(self):
        self.run_check(PLAN + "\n输出：<本项目特有的待填项>", 0, "WARN:")

    def test_code_samples_paths_and_link_destinations_are_not_plan_ids(self):
        extra = r"""
```markdown
| 检查 ID | 需求/验收行与可观察结果 | 命令/入口与环境 | 执行时点 |
| --- | --- | --- | --- |
| V-1 | R-99 | command | M99 |
```
~~~python
sample = 'T-99 <自定义占位>'
~~~
`src/R-99.ts`、docs/V-99.md、[参考](other/M99.md)，示例命令 `echo V-99`。
UI 使用 <select> 和 `Array<T>`，文案支持 `a \| b`。
"""
        self.assertIn("WARN 0", self.run_check(PLAN + extra))

    def test_missing_baseline_or_invalid_readiness_fails(self):
        self.run_check(PLAN.replace("调查基线", "其他字段"), 1, "缺少或未填：调查基线")
        self.run_check(PLAN.replace("2026-10-06 · abc123 · clean", "TBD"), 1, "缺少或未填：调查基线")
        self.run_check(PLAN.replace("Ready", "Completed"), 1, "计划状态无效")

    def test_missing_requirement_source_only_warns(self):
        self.run_check(PLAN.replace("> 需求来源：用户需求，无 PRD。\n", ""), 0, "WARN: ")
        self.run_check(PLAN.replace("用户需求，无 PRD", "—"), 0, "缺少或未填：需求来源")

    def test_escaped_pipes_pass_but_broken_table_width_fails(self):
        self.run_check(PLAN.replace("保存新标题", r"保存 a \| b"))
        self.run_check(PLAN.replace("保存新标题", "保存 a | b"), 1, "列数不一致")


if __name__ == "__main__":
    unittest.main()
