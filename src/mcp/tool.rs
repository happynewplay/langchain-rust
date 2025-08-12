use std::sync::Arc;

use async_trait::async_trait;
use rmcp::model::{CallToolRequestParam, InitializeRequestParam, object};
use rmcp::service::RunningService;
use rmcp::RoleClient;
use serde_json::{Map, Value};

use crate::tools::Tool;

use super::error::McpError;

/// A wrapper around an MCP tool that implements the langchain-rust Tool trait
pub struct McpTool {
    /// The underlying MCP tool definition
    tool: rmcp::model::Tool,
    /// The MCP client used to call the tool
    client: Arc<RunningService<RoleClient, InitializeRequestParam>>,
}

impl McpTool {
    /// Create a new MCP tool wrapper
    pub fn new(
        tool: rmcp::model::Tool,
        client: Arc<RunningService<RoleClient, InitializeRequestParam>>,
    ) -> Self {
        Self { tool, client }
    }

    /// Get the underlying MCP tool definition
    pub fn mcp_tool(&self) -> &rmcp::model::Tool {
        &self.tool
    }

    /// Get the MCP client
    pub fn client(&self) -> &Arc<RunningService<RoleClient, InitializeRequestParam>> {
        &self.client
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> String {
        self.tool.name.to_string()
    }

    fn description(&self) -> String {
        self.tool
            .description
            .clone()
            .unwrap_or_default()
            .to_string()
    }

    fn parameters(&self) -> Value {
        self.tool.schema_as_json_value()
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn std::error::Error>> {
        // Call the MCP tool through the client
        let response = self
            .client
            .call_tool(CallToolRequestParam {
                name: self.tool.name.clone(),
                arguments: Some(object(input)),
            })
            .await
            .map_err(|e| McpError::ToolCallError(e.to_string()))?;

        // Extract text content from the response
        let mut result = String::new();
        let raw_content = response.content.unwrap_or_default();
        for content in raw_content {
            if let Some(text) = content.as_text() {
                result.push_str(&text.text);
            }
        }

        Ok(result)
    }

    async fn parse_input(&self, input: &str) -> Value {
        match serde_json::from_str::<Map<String, Value>>(input) {
            Ok(parsed_input) => Value::Object(parsed_input),
            Err(_) => serde_json::json!({
                "value": input,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_tool_name() {
        // Create a mock MCP tool for testing
        let schema_map = serde_json::Map::new();
        let mcp_tool = rmcp::model::Tool {
            name: "test_tool".into(),
            description: Some("A test tool".into()),
            input_schema: std::sync::Arc::new(schema_map),
            annotations: None,
            output_schema: None,
        };

        // Test basic properties
        assert_eq!(mcp_tool.name, "test_tool");
        assert_eq!(mcp_tool.description.as_ref().unwrap(), "A test tool");
    }
}
