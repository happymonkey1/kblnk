use thiserror::Error;

#[derive(Debug, Error)]
pub enum StreamingClientError {
    #[error("invalid auth credentials")]
    InvalidAuthCredentials,

    #[error("bad gateway")]
    BadGateway,

    #[error("generic server error")]
    ServerError,

    #[error("bad request")]
    BadRequest,

    #[error("generic forbidden error")]
    Forbidden,

    #[error("generic unhandled internal cli error")]
    InternalError,
    
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StreamingClientError>;
