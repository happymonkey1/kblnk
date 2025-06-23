use thiserror::Error;
use tracing::info;
use crate::external::auth::auth_credentials::AuthCredentials;
pub(crate) use crate::external::{ConversationStateMessage, LlmServerProvider, StreamingClient};
use crate::external::client::google_streaming_client::{GoogleStreamingClient, GoogleStreamingClientConfig};
use crate::external::model::{SendMessageResponseStream};

use crate::external::error::{Result, StreamingClientError};

pub struct StreamingClientImpl {
    inner: Box<dyn StreamingClient>,
}

impl StreamingClientImpl {
    pub async fn new(config: StreamingClientConfig) -> Result<Self> {
        let creds = config.auth_credentials.ok_or_else(|| StreamingClientError::InvalidAuthCredentials)?;
        
        let client = match config.llm_server_provider {
            LlmServerProvider::LlamaCpp => todo!("LLamaCpp does not have a supported StreamingClient at the moment"),
            LlmServerProvider::GoogleAiStudio { model } => {
                info!("Building Google streaming client");
                let api_key = creds.google_api_key.ok_or_else(|| StreamingClientError::InvalidAuthCredentials)?;
                
                let config = GoogleStreamingClientConfig{
                    model,
                };
                
                GoogleStreamingClient::new(api_key, config).await
            }
        }?;
        
        Ok(StreamingClientImpl{
            inner: Box::new(client),
        })
    }

    pub async fn send_message(
        &self,
        conversation_state_message: ConversationStateMessage
    ) -> Result<SendMessageResponseStream> {
        self.inner.send_message(conversation_state_message).await
    }
}

pub struct StreamingClientConfig {
    /// LLM Server provider
    llm_server_provider: LlmServerProvider,
    /// Optional authentication credentials
    /// Some language model providers may not require credentials (For example, self-hosted)
    auth_credentials: Option<AuthCredentials> 
}

impl StreamingClientConfig {
    pub fn builder() -> StreamingClientConfigBuilder {
        StreamingClientConfigBuilder {
            llm_server_provider: None,
            auth_credentials: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum StreamingClientConfigBuilderError {
    #[error("Required option '{0}' is not present")]
    RequiredOptionNotFound(String),
}

pub struct StreamingClientConfigBuilder {
    llm_server_provider: Option<LlmServerProvider>,
    auth_credentials: Option<AuthCredentials>,
}

impl StreamingClientConfigBuilder {
    pub fn build(self) -> std::result::Result<StreamingClientConfig, StreamingClientConfigBuilderError> {
        let llm_server_provider = self.llm_server_provider.ok_or_else(|| StreamingClientConfigBuilderError::RequiredOptionNotFound("llm_server_provider".to_string()))?;
        
        Ok(StreamingClientConfig {
            llm_server_provider, 
            auth_credentials: self.auth_credentials,
        })
    }
    
    pub fn with_llm_provider(mut self, llm_server_provider: LlmServerProvider) -> Self {
        self.llm_server_provider = Some(llm_server_provider);
        self
    }
    
    pub fn with_auth_credentials(mut self, auth_credentials: Option<AuthCredentials>) -> Self {
        self.auth_credentials = auth_credentials;
        self
    }
}