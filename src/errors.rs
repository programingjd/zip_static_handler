use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Wrapped(Box<dyn std::error::Error>),
}

impl<T: std::error::Error + 'static> From<T> for Error {
    fn from(value: T) -> Self {
        Self::Wrapped(Box::new(value))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Wrapped(inner) => inner.fmt(f),
        }
    }
}
