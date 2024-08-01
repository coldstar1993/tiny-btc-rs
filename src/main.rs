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

