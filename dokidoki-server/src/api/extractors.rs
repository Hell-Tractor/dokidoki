use std::ops::Deref;

use axum::{
    Json,
    extract::{FromRequest, Request},
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::AppError;

/// 反序列化 JSON 请求体并执行 `validator` 校验。
///
/// ```ignore
/// async fn register(ValidatedJson(body): ValidatedJson<RegisterRequest>) -> ApiResult<_> {
///     let username = &body.username;
/// }
/// ```
pub struct ValidatedJson<T>(pub T);

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection)?;

        value
            .validate()
            .map_err(|err| AppError::bad_request(format_validation_errors(&err)))?;

        Ok(ValidatedJson(value))
    }
}

fn json_rejection(err: axum::extract::rejection::JsonRejection) -> AppError {
    if let axum::extract::rejection::JsonRejection::BytesRejection(bytes_rejection) = &err {
        if bytes_rejection.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE {
            return AppError::payload_too_large();
        }
    }
    AppError::bad_request(err.to_string())
}

fn format_validation_errors(err: &validator::ValidationErrors) -> String {
    err.field_errors()
        .iter()
        .flat_map(|(field, errors)| {
            errors.iter().map(move |e| {
                let msg = e
                    .message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| e.code.to_string());
                format!("{field}: {msg}")
            })
        })
        .collect::<Vec<_>>()
        .join("; ")
}
