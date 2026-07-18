# Opt-in guard hooks

Nothing in this folder runs by itself. These are reviewed, inert templates: a
shared Python policy (`shared/policy/programmer_hook.py`), thin host adapters
(`adapters/claude`, `adapters/codex`), and fragments containing a
`__PLUGIN_ROOT__` placeholder.

To use them:

1. Read `shared/policy/programmer_hook.py` - it is short and the whole point
   is that you can review every decision it makes.
2. Run `scripts/render-hooks.ps1` to produce host-ready JSON in
   `rendered-hooks/` with real paths.
3. Merge the entries you want into your host's hook settings yourself.

What the policy does:

| Event | Behavior |
|---|---|
| SessionStart | One orientation line: the guard is active |
| PreToolUse (command tools) | Deny disk-destroying patterns, recursive deletes outside disposable paths, bare force-pushes, and commands that kill MCP server processes; warn on cargo via powershell |
| PreToolUse (kill_process) | Deny kills aimed at live MCP server executables |
| PreToolUse (write tools) | Warn (archive-first) on writes under protected config roots |
| PostToolUseFailure | Hint at tool_fallback / smart_exec / security_audit_log |

Every decision is appended to `%LOCALAPPDATA%/ProgrammerWander/hooks/audit.jsonl`.
The policy requires Python 3.10+ on PATH. It never blocks silently: a denial
always carries the reason back to the model.
