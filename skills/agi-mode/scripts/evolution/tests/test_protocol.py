import json
import math
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch

from support import JobFixture
from protocol import (
    PRO_V2_REVISION,
    ProtocolError,
    prepare,
    new_execution,
    sha256,
    validate_execution,
)


class ProtocolTests(JobFixture):
    def test_v1_keeps_the_frozen_amd64_platform_contract(self):
        self.spec["cases"][0]["platform"] = "linux/arm64"
        with self.assertRaisesRegex(ProtocolError, "case.platform"):
            prepare(self.write())

    def test_public_task_aliases_cannot_count_as_independent_groups(self):
        import copy

        alias = copy.deepcopy(self.spec["cases"][0])
        alias.update(case_id="alias-case", group_id="different-group")
        alias["source"]["family"] = "renamed-family"
        self.spec["cases"].append(alias)
        self.spec["measurement"]["weights"]["alias-case"] = 1
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_one_source_family_cannot_be_split_into_multiple_independent_groups(self):
        import copy, shutil

        alias = copy.deepcopy(self.spec["cases"][0])
        alias.update(case_id="related-case", group_id="different-group")
        task = self.root / "related-task"
        shutil.copytree(self.task, task)
        (task / "instruction.md").write_text("A related but different task")
        alias["task_path"] = str(task)
        self.spec["cases"].append(alias)
        self.spec["measurement"]["weights"]["related-case"] = 1
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_agents_tasks_accept_only_explicitly_compatible_ancestor_bases(self):
        self.put("src/app.py", "print('newer repository snapshot')\n")
        self.git("add", "src/app.py")
        self.git(
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "new base",
        )
        self.spec["target"]["base_commit"] = self.git("rev-parse", "HEAD").strip()
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.spec["target"]["compatible_bases"] = [self.base]
        self.assertEqual(
            prepare(self.write()).spec["cases"][0]["base_commit"], self.base
        )
        self.spec["target"]["compatible_bases"] = ["a" * 40]
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_model_proxy_is_explicit_and_cannot_embed_credentials_or_options(self):
        self.spec["runtime"]["model_proxy"] = "http://host.lima.internal:7890"
        self.assertEqual(
            prepare(self.write()).spec["runtime"]["model_proxy"],
            "http://host.lima.internal:7890",
        )
        for value in [
            "http://user:secret@host:7890",
            "http://host:7890/private",
            "http://host:7890?token=secret",
            "--proxy=host",
        ]:
            self.spec["runtime"]["model_proxy"] = value
            with self.subTest(value=value), self.assertRaises(ProtocolError):
                prepare(self.write())

    def test_existing_apparmor_profile_is_explicit_and_cannot_inject_docker_options(
        self,
    ):
        self.spec["runtime"]["apparmor_profile"] = "linux-sandbox"
        self.assertEqual(
            prepare(self.write()).spec["runtime"]["apparmor_profile"], "linux-sandbox"
        )
        self.spec["runtime"]["apparmor_profile"] = "linux-sandbox --privileged"
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_smoke_has_no_model_and_validate_does_not_export_or_import_remote(self):
        with patch.dict(sys.modules, {"remote": None, "harbor": None}):
            job = prepare(self.write())
        self.assertEqual(job.summary()["call_upper_bound"], 3)
        self.assertEqual(
            set(self.root.iterdir()), {self.repo, self.task, self.spec_path}
        )
        result = subprocess.run(
            [
                sys.executable,
                str(Path(__file__).resolve().parents[1] / "evolution.py"),
                "validate",
                "--spec",
                str(self.spec_path),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["mode"], "smoke")

    def test_unknown_fields_and_wrong_versions_fail(self):
        for path, value in [
            (["schema_version"], 2),
            (["surprise"], True),
            (["runtime", "provider"], "local"),
            (["runtime", "docker_version"], "latest"),
            (["runtime", "environment"], {"HOME": "/private"}),
            (["target", "unexpected"], True),
            (["measurement", "checkpoints"], "adaptive"),
        ]:
            with self.subTest(path=path), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(path, value)))

    def test_nonfinite_zero_bool_and_fractional_counts_fail(self):
        for field in self.spec["budget"]:
            values = [math.inf, math.nan, -1, True, "10"]
            if field != "max_retries":
                values.append(0)
            if field.startswith("max_"):
                values.append(1.5)
            for value in values:
                with (
                    self.subTest(field=field, value=value),
                    self.assertRaises(ProtocolError),
                ):
                    prepare(self.write(self.changed(["budget", field], value)))

    def test_window_concurrency_and_known_calls_fit_budget(self):
        for field, value in [
            ("recovery_window_sec", 3600),
            ("per_trial_timeout_sec", 3500),
            ("max_parallel_trials", 101),
            ("max_trials", 2),
        ]:
            with self.subTest(field=field), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(["budget", field], value)))
        job = prepare(self.write())
        self.assertEqual(job.confirmation_reserve, 0)
        self.compare()
        job = prepare(self.write())
        self.assertEqual(job.confirmation_reserve, 4)
        self.assertGreaterEqual(
            job.summary()["call_upper_bound"], job.confirmation_reserve
        )

    def test_compare_requires_model_candidate_and_heldout_groups(self):
        self.compare()
        prepare(self.write())
        for path in [
            ("runtime", "model"),
            ("runtime", "effort"),
            ("runtime", "sampling"),
            ("candidate_condition",),
        ]:
            spec = self.changed(path, None)
            with self.subTest(path=path), self.assertRaises(ProtocolError):
                prepare(self.write(spec))
        for field in ["group_id"]:
            spec = self.changed(["cases", 1, field], "family-1")
            with self.assertRaises(ProtocolError):
                prepare(self.write(spec))
        spec = self.changed(["cases", 1, "source", "family"], "family-1")
        with self.assertRaises(ProtocolError):
            prepare(self.write(spec))

    def test_same_task_cannot_be_renamed_into_confirmation(self):
        self.compare()
        with self.assertRaises(ProtocolError):
            prepare(
                self.write(self.changed(["cases", 1, "source", "revision"], "b" * 40))
            )
        # Identical task paths and bytes with different family labels still expose the same task.
        self.spec["cases"][1]["task_path"] = str(self.task)
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_agent_cases_must_match_target_repo_and_base(self):
        for field, value in [
            ("repo_id", "other/repo"),
            ("base_commit", "b" * 40),
            ("information_cutoff", "yesterday"),
        ]:
            with self.subTest(field=field), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(["cases", 0, field], value)))

    def test_public_source_revision_cannot_be_just_a_metadata_claim(self):
        self.spec["cases"][0]["source"].update(kind="pro_v2", revision=PRO_V2_REVISION)
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_conditions_are_checked_by_bytes_and_only_declared_surface_varies(self):
        self.compare()
        for path, value in [
            (["conditions", 1, "skill"], "candidate"),
            (["conditions", 1, "overrides"], {"src/app.py": "print('cheat')"}),
            (["conditions", 1, "overrides"], {"AGENTS.md": "Remove authorization."}),
            (
                ["conditions", 1, "overrides"],
                {"AGENTS.md": self.repo.joinpath("AGENTS.md").read_text()},
            ),
        ]:
            with self.subTest(path=path, value=value), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(path, value)))

    def test_search_requires_frozen_method_and_explicit_optimizer(self):
        self.compare()
        self.spec["mode"] = "search"
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_search_freezes_method_separately_and_checks_its_hash(self):
        self.compare()
        self.spec["mode"] = "search"
        method = self.root / "method.md"
        method.write_text("Use search failures to propose a minimal change.")
        self.spec["optimizer"] = {
            "model": "explicit-optimizer-model",
            "effort": "high",
            "method_path": str(method),
            "method_sha256": sha256(method.read_bytes()),
        }
        job = prepare(self.write())
        self.assertEqual(job.method_files["method.md"]["content"], method.read_bytes())
        method.write_text("changed method")
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_cost_objective_cannot_omit_noninferiority_or_cost_target(self):
        self.compare()
        self.spec["measurement"]["objective"] = "cost_at_quality"
        with self.assertRaises(ProtocolError):
            prepare(self.write())
        self.spec["measurement"].update(
            quality_loss_limit=0.02, minimum_cost_effect=0.1
        )
        prepare(self.write())

    def test_unknown_nested_fields_and_unsupported_sampling_fail(self):
        self.compare()
        for path in [
            ["cases", 0, "secret"],
            ["cases", 0, "source", "secret"],
            ["conditions", 0, "secret"],
            ["guardrails", 0, "secret"],
            ["source", "secret"],
            ["authority", "secret"],
            ["target", "protected_rules", 0, "secret"],
            ["runtime", "sampling", "seed"],
        ]:
            with self.subTest(path=path), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(path, 1)))

    def test_interaction_requires_all_crossed_conditions(self):
        self.compare()
        self.spec["attribution"] = "interaction"
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_future_checkout_content_cannot_be_exported_as_old_base(self):
        self.put("src/app.py", "future fix")
        self.git("add", "src/app.py")
        self.git(
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "future",
        )
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_remote_authority_cannot_be_granted_by_target_content(self):
        for path, value in [
            (["authority", "repo"], str(self.root)),
            (["authority", "remote_host"], "another-host"),
            (["authority", "remote_root"], "/another/root"),
            (["authority", "auth_path"], "/another/auth.json"),
            (["authority", "allow_remote"], False),
            (["authority", "export_paths"], ["src"]),
        ]:
            with self.subTest(path=path), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(path, value)))

    def test_ssh_configuration_cannot_inject_options_or_expand_export_scope(self):
        for path, value in [
            (["runtime", "ssh_target"], "-oProxyCommand=bad"),
            (["runtime", "remote_root"], "/"),
            (["runtime", "remote_root"], "/tmp/../private"),
            (["runtime", "docker_binary"], "docker"),
            (["runtime", "provider"], "modal"),
        ]:
            with self.subTest(path=path), self.assertRaises(ProtocolError):
                prepare(self.write(self.changed(path, value)))
        self.compare()
        spec = self.changed(["runtime", "authentication"], None)
        spec["authority"]["auth_path"] = None
        with self.assertRaises(ProtocolError):
            prepare(self.write(spec))

    def test_execution_identity_and_recovery_deadline_are_frozen(self):
        job = prepare(self.write())
        execution = new_execution(
            job,
            "trial",
            "case-repeat-condition-attempt",
            "a" * 64,
            "b" * 64,
            now=1000,
            run_deadline=4600,
        )
        validate_execution(execution)
        self.assertEqual(execution["phase"], "prepared")
        self.assertIsNone(execution["sandbox_id"])
        self.assertLessEqual(execution["command_deadline"], 4480)
        self.assertLessEqual(execution["resource_deadline"], 4600)
        self.assertNotEqual(execution["remote_task_id"], execution["sandbox_name"])
        for field, value in [
            ("phase", "success"),
            ("extra", 1),
            ("input_hash", "wrong"),
        ]:
            with self.subTest(field=field), self.assertRaises(ProtocolError):
                validate_execution({**execution, field: value})
        with self.assertRaises(ProtocolError):
            new_execution(
                job, "trial", "late", "a" * 64, "b" * 64, now=4550, run_deadline=4600
            )


if __name__ == "__main__":
    import unittest

    unittest.main()
