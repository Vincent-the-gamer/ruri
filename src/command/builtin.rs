//! Built-in command implementations for Ruri.
//!
//! - `/help` — Show available commands and version info
//! - `/sid`  — Show current session info (UMO, UID, Bot ID, etc.)
//! - `/reset` — Reset the current conversation's LLM context
//! - `/stop` — Stop the currently running agent task in the current session
//! - `/new` — Create and switch to a new conversation
//! - `/set <key> <value>` — Set a session variable
//! - `/unset <key>` — Remove a session variable
//! - `/dashboard_update` — Update the WebUI (requires admin)
//!
//! Every command execution emits structured tracing logs at the operation level,
//! which are automatically captured by the LogManager and pushed to the frontend
//! via the WebSocket broadcast pipeline.

use crate::command::{Command, CommandContext, CommandResult};
use async_trait::async_trait;

// ─── /help ──────────────────────────────────────────────────────

/// `/help` — Show available commands and version info.
pub struct HelpCommand;

#[async_trait]
impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn description(&self) -> &str {
        "查看当前启用的指令和 Ruri 版本信息"
    }

    fn usage(&self) -> &str {
        "/help"
    }

    fn hidden(&self) -> bool {
        true
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        let version = env!("CARGO_PKG_VERSION");

        let mut lines = Vec::new();
        lines.push(format!("🤖 Ruri v{}", version));
        lines.push(String::new());
        lines.push("可用指令：".to_string());

        // Access the dispatcher through state to list commands
        let dispatcher = ctx.state.command_dispatcher.read().await;
        let commands = dispatcher.list_commands();

        let mut visible_count = 0;
        for cmd in &commands {
            if cmd.hidden() {
                continue;
            }
            visible_count += 1;
            let admin_marker = if cmd.require_admin() { " 🔒" } else { "" };
            lines.push(format!(
                "  {} — {}{}",
                cmd.usage(),
                cmd.description(),
                admin_marker
            ));
        }

        lines.push(String::new());
        lines.push("提示：/help、/set、/unset 默认不在列表中显示，但仍可使用。".to_string());

        tracing::info!(
            command = %ctx.command_name,
            user_id = %ctx.user_id,
            session_id = %ctx.session_id,
            visible_commands = visible_count,
            total_commands = commands.len(),
            "Listed available commands"
        );

        CommandResult::text(lines.join("\n"))
    }
}

// ─── /sid ───────────────────────────────────────────────────────

/// `/sid` — Show current session info.
pub struct SidCommand;

#[async_trait]
impl Command for SidCommand {
    fn name(&self) -> &str {
        "sid"
    }

    fn description(&self) -> &str {
        "查看当前消息来源信息（UMO、用户 ID、平台 ID 等）"
    }

    fn usage(&self) -> &str {
        "/sid"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        let umo = &ctx.session_id;
        let uid = &ctx.user_id;
        let bot_id = &ctx.self_id;
        let platform_id = &ctx.platform_id;
        let message_type_label = ctx.message_type_label();
        let session_id = &ctx.session_id;

        let mut lines = Vec::new();
        lines.push("📋 当前消息来源信息：".to_string());
        lines.push(format!("  UMO（统一标识）: {}", umo));
        lines.push(format!("  UID（用户 ID）: {}", uid));
        lines.push(format!("  Bot ID: {}", bot_id));
        lines.push(format!("  Platform ID: {}", platform_id));
        lines.push(format!("  消息类型: {}", message_type_label));
        lines.push(format!("  Session ID: {}", session_id));

        if !ctx.group_id.is_empty() {
            lines.push(format!("  群 ID: {}", ctx.group_id));
            lines.push(String::new());
            lines.push("💡 群 ID 可用于将整个群加入白名单。".to_string());
        }

        lines.push(String::new());
        lines.push("💡 常见用途：".to_string());
        lines.push("  - 添加管理员：获取 UID 后在 WebUI 配置中添加".to_string());
        lines.push("  - 配置白名单：使用 UMO 或群 ID 控制访问".to_string());

        tracing::info!(
            command = %ctx.command_name,
            user_id = %uid,
            session_id = %session_id,
            umo = %umo,
            bot_id = %bot_id,
            platform = %platform_id,
            message_type = %message_type_label,
            "User requested session info"
        );

        CommandResult::text(lines.join("\n"))
    }
}

// ─── /reset ─────────────────────────────────────────────────────

/// `/reset` — Reset the current conversation's LLM context.
pub struct ResetCommand;

#[async_trait]
impl Command for ResetCommand {
    fn name(&self) -> &str {
        "reset"
    }

    fn description(&self) -> &str {
        "重置当前会话的 LLM 上下文"
    }

    fn usage(&self) -> &str {
        "/reset"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        let session_id = &ctx.session_id;
        let mut cancelled_task = false;
        let mut deleted_conversations = 0;

        // 1. Cancel any running task for this session
        {
            let mut tasks = ctx.state.running_agent_tasks.write().await;
            if let Some(cancel_token) = tasks.remove(session_id) {
                cancel_token.cancel();
                cancelled_task = true;
                tracing::info!(
                    command = %ctx.command_name,
                    session_id = %session_id,
                    "Cancelled running agent task"
                );
            }
        }

        // 2. Clear conversation from the database
        let conv_db = ctx.state.conversation_db.read().await;
        if let Some(db) = conv_db.as_ref() {
            // Find conversations for this session and delete them
            let conversations = db
                .list_conversations(Some(crate::conversation::models::ConversationFilter {
                    bot_name: None,
                    chat_type: None,
                    keyword: Some(session_id.clone()),
                }))
                .await;

            if let Ok(convs) = conversations {
                for conv in convs {
                    if conv.chat_id == *session_id {
                        if let Err(e) = db.delete_conversation(&conv.id).await {
                            tracing::warn!(
                                command = %ctx.command_name,
                                conversation_id = %conv.id,
                                error = %e,
                                "Failed to delete conversation"
                            );
                        } else {
                            deleted_conversations += 1;
                            tracing::info!(
                                command = %ctx.command_name,
                                conversation_id = %conv.id,
                                "Deleted conversation"
                            );
                        }
                    }
                }
            }

            // Reset the webui chat conversation ID only if this command
            // came from the webui (not from a platform adapter)
            if ctx.platform_id == "webui" {
                let chat_conv_id = ctx.state.chat_conversation_id.read().await;
                if chat_conv_id.is_some() {
                    drop(chat_conv_id);
                    let mut conv_id = ctx.state.chat_conversation_id.write().await;
                    *conv_id = None;
                }
            }
        } else {
            // No database - at least reset the in-memory conversation ID
            // (only for webui sessions)
            if ctx.platform_id == "webui" {
                let mut conv_id = ctx.state.chat_conversation_id.write().await;
                *conv_id = None;
            }
        }

        // 3. Clear session variables for this session
        {
            let mut vars = ctx.state.session_variables.write().await;
            let had_vars = vars.remove(session_id).is_some();
            if had_vars {
                tracing::info!(
                    command = %ctx.command_name,
                    session_id = %session_id,
                    "Cleared session variables"
                );
            }
        }

        tracing::info!(
            command = %ctx.command_name,
            user_id = %ctx.user_id,
            session_id = %session_id,
            cancelled_task = cancelled_task,
            deleted_conversations = deleted_conversations,
            "Conversation context reset complete"
        );

        CommandResult::text("✅ 当前会话已重置。上下文已清空，新对话将从零开始。")
    }
}

// ─── /stop ──────────────────────────────────────────────────────

/// `/stop` — Stop the currently running agent task.
pub struct StopCommand;

#[async_trait]
impl Command for StopCommand {
    fn name(&self) -> &str {
        "stop"
    }

    fn description(&self) -> &str {
        "停止当前会话中正在运行的 Agent 任务"
    }

    fn usage(&self) -> &str {
        "/stop"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        let session_id = &ctx.session_id;

        let mut tasks = ctx.state.running_agent_tasks.write().await;
        if let Some(cancel_token) = tasks.remove(session_id) {
            cancel_token.cancel();
            tracing::info!(
                command = %ctx.command_name,
                user_id = %ctx.user_id,
                session_id = %session_id,
                "Stopped running agent task"
            );
            CommandResult::text("⏹ 已停止当前运行中的任务。")
        } else {
            tracing::info!(
                command = %ctx.command_name,
                user_id = %ctx.user_id,
                session_id = %session_id,
                "No running task to stop"
            );
            CommandResult::text("ℹ️ 当前会话没有运行中的任务。")
        }
    }
}

// ─── /new ───────────────────────────────────────────────────────

/// `/new` — Create and switch to a new conversation.
pub struct NewCommand;

#[async_trait]
impl Command for NewCommand {
    fn name(&self) -> &str {
        "new"
    }

    fn description(&self) -> &str {
        "创建并切换到一个新对话"
    }

    fn usage(&self) -> &str {
        "/new"
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        let conv_db = ctx.state.conversation_db.read().await;

        if let Some(db) = conv_db.as_ref() {
            let chat_type = match ctx.message_type {
                crate::platform::types::MessageType::GroupMessage => {
                    crate::conversation::models::ChatType::Group
                }
                crate::platform::types::MessageType::FriendMessage => {
                    crate::conversation::models::ChatType::Private
                }
            };

            let chat_id = if ctx.group_id.is_empty() {
                ctx.session_id.clone()
            } else {
                ctx.group_id.clone()
            };

            match db
                .create_conversation(crate::conversation::models::CreateConversationRequest {
                    bot_name: ctx.platform_id.clone(),
                    chat_type,
                    chat_id: chat_id.clone(),
                    title: Some(format!(
                        "新对话 ({})",
                        chrono::Local::now().format("%Y-%m-%d %H:%M")
                    )),
                })
                .await
            {
                Ok(conversation) => {
                    tracing::info!(
                        command = %ctx.command_name,
                        user_id = %ctx.user_id,
                        session_id = %ctx.session_id,
                        conversation_id = %conversation.id,
                        chat_type = %ctx.message_type_label(),
                        "Created new conversation"
                    );

                    // Update the webui conversation ID only if this command
                    // came from the webui (not from a platform adapter)
                    if ctx.platform_id == "webui" {
                        let mut conv_id = ctx.state.chat_conversation_id.write().await;
                        *conv_id = Some(conversation.id.clone());
                    }

                    CommandResult::text(format!(
                        "✅ 已创建新对话。\n对话 ID: {}\n时间: {}",
                        conversation.id,
                        conversation.created_at.format("%Y-%m-%d %H:%M:%S")
                    ))
                }
                Err(e) => {
                    tracing::error!(
                        command = %ctx.command_name,
                        user_id = %ctx.user_id,
                        session_id = %ctx.session_id,
                        error = %e,
                        "Failed to create new conversation"
                    );
                    CommandResult::text(format!("❌ 创建新对话失败：{}", e))
                }
            }
        } else {
            // No database available, just reset in-memory state
            // (only for webui sessions)
            if ctx.platform_id == "webui" {
                let mut conv_id = ctx.state.chat_conversation_id.write().await;
                *conv_id = None;
            }

            tracing::warn!(
                command = %ctx.command_name,
                user_id = %ctx.user_id,
                session_id = %ctx.session_id,
                "Database unavailable, reset conversation in memory only"
            );

            CommandResult::text("✅ 已重置对话状态（数据库不可用，仅在内存中重置）。")
        }
    }
}

// ─── /set ───────────────────────────────────────────────────────

/// `/set <key> <value>` — Set a session variable.
pub struct SetCommand;

#[async_trait]
impl Command for SetCommand {
    fn name(&self) -> &str {
        "set"
    }

    fn description(&self) -> &str {
        "设置当前会话变量"
    }

    fn usage(&self) -> &str {
        "/set <key> <value>"
    }

    fn hidden(&self) -> bool {
        true
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        let args = ctx.args.trim();

        if args.is_empty() {
            tracing::info!(
                command = %ctx.command_name,
                user_id = %ctx.user_id,
                session_id = %ctx.session_id,
                "Set command called without arguments, showing usage"
            );
            return CommandResult::text("用法：/set <key> <value>\n示例：/set language zh-CN");
        }

        // Split into key and value
        let (key, value) = if let Some(space_pos) = args.find(char::is_whitespace) {
            (
                args[..space_pos].to_string(),
                args[space_pos..].trim().to_string(),
            )
        } else {
            (args.to_string(), String::new())
        };

        if key.is_empty() {
            tracing::warn!(
                command = %ctx.command_name,
                user_id = %ctx.user_id,
                session_id = %ctx.session_id,
                "Set command called with empty key"
            );
            return CommandResult::text("❌ 变量名不能为空。");
        }

        // Store the variable
        {
            let mut vars = ctx.state.session_variables.write().await;
            vars.entry(ctx.session_id.clone())
                .or_insert_with(std::collections::HashMap::new)
                .insert(key.clone(), value.clone());
        }

        tracing::info!(
            command = %ctx.command_name,
            user_id = %ctx.user_id,
            session_id = %ctx.session_id,
            key = %key,
            value = %value,
            "Session variable set"
        );

        CommandResult::text(format!("✅ 已设置会话变量：{} = {}", key, value))
    }
}

// ─── /unset ─────────────────────────────────────────────────────

/// `/unset <key>` — Remove a session variable.
pub struct UnsetCommand;

#[async_trait]
impl Command for UnsetCommand {
    fn name(&self) -> &str {
        "unset"
    }

    fn description(&self) -> &str {
        "移除当前会话变量"
    }

    fn usage(&self) -> &str {
        "/unset <key>"
    }

    fn hidden(&self) -> bool {
        true
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        let key = ctx.args.trim();

        if key.is_empty() {
            // If no key specified, show all variables for this session
            let vars = ctx.state.session_variables.read().await;
            if let Some(session_vars) = vars.get(&ctx.session_id) {
                if session_vars.is_empty() {
                    tracing::info!(
                        command = %ctx.command_name,
                        user_id = %ctx.user_id,
                        session_id = %ctx.session_id,
                        "No session variables set"
                    );
                    return CommandResult::text("ℹ️ 当前会话没有设置任何变量。");
                }
                let mut lines = vec!["📋 当前会话变量：".to_string()];
                for (k, v) in session_vars {
                    lines.push(format!("  {} = {}", k, v));
                }
                lines.push(String::new());
                lines.push("使用 /unset <key> 移除指定变量。".to_string());

                tracing::info!(
                    command = %ctx.command_name,
                    user_id = %ctx.user_id,
                    session_id = %ctx.session_id,
                    variable_count = session_vars.len(),
                    "Listed session variables"
                );

                return CommandResult::text(lines.join("\n"));
            } else {
                tracing::info!(
                    command = %ctx.command_name,
                    user_id = %ctx.user_id,
                    session_id = %ctx.session_id,
                    "No session variables set"
                );
                return CommandResult::text("ℹ️ 当前会话没有设置任何变量。");
            }
        }

        let mut vars = ctx.state.session_variables.write().await;
        if let Some(session_vars) = vars.get_mut(&ctx.session_id) {
            if session_vars.remove(&key.to_string()).is_some() {
                // Clean up empty session maps
                if session_vars.is_empty() {
                    vars.remove(&ctx.session_id);
                }

                tracing::info!(
                    command = %ctx.command_name,
                    user_id = %ctx.user_id,
                    session_id = %ctx.session_id,
                    key = %key,
                    "Session variable removed"
                );

                CommandResult::text(format!("✅ 已移除会话变量：{}", key))
            } else {
                tracing::info!(
                    command = %ctx.command_name,
                    user_id = %ctx.user_id,
                    session_id = %ctx.session_id,
                    key = %key,
                    "Variable not found"
                );
                CommandResult::text(format!("ℹ️ 变量 '{}' 不存在。", key))
            }
        } else {
            tracing::info!(
                command = %ctx.command_name,
                user_id = %ctx.user_id,
                session_id = %ctx.session_id,
                key = %key,
                "No variables for this session"
            );
            CommandResult::text(format!("ℹ️ 变量 '{}' 不存在。", key))
        }
    }
}

// ─── /dashboard_update ──────────────────────────────────────────

/// `/dashboard_update` — Update the WebUI (requires admin).
pub struct DashboardUpdateCommand;

#[async_trait]
impl Command for DashboardUpdateCommand {
    fn name(&self) -> &str {
        "dashboard_update"
    }

    fn description(&self) -> &str {
        "更新 Ruri WebUI（需要管理员权限）"
    }

    fn usage(&self) -> &str {
        "/dashboard_update"
    }

    fn require_admin(&self) -> bool {
        true
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult {
        // The WebUI is embedded at compile time via rust-embed, so we cannot
        // update it at runtime. Instead, we inform the user that they need to
        // rebuild the application.
        tracing::warn!(
            command = %ctx.command_name,
            user_id = %ctx.user_id,
            session_id = %ctx.session_id,
            "dashboard_update requested but WebUI is compile-time embedded"
        );

        CommandResult::text(
            "ℹ️ Ruri 的 WebUI 是在编译时嵌入的，无法在运行时更新。\n\
             要更新 WebUI，请拉取最新代码并重新构建：\n\
             \n\
             1. git pull\n\
             2. cd webui && pnpm install && pnpm build\n\
             3. cargo build --release",
        )
    }
}
