use std::sync::Arc;

use crate::{
    agent::{AgentError, OpenAiToolAgent, OpenAiToolAgentBuilder},
    chain::options::ChainCallOptions,
    language_models::llm::LLM,
    tools::Tool,
};

#[cfg(feature = "mcp")]
use crate::mcp::{McpClient, McpError};

/// Builder for creating OpenAI tool agents with MCP (Model Context Protocol) support
pub struct McpAgentBuilder {
    /// Regular tools (non-MCP)
    tools: Option<Vec<Arc<dyn Tool>>>,
    /// MCP tools
    #[cfg(feature = "mcp")]
    mcp_tools: Option<Vec<Arc<dyn Tool>>>,
    /// Agent prefix/system prompt
    prefix: Option<String>,
    /// Chain call options
    options: Option<ChainCallOptions>,
}

impl McpAgentBuilder {
    /// Create a new MCP agent builder
    pub fn new() -> Self {
        Self {
            tools: None,
            #[cfg(feature = "mcp")]
            mcp_tools: None,
            prefix: None,
            options: None,
        }
    }

    /// Add regular (non-MCP) tools to the agent
    pub fn tools(mut self, tools: &[Arc<dyn Tool>]) -> Self {
        self.tools = Some(tools.to_vec());
        self
    }

    /// Add MCP tools from an MCP client
    #[cfg(feature = "mcp")]
    pub async fn mcp_tools(mut self, mcp_client: &McpClient) -> Result<Self, McpError> {
        let tools = mcp_client.get_langchain_tools().await?;
        self.mcp_tools = Some(tools);
        Ok(self)
    }

    /// Add MCP tools directly
    #[cfg(feature = "mcp")]
    pub fn mcp_tools_direct(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.mcp_tools = Some(tools);
        self
    }

    /// Set the agent prefix/system prompt
    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set chain call options
    pub fn options(mut self, options: ChainCallOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Build the agent with the specified LLM
    pub fn build<L: LLM + 'static>(self, llm: L) -> Result<OpenAiToolAgent, AgentError> {
        // Combine regular tools and MCP tools
        let mut all_tools = self.tools.unwrap_or_default();
        
        #[cfg(feature = "mcp")]
        if let Some(mcp_tools) = self.mcp_tools {
            all_tools.extend(mcp_tools);
        }

        // Use the existing OpenAI tool agent builder
        let mut builder = OpenAiToolAgentBuilder::new().tools(&all_tools);

        if let Some(prefix) = self.prefix {
            builder = builder.prefix(prefix);
        }

        if let Some(options) = self.options {
            builder = builder.options(options);
        }

        builder.build(llm)
    }
}

impl Default for McpAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_agent_builder_creation() {
        let builder = McpAgentBuilder::new();
        assert!(builder.tools.is_none());
        #[cfg(feature = "mcp")]
        assert!(builder.mcp_tools.is_none());
        assert!(builder.prefix.is_none());
        assert!(builder.options.is_none());
    }

    #[test]
    fn test_mcp_agent_builder_with_prefix() {
        let builder = McpAgentBuilder::new().prefix("Test prefix");
        assert_eq!(builder.prefix.as_ref().unwrap(), "Test prefix");
    }
}
