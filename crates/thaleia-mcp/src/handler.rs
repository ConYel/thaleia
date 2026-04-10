//! MCP Server handler - JSON-RPC over stdio (Legacy implementation)
//!
//! NOTE: This is the OLD implementation kept for backward compatibility.
//! The new implementation uses rmcp_server.rs with the official rmcp SDK.
//! This file will be deprecated in favor of the rmcp implementation.

use std::io::{BufRead, Write};
use std::sync::Arc;

use crate::Result;
use crate::protocol;
use crate::tools::{ThaleiaToolExecutor, ToolCall};
use crate::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

/// MCP Server implementation
pub struct McpServer {
    executor: Arc<ThaleiaToolExecutor>,
}

impl McpServer {
    /// Create new MCP server
    pub fn new() -> Self {
        Self {
            executor: Arc::new(ThaleiaToolExecutor::new()),
        }
    }

    /// Run the MCP server (blocking)
    pub fn run(&self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();

        let handler = self.executor.clone();

        // Use tokio runtime for async tool execution
        let rt = tokio::runtime::Runtime::new()?;

        // Read loop
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    // Broken pipe (stdout closed) or EOF - normal shutdown
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        tracing::debug!("Stdin closed, server exiting");
                    } else {
                        tracing::error!("Failed to read stdin: {}", e);
                    }
                    break;
                }
            };

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse request
            let response: JsonRpcResponse = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(request) => {
                    let handler = handler.clone();
                    rt.block_on(async { Self::handle_request(request, handler).await })
                        .unwrap_or_else(|e| {
                            tracing::error!("Tool execution error: {}", e);
                            JsonRpcResponse::Error {
                                jsonrpc: protocol::JSONRPC_VERSION.to_string(),
                                id: serde_json::Value::Null,
                                error: JsonRpcError {
                                    code: protocol::ERR_INTERNAL_ERROR,
                                    message: e.to_string(),
                                    data: None,
                                },
                            }
                        })
                }
                Err(e) => {
                    tracing::warn!("Invalid JSON-RPC request: {}", e);
                    JsonRpcResponse::Error {
                        jsonrpc: protocol::JSONRPC_VERSION.to_string(),
                        id: serde_json::Value::Null,
                        error: JsonRpcError {
                            code: protocol::ERR_PARSE_ERROR,
                            message: format!("Parse error: {}", e),
                            data: None,
                        },
                    }
                }
            };

            // Write response
            let response_json = serde_json::to_string(&response)?;
            if let Err(e) = writeln!(stdout, "{}", response_json) {
                // Broken pipe - client disconnected
                tracing::debug!("Failed to write to stdout: {}, exiting", e);
                break;
            }
            let _ = stdout.flush();
        }

        // Explicitly shutdown the runtime
        rt.shutdown_background();

        Ok(())
    }

    /// Handle incoming JSON-RPC request
    async fn handle_request(
        request: JsonRpcRequest,
        executor: Arc<ThaleiaToolExecutor>,
    ) -> Result<JsonRpcResponse> {
        let id = request.id.clone();

        match request.method.as_str() {
            "initialize" => {
                let result = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "thaleia-mcp",
                        "version": "0.1.0"
                    }
                });
                Ok(JsonRpcResponse::Success {
                    jsonrpc: protocol::JSONRPC_VERSION.to_string(),
                    id,
                    result,
                })
            }

            "tools/list" => {
                let tools: Vec<serde_json::Value> = crate::tools::MCP_TOOLS.iter()
                    .map(|(name, description, input_schema)| {
                        serde_json::json!({
                            "name": name,
                            "description": description,
                            "inputSchema": serde_json::from_str::<serde_json::Value>(input_schema).unwrap()
                        })
                    })
                    .collect();

                let result = serde_json::json!({ "tools": tools });
                Ok(JsonRpcResponse::Success {
                    jsonrpc: protocol::JSONRPC_VERSION.to_string(),
                    id,
                    result,
                })
            }

            "tools/call" => {
                let params = request
                    .params
                    .ok_or_else(|| anyhow::anyhow!("Missing params for tools/call"))?;

                let name = params
                    .get("name")
                    .and_then(|v: &serde_json::Value| v.as_str().map(String::from))
                    .ok_or_else(|| anyhow::anyhow!("Missing 'name' in params"))?;

                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let call = ToolCall { name, arguments };
                let response = executor.execute_tool(call).await?;

                let result = serde_json::json!({
                    "content": response.content,
                    "isError": response.is_error
                });

                Ok(JsonRpcResponse::Success {
                    jsonrpc: protocol::JSONRPC_VERSION.to_string(),
                    id,
                    result,
                })
            }

            "notifications/initialized" => Ok(JsonRpcResponse::Success {
                jsonrpc: protocol::JSONRPC_VERSION.to_string(),
                id: serde_json::Value::Null,
                result: serde_json::Value::Null,
            }),

            _ => Ok(JsonRpcResponse::Error {
                jsonrpc: protocol::JSONRPC_VERSION.to_string(),
                id,
                error: JsonRpcError {
                    code: protocol::ERR_METHOD_NOT_FOUND,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                },
            }),
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Run MCP server (convenience function)
pub fn run_server() -> Result<()> {
    let server = McpServer::new();
    server.run()
}
