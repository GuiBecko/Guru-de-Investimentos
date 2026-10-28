mod db;
mod error;

pub use db::{Db, SCHEMA_VERSION};
pub use error::CoreError;

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
