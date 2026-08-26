//! Tool Registry and Dispatch
//!
//! 2026-07-29 rebuild: the hidden term_* legacy alias layer was retired, name-duplicates
//! removed, ratified merges applied, sessions collapsed to shell_session + live_shell,
//! and relocations executed (watch/webhook -> autonomous pulse; git -> git add-on server;
//! http_scrape -> Hands' domain; notify -> local/ops). Decision record:
//! Inbox/tool-usage-reports-20260727/SCRATCHPAD_programmer_walkthrough.md and
//! RELOCATIONS_programmer_rebuild_20260729.md.
//!
//! Census contract: exactly 49 advertised tools. Every name in execute_tool appears in
//! get_tool_definitions and vice versa - there is no hidden dispatch surface.

mod config;
mod file;
mod http;
mod infra;
mod planner;
mod psession;
mod registry;
mod runtime;
mod search;
mod security;
mod shell;
mod sqlite;
mod system;
mod transform;
mod wsl;

use anyhow::Result;
use serde_json::{json, Value};

/// Get all tool definitions for MCP tools/list
pub fn get_tool_definitions() -> Vec<Value> {
    vec![
        // ============ FILE OPERATIONS (7) ============
        json!({
            "name": "read_file",
            "description": "Read file with smart options: search for pattern, range of lines, tail with delta polling, or auto-truncate large files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"},
                    "search": {"type": "string", "description": "Grep for pattern, return matching lines"},
                    "range": {"type": "string", "description": "Line range like 50:100 (legacy alias: lines)"},
                    "tail": {"type": "integer", "description": "Return last N lines plus byte_offset"},
                    "since_bytes": {"type": "integer", "description": "Byte offset from a previous tail call - returns only NEW content (delta polling)"},
                    "max_kb": {"type": "integer", "description": "Max KB to return", "default": 100}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "write_file",
            "description": "Write content to file. Creates parent directories if needed. mode=append streams to the end of the file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Target file path"},
                    "content": {"type": "string", "description": "Content to write"},
                    "mode": {"type": "string", "description": "rewrite or append", "default": "rewrite"}
                },
                "required": ["path", "content"]
            }
        }),
        json!({
            "name": "edit_block",
            "description": "Guarded single-file text replacement: exact literal match, fails unless the occurrence count equals expected_replacements. Use transform_find_replace for bulk/regex sweeps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to file"},
                    "old_string": {"type": "string", "description": "Text to find"},
                    "new_string": {"type": "string", "description": "Replacement text"},
                    "expected_replacements": {"type": "integer", "description": "Expected count", "default": 1}
                },
                "required": ["file_path", "old_string", "new_string"]
            }
        }),
        json!({
            "name": "copy_file",
            "description": "Copy file with metadata preservation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string"},
                    "destination": {"type": "string"}
                },
                "required": ["source", "destination"]
            }
        }),
        json!({
            "name": "move_file",
            "description": "Move or rename file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string"},
                    "destination": {"type": "string"}
                },
                "required": ["source", "destination"]
            }
        }),
        json!({
            "name": "create_dir",
            "description": "Create directory recursively.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "list_dir",
            "description": "List directory contents recursively.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "depth": {"type": "integer", "default": 2},
                    "sort_by": {"type": "string", "default": "name"}
                },
                "required": ["path"]
            }
        }),
        // ============ SHELL EXECUTION (5) ============
        json!({
            "name": "cmd",
            "description": "Execute a Windows Command Prompt command through cmd.exe /C and return output with exit code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command Prompt command to execute"},
                    "timeout": {"type": "integer", "description": "Timeout in seconds", "default": 30}
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "powershell",
            "description": "Execute PowerShell command. Most versatile single tool for Windows.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "PowerShell command to execute"},
                    "timeout_secs": {"type": "integer", "description": "Timeout in seconds (default: 30)", "default": 30}
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "kill_process",
            "description": "Kill process by PID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "pid": {"type": "integer"}
                },
                "required": ["pid"]
            }
        }),
        json!({
            "name": "list_process",
            "description": "List running processes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "filter_name": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "shortcut",
            "description": "Run saved shortcuts and/or raw command sequences in a session. mode=list shows saved shortcuts; mode=run executes names=[] (saved, with params substitution) and/or commands=[] (raw), stopping on first error by default.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mode": {"type": "string", "description": "run or list", "default": "run"},
                    "names": {"type": "array", "items": {"type": "string"}, "description": "Saved shortcut names to run in order"},
                    "commands": {"type": "array", "items": {"type": "string"}, "description": "Raw commands to run in order (after any names)"},
                    "params": {"type": "object", "description": "Substitutions for $placeholders in saved shortcuts"},
                    "session_id": {"type": "string", "description": "shell_session to run inside", "default": "default"},
                    "stop_on_error": {"type": "boolean", "default": true}
                }
            }
        }),
        // ============ SESSIONS (2) ============
        json!({
            "name": "shell_session",
            "description": "State session: remembered cwd + env applied to each fresh command. Actions: create, run, list, destroy, cd, env (set/get), history, read (recent outputs). State auto-persists to disk on every change - survives crashes and restarts with no checkpoint ceremony. For an interactive live process (REPL, in-memory shell variables), use live_shell instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "create | run | list | destroy | cd | env | history | read"},
                    "session_id": {"type": "string", "description": "Session id (default: default)"},
                    "name": {"type": "string", "description": "create: session name"},
                    "cwd": {"type": "string", "description": "create: working directory"},
                    "command": {"type": "string", "description": "run: command to execute"},
                    "timeout": {"type": "integer", "description": "run: timeout seconds", "default": 30},
                    "path": {"type": "string", "description": "cd: directory to change to"},
                    "key": {"type": "string", "description": "env: variable name (omit to get all)"},
                    "value": {"type": "string", "description": "env: value to set (omit to get)"},
                    "limit": {"type": "integer", "description": "history: entries to return", "default": 10},
                    "last": {"type": "integer", "description": "read: recent outputs to return", "default": 5}
                },
                "required": ["action"]
            }
        }),
        json!({
            "name": "live_shell",
            "description": "Real long-lived shell process (PowerShell or WSL) you can talk to: holds a REPL, keeps in-memory variables, incremental reads. Actions: create, run, read, history, list, destroy, checkpoint (records backend/cwd/env/history to disk - enough to recreate an equivalent process), recover (respawns from a checkpoint). For cheap remembered-cwd/env command running, use shell_session instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "create | run | read | history | list | destroy | checkpoint | recover"},
                    "session_id": {"type": "string", "description": "Session id from create"},
                    "name": {"type": "string", "description": "create: session name (default: default)"},
                    "shell": {"type": "string", "description": "create: powershell (default) or wsl"},
                    "cwd": {"type": "string", "description": "create: working directory"},
                    "distro": {"type": "string", "description": "create: WSL distro (default: Ubuntu-24.04)"},
                    "command": {"type": "string", "description": "run: command to execute"},
                    "timeout_secs": {"type": "integer", "description": "run: timeout seconds", "default": 30},
                    "tail": {"type": "integer", "description": "read: lines to return", "default": 20},
                    "checkpoint_path": {"type": "string", "description": "checkpoint/recover: file path (default: PROGRAMMER_STATE_DIR/live_shells/<id>.checkpoint.json)"}
                },
                "required": ["action"]
            }
        }),
        // ============ SEARCH (1) ============
        json!({
            "name": "search_file",
            "description": "Search for files by name or content.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "pattern": {"type": "string"},
                    "search_type": {"type": "string", "default": "files"},
                    "file_pattern": {"type": "string"},
                    "ignore_case": {"type": "boolean", "default": true},
                    "max_results": {"type": "integer"}
                },
                "required": ["path", "pattern"]
            }
        }),
        // ============ SYSTEM (5) ============
        json!({
            "name": "screenshot",
            "description": "Take a screenshot for troubleshooting. Returns file path + metadata only (no raw bytes). Capped at 1MB - lower quality/scale if exceeded. Default: quality=60, scale=0.75.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "save_path": {"type": "string", "description": "Output path (default: PROGRAMMER_STATE_DIR/screenshots/screenshot_<ts>.jpg)"},
                    "quality": {"type": "integer", "description": "JPEG quality 1-100 (default 60)"},
                    "scale": {"type": "number", "description": "Scale factor 0.1-1.0 (default 0.75)"}
                }
            }
        }),
        json!({
            "name": "system_info",
            "description": "Get system info: OS, CPU, memory, disk.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "clipboard_read",
            "description": "Read from clipboard.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "clipboard_write",
            "description": "Write to clipboard.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "content": {"type": "string"}
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "md2docx",
            "description": "Convert Markdown file to DOCX via pandoc.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input": {"type": "string", "description": ".md file path"},
                    "output": {"type": "string", "description": ".docx output path"}
                },
                "required": ["input", "output"]
            }
        }),
        // ============ WSL (4) ============
        json!({"name": "wsl_run", "description": "Run command in WSL. Returns output summary + log path.", "inputSchema": {"type": "object", "properties": {"command": {"type": "string", "description": "Command to run in WSL"}, "timeout_secs": {"type": "integer", "description": "Timeout (default: 120)", "default": 120}}, "required": ["command"]}}),
        json!({"name": "wsl_bg", "description": "Launch WSL background job. Returns job_id. Poll with wsl_status.", "inputSchema": {"type": "object", "properties": {"command": {"type": "string", "description": "Command to run in background"}, "job_name": {"type": "string", "description": "Optional friendly name"}}, "required": ["command"]}}),
        json!({"name": "wsl_status", "description": "Check WSL background job status. Use job_id=all to list all.", "inputSchema": {"type": "object", "properties": {"job_id": {"type": "string", "description": "Job ID or all"}, "tail": {"type": "integer", "description": "Log lines to return (default: 10)", "default": 10}}, "required": ["job_id"]}}),
        json!({"name": "wsl_log", "description": "Get full or partial log from a WSL background job.", "inputSchema": {"type": "object", "properties": {"job_id": {"type": "string", "description": "Job ID"}, "lines": {"type": "string", "description": "Range like 1:50 or last:20 (default: last:50)"}}, "required": ["job_id"]}}),
        // ============ TRANSFORMS (13) + FILE STATS (1) ============
        json!({
            "name": "archive_create",
            "description": "Create archive (zip, tar, tar.gz, tar.bz2).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "output": {"type": "string"},
                    "format": {"type": "string", "default": "zip"}
                },
                "required": ["paths", "output"]
            }
        }),
        json!({
            "name": "archive_extract",
            "description": "Extract archive (auto-detect format).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "archive_path": {"type": "string"},
                    "destination": {"type": "string"}
                },
                "required": ["archive_path"]
            }
        }),
        json!({
            "name": "transform_bulk_rename",
            "description": "Regex-based batch rename.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "directory": {"type": "string"},
                    "pattern": {"type": "string"},
                    "replacement": {"type": "string"},
                    "dry_run": {"type": "boolean", "default": true}
                },
                "required": ["directory", "pattern", "replacement"]
            }
        }),
        json!({
            "name": "transform_sync_dir",
            "description": "Sync directories with modes: mirror, update, backup.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": {"type": "string"},
                    "destination": {"type": "string"},
                    "mode": {"type": "string", "default": "update"},
                    "dry_run": {"type": "boolean", "default": true},
                    "exclude": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["source", "destination"]
            }
        }),
        json!({
            "name": "diff_file",
            "description": "Create unified diff between two files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path1": {"type": "string"},
                    "path2": {"type": "string"},
                    "context_lines": {"type": "integer", "default": 3}
                },
                "required": ["path1", "path2"]
            }
        }),
        json!({
            "name": "transform_file",
            "description": "Apply Python transform to matching files (requires python on PATH).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "directory": {"type": "string", "default": "."},
                    "pattern": {"type": "string"},
                    "transform_code": {"type": "string"},
                    "dry_run": {"type": "boolean", "default": true}
                },
                "required": ["pattern", "transform_code"]
            }
        }),
        json!({
            "name": "base64",
            "description": "Base64 encode or decode a string.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mode": {"type": "string", "description": "encode or decode", "default": "encode"},
                    "input": {"type": "string", "description": "Text to encode / base64 to decode"}
                },
                "required": ["input"]
            }
        }),
        json!({
            "name": "json",
            "description": "Format (pretty-print) or minify JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "mode": {"type": "string", "description": "format or minify", "default": "format"},
                    "input": {"type": "string", "description": "JSON string"},
                    "indent": {"type": "integer", "description": "Spaces (default: 2)"}
                },
                "required": ["input"]
            }
        }),
        json!({
            "name": "convert",
            "description": "Convert tabular data between formats: csv to json (first row = headers) or json to csv.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string", "description": "csv or json"},
                    "to": {"type": "string", "description": "json or csv"},
                    "input": {"type": "string", "description": "Data to convert"},
                    "delimiter": {"type": "string", "description": "Delimiter (default: comma)"}
                },
                "required": ["from", "to", "input"]
            }
        }),
        json!({
            "name": "transform_find_replace",
            "description": "Bulk find/replace across file(s): multi-file, regex-capable, no count guard. For a surgical single-file edit with an occurrence-count guard, use edit_block.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File or directory path"},
                    "find": {"type": "string", "description": "Text or regex to find"},
                    "replace": {"type": "string", "description": "Replacement text"},
                    "regex": {"type": "boolean", "description": "Use regex (default: false)"},
                    "recursive": {"type": "boolean", "description": "Search subdirs (default: false)"}
                },
                "required": ["path", "find", "replace"]
            }
        }),
        json!({
            "name": "transform_hash_file",
            "description": "Compute file checksum (MD5, SHA256).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path"},
                    "algorithm": {"type": "string", "description": "md5 or sha256 (default: sha256)"}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "transform_scaffold",
            "description": "Generate project scaffolding. Creates boilerplate structure.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "template": {"type": "string", "description": "Template: rust-mcp, python-mcp, nextjs, fastapi, expo"},
                    "name": {"type": "string", "description": "Project name"},
                    "output_dir": {"type": "string", "description": "Output directory (default: current)"}
                },
                "required": ["template", "name"]
            }
        }),
        json!({
            "name": "grep",
            "description": "Search files for pattern, return matching lines with context.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File or directory"},
                    "pattern": {"type": "string", "description": "Search pattern (regex)"},
                    "context": {"type": "integer", "description": "Lines of context (default: 0)"},
                    "recursive": {"type": "boolean", "description": "Search subdirs (default: false)"}
                },
                "required": ["path", "pattern"]
            }
        }),
        json!({
            "name": "file_stats",
            "description": "File/directory stats without reading content: node metadata (size, timestamps, readonly, is_dir) plus recursive directory aggregation (file/dir counts, total size).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to analyze"},
                    "recursive": {"type": "boolean", "description": "Include subdirs (default: false)"}
                },
                "required": ["path"]
            }
        }),
        // ============ NETWORK (2) ============
        json!({
            "name": "http_request",
            "description": "Make HTTP request. With save=<path>, downloads the body to disk (resumes partial downloads via Range unless resume=false).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "method": {"type": "string", "default": "GET"},
                    "headers": {"type": "object"},
                    "body": {"type": "string"},
                    "timeout": {"type": "integer", "default": 30},
                    "save": {"type": "string", "description": "File path to save the response body to (download mode)"},
                    "resume": {"type": "boolean", "description": "save mode: resume partial download via Range", "default": true}
                },
                "required": ["url"]
            }
        }),
        json!({
            "name": "port_check",
            "description": "Test TCP connectivity to a host:port. Returns whether the port is open and connection time.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "host": {"type": "string", "description": "Host to connect to (default: 127.0.0.1)", "default": "127.0.0.1"},
                    "port": {"type": "integer", "description": "Port number"},
                    "timeout_ms": {"type": "integer", "description": "Connection timeout in ms (default: 2000)", "default": 2000}
                },
                "required": ["port"]
            }
        }),
        // ============ SECURITY (2) ============
        json!({"name": "security_check_cmd", "description": "Evaluate a command with the same target-aware policy enforced by command-running tools.", "inputSchema": {"type": "object", "properties": {"command": {"type": "string", "description": "Command to check"}}, "required": ["command"]}}),
        json!({"name": "security_audit_log", "description": "View recent security audit log entries.", "inputSchema": {"type": "object", "properties": {"lines": {"type": "integer", "description": "Number of recent entries (default: 20)", "default": 20}}}}),
        // ============ INFRA (3) ============
        json!({"name": "server_health", "description": "Check running processes by server name without assuming an installation directory.", "inputSchema": {"type": "object", "properties": {"servers": {"type": "array", "items": {"type": "string"}, "description": "Server process names to check (default: programmer)"}}}}),
        json!({"name": "deploy_preflight", "description": "Pre-deploy project checks. Validates a caller-supplied project path, or the current project when omitted.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string", "description": "Project directory (default: current directory)"}, "target": {"type": "string", "description": "Legacy alias for path"}}}}),
        json!({
            "name": "config_validate_mcp",
            "description": "Validate MCP configuration file. Checks structure and command existence.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "config_path": {"type": "string", "description": "Path to config (auto-detects if not provided)"}
                }
            }
        }),
        // ============ META (4) ============
        json!({"name": "plan", "description": "Analyze a task and return its ingredients: tools needed, dependencies, breadcrumb recommendation. Pass assemble=<plan object> to enrich an existing plan with cross-server requirements instead.", "inputSchema": {"type": "object", "properties": {"task": {"type": "string", "description": "What needs to be done"}, "context": {"type": "string", "description": "Additional context"}, "assemble": {"type": "object", "description": "A plan object to enrich (replaces the former plan_assemble)"}}}}),
        json!({"name": "sqlite_query", "description": "Execute a read-only SQL query against a SQLite database. Returns results as JSON array.", "inputSchema": {"type": "object", "properties": {"db_path": {"type": "string", "description": "Path to the .db file"}, "sql": {"type": "string", "description": "SQL query to execute (SELECT only)"}, "max_rows": {"type": "integer", "description": "Max rows to return (default 100)", "default": 100}}, "required": ["db_path", "sql"]}}),
        json!({"name": "registry_read", "description": "Read Windows registry values from approved locations only.", "inputSchema": {"type": "object", "properties": {"key": {"type": "string", "description": "Full registry path, e.g. HKLM\\SOFTWARE\\Microsoft"}, "value_name": {"type": "string", "description": "Optional specific value name. Empty string reads the default value."}, "recursive": {"type": "boolean", "description": "Include one level of subkeys.", "default": false}}, "required": ["key"]}}),
        json!({"name": "doctor", "description": "Per-host capability self-report: git present, WSL present (+distros), shells available, python/pandoc/cargo, state dirs, build profile.", "inputSchema": {"type": "object", "properties": {}}}),
    ]
}

/// Execute a tool by name. Canonical names only - the term_* alias layer and all
/// name-duplicates were retired 2026-07-29.
pub async fn execute_tool(name: &str, args: Value) -> Result<Value> {
    match name {
        // File operations
        "read_file" => file::read_file(args).await,
        "write_file" => file::write_file(args).await,
        "edit_block" => file::edit_block(args).await,
        "copy_file" => file::copy_file(args).await,
        "move_file" => file::move_file(args).await,
        "create_dir" => file::create_directory(args).await,
        "list_dir" => file::list_directory(args).await,

        // Shell
        "cmd" => shell::execute(args).await,
        "powershell" => shell::powershell(args).await,
        "kill_process" => system::kill_process(args).await,
        "list_process" => system::list_processes(args).await,
        "shortcut" => shell::shortcut(args).await,

        // Sessions
        "shell_session" => shell::shell_session(args).await,
        "live_shell" => psession::live_shell(args).await,

        // Search
        "search_file" => search::search(args).await,

        // System
        "screenshot" => Ok(system::screenshot(&args)),
        "system_info" => system::get_info().await,
        "clipboard_read" => system::clipboard_read().await,
        "clipboard_write" => system::clipboard_write(args).await,
        "md2docx" => shell::md2docx(args).await,

        // WSL
        "wsl_run" => wsl::run(args).await,
        "wsl_bg" => wsl::bg(args).await,
        "wsl_status" => wsl::status(args).await,
        "wsl_log" => wsl::log_output(args).await,

        // Transforms + stats
        "archive_create" => transform::archive(args).await,
        "archive_extract" => transform::extract(args).await,
        "transform_bulk_rename" => transform::bulk_rename(args).await,
        "transform_sync_dir" => transform::sync_directories(args).await,
        "diff_file" => transform::diff_files(args).await,
        "transform_file" => transform::transform_files(args).await,
        "base64" => transform::base64_tool(args).await,
        "json" => transform::json_tool(args).await,
        "convert" => transform::convert_tool(args).await,
        "transform_find_replace" => transform::find_replace(args).await,
        "transform_hash_file" => transform::hash_file(args).await,
        "transform_scaffold" => transform::scaffold(args).await,
        "grep" => transform::grep(args).await,
        "file_stats" => transform::file_stats(args).await,

        // Network
        "http_request" => http::request(args).await,
        "port_check" => system::port_check(args).await,

        // Security
        "security_check_cmd" => security::check_command(args).await,
        "security_audit_log" => security::audit_log(args).await,

        // Infra
        "server_health" => infra::server_health(args).await,
        "deploy_preflight" => infra::preflight_deploy(args).await,
        "config_validate_mcp" => config::validate_mcp_config(args).await,

        // Meta
        "plan" => {
            if args.get("assemble").map(|v| v.is_object()).unwrap_or(false) {
                Ok(planner::assemble(&json!({"plan": args["assemble"]})))
            } else {
                Ok(planner::plan(&args))
            }
        }
        "sqlite_query" => sqlite::query(args).await,
        "registry_read" => Ok(registry::execute("registry_read", &args)),
        "doctor" => system::doctor().await,

        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}

#[cfg(test)]
mod tests {
    use super::get_tool_definitions;
    use std::collections::HashSet;

    #[test]
    fn every_build_advertises_exactly_49_unique_tools_with_honest_cmd_name() {
        let definitions = get_tool_definitions();
        let names: Vec<&str> = definitions
            .iter()
            .map(|definition| definition["name"].as_str().expect("tool name"))
            .collect();
        let unique: HashSet<&str> = names.iter().copied().collect();

        assert_eq!(definitions.len(), 49);
        assert_eq!(unique.len(), 49);
        assert!(unique.contains("cmd"));
        assert!(!unique.contains("bash"));
    }
}
