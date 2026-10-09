#!/usr/bin/env python3
"""Read-only context reminder for the verified Codex 0.160.0 transcript format."""

import json
import os
from pathlib import Path
import re
import sqlite3
import sys
import uuid


# Percent of the window; the handler command may override it with `--threshold N`.
DEFAULT_THRESHOLD = 65

UNAVAILABLE = {
    "systemMessage": "上下文 goal guard 检测不可用；请核对 CODEX_HOME、会话格式及随附的启用说明。"
                     "持续异常时请在 /hooks 中停用本 guard 并按说明排查。"
}


def root_transcript(data, home):
    if data.get("agent_id"):
        return None
    session_id = data["session_id"]
    if not isinstance(session_id, str):
        raise ValueError("invalid session identity")
    uuid.UUID(session_id)
    path = Path(data["transcript_path"])
    if not path.is_absolute() or not path.resolve().is_relative_to((home / "sessions").resolve()):
        raise ValueError("transcript outside current home")
    with path.open("rb") as stream:
        header = stream.readline(1024 * 1024)
    if not header.endswith(b"\n"):
        raise ValueError("incomplete session metadata")
    record = json.loads(header)
    if record["type"] != "session_meta":
        raise ValueError("missing session metadata")
    meta = record["payload"]
    if not isinstance(meta, dict):
        raise ValueError("unknown session metadata")
    source = meta.get("source")
    if meta.get("parent_thread_id") or isinstance(source, dict) and "subagent" in source:
        return None
    if (meta["id"] != session_id or meta["session_id"] != session_id
            or source not in ("cli", "exec", "vscode") or meta["cli_version"] != "0.160.0"):
        raise ValueError("unverified identity or format")
    return path


def goal_active(home, thread):
    database = home / "goals_1.sqlite"
    con = sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True, timeout=0.2)
    try:
        found = con.execute("SELECT status FROM thread_goals WHERE thread_id = ?", (thread,)).fetchone()
    finally:
        con.close()
    if found is not None and not isinstance(found[0], str):
        raise ValueError("unknown goal status format")
    return found is not None and found[0] == "active"


def reverse_records(stream):
    stream.seek(0, os.SEEK_END)
    remaining = stream.tell()
    pending = b""
    discard_tail = True
    while remaining:
        size = min(65536, remaining)
        remaining -= size
        stream.seek(remaining)
        parts = (stream.read(size) + pending).split(b"\n")
        pending = parts.pop(0)
        for line in reversed(parts):
            # Drop the trailing fragment, or the empty line after a final newline.
            if discard_tail:
                discard_tail = False
            elif line.strip():
                yield json.loads(line)
    if pending.strip() and not discard_tail:
        yield json.loads(pending)


def context_usage(path, model):
    if not isinstance(model, str) or not model:
        raise ValueError("missing current model")
    candidate = None
    newer_context = False
    windows = []
    window_ids = []
    with path.open("rb") as stream:
        for record in reverse_records(stream):
            kind = record["type"]
            payload = record["payload"]
            if not isinstance(payload, dict):
                raise ValueError("unknown record payload")
            if kind == "compacted" or (kind == "event_msg" and payload.get("type") in
                                      ("context_compacted", "model_context_changed", "model_changed")):
                if candidate is None or newer_context:
                    raise ValueError("statistics invalidated")
                return candidate
            if kind == "turn_context":
                if payload["model"] != model:
                    raise ValueError("statistics belong to another model")
                context = payload.get("context_window")
                if context is not None:
                    identity = context["window_id"]
                    if not isinstance(identity, str) or not identity:
                        raise ValueError("unknown context identity")
                    window_ids.append(identity)
                if "model_context_window" in payload:
                    windows.append(payload["model_context_window"])
                if candidate is not None:
                    if any(type(w) is not int or w != candidate[1] for w in windows):
                        raise ValueError("context window changed")
                    if len(set(window_ids)) > 1 or window_ids and context is None:
                        raise ValueError("context identity changed")
                    return candidate
                newer_context = True
            elif kind == "event_msg" and payload.get("type") == "token_count" and candidate is None:
                info = payload["info"]
                used = info["last_token_usage"]["total_tokens"]
                window = info["model_context_window"]
                if type(used) is not int or used < 0 or type(window) is not int or window <= 0:
                    raise ValueError("invalid token statistics")
                candidate = used, window
            elif kind == "session_meta":
                if candidate is not None and not newer_context:
                    return candidate
                break
            elif kind not in ("event_msg", "response_item", "token_usage_record", "world_state"):
                raise ValueError("unknown record format")
    raise ValueError("no current token statistics")


def threshold(args):
    if not args:
        return DEFAULT_THRESHOLD
    if len(args) == 2 and args[0] == "--threshold" and re.fullmatch(r"[1-9][0-9]?", args[1]):
        return int(args[1])
    raise ValueError("invalid threshold argument")


def reminder(used, window, limit):
    return (
        f"本地上下文估算使用率 {used / window * 100:.2f}% 已严格超过 {limit}%，当前 goal 仍 active。"
        "停止开始新的实质任务，先将已启动工作处理到可交接状态；沿用已有任务计划和记录更新进度，"
        "写清已完成、未完成、已验证、未验证、下一步及仍在运行的进程。没有既有记录时在最终回复交接即可，"
        f"记录写入失败也先在回复保存最小交接摘要。仅当用户对当前 goal 有显式的 {limit}% 暂停请求时，"
        "按该请求使用原生 goal 工具设为 paused 并核对；不清楚则向用户确认并如实说明尚未暂停，提示 /goal pause。"
        "hook 不构成暂停授权；后续 resume 撤销原请求。不要以 complete 代替暂停或直接写数据库。"
        "然后报告真实状态、记录位置或交接摘要，提醒用户在同一工作目录自行新开对话继续，"
        "新对话不会继承 goal，需用户手动设置 goal 并重新明确暂停策略。重复提醒视为同一次收尾。"
    )


def run(data, args=()):
    if not isinstance(data, dict):
        raise ValueError("invalid hook input")
    event = data["hook_event_name"]
    if event not in ("PreToolUse", "Stop"):
        return {}
    if event == "Stop" and type(data["stop_hook_active"]) is not bool:
        raise ValueError("invalid stop state")
    home = Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex")))
    if not home.is_absolute():
        raise ValueError("unknown Codex home")
    home = home.resolve()
    path = root_transcript(data, home)
    if path is None or not goal_active(home, data["session_id"]):
        return {}
    limit = threshold(args)
    used, window = context_usage(path, data["model"])
    if used * 100 <= window * limit:
        return {}
    if event == "PreToolUse":
        return {"hookSpecificOutput": {"hookEventName": event, "additionalContext": reminder(used, window, limit)}}
    if not data["stop_hook_active"]:
        return {"decision": "block", "reason": reminder(used, window, limit)}
    return {"systemMessage": f"上下文超过 {limit}%，goal 仍 active，收尾暂停尚未成功。"
                             "请在原对话执行 /goal pause，再按交接说明自行新开对话；此告警不停止 goal 自动续跑。"}


if __name__ == "__main__":
    try:
        result = run(json.load(sys.stdin), sys.argv[1:])
    except (ValueError, TypeError, KeyError, OSError, sqlite3.Error):
        result = UNAVAILABLE
    print(json.dumps(result))
