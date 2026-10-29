"""SSH/Docker execution and its standalone bounded command supervisor."""

import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import re
import signal
import shlex
import stat
import subprocess
import sys
import tempfile
import tarfile
import time
import uuid
import urllib.request


TASK_PATH = "/go/bin:/usr/local/go/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"


class RemoteError(RuntimeError):
    pass


class UnresolvedError(RemoteError):
    pass


def encoded(value):
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def digest(content):
    return hashlib.sha256(content).hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def save_bytes(path, content):
    path = Path(path)
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=".receipt-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def save_json(path, value):
    save_bytes(path, encoded(value) + b"\n")


def ssh_request(runtime, source, operation, execution, payload):
    runner_hash = digest(source)
    if runner_hash != execution["runner_hash"]:
        raise UnresolvedError("runner source differs from the frozen execution")
    script = runtime["remote_root"] + "/tools/" + runner_hash + "/remote.py"
    bootstrap = (
        "import hashlib,pathlib,sys; p=pathlib.Path(sys.argv[1]); b=sys.stdin.buffer.read(); "
        "assert hashlib.sha256(b).hexdigest()==sys.argv[2]; "
        "p.parent.mkdir(parents=True,exist_ok=True,mode=0o700); "
        "assert not p.is_symlink(); "
        "assert not p.exists() or p.read_bytes()==b; "
        "p.write_bytes(b) if not p.exists() else None"
    )
    ssh = [
        "ssh",
        "-o",
        "BatchMode=yes",
        "-o",
        "ForwardAgent=no",
        "-o",
        "ClearAllForwardings=yes",
        "-o",
        "ConnectTimeout=10",
        "-o",
        "ConnectionAttempts=1",
        "--",
        runtime["ssh_target"],
    ]
    try:
        deployed = subprocess.run(
            [*ssh, shlex.join(["python3", "-c", bootstrap, script, runner_hash])],
            input=source,
            capture_output=True,
            timeout=30,
        )
        if deployed.returncode:
            raise RemoteError("remote runner could not be staged")
        request = {
            "runtime": runtime,
            "operation": operation,
            "execution": execution,
            "payload": payload,
        }
        timeout = 300
        if operation in {"create", "setup_exec"}:
            timeout = max(1, min(1000, execution["command_deadline"] - time.time()))
        response = subprocess.run(
            [*ssh, shlex.join(["python3", script, "--dispatch"])],
            input=encoded(request),
            capture_output=True,
            timeout=timeout,
        )
        result = json.loads(response.stdout)
        if not isinstance(result, dict):
            raise UnresolvedError("remote response has no execution object")
        if "error" in result:
            error = UnresolvedError if result.get("unresolved") else RemoteError
            raise error(result["error"])
        if response.returncode:
            raise UnresolvedError("remote request did not return a confirmed result")
        return result
    except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError) as error:
        raise UnresolvedError("SSH response could not be confirmed") from error


class RemoteClient:
    def __init__(self, runtime, runner_source=None):
        self.runtime = runtime
        self.runner_source = (
            Path(__file__).read_bytes() if runner_source is None else runner_source
        )
        self.runner_hash = digest(self.runner_source)

    def _rpc(self, operation, execution, payload=None):
        return ssh_request(
            self.runtime, self.runner_source, operation, execution, payload
        )

    def submit(self, execution, payload, save):
        if execution["submission_status"] != "not_submitted":
            return self.query(execution, save)
        execution["submission_status"] = "intent_saved"
        save(execution)
        try:
            resource = self._rpc("create", execution, payload)
            execution.update(
                sandbox_id=resource["sandbox_id"],
                engine_id=resource["engine_id"],
                submission_status="resource_created",
            )
            save(execution)
            if payload.get("needs_codex"):
                self.prepare_codex(execution, payload)
            if payload.get("needs_typesafe"):
                self.prepare_typesafe(execution, payload)
            if time.time() >= execution["command_deadline"]:
                raise UnresolvedError(
                    "command deadline reached during setup; original command was not launched"
                )
            execution["submission_status"] = "launch_intent_saved"
            save(execution)
            self._rpc("launch", execution)
            execution["submission_status"] = "runner_submitted"
            save(execution)
            return self.query(execution, save)
        except (RemoteError, KeyError) as error:
            execution["submission_status"] = "unresolved"
            save(execution)
            raise UnresolvedError(
                "submission result is unresolved; query the registered execution"
            ) from error

    def query(self, execution, save):
        from protocol import ProtocolError, validate_execution

        try:
            for attempt in range(3):
                try:
                    result = self._rpc("query", execution)
                    break
                except UnresolvedError as error:
                    if (
                        str(error) != "original command receipt is unavailable"
                        or attempt == 2
                        or time.time() >= execution["resource_deadline"]
                    ):
                        raise
                    time.sleep(0.25)
            receipt = result["receipt"]
            validate_execution(receipt)
            immutable = [
                "schema_version",
                "remote_task_id",
                "role",
                "idempotency_key",
                "input_hash",
                "command_hash",
                "runner_hash",
                "runner_version",
                "ssh_target",
                "remote_root",
                "sandbox_name",
                "receipt_path",
                "command_deadline",
                "resource_deadline",
            ]
            if any(receipt[key] != execution[key] for key in immutable):
                raise UnresolvedError("remote execution identity has drifted")
            for key in ["sandbox_id", "engine_id"]:
                if execution[key] is not None and receipt[key] != execution[key]:
                    raise UnresolvedError("remote resource identity has drifted")
            for key in [
                "sandbox_id",
                "engine_id",
                "phase",
                "process",
                "exit_code",
                "termination_reason",
                "artifact_manifest",
            ]:
                execution[key] = receipt[key]
            save(execution)
            phase = receipt["phase"]
            if (
                phase in {"starting", "running", "capturing"}
                and not result["supervisor_alive"]
            ):
                if result["command_alive"]:
                    raise UnresolvedError(
                        "original supervisor is unavailable while the command is still running"
                    )
                recovered = self._rpc("capture_only", execution)
                if recovered.get("receipt", {}).get("phase") != "captured":
                    raise UnresolvedError(
                        "capture-only could not preserve the stopped execution"
                    )
                return self.query(execution, save)
            if phase == "prepared" and execution["submission_status"] in {
                "launch_intent_saved",
                "runner_submitted",
                "unresolved",
            }:
                raise UnresolvedError("launch window is unresolved")
            if phase == "captured" and receipt["exit_code"] is None:
                raise UnresolvedError("original exit receipt is missing")
            if phase in {"running", "capturing", "captured"}:
                execution["submission_status"] = "runner_submitted"
                save(execution)
            return result
        except (RemoteError, ProtocolError, KeyError) as error:
            execution["submission_status"] = "unresolved"
            save(execution)
            if isinstance(error, UnresolvedError):
                raise
            raise UnresolvedError(
                "registered remote execution could not be queried"
            ) from error

    def cancel(self, execution, reason, save):
        execution["cancel_reason"] = reason
        save(execution)
        return self._rpc("cancel", execution, {"reason": reason})

    def export(self, execution, destination, save):
        from protocol import validate_execution

        validate_execution(execution)
        if execution["phase"] != "captured":
            raise UnresolvedError("execution has no complete capture receipt")
        destination = Path(destination)
        if destination.is_symlink():
            raise UnresolvedError("artifact destination cannot be a symlink")
        destination = destination.resolve()
        destination.mkdir(mode=0o700, parents=True, exist_ok=True)
        execution["export_status"] = "partial"
        save(execution)
        for row in execution["artifact_manifest"]:
            path = destination / row["path"]
            if any(parent.is_symlink() for parent in [path, *path.parents]):
                raise UnresolvedError("artifact path contains a symlink")
            if path.exists():
                content = path.read_bytes()
            else:
                result = self._rpc("read_artifact", execution, {"path": row["path"]})
                try:
                    content = base64.b64decode(result["data"], validate=True)
                except (ValueError, KeyError) as error:
                    raise UnresolvedError("artifact download is incomplete") from error
            if len(content) != row["size"] or digest(content) != row["sha256"]:
                raise UnresolvedError(
                    "artifact bytes do not match the captured manifest"
                )
            if not path.exists():
                save_bytes(path, content)
        execution["export_status"] = "exported"
        save(execution)
        return destination

    def cleanup(self, execution, save, force_reason=None):
        if execution["export_status"] != "exported" and not force_reason:
            raise UnresolvedError(
                "cleanup requires exported evidence or a recorded force reason"
            )
        execution["cleanup_status"] = "pending"
        save(execution)
        result = self._rpc("cleanup", execution, {"force_reason": force_reason})
        if not result.get("terminated"):
            raise UnresolvedError("registered resource termination is not confirmed")
        execution["cleanup_status"] = "terminated"
        save(execution)
        return result

    def prepare_codex(self, execution, payload):
        import asyncio
        from pathlib import PurePosixPath
        from types import SimpleNamespace
        from harbor.agents.installed.codex import Codex

        client = self

        class DockerSetup:
            default_user = None

            async def exec(
                self, command, user=None, env=None, cwd=None, timeout_sec=None
            ):
                result = client._rpc(
                    "setup_exec",
                    execution,
                    {
                        "command": command,
                        "user": user,
                        "cwd": cwd,
                        "env": {
                            "HOME": "/home/evolution",
                            "PATH": "/usr/local/bin:/usr/bin:/bin",
                            **(env or {}),
                        },
                        "timeout_sec": timeout_sec,
                    },
                )
                return SimpleNamespace(**result)

            async def upload_file(self, source_path, target_path):
                client._rpc(
                    "setup_files",
                    execution,
                    {target_path: encoded_file(Path(source_path).read_bytes(), 0o600)},
                )

        class IsolatedCodex(Codex):
            _REMOTE_CODEX_HOME = PurePosixPath("/home/evolution/.codex")

            async def install(self, environment):
                cached = client._rpc("restore_codex", execution)
                if not cached["restored"]:
                    available = await environment.exec(
                        command="command -v node && command -v npm", user="root"
                    )
                    architecture = (
                        "arm64"
                        if client.runtime["codex_architecture"] == "aarch64"
                        else "x64"
                    )
                    command = (
                        "npm install --global --force --prefix /opt/evolution-codex --registry https://registry.npmjs.org @openai/codex@"
                        + self._version
                        + "-linux-"
                        + architecture
                    )
                    if available.return_code != 0:
                        command = shlex.join(
                            [
                                "python3",
                                "-c",
                                "import sys; sys.path.insert(0,'/evolution-control'); "
                                "from remote import install_codex_archive; "
                                "install_codex_archive(sys.argv[1],sys.argv[2],sys.argv[3])",
                                "/opt/evolution-codex/lib/node_modules/@openai/codex",
                                self._version,
                                architecture,
                            ]
                        )
                    installed = await environment.exec(
                        command=command,
                        user="root",
                        timeout_sec=min(
                            900,
                            max(1, int(execution["command_deadline"] - time.time())),
                        ),
                    )
                    if installed.return_code != 0:
                        raise RemoteError("fixed Codex package installation failed")
                link = (
                    "import os,pathlib; root=pathlib.Path('/opt/evolution-codex/lib/node_modules/@openai/codex'); "
                    "names={'codex','codex-code-mode-host','codex-exec-server','rg','bwrap'}; "
                    "bins=[p for p in root.rglob('*') if p.is_file() and p.name in names and os.access(p,os.X_OK)]; "
                    "[(pathlib.Path('/usr/local/bin',p.name).unlink(missing_ok=True),pathlib.Path('/usr/local/bin',p.name).symlink_to(p)) for p in bins]"
                )
                await self.exec_as_root(
                    environment, command=shlex.join(["python3", "-c", link])
                )
                if not cached["restored"]:
                    client._rpc("cache_codex", execution)

        async def install():
            with tempfile.TemporaryDirectory(prefix="evolution-setup-") as logs:
                agent = IsolatedCodex(
                    logs_dir=Path(logs),
                    model_name=self.runtime.get("model"),
                    version=self.runtime["codex_version"],
                    config=payload["codex_config"],
                )
                environment = DockerSetup()
                await agent.install(environment)
                await agent._upload_effective_config(
                    environment,
                    agent._build_effective_config(),
                    "/home/evolution/.codex/config.toml",
                )

        asyncio.run(install())
        import toml

        receipt = self._rpc("finalize_setup", execution)
        if receipt["config_sha256"] != digest(
            toml.dumps(payload["codex_config"]).encode()
        ):
            raise RemoteError(
                "native Codex configuration bytes differ from the declared configuration"
            )

    def prepare_typesafe(self, execution, payload):
        key = os.environ.get("TYPESAFE_API_KEY")
        if not key:
            raise RemoteError("TypeSafe controller credential is unavailable")
        result = self._rpc("prepare_typesafe", execution, {"key": key})
        if result != {"prepared": True}:
            raise UnresolvedError("TypeSafe credential preparation is unresolved")

    def destroy_typesafe(self, execution):
        result = self._rpc("destroy_typesafe", execution)
        if result != {"destroyed": True}:
            raise UnresolvedError("TypeSafe credential destruction is unresolved")


def isolation_script(case, *, schema_version=1):
    settings = {"repo": case["repo_path"], "base_commit": case["base_commit"]}
    return (
        "settings="
        + repr(settings)
        + "\n"
        + """
import json, os, pathlib, socket, subprocess
checks = {'unprivileged_user': os.getuid() == 10000}
"""
        + ("""import shutil
npm = shutil.which('npm')
checks['installed_npm_ready'] = True
if npm:
    try:
        started = subprocess.run([npm, '--version'], text=True, capture_output=True, timeout=30)
        checks['installed_npm_ready'] = started.returncode == 0 and bool(started.stdout.strip())
    except (OSError, subprocess.TimeoutExpired):
        checks['installed_npm_ready'] = False
""" if schema_version == 2 else "")
        + """for name, path in [('auth_hidden', '/home/evolution/.codex/auth.json'), ('control_hidden', '/evolution-control/receipt.json')]:
    try:
        pathlib.Path(path).read_bytes()
        checks[name] = False
    except (PermissionError, FileNotFoundError):
        checks[name] = True
try:
    pathlib.Path('/evolution-control/receipt.json').write_text('forbidden')
    checks['control_write_denied'] = False
except (PermissionError, FileNotFoundError):
    checks['control_write_denied'] = True
for name, path in [('no_grader', '/tests/test.sh'), ('no_oracle', '/solution/gold_patch.diff')]:
    try:
        checks[name] = not pathlib.Path(path).exists()
    except PermissionError:
        checks[name] = True
checks['clean_tool_environment'] = not any(any(word in key.upper() for word in ['TOKEN','SECRET','API_KEY','SSH_AUTH']) for key in os.environ)
checks['base_matches'] = subprocess.check_output(['git','-C',settings['repo'],'rev-parse','HEAD'],text=True).strip() == settings['base_commit']
ancestors = set(subprocess.check_output(['git','-C',settings['repo'],'rev-list',settings['base_commit']],text=True).splitlines())
objects = subprocess.check_output(['git','-C',settings['repo'],'cat-file','--batch-all-objects','--batch-check=%(objectname) %(objecttype)'],text=True).splitlines()
commits = {line.split()[0] for line in objects if line.endswith(' commit')}
checks['no_later_git_objects'] = commits <= ancestors
try:
    connection = socket.create_connection(('github.com', 443), timeout=2)
    connection.close()
    checks['external_network_blocked'] = False
except OSError:
    checks['external_network_blocked'] = True
try:
    with socket.socket() as server:
        server.bind(('127.0.0.1', 0))
        server.listen(1)
        connection = socket.create_connection(server.getsockname(), timeout=2)
        connection.close()
    checks['loopback_available'] = True
except OSError:
    checks['loopback_available'] = False
marker = pathlib.Path(settings['repo']) / 'src/evolution_capture_probe.js'
marker.parent.mkdir(parents=True, exist_ok=True)
marker.write_text('// deterministic capture probe\\n')
print(json.dumps({'isolation_passed': all(checks.values()), 'checks': checks, 'environment_names': sorted(os.environ)}))
raise SystemExit(0 if all(checks.values()) else 1)
"""
    )


def native_skill_files(job, bundle):
    from protocol import skill_name

    baseline = job.bundles[job.spec["baseline_condition"]]
    descriptors = [path for path in job.spec["target"]["surfaces"]["skill"]
                   if Path(path).name == "SKILL.md"]
    descriptors.extend(root + "/SKILL.md" for root in job.spec["source"].get("fixed_skill_roots", []))
    files = {}
    for path in descriptors:
        if path not in bundle:
            continue
        root = Path(path).parent.as_posix()
        name = skill_name(baseline[path]["content"])
        for item, row in bundle.items():
            if item.startswith(root + "/"):
                destination = "/home/evolution/.agents/skills/" + name + "/" + item[len(root) + 1:]
                files[destination] = encoded_file(row["content"], row["mode"])
    return files


def smoke_payload(job, case, purpose):
    runtime = job.spec["runtime"]
    repo = case["repo_path"]
    views = job.case_files[case["case_id"]]
    files = {}
    for name, row in views["public_paths"].items():
        files["/evolution-input/task/" + name] = encoded_file(
            row["content"], row["mode"]
        )
    native = purpose == "probe"
    baseline = job.bundles[job.spec["baseline_condition"]]
    same_repository = (case["repo_id"], case["base_commit"]) == (
        job.spec["target"]["repo_id"],
        job.spec["target"]["base_commit"],
    )
    removed = []
    if same_repository:
        for name, row in baseline.items():
            if native and name in job.spec["target"]["surfaces"]["skill"]:
                removed.append(repo + "/" + name)
            else:
                files[repo + "/" + name] = encoded_file(row["content"], row["mode"])
    configuration = codex_configuration(
        runtime.get("model", "gpt-6.1-sol"), runtime.get("effort", "high"),
        schema_version=job.spec["schema_version"],
    )
    if native:
        files.update(native_skill_files(job, baseline))
        if "model" not in runtime:
            files["/home/evolution/.codex/auth.json"] = encoded_file(
                encoded({"OPENAI_API_KEY": "isolation-sentinel-not-a-credential"}),
                0o600,
            )
        args = codex_arguments(configuration, repo,
            probe=isolation_script(case, schema_version=job.spec["schema_version"]))
        if "model" in runtime:
            startup = codex_arguments(
                configuration,
                repo,
                prompt="Reply exactly RUNTIME_READY. Do not inspect or change files or use tools.",
            )
            args = ["bash", "-c", shlex.join(args) + " && " + shlex.join(startup)]
        outputs = {
            "instruction-receipt.json": "/evolution-control/instruction-receipt.json"
        }
    else:
        for name, row in views["grader_paths"].items():
            files["/" + name] = encoded_file(row["content"], row["mode"])
        if purpose == "oracle":
            for name, row in views["oracle_paths"].items():
                files["/" + name] = encoded_file(row["content"], row["mode"])
        script = (
            "bash /solution/solve.sh && bash /tests/test.sh"
            if purpose == "oracle"
            else "bash /tests/test.sh"
        )
        args = ["bash", "-c", script]
        outputs = {
            "reward.txt": "/logs/verifier/reward.txt",
            "instruction-receipt.json": "/evolution-control/instruction-receipt.json",
        }
    return {
        "image": case["image"],
        "platform": case["platform"],
        "repo_path": repo,
        "base_commit": case["base_commit"],
        "files": files,
        "remove_paths": removed,
        "needs_codex": native,
        "codex_config": configuration if native else None,
        "grading": not native,
        "command": {
            "args": args,
            "cwd": repo,
            "uid": 10000,
            "gid": 10000,
            "env": {
                "HOME": "/home/evolution",
                "CODEX_HOME": "/home/evolution/.codex",
                "PATH": TASK_PATH,
                "GOPATH": "/go",
                "GOCACHE": "/tmp/evolution-go-cache",
                "LANG": "C.UTF-8",
                "USER": "evolution",
                **(
                    {
                        "HTTP_PROXY": runtime["model_proxy"],
                        "HTTPS_PROXY": runtime["model_proxy"],
                    }
                    if native and "model_proxy" in runtime
                    else {}
                ),
            },
            "capture": {
                "repo": repo,
                "baseline_ref_file": "/evolution-control/base-tree.txt",
                "excluded_paths": job.spec["target"]["surfaces"]["agents"],
                "session_dir": "/home/evolution/.codex/sessions" if native else None,
                "runtime_skills": native and "model" in runtime,
                "files": outputs,
                "optional_files": {}
                if native
                else {
                    "grade-output.json": "/logs/verifier/output.json",
                    "run-script-stdout.txt": "/logs/verifier/run-script-stdout.txt",
                    "run-script-stderr.txt": "/logs/verifier/run-script-stderr.txt",
                },
            },
        },
    }


def trial_payload(job, case, bundle):
    payload = smoke_payload(job, case, "probe")
    files = payload["files"]
    for name in list(files):
        if name.startswith("/home/evolution/.agents/skills/"):
            del files[name]
    from protocol import task_contract

    files.update(native_skill_files(job, bundle))
    for path in job.spec["target"]["surfaces"]["agents"]:
        destination = case["repo_path"] + "/" + path
        files.pop(destination, None)
        if path in bundle:
            files[destination] = encoded_file(
                bundle[path]["content"], bundle[path]["mode"]
            )
        else:
            payload["remove_paths"].append(destination)
    instruction = job.case_files[case["case_id"]]["public_paths"]["instruction.md"][
        "content"
    ].decode("utf-8")
    if job.spec.get("evaluation", {}).get("scoring") == "continuous_100_v1":
        contract = task_contract(case)
        files["/evolution-input/task-contract.md"] = encoded_file(contract, 0o444)
        instruction += "\n\nAdditional frozen observable task requirements:\n" + contract.decode("utf-8")
    payload["command"]["args"] = codex_arguments(
        payload["codex_config"], case["repo_path"], prompt=instruction
    )
    return payload


def grading_payload(job, case, patch):
    payload = smoke_payload(job, case, "empty")
    payload["files"]["/evolution-input/model.patch"] = encoded_file(patch)
    payload["command"]["args"] = [
        "bash",
        "-c",
        "git apply --binary /evolution-input/model.patch && bash /tests/test.sh"
        if patch
        else "bash /tests/test.sh",
    ]
    payload["command"]["capture"]["optional_files"]["checks.json"] = (
        "/logs/verifier/checks.json"
    )
    if job.spec["schema_version"] == 2:
        rubric = json.loads(job.evaluation_files[job.spec["evaluation"]["rubrics"][case["case_id"]]]["content"])
        names = sorted({name for c in rubric["criteria"] for name in c["context_paths"] if name.startswith("repo/")})
        source = """
import json, pathlib, sys
root = pathlib.Path.cwd()
result = {'files': {}, 'omissions': []}
for name in json.loads(sys.argv[1]):
    path = root / name.removeprefix('repo/')
    try:
        if '..' in path.relative_to(root).parts or any(p.is_symlink() for p in [path, *path.parents] if p != root):
            raise ValueError('unsafe path')
        result['files'][name] = path.read_text(encoding='utf-8')
    except (OSError, ValueError, UnicodeError):
        result['omissions'].append(name)
pathlib.Path('/logs/verifier/quality-context.json').write_text(json.dumps(result))
"""
        collect = shlex.join(["python3", "-c", source, encoded(names).decode()])
        verify = "bash /tests/test.sh"
        observations = sorted({name for c in rubric["criteria"] for name in c["context_paths"]
                               if name.startswith("verifier/")})
        if observations:
            observe_source = """
import json, pathlib, sys
root = pathlib.Path('/logs/verifier/public')
result = {'files': {}, 'omissions': []}
for name in json.loads(sys.argv[1]):
    path = root / name.removeprefix('verifier/')
    try:
        if '..' in path.relative_to(root).parts or any(p.is_symlink() for p in [path, *path.parents]):
            raise ValueError('unsafe observation path')
        result['files'][name] = path.read_bytes().decode('utf-8')
    except (OSError, ValueError, UnicodeError):
        result['omissions'].append(name)
pathlib.Path('/logs/verifier/public-context.json').write_text(json.dumps(result))
"""
            observe = shlex.join(["python3", "-c", observe_source, encoded(observations).decode()])
            verify = '{ bash /tests/test.sh; xgent_verifier_status=$?; ' + observe + '; exit "$xgent_verifier_status"; }'
            payload["command"]["capture"]["files"]["public-context.json"] = "/logs/verifier/public-context.json"
        payload["command"]["args"] = ["bash", "-c",
            ("git apply --binary /evolution-input/model.patch && " if patch else "") + collect + " && " + verify]
        payload["command"]["capture"]["files"]["quality-context.json"] = "/logs/verifier/quality-context.json"
    return payload


def regression_payload(job, case, patch, checker):
    payload = grading_payload(job, case, b"")
    payload["files"]["/evolution-input/model.patch"] = encoded_file(patch)
    payload["files"]["/evolution-input/checker.json"] = encoded_file(encoded(checker))
    script = """
import json, pathlib, subprocess
checker = json.loads(pathlib.Path('/evolution-input/checker.json').read_text())
patch = '/evolution-input/model.patch'
base_tree = subprocess.check_output(['git','write-tree'],text=True).strip()
def run(argv):
    p = subprocess.run(argv, capture_output=True, text=True, errors='replace')
    return {'exit_code': p.returncode, 'stdout': p.stdout, 'stderr': p.stderr}
def reset():
    subprocess.run(['git','read-tree','--reset','-u',base_tree],check=True)
    subprocess.run(['git','clean','-fd'],check=True,capture_output=True)
result = {'status': 'unknown', 'level': None, 'reason': 'replay_setup_failed', 'base_tree': base_tree}
selection = [arg for path in checker['test_paths'] for arg in ['--include',path]]
selected = run(['git','apply','--numstat','--allow-empty',*selection,patch])
test_patch = run(['git','apply','--binary','--allow-empty',*selection,patch])
if test_patch['exit_code'] == 0 and selected['exit_code'] == 0:
    if not selected['stdout'].strip():
        result.update(status='valid',level=0,reason='no_changed_regression_test')
    else:
        old = run(checker['argv'])
        reset()
        applied = run(['git','apply','--binary',patch])
        if applied['exit_code'] == 0:
            fixed = run(checker['argv'])
            old_text = old['stdout'] + old['stderr']
            environment = ['ModuleNotFoundError:', 'ImportError while importing test module', 'command not found', 'collected 0 items']
            attributed = old['exit_code'] == checker['expected_failure']['exit_code'] and checker['expected_failure']['contains'] in old_text
            if not any(marker in old_text + fixed['stdout'] + fixed['stderr'] for marker in environment):
                result.update(status='valid',level=int(attributed and fixed['exit_code']==0),reason='counterfactual_replay')
            else:
                result['reason'] = 'replay_environment_failure'
            result.update(original=old,fixed=fixed,defect_attributed=attributed)
pathlib.Path('/logs/verifier/regression-replay.json').write_text(json.dumps(result))
"""
    payload["command"]["args"] = ["python3", "-c", script]
    payload["command"]["capture"]["files"] = {
        "regression-replay.json": "/logs/verifier/regression-replay.json",
        "instruction-receipt.json": "/evolution-control/instruction-receipt.json",
    }
    payload["command"]["capture"]["optional_files"] = {}
    return payload


def generation_payload(job, source, instruction, outputs, *, optimizer=False):
    case = job.spec["cases"][0]
    payload = smoke_payload(job, case, "probe")
    runtime = job.spec["optimizer"] if optimizer else job.spec["runtime"]
    configuration = codex_configuration(runtime["model"], runtime["effort"],
        schema_version=job.spec["schema_version"])
    payload.update(
        repo_path="/work",
        base_commit=None,
        generated_workspace=True,
        codex_config=configuration,
    )
    payload["files"] = {"/work/source.json": encoded_file(encoded(source))}
    payload["remove_paths"] = []
    if optimizer:
        payload["files"]["/evolution-input/method.md"] = encoded_file(
            job.method_files["method.md"]["content"]
        )
    payload["command"].update(
        args=codex_arguments(configuration, "/work", prompt=instruction), cwd="/work"
    )
    payload["command"]["capture"] = {
        "repo": None,
        "excluded_paths": [],
        "session_dir": "/home/evolution/.codex/sessions",
        "runtime_skills": True,
        "files": {
            **{name: "/work/" + name for name in outputs},
            "instruction-receipt.json": "/evolution-control/instruction-receipt.json",
        },
    }
    return payload


def typesafe_payload(job, request, *, long_context=False):
    from typesafe_worker import validate_request

    validate_request(request)
    if type(long_context) is not bool:
        raise RemoteError("invalid TypeSafe long-context setting")
    payload = generation_payload(job, {}, "", [])
    credential = "/home/evolution/.typesafe/api-key"
    payload.update(needs_codex=False, needs_typesafe=True, codex_config=None)
    payload["files"] = {
        "/work/request.json": encoded_file(encoded(request)),
        "/work/typesafe_worker.py": encoded_file(job.worker_files["typesafe_worker.py"]["content"]),
    }
    payload["command"].update(
        args=["python3", "/work/typesafe_worker.py", "--request", "/work/request.json",
              "--credential", credential, "--output", "/work/auxiliary-response.json",
              "--timeout", str(max(1, job.spec["budget"]["per_trial_timeout_sec"] - 10))],
        typesafe_credential=credential,
    )
    if long_context:
        payload["command"]["args"].append("--long-context")
    payload["command"]["capture"] = {
        "repo": None, "excluded_paths": [], "session_dir": None,
        "files": {"auxiliary-response.json": "/work/auxiliary-response.json",
                  "instruction-receipt.json": "/evolution-control/instruction-receipt.json"},
    }
    return payload


def prepare_typesafe_credential(runtime, execution, payload):
    control = host_control(execution)
    packet = read_json(control / "payload.json")
    if execution["role"] != "grader" or not packet.get("needs_typesafe") or packet.get("needs_codex"):
        raise RemoteError("credential preparation requires the registered API worker")
    if packet["command"].get("typesafe_credential") != "/home/evolution/.typesafe/api-key":
        raise RemoteError("API credential path differs")
    if set(payload) != {"key"} or not isinstance(payload["key"], str) or not payload["key"] or any(
        character.isspace() for character in payload["key"]
    ):
        raise RemoteError("API credential is unavailable")
    if (control / "typesafe-credential.json").exists():
        raise UnresolvedError("API credential preparation was already attempted; query the original execution")
    save_json(control / "typesafe-credential.json", {"prepared": False, "destroyed": False})
    # The secret uses its own stdin operation, never create_resource's persisted payload.
    key_path = control / "typesafe.key"
    save_bytes(key_path, payload["key"].encode())
    key_path.chmod(0o600)
    container_exec(runtime, execution, ["python3", "-c",
        "import os,pathlib; p=pathlib.Path('/home/evolution/.typesafe'); "
        "p.mkdir(mode=0o700,exist_ok=True); os.chown(p,10000,10000); os.chmod(p,0o700)"])
    container_files(runtime, execution, {
        "/home/evolution/.typesafe/api-key": encoded_file(key_path.read_bytes(), 0o600),
        "/evolution-control/redaction.json": encoded_file(encoded([payload["key"]]), 0o600),
    })
    container_exec(runtime, execution, ["chown", "10000:10000", "/home/evolution/.typesafe/api-key"])
    key_path.unlink()
    save_json(control / "typesafe-credential.json", {"prepared": True, "destroyed": False})
    return {"prepared": True}


def destroy_typesafe_credential(runtime, execution, resource):
    control = host_control(execution)
    receipt = control / "typesafe-credential.json"
    if not receipt.exists():
        return True
    (control / "typesafe.key").unlink(missing_ok=True)
    if resource is not None:
        container_exec(runtime, execution, ["python3", "-c",
            "import pathlib; pathlib.Path('/home/evolution/.typesafe/api-key').unlink(missing_ok=True); "
            "pathlib.Path('/evolution-control/redaction.json').unlink(missing_ok=True)"])
    save_json(receipt, {"prepared": read_json(receipt)["prepared"], "destroyed": True})
    return True


def host_environment(runtime):
    return {
        "HOME": str(Path.home()),
        "PATH": str(Path(runtime["docker_binary"]).parent)
        + ":/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
        "LANG": "C.UTF-8",
    }


def docker(runtime, arguments, *, content=None, timeout=60, check=True):
    result = subprocess.run(
        [runtime["docker_binary"], *arguments],
        input=content,
        capture_output=True,
        env=host_environment(runtime),
        timeout=timeout,
    )
    if check and result.returncode:
        raise RemoteError("remote Docker command failed")
    return result


def engine_identity(runtime):
    result = docker(
        runtime,
        [
            "info",
            "--format",
            '{"id":{{json .ID}},"version":{{json .ServerVersion}},"architecture":{{json .Architecture}}}',
        ],
    )
    value = json.loads(result.stdout)
    if value["version"] != runtime["docker_version"]:
        raise UnresolvedError("Docker version differs from the frozen runtime")
    architecture = {
        "arm64": "aarch64",
        "aarch64": "aarch64",
        "amd64": "x86_64",
        "x86_64": "x86_64",
    }.get(value["architecture"])
    if architecture != runtime["codex_architecture"]:
        raise UnresolvedError("Codex architecture differs from the Docker kernel")
    return value["id"]


def host_control(execution):
    task_id = str(uuid.UUID(execution["remote_task_id"]))
    if (
        task_id != execution["remote_task_id"]
        or execution["sandbox_name"] != "evolve-" + task_id
    ):
        raise UnresolvedError("invalid registered execution identity")
    root = Path(execution["remote_root"]) / "executions" / task_id / "control"
    if str(root / "receipt.json") != execution["receipt_path"]:
        raise UnresolvedError("registered receipt path differs")
    if root.is_symlink():
        raise UnresolvedError("execution control directory cannot be a symlink")
    return root


def owned_resource(runtime, execution):
    identity = engine_identity(runtime)
    if execution["engine_id"] is not None and execution["engine_id"] != identity:
        raise UnresolvedError("registered Docker engine differs")
    result = docker(
        runtime,
        ["inspect", execution["sandbox_id"] or execution["sandbox_name"]],
        check=False,
    )
    if result.returncode:
        # Confirm an absent name with a successful engine query; arbitrary
        # transport and daemon failures must not be interpreted as deletion.
        names = docker(
            runtime,
            [
                "ps",
                "-a",
                "--no-trunc",
                "--filter",
                "name=^/" + execution["sandbox_name"] + "$",
                "--format",
                "{{.ID}}",
            ],
        )
        if names.stdout.strip():
            raise UnresolvedError("resource inspection is unresolved")
        return None
    resource = json.loads(result.stdout)[0]
    labels = resource["Config"].get("Labels") or {}
    if (
        labels.get("xgent.execution") != execution["remote_task_id"]
        or labels.get("xgent.input") != execution["input_hash"]
    ):
        raise UnresolvedError("resource ownership labels differ")
    if (
        execution["sandbox_id"] is not None
        and resource["Id"] != execution["sandbox_id"]
    ):
        raise UnresolvedError("registered container ID differs")
    return resource


def container_exec(
    runtime, execution, args, *, user="root", cwd=None, env=None, timeout=60, check=True
):
    arguments = ["exec", "--user", str(user)]
    if cwd:
        arguments += ["--workdir", cwd]
    for key, value in (env or {}).items():
        arguments += ["--env", key + "=" + value]
    return docker(
        runtime,
        [*arguments, execution["sandbox_id"], *args],
        timeout=timeout,
        check=check,
    )


def container_files(runtime, execution, files):
    archive = io.BytesIO()
    with tarfile.open(fileobj=archive, mode="w") as stream:
        for name, row in files.items():
            path = Path(name)
            if not path.is_absolute() or ".." in path.parts:
                raise RemoteError(
                    "container file path is outside the declared namespace"
                )
            raw = base64.b64decode(row["data"], validate=True)
            info = tarfile.TarInfo(name.lstrip("/"))
            info.size = len(raw)
            info.mode = row.get("mode", 0o644)
            stream.addfile(info, io.BytesIO(raw))
    docker(
        runtime, ["cp", "-", execution["sandbox_id"] + ":/"], content=archive.getvalue()
    )


def encoded_file(content, mode=0o644):
    return {"data": base64.b64encode(content).decode("ascii"), "mode": mode}


def install_codex_archive(destination, version, architecture):
    label = version + "-linux-" + architecture
    url = "https://registry.npmjs.org/@openai%2Fcodex/" + label
    with urllib.request.urlopen(url, timeout=120) as response:
        metadata = json.load(response)
    tarball = "https://registry.npmjs.org/@openai/codex/-/codex-" + label + ".tgz"
    if (
        metadata.get("name") != "@openai/codex"
        or metadata.get("version") != label
        or metadata.get("os") != ["linux"]
        or metadata.get("cpu") != [architecture]
        or metadata["dist"]["tarball"] != tarball
    ):
        raise RemoteError("fixed Codex package metadata differs")
    with urllib.request.urlopen(tarball, timeout=120) as response:
        raw = response.read()
    integrity = "sha512-" + base64.b64encode(hashlib.sha512(raw).digest()).decode()
    if metadata["dist"]["integrity"] != integrity:
        raise RemoteError("fixed Codex package integrity differs")
    destination = Path(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        raise RemoteError("native Codex installation destination already exists")
    with tempfile.TemporaryDirectory(
        prefix=".codex-", dir=destination.parent
    ) as temporary:
        staged = Path(temporary) / "package"
        with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
            for member in archive:
                path = Path(member.name)
                if (
                    path.is_absolute()
                    or ".." in path.parts
                    or not path.parts
                    or path.parts[0] != "package"
                ):
                    raise RemoteError(
                        "native Codex archive path is outside its package"
                    )
                if member.isdir():
                    continue
                if not member.isfile():
                    raise RemoteError("native Codex archive contains a nonregular file")
                target = staged.joinpath(*path.parts[1:])
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.extractfile(member).read())
                target.chmod(member.mode & 0o777)
        staged.rename(destination)


def instruction_inventory(repo, home, expected):
    repo, home = Path(repo), Path(home)
    paths = set()
    for root, directories, files in os.walk(repo, followlinks=False):
        directories[:] = [name for name in directories if name != ".git"]
        paths.update(
            Path(root) / name
            for name in files
            if name in {"AGENTS.md", "AGENTS.override.md", "SKILL.md"}
        )
    for root in [repo, *repo.parents]:
        paths.update(
            root / name
            for name in ["AGENTS.md", "AGENTS.override.md"]
            if (root / name).exists()
        )
        paths.update(
            path for path in (root / ".agents/skills").rglob("*") if path.is_file()
        )
    for root in [home / ".agents/skills", home / ".codex/skills", Path("/etc/codex")]:
        paths.update(path for path in root.rglob("*") if path.is_file())
    paths.update(
        home / ".codex" / name
        for name in ["AGENTS.md", "AGENTS.override.md", "memory.md"]
        if (home / ".codex" / name).exists()
    )
    for path in list(paths):
        if path.name == "SKILL.md":
            paths.update(item for item in path.parent.rglob("*") if item.is_file())
    targets = set()
    for name in expected:
        path = Path(name)
        if path.name == "SKILL.md" and path.is_relative_to(home):
            match = re.search(
                rb"(?m)^name:[ \t]*['\"]?([a-z0-9][a-z0-9-]*)", path.read_bytes()
            )
            if match:
                targets.add(match[1])
    sources, gaps = [], []
    for path in sorted(paths):
        if path.is_symlink() or any(
            parent.is_symlink()
            for parent in path.parents
            if parent.is_relative_to(repo) or parent.is_relative_to(home)
        ):
            gaps.append("symlink_instruction:" + str(path))
            continue
        content = path.read_bytes()
        hashed = digest(content)
        if str(path) in expected and hashed == expected[str(path)]["sha256"]:
            origin = "injected"
        elif path.is_relative_to(repo):
            original = subprocess.run(
                [
                    "git",
                    "-c",
                    "safe.directory=" + str(repo),
                    "-C",
                    str(repo),
                    "show",
                    "HEAD:" + path.relative_to(repo).as_posix(),
                ],
                capture_output=True,
            )
            origin = (
                "repository_background"
                if original.returncode == 0 and original.stdout == content
                else "unknown"
            )
        else:
            origin = "unknown"
        if origin == "unknown":
            gaps.append("undeclared_instruction:" + str(path))
        if path.name == "SKILL.md" and str(path) not in expected:
            match = re.search(rb"(?m)^name:[ \t]*['\"]?([a-z0-9][a-z0-9-]*)", content)
            if match and match[1] in targets:
                gaps.append("duplicate_target_skill:" + str(path))
        sources.append({"path": str(path), "sha256": hashed, "origin": origin})
    return {"sources": sources, "gaps": sorted(set(gaps))}


def credential_values(value):
    result = []
    if isinstance(value, dict):
        for key, item in value.items():
            if (
                isinstance(item, str)
                and len(item) >= 16
                and any(word in key.upper() for word in ["TOKEN", "API_KEY", "SECRET"])
            ):
                result.append(item)
            else:
                result.extend(credential_values(item))
    elif isinstance(value, list):
        for item in value:
            result.extend(credential_values(item))
    return result


def tooling_manifest(directory):
    directory = Path(directory).resolve(strict=True)
    result = []
    for path in sorted(directory.rglob("*")):
        mode = path.lstat().st_mode
        entry = {
            "path": path.relative_to(directory).as_posix(),
            "mode": stat.S_IMODE(mode),
        }
        if path.is_symlink():
            if not path.resolve().is_relative_to(directory):
                raise UnresolvedError("cached tooling link escapes its package")
            entry["link"] = os.readlink(path)
        elif path.is_file():
            entry["sha256"] = digest(path.read_bytes())
        elif path.is_dir():
            continue
        else:
            raise UnresolvedError("cached tooling contains a special file")
        result.append(entry)
    if not result:
        raise UnresolvedError("cached tooling package is empty")
    return result


def codex_configuration(model="gpt-6.1-sol", effort="high", *, schema_version=1):
    configuration = {
        "model": model,
        "model_reasoning_effort": effort,
        "approval_policy": "never",
        "default_permissions": "evolution",
        "web_search": "disabled",
        "mcp_servers": {},
        "notify": [],
        "features": {
            "multi_agent": False,
            "multi_agent_v2": False,
            "plugins": False,
            "apps": False,
            "shell_snapshot": False,
            "network_proxy": True,
        },
        "shell_environment_policy": {
            "inherit": "none",
            "ignore_default_excludes": False,
            "experimental_use_profile": False,
            "set": {
                "HOME": "/home/evolution",
                "PATH": TASK_PATH,
                "GOPATH": "/go",
                "GOCACHE": "/tmp/evolution-go-cache",
                "LANG": "C.UTF-8",
            },
        },
        "permissions": {
            "evolution": {
                "filesystem": {
                    ":minimal": "read",
                    ":workspace_roots": "write",
                    "/tmp": "write",
                    "/usr/local": "read",
                    "/opt/evolution-codex": "read",
                    "/go": "write",
                    "/home/evolution/.nvm": "read",
                    "/home/evolution/.agents": "read",
                    "/evolution-input": "read",
                    "/home/evolution/.codex": "deny",
                    "/evolution-control": "deny",
                    "/tests": "deny",
                    "/solution": "deny",
                },
                "network": {
                    "enabled": True,
                    "mode": "limited",
                    "allow_local_binding": False,
                    "domains": {"localhost": "allow", "127.0.0.1": "allow"},
                },
            }
        },
    }
    if schema_version == 2:
        filesystem = configuration["permissions"]["evolution"]["filesystem"]
        # The managed proxy writes public CA bundles beside private Codex state.
        # A parent deny also hides the injected public CA from child tools.
        del filesystem["/home/evolution/.codex"]
        # A directory component keeps rg's filename globs from recursing into
        # hidden state directories and layering a second mask over their files.
        filesystem.update({path: "deny" for path in (
            "/home/evolution/[.]codex/auth*",
            "/home/evolution/[.]codex/*.json*",
            "/home/evolution/[.]codex/config*.toml",
            "/home/evolution/.codex/.env",
            "/home/evolution/.codex/sessions",
            "/home/evolution/.codex/archived_sessions",
            "/home/evolution/.codex/log",
            "/home/evolution/[.]codex/*.sqlite*",
            "/home/evolution/[.]codex/.tmp*",
        )})
        filesystem["/home/evolution/.codex/proxy/**"] = "read"
    return configuration


def toml_value(value):
    if isinstance(value, dict):
        return (
            "{"
            + ",".join(
                json.dumps(key) + "=" + toml_value(item)
                for key, item in sorted(value.items())
            )
            + "}"
        )
    return json.dumps(value, ensure_ascii=False, allow_nan=False)


def codex_arguments(configuration, cwd, *, prompt=None, probe=None):
    overrides = [
        part
        for key, value in sorted(configuration.items())
        for part in ["-c", key + "=" + toml_value(value)]
    ]
    if probe is not None:
        return [
            "/usr/local/bin/codex",
            "sandbox",
            "-C",
            cwd,
            "-P",
            "evolution",
            *overrides,
            "--",
            "python3",
            "-c",
            probe,
        ]
    return [
        "/usr/local/bin/codex",
        "exec",
        "--strict-config",
        "--ignore-user-config",
        "--ignore-rules",
        "--skip-git-repo-check",
        "--json",
        "-C",
        cwd,
        *overrides,
        "--",
        prompt,
    ]


def create_resource(runtime, execution, payload):
    if (
        digest(encoded(payload)) != execution["input_hash"]
        or digest(encoded(payload["command"])) != execution["command_hash"]
    ):
        raise RemoteError("submitted inputs differ from their frozen hashes")
    if payload["command"]["uid"] != 10000 or payload["command"]["gid"] != 10000:
        raise RemoteError("task command must use the unprivileged task identity")
    control = host_control(execution)
    control.mkdir(mode=0o700, parents=True, exist_ok=False)
    save_json(control / "receipt.json", execution)
    save_json(control / "runtime.json", runtime)
    save_json(control / "payload.json", payload)
    save_bytes(control / "remote.py", Path(__file__).read_bytes())
    identity = engine_identity(runtime)
    present = docker(runtime, ["image", "inspect", payload["image"]], check=False)
    if present.returncode:
        docker(
            runtime,
            ["pull", "--platform", payload["platform"], payload["image"]],
            timeout=300,
        )
    remaining = int(execution["resource_deadline"] - time.time())
    if remaining <= 0:
        raise RemoteError("run deadline reached before resource creation")
    args = [
        "run",
        "--detach",
        "--init",
        "--name",
        execution["sandbox_name"],
        "--platform",
        payload["platform"],
        "--label",
        "xgent.execution=" + execution["remote_task_id"],
        "--label",
        "xgent.input=" + execution["input_hash"],
        "--cpus",
        "1",
        "--memory",
        "4g",
        "--pids-limit",
        "512",
        "--ulimit",
        "core=0",
        "--security-opt",
        "no-new-privileges",
        "--cap-drop",
        "NET_RAW",
        "--cap-add",
        "NET_ADMIN",
        "--security-opt",
        "seccomp=unconfined",
        "--security-opt",
        "apparmor=" + runtime.get("apparmor_profile", "unconfined"),
        "--entrypoint",
        "/bin/sh",
        payload["image"],
        "-c",
        "sleep " + str(remaining),
    ]
    created = docker(runtime, args, timeout=300)
    execution.update(
        sandbox_id=created.stdout.decode().strip(),
        engine_id=identity,
        submission_status="resource_created",
    )
    save_json(control / "receipt.json", execution)
    resource = owned_resource(runtime, execution)
    if resource.get("AppArmorProfile") != runtime.get("apparmor_profile", "unconfined"):
        raise RemoteError(
            "actual container AppArmor profile differs from the frozen runtime"
        )
    if any(item["Type"] == "bind" for item in resource.get("Mounts", [])):
        raise RemoteError("task container has an undeclared host bind mount")
    removed = payload.get("remove_paths", [])
    root = Path(payload["repo_path"])
    if any(
        not Path(name).is_relative_to(root)
        or Path(name) == root
        or ".." in Path(name).parts
        or name in payload["files"]
        for name in removed
    ):
        raise RemoteError(
            "input removal is outside the task repository or conflicts with staged bytes"
        )
    if removed:
        container_exec(
            runtime,
            execution,
            [
                "python3",
                "-c",
                "import json,pathlib,sys; "
                "[pathlib.Path(name).unlink(missing_ok=True) for name in json.loads(sys.argv[1])]",
                encoded(removed).decode(),
            ],
        )
    files = dict(payload["files"])
    files.update(
        {
            "/evolution-control/remote.py": encoded_file(Path(__file__).read_bytes()),
            "/evolution-control/command.json": encoded_file(
                encoded(payload["command"])
            ),
            "/evolution-control/payload.json": encoded_file(encoded(payload)),
            "/evolution-control/receipt.json": encoded_file(encoded(execution)),
        }
    )
    container_files(runtime, execution, files)
    setup = (
        "import os,pathlib; "
        "p=pathlib.Path('/home/evolution');p.mkdir(parents=True,exist_ok=True);os.chown(p,10000,10000); "
        "os.chmod('/evolution-control',0o700)"
    )
    container_exec(runtime, execution, ["python3", "-c", setup])
    basic = """
import base64, hashlib, json, os, pathlib, subprocess, sys
packet = json.loads(pathlib.Path('/evolution-control/payload.json').read_text())
repo = pathlib.Path(packet['repo_path'])
if packet.get('generated_workspace'):
    subprocess.run(['git','-C',str(repo),'init','-q'],check=True)
    subprocess.run(['git','-C',str(repo),'add','-A'],check=True)
    subprocess.run(['git','-C',str(repo),'-c','user.name=Evaluation','-c','user.email=evaluation@invalid','-c','core.hooksPath=/dev/null','commit','-qm','Input snapshot'],check=True)
actual = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
if not packet.get('generated_workspace') and actual != packet['base_commit']:
    raise ValueError('task image base differs: ' + actual)
git = ['git', '-c', 'core.hooksPath=/dev/null', '-c', 'core.fsmonitor=false', '-C', str(repo)]
if not packet.get('generated_workspace'):
    ancestors = set(subprocess.check_output([*git, 'rev-list', packet['base_commit']], text=True).splitlines())
    objects = subprocess.check_output([*git, 'cat-file', '--batch-all-objects', '--batch-check=%(objectname) %(objecttype)'], text=True).splitlines()
    commits = {line.split()[0] for line in objects if line.endswith(' commit')}
    if not commits <= ancestors:
        raise ValueError('task image contains later Git commits')
subprocess.run([*git, 'add', '-A', '--', '.'], check=True)
tree = subprocess.check_output([*git, 'write-tree'], text=True).strip()
pathlib.Path('/evolution-control/base-tree.txt').write_text(tree)
subprocess.run(['chown', '-R', '10000:10000', str(repo)], check=True)
if pathlib.Path('/go').is_dir():
    subprocess.run(['chown', '-R', '10000:10000', '/go'], check=True)
if packet.get('grading'):
    logs = pathlib.Path('/logs/verifier')
    logs.mkdir(parents=True, exist_ok=True)
    os.chown(logs, 10000, 10000)
files = {}
for name in packet.get('remove_paths', []):
    if pathlib.Path(name).exists() or pathlib.Path(name).is_symlink():
        raise ValueError('removed instruction is still present')
for name, row in packet['files'].items():
    path = pathlib.Path(name)
    expected = base64.b64decode(row['data'])
    if path.is_symlink() or path.read_bytes() != expected:
        raise ValueError('staged file differs')
    files[name] = hashlib.sha256(expected).hexdigest()
receipt = {'base_commit': actual, 'baseline_tree': tree, 'input_files': files, 'removed_paths': packet.get('remove_paths', []), 'uid': 10000}
pathlib.Path('/evolution-control/instruction-receipt.json').write_text(json.dumps(receipt))
print(json.dumps(receipt))
"""
    prepared = container_exec(
        runtime,
        execution,
        ["python3", "-c", basic],
        timeout=max(1, min(180, execution["command_deadline"] - time.time())),
        check=False,
    )
    if prepared.returncode:
        raise RemoteError(
            "task image or staged input did not match its frozen contract"
        )
    receipt = json.loads(prepared.stdout)
    receipt.update(
        image_id=resource["Image"],
        apparmor_profile=resource.get("AppArmorProfile"),
        mounts=[
            {
                "type": item["Type"],
                "destination": item["Destination"],
                "read_write": item["RW"],
            }
            for item in resource.get("Mounts", [])
        ],
    )
    save_json(control / "instruction-receipt.json", receipt)
    container_files(
        runtime,
        execution,
        {"/evolution-control/instruction-receipt.json": encoded_file(encoded(receipt))},
    )
    return {"sandbox_id": execution["sandbox_id"], "engine_id": identity}


def inner_inspection(runtime, execution):
    result = container_exec(
        runtime,
        execution,
        ["python3", "/evolution-control/remote.py", "--inspect", "/evolution-control"],
        check=False,
    )
    if result.returncode:
        raise UnresolvedError("original command receipt is unavailable")
    return json.loads(result.stdout)


def mirror_capture(runtime, execution, receipt):
    control = host_control(execution)
    with tempfile.TemporaryDirectory(prefix="capture-", dir=control) as temporary:
        target = Path(temporary) / "artifacts"
        docker(
            runtime,
            [
                "cp",
                execution["sandbox_id"] + ":/evolution-control/artifacts",
                str(target),
            ],
        )
        for row in receipt["artifact_manifest"]:
            name = Path(row["path"])
            if name.is_absolute() or ".." in name.parts:
                raise UnresolvedError("artifact manifest escapes capture directory")
            source = target / name
            raw = source.read_bytes()
            if len(raw) != row["size"] or digest(raw) != row["sha256"]:
                raise UnresolvedError("captured artifact hash differs")
            copy_artifact(source, control / "artifacts" / name)
    save_json(control / "receipt.json", receipt)
    if (control / "typesafe-credential.json").exists():
        destroy_typesafe_credential(runtime, execution, owned_resource(runtime, execution))


def host_supervise(control):
    control = Path(control)
    runtime = read_json(control / "runtime.json")
    execution = read_json(control / "receipt.json")
    descriptor = os.open(
        control / "host-launch.claim", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
    )
    os.close(descriptor)
    docker(
        runtime,
        [
            "exec",
            "--detach",
            "--user",
            "root",
            execution["sandbox_id"],
            "python3",
            "/evolution-control/remote.py",
            "--supervise",
            "/evolution-control",
        ],
    )
    while time.time() < execution["resource_deadline"]:
        try:
            inspected = inner_inspection(runtime, execution)
            receipt = inspected["receipt"]
            if receipt["phase"] == "captured":
                mirror_capture(runtime, execution, receipt)
                return receipt
            save_json(control / "receipt.json", receipt)
        except (RemoteError, OSError, json.JSONDecodeError):
            pass
        time.sleep(0.5)
    raise UnresolvedError(
        "capture window expired; preserve the registered resource and receipts"
    )


def dispatch_request(request):
    runtime, execution, payload = (
        request["runtime"],
        request["execution"],
        request.get("payload"),
    )
    if (runtime["ssh_target"], runtime["remote_root"]) != (
        execution["ssh_target"],
        execution["remote_root"],
    ):
        raise UnresolvedError("SSH execution scope differs")
    if digest(Path(__file__).read_bytes()) != execution["runner_hash"]:
        raise UnresolvedError("deployed runner differs from the frozen source")
    operation = request["operation"]
    if operation == "create":
        return create_resource(runtime, execution, payload)
    control = host_control(execution)
    original = read_json(control / "receipt.json")
    for key in ["remote_task_id", "input_hash", "command_hash", "runner_hash"]:
        if original[key] != execution[key]:
            raise UnresolvedError("stored execution identity differs")
    if operation == "read_artifact":
        matches = [
            row
            for row in original["artifact_manifest"]
            if row["path"] == payload["path"]
        ]
        if original["phase"] != "captured" or len(matches) != 1:
            raise UnresolvedError("artifact is not in the captured manifest")
        return {
            "data": base64.b64encode(
                (control / "artifacts" / matches[0]["path"]).read_bytes()
            ).decode("ascii")
        }
    resource = owned_resource(runtime, original)
    if operation == "destroy_typesafe":
        return {"destroyed": destroy_typesafe_credential(runtime, original, resource)}
    if operation == "prepare_typesafe":
        if resource is None or original["submission_status"] != "resource_created":
            raise UnresolvedError("credential preparation requires the original prepared resource")
        return prepare_typesafe_credential(runtime, original, payload)
    if operation == "query":
        if original["phase"] == "captured" and (control / "artifacts").is_dir():
            destroy_typesafe_credential(runtime, original, resource)
            return {
                "receipt": original,
                "resource_state": "missing"
                if resource is None
                else resource["State"]["Status"],
                "command_alive": False,
                "supervisor_alive": False,
            }
        if resource is None or not resource["State"]["Running"]:
            raise UnresolvedError(
                "resource ended before complete evidence was recovered"
            )
        inspected = inner_inspection(runtime, original)
        if inspected["receipt"]["phase"] == "captured":
            mirror_capture(runtime, original, inspected["receipt"])
        return {**inspected, "resource_state": resource["State"]["Status"]}
    if operation == "cleanup":
        if original["phase"] != "captured" and not payload.get("force_reason"):
            raise UnresolvedError("cleanup would discard uncaptured evidence")
        credentials_destroyed = destroy_typesafe_credential(runtime, original, resource)
        if resource is not None:
            docker(runtime, ["stop", "--time", "2", resource["Id"]])
            docker(runtime, ["rm", "--volumes", resource["Id"]])
        return {"terminated": owned_resource(runtime, original) is None and credentials_destroyed}
    if resource is None:
        raise UnresolvedError("registered container is unavailable")
    if operation == "cancel":
        container_files(
            runtime,
            original,
            {"/evolution-control/cancel.json": encoded_file(encoded(payload))},
        )
        return {"cancel_requested": True}
    if operation == "capture_only":
        captured = container_exec(
            runtime,
            original,
            [
                "python3",
                "/evolution-control/remote.py",
                "--capture-only",
                "/evolution-control",
            ],
            check=False,
        )
        if captured.returncode:
            raise UnresolvedError(
                "original processes could not be confirmed stopped for capture-only"
            )
        receipt = json.loads(captured.stdout)
        mirror_capture(runtime, original, receipt)
        return {"receipt": receipt}
    if operation in {"restore_codex", "cache_codex"}:
        if (control / "host-launch.claim").exists():
            raise UnresolvedError("tooling cache cannot change after task startup")
        cache = (
            Path(runtime["remote_root"])
            / "tools"
            / (
                "codex-"
                + runtime["codex_version"]
                + "-"
                + runtime["codex_architecture"]
            )
        )
        package, index = cache / "package", cache / "manifest.json"
        if operation == "restore_codex":
            if not index.exists():
                return {"restored": False}
            entries = tooling_manifest(package)
            if entries != read_json(index):
                raise UnresolvedError("cached tooling differs from its saved manifest")
            docker(
                runtime,
                ["cp", str(package), original["sandbox_id"] + ":/opt/evolution-codex"],
                timeout=120,
            )
            return {"restored": True, "manifest_sha256": digest(encoded(entries))}
        version = (
            container_exec(runtime, original, ["/usr/local/bin/codex", "--version"])
            .stdout.decode()
            .strip()
        )
        if version != "codex-cli " + runtime["codex_version"]:
            raise RemoteError("cannot cache a different Codex version")
        staging = control / "installed-tooling"
        docker(
            runtime,
            ["cp", original["sandbox_id"] + ":/opt/evolution-codex", str(staging)],
            timeout=120,
        )
        entries = tooling_manifest(staging)
        cache.mkdir(mode=0o700, parents=True, exist_ok=True)
        if index.exists():
            if read_json(index) != entries or tooling_manifest(package) != entries:
                raise UnresolvedError(
                    "installed tooling differs from the existing cache"
                )
        else:
            staging.rename(package)
            save_json(index, entries)
        return {"cached": True, "manifest_sha256": digest(encoded(entries))}
    if operation == "setup_exec":
        if (control / "host-launch.claim").exists():
            raise UnresolvedError(
                "setup cannot execute after the original command has started"
            )
        result = container_exec(
            runtime,
            original,
            ["bash", "-c", payload["command"]],
            user=payload.get("user") or "root",
            cwd=payload.get("cwd"),
            env=payload.get("env"),
            timeout=payload.get("timeout_sec") or 300,
            check=False,
        )
        observation = {
            "return_code": result.returncode,
            "stdout": result.stdout.decode(errors="replace"),
            "stderr": result.stderr.decode(errors="replace"),
        }
        with (control / "setup.jsonl").open("ab") as stream:
            stream.write(encoded(observation) + b"\n")
        return observation
    if operation == "setup_files":
        if (control / "host-launch.claim").exists():
            raise UnresolvedError("setup cannot alter a started execution")
        container_files(runtime, original, payload)
        return {"staged": True}
    if operation == "finalize_setup":
        if (control / "host-launch.claim").exists():
            raise UnresolvedError("setup cannot alter a started execution")
        packet = read_json(control / "payload.json")
        authentication = runtime.get("authentication")
        if authentication is not None:
            authentication_bytes = Path(authentication["path"]).read_bytes()
            container_files(
                runtime,
                original,
                {
                    "/home/evolution/.codex/auth.json": encoded_file(
                        authentication_bytes, 0o600
                    ),
                    "/evolution-control/redaction.json": encoded_file(
                        encoded(credential_values(json.loads(authentication_bytes))),
                        0o600,
                    ),
                },
            )
        setup = (
            "import base64,hashlib,json,os,pathlib,subprocess,sys; "
            "sys.path.insert(0,'/evolution-control'); from remote import instruction_inventory; "
            "packet=json.loads(pathlib.Path('/evolution-control/payload.json').read_text()); "
            "home=pathlib.Path('/home/evolution'); "
            "[(os.chown(p,10000,10000)) for p in [home,*home.rglob('*')] if not p.is_symlink()]; "
            "binary=pathlib.Path('/usr/local/bin/codex').resolve(); "
            "candidates=[binary]+list(binary.parent.parent.parent.rglob('codex')); "
            "native=next(p for p in candidates if p.is_file() and p.open('rb').read(4)==b'\\x7fELF'); "
            "receipt=json.loads(pathlib.Path('/evolution-control/instruction-receipt.json').read_text()); "
            "inventory=instruction_inventory(pathlib.Path(packet['repo_path']),home,{name:{'sha256':hashlib.sha256(base64.b64decode(row['data'])).hexdigest()} for name,row in packet['files'].items()}); "
            "receipt.update({'instruction_sources':inventory['sources'],'source_gaps':inventory['gaps']}); "
            "receipt.update({'codex_version':subprocess.check_output(['/usr/local/bin/codex','--version'],text=True).strip(),"
            "'binary_sha256':hashlib.sha256(native.read_bytes()).hexdigest(),"
            "'launcher_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),"
            "'config_sha256':hashlib.sha256(pathlib.Path('/home/evolution/.codex/config.toml').read_bytes()).hexdigest()}); "
            "pathlib.Path('/evolution-control/instruction-receipt.json').write_text(json.dumps(receipt)); "
            "assert not inventory['gaps'], 'undeclared instruction sources'; "
            "print(json.dumps(receipt))"
        )
        result = container_exec(runtime, original, ["python3", "-c", setup])
        receipt = json.loads(result.stdout)
        if receipt["codex_version"] != "codex-cli " + runtime["codex_version"]:
            raise RemoteError("installed Codex version differs from the frozen runtime")
        if any(
            receipt.get(name) != expected
            for name, expected in packet.get("expected_runtime", {}).items()
        ):
            raise RemoteError("actual Codex runtime differs from the calibrated binary")
        expected_config = packet["codex_config"]
        receipt["configuration"] = expected_config
        save_json(control / "instruction-receipt.json", receipt)
        container_files(
            runtime,
            original,
            {
                "/evolution-control/instruction-receipt.json": encoded_file(
                    encoded(receipt)
                )
            },
        )
        return receipt
    if operation == "launch":
        if (control / "host-launch.claim").exists():
            return {"launched": True}
        with (control / "host-runner.log").open("ab") as output:
            process = subprocess.Popen(
                [
                    sys.executable,
                    str(control / "remote.py"),
                    "--host-supervise",
                    str(control),
                ],
                stdin=subprocess.DEVNULL,
                stdout=output,
                stderr=output,
                start_new_session=True,
                env=host_environment(runtime),
            )
        save_json(
            control / "host-supervisor.json",
            {
                "pid": process.pid,
                "pgid": process.pid,
                "start_identity": process_identity(process.pid),
            },
        )
        return {"launched": True}
    raise RemoteError("unknown remote operation")


def process_identity(pid):
    try:
        proc = Path("/proc") / str(pid)
        if proc.is_dir():
            # Emulated /proc/stat start ticks can differ by one tick between
            # self and external readers. The task directory inode is stable
            # for that process lifetime and changes when the PID is reused.
            return (
                Path("/proc/sys/kernel/random/boot_id").read_text().strip()
                + ":"
                + str(proc.stat().st_ino)
            )
        result = subprocess.run(
            ["ps", "-o", "lstart=", "-p", str(pid)], capture_output=True, text=True
        )
        return result.stdout.strip() or None
    except (OSError, ValueError, IndexError):
        return None


def process_alive(process):
    if process is None:
        return False
    try:
        os.kill(process["pid"], 0)
        return process_identity(process["pid"]) == process["start_identity"]
    except (OSError, KeyError):
        return False


def inspect_command(control):
    control = Path(control)
    receipt = read_json(control / "receipt.json")
    runner_path = control / "supervisor.json"
    runner = read_json(runner_path) if runner_path.exists() else None
    command_alive = process_alive(receipt.get("process"))
    supervisor_alive = process_alive(runner)
    latest = read_json(control / "receipt.json")
    if latest != receipt:
        receipt = latest
        command_alive = process_alive(receipt.get("process"))
    return {
        "receipt": receipt,
        "command_alive": command_alive,
        "supervisor_alive": supervisor_alive,
        "claimed": (control / "launch.claim").exists(),
    }


def process_options(command):
    options = {"cwd": command["cwd"], "env": command["env"], "start_new_session": True}
    if command["uid"] is not None:
        options.update(user=command["uid"], group=command["gid"], extra_groups=[])
    return options


def task_pids(uid):
    # The production supervisor is root inside its own container. Never scan
    # or signal the developer's user during deterministic process fixtures.
    if uid is None or uid < 10000 or os.geteuid() != 0 or not Path("/proc").is_dir():
        return []
    result = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            for line in (entry / "status").read_text().splitlines():
                if line.startswith("Uid:") and int(line.split()[1]) == uid:
                    result.append(int(entry.name))
        except (OSError, ValueError):
            continue
    return result


def stop_processes(process, uid, child=None, grace=0.5):
    for sig in [signal.SIGTERM, signal.SIGKILL]:
        if process is not None:
            try:
                os.killpg(process["pgid"], sig)
            except ProcessLookupError:
                pass
        for pid in task_pids(uid):
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                pass
        if sig == signal.SIGTERM:
            time.sleep(grace)
            if child is not None:
                # Reap a terminated leader before signalling remaining group
                # members; macOS rejects killpg against a zombie-only group.
                child.poll()


def copy_artifact(source, destination):
    source = Path(source)
    path = source
    while path != path.parent:
        if path.is_symlink():
            raise ValueError("artifact source cannot be a symlink")
        path = path.parent
    descriptor = os.open(source, os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW)
    with os.fdopen(descriptor, "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("artifact source must be a regular file")
        save_bytes(destination, stream.read())


def capture_artifacts(control, command):
    artifacts = control / "artifacts"
    artifacts.mkdir(mode=0o700, exist_ok=True)
    gaps = []
    capture = command["capture"]
    repository = capture["repo"]
    if repository is not None:
        env = {
            **command["env"],
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": "/dev/null",
        }
        options = {**process_options(command), "cwd": repository, "env": env}
        git = [
            "git",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.pager=cat",
        ]
        added = subprocess.run(
            [*git, "add", "-A", "--", "."], capture_output=True, timeout=30, **options
        )
        baseline = (
            Path(capture["baseline_ref_file"]).read_text().strip()
            if capture.get("baseline_ref_file")
            else "HEAD"
        )
        patch = subprocess.run(
            [
                *git,
                "diff",
                "--cached",
                "--binary",
                "--no-ext-diff",
                "--no-textconv",
                baseline,
                "--",
                ".",
                *[":(exclude)" + name for name in capture["excluded_paths"]],
            ],
            capture_output=True,
            timeout=30,
            **options,
        )
        if added.returncode or patch.returncode:
            gaps.append("patch_capture_failed")
            save_json(
                artifacts / "patch-capture.json",
                {
                    "add_exit_code": added.returncode,
                    "diff_exit_code": patch.returncode,
                    "add_stderr": added.stderr.decode(errors="replace"),
                    "diff_stderr": patch.stderr.decode(errors="replace"),
                },
            )
        else:
            save_bytes(artifacts / "model.patch", patch.stdout)
    session = capture["session_dir"]
    runtime_files, runtime_names = {}, []
    if session is not None:
        root = Path(session)
        if root.is_dir() and not root.is_symlink():
            for source in sorted(root.rglob("*")):
                if source.is_dir() and not source.is_symlink():
                    continue
                try:
                    copy_artifact(
                        source, artifacts / "sessions" / source.relative_to(root)
                    )
                except (OSError, ValueError):
                    gaps.append("session_capture_failed")
        if capture.get("runtime_skills"):
            builtins = root.parent / "skills/.system"
            for entry in sorted(builtins.glob("*/SKILL.md")):
                runtime_names.append(entry.parent.name)
                for source in sorted(entry.parent.rglob("*")):
                    if source.is_dir() and not source.is_symlink():
                        continue
                    if "__pycache__" in source.parts or source.suffix == ".pyc":
                        continue
                    name = source.relative_to(builtins).as_posix()
                    try:
                        copy_artifact(source, artifacts / "runtime-skills" / name)
                        runtime_files[name] = stat.S_IMODE(source.stat().st_mode)
                    except (OSError, ValueError):
                        gaps.append("runtime_skill_capture_failed")
            if not runtime_names:
                gaps.append("runtime_skills_unavailable")
    for name, source in capture["files"].items():
        path = Path(name)
        if path.is_absolute() or ".." in path.parts:
            raise ValueError("artifact name must stay relative")
        try:
            copy_artifact(source, artifacts / path)
        except (OSError, ValueError):
            gaps.append("missing_artifact:" + name)
    for name, source in capture.get("optional_files", {}).items():
        path = Path(name)
        if path.is_absolute() or ".." in path.parts:
            raise ValueError("optional artifact name must stay relative")
        if Path(source).exists():
            copy_artifact(source, artifacts / path)
    secrets = (
        read_json(control / "redaction.json")
        if (control / "redaction.json").exists()
        else []
    )
    auth_path = Path(session).parent / "auth.json" if session is not None else None
    if auth_path is not None and auth_path.is_file():
        secrets.extend(credential_values(read_json(auth_path)))
    redacted = []
    for path in sorted(artifacts.rglob("*")):
        if not path.is_file():
            continue
        raw = path.read_bytes()
        changed = raw
        for secret in set(secrets):
            changed = changed.replace(secret.encode(), b"[REDACTED]")
        if changed != raw:
            if path.name.endswith(".patch"):
                path.unlink()
                gaps.append("credential_in_patch")
            else:
                save_bytes(path, changed)
                redacted.append(path.relative_to(artifacts).as_posix())
    if auth_path is not None and auth_path.exists():
        auth_path.unlink()
    (control / "redaction.json").unlink(missing_ok=True)
    if command.get("typesafe_credential"):
        credential = Path(command["typesafe_credential"])
        credential.unlink(missing_ok=True)
        save_json(artifacts / "credential-receipt.json", {"provider": "typesafe", "destroyed": not credential.exists()})
    if capture.get("runtime_skills"):
        save_json(
            artifacts / "runtime-skills.json",
            {
                "skills": runtime_names,
                "files": [
                    {
                        "path": name,
                        "mode": mode,
                        "sha256": digest(
                            (artifacts / "runtime-skills" / name).read_bytes()
                        ),
                    }
                    for name, mode in sorted(runtime_files.items())
                ],
            },
        )
    save_json(
        artifacts / "capture.json",
        {"gaps": sorted(set(gaps)), "redacted_artifacts": redacted},
    )
    manifest = []
    for path in sorted(artifacts.rglob("*")):
        if path.is_dir():
            continue
        raw = path.read_bytes()
        manifest.append(
            {
                "path": path.relative_to(artifacts).as_posix(),
                "size": len(raw),
                "sha256": digest(raw),
            }
        )
    return manifest


def verify_supervisor_inputs(control):
    receipt = read_json(control / "receipt.json")
    command = read_json(control / "command.json")
    if digest(encoded(command)) != receipt["command_hash"]:
        raise ValueError("frozen command hash mismatch")
    if digest(Path(__file__).read_bytes()) != receipt["runner_hash"]:
        raise ValueError("frozen supervisor hash mismatch")
    return receipt, command


def run_supervisor(control):
    control = Path(control)
    receipt, command = verify_supervisor_inputs(control)
    try:
        descriptor = os.open(
            control / "launch.claim", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
        )
    except FileExistsError:
        return read_json(control / "receipt.json")
    os.close(descriptor)
    save_json(
        control / "supervisor.json",
        {
            "pid": os.getpid(),
            "pgid": os.getpgrp(),
            "start_identity": process_identity(os.getpid()),
        },
    )
    receipt["phase"] = "starting"
    save_json(control / "receipt.json", receipt)
    artifacts = control / "artifacts"
    artifacts.mkdir(mode=0o700, exist_ok=True)
    child = None
    started = None
    try:
        with (
            (artifacts / "stdout.jsonl").open("wb") as stdout,
            (artifacts / "stderr.txt").open("wb") as stderr,
        ):
            if time.time() >= receipt["command_deadline"]:
                receipt.update(
                    phase="capturing", termination_reason="not_started_deadline"
                )
                save_json(control / "receipt.json", receipt)
                receipt["artifact_manifest"] = capture_artifacts(control, command)
                receipt["phase"] = "captured"
                save_json(control / "receipt.json", receipt)
                return receipt
            started = time.monotonic()
            child = subprocess.Popen(
                command["args"],
                stdin=subprocess.DEVNULL,
                stdout=stdout,
                stderr=stderr,
                **process_options(command),
            )
            identity = process_identity(child.pid)
            receipt["process"] = {
                "pid": child.pid,
                "pgid": child.pid,
                "start_identity": identity or "already_exited",
            }
            receipt["phase"] = "running"
            save_json(control / "receipt.json", receipt)
            reason = "exited"
            while child.poll() is None:
                if (control / "cancel.json").exists():
                    reason = "cancelled"
                    break
                if time.time() >= receipt["command_deadline"]:
                    reason = "timeout"
                    break
                time.sleep(0.05)
            stop_processes(receipt["process"], command["uid"], child)
            receipt["exit_code"] = child.wait(timeout=5)
            receipt["termination_reason"] = reason
    except OSError:
        if child is not None:
            stop_processes(receipt["process"], command["uid"], child)
            receipt["exit_code"] = child.wait(timeout=5)
        receipt["termination_reason"] = "launch_failed"
    receipt["phase"] = "capturing"
    save_json(control / "receipt.json", receipt)
    if started is not None and child is not None and receipt["exit_code"] is not None:
        save_json(
            artifacts / "command-metrics.json",
            {"duration_sec": time.monotonic() - started},
        )
    receipt["artifact_manifest"] = capture_artifacts(control, command)
    receipt["phase"] = "captured"
    save_json(control / "receipt.json", receipt)
    return receipt


def capture_only(control):
    control = Path(control)
    receipt, command = verify_supervisor_inputs(control)
    if receipt["phase"] == "captured":
        return receipt
    inspection = inspect_command(control)
    if (
        inspection["command_alive"]
        or inspection["supervisor_alive"]
        or task_pids(command["uid"])
    ):
        raise ValueError("cannot capture while original execution may still be running")
    receipt["phase"] = "capturing"
    if receipt["exit_code"] is None:
        receipt["termination_reason"] = "original_exit_unknown"
    save_json(control / "receipt.json", receipt)
    receipt["artifact_manifest"] = capture_artifacts(control, command)
    receipt["phase"] = "captured"
    save_json(control / "receipt.json", receipt)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    entry = parser.add_mutually_exclusive_group(required=True)
    entry.add_argument("--supervise", type=Path)
    entry.add_argument("--inspect", type=Path)
    entry.add_argument("--capture-only", type=Path)
    entry.add_argument("--host-supervise", type=Path)
    entry.add_argument("--dispatch", action="store_true")
    args = parser.parse_args()
    try:
        if args.dispatch:
            result = dispatch_request(json.load(sys.stdin))
        elif args.host_supervise:
            result = host_supervise(args.host_supervise)
        elif args.supervise:
            result = run_supervisor(args.supervise)
        elif args.capture_only:
            result = capture_only(args.capture_only)
        else:
            result = inspect_command(args.inspect)
        print(json.dumps(result, ensure_ascii=False))
        return 0
    except Exception as error:
        message = (
            str(error) if isinstance(error, RemoteError) else "remote operation failed"
        )
        print(
            json.dumps(
                {"error": message, "unresolved": isinstance(error, UnresolvedError)}
            )
        )
        return 3 if isinstance(error, UnresolvedError) else 4


if __name__ == "__main__":
    raise SystemExit(main())
