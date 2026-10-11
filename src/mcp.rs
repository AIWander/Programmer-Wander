//! MCP Protocol Implementation (JSON-RPC over stdio)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use tracing::{error, info, warn};

use crate::tools;

// ============ MCP PROTOCOL TYPES ============

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// ============ MCP SERVER ============

pub async fn run_stdio_server() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    info!("MCP server ready, listening on stdio");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                error!("Read error: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                // Parse error - send error response
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        // Validate JSON-RPC 2.0 version
        if let Some(ref version) = request.jsonrpc {
            if version != "2.0" {
                warn!("Invalid JSON-RPC version: {}", version);
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id.clone().unwrap_or(Value::Null),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32600,
                        message: format!(
                            "Invalid JSON-RPC version: expected '2.0', got '{}'",
                            version
                        ),
                        data: None,
                    }),
                };
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            }
        }

        // Get method name
        let method = match &request.method {
            Some(m) => m.clone(),
            None => {
                warn!("Request missing method");
                continue;
            }
        };

        // NOTIFICATIONS: Don't respond to notifications (no id, or method starts with "notifications/")
        if request.id.is_none() || method.starts_with("notifications/") {
            info!("Notification received: {} (no response)", method);
            continue;
        }

        let response = handle_request(&method, request.id, request.params).await;
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }

    Ok(())
}

async fn handle_request(method: &str, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
    let id = id.unwrap_or(Value::Null);

    match method {
        "initialize" => {
            info!("Initialize request received");
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": env!("CARGO_PKG_NAME"),
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
                error: None,
            }
        }

        "tools/list" => {
            info!("Tools list requested");
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "tools": tools::get_tool_definitions()
                })),
                error: None,
            }
        }

        "tools/call" => {
            let params = params.unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let tool_args = params.get("arguments").cloned().unwrap_or(json!({}));

            info!("Tool call: {}", tool_name);

            match tools::execute_tool(tool_name, tool_args).await {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                        }]
                    })),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": format!("{{\"success\": false, \"error\": \"{}\"}}", e)
                        }],
                        "isError": true
                    })),
                    error: None,
                },
            }
        }

        "ping" => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({})),
            error: None,
        },

        _ => {
            warn!("Unknown method: {}", method);
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", method),
                    data: None,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::handle_request;
    use serde_json::json;

    #[tokio::test]
    async fn initialize_identity_comes_from_package_metadata() {
        let response = handle_request("initialize", Some(json!(1)), None).await;
        let result = response.result.expect("initialize result");
        assert_eq!(result["serverInfo"]["name"], env!("CARGO_PKG_NAME"));
        assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    }
}
