# Programmer for Claude Desktop

Programmer is a Claude Desktop extension that lets Claude work on your Windows computer the
way a developer would: read and edit files, run commands in Command Prompt and PowerShell, keep
a shell session open across several steps, run Linux commands through WSL, search code, and
check ports and processes. It is one small program that runs on your computer only while Claude
Desktop is open.

Publisher: AIWander. Source: https://github.com/AIWander/Programmer-Wander (Apache-2.0).
Version: 2.0.0-rc.1 (release candidate).

**Unsigned build.** Until AIWander signs a release, these files carry no publisher signature.
Windows and Claude Desktop will say so when you install. Check the SHA-256 hash (below) before
installing an unsigned file.

## Before you install: what it can do

Programmer runs as you, the signed-in Windows user. Claude can use it to read, change, delete
and run anything your Windows account can reach, on any drive. This release has **no setting
to limit it to particular folders**. It has a built-in safety policy that blocks some clearly
destructive commands (Claude can show the verdict with `security_check_cmd`), but that policy
is a safety net, not a sandbox. Claude Desktop asks you before each tool call unless you choose
to allow it; read commands that delete, move or overwrite before you approve them.

## Requirements

- Windows 10 or Windows 11.
- Claude Desktop for Windows, version 0.10.0 or newer.
- The bundle that matches your processor:
  - `programmer-2.0.0-rc.1-win-arm64.mcpb` for Windows on ARM (for example Snapdragon laptops).
  - `programmer-2.0.0-rc.1-win-x64.mcpb` for Intel and AMD processors.

  To check, open PowerShell and run `$env:PROCESSOR_ARCHITECTURE`. It prints `ARM64` or `AMD64`
  (AMD64 means the x64 bundle).
- Optional: WSL for the `wsl_*` tools, pandoc for `md2docx`, Python for `transform_file`, git if
  you want Claude to run git commands. Programmer works without them; those tools then report
  what is missing.

## Install

1. Download the `.mcpb` file for your processor and `SHA256SUMS.txt` from the release page.
2. Optional but recommended for unsigned builds: in PowerShell, run
   `Get-FileHash .\programmer-2.0.0-rc.1-win-arm64.mcpb -Algorithm SHA256` (use your file name)
   and check that the hash matches the line in `SHA256SUMS.txt`.
3. Install it in any one of these ways:
   - double-click the `.mcpb` file, or
   - drag it into the Claude Desktop window, or
   - in Claude Desktop open **Settings > Extensions > Advanced settings > Install Extension...**
     and choose the file.
4. Claude Desktop shows the extension's name, publisher and tool list. Click **Install**.

If Windows SmartScreen warns about the download, that is because the file is unsigned. Choose
**More info > Keep** (browser) only if the hash matched in step 2.

## First-run check

Start a new chat and ask:

- "What Programmer tools do you have?" Claude should list tools such as `read_file`,
  `edit_block`, `cmd`, `powershell`, `shell_session` and `doctor` (49 in total).
- "Run the Programmer doctor tool." It reports what this computer has: git, WSL, shells,
  Python, pandoc, cargo and where Programmer keeps its state.

If the tools do not appear, see Troubleshooting.

## Tools (49)

| Tool | What it does |
|---|---|
| read_file | Read a file: plain, pattern search, line range, or tail with delta polling |
| write_file | Write or overwrite a file (creates parent folders); mode=append adds to the end |
| edit_block | Exact-match replacement that fails unless the match count is as expected |
| copy_file | Copy a file, keeping metadata |
| move_file | Move or rename a file |
| create_dir | Create a folder, including parents |
| list_dir | List folder contents |
| cmd | Run a Command Prompt command and return output and exit code |
| powershell | Run a PowerShell command and return output |
| kill_process | Stop a process by PID |
| list_process | List running processes |
| shortcut | List saved shortcuts or run saved or raw command sequences in a session |
| shell_session | Shell session that remembers the folder and environment across commands |
| live_shell | Long-lived PowerShell or WSL process for REPLs and in-memory state |
| search_file | Find files by name or content |
| screenshot | Take a screenshot; returns the file path and metadata only |
| system_info | OS, CPU, memory and disk information |
| clipboard_read | Read the clipboard |
| clipboard_write | Write the clipboard |
| md2docx | Convert Markdown to Word (needs pandoc) |
| wsl_run | Run a command in WSL and wait for it |
| wsl_bg | Start a WSL background job |
| wsl_status | Check WSL background jobs |
| wsl_log | Read a WSL background job's log |
| archive_create | Create zip, tar, tar.gz or tar.bz2 archives |
| archive_extract | Extract an archive (format detected automatically) |
| transform_bulk_rename | Rename many files with a regular expression |
| transform_sync_dir | Sync folders (mirror, update, backup) |
| diff_file | Show the differences between two files |
| transform_file | Apply a Python transform to matching files (needs Python) |
| base64 | Base64 encode or decode |
| json | Pretty-print or minify JSON |
| convert | CSV to JSON or JSON to CSV |
| transform_find_replace | Find and replace across many files (no count guard) |
| transform_hash_file | MD5 or SHA-256 of a file |
| transform_scaffold | Generate project scaffolding |
| grep | Search files for a pattern with context lines |
| file_stats | File and folder sizes and dates without reading content |
| http_request | Make an HTTP request; save= downloads to disk with resume |
| port_check | Test whether a host and port accept connections |
| security_check_cmd | Show the safety-policy verdict for a command |
| security_audit_log | Show recent safety-policy decisions |
| server_health | Check running processes by name |
| deploy_preflight | Pre-deploy checks for a project folder |
| config_validate_mcp | Check an MCP configuration file |
| plan | Break a task into tools, dependencies and tracking advice |
| sqlite_query | Read-only SQL against a SQLite file |
| registry_read | Read approved Windows registry locations |
| doctor | Report what this computer has and where Programmer keeps state |

The companion skills (`programmer-*.zip` on the release page) teach Claude when to use which
tool. Upload them in Claude under **Settings > Capabilities > Skills**; they are optional.

## Privacy Policy

Programmer collects nothing and sends nothing on its own. It has no account, no analytics and
no telemetry, and it runs no background service: it starts when Claude Desktop starts it and
exits when Desktop stops it. What it reads or runs goes back only to Claude through Claude
Desktop, where it becomes part of your conversation under Anthropic's terms. It contacts the
network only when Claude calls `http_request` or `port_check`, and only the address in that
request. Programmer keeps its own working state (a safety audit log of checked commands, shell
session settings, screenshots you ask for, WSL job logs) in a `.programmer` folder inside the
extension's install folder. That state stays on your computer, is never shared, and is deleted
when you uninstall or update the extension. The full policy is in [PRIVACY.md](PRIVACY.md).
Questions: open an issue at https://github.com/AIWander/Programmer-Wander/issues or email
contact@aiwander.ai.

## Uninstall

In Claude Desktop open **Settings > Extensions**, choose **Programmer (AIWander)** and click
**Uninstall**. That removes the program and its `.programmer` state folder.

What stays behind:

- Anything Claude created or changed on your computer while using the tools. Uninstalling
  does not undo file edits or commands.
- Claude Desktop's own log of the extension's output, under `%APPDATA%\Claude\logs\`, which
  Claude Desktop manages.
- Uploaded skills stay in your Claude account until you delete them under
  **Settings > Capabilities > Skills**.

## Troubleshooting

- **The tools do not appear.** Open **Settings > Extensions** and check that Programmer is
  enabled. Then start a new chat. If it shows an error, open the extension's log from that
  page (or look in `%APPDATA%\Claude\logs\` for a file whose name contains `Programmer`).
- **"This app can't run on your PC" or the server exits at once.** You installed the bundle
  for the other processor type. Uninstall it and install the one that matches
  `$env:PROCESSOR_ARCHITECTURE`.
- **Two copies of every tool.** Programmer is also configured another way, for example by
  `programmer.exe install --target claude-desktop` or a suite installer. Remove the
  `programmer` entry from `%APPDATA%\Claude\claude_desktop_config.json`, or uninstall the
  extension, so only one remains.
- **SmartScreen or "unknown publisher".** Expected for unsigned builds. Verify the hash first.
- **A command was blocked.** The safety policy rejected it. Ask Claude to run
  `security_check_cmd` on the command to see why.

## Known limits

- No folder restriction: the tools can reach anything your account can.
- The tools do not declare MCP read-only or destructive hints yet, so Claude Desktop cannot
  group them that way.
- State in `.programmer` is reset when the extension is updated.
- Release candidate: 2.0.0-rc.1. Unsigned until a signed release is published.
- Windows only. A macOS counterpart is a separate project (Papple).

## Support

Bugs and questions: https://github.com/AIWander/Programmer-Wander/issues.
Security issues: follow SECURITY.md in the repository (private report, or contact@aiwander.ai).

## Building from source

From a clone of the repository, on Windows with Rust, Node.js 20+ and the mcpb CLI
(`npm install -g @anthropic-ai/mcpb`):

```powershell
pwsh -NoProfile -File mcpb\pack.ps1 -Clean          # host architecture
pwsh -NoProfile -File mcpb\pack.ps1 -Clean -X64     # host plus x64
```

Output: `mcpb\dist\*.mcpb` and `mcpb\dist\SHA256SUMS.txt`; build report in
`mcpb\build\pack-report.json`. Signing is described in [SIGNING.md](SIGNING.md).
