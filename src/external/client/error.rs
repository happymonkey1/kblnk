use crate::external::error::StreamingClientError;

impl From<google_ai_rs::error::Error> for StreamingClientError {
    fn from(value: google_ai_rs::error::Error) -> Self {
        match value {
            google_ai_rs::error::Error::Setup(_) => StreamingClientError::InternalError,
            google_ai_rs::error::Error::Net(_) => StreamingClientError::BadGateway,
            google_ai_rs::error::Error::Service(_) => StreamingClientError::ServerError,
            google_ai_rs::error::Error::Stream(_) => StreamingClientError::ServerError,
            google_ai_rs::error::Error::Auth(_) => StreamingClientError::Forbidden,
            google_ai_rs::error::Error::InvalidArgument(_) => StreamingClientError::BadRequest,
            google_ai_rs::error::Error::InvalidContent(_) => StreamingClientError::BadRequest,
            _ => StreamingClientError::InternalError,
        }
    }
}

impl From<StreamingClientError> for google_ai_rs::error::Error {
    fn from(value: StreamingClientError) -> Self {
        google_ai_rs::error::SetupError::new("internal client error", Box::new(value)) 
    }
}
