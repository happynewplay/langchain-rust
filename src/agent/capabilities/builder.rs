use std::marker::PhantomData;
use serde_json::Value;

use crate::agent::{Agent, AgentError};

use super::{
    CapabilityEnhancedAgent, ReflectionCapability, TaskPlanningCapability,
    CodeExecutionCapability, ReActCapability, CapabilityManager,
};

/// Builder for creating capability-enhanced agents
pub struct CapabilityAgentBuilder<A: Agent> {
    agent: Option<A>,
    capabilities: CapabilityManager,
    initialization_config: Option<Value>,
}

impl<A: Agent> CapabilityAgentBuilder<A> {
    /// Create a new builder with the given agent
    pub fn new(agent: A) -> Self {
        Self {
            agent: Some(agent),
            capabilities: CapabilityManager::new(),
            initialization_config: None,
        }
    }
    
    /// Add a reflection capability
    pub fn with_reflection<R: ReflectionCapability + 'static>(mut self, capability: R) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Add a reflection capability with priority
    pub fn with_reflection_priority<R: ReflectionCapability + 'static>(
        mut self, 
        capability: R, 
        priority: i32
    ) -> Self {
        self.capabilities.add_capability_with_priority(capability, priority);
        self
    }
    
    /// Add a task planning capability
    pub fn with_task_planning<T: TaskPlanningCapability + 'static>(mut self, capability: T) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Add a task planning capability with priority
    pub fn with_task_planning_priority<T: TaskPlanningCapability + 'static>(
        mut self, 
        capability: T, 
        priority: i32
    ) -> Self {
        self.capabilities.add_capability_with_priority(capability, priority);
        self
    }
    
    /// Add a code execution capability
    pub fn with_code_execution<C: CodeExecutionCapability + 'static>(mut self, capability: C) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Add a code execution capability with priority
    pub fn with_code_execution_priority<C: CodeExecutionCapability + 'static>(
        mut self, 
        capability: C, 
        priority: i32
    ) -> Self {
        self.capabilities.add_capability_with_priority(capability, priority);
        self
    }
    
    /// Add a ReAct capability
    pub fn with_react<R: ReActCapability + 'static>(mut self, capability: R) -> Self {
        self.capabilities.add_capability(capability);
        self
    }
    
    /// Add a ReAct capability with priority
    pub fn with_react_priority<R: ReActCapability + 'static>(
        mut self, 
        capability: R, 
        priority: i32
    ) -> Self {
        self.capabilities.add_capability_with_priority(capability, priority);
        self
    }
    
    /// Set initialization configuration for capabilities
    pub fn with_initialization_config(mut self, config: Value) -> Self {
        self.initialization_config = Some(config);
        self
    }
    
    /// Build the capability-enhanced agent
    pub async fn build(mut self) -> Result<CapabilityEnhancedAgent<A>, AgentError> {
        let agent = self.agent
            .take()
            .ok_or_else(|| AgentError::OtherError("Agent is required".to_string()))?;
        
        let mut enhanced = CapabilityEnhancedAgent {
            inner_agent: agent,
            capabilities: self.capabilities,
        };
        
        // Initialize capabilities if configuration is provided
        if let Some(config) = self.initialization_config {
            enhanced.capabilities.initialize_capabilities(config).await?;
        }
        
        Ok(enhanced)
    }
    
    /// Build the capability-enhanced agent synchronously (without initialization)
    pub fn build_sync(mut self) -> Result<CapabilityEnhancedAgent<A>, AgentError> {
        let agent = self.agent
            .take()
            .ok_or_else(|| AgentError::OtherError("Agent is required".to_string()))?;
        
        Ok(CapabilityEnhancedAgent {
            inner_agent: agent,
            capabilities: self.capabilities,
        })
    }
}

/// Specialized builder for common capability combinations
pub struct PresetCapabilityBuilder<A: Agent> {
    _phantom: PhantomData<A>,
}

impl<A: Agent> PresetCapabilityBuilder<A> {
    /// Create a research-focused agent with reflection and task planning
    pub fn research_agent(agent: A) -> CapabilityAgentBuilder<A> {
        CapabilityAgentBuilder::new(agent)
            // Note: Actual implementations would be added here
            // .with_reflection(ResearchReflectionCapability::new())
            // .with_task_planning(ResearchTaskPlanner::new())
    }
    
    /// Create a development-focused agent with all capabilities
    pub fn development_agent(agent: A) -> CapabilityAgentBuilder<A> {
        CapabilityAgentBuilder::new(agent)
            // Note: Actual implementations would be added here
            // .with_reflection(CodeReflectionCapability::new())
            // .with_task_planning(SoftwareTaskPlanner::new())
            // .with_code_execution(SandboxedCodeExecutor::new())
            // .with_react(CodeReActCapability::new())
    }
    
    /// Create an analysis-focused agent with reflection and ReAct
    pub fn analysis_agent(agent: A) -> CapabilityAgentBuilder<A> {
        CapabilityAgentBuilder::new(agent)
            // Note: Actual implementations would be added here
            // .with_reflection(AnalysisReflectionCapability::new())
            // .with_react(AnalysisReActCapability::new())
    }
    
    /// Create a planning-focused agent with task planning and reflection
    pub fn planning_agent(agent: A) -> CapabilityAgentBuilder<A> {
        CapabilityAgentBuilder::new(agent)
            // Note: Actual implementations would be added here
            // .with_task_planning(HierarchicalTaskPlanner::new())
            // .with_reflection(PlanningReflectionCapability::new())
    }
}

/// Fluent interface for building agents with capabilities
pub trait CapabilityBuilderExt<A: Agent> {
    /// Start building capabilities for this agent
    fn with_capabilities(self) -> CapabilityAgentBuilder<A>;
    
    /// Create a research-focused version of this agent
    fn as_research_agent(self) -> CapabilityAgentBuilder<A>;
    
    /// Create a development-focused version of this agent
    fn as_development_agent(self) -> CapabilityAgentBuilder<A>;
    
    /// Create an analysis-focused version of this agent
    fn as_analysis_agent(self) -> CapabilityAgentBuilder<A>;
    
    /// Create a planning-focused version of this agent
    fn as_planning_agent(self) -> CapabilityAgentBuilder<A>;
}

impl<A: Agent> CapabilityBuilderExt<A> for A {
    fn with_capabilities(self) -> CapabilityAgentBuilder<A> {
        CapabilityAgentBuilder::new(self)
    }
    
    fn as_research_agent(self) -> CapabilityAgentBuilder<A> {
        PresetCapabilityBuilder::research_agent(self)
    }
    
    fn as_development_agent(self) -> CapabilityAgentBuilder<A> {
        PresetCapabilityBuilder::development_agent(self)
    }
    
    fn as_analysis_agent(self) -> CapabilityAgentBuilder<A> {
        PresetCapabilityBuilder::analysis_agent(self)
    }
    
    fn as_planning_agent(self) -> CapabilityAgentBuilder<A> {
        PresetCapabilityBuilder::planning_agent(self)
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
    async fn test_capability_builder() {
        let agent = MockAgent::new("test");
        let builder = CapabilityAgentBuilder::new(agent);
        
        let enhanced = builder.build_sync().unwrap();
        assert_eq!(enhanced.capabilities().capability_count(), 0);
    }

    #[tokio::test]
    async fn test_fluent_interface() {
        let agent = MockAgent::new("test");
        let builder = agent.with_capabilities();
        
        let enhanced = builder.build_sync().unwrap();
        assert_eq!(enhanced.inner().name, "test");
    }

    #[tokio::test]
    async fn test_preset_builders() {
        let agent = MockAgent::new("test");
        
        // Test research agent preset
        let research_builder = agent.as_research_agent();
        let _research_agent = research_builder.build_sync().unwrap();
        
        // Test development agent preset
        let agent = MockAgent::new("test");
        let dev_builder = agent.as_development_agent();
        let _dev_agent = dev_builder.build_sync().unwrap();
    }
}
