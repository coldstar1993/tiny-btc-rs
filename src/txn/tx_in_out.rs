use crate::common::Result;
use crate::wallet::hash_pub_key;
use bitcoincash_addr::Address;
use failure::format_err;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// TXInput represents a transaction input
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXInput {
    pub txid: String,
    pub vout: i32,
    pub signature: Vec<u8>,
    pub pub_key: Vec<u8>,
}

/// TXOutput represents a transaction output
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TXOutput {
    pub value: i32,
    pub pub_key_hash: Vec<u8>,
}
