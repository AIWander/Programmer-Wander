# programmer-skills (plugin profile)

Skill-only profile: the same five skills as the `programmer` profile with no
hook code of any kind. For hosts or users that do not want lifecycle hooks.

- `skills/` - programmer-getting-started, programmer-toolmap,
  programmer-dev-loop, programmer-background-ops, programmer-safe-ops

Expects an existing `programmer` MCP connection
(`programmer.exe install --target <host>`). Install exactly one profile -
this one or `programmer`, not both.
