use std::{error::Error, fmt::Display};

use crate::app::{MIN_TERM_COL, MIN_TERM_ROW};

#[derive(Debug, Clone)]
pub struct WordTooLongError {
    word: String,
    max_length: u16,
}

impl WordTooLongError {
    pub fn new(word: impl Into<String>, max_length: u16) -> WordTooLongError {
        WordTooLongError {
            word: word.into(),
            max_length,
        }
    }
}

impl Display for WordTooLongError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "The word \"{}\" is too long for the current terminal size or longer than {} characters.",
            self.word,
            self.max_length,
        ))
    }
}

impl Error for WordTooLongError {}

#[derive(Debug)]
pub struct TerminalTooSmallError;

impl Display for TerminalTooSmallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "The terminal size is too small. Min column count is {} and minimum row count is {}.",
            MIN_TERM_COL, MIN_TERM_ROW
        ))
    }
}

#[derive(Debug)]
pub enum TyperError {
    TerminalTooSmallError(TerminalTooSmallError),
    WordTooLongError(WordTooLongError),
}

impl Display for TyperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            TyperError::TerminalTooSmallError(e) => e.to_string(),
            TyperError::WordTooLongError(e) => e.to_string(),
        };
        f.write_str(&text)
    }
}
