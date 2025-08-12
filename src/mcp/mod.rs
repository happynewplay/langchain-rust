//! Model Context Protocol (MCP) client implementation for langchain-rust
//!
//! This module provides MCP client functionality that allows langchain-rust agents
//! to interact with MCP servers and use MCP tools. The implementation is based on
//! the RMCP library and supports SSE (Server-Sent Events) transport.
//!
//! # Features
//!
//! - **MCP Client**: Connect to MCP servers via SSE transport
//! - **Tool Integration**: Use MCP tools as langchain-rust tools
//! - **Agent Support**: Integrate MCP tools with existing agent systems
//! - **Streaming Support**: Compatible with langchain-rust streaming infrastructure
//!
//! # Example
//!
//! ```rust,ignore
//! use langchain_rust::mcp::{McpClient, McpClientConfig};
//! use langchain_rust::agent::OpenAiToolAgentBuilder;
//! use langchain_rust::llm::openai::OpenAI;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to MCP server
//!     let mcp_client = McpClient::connect("http://127.0.0.1:8000/sse").await?;
//!     
//!     // Get MCP tools as langchain tools
//!     let mcp_tools = mcp_client.get_langchain_tools().await?;
//!     
//!     // Create agent with MCP tools
//!     let llm = OpenAI::default();
//!     let agent = OpenAiToolAgentBuilder::new()
//!         .tools(&mcp_tools)
//!         .build(llm)?;
//!     
//!     // Use the agent...
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod tool;

#[cfg(test)]
mod tests;

pub use client::{McpClient, McpClientConfig, McpTransport};
pub use error::McpError;
pub use tool::McpTool;

// Re-export commonly used types from rmcp for convenience
pub use rmcp::model::Tool as RmcpTool;
pub use rmcp::service::RunningService;
pub use rmcp::RoleClient;
