use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{code}: {message}")]
    App { code: &'static str, message: String },

    #[error("storage error")]
    Io(#[from] std::io::Error),

    #[error("serialization error")]
    Serde(#[from] serde_json::Error),
}

impl Error {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::App {
            code: "VALIDATION_ERROR",
            message: message.into(),
        }
    }

    pub fn storage(message: impl Into<String>) -> Self {
        Self::App {
            code: "STORAGE_ERROR",
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::App { code, .. } => code,
            Self::Io(_) => "STORAGE_ERROR",
            Self::Serde(_) => "SERIALIZATION_ERROR",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::App { message, .. } => message.clone(),
            Self::Io(_) => "Failed to read or write local configuration".to_string(),
            Self::Serde(_) => "Failed to parse local configuration".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub code: &'static str,
    pub message: String,
}

impl From<Error> for ErrorPayload {
    fn from(value: Error) -> Self {
        Self {
            code: value.code(),
            message: value.message(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
