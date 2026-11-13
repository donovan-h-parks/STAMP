use std::fmt;

/// An error with a message suitable for showing to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StampError(pub String);

impl StampError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        StampError(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StampError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StampError {}

pub type Result<T> = std::result::Result<T, StampError>;
