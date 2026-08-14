#![forbid(unsafe_code)]

pub mod api;
pub mod date;
pub mod ids;
pub mod metrics;
pub mod model;
pub mod openapi;
pub mod query;
pub mod snapshot;
pub mod store;
pub mod text;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
