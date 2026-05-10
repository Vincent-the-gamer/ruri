use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, broadcast};
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    Layer, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt,
};

/// Global atomic counter for generating unique log entry IDs.
static LOG_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 日志级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<tracing::Level> for LogLevel {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unique auto-incrementing ID for deduplication on the frontend.
    pub id: u64,
    pub timestamp: u64,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Structured fields from the tracing event (excludes `message` which is a top-level field).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

impl LogEntry {
    pub fn new(
        level: LogLevel,
        target: String,
        message: String,
        module_path: Option<String>,
        file: Option<String>,
        line: Option<u32>,
        fields: BTreeMap<String, String>,
    ) -> Self {
        Self {
            id: LOG_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            level,
            target,
            message,
            module_path,
            file,
            line,
            fields,
        }
    }
}

/// 日志管理器
pub struct LogManager {
    /// 最大保存的日志条数
    max_logs: usize,
    /// 日志存储
    logs: Arc<RwLock<Vec<LogEntry>>>,
    /// 日志广播通道
    broadcast_tx: broadcast::Sender<LogEntry>,
}

impl LogManager {
    /// 创建新的日志管理器
    pub fn new(max_logs: usize) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            max_logs,
            logs: Arc::new(RwLock::new(Vec::new())),
            broadcast_tx,
        }
    }

    /// 添加日志条目
    pub fn add_log(&self, entry: LogEntry) {
        let logs = self.logs.clone();
        let tx = self.broadcast_tx.clone();
        let max_logs = self.max_logs;

        // 异步存储日志
        tokio::spawn(async move {
            let mut logs_guard = logs.write().await;

            // 添加新日志
            logs_guard.push(entry.clone());

            // 限制日志数量
            if logs_guard.len() > max_logs {
                let to_remove = logs_guard.len() - max_logs;
                logs_guard.drain(0..to_remove);
            }
            drop(logs_guard);

            // 广播新日志
            let _ = tx.send(entry);
        });
    }

    /// 获取所有日志
    pub async fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.read().await.clone()
    }

    /// 清空日志
    pub async fn clear_logs(&self) {
        self.logs.write().await.clear();
    }

    /// 订阅日志广播
    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.broadcast_tx.subscribe()
    }

    /// Get the timestamp of the latest log entry (for gap detection on reconnect).
    pub async fn latest_timestamp(&self) -> Option<u64> {
        let logs = self.logs.read().await;
        logs.last().map(|entry| entry.timestamp)
    }

    /// Get all log entries with a timestamp strictly greater than `since`.
    pub async fn get_logs_since(&self, since: u64) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|entry| entry.timestamp > since)
            .cloned()
            .collect()
    }
}

/// 自定义日志层
pub struct LogLayer {
    log_manager: Arc<LogManager>,
}

impl LogLayer {
    pub fn new(log_manager: Arc<LogManager>) -> Self {
        Self { log_manager }
    }
}

impl<S> Layer<S> for LogLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();

        // 获取日志级别
        let level = LogLevel::from(*metadata.level());

        // 获取目标模块
        let target = metadata.target().to_string();

        // 获取模块路径
        let module_path = metadata.module_path().map(|m| m.to_string());

        // 获取文件和行号
        let file = metadata.file().map(|f| f.to_string());
        let line = metadata.line();

        // 访问事件的字段
        let mut visitor = LogVisitor::new();
        event.record(&mut visitor);
        let message = visitor.message;
        let fields = visitor.fields;

        // 创建并存储日志条目
        let entry = LogEntry::new(level, target, message, module_path, file, line, fields);
        self.log_manager.add_log(entry);
    }
}

/// Strip surrounding double-quotes from a `Debug`-formatted string when the
/// entire output is a single quoted string literal. This produces cleaner
/// message text for typical `tracing` log messages.
fn strip_debug_quotes(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        // Check that there are no unescaped quotes inside (i.e. it's a simple string literal).
        let inner = &s[1..s.len() - 1];
        let mut chars = inner.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                // Skip the escaped character.
                chars.next();
            } else if c == '"' {
                // Unescaped interior quote — not a simple string literal.
                return s.to_string();
            }
        }
        // It's a simple string literal — return the unescaped content.
        // Unescape simple escape sequences.
        let mut result = String::with_capacity(inner.len());
        let mut inner_chars = inner.chars().peekable();
        while let Some(c) = inner_chars.next() {
            if c == '\\' {
                if let Some(next) = inner_chars.next() {
                    match next {
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        _ => {
                            result.push('\\');
                            result.push(next);
                        }
                    }
                } else {
                    result.push('\\');
                }
            } else {
                result.push(c);
            }
        }
        result
    } else {
        s.to_string()
    }
}

/// 用于提取日志消息及结构化字段的 Visitor
struct LogVisitor {
    message: String,
    fields: BTreeMap<String, String>,
}

impl LogVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
            fields: BTreeMap::new(),
        }
    }
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            // Format with Debug, then strip surrounding quotes if the result looks like
            // a plain string — this gives cleaner output for typical log messages.
            let formatted = format!("{:?}", value);
            self.message = strip_debug_quotes(&formatted);
        } else {
            // Store all other fields with Debug formatting.
            self.fields
                .insert(field.name().to_string(), format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        let formatted = format!("{}", value);
        if field.name() == "message" {
            self.message = formatted;
        } else {
            self.fields
                .insert(field.name().to_string(), format!("{}", value));
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), value.to_string());
    }
}

/// 初始化日志系统，返回LogManager实例
pub fn init_logging(max_logs: usize) -> Arc<LogManager> {
    let log_manager = Arc::new(LogManager::new(max_logs));

    // 创建日志层
    let log_layer = LogLayer::new(log_manager.clone());

    // 初始化 tracing subscriber
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .with(log_layer)
        .init();

    log_manager
}
