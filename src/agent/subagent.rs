//! Sub-Agent (Handoff) system.
//!
//! Inspired by AstrBot's sub-agent architecture, this module allows the main
//! agent to delegate tasks to specialized sub-agents via handoff tools.
//!
//! # Architecture
//!
//! - **SubAgentDefinition**: Configuration for a sub-agent (name, system prompt,
//!   description, tool filters, optional model override).
//!
//! - **HandoffTool**: A tool registered with the main agent. When called by the
//!   model, it spawns a temporary sub-agent, runs it with the delegated input,
//!   and returns the result as the tool output.
//!
//! - **SubAgentOrchestrator**: Manages sub-agent definitions. Loads configs,
//!   creates HandoffTools, and handles the registration lifecycle.

use crate::agent::runner::{Agent, AgentConfig};
use crate::agent::tool_executor::{Tool, ToolError, ToolExecutor};
use crate::provider::Provider;
use crate::types::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

// ─── SubAgent Definition ────────────────────────────────────────────

fn default_enabled() -> bool {
    true
}

/// Configuration for a sub-agent that the main agent can hand off tasks to.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SubAgentDefinition {
    pub name: String,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default)]
    pub system_prompt: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub model: Option<String>,

    /// - `None` = ALL available tools
    /// - `Some([])` = no tools
    /// - `Some(["name", ...])` = only those
    #[serde(default)]
    pub tools: Option<Vec<String>>,

    #[serde(default)]
    pub max_tool_rounds: Option<u32>,
}

impl SubAgentDefinition {
    pub fn effective_description(&self) -> String {
        if !self.description.is_empty() {
            self.description.clone()
        } else if !self.system_prompt.is_empty() {
            let chars: String = self.system_prompt.chars().take(120).collect();
            if self.system_prompt.chars().count() > 120 {
                format!("{}...", chars)
            } else {
                chars
            }
        } else {
            format!("Delegate tasks to {} agent.", self.name)
        }
    }

    pub fn tool_name(&self) -> String {
        format!("transfer_to_{}", self.name)
    }
}

// ─── HandoffTool ────────────────────────────────────────────────────

/// A tool that delegates tasks to a sub-agent.
pub struct HandoffTool {
    definition: SubAgentDefinition,
    provider: Arc<dyn Provider>,
    base_config: AgentConfig,
    /// Shared tool executor from the main agent (contains all registered tools).
    tool_executor: Arc<ToolExecutor>,
    /// Shared notification channel for pushing background sub-agent results
    /// back into the main agent's conversation history.
    background_notify: Option<Arc<std::sync::Mutex<Vec<String>>>>,
    /// Shared metrics collector for tracking token usage across sub-agents.
    metrics: Option<std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>>,
    /// Metrics source for labeling token usage.
    metrics_source: Option<crate::metrics::TokenSource>,
}

impl HandoffTool {
    pub fn new(
        definition: SubAgentDefinition,
        provider: Arc<dyn Provider>,
        base_config: AgentConfig,
        tool_executor: Arc<ToolExecutor>,
        background_notify: Option<Arc<std::sync::Mutex<Vec<String>>>>,
        metrics: Option<std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>>,
        metrics_source: Option<crate::metrics::TokenSource>,
    ) -> Self {
        Self {
            definition,
            provider,
            base_config,
            tool_executor,
            background_notify,
            metrics,
            metrics_source,
        }
    }

    pub fn agent_name(&self) -> &str {
        &self.definition.name
    }

    /// Filter tool names based on the sub-agent's `tools` field.
    fn filtered_tool_names(&self) -> Option<Vec<String>> {
        match &self.definition.tools {
            None => None, // all tools
            Some(names) => {
                let filtered: Vec<String> = names
                    .iter()
                    .filter(|n| !n.starts_with("transfer_to_"))
                    .cloned()
                    .collect();
                Some(filtered)
            }
        }
    }
}

#[async_trait]
impl Tool for HandoffTool {
    fn definition(&self) -> ToolDefinition {
        let desc = self.definition.effective_description();
        ToolDefinition::function(self.definition.tool_name())
            .description(desc)
            .parameter_with_description(
                "input",
                ParameterType::String,
                true,
                Some("The task to delegate to the sub-agent."),
            )
            .parameter_with_description(
                "background_task",
                ParameterType::Boolean,
                false,
                Some("Set to true if the task may take noticeable time. Defaults to false."),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        #[derive(serde::Deserialize)]
        struct HandoffArgs {
            input: String,
            #[serde(default)]
            background_task: bool,
        }

        let handoff_args: HandoffArgs = serde_json::from_str(args)
            .map_err(|e| ToolError::InvalidArguments(format!("Bad args: {}", e)))?;

        let input = handoff_args.input.trim().to_string();
        if input.is_empty() {
            return Err(ToolError::InvalidArguments(
                "'input' is required".to_string(),
            ));
        }

        let max_rounds = self
            .definition
            .max_tool_rounds
            .unwrap_or(self.base_config.max_tool_rounds);

        if handoff_args.background_task {
            // ── Background execution path ──
            let task_id = uuid::Uuid::new_v4().to_string();
            let name = self.definition.name.clone();
            let notify = self.background_notify.clone();

            // Clone everything needed for the spawned task
            let provider = self.provider.clone();
            let base_config = self.base_config.clone();
            let definition = self.definition.clone();
            let tool_executor = self.tool_executor.clone();
            let filtered_names = self.filtered_tool_names();

            let task_id_clone = task_id.clone();
            let metrics_clone = self.metrics.clone();
            let source_clone = self.metrics_source.clone();
            tokio::spawn(async move {
                tracing::info!(
                    subagent = %name,
                    task_id = %task_id_clone,
                    input_len = input.len(),
                    "HandoffTool: starting background sub-agent task"
                );

                let mut sub_config = base_config.clone();
                sub_config.max_tool_rounds = max_rounds;

                let mut sub_agent = Agent::with_config_via_arc(provider.clone(), sub_config);

                if let Some(ref m) = metrics_clone {
                    sub_agent.set_metrics(m.clone());
                }
                if let Some(ref ms) = source_clone {
                    sub_agent.set_metrics_source(ms.clone());
                }

                if !definition.system_prompt.is_empty() {
                    sub_agent.set_system_prompt(&definition.system_prompt);
                }

                // Register filtered tools on the sub-agent
                let all_tool_map = tool_executor.tools_map();
                let filtered_map: HashMap<String, Arc<dyn Tool>> = match filtered_names {
                    None => all_tool_map
                        .into_iter()
                        .filter(|(n, _)| !n.starts_with("transfer_to_"))
                        .collect(),
                    Some(names) => {
                        let name_set: std::collections::HashSet<&str> =
                            names.iter().map(|s| s.as_str()).collect();
                        all_tool_map
                            .into_iter()
                            .filter(|(n, _)| name_set.contains(n.as_str()))
                            .collect()
                    }
                };
                sub_agent.register_tools_from_map(filtered_map);

                let user_msg = ChatMessage::user(&input);
                let start = std::time::Instant::now();
                let result = match sub_agent.chat_with_message(user_msg).await {
                    Ok(response) => {
                        let duration = start.elapsed();
                        let text = response
                            .choices
                            .first()
                            .and_then(|c| c.message.content.as_ref())
                            .and_then(|c| c.as_text_full())
                            .unwrap_or_default();
                        tracing::info!(
                            subagent = %name,
                            task_id = %task_id_clone,
                            duration_ms = duration.as_millis(),
                            result_len = text.len(),
                            "HandoffTool: background sub-agent completed"
                        );
                        text
                    }
                    Err(e) => {
                        tracing::error!(
                            subagent = %name,
                            task_id = %task_id_clone,
                            error = %e,
                            "HandoffTool: background sub-agent failed"
                        );
                        format!("Error: {}", e)
                    }
                };

                // Push result back into the main conversation
                let notification = format!(
                    "[Background task completed] sub-agent '{}' (task_id={}):\n\n{}",
                    name, task_id_clone, result
                );
                if let Some(notify) = notify {
                    if let Ok(mut guard) = notify.lock() {
                        guard.push(notification);
                    }
                }
            });

            Ok(format!(
                "Background task submitted to sub-agent '{}'. task_id={}. You will be notified when it finishes.",
                self.definition.name, task_id
            ))
        } else {
            // ── Synchronous execution path (original behaviour) ──
            tracing::info!(
                subagent = %self.definition.name,
                input_len = input.len(),
                "HandoffTool: delegating to sub-agent"
            );

            let mut sub_config = self.base_config.clone();
            sub_config.max_tool_rounds = max_rounds;

            let mut sub_agent = Agent::with_config_via_arc(self.provider.clone(), sub_config);

            if let Some(ref m) = self.metrics {
                sub_agent.set_metrics(m.clone());
            }
            if let Some(ref ms) = self.metrics_source {
                sub_agent.set_metrics_source(ms.clone());
            }

            if !self.definition.system_prompt.is_empty() {
                sub_agent.set_system_prompt(&self.definition.system_prompt);
            }

            // Register filtered tools on the sub-agent
            let all_tool_map = self.tool_executor.tools_map();
            let filtered_map: HashMap<String, Arc<dyn Tool>> = match self.filtered_tool_names() {
                None => all_tool_map
                    .into_iter()
                    .filter(|(name, _)| !name.starts_with("transfer_to_"))
                    .collect(),
                Some(names) => {
                    let name_set: std::collections::HashSet<&str> =
                        names.iter().map(|s| s.as_str()).collect();
                    all_tool_map
                        .into_iter()
                        .filter(|(name, _)| name_set.contains(name.as_str()))
                        .collect()
                }
            };
            sub_agent.register_tools_from_map(filtered_map);

            // Run the sub-agent
            let user_msg = ChatMessage::user(&input);
            let start = std::time::Instant::now();
            let response = sub_agent
                .chat_with_message(user_msg)
                .await
                .map_err(|e| ToolError::ExecutionError(format!("Sub-agent failed: {}", e)))?;

            let duration = start.elapsed();
            let result_text = response
                .choices
                .first()
                .and_then(|c| c.message.content.as_ref())
                .and_then(|c| c.as_text_full())
                .unwrap_or_default();

            tracing::info!(
                subagent = %self.definition.name,
                duration_ms = duration.as_millis(),
                result_len = result_text.len(),
                "HandoffTool: sub-agent completed"
            );

            Ok(format!(
                "[{} sub-agent result]\n\n{}",
                self.definition.name, result_text
            ))
        }
    }
}

// ─── SubAgent Orchestrator ──────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct SubAgentOrchestratorConfig {
    #[serde(default)]
    pub main_enable: bool,

    #[serde(default)]
    pub remove_main_duplicate_tools: bool,

    #[serde(default)]
    pub router_system_prompt: String,

    #[serde(default)]
    pub agents: Vec<SubAgentDefinition>,
}

pub struct SubAgentOrchestrator {
    config: SubAgentOrchestratorConfig,
}

impl SubAgentOrchestrator {
    pub fn new() -> Self {
        Self {
            config: SubAgentOrchestratorConfig::default(),
        }
    }

    pub fn load_from_config(&mut self, config: SubAgentOrchestratorConfig) {
        self.config = config;
        if self.config.main_enable {
            let enabled_count = self.config.agents.iter().filter(|a| a.enabled).count();
            tracing::info!(
                total = self.config.agents.len(),
                enabled = enabled_count,
                "SubAgentOrchestrator: loaded"
            );
            for agent in &self.config.agents {
                if agent.enabled {
                    tracing::info!(name = %agent.name, "SubAgentOrchestrator: registered");
                }
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.main_enable && self.config.agents.iter().any(|a| a.enabled)
    }

    pub fn create_handoff_tools(
        &self,
        provider: Arc<dyn Provider>,
        base_config: &AgentConfig,
        tool_executor: Arc<ToolExecutor>,
        background_notify: Option<Arc<std::sync::Mutex<Vec<String>>>>,
        metrics: Option<std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>>,
        metrics_source: Option<crate::metrics::TokenSource>,
    ) -> Vec<HandoffTool> {
        if !self.is_enabled() {
            return Vec::new();
        }
        self.config
            .agents
            .iter()
            .filter(|a| a.enabled)
            .map(|def| {
                HandoffTool::new(
                    def.clone(),
                    provider.clone(),
                    base_config.clone(),
                    tool_executor.clone(),
                    background_notify.clone(),
                    metrics.clone(),
                    metrics_source.clone(),
                )
            })
            .collect()
    }

    pub fn router_prompt(&self) -> Option<String> {
        if !self.is_enabled() {
            return None;
        }
        if !self.config.router_system_prompt.is_empty() {
            return Some(self.config.router_system_prompt.clone());
        }
        let mut parts = vec![
            "## Available Sub-Agents".to_string(),
            "Use the `transfer_to_<name>` tools to delegate tasks:".to_string(),
            String::new(),
        ];
        for agent in &self.config.agents {
            if agent.enabled {
                parts.push(format!(
                    "- **{}**: {}",
                    agent.name,
                    agent.effective_description()
                ));
            }
        }
        Some(parts.join("\n"))
    }

    pub fn assigned_tool_names(&self) -> Vec<String> {
        if !self.config.remove_main_duplicate_tools {
            return Vec::new();
        }
        let mut names = Vec::new();
        for agent in &self.config.agents {
            if !agent.enabled {
                continue;
            }
            if let Some(tools) = &agent.tools {
                for t in tools {
                    if !t.starts_with("transfer_to_") {
                        names.push(t.clone());
                    }
                }
            }
        }
        names
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &SubAgentOrchestratorConfig {
        &self.config
    }

    /// Get mutable reference to the config (for API handlers).
    #[allow(dead_code)]
    pub fn config_mut(&mut self) -> &mut SubAgentOrchestratorConfig {
        &mut self.config
    }
}

impl Default for SubAgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name() {
        let def = SubAgentDefinition {
            name: "researcher".into(),
            enabled: true,
            system_prompt: "You are a researcher.".into(),
            description: String::new(),
            model: None,
            tools: None,
            max_tool_rounds: None,
        };
        assert_eq!(def.tool_name(), "transfer_to_researcher");
    }

    #[test]
    fn test_description_explicit() {
        let def = SubAgentDefinition {
            name: "coder".into(),
            enabled: true,
            system_prompt: "You write code.".into(),
            description: "Write and review code.".into(),
            model: None,
            tools: None,
            max_tool_rounds: None,
        };
        assert_eq!(def.effective_description(), "Write and review code.");
    }

    #[test]
    fn test_description_fallback() {
        let def = SubAgentDefinition {
            name: "writer".into(),
            enabled: true,
            system_prompt: "You are a writer.".into(),
            description: String::new(),
            model: None,
            tools: None,
            max_tool_rounds: None,
        };
        assert_eq!(def.effective_description(), "You are a writer.");
    }

    #[test]
    fn test_orchestrator_empty() {
        let orch = SubAgentOrchestrator::new();
        assert!(!orch.is_enabled());
    }
}
