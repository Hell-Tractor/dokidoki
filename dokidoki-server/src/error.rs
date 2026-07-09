/// API 错误码，与《接口设计说明书》§1.4 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    BadRequest,
    InvalidToken,
    InvalidCredentials,
    UsernameTaken,
    NotFound,
    PayloadTooLarge,
    UnsupportedMedia,
    InternalError,
    LlmUnavailable,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BadRequest => "BAD_REQUEST",
            Self::InvalidToken => "INVALID_TOKEN",
            Self::InvalidCredentials => "INVALID_CREDENTIALS",
            Self::UsernameTaken => "USERNAME_TAKEN",
            Self::NotFound => "NOT_FOUND",
            Self::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            Self::UnsupportedMedia => "UNSUPPORTED_MEDIA",
            Self::InternalError => "INTERNAL_ERROR",
            Self::LlmUnavailable => "LLM_UNAVAILABLE",
        }
    }

    pub const fn default_message(self) -> &'static str {
        match self {
            Self::BadRequest => "请求参数无效",
            Self::InvalidToken => "Token 无效或已注销",
            Self::InvalidCredentials => "用户名或密码错误",
            Self::UsernameTaken => "用户名已存在",
            Self::NotFound => "资源不存在",
            Self::PayloadTooLarge => "上传内容过大",
            Self::UnsupportedMedia => "不支持的媒体类型",
            Self::InternalError => "服务端错误",
            Self::LlmUnavailable => "LLM 服务不可用",
        }
    }
}

/// HTTP handler 层错误；通过 `?` 传播，由 `api/response` 转为 JSON。
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn with_code(code: ErrorCode) -> Self {
        Self::new(code, code.default_message())
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, message)
    }

    pub fn invalid_token() -> Self {
        Self::with_code(ErrorCode::InvalidToken)
    }

    pub fn invalid_credentials() -> Self {
        Self::with_code(ErrorCode::InvalidCredentials)
    }

    pub fn username_taken() -> Self {
        Self::with_code(ErrorCode::UsernameTaken)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn payload_too_large() -> Self {
        Self::with_code(ErrorCode::PayloadTooLarge)
    }

    pub fn unsupported_media() -> Self {
        Self::with_code(ErrorCode::UnsupportedMedia)
    }

    pub fn llm_unavailable() -> Self {
        Self::with_code(ErrorCode::LlmUnavailable)
    }

    pub fn internal(err: impl std::error::Error) -> Self {
        tracing::error!("{err:?}");
        Self::with_code(ErrorCode::InternalError)
    }
}

/// 进程启动、配置加载等非 HTTP 错误。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

pub type Result<T> = std::result::Result<T, Error>;
