use thiserror::Error;
use crate::parser::parser::{ParserState, Token};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseError {
    #[error("unexpected character: {0}")]
    UnexpectedCharacterError(char),
    #[error("unexpected token: {0:?}")]
    UnexpectedToken(ParserState, Token),
    #[error("invalid xml document")]
    InvalidDocumentError,
    #[error("unexpected root node")]
    UnexpectedRoot,
    #[error("unexpected content outside of root node")]
    ContentOutsideRoot,
    #[error("unmatched close tag: {0}")]
    UnmatchedCloseTag(String),
    #[error("mismatched tags (expected {expected:?}, found {found:?}")]
    TagMismatch { expected: (String, Option<String>), found: (String, Option<String>) },
    #[error("unclosed tags: {0:?}")]
    UnclosedTags(Vec<String>),
    #[error("parsed empty xml document")]
    EmptyDocument,
    #[error("parent not found")]
    InvalidParent,
    #[error("invalid internal state (state {0:?}, token: {1:?}")]
    InvalidStateError(ParserState, Token)
}

pub type Result<T> = std::result::Result<T, ParseError>;
