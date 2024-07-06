//! transaction implement

use crate::common::Result;
use crate::blockchain::UTXOSet;
use crate::wallet::{Wallet, hash_pub_key};
use crate::txn::tx_in_out::{TXInput, TXOutput};
use bincode::serialize;
use bitcoincash_addr::Address;
use crypto::digest::Digest;
use crypto::ed25519;
use crypto::sha2::Sha256;
use failure::format_err;
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::{debug, error, info};
use rand::rngs::OsRng;

const SUBSIDY: i32 = 10;

/// Transaction represents a Bitcoin transaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub vin: Vec<TXInput>,
    pub vout: Vec<TXOutput>,
}
