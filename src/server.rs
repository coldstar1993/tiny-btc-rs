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

    pub fn start_server(&self) -> Result<()> {
        // clone a new Server instance, due to the usage within thread::spawn below.
        let server1 = Server {
            node_address: self.node_address.clone(),
            mining_address: self.mining_address.clone(),
            inner: Arc::clone(&self.inner),
        };

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1000));
            if server1.get_best_height()? == -1 {
                server1.request_blocks()
            } else {
                let nodeset = server1.get_known_nodes();
                for node in nodeset {
                    server1.send_version(&node);
                }
                Ok(())
            }
        });

        info!(
            "Start server at {}, minning address: {}",
            &self.node_address, &self.mining_address
        );

        let listener = TcpListener::bind(&self.node_address).unwrap();
        info!("Server listen...");

        for stream in listener.incoming() {
            let stream = stream?;
            let server1 = Server {
                node_address: self.node_address.clone(),
                mining_address: self.mining_address.clone(),
                inner: Arc::clone(&self.inner),
            };
            thread::spawn(move || server1.handle_connection(stream));
        }

        Ok(())
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

    fn get_known_nodes(&self) -> HashSet<String> {
        self.inner.lock().unwrap().known_nodes.clone()
    }

    fn node_is_known(&self, addr: &str) -> bool {
        self.inner.lock().unwrap().known_nodes.get(addr).is_some()
    }

    fn request_blocks(&self) -> Result<()> {
        for node in self.get_known_nodes() {
            self.send_get_blocks(&node)?
        }
        Ok(())
    }

    fn add_block(&self, block: Block) -> Result<()> {
        self.inner.lock().unwrap().utxo.blockchain.add_block(block)
    }

    fn mine_block(&self, txs: Vec<Transaction>) -> Result<Block> {
        self.inner.lock().unwrap().utxo.blockchain.mine_block(txs)
    }

    fn get_block(&self, block_hash: &str) -> Result<Block> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .get_block(block_hash)
    }

    fn get_block_hashs(&self) -> Vec<String> {
        self.inner.lock().unwrap().utxo.blockchain.get_block_hashs()
    }

    fn get_best_height(&self) -> Result<i32> {
        self.inner.lock().unwrap().utxo.blockchain.get_best_height()
    }

    fn verify_tx(&self, tx: &Transaction) -> Result<bool> {
        self.inner
            .lock()
            .unwrap()
            .utxo
            .blockchain
            .verify_transacton(tx)
    }

    fn utxo_reindex(&self) -> Result<()> {
        self.inner.lock().unwrap().utxo.reindex()
    }

    fn get_mempool(&self) -> HashMap<String, Transaction> {
        self.inner.lock().unwrap().mempool.clone()
    }

    fn insert_mempool(&self, tx: Transaction) {
        self.inner.lock().unwrap().mempool.insert(tx.id.clone(), tx);
    }

    fn clear_mempool(&self) {
        self.inner.lock().unwrap().mempool.clear()
    }

    fn get_mempool_tx(&self, addr: &str) -> Option<Transaction> {
        match self.inner.lock().unwrap().mempool.get(addr) {
            Some(tx) => Some(tx.clone()),
            None => None,
        }
    }

    fn replace_in_transit(&self, hashs: Vec<String>) {
        let bit = &mut self.inner.lock().unwrap().blocks_in_transit;
        bit.clone_from(&hashs);
    }

    fn get_in_transit(&self) -> Vec<String> {
        self.inner.lock().unwrap().blocks_in_transit.clone()
    }

    fn send_data(&self, addr: &str, data: &[u8]) -> Result<()> {
        if addr == &self.node_address {
            return Ok(());
        }
        let mut stream = match TcpStream::connect(addr) {
            Ok(s) => s,
            Err(_) => {
                self.remove_node(addr); // TODO should record score/ratio for each neighbor node
                return Ok(());
            }
        };

        stream.write(data)?;

        info!("data send successfully");
        Ok(())
    }

    fn send_block(&self, addr: &str, b: &Block) -> Result<()> {
        info!("send block data to: {} block hash: {}", addr, b.get_hash());
        let data = Blockmsg {
            addr_from: self.node_address.clone(),
            block: b.clone(),
        };
        let data = serialize(&(cmd_to_bytes("block"), data))?;
        self.send_data(addr, &data)
    }

    fn send_addr(&self, addr: &str) -> Result<()> {
        info!("send address info to: {}", addr);
        let nodes = self.get_known_nodes();
        let data = serialize(&(cmd_to_bytes("addr"), nodes))?;
        self.send_data(addr, &data)
    }

    /// inventory message:在比特币协议中，inv 消息（inventory message）用于节点间通告他们拥有哪些数据（如交易、区块）。这是一种节点用来告知其他节点自己拥有哪些对象的消息类型。
    ///
    /// Inventory Relay 机制:
    /// * 节点不直接广播原始交易(完整体)
    /// * 先发送 ​​INV消息​​（交易ID\区块ID）
    /// * 邻居节点主动请求完整数据：
    fn send_inv(&self, addr: &str, kind: &str, items: Vec<String>) -> Result<()> {
        info!(
            "send inv message to: {} kind: {} data: {:?}",
            addr, kind, items
        );
        let data = Invmsg {
            addr_from: self.node_address.clone(),
            kind: kind.to_string(),
            items,
        };
        let data = serialize(&(cmd_to_bytes("inv"), data))?;
        self.send_data(addr, &data)
    }

    fn send_get_blocks(&self, addr: &str) -> Result<()> {
        info!("send get blocks message to: {}", addr);
        let data = GetBlocksmsg {
            addr_from: self.node_address.clone(),
        };
        let data = serialize(&(cmd_to_bytes("getblocks"), data))?;
        self.send_data(addr, &data)
    }

    fn send_get_data(&self, addr: &str, kind: &str, id: &str) -> Result<()> {
        info!(
            "send get data message to: {} kind: {} id: {}",
            addr, kind, id
        );
        let data = GetDatamsg {
            addr_from: self.node_address.clone(),
            kind: kind.to_string(),
            id: id.to_string(),
        };
        let data = serialize(&(cmd_to_bytes("getdata"), data))?;
        self.send_data(addr, &data)
    }

    pub fn send_tx(&self, addr: &str, tx: &Transaction) -> Result<()> {
        info!("send tx to: {} txid: {}", addr, &tx.id);
        let data = Txmsg {
            addr_from: self.node_address.clone(),
            transaction: tx.clone(),
        };
        let data = serialize(&(cmd_to_bytes("tx"), data))?;
        self.send_data(addr, &data)
    }

    pub fn send_transaction(tx: &Transaction, utxoset: UTXOSet) -> Result<()> {
        let server = Server::new("7000", "", utxoset)?;
        server.send_tx(&CONFIG.read().unwrap().current_node(), tx)?;
        Ok(())
    }

    fn send_version(&self, addr: &str) -> Result<()> {
        info!("send version info to: {}", addr);
        let data = Versionmsg {
            addr_from: self.node_address.clone(),
            best_height: self.get_best_height()?,
            version: VERSION,
        };
        let data = serialize(&(cmd_to_bytes("version"), data))?;
        self.send_data(addr, &data)
    }

    /// recieving version msg means the sender (perhaps just start up) expects to exchange info including node_set, best_height, etc.
    fn handle_version(&self, msg: Versionmsg) -> Result<()> {
        info!("receive version msg: {:#?}", msg);
        let my_best_height = self.get_best_height()?;
        if my_best_height < msg.best_height {
            self.send_get_blocks(&msg.addr_from)?;
        } else if my_best_height > msg.best_height {
            self.send_version(&msg.addr_from)?;
        }

        // share local node_set
        self.send_addr(&msg.addr_from)?;

        if !self.node_is_known(&msg.addr_from) {
            self.add_nodes(&msg.addr_from);
        }
        Ok(())
    }

    /// handle a new tx from user
    fn handle_tx(&self, msg: Txmsg) -> Result<()> {
        info!("receive tx msg: {} {}", msg.addr_from, &msg.transaction.id);

        // if this tx has never been seen, then broadcast it to neighbor
        let mut mempool = self.get_mempool();
        debug!("Current mempool: {:#?}", &mempool);

        if !mempool.contains_key(&msg.transaction.id) {
            // if this is a totally new tx, then store it into mempool and broadcat it.
            info!(
                "mempool does NOT contain the tx:{}, then store it and broadcast it to neighbor",
                &msg.transaction.id
            );

            self.insert_mempool(msg.transaction.clone());

            let known_nodes = self.get_known_nodes();
            for node in known_nodes {
                if node != self.node_address && node != msg.addr_from {
                    self.send_inv(&node, "tx", vec![msg.transaction.id.clone()])?;
                }
            }
        }

        if mempool.len() >= 1 && !self.mining_address.is_empty() {
            loop {
                let mut txs = Vec::new();

                for (_, tx) in &mempool {
                    if self.verify_tx(tx)? {
                        txs.push(tx.clone());
                    }
                }

                if txs.is_empty() {
                    return Ok(());
                }

                let cbtx = Transaction::new_coinbase(self.mining_address.clone(), String::new())?;
                txs.push(cbtx);

                for tx in &txs {
                    mempool.remove(&tx.id);
                }

                let new_block = self.mine_block(txs)?;
                self.utxo_reindex()?;

                // broadcast the newly mined block hash to neighbor nodes
                for node in self.get_known_nodes() {
                    if node != self.node_address {
                        self.send_inv(&node, "block", vec![new_block.get_hash()])?;
                    }
                }

                if mempool.len() == 0 {
                    break;
                }
            }
            self.clear_mempool();
        }

        Ok(())
    }

    fn handle_inv(&self, msg: Invmsg) -> Result<()> {
        info!("receive inv msg: {:#?}", msg);
        if msg.kind == "block" {
            let block_hash = &msg.items[0];
            self.send_get_data(&msg.addr_from, "block", block_hash)?;

            let mut new_in_transit = Vec::new();
            for b in &msg.items {
                if b != block_hash {
                    new_in_transit.push(b.clone());
                }
            }
            self.replace_in_transit(new_in_transit);
        } else if msg.kind == "tx" {
            let txid = &msg.items[0];
            match self.get_mempool_tx(txid) {
                Some(tx) => {
                    if tx.id.is_empty() {
                        self.send_get_data(&msg.addr_from, "tx", txid)?
                    }
                }
                None => self.send_get_data(&msg.addr_from, "tx", txid)?,
            }
        }
        Ok(())
    }

    /// send back all block hashes in local ledger to **`from_addr`**
    fn handle_get_blocks(&self, msg: GetBlocksmsg) -> Result<()> {
        info!("receive get blocks msg: {:#?}", msg);
        let block_hashs = self.get_block_hashs();
        self.send_inv(&msg.addr_from, "block", block_hashs)?;
        Ok(())
    }

    /// recieve a block and persist it
    fn handle_block(&self, msg: Blockmsg) -> Result<()> {
        info!(
            "receive block msg: {}, {}",
            msg.addr_from,
            msg.block.get_hash()
        );
        self.add_block(msg.block)?;

        let mut in_transit = self.get_in_transit();
        if in_transit.len() > 0 {
            let block_hash = &in_transit[0];
            self.send_get_data(&msg.addr_from, "block", block_hash)?;
            in_transit.remove(0);
            self.replace_in_transit(in_transit);
        } else {
            self.utxo_reindex()?;
        }

        Ok(())
    }

    /// send back complete block/tx body to **`from_addr`**
    fn handle_get_data(&self, msg: GetDatamsg) -> Result<()> {
        info!("receive get data msg: {:#?}", msg);
        if msg.kind == "block" {
            let block = self.get_block(&msg.id)?;
            self.send_block(&msg.addr_from, &block)?;
        } else if msg.kind == "tx" {
            let tx = self.get_mempool_tx(&msg.id).unwrap();
            self.send_tx(&msg.addr_from, &tx)?;
        }
        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        let mut buffer = Vec::new();
        let count = stream.read_to_end(&mut buffer)?;
        info!("Accept request: length {}", count);

        let cmd = bytes_to_cmd(&buffer)?;

        match cmd {
            Message::Version(data) => self.handle_version(data)?,
            Message::Addr(data) => self.handle_addr(data)?,
            Message::Block(data) => self.handle_block(data)?,
            Message::Inv(data) => self.handle_inv(data)?,
            Message::GetBlock(data) => self.handle_get_blocks(data)?,
            Message::GetData(data) => self.handle_get_data(data)?,
            Message::Tx(data) => self.handle_tx(data)?,
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
