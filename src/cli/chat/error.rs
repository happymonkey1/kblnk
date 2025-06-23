use thiserror::Error;
use crate::external::auth::error::AuthCredentialsError;
use crate::external::error::StreamingClientError;
use crate::external::streaming_client::StreamingClientConfigBuilderError;

pub type Result<T> = std::result::Result<T, ChatError>;

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("generic failure during initialize: {0}")]
    InitializationError(String),
    #[error("interrupted")]
    Interrupted,
    #[error("{0}")]
    Readline(#[from] rustyline::error::ReadlineError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    StreamingClientError(#[from] StreamingClientError)
}

impl From<StreamingClientConfigBuilderError> for ChatError {
    fn from(value: StreamingClientConfigBuilderError) -> Self {
        Self::InitializationError(format!("Streaming Client config construction failed: {value}"))
    }
}

impl From<AuthCredentialsError> for ChatError {
    fn from(value: AuthCredentialsError) -> Self {
        match value {
            AuthCredentialsError::SerdeJsonError(err) => ChatError::SerdeJsonError(err),
            AuthCredentialsError::InitializationError => ChatError::InitializationError("Auth initialization error".to_string()),
            AuthCredentialsError::IoError(err) => ChatError::IoError(err),
        }
    }
}
