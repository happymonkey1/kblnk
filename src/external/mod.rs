use std::env;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::error;
use crate::external::model::SendMessageResponseStream;

pub mod streaming_client;
pub mod model;
pub mod client;
pub mod auth;
pub mod error;

pub use client::GoogleModel;

#[async_trait]
pub trait StreamingClient {
    async fn send_message(&self, message: ConversationStateMessage) -> error::Result<SendMessageResponseStream>;
}

/// ConversationState capable of being sent as a message via the streaming client
pub struct ConversationStateMessage {
    conversation_id: Option<String>,
    user_input_message: Option<UserInputMessage>,
    history: Option<Vec<ChatMessage>>
}

pub enum ChatMessage {
    UserInputMessage(UserInputMessage),
    LlmResponseMessage(LlmResponseMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInputMessage {
    pub content: String,
    pub user_input_message_context: Option<UserInputMessageContext>,
}

/// Stores context from the user's "working environment", such as environment variables,
/// git status, etc.
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInputMessageContext {
    pub env_state: Option<EnvState>,
    pub git_state: Option<GitState>,
}

impl UserInputMessageContext {
    pub fn new(
        env_state: Option<EnvState>,
        git_state: Option<GitState>,
    ) -> Self {
        
        Self {
            env_state,
            git_state,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvState {
    /// Operating system
    pub os: Option<String>,
    /// Current working directory
    pub cwd: Option<String>,
    pub env_variables: Vec<EnvVar>,
}

impl EnvState {
    pub fn new() -> Self {
        let cwd = match std::env::current_dir() {
            Ok(cwd) => Some(cwd.to_string_lossy().into()),
            Err(err) => {
                error!("Failed to fetch the current working directory: {err:?}");
                Some(String::new())
            }
        };
        
        Self {
            os: Some(env::consts::OS.into()),
            cwd,
            env_variables: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    
}

pub struct LlmResponseMessage;

impl ConversationStateMessage {
    pub fn new(
        conversation_id: String,
        user_input_message: Option<UserInputMessage>,
        history: Vec<ChatMessage>,
    ) -> Self {
        Self {
            conversation_id: Some(conversation_id),
            user_input_message,
            history: Some(history),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LlmServerProvider {
    LlamaCpp,
    GoogleAiStudio{ model: GoogleModel },
}