use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub(crate) const CONTENT_TYPE_TEXT: &str = "text";
pub(crate) const CONTENT_TYPE_IMAGE: &str = "image";

/// 消息类型专用字段；存于 `messages.metadata` JSON 列。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// 媒体文件相对路径（如图片、语音）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Message {
    pub id: String,
    pub role: String,
    pub content: Option<String>,
    pub content_type: String,
    pub turn_id: Option<String>,
    pub seq_in_turn: i32,
    pub(crate) metadata: Option<sqlx::types::Json<MessageMetadata>>,
    pub reply_to_id: Option<String>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// 读取类型专用 metadata（crate 内部 helper 用）。
    fn metadata(&self) -> Option<&MessageMetadata> {
        self.metadata.as_deref()
    }

    pub fn media_url(&self, suffix: &str) -> Option<String> {
        self.metadata()
            .and_then(|meta| meta.path.as_ref())
            .map(|_| format!("/api/v1/messages/{}/{}", self.id, suffix))
    }
}
