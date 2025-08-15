//! Error recovery mechanisms for agent output parsing

use std::collections::HashMap;
use regex::Regex;
use serde_json::Value;
use crate::agent::AgentError;

/// Recovery strategy configuration
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum number of recovery attempts
    pub max_attempts: usize,
    /// Enable progressive recovery strategies
    pub enable_progressive: bool,
    /// Enable fallback to default values
    pub enable_fallbacks: bool,
    /// Custom recovery patterns
    pub custom_patterns: HashMap<String, String>,
    /// Confidence threshold for accepting recovered output
    pub confidence_threshold: f64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            enable_progressive: true,
            enable_fallbacks: true,
            custom_patterns: HashMap::new(),
            confidence_threshold: 0.7,
        }
    }
}

/// Recovery result with confidence scoring
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Recovered text
    pub recovered_text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Strategy used for recovery
    pub strategy_used: RecoveryStrategy,
    /// Whether the recovery was successful
    pub success: bool,
    /// Additional metadata about the recovery
    pub metadata: HashMap<String, String>,
}

/// Available recovery strategies
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryStrategy {
    /// Direct JSON repair
    JsonRepair,
    /// Template-based reconstruction
    TemplateReconstruction,
    /// Pattern matching and extraction
    PatternExtraction,
    /// Semantic analysis and repair
    SemanticRepair,
    /// Fallback to default values
    FallbackDefaults,
    /// Custom recovery logic
    Custom(String),
}

/// Error recovery engine
pub struct ErrorRecoveryEngine {
    config: RecoveryConfig,
    strategies: Vec<Box<dyn RecoveryStrategyImpl>>,
    pattern_cache: HashMap<String, Regex>,
}

/// Trait for implementing recovery strategies
pub trait RecoveryStrategyImpl: Send + Sync {
    /// Attempt to recover the text using this strategy
    fn recover(&self, text: &str, context: &RecoveryContext) -> Result<RecoveryResult, AgentError>;
    
    /// Get the strategy type
    fn strategy_type(&self) -> RecoveryStrategy;
    
    /// Get the confidence score for this strategy with the given input
    fn confidence_score(&self, text: &str, context: &RecoveryContext) -> f64;
}

/// Context information for recovery strategies
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    /// Expected format type
    pub format_type: String,
    /// Available tools for validation
    pub available_tools: Vec<String>,
    /// Previous recovery attempts
    pub previous_attempts: Vec<RecoveryStrategy>,
    /// Original error that triggered recovery
    pub original_error: String,
    /// Additional context data
    pub context_data: HashMap<String, Value>,
}

impl ErrorRecoveryEngine {
    pub fn new(config: RecoveryConfig) -> Self {
        let mut engine = Self {
            config,
            strategies: Vec::new(),
            pattern_cache: HashMap::new(),
        };
        
        engine.register_default_strategies();
        engine
    }

    /// Register a custom recovery strategy
    pub fn register_strategy(&mut self, strategy: Box<dyn RecoveryStrategyImpl>) {
        self.strategies.push(strategy);
    }

    /// Attempt to recover from a parsing error
    pub fn recover(&mut self, text: &str, context: &RecoveryContext) -> Result<RecoveryResult, AgentError> {
        let mut best_result = None;
        let mut best_confidence = 0.0;

        // Try each strategy in order of confidence
        let mut strategies_by_confidence: Vec<_> = self.strategies.iter()
            .map(|s| (s.confidence_score(text, context), s))
            .collect();
        
        strategies_by_confidence.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (confidence, strategy) in strategies_by_confidence {
            // Skip strategies that have already been tried
            if context.previous_attempts.contains(&strategy.strategy_type()) {
                continue;
            }

            // Skip strategies with very low confidence
            if confidence < 0.1 {
                continue;
            }

            match strategy.recover(text, context) {
                Ok(result) => {
                    if result.success && result.confidence > best_confidence {
                        best_confidence = result.confidence;
                        best_result = Some(result);
                        
                        // If we have a high-confidence result, use it
                        if best_confidence >= self.config.confidence_threshold {
                            break;
                        }
                    }
                }
                Err(_) => {
                    // Strategy failed, continue to next
                    continue;
                }
            }
        }

        best_result.ok_or_else(|| AgentError::OutputParsingError("All recovery strategies failed".to_string()))
    }

    /// Register default recovery strategies
    fn register_default_strategies(&mut self) {
        self.strategies.push(Box::new(JsonRepairStrategy::new()));
        self.strategies.push(Box::new(TemplateReconstructionStrategy::new()));
        self.strategies.push(Box::new(PatternExtractionStrategy::new()));
        self.strategies.push(Box::new(SemanticRepairStrategy::new()));
        
        if self.config.enable_fallbacks {
            self.strategies.push(Box::new(FallbackDefaultsStrategy::new()));
        }
    }
}

/// JSON repair strategy
pub struct JsonRepairStrategy {
    repair_patterns: Vec<(Regex, String)>,
}

impl JsonRepairStrategy {
    pub fn new() -> Self {
        let repair_patterns = vec![
            // Fix placeholder JSON
            (Regex::new(r"\{JSON\}").unwrap(), "{}".to_string()),
            (Regex::new(r"\{json\}").unwrap(), "{}".to_string()),
            (Regex::new(r"\[JSON\]").unwrap(), "{}".to_string()),
            // Fix single quotes
            (Regex::new(r"'([^']*)'").unwrap(), r#""$1""#.to_string()),
            // Fix unquoted keys
            (Regex::new(r"\{(\w+):").unwrap(), r#"{"$1":"#.to_string()),
            // Fix trailing commas
            (Regex::new(r",\s*([}\]])").unwrap(), "$1".to_string()),
        ];

        Self { repair_patterns }
    }
}

impl RecoveryStrategyImpl for JsonRepairStrategy {
    fn recover(&self, text: &str, _context: &RecoveryContext) -> Result<RecoveryResult, AgentError> {
        let mut recovered = text.to_string();
        let mut changes_made = 0;

        for (pattern, replacement) in &self.repair_patterns {
            let before = recovered.clone();
            recovered = pattern.replace_all(&recovered, replacement).to_string();
            if recovered != before {
                changes_made += 1;
            }
        }

        // Try to parse as JSON to validate
        let confidence = if changes_made > 0 {
            match serde_json::from_str::<Value>(&recovered) {
                Ok(_) => 0.9,
                Err(_) => 0.3,
            }
        } else {
            0.1
        };

        Ok(RecoveryResult {
            recovered_text: recovered,
            confidence,
            strategy_used: RecoveryStrategy::JsonRepair,
            success: changes_made > 0,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("changes_made".to_string(), changes_made.to_string());
                meta
            },
        })
    }

    fn strategy_type(&self) -> RecoveryStrategy {
        RecoveryStrategy::JsonRepair
    }

    fn confidence_score(&self, text: &str, _context: &RecoveryContext) -> f64 {
        // Higher confidence if text contains JSON-like patterns
        let json_indicators = ["{", "}", "[", "]", ":", ","];
        let indicator_count = json_indicators.iter()
            .map(|&indicator| text.matches(indicator).count())
            .sum::<usize>();

        (indicator_count as f64 / text.len() as f64).min(1.0)
    }
}

/// Template reconstruction strategy
pub struct TemplateReconstructionStrategy {
    templates: HashMap<String, String>,
}

impl TemplateReconstructionStrategy {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        
        // ReAct template
        templates.insert("react".to_string(), 
            "Thought: {thought}\nAction: {action}\nAction Input: {action_input}".to_string());
        
        // Chat template
        templates.insert("chat".to_string(),
            r#"{"action": "{action}", "action_input": "{action_input}"}"#.to_string());

        Self { templates }
    }
}

impl RecoveryStrategyImpl for TemplateReconstructionStrategy {
    fn recover(&self, text: &str, context: &RecoveryContext) -> Result<RecoveryResult, AgentError> {
        let template = self.templates.get(&context.format_type)
            .ok_or_else(|| AgentError::OutputParsingError("No template for format type".to_string()))?;

        // Extract values from text
        let mut extracted_values = HashMap::new();
        
        // Simple extraction patterns
        let patterns = vec![
            ("thought", r"Thought:\s*(.+?)(?:\n|$)"),
            ("action", r"Action:\s*(.+?)(?:\n|$)"),
            ("action_input", r"Action Input:\s*(.+?)(?:\n|$)"),
        ];

        for (key, pattern) in patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(caps) = regex.captures(text) {
                    if let Some(value) = caps.get(1) {
                        extracted_values.insert(key.to_string(), value.as_str().trim().to_string());
                    }
                }
            }
        }

        // Calculate confidence based on how many values were extracted
        let expected_placeholders = template.matches('{').count();
        let extracted_count = extracted_values.len();

        // Reconstruct using template
        let mut reconstructed = template.clone();
        for (key, value) in &extracted_values {
            reconstructed = reconstructed.replace(&format!("{{{}}}", key), value);
        }
        let confidence = if expected_placeholders > 0 {
            extracted_count as f64 / expected_placeholders as f64
        } else {
            0.0
        };

        Ok(RecoveryResult {
            recovered_text: reconstructed,
            confidence,
            strategy_used: RecoveryStrategy::TemplateReconstruction,
            success: extracted_count > 0,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("extracted_fields".to_string(), extracted_count.to_string());
                meta.insert("expected_fields".to_string(), expected_placeholders.to_string());
                meta
            },
        })
    }

    fn strategy_type(&self) -> RecoveryStrategy {
        RecoveryStrategy::TemplateReconstruction
    }

    fn confidence_score(&self, text: &str, context: &RecoveryContext) -> f64 {
        // Higher confidence if text contains format-specific keywords
        let keywords = match context.format_type.as_str() {
            "react" => vec!["Thought:", "Action:", "Action Input:"],
            "chat" => vec!["action", "action_input"],
            _ => vec![],
        };

        let keyword_count = keywords.iter()
            .map(|&keyword| if text.contains(keyword) { 1 } else { 0 })
            .sum::<usize>();

        if keywords.is_empty() {
            0.0
        } else {
            keyword_count as f64 / keywords.len() as f64
        }
    }
}

/// Pattern extraction strategy
pub struct PatternExtractionStrategy;

impl PatternExtractionStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl RecoveryStrategyImpl for PatternExtractionStrategy {
    fn recover(&self, text: &str, _context: &RecoveryContext) -> Result<RecoveryResult, AgentError> {
        // Try to extract any JSON-like content
        let json_pattern = Regex::new(r"\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}").unwrap();
        
        if let Some(json_match) = json_pattern.find(text) {
            let extracted = json_match.as_str().to_string();
            
            // Validate the extracted JSON
            let confidence = match serde_json::from_str::<Value>(&extracted) {
                Ok(_) => 0.8,
                Err(_) => 0.4,
            };

            return Ok(RecoveryResult {
                recovered_text: extracted,
                confidence,
                strategy_used: RecoveryStrategy::PatternExtraction,
                success: true,
                metadata: HashMap::new(),
            });
        }

        Err(AgentError::OutputParsingError("No extractable patterns found".to_string()))
    }

    fn strategy_type(&self) -> RecoveryStrategy {
        RecoveryStrategy::PatternExtraction
    }

    fn confidence_score(&self, text: &str, _context: &RecoveryContext) -> f64 {
        // Check for extractable patterns
        let patterns = [r"\{.*\}", r"\[.*\]", r#"".*""#];
        
        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(text) {
                    return 0.6;
                }
            }
        }
        
        0.1
    }
}

/// Semantic repair strategy (placeholder)
pub struct SemanticRepairStrategy;

impl SemanticRepairStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl RecoveryStrategyImpl for SemanticRepairStrategy {
    fn recover(&self, _text: &str, _context: &RecoveryContext) -> Result<RecoveryResult, AgentError> {
        // This would implement more sophisticated semantic analysis
        // For now, it's a placeholder
        Err(AgentError::OutputParsingError("Semantic repair not implemented".to_string()))
    }

    fn strategy_type(&self) -> RecoveryStrategy {
        RecoveryStrategy::SemanticRepair
    }

    fn confidence_score(&self, _text: &str, _context: &RecoveryContext) -> f64 {
        0.0 // Not implemented
    }
}

/// Fallback defaults strategy
pub struct FallbackDefaultsStrategy;

impl FallbackDefaultsStrategy {
    pub fn new() -> Self {
        Self
    }
}

impl RecoveryStrategyImpl for FallbackDefaultsStrategy {
    fn recover(&self, _text: &str, context: &RecoveryContext) -> Result<RecoveryResult, AgentError> {
        // Provide sensible defaults based on format type
        let default_output = match context.format_type.as_str() {
            "react" => "Thought: I need to process this request.\nAction: search\nAction Input: {}".to_string(),
            "chat" => r#"{"action": "search", "action_input": "{}"}"#.to_string(),
            _ => "{}".to_string(),
        };

        Ok(RecoveryResult {
            recovered_text: default_output,
            confidence: 0.2, // Low confidence since this is a fallback
            strategy_used: RecoveryStrategy::FallbackDefaults,
            success: true,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("fallback_type".to_string(), context.format_type.clone());
                meta
            },
        })
    }

    fn strategy_type(&self) -> RecoveryStrategy {
        RecoveryStrategy::FallbackDefaults
    }

    fn confidence_score(&self, _text: &str, _context: &RecoveryContext) -> f64 {
        0.1 // Always low confidence as this is a last resort
    }
}
