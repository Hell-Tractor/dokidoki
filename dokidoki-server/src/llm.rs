mod backend;
pub mod fake;
pub mod http;

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
}

pub struct LlmClient {
    backend: Arc<dyn LlmBackend>,
    /// 与 `backend` 同指针；仅 fake 模式用于 dev queue。
    fake: Option<Arc<FakeLlmBackend>>,
}

impl LlmClient {
    pub fn from_config(config: &Llm) -> Self {
        match config.mode.as_str() {
            "http" => Self {
                backend: Arc::new(HttpLlmBackend::new(config)),
                fake: None,
            },
            _ => {
                let fake = Arc::new(FakeLlmBackend::new(config.fake_default.clone()));
                Self {
                    backend: fake.clone(),
                    fake: Some(fake),
                }
            }
        }
    }

    pub fn queue_responses(&self, responses: Vec<String>) {
        if let Some(fake) = &self.fake {
            fake.queue_responses(responses);
        }
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<String, AppError> {
        self.backend.chat(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Llm {
        Llm {
            mode: "fake".into(),
            fake_default: "[REPLY] default".into(),
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            vision_model: String::new(),
        }
    }

    #[tokio::test]
    async fn client_delegates_to_fake_backend() {
        let client = LlmClient::from_config(&test_config());
        client.queue_responses(vec!["[REPLY] first".into()]);
        let response = client
            .chat(ChatRequest {
                conversation_id: "c".into(),
                turn_id: "t".into(),
                messages: vec![],
            })
            .await
            .unwrap();
        assert_eq!(response, "[REPLY] first");
    }
}
