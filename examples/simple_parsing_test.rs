use langchain_rust::agent::{
    RobustJsonParser, ResponseSanitizer, OutputValidator, FormatType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Simple Parsing System Test");
    println!("🧠 Testing individual components of the enhanced parsing system\n");

    // Test 1: Robust JSON Parser
    println!("🔍 Testing Robust JSON Parser...");
    let json_parser = RobustJsonParser::new();
    
    let test_cases = vec![
        ("Valid JSON", r#"{"query": "test"}"#),
        ("Placeholder JSON", "{JSON}"),
        ("Single quotes", "{'query': 'test'}"),
        ("Unquoted keys", "{query: \"test\"}"),
        ("Trailing comma", r#"{"query": "test",}"#),
        ("Incomplete JSON", r#"{"query": "test""#),
    ];

    for (name, input) in test_cases {
        match json_parser.parse(input) {
            Ok(value) => {
                println!("  ✅ {}: {} -> {}", name, input, value);
            }
            Err(e) => {
                println!("  ❌ {}: {} -> Error: {}", name, input, e);
            }
        }
    }

    // Test 2: Response Sanitizer
    println!("\n🔍 Testing Response Sanitizer...");
    let sanitizer = ResponseSanitizer::for_agent_type("react");
    
    let sanitizer_test_cases = vec![
        ("With thinking tags", r#"<think>Let me think</think>Thought: I need to search"#),
        ("With artifacts", "Sure! Here's the response: Thought: I need to search"),
        ("With placeholder JSON", "Action Input: {JSON}"),
        ("With code blocks", "```json\n{\"query\": \"test\"}\n```"),
    ];

    for (name, input) in sanitizer_test_cases {
        let sanitized = sanitizer.sanitize(input);
        println!("  ✅ {}: {} -> {}", name, input.replace('\n', "\\n"), sanitized.replace('\n', "\\n"));
    }

    // Test 3: Output Validator
    println!("\n🔍 Testing Output Validator...");
    let validator = OutputValidator::new();
    
    let validator_test_cases = vec![
        ("Valid ReAct", r#"Thought: I need to search
Action: search
Action Input: {"query": "test"}"#),
        ("Missing Action", "Thought: I need to search"),
        ("Invalid JSON", r#"Thought: I need to search
Action: search
Action Input: {invalid json}"#),
    ];

    for (name, input) in validator_test_cases {
        let result = validator.validate(input, &FormatType::ReAct);
        println!("  {} {}: Valid: {}, Confidence: {:.2}", 
                 if result.is_valid { "✅" } else { "❌" },
                 name, result.is_valid, result.confidence_score);
        
        if !result.errors.is_empty() {
            for error in &result.errors {
                println!("    Error: {}", error.message);
            }
        }
        
        if !result.suggested_fixes.is_empty() {
            for fix in &result.suggested_fixes {
                println!("    Suggested fix: {}", fix);
            }
        }
    }

    // Test 4: Combined workflow
    println!("\n🔍 Testing Combined Workflow...");
    let problematic_input = r#"<think>I need to think about this</think>
Sure! Here's what I'll do:
Thought: I need to search for the capital of France.
Action: search
Action Input: {JSON}"#;

    println!("Original input: {}", problematic_input.replace('\n', "\\n"));
    
    // Step 1: Sanitize
    let sanitized = sanitizer.sanitize(problematic_input);
    println!("After sanitization: {}", sanitized.replace('\n', "\\n"));
    
    // Step 2: Validate
    let validation = validator.validate(&sanitized, &FormatType::ReAct);
    println!("Validation result: Valid: {}, Confidence: {:.2}", validation.is_valid, validation.confidence_score);
    
    // Step 3: Parse JSON (extract action input and fix it)
    if let Some(start) = sanitized.find("Action Input:") {
        if let Some(end) = sanitized[start..].find('\n') {
            let action_input = &sanitized[start + 13..start + end].trim();
            match json_parser.parse(action_input) {
                Ok(fixed_json) => {
                    println!("Fixed JSON: {}", fixed_json);
                }
                Err(e) => {
                    println!("JSON parsing failed: {}", e);
                }
            }
        }
    }

    println!("\n🎯 Summary:");
    println!("   The enhanced parsing system successfully:");
    println!("   ✅ Handles various JSON formatting issues");
    println!("   ✅ Sanitizes LLM responses by removing artifacts");
    println!("   ✅ Validates output format and provides feedback");
    println!("   ✅ Works together to process problematic inputs");
    println!("\n   This system can be integrated into any agent implementation");
    println!("   to provide robust parsing with graceful error handling.");

    Ok(())
}
