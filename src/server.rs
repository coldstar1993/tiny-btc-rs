//! server of Blockchain

use super::*;
use crate::blockchain::UTXOSet;

use crate::block::Block;
use crate::txn::Transaction;

use bincode::{deserialize, serialize};
use failure::format_err;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::*;
use std::thread;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Message {
    Addr(Vec<String>),
    Version(Versionmsg),
    Tx(Txmsg),
    GetData(GetDatamsg),
    GetBlock(GetBlocksmsg),
    Inv(Invmsg),
    Block(Blockmsg),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Blockmsg {
    addr_from: String,
    block: Block,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GetBlocksmsg {
    addr_from: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GetDatamsg {
    addr_from: String,
    kind: String,
    id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Invmsg {
    addr_from: String,
    kind: String,
    items: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Txmsg {
    addr_from: String,
    transaction: Transaction,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Versionmsg {
    addr_from: String,
    version: i32,
    best_height: i32,
}

pub struct Server {
    /**
     * current node address(ip:port)
     */
    node_address: String,
    /**
     * current node mining wallet addr
     */
    mining_address: String,
    /**
     * commonly shared instance
     */
    inner: Arc<Mutex<ServerInner>>,
}

struct ServerInner {
    /**
     * neighbor nodes: will be updated dynamically by communication with other nodes
     */
    known_nodes: HashSet<String>,
    /**
     * utxo set of ledger: will be updated dynamically with new block
     */
    utxo: UTXOSet,
    /**
     * blocks need to be fetched back
     */
    blocks_in_transit: Vec<String>,
    /**
     * memory pool: storing tx of one block time range
     */
    mempool: HashMap<String, Transaction>,
}

const CMD_LEN: usize = 12;
const VERSION: i32 = 1;

