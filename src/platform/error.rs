use thiserror::Error;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("home dir does not exist")]
    HomeDirNotExistError,
}

pub type Result<T> = std::result::Result<T, PlatformError>;