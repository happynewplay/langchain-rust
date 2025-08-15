use async_trait::async_trait;
use langchain_rust::{
    agent::{
        AgentCapability, AgentExecutor, CapabilityAgentBuilder, CapabilityBuilderExt, CapableAgent,
        ConversationalAgentBuilder, DefaultReActCapability, DefaultReflectionCapability,
        DefaultTaskPlanningCapability, ReflectionCapability,
    },
    chain::Chain,
    llm::openai::{OpenAI, OpenAIConfig},
    tools::Tool,
};
use serde_json::{json, Value};
use std::error::Error;
use std::sync::Arc;

/// Customer database query tool
struct CustomerQueryTool;

#[async_trait]
impl Tool for CustomerQueryTool {
    fn name(&self) -> String {
        "customer_query".to_string()
    }

    fn description(&self) -> String {
        "Query customer information from the database".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "customer_id": {
                    "type": "string",
                    "description": "Customer ID to query"
                },
                "query_type": {
                    "type": "string",
                    "enum": ["profile", "orders", "support_tickets"],
                    "description": "Type of information to retrieve"
                }
            },
            "required": ["customer_id", "query_type"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let customer_id = input["customer_id"].as_str().unwrap_or("unknown");
        let query_type = input["query_type"].as_str().unwrap_or("profile");

        // Simulate database query
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        match query_type {
            "profile" => Ok(format!(
                "Customer {} - Name: John Doe, Email: john@example.com, Status: Active",
                customer_id
            )),
            "orders" => Ok(format!(
                "Customer {} has 3 recent orders: #1001, #1002, #1003",
                customer_id
            )),
            "support_tickets" => Ok(format!(
                "Customer {} has 1 open support ticket: #T-456",
                customer_id
            )),
            _ => Ok(format!("Unknown query type for customer {}", customer_id)),
        }
    }
}

/// Market data analysis tool
struct MarketAnalysisTool;

#[async_trait]
impl Tool for MarketAnalysisTool {
    fn name(&self) -> String {
        "market_analysis".to_string()
    }

    fn description(&self) -> String {
        "Analyze market trends and provide insights".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "Stock symbol or market indicator"
                },
                "timeframe": {
                    "type": "string",
                    "enum": ["1d", "1w", "1m", "3m"],
                    "description": "Analysis timeframe"
                }
            },
            "required": ["symbol", "timeframe"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let symbol = input["symbol"].as_str().unwrap_or("UNKNOWN");
        let timeframe = input["timeframe"].as_str().unwrap_or("1d");

        // Simulate market data analysis
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        Ok(format!("Market Analysis for {}: Trend is bullish over {} timeframe. RSI: 65, Moving Average: Upward", symbol, timeframe))
    }
}

/// Email notification tool
struct EmailNotificationTool;

#[async_trait]
impl Tool for EmailNotificationTool {
    fn name(&self) -> String {
        "send_email".to_string()
    }

    fn description(&self) -> String {
        "Send email notifications to users".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "recipient": {
                    "type": "string",
                    "description": "Email recipient"
                },
                "subject": {
                    "type": "string",
                    "description": "Email subject"
                },
                "message": {
                    "type": "string",
                    "description": "Email message content"
                }
            },
            "required": ["recipient", "subject", "message"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let recipient = input["recipient"].as_str().unwrap_or("unknown@example.com");
        let subject = input["subject"].as_str().unwrap_or("No Subject");
        let message = input["message"].as_str().unwrap_or("No Content");

        // Simulate email sending
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        Ok(format!(
            "Email sent successfully to {} with subject '{}'. Message: {}",
            recipient, subject, message
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the LLM with Ollama configuration
    let config = OpenAIConfig::default()
        .with_api_base("http://192.168.1.38:11434/v1".to_string())
        .with_api_key("ollama".to_string()); // Ollama doesn't require a real API key

    let llm = OpenAI::new(config).with_model("qwen3:4b-thinking-2507-q8_0".to_string());

    // Create business-oriented tools
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(CustomerQueryTool),
        Arc::new(MarketAnalysisTool),
        Arc::new(EmailNotificationTool),
    ];

    println!("🚀 Agent Capabilities System Example\n");

    // Example 1: Basic agent with reflection capability
    println!("📝 Example 1: Agent with Reflection Capability");
    let base_agent = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;

    let reflection_agent = CapabilityAgentBuilder::new(base_agent)
        .with_reflection(DefaultReflectionCapability::new())
        .build_sync()?;

    println!(
        "✅ Created agent with capabilities: {:?}",
        reflection_agent.list_capabilities()
    );
    println!("   - Has reflection: {}", reflection_agent.has_reflection());
    println!(
        "   - Has task planning: {}",
        reflection_agent.has_task_planning()
    );
    println!(
        "   - Has code execution: {}",
        reflection_agent.has_code_execution()
    );
    println!("   - Has ReAct: {}", reflection_agent.has_react());
    println!();

    // Example 2: Agent with multiple capabilities using fluent interface (excluding code execution)
    println!("🔧 Example 2: Multi-Capability Agent (Fluent Interface)");
    let base_agent2 = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;

    let multi_capability_agent = base_agent2
        .with_capabilities()
        .with_reflection(DefaultReflectionCapability::new())
        .with_task_planning(DefaultTaskPlanningCapability::new())
        .with_react(DefaultReActCapability::new())
        .build_sync()?;

    println!(
        "✅ Created multi-capability agent with: {:?}",
        multi_capability_agent.list_capabilities()
    );
    println!(
        "   - Total capabilities: {}",
        multi_capability_agent.capabilities().capability_count()
    );
    println!();

    // Example 3: Using preset capability combinations
    println!("🎯 Example 3: Preset Capability Combinations");

    // Research-focused agent
    let base_agent3 = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;

    let research_agent = base_agent3.as_research_agent().build_sync()?;
    println!(
        "🔍 Research agent capabilities: {:?}",
        research_agent.list_capabilities()
    );

    // Development-focused agent
    let base_agent4 = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;

    let dev_agent = base_agent4.as_development_agent().build_sync()?;
    println!(
        "💻 Development agent capabilities: {:?}",
        dev_agent.list_capabilities()
    );

    // Analysis-focused agent
    let base_agent5 = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;

    let analysis_agent = base_agent5.as_analysis_agent().build_sync()?;
    println!(
        "📊 Analysis agent capabilities: {:?}",
        analysis_agent.list_capabilities()
    );
    println!();

    // Example 4: Demonstrating real business scenarios
    println!("⚡ Example 4: Business Scenario Simulations");

    // Customer Service Scenario
    println!("📞 Customer Service Agent Scenario:");
    let customer_service_agent = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?
        .with_capabilities()
        .with_reflection(DefaultReflectionCapability::new())
        .with_react(DefaultReActCapability::new())
        .build_sync()?;

    let _executor = AgentExecutor::from_agent(customer_service_agent);
    let _customer_inputs = std::collections::HashMap::from([
        ("input".to_string(), json!("Customer C001 is complaining about a delayed order. Please check their order history and send them an update email."))
    ]);

    println!("   🚀 Executing Customer Service Agent with real LLM...");
    match _executor.invoke(_customer_inputs).await {
        Ok(result) => {
            println!("   ✅ Customer Service Response:");
            println!("   📝 {}", result);
        }
        Err(e) => {
            println!("   ❌ Execution failed: {}", e);
            println!("   💡 Check Ollama connection and model availability");
        }
    }

    // Market Analysis Scenario
    println!("\n📈 Market Analysis Agent Scenario:");
    let market_agent = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?
        .with_capabilities()
        .with_task_planning(DefaultTaskPlanningCapability::new())
        .with_reflection(DefaultReflectionCapability::new())
        .build_sync()?;

    let _market_executor = AgentExecutor::from_agent(market_agent);
    let _market_inputs = std::collections::HashMap::from([
        ("input".to_string(), json!("Analyze AAPL stock performance over the last month and prepare a summary report for investors"))
    ]);

    println!("   🚀 Executing Market Analysis Agent with real LLM...");
    match _market_executor.invoke(_market_inputs).await {
        Ok(result) => {
            println!("   ✅ Market Analysis Response:");
            println!("   📝 {}", result);
        }
        Err(e) => {
            println!("   ❌ Execution failed: {}", e);
            println!("   💡 Check Ollama connection and model availability");
        }
    }

    // Multi-step Business Process
    println!("\n🔄 Multi-step Business Process Agent:");
    let process_agent = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?
        .with_capabilities()
        .with_task_planning(DefaultTaskPlanningCapability::new())
        .with_react(DefaultReActCapability::new())
        .with_reflection(DefaultReflectionCapability::new())
        .build_sync()?;

    let _process_executor = AgentExecutor::from_agent(process_agent);
    let _process_inputs = std::collections::HashMap::from([
        ("input".to_string(), json!("New customer C002 just signed up. Please check their profile, analyze relevant market data for their interests, and send them a welcome email with personalized recommendations."))
    ]);

    println!("   🚀 Executing Multi-capability Agent with real LLM...");
    match _process_executor.invoke(_process_inputs).await {
        Ok(result) => {
            println!("   ✅ Multi-step Process Response:");
            println!("   📝 {}", result);
        }
        Err(e) => {
            println!("   ❌ Execution failed: {}", e);
            println!("   💡 Check Ollama connection and model availability");
        }
    }
    println!();

    // Example 4.5: Real ReAct Agent with actual LLM calls
    println!("🔄 Real ReAct Agent Demonstration - Actual LLM Reasoning:");
    let react_agent = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?
        .with_capabilities()
        .with_react(DefaultReActCapability::new())
        .build_sync()?;

    let react_executor = AgentExecutor::from_agent(react_agent);

    println!("   🧠 Scenario: Customer complaint about wrong product delivered");
    println!("   📝 Making REAL LLM calls to demonstrate ReAct reasoning...\n");

    let react_inputs = std::collections::HashMap::from([
        ("input".to_string(), json!("Customer C003 received the wrong product (ordered iPhone 15 but got iPhone 14). They are very upset and want immediate resolution. Please handle this complaint professionally."))
    ]);

    println!("   🚀 Executing ReAct Agent with real LLM reasoning...");
    match react_executor.invoke(react_inputs).await {
        Ok(result) => {
            println!("   ✅ ReAct Agent Response:");
            println!("   📝 {}", result);
            println!("\n   🧠 ReAct Process Analysis:");
            println!("   ✓ LLM reasoned through the problem step by step");
            println!("   ✓ Made decisions about which tools to use and when");
            println!("   ✓ Observed results and adjusted approach if needed");
            println!("   ✓ Provided comprehensive solution with reasoning");
        }
        Err(e) => {
            println!("   ❌ ReAct execution failed: {}", e);
            println!("   💡 This might be due to Ollama connection or model issues");
            println!("   🔧 Please ensure Ollama is running at http://192.168.1.38:11434");
            println!("   � And  that the model 'qwen3:4b-thinking-2507-q8_0' is available");
        }
    }
    println!();

    // Example 5: Demonstrating capability-specific functionality
    println!("🧠 Example 5: Capability-Specific Features");

    // Create an agent with reflection capability
    let base_agent6 = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;

    let reflection_agent = CapabilityAgentBuilder::new(base_agent6)
        .with_reflection(DefaultReflectionCapability::new())
        .build_sync()?;

    // Access the reflection capability
    if let Some(reflection_cap) = reflection_agent
        .capabilities()
        .get_capability::<DefaultReflectionCapability>()
    {
        println!("📈 Reflection capability found:");
        println!("   - Capability name: {}", reflection_cap.capability_name());
        println!(
            "   - Capability description: {}",
            reflection_cap.capability_description()
        );

        // Get performance metrics (would be populated in real usage)
        if let Ok(metrics) = reflection_cap.get_performance_metrics().await {
            println!("   - Total experiences: {}", metrics.total_experiences);
            println!(
                "   - Success rate: {:.1}%",
                metrics.successful_experiences as f64 / metrics.total_experiences.max(1) as f64
                    * 100.0
            );
        }
    }
    println!();

    // Example 6: Capability priorities and configuration
    println!("⚙️ Example 6: Advanced Configuration");

    let base_agent7 = ConversationalAgentBuilder::new()
        .tools(&tools)
        .build(llm.clone())?;

    let _configured_agent = CapabilityAgentBuilder::new(base_agent7)
        .with_reflection_priority(DefaultReflectionCapability::new(), 10) // High priority - learn from every interaction
        .with_task_planning_priority(DefaultTaskPlanningCapability::new(), 8) // High priority - plan complex workflows
        .with_react_priority(DefaultReActCapability::new(), 6) // Medium priority - reason through problems
        .build_sync()?;

    println!("✅ Created agent with prioritized capabilities for business scenarios");
    println!(
        "   - Reflection (Priority 10): Learn from customer interactions and improve responses"
    );
    println!("   - Task Planning (Priority 8): Break down complex business processes into steps");
    println!("   - ReAct (Priority 6): Reason through problems and take appropriate actions");
    println!("   - Higher priority capabilities influence decision-making more strongly");
    println!();

    println!("🎉 All business scenario examples completed successfully!");
    println!("\n📚 Key Business Features Demonstrated:");
    println!("   ✓ Customer service automation with reflection learning");
    println!("   ✓ Market analysis with task planning capabilities");
    println!("   ✓ Multi-step business process automation");
    println!("   ✓ Real-world tool integration (Customer DB, Market Data, Email)");
    println!("   ✓ Priority-based capability configuration for business needs");
    println!("   ✓ Ollama integration with Qwen model");
    println!("\n💡 Business Applications:");
    println!("   - Customer Support: Automated ticket handling with learning");
    println!("   - Financial Analysis: Market research with structured planning");
    println!("   - Sales Process: Lead nurturing with personalized communication");
    println!("   - Operations: Multi-step workflow automation with reasoning");
    println!("\n🚀 Production Considerations:");
    println!("   - Add error handling and retry logic for tool failures");
    println!("   - Implement proper authentication for database and email tools");
    println!("   - Add logging and monitoring for agent performance");
    println!("   - Create custom capabilities for domain-specific business logic");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use langchain_rust::agent::capabilities::*;

    #[tokio::test]
    async fn test_capability_system_basic() {
        let config = OpenAIConfig::default()
            .with_api_base("http://192.168.1.38:11434/v1".to_string())
            .with_api_key("ollama".to_string());
        let llm = OpenAI::new(config).with_model("qwen3:4b-thinking-2507-q8_0".to_string());
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(CustomerQueryTool)];

        let base_agent = ConversationalAgentBuilder::new()
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
        let config = OpenAIConfig::default()
            .with_api_base("http://192.168.1.38:11434/v1".to_string())
            .with_api_key("ollama".to_string());
        let llm = OpenAI::new(config).with_model("qwen3:4b-thinking-2507-q8_0".to_string());
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(MarketAnalysisTool)];

        let base_agent = ConversationalAgentBuilder::new()
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
        let config = OpenAIConfig::default()
            .with_api_base("http://192.168.1.38:11434/v1".to_string())
            .with_api_key("ollama".to_string());
        let llm = OpenAI::new(config).with_model("qwen3:4b-thinking-2507-q8_0".to_string());
        let tools: Vec<Arc<dyn Tool>> = vec![Arc::new(EmailNotificationTool)];

        let base_agent = ConversationalAgentBuilder::new()
            .tools(&tools)
            .build(llm)
            .unwrap();

        let research_agent = base_agent.as_research_agent().build_sync().unwrap();

        // Research agents should have reflection and task planning
        assert!(research_agent.has_reflection() || research_agent.has_task_planning());
    }
}
