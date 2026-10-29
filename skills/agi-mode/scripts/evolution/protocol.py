"""Validate and freeze experiment inputs without importing a remote runtime."""

from copy import deepcopy
from dataclasses import dataclass, field
from datetime import datetime
import errno
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import tempfile
import uuid


VERSIONS = {"harbor_version": "0.24.0", "codex_version": "0.162.1"}
PRO_V2_REVISION = "66f92766bba642462d4bbe5479e83f91f9211862"
PHASES = {"prepared", "starting", "running", "capturing", "captured"}
ROLES = {"optimizer", "trial", "grader", "probe"}


class ProtocolError(ValueError):
    pass


def canonical(value):
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def sha256(content):
    return hashlib.sha256(content).hexdigest()


def task_contract(case):
    return ("\n\n".join(case["observable_requirements"]) + "\n").encode("utf-8")


def parse_json(content):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ProtocolError(f"duplicate JSON field: {key}")
            result[key] = value
        return result

    def invalid_constant(value):
        raise ProtocolError(f"nonfinite JSON number: {value}")

    return json.loads(
        content.decode("utf-8") if isinstance(content, bytes) else content,
        object_pairs_hook=pairs,
        parse_constant=invalid_constant,
    )


def load_json(path):
    return parse_json(Path(path).read_text(encoding="utf-8"))


def fields(value, label, required, optional=()):
    if not isinstance(value, dict):
        raise ProtocolError(f"{label}: expected object")
    missing = set(required) - value.keys()
    unknown = value.keys() - set(required) - set(optional)
    if missing or unknown:
        raise ProtocolError(
            f"{label}: missing fields {sorted(missing)}, unknown fields {sorted(unknown)}"
        )
    return value


def text(value, label):
    if not isinstance(value, str) or not value.strip():
        raise ProtocolError(f"{label}: expected nonempty string")
    return value


def number(value, label, minimum=0, maximum=None, integer=False):
    if type(value) not in (int, float) or not math.isfinite(value) or value < minimum:
        raise ProtocolError(f"{label}: expected finite number >= {minimum}")
    if integer and type(value) is not int:
        raise ProtocolError(f"{label}: expected integer")
    if maximum is not None and value > maximum:
        raise ProtocolError(f"{label}: exceeds {maximum}")
    return value


def boolean(value, label):
    if type(value) is not bool:
        raise ProtocolError(f"{label}: expected boolean")
    return value


def choice(value, options, label):
    if not isinstance(value, str) or value not in options:
        raise ProtocolError(f"{label}: expected one of {sorted(options)}")
    return value


def rows(value, label, empty=False):
    if not isinstance(value, list) or (not empty and not value):
        raise ProtocolError(
            f"{label}: expected {'possibly empty ' if empty else 'nonempty '}list"
        )
    return value


def fingerprint(value, label, length=64):
    if not isinstance(value, str) or not re.fullmatch(f"[0-9a-f]{{{length}}}", value):
        raise ProtocolError(f"{label}: expected {length}-character content hash")
    return value


def relative_path(name):
    text(name, "path")
    path = PurePosixPath(name)
    if (
        path.is_absolute()
        or ".." in path.parts
        or "\\" in name
        or path.as_posix() != name
        or name == "."
    ):
        raise ProtocolError(f"path must be canonical and relative: {name}")
    if "\x00" in name:
        raise ProtocolError("path contains NUL")
    return name


def remote_path(name):
    text(name, "remote path")
    path = PurePosixPath(name)
    if (
        not path.is_absolute()
        or ".." in path.parts
        or path.as_posix() != name
        or "\x00" in name
        or name == "/"
    ):
        raise ProtocolError(
            "remote paths must be canonical, absolute and below the root"
        )
    return name


def export_path(name):
    relative_path(name)
    parts = PurePosixPath(name).parts
    for index, part in enumerate(parts):
        if (
            part == ".git"
            or part == ".env"
            or part.startswith(".env.")
            or part in {".mcp.json", "auth.json", "credentials.json"}
        ):
            raise ProtocolError(
                f"host configuration or credential path cannot be exported: {name}"
            )
        if part in {".codex", ".claude", ".agents"} and parts[
            index + 1 : index + 2
        ] != ("skills",):
            raise ProtocolError(f"host configuration cannot be exported: {name}")
    return name


def path_list(value, label, empty=False):
    names = [export_path(name) for name in rows(value, label, empty)]
    if len(names) != len(set(names)):
        raise ProtocolError(f"{label}: duplicate paths")
    return names


def covered(name, allowed):
    return any(name == prefix or name.startswith(prefix + "/") for prefix in allowed)


def directory(name, base):
    path = Path(text(name, "directory"))
    if not path.is_absolute():
        path = base / path
    path = path.resolve(strict=True)
    if not path.is_dir():
        raise ProtocolError("expected directory")
    return path


def read_file(root, name):
    export_path(name)
    # O_NONBLOCK avoids opening a FIFO before checking its type; O_NOFOLLOW
    # and directory descriptors keep every lookup inside the authorized root.
    directory_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        parts = PurePosixPath(name).parts
        for part in parts[:-1]:
            child_fd = os.open(
                part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=directory_fd
            )
            os.close(directory_fd)
            directory_fd = child_fd
        descriptor = os.open(
            parts[-1], os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW, dir_fd=directory_fd
        )
    except OSError as error:
        if error.errno in {errno.ELOOP, errno.ENOTDIR}:
            raise ProtocolError(
                f"symlink or invalid directory cannot be exported: {name}"
            ) from None
        raise
    finally:
        os.close(directory_fd)
    with os.fdopen(descriptor, "rb") as stream:
        info = os.fstat(stream.fileno())
        if not stat.S_ISREG(info.st_mode):
            raise ProtocolError(f"only regular files can be exported: {name}")
        mode = stat.S_IMODE(info.st_mode)
        if mode & 0o7000:
            raise ProtocolError(f"special permission bits are not supported: {name}")
        content = stream.read()
    return {"content": content, "sha256": sha256(content), "mode": mode}


def manifest(files):
    return {
        name: {
            "sha256": row["sha256"],
            "mode": row["mode"],
            "size": len(row["content"]),
        }
        for name, row in sorted(files.items())
    }


def bundle_hash(files):
    return sha256(canonical(manifest(files)))


def git(root, *args):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if result.returncode:
        # Git diagnostics may include credential-bearing remote URLs.
        raise ProtocolError("repository or base commit could not be read")
    return result.stdout


def repository_files(spec):
    root = Path(spec["target"]["repo"])
    base = spec["target"]["base_commit"]
    top = Path(os.fsdecode(git(root, "rev-parse", "--show-toplevel")).strip()).resolve()
    if top != root:
        raise ProtocolError("target.repo must be the repository root")
    if git(root, "rev-parse", base + "^{commit}").decode().strip() != base:
        raise ProtocolError("base_commit must identify an existing commit")
    if git(root, "rev-parse", "HEAD").decode().strip() != base:
        raise ProtocolError(
            "checkout HEAD must match base_commit before including dirty files"
        )
    entries = git(root, "ls-tree", "-rz", "--full-tree", base).split(b"\x00")
    tracked = {}
    for entry in entries:
        if not entry:
            continue
        meta, raw_name = entry.split(b"\t", 1)
        mode, kind, _ = meta.decode().split()
        tracked[os.fsdecode(raw_name)] = (mode, kind)
    selected = set()
    for prefix in spec["source"]["paths"]:
        matches = {name for name in tracked if covered(name, [prefix])}
        if not matches:
            raise ProtocolError(
                f"source path has no files at the base commit: {prefix}"
            )
        selected.update(matches)
    for name in spec["source"]["new_paths"]:
        if name in tracked:
            raise ProtocolError(f"new_path already exists at base commit: {name}")
        selected.add(name)
    result, deleted = {}, []
    for name in sorted(selected):
        export_path(name)
        if not covered(name, spec["authority"]["export_paths"]):
            raise ProtocolError(f"file outside export authority: {name}")
        if name in tracked and tracked[name] not in {
            ("100644", "blob"),
            ("100755", "blob"),
        }:
            raise ProtocolError(f"symlinks and submodules cannot be exported: {name}")
        try:
            result[name] = read_file(root, name)
        except FileNotFoundError:
            if name not in tracked:
                raise ProtocolError(f"missing authorized new file: {name}") from None
            deleted.append(name)
    for name in spec["target"]["allowed_paths"]:
        if name in tracked and name not in selected:
            raise ProtocolError(f"target file is missing from source allowlist: {name}")
    return result, deleted


def validate_target(spec, base):
    target = fields(
        spec["target"],
        "target",
        {
            "kind",
            "repo",
            "repo_id",
            "base_commit",
            "allowed_paths",
            "surfaces",
            "protected_rules",
        },
        {"compatible_bases"},
    )
    choice(target["kind"], {"skill", "agents"}, "target.kind")
    target["repo"] = str(directory(target["repo"], base))
    text(target["repo_id"], "target.repo_id")
    fingerprint(target["base_commit"], "target.base_commit", 40)
    for revision in rows(
        target.get("compatible_bases", []), "compatible_bases", empty=True
    ):
        fingerprint(revision, "compatible base", 40)
        git(
            Path(target["repo"]),
            "merge-base",
            "--is-ancestor",
            revision,
            target["base_commit"],
        )
    allowed = path_list(target["allowed_paths"], "target.allowed_paths")
    surfaces = fields(target["surfaces"], "target.surfaces", {"skill", "agents"})
    for surface in surfaces:
        path_list(surfaces[surface], f"surfaces.{surface}", empty=True)
    all_paths = surfaces["skill"] + surfaces["agents"]
    if set(all_paths) != set(allowed) or len(set(all_paths)) != len(all_paths):
        raise ProtocolError("instruction surfaces must partition allowed_paths")
    if not surfaces[target["kind"]]:
        raise ProtocolError("target kind must have a nonempty instruction surface")
    for rule in rows(target["protected_rules"], "protected_rules", empty=True):
        fields(rule, "protected_rule", {"path", "text", "kind", "verification"})
        if relative_path(rule["path"]) not in allowed:
            raise ProtocolError("protected rule is outside the target")
        text(rule["text"], "protected_rule.text")
        choice(
            rule["kind"],
            {"principle", "authority", "scope", "fact"},
            "protected_rule.kind",
        )
        choice(
            rule["verification"],
            {"exact", "independent"},
            "protected_rule.verification",
        )
    authority = fields(
        spec["authority"],
        "authority",
        {
            "repo",
            "export_paths",
            "write_paths",
            "remote_host",
            "remote_root",
            "auth_path",
            "allow_remote",
            "allow_apply",
            "allow_unverified",
        },
        {"evaluation_root", "allow_typesafe_auxiliary"} if spec["schema_version"] == 2 else (),
    )
    authority["repo"] = str(directory(authority["repo"], base))
    if authority["repo"] != target["repo"]:
        raise ProtocolError("authority.repo does not match target.repo")
    for name in ["export_paths", "write_paths"]:
        path_list(authority[name], "authority." + name, empty=name == "write_paths")
    if any(name not in allowed for name in authority["write_paths"]):
        raise ProtocolError("write authority must enumerate target files exactly")
    for name in ["allow_remote", "allow_apply", "allow_unverified"]:
        boolean(authority[name], "authority." + name)
    if "allow_typesafe_auxiliary" in authority:
        boolean(authority["allow_typesafe_auxiliary"], "authority.allow_typesafe_auxiliary")
    if not authority["allow_remote"]:
        raise ProtocolError("remote export and execution have not been authorized")
    source = fields(spec["source"], "source", {"paths", "new_paths"}, {"fixed_skill_roots"})
    path_list(source["paths"], "source.paths")
    path_list(source["new_paths"], "source.new_paths", empty=True)
    if "fixed_skill_roots" in source:
        path_list(source["fixed_skill_roots"], "source.fixed_skill_roots", empty=True)
    if any(
        not covered(name, authority["export_paths"])
        for name in source["paths"] + source["new_paths"]
    ):
        raise ProtocolError("source allowlist exceeds export authority")


def validate_runtime(spec):
    runtime = fields(
        spec["runtime"],
        "runtime",
        {
            "provider",
            "ssh_target",
            "remote_root",
            "docker_binary",
            "docker_version",
            "codex_architecture",
            "authentication",
            *VERSIONS,
        },
        {"model", "effort", "tools", "sampling", "apparmor_profile", "model_proxy"},
    )
    choice(runtime["provider"], {"ssh"}, "runtime.provider")
    if "model_proxy" in runtime and not re.fullmatch(
        r"https?://[A-Za-z0-9][A-Za-z0-9.-]*:[0-9]{1,5}",
        text(runtime["model_proxy"], "model_proxy"),
    ):
        raise ProtocolError(
            "model_proxy must name an existing HTTP proxy without credentials or paths"
        )
    if "apparmor_profile" in runtime and not re.fullmatch(
        r"[A-Za-z0-9][A-Za-z0-9_.-]{0,127}",
        text(runtime["apparmor_profile"], "apparmor_profile"),
    ):
        raise ProtocolError(
            "apparmor_profile must name an existing profile without options"
        )
    choice(
        runtime["codex_architecture"],
        {"aarch64", "x86_64"},
        "runtime.codex_architecture",
    )
    if not re.fullmatch(
        r"(?:[A-Za-z0-9._-]+@)?[A-Za-z0-9][A-Za-z0-9._-]*",
        text(runtime["ssh_target"], "ssh_target"),
    ):
        raise ProtocolError(
            "ssh_target must be a host or user@host without SSH options"
        )
    for key in ["remote_root", "docker_binary"]:
        remote_path(runtime[key])
    if not re.fullmatch(
        r"[0-9]+\.[0-9]+\.[0-9]+", text(runtime["docker_version"], "docker_version")
    ):
        raise ProtocolError("docker_version must be explicit")
    authority = spec["authority"]
    if (authority["remote_host"], authority["remote_root"]) != (
        runtime["ssh_target"],
        runtime["remote_root"],
    ):
        raise ProtocolError("remote host or output root is outside authority")
    authentication = runtime["authentication"]
    if authentication is not None:
        fields(authentication, "authentication", {"mode", "path"})
        choice(authentication["mode"], {"remote_codex_login"}, "authentication.mode")
        remote_path(authentication["path"])
    auth_path = None if authentication is None else authentication["path"]
    if authority["auth_path"] != auth_path:
        raise ProtocolError("authentication source is outside authority")
    for key, version in VERSIONS.items():
        if runtime[key] != version:
            raise ProtocolError(f"runtime.{key}: expected {version}")
    if spec["mode"] != "smoke" or "model" in runtime:
        if authentication is None:
            raise ProtocolError(
                "model execution requires explicitly authorized remote Codex login"
            )
        for key in ["model", "effort", "tools", "sampling"]:
            if key not in runtime:
                raise ProtocolError(f"runtime.{key}: required for model execution")
        text(runtime["model"], "runtime.model")
        choice(
            runtime["effort"],
            {"minimal", "low", "medium", "high", "xhigh"},
            "runtime.effort",
        )
        tools = rows(runtime["tools"], "runtime.tools")
        if any(tool != "shell" for tool in tools) or len(tools) != len(set(tools)):
            raise ProtocolError("only the explicit shell tool is supported")
        sampling = fields(runtime["sampling"], "runtime.sampling", {"seed"})
        if sampling["seed"] is not None:
            raise ProtocolError(
                "this Codex runtime has no controllable sampling seed; record null"
            )


def validate_measurement(spec):
    budget = fields(
        spec["budget"],
        "budget",
        {
            "max_candidates",
            "max_trials",
            "max_parallel_trials",
            "per_trial_timeout_sec",
            "total_deadline_sec",
            "recovery_window_sec",
            "max_retries",
        },
        {"max_auxiliary_calls"} if spec["schema_version"] == 2 else (),
    )
    for key, value in budget.items():
        number(
            value,
            "budget." + key,
            minimum=0 if key in {"max_retries", "max_auxiliary_calls"} else 1,
            integer=key.startswith("max_"),
        )
    if budget["max_parallel_trials"] > budget["max_trials"]:
        raise ProtocolError("parallel trials exceed total trials")
    if (
        budget["per_trial_timeout_sec"] + budget["recovery_window_sec"]
        > budget["total_deadline_sec"]
    ):
        raise ProtocolError("trial timeout and recovery window exceed total deadline")
    measurement = fields(
        spec["measurement"],
        "measurement",
        {
            "objective",
            "primary_metric",
            "minimum_effect",
            "confidence_level",
            "minimum_groups",
            "repeats",
            "seed",
            "checkpoint",
            "confirmation",
            "weights",
        },
        {"quality_loss_limit", "minimum_cost_effect"}
        | ({"minimum_effect_reason", "search_policy"} if spec["schema_version"] == 2 else set()),
    )
    choice(
        measurement["objective"],
        {"quality", "cost_at_quality"},
        "measurement.objective",
    )
    choice(
        measurement["primary_metric"],
        {"task_quality"} if spec["schema_version"] == 2 else {"task_success"},
        "measurement.primary_metric",
    )
    if spec["schema_version"] == 2:
        if measurement["objective"] != "quality":
            raise ProtocolError("v2 supports the frozen quality objective only")
        text(measurement.get("minimum_effect_reason"), "minimum_effect_reason")
        if spec["runtime"]["model"] != "gpt-6.1-sol" or spec["runtime"]["effort"] != "high":
            raise ProtocolError("v2 quality experiments require gpt-6.1-sol/high")
        if "search_policy" in measurement:
            text(measurement["search_policy"], "measurement.search_policy")
            choice(measurement["search_policy"], {"strict", "bounded_exploration_v1"}, "measurement.search_policy")
        if measurement.get("search_policy") == "bounded_exploration_v1":
            if spec["mode"] != "search" or budget["max_candidates"] != 1:
                raise ProtocolError("bounded exploration requires search with exactly one candidate limit")
            if spec["authority"]["allow_apply"] is not False or spec["authority"]["write_paths"]:
                raise ProtocolError("bounded exploration cannot have apply or write authority")
    number(measurement["minimum_effect"], "minimum_effect", maximum=1)
    number(
        measurement["confidence_level"], "confidence_level", minimum=0.5, maximum=0.999
    )
    number(measurement["minimum_groups"], "minimum_groups", minimum=2, integer=True)
    number(measurement["repeats"], "repeats", minimum=1, integer=True)
    number(measurement["seed"], "seed", integer=True)
    choice(measurement["checkpoint"], {"fixed"}, "measurement.checkpoint")
    choice(measurement["confirmation"], {"once"}, "measurement.confirmation")
    if measurement["objective"] == "cost_at_quality":
        for key in ["quality_loss_limit", "minimum_cost_effect"]:
            if key not in measurement:
                raise ProtocolError(f"measurement.{key}: required for cost objective")
            number(
                measurement[key],
                key,
                maximum=1 if key == "quality_loss_limit" else None,
            )
    else:
        for key in ["quality_loss_limit", "minimum_cost_effect"]:
            if key in measurement:
                number(
                    measurement[key],
                    key,
                    maximum=1 if key == "quality_loss_limit" else None,
                )
    for guardrail in rows(spec["guardrails"], "guardrails"):
        fields(guardrail, "guardrail", {"name", "minimum"})
        text(guardrail["name"], "guardrail.name")
        number(guardrail["minimum"], "guardrail.minimum", maximum=1)
    names = [item["name"] for item in spec["guardrails"]]
    if len(names) != len(set(names)):
        raise ProtocolError("duplicate guardrail name")


def verify_public_source(case, root, views):
    if case["source"]["kind"] != "pro_v2":
        return
    top = Path(os.fsdecode(git(root, "rev-parse", "--show-toplevel")).strip()).resolve()
    if git(top, "rev-parse", "HEAD").decode().strip() != PRO_V2_REVISION:
        raise ProtocolError("public task checkout is not at the fixed source revision")
    prefix = root.relative_to(top).as_posix()
    if len(PurePosixPath(prefix).parts) != 3 or not prefix.startswith("v2/tasks/"):
        raise ProtocolError(
            "public task must come from the fixed source task directory"
        )
    files = {name: row for view in views.values() for name, row in view.items()}
    tracked = {}
    for entry in git(
        top, "ls-tree", "-rz", "--full-tree", PRO_V2_REVISION, "--", prefix
    ).split(b"\x00"):
        if not entry:
            continue
        metadata, name = entry.split(b"\t", 1)
        mode, kind, digest = metadata.decode().split()
        relative = PurePosixPath(os.fsdecode(name)).relative_to(prefix).as_posix()
        tracked[relative] = (mode, kind, digest)
    if set(files) != set(tracked):
        raise ProtocolError("public task views must account for every frozen task file")
    for name, row in files.items():
        mode, kind, digest = tracked[name]
        blob_id = hashlib.sha1(
            b"blob " + str(len(row["content"])).encode() + b"\x00" + row["content"]
        ).hexdigest()
        expected_mode = "100755" if row["mode"] & 0o111 else "100644"
        if kind != "blob" or mode != expected_mode or blob_id != digest:
            raise ProtocolError(
                "public task file differs from the fixed source revision"
            )


def task_files(spec, base):
    case_files, ids, split_groups, split_families, split_tasks = {}, set(), {}, {}, {}
    family_groups, task_groups = {}, {}
    for case in rows(spec["cases"], "cases"):
        fields(
            case,
            "case",
            {
                "case_id",
                "group_id",
                "repo_id",
                "base_commit",
                "split",
                "task_path",
                "source",
                "public_paths",
                "grader_paths",
                "oracle_paths",
                "image",
                "observable_requirements",
                "information_cutoff",
            },
            {"grader_ref", "platform", "repo_path"},
        )
        for key in ["case_id", "group_id", "repo_id"]:
            text(case[key], "case." + key)
        if case["case_id"] in ids:
            raise ProtocolError("duplicate case_id")
        ids.add(case["case_id"])
        fingerprint(case["base_commit"], "case.base_commit", 40)
        choice(case["split"], {"search", "validation", "confirmation"}, "case.split")
        source = fields(case["source"], "case.source", {"kind", "revision", "family"})
        choice(source["kind"], {"repo", "pro_v2"}, "case.source.kind")
        text(source["family"], "case.source.family")
        fingerprint(source["revision"], "case.source.revision", 40)
        expected_revision = (
            PRO_V2_REVISION if source["kind"] == "pro_v2" else case["base_commit"]
        )
        if source["revision"] != expected_revision:
            raise ProtocolError("task source revision does not match the frozen source")
        target = spec["target"]
        if target["kind"] == "agents" and (
            case["repo_id"] != target["repo_id"]
            or case["base_commit"]
            not in [target["base_commit"], *target.get("compatible_bases", [])]
        ):
            raise ProtocolError(
                "AGENTS task must match the target repository and an explicitly compatible base"
            )
        if not re.fullmatch(
            r"[^\s]+@sha256:[0-9a-f]{64}", text(case["image"], "case.image")
        ):
            raise ProtocolError("task image must be pinned by digest")
        case.setdefault("platform", "linux/amd64")
        platforms = {"linux/amd64"}
        if spec["schema_version"] == 2 and source["kind"] == "repo":
            platforms.add("linux/arm64")
        choice(case["platform"], platforms, "case.platform")
        for item in rows(case["observable_requirements"], "observable_requirements"):
            text(item, "observable requirement")
        try:
            cutoff = datetime.fromisoformat(
                text(case["information_cutoff"], "information_cutoff").replace(
                    "Z", "+00:00"
                )
            )
        except ValueError:
            raise ProtocolError("information_cutoff must be an ISO timestamp") from None
        if cutoff.tzinfo is None:
            raise ProtocolError("information_cutoff must include timezone")
        root = directory(case["task_path"], base)
        case["task_path"] = str(root)
        views = {}
        for key in ["public_paths", "grader_paths", "oracle_paths"]:
            names = path_list(case[key], "case." + key)
            views[key] = {name: read_file(root, name) for name in names}
        if "instruction.md" not in views["public_paths"]:
            raise ProtocolError("public task view must contain instruction.md")
        if "repo_path" not in case:
            match = re.search(
                r"A code repository is available in the `([^`]+)` directory",
                views["public_paths"]["instruction.md"]["content"].decode("utf-8"),
            )
            if match is None:
                raise ProtocolError(
                    "case.repo_path is required when the public task does not declare it"
                )
            case["repo_path"] = match[1]
        remote_path(case["repo_path"])
        if any(
            covered(case["repo_path"], [name])
            for name in [
                "/evolution-control",
                "/evolution-input",
                "/home/evolution",
                "/tests",
                "/solution",
            ]
        ):
            raise ProtocolError(
                "task repository overlaps the runtime control namespace"
            )
        public = set(views["public_paths"])
        private = set(views["grader_paths"]) | set(views["oracle_paths"])
        if public & private or any(
            PurePosixPath(name).parts[0] in {"tests", "solution", "answers"}
            for name in public
        ):
            raise ProtocolError("public task view exposes grader or oracle")
        public_inodes = {(root / name).stat().st_ino for name in public}
        private_inodes = {(root / name).stat().st_ino for name in private}
        if public_inodes & private_inodes:
            raise ProtocolError("public task view aliases private material")
        verify_public_source(case, root, views)
        grader_ref = bundle_hash(views["grader_paths"])
        if "grader_ref" in case and case["grader_ref"] != grader_ref:
            raise ProtocolError("grader_ref does not match the actual grading files")
        case["grader_ref"] = grader_ref
        identity = bundle_hash(views["public_paths"])
        for mapping, key in [
            (family_groups, (case["repo_id"], source["family"])),
            (task_groups, (case["repo_id"], identity)),
        ]:
            if mapping.setdefault(key, case["group_id"]) != case["group_id"]:
                raise ProtocolError(
                    "related task sources cannot count as independent groups"
                )
        for mapping, key in [
            (split_groups, (case["repo_id"], case["group_id"])),
            (split_families, (case["repo_id"], source["family"])),
            (split_tasks, (case["repo_id"], identity)),
        ]:
            if key in mapping and mapping[key] != case["split"]:
                raise ProtocolError(
                    "related tasks cannot cross search/validation/confirmation splits"
                )
            mapping[key] = case["split"]
        case_files[case["case_id"]] = views
    splits = {case["split"] for case in spec["cases"]}
    bounded_exploration = (spec["schema_version"] == 2 and spec["mode"] == "search"
        and spec["measurement"].get("search_policy") == "bounded_exploration_v1")
    if "search" not in splits or (
        spec["mode"] not in {"smoke", "calibrate"} and not bounded_exploration and "confirmation" not in splits
    ):
        raise ProtocolError(
            "mode requires search and, for comparisons, heldout confirmation tasks"
        )
    if spec["mode"] == "calibrate" and splits != {"search"}:
        raise ProtocolError("calibrate does not expose confirmation or validation tasks")
    weights = spec["measurement"]["weights"]
    if not isinstance(weights, dict) or set(weights) != ids:
        raise ProtocolError("measurement.weights must name every case exactly once")
    for weight in weights.values():
        number(weight, "case weight", minimum=0.000001)
    return case_files


def skill_name(content):
    header = re.match(rb"\A---\r?\n(.*?)\r?\n---(?:\r?\n|\Z)", content, re.DOTALL)
    if header is None:
        return None
    name = re.search(
        rb"(?m)^name:[ \t]*['\"]?([a-z0-9][a-z0-9-]*)['\"]?[ \t]*(?:#.*)?$", header[1]
    )
    return name[1].decode() if name else None


def condition_bundles(spec, baseline):
    target = spec["target"]
    surfaces = target["surfaces"]
    skill_roots = {
        str(PurePosixPath(path).parent)
        for path in surfaces["skill"]
        if PurePosixPath(path).name == "SKILL.md"
    }
    for root in skill_roots:
        if any(
            covered(path, [root]) and path not in surfaces["skill"] for path in baseline
        ):
            raise ProtocolError(
                "skill surface omits files belonging to the target skill"
            )
    target_skills = [
        baseline[path]["content"]
        for path in surfaces["skill"]
        if path in baseline and PurePosixPath(path).name == "SKILL.md"
    ]
    target_names = {skill_name(content) for content in target_skills}
    if surfaces["skill"] and (not target_skills or None in target_names):
        raise ProtocolError(
            "target skill requires SKILL.md with a canonical name field"
        )
    for path, row in baseline.items():
        if (
            path not in surfaces["skill"]
            and PurePosixPath(path).name == "SKILL.md"
            and (
                row["content"] in target_skills
                or skill_name(row["content"]) in target_names
            )
        ):
            raise ProtocolError(
                "target skill has another readable copy outside its surface"
            )
    bundles, levels, differences = {}, {"skill": {}, "agents": {}}, {}
    for condition in rows(spec["conditions"], "conditions"):
        fields(condition, "condition", {"condition_id", "skill", "agents", "overrides"})
        name = text(condition["condition_id"], "condition_id")
        if name in bundles:
            raise ProtocolError("duplicate condition_id")
        choice(
            condition["skill"],
            {"absent", "baseline", "candidate", "fixed"},
            "condition.skill",
        )
        choice(
            condition["agents"],
            {"minimal", "baseline", "candidate", "fixed"},
            "condition.agents",
        )
        if not isinstance(condition["overrides"], dict):
            raise ProtocolError("condition.overrides must be a path-to-content object")
        files = deepcopy(baseline)
        changed = []
        for path, content in condition["overrides"].items():
            export_path(path)
            if path not in target["allowed_paths"]:
                raise ProtocolError(f"condition changes an undeclared surface: {path}")
            if content is None:
                files.pop(path, None)
            elif isinstance(content, str):
                raw = content.encode("utf-8")
                files[path] = {
                    "content": raw,
                    "sha256": sha256(raw),
                    "mode": baseline.get(path, {}).get("mode", 0o644),
                }
            else:
                raise ProtocolError("condition content must be text or null")
            if baseline.get(path) != files.get(path):
                changed.append(path)
        for rule in target["protected_rules"]:
            if condition["skill"] == "absent" and rule["path"] in surfaces["skill"]:
                continue
            if rule["verification"] == "exact" and rule[
                "text"
            ].encode() not in files.get(rule["path"], {}).get("content", b""):
                raise ProtocolError(
                    f"condition removes a protected rule from {rule['path']}"
                )
        for surface, paths in surfaces.items():
            level = condition[surface]
            content_hash = bundle_hash(
                {path: files[path] for path in paths if path in files}
            )
            prior = levels[surface]
            if level in prior and prior[level] != content_hash:
                raise ProtocolError(f"same {surface} level has different bytes")
            if level not in prior and content_hash in prior.values():
                raise ProtocolError(f"different {surface} levels have identical bytes")
            prior[level] = content_hash
            if not paths and level != "fixed":
                raise ProtocolError(f"{surface} factor is declared without a surface")
            if level == "absent" and any(path in files for path in paths):
                raise ProtocolError(
                    "absent skill condition still contains target skill files"
                )
            if level in {"baseline", "fixed"} and any(
                files.get(path) != baseline.get(path) for path in paths
            ):
                raise ProtocolError(
                    f"{surface} baseline/fixed condition changes actual bytes"
                )
        bundles[name] = files
        differences[name] = sorted(changed)
    initial = text(spec["baseline_condition"], "baseline_condition")
    if initial not in bundles or bundles[initial] != baseline:
        raise ProtocolError(
            "baseline_condition must reproduce the actual initial bytes"
        )
    if spec["mode"] not in {"smoke", "calibrate"}:
        selected = spec.get("candidate_condition")
        if selected is not None:
            text(selected, "candidate_condition")
        if spec["mode"] == "compare" and (
            selected not in bundles or selected == initial
        ):
            raise ProtocolError(
                "compare requires a distinct frozen candidate_condition"
            )
        if target["kind"] == "skill" and "absent" not in levels["skill"]:
            raise ProtocolError("skill comparison requires a no-target-skill control")
    return bundles, differences


def candidate_from_patch(job, parent, proposal, patch, *, search_records=None):
    fields(
        proposal,
        "proposal",
        {
            "parent_hash",
            "hypothesis",
            "expected_effect",
            "counterexample",
            "changed_paths",
        } | ({"mechanism"} if job.spec["schema_version"] == 2 else set()),
    )
    if proposal["parent_hash"] != bundle_hash(parent):
        raise ProtocolError("proposal does not name the frozen parent")
    for key in ["hypothesis", "expected_effect", "counterexample"]:
        text(proposal[key], "proposal." + key)
    if job.spec["schema_version"] == 2:
        from quality import continuous_item

        mechanism = fields(proposal["mechanism"], "one mechanism", {
            "criterion_id", "patch_sha256", "cause", "expected_criteria", "regression_risk"})
        for key in ("criterion_id", "cause", "regression_risk"):
            text(mechanism[key], "mechanism." + key)
        fingerprint(mechanism["patch_sha256"], "mechanism artifact")
        rubrics = {c["case_id"]: parse_json(job.evaluation_files[job.spec["evaluation"]["rubrics"][c["case_id"]]]["content"])
                   for c in job.spec["cases"] if c["split"] == "search"}
        known = {c["criterion_id"] for r in rubrics.values() for c in r["criteria"]}
        expected = rows(mechanism["expected_criteria"], "expected quality criteria")
        if any(not isinstance(name, str) or name not in known for name in expected) or len(set(expected)) != len(expected) or mechanism["criterion_id"] not in expected:
            raise ProtocolError("mechanism must target the observed public quality criterion")
        supported = False
        for record in search_records or []:
            if not (record.get("schema_version") == 2 and record.get("purpose") == "task"
                    and record.get("split") == "search" and record.get("status") in {"ok", "behavior_failure"}
                    and record.get("candidate_hash") == proposal["parent_hash"]
                    and record.get("patch_sha256") == mechanism["patch_sha256"]
                    and type(record.get("task_success")) is int and record["task_success"] in (0, 1)
                    and record.get("quality_status") == "valid"):
                continue
            criterion = next((c for c in rubrics.get(record.get("case_id"), {}).get("criteria", [])
                              if c["criterion_id"] == mechanism["criterion_id"]), None)
            component = record.get("quality_components", {}).get(mechanism["criterion_id"], {})
            if (criterion is not None
                    and (not criterion["gate"] or mechanism["criterion_id"] in record.get("failed_gates", []))
                    and component.get("status") == "valid" and type(component.get("level")) in (int, float)
                    and (0 <= component["level"] < 1 if continuous_item(rubrics[record["case_id"]], criterion)
                         else component["level"] in (0, 0.5))):
                supported = True
                break
        if not supported:
            raise ProtocolError("mechanism lacks a measured current-parent search weakness")
    declared = path_list(proposal["changed_paths"], "proposal.changed_paths")
    allowed = job.spec["target"]["allowed_paths"]
    if any(name not in allowed for name in declared):
        raise ProtocolError("proposal changes paths outside the target surface")
    with tempfile.TemporaryDirectory(prefix="instruction-candidate-") as temporary:
        root = Path(temporary)
        for name, row in parent.items():
            destination = root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(row["content"])
            destination.chmod(row["mode"])
        applied = subprocess.run(
            [
                "git",
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
                "apply",
                "--binary",
                "-",
            ],
            cwd=root,
            input=patch,
            capture_output=True,
        )
        if applied.returncode:
            raise ProtocolError(
                "candidate patch cannot be applied safely to its parent"
            )
        candidate = {}
        for path in sorted(root.rglob("*")):
            if path.is_dir() and not path.is_symlink():
                continue
            name = path.relative_to(root).as_posix()
            candidate[name] = read_file(root, name)
    changed = sorted(
        name
        for name in parent.keys() | candidate.keys()
        if parent.get(name) != candidate.get(name)
    )
    if changed != sorted(declared):
        raise ProtocolError("proposal changed_paths differs from the actual patch")
    for rule in job.spec["target"]["protected_rules"]:
        if rule["verification"] == "exact" and rule[
            "text"
        ].encode() not in candidate.get(rule["path"], {}).get("content", b""):
            raise ProtocolError("candidate removes a protected text block")
    references = set()
    for views in job.case_files.values():
        for row in views["oracle_paths"].values():
            content = row["content"].strip()
            if len(content) >= 16:
                references.add(content)
            references.update(
                line[1:].strip()
                for line in content.splitlines()
                if line.startswith(b"+")
                and not line.startswith(b"+++")
                and len(line.strip()) >= 33
            )
    for name in changed:
        raw = candidate.get(name, {}).get("content", b"")
        original = parent.get(name, {}).get("content", b"")
        if any(
            reference in raw and reference not in original for reference in references
        ):
            raise ProtocolError("candidate contains known private reference content")
        for case in job.spec["cases"]:
            if len(case["case_id"]) >= 8 and case["case_id"].encode() in raw:
                raise ProtocolError("candidate contains restricted task identifiers")
        if Path(name).name == "SKILL.md" and name in parent:
            if skill_name(raw) != skill_name(parent[name]["content"]):
                raise ProtocolError("candidate changes the skill invocation name")
    return candidate


@dataclass
class PreparedJob:
    spec: dict
    bundles: dict
    case_files: dict
    method_files: dict
    deleted_paths: list
    differences: dict
    minimum_calls: int
    confirmation_reserve: int
    evaluation_files: dict = field(default_factory=dict)
    worker_files: dict = field(default_factory=dict)
    call_plan: dict = field(default_factory=dict)

    @property
    def bundle_hashes(self):
        return {name: bundle_hash(files) for name, files in self.bundles.items()}

    def inputs(self):
        result = {
            "bundles": {name: manifest(files) for name, files in self.bundles.items()},
            "cases": {
                name: {view: manifest(files) for view, files in views.items()}
                for name, views in self.case_files.items()
            },
            "method": manifest(self.method_files),
            "deleted_paths": self.deleted_paths,
        }
        if self.spec["schema_version"] == 2:
            result.update(evaluation=manifest(self.evaluation_files), workers=manifest(self.worker_files))
        return result

    @property
    def protocol_hash(self):
        return sha256(canonical({"spec": self.spec, "inputs": self.inputs()}))

    def summary(self):
        return {
            "schema_version": self.spec["schema_version"],
            "protocol_id": self.spec["protocol_id"],
            "protocol_hash": self.protocol_hash,
            "mode": self.spec["mode"],
            "configuration": deepcopy(self.spec),
            "export_files": sorted(self.bundles[self.spec["baseline_condition"]]),
            "condition_differences": self.differences,
            "bundle_hashes": self.bundle_hashes,
            "minimum_calls": self.minimum_calls,
            "confirmation_reserve": self.confirmation_reserve,
            "call_upper_bound": min(
                self.spec["budget"]["max_trials"],
                self.minimum_calls * (1 + self.spec["budget"]["max_retries"]),
            )
            if self.spec["mode"] == "smoke"
            else self.spec["budget"]["max_trials"],
            "effect_eligible": self.spec["mode"] not in {"smoke", "calibrate"},
            **({"call_plan": self.call_plan} if self.spec["schema_version"] == 2 else {}),
        }

    def optimizer_view(self):
        cases = []
        for case in self.spec["cases"]:
            if case["split"] != "search":
                continue
            cases.append(
                {
                    "case_id": case["case_id"],
                    "group_id": case["group_id"],
                    "repo_id": case["repo_id"],
                    "observable_requirements": case["observable_requirements"],
                    "files": {
                        name: row["content"].decode("utf-8")
                        for name, row in self.case_files[case["case_id"]][
                            "public_paths"
                        ].items()
                    },
                }
            )
        return {"schema_version": 1, "cases": cases}


def evaluation_files(spec, base):
    from quality import CONTINUOUS_SCORING, validate_calibration, validate_rubric, validate_auxiliary_rubric

    evaluation = fields(spec.get("evaluation"), "evaluation", {
        "root", "paths", "rubrics", "calibration",
    }, {"auxiliary", "scoring"})
    if "scoring" in evaluation:
        choice(evaluation["scoring"], {CONTINUOUS_SCORING}, "evaluation.scoring")
    root = directory(evaluation["root"], base)
    authority_root = directory(spec["authority"].get("evaluation_root"), base)
    if root != authority_root:
        raise ProtocolError("evaluation root is outside explicit authority")
    evaluation["root"] = spec["authority"]["evaluation_root"] = str(root)
    files = {name: read_file(root, name) for name in path_list(evaluation["paths"], "evaluation.paths")}

    def document(name):
        relative_path(name)
        if name not in files:
            raise ProtocolError("evaluation file is outside its allowlist")
        return parse_json(files[name]["content"])

    ids = {case["case_id"] for case in spec["cases"]}
    if not isinstance(evaluation["rubrics"], dict) or evaluation["rubrics"].keys() != ids:
        raise ProtocolError("evaluation.rubrics must name every case exactly once")
    rubrics = {case_id: validate_rubric(document(name), case_id=case_id)
               for case_id, name in evaluation["rubrics"].items()}
    scoring = {r.get("scoring") for r in rubrics.values()}
    if len(scoring) != 1 or ("scoring" in evaluation and evaluation["scoring"] not in scoring):
        raise ProtocolError("quality rubrics use different scoring contracts")
    status_formats = {r.get("derive_assessment_status", False) for r in rubrics.values()
                      if any(c["checker_kind"] == "judge" for c in r["criteria"])}
    if len(status_formats) > 1:
        raise ProtocolError("quality rubrics use different assessment status formats")
    if next(iter(scoring)) is not None:
        evaluation["scoring"] = next(iter(scoring))
    for case in spec["cases"]:
        public = {name: read_file(Path(case["task_path"]), name)["content"].decode("utf-8")
                  for name in case["public_paths"]}
        if evaluation.get("scoring") == CONTINUOUS_SCORING:
            if "task-contract.md" in public:
                raise ProtocolError("task-contract.md is reserved for declared observable requirements")
            public["task-contract.md"] = task_contract(case).decode("utf-8")
        for criterion in rubrics[case["case_id"]]["criteria"]:
            name, separator, quote = criterion["public_requirement_ref"].partition(": ")
            if not separator or name not in public or not quote or quote not in public[name]:
                raise ProtocolError("quality criterion must reference an actual public requirement")
    calibration = validate_calibration(document(evaluation["calibration"]), files, rubrics)
    if any(next(c for c in spec["cases"] if c["case_id"] == a["case_id"])["split"] != "search"
           for a in calibration["artifacts"]):
        raise ProtocolError("calibration cannot consume heldout tasks")
    workers = {name: read_file(Path(__file__).parent, name) for name in ("quality.py", "remote.py")}
    if evaluation.get("scoring") == CONTINUOUS_SCORING:
        workers.update({name: read_file(Path(__file__).parent, name) for name in ("protocol.py", "evolution.py", "compare.py")})
    auxiliary = evaluation.get("auxiliary")
    if auxiliary is not None:
        fields(auxiliary, "evaluation.auxiliary", {
            "provider", "model", "criterion_ids", "rubric_path", "calibration_path", "authentication",
        }, {"long_context"})
        if "long_context" in auxiliary:
            boolean(auxiliary["long_context"], "auxiliary.long_context")
        choice(auxiliary["provider"], {"typesafe"}, "auxiliary provider")
        choice(auxiliary["model"], {"jev-1.14.0"}, "auxiliary model")
        authentication = fields(auxiliary["authentication"], "auxiliary authentication", {"source", "name"})
        if authentication != {"source": "controller_env", "name": "TYPESAFE_API_KEY"}:
            raise ProtocolError("auxiliary credential must come from TYPESAFE_API_KEY in the controller environment")
        if spec["authority"].get("allow_typesafe_auxiliary") is not True:
            raise ProtocolError("TypeSafe evidence export is not explicitly authorized")
        if "max_auxiliary_calls" not in spec["budget"]:
            raise ProtocolError("auxiliary scoring requires a finite sub-budget")
        if spec["budget"]["max_auxiliary_calls"] > spec["budget"]["max_trials"]:
            raise ProtocolError("auxiliary sub-budget exceeds the total call budget")
        validate_auxiliary_rubric(document(auxiliary["rubric_path"]), rubrics, auxiliary["criterion_ids"])
        auxiliary_calibration = validate_calibration(document(auxiliary["calibration_path"]), files, rubrics,
                                                     auxiliary=True, criterion_ids=auxiliary["criterion_ids"])
        artifacts = {a["artifact_id"]: a for a in calibration["artifacts"]}
        for artifact in auxiliary_calibration["artifacts"]:
            if next(c for c in spec["cases"] if c["case_id"] == artifact["case_id"])["split"] != "search":
                raise ProtocolError("auxiliary calibration cannot consume heldout tasks")
            prior = artifacts.get(artifact["artifact_id"])
            if prior is not None and prior != artifact:
                raise ProtocolError("calibration artifact id aliases different frozen inputs")
            if any(artifact["expected_levels"][name] not in (0, 0.5, 1)
                   for name in auxiliary["criterion_ids"] if name in artifact["expected_levels"]):
                raise ProtocolError("auxiliary calibration requires its own three semantic reference levels")
        workers["typesafe_worker.py"] = read_file(Path(__file__).parent, "typesafe_worker.py")
    elif spec["budget"].get("max_auxiliary_calls", 0):
        raise ProtocolError("auxiliary budget requires auxiliary configuration")
    return files, workers, rubrics, calibration


def quality_case_calls(criteria):
    return 2 + sum(c["checker_kind"] == "regression_replay" for c in criteria) + 2 * any(
        c["checker_kind"] == "judge" for c in criteria)


def quality_call_plan(spec, files, rubrics, calibration, bundles):
    def per_case(case):
        criteria = rubrics[case["case_id"]]["criteria"]
        return quality_case_calls(criteria)

    repeats = spec["measurement"]["repeats"]
    search = sum(per_case(c) for c in spec["cases"] if c["split"] == "search") * repeats
    validation = sum(per_case(c) for c in spec["cases"] if c["split"] == "validation") * repeats
    confirmation = 2 * sum(per_case(c) for c in spec["cases"] if c["split"] == "confirmation") * repeats
    calibration_calls = 1 + 2 * len(spec["cases"])
    for artifact in calibration["artifacts"]:
        calibration_calls += per_case({"case_id": artifact["case_id"]}) - 1
    auxiliary = spec["evaluation"].get("auxiliary")
    if auxiliary is not None:
        primary_ids = {a["artifact_id"] for a in calibration["artifacts"]}
        auxiliary_calibration = parse_json(files[auxiliary["calibration_path"]]["content"])
        for artifact in auxiliary_calibration["artifacts"]:
            if artifact["artifact_id"] not in primary_ids:
                criteria = rubrics[artifact["case_id"]]["criteria"]
                calibration_calls += 1 + sum(c["checker_kind"] == "regression_replay" for c in criteria)
    plan = {"runtime_and_quality_calibration": calibration_calls, "baseline_aa": 2 * search,
            "initial_conditions": 0 if spec["mode"] == "calibrate" else len(bundles) * (search + validation),
            "optimizer_and_policy": 2 * spec["budget"]["max_candidates"] if spec["mode"] == "search" else 1 if spec["mode"] == "compare" else 0,
            "candidate_pairs": 2 * (search + validation) * spec["budget"]["max_candidates"] if spec["mode"] == "search" else 0,
            "confirmation_reserve": confirmation,
            "auxiliary_maximum": spec["budget"].get("max_auxiliary_calls", 0)}
    planned = sum(plan.values())
    optional_rounds = max(0, spec["budget"]["max_candidates"] - 1) if spec["mode"] == "search" else 0
    plan["minimum_calls"] = planned - optional_rounds * (2 + 2 * (search + validation))
    plan["finite_retry_allowance"] = (planned - plan["auxiliary_maximum"]) * spec["budget"]["max_retries"]
    plan["maximum_planned_calls"] = planned + plan["finite_retry_allowance"]
    return plan


def prepare(path):
    path = Path(path).resolve(strict=True)
    spec = load_json(path)
    fields(
        spec,
        "JobSpec",
        {
            "schema_version",
            "protocol_id",
            "question",
            "mode",
            "target",
            "source",
            "runtime",
            "authority",
            "budget",
            "measurement",
            "guardrails",
            "cases",
            "conditions",
            "baseline_condition",
        },
        {"optimizer", "candidate_condition", "attribution", "evaluation"},
    )
    if type(spec["schema_version"]) is not int or spec["schema_version"] not in {1, 2}:
        raise ProtocolError("unsupported schema_version")
    text(spec["protocol_id"], "protocol_id")
    text(spec["question"], "question")
    choice(spec["mode"], {"calibrate", "compare", "search"} if spec["schema_version"] == 2 else {"smoke", "compare", "search"}, "mode")
    if spec["schema_version"] == 1 and "evaluation" in spec:
        raise ProtocolError("quality evaluation requires schema_version=2")
    validate_target(spec, path.parent)
    validate_runtime(spec)
    validate_measurement(spec)
    baseline, deleted = repository_files(spec)
    cases = task_files(spec, path.parent)
    evaluation, workers, rubrics, calibration = {}, {}, {}, None
    if spec["schema_version"] == 2:
        evaluation, workers, rubrics, calibration = evaluation_files(spec, path.parent)
    private_files = set()
    for case in spec["cases"]:
        root = Path(case["task_path"])
        for name in case["grader_paths"] + case["oracle_paths"]:
            info = (root / name).stat()
            private_files.add((info.st_dev, info.st_ino))
    for name in baseline:
        info = (Path(spec["target"]["repo"]) / name).stat()
        if (info.st_dev, info.st_ino) in private_files:
            raise ProtocolError(
                "repository export contains private grading or oracle material"
            )
    fixed_roots = spec["source"].get("fixed_skill_roots", [])
    target_roots = {str(PurePosixPath(path).parent) for path in spec["target"]["surfaces"]["skill"]
                    if PurePosixPath(path).name == "SKILL.md"}
    skill_names = {skill_name(baseline[path]["content"]) for path in spec["target"]["surfaces"]["skill"]
                   if PurePosixPath(path).name == "SKILL.md" and path in baseline}
    for root in fixed_roots:
        descriptor = root + "/SKILL.md"
        if descriptor not in baseline:
            raise ProtocolError("fixed skill descriptor must be in the source export")
        if any(covered(root, [target]) or covered(target, [root]) for target in target_roots) or any(
            covered(path, [root]) for path in spec["target"]["allowed_paths"]
        ):
            raise ProtocolError("fixed skill cannot overlap a candidate surface")
        name = skill_name(baseline[descriptor]["content"])
        if name is None or name in skill_names:
            raise ProtocolError("fixed skill identity is missing or duplicated")
        skill_names.add(name)
    bundles, differences = condition_bundles(spec, baseline)
    method = {}
    if spec["mode"] == "search":
        if "optimizer" not in spec:
            raise ProtocolError("search requires an optimizer")
        optimizer = fields(
            spec["optimizer"],
            "optimizer",
            {"model", "effort", "method_path", "method_sha256"},
        )
        text(optimizer["model"], "optimizer.model")
        choice(
            optimizer["effort"],
            {"minimal", "low", "medium", "high", "xhigh"},
            "optimizer.effort",
        )
        fingerprint(optimizer["method_sha256"], "optimizer.method_sha256")
        method_path = Path(optimizer["method_path"])
        if not method_path.is_absolute():
            method_path = path.parent / method_path
        method = {
            "method.md": read_file(
                method_path.parent.resolve(strict=True), method_path.name
            )
        }
        optimizer["method_path"] = str(method_path.absolute())
        if method["method.md"]["sha256"] != optimizer["method_sha256"]:
            raise ProtocolError("optimizer method hash mismatch")
        if spec["schema_version"] == 2 and (optimizer["model"], optimizer["effort"]) != ("gpt-6.1-sol", "high"):
            raise ProtocolError("v2 optimizer requires gpt-6.1-sol/high")
    elif "optimizer" in spec:
        raise ProtocolError("optimizer is only valid in search mode")
    if "attribution" in spec:
        choice(
            spec["attribution"],
            {"skill", "agents", "joint", "interaction"},
            "attribution",
        )
        if spec["attribution"] == "interaction":
            combinations = {(c["skill"], c["agents"]) for c in spec["conditions"]}
            required = {
                (s, a)
                for s in ["baseline", "candidate"]
                for a in ["baseline", "candidate"]
            }
            if not required.issubset(combinations):
                raise ProtocolError(
                    "interaction attribution requires a frozen crossed design"
                )
        elif spec["attribution"] in {"skill", "agents"}:
            background = "agents" if spec["attribution"] == "skill" else "skill"
            if len({c[background] for c in spec["conditions"]}) != 1:
                raise ProtocolError(
                    "single-factor attribution changes the other factor"
                )
    repeats = spec["measurement"]["repeats"]
    search_count = sum(case["split"] == "search" for case in spec["cases"])
    validation_count = sum(case["split"] == "validation" for case in spec["cases"])
    confirmation_count = sum(case["split"] == "confirmation" for case in spec["cases"])
    reserve = 0 if spec["mode"] == "smoke" else 4 * confirmation_count * repeats
    minimum = 1 + 2 * len(cases)
    if spec["mode"] != "smoke":
        minimum += (
            4 * search_count * repeats
            + 2 * len(bundles) * (search_count + validation_count) * repeats
            + reserve
        )
        if spec["mode"] == "search":
            minimum += 2 + 4 * (search_count + validation_count) * repeats
        else:
            minimum += 1
    call_plan = {}
    if spec["schema_version"] == 2:
        call_plan = quality_call_plan(spec, evaluation, rubrics, calibration, bundles)
        minimum, reserve = call_plan["minimum_calls"], call_plan["confirmation_reserve"]
    if minimum > spec["budget"]["max_trials"]:
        raise ProtocolError(
            f"budget cannot fund calibration, baseline and final confirmation: requires {minimum}"
        )
    return PreparedJob(
        spec, bundles, cases, method, deleted, differences, minimum, reserve,
        evaluation, workers, call_plan,
    )


def new_execution(
    job, role, idempotency_key, command_hash, runner_hash, *, now, run_deadline
):
    choice(role, ROLES, "role")
    budget = job.spec["budget"]
    command_deadline = min(
        now + budget["per_trial_timeout_sec"],
        run_deadline - budget["recovery_window_sec"],
    )
    if command_deadline <= now:
        raise ProtocolError(
            "insufficient time for command and reserved recovery window"
        )
    task_id = str(uuid.uuid4())
    result = {
        "schema_version": 1,
        "remote_task_id": task_id,
        "role": role,
        "idempotency_key": text(idempotency_key, "idempotency_key"),
        "input_hash": job.protocol_hash,
        "command_hash": command_hash,
        "runner_hash": runner_hash,
        "runner_version": 1,
        "ssh_target": job.spec["runtime"]["ssh_target"],
        "remote_root": job.spec["runtime"]["remote_root"],
        "sandbox_name": "evolve-" + task_id,
        "sandbox_id": None,
        "engine_id": None,
        "receipt_path": job.spec["runtime"]["remote_root"]
        + "/executions/"
        + task_id
        + "/control/receipt.json",
        "phase": "prepared",
        "submission_status": "not_submitted",
        "process": None,
        "exit_code": None,
        "termination_reason": None,
        "artifact_manifest": [],
        "export_status": "not_exported",
        "cleanup_status": "not_cleaned",
        "cancel_reason": None,
        "command_deadline": command_deadline,
        "resource_deadline": run_deadline,
    }
    validate_execution(result)
    return result


def validate_execution(value):
    fields(
        value,
        "RemoteExecution",
        {
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
            "sandbox_id",
            "engine_id",
            "receipt_path",
            "phase",
            "submission_status",
            "process",
            "exit_code",
            "termination_reason",
            "artifact_manifest",
            "export_status",
            "cleanup_status",
            "cancel_reason",
            "command_deadline",
            "resource_deadline",
        },
    )
    if (
        type(value["schema_version"]) is not int
        or value["schema_version"] != 1
        or type(value["runner_version"]) is not int
        or value["runner_version"] != 1
    ):
        raise ProtocolError("unsupported execution or runner schema")
    try:
        uuid.UUID(value["remote_task_id"])
    except (ValueError, TypeError, AttributeError):
        raise ProtocolError("invalid remote_task_id") from None
    choice(value["role"], ROLES, "role")
    choice(value["phase"], PHASES, "phase")
    for key in ["input_hash", "command_hash", "runner_hash"]:
        fingerprint(value[key], key)
    for key in [
        "ssh_target",
        "remote_root",
        "idempotency_key",
        "sandbox_name",
        "receipt_path",
    ]:
        text(value[key], key)
    expected_receipt = (
        remote_path(value["remote_root"])
        + "/executions/"
        + value["remote_task_id"]
        + "/control/receipt.json"
    )
    if (
        value["sandbox_name"] != "evolve-" + value["remote_task_id"]
        or value["receipt_path"] != expected_receipt
    ):
        raise ProtocolError("remote execution resource or receipt identity has drifted")
    if value["sandbox_id"] is not None:
        text(value["sandbox_id"], "sandbox_id")
    if value["engine_id"] is not None:
        text(value["engine_id"], "engine_id")
    choice(
        value["submission_status"],
        {
            "not_submitted",
            "intent_saved",
            "resource_created",
            "launch_intent_saved",
            "runner_submitted",
            "unresolved",
        },
        "submission_status",
    )
    choice(
        value["export_status"],
        {"not_exported", "partial", "exported", "unresolved"},
        "export_status",
    )
    choice(
        value["cleanup_status"],
        {"not_cleaned", "pending", "terminated", "unresolved"},
        "cleanup_status",
    )
    for key in ["command_deadline", "resource_deadline"]:
        number(value[key], key, minimum=1)
    if value["command_deadline"] >= value["resource_deadline"]:
        raise ProtocolError("command deadline must leave a recovery window")
    if value["exit_code"] is not None and type(value["exit_code"]) is not int:
        raise ProtocolError("exit_code must be integer or null")
    for key in ["termination_reason", "cancel_reason"]:
        if value[key] is not None:
            text(value[key], key)
    if value["phase"] == "running" and value["process"] is None:
        raise ProtocolError("running receipt must identify the original process")
    if value["process"] is not None:
        fields(value["process"], "process", {"pid", "pgid", "start_identity"})
        for key in ["pid", "pgid"]:
            number(value["process"][key], key, minimum=1, integer=True)
        text(value["process"]["start_identity"], "process.start_identity")
    paths = set()
    for artifact in rows(value["artifact_manifest"], "artifact_manifest", empty=True):
        fields(artifact, "artifact", {"path", "size", "sha256"})
        name = relative_path(artifact["path"])
        if name in paths:
            raise ProtocolError("duplicate artifact path")
        paths.add(name)
        fingerprint(artifact["sha256"], "artifact.sha256")
        number(artifact["size"], "artifact.size", integer=True)
    return value
