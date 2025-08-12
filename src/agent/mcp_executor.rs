use std::pin::Pin;
use std::sync::Arc;

use async_stream::stream;
use futures::Stream;
use futures_util::StreamExt;

use crate::{
    agent::{Agent, AgentError},
    prompt::PromptArgs,
    schemas::{
        agent::{AgentAction, AgentEvent},
    },
};

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
    /// Tool execution completed
    ToolResult {
        tool_name: String,
        result: String,
    },
    /// Agent execution completed
    Finished {
        output: String,
    },
    /// Error occurred during execution
    Error {
        error: String,
    },
}

/// Stream type for MCP agent events
pub type McpAgentStream = Pin<Box<dyn Stream<Item = Result<McpAgentEvent, AgentError>> + Send>>;

/// Executor for agents with MCP tool support and streaming capabilities
pub struct McpAgentExecutor {
    /// The underlying agent
    agent: Arc<dyn Agent>,
    /// Maximum number of iterations before stopping
    max_iterations: usize,
    /// Whether to break on first error
    break_on_error: bool,
}

impl McpAgentExecutor {
    /// Create a new MCP agent executor
    pub fn new(agent: Arc<dyn Agent>) -> Self {
        Self {
            agent,
            max_iterations: 10,
            break_on_error: true,
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

    /// Execute the agent with streaming support
    pub async fn stream(&self, inputs: PromptArgs) -> Result<McpAgentStream, AgentError> {
        let agent = self.agent.clone();
        let max_iterations = self.max_iterations;
        let break_on_error = self.break_on_error;

        let s = stream! {
            let mut intermediate_steps: Vec<(AgentAction, String)> = Vec::new();
            let mut iteration = 0;

            loop {
                if iteration >= max_iterations {
                    yield Ok(McpAgentEvent::Error {
                        error: format!("Maximum iterations ({}) reached", max_iterations),
                    });
                    break;
                }

                // Planning phase
                yield Ok(McpAgentEvent::Planning);

                // Get next action from agent
                let event = match agent.plan(&intermediate_steps, inputs.clone()).await {
                    Ok(event) => event,
                    Err(e) => {
                        yield Ok(McpAgentEvent::Error {
                            error: e.to_string(),
                        });
                        if break_on_error {
                            break;
                        }
                        continue;
                    }
                };

                match event {
                    AgentEvent::Action(actions) => {
                        // Execute each action
                        for action in actions {
                            // Emit tool call event
                            yield Ok(McpAgentEvent::ToolCall {
                                tool_name: action.tool.clone(),
                                tool_input: action.tool_input.clone(),
                            });

                            // Find and execute the tool
                            let tools = agent.get_tools();
                            let tool = tools.iter().find(|t| t.name() == action.tool);

                            let result = match tool {
                                Some(tool) => {
                                    // Execute tool and handle errors without keeping the error across await
                                    let tool_result = tool.call(&action.tool_input).await;
                                    match tool_result {
                                        Ok(result) => result,
                                        Err(_) => {
                                            let error_msg = format!("Tool execution failed for tool: {}", action.tool);
                                            yield Ok(McpAgentEvent::Error {
                                                error: error_msg.clone(),
                                            });
                                            if break_on_error {
                                                return;
                                            }
                                            error_msg
                                        }
                                    }
                                }
                                None => {
                                    let error_msg = format!("Tool '{}' not found", action.tool);
                                    yield Ok(McpAgentEvent::Error {
                                        error: error_msg.clone(),
                                    });
                                    if break_on_error {
                                        return;
                                    }
                                    error_msg
                                }
                            };

                            // Emit tool result event
                            yield Ok(McpAgentEvent::ToolResult {
                                tool_name: action.tool.clone(),
                                result: result.clone(),
                            });

                            // Add to intermediate steps
                            intermediate_steps.push((action, result));
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
}

// Note: AgentExecutor is a struct, not a trait in langchain-rust
// The McpAgentExecutor provides similar functionality with streaming support

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_agent_executor_creation() {
        // This test would require a mock agent implementation
        // For now, just test that the struct can be created
        // let agent = Arc::new(MockAgent::new());
        // let executor = McpAgentExecutor::new(agent);
        // assert_eq!(executor.max_iterations, 10);
        // assert!(executor.break_on_error);
    }
}
