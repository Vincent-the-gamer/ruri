use crate::types::ChatMessage;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A Skill is a modular capability that can be attached to an Agent.
///
/// Skills can:
/// - Inject system prompts to guide the model's behavior
/// - Pre-process user messages before they reach the model
/// - Post-process the model's responses
/// - Maintain state across conversation turns
///
/// Examples of skills:
/// - "Memory" — stores and retrieves conversation context
/// - "WebSearch" — augments responses with web search results
/// - "CodeExecution" — runs code and feeds results back
/// - "RAG" — retrieves relevant documents to augment prompts
#[async_trait]
pub trait Skill: Send + Sync {
    /// The name of this skill.
    fn name(&self) -> &str;

    /// A short description of this skill, used for skill indexing/routing.
    /// Defaults to an empty string if not overridden.
    fn description(&self) -> &str {
        ""
    }

    /// Called once when the skill is attached to an Agent.
    /// Return a list of system messages that should be injected.
    async fn on_attach(&self) -> Vec<ChatMessage> {
        Vec::new()
    }

    /// Pre-process a user message before it's sent to the model.
    /// Returns the (possibly modified) list of messages.
    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        let _ = messages; // no-op by default
    }

    /// Post-process a model response after it's received.
    /// Returns the (possibly modified) response message.
    async fn on_response(&self, response: &mut ChatMessage) {
        let _ = response; // no-op by default
    }

    /// Called when a tool call result is available.
    /// Returns whether the agent should continue processing.
    async fn on_tool_result(&self, tool_name: &str, result: &str) -> bool {
        let _ = (tool_name, result);
        true // continue by default
    }

    /// Whether this skill should be active for the current conversation turn.
    fn is_active(&self) -> bool {
        true
    }
}

// ─── Built-in Skills ─────────────────────────────────────────────────

/// A simple system prompt skill that injects a fixed system message.
pub struct SystemPromptSkill {
    prompt: String,
}

impl SystemPromptSkill {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }
}

#[async_trait]
impl Skill for SystemPromptSkill {
    fn name(&self) -> &str {
        "system_prompt"
    }
    fn description(&self) -> &str {
        "Injects a custom system prompt"
    }
    async fn on_attach(&self) -> Vec<ChatMessage> {
        vec![ChatMessage::system(&self.prompt)]
    }
}

/// A skill that adds conversation memory by tracking message history.
pub struct MemorySkill {
    max_messages: usize,
}

impl MemorySkill {
    pub fn new(max_messages: usize) -> Self {
        Self { max_messages }
    }
}

#[async_trait]
impl Skill for MemorySkill {
    fn name(&self) -> &str {
        "memory"
    }
    fn description(&self) -> &str {
        "Manages conversation memory by trimming old messages"
    }

    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        // Keep system messages and the most recent messages
        if messages.len() > self.max_messages {
            let system_count = messages
                .iter()
                .take_while(|m| m.role == crate::types::MessageRole::System)
                .count();

            let available = self.max_messages.saturating_sub(system_count);
            if messages.len() - system_count > available {
                let drain_count = messages.len() - system_count - available;
                messages.drain(system_count..system_count + drain_count);
            }
        }
    }
}

/// A skill that prefixes user messages with context information.
pub struct ContextPrefixSkill {
    prefix: String,
}

impl ContextPrefixSkill {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

#[async_trait]
impl Skill for ContextPrefixSkill {
    fn name(&self) -> &str {
        "context_prefix"
    }
    fn description(&self) -> &str {
        "Prepends a context prefix to user messages"
    }

    async fn on_user_message(&self, messages: &mut Vec<ChatMessage>) {
        if let Some(last) = messages.last_mut()
            && last.role == crate::types::MessageRole::User
            && let Some(crate::types::MessageContent::Text(ref text)) = last.content
        {
            let new_content = format!("{}\n\n{}", self.prefix, text);
            last.content = Some(crate::types::MessageContent::Text(new_content));
        }
    }
}

// ─── Skill Package (SKILL.md) ──────────────────────────────────────────

/// Configuration for a hook that runs a shell command.
#[derive(Debug, Clone)]
pub struct SkillHook {
    /// The shell command to run.
    pub command: String,
    /// Whether to capture and inject the output as context.
    pub capture_output: bool,
}

/// A full-featured skill parsed from a SKILL.md package.
///
/// Unlike the simple `SystemPromptSkill`, `SkillPackageSkill` interprets
/// the SKILL.md frontmatter fields:
///
/// - **`when_to_use`**: condition description for when this skill is relevant
///   (injected into system prompt and used for `is_active` heuristics).
/// - **`argument_hint`** / **`arguments`**: parameter definitions injected into
///   the system prompt so the model knows how to invoke the skill.
/// - **`allowed_tools`**: tool whitelist — tools the model is allowed to use
///   within this skill.
/// - **`context`**: additional context string prepended to the skill prompt.
/// - **`hooks`**: pre/post processing hooks (shell commands to run).
/// - **`paths`**: file paths injected as available context.
/// - **`shell`**: shell command that defines the skill's execution logic.
/// - **`user_invocable`**: whether the user can directly invoke this skill.
/// - **`disable_model_invocation`**: whether to skip model invocation after
///   skill processing.
#[allow(dead_code)]
pub struct SkillPackageSkill {
    /// Skill name (from frontmatter `name` or directory name).
    name: String,
    /// Skill description.
    description: String,
    /// Main markdown content of the SKILL.md.
    content: String,

    // ── Frontmatter fields ──────────────────────────────────────
    /// When this skill should be used (injected into system prompt).
    when_to_use: Option<String>,
    /// Hint for how to pass arguments to this skill.
    argument_hint: Option<String>,
    /// Structured argument definitions (JSON value from YAML).
    arguments: Option<serde_json::Value>,
    /// Whether to disable model invocation after this skill runs.
    disable_model_invocation: bool,
    /// Whether the user can directly invoke this skill.
    user_invocable: bool,
    /// Tools that are allowed within this skill.
    allowed_tools: Option<Vec<String>>,
    /// Model override for this skill.
    model: Option<String>,
    /// Reasoning effort override for this skill.
    effort: Option<String>,
    /// Additional context to inject.
    context: Option<String>,
    /// Agent configuration reference.
    agent: Option<String>,
    /// Hooks to run at various lifecycle points.
    hooks: Vec<SkillHook>,
    /// File paths relevant to this skill.
    paths: Vec<String>,
    /// Shell command that defines the skill's core execution logic.
    shell: Option<String>,

    // ── Runtime state ───────────────────────────────────────────
    /// Cached result of the most recent shell/hook execution.
    /// Shared via `RwLock` so the async Skill trait methods can access it.
    shell_output: Arc<RwLock<Option<String>>>,
}

impl SkillPackageSkill {
    /// Build a `SkillPackageSkill` from the stored skill config JSON.
    ///
    /// The `config` value is the same JSON object that `upload_skill_package`
    /// builds from the SKILL.md frontmatter.
    pub fn from_config(name: String, description: String, config: &serde_json::Value) -> Self {
        let content = config["content"].as_str().unwrap_or("").to_string();
        let when_to_use = config["when_to_use"].as_str().map(|s| s.to_string());
        let argument_hint = config["argument_hint"].as_str().map(|s| s.to_string());
        let arguments = config.get("arguments").cloned();
        let disable_model_invocation = config["disable_model_invocation"]
            .as_bool()
            .unwrap_or(false);
        let user_invocable = config["user_invocable"].as_bool().unwrap_or(true);

        let allowed_tools = config
            .get("allowed_tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });

        let model = config["model"].as_str().map(|s| s.to_string());
        let effort = config["effort"].as_str().map(|s| s.to_string());
        let context = config["context"].as_str().map(|s| s.to_string());
        let agent = config["agent"].as_str().map(|s| s.to_string());

        // Parse hooks: expected format is { "pre": "cmd", "post": "cmd" }
        // or an array of { "command": "...", "capture_output": true }
        let hooks = Self::parse_hooks(config.get("hooks"));

        // Parse paths: can be a string or array of strings
        let paths = Self::parse_paths(config.get("paths"));

        let shell = config["shell"].as_str().map(|s| s.to_string());

        Self {
            name,
            description,
            content,
            when_to_use,
            argument_hint,
            arguments,
            disable_model_invocation,
            user_invocable,
            allowed_tools,
            model,
            effort,
            context,
            agent,
            hooks,
            paths,
            shell,
            shell_output: Arc::new(RwLock::new(None)),
        }
    }

    /// Parse hooks from the config JSON value.
    fn parse_hooks(hooks_val: Option<&serde_json::Value>) -> Vec<SkillHook> {
        let Some(val) = hooks_val else {
            return Vec::new();
        };

        // Format 1: object with "pre" and/or "post" keys
        if let Some(obj) = val.as_object() {
            let mut result = Vec::new();
            for key in &["pre", "post", "on_start", "on_end"] {
                if let Some(cmd) = obj.get(*key).and_then(|v| v.as_str()) {
                    result.push(SkillHook {
                        command: cmd.to_string(),
                        capture_output: true,
                    });
                }
            }
            return result;
        }

        // Format 2: array of objects with "command" and optional "capture_output"
        if let Some(arr) = val.as_array() {
            return arr
                .iter()
                .filter_map(|item| {
                    let command = item.get("command")?.as_str()?.to_string();
                    let capture_output = item
                        .get("capture_output")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    Some(SkillHook {
                        command,
                        capture_output,
                    })
                })
                .collect();
        }

        // Format 3: single string command
        if let Some(cmd) = val.as_str() {
            return vec![SkillHook {
                command: cmd.to_string(),
                capture_output: true,
            }];
        }

        Vec::new()
    }

    /// Parse paths from the config JSON value.
    fn parse_paths(paths_val: Option<&serde_json::Value>) -> Vec<String> {
        let Some(val) = paths_val else {
            return Vec::new();
        };

        // Array of strings
        if let Some(arr) = val.as_array() {
            return arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        // Single string
        if let Some(s) = val.as_str() {
            return vec![s.to_string()];
        }

        Vec::new()
    }

    /// Build the full system prompt for this skill.
    fn build_system_prompt(&self) -> String {
        let mut parts = Vec::new();

        // Skill header
        parts.push(format!("# Skill: {}", self.name));

        if !self.description.is_empty() {
            parts.push(format!("\n{}", self.description));
        }

        // When to use
        if let Some(ref when) = self.when_to_use {
            parts.push(format!("\n## When to Use\n{}", when));
        }

        // Arguments
        if let Some(ref hint) = self.argument_hint {
            parts.push(format!("\n## Arguments\n{}", hint));
        }
        if let Some(ref args) = self.arguments {
            if let Some(args_str) = serde_json::to_string_pretty(args).ok() {
                parts.push(format!("\n```json\n{}\n```", args_str));
            }
        }

        // Allowed tools
        if let Some(ref tools) = self.allowed_tools {
            parts.push(format!("\n## Allowed Tools\n{}", tools.join(", ")));
        }

        // Paths
        if !self.paths.is_empty() {
            parts.push(format!(
                "\n## Relevant Paths\n{}",
                self.paths
                    .iter()
                    .map(|p| format!("- `{}`", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        // Context
        if let Some(ref ctx) = self.context {
            parts.push(format!("\n## Context\n{}", ctx));
        }

        // Invocability
        if self.user_invocable {
            parts.push("\nThis skill can be invoked directly by the user.".to_string());
        }
        if self.disable_model_invocation {
            parts.push(
                "\nThis skill runs without model invocation — only the skill logic is executed."
                    .to_string(),
            );
        }

        // Main content
        if !self.content.is_empty() {
            parts.push(format!("\n---\n{}", self.content));
        }

        parts.join("\n")
    }

    /// Run a shell command and return its output.
    ///
    /// Includes a configurable timeout to prevent hanging commands from
    /// blocking the agent processing loop indefinitely.
    pub async fn run_shell_command(command: &str) -> Result<String, String> {
        Self::run_shell_command_with_timeout(command, 60).await
    }

    /// Run a shell command with a custom timeout (in seconds).
    pub async fn run_shell_command_with_timeout(
        command: &str,
        timeout_secs: u64,
    ) -> Result<String, String> {
        tracing::info!(command = %command, timeout_secs = timeout_secs, "Skill executing shell command");

        #[cfg(target_os = "windows")]
        let shell_future = tokio::process::Command::new("cmd")
            .args(["/C", command])
            .output();

        #[cfg(not(target_os = "windows"))]
        let shell_future = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output();

        let output =
            tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), shell_future)
                .await
                .map_err(|_| format!("Shell command timed out after {} seconds", timeout_secs))?;

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();

                if out.status.success() {
                    if stderr.is_empty() {
                        Ok(stdout)
                    } else {
                        Ok(format!("{}\n[stderr]: {}", stdout, stderr))
                    }
                } else {
                    Err(format!(
                        "Shell command failed (exit {}): {}",
                        out.status.code().unwrap_or(-1),
                        stderr
                    ))
                }
            }
            Err(e) => Err(format!("Failed to execute shell command: {}", e)),
        }
    }

    /// Run all hooks and collect outputs.
    pub async fn run_hooks(&self) -> Vec<String> {
        let mut outputs = Vec::new();
        for hook in &self.hooks {
            match Self::run_shell_command(&hook.command).await {
                Ok(output) if hook.capture_output => {
                    tracing::info!(
                        hook_command = %hook.command,
                        output_len = output.len(),
                        "Skill hook executed successfully"
                    );
                    outputs.push(output);
                }
                Ok(_) => {
                    tracing::info!(
                        hook_command = %hook.command,
                        "Skill hook executed successfully (output not captured)"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        hook_command = %hook.command,
                        error = %e,
                        "Skill hook execution failed"
                    );
                    outputs.push(format!("[Hook error: {}]", e));
                }
            }
        }
        outputs
    }

    /// Get the set of allowed tool names for this skill.
    #[allow(dead_code)]
    pub fn allowed_tool_set(&self) -> Option<HashSet<String>> {
        self.allowed_tools
            .as_ref()
            .map(|tools| tools.iter().cloned().collect())
    }

    /// Get the model override for this skill.
    #[allow(dead_code)]
    pub fn model_override(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Get the effort override for this skill.
    #[allow(dead_code)]
    pub fn effort_override(&self) -> Option<&str> {
        self.effort.as_deref()
    }

    /// Whether model invocation should be disabled for this skill.
    #[allow(dead_code)]
    pub fn should_disable_model_invocation(&self) -> bool {
        self.disable_model_invocation
    }

    /// Get the shell command for this skill, if defined.
    pub fn shell_command(&self) -> &Option<String> {
        &self.shell
    }

    /// Whether this skill is user-invocable.
    #[allow(dead_code)]
    pub fn is_user_invocable(&self) -> bool {
        self.user_invocable
    }

    /// When this skill should be used, for routing.
    #[allow(dead_code)]
    pub fn when_to_use(&self) -> Option<&str> {
        self.when_to_use.as_deref()
    }
}

#[async_trait]
impl Skill for SkillPackageSkill {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn on_attach(&self) -> Vec<ChatMessage> {
        let mut prompt = self.build_system_prompt();

        // Run hooks on attach and inject their output as context
        let hook_outputs = self.run_hooks().await;
        if !hook_outputs.is_empty() {
            prompt.push_str(&format!(
                "\n\n## Hook Output\n{}",
                hook_outputs.join("\n\n")
            ));
        }

        // Run the shell command if defined and inject its output
        if let Some(ref shell_cmd) = self.shell {
            match Self::run_shell_command(shell_cmd).await {
                Ok(output) => {
                    tracing::info!(
                        skill = %self.name,
                        output_len = output.len(),
                        "Skill shell command executed on attach"
                    );
                    // Cache the output for later use
                    *self.shell_output.write().await = Some(output.clone());
                    prompt.push_str(&format!(
                        "\n\n## Shell Command Output\n```\n{}\n```",
                        output
                    ));
                }
                Err(e) => {
                    tracing::warn!(
                        skill = %self.name,
                        error = %e,
                        "Skill shell command failed on attach"
                    );
                    prompt.push_str(&format!("\n\n## Shell Command Error\n```\n{}\n```", e));
                }
            }
        }

        if prompt.is_empty() {
            Vec::new()
        } else {
            // Return as a system message — this will be collected by
            // Agent::initialize_skills() into skill_contexts and dynamically
            // injected into user messages, NOT as a system prompt.
            vec![ChatMessage::system(&prompt)]
        }
    }

    async fn on_user_message(&self, _messages: &mut Vec<ChatMessage>) {
        // The skill's static context (from on_attach) is automatically injected
        // into user messages by Agent::inject_skill_contexts(). We only need
        // to handle dynamic per-turn context here.

        // If this skill has a "when_to_use" directive, skip automatic context injection.
        // The collected context from on_attach() already describes when to use
        // this skill, and the model will apply it based on relevance.
        if self.when_to_use.is_some() {
            return;
        }

        // Skills without when_to_use are "always-on" context injectors.
        // The static context (context prefix, shell output) is already handled
        // by inject_skill_contexts(). We don't need to duplicate it here.
        // Dynamic per-turn logic would go here if needed in the future.
    }

    async fn on_response(&self, response: &mut ChatMessage) {
        // If hooks contain a "post" command, run it with the response text
        // This allows post-processing of the model's output
        let _ = response; // Currently no post-processing needed
    }

    async fn on_tool_result(&self, tool_name: &str, _result: &str) -> bool {
        // If allowed_tools is set, check if this tool is in the whitelist
        if let Some(ref tools) = self.allowed_tools {
            if !tools.contains(&tool_name.to_string()) {
                tracing::warn!(
                    skill = %self.name,
                    tool = %tool_name,
                    "Tool not in skill's allowed_tools list, but executed anyway (whitelist is advisory)"
                );
            }
        }
        // Continue processing
        true
    }

    fn is_active(&self) -> bool {
        // Skills are active by default.
        // If user_invocable is false, the skill still works but can't be
        // directly invoked by the user. It remains active for the system.
        true
    }
}
