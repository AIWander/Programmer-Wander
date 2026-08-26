//! Generic process health and project preflight checks.

use anyhow::Result;
use serde_json::{json, Value};
use std::path::PathBuf;
use sysinfo::{PidExt, ProcessExt, System, SystemExt};

pub async fn server_health(args: Value) -> Result<Value> {
    let requested: Vec<String> = args
        .get("servers")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| {
                    value
                        .as_str()
                        .filter(|name| !name.trim().is_empty())
                        .map(String::from)
                })
                .collect()
        })
        .filter(|values: &Vec<String>| !values.is_empty())
        .unwrap_or_else(|| vec![env!("CARGO_PKG_NAME").to_string()]);

    let system = System::new_all();
    let servers: Vec<Value> =
        requested
            .iter()
            .map(|requested_name| {
                let needle = requested_name.trim_end_matches(".exe").to_ascii_lowercase();
                let matches: Vec<Value> = system.processes().iter().filter_map(|(pid, process)| {
            let process_name = process.name().trim_end_matches(".exe");
            if process_name.to_ascii_lowercase().contains(&needle) {
                Some(json!({
                    "name": process.name(),
                    "pid": pid.as_u32(),
                    "memory_mb": ((process.memory() as f64) / 1024.0 * 10.0).round() / 10.0,
                }))
            } else {
                None
            }
        }).collect();
                json!({
                    "query": requested_name,
                    "alive": !matches.is_empty(),
                    "matches": matches,
                })
            })
            .collect();

    Ok(json!({"servers": servers, "query_count": requested.len()}))
}

// tool_fallback retired 2026-07-29: its hardcoded map referenced retired servers and never
// read the real fallback map (Volumes/logs/error_fallbacks.json). Replaced by the 3-strike
// PostToolUse hook + autonomous:error_get_fallback.

pub async fn preflight_deploy(args: Value) -> Result<Value> {
    let requested = args
        .get("path")
        .and_then(|value| value.as_str())
        .or_else(|| args.get("target").and_then(|value| value.as_str()));
    let mut project_path = requested
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    if project_path.is_file() {
        project_path = project_path
            .parent()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("project path has no parent"))?;
    }

    let resolved = project_path
        .canonicalize()
        .unwrap_or_else(|_| project_path.clone());
    let manifest_path = resolved.join("Cargo.toml");
    let source_path = resolved.join("src");
    let manifest_valid = std::fs::read_to_string(&manifest_path)
        .map(|content| content.lines().any(|line| line.trim() == "[package]"))
        .unwrap_or(false);
    let ready =
        resolved.is_dir() && source_path.is_dir() && manifest_path.is_file() && manifest_valid;

    Ok(json!({
        "path": resolved,
        "ready": ready,
        "checks": {
            "project_directory_exists": resolved.is_dir(),
            "source_directory_exists": source_path.is_dir(),
            "cargo_manifest_exists": manifest_path.is_file(),
            "cargo_manifest_has_package": manifest_valid,
        }
    }))
}
