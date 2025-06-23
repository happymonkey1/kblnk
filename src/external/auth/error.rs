use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthCredentialsError {
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("initialization error")]
    InitializationError,
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AuthCredentialsError>;