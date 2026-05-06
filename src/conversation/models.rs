use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 聊天类型：群聊或私聊
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "chat_type", rename_all = "lowercase")]
pub enum ChatType {
    #[serde(rename = "group")]
    #[sqlx(rename = "group")]
    Group,
    #[serde(rename = "private")]
    #[sqlx(rename = "private")]
    Private,
}

/// 消息模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// 会话（对话）模型
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Conversation {
    pub id: String,
    pub bot_name: String,      // 机器人名称（配置文件名）
    pub chat_type: ChatType,   // 聊天类型（群聊或私聊）
    pub chat_id: String,       // 群ID或聊天对象ID
    pub title: Option<String>, // 对话标题
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 创建新会话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub bot_name: String,
    pub chat_type: ChatType,
    pub chat_id: String,
    pub title: Option<String>,
}

/// 添加消息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMessageRequest {
    pub conversation_id: String,
    pub role: String,
    pub content: String,
}

/// 查询会话的过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationFilter {
    pub bot_name: Option<String>,
    pub chat_type: Option<ChatType>,
    pub keyword: Option<String>,
}
