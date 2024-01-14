use crate::errors::Error::{Message, Wrapped};
use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Wrapped(Box<dyn std::error::Error>),
    Message(&'static str),
}

impl<T: std::error::Error + 'static> From<T> for Error {
    fn from(value: T) -> Self {
        Wrapped(Box::new(value))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Wrapped(inner) => inner.fmt(f),
            Message(inner) => write!(f, "{inner}"),
        }
    }
}
