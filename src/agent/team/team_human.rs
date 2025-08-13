use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::{
    agent::{
        human::{HumanAgentConfig, HumanInteractionInterface, HumanInteractionManager, InteractionContext},
        Agent, AgentError,
    },
    prompt::PromptArgs,
    schemas::agent::{AgentAction, AgentEvent, AgentFinish},
    tools::Tool,
};

use super::{
    config::TeamAgentConfig,
    execution::TeamExecutor,
};

/// Configuration for team-human hybrid agent
#[derive(Clone)]
pub struct TeamHumanAgentConfig {
    /// Team agent configuration
    pub team_config: TeamAgentConfig,
    /// Human agent configuration
    pub human_config: HumanAgentConfig,
    /// Whether to check for human intervention before team execution
    pub intervene_before_team: bool,
    /// Whether to check for human intervention after team execution
    pub intervene_after_team: bool,
    /// Whether to check for human intervention on team errors
    pub intervene_on_team_error: bool,
}

impl TeamHumanAgentConfig {
    /// Create a new team-human agent configuration
    pub fn new(team_config: TeamAgentConfig, human_config: HumanAgentConfig) -> Self {
        Self {
            team_config,
            human_config,
            intervene_before_team: true,
            intervene_after_team: false,
            intervene_on_team_error: true,
        }
    }

    /// Set whether to intervene before team execution
    pub fn with_intervene_before_team(mut self, intervene: bool) -> Self {
        self.intervene_before_team = intervene;
        self
    }

    /// Set whether to intervene after team execution
    pub fn with_intervene_after_team(mut self, intervene: bool) -> Self {
        self.intervene_after_team = intervene;
        self
    }

    /// Set whether to intervene on team errors
    pub fn with_intervene_on_team_error(mut self, intervene: bool) -> Self {
        self.intervene_on_team_error = intervene;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate team configuration
        self.team_config.validate()?;
        
        // Validate human configuration
        self.human_config.validate()?;
        
        // Additional validation for team-human combination
        if self.team_config.child_agents.is_empty() {
            return Err("Team-human agent must have at least one child agent".to_string());
        }

        Ok(())
    }
}

/// A hybrid agent that combines team orchestration with human interaction
pub struct TeamHumanAgent {
    /// Configuration for the hybrid agent
    config: TeamHumanAgentConfig,
    /// Team executor for handling team execution
    team_executor: TeamExecutor,
    /// Human interaction manager
    interaction_manager: HumanInteractionManager,
    /// Combined tools from all child agents
    tools: Vec<Arc<dyn Tool>>,
}

impl TeamHumanAgent {
    /// Create a new team-human agent with console interface
    pub fn new(config: TeamHumanAgentConfig) -> Result<Self, AgentError> {
        config.validate().map_err(|e| AgentError::OtherError(e))?;

        let team_executor = TeamExecutor::new(config.team_config.clone())?;
        let interaction_manager = HumanInteractionManager::with_console(config.human_config.clone());

        // Collect all tools from child agents
        let mut tools = Vec::new();
        for child in &config.team_config.child_agents {
            tools.extend(child.agent.get_tools());
        }

        Ok(Self {
            config,
            team_executor,
            interaction_manager,
            tools,
        })
    }

    /// Create a new team-human agent with custom interface
    pub fn with_interface(
        config: TeamHumanAgentConfig,
        interface: Box<dyn HumanInteractionInterface>,
    ) -> Result<Self, AgentError> {
        config.validate().map_err(|e| AgentError::OtherError(e))?;

        let team_executor = TeamExecutor::new(config.team_config.clone())?;
        let interaction_manager = HumanInteractionManager::new(config.human_config.clone(), interface);

        // Collect all tools from child agents
        let mut tools = Vec::new();
        for child in &config.team_config.child_agents {
            tools.extend(child.agent.get_tools());
        }

        Ok(Self {
            config,
            team_executor,
            interaction_manager,
            tools,
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &TeamHumanAgentConfig {
        &self.config
    }

    /// Execute the team-human hybrid logic
    async fn execute_hybrid(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<String, AgentError> {
        let mut current_inputs = inputs.clone();
        let mut final_output = String::new();

        // Extract input for context
        let input_text = inputs
            .get("input")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Phase 1: Pre-team human intervention
        if self.config.intervene_before_team {
            let context = InteractionContext::new(input_text.clone())
                .with_additional("phase", "before_team".to_string())
                .with_additional("team_agents", format!("{}", self.config.team_config.child_agents.len()));

            if self.should_intervene(&context) {
                let interaction_result = self.request_human_input(&context, Some("Pre-team intervention:")).await?;
                
                if interaction_result.terminated {
                    return Ok(interaction_result.response);
                }

                if interaction_result.success {
                    // Update inputs with human response
                    current_inputs.insert("human_pre_team_input".to_string(), json!(interaction_result.response));
                    final_output.push_str(&format!("Pre-team human input: {}\n\n", interaction_result.response));
                }
            }
        }

        // Phase 2: Team execution
        let team_result = match self.team_executor.execute(intermediate_steps, current_inputs.clone()).await {
            Ok(result) => result,
            Err(e) => {
                // Phase 2a: Error intervention
                if self.config.intervene_on_team_error {
                    let context = InteractionContext::new(input_text.clone())
                        .with_error(e.to_string())
                        .with_additional("phase", "team_error".to_string());

                    if self.should_intervene(&context) {
                        let interaction_result = self.request_human_input(&context, Some("Team execution failed. How should we proceed?")).await?;
                        
                        if interaction_result.terminated {
                            return Ok(interaction_result.response);
                        }

                        if interaction_result.success {
                            final_output.push_str(&format!("Team error handled by human: {}\n\n", interaction_result.response));
                            return Ok(final_output + &interaction_result.response);
                        }
                    }
                }
                return Err(e);
            }
        };

        // Add team results to output
        final_output.push_str(&format!("Team execution completed:\n{}\n\n", team_result.final_output));

        // Phase 3: Post-team human intervention
        if self.config.intervene_after_team {
            let context = InteractionContext::new(input_text)
                .with_output(team_result.final_output.clone())
                .with_additional("phase", "after_team".to_string())
                .with_additional("team_success", team_result.success.to_string());

            if self.should_intervene(&context) {
                let interaction_result = self.request_human_input(&context, Some("Post-team intervention:")).await?;
                
                if interaction_result.terminated {
                    return Ok(interaction_result.response);
                }

                if interaction_result.success {
                    final_output.push_str(&format!("Post-team human input: {}", interaction_result.response));
                } else {
                    final_output.push_str("Post-team human intervention failed");
                }
            }
        }

        Ok(final_output)
    }

    /// Check if human intervention should occur
    fn should_intervene(&self, context: &InteractionContext) -> bool {
        let temp_manager = HumanInteractionManager::with_console(self.config.human_config.clone());
        temp_manager.should_intervene(context)
    }

    /// Check if termination should occur
    fn should_terminate(&self, context: &InteractionContext) -> bool {
        let temp_manager = HumanInteractionManager::with_console(self.config.human_config.clone());
        temp_manager.should_terminate(context)
    }

    /// Request human input
    async fn request_human_input(
        &self,
        context: &InteractionContext,
        custom_prompt: Option<&str>,
    ) -> Result<crate::agent::human::HumanInteractionResult, AgentError> {
        let mut temp_manager = HumanInteractionManager::with_console(self.config.human_config.clone());
        temp_manager.request_human_input(context, custom_prompt).await
    }
}

#[async_trait]
impl Agent for TeamHumanAgent {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        let output = self.execute_hybrid(intermediate_steps, inputs).await?;
        
        Ok(AgentEvent::Finish(AgentFinish { output }))
    }

    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}

/// Tool wrapper for team-human agent
pub struct TeamHumanAgentTool {
    /// The team-human agent instance
    agent: Arc<TeamHumanAgent>,
    /// Name for this tool
    name: String,
    /// Description for this tool
    description: String,
}

impl TeamHumanAgentTool {
    /// Create a new team-human agent tool
    pub fn new<S: Into<String>>(
        agent: Arc<TeamHumanAgent>,
        name: S,
        description: S,
    ) -> Self {
        Self {
            agent,
            name: name.into(),
            description: description.into(),
        }
    }

    /// Create a team-human agent tool with default name and description
    pub fn from_agent(agent: Arc<TeamHumanAgent>) -> Self {
        let name = "team_human_agent".to_string();
        let description = "A hybrid agent that combines team orchestration with human interaction capabilities".to_string();

        Self::new(agent, name, description)
    }
}

#[async_trait]
impl Tool for TeamHumanAgentTool {
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

        // Execute the team-human agent
        match self.agent.plan(&[], inputs).await {
            Ok(AgentEvent::Finish(finish)) => Ok(finish.output),
            Ok(AgentEvent::Action(_)) => Err("Team-human agent returned Action instead of Finish".into()),
            Err(e) => Err(e.into()),
        }
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input for the team-human agent"
                }
            },
            "required": ["input"]
        })
    }
}
