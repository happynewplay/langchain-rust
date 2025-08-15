//! Common interface for all agent output parsers

use async_trait::async_trait;
use std::sync::Arc;
use crate::{
    agent::AgentError,
    schemas::agent::AgentEvent,
    tools::Tool,
};
use super::{
    OutputValidator, ValidationResult, FormatType,
    ResponseSanitizer, RobustJsonParser,
};

/// Configuration for agent output parsing
#[derive(Debug, Clone)]
pub struct ParsingConfig {
    /// Enable robust JSON parsing with error recovery
    pub enable_json_recovery: bool,
    /// Enable response sanitization
    pub enable_sanitization: bool,
    /// Enable output validation
    pub enable_validation: bool,
    /// Maximum number of parsing retry attempts
    pub max_retry_attempts: usize,
    /// Agent type for format-specific handling
    pub agent_type: String,
    /// Available tools for validation
    pub available_tools: Vec<String>,
    /// Strict mode - fail on any validation errors
    pub strict_mode: bool,
}

impl Default for ParsingConfig {
    fn default() -> Self {
        Self {
            enable_json_recovery: true,
            enable_sanitization: true,
            enable_validation: true,
            max_retry_attempts: 3,
            agent_type: "generic".to_string(),
            available_tools: Vec::new(),
            strict_mode: false,
        }
    }
}

/// Parsing result with detailed information
#[derive(Debug)]
pub struct ParsingResult {
    /// The parsed agent event
    pub event: AgentEvent,
    /// Validation result if validation was enabled
    pub validation: Option<ValidationResult>,
    /// Whether any recovery mechanisms were used
    pub recovery_used: bool,
    /// Number of retry attempts made
    pub retry_attempts: usize,
    /// Original raw input
    pub raw_input: String,
    /// Sanitized input (if sanitization was used)
    pub sanitized_input: Option<String>,
}

/// Common interface for all agent output parsers
#[async_trait]
pub trait AgentOutputParser: Send + Sync {
    /// Parse LLM output into an AgentEvent
    async fn parse(&self, text: &str) -> Result<AgentEvent, AgentError> {
        self.parse_with_config(text, &ParsingConfig::default()).await
            .map(|result| result.event)
    }

    /// Parse with detailed configuration and result information
    async fn parse_with_config(&self, text: &str, config: &ParsingConfig) -> Result<ParsingResult, AgentError>;

    /// Get the format type this parser handles
    fn format_type(&self) -> FormatType;

    /// Get parser-specific configuration
    fn get_config(&self) -> ParsingConfig {
        ParsingConfig::default()
    }

    /// Update available tools for validation
    fn update_tools(&mut self, tools: &[Arc<dyn Tool>]);

    /// Validate output format without parsing
    fn validate_format(&self, text: &str) -> ValidationResult;
}

/// Enhanced parser that implements the common parsing pipeline
pub struct EnhancedAgentParser {
    /// Core parser implementation
    core_parser: Box<dyn CoreParser>,
    /// JSON parser for robust JSON handling
    json_parser: RobustJsonParser,
    /// Response sanitizer
    sanitizer: ResponseSanitizer,
    /// Output validator
    validator: OutputValidator,
    /// Parser configuration
    config: ParsingConfig,
}

/// Core parser trait for format-specific parsing logic
#[async_trait]
pub trait CoreParser: Send + Sync {
    /// Parse sanitized and validated input
    async fn parse_core(&self, text: &str) -> Result<AgentEvent, AgentError>;
    
    /// Get the format type
    fn format_type(&self) -> FormatType;
    
    /// Extract specific fields from the text
    fn extract_fields(&self, text: &str) -> Result<ParsedFields, AgentError>;
}

/// Parsed fields from agent output
#[derive(Debug, Clone)]
pub struct ParsedFields {
    pub thought: Option<String>,
    pub action: Option<String>,
    pub action_input: Option<String>,
    pub final_answer: Option<String>,
    pub raw_content: String,
}

impl EnhancedAgentParser {
    pub fn new(core_parser: Box<dyn CoreParser>, config: ParsingConfig) -> Self {
        let sanitizer = ResponseSanitizer::for_agent_type(&config.agent_type);
        let validator = OutputValidator::new();
        
        // Configure validator with available tools
        if !config.available_tools.is_empty() {
            // Update validator with tool information
            // This would be implemented based on the specific validator setup
        }

        Self {
            core_parser,
            json_parser: RobustJsonParser::new(),
            sanitizer,
            validator,
            config,
        }
    }

    /// Create a parser for a specific agent type
    pub fn for_agent_type(agent_type: &str, tools: &[Arc<dyn Tool>]) -> Self {
        let config = ParsingConfig {
            agent_type: agent_type.to_string(),
            available_tools: tools.iter().map(|t| t.name()).collect(),
            ..ParsingConfig::default()
        };

        // Create appropriate core parser based on agent type
        let core_parser: Box<dyn CoreParser> = match agent_type {
            "react" => Box::new(ReActCoreParser::new()),
            "chat" => Box::new(ChatCoreParser::new()),
            "openai_tools" => Box::new(OpenAIToolsCoreParser::new()),
            _ => Box::new(GenericCoreParser::new()),
        };

        Self::new(core_parser, config)
    }
}

#[async_trait]
impl AgentOutputParser for EnhancedAgentParser {
    async fn parse_with_config(&self, text: &str, config: &ParsingConfig) -> Result<ParsingResult, AgentError> {
        let mut result = ParsingResult {
            event: AgentEvent::Finish(crate::schemas::agent::AgentFinish {
                output: "Parsing failed".to_string(),
            }),
            validation: None,
            recovery_used: false,
            retry_attempts: 0,
            raw_input: text.to_string(),
            sanitized_input: None,
        };

        let mut current_text = text.to_string();

        // Step 1: Sanitization
        if config.enable_sanitization {
            current_text = self.sanitizer.sanitize(&current_text);
            result.sanitized_input = Some(current_text.clone());
        }

        // Step 2: Validation (pre-parsing)
        if config.enable_validation {
            let validation_result = self.validator.validate(&current_text, &self.core_parser.format_type());
            result.validation = Some(validation_result.clone());
            
            if config.strict_mode && !validation_result.is_valid {
                return Err(AgentError::OutputParsingError(
                    format!("Validation failed: {:?}", validation_result.errors)
                ));
            }
        }

        // Step 3: Parsing with retry logic
        let mut last_error = None;
        for attempt in 0..=config.max_retry_attempts {
            result.retry_attempts = attempt;
            
            match self.core_parser.parse_core(&current_text).await {
                Ok(event) => {
                    result.event = event;
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    
                    // Apply recovery strategies
                    if config.enable_json_recovery && attempt < config.max_retry_attempts {
                        if let Ok(recovered_text) = self.apply_recovery_strategy(&current_text, attempt) {
                            current_text = recovered_text;
                            result.recovery_used = true;
                            continue;
                        }
                    }
                }
            }
        }

        // If all attempts failed, return the last error
        Err(last_error.unwrap_or_else(|| AgentError::OutputParsingError("Unknown parsing error".to_string())))
    }

    fn format_type(&self) -> FormatType {
        self.core_parser.format_type()
    }

    fn get_config(&self) -> ParsingConfig {
        self.config.clone()
    }

    fn update_tools(&mut self, tools: &[Arc<dyn Tool>]) {
        self.config.available_tools = tools.iter().map(|t| t.name()).collect();
        // Update validator with new tools
    }

    fn validate_format(&self, text: &str) -> ValidationResult {
        self.validator.validate(text, &self.core_parser.format_type())
    }
}

impl EnhancedAgentParser {
    /// Apply recovery strategies based on the attempt number
    fn apply_recovery_strategy(&self, text: &str, attempt: usize) -> Result<String, AgentError> {
        match attempt {
            0 => {
                // First attempt: Try to fix JSON in action input
                self.fix_json_in_action_input(text)
            }
            1 => {
                // Second attempt: Try to extract and repair structured content
                self.extract_and_repair_structure(text)
            }
            2 => {
                // Third attempt: Apply aggressive sanitization
                self.aggressive_sanitization(text)
            }
            _ => Err(AgentError::OutputParsingError("No more recovery strategies available".to_string())),
        }
    }

    fn fix_json_in_action_input(&self, text: &str) -> Result<String, AgentError> {
        // Extract action input and try to fix it
        if let Ok(fields) = self.core_parser.extract_fields(text) {
            if let Some(action_input) = fields.action_input {
                match self.json_parser.parse(&action_input) {
                    Ok(fixed_json) => {
                        let fixed_input = serde_json::to_string(&fixed_json)
                            .map_err(|e| AgentError::OutputParsingError(e.to_string()))?;
                        
                        // Replace the action input in the original text
                        let result = text.replace(&action_input, &fixed_input);
                        return Ok(result);
                    }
                    Err(_) => {}
                }
            }
        }
        
        Err(AgentError::OutputParsingError("Could not fix JSON in action input".to_string()))
    }

    fn extract_and_repair_structure(&self, text: &str) -> Result<String, AgentError> {
        // Try to extract and rebuild the structure
        if let Ok(fields) = self.core_parser.extract_fields(text) {
            let mut rebuilt = String::new();
            
            if let Some(thought) = fields.thought {
                rebuilt.push_str(&format!("Thought: {}\n", thought));
            }
            
            if let Some(action) = fields.action {
                rebuilt.push_str(&format!("Action: {}\n", action));
                
                if let Some(action_input) = fields.action_input {
                    // Try to fix the action input
                    let fixed_input = self.json_parser.parse(&action_input)
                        .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()))
                        .unwrap_or_else(|_| "{}".to_string());
                    
                    rebuilt.push_str(&format!("Action Input: {}\n", fixed_input));
                }
            }
            
            if let Some(final_answer) = fields.final_answer {
                rebuilt.push_str(&format!("Final Answer: {}\n", final_answer));
            }
            
            if !rebuilt.is_empty() {
                return Ok(rebuilt.trim().to_string());
            }
        }
        
        Err(AgentError::OutputParsingError("Could not extract and repair structure".to_string()))
    }

    fn aggressive_sanitization(&self, text: &str) -> Result<String, AgentError> {
        // Apply very aggressive sanitization
        let mut config = super::SanitizationConfig::default();
        config.remove_artifacts = true;
        config.fix_formatting = true;
        config.normalize_whitespace = true;
        
        // Add aggressive custom replacements
        config.custom_replacements.insert(r"\{[^}]*JSON[^}]*\}".to_string(), "{}".to_string());
        config.custom_replacements.insert(r"\[.*?JSON.*?\]".to_string(), "{}".to_string());
        
        let aggressive_sanitizer = ResponseSanitizer::new(config);
        let sanitized = aggressive_sanitizer.sanitize(text);
        
        if sanitized != text {
            Ok(sanitized)
        } else {
            Err(AgentError::OutputParsingError("Aggressive sanitization did not change the text".to_string()))
        }
    }
}

// Placeholder implementations for different core parsers
// These would be implemented with the actual parsing logic for each agent type

pub struct ReActCoreParser;
impl ReActCoreParser {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl CoreParser for ReActCoreParser {
    async fn parse_core(&self, _text: &str) -> Result<AgentEvent, AgentError> {
        // Implementation would go here
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
    
    fn format_type(&self) -> FormatType {
        FormatType::ReAct
    }
    
    fn extract_fields(&self, _text: &str) -> Result<ParsedFields, AgentError> {
        // Implementation would go here
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
}

pub struct ChatCoreParser;
impl ChatCoreParser {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl CoreParser for ChatCoreParser {
    async fn parse_core(&self, _text: &str) -> Result<AgentEvent, AgentError> {
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
    
    fn format_type(&self) -> FormatType {
        FormatType::Chat
    }
    
    fn extract_fields(&self, _text: &str) -> Result<ParsedFields, AgentError> {
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
}

pub struct OpenAIToolsCoreParser;
impl OpenAIToolsCoreParser {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl CoreParser for OpenAIToolsCoreParser {
    async fn parse_core(&self, _text: &str) -> Result<AgentEvent, AgentError> {
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
    
    fn format_type(&self) -> FormatType {
        FormatType::OpenAITools
    }
    
    fn extract_fields(&self, _text: &str) -> Result<ParsedFields, AgentError> {
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
}

pub struct GenericCoreParser;
impl GenericCoreParser {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl CoreParser for GenericCoreParser {
    async fn parse_core(&self, _text: &str) -> Result<AgentEvent, AgentError> {
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
    
    fn format_type(&self) -> FormatType {
        FormatType::Custom("generic".to_string())
    }
    
    fn extract_fields(&self, _text: &str) -> Result<ParsedFields, AgentError> {
        Err(AgentError::OutputParsingError("Not implemented".to_string()))
    }
}
