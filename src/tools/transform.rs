//! Transform - Archive, sync, bulk file operations + Token-saving utilities
//! High-performance file operations at scale

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

// ============ TOKEN-SAVING TRANSFORMS (from mcp-windows) ============

// ---- Mode-flip survivors (2026-07-29 rebuild): base64 / json / convert ----
// Three entries that differed only by a direction word collapsed into mode params.

/// base64(mode=encode|decode, input)
pub async fn base64_tool(args: Value) -> Result<Value> {
    let mode = args["mode"].as_str().unwrap_or("encode");
    let input = args["input"]
        .as_str()
        .or(args["text"].as_str())
        .or(args["encoded"].as_str())
        .unwrap_or("");
    match mode {
        "encode" => base64_encode(json!({"text": input})).await,
        "decode" => base64_decode(json!({"encoded": input})).await,
        _ => Ok(json!({"error": "mode must be encode or decode"})),
    }
}

/// json(mode=format|minify, input, indent)
pub async fn json_tool(args: Value) -> Result<Value> {
    let mode = args["mode"].as_str().unwrap_or("format");
    let input = args["input"]
        .as_str()
        .or(args["json_string"].as_str())
        .unwrap_or("");
    match mode {
        "format" => json_format(json!({"json_string": input, "indent": args["indent"]})).await,
        "minify" => json_minify(json!({"json_string": input})).await,
        _ => Ok(json!({"error": "mode must be format or minify"})),
    }
}

/// convert(from=csv|json, to=json|csv, input, delimiter)
pub async fn convert_tool(args: Value) -> Result<Value> {
    let from = args["from"].as_str().unwrap_or("");
    let to = args["to"].as_str().unwrap_or("");
    let input = args["input"]
        .as_str()
        .or(args["csv_string"].as_str())
        .or(args["json_array"].as_str())
        .unwrap_or("");
    let delimiter = args["delimiter"].clone();
    match (from, to) {
        ("csv", "json") => csv_to_json(json!({"csv_string": input, "delimiter": delimiter})).await,
        ("json", "csv") => json_to_csv(json!({"json_array": input, "delimiter": delimiter})).await,
        _ => Ok(json!({"error": "supported conversions: from=csv,to=json or from=json,to=csv"})),
    }
}

/// Pretty-print JSON
pub async fn json_format(args: Value) -> Result<Value> {
    let json_string = args["json_string"].as_str().unwrap_or("");
    let _indent = args["indent"].as_u64().unwrap_or(2) as usize;

    match serde_json::from_str::<Value>(json_string) {
        Ok(parsed) => {
            let formatted = serde_json::to_string_pretty(&parsed)?;
            Ok(json!({"formatted": formatted}))
        }
        Err(e) => Ok(json!({"error": format!("Invalid JSON: {}", e)})),
    }
}

/// Minify JSON
pub async fn json_minify(args: Value) -> Result<Value> {
    let json_string = args["json_string"].as_str().unwrap_or("");

    match serde_json::from_str::<Value>(json_string) {
        Ok(parsed) => {
            let minified = serde_json::to_string(&parsed)?;
            Ok(json!({"minified": minified}))
        }
        Err(e) => Ok(json!({"error": format!("Invalid JSON: {}", e)})),
    }
}

/// Base64 encode
pub async fn base64_encode(args: Value) -> Result<Value> {
    let text = args["text"].as_str().unwrap_or("");
    Ok(json!({"encoded": BASE64.encode(text.as_bytes())}))
}

/// Base64 decode
pub async fn base64_decode(args: Value) -> Result<Value> {
    let encoded = args["encoded"].as_str().unwrap_or("");

    match BASE64.decode(encoded) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(decoded) => Ok(json!({"decoded": decoded})),
            Err(_) => Ok(json!({"error": "Not valid UTF-8"})),
        },
        Err(e) => Ok(json!({"error": format!("Invalid base64: {}", e)})),
    }
}

/// CSV to JSON
pub async fn csv_to_json(args: Value) -> Result<Value> {
    let csv = args["csv_string"].as_str().unwrap_or("");
    let delim = args["delimiter"]
        .as_str()
        .unwrap_or(",")
        .chars()
        .next()
        .unwrap_or(',');

    let lines: Vec<&str> = csv.lines().collect();
    if lines.is_empty() {
        return Ok(json!({"error": "Empty CSV"}));
    }

    let headers: Vec<&str> = lines[0].split(delim).map(|s| s.trim()).collect();
    let records: Vec<Value> = lines[1..]
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let vals: Vec<&str> = line.split(delim).map(|s| s.trim()).collect();
            let mut map = serde_json::Map::new();
            for (i, h) in headers.iter().enumerate() {
                let v = vals.get(i).unwrap_or(&"");
                map.insert(h.to_string(), json!(v));
            }
            Value::Object(map)
        })
        .collect();

    Ok(json!({"records": records, "count": records.len()}))
}

/// JSON to CSV
pub async fn json_to_csv(args: Value) -> Result<Value> {
    let json_str = args["json_array"].as_str().unwrap_or("[]");
    let delim = args["delimiter"].as_str().unwrap_or(",");

    let array: Vec<Value> = match serde_json::from_str(json_str) {
        Ok(a) => a,
        Err(e) => return Ok(json!({"error": format!("Invalid JSON: {}", e)})),
    };

    if array.is_empty() {
        return Ok(json!({"csv": "", "rows": 0}));
    }

    let headers: Vec<String> = match &array[0] {
        Value::Object(obj) => obj.keys().cloned().collect(),
        _ => return Ok(json!({"error": "Array must contain objects"})),
    };

    let mut lines = vec![headers.join(delim)];
    for item in &array {
        if let Value::Object(obj) = item {
            let vals: Vec<String> = headers
                .iter()
                .map(|h| {
                    obj.get(h)
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            _ => v.to_string().trim_matches('"').to_string(),
                        })
                        .unwrap_or_default()
                })
                .collect();
            lines.push(vals.join(delim));
        }
    }

    Ok(json!({"csv": lines.join("\n"), "rows": array.len()}))
}

/// Find/replace in file(s) - saves reading entire file into chat
pub async fn find_replace(args: Value) -> Result<Value> {
    let path = args["path"].as_str().unwrap_or("");
    let find = args["find"].as_str().unwrap_or("");
    let replace = args["replace"].as_str().unwrap_or("");
    let use_regex = args["regex"].as_bool().unwrap_or(false);
    let recursive = args["recursive"].as_bool().unwrap_or(false);

    let p = Path::new(path);
    let mut total_replacements = 0;
    let mut files_modified = Vec::new();

    let files_to_process: Vec<PathBuf> = if p.is_file() {
        vec![p.to_path_buf()]
    } else if p.is_dir() {
        if recursive {
            WalkDir::new(p)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            fs::read_dir(p)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        }
    } else {
        return Ok(json!({"error": format!("Path not found: {}", path)}));
    };

    for file_path in files_to_process {
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue, // Skip binary/unreadable files
        };

        let (new_content, count) = if use_regex {
            match Regex::new(find) {
                Ok(re) => {
                    let matches = re.find_iter(&content).count();
                    (re.replace_all(&content, replace).to_string(), matches)
                }
                Err(e) => return Ok(json!({"error": format!("Invalid regex: {}", e)})),
            }
        } else {
            let count = content.matches(find).count();
            (content.replace(find, replace), count)
        };

        if count > 0 {
            fs::write(&file_path, &new_content)?;
            total_replacements += count;
            files_modified.push(file_path.to_string_lossy().to_string());
        }
    }

    Ok(json!({
        "path": path,
        "replacements": total_replacements,
        "files_modified": files_modified.len(),
        "files": files_modified
    }))
}

/// File hash (MD5/SHA256)
pub async fn hash_file(args: Value) -> Result<Value> {
    let path = args["path"].as_str().unwrap_or("");
    let algorithm = args["algorithm"]
        .as_str()
        .unwrap_or("sha256")
        .to_uppercase();

    // Use PowerShell for hashing
    let output = std::process::Command::new("powershell.exe")
        .args(&[
            "-Command",
            &format!(
                "(Get-FileHash -Path '{}' -Algorithm {}).Hash",
                path, algorithm
            ),
        ])
        .output()?;

    if output.status.success() {
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        Ok(json!({
            "path": path,
            "algorithm": algorithm.to_lowercase(),
            "hash": hash,
            "size": size
        }))
    } else {
        Ok(json!({"error": String::from_utf8_lossy(&output.stderr).to_string()}))
    }
}

/// File/directory stats without reading content.
/// Union of the former get_file_info (node metadata: timestamps, readonly, is_dir/is_file)
/// and file_stats (aggregation: counts, total_size) - proven complementary by live diff
/// 2026-07-29; a remove-as-duplicate would have lost one axis.
pub async fn file_stats(args: Value) -> Result<Value> {
    let path = args["path"].as_str().unwrap_or("");
    let recursive = args["recursive"].as_bool().unwrap_or(false);

    let meta = fs::metadata(path)?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let readonly = meta.permissions().readonly();

    if meta.is_file() {
        Ok(json!({
            "type": "file",
            "path": path,
            "size": meta.len(),
            "size_human": format_size(meta.len()),
            "is_file": true,
            "is_dir": false,
            "modified_unix": modified,
            "readonly": readonly
        }))
    } else {
        let mut total_size: u64 = 0;
        let mut file_count: u64 = 0;
        let mut dir_count: u64 = 0;

        let walker = if recursive {
            WalkDir::new(path).into_iter()
        } else {
            WalkDir::new(path).max_depth(1).into_iter()
        };

        for entry in walker.filter_map(|e| e.ok()) {
            if let Ok(m) = entry.metadata() {
                if m.is_file() {
                    total_size += m.len();
                    file_count += 1;
                } else if m.is_dir() && entry.depth() > 0 {
                    dir_count += 1;
                }
            }
        }

        Ok(json!({
            "type": "directory",
            "path": path,
            "files": file_count,
            "directories": dir_count,
            "total_size": total_size,
            "total_size_human": format_size(total_size),
            "recursive": recursive,
            "is_file": false,
            "is_dir": true,
            "modified_unix": modified,
            "readonly": readonly
        }))
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Grep - search files for pattern
pub async fn grep(args: Value) -> Result<Value> {
    let path = args["path"].as_str().unwrap_or("");
    let pattern = args["pattern"].as_str().unwrap_or("");
    let context = args["context"].as_u64().unwrap_or(0) as usize;
    let recursive = args["recursive"].as_bool().unwrap_or(false);

    let re = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return Ok(json!({"error": format!("Invalid regex: {}", e)})),
    };

    let p = Path::new(path);
    let files_to_search: Vec<PathBuf> = if p.is_file() {
        vec![p.to_path_buf()]
    } else if p.is_dir() {
        if recursive {
            WalkDir::new(p)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            fs::read_dir(p)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        }
    } else {
        return Ok(json!({"error": format!("Path not found: {}", path)}));
    };

    let mut all_matches: Vec<Value> = Vec::new();

    for file_path in files_to_search {
        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if re.is_match(line) {
                let start_ctx = i.saturating_sub(context);
                let end_ctx = (i + context + 1).min(lines.len());
                let context_lines: Vec<String> = lines[start_ctx..end_ctx]
                    .iter()
                    .enumerate()
                    .map(|(j, l)| format!("{}: {}", start_ctx + j + 1, l))
                    .collect();

                all_matches.push(json!({
                    "file": file_path.to_string_lossy(),
                    "line": i + 1,
                    "match": line,
                    "context": context_lines
                }));
            }
        }
    }

    Ok(json!({
        "path": path,
        "pattern": pattern,
        "matches": all_matches,
        "count": all_matches.len()
    }))
}

/// Project scaffolding
pub async fn scaffold(args: Value) -> Result<Value> {
    let template = args["template"].as_str().unwrap_or("");
    let name = args["name"].as_str().unwrap_or("");
    let output = args["output_dir"].as_str().unwrap_or(".");

    let base_path = Path::new(output).join(name);
    fs::create_dir_all(&base_path)?;

    let files_created: Vec<String> = match template {
        "rust-mcp" => scaffold_rust_mcp(&base_path, name),
        "python-mcp" => scaffold_python_mcp(&base_path, name),
        "nextjs" => scaffold_nextjs(&base_path, name),
        "fastapi" => scaffold_fastapi(&base_path, name),
        "expo" => scaffold_expo(&base_path, name),
        _ => {
            return Ok(
                json!({"error": format!("Unknown template: {}. Use: rust-mcp, python-mcp, nextjs, fastapi, expo", template)}),
            )
        }
    };

    Ok(json!({
        "template": template,
        "name": name,
        "path": base_path.to_string_lossy(),
        "files_created": files_created
    }))
}

fn scaffold_rust_mcp(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let cargo = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
chrono = {{ version = "0.4", features = ["serde"] }}
anyhow = "1"
"#,
        name
    );
    write_scaffold(&base.join("Cargo.toml"), &cargo, &mut files);

    fs::create_dir_all(base.join("src/tools")).ok();

    let main = r#"use std::io::{self, BufRead, Write};
use serde_json::{json, Value};

mod tools;

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    
    for line in stdin.lock().lines().flatten() {
        if let Ok(request) = serde_json::from_str::<Value>(&line) {
            let response = handle_request(&request);
            let _ = writeln!(stdout, "{}", serde_json::to_string(&response).unwrap());
            let _ = stdout.flush();
        }
    }
}

fn handle_request(request: &Value) -> Value {
    match request["method"].as_str().unwrap_or("") {
        "initialize" => json!({"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "serverInfo": {"name": env!("CARGO_PKG_NAME"), "version": env!("CARGO_PKG_VERSION")}}),
        "tools/list" => json!({"tools": tools::get_definitions()}),
        "tools/call" => {
            let name = request["params"]["name"].as_str().unwrap_or("");
            let args = &request["params"]["arguments"];
            json!({"content": [{"type": "text", "text": serde_json::to_string(&tools::execute(name, args)).unwrap()}]})
        }
        _ => json!({"error": "unknown method"})
    }
}
"#;
    write_scaffold(&base.join("src/main.rs"), main, &mut files);

    let tools_mod = r#"use serde_json::{json, Value};

pub fn get_definitions() -> Vec<Value> {
    vec![
        json!({"name": "hello", "description": "Say hello", "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}}),
    ]
}

pub fn execute(name: &str, args: &Value) -> Value {
    match name {
        "hello" => json!({"message": format!("Hello, {}!", args["name"].as_str().unwrap_or("World"))}),
        _ => json!({"error": format!("Unknown tool: {}", name)})
    }
}
"#;
    write_scaffold(&base.join("src/tools/mod.rs"), tools_mod, &mut files);

    files
}

fn scaffold_python_mcp(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let server = format!(
        r#"#!/usr/bin/env python3
"""MCP Server: {}"""
import asyncio
from mcp.server import Server
from mcp.server.stdio import stdio_server

server = Server("{}")

@server.list_tools()
async def list_tools():
    return [{{"name": "hello", "description": "Say hello", "inputSchema": {{"type": "object", "properties": {{"name": {{"type": "string"}}}}, "required": ["name"]}}}}]

@server.call_tool()
async def call_tool(name: str, arguments: dict):
    if name == "hello":
        return f"Hello, {{arguments.get('name', 'World')}}!"
    raise ValueError(f"Unknown tool: {{name}}")

async def main():
    async with stdio_server() as (read, write):
        await server.run(read, write, server.create_initialization_options())

if __name__ == "__main__":
    asyncio.run(main())
"#,
        name, name
    );
    write_scaffold(&base.join("server.py"), &server, &mut files);
    write_scaffold(&base.join("requirements.txt"), "mcp>=1.0.0\n", &mut files);

    files
}

fn scaffold_nextjs(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let package = format!(
        r#"{{
  "name": "{}",
  "version": "0.1.0",
  "scripts": {{ "dev": "next dev", "build": "next build", "start": "next start" }},
  "dependencies": {{ "next": "^14.0.0", "react": "^18.2.0", "react-dom": "^18.2.0" }}
}}
"#,
        name
    );
    write_scaffold(&base.join("package.json"), &package, &mut files);

    fs::create_dir_all(base.join("app")).ok();
    write_scaffold(
        &base.join("app/page.tsx"),
        "export default function Home() {\n  return <main><h1>Hello World</h1></main>\n}\n",
        &mut files,
    );
    write_scaffold(&base.join("app/layout.tsx"), "export default function RootLayout({ children }: { children: React.ReactNode }) {\n  return <html><body>{children}</body></html>\n}\n", &mut files);

    files
}

fn scaffold_fastapi(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let main = format!(
        r#"from fastapi import FastAPI

app = FastAPI(title="{}")

@app.get("/")
def root():
    return {{"message": "Hello World"}}

@app.get("/health")
def health():
    return {{"status": "ok"}}
"#,
        name
    );
    write_scaffold(&base.join("main.py"), &main, &mut files);
    write_scaffold(
        &base.join("requirements.txt"),
        "fastapi>=0.100.0\nuvicorn>=0.23.0\n",
        &mut files,
    );

    files
}

fn scaffold_expo(base: &Path, name: &str) -> Vec<String> {
    let mut files = Vec::new();

    let package = format!(
        r#"{{
  "name": "{}",
  "version": "1.0.0",
  "main": "expo-router/entry",
  "scripts": {{ "start": "expo start", "android": "expo start --android", "ios": "expo start --ios" }},
  "dependencies": {{ "expo": "~50.0.0", "expo-router": "~3.4.0", "react": "18.2.0", "react-native": "0.73.2" }}
}}
"#,
        name
    );
    write_scaffold(&base.join("package.json"), &package, &mut files);

    fs::create_dir_all(base.join("app")).ok();
    write_scaffold(&base.join("app/index.tsx"), "import { Text, View } from 'react-native';\n\nexport default function Home() {\n  return <View><Text>Hello World</Text></View>;\n}\n", &mut files);
    write_scaffold(
        &base.join("app.json"),
        &format!(r#"{{"expo": {{"name": "{}"}}}}"#, name),
        &mut files,
    );

    files
}

fn write_scaffold(path: &Path, content: &str, files: &mut Vec<String>) {
    if fs::write(path, content).is_ok() {
        files.push(path.to_string_lossy().to_string());
    }
}

// ============ ORIGINAL ANTIGRAVITY TRANSFORMS ============

/// Create archive (zip, tar, tar.gz, tar.bz2)
pub async fn archive(args: Value) -> Result<Value> {
    let paths = args["paths"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    let output = args["output"].as_str().unwrap_or("archive.zip");
    let format = args["format"].as_str().unwrap_or("zip");

    if paths.is_empty() {
        return Ok(json!({"success": false, "error": "No paths provided"}));
    }

    let output_path = PathBuf::from(output);

    match format {
        "zip" => {
            let file = fs::File::create(&output_path)?;
            let mut zip = ZipWriter::new(file);
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);

            let mut file_count = 0;

            for path_str in &paths {
                let path = Path::new(path_str);
                if path.is_file() {
                    let name = path.file_name().unwrap().to_string_lossy();
                    zip.start_file(name.to_string(), options)?;
                    let mut f = fs::File::open(path)?;
                    let mut buffer = Vec::new();
                    f.read_to_end(&mut buffer)?;
                    zip.write_all(&buffer)?;
                    file_count += 1;
                } else if path.is_dir() {
                    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        if entry_path.is_file() {
                            let rel_path = entry_path
                                .strip_prefix(path.parent().unwrap_or(path))
                                .unwrap_or(entry_path);
                            zip.start_file(rel_path.to_string_lossy().replace("\\", "/"), options)?;
                            let mut f = fs::File::open(entry_path)?;
                            let mut buffer = Vec::new();
                            f.read_to_end(&mut buffer)?;
                            zip.write_all(&buffer)?;
                            file_count += 1;
                        }
                    }
                }
            }

            zip.finish()?;
            let size = fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);

            Ok(json!({
                "success": true,
                "format": "zip",
                "output": output,
                "files_archived": file_count,
                "size_bytes": size
            }))
        }
        "tar" | "tar.gz" | "tar.bz2" => {
            let compress_flag = match format {
                "tar.gz" => "-czf",
                "tar.bz2" => "-cjf",
                _ => "-cf",
            };

            let paths_str = paths.join("\" \"");
            let cmd = format!(r#"tar {} "{}" "{}""#, compress_flag, output, paths_str);

            let output_result = std::process::Command::new("powershell")
                .args(["-Command", &cmd])
                .output()?;

            if output_result.status.success() {
                let size = fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);
                Ok(json!({
                    "success": true,
                    "format": format,
                    "output": output,
                    "size_bytes": size
                }))
            } else {
                Ok(json!({
                    "success": false,
                    "error": String::from_utf8_lossy(&output_result.stderr).to_string()
                }))
            }
        }
        _ => Ok(json!({"success": false, "error": format!("Unknown format: {}", format)})),
    }
}

/// Extract archive (auto-detect format)
pub async fn extract(args: Value) -> Result<Value> {
    let archive_path = args["archive_path"].as_str().unwrap_or("");
    let destination = args["destination"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let p = Path::new(archive_path);
            p.parent()
                .unwrap_or(Path::new("."))
                .to_string_lossy()
                .to_string()
        });

    let path = Path::new(archive_path);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    fs::create_dir_all(&destination)?;

    if ext == "zip" || archive_path.ends_with(".zip") {
        let file = fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut extracted = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = Path::new(&destination).join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
                extracted += 1;
            }
        }

        Ok(json!({
            "success": true,
            "archive": archive_path,
            "destination": destination,
            "files_extracted": extracted
        }))
    } else {
        let extract_flag = if archive_path.contains(".gz") {
            "-xzf"
        } else if archive_path.contains(".bz2") {
            "-xjf"
        } else {
            "-xf"
        };

        let cmd = format!(
            r#"tar {} "{}" -C "{}""#,
            extract_flag, archive_path, destination
        );

        let output = std::process::Command::new("powershell")
            .args(["-Command", &cmd])
            .output()?;

        if output.status.success() {
            Ok(json!({
                "success": true,
                "archive": archive_path,
                "destination": destination
            }))
        } else {
            Ok(json!({
                "success": false,
                "error": String::from_utf8_lossy(&output.stderr).to_string()
            }))
        }
    }
}

/// Regex-based batch rename
pub async fn bulk_rename(args: Value) -> Result<Value> {
    let directory = args["directory"].as_str().unwrap_or(".");
    let pattern = args["pattern"].as_str().unwrap_or("");
    let replacement = args["replacement"].as_str().unwrap_or("");
    let dry_run = args["dry_run"].as_bool().unwrap_or(true);

    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => return Ok(json!({"success": false, "error": format!("Invalid regex: {}", e)})),
    };

    let mut renames = Vec::new();

    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if regex.is_match(filename) {
                    let new_name = regex.replace_all(filename, replacement).to_string();
                    if new_name != filename {
                        let new_path = path.parent().unwrap().join(&new_name);

                        renames.push(json!({
                            "from": filename,
                            "to": new_name
                        }));

                        if !dry_run {
                            fs::rename(&path, &new_path)?;
                        }
                    }
                }
            }
        }
    }

    Ok(json!({
        "success": true,
        "directory": directory,
        "pattern": pattern,
        "replacement": replacement,
        "dry_run": dry_run,
        "renames": renames,
        "count": renames.len()
    }))
}

/// Sync directories with different modes
pub async fn sync_directories(args: Value) -> Result<Value> {
    let source = args["source"].as_str().unwrap_or("");
    let destination = args["destination"].as_str().unwrap_or("");
    let mode = args["mode"].as_str().unwrap_or("update");
    let dry_run = args["dry_run"].as_bool().unwrap_or(true);
    let exclude = args["exclude"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    if source.is_empty() || destination.is_empty() {
        return Ok(json!({"success": false, "error": "Source and destination required"}));
    }

    fs::create_dir_all(destination)?;

    let mut copied = Vec::new();
    let mut deleted = Vec::new();
    let mut skipped = 0;

    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        let src_path = entry.path();
        let rel_path = src_path.strip_prefix(source).unwrap_or(src_path);
        let rel_str = rel_path.to_string_lossy().to_string();

        if exclude.iter().any(|ex| rel_str.contains(ex)) {
            skipped += 1;
            continue;
        }

        let dst_path = Path::new(destination).join(rel_path);

        if src_path.is_dir() {
            if !dry_run {
                fs::create_dir_all(&dst_path)?;
            }
        } else if src_path.is_file() {
            let should_copy = match mode {
                "mirror" | "backup" => true,
                "update" => {
                    if !dst_path.exists() {
                        true
                    } else {
                        let src_modified = fs::metadata(src_path)?.modified()?;
                        let dst_modified = fs::metadata(&dst_path)?.modified()?;
                        src_modified > dst_modified
                    }
                }
                _ => true,
            };

            if should_copy {
                copied.push(rel_str.clone());
                if !dry_run {
                    if let Some(parent) = dst_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(src_path, &dst_path)?;
                }
            }
        }
    }

    if mode == "mirror" {
        for entry in WalkDir::new(destination).into_iter().filter_map(|e| e.ok()) {
            let dst_path = entry.path();
            let rel_path = dst_path.strip_prefix(destination).unwrap_or(dst_path);
            let src_path = Path::new(source).join(rel_path);

            if !src_path.exists() && dst_path.is_file() {
                let rel_str = rel_path.to_string_lossy().to_string();
                deleted.push(rel_str);
                if !dry_run {
                    fs::remove_file(dst_path)?;
                }
            }
        }
    }

    Ok(json!({
        "success": true,
        "source": source,
        "destination": destination,
        "mode": mode,
        "dry_run": dry_run,
        "files_copied": copied.len(),
        "files_deleted": deleted.len(),
        "files_skipped": skipped,
        "copied": copied,
        "deleted": deleted
    }))
}

/// Create unified diff between two files
pub async fn diff_files(args: Value) -> Result<Value> {
    let path1 = args["path1"]
        .as_str()
        .or(args["file_a"].as_str())
        .unwrap_or("");
    let path2 = args["path2"]
        .as_str()
        .or(args["file_b"].as_str())
        .unwrap_or("");
    let _context_lines = args["context_lines"].as_i64().unwrap_or(3);

    let content1 = fs::read_to_string(path1)?;
    let content2 = fs::read_to_string(path2)?;

    let lines1: Vec<&str> = content1.lines().collect();
    let lines2: Vec<&str> = content2.lines().collect();

    let mut diff_output = Vec::new();
    diff_output.push(format!("--- {}", path1));
    diff_output.push(format!("+++ {}", path2));

    let max_len = std::cmp::max(lines1.len(), lines2.len());
    let mut changes = 0;

    for i in 0..max_len {
        let line1 = lines1.get(i);
        let line2 = lines2.get(i);

        match (line1, line2) {
            (Some(a), Some(b)) if a != b => {
                diff_output.push(format!("{}:- {}", i + 1, a));
                diff_output.push(format!("{}:+ {}", i + 1, b));
                changes += 1;
            }
            (Some(a), None) => {
                diff_output.push(format!("{}:- {}", i + 1, a));
                changes += 1;
            }
            (None, Some(b)) => {
                diff_output.push(format!("{}:+ {}", i + 1, b));
                changes += 1;
            }
            _ => {}
        }
    }

    Ok(json!({
        "success": true,
        "path1": path1,
        "path2": path2,
        "lines_in_file1": lines1.len(),
        "lines_in_file2": lines2.len(),
        "changes": changes,
        "diff": diff_output.join("\n")
    }))
}

/// Apply Python transform to matching files  
pub async fn transform_files(args: Value) -> Result<Value> {
    let directory = args["directory"].as_str().unwrap_or(".");
    let pattern = args["pattern"].as_str().unwrap_or("*");
    let transform_code = args["transform_code"].as_str().unwrap_or("");
    let dry_run = args["dry_run"].as_bool().unwrap_or(true);

    let regex_pattern = pattern
        .replace(".", "\\.")
        .replace("*", ".*")
        .replace("?", ".");
    let regex = Regex::new(&regex_pattern)?;

    let mut transformed = Vec::new();
    let mut errors = Vec::new();

    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if regex.is_match(filename) {
                    let content = match fs::read_to_string(path) {
                        Ok(c) => c,
                        Err(e) => {
                            errors.push(
                                json!({"file": path.to_string_lossy(), "error": e.to_string()}),
                            );
                            continue;
                        }
                    };

                    let escaped_content = content
                        .replace("\\", "\\\\")
                        .replace("\"", "\\\"")
                        .replace("\n", "\\n")
                        .replace("\r", "\\r");

                    let py_code = format!(
                        r#"
content = "{}"
result = {}
print(result)
"#,
                        escaped_content, transform_code
                    );

                    let output = std::process::Command::new("python")
                        .args(["-c", &py_code])
                        .output()?;

                    if output.status.success() {
                        let new_content = String::from_utf8_lossy(&output.stdout).to_string();

                        transformed.push(json!({
                            "file": path.to_string_lossy(),
                            "original_size": content.len(),
                            "new_size": new_content.len()
                        }));

                        if !dry_run {
                            fs::write(path, new_content.trim())?;
                        }
                    } else {
                        errors.push(json!({
                            "file": path.to_string_lossy(),
                            "error": String::from_utf8_lossy(&output.stderr).to_string()
                        }));
                    }
                }
            }
        }
    }

    Ok(json!({
        "success": true,
        "directory": directory,
        "pattern": pattern,
        "dry_run": dry_run,
        "transformed": transformed.len(),
        "errors": errors.len(),
        "files": transformed,
        "error_details": errors
    }))
}
