---
name: programmer-background-ops
description: Long-running and event-driven work with Programmer-Wander - persistent shell sessions, WSL background jobs, filesystem watchers, local webhooks, and delta log polling. Surface when a task runs longer than one tool call, needs to react to file changes or HTTP callbacks, or spans Windows and WSL.
---

# Programmer Background Ops

## Persistent shells (psession)

State - variables, CWD, environment - survives across MCP calls.

```
psession_create              (PowerShell or WSL flavor)
psession_run "cmd"           (repeat as needed; state persists)
psession_read                (output buffer)
psession_history             (command log)
psession_destroy             (always clean up)
```

Use for: multi-step builds, REPL-ish exploration, anything where `cd` or an
env var must stick.

## WSL background jobs

```
wsl_bg "./long_pipeline.sh"   -> returns job_id
wsl_status job_id             (or job_id=all)
wsl_log job_id                (full or partial log)
```

One-shot Linux commands: `wsl_run` (returns summary + log path).

## Filesystem watchers

```
watch_resource path           -> watch id
get_alert                     (fired events)
list_watch / stop_watch
```

Pattern: watch a config or artifact dir, react on change
(`read_file` -> process -> `notify`). Always `stop_watch` when the task ends -
orphaned watchers keep firing.

## Local webhooks

```
webhook_add_route -> webhook_start -> (external system calls in) -> webhook_stop
```

Use for OAuth callbacks and CI pings during development. Loopback only; do not
expose routes beyond localhost.

## Delta log polling

`tail_file` returns the last N lines plus a byte offset. Pass `since_bytes`
from the previous call to get only NEW content - the cheap way to follow a
growing log without re-reading it.

## Choosing the shape

| Situation | Tool |
|---|---|
| State must persist across calls | `psession_*` |
| Long Linux job, poll later | `wsl_bg` + `wsl_log` |
| React to file changes | `watch_resource` + `get_alert` |
| Receive an HTTP callback | `webhook_*` |
| Follow a log file | `tail_file` with `since_bytes` |
