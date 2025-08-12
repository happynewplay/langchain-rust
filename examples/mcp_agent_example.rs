#[cfg(feature = "mcp")]
use langchain_rust::{
    agent::{AgentExecutor, McpAgentBuilder},
    chain::Chain,
    llm::openai::OpenAI,
    mcp::{McpClient, McpClientConfig},
    prompt_args,
};

#[cfg(not(feature = "mcp"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("This example requires the 'mcp' feature to be enabled.");
    println!("Run with: cargo run --example mcp_agent_example --features mcp");
    Ok(())
}

#[cfg(feature = "mcp")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("🚀 Starting MCP Agent Example...");

    // Configure MCP client - you can choose different transport methods:

    // Option 1: SSE (Server-Sent Events) transport
    let mcp_config = McpClientConfig::new_sse("http://127.0.0.1:8000/sse")
        .with_client_name("langchain-rust-example")
        .with_client_version("1.0.0");

    // Option 2: Stdio transport (uncomment to use)
    // let mcp_config = McpClientConfig::new_stdio()
    //     .with_client_name("langchain-rust-example")
    //     .with_client_version("1.0.0");

    // Option 3: Child process transport (uncomment to use)
    // let mcp_config = McpClientConfig::new_child_process("python", vec!["-m".to_string(), "mcp_server".to_string()])
    //     .with_client_name("langchain-rust-example")
    //     .with_client_version("1.0.0");

    // Option 4: Streamable HTTP transport (uncomment to use)
    // let mcp_config = McpClientConfig::new_streamable_http("http://127.0.0.1:8000/stream")
    //     .with_client_name("langchain-rust-example")
    //     .with_client_version("1.0.0");

    // Connect to MCP server
    println!("🔗 Connecting to MCP server...");
    let mcp_client = McpClient::new(mcp_config).await?;

    // List available tools
    let tools = mcp_client.list_tools().await?;
    println!("📋 Available MCP tools:");
    for tool in &tools {
        println!("  - {}: {}", tool.name, tool.description.as_deref().unwrap_or("No description"));
    }

    // Create OpenAI LLM
    let llm = OpenAI::default();

    // Build agent with MCP tools
    println!("🤖 Building agent with MCP tools...");
    let agent_builder = McpAgentBuilder::new()
        .mcp_tools(&mcp_client)
        .await?
        .prefix(
            "You are a helpful AI assistant with access to mathematical tools. \
             When asked to perform calculations, use the available tools to get accurate results. \
             Always explain your reasoning and show the step-by-step process."
        );

    let agent = agent_builder.build(llm)?;

    // Create executor (using standard AgentExecutor for now)
    let executor = AgentExecutor::from_agent(agent)
        .with_max_iterations(10)
        .with_break_if_error(true);

    // Example calculation task
    let input_variables = prompt_args! {
        "input" => "Calculate the factorial of 5, then add 10 to the result."
    };

    println!("💭 Task: Calculate the factorial of 5, then add 10 to the result.");
    println!("🔄 Executing...\n");

    // Execute the agent
    let result = executor.invoke(input_variables).await?;
    println!("🎉 Final result: {}", result);

    println!("\n✨ MCP Agent Example completed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running MCP server
    async fn test_mcp_client_connection() {
        let config = McpClientConfig::new("http://127.0.0.1:8000/sse");
        let result = McpClient::new(config).await;
        
        // This test would pass if an MCP server is running
        // For CI/CD, this should be ignored or use a mock server
        match result {
            Ok(_) => println!("Successfully connected to MCP server"),
            Err(e) => println!("Failed to connect to MCP server: {}", e),
        }
    }
}
