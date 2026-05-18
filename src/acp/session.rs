use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol::Client;
use agent_client_protocol::ConnectionTo;
use agent_client_protocol::schema::ContentBlock;
use tokio::sync::RwLock;

use crate::agent::runner::{Agent, AgentConfig};
use crate::agent::skill::Skill;
use crate::provider::Provider;
use crate::types::WebSearchConfig;

/// State tracked for each ACP session.
pub struct AcpSession {
    /// The ruri Agent instance driving this conversation.
    pub agent: Agent,
    /// Current mode ID.
    pub current_mode: String,
    /// Cancellation flag — set to `true` when the client sends a
    /// CancelNotification. This flag is checked at the start of
    /// `handle_session_prompt` only to detect stale cancel requests.
    /// It is always reset to `false` before a new prompt is accepted,
    /// ensuring the session remains usable for continued conversation.
    pub cancelled: bool,
}

impl AcpSession {
    /// Create a new ACP session with ACP tools applied.
    pub async fn new_with_skills_and_acp(
        provider: Box<dyn Provider>,
        _cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        available_skills: Vec<(String, String, Option<String>)>,
        session_id: String,
        web_search_config: Arc<RwLock<WebSearchConfig>>,
        computer_use_config: crate::computer_use::ComputerUseConfig,
        knowledge_base_service: Arc<RwLock<Option<crate::knowledge::KnowledgeBaseService>>>,
        active_knowledge_base_ids: Vec<String>,
        persona_prompt: Option<String>,
    ) -> Self {
        let config = AgentConfig::new()
            .with_max_tool_rounds(10)
            .with_auto_execute_tools(true);

        let mut agent = Agent::with_config(provider, config);

        // Add skills before built-in tools so they can be initialized
        for skill in skills {
            agent.add_skill(skill);
        }

        // Index available skills for routing (non-active skills are listed
        // in the system prompt so the model knows they exist and can route to them)
        for (name, description, when_to_use) in available_skills {
            agent.add_available_skill(name, description, when_to_use);
        }

        // Register tools based on computer_use_config runtime
        match computer_use_config.runtime {
            crate::computer_use::ComputerUseRuntime::None => {
                // Basic tools + BashTool (same as WebUI when computer use is disabled)
                let blacklist = computer_use_config.shell_command_blacklist.clone();
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::DeleteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::BashTool::new(
                    blacklist,
                )));
            }
            crate::computer_use::ComputerUseRuntime::Local => {
                // Create permission checker and workspace manager
                let data_dir = crate::computer_use::workspace::default_data_dir();
                let temp_dir = std::env::temp_dir();
                let permission_checker = Arc::new(crate::computer_use::PermissionChecker::new(
                    computer_use_config.clone(),
                    data_dir,
                    temp_dir,
                ));
                // Create a WorkspaceManager - use a shared one for ACP
                let workspace_manager = Arc::new(crate::computer_use::WorkspaceManager::new(
                    crate::computer_use::workspace::default_data_dir(),
                ));

                let tool_context = Arc::new(crate::computer_use::ComputerUseContext {
                    user_id: "acp_user".to_string(),
                    session_id: session_id.clone(),
                    permission_checker,
                    workspace_manager,
                    require_shell_confirmation: true, // ACP mode: user must click button to confirm shell execution
                });

                let can_use_power_tools = computer_use_config.can_use_power_tools("acp_user");

                // Register wrapped file tools with permission checking
                agent.register_tool(Arc::new(crate::computer_use::WrappedReadFileTool::new(
                    tool_context.clone(),
                )));
                agent.register_tool(Arc::new(crate::computer_use::WrappedWriteFileTool::new(
                    tool_context.clone(),
                )));
                agent.register_tool(Arc::new(
                    crate::computer_use::WrappedListDirectoryTool::new(tool_context.clone()),
                ));

                // Register Shell and Python tools only if user has permission
                if can_use_power_tools {
                    agent.register_tool(Arc::new(crate::computer_use::ShellTool::new(
                        tool_context.clone(),
                    )));
                    agent.register_tool(Arc::new(crate::computer_use::PythonTool::new(
                        tool_context.clone(),
                    )));
                }

                // Register other basic tools (not wrapped)
                agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::DeleteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
            }
            crate::computer_use::ComputerUseRuntime::AioSandbox => {
                // AIO Sandbox mode - use sandbox tools via HTTP API
                match &computer_use_config.aio_sandbox_config {
                    Some(config) => {
                        let client = Arc::new(crate::computer_use::AioSandboxClient::new(
                            config.endpoint.clone(),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxShellTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxReadFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxWriteFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxListDirectoryTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxCreateFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxEditFileTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxFindFilesTool::new(client.clone()),
                        ));
                        agent.register_tool(Arc::new(
                            crate::computer_use::AioSandboxSearchInFileTool::new(client),
                        ));
                    }
                    None => {
                        tracing::error!(
                            "AIO Sandbox runtime selected but no sandbox config provided, falling back to basic tools"
                        );
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::DeleteFileTool));
                        agent.register_tool(Arc::new(
                            crate::agent::builtin_tools::ListDirectoryTool,
                        ));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
                    }
                }
            }
        }

        // Register WebSearchTool only if properly configured
        let web_search_available = web_search_config
            .try_read()
            .map(|config| {
                config.enabled
                    && match config.search_engine {
                        crate::types::SearchEngine::DuckDuckGo => true,
                        _ => config.api_key.is_some(),
                    }
            })
            .unwrap_or(false);

        if web_search_available {
            agent.register_tool(Arc::new(crate::agent::builtin_tools::WebSearchTool::new(
                web_search_config,
            )));
        }

        // Add Knowledge Base skill and search tool if active_knowledge_base_ids is not empty
        if !active_knowledge_base_ids.is_empty() {
            // Use Hybrid mode for reliable knowledge base retrieval
            let kb_skill = crate::knowledge::KnowledgeBaseSkill::new(
                active_knowledge_base_ids.clone(),
                crate::knowledge::skill::KnowledgeBaseRetrievalMode::Hybrid,
                Arc::clone(&knowledge_base_service),
            );
            agent.add_skill(Arc::new(kb_skill));

            let kb_search_tool = crate::knowledge::KnowledgeBaseSearchTool::new(
                knowledge_base_service,
                active_knowledge_base_ids.clone(),
                crate::knowledge::skill::DEFAULT_KB_SEARCH_TOP_K,
            );
            agent.register_tool(Arc::new(kb_search_tool));

            tracing::info!(
                kb_count = active_knowledge_base_ids.len(),
                "KnowledgeBaseSkill and KnowledgeBaseSearchTool added to ACP agent"
            );
        }

        // Set persona as the system prompt if configured.
        // Persona is NOT added as a skill — it is set as the agent's sole
        // system message. Skills (including KB) dynamically inject their
        // context into user messages, never as system messages.
        if let Some(ref prompt) = persona_prompt {
            if !prompt.is_empty() {
                tracing::info!("Setting persona as system prompt for ACP agent");
                agent.set_system_prompt(prompt);
            }
        }

        agent.initialize_skills().await;

        Self {
            agent,
            current_mode: "ask".to_string(),
            cancelled: false,
        }
    }

    /// Extract text content from ACP prompt ContentBlock list.
    pub fn extract_text_from_prompt(prompt: &[ContentBlock]) -> String {
        prompt
            .iter()
            .filter_map(|block| {
                if let ContentBlock::Text(text_content) = block {
                    Some(text_content.text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Manages all active ACP sessions.
pub struct SessionManager {
    sessions: RwLock<HashMap<String, AcpSession>>,
    /// Maps session IDs to client connections for sending requests.
    connections: RwLock<HashMap<String, Arc<ConnectionTo<Client>>>>,
    /// Maps session IDs to cancellation tokens for currently-running prompts.
    /// Registered when a prompt starts, unregistered when it finishes.
    /// Used by `cancel_session` to stop the running agent loop even when
    /// the session has been taken out of the `sessions` map.
    running_prompt_tokens: RwLock<HashMap<String, tokio_util::sync::CancellationToken>>,
    /// Web search configuration shared across sessions.
    web_search_config: Arc<RwLock<WebSearchConfig>>,
    /// Computer use configuration shared across sessions.
    computer_use_config: crate::computer_use::ComputerUseConfig,
    /// Knowledge base service shared across sessions.
    knowledge_base_service: Arc<RwLock<Option<crate::knowledge::KnowledgeBaseService>>>,
    /// Active knowledge base IDs from the active config profile.
    active_knowledge_base_ids: Vec<String>,
}

impl SessionManager {
    /// Create a new SessionManager with the given configurations.
    pub fn new(
        web_search_config: Arc<RwLock<WebSearchConfig>>,
        computer_use_config: crate::computer_use::ComputerUseConfig,
        knowledge_base_service: Arc<RwLock<Option<crate::knowledge::KnowledgeBaseService>>>,
        active_knowledge_base_ids: Vec<String>,
    ) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            running_prompt_tokens: RwLock::new(HashMap::new()),
            web_search_config,
            computer_use_config,
            knowledge_base_service,
            active_knowledge_base_ids,
        }
    }

    /// Register a connection for a session.
    pub async fn register_connection(
        &self,
        session_id: String,
        connection: Arc<ConnectionTo<Client>>,
    ) {
        self.connections
            .write()
            .await
            .insert(session_id, connection);
    }

    /// Create a new session with skills and persona, returning its ID.
    pub async fn create_session_with_skills_and_persona(
        &self,
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        available_skills: Vec<(String, String, Option<String>)>,
        persona_prompt: Option<String>,
    ) -> String {
        self.create_session_with_skills_and_acp(
            provider,
            cwd,
            skills,
            None,
            available_skills,
            persona_prompt,
        )
        .await
    }

    /// Create a new session with skills and ACP tools, optionally with a session ID.
    pub async fn create_session_with_skills_and_acp(
        &self,
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        session_id: Option<String>,
        available_skills: Vec<(String, String, Option<String>)>,
        persona_prompt: Option<String>,
    ) -> String {
        let session_id_val = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let session = AcpSession::new_with_skills_and_acp(
            provider,
            cwd,
            skills,
            available_skills,
            session_id_val.clone(),
            Arc::clone(&self.web_search_config),
            self.computer_use_config.clone(),
            Arc::clone(&self.knowledge_base_service),
            self.active_knowledge_base_ids.clone(),
            persona_prompt,
        )
        .await;
        self.sessions
            .write()
            .await
            .insert(session_id_val.clone(), session);
        session_id_val
    }

    /// Take the session out for processing, then put it back.
    pub async fn take_session(&self, session_id: &str) -> Option<AcpSession> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id)
    }

    /// Put a session back after processing.
    pub async fn return_session(&self, session_id: String, session: AcpSession) {
        self.sessions.write().await.insert(session_id, session);
    }

    /// Register a cancellation token for a currently-running prompt.
    ///
    /// Called by `handle_session_prompt` before spawning the agent task.
    /// The token is unregistered when the prompt finishes.
    pub async fn register_prompt_token(
        &self,
        session_id: String,
        token: tokio_util::sync::CancellationToken,
    ) {
        self.running_prompt_tokens
            .write()
            .await
            .insert(session_id, token);
    }

    /// Unregister the cancellation token for a completed prompt.
    ///
    /// Called by `handle_session_prompt` after the agent task finishes.
    pub async fn unregister_prompt_token(&self, session_id: &str) {
        self.running_prompt_tokens.write().await.remove(session_id);
    }

    /// Cancel a session.
    ///
    /// This performs two actions:
    /// 1. Cancels the `CancellationToken` for any currently-running prompt,
    ///    which causes the agent loop to stop promptly (between rounds,
    ///    during tool execution, etc.).
    /// 2. Sets the `cancelled` flag on the session (if it is currently in
    ///    the sessions map) to prevent a queued prompt from starting.
    ///
    /// After cancellation, the `cancelled` flag is reset to `false` by
    /// `handle_session_prompt` when the next prompt is accepted, so the
    /// session remains usable for continued conversation.
    pub async fn cancel_session(&self, session_id: &str) {
        // 1. Cancel the running prompt's token — this stops the active agent loop
        let token_opt = self
            .running_prompt_tokens
            .read()
            .await
            .get(session_id)
            .cloned();
        if let Some(token) = token_opt {
            tracing::info!(
                session_id = %session_id,
                "Cancelling running prompt via CancellationToken"
            );
            token.cancel();
        } else {
            tracing::debug!(
                session_id = %session_id,
                "No running prompt token found for cancellation"
            );
        }

        // 2. Set cancelled flag on the session (if it's in the map)
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.cancelled = true;
            tracing::info!(
                session_id = %session_id,
                "Set cancelled flag on session in map"
            );
        } else {
            tracing::debug!(
                session_id = %session_id,
                "Session not in map (likely taken out for prompt processing), cancelled via token only"
            );
        }
    }

    /// Load an existing session with skills and persona applied.
    pub async fn load_session_with_skills_and_persona(
        &self,
        provider: Box<dyn Provider>,
        session_id: String,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        available_skills: Vec<(String, String, Option<String>)>,
        persona_prompt: Option<String>,
    ) -> bool {
        let session = AcpSession::new_with_skills_and_acp(
            provider,
            cwd,
            skills,
            available_skills,
            session_id.clone(),
            Arc::clone(&self.web_search_config),
            self.computer_use_config.clone(),
            Arc::clone(&self.knowledge_base_service),
            self.active_knowledge_base_ids.clone(),
            persona_prompt,
        )
        .await;
        self.sessions.write().await.insert(session_id, session);
        true
    }

    /// Close a session and free resources.
    pub async fn close_session(&self, session_id: &str) -> bool {
        // Also cancel any running prompt
        let token_opt = self
            .running_prompt_tokens
            .read()
            .await
            .get(session_id)
            .cloned();
        if let Some(token) = token_opt {
            token.cancel();
        }
        self.running_prompt_tokens.write().await.remove(session_id);
        self.connections.write().await.remove(session_id);
        self.sessions.write().await.remove(session_id).is_some()
    }

    /// Get a summary of a session's conversation history for forking.
    ///
    /// Returns `None` if the session does not exist, or `Some(summary)`
    /// containing a plain-text concatenation of the last few messages.
    pub async fn get_session_summary(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            Some(session.agent.get_conversation_summary())
        } else {
            None
        }
    }

    /// Create a forked session with an optional summary injected as initial history.
    ///
    /// This creates a brand-new session (new ID, fresh agent) that carries
    /// over the provider, skills, and persona from the source session. If a
    /// summary is provided it is injected as the first user/assistant pair so
    /// the forked agent has context from the previous conversation.
    pub async fn create_forked_session(
        &self,
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        available_skills: Vec<(String, String, Option<String>)>,
        persona_prompt: Option<String>,
        summary: Option<String>,
    ) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut session = AcpSession::new_with_skills_and_acp(
            provider,
            cwd,
            skills,
            available_skills,
            session_id.clone(),
            Arc::clone(&self.web_search_config),
            self.computer_use_config.clone(),
            Arc::clone(&self.knowledge_base_service),
            self.active_knowledge_base_ids.clone(),
            persona_prompt,
        )
        .await;

        // Inject the summary as initial conversation history if available
        if let Some(ref summary_text) = summary {
            if !summary_text.is_empty() {
                session.agent.inject_history_summary(summary_text.clone());
            }
        }

        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new(
            Arc::new(RwLock::new(WebSearchConfig::default())),
            crate::computer_use::ComputerUseConfig::default(),
            Arc::new(RwLock::new(None)),
            Vec::new(),
        )
    }
}
