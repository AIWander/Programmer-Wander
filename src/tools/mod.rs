//! Tool Registry and Dispatch
// NAV: TOC at line 1098 | 2 fn | 0 struct | 2026-04-08

mod config;
mod file;
mod git;
mod http;
mod infra;
mod planner;
mod psession;
mod registry;
mod search;
mod security;
mod shell;
mod smart;
mod sqlite;
mod system;
mod transform;
mod webhook;
mod wsl;

use anyhow::Result;
use serde_json::{json, Value};

/// Get all tool definitions for MCP tools/list
pub fn get_tool_definitions() -> Vec<Value> {
    let mut defs = vec![
        // ============ FILE OPERATIONS ============
        json!({
            "name": "read_file",
            "description": "Read file with smart options: search for pattern, get specific lines, or auto-truncate large files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"},
                    "search": {"type": "string", "description": "Grep for pattern, return matching lines"},
                    "lines": {"type": "string", "description": "Line range like 50:100"},
                    "max_kb": {"type": "integer", "description": "Max KB to return", "default": 100},
                    "offset": {"type": "integer", "description": "Legacy start line", "default": 0},
                    "length": {"type": "integer", "description": "Legacy max lines", "default": -1}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "write_file",
            "description": "Write content to file. Creates parent directories if needed.",
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
            "description": "Replace text in file with string replacement.",
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
            "name": "get_file_info",
            "description": "Get file metadata: size, dates, permissions.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
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
        // ============ SHELL EXECUTION ============
        json!({
            "name": "bash",
            "description": "Execute shell command and return output with exit code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command to execute"},
                    "timeout": {"type": "integer", "description": "Timeout in seconds", "default": 30}
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "run",
            "description": "Execute command in terminal session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string"},
                    "session_id": {"type": "string"},
                    "timeout": {"type": "integer", "default": 30}
                },
                "required": ["command", "session_id"]
            }
        }),
        json!({
            "name": "chain",
            "description": "Execute sequence of commands atomically - stops on first failure.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "commands": {"type": "array", "items": {"type": "string"}},
                    "session_id": {"type": "string"},
                    "stop_on_error": {"type": "boolean", "default": true}
                },
                "required": ["commands", "session_id"]
            }
        }),
        json!({
            "name": "session_create",
            "description": "Create persistent terminal session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "cwd": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "session_list",
            "description": "List all active terminal sessions.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "session_destroy",
            "description": "Destroy terminal session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"}
                },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "session_set_env",
            "description": "Set environment variable in session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "key": {"type": "string"},
                    "value": {"type": "string"}
                },
                "required": ["session_id", "key", "value"]
            }
        }),
        json!({
            "name": "session_get_env",
            "description": "Get environment variable(s) from session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "key": {"type": "string"}
                },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "session_history",
            "description": "Get command history with exit codes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "limit": {"type": "integer", "default": 10}
                },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "session_read_output",
            "description": "Read recent output from session buffer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "lines": {"type": "integer", "default": 50}
                },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "shortcut",
            "description": "Run pre-built command shortcut.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "shortcut_name": {"type": "string"},
                    "params": {"type": "object"}
                },
                "required": ["session_id", "shortcut_name"]
            }
        }),
        json!({
            "name": "list_shortcut",
            "description": "List all available command shortcuts.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "session_checkpoint",
            "description": "Save session state to checkpoint file for crash recovery.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string", "description": "Session name (default: 'default')"},
                    "checkpoint_path": {"type": "string", "description": "Path to save checkpoint (default: C:/temp/session_{name}.checkpoint)"}
                }
            }
        }),
        json!({
            "name": "session_recover",
            "description": "Recover session from checkpoint file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "checkpoint_path": {"type": "string", "description": "Path to checkpoint file"}
                },
                "required": ["checkpoint_path"]
            }
        }),
        // ============ SEARCH ============
        json!({
            "name": "search_start",
            "description": "Start streaming search for files by name or content.",
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
        json!({
            "name": "search_file",
            "description": "Search for files by name or content (simple interface).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "pattern": {"type": "string"},
                    "search_type": {"type": "string", "default": "files"}
                },
                "required": ["path", "pattern"]
            }
        }),
        // ============ SYSTEM ============
        json!({
            "name": "screenshot",
            "description": "Take a screenshot for troubleshooting. Returns file path + metadata only (no raw bytes). Capped at 1MB — lower quality/scale if exceeded. Default: quality=60, scale=0.75.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "save_path": {"type": "string", "description": "Output path (default: C:\\temp\\screenshot_<ts>.jpg)"},
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
        // ============ GIT ============
        json!({
            "name": "git_status",
            "description": "Get git status: branch, modified, staged files.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."}
                }
            }
        }),
        json!({
            "name": "git_diff",
            "description": "Get git diff.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "staged": {"type": "boolean", "default": false},
                    "file": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "git_commit",
            "description": "Create git commit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "message": {"type": "string"},
                    "files": {"type": "array", "items": {"type": "string"}}
                }
            }
        }),
        json!({
            "name": "git_push",
            "description": "Push commits to remote.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "remote": {"type": "string", "default": "origin"},
                    "branch": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "git_pull",
            "description": "Pull changes from remote.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "remote": {"type": "string", "default": "origin"}
                }
            }
        }),
        json!({
            "name": "git_log",
            "description": "Get commit history.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "limit": {"type": "integer", "default": 10}
                }
            }
        }),
        json!({
            "name": "git_branch",
            "description": "List, create, or delete branches.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "name": {"type": "string"},
                    "delete": {"type": "boolean", "default": false}
                }
            }
        }),
        json!({
            "name": "git_checkout",
            "description": "Switch branch or restore file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "branch": {"type": "string"},
                    "file": {"type": "string"},
                    "create": {"type": "boolean", "default": false}
                }
            }
        }),
        json!({
            "name": "git_stash",
            "description": "Git stash: push, pop, list, drop.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."},
                    "action": {"type": "string", "default": "push"},
                    "message": {"type": "string"}
                }
            }
        }),
        json!({
            "name": "git_diff_summary",
            "description": "AI-friendly structured diff for commit messages.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo_path": {"type": "string", "default": "."}
                }
            }
        }),
        // ============ HTTP ============
        json!({
            "name": "http_request",
            "description": "Make HTTP request.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "method": {"type": "string", "default": "GET"},
                    "headers": {"type": "object"},
                    "body": {"type": "string"},
                    "timeout": {"type": "integer", "default": 30}
                },
                "required": ["url"]
            }
        }),
        json!({
            "name": "http_download",
            "description": "Download file from URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": {"type": "string"},
                    "destination": {"type": "string"}
                },
                "required": ["url", "destination"]
            }
        }),
        // ============ WEBHOOKS ============
        json!({
            "name": "webhook_start",
            "description": "Start webhook server for external triggers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "port": {"type": "integer", "default": 9000},
                    "routes": {"type": "object"}
                },
                "required": ["port", "routes"]
            }
        }),
        json!({
            "name": "webhook_stop",
            "description": "Stop a webhook server.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "server_id": {"type": "string"}
                },
                "required": ["server_id"]
            }
        }),
        json!({
            "name": "webhook_list",
            "description": "List all webhook servers.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "webhook_add_route",
            "description": "Add route to existing webhook server.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "server_id": {"type": "string"},
                    "path": {"type": "string"},
                    "action": {"type": "string"}
                },
                "required": ["server_id", "path", "action"]
            }
        }),
        // ============ TRANSFORM ============
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
            "description": "Apply Python transform to matching files.",
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
        // ============ TOKEN-SAVING TRANSFORMS (ported from mcp-windows) ============
        json!({
            "name": "transform_json_format",
            "description": "Pretty-print JSON with proper indentation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "json_string": {"type": "string", "description": "JSON to format"},
                    "indent": {"type": "integer", "description": "Spaces (default: 2)"}
                },
                "required": ["json_string"]
            }
        }),
        json!({
            "name": "transform_json_minify",
            "description": "Minify JSON by removing whitespace.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "json_string": {"type": "string", "description": "JSON to minify"}
                },
                "required": ["json_string"]
            }
        }),
        json!({
            "name": "transform_base64_encode",
            "description": "Encode string to base64.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to encode"}
                },
                "required": ["text"]
            }
        }),
        json!({
            "name": "transform_base64_decode",
            "description": "Decode base64 to string.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "encoded": {"type": "string", "description": "Base64 to decode"}
                },
                "required": ["encoded"]
            }
        }),
        json!({
            "name": "transform_csv_to_json",
            "description": "Convert CSV to JSON array. First row = headers.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "csv_string": {"type": "string", "description": "CSV data"},
                    "delimiter": {"type": "string", "description": "Delimiter (default: comma)"}
                },
                "required": ["csv_string"]
            }
        }),
        json!({
            "name": "transform_json_to_csv",
            "description": "Convert JSON array to CSV.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "json_array": {"type": "string", "description": "JSON array"},
                    "delimiter": {"type": "string", "description": "Delimiter (default: comma)"}
                },
                "required": ["json_array"]
            }
        }),
        json!({
            "name": "transform_find_replace",
            "description": "Find/replace in file(s). Saves reading entire file into chat.",
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
            "name": "file_stats",
            "description": "Get file/directory stats without reading content.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to analyze"},
                    "recursive": {"type": "boolean", "description": "Include subdirs (default: false)"}
                },
                "required": ["path"]
            }
        }),
        json!({
            "name": "extract_lines",
            "description": "Extract specific line range from file. Saves reading entire file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path"},
                    "start": {"type": "integer", "description": "Start line (1-indexed)"},
                    "end": {"type": "integer", "description": "End line (inclusive, -1 for EOF)"}
                },
                "required": ["path", "start"]
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
        // ============ SMART ROUTING ============
        json!({
            "name": "smart_exec",
            "description": "Auto-routing command execution. Analyzes command and routes to term_run (simple), session (needs env/cwd), or powershell (PS syntax). Returns which route was used.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Command to execute"},
                    "cwd": {"type": "string", "description": "Working directory (triggers session mode)"},
                    "needs_env": {"type": "boolean", "default": false, "description": "If true, uses persistent session"}
                },
                "required": ["command"]
            }
        }),
        json!({
            "name": "smart_read",
            "description": "Auto-routing file read. Routes to term_read_file (default), term_grep (pattern search), term_extract_lines (specific lines), or diff_files (comparison).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"},
                    "find": {"type": "string", "description": "Search for pattern (uses grep)"},
                    "lines": {"type": "string", "description": "Line range like '50:100'"},
                    "compare_to": {"type": "string", "description": "Compare with another file (returns diff)"},
                    "max_kb": {"type": "integer", "default": 100, "description": "Max KB to return (default: 100)"}
                },
                "required": ["path"]
            }
        }),
        // ============ MCP CONFIG & IDE ============
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
        // ============ RESOURCE MONITORING ============
        json!({
            "name": "watch_resource",
            "description": "Monitor system resources and alert on thresholds.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "thresholds": {"type": "object", "description": "cpu/memory/disk thresholds (%)"},
                    "interval_seconds": {"type": "integer", "description": "Check interval", "default": 60}
                }
            }
        }),
        json!({
            "name": "stop_watch",
            "description": "Stop resource monitoring.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "watch_id": {"type": "string"}
                },
                "required": ["watch_id"]
            }
        }),
        json!({
            "name": "get_alert",
            "description": "Get resource alerts.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "watch_id": {"type": "string", "description": "Filter by watch ID"},
                    "limit": {"type": "integer", "default": 50}
                }
            }
        }),
        json!({
            "name": "list_watch",
            "description": "List active resource watches.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        // ============ RECOVERY ============
        json!({
            "name": "session_recovery_status",
            "description": "Check recovery status - shows recoverable sessions and resumable operations.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "session_recover_data",
            "description": "Get recovery data for a crashed session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"}
                },
                "required": ["session_id"]
            }
        }),
        json!({
            "name": "session_resume_op",
            "description": "Resume an interrupted long-running operation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "checkpoint_id": {"type": "string"}
                },
                "required": ["checkpoint_id"]
            }
        }),
        json!({
            "name": "session_clear_recovery",
            "description": "Clear all recovery data.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({"name": "powershell", "description": "Execute PowerShell command. Most versatile single tool for Windows.", "inputSchema": {"type": "object", "properties": {"command": {"type": "string", "description": "PowerShell command to execute"}, "timeout_secs": {"type": "integer", "description": "Timeout in seconds (default: 30)", "default": 30}}, "required": ["command"]}}),
        json!({"name": "md2docx", "description": "Convert Markdown file to DOCX via pandoc.", "inputSchema": {"type": "object", "properties": {"input": {"type": "string", "description": ".md file path"}, "output": {"type": "string", "description": ".docx output path"}}, "required": ["input", "output"]}}),
        json!({"name": "git_clone", "description": "Clone a git repository.", "inputSchema": {"type": "object", "properties": {"url": {"type": "string", "description": "Repository URL"}, "destination": {"type": "string", "description": "Local directory"}, "branch": {"type": "string", "description": "Branch to clone (optional)"}}, "required": ["url"]}}),
        json!({"name": "git_remote", "description": "Manage git remotes: list, add, remove.", "inputSchema": {"type": "object", "properties": {"repo_path": {"type": "string", "default": "."}, "action": {"type": "string", "description": "list (default), add, remove", "default": "list"}, "name": {"type": "string", "description": "Remote name (for add/remove)"}, "url": {"type": "string", "description": "Remote URL (for add)"}}}}),
        json!({"name": "wsl_run", "description": "Run command in WSL. Returns output summary + log path.", "inputSchema": {"type": "object", "properties": {"command": {"type": "string", "description": "Command to run in WSL"}, "timeout_secs": {"type": "integer", "description": "Timeout (default: 120)", "default": 120}}, "required": ["command"]}}),
        json!({"name": "wsl_bg", "description": "Launch WSL background job. Returns job_id. Poll with wsl_status.", "inputSchema": {"type": "object", "properties": {"command": {"type": "string", "description": "Command to run in background"}, "job_name": {"type": "string", "description": "Optional friendly name"}}, "required": ["command"]}}),
        json!({"name": "wsl_status", "description": "Check WSL background job status. Use job_id=all to list all.", "inputSchema": {"type": "object", "properties": {"job_id": {"type": "string", "description": "Job ID or all"}, "tail": {"type": "integer", "description": "Log lines to return (default: 10)", "default": 10}}, "required": ["job_id"]}}),
        json!({"name": "wsl_log", "description": "Get full or partial log from a WSL background job.", "inputSchema": {"type": "object", "properties": {"job_id": {"type": "string", "description": "Job ID"}, "lines": {"type": "string", "description": "Range like 1:50 or last:20 (default: last:50)"}}, "required": ["job_id"]}}),
        json!({"name": "psession_create", "description": "Create persistent shell session (PowerShell or WSL). State persists across calls.", "inputSchema": {"type": "object", "properties": {"name": {"type": "string", "description": "Session name (default: default)"}, "shell": {"type": "string", "description": "powershell (default) or wsl", "default": "powershell"}, "cwd": {"type": "string", "description": "Working directory (default: C:\\\\)"}}}}),
        json!({"name": "psession_run", "description": "Run command in persistent session. Variables, CWD, state persist.", "inputSchema": {"type": "object", "properties": {"session_id": {"type": "string", "description": "Session ID from psession_create"}, "command": {"type": "string", "description": "Command to run"}, "timeout_secs": {"type": "integer", "description": "Timeout (default: 30)", "default": 30}}, "required": ["session_id", "command"]}}),
        json!({"name": "psession_destroy", "description": "Kill a persistent session.", "inputSchema": {"type": "object", "properties": {"session_id": {"type": "string"}}, "required": ["session_id"]}}),
        json!({"name": "psession_list", "description": "List all persistent sessions.", "inputSchema": {"type": "object", "properties": {}}}),
        json!({"name": "psession_read", "description": "Read output buffer from persistent session.", "inputSchema": {"type": "object", "properties": {"session_id": {"type": "string"}, "tail": {"type": "integer", "description": "Lines to return (default: 20)", "default": 20}}, "required": ["session_id"]}}),
        json!({"name": "psession_history", "description": "Get command history for a persistent session.", "inputSchema": {"type": "object", "properties": {"session_id": {"type": "string"}}, "required": ["session_id"]}}),
        json!({"name": "append_file", "description": "Append content to a file.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string", "description": "File path"}, "content": {"type": "string", "description": "Content to append"}}, "required": ["path", "content"]}}),
        json!({"name": "session_cd", "description": "Change working directory in a session.", "inputSchema": {"type": "object", "properties": {"path": {"type": "string", "description": "Directory to change to"}, "session_id": {"type": "string", "description": "Session ID"}}, "required": ["path", "session_id"]}}),
        json!({"name": "shortcut_chain", "description": "Run multiple shortcuts in sequence. Stops on first error by default.", "inputSchema": {"type": "object", "properties": {"shortcuts": {"type": "array", "items": {"type": "string"}, "description": "Shortcut names to run"}, "stop_on_error": {"type": "boolean", "default": true}}, "required": ["shortcuts"]}}),
        json!({"name": "http_scrape", "description": "Fetch URL and strip HTML, returning text content only.", "inputSchema": {"type": "object", "properties": {"url": {"type": "string", "description": "URL to scrape"}, "selector": {"type": "string", "description": "CSS selector (optional)"}}, "required": ["url"]}}),
        json!({"name": "security_check_cmd", "description": "Check if a command is safe to execute. Returns warnings for dangerous patterns.", "inputSchema": {"type": "object", "properties": {"command": {"type": "string", "description": "Command to check"}}, "required": ["command"]}}),
        json!({"name": "security_audit_log", "description": "View recent security audit log entries.", "inputSchema": {"type": "object", "properties": {"lines": {"type": "integer", "description": "Number of recent entries (default: 20)", "default": 20}}}}),
        json!({"name": "server_health", "description": "Check which MCP servers are alive. Returns process status.", "inputSchema": {"type": "object", "properties": {"servers": {"type": "array", "items": {"type": "string"}, "description": "Specific servers to check (default: all)"}}}}),
        json!({"name": "tool_fallback", "description": "Look up fallback tool when primary is unavailable.", "inputSchema": {"type": "object", "properties": {"tool": {"type": "string", "description": "Full tool name that failed"}}, "required": ["tool"]}}),
        json!({"name": "deploy_preflight", "description": "Pre-deploy safety checks. Verifies source exists and servers are running.", "inputSchema": {"type": "object", "properties": {"target": {"type": "string", "description": "Server name to deploy"}}, "required": ["target"]}}),
        json!({"name": "plan", "description": "Analyze a task and return its ingredients: what tools are needed, which depend on each other, and whether breadcrumbing is warranted. Does NOT prescribe step order - Claude decides execution.", "inputSchema": {"type": "object", "properties": {"task": {"type": "string", "description": "What needs to be done"}, "context": {"type": "string", "description": "Additional context"}}, "required": ["task"]}}),
        json!({"name": "plan_assemble", "description": "Enrich a plan with cross-server requirements.", "inputSchema": {"type": "object", "properties": {"plan": {"type": "object"}}, "required": ["plan"]}}),
        json!({"name": "sqlite_query", "description": "Execute a read-only SQL query against a SQLite database. Returns results as JSON array.", "inputSchema": {"type": "object", "properties": {"db_path": {"type": "string", "description": "Path to the .db file"}, "sql": {"type": "string", "description": "SQL query to execute (SELECT only)"}, "max_rows": {"type": "integer", "description": "Max rows to return (default 100)", "default": 100}}, "required": ["db_path", "sql"]}}),
        json!({"name": "port_check", "description": "Test TCP connectivity to a host:port. Returns whether the port is open and connection time.", "inputSchema": {"type": "object", "properties": {"host": {"type": "string", "description": "Host to connect to (default: 127.0.0.1)", "default": "127.0.0.1"}, "port": {"type": "integer", "description": "Port number"}, "timeout_ms": {"type": "integer", "description": "Connection timeout in ms (default: 2000)", "default": 2000}}, "required": ["port"]}}),
        json!({"name": "registry_read", "description": "Read Windows registry values from approved locations only.", "inputSchema": {"type": "object", "properties": {"key": {"type": "string", "description": "Full registry path, e.g. HKLM\\SOFTWARE\\Microsoft"}, "value_name": {"type": "string", "description": "Optional specific value name. Empty string reads the default value."}, "recursive": {"type": "boolean", "description": "Include one level of subkeys.", "default": false}}, "required": ["key"]}}),
        json!({"name": "tail_file", "description": "Return last N lines of a file plus current byte offset. Pass since_bytes from a previous call to get only NEW content (delta polling).", "inputSchema": {"type": "object", "properties": {"path": {"type": "string", "description": "File path to tail"}, "lines": {"type": "integer", "description": "Number of lines to return (default: 50)", "default": 50}, "since_bytes": {"type": "integer", "description": "Byte offset from previous call. 0 = read from end.", "default": 0}}, "required": ["path"]}}),
        json!({"name": "notify", "description": "Show a silent Windows toast notification.", "inputSchema": {"type": "object", "properties": {"title": {"type": "string", "description": "Notification title"}, "body": {"type": "string", "description": "Notification body"}, "icon": {"type": "string", "enum": ["info", "warning", "error"], "default": "info"}, "duration_ms": {"type": "integer", "default": 5000}}, "required": ["title", "body"]}}),
    ];

    // Category post-pass: stable-sort the flat list into the workbench sequence
    // (Files -> Search -> Shell -> Git -> Sessions -> Background -> Net -> Data
    // -> System -> Guard -> Plan) and tag each description so hosts that render
    // tools as one flat list still read grouped. Tool names never change.
    fn category_of(name: &str) -> (usize, &'static str) {
        if name.starts_with("git_") {
            return (4, "Git");
        }
        if name.starts_with("session_") || name.starts_with("psession_") {
            return (5, "Sessions");
        }
        if name.starts_with("wsl_")
            || name.starts_with("webhook_")
            || matches!(name, "watch_resource" | "stop_watch" | "list_watch" | "get_alert")
        {
            return (6, "Background");
        }
        if name.starts_with("http_") || name == "port_check" {
            return (7, "Net");
        }
        if name.starts_with("transform_")
            || name.starts_with("archive_")
            || matches!(name, "sqlite_query" | "md2docx")
        {
            return (8, "Data");
        }
        match name {
            "read_file" | "write_file" | "append_file" | "edit_block" | "copy_file"
            | "move_file" | "create_dir" | "list_dir" | "get_file_info" | "file_stats"
            | "diff_file" | "tail_file" | "extract_lines" => (1, "Files"),
            "grep" | "search_start" | "search_file" | "smart_read" => (2, "Search"),
            "bash" | "powershell" | "run" | "chain" | "smart_exec" | "shortcut"
            | "list_shortcut" | "shortcut_chain" => (3, "Shell"),
            "screenshot" | "system_info" | "clipboard_read" | "clipboard_write"
            | "list_process" | "kill_process" | "registry_read" | "notify"
            | "server_health" => (9, "System"),
            "security_check_cmd" | "security_audit_log" | "deploy_preflight"
            | "tool_fallback" | "config_validate_mcp" => (10, "Guard"),
            "plan" | "plan_assemble" => (11, "Plan"),
            _ => (12, "Utility"),
        }
    }

    for def in defs.iter_mut() {
        let name = def["name"].as_str().unwrap_or("").to_string();
        let (_, label) = category_of(&name);
        if let Some(desc) = def["description"].as_str() {
            def["description"] = json!(format!("[{label}] {desc}"));
        }
    }
    defs.sort_by_key(|def| category_of(def["name"].as_str().unwrap_or("")).0);
    defs
}

/// Execute a tool by name
pub async fn execute_tool(name: &str, args: Value) -> Result<Value> {
    match name {
        // File operations
        "read_file" | "term_read_file" => file::read_file(args).await,
        "write_file" | "term_write_file" => file::write_file(args).await,
        "edit_block" | "term_edit_block" => file::edit_block(args).await,
        "copy_file" | "term_copy_file" => file::copy_file(args).await,
        "move_file" | "term_move_file" => file::move_file(args).await,
        "get_file_info" | "term_get_file_info" => file::get_file_info(args).await,
        "create_dir" | "term_create_directory" => file::create_directory(args).await,
        "list_dir" | "term_list_directory" => file::list_directory(args).await,

        // Shell
        "bash" | "run" | "term_run" => shell::execute(args).await,
        "chain" | "term_chain" => shell::chain(args).await,
        "session_create" | "term_create_session" => shell::create_session(args).await,
        "session_list" | "term_list_sessions" => shell::list_sessions().await,
        "session_destroy" | "term_destroy" => shell::destroy_session(args).await,
        "session_set_env" | "term_set_env" => shell::set_env(args).await,
        "session_get_env" | "term_get_env" => shell::get_env(args).await,
        "session_history" | "term_history" => shell::history(args).await,
        "session_read_output" | "term_read_output" => shell::read_output(args).await,
        "shortcut" | "term_shortcut" => shell::shortcut(args).await,
        "list_shortcut" | "term_list_shortcuts" => shell::list_shortcuts().await,
        "session_checkpoint" | "term_session_checkpoint" => shell::session_checkpoint(args).await,
        "session_recover" | "term_session_recover_file" => {
            shell::session_recover_from_file(args).await
        }

        // Search
        "search_start" | "search_file" | "term_start_search" | "search_files" => {
            search::search(args).await
        }

        // System
        "screenshot" => Ok(system::screenshot(&args)),
        "system_info" | "term_get_system_info" => system::get_info().await,
        "clipboard_read" | "term_clipboard_read" => system::clipboard_read().await,
        "clipboard_write" | "term_clipboard_write" => system::clipboard_write(args).await,
        "list_process" | "term_list_processes" => system::list_processes(args).await,
        "kill_process" | "term_kill_process" => system::kill_process(args).await,

        // Browser

        // Git
        "git_status" | "term_git_status" => git::status(args).await,
        "git_diff" | "term_git_diff" => git::diff(args).await,
        "git_commit" | "term_git_commit" => git::commit(args).await,
        "git_push" | "term_git_push" => git::push(args).await,
        "git_pull" | "term_git_pull" => git::pull(args).await,
        "git_log" | "term_git_log" => git::log(args).await,
        "git_branch" | "term_git_branch" => git::branch(args).await,
        "git_checkout" | "term_git_checkout" => git::checkout(args).await,
        "git_stash" | "term_git_stash" => git::stash(args).await,
        "git_diff_summary" | "term_git_diff_summary" => git::diff_summary(args).await,
        // HTTP
        "http_request" | "term_http_request" => http::request(args).await,
        "http_download" | "term_download_file" => http::download(args).await,

        // Webhooks
        "webhook_start" | "term_webhook_server" => webhook::start_webhook_server(args).await,
        "webhook_stop" | "term_stop_webhook" => webhook::stop_webhook_server(args).await,
        "webhook_list" | "term_list_webhooks" => webhook::list_webhook_servers().await,
        "webhook_add_route" | "term_add_webhook_route" => webhook::add_webhook_route(args).await,

        // Transform
        "archive_create" | "term_archive" => transform::archive(args).await,
        "archive_extract" | "term_extract" => transform::extract(args).await,
        "transform_bulk_rename" | "term_bulk_rename" => transform::bulk_rename(args).await,
        "transform_sync_dir" | "term_sync_directories" => transform::sync_directories(args).await,
        "diff_file" | "term_diff_files" => transform::diff_files(args).await,
        "transform_file" | "term_transform_files" => transform::transform_files(args).await,

        // MCP Config & IDE
        "transform_json_format" | "term_json_format" => transform::json_format(args).await,
        "transform_json_minify" | "term_json_minify" => transform::json_minify(args).await,
        "transform_base64_encode" | "term_base64_encode" => transform::base64_encode(args).await,
        "transform_base64_decode" | "term_base64_decode" => transform::base64_decode(args).await,
        "transform_csv_to_json" | "term_csv_to_json" => transform::csv_to_json(args).await,
        "transform_json_to_csv" | "term_json_to_csv" => transform::json_to_csv(args).await,
        "transform_find_replace" | "term_find_replace" => transform::find_replace(args).await,
        "transform_hash_file" | "term_hash_file" => transform::hash_file(args).await,
        "file_stats" | "term_file_stats" => transform::file_stats(args).await,
        "extract_lines" | "term_extract_lines" => transform::extract_lines(args).await,
        "grep" | "term_grep" => transform::grep(args).await,
        "transform_scaffold" | "term_scaffold" => transform::scaffold(args).await,

        // Smart routing
        "smart_exec" | "term_smart_exec" => smart::smart_exec(args).await,
        "smart_read" | "term_smart_read" => smart::smart_read(args).await,

        // MCP Config & IDE
        "config_validate_mcp" | "term_validate_mcp_config" => {
            config::validate_mcp_config(args).await
        }
        // Resource Monitoring
        "watch_resource" | "term_watch_resources" => system::watch_resources(args).await,
        "stop_watch" | "term_stop_watch" => system::stop_resource_watch(args).await,
        "get_alert" | "term_get_alerts" => system::get_resource_alerts(args).await,
        "list_watch" | "term_list_watches" => system::list_resource_watches().await,

        // Config & Recovery
        "config_get" | "term_get_config" => config::get_config().await,
        "config_set" | "term_set_config" => config::set_config(args).await,
        "config_reload" | "term_reload_config" => config::reload_config().await,
        "config_get_usage_stats" | "term_get_usage_stats" => config::get_usage_stats().await,
        "config_get_recent_calls" | "term_get_recent_calls" => config::get_recent_calls(args).await,
        "session_recovery_status" | "term_recovery_status" => config::recovery_status().await,
        "session_recover_data" | "term_recover_session" => config::recover_session(args).await,
        "session_resume_op" | "term_resume_operation" => config::resume_operation(args).await,
        "session_clear_recovery" | "term_clear_recovery" => config::clear_recovery().await,

        "powershell" => shell::powershell(args).await,
        "md2docx" => shell::md2docx(args).await,
        "git_clone" | "term_git_clone" => git::clone(args).await,
        "git_remote" | "term_git_remote" => git::remote(args).await,
        "wsl_run" => wsl::run(args).await,
        "wsl_bg" => wsl::bg(args).await,
        "wsl_status" => wsl::status(args).await,
        "wsl_log" => wsl::log_output(args).await,
        "psession_create" => psession::create(args).await,
        "psession_run" => psession::run(args).await,
        "psession_destroy" => psession::destroy(args).await,
        "psession_list" => psession::list(args).await,
        "psession_read" => psession::read_output(args).await,
        "psession_history" => psession::history(args).await,
        "append_file" | "term_append_file" => file::append_file(args).await,
        "session_cd" | "term_session_cd" => shell::session_cd(args).await,
        "shortcut_chain" | "term_shortcut_chain" => shell::shortcut_chain(args).await,
        "http_scrape" | "term_http_scrape" => http::scrape(args).await,
        "security_check_cmd" | "security_check_command" => security::check_command(args).await,
        "security_audit_log" => security::audit_log(args).await,
        "server_health" => infra::server_health(args).await,
        "tool_fallback" => infra::tool_fallback(args).await,
        "deploy_preflight" | "preflight_deploy" => infra::preflight_deploy(args).await,
        "plan" => Ok(planner::plan(&args)),
        "plan_assemble" | "assemble" => Ok(planner::assemble(&args)),
        "sqlite_query" => sqlite::query(args).await,
        "port_check" => system::port_check(args).await,
        "registry_read" => Ok(registry::execute("registry_read", &args)),
        "tail_file" => Ok(system::tail_file(&args)),
        "notify" => Ok(system::notify(&args)),
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}

// === FILE NAVIGATION ===
// Generated: 2026-04-08T14:12:36
// Total: 1095 lines | 2 functions | 0 structs | 0 constants
//
// IMPORTS: anyhow, serde_json
//
// FUNCTIONS:
//   pub +get_tool_definitions: 25-954 [LARGE]
//   pub +execute_tool: 957-1095 [LARGE]
//
// === END FILE NAVIGATION ===
