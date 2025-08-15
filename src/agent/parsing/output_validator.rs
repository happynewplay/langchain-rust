//! Unified output validation framework for agent responses

use regex::Regex;
use std::collections::HashMap;

/// Validation result with detailed information
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub confidence_score: f64,
    pub suggested_fixes: Vec<String>,
}

/// Validation error with context
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub message: String,
    pub location: Option<String>,
    pub severity: ErrorSeverity,
}

/// Types of validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    MissingRequiredField,
    InvalidFormat,
    InvalidJsonStructure,
    UnexpectedContent,
    IncompleteResponse,
    InvalidToolName,
    MalformedActionInput,
}

/// Validation warnings for non-critical issues
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub warning_type: ValidationWarningType,
    pub message: String,
    pub suggestion: Option<String>,
}

/// Types of validation warnings
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarningType {
    SuboptimalFormat,
    UnusualStructure,
    PotentialAmbiguity,
    PerformanceImpact,
}

/// Error severity levels
#[derive(Debug, Clone, PartialEq, Ord, PartialOrd, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Output format specifications
#[derive(Debug, Clone)]
pub struct OutputFormat {
    pub format_type: FormatType,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub field_validators: HashMap<String, FieldValidator>,
    pub structure_rules: Vec<StructureRule>,
}

/// Supported output formats
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FormatType {
    ReAct,
    Chat,
    OpenAITools,
    Custom(String),
}

/// Field validation rules
#[derive(Debug, Clone)]
pub struct FieldValidator {
    pub field_type: FieldType,
    pub pattern: Option<Regex>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub allowed_values: Option<Vec<String>>,
}

/// Field types for validation
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Json,
    ToolName,
    ActionInput,
    Thought,
    FinalAnswer,
}

/// Structure validation rules
#[derive(Debug, Clone)]
pub struct StructureRule {
    pub rule_type: StructureRuleType,
    pub description: String,
    pub validator: fn(&str) -> bool,
}

/// Types of structure rules
#[derive(Debug, Clone, PartialEq)]
pub enum StructureRuleType {
    StartsWith,
    Contains,
    FollowsPattern,
    HasSequence,
    ValidJson,
}

/// Comprehensive output validator
pub struct OutputValidator {
    formats: HashMap<FormatType, OutputFormat>,
    json_parser: super::RobustJsonParser,
}

impl OutputValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            formats: HashMap::new(),
            json_parser: super::RobustJsonParser::new(),
        };
        
        validator.register_default_formats();
        validator
    }

    /// Register a custom output format
    pub fn register_format(&mut self, format: OutputFormat) {
        self.formats.insert(format.format_type.clone(), format);
    }

    /// Validate output against a specific format
    pub fn validate(&self, output: &str, format_type: &FormatType) -> ValidationResult {
        let format = match self.formats.get(format_type) {
            Some(f) => f,
            None => return ValidationResult::error(format!("Unknown format type: {:?}", format_type)),
        };

        let mut result = ValidationResult::new();
        
        // Validate structure
        self.validate_structure(output, format, &mut result);
        
        // Validate fields
        self.validate_fields(output, format, &mut result);
        
        // Calculate confidence score
        result.confidence_score = self.calculate_confidence(&result);
        
        // Generate suggested fixes
        result.suggested_fixes = self.generate_fixes(&result, output);
        
        result
    }

    /// Validate structure rules
    fn validate_structure(&self, output: &str, format: &OutputFormat, result: &mut ValidationResult) {
        for rule in &format.structure_rules {
            if !(rule.validator)(output) {
                result.add_error(ValidationError {
                    error_type: ValidationErrorType::InvalidFormat,
                    message: format!("Structure rule violated: {}", rule.description),
                    location: None,
                    severity: ErrorSeverity::High,
                });
            }
        }
    }

    /// Validate required and optional fields
    fn validate_fields(&self, output: &str, format: &OutputFormat, result: &mut ValidationResult) {
        // Check required fields
        for field in &format.required_fields {
            if let Some(validator) = format.field_validators.get(field) {
                if !self.validate_field(output, field, validator, result) {
                    result.add_error(ValidationError {
                        error_type: ValidationErrorType::MissingRequiredField,
                        message: format!("Required field '{}' is missing or invalid", field),
                        location: Some(field.clone()),
                        severity: ErrorSeverity::Critical,
                    });
                }
            }
        }
    }

    /// Validate a specific field
    fn validate_field(&self, output: &str, field_name: &str, validator: &FieldValidator, result: &mut ValidationResult) -> bool {
        match validator.field_type {
            FieldType::Json => self.validate_json_field(output, field_name, validator, result),
            FieldType::ToolName => self.validate_tool_name_field(output, field_name, validator, result),
            FieldType::String => self.validate_string_field(output, field_name, validator, result),
            _ => true, // Default to valid for other types
        }
    }

    /// Validate JSON field
    fn validate_json_field(&self, output: &str, field_name: &str, validator: &FieldValidator, result: &mut ValidationResult) -> bool {
        // Extract JSON content for the field
        if let Some(json_content) = self.extract_field_content(output, field_name) {
            match self.json_parser.parse(&json_content) {
                Ok(_) => true,
                Err(_) => {
                    result.add_error(ValidationError {
                        error_type: ValidationErrorType::MalformedActionInput,
                        message: format!("Field '{}' contains invalid JSON", field_name),
                        location: Some(field_name.to_string()),
                        severity: ErrorSeverity::High,
                    });
                    false
                }
            }
        } else {
            false
        }
    }

    /// Validate tool name field
    fn validate_tool_name_field(&self, output: &str, field_name: &str, validator: &FieldValidator, result: &mut ValidationResult) -> bool {
        if let Some(tool_name) = self.extract_field_content(output, field_name) {
            if let Some(allowed_values) = &validator.allowed_values {
                if !allowed_values.contains(&tool_name) {
                    result.add_error(ValidationError {
                        error_type: ValidationErrorType::InvalidToolName,
                        message: format!("Unknown tool name: '{}'", tool_name),
                        location: Some(field_name.to_string()),
                        severity: ErrorSeverity::High,
                    });
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Validate string field
    fn validate_string_field(&self, output: &str, field_name: &str, validator: &FieldValidator, result: &mut ValidationResult) -> bool {
        if let Some(content) = self.extract_field_content(output, field_name) {
            // Check length constraints
            if let Some(min_len) = validator.min_length {
                if content.len() < min_len {
                    result.add_warning(ValidationWarning {
                        warning_type: ValidationWarningType::SuboptimalFormat,
                        message: format!("Field '{}' is shorter than recommended minimum", field_name),
                        suggestion: Some(format!("Consider providing more detailed content (minimum {} characters)", min_len)),
                    });
                }
            }
            
            if let Some(max_len) = validator.max_length {
                if content.len() > max_len {
                    result.add_warning(ValidationWarning {
                        warning_type: ValidationWarningType::PerformanceImpact,
                        message: format!("Field '{}' exceeds recommended maximum length", field_name),
                        suggestion: Some(format!("Consider shortening content (maximum {} characters)", max_len)),
                    });
                }
            }
            
            // Check pattern matching
            if let Some(pattern) = &validator.pattern {
                if !pattern.is_match(&content) {
                    result.add_error(ValidationError {
                        error_type: ValidationErrorType::InvalidFormat,
                        message: format!("Field '{}' does not match required pattern", field_name),
                        location: Some(field_name.to_string()),
                        severity: ErrorSeverity::Medium,
                    });
                    return false;
                }
            }
            
            true
        } else {
            false
        }
    }

    /// Extract content for a specific field from output
    fn extract_field_content(&self, output: &str, field_name: &str) -> Option<String> {
        let pattern = format!(r"{}:\s*(.+?)(?:\n|$)", regex::escape(field_name));
        let regex = Regex::new(&pattern).ok()?;
        
        regex.captures(output)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().trim().to_string())
    }

    /// Calculate confidence score based on validation results
    fn calculate_confidence(&self, result: &ValidationResult) -> f64 {
        let error_penalty = result.errors.iter()
            .map(|e| match e.severity {
                ErrorSeverity::Critical => 0.4,
                ErrorSeverity::High => 0.2,
                ErrorSeverity::Medium => 0.1,
                ErrorSeverity::Low => 0.05,
            })
            .sum::<f64>();
        
        let warning_penalty = result.warnings.len() as f64 * 0.02;
        
        (1.0 - error_penalty - warning_penalty).max(0.0)
    }

    /// Generate suggested fixes based on validation errors
    fn generate_fixes(&self, result: &ValidationResult, output: &str) -> Vec<String> {
        let mut fixes = Vec::new();
        
        for error in &result.errors {
            match error.error_type {
                ValidationErrorType::MissingRequiredField => {
                    if let Some(field) = &error.location {
                        fixes.push(format!("Add missing '{}:' field to the output", field));
                    }
                }
                ValidationErrorType::MalformedActionInput => {
                    fixes.push("Fix JSON syntax in Action Input field".to_string());
                }
                ValidationErrorType::InvalidToolName => {
                    fixes.push("Use a valid tool name from the available tools list".to_string());
                }
                ValidationErrorType::InvalidFormat => {
                    fixes.push("Follow the exact format specified in the prompt".to_string());
                }
                _ => {}
            }
        }
        
        fixes
    }

    /// Register default formats for common agent types
    fn register_default_formats(&mut self) {
        // ReAct format
        let react_format = OutputFormat {
            format_type: FormatType::ReAct,
            required_fields: vec!["Thought".to_string(), "Action".to_string(), "Action Input".to_string()],
            optional_fields: vec!["Final Answer".to_string()],
            field_validators: {
                let mut validators = HashMap::new();
                validators.insert("Action Input".to_string(), FieldValidator {
                    field_type: FieldType::Json,
                    pattern: None,
                    min_length: Some(2),
                    max_length: Some(1000),
                    allowed_values: None,
                });
                validators.insert("Action".to_string(), FieldValidator {
                    field_type: FieldType::ToolName,
                    pattern: None,
                    min_length: Some(1),
                    max_length: Some(50),
                    allowed_values: None, // Will be set dynamically
                });
                validators
            },
            structure_rules: vec![
                StructureRule {
                    rule_type: StructureRuleType::StartsWith,
                    description: "Must start with 'Thought:'".to_string(),
                    validator: |output| output.trim_start().starts_with("Thought:"),
                },
            ],
        };
        
        self.formats.insert(FormatType::ReAct, react_format);
    }
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            confidence_score: 1.0,
            suggested_fixes: Vec::new(),
        }
    }

    pub fn error(message: String) -> Self {
        let mut result = Self::new();
        result.is_valid = false;
        result.add_error(ValidationError {
            error_type: ValidationErrorType::UnexpectedContent,
            message,
            location: None,
            severity: ErrorSeverity::Critical,
        });
        result
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
}

impl Default for OutputValidator {
    fn default() -> Self {
        Self::new()
    }
}
