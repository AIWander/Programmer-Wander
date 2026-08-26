# Programmer-Wander capability map

Source authority: the current `src/tools/mod.rs` registry. Every supported build exposes exactly 49 unique tool definitions. Run `scripts/validate-capability-map.ps1` after registry changes and treat live `tools/list` as authority for an installed binary.

<!-- TOOL_MAP_START -->
## Files (7)

`read_file`, `write_file`, `edit_block`, `copy_file`, `move_file`, `create_dir`, `list_dir`

## Shell (5)

`cmd`, `powershell`, `shortcut`, `list_process`, `kill_process`

## Sessions (2)

`shell_session`, `live_shell`

## Search (1)

`search_file`

## System (5)

`screenshot`, `system_info`, `clipboard_read`, `clipboard_write`, `md2docx`

## WSL (4)

`wsl_run`, `wsl_bg`, `wsl_status`, `wsl_log`

## Transforms and stats (14)

`archive_create`, `archive_extract`, `transform_bulk_rename`, `transform_sync_dir`, `diff_file`, `transform_file`, `base64`, `json`, `convert`, `transform_find_replace`, `transform_hash_file`, `transform_scaffold`, `grep`, `file_stats`

## Network (2)

`http_request`, `port_check`

## Guard (2)

`security_check_cmd`, `security_audit_log`

## Infrastructure and meta (7)

`server_health`, `deploy_preflight`, `config_validate_mcp`, `plan`, `sqlite_query`, `registry_read`, `doctor`
<!-- TOOL_MAP_END -->

## Exclusions

The registry does not expose browser interaction, UIA, OCR, visual clicking, credential storage, Git-specific APIs, watchers, webhooks, Workflow recording or replay, CPC extraction, or multi-agent delegation. Route those abilities to their actual owners. Ordinary Git commands remain available through `cmd`.
