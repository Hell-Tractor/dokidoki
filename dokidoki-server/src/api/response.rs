use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::error::{AppError, ErrorCode};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip)]
    status: StatusCode,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            data,
            status: StatusCode::OK,
        }
    }

    pub fn created(data: T) -> Self {
        Self {
            data,
            status: StatusCode::CREATED,
        }
    }

    pub fn accepted(data: T) -> Self {
        Self {
            data,
            status: StatusCode::ACCEPTED,
        }
    }

    // pub fn with_status(status: StatusCode, data: T) -> Self {
    //     Self { data, status }
    // }

    // pub fn status(&self) -> StatusCode {
    //     self.status
    // }
}

impl ApiResponse<&'static str> {
    /// 无业务 body 的成功响应：`{ "data": "ok" }`
    pub fn ok_empty() -> Self {
        Self::ok("ok")
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status = self.status;
        (status, Json(self)).into_response()
    }
}

#[derive(Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,
}

#[derive(Serialize)]
struct ApiErrorEnvelope {
    error: ErrorBody,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = error_code_status(self.code());
        match self.code() {
            ErrorCode::InternalError | ErrorCode::LlmUnavailable => {
                tracing::error!(
                    code = self.code().as_str(),
                    message = %self.message(),
                    status = %status,
                    "api error response"
                );
            }
            ErrorCode::InvalidCredentials | ErrorCode::InvalidToken | ErrorCode::UsernameTaken => {
                tracing::warn!(
                    code = self.code().as_str(),
                    message = %self.message(),
                    status = %status,
                    "api error response"
                );
            }
            ErrorCode::BadRequest
            | ErrorCode::NotFound
            | ErrorCode::PayloadTooLarge
            | ErrorCode::UnsupportedMedia => {
                tracing::debug!(
                    code = self.code().as_str(),
                    message = %self.message(),
                    status = %status,
                    "api error response"
                );
            }
        }
        let body = Json(ApiErrorEnvelope {
            error: ErrorBody {
                code: self.code().as_str(),
                message: self.message().to_owned(),
            },
        });
        (status, body).into_response()
    }
}

fn error_code_status(code: ErrorCode) -> StatusCode {
    match code {
        ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
        ErrorCode::InvalidToken | ErrorCode::InvalidCredentials => StatusCode::UNAUTHORIZED,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::UsernameTaken => StatusCode::CONFLICT,
        ErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
        ErrorCode::UnsupportedMedia => StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ErrorCode::LlmUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        ErrorCode::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// handler 统一返回 `ApiResult<T>`（Axum 自带 `Result` 的 `IntoResponse`）。
pub type ApiResult<T> = Result<ApiResponse<T>, AppError>;
