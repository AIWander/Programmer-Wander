//! Shell execution + shell_session (simulated state session).
//!
//! 2026-07-29 rebuild (Stage 3): the 10 session_* verbs and their checkpoint/recover pair
//! collapsed into shell_session(action=...). The session's entire state is tiny (cwd + env +
//! history), so it is AUTO-PERSISTED to disk on every change and lazily reloaded on server
//! start - crash recovery is free instead of being 6 tools.
//!
//! Defects fixed during the rewrite (found in the Step 0 source review):
//! - session env vars were stored but never applied to spawned commands; now injected.
//! - session_read_output was a placeholder; history now records real output tails.
//! - shortcut_chain passed the wrong key and never worked; the merged shortcut fixes it,
//!   and gains params + session_id on chained runs.

use anyhow::Result;
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::info;

use super::{runtime, security};

const DEFAULT_TIMEOUT: u64 = 30;
const OUTPUT_TAIL_CHARS: usize = 2000;
// Session storage - lazily loads persisted sessions from the portable state directory,
// which is what makes recovery after a crash/restart transparent.
static SESSIONS: Lazy<Arc<Mutex<HashMap<String, Session>>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(runtime::state_path("shell_sessions")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(saved) = serde_json::from_str::<Value>(&content) {
                        if let Some(id) = saved["session_id"].as_str() {
                            map.insert(id.to_string(), Session::from_saved(&saved));
                        }
                    }
                }
            }
        }
    }
    Arc::new(Mutex::new(map))
});

struct Session {
    cwd: String,
    env: HashMap<String, String>,
    history: Vec<HistoryEntry>,
}

#[derive(Clone)]
struct HistoryEntry {
    command: String,
    exit_code: Option<i32>,
    timestamp: u64,
    output_tail: String,
}

impl Session {
    fn from_saved(saved: &Value) -> Session {
        let cwd = saved["cwd"].as_str().map(String::from).unwrap_or_else(|| {
            runtime::default_working_dir()
                .to_string_lossy()
                .into_owned()
        });
        let mut env = HashMap::new();
        if let Some(obj) = saved["env"].as_object() {
            for (k, v) in obj {
                if let Some(val) = v.as_str() {
                    env.insert(k.clone(), val.to_string());
                }
            }
        }
        let mut history = Vec::new();
        if let Some(arr) = saved["history"].as_array() {
            for h in arr {
                history.push(HistoryEntry {
                    command: h["command"].as_str().unwrap_or("").to_string(),
                    exit_code: h["exit_code"].as_i64().map(|c| c as i32),
                    timestamp: h["timestamp"].as_u64().unwrap_or(0),
                    output_tail: h["output_tail"].as_str().unwrap_or("").to_string(),
                });
            }
        }
        Session { cwd, env, history }
    }

    fn to_saved(&self, session_id: &str) -> Value {
        json!({
            "session_id": session_id,
            "cwd": self.cwd,
            "env": self.env,
            "history": self.history.iter().rev().take(50).rev().map(|h| json!({
                "command": h.command,
                "exit_code": h.exit_code,
                "timestamp": h.timestamp,
                "output_tail": h.output_tail,
            })).collect::<Vec<_>>(),
            "saved_at": chrono::Utc::now().to_rfc3339(),
        })
    }
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

fn persist(session_id: &str, session: &Session) {
    let state_dir = runtime::state_path("shell_sessions");
    let _ = std::fs::create_dir_all(&state_dir);
    let path = state_dir.join(format!("{}.json", sanitize_id(session_id)));
    if let Ok(data) = serde_json::to_string_pretty(&session.to_saved(session_id)) {
        let _ = std::fs::write(path, data);
    }
}

fn unpersist(session_id: &str) {
    let path =
        runtime::state_path("shell_sessions").join(format!("{}.json", sanitize_id(session_id)));
    let _ = std::fs::remove_file(path);
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Execute a Windows Command Prompt command (the `cmd` tool).
pub async fn execute(args: Value) -> Result<Value> {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let timeout_secs = args
        .get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT);
    let session_id = args.get("session_id").and_then(|v| v.as_str());

    if command.is_empty() {
        anyhow::bail!("command is required");
    }

    security::enforce_command_safety(command, "cmd")?;

    info!("Executing: {}", &command[..command.len().min(80)]);

    // Pull session cwd + env if a session is named
    let (cwd, env) = if let Some(sid) = session_id {
        let sessions = SESSIONS.lock().await;
        match sessions.get(sid) {
            Some(s) => (Some(s.cwd.clone()), s.env.clone()),
            None => (None, HashMap::new()),
        }
    } else {
        (None, HashMap::new())
    };

    let mut cmd = Command::new("cmd");
    cmd.args(["/C", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(dir) = &cwd {
        cmd.current_dir(dir);
    }
    // Session env vars are injected on top of the inherited environment
    for (k, v) in &env {
        cmd.env(k, v);
    }

    let result = timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code();
            let success = exit_code == Some(0);

            // Record in session history with a real output tail, then persist
            if let Some(sid) = session_id {
                let combined = if stderr.is_empty() {
                    stdout.clone()
                } else {
                    format!("{}\n[stderr] {}", stdout, stderr)
                };
                let tail: String = if combined.len() > OUTPUT_TAIL_CHARS {
                    let start = combined.len() - OUTPUT_TAIL_CHARS;
                    // avoid slicing mid-UTF-8-char
                    let mut idx = start;
                    while !combined.is_char_boundary(idx) {
                        idx += 1;
                    }
                    combined[idx..].to_string()
                } else {
                    combined
                };

                let mut sessions = SESSIONS.lock().await;
                if let Some(session) = sessions.get_mut(sid) {
                    session.history.push(HistoryEntry {
                        command: command.to_string(),
                        exit_code,
                        timestamp: now_unix(),
                        output_tail: tail,
                    });
                    persist(sid, session);
                }
            }

            Ok(json!({
                "success": success,
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": exit_code,
                "runtime": "completed"
            }))
        }
        Ok(Err(e)) => Ok(json!({
            "success": false,
            "error": format!("Execute failed: {}", e)
        })),
        Err(_) => Ok(json!({
            "success": false,
            "error": format!("Command timed out after {}s", timeout_secs)
        })),
    }
}

/// Execute a sequence of raw commands in a session, stop on first failure by default.
/// (Internal engine for shortcut mode=run with commands=[]; formerly the `chain` tool.)
async fn run_commands(commands: &[String], session_id: &str, stop_on_error: bool) -> Result<Value> {
    let mut results = Vec::new();
    let mut all_success = true;
    let mut failed_at: Option<usize> = None;

    for (i, cmd) in commands.iter().enumerate() {
        let result = execute(json!({
            "command": cmd,
            "session_id": session_id
        }))
        .await?;

        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        results.push(result);

        if !success {
            all_success = false;
            failed_at = Some(i);
            if stop_on_error {
                break;
            }
        }
    }

    Ok(json!({
        "success": all_success,
        "results": results,
        "failed_at": failed_at,
        "commands_run": results.len()
    }))
}

/// Create a state session.
pub async fn create_session(args: Value) -> Result<Value> {
    let name = args.get("name").and_then(|v| v.as_str());
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| {
            runtime::default_working_dir()
                .to_string_lossy()
                .into_owned()
        });

    let session_id = name
        .map(|n| sanitize_id(n))
        .unwrap_or_else(|| format!("session_{:08x}", rand::random::<u32>()));

    let mut sessions = SESSIONS.lock().await;
    let session = Session {
        cwd: cwd.clone(),
        env: HashMap::new(),
        history: Vec::new(),
    };
    persist(&session_id, &session);
    sessions.insert(session_id.clone(), session);

    info!("Created session: {}", session_id);

    Ok(json!({
        "success": true,
        "session_id": session_id,
        "cwd": cwd,
        "persisted": true
    }))
}

// Predefined shortcuts
fn get_shortcuts() -> HashMap<&'static str, Vec<&'static str>> {
    let mut shortcuts = HashMap::new();
    shortcuts.insert(
        "git_commit_push",
        vec!["git add -A", "git commit -m \"$message\"", "git push"],
    );
    shortcuts.insert(
        "npm_build_deploy",
        vec!["npm install", "npm run build", "npm run deploy"],
    );
    shortcuts.insert(
        "pip_install_freeze",
        vec!["pip install $packages", "pip freeze > requirements.txt"],
    );
    shortcuts.insert(
        "python_venv_activate",
        vec!["python -m venv .venv", ".venv\\Scripts\\activate"],
    );
    shortcuts
}

fn substitute_params(commands: &[&str], params: &Value) -> Vec<String> {
    commands
        .iter()
        .map(|cmd| {
            let mut result = cmd.to_string();
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    let placeholder = format!("${}", key);
                    if let Some(v) = value.as_str() {
                        result = result.replace(&placeholder, v);
                    }
                }
            }
            result
        })
        .collect()
}

/// shortcut(mode=run|list, names=[], commands=[], params=, session_id=, stop_on_error=)
/// Absorbs the former shortcut / chain / shortcut_chain / list_shortcut.
/// Axis is saved-name versus raw-command: names=[] runs saved shortcuts (params substituted),
/// commands=[] runs raw commands; both run inside session_id with stop_on_error control.
pub async fn shortcut(args: Value) -> Result<Value> {
    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("run");

    if mode == "list" {
        let shortcuts = get_shortcuts();
        let list: Vec<Value> = shortcuts
            .iter()
            .map(|(name, cmds)| {
                json!({
                    "name": name,
                    "commands": cmds,
                    "description": format!("{} step workflow", cmds.len())
                })
            })
            .collect();
        return Ok(json!({
            "success": true,
            "shortcuts": list,
            "count": list.len()
        }));
    }

    if mode != "run" {
        return Ok(json!({"success": false, "error": "mode must be run or list"}));
    }

    let session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let stop_on_error = args
        .get("stop_on_error")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let params = &args["params"];

    // Legacy single-name compatibility: shortcut_name= folds into names=[]
    let mut names: Vec<String> = args
        .get("names")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    if names.is_empty() {
        if let Some(n) = args.get("shortcut_name").and_then(|v| v.as_str()) {
            names.push(n.to_string());
        }
    }

    let raw_commands: Vec<String> = args
        .get("commands")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    if names.is_empty() && raw_commands.is_empty() {
        anyhow::bail!("mode=run requires names=[] (saved shortcuts) or commands=[] (raw commands)");
    }

    // Expand saved shortcuts to their command lists, then append raw commands
    let shortcuts = get_shortcuts();
    let mut all_commands: Vec<String> = Vec::new();
    let mut expanded: Vec<Value> = Vec::new();

    for name in &names {
        match shortcuts.get(name.as_str()) {
            Some(cmds) => {
                let substituted = substitute_params(cmds, params);
                expanded.push(json!({"shortcut": name, "commands": substituted}));
                all_commands.extend(substituted);
            }
            None => {
                return Ok(json!({
                    "success": false,
                    "error": format!("Unknown shortcut: {}", name),
                    "available": shortcuts.keys().collect::<Vec<_>>()
                }));
            }
        }
    }
    all_commands.extend(raw_commands);

    // Validate the complete expanded sequence before executing its first command.
    for command in &all_commands {
        security::enforce_command_safety(command, "shortcut")?;
    }

    let mut result = run_commands(&all_commands, session_id, stop_on_error).await?;
    if let Some(obj) = result.as_object_mut() {
        if !expanded.is_empty() {
            obj.insert("shortcuts_expanded".to_string(), json!(expanded));
        }
        obj.insert("session_id".to_string(), json!(session_id));
    }
    Ok(result)
}

/// shell_session(action=create|run|list|destroy|cd|env|history|read)
/// The simulated state session: remembered cwd + env applied to each fresh command.
/// State auto-persists below PROGRAMMER_STATE_DIR on every change.
pub async fn shell_session(args: Value) -> Result<Value> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");

    match action {
        "create" => create_session(args).await,

        "run" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            if command.is_empty() {
                anyhow::bail!("command is required");
            }
            security::enforce_command_safety(command, "shell_session")?;
            // Auto-create the session on first run so run-before-create just works
            {
                let sessions = SESSIONS.lock().await;
                if !sessions.contains_key(session_id) {
                    drop(sessions);
                    let _ = create_session(json!({"name": session_id})).await;
                }
            }
            execute(json!({
                "command": command,
                "timeout": args.get("timeout").cloned().unwrap_or(json!(DEFAULT_TIMEOUT)),
                "session_id": session_id
            }))
            .await
        }

        "list" => {
            let sessions = SESSIONS.lock().await;
            let list: Vec<Value> = sessions
                .iter()
                .map(|(id, s)| {
                    json!({
                        "session_id": id,
                        "cwd": s.cwd,
                        "env_count": s.env.len(),
                        "history_count": s.history.len()
                    })
                })
                .collect();
            Ok(json!({
                "success": true,
                "sessions": list,
                "count": list.len()
            }))
        }

        "destroy" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if session_id.is_empty() {
                anyhow::bail!("session_id is required");
            }
            let mut sessions = SESSIONS.lock().await;
            if sessions.remove(session_id).is_some() {
                unpersist(session_id);
                Ok(json!({"success": true, "session_id": session_id}))
            } else {
                Ok(json!({"success": false, "error": "Session not found"}))
            }
        }

        "cd" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            if session_id.is_empty() || path.is_empty() {
                anyhow::bail!("session_id and path are required");
            }
            if !std::path::Path::new(path).exists() {
                anyhow::bail!("Directory does not exist: {}", path);
            }
            let mut sessions = SESSIONS.lock().await;
            match sessions.get_mut(session_id) {
                Some(session) => {
                    session.cwd = path.to_string();
                    persist(session_id, session);
                    Ok(json!({"success": true, "session_id": session_id, "cwd": path}))
                }
                None => anyhow::bail!("Session not found: {}", session_id),
            }
        }

        "env" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let key = args.get("key").and_then(|v| v.as_str());
            let value = args.get("value").and_then(|v| v.as_str());

            let mut sessions = SESSIONS.lock().await;
            let session = match sessions.get_mut(session_id) {
                Some(s) => s,
                None => anyhow::bail!("Session not found: {}", session_id),
            };

            match (key, value) {
                (Some(k), Some(v)) => {
                    session.env.insert(k.to_string(), v.to_string());
                    persist(session_id, session);
                    Ok(json!({"success": true, "session_id": session_id, "key": k, "value": v}))
                }
                (Some(k), None) => {
                    let session_value = session.env.get(k).cloned();
                    let value = session_value.or_else(|| std::env::var(k).ok());
                    Ok(json!({"success": true, "key": k, "value": value}))
                }
                (None, _) => {
                    Ok(json!({"success": true, "session_id": session_id, "env": session.env}))
                }
            }
        }

        "history" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
            let sessions = SESSIONS.lock().await;
            match sessions.get(session_id) {
                Some(session) => {
                    let history: Vec<Value> = session
                        .history
                        .iter()
                        .rev()
                        .take(limit)
                        .map(|h| {
                            json!({
                                "command": h.command,
                                "exit_code": h.exit_code,
                                "timestamp": h.timestamp
                            })
                        })
                        .collect();
                    Ok(json!({
                        "success": true,
                        "session_id": session_id,
                        "history": history,
                        "count": history.len()
                    }))
                }
                None => Ok(json!({"success": false, "error": "Session not found"})),
            }
        }

        "read" => {
            let session_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("default");
            let last = args.get("last").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
            let sessions = SESSIONS.lock().await;
            match sessions.get(session_id) {
                Some(session) => {
                    let outputs: Vec<Value> = session
                        .history
                        .iter()
                        .rev()
                        .take(last)
                        .map(|h| {
                            json!({
                                "command": h.command,
                                "exit_code": h.exit_code,
                                "output_tail": h.output_tail
                            })
                        })
                        .collect();
                    Ok(json!({
                        "success": true,
                        "session_id": session_id,
                        "outputs": outputs,
                        "count": outputs.len()
                    }))
                }
                None => Ok(json!({"success": false, "error": "Session not found"})),
            }
        }

        _ => anyhow::bail!(
            "action must be one of: create, run, list, destroy, cd, env, history, read"
        ),
    }
}

pub async fn powershell(args: Value) -> Result<Value> {
    let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let timeout_secs = args
        .get("timeout_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30);

    if command.is_empty() {
        anyhow::bail!("command is required");
    }

    security::enforce_command_safety(command, "powershell")?;

    info!("PowerShell: {}", &command[..command.len().min(80)]);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok(json!({
                "exit_code": output.status.code().unwrap_or(-1),
                "stdout": stdout.trim(),
                "stderr": stderr.trim(),
                "success": output.status.success()
            }))
        }
        Ok(Err(e)) => Ok(json!({"error": e.to_string()})),
        Err(_) => Ok(json!({"error": format!("Timed out after {}s", timeout_secs)})),
    }
}

pub async fn md2docx(args: Value) -> Result<Value> {
    let input = args.get("input").and_then(|v| v.as_str()).unwrap_or("");
    let output = args.get("output").and_then(|v| v.as_str()).unwrap_or("");

    if input.is_empty() || output.is_empty() {
        anyhow::bail!("input and output are required");
    }

    let result = tokio::process::Command::new("pandoc")
        .args([input, "-o", output])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match result {
        Ok(out) => {
            if out.status.success() {
                Ok(json!({"success": true, "output": output}))
            } else {
                Ok(json!({"error": String::from_utf8_lossy(&out.stderr).to_string()}))
            }
        }
        Err(e) => Ok(json!({"error": e.to_string()})),
    }
}
