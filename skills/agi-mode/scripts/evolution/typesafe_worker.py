"""One text-only System One request; no tools, retries or provider selection."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import stat
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.request import HTTPRedirectHandler, Request, build_opener


ENDPOINT = "https://api.typesafe.ai/v1/systemone"
MODEL = "jev-1.14.0"


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, newurl):
        return None


def urlopen(request, *, timeout):
    return build_opener(NoRedirect()).open(request, timeout=timeout)


def json_pairs(items):
    result = {}
    for name, value in items:
        if name in result:
            raise ValueError("duplicate response field")
        result[name] = value
    return result


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")


def validate_request(value):
    if not isinstance(value, dict) or set(value) != {"state", "model", "questions"} or value["model"] != MODEL:
        raise ValueError("invalid request contract")
    state = value["state"]
    if not isinstance(state, dict) or set(state) != {"files"} or not isinstance(state["files"], dict):
        raise ValueError("only text evidence is supported")
    if not state["files"] or any(not isinstance(k, str) or not isinstance(v, str) for k, v in state["files"].items()):
        raise ValueError("missing text context")
    if not isinstance(value["questions"], dict) or not value["questions"]:
        raise ValueError("missing questions")
    for item in value["questions"].values():
        if not isinstance(item, dict) or set(item) != {"type", "instructions", "criteria"} or item["type"] != "score":
            raise ValueError("only Score is supported")
        if not isinstance(item["instructions"], str) or not item["instructions"].strip():
            raise ValueError("missing instructions")
        if not isinstance(item["criteria"], list) or len(item["criteria"]) != 3 or any(
            not isinstance(level, str) or not level.strip() for level in item["criteria"]
        ):
            raise ValueError("three complete levels are required")
    raw = canonical(value)
    if len(raw) > 32 * 1024 * 1024:
        raise ValueError("request is too large")
    return raw


def atomic_result(path, result):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=".response-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(canonical(result) + b"\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def run_request(request, credential_path, output, *, timeout, long_context=False):
    started = time.monotonic()
    result = {"status": "unknown", "reason": "invalid_request", "response": None,
              "request_sha256": None, "latency_sec": None, "request_attempted": False}
    key_path = Path(credential_path)
    try:
        raw = validate_request(request)
        result["request_sha256"] = hashlib.sha256(raw).hexdigest()
        if not math.isfinite(timeout) or timeout <= 0:
            raise ValueError("invalid timeout")
        if type(long_context) is not bool:
            raise ValueError("invalid long-context setting")
        descriptor = os.open(key_path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            info = os.fstat(stream.fileno())
            if not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600:
                raise OSError("credential is not private")
            key = stream.read().decode("utf-8").strip()
        if not key or any(character.isspace() for character in key):
            raise OSError("credential unavailable")
        headers = {
            "Authorization": "Bearer " + key, "Content-Type": "application/json",
        }
        if long_context:
            headers["TypeSafe-Danger-Zone-Long-Context"] = "true"
        packet = Request(ENDPOINT, data=raw, method="POST", headers=headers)
        result["request_attempted"] = True
        try:
            with urlopen(packet, timeout=timeout) as response:
                body = response.read(32 * 1024 * 1024 + 1)
            if len(body) > 32 * 1024 * 1024 or key.encode() in body:
                raise ValueError("invalid provider response")
            value = json.loads(body, object_pairs_hook=json_pairs,
                               parse_constant=lambda _: (_ for _ in ()).throw(ValueError()))
            canonical(value)
            result.update(status="valid", reason="response_received", response=value)
        except HTTPError as error:
            result.update(status="unavailable", reason="http_" + str(error.code))
        except (URLError, TimeoutError, OSError):
            result.update(status="unavailable", reason="upstream_outcome_unknown")
    except (ValueError, TypeError, UnicodeError):
        result.update(status="unknown", reason="malformed_request_or_response")
    except OSError:
        result.update(status="unavailable", reason="credential_unavailable")
    finally:
        key_path.unlink(missing_ok=True)
    result["latency_sec"] = time.monotonic() - started
    atomic_result(output, result)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--credential", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--timeout", required=True, type=float)
    parser.add_argument("--long-context", action="store_true")
    args = parser.parse_args()
    run_request(json.loads(args.request.read_bytes()), args.credential, args.output,
                timeout=args.timeout, long_context=args.long_context)


if __name__ == "__main__":
    main()
