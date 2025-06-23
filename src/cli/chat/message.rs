use serde::{Deserialize, Serialize};
use crate::external::{EnvState, UserInputMessage, UserInputMessageContext};

const USER_ENTRY_START: &str = "<user-query>";
const USER_ENTRY_END: &str = "</user-query>";

#[derive(Debug, Clone)]
pub struct UserMessage {
    pub additional_context: String,
    pub env_context: UserEnvContext,
    pub content: UserMessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserMessageContent {
    Prompt { prompt: String },
}

impl UserMessage {
    /// Construct a new [UserMessage] from an input prompt
    pub fn new_prompt(prompt: String) -> Self {
        Self {
            additional_context: String::new(),
            env_context: UserEnvContext::generate_new(),
            content: UserMessageContent::Prompt { prompt },
        }
    }
    
    /// Convert the user message into a [UserInputMessage] to be stored
    /// in a [external::ConversationStateMessage]
    pub fn into_user_history(self) -> UserInputMessage {
        UserInputMessage {
            content: self.prompt().unwrap_or_default().to_string(),
            // TODO: pass git state
            user_input_message_context: Some(UserInputMessageContext::new(self.env_context.env_state, None)),
        }
    }
    
    /// Convert the user message into a [UserInputMessage] to be sent
    /// in a [external::ConversationStateMessage]
    pub fn into_user_input_message(self) -> UserInputMessage {
        let formatted_prompt = match self.prompt() {
            Some(prompt) if !prompt.is_empty() => {
                format!("{}{}{}", USER_ENTRY_START, prompt, USER_ENTRY_END)
            }
            _ => String::new(),
        };
        
        UserInputMessage {
            content: formatted_prompt,
            // TODO: pass git state
            user_input_message_context: Some(UserInputMessageContext::new(self.env_context.env_state, None)),
        }
    }

    pub fn additional_context(&self) -> &str {
        &self.additional_context
    }

    pub fn content(&self) -> &UserMessageContent {
        &self.content
    }
    
    pub fn prompt(&self) -> Option<&str> {
        match &self.content {
            UserMessageContent::Prompt { prompt } => Some(prompt.as_str())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserEnvContext {
    env_state: Option<EnvState>,
}

impl UserEnvContext {
    pub fn generate_new() -> Self {
        Self {
            env_state: Some(EnvState::new()),
        }
    }
}

pub enum LlmMessage {
    Response {
        message_id: Option<String>,
        content: String,
    }
}

impl LlmMessage {
    pub fn new_response(message_id: Option<String>, content: String) -> Self {
        Self::Response {
            message_id,
            content,
        }
    }
    
    pub fn message_id(&self) -> Option<&str> {
        match self {
            Self::Response { message_id, .. } => message_id.as_deref(),
        }
    }
    
    pub fn content(&self) -> &str {
        match self {
            Self::Response { content, .. } => content.as_str(),
        }
    }
}

