from pathlib import Path
import os
import subprocess
import tempfile
import unittest
from unittest import mock
from bash_check import run_progress


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "skills/dev-plan/scripts/check_progress.sh"
HEADER = (
    "| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 | 提交 |\n"
    "| --- | --- | --- | --- | --- | --- |"
)
LEGACY_HEADER = (
    "| Milestone | 状态 | 更新时间 | 简要记录 | 实现与验收记录 |\n"
    "| --- | --- | --- | --- | --- |"
)
PLAN = """# 测试计划

## 实施进度

### 恢复快照

- 最近更新：2026-10-07 10:00 +08:00
- 当前进度：{done}/2 个里程碑完成
- 当前状态：{state}
- 最近完成：{latest}
- 下一步：{next}
- 当前阻塞：无
- 代码基线：abc1234

### 完成记录

{records}

## 10. 里程碑与验收安排

| # | 里程碑 | 前置依赖 | 内容与并行边界 | 验证/退出条件 |
| --- | --- | --- | --- | --- |
| M1 | 第一步 | 无 | 改 A | V-1；回写「实施进度」 |
| M2 | 第二步 | M1 | 改 B | V-2；回写「实施进度」 |
"""


def row(milestone, commit=None):
    cells = [milestone, "已完成", "2026-10-07 10:00 +08:00", "完成", f"[{milestone} 记录](计划.records/{milestone}.md)"]
    if commit is not None:
        cells.append(commit)
    return "| " + " | ".join(cells) + " |"


class ProgressCommitTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="Progress Check ")
        self.addCleanup(self.temp.cleanup)
        self.dir = Path(self.temp.name)
        self.plan = self.dir / "计划.md"
        (self.dir / "计划.records").mkdir()
        for milestone in ("M1", "M2"):
            (self.dir / f"计划.records/{milestone}.md").write_text("# 记录\n", encoding="utf-8")

    def run_check(self, rows, header=HEADER, code=0, diagnostic=None):
        done = len(rows)
        latest = "M2 · 第二步" if done == 2 else "M1 · 第一步"
        state, nxt = ("已完成", "代码评审") if done == 2 else ("进行中 M2", "M2 · 改 B")
        records = header + "\n" + "\n".join(rows)
        self.plan.write_text(
            PLAN.format(done=done, state=state, latest=latest, next=nxt, records=records),
            encoding="utf-8",
        )
        result = run_progress(SCRIPT, self.plan, self.dir, 10)
        self.assertEqual(result.returncode, code, result.stdout + result.stderr)
        if diagnostic:
            self.assertIn(diagnostic, result.stdout)
        return result.stdout

    def test_pending_commit_is_allowed_before_all_milestones_complete(self):
        self.assertIn("WARN 0", self.run_check([row("M1", "待提交")]))

    def test_completed_row_needs_sha_or_pending(self):
        for commit in ("—", "", "稍后"):
            with self.subTest(commit=commit):
                self.run_check([row("M1", commit)], code=1, diagnostic="须为短 SHA 或「待提交」")

    def test_final_state_rejects_pending_rows(self):
        self.run_check([row("M1", "`a1b2c3d`"), row("M2", "待提交")], code=1, diagnostic="仍写「待提交」")

    def test_final_state_with_implementation_and_fix_shas_passes(self):
        output = self.run_check([row("M1", "`a1b2c3d`"), row("M2", "`b2c3d4e`；修复 `c3d4e5f`")])
        self.assertIn("WARN 0", output)

    def test_snapshot_colons_are_complete_characters_in_byte_locale(self):
        with mock.patch.dict(os.environ, {"LC_ALL": "C"}):
            self.assertIn("ERROR 0", self.run_check([row("M1", "待提交")]))
            self.plan.write_text(self.plan.read_text(encoding="utf-8").replace("：", ":"), encoding="utf-8")
            result = run_progress(SCRIPT, self.plan, self.dir, 10)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("ERROR 0", result.stdout)

    def test_ui_fix_sha_after_code_fix_passes(self):
        output = self.run_check([row("M1", "`a1b2c3d`"), row("M2", "`b2c3d4e`；修复 `c3d4e5f`；UI修复 `d4e5f6a`")])
        self.assertIn("WARN 0", output)

    def test_legacy_table_without_commit_column_only_warns(self):
        self.run_check([row("M1"), row("M2")], header=LEGACY_HEADER, diagnostic="没有「提交」列")

    def test_unknown_sha_warns_inside_git_repository(self):
        def git(*args):
            return subprocess.run(
                ["git", "-c", "user.name=t", "-c", "user.email=t@example.com", *args],
                cwd=self.dir, text=True, capture_output=True, check=True,
            ).stdout.strip()

        git("init", "-q")
        (self.dir / "a.txt").write_text("a\n", encoding="utf-8")
        git("add", "-A")
        git("commit", "-qm", "impl")
        sha = git("rev-parse", "--short", "HEAD")
        output = self.run_check([row("M1", sha), row("M2", "修复 `deadbee`")])
        self.assertIn("deadbee 在本地仓库找不到", output)
        self.assertNotIn(f"{sha} 在本地仓库找不到", output)


if __name__ == "__main__":
    unittest.main()
