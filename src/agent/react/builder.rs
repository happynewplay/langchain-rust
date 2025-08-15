use std::sync::Arc;

use crate::{
    agent::AgentError,
    chain::{options::ChainCallOptions, LLMChainBuilder},
    language_models::llm::LLM,
    tools::Tool,
};

use super::{agent::ReActAgent, output_parser::ReActOutputParser, prompt::{REACT_PREFIX, REACT_SUFFIX}};

/// Builder for creating ReAct agents
pub struct ReActAgentBuilder {
    tools: Option<Vec<Arc<dyn Tool>>>,
    prefix: Option<String>,
    suffix: Option<String>,
    options: Option<ChainCallOptions>,
}

impl ReActAgentBuilder {
    /// Create a new ReAct agent builder
    pub fn new() -> Self {
        Self {
            tools: None,
            prefix: None,
            suffix: None,
            options: None,
        }
    }

    /// Set the tools available to the agent
    pub fn tools(mut self, tools: &[Arc<dyn Tool>]) -> Self {
        self.tools = Some(tools.to_vec());
        self
    }

    /// Set a custom prefix for the agent prompt
    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set a custom suffix for the agent prompt
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Set chain call options
    pub fn options(mut self, options: ChainCallOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Build the ReAct agent
    pub fn build<L: Into<Box<dyn LLM>>>(self, llm: L) -> Result<ReActAgent, AgentError> {
        let tools = self.tools.unwrap_or_default();
        let prefix = self.prefix.unwrap_or_else(|| REACT_PREFIX.to_string());
        let suffix = self.suffix.unwrap_or_else(|| REACT_SUFFIX.to_string());

        let prompt = ReActAgent::create_prompt(&tools, &suffix, &prefix)?;
        let default_options = ChainCallOptions::default().with_max_tokens(2000);
        let chain = Box::new(
            LLMChainBuilder::new()
                .prompt(prompt)
                .llm(llm)
                .options(self.options.unwrap_or(default_options))
                .build()?,
        );

        Ok(ReActAgent {
            chain,
            tools,
            output_parser: ReActOutputParser::new(),
        })
    }
}

impl Default for ReActAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}
