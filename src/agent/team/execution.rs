use std::collections::HashMap;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::time::timeout;
use futures::future::try_join_all;

use crate::{
    agent::AgentError,
    prompt::PromptArgs,
    schemas::agent::{AgentAction, AgentEvent},
};

use super::config::{ChildAgentConfig, ExecutionPattern, ExecutionStep, TeamAgentConfig};

/// Result of executing a child agent
#[derive(Debug, Clone)]
pub struct ChildAgentResult {
    /// ID of the agent that produced this result
    pub agent_id: String,
    /// The result output from the agent
    pub output: String,
    /// Whether the execution was successful
    pub success: bool,
    /// Error message if execution failed
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Aggregated results from team execution
#[derive(Debug, Clone)]
pub struct TeamExecutionResult {
    /// Results from individual child agents
    pub child_results: Vec<ChildAgentResult>,
    /// Final aggregated output
    pub final_output: String,
    /// Whether the overall execution was successful
    pub success: bool,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
}

/// Executor for team agent execution patterns
pub struct TeamExecutor {
    config: TeamAgentConfig,
}

impl TeamExecutor {
    /// Create a new team executor
    pub fn new(config: TeamAgentConfig) -> Result<Self, AgentError> {
        config.validate().map_err(|e| AgentError::OtherError(e))?;
        Ok(Self { config })
    }

    /// Execute the team according to the configured pattern
    pub async fn execute(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<TeamExecutionResult, AgentError> {
        let start_time = std::time::Instant::now();

        let result = match &self.config.execution_pattern {
            ExecutionPattern::Concurrent => {
                self.execute_concurrent(intermediate_steps, inputs).await
            }
            ExecutionPattern::Sequential => {
                self.execute_sequential(intermediate_steps, inputs).await
            }
            ExecutionPattern::Hybrid(steps) => {
                self.execute_hybrid(steps, intermediate_steps, inputs).await
            }
        };

        let total_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(mut team_result) => {
                team_result.total_execution_time_ms = total_time;
                Ok(team_result)
            }
            Err(e) => Err(e),
        }
    }

    /// Execute all child agents concurrently
    async fn execute_concurrent(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<TeamExecutionResult, AgentError> {
        let futures: Vec<_> = self
            .config
            .child_agents
            .iter()
            .map(|child| self.execute_child_agent(child, intermediate_steps, inputs.clone()))
            .collect();

        let results = if let Some(global_timeout) = self.config.global_timeout {
            timeout(Duration::from_secs(global_timeout), try_join_all(futures))
                .await
                .map_err(|_| AgentError::OtherError("Global timeout exceeded".to_string()))?
                .map_err(|e| e)?
        } else {
            try_join_all(futures).await?
        };

        self.aggregate_results(results)
    }

    /// Execute child agents sequentially
    async fn execute_sequential(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<TeamExecutionResult, AgentError> {
        let mut results = Vec::new();
        let mut current_input = inputs;

        for child in &self.config.child_agents {
            let result = self
                .execute_child_agent(child, intermediate_steps, current_input.clone())
                .await?;

            // Update input for next agent with previous agent's output
            current_input.insert("previous_agent_output".to_string(), json!(result.output));
            current_input.insert(
                "previous_agent_id".to_string(),
                json!(result.agent_id.clone()),
            );

            results.push(result);

            // Break on error if configured
            if self.config.break_on_error && !results.last().unwrap().success {
                break;
            }
        }

        self.aggregate_results(results)
    }

    /// Execute child agents according to hybrid pattern
    async fn execute_hybrid(
        &self,
        steps: &[ExecutionStep],
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<TeamExecutionResult, AgentError> {
        let mut all_results = Vec::new();
        let mut step_outputs: HashMap<usize, Vec<ChildAgentResult>> = HashMap::new();

        for (step_idx, step) in steps.iter().enumerate() {
            // Prepare input for this step based on dependencies
            let mut step_input = inputs.clone();
            
            // Add outputs from dependent steps
            for &dep_idx in &step.dependencies {
                if let Some(dep_results) = step_outputs.get(&dep_idx) {
                    let dep_outputs: Vec<Value> = dep_results
                        .iter()
                        .map(|r| json!({"agent_id": r.agent_id, "output": r.output}))
                        .collect();
                    step_input.insert(
                        format!("step_{}_outputs", dep_idx),
                        json!(dep_outputs),
                    );
                }
            }

            // Get child agents for this step
            let step_agents: Vec<&ChildAgentConfig> = step
                .agent_ids
                .iter()
                .filter_map(|id| self.config.child_agents.iter().find(|c| &c.id == id))
                .collect();

            // Execute agents in this step
            let step_results = if step.concurrent {
                // Execute concurrently
                let futures: Vec<_> = step_agents
                    .iter()
                    .map(|child| {
                        self.execute_child_agent(child, intermediate_steps, step_input.clone())
                    })
                    .collect();

                try_join_all(futures).await?
            } else {
                // Execute sequentially within the step
                let mut results = Vec::new();
                let mut current_input = step_input;

                for child in step_agents {
                    let result = self
                        .execute_child_agent(child, intermediate_steps, current_input.clone())
                        .await?;

                    current_input.insert("previous_agent_output".to_string(), json!(result.output));
                    current_input.insert(
                        "previous_agent_id".to_string(),
                        json!(result.agent_id.clone()),
                    );

                    results.push(result);

                    if self.config.break_on_error && !results.last().unwrap().success {
                        break;
                    }
                }

                results
            };

            // Store step results
            step_outputs.insert(step_idx, step_results.clone());
            all_results.extend(step_results);

            // Break on error if configured
            if self.config.break_on_error && all_results.iter().any(|r| !r.success) {
                break;
            }
        }

        self.aggregate_results(all_results)
    }

    /// Execute a single child agent
    async fn execute_child_agent(
        &self,
        child: &ChildAgentConfig,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<ChildAgentResult, AgentError> {
        let start_time = std::time::Instant::now();

        let execution_future = async {
            match child.agent.plan(intermediate_steps, inputs).await {
                Ok(AgentEvent::Finish(finish)) => Ok(ChildAgentResult {
                    agent_id: child.id.clone(),
                    output: finish.output,
                    success: true,
                    error: None,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                }),
                Ok(AgentEvent::Action(_)) => {
                    // For team agents, we expect child agents to return Finish events
                    // Actions would need to be handled by a higher-level executor
                    Err(AgentError::OtherError(
                        "Child agent returned Action instead of Finish".to_string(),
                    ))
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if child.critical {
                        Err(e)
                    } else {
                        Ok(ChildAgentResult {
                            agent_id: child.id.clone(),
                            output: format!("Error: {}", error_msg),
                            success: false,
                            error: Some(error_msg),
                            execution_time_ms: start_time.elapsed().as_millis() as u64,
                        })
                    }
                }
            }
        };

        // Apply timeout if configured
        if let Some(timeout_secs) = child.timeout {
            timeout(Duration::from_secs(timeout_secs), execution_future)
                .await
                .map_err(|_| {
                    AgentError::OtherError(format!(
                        "Agent {} timed out after {} seconds",
                        child.id, timeout_secs
                    ))
                })?
        } else {
            execution_future.await
        }
    }

    /// Aggregate results from child agents
    fn aggregate_results(
        &self,
        results: Vec<ChildAgentResult>,
    ) -> Result<TeamExecutionResult, AgentError> {
        let success = results.iter().all(|r| r.success);

        // Create aggregated output
        let final_output = if success {
            // Combine all successful outputs
            let outputs: Vec<String> = results
                .iter()
                .map(|r| format!("{}: {}", r.agent_id, r.output))
                .collect();
            outputs.join("\n\n")
        } else {
            // Include error information
            let outputs: Vec<String> = results
                .iter()
                .map(|r| {
                    if r.success {
                        format!("{}: {}", r.agent_id, r.output)
                    } else {
                        format!(
                            "{}: ERROR - {}",
                            r.agent_id,
                            r.error.as_ref().unwrap_or(&"Unknown error".to_string())
                        )
                    }
                })
                .collect();
            outputs.join("\n\n")
        };

        Ok(TeamExecutionResult {
            child_results: results,
            final_output,
            success,
            total_execution_time_ms: 0, // Will be set by caller
        })
    }
}
