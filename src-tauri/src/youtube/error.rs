use crate::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum YoutubeError {
    #[error("{message}")]
    InvalidUrl { message: String },

    #[error("{message}")]
    VideoNotFound { message: String },

    #[error("{message}")]
    VideoRestricted { message: String },

    #[error("{message}")]
    ApiRejected { message: String },

    #[error("{message}")]
    NoSubtitle { message: String },

    #[error("{message}")]
    SubtitleCorrupt { message: String },

    #[error("{message}")]
    Network { message: String },
}

impl YoutubeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidUrl { .. } => "VALIDATION_ERROR",
            Self::VideoNotFound { .. } => "VIDEO_NOT_FOUND",
            Self::VideoRestricted { .. } => "VIDEO_RESTRICTED",
            Self::ApiRejected { .. } => "API_REJECTED",
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

    pub fn video_not_found(message: impl Into<String>) -> Self {
        Self::VideoNotFound {
            message: message.into(),
        }
    }

    pub fn video_restricted(message: impl Into<String>) -> Self {
        Self::VideoRestricted {
            message: message.into(),
        }
    }

    pub fn api_rejected(message: impl Into<String>) -> Self {
        Self::ApiRejected {
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

impl From<YoutubeError> for Error {
    fn from(value: YoutubeError) -> Self {
        Self::App {
            code: value.code(),
            message: value.to_string(),
        }
    }
}

impl From<YoutubeError> for crate::error::ErrorPayload {
    fn from(value: YoutubeError) -> Self {
        crate::error::ErrorPayload::from(Error::from(value))
    }
}

pub type Result<T> = std::result::Result<T, YoutubeError>;
