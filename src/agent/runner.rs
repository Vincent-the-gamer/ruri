use crate::agent::skill::Skill;
use crate::agent::tool_executor::ToolExecutor;
use crate::provider::Provider;
use crate::transport::HttpTransport;
use crate::types::*;
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

            let response = self.transport.send(request).await?;

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
        let messages = if self.transport.supports_multimodal() {
            self.history.clone()
        } else {
            self.strip_multimodal_content()
        };

        let mut request = ChatRequest::new(messages).with_model(self.transport.default_model());

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

    /// Strip image content parts from all messages in history, logging a
    /// warning the first time images are dropped for a given session.
    ///
    /// Messages that only contained a single image (no text) are converted to
    /// a placeholder text message so that the conversation structure is
    /// preserved.
    fn strip_multimodal_content(&self) -> Vec<ChatMessage> {
        self.history
            .iter()
            .map(|msg| {
                let Some(ref content) = msg.content else {
                    return msg.clone();
                };

                match content {
                    MessageContent::Text(_) => msg.clone(),
                    MessageContent::Parts(parts) => {
                        let has_images = parts
                            .iter()
                            .any(|p| p.part_type == ContentPartType::ImageUrl || p.part_type == ContentPartType::Image);

                        if !has_images {
                            return msg.clone();
                        }

                        // Log a warning about dropped images
                        tracing::warn!(
                            role = ?msg.role,
                            "Dropping image content from message because the active provider does not support multimodal. \
                             Enable multimodal on the provider or start the backend with the --multimodal flag."
                        );

                        // Keep only text parts
                        let text_parts: Vec<&ContentPart> = parts
                            .iter()
                            .filter(|p| p.part_type == ContentPartType::Text)
                            .collect();

                        let new_content = if text_parts.is_empty() {
                            // No text parts remaining — use a placeholder
                            Some(MessageContent::Text(
                                "[Image content was removed: the active provider does not support multimodal]"
                                    .to_string(),
                            ))
                        } else if text_parts.len() == 1 {
                            // Single text part — simplify to plain text
                            Some(MessageContent::Text(
                                text_parts[0]
                                    .text
                                    .clone()
                                    .unwrap_or_default(),
                            ))
                        } else {
                            // Multiple text parts — keep them
                            Some(MessageContent::Parts(
                                text_parts
                                    .into_iter()
                                    .cloned()
                                    .collect(),
                            ))
                        };

                        ChatMessage {
                            content: new_content,
                            ..msg.clone()
                        }
                    }
                }
            })
            .collect()
    }

    /// Initialize skills that haven't been attached yet.
    ///
    /// Calls `on_attach()` on every skill and inserts the returned system
    /// messages **after** any existing system messages in the history, so
    /// that previously loaded system prompts keep their relative order.
    pub async fn initialize_skills(&mut self) {
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
