use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::agent::runner::{Agent, AgentConfig};
use crate::agent::skill::Skill;
use crate::provider::Provider;

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
        Self::new_with_skills(provider, cwd, Vec::new())
    }

    /// Create a new ACP session with skills applied.
    pub fn new_with_skills(
        provider: Box<dyn Provider>,
        cwd: String,
        skills: Vec<Arc<dyn Skill>>,
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
        agent.register_tool(Arc::new(crate::agent::tool_executor::EchoTool));
        agent.register_tool(Arc::new(crate::agent::tool_executor::CalculatorTool));
        agent.register_tool(Arc::new(crate::agent::tool_executor::DateTimeTool));

        Self {
            agent,
            cwd,
            current_mode: "ask".to_string(),
            cancelled: false,
        }
    }

    /// Extract text content from ACP prompt ContentBlock list.
    /// The prompt is a JSON array of ContentBlock objects.
    pub fn extract_text_from_prompt(prompt: &serde_json::Value) -> String {
        prompt
            .as_array()
            .map(|blocks| {
                blocks
                    .iter()
                    .filter_map(|b| {
                        if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                            b.get("text").and_then(|t| t.as_str()).map(String::from)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default()
    }

    /// Get the available modes for this agent as a JSON value.
    pub fn mode_state_json() -> serde_json::Value {
        serde_json::json!({
            "currentModeId": "ask",
            "availableModes": [
                {
                    "id": "ask",
                    "name": "Ask",
                    "description": "Request permission before making any changes"
                },
                {
                    "id": "code",
                    "name": "Code",
                    "description": "Write and modify code with full tool access"
                }
            ]
        })
    }
}

/// Manages all active ACP sessions.
pub struct SessionManager {
    sessions: RwLock<HashMap<String, AcpSession>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
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
        let session_id = uuid::Uuid::new_v4().to_string();
        let session = AcpSession::new_with_skills(provider, cwd, skills);
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
        let session = AcpSession::new_with_skills(provider, cwd, skills);
        self.sessions.write().await.insert(session_id, session);
        true
    }

    /// Close a session and free resources.
    pub async fn close_session(&self, session_id: &str) -> bool {
        self.sessions.write().await.remove(session_id).is_some()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
