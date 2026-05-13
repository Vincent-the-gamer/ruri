//! WeChat iLink protocol types.
//!
//! These types mirror the JSON structures used by the WeChat ClawBot API.
//! See: https://github.com/Tencent/openclaw-weixin

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Message type constants
// ---------------------------------------------------------------------------

/// Message item content type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageItemType {
    None = 0,
    Text = 1,
    Image = 2,
    Voice = 3,
    File = 4,
    Video = 5,
}

// ---------------------------------------------------------------------------
// CDN media reference
// ---------------------------------------------------------------------------

/// CDN media reference with AES-128-ECB encryption info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CDNMedia {
    /// Encrypted parameters for CDN download/upload.
    #[serde(default)]
    pub encrypt_query_param: Option<String>,
    /// Base64-encoded AES-128 key.
    #[serde(default)]
    pub aes_key: Option<String>,
    /// Encryption type: 0 = only fileid, 1 = includes thumbnail etc.
    #[serde(default)]
    pub encrypt_type: Option<u32>,
    /// Full download URL (returned by server, no need to construct).
    #[serde(default)]
    pub full_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Message item sub-types
// ---------------------------------------------------------------------------

/// Text content within a message item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TextItem {
    #[serde(default)]
    pub text: Option<String>,
}

/// Image content within a message item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImageItem {
    /// Original image CDN reference.
    #[serde(default)]
    pub media: Option<CDNMedia>,
    /// Thumbnail CDN reference.
    #[serde(default)]
    pub thumb_media: Option<CDNMedia>,
    /// Raw AES-128 key as hex string (16 bytes), preferred over media.aes_key.
    #[serde(default)]
    pub aeskey: Option<String>,
    /// Image URL.
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub mid_size: Option<u64>,
    #[serde(default)]
    pub thumb_size: Option<u64>,
    #[serde(default)]
    pub thumb_height: Option<u32>,
    #[serde(default)]
    pub thumb_width: Option<u32>,
    #[serde(default)]
    pub hd_size: Option<u64>,
}

/// Voice content within a message item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VoiceItem {
    #[serde(default)]
    pub media: Option<CDNMedia>,
    /// Encoding type: 1=pcm, 2=adpcm, 3=feature, 4=speex, 5=amr, 6=silk, 7=mp3, 8=ogg-speex.
    #[serde(default)]
    pub encode_type: Option<u32>,
    #[serde(default)]
    pub bits_per_sample: Option<u32>,
    /// Sample rate in Hz.
    #[serde(default)]
    pub sample_rate: Option<u32>,
    /// Play duration in milliseconds.
    #[serde(default)]
    pub playtime: Option<u64>,
    /// Voice-to-text transcription.
    #[serde(default)]
    pub text: Option<String>,
}

/// File content within a message item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FileItem {
    #[serde(default)]
    pub media: Option<CDNMedia>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub len: Option<String>,
}

/// Video content within a message item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VideoItem {
    #[serde(default)]
    pub media: Option<CDNMedia>,
    #[serde(default)]
    pub video_size: Option<u64>,
    #[serde(default)]
    pub play_length: Option<u64>,
    #[serde(default)]
    pub video_md5: Option<String>,
    #[serde(default)]
    pub thumb_media: Option<CDNMedia>,
    #[serde(default)]
    pub thumb_size: Option<u64>,
    #[serde(default)]
    pub thumb_height: Option<u32>,
    #[serde(default)]
    pub thumb_width: Option<u32>,
}

/// Referenced (quoted) message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RefMessage {
    #[serde(default)]
    pub message_item: Option<Box<MessageItem>>,
    /// Quoted message summary.
    #[serde(default)]
    pub title: Option<String>,
}

/// A single content item within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MessageItem {
    /// Content type (see MessageItemType).
    #[serde(default)]
    #[serde(rename = "type")]
    pub item_type: Option<u32>,
    #[serde(default)]
    pub create_time_ms: Option<u64>,
    #[serde(default)]
    pub update_time_ms: Option<u64>,
    #[serde(default)]
    pub is_completed: Option<bool>,
    #[serde(default)]
    pub msg_id: Option<String>,
    #[serde(default)]
    pub ref_msg: Option<Box<RefMessage>>,
    #[serde(default)]
    pub text_item: Option<TextItem>,
    #[serde(default)]
    pub image_item: Option<ImageItem>,
    #[serde(default)]
    pub voice_item: Option<VoiceItem>,
    #[serde(default)]
    pub file_item: Option<FileItem>,
    #[serde(default)]
    pub video_item: Option<VideoItem>,
}

// ---------------------------------------------------------------------------
// Unified message
// ---------------------------------------------------------------------------

/// A unified WeChat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WeixinMessage {
    #[serde(default)]
    pub seq: Option<u64>,
    #[serde(default)]
    pub message_id: Option<u64>,
    #[serde(default)]
    pub from_user_id: Option<String>,
    #[serde(default)]
    pub to_user_id: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub create_time_ms: Option<u64>,
    #[serde(default)]
    pub update_time_ms: Option<u64>,
    #[serde(default)]
    pub delete_time_ms: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    /// 1=USER, 2=BOT.
    #[serde(default)]
    pub message_type: Option<u32>,
    /// 0=NEW, 1=GENERATING, 2=FINISH.
    #[serde(default)]
    pub message_state: Option<u32>,
    #[serde(default)]
    pub item_list: Option<Vec<MessageItem>>,
    /// Context token for replying in the same conversation.
    #[serde(default)]
    pub context_token: Option<String>,
}

// ---------------------------------------------------------------------------
// API request/response types
// ---------------------------------------------------------------------------

/// Base info attached to every API request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BaseInfo {
    #[serde(default)]
    pub channel_version: Option<String>,
}

/// getUpdates response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GetUpdatesResp {
    #[serde(default)]
    pub ret: Option<i64>,
    #[serde(default)]
    pub errcode: Option<i64>,
    #[serde(default)]
    pub errmsg: Option<String>,
    #[serde(default)]
    pub msgs: Option<Vec<WeixinMessage>>,
    #[serde(default)]
    pub get_updates_buf: Option<String>,
    /// Server-suggested long-poll timeout for next request (ms).
    #[serde(default)]
    pub longpolling_timeout_ms: Option<u64>,
}

/// sendMessage request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SendMessageReq {
    pub msg: WeixinMessageSend,
    #[serde(default)]
    pub base_info: Option<BaseInfo>,
}

/// The `msg` field in a sendMessage request.
///
/// Mirrors the WeixinMessage structure used by the official openclaw-weixin plugin.
/// Required fields: `to_user_id`, `client_id`, `message_type` (2=BOT),
/// `message_state` (2=FINISH), `item_list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WeixinMessageSend {
    /// Bot's user ID (can be empty).
    #[serde(default)]
    pub from_user_id: String,
    /// Target user ID.
    pub to_user_id: String,
    /// Unique client-generated message ID.
    pub client_id: String,
    /// Message type: 2 = BOT.
    pub message_type: u32,
    /// Message state: 2 = FINISH.
    pub message_state: u32,
    /// Message content items.
    pub item_list: Vec<SendMessageItem>,
    /// Context token for replying in the same conversation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_token: Option<String>,
}

/// A single item in a sendMessage request's item_list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SendMessageItem {
    #[serde(rename = "type")]
    pub item_type: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_item: Option<TextItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_item: Option<ImageItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_item: Option<FileItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_item: Option<VideoItem>,
}

impl SendMessageItem {
    /// Create a text message item.
    pub fn text(text: &str) -> Self {
        Self {
            item_type: MessageItemType::Text as u32,
            text_item: Some(TextItem {
                text: Some(text.to_string()),
            }),
            image_item: None,
            file_item: None,
            video_item: None,
        }
    }
}

// ---------------------------------------------------------------------------
// getConfig / sendTyping types
// ---------------------------------------------------------------------------

/// getConfig response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GetConfigResp {
    #[serde(default)]
    pub ret: Option<i64>,
    #[serde(default)]
    pub errmsg: Option<String>,
    /// Base64-encoded typing ticket for sendTyping.
    #[serde(default)]
    pub typing_ticket: Option<String>,
}

// ---------------------------------------------------------------------------
// QR Login types
// ---------------------------------------------------------------------------

/// QR code response from get_bot_qrcode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QrCodeResponse {
    pub qrcode: String,
    pub qrcode_img_content: String,
}

/// QR code status response from get_qrcode_status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QrStatusResponse {
    /// "wait" | "scaned" | "confirmed" | "expired" | "scaned_but_redirect"
    pub status: String,
    #[serde(default)]
    pub bot_token: Option<String>,
    #[serde(default)]
    pub ilink_bot_id: Option<String>,
    #[serde(default)]
    pub baseurl: Option<String>,
    #[serde(default)]
    pub ilink_user_id: Option<String>,
    #[serde(default)]
    pub redirect_host: Option<String>,
}
