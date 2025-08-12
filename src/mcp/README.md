# Model Context Protocol (MCP) Support for langchain-rust

This module provides Model Context Protocol (MCP) client functionality for langchain-rust, allowing agents to interact with MCP servers and use MCP tools seamlessly.

## Features

- **Multiple Transport Protocols**: Support for SSE, stdio, child process, and streamable HTTP transports
- **MCP Client**: Connect to MCP servers using various transport methods
- **Tool Integration**: Use MCP tools as langchain-rust tools
- **Agent Support**: Integrate MCP tools with existing agent systems
- **Error Handling**: Comprehensive error handling for MCP operations
- **Configuration**: Flexible client configuration options

## Quick Start

### 1. Enable MCP Feature

Add the MCP feature to your `Cargo.toml`:

```toml
[dependencies]
langchain-rust = { version = "4.6.0", features = ["mcp"] }
```

### 2. Basic Usage

```rust
use langchain_rust::{
    agent::{AgentExecutor, McpAgentBuilder},
    llm::openai::OpenAI,
    mcp::{McpClient, McpClientConfig},
    prompt_args,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MCP server using SSE transport
    let mcp_client = McpClient::connect_sse("http://127.0.0.1:8000/sse").await?;

    // Alternative transport methods:
    // let mcp_client = McpClient::connect_stdio().await?;
    // let mcp_client = McpClient::connect_child_process("python", vec!["-m".to_string(), "mcp_server".to_string()]).await?;
    // let mcp_client = McpClient::connect_streamable_http("http://127.0.0.1:8000/stream").await?;

    // Get MCP tools as langchain tools
    let mcp_tools = mcp_client.get_langchain_tools().await?;

    // Create agent with MCP tools
    let llm = OpenAI::default();
    let agent = McpAgentBuilder::new()
        .mcp_tools(&mcp_client)
        .await?
        .prefix("You are a helpful AI assistant with access to tools.")
        .build(llm)?;

    // Create executor and run
    let executor = AgentExecutor::from_agent(agent);
    let result = executor.invoke(prompt_args! {
        "input" => "Calculate the factorial of 5"
    }).await?;

    println!("Result: {}", result);
    Ok(())
}
```

## Transport Protocols

The MCP client supports multiple transport protocols for different use cases:

### 1. SSE (Server-Sent Events)
Best for web-based MCP servers and HTTP-based communication:

```rust
// Simple connection
let client = McpClient::connect_sse("http://127.0.0.1:8000/sse").await?;

// With configuration
let config = McpClientConfig::new_sse("http://127.0.0.1:8000/sse")
    .with_client_name("my-app")
    .with_client_version("1.0.0");
let client = McpClient::new(config).await?;
```

### 2. Stdio (Standard Input/Output)
Best for command-line tools and direct process communication:

```rust
// Connect via stdio
let client = McpClient::connect_stdio().await?;

// With configuration
let config = McpClientConfig::new_stdio()
    .with_client_name("my-cli-app");
let client = McpClient::new(config).await?;
```

### 3. Child Process
Best for launching and communicating with MCP server processes:

```rust
// Launch Python MCP server
let client = McpClient::connect_child_process(
    "python",
    vec!["-m".to_string(), "mcp_server".to_string()]
).await?;

// Launch Node.js MCP server
let client = McpClient::connect_child_process(
    "node",
    vec!["server.js".to_string()]
).await?;
```

### 4. Streamable HTTP
Best for HTTP-based streaming communication:

```rust
// Connect to streamable HTTP server
let client = McpClient::connect_streamable_http("http://127.0.0.1:8000/stream").await?;
```

## Components

### McpClient

The main client for connecting to MCP servers using various transport protocols:

```rust
use langchain_rust::mcp::{McpClient, McpClientConfig};

// SSE transport (default for backward compatibility)
let client = McpClient::connect("http://127.0.0.1:8000/sse").await?;

// Explicit transport selection
let client = McpClient::connect_sse("http://127.0.0.1:8000/sse").await?;
let client = McpClient::connect_stdio().await?;
let client = McpClient::connect_child_process("python", vec!["-m".to_string(), "server".to_string()]).await?;
let client = McpClient::connect_streamable_http("http://127.0.0.1:8000/stream").await?;
```

### McpAgentBuilder

Builder for creating agents with MCP tool support:

```rust
use langchain_rust::agent::McpAgentBuilder;

let agent = McpAgentBuilder::new()
    .mcp_tools(&mcp_client).await?  // Add MCP tools
    .tools(&regular_tools)          // Add regular tools
    .prefix("Custom system prompt")
    .build(llm)?;
```

### McpTool

Wrapper that makes MCP tools compatible with langchain-rust:

```rust
// MCP tools are automatically wrapped when using McpAgentBuilder
// or can be created manually:
let mcp_tools = mcp_client.get_langchain_tools().await?;
```

## Configuration

### McpClientConfig

```rust
use langchain_rust::mcp::McpClientConfig;

let config = McpClientConfig::new("http://127.0.0.1:8000/sse")
    .with_client_name("my-langchain-app")
    .with_client_version("1.0.0");
```

### Default Configuration

```rust
let config = McpClientConfig::default();
// server_url: "http://127.0.0.1:8000/sse"
// client_name: "langchain-rust-mcp-client"
// client_version: "0.1.0"
```

## Error Handling

The module provides comprehensive error handling through the `McpError` enum:

```rust
use langchain_rust::mcp::McpError;

match mcp_client.list_tools().await {
    Ok(tools) => println!("Found {} tools", tools.len()),
    Err(McpError::ConnectionError(e)) => eprintln!("Connection failed: {}", e),
    Err(McpError::ToolCallError(e)) => eprintln!("Tool call failed: {}", e),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Examples

See the `examples/` directory for complete examples:

- `examples/mcp_agent_example.rs` - Basic MCP agent usage
- `examples/mcp_demo_server.rs` - Simple MCP server for testing

## Testing

Run MCP-specific tests:

```bash
cargo test --features mcp mcp
```

Integration tests require a running MCP server and are marked as `#[ignore]`:

```bash
# Start the demo server first
cargo run --example mcp_demo_server

# Then run integration tests
cargo test --features mcp mcp -- --ignored
```

## Limitations

- Currently only supports SSE (Server-Sent Events) transport
- Streaming executor is temporarily disabled due to Send trait issues
- Requires MCP server to be running for integration tests

## Future Enhancements

- Support for additional MCP transport methods (stdio, HTTP)
- Streaming agent executor with proper async stream handling
- Enhanced error recovery and retry mechanisms
- Built-in MCP server discovery and health checking

## Dependencies

The MCP functionality depends on the `rmcp` crate for the underlying MCP protocol implementation.
