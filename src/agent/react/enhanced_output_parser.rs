//! Enhanced ReAct output parser using the new parsing system

use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;

use crate::{
    agent::{
        AgentError,
        parsing::{
            CoreParser, ParsedFields, FormatType, AgentOutputParser,
            EnhancedAgentParser, ParsingConfig, ParsingResult,
            RobustJsonParser,
        },
    },
    schemas::agent::{AgentAction, AgentEvent, AgentFinish},
    tools::Tool,
};

/// Enhanced ReAct output parser with comprehensive error handling
pub struct EnhancedReActOutputParser {
    /// Core enhanced parser
    enhanced_parser: EnhancedAgentParser,
    /// Regex patterns for ReAct format
    thought_regex: Regex,
    action_regex: Regex,
    action_input_regex: Regex,
    final_answer_regex: Regex,
    /// Available tools for validation
    available_tools: Vec<String>,
}

impl EnhancedReActOutputParser {
    pub fn new() -> Self {
        let core_parser = Box::new(ReActCoreParserImpl::new());
        let config = ParsingConfig {
            agent_type: "react".to_string(),
            enable_json_recovery: true,
            enable_sanitization: true,
            enable_validation: true,
            max_retry_attempts: 3,
            strict_mode: false,
            available_tools: Vec::new(),
        };
        
        let enhanced_parser = EnhancedAgentParser::new(core_parser, config);
        
        Self {
            enhanced_parser,
            thought_regex: Regex::new(r"Thought:\s*(.+)")
                .expect("Invalid thought regex"),
            action_regex: Regex::new(r"Action:\s*(.+)")
                .expect("Invalid action regex"),
            action_input_regex: Regex::new(r"Action Input:\s*(.+)")
                .expect("Invalid action input regex"),
            final_answer_regex: Regex::new(r"Final Answer:\s*(.+)")
                .expect("Invalid final answer regex"),
            available_tools: Vec::new(),
        }
    }

    /// Create parser with specific tools
    pub fn with_tools(tools: &[Arc<dyn Tool>]) -> Self {
        let mut parser = Self::new();
        parser.update_tools(tools);
        parser
    }

    /// Parse with detailed result information
    pub async fn parse_with_details(&self, text: &str) -> Result<ParsingResult, AgentError> {
        let config = self.enhanced_parser.get_config();
        self.enhanced_parser.parse_with_config(text, &config).await
    }

    /// Extract thought from text
    fn extract_thought(&self, text: &str) -> Option<String> {
        self.thought_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                // Stop at the first occurrence of Action or Final Answer
                if let Some(pos) = content.find("\nAction") {
                    content[..pos].trim().to_string()
                } else if let Some(pos) = content.find("\nFinal Answer") {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }

    /// Extract action from text
    fn extract_action(&self, text: &str) -> Option<String> {
        self.action_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                // Stop at the first newline
                if let Some(pos) = content.find('\n') {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }

    /// Extract action input from text
    fn extract_action_input(&self, text: &str) -> Option<String> {
        self.action_input_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                // Stop at Observation
                if let Some(pos) = content.find("\nObservation") {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }

    /// Extract final answer from text
    fn extract_final_answer(&self, text: &str) -> Option<String> {
        self.final_answer_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                // Stop at the first newline
                if let Some(pos) = content.find('\n') {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }
}

#[async_trait]
impl AgentOutputParser for EnhancedReActOutputParser {
    async fn parse(&self, text: &str) -> Result<AgentEvent, AgentError> {
        self.enhanced_parser.parse(text).await
    }

    async fn parse_with_config(&self, text: &str, config: &ParsingConfig) -> Result<ParsingResult, AgentError> {
        self.enhanced_parser.parse_with_config(text, config).await
    }

    fn format_type(&self) -> FormatType {
        FormatType::ReAct
    }

    fn update_tools(&mut self, tools: &[Arc<dyn Tool>]) {
        self.available_tools = tools.iter().map(|t| t.name()).collect();
        self.enhanced_parser.update_tools(tools);
    }

    fn validate_format(&self, text: &str) -> crate::agent::parsing::ValidationResult {
        self.enhanced_parser.validate_format(text)
    }
}

/// Core parser implementation for ReAct format
struct ReActCoreParserImpl {
    thought_regex: Regex,
    action_regex: Regex,
    action_input_regex: Regex,
    final_answer_regex: Regex,
    json_parser: RobustJsonParser,
}

impl ReActCoreParserImpl {
    fn new() -> Self {
        Self {
            thought_regex: Regex::new(r"Thought:\s*(.+)")
                .expect("Invalid thought regex"),
            action_regex: Regex::new(r"Action:\s*(.+)")
                .expect("Invalid action regex"),
            action_input_regex: Regex::new(r"Action Input:\s*(.+)")
                .expect("Invalid action input regex"),
            final_answer_regex: Regex::new(r"Final Answer:\s*(.+)")
                .expect("Invalid final answer regex"),
            json_parser: RobustJsonParser::new(),
        }
    }

    fn extract_thought(&self, text: &str) -> Option<String> {
        self.thought_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                if let Some(pos) = content.find("\nAction") {
                    content[..pos].trim().to_string()
                } else if let Some(pos) = content.find("\nFinal Answer") {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }

    fn extract_action(&self, text: &str) -> Option<String> {
        self.action_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                if let Some(pos) = content.find('\n') {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }

    fn extract_action_input(&self, text: &str) -> Option<String> {
        self.action_input_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                if let Some(pos) = content.find("\nObservation") {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }

    fn extract_final_answer(&self, text: &str) -> Option<String> {
        self.final_answer_regex
            .captures(text)
            .and_then(|caps| caps.get(1))
            .map(|m| {
                let content = m.as_str().trim();
                if let Some(pos) = content.find('\n') {
                    content[..pos].trim().to_string()
                } else {
                    content.to_string()
                }
            })
    }
}

#[async_trait]
impl CoreParser for ReActCoreParserImpl {
    async fn parse_core(&self, text: &str) -> Result<AgentEvent, AgentError> {
        let text = text.trim();

        // Check for final answer first
        if let Some(final_answer) = self.extract_final_answer(text) {
            return Ok(AgentEvent::Finish(AgentFinish {
                output: final_answer,
            }));
        }

        // Extract thought (optional but good for logging)
        let thought = self.extract_thought(text);

        // Extract action and action input
        let action_name = self.extract_action(text)
            .ok_or_else(|| AgentError::OutputParsingError(
                format!("Could not parse action from output: {}", text)
            ))?;

        let action_input = self.extract_action_input(text)
            .ok_or_else(|| AgentError::OutputParsingError(
                format!("Could not parse action input from output: {}", text)
            ))?;

        // Use robust JSON parser to handle the action input
        let parsed_json = self.json_parser.parse(&action_input)?;
        let fixed_action_input = serde_json::to_string(&parsed_json)
            .map_err(|e| AgentError::OutputParsingError(
                format!("Failed to serialize parsed JSON: {}", e)
            ))?;

        let log_message = if let Some(thought) = thought {
            format!("Thought: {}\nAction: {}\nAction Input: {}", thought, action_name, fixed_action_input)
        } else {
            format!("Action: {}\nAction Input: {}", action_name, fixed_action_input)
        };

        Ok(AgentEvent::Action(vec![AgentAction {
            tool: action_name,
            tool_input: fixed_action_input,
            log: log_message,
        }]))
    }

    fn format_type(&self) -> FormatType {
        FormatType::ReAct
    }

    fn extract_fields(&self, text: &str) -> Result<ParsedFields, AgentError> {
        Ok(ParsedFields {
            thought: self.extract_thought(text),
            action: self.extract_action(text),
            action_input: self.extract_action_input(text),
            final_answer: self.extract_final_answer(text),
            raw_content: text.to_string(),
        })
    }
}

impl Default for EnhancedReActOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_valid_react_output() {
        let parser = EnhancedReActOutputParser::new();
        let output = r#"Thought: I need to search for information.
Action: search
Action Input: {"query": "test"}"#;

        let result = parser.parse(output).await.unwrap();
        match result {
            AgentEvent::Action(actions) => {
                assert_eq!(actions.len(), 1);
                assert_eq!(actions[0].tool, "search");
                assert_eq!(actions[0].tool_input, r#"{"query":"test"}"#);
            }
            _ => panic!("Expected action event"),
        }
    }

    #[tokio::test]
    async fn test_parse_malformed_json() {
        let parser = EnhancedReActOutputParser::new();
        let output = r#"Thought: I need to search for information.
Action: search
Action Input: {JSON}"#;

        let result = parser.parse(output).await.unwrap();
        match result {
            AgentEvent::Action(actions) => {
                assert_eq!(actions.len(), 1);
                assert_eq!(actions[0].tool, "search");
                // Should be fixed to empty object
                assert_eq!(actions[0].tool_input, "{}");
            }
            _ => panic!("Expected action event"),
        }
    }

    #[tokio::test]
    async fn test_parse_with_thinking_tags() {
        let parser = EnhancedReActOutputParser::new();
        let output = r#"<think>Let me think about this</think>
Thought: I need to search for information.
Action: search
Action Input: {"query": "test"}"#;

        let result = parser.parse(output).await.unwrap();
        match result {
            AgentEvent::Action(actions) => {
                assert_eq!(actions.len(), 1);
                assert_eq!(actions[0].tool, "search");
            }
            _ => panic!("Expected action event"),
        }
    }

    #[tokio::test]
    async fn test_parse_final_answer() {
        let parser = EnhancedReActOutputParser::new();
        let output = r#"Thought: I have the information I need.
Final Answer: The answer is 42."#;

        let result = parser.parse(output).await.unwrap();
        match result {
            AgentEvent::Finish(finish) => {
                assert_eq!(finish.output, "The answer is 42.");
            }
            _ => panic!("Expected finish event"),
        }
    }
}
