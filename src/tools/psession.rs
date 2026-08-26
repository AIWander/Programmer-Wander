//! live_shell - real persistent shell processes (PowerShell and WSL backends).
//!
//! The legacy psession_* tools collapsed into live_shell(action=...), with checkpoint/recover.
//! You cannot
//! freeze a live process; a checkpoint records what is sufficient to RECREATE an
//! equivalent process after a crash or restart - backend, cwd (probed live when the
//! process responds, creation value otherwise), an env snapshot (probed live, stored for
//! inspection and selective replay - not blindly re-applied), and command history.
//! That is the honest, implementable form of "recover a live shell".

use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use tracing::info;

use super::{runtime, security};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const DEFAULT_DISTRO: &str = "Ubuntu-24.04";
const ENV_SNAPSHOT_MAX_LINES: usize = 200;

static PSESSIONS: Lazy<Arc<Mutex<HashMap<String, PersistentSession>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

struct PersistentSession {
    name: String,
    shell_type: String, // "powershell" or "wsl"
    distro: String,
    creation_cwd: String,
    child: Child,
    output_buffer: Arc<Mutex<Vec<String>>>,
    history: Vec<String>,
    created_at: String,
}

fn start_reader(stream: impl std::io::Read + Send + 'static, buffer: Arc<Mutex<Vec<String>>>) {
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().flatten() {
            buffer.lock().unwrap().push(line);
        }
    });
}

fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

async fn create_internal(name: &str, shell: &str, cwd: &str, distro: &str) -> Result<Value> {
    info!("Creating live {} shell: {}", shell, name);

    let mut cmd = match shell {
        "wsl" => {
            let mut c = Command::new("wsl");
            c.args(["-d", distro, "--", "bash"]);
            c
        }
        _ => {
            let mut c = Command::new("powershell");
            c.args(["-NoLogo", "-NoProfile", "-Command", "-"]);
            c
        }
    };

    cmd.current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn {}: {}", shell, e))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to take stdout"))?;

    let buffer = Arc::new(Mutex::new(Vec::new()));
    start_reader(stdout, buffer.clone());

    // Also capture stderr into the same buffer
    if let Some(stderr) = child.stderr.take() {
        start_reader(stderr, buffer.clone());
    }

    thread::sleep(std::time::Duration::from_millis(200));

    let session_id = sanitize_id(name);
    let created = chrono::Local::now().to_rfc3339();

    let mut sessions = PSESSIONS.lock().unwrap();
    sessions.insert(
        session_id.clone(),
        PersistentSession {
            name: name.to_string(),
            shell_type: shell.to_string(),
            distro: distro.to_string(),
            creation_cwd: cwd.to_string(),
            child,
            output_buffer: buffer,
            history: Vec::new(),
            created_at: created.clone(),
        },
    );

    Ok(json!({
        "session_id": session_id,
        "shell": shell,
        "name": name,
        "cwd": cwd,
        "distro": if shell == "wsl" { json!(distro) } else { Value::Null },
        "created_at": created
    }))
}

/// Run a command in a live session; returns (output, completed).
/// record_history=false is used by checkpoint probes so probing does not pollute history.
fn run_internal(
    session_id: &str,
    command: &str,
    timeout_secs: u64,
    record_history: bool,
) -> Result<(String, bool)> {
    security::enforce_command_safety(command, "live_shell")?;

    let mut sessions = PSESSIONS.lock().unwrap();
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

    // Record pre-command buffer position
    let start_pos = session.output_buffer.lock().unwrap().len();

    // Write command to stdin with a completion marker
    let marker = format!(
        "__DONE_{}__",
        uuid::Uuid::new_v4()
            .to_string()
            .get(..8)
            .unwrap_or("00000000")
    );
    let stdin = session
        .child
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("stdin not available"))?;

    let full_cmd = if session.shell_type == "wsl" {
        format!("{}\necho {}\n", command, marker)
    } else {
        format!("{}\nWrite-Output '{}'\n", command, marker)
    };

    stdin
        .write_all(full_cmd.as_bytes())
        .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
    stdin
        .flush()
        .map_err(|e| anyhow::anyhow!("Flush failed: {}", e))?;

    if record_history {
        session.history.push(command.to_string());
    }

    // Wait for marker with timeout; drop the sessions lock while waiting
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let buffer = session.output_buffer.clone();
    drop(sessions);

    let mut output_lines = Vec::new();
    let mut found_marker = false;

    loop {
        if std::time::Instant::now() > deadline {
            break;
        }

        {
            let buf = buffer.lock().unwrap();
            let current_len = buf.len();
            if current_len > start_pos {
                for i in start_pos..current_len {
                    if buf[i].contains(&marker) {
                        found_marker = true;
                        output_lines = buf[start_pos..i].to_vec();
                        break;
                    }
                }
                if found_marker {
                    break;
                }
            }
        }

        thread::sleep(std::time::Duration::from_millis(50));
    }

    if !found_marker {
        let buf = buffer.lock().unwrap();
        if buf.len() > start_pos {
            output_lines = buf[start_pos..].to_vec();
        }
    }

    Ok((output_lines.join("\n"), found_marker))
}

fn default_checkpoint_path(session_id: &str) -> String {
    runtime::state_path("live_shells")
        .join(format!("{}.checkpoint.json", sanitize_id(session_id)))
        .to_string_lossy()
        .into_owned()
}

/// live_shell(action=create|run|read|history|list|destroy|checkpoint|recover)
pub async fn live_shell(args: Value) -> Result<Value> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

    match action {
        "create" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let shell = args
                .get("shell")
                .and_then(|v| v.as_str())
                .unwrap_or("powershell");
            let default_cwd = std::env::var("WORKSPACE_PATH")
                .ok()
                .filter(|path| std::path::Path::new(path).is_dir())
                .unwrap_or_else(|| {
                    runtime::default_working_dir()
                        .to_string_lossy()
                        .into_owned()
                });
            let cwd = args
                .get("cwd")
                .and_then(|v| v.as_str())
                .unwrap_or(&default_cwd);
            let distro = args
                .get("distro")
                .and_then(|v| v.as_str())
                .unwrap_or(DEFAULT_DISTRO);
            create_internal(name, shell, cwd, distro).await
        }

        "run" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let timeout_secs = args
                .get("timeout_secs")
                .and_then(|v| v.as_u64())
                .unwrap_or(30);

            if session_id.is_empty() || command.is_empty() {
                anyhow::bail!("session_id and command are required");
            }

            security::enforce_command_safety(command, "live_shell")?;
            info!(
                "live_shell run [{}]: {}",
                session_id,
                &command[..command.len().min(80)]
            );

            let (output, completed) = run_internal(session_id, command, timeout_secs, true)?;
            Ok(json!({
                "session_id": session_id,
                "output": output,
                "lines": output.lines().count(),
                "completed": completed,
                "timed_out": !completed
            }))
        }

        "read" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let tail_n = args.get("tail").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

            if session_id.is_empty() {
                anyhow::bail!("session_id is required");
            }

            let sessions = PSESSIONS.lock().unwrap();
            let session = sessions
                .get(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

            let buf = session.output_buffer.lock().unwrap();
            let total = buf.len();
            let start = if total > tail_n { total - tail_n } else { 0 };

            Ok(json!({
                "session_id": session_id,
                "total_lines": total,
                "tail": buf[start..].join("\n"),
            }))
        }

        "history" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if session_id.is_empty() {
                anyhow::bail!("session_id is required");
            }

            let sessions = PSESSIONS.lock().unwrap();
            let session = sessions
                .get(session_id)
                .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

            Ok(json!({
                "session_id": session_id,
                "history": session.history,
                "count": session.history.len()
            }))
        }

        "list" => {
            let sessions = PSESSIONS.lock().unwrap();
            let list: Vec<Value> = sessions
                .iter()
                .map(|(id, s)| {
                    json!({
                        "session_id": id,
                        "name": s.name,
                        "shell": s.shell_type,
                        "history_count": s.history.len(),
                        "buffer_lines": s.output_buffer.lock().unwrap().len(),
                        "created_at": s.created_at,
                    })
                })
                .collect();

            Ok(json!({"sessions": list, "count": list.len()}))
        }

        "destroy" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if session_id.is_empty() {
                anyhow::bail!("session_id is required");
            }

            let mut sessions = PSESSIONS.lock().unwrap();
            if let Some(mut session) = sessions.remove(session_id) {
                let _ = session.child.kill();
                Ok(json!({"destroyed": session_id}))
            } else {
                Ok(json!({"error": format!("Session not found: {}", session_id)}))
            }
        }

        "checkpoint" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if session_id.is_empty() {
                anyhow::bail!("session_id is required");
            }
            let default_path = default_checkpoint_path(session_id);
            let checkpoint_path = args
                .get("checkpoint_path")
                .and_then(|v| v.as_str())
                .unwrap_or(&default_path);

            // Snapshot static fields first
            let (name, shell_type, distro, creation_cwd, history) = {
                let sessions = PSESSIONS.lock().unwrap();
                let session = sessions
                    .get(session_id)
                    .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;
                (
                    session.name.clone(),
                    session.shell_type.clone(),
                    session.distro.clone(),
                    session.creation_cwd.clone(),
                    session.history.clone(),
                )
            };

            // Probe the live process for current cwd and an env snapshot.
            // If the process is wedged, fall back to creation values.
            let (cwd_cmd, env_cmd) = if shell_type == "wsl" {
                ("pwd", "env")
            } else {
                (
                    "(Get-Location).Path",
                    "Get-ChildItem env: | ForEach-Object { \"$($_.Name)=$($_.Value)\" }",
                )
            };

            let (probed_cwd, cwd_ok) =
                run_internal(session_id, cwd_cmd, 5, false).unwrap_or((String::new(), false));
            let (probed_env, env_ok) =
                run_internal(session_id, env_cmd, 5, false).unwrap_or((String::new(), false));

            let cwd = if cwd_ok && !probed_cwd.trim().is_empty() {
                probed_cwd
                    .trim()
                    .lines()
                    .last()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            } else {
                creation_cwd.clone()
            };
            let env_lines: Vec<String> = if env_ok {
                probed_env
                    .lines()
                    .take(ENV_SNAPSHOT_MAX_LINES)
                    .map(String::from)
                    .collect()
            } else {
                Vec::new()
            };

            let checkpoint = json!({
                "kind": "live_shell_checkpoint",
                "session_id": session_id,
                "name": name,
                "shell": shell_type,
                "distro": distro,
                "cwd": cwd,
                "cwd_probed_live": cwd_ok,
                "env": env_lines,
                "env_probed_live": env_ok,
                "history": history,
                "saved_at": chrono::Utc::now().to_rfc3339(),
            });

            if let Some(parent) = std::path::Path::new(checkpoint_path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(checkpoint_path, serde_json::to_string_pretty(&checkpoint)?) {
                Ok(_) => Ok(json!({
                    "success": true,
                    "checkpoint_path": checkpoint_path,
                    "session_id": session_id,
                    "cwd": checkpoint["cwd"],
                    "cwd_probed_live": cwd_ok,
                    "env_lines": env_lines.len(),
                    "commands_saved": checkpoint["history"].as_array().map(|a| a.len()).unwrap_or(0)
                })),
                Err(e) => Ok(json!({"error": format!("Failed to write checkpoint: {}", e)})),
            }
        }

        "recover" => {
            // Accept an explicit checkpoint_path, or derive it from session_id
            let path = match args.get("checkpoint_path").and_then(|v| v.as_str()) {
                Some(p) => p.to_string(),
                None => {
                    let sid = args
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if sid.is_empty() {
                        anyhow::bail!("checkpoint_path or session_id is required");
                    }
                    default_checkpoint_path(sid)
                }
            };

            let data = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("Failed to read checkpoint {}: {}", path, e))?;
            let checkpoint: Value = serde_json::from_str(&data)
                .map_err(|e| anyhow::anyhow!("Invalid checkpoint format: {}", e))?;

            let name = checkpoint["name"].as_str().unwrap_or("recovered");
            let shell = checkpoint["shell"].as_str().unwrap_or("powershell");
            let distro = checkpoint["distro"].as_str().unwrap_or(DEFAULT_DISTRO);
            let fallback_cwd = runtime::default_working_dir()
                .to_string_lossy()
                .into_owned();
            let cwd = checkpoint["cwd"].as_str().unwrap_or(&fallback_cwd);

            // Recreate an equivalent process: same backend + cwd (+distro)
            let created = create_internal(name, shell, cwd, distro).await?;
            let new_id = created["session_id"].as_str().unwrap_or("").to_string();

            // Restore history as context (not replayed - replaying arbitrary past commands
            // is not safe; the checkpointed env snapshot is returned for selective replay)
            let history: Vec<String> = checkpoint["history"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            {
                let mut sessions = PSESSIONS.lock().unwrap();
                if let Some(session) = sessions.get_mut(&new_id) {
                    session.history = history.clone();
                }
            }

            Ok(json!({
                "success": true,
                "session_id": new_id,
                "recovered_from": path,
                "saved_at": checkpoint["saved_at"],
                "shell": shell,
                "cwd": cwd,
                "commands_restored": history.len(),
                "env_snapshot_lines": checkpoint["env"].as_array().map(|a| a.len()).unwrap_or(0),
                "note": "History restored as context; env snapshot available in the checkpoint for selective replay. Nothing was auto-replayed."
            }))
        }

        _ => anyhow::bail!(
            "action must be one of: create, run, read, history, list, destroy, checkpoint, recover"
        ),
    }
}
