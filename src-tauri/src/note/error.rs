use crate::error::Error;
use crate::llm::LlmError;

pub type NoteResult<T> = Result<T, NoteError>;

#[derive(Debug, thiserror::Error)]
pub enum NoteError {
    #[error("job cancelled")]
    Cancelled,

    #[error(transparent)]
    Llm(#[from] LlmError),
}

impl NoteError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Cancelled => "JOB_CANCELLED",
            Self::Llm(err) => err.code(),
        }
    }
}

impl From<NoteError> for Error {
    fn from(value: NoteError) -> Self {
        Self::App {
            code: value.code(),
            message: match &value {
                NoteError::Cancelled => "Note generation was cancelled".to_string(),
                NoteError::Llm(err) => err.to_string(),
            },
        }
    }
}

impl From<NoteError> for crate::error::ErrorPayload {
    fn from(value: NoteError) -> Self {
        crate::error::ErrorPayload::from(Error::from(value))
    }
}
