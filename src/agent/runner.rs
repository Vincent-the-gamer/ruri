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
    /// Optional system prompt prepended to every conversation.
    pub system_prompt: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_tool_rounds: MAX_TOOL_ROUNDS,
            auto_execute_tools: true,
            system_prompt: None,
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

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
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
    /// Create a new Agent with the given provider and default configuration.
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self::with_config(provider, AgentConfig::default())
    }

    /// Create a new Agent with custom configuration.
    pub fn with_config(provider: Box<dyn Provider>, config: AgentConfig) -> Self {
        Self {
            transport: HttpTransport::with_default_config(provider),
            tool_executor: ToolExecutor::new(),
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

    /// Register a tool with the agent.
    pub fn register_tool(&mut self, tool: Arc<dyn crate::agent::tool_executor::Tool>) {
        self.tool_executor.register(tool);
    }

    /// Get all tool definitions for external use.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tool_executor.definitions()
    }

    /// Clear conversation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Get a reference to the conversation history.
    pub fn history(&self) -> &[ChatMessage] {
        &self.history
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
    pub async fn chat(&mut self, user_message: impl Into<String>) -> Result<ChatResponse, crate::provider::ProviderError> {
        let user_msg = ChatMessage::user(user_message);
        self.chat_with_message(user_msg).await
    }

    /// Run a single turn with a pre-constructed message.
    pub async fn chat_with_message(&mut self, message: ChatMessage) -> Result<ChatResponse, crate::provider::ProviderError> {
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
                                skill.on_tool_result(&call.function.name, &result.content).await;
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
                        tracing::warn!(
                            rounds = round,
                            "Maximum tool rounds reached, stopping"
                        );
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
    fn build_request(&self) -> ChatRequest {
        let mut request = ChatRequest::new(self.history.clone())
            .with_model(self.transport.default_model());

        // Add tools if any are registered
        let tool_defs = self.tool_executor.definitions();
        if !tool_defs.is_empty() {
            request = request.with_tools(tool_defs);
        }

        request
    }

    /// Initialize skills that haven't been attached yet.
    pub async fn initialize_skills(&mut self) {
        for skill in &self.skills {
            let system_messages = skill.on_attach().await;
            for msg in system_messages {
                // Insert system messages at the beginning of history
                self.history.insert(0, msg);
            }
        }
    }

    /// Run skill pre-processing on user messages.
    async fn run_skills_on_user_message(&mut self) {
        for skill in &self.skills {
            if skill.is_active() {
                skill.on_user_message(&mut self.history).await;
            }
        }
    }

    /// Run skill post-processing on the last response.
    async fn run_skills_on_response(&mut self) {
        if let Some(last) = self.history.last_mut() {
            if last.role == MessageRole::Assistant {
                for skill in &self.skills {
                    if skill.is_active() {
                        skill.on_response(last).await;
                    }
                }
            }
        }
    }
}
