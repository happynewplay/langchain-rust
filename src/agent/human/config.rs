use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use crate::schemas::memory::BaseMemory;

/// Condition that triggers human intervention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionCondition {
    /// Pattern to match against (can be regex or simple string)
    pub pattern: String,
    /// Whether to use regex matching (default: false for simple string matching)
    pub use_regex: bool,
    /// Field to check the pattern against (e.g., "input", "output", "error")
    pub field: String,
    /// Optional description of what this condition checks
    pub description: Option<String>,
}

/// Condition that triggers automatic termination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationCondition {
    /// Pattern to match against
    pub pattern: String,
    /// Whether to use regex matching
    pub use_regex: bool,
    /// Field to check the pattern against
    pub field: String,
    /// Similarity threshold for fuzzy matching (0.0 to 1.0)
    pub similarity_threshold: Option<f64>,
    /// Optional description of what this condition checks
    pub description: Option<String>,
}

/// Configuration for human agent behavior
#[derive(Clone)]
pub struct HumanAgentConfig {
    /// Conditions that trigger human intervention
    pub intervention_conditions: Vec<InterventionCondition>,
    /// Conditions that trigger automatic termination
    pub termination_conditions: Vec<TerminationCondition>,
    /// Maximum number of human interventions allowed
    pub max_interventions: Option<u32>,
    /// Timeout for waiting for human input (in seconds)
    pub input_timeout: Option<u64>,
    /// Default prompt to show to human when intervention is triggered
    pub default_prompt: Option<String>,
    /// Whether to allow empty human responses
    pub allow_empty_response: bool,
    /// System prompt/prefix for the human agent
    pub prefix: Option<String>,
    /// Memory for storing conversation history
    pub memory: Option<Arc<Mutex<dyn BaseMemory>>>,
    /// Whether to include memory context in human prompts
    pub include_memory_in_prompts: bool,
}

impl Default for HumanAgentConfig {
    fn default() -> Self {
        Self {
            intervention_conditions: Vec::new(),
            termination_conditions: Vec::new(),
            max_interventions: Some(10),
            input_timeout: Some(300), // 5 minutes default
            default_prompt: Some("Please provide your input:".to_string()),
            allow_empty_response: false,
            prefix: None,
            memory: None,
            include_memory_in_prompts: true,
        }
    }
}

impl HumanAgentConfig {
    /// Create a new human agent configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an intervention condition
    pub fn add_intervention_condition(mut self, condition: InterventionCondition) -> Self {
        self.intervention_conditions.push(condition);
        self
    }

    /// Add a termination condition
    pub fn add_termination_condition(mut self, condition: TerminationCondition) -> Self {
        self.termination_conditions.push(condition);
        self
    }

    /// Set maximum interventions
    pub fn with_max_interventions(mut self, max: u32) -> Self {
        self.max_interventions = Some(max);
        self
    }

    /// Set input timeout
    pub fn with_input_timeout(mut self, timeout_seconds: u64) -> Self {
        self.input_timeout = Some(timeout_seconds);
        self
    }

    /// Set default prompt
    pub fn with_default_prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.default_prompt = Some(prompt.into());
        self
    }

    /// Set whether to allow empty responses
    pub fn with_allow_empty_response(mut self, allow: bool) -> Self {
        self.allow_empty_response = allow;
        self
    }

    /// Set system prompt/prefix
    pub fn with_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set memory for the human agent
    pub fn with_memory(mut self, memory: Arc<Mutex<dyn BaseMemory>>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Set whether to include memory context in human prompts
    pub fn with_include_memory_in_prompts(mut self, include: bool) -> Self {
        self.include_memory_in_prompts = include;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.intervention_conditions.is_empty() {
            return Err("Human agent must have at least one intervention condition".to_string());
        }

        if self.termination_conditions.is_empty() {
            return Err("Human agent must have at least one termination condition".to_string());
        }

        // Validate intervention conditions
        for (idx, condition) in self.intervention_conditions.iter().enumerate() {
            if condition.pattern.is_empty() {
                return Err(format!("Intervention condition {} has empty pattern", idx));
            }
            if condition.field.is_empty() {
                return Err(format!("Intervention condition {} has empty field", idx));
            }
        }

        // Validate termination conditions
        for (idx, condition) in self.termination_conditions.iter().enumerate() {
            if condition.pattern.is_empty() {
                return Err(format!("Termination condition {} has empty pattern", idx));
            }
            if condition.field.is_empty() {
                return Err(format!("Termination condition {} has empty field", idx));
            }
            if let Some(threshold) = condition.similarity_threshold {
                if threshold < 0.0 || threshold > 1.0 {
                    return Err(format!(
                        "Termination condition {} has invalid similarity threshold: {}",
                        idx, threshold
                    ));
                }
            }
        }

        Ok(())
    }
}

impl InterventionCondition {
    /// Create a new intervention condition
    pub fn new<P: Into<String>, F: Into<String>>(pattern: P, field: F) -> Self {
        Self {
            pattern: pattern.into(),
            use_regex: false,
            field: field.into(),
            description: None,
        }
    }

    /// Create a regex-based intervention condition
    pub fn regex<P: Into<String>, F: Into<String>>(pattern: P, field: F) -> Self {
        Self {
            pattern: pattern.into(),
            use_regex: true,
            field: field.into(),
            description: None,
        }
    }

    /// Add a description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Check if this condition matches the given context
    pub fn matches(&self, context: &HashMap<String, String>) -> bool {
        if let Some(value) = context.get(&self.field) {
            if self.use_regex {
                if let Ok(regex) = regex::Regex::new(&self.pattern) {
                    regex.is_match(value)
                } else {
                    false
                }
            } else {
                value.contains(&self.pattern)
            }
        } else {
            false
        }
    }
}

impl TerminationCondition {
    /// Create a new termination condition
    pub fn new<P: Into<String>, F: Into<String>>(pattern: P, field: F) -> Self {
        Self {
            pattern: pattern.into(),
            use_regex: false,
            field: field.into(),
            similarity_threshold: None,
            description: None,
        }
    }

    /// Create a regex-based termination condition
    pub fn regex<P: Into<String>, F: Into<String>>(pattern: P, field: F) -> Self {
        Self {
            pattern: pattern.into(),
            use_regex: true,
            field: field.into(),
            similarity_threshold: None,
            description: None,
        }
    }

    /// Create a similarity-based termination condition
    pub fn similarity<P: Into<String>, F: Into<String>>(pattern: P, field: F, threshold: f64) -> Self {
        Self {
            pattern: pattern.into(),
            use_regex: false,
            field: field.into(),
            similarity_threshold: Some(threshold),
            description: None,
        }
    }

    /// Add a description
    pub fn with_description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Check if this condition matches the given context
    pub fn matches(&self, context: &HashMap<String, String>) -> bool {
        if let Some(value) = context.get(&self.field) {
            if self.use_regex {
                if let Ok(regex) = regex::Regex::new(&self.pattern) {
                    regex.is_match(value)
                } else {
                    false
                }
            } else if let Some(threshold) = self.similarity_threshold {
                // Simple similarity check using Levenshtein distance
                let similarity = self.calculate_similarity(value, &self.pattern);
                similarity >= threshold
            } else {
                value.contains(&self.pattern)
            }
        } else {
            false
        }
    }

    /// Calculate similarity between two strings using a simple metric
    fn calculate_similarity(&self, s1: &str, s2: &str) -> f64 {
        if s1 == s2 {
            return 1.0;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        // Simple similarity based on common substrings
        let s1_lower = s1.to_lowercase();
        let s2_lower = s2.to_lowercase();
        
        let common_chars = s1_lower
            .chars()
            .filter(|c| s2_lower.contains(*c))
            .count();
        
        let max_len = s1.len().max(s2.len());
        common_chars as f64 / max_len as f64
    }
}
