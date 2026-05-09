//! Built-in command system for Ruri.
//!
//! This module provides a command dispatching mechanism similar to AstrBot's
//! built-in commands. Commands are prefixed with `/` (e.g. `/help`, `/reset`)
//! and are intercepted before the message is sent to the AI agent.
//!
//! All command operations are logged via `tracing`, which means they are
//! automatically synced to the frontend through the LogManager + WebSocket
//! broadcast pipeline.
//!
//! # Supported commands
//!
//! - `/help` — Show available commands and version info
//! - `/sid`  — Show current session info (UMO, UID, Bot ID, etc.)
//! - `/reset` — Reset the current conversation's LLM context
//! - `/stop` — Stop the currently running agent task in the current session
//! - `/new` — Create and switch to a new conversation
//! - `/set <key> <value>` — Set a session variable
//! - `/unset <key>` — Remove a session variable
//! - `/dashboard_update` — Update the WebUI (requires admin)

pub mod builtin;

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// The default command prefix.
const DEFAULT_PREFIX: &str = "/";

/// Context provided to each command handler.
pub struct CommandContext {
    /// The raw message text (including the command prefix).
    pub raw_message: String,
    /// Parsed command name (e.g. "help" from "/help").
    /// Populated by the dispatcher during parsing.
    pub command_name: String,
    /// Arguments after the command name (e.g. "key value" from "/set key value").
    pub args: String,
    /// Session ID of the current conversation.
    pub session_id: String,
    /// User ID of the message sender.
    pub user_id: String,
    /// Platform adapter instance ID (e.g. "dingtalk", "discord").
    pub platform_id: String,
    /// The bot's own ID on the platform.
    pub self_id: String,
    /// Message type (group or private).
    pub message_type: crate::platform::types::MessageType,
    /// Group ID (empty string for private messages).
    pub group_id: String,
    /// Shared application state.
    pub state: Arc<crate::api::AppState>,
}

impl CommandContext {
    /// Return a human-readable label for the message type.
    pub fn message_type_label(&self) -> &'static str {
        match self.message_type {
            crate::platform::types::MessageType::GroupMessage => "群聊",
            crate::platform::types::MessageType::FriendMessage => "私聊",
        }
    }
}

/// Result returned by a command handler.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// The text reply to send back to the user.
    pub reply: String,
}

impl CommandResult {
    /// Create a simple text reply.
    pub fn text(reply: impl Into<String>) -> Self {
        Self {
            reply: reply.into(),
        }
    }
}

/// A trait for implementing a command.
#[async_trait]
pub trait Command: Send + Sync {
    /// The command name (without the prefix), e.g. "help".
    fn name(&self) -> &str;

    /// A short description of what the command does.
    fn description(&self) -> &str;

    /// Usage hint, e.g. "/set <key> <value>".
    fn usage(&self) -> &str {
        ""
    }

    /// Whether this command requires admin privileges.
    fn require_admin(&self) -> bool {
        false
    }

    /// Whether this command should be hidden from `/help` output.
    fn hidden(&self) -> bool {
        false
    }

    /// Execute the command.
    async fn execute(&self, ctx: &CommandContext) -> CommandResult;
}

/// The command dispatcher that registers and routes commands.
pub struct CommandDispatcher {
    commands: HashMap<String, Arc<dyn Command>>,
    prefix: String,
}

impl CommandDispatcher {
    /// Create a new dispatcher with the default `/` prefix.
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            prefix: DEFAULT_PREFIX.to_string(),
        }
    }

    /// Register a command.
    pub fn register(&mut self, command: Arc<dyn Command>) {
        tracing::info!(
            command = %command.name(),
            "Registered built-in command"
        );
        self.commands.insert(command.name().to_string(), command);
    }

    /// Get the current prefix.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Update the command prefix.
    pub fn set_prefix(&mut self, prefix: String) {
        self.prefix = prefix;
    }

    /// Check if a message is a command (starts with the prefix).
    pub fn is_command(&self, message: &str) -> bool {
        message.starts_with(&self.prefix)
    }

    /// Parse a message and dispatch to the appropriate command handler.
    ///
    /// Returns `Some(CommandResult)` if the message was a recognized command,
    /// or `None` if the message was not a command or the command was not found.
    pub async fn dispatch(&self, ctx: CommandContext) -> Option<CommandResult> {
        let message = ctx.raw_message.trim();

        if !message.starts_with(&self.prefix) {
            return None;
        }

        // Strip the prefix
        let without_prefix = &message[self.prefix.len()..];

        // Split into command name and arguments
        let (cmd_name, args) = if let Some(space_pos) = without_prefix.find(char::is_whitespace) {
            (
                &without_prefix[..space_pos],
                without_prefix[space_pos..].trim(),
            )
        } else {
            (without_prefix, "")
        };

        // Ignore empty command (just the prefix)
        if cmd_name.is_empty() {
            return None;
        }

        // Find the command handler
        let command = match self.commands.get(cmd_name) {
            Some(c) => c,
            None => {
                tracing::info!(
                    command = %cmd_name,
                    user_id = %ctx.user_id,
                    session_id = %ctx.session_id,
                    "Unknown command ignored"
                );
                return None;
            }
        };

        // Check admin permission if required
        if command.require_admin() {
            let admin_ids = {
                let config = ctx.state.computer_use_config.read().await;
                config.admin_ids.clone()
            };
            if !admin_ids.is_empty() && !admin_ids.contains(&ctx.user_id) {
                tracing::warn!(
                    command = %cmd_name,
                    user_id = %ctx.user_id,
                    session_id = %ctx.session_id,
                    "Command rejected: insufficient permissions"
                );
                return Some(CommandResult::text("⛔ 权限不足：该指令需要管理员权限。"));
            }
        }

        // Build the parsed context with command_name filled
        let parsed_ctx = CommandContext {
            raw_message: ctx.raw_message.clone(),
            command_name: cmd_name.to_string(),
            args: args.to_string(),
            session_id: ctx.session_id,
            user_id: ctx.user_id,
            platform_id: ctx.platform_id,
            self_id: ctx.self_id,
            message_type: ctx.message_type,
            group_id: ctx.group_id,
            state: ctx.state,
        };

        tracing::info!(
            command = %cmd_name,
            args = %args,
            user_id = %parsed_ctx.user_id,
            session_id = %parsed_ctx.session_id,
            platform = %parsed_ctx.platform_id,
            message_type = %parsed_ctx.message_type_label(),
            "Executing built-in command"
        );

        let result = command.execute(&parsed_ctx).await;

        tracing::info!(
            command = %cmd_name,
            user_id = %parsed_ctx.user_id,
            session_id = %parsed_ctx.session_id,
            reply_len = result.reply.len(),
            "Command executed successfully"
        );

        Some(result)
    }

    /// List all registered commands (for `/help` output).
    pub fn list_commands(&self) -> Vec<&Arc<dyn Command>> {
        let mut cmds: Vec<_> = self.commands.values().collect();
        cmds.sort_by_key(|c| c.name());
        cmds
    }

    /// List all registered commands as serializable info structs.
    pub fn list_commands_info(&self, prefix: &str) -> Vec<BuiltinCommandInfo> {
        let mut cmds: Vec<_> = self
            .commands
            .values()
            .map(|c| BuiltinCommandInfo {
                name: c.name().to_string(),
                description: c.description().to_string(),
                usage: c.usage().replace('/', prefix).to_string(),
                require_admin: c.require_admin(),
                hidden: c.hidden(),
            })
            .collect();
        cmds.sort_by_key(|c| c.name.clone());
        cmds
    }
}

/// Summary information about a registered command (for API output).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BuiltinCommandInfo {
    /// The command name (without prefix), e.g. "help".
    pub name: String,
    /// A short description of what the command does.
    pub description: String,
    /// Usage hint, e.g. "/set <key> <value>".
    pub usage: String,
    /// Whether this command requires admin privileges.
    pub require_admin: bool,
    /// Whether this command should be hidden from listings.
    pub hidden: bool,
}

/// Create a dispatcher pre-loaded with all built-in commands.
pub fn create_builtin_dispatcher() -> CommandDispatcher {
    let mut dispatcher = CommandDispatcher::new();

    dispatcher.register(Arc::new(builtin::HelpCommand));
    dispatcher.register(Arc::new(builtin::SidCommand));
    dispatcher.register(Arc::new(builtin::ResetCommand));
    dispatcher.register(Arc::new(builtin::StopCommand));
    dispatcher.register(Arc::new(builtin::NewCommand));
    dispatcher.register(Arc::new(builtin::SetCommand));
    dispatcher.register(Arc::new(builtin::UnsetCommand));
    dispatcher.register(Arc::new(builtin::DashboardUpdateCommand));

    tracing::info!(
        count = dispatcher.commands.len(),
        "Built-in command dispatcher initialized"
    );

    dispatcher
}
