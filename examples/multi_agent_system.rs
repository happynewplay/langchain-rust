use std::{error::Error, sync::Arc};

use async_trait::async_trait;
use langchain_rust::{
    agent::{
        AgentExecutor, ConversationalAgentBuilder, OpenAiToolAgentBuilder,
        TeamAgentBuilder, HumanAgentBuilder, TeamHumanAgentBuilder,
        AgentRegistry, InterventionCondition, TerminationCondition,
        ExecutionPattern, ExecutionStep,
    },
    chain::Chain,
    llm::openai::OpenAI,
    llm::ollama::openai::OllamaConfig,
    schemas::{memory::BaseMemory, Message},
    prompt_args,
    tools::{CommandExecutor, Tool},
};

use serde_json::Value;
use tokio::sync::Mutex;

// Note: Add redis = "0.24" to Cargo.toml dependencies for Redis support
// For this demo, we'll create a mock Redis implementation

// Mock Redis Memory Implementation for Demo
// In production, you would use a real Redis client like redis-rs
#[derive(Clone)]
struct RedisMemory {
    _redis_url: String,
    key_prefix: String,
    // In-memory storage for demo purposes
    messages: Arc<Mutex<Vec<Message>>>,
}

impl RedisMemory {
    pub fn new(redis_url: &str, key_prefix: &str) -> Result<Self, Box<dyn Error>> {
        println!("🔗 Connecting to Redis at: {}", redis_url);
        println!("📝 Using key prefix: {}", key_prefix);

        Ok(Self {
            _redis_url: redis_url.to_string(),
            key_prefix: key_prefix.to_string(),
            messages: Arc::new(Mutex::new(Vec::new())),
        })
    }

    fn messages_key(&self) -> String {
        format!("{}:messages", self.key_prefix)
    }
}

impl BaseMemory for RedisMemory {
    fn messages(&self) -> Vec<Message> {
        // In a real implementation, you would fetch from Redis here
        // For demo, we'll use the in-memory storage
        if let Ok(messages) = self.messages.try_lock() {
            println!("📖 Reading {} messages from Redis key: {}", messages.len(), self.messages_key());
            messages.clone()
        } else {
            vec![]
        }
    }

    fn add_message(&mut self, message: Message) {
        // In a real implementation, you would store to Redis here
        println!("📝 Storing message to Redis key {}: {:?}", self.messages_key(), message);

        // For demo, store in memory
        if let Ok(mut messages) = self.messages.try_lock() {
            messages.push(message);
        }
    }

    fn clear(&mut self) {
        println!("🗑️ Clearing Redis memory at key: {}", self.messages_key());

        // For demo, clear memory
        if let Ok(mut messages) = self.messages.try_lock() {
            messages.clear();
        }
    }
}

// Custom tool for demonstration
struct Calculator {}

#[async_trait]
impl Tool for Calculator {
    fn name(&self) -> String {
        "Calculator".to_string()
    }
    
    fn description(&self) -> String {
        "Useful for mathematical calculations".to_string()
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let input_str = input.as_str().unwrap_or("0");
        // Simple calculation - in real implementation, you'd parse and evaluate
        Ok(format!("Calculated result for: {}", input_str))
    }
}

struct DataAnalyzer {}

#[async_trait]
impl Tool for DataAnalyzer {
    fn name(&self) -> String {
        "DataAnalyzer".to_string()
    }
    
    fn description(&self) -> String {
        "Analyzes data and provides insights".to_string()
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let input_str = input.as_str().unwrap_or("");
        Ok(format!("Data analysis complete for: {}", input_str))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Multi-Agent System Demo");
    println!("==========================\n");

    // Create Ollama LLM with custom configuration
    let ollama_config = OllamaConfig::new()
        .with_api_base("http://192.168.1.38:11434/v1");

    let llm = OpenAI::new(ollama_config)
        .with_model("qwen3:4b-thinking-2507-q8_0");
    
    // Create tools
    let calculator = Arc::new(Calculator {});
    let data_analyzer = Arc::new(DataAnalyzer {});
    let command_executor = Arc::new(CommandExecutor::default());
    
    let tools = vec![
        calculator.clone() as Arc<dyn Tool>,
        data_analyzer.clone() as Arc<dyn Tool>,
        command_executor.clone() as Arc<dyn Tool>,
    ];

    // Demo 1: Basic Team Agent with Sequential Execution and Redis Memory
    println!("📋 Demo 1: Sequential Team Agent with Redis Memory");
    println!("--------------------------------------------------");

    // Create Redis memory for team coordination
    let redis_memory = RedisMemory::new("redis://172.16.0.127:6379", "team_agent")
        .expect("Failed to connect to Redis");
    let team_memory = Arc::new(tokio::sync::Mutex::new(redis_memory));

    // Create individual agents
    let math_agent = Arc::new(
        ConversationalAgentBuilder::new()
            .tools(&[calculator.clone()])
            .prefix("You are a math specialist. Focus on calculations and numerical analysis.")
            .build(llm.clone())?
    ) as Arc<dyn langchain_rust::agent::Agent>;

    let data_agent = Arc::new(
        ConversationalAgentBuilder::new()
            .tools(&[data_analyzer.clone()])
            .prefix("You are a data analyst. Focus on data interpretation and insights.")
            .build(llm.clone())?
    ) as Arc<dyn langchain_rust::agent::Agent>;

    // Create sequential team with memory support
    let sequential_team = TeamAgentBuilder::sequential_team([
        ("math_agent", math_agent.clone()),
        ("data_agent", data_agent.clone()),
    ])
    .prefix("You are coordinating a sequential analysis workflow.")
    .memory(team_memory.clone())
    .coordination_prompts(true)
    .build()?;
    
    let team_executor = AgentExecutor::from_agent(sequential_team);
    
    let result = team_executor.invoke(prompt_args! {
        "input" => "Analyze the sales data: [100, 150, 200, 175, 300] and calculate the average"
    }).await?;
    
    println!("Sequential Team Result:\n{}\n", result);

    // Demo 2: Concurrent Team Agent
    println!("🔄 Demo 2: Concurrent Team Agent");
    println!("--------------------------------");
    
    let concurrent_team = TeamAgentBuilder::concurrent_team([
        ("math_agent", math_agent.clone()),
        ("data_agent", data_agent.clone()),
    ])
    .prefix("You are coordinating concurrent analysis tasks.")
    .build()?;
    
    let concurrent_executor = AgentExecutor::from_agent(concurrent_team);
    
    let result = concurrent_executor.invoke(prompt_args! {
        "input" => "Process this dataset independently: revenue=[1000, 1200, 1100], costs=[800, 900, 850]"
    }).await?;
    
    println!("Concurrent Team Result:\n{}\n", result);

    // Demo 3: Complex Hybrid Execution Pattern
    println!("🔀 Demo 3: Hybrid Execution Pattern");
    println!("-----------------------------------");
    
    let system_agent = Arc::new(
        ConversationalAgentBuilder::new()
            .tools(&[command_executor.clone()])
            .prefix("You are a system administrator. Handle system-related tasks.")
            .build(llm.clone())?
    ) as Arc<dyn langchain_rust::agent::Agent>;

    // Create a complex pattern: system_agent → (math_agent || data_agent) → coordinator
    let coordinator_agent = Arc::new(
        OpenAiToolAgentBuilder::new()
            .tools(&tools)
            .prefix("You are a coordinator. Synthesize results from multiple agents.")
            .build(llm.clone())?
    ) as Arc<dyn langchain_rust::agent::Agent>;
    
    let hybrid_team = TeamAgentBuilder::pipeline_with_concurrent(
        ("system_agent", system_agent.clone()),
        ("math_agent", math_agent.clone()),
        ("data_agent", data_agent.clone()),
        ("coordinator", coordinator_agent.clone()),
    )
    .prefix("You are managing a complex multi-stage workflow.")
    .build()?;
    
    let hybrid_executor = AgentExecutor::from_agent(hybrid_team);
    
    let result = hybrid_executor.invoke(prompt_args! {
        "input" => "Check system status, then analyze performance metrics: CPU=75%, Memory=60%, Disk=45%"
    }).await?;
    
    println!("Hybrid Team Result:\n{}\n", result);

    // Demo 4: Human Agent with Intervention and Redis Memory
    println!("👤 Demo 4: Human Agent with Redis Memory (Simulated)");
    println!("----------------------------------------------------");

    // Create Redis memory for human agent
    let human_redis_memory = RedisMemory::new("redis://172.16.0.127:6379", "human_agent")
        .expect("Failed to connect to Redis");
    let human_memory = Arc::new(tokio::sync::Mutex::new(human_redis_memory));

    // Note: In a real scenario, this would prompt for actual human input
    // For demo purposes, we'll show the configuration
    let _human_agent = HumanAgentBuilder::keyword_intervention(vec!["help", "error", "review"])
        .max_interventions(3)
        .input_timeout(30)
        .memory(human_memory.clone())
        .include_memory_in_prompts(true)
        .prefix("You are a human oversight agent. Intervene when needed.")
        .build()?;

    println!("Human agent configured with:");
    println!("- Keywords: help, error, review");
    println!("- Max interventions: 3");
    println!("- Timeout: 30 seconds");
    println!("- Memory support: enabled");
    println!("- Memory in prompts: enabled\n");

    // Demo 5: Team-Human Hybrid Agent with Shared Redis Memory
    println!("🤝 Demo 5: Team-Human Hybrid Agent with Shared Redis Memory");
    println!("-----------------------------------------------------------");

    // Create shared Redis memory for team-human hybrid
    let hybrid_redis_memory = RedisMemory::new("redis://172.16.0.127:6379", "hybrid_agent")
        .expect("Failed to connect to Redis");
    let hybrid_memory = Arc::new(tokio::sync::Mutex::new(hybrid_redis_memory));

    let _team_human_agent = TeamHumanAgentBuilder::new()
        .add_agent("math_agent", math_agent.clone())
        .add_agent("data_agent", data_agent.clone())
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

    println!("Team-Human agent configured with:");
    println!("- Sequential team execution");
    println!("- Shared memory across team and human components");
    println!("- Coordination prompts enabled");
    println!("- Memory context in human prompts");
    println!("- Pre-team intervention on 'complex' keyword");
    println!("- Error intervention enabled");
    println!("- Termination on 'done' keyword\n");

    // Demo 6: Agent Registry and Universal Tools
    println!("🔧 Demo 6: Agent Registry and Universal Tools");
    println!("---------------------------------------------");
    
    let mut registry = AgentRegistry::new()
        .with_default_timeout(60);
    
    registry.register("math_specialist", math_agent.clone());
    registry.register("data_specialist", data_agent.clone());
    registry.register("system_admin", system_agent.clone());
    
    println!("Registered agents: {:?}", registry.agent_names());
    
    // Convert agents to tools
    let agent_tools = registry.as_tools();
    println!("Created {} agent tools", agent_tools.len());
    
    // Create a meta-agent that can use other agents as tools
    let meta_agent = OpenAiToolAgentBuilder::new()
        .tools(&registry.combined_tools(&tools))
        .prefix("You are a meta-agent that can delegate tasks to specialist agents.")
        .build(llm.clone())?;
    
    let meta_executor = AgentExecutor::from_agent(meta_agent);
    
    let result = meta_executor.invoke(prompt_args! {
        "input" => "I need to analyze some data and perform calculations. Please delegate appropriately."
    }).await?;
    
    println!("Meta-agent delegation result:\n{}\n", result);

    // Demo 7: Nested Team Agents
    println!("🏗️  Demo 7: Nested Team Agents");
    println!("------------------------------");
    
    // Create sub-teams
    let analysis_team = Arc::new(TeamAgentBuilder::concurrent_team([
        ("math_agent", math_agent.clone()),
        ("data_agent", data_agent.clone()),
    ])
    .prefix("Analysis team for mathematical and data tasks.")
    .build()?) as Arc<dyn langchain_rust::agent::Agent>;

    let operations_team = Arc::new(TeamAgentBuilder::new()
        .add_agent("system_admin", system_agent.clone())
        .sequential()
        .prefix("Operations team for system tasks.")
        .build()?) as Arc<dyn langchain_rust::agent::Agent>;
    
    // Create master team with nested teams
    let master_team = TeamAgentBuilder::new()
        .add_team_agent("analysis_team", analysis_team)
        .add_team_agent("operations_team", operations_team)
        .add_agent("coordinator", coordinator_agent.clone())
        .execution_pattern(ExecutionPattern::Hybrid(vec![
            ExecutionStep {
                agent_ids: vec!["analysis_team".to_string(), "operations_team".to_string()],
                concurrent: true,
                dependencies: vec![],
            },
            ExecutionStep {
                agent_ids: vec!["coordinator".to_string()],
                concurrent: false,
                dependencies: vec![0],
            },
        ]))
        .prefix("Master team coordinating nested sub-teams.")
        .build()?;
    
    let master_executor = AgentExecutor::from_agent(master_team);
    
    let result = master_executor.invoke(prompt_args! {
        "input" => "Comprehensive analysis: check system health and analyze performance data [95, 87, 92, 88, 94]"
    }).await?;
    
    println!("Nested team result:\n{}\n", result);

    println!("✅ Multi-Agent System Demo Complete!");
    println!("=====================================");
    println!("This demo showcased:");
    println!("1. Sequential team execution with Redis memory");
    println!("2. Concurrent team execution");
    println!("3. Hybrid execution patterns");
    println!("4. Human agent configuration with Redis memory");
    println!("5. Team-human hybrid agents with shared Redis memory");
    println!("6. Agent registry and universal tools");
    println!("7. Nested team agents");
    println!("\nConfiguration used:");
    println!("🤖 LLM: Ollama at 192.168.1.38:11434");
    println!("📊 Model: qwen3:4b-thinking-2507-q8_0");
    println!("💾 Memory: Redis at 172.16.0.127:6379");
    println!("\nAll agent types can be used as tools and integrated with MCP when the feature is enabled.");

    Ok(())
}
