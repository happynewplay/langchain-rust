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

/// Simple search tool for testing
struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> String {
        "search".to_string()
    }

    fn description(&self) -> String {
        "Search for information on the internet. Input should be a JSON object with 'query' field.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let query = input["query"].as_str().unwrap_or("unknown");
        println!("🔍 [SEARCH] Searching for: {}", query);
        
        // Simulate search results
        let result = match query.to_lowercase().as_str() {
            q if q.contains("weather") => "Today's weather is sunny with 25°C temperature.",
            q if q.contains("capital") && q.contains("france") => "The capital of France is Paris.",
            q if q.contains("2+2") || q.contains("2 + 2") => "2 + 2 equals 4.",
            _ => "Search completed. No specific results found.",
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
        Arc::new(SearchTool),
    ];

    println!("🚀 Simple ReAct Test");
    println!("🧠 Testing ReAct format compliance\n");

    // Create ReAct agent with very explicit instructions
    let react_agent = ReActAgentBuilder::new()
        .tools(&tools)
        .prefix(r#"You are a ReAct agent. You MUST use this exact format:

Thought: [your reasoning]
Action: search
Action Input: {"query": "your search query"}

Do NOT provide direct answers. ALWAYS use the search tool first.

Example:
Thought: I need to search for information about the weather.
Action: search
Action Input: {"query": "weather today"}

Now follow this format exactly:"#)
        .build(llm)?;

    println!("✅ Created ReAct agent with {} tools", tools.len());

    // Create executor with memory
    let memory = SimpleMemory::new();
    let executor = AgentExecutor::from_agent(react_agent)
        .with_memory(memory.into())
        .with_max_iterations(3);

    println!("🔍 Starting ReAct test...\n");

    // Simple test case
    let test_input = "What is the capital of France?";

    println!("📝 Input: {}\n", test_input);
    println!("{}", "=".repeat(50));

    match executor.invoke(prompt_args! {
        "input" => test_input
    }).await {
        Ok(result) => {
            println!("\n{}", "=".repeat(50));
            println!("🎯 Final Result:");
            println!("{}", result);
            
            println!("\n✅ ReAct Test Analysis:");
            println!("🔄 The agent should have:");
            println!("   1. Started with 'Thought:'");
            println!("   2. Used 'Action: search'");
            println!("   3. Provided 'Action Input:' with JSON");
            println!("   4. Received observation from tool");
            println!("   5. Provided final answer");
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}
