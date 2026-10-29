import json
import base64
import hashlib
import io
from copy import deepcopy
import os
from pathlib import Path
import subprocess
import sys
import threading
import tarfile
import time
import unittest
from unittest.mock import patch

from support import JobFixture
from test_quality import QualityFixture
from evolution import write_json
from protocol import canonical, new_execution, prepare, sha256
from remote import (
    inspect_command,
    run_supervisor,
    capture_only,
    RemoteClient,
    UnresolvedError,
    trial_payload,
    grading_payload,
    instruction_inventory,
    tooling_manifest,
)


class CodexFilesystemProfileTests(unittest.TestCase):
    def test_v2_private_globs_are_directory_anchored_and_survive_cli_toml(self):
        import toml, tomllib
        from remote import codex_arguments, codex_configuration
        configuration = codex_configuration(schema_version=2)
        filesystem = configuration["permissions"]["evolution"]["filesystem"]
        root = "/home/evolution/"
        for suffix in ("auth*", "*.json*", "config*.toml", "*.sqlite*", ".tmp*"):
            self.assertEqual(filesystem[root + "[.]codex/" + suffix], "deny")
            self.assertNotIn(root + ".codex/" + suffix, filesystem)
        for path in (".env", "sessions", "archived_sessions", "log"):
            self.assertEqual(filesystem[root + ".codex/" + path], "deny")
        self.assertEqual(filesystem[root + ".codex/proxy/**"], "read")
        self.assertNotIn(root + ".codex", filesystem)
        args = codex_arguments(configuration, "/work", probe="pass")
        encoded = next(arg for arg in args if arg.startswith("permissions="))
        self.assertEqual(tomllib.loads(encoded)["permissions"]["evolution"]["filesystem"], filesystem)
        self.assertEqual(tomllib.loads(toml.dumps(configuration)), configuration)
        legacy = codex_configuration(schema_version=1)
        self.assertEqual(legacy, codex_configuration())
        self.assertEqual(legacy["permissions"]["evolution"]["filesystem"][root + ".codex"], "deny")


class FixedSkillCatalogTests(QualityFixture):
    def setUp(self):
        super().setUp()
        self.quality_spec()
        self.put("skills/fixed-helper/SKILL.md", "---\nname: fixed-helper\ndescription: Fixed context helper\n---\nRead reference.md.\n")
        self.put("skills/fixed-helper/reference.md", "Fixed public reference.\n")
        self.spec["source"]["new_paths"] = ["skills/fixed-helper/SKILL.md", "skills/fixed-helper/reference.md"]
        self.spec["source"]["fixed_skill_roots"] = ["skills/fixed-helper"]
        self.spec["target"].update(kind="skill", allowed_paths=["skills/debugging/SKILL.md"],
                                   surfaces={"skill": ["skills/debugging/SKILL.md"], "agents": []})
        self.spec["target"]["protected_rules"] = [{"path": "skills/debugging/SKILL.md",
            "text": "Find the root cause.", "kind": "principle", "verification": "exact"}]
        self.spec["authority"]["write_paths"] = []
        self.spec["conditions"] = [
            {"condition_id": "baseline", "skill": "baseline", "agents": "fixed", "overrides": {}},
            {"condition_id": "absent", "skill": "absent", "agents": "fixed",
             "overrides": {"skills/debugging/SKILL.md": None}},
        ]

    def test_fixed_catalog_is_loaded_and_preserved_when_target_is_absent_or_changed(self):
        from remote import smoke_payload
        job = prepare(self.write())
        case = job.spec["cases"][0]
        probe = smoke_payload(job, case, "probe")
        original = trial_payload(job, case, job.bundles["baseline"])
        absent = trial_payload(job, case, job.bundles["absent"])
        candidate = deepcopy(job.bundles["baseline"])
        candidate["skills/debugging/SKILL.md"]["content"] += b"Check the documented boundaries.\n"
        changed = trial_payload(job, case, candidate)
        prefix = "/home/evolution/.agents/skills/fixed-helper/"
        expected = {key: value for key, value in probe["files"].items() if key.startswith(prefix)}
        self.assertEqual(set(expected), {prefix + "SKILL.md", prefix + "reference.md"})
        for payload in [original, absent, changed]:
            self.assertEqual({key: value for key, value in payload["files"].items() if key.startswith(prefix)}, expected)
        target = "/home/evolution/.agents/skills/debugging/SKILL.md"
        self.assertIn(target, original["files"])
        self.assertNotIn(target, absent["files"])
        self.assertIn("/app/skills/debugging/SKILL.md", absent["remove_paths"])
        self.assertNotEqual(changed["files"][target], original["files"][target])

    def test_fixed_catalog_requires_exported_disjoint_unique_skill_roots(self):
        from protocol import ProtocolError
        for roots in [["skills/not-exported"], ["skills/debugging"], ["../outside"], True]:
            self.spec["source"]["fixed_skill_roots"] = roots
            with self.subTest(roots=roots), self.assertRaises(ProtocolError):
                prepare(self.write())
        self.spec["source"]["fixed_skill_roots"] = ["skills/fixed-helper"]
        self.put("skills/fixed-helper/SKILL.md", "---\nname: debugging\ndescription: Duplicate fixed identity\n---\nFixed helper.\n")
        with self.assertRaises(ProtocolError):
            prepare(self.write())

    def test_fixed_catalog_is_explicit_and_also_available_to_unscored_material_runs(self):
        from remote import smoke_payload
        self.spec["schema_version"] = 1
        self.spec["mode"] = "smoke"
        self.spec.pop("evaluation")
        self.spec["authority"].pop("evaluation_root")
        self.spec["measurement"].update(primary_metric="task_success")
        self.spec["measurement"].pop("minimum_effect_reason")
        job = prepare(self.write())
        path = "/home/evolution/.agents/skills/fixed-helper/SKILL.md"
        self.assertIn(path, smoke_payload(job, job.spec["cases"][0], "probe")["files"])
        self.spec["source"].pop("fixed_skill_roots")
        original = prepare(self.write())
        self.assertNotIn(path, smoke_payload(original, original.spec["cases"][0], "probe")["files"])


class PayloadTests(JobFixture):
    def test_v2_probe_rejects_unhealthy_installed_npm_and_allows_other_images(self):
        from contextlib import redirect_stdout
        import shutil
        import socket
        from remote import isolation_script

        executable = self.root / "npm"
        read_bytes, write_text, exists = Path.read_bytes, Path.write_text, Path.exists
        connect = socket.create_connection

        def protected_read(path):
            if str(path) in {"/home/evolution/.codex/auth.json", "/evolution-control/receipt.json"}:
                raise PermissionError(path)
            return read_bytes(path)

        def protected_write(path, *args, **kwargs):
            if str(path) == "/evolution-control/receipt.json":
                raise PermissionError(path)
            return write_text(path, *args, **kwargs)

        def visible(path):
            if str(path) in {"/tests/test.sh", "/solution/gold_patch.diff"}:
                return False
            return exists(path)

        def isolated_connect(address, *args, **kwargs):
            if address == ("github.com", 443):
                raise OSError("blocked by the fixture")
            return connect(address, *args, **kwargs)

        for status, version, expected in [(None, "", True), (0, "11.6.2", True),
                                           (243, "", False), (0, "", False)]:
            executable.write_text(f"#!/bin/sh\nprintf '%s' '{version}'\nexit {status or 0}\n")
            executable.chmod(0o755)
            output = io.StringIO()
            with self.subTest(status=status, version=version), patch.dict(os.environ, {"PATH": os.defpath}, clear=True), \
                    patch("os.getuid", return_value=10000), \
                    patch.object(Path, "read_bytes", protected_read), \
                    patch.object(Path, "write_text", protected_write), \
                    patch.object(Path, "exists", visible), \
                    patch.object(socket, "create_connection", isolated_connect), \
                    patch.object(shutil, "which", return_value=None if status is None else str(executable)), \
                    redirect_stdout(output), self.assertRaises(SystemExit) as stopped:
                exec(isolation_script({"repo_path": str(self.repo), "base_commit": self.base}, schema_version=2), {})
            observation = json.loads(output.getvalue())
            self.assertEqual(observation["checks"]["installed_npm_ready"], expected)
            self.assertEqual(observation["isolation_passed"], expected)
            self.assertEqual(stopped.exception.code, 0 if expected else 1)

    def test_missing_node_installs_the_verified_native_archive(self):
        import toml
        from remote import smoke_payload

        job = prepare(self.write())
        payload = smoke_payload(job, job.spec["cases"][0], "probe")
        client = RemoteClient(job.spec["runtime"])
        commands = []

        def rpc(operation, execution, packet=None):
            if operation == "restore_codex":
                return {"restored": False}
            if operation == "setup_exec":
                commands.append(packet["command"])
                return {
                    "return_code": 1
                    if packet["command"].startswith("command -v node")
                    else 0,
                    "stdout": "",
                    "stderr": "",
                }
            if operation in {"setup_files", "cache_codex"}:
                return {}
            if operation == "finalize_setup":
                return {
                    "config_sha256": sha256(
                        toml.dumps(payload["codex_config"]).encode()
                    )
                }
            self.fail(operation)

        client._rpc = rpc
        client.prepare_codex({"command_deadline": time.time() + 120}, payload)
        self.assertTrue(any("install_codex_archive" in command for command in commands))
        self.assertFalse(any("npm install" in command for command in commands))

    def test_repository_preparation_can_finish_after_sixty_seconds_within_its_budget(
        self,
    ):
        from remote import create_resource, encoded, digest, smoke_payload

        job = prepare(self.write())
        payload = smoke_payload(job, job.spec["cases"][0], "probe")
        execution = new_execution(
            job,
            "probe",
            "slow-preparation",
            digest(encoded(payload["command"])),
            "a" * 64,
            now=time.time(),
            run_deadline=time.time() + 300,
        )
        execution["input_hash"] = digest(encoded(payload))
        execution["command_deadline"] = time.time() + 120

        def container_exec(runtime, receipt, args, **kwargs):
            if "base-tree.txt" in args[-1]:
                timeout = kwargs.get("timeout", 60)
                if timeout < 75:
                    raise subprocess.TimeoutExpired(args, timeout)
                self.assertLessEqual(timeout, 120)
            return subprocess.CompletedProcess(args, 0, b"{}", b"")

        resource = {"Image": "image-id", "Mounts": [], "AppArmorProfile": "unconfined"}
        with patch.multiple(
            "remote",
            host_control=lambda value: self.root / "remote-control",
            engine_identity=lambda runtime: "engine-id",
            docker=lambda *args, **kwargs: subprocess.CompletedProcess(
                [], 0, b"container-id", b""
            ),
            owned_resource=lambda *args: resource,
            container_files=lambda *args: None,
            container_exec=container_exec,
        ):
            result = create_resource(job.spec["runtime"], execution, payload)
        self.assertEqual(result["sandbox_id"], "container-id")

    def test_verified_native_cache_does_not_require_node_in_the_task_image(self):
        import toml
        from remote import smoke_payload

        job = prepare(self.write())
        payload = smoke_payload(job, job.spec["cases"][0], "probe")
        client = RemoteClient(job.spec["runtime"])
        commands = []

        def rpc(operation, execution, packet=None):
            if operation == "restore_codex":
                return {"restored": True}
            if operation == "setup_exec":
                command = packet["command"]
                commands.append(command)
                if command.startswith("command -v node"):
                    return {"return_code": 1, "stdout": "", "stderr": ""}
                self.assertNotIn("curl", command)
                self.assertNotIn("npm install", command)
                return {"return_code": 0, "stdout": "", "stderr": ""}
            if operation == "setup_files":
                return {"staged": True}
            if operation == "finalize_setup":
                return {
                    "config_sha256": sha256(
                        toml.dumps(payload["codex_config"]).encode()
                    )
                }
            self.fail(operation)

        client._rpc = rpc
        client.prepare_codex({}, payload)
        self.assertTrue(commands)
        self.assertFalse(
            any(command.startswith("command -v node") for command in commands)
        )

    def test_existing_proxy_routes_model_driver_and_is_not_inherited_by_tools(self):
        self.spec["runtime"]["model_proxy"] = "http://host.lima.internal:7890"
        job = prepare(self.write())
        payload = trial_payload(job, job.spec["cases"][0], job.bundles["baseline"])
        self.assertEqual(
            payload["command"]["env"]["HTTPS_PROXY"],
            self.spec["runtime"]["model_proxy"],
        )
        self.assertNotIn(
            "HTTPS_PROXY", payload["codex_config"]["shell_environment_policy"]["set"]
        )
        self.assertIs(payload["codex_config"]["features"]["apps"], False)

    def test_cached_tooling_tracks_bytes_modes_and_rejects_escape_links(self):
        directory = self.root / "tooling"
        directory.mkdir()
        tool = directory / "codex"
        tool.write_bytes(b"fixed binary")
        tool.chmod(0o755)
        (directory / "launcher").symlink_to("codex")
        first = tooling_manifest(directory)
        tool.write_bytes(b"different binary")
        self.assertNotEqual(tooling_manifest(directory), first)
        tool.write_bytes(b"fixed binary")
        tool.chmod(0o644)
        self.assertNotEqual(tooling_manifest(directory), first)
        (directory / "launcher").unlink()
        (directory / "launcher").symlink_to(self.repo / "AGENTS.md")
        with self.assertRaises(UnresolvedError):
            tooling_manifest(directory)

    def test_inventory_allows_tracked_background_and_injection_but_rejects_unknown_sources(
        self,
    ):
        home = self.root / "fresh-home"
        installed = home / ".agents/skills/debugging/SKILL.md"
        installed.parent.mkdir(parents=True)
        installed.write_text("---\nname: other-skill\n---\nDeclared injection.")
        expected = {str(installed): {"sha256": sha256(installed.read_bytes())}}
        receipt = instruction_inventory(self.repo, home, expected)
        self.assertFalse(receipt["gaps"])
        self.assertIn(
            str(self.repo / "AGENTS.md"), [row["path"] for row in receipt["sources"]]
        )
        unexpected = home / ".codex/AGENTS.override.md"
        unexpected.parent.mkdir(parents=True)
        unexpected.write_text("Unknown global rule")
        self.put("nested/AGENTS.md", "Untracked instruction")
        receipt = instruction_inventory(self.repo, home, expected)
        self.assertEqual(len(receipt["gaps"]), 2)

    def test_tracked_background_is_verified_across_runtime_ownership(self):
        home = self.root / "fresh-home"
        with patch.dict(os.environ, {"GIT_TEST_ASSUME_DIFFERENT_OWNER": "1",
                                     "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_NOSYSTEM": "1"}):
            ordinary = subprocess.run(["git", "-C", str(self.repo), "show", "HEAD:AGENTS.md"],
                                      capture_output=True)
            self.assertNotEqual(ordinary.returncode, 0)
            self.assertIn(b"dubious ownership", ordinary.stderr)
            receipt = instruction_inventory(self.repo, home, {})
            self.assertFalse(receipt["gaps"])
            background = next(row for row in receipt["sources"]
                              if row["path"] == str(self.repo / "AGENTS.md"))
            self.assertEqual(background["origin"], "repository_background")
            self.put("AGENTS.md", "Changed after the frozen commit")
            self.put("nested/AGENTS.md", "Untracked instruction")
            receipt = instruction_inventory(self.repo, home, {})
            self.assertEqual(set(receipt["gaps"]), {
                "undeclared_instruction:" + str(self.repo / "AGENTS.md"),
                "undeclared_instruction:" + str(self.repo / "nested/AGENTS.md"),
            })

    def test_inventory_rejects_another_readable_copy_of_the_target_skill(self):
        home = self.root / "fresh-home"
        installed = home / ".agents/skills/debugging/SKILL.md"
        installed.parent.mkdir(parents=True)
        installed.write_bytes((self.repo / "skills/debugging/SKILL.md").read_bytes())
        receipt = instruction_inventory(
            self.repo,
            home,
            {
                str(installed): {"sha256": sha256(installed.read_bytes())},
            },
        )
        self.assertIn(
            "duplicate_target_skill:" + str(self.repo / "skills/debugging/SKILL.md"),
            receipt["gaps"],
        )

    def test_trial_and_new_grader_receive_authorized_dirty_repository_bytes(self):
        self.put("src/app.py", "print('authorized dirty input')\n")
        job = prepare(self.write())
        case = job.spec["cases"][0]
        bundle = job.bundles["baseline"]
        for payload in [
            trial_payload(job, case, bundle),
            grading_payload(job, case, b""),
        ]:
            self.assertEqual(
                base64.b64decode(payload["files"]["/app/src/app.py"]["data"]),
                bundle["src/app.py"]["content"],
            )

    def test_removed_agents_file_is_removed_from_task_image(self):
        self.spec["target"]["protected_rules"] = []
        job = prepare(self.write())
        bundle = dict(job.bundles["baseline"])
        del bundle["AGENTS.md"]
        payload = trial_payload(job, job.spec["cases"][0], bundle)
        self.assertNotIn("/app/AGENTS.md", payload["files"])
        self.assertIn("/app/AGENTS.md", payload["remove_paths"])

    def test_skill_trial_does_not_install_target_twice_inside_its_own_repository(self):
        path = "skills/debugging/SKILL.md"
        self.spec["target"].update(
            kind="skill",
            allowed_paths=[path],
            surfaces={"skill": [path], "agents": []},
            protected_rules=[],
        )
        self.spec["authority"]["write_paths"] = [path]
        self.spec["conditions"][0].update(skill="baseline", agents="fixed")
        job = prepare(self.write())
        payload = trial_payload(job, job.spec["cases"][0], job.bundles["baseline"])
        self.assertIn(
            "/home/evolution/.agents/skills/debugging/SKILL.md", payload["files"]
        )
        self.assertNotIn("/app/" + path, payload["files"])
        self.assertIn("/app/" + path, payload["remove_paths"])


class ArchiveInstallTests(JobFixture):
    def distribution(self, name="package/vendor/codex", symbolic=False):
        content = b"fixed native executable"
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode="w:gz") as archive:
            member = tarfile.TarInfo(name)
            member.mode = 0o755
            if symbolic:
                member.type = tarfile.SYMTYPE
                member.linkname = "../../escape"
                archive.addfile(member)
            else:
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
        body = raw.getvalue()
        metadata = {
            "name": "@openai/codex",
            "version": "0.162.1-linux-arm64",
            "os": ["linux"],
            "cpu": ["arm64"],
            "dist": {
                "tarball": "https://registry.npmjs.org/@openai/codex/-/codex-0.162.1-linux-arm64.tgz",
                "integrity": "sha512-"
                + base64.b64encode(hashlib.sha512(body).digest()).decode(),
            },
        }
        return metadata, body, content

    def test_native_archive_preserves_verified_package_bytes_and_modes(self):
        from remote import install_codex_archive

        metadata, body, content = self.distribution()
        target = self.root / "native-package"
        with patch(
            "urllib.request.urlopen",
            side_effect=[io.BytesIO(json.dumps(metadata).encode()), io.BytesIO(body)],
        ):
            install_codex_archive(target, "0.162.1", "arm64")
        binary = target / "vendor/codex"
        self.assertEqual(binary.read_bytes(), content)
        self.assertEqual(binary.stat().st_mode & 0o777, 0o755)

    def test_bad_integrity_or_unsafe_archive_is_rejected(self):
        from remote import install_codex_archive, RemoteError

        for kind in ["integrity", "traversal", "symlink"]:
            with self.subTest(kind=kind):
                name = (
                    "package/../../escape"
                    if kind == "traversal"
                    else "package/vendor/codex"
                )
                metadata, body, unused = self.distribution(
                    name, symbolic=kind == "symlink"
                )
                if kind == "integrity":
                    body += b"changed"
                target = self.root / kind
                with patch(
                    "urllib.request.urlopen",
                    side_effect=[
                        io.BytesIO(json.dumps(metadata).encode()),
                        io.BytesIO(body),
                    ],
                ):
                    with self.assertRaises(RemoteError):
                        install_codex_archive(target, "0.162.1", "arm64")
                self.assertFalse(target.exists())
                self.assertFalse((self.root / "escape").exists())


class SupervisorTests(JobFixture):
    def command(self, script, timeout=10):
        self.control = self.root / "control"
        self.control.mkdir()
        self.command_spec = {
            "args": [sys.executable, "-c", script],
            "cwd": str(self.repo),
            "env": {
                "PATH": os.environ.get("PATH", ""),
                "HOME": str(self.root / "fresh-home"),
            },
            "uid": None,
            "gid": None,
            "capture": {
                "repo": str(self.repo),
                "excluded_paths": ["AGENTS.md"],
                "session_dir": None,
                "files": {},
            },
        }
        self.execution = new_execution(
            prepare(self.write()),
            "trial",
            "fixture-command",
            sha256(canonical(self.command_spec)),
            sha256(
                Path(__file__).resolve().parents[1].joinpath("remote.py").read_bytes()
            ),
            now=time.time(),
            run_deadline=time.time() + 200,
        )
        self.execution["command_deadline"] = time.time() + timeout
        write_json(self.control / "command.json", self.command_spec)
        write_json(self.control / "receipt.json", self.execution)

    def test_original_exit_code_stdout_and_new_source_patch_are_preserved(self):
        self.command(
            "from pathlib import Path; Path('src/new.py').write_text('new source'); print('trace'); raise SystemExit(7)"
        )
        receipt = run_supervisor(self.control)
        self.assertEqual(receipt["phase"], "captured")
        self.assertEqual(receipt["exit_code"], 7)
        self.assertEqual(receipt["termination_reason"], "exited")
        self.assertIn("trace", (self.control / "artifacts/stdout.jsonl").read_text())
        self.assertIn(
            "src/new.py", (self.control / "artifacts/model.patch").read_text()
        )
        for row in receipt["artifact_manifest"]:
            raw = (self.control / "artifacts" / row["path"]).read_bytes()
            self.assertEqual(sha256(raw), row["sha256"])
            self.assertEqual(len(raw), row["size"])

    def test_repeated_supervision_does_not_execute_original_command_again(self):
        self.command(
            "from pathlib import Path; p=Path('counter'); p.write_text(p.read_text()+'x' if p.exists() else 'x')"
        )
        first = run_supervisor(self.control)
        self.assertEqual(run_supervisor(self.control), first)
        self.assertEqual((self.repo / "counter").read_text(), "x")

    def test_credentials_are_redacted_before_publishing_capture_hashes(self):
        secret = "fixture-secret-value-for-redaction"
        self.command("print('fixture-secret-value-for-redaction')")
        write_json(self.control / "redaction.json", [secret])
        receipt = run_supervisor(self.control)
        self.assertNotIn(secret, (self.control / "artifacts/stdout.jsonl").read_text())
        self.assertIn(
            "[REDACTED]", (self.control / "artifacts/stdout.jsonl").read_text()
        )
        self.assertFalse((self.control / "redaction.json").exists())
        self.assertEqual(receipt["phase"], "captured")

    def test_runtime_builtin_sources_are_captured_separately_from_user_inputs(self):
        self.command("print('completed')")
        home = (self.root / "fresh-home/.codex").resolve()
        (home / "sessions").mkdir(parents=True)
        skill = home / "skills/.system/helper"
        (skill / "references").mkdir(parents=True)
        (skill / "SKILL.md").write_text(
            "---\nname: helper\n---\nFixed runtime skill.\n"
        )
        (skill / "references/method.md").write_text("Fixed runtime reference.")
        self.command_spec["capture"].update(
            session_dir=str(home / "sessions"), runtime_skills=True
        )
        self.execution["command_hash"] = sha256(canonical(self.command_spec))
        write_json(self.control / "command.json", self.command_spec)
        write_json(self.control / "receipt.json", self.execution)
        run_supervisor(self.control)
        artifacts = self.control / "artifacts"
        captured = json.loads((artifacts / "runtime-skills.json").read_text())
        self.assertEqual(captured["skills"], ["helper"])
        self.assertEqual(len(captured["files"]), 2)
        for row in captured["files"]:
            self.assertEqual(
                sha256((artifacts / "runtime-skills" / row["path"]).read_bytes()),
                row["sha256"],
            )

    def test_credentials_in_code_patch_leave_a_capture_gap(self):
        self.command(
            "from pathlib import Path; Path('src/secret.py').write_text('fixture-secret-value-in-code')"
        )
        write_json(self.control / "redaction.json", ["fixture-secret-value-in-code"])
        run_supervisor(self.control)
        self.assertFalse((self.control / "artifacts/model.patch").exists())
        self.assertIn(
            "credential_in_patch",
            json.loads((self.control / "artifacts/capture.json").read_text())["gaps"],
        )

    def test_timeout_keeps_partial_new_source_and_real_signal_exit(self):
        self.command(
            "from pathlib import Path; import time; Path('src/partial.py').write_text('partial'); time.sleep(30)",
            timeout=0.5,
        )
        receipt = run_supervisor(self.control)
        self.assertEqual(receipt["termination_reason"], "timeout")
        self.assertNotEqual(receipt["exit_code"], 0)
        self.assertIn(
            "src/partial.py", (self.control / "artifacts/model.patch").read_text()
        )

    def test_cancellation_stops_original_process_before_capture(self):
        self.command("import time; time.sleep(30)")
        errors = []

        def worker():
            try:
                run_supervisor(self.control)
            except Exception as error:
                errors.append(error)

        thread = threading.Thread(target=worker)
        thread.start()
        deadline = time.monotonic() + 5
        while (
            time.monotonic() < deadline
            and inspect_command(self.control)["receipt"]["phase"] != "running"
        ):
            time.sleep(0.02)
        self.assertTrue(inspect_command(self.control)["command_alive"])
        write_json(self.control / "cancel.json", {"reason": "deadline"})
        thread.join(5)
        self.assertFalse(thread.is_alive())
        self.assertFalse(errors)
        receipt = json.loads((self.control / "receipt.json").read_text())
        self.assertEqual(receipt["termination_reason"], "cancelled")
        self.assertFalse(inspect_command(self.control)["command_alive"])

    def test_ambiguous_start_claim_does_not_restart_or_invent_exit(self):
        self.command("raise RuntimeError('must not run')")
        (self.control / "launch.claim").touch()
        self.execution["phase"] = "starting"
        write_json(self.control / "receipt.json", self.execution)
        self.assertEqual(run_supervisor(self.control)["phase"], "starting")
        receipt = capture_only(self.control)
        self.assertIsNone(receipt["exit_code"])
        self.assertEqual(receipt["termination_reason"], "original_exit_unknown")


class LifecycleTests(JobFixture):
    def test_transient_unreadable_receipt_is_requeried_without_another_launch(self):
        queries = 0

        def rpc(operation, execution, payload=None):
            nonlocal queries
            if operation == "query":
                queries += 1
                if queries == 1:
                    raise UnresolvedError("original command receipt is unavailable")
            return self.rpc(operation, execution, payload)

        self.client._rpc = rpc
        self.client.submit(self.execution, {}, self.save)
        self.assertEqual(self.execution["phase"], "running")
        self.assertEqual(queries, 2)
        self.assertEqual(self.calls.count("create"), 1)
        self.assertEqual(self.calls.count("launch"), 1)

    def test_persistently_unreadable_receipt_stays_unresolved_after_bounded_queries(
        self,
    ):
        queries = 0

        def rpc(operation, execution, payload=None):
            nonlocal queries
            if operation == "query":
                queries += 1
                raise UnresolvedError("original command receipt is unavailable")
            return self.rpc(operation, execution, payload)

        self.client._rpc = rpc
        with self.assertRaises(UnresolvedError):
            self.client.submit(self.execution, {}, self.save)
        self.assertEqual(queries, 3)
        self.assertEqual(self.execution["submission_status"], "unresolved")
        self.assertEqual(self.calls.count("launch"), 1)

    def setUp(self):
        super().setUp()
        self.job = prepare(self.write())
        self.execution = new_execution(
            self.job,
            "trial",
            "identity",
            "a" * 64,
            "b" * 64,
            now=time.time(),
            run_deadline=time.time() + 300,
        )
        self.client = RemoteClient(self.job.spec["runtime"])
        self.saved = []
        self.calls = []
        self.remote = deepcopy(self.execution)

    def save(self, execution):
        self.saved.append(deepcopy(execution))

    def rpc(self, operation, execution, payload=None):
        self.calls.append(operation)
        if operation == "create":
            self.remote.update(sandbox_id="container-id", engine_id="engine-id")
            return {"sandbox_id": "container-id", "engine_id": "engine-id"}
        if operation == "launch":
            self.assertTrue(
                any(row["sandbox_id"] == "container-id" for row in self.saved)
            )
            self.remote.update(
                phase="running",
                process={"pid": 200, "pgid": 200, "start_identity": "original"},
            )
            return {"launched": True}
        if operation == "query":
            return {
                "receipt": self.remote,
                "resource_state": "running",
                "command_alive": True,
                "supervisor_alive": True,
            }
        if operation == "read_artifact":
            return {"data": base64.b64encode(b"original artifact").decode()}
        if operation == "cleanup":
            return {"terminated": True}
        return {}

    def test_resource_identity_is_saved_before_launch_and_second_submit_only_queries(
        self,
    ):
        self.client._rpc = self.rpc
        self.client.submit(self.execution, {}, self.save)
        self.assertEqual(self.calls, ["create", "launch", "query"])
        self.assertEqual(self.saved[0]["submission_status"], "intent_saved")
        self.client.submit(self.execution, {}, self.save)
        self.assertEqual(self.calls, ["create", "launch", "query", "query"])

    def test_lost_submit_response_and_identity_drift_do_not_repeat_paid_work(self):
        def lost(operation, execution, payload=None):
            self.calls.append(operation)
            raise UnresolvedError("connection lost")

        self.client._rpc = lost
        with self.assertRaises(UnresolvedError):
            self.client.submit(self.execution, {}, self.save)
        self.assertEqual(self.execution["submission_status"], "unresolved")
        self.client._rpc = self.rpc
        self.remote["command_hash"] = "c" * 64
        with self.assertRaises(UnresolvedError):
            self.client.submit(self.execution, {}, self.save)
        self.assertEqual(self.calls, ["create", "query"])

    def test_export_is_repeatable_and_does_not_rerun_command(self):
        content = b"original artifact"
        self.remote.update(
            phase="captured",
            exit_code=0,
            artifact_manifest=[
                {"path": "model.patch", "size": len(content), "sha256": sha256(content)}
            ],
        )
        self.execution.update(self.remote)
        self.client._rpc = self.rpc
        destination = self.root / "artifacts"
        self.client.export(self.execution, destination, self.save)
        self.client.export(self.execution, destination, self.save)
        self.assertEqual(self.calls, ["read_artifact"])
        self.assertEqual((destination / "model.patch").read_bytes(), content)
        self.assertEqual(self.execution["export_status"], "exported")
        self.client.cleanup(self.execution, self.save)
        self.assertEqual(self.execution["cleanup_status"], "terminated")

    def test_cleanup_requires_export_or_recorded_force_reason(self):
        self.client._rpc = self.rpc
        with self.assertRaises(UnresolvedError):
            self.client.cleanup(self.execution, self.save)
        self.assertFalse(self.calls)

    def test_process_state_cannot_be_inferred_from_resource_alive(self):
        self.remote["phase"] = "starting"
        self.client._rpc = lambda *args: {
            "receipt": self.remote,
            "resource_state": "running",
            "command_alive": False,
            "supervisor_alive": False,
        }
        with self.assertRaises(UnresolvedError):
            self.client.query(self.execution, self.save)

    def test_live_supervisor_can_finish_capture_after_child_exits(self):
        self.remote.update(
            phase="running",
            process={"pid": 200, "pgid": 200, "start_identity": "original"},
        )
        self.client._rpc = lambda *args: {
            "receipt": self.remote,
            "resource_state": "running",
            "command_alive": False,
            "supervisor_alive": True,
        }
        result = self.client.query(self.execution, self.save)
        self.assertFalse(result["command_alive"])
        self.assertEqual(self.execution["submission_status"], "runner_submitted")

    def test_starting_with_live_supervisor_waits_without_resubmitting(self):
        self.remote["phase"] = "starting"
        self.client._rpc = lambda *args: {
            "receipt": self.remote,
            "resource_state": "running",
            "command_alive": False,
            "supervisor_alive": True,
        }
        result = self.client.query(self.execution, self.save)
        self.assertEqual(result["receipt"]["phase"], "starting")

    def test_lost_supervisor_captures_stopped_command_without_inventing_an_exit(self):
        self.remote.update(
            phase="running",
            process={"pid": 200, "pgid": 200, "start_identity": "original"},
        )
        calls = []

        def rpc(operation, execution, payload=None):
            calls.append(operation)
            if operation == "capture_only":
                self.remote.update(
                    phase="captured",
                    exit_code=None,
                    termination_reason="original_exit_unknown",
                )
            return {
                "receipt": self.remote,
                "resource_state": "running",
                "command_alive": False,
                "supervisor_alive": False,
            }

        self.client._rpc = rpc
        with self.assertRaises(UnresolvedError):
            self.client.query(self.execution, self.save)
        self.assertEqual(calls, ["query", "capture_only", "query"])
        self.assertEqual(self.execution["phase"], "captured")
        self.assertIsNone(self.execution["exit_code"])


if __name__ == "__main__":
    import unittest

    unittest.main()
