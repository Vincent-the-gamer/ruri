use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, broadcast};
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    Layer, layer::SubscriberExt, registry::LookupSpan, util::SubscriberInitExt,
};

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
    pub timestamp: u64,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

impl LogEntry {
    pub fn new(
        level: LogLevel,
        target: String,
        message: String,
        file: Option<String>,
        line: Option<u32>,
    ) -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            level,
            target,
            message,
            file,
            line,
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
        let (broadcast_tx, _) = broadcast::channel(100);
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

        // 获取文件和行号
        let file = metadata.file().map(|f| f.to_string());
        let line = metadata.line();

        // 访问事件的字段
        let mut visitor = LogVisitor::new();
        event.record(&mut visitor);
        let message = visitor.message;

        // 创建并存储日志条目
        let entry = LogEntry::new(level, target, message, file, line);
        self.log_manager.add_log(entry);
    }
}

/// 用于提取日志消息的Visitor
struct LogVisitor {
    message: String,
}

impl LogVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
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
