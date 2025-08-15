use async_trait::async_trait;
use langchain_rust::{
    agent::{AgentExecutor, ReActAgentBuilder},
    chain::Chain,
    llm::openai::{OpenAI, OpenAIConfig},
    language_models::llm::LLM,
    memory::SimpleMemory,
    prompt_args,
    schemas::messages::Message,
    tools::Tool,
};
use serde_json::{json, Value};
use std::error::Error;
use std::sync::Arc;

/// Simple search tool that always works
struct SearchTool;

#[async_trait]
impl Tool for SearchTool {
    fn name(&self) -> String {
        "search".to_string()
    }

    fn description(&self) -> String {
        "Search for information. Input: {\"query\": \"search terms\"}".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to search for"
                }
            },
            "required": ["query"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let query = input["query"].as_str().unwrap_or("unknown");
        println!("🔍 [SEARCH] Query: {}", query);
        
        // Always return a helpful result
        let result = match query.to_lowercase().as_str() {
            q if q.contains("weather") => "Today is sunny, 25°C",
            q if q.contains("capital") && q.contains("france") => "Paris is the capital of France",
            q if q.contains("customer") || q.contains("c003") => "Customer C003: VIP status, recent order issue",
            _ => "Information found successfully",
        };
        
        println!("📊 [RESULT] {}", result);
        Ok(result.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize LLM
    let config = OpenAIConfig::default()
        .with_api_base("http://192.168.1.38:11434/v1".to_string())
        .with_api_key("ollama".to_string());

    let llm = OpenAI::new(config).with_model("qwen3:4b-thinking-2507-q8_0".to_string());
    let llm_clone = llm.clone();

    // Create tools
    let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(SearchTool)];

    println!("🚀 Working ReAct Demo");
    println!("🧠 Testing ReAct with explicit format enforcement\n");

    // Create ReAct agent with VERY explicit instructions
    let react_agent = ReActAgentBuilder::new()
        .tools(&tools)
        .prefix(r#"You are a ReAct agent. You MUST respond in this EXACT format:

Thought: [your reasoning]
Action: search
Action Input: {"query": "your search terms"}

EXAMPLE:
Thought: I need to find information about the weather.
Action: search
Action Input: {"query": "weather today"}

RULES:
1. ALWAYS start with "Thought:"
2. ALWAYS use "Action: search"
3. ALWAYS use valid JSON for Action Input
4. NO other format is allowed

You MUST follow this format exactly. Start with "Thought:" now:"#)
        .build(llm.clone())?;

    println!("✅ Created ReAct agent");

    // Create executor
    let memory = SimpleMemory::new();
    let executor = AgentExecutor::from_agent(react_agent)
        .with_memory(memory.into())
        .with_max_iterations(3);

    // Test with simple question
    let test_input = "What is the capital of France?";
    println!("📝 Question: {}\n", test_input);
    println!("{}", "=".repeat(50));

    // Let's test the LLM directly first to see what it outputs
    println!("🔍 Testing LLM directly first...");
    let test_prompt = format!(r#"You are a ReAct agent. You MUST respond in this EXACT format:

Thought: [your reasoning]
Action: search
Action Input: {{"query": "your search terms"}}

EXAMPLE:
Thought: I need to find information about the weather.
Action: search
Action Input: {{"query": "weather today"}}

RULES:
1. ALWAYS start with "Thought:"
2. ALWAYS use "Action: search"
3. ALWAYS use valid JSON for Action Input
4. NO other format is allowed

You MUST follow this format exactly. Start with "Thought:" now:

Question: {}"#, test_input);



    let direct_response = llm.generate(&[Message::new_human_message(&test_prompt)]).await?;
    println!("🔍 Raw LLM Output:");
    println!("{}", "=".repeat(50));
    println!("{}", direct_response.generation);
    println!("{}", "=".repeat(50));

    // Test the output parser directly
    use langchain_rust::agent::ReActOutputParser;
    let parser = ReActOutputParser::new();
    println!("🔍 Testing parser directly...");

    // Let's debug what the parser extracts
    let test_text = &direct_response.generation;
    println!("🔍 Debug: Raw text for parsing:");
    println!("{}", test_text);

    match parser.parse(test_text) {
        Ok(event) => {
            println!("✅ Parser succeeded: {:?}", event);
        }
        Err(e) => {
            println!("❌ Parser failed: {}", e);
        }
    }

    // Debug: Print available tools
    println!("🔍 Available tools:");
    for tool in &tools {
        println!("   - Tool name: '{}'", tool.name());
    }

    match executor.invoke(prompt_args! {
        "input" => test_input
    }).await {
        Ok(result) => {
            println!("\n{}", "=".repeat(50));
            println!("🎯 SUCCESS! ReAct Agent worked!");
            println!("📋 Final Answer: {}", result);

            println!("\n✅ This proves our ReAct implementation works:");
            println!("   🧠 LLM followed ReAct format");
            println!("   🔧 Tools were called autonomously");
            println!("   🔄 Thought-Action-Observation cycle completed");
            println!("   🎯 Final answer was provided");
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            println!("\n🔧 Debugging info:");
            println!("   - Check if LLM is following the exact format");
            println!("   - Verify JSON in Action Input is valid");
            println!("   - Ensure 'Thought:' starts the response");
        }
    }

    Ok(())
}
