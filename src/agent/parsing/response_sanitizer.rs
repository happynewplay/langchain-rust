//! Response sanitization for cleaning and normalizing LLM outputs

use regex::Regex;
use std::collections::HashMap;

/// Configuration for response sanitization
#[derive(Debug, Clone)]
pub struct SanitizationConfig {
    /// Remove thinking tags and internal reasoning
    pub remove_thinking_tags: bool,
    /// Extract content from markdown code blocks
    pub extract_code_blocks: bool,
    /// Normalize whitespace and line endings
    pub normalize_whitespace: bool,
    /// Remove common LLM artifacts
    pub remove_artifacts: bool,
    /// Fix common formatting issues
    pub fix_formatting: bool,
    /// Custom replacement patterns
    pub custom_replacements: HashMap<String, String>,
}

impl Default for SanitizationConfig {
    fn default() -> Self {
        Self {
            remove_thinking_tags: true,
            extract_code_blocks: true,
            normalize_whitespace: true,
            remove_artifacts: true,
            fix_formatting: true,
            custom_replacements: HashMap::new(),
        }
    }
}

/// Response sanitizer for cleaning LLM outputs
pub struct ResponseSanitizer {
    config: SanitizationConfig,
    thinking_tag_regex: Regex,
    code_block_regex: Regex,
    artifact_patterns: Vec<Regex>,
    whitespace_regex: Regex,
}

impl ResponseSanitizer {
    pub fn new(config: SanitizationConfig) -> Self {
        let thinking_tag_regex = Regex::new(r"<think>.*?</think>").unwrap();
        let code_block_regex = Regex::new(r"```(?:json|javascript|text)?\s*([\s\S]*?)\s*```").unwrap();
        let whitespace_regex = Regex::new(r"\s+").unwrap();
        
        let artifact_patterns = vec![
            // Common LLM prefixes and suffixes
            Regex::new(r"^(Here's|Here is|I'll|I will|Let me|Based on|According to).*?:").unwrap(),
            Regex::new(r"(I hope this helps|Let me know if you need|Feel free to ask).*$").unwrap(),
            // Conversational elements
            Regex::new(r"^(Sure|Certainly|Of course|Absolutely)[,!.]?\s*").unwrap(),
            // Meta-commentary
            Regex::new(r"\(Note:.*?\)").unwrap(),
            Regex::new(r"\[Note:.*?\]").unwrap(),
            // Placeholder text
            Regex::new(r"\[INSERT.*?\]").unwrap(),
            Regex::new(r"\{PLACEHOLDER.*?\}").unwrap(),
            // Common formatting artifacts
            Regex::new(r"^\s*[-*]\s*").unwrap(), // List markers at start
            Regex::new(r"^\s*\d+\.\s*").unwrap(), // Numbered list markers
        ];

        Self {
            config,
            thinking_tag_regex,
            code_block_regex,
            artifact_patterns,
            whitespace_regex,
        }
    }

    /// Sanitize a response according to the configuration
    pub fn sanitize(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Apply sanitization steps in order
        if self.config.remove_thinking_tags {
            result = self.remove_thinking_tags(&result);
        }

        if self.config.extract_code_blocks {
            result = self.extract_code_blocks(&result);
        }

        if self.config.remove_artifacts {
            result = self.remove_artifacts(&result);
        }

        if self.config.fix_formatting {
            result = self.fix_formatting(&result);
        }

        if self.config.normalize_whitespace {
            result = self.normalize_whitespace(&result);
        }

        // Apply custom replacements
        for (pattern, replacement) in &self.config.custom_replacements {
            if let Ok(regex) = Regex::new(pattern) {
                result = regex.replace_all(&result, replacement).to_string();
            }
        }

        result.trim().to_string()
    }

    /// Remove thinking tags and internal reasoning
    fn remove_thinking_tags(&self, input: &str) -> String {
        let mut result = self.thinking_tag_regex.replace_all(input, "").to_string();
        
        // Also handle unclosed thinking tags
        if let Some(start) = result.find("<think>") {
            if result[start..].find("</think>").is_none() {
                // Remove everything from <think> to end if no closing tag
                result = result[..start].to_string();
            }
        }
        
        // Handle other thinking patterns
        let other_thinking_patterns = vec![
            Regex::new(r"<thinking>.*?</thinking>").unwrap(),
            Regex::new(r"\(thinking:.*?\)").unwrap(),
            Regex::new(r"\[thinking:.*?\]").unwrap(),
        ];
        
        for pattern in other_thinking_patterns {
            result = pattern.replace_all(&result, "").to_string();
        }
        
        result
    }

    /// Extract content from markdown code blocks
    fn extract_code_blocks(&self, input: &str) -> String {
        if let Some(caps) = self.code_block_regex.captures(input) {
            if let Some(content) = caps.get(1) {
                return content.as_str().trim().to_string();
            }
        }
        input.to_string()
    }

    /// Remove common LLM artifacts and conversational elements
    fn remove_artifacts(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        for pattern in &self.artifact_patterns {
            result = pattern.replace_all(&result, "").to_string();
        }
        
        result
    }

    /// Fix common formatting issues
    fn fix_formatting(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        // Fix common JSON formatting issues
        result = self.fix_json_formatting(&result);
        
        // Fix line ending issues
        result = result.replace("\r\n", "\n").replace("\r", "\n");
        
        // Fix multiple consecutive newlines
        let multiple_newlines = Regex::new(r"\n{3,}").unwrap();
        result = multiple_newlines.replace_all(&result, "\n\n").to_string();
        
        // Fix spacing around colons in structured formats
        let colon_spacing = Regex::new(r"(\w+)\s*:\s*").unwrap();
        result = colon_spacing.replace_all(&result, "$1: ").to_string();
        
        result
    }

    /// Fix JSON-specific formatting issues
    fn fix_json_formatting(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        // Fix common JSON issues
        let json_fixes = vec![
            // Fix single quotes to double quotes
            (r"'([^']*)'", r#""$1""#),
            // Fix unquoted keys
            (r"\{(\w+):", r#"{"$1":"#),
            (r",\s*(\w+):", r#", "$1":"#),
            // Fix trailing commas
            (r",\s*([}\]])", r"$1"),
            // Fix missing quotes around string values
            (r":\s*([a-zA-Z][a-zA-Z0-9\s]*[a-zA-Z0-9])\s*([,}])", r#": "$1"$2"#),
        ];
        
        for (pattern, replacement) in json_fixes {
            if let Ok(regex) = Regex::new(pattern) {
                result = regex.replace_all(&result, replacement).to_string();
            }
        }
        
        result
    }

    /// Normalize whitespace and line endings
    fn normalize_whitespace(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        // Normalize line endings
        result = result.replace("\r\n", "\n").replace("\r", "\n");
        
        // Remove trailing whitespace from lines
        let lines: Vec<String> = result
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        result = lines.join("\n");
        
        // Normalize spacing within lines (but preserve structure)
        let lines: Vec<String> = result
            .lines()
            .map(|line| {
                if line.trim().is_empty() {
                    String::new()
                } else {
                    // Preserve structure by only normalizing internal whitespace
                    let trimmed = line.trim();
                    if trimmed.contains(':') {
                        // Structured line - preserve format
                        trimmed.to_string()
                    } else {
                        // Regular line - normalize whitespace
                        self.whitespace_regex.replace_all(trimmed, " ").to_string()
                    }
                }
            })
            .collect();
        
        lines.join("\n")
    }

    /// Create a sanitizer with specific presets
    pub fn for_agent_type(agent_type: &str) -> Self {
        let config = match agent_type {
            "react" => SanitizationConfig {
                remove_thinking_tags: true,
                extract_code_blocks: false, // ReAct doesn't use code blocks
                normalize_whitespace: true,
                remove_artifacts: true,
                fix_formatting: true,
                custom_replacements: {
                    let mut replacements = HashMap::new();
                    // Common ReAct-specific fixes
                    replacements.insert(r"\{JSON\}".to_string(), "{}".to_string());
                    replacements.insert(r"\{json\}".to_string(), "{}".to_string());
                    replacements.insert(r"\[JSON object\]".to_string(), "{}".to_string());
                    replacements
                },
            },
            "chat" => SanitizationConfig {
                remove_thinking_tags: true,
                extract_code_blocks: true, // Chat agents often use code blocks
                normalize_whitespace: true,
                remove_artifacts: true,
                fix_formatting: true,
                custom_replacements: HashMap::new(),
            },
            "openai_tools" => SanitizationConfig {
                remove_thinking_tags: false, // OpenAI tools format is more structured
                extract_code_blocks: true,
                normalize_whitespace: false, // Preserve exact formatting
                remove_artifacts: false,
                fix_formatting: true,
                custom_replacements: HashMap::new(),
            },
            _ => SanitizationConfig::default(),
        };
        
        Self::new(config)
    }
}

impl Default for ResponseSanitizer {
    fn default() -> Self {
        Self::new(SanitizationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_thinking_tags() {
        let sanitizer = ResponseSanitizer::default();
        let input = r#"<think>Let me think about this</think>Thought: I need to search"#;
        let result = sanitizer.sanitize(input);
        assert_eq!(result, "Thought: I need to search");
    }

    #[test]
    fn test_extract_code_blocks() {
        let sanitizer = ResponseSanitizer::default();
        let input = r#"```json
{"query": "test"}
```"#;
        let result = sanitizer.sanitize(input);
        assert_eq!(result, r#"{"query": "test"}"#);
    }

    #[test]
    fn test_remove_artifacts() {
        let sanitizer = ResponseSanitizer::default();
        let input = "Sure! Here's the response: Thought: I need to search";
        let result = sanitizer.sanitize(input);
        assert_eq!(result, "Thought: I need to search");
    }

    #[test]
    fn test_fix_json_formatting() {
        let sanitizer = ResponseSanitizer::default();
        let input = r#"{'query': 'test',}"#;
        let result = sanitizer.sanitize(input);
        assert!(result.contains(r#""query": "test""#));
    }

    #[test]
    fn test_react_specific_sanitization() {
        let sanitizer = ResponseSanitizer::for_agent_type("react");
        let input = "Thought: I need to search\nAction: search\nAction Input: {JSON}";
        let result = sanitizer.sanitize(input);
        assert!(result.contains("Action Input: {}"));
    }

    #[test]
    fn test_normalize_whitespace() {
        let sanitizer = ResponseSanitizer::default();
        let input = "Thought:    I   need   to   search\nAction:  search  ";
        let result = sanitizer.sanitize(input);
        assert_eq!(result, "Thought: I need to search\nAction: search");
    }
}
