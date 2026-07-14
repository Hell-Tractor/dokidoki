use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{config::Llm, error::AppError};

use super::{backend::LlmBackend, ChatRequest, LlmMessage};

pub struct HttpLlmBackend {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl HttpLlmBackend {
    pub fn new(config: &Llm) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("reqwest client");

        Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_owned(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ApiMessage>,
}

#[derive(Serialize)]
struct ApiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[async_trait]
impl LlmBackend for HttpLlmBackend {
    async fn chat(&self, request: ChatRequest) -> Result<String, AppError> {
        if self.api_key.is_empty() || self.model.is_empty() || self.base_url.is_empty() {
            tracing::warn!(
                conversation_id = %request.conversation_id,
                "llm http misconfigured: empty api_key/model/base_url"
            );
            return Err(AppError::llm_unavailable());
        }

        let body = ChatCompletionRequest {
            model: self.model.clone(),
            messages: request
                .messages
                .into_iter()
                .map(|LlmMessage { role, content }| ApiMessage { role, content })
                .collect(),
        };

        let response = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                tracing::error!(conversation_id = %request.conversation_id, "llm http request failed: {err}");
                AppError::llm_unavailable()
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!(
                conversation_id = %request.conversation_id,
                status = %status,
                body = %body,
                "llm http error response"
            );
            return Err(AppError::llm_unavailable());
        }

        let payload: ChatCompletionResponse = response.json().await.map_err(|err| {
            tracing::error!(conversation_id = %request.conversation_id, "llm http decode failed: {err}");
            AppError::llm_unavailable()
        })?;

        payload
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| {
                tracing::error!(conversation_id = %request.conversation_id, "llm http empty choices");
                AppError::llm_unavailable()
            })
    }
}
