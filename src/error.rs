use std::{error::Error, fmt::Display};

#[derive(Debug)]
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
            "The word \"{}\" is too long for the current terminal size",
            self.word
        ))
    }
}

impl Error for WordTooLongError {}
