use thiserror::Error;
use crate::external::ConversationStateMessage;
use crate::external::model::SendMessageResponse;

pub trait StreamingClient {
    async fn send_message(&self, message: ConversationStateMessage) -> Result<SendMessageResponse>;
}

pub struct StreamingClientImpl {
    inner: inner::Inner,
}

mod inner {
    use std::sync::{Arc, Mutex};
    use crate::external::client::google_streaming_client::GoogleStreamingClient;

    #[derive(Clone, Debug)]
    pub enum Inner{
        GoogleAiStudio(GoogleStreamingClient),
        HttpsStreamingClient(),
    }
}

#[derive(Debug, Error)]
pub enum StreamingClientError {
    #[error("invalid auth credentials type")]
    InvalidAuthCredentialsType,
    
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
}

pub type Result<T> = std::result::Result<T, StreamingClientError>;

impl StreamingClientImpl {
    pub fn new() -> Result<Self> {

    }

    pub async fn send_message(
        &self,
        conversation_state_message: ConversationStateMessage
    ) -> Result<SendMessageResponse> {

        match &self.inner {
            inner::Inner::GoogleAiStudio(client) => {

                client.generative_model()
            }
        }

    }
}