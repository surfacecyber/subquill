use crate::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("{message}")]
    Auth { message: String },

    #[error("{message}")]
    RateLimited { message: String },

    #[error("{message}")]
    Timeout { message: String },

    #[error("{message}")]
    Network { message: String },

    #[error("{message}")]
    Api { message: String },

    #[error("{message}")]
    InvalidResponse { message: String },
}

impl LlmError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Auth { .. } => "LLM_AUTH_ERROR",
            Self::RateLimited { .. } => "LLM_RATE_LIMITED",
            Self::Timeout { .. } => "LLM_TIMEOUT",
            Self::Network { .. } => "LLM_NETWORK_ERROR",
            Self::Api { .. } => "LLM_API_ERROR",
            Self::InvalidResponse { .. } => "LLM_INVALID_RESPONSE",
        }
    }

    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth {
            message: message.into(),
        }
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::RateLimited {
            message: message.into(),
        }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout {
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    pub fn api(message: impl Into<String>) -> Self {
        Self::Api {
            message: message.into(),
        }
    }

    pub fn invalid_response(message: impl Into<String>) -> Self {
        Self::InvalidResponse {
            message: message.into(),
        }
    }
}

impl From<LlmError> for Error {
    fn from(value: LlmError) -> Self {
        Self::App {
            code: value.code(),
            message: value.to_string(),
        }
    }
}

impl From<LlmError> for crate::error::ErrorPayload {
    fn from(value: LlmError) -> Self {
        crate::error::ErrorPayload::from(Error::from(value))
    }
}

pub type Result<T> = std::result::Result<T, LlmError>;
