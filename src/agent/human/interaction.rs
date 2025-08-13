use std::collections::HashMap;
use std::io::{self, Write};
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::timeout;

use crate::agent::AgentError;

use super::config::HumanAgentConfig;

/// Result of human interaction
#[derive(Debug, Clone)]
pub struct HumanInteractionResult {
    /// The human's response
    pub response: String,
    /// Whether the interaction was successful
    pub success: bool,
    /// Whether termination was triggered
    pub terminated: bool,
    /// Error message if interaction failed
    pub error: Option<String>,
    /// Time taken for the interaction in milliseconds
    pub interaction_time_ms: u64,
}

/// Context for human interaction
#[derive(Debug, Clone)]
pub struct InteractionContext {
    /// Current input being processed
    pub input: String,
    /// Current output (if any)
    pub output: Option<String>,
    /// Any error that occurred
    pub error: Option<String>,
    /// Additional context fields
    pub additional: HashMap<String, String>,
}

impl InteractionContext {
    /// Create a new interaction context
    pub fn new<S: Into<String>>(input: S) -> Self {
        Self {
            input: input.into(),
            output: None,
            error: None,
            additional: HashMap::new(),
        }
    }

    /// Set the output
    pub fn with_output<S: Into<String>>(mut self, output: S) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Set an error
    pub fn with_error<S: Into<String>>(mut self, error: S) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Add additional context
    pub fn with_additional<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.additional.insert(key.into(), value.into());
        self
    }

    /// Convert to a HashMap for condition checking
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("input".to_string(), self.input.clone());
        
        if let Some(output) = &self.output {
            map.insert("output".to_string(), output.clone());
        }
        
        if let Some(error) = &self.error {
            map.insert("error".to_string(), error.clone());
        }
        
        map.extend(self.additional.clone());
        map
    }
}

/// Trait for human interaction interfaces
#[async_trait::async_trait]
pub trait HumanInteractionInterface: Send + Sync {
    /// Request input from human
    async fn request_input(&self, prompt: &str, context: &InteractionContext) -> Result<String, Box<dyn std::error::Error>>;
    
    /// Display information to human
    async fn display_info(&self, message: &str) -> Result<(), Box<dyn std::error::Error>>;
}

/// Console-based human interaction interface
pub struct ConsoleInterface;

#[async_trait::async_trait]
impl HumanInteractionInterface for ConsoleInterface {
    async fn request_input(&self, prompt: &str, context: &InteractionContext) -> Result<String, Box<dyn std::error::Error>> {
        // Display context information
        println!("\n=== Human Intervention Required ===");
        println!("Input: {}", context.input);
        
        if let Some(output) = &context.output {
            println!("Current Output: {}", output);
        }
        
        if let Some(error) = &context.error {
            println!("Error: {}", error);
        }
        
        if !context.additional.is_empty() {
            println!("Additional Context:");
            for (key, value) in &context.additional {
                println!("  {}: {}", key, value);
            }
        }
        
        println!("==================================");
        print!("{} ", prompt);
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }
    
    async fn display_info(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("[INFO] {}", message);
        Ok(())
    }
}

/// Manager for human interactions
pub struct HumanInteractionManager {
    config: HumanAgentConfig,
    interface: Box<dyn HumanInteractionInterface>,
    intervention_count: u32,
}

impl HumanInteractionManager {
    /// Create a new human interaction manager
    pub fn new(config: HumanAgentConfig, interface: Box<dyn HumanInteractionInterface>) -> Self {
        Self {
            config,
            interface,
            intervention_count: 0,
        }
    }

    /// Create a manager with console interface
    pub fn with_console(config: HumanAgentConfig) -> Self {
        Self::new(config, Box::new(ConsoleInterface))
    }

    /// Check if intervention is needed based on context
    pub fn should_intervene(&self, context: &InteractionContext) -> bool {
        let context_map = context.to_map();
        
        // Check if max interventions reached
        if let Some(max) = self.config.max_interventions {
            if self.intervention_count >= max {
                return false;
            }
        }
        
        // Check intervention conditions
        self.config.intervention_conditions.iter().any(|condition| {
            condition.matches(&context_map)
        })
    }

    /// Check if termination is triggered based on context
    pub fn should_terminate(&self, context: &InteractionContext) -> bool {
        let context_map = context.to_map();
        
        // Check termination conditions
        self.config.termination_conditions.iter().any(|condition| {
            condition.matches(&context_map)
        })
    }

    /// Request human input
    pub async fn request_human_input(
        &mut self,
        context: &InteractionContext,
        custom_prompt: Option<&str>,
    ) -> Result<HumanInteractionResult, AgentError> {
        let start_time = std::time::Instant::now();

        // Check if we've exceeded max interventions
        if let Some(max) = self.config.max_interventions {
            if self.intervention_count >= max {
                return Ok(HumanInteractionResult {
                    response: String::new(),
                    success: false,
                    terminated: false,
                    error: Some("Maximum interventions exceeded".to_string()),
                    interaction_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        }

        // Check for termination first
        if self.should_terminate(context) {
            return Ok(HumanInteractionResult {
                response: "Termination condition met".to_string(),
                success: true,
                terminated: true,
                error: None,
                interaction_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Prepare prompt
        let prompt = custom_prompt
            .or(self.config.default_prompt.as_deref())
            .unwrap_or("Please provide your input:");

        // Request input with timeout
        let input_future = self.interface.request_input(prompt, context);
        
        let response = if let Some(timeout_secs) = self.config.input_timeout {
            match timeout(Duration::from_secs(timeout_secs), input_future).await {
                Ok(Ok(response)) => response,
                Ok(Err(e)) => {
                    return Ok(HumanInteractionResult {
                        response: String::new(),
                        success: false,
                        terminated: false,
                        error: Some(format!("Input error: {}", e)),
                        interaction_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
                Err(_) => {
                    return Ok(HumanInteractionResult {
                        response: String::new(),
                        success: false,
                        terminated: false,
                        error: Some("Input timeout".to_string()),
                        interaction_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            }
        } else {
            match input_future.await {
                Ok(response) => response,
                Err(e) => {
                    return Ok(HumanInteractionResult {
                        response: String::new(),
                        success: false,
                        terminated: false,
                        error: Some(format!("Input error: {}", e)),
                        interaction_time_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            }
        };

        // Check if empty response is allowed
        if response.is_empty() && !self.config.allow_empty_response {
            return Ok(HumanInteractionResult {
                response: String::new(),
                success: false,
                terminated: false,
                error: Some("Empty response not allowed".to_string()),
                interaction_time_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Increment intervention count
        self.intervention_count += 1;

        Ok(HumanInteractionResult {
            response,
            success: true,
            terminated: false,
            error: None,
            interaction_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Display information to human
    pub async fn display_info(&self, message: &str) -> Result<(), AgentError> {
        self.interface
            .display_info(message)
            .await
            .map_err(|e| AgentError::OtherError(format!("Display error: {}", e)))
    }

    /// Get current intervention count
    pub fn intervention_count(&self) -> u32 {
        self.intervention_count
    }

    /// Reset intervention count
    pub fn reset_intervention_count(&mut self) {
        self.intervention_count = 0;
    }
}
