use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;

use crate::{
    agent::{Agent, AgentError},
    prompt::PromptArgs,
    schemas::agent::{AgentAction, AgentEvent},
    tools::Tool,
};

use super::{
    CapabilityManager, ActionContext, ReflectionCapability, TaskPlanningCapability,
    CodeExecutionCapability, ReActCapability,
};

/// Trait for agents that support capabilities
#[async_trait]
pub trait CapableAgent: Agent {
    /// Get access to the capability manager
    fn capabilities(&self) -> &CapabilityManager;
    
    /// Get mutable access to the capability manager
    fn capabilities_mut(&mut self) -> &mut CapabilityManager;
    
    /// Convenience method to check for reflection capability
    fn has_reflection(&self) -> bool {
        // Simplified check - look for capability by name
        self.capabilities().list_capabilities().iter()
            .any(|name| name.contains("reflection"))
    }

    /// Convenience method to check for task planning capability
    fn has_task_planning(&self) -> bool {
        // Simplified check - look for capability by name
        self.capabilities().list_capabilities().iter()
            .any(|name| name.contains("task_planning") || name.contains("planning"))
    }

    /// Convenience method to check for code execution capability
    fn has_code_execution(&self) -> bool {
        // Simplified check - look for capability by name
        self.capabilities().list_capabilities().iter()
            .any(|name| name.contains("code_execution") || name.contains("execution"))
    }

    /// Convenience method to check for ReAct capability
    fn has_react(&self) -> bool {
        // Simplified check - look for capability by name
        self.capabilities().list_capabilities().iter()
            .any(|name| name.contains("react"))
    }
    
    /// Get a list of all capability names
    fn list_capabilities(&self) -> Vec<&'static str> {
        self.capabilities().list_capabilities()
    }
}

/// Wrapper that adds capabilities to existing agents
pub struct CapabilityEnhancedAgent<A: Agent> {
    pub(crate) inner_agent: A,
    pub(crate) capabilities: CapabilityManager,
}

impl<A: Agent> CapabilityEnhancedAgent<A> {
    /// Create a new capability-enhanced agent
    pub fn new(agent: A) -> Self {
        Self {
            inner_agent: agent,
            capabilities: CapabilityManager::new(),
        }
    }
    
    /// Add a reflection capability
    pub fn with_reflection<R: ReflectionCapability + 'static>(mut self, capability: R) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Add a task planning capability
    pub fn with_task_planning<T: TaskPlanningCapability + 'static>(mut self, capability: T) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Add a code execution capability
    pub fn with_code_execution<C: CodeExecutionCapability + 'static>(mut self, capability: C) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Add a ReAct capability
    pub fn with_react<R: ReActCapability + 'static>(mut self, capability: R) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Get access to the inner agent
    pub fn inner(&self) -> &A {
        &self.inner_agent
    }
    
    /// Get mutable access to the inner agent
    pub fn inner_mut(&mut self) -> &mut A {
        &mut self.inner_agent
    }
    
    /// Enhanced planning that leverages capabilities
    async fn plan_with_capabilities(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        let mut enhanced_inputs = inputs.clone();
        
        // Apply pre-planning enhancements
        self.capabilities
            .apply_pre_plan_enhancements(intermediate_steps, &mut enhanced_inputs)
            .await?;
        
        // Call the inner agent's plan method
        let mut event = self.inner_agent.plan(intermediate_steps, enhanced_inputs.clone()).await?;
        
        // Apply post-planning enhancements
        self.capabilities
            .apply_post_plan_enhancements(intermediate_steps, &enhanced_inputs, &mut event)
            .await?;
        
        Ok(event)
    }
    
    /// Process action results through capabilities
    pub async fn process_action_result(
        &self,
        action: &AgentAction,
        result: &str,
        intermediate_steps: &[(AgentAction, String)],
        inputs: &PromptArgs,
    ) -> Result<String, AgentError> {
        let context = ActionContext {
            intermediate_steps: intermediate_steps.to_vec(),
            current_inputs: inputs.clone(),
            execution_metadata: json!({
                "agent_type": std::any::type_name::<A>(),
                "capabilities": self.capabilities.list_capabilities(),
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }),
        };
        
        let processed = self.capabilities
            .process_action_results(action, result, &context)
            .await?;
        
        Ok(processed.modified_result.unwrap_or_else(|| result.to_string()))
    }
}

#[async_trait]
impl<A: Agent> Agent for CapabilityEnhancedAgent<A> {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        self.plan_with_capabilities(intermediate_steps, inputs).await
    }
    
    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools = self.inner_agent.get_tools();
        
        // Add tools from capabilities
        tools.extend(self.capabilities.get_all_tools());
        
        tools
    }
}

#[async_trait]
impl<A: Agent> CapableAgent for CapabilityEnhancedAgent<A> {
    fn capabilities(&self) -> &CapabilityManager {
        &self.capabilities
    }
    
    fn capabilities_mut(&mut self) -> &mut CapabilityManager {
        &mut self.capabilities
    }
}

/// Helper trait for converting regular agents to capability-enhanced agents
pub trait IntoCapableAgent<A: Agent> {
    /// Convert this agent into a capability-enhanced agent
    fn into_capable(self) -> CapabilityEnhancedAgent<A>;
}

impl<A: Agent> IntoCapableAgent<A> for A {
    fn into_capable(self) -> CapabilityEnhancedAgent<A> {
        CapabilityEnhancedAgent::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::AgentError,
        prompt::PromptArgs,
        schemas::agent::{AgentEvent, AgentFinish},
        tools::Tool,
    };
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock agent for testing
    struct MockAgent {
        name: String,
    }

    impl MockAgent {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
            }
        }
    }

    #[async_trait]
    impl Agent for MockAgent {
        async fn plan(
            &self,
            _intermediate_steps: &[(AgentAction, String)],
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
    async fn test_capability_enhanced_agent_creation() {
        let agent = MockAgent::new("test");
        let enhanced = CapabilityEnhancedAgent::new(agent);
        
        assert_eq!(enhanced.capabilities().capability_count(), 0);
        assert!(enhanced.capabilities().is_empty());
    }

    #[tokio::test]
    async fn test_into_capable_trait() {
        let agent = MockAgent::new("test");
        let enhanced = agent.into_capable();
        
        assert_eq!(enhanced.inner().name, "test");
        assert_eq!(enhanced.capabilities().capability_count(), 0);
    }

    #[tokio::test]
    async fn test_enhanced_agent_plan() {
        let agent = MockAgent::new("test");
        let enhanced = CapabilityEnhancedAgent::new(agent);
        
        let inputs = std::collections::HashMap::from([
            ("input".to_string(), serde_json::json!("test input"))
        ]);
        
        let result = enhanced.plan(&[], inputs).await.unwrap();
        
        match result {
            AgentEvent::Finish(finish) => {
                assert!(finish.output.contains("test"));
                assert!(finish.output.contains("test input"));
            }
            _ => panic!("Expected AgentFinish"),
        }
    }
}
