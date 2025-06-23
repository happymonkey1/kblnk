use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("home dir does not exist")]
    HomeDirNotExistError,
    #[error("invalid directory")]
    InvalidDirectoryError,
    #[error("file does not exist")]
    FileNotExistError,
    #[error("generic io error")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, PlatformError>;