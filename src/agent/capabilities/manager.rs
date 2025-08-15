use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use async_trait::async_trait;
use serde_json::Value;

use crate::{
    agent::AgentError,
    prompt::PromptArgs,
    schemas::agent::{AgentAction, AgentEvent},
    tools::Tool,
};

use super::{
    AgentCapability, ActionContext, ProcessedResult
};

/// Manages a collection of agent capabilities
pub struct CapabilityManager {
    capabilities: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    capability_names: HashMap<TypeId, &'static str>,
    capability_priorities: HashMap<TypeId, i32>,
}

impl CapabilityManager {
    /// Create a new empty capability manager
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            capability_names: HashMap::new(),
            capability_priorities: HashMap::new(),
        }
    }
    
    /// Add a capability to the manager
    pub fn add_capability<T: AgentCapability + 'static>(&mut self, capability: T) -> &mut Self {
        let type_id = TypeId::of::<T>();
        let name = capability.capability_name();
        let priority = 0; // Default priority
        
        self.capability_names.insert(type_id, name);
        self.capability_priorities.insert(type_id, priority);
        self.capabilities.insert(type_id, Box::new(capability));
        self
    }
    
    /// Add a capability with a specific priority
    pub fn add_capability_with_priority<T: AgentCapability + 'static>(
        &mut self, 
        capability: T, 
        priority: i32
    ) -> &mut Self {
        let type_id = TypeId::of::<T>();
        let name = capability.capability_name();
        
        self.capability_names.insert(type_id, name);
        self.capability_priorities.insert(type_id, priority);
        self.capabilities.insert(type_id, Box::new(capability));
        self
    }
    
    /// Get a capability by type
    pub fn get_capability<T: AgentCapability + 'static>(&self) -> Option<&T> {
        self.capabilities
            .get(&TypeId::of::<T>())
            .and_then(|cap| cap.downcast_ref::<T>())
    }
    
    /// Get a mutable capability by type
    pub fn get_capability_mut<T: AgentCapability + 'static>(&mut self) -> Option<&mut T> {
        self.capabilities
            .get_mut(&TypeId::of::<T>())
            .and_then(|cap| cap.downcast_mut::<T>())
    }
    
    /// Check if a capability exists
    pub fn has_capability<T: AgentCapability + 'static>(&self) -> bool {
        self.capabilities.contains_key(&TypeId::of::<T>())
    }
    
    /// Remove a capability
    pub fn remove_capability<T: AgentCapability + 'static>(&mut self) -> Option<T> {
        let type_id = TypeId::of::<T>();
        self.capability_names.remove(&type_id);
        self.capability_priorities.remove(&type_id);
        self.capabilities
            .remove(&type_id)
            .and_then(|cap| cap.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }
    
    /// List all capability names
    pub fn list_capabilities(&self) -> Vec<&'static str> {
        self.capability_names.values().copied().collect()
    }
    
    /// Get the number of capabilities
    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
    
    /// Check if any capabilities are present
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
    
    /// Get all tools provided by capabilities
    pub fn get_all_tools(&self) -> Vec<Arc<dyn Tool>> {
        // For now, return empty vector - tools will be provided by specific capability implementations
        Vec::new()
    }
    
    /// Apply pre-planning enhancements from all capabilities
    pub async fn apply_pre_plan_enhancements(
        &self,
        _intermediate_steps: &[(AgentAction, String)],
        _inputs: &mut PromptArgs,
    ) -> Result<(), AgentError> {
        // Simplified implementation - specific capability types will handle their own enhancement
        Ok(())
    }

    /// Apply post-planning enhancements from all capabilities
    pub async fn apply_post_plan_enhancements(
        &self,
        _intermediate_steps: &[(AgentAction, String)],
        _inputs: &PromptArgs,
        _event: &mut AgentEvent,
    ) -> Result<(), AgentError> {
        // Simplified implementation - specific capability types will handle their own enhancement
        Ok(())
    }

    /// Process action results through all capable processors
    pub async fn process_action_results(
        &self,
        _action: &AgentAction,
        result: &str,
        _context: &ActionContext,
    ) -> Result<ProcessedResult, AgentError> {
        // Simplified implementation - return the result unchanged
        Ok(ProcessedResult {
            modified_result: Some(result.to_string()),
            additional_context: None,
            should_continue: true,
        })
    }

    /// Initialize all capabilities that require initialization
    pub async fn initialize_capabilities(&mut self, _config: Value) -> Result<(), AgentError> {
        // Simplified implementation - specific capability types will handle their own initialization
        Ok(())
    }

    /// Cleanup all capabilities that require cleanup
    pub async fn cleanup_capabilities(&mut self) -> Result<(), AgentError> {
        // Simplified implementation - specific capability types will handle their own cleanup
        Ok(())
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CapabilityManager {
    fn drop(&mut self) {
        // Note: We can't call async cleanup in Drop, so this is just for logging
        if !self.is_empty() {
            log::debug!("CapabilityManager dropped with {} capabilities", self.capability_count());
        }
    }
}
