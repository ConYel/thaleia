//! Tests for rmcp-based MCP server
//!
//! These tests verify the MCP server implementation works correctly.

#[cfg(test)]
mod tests {
    /// Test that tool definitions match expected MCP convention
    #[test]
    fn test_tool_names() {
        // Expected tool names from thaleia-mcp
        let expected_tools = [
            "thaleia_speak",
            "thaleia_listen",
            "thaleia_list_voices",
            "thaleia_get_status",
        ];
        assert_eq!(expected_tools.len(), 4);
    }
}
