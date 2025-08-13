use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::{
    agent::{Agent, AgentError},
    prompt::PromptArgs,
    schemas::agent::{AgentAction, AgentEvent, AgentFinish},
    tools::Tool,
};

use super::{
    config::TeamAgentConfig,
    execution::{TeamExecutor, TeamExecutionResult},
};

/// A team agent that orchestrates multiple child agents
pub struct TeamAgent {
    /// Configuration for the team
    config: TeamAgentConfig,
    /// Executor for handling team execution patterns
    executor: TeamExecutor,
    /// Combined tools from all child agents
    tools: Vec<Arc<dyn Tool>>,
}

impl TeamAgent {
    /// Create a new team agent
    pub fn new(config: TeamAgentConfig) -> Result<Self, AgentError> {
        // Validate configuration
        config.validate().map_err(|e| AgentError::OtherError(e))?;

        // Create executor
        let executor = TeamExecutor::new(config.clone())?;

        // Collect all tools from child agents
        let mut tools = Vec::new();
        for child in &config.child_agents {
            tools.extend(child.agent.get_tools());
        }

        Ok(Self {
            config,
            executor,
            tools,
        })
    }

    /// Get the team configuration
    pub fn config(&self) -> &TeamAgentConfig {
        &self.config
    }

    /// Get the number of child agents
    pub fn child_count(&self) -> usize {
        self.config.child_agents.len()
    }

    /// Get child agent IDs
    pub fn child_agent_ids(&self) -> Vec<String> {
        self.config
            .child_agents
            .iter()
            .map(|c| c.id.clone())
            .collect()
    }

    /// Execute the team and format the result for the agent interface
    async fn execute_team(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<TeamExecutionResult, AgentError> {
        // Add team context to inputs
        let mut team_inputs = inputs.clone();

        // Add team metadata
        team_inputs.insert("team_agent_id".to_string(), json!("team"));
        team_inputs.insert(
            "child_agent_ids".to_string(),
            json!(self.child_agent_ids()),
        );
        team_inputs.insert(
            "execution_pattern".to_string(),
            json!(format!("{:?}", self.config.execution_pattern)),
        );

        // Add team prefix if configured
        if let Some(prefix) = &self.config.prefix {
            team_inputs.insert("team_prefix".to_string(), json!(prefix));
        }

        // Add memory context if available
        if let Some(memory) = &self.config.memory {
            let memory_guard = memory.lock().await;
            let chat_history = memory_guard.messages();
            team_inputs.insert("chat_history".to_string(), json!(chat_history));

            if self.config.use_coordination_prompts {
                let coordination_context = format!(
                    "Team coordination context: {} child agents executing in {:?} pattern",
                    self.config.child_agents.len(),
                    self.config.execution_pattern
                );
                team_inputs.insert("coordination_context".to_string(), json!(coordination_context));
            }
        }

        // Execute the team
        self.executor
            .execute(intermediate_steps, team_inputs)
            .await
    }

    /// Format team execution result into a readable output
    fn format_team_output(&self, result: &TeamExecutionResult) -> String {
        let mut output = String::new();

        // Add team summary
        output.push_str(&format!(
            "Team Execution Summary:\n- Total agents: {}\n- Successful: {}\n- Execution time: {}ms\n\n",
            result.child_results.len(),
            result.child_results.iter().filter(|r| r.success).count(),
            result.total_execution_time_ms
        ));

        // Add individual agent results
        output.push_str("Individual Agent Results:\n");
        for (idx, child_result) in result.child_results.iter().enumerate() {
            output.push_str(&format!(
                "{}. Agent '{}' ({}ms): {}\n",
                idx + 1,
                child_result.agent_id,
                child_result.execution_time_ms,
                if child_result.success {
                    "SUCCESS"
                } else {
                    "FAILED"
                }
            ));

            if child_result.success {
                output.push_str(&format!("   Output: {}\n", child_result.output));
            } else if let Some(error) = &child_result.error {
                output.push_str(&format!("   Error: {}\n", error));
            }
            output.push('\n');
        }

        // Add final aggregated output
        output.push_str("Final Aggregated Output:\n");
        output.push_str(&result.final_output);

        output
    }
}

#[async_trait]
impl Agent for TeamAgent {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        // Execute the team
        let team_result = self.execute_team(intermediate_steps, inputs).await?;

        // Format the output
        let formatted_output = self.format_team_output(&team_result);

        // Return as AgentFinish
        Ok(AgentEvent::Finish(AgentFinish {
            output: formatted_output,
        }))
    }

    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}

/// Tool wrapper that allows a team agent to be used as a tool by other agents
pub struct TeamAgentTool {
    /// The team agent instance
    team_agent: Arc<TeamAgent>,
    /// Name for this tool
    name: String,
    /// Description for this tool
    description: String,
}

impl TeamAgentTool {
    /// Create a new team agent tool
    pub fn new<S: Into<String>>(
        team_agent: Arc<TeamAgent>,
        name: S,
        description: S,
    ) -> Self {
        Self {
            team_agent,
            name: name.into(),
            description: description.into(),
        }
    }

    /// Create a team agent tool with default name and description
    pub fn from_team_agent(team_agent: Arc<TeamAgent>) -> Self {
        let child_ids = team_agent.child_agent_ids();
        let name = format!("team_agent_{}", child_ids.len());
        let description = format!(
            "A team agent that coordinates {} child agents: {}",
            child_ids.len(),
            child_ids.join(", ")
        );

        Self::new(team_agent, name, description)
    }
}

#[async_trait]
impl Tool for TeamAgentTool {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    async fn run(&self, input: serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
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

        // Execute the team agent
        match self.team_agent.plan(&[], inputs).await {
            Ok(AgentEvent::Finish(finish)) => Ok(finish.output),
            Ok(AgentEvent::Action(_)) => Err("Team agent returned Action instead of Finish".into()),
            Err(e) => Err(e.into()),
        }
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input for the team agent"
                }
            },
            "required": ["input"]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::{Agent, AgentError},
        prompt::PromptArgs,
        schemas::agent::{AgentEvent, AgentFinish},
        tools::Tool,
    };
    use async_trait::async_trait;
    use std::sync::Arc;
    use super::super::config::{ChildAgentConfig, ExecutionPattern};

    // Mock agent for testing
    struct MockAgent {
        name: String,
        response: String,
    }

    impl MockAgent {
        fn new(name: &str, response: &str) -> Self {
            Self {
                name: name.to_string(),
                response: response.to_string(),
            }
        }
    }

    #[async_trait]
    impl Agent for MockAgent {
        async fn plan(
            &self,
            _intermediate_steps: &[(crate::schemas::agent::AgentAction, String)],
            inputs: PromptArgs,
        ) -> Result<AgentEvent, AgentError> {
            let input = inputs.get("input").and_then(|v| v.as_str()).unwrap_or("");
            let output = format!("{}: processed '{}'", self.name, input);
            Ok(AgentEvent::Finish(AgentFinish { output }))
        }

        fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
            vec![]
        }
    }

    #[tokio::test]
    async fn test_sequential_team_agent() {
        let agent_a = Arc::new(MockAgent::new("Agent A", "result A"));
        let agent_b = Arc::new(MockAgent::new("Agent B", "result B"));

        let config = TeamAgentConfig::new()
            .add_child_agent(ChildAgentConfig::new("agent_a", agent_a))
            .add_child_agent(ChildAgentConfig::new("agent_b", agent_b))
            .with_execution_pattern(ExecutionPattern::Sequential);

        let team_agent = TeamAgent::new(config).unwrap();

        let inputs = std::collections::HashMap::from([
            ("input".to_string(), serde_json::json!("test input"))
        ]);

        let result = team_agent.plan(&[], inputs).await.unwrap();

        match result {
            AgentEvent::Finish(finish) => {
                assert!(finish.output.contains("Agent A"));
                assert!(finish.output.contains("Agent B"));
                assert!(finish.output.contains("test input"));
            }
            _ => panic!("Expected AgentFinish"),
        }
    }

    #[tokio::test]
    async fn test_concurrent_team_agent() {
        let agent_a = Arc::new(MockAgent::new("Agent A", "result A"));
        let agent_b = Arc::new(MockAgent::new("Agent B", "result B"));

        let config = TeamAgentConfig::new()
            .add_child_agent(ChildAgentConfig::new("agent_a", agent_a))
            .add_child_agent(ChildAgentConfig::new("agent_b", agent_b))
            .with_execution_pattern(ExecutionPattern::Concurrent);

        let team_agent = TeamAgent::new(config).unwrap();

        let inputs = std::collections::HashMap::from([
            ("input".to_string(), serde_json::json!("test input"))
        ]);

        let result = team_agent.plan(&[], inputs).await.unwrap();

        match result {
            AgentEvent::Finish(finish) => {
                assert!(finish.output.contains("Agent A"));
                assert!(finish.output.contains("Agent B"));
            }
            _ => panic!("Expected AgentFinish"),
        }
    }

    #[tokio::test]
    async fn test_team_agent_validation() {
        // Test empty team validation
        let config = TeamAgentConfig::new();
        assert!(TeamAgent::new(config).is_err());

        // Test duplicate ID validation
        let agent = Arc::new(MockAgent::new("Agent", "result"));
        let config = TeamAgentConfig::new()
            .add_child_agent(ChildAgentConfig::new("same_id", agent.clone()))
            .add_child_agent(ChildAgentConfig::new("same_id", agent));

        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_team_agent_as_tool() {
        let agent_a = Arc::new(MockAgent::new("Agent A", "result A"));
        let config = TeamAgentConfig::new()
            .add_child_agent(ChildAgentConfig::new("agent_a", agent_a));

        let team_agent = Arc::new(TeamAgent::new(config).unwrap());
        let tool = TeamAgentTool::from_team_agent(team_agent);

        let result = tool.run(serde_json::json!("test input")).await.unwrap();
        assert!(result.contains("Agent A"));
        assert!(result.contains("test input"));
    }
}
