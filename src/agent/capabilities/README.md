# Agent Capabilities System

The Agent Capabilities System provides an extensible framework for adding specialized behaviors to agents in langchain-rust. This system allows you to compose agents with different combinations of capabilities while maintaining clean separation of concerns and backward compatibility.

## Overview

The capability system is built around four core capability types:

1. **Reflection Agent** - Self-evaluation and learning from past actions
2. **Task Planning Agent** - Breaking down complex goals into executable sub-tasks  
3. **Code Execution Agent** - Executing and evaluating code snippets safely
4. **ReAct Agent** - Reasoning and acting in iterative cycles (Reason + Act pattern)

## Architecture

### Core Components

- **`AgentCapability`** - Base trait that all capabilities implement
- **`CapabilityManager`** - Manages a collection of capabilities for an agent
- **`CapabilityEnhancedAgent<A>`** - Wrapper that adds capabilities to existing agents
- **`CapabilityAgentBuilder<A>`** - Builder pattern for composing agents with capabilities

### Design Principles

- **Composable** - Mix and match capabilities as needed
- **Extensible** - Easy to add new capability types
- **Backward Compatible** - Existing agents work unchanged
- **Type Safe** - Compile-time capability checking
- **Low Coupling** - Capabilities don't depend on each other directly

## Quick Start

### Basic Usage

```rust
use langchain_rust::agent::{
    capabilities::{
        CapabilityAgentBuilder, DefaultReflectionCapability,
        DefaultTaskPlanningCapability, CapabilityBuilderExt
    },
    chat::ChatAgentBuilder,
};

// Create a base agent
let base_agent = ChatAgentBuilder::new()
    .tools(&tools)
    .build(llm)?;

// Add capabilities using the builder pattern
let enhanced_agent = CapabilityAgentBuilder::new(base_agent)
    .with_reflection(DefaultReflectionCapability::new())
    .with_task_planning(DefaultTaskPlanningCapability::new())
    .build_sync()?;

// Check what capabilities are available
println!("Has reflection: {}", enhanced_agent.has_reflection());
println!("Has task planning: {}", enhanced_agent.has_task_planning());
```

### Fluent Interface

```rust
// Use the fluent interface for more concise syntax
let enhanced_agent = base_agent
    .with_capabilities()
    .with_reflection(DefaultReflectionCapability::new())
    .with_task_planning(DefaultTaskPlanningCapability::new())
    .with_code_execution(DefaultCodeExecutionCapability::new())
    .with_react(DefaultReActCapability::new())
    .build_sync()?;
```

### Preset Combinations

```rust
// Use preset capability combinations for common use cases
let research_agent = base_agent.as_research_agent().build_sync()?;
let dev_agent = base_agent.as_development_agent().build_sync()?;
let analysis_agent = base_agent.as_analysis_agent().build_sync()?;
```

## Capability Types

### Reflection Capability

Enables agents to reflect on their actions and learn from experience.

**Features:**
- Action result analysis
- Performance metrics tracking
- Insight generation
- Learning from failures

**Usage:**
```rust
let reflection_cap = DefaultReflectionCapability::new();
let agent = base_agent
    .with_capabilities()
    .with_reflection(reflection_cap)
    .build_sync()?;

// Access reflection insights
if let Some(cap) = agent.capabilities().get_capability::<DefaultReflectionCapability>() {
    let insights = cap.get_reflection_insights().await?;
    let metrics = cap.get_performance_metrics().await?;
}
```

### Task Planning Capability

Breaks down complex tasks into manageable subtasks with dependencies.

**Features:**
- Task decomposition
- Dependency management
- Progress tracking
- Plan validation

**Usage:**
```rust
let planning_cap = DefaultTaskPlanningCapability::new();
let agent = base_agent
    .with_capabilities()
    .with_task_planning(planning_cap)
    .build_sync()?;

// Access planning functionality
if let Some(cap) = agent.capabilities().get_capability::<DefaultTaskPlanningCapability>() {
    let plan = cap.decompose_task("Complex task", &context).await?;
    let next_task = cap.get_next_subtask(&plan).await?;
}
```

### Code Execution Capability

Safely executes and validates code in multiple programming languages.

**Features:**
- Multi-language support (Python, JavaScript, Bash, SQL)
- Security validation
- Sandboxed execution
- Resource limits

**Usage:**
```rust
let code_cap = DefaultCodeExecutionCapability::new();
let agent = base_agent
    .with_capabilities()
    .with_code_execution(code_cap)
    .build_sync()?;

// Execute code safely
if let Some(cap) = agent.capabilities().get_capability::<DefaultCodeExecutionCapability>() {
    let validation = cap.validate_code("print('hello')", "python").await?;
    if validation.is_valid {
        let result = cap.execute_code("print('hello')", "python", &context).await?;
    }
}
```

### ReAct Capability

Implements the Reasoning + Acting pattern for iterative problem solving.

**Features:**
- Structured reasoning chains
- Action planning
- Cycle reflection
- Strategy adaptation

**Usage:**
```rust
let react_cap = DefaultReActCapability::new();
let agent = base_agent
    .with_capabilities()
    .with_react(react_cap)
    .build_sync()?;

// Execute ReAct cycles
if let Some(cap) = agent.capabilities().get_capability::<DefaultReActCapability>() {
    let reasoning = cap.reason("observation", &context).await?;
    let action = cap.plan_action(&reasoning, &tools).await?;
    let cycle = cap.execute_react_cycle("initial observation", &context, &tools).await?;
}
```

## Advanced Features

### Priority-Based Capabilities

```rust
let agent = CapabilityAgentBuilder::new(base_agent)
    .with_reflection_priority(DefaultReflectionCapability::new(), 10) // High priority
    .with_task_planning_priority(DefaultTaskPlanningCapability::new(), 5) // Medium priority
    .build_sync()?;
```

### Custom Capabilities

Implement your own capabilities by extending the base traits:

```rust
use langchain_rust::agent::capabilities::{AgentCapability, PlanningEnhancer};

struct CustomCapability {
    // Your capability data
}

impl AgentCapability for CustomCapability {
    fn capability_name(&self) -> &'static str {
        "custom_capability"
    }
    
    fn capability_description(&self) -> &'static str {
        "My custom capability"
    }
}

#[async_trait]
impl PlanningEnhancer for CustomCapability {
    async fn pre_plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: &mut PromptArgs,
    ) -> Result<(), AgentError> {
        // Your custom pre-planning logic
        Ok(())
    }
}
```

## Integration with AgentExecutor

Capability-enhanced agents work seamlessly with the existing `AgentExecutor`:

```rust
let enhanced_agent = base_agent
    .with_capabilities()
    .with_reflection(DefaultReflectionCapability::new())
    .with_task_planning(DefaultTaskPlanningCapability::new())
    .build_sync()?;

let executor = AgentExecutor::from_agent(enhanced_agent);
let result = executor.invoke(inputs).await?;
```

## Best Practices

1. **Start Simple** - Begin with one capability and add more as needed
2. **Use Presets** - Leverage preset combinations for common use cases
3. **Monitor Performance** - Use reflection capabilities to track agent performance
4. **Validate Code** - Always validate code before execution in production
5. **Handle Errors** - Implement proper error handling for capability operations
6. **Test Thoroughly** - Test capability combinations to ensure they work well together

## Examples

See `examples/agent_capabilities_example.rs` for comprehensive usage examples demonstrating all features of the capability system.

## Future Extensions

The capability system is designed to be easily extensible. Potential future capabilities include:

- **Memory Capability** - Advanced memory management and retrieval
- **Communication Capability** - Multi-agent communication protocols
- **Learning Capability** - Online learning and model adaptation
- **Monitoring Capability** - Real-time performance monitoring and alerting
- **Security Capability** - Advanced security scanning and threat detection
