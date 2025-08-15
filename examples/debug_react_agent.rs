use async_trait::async_trait;
use langchain_rust::{
    agent::{AgentExecutor, ReActAgentBuilder},
    chain::Chain,
    llm::openai::{OpenAI, OpenAIConfig},
    memory::SimpleMemory,
    prompt_args,
    tools::Tool,
};
use serde_json::{json, Value};
use std::error::Error;
use std::sync::Arc;

/// Simple calculator tool for testing
struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> String {
        "calculator".to_string()
    }

    fn description(&self) -> String {
        "Perform basic arithmetic calculations. Input should be a JSON object with 'expression' field.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Mathematical expression to evaluate (e.g., '2 + 2')"
                }
            },
            "required": ["expression"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let expression = input["expression"].as_str().unwrap_or("0");
        println!("🔧 [TOOL] calculator(expression: {})", expression);
        
        // Simple calculation simulation
        let result = match expression {
            "2 + 2" => "4",
            "10 * 5" => "50",
            "100 / 4" => "25",
            _ => "42", // Default answer
        };
        
        println!("📊 [RESULT] {}", result);
        Ok(result.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the LLM with Ollama configuration
    let config = OpenAIConfig::default()
        .with_api_base("http://192.168.1.38:11434/v1".to_string())
        .with_api_key("ollama".to_string());

    let llm = OpenAI::new(config).with_model("qwen3:4b-thinking-2507-q8_0".to_string());

    // Create simple tool
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(CalculatorTool),
    ];

    println!("🚀 Debug ReAct Agent");
    println!("🧠 Testing basic ReAct functionality\n");

    // Create ReAct agent
    let react_agent = ReActAgentBuilder::new()
        .tools(&tools)
        .prefix("You are a helpful assistant that can perform calculations. You MUST only use the available tools. Use the ReAct format: Thought, Action, Action Input, then wait for Observation.")
        .build(llm)?;

    println!("✅ Created ReAct agent with {} tools", tools.len());

    // Create executor with memory
    let memory = SimpleMemory::new();
    let executor = AgentExecutor::from_agent(react_agent)
        .with_memory(memory.into())
        .with_max_iterations(3);

    println!("🔍 Starting simple test...\n");

    // Simple test case
    let test_input = "What is 2 + 2?";

    println!("📝 Input: {}\n", test_input);
    println!("{}", "=".repeat(50));

    match executor.invoke(prompt_args! {
        "input" => test_input
    }).await {
        Ok(result) => {
            println!("\n{}", "=".repeat(50));
            println!("🎯 Final Result:");
            println!("{}", result);
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}
