# programmer (plugin profile)

Full profile: five skills plus inert, reviewable guard-hook templates.

Contents:

- `skills/` - programmer-getting-started, programmer-toolmap,
  programmer-dev-loop, programmer-background-ops, programmer-safe-ops
- `hooks/opt-in/` - guard policy + adapters + fragments (never auto-loaded;
  see `hooks/opt-in/README.md`)
- `rendered-hooks/` - host-ready JSON produced by `scripts/render-hooks.ps1`
- `scripts/render-hooks.ps1` - renders fragments with this plugin's real path

This profile expects an existing `programmer` MCP connection. It does not
install the server, edit host configuration, or wire hooks by itself - hook
wiring is an explicit, reviewed user step.

If you want skills only and no hook code at all, install the
`programmer-skills` profile instead. Install exactly one profile.
