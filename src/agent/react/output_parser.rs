use regex::Regex;
use serde_json::Value;

use crate::{
    agent::{AgentError, RobustJsonParser},
    schemas::agent::{AgentAction, AgentEvent, AgentFinish},
};

/// Parser for ReAct agent output that can handle Thought/Action/Observation patterns
pub struct ReActOutputParser {
    thought_regex: Regex,
    action_regex: Regex,
    action_input_regex: Regex,
    final_answer_regex: Regex,
    json_parser: RobustJsonParser,
}

impl ReActOutputParser {
    pub fn new() -> Self {
        Self {
            thought_regex: Regex::new(r"Thought:\s*([^\n\r]+)")
                .expect("Invalid thought regex"),
            action_regex: Regex::new(r"Action:\s*([^\n\r]+)")
                .expect("Invalid action regex"),
            action_input_regex: Regex::new(r"Action Input:\s*([^\n\r]+)")
                .expect("Invalid action input regex"),
            final_answer_regex: Regex::new(r"Final Answer:\s*([^\n\r]+)")
                .expect("Invalid final answer regex"),
            json_parser: RobustJsonParser::new(),
        }
    }

    /// Parse the LLM output and determine if it's an action or final answer
    pub fn parse(&self, text: &str) -> Result<AgentEvent, AgentError> {
        let text = text.trim();

        // Remove thinking tags if present
        let cleaned_text = self.remove_thinking_tags(text);

        // Check for final answer first
        if let Some(final_answer) = self.extract_final_answer(&cleaned_text) {
            return Ok(AgentEvent::Finish(AgentFinish {
                output: final_answer,
            }));
        }

        // Extract thought (optional but good for logging)
        let thought = self.extract_thought(&cleaned_text);

        // Extract action and action input
        let action_name = self.extract_action(&cleaned_text)
            .ok_or_else(|| AgentError::OutputParsingError(
                format!("Could not parse action from output: {}", cleaned_text)
            ))?;

        let action_input = self.extract_action_input(&cleaned_text)
            .ok_or_else(|| AgentError::OutputParsingError(
                format!("Could not parse action input from output: {}", cleaned_text)
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

    /// Remove thinking tags and extract the actual response
    fn remove_thinking_tags(&self, text: &str) -> String {
        // Find the end of the </think> tag and take everything after it
        if let Some(end_pos) = text.find("</think>") {
            let after_think = &text[end_pos + 8..]; // 8 is the length of "</think>"
            after_think.trim().to_string()
        } else {
            // No thinking tags, return as is
            text.trim().to_string()
        }
    }

    /// Fix common JSON formatting issues in action input
    fn fix_action_input(&self, input: &str) -> String {
        let mut input = input.trim();

        // Remove trailing punctuation that might interfere
        if input.ends_with('.') || input.ends_with(',') {
            input = &input[..input.len()-1];
        }

        // Remove any trailing quotes that might have been added incorrectly
        let cleaned = if input.ends_with('"') && input.starts_with('"') && input.len() > 2 {
            // Check if it's a quoted JSON object
            let inner = &input[1..input.len()-1];
            if inner.starts_with('{') && inner.ends_with('}') {
                inner
            } else {
                input
            }
        } else if input.ends_with("}\"") && input.starts_with('{') {
            // Handle case like: {"query": "value"}"
            &input[..input.len()-1]
        } else {
            input
        };

        // If it's already valid JSON, return as is
        if serde_json::from_str::<Value>(cleaned).is_ok() {
            return cleaned.to_string();
        }

        // If it doesn't look like JSON at all, wrap it in quotes
        if !cleaned.starts_with('{') && !cleaned.starts_with('[') && !cleaned.starts_with('"') {
            return format!("\"{}\"", cleaned.replace('"', "\\\""));
        }

        // Try to fix common JSON issues
        let mut fixed = cleaned.to_string();

        // Fix unquoted keys in JSON objects
        if fixed.starts_with('{') && fixed.ends_with('}') {
            // Simple regex-like replacement for common patterns
            // Replace {key: with {"key":
            fixed = fixed.replace("{query:", r#"{"query":"#);
            fixed = fixed.replace("{expression:", r#"{"expression":"#);
            fixed = fixed.replace("{customer_id:", r#"{"customer_id":"#);
            fixed = fixed.replace("{action:", r#"{"action":"#);

            // Fix single quotes to double quotes
            if fixed.contains("'") {
                // Replace single quotes with double quotes, but be careful about escaping
                let mut result = String::new();
                let mut in_string = false;
                let mut chars = fixed.chars().peekable();

                while let Some(ch) = chars.next() {
                    match ch {
                        '\'' => {
                            result.push('"');
                            in_string = !in_string;
                        }
                        '"' if !in_string => {
                            result.push('"');
                            in_string = !in_string;
                        }
                        _ => result.push(ch),
                    }
                }
                fixed = result;
            }
        }

        // If it looks like it should be a string but isn't quoted
        if !fixed.starts_with('"') && !fixed.starts_with('{') && !fixed.starts_with('[') {
            fixed = format!("\"{}\"", fixed.replace('"', "\\\""));
        }

        fixed
    }
}

impl Default for ReActOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_action() {
        let parser = ReActOutputParser::new();
        let output = r#"Thought: I need to search for information about the weather.
Action: search
Action Input: {"query": "weather today"}"#;

        let result = parser.parse(output).unwrap();
        match result {
            AgentEvent::Action(actions) => {
                assert_eq!(actions.len(), 1);
                assert_eq!(actions[0].tool, "search");
                assert_eq!(actions[0].tool_input, r#"{"query": "weather today"}"#);
            }
            _ => panic!("Expected action event"),
        }
    }

    #[test]
    fn test_parse_final_answer() {
        let parser = ReActOutputParser::new();
        let output = r#"Thought: I have gathered enough information.
Final Answer: The weather today is sunny with a temperature of 25°C."#;

        let result = parser.parse(output).unwrap();
        match result {
            AgentEvent::Finish(finish) => {
                assert_eq!(finish.output, "The weather today is sunny with a temperature of 25°C.");
            }
            _ => panic!("Expected finish event"),
        }
    }

    #[test]
    fn test_parse_action_without_thought() {
        let parser = ReActOutputParser::new();
        let output = r#"Action: calculate
Action Input: {"expression": "2 + 2"}"#;

        let result = parser.parse(output).unwrap();
        match result {
            AgentEvent::Action(actions) => {
                assert_eq!(actions.len(), 1);
                assert_eq!(actions[0].tool, "calculate");
                assert_eq!(actions[0].tool_input, r#"{"expression": "2 + 2"}"#);
            }
            _ => panic!("Expected action event"),
        }
    }
}
