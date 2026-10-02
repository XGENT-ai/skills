#!/usr/bin/env python3
"""Check receipt coverage and file freshness; never infer task success."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def digest(path):
    hasher = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def text(value, label):
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{label}: expected nonempty string")
    return value


def rows(value, label):
    if not isinstance(value, list) or not value:
        raise ValueError(f"{label}: expected nonempty list")
    return value


def record(value, label):
    if not isinstance(value, dict):
        raise ValueError(f"{label}: expected object")
    return value


def indexed(value, label):
    result = {}
    for item in rows(value, label):
        item = record(item, label)
        key = text(item.get("id"), f"{label}.id")
        if key in result:
            raise ValueError(f"{label}: duplicate id {key}")
        result[key] = item
    return result


def local_file(root, name):
    relative = Path(text(name, "path"))
    if relative.is_absolute():
        raise ValueError(f"path must be relative: {name}")
    path = (root / relative).resolve()
    if not path.is_relative_to(root):
        raise ValueError(f"path escapes root: {name}")
    if not path.is_file():
        raise ValueError(f"missing file: {name}")
    return path


def fingerprints(root, entries, label, issues):
    seen = set()
    for entry in rows(entries, label):
        entry = record(entry, label)
        name = text(entry.get("path"), f"{label}.path")
        expected = text(entry.get("sha256"), f"{label}.sha256")
        if not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise ValueError(f"{label}: invalid sha256 for {name}")
        try:
            path = local_file(root, name)
            if path in seen:
                raise ValueError(f"duplicate file: {name}")
            seen.add(path)
            if digest(path) != expected:
                issues.append(f"{label}: changed file {name}")
        except (ValueError, OSError) as error:
            issues.append(f"{label}: {error}")
    return seen


def check(root, contract_path, receipts_path):
    contract = record(json.loads(contract_path.read_text()), "contract")
    receipts = record(json.loads(receipts_path.read_text()), "receipts")
    text(contract.get("goal"), "contract.goal")
    requirements = indexed(contract.get("requirements"), "requirements")
    raw_checks = receipts.get("checks")
    checks = {} if raw_checks == [] else indexed(raw_checks, "checks")
    issues = []
    if receipts.get("contract_sha256") != digest(contract_path):
        issues.append("contract changed or receipt is bound to another contract")
    required = set()
    for key, requirement in requirements.items():
        for name in rows(requirement.get("checks"), f"requirement {key}.checks"):
            required.add(text(name, f"requirement {key}.check"))
    for key in sorted(required):
        if key not in checks:
            issues.append(f"{key}: missing required check")
            continue
        item = checks[key]
        if item.get("status") != "passed":
            issues.append(f"{key}: required check is not passed")
            continue
        text(item.get("observation"), f"{key}.observation")
        inputs = fingerprints(root, item.get("inputs"), f"{key}.inputs", issues)
        artifacts = fingerprints(root, item.get("artifacts"), f"{key}.artifacts", issues)
        if inputs & artifacts:
            issues.append(f"{key}: input and evidence must be separate files")
    return {
        "records_consistent": not issues,
        "requirements": len(requirements),
        "required_checks": len(required),
        "issues": issues,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    snapshot = commands.add_parser("fingerprint")
    snapshot.add_argument("--root", type=Path, required=True)
    snapshot.add_argument("paths", nargs="+")
    verify = commands.add_parser("check")
    verify.add_argument("--root", type=Path, required=True)
    verify.add_argument("--contract", type=Path, required=True)
    verify.add_argument("--receipts", type=Path, required=True)
    args = parser.parse_args()
    try:
        root = args.root.resolve(strict=True)
        if not root.is_dir():
            raise ValueError("root must be a directory")
        if args.command == "fingerprint":
            result = [
                {"path": name, "sha256": digest(local_file(root, name))}
                for name in args.paths
            ]
            code = 0
        else:
            result = check(root, args.contract, args.receipts)
            code = 0 if result["records_consistent"] else 1
    except (ValueError, OSError) as error:
        result = {"records_consistent": False, "issues": [str(error)]}
        code = 2
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
