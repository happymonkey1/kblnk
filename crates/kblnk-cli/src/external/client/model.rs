use google_ai_rs::{Content, Part, TryIntoContents};
use google_ai_rs::genai::ResponseStream;
use google_ai_rs::proto::GenerateContentResponse;
use log::info;
use serde_json::Value;
use tracing::{debug, warn};
use crate::external::{ChatMessage, ConversationStateMessage, LlmResponseMessage, UserInputMessage};
use crate::external::client::{LLM_RESPONSE_CONTEXT_HEADER_END, LLM_RESPONSE_CONTEXT_HEADER_START, USER_ENV_CONTEXT_HEADER_END, USER_ENV_CONTEXT_HEADER_START};
use crate::external::error::StreamingClientError;
use crate::external::model::{ChatResponseStream, SendMessageResponseStream};

impl TryIntoContents for ConversationStateMessage {
    fn try_into_contents(self) -> Result<Vec<Content>, google_ai_rs::error::Error> {
        let mut parts: Vec<Part> = vec![];

        // Try to push the next user message
        if let Some(content) = self.user_input_message {
            parts.push(content.try_into()?)
        }

        // Try to push conversation history
        if let Some(history) = self.history {
            for history_entry in history {
                parts.push(history_entry.try_into()?)
            }
        }

        Ok(vec![
            google_ai_rs::Content::from(parts)
        ])
    }
}

impl TryInto<Part> for UserInputMessage {
    type Error = StreamingClientError;

    fn try_into(self) -> crate::external::error::Result<google_ai_rs::Part> {
        let formatted_content = if let Some(context) = self.user_input_message_context {
            let serialized_context = serde_json::to_string_pretty(&context)?;
            // TODO: should probably do the prompt formatting in a common location
            format!("{}\n{}\n{}", USER_ENV_CONTEXT_HEADER_START, serialized_context, USER_ENV_CONTEXT_HEADER_END)
        } else {
            String::new()
        };

        let formatted_part = format!("{}\n{}", self.content, formatted_content);

        Ok(Part::text(formatted_part.as_str()))
    }
}

impl TryInto<Part> for LlmResponseMessage {
    type Error = StreamingClientError;

    fn try_into(self) -> Result<Part, Self::Error> {
        let formatted_content = format!(
            "{}\n{}\n{}",
            LLM_RESPONSE_CONTEXT_HEADER_START,
            self.content,
            LLM_RESPONSE_CONTEXT_HEADER_END
        );

        Ok(Part::text(formatted_content.as_str()))
    }
}

impl TryInto<Part> for ChatMessage {
    type Error = StreamingClientError;

    fn try_into(self) -> Result<Part, Self::Error> {
        match self {
            ChatMessage::UserInputMessage(user) => user.try_into(),
            ChatMessage::LlmResponseMessage(llm) => llm.try_into(),
        }
    }
}

impl From<ResponseStream> for SendMessageResponseStream {
    fn from(value: ResponseStream) -> Self {
        SendMessageResponseStream::GoogleAiStudio(value)
    }
}

impl From<GenerateContentResponse> for ChatResponseStream {
    // TODO: verify in Google AI studio docs that the model is not hallucinating json responses...
    fn from(value: GenerateContentResponse) -> Self {
        let raw_response = value.text();
        Self::LlmResponseEvent { content: raw_response }
    }
}
