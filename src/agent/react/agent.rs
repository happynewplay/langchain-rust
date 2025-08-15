use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::{
    agent::{agent::Agent, AgentError},
    chain::chain_trait::Chain,
    message_formatter,
    prompt::{
        HumanMessagePromptTemplate, MessageFormatterStruct, MessageOrTemplate, PromptArgs,
        PromptFromatter,
    },
    prompt_args,
    schemas::{
        agent::{AgentAction, AgentEvent},
        messages::Message,
    },
    template_jinja2,
    tools::Tool,
};

use super::output_parser::ReActOutputParser;

/// ReAct Agent that performs autonomous reasoning-action cycles
pub struct ReActAgent {
    pub(crate) chain: Box<dyn Chain>,
    pub(crate) tools: Vec<Arc<dyn Tool>>,
    pub(crate) output_parser: ReActOutputParser,
}

impl ReActAgent {
    /// Create a prompt template for the ReAct agent
    pub fn create_prompt(
        tools: &[Arc<dyn Tool>],
        suffix: &str,
        prefix: &str,
    ) -> Result<MessageFormatterStruct, AgentError> {
        let tool_string = tools
            .iter()
            .map(|tool| format!("{}: {}", tool.name(), tool.description()))
            .collect::<Vec<_>>()
            .join("\n");
        let tool_names = tools
            .iter()
            .map(|tool| tool.name())
            .collect::<Vec<_>>()
            .join(", ");

        let suffix_prompt = template_jinja2!(suffix, "tools", "tool_names", "chat_history");

        let input_variables = prompt_args! {
            "tools" => tool_string,
            "tool_names" => tool_names,
            "chat_history" => "",
        };

        let suffix_prompt = suffix_prompt.format(input_variables)?;
        let formatter = message_formatter![
            MessageOrTemplate::Message(Message::new_system_message(prefix)),
            MessageOrTemplate::MessagesPlaceholder("chat_history".to_string()),
            MessageOrTemplate::Template(
                Box::new(HumanMessagePromptTemplate::new(template_jinja2!(suffix_prompt, "input", "agent_scratchpad")))
            )
        ];

        Ok(formatter)
    }

    /// Construct the agent scratchpad from intermediate steps
    fn construct_scratchpad(&self, intermediate_steps: &[(AgentAction, String)]) -> Result<String, AgentError> {
        let mut thoughts = Vec::new();
        
        for (action, observation) in intermediate_steps {
            // Parse the log to extract the thought if present
            let log_parts: Vec<&str> = action.log.split('\n').collect();
            
            // Add the thought and action from the log
            for part in log_parts {
                if part.trim().starts_with("Thought:") || 
                   part.trim().starts_with("Action:") || 
                   part.trim().starts_with("Action Input:") {
                    thoughts.push(part.trim().to_string());
                }
            }
            
            // Add the observation
            thoughts.push(format!("Observation: {}", observation));
        }
        
        if thoughts.is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("\n{}", thoughts.join("\n")))
        }
    }
}

#[async_trait]
impl Agent for ReActAgent {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        let scratchpad = self.construct_scratchpad(intermediate_steps)?;
        let mut inputs = inputs.clone();
        inputs.insert("agent_scratchpad".to_string(), json!(scratchpad));
        
        let output = self.chain.call(inputs.clone()).await?.generation;
        let parsed_output = self.output_parser.parse(&output)?;
        
        Ok(parsed_output)
    }

    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}
