use serde::{Deserialize, Serialize};

pub mod google_streaming_client;
mod model;
mod error;

pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";

const USER_ENV_CONTEXT_HEADER_START: &str = "<user-env-context>";
const USER_ENV_CONTEXT_HEADER_END: &str = "</user-env-context>";
const LLM_RESPONSE_CONTEXT_HEADER_START: &str = "<previous-assistant-response>";
const LLM_RESPONSE_CONTEXT_HEADER_END: &str = "</previous-assistant-response>";

/// GenAI Model Name(s) for Google AI Studio
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GoogleModel {
    Gemini25Flash,
}

impl GoogleModel {
    pub fn to_model_name<'a>(&self) -> &'a str {
        match self {
            GoogleModel::Gemini25Flash => GEMINI_2_5_FLASH
        }
    }
}