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
    /// Custom error message to show users when a tool call or API request fails.
    /// If not set, the raw error message is returned.
    pub custom_error_message: Option<String>,
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
            custom_error_message: None,
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

    pub fn with_custom_error_message(mut self, message: Option<String>) -> Self {
        self.custom_error_message = message;
        self
    }

    /// Returns the message to display when the maximum tool rounds limit is reached.
    ///
    /// If a `custom_error_message` is configured, it is used; otherwise a
    /// descriptive default warning is returned.
    pub fn max_rounds_reached_message(&self) -> String {
        self.custom_error_message.clone().unwrap_or_else(|| {
            format!(
                "⚠️ Maximum tool call rounds ({}) reached, stopping.",
                self.max_tool_rounds
            )
        })
    }
}

/// The core Agent that orchestrates everything.
///
/// An Agent ties together:
/// - A **Provider** (via HTTP Transport) for communicating with AI models
/// - **Skills** that modify behavior at various lifecycle hooks
/// - **Tools** that the model can invoke and the agent can execute
pub struct Agent {
    transport: HttpTransport,
    tool_executor: ToolExecutor,
    skills: Vec<Arc<dyn Skill>>,
    config: AgentConfig,
    /// Conversation history maintained across turns.
    history: Vec<ChatMessage>,
    /// Whether `initialize_skills()` has been called at least once.
    /// Prevents duplicate system prompt injection if `initialize_skills()`
    /// is called explicitly *and* again via the auto-init guard.
    skills_initialized: bool,
}

impl Agent {
    /// Create a new Agent with custom configuration.
    pub fn with_config(provider: Box<dyn Provider>, config: AgentConfig) -> Self {
        let custom_error_message = config.custom_error_message.clone();
        let mut tool_executor = ToolExecutor::new();
        tool_executor.set_custom_error_message(custom_error_message);
        Self {
            transport: HttpTransport::with_default_config(provider),
            tool_executor,
            skills: Vec::new(),
            config,
            history: Vec::new(),
            skills_initialized: false,
        }
    }

    /// Add a skill to the agent.
    pub fn add_skill(&mut self, skill: Arc<dyn Skill>) {
        // Call on_attach and add system messages to history
        // Note: on_attach is async, so we'll handle it in run()
        self.skills.push(skill);
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
        self.history = history;
    }

    /// Run a single turn of the conversation.
    ///
    /// This method:
    /// 1. Attaches skills (on first call)
    /// 2. Pre-processes user messages through skills
    /// 3. Sends the request to the model
    /// 4. If the model requests tool calls, executes them and loops
    /// 5. Post-processes the response through skills
    /// 6. Returns the final response
    pub async fn chat(
        &mut self,
        user_message: impl Into<String>,
    ) -> Result<ChatResponse, crate::provider::ProviderError> {
        let user_msg = ChatMessage::user(user_message);
        self.chat_with_message(user_msg).await
    }

    /// Run a single turn with a pre-constructed message.
    pub async fn chat_with_message(
        &mut self,
        message: ChatMessage,
    ) -> Result<ChatResponse, crate::provider::ProviderError> {
        // Auto-initialize skills on first call if not done explicitly.
        // This ensures persona and skill system prompts are always injected,
        // even if the caller forgot to call `initialize_skills()`.
        if !self.skills_initialized {
            tracing::info!("Skills not yet initialized, auto-initializing before first chat");
            self.initialize_skills().await;
        }

        // Add user message to history
        self.history.push(message);

        // Run skill pre-processing
        self.run_skills_on_user_message().await;

        // Build request and run the tool loop
        let mut round = 0u32;
        let response = loop {
            let request = self.build_request();

            tracing::info!(
                round = round,
                provider = %self.transport.provider_name(),
                model = %self.transport.default_model(),
                messages = request.messages.len(),
                "Sending chat request"
            );

            let mut response = self.transport.send(request).await?;

            // Check if the model wants to call tools
            let choice = &response.choices[0];
            let tool_calls = choice.message.tool_calls.clone();

            match tool_calls {
                Some(calls) if self.config.auto_execute_tools && !calls.is_empty() => {
                    // Add assistant's tool-call message to history
                    let assistant_msg = choice.message.clone();
                    self.history.push(assistant_msg);

                    // Execute each tool call
                    for call in &calls {
                        tracing::info!(tool = %call.function.name, "Executing tool call");

                        let result = self
                            .tool_executor
                            .execute_with_id(&call.id, &call.function)
                            .await;

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
                        break response;
                    }
                    // Loop back to get the model's next response
                }
                _ => {
                    // No tool calls — add to history and return
                    self.history.push(choice.message.clone());
                    break response;
                }
            }
        };

        // Run skill post-processing on the final assistant message
        self.run_skills_on_response().await;

        Ok(response)
    }

    /// Build a ChatRequest from the current history and configuration.
    ///
    /// If the provider does not support multimodal content, image content
    /// parts are automatically stripped from messages and a warning is logged.
    fn build_request(&self) -> ChatRequest {
        let mut request =
            ChatRequest::new(self.history.clone()).with_model(self.transport.default_model());

        // If the provider does not support multimodal content, strip images upfront.
        if !self.transport.supports_multimodal() {
            request = request.strip_multimodal_content();
        }

        // Add tools if any are registered
        let tool_defs = self.tool_executor.definitions();
        if !tool_defs.is_empty() {
            request = request.with_tools(tool_defs);

            // Apply tool_choice and parallel_tool_calls only when tools are present
            //
            // Note: Per Function Calling spec, tool_choice should be removed when
            // the model is summarizing tool outputs. However, we apply it on every
            // request so that multi-round tool use cases (where the model needs to
            // call tools sequentially) work correctly. If you want the model to
            // produce a final summary instead of continuing to call tools, set
            // tool_choice to "auto" or "none" after the first tool round.
            if let Some(ref choice) = self.config.tool_choice {
                request = request.with_tool_choice(choice.clone());
            }
            if let Some(enabled) = self.config.parallel_tool_calls {
                request = request.with_parallel_tool_calls(enabled);
            }
        }

        request
    }

    /// Initialize skills that haven't been attached yet.
    ///
    /// Calls `on_attach()` on every skill and inserts the returned system
    /// messages **after** any existing system messages in the history, so
    /// that previously loaded system prompts keep their relative order.
    pub async fn initialize_skills(&mut self) {
        if self.skills_initialized {
            tracing::debug!(
                skills_count = self.skills.len(),
                "Skills already initialized, skipping duplicate initialization"
            );
            return;
        }

        for skill in &self.skills {
            let system_messages = skill.on_attach().await;
            tracing::info!(
                skill = %skill.name(),
                num_system_messages = system_messages.len(),
                "Initializing skill with on_attach"
            );
            if system_messages.is_empty() {
                continue;
            }
            // Find the index right after the last system message
            let insert_pos = self
                .history
                .iter()
                .take_while(|m| m.role == MessageRole::System)
                .count();
            for (i, msg) in system_messages.into_iter().enumerate() {
                self.history.insert(insert_pos + i, msg);
            }
        }

        self.skills_initialized = true;
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
    /// 1. Pre-process the user message through skills
    /// 2. Stream the model's response (content + tool calls)
    /// 3. If tool calls are present, execute them and emit results
    /// 4. Loop back for more model responses (up to `max_tool_rounds`)
    /// 5. Post-process through skills
    /// 6. Emit `Done`
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

                // Tool loop
                let mut round = 0u32;
                let max_rounds = agent.config.max_tool_rounds;

                loop {
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
                    while let Some(event_result) = stream.next().await {
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
                                        // Tool call completed, don't forward this to the client
                                        // We'll execute the tool and emit ToolResult instead
                                    }
                                    StreamEvent::ContentDelta { delta } => {
                                        content_text.push_str(delta);
                                    }
                                    StreamEvent::Done { .. } => {
                                        // End of this streaming round, don't forward
                                    }
                                    StreamEvent::ToolResult { .. } => {
                                        // Shouldn't happen from provider, but forward anyway
                                    }
                                    StreamEvent::Error { .. } => {
                                        // Forward errors
                                    }
                                }
                                // Forward all events to the client except ToolCallEnd and Done
                                // (we manage those ourselves after tool execution)
                                if !matches!(
                                    event,
                                    StreamEvent::ToolCallEnd { .. } | StreamEvent::Done { .. }
                                ) {
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
                                // (We re-enter the same event loop logic, but simpler
                                // since we've already stripped images and won't get
                                // MultimodalNotSupported again.)
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
                                                StreamEvent::ToolCallEnd { .. } | StreamEvent::Done { .. }
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

                        // Execute each tool call
                        for call in &tool_calls_for_history {
                            tracing::info!(tool = %call.function.name, "Executing tool call");

                            let result = agent
                                .tool_executor
                                .execute_with_id(&call.id, &call.function)
                                .await;

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
                        }

                        round += 1;
                        if round >= max_rounds {
                            tracing::warn!(rounds = round, "Maximum tool rounds reached, stopping");
                            // Emit a meaningful warning message as the last content
                            // so the client doesn't receive a blank response.
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
