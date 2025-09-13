use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct ConfError {
    message: String,
}

impl Debug for ConfError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Display for ConfError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ConfError {
    
}

impl ConfError {
    pub fn new(message: &str) -> ConfError {
        ConfError { message: message.to_string() }
    }
}