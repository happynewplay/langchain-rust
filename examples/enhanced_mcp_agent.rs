#[cfg(feature = "mcp")]
use std::sync::Arc;

#[cfg(feature = "mcp")]
use langchain_rust::{
    agent::{McpAgentEvent, McpAgentExecutor, McpExecutionConfig, OpenAiToolAgentBuilder},
    llm::openai::OpenAI,
    mcp::{McpClient, McpClientConfig, McpTransport},
    prompt_args,
};

/// Example demonstrating enhanced MCP agent with multiple tool calling support
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "mcp")]
    {
        // Initialize logging
        env_logger::init();

        // Create MCP client configuration for different transport types
        let mcp_configs = vec![
            McpClientConfig::new_sse("http://127.0.0.1:8000/sse")
                .with_client_name("enhanced-mcp-agent")
                .with_client_version("1.0.0"),
            McpClientConfig::new_child_process(
                "python",
                vec!["-m".to_string(), "mcp_server".to_string()],
            ),
            McpClientConfig::new_stdio(),
        ];

        // Try to connect to an MCP server (using the first available config)
        let mcp_client = match McpClient::new(mcp_configs[0].clone()).await {
            Ok(client) => client,
            Err(e) => {
                println!("Failed to connect to MCP server: {}", e);
                println!(
                    "This example requires an MCP server running at http://127.0.0.1:8000/sse"
                );
                return Ok(());
            }
        };

        // Get MCP tools
        let mcp_tools = mcp_client.get_langchain_tools().await?;
        println!("Connected to MCP server with {} tools", mcp_tools.len());

        // List available tools
        for tool in &mcp_tools {
            println!("- {}: {}", tool.name(), tool.description());
        }

        // Create LLM
        let llm = OpenAI::default();

        // Create agent with MCP tools
        let agent = OpenAiToolAgentBuilder::new()
            .tools(&mcp_tools)
            .prefix("You are an AI assistant with access to MCP tools. You can call multiple tools in parallel when appropriate.")
            .build(llm)?;

        // Create enhanced MCP executor with parallel execution enabled
        let mcp_config = McpExecutionConfig {
            parallel_execution: true,
            max_parallel_calls: 3,
            tool_timeout_ms: 30000,
            retry_on_failure: true,
            max_retries: 2,
        };

        let executor = McpAgentExecutor::new(Arc::new(agent))
            .with_mcp_config(mcp_config)
            .with_max_iterations(10)
            .with_break_on_error(false);

        // Example 1: Single tool call
        println!("\n=== Example 1: Single Tool Call ===");
        let result = executor
            .invoke(prompt_args! {
                "input" => "What tools are available?"
            })
            .await?;
        println!("Result: {}", result);

        // Example 2: Multiple tool calls (if multiple tools are available)
        if mcp_tools.len() > 1 {
            println!("\n=== Example 2: Multiple Tool Calls ===");
            let result = executor.invoke(prompt_args! {
                "input" => "Use multiple tools to gather comprehensive information about the current system status"
            }).await?;
            println!("Result: {}", result);
        }

        // Example 3: Streaming execution with event monitoring
        println!("\n=== Example 3: Streaming Execution ===");
        let mut stream = executor
            .stream(prompt_args! {
                "input" => "Perform a comprehensive analysis using all available tools"
            })
            .await?;

        use futures_util::StreamExt;
        while let Some(event) = stream.next().await {
            match event? {
                McpAgentEvent::Planning => {
                    println!("🤔 Agent is planning...");
                }
                McpAgentEvent::ToolCall { tool_name, .. } => {
                    println!("🔧 Calling tool: {}", tool_name);
                }
                McpAgentEvent::ParallelToolCalls { tool_names, count } => {
                    println!("⚡ Calling {} tools in parallel: {:?}", count, tool_names);
                }
                McpAgentEvent::ToolResult {
                    tool_name,
                    execution_time_ms,
                    ..
                } => {
                    println!("✅ Tool {} completed in {}ms", tool_name, execution_time_ms);
                }
                McpAgentEvent::ParallelToolResults { results } => {
                    println!("⚡ Parallel execution completed:");
                    for (tool_name, _, time_ms) in results {
                        println!("  - {}: {}ms", tool_name, time_ms);
                    }
                }
                McpAgentEvent::McpError {
                    error,
                    tool_name,
                    recoverable,
                } => {
                    println!(
                        "❌ MCP Error in {}: {} (recoverable: {})",
                        tool_name, error, recoverable
                    );
                }
                McpAgentEvent::Error { error } => {
                    println!("❌ Error: {}", error);
                }
                McpAgentEvent::Finished { output } => {
                    println!("🎉 Execution completed!");
                    println!("Final output: {}", output);
                    break;
                }
            }
        }

        // Example 4: Configuration variations
        println!("\n=== Example 4: Sequential Execution Mode ===");
        let sequential_executor = McpAgentExecutor::new(Arc::new(
            OpenAiToolAgentBuilder::new()
                .tools(&mcp_tools)
                .prefix("You are an AI assistant. Execute tools sequentially for careful analysis.")
                .build(OpenAI::default())?,
        ))
        .with_parallel_execution(false)
        .with_max_iterations(5);

        let result = sequential_executor
            .invoke(prompt_args! {
                "input" => "Analyze the system step by step using available tools"
            })
            .await?;
        println!("Sequential result: {}", result);

        println!("\n✨ Enhanced MCP agent demonstration completed!");
    }

    #[cfg(not(feature = "mcp"))]
    {
        println!("This example requires the 'mcp' feature to be enabled.");
        println!("Run with: cargo run --example enhanced_mcp_agent --features mcp");
    }

    Ok(())
}

/// Helper function to demonstrate MCP tool capabilities
#[cfg(feature = "mcp")]
async fn demonstrate_mcp_capabilities(
    client: &McpClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MCP Server Capabilities ===");

    // List all available tools
    let tools = client.list_tools().await?;
    println!("Available tools: {}", tools.len());

    for tool in &tools {
        println!("Tool: {}", tool.name);
        if let Some(description) = &tool.description {
            println!("  Description: {}", description);
        }
        println!(
            "  Schema: {}",
            serde_json::to_string_pretty(&tool.input_schema)?
        );
    }

    Ok(())
}

/// Configuration examples for different MCP setups
#[cfg(feature = "mcp")]
fn create_mcp_configs() -> Vec<McpClientConfig> {
    vec![
        // SSE transport for web-based MCP servers
        McpClientConfig::new_sse("http://localhost:8000/sse")
            .with_client_name("langchain-rust-client")
            .with_client_version("1.0.0"),
        // Child process for Python MCP servers
        McpClientConfig::new_child_process(
            "python",
            vec!["-m".to_string(), "my_mcp_server".to_string()],
        ),
        // Stdio for direct communication
        McpClientConfig::new_stdio(),
        // Streamable HTTP for HTTP-based servers
        McpClientConfig::new_streamable_http("http://localhost:8080/stream"),
    ]
}
