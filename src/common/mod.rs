pub mod config;

pub use config::*;

pub type Result<T> = std::result::Result<T, failure::Error>;
