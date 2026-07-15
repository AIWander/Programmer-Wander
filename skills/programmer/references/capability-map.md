# Programmer-Wander capability map

Source audit: `src/tools/mod.rs` at commit `fefb0046edff8b705e314402fee5d5b6ae84c891`. That registry contains 105 unique tool definitions. Run `scripts/validate-capability-map.ps1` after registry changes and treat live `tools/list` as authority for an installed binary.

<!-- TOOL_MAP_START -->
## Files and text (18)

`read_file`, `write_file`, `edit_block`, `copy_file`, `move_file`, `get_file_info`, `create_dir`, `list_dir`, `append_file`, `tail_file`, `file_stats`, `extract_lines`, `grep`, `diff_file`, `smart_read`, `archive_create`, `archive_extract`, `md2docx`

## Shells and sessions (28)

`bash`, `run`, `chain`, `powershell`, `smart_exec`, `session_create`, `session_list`, `session_destroy`, `session_set_env`, `session_get_env`, `session_history`, `session_read_output`, `session_checkpoint`, `session_recover`, `session_cd`, `session_recovery_status`, `session_recover_data`, `session_resume_op`, `session_clear_recovery`, `shortcut`, `list_shortcut`, `shortcut_chain`, `psession_create`, `psession_run`, `psession_destroy`, `psession_list`, `psession_read`, `psession_history`

## Git (12)

`git_status`, `git_diff`, `git_commit`, `git_push`, `git_pull`, `git_log`, `git_branch`, `git_checkout`, `git_stash`, `git_diff_summary`, `git_clone`, `git_remote`

## WSL (4)

`wsl_run`, `wsl_bg`, `wsl_status`, `wsl_log`

## HTTP and webhooks (8)

`http_request`, `http_download`, `http_scrape`, `webhook_start`, `webhook_stop`, `webhook_list`, `webhook_add_route`, `port_check`

## Transformations and search (14)

`search_start`, `search_file`, `transform_bulk_rename`, `transform_sync_dir`, `transform_file`, `transform_json_format`, `transform_json_minify`, `transform_base64_encode`, `transform_base64_decode`, `transform_csv_to_json`, `transform_json_to_csv`, `transform_find_replace`, `transform_hash_file`, `transform_scaffold`

## System and monitoring (14)

`screenshot`, `system_info`, `clipboard_read`, `clipboard_write`, `list_process`, `kill_process`, `config_validate_mcp`, `watch_resource`, `stop_watch`, `get_alert`, `list_watch`, `sqlite_query`, `registry_read`, `notify`

## Security, infrastructure, and planning (7)

`security_check_cmd`, `security_audit_log`, `server_health`, `tool_fallback`, `deploy_preflight`, `plan`, `plan_assemble`
<!-- TOOL_MAP_END -->

## Exclusions

The registry does not expose browser interaction, UIA, OCR, visual clicking, credential storage, Workflow recording or replay, CPC extraction, or multi-agent delegation. Route those abilities to their actual owners.
