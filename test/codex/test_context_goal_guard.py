from contextlib import closing
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import unittest
import uuid


SCRIPT = Path(__file__).resolve().parents[2] / "hooks/codex/context-goal-guard.py"


def row(kind, payload):
    return {"type": kind, "payload": payload}


def usage(used=6501, window=10000):
    return row("event_msg", {"type": "token_count", "info": {
        "last_token_usage": {"total_tokens": used, "cached_input_tokens": 6000,
                             "reasoning_output_tokens": 100},
        "total_token_usage": {"total_tokens": 999999999},
        "model_context_window": window,
    }})


class GuardTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="Guard Home # ")
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.sessions = self.home / "sessions"
        self.sessions.mkdir()
        self.transcript = self.sessions / "session.jsonl"
        self.thread = str(uuid.uuid4())
        self.meta = {"id": self.thread, "session_id": self.thread,
                     "source": "vscode", "cli_version": "0.160.0"}
        self.context = row("turn_context", {"model": "test-model"})
        self.input = {"hook_event_name": "PreToolUse", "session_id": self.thread,
                      "transcript_path": str(self.transcript), "model": "test-model"}
        self.db = self.home / "goals_1.sqlite"
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("CREATE TABLE thread_goals (thread_id TEXT PRIMARY KEY, status TEXT, objective TEXT)")
            con.execute("INSERT INTO thread_goals VALUES (?, 'active', 'PRIVATE_GOAL')", (self.thread,))
        self.write()

    def write(self, records=None, tail=b""):
        records = records if records is not None else [self.context, usage()]
        self.transcript.write_bytes(
            b"".join((json.dumps(r) + "\n").encode() for r in
                     [row("session_meta", self.meta), *records]) + tail)

    def status(self, value):
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("UPDATE thread_goals SET status=?", (value,))

    def run_guard(self, data=None, env=None, raw=None):
        environment = dict(os.environ, CODEX_HOME=str(self.home))
        if env:
            environment.update(env)
        process = subprocess.run(["python3", str(SCRIPT)],
                                 input=raw if raw is not None else json.dumps(data or self.input),
                                 text=True, capture_output=True, env=environment,
                                 cwd=self.home, timeout=3)
        self.assertEqual(process.returncode, 0, process.stderr)
        self.assertEqual(process.stderr, "")
        return json.loads(process.stdout)

    def warning(self, result):
        self.assertIn("检测不可用", result["systemMessage"])
        self.assertIn("/hooks", result["systemMessage"])
        self.assertNotIn("decision", result)

    def test_threshold_and_usage_accounting(self):
        for used, trigger in [(6499, False), (6500, False), (6501, True), (1000, False)]:
            with self.subTest(used=used):
                self.write([self.context, usage(used)])
                result = self.run_guard()
                if trigger:
                    output = result["hookSpecificOutput"]
                    self.assertEqual(output["hookEventName"], "PreToolUse")
                    self.assertIn("65.01%", output["additionalContext"])
                    self.assertNotIn("permissionDecision", output)
                else:
                    self.assertEqual(result, {})

    def test_goal_checked_before_usage(self):
        self.write([])
        for state in ["paused", "complete", "blocked", "budget_limited"]:
            with self.subTest(state=state):
                self.status(state)
                self.assertEqual(self.run_guard(), {})
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("DELETE FROM thread_goals")
        self.assertEqual(self.run_guard(), {})

    def test_stop_request_failure_and_paused_noop(self):
        data = dict(self.input, hook_event_name="Stop", stop_hook_active=False)
        result = self.run_guard(data)
        self.assertEqual(result["decision"], "block")
        self.assertIn("显式", result["reason"])
        result = self.run_guard(dict(data, stop_hook_active=True))
        self.assertIn("/goal pause", result["systemMessage"])
        self.assertNotIn("decision", result)
        self.assertNotIn("已暂停", result["systemMessage"])
        self.status("paused")
        self.assertEqual(self.run_guard(data), {})

    def test_reminder_privacy_and_handoff(self):
        self.write([self.context, row("response_item", {"text": "PRIVATE_TRANSCRIPT"}), usage()])
        result = self.run_guard()
        message = result["hookSpecificOutput"]["additionalContext"]
        for word in ["已有", "交接", "显式", "paused", "新开对话", "没有", "进程"]:
            self.assertIn(word, message)
        self.assertNotIn("PRIVATE", json.dumps(result))
        self.assertNotIn("continue", result)

    def test_no_task_or_database_writes(self):
        (self.home / "progress.md").write_text("existing progress")
        before = {p.relative_to(self.home): p.read_bytes() for p in self.home.rglob("*") if p.is_file()}
        self.run_guard()
        self.run_guard(dict(self.input, hook_event_name="Stop", stop_hook_active=False))
        after = {p.relative_to(self.home): p.read_bytes() for p in self.home.rglob("*") if p.is_file()}
        self.assertEqual(before, after)
        with closing(sqlite3.connect(self.db)) as con, con:
            self.assertEqual(con.execute("SELECT * FROM thread_goals").fetchall(),
                             [(self.thread, "active", "PRIVATE_GOAL")])

    def test_fork_uses_own_goal(self):
        parent = str(uuid.uuid4())
        self.meta["forked_from_id"] = parent
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("INSERT INTO thread_goals VALUES (?, 'active', 'parent')", (parent,))
        self.status("paused")
        self.write()
        self.assertEqual(self.run_guard(), {})
        self.status("active")
        self.assertIn("hookSpecificOutput", self.run_guard())

    def test_child_never_uses_parent_goal(self):
        self.meta.update(id=str(uuid.uuid4()), parent_thread_id=self.thread,
                         source={"subagent": {"thread_spawn": {"parent_thread_id": self.thread}}})
        self.write()
        self.assertEqual(self.run_guard(), {})
        self.assertEqual(self.run_guard(dict(self.input, agent_id=self.meta["id"])), {})

    def test_inherited_thread_environment_is_not_authoritative(self):
        self.assertIn("hookSpecificOutput", self.run_guard(env={
            "CODEX_THREAD_ID": str(uuid.uuid4()), "CODEX_SESSION_ID": str(uuid.uuid4())}))

    def test_identity_and_format_mismatches(self):
        for key, value in [("id", str(uuid.uuid4())), ("session_id", str(uuid.uuid4())),
                           ("source", "unknown-new-source"), ("cli_version", "9.9.9")]:
            with self.subTest(key=key):
                old = self.meta[key]
                self.meta[key] = value
                self.write()
                self.warning(self.run_guard())
                self.meta[key] = old
        self.write()
        outside = self.home / "other.jsonl"
        outside.write_bytes(self.transcript.read_bytes())
        self.warning(self.run_guard(dict(self.input, transcript_path=str(outside))))

    def test_bad_input_and_missing_transcript(self):
        for raw in ["{", "[]", "null", "{}"]:
            with self.subTest(raw=raw):
                self.warning(self.run_guard(raw=raw))
        self.transcript.unlink()
        self.warning(self.run_guard())

    def test_unknown_metadata_payload_and_empty_home(self):
        for value in [None, [], "new format"]:
            with self.subTest(value=value):
                self.transcript.write_text(json.dumps(row("session_meta", value)) + "\n")
                self.warning(self.run_guard())
        self.write()
        self.warning(self.run_guard(env={"CODEX_HOME": ""}))

    def test_changed_context_identity(self):
        old = row("turn_context", {"model": "test-model", "context_window": {"window_id": "old"}})
        new = row("turn_context", {"model": "test-model", "context_window": {"window_id": "new"}})
        self.write([old, usage(), new])
        self.warning(self.run_guard())
        self.write([old, usage(), new, usage(1)])
        self.assertEqual(self.run_guard(), {})

    def test_unreadable_database(self):
        self.db.chmod(0)
        self.addCleanup(self.db.chmod, 0o600)
        self.warning(self.run_guard())

    def test_bad_statistics_do_not_fall_back(self):
        for used, window in [(True, 10000), (-1, 10000), (1.5, 10000),
                             ("6501", 10000), (None, 10000), (6501, 0),
                             (6501, False), (6501, -1)]:
            with self.subTest(used=used, window=window):
                self.write([self.context, usage(), usage(used, window)])
                self.warning(self.run_guard())
        self.write([self.context, usage(), row("event_msg", {"type": "token_count", "info": None})])
        self.warning(self.run_guard())

    def test_latest_complete_line_and_large_transcript(self):
        self.write([self.context, usage()], tail=b'{"type":"event_msg","payload":')
        self.assertIn("hookSpecificOutput", self.run_guard())
        self.write([self.context, usage(), usage(1)])
        self.assertEqual(self.run_guard(), {})
        self.write([self.context, *[row("response_item", {"text": "x" * 8192})] * 512, usage()])
        self.assertIn("hookSpecificOutput", self.run_guard())
        self.write([self.context, usage()], tail=b"corrupt complete line\n")
        self.warning(self.run_guard())

    def test_compaction_and_model_switch_invalidate_old_statistics(self):
        markers = [row("compacted", {}), row("event_msg", {"type": "context_compacted"}),
                   row("turn_context", {"model": "new-model"}),
                   row("turn_context", {"model": "test-model", "model_context_window": 20000})]
        for marker in markers:
            with self.subTest(marker=marker):
                self.write([self.context, usage(), marker])
                self.warning(self.run_guard())
        self.write([self.context, usage(), row("compacted", {}), self.context, usage(1)])
        self.assertEqual(self.run_guard(), {})
        self.write([self.context, usage(), self.context])
        self.assertIn("hookSpecificOutput", self.run_guard())

    def test_database_failures_never_create_or_modify_database(self):
        self.db.unlink()
        self.warning(self.run_guard())
        self.assertFalse(self.db.exists())
        self.db.write_bytes(b"not a database")
        self.warning(self.run_guard())
        self.assertEqual(self.db.read_bytes(), b"not a database")
        self.db.unlink()
        with closing(sqlite3.connect(self.db)) as con, con:
            con.execute("CREATE TABLE incompatible (value TEXT)")
        self.warning(self.run_guard())

    def test_locked_database(self):
        con = sqlite3.connect(self.db)
        self.addCleanup(con.close)
        con.execute("BEGIN EXCLUSIVE")
        self.warning(self.run_guard())

    def test_default_home_and_stop_validation(self):
        self.warning(self.run_guard(dict(self.input, hook_event_name="Stop", stop_hook_active="false")))
        # Override HOME only in this isolated child to exercise Codex's default location.
        default = self.home / ".codex"
        default.mkdir()
        self.sessions.rename(default / "sessions")
        self.db.rename(default / "goals_1.sqlite")
        data = dict(self.input, transcript_path=str(default / "sessions/session.jsonl"))
        env = dict(os.environ, HOME=str(self.home))
        env.pop("CODEX_HOME", None)
        result = subprocess.run(["python3", str(SCRIPT)], input=json.dumps(data),
                                env=env, capture_output=True, text=True, timeout=3)
        self.assertEqual(result.returncode, 0)
        self.assertIn("hookSpecificOutput", json.loads(result.stdout))


if __name__ == "__main__":
    unittest.main()
