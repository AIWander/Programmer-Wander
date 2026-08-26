//! HTTP Operations
//! 2026-07-29 rebuild: http_download absorbed into request(save=) keeping Range-resume;
//! http_scrape removed (Hands' domain - hands:browser_http_scrape / local:http_scrape cover it).

use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::fs;
use tracing::info;

/// Make HTTP request. With save=<path>, streams the body to disk instead of returning it
/// (absorbs the former http_download, including resume-via-Range).
pub async fn request(args: Value) -> Result<Value> {
    let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
    let headers: HashMap<String, String> = args
        .get("headers")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let body = args.get("body").and_then(|v| v.as_str());
    let timeout_secs = args.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30);
    let save = args
        .get("save")
        .and_then(|v| v.as_str())
        .or_else(|| args.get("destination").and_then(|v| v.as_str()));
    let resume = args.get("resume").and_then(|v| v.as_bool()).unwrap_or(true);

    if url.is_empty() {
        anyhow::bail!("url is required");
    }

    info!("HTTP {} {}", method, url);

    // SAVE MODE: download to disk (former http_download semantics, GET + Range resume)
    if let Some(destination) = save {
        if let Some(parent) = std::path::Path::new(destination).parent() {
            let _ = fs::create_dir_all(parent).await;
        }

        let client = reqwest::Client::new();
        let start = Instant::now();

        let existing_size = if resume {
            fs::metadata(destination).await.map(|m| m.len()).ok()
        } else {
            None
        };

        let mut request = client.get(url);
        for (key, value) in &headers {
            request = request.header(key, value);
        }
        if let Some(size) = existing_size {
            if size > 0 {
                request = request.header("Range", format!("bytes={}-", size));
            }
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() && status.as_u16() != 206 {
            return Ok(json!({
                "success": false,
                "error": format!("HTTP {}", status)
            }));
        }

        let bytes = response.bytes().await?;

        if existing_size.is_some() && status.as_u16() == 206 {
            let mut file = tokio::fs::OpenOptions::new()
                .append(true)
                .open(destination)
                .await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &bytes).await?;
        } else {
            fs::write(destination, &bytes).await?;
        }

        let elapsed = start.elapsed().as_millis() as u64;
        let final_size = fs::metadata(destination)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        return Ok(json!({
            "success": true,
            "saved_to": destination,
            "size_bytes": final_size,
            "download_time_ms": elapsed,
            "resumed": existing_size.is_some() && status.as_u16() == 206
        }));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;

    let start = Instant::now();

    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        "HEAD" => client.head(url),
        _ => anyhow::bail!("Unsupported method: {}", method),
    };

    for (key, value) in headers {
        request = request.header(&key, &value);
    }

    if let Some(b) = body {
        request = request.body(b.to_string());
    }

    let response = request.send().await?;
    let elapsed = start.elapsed().as_millis() as u64;

    let status = response.status().as_u16();
    let response_headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_text = response.text().await?;

    Ok(json!({
        "success": status >= 200 && status < 300,
        "status_code": status,
        "headers": response_headers,
        "body": body_text,
        "body_length": body_text.len(),
        "response_time_ms": elapsed
    }))
}
