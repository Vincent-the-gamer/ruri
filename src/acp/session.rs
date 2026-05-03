use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol::Client;
use agent_client_protocol::ConnectionTo;
use agent_client_protocol::schema::ContentBlock;
use tokio::sync::RwLock;

use crate::agent::acp_tools::RequestManager;
use crate::agent::runner::{Agent, AgentConfig};
use crate::agent::skill::Skill;
use crate::provider::Provider;
use crate::types::WebSearchConfig;

/// State tracked for each ACP session.
pub struct AcpSession {
    /// The ruri Agent instance driving this conversation.
    pub agent: Agent,
    /// The working directory for this session.
    pub cwd: String,
    /// Current mode ID.
    pub current_mode: String,
    /// Cancellation flag.
    pub cancelled: bool,
}

impl AcpSession {
    /// Create a new ACP session with only built-in tools (backward compatible).
    pub fn new(provider: Box<dyn Provider>, cwd: String) -> Self {
        Self::new_with_skills(
            provider,
            cwd,
            Vec::new(),
            Arc::new(RwLock::new(WebSearchConfig::default())),
        )
    }

    /// Create a new ACP session with skills applied.
    pub fn new_with_skills(
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        web_search_config: Arc<RwLock<WebSearchConfig>>,
    ) -> Self {
        let config = AgentConfig::new()
            .with_max_tool_rounds(10)
            .with_auto_execute_tools(true);

        let mut agent = Agent::with_config(provider, config);

        // Add skills before built-in tools so they can be initialized
        for skill in skills {
            agent.add_skill(skill);
        }

        // Register built-in tools
        agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));

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

        Self {
            agent,
            cwd,
            current_mode: "ask".to_string(),
            cancelled: false,
        }
    }

    /// Create a new ACP session with ACP tools applied.
    pub fn new_with_skills_and_acp(
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
        _session_id: String,
        _request_manager: Arc<RequestManager>,
        web_search_config: Arc<RwLock<WebSearchConfig>>,
    ) -> Self {
        let config = AgentConfig::new()
            .with_max_tool_rounds(10)
            .with_auto_execute_tools(true);

        let mut agent = Agent::with_config(provider, config);

        // Add skills before built-in tools so they can be initialized
        for skill in skills {
            agent.add_skill(skill);
        }

        // Register built-in tools for now
        // TODO: Replace with ACP tools once we have proper SessionManager access
        agent.register_tool(Arc::new(crate::agent::builtin_tools::ReadFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::WriteFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::CreateFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::EditFileTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::ListDirectoryTool));
        agent.register_tool(Arc::new(crate::agent::builtin_tools::SearchFilesTool));

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

        Self {
            agent,
            cwd,
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
    /// Manages pending ACP requests.
    request_manager: Arc<RequestManager>,
    /// Web search configuration shared across sessions.
    web_search_config: Arc<RwLock<WebSearchConfig>>,
}

impl SessionManager {
    /// Create a new SessionManager with the given web search configuration.
    pub fn new(web_search_config: Arc<RwLock<WebSearchConfig>>) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            request_manager: Arc::new(RequestManager::new()),
            web_search_config,
        }
    }

    /// Get the request manager for handling ACP responses.
    pub fn get_request_manager(&self) -> Arc<RequestManager> {
        Arc::clone(&self.request_manager)
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

    /// Get a connection for a session.
    pub async fn get_connection(&self, session_id: &str) -> Option<Arc<ConnectionTo<Client>>> {
        self.connections.read().await.get(session_id).cloned()
    }

    /// Remove a connection for a session.
    pub async fn remove_connection(&self, session_id: &str) {
        self.connections.write().await.remove(session_id);
    }

    /// Create a new session and return its ID (backward compatible, no skills).
    pub async fn create_session(&self, provider: Box<dyn Provider>, cwd: String) -> String {
        self.create_session_with_skills(provider, cwd, Vec::new())
            .await
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
        let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let session = AcpSession::new_with_skills_and_acp(
            provider,
            cwd,
            skills,
            session_id.clone(),
            Arc::clone(&self.request_manager),
            Arc::clone(&self.web_search_config),
        );
        self.sessions
            .write()
            .await
            .insert(session_id.clone(), session);
        session_id
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

    /// Check if a session is cancelled.
    pub async fn is_cancelled(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.cancelled)
            .unwrap_or(true)
    }

    /// Load an existing session (for session/load support), backward compatible.
    pub async fn load_session(
        &self,
        provider: Box<dyn Provider>,
        session_id: String,
        cwd: String,
    ) -> bool {
        self.load_session_with_skills(provider, session_id, cwd, Vec::new())
            .await
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
            Arc::clone(&self.request_manager),
            Arc::clone(&self.web_search_config),
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
        Self::new(Arc::new(RwLock::new(WebSearchConfig::default())))
    }
}
