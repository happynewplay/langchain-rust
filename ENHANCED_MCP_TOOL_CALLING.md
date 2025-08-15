# Enhanced Multiple MCP Tool Calling Support

This document describes the enhanced multiple MCP (Model Context Protocol) tool calling functionality implemented in langchain-rust.

## Overview

The agent system in langchain-rust now supports sophisticated multiple MCP tool calling with the following enhancements:

- ✅ **Parallel MCP Tool Execution** - Execute multiple MCP tools concurrently when beneficial
- ✅ **Enhanced Error Handling** - MCP-specific error recovery and reporting
- ✅ **Configurable Execution** - Fine-tune parallel execution behavior
- ✅ **Streaming Support** - Real-time monitoring of tool execution progress
- ✅ **Performance Optimization** - Intelligent grouping and batching of tool calls

## Key Features

### 1. Multiple Tool Calling Support

The agent system already supported multiple tool calling through the `AgentEvent::Action(Vec<AgentAction>)` structure. The enhancements add:

- **Parallel Execution**: MCP tools can be executed in parallel when appropriate
- **Mixed Execution**: Regular tools and MCP tools can be mixed in the same action set
- **Intelligent Grouping**: Tools are automatically grouped by type for optimal execution

### 2. Enhanced MCP Executor

The `McpAgentExecutor` provides advanced execution capabilities:

```rust
use langchain_rust::agent::{McpAgentExecutor, McpExecutionConfig};

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

### 3. Advanced Event Monitoring

Enhanced event types for better observability:

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
    agent::{McpAgentExecutor, OpenAiToolAgentBuilder},
    mcp::McpClient,
    llm::openai::OpenAI,
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

// Create executor with parallel execution
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
    "input" => "Perform comprehensive analysis using all available tools"
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
let custom_executor = McpAgentExecutor::new(agent)
    .with_mcp_config(high_perf_config)
    .with_max_iterations(15)
    .with_break_on_error(false);
```

## Architecture

### Tool Detection and Grouping

The system automatically detects MCP tools using heuristics:

1. **Name-based detection**: Tools with names starting with "mcp_"
2. **Description-based detection**: Tools mentioning "MCP" or "Model Context Protocol"
3. **Parameter-based detection**: Tools with MCP-specific parameters

### Execution Strategy

1. **Sequential Execution**: Regular tools are executed sequentially
2. **Parallel Execution**: MCP tools are grouped and executed in parallel batches
3. **Mixed Execution**: Both strategies are combined for optimal performance

### Error Handling

- **MCP-specific errors**: Special handling for MCP connection and protocol errors
- **Recoverable errors**: Automatic retry for transient failures
- **Error isolation**: Failures in one tool don't necessarily stop others

## Performance Considerations

### Parallel Execution Benefits

- **Reduced Latency**: Multiple tools execute simultaneously
- **Better Resource Utilization**: Network and I/O operations overlap
- **Improved Throughput**: Higher overall task completion rate

### Configuration Guidelines

- **max_parallel_calls**: Start with 3-5, adjust based on server capacity
- **tool_timeout_ms**: Set based on expected tool execution time
- **retry_on_failure**: Enable for production environments
- **max_retries**: Keep low (1-3) to avoid excessive delays

## Integration with Existing Systems

### Backward Compatibility

- All existing agent code continues to work unchanged
- MCP tools integrate seamlessly with regular tools
- No breaking changes to existing APIs

### Team Agent Integration

```rust
// MCP tools work seamlessly with team agents
let team_agent = TeamAgentBuilder::sequential_team([
    ("mcp_specialist", mcp_agent),
    ("regular_agent", regular_agent),
])
.build()?;

let executor = McpAgentExecutor::new(Arc::new(team_agent))
    .with_parallel_execution(true);
```

### Memory Integration

```rust
// MCP agents support memory integration
let memory = Arc::new(Mutex::new(SimpleMemory::new()));
let executor = AgentExecutor::from_agent(mcp_agent)
    .with_memory(memory);
```

## Testing and Validation

The implementation includes comprehensive tests:

- **Unit tests**: Individual component functionality
- **Integration tests**: End-to-end execution scenarios
- **Performance tests**: Parallel execution benchmarks
- **Error handling tests**: Failure recovery scenarios

Run tests with:
```bash
cargo test --features mcp mcp_executor
```

## Future Enhancements

Planned improvements include:

1. **Batching Optimization**: Group calls to the same MCP server
2. **Load Balancing**: Distribute calls across multiple MCP servers
3. **Caching**: Cache tool results for repeated calls
4. **Metrics**: Detailed performance and usage metrics
5. **Circuit Breaker**: Automatic failure protection

## Conclusion

The enhanced multiple MCP tool calling support provides a robust, scalable foundation for building sophisticated AI agents that can efficiently leverage multiple MCP tools. The implementation maintains backward compatibility while adding powerful new capabilities for parallel execution, error handling, and performance optimization.
