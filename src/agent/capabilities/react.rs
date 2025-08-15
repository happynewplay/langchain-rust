use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::{
    agent::AgentError,
    prompt::PromptArgs,
    schemas::agent::AgentAction,
    tools::Tool,
};

use super::{
    AgentCapability, PlanningEnhancer, ActionProcessor, ActionContext, ProcessedResult,
};

/// Trait for ReAct (Reasoning + Acting) capabilities that enable iterative reasoning and action cycles
#[async_trait]
pub trait ReActCapability: AgentCapability + PlanningEnhancer + ActionProcessor {
    /// Perform reasoning based on an observation
    async fn reason(
        &self,
        observation: &str,
        context: &ReasoningContext,
    ) -> Result<ReasoningResult, AgentError>;
    
    /// Plan the next action based on reasoning
    async fn plan_action(
        &self,
        reasoning: &ReasoningResult,
        available_tools: &[Arc<dyn Tool>],
    ) -> Result<PlannedAction, AgentError>;
    
    /// Reflect on a complete ReAct cycle
    async fn reflect_on_cycle(
        &self,
        cycle: &ReActCycle,
    ) -> Result<CycleReflection, AgentError>;
    
    /// Get reasoning patterns and insights
    async fn get_reasoning_patterns(&self) -> Result<Vec<ReasoningPattern>, AgentError>;
    
    /// Update reasoning strategies based on feedback
    async fn update_reasoning_strategy(
        &self,
        feedback: &StrategyFeedback,
    ) -> Result<(), AgentError>;
    
    /// Execute a complete ReAct cycle
    async fn execute_react_cycle(
        &self,
        initial_observation: &str,
        context: &ReasoningContext,
        available_tools: &[Arc<dyn Tool>],
    ) -> Result<ReActCycle, AgentError>;
}

/// Context for reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningContext {
    /// Current goal or objective
    pub goal: String,
    /// Previous reasoning cycles
    pub previous_cycles: Vec<ReActCycle>,
    /// Available knowledge or facts
    pub knowledge_base: HashMap<String, Value>,
    /// Constraints or limitations
    pub constraints: Vec<String>,
    /// Time pressure or urgency
    pub urgency_level: UrgencyLevel,
    /// Domain-specific context
    pub domain_context: HashMap<String, Value>,
}

impl ReasoningContext {
    pub fn new(goal: String) -> Self {
        Self {
            goal,
            previous_cycles: Vec::new(),
            knowledge_base: HashMap::new(),
            constraints: Vec::new(),
            urgency_level: UrgencyLevel::Normal,
            domain_context: HashMap::new(),
        }
    }
    
    pub fn with_knowledge(mut self, key: String, value: Value) -> Self {
        self.knowledge_base.insert(key, value);
        self
    }
    
    pub fn with_constraint(mut self, constraint: String) -> Self {
        self.constraints.push(constraint);
        self
    }
    
    pub fn with_urgency(mut self, urgency: UrgencyLevel) -> Self {
        self.urgency_level = urgency;
        self
    }
}

/// Urgency levels for reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UrgencyLevel {
    Low,
    Normal,
    High,
    Critical,
}

/// Result of reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    /// The reasoning chain or thought process
    pub reasoning_chain: Vec<ReasoningStep>,
    /// Conclusion reached
    pub conclusion: String,
    /// Confidence in the reasoning (0.0 to 1.0)
    pub confidence: f64,
    /// Alternative hypotheses considered
    pub alternatives: Vec<String>,
    /// Assumptions made during reasoning
    pub assumptions: Vec<String>,
    /// Reasoning strategy used
    pub strategy: ReasoningStrategy,
    /// Time taken for reasoning
    pub reasoning_time: Duration,
}

/// A single step in the reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// Type of reasoning step
    pub step_type: ReasoningStepType,
    /// Description of the step
    pub description: String,
    /// Input to this step
    pub input: String,
    /// Output from this step
    pub output: String,
    /// Confidence in this step
    pub confidence: f64,
}

/// Types of reasoning steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningStepType {
    /// Observation analysis
    Observation,
    /// Hypothesis formation
    Hypothesis,
    /// Deduction
    Deduction,
    /// Induction
    Induction,
    /// Abduction (inference to best explanation)
    Abduction,
    /// Analogy
    Analogy,
    /// Causal reasoning
    Causal,
    /// Constraint satisfaction
    Constraint,
}

/// Reasoning strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningStrategy {
    /// Forward chaining from observations
    ForwardChaining,
    /// Backward chaining from goals
    BackwardChaining,
    /// Breadth-first exploration
    BreadthFirst,
    /// Depth-first exploration
    DepthFirst,
    /// Best-first search
    BestFirst,
    /// Analogical reasoning
    Analogical,
    /// Case-based reasoning
    CaseBased,
}

/// A planned action based on reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    /// The action to take
    pub action: AgentAction,
    /// Justification for this action
    pub justification: String,
    /// Expected outcome
    pub expected_outcome: String,
    /// Confidence in this action choice (0.0 to 1.0)
    pub confidence: f64,
    /// Alternative actions considered
    pub alternatives: Vec<AgentAction>,
    /// Risk assessment
    pub risk_assessment: RiskAssessment,
}

/// Risk assessment for an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// Overall risk level
    pub risk_level: RiskLevel,
    /// Potential negative outcomes
    pub potential_risks: Vec<String>,
    /// Mitigation strategies
    pub mitigation_strategies: Vec<String>,
    /// Probability of success
    pub success_probability: f64,
}

/// Risk levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// A complete ReAct cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActCycle {
    /// Unique identifier for this cycle
    pub id: String,
    /// Initial observation
    pub observation: String,
    /// Reasoning performed
    pub reasoning: ReasoningResult,
    /// Action planned and taken
    pub action: PlannedAction,
    /// Result of the action
    pub action_result: String,
    /// Cycle number in the sequence
    pub cycle_number: usize,
    /// When this cycle started
    pub start_time: SystemTime,
    /// When this cycle completed
    pub end_time: Option<SystemTime>,
    /// Total cycle duration
    pub duration: Option<Duration>,
}

impl ReActCycle {
    pub fn new(id: String, observation: String, cycle_number: usize) -> Self {
        Self {
            id,
            observation,
            reasoning: ReasoningResult {
                reasoning_chain: Vec::new(),
                conclusion: String::new(),
                confidence: 0.0,
                alternatives: Vec::new(),
                assumptions: Vec::new(),
                strategy: ReasoningStrategy::ForwardChaining,
                reasoning_time: Duration::from_secs(0),
            },
            action: PlannedAction {
                action: AgentAction {
                    tool: String::new(),
                    tool_input: String::new(),
                    log: String::new(),
                },
                justification: String::new(),
                expected_outcome: String::new(),
                confidence: 0.0,
                alternatives: Vec::new(),
                risk_assessment: RiskAssessment {
                    risk_level: RiskLevel::Medium,
                    potential_risks: Vec::new(),
                    mitigation_strategies: Vec::new(),
                    success_probability: 0.5,
                },
            },
            action_result: String::new(),
            cycle_number,
            start_time: SystemTime::now(),
            end_time: None,
            duration: None,
        }
    }
    
    pub fn complete(mut self, action_result: String) -> Self {
        self.action_result = action_result;
        self.end_time = Some(SystemTime::now());
        if let Ok(duration) = self.end_time.unwrap().duration_since(self.start_time) {
            self.duration = Some(duration);
        }
        self
    }
}

/// Reflection on a ReAct cycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleReflection {
    /// What went well in this cycle
    pub successes: Vec<String>,
    /// What could be improved
    pub improvements: Vec<String>,
    /// Lessons learned
    pub lessons_learned: Vec<String>,
    /// Effectiveness score (0.0 to 1.0)
    pub effectiveness_score: f64,
    /// Reasoning quality assessment
    pub reasoning_quality: ReasoningQuality,
    /// Action quality assessment
    pub action_quality: ActionQuality,
}

/// Assessment of reasoning quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningQuality {
    /// Logical consistency score
    pub logical_consistency: f64,
    /// Completeness of reasoning
    pub completeness: f64,
    /// Efficiency of reasoning process
    pub efficiency: f64,
    /// Creativity in problem-solving
    pub creativity: f64,
}

/// Assessment of action quality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionQuality {
    /// Appropriateness of action choice
    pub appropriateness: f64,
    /// Execution quality
    pub execution_quality: f64,
    /// Outcome alignment with expectations
    pub outcome_alignment: f64,
    /// Risk management effectiveness
    pub risk_management: f64,
}

/// Reasoning patterns identified over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPattern {
    /// Pattern identifier
    pub id: String,
    /// Description of the pattern
    pub description: String,
    /// Contexts where this pattern is effective
    pub effective_contexts: Vec<String>,
    /// Success rate of this pattern
    pub success_rate: f64,
    /// Examples of this pattern in use
    pub examples: Vec<String>,
    /// When this pattern was identified
    pub identified_at: SystemTime,
}

/// Feedback for updating reasoning strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyFeedback {
    /// Strategy being evaluated
    pub strategy: ReasoningStrategy,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    /// Specific feedback points
    pub feedback_points: Vec<String>,
    /// Suggested improvements
    pub suggested_improvements: Vec<String>,
    /// Context where this feedback applies
    pub context: String,
}

/// Performance metrics for reasoning strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average reasoning time
    pub average_reasoning_time: Duration,
    /// Success rate
    pub success_rate: f64,
    /// Confidence accuracy (how well confidence predicts success)
    pub confidence_accuracy: f64,
    /// Efficiency score
    pub efficiency_score: f64,
}

/// Default implementation of ReAct capability
pub struct DefaultReActCapability {
    /// History of ReAct cycles
    cycle_history: Vec<ReActCycle>,
    /// Identified reasoning patterns
    reasoning_patterns: Vec<ReasoningPattern>,
    /// Current reasoning strategy
    current_strategy: ReasoningStrategy,
    /// Configuration
    max_reasoning_steps: usize,
    max_cycle_history: usize,
    confidence_threshold: f64,
}

impl DefaultReActCapability {
    /// Create a new default ReAct capability
    pub fn new() -> Self {
        Self {
            cycle_history: Vec::new(),
            reasoning_patterns: Vec::new(),
            current_strategy: ReasoningStrategy::ForwardChaining,
            max_reasoning_steps: 10,
            max_cycle_history: 100,
            confidence_threshold: 0.7,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(
        max_reasoning_steps: usize,
        max_cycle_history: usize,
        confidence_threshold: f64,
    ) -> Self {
        Self {
            cycle_history: Vec::new(),
            reasoning_patterns: Vec::new(),
            current_strategy: ReasoningStrategy::ForwardChaining,
            max_reasoning_steps,
            max_cycle_history,
            confidence_threshold,
        }
    }
    
    /// Generate a unique cycle ID
    fn generate_cycle_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("react_{:x}", timestamp)
    }

    /// Perform forward chaining reasoning
    fn reason_forward_chaining(&self, observation: &str, context: &ReasoningContext) -> ReasoningResult {
        let start_time = SystemTime::now();
        let mut reasoning_chain = Vec::new();

        // Step 1: Analyze observation
        reasoning_chain.push(ReasoningStep {
            step_type: ReasoningStepType::Observation,
            description: "Analyzing the current observation".to_string(),
            input: observation.to_string(),
            output: format!("Observed: {}", observation),
            confidence: 0.9,
        });

        // Step 2: Form hypothesis based on goal
        let hypothesis = format!("To achieve '{}', I need to understand: {}", context.goal, observation);
        reasoning_chain.push(ReasoningStep {
            step_type: ReasoningStepType::Hypothesis,
            description: "Forming hypothesis about next steps".to_string(),
            input: context.goal.clone(),
            output: hypothesis.clone(),
            confidence: 0.8,
        });

        // Step 3: Apply deductive reasoning
        let deduction = if observation.contains("error") || observation.contains("failed") {
            "The previous action was unsuccessful, need to try a different approach".to_string()
        } else if observation.contains("success") || observation.contains("completed") {
            "The previous action was successful, can proceed to next step".to_string()
        } else {
            "Need more information to determine the best next action".to_string()
        };

        reasoning_chain.push(ReasoningStep {
            step_type: ReasoningStepType::Deduction,
            description: "Applying logical deduction".to_string(),
            input: observation.to_string(),
            output: deduction.clone(),
            confidence: 0.85,
        });

        let reasoning_time = start_time.elapsed().unwrap_or(Duration::from_millis(100));

        ReasoningResult {
            reasoning_chain,
            conclusion: deduction,
            confidence: 0.8,
            alternatives: vec![
                "Could try a different tool".to_string(),
                "Could modify the approach".to_string(),
            ],
            assumptions: vec![
                "The observation accurately reflects the current state".to_string(),
                "The goal remains valid and achievable".to_string(),
            ],
            strategy: ReasoningStrategy::ForwardChaining,
            reasoning_time,
        }
    }

    /// Select the best tool for the given reasoning result
    fn select_best_tool(&self, reasoning: &ReasoningResult, available_tools: &[Arc<dyn Tool>]) -> Option<Arc<dyn Tool>> {
        if available_tools.is_empty() {
            return None;
        }

        // Simple tool selection based on reasoning conclusion
        let conclusion_lower = reasoning.conclusion.to_lowercase();

        for tool in available_tools {
            let tool_name_lower = tool.name().to_lowercase();
            let tool_desc_lower = tool.description().to_lowercase();

            // Match based on keywords in conclusion
            if conclusion_lower.contains("search") && (tool_name_lower.contains("search") || tool_desc_lower.contains("search")) {
                return Some(tool.clone());
            }

            if conclusion_lower.contains("calculate") && (tool_name_lower.contains("calc") || tool_desc_lower.contains("math")) {
                return Some(tool.clone());
            }

            if conclusion_lower.contains("write") && (tool_name_lower.contains("write") || tool_desc_lower.contains("file")) {
                return Some(tool.clone());
            }

            if conclusion_lower.contains("execute") && (tool_name_lower.contains("exec") || tool_desc_lower.contains("run")) {
                return Some(tool.clone());
            }
        }

        // Default to first available tool
        available_tools.first().cloned()
    }

    /// Assess risk for a planned action
    fn assess_action_risk(&self, action: &AgentAction, reasoning: &ReasoningResult) -> RiskAssessment {
        let mut potential_risks = Vec::new();
        let mut mitigation_strategies = Vec::new();
        let mut risk_level = RiskLevel::Low;

        // Assess based on tool type
        let tool_lower = action.tool.to_lowercase();

        if tool_lower.contains("delete") || tool_lower.contains("remove") {
            risk_level = RiskLevel::High;
            potential_risks.push("Data loss".to_string());
            mitigation_strategies.push("Backup before deletion".to_string());
        }

        if tool_lower.contains("execute") || tool_lower.contains("run") {
            risk_level = RiskLevel::Medium;
            potential_risks.push("Unintended side effects".to_string());
            mitigation_strategies.push("Test in safe environment first".to_string());
        }

        if tool_lower.contains("network") || tool_lower.contains("web") {
            potential_risks.push("Network connectivity issues".to_string());
            mitigation_strategies.push("Handle network errors gracefully".to_string());
        }

        // Assess based on confidence
        let success_probability = if reasoning.confidence > 0.8 {
            0.9
        } else if reasoning.confidence > 0.6 {
            0.7
        } else {
            0.5
        };

        RiskAssessment {
            risk_level,
            potential_risks,
            mitigation_strategies,
            success_probability,
        }
    }
}

impl AgentCapability for DefaultReActCapability {
    fn capability_name(&self) -> &'static str {
        "default_react"
    }

    fn capability_description(&self) -> &'static str {
        "Default implementation of ReAct (Reasoning + Acting) capability for iterative problem solving"
    }
}

#[async_trait]
impl PlanningEnhancer for DefaultReActCapability {
    async fn pre_plan(
        &self,
        _intermediate_steps: &[(AgentAction, String)],
        inputs: &mut PromptArgs,
    ) -> Result<(), AgentError> {
        // Add ReAct context to planning
        inputs.insert(
            "react_enabled".to_string(),
            serde_json::json!(true),
        );

        inputs.insert(
            "reasoning_strategy".to_string(),
            serde_json::json!(format!("{:?}", self.current_strategy)),
        );

        // Add recent cycle insights
        if !self.cycle_history.is_empty() {
            let recent_cycles: Vec<Value> = self.cycle_history
                .iter()
                .rev()
                .take(3)
                .map(|cycle| serde_json::json!({
                    "cycle_number": cycle.cycle_number,
                    "reasoning_confidence": cycle.reasoning.confidence,
                    "action_tool": cycle.action.action.tool,
                    "success": !cycle.action_result.contains("error"),
                }))
                .collect();

            inputs.insert(
                "recent_react_cycles".to_string(),
                serde_json::json!(recent_cycles),
            );
        }

        // Add reasoning patterns
        if !self.reasoning_patterns.is_empty() {
            let pattern_summaries: Vec<String> = self.reasoning_patterns
                .iter()
                .take(5)
                .map(|pattern| format!("{}: {:.1}% success", pattern.description, pattern.success_rate * 100.0))
                .collect();

            inputs.insert(
                "reasoning_patterns".to_string(),
                serde_json::json!(pattern_summaries),
            );
        }

        Ok(())
    }
}

#[async_trait]
impl ActionProcessor for DefaultReActCapability {
    async fn process_action_result(
        &self,
        _action: &AgentAction,
        result: &str,
        context: &ActionContext,
    ) -> Result<ProcessedResult, AgentError> {
        let mut processed = ProcessedResult::default();

        // Analyze the result for ReAct insights
        let success = !result.contains("error") && !result.contains("failed");

        // Create reasoning context for this result
        let reasoning_context = ReasoningContext::new(
            context.current_inputs
                .get("input")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown goal")
                .to_string()
        );

        // Perform quick reasoning about the result
        let reasoning = self.reason_forward_chaining(result, &reasoning_context);

        processed.additional_context = Some(serde_json::json!({
            "react_analysis": {
                "action_success": success,
                "reasoning_confidence": reasoning.confidence,
                "reasoning_conclusion": reasoning.conclusion,
                "suggested_next_steps": reasoning.alternatives,
            }
        }));

        // If reasoning suggests improvements, modify the result
        if reasoning.confidence > self.confidence_threshold && !success {
            processed.modified_result = Some(format!(
                "{}\n\nReAct Analysis: {} (Confidence: {:.1}%)",
                result,
                reasoning.conclusion,
                reasoning.confidence * 100.0
            ));
        }

        Ok(processed)
    }
}

#[async_trait]
impl ReActCapability for DefaultReActCapability {
    async fn reason(
        &self,
        observation: &str,
        context: &ReasoningContext,
    ) -> Result<ReasoningResult, AgentError> {
        match self.current_strategy {
            ReasoningStrategy::ForwardChaining => Ok(self.reason_forward_chaining(observation, context)),
            ReasoningStrategy::BackwardChaining => Ok(self.reason_backward_chaining(observation, context)),
            _ => Ok(self.reason_forward_chaining(observation, context)), // Default fallback
        }
    }

    async fn plan_action(
        &self,
        reasoning: &ReasoningResult,
        available_tools: &[Arc<dyn Tool>],
    ) -> Result<PlannedAction, AgentError> {
        let selected_tool = self.select_best_tool(reasoning, available_tools)
            .ok_or_else(|| AgentError::OtherError("No suitable tool found".to_string()))?;

        // Create the action
        let action = AgentAction {
            tool: selected_tool.name(),
            tool_input: reasoning.conclusion.clone(),
            log: format!("ReAct reasoning: {}", reasoning.conclusion),
        };

        // Assess risk
        let risk_assessment = self.assess_action_risk(&action, reasoning);

        // Generate alternatives
        let alternatives = available_tools
            .iter()
            .filter(|tool| tool.name() != selected_tool.name())
            .take(2)
            .map(|tool| AgentAction {
                tool: tool.name(),
                tool_input: reasoning.conclusion.clone(),
                log: format!("Alternative action: {}", tool.name()),
            })
            .collect();

        Ok(PlannedAction {
            action,
            justification: format!("Based on reasoning: {}", reasoning.conclusion),
            expected_outcome: "Action should help progress toward the goal".to_string(),
            confidence: reasoning.confidence,
            alternatives,
            risk_assessment,
        })
    }

    async fn reflect_on_cycle(
        &self,
        cycle: &ReActCycle,
    ) -> Result<CycleReflection, AgentError> {
        let mut successes = Vec::new();
        let mut improvements = Vec::new();
        let mut lessons_learned = Vec::new();

        // Analyze reasoning quality
        let reasoning_quality = ReasoningQuality {
            logical_consistency: if cycle.reasoning.reasoning_chain.len() > 2 { 0.8 } else { 0.6 },
            completeness: cycle.reasoning.confidence,
            efficiency: if cycle.reasoning.reasoning_time < Duration::from_secs(5) { 0.9 } else { 0.7 },
            creativity: if cycle.reasoning.alternatives.len() > 1 { 0.8 } else { 0.5 },
        };

        // Analyze action quality
        let action_success = !cycle.action_result.contains("error");
        let action_quality = ActionQuality {
            appropriateness: cycle.action.confidence,
            execution_quality: if action_success { 0.9 } else { 0.3 },
            outcome_alignment: if action_success { 0.8 } else { 0.4 },
            risk_management: match cycle.action.risk_assessment.risk_level {
                RiskLevel::Low => 0.9,
                RiskLevel::Medium => 0.7,
                RiskLevel::High => 0.5,
                RiskLevel::Critical => 0.3,
            },
        };

        // Generate insights
        if action_success {
            successes.push("Action executed successfully".to_string());
            successes.push(format!("Tool '{}' was effective", cycle.action.action.tool));
        } else {
            improvements.push("Action execution failed, consider alternative approaches".to_string());
            improvements.push("Review tool selection criteria".to_string());
        }

        if cycle.reasoning.confidence > 0.8 {
            successes.push("High confidence reasoning".to_string());
        } else {
            improvements.push("Reasoning confidence could be improved".to_string());
        }

        // Generate lessons
        lessons_learned.push(format!(
            "Tool '{}' {} for this type of task",
            cycle.action.action.tool,
            if action_success { "works well" } else { "may not be suitable" }
        ));

        if let Some(duration) = cycle.duration {
            if duration > Duration::from_secs(30) {
                lessons_learned.push("Cycle took longer than expected, consider optimizing".to_string());
            }
        }

        // Calculate effectiveness score
        let effectiveness_score = (reasoning_quality.completeness +
                                 action_quality.execution_quality +
                                 action_quality.outcome_alignment) / 3.0;

        Ok(CycleReflection {
            successes,
            improvements,
            lessons_learned,
            effectiveness_score,
            reasoning_quality,
            action_quality,
        })
    }

    async fn get_reasoning_patterns(&self) -> Result<Vec<ReasoningPattern>, AgentError> {
        Ok(self.reasoning_patterns.clone())
    }

    async fn update_reasoning_strategy(
        &self,
        feedback: &StrategyFeedback,
    ) -> Result<(), AgentError> {
        // In a real implementation, this would update the strategy based on feedback
        // For now, we'll just log the feedback
        log::info!(
            "Received strategy feedback for {:?}: success rate {:.1}%",
            feedback.strategy,
            feedback.performance_metrics.success_rate * 100.0
        );

        // Could implement strategy switching logic here
        if feedback.performance_metrics.success_rate < 0.5 {
            log::warn!("Strategy performance below threshold, consider switching");
        }

        Ok(())
    }

    async fn execute_react_cycle(
        &self,
        initial_observation: &str,
        context: &ReasoningContext,
        available_tools: &[Arc<dyn Tool>],
    ) -> Result<ReActCycle, AgentError> {
        let cycle_id = self.generate_cycle_id();
        let cycle_number = self.cycle_history.len() + 1;

        let mut cycle = ReActCycle::new(cycle_id, initial_observation.to_string(), cycle_number);

        // Step 1: Reason about the observation
        cycle.reasoning = self.reason(initial_observation, context).await?;

        // Step 2: Plan action based on reasoning
        cycle.action = self.plan_action(&cycle.reasoning, available_tools).await?;

        // Step 3: Simulate action execution (in real implementation, this would execute the action)
        let action_result = format!(
            "Simulated execution of tool '{}' with input: {}",
            cycle.action.action.tool,
            cycle.action.action.tool_input
        );

        cycle = cycle.complete(action_result);

        Ok(cycle)
    }
}

impl DefaultReActCapability {
    /// Perform backward chaining reasoning
    fn reason_backward_chaining(&self, observation: &str, context: &ReasoningContext) -> ReasoningResult {
        let start_time = SystemTime::now();
        let mut reasoning_chain = Vec::new();

        // Start from the goal and work backwards
        reasoning_chain.push(ReasoningStep {
            step_type: ReasoningStepType::Hypothesis,
            description: "Starting from the goal".to_string(),
            input: context.goal.clone(),
            output: format!("Goal: {}", context.goal),
            confidence: 0.9,
        });

        // Determine what's needed to achieve the goal
        let needed_action = if context.goal.to_lowercase().contains("find") || context.goal.to_lowercase().contains("search") {
            "Need to search for information"
        } else if context.goal.to_lowercase().contains("create") || context.goal.to_lowercase().contains("write") {
            "Need to create or write something"
        } else if context.goal.to_lowercase().contains("calculate") || context.goal.to_lowercase().contains("compute") {
            "Need to perform calculations"
        } else {
            "Need to analyze the current situation"
        };

        reasoning_chain.push(ReasoningStep {
            step_type: ReasoningStepType::Deduction,
            description: "Determining required action".to_string(),
            input: context.goal.clone(),
            output: needed_action.to_string(),
            confidence: 0.8,
        });

        // Connect to current observation
        reasoning_chain.push(ReasoningStep {
            step_type: ReasoningStepType::Observation,
            description: "Connecting observation to required action".to_string(),
            input: observation.to_string(),
            output: format!("Current state: {} | Required: {}", observation, needed_action),
            confidence: 0.75,
        });

        let reasoning_time = start_time.elapsed().unwrap_or(Duration::from_millis(120));

        ReasoningResult {
            reasoning_chain,
            conclusion: needed_action.to_string(),
            confidence: 0.75,
            alternatives: vec![
                "Could approach the goal differently".to_string(),
                "Could break down the goal into smaller steps".to_string(),
            ],
            assumptions: vec![
                "The goal is achievable with available tools".to_string(),
                "The current observation is relevant to the goal".to_string(),
            ],
            strategy: ReasoningStrategy::BackwardChaining,
            reasoning_time,
        }
    }
}

impl Default for DefaultReActCapability {
    fn default() -> Self {
        Self::new()
    }
}
