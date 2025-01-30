use std::{error::Error, fmt::Display};

#[derive(Debug, Clone)]
pub struct WordTooLongError {
    word: String,
}

impl WordTooLongError {
    pub fn new(word: impl Into<String>) -> WordTooLongError {
        WordTooLongError { word: word.into() }
    }
}

impl Display for WordTooLongError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "The word \"{}\" is too long for the current terminal size or longer than 80 characters.",
            self.word
        ))
    }
}

impl Error for WordTooLongError {}

#[derive(Debug)]
pub struct TerminalTooSmallError;

impl Display for TerminalTooSmallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "The terminal size is too small. Min column count is 65 and minimum row count is 15.",
        )
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
