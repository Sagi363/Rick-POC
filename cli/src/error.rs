use std::fmt;
use std::io;

#[derive(Debug)]
pub enum RickError {
    Io(io::Error),
    Parse(String),
    NotFound(String),
    InvalidState(String),
}

impl fmt::Display for RickError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RickError::Io(e) => write!(f, "IO error: {}", e),
            RickError::Parse(msg) => write!(f, "Parse error: {}", msg),
            RickError::NotFound(msg) => write!(f, "Not found: {}", msg),
            RickError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl From<io::Error> for RickError {
    fn from(e: io::Error) -> Self {
        RickError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, RickError>;
