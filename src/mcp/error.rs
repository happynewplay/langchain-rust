use thiserror::Error;

/// Errors that can occur when working with MCP (Model Context Protocol) clients and tools
#[derive(Error, Debug)]
pub enum McpError {
    /// Error from the underlying RMCP library
    #[error("RMCP error: {0}")]
    RmcpError(#[from] rmcp::ErrorData),

    /// Error when initializing MCP client
    #[error("Failed to initialize MCP client: {0}")]
    InitializationError(String),

    /// Error when calling MCP tools
    #[error("Tool call failed: {0}")]
    ToolCallError(String),

    /// Error when parsing tool responses
    #[error("Failed to parse tool response: {0}")]
    ParseError(String),

    /// Error when connecting to MCP server
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Error when tool is not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Generic error for other MCP-related issues
    #[error("MCP error: {0}")]
    Other(String),
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::ParseError(err.to_string())
    }
}

impl From<reqwest::Error> for McpError {
    fn from(err: reqwest::Error) -> Self {
        McpError::ConnectionError(err.to_string())
    }
}
