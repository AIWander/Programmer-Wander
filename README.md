# Programmer-Wander

> An MCP server that gives any AI a complete Rust + Windows dev shell.

**Status:** free, public, and alpha. Built for [Claude Desktop](https://claude.ai/download), [Cowork](https://claude.ai/cowork), [LM Studio](https://lmstudio.ai), [Claude Code](https://claude.ai/code), and any host that speaks MCP.

**Part of [CPC](https://github.com/AIWander) (Copy Paste Compute).** Programmer-Wander is
one member of the free core trio alongside AI-Hands and Voice-Command.

[![Build](https://github.com/AIWander/Programmer-Wander/actions/workflows/build.yml/badge.svg)](https://github.com/AIWander/Programmer-Wander/actions)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows%20x64%20%7C%20ARM64-blue.svg)](https://github.com/AIWander/Programmer-Wander/releases)

## What it does

`programmer` is a single-binary MCP server that exposes a Rust developer's toolbox to any AI:

| Category | Tools |
|---|---|
| **Files** | read_file, write_file, append_file, copy_file, move_file, create_dir, diff_file, file_stats, get_file_info, edit_block |
| **Shells** | bash, powershell, run, smart_exec, chain |
| **Git (full)** | branch, checkout, clone, commit, diff, diff_summary, log, pull, push, remote, reset, stash, status |
| **WSL** | wsl_run, wsl_bg, wsl_log, wsl_status |
| **HTTP** | http_request, http_download, http_fetch, http_scrape |
| **Watchers** | watch_resource, list_watch, stop_watch |
| **Webhooks** | webhook_add_route, webhook_list, webhook_start, webhook_stop |
| **Persistent shells** | psession_create, psession_destroy, psession_history, psession_list, psession_read, psession_run |
| **Sessions** | session_*, session_history, session_checkpoint, session_recover |
| **Transforms** | grep, find_replace, extract_lines, json_format, csv↔json, base64_*, hash_file, bulk_rename, scaffold, sync_dir |
| **Utility** | screenshot, port_check, kill_process, list_process, archive_*, sqlite_query, md2docx, notify, clipboard_*, registry_read |

~80 tools total. No external dependencies. Single static-linked .exe. Works standalone — does not require any other MCP server.

## Safety model

Programmer-Wander is a powerful local dev shell. If your AI host can call it, the AI can read and write files, run shell commands, use git, call WSL, and operate on your clipboard. Install it only for AI clients you trust, and review the tools you enable.

Command-entry tools (`run`, `bash`, `powershell`, `smart_exec`, `chain`, `psession_run`, `wsl_run`, and `wsl_bg`) run through `security_check_cmd` before execution. Critical destructive patterns are blocked and logged; recursive delete-style commands must target an obviously disposable path such as `target/`, `build/`, `tmp/`, or `.cache/`.

AIWander tools are local, user-authorized MCP capability surfaces. They do not grant an AI new permissions by themselves. They expose tools the user explicitly installs and enables. Sensitive actions should be confirmed by the user, credentials should stay in the OS keyring or local vault, and demos should use mock data.

## Install

### Option 1 — Portable (recommended)

1. Download `programmer-windows-x64.zip` (or `arm64`) from [Releases](https://github.com/AIWander/Programmer-Wander/releases/latest)
2. Extract to a stable folder, e.g. `C:\tools\programmer\`
3. Register with your AI host:
   ```powershell
   C:\tools\programmer\programmer.exe install --target claude-desktop
   # or: --target lm-studio, --target cowork, --target claude-code
   # Advanced: --target all registers every detected host.
   ```
4. Restart your AI host

### Option 2 — MSI installer

1. Download `programmer-windows-x64.msi` from [Releases](https://github.com/AIWander/Programmer-Wander/releases/latest)
2. Run the MSI (UAC prompt; click Allow)
3. The installer registers detected hosts as a broad install path
4. For single-host registration, use the portable option above
5. Restart your AI host

### Option 3 — Have your AI install it for you

Open Claude / ChatGPT / your local LLM and paste:

> Install **AIWander/Programmer-Wander** on my Windows machine using the AI install runbook at <https://github.com/AIWander/Programmer-Wander#for-ai-assistants>

Your AI will follow the steps below.

## For AI Assistants

If you're an AI installing this for a user, run these steps in order:

### 1. Detect environment

```powershell
$arch = if ((Get-CimInstance Win32_Processor).Architecture -eq 12) { "arm64" } else { "x64" }
$hosts = @()
if (Test-Path "$env:APPDATA\Claude\claude_desktop_config.json") { $hosts += "claude-desktop" }
if (Test-Path "$env:USERPROFILE\.lmstudio\mcp.json")            { $hosts += "lm-studio"      }
if (Test-Path "$env:USERPROFILE\.claude\settings.json")         { $hosts += "claude-code"    }
Write-Host "Arch: $arch | Hosts detected: $($hosts -join ', ')"
```

### 2. Download latest release (portable)

```powershell
$url = "https://github.com/AIWander/Programmer-Wander/releases/latest/download/programmer-windows-$arch.zip"
Invoke-WebRequest $url -OutFile $env:TEMP\programmer.zip
New-Item -ItemType Directory -Force -Path C:\tools\programmer | Out-Null
Expand-Archive $env:TEMP\programmer.zip -DestinationPath C:\tools\programmer -Force
```

### 3. Register with detected hosts

```powershell
foreach ($h in $hosts) {
  C:\tools\programmer\programmer.exe install --target $h
}
```

### 4. Verify

```powershell
C:\tools\programmer\programmer.exe --version
```

### 5. Tell the user

"Programmer-Wander is installed and registered with: $($hosts -join ', '). Restart those host apps now and the new tools will appear."

## Uninstall

```powershell
C:\tools\programmer\programmer.exe uninstall --target all
Remove-Item C:\tools\programmer -Recurse -Force
```

## State directory

`programmer` keeps its state (breadcrumbs, file watchers, session checkpoints) in `./.programmer/` relative to the exe. This makes the install fully portable — copy the exe + its `./.programmer/` folder to a different machine and your state goes with it.

## Build from source

```bash
git clone https://github.com/AIWander/Programmer-Wander
cd Programmer-Wander
cargo build --release
# Binary at: target/release/programmer.exe
```

Requires Rust 1.75+.

## Companion: AIWander/Universal-Ops

Programmer-Wander is the **dev shell** for a single AI. The separate manager and dashboard
products are **Beta and coming soon**; their active test surface is
[manager-universal](https://github.com/AIWander/manager-universal).

The two repos are independent — you can install either, both, or neither.

## License

Apache 2.0. See [LICENSE](LICENSE).
