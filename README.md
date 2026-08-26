# Programmer-Wander

> An MCP server that gives any AI a complete Rust + Windows dev shell.

**Status:** free, public, and release-candidate. Built for [Claude Desktop](https://claude.ai/download), [LM Studio](https://lmstudio.ai), [Claude Code](https://claude.ai/code), and local hosts that speak MCP.

**Part of [CPC](https://github.com/AIWander) (Copy Paste Compute).** Programmer-Wander is
one member of the free core trio alongside AI-Hands and Voice-Command.

## Optional hands-free add-on

[Voice-Command v3.0.0](https://github.com/AIWander/Voice-Command/releases/tag/v3.0.0)
is the separate, free headset companion for people who want to speak while Programmer
does local work. Programmer does not require Voice. Voice requires the user's own
microphone permission and local setup; using either product from a web or mobile AI
also requires an authenticated remote connector to the Windows host.

[![Build](https://github.com/AIWander/Programmer-Wander/actions/workflows/build.yml/badge.svg)](https://github.com/AIWander/Programmer-Wander/actions)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows%20x64%20%7C%20ARM64-blue.svg)](https://github.com/AIWander/Programmer-Wander/releases)

## What it does

`programmer` is a single-binary MCP server that exposes a Rust developer's toolbox to any AI:

| Ability group | Count | Purpose |
|---|---:|---|
| **Files** | 7 | Read, write, edit, copy, move, create, and list local files |
| **Shell** | 5 | Command Prompt, PowerShell, shortcuts, and process control |
| **Sessions** | 2 | Remembered command context and a real long-lived shell |
| **Search** | 1 | Search files by name or content |
| **System** | 5 | Screenshots, system info, clipboard, and Markdown conversion |
| **WSL** | 4 | Foreground and background Linux execution from Windows |
| **Transforms and stats** | 14 | Archives, structured data, encoding, diffs, scaffolds, and bulk changes |
| **Network** | 2 | HTTP requests and port checks |
| **Guard** | 2 | Command safety evaluation and privacy-safe audit records |
| **Infra and meta** | 7 | Process health, preflight, config validation, planning, SQLite, registry, and doctor |

Every supported build exposes exactly 49 unique tools. The same names appear in `tools/list` and the dispatch surface: `cmd` is present, and no feature flag adds hidden tools. Programmer is one executable and does not require another MCP server; individual abilities such as WSL or Markdown conversion still require the corresponding OS program.

Skills and optional guard hooks ship from the companion repository
[AIWander/aiprogrammer](https://github.com/AIWander/aiprogrammer) — install with
`claude plugin marketplace add AIWander/aiprogrammer`, then pick either the `programmer`
profile (skills plus inert, reviewable guard-hook templates) or `programmer-skills`
(skills only). The repository root here is additionally its own plugin package (see
"Optional plugin surface" below).

## Safety model

Programmer-Wander is a powerful local dev shell. If your AI host can call it, the AI can read and write files, run commands, call WSL, and operate on your clipboard. Install it only for AI clients you trust, and review the tools you enable.

Command-entry tools (`cmd`, `powershell`, `shortcut`, `shell_session`, `live_shell`, `wsl_run`, and `wsl_bg`) enforce the same target-aware safety policy used by `security_check_cmd`. Critical destructive patterns are blocked and logged without recording the command body; every recursive-delete target must stay inside an explicitly disposable directory such as `target/`, `build/`, `tmp/`, or `.cache/`.

AIWander tools are local, user-authorized MCP capability surfaces. They do not grant an AI new permissions by themselves. They expose tools the user explicitly installs and enables. Sensitive actions should be confirmed by the user, credentials should stay in the OS keyring or local vault, and demos should use mock data.

## Install

### Optional plugin surface

The repository root is also a Codex- and Claude-compatible plugin package. It includes two skills with separate jobs:

- `programmer-getting-started` installs, connects, verifies, and troubleshoots the MCP process.
- `programmer` routes active development work across the exact 49-tool contract by ability.

Install `programmer.exe` first, then add this repository through the local host's plugin flow. The included `.mcp.json` uses `programmer.exe` from `PATH`, so it contains no machine-specific user path. The plugin does not silently modify another AI's hooks or config; see [the host application guide](instructions/APPLY_TO_AI_HOSTS.md) for host-specific guidance.

### Option 1 — Signed portable release candidate (recommended)

The v2 release candidate ships as Authenticode-signed portable ZIPs. It does not
claim installer acceptance yet. Use the exact [v2.0.0-rc.1 prerelease](https://github.com/AIWander/Programmer-Wander/releases/tag/v2.0.0-rc.1), not the older `latest` alpha release.

1. Download `programmer-wander-v2.0.0-rc.1-windows-x64.zip` (or `arm64`) from that prerelease
2. Extract to a stable folder, e.g. `C:\tools\programmer\`
3. Register with your AI host:
   ```powershell
   C:\tools\programmer\programmer.exe install --target claude-desktop
   # or: --target lm-studio, --target claude-code
   # Advanced: --target all registers every detected host.
   ```
4. Restart your AI host

Verify any download against `SHA256SUMS` on the release.

### Option 2 — Have your AI install it for you

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

### 2. Download the exact release candidate (portable)

```powershell
$version = "v2.0.0-rc.1"
$url = "https://github.com/AIWander/Programmer-Wander/releases/download/$version/programmer-wander-$version-windows-$arch.zip"
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

`programmer` keeps local session state, logs, screenshots, and recovery checkpoints in `.programmer/` beside the executable. Set `PROGRAMMER_STATE_DIR` to use a different directory. If the executable directory cannot be resolved, Programmer safely falls back to `.programmer/` under the current directory.

## Build from source

```console
git clone https://github.com/AIWander/Programmer-Wander
cd Programmer-Wander
cargo build --release
# Binary at: target/release/programmer.exe
```

Requires Rust 1.85+.

## Upgrade: UniMan ($5)

Programmer-Wander is the **free dev shell** for a single AI, and stays free. The paid
upgrade is **UniMan** — a universal manager that delegates work to Claude Code, Codex
CLI, and Grok Build CLI, plus a live local dashboard that covers both products: it
detects a Programmer-Wander install automatically and shows it alongside your
delegated sessions. Get it at [aiwander.ai](https://aiwander.ai).

The two products are independent — Programmer-Wander works fully on its own.

## License

Apache 2.0. See [LICENSE](LICENSE).
