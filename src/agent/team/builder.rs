use std::sync::Arc;

use crate::{
    agent::{
        human::{HumanAgentConfig, HumanInteractionInterface, InterventionCondition, TerminationCondition},
        Agent, AgentError,
    },
};

use super::{
    agent::TeamAgent,
    config::{ChildAgentConfig, ExecutionPattern, TeamAgentConfig},
    team_human::{TeamHumanAgent, TeamHumanAgentConfig},
};

/// Builder for creating team agents
pub struct TeamAgentBuilder {
    config: TeamAgentConfig,
}

impl TeamAgentBuilder {
    /// Create a new team agent builder
    pub fn new() -> Self {
        Self {
            config: TeamAgentConfig::new(),
        }
    }

    /// Add a child agent to the team
    pub fn add_agent<S: Into<String>>(
        mut self,
        id: S,
        agent: Arc<dyn Agent>,
    ) -> Self {
        let child_config = ChildAgentConfig::new(id, agent);
        self.config = self.config.add_child_agent(child_config);
        self
    }

    /// Add a team agent as a child (nested team)
    pub fn add_team_agent<S: Into<String>>(
        mut self,
        id: S,
        team_agent: Arc<dyn Agent>,
    ) -> Self {
        let child_config = ChildAgentConfig::new_team_agent(id, team_agent);
        self.config = self.config.add_child_agent(child_config);
        self
    }

    /// Add a child agent with custom configuration
    pub fn add_agent_with_config(mut self, config: ChildAgentConfig) -> Self {
        self.config = self.config.add_child_agent(config);
        self
    }

    /// Add multiple child agents
    pub fn add_agents<I, S>(mut self, agents: I) -> Self
    where
        I: IntoIterator<Item = (S, Arc<dyn Agent>)>,
        S: Into<String>,
    {
        for (id, agent) in agents {
            self = self.add_agent(id, agent);
        }
        self
    }

    /// Set the execution pattern
    pub fn execution_pattern(mut self, pattern: ExecutionPattern) -> Self {
        self.config = self.config.with_execution_pattern(pattern);
        self
    }

    /// Set execution to concurrent
    pub fn concurrent(self) -> Self {
        self.execution_pattern(ExecutionPattern::Concurrent)
    }

    /// Set execution to sequential
    pub fn sequential(self) -> Self {
        self.execution_pattern(ExecutionPattern::Sequential)
    }

    /// Set execution to hybrid with custom steps
    pub fn hybrid(self, steps: Vec<super::config::ExecutionStep>) -> Self {
        self.execution_pattern(ExecutionPattern::Hybrid(steps))
    }

    /// Set maximum iterations
    pub fn max_iterations(mut self, max_iterations: i32) -> Self {
        self.config = self.config.with_max_iterations(max_iterations);
        self
    }

    /// Set break on error behavior
    pub fn break_on_error(mut self, break_on_error: bool) -> Self {
        self.config = self.config.with_break_on_error(break_on_error);
        self
    }

    /// Set global timeout
    pub fn global_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config = self.config.with_global_timeout(timeout_seconds);
        self
    }

    /// Set system prompt/prefix
    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.config = self.config.with_prefix(prefix);
        self
    }

    /// Set memory for the team agent
    pub fn memory(mut self, memory: Arc<tokio::sync::Mutex<dyn crate::schemas::memory::BaseMemory>>) -> Self {
        self.config = self.config.with_memory(memory);
        self
    }

    /// Set whether to use coordination prompts
    pub fn coordination_prompts(mut self, use_coordination_prompts: bool) -> Self {
        self.config = self.config.with_coordination_prompts(use_coordination_prompts);
        self
    }

    /// Build the team agent
    pub fn build(self) -> Result<TeamAgent, AgentError> {
        TeamAgent::new(self.config)
    }

    /// Build the team agent and wrap it as a tool
    pub fn build_as_tool<S: Into<String>>(
        self,
        name: S,
        description: S,
    ) -> Result<super::agent::TeamAgentTool, AgentError> {
        let team_agent = Arc::new(self.build()?);
        Ok(super::agent::TeamAgentTool::new(
            team_agent,
            name,
            description,
        ))
    }

    /// Build the team agent and wrap it as a tool with auto-generated name/description
    pub fn build_as_auto_tool(self) -> Result<super::agent::TeamAgentTool, AgentError> {
        let team_agent = Arc::new(self.build()?);
        Ok(super::agent::TeamAgentTool::from_team_agent(team_agent))
    }
}

impl Default for TeamAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating common execution patterns
impl TeamAgentBuilder {
    /// Create a simple sequential team with the given agents
    pub fn sequential_team<I, S>(agents: I) -> Self
    where
        I: IntoIterator<Item = (S, Arc<dyn Agent>)>,
        S: Into<String>,
    {
        Self::new().add_agents(agents).sequential()
    }

    /// Create a simple concurrent team with the given agents
    pub fn concurrent_team<I, S>(agents: I) -> Self
    where
        I: IntoIterator<Item = (S, Arc<dyn Agent>)>,
        S: Into<String>,
    {
        Self::new().add_agents(agents).concurrent()
    }

    /// Create a pipeline where agent A feeds into agent B, which runs concurrently with agent C,
    /// and then agent D receives the combined results
    pub fn pipeline_with_concurrent<S: Into<String>>(
        agent_a: (S, Arc<dyn Agent>),
        agent_b: (S, Arc<dyn Agent>),
        agent_c: (S, Arc<dyn Agent>),
        agent_d: (S, Arc<dyn Agent>),
    ) -> Self {
        let (id_a, agent_a) = agent_a;
        let (id_b, agent_b) = agent_b;
        let (id_c, agent_c) = agent_c;
        let (id_d, agent_d) = agent_d;

        let id_a = id_a.into();
        let id_b = id_b.into();
        let id_c = id_c.into();
        let id_d = id_d.into();

        let steps = vec![
            // Step 0: Agent A runs alone
            super::config::ExecutionStep {
                agent_ids: vec![id_a.clone()],
                concurrent: false,
                dependencies: vec![],
            },
            // Step 1: Agent B and C run concurrently, both depend on A
            super::config::ExecutionStep {
                agent_ids: vec![id_b.clone(), id_c.clone()],
                concurrent: true,
                dependencies: vec![0],
            },
            // Step 2: Agent D runs alone, depends on step 1 (B and C)
            super::config::ExecutionStep {
                agent_ids: vec![id_d.clone()],
                concurrent: false,
                dependencies: vec![1],
            },
        ];

        Self::new()
            .add_agent(id_a, agent_a)
            .add_agent(id_b, agent_b)
            .add_agent(id_c, agent_c)
            .add_agent(id_d, agent_d)
            .hybrid(steps)
    }

    /// Create a fan-out pattern where one agent feeds into multiple concurrent agents
    pub fn fan_out<S: Into<String>>(
        source_agent: (S, Arc<dyn Agent>),
        target_agents: Vec<(S, Arc<dyn Agent>)>,
    ) -> Self {
        let (source_id, source_agent) = source_agent;
        let source_id = source_id.into();

        let mut builder = Self::new().add_agent(source_id.clone(), source_agent);

        let mut target_ids = Vec::new();
        for (id, agent) in target_agents {
            let id = id.into();
            target_ids.push(id.clone());
            builder = builder.add_agent(id, agent);
        }

        let steps = vec![
            // Step 0: Source agent runs alone
            super::config::ExecutionStep {
                agent_ids: vec![source_id],
                concurrent: false,
                dependencies: vec![],
            },
            // Step 1: All target agents run concurrently, depend on source
            super::config::ExecutionStep {
                agent_ids: target_ids,
                concurrent: true,
                dependencies: vec![0],
            },
        ];

        builder.hybrid(steps)
    }

    /// Create a fan-in pattern where multiple agents feed into one final agent
    pub fn fan_in<S: Into<String>>(
        source_agents: Vec<(S, Arc<dyn Agent>)>,
        target_agent: (S, Arc<dyn Agent>),
    ) -> Self {
        let (target_id, target_agent) = target_agent;
        let target_id = target_id.into();

        let mut builder = Self::new();
        let mut source_ids = Vec::new();

        for (id, agent) in source_agents {
            let id = id.into();
            source_ids.push(id.clone());
            builder = builder.add_agent(id, agent);
        }

        builder = builder.add_agent(target_id.clone(), target_agent);

        let steps = vec![
            // Step 0: All source agents run concurrently
            super::config::ExecutionStep {
                agent_ids: source_ids,
                concurrent: true,
                dependencies: vec![],
            },
            // Step 1: Target agent runs alone, depends on all sources
            super::config::ExecutionStep {
                agent_ids: vec![target_id],
                concurrent: false,
                dependencies: vec![0],
            },
        ];

        builder.hybrid(steps)
    }

    /// Create a nested team pattern: team_agent_a → team_agent_b, with team_agent_c running concurrently
    /// team_agent_leader aggregates results from team_agent_b and team_agent_c
    pub fn nested_team_pattern<S: Into<String>>(
        team_a: (S, Arc<dyn Agent>),
        team_b: (S, Arc<dyn Agent>),
        team_c: (S, Arc<dyn Agent>),
        team_leader: (S, Arc<dyn Agent>),
    ) -> Self {
        let (id_a, agent_a) = team_a;
        let (id_b, agent_b) = team_b;
        let (id_c, agent_c) = team_c;
        let (id_leader, agent_leader) = team_leader;

        let id_a = id_a.into();
        let id_b = id_b.into();
        let id_c = id_c.into();
        let id_leader = id_leader.into();

        let steps = vec![
            // Step 0: Team A runs alone
            super::config::ExecutionStep {
                agent_ids: vec![id_a.clone()],
                concurrent: false,
                dependencies: vec![],
            },
            // Step 1: Team B and Team C run concurrently, B depends on A, C runs independently
            super::config::ExecutionStep {
                agent_ids: vec![id_b.clone(), id_c.clone()],
                concurrent: true,
                dependencies: vec![0], // Both depend on step 0 (team A)
            },
            // Step 2: Team Leader runs alone, depends on step 1 (teams B and C)
            super::config::ExecutionStep {
                agent_ids: vec![id_leader.clone()],
                concurrent: false,
                dependencies: vec![1],
            },
        ];

        Self::new()
            .add_team_agent(id_a, agent_a)
            .add_team_agent(id_b, agent_b)
            .add_team_agent(id_c, agent_c)
            .add_team_agent(id_leader, agent_leader)
            .hybrid(steps)
    }

    /// Create a complex multi-layer team pattern
    pub fn multi_layer_team<S: Into<String>>(
        layer1_agents: Vec<(S, Arc<dyn Agent>)>,
        layer2_teams: Vec<(S, Arc<dyn Agent>)>,
        final_coordinator: (S, Arc<dyn Agent>),
    ) -> Self {
        let mut builder = Self::new();
        let mut layer1_ids = Vec::new();
        let mut layer2_ids = Vec::new();

        // Add layer 1 agents
        for (id, agent) in layer1_agents {
            let id = id.into();
            layer1_ids.push(id.clone());
            builder = builder.add_agent(id, agent);
        }

        // Add layer 2 team agents
        for (id, team) in layer2_teams {
            let id = id.into();
            layer2_ids.push(id.clone());
            builder = builder.add_team_agent(id, team);
        }

        // Add final coordinator
        let (coordinator_id, coordinator_agent) = final_coordinator;
        let coordinator_id = coordinator_id.into();
        builder = builder.add_team_agent(coordinator_id.clone(), coordinator_agent);

        let steps = vec![
            // Step 0: Layer 1 agents run concurrently
            super::config::ExecutionStep {
                agent_ids: layer1_ids,
                concurrent: true,
                dependencies: vec![],
            },
            // Step 1: Layer 2 teams run concurrently, depend on layer 1
            super::config::ExecutionStep {
                agent_ids: layer2_ids,
                concurrent: true,
                dependencies: vec![0],
            },
            // Step 2: Final coordinator runs alone, depends on layer 2
            super::config::ExecutionStep {
                agent_ids: vec![coordinator_id],
                concurrent: false,
                dependencies: vec![1],
            },
        ];

        builder.hybrid(steps)
    }
}

/// Builder for creating team-human hybrid agents
pub struct TeamHumanAgentBuilder {
    team_builder: TeamAgentBuilder,
    human_config: HumanAgentConfig,
    intervene_before_team: bool,
    intervene_after_team: bool,
    intervene_on_team_error: bool,
    interface: Option<Box<dyn HumanInteractionInterface>>,
}

impl TeamHumanAgentBuilder {
    /// Create a new team-human agent builder
    pub fn new() -> Self {
        Self {
            team_builder: TeamAgentBuilder::new(),
            human_config: HumanAgentConfig::new(),
            intervene_before_team: true,
            intervene_after_team: false,
            intervene_on_team_error: true,
            interface: None,
        }
    }

    /// Add a child agent to the team
    pub fn add_agent<S: Into<String>>(mut self, id: S, agent: Arc<dyn Agent>) -> Self {
        self.team_builder = self.team_builder.add_agent(id, agent);
        self
    }

    /// Add a team agent as a child (nested team)
    pub fn add_team_agent<S: Into<String>>(mut self, id: S, team_agent: Arc<dyn Agent>) -> Self {
        self.team_builder = self.team_builder.add_team_agent(id, team_agent);
        self
    }

    /// Set the execution pattern
    pub fn execution_pattern(mut self, pattern: ExecutionPattern) -> Self {
        self.team_builder = self.team_builder.execution_pattern(pattern);
        self
    }

    /// Set execution to concurrent
    pub fn concurrent(mut self) -> Self {
        self.team_builder = self.team_builder.concurrent();
        self
    }

    /// Set execution to sequential
    pub fn sequential(mut self) -> Self {
        self.team_builder = self.team_builder.sequential();
        self
    }

    /// Add an intervention condition
    pub fn add_intervention_condition(mut self, condition: InterventionCondition) -> Self {
        self.human_config = self.human_config.add_intervention_condition(condition);
        self
    }

    /// Add a termination condition
    pub fn add_termination_condition(mut self, condition: TerminationCondition) -> Self {
        self.human_config = self.human_config.add_termination_condition(condition);
        self
    }

    /// Set whether to intervene before team execution
    pub fn intervene_before_team(mut self, intervene: bool) -> Self {
        self.intervene_before_team = intervene;
        self
    }

    /// Set whether to intervene after team execution
    pub fn intervene_after_team(mut self, intervene: bool) -> Self {
        self.intervene_after_team = intervene;
        self
    }

    /// Set whether to intervene on team errors
    pub fn intervene_on_team_error(mut self, intervene: bool) -> Self {
        self.intervene_on_team_error = intervene;
        self
    }

    /// Set custom human interaction interface
    pub fn interface(mut self, interface: Box<dyn HumanInteractionInterface>) -> Self {
        self.interface = Some(interface);
        self
    }

    /// Set maximum interventions
    pub fn max_interventions(mut self, max: u32) -> Self {
        self.human_config = self.human_config.with_max_interventions(max);
        self
    }

    /// Set input timeout
    pub fn input_timeout(mut self, timeout_seconds: u64) -> Self {
        self.human_config = self.human_config.with_input_timeout(timeout_seconds);
        self
    }

    /// Set memory for both team and human components
    pub fn memory(mut self, memory: Arc<tokio::sync::Mutex<dyn crate::schemas::memory::BaseMemory>>) -> Self {
        // Set memory for team component
        self.team_builder = self.team_builder.memory(memory.clone());
        // Set memory for human component
        self.human_config = self.human_config.with_memory(memory);
        self
    }

    /// Set whether to include memory in human prompts
    pub fn include_memory_in_prompts(mut self, include: bool) -> Self {
        self.human_config = self.human_config.with_include_memory_in_prompts(include);
        self
    }

    /// Set whether to use coordination prompts in team
    pub fn coordination_prompts(mut self, use_coordination_prompts: bool) -> Self {
        self.team_builder = self.team_builder.coordination_prompts(use_coordination_prompts);
        self
    }

    /// Build the team-human agent
    pub fn build(self) -> Result<TeamHumanAgent, AgentError> {
        // Build team config from team builder
        let team_config = self.team_builder.config;

        // Create team-human config
        let config = TeamHumanAgentConfig::new(team_config, self.human_config)
            .with_intervene_before_team(self.intervene_before_team)
            .with_intervene_after_team(self.intervene_after_team)
            .with_intervene_on_team_error(self.intervene_on_team_error);

        // Create agent
        if let Some(interface) = self.interface {
            TeamHumanAgent::with_interface(config, interface)
        } else {
            TeamHumanAgent::new(config)
        }
    }

    /// Build the team-human agent and wrap it as a tool
    pub fn build_as_tool<S: Into<String>>(
        self,
        name: S,
        description: S,
    ) -> Result<super::team_human::TeamHumanAgentTool, AgentError> {
        let agent = Arc::new(self.build()?);
        Ok(super::team_human::TeamHumanAgentTool::new(
            agent,
            name,
            description,
        ))
    }

    /// Build the team-human agent and wrap it as a tool with auto-generated name/description
    pub fn build_as_auto_tool(self) -> Result<super::team_human::TeamHumanAgentTool, AgentError> {
        let agent = Arc::new(self.build()?);
        Ok(super::team_human::TeamHumanAgentTool::from_agent(agent))
    }
}

impl Default for TeamHumanAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}
