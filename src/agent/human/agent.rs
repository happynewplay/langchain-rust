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
    config::HumanAgentConfig,
    interaction::{HumanInteractionInterface, HumanInteractionManager, InteractionContext},
};

/// A human agent that can request human intervention based on conditions
pub struct HumanAgent {
    /// Configuration for the human agent
    config: HumanAgentConfig,
    /// Manager for human interactions
    interaction_manager: HumanInteractionManager,
    /// Tools available to this agent
    tools: Vec<Arc<dyn Tool>>,
}

impl HumanAgent {
    /// Create a new human agent with console interface
    pub fn new(config: HumanAgentConfig) -> Result<Self, AgentError> {
        config.validate().map_err(|e| AgentError::OtherError(e))?;
        
        let interaction_manager = HumanInteractionManager::with_console(config.clone());
        
        Ok(Self {
            config,
            interaction_manager,
            tools: Vec::new(),
        })
    }

    /// Create a new human agent with custom interface
    pub fn with_interface(
        config: HumanAgentConfig,
        interface: Box<dyn HumanInteractionInterface>,
    ) -> Result<Self, AgentError> {
        config.validate().map_err(|e| AgentError::OtherError(e))?;
        
        let interaction_manager = HumanInteractionManager::new(config.clone(), interface);
        
        Ok(Self {
            config,
            interaction_manager,
            tools: Vec::new(),
        })
    }

    /// Add tools to the human agent
    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools = tools;
        self
    }

    /// Get the human agent configuration
    pub fn config(&self) -> &HumanAgentConfig {
        &self.config
    }

    /// Get current intervention count
    pub fn intervention_count(&self) -> u32 {
        self.interaction_manager.intervention_count()
    }

    /// Process input and determine if human intervention is needed
    async fn process_with_human_intervention(
        &mut self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        // Extract input for context
        let input_text = inputs
            .get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Create interaction context
        let mut context = InteractionContext::new(input_text);
        
        // Add intermediate steps to context
        if !intermediate_steps.is_empty() {
            let steps_summary = intermediate_steps
                .iter()
                .map(|(action, observation)| format!("Tool: {}, Result: {}", action.tool, observation))
                .collect::<Vec<_>>()
                .join("; ");
            context = context.with_additional("intermediate_steps", steps_summary);
        }

        // Add prefix to context if available
        if let Some(prefix) = &self.config.prefix {
            context = context.with_additional("system_prompt", prefix.clone());
        }

        // Add memory context if available and configured
        if let Some(memory) = &self.config.memory {
            if self.config.include_memory_in_prompts {
                let memory_guard = memory.lock().await;
                let chat_history = memory_guard.messages();
                let history_summary = chat_history
                    .iter()
                    .map(|msg| format!("{:?}: {}", msg.message_type, msg.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                context = context.with_additional("chat_history", history_summary);
            }
        }

        // Check for termination first
        if self.interaction_manager.should_terminate(&context) {
            return Ok(AgentEvent::Finish(AgentFinish {
                output: "Termination condition met - ending execution".to_string(),
            }));
        }

        // Check if human intervention is needed
        if self.interaction_manager.should_intervene(&context) {
            // Display current context to human
            self.interaction_manager
                .display_info("Human intervention triggered")
                .await?;

            // Request human input
            let interaction_result = self
                .interaction_manager
                .request_human_input(&context, None)
                .await?;

            if interaction_result.terminated {
                return Ok(AgentEvent::Finish(AgentFinish {
                    output: interaction_result.response,
                }));
            }

            if !interaction_result.success {
                return Err(AgentError::OtherError(
                    interaction_result
                        .error
                        .unwrap_or_else(|| "Human interaction failed".to_string()),
                ));
            }

            // Return human response as the final output
            Ok(AgentEvent::Finish(AgentFinish {
                output: interaction_result.response,
            }))
        } else {
            // No intervention needed, return a default response
            let default_response = format!(
                "Processed input: {}. No human intervention required.",
                context.input
            );
            
            Ok(AgentEvent::Finish(AgentFinish {
                output: default_response,
            }))
        }
    }
}

#[async_trait]
impl Agent for HumanAgent {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        // Note: We need to make self mutable for interaction_manager, but the trait doesn't allow it
        // For now, we'll create a new manager for each call
        // In a real implementation, you might want to use Arc<Mutex<>> or similar
        
        let mut temp_manager = HumanInteractionManager::with_console(self.config.clone());
        
        // Extract input for context
        let input_text = inputs
            .get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Create interaction context
        let mut context = InteractionContext::new(input_text);
        
        // Add intermediate steps to context
        if !intermediate_steps.is_empty() {
            let steps_summary = intermediate_steps
                .iter()
                .map(|(action, observation)| format!("Tool: {}, Result: {}", action.tool, observation))
                .collect::<Vec<_>>()
                .join("; ");
            context = context.with_additional("intermediate_steps", steps_summary);
        }

        // Add prefix to context if available
        if let Some(prefix) = &self.config.prefix {
            context = context.with_additional("system_prompt", prefix.clone());
        }

        // Check for termination first
        if temp_manager.should_terminate(&context) {
            return Ok(AgentEvent::Finish(AgentFinish {
                output: "Termination condition met - ending execution".to_string(),
            }));
        }

        // Check if human intervention is needed
        if temp_manager.should_intervene(&context) {
            // Display current context to human
            temp_manager
                .display_info("Human intervention triggered")
                .await?;

            // Request human input
            let interaction_result = temp_manager
                .request_human_input(&context, None)
                .await?;

            if interaction_result.terminated {
                return Ok(AgentEvent::Finish(AgentFinish {
                    output: interaction_result.response,
                }));
            }

            if !interaction_result.success {
                return Err(AgentError::OtherError(
                    interaction_result
                        .error
                        .unwrap_or_else(|| "Human interaction failed".to_string()),
                ));
            }

            // Return human response as the final output
            Ok(AgentEvent::Finish(AgentFinish {
                output: interaction_result.response,
            }))
        } else {
            // No intervention needed, return a default response
            let default_response = format!(
                "Processed input: {}. No human intervention required.",
                context.input
            );
            
            Ok(AgentEvent::Finish(AgentFinish {
                output: default_response,
            }))
        }
    }

    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}

/// Tool wrapper that allows a human agent to be used as a tool by other agents
pub struct HumanAgentTool {
    /// The human agent instance
    human_agent: Arc<HumanAgent>,
    /// Name for this tool
    name: String,
    /// Description for this tool
    description: String,
}

impl HumanAgentTool {
    /// Create a new human agent tool
    pub fn new<S: Into<String>>(
        human_agent: Arc<HumanAgent>,
        name: S,
        description: S,
    ) -> Self {
        Self {
            human_agent,
            name: name.into(),
            description: description.into(),
        }
    }

    /// Create a human agent tool with default name and description
    pub fn from_human_agent(human_agent: Arc<HumanAgent>) -> Self {
        let name = "human_agent".to_string();
        let description = "A human agent that can request human intervention based on configured conditions".to_string();

        Self::new(human_agent, name, description)
    }
}

#[async_trait]
impl Tool for HumanAgentTool {
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

        // Execute the human agent
        match self.human_agent.plan(&[], inputs).await {
            Ok(AgentEvent::Finish(finish)) => Ok(finish.output),
            Ok(AgentEvent::Action(_)) => Err("Human agent returned Action instead of Finish".into()),
            Err(e) => Err(e.into()),
        }
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input for the human agent"
                }
            },
            "required": ["input"]
        })
    }
}
