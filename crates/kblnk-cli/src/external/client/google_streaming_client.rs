pub(crate) use crate::external::client::GoogleModel;
use crate::external::model::SendMessageResponseStream;
use crate::external::streaming_client::StreamingClient;
use crate::external::ConversationStateMessage;
use async_trait::async_trait;
use google_ai_rs::{Client, TryIntoContents};
use crate::external::consts::CONTEXT_WINDOW_SIZE_1_M;
use crate::external::error::Result;

// Reference: https://docs.rs/google-ai-rs/latest/google_ai_rs/

#[derive(Clone, Debug)]
pub struct GoogleStreamingClient {
    client: Client,
    config: GoogleStreamingClientConfig,
}

#[derive(Clone, Debug)]
pub struct GoogleStreamingClientConfig {
    pub(crate) model: GoogleModel,
}


impl GoogleStreamingClient {
    pub async fn new(
        api_key: String,
        config: GoogleStreamingClientConfig,
    ) -> Result<Self> {
        let client = Client::new(api_key.into()).await?;

        Ok(Self {
            client,
            config,
        })
    }
    
    pub fn get_model_name<'a>(&self) -> &'a str {
        self.config.model.to_model_name()
    }
}

#[async_trait]
impl StreamingClient for GoogleStreamingClient {
    async fn send_message(&self, message: ConversationStateMessage) -> Result<SendMessageResponseStream> {
        // TODO: expose model configuration
        let model = self.client.generative_model(self.get_model_name())
            .with_system_instruction(include_str!("../../../resources/system_prompt.md"));

        let response_stream = model.stream_generate_content(message.try_into_contents()?)
            .await?;
        
        Ok(SendMessageResponseStream::from(response_stream))
    }

    fn get_context_window_size(&self) -> usize {
        match self.config.model {
            GoogleModel::Gemini25Flash => CONTEXT_WINDOW_SIZE_1_M
        }
    }
}
