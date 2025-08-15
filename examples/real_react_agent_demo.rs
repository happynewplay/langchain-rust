use async_trait::async_trait;
use langchain_rust::{
    agent::{AgentExecutor, ConversationalAgentBuilder},
    chain::Chain,
    llm::openai::{OpenAI, OpenAIConfig},
    tools::Tool,
};
use serde_json::{json, Value};
use std::collections::HashMap;
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

        println!(
            "🔧 [TOOL CALL] customer_query(customer_id: {}, query_type: {})",
            customer_id, query_type
        );

        // Simulate database query
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let result = match query_type {
            "profile" => format!(
                "Customer {} - Name: John Doe, Email: john@example.com, Status: Active, Phone: +1-555-0123",
                customer_id
            ),
            "orders" => format!(
                "Customer {} recent orders: #1001 (iPhone 15 - Ordered 2024-01-15, Status: Pending), #1002 (AirPods - Delivered 2024-01-10), #1003 (iPhone 14 - Delivered 2024-01-16 by mistake)",
                customer_id
            ),
            "support_tickets" => format!(
                "Customer {} support tickets: #T-456 (Wrong product delivered - iPhone 14 instead of iPhone 15, Status: Open, Priority: High)",
                customer_id
            ),
            _ => format!("Unknown query type for customer {}", customer_id),
        };

        println!("📊 [TOOL RESULT] {}", result);
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
        "Send email notifications to customers".to_string()
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
        let message = input["message"].as_str().unwrap_or("No Content");

        println!(
            "🔧 [TOOL CALL] send_email(recipient: {}, subject: {})",
            recipient, subject
        );

        // Simulate email sending
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let result = format!(
            "Email sent successfully to {} with subject '{}'. Message preview: {}...",
            recipient,
            subject,
            if message.len() > 50 {
                &message[..50]
            } else {
                message
            }
        );

        println!("📊 [TOOL RESULT] {}", result);
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
        "Manage customer orders - create replacements, refunds, or updates".to_string()
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

        println!(
            "🔧 [TOOL CALL] order_management(action: {}, order_id: {})",
            action, order_id
        );

        // Simulate order processing
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let result = match action {
            "create_replacement" => format!(
                "Replacement order created for {}. New order #R{}-001 will ship within 24 hours. {}",
                order_id, order_id, details
            ),
            "process_refund" => format!(
                "Refund processed for order {}. Amount will be credited within 3-5 business days. {}",
                order_id, details
            ),
            "update_status" => format!(
                "Order {} status updated. {}",
                order_id, details
            ),
            _ => format!("Unknown action for order {}", order_id),
        };

        println!("📊 [TOOL RESULT] {}", result);
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
        Arc::new(EmailNotificationTool),
        Arc::new(OrderManagementTool),
    ];

    println!("🚀 Real ReAct Agent Demonstration");
    println!("🧠 Using langchain-rust ConversationalAgent with ReAct capabilities");
    println!("📝 Agent will autonomously decide which tools to use and when\n");

    // Create ReAct agent
    let agent = ConversationalAgentBuilder::new().tools(&tools).build(llm)?;

    let executor = AgentExecutor::from_agent(agent);

    // Single Complex ReAct Test Case - Agent will perform multiple reasoning-action cycles autonomously
    println!("{}", "=".repeat(80));
    println!("🔥 Complex ReAct Agent Test - Multi-Step Autonomous Reasoning");
    println!("{}", "=".repeat(80));

    let complex_inputs = HashMap::from([
        ("input".to_string(), json!("Customer C003 is extremely upset and threatening to leave. They ordered an iPhone 15 but received an iPhone 14. They want immediate resolution including: 1) Explanation of what went wrong, 2) Immediate replacement, 3) Compensation for the inconvenience, 4) Assurance this won't happen again. Please handle this comprehensively - investigate the issue, take corrective actions, and provide a complete resolution."))
    ]);

    println!("📥 Complex Input: Angry customer C003 demanding comprehensive resolution");
    println!("🤖 Agent will now perform MULTIPLE reasoning-action cycles autonomously...");
    println!("🔄 Watch for multiple tool calls as agent reasons through the problem\n");

    match executor.invoke(complex_inputs).await {
        Ok(result) => {
            println!("\n✅ Agent Final Response After Multiple ReAct Cycles:");
            println!("📝 {}", result);
        }
        Err(e) => {
            println!("❌ Agent execution failed: {}", e);
        }
    }

    println!("\n\n🎯 ReAct Agent Analysis:");
    println!("✓ Agent autonomously chose which tools to use");
    println!("✓ Agent reasoned through problems step by step");
    println!("✓ Agent observed tool results and adjusted strategy");
    println!("✓ Agent made decisions based on context and observations");
    println!("✓ Agent provided comprehensive solutions");

    println!("\n🚀 Key Differences from Manual Approach:");
    println!("✓ No manual tool selection - agent decides autonomously");
    println!("✓ Agent handles reasoning-action-observation cycles internally");
    println!("✓ Agent can adapt strategy based on intermediate results");
    println!("✓ Agent provides natural language responses with actions taken");
    println!("✓ True AI-driven problem solving, not scripted workflows");

    Ok(())
}
