use thiserror::Error;
use crate::platform::error::PlatformError;

#[derive(Debug, Error)]
pub enum AuthCredentialsError {
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("initialization error")]
    InitializationError,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    PlatformError(#[from] PlatformError),
}

pub type Result<T> = std::result::Result<T, AuthCredentialsError>;