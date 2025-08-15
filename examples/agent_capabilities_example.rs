use langchain_rust::{
    agent::{
        capabilities::{
            CapabilityAgentBuilder, CapableAgent, DefaultReflectionCapability,
            DefaultTaskPlanningCapability, DefaultCodeExecutionCapability, DefaultReActCapability,
            CapabilityBuilderExt,
        },
        chat::ChatAgentBuilder,
        AgentExecutor,
    },
    llm::openai::OpenAI,
    tools::{Tool, CommandExecutor},
    prompt::PromptArgs,
};
use std::sync::Arc;
use std::error::Error;
use async_trait::async_trait;
use serde_json::{json, Value};

/// Example tool for demonstration
struct ExampleTool;

#[async_trait]
impl Tool for ExampleTool {
    fn name(&self) -> String {
        "example_tool".to_string()
    }
    
    fn description(&self) -> String {
        "An example tool for demonstration purposes".to_string()
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input for the example tool"
                }
            },
            "required": ["input"]
        })
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let input_str = input["input"].as_str().unwrap_or("default");
        Ok(format!("Example tool processed: {}", input_str))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the LLM (you'll need to set your OpenAI API key)
    let llm = OpenAI::default();
    
    // Create some tools
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ExampleTool),
        Arc::new(CommandExecutor::new()),
    ];
    
    println!("🚀 Agent Capabilities System Example\n");
    
    // Example 1: Basic agent with reflection capability
    println!("📝 Example 1: Agent with Reflection Capability");
    let base_agent = ChatAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;
    
    let reflection_agent = CapabilityAgentBuilder::new(base_agent)
        .with_reflection(DefaultReflectionCapability::new())
        .build_sync()?;
    
    println!("✅ Created agent with capabilities: {:?}", reflection_agent.list_capabilities());
    println!("   - Has reflection: {}", reflection_agent.has_reflection());
    println!("   - Has task planning: {}", reflection_agent.has_task_planning());
    println!("   - Has code execution: {}", reflection_agent.has_code_execution());
    println!("   - Has ReAct: {}", reflection_agent.has_react());
    println!();
    
    // Example 2: Agent with multiple capabilities using fluent interface
    println!("🔧 Example 2: Multi-Capability Agent (Fluent Interface)");
    let base_agent2 = ChatAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;
    
    let multi_capability_agent = base_agent2
        .with_capabilities()
        .with_reflection(DefaultReflectionCapability::new())
        .with_task_planning(DefaultTaskPlanningCapability::new())
        .with_code_execution(DefaultCodeExecutionCapability::new())
        .with_react(DefaultReActCapability::new())
        .build_sync()?;
    
    println!("✅ Created multi-capability agent with: {:?}", multi_capability_agent.list_capabilities());
    println!("   - Total capabilities: {}", multi_capability_agent.capabilities().capability_count());
    println!();
    
    // Example 3: Using preset capability combinations
    println!("🎯 Example 3: Preset Capability Combinations");
    
    // Research-focused agent
    let base_agent3 = ChatAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;
    
    let research_agent = base_agent3.as_research_agent().build_sync()?;
    println!("🔍 Research agent capabilities: {:?}", research_agent.list_capabilities());
    
    // Development-focused agent
    let base_agent4 = ChatAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;
    
    let dev_agent = base_agent4.as_development_agent().build_sync()?;
    println!("💻 Development agent capabilities: {:?}", dev_agent.list_capabilities());
    
    // Analysis-focused agent
    let base_agent5 = ChatAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;
    
    let analysis_agent = base_agent5.as_analysis_agent().build_sync()?;
    println!("📊 Analysis agent capabilities: {:?}", analysis_agent.list_capabilities());
    println!();
    
    // Example 4: Using capabilities with AgentExecutor
    println!("⚡ Example 4: Running Agent with Capabilities");
    
    let executor = AgentExecutor::from_agent(multi_capability_agent);
    
    // Create a sample input
    let inputs = std::collections::HashMap::from([
        ("input".to_string(), json!("Analyze the current state and plan next steps"))
    ]);
    
    // Note: In a real application, you would run the executor
    // let result = executor.invoke(inputs).await?;
    // println!("Agent result: {}", result);
    
    println!("✅ Agent executor created successfully with capability-enhanced agent");
    println!("   (Actual execution would require valid LLM configuration)");
    println!();
    
    // Example 5: Demonstrating capability-specific functionality
    println!("🧠 Example 5: Capability-Specific Features");
    
    // Create an agent with reflection capability
    let base_agent6 = ChatAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;
    
    let reflection_agent = CapabilityAgentBuilder::new(base_agent6)
        .with_reflection(DefaultReflectionCapability::new())
        .build_sync()?;
    
    // Access the reflection capability
    if let Some(reflection_cap) = reflection_agent.capabilities().get_capability::<DefaultReflectionCapability>() {
        println!("📈 Reflection capability found:");
        println!("   - Capability name: {}", reflection_cap.capability_name());
        println!("   - Capability description: {}", reflection_cap.capability_description());
        
        // Get performance metrics (would be populated in real usage)
        if let Ok(metrics) = reflection_cap.get_performance_metrics().await {
            println!("   - Total experiences: {}", metrics.total_experiences);
            println!("   - Success rate: {:.1}%", metrics.successful_experiences as f64 / metrics.total_experiences.max(1) as f64 * 100.0);
        }
    }
    println!();
    
    // Example 6: Capability priorities and configuration
    println!("⚙️ Example 6: Advanced Configuration");
    
    let base_agent7 = ChatAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;
    
    let configured_agent = CapabilityAgentBuilder::new(base_agent7)
        .with_reflection_priority(DefaultReflectionCapability::new(), 10) // High priority
        .with_task_planning_priority(DefaultTaskPlanningCapability::new(), 5) // Medium priority
        .with_code_execution_priority(DefaultCodeExecutionCapability::new(), 8) // High priority
        .with_react_priority(DefaultReActCapability::new(), 3) // Lower priority
        .build_sync()?;
    
    println!("✅ Created agent with prioritized capabilities");
    println!("   - Capabilities will be executed in priority order during planning");
    println!("   - Higher priority capabilities can override lower priority ones");
    println!();
    
    println!("🎉 All examples completed successfully!");
    println!("\n📚 Key Features Demonstrated:");
    println!("   ✓ Basic capability addition");
    println!("   ✓ Fluent interface for capability building");
    println!("   ✓ Preset capability combinations");
    println!("   ✓ Integration with AgentExecutor");
    println!("   ✓ Capability-specific functionality access");
    println!("   ✓ Priority-based capability configuration");
    println!("\n💡 Next Steps:");
    println!("   - Implement custom capabilities by extending the base traits");
    println!("   - Create domain-specific capability combinations");
    println!("   - Integrate with real LLM providers for full functionality");
    println!("   - Add persistence for capability learning and reflection");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use langchain_rust::agent::capabilities::*;
    
    #[tokio::test]
    async fn test_capability_system_basic() {
        let llm = OpenAI::default();
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(ExampleTool)];
        
        let base_agent = ChatAgentBuilder::new()
            .tools(&tools)
            .build(llm)
            .unwrap();
        
        let enhanced_agent = CapabilityAgentBuilder::new(base_agent)
            .with_reflection(DefaultReflectionCapability::new())
            .build_sync()
            .unwrap();
        
        assert!(enhanced_agent.has_reflection());
        assert!(!enhanced_agent.has_task_planning());
        assert_eq!(enhanced_agent.capabilities().capability_count(), 1);
    }
    
    #[tokio::test]
    async fn test_fluent_interface() {
        let llm = OpenAI::default();
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(ExampleTool)];
        
        let base_agent = ChatAgentBuilder::new()
            .tools(&tools)
            .build(llm)
            .unwrap();
        
        let enhanced_agent = base_agent
            .with_capabilities()
            .with_reflection(DefaultReflectionCapability::new())
            .with_task_planning(DefaultTaskPlanningCapability::new())
            .build_sync()
            .unwrap();
        
        assert!(enhanced_agent.has_reflection());
        assert!(enhanced_agent.has_task_planning());
        assert_eq!(enhanced_agent.capabilities().capability_count(), 2);
    }
    
    #[tokio::test]
    async fn test_preset_combinations() {
        let llm = OpenAI::default();
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(ExampleTool)];
        
        let base_agent = ChatAgentBuilder::new()
            .tools(&tools)
            .build(llm)
            .unwrap();
        
        let research_agent = base_agent.as_research_agent().build_sync().unwrap();
        
        // Research agents should have reflection and task planning
        assert!(research_agent.has_reflection() || research_agent.has_task_planning());
    }
}
