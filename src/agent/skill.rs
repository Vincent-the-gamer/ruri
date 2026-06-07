use crate::types::ChatMessage;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(target_os = "windows")]
use base64;

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
    ///
    /// Inactive skills (e.g. skills with `when_to_use` conditions) do NOT have
    /// their context collected or injected into messages. They are listed in the
    /// skill routing instruction so the model can invoke them on demand.
    fn is_active(&self) -> bool {
        true
    }

    /// When this skill should be used, for skill routing.
    /// Returns `None` for always-on skills (loaded by default).
    /// Returns `Some(description)` for conditional skills (loaded on demand).
    fn when_to_use(&self) -> Option<&str> {
        None
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
    ///
    /// The `blacklist` parameter contains shell command substrings that are
    /// strictly forbidden. If the command matches any blacklist entry, it is
    /// rejected before execution. This is a security measure to prevent
    /// dangerous operations.
    pub async fn run_shell_command(command: &str, blacklist: &[String]) -> Result<String, String> {
        Self::run_shell_command_with_timeout(command, 60, blacklist).await
    }

    /// Run a shell command with a custom timeout (in seconds).
    ///
    /// On Linux/macOS, uses `sh`. On Windows, uses PowerShell.
    ///
    /// Before execution, the command is checked against the `blacklist`.
    /// If the command matches any blacklisted pattern, it is rejected with
    /// a descriptive error that allows the caller (e.g. the AI agent) to
    /// try a different, safe command instead.
    pub async fn run_shell_command_with_timeout(
        command: &str,
        timeout_secs: u64,
        blacklist: &[String],
    ) -> Result<String, String> {
        // ── Blacklist check ──────────────────────────────────────
        // Strictly reject any command that matches a blacklisted pattern.
        // This prevents dangerous operations even when the AI agent
        // tries alternative commands after a failure.
        if !blacklist.is_empty() {
            let lower_cmd = command.to_lowercase();
            for entry in blacklist {
                if lower_cmd.contains(&entry.to_lowercase()) {
                    return Err(format!(
                        "⛔ Command blocked by security policy: the command contains a \
                         blacklisted pattern ('{entry}'). Please use a different, \
                         safe approach to accomplish this task. Do NOT attempt to \
                         bypass this restriction by encoding, escaping, or \
                         wrapping the command.",
                    ));
                }
            }
        }

        tracing::info!(command = %command, timeout_secs = timeout_secs, "Skill executing shell command");

        // ── Platform-specific execution ─────────────────────────
        #[cfg(target_os = "windows")]
        let output_result = Self::run_shell_command_windows(command, timeout_secs).await;

        #[cfg(not(target_os = "windows"))]
        let output_result = Self::run_shell_command_unix(command, timeout_secs).await;

        let output = output_result?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            if stderr.is_empty() {
                Ok(stdout)
            } else {
                Ok(format!("{}\n[stderr]: {}", stdout, stderr))
            }
        } else {
            Err(format!(
                "Shell command failed (exit {}): {}. \
                 You may try an alternative command or a different \
                 approach to accomplish this task.",
                output.status.code().unwrap_or(-1),
                stderr
            ))
        }
    }

    /// Execute a shell command on Windows using `std::process::Command`
    /// with non-inheritable pipes inside `spawn_blocking`.
    ///
    /// Non-inheritable pipes prevent grandchild processes (e.g., a browser
    /// spawned by Playwright CLI) from inheriting the stdout/stderr pipe
    /// write handles. Without this, `read_to_end()` would block indefinitely
    /// because those grandchild processes hold the write end open even
    /// after PowerShell has exited.
    ///
    /// `CREATE_NEW_PROCESS_GROUP` is used instead of `CREATE_NO_WINDOW` so
    /// that GUI applications (browsers, editors, etc.) can open their own
    /// windows while still isolating Ctrl+C / Ctrl+Break signal propagation.
    #[cfg(target_os = "windows")]
    async fn run_shell_command_windows(
        command: &str,
        timeout_secs: u64,
    ) -> Result<std::process::Output, String> {
        use std::io::Read;
        use std::os::windows::process::CommandExt;
        use std::process::Command as StdCommand;

        // CREATE_NEW_PROCESS_GROUP: creates a new process group so that
        // Ctrl+C / Ctrl+Break events don't propagate to the Ruri server.
        // Unlike CREATE_NO_WINDOW, this does NOT prevent GUI applications
        // (e.g., browsers launched by Playwright CLI) from opening their
        // own windows.
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

        let command = command.to_string();

        // Shared guard holder: the blocking thread writes the guard after
        // spawning the child, and the async timeout handler reads it to kill.
        let guard_holder = std::sync::Arc::new(std::sync::Mutex::new(
            None::<crate::agent::builtin_tools::ProcessGroupGuard>,
        ));
        let gh = guard_holder.clone();

        let blocking_task = tokio::task::spawn_blocking(move || {
            // Prepend UTF-8 output encoding fix so PowerShell outputs valid
            // UTF-8 instead of the system OEM code page (e.g., GBK).
            let cmd_with_enc = format!(
                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8; {command}"
            );
            // Encode command as UTF-16LE and base64-encode it, so PowerShell
            // passes it through without interpreting special characters.
            let encoded = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                cmd_with_enc
                    .encode_utf16()
                    .flat_map(|c| c.to_le_bytes())
                    .collect::<Vec<u8>>(),
            );

            // Create non-inheritable pipes for stdout and stderr.
            // This ensures that child processes spawned by PowerShell
            // (e.g., browsers from Playwright CLI) do NOT inherit the
            // pipe write handles, so read_to_end() returns as soon as
            // PowerShell itself exits.
            let (mut stdout_reader, stdout_writer) =
                crate::agent::builtin_tools::create_noninheritable_pipe()
                    .map_err(|e| format!("Failed to create stdout pipe: {}", e))?;
            let (mut stderr_reader, stderr_writer) =
                crate::agent::builtin_tools::create_noninheritable_pipe()
                    .map_err(|e| format!("Failed to create stderr pipe: {}", e))?;

            let mut child = StdCommand::new("powershell")
                .args(["-NoProfile", "-EncodedCommand", &encoded])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::from(stdout_writer))
                .stderr(std::process::Stdio::from(stderr_writer))
                .creation_flags(CREATE_NEW_PROCESS_GROUP)
                .spawn()
                .map_err(|e| {
                    format!(
                        "Failed to spawn shell command: {}. \
                         Please check the command syntax and try again, or use \
                         a different approach.",
                        e
                    )
                })?;

            // Create a Job Object guard that kills the entire process tree
            // on timeout. The guard is stored in the shared holder so the
            // async timeout path can trigger TerminateJobObject.
            let guard = crate::agent::builtin_tools::ProcessGroupGuard::new(child.id());
            *gh.lock().unwrap() = Some(guard);

            // Wait for the process to exit. If the timeout fires in the
            // async layer, guard.kill() will terminate the job, causing
            // this wait() to return.
            let status = child.wait().map_err(|e| {
                format!(
                    "Failed to execute shell command: {}. \
                     Please check the command syntax and try again, or use \
                     a different approach.",
                    e
                )
            })?;

            // Now read the pipes. Since the pipe handles are non-inheritable,
            // child processes of PowerShell cannot hold the write end open.
            // read_to_end() will return as soon as PowerShell exits.
            let mut stdout = Vec::new();
            stdout_reader
                .read_to_end(&mut stdout)
                .map_err(|e| format!("Failed to read stdout: {}", e))?;
            let mut stderr = Vec::new();
            stderr_reader
                .read_to_end(&mut stderr)
                .map_err(|e| format!("Failed to read stderr: {}", e))?;

            // Normal completion — disarm the guard so Drop doesn't double-kill.
            if let Some(ref mut guard) = *gh.lock().unwrap() {
                guard.disarm();
            }

            Ok(std::process::Output {
                status,
                stdout,
                stderr,
            })
        });

        match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), blocking_task)
            .await
        {
            Ok(join_result) => join_result.map_err(|e| format!("Blocking task panicked: {}", e))?,
            Err(_elapsed) => {
                // Timeout — kill the entire process tree.
                // This causes child.wait() in the blocking thread to return,
                // releasing the thread back to the spawn_blocking pool.
                let mut lock = guard_holder.lock().unwrap();
                if let Some(ref guard) = *lock {
                    guard.kill();
                }
                // Disarm to prevent Drop from double-killing.
                if let Some(ref mut guard) = *lock {
                    guard.disarm();
                }
                Err(format!(
                    "Shell command timed out after {} seconds. \
                     Consider using a different approach with a shorter \
                     execution time, or try breaking the task into \
                     smaller steps.",
                    timeout_secs
                ))
            }
        }
    }

    /// Execute a shell command on Unix (Linux/macOS) using tokio's
    /// async process management with process group support.
    #[cfg(not(target_os = "windows"))]
    async fn run_shell_command_unix(
        command: &str,
        timeout_secs: u64,
    ) -> Result<std::process::Output, String> {
        // Use spawn() + wait_with_output() instead of output() because
        // output() can cause deadlock issues with piped stdio.
        let child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .process_group(0)
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to spawn shell command: {}. \
                     Please check the command syntax and try again, or use \
                     a different approach.",
                    e
                )
            })?;

        let child_pid = child.id();
        let mut child = Some(child);

        // ProcessGroupGuard for timeout cleanup.
        let _guard = tokio::task::spawn_blocking(move || {
            crate::agent::builtin_tools::ProcessGroupGuard::new(child_pid.unwrap_or(0))
        })
        .await
        .unwrap_or_else(|_| crate::agent::builtin_tools::ProcessGroupGuard::new(0));

        tokio::select! {
            result = child.take().unwrap().wait_with_output() => {
                result.map_err(|e| {
                    format!(
                        "Failed to execute shell command: {}. \
                         Please check the command syntax and try again, or use \
                         a different approach.",
                        e
                    )
                })
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                // Timeout: kill the process group, then wait for cleanup.
                _guard.kill();
                let _ = child.take().unwrap().wait_with_output().await;
                Err(format!(
                    "Shell command timed out after {} seconds. \
                     Consider using a different approach with a shorter \
                     execution time, or try breaking the task into \
                     smaller steps.",
                    timeout_secs
                ))
            }
        }
    }

    /// Run all hooks and collect outputs.
    ///
    /// The `blacklist` is enforced for every hook command. If any hook
    /// command is blacklisted, its error is recorded and execution continues
    /// with the remaining hooks.
    pub async fn run_hooks(&self, blacklist: &[String]) -> Vec<String> {
        let mut outputs = Vec::new();
        for hook in &self.hooks {
            match Self::run_shell_command(&hook.command, blacklist).await {
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

    /// Execute the skill's shell command (if defined) and hooks, returning
    /// formatted output strings that can be appended to the skill context.
    /// This should be called ONLY when the skill is explicitly invoked,
    /// not during attach/initialization.
    ///
    /// The `blacklist` is enforced for the shell command and all hooks.
    /// If the shell command is blacklisted or fails, the error is included
    /// in the output so the AI agent can try alternative approaches.
    pub async fn execute_shell_and_hooks(&self, blacklist: &[String]) -> Vec<String> {
        let mut outputs = Vec::new();

        // Run hooks
        let hook_outputs = self.run_hooks(blacklist).await;
        if !hook_outputs.is_empty() {
            outputs.push(format!("## Hook Output\n{}", hook_outputs.join("\n\n")));
        }

        // Run the shell command if defined
        if let Some(ref shell_cmd) = self.shell {
            match Self::run_shell_command(shell_cmd, blacklist).await {
                Ok(output) => {
                    tracing::info!(
                        skill = %self.name,
                        output_len = output.len(),
                        "Skill shell command executed on invoke"
                    );
                    *self.shell_output.write().await = Some(output.clone());
                    outputs.push(format!("## Shell Command Output\n```\n{}\n```", output));
                }
                Err(e) => {
                    tracing::warn!(
                        skill = %self.name,
                        error = %e,
                        "Skill shell command failed on invoke"
                    );
                    outputs.push(format!("## Shell Command Error\n```\n{}\n```", e));
                }
            }
        }

        outputs
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
        // Build the skill prompt WITHOUT executing shell commands or hooks.
        // Shell and hooks are executed separately by callers (e.g.,
        // InvokeSkillTool or try_dispatch_skill_command) only when the
        // skill is explicitly invoked. This prevents shell commands from
        // running eagerly before the LLM has a chance to decide whether
        // the skill should be used.
        let prompt = self.build_system_prompt();
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
        // Skills without when_to_use are "always-on" — their context is
        // collected and injected into every user message by default.
        // Skills with when_to_use are "on-demand" — they are only loaded
        // when the model routes to them based on the when_to_use condition.
        self.when_to_use.is_none()
    }

    fn when_to_use(&self) -> Option<&str> {
        self.when_to_use.as_deref()
    }
}
