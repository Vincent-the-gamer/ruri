//! OneBot v12 standard data types.
//!
//! This module defines the core data structures used in the OneBot v12
//! protocol, including events, actions, message segments, and response
//! codes.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 1. Self (Robot Identity)
// ---------------------------------------------------------------------------

/// 机器人自身标识
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ob12Self {
    /// 机器人平台名称，如 "qq", "telegram", "discord"
    pub platform: String,
    /// 机器人用户 ID
    pub user_id: String,
}

// ---------------------------------------------------------------------------
// 2. Message Segments (消息段)
// ---------------------------------------------------------------------------

/// OneBot v12 消息段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// 消息段名称
    #[serde(rename = "type")]
    pub segment_type: String,
    /// 消息段参数
    pub data: serde_json::Map<String, serde_json::Value>,
}

impl Segment {
    /// Create a segment from a type name and an optional data map.
    pub fn new(segment_type: &str, data: serde_json::Map<String, serde_json::Value>) -> Self {
        Self {
            segment_type: segment_type.to_owned(),
            data,
        }
    }

    /// Text segment (纯文本)
    pub fn text(text: &str) -> Self {
        let mut data = serde_json::Map::new();
        data.insert(
            "text".to_owned(),
            serde_json::Value::String(text.to_owned()),
        );
        Self::new("text", data)
    }

    /// Image segment (图片)
    pub fn image(file_id: &str) -> Self {
        let mut data = serde_json::Map::new();
        data.insert(
            "file_id".to_owned(),
            serde_json::Value::String(file_id.to_owned()),
        );
        Self::new("image", data)
    }
}

// ---------------------------------------------------------------------------
// 3. Event (事件)
// ---------------------------------------------------------------------------

/// OneBot v12 事件
///
/// Non-meta events must include the `self` field identifying the bot.
/// All extra fields not in the fixed schema are captured in [`extra`](Ob12Event::extra).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ob12Event {
    /// Event unique ID
    pub id: String,
    /// Event timestamp (Unix seconds, may be fractional)
    pub time: f64,
    /// Event type: "meta", "message", "notice", "request"
    #[serde(rename = "type")]
    pub event_type: String,
    /// Detailed event type (e.g. "private", "group", "friend_add")
    pub detail_type: String,
    /// Event sub-type
    #[serde(default)]
    pub sub_type: String,
    /// The bot identity — required for non-meta events.
    /// Serialized as `"self"` in JSON.
    #[serde(default, rename = "self", skip_serializing_if = "Option::is_none")]
    pub self_: Option<Ob12Self>,
    /// All other fields not covered above
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// 4. Action Request (动作请求)
// ---------------------------------------------------------------------------

/// OneBot v12 动作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Action name, e.g. "send_message"
    pub action: String,
    /// Action parameters
    pub params: serde_json::Map<String, serde_json::Value>,
    /// Echo key — when present, the response must carry the same value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<String>,
}

// ---------------------------------------------------------------------------
// 5. Action Response (动作响应)
// ---------------------------------------------------------------------------

/// OneBot v12 动作响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    /// "ok" or "failed"
    pub status: String,
    /// Return code
    pub retcode: i64,
    /// Response data
    pub data: serde_json::Value,
    /// Human-readable message
    pub message: String,
    /// Echo key mirrored from the request
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<String>,
}

impl ActionResponse {
    /// Build a successful response.
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            status: "ok".to_owned(),
            retcode: retcode::OK,
            data,
            message: String::new(),
            echo: None,
        }
    }

    /// Build a failure response.
    pub fn failed(retcode: i64, message: String) -> Self {
        Self {
            status: "failed".to_owned(),
            retcode,
            data: serde_json::Value::Null,
            message,
            echo: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Return codes
// ---------------------------------------------------------------------------

/// OneBot v12 return code constants.
pub mod retcode {
    /// 成功
    pub const OK: i64 = 0;

    // 1xxxx Request errors
    /// 请求体语法错误
    pub const BAD_REQUEST: i64 = 10001;
    /// 不支持的动作
    pub const UNSUPPORTED_ACTION: i64 = 10002;
    /// 请求参数错误
    pub const BAD_PARAM: i64 = 10003;
    /// 不支持的参数
    pub const UNSUPPORTED_PARAM: i64 = 10004;
    // 2xxxx Handler errors
    /// 请求处理器内部错误
    pub const INTERNAL_HANDLER_ERROR: i64 = 20002;
}

// ---------------------------------------------------------------------------
// 7. Flexible message type
// ---------------------------------------------------------------------------

/// Flexible message type that accepts a plain string, a single segment,
/// or an array of segments — matching the OneBot v12 specification where
/// `message` can appear in any of these forms.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Ob12Message {
    /// Plain text — treated as a single text segment.
    Text(String),
    /// A single message segment.
    Segment(Segment),
    /// An array of message segments.
    Segments(Vec<Segment>),
}

impl Ob12Message {
    /// Convert to a `Vec<Segment>`, expanding plain strings into text segments.
    pub fn to_segments(&self) -> Vec<Segment> {
        match self {
            Ob12Message::Text(t) => vec![Segment::text(t)],
            Ob12Message::Segment(s) => vec![s.clone()],
            Ob12Message::Segments(v) => v.clone(),
        }
    }
}
