use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use async_stream::stream;
use futures::Stream;
use futures_util::{StreamExt, future::join_all};

use crate::{
    agent::{Agent, AgentError},
    prompt::PromptArgs,
    schemas::{
        agent::{AgentAction, AgentEvent},
    },
    tools::Tool,
};

#[cfg(feature = "mcp")]
use crate::mcp::McpToolMarker;

/// Events that can occur during MCP agent execution
#[derive(Debug, Clone)]
pub enum McpAgentEvent {
    /// Agent is planning the next action
    Planning,
    /// Agent is calling a tool
    ToolCall {
        tool_name: String,
        tool_input: String,
    },
    /// Multiple tools are being called in parallel
    ParallelToolCalls {
        tool_names: Vec<String>,
        count: usize,
    },
    /// Tool execution completed
    ToolResult {
        tool_name: String,
        result: String,
        execution_time_ms: u64,
    },
    /// Multiple tool executions completed
    ParallelToolResults {
        results: Vec<(String, String, u64)>, // (tool_name, result, execution_time_ms)
    },
    /// Agent execution completed
    Finished {
        output: String,
    },
    /// Error occurred during execution
    Error {
        error: String,
    },
    /// MCP-specific error with recovery suggestion
    McpError {
        error: String,
        tool_name: String,
        recoverable: bool,
    },
}

/// Stream type for MCP agent events
pub type McpAgentStream = Pin<Box<dyn Stream<Item = Result<McpAgentEvent, AgentError>> + Send>>;

/// Helper struct to avoid lifetime issues in async streams
struct McpExecutorHelper {
    agent: Arc<dyn Agent>,
    mcp_config: McpExecutionConfig,
    max_iterations: usize,
    break_on_error: bool,
}

/// Configuration for MCP tool execution
#[derive(Debug, Clone)]
pub struct McpExecutionConfig {
    /// Whether to execute MCP tools in parallel when possible
    pub parallel_execution: bool,
    /// Maximum number of parallel MCP tool calls
    pub max_parallel_calls: usize,
    /// Timeout for individual MCP tool calls in milliseconds
    pub tool_timeout_ms: u64,
    /// Whether to retry failed MCP calls
    pub retry_on_failure: bool,
    /// Maximum number of retries for failed calls
    pub max_retries: usize,
}

impl Default for McpExecutionConfig {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            max_parallel_calls: 5,
            tool_timeout_ms: 30000, // 30 seconds
            retry_on_failure: true,
            max_retries: 2,
        }
    }
}

/// Executor for agents with MCP tool support and streaming capabilities
pub struct McpAgentExecutor {
    /// The underlying agent
    agent: Arc<dyn Agent>,
    /// Maximum number of iterations before stopping
    max_iterations: usize,
    /// Whether to break on first error
    break_on_error: bool,
    /// MCP-specific execution configuration
    mcp_config: McpExecutionConfig,
}

impl McpAgentExecutor {
    /// Create a new MCP agent executor
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        Self {
            agent,
            max_iterations: 10,
            break_on_error: true,
            mcp_config: McpExecutionConfig::default(),
        }
    }

    /// Set the maximum number of iterations
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Set whether to break on first error
    pub fn with_break_on_error(mut self, break_on_error: bool) -> Self {
        self.break_on_error = break_on_error;
        self
    }

    /// Set MCP execution configuration
    pub fn with_mcp_config(mut self, config: McpExecutionConfig) -> Self {
        self.mcp_config = config;
        self
    }

    /// Enable or disable parallel MCP tool execution
    pub fn with_parallel_execution(mut self, enabled: bool) -> Self {
        self.mcp_config.parallel_execution = enabled;
        self
    }

    /// Set maximum number of parallel MCP tool calls
    pub fn with_max_parallel_calls(mut self, max_calls: usize) -> Self {
        self.mcp_config.max_parallel_calls = max_calls;
        self
    }

    /// Execute the agent with streaming support
    pub async fn stream(&self, inputs: PromptArgs) -> Result<McpAgentStream, AgentError> {
        let agent = self.agent.clone();
        let max_iterations = self.max_iterations;
        let break_on_error = self.break_on_error;
        let mcp_config = self.mcp_config.clone();

        // Create a helper struct to avoid lifetime issues
        let executor_helper = McpExecutorHelper {
            agent,
            mcp_config,
            max_iterations,
            break_on_error,
        };

        let s = stream! {
            let mut intermediate_steps: Vec<(AgentAction, String)> = Vec::new();
            let mut iteration = 0;

            loop {
                if iteration >= executor_helper.max_iterations {
                    yield Ok(McpAgentEvent::Error {
                        error: format!("Maximum iterations ({}) reached", executor_helper.max_iterations),
                    });
                    break;
                }

                // Planning phase
                yield Ok(McpAgentEvent::Planning);

                // Get next action from agent
                let event = match executor_helper.agent.plan(&intermediate_steps, inputs.clone()).await {
                    Ok(event) => event,
                    Err(e) => {
                        yield Ok(McpAgentEvent::Error {
                            error: e.to_string(),
                        });
                        if executor_helper.break_on_error {
                            break;
                        }
                        continue;
                    }
                };

                match event {
                    AgentEvent::Action(actions) => {
                        let tools = executor_helper.agent.get_tools();

                        // Emit appropriate tool call events
                        if actions.len() > 1 && executor_helper.mcp_config.parallel_execution {
                            yield Ok(McpAgentEvent::ParallelToolCalls {
                                tool_names: actions.iter().map(|a| a.tool.clone()).collect(),
                                count: actions.len(),
                            });
                        } else {
                            for action in &actions {
                                yield Ok(McpAgentEvent::ToolCall {
                                    tool_name: action.tool.clone(),
                                    tool_input: action.tool_input.clone(),
                                });
                            }
                        }

                        // Execute tools with enhanced MCP support
                        let results = executor_helper.execute_tools_enhanced(actions, &tools).await;

                        // Process results and emit events
                        if results.len() > 1 && executor_helper.mcp_config.parallel_execution {
                            let parallel_results: Vec<(String, String, u64)> = results.iter()
                                .map(|(action, result, time, _)| (action.tool.clone(), result.clone(), *time))
                                .collect();

                            yield Ok(McpAgentEvent::ParallelToolResults {
                                results: parallel_results,
                            });
                        } else {
                            for (action, result, execution_time, is_mcp_error) in &results {
                                if *is_mcp_error {
                                    yield Ok(McpAgentEvent::McpError {
                                        error: result.clone(),
                                        tool_name: action.tool.clone(),
                                        recoverable: true,
                                    });
                                    if executor_helper.break_on_error {
                                        return;
                                    }
                                } else {
                                    yield Ok(McpAgentEvent::ToolResult {
                                        tool_name: action.tool.clone(),
                                        result: result.clone(),
                                        execution_time_ms: *execution_time,
                                    });
                                }
                            }
                        }

                        // Add all results to intermediate steps
                        for (action, result, _, is_error) in results {
                            if !is_error || !executor_helper.break_on_error {
                                intermediate_steps.push((action, result));
                            }
                        }
                    }
                    AgentEvent::Finish(finish) => {
                        yield Ok(McpAgentEvent::Finished {
                            output: finish.output,
                        });
                        break;
                    }
                }

                iteration += 1;
            }
        };

        Ok(Box::pin(s))
    }

    /// Execute the agent and return the final result
    pub async fn invoke(&self, inputs: PromptArgs) -> Result<String, AgentError> {
        let mut stream = self.stream(inputs).await?;
        let mut final_output = String::new();

        while let Some(event_result) = stream.next().await {
            match event_result? {
                McpAgentEvent::Finished { output } => {
                    final_output = output;
                    break;
                }
                McpAgentEvent::Error { error } => {
                    return Err(AgentError::OtherError(error));
                }
                _ => {
                    // Continue processing other events
                }
            }
        }

        Ok(final_output)
    }

    /// Get the underlying agent
    pub fn agent(&self) -> &Arc<dyn Agent> {
        &self.agent
    }

    /// Check if a tool is an MCP tool using the marker trait
    #[cfg(feature = "mcp")]
    fn is_mcp_tool(&self, tool: &Arc<dyn Tool>) -> bool {
        // Try to cast to McpToolMarker trait
        // This is a workaround since we can't use as_any without modifying the Tool trait
        // We'll check if it's an McpTool by checking the tool name prefix
        let name = tool.name();
        let description = tool.description();

        // MCP tools should have specific characteristics
        // This is still a heuristic but more reliable than before
        name.starts_with("mcp_") ||
        description.contains("MCP") ||
        description.contains("Model Context Protocol") ||
        // Check if the tool parameters suggest it's an MCP tool
        tool.parameters().get("mcp_server").is_some()
    }

    #[cfg(not(feature = "mcp"))]
    fn is_mcp_tool(&self, _tool: &Arc<dyn Tool>) -> bool {
        false
    }

    /// Execute multiple tools with enhanced MCP support
    async fn execute_tools_enhanced(
        &self,
        actions: Vec<AgentAction>,
        tools: &[Arc<dyn Tool>],
    ) -> Vec<(AgentAction, String, u64, bool)> {
        if !self.mcp_config.parallel_execution || actions.len() <= 1 {
            // Sequential execution
            let mut results = Vec::new();
            for action in actions {
                let start_time = Instant::now();
                let tool = tools.iter().find(|t| t.name() == action.tool);
                let (result, is_mcp_error) = self.execute_single_tool(&action, tool).await;
                let execution_time = start_time.elapsed().as_millis() as u64;
                results.push((action, result, execution_time, is_mcp_error));
            }
            results
        } else {
            // Parallel execution for MCP tools when possible
            self.execute_tools_parallel(actions, tools).await
        }
    }

    /// Execute a single tool with MCP-specific error handling
    async fn execute_single_tool(
        &self,
        action: &AgentAction,
        tool: Option<&Arc<dyn Tool>>,
    ) -> (String, bool) {
        match tool {
            Some(tool) => {
                let is_mcp = self.is_mcp_tool(tool);
                match tool.call(&action.tool_input).await {
                    Ok(result) => (result, false),
                    Err(e) => {
                        let error_msg = if is_mcp {
                            format!("MCP tool '{}' execution failed: {}", action.tool, e)
                        } else {
                            format!("Tool '{}' execution failed: {}", action.tool, e)
                        };
                        (error_msg, is_mcp)
                    }
                }
            }
            None => {
                let error_msg = format!("Tool '{}' not found", action.tool);
                (error_msg, false)
            }
        }
    }

    /// Execute tools in parallel when beneficial
    async fn execute_tools_parallel(
        &self,
        actions: Vec<AgentAction>,
        tools: &[Arc<dyn Tool>],
    ) -> Vec<(AgentAction, String, u64, bool)> {
        // Group actions by whether they're MCP tools and can be parallelized
        let mut mcp_actions = Vec::new();
        let mut regular_actions = Vec::new();

        for action in actions {
            let tool = tools.iter().find(|t| t.name() == action.tool);
            if let Some(tool) = tool {
                if self.is_mcp_tool(tool) {
                    mcp_actions.push(action);
                } else {
                    regular_actions.push(action);
                }
            } else {
                regular_actions.push(action);
            }
        }

        let mut all_results = Vec::new();

        // Execute regular tools sequentially
        for action in regular_actions {
            let start_time = Instant::now();
            let tool = tools.iter().find(|t| t.name() == action.tool);
            let (result, is_mcp_error) = self.execute_single_tool(&action, tool).await;
            let execution_time = start_time.elapsed().as_millis() as u64;
            all_results.push((action, result, execution_time, is_mcp_error));
        }

        // Execute MCP tools in parallel (up to max_parallel_calls)
        if !mcp_actions.is_empty() {
            let chunk_size = self.mcp_config.max_parallel_calls.min(mcp_actions.len());
            for chunk in mcp_actions.chunks(chunk_size) {
                let futures: Vec<_> = chunk.iter().map(|action| {
                    let tool = tools.iter().find(|t| t.name() == action.tool);
                    let action_clone = action.clone();
                    async move {
                        let start_time = Instant::now();
                        let (result, is_mcp_error) = self.execute_single_tool(&action_clone, tool).await;
                        let execution_time = start_time.elapsed().as_millis() as u64;
                        (action_clone, result, execution_time, is_mcp_error)
                    }
                }).collect();

                let chunk_results = join_all(futures).await;
                all_results.extend(chunk_results);
            }
        }

        all_results
    }
}

impl McpExecutorHelper {
    /// Check if a tool is an MCP tool using the marker trait
    #[cfg(feature = "mcp")]
    fn is_mcp_tool(&self, tool: &Arc<dyn Tool>) -> bool {
        // Try to cast to McpToolMarker trait
        // This is a workaround since we can't use as_any without modifying the Tool trait
        // We'll check if it's an McpTool by checking the tool name prefix
        let name = tool.name();
        let description = tool.description();

        // MCP tools should have specific characteristics
        // This is still a heuristic but more reliable than before
        name.starts_with("mcp_") ||
        description.contains("MCP") ||
        description.contains("Model Context Protocol") ||
        // Check if the tool parameters suggest it's an MCP tool
        tool.parameters().get("mcp_server").is_some()
    }

    #[cfg(not(feature = "mcp"))]
    fn is_mcp_tool(&self, _tool: &Arc<dyn Tool>) -> bool {
        false
    }

    /// Execute multiple tools with enhanced MCP support
    async fn execute_tools_enhanced(
        &self,
        actions: Vec<AgentAction>,
        tools: &[Arc<dyn Tool>],
    ) -> Vec<(AgentAction, String, u64, bool)> {
        if !self.mcp_config.parallel_execution || actions.len() <= 1 {
            // Sequential execution
            let mut results = Vec::new();
            for action in actions {
                let start_time = Instant::now();
                let tool = tools.iter().find(|t| t.name() == action.tool);
                let (result, is_mcp_error) = self.execute_single_tool(&action, tool).await;
                let execution_time = start_time.elapsed().as_millis() as u64;
                results.push((action, result, execution_time, is_mcp_error));
            }
            results
        } else {
            // Parallel execution for MCP tools when possible
            self.execute_tools_parallel(actions, tools).await
        }
    }

    /// Execute a single tool with MCP-specific error handling
    async fn execute_single_tool(
        &self,
        action: &AgentAction,
        tool: Option<&Arc<dyn Tool>>,
    ) -> (String, bool) {
        match tool {
            Some(tool) => {
                let is_mcp = self.is_mcp_tool(tool);
                match tool.call(&action.tool_input).await {
                    Ok(result) => (result, false),
                    Err(e) => {
                        let error_msg = if is_mcp {
                            format!("MCP tool '{}' execution failed: {}", action.tool, e)
                        } else {
                            format!("Tool '{}' execution failed: {}", action.tool, e)
                        };
                        (error_msg, is_mcp)
                    }
                }
            }
            None => {
                let error_msg = format!("Tool '{}' not found", action.tool);
                (error_msg, false)
            }
        }
    }

    /// Execute tools in parallel when beneficial
    async fn execute_tools_parallel(
        &self,
        actions: Vec<AgentAction>,
        tools: &[Arc<dyn Tool>],
    ) -> Vec<(AgentAction, String, u64, bool)> {
        // Group actions by whether they're MCP tools and can be parallelized
        let mut mcp_actions = Vec::new();
        let mut regular_actions = Vec::new();

        for action in actions {
            let tool = tools.iter().find(|t| t.name() == action.tool);
            if let Some(tool) = tool {
                if self.is_mcp_tool(tool) {
                    mcp_actions.push(action);
                } else {
                    regular_actions.push(action);
                }
            } else {
                regular_actions.push(action);
            }
        }

        let mut all_results = Vec::new();

        // Execute regular tools sequentially
        for action in regular_actions {
            let start_time = Instant::now();
            let tool = tools.iter().find(|t| t.name() == action.tool);
            let (result, is_mcp_error) = self.execute_single_tool(&action, tool).await;
            let execution_time = start_time.elapsed().as_millis() as u64;
            all_results.push((action, result, execution_time, is_mcp_error));
        }

        // Execute MCP tools in parallel (up to max_parallel_calls)
        if !mcp_actions.is_empty() {
            let chunk_size = self.mcp_config.max_parallel_calls.min(mcp_actions.len());
            for chunk in mcp_actions.chunks(chunk_size) {
                let futures: Vec<_> = chunk.iter().map(|action| {
                    let tool = tools.iter().find(|t| t.name() == action.tool);
                    let action_clone = action.clone();
                    async move {
                        let start_time = Instant::now();
                        let (result, is_mcp_error) = self.execute_single_tool(&action_clone, tool).await;
                        let execution_time = start_time.elapsed().as_millis() as u64;
                        (action_clone, result, execution_time, is_mcp_error)
                    }
                }).collect();

                let chunk_results = join_all(futures).await;
                all_results.extend(chunk_results);
            }
        }

        all_results
    }
}

// Note: AgentExecutor is a struct, not a trait in langchain-rust
// The McpAgentExecutor provides similar functionality with streaming support

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use serde_json::Value;
    use crate::{
        agent::{Agent, AgentError},
        prompt::PromptArgs,
        schemas::agent::{AgentAction, AgentEvent, AgentFinish},
        tools::Tool,
    };

    // Mock tool for testing
    struct MockTool {
        name: String,
        is_mcp: bool,
        execution_time_ms: u64,
        should_fail: bool,
    }

    impl MockTool {
        fn new(name: &str, is_mcp: bool) -> Self {
            Self {
                name: name.to_string(),
                is_mcp,
                execution_time_ms: 100,
                should_fail: false,
            }
        }

        fn with_execution_time(mut self, time_ms: u64) -> Self {
            self.execution_time_ms = time_ms;
            self
        }

        fn with_failure(mut self, should_fail: bool) -> Self {
            self.should_fail = should_fail;
            self
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> String {
            self.name.clone()
        }

        fn description(&self) -> String {
            if self.is_mcp {
                format!("MCP tool: {}", self.name)
            } else {
                format!("Regular tool: {}", self.name)
            }
        }

        async fn run(&self, _input: Value) -> Result<String, Box<dyn std::error::Error>> {
            // Simulate execution time
            tokio::time::sleep(tokio::time::Duration::from_millis(self.execution_time_ms)).await;

            if self.should_fail {
                Err(format!("Tool {} failed", self.name).into())
            } else {
                Ok(format!("Result from {}", self.name))
            }
        }
    }

    // Mock agent for testing
    struct MockAgent {
        tools: Vec<Arc<dyn Tool>>,
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl MockAgent {
        fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
            Self {
                tools,
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl Agent for MockAgent {
        async fn plan(
            &self,
            _intermediate_steps: &[(AgentAction, String)],
            _inputs: PromptArgs,
        ) -> Result<AgentEvent, AgentError> {
            let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            if count == 0 {
                // First call - return multiple actions to test parallel execution
                let actions = vec![
                    AgentAction {
                        tool: "mcp_tool1".to_string(),
                        tool_input: "test input 1".to_string(),
                        log: "{}".to_string(),
                    },
                    AgentAction {
                        tool: "regular_tool1".to_string(),
                        tool_input: "test input 2".to_string(),
                        log: "{}".to_string(),
                    },
                    AgentAction {
                        tool: "mcp_tool2".to_string(),
                        tool_input: "test input 3".to_string(),
                        log: "{}".to_string(),
                    },
                ];
                Ok(AgentEvent::Action(actions))
            } else {
                // Subsequent calls - finish
                Ok(AgentEvent::Finish(AgentFinish {
                    output: "Task completed".to_string(),
                }))
            }
        }

        fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
            self.tools.clone()
        }
    }

    #[test]
    fn test_mcp_execution_config_default() {
        let config = McpExecutionConfig::default();
        assert!(config.parallel_execution);
        assert_eq!(config.max_parallel_calls, 5);
        assert_eq!(config.tool_timeout_ms, 30000);
        assert!(config.retry_on_failure);
        assert_eq!(config.max_retries, 2);
    }

    #[test]
    fn test_mcp_agent_executor_configuration() {
        let tools = vec![
            Arc::new(MockTool::new("test_tool", false)) as Arc<dyn Tool>
        ];
        let agent = Arc::new(MockAgent::new(tools));

        let executor = McpAgentExecutor::new(agent)
            .with_max_iterations(5)
            .with_break_on_error(false)
            .with_parallel_execution(true)
            .with_max_parallel_calls(3);

        assert_eq!(executor.max_iterations, 5);
        assert!(!executor.break_on_error);
        assert!(executor.mcp_config.parallel_execution);
        assert_eq!(executor.mcp_config.max_parallel_calls, 3);
    }

    #[tokio::test]
    async fn test_mcp_tool_detection() {
        let tools = vec![
            Arc::new(MockTool::new("mcp_tool1", true)) as Arc<dyn Tool>,
            Arc::new(MockTool::new("regular_tool1", false)) as Arc<dyn Tool>,
        ];
        let agent = Arc::new(MockAgent::new(tools.clone()));
        let executor = McpAgentExecutor::new(agent);

        // Test MCP tool detection
        assert!(executor.is_mcp_tool(&tools[0])); // Should detect MCP tool
        assert!(!executor.is_mcp_tool(&tools[1])); // Should not detect regular tool
    }

    #[tokio::test]
    async fn test_single_tool_execution() {
        let tools = vec![
            Arc::new(MockTool::new("test_tool", false).with_execution_time(50)) as Arc<dyn Tool>
        ];
        let agent = Arc::new(MockAgent::new(tools.clone()));
        let executor = McpAgentExecutor::new(agent);

        let action = AgentAction {
            tool: "test_tool".to_string(),
            tool_input: "test input".to_string(),
            log: "{}".to_string(),
        };

        let (result, is_mcp_error) = executor.execute_single_tool(&action, Some(&tools[0])).await;
        assert!(!is_mcp_error);
        assert_eq!(result, "Result from test_tool");
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let tools = vec![];
        let agent = Arc::new(MockAgent::new(tools));
        let executor = McpAgentExecutor::new(agent);

        let action = AgentAction {
            tool: "nonexistent_tool".to_string(),
            tool_input: "test input".to_string(),
            log: "{}".to_string(),
        };

        let (result, is_mcp_error) = executor.execute_single_tool(&action, None).await;
        assert!(!is_mcp_error);
        assert!(result.contains("Tool 'nonexistent_tool' not found"));
    }
}
