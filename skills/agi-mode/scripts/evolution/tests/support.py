import copy
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


class JobFixture(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.git("init", "-q")
        self.put("AGENTS.md", "Preserve authorization.\nRun the project tests.\n")
        self.put("src/app.py", "print('base')\n")
        self.put(
            "skills/debugging/SKILL.md",
            "---\nname: debugging\ndescription: Diagnose bugs\n---\nFind the root cause.\n",
        )
        self.git("add", ".")
        self.git(
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "base",
        )
        self.base = self.git("rev-parse", "HEAD").strip()
        self.task = self.root / "task"
        self.task.mkdir()
        for name, content in {
            "instruction.md": "Fix the bug using the public contract.",
            "environment/Dockerfile": "FROM python:3.12\n",
            "tests/test.sh": "private grading command",
            "solution/solve.sh": "private oracle patch",
        }.items():
            path = self.task / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
        self.spec = {
            "schema_version": 1,
            "protocol_id": "fixture-v1",
            "question": "Does the repository instruction improve task success?",
            "mode": "smoke",
            "target": {
                "kind": "agents",
                "repo": str(self.repo),
                "repo_id": "fixture/repo",
                "base_commit": self.base,
                "allowed_paths": ["AGENTS.md"],
                "surfaces": {"skill": [], "agents": ["AGENTS.md"]},
                "protected_rules": [
                    {
                        "path": "AGENTS.md",
                        "text": "Preserve authorization.",
                        "kind": "authority",
                        "verification": "exact",
                    }
                ],
            },
            "source": {"paths": ["AGENTS.md", "src", "skills"], "new_paths": []},
            "runtime": {
                "provider": "ssh",
                "ssh_target": "test@evaluation-host",
                "remote_root": "/tmp/evolution",
                "docker_binary": "/usr/bin/docker",
                "docker_version": "29.5.2",
                "codex_architecture": "x86_64",
                "authentication": None,
                "harbor_version": "0.24.0",
                "codex_version": "0.162.1",
            },
            "authority": {
                "repo": str(self.repo),
                "export_paths": ["AGENTS.md", "src", "skills"],
                "write_paths": ["AGENTS.md"],
                "remote_host": "test@evaluation-host",
                "remote_root": "/tmp/evolution",
                "auth_path": None,
                "allow_remote": True,
                "allow_apply": False,
                "allow_unverified": False,
            },
            "budget": {
                "max_candidates": 1,
                "max_trials": 100,
                "max_parallel_trials": 1,
                "per_trial_timeout_sec": 60,
                "total_deadline_sec": 3600,
                "recovery_window_sec": 120,
                "max_retries": 0,
            },
            "measurement": {
                "objective": "quality",
                "primary_metric": "task_success",
                "minimum_effect": 0.05,
                "confidence_level": 0.95,
                "minimum_groups": 5,
                "repeats": 1,
                "seed": 7,
                "checkpoint": "fixed",
                "confirmation": "once",
                "weights": {"search-case": 1},
            },
            "guardrails": [{"name": "authorization", "minimum": 1}],
            "cases": [self.case("search-case", "family-1", "search")],
            "conditions": [
                {
                    "condition_id": "baseline",
                    "skill": "fixed",
                    "agents": "baseline",
                    "overrides": {},
                }
            ],
            "baseline_condition": "baseline",
        }
        self.spec_path = self.root / "job.json"

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.repo), *args], text=True)

    def put(self, name, content):
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def case(self, name, family, split):
        return {
            "case_id": name,
            "group_id": family,
            "repo_id": "fixture/repo",
            "base_commit": self.base,
            "split": split,
            "task_path": str(self.task),
            "repo_path": "/app",
            "source": {"kind": "repo", "revision": self.base, "family": family},
            "public_paths": ["instruction.md", "environment/Dockerfile"],
            "grader_paths": ["tests/test.sh"],
            "oracle_paths": ["solution/solve.sh"],
            "image": "registry.example/task@sha256:" + "a" * 64,
            "observable_requirements": ["The public bug reproducer passes."],
            "information_cutoff": "2026-10-01T00:00:00+00:00",
        }

    def write(self, spec=None):
        self.spec_path.write_text(json.dumps(self.spec if spec is None else spec))
        return self.spec_path

    def compare(self):
        self.spec["mode"] = "compare"
        self.spec["runtime"].update(
            model="explicit-test-model",
            effort="high",
            tools=["shell"],
            sampling={"seed": None},
            authentication={
                "mode": "remote_codex_login",
                "path": "/home/test/.codex/auth.json",
            },
        )
        self.spec["authority"]["auth_path"] = "/home/test/.codex/auth.json"
        self.spec["conditions"].append(
            {
                "condition_id": "candidate",
                "skill": "fixed",
                "agents": "candidate",
                "overrides": {
                    "AGENTS.md": "Preserve authorization.\nRun tests before delivery.\n"
                },
            }
        )
        self.spec["candidate_condition"] = "candidate"
        self.spec["cases"].append(
            self.case("confirmation-case", "family-2", "confirmation")
        )
        heldout = self.root / "heldout"
        shutil.copytree(self.task, heldout)
        (heldout / "instruction.md").write_text("Fix a different heldout bug.")
        self.spec["cases"][1]["task_path"] = str(heldout)
        self.spec["measurement"]["weights"]["confirmation-case"] = 1
        return self.spec

    def changed(self, path, value):
        spec = copy.deepcopy(self.spec)
        node = spec
        for key in path[:-1]:
            node = node[key]
        node[path[-1]] = value
        return spec
