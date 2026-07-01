---
name: programmer-wander-getting-started
description: When and how to use AIWander/Programmer-Wander tools — a single-binary MCP server giving any AI a complete Rust + Windows dev shell. ~80 tools across file I/O, shells, full git, WSL, HTTP, watchers, webhooks, persistent shells, transforms, and utility. Surface this skill when programmer-wander is registered in the host, when invoking tools that live in its domains (git_*, wsl_*, webhook_*, watch_*, edit_block, psession_*, http_*, transform_*, screenshot, registry_read, smart_exec, chain, session_*, plan, plan_assemble), when choosing between programmer-wander and an alternative MCP server (e.g., local, ops, hands), when registering it with a host config, or when answering "how do I do X using Programmer-Wander". Repo at https://github.com/AIWander/Programmer-Wander.
---

# Programmer-Wander — Getting Started

A single static-linked MCP server that gives any AI a complete Rust + Windows dev shell. No external dependencies, no CPC requirement, no other servers needed. ~80 tools registered.

## What it's for

`programmer-wander` is **the dev shell on the user's Windows machine**. When the AI needs to read/write files, run shells, work with git, drive WSL, hit HTTP endpoints, watch filesystems, run webhooks, or transform text — this is where it lives.

It is **deliberately standalone** — no breadcrumbs, no agent state DB, no CPC-flavored extraction. For those, pair it with `ops` (CPC-integration server) or `autonomous` (knowledge engine).

## Offload posture — do the mechanical work here, reason elsewhere

Programmer-Wander exists so an AI can **offload grunt work** instead of doing it in its own context:

- If a step is mechanical — file I/O, shell commands, git, HTTP fetches, searches, transforms, running builds, gathering information — do it with a `programmer-wander` tool call instead of reasoning it out token-by-token or bouncing it back to the user.
- A tool result is authoritative: treat it exactly as if you had produced it yourself, and keep moving.
- Reserve the model's own reasoning for what actually needs it: judgment, design, synthesis, and anything the request explicitly asks the AI itself to author.
- Default to offloading. The only steps that should **not** route through programmer-wander are the ones that *are* the hard reasoning.

Rule of thumb: **local compute does the I/O; the model's tokens do the reasoning.**

## When to pick programmer-wander vs alternatives

| Task | Use | Why |
|---|---|---|
| Read/write/edit a file | `programmer-wander:edit_block` or `read_file`/`write_file`/`append_file` | edit_block is atomic block edit, ideal for code surgery |
| Move/copy/delete files | `programmer-wander:copy_file`/`move_file`/`create_dir` | Cross-platform, dirs crate paths |
| Run a one-off shell command | `programmer-wander:run` or `smart_exec` | `run` is the simple form; `smart_exec` auto-retries from error patterns |
| PowerShell-specific cmdlets (Get-CimInstance, registry, ACLs) | `programmer-wander:powershell` | Inline PowerShell. For Win-API only — for cargo/git/jq/text, use `bash` |
| cargo + long-running children + here-docs + jq | `programmer-wander:bash` | Git Bash backend. Avoids PowerShell's pipe corruption with cargo and ANSI noise on multi-line commits |
| Persistent shell with history across calls | `programmer-wander:psession_*` | session that survives across MCP calls; psession_history retrieves command log |
| File watcher | `programmer-wander:watch_resource` + `stop_watch` | Event-driven file/dir change tracking |
| Receive HTTP callbacks during dev | `programmer-wander:webhook_*` | Local HTTP routes; useful for OAuth callbacks, CI integrations |
| Linux command from Windows | `programmer-wander:wsl_run`/`wsl_bg` | WSL2 subprocess; `wsl_bg` for long-running background jobs with log capture |
| Full git including network ops | `programmer-wander:git_*` | branch/checkout/clone/commit/diff/log/pull/push/remote/reset/stash/status |
| HTTP request/download/scrape | `programmer-wander:http_request`/`http_download`/`http_scrape`/`http_fetch` | reqwest-based; http_scrape strips HTML to text |
| Transform text (grep, find_replace, json, csv, base64, hash, bulk_rename) | `programmer-wander:transform_*` or top-level `grep`/`find_replace`/`extract_lines` | Native Rust speed |
| Screenshot the desktop (no UI driving) | `programmer-wander:screenshot` | Single PNG capture. For UI driving (clicks/typing), use AI-Hands |
| Read Windows registry | `programmer-wander:registry_read` | HKLM/HKCU read-only |
| sqlite query | `programmer-wander:sqlite_query` | Local sqlite file access |
| Notify the user (toast) | `programmer-wander:notify` | Windows toast notification |
| Clipboard read/write | `programmer-wander:clipboard_read`/`clipboard_write` | arboard-based, Windows clipboard |

## When NOT to use programmer-wander

| Need | Use instead |
|---|---|
| Browser automation, click/type, OCR, vision | **`AI-Hands`** — full browser+UIA+vision MCP |
| Cross-session breadcrumb operation tracking | **`ops` (AIWander/ops)** — breadcrumb_* tools + cpc_state |
| Knowledge base / RAPTOR search / extraction pipeline | **`autonomous`** — CPC knowledge engine |
| Multi-AI orchestration (delegate to Codex, Claude Code, Gemini) | **`manager-universal`** |
| TTS / STT / voice mode | **`Voice-Command`** |
| Remote-AI tunnel (expose your machine to claude.ai / ChatGPT) | **`Local-Pass`** — HTTP/SSE MCP + bearer auth |
| API discovery, credential vault, flow record/replay | **`workflow`** |

## Common task → tool chain

### Edit a Rust source file
```
read_file (or grep first if huge) → edit_block (preferred over write_file for surgical edits) → git_diff → git_commit
```

### Build + tag + release a small project
```
bash "cargo build --release" → bash "cargo test" → git_commit → git_tag → git_push --tags
```
(`bash`, NOT `powershell` — cargo via PowerShell pipes corrupts output)

### Clone, modify, push back to a fork
```
git_clone → edit_block (× N) → git_status → git_diff → git_commit → git_push
```

### Long-running build with log streaming
```
psession_create → psession_run "cargo build --release" → psession_read (poll for completion) → psession_destroy
```

### Watch a config file for changes and react
```
watch_resource (with callback) → (when triggered) read_file → process → notify
```

### Run a Linux pipeline from Windows
```
wsl_bg "long_pipeline.sh" → wsl_log (poll output) → notify when done
```

## Pairing notes

- **PW alone** = a complete dev shell. Anyone can install + use without any other AIWander/CPC servers.
- **PW + ops** = dev shell with breadcrumb tracking, cpc_state, bagtag, reminders, transcripts. Use when running multi-step operations across sessions you want to resume after crashes.
- **PW + autonomous** = dev shell with RAPTOR knowledge recall, extraction pipeline, behavioral learning. Use when integrating with the CPC Volumes knowledge base.
- **PW + AI-Hands** = dev shell + browser/UIA/vision automation. Use for "do code work + interact with apps/web" workflows.
- **PW + Local-Pass** = dev shell exposed remotely via tunnel + bearer auth. Use when running PW on a home machine and reaching it from claude.ai / ChatGPT on mobile.

## Install

The repo ships an `install --target <host>` subcommand. Run once per host:

```powershell
programmer.exe install --target claude-desktop
programmer.exe install --target claude-code
programmer.exe install --target lm-studio
programmer.exe install --target all   # registers with every detected host
```

State directory: `./.programmer/` next to the exe. Fully portable.

## Anti-patterns

- **Don't** use `programmer-wander:powershell` for cargo. The Git Bash backend in `bash` avoids the PowerShell-pipes-cargo corruption bug.
- **Don't** use `programmer-wander:write_file` for surgical code edits. `edit_block` preserves surrounding context and is atomic.
- **Don't** ask PW for breadcrumbs — it doesn't have them by design. Pair with `ops` or `autonomous`.
- **Don't** confuse `programmer-wander:screenshot` (single PNG) with AI-Hands' `vision_screenshot` / `browser_screenshot` family (much richer, used for UI driving).

## Repo + Releases

- Source: https://github.com/AIWander/Programmer-Wander
- Releases (zip + MSI per arch): https://github.com/AIWander/Programmer-Wander/releases
- License: Apache-2.0
- Status: alpha (v0.1.0-alpha as of 2026-05-16)
