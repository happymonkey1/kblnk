use thiserror::Error;
use crate::cli::chat::error::ChatError;

#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    ChatError(#[from] ChatError)
}

pub type Result<T> = std::result::Result<T, CliError>;