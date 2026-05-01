use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol::schema::{
    AgentCapabilities, AuthenticateRequest, AuthenticateResponse, CancelNotification,
    CloseSessionRequest, CloseSessionResponse, ContentBlock, ContentChunk, Implementation,
    InitializeRequest, InitializeResponse, ListSessionsRequest, ListSessionsResponse,
    LoadSessionRequest, LoadSessionResponse, McpCapabilities, NewSessionRequest,
    NewSessionResponse, PromptCapabilities, PromptRequest, PromptResponse, ProtocolVersion,
    SessionCapabilities, SessionCloseCapabilities, SessionId, SessionListCapabilities, SessionMode,
    SessionModeId, SessionModeState, SetSessionConfigOptionRequest, SetSessionConfigOptionResponse,
    SetSessionModeRequest, SetSessionModeResponse, StopReason,
};
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo, Error};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::acp::session::{AcpSession, SessionManager};
use crate::api::state::{
    AppState, PersistedConfig, PersistedProvider, PersistedSkill, StoredProvider, StoredSkill,
    default_config_path,
};
use crate::provider::Provider;

/// Runs the ACP server over stdio.
///
/// The agent communicates using the `agent_client_protocol` crate's
/// `Agent` builder pattern, which handles all JSON-RPC framing,
/// serialization, and deserialization automatically.
pub async fn run_acp_server() -> anyhow::Result<()> {
    run_acp_server_with_config_path(None).await
}

/// Runs the ACP server with an optional config file path.
pub async fn run_acp_server_with_config_path(config_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config_path = config_path.unwrap_or_else(default_config_path);
    tracing::info!(
        "Starting ACP server on stdio, config path: {}",
        config_path.display()
    );

    let agent_state = Arc::new(RuriAgentState::new(&config_path));
    let session_manager = Arc::new(SessionManager::new());

    let stdin = tokio::io::stdin().compat();
    let stdout = tokio::io::stdout().compat_write();

    Agent
        .builder()
        .name("ruri-acp")
        .on_receive_request(
            {
                let agent_state = agent_state.clone();
                async move |request: InitializeRequest, responder, _cx| {
                    let result = handle_initialize(agent_state.clone(), request).await;
                    responder.respond_with_result(result)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                async move |_request: AuthenticateRequest, responder, _cx| {
                    tracing::info!("ACP authenticate request received");
                    responder.respond(AuthenticateResponse::new())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let agent_state = agent_state.clone();
                let session_manager = session_manager.clone();
                async move |request: NewSessionRequest, responder, cx: ConnectionTo<Client>| {
                    let agent_state = agent_state.clone();
                    let session_manager = session_manager.clone();
                    let cx2 = cx.clone();
                    cx.spawn(async move {
                        let result =
                            handle_session_new(agent_state, session_manager, request, cx2).await;
                        responder.respond_with_result(result)
                    })?;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let agent_state = agent_state.clone();
                let session_manager = session_manager.clone();
                async move |request: LoadSessionRequest, responder, cx: ConnectionTo<Client>| {
                    let agent_state = agent_state.clone();
                    let session_manager = session_manager.clone();
                    let cx2 = cx.clone();
                    cx.spawn(async move {
                        let result =
                            handle_session_load(agent_state, session_manager, request, cx2).await;
                        responder.respond_with_result(result)
                    })?;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                async move |request: ListSessionsRequest, responder, _cx| {
                    let result = handle_session_list(request).await;
                    responder.respond_with_result(result)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let session_manager = session_manager.clone();
                async move |request: CloseSessionRequest, responder, _cx| {
                    let session_manager = session_manager.clone();
                    let result = handle_session_close(session_manager, request).await;
                    responder.respond_with_result(result)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let session_manager = session_manager.clone();
                async move |request: SetSessionModeRequest, responder, _cx| {
                    let session_manager = session_manager.clone();
                    let result = handle_session_set_mode(session_manager, request).await;
                    responder.respond_with_result(result)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                async move |request: SetSessionConfigOptionRequest, responder, _cx| {
                    let result = handle_session_set_config_option(request).await;
                    responder.respond_with_result(result)
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let session_manager = session_manager.clone();
                async move |request: PromptRequest, responder, cx: ConnectionTo<Client>| {
                    let session_manager = session_manager.clone();
                    let cx2 = cx.clone();
                    cx.spawn(async move {
                        let result = handle_session_prompt(session_manager, request, cx2).await;
                        responder.respond_with_result(result)
                    })?;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            {
                let session_manager = session_manager.clone();
                async move |notification: CancelNotification, _cx| {
                    let session_id_str = notification.session_id.0.as_ref();
                    tracing::info!("Cancelling session: {}", session_id_str);
                    session_manager.cancel_session(session_id_str).await;
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .connect_to(ByteStreams::new(stdout, stdin))
        .await
        .map_err(|e| anyhow::anyhow!("ACP error: {}", e))?;

    tracing::info!("ACP server connected successfully");

    Ok(())
}

// ─── Shared Agent State ────────────────────────────────────────────

/// State shared across all ACP request handlers.
struct RuriAgentState {
    provider_factory: ProviderFactory,
}

impl RuriAgentState {
    fn new(config_path: &Path) -> Self {
        Self {
            provider_factory: ProviderFactory::from_config_path(config_path),
        }
    }
}

// ─── Protocol Handlers ────────────────────────────────────────────

async fn handle_initialize(
    _agent_state: Arc<RuriAgentState>,
    request: InitializeRequest,
) -> Result<InitializeResponse, Error> {
    tracing::info!(
        "ACP initialize request received, protocol_version={:?}",
        request.protocol_version
    );

    let protocol_version = ProtocolVersion::V1;

    let agent_capabilities = AgentCapabilities::new()
        .prompt_capabilities(
            PromptCapabilities::new()
                .embedded_context(true)
                .image(false),
        )
        .mcp_capabilities(McpCapabilities::new().http(false))
        .load_session(true);

    let session_capabilities = SessionCapabilities::new()
        .close(SessionCloseCapabilities::new())
        .list(SessionListCapabilities::new());

    let agent_capabilities = agent_capabilities.session_capabilities(session_capabilities);

    Ok(InitializeResponse::new(protocol_version)
        .agent_capabilities(agent_capabilities)
        .agent_info(Implementation::new("ruri", env!("CARGO_PKG_VERSION")).title("Ruri AI Agent"))
        .auth_methods(vec![]))
}

async fn handle_session_new(
    agent_state: Arc<RuriAgentState>,
    session_manager: Arc<SessionManager>,
    request: NewSessionRequest,
    cx: ConnectionTo<Client>,
) -> Result<NewSessionResponse, Error> {
    let cwd = request.cwd.display().to_string();
    tracing::info!("Creating new ACP session, cwd={}", cwd);

    let provider = agent_state
        .provider_factory
        .create_provider()
        .map_err(|e| Error::internal_error().data(e.to_string()))?;
    let skills = agent_state.provider_factory.build_skills();

    let session_id = session_manager
        .create_session_with_skills(provider, cwd, skills)
        .await;

    let modes = build_mode_state();

    // Register a dynamic handler for session-update notifications from this session
    let session_id_clone = session_id.clone();
    let sm = session_manager.clone();
    let registration = cx
        .add_dynamic_handler(async_stream_handler(session_id_clone, sm))
        .map_err(|e| Error::internal_error().data(e.to_string()))?;

    registration.run_indefinitely();

    Ok(NewSessionResponse::new(SessionId::new(session_id)).modes(modes))
}

async fn handle_session_load(
    agent_state: Arc<RuriAgentState>,
    session_manager: Arc<SessionManager>,
    request: LoadSessionRequest,
    cx: ConnectionTo<Client>,
) -> Result<LoadSessionResponse, Error> {
    let session_id = request.session_id.0.as_ref().to_string();
    let cwd = request.cwd.display().to_string();
    tracing::info!("Loading session: {}", session_id);

    let provider = agent_state
        .provider_factory
        .create_provider()
        .map_err(|e| Error::internal_error().data(e.to_string()))?;
    let skills = agent_state.provider_factory.build_skills();

    session_manager
        .load_session_with_skills(provider, session_id.clone(), cwd, skills)
        .await;

    let modes = build_mode_state();

    // Register a dynamic handler for session-update notifications from this session
    let sm = session_manager.clone();
    let registration = cx
        .add_dynamic_handler(async_stream_handler(session_id.clone(), sm))
        .map_err(|e| Error::internal_error().data(e.to_string()))?;

    registration.run_indefinitely();

    Ok(LoadSessionResponse::new().modes(modes))
}

async fn handle_session_list(_request: ListSessionsRequest) -> Result<ListSessionsResponse, Error> {
    Ok(ListSessionsResponse::new(vec![]))
}

async fn handle_session_close(
    session_manager: Arc<SessionManager>,
    request: CloseSessionRequest,
) -> Result<CloseSessionResponse, Error> {
    let session_id = request.session_id.0.as_ref();
    tracing::info!("Closing session: {}", session_id);
    session_manager.close_session(session_id).await;
    Ok(CloseSessionResponse::new())
}

async fn handle_session_set_mode(
    session_manager: Arc<SessionManager>,
    request: SetSessionModeRequest,
) -> Result<SetSessionModeResponse, Error> {
    let session_id = request.session_id.0.as_ref();
    let mode_id = request.mode_id.0.as_ref();
    tracing::info!("Setting mode: session={}, mode={}", session_id, mode_id);

    if let Some(mut session) = session_manager.take_session(session_id).await {
        session.current_mode = mode_id.to_string();
        session_manager
            .return_session(session_id.to_string(), session)
            .await;
    }

    Ok(SetSessionModeResponse::default())
}

async fn handle_session_set_config_option(
    _request: SetSessionConfigOptionRequest,
) -> Result<SetSessionConfigOptionResponse, Error> {
    Ok(SetSessionConfigOptionResponse::new(vec![]))
}

async fn handle_session_prompt(
    session_manager: Arc<SessionManager>,
    request: PromptRequest,
    cx: ConnectionTo<Client>,
) -> Result<PromptResponse, Error> {
    let session_id_str = request.session_id.0.as_ref().to_string();
    let text = AcpSession::extract_text_from_prompt(&request.prompt);

    tracing::info!(
        "Session prompt: session_id={}, text_len={}",
        session_id_str,
        text.len()
    );

    // Take the session out for processing
    let mut session = session_manager
        .take_session(&session_id_str)
        .await
        .ok_or_else(|| Error::resource_not_found(None))?;

    if session.cancelled {
        session_manager
            .return_session(session_id_str.clone(), session)
            .await;
        return Ok(PromptResponse::new(StopReason::Cancelled));
    }

    // Process the prompt through the agent
    let result = session.agent.chat(&text).await;

    match result {
        Ok(response) => {
            let content = response
                .choices
                .first()
                .and_then(|c| c.message.content.as_ref())
                .and_then(|c| c.as_text())
                .unwrap_or("")
                .to_string();

            // Determine stop reason from the model's finish_reason
            let stop_reason = response
                .choices
                .first()
                .and_then(|c| c.finish_reason.as_deref())
                .map(|fr| match fr {
                    "stop" => StopReason::EndTurn,
                    "length" => StopReason::MaxTokens,
                    "content_filter" => StopReason::Refusal,
                    _ => StopReason::EndTurn,
                })
                .unwrap_or(StopReason::EndTurn);

            // Send the agent's response as a session/update notification
            use agent_client_protocol::schema::{
                AgentNotification, SessionNotification, SessionUpdate,
            };

            let text_content = agent_client_protocol::schema::TextContent::new(content);
            let content_block = ContentBlock::Text(text_content);
            let content_chunk = ContentChunk::new(content_block);
            let update = SessionUpdate::AgentMessageChunk(content_chunk);

            // Debug log the notification content
            tracing::debug!(
                "Sending SessionNotification: session_id={}, update_type=AgentMessageChunk",
                request.session_id.0.as_ref()
            );

            let notification = AgentNotification::SessionNotification(SessionNotification::new(
                request.session_id.clone(),
                update,
            ));

            cx.send_notification(notification)
                .map_err(|e| Error::internal_error().data(e.to_string()))?;

            // Debug log the response
            tracing::debug!("Sending PromptResponse: stop_reason={:?}", stop_reason);

            session_manager
                .return_session(session_id_str, session)
                .await;

            let response = PromptResponse::new(stop_reason);
            tracing::debug!("PromptResponse created successfully");

            // Log the serialized response for debugging
            match serde_json::to_string(&response) {
                Ok(json) => tracing::debug!("PromptResponse serialized: {}", json),
                Err(e) => tracing::error!("Failed to serialize PromptResponse: {}", e),
            }

            Ok(response)
        }
        Err(e) => {
            session_manager
                .return_session(session_id_str, session)
                .await;
            Err(Error::internal_error().data(format!("Agent error: {}", e)))
        }
    }
}

// ─── Helpers ────────────────────────────────────────────────────────

/// Build the session mode state for ruri.
fn build_mode_state() -> SessionModeState {
    SessionModeState::new(
        SessionModeId::from("ask"),
        vec![
            SessionMode::new("ask", "Ask")
                .description("Request permission before making any changes"),
            SessionMode::new("code", "Code")
                .description("Write and modify code with full tool access"),
        ],
    )
}

/// Creates a dynamic handler that processes streaming messages for a specific session.
fn async_stream_handler(
    _session_id: String,
    _session_manager: Arc<SessionManager>,
) -> impl agent_client_protocol::HandleDispatchFrom<Client> + 'static {
    use agent_client_protocol::{Dispatch, Handled};

    // For now, we process prompts synchronously (no streaming),
    // so we just let messages pass through unhandled.
    struct StreamHandler;

    #[allow(refining_impl_trait)]
    impl agent_client_protocol::HandleDispatchFrom<Client> for StreamHandler {
        fn describe_chain(&self) -> impl std::fmt::Debug {
            "StreamHandler"
        }

        fn handle_dispatch_from(
            &mut self,
            message: Dispatch,
            _connection: ConnectionTo<Client>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Handled<Dispatch>, Error>> + Send + '_>,
        > {
            Box::pin(std::future::ready(Ok(Handled::No {
                message,
                retry: false,
            })))
        }
    }

    StreamHandler
}

// ─── Provider Factory ──────────────────────────────────────────────

/// Creates providers and skills for ACP sessions, using persisted config
/// when available and falling back to environment variables.
pub struct ProviderFactory {
    /// The loaded persisted config (if available).
    config: Option<PersistedConfig>,
}

impl ProviderFactory {
    /// Create a ProviderFactory that reads from the given config file path.
    /// Falls back to environment variables if the config file doesn't exist
    /// or is invalid.
    pub fn from_config_path(config_path: &Path) -> Self {
        let config = match Self::load_config(config_path) {
            Ok(c) => {
                tracing::info!("ACP loaded config from {}", config_path.display());
                Some(c)
            }
            Err(e) => {
                tracing::info!(
                    "ACP could not load config from {}: {}, will use env vars",
                    config_path.display(),
                    e
                );
                None
            }
        };
        Self { config }
    }

    /// Load the persisted config file.
    fn load_config(path: &Path) -> anyhow::Result<PersistedConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;
        let config: PersistedConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
        Ok(config)
    }

    /// Resolve the active provider ID for ACP mode.
    /// Priority: ACP config provider → API mode active provider → None.
    fn resolve_acp_provider_id(&self) -> Option<String> {
        if let Some(ref config) = self.config {
            // ACP-specific provider takes priority
            if let Some(ref acp_provider_id) = config.acp_config.active_provider_id
                && config.providers.contains_key(acp_provider_id)
            {
                return Some(acp_provider_id.clone());
            }
            // Fall back to API mode active provider
            if let Some(ref api_active) = config.active_provider_id
                && config.providers.contains_key(api_active)
            {
                return Some(api_active.clone());
            }
        }
        None
    }

    /// Create a provider from the persisted config or environment variables.
    pub fn create_provider(&self) -> anyhow::Result<Box<dyn Provider>> {
        // Try to create from persisted config first
        if let Some(provider_id) = self.resolve_acp_provider_id()
            && let Some(ref config) = self.config
            && let Some(persisted) = config.providers.get(&provider_id)
        {
            let stored = Self::persisted_to_stored_provider(persisted);
            match AppState::build_provider(&stored) {
                Ok(provider) => {
                    tracing::info!(
                        "ACP using persisted provider: {} ({})",
                        persisted.name,
                        persisted.provider_type
                    );
                    return Ok(provider);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to build provider from config: {}, falling back to env vars",
                        e
                    );
                }
            }
        }

        // Fall back to environment variables
        self.create_provider_from_env()
    }

    /// Build skill instances based on the persisted ACP config.
    /// Only returns skills whose names are listed in `acp_config.active_skill_names`.
    pub fn build_skills(&self) -> Vec<Arc<dyn crate::agent::skill::Skill>> {
        let Some(ref config) = self.config else {
            return Vec::new();
        };

        if config.acp_config.active_skill_names.is_empty() {
            return Vec::new();
        }

        let stored_skills: HashMap<String, StoredSkill> = config
            .skills
            .iter()
            .map(|(name, s)| (name.clone(), Self::persisted_to_stored_skill(s)))
            .collect();

        AppState::build_skills(&stored_skills, Some(&config.acp_config.active_skill_names))
    }

    /// Convert a PersistedProvider to a StoredProvider.
    fn persisted_to_stored_provider(p: &PersistedProvider) -> StoredProvider {
        let created_at = chrono::DateTime::parse_from_rfc3339(&p.created_at)
            .map(|dt| dt.to_utc())
            .unwrap_or(chrono::Utc::now());
        StoredProvider {
            id: p.id.clone(),
            name: p.name.clone(),
            provider_type: p.provider_type.clone(),
            config_json: p.config_json.clone(),
            is_active: p.is_active,
            created_at,
        }
    }

    /// Convert a PersistedSkill to a StoredSkill.
    fn persisted_to_stored_skill(s: &PersistedSkill) -> StoredSkill {
        StoredSkill {
            name: s.name.clone(),
            description: s.description.clone(),
            skill_type: s.skill_type.clone(),
            config: s.config.clone(),
            is_active: s.is_active,
        }
    }

    /// Create a provider from environment variables (original behavior).
    fn create_provider_from_env(&self) -> anyhow::Result<Box<dyn Provider>> {
        let provider_type = if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            "anthropic"
        } else if std::env::var("OPENAI_API_KEY").is_ok() {
            "openai"
        } else if std::env::var("CUSTOM_API_URL").is_ok() {
            "custom"
        } else {
            "openai"
        };

        match provider_type {
            "anthropic" => {
                let api_key = std::env::var("ANTHROPIC_API_KEY")
                    .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;
                let model = std::env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
                Ok(Box::new(
                    crate::provider::anthropic::AnthropicProvider::new(api_key, model),
                ))
            }
            "custom" => {
                let url = std::env::var("CUSTOM_API_URL")
                    .map_err(|_| anyhow::anyhow!("CUSTOM_API_URL not set"))?;
                let api_key = std::env::var("CUSTOM_API_KEY").ok();
                let model = std::env::var("CUSTOM_MODEL").unwrap_or_else(|_| "default".to_string());
                let config = crate::provider::custom::CustomProviderConfig {
                    base_url: url,
                    chat_path: "/v1/chat/completions".to_string(),
                    method: "POST".to_string(),
                    auth_header: Some("Authorization".to_string()),
                    auth_prefix: "Bearer ".to_string(),
                    extra_headers: HashMap::new(),
                    request_template: None,
                    response_content_path: None,
                    response_tool_calls_path: None,
                    response_model_path: None,
                    response_finish_reason_path: None,
                    default_model: model,
                    use_openai_format: true,
                };
                Ok(Box::new(crate::provider::custom::CustomProvider::new(
                    config, api_key,
                )))
            }
            _ => {
                let base_url = std::env::var("OPENAI_BASE_URL")
                    .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
                let api_key = std::env::var("OPENAI_API_KEY").ok();
                let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());
                Ok(Box::new(crate::provider::openai::OpenAIProvider::new(
                    base_url, api_key, model,
                )))
            }
        }
    }
}

impl Default for ProviderFactory {
    fn default() -> Self {
        Self::from_config_path(&default_config_path())
    }
}

#[cfg(test)]
mod tests {
    use agent_client_protocol::schema::*;

    #[test]
    fn test_prompt_response_serialization() {
        let resp = PromptResponse::new(StopReason::EndTurn);
        let json = serde_json::to_string(&resp).unwrap();
        eprintln!("PromptResponse (EndTurn): {}", json);
        assert!(
            json.contains("\"stopReason\":\"end_turn\""),
            "Expected stopReason:end_turn, got: {}",
            json
        );

        let resp2 = PromptResponse::new(StopReason::Cancelled);
        let json2 = serde_json::to_string(&resp2).unwrap();
        eprintln!("PromptResponse (Cancelled): {}", json2);
        assert!(
            json2.contains("\"stopReason\":\"cancelled\""),
            "Expected stopReason:cancelled, got: {}",
            json2
        );
    }

    #[test]
    fn test_session_notification_serialization() {
        let notif = SessionNotification::new(
            SessionId::new("test-123"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Hello world"),
            ))),
        );
        let json = serde_json::to_string(&notif).unwrap();
        eprintln!("SessionNotification: {}", json);
        assert!(
            json.contains("\"sessionId\":\"test-123\""),
            "Expected sessionId, got: {}",
            json
        );
        assert!(
            json.contains("\"sessionUpdate\":\"agent_message_chunk\""),
            "Expected sessionUpdate, got: {}",
            json
        );
        assert!(
            json.contains("\"type\":\"text\""),
            "Expected type:text, got: {}",
            json
        );
    }

    #[test]
    fn test_initialize_response_serialization() {
        let resp = InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(AgentCapabilities::new())
            .agent_info(Implementation::new("ruri", "0.1.0"));
        let json = serde_json::to_string(&resp).unwrap();
        eprintln!("InitializeResponse: {}", json);
        assert!(
            json.contains("\"protocolVersion\":1"),
            "Expected protocolVersion:1, got: {}",
            json
        );
        assert!(
            json.contains("\"agentInfo\""),
            "Expected agentInfo, got: {}",
            json
        );
    }

    #[test]
    fn test_new_session_response_serialization() {
        let resp = NewSessionResponse::new(SessionId::new("sess_abc123"));
        let json = serde_json::to_string(&resp).unwrap();
        eprintln!("NewSessionResponse: {}", json);
        assert!(
            json.contains("\"sessionId\":\"sess_abc123\""),
            "Expected sessionId, got: {}",
            json
        );
    }

    #[test]
    fn test_agent_notification_serialization() {
        let notif = AgentNotification::SessionNotification(SessionNotification::new(
            SessionId::new("test-456"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Hello"),
            ))),
        ));
        let json = serde_json::to_string(&notif).unwrap();
        eprintln!("AgentNotification: {}", json);

        // Verify it can round-trip
        let parsed: AgentNotification = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, json2, "Round-trip serialization mismatch");
    }

    /// Test the exact same InitializeResponse that handle_initialize builds
    #[test]
    fn test_handle_initialize_response_format() {
        let protocol_version = ProtocolVersion::V1;

        let agent_capabilities = AgentCapabilities::new()
            .prompt_capabilities(
                PromptCapabilities::new()
                    .embedded_context(true)
                    .image(false),
            )
            .mcp_capabilities(McpCapabilities::new().http(false))
            .load_session(true);

        let session_capabilities = SessionCapabilities::new()
            .close(SessionCloseCapabilities::new())
            .list(SessionListCapabilities::new());

        let agent_capabilities = agent_capabilities.session_capabilities(session_capabilities);

        let resp = InitializeResponse::new(protocol_version)
            .agent_capabilities(agent_capabilities)
            .agent_info(Implementation::new("ruri", "0.1.0").title("Ruri AI Agent"))
            .auth_methods(vec![]);

        let json = serde_json::to_string(&resp).unwrap();
        eprintln!("handle_initialize response: {}", json);

        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        eprintln!("Pretty:\n{}", serde_json::to_string_pretty(&value).unwrap());

        // Verify the client can round-trip deserialize this
        // Simulate what a client without unstable features would see
        let _reparsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        eprintln!("Round-trip OK");
    }

    /// Test the full wire format of a session/prompt response
    #[test]
    fn test_prompt_response_wire_format() {
        // Simulate what the wire format looks like for a complete session/prompt exchange
        let prompt_resp = PromptResponse::new(StopReason::EndTurn);
        let resp_value = serde_json::to_value(&prompt_resp).unwrap();
        eprintln!(
            "PromptResponse value: {}",
            serde_json::to_string_pretty(&resp_value).unwrap()
        );

        // This is what a JSON-RPC 2.0 response would look like on the wire
        let wire = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": resp_value
        });
        eprintln!(
            "Wire format:\n{}",
            serde_json::to_string_pretty(&wire).unwrap()
        );

        // The result should have exactly stopReason
        assert_eq!(wire["result"]["stopReason"], "end_turn");
    }

    /// Test session/update notification wire format
    #[test]
    fn test_session_update_wire_format() {
        let notif = AgentNotification::SessionNotification(SessionNotification::new(
            SessionId::new("sess_abc123"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Hello from agent"),
            ))),
        ));
        let notif_value = serde_json::to_value(&notif).unwrap();
        eprintln!(
            "Notification params: {}",
            serde_json::to_string_pretty(&notif_value).unwrap()
        );

        // Wire format as JSON-RPC notification
        let wire = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": notif_value
        });
        eprintln!(
            "Wire format:\n{}",
            serde_json::to_string_pretty(&wire).unwrap()
        );

        // Verify key fields
        assert_eq!(wire["method"], "session/update");
        assert_eq!(wire["params"]["sessionId"], "sess_abc123");
        assert_eq!(
            wire["params"]["update"]["sessionUpdate"],
            "agent_message_chunk"
        );
        assert_eq!(wire["params"]["update"]["content"]["type"], "text");
    }

    // ─── Round-trip deserialization tests for all response types ───────────

    /// Helper: serialize a value, then deserialize it back and verify the
    /// round-tripped JSON is identical. Also returns the intermediate JSON
    /// for further assertions.
    fn assert_round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(
        value: &T,
        label: &str,
    ) -> serde_json::Value {
        let json = serde_json::to_string(value).unwrap();
        let parsed: T = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("{}: failed to deserialize round-trip: {}", label, e));
        let json2 = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, json2, "{}: round-trip serialization mismatch", label);
        serde_json::to_value(value).unwrap()
    }

    /// Test round-trip deserialization of InitializeResponse exactly as
    /// `handle_initialize` builds it.
    #[test]
    fn test_round_trip_initialize_response() {
        let protocol_version = ProtocolVersion::V1;

        let agent_capabilities = AgentCapabilities::new()
            .prompt_capabilities(
                PromptCapabilities::new()
                    .embedded_context(true)
                    .image(false),
            )
            .mcp_capabilities(McpCapabilities::new().http(false))
            .load_session(true)
            .session_capabilities(
                SessionCapabilities::new()
                    .close(SessionCloseCapabilities::new())
                    .list(SessionListCapabilities::new()),
            );

        let resp = InitializeResponse::new(protocol_version)
            .agent_capabilities(agent_capabilities)
            .agent_info(
                Implementation::new("ruri", env!("CARGO_PKG_VERSION")).title("Ruri AI Agent"),
            )
            .auth_methods(vec![]);

        let value = assert_round_trip(&resp, "InitializeResponse");

        // Core fields
        assert_eq!(value["protocolVersion"], 1);
        assert!(value["agentInfo"]["name"].is_string());
        assert_eq!(value["agentInfo"]["title"], "Ruri AI Agent");

        // Capabilities
        assert_eq!(value["agentCapabilities"]["loadSession"], true);
        assert_eq!(
            value["agentCapabilities"]["promptCapabilities"]["embeddedContext"],
            true
        );
        assert_eq!(
            value["agentCapabilities"]["promptCapabilities"]["image"],
            false
        );
        assert_eq!(value["agentCapabilities"]["mcpCapabilities"]["http"], false);
        assert!(value["agentCapabilities"]["sessionCapabilities"]["close"].is_object());
        assert!(value["agentCapabilities"]["sessionCapabilities"]["list"].is_object());

        // auth_methods is empty array -> serialized as []
        assert_eq!(value["authMethods"], serde_json::json!([]));
    }

    /// Test round-trip deserialization of AuthenticateResponse.
    #[test]
    fn test_round_trip_authenticate_response() {
        let resp = AuthenticateResponse::new();
        let value = assert_round_trip(&resp, "AuthenticateResponse");
        // Default AuthenticateResponse has no non-skipped fields, so it's `{}`
        assert_eq!(value, serde_json::json!({}));
    }

    /// Test round-trip deserialization of NewSessionResponse.
    #[test]
    fn test_round_trip_new_session_response() {
        let modes = SessionModeState::new(
            SessionModeId::from("ask"),
            vec![
                SessionMode::new("ask", "Ask")
                    .description("Request permission before making any changes"),
                SessionMode::new("code", "Code")
                    .description("Write and modify code with full tool access"),
            ],
        );

        let resp = NewSessionResponse::new(SessionId::new("sess_roundtrip")).modes(modes);
        let value = assert_round_trip(&resp, "NewSessionResponse");

        assert_eq!(value["sessionId"], "sess_roundtrip");
        assert_eq!(value["modes"]["currentModeId"], "ask");
        assert_eq!(
            value["modes"]["availableModes"].as_array().unwrap().len(),
            2
        );
    }

    /// Test round-trip deserialization of LoadSessionResponse.
    #[test]
    fn test_round_trip_load_session_response() {
        let modes = SessionModeState::new(
            SessionModeId::from("ask"),
            vec![
                SessionMode::new("ask", "Ask")
                    .description("Request permission before making any changes"),
                SessionMode::new("code", "Code")
                    .description("Write and modify code with full tool access"),
            ],
        );

        let resp = LoadSessionResponse::new().modes(modes);
        let value = assert_round_trip(&resp, "LoadSessionResponse");

        assert_eq!(value["modes"]["currentModeId"], "ask");
        assert_eq!(
            value["modes"]["availableModes"].as_array().unwrap().len(),
            2
        );
    }

    /// Test round-trip deserialization of ListSessionsResponse.
    #[test]
    fn test_round_trip_list_sessions_response() {
        let resp = ListSessionsResponse::new(vec![]);
        let value = assert_round_trip(&resp, "ListSessionsResponse");
        assert_eq!(value["sessions"], serde_json::json!([]));
    }

    /// Test round-trip deserialization of CloseSessionResponse.
    #[test]
    fn test_round_trip_close_session_response() {
        let resp = CloseSessionResponse::new();
        let value = assert_round_trip(&resp, "CloseSessionResponse");
        // Default CloseSessionResponse has no non-skipped fields
        assert_eq!(value, serde_json::json!({}));
    }

    /// Test round-trip deserialization of SetSessionModeResponse.
    #[test]
    fn test_round_trip_set_session_mode_response() {
        let resp = SetSessionModeResponse::new();
        let value = assert_round_trip(&resp, "SetSessionModeResponse");
        assert_eq!(value, serde_json::json!({}));
    }

    /// Test round-trip deserialization of SetSessionConfigOptionResponse.
    #[test]
    fn test_round_trip_set_session_config_option_response() {
        let resp = SetSessionConfigOptionResponse::new(vec![]);
        let value = assert_round_trip(&resp, "SetSessionConfigOptionResponse");
        assert_eq!(value["configOptions"], serde_json::json!([]));
    }

    /// Test round-trip deserialization of PromptResponse for every StopReason variant
    /// that our agent can produce.
    #[test]
    fn test_round_trip_prompt_response_all_stop_reasons() {
        let stop_reasons = vec![
            (StopReason::EndTurn, "end_turn"),
            (StopReason::MaxTokens, "max_tokens"),
            (StopReason::Refusal, "refusal"),
            (StopReason::Cancelled, "cancelled"),
        ];

        for (reason, expected_json_value) in stop_reasons {
            let resp = PromptResponse::new(reason);
            let value = assert_round_trip(&resp, &format!("PromptResponse({:?})", reason));
            assert_eq!(
                value["stopReason"], expected_json_value,
                "stopReason mismatch for {:?}",
                reason
            );
        }
    }

    // ─── InitializeResponse auth:{} feature gate test ────────────────────

    /// When compiled with the `unstable` feature (which includes `unstable_logout`),
    /// `AgentCapabilities` has an `auth` field of type `AgentAuthCapabilities`.
    /// The default `AgentAuthCapabilities { logout: None, meta: None }` serializes
    /// to `{}` because of `#[skip_serializing_none]`. This test verifies that:
    ///
    /// 1. The `auth` field is present in the serialized output as an empty object.
    /// 2. A client that doesn't know about the `auth` field can still parse the
    ///    JSON (simulated by deserializing into `serde_json::Value`).
    /// 3. The full `InitializeResponse` round-trips correctly.
    #[test]
    fn test_initialize_response_auth_field_unstable_logout() {
        let agent_capabilities = AgentCapabilities::new()
            .prompt_capabilities(
                PromptCapabilities::new()
                    .embedded_context(true)
                    .image(false),
            )
            .mcp_capabilities(McpCapabilities::new().http(false))
            .load_session(true)
            .session_capabilities(
                SessionCapabilities::new()
                    .close(SessionCloseCapabilities::new())
                    .list(SessionListCapabilities::new()),
            );

        // Verify the auth field is present and is an empty object {}
        let caps_value = serde_json::to_value(&agent_capabilities).unwrap();
        eprintln!(
            "AgentCapabilities (with unstable_logout): {}",
            serde_json::to_string_pretty(&caps_value).unwrap()
        );
        assert!(
            caps_value["auth"].is_object(),
            "Expected 'auth' field to be an object in AgentCapabilities, got: {:?}",
            caps_value.get("auth")
        );
        assert_eq!(
            caps_value["auth"],
            serde_json::json!({}),
            "Expected auth to be empty object {{}}, got: {:?}",
            caps_value.get("auth")
        );

        // Build the full InitializeResponse as handle_initialize does
        let resp = InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(agent_capabilities)
            .agent_info(
                Implementation::new("ruri", env!("CARGO_PKG_VERSION")).title("Ruri AI Agent"),
            )
            .auth_methods(vec![]);

        let json = serde_json::to_string(&resp).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        eprintln!(
            "InitializeResponse (with auth):\n{}",
            serde_json::to_string_pretty(&value).unwrap()
        );

        // The agentCapabilities.auth field must be present and be {}
        assert_eq!(
            value["agentCapabilities"]["auth"],
            serde_json::json!({}),
            "Expected agentCapabilities.auth to be {{}}, got: {:?}",
            value["agentCapabilities"].get("auth")
        );

        // Simulate a client without the `unstable_logout` feature:
        // It would deserialize the JSON into its own InitializeResponse type that
        // doesn't have the `auth` field. serde's default behavior is to ignore
        // unknown fields, so this should succeed at the Value level.
        let reparsed: serde_json::Value = serde_json::from_str(&json)
            .expect("Client without unstable_logout should be able to parse this JSON");

        // The auth field is still visible in the raw Value
        assert_eq!(
            reparsed["agentCapabilities"]["auth"],
            serde_json::json!({}),
            "auth field should still be visible in raw JSON"
        );

        // Full typed round-trip must also work
        let typed_reparsed: InitializeResponse =
            serde_json::from_str(&json).expect("Typed round-trip should succeed");
        let json_again = serde_json::to_string(&typed_reparsed).unwrap();
        assert_eq!(
            json, json_again,
            "InitializeResponse round-trip mismatch with auth field"
        );
    }

    /// Verify that when we explicitly set the `auth.logout` capability,
    /// it round-trips correctly through JSON-RPC.
    #[test]
    fn test_initialize_response_auth_with_logout_capability() {
        let agent_capabilities = AgentCapabilities::new()
            .prompt_capabilities(
                PromptCapabilities::new()
                    .embedded_context(true)
                    .image(false),
            )
            .mcp_capabilities(McpCapabilities::new().http(false))
            .load_session(true)
            .session_capabilities(
                SessionCapabilities::new()
                    .close(SessionCloseCapabilities::new())
                    .list(SessionListCapabilities::new()),
            )
            .auth(AgentAuthCapabilities::new().logout(LogoutCapabilities::new()));

        let resp = InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(agent_capabilities)
            .agent_info(Implementation::new("ruri", "0.1.0"))
            .auth_methods(vec![]);

        let value = assert_round_trip(&resp, "InitializeResponse (auth with logout)");
        eprintln!(
            "InitializeResponse (auth.logout):\n{}",
            serde_json::to_string_pretty(&value).unwrap()
        );

        // The logout field should be present as an empty object
        assert_eq!(
            value["agentCapabilities"]["auth"]["logout"],
            serde_json::json!({}),
            "Expected auth.logout to be {{}}"
        );
    }

    // ─── SessionNotification / AgentNotification round-trip tests ───────

    /// Test round-trip of SessionNotification with AgentMessageChunk.
    #[test]
    fn test_round_trip_session_notification_agent_message_chunk() {
        let notif = SessionNotification::new(
            SessionId::new("sess_notif"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Agent response text"),
            ))),
        );
        let value = assert_round_trip(&notif, "SessionNotification::AgentMessageChunk");
        assert_eq!(value["sessionId"], "sess_notif");
        assert_eq!(value["update"]["sessionUpdate"], "agent_message_chunk");
        assert_eq!(value["update"]["content"]["type"], "text");
        assert_eq!(value["update"]["content"]["text"], "Agent response text");
    }

    /// Test round-trip of AgentNotification wrapping a SessionNotification.
    #[test]
    fn test_round_trip_agent_notification_session_notification() {
        let notif = AgentNotification::SessionNotification(SessionNotification::new(
            SessionId::new("sess_wrap"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Wrapped notification"),
            ))),
        ));
        let value = assert_round_trip(&notif, "AgentNotification::SessionNotification");
        assert_eq!(value["sessionId"], "sess_wrap");
        assert_eq!(value["update"]["sessionUpdate"], "agent_message_chunk");
    }

    /// Test round-trip of other SessionUpdate variants we might produce.
    #[test]
    fn test_round_trip_session_update_current_mode_update() {
        let notif = SessionNotification::new(
            SessionId::new("sess_mode"),
            SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(SessionModeId::from("code"))),
        );
        let value = assert_round_trip(&notif, "SessionNotification::CurrentModeUpdate");
        assert_eq!(value["update"]["sessionUpdate"], "current_mode_update");
        assert_eq!(value["update"]["currentModeId"], "code");
    }

    /// Test round-trip of AgentNotification wrapped in a JSON-RPC 2.0
    /// notification envelope, then deserialized from the wire.
    #[test]
    fn test_round_trip_agent_notification_from_wire() {
        let notif = AgentNotification::SessionNotification(SessionNotification::new(
            SessionId::new("sess_wire"),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new("Wire format test"),
            ))),
        ));

        // Build the JSON-RPC 2.0 notification envelope
        let wire = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": serde_json::to_value(&notif).unwrap()
        });

        let wire_json = serde_json::to_string(&wire).unwrap();
        eprintln!(
            "AgentNotification wire format:\n{}",
            serde_json::to_string_pretty(&wire).unwrap()
        );

        // Parse the envelope
        let parsed_wire: serde_json::Value = serde_json::from_str(&wire_json).unwrap();
        assert_eq!(parsed_wire["jsonrpc"], "2.0");
        assert_eq!(parsed_wire["method"], "session/update");

        // Extract the params and deserialize back into AgentNotification
        let params = &parsed_wire["params"];
        let reparsed: AgentNotification = serde_json::from_value(params.clone())
            .expect("Should be able to deserialize AgentNotification from wire params");

        let json_again = serde_json::to_string(&reparsed).unwrap();
        let original_json = serde_json::to_string(&notif).unwrap();
        assert_eq!(
            json_again, original_json,
            "AgentNotification round-trip from wire mismatch"
        );
    }

    // ─── Full wire format tests for each response type ──────────────────

    /// Helper: build a JSON-RPC 2.0 response envelope and verify the
    /// `result` can be deserialized back into the given type.
    fn assert_wire_response_round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(
        result: &T,
        id: i64,
        label: &str,
    ) -> serde_json::Value {
        let result_value = serde_json::to_value(result).unwrap();
        let wire = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result_value
        });

        let wire_json = serde_json::to_string(&wire).unwrap();
        eprintln!(
            "{} wire format:\n{}",
            label,
            serde_json::to_string_pretty(&wire).unwrap()
        );

        // Parse the wire JSON
        let parsed_wire: serde_json::Value = serde_json::from_str(&wire_json).unwrap();
        assert_eq!(parsed_wire["jsonrpc"], "2.0");
        assert_eq!(parsed_wire["id"], id);

        // Extract the result and deserialize back into the typed struct
        let reparsed: T = serde_json::from_value(parsed_wire["result"].clone())
            .unwrap_or_else(|e| panic!("{}: failed to deserialize result from wire: {}", label, e));

        // Verify round-trip
        let result_json = serde_json::to_string(&result).unwrap();
        let reparsed_json = serde_json::to_string(&reparsed).unwrap();
        assert_eq!(
            result_json, reparsed_json,
            "{}: wire round-trip mismatch",
            label
        );

        wire
    }

    #[test]
    fn test_wire_format_initialize_response() {
        let agent_capabilities = AgentCapabilities::new()
            .prompt_capabilities(
                PromptCapabilities::new()
                    .embedded_context(true)
                    .image(false),
            )
            .mcp_capabilities(McpCapabilities::new().http(false))
            .load_session(true)
            .session_capabilities(
                SessionCapabilities::new()
                    .close(SessionCloseCapabilities::new())
                    .list(SessionListCapabilities::new()),
            );

        let resp = InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(agent_capabilities)
            .agent_info(
                Implementation::new("ruri", env!("CARGO_PKG_VERSION")).title("Ruri AI Agent"),
            )
            .auth_methods(vec![]);

        let wire = assert_wire_response_round_trip(&resp, 1, "InitializeResponse");
        assert_eq!(wire["result"]["protocolVersion"], 1);
        assert_eq!(wire["result"]["agentCapabilities"]["loadSession"], true);
    }

    #[test]
    fn test_wire_format_authenticate_response() {
        let resp = AuthenticateResponse::new();
        let wire = assert_wire_response_round_trip(&resp, 2, "AuthenticateResponse");
        assert_eq!(wire["result"], serde_json::json!({}));
    }

    #[test]
    fn test_wire_format_new_session_response() {
        let modes = SessionModeState::new(
            SessionModeId::from("ask"),
            vec![
                SessionMode::new("ask", "Ask")
                    .description("Request permission before making any changes"),
                SessionMode::new("code", "Code")
                    .description("Write and modify code with full tool access"),
            ],
        );
        let resp = NewSessionResponse::new(SessionId::new("sess_wire_new")).modes(modes);
        let wire = assert_wire_response_round_trip(&resp, 3, "NewSessionResponse");
        assert_eq!(wire["result"]["sessionId"], "sess_wire_new");
    }

    #[test]
    fn test_wire_format_load_session_response() {
        let modes = SessionModeState::new(
            SessionModeId::from("ask"),
            vec![
                SessionMode::new("ask", "Ask")
                    .description("Request permission before making any changes"),
                SessionMode::new("code", "Code")
                    .description("Write and modify code with full tool access"),
            ],
        );
        let resp = LoadSessionResponse::new().modes(modes);
        let wire = assert_wire_response_round_trip(&resp, 4, "LoadSessionResponse");
        assert_eq!(wire["result"]["modes"]["currentModeId"], "ask");
    }

    #[test]
    fn test_wire_format_list_sessions_response() {
        let resp = ListSessionsResponse::new(vec![]);
        let wire = assert_wire_response_round_trip(&resp, 5, "ListSessionsResponse");
        assert_eq!(wire["result"]["sessions"], serde_json::json!([]));
    }

    #[test]
    fn test_wire_format_close_session_response() {
        let resp = CloseSessionResponse::new();
        let wire = assert_wire_response_round_trip(&resp, 6, "CloseSessionResponse");
        assert_eq!(wire["result"], serde_json::json!({}));
    }

    #[test]
    fn test_wire_format_set_session_mode_response() {
        let resp = SetSessionModeResponse::new();
        let wire = assert_wire_response_round_trip(&resp, 7, "SetSessionModeResponse");
        assert_eq!(wire["result"], serde_json::json!({}));
    }

    #[test]
    fn test_wire_format_set_session_config_option_response() {
        let resp = SetSessionConfigOptionResponse::new(vec![]);
        let wire = assert_wire_response_round_trip(&resp, 8, "SetSessionConfigOptionResponse");
        assert_eq!(wire["result"]["configOptions"], serde_json::json!([]));
    }

    #[test]
    fn test_wire_format_prompt_response() {
        let resp = PromptResponse::new(StopReason::EndTurn);
        let wire = assert_wire_response_round_trip(&resp, 9, "PromptResponse");
        assert_eq!(wire["result"]["stopReason"], "end_turn");
    }

    /// Test that a JSON-RPC error response on the wire is also parseable.
    #[test]
    fn test_wire_format_error_response() {
        let error =
            agent_client_protocol::Error::internal_error().data("Agent error: something broke");
        let error_value = serde_json::to_value(&error).unwrap();
        let wire = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "error": error_value
        });
        eprintln!(
            "Error response wire format:\n{}",
            serde_json::to_string_pretty(&wire).unwrap()
        );
        assert_eq!(wire["jsonrpc"], "2.0");
        assert!(wire["error"].is_object());
        assert!(wire["error"]["code"].is_number());
        assert!(wire["error"]["message"].is_string());
    }

    // ─── Cross-feature compatibility: client without unstable features ────

    /// Simulate how a client that does NOT have the `unstable_logout` feature
    /// would parse our `InitializeResponse`. Since serde's default is to
    /// ignore unknown fields, the `auth:{}` field in the JSON should not
    /// cause any issue.
    #[test]
    fn test_initialize_response_client_without_unstable_logout() {
        let agent_capabilities = AgentCapabilities::new()
            .prompt_capabilities(
                PromptCapabilities::new()
                    .embedded_context(true)
                    .image(false),
            )
            .mcp_capabilities(McpCapabilities::new().http(false))
            .load_session(true)
            .session_capabilities(
                SessionCapabilities::new()
                    .close(SessionCloseCapabilities::new())
                    .list(SessionListCapabilities::new()),
            );

        let resp = InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(agent_capabilities)
            .agent_info(Implementation::new("ruri", "0.1.0").title("Ruri AI Agent"))
            .auth_methods(vec![]);

        let json = serde_json::to_string(&resp).unwrap();

        // Simulate a client that parses into a generic JSON value.
        // If the client's Rust struct doesn't have the `auth` field,
        // serde will simply ignore it (no deny_unknown_fields is used).
        let client_value: serde_json::Value = serde_json::from_str(&json)
            .expect("Client without unstable_logout should parse this fine");

        // The json still contains the auth field, which the client can
        // safely ignore
        assert!(
            client_value["agentCapabilities"].get("auth").is_some(),
            "auth field should be present in the raw JSON"
        );
        assert_eq!(
            client_value["agentCapabilities"]["auth"],
            serde_json::json!({}),
            "auth should be an empty object"
        );

        // Verify the client can still read the standard fields
        assert_eq!(client_value["protocolVersion"], 1);
        assert_eq!(client_value["agentInfo"]["name"], "ruri");
        assert_eq!(client_value["agentInfo"]["title"], "Ruri AI Agent");
        assert_eq!(client_value["agentCapabilities"]["loadSession"], true);
    }
}
