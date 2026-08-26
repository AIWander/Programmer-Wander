//! System Operations (clipboard, processes, screenshots, system info, doctor)
//! 2026-07-29 rebuild: resource monitoring relocated to autonomous (pulse group);
//! notify removed (local/ops/manager cover it); tail_file absorbed into read_file(tail=).

use anyhow::Result;
use serde_json::{json, Value};
use tokio::process::Command;
use tracing::info;

use super::runtime;

/// Get system information
pub async fn get_info() -> Result<Value> {
    let hostname = std::env::var("COMPUTERNAME").unwrap_or_default();
    let user = std::env::var("USERNAME").unwrap_or_default();
    let home = std::env::var("USERPROFILE").unwrap_or_default();

    // Get memory info via PowerShell
    let mem_output = Command::new("powershell")
        .args(["-Command", "(Get-CimInstance Win32_OperatingSystem | Select-Object FreePhysicalMemory,TotalVisibleMemorySize | ConvertTo-Json)"])
        .output()
        .await
        .ok();

    let (free_mem, total_mem) = if let Some(out) = mem_output {
        let json_str = String::from_utf8_lossy(&out.stdout);
        if let Ok(v) = serde_json::from_str::<Value>(&json_str) {
            let free = v
                .get("FreePhysicalMemory")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                * 1024;
            let total = v
                .get("TotalVisibleMemorySize")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                * 1024;
            (free, total)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    // Get CPU info
    let cpu_output = Command::new("powershell")
        .args(["-Command", "(Get-CimInstance Win32_Processor).Name"])
        .output()
        .await
        .ok();

    let cpu_name = cpu_output
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    Ok(json!({
        "success": true,
        "hostname": hostname,
        "user": user,
        "home": home,
        "os": "Windows",
        "arch": std::env::consts::ARCH,
        "cpu": cpu_name,
        "memory": {
            "total_bytes": total_mem,
            "free_bytes": free_mem,
            "used_percent": if total_mem > 0 {
                ((total_mem - free_mem) as f64 / total_mem as f64 * 100.0).round()
            } else { 0.0 }
        },
        "server": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Doctor - per-host capability self-report (2026-07-29 rebuild).
/// One call answers "what can this server actually do on this machine":
/// git present, WSL present (+distros), shells available, python/pandoc/cargo, state dirs.
pub async fn doctor() -> Result<Value> {
    async fn probe(cmd: &str, args: &[&str]) -> Option<String> {
        let out = Command::new(cmd)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .await
            .ok()?;
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            Some(text.lines().next().unwrap_or("").trim().to_string())
        } else {
            None
        }
    }

    let git = probe("git", &["--version"]).await;
    let powershell = probe(
        "powershell",
        &[
            "-NoProfile",
            "-Command",
            "$PSVersionTable.PSVersion.ToString()",
        ],
    )
    .await;
    let pwsh = probe(
        "pwsh",
        &[
            "-NoProfile",
            "-Command",
            "$PSVersionTable.PSVersion.ToString()",
        ],
    )
    .await;
    let bash = probe("bash", &["--version"]).await;
    let python = probe("python", &["--version"]).await;
    let pandoc = probe("pandoc", &["--version"]).await;
    let cargo = probe("cargo", &["--version"]).await;

    // wsl -l -q emits UTF-16LE; decode manually and strip NULs
    let wsl_distros: Vec<String> = match Command::new("wsl")
        .args(["-l", "-q"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .await
    {
        Ok(out) if out.status.success() => {
            let utf16: Vec<u16> = out
                .stdout
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&utf16)
                .lines()
                .map(|l| l.trim().trim_matches('\0').to_string())
                .filter(|l| !l.is_empty())
                .collect()
        }
        _ => Vec::new(),
    };

    let state_dir = runtime::state_dir();
    let state_writable = std::fs::create_dir_all(&state_dir).is_ok();

    Ok(json!({
        "server": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "profile": "canonical-49",
        "os": "Windows",
        "arch": std::env::consts::ARCH,
        "git": {"present": git.is_some(), "version": git,
                 "note": "git operations remain available through cmd or PowerShell when git is on PATH"},
        "wsl": {"present": !wsl_distros.is_empty(), "distros": wsl_distros},
        "shells": {
            "powershell": powershell,
            "pwsh": pwsh,
            "cmd": true,
            "bash": bash
        },
        "runtimes": {
            "python": python,
            "pandoc": pandoc,
            "cargo": cargo
        },
        "state_dir": {"path": state_dir, "writable": state_writable},
        "notes": {
            "python": "transform_file requires python on PATH",
            "pandoc": "md2docx requires pandoc on PATH"
        }
    }))
}

/// Read clipboard
pub async fn clipboard_read() -> Result<Value> {
    match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
        Ok(content) => Ok(json!({
            "success": true,
            "content": content
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("Clipboard read failed: {}", e)
        })),
    }
}

/// Write to clipboard
pub async fn clipboard_write(args: Value) -> Result<Value> {
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(content.to_string())) {
        Ok(()) => Ok(json!({
            "success": true,
            "size": content.len()
        })),
        Err(e) => Ok(json!({
            "success": false,
            "error": format!("Clipboard write failed: {}", e)
        })),
    }
}

/// List processes
pub async fn list_processes(args: Value) -> Result<Value> {
    let filter = args.get("filter_name").and_then(|v| v.as_str());

    let ps_cmd = if let Some(f) = filter {
        format!("Get-Process | Where-Object {{$_.Name -like '*{}*'}} | Select-Object Id,Name,CPU,WorkingSet64 -First 50 | ConvertTo-Json", f)
    } else {
        "Get-Process | Select-Object Id,Name,CPU,WorkingSet64 -First 50 | ConvertTo-Json"
            .to_string()
    };

    let output = Command::new("powershell")
        .args(["-Command", &ps_cmd])
        .output()
        .await?;

    let json_str = String::from_utf8_lossy(&output.stdout);
    let processes: Value = serde_json::from_str(&json_str).unwrap_or(json!([]));

    // Normalize to array (single result comes as object)
    let processes = if processes.is_array() {
        processes
    } else if processes.is_object() {
        json!([processes])
    } else {
        json!([])
    };

    Ok(json!({
        "success": true,
        "processes": processes,
        "count": processes.as_array().map(|a| a.len()).unwrap_or(0)
    }))
}

/// Kill process by PID
pub async fn kill_process(args: Value) -> Result<Value> {
    let pid = args.get("pid").and_then(|v| v.as_u64()).unwrap_or(0);

    if pid == 0 {
        anyhow::bail!("pid is required");
    }

    let output = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
        .await?;

    if output.status.success() {
        info!("Killed process {}", pid);
        Ok(json!({
            "success": true,
            "pid": pid
        }))
    } else {
        Ok(json!({
            "success": false,
            "error": String::from_utf8_lossy(&output.stderr).to_string()
        }))
    }
}

/// Test TCP connectivity to a host:port
pub async fn port_check(args: Value) -> Result<Value> {
    let host = args
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("127.0.0.1");
    let port = match args.get("port").and_then(|v| v.as_u64()) {
        Some(p) => p as u16,
        None => anyhow::bail!("port required"),
    };
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(2000);

    let addr = format!("{}:{}", host, port);
    let socket_addr: std::net::SocketAddr = addr
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address {}: {}", addr, e))?;

    let timeout_dur = std::time::Duration::from_millis(timeout_ms);

    // Run blocking TCP connect in spawn_blocking
    let host_owned = host.to_string();
    tokio::task::spawn_blocking(move || {
        let start = std::time::Instant::now();
        match std::net::TcpStream::connect_timeout(&socket_addr, timeout_dur) {
            Ok(_) => {
                let elapsed_ms = start.elapsed().as_millis();
                Ok(json!({
                    "open": true,
                    "host": host_owned,
                    "port": port,
                    "connect_time_ms": elapsed_ms,
                }))
            }
            Err(e) => {
                let elapsed_ms = start.elapsed().as_millis();
                Ok(json!({
                    "open": false,
                    "host": host_owned,
                    "port": port,
                    "error": e.to_string(),
                    "elapsed_ms": elapsed_ms,
                }))
            }
        }
    })
    .await?
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Take a screenshot for troubleshooting. Returns path + metadata only (no raw bytes).
/// Refuses if the resulting file exceeds 1MB.
pub fn screenshot(args: &Value) -> Value {
    let save_path = args.get("save_path").and_then(|v| v.as_str());
    let quality = args.get("quality").and_then(|v| v.as_u64()).unwrap_or(60) as u8;
    let scale = args.get("scale").and_then(|v| v.as_f64()).unwrap_or(0.75);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let path = save_path.map(String::from).unwrap_or_else(|| {
        runtime::state_path("screenshots")
            .join(format!("screenshot_{}.jpg", timestamp))
            .to_string_lossy()
            .into_owned()
    });

    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let ps_script = format!(
        r#"
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
$g = [System.Drawing.Graphics]::FromImage($bitmap)
$g.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
$w = [int]($screen.Width * {scale})
$h = [int]($screen.Height * {scale})
$scaled = New-Object System.Drawing.Bitmap($w, $h)
$gs = [System.Drawing.Graphics]::FromImage($scaled)
$gs.DrawImage($bitmap, 0, 0, $w, $h)
$enc = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object {{ $_.MimeType -eq 'image/jpeg' }}
$ep = New-Object System.Drawing.Imaging.EncoderParameters(1)
$ep.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter([System.Drawing.Imaging.Encoder]::Quality, {quality})
$scaled.Save('{path}', $enc, $ep)
$scaled.Dispose(); $bitmap.Dispose()
"#,
        scale = scale,
        quality = quality,
        path = path.replace('\\', "\\\\")
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if size > 1_048_576 {
                let _ = std::fs::remove_file(&path);
                return json!({"error": format!("Screenshot too large ({} bytes). Lower quality or scale.", size)});
            }
            json!({"success": true, "path": path, "size_bytes": size, "quality": quality, "scale": scale})
        }
        Ok(o) => json!({"error": String::from_utf8_lossy(&o.stderr).trim().to_string()}),
        Err(e) => json!({"error": e.to_string()}),
    }
}
