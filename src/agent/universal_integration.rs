use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{
    agent::Agent,
    prompt::PromptArgs,
    schemas::agent::AgentEvent,
    tools::Tool,
};

/// Universal agent tool wrapper that can wrap any agent as a tool
pub struct UniversalAgentTool {
    /// The agent instance
    agent: Arc<dyn Agent>,
    /// Name for this tool
    name: String,
    /// Description for this tool
    description: String,
    /// Timeout for agent execution (in seconds)
    timeout: Option<u64>,
}

impl UniversalAgentTool {
    /// Create a new universal agent tool
    pub fn new<S: Into<String>>(
        agent: Arc<dyn Agent>,
        name: S,
        description: S,
    ) -> Self {
        Self {
            agent,
            name: name.into(),
            description: description.into(),
            timeout: None,
        }
    }

    /// Set timeout for agent execution
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout = Some(timeout_seconds);
        self
    }

    /// Create a universal agent tool with auto-generated name and description
    pub fn from_agent(agent: Arc<dyn Agent>) -> Self {
        let name = "agent_tool".to_string();
        let description = "A universal agent tool that can execute any agent".to_string();
        Self::new(agent, name, description)
    }
}

#[async_trait]
impl Tool for UniversalAgentTool {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn std::error::Error>> {
        // Parse input as PromptArgs
        let inputs = if input.is_object() {
            input
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            // If input is a string, use it as the "input" key
            let mut args = std::collections::HashMap::new();
            args.insert("input".to_string(), input);
            args
        };

        // Execute the agent with optional timeout
        let execution_future = async {
            match self.agent.plan(&[], inputs).await {
                Ok(AgentEvent::Finish(finish)) => Ok(finish.output),
                Ok(AgentEvent::Action(_)) => Err("Agent returned Action instead of Finish".into()),
                Err(e) => Err(e.into()),
            }
        };

        if let Some(timeout_secs) = self.timeout {
            match tokio::time::timeout(
                std::time::Duration::from_secs(timeout_secs),
                execution_future,
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(format!("Agent execution timed out after {} seconds", timeout_secs).into()),
            }
        } else {
            execution_future.await
        }
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input for the agent"
                }
            },
            "required": ["input"]
        })
    }
}

/// Agent registry for managing multiple agents as tools
pub struct AgentRegistry {
    /// Registered agents
    agents: std::collections::HashMap<String, Arc<dyn Agent>>,
    /// Default timeout for agent execution
    default_timeout: Option<u64>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: std::collections::HashMap::new(),
            default_timeout: Some(300), // 5 minutes default
        }
    }

    /// Set default timeout for all agents
    pub fn with_default_timeout(mut self, timeout_seconds: u64) -> Self {
        self.default_timeout = Some(timeout_seconds);
        self
    }

    /// Register an agent
    pub fn register<S: Into<String>>(&mut self, name: S, agent: Arc<dyn Agent>) {
        self.agents.insert(name.into(), agent);
    }

    /// Get an agent by name
    pub fn get_agent(&self, name: &str) -> Option<Arc<dyn Agent>> {
        self.agents.get(name).cloned()
    }

    /// Get all registered agent names
    pub fn agent_names(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    /// Convert all registered agents to tools
    pub fn as_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.agents
            .iter()
            .map(|(name, agent)| {
                let description = format!("Agent: {}", name);
                let tool = UniversalAgentTool::new(agent.clone(), name.clone(), description);
                
                let tool = if let Some(timeout) = self.default_timeout {
                    tool.with_timeout(timeout)
                } else {
                    tool
                };
                
                Arc::new(tool) as Arc<dyn Tool>
            })
            .collect()
    }

    /// Convert a specific agent to a tool
    pub fn agent_as_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.agents.get(name).map(|agent| {
            let description = format!("Agent: {}", name);
            let tool = UniversalAgentTool::new(agent.clone(), name, description);
            
            let tool = if let Some(timeout) = self.default_timeout {
                tool.with_timeout(timeout)
            } else {
                tool
            };
            
            Arc::new(tool) as Arc<dyn Tool>
        })
    }

    /// Create a combined tool set with regular tools and agent tools
    pub fn combined_tools(&self, regular_tools: &[Arc<dyn Tool>]) -> Vec<Arc<dyn Tool>> {
        let mut tools = regular_tools.to_vec();
        tools.extend(self.as_tools());
        tools
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for MCP integration
pub mod mcp_integration {
    use super::*;
    
    #[cfg(feature = "mcp")]
    use crate::mcp::McpClient;

    /// Create a combined tool set with regular tools, agent tools, and MCP tools
    #[cfg(feature = "mcp")]
    pub async fn create_universal_toolset(
        regular_tools: &[Arc<dyn Tool>],
        agent_registry: &AgentRegistry,
        mcp_client: Option<&McpClient>,
    ) -> Result<Vec<Arc<dyn Tool>>, Box<dyn std::error::Error>> {
        let mut tools = regular_tools.to_vec();
        
        // Add agent tools
        tools.extend(agent_registry.as_tools());
        
        // Add MCP tools if client is provided
        if let Some(client) = mcp_client {
            let mcp_tools = client.get_langchain_tools().await?;
            tools.extend(mcp_tools);
        }
        
        Ok(tools)
    }

    /// Create a universal toolset without MCP (for when MCP feature is disabled)
    pub fn create_toolset_without_mcp(
        regular_tools: &[Arc<dyn Tool>],
        agent_registry: &AgentRegistry,
    ) -> Vec<Arc<dyn Tool>> {
        agent_registry.combined_tools(regular_tools)
    }
}

/// Serialization helpers for agent responses
pub mod serialization {
    use super::*;
    use serde::{Deserialize, Serialize};

    /// Serializable agent response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SerializableAgentResponse {
        /// The output from the agent
        pub output: String,
        /// Whether the execution was successful
        pub success: bool,
        /// Error message if execution failed
        pub error: Option<String>,
        /// Execution time in milliseconds
        pub execution_time_ms: u64,
        /// Agent metadata
        pub metadata: std::collections::HashMap<String, Value>,
    }

    impl SerializableAgentResponse {
        /// Create a successful response
        pub fn success<S: Into<String>>(output: S, execution_time_ms: u64) -> Self {
            Self {
                output: output.into(),
                success: true,
                error: None,
                execution_time_ms,
                metadata: std::collections::HashMap::new(),
            }
        }

        /// Create an error response
        pub fn error<S: Into<String>>(error: S, execution_time_ms: u64) -> Self {
            Self {
                output: String::new(),
                success: false,
                error: Some(error.into()),
                execution_time_ms,
                metadata: std::collections::HashMap::new(),
            }
        }

        /// Add metadata
        pub fn with_metadata<S: Into<String>>(mut self, key: S, value: Value) -> Self {
            self.metadata.insert(key.into(), value);
            self
        }

        /// Serialize to JSON
        pub fn to_json(&self) -> Result<String, serde_json::Error> {
            serde_json::to_string(self)
        }

        /// Deserialize from JSON
        pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
            serde_json::from_str(json)
        }
    }

    /// Execute an agent and return a serializable response
    pub async fn execute_agent_serializable(
        agent: Arc<dyn Agent>,
        inputs: PromptArgs,
    ) -> SerializableAgentResponse {
        let start_time = std::time::Instant::now();

        match agent.plan(&[], inputs).await {
            Ok(AgentEvent::Finish(finish)) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                SerializableAgentResponse::success(finish.output, execution_time)
            }
            Ok(AgentEvent::Action(_)) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                SerializableAgentResponse::error("Agent returned Action instead of Finish", execution_time)
            }
            Err(e) => {
                let execution_time = start_time.elapsed().as_millis() as u64;
                SerializableAgentResponse::error(e.to_string(), execution_time)
            }
        }
    }
}
