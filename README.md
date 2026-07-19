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

| Ability group | Count | Purpose |
|---|---:|---|
| **Files and text** | 18 | Read, edit, search, inspect, archive, and convert local files |
| **Shells and sessions** | 28 | One-shot, persistent, shortcut, and recovery-aware command execution |
| **Git** | 12 | Local and remote repository workflows |
| **WSL** | 4 | Foreground and background Linux execution from Windows |
| **HTTP and webhooks** | 8 | Requests, downloads, scraping, ports, and local callback routes |
| **Transformations and search** | 14 | Bulk, structured-data, encoding, scaffold, and search operations |
| **System and monitoring** | 14 | Processes, watchers, clipboard, registry, SQLite, screenshots, and notifications |
| **Security, infrastructure, and planning** | 7 | Command checks, audits, health, fallback, deployment preflight, and plans |

105 tools in the current source registry, each description tagged with its category (`[Files]`, `[Git]`, `[Guard]`, ...) and listed in a curated workbench order, so tool lists read grouped in any host. No external dependencies. Single static-linked .exe. Works standalone — does not require any other MCP server.

Skills and optional guard hooks ship from the companion repository
[AIWander/aiprogrammer](https://github.com/AIWander/aiprogrammer) — install with
`claude plugin marketplace add AIWander/aiprogrammer`, then pick either the `programmer`
profile (five skills plus inert, reviewable guard-hook templates) or `programmer-skills`
(skills only). The repository root here is additionally its own plugin package (see
"Optional plugin surface" below).

## Safety model

Programmer-Wander is a powerful local dev shell. If your AI host can call it, the AI can read and write files, run shell commands, use git, call WSL, and operate on your clipboard. Install it only for AI clients you trust, and review the tools you enable.

Command-entry tools (`run`, `bash`, `powershell`, `smart_exec`, `chain`, `psession_run`, `wsl_run`, and `wsl_bg`) run through `security_check_cmd` before execution. Critical destructive patterns are blocked and logged; recursive delete-style commands must target an obviously disposable path such as `target/`, `build/`, `tmp/`, or `.cache/`.

AIWander tools are local, user-authorized MCP capability surfaces. They do not grant an AI new permissions by themselves. They expose tools the user explicitly installs and enables. Sensitive actions should be confirmed by the user, credentials should stay in the OS keyring or local vault, and demos should use mock data.

## Install

### Optional plugin surface

The repository root is also a Codex- and Claude-compatible plugin package. It includes two skills with separate jobs:

- `programmer-getting-started` installs, connects, verifies, and troubleshoots the MCP process.
- `programmer` routes active development work across the 105-tool contract by ability.

Install `programmer.exe` first, then add this repository through the local host's plugin flow. The included `.mcp.json` uses `programmer.exe` from `PATH`, so it contains no machine-specific user path. The plugin does not silently modify another AI's hooks or config; see [the host application guide](instructions/APPLY_TO_AI_HOSTS.md) for host-specific guidance.

### Option 1 — Portable (recommended)

1. Download `programmer-wander-windows-x64.zip` (or `arm64`) from [Releases](https://github.com/AIWander/Programmer-Wander/releases/latest)
2. Extract to a stable folder, e.g. `C:\tools\programmer\`
3. Register with your AI host:
   ```powershell
   C:\tools\programmer\programmer.exe install --target claude-desktop
   # or: --target lm-studio, --target cowork, --target claude-code
   # Advanced: --target all registers every detected host.
   ```
4. Restart your AI host

### Option 2 — MSI installer

1. Download `programmer-wander-windows-x64.msi` from [Releases](https://github.com/AIWander/Programmer-Wander/releases/latest)
2. Run the MSI (UAC prompt; click Allow)
3. The MSI installs `programmer.exe` and adds its directory to `PATH`; it does not modify AI-host configuration
4. In a new terminal, run `programmer.exe install --target <host>` for each host you explicitly want to register
5. Restart only the host whose configuration you changed

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
$url = "https://github.com/AIWander/Programmer-Wander/releases/latest/download/programmer-wander-windows-$arch.zip"
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

`programmer` keeps its local session state, file watchers, and recovery checkpoints in `./.programmer/` relative to the exe. This makes the install fully portable — copy the exe + its `./.programmer/` folder to a different machine and your state goes with it.

## Build from source

```bash
git clone https://github.com/AIWander/Programmer-Wander
cd Programmer-Wander
cargo build --release
# Binary at: target/release/programmer.exe
```

Requires Rust 1.75+.

## Upgrade: UniMan ($5)

Programmer-Wander is the **free dev shell** for a single AI, and stays free. The paid
upgrade is **UniMan** — a universal manager that delegates work to Claude Code, Codex
CLI, and Grok Build CLI, plus a live local dashboard that covers both products: it
detects a Programmer-Wander install automatically and shows it alongside your
delegated sessions. Get it at [aiwander.ai](https://aiwander.ai).

The two products are independent — Programmer-Wander works fully on its own.

## License

Apache 2.0. See [LICENSE](LICENSE).
