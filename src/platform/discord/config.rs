use serde::{Deserialize, Serialize};

/// Configuration for a Discord bot.
///
/// Example YAML:
/// ```yaml
/// platforms:
///   - type: discord
///     id: my-discord-bot
///     enable: true
///     token: "BOT_TOKEN_HERE"
///     pre_response_reactions: true  # optional: add a reaction while processing
///     reaction_emojis: ["👍", "🤔", "⏳"]  # optional: emojis for pre-response
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// The Discord bot token.
    pub token: String,
    /// Whether to add a pre-response reaction to indicate the bot is processing.
    #[serde(default)]
    pub pre_response_reactions: bool,
    /// List of Unicode emoji to use as pre-response reactions (randomly chosen).
    #[serde(default = "default_reaction_emojis")]
    pub reaction_emojis: Vec<String>,
}

fn default_reaction_emojis() -> Vec<String> {
    vec!["👍".to_string(), "🤔".to_string(), "⏳".to_string()]
}
