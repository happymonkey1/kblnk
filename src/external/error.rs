use serde::Deserialize;
use thiserror::Error;
use crate::platform::error::PlatformError;

#[derive(Debug, Error)]
pub enum StreamingClientError {
    #[error("invalid auth credentials")]
    InvalidAuthCredentialsError,
    
    #[error("bad gateway")]
    BadGatewayError,

    #[error("generic server error")]
    ServerError,

    #[error("bad request")]
    BadRequestError,

    #[error("generic forbidden error")]
    ForbiddenError,

    #[error("generic unhandled internal cli error")]
    InternalError,
    
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::error::Error),
    
    #[error(transparent)]
    PlatformError(#[from] PlatformError),
    
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, StreamingClientError>;
