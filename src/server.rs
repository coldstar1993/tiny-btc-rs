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

impl Server {
    pub fn new(port: &str, miner_address: &str, utxo: UTXOSet) -> Result<Server> {
        let mut node_set = HashSet::new();

        let json_str = fs::read_to_string("seeds.json").expect("cannot read from seeds.json");
        let json_value: Value = serde_json::from_str(&json_str).unwrap();
        if let Value::Object(obj) = &json_value {
            if let Some(Value::Array(seeds)) = obj.get("seeds") {
                for seed in seeds.iter() {
                    if let Value::String(addr) = seed {
                        node_set.insert(String::from(addr));
                    }
                }
            }
        }

        Ok(Server {
            node_address: String::from("localhost:") + port,
            mining_address: miner_address.to_string(),
            inner: Arc::new(Mutex::new(ServerInner {
                known_nodes: node_set,
                utxo,
                blocks_in_transit: Vec::new(),
                mempool: HashMap::new(),
            })),
        })
    }

    /// recieve sender's node_set
    fn handle_addr(&self, msg: Vec<String>) -> Result<()> {
        info!("receive address msg: {:#?}", msg);
        for node in msg {
            self.add_nodes(&node);
        }
        //self.request_blocks()?;
        Ok(())
    }

    fn remove_node(&self, addr: &str) {
        self.inner.lock().unwrap().known_nodes.remove(addr);
    }

    fn add_nodes(&self, addr: &str) {
        self.inner
            .lock()
            .unwrap()
            .known_nodes
            .insert(String::from(addr));
    }

    
    fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        let mut buffer = Vec::new();
        let count = stream.read_to_end(&mut buffer)?;
        info!("Accept request: length {}", count);

        let cmd = bytes_to_cmd(&buffer)?;

        match cmd {
            Message::Addr(data) => self.handle_addr(data)?,
            Message::Block(data) => (),
            Message::Inv(data) => (),
            Message::GetBlock(data) => (),
            Message::GetData(data) => (),
            Message::Tx(data) => (),
            Message::Version(data) => (),
        }

        Ok(())
    }
}

fn cmd_to_bytes(cmd: &str) -> [u8; CMD_LEN] {
    let mut data = [0; CMD_LEN];
    for (i, d) in cmd.as_bytes().iter().enumerate() {
        data[i] = *d;
    }
    data
}

fn bytes_to_cmd(bytes: &[u8]) -> Result<Message> {
    let mut cmd = Vec::new();
    let cmd_bytes = &bytes[..CMD_LEN];
    let data = &bytes[CMD_LEN..];
    for b in cmd_bytes {
        if 0 as u8 != *b {
            cmd.push(*b);
        }
    }
    info!("cmd: {}", String::from_utf8(cmd.clone())?);

    if cmd == "addr".as_bytes() {
        let data: Vec<String> = deserialize(data)?;
        Ok(Message::Addr(data))
    } else if cmd == "block".as_bytes() {
        let data: Blockmsg = deserialize(data)?;
        Ok(Message::Block(data))
    } else if cmd == "inv".as_bytes() {
        let data: Invmsg = deserialize(data)?;
        Ok(Message::Inv(data))
    } else if cmd == "getblocks".as_bytes() {
        let data: GetBlocksmsg = deserialize(data)?;
        Ok(Message::GetBlock(data))
    } else if cmd == "getdata".as_bytes() {
        let data: GetDatamsg = deserialize(data)?;
        Ok(Message::GetData(data))
    } else if cmd == "tx".as_bytes() {
        let data: Txmsg = deserialize(data)?;
        Ok(Message::Tx(data))
    } else if cmd == "version".as_bytes() {
        let data: Versionmsg = deserialize(data)?;
        Ok(Message::Version(data))
    } else {
        Err(format_err!("Unknown command in the server"))
    }
}
