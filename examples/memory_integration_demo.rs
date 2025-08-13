use std::sync::Arc;

use async_trait::async_trait;
use langchain_rust::{
    agent::{
        AgentExecutor, ConversationalAgentBuilder, TeamAgentBuilder, HumanAgentBuilder,
        TeamHumanAgentBuilder,
        human::{InterventionCondition, TerminationCondition},
    },
    llm::openai::{OpenAI, OpenAIModel},
    memory::SimpleMemory,
    prompt_args,
    tools::Tool,
};
use serde_json::Value;
use tokio::sync::Mutex;

// Mock tool for demonstration
struct MockCalculator;

#[async_trait]
impl Tool for MockCalculator {
    fn name(&self) -> String {
        "Calculator".to_string()
    }
    
    fn description(&self) -> String {
        "Performs mathematical calculations".to_string()
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn std::error::Error>> {
        let input_str = input.as_str().unwrap_or("0");
        Ok(format!("Calculation result for: {}", input_str))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Memory Integration Demo");
    println!("=========================\n");

    // Create LLM (in real usage, you'd configure with API keys)
    let llm = OpenAI::default().with_model(OpenAIModel::Gpt4.to_string());
    
    // Create tools
    let calculator = Arc::new(MockCalculator);
    
    // Demo 1: Team Agent with Memory
    println!("📋 Demo 1: Team Agent with Shared Memory");
    println!("----------------------------------------");
    
    // Create shared memory for team coordination
    let team_memory = Arc::new(Mutex::new(SimpleMemory::new()));
    
    // Create individual agents
    let math_agent = Arc::new(
        ConversationalAgentBuilder::new()
            .tools(&[calculator.clone()])
            .prefix("You are a math specialist.")
            .build(llm.clone())?
    );
    
    let analysis_agent = Arc::new(
        ConversationalAgentBuilder::new()
            .tools(&[calculator.clone()])
            .prefix("You are an analysis specialist.")
            .build(llm.clone())?
    );
    
    // Create team with memory support
    let team_agent = TeamAgentBuilder::sequential_team([
        ("math_agent", math_agent.clone()),
        ("analysis_agent", analysis_agent.clone()),
    ])
    .prefix("You are coordinating mathematical analysis.")
    .memory(team_memory.clone())
    .coordination_prompts(true)
    .build()?;
    
    println!("✅ Team agent created with:");
    println!("   - Shared memory across child agents");
    println!("   - Coordination prompts enabled");
    println!("   - Sequential execution pattern\n");

    // Demo 2: Human Agent with Memory
    println!("👤 Demo 2: Human Agent with Memory");
    println!("----------------------------------");
    
    // Create memory for human agent
    let human_memory = Arc::new(Mutex::new(SimpleMemory::new()));
    
    let human_agent = HumanAgentBuilder::keyword_intervention(vec!["help", "review"])
        .max_interventions(3)
        .input_timeout(30)
        .memory(human_memory.clone())
        .include_memory_in_prompts(true)
        .prefix("You are a human oversight agent.")
        .build()?;
    
    println!("✅ Human agent created with:");
    println!("   - Memory for conversation history");
    println!("   - Memory context in intervention prompts");
    println!("   - Keyword-based intervention triggers\n");

    // Demo 3: Team-Human Hybrid with Shared Memory
    println!("🤝 Demo 3: Team-Human Hybrid with Shared Memory");
    println!("-----------------------------------------------");
    
    // Create shared memory for hybrid agent
    let hybrid_memory = Arc::new(Mutex::new(SimpleMemory::new()));
    
    let hybrid_agent = TeamHumanAgentBuilder::new()
        .add_agent("math_agent", math_agent.clone())
        .add_agent("analysis_agent", analysis_agent.clone())
        .sequential()
        .memory(hybrid_memory.clone())
        .coordination_prompts(true)
        .include_memory_in_prompts(true)
        .add_intervention_condition(
            InterventionCondition::new("complex", "input")
                .with_description("Intervene on complex tasks")
        )
        .add_termination_condition(
            TerminationCondition::new("done", "input")
                .with_description("Terminate when user says done")
        )
        .intervene_before_team(true)
        .intervene_after_team(false)
        .intervene_on_team_error(true)
        .build()?;
    
    println!("✅ Team-Human hybrid created with:");
    println!("   - Shared memory across team and human components");
    println!("   - Coordination prompts for team context");
    println!("   - Memory context in human intervention prompts");
    println!("   - Complex intervention and termination logic\n");

    // Demo 4: Memory Persistence Across Executions
    println!("💾 Demo 4: Memory Persistence");
    println!("-----------------------------");
    
    // Create executor with memory
    let executor = AgentExecutor::from_agent(team_agent)
        .with_memory(team_memory.clone());
    
    println!("✅ Agent executor created with persistent memory");
    println!("   - Memory will persist across multiple invocations");
    println!("   - Chat history will be maintained");
    println!("   - Context will accumulate over time\n");

    // Demo 5: Memory Configuration Options
    println!("⚙️  Demo 5: Memory Configuration Options");
    println!("---------------------------------------");
    
    println!("Available memory configuration options:");
    println!("📋 Team Agents:");
    println!("   - .memory(memory): Set shared memory for coordination");
    println!("   - .coordination_prompts(true): Include team context in prompts");
    println!();
    println!("👤 Human Agents:");
    println!("   - .memory(memory): Set memory for conversation history");
    println!("   - .include_memory_in_prompts(true): Include history in intervention prompts");
    println!();
    println!("🤝 Team-Human Hybrids:");
    println!("   - .memory(memory): Set shared memory for both components");
    println!("   - .coordination_prompts(true): Enable team coordination context");
    println!("   - .include_memory_in_prompts(true): Include memory in human prompts");
    println!();
    println!("🔧 AgentExecutor:");
    println!("   - .with_memory(memory): Set memory for the executor");
    println!("   - Memory persists across multiple .invoke() calls");

    println!("\n✅ Memory Integration Demo Complete!");
    println!("====================================");
    println!("Key Benefits:");
    println!("1. Shared context across agent teams");
    println!("2. Persistent conversation history");
    println!("3. Enhanced coordination through memory");
    println!("4. Human intervention with full context");
    println!("5. Flexible memory configuration options");

    Ok(())
}
