//! MCP config validation.
//! 2026-07-29 rebuild: the hidden config_* family (get/set/reload/usage_stats/recent_calls)
//! and the session recovery quartet were deleted - all were dead facades (state never
//! populated, values consumed by no code path; writers were dead code). The unreachable
//! IDE-import helpers (no dispatch entries) went with them. Real session recovery now
//! lives in shell.rs (auto-persist) and psession.rs (live_shell checkpoint/recover).

use anyhow::Result;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// Validate MCP configuration file
pub async fn validate_mcp_config(args: Value) -> Result<Value> {
    let config_path = args["config_path"].as_str();

    // Auto-detect config location
    let path = if let Some(p) = config_path {
        PathBuf::from(p)
    } else {
        let user_profile = std::env::var("APPDATA").unwrap_or_default();
        PathBuf::from(user_profile)
            .join("Claude")
            .join("claude_desktop_config.json")
    };

    if !path.exists() {
        return Ok(json!({
            "success": false,
            "error": format!("Config file not found: {}", path.display()),
            "searched_path": path.to_string_lossy()
        }));
    }

    // Read and parse
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "error": format!("Failed to read config: {}", e),
                "path": path.to_string_lossy()
            }))
        }
    };

    let config: Value = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            return Ok(json!({
                "success": false,
                "error": format!("Invalid JSON: {}", e),
                "path": path.to_string_lossy()
            }))
        }
    };

    let mut issues: Vec<String> = Vec::new();
    let mut servers_found: Vec<String> = Vec::new();
    let mut commands_checked: Vec<Value> = Vec::new();

    // Check structure
    if let Some(servers) = config.get("mcpServers").and_then(|s| s.as_object()) {
        for (name, server_config) in servers {
            servers_found.push(name.clone());

            // Check command exists
            if let Some(cmd) = server_config.get("command").and_then(|c| c.as_str()) {
                let cmd_path = PathBuf::from(cmd);
                let exists = cmd_path.exists();

                commands_checked.push(json!({
                    "server": name,
                    "command": cmd,
                    "exists": exists
                }));

                if !exists {
                    issues.push(format!("Server '{}': command not found: {}", name, cmd));
                }
            } else {
                issues.push(format!("Server '{}': missing 'command' field", name));
            }
        }
    } else {
        issues.push("Missing 'mcpServers' object".to_string());
    }

    Ok(json!({
        "success": issues.is_empty(),
        "path": path.to_string_lossy(),
        "servers_found": servers_found,
        "servers_count": servers_found.len(),
        "commands_checked": commands_checked,
        "issues": issues,
        "valid": issues.is_empty()
    }))
}
