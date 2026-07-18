---
name: programmer-toolmap
description: Full organized reference for all Programmer-Wander tools by category - Files, Search, Shell, Git, Sessions, Background, Net, Data, System, Guard, Plan. Surface when choosing which programmer tool fits a task, when a tool call failed and an alternative is needed, or when the user asks what programmer can do.
---

# Programmer Toolmap

Every tool, grouped exactly as the server presents them. Tool names are stable;
descriptions carry the `[Category]` tag.

## [Files]

| Tool | Use |
|---|---|
| `read_file` | Read with pattern search, line ranges, auto-truncation |
| `write_file` | Write/overwrite (creates parent dirs); NOT for surgical edits |
| `append_file` | Append content |
| `edit_block` | Atomic old-string/new-string replacement - the code-surgery tool |
| `copy_file` / `move_file` | Copy with metadata / move or rename |
| `create_dir` / `list_dir` | Make dirs / list contents |
| `get_file_info` / `file_stats` | Metadata / line-word-char counts |
| `diff_file` | Diff two files |
| `tail_file` | Last N lines + byte offset; pass since_bytes for delta polling |
| `extract_lines` | Pull specific line ranges |

## [Search]

| Tool | Use |
|---|---|
| `grep` | Regex search in files, native Rust speed |
| `search_file` | Find files by name pattern |
| `search_start` | Larger async search |
| `smart_read` | Read with automatic strategy for big files |

## [Shell]

| Tool | Use |
|---|---|
| `bash` | Git Bash backend - cargo, jq, here-docs, multi-line commits |
| `powershell` | Windows cmdlets, CIM, ACLs, registry-adjacent work |
| `run` | Simple one-off command |
| `smart_exec` | Run with auto-retry from known error patterns |
| `chain` | Multi-command sequence, stops on error |
| `shortcut` / `list_shortcut` / `shortcut_chain` | Saved command shortcuts |

## [Git]

`git_status`, `git_diff`, `git_diff_summary`, `git_log`, `git_branch`,
`git_checkout`, `git_commit`, `git_stash`, `git_clone`, `git_remote`,
`git_pull`, `git_push` - full local + network git.

## [Sessions]

| Tool | Use |
|---|---|
| `psession_create/run/read/history/list/destroy` | Persistent shell - variables, CWD, state survive across calls |
| `session_create/list/destroy` | Tracked session lifecycle |
| `session_set_env/get_env/cd` | Session environment |
| `session_history/read_output` | What ran, what it printed |
| `session_checkpoint/recover` | Checkpoint and restore |
| `session_recovery_status/recover_data/resume_op/clear_recovery` | Crash recovery |

## [Background]

| Tool | Use |
|---|---|
| `watch_resource` / `list_watch` / `stop_watch` / `get_alert` | Filesystem watchers + fired events |
| `webhook_start/stop/list/add_route` | Local HTTP callback routes (OAuth, CI) |
| `wsl_run` | One-shot Linux command |
| `wsl_bg` / `wsl_status` / `wsl_log` | Long Linux jobs with log capture |

## [Net]

`http_request` (full verbs/headers), `http_download`, `http_scrape`
(HTML-to-text), `port_check` (TCP connectivity + timing).

## [Data]

| Tool | Use |
|---|---|
| `transform_json_format/json_minify` | Pretty/minify JSON |
| `transform_csv_to_json/json_to_csv` | Tabular conversion |
| `transform_base64_encode/decode` | Base64 |
| `transform_hash_file` | File hashes |
| `transform_find_replace` | Bulk find/replace across files |
| `transform_bulk_rename` | Pattern renames |
| `transform_sync_dir` | Directory sync |
| `transform_scaffold` | Project scaffolding |
| `transform_file` | General file transform |
| `archive_create` / `archive_extract` | zip/tar |
| `sqlite_query` | Read-only SQL on a SQLite file |
| `md2docx` | Markdown to DOCX via pandoc |

## [System]

`screenshot` (single PNG, no UI driving), `system_info`, `clipboard_read`,
`clipboard_write`, `list_process`, `kill_process` (see safe-ops before using),
`registry_read` (approved HKLM/HKCU locations, read-only), `notify` (toast),
`server_health` (which MCP servers are alive).

## [Guard]

| Tool | Use |
|---|---|
| `security_check_cmd` | Pre-flight a command for dangerous patterns |
| `security_audit_log` | Recent security decisions |
| `deploy_preflight` | Pre-deploy checks: sources exist, servers running |
| `tool_fallback` | Fallback tool when primary is down |
| `config_validate_mcp` | Validate MCP host config JSON |

## [Plan]

`plan` (task ingredients: tools needed, dependencies, breadcrumb-worthiness),
`plan_assemble` (enrich with cross-server requirements).

## Pick-this-not-that

- Edit code: `edit_block`, not `write_file`.
- cargo/git/jq/pipelines: `bash`, not `powershell`.
- Long-running anything: `psession_*` or `wsl_bg`, not a blocking `run`.
- Poll a growing log: `tail_file` with `since_bytes`, not repeated `read_file`.
- Kill a process: check it is not a running MCP server first (`server_health`).
