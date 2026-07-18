# Plugins

Install exactly one current Programmer-Wander profile.

| Plugin | Purpose |
| --- | --- |
| `programmer` | Five skills plus inert guard-hook templates for hosts that can review, trust, and probe hooks |
| `programmer-skills` | The same five skills with no hook code |

Both profiles expect an existing `programmer` MCP connection (register the
server with `programmer.exe install --target <host>`). Neither profile installs
an MCP endpoint, edits host configuration, swaps a binary, or restarts the
server. Hook wiring in the `programmer` profile is an explicit user step via
`scripts/render-hooks.ps1` after reviewing the policy source.
