//! MCP Server for Thaleia - Model Context Protocol implementation
//!
//! This module implements the MCP protocol using the official rmcp SDK (v1.3).
//! Designed to be green: minimal token usage, sparse session history.
//!
//! # MCP Protocol
//!
//! - JSON-RPC 2.0 over stdio or HTTP
//! - Tools exposed as MCP methods using rmcp macros
//! - Session state kept minimal for environmental efficiency

use serde::{Deserialize, Serialize};

pub mod handler;
pub mod session;
pub mod tools;

// New rmcp-based server (v1.3)
pub mod rmcp_server;

// Audio thread manager (bridges async MCP with sync audio)
pub mod audio_manager;

pub use handler::{McpServer, run_server};
pub use rmcp_server::SessionMode;
pub use session::SessionState;
pub use tools::{ToolCall, ToolDefinition, ToolResponse};

// Re-export debug utilities from thaleia-core (unified system)
pub use thaleia_core::{init_logging, is_debug, set_debug, thaleia_debug};

// Re-export for convenience
pub use anyhow::Result;
pub use serde_json::Value;

/// MCP JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Success {
        jsonrpc: String,
        id: Value,
        result: Value,
    },
    Error {
        jsonrpc: String,
        id: Value,
        error: JsonRpcError,
    },
}

/// MCP JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// MCP protocol constants
pub mod protocol {
    // JSON-RPC error codes
    pub const ERR_PARSE_ERROR: i32 = -32700;
    pub const ERR_INVALID_REQUEST: i32 = -32600;
    pub const ERR_METHOD_NOT_FOUND: i32 = -32601;
    pub const ERR_INVALID_PARAMS: i32 = -32602;
    pub const ERR_INTERNAL_ERROR: i32 = -32603;

    // MCP protocol version
    pub const JSONRPC_VERSION: &str = "2.0";

    // Tool name prefixes
    pub const TOOL_PREFIX: &str = "thaleia_";
}
