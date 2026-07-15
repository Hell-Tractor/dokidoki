use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::error::AppError;

use super::{backend::LlmBackend, ChatRequest};

pub struct FakeLlmBackend {
    queue: Mutex<VecDeque<String>>,
    default_response: String,
}

impl FakeLlmBackend {
    pub fn new(default_response: String) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            default_response,
        }
    }

    pub fn queue_responses(&self, responses: Vec<String>) {
        let mut queue = self.queue.lock().expect("fake llm queue lock");
        queue.extend(responses);
    }

    fn next_response(&self) -> String {
        let mut queue = self.queue.lock().expect("fake llm queue lock");
        queue
            .pop_front()
            .unwrap_or_else(|| self.default_response.clone())
    }
}

#[async_trait]
impl LlmBackend for FakeLlmBackend {
    async fn chat(&self, request: ChatRequest) -> Result<String, AppError> {
        let from_queue = {
            let queue = self.queue.lock().expect("fake llm queue lock");
            !queue.is_empty()
        };
        let response = self.next_response();
        tracing::debug!(
            conversation_id = %request.conversation_id,
            turn_id = %request.turn_id,
            from_queue,
            reply_chars = response.len(),
            "fake llm response"
        );
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn uses_queue_then_default() {
        let backend = FakeLlmBackend::new("[REPLY] default".into());
        backend.queue_responses(vec!["[REPLY] first".into()]);
        assert_eq!(
            backend.chat(ChatRequest {
                conversation_id: "c".into(),
                turn_id: "t".into(),
                messages: vec![],
            })
            .await
            .unwrap(),
            "[REPLY] first"
        );
        assert_eq!(
            backend.chat(ChatRequest {
                conversation_id: "c".into(),
                turn_id: "t".into(),
                messages: vec![],
            })
            .await
            .unwrap(),
            "[REPLY] default"
        );
    }
}
