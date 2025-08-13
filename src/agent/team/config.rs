use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::agent::Agent;

/// Execution pattern for team agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionPattern {
    /// All agents execute concurrently and results are aggregated
    Concurrent,
    /// Agents execute in sequence, each receiving the previous agent's output
    Sequential,
    /// Complex dependency chains with concurrent and sequential execution
    Hybrid(Vec<ExecutionStep>),
}

/// Represents a step in hybrid execution pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Agent IDs that should execute in this step
    pub agent_ids: Vec<String>,
    /// Whether agents in this step should execute concurrently
    pub concurrent: bool,
    /// Dependencies on previous steps (by step index)
    pub dependencies: Vec<usize>,
}

/// Configuration for a child agent in a team
#[derive(Clone)]
pub struct ChildAgentConfig {
    /// Unique identifier for this agent within the team
    pub id: String,
    /// The actual agent instance
    pub agent: Arc<dyn Agent>,
    /// Optional timeout for this agent's execution (in seconds)
    pub timeout: Option<u64>,
    /// Whether this agent's failure should stop the entire team execution
    pub critical: bool,
    /// Whether this is a nested team agent
    pub is_team_agent: bool,
}

/// Configuration for team agent behavior
#[derive(Clone)]
pub struct TeamAgentConfig {
    /// Child agents that make up this team
    pub child_agents: Vec<ChildAgentConfig>,
    /// Execution pattern for the team
    pub execution_pattern: ExecutionPattern,
    /// Maximum number of iterations for the team agent
    pub max_iterations: Option<i32>,
    /// Whether to break on first error
    pub break_on_error: bool,
    /// Global timeout for the entire team execution (in seconds)
    pub global_timeout: Option<u64>,
    /// System prompt/prefix for the team agent
    pub prefix: Option<String>,
}

impl Default for TeamAgentConfig {
    fn default() -> Self {
        Self {
            child_agents: Vec::new(),
            execution_pattern: ExecutionPattern::Sequential,
            max_iterations: Some(10),
            break_on_error: true,
            global_timeout: Some(300), // 5 minutes default
            prefix: None,
        }
    }
}

impl TeamAgentConfig {
    /// Create a new team agent configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a child agent to the team
    pub fn add_child_agent(mut self, config: ChildAgentConfig) -> Self {
        self.child_agents.push(config);
        self
    }

    /// Set the execution pattern
    pub fn with_execution_pattern(mut self, pattern: ExecutionPattern) -> Self {
        self.execution_pattern = pattern;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max_iterations: i32) -> Self {
        self.max_iterations = Some(max_iterations);
        self
    }

    /// Set break on error behavior
    pub fn with_break_on_error(mut self, break_on_error: bool) -> Self {
        self.break_on_error = break_on_error;
        self
    }

    /// Set global timeout
    pub fn with_global_timeout(mut self, timeout_seconds: u64) -> Self {
        self.global_timeout = Some(timeout_seconds);
        self
    }

    /// Set system prompt/prefix
    pub fn with_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.child_agents.is_empty() {
            return Err("Team agent must have at least one child agent".to_string());
        }

        // Check for duplicate agent IDs
        let mut ids = std::collections::HashSet::new();
        for child in &self.child_agents {
            if !ids.insert(&child.id) {
                return Err(format!("Duplicate agent ID: {}", child.id));
            }
        }

        // Validate hybrid execution pattern
        if let ExecutionPattern::Hybrid(steps) = &self.execution_pattern {
            for (step_idx, step) in steps.iter().enumerate() {
                // Check that all agent IDs in steps exist
                for agent_id in &step.agent_ids {
                    if !self.child_agents.iter().any(|c| &c.id == agent_id) {
                        return Err(format!("Unknown agent ID in execution step: {}", agent_id));
                    }
                }

                // Check that dependencies are valid
                for &dep in &step.dependencies {
                    if dep >= step_idx {
                        return Err(format!("Invalid dependency: step {} cannot depend on step {} (must be earlier)", step_idx, dep));
                    }
                }
            }

            // Check that all agents are included in at least one step
            let mut covered_agents = std::collections::HashSet::new();
            for step in steps {
                for agent_id in &step.agent_ids {
                    covered_agents.insert(agent_id);
                }
            }

            for child in &self.child_agents {
                if !covered_agents.contains(&child.id) {
                    return Err(format!("Agent {} is not included in any execution step", child.id));
                }
            }
        }

        Ok(())
    }
}

impl ChildAgentConfig {
    /// Create a new child agent configuration
    pub fn new<S: Into<String>>(id: S, agent: Arc<dyn Agent>) -> Self {
        Self {
            id: id.into(),
            agent,
            timeout: None,
            critical: true,
            is_team_agent: false,
        }
    }

    /// Create a new team agent configuration
    pub fn new_team_agent<S: Into<String>>(id: S, agent: Arc<dyn Agent>) -> Self {
        Self {
            id: id.into(),
            agent,
            timeout: None,
            critical: true,
            is_team_agent: true,
        }
    }

    /// Set timeout for this agent
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout = Some(timeout_seconds);
        self
    }

    /// Set whether this agent is critical
    pub fn with_critical(mut self, critical: bool) -> Self {
        self.critical = critical;
        self
    }

    /// Set whether this is a team agent
    pub fn with_team_agent(mut self, is_team_agent: bool) -> Self {
        self.is_team_agent = is_team_agent;
        self
    }
}
