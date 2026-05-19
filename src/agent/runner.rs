use crate::agent::skill::Skill;
use crate::agent::tool_executor::ToolExecutor;
use crate::provider::{Provider, ProviderError};
use crate::transport::HttpTransport;
use crate::types::*;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
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
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_tool_rounds: MAX_TOOL_ROUNDS,
            auto_execute_tools: true,
            tool_choice: None,
            parallel_tool_calls: None,
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
}

/// An entry in the skill index, used for skill routing.
#[derive(Debug, Clone)]
struct SkillIndexEntry {
    name: String,
    description: String,
    when_to_use: Option<String>,
}

impl Agent {
    /// Create a new Agent with custom configuration.
    pub fn with_config(provider: Box<dyn Provider>, config: AgentConfig) -> Self {
        let tool_executor = ToolExecutor::new();
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
            cancel_token: None,
            tool_permission_tx: None,
            tool_notify_tx: None,
        }
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
    pub fn add_available_skill(
        &mut self,
        name: String,
        description: String,
        when_to_use: Option<String>,
    ) {
        self.available_skill_index.push(SkillIndexEntry {
            name,
            description,
            when_to_use,
        });
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

        parts.push("## Available Skills".to_string());
        parts.push(
            "The following skills are available. When the user request matches \
             a skill, you should prioritize using that skill over other tools."
                .to_string(),
        );

        // List active skills (fully loaded with context)
        for skill in &self.skills {
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

        // List available skills (index-only, not fully loaded)
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

        // ── Tool calling priority & failure handling ─────────────────
        parts.push("## Tool Calling Priority (default order)".to_string());
        parts.push(
            "When processing a user request, follow this default priority order:\n\n\
            1. **Skill** — If any listed skill matches the user request (by name, description, or \"when_to_use\" condition), use it FIRST.\n\
            2. **web_search** — If no skill matches and the user needs external/real-time information, use web_search.\n\
            3. **bash / shell** — Use as a last resort for system-level operations or complex file manipulations.\n\
            4. **Other tools** (read_file, write_file, etc.) — Use as needed to support the above.\n\n\
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
                            }
                            StreamEvent::Done { .. } => {
                                // End of this streaming round
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
                                            if let Some(tc) = tool_calls_accum
                                                .iter_mut()
                                                .find(|tc| &tc.id == tool_call_id)
                                            {
                                                tc.arguments.push_str(arguments_delta);
                                            }
                                        }
                                        StreamEvent::ContentDelta { delta } => {
                                            content_text.push_str(delta);
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

                // Execute each tool call, skipping duplicates from the same round
                // to prevent redundant work when the LLM calls similar tools in parallel.
                let mut executed_tool_keys: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
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

                // Detect tool failures: if any tool returned TOOL_ERROR, break the loop
                // to prevent the model from auto-fallback to other tools.
                let tool_failed = tool_calls_for_history.iter().any(|call| {
                    self.history.iter().any(|msg| {
                        msg.role == MessageRole::Tool
                            && msg.tool_call_id.as_deref() == Some(&call.id)
                            && msg
                                .content
                                .as_ref()
                                .and_then(|c| c.as_text_full())
                                .is_some_and(|t| t.starts_with("TOOL_ERROR:"))
                    })
                });
                if tool_failed {
                    tracing::info!("Tool execution failed, breaking loop to prevent auto-fallback");
                    return Ok("tool_error".to_string());
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

                    // Execute each tool call, skipping duplicates from the same round
                    // to prevent redundant work when the LLM calls similar tools in parallel,
                    // and cancelling promptly if /stop is invoked.
                    let mut executed_tool_keys: std::collections::HashSet<String> =
                        std::collections::HashSet::new();
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

                    // Detect tool failures: if any tool returned TOOL_ERROR, break the loop
                    // to prevent the model from auto-fallback to other tools.
                    let tool_failed = calls.iter().any(|call| {
                        self.history.iter().any(|msg| {
                            msg.role == MessageRole::Tool
                                && msg.tool_call_id.as_deref() == Some(&call.id)
                                && msg
                                    .content
                                    .as_ref()
                                    .and_then(|c| c.as_text_full())
                                    .is_some_and(|t| t.starts_with("TOOL_ERROR:"))
                        })
                    });
                    if tool_failed {
                        tracing::info!(
                            "Tool execution failed, breaking loop to prevent auto-fallback"
                        );
                        let error_response = ChatResponse {
                            id: None,
                            object: Some("chat.completion".to_string()),
                            model: None,
                            choices: vec![crate::types::Choice {
                                index: 0,
                                message: ChatMessage::assistant(""),
                                finish_reason: Some("tool_error".to_string()),
                            }],
                            usage: None,
                            extra: serde_json::Map::new(),
                        };
                        break 'agent_loop error_response;
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

        // Inject the persona as the sole system message
        if let Some(ref prompt) = self.system_prompt {
            // Append skill routing instruction to the system prompt
            let full_prompt = if let Some(routing) = self.build_skill_routing_instruction() {
                format!("{}\n\n{}", prompt, routing)
            } else {
                prompt.clone()
            };
            messages.push(ChatMessage::system(&full_prompt));
        } else if let Some(routing) = self.build_skill_routing_instruction() {
            // No persona but we have a skill routing instruction — still inject it
            // as a system message so the model knows about available skills.
            messages.push(ChatMessage::system(&routing));
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
                                    }
                                    StreamEvent::Done { .. } => {
                                        // End of this streaming round, don't forward
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
                                                    if let Some(tc) = tool_calls_accum
                                                        .iter_mut()
                                                        .find(|tc| &tc.id == tool_call_id)
                                                    {
                                                        tc.arguments.push_str(arguments_delta);
                                                    }
                                                }
                                                StreamEvent::ContentDelta { delta } => {
                                                    content_text.push_str(delta);
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

                        // Execute each tool call, skipping duplicates from the same round
                        // to prevent redundant work when the LLM calls similar tools in parallel,
                        // and cancelling promptly if /stop is invoked.
                        let mut executed_tool_keys: std::collections::HashSet<String> =
                            std::collections::HashSet::new();
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

                        // Detect tool failures: if any tool returned TOOL_ERROR, break the loop
                        // to prevent the model from auto-fallback to other tools.
                        let tool_failed = tool_calls_for_history.iter().any(|call| {
                            agent.history.iter().any(|msg| {
                                msg.role == MessageRole::Tool
                                    && msg.tool_call_id.as_deref() == Some(&call.id)
                                    && msg.content.as_ref().and_then(|c| c.as_text_full())
                                        .is_some_and(|t| t.starts_with("TOOL_ERROR:"))
                            })
                        });
                        if tool_failed {
                            tracing::info!("Tool execution failed, breaking loop to prevent auto-fallback");
                            break;
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
