use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    agent::AgentError,
    prompt::PromptArgs,
    schemas::agent::AgentAction,
};

use super::{
    AgentCapability, PlanningEnhancer, ActionProcessor, ActionContext, ProcessedResult,
    ToolProvider,
};

/// Trait for code execution capabilities that can execute and evaluate code snippets
#[async_trait]
pub trait CodeExecutionCapability: AgentCapability + PlanningEnhancer + ActionProcessor + ToolProvider {
    /// Execute code in the specified language
    async fn execute_code(
        &self,
        code: &str,
        language: &str,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult, AgentError>;
    
    /// Validate code syntax and basic structure
    async fn validate_code(
        &self,
        code: &str,
        language: &str,
    ) -> Result<ValidationResult, AgentError>;
    
    /// Get information about the execution environment
    async fn get_execution_environment(&self) -> Result<EnvironmentInfo, AgentError>;
    
    /// List supported programming languages
    fn get_supported_languages(&self) -> Vec<String>;
    
    /// Get security restrictions for code execution
    fn get_security_restrictions(&self) -> SecurityRestrictions;
    
    /// Execute code with timeout and resource limits
    async fn execute_code_safe(
        &self,
        code: &str,
        language: &str,
        context: &ExecutionContext,
        timeout: Duration,
        memory_limit: Option<u64>,
    ) -> Result<ExecutionResult, AgentError>;
}

/// Context for code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Working directory for execution
    pub working_directory: Option<String>,
    /// Environment variables
    pub environment_variables: HashMap<String, String>,
    /// Input data for the code
    pub input_data: Option<String>,
    /// Additional files or resources
    pub resources: HashMap<String, String>,
    /// Execution mode (sandbox, local, etc.)
    pub execution_mode: ExecutionMode,
    /// Security context
    pub security_context: SecurityContext,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            working_directory: None,
            environment_variables: HashMap::new(),
            input_data: None,
            resources: HashMap::new(),
            execution_mode: ExecutionMode::Sandbox,
            security_context: SecurityContext::default(),
        }
    }
    
    pub fn with_working_directory(mut self, dir: String) -> Self {
        self.working_directory = Some(dir);
        self
    }
    
    pub fn with_environment_variable(mut self, key: String, value: String) -> Self {
        self.environment_variables.insert(key, value);
        self
    }
    
    pub fn with_input_data(mut self, data: String) -> Self {
        self.input_data = Some(data);
        self
    }
}

/// Mode of code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute in a sandboxed environment
    Sandbox,
    /// Execute in a container
    Container,
    /// Execute locally (less secure)
    Local,
    /// Execute remotely
    Remote,
}

/// Security context for code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Whether network access is allowed
    pub allow_network: bool,
    /// Whether file system access is allowed
    pub allow_file_system: bool,
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum memory usage
    pub max_memory_mb: u64,
    /// Allowed system calls
    pub allowed_syscalls: Vec<String>,
    /// Blocked imports/modules
    pub blocked_imports: Vec<String>,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_file_system: false,
            max_execution_time: Duration::from_secs(30),
            max_memory_mb: 128,
            allowed_syscalls: Vec::new(),
            blocked_imports: vec![
                "os".to_string(),
                "subprocess".to_string(),
                "sys".to_string(),
                "socket".to_string(),
            ],
        }
    }
}

/// Result of code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Standard output from the execution
    pub stdout: String,
    /// Standard error from the execution
    pub stderr: String,
    /// Exit code (0 for success)
    pub exit_code: i32,
    /// Execution time
    pub execution_time: Duration,
    /// Memory usage in bytes
    pub memory_usage: Option<u64>,
    /// Whether execution was successful
    pub success: bool,
    /// Any errors that occurred
    pub errors: Vec<String>,
    /// Return value (if applicable)
    pub return_value: Option<Value>,
    /// Execution metadata
    pub metadata: HashMap<String, Value>,
}

impl ExecutionResult {
    pub fn success(stdout: String, execution_time: Duration) -> Self {
        Self {
            stdout,
            stderr: String::new(),
            exit_code: 0,
            execution_time,
            memory_usage: None,
            success: true,
            errors: Vec::new(),
            return_value: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn failure(stderr: String, exit_code: i32, execution_time: Duration) -> Self {
        Self {
            stdout: String::new(),
            stderr,
            exit_code,
            execution_time,
            memory_usage: None,
            success: false,
            errors: Vec::new(),
            return_value: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_memory_usage(mut self, memory: u64) -> Self {
        self.memory_usage = Some(memory);
        self
    }
    
    pub fn with_return_value(mut self, value: Value) -> Self {
        self.return_value = Some(value);
        self
    }
}

/// Result of code validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the code is valid
    pub is_valid: bool,
    /// Syntax errors
    pub syntax_errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Security issues
    pub security_issues: Vec<SecurityIssue>,
    /// Suggestions for improvement
    pub suggestions: Vec<String>,
    /// Confidence in the validation (0.0 to 1.0)
    pub confidence: f64,
}

/// Security issue found in code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    /// Type of security issue
    pub issue_type: SecurityIssueType,
    /// Description of the issue
    pub description: String,
    /// Severity level
    pub severity: SecuritySeverity,
    /// Line number where the issue occurs
    pub line_number: Option<usize>,
    /// Suggested fix
    pub suggested_fix: Option<String>,
}

/// Types of security issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityIssueType {
    /// Dangerous import or module usage
    DangerousImport,
    /// File system access
    FileSystemAccess,
    /// Network access
    NetworkAccess,
    /// System command execution
    SystemCommand,
    /// Potential code injection
    CodeInjection,
    /// Resource exhaustion risk
    ResourceExhaustion,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Information about the execution environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Available interpreters/compilers
    pub available_interpreters: HashMap<String, String>,
    /// System information
    pub system_info: SystemInfo,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Security features
    pub security_features: Vec<String>,
}

/// System information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system
    pub os: String,
    /// Architecture
    pub arch: String,
    /// Available memory
    pub available_memory_mb: u64,
    /// CPU cores
    pub cpu_cores: usize,
}

/// Resource limits for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum execution time
    pub max_execution_time: Duration,
    /// Maximum memory usage
    pub max_memory_mb: u64,
    /// Maximum output size
    pub max_output_size_kb: u64,
    /// Maximum file size
    pub max_file_size_kb: u64,
}

/// Security restrictions for code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRestrictions {
    /// Whether sandboxing is enabled
    pub sandboxing_enabled: bool,
    /// Blocked imports/modules
    pub blocked_imports: Vec<String>,
    /// Blocked functions
    pub blocked_functions: Vec<String>,
    /// Network access policy
    pub network_policy: NetworkPolicy,
    /// File system access policy
    pub filesystem_policy: FilesystemPolicy,
}

/// Network access policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkPolicy {
    /// No network access allowed
    Blocked,
    /// Only specific domains allowed
    Whitelist(Vec<String>),
    /// All domains except specific ones
    Blacklist(Vec<String>),
    /// Full network access
    Allowed,
}

/// File system access policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilesystemPolicy {
    /// No file system access
    Blocked,
    /// Read-only access to specific directories
    ReadOnly(Vec<String>),
    /// Read-write access to specific directories
    ReadWrite(Vec<String>),
    /// Full file system access
    Full,
}

/// Default implementation of code execution capability
pub struct DefaultCodeExecutionCapability {
    /// Supported languages
    supported_languages: Vec<String>,
    /// Security restrictions
    security_restrictions: SecurityRestrictions,
    /// Execution history
    execution_history: Vec<ExecutionRecord>,
    /// Maximum history size
    max_history_size: usize,
}

/// Record of a code execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique identifier
    pub id: String,
    /// Code that was executed
    pub code: String,
    /// Language used
    pub language: String,
    /// Execution result
    pub result: ExecutionResult,
    /// When the execution occurred
    pub timestamp: SystemTime,
}

impl DefaultCodeExecutionCapability {
    /// Create a new default code execution capability
    pub fn new() -> Self {
        Self {
            supported_languages: vec![
                "python".to_string(),
                "javascript".to_string(),
                "bash".to_string(),
                "sql".to_string(),
            ],
            security_restrictions: SecurityRestrictions {
                sandboxing_enabled: true,
                blocked_imports: vec![
                    "os".to_string(),
                    "subprocess".to_string(),
                    "sys".to_string(),
                    "socket".to_string(),
                    "urllib".to_string(),
                    "requests".to_string(),
                ],
                blocked_functions: vec![
                    "eval".to_string(),
                    "exec".to_string(),
                    "compile".to_string(),
                    "__import__".to_string(),
                ],
                network_policy: NetworkPolicy::Blocked,
                filesystem_policy: FilesystemPolicy::Blocked,
            },
            execution_history: Vec::new(),
            max_history_size: 100,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(
        supported_languages: Vec<String>,
        security_restrictions: SecurityRestrictions,
    ) -> Self {
        Self {
            supported_languages,
            security_restrictions,
            execution_history: Vec::new(),
            max_history_size: 100,
        }
    }
    
    /// Add an execution record to history
    fn add_execution_record(&mut self, record: ExecutionRecord) {
        self.execution_history.push(record);
        
        // Keep history size under limit
        if self.execution_history.len() > self.max_history_size {
            self.execution_history.remove(0);
        }
    }
    
    /// Generate a unique execution ID
    fn generate_execution_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("exec_{:x}", timestamp)
    }

    /// Validate Python code for security issues
    fn validate_python_code(&self, code: &str) -> ValidationResult {
        let mut syntax_errors = Vec::new();
        let warnings = Vec::new();
        let mut security_issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check for blocked imports
        for (line_num, line) in code.lines().enumerate() {
            let line_trimmed = line.trim();

            // Check for dangerous imports
            for blocked_import in &self.security_restrictions.blocked_imports {
                if line_trimmed.contains(&format!("import {}", blocked_import)) ||
                   line_trimmed.contains(&format!("from {}", blocked_import)) {
                    security_issues.push(SecurityIssue {
                        issue_type: SecurityIssueType::DangerousImport,
                        description: format!("Blocked import detected: {}", blocked_import),
                        severity: SecuritySeverity::High,
                        line_number: Some(line_num + 1),
                        suggested_fix: Some("Remove or replace with a safer alternative".to_string()),
                    });
                }
            }

            // Check for blocked functions
            for blocked_func in &self.security_restrictions.blocked_functions {
                if line_trimmed.contains(&format!("{}(", blocked_func)) {
                    security_issues.push(SecurityIssue {
                        issue_type: SecurityIssueType::CodeInjection,
                        description: format!("Dangerous function detected: {}", blocked_func),
                        severity: SecuritySeverity::Critical,
                        line_number: Some(line_num + 1),
                        suggested_fix: Some("Avoid using dynamic code execution".to_string()),
                    });
                }
            }

            // Check for file operations
            if line_trimmed.contains("open(") || line_trimmed.contains("file(") {
                security_issues.push(SecurityIssue {
                    issue_type: SecurityIssueType::FileSystemAccess,
                    description: "File system access detected".to_string(),
                    severity: SecuritySeverity::Medium,
                    line_number: Some(line_num + 1),
                    suggested_fix: Some("Ensure file access is necessary and safe".to_string()),
                });
            }
        }

        // Basic syntax validation (simplified)
        let has_syntax_errors = code.contains("SyntaxError") ||
                               code.lines().any(|line| line.trim().ends_with(":") && !line.trim().starts_with("#"));

        if has_syntax_errors {
            syntax_errors.push("Potential syntax errors detected".to_string());
        }

        // Generate suggestions
        if code.lines().count() > 50 {
            suggestions.push("Consider breaking down large code blocks into smaller functions".to_string());
        }

        if !code.contains("def ") && code.lines().count() > 10 {
            suggestions.push("Consider organizing code into functions for better readability".to_string());
        }

        let confidence = if security_issues.is_empty() && syntax_errors.is_empty() {
            0.9
        } else if security_issues.iter().any(|issue| matches!(issue.severity, SecuritySeverity::Critical)) {
            0.3
        } else {
            0.6
        };

        ValidationResult {
            is_valid: syntax_errors.is_empty() &&
                     !security_issues.iter().any(|issue| matches!(issue.severity, SecuritySeverity::Critical)),
            syntax_errors,
            warnings,
            security_issues,
            suggestions,
            confidence,
        }
    }

    /// Execute Python code in a simulated environment
    async fn execute_python_code(
        &self,
        code: &str,
        _context: &ExecutionContext,
    ) -> Result<ExecutionResult, AgentError> {
        let start_time = SystemTime::now();

        // Validate code first
        let validation = self.validate_python_code(code);
        if !validation.is_valid {
            return Ok(ExecutionResult::failure(
                format!("Code validation failed: {:?}", validation.security_issues),
                1,
                start_time.elapsed().unwrap_or(Duration::from_secs(0)),
            ));
        }

        // Simulate code execution (in a real implementation, this would use a sandbox)
        let execution_time = Duration::from_millis(100 + (code.len() as u64 * 2));

        // Simple pattern matching for common Python operations
        let mut output = String::new();

        if code.contains("print(") {
            // Extract print statements (simplified)
            for line in code.lines() {
                if line.trim().starts_with("print(") {
                    let content = line.trim()
                        .strip_prefix("print(")
                        .and_then(|s| s.strip_suffix(")"))
                        .unwrap_or("Hello, World!");
                    output.push_str(&format!("{}\n", content.trim_matches('"').trim_matches('\'')));
                }
            }
        } else if code.contains("def ") {
            output.push_str("Function defined successfully\n");
        } else if code.contains("=") && !code.contains("==") {
            output.push_str("Variable assignment completed\n");
        } else {
            output.push_str("Code executed successfully\n");
        }

        // Check for potential errors
        if code.contains("1/0") || code.contains("division by zero") {
            return Ok(ExecutionResult::failure(
                "ZeroDivisionError: division by zero".to_string(),
                1,
                execution_time,
            ));
        }

        if code.contains("undefined_variable") {
            return Ok(ExecutionResult::failure(
                "NameError: name 'undefined_variable' is not defined".to_string(),
                1,
                execution_time,
            ));
        }

        Ok(ExecutionResult::success(output, execution_time)
            .with_memory_usage(1024 * 1024)) // 1MB simulated
    }
}

impl AgentCapability for DefaultCodeExecutionCapability {
    fn capability_name(&self) -> &'static str {
        "default_code_execution"
    }

    fn capability_description(&self) -> &'static str {
        "Default implementation of code execution capability with security restrictions"
    }
}

use std::sync::Arc;
use crate::tools::Tool;
use super::tools::{CodeExecutionTool, CodeValidationTool};

impl ToolProvider for DefaultCodeExecutionCapability {
    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![
            Arc::new(CodeExecutionTool::new(self.supported_languages.clone())),
            Arc::new(CodeValidationTool::new()),
        ]
    }
}

#[async_trait]
impl PlanningEnhancer for DefaultCodeExecutionCapability {
    async fn pre_plan(
        &self,
        _intermediate_steps: &[(AgentAction, String)],
        inputs: &mut PromptArgs,
    ) -> Result<(), AgentError> {
        // Add code execution context
        inputs.insert(
            "code_execution_available".to_string(),
            serde_json::json!(true),
        );

        inputs.insert(
            "supported_languages".to_string(),
            serde_json::json!(self.supported_languages),
        );

        // Add recent execution history context
        if !self.execution_history.is_empty() {
            let recent_executions: Vec<Value> = self.execution_history
                .iter()
                .rev()
                .take(3)
                .map(|record| serde_json::json!({
                    "language": record.language,
                    "success": record.result.success,
                    "execution_time_ms": record.result.execution_time.as_millis(),
                }))
                .collect();

            inputs.insert(
                "recent_code_executions".to_string(),
                serde_json::json!(recent_executions),
            );
        }

        Ok(())
    }
}

#[async_trait]
impl ActionProcessor for DefaultCodeExecutionCapability {
    async fn process_action_result(
        &self,
        action: &AgentAction,
        result: &str,
        _context: &ActionContext,
    ) -> Result<ProcessedResult, AgentError> {
        let mut processed = ProcessedResult::default();

        // Check if this action involved code execution
        if action.tool.to_lowercase().contains("code") ||
           action.tool.to_lowercase().contains("execute") ||
           action.tool.to_lowercase().contains("python") {

            // Analyze the execution result
            let success = !result.contains("Error") && !result.contains("Failed");

            processed.additional_context = Some(serde_json::json!({
                "code_execution_detected": true,
                "execution_successful": success,
                "tool_used": action.tool,
            }));

            // If execution failed, suggest improvements
            if !success {
                processed.modified_result = Some(format!(
                    "{}\n\nSuggestion: Consider validating code before execution or using error handling.",
                    result
                ));
            }
        }

        Ok(processed)
    }
}

#[async_trait]
impl CodeExecutionCapability for DefaultCodeExecutionCapability {
    async fn execute_code(
        &self,
        code: &str,
        language: &str,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult, AgentError> {
        if !self.supported_languages.contains(&language.to_lowercase()) {
            return Err(AgentError::OtherError(
                format!("Unsupported language: {}", language),
            ));
        }

        let result = match language.to_lowercase().as_str() {
            "python" => self.execute_python_code(code, context).await?,
            "javascript" => self.execute_javascript_code(code, context).await?,
            "bash" => self.execute_bash_code(code, context).await?,
            "sql" => self.execute_sql_code(code, context).await?,
            _ => return Err(AgentError::OtherError(
                format!("Language '{}' not implemented", language),
            )),
        };

        // Record the execution
        let record = ExecutionRecord {
            id: self.generate_execution_id(),
            code: code.to_string(),
            language: language.to_string(),
            result: result.clone(),
            timestamp: SystemTime::now(),
        };

        // Note: In a real implementation, you'd need mutable access to self
        // This would require using Arc<Mutex<>> or similar
        log::info!("Code execution completed: {} ({})", record.id, language);

        Ok(result)
    }

    async fn validate_code(
        &self,
        code: &str,
        language: &str,
    ) -> Result<ValidationResult, AgentError> {
        match language.to_lowercase().as_str() {
            "python" => Ok(self.validate_python_code(code)),
            "javascript" => Ok(self.validate_javascript_code(code)),
            "bash" => Ok(self.validate_bash_code(code)),
            "sql" => Ok(self.validate_sql_code(code)),
            _ => Err(AgentError::OtherError(
                format!("Validation not supported for language: {}", language),
            )),
        }
    }

    async fn get_execution_environment(&self) -> Result<EnvironmentInfo, AgentError> {
        let mut interpreters = HashMap::new();
        interpreters.insert("python".to_string(), "Python 3.9+".to_string());
        interpreters.insert("javascript".to_string(), "Node.js 16+".to_string());
        interpreters.insert("bash".to_string(), "Bash 5.0+".to_string());
        interpreters.insert("sql".to_string(), "SQLite 3.0+".to_string());

        Ok(EnvironmentInfo {
            available_interpreters: interpreters,
            system_info: SystemInfo {
                os: "Sandboxed Environment".to_string(),
                arch: "x86_64".to_string(),
                available_memory_mb: 512,
                cpu_cores: 2,
            },
            resource_limits: ResourceLimits {
                max_execution_time: Duration::from_secs(30),
                max_memory_mb: 128,
                max_output_size_kb: 1024,
                max_file_size_kb: 1024,
            },
            security_features: vec![
                "Sandboxing".to_string(),
                "Import filtering".to_string(),
                "Resource limits".to_string(),
                "Network isolation".to_string(),
            ],
        })
    }

    fn get_supported_languages(&self) -> Vec<String> {
        self.supported_languages.clone()
    }

    fn get_security_restrictions(&self) -> SecurityRestrictions {
        self.security_restrictions.clone()
    }

    async fn execute_code_safe(
        &self,
        code: &str,
        language: &str,
        context: &ExecutionContext,
        timeout: Duration,
        memory_limit: Option<u64>,
    ) -> Result<ExecutionResult, AgentError> {
        // Create a modified context with additional safety measures
        let mut safe_context = context.clone();
        safe_context.security_context.max_execution_time = timeout;
        if let Some(memory) = memory_limit {
            safe_context.security_context.max_memory_mb = memory / (1024 * 1024);
        }

        // Execute with the safe context
        self.execute_code(code, language, &safe_context).await
    }
}

impl DefaultCodeExecutionCapability {
    /// Execute JavaScript code (simulated)
    async fn execute_javascript_code(
        &self,
        code: &str,
        _context: &ExecutionContext,
    ) -> Result<ExecutionResult, AgentError> {
        let _start_time = SystemTime::now();
        let execution_time = Duration::from_millis(80 + (code.len() as u64));

        // Simple simulation
        let mut output = String::new();

        if code.contains("console.log(") {
            for line in code.lines() {
                if line.trim().contains("console.log(") {
                    output.push_str("JavaScript output\n");
                }
            }
        } else {
            output.push_str("JavaScript code executed\n");
        }

        Ok(ExecutionResult::success(output, execution_time))
    }

    /// Execute Bash code (simulated)
    async fn execute_bash_code(
        &self,
        code: &str,
        _context: &ExecutionContext,
    ) -> Result<ExecutionResult, AgentError> {
        let _start_time = SystemTime::now();
        let execution_time = Duration::from_millis(50 + (code.len() as u64));

        // Check for dangerous commands
        let dangerous_commands = ["rm -rf", "sudo", "chmod 777", "dd if="];
        for cmd in dangerous_commands {
            if code.contains(cmd) {
                return Ok(ExecutionResult::failure(
                    format!("Dangerous command blocked: {}", cmd),
                    1,
                    execution_time,
                ));
            }
        }

        let output = if code.contains("echo") {
            "Bash echo output\n".to_string()
        } else if code.contains("ls") {
            "file1.txt\nfile2.txt\ndirectory/\n".to_string()
        } else {
            "Bash command executed\n".to_string()
        };

        Ok(ExecutionResult::success(output, execution_time))
    }

    /// Execute SQL code (simulated)
    async fn execute_sql_code(
        &self,
        code: &str,
        _context: &ExecutionContext,
    ) -> Result<ExecutionResult, AgentError> {
        let _start_time = SystemTime::now();
        let execution_time = Duration::from_millis(30 + (code.len() as u64));

        // Check for dangerous SQL operations
        let dangerous_operations = ["DROP TABLE", "DELETE FROM", "TRUNCATE", "ALTER TABLE"];
        for op in dangerous_operations {
            if code.to_uppercase().contains(op) {
                return Ok(ExecutionResult::failure(
                    format!("Dangerous SQL operation blocked: {}", op),
                    1,
                    execution_time,
                ));
            }
        }

        let output = if code.to_uppercase().contains("SELECT") {
            "Query executed successfully\nRows returned: 5\n".to_string()
        } else if code.to_uppercase().contains("INSERT") {
            "1 row inserted\n".to_string()
        } else if code.to_uppercase().contains("UPDATE") {
            "2 rows updated\n".to_string()
        } else {
            "SQL statement executed\n".to_string()
        };

        Ok(ExecutionResult::success(output, execution_time))
    }

    /// Validate JavaScript code
    fn validate_javascript_code(&self, code: &str) -> ValidationResult {
        let mut security_issues = Vec::new();
        let warnings = Vec::new();

        // Check for dangerous patterns
        if code.contains("eval(") {
            security_issues.push(SecurityIssue {
                issue_type: SecurityIssueType::CodeInjection,
                description: "Use of eval() detected".to_string(),
                severity: SecuritySeverity::High,
                line_number: None,
                suggested_fix: Some("Avoid using eval()".to_string()),
            });
        }

        if code.contains("require('fs')") || code.contains("require(\"fs\")") {
            security_issues.push(SecurityIssue {
                issue_type: SecurityIssueType::FileSystemAccess,
                description: "File system access detected".to_string(),
                severity: SecuritySeverity::Medium,
                line_number: None,
                suggested_fix: Some("Ensure file access is necessary".to_string()),
            });
        }

        ValidationResult {
            is_valid: security_issues.iter().all(|issue| !matches!(issue.severity, SecuritySeverity::Critical)),
            syntax_errors: Vec::new(),
            warnings,
            security_issues,
            suggestions: Vec::new(),
            confidence: 0.8,
        }
    }

    /// Validate Bash code
    fn validate_bash_code(&self, code: &str) -> ValidationResult {
        let mut security_issues = Vec::new();

        let dangerous_patterns = [
            ("rm -rf", SecuritySeverity::Critical),
            ("sudo", SecuritySeverity::High),
            ("chmod 777", SecuritySeverity::High),
            ("wget", SecuritySeverity::Medium),
            ("curl", SecuritySeverity::Medium),
        ];

        for (pattern, severity) in dangerous_patterns {
            if code.contains(pattern) {
                security_issues.push(SecurityIssue {
                    issue_type: SecurityIssueType::SystemCommand,
                    description: format!("Dangerous command detected: {}", pattern),
                    severity,
                    line_number: None,
                    suggested_fix: Some("Use safer alternatives".to_string()),
                });
            }
        }

        ValidationResult {
            is_valid: security_issues.iter().all(|issue| !matches!(issue.severity, SecuritySeverity::Critical)),
            syntax_errors: Vec::new(),
            warnings: Vec::new(),
            security_issues,
            suggestions: Vec::new(),
            confidence: 0.9,
        }
    }

    /// Validate SQL code
    fn validate_sql_code(&self, code: &str) -> ValidationResult {
        let mut security_issues = Vec::new();
        let code_upper = code.to_uppercase();

        let dangerous_operations = [
            ("DROP", SecuritySeverity::Critical),
            ("DELETE", SecuritySeverity::High),
            ("TRUNCATE", SecuritySeverity::High),
            ("ALTER", SecuritySeverity::Medium),
        ];

        for (operation, severity) in dangerous_operations {
            if code_upper.contains(operation) {
                security_issues.push(SecurityIssue {
                    issue_type: SecurityIssueType::SystemCommand,
                    description: format!("Potentially dangerous SQL operation: {}", operation),
                    severity,
                    line_number: None,
                    suggested_fix: Some("Ensure this operation is intended".to_string()),
                });
            }
        }

        ValidationResult {
            is_valid: security_issues.iter().all(|issue| !matches!(issue.severity, SecuritySeverity::Critical)),
            syntax_errors: Vec::new(),
            warnings: Vec::new(),
            security_issues,
            suggestions: Vec::new(),
            confidence: 0.85,
        }
    }
}

impl Default for DefaultCodeExecutionCapability {
    fn default() -> Self {
        Self::new()
    }
}
