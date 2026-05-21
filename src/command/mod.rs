//! Built-in command system for Ruri.
//!
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
//! - `/whoami` — Show current user's ID, identity and admin status
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
    /// The command prefix configured for the active profile (e.g. "/" or "#").
    /// Populated by the dispatcher so that command handlers can use the
    /// correct prefix in messages (usage hints, help text, etc.).
    pub prefix: String,
    /// List of enabled built-in command names for the active profile.
    /// If non-empty, only commands in this list are considered enabled;
    /// if empty, all commands are considered enabled (fallback).
    pub enabled_commands: Vec<String>,
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
    enabled_commands: Vec<String>,
}

impl CommandDispatcher {
    /// Create a new dispatcher with the default `/` prefix.
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            prefix: DEFAULT_PREFIX.to_string(),
            enabled_commands: Vec::new(),
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

    /// Get a reference to the registered commands map.
    pub fn commands(&self) -> &HashMap<String, Arc<dyn Command>> {
        &self.commands
    }

    /// Get the current prefix.
    #[allow(dead_code)]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Update the command prefix.
    pub fn set_prefix(&mut self, prefix: String) {
        self.prefix = prefix;
    }

    /// Update the list of enabled commands.
    pub fn set_enabled_commands(&mut self, commands: Vec<String>) {
        self.enabled_commands = commands;
    }

    /// Check if a command is enabled.
    #[allow(dead_code)]
    pub fn is_command_enabled(&self, command_name: &str) -> bool {
        self.enabled_commands.contains(&command_name.to_string())
    }

    /// Check if a message starts with the command prefix.
    ///
    /// Note: a message starting with the prefix is **not** necessarily a known
    /// command — it may be a prefix-only or unrecognized command that should
    /// fall through to the LLM. Use `dispatch()` for the actual routing decision.
    #[allow(dead_code)]
    pub fn is_command(&self, message: &str) -> bool {
        message.starts_with(&self.prefix)
    }

    /// Parse a message and dispatch to the appropriate command handler.
    ///
    /// Returns `Some(CommandResult)` if a known command was matched and executed
    /// (including disabled/admin-rejected commands which return an error message),
    /// or `None` if the message should fall through to the LLM (no prefix,
    /// prefix-only, or prefix + unrecognized command name).
    pub async fn dispatch(&self, ctx: CommandContext) -> Option<CommandResult> {
        let message = ctx.raw_message.trim();

        if !message.starts_with(&ctx.prefix) {
            return None;
        }

        // Strip the prefix
        let without_prefix = &message[ctx.prefix.len()..];

        // Split into command name and arguments
        let (cmd_name, args) = if let Some(space_pos) = without_prefix.find(char::is_whitespace) {
            (
                &without_prefix[..space_pos],
                without_prefix[space_pos..].trim(),
            )
        } else {
            (without_prefix, "")
        };

        // Empty command (just the prefix) — treat as a normal message
        // so it falls through to the LLM.
        if cmd_name.is_empty() {
            return None;
        }

        // Find the command handler
        let command = match self.commands.get(cmd_name) {
            Some(c) => c,
            None => {
                // No built-in command matched — check if a skill package with
                // `user_invocable: true` and `disable_model_invocation: true`
                // can handle this directly (without calling the LLM).
                if let Some(result) =
                    Self::try_dispatch_skill_command(ctx.state.clone(), cmd_name, args, &ctx).await
                {
                    return Some(result);
                }

                // Unknown command — let it fall through to the LLM instead of
                // returning an error, so that prefix-like messages still reach
                // the model for a natural response.
                tracing::info!(
                    command = %cmd_name,
                    user_id = %ctx.user_id,
                    session_id = %ctx.session_id,
                    "Unrecognized command, falling through to LLM"
                );
                return None;
            }
        };

        // Check if this command is enabled (per-context)
        let is_enabled = if ctx.enabled_commands.is_empty() {
            // Empty enabled_commands means "all commands enabled" (backward compat / default behavior)
            true
        } else {
            ctx.enabled_commands.iter().any(|c| c == cmd_name)
        };
        if !is_enabled {
            tracing::info!(
                command = %cmd_name,
                user_id = %ctx.user_id,
                session_id = %ctx.session_id,
                "Command disabled"
            );
            return Some(CommandResult::text(format!("⚠ 指令 {} 已禁用。", cmd_name)));
        }

        // Check admin permission: use per-command override from config,
        // falling back to the command's default `require_admin()`.
        let require_admin = {
            let config = ctx.state.computer_use_config.read().await;
            config.is_command_admin_required(cmd_name, command.require_admin())
        };

        if require_admin {
            // WebUI users are always considered admins (they are authenticated)
            let is_webui = ctx.platform_id == "webui";
            let is_admin = if is_webui {
                true
            } else {
                let config = ctx.state.computer_use_config.read().await;
                config.admin_ids.is_empty() || config.is_admin(&ctx.user_id)
            };

            if !is_admin {
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
            prefix: ctx.prefix.clone(),
            enabled_commands: ctx.enabled_commands.clone(),
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
    ///
    /// `command_admin_required` comes from the active config profile and provides
    /// runtime overrides for the per-command admin requirement.
    /// `enabled_commands` determines which commands are enabled.
    pub fn list_commands_info(
        &self,
        prefix: &str,
        command_admin_required: &std::collections::HashMap<String, bool>,
        enabled_commands: &[String],
    ) -> Vec<BuiltinCommandInfo> {
        let mut cmds: Vec<_> = self
            .commands
            .values()
            .map(|c| {
                let default = c.require_admin();
                let effective = command_admin_required
                    .get(c.name())
                    .copied()
                    .unwrap_or(default);
                BuiltinCommandInfo {
                    name: c.name().to_string(),
                    description: c.description().to_string(),
                    usage: c.usage().replace('/', prefix).to_string(),
                    require_admin: effective,
                    default_require_admin: default,
                    hidden: c.hidden(),
                    enabled: enabled_commands.contains(&c.name().to_string()),
                }
            })
            .collect();
        cmds.sort_by_key(|c| c.name.clone());
        cmds
    }

    /// Try to dispatch a command as a skill package invocation.
    ///
    /// When a built-in command is not found, this method checks whether a skill
    /// package with `user_invocable: true` and `disable_model_invocation: true`
    /// matches the command name. If so, the skill is executed directly (shell,
    /// hooks) and its output is returned **without calling the LLM**.
    async fn try_dispatch_skill_command(
        state: Arc<crate::api::AppState>,
        cmd_name: &str,
        args: &str,
        ctx: &CommandContext,
    ) -> Option<CommandResult> {
        use crate::agent::skill::SkillPackageSkill;

        // Resolve the skill names that are active for the current context.
        // For WebUI (platform_id == "webui") we use the debug session skills;
        // for platform messages we use the config profile skills.
        let skill_configs: Vec<(String, String, serde_json::Value)> = {
            if ctx.platform_id == "webui" {
                // Use debug session skills
                let debug = state.debug_session.read().await;
                debug
                    .skills
                    .iter()
                    .filter(|s| debug.active_skill_names.contains(&s.name))
                    .map(|s| {
                        let config = serde_json::to_value(s.config.clone()).unwrap_or_default();
                        (s.name.clone(), s.description.clone(), config)
                    })
                    .collect()
            } else {
                // Use global skills filtered by the active config profile
                let skills = state.skills.read().await;
                // Determine active skill names from config profiles
                let profiles = state.config_profiles.read().await;
                let active_profile = profiles
                    .values()
                    .find(|p| p.is_active && p.enable && p.platform_ids.contains(&ctx.platform_id))
                    .or_else(|| profiles.values().find(|p| p.is_active && p.enable));

                let active_names = active_profile
                    .map(|p| p.active_skill_names.as_slice())
                    .unwrap_or(&[]);

                skills
                    .iter()
                    .filter(|(_, s)| s.is_active)
                    .filter(|(name, _)| active_names.contains(name))
                    .map(|(name, s)| (name.clone(), s.description.clone(), s.config.clone()))
                    .collect()
            }
        };

        // Find a skill matching the command name
        for (skill_name, description, config) in &skill_configs {
            if skill_name != cmd_name {
                continue;
            }

            // Check if this skill is user_invocable and disable_model_invocation
            let user_invocable = config["user_invocable"].as_bool().unwrap_or(true);
            let disable_model_invocation = config["disable_model_invocation"]
                .as_bool()
                .unwrap_or(false);

            if !user_invocable || !disable_model_invocation {
                continue;
            }

            tracing::info!(
                skill = %skill_name,
                user_id = %ctx.user_id,
                session_id = %ctx.session_id,
                platform = %ctx.platform_id,
                "Dispatching skill command without model invocation"
            );

            // Build the skill from config
            let skill =
                SkillPackageSkill::from_config(skill_name.clone(), description.clone(), config);

            // Get the shell command blacklist for security enforcement
            let blacklist = ctx.state.shell_command_blacklist.read().await.clone();

            // Execute the skill's shell command if defined
            let mut result_parts = Vec::new();

            // Run hooks
            let hook_outputs = skill.run_hooks(&blacklist).await;
            if !hook_outputs.is_empty() {
                result_parts.push(format!(
                    "📦 Skill '{}':\n{}",
                    skill_name,
                    hook_outputs.join("\n\n")
                ));
            }

            // Run the shell command if defined
            if let Some(shell_cmd) = skill.shell_command() {
                // Inject args into the shell command if provided
                let full_cmd = if args.is_empty() {
                    shell_cmd.clone()
                } else {
                    format!("{} {}", shell_cmd, args)
                };

                match SkillPackageSkill::run_shell_command(&full_cmd, &blacklist).await {
                    Ok(output) => {
                        tracing::info!(
                            skill = %skill_name,
                            output_len = output.len(),
                            "Skill command executed"
                        );
                        result_parts.push(output);
                    }
                    Err(e) => {
                        tracing::error!(
                            skill = %skill_name,
                            error = %e,
                            "Skill command failed"
                        );
                        result_parts.push(format!("❌ 执行失败：{}", e));
                    }
                }
            } else if result_parts.is_empty() {
                // No shell and no hooks — nothing to execute
                result_parts.push(format!(
                    "📦 Skill '{}' executed (no shell command or hooks defined)",
                    skill_name
                ));
            }

            return Some(CommandResult::text(result_parts.join("\n\n")));
        }

        None
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
    /// Whether this command currently requires admin privileges
    /// (reflecting any runtime override from config).
    pub require_admin: bool,
    /// The built-in default for whether this command requires admin privileges.
    pub default_require_admin: bool,
    /// Whether this command should be hidden from listings.
    pub hidden: bool,
    /// Whether this command is enabled in the active config profile.
    pub enabled: bool,
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
    dispatcher.register(Arc::new(builtin::WhoamiCommand));
    dispatcher.register(Arc::new(builtin::DashboardUpdateCommand));

    tracing::info!(
        count = dispatcher.commands.len(),
        "Built-in command dispatcher initialized"
    );

    dispatcher
}
