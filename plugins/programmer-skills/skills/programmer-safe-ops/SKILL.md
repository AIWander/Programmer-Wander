---
name: programmer-safe-ops
description: Safety discipline for Programmer-Wander's powerful tools - command pre-flight, archive-first overwrites, staged binary swaps, process-kill hygiene, and audit trail. Surface before destructive commands, before overwriting configs or binaries, before kill_process, before deploys, and when a security warning or blocked command appears.
---

# Programmer Safe Ops

Programmer is a full dev shell. These rules keep it reversible.

## Command pre-flight

Command-entry tools (`run`, `bash`, `powershell`, `smart_exec`, `chain`,
`psession_run`, `wsl_run`, `wsl_bg`) pass through `security_check_cmd`
automatically. Critical destructive patterns are blocked and logged; recursive
deletes must target an obviously disposable path (`target/`, `build/`, `tmp/`,
`.cache/`). To check a command before running it, call `security_check_cmd`
directly. Review decisions with `security_audit_log`.

## Archive-first

Before replacing any working file, config, or binary: copy the current version
aside (`copy_file` to a backups dir with a date suffix). An overwrite you can
roll back is an experiment; one you cannot is an incident.

## Staged swaps - never hot-swap a running server

A running .exe is locked and may be an MCP server the host is actively using.

- Build to an alternate target dir (`CARGO_TARGET_DIR=...`).
- Stage the new binary next to the old one (`programmer.exe.new`).
- Rename-swap only when the process is stopped - and if the process is an MCP
  server, the USER restarts it at their boundary, not the AI mid-session.

## kill_process hygiene

Before `kill_process`, run `server_health` and `list_process` - confirm the
target is not a live MCP server or something the user is working in. Prefer
letting owners stop their own processes.

## Deploys

`deploy_preflight` verifies sources exist and servers are running before you
copy anything. Run it before any deploy-shaped operation.

## Escalation

If a command is blocked or warned, do not shell-escape around the guard
(alternate encodings, indirection through scripts). Either the command is safe
enough to fix properly, or it should not run.
