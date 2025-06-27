use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info};
use crate::external::auth::auth_credentials::AuthCredentials;
pub(crate) use crate::external::{ConversationStateMessage, LlmServerProvider, StreamingClient};
use crate::external::client::google_streaming_client::{GoogleStreamingClient, GoogleStreamingClientConfig};
use crate::external::model::{SendMessageResponseStream};
use crate::external::error::{Result, StreamingClientError};
use crate::external::STREAMING_CLIENT_CONFIG_FILE_NAME;
use crate::platform::error::PlatformError;
use crate::platform::PlatformContext;

pub struct StreamingClientImpl {
    inner: Box<dyn StreamingClient>,
}

impl StreamingClientImpl {
    pub async fn new(config: StreamingClientConfig) -> Result<Self> {
        let creds = config.auth_credentials.ok_or_else(|| StreamingClientError::InvalidAuthCredentialsError)?;
        
        let client = match config.llm_server_provider {
            LlmServerProvider::LlamaCpp => todo!("LLamaCpp does not have a supported StreamingClient at the moment"),
            LlmServerProvider::GoogleAiStudio { model } => {
                info!("Building Google streaming client");
                let api_key = creds.google_api_key.ok_or_else(|| StreamingClientError::InvalidAuthCredentialsError)?;
                
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
    
    pub fn get_context_window_size(&self) -> usize { self.inner.get_context_window_size() }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingClientConfig {
    /// LLM Server provider
    pub llm_server_provider: LlmServerProvider,
    /// Optional authentication credentials
    /// Some language model providers may not require credentials (For example, self-hosted)
    #[serde(skip)]
    pub auth_credentials: Option<AuthCredentials> 
}

impl StreamingClientConfig {
    pub fn builder() -> StreamingClientConfigBuilder {
        StreamingClientConfigBuilder {
            llm_server_provider: None,
            auth_credentials: None,
        }
    }
    
    /// Builds a [StreamingClientConfig] from a serialized file.
    pub async fn build_streaming_client_config(context: Arc<PlatformContext>) -> Result<StreamingClientConfig> {
        let app_dir = context.get_app_config_directory()?;
        let client_config_filepath = app_dir.join(STREAMING_CLIENT_CONFIG_FILE_NAME);
        
        info!("Trying to read client config from '{:?}'", &client_config_filepath);
        if !context.fs().exists(&client_config_filepath).await {
             return Err(StreamingClientError::PlatformError(PlatformError::FileNotExistError))
        }
        
        let client_config_string = context.fs().read_to_string(client_config_filepath).await
            .map_err(|err| PlatformError::IoError(err))?;
        
        Ok(serde_json::from_str(client_config_string.as_str())?)
    }

    // TODO: config abstraction
    pub fn get_streaming_client_config_file_path(context: Arc<PlatformContext>) -> Result<PathBuf> {
        let app_dir = context.get_app_config_directory()?;
        // TODO: should optionally read from env
        Ok(app_dir.join(STREAMING_CLIENT_CONFIG_FILE_NAME))
    }
    
    // TODO: config abstraction
    pub async fn save_config(&self, context: Arc<PlatformContext>, path: impl AsRef<Path>) -> Result<()> {
        let config_string = serde_json::to_string_pretty(self)?;

        let parent_path = if let Some(path) = path.as_ref().parent() {
            path
        } else {
            error!("AuthCredentials parent config path is not valid: '{:?}'", path.as_ref());
            return Err(StreamingClientError::PlatformError(PlatformError::InvalidDirectoryError))
        };

        if !context.fs().exists(parent_path).await {
            info!("Creating directories for non-existent parent path: '{:?}'", parent_path);
            context.fs().create_dir_all(parent_path).await?;
        }

        info!("Saving StreamingClient config to '{:?}'", path.as_ref());

        context.fs().write(path, config_string).await?;

        Ok(())
    }

    // TODO: config abstraction
    pub async fn save_config_to_default_location(&self, context: Arc<PlatformContext>) -> Result<()> {
        let config_filepath = Self::get_streaming_client_config_file_path(Arc::clone(&context))?;
        self.save_config(context, config_filepath).await
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