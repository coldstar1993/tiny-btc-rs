//! Block implement of blockchain

use crate::txn::*;
use serde::{Deserialize, Serialize};

const TARGET_HEXS: usize = 4;

/// Block keeps block headers
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    timestamp: u128,
    /// will calc merkle tree root when mining
    transactions: Vec<Transaction>,
    prev_block_hash: String,
    hash: String,
    nonce: i32,
    height: i32,
}
