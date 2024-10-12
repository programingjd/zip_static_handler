pub mod builder;
mod compression;
pub mod errors;
pub mod github;
pub mod handler;
pub mod http;
mod path;
pub mod types;

#[cfg(feature = "hyper")]
pub mod hyper;

#[cfg(feature = "axum")]
pub mod axum;

#[cfg(feature = "actix")]
pub mod actix;

#[cfg(feature = "rocket")]
pub mod rocket;

#[cfg(feature = "xitca")]
pub mod xitca;

#[cfg(feature = "salvo")]
pub mod salvo;

#[cfg(feature = "min_http11_parser")]
pub mod min_http11;

#[cfg(test)]
pub(crate) static INIT: std::sync::Once = std::sync::Once::new();
