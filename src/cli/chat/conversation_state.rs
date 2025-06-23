use std::collections::VecDeque;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};
use crate::cli::chat::context::{ContextFile, ContextManager};
use crate::cli::chat::message::{LlmMessage, UserMessage, UserMessageContent};
use crate::cli::chat::prompt::Prompt;
use crate::cli::chat::Role;
use crate::external::{ChatMessage, ConversationStateMessage, LlmResponseMessage};
use crate::platform::PlatformContext;

const CONTEXT_PROMPT: &str = "This summary contains all relevant information from your previous conversation with the user. You must reference this information when answering any and all subsequent user queries. Explicitly acknowledge specific details from the summary provided when they're relevant to the query.";
const LLM_RESPONSE_PROMPT: &str = "I will fully incorporate this information when generating my responses, and explicitly acknowledge relevant parts of the summary when answering questions.";

#[derive(Debug, Error)]
pub enum ConversationStateError {
    #[error("next user message is not valid")]
    InvalidNextMessage,
}

pub type Result<T> = std::result::Result<T, ConversationStateError>;

/// In-memory representation of the conversation
pub struct ConversationState {
    conversation_id: String,
    next_message: Option<UserMessage>,
    history: VecDeque<ConversationHistoryEntry>,
    pub transcript: VecDeque<String>,
    valid_history_range: (usize, usize),
    pub context_manager: Option<ContextManager>,
    context_message_length: Option<usize>,
    latest_summary: Option<String>,
}

impl ConversationState {
    pub async fn new(
        context: Arc<PlatformContext>,
        conversation_id: &str,
    ) -> Self {
        let context_manager = match ContextManager::new(context, None).await {
            Ok(context_manager) => Some(context_manager),
            Err(err) => {
                warn!("Failed to initialize context manager: {}", err);
                None
            }
        };
        
        Self {
            conversation_id: conversation_id.to_string(),
            next_message: None,
            history: VecDeque::new(),
            transcript: VecDeque::new(),
            valid_history_range: Default::default(),
            context_manager,
            context_message_length: None,
            latest_summary: None,
        }
    }
    
    pub fn conversation_id(&self) -> &str {
        self.conversation_id.as_ref()
    }
    
    pub fn latest_summary(&self) -> Option<&str> {
        self.latest_summary.as_deref()
    }
    
    pub fn history(&self) -> &VecDeque<ConversationHistoryEntry> {
        &self.history
    }
    
    pub fn next_user_message(&self) -> Option<&UserMessage> {
        self.next_message.as_ref()
    }
    
    pub fn reset_next_user_message(&mut self) {
        self.next_message = None;
    }
    
    pub fn set_next_user_message(&mut self, input: String) {
        debug_assert!(self.next_message.is_none(), "next_message should be null");
        if let Some(next_message) = self.next_message.as_ref() {
            warn!("next_message='{next_message:?}' should be null");
        }
        
        let input = if input.is_empty() {
            warn!("input should not be empty when adding new message to the conversation state!");
            "<empty>".to_string()
        } else {
            input
        };
        
        let user_message = UserMessage::new_prompt(input);
        self.next_message = Some(user_message)
    }
    
    pub fn message_id(&self) -> Option<&str> {
        self.history.back().and_then(|entry| {
            let ConversationHistoryEntry { user_message, llm_message } = entry;
            llm_message.message_id()
        })
    }
    
    pub fn clear(&mut self, preserve_summary: bool) {
        self.next_message = None;
        self.history.clear();
        if !preserve_summary {
            self.latest_summary = None;
        }
    }
    
    pub fn context_message_length(&self) -> Option<usize> {
        self.context_message_length
    }
    
    pub fn append_prompts(&mut self, mut prompts: VecDeque<Prompt>) -> Option<String> {
        let last_msg = prompts.pop_back()?;
        let (mut candidate_user, mut candidate_llm) = (None::<UserMessage>, None::<LlmMessage>);
        while let Some(prompt) = prompts.pop_front() {
            let Prompt { role, content } = prompt;
            match role {
                Role::User => {
                    let user_msg = UserMessage::new_prompt(content.to_string());
                    candidate_user.replace(user_msg);
                }
                Role::LlmAssistant => {
                    let llm_msg = LlmMessage::new_response(None, content.into());
                    candidate_llm.replace(llm_msg);
                }
            }
            
            if candidate_user.is_some() && candidate_llm.is_some() {
                let llm = candidate_llm.take().unwrap();
                let user = candidate_user.take().unwrap();
                self.append_llm_transcript(&llm);
                self.history.push_back(ConversationHistoryEntry::new(user, llm));
            }
        }

        Some(last_msg.content.to_string())
    }
    
    pub fn push_llm_message(&mut self, message: LlmMessage) {
        debug_assert!(self.next_message.is_some(), "next_message should not be null");
        let next_user_message = self.next_message.take().expect("next_message should not be null");
        
        self.append_llm_transcript(&message);
        self.history.push_back(ConversationHistoryEntry::new(next_user_message, message));
    }
    
    /// Convert into a [external::ConversationStateMessage] capable of being sent to external APIs
    pub(crate) async fn as_sendable_conversation_state(&mut self) -> ConversationStateMessage {
        debug_assert!(self.next_message.is_some(), "next_message should not be null");
        self.history.drain(self.valid_history_range.1..);
        self.history.drain(..self.valid_history_range.0);
        
        let context = self.backend_conversation_state().await;
        
        context.into_conversation_state_message()
            .expect("Failed to convert in-memory conversation state to conversation state message!")
    }
    
    pub async fn backend_conversation_state(&mut self) -> BackendConversationState<'_> {
        let conversation_start_context = None;
        
        let (context_messages, dropped_context_files) =
            self.build_context_messages(conversation_start_context).await;
        
        BackendConversationState {
            conversation_id: self.conversation_id.as_str(),
            next_user_message: self.next_message.as_ref(),
            history: self.history.range(self.valid_history_range.0..self.valid_history_range.1),
            context_messages,
            dropped_context_files,
        }
    }
    
    /// Build internal summary, context files, and the conversation together
    /// that is sent to the model
    async fn build_context_messages(
        &mut self,
        conversation_start_context: Option<String>,
    ) -> (Option<Vec<ConversationHistoryEntry>>, Vec<ContextFile>) {
        let mut context_content = String::new();
        let mut dropped_context_files = Vec::new();
        if let Some(summary) = &self.latest_summary {
            context_content.push_str("<summary>\n");
            context_content.push_str(CONTEXT_PROMPT);
            context_content.push_str(summary);
            context_content.push_str("\n");
            context_content.push_str("</summary>\n");
        }
        
        // Try to add context files
        if let Some(context_manager) = self.context_manager.as_ref() {
            match context_manager.collect_context_files_with_limit().await {
                Ok((files_to_use, files_to_drop)) => {
                    if !files_to_drop.is_empty() {
                        dropped_context_files.extend(files_to_drop);
                    }
                    
                    if !files_to_use.is_empty() {
                        context_content.push_str("<context>\n");
                        for ContextFile { filename, content } in files_to_use {
                            context_content.push_str(&format!("[{}]\n{}\n", filename, content));
                        }
                        context_content.push_str("</context>\n");
                    }
                }
                Err(err) => {
                    warn!("Failed to retrieve context files: {}", err);
                }
            }
        }
        
        if let Some(context) = conversation_start_context {
            context_content.push_str(&context);
        }
        
        if !context_content.is_empty() {
            info!("Context content is not empty");
            
            self.context_message_length = Some(context_content.len());
            let user_message = UserMessage::new_prompt(context_content);
            let llm_message = LlmMessage::new_response(None, LLM_RESPONSE_PROMPT.into());
            (Some(vec![ConversationHistoryEntry{ user_message, llm_message }]), dropped_context_files)
        } else {
            info!("Context content is empty");
            (None, dropped_context_files)
        }
    }
    
    fn append_user_transcript(&mut self, message: &UserMessage) {
        let content = match message.content() {
            UserMessageContent::Prompt { prompt } => prompt,
        };
        
        self.append_transcript(format!("> {}", content.replace("\n", ">")))
    }
    
    fn append_llm_transcript(&mut self, message: &LlmMessage) {
        self.append_transcript(format!("{}", message.content()))
    }
    
    fn append_transcript(&mut self, message: String) {
        self.transcript.push_back(message);
    }
}

/// Simple wrapper for a tuple of ([UserMessage], [LlmMessage])
pub struct ConversationHistoryEntry {
    pub user_message: UserMessage,
    pub llm_message: LlmMessage,
}

impl ConversationHistoryEntry {
    pub fn new(user_message: UserMessage, llm_message: LlmMessage) -> Self {
        Self { user_message, llm_message }
    }
}

pub type BackendConversationState<'a> = BackendConversationStateImpl<
    'a,
    std::collections::vec_deque::Iter<'a, ConversationHistoryEntry>,
    Option<Vec<ConversationHistoryEntry>>,
>;

#[derive(Debug, Clone)]
pub struct BackendConversationStateImpl<'a, T, U> {
    pub conversation_id: &'a str,
    pub next_user_message: Option<&'a UserMessage>,
    pub history: T,
    pub context_messages: U,
    pub dropped_context_files: Vec<ContextFile>,
}

impl BackendConversationStateImpl<'_, std::collections::vec_deque::Iter<'_, ConversationHistoryEntry>, Option<Vec<ConversationHistoryEntry>>> {
    fn into_conversation_state_message(self) -> Result<ConversationStateMessage> {
        let history = flatten_history(self.context_messages.unwrap_or_default().iter().chain(self.history));
        let user_input_message = self.next_user_message
            .cloned()
            .map(|user_message| user_message.into_user_input_message())
            .ok_or(ConversationStateError::InvalidNextMessage)?;
        
        Ok(ConversationStateMessage::new(
            self.conversation_id.to_string(),
            Some(user_input_message),
            history,
        ))
    }

    pub fn calculate_conversation_size(&self) -> ConversationSize {
        let mut user_chars = 0;
        let mut llm_chars = 0;
        let mut context_chars = 0;

        let history = self.history.clone();
        for history_entry in history {
            user_chars += history_entry.user_message.get_char_count();
            llm_chars += history_entry.llm_message.get_char_count();
        }

        context_chars += self
            .context_messages
            .as_ref()
            .map(|v| {
                v.iter()
                    .fold(0, |acc, ConversationHistoryEntry{ user_message, llm_message }| {
                        acc + user_message.get_char_count() + llm_message.get_char_count()
                    })
            })
            .unwrap_or_default();

        ConversationSize {
            context_messages: context_chars.into(),
            user_messages: user_chars.into(),
            assistant_messages: llm_chars.into(),
        }
    }
}

/// Token usage calculations of the conversation stored in-memory 
#[derive(Debug, Clone, Copy)]
pub struct ConversationSize {
    pub context_messages: usize,
    pub user_messages: usize,
    pub assistant_messages: usize,
}

/// Flatten an in-memory conversation history into a series of [ChatMessage]
fn flatten_history<'a, T>(history: T) -> Vec<ChatMessage>
where
    T: Iterator<Item = &'a ConversationHistoryEntry>
{
    history.fold(Vec::new(), |mut acc, entry| {
        let ConversationHistoryEntry { user_message, llm_message } = entry;
        acc.push(ChatMessage::UserInputMessage(user_message.clone().into_user_history()));
        acc.push(ChatMessage::LlmResponseMessage(LlmResponseMessage::from(llm_message.clone())));
        acc
    })
}
