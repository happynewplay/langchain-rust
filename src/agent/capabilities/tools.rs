use std::error::Error;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::tools::Tool;

/// Tool for executing code through the code execution capability
pub struct CodeExecutionTool {
    supported_languages: Vec<String>,
}

impl CodeExecutionTool {
    pub fn new(supported_languages: Vec<String>) -> Self {
        Self {
            supported_languages,
        }
    }
}

#[async_trait]
impl Tool for CodeExecutionTool {
    fn name(&self) -> String {
        "code_executor".to_string()
    }
    
    fn description(&self) -> String {
        format!(
            "Execute code in supported languages: {}. Provide code and language as input.",
            self.supported_languages.join(", ")
        )
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "The code to execute"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language",
                    "enum": self.supported_languages
                }
            },
            "required": ["code", "language"]
        })
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let code = input["code"]
            .as_str()
            .ok_or("Code is required")?;
        let language = input["language"]
            .as_str()
            .ok_or("Language is required")?;
        
        if !self.supported_languages.contains(&language.to_string()) {
            return Err(format!("Unsupported language: {}", language).into());
        }
        
        // In a real implementation, this would use the actual code execution capability
        // For now, we'll simulate execution
        let result = match language {
            "python" => {
                if code.contains("print(") {
                    "Python code executed successfully\nOutput: Hello, World!"
                } else {
                    "Python code executed successfully"
                }
            }
            "javascript" => {
                if code.contains("console.log(") {
                    "JavaScript code executed successfully\nOutput: Hello, World!"
                } else {
                    "JavaScript code executed successfully"
                }
            }
            "bash" => {
                if code.contains("echo") {
                    "Bash command executed successfully\nOutput: Hello, World!"
                } else {
                    "Bash command executed successfully"
                }
            }
            "sql" => {
                if code.to_uppercase().contains("SELECT") {
                    "SQL query executed successfully\nRows returned: 5"
                } else {
                    "SQL statement executed successfully"
                }
            }
            _ => "Code executed successfully",
        };
        
        Ok(result.to_string())
    }
}

/// Tool for validating code before execution
pub struct CodeValidationTool;

impl CodeValidationTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for CodeValidationTool {
    fn name(&self) -> String {
        "code_validator".to_string()
    }
    
    fn description(&self) -> String {
        "Validate code for syntax errors and security issues before execution".to_string()
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "The code to validate"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language",
                    "enum": ["python", "javascript", "bash", "sql"]
                }
            },
            "required": ["code", "language"]
        })
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let code = input["code"]
            .as_str()
            .ok_or("Code is required")?;
        let language = input["language"]
            .as_str()
            .ok_or("Language is required")?;
        
        // Simple validation simulation
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        
        match language {
            "python" => {
                if code.contains("eval(") || code.contains("exec(") {
                    issues.push("Security issue: Dynamic code execution detected");
                }
                if code.contains("import os") || code.contains("import subprocess") {
                    issues.push("Security issue: Dangerous import detected");
                }
                if code.lines().any(|line| line.trim().ends_with(":") && !line.trim().starts_with("#")) {
                    warnings.push("Warning: Incomplete code block detected");
                }
            }
            "javascript" => {
                if code.contains("eval(") {
                    issues.push("Security issue: eval() usage detected");
                }
                if code.contains("require('fs')") {
                    warnings.push("Warning: File system access detected");
                }
            }
            "bash" => {
                if code.contains("rm -rf") {
                    issues.push("Critical: Dangerous deletion command detected");
                }
                if code.contains("sudo") {
                    issues.push("Security issue: Privilege escalation detected");
                }
            }
            "sql" => {
                if code.to_uppercase().contains("DROP TABLE") {
                    issues.push("Critical: Table deletion detected");
                }
                if code.to_uppercase().contains("DELETE FROM") {
                    warnings.push("Warning: Data deletion detected");
                }
            }
            _ => {
                return Err(format!("Validation not supported for language: {}", language).into());
            }
        }
        
        let mut result = String::new();
        
        if issues.is_empty() && warnings.is_empty() {
            result.push_str("✅ Code validation passed\nNo issues detected");
        } else {
            result.push_str("⚠️ Code validation completed with issues:\n\n");
            
            if !issues.is_empty() {
                result.push_str("🚨 Issues:\n");
                for issue in issues {
                    result.push_str(&format!("  - {}\n", issue));
                }
                result.push('\n');
            }
            
            if !warnings.is_empty() {
                result.push_str("⚠️ Warnings:\n");
                for warning in warnings {
                    result.push_str(&format!("  - {}\n", warning));
                }
            }
        }
        
        Ok(result)
    }
}

/// Tool for task planning and decomposition
pub struct TaskPlannerTool;

impl TaskPlannerTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TaskPlannerTool {
    fn name(&self) -> String {
        "task_planner".to_string()
    }
    
    fn description(&self) -> String {
        "Break down complex tasks into manageable subtasks with dependencies".to_string()
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The main task to decompose"
                },
                "context": {
                    "type": "string",
                    "description": "Additional context or constraints",
                    "default": ""
                }
            },
            "required": ["task"]
        })
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let task = input["task"]
            .as_str()
            .ok_or("Task is required")?;
        let context = input["context"]
            .as_str()
            .unwrap_or("");
        
        // Simple task decomposition
        let task_lower = task.to_lowercase();
        let mut subtasks = Vec::new();
        
        if task_lower.contains("research") || task_lower.contains("find") {
            subtasks.push("1. Define research scope and objectives");
            subtasks.push("2. Identify relevant sources and databases");
            subtasks.push("3. Gather and collect information");
            subtasks.push("4. Analyze and synthesize findings");
            subtasks.push("5. Document results and conclusions");
        } else if task_lower.contains("write") || task_lower.contains("create") {
            subtasks.push("1. Plan structure and outline");
            subtasks.push("2. Research background information");
            subtasks.push("3. Create initial draft");
            subtasks.push("4. Review and revise content");
            subtasks.push("5. Finalize and format");
        } else if task_lower.contains("analyze") || task_lower.contains("evaluate") {
            subtasks.push("1. Define analysis criteria and metrics");
            subtasks.push("2. Collect and prepare data");
            subtasks.push("3. Apply analysis methods");
            subtasks.push("4. Interpret results");
            subtasks.push("5. Present findings and recommendations");
        } else {
            // Generic decomposition
            subtasks.push("1. Understand the requirements");
            subtasks.push("2. Plan the approach");
            subtasks.push("3. Execute the main work");
            subtasks.push("4. Review and validate results");
            subtasks.push("5. Finalize and deliver");
        }
        
        let mut result = format!("📋 Task Plan for: {}\n\n", task);
        
        if !context.is_empty() {
            result.push_str(&format!("Context: {}\n\n", context));
        }
        
        result.push_str("Subtasks:\n");
        for subtask in subtasks {
            result.push_str(&format!("  {}\n", subtask));
        }
        
        result.push_str("\n💡 Tip: Execute subtasks in order, as they may have dependencies.");
        
        Ok(result)
    }
}

/// Tool for reflection and self-evaluation
pub struct ReflectionTool;

impl ReflectionTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReflectionTool {
    fn name(&self) -> String {
        "reflection_analyzer".to_string()
    }
    
    fn description(&self) -> String {
        "Analyze past actions and outcomes to generate insights and improvements".to_string()
    }
    
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "The action that was taken"
                },
                "result": {
                    "type": "string",
                    "description": "The result or outcome of the action"
                },
                "goal": {
                    "type": "string",
                    "description": "The original goal or objective"
                }
            },
            "required": ["action", "result", "goal"]
        })
    }
    
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let action = input["action"]
            .as_str()
            .ok_or("Action is required")?;
        let result = input["result"]
            .as_str()
            .ok_or("Result is required")?;
        let goal = input["goal"]
            .as_str()
            .ok_or("Goal is required")?;
        
        let success = !result.to_lowercase().contains("error") && 
                     !result.to_lowercase().contains("failed");
        
        let mut reflection = format!("🤔 Reflection Analysis\n\n");
        reflection.push_str(&format!("Goal: {}\n", goal));
        reflection.push_str(&format!("Action: {}\n", action));
        reflection.push_str(&format!("Result: {}\n\n", result));
        
        if success {
            reflection.push_str("✅ Analysis: Action was successful\n\n");
            reflection.push_str("💡 Insights:\n");
            reflection.push_str("  - This approach worked well for this type of task\n");
            reflection.push_str("  - Consider using similar methods for related goals\n");
            reflection.push_str("  - The action aligned well with the objective\n\n");
            reflection.push_str("📈 Recommendations:\n");
            reflection.push_str("  - Document this successful pattern for future use\n");
            reflection.push_str("  - Consider optimizing the approach for efficiency\n");
        } else {
            reflection.push_str("❌ Analysis: Action was not successful\n\n");
            reflection.push_str("💡 Insights:\n");
            reflection.push_str("  - This approach may not be suitable for this task type\n");
            reflection.push_str("  - Consider alternative methods or tools\n");
            reflection.push_str("  - Review the goal to ensure it's achievable\n\n");
            reflection.push_str("📈 Recommendations:\n");
            reflection.push_str("  - Try a different approach or tool\n");
            reflection.push_str("  - Break down the goal into smaller steps\n");
            reflection.push_str("  - Seek additional information or resources\n");
        }
        
        Ok(reflection)
    }
}
