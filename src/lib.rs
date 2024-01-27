pub mod builder;
mod compression;
pub mod errors;
pub mod github;
pub mod handler;
pub mod http;
mod path;
mod types;

#[cfg(feature = "hyper")]
pub mod hyper;

#[cfg(feature = "axum")]
pub mod axum;

#[cfg(feature = "actix")]
pub mod actix;

#[cfg(test)]
pub(crate) static INIT: std::sync::Once = std::sync::Once::new();
