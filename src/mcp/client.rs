use std::sync::Arc;

use rmcp::model::{ClientCapabilities, ClientInfo, Implementation, InitializeRequestParam};
use rmcp::service::RunningService;
use rmcp::transport::{SseClientTransport, TokioChildProcess, ConfigureCommandExt, stdio, StreamableHttpClientTransport};
use rmcp::{RoleClient, ServiceExt};
use tokio::process::Command;

use super::error::McpError;
use super::tool::McpTool;

/// Transport type for MCP communication
#[derive(Debug, Clone)]
pub enum McpTransport {
    /// SSE (Server-Sent Events) transport
    Sse { server_url: String },
    /// Standard input/output transport
    Stdio,
    /// Child process transport with command
    ChildProcess { command: String, args: Vec<String> },
    /// Streamable HTTP transport
    StreamableHttp { server_url: String },
}

/// Configuration for MCP client
#[derive(Debug, Clone)]
pub struct McpClientConfig {
    /// Transport configuration
    pub transport: McpTransport,
    /// Client name for identification
    pub client_name: String,
    /// Client version
    pub client_version: String,
    /// Protocol version (defaults to latest)
    pub protocol_version: Option<String>,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            transport: McpTransport::Sse {
                server_url: "http://127.0.0.1:8000/sse".to_string(),
            },
            client_name: "langchain-rust-mcp-client".to_string(),
            client_version: "0.1.0".to_string(),
            protocol_version: None,
        }
    }
}

impl McpClientConfig {
    /// Create a new MCP client configuration with SSE transport
    pub fn new_sse(server_url: impl Into<String>) -> Self {
        Self {
            transport: McpTransport::Sse {
                server_url: server_url.into(),
            },
            ..Default::default()
        }
    }

    /// Create a new MCP client configuration with stdio transport
    pub fn new_stdio() -> Self {
        Self {
            transport: McpTransport::Stdio,
            ..Default::default()
        }
    }

    /// Create a new MCP client configuration with child process transport
    pub fn new_child_process(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            transport: McpTransport::ChildProcess {
                command: command.into(),
                args,
            },
            ..Default::default()
        }
    }

    /// Create a new MCP client configuration with streamable HTTP transport
    pub fn new_streamable_http(server_url: impl Into<String>) -> Self {
        Self {
            transport: McpTransport::StreamableHttp {
                server_url: server_url.into(),
            },
            ..Default::default()
        }
    }

    /// Set the client name
    pub fn with_client_name(mut self, name: impl Into<String>) -> Self {
        self.client_name = name.into();
        self
    }

    /// Set the client version
    pub fn with_client_version(mut self, version: impl Into<String>) -> Self {
        self.client_version = version.into();
        self
    }

    /// Set the protocol version
    pub fn with_protocol_version(mut self, version: impl Into<String>) -> Self {
        self.protocol_version = Some(version.into());
        self
    }
}

/// MCP client for connecting to Model Context Protocol servers
pub struct McpClient {
    /// The running MCP service
    service: Arc<RunningService<RoleClient, InitializeRequestParam>>,
    /// Client configuration
    config: McpClientConfig,
}

impl McpClient {
    /// Create a new MCP client with the given configuration
    pub async fn new(config: McpClientConfig) -> Result<Self, McpError> {
        // Create client info
        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: config.client_name.clone(),
                version: config.client_version.clone(),
            },
        };

        // Create transport based on configuration
        let service = match &config.transport {
            McpTransport::Sse { server_url } => {
                let transport = SseClientTransport::start(server_url.clone())
                    .await
                    .map_err(|e| McpError::InitializationError(format!("Failed to start SSE transport: {}", e)))?;

                client_info
                    .serve(transport)
                    .await
                    .map_err(|e| McpError::InitializationError(format!("Failed to initialize SSE service: {}", e)))?
            }
            McpTransport::Stdio => {
                let transport = stdio();

                client_info
                    .serve(transport)
                    .await
                    .map_err(|e| McpError::InitializationError(format!("Failed to initialize stdio service: {}", e)))?
            }
            McpTransport::ChildProcess { command, args } => {
                let mut cmd = Command::new(command);
                cmd.args(args);
                let transport = TokioChildProcess::new(cmd.configure(|_| {}))
                    .map_err(|e| McpError::InitializationError(format!("Failed to create child process: {}", e)))?;

                client_info
                    .serve(transport)
                    .await
                    .map_err(|e| McpError::InitializationError(format!("Failed to initialize child process service: {}", e)))?
            }
            McpTransport::StreamableHttp { server_url } => {
                let transport = StreamableHttpClientTransport::from_uri(server_url.clone());

                client_info
                    .serve(transport)
                    .await
                    .map_err(|e| McpError::InitializationError(format!("Failed to initialize streamable HTTP service: {}", e)))?
            }
        };

        Ok(Self {
            service: Arc::new(service),
            config,
        })
    }

    /// Create a new MCP client with SSE transport for the given server URL
    pub async fn connect_sse(server_url: impl Into<String>) -> Result<Self, McpError> {
        let config = McpClientConfig::new_sse(server_url);
        Self::new(config).await
    }

    /// Create a new MCP client with stdio transport
    pub async fn connect_stdio() -> Result<Self, McpError> {
        let config = McpClientConfig::new_stdio();
        Self::new(config).await
    }

    /// Create a new MCP client with child process transport
    pub async fn connect_child_process(command: impl Into<String>, args: Vec<String>) -> Result<Self, McpError> {
        let config = McpClientConfig::new_child_process(command, args);
        Self::new(config).await
    }

    /// Create a new MCP client with streamable HTTP transport
    pub async fn connect_streamable_http(server_url: impl Into<String>) -> Result<Self, McpError> {
        let config = McpClientConfig::new_streamable_http(server_url);
        Self::new(config).await
    }

    /// Create a new MCP client with default SSE configuration (backward compatibility)
    pub async fn connect(server_url: impl Into<String>) -> Result<Self, McpError> {
        Self::connect_sse(server_url).await
    }

    /// Get all available tools from the MCP server
    pub async fn list_tools(&self) -> Result<Vec<rmcp::model::Tool>, McpError> {
        self.service
            .list_all_tools()
            .await
            .map_err(|e| McpError::ToolCallError(format!("Failed to list tools: {}", e)))
    }

    /// Get all available tools as langchain-rust Tool instances
    pub async fn get_langchain_tools(&self) -> Result<Vec<Arc<dyn crate::tools::Tool>>, McpError> {
        let mcp_tools = self.list_tools().await?;
        let mut tools: Vec<Arc<dyn crate::tools::Tool>> = Vec::with_capacity(mcp_tools.len());
        
        for mcp_tool in mcp_tools {
            let tool = McpTool::new(mcp_tool, self.service.clone());
            tools.push(Arc::new(tool));
        }
        
        Ok(tools)
    }

    /// Get a specific tool by name
    pub async fn get_tool(&self, name: &str) -> Result<Option<McpTool>, McpError> {
        let tools = self.list_tools().await?;
        
        for tool in tools {
            if tool.name == name {
                return Ok(Some(McpTool::new(tool, self.service.clone())));
            }
        }
        
        Ok(None)
    }

    /// Get the underlying MCP service
    pub fn service(&self) -> &Arc<RunningService<RoleClient, InitializeRequestParam>> {
        &self.service
    }

    /// Get the client configuration
    pub fn config(&self) -> &McpClientConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_mcp_client_config_child_process() {
        let config = McpClientConfig::new_child_process("python", vec!["-m".to_string(), "mcp_server".to_string()]);

        match &config.transport {
            McpTransport::ChildProcess { command, args } => {
                assert_eq!(command, "python");
                assert_eq!(args, &vec!["-m".to_string(), "mcp_server".to_string()]);
            }
            _ => panic!("Expected child process transport"),
        }
    }

    #[test]
    fn test_mcp_client_config_streamable_http() {
        let config = McpClientConfig::new_streamable_http("http://localhost:8080/stream");

        match &config.transport {
            McpTransport::StreamableHttp { server_url } => {
                assert_eq!(server_url, "http://localhost:8080/stream");
            }
            _ => panic!("Expected streamable HTTP transport"),
        }
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
}
