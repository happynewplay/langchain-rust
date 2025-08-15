# Enhanced MCP Multiple Tool Calling Support

This document describes the enhanced Model Context Protocol (MCP) multiple tool calling functionality implemented in langchain-rust.

## Overview

The langchain-rust agent system now supports advanced multiple MCP tool calling with the following enhancements:

- ✅ **Parallel MCP Tool Execution** - Execute multiple MCP tools concurrently when beneficial
- ✅ **Enhanced Error Handling** - MCP-specific error recovery and reporting
- ✅ **Configurable Execution** - Fine-grained control over execution behavior
- ✅ **Streaming Support** - Real-time monitoring of tool execution progress
- ✅ **Performance Optimization** - Intelligent grouping and batching of tool calls

## Key Features

### 1. Multiple Tool Calling Support

The agent system already supported multiple tool calling through the `AgentEvent::Action(Vec<AgentAction>)` structure. The enhancements add:

- **Parallel Execution**: MCP tools can be executed in parallel when safe to do so
- **Mixed Execution**: Regular tools and MCP tools can be mixed in the same action set
- **Intelligent Grouping**: Tools are automatically grouped by type for optimal execution

### 2. Enhanced MCP Executor

The `McpAgentExecutor` provides advanced execution capabilities:

```rust
use langchain_rust::agent::mcp_executor::{McpAgentExecutor, McpExecutionConfig};

let config = McpExecutionConfig {
    parallel_execution: true,
    max_parallel_calls: 3,
    tool_timeout_ms: 30000,
    retry_on_failure: true,
    max_retries: 2,
};

let executor = McpAgentExecutor::new(agent)
    .with_mcp_config(config)
    .with_max_iterations(10)
    .with_break_on_error(false);
```

### 3. Streaming Events

Enhanced event types for better monitoring:

```rust
pub enum McpAgentEvent {
    Planning,
    ToolCall { tool_name: String, tool_input: String },
    ParallelToolCalls { tool_names: Vec<String>, count: usize },
    ToolResult { tool_name: String, result: String, execution_time_ms: u64 },
    ParallelToolResults { results: Vec<(String, String, u64)> },
    McpError { error: String, tool_name: String, recoverable: bool },
    Finished { output: String },
    Error { error: String },
}
```

## Usage Examples

### Basic Multiple Tool Calling

```rust
use langchain_rust::{
    agent::{mcp_executor::McpAgentExecutor, OpenAiToolAgentBuilder},
    llm::openai::OpenAI,
    mcp::McpClient,
    prompt_args,
};

// Connect to MCP server
let mcp_client = McpClient::connect("http://127.0.0.1:8000/sse").await?;
let mcp_tools = mcp_client.get_langchain_tools().await?;

// Create agent with MCP tools
let agent = OpenAiToolAgentBuilder::new()
    .tools(&mcp_tools)
    .prefix("You can call multiple tools in parallel when appropriate.")
    .build(OpenAI::default())?;

// Create enhanced executor
let executor = McpAgentExecutor::new(Arc::new(agent))
    .with_parallel_execution(true)
    .with_max_parallel_calls(3);

// Execute with multiple tool calls
let result = executor.invoke(prompt_args! {
    "input" => "Use multiple tools to analyze the system comprehensively"
}).await?;
```

### Streaming Execution with Event Monitoring

```rust
use futures_util::StreamExt;

let mut stream = executor.stream(prompt_args! {
    "input" => "Perform analysis using all available tools"
}).await?;

while let Some(event) = stream.next().await {
    match event? {
        McpAgentEvent::ParallelToolCalls { tool_names, count } => {
            println!("⚡ Executing {} tools in parallel: {:?}", count, tool_names);
        }
        McpAgentEvent::ParallelToolResults { results } => {
            println!("⚡ Parallel execution completed:");
            for (tool_name, _, time_ms) in results {
                println!("  - {}: {}ms", tool_name, time_ms);
            }
        }
        McpAgentEvent::McpError { error, tool_name, recoverable } => {
            println!("❌ MCP Error in {}: {} (recoverable: {})", tool_name, error, recoverable);
        }
        McpAgentEvent::Finished { output } => {
            println!("🎉 Execution completed: {}", output);
            break;
        }
        _ => {} // Handle other events
    }
}
```

### Configuration Options

```rust
// High-performance configuration
let high_perf_config = McpExecutionConfig {
    parallel_execution: true,
    max_parallel_calls: 5,
    tool_timeout_ms: 15000,
    retry_on_failure: true,
    max_retries: 3,
};

// Conservative configuration
let conservative_config = McpExecutionConfig {
    parallel_execution: false,
    max_parallel_calls: 1,
    tool_timeout_ms: 60000,
    retry_on_failure: true,
    max_retries: 1,
};

// Custom configuration
let executor = McpAgentExecutor::new(agent)
    .with_mcp_config(high_perf_config)
    .with_max_iterations(15)
    .with_break_on_error(false);
```

## Architecture

### MCP Tool Detection

The system uses heuristic-based detection to identify MCP tools:

```rust
fn is_mcp_tool(&self, tool: &Arc<dyn Tool>) -> bool {
    let name = tool.name();
    let description = tool.description();
    
    name.starts_with("mcp_") || 
    description.contains("MCP") || 
    description.contains("Model Context Protocol") ||
    tool.parameters().get("mcp_server").is_some()
}
```

### Execution Flow

1. **Planning Phase**: Agent determines which tools to call
2. **Tool Grouping**: Tools are grouped by type (MCP vs regular)
3. **Parallel Execution**: MCP tools are executed in parallel (up to max_parallel_calls)
4. **Sequential Execution**: Regular tools are executed sequentially
5. **Result Aggregation**: All results are collected and processed
6. **Event Emission**: Appropriate events are emitted for monitoring

### Error Handling

- **MCP-Specific Errors**: Special handling for MCP connection and execution errors
- **Recoverable Errors**: Distinction between recoverable and fatal errors
- **Retry Logic**: Configurable retry behavior for failed tool calls
- **Graceful Degradation**: Continue execution when possible despite errors

## Performance Considerations

### Parallel Execution Benefits

- **Reduced Latency**: Multiple tools execute simultaneously
- **Better Resource Utilization**: Efficient use of network and compute resources
- **Improved Throughput**: Higher overall execution speed

### Configuration Guidelines

- **max_parallel_calls**: Set based on MCP server capacity (typically 3-5)
- **tool_timeout_ms**: Balance between responsiveness and reliability
- **retry_on_failure**: Enable for production environments
- **max_retries**: Keep low (1-3) to avoid excessive delays

## Integration with Existing Systems

### Backward Compatibility

- ✅ Existing agent code continues to work unchanged
- ✅ Regular tools are unaffected by MCP enhancements
- ✅ Sequential execution mode available for compatibility

### Agent Types

The enhanced MCP executor works with:

- `OpenAiToolAgent` - Full support for multiple tool calling
- `ConversationalAgent` - Basic support through tool integration
- `TeamAgent` - Can use MCP tools as part of team workflows
- Custom agents implementing the `Agent` trait

## Testing

Comprehensive test suite covers:

- ✅ Configuration validation
- ✅ Tool detection logic
- ✅ Parallel execution behavior
- ✅ Error handling scenarios
- ✅ Event emission correctness

Run tests with:
```bash
cargo test --features mcp mcp_executor
```

## Future Enhancements

Planned improvements include:

1. **Tool Batching**: Group multiple calls to the same MCP server
2. **Load Balancing**: Distribute calls across multiple MCP servers
3. **Caching**: Cache tool results for improved performance
4. **Metrics**: Detailed performance and reliability metrics
5. **Circuit Breaker**: Automatic failure detection and recovery

## Conclusion

The enhanced MCP multiple tool calling support provides a robust, performant, and flexible foundation for building sophisticated AI agents that can efficiently leverage multiple MCP tools. The implementation maintains backward compatibility while adding powerful new capabilities for parallel execution, enhanced error handling, and real-time monitoring.
