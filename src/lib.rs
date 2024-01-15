mod compression;
pub mod errors;
pub mod github;
pub mod handler;
mod headers;
pub mod http;
mod path;
mod types;

#[cfg(test)]
pub(crate) static INIT: std::sync::Once = std::sync::Once::new();
