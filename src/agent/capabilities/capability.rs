use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;

use crate::{
    agent::AgentError,
    prompt::PromptArgs,
    schemas::agent::{AgentAction, AgentEvent},
    tools::Tool,
};

/// Core trait that all agent capabilities must implement
pub trait AgentCapability: Send + Sync {
    /// Returns the unique name of this capability
    fn capability_name(&self) -> &'static str;
    
    /// Returns the version of this capability implementation
    fn capability_version(&self) -> &'static str {
        "1.0.0"
    }
    
    /// Returns a description of what this capability provides
    fn capability_description(&self) -> &'static str {
        "No description provided"
    }
    
    /// Returns whether this capability is enabled
    fn is_enabled(&self) -> bool {
        true
    }
}

/// Trait for capabilities that can enhance agent planning
#[async_trait]
pub trait PlanningEnhancer: AgentCapability {
    /// Called before the agent's plan method to potentially modify inputs
    async fn pre_plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: &mut PromptArgs,
    ) -> Result<(), AgentError> {
        let _ = (intermediate_steps, inputs);
        Ok(())
    }
    
    /// Called after the agent's plan method to potentially modify the result
    async fn post_plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: &PromptArgs,
        event: &mut AgentEvent,
    ) -> Result<(), AgentError> {
        let _ = (intermediate_steps, inputs, event);
        Ok(())
    }
}

/// Trait for capabilities that provide additional tools
pub trait ToolProvider: AgentCapability {
    /// Returns tools that this capability provides
    fn get_tools(&self) -> Vec<Arc<dyn Tool>>;
}

/// Trait for capabilities that can process action results
#[async_trait]
pub trait ActionProcessor: AgentCapability {
    /// Called after an action is executed to process the result
    async fn process_action_result(
        &self,
        action: &AgentAction,
        result: &str,
        context: &ActionContext,
    ) -> Result<ProcessedResult, AgentError>;
}

/// Context information for action processing
#[derive(Debug, Clone)]
pub struct ActionContext {
    pub intermediate_steps: Vec<(AgentAction, String)>,
    pub current_inputs: PromptArgs,
    pub execution_metadata: Value,
}

/// Result of action processing
#[derive(Debug, Clone)]
pub struct ProcessedResult {
    pub modified_result: Option<String>,
    pub additional_context: Option<Value>,
    pub should_continue: bool,
}

impl Default for ProcessedResult {
    fn default() -> Self {
        Self {
            modified_result: None,
            additional_context: None,
            should_continue: true,
        }
    }
}

/// Marker trait for capabilities that require initialization
#[async_trait]
pub trait InitializableCapability: AgentCapability {
    /// Initialize the capability with the given configuration
    async fn initialize(&mut self, config: Value) -> Result<(), AgentError>;
    
    /// Check if the capability is properly initialized
    fn is_initialized(&self) -> bool;
}

/// Marker trait for capabilities that need cleanup
#[async_trait]
pub trait CleanupCapability: AgentCapability {
    /// Cleanup resources when the capability is no longer needed
    async fn cleanup(&mut self) -> Result<(), AgentError>;
}

/// Configuration for capability behavior
#[derive(Debug, Clone)]
pub struct CapabilityConfig {
    pub enabled: bool,
    pub priority: i32,
    pub settings: Value,
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: 0,
            settings: Value::Null,
        }
    }
}

/// Trait for capabilities that support configuration
pub trait ConfigurableCapability: AgentCapability {
    /// Get the current configuration
    fn get_config(&self) -> &CapabilityConfig;
    
    /// Update the configuration
    fn set_config(&mut self, config: CapabilityConfig) -> Result<(), AgentError>;
}
