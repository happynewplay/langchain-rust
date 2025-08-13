# Multi-Agent System for LangChain Rust

This document describes the comprehensive multi-agent system implementation for langchain-rust, providing team orchestration, human interaction, and universal tool integration capabilities.

## Overview

The multi-agent system extends langchain-rust with sophisticated agent coordination patterns, enabling:

- **Team Agents**: Orchestrate multiple agents with various execution patterns
- **Human Agents**: Integrate human intervention based on configurable conditions
- **Team-Human Hybrid Agents**: Combine team orchestration with human oversight
- **Universal Tool Integration**: Use any agent as a tool, with MCP support
- **Nested Team Support**: Create complex hierarchical agent structures

## Core Components

### 1. Team Agents

Team agents coordinate multiple child agents using different execution patterns:

#### Execution Patterns

- **Concurrent**: All agents execute simultaneously, results are aggregated
- **Sequential**: Agents execute in order, each receiving the previous agent's output
- **Hybrid**: Complex dependency chains with mixed concurrent/sequential execution

#### Basic Usage

```rust
use langchain_rust::{
    agent::{TeamAgentBuilder, ExecutionPattern},
    memory::SimpleMemory,
};
use std::sync::Arc;
use tokio::sync::Mutex;

// Create shared memory for team coordination
let team_memory = Arc::new(Mutex::new(SimpleMemory::new()));

// Sequential team with memory
let team = TeamAgentBuilder::sequential_team([
    ("agent_a", agent_a),
    ("agent_b", agent_b),
])
.prefix("You are coordinating a sequential workflow.")
.memory(team_memory.clone())
.coordination_prompts(true)
.build()?;

// Concurrent team
let team = TeamAgentBuilder::concurrent_team([
    ("agent_a", agent_a),
    ("agent_b", agent_b),
])
.memory(team_memory.clone())
.build()?;

// Complex hybrid pattern
let team = TeamAgentBuilder::pipeline_with_concurrent(
    ("agent_a", agent_a),  // Runs first
    ("agent_b", agent_b),  // Runs concurrently with agent_c
    ("agent_c", agent_c),  // Runs concurrently with agent_b
    ("agent_d", agent_d),  // Runs after agent_b and agent_c complete
)
.build()?;
```

#### Advanced Patterns

```rust
// Custom hybrid execution
let steps = vec![
    ExecutionStep {
        agent_ids: vec!["agent_a".to_string()],
        concurrent: false,
        dependencies: vec![],
    },
    ExecutionStep {
        agent_ids: vec!["agent_b".to_string(), "agent_c".to_string()],
        concurrent: true,
        dependencies: vec![0], // Depends on step 0
    },
];

let team = TeamAgentBuilder::new()
    .add_agent("agent_a", agent_a)
    .add_agent("agent_b", agent_b)
    .add_agent("agent_c", agent_c)
    .hybrid(steps)
    .build()?;
```

### 2. Human Agents

Human agents enable conditional human intervention based on configurable triggers:

#### Configuration

```rust
use langchain_rust::{
    agent::{
        HumanAgentBuilder,
        human::{InterventionCondition, TerminationCondition}
    },
    memory::SimpleMemory,
};
use std::sync::Arc;
use tokio::sync::Mutex;

// Create memory for human agent
let human_memory = Arc::new(Mutex::new(SimpleMemory::new()));

let human_agent = HumanAgentBuilder::new()
    .add_intervention_condition(
        InterventionCondition::new("error", "error")
            .with_description("Intervene on errors")
    )
    .add_intervention_condition(
        InterventionCondition::regex(r"complex|difficult", "input")
            .with_description("Intervene on complex tasks")
    )
    .add_termination_condition(
        TerminationCondition::new("done", "input")
            .with_description("Terminate when user says done")
    )
    .add_termination_condition(
        TerminationCondition::similarity("finished", "input", 0.8)
            .with_description("Terminate on similar phrases")
    )
    .max_interventions(5)
    .input_timeout(300) // 5 minutes
    .memory(human_memory.clone())
    .include_memory_in_prompts(true)
    .build()?;
```

#### Pre-built Patterns

```rust
// Intervene on errors
let agent = HumanAgentBuilder::error_intervention().build()?;

// Intervene on keywords
let agent = HumanAgentBuilder::keyword_intervention(vec!["help", "review"]).build()?;

// Intervene on regex patterns
let agent = HumanAgentBuilder::regex_intervention(vec![r"\berror\b", r"failed?"]).build()?;

// Always intervene (manual control)
let agent = HumanAgentBuilder::always_intervene().build()?;
```

### 3. Team-Human Hybrid Agents

Combine team orchestration with human intervention:

```rust
use langchain_rust::{
    agent::TeamHumanAgentBuilder,
    memory::SimpleMemory,
};
use std::sync::Arc;
use tokio::sync::Mutex;

// Create shared memory for team-human hybrid
let hybrid_memory = Arc::new(Mutex::new(SimpleMemory::new()));

let hybrid_agent = TeamHumanAgentBuilder::new()
    .add_agent("math_agent", math_agent)
    .add_agent("data_agent", data_agent)
    .sequential()
    .memory(hybrid_memory.clone())
    .coordination_prompts(true)
    .include_memory_in_prompts(true)
    .add_intervention_condition(
        InterventionCondition::new("complex", "input")
    )
    .add_termination_condition(
        TerminationCondition::new("done", "input")
    )
    .intervene_before_team(true)   // Check before team execution
    .intervene_after_team(false)   // Don't check after team execution
    .intervene_on_team_error(true) // Check on team errors
    .build()?;
```

### 4. Universal Tool Integration

#### Agent Registry

Manage multiple agents and convert them to tools:

```rust
use langchain_rust::agent::AgentRegistry;

let mut registry = AgentRegistry::new()
    .with_default_timeout(300); // 5 minutes

registry.register("math_specialist", math_agent);
registry.register("data_analyst", data_agent);
registry.register("system_admin", system_agent);

// Convert all agents to tools
let agent_tools = registry.as_tools();

// Combine with regular tools
let all_tools = registry.combined_tools(&regular_tools);
```

#### Universal Agent Tool

Wrap any agent as a tool:

```rust
use langchain_rust::agent::UniversalAgentTool;

let agent_tool = UniversalAgentTool::new(
    agent,
    "my_agent",
    "A specialized agent for specific tasks"
)
.with_timeout(60);

// Use in another agent
let meta_agent = OpenAiToolAgentBuilder::new()
    .tools(&[Arc::new(agent_tool)])
    .build(llm)?;
```

#### MCP Integration

When the `mcp` feature is enabled:

```rust
use langchain_rust::agent::mcp_integration;

// Create universal toolset with MCP
let tools = mcp_integration::create_universal_toolset(
    &regular_tools,
    &agent_registry,
    Some(&mcp_client)
).await?;

// Without MCP
let tools = mcp_integration::create_toolset_without_mcp(
    &regular_tools,
    &agent_registry
);
```

### 5. Nested Team Agents

Create hierarchical agent structures:

```rust
// Create sub-teams
let analysis_team = Arc::new(TeamAgentBuilder::concurrent_team([
    ("math_agent", math_agent),
    ("data_agent", data_agent),
]).build()?) as Arc<dyn Agent>;

let operations_team = Arc::new(TeamAgentBuilder::new()
    .add_agent("system_admin", system_agent)
    .build()?) as Arc<dyn Agent>;

// Create master team with nested teams
let master_team = TeamAgentBuilder::nested_team_pattern(
    ("analysis_team", analysis_team),
    ("operations_team", operations_team),
    ("quality_team", quality_team),
    ("coordinator", coordinator_agent),
)
.build()?;
```

## LLM and Memory Integration

### Memory Support

All agent types support memory integration for maintaining conversation history and context:

#### Team Agent Memory

```rust
use langchain_rust::{agent::TeamAgentBuilder, memory::SimpleMemory};
use std::sync::Arc;
use tokio::sync::Mutex;

let team_memory = Arc::new(Mutex::new(SimpleMemory::new()));

let team = TeamAgentBuilder::new()
    .add_agent("agent_a", agent_a)
    .add_agent("agent_b", agent_b)
    .memory(team_memory.clone())
    .coordination_prompts(true)  // Include team coordination context
    .build()?;
```

#### Human Agent Memory

```rust
let human_memory = Arc::new(Mutex::new(SimpleMemory::new()));

let human_agent = HumanAgentBuilder::new()
    .add_intervention_condition(InterventionCondition::new("help", "input"))
    .add_termination_condition(TerminationCondition::new("done", "input"))
    .memory(human_memory.clone())
    .include_memory_in_prompts(true)  // Include chat history in human prompts
    .build()?;
```

#### Shared Memory Across Agent Types

```rust
// Create shared memory for coordination
let shared_memory = Arc::new(Mutex::new(SimpleMemory::new()));

// Team-human hybrid with shared memory
let hybrid_agent = TeamHumanAgentBuilder::new()
    .add_agent("specialist", specialist_agent)
    .memory(shared_memory.clone())  // Shared across team and human components
    .coordination_prompts(true)
    .include_memory_in_prompts(true)
    .build()?;
```

### Memory Features

- **Shared Context**: Memory can be shared across multiple agents for coordination
- **Conversation History**: Automatic tracking of agent interactions and human inputs
- **Context Injection**: Memory content is automatically included in agent prompts
- **Coordination Prompts**: Team agents can include coordination context from memory
- **Human Context**: Human agents can include conversation history in intervention prompts

### LLM Integration

While team and human agents don't directly use LLMs (they orchestrate other agents that do), they fully support:

- **Child Agent LLMs**: All child agents can use any supported LLM
- **Memory Integration**: LLM-based child agents benefit from shared memory context
- **Tool Integration**: LLM agents can use team/human agents as tools
- **Prompt Enhancement**: Memory and coordination context enhances LLM prompts

## Advanced Features

### Serialization Support

```rust
use langchain_rust::agent::serialization;

// Execute agent with serializable response
let response = serialization::execute_agent_serializable(agent, inputs).await;

// Serialize to JSON
let json = response.to_json()?;

// Deserialize from JSON
let response = SerializableAgentResponse::from_json(&json)?;
```

### Error Handling and Timeouts

All agent types support:
- Configurable timeouts
- Error propagation
- Critical vs non-critical agent failures
- Graceful degradation

### Memory Integration

All agents integrate with langchain-rust memory systems:

```rust
let executor = AgentExecutor::from_agent(team_agent)
    .with_memory(SimpleMemory::new().into());
```

## Examples

See `examples/multi_agent_system.rs` for a comprehensive demonstration of all features.

## Configuration Validation

All agent configurations include validation:
- Team agents must have at least one child agent
- Human agents must have intervention and termination conditions
- Hybrid execution patterns must have valid dependencies
- Agent IDs must be unique within teams

## Thread Safety

All components are designed for concurrent use:
- Agents implement `Send + Sync`
- Tools are thread-safe
- Memory systems use appropriate synchronization

## Integration with Existing Systems

The multi-agent system is fully compatible with:
- Existing langchain-rust agents
- All LLM providers
- Memory systems
- Tool ecosystem
- MCP protocol (when feature enabled)

## Performance Considerations

- Concurrent execution uses `tokio` for efficient async operations
- Timeouts prevent hanging operations
- Memory usage scales with number of concurrent agents
- Tool calls are properly awaited and error-handled

This multi-agent system provides a powerful foundation for building complex AI workflows with proper orchestration, human oversight, and universal tool integration.
