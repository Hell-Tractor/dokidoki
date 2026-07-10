use async_trait::async_trait;

use crate::error::AppError;

use super::ChatRequest;

#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<String, AppError>;
}
