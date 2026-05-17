use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_client_protocol::schema::{
    AuthenticateRequest, AuthenticateResponse, CancelNotification, CloseSessionRequest,
    CloseSessionResponse, ContentBlock, ContentChunk, InitializeRequest, InitializeResponse,
    ListSessionsRequest, ListSessionsResponse, LoadSessionRequest, LoadSessionResponse,
    NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, ProtocolVersion,
    SessionId, SessionMode, SessionModeId, SessionModeState, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
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
/// Runs the ACP server with an optional config file path.
pub async fn run_acp_server_with_config_path(config_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config_path = config_path.unwrap_or_else(default_config_path);
    tracing::info!(
        "Starting ACP server on stdio, config path: {}",
        config_path.display()
    );

    let agent_state = Arc::new(RuriAgentState::new(&config_path).await);
    let session_manager = Arc::new(SessionManager::new(
        Arc::clone(&agent_state.web_search_config),
        agent_state.computer_use_config.clone(),
        Arc::clone(&agent_state.knowledge_base_service),
        agent_state.active_knowledge_base_ids.clone(),
    ));

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
    provider_factory: tokio::sync::RwLock<ProviderFactory>,
    /// Web search configuration shared across sessions.
    web_search_config: Arc<tokio::sync::RwLock<crate::types::WebSearchConfig>>,
    /// Computer use configuration shared across sessions.
    computer_use_config: crate::computer_use::ComputerUseConfig,
    /// Knowledge base service shared across sessions.
    knowledge_base_service:
        Arc<tokio::sync::RwLock<Option<crate::knowledge::KnowledgeBaseService>>>,
    /// Active knowledge base IDs from the active config profile.
    active_knowledge_base_ids: Vec<String>,
}

impl RuriAgentState {
    async fn new(config_path: &Path) -> Self {
        let provider_factory = ProviderFactory::from_config_path(config_path);
        let web_search_config = provider_factory.get_web_search_config();

        // Extract computer_use_config and active_knowledge_base_ids from persisted config
        // Knowledge base IDs now come from AcpConfig (ACP-specific configuration)
        let (computer_use_config, active_knowledge_base_ids) = provider_factory
            .config
            .as_ref()
            .map(|c| {
                let kb_ids = c.acp_config.active_knowledge_base_ids.clone();
                (c.computer_use_config.clone(), kb_ids)
            })
            .unwrap_or_default();

        // Initialize the database and KnowledgeBaseService
        let knowledge_base_service: Arc<
            tokio::sync::RwLock<Option<crate::knowledge::KnowledgeBaseService>>,
        > = {
            let db_path = crate::db::database_path();
            match crate::db::init(db_path).await {
                Ok(pool) => {
                    tracing::info!("ACP: Database initialized for knowledge base");
                    match crate::knowledge::KnowledgeBaseStore::new(pool).await {
                        Ok(kb_store) => {
                            let kb_service = crate::knowledge::KnowledgeBaseService::new(
                                std::sync::Arc::new(kb_store),
                            );
                            Arc::new(tokio::sync::RwLock::new(Some(kb_service)))
                        }
                        Err(e) => {
                            tracing::warn!("ACP: Failed to initialize KB store: {}", e);
                            Arc::new(tokio::sync::RwLock::new(None))
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("ACP: Failed to initialize database: {}", e);
                    Arc::new(tokio::sync::RwLock::new(None))
                }
            }
        };

        Self {
            provider_factory: tokio::sync::RwLock::new(provider_factory),
            web_search_config,
            computer_use_config,
            knowledge_base_service,
            active_knowledge_base_ids,
        }
    }
}

// ─── Protocol Handlers ────────────────────────────────────────────

async fn handle_initialize(
    _agent_state: Arc<RuriAgentState>,
    _request: InitializeRequest,
) -> Result<InitializeResponse, Error> {
    Ok(InitializeResponse::new(ProtocolVersion::from(1u16)))
}

async fn handle_session_new(
    agent_state: Arc<RuriAgentState>,
    session_manager: Arc<SessionManager>,
    request: NewSessionRequest,
    cx: ConnectionTo<Client>,
) -> Result<NewSessionResponse, Error> {
    let cwd = request.cwd.display().to_string();
    tracing::info!("Creating new ACP session, cwd={}", cwd);

    let mut pf = agent_state.provider_factory.write().await;
    let provider = pf
        .create_provider()
        .map_err(|e| Error::internal_error().data(e.to_string()))?;
    let (skills, persona_prompt) = pf.build_skills_and_persona();
    drop(pf);

    let session_id = session_manager
        .create_session_with_skills_and_persona(provider, cwd, skills, persona_prompt)
        .await;

    // Register connection for ACP file system operations
    session_manager
        .register_connection(session_id.clone(), Arc::new(cx.clone()))
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

    let mut pf = agent_state.provider_factory.write().await;
    let provider = pf
        .create_provider()
        .map_err(|e| Error::internal_error().data(e.to_string()))?;
    let (skills, persona_prompt) = pf.build_skills_and_persona();
    drop(pf);

    session_manager
        .load_session_with_skills_and_persona(
            provider,
            session_id.clone(),
            cwd,
            skills,
            persona_prompt,
        )
        .await;

    // Register connection for ACP file system operations
    session_manager
        .register_connection(session_id.clone(), Arc::new(cx.clone()))
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
                .and_then(|c| c.as_text_full())
                .unwrap_or_default();

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
    /// Path to the config file on disk.
    config_path: PathBuf,
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
        Self {
            config_path: config_path.to_path_buf(),
            config,
        }
    }

    /// Load the persisted config file.
    fn load_config(path: &Path) -> anyhow::Result<PersistedConfig> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config: {}", e))?;
        let config: PersistedConfig = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config: {}", e))?;
        Ok(config)
    }

    /// Re-read the config file from disk and update the cached config.
    /// On failure, keeps the previous config so existing sessions continue to work.
    fn reload_config(&mut self) {
        match Self::load_config(&self.config_path) {
            Ok(c) => {
                tracing::debug!("ACP reloaded config from {}", self.config_path.display());
                self.config = Some(c);
            }
            Err(e) => {
                tracing::warn!(
                    "ACP failed to reload config from {}: {}, keeping previous config",
                    self.config_path.display(),
                    e
                );
                // Keep the previous config on failure
            }
        }
    }

    /// Get the web search configuration from the persisted config.
    /// Returns a default configuration if no config is loaded.
    pub fn get_web_search_config(&self) -> Arc<tokio::sync::RwLock<crate::types::WebSearchConfig>> {
        let config = self
            .config
            .as_ref()
            .map(|c| c.web_search_config.clone())
            .unwrap_or_default();
        Arc::new(tokio::sync::RwLock::new(config))
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
    pub fn create_provider(&mut self) -> anyhow::Result<Box<dyn Provider>> {
        // Hot-reload config so new sessions pick up WebUI changes
        self.reload_config();

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

    /// Build skill instances and resolve persona based on the persisted ACP config.
    ///
    /// Returns a tuple of (skills, persona_prompt):
    /// - `skills`: only the skills listed in `acp_config.active_skill_names`
    /// - `persona_prompt`: the persona system prompt from the active config profile,
    ///   if configured. This should be set via `agent.set_system_prompt()`, NOT
    ///   added as a skill, to ensure it remains the sole system message.
    pub fn build_skills_and_persona(
        &mut self,
    ) -> (Vec<Arc<dyn crate::agent::skill::Skill>>, Option<String>) {
        // Hot-reload config so new sessions pick up WebUI changes
        self.reload_config();

        let Some(ref config) = self.config else {
            return (Vec::new(), None);
        };

        let mut skills: Vec<Arc<dyn crate::agent::skill::Skill>> = Vec::new();

        // Build regular skills from ACP config
        if !config.acp_config.active_skill_names.is_empty() {
            let stored_skills: HashMap<String, StoredSkill> = config
                .skills
                .iter()
                .map(|(name, s)| (name.clone(), Self::persisted_to_stored_skill(s)))
                .collect();

            skills =
                AppState::build_skills(&stored_skills, Some(&config.acp_config.active_skill_names));
        }

        // Resolve persona via persona_id from the active config profile + persona library
        let active_profile_persona_id = config
            .config_profiles
            .values()
            .filter(|p| p.is_active && p.enable)
            .find_map(|p| p.persona_id.clone());

        let persona_prompt = if let Some(pid) = active_profile_persona_id {
            if let Some(persona) = config.personas.get(&pid) {
                if !persona.prompt.is_empty() {
                    tracing::info!(
                        persona_id = %pid,
                        persona_name = %persona.name,
                        "ACP resolved persona system prompt from library"
                    );
                    Some(persona.prompt.clone())
                } else {
                    None
                }
            } else {
                tracing::warn!(
                    persona_id = %pid,
                    "ACP: persona_id not found in library, no persona will be applied"
                );
                None
            }
        } else {
            None
        };

        (skills, persona_prompt)
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
