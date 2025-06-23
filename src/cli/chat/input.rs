use tracing::error;
use crate::cli::util::input_source::InputSource;
use crate::cli::chat::error::{ChatError, Result};

pub async fn read_user_input(input_source: &mut InputSource, prompt: Option<&str>) -> Result<String> {
    match input_source.readline(prompt) {
        Ok(input) => {
            match input {
                Some(input) => Ok(input),
                None => Err(ChatError::Interrupted)
            }
        }
        Err(err) => {
            error!("Error reading user input: {err:?}");
            Err(ChatError::InvalidUserInput)
        },
    }
}