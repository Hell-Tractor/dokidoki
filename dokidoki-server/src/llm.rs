mod backend;
pub mod fake;
pub mod http;
pub mod schema;

use std::sync::Arc;

use crate::{config::Llm, error::AppError};

use backend::LlmBackend;
use fake::FakeLlmBackend;
use http::HttpLlmBackend;

#[derive(Debug, Clone)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub conversation_id: String,
    pub turn_id: String,
    pub messages: Vec<LlmMessage>,
    /// `off` | `json_object` | `json_schema`
    pub response_format: String,
}

impl ChatRequest {
    pub fn new(
        conversation_id: impl Into<String>,
        turn_id: impl Into<String>,
        messages: Vec<LlmMessage>,
    ) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            turn_id: turn_id.into(),
            messages,
            response_format: String::new(),
        }
    }
}

pub struct LlmClient {
    backend: Arc<dyn LlmBackend>,
    /// 与 `backend` 同指针；仅 fake 模式用于 dev queue。
    fake: Option<Arc<FakeLlmBackend>>,
    response_format: String,
}

impl LlmClient {
    pub fn from_config(config: &Llm) -> Self {
        let response_format = config.response_format.clone();
        match config.mode.as_str() {
            "http" => Self {
                backend: Arc::new(HttpLlmBackend::new(config)),
                fake: None,
                response_format,
            },
            _ => {
                let fake = Arc::new(FakeLlmBackend::new(config.fake_default.clone()));
                Self {
                    backend: fake.clone(),
                    fake: Some(fake),
                    response_format,
                }
            }
        }
    }

    pub fn queue_responses(&self, responses: Vec<String>) {
        if let Some(fake) = &self.fake {
            fake.queue_responses(responses);
        }
    }

    pub async fn chat(&self, mut request: ChatRequest) -> Result<String, AppError> {
        if request.response_format.is_empty() {
            request.response_format = self.response_format.clone();
        }
        let conversation_id = request.conversation_id.clone();
        let turn_id = request.turn_id.clone();
        let message_count = request.messages.len();
        let prompt_chars: usize = request
            .messages
            .iter()
            .map(|message| message.content.len())
            .sum();

        tracing::debug!(
            conversation_id = %conversation_id,
            turn_id = %turn_id,
            message_count,
            prompt_chars,
            response_format = %request.response_format,
            "llm request"
        );
        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(
                conversation_id = %conversation_id,
                turn_id = %turn_id,
                prompt = %format_messages_for_trace(&request.messages),
                "llm request body"
            );
        }

        let started = std::time::Instant::now();
        let result = self.backend.chat(request).await;
        let elapsed_ms = started.elapsed().as_millis();

        match &result {
            Ok(raw) => {
                tracing::debug!(
                    conversation_id = %conversation_id,
                    turn_id = %turn_id,
                    elapsed_ms,
                    reply_chars = raw.len(),
                    "llm response ok"
                );
                tracing::trace!(
                    conversation_id = %conversation_id,
                    turn_id = %turn_id,
                    reply = %raw,
                    "llm response body"
                );
            }
            Err(err) => {
                tracing::warn!(
                    conversation_id = %conversation_id,
                    turn_id = %turn_id,
                    elapsed_ms,
                    "llm response error: {err}"
                );
            }
        }

        result
    }
}

fn format_messages_for_trace(messages: &[LlmMessage]) -> String {
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            format!(
                "----- [{index}] role={} chars={} -----\n{}",
                message.role,
                message.content.len(),
                message.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Llm {
        Llm {
            mode: "fake".into(),
            fake_default: r#"{"action":{"type":"reply","bubbles":["default"]}}"#.into(),
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            vision_model: String::new(),
            response_format: "json_object".into(),
        }
    }

    #[tokio::test]
    async fn client_delegates_to_fake_backend() {
        let client = LlmClient::from_config(&test_config());
        client.queue_responses(vec![
            r#"{"action":{"type":"reply","bubbles":["first"]}}"#.into(),
        ]);
        let response = client
            .chat(ChatRequest::new("c", "t", vec![]))
            .await
            .unwrap();
        assert!(response.contains("first"));
    }
}
