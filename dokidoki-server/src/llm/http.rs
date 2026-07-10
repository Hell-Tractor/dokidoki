use async_trait::async_trait;

use crate::error::AppError;

use super::{backend::LlmBackend, ChatRequest};

pub struct HttpLlmBackend {
}

impl HttpLlmBackend {
    pub fn new() -> Self {
        todo!()
    }
}

#[async_trait]
impl LlmBackend for HttpLlmBackend {
    async fn chat(&self, _request: ChatRequest) -> Result<String, AppError> {
        Err(AppError::llm_unavailable())
    }
}
