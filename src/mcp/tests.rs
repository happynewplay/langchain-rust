#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::{McpClient, McpClientConfig, McpTransport};
    use serde_json::json;

    #[test]
    fn test_mcp_client_config_sse() {
        let config = McpClientConfig::new_sse("http://localhost:8080/sse")
            .with_client_name("test-client")
            .with_client_version("1.0.0");

        match &config.transport {
            McpTransport::Sse { server_url } => {
                assert_eq!(server_url, "http://localhost:8080/sse");
            }
            _ => panic!("Expected SSE transport"),
        }
        assert_eq!(config.client_name, "test-client");
        assert_eq!(config.client_version, "1.0.0");
    }

    #[test]
    fn test_mcp_client_config_stdio() {
        let config = McpClientConfig::new_stdio()
            .with_client_name("stdio-client");

        match &config.transport {
            McpTransport::Stdio => {},
            _ => panic!("Expected stdio transport"),
        }
        assert_eq!(config.client_name, "stdio-client");
    }

    #[test]
    fn test_default_config() {
        let config = McpClientConfig::default();
        match &config.transport {
            McpTransport::Sse { server_url } => {
                assert_eq!(server_url, "http://127.0.0.1:8000/sse");
            }
            _ => panic!("Expected SSE transport as default"),
        }
        assert_eq!(config.client_name, "langchain-rust-mcp-client");
        assert_eq!(config.client_version, "0.1.0");
    }

    #[test]
    fn test_mcp_tool_creation() {
        // Create a mock MCP tool for testing
        let schema_map = serde_json::Map::new();
        let mcp_tool = rmcp::model::Tool {
            name: "test_tool".into(),
            description: Some("A test tool".into()),
            input_schema: std::sync::Arc::new(schema_map),
            annotations: None,
            output_schema: None,
        };

        // Test that we can create the tool wrapper
        // Note: We can't test the actual functionality without a real MCP server
        assert_eq!(mcp_tool.name, "test_tool");
        assert_eq!(mcp_tool.description.as_ref().unwrap(), "A test tool");
    }

    #[test]
    fn test_mcp_tool_parameters() {
        let mut schema_map = serde_json::Map::new();
        schema_map.insert("type".to_string(), json!("object"));

        let mcp_tool = rmcp::model::Tool {
            name: "sum".into(),
            description: Some("Add two numbers".into()),
            input_schema: std::sync::Arc::new(schema_map),
            annotations: None,
            output_schema: None,
        };

        let schema = mcp_tool.schema_as_json_value();
        assert_eq!(schema["type"], "object");
    }

    // Integration tests would require a running MCP server
    // These are marked as ignored and can be run manually when a server is available
    
    #[tokio::test]
    #[ignore]
    async fn test_mcp_client_connection_sse() {
        let config = McpClientConfig::new_sse("http://127.0.0.1:8000/sse");
        let result = McpClient::new(config).await;
        
        match result {
            Ok(client) => {
                println!("Successfully connected to MCP server");
                
                // Test listing tools
                let tools_result = client.list_tools().await;
                match tools_result {
                    Ok(tools) => {
                        println!("Found {} tools", tools.len());
                        for tool in tools {
                            println!("Tool: {} - {}", tool.name, tool.description.unwrap_or_default());
                        }
                    }
                    Err(e) => println!("Failed to list tools: {}", e),
                }
            }
            Err(e) => println!("Failed to connect to MCP server: {}", e),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_mcp_tool_execution() {
        let config = McpClientConfig::new_sse("http://127.0.0.1:8000/sse");
        let client = McpClient::new(config).await.expect("Failed to connect to MCP server");
        
        // Get tools as langchain tools
        let tools = client.get_langchain_tools().await.expect("Failed to get tools");
        
        // Find the sum tool
        let sum_tool = tools.iter().find(|t| t.name() == "sum");
        
        if let Some(tool) = sum_tool {
            // Test the tool
            let input = json!({"a": 3, "b": 5});
            let result = tool.run(input).await.expect("Tool execution failed");
            assert_eq!(result, "8");
            println!("Sum tool test passed: 3 + 5 = {}", result);
        } else {
            panic!("Sum tool not found");
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_mcp_client_stdio() {
        // This test requires a stdio-based MCP server to be available
        let result = McpClient::connect_stdio().await;

        match result {
            Ok(client) => {
                println!("Successfully connected to MCP server via stdio");

                // Test listing tools
                let tools_result = client.list_tools().await;
                match tools_result {
                    Ok(tools) => {
                        println!("Found {} tools via stdio", tools.len());
                    }
                    Err(e) => println!("Failed to list tools via stdio: {}", e),
                }
            }
            Err(e) => println!("Failed to connect to MCP server via stdio: {}", e),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_mcp_client_child_process() {
        // This test requires a Python MCP server to be available
        let result = McpClient::connect_child_process("python", vec!["-m".to_string(), "mcp_server".to_string()]).await;

        match result {
            Ok(client) => {
                println!("Successfully connected to MCP server via child process");

                // Test listing tools
                let tools_result = client.list_tools().await;
                match tools_result {
                    Ok(tools) => {
                        println!("Found {} tools via child process", tools.len());
                    }
                    Err(e) => println!("Failed to list tools via child process: {}", e),
                }
            }
            Err(e) => println!("Failed to connect to MCP server via child process: {}", e),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_mcp_client_streamable_http() {
        // This test requires a streamable HTTP MCP server to be available
        let result = McpClient::connect_streamable_http("http://127.0.0.1:8000/stream").await;

        match result {
            Ok(client) => {
                println!("Successfully connected to MCP server via streamable HTTP");

                // Test listing tools
                let tools_result = client.list_tools().await;
                match tools_result {
                    Ok(tools) => {
                        println!("Found {} tools via streamable HTTP", tools.len());
                    }
                    Err(e) => println!("Failed to list tools via streamable HTTP: {}", e),
                }
            }
            Err(e) => println!("Failed to connect to MCP server via streamable HTTP: {}", e),
        }
    }
}
