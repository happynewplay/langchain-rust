//! Robust JSON parser with comprehensive error recovery for LLM outputs

use regex::Regex;
use serde_json::{Value, Error as JsonError};
use std::io;
use crate::agent::AgentError;

/// Robust JSON parser that can handle common LLM output issues
#[derive(Debug, Clone)]
pub struct RobustJsonParser {
    /// Regex patterns for common JSON placeholder issues
    placeholder_patterns: Vec<Regex>,
    /// Maximum attempts for JSON repair
    max_repair_attempts: usize,
}

impl RobustJsonParser {
    pub fn new() -> Self {
        let placeholder_patterns = vec![
            // Match {JSON}, {json}, etc.
            Regex::new(r"\{JSON\}").unwrap(),
            Regex::new(r"\{json\}").unwrap(),
            Regex::new(r"\{Json\}").unwrap(),
            // Match placeholder text like [JSON object], etc.
            Regex::new(r"\[JSON object\]").unwrap(),
            Regex::new(r"\[json object\]").unwrap(),
            Regex::new(r"\[JSON\]").unwrap(),
            // Match template placeholders
            Regex::new(r"\{\{[^}]+\}\}").unwrap(),
            // Match incomplete JSON patterns
            Regex::new(r"\{[^}]*$").unwrap(),
        ];

        Self {
            placeholder_patterns,
            max_repair_attempts: 5,
        }
    }

    /// Parse JSON with comprehensive error recovery
    pub fn parse(&self, input: &str) -> Result<Value, AgentError> {
        let input = input.trim();
        
        // First, try direct parsing
        if let Ok(value) = serde_json::from_str::<Value>(input) {
            return Ok(value);
        }

        // Apply sanitization and repair strategies
        let sanitized = self.sanitize_input(input);
        
        // Try parsing the sanitized input
        if let Ok(value) = serde_json::from_str::<Value>(&sanitized) {
            return Ok(value);
        }

        // Apply progressive repair strategies
        for attempt in 0..self.max_repair_attempts {
            if let Ok(value) = self.attempt_repair(&sanitized, attempt) {
                return Ok(value);
            }
        }

        // If all else fails, try to extract any JSON-like content
        if let Some(extracted) = self.extract_json_content(input) {
            if let Ok(value) = serde_json::from_str::<Value>(&extracted) {
                return Ok(value);
            }
        }

        // Final fallback: create a simple string value
        Ok(Value::String(input.to_string()))
    }

    /// Sanitize input by removing common LLM artifacts
    fn sanitize_input(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Remove thinking tags
        if let Some(start) = result.find("<think>") {
            if let Some(end) = result.find("</think>") {
                result = format!("{}{}", &result[..start], &result[end + 8..]);
            }
        }

        // Remove markdown code blocks
        let code_block_regex = Regex::new(r"```(?:json)?\s*([\s\S]*?)\s*```").unwrap();
        if let Some(caps) = code_block_regex.captures(&result) {
            if let Some(json_content) = caps.get(1) {
                result = json_content.as_str().to_string();
            }
        }

        // Replace common placeholders with empty objects
        for pattern in &self.placeholder_patterns {
            result = pattern.replace_all(&result, "{}").to_string();
        }

        // Clean up whitespace
        result.trim().to_string()
    }

    /// Attempt to repair JSON using various strategies
    fn attempt_repair(&self, input: &str, attempt: usize) -> Result<Value, JsonError> {
        match attempt {
            0 => self.repair_quotes(input),
            1 => self.repair_trailing_commas(input),
            2 => self.repair_unquoted_keys(input),
            3 => self.repair_incomplete_structures(input),
            4 => self.repair_with_partial_parsing(input),
            _ => Err(JsonError::io(io::Error::new(io::ErrorKind::InvalidData, "All repair attempts exhausted"))),
        }
    }

    /// Fix quote issues in JSON
    fn repair_quotes(&self, input: &str) -> Result<Value, JsonError> {
        let mut result = input.to_string();
        
        // Replace single quotes with double quotes
        let single_quote_regex = Regex::new(r"'([^']*)'").unwrap();
        result = single_quote_regex.replace_all(&result, r#""$1""#).to_string();
        
        // Fix escaped quotes
        result = result.replace(r#"\""#, r#"""#);
        
        serde_json::from_str(&result)
    }

    /// Remove trailing commas
    fn repair_trailing_commas(&self, input: &str) -> Result<Value, JsonError> {
        let trailing_comma_regex = Regex::new(r",\s*([}\]])").unwrap();
        let result = trailing_comma_regex.replace_all(input, "$1").to_string();
        serde_json::from_str(&result)
    }

    /// Add quotes to unquoted keys
    fn repair_unquoted_keys(&self, input: &str) -> Result<Value, JsonError> {
        let mut result = input.to_string();
        
        // Common unquoted key patterns
        let patterns = vec![
            (r"\{(\w+):", r#"{"$1":"#),
            (r",\s*(\w+):", r#", "$1":"#),
            (r"\{\s*(\w+)\s*:", r#"{"$1":"#),
        ];
        
        for (pattern, replacement) in patterns {
            let regex = Regex::new(pattern).unwrap();
            result = regex.replace_all(&result, replacement).to_string();
        }
        
        serde_json::from_str(&result)
    }

    /// Repair incomplete JSON structures
    fn repair_incomplete_structures(&self, input: &str) -> Result<Value, JsonError> {
        let mut result = input.to_string();
        
        // Count braces and brackets to determine what's missing
        let open_braces = result.matches('{').count();
        let close_braces = result.matches('}').count();
        let open_brackets = result.matches('[').count();
        let close_brackets = result.matches(']').count();
        
        // Add missing closing braces
        for _ in 0..(open_braces.saturating_sub(close_braces)) {
            result.push('}');
        }
        
        // Add missing closing brackets
        for _ in 0..(open_brackets.saturating_sub(close_brackets)) {
            result.push(']');
        }
        
        serde_json::from_str(&result)
    }

    /// Use partial parsing to extract valid JSON portions
    fn repair_with_partial_parsing(&self, input: &str) -> Result<Value, JsonError> {
        // Try to parse progressively smaller portions of the input
        let chars: Vec<char> = input.chars().collect();
        
        for end in (1..=chars.len()).rev() {
            let partial: String = chars[0..end].iter().collect();
            if let Ok(value) = serde_json::from_str::<Value>(&partial) {
                return Ok(value);
            }
        }
        
        Err(JsonError::io(io::Error::new(io::ErrorKind::InvalidData, "No valid JSON found in partial parsing")))
    }

    /// Extract JSON-like content from mixed text
    fn extract_json_content(&self, input: &str) -> Option<String> {
        // Look for JSON object patterns
        let json_regex = Regex::new(r"\{[^{}]*(?:\{[^{}]*\}[^{}]*)*\}").unwrap();
        
        if let Some(mat) = json_regex.find(input) {
            return Some(mat.as_str().to_string());
        }
        
        // Look for JSON array patterns
        let array_regex = Regex::new(r"\[[^\[\]]*(?:\[[^\[\]]*\][^\[\]]*)*\]").unwrap();
        
        if let Some(mat) = array_regex.find(input) {
            return Some(mat.as_str().to_string());
        }
        
        None
    }
}

impl Default for RobustJsonParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_json() {
        let parser = RobustJsonParser::new();
        let result = parser.parse(r#"{"query": "test"}"#).unwrap();
        assert_eq!(result["query"], "test");
    }

    #[test]
    fn test_parse_placeholder_json() {
        let parser = RobustJsonParser::new();
        let result = parser.parse("{JSON}").unwrap();
        assert_eq!(result, serde_json::json!({}));
    }

    #[test]
    fn test_parse_single_quotes() {
        let parser = RobustJsonParser::new();
        let result = parser.parse("{'query': 'test'}").unwrap();
        assert_eq!(result["query"], "test");
    }

    #[test]
    fn test_parse_trailing_comma() {
        let parser = RobustJsonParser::new();
        let result = parser.parse(r#"{"query": "test",}"#).unwrap();
        assert_eq!(result["query"], "test");
    }

    #[test]
    fn test_parse_unquoted_keys() {
        let parser = RobustJsonParser::new();
        let result = parser.parse(r#"{query: "test"}"#).unwrap();
        assert_eq!(result["query"], "test");
    }

    #[test]
    fn test_parse_incomplete_json() {
        let parser = RobustJsonParser::new();
        let result = parser.parse(r#"{"query": "test""#).unwrap();
        assert_eq!(result["query"], "test");
    }

    #[test]
    fn test_parse_with_thinking_tags() {
        let parser = RobustJsonParser::new();
        let input = r#"<think>Let me think about this</think>{"query": "test"}"#;
        let result = parser.parse(input).unwrap();
        assert_eq!(result["query"], "test");
    }

    #[test]
    fn test_parse_markdown_code_block() {
        let parser = RobustJsonParser::new();
        let input = r#"```json
{"query": "test"}
```"#;
        let result = parser.parse(input).unwrap();
        assert_eq!(result["query"], "test");
    }
}
