---
name: programmer-getting-started
description: Install, connect, verify, and troubleshoot AIWander Programmer-Wander as a local Windows MCP server and optional Codex or Claude plugin. Use when setting up Programmer-Wander, choosing an x64 or ARM64 build, registering its MCP process with an AI host, checking whether the tools loaded, or diagnosing a missing or mismatched Programmer tool surface.
---

# Programmer Getting Started

Treat the plugin, skill, executable, and host connection as separate layers. The plugin supplies guidance and MCP registration metadata; `programmer.exe` supplies the tools; the AI host decides whether either is loaded.

## Connect it safely

1. Obtain the release matching the Windows architecture and place `programmer.exe` in a stable directory on `PATH`.
2. For a plugin-capable local host, install this repository as the plugin root so it can discover `.codex-plugin/` or `.claude-plugin/`, `.mcp.json`, and `skills/`.
3. For a local MCP host without plugin support, use `.mcp.json` as the template and add the entry through that host's own UI or documented configuration flow.
4. Do not silently edit another AI's config or hooks. The executable's `install --target` command is an explicit config mutation; run it only when the user asks for that target to be changed.
5. Restart or reload the host, request `tools/list`, and confirm the server identity before using mutation or command tools.

Read [the host application guide](../../instructions/APPLY_TO_AI_HOSTS.md) when the target UI is unclear.

## Verify the connection

- Run `programmer.exe --version` in a terminal to prove the executable resolves.
- Inspect the host's MCP status or logs for a successful stdio launch.
- Treat live `tools/list` as authoritative. The repository revision that introduced this skill defines 105 unique tools in `src/tools/mod.rs`; a different live count means the binary and source revision differ, not necessarily that startup failed.
- Confirm the target architecture when Windows reports an invalid executable or immediate process exit.

## Diagnose by layer

| Symptom | Check |
|---|---|
| Skill appears but no Programmer tools | `.mcp.json`, executable resolution, host MCP logs |
| Tools appear but skill does not | Plugin or skill installation path and host skill discovery |
| Local terminal works but a web AI cannot connect | A local stdio process is not remotely reachable; use an authenticated remote MCP bridge |
| Tool count or schema differs | Compare live `tools/list` with the installed binary version and source revision |
| A command is denied | Read the returned safety reason; do not bypass it with another shell wrapper |

## Boundaries

Programmer-Wander is a local development surface. It does not add browser clicking, OCR, credential storage, durable knowledge recall, or multi-agent orchestration. Route those abilities to a tool that actually owns them.
