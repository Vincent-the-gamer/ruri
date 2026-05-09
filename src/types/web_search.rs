use serde::{Deserialize, Serialize};

/// Supported web search engines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngine {
    /// DuckDuckGo (free, no API key required)
    DuckDuckGo,
    /// Tavily Search API
    Tavily,
    /// BoCha Search API
    BoCha,
    /// Baidu AI Search
    Baidu,
    /// Brave Search API
    Brave,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::DuckDuckGo
    }
}

/// Configuration for web search functionality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    /// The search engine to use.
    #[serde(default)]
    pub search_engine: SearchEngine,

    /// API key for the search engine (required for Tavily, BoCha, Baidu, Brave).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Maximum number of search results to return.
    #[serde(default = "default_max_results")]
    pub max_results: usize,

    /// Whether web search is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_max_results() -> usize {
    10
}

fn default_enabled() -> bool {
    true
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            search_engine: SearchEngine::default(),
            api_key: None,
            max_results: default_max_results(),
            enabled: default_enabled(),
        }
    }
}
