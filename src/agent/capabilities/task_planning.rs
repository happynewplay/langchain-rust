use std::collections::{HashMap, HashSet};
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
};

/// Trait for task planning capabilities that break down complex goals into executable sub-tasks
#[async_trait]
pub trait TaskPlanningCapability: AgentCapability + PlanningEnhancer + ActionProcessor {
    /// Decompose a complex task into a structured plan
    async fn decompose_task(
        &self,
        task: &str,
        context: &PlanningContext,
    ) -> Result<TaskPlan, AgentError>;
    
    /// Update an existing plan based on feedback or new information
    async fn update_plan(
        &self,
        plan: &mut TaskPlan,
        feedback: &PlanFeedback,
    ) -> Result<(), AgentError>;
    
    /// Get the next subtask to execute from the plan
    async fn get_next_subtask(&self, plan: &TaskPlan) -> Result<Option<SubTask>, AgentError>;
    
    /// Mark a subtask as completed and update dependencies
    async fn complete_subtask(
        &self,
        plan: &mut TaskPlan,
        subtask_id: &str,
        result: &str,
    ) -> Result<(), AgentError>;
    
    /// Get the current progress of a plan
    async fn get_plan_progress(&self, plan: &TaskPlan) -> Result<PlanProgress, AgentError>;
    
    /// Validate that a plan is feasible and well-formed
    async fn validate_plan(&self, plan: &TaskPlan) -> Result<PlanValidation, AgentError>;
}

/// Context information for task planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningContext {
    /// Available tools for task execution
    pub available_tools: Vec<String>,
    /// Time constraints
    pub time_constraints: Option<Duration>,
    /// Resource constraints
    pub resource_constraints: HashMap<String, Value>,
    /// Previous planning history
    pub planning_history: Vec<TaskPlan>,
    /// Current environment state
    pub environment_state: HashMap<String, Value>,
    /// User preferences or requirements
    pub preferences: HashMap<String, Value>,
}

impl PlanningContext {
    pub fn new(available_tools: Vec<String>) -> Self {
        Self {
            available_tools,
            time_constraints: None,
            resource_constraints: HashMap::new(),
            planning_history: Vec::new(),
            environment_state: HashMap::new(),
            preferences: HashMap::new(),
        }
    }
    
    pub fn with_time_constraint(mut self, duration: Duration) -> Self {
        self.time_constraints = Some(duration);
        self
    }
    
    pub fn with_resource_constraint(mut self, resource: String, limit: Value) -> Self {
        self.resource_constraints.insert(resource, limit);
        self
    }
}

/// A structured plan for achieving a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    /// Unique identifier for this plan
    pub id: String,
    /// Main goal this plan aims to achieve
    pub main_goal: String,
    /// List of subtasks
    pub subtasks: Vec<SubTask>,
    /// Dependencies between subtasks
    pub dependencies: HashMap<String, Vec<String>>,
    /// Estimated total completion time
    pub estimated_completion_time: Option<Duration>,
    /// Plan metadata
    pub metadata: HashMap<String, Value>,
    /// When this plan was created
    pub created_at: SystemTime,
    /// Current status of the plan
    pub status: PlanStatus,
}

impl TaskPlan {
    pub fn new(id: String, main_goal: String) -> Self {
        Self {
            id,
            main_goal,
            subtasks: Vec::new(),
            dependencies: HashMap::new(),
            estimated_completion_time: None,
            metadata: HashMap::new(),
            created_at: SystemTime::now(),
            status: PlanStatus::Created,
        }
    }
    
    pub fn add_subtask(&mut self, subtask: SubTask) {
        self.subtasks.push(subtask);
    }
    
    pub fn add_dependency(&mut self, subtask_id: String, depends_on: Vec<String>) {
        self.dependencies.insert(subtask_id, depends_on);
    }
    
    pub fn get_subtask(&self, id: &str) -> Option<&SubTask> {
        self.subtasks.iter().find(|task| task.id == id)
    }
    
    pub fn get_subtask_mut(&mut self, id: &str) -> Option<&mut SubTask> {
        self.subtasks.iter_mut().find(|task| task.id == id)
    }
}

/// A single subtask within a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// Unique identifier for this subtask
    pub id: String,
    /// Description of what this subtask should accomplish
    pub description: String,
    /// Current status of this subtask
    pub status: TaskStatus,
    /// Tools required to complete this subtask
    pub required_tools: Vec<String>,
    /// Estimated duration for this subtask
    pub estimated_duration: Option<Duration>,
    /// Actual duration (filled when completed)
    pub actual_duration: Option<Duration>,
    /// Priority level (1-10, 10 being highest)
    pub priority: u8,
    /// Additional parameters or configuration
    pub parameters: HashMap<String, Value>,
    /// Result of the subtask (when completed)
    pub result: Option<String>,
    /// When this subtask was created
    pub created_at: SystemTime,
    /// When this subtask was completed (if applicable)
    pub completed_at: Option<SystemTime>,
}

impl SubTask {
    pub fn new(id: String, description: String) -> Self {
        Self {
            id,
            description,
            status: TaskStatus::Pending,
            required_tools: Vec::new(),
            estimated_duration: None,
            actual_duration: None,
            priority: 5,
            parameters: HashMap::new(),
            result: None,
            created_at: SystemTime::now(),
            completed_at: None,
        }
    }
    
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.required_tools = tools;
        self
    }
    
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.min(10);
        self
    }
    
    pub fn with_estimated_duration(mut self, duration: Duration) -> Self {
        self.estimated_duration = Some(duration);
        self
    }
}

/// Status of a task or plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// Task has been created but not started
    Pending,
    /// Task is currently being executed
    InProgress,
    /// Task has been completed successfully
    Completed,
    /// Task failed to complete
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task is blocked waiting for dependencies
    Blocked,
}

/// Status of an entire plan
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlanStatus {
    /// Plan has been created
    Created,
    /// Plan is being executed
    InProgress,
    /// Plan has been completed successfully
    Completed,
    /// Plan execution failed
    Failed,
    /// Plan was cancelled
    Cancelled,
    /// Plan is paused
    Paused,
}

/// Feedback for updating a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFeedback {
    /// Type of feedback
    pub feedback_type: FeedbackType,
    /// Specific subtask this feedback relates to (if applicable)
    pub subtask_id: Option<String>,
    /// Feedback message
    pub message: String,
    /// Suggested changes
    pub suggested_changes: Vec<PlanChange>,
    /// Severity of the feedback
    pub severity: FeedbackSeverity,
}

/// Types of feedback that can be provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackType {
    /// Feedback about task completion
    Completion,
    /// Feedback about task failure
    Failure,
    /// Feedback about resource constraints
    ResourceConstraint,
    /// Feedback about time constraints
    TimeConstraint,
    /// General feedback about plan quality
    PlanQuality,
    /// User feedback
    UserFeedback,
}

/// Severity levels for feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackSeverity {
    /// Informational feedback
    Info,
    /// Warning that should be addressed
    Warning,
    /// Error that must be addressed
    Error,
    /// Critical issue that requires immediate attention
    Critical,
}

/// Suggested changes to a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanChange {
    /// Add a new subtask
    AddSubtask(SubTask),
    /// Remove a subtask
    RemoveSubtask(String),
    /// Modify an existing subtask
    ModifySubtask(String, HashMap<String, Value>),
    /// Change dependencies
    UpdateDependencies(String, Vec<String>),
    /// Adjust time estimates
    AdjustTimeEstimate(String, Duration),
    /// Change priority
    ChangePriority(String, u8),
}

/// Progress information for a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanProgress {
    /// Total number of subtasks
    pub total_subtasks: usize,
    /// Number of completed subtasks
    pub completed_subtasks: usize,
    /// Number of failed subtasks
    pub failed_subtasks: usize,
    /// Number of subtasks in progress
    pub in_progress_subtasks: usize,
    /// Overall completion percentage (0.0 to 1.0)
    pub completion_percentage: f64,
    /// Estimated time remaining
    pub estimated_time_remaining: Option<Duration>,
    /// Time elapsed so far
    pub time_elapsed: Duration,
    /// Current bottlenecks or blocking issues
    pub bottlenecks: Vec<String>,
}

/// Validation result for a plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanValidation {
    /// Whether the plan is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Suggestions for improvement
    pub suggestions: Vec<String>,
    /// Feasibility score (0.0 to 1.0)
    pub feasibility_score: f64,
}

/// Default implementation of task planning capability
pub struct DefaultTaskPlanningCapability {
    /// Active plans
    active_plans: HashMap<String, TaskPlan>,
    /// Planning history
    planning_history: Vec<TaskPlan>,
    /// Configuration
    max_subtasks_per_plan: usize,
    max_active_plans: usize,
    default_priority: u8,
}

impl DefaultTaskPlanningCapability {
    /// Create a new default task planning capability
    pub fn new() -> Self {
        Self {
            active_plans: HashMap::new(),
            planning_history: Vec::new(),
            max_subtasks_per_plan: 50,
            max_active_plans: 10,
            default_priority: 5,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(
        max_subtasks_per_plan: usize,
        max_active_plans: usize,
        default_priority: u8,
    ) -> Self {
        Self {
            active_plans: HashMap::new(),
            planning_history: Vec::new(),
            max_subtasks_per_plan,
            max_active_plans,
            default_priority,
        }
    }
    
    /// Generate a unique plan ID
    fn generate_plan_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("plan_{:x}", timestamp)
    }

    /// Generate a unique subtask ID
    fn generate_subtask_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("task_{:x}", timestamp)
    }
    
    /// Simple task decomposition algorithm
    fn decompose_simple_task(&self, task: &str, _context: &PlanningContext) -> Vec<SubTask> {
        let mut subtasks = Vec::new();
        
        // This is a simplified decomposition - in practice, you'd use more sophisticated NLP/AI
        let task_lower = task.to_lowercase();
        
        if task_lower.contains("research") || task_lower.contains("find") || task_lower.contains("search") {
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Gather initial information and resources".to_string(),
                )
                .with_tools(vec!["search".to_string(), "web_search".to_string()])
                .with_priority(8)
                .with_estimated_duration(Duration::from_secs(300)),
            );
            
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Analyze and synthesize findings".to_string(),
                )
                .with_priority(7)
                .with_estimated_duration(Duration::from_secs(600)),
            );
        }
        
        if task_lower.contains("write") || task_lower.contains("create") || task_lower.contains("generate") {
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Plan content structure and outline".to_string(),
                )
                .with_priority(9)
                .with_estimated_duration(Duration::from_secs(180)),
            );
            
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Create initial draft".to_string(),
                )
                .with_priority(8)
                .with_estimated_duration(Duration::from_secs(900)),
            );
            
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Review and refine content".to_string(),
                )
                .with_priority(6)
                .with_estimated_duration(Duration::from_secs(300)),
            );
        }
        
        if task_lower.contains("analyze") || task_lower.contains("evaluate") {
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Collect and prepare data for analysis".to_string(),
                )
                .with_priority(9)
                .with_estimated_duration(Duration::from_secs(240)),
            );
            
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Perform detailed analysis".to_string(),
                )
                .with_priority(8)
                .with_estimated_duration(Duration::from_secs(720)),
            );
            
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Summarize findings and conclusions".to_string(),
                )
                .with_priority(7)
                .with_estimated_duration(Duration::from_secs(180)),
            );
        }
        
        // If no specific patterns matched, create a generic breakdown
        if subtasks.is_empty() {
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    format!("Understand and analyze the task: {}", task),
                )
                .with_priority(self.default_priority)
                .with_estimated_duration(Duration::from_secs(120)),
            );
            
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Execute the main task".to_string(),
                )
                .with_priority(self.default_priority)
                .with_estimated_duration(Duration::from_secs(600)),
            );
            
            subtasks.push(
                SubTask::new(
                    self.generate_subtask_id(),
                    "Verify and finalize results".to_string(),
                )
                .with_priority(self.default_priority - 1)
                .with_estimated_duration(Duration::from_secs(120)),
            );
        }
        
        subtasks
    }
}

impl AgentCapability for DefaultTaskPlanningCapability {
    fn capability_name(&self) -> &'static str {
        "default_task_planning"
    }

    fn capability_description(&self) -> &'static str {
        "Default implementation of task planning capability for breaking down complex goals"
    }
}

#[async_trait]
impl PlanningEnhancer for DefaultTaskPlanningCapability {
    async fn pre_plan(
        &self,
        _intermediate_steps: &[(AgentAction, String)],
        inputs: &mut PromptArgs,
    ) -> Result<(), AgentError> {
        // Add planning context to inputs
        if !self.active_plans.is_empty() {
            let active_plan_summaries: Vec<Value> = self.active_plans
                .values()
                .map(|plan| serde_json::json!({
                    "id": plan.id,
                    "goal": plan.main_goal,
                    "status": plan.status,
                    "subtasks_count": plan.subtasks.len(),
                    "completed_subtasks": plan.subtasks.iter().filter(|t| t.status == TaskStatus::Completed).count(),
                }))
                .collect();

            inputs.insert(
                "active_plans".to_string(),
                serde_json::json!(active_plan_summaries),
            );
        }

        // Add planning insights based on history
        if !self.planning_history.is_empty() {
            let recent_plans: Vec<String> = self.planning_history
                .iter()
                .rev()
                .take(3)
                .map(|plan| format!("Goal: {} (Status: {:?})", plan.main_goal, plan.status))
                .collect();

            inputs.insert(
                "planning_history".to_string(),
                serde_json::json!(recent_plans),
            );
        }

        Ok(())
    }
}

#[async_trait]
impl ActionProcessor for DefaultTaskPlanningCapability {
    async fn process_action_result(
        &self,
        action: &AgentAction,
        _result: &str,
        _context: &ActionContext,
    ) -> Result<ProcessedResult, AgentError> {
        let mut processed = ProcessedResult::default();

        // Check if this action relates to any active plans
        for plan in self.active_plans.values() {
            for subtask in &plan.subtasks {
                if subtask.status == TaskStatus::InProgress &&
                   subtask.required_tools.contains(&action.tool) {

                    // This action might be related to this subtask
                    processed.additional_context = Some(serde_json::json!({
                        "related_plan": plan.id,
                        "related_subtask": subtask.id,
                        "subtask_description": subtask.description,
                    }));
                    break;
                }
            }
        }

        Ok(processed)
    }
}

#[async_trait]
impl TaskPlanningCapability for DefaultTaskPlanningCapability {
    async fn decompose_task(
        &self,
        task: &str,
        context: &PlanningContext,
    ) -> Result<TaskPlan, AgentError> {
        if self.active_plans.len() >= self.max_active_plans {
            return Err(AgentError::OtherError(
                "Maximum number of active plans reached".to_string(),
            ));
        }

        let plan_id = self.generate_plan_id();
        let mut plan = TaskPlan::new(plan_id.clone(), task.to_string());

        // Decompose the task into subtasks
        let subtasks = self.decompose_simple_task(task, context);

        if subtasks.len() > self.max_subtasks_per_plan {
            return Err(AgentError::OtherError(
                format!("Task decomposition resulted in too many subtasks: {}", subtasks.len()),
            ));
        }

        // Add subtasks to the plan
        for subtask in subtasks {
            plan.add_subtask(subtask);
        }

        // Set up basic dependencies (sequential by default)
        if plan.subtasks.len() > 1 {
            for i in 1..plan.subtasks.len() {
                let current_id = plan.subtasks[i].id.clone();
                let previous_id = plan.subtasks[i - 1].id.clone();
                plan.add_dependency(current_id, vec![previous_id]);
            }
        }

        // Calculate estimated completion time
        let total_duration: Duration = plan.subtasks
            .iter()
            .filter_map(|task| task.estimated_duration)
            .sum();

        if total_duration > Duration::from_secs(0) {
            plan.estimated_completion_time = Some(total_duration);
        }

        // Add context metadata
        plan.metadata.insert("available_tools".to_string(), serde_json::json!(context.available_tools));
        if let Some(time_constraint) = context.time_constraints {
            plan.metadata.insert("time_constraint".to_string(), serde_json::json!(time_constraint.as_secs()));
        }

        plan.status = PlanStatus::Created;

        Ok(plan)
    }

    async fn update_plan(
        &self,
        plan: &mut TaskPlan,
        feedback: &PlanFeedback,
    ) -> Result<(), AgentError> {
        match feedback.severity {
            FeedbackSeverity::Critical | FeedbackSeverity::Error => {
                // Handle critical feedback immediately
                for change in &feedback.suggested_changes {
                    match change {
                        PlanChange::AddSubtask(subtask) => {
                            plan.add_subtask(subtask.clone());
                        }
                        PlanChange::RemoveSubtask(id) => {
                            plan.subtasks.retain(|task| task.id != *id);
                            plan.dependencies.remove(id);
                            // Remove this task from other dependencies
                            for deps in plan.dependencies.values_mut() {
                                deps.retain(|dep| dep != id);
                            }
                        }
                        PlanChange::ModifySubtask(id, modifications) => {
                            if let Some(subtask) = plan.get_subtask_mut(id) {
                                for (key, value) in modifications {
                                    match key.as_str() {
                                        "description" => {
                                            if let Some(desc) = value.as_str() {
                                                subtask.description = desc.to_string();
                                            }
                                        }
                                        "priority" => {
                                            if let Some(priority) = value.as_u64() {
                                                subtask.priority = (priority as u8).min(10);
                                            }
                                        }
                                        _ => {
                                            subtask.parameters.insert(key.clone(), value.clone());
                                        }
                                    }
                                }
                            }
                        }
                        PlanChange::UpdateDependencies(id, new_deps) => {
                            plan.dependencies.insert(id.clone(), new_deps.clone());
                        }
                        PlanChange::AdjustTimeEstimate(id, duration) => {
                            if let Some(subtask) = plan.get_subtask_mut(id) {
                                subtask.estimated_duration = Some(*duration);
                            }
                        }
                        PlanChange::ChangePriority(id, priority) => {
                            if let Some(subtask) = plan.get_subtask_mut(id) {
                                subtask.priority = *priority;
                            }
                        }
                    }
                }
            }
            FeedbackSeverity::Warning => {
                // Handle warnings with more consideration
                log::warn!("Plan feedback warning: {}", feedback.message);
            }
            FeedbackSeverity::Info => {
                // Log informational feedback
                log::info!("Plan feedback: {}", feedback.message);
            }
        }

        Ok(())
    }

    async fn get_next_subtask(&self, plan: &TaskPlan) -> Result<Option<SubTask>, AgentError> {
        // Find the highest priority task that is ready to execute
        let mut ready_tasks: Vec<&SubTask> = plan.subtasks
            .iter()
            .filter(|task| {
                // Task must be pending
                if task.status != TaskStatus::Pending {
                    return false;
                }

                // Check if all dependencies are completed
                if let Some(deps) = plan.dependencies.get(&task.id) {
                    for dep_id in deps {
                        if let Some(dep_task) = plan.get_subtask(dep_id) {
                            if dep_task.status != TaskStatus::Completed {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .collect();

        // Sort by priority (highest first)
        ready_tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(ready_tasks.first().map(|&task| task.clone()))
    }

    async fn complete_subtask(
        &self,
        plan: &mut TaskPlan,
        subtask_id: &str,
        result: &str,
    ) -> Result<(), AgentError> {
        if let Some(subtask) = plan.get_subtask_mut(subtask_id) {
            subtask.status = TaskStatus::Completed;
            subtask.result = Some(result.to_string());
            subtask.completed_at = Some(SystemTime::now());

            // Calculate actual duration if the task was in progress
            if let Ok(elapsed) = subtask.completed_at.unwrap().duration_since(subtask.created_at) {
                subtask.actual_duration = Some(elapsed);
            }

            // Check if all subtasks are completed
            let all_completed = plan.subtasks
                .iter()
                .all(|task| task.status == TaskStatus::Completed || task.status == TaskStatus::Cancelled);

            if all_completed {
                plan.status = PlanStatus::Completed;
            }

            Ok(())
        } else {
            Err(AgentError::OtherError(
                format!("Subtask with ID '{}' not found", subtask_id),
            ))
        }
    }

    async fn get_plan_progress(&self, plan: &TaskPlan) -> Result<PlanProgress, AgentError> {
        let total_subtasks = plan.subtasks.len();
        let completed_subtasks = plan.subtasks
            .iter()
            .filter(|task| task.status == TaskStatus::Completed)
            .count();
        let failed_subtasks = plan.subtasks
            .iter()
            .filter(|task| task.status == TaskStatus::Failed)
            .count();
        let in_progress_subtasks = plan.subtasks
            .iter()
            .filter(|task| task.status == TaskStatus::InProgress)
            .count();

        let completion_percentage = if total_subtasks > 0 {
            completed_subtasks as f64 / total_subtasks as f64
        } else {
            0.0
        };

        // Calculate time elapsed
        let time_elapsed = plan.created_at
            .elapsed()
            .unwrap_or(Duration::from_secs(0));

        // Estimate time remaining
        let estimated_time_remaining = if let Some(total_estimated) = plan.estimated_completion_time {
            if completion_percentage > 0.0 {
                let estimated_total_time = Duration::from_secs(
                    (time_elapsed.as_secs() as f64 / completion_percentage) as u64
                );
                estimated_total_time.checked_sub(time_elapsed)
            } else {
                Some(total_estimated)
            }
        } else {
            None
        };

        // Identify bottlenecks
        let mut bottlenecks = Vec::new();
        for task in &plan.subtasks {
            if task.status == TaskStatus::Blocked {
                bottlenecks.push(format!("Task '{}' is blocked", task.description));
            }
        }

        Ok(PlanProgress {
            total_subtasks,
            completed_subtasks,
            failed_subtasks,
            in_progress_subtasks,
            completion_percentage,
            estimated_time_remaining,
            time_elapsed,
            bottlenecks,
        })
    }

    async fn validate_plan(&self, plan: &TaskPlan) -> Result<PlanValidation, AgentError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Check for circular dependencies
        if self.has_circular_dependencies(plan) {
            errors.push("Plan contains circular dependencies".to_string());
        }

        // Check for orphaned dependencies
        for (task_id, deps) in &plan.dependencies {
            for dep_id in deps {
                if !plan.subtasks.iter().any(|task| task.id == *dep_id) {
                    errors.push(format!("Task '{}' depends on non-existent task '{}'", task_id, dep_id));
                }
            }
        }

        // Check for tasks without required tools
        for task in &plan.subtasks {
            if task.required_tools.is_empty() && !task.description.to_lowercase().contains("plan") {
                warnings.push(format!("Task '{}' has no required tools specified", task.description));
            }
        }

        // Check time estimates
        let tasks_without_estimates = plan.subtasks
            .iter()
            .filter(|task| task.estimated_duration.is_none())
            .count();

        if tasks_without_estimates > 0 {
            suggestions.push(format!("{} tasks could benefit from time estimates", tasks_without_estimates));
        }

        // Calculate feasibility score
        let mut feasibility_score: f64 = 1.0;

        if !errors.is_empty() {
            feasibility_score -= 0.5;
        }

        if warnings.len() > plan.subtasks.len() / 2 {
            feasibility_score -= 0.2;
        }

        if plan.subtasks.len() > self.max_subtasks_per_plan {
            feasibility_score -= 0.3;
        }

        feasibility_score = feasibility_score.max(0.0);

        Ok(PlanValidation {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
            feasibility_score,
        })
    }
}

impl DefaultTaskPlanningCapability {
    /// Check if the plan has circular dependencies
    fn has_circular_dependencies(&self, plan: &TaskPlan) -> bool {
        fn visit_task(
            task_id: &str,
            dependencies: &HashMap<String, Vec<String>>,
            visited: &mut HashSet<String>,
            rec_stack: &mut HashSet<String>,
        ) -> bool {
            visited.insert(task_id.to_string());
            rec_stack.insert(task_id.to_string());

            if let Some(deps) = dependencies.get(task_id) {
                for dep in deps {
                    if !visited.contains(dep) {
                        if visit_task(dep, dependencies, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack.contains(dep) {
                        return true;
                    }
                }
            }

            rec_stack.remove(task_id);
            false
        }

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for task in &plan.subtasks {
            if !visited.contains(&task.id) {
                if visit_task(&task.id, &plan.dependencies, &mut visited, &mut rec_stack) {
                    return true;
                }
            }
        }

        false
    }
}

impl Default for DefaultTaskPlanningCapability {
    fn default() -> Self {
        Self::new()
    }
}
