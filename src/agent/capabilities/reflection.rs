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
};

/// Trait for reflection capabilities that enable self-evaluation and learning
#[async_trait]
pub trait ReflectionCapability: AgentCapability + PlanningEnhancer + ActionProcessor {
    /// Reflect on a completed action and its result
    async fn reflect_on_action(
        &self,
        action: &AgentAction,
        result: &str,
        context: &ReflectionContext,
    ) -> Result<ReflectionResult, AgentError>;
    
    /// Learn from a complete experience (sequence of actions)
    async fn learn_from_experience(&self, experience: &Experience) -> Result<(), AgentError>;
    
    /// Get insights from accumulated reflections
    async fn get_reflection_insights(&self) -> Result<Vec<Insight>, AgentError>;
    
    /// Get performance metrics
    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, AgentError>;
    
    /// Clear reflection history (useful for testing or reset)
    async fn clear_reflection_history(&mut self) -> Result<(), AgentError>;
}

/// Context information for reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionContext {
    /// Previous actions in the current session
    pub previous_actions: Vec<(AgentAction, String)>,
    /// Current goal or objective
    pub current_goal: String,
    /// Execution history from previous sessions
    pub execution_history: Vec<Experience>,
    /// Additional context metadata
    pub metadata: HashMap<String, Value>,
    /// Timestamp of the reflection
    pub timestamp: SystemTime,
}

impl ReflectionContext {
    pub fn new(goal: String) -> Self {
        Self {
            previous_actions: Vec::new(),
            current_goal: goal,
            execution_history: Vec::new(),
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        }
    }
    
    pub fn with_previous_actions(mut self, actions: Vec<(AgentAction, String)>) -> Self {
        self.previous_actions = actions;
        self
    }
    
    pub fn with_execution_history(mut self, history: Vec<Experience>) -> Self {
        self.execution_history = history;
        self
    }
    
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Result of a reflection process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
    /// Insights gained from the reflection
    pub insights: Vec<Insight>,
    /// Suggested improvements for future actions
    pub suggested_improvements: Vec<Improvement>,
    /// Confidence score for the reflection (0.0 to 1.0)
    pub confidence_score: f64,
    /// Lessons learned
    pub lessons_learned: Vec<String>,
    /// Performance assessment
    pub performance_assessment: PerformanceAssessment,
}

/// An insight gained from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    /// Type of insight
    pub insight_type: InsightType,
    /// Description of the insight
    pub description: String,
    /// Confidence in this insight (0.0 to 1.0)
    pub confidence: f64,
    /// Supporting evidence
    pub evidence: Vec<String>,
    /// When this insight was discovered
    pub timestamp: SystemTime,
}

/// Types of insights that can be gained
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    /// Insight about tool usage patterns
    ToolUsage,
    /// Insight about problem-solving strategies
    Strategy,
    /// Insight about error patterns
    ErrorPattern,
    /// Insight about efficiency improvements
    Efficiency,
    /// Insight about goal achievement patterns
    GoalAchievement,
    /// Custom insight type
    Custom(String),
}

/// A suggested improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    /// Area that needs improvement
    pub area: ImprovementArea,
    /// Description of the improvement
    pub description: String,
    /// Priority of this improvement (1-10, 10 being highest)
    pub priority: u8,
    /// Specific actions to implement the improvement
    pub action_items: Vec<String>,
    /// Expected impact of the improvement
    pub expected_impact: String,
}

/// Areas where improvements can be made
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementArea {
    /// Tool selection and usage
    ToolUsage,
    /// Planning and strategy
    Planning,
    /// Error handling
    ErrorHandling,
    /// Communication and output quality
    Communication,
    /// Efficiency and performance
    Efficiency,
    /// Custom improvement area
    Custom(String),
}

/// Assessment of performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAssessment {
    /// Overall performance score (0.0 to 1.0)
    pub overall_score: f64,
    /// Success rate for achieving goals
    pub success_rate: f64,
    /// Average time to complete tasks
    pub average_completion_time: Option<Duration>,
    /// Error rate
    pub error_rate: f64,
    /// Tool usage efficiency
    pub tool_efficiency: f64,
    /// Detailed breakdown by category
    pub category_scores: HashMap<String, f64>,
}

/// A complete experience for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    /// Unique identifier for this experience
    pub id: String,
    /// Goal that was being pursued
    pub goal: String,
    /// Sequence of actions taken
    pub actions: Vec<(AgentAction, String)>,
    /// Final outcome
    pub outcome: ExperienceOutcome,
    /// Duration of the experience
    pub duration: Duration,
    /// Lessons learned from this experience
    pub lessons: Vec<String>,
    /// When this experience occurred
    pub timestamp: SystemTime,
}

/// Outcome of an experience
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExperienceOutcome {
    /// Goal was successfully achieved
    Success { result: String },
    /// Goal was partially achieved
    PartialSuccess { result: String, missing: Vec<String> },
    /// Goal was not achieved
    Failure { error: String, reason: String },
    /// Experience was interrupted
    Interrupted { reason: String },
}

/// Performance metrics over time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total number of experiences
    pub total_experiences: usize,
    /// Number of successful experiences
    pub successful_experiences: usize,
    /// Average performance score over time
    pub average_performance: f64,
    /// Performance trend (improving, declining, stable)
    pub trend: PerformanceTrend,
    /// Most common error types
    pub common_errors: Vec<(String, usize)>,
    /// Most effective tools
    pub effective_tools: Vec<(String, f64)>,
    /// Time-based metrics
    pub time_metrics: TimeMetrics,
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTrend {
    Improving { rate: f64 },
    Declining { rate: f64 },
    Stable,
    InsufficientData,
}

/// Time-based performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeMetrics {
    /// Average time per action
    pub average_action_time: Duration,
    /// Average time per experience
    pub average_experience_time: Duration,
    /// Fastest completion time
    pub fastest_completion: Option<Duration>,
    /// Slowest completion time
    pub slowest_completion: Option<Duration>,
}

/// Default implementation of reflection capability
pub struct DefaultReflectionCapability {
    /// Storage for experiences and insights
    experiences: Vec<Experience>,
    insights: Vec<Insight>,
    performance_history: Vec<PerformanceAssessment>,
    /// Configuration
    max_experiences: usize,
    max_insights: usize,
    reflection_threshold: f64,
}

impl DefaultReflectionCapability {
    /// Create a new default reflection capability
    pub fn new() -> Self {
        Self {
            experiences: Vec::new(),
            insights: Vec::new(),
            performance_history: Vec::new(),
            max_experiences: 1000,
            max_insights: 500,
            reflection_threshold: 0.7,
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(
        max_experiences: usize,
        max_insights: usize,
        reflection_threshold: f64,
    ) -> Self {
        Self {
            experiences: Vec::new(),
            insights: Vec::new(),
            performance_history: Vec::new(),
            max_experiences,
            max_insights,
            reflection_threshold,
        }
    }
    
    /// Analyze patterns in experiences to generate insights
    fn analyze_patterns(&self) -> Vec<Insight> {
        let mut insights = Vec::new();
        
        // Analyze tool usage patterns
        if let Some(tool_insight) = self.analyze_tool_usage() {
            insights.push(tool_insight);
        }
        
        // Analyze error patterns
        if let Some(error_insight) = self.analyze_error_patterns() {
            insights.push(error_insight);
        }
        
        // Analyze success patterns
        if let Some(success_insight) = self.analyze_success_patterns() {
            insights.push(success_insight);
        }
        
        insights
    }
    
    fn analyze_tool_usage(&self) -> Option<Insight> {
        if self.experiences.is_empty() {
            return None;
        }
        
        let mut tool_usage: HashMap<String, usize> = HashMap::new();
        let mut tool_success: HashMap<String, usize> = HashMap::new();
        
        for experience in &self.experiences {
            for (action, _) in &experience.actions {
                *tool_usage.entry(action.tool.clone()).or_insert(0) += 1;
                
                if matches!(experience.outcome, ExperienceOutcome::Success { .. }) {
                    *tool_success.entry(action.tool.clone()).or_insert(0) += 1;
                }
            }
        }
        
        // Find most effective tools
        let mut effectiveness: Vec<(String, f64)> = tool_usage
            .iter()
            .map(|(tool, usage)| {
                let success_count = tool_success.get(tool).unwrap_or(&0);
                let effectiveness = *success_count as f64 / *usage as f64;
                (tool.clone(), effectiveness)
            })
            .collect();
        
        effectiveness.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        if let Some((best_tool, best_rate)) = effectiveness.first() {
            Some(Insight {
                insight_type: InsightType::ToolUsage,
                description: format!(
                    "Tool '{}' has the highest success rate at {:.2}%",
                    best_tool,
                    best_rate * 100.0
                ),
                confidence: 0.8,
                evidence: vec![format!("Analyzed {} experiences", self.experiences.len())],
                timestamp: SystemTime::now(),
            })
        } else {
            None
        }
    }
    
    fn analyze_error_patterns(&self) -> Option<Insight> {
        let error_experiences: Vec<_> = self.experiences
            .iter()
            .filter(|exp| matches!(exp.outcome, ExperienceOutcome::Failure { .. }))
            .collect();
        
        if error_experiences.is_empty() {
            return None;
        }
        
        // This is a simplified analysis - in practice, you'd want more sophisticated pattern detection
        let error_rate = error_experiences.len() as f64 / self.experiences.len() as f64;
        
        Some(Insight {
            insight_type: InsightType::ErrorPattern,
            description: format!(
                "Current error rate is {:.1}%. {} out of {} experiences failed.",
                error_rate * 100.0,
                error_experiences.len(),
                self.experiences.len()
            ),
            confidence: 0.9,
            evidence: vec![format!("Analyzed {} total experiences", self.experiences.len())],
            timestamp: SystemTime::now(),
        })
    }
    
    fn analyze_success_patterns(&self) -> Option<Insight> {
        let successful_experiences: Vec<_> = self.experiences
            .iter()
            .filter(|exp| matches!(exp.outcome, ExperienceOutcome::Success { .. }))
            .collect();

        if successful_experiences.is_empty() {
            return None;
        }

        let success_rate = successful_experiences.len() as f64 / self.experiences.len() as f64;

        Some(Insight {
            insight_type: InsightType::GoalAchievement,
            description: format!(
                "Success rate is {:.1}%. {} out of {} experiences were successful.",
                success_rate * 100.0,
                successful_experiences.len(),
                self.experiences.len()
            ),
            confidence: 0.9,
            evidence: vec![format!("Analyzed {} total experiences", self.experiences.len())],
            timestamp: SystemTime::now(),
        })
    }
}

impl AgentCapability for DefaultReflectionCapability {
    fn capability_name(&self) -> &'static str {
        "default_reflection"
    }

    fn capability_description(&self) -> &'static str {
        "Default implementation of reflection capability for self-evaluation and learning"
    }
}

#[async_trait]
impl PlanningEnhancer for DefaultReflectionCapability {
    async fn pre_plan(
        &self,
        _intermediate_steps: &[(AgentAction, String)],
        inputs: &mut PromptArgs,
    ) -> Result<(), AgentError> {
        // Add reflection insights to the planning context
        if !self.insights.is_empty() {
            let recent_insights: Vec<String> = self.insights
                .iter()
                .take(5) // Take the 5 most recent insights
                .map(|insight| format!("{:?}: {}", insight.insight_type, insight.description))
                .collect();

            inputs.insert(
                "reflection_insights".to_string(),
                serde_json::json!(recent_insights),
            );
        }

        // Add performance context
        if let Ok(metrics) = self.get_performance_metrics().await {
            inputs.insert(
                "performance_context".to_string(),
                serde_json::json!({
                    "success_rate": metrics.successful_experiences as f64 / metrics.total_experiences.max(1) as f64,
                    "average_performance": metrics.average_performance,
                    "trend": format!("{:?}", metrics.trend),
                }),
            );
        }

        Ok(())
    }
}

#[async_trait]
impl ActionProcessor for DefaultReflectionCapability {
    async fn process_action_result(
        &self,
        action: &AgentAction,
        result: &str,
        context: &ActionContext,
    ) -> Result<ProcessedResult, AgentError> {
        // Create reflection context
        let reflection_context = ReflectionContext {
            previous_actions: context.intermediate_steps.clone(),
            current_goal: context.current_inputs
                .get("input")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown goal")
                .to_string(),
            execution_history: self.experiences.clone(),
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        // Perform reflection on this action
        let reflection_result = self.reflect_on_action(action, result, &reflection_context).await?;

        // If reflection confidence is high, add insights to the result
        let mut processed = ProcessedResult::default();

        if reflection_result.confidence_score >= self.reflection_threshold {
            let insights_summary = reflection_result.insights
                .iter()
                .map(|insight| insight.description.clone())
                .collect::<Vec<_>>()
                .join("; ");

            if !insights_summary.is_empty() {
                processed.additional_context = Some(serde_json::json!({
                    "reflection_insights": insights_summary,
                    "confidence": reflection_result.confidence_score,
                }));
            }
        }

        Ok(processed)
    }
}

#[async_trait]
impl ReflectionCapability for DefaultReflectionCapability {
    async fn reflect_on_action(
        &self,
        action: &AgentAction,
        result: &str,
        context: &ReflectionContext,
    ) -> Result<ReflectionResult, AgentError> {
        let mut insights = Vec::new();
        let mut improvements = Vec::new();
        let mut lessons_learned = Vec::new();

        // Analyze the action effectiveness
        let is_successful = !result.contains("error") && !result.contains("failed");

        if is_successful {
            insights.push(Insight {
                insight_type: InsightType::ToolUsage,
                description: format!("Tool '{}' was effective for this type of task", action.tool),
                confidence: 0.7,
                evidence: vec![format!("Action succeeded: {}", result)],
                timestamp: SystemTime::now(),
            });

            lessons_learned.push(format!("Tool '{}' works well for similar tasks", action.tool));
        } else {
            insights.push(Insight {
                insight_type: InsightType::ErrorPattern,
                description: format!("Tool '{}' may not be suitable for this task type", action.tool),
                confidence: 0.6,
                evidence: vec![format!("Action failed: {}", result)],
                timestamp: SystemTime::now(),
            });

            improvements.push(Improvement {
                area: ImprovementArea::ToolUsage,
                description: format!("Consider alternative tools for tasks similar to: {}", action.tool_input),
                priority: 7,
                action_items: vec![
                    "Research alternative tools".to_string(),
                    "Test different approaches".to_string(),
                ],
                expected_impact: "Improved success rate for similar tasks".to_string(),
            });

            lessons_learned.push(format!("Tool '{}' had issues with this task type", action.tool));
        }

        // Calculate confidence based on available data
        let confidence_score = if context.previous_actions.len() > 3 {
            0.8
        } else {
            0.6
        };

        // Create performance assessment
        let performance_assessment = PerformanceAssessment {
            overall_score: if is_successful { 0.8 } else { 0.3 },
            success_rate: if is_successful { 1.0 } else { 0.0 },
            average_completion_time: None,
            error_rate: if is_successful { 0.0 } else { 1.0 },
            tool_efficiency: if is_successful { 0.9 } else { 0.2 },
            category_scores: HashMap::new(),
        };

        Ok(ReflectionResult {
            insights,
            suggested_improvements: improvements,
            confidence_score,
            lessons_learned,
            performance_assessment,
        })
    }

    async fn learn_from_experience(&self, experience: &Experience) -> Result<(), AgentError> {
        // In a real implementation, this would update internal models or knowledge bases
        // For now, we'll just log the learning
        log::info!(
            "Learning from experience '{}': {} actions, outcome: {:?}",
            experience.id,
            experience.actions.len(),
            experience.outcome
        );

        // Generate insights from this experience
        let insights = self.analyze_patterns();

        // Store insights (in a real implementation, you'd persist these)
        log::debug!("Generated {} insights from experience", insights.len());

        Ok(())
    }

    async fn get_reflection_insights(&self) -> Result<Vec<Insight>, AgentError> {
        Ok(self.insights.clone())
    }

    async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, AgentError> {
        let total_experiences = self.experiences.len();
        let successful_experiences = self.experiences
            .iter()
            .filter(|exp| matches!(exp.outcome, ExperienceOutcome::Success { .. }))
            .count();

        let average_performance = if total_experiences > 0 {
            successful_experiences as f64 / total_experiences as f64
        } else {
            0.0
        };

        // Calculate trend (simplified)
        let trend = if total_experiences < 5 {
            PerformanceTrend::InsufficientData
        } else {
            // Compare recent performance to overall
            let recent_success = self.experiences
                .iter()
                .rev()
                .take(5)
                .filter(|exp| matches!(exp.outcome, ExperienceOutcome::Success { .. }))
                .count() as f64 / 5.0;

            if recent_success > average_performance + 0.1 {
                PerformanceTrend::Improving { rate: recent_success - average_performance }
            } else if recent_success < average_performance - 0.1 {
                PerformanceTrend::Declining { rate: average_performance - recent_success }
            } else {
                PerformanceTrend::Stable
            }
        };

        // Calculate time metrics
        let durations: Vec<Duration> = self.experiences.iter().map(|exp| exp.duration).collect();
        let average_experience_time = if !durations.is_empty() {
            let total_duration: Duration = durations.iter().sum();
            total_duration / durations.len() as u32
        } else {
            Duration::from_secs(0)
        };

        let time_metrics = TimeMetrics {
            average_action_time: Duration::from_secs(5), // Placeholder
            average_experience_time,
            fastest_completion: durations.iter().min().copied(),
            slowest_completion: durations.iter().max().copied(),
        };

        Ok(PerformanceMetrics {
            total_experiences,
            successful_experiences,
            average_performance,
            trend,
            common_errors: Vec::new(), // Would be populated in real implementation
            effective_tools: Vec::new(), // Would be populated in real implementation
            time_metrics,
        })
    }

    async fn clear_reflection_history(&mut self) -> Result<(), AgentError> {
        self.experiences.clear();
        self.insights.clear();
        self.performance_history.clear();
        Ok(())
    }
}

impl Default for DefaultReflectionCapability {
    fn default() -> Self {
        Self::new()
    }
}
