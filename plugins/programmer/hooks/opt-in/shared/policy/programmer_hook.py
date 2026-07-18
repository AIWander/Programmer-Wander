#!/usr/bin/env python3
"""Portable guard policy for optional Programmer-Wander hook templates.

Read-only against the payload; writes nothing but an audit line. Decisions:
  deny  - destructive command patterns, kills aimed at live MCP servers
  warn  - cargo via powershell, writes into protected config roots
  observe - everything else
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

COMMAND_TOOLS = {
    "run", "bash", "powershell", "smart_exec", "chain",
    "psession_run", "wsl_run", "wsl_bg",
}
WRITE_TOOLS = {
    "write_file", "edit_block", "move_file", "copy_file",
    "transform_find_replace", "transform_sync_dir", "transform_bulk_rename",
}
SERVER_EXES = (
    "programmer", "autonomous", "hands", "local", "manager",
    "workflow", "voice", "ops", "uniman",
)
DISPOSABLE = ("target/", "target\\", "build/", "build\\", "tmp/", "tmp\\",
              ".cache", "node_modules", "dist/", "dist\\")

RECURSIVE_DELETE = re.compile(
    r"(rm\s+(-[a-z]*r[a-z]*f|-[a-z]*f[a-z]*r)[a-z]*\b|remove-item\b.*-recurse.*-force|rmdir\s+/s|del\s+/s)",
    re.IGNORECASE,
)
FORCE_PUSH = re.compile(r"git\s+push\b.*(\s--force\b|\s-f\b)", re.IGNORECASE)
PROCESS_KILL = re.compile(r"(taskkill\b|stop-process\b|kill(all)?\s)", re.IGNORECASE)
DISK_DESTROY = re.compile(r"(format-volume|diskpart|cipher\s+/w|mkfs\.)", re.IGNORECASE)
CARGO = re.compile(r"\b(cargo|rustc)\b", re.IGNORECASE)


def read_payload() -> dict[str, Any]:
    raw = sys.stdin.read()
    if not raw.strip():
        return {}
    value = json.loads(raw)
    return value if isinstance(value, dict) else {}


def tool_name(payload: dict[str, Any]) -> str:
    for key in ("tool_name", "toolName", "name"):
        value = payload.get(key)
        if isinstance(value, str):
            return value
    return ""


def base_tool(payload: dict[str, Any]) -> str:
    """Strip mcp__programmer__ style prefixes down to the bare tool name."""
    return tool_name(payload).split("__")[-1]


def tool_args(payload: dict[str, Any]) -> dict[str, Any]:
    for key in ("tool_input", "toolInput", "arguments", "args"):
        value = payload.get(key)
        if isinstance(value, dict):
            return value
    return {}


def command_text(args: dict[str, Any]) -> str:
    parts = []
    for key in ("command", "cmd", "script", "commands"):
        value = args.get(key)
        if isinstance(value, str):
            parts.append(value)
        elif isinstance(value, list):
            parts.extend(str(item) for item in value)
    return "\n".join(parts)


def protected_path(path: str) -> bool:
    lowered = path.replace("\\", "/").lower()
    home = str(Path.home()).replace("\\", "/").lower()
    roots = (
        f"{home}/.claude/", f"{home}/.codex/",
        "appdata/roaming/claude", "c:/cpc/config",
    )
    if any(root in lowered for root in roots):
        return True
    name = lowered.rsplit("/", 1)[-1]
    return name.startswith("operating_") or name == "cpc_state.json"


def emit_context(event: str, message: str) -> None:
    sys.stdout.write(json.dumps({
        "hookSpecificOutput": {"hookEventName": event, "additionalContext": message}
    }, ensure_ascii=True))


def emit_deny(event: str, reason: str) -> None:
    sys.stdout.write(json.dumps({
        "decision": "deny",
        "hookSpecificOutput": {
            "hookEventName": event,
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        },
    }, ensure_ascii=True))


def audit(event: str, host: str, payload: dict[str, Any], decision: str, detail: str) -> None:
    try:
        root = Path(os.environ.get("LOCALAPPDATA") or Path.home()) / "ProgrammerWander" / "hooks"
        root.mkdir(parents=True, exist_ok=True)
        entry = {
            "timestamp": dt.datetime.now(dt.timezone.utc).isoformat(),
            "event": event,
            "host": host,
            "tool": tool_name(payload),
            "decision": decision,
            "detail": detail,
        }
        with (root / "audit.jsonl").open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(entry, ensure_ascii=True, sort_keys=True) + "\n")
    except OSError:
        pass


def check_command(event: str, cmd: str) -> tuple[str, str]:
    """Return (decision, message) for a command string."""
    if DISK_DESTROY.search(cmd):
        return "deny", "Disk-destroying command pattern blocked by the Programmer guard hook."
    if RECURSIVE_DELETE.search(cmd) and not any(token in cmd.lower() for token in DISPOSABLE):
        return ("deny",
                "Recursive delete outside disposable paths (target/, build/, tmp/, .cache, "
                "node_modules, dist/). Narrow the target or delete interactively.")
    if FORCE_PUSH.search(cmd) and "--force-with-lease" not in cmd:
        return ("deny",
                "Bare force-push blocked. Use --force-with-lease, or have the user push.")
    if PROCESS_KILL.search(cmd) and any(exe in cmd.lower() for exe in SERVER_EXES):
        return ("deny",
                "Command appears to kill a running MCP server. Stage the change "
                "(.new + swap) and let the user restart at their boundary.")
    if CARGO.search(cmd):
        return ("warn-cargo", "")
    return "observe", ""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", required=True)
    parser.add_argument("--host", required=True, choices=("claude", "codex"))
    options = parser.parse_args()

    try:
        payload = read_payload()
    except (json.JSONDecodeError, OSError):
        return 0

    event = options.event
    decision, detail = "observe", ""

    if event == "SessionStart":
        emit_context(event, "Programmer-Wander guard hooks active: destructive commands, "
                            "server kills, and bare force-pushes are denied; writes to "
                            "protected config roots get an archive-first reminder.")
    elif event == "PreToolUse":
        tool = base_tool(payload)
        args = tool_args(payload)
        if tool in COMMAND_TOOLS:
            cmd = command_text(args)
            decision, message = check_command(event, cmd)
            if decision == "deny":
                detail = message
                audit(event, options.host, payload, decision, detail)
                emit_deny(event, message)
                return 0
            if decision == "warn-cargo" and tool == "powershell":
                decision, detail = "warn", "cargo-via-powershell"
                emit_context(event, "cargo/rustc through the powershell tool corrupts piped "
                                    "output. Use the bash tool for cargo work.")
        elif tool == "kill_process":
            target = str(args.get("name", "")) + " " + str(args.get("process", ""))
            if any(exe in target.lower() for exe in SERVER_EXES):
                decision = "deny"
                detail = "kill_process on MCP server"
                audit(event, options.host, payload, decision, detail)
                emit_deny(event, "kill_process target looks like a live MCP server. Verify with "
                                 "server_health; stage a swap instead of killing it mid-session.")
                return 0
        elif tool in WRITE_TOOLS:
            paths = " ".join(str(args.get(key, "")) for key in
                             ("path", "file_path", "destination", "target", "source"))
            if protected_path(paths):
                decision, detail = "warn", "write-to-protected-root"
                emit_context(event, "Target is under a protected config root. Archive-first: "
                                    "copy the current version to a backups dir before overwriting.")
    elif event == "PostToolUseFailure":
        emit_context(event, "Programmer tool failed. Consider tool_fallback for an alternative, "
                            "smart_exec for auto-retry, or security_audit_log if it was blocked.")

    audit(event, options.host, payload, decision, detail)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
