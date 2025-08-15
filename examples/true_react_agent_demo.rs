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

/// Customer database query tool
struct CustomerQueryTool;

#[async_trait]
impl Tool for CustomerQueryTool {
    fn name(&self) -> String {
        "customer_query".to_string()
    }

    fn description(&self) -> String {
        "Query customer information from the database. Use this to get customer profile, orders, or support tickets.".to_string()
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

        println!("🔧 [TOOL] customer_query(customer_id: {}, query_type: {})", customer_id, query_type);
        
        // Simulate database query
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let result = match query_type {
            "profile" => format!(
                "Customer {} - Name: John Doe, Email: john@example.com, Status: VIP, Phone: +1-555-0123, Account Balance: $1,250",
                customer_id
            ),
            "orders" => format!(
                "Customer {} recent orders: #1001 (iPhone 15 - Ordered 2024-01-15, Status: Processing), #1002 (AirPods - Delivered 2024-01-10), #1003 (iPhone 14 - Delivered 2024-01-16 by mistake - ERROR)",
                customer_id
            ),
            "support_tickets" => format!(
                "Customer {} support tickets: #T-456 (Wrong product delivered - iPhone 14 instead of iPhone 15, Status: Open, Priority: High, Created: 2024-01-16)",
                customer_id
            ),
            _ => format!("Unknown query type for customer {}", customer_id),
        };
        
        println!("📊 [RESULT] {}", result);
        Ok(result)
    }
}

/// Order management tool
struct OrderManagementTool;

#[async_trait]
impl Tool for OrderManagementTool {
    fn name(&self) -> String {
        "order_management".to_string()
    }

    fn description(&self) -> String {
        "Manage customer orders - create replacements, process refunds, or update status.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create_replacement", "process_refund", "update_status"],
                    "description": "Action to perform on the order"
                },
                "order_id": {
                    "type": "string",
                    "description": "Original order ID"
                },
                "details": {
                    "type": "string",
                    "description": "Additional details for the action"
                }
            },
            "required": ["action", "order_id"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let action = input["action"].as_str().unwrap_or("unknown");
        let order_id = input["order_id"].as_str().unwrap_or("unknown");
        let details = input["details"].as_str().unwrap_or("");

        println!("🔧 [TOOL] order_management(action: {}, order_id: {})", action, order_id);
        
        // Simulate order processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let result = match action {
            "create_replacement" => format!(
                "✅ Replacement order #R{}-001 created for original order {}. iPhone 15 will ship within 24 hours via express delivery. Tracking: EX123456789. {}",
                order_id, order_id, details
            ),
            "process_refund" => format!(
                "✅ Refund of $999 processed for order {}. Amount will be credited to customer's payment method within 3-5 business days. Reference: REF-{}. {}",
                order_id, order_id, details
            ),
            "update_status" => format!(
                "✅ Order {} status updated to: {}. Customer will be notified automatically.",
                order_id, details
            ),
            _ => format!("❌ Unknown action '{}' for order {}", action, order_id),
        };
        
        println!("📊 [RESULT] {}", result);
        Ok(result)
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
        "Send email notifications to customers.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "recipient": {
                    "type": "string",
                    "description": "Email recipient address"
                },
                "subject": {
                    "type": "string",
                    "description": "Email subject line"
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
        let _message = input["message"].as_str().unwrap_or("No Content");

        println!("🔧 [TOOL] send_email(recipient: {}, subject: '{}')", recipient, subject);
        
        // Simulate email sending
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let result = format!(
            "✅ Email sent successfully to {} with subject '{}'. Message delivered to inbox. Delivery ID: EM-{:x}",
            recipient, 
            subject,
            std::ptr::addr_of!(recipient) as usize
        );
        
        println!("📊 [RESULT] {}", result);
        Ok(result)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the LLM with Ollama configuration
    let config = OpenAIConfig::default()
        .with_api_base("http://192.168.1.38:11434/v1".to_string())
        .with_api_key("ollama".to_string());

    let llm = OpenAI::new(config).with_model("qwen3:4b-thinking-2507-q8_0".to_string());

    // Create tools
    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(CustomerQueryTool),
        Arc::new(OrderManagementTool),
        Arc::new(EmailNotificationTool),
    ];

    println!("🚀 True ReAct Agent Demonstration");
    println!("🧠 Using REAL ReAct Pattern - LLM makes autonomous decisions");
    println!("🔄 Agent will perform autonomous Thought-Action-Observation cycles\n");

    // Create ReAct agent with very explicit instructions
    let react_agent = ReActAgentBuilder::new()
        .tools(&tools)
        .prefix(r#"You are a customer service ReAct agent. You MUST use the ReAct format.

CRITICAL: Do NOT respond conversationally. You MUST start every response with "Thought:" followed by "Action:" and "Action Input:".

Your job is to resolve customer issues using the available tools. Always start with gathering customer information."#)
        .build(llm)?;

    println!("✅ Created ReAct agent with {} tools", tools.len());

    // Create executor with memory
    let memory = SimpleMemory::new();
    let executor = AgentExecutor::from_agent(react_agent)
        .with_memory(memory.into())
        .with_max_iterations(10);

    println!("🔍 Starting autonomous ReAct cycles...\n");

    // Test case: Complex customer complaint requiring multiple steps
    let test_input = "Customer C003 is extremely upset. They ordered iPhone 15 but received iPhone 14. They are threatening to cancel and demanding immediate resolution. Please help resolve this issue completely.";

    println!("📝 Input: {}\n", test_input);
    println!("{}", "=".repeat(80));
    println!("🤖 LLM will now autonomously decide what to do...");
    println!("{}", "=".repeat(80));

    match executor.invoke(prompt_args! {
        "input" => test_input
    }).await {
        Ok(result) => {
            println!("\n{}", "=".repeat(80));
            println!("🎯 Final Result:");
            println!("{}", "=".repeat(80));
            println!("{}", result);
            
            println!("\n{}", "=".repeat(80));
            println!("✅ ReAct Agent Analysis:");
            println!("{}", "=".repeat(80));
            println!("🧠 The LLM autonomously:");
            println!("   ✓ Analyzed the customer complaint");
            println!("   ✓ Decided which tools to use and when");
            println!("   ✓ Made multiple tool calls based on observations");
            println!("   ✓ Reasoned through each step independently");
            println!("   ✓ Provided a comprehensive solution");
            println!("\n🔄 This demonstrates REAL ReAct behavior:");
            println!("   • Thought: LLM's internal reasoning");
            println!("   • Action: LLM's autonomous tool selection");
            println!("   • Observation: Tool results feeding back to LLM");
            println!("   • Repeat until problem is solved");
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}
