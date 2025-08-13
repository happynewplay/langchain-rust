use std::sync::Arc;

use crate::{
    agent::AgentError,
    tools::Tool,
};

use super::{
    agent::HumanAgent,
    config::{HumanAgentConfig, InterventionCondition, TerminationCondition},
    interaction::HumanInteractionInterface,
};

/// Builder for creating human agents
pub struct HumanAgentBuilder {
    config: HumanAgentConfig,
    tools: Vec<Arc<dyn Tool>>,
    interface: Option<Box<dyn HumanInteractionInterface>>,
}

impl HumanAgentBuilder {
    /// Create a new human agent builder
    pub fn new() -> Self {
        Self {
            config: HumanAgentConfig::new(),
            tools: Vec::new(),
            interface: None,
        }
    }

    /// Add an intervention condition
    pub fn add_intervention_condition(mut self, condition: InterventionCondition) -> Self {
        self.config = self.config.add_intervention_condition(condition);
        self
    }

    /// Add multiple intervention conditions
    pub fn intervention_conditions(mut self, conditions: Vec<InterventionCondition>) -> Self {
        for condition in conditions {
            self.config = self.config.add_intervention_condition(condition);
        }
        self
    }

    /// Add a termination condition
    pub fn add_termination_condition(mut self, condition: TerminationCondition) -> Self {
        self.config = self.config.add_termination_condition(condition);
        self
    }

    /// Add multiple termination conditions
    pub fn termination_conditions(mut self, conditions: Vec<TerminationCondition>) -> Self {
        for condition in conditions {
            self.config = self.config.add_termination_condition(condition);
        }
        self
    }

    /// Set maximum interventions
    pub fn max_interventions(mut self, max: u32) -> Self {
        self.config = self.config.with_max_interventions(max);
        self
    }

    /// Set input timeout
    pub fn input_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config = self.config.with_input_timeout(timeout_seconds);
        self
    }

    /// Set default prompt
    pub fn default_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.config = self.config.with_default_prompt(prompt);
        self
    }

    /// Set whether to allow empty responses
    pub fn allow_empty_response(mut self, allow: bool) -> Self {
        self.config = self.config.with_allow_empty_response(allow);
        self
    }

    /// Set system prompt/prefix
    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.config = self.config.with_prefix(prefix);
        self
    }

    /// Add tools to the human agent
    pub fn tools(mut self, tools: &[Arc<dyn Tool>]) -> Self {
        self.tools.extend_from_slice(tools);
        self
    }

    /// Add a single tool
    pub fn tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set custom human interaction interface
    pub fn interface(mut self, interface: Box<dyn HumanInteractionInterface>) -> Self {
        self.interface = Some(interface);
        self
    }

    /// Build the human agent
    pub fn build(self) -> Result<HumanAgent, AgentError> {
        let agent = if let Some(interface) = self.interface {
            HumanAgent::with_interface(self.config, interface)?
        } else {
            HumanAgent::new(self.config)?
        };

        Ok(agent.with_tools(self.tools))
    }

    /// Build the human agent and wrap it as a tool
    pub fn build_as_tool<S: Into<String>>(
        self,
        name: S,
        description: S,
    ) -> Result<super::agent::HumanAgentTool, AgentError> {
        let human_agent = Arc::new(self.build()?);
        Ok(super::agent::HumanAgentTool::new(
            human_agent,
            name,
            description,
        ))
    }

    /// Build the human agent and wrap it as a tool with auto-generated name/description
    pub fn build_as_auto_tool(self) -> Result<super::agent::HumanAgentTool, AgentError> {
        let human_agent = Arc::new(self.build()?);
        Ok(super::agent::HumanAgentTool::from_human_agent(human_agent))
    }
}

impl Default for HumanAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating common human agent patterns
impl HumanAgentBuilder {
    /// Create a human agent that intervenes on errors
    pub fn error_intervention() -> Self {
        Self::new()
            .add_intervention_condition(
                InterventionCondition::new("error", "error")
                    .with_description("Intervene when an error occurs"),
            )
            .add_termination_condition(
                TerminationCondition::new("done", "input")
                    .with_description("Terminate when user says done"),
            )
            .add_termination_condition(
                TerminationCondition::new("exit", "input")
                    .with_description("Terminate when user says exit"),
            )
    }

    /// Create a human agent that intervenes on specific keywords
    pub fn keyword_intervention<S: Into<String>>(keywords: Vec<S>) -> Self {
        let mut builder = Self::new();

        for keyword in keywords {
            let keyword = keyword.into();
            builder = builder.add_intervention_condition(
                InterventionCondition::new(keyword.clone(), "input")
                    .with_description(format!("Intervene when input contains '{}'", keyword)),
            );
        }

        builder
            .add_termination_condition(
                TerminationCondition::new("done", "input")
                    .with_description("Terminate when user says done"),
            )
            .add_termination_condition(
                TerminationCondition::new("exit", "input")
                    .with_description("Terminate when user says exit"),
            )
    }

    /// Create a human agent that intervenes based on regex patterns
    pub fn regex_intervention<S: Into<String>>(patterns: Vec<S>) -> Self {
        let mut builder = Self::new();

        for pattern in patterns {
            let pattern = pattern.into();
            builder = builder.add_intervention_condition(
                InterventionCondition::regex(pattern.clone(), "input")
                    .with_description(format!("Intervene when input matches pattern '{}'", pattern)),
            );
        }

        builder
            .add_termination_condition(
                TerminationCondition::new("done", "input")
                    .with_description("Terminate when user says done"),
            )
            .add_termination_condition(
                TerminationCondition::new("exit", "input")
                    .with_description("Terminate when user says exit"),
            )
    }

    /// Create a human agent that always intervenes (for manual control)
    pub fn always_intervene() -> Self {
        Self::new()
            .add_intervention_condition(
                InterventionCondition::regex(".*", "input")
                    .with_description("Always intervene on any input"),
            )
            .add_termination_condition(
                TerminationCondition::new("done", "input")
                    .with_description("Terminate when user says done"),
            )
            .add_termination_condition(
                TerminationCondition::new("exit", "input")
                    .with_description("Terminate when user says exit"),
            )
    }

    /// Create a human agent that intervenes when output contains certain patterns
    pub fn output_pattern_intervention<S: Into<String>>(patterns: Vec<S>) -> Self {
        let mut builder = Self::new();

        for pattern in patterns {
            let pattern = pattern.into();
            builder = builder.add_intervention_condition(
                InterventionCondition::new(pattern.clone(), "output")
                    .with_description(format!("Intervene when output contains '{}'", pattern)),
            );
        }

        builder
            .add_termination_condition(
                TerminationCondition::new("done", "input")
                    .with_description("Terminate when user says done"),
            )
            .add_termination_condition(
                TerminationCondition::new("exit", "input")
                    .with_description("Terminate when user says exit"),
            )
    }

    /// Create a human agent with similarity-based termination
    pub fn similarity_termination<S: Into<String>>(
        termination_phrases: Vec<(S, f64)>,
    ) -> Self {
        let mut builder = Self::new()
            .add_intervention_condition(
                InterventionCondition::regex(".*", "input")
                    .with_description("Always intervene on any input"),
            );

        for (phrase, threshold) in termination_phrases {
            let phrase = phrase.into();
            builder = builder.add_termination_condition(
                TerminationCondition::similarity(phrase.clone(), "input", threshold)
                    .with_description(format!(
                        "Terminate when input is similar to '{}' (threshold: {})",
                        phrase, threshold
                    )),
            );
        }

        builder
    }
}
