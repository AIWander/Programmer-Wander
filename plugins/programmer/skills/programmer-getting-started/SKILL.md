---
name: programmer-getting-started
description: Orientation for the Programmer-Wander MCP server - a single-binary Rust + Windows dev shell with ~105 tools organized into 11 categories (Files, Search, Shell, Git, Sessions, Background, Net, Data, System, Guard, Plan). Surface when programmer is registered in the host, when picking a tool for file I/O, shells, git, WSL, HTTP, watchers, webhooks, or transforms, when choosing between programmer and an alternative MCP server, or when answering "how do I do X with programmer".
---

# Programmer - Getting Started

Programmer-Wander is **the dev shell on this Windows machine**: one static-linked
`programmer.exe`, no external dependencies, no other servers required. Every tool
description is prefixed with its category tag, so a flat tool list still reads
grouped:

| Tag | What lives there |
|---|---|
| `[Files]` | read/write/append/edit_block, copy/move, dirs, diff, tail, stats |
| `[Search]` | grep, search_file, search_start, smart_read |
| `[Shell]` | bash, powershell, run, chain, smart_exec, shortcuts |
| `[Git]` | full git - status through clone/push/stash/remote |
| `[Sessions]` | persistent shells (psession_*) + tracked sessions with recovery |
| `[Background]` | file watchers, webhooks, WSL background jobs |
| `[Net]` | http_request/download/scrape, port_check |
| `[Data]` | transforms (json/csv/base64/hash/rename/scaffold), archives, sqlite, md2docx |
| `[System]` | screenshot, clipboard, processes, registry, notify |
| `[Guard]` | security_check_cmd, audit log, deploy_preflight, tool_fallback |
| `[Plan]` | plan, plan_assemble |

Full per-tool reference: see the `programmer-toolmap` skill.

## Offload posture

Do mechanical work with programmer tool calls instead of reasoning it out
token-by-token: file I/O, shell commands, git, fetches, searches, transforms,
builds. Local compute does the I/O; the model's tokens do the reasoning.

## First moves

- One-off command: `run` (simple) or `bash` (Git Bash - use for cargo, jq, here-docs)
- Windows-specific cmdlets/ACLs/CIM: `powershell`
- Surgical code edit: `edit_block` (never `write_file` for edits)
- Long build: `psession_create` then `psession_run` then `psession_read`
- Linux from Windows: `wsl_run`, or `wsl_bg` + `wsl_log` for long jobs

## When NOT to use programmer

| Need | Use instead |
|---|---|
| Browser/UI automation, OCR, vision | AI-Hands |
| Cross-session breadcrumbs / agent state | ops or autonomous |
| Knowledge base search / extraction | autonomous |
| Multi-AI delegation | manager / UniMan |
| Voice | Voice-Command |

## Hard-won rules

- cargo via `bash`, never `powershell` - PowerShell pipes corrupt cargo output.
- `edit_block` over `write_file` for code surgery - atomic, context-preserving.
- A locked .exe cannot be overwritten while running: build to an alternate
  target dir, then swap with a rename. See `programmer-safe-ops`.

## Install / registration

`programmer.exe install --target claude-desktop|claude-code|lm-studio|cowork|all`
State directory `./.programmer/` sits next to the exe; fully portable.
Repo: https://github.com/AIWander/Programmer-Wander
