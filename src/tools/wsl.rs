//! WSL tools - Run commands and background jobs in WSL
//! Background jobs stay alive because programmer.exe holds the child process handle

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tracing::info;

use super::{runtime, security};

static JOBS: Mutex<Option<HashMap<String, WslJob>>> = Mutex::new(None);

struct WslJob {
    pid: u32,
    status_file: String,
    started: Instant,
}

fn get_jobs() -> std::sync::MutexGuard<'static, Option<HashMap<String, WslJob>>> {
    let mut guard = JOBS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

fn gen_job_id() -> String {
    format!("wsl_{}", uuid::Uuid::new_v4().simple())
}

fn sanitize_job_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .take(96)
        .collect()
}

fn wsl_state_dir() -> std::path::PathBuf {
    runtime::state_path("wsl")
}

fn job_paths(job_id: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let safe_id = sanitize_job_id(job_id);
    let state_dir = wsl_state_dir();
    (
        state_dir.join(format!("wsl_bg_{}.log", safe_id)),
        state_dir.join(format!("wsl_bg_{}.status", safe_id)),
    )
}

pub async fn run(args: Value) -> Result<Value> {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let timeout = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(120);

    if command.is_empty() {
        anyhow::bail!("command is required");
    }

    security::enforce_command_safety(command, "wsl_run")?;

    info!("WSL run: {}", &command[..command.len().min(80)]);

    let state_dir = wsl_state_dir();
    let _ = fs::create_dir_all(&state_dir);
    let log_path = state_dir.join(format!(
        "wsl_run_{}_{}.log",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        uuid::Uuid::new_v4().simple()
    ));

    let start = Instant::now();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout),
        tokio::process::Command::new("wsl")
            .args(["-d", "Ubuntu-24.04", "--", "bash", "-c", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await;

    let duration = start.elapsed().as_secs();

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{}{}", stdout, stderr);
            let _ = fs::write(&log_path, &combined);
            let lines: Vec<&str> = combined.lines().collect();
            let line_count = lines.len();
            let tail: Vec<&str> = lines.iter().rev().take(15).rev().cloned().collect();

            Ok(json!({
                "status": if output.status.success() { "success" } else { "failed" },
                "exit_code": output.status.code().unwrap_or(-1),
                "duration_secs": duration,
                "total_lines": line_count,
                "log": log_path,
                "tail": tail.join("\n")
            }))
        }
        Ok(Err(e)) => Ok(json!({"error": format!("WSL launch failed: {}", e)})),
        Err(_) => Ok(json!({"error": format!("Timed out after {}s", timeout)})),
    }
}

pub async fn bg(args: Value) -> Result<Value> {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    if command.is_empty() {
        anyhow::bail!("command is required");
    }

    security::enforce_command_safety(command, "wsl_bg")?;

    let job_id = args
        .get("job_name")
        .and_then(|v| v.as_str())
        .map(sanitize_job_id)
        .unwrap_or_else(gen_job_id);

    let state_dir = wsl_state_dir();
    fs::create_dir_all(&state_dir)?;
    let (log_file, status_file) = job_paths(&job_id);

    let _ = fs::write(&status_file, r#"{"status":"running"}"#);

    let log_handle = fs::File::create(&log_file)?;
    let err_handle = log_handle.try_clone()?;

    let child = std::process::Command::new("wsl")
        .args(["-d", "Ubuntu-24.04", "--", "bash", "-c", command])
        .stdout(log_handle)
        .stderr(err_handle)
        .spawn();

    match child {
        Ok(child) => {
            let pid = child.id();
            let mut jobs = get_jobs();
            if let Some(ref mut map) = *jobs {
                map.insert(
                    job_id.clone(),
                    WslJob {
                        pid,
                        status_file: status_file.to_string_lossy().into_owned(),
                        started: Instant::now(),
                    },
                );
            }

            let sf = status_file.clone();
            let jid = job_id.clone();
            std::thread::spawn(move || {
                let mut child = child;
                let result = child.wait();
                let exit = match result {
                    Ok(status) => status.code().unwrap_or(-1),
                    Err(_) => -1,
                };
                let status = if exit == 0 { "done" } else { "failed" };
                let _ = fs::write(
                    &sf,
                    format!(r#"{{"status":"{}","exit_code":{}}}"#, status, exit),
                );
                if let Ok(mut jobs) = JOBS.lock() {
                    if let Some(ref mut map) = *jobs {
                        map.remove(&jid);
                    }
                }
            });

            Ok(json!({
                "job_id": job_id,
                "pid": pid,
                "log": log_file,
                "status_file": status_file,
                "poll": format!("wsl_status(job_id='{}')", job_id)
            }))
        }
        Err(e) => {
            let _ = fs::write(
                &status_file,
                format!(r#"{{"status":"failed","error":"{}"}}"#, e),
            );
            Ok(json!({"error": format!("WSL spawn failed: {}", e)}))
        }
    }
}

pub async fn status(args: Value) -> Result<Value> {
    let job_id = args.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
    let tail_n = args.get("tail").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    if job_id.is_empty() {
        anyhow::bail!("job_id is required");
    }

    if job_id == "all" {
        let jobs = get_jobs();
        let mut result = Vec::new();
        if let Some(ref map) = *jobs {
            for (id, job) in map.iter() {
                let st = fs::read_to_string(&job.status_file).unwrap_or_default();
                result.push(json!({
                    "job_id": id,
                    "pid": job.pid,
                    "elapsed_secs": job.started.elapsed().as_secs(),
                    "status": st,
                }));
            }
        }
        if let Ok(entries) = fs::read_dir(wsl_state_dir()) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("wsl_bg_") && name.ends_with(".status") {
                    let jid = name
                        .trim_start_matches("wsl_bg_")
                        .trim_end_matches(".status");
                    if !result
                        .iter()
                        .any(|r| r.get("job_id").and_then(|v| v.as_str()) == Some(jid))
                    {
                        let st = fs::read_to_string(entry.path()).unwrap_or_default();
                        result.push(json!({"job_id": jid, "status": st, "completed": true}));
                    }
                }
            }
        }
        return Ok(json!({"jobs": result}));
    }

    let (log_file, status_file) = job_paths(job_id);

    let st = fs::read_to_string(&status_file)
        .unwrap_or_else(|_| r#"{"error":"job not found"}"#.to_string());

    let tail = if log_file.exists() {
        let content = fs::read_to_string(&log_file).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        let start = if lines.len() > tail_n {
            lines.len() - tail_n
        } else {
            0
        };
        lines[start..].join("\n")
    } else {
        String::new()
    };

    let total_lines = if log_file.exists() {
        fs::read_to_string(&log_file)
            .unwrap_or_default()
            .lines()
            .count()
    } else {
        0
    };

    Ok(json!({
        "job_id": job_id,
        "status": serde_json::from_str::<Value>(&st).unwrap_or(json!(st)),
        "total_lines": total_lines,
        "tail": tail,
    }))
}

pub async fn log_output(args: Value) -> Result<Value> {
    let job_id = args.get("job_id").and_then(|v| v.as_str()).unwrap_or("");
    if job_id.is_empty() {
        anyhow::bail!("job_id is required");
    }

    let (log_file, _) = job_paths(job_id);
    if !log_file.exists() {
        return Ok(json!({"error": format!("Log not found: {}", log_file.display())}));
    }

    let content = fs::read_to_string(&log_file)?;
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    let range = args
        .get("lines")
        .and_then(|v| v.as_str())
        .unwrap_or("last:50");

    let (start, end) = if range.starts_with("last:") {
        let n: usize = range.trim_start_matches("last:").parse().unwrap_or(50);
        let s = if total > n { total - n } else { 0 };
        (s, total)
    } else if let Some((a, b)) = range.split_once(':') {
        let s: usize = a.parse::<usize>().unwrap_or(1).saturating_sub(1);
        let e: usize = b.parse().unwrap_or(total);
        (s, e.min(total))
    } else {
        (0, total.min(50))
    };

    Ok(json!({
        "job_id": job_id,
        "total_lines": total,
        "range": format!("{}:{}", start + 1, end),
        "content": lines[start..end].join("\n"),
    }))
}
