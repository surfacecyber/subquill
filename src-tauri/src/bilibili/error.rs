use crate::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum BilibiliError {
    #[error("{message}")]
    InvalidUrl { message: String },

    #[error("{message}")]
    ShortLinkFailed { message: String },

    #[error("{message}")]
    VideoNotFound { message: String },

    #[error("{message}")]
    ApiRejected { message: String },

    #[error("{message}")]
    PartOutOfRange { message: String },

    #[error("{message}")]
    AuthRequired { message: String },

    #[error("{message}")]
    RateLimited { message: String },

    #[error("{message}")]
    NoSubtitle { message: String },

    #[error("{message}")]
    SubtitleCorrupt { message: String },

    #[error("{message}")]
    Network { message: String },
}

impl BilibiliError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidUrl { .. } => "VALIDATION_ERROR",
            Self::ShortLinkFailed { .. } => "SHORT_LINK_FAILED",
            Self::VideoNotFound { .. } => "VIDEO_NOT_FOUND",
            Self::ApiRejected { .. } => "API_REJECTED",
            Self::PartOutOfRange { .. } => "PART_OUT_OF_RANGE",
            Self::AuthRequired { .. } => "AUTH_REQUIRED",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::NoSubtitle { .. } => "NO_SUBTITLE",
            Self::SubtitleCorrupt { .. } => "SUBTITLE_CORRUPT",
            Self::Network { .. } => "NETWORK_ERROR",
        }
    }

    pub fn invalid_url(message: impl Into<String>) -> Self {
        Self::InvalidUrl {
            message: message.into(),
        }
    }

    pub fn short_link_failed(message: impl Into<String>) -> Self {
        Self::ShortLinkFailed {
            message: message.into(),
        }
    }

    pub fn video_not_found(message: impl Into<String>) -> Self {
        Self::VideoNotFound {
            message: message.into(),
        }
    }

    pub fn api_rejected(message: impl Into<String>) -> Self {
        Self::ApiRejected {
            message: message.into(),
        }
    }

    pub fn part_out_of_range(message: impl Into<String>) -> Self {
        Self::PartOutOfRange {
            message: message.into(),
        }
    }

    pub fn auth_required(message: impl Into<String>) -> Self {
        Self::AuthRequired {
            message: message.into(),
        }
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::RateLimited {
            message: message.into(),
        }
    }

    pub fn no_subtitle(message: impl Into<String>) -> Self {
        Self::NoSubtitle {
            message: message.into(),
        }
    }

    pub fn subtitle_corrupt(message: impl Into<String>) -> Self {
        Self::SubtitleCorrupt {
            message: message.into(),
        }
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }
}

impl From<BilibiliError> for Error {
    fn from(value: BilibiliError) -> Self {
        Self::App {
            code: value.code(),
            message: value.to_string(),
        }
    }
}

impl From<BilibiliError> for crate::error::ErrorPayload {
    fn from(value: BilibiliError) -> Self {
        crate::error::ErrorPayload::from(Error::from(value))
    }
}

pub type Result<T> = std::result::Result<T, BilibiliError>;
