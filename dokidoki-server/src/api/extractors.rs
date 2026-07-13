use std::ops::Deref;

use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Query, Request},
    http::request::Parts,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::{db::models::User, error::AppError};

/// 由 `require_auth` 中间件注入，供 `AuthUser` extractor 读取。
#[derive(Clone)]
pub(crate) struct AuthContext {
    pub user: User,
}

/// 当前已鉴权用户；须挂在受 `require_auth` 保护的路由上。
pub struct AuthUser(pub User);

impl Deref for AuthUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .map(|ctx| AuthUser(ctx.user.clone()))
            .ok_or_else(AppError::invalid_token)
    }
}

/// 反序列化 JSON 请求体并执行 `validator` 校验。见 `ValidatedQuery`（query 参数）。
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

/// 反序列化 query 参数并执行 `validator` 校验。
pub struct ValidatedQuery<T>(pub T);

impl<T> Deref for ValidatedQuery<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|err| AppError::bad_request(err.to_string()))?;

        value
            .validate()
            .map_err(|err| AppError::bad_request(format_validation_errors(&err)))?;

        Ok(ValidatedQuery(value))
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
