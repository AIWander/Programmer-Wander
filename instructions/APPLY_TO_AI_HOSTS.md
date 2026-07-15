# Apply Programmer-Wander to an AI host

Install the matching x64 or ARM64 Rust release first and make `programmer.exe` available on `PATH`. This package does not silently edit another AI's configuration or hooks.

## Codex

Install the repository root through Codex's plugin flow. The host can then discover `.codex-plugin/plugin.json`, `.mcp.json`, and both skills. Start a fresh task if the current one does not refresh plugin tools.

## Claude Code

Install the repository root through Claude Code's plugin flow. The `.claude-plugin/plugin.json`, `.mcp.json`, and `skills/` surfaces are intentionally colocated. Reload Claude Code after installation.

## Claude Desktop, LM Studio, and other local MCP hosts

Use the `mcpServers.programmer-wander` object in `.mcp.json` as a template in the host's own MCP settings. Confirm that `programmer.exe` resolves for that host account, save through the host's supported flow, and restart the host.

## ChatGPT, Grok, and other web-only UIs

A browser UI cannot launch a local stdio executable by itself. Use an authenticated remote MCP bridge or a supported desktop connector, then add the remote endpoint through that product's own UI. Do not paste a local Windows path into a web-only configuration and call it connected.

## Verify

Request the host's MCP status and `tools/list`. The plugin can provide instructions even when the executable failed to start, so verify both the skill and tool layers independently.

## Optional hooks

Read `hooks/OPT_IN.md`. No hook is enabled automatically, and the Codex plugin manifest deliberately does not claim a hook capability.
