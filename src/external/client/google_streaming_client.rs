use google_ai_rs::{Client, Content, TryIntoContents};
use tracing::error;
use crate::external::auth::auth_credentials::AuthCredentials;
use crate::external::{ConversationStateMessage, UserInputMessage};
use crate::external::model::SendMessageResponse;
use crate::external::streaming_client::{StreamingClient, StreamingClientError};

// Reference: https://docs.rs/google-ai-rs/latest/google_ai_rs/

const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";

#[derive(Clone, Debug)]
pub struct GoogleStreamingClient {
    client: Client,
}

impl GoogleStreamingClient {
    pub async fn new(
        credentials: AuthCredentials,
    ) -> Result<Self> {
        let credentials = match credentials {
            AuthCredentials::ApiKey(api_key) => Ok(api_key),
            _ => {
                error!("Invalid AuthCredentials type for GoogleStreamingClient");
                Err(StreamingClientError::InvalidAuthCredentialsType)
            }
        }?;

        let client = Client::new(credentials.into()).await?;
        // TODO: expose model configuration


        Ok(Self {
            client,
        })
    }
}

impl StreamingClient for GoogleStreamingClient {
    async fn send_message(&self, message: ConversationStateMessage) -> crate::external::streaming_client::Result<SendMessageResponse> {
        let model = self.client.generative_model(GEMINI_2_5_FLASH)
            .with_response_format("application/json");

        let response = model.stream_generate_content(

        )
            .await?;

    }
}

type Result<T> = crate::external::streaming_client::Result<T>;

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

impl TryIntoContents for ConversationStateMessage {
    fn try_into_contents(self) -> std::result::Result<Vec<Content>, google_ai_rs::error::Error> {
        let mut parts: Vec<google_ai_rs::Part> = vec![];
        
        // Try to push the next user message 
        if let Some(content) = self.user_input_message {
            parts.push(google_ai_rs::Part::from(content))
        }
        
        // Try to push conversation history
        if let Some(history) = self.history {
            for history_entry in history {
                parts.push(google_ai_rs::Part::from(history_entry))
            }
        }
        
        Ok(vec![
            google_ai_rs::Content::from(parts)
        ])
    }
}