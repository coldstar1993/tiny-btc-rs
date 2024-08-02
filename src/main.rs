use std::env;
use std::sync::{LazyLock, RwLock};

use env_logger::Env;

use crate::cli::Cli;
use crate::common::{AppConfig, Result};

mod block;
mod blockchain;
mod cli;
mod common;
mod server;
mod txn;
mod wallet;

pub static CONFIG: LazyLock<RwLock<AppConfig>> = LazyLock::new(||{
    RwLock::new(AppConfig{
        port: env::var("port").unwrap_or(String::from("3000")).parse().unwrap()
    })
});

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    Ok(())
}
