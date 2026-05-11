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
    /// Cancellation flag.
    pub cancelled: bool,
}

impl AcpSession {
    /// Create a new ACP session with ACP tools applied.
    pub fn new_with_skills_and_acp(
        provider: Box<dyn Provider>,
        _cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        session_id: String,
        web_search_config: Arc<RwLock<WebSearchConfig>>,
        computer_use_config: crate::computer_use::ComputerUseConfig,
        knowledge_base_service: Arc<RwLock<Option<crate::knowledge::KnowledgeBaseService>>>,
        active_knowledge_base_ids: Vec<String>,
    ) -> Self {
        let config = AgentConfig::new()
            .with_max_tool_rounds(10)
            .with_auto_execute_tools(true);

        let mut agent = Agent::with_config(provider, config);

        // Add skills before built-in tools so they can be initialized
        for skill in skills {
            agent.add_skill(skill);
        }

        // Register tools based on computer_use_config runtime
        match computer_use_config.runtime {
            crate::computer_use::ComputerUseRuntime::None => {
                // Basic tools + BashTool (same as WebUI when computer use is disabled)
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
                agent.register_tool(Arc::new(crate::agent::builtin_tools::BashTool));
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
                agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
            }
            crate::computer_use::ComputerUseRuntime::AioSandbox => {
                // AIO Sandbox mode - use sandbox tools via HTTP API
                match &computer_use_config.aio_sandbox_config {
                    Some(config) => {
                        let client = Arc::new(crate::computer_use::AioSandboxClient::new(config.endpoint.clone()));
                        agent.register_tool(Arc::new(crate::computer_use::AioSandboxShellTool::new(client.clone())));
                        agent.register_tool(Arc::new(crate::computer_use::AioSandboxReadFileTool::new(client.clone())));
                        agent.register_tool(Arc::new(crate::computer_use::AioSandboxWriteFileTool::new(client.clone())));
                        agent.register_tool(Arc::new(crate::computer_use::AioSandboxListDirectoryTool::new(client)));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));
                    }
                    None => {
                        tracing::error!("AIO Sandbox runtime selected but no sandbox config provided, falling back to basic tools");
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
                        agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
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

        // Add Knowledge Base skill if active_knowledge_base_ids is not empty
        if !active_knowledge_base_ids.is_empty() {
            let kb_skill = crate::knowledge::KnowledgeBaseSkill::new(
                knowledge_base_service,
                active_knowledge_base_ids,
                5, // top_k
            );
            agent.add_skill(Arc::new(kb_skill));
            tracing::info!("KnowledgeBaseSkill added to ACP agent");
        }

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

    /// Create a new session with skills and return its ID.
    pub async fn create_session_with_skills(
        &self,
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
    ) -> String {
        self.create_session_with_skills_and_acp(provider, cwd, skills, None)
            .await
    }

    /// Create a new session with skills and ACP tools, optionally with a session ID.
    pub async fn create_session_with_skills_and_acp(
        &self,
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        session_id: Option<String>,
    ) -> String {
        let session_id_val = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let session = AcpSession::new_with_skills_and_acp(
            provider,
            cwd,
            skills,
            session_id_val.clone(),
            Arc::clone(&self.web_search_config),
            self.computer_use_config.clone(),
            Arc::clone(&self.knowledge_base_service),
            self.active_knowledge_base_ids.clone(),
        );
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

    /// Cancel a session.
    pub async fn cancel_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.cancelled = true;
        }
    }

    /// Load an existing session with skills applied.
    pub async fn load_session_with_skills(
        &self,
        provider: Box<dyn Provider>,
        session_id: String,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
    ) -> bool {
        let session = AcpSession::new_with_skills_and_acp(
            provider,
            cwd,
            skills,
            session_id.clone(),
            Arc::clone(&self.web_search_config),
            self.computer_use_config.clone(),
            Arc::clone(&self.knowledge_base_service),
            self.active_knowledge_base_ids.clone(),
        );
        self.sessions.write().await.insert(session_id, session);
        true
    }

    /// Close a session and free resources.
    pub async fn close_session(&self, session_id: &str) -> bool {
        self.connections.write().await.remove(session_id);
        self.sessions.write().await.remove(session_id).is_some()
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
