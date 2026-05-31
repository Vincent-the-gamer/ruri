use crate::agent::skill::Skill;
use crate::agent::skill::SkillPackageSkill;
use crate::agent::tool_executor::{Tool, ToolError, ToolExecutor};
use crate::provider::{Provider, ProviderError};
use crate::transport::HttpTransport;
use crate::types::*;
use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use std::collections::HashMap;
use std::sync::Arc;

/// Maximum number of tool-call round-trips before forcing a stop.
const MAX_TOOL_ROUNDS: u32 = 10;

/// Configuration for the Agent.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Maximum number of tool-call round-trips before forcing a stop.
    pub max_tool_rounds: u32,
    /// Whether to automatically execute tool calls.
    pub auto_execute_tools: bool,
    /// Controls which (if any) tool the model should call.
    /// When `None`, the API default (`"auto"`) is used.
    pub tool_choice: Option<crate::types::ToolChoice>,
    /// When `Some(true)`, the model may return multiple tool calls in a single
    /// response so that independent tools can be invoked in parallel.
    /// When `None`, the API default is used.
    pub parallel_tool_calls: Option<bool>,
    /// Whether the LLM's extended thinking (chain-of-thought reasoning) is enabled.
    /// When disabled, the model will not spend extra tokens on internal reasoning.
    /// Defaults to `true`.
    pub thinking_enabled: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_tool_rounds: MAX_TOOL_ROUNDS,
            auto_execute_tools: true,
            tool_choice: None,
            parallel_tool_calls: None,
            thinking_enabled: true,
        }
    }
}

impl AgentConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_tool_rounds(mut self, rounds: u32) -> Self {
        self.max_tool_rounds = rounds;
        self
    }

    pub fn with_auto_execute_tools(mut self, auto: bool) -> Self {
        self.auto_execute_tools = auto;
        self
    }

    /// Returns the message to display when the maximum tool rounds limit is reached.
    pub fn max_rounds_reached_message(&self) -> String {
        format!(
            "⚠️ Maximum tool call rounds ({}) reached, stopping.",
            self.max_tool_rounds
        )
    }
}

/// The core Agent that orchestrates everything.
///
/// An Agent ties together:
/// - A **Provider** (via HTTP Transport) for communicating with AI models
/// - A **System Prompt** (persona) — the single source of truth for the
///   model's identity and core behavior, injected as the sole system message
/// - **Skills** that dynamically inject context into user messages and
///   process messages at various lifecycle hooks
/// - **Tools** that the model can invoke and the agent can execute
///
/// # Architecture: Persona vs. Skills vs. RAG
///
/// Following the Agent specification:
///
/// - **Persona** is the **system prompt** — the only system message in the
///   conversation. It defines the model's identity, personality, and core
///   behavioral guidelines. Set via [`Agent::set_system_prompt`].
///
/// - **Skills** (including RAG/knowledge base) dynamically inject their
///   context into user messages, NOT as system messages. Each skill's
///   `on_attach()` returns context text that is collected and prepended
///   to user messages on every turn. This ensures the persona is never
///   overridden and skills can be added/removed without side effects.
///
/// - **RAG** (knowledge base) is a special skill that augments user messages
///   with retrieved context. It follows the same dynamic injection pattern.
pub struct Agent {
    transport: HttpTransport,
    tool_executor: ToolExecutor,
    skills: Vec<Arc<dyn Skill>>,
    config: AgentConfig,
    /// Conversation history maintained across turns.
    history: Vec<ChatMessage>,
    /// The persona / system prompt — the single source of truth for model identity.
    /// Injected as the sole system message at the start of every request.
    system_prompt: Option<String>,
    /// Collected context strings from skills' `on_attach()`.
    /// These are dynamically injected into user messages each turn,
    /// NOT as system messages.
    skill_contexts: Vec<String>,
    /// Whether `initialize_skills()` has been called at least once.
    /// Prevents duplicate initialization.
    skills_initialized: bool,
    /// Skill index for routing: all available skills' (name, description, when_to_use).
    /// This is used to build a routing instruction in the system prompt so
    /// the model knows which skills are available and prioritizes them over
    /// built-in tools.
    available_skill_index: Vec<SkillIndexEntry>,
    /// Full configuration for on-demand skills that can be dynamically loaded.
    /// Stores the complete skill config so invoke_skill tool can load them.
    /// Shared via Arc<RwLock> so InvokeSkillTool can access it.
    available_skill_configs: Arc<std::sync::RwLock<HashMap<String, AvailableSkillConfig>>>,
    /// Optional cancellation token — when cancelled, the agent loop stops
    /// between rounds (not mid-API-call). Set by the caller before invoking
    /// `chat_with_message` to allow `/stop` to fully terminate the agent.
    cancel_token: Option<tokio_util::sync::CancellationToken>,
    /// Optional channel for requesting tool execution permission.
    /// When set, before executing a tool, the agent sends (tool_name, args)
    /// through the channel and waits for a response (true = allow, false = deny).
    tool_permission_tx:
        Option<tokio::sync::mpsc::Sender<(String, String, tokio::sync::oneshot::Sender<bool>)>>,
    /// Optional channel for notifying external listeners about tool execution
    /// status. (tool_name, arguments_preview) is sent just before a tool runs.
    /// Used by platform handlers to show tool execution feedback to users.
    tool_notify_tx: Option<tokio::sync::mpsc::UnboundedSender<(String, String)>>,
    /// Metrics collector for tracking token usage and request counts.
    metrics: Option<std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>>,
    /// Source of the metrics (debug_session / profile / acp) for token source tracking.
    metrics_source: Option<crate::metrics::TokenSource>,
}

/// An entry in the skill index, used for skill routing.
#[derive(Debug, Clone)]
struct SkillIndexEntry {
    name: String,
    description: String,
    when_to_use: Option<String>,
}

/// Full configuration for an on-demand skill that can be dynamically loaded.
#[derive(Debug, Clone)]
struct AvailableSkillConfig {
    /// Skill name
    name: String,
    /// Skill description
    description: String,
    /// When to use condition
    when_to_use: Option<String>,
    /// Full skill configuration JSON
    config: serde_json::Value,
}

/// A tool that allows the model to dynamically invoke on-demand skills.
///
/// When the model calls this tool, the skill's full content is loaded and
/// returned to the model, allowing it to use the skill's capabilities.
struct InvokeSkillTool {
    configs: std::sync::Arc<std::sync::RwLock<HashMap<String, AvailableSkillConfig>>>,
    shell_command_blacklist: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
}

impl InvokeSkillTool {
    fn new(
        configs: std::sync::Arc<std::sync::RwLock<HashMap<String, AvailableSkillConfig>>>,
        shell_command_blacklist: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
    ) -> Self {
        Self {
            configs,
            shell_command_blacklist,
        }
    }
}

#[async_trait]
impl Tool for InvokeSkillTool {
    fn definition(&self) -> ToolDefinition {
        // Build description from available skill names
        let skill_names: Vec<String> = self.configs.read().unwrap().keys().cloned().collect();
        let desc = format!(
            "Dynamically load and invoke an on-demand skill. \
             Use this tool when the user request matches one of the available on-demand skills. \
             The skill's full content will be loaded and returned to you. \
             Available skills: {:?}",
            skill_names
        );

        ToolDefinition::function("invoke_skill")
            .description(desc)
            .parameter_with_description(
                "skill_name",
                ParameterType::String,
                true,
                Some("Name of the skill to invoke"),
            )
            .parameter_with_description(
                "arguments",
                ParameterType::String,
                false,
                Some("Arguments to pass to the skill (optional)"),
            )
            .build()
    }

    async fn execute(&self, args: &str) -> Result<String, ToolError> {
        #[derive(serde::Deserialize)]
        struct InvokeArgs {
            skill_name: String,
            #[serde(default)]
            #[allow(dead_code)] // Reserved for future use when skills need parameters
            arguments: Option<String>,
        }

        let invoke_args: InvokeArgs =
            serde_json::from_str(args).map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

        // Note: arguments field is reserved for future use when skills need parameters

        let skill_config = {
            let configs = self.configs.read().unwrap();
            match configs.get(&invoke_args.skill_name) {
                Some(config) => config.clone(),
                None => {
                    let available: Vec<String> = configs.keys().cloned().collect();
                    return Err(ToolError::NotFound(format!(
                        "Skill '{}' not found. Available skills: {:?}",
                        invoke_args.skill_name, available
                    )));
                }
            }
        };

        // Build the skill from config
        let skill = SkillPackageSkill::from_config(
            skill_config.name.clone(),
            skill_config.description.clone(),
            &skill_config.config,
        );

        // Load the skill's prompt/content via on_attach (no shell/hooks yet)
        let context_messages = skill.on_attach().await;

        // Extract the content
        let mut content_parts = Vec::new();
        content_parts.push(format!(
            "📦 Skill '{}' loaded successfully.",
            skill_config.name
        ));

        if let Some(when) = &skill_config.when_to_use {
            content_parts.push(format!("When to use: {}", when));
        }

        for msg in context_messages {
            if let Some(ref content) = msg.content {
                if let Some(text) = content.as_text_full() {
                    if !text.is_empty() {
                        content_parts.push(format!("\n---\n{}\n---", text));
                    }
                }
            }
        }

        // Execute shell and hooks ONLY when the skill is explicitly invoked.
        // This runs both in a single call (no duplication).
        // The blacklist is enforced for all commands (shell + hooks) to prevent
        // dangerous operations even when the AI agent tries alternative approaches.
        let blacklist = self.shell_command_blacklist.read().await.clone();
        let shell_and_hook_outputs = skill.execute_shell_and_hooks(&blacklist).await;
        if !shell_and_hook_outputs.is_empty() {
            content_parts.push("\n## Execution Output:".to_string());
            content_parts.extend(shell_and_hook_outputs);
        }

        Ok(content_parts.join("\n"))
    }
}

impl Agent {
    /// Create a new Agent with custom configuration.
    pub fn with_config(provider: Box<dyn Provider>, config: AgentConfig) -> Self {
        let tool_executor = ToolExecutor::new();
        let available_skill_configs = Arc::new(std::sync::RwLock::new(HashMap::new()));
        Self {
            transport: HttpTransport::with_default_config(provider),
            tool_executor,
            skills: Vec::new(),
            config,
            history: Vec::new(),
            system_prompt: None,
            skill_contexts: Vec::new(),
            skills_initialized: false,
            available_skill_index: Vec::new(),
            available_skill_configs,
            cancel_token: None,
            tool_permission_tx: None,
            tool_notify_tx: None,
            metrics: None,
            metrics_source: None,
        }
    }

    /// Set the metrics collector for tracking token usage and request counts.
    pub fn set_metrics(
        &mut self,
        metrics: std::sync::Arc<tokio::sync::RwLock<crate::metrics::MetricsCollector>>,
    ) {
        self.transport.set_metrics(metrics.clone());
        self.metrics = Some(metrics);
    }

    /// Set the metrics source for token source tracking.
    /// Also propagates to the transport layer.
    pub fn set_metrics_source(&mut self, source: crate::metrics::TokenSource) {
        self.transport.set_metrics_source(source.clone());
        self.metrics_source = Some(source);
    }

    /// Set the cancellation token for this agent.
    ///
    /// When the token is cancelled, the agent loop will stop between
    /// rounds (not mid-API-call), ensuring `/stop` fully terminates
    /// the agent instead of just cancelling the current tool call.
    pub fn set_cancel_token(&mut self, token: tokio_util::sync::CancellationToken) {
        self.cancel_token = Some(token);
    }

    /// Set the tool permission channel for this agent.
    ///
    /// When set, before executing each tool call, the agent sends
    /// `(tool_name, arguments, oneshot_sender)` through this channel and
    /// waits for a response. `true` allows execution, `false` denies it.
    pub fn set_tool_permission_tx(
        &mut self,
        tx: tokio::sync::mpsc::Sender<(String, String, tokio::sync::oneshot::Sender<bool>)>,
    ) {
        self.tool_permission_tx = Some(tx);
    }

    /// Set the tool notification channel. When set, (tool_name, arguments_preview)
    /// is sent before each tool executes, allowing platform handlers to notify users.
    pub fn set_tool_notify_tx(&mut self, tx: tokio::sync::mpsc::UnboundedSender<(String, String)>) {
        self.tool_notify_tx = Some(tx);
    }

    /// Set the system prompt (persona) for this agent.
    ///
    /// The system prompt is the **only** system message in the conversation.
    /// It defines the model's identity and core behavioral guidelines.
    /// All other context (skills, RAG, etc.) is dynamically injected into
    /// user messages, never as system messages.
    pub fn set_system_prompt(&mut self, prompt: impl Into<String>) {
        self.system_prompt = Some(prompt.into());
    }

    /// Add a skill to the agent.
    ///
    /// Skills augment the agent's behavior by dynamically injecting context
    /// into user messages. Their `on_attach()` context is collected (not
    /// injected as system messages) and prepended to user messages on each
    /// turn via the skill's `on_user_message()` hook.
    pub fn add_skill(&mut self, skill: Arc<dyn Skill>) {
        self.skills.push(skill);
    }

    /// Add an "available" skill to the index for skill routing.
    ///
    /// Available skills are NOT fully loaded (their `on_attach()` is not called)
    /// and their context is NOT injected into user messages. Instead, they are
    /// listed in a skill index that is appended to the system prompt, allowing
    /// the model to know they exist and when to use them.
    ///
    /// If a user's message matches an available skill (by name/description/when_to_use),
    /// the model is instructed to prioritize invoking that skill over other tools.
    ///
    /// Note: Prefer [`add_available_skill_with_config`] for full functionality.
    #[allow(dead_code)]
    pub fn add_available_skill(
        &mut self,
        name: String,
        description: String,
        when_to_use: Option<String>,
    ) {
        self.available_skill_index.push(SkillIndexEntry {
            name: name.clone(),
            description: description.clone(),
            when_to_use: when_to_use.clone(),
        });
    }

    /// Add an "available" skill with full configuration for dynamic loading.
    ///
    /// This is similar to [`add_available_skill`], but also stores the complete
    /// skill configuration so that the `invoke_skill` tool can dynamically load
    /// the skill's full content when the model decides to use it.
    pub fn add_available_skill_with_config(
        &mut self,
        name: String,
        description: String,
        when_to_use: Option<String>,
        config: serde_json::Value,
    ) {
        // Add to index for routing
        self.available_skill_index.push(SkillIndexEntry {
            name: name.clone(),
            description: description.clone(),
            when_to_use: when_to_use.clone(),
        });
        // Store full config for dynamic loading
        let mut configs = self.available_skill_configs.write().unwrap();
        configs.insert(
            name.clone(),
            AvailableSkillConfig {
                name,
                description,
                when_to_use,
                config,
            },
        );
    }

    /// Register the invoke_skill tool that allows dynamic loading of on-demand skills.
    /// Call this after adding all available skills.
    pub fn register_invoke_skill_tool(
        &mut self,
        shell_command_blacklist: std::sync::Arc<tokio::sync::RwLock<Vec<String>>>,
    ) {
        let configs = Arc::clone(&self.available_skill_configs);
        let tool = InvokeSkillTool::new(configs, shell_command_blacklist);
        self.register_tool(Arc::new(tool));
    }

    /// Build a skill routing instruction string that lists all available skills.
    ///
    /// This is appended to the system prompt so the model knows which skills
    /// are available and should be prioritized.
    fn build_skill_routing_instruction(&self) -> Option<String> {
        if self.available_skill_index.is_empty() && self.skills.is_empty() {
            return None;
        }

        let mut parts = Vec::new();

        // ── Active skills (always-on, context injected) ──
        let active_skills: Vec<&Arc<dyn Skill>> =
            self.skills.iter().filter(|s| s.is_active()).collect();

        if !active_skills.is_empty() {
            parts.push("## Available Skills".to_string());
            parts.push(
                "The following skills are always active and their context is \
                 automatically available. When the user request matches a skill, \
                 you should prioritize using that skill over other tools."
                    .to_string(),
            );
            for skill in &active_skills {
                let name = skill.name();
                let desc = skill.description();
                if !name.is_empty() {
                    if desc.is_empty() {
                        parts.push(format!("- **{}** (active)", name));
                    } else {
                        parts.push(format!("- **{}**: {} (active)", name, desc));
                    }
                }
            }
            parts.push(String::new());
        }

        // ── On-demand skills (conditional, loaded only when matched) ──
        let on_demand_skills: Vec<&Arc<dyn Skill>> =
            self.skills.iter().filter(|s| !s.is_active()).collect();

        let has_on_demand = !on_demand_skills.is_empty() || !self.available_skill_index.is_empty();

        if has_on_demand {
            if active_skills.is_empty() {
                parts.push("## Available Skills".to_string());
            }
            parts.push("## On-Demand Skills".to_string());
            parts.push(
                "The following skills are available on demand. Use them ONLY when \
                 the user request matches their \"Use when\" condition or description. \
                 Do NOT invoke them for unrelated requests."
                    .to_string(),
            );

            // List on-demand skills from self.skills (inactive, have when_to_use)
            for skill in &on_demand_skills {
                let name = skill.name();
                let desc = skill.description();
                if !name.is_empty() {
                    let mut line = format!("- **{}**", name);
                    if !desc.is_empty() {
                        line.push_str(&format!(": {}", desc));
                    }
                    if let Some(when) = skill.when_to_use() {
                        if !when.is_empty() {
                            line.push_str(&format!(" - Use when: {}", when));
                        }
                    }
                    parts.push(line);
                }
            }

            // List available skills from the index
            for entry in &self.available_skill_index {
                let mut line = format!("- **{}**", entry.name);
                if !entry.description.is_empty() {
                    line.push_str(&format!(": {}", entry.description));
                }
                if let Some(ref when) = entry.when_to_use {
                    if !when.is_empty() {
                        line.push_str(&format!(" - Use when: {}", when));
                    }
                }
                parts.push(line);
            }

            parts.push(String::new());
        }

        // ── Tool calling priority & failure handling ─────────────────
        parts.push("## Tool Calling Priority (MUST FOLLOW)".to_string());
        parts.push(
            "CRITICAL: Before calling ANY tool, you MUST first check if any available skill can fulfill the user's request.\n\n\
            When processing a user request, follow this STRICT priority order:\n\n\
            1. **Skill (ALWAYS FIRST)** — Scan ALL listed skills (both active and on-demand). If ANY skill matches the user request by name, description, or \"when_to_use\" condition, use that skill FIRST via the invoke_skill tool. Skills are the PRIMARY capability mechanism. Do NOT skip this step.\n\
            2. **knowledge_base_search** — Only if NO skill matches AND the user's question might benefit from information stored in the configured knowledge bases. Use this to retrieve relevant context from the knowledge base. Always cite source documents when using knowledge base information.\n\
            3. **web_search** — Only if NO skill matches, knowledge base has no relevant information, AND the user needs external/real-time information or knowledge lookup.\n\
            4. **Dedicated tools** (read_file, write_file, grep, list_directory, etc.) — Use these specialized tools when they directly serve the request. Always prefer these over raw shell commands.\n\
            5. **python / shell / bash (LAST RESORT)** — ONLY use raw shell commands when absolutely no other tool or skill can fulfill the request. shell/bash should NEVER be your first choice. Always exhaust skills and dedicated tools first.\n\n\
            **Exception**: If the user explicitly specifies which tool or method to use (e.g. \"use web search\", \"run a shell command\", \"use the X skill\"), follow their instruction and skip the default priority."
                .to_string(),
        );

        parts.push(String::new());

        // ── Tool failure handling ──────────────────────────────────
        parts.push("## Tool Failure Handling".to_string());
        parts.push(
            "If a tool call returns an error or fails:\n\
            - Do NOT automatically fall back to other tools or try alternative methods on your own.\n\
            - Instead, clearly explain the failure to the user and ASK whether they would like you to try a different approach or tool.\n\
            - Wait for the user's confirmation before proceeding with any alternative."
                .to_string(),
        );

        Some(parts.join("\n"))
    }

    /// Override the `tool_choice` parameter for every request made by this agent.
    pub fn set_tool_choice(&mut self, choice: Option<crate::types::ToolChoice>) {
        self.config.tool_choice = choice;
    }

    /// Override the `parallel_tool_calls` parameter for every request made by this agent.
    pub fn set_parallel_tool_calls(&mut self, enabled: Option<bool>) {
        self.config.parallel_tool_calls = enabled;
    }

    /// Register a tool with the agent.
    pub fn register_tool(&mut self, tool: Arc<dyn crate::agent::tool_executor::Tool>) {
        self.tool_executor.register(tool);
    }

    /// Set the conversation history (for restoring from persistence).
    pub fn set_history(&mut self, history: Vec<ChatMessage>) {
        // Strip any existing system messages from history — the system prompt
        // is managed separately via `system_prompt` and will be injected
        // automatically by `build_request()`.
        self.history = history
            .into_iter()
            .filter(|m| m.role != MessageRole::System)
            .collect();
    }

    /// Run a single turn of the conversation with streaming output.
    ///
    /// This is similar to [`chat`], but instead of returning the full response,
    /// it streams [`StreamEvent`]s to the provided sender as they arrive.
    /// The agent remains owned by the caller and is updated in place.
    ///
    /// Returns a string indicating the stop reason (e.g. "stop", "length", "cancelled")
    /// when the turn completes, or an error.
    pub async fn chat_streaming(
        &mut self,
        user_message: impl Into<String>,
        tx: &tokio::sync::mpsc::Sender<Result<StreamEvent, ProviderError>>,
    ) -> Result<String, ProviderError> {
        let user_msg = ChatMessage::user(user_message);

        // Auto-initialize skills on first call
        if !self.skills_initialized {
            tracing::info!(
                "Skills not yet initialized, auto-initializing before first streaming chat"
            );
            self.initialize_skills().await;
        }

        // Add user message to history
        self.history.push(user_msg);

        // Run skill pre-processing
        self.run_skills_on_user_message().await;

        // Inject skill contexts into the user message
        self.inject_skill_contexts().await;

        // Tool loop
        let mut round = 0u32;
        let max_rounds = self.config.max_tool_rounds;
        // Track all executed tool+args combinations across rounds to prevent
        // infinite loops when the LLM keeps retrying the same failing tool.
        let mut executed_tool_keys: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        loop {
            // Check cancellation before each round
            if let Some(ref token) = self.cancel_token {
                if token.is_cancelled() {
                    tracing::info!(
                        round = round,
                        "Streaming agent loop cancelled via CancellationToken"
                    );
                    let _ = tx
                        .send(Ok(StreamEvent::Error {
                            error: "任务已停止".to_string(),
                        }))
                        .await;
                    return Ok("cancelled".to_string());
                }
            }

            let request = self.build_request();

            tracing::info!(
                round = round,
                provider = %self.transport.provider_name(),
                model = %self.transport.default_model(),
                messages = request.messages.len(),
                "Sending streaming chat request"
            );

            // Stream from the provider (with multimodal fallback)
            let request_clone = request.clone();
            let mut stream = self.transport.send_stream(request_clone);

            let mut has_tool_calls = false;
            let mut tool_calls_accum: Vec<AccumulatedToolCall> = Vec::new();
            let mut content_text = String::new();
            let mut _response_bytes: u64 = 0;

            while let Some(event_result) = stream.next().await {
                // Check cancellation at the top of each stream iteration
                if let Some(ref token) = self.cancel_token {
                    if token.is_cancelled() {
                        tracing::info!(
                            round = round,
                            "Streaming agent cancelled during stream processing"
                        );
                        let _ = tx
                            .send(Ok(StreamEvent::Error {
                                error: "任务已停止".to_string(),
                            }))
                            .await;
                        return Ok("cancelled".to_string());
                    }
                }

                match event_result {
                    Ok(event) => {
                        match &event {
                            StreamEvent::ToolCallStart {
                                tool_call_id,
                                function_name,
                            } => {
                                has_tool_calls = true;
                                tool_calls_accum.push(AccumulatedToolCall {
                                    id: tool_call_id.clone(),
                                    function_name: function_name.clone(),
                                    arguments: String::new(),
                                });
                            }
                            StreamEvent::ToolCallDelta {
                                tool_call_id,
                                arguments_delta,
                            } => {
                                _response_bytes += arguments_delta.len() as u64;
                                if let Some(tc) = tool_calls_accum
                                    .iter_mut()
                                    .find(|tc| &tc.id == tool_call_id)
                                {
                                    tc.arguments.push_str(arguments_delta);
                                }
                            }
                            StreamEvent::ToolCallEnd { .. } => {
                                // Tool call completed — now forwarded to consumers
                            }
                            StreamEvent::ContentDelta { delta } => {
                                content_text.push_str(delta);
                                _response_bytes += delta.len() as u64;
                            }
                            StreamEvent::Done { .. } => {
                                // Metrics (traffic, tokens) are recorded by HttpTransport::send_stream.
                            }
                            StreamEvent::ToolResult { .. } | StreamEvent::ToolExecuting { .. } => {
                                // Shouldn't happen from provider (synthesized by runner)
                            }
                            StreamEvent::Error { .. } => {
                                // Forward errors
                            }
                        }
                        // Forward all events except Done
                        if !matches!(event, StreamEvent::Done { .. }) {
                            if tx.send(Ok(event)).await.is_err() {
                                return Ok("stop".to_string());
                            }
                        }
                    }
                    Err(ProviderError::MultimodalNotSupported) => {
                        tracing::warn!(
                            "Streaming request failed because the model does not support multimodal. \
                             Retrying with image content stripped."
                        );
                        let stripped = request.strip_multimodal_content();
                        let mut retry_stream = self.transport.send_stream(stripped);

                        while let Some(retry_event) = retry_stream.next().await {
                            match retry_event {
                                Ok(event) => {
                                    match &event {
                                        StreamEvent::ToolCallStart {
                                            tool_call_id,
                                            function_name,
                                        } => {
                                            has_tool_calls = true;
                                            tool_calls_accum.push(AccumulatedToolCall {
                                                id: tool_call_id.clone(),
                                                function_name: function_name.clone(),
                                                arguments: String::new(),
                                            });
                                        }
                                        StreamEvent::ToolCallDelta {
                                            tool_call_id,
                                            arguments_delta,
                                        } => {
                                            _response_bytes += arguments_delta.len() as u64;
                                            if let Some(tc) = tool_calls_accum
                                                .iter_mut()
                                                .find(|tc| &tc.id == tool_call_id)
                                            {
                                                tc.arguments.push_str(arguments_delta);
                                            }
                                        }
                                        StreamEvent::ContentDelta { delta } => {
                                            content_text.push_str(delta);
                                            _response_bytes += delta.len() as u64;
                                        }
                                        StreamEvent::Done { .. } => {
                                            // Metrics (traffic, tokens) are recorded by HttpTransport::send_stream.
                                        }
                                        _ => {}
                                    }
                                    if !matches!(event, StreamEvent::Done { .. }) {
                                        if tx.send(Ok(event)).await.is_err() {
                                            return Ok("stop".to_string());
                                        }
                                    }
                                }
                                Err(e) => {
                                    let _ = tx.send(Err(e)).await;
                                    return Err(ProviderError::MultimodalNotSupported);
                                }
                            }
                        }
                        break; // Exit the retry, continue to tool handling below
                    }
                    Err(e) => {
                        // Send the error to the stream receiver
                        let error_msg = e.to_string();
                        let _ = tx.send(Err(e)).await;
                        return Err(ProviderError::Custom(error_msg));
                    }
                }
            }

            // If the provider stream was completely empty (no events at all),
            // treat it as an error rather than silently returning Done.
            if !has_tool_calls && content_text.is_empty() && tool_calls_accum.is_empty() {
                tracing::error!(
                    round = round,
                    provider = %self.transport.provider_name(),
                    "chat_streaming: provider returned an empty stream with no events"
                );
                let error_msg = "The model returned an empty response. Please try again.";
                if tx
                    .send(Ok(StreamEvent::Error {
                        error: error_msg.to_string(),
                    }))
                    .await
                    .is_err()
                {
                    return Ok("stop".to_string());
                }
                self.history
                    .push(ChatMessage::assistant(format!("⚠️ {}", error_msg)));
                break;
            }

            // Handle what happened in this round
            if has_tool_calls && self.config.auto_execute_tools {
                // Build assistant message with tool calls for history
                let tool_calls_for_history: Vec<ToolCall> = tool_calls_accum
                    .iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        call_type: crate::types::ToolCallType::Function,
                        function: FunctionCall {
                            name: tc.function_name.clone(),
                            arguments: tc.arguments.clone(),
                        },
                    })
                    .collect();

                let assistant_msg = ChatMessage::assistant_with_tool_calls(
                    if content_text.is_empty() {
                        None
                    } else {
                        Some(content_text)
                    },
                    tool_calls_for_history.clone(),
                );
                self.history.push(assistant_msg);

                // Execute each tool call, skipping duplicates across all rounds
                // to prevent infinite loops when the LLM keeps retrying the same
                // failing tool with the same arguments.
                for call in &tool_calls_for_history {
                    tracing::info!(tool = %call.function.name, "Executing tool call");

                    // Build a dedup key: tool_name + sorted args.
                    // If the LLM calls the same tool with the same arguments twice
                    // in the same round, we skip the duplicate.
                    let dedup_key = format!("{}|{}", call.function.name, call.function.arguments);
                    if !executed_tool_keys.insert(dedup_key.clone()) {
                        tracing::info!(
                            tool = %call.function.name,
                            round = round,
                            "Skipping duplicate tool call in same round"
                        );
                        // Add a synthetic result so the LLM knows we skipped it
                        let skip_msg = format!(
                            "⏭ Skipped duplicate call to `{}` — already executed in this round.",
                            call.function.name
                        );
                        self.history
                            .push(ChatMessage::tool_result(&call.id, &skip_msg));
                        let _ = tx
                            .send(Ok(StreamEvent::ToolResult {
                                tool_call_id: call.id.clone(),
                                tool_name: call.function.name.clone(),
                                content: skip_msg,
                            }))
                            .await;
                        continue;
                    }

                    // Check permission if a permission channel is configured
                    let allowed = if let Some(ref perm_tx) = self.tool_permission_tx {
                        let (resp_tx, resp_rx) = tokio::sync::oneshot::channel();
                        if perm_tx
                            .send((
                                call.function.name.clone(),
                                call.function.arguments.clone(),
                                resp_tx,
                            ))
                            .await
                            .is_err()
                        {
                            tracing::warn!("Permission channel closed, denying tool execution");
                            false
                        } else {
                            match resp_rx.await {
                                Ok(allowed) => allowed,
                                Err(_) => {
                                    tracing::warn!(
                                        "Permission response dropped, denying tool execution"
                                    );
                                    false
                                }
                            }
                        }
                    } else {
                        true
                    };

                    // ── Notify the client immediately that a tool is about to execute ──
                    let args_preview = if call.function.arguments.len() > 200 {
                        let end = call.function.arguments.floor_char_boundary(200);
                        format!("{}...", &call.function.arguments[..end])
                    } else {
                        call.function.arguments.clone()
                    };
                    if tx
                        .send(Ok(StreamEvent::ToolExecuting {
                            tool_call_id: call.id.clone(),
                            tool_name: call.function.name.clone(),
                            arguments_preview: args_preview,
                        }))
                        .await
                        .is_err()
                    {
                        return Ok("stop".to_string());
                    }

                    // Add a natural-language progress message to history so the user
                    // sees what's happening and can correct if needed.
                    let progress_msg = format!(
                        "Let me use `{}` to help you with this...",
                        call.function.name
                    );
                    self.history.push(ChatMessage::assistant(&progress_msg));
                    // Also send it as a content delta for the user to see
                    let _ = tx
                        .send(Ok(StreamEvent::ContentDelta {
                            delta: progress_msg,
                        }))
                        .await;

                    let result = if !allowed {
                        ToolResult {
                            tool_call_id: call.id.clone(),
                            content: "Permission denied by user".to_string(),
                        }
                    } else if let Some(ref token) = self.cancel_token {
                        tokio::select! {
                            r = self.tool_executor.execute_with_id(&call.id, &call.function) => r,
                            _ = token.cancelled() => {
                                tracing::info!(
                                    tool = %call.function.name,
                                    round = round,
                                    "Streaming tool execution cancelled via CancellationToken"
                                );
                                ToolResult {
                                    tool_call_id: call.id.clone(),
                                    content: "⏹ 任务已停止。".to_string(),
                                }
                            }
                        }
                    } else {
                        self.tool_executor
                            .execute_with_id(&call.id, &call.function)
                            .await
                    };

                    // Notify skills
                    for skill in &self.skills {
                        if skill.is_active() {
                            skill
                                .on_tool_result(&call.function.name, &result.content)
                                .await;
                        }
                    }

                    // Add tool result to history
                    self.history.push(ChatMessage::tool_result(
                        &result.tool_call_id,
                        &result.content,
                    ));

                    // Emit tool result event
                    if tx
                        .send(Ok(StreamEvent::ToolResult {
                            tool_call_id: result.tool_call_id,
                            tool_name: call.function.name.clone(),
                            content: result.content,
                        }))
                        .await
                        .is_err()
                    {
                        return Ok("stop".to_string());
                    }

                    // Check cancellation after each tool call
                    if let Some(ref token) = self.cancel_token {
                        if token.is_cancelled() {
                            tracing::info!(
                                tool = %call.function.name,
                                round = round,
                                "Streaming agent loop cancelled after tool execution"
                            );
                            let _ = tx
                                .send(Ok(StreamEvent::Error {
                                    error: "任务已停止".to_string(),
                                }))
                                .await;
                            return Ok("cancelled".to_string());
                        }
                    }
                }

                round += 1;
                if round >= max_rounds {
                    tracing::warn!(rounds = round, "Maximum tool rounds reached, stopping");
                    let warning = self.config.max_rounds_reached_message();
                    self.history.push(ChatMessage::assistant(&warning));
                    if tx
                        .send(Ok(StreamEvent::ContentDelta { delta: warning }))
                        .await
                        .is_err()
                    {
                        return Ok("length".to_string());
                    }
                    return Ok("length".to_string());
                }

                // Check cancellation between rounds
                if let Some(ref token) = self.cancel_token {
                    if token.is_cancelled() {
                        tracing::info!(
                            round = round,
                            "Streaming agent loop cancelled between rounds"
                        );
                        let _ = tx
                            .send(Ok(StreamEvent::Error {
                                error: "任务已停止".to_string(),
                            }))
                            .await;
                        return Ok("cancelled".to_string());
                    }
                }

                // Loop back for next round
            } else {
                // No tool calls — add the assistant message to history
                self.history.push(ChatMessage::assistant(&content_text));
                break;
            }
        }

        // Run skill post-processing
        self.run_skills_on_response().await;

        // Emit final Done event
        let _ = tx.send(Ok(StreamEvent::Done { usage: None })).await;

        Ok("stop".to_string())
    }

    /// Run a single turn with a pre-constructed message.
    pub async fn chat_with_message(
        &mut self,
        message: ChatMessage,
    ) -> Result<ChatResponse, crate::provider::ProviderError> {
        // Auto-initialize skills on first call if not done explicitly.
        if !self.skills_initialized {
            tracing::info!("Skills not yet initialized, auto-initializing before first chat");
            self.initialize_skills().await;
        }

        // Add user message to history
        self.history.push(message);

        // Run skill pre-processing (including dynamic context injection)
        self.run_skills_on_user_message().await;

        // Inject collected skill contexts into the user message
        self.inject_skill_contexts().await;

        // Build request and run the tool loop
        let mut round = 0u32;
        // Track all executed tool+args combinations across rounds to prevent
        // infinite loops when the LLM keeps retrying the same failing tool.
        let mut executed_tool_keys: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let response = 'agent_loop: loop {
            // Check cancellation before each round — if /stop was invoked,
            // abort immediately instead of continuing to the next tool round.
            if let Some(ref token) = self.cancel_token {
                if token.is_cancelled() {
                    tracing::info!(round = round, "Agent loop cancelled via CancellationToken");
                    let stopped_msg = ChatMessage::assistant("⏹ 任务已停止。");
                    self.history.push(stopped_msg.clone());
                    let stopped_response = ChatResponse {
                        id: None,
                        object: Some("chat.completion".to_string()),
                        model: None,
                        choices: vec![crate::types::Choice {
                            index: 0,
                            message: stopped_msg,
                            finish_reason: Some("stop".to_string()),
                        }],
                        usage: None,
                        extra: serde_json::Map::new(),
                    };
                    break 'agent_loop stopped_response;
                }
            }

            let request = self.build_request();

            tracing::info!(
                round = round,
                provider = %self.transport.provider_name(),
                model = %self.transport.default_model(),
                messages = request.messages.len(),
                "Sending chat request"
            );

            // Send the request to the model. Use tokio::select! with the
            // cancellation token so that /stop can cancel a slow LLM API call.
            let send_result = if let Some(ref token) = self.cancel_token {
                tokio::select! {
                    result = self.transport.send(request) => result,
                    _ = token.cancelled() => {
                        tracing::info!(round = round, "Agent LLM request cancelled via CancellationToken");
                        let stopped_msg = ChatMessage::assistant("⏹ 任务已停止。");
                        self.history.push(stopped_msg.clone());
                        let stopped_response = ChatResponse {
                            id: None,
                            object: Some("chat.completion".to_string()),
                            model: None,
                            choices: vec![crate::types::Choice {
                                index: 0,
                                message: stopped_msg,
                                finish_reason: Some("stop".to_string()),
                            }],
                            usage: None,
                            extra: serde_json::Map::new(),
                        };
                        break 'agent_loop stopped_response;
                    }
                }
            } else {
                self.transport.send(request).await
            };
            let mut response = send_result?;

            // Check if the model wants to call tools
            let choice = &response.choices[0];
            let tool_calls = choice.message.tool_calls.clone();

            match tool_calls {
                Some(calls) if self.config.auto_execute_tools && !calls.is_empty() => {
                    // Add assistant's tool-call message to history
                    let assistant_msg = choice.message.clone();
                    self.history.push(assistant_msg);

                    // Execute each tool call, skipping duplicates across all rounds
                    // to prevent infinite loops when the LLM keeps retrying the same
                    // failing tool with the same arguments.
                    for call in &calls {
                        tracing::info!(tool = %call.function.name, "Executing tool call");

                        // Build a dedup key: tool_name + args.
                        // If the LLM calls the same tool with the same arguments twice
                        // in the same round, skip the duplicate.
                        let dedup_key =
                            format!("{}|{}", call.function.name, call.function.arguments);
                        if !executed_tool_keys.insert(dedup_key.clone()) {
                            tracing::info!(
                                tool = %call.function.name,
                                round = round,
                                "Skipping duplicate tool call in same round"
                            );
                            let skip_msg = format!(
                                "⏭ Skipped duplicate call to `{}` — already executed in this round.",
                                call.function.name
                            );
                            self.history
                                .push(ChatMessage::tool_result(&call.id, &skip_msg));
                            // Check cancellation after skip too
                            if let Some(ref token) = self.cancel_token {
                                if token.is_cancelled() {
                                    break 'agent_loop ChatResponse {
                                        id: None,
                                        object: Some("chat.completion".to_string()),
                                        model: None,
                                        choices: vec![crate::types::Choice {
                                            index: 0,
                                            message: ChatMessage::assistant("⏹ 任务已停止。"),
                                            finish_reason: Some("stop".to_string()),
                                        }],
                                        usage: None,
                                        extra: serde_json::Map::new(),
                                    };
                                }
                            }
                            continue;
                        }

                        // Notify external listeners (e.g. platform handlers) that a
                        // tool is about to execute, so users get immediate feedback.
                        if let Some(ref tx) = self.tool_notify_tx {
                            let args_preview = if call.function.arguments.len() > 200 {
                                let end = call.function.arguments.floor_char_boundary(200);
                                format!("{}...", &call.function.arguments[..end])
                            } else {
                                call.function.arguments.clone()
                            };
                            let _ = tx.send((call.function.name.clone(), args_preview));
                        }

                        // Use tokio::select! so that /stop can cancel a
                        // long-running tool call (e.g. web_search) mid-execution.
                        let result = if let Some(ref token) = self.cancel_token {
                            tokio::select! {
                                r = self.tool_executor.execute_with_id(&call.id, &call.function) => r,
                                _ = token.cancelled() => {
                                    tracing::info!(
                                        tool = %call.function.name,
                                        round = round,
                                        "Tool execution cancelled via CancellationToken"
                                    );
                                    // Return a synthetic cancelled result so we can
                                    // break out gracefully after this iteration.
                                    ToolResult {
                                        tool_call_id: call.id.clone(),
                                        content: "⏹ 任务已停止。".to_string(),
                                    }
                                }
                            }
                        } else {
                            self.tool_executor
                                .execute_with_id(&call.id, &call.function)
                                .await
                        };

                        // Notify skills
                        for skill in &self.skills {
                            if skill.is_active() {
                                skill
                                    .on_tool_result(&call.function.name, &result.content)
                                    .await;
                            }
                        }

                        // Add tool result to history
                        self.history.push(ChatMessage::tool_result(
                            &result.tool_call_id,
                            &result.content,
                        ));

                        // Check cancellation after each individual tool call —
                        // don't wait for the remaining tools in this round.
                        if let Some(ref token) = self.cancel_token {
                            if token.is_cancelled() {
                                tracing::info!(
                                    tool = %call.function.name,
                                    round = round,
                                    "Agent loop cancelled after tool execution via CancellationToken"
                                );
                                let stopped_msg = ChatMessage::assistant("⏹ 任务已停止。");
                                self.history.push(stopped_msg.clone());
                                let stopped_response = ChatResponse {
                                    id: None,
                                    object: Some("chat.completion".to_string()),
                                    model: None,
                                    choices: vec![crate::types::Choice {
                                        index: 0,
                                        message: stopped_msg,
                                        finish_reason: Some("stop".to_string()),
                                    }],
                                    usage: None,
                                    extra: serde_json::Map::new(),
                                };
                                break 'agent_loop stopped_response;
                            }
                        }
                    }

                    round += 1;
                    if round >= self.config.max_tool_rounds {
                        tracing::warn!(rounds = round, "Maximum tool rounds reached, stopping");
                        // Instead of returning a blank response (the last API response
                        // only contained tool calls), inject a meaningful message so the
                        // user isn't left with empty content.
                        let warning = self.config.max_rounds_reached_message();
                        let assistant_msg = ChatMessage::assistant(&warning);
                        self.history.push(assistant_msg.clone());
                        // Patch the response so the caller sees the warning text.
                        response.choices[0].message = assistant_msg;
                        break 'agent_loop response;
                    }
                    // Loop back to get the model's next response
                }
                _ => {
                    // No tool calls — add to history and return
                    self.history.push(choice.message.clone());
                    break 'agent_loop response;
                }
            }
        };

        // Run skill post-processing on the final assistant message
        self.run_skills_on_response().await;

        Ok(response)
    }

    /// Run a single turn with a pre-constructed message and return
    /// the response along with tool-related messages from the history.
    ///
    /// This is the same as [`chat_with_message`], but additionally returns
    /// all tool-call messages (assistant messages with `tool_calls`) and
    /// tool-result messages from the history that were added during this turn.
    /// These can be used for DB persistence of tool interactions.
    pub async fn chat_with_message_and_tool_history(
        &mut self,
        message: ChatMessage,
    ) -> Result<(ChatResponse, Vec<ChatMessage>), crate::provider::ProviderError> {
        let history_len_before = self.history.len();
        let response = self.chat_with_message(message).await?;

        // Extract tool-related messages that were added during this turn.
        // This includes assistant messages with tool_calls and tool result messages.
        let tool_messages: Vec<ChatMessage> = self
            .history
            .iter()
            .skip(history_len_before)
            .filter(|m| {
                // Skip the initial user message and the final assistant text message
                // (those are persisted separately). Include only:
                // - Assistant messages that have tool_calls (intermediate tool-call rounds)
                // - Tool result messages
                m.role == MessageRole::Tool
                    || (m.role == MessageRole::Assistant
                        && m.tool_calls.is_some()
                        && !m.tool_calls.as_ref().unwrap().is_empty())
            })
            .cloned()
            .collect();

        Ok((response, tool_messages))
    }

    /// Build a ChatRequest from the current history and configuration.
    ///
    /// The system prompt (persona) is injected as the **first** message
    /// in the request, ensuring it is the sole system message. Skill contexts
    /// have already been dynamically injected into user messages by
    /// `inject_skill_contexts()`.
    ///
    /// If the provider does not support multimodal content, image content
    /// parts are automatically stripped from messages and a warning is logged.
    fn build_request(&self) -> ChatRequest {
        // Build the messages array with the system prompt as the first message
        let mut messages = Vec::new();

        // Detect current operating system for shell command hints
        let os_info = if cfg!(target_os = "windows") {
            "Operating System: Windows (Shell: PowerShell)"
        } else if cfg!(target_os = "macos") {
            "Operating System: macOS (Shell: bash)"
        } else if cfg!(target_os = "linux") {
            "Operating System: Linux (Shell: bash)"
        } else {
            "Operating System: Unknown (Shell: bash)"
        };

        // Inject the persona as the sole system message
        if let Some(ref prompt) = self.system_prompt {
            // Append skill routing instruction and OS info to the system prompt
            let mut full_prompt = prompt.clone();
            if let Some(routing) = self.build_skill_routing_instruction() {
                full_prompt.push_str(&format!("\n\n{}", routing));
            }
            full_prompt.push_str(&format!("\n\n{}", os_info));
            messages.push(ChatMessage::system(&full_prompt));
        } else if let Some(routing) = self.build_skill_routing_instruction() {
            // No persona but we have a skill routing instruction — still inject it
            // as a system message so the model knows about available skills.
            let mut full_prompt = routing;
            full_prompt.push_str(&format!("\n{}", os_info));
            messages.push(ChatMessage::system(&full_prompt));
        } else {
            // No persona and no routing instruction — inject OS info as system message
            messages.push(ChatMessage::system(os_info));
        }

        // Add conversation history (system messages already stripped by set_history)
        messages.extend(self.history.iter().cloned());

        let mut request = ChatRequest::new(messages).with_model(self.transport.default_model());

        // If the provider does not support multimodal content, strip images upfront.
        if !self.transport.supports_multimodal() {
            request = request.strip_multimodal_content();
        }

        // Add tools if any are registered
        let tool_defs = self.tool_executor.definitions();
        if !tool_defs.is_empty() {
            request = request.with_tools(tool_defs);

            if let Some(ref choice) = self.config.tool_choice {
                request = request.with_tool_choice(choice.clone());
            }
            if let Some(enabled) = self.config.parallel_tool_calls {
                request = request.with_parallel_tool_calls(enabled);
            }
        }

        // Apply thinking configuration: when disabled, instruct the model
        // not to use extended thinking / chain-of-thought reasoning.
        // This uses Anthropic's format (supported by many API-compatible providers).
        if !self.config.thinking_enabled {
            request.extra.insert(
                "thinking".to_string(),
                serde_json::json!({"type": "disabled"}),
            );
        }

        request
    }

    /// Initialize skills by collecting their context from `on_attach()`.
    ///
    /// Unlike the previous architecture where skills injected system messages,
    /// this method collects skill context strings that will be **dynamically
    /// injected into user messages** each turn. The persona (system prompt)
    /// remains the sole system message.
    pub async fn initialize_skills(&mut self) {
        if self.skills_initialized {
            tracing::debug!(
                skills_count = self.skills.len(),
                "Skills already initialized, skipping duplicate initialization"
            );
            return;
        }

        for skill in &self.skills {
            // Skip inactive skills (those with when_to_use conditions).
            // They are listed in the routing instruction for on-demand
            // invocation, but their context is NOT injected by default.
            if !skill.is_active() {
                tracing::info!(
                    skill = %skill.name(),
                    "Skipping inactive skill — will be available on demand via routing"
                );
                continue;
            }

            let attach_messages = skill.on_attach().await;
            tracing::info!(
                skill = %skill.name(),
                num_system_messages = attach_messages.len(),
                "Collecting skill context from on_attach"
            );
            // Extract text content from system messages returned by on_attach()
            // and store them as context strings for dynamic injection.
            // We do NOT inject them as system messages.
            for msg in attach_messages {
                if msg.role == MessageRole::System {
                    if let Some(ref content) = msg.content {
                        let text = content.as_text_full().unwrap_or_default();
                        if !text.is_empty() {
                            tracing::info!(
                                skill = %skill.name(),
                                context_len = text.len(),
                                "Collected context from skill on_attach"
                            );
                            self.skill_contexts.push(text);
                        }
                    }
                }
            }
        }

        self.skills_initialized = true;
    }

    /// Inject collected skill contexts into the last user message.
    ///
    /// This dynamically prepends context blocks from skills to the user's
    /// message, ensuring the model sees the relevant context without
    /// polluting the system prompt.
    async fn inject_skill_contexts(&mut self) {
        if self.skill_contexts.is_empty() {
            return;
        }

        // Find the last user message
        let last_user_idx = self
            .history
            .iter()
            .rposition(|m| m.role == MessageRole::User);

        if let Some(idx) = last_user_idx {
            let context_block = self
                .skill_contexts
                .iter()
                .filter(|c| !c.is_empty())
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");

            if context_block.is_empty() {
                return;
            }

            // Prepend context to the user message
            let msg = &mut self.history[idx];
            if let Some(ref content) = msg.content {
                let text = content.as_text_full().unwrap_or_default();
                let new_content = format!(
                    "[Skill Context]\n{}\n[/Skill Context]\n\n{}",
                    context_block, text
                );
                msg.content = Some(MessageContent::Text(new_content));
            }
        }
    }

    /// Run skill pre-processing on user messages.
    async fn run_skills_on_user_message(&mut self) {
        for skill in &self.skills {
            if skill.is_active() {
                tracing::info!(
                    skill = %skill.name(),
                    history_len = self.history.len(),
                    "Running skill on_user_message"
                );
                skill.on_user_message(&mut self.history).await;
            }
        }
    }

    /// Run skill post-processing on the last response.
    async fn run_skills_on_response(&mut self) {
        if let Some(last) = self.history.last_mut()
            && last.role == MessageRole::Assistant
        {
            for skill in &self.skills {
                if skill.is_active() {
                    tracing::info!(
                        skill = %skill.name(),
                        "Running skill on_response"
                    );
                    skill.on_response(last).await;
                }
            }
        }
    }

    /// Get a summary of the conversation history for forking.
    ///
    /// Concatenates the last few user/assistant messages (skipping tool
    /// messages) into a plain-text summary that can be injected into a
    /// forked session so the new agent has context from the previous
    /// conversation.
    pub fn get_conversation_summary(&self) -> String {
        if self.history.is_empty() {
            return String::new();
        }

        let mut summary_parts = Vec::new();
        let max_messages = 20;
        let start = if self.history.len() > max_messages {
            self.history.len() - max_messages
        } else {
            0
        };

        if start > 0 {
            summary_parts.push(format!(
                "(Previous conversation with {} messages summarized)",
                start
            ));
        }

        for msg in &self.history[start..] {
            match msg.role {
                MessageRole::User => {
                    let text = msg
                        .content
                        .as_ref()
                        .and_then(|c| c.as_text_full())
                        .unwrap_or_default();
                    let truncated: String = text.chars().take(500).collect();
                    summary_parts.push(format!("User: {}", truncated));
                }
                MessageRole::Assistant => {
                    // Skip assistant messages that are just tool-call wrappers
                    if msg.tool_calls.is_some() {
                        continue;
                    }
                    let text = msg
                        .content
                        .as_ref()
                        .and_then(|c| c.as_text_full())
                        .unwrap_or_default();
                    let truncated: String = text.chars().take(500).collect();
                    summary_parts.push(format!("Assistant: {}", truncated));
                }
                MessageRole::System | MessageRole::Tool => {
                    // Skip system and tool messages in summary
                }
            }
        }

        summary_parts.join("\n\n")
    }

    /// Inject a conversation summary as the initial history for a forked session.
    ///
    /// Adds the summary as the first user message with an assistant
    /// acknowledgment, so the new agent has context from the previous
    /// conversation while keeping the conversation history balanced.
    pub fn inject_history_summary(&mut self, summary: String) {
        if summary.is_empty() {
            return;
        }

        self.history.push(ChatMessage::user(format!(
            "[Conversation Context from Previous Session]\n\n{}",
            summary
        )));

        self.history.push(ChatMessage::assistant(
            "Understood. I have the context from the previous conversation. How can I help you continue?",
        ));
    }
}

/// Streaming version of the Agent that yields [`StreamEvent`]s as they arrive.
///
/// This is a standalone struct rather than a method on [`Agent`] because
/// streaming requires long-lived mutable access to the agent's state
/// (history, tool execution, etc.), which conflicts with Rust's borrow
/// checker when using `async_stream`.
///
/// Usage:
/// ```ignore
/// let streamer = AgentStreamer::new(agent, user_message);
/// let event_stream = streamer.into_stream();
/// // event_stream implements Stream<Item = Result<StreamEvent, ProviderError>>
/// ```
pub struct AgentStreamer {
    agent: Agent,
    user_message: ChatMessage,
}

impl AgentStreamer {
    /// Create a new streamer from an agent and a user message.
    pub fn new(agent: Agent, user_message: ChatMessage) -> Self {
        Self {
            agent,
            user_message,
        }
    }

    /// Consume the streamer and return a stream of [`StreamEvent`]s.
    ///
    /// The stream handles the full agent lifecycle:
    /// 1. Initialize skills (collect context, no system messages)
    /// 2. Pre-process the user message through skills (dynamic context injection)
    /// 3. Inject skill contexts into the user message
    /// 4. Stream the model's response (content + tool calls)
    /// 5. If tool calls are present, execute them and emit results
    /// 6. Loop back for more model responses (up to `max_tool_rounds`)
    /// 7. Post-process through skills
    /// 8. Emit `Done`
    pub fn into_stream(self) -> BoxStream<'static, Result<StreamEvent, ProviderError>> {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<StreamEvent, ProviderError>>(256);

        tokio::spawn(async move {
            use futures_util::FutureExt;
            use std::panic::AssertUnwindSafe;

            let result = AssertUnwindSafe(async {
                let mut agent = self.agent;
                let user_message = self.user_message;

                // Auto-initialize skills on first call if not done explicitly.
                if !agent.skills_initialized {
                    tracing::info!("Skills not yet initialized, auto-initializing before first stream");
                    agent.initialize_skills().await;
                }

                // Add user message to history
                agent.history.push(user_message);

                // Run skill pre-processing
                agent.run_skills_on_user_message().await;

                // Inject collected skill contexts into the user message
                agent.inject_skill_contexts().await;

                // Tool loop
                let mut round = 0u32;
                let max_rounds = agent.config.max_tool_rounds;
                // Track all executed tool+args combinations across rounds to prevent
                // infinite loops when the LLM keeps retrying the same failing tool.
                let mut executed_tool_keys: std::collections::HashSet<String> =
                    std::collections::HashSet::new();

                loop {
                    // Check cancellation before each round — if /stop was invoked,
                    // abort immediately instead of continuing to the next tool round.
                    if let Some(ref token) = agent.cancel_token {
                        if token.is_cancelled() {
                            tracing::info!(round = round, "Streaming agent loop cancelled via CancellationToken");
                            let _ = tx.send(Ok(StreamEvent::Error {
                                error: "任务已停止".to_string(),
                            })).await;
                            break;
                        }
                    }

                    let request = agent.build_request();

                    tracing::info!(
                        round = round,
                        provider = %agent.transport.provider_name(),
                        model = %agent.transport.default_model(),
                        messages = request.messages.len(),
                        "Sending streaming chat request"
                    );

                    // Stream from the provider (with multimodal fallback)
                    let mut stream = agent.transport.send_stream(request.clone());

                    let mut has_tool_calls = false;
                    let mut tool_calls_accum: Vec<AccumulatedToolCall> = Vec::new();
                    let mut content_text = String::new();
                    let mut _response_bytes: u64 = 0;

                    use futures_util::StreamExt;

                    // Helper: check cancellation and break out of the outer
                    // tool loop if the token has been cancelled.
                    macro_rules! check_stream_cancel {
                        () => {
                            if let Some(ref token) = agent.cancel_token {
                                if token.is_cancelled() {
                                    tracing::info!(round = round, "Streaming agent cancelled during stream processing");
                                    let _ = tx.send(Ok(StreamEvent::Error {
                                        error: "任务已停止".to_string(),
                                    })).await;
                                    return;
                                }
                            }
                        };
                    }

                    while let Some(event_result) = stream.next().await {
                        // Check cancellation at the top of each stream iteration.
                        // This ensures that even while streaming a response, the
                        // agent will stop promptly when /stop is invoked.
                        check_stream_cancel!();
                        match event_result {
                            Ok(event) => {
                                match &event {
                                    StreamEvent::ToolCallStart {
                                        tool_call_id,
                                        function_name,
                                    } => {
                                        has_tool_calls = true;
                                        tool_calls_accum.push(AccumulatedToolCall {
                                            id: tool_call_id.clone(),
                                            function_name: function_name.clone(),
                                            arguments: String::new(),
                                        });
                                    }
                                    StreamEvent::ToolCallDelta {
                                        tool_call_id,
                                        arguments_delta,
                                    } => {
                                        _response_bytes += arguments_delta.len() as u64;
                                        if let Some(tc) = tool_calls_accum
                                            .iter_mut()
                                            .find(|tc| &tc.id == tool_call_id)
                                        {
                                            tc.arguments.push_str(arguments_delta);
                                        }
                                    }
                                    StreamEvent::ToolCallEnd { .. } => {
                                        // Tool call completed — now forwarded to consumers
                                    }
                                    StreamEvent::ContentDelta { delta } => {
                                        content_text.push_str(delta);
                                        _response_bytes += delta.len() as u64;
                                    }
                                    StreamEvent::Done { .. } => {
                                        // Metrics (traffic, tokens) are recorded by HttpTransport::send_stream.
                                    }
                                    StreamEvent::ToolResult { .. } | StreamEvent::ToolExecuting { .. } => {
                                        // Shouldn't happen from provider, but forward anyway
                                    }
                                    StreamEvent::Error { .. } => {
                                        // Forward errors
                                    }
                                }
                                // Forward all events to the client except Done
                                // (we manage Done ourselves after tool execution)
                                if !matches!(event, StreamEvent::Done { .. }) {
                                    if tx.send(Ok(event)).await.is_err() {
                                        return; // receiver dropped
                                    }
                                }
                            }
                            Err(ProviderError::MultimodalNotSupported) => {
                                // The model doesn't support multimodal content.
                                // Retry with images stripped from the request.
                                tracing::warn!(
                                    "Streaming request failed because the model does not support multimodal. \
                                     Retrying with image content stripped."
                                );
                                let stripped = request.strip_multimodal_content();
                                let mut retry_stream = agent.transport.send_stream(stripped);

                                // Process the retry stream in-place
                                while let Some(retry_event) = retry_stream.next().await {
                                    match retry_event {
                                        Ok(event) => {
                                            match &event {
                                                StreamEvent::ToolCallStart {
                                                    tool_call_id,
                                                    function_name,
                                                } => {
                                                    has_tool_calls = true;
                                                    tool_calls_accum.push(AccumulatedToolCall {
                                                        id: tool_call_id.clone(),
                                                        function_name: function_name.clone(),
                                                        arguments: String::new(),
                                                    });
                                                }
                                                StreamEvent::ToolCallDelta {
                                                    tool_call_id,
                                                    arguments_delta,
                                                } => {
                                                    _response_bytes += arguments_delta.len() as u64;
                                                    if let Some(tc) = tool_calls_accum
                                                        .iter_mut()
                                                        .find(|tc| &tc.id == tool_call_id)
                                                    {
                                                        tc.arguments.push_str(arguments_delta);
                                                    }
                                                }
                                                StreamEvent::ContentDelta { delta } => {
                                                    content_text.push_str(delta);
                                                    _response_bytes += delta.len() as u64;
                                                }
                                                StreamEvent::Done { .. } => {
                                                    // Metrics (traffic, tokens) are recorded by HttpTransport::send_stream.
                                                }
                                                _ => {}
                                            }
                                            if !matches!(
                                                event,
                                                StreamEvent::Done { .. }
                                            ) {
                                                if tx.send(Ok(event)).await.is_err() {
                                                    return;
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(Err(e)).await;
                                            return;
                                        }
                                    }
                                }
                                break; // Exit the retry, continue to tool handling below
                            }
                            Err(e) => {
                                let _ = tx.send(Err(e)).await;
                                return;
                            }
                        }
                    }

                    // If the provider stream was completely empty (no events at all),
                    // treat it as an error rather than silently returning Done.
                    if !has_tool_calls && content_text.is_empty() && tool_calls_accum.is_empty() {
                        tracing::error!(
                            round = round,
                            provider = %agent.transport.provider_name(),
                            "Provider returned an empty stream with no events"
                        );
                        let error_msg = "The model returned an empty response. Please try again.";
                        if tx
                            .send(Ok(StreamEvent::Error {
                                error: error_msg.to_string(),
                            }))
                            .await
                            .is_err()
                        {
                            return;
                        }
                        agent.history.push(ChatMessage::assistant(format!(
                            "⚠️ {}",
                            error_msg
                        )));
                        break;
                    }

                    // Now handle what happened in this round
                    if has_tool_calls && agent.config.auto_execute_tools {
                        // Build assistant message with tool calls for history
                        let tool_calls_for_history: Vec<ToolCall> = tool_calls_accum
                            .iter()
                            .map(|tc| ToolCall {
                                id: tc.id.clone(),
                                call_type: crate::types::ToolCallType::Function,
                                function: FunctionCall {
                                    name: tc.function_name.clone(),
                                    arguments: tc.arguments.clone(),
                                },
                            })
                            .collect();

                        let assistant_msg = ChatMessage::assistant_with_tool_calls(
                            if content_text.is_empty() {
                                None
                            } else {
                                Some(content_text)
                            },
                            tool_calls_for_history.clone(),
                        );
                        agent.history.push(assistant_msg);

                        // Execute each tool call, skipping duplicates across all rounds
                        // to prevent infinite loops when the LLM keeps retrying the same
                        // failing tool with the same arguments.
                        for call in &tool_calls_for_history {
                            tracing::info!(tool = %call.function.name, "Executing tool call");

                            // Build a dedup key: tool_name + args.
                            // If the LLM calls the same tool with the same arguments twice
                            // in the same round, skip the duplicate.
                            let dedup_key = format!(
                                "{}|{}",
                                call.function.name, call.function.arguments
                            );
                            if !executed_tool_keys.insert(dedup_key.clone()) {
                                tracing::info!(
                                    tool = %call.function.name,
                                    round = round,
                                    "Streaming: skipping duplicate tool call in same round"
                                );
                                let skip_msg = format!(
                                    "⏭ Skipped duplicate call to `{}` — already executed in this round.",
                                    call.function.name
                                );
                                agent.history.push(ChatMessage::tool_result(
                                    &call.id,
                                    &skip_msg,
                                ));
                                let _ = tx
                                    .send(Ok(StreamEvent::ToolResult {
                                        tool_call_id: call.id.clone(),
                                        tool_name: call.function.name.clone(),
                                        content: skip_msg,
                                    }))
                                    .await;
                                // Check cancellation after skip too
                                check_stream_cancel!();
                                continue;
                            }

                            // ── Notify external listeners via the tool_notify_tx channel ──
                            if let Some(ref tx) = agent.tool_notify_tx {
                                let args_preview = if call.function.arguments.len() > 200 {
                                    let end = call.function.arguments.floor_char_boundary(200);
                                    format!("{}...", &call.function.arguments[..end])
                                } else {
                                    call.function.arguments.clone()
                                };
                                let _ = tx.send((call.function.name.clone(), args_preview));
                            }

                            // ── Notify the client immediately that a tool is about to execute ──
                            // This prevents the "no response" perception during long tool calls.
                            let args_preview = if call.function.arguments.len() > 200 {
                                let end = call.function.arguments.floor_char_boundary(200);
                                format!("{}...", &call.function.arguments[..end])
                            } else {
                                call.function.arguments.clone()
                            };
                            if tx
                                .send(Ok(StreamEvent::ToolExecuting {
                                    tool_call_id: call.id.clone(),
                                    tool_name: call.function.name.clone(),
                                    arguments_preview: args_preview,
                                }))
                                .await
                                .is_err()
                            {
                                return;
                            }

                            // Use tokio::select! so that /stop can cancel a
                            // long-running tool call mid-execution.
                            let result = if let Some(ref token) = agent.cancel_token {
                                tokio::select! {
                                    r = agent.tool_executor.execute_with_id(&call.id, &call.function) => r,
                                    _ = token.cancelled() => {
                                        tracing::info!(
                                            tool = %call.function.name,
                                            round = round,
                                            "Streaming tool execution cancelled via CancellationToken"
                                        );
                                        ToolResult {
                                            tool_call_id: call.id.clone(),
                                            content: "⏹ 任务已停止。".to_string(),
                                        }
                                    }
                                }
                            } else {
                                agent.tool_executor.execute_with_id(&call.id, &call.function).await
                            };

                            // Notify skills
                            for skill in &agent.skills {
                                if skill.is_active() {
                                    skill
                                        .on_tool_result(&call.function.name, &result.content)
                                        .await;
                                }
                            }

                            // Add tool result to history
                            agent.history.push(ChatMessage::tool_result(
                                &result.tool_call_id,
                                &result.content,
                            ));

                            // Emit tool result event to client
                            if tx
                                .send(Ok(StreamEvent::ToolResult {
                                    tool_call_id: result.tool_call_id,
                                    tool_name: call.function.name.clone(),
                                    content: result.content,
                                }))
                                .await
                                .is_err()
                            {
                                return;
                            }

                            // Check cancellation after each individual tool call —
                            // don't wait for the remaining tools in this round.
                            if let Some(ref token) = agent.cancel_token {
                                if token.is_cancelled() {
                                    tracing::info!(
                                        tool = %call.function.name,
                                        round = round,
                                        "Streaming agent loop cancelled after tool execution via CancellationToken"
                                    );
                                    let _ = tx.send(Ok(StreamEvent::Error {
                                        error: "任务已停止".to_string(),
                                    })).await;
                                    break;
                                }
                            }
                        }

                        round += 1;
                        if round >= max_rounds {
                            tracing::warn!(rounds = round, "Maximum tool rounds reached, stopping");
                            let warning = agent.config.max_rounds_reached_message();
                            agent.history.push(ChatMessage::assistant(&warning));
                            if tx
                                .send(Ok(StreamEvent::ContentDelta { delta: warning }))
                                .await
                                .is_err()
                            {
                                return;
                            }
                            break;
                        }

                        // Check cancellation after the round (belt-and-suspenders;
                        // the per-tool check above should already catch this).
                        if let Some(ref token) = agent.cancel_token {
                            if token.is_cancelled() {
                                tracing::info!(round = round, "Streaming agent loop cancelled between rounds via CancellationToken");
                                let _ = tx.send(Ok(StreamEvent::Error {
                                    error: "任务已停止".to_string(),
                                })).await;
                                break;
                            }
                        }

                        // Loop back for next round
                    } else {
                        // No tool calls — add the assistant message to history
                        agent.history.push(ChatMessage::assistant(&content_text));
                        break;
                    }
                }

                // Run skill post-processing
                agent.run_skills_on_response().await;

                // Emit final Done event
                let _ = tx.send(Ok(StreamEvent::Done { usage: None })).await;
            })
            .catch_unwind()
            .await;

            if let Err(panic_payload) = result {
                // The agent loop panicked — send an error event so the client knows
                let msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Agent stream panicked".to_string()
                };
                tracing::error!(error = %msg, "Agent stream panicked, sending error event");
                let _ = tx.send(Ok(StreamEvent::Error { error: msg })).await;
            }
        });

        // Convert the receiver into a stream
        tokio_stream::wrappers::ReceiverStream::new(rx).boxed()
    }
}

/// Helper struct to accumulate tool call data across streaming chunks.
struct AccumulatedToolCall {
    id: String,
    function_name: String,
    arguments: String,
}
